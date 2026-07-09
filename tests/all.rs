use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
use std::time::{Duration, Instant};

const TEST_DEADLINE_SECS: f64 = 60.0;
const WALL_BUDGET_SECS: f64 = 600.0;
const PROBE_DEADLINE_SECS: f64 = 10.0;
const KFD_TEARDOWN_DEADLINE_SECS: f64 = 30.0;
const DEVICE_FREE_DEADLINE_SECS: f64 = 30.0;
const MAX_RESTART_UNITS: usize = 8;
const MAX_CMDLINE_CHARS: usize = 120;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

struct Test {
      id: String,
      exe: PathBuf,
      cwd: PathBuf,
      name: String,
      ignored: bool,
}

#[derive(Clone, PartialEq)]
enum Outcome {
      Pass,
      ExitFail(i32),
      Signal(i32),
      Deadline,
}

struct Attempt {
      secs: f64,
      outcome: Outcome,
      capture: Vec<String>,
}

struct Log {
      f: File,
}

impl Log {
      fn line(&mut self, s: &str) {
            writeln!(self.f, "{s}").expect("suite.log write");
            self.f.flush().expect("suite.log flush");
            eprintln!("{s}");
      }
}

static CHILD_PGID: AtomicI32 = AtomicI32::new(0);

#[repr(C)]
struct Restart {
      exe: *const libc::c_char,
      n: usize,
      argv: [*const *const libc::c_char; MAX_RESTART_UNITS],
}

static RESTART: AtomicPtr<Restart> = AtomicPtr::new(std::ptr::null_mut());

fn arm_restart(systemctl: &Path, units: &[String]) {
      let exe = CString::new(systemctl.as_os_str().as_bytes()).expect("systemctl path");
      let mut r = Restart {
            exe: exe.into_raw(),
            n: 0,
            argv: [std::ptr::null(); MAX_RESTART_UNITS],
      };
      for u in units.iter().take(MAX_RESTART_UNITS) {
            let mut v: Vec<*const libc::c_char> = Vec::with_capacity(5);
            for s in ["systemctl", "--user", "start", u.as_str()] {
                  v.push(CString::new(s).expect("argv element").into_raw());
            }
            v.push(std::ptr::null());
            r.argv[r.n] = Box::leak(v.into_boxed_slice()).as_ptr();
            r.n += 1;
      }
      RESTART.store(Box::into_raw(Box::new(r)), Ordering::SeqCst);
}

fn disarm_restart() {
      RESTART.store(std::ptr::null_mut(), Ordering::SeqCst);
}

fn restart_units_from_handler() {
      let p = RESTART.load(Ordering::SeqCst);
      if p.is_null() {
            return;
      }
      unsafe {
            let r = &*p;
            for i in 0..r.n {
                  match libc::fork() {
                        0 => {
                              libc::execv(r.exe, r.argv[i]);
                              libc::_exit(127);
                        }
                        pid if pid > 0 => {
                              let mut st: libc::c_int = 0;
                              libc::waitpid(pid, &mut st, 0);
                        }
                        _ => {}
                  }
            }
      }
}

extern "C" fn kill_child_group(_sig: i32) {
      let pgid = CHILD_PGID.load(Ordering::SeqCst);
      if pgid > 0 {
            unsafe {
                  libc::kill(-pgid, libc::SIGKILL);
            }
      }
      restart_units_from_handler();
      unsafe { libc::_exit(130) }
}

fn install_traps() {
      let h = kill_child_group as extern "C" fn(i32) as libc::sighandler_t;
      unsafe {
            libc::signal(libc::SIGINT, h);
            libc::signal(libc::SIGTERM, h);
            libc::signal(libc::SIGHUP, h);
      }
}

fn main() {
      install_traps();
      let code = run();
      let pgid = CHILD_PGID.load(Ordering::SeqCst);
      if pgid > 0 {
            unsafe {
                  libc::kill(-pgid, libc::SIGKILL);
            }
      }
      std::process::exit(code);
}

fn run() -> i32 {
      let t0 = Instant::now();
      let args: Vec<String> = std::env::args().skip(1).collect();
      let filter: Option<String> = match args.as_slice() {
            [] => None,
            [a] if a == "all" => None,
            [a] => Some(a.clone()),
            _ => {
                  eprintln!("suite: takes at most one arg (\"all\" or an id substring), got {args:?}");
                  return 2;
            }
      };

      let binaries = match discover_binaries() {
            Ok(b) => b,
            Err(e) => {
                  eprintln!("suite: discovery failed: {e}");
                  return 2;
            }
      };
      let probe = match build_probe() {
            Ok(p) => p,
            Err(e) => {
                  eprintln!("suite: probe build failed: {e}");
                  return 2;
            }
      };
      let mut tests: Vec<Test> = Vec::new();
      for (pkg, target, exe, cwd) in &binaries {
            let (names, ignored) = match list_tests(exe, cwd) {
                  Ok(v) => v,
                  Err(e) => {
                        eprintln!("suite: --list failed for {}: {e}", exe.display());
                        return 2;
                  }
            };
            for name in names {
                  tests.push(Test {
                        id: format!("{pkg}/{target}/{name}"),
                        exe: exe.clone(),
                        cwd: cwd.clone(),
                        ignored: ignored.contains(&name),
                        name,
                  });
            }
      }
      let colliding: Vec<&str> =
            tests.iter().filter(|t| t.id.contains("all")).map(|t| t.id.as_str()).collect();
      if !colliding.is_empty() {
            eprintln!("suite: test ids may not contain \"all\" (cargo filter collision):");
            for id in colliding {
                  eprintln!("  {id}");
            }
            return 2;
      }
      if let Some(f) = &filter {
            tests.retain(|t| t.id.contains(f.as_str()));
      }
      tests.sort_by(|a, b| a.id.cmp(&b.id));
      let discovered = tests.len();

      let mut log = Log {
            f: File::create(Path::new(ROOT).join("suite.log")).expect("create suite.log"),
      };
      let head = format!("[S] discovered={} binaries={}", discovered, binaries.len());
      log.line(&head);
      println!("{head}");
      if discovered == 0 {
            let mut gate = DeviceGate { state: Device::Clear, holders: Vec::new(), restored: true };
            return finish(&mut log, 0, 0, 0, t0, 0.0, 3, &mut gate);
      }

      let mut gate = DeviceGate::open(&mut log, &probe);
      if gate.skip_all() {
            let reason = gate.skip_reason();
            for t in &tests {
                  log.line(&format!("[SKIP] {} 0.0 {reason}", t.id));
            }
            log.line("[S] SKIP-MODE no test ran: free /dev/kfd or stop its holder by hand");
            return finish(&mut log, 0, 0, discovered, t0, 0.0, 4, &mut gate);
      }
      let known = gate.known();

      let (mut passed, mut failed) = (0usize, 0usize);
      let mut test_secs = 0.0f64;
      let mut prev_violent: Option<String> = None;
      let mut rerun: Vec<(usize, Attempt, String)> = Vec::new();
      let mut poisoned: Option<String> = None;
      let mut busy_abort = false;

      for (i, t) in tests.iter().enumerate() {
            if t.ignored {
                  log.line(&format!("[FAIL] {} 0.0 ignored", t.id));
                  failed += 1;
                  prev_violent = None;
                  continue;
            }
            let leaked = new_holders(&known);
            if !leaked.is_empty() {
                  log.line(&format!("[S] DEVICE-BUSY before={} pids={leaked:?}", t.id));
                  busy_abort = true;
                  break;
            }
            let att = run_test(t);
            let violent = matches!(att.outcome, Outcome::Signal(_) | Outcome::Deadline);
            if let Some(culprit) = prev_violent.clone().filter(|_| att.outcome != Outcome::Pass) {
                  rerun.push((i, att, culprit));
                  prev_violent = if violent { Some(t.id.clone()) } else { None };
                  if violent && !probe_ok(&probe) {
                        poisoned = Some(t.id.clone());
                        break;
                  }
                  continue;
            }
            record(&mut log, &t.id, &att, &mut passed, &mut failed, &mut test_secs, None);
            prev_violent = if violent { Some(t.id.clone()) } else { None };
            if violent && !probe_ok(&probe) {
                  poisoned = Some(t.id.clone());
                  break;
            }
      }

      if busy_abort {
            return finish(&mut log, passed, failed, 0, t0, test_secs, 4, &mut gate);
      }

      if poisoned.is_none() {
            for (i, first, culprit) in &rerun {
                  let t = &tests[*i];
                  let leaked = new_holders(&known);
                  if !leaked.is_empty() {
                        log.line(&format!("[S] DEVICE-BUSY before={} pids={leaked:?}", t.id));
                        return finish(&mut log, passed, failed, 0, t0, test_secs, 4, &mut gate);
                  }
                  let att = run_test(t);
                  let violent = matches!(att.outcome, Outcome::Signal(_) | Outcome::Deadline);
                  record(
                        &mut log,
                        &t.id,
                        &att,
                        &mut passed,
                        &mut failed,
                        &mut test_secs,
                        Some((first, culprit.as_str())),
                  );
                  if violent && !probe_ok(&probe) {
                        poisoned = Some(t.id.clone());
                        break;
                  }
            }
      }

      if let Some(after) = &poisoned {
            log.line(&format!("[S] DEVICE-POISONED after={after}"));
            return finish(&mut log, passed, failed, 0, t0, test_secs, 4, &mut gate);
      }

      let total = passed + failed;
      let code = if total != discovered {
            5
      } else if failed > 0 {
            1
      } else if t0.elapsed().as_secs_f64() > WALL_BUDGET_SECS {
            6
      } else {
            0
      };
      finish(&mut log, passed, failed, 0, t0, test_secs, code, &mut gate)
}

fn new_holders(known: &[i32]) -> Vec<i32> {
      kfd_holders().into_iter().filter(|p| !known.contains(p)).collect()
}

fn record(
      log: &mut Log,
      id: &str,
      att: &Attempt,
      passed: &mut usize,
      failed: &mut usize,
      test_secs: &mut f64,
      first_try: Option<(&Attempt, &str)>,
) {
      *test_secs += att.secs;
      let mut k = 0usize;
      match &att.outcome {
            Outcome::Pass => {
                  log.line(&format!("[PASS] {} {:.1}", id, att.secs));
                  *passed += 1;
            }
            o => {
                  log.line(&format!("[FAIL] {} {:.1} {}", id, att.secs, reason(o)));
                  *failed += 1;
                  for line in &att.capture {
                        k += 1;
                        log.line(&format!("[E{k}] {line}"));
                  }
            }
      }
      if let Some((first, culprit)) = first_try {
            k += 1;
            log.line(&format!(
                  "[E{k}] first-try: FAIL {:.1} {} suspect-contamination culprit={culprit}",
                  first.secs,
                  reason(&first.outcome)
            ));
            for line in &first.capture {
                  k += 1;
                  log.line(&format!("[E{k}] first-try: {line}"));
            }
      }
}

fn reason(o: &Outcome) -> String {
      match o {
            Outcome::Pass => String::new(),
            Outcome::ExitFail(c) => format!("exit={c}"),
            Outcome::Signal(s) => format!("signal={s}"),
            Outcome::Deadline => "deadline".into(),
      }
}

fn finish(
      log: &mut Log,
      passed: usize,
      failed: usize,
      skipped: usize,
      t0: Instant,
      test_secs: f64,
      code: i32,
      gate: &mut DeviceGate,
) -> i32 {
      gate.restore(log);
      let wall = t0.elapsed().as_secs_f64();
      let tail = format!(
            "[S] passed={passed} failed={failed} skipped={skipped} total={} wall={wall:.1}s overhead={:.1}s exit={code}",
            passed + failed + skipped,
            wall - test_secs
      );
      log.line(&tail);
      println!("{tail}");
      let text = std::fs::read_to_string(Path::new(ROOT).join("suite.log")).expect("reread log");
      for line in text.lines() {
            if !grammar_ok(line) {
                  eprintln!("suite: log grammar violation: {line:?}");
                  return 2;
            }
      }
      code
}

fn grammar_ok(line: &str) -> bool {
      if line.starts_with("[S] ")
            || line.starts_with("[PASS] ")
            || line.starts_with("[FAIL] ")
            || line.starts_with("[SKIP] ")
      {
            return true;
      }
      if let Some(rest) = line.strip_prefix("[E") {
            if let Some(close) = rest.find(']') {
                  return close > 0
                        && rest[..close].bytes().all(|b| b.is_ascii_digit())
                        && (rest[close + 1..].starts_with(' ') || rest[close + 1..].is_empty());
            }
      }
      false
}

fn await_kfd_teardown(pid: u32) {
      let path = format!("/sys/class/kfd/kfd/proc/{pid}");
      let t0 = Instant::now();
      while Path::new(&path).exists() {
            if t0.elapsed().as_secs_f64() >= KFD_TEARDOWN_DEADLINE_SECS {
                  eprintln!("[RUN] kfd teardown of pid {pid} still live after {:.0}s — proceeding", t0.elapsed().as_secs_f64());
                  return;
            }
            std::thread::sleep(Duration::from_millis(3));
      }
}

fn run_test(t: &Test) -> Attempt {
      let cap_path = Path::new(ROOT).join("target").join(".suite_capture");
      let cap = File::create(&cap_path).expect("create capture file");
      let cap2 = cap.try_clone().expect("clone capture handle");
      let mut cmd = Command::new(&t.exe);
      cmd.args(["--exact", &t.name, "--nocapture"])
            .current_dir(&t.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::from(cap))
            .stderr(Stdio::from(cap2));
      unsafe {
            cmd.pre_exec(|| {
                  libc::setsid();
                  Ok(())
            });
      }
      let mut child = cmd.spawn().expect("spawn test process");
      let child_pid = child.id();
      CHILD_PGID.store(child_pid as i32, Ordering::SeqCst);
      let start = Instant::now();
      let mut next_tick = 10u64;
      let (secs, status) = loop {
            match child.try_wait().expect("try_wait") {
                  Some(st) => break (start.elapsed().as_secs_f64(), Some(st)),
                  None => {
                        let el = start.elapsed().as_secs_f64();
                        if el >= next_tick as f64 {
                              eprintln!("[RUN] {} {}s", t.id, next_tick);
                              next_tick += 10;
                        }
                        if el >= TEST_DEADLINE_SECS {
                              unsafe {
                                    libc::kill(-(child_pid as i32), libc::SIGKILL);
                              }
                              child.wait().expect("waitpid after kill");
                              break (TEST_DEADLINE_SECS, None);
                        }
                        std::thread::sleep(Duration::from_millis(3));
                  }
            }
      };
      CHILD_PGID.store(0, Ordering::SeqCst);
      await_kfd_teardown(child_pid);
      let outcome = match status {
            None => Outcome::Deadline,
            Some(st) if st.success() => Outcome::Pass,
            Some(st) => match (st.code(), st.signal()) {
                  (Some(c), _) => Outcome::ExitFail(c),
                  (None, Some(s)) => Outcome::Signal(s),
                  (None, None) => Outcome::Signal(-1),
            },
      };
      let capture = if outcome == Outcome::Pass {
            Vec::new()
      } else {
            let mut buf = Vec::new();
            if let Ok(mut f) = File::open(&cap_path) {
                  let _ = f.read_to_end(&mut buf);
            }
            String::from_utf8_lossy(&buf).lines().map(str::to_owned).collect()
      };
      Attempt { secs, outcome, capture }
}

fn probe_ok(probe: &Path) -> bool {
      let mut cmd = Command::new(probe);
      cmd.current_dir(ROOT)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
      unsafe {
            cmd.pre_exec(|| {
                  libc::setsid();
                  Ok(())
            });
      }
      let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return false,
      };
      let pid = child.id();
      CHILD_PGID.store(pid as i32, Ordering::SeqCst);
      let start = Instant::now();
      let ok = loop {
            match child.try_wait() {
                  Ok(Some(st)) => break st.success(),
                  Ok(None) => {
                        if start.elapsed().as_secs_f64() >= PROBE_DEADLINE_SECS {
                              unsafe {
                                    libc::kill(-(pid as i32), libc::SIGKILL);
                              }
                              let _ = child.wait();
                              break false;
                        }
                        std::thread::sleep(Duration::from_millis(3));
                  }
                  Err(_) => break false,
            }
      };
      CHILD_PGID.store(0, Ordering::SeqCst);
      await_kfd_teardown(pid);
      ok
}

struct Holder {
      pid: i32,
      unit: Option<String>,
      comm: String,
      cmd: String,
}

enum Device {
      Clear,
      Stopped(Vec<String>),
      Shared,
      Unusable,
}

struct DeviceGate {
      state: Device,
      holders: Vec<Holder>,
      restored: bool,
}

impl DeviceGate {
      fn open(log: &mut Log, probe: &Path) -> DeviceGate {
            let pids = kfd_holders();
            if pids.is_empty() {
                  return DeviceGate { state: Device::Clear, holders: Vec::new(), restored: true };
            }
            let systemctl = find_systemctl();
            let holders: Vec<Holder> = pids
                  .iter()
                  .map(|&pid| Holder {
                        pid,
                        unit: systemctl.as_ref().and_then(|_| user_unit_of(pid)),
                        comm: proc_field(pid, "comm"),
                        cmd: proc_cmdline(pid),
                  })
                  .collect();
            for h in &holders {
                  log.line(&format!(
                        "[S] HOLDER pid={} unit={} comm={} cmd={}",
                        h.pid,
                        h.unit.as_deref().unwrap_or("-"),
                        h.comm,
                        h.cmd
                  ));
            }

            if let Some(sc) = systemctl.filter(|_| holders.iter().all(|h| h.unit.is_some())) {
                  let mut units: Vec<String> =
                        holders.iter().filter_map(|h| h.unit.clone()).collect();
                  units.sort();
                  units.dedup();
                  let mut stopped_clean = true;
                  for u in &units {
                        let (rc, err) = systemctl_run(&sc, &["--user", "stop", u]);
                        log.line(&format!("[S] DAEMON-STOP unit={u} rc={rc}{}", note(rc, &err)));
                        stopped_clean &= rc == 0;
                  }
                  arm_restart(&sc, &units);
                  let t0 = Instant::now();
                  if stopped_clean && wait_device_free(&pids) {
                        log.line(&format!("[S] DEVICE-FREE wait={:.1}s", t0.elapsed().as_secs_f64()));
                        return DeviceGate { state: Device::Stopped(units), holders, restored: false };
                  }
                  let mut g = DeviceGate { state: Device::Stopped(units), holders, restored: false };
                  g.restore(log);
                  g.state = probe_state(log, probe);
                  return g;
            }

            DeviceGate { state: probe_state(log, probe), holders, restored: true }
      }

      fn skip_all(&self) -> bool {
            matches!(self.state, Device::Unusable)
      }

      fn known(&self) -> Vec<i32> {
            match self.state {
                  Device::Shared | Device::Unusable => self.holders.iter().map(|h| h.pid).collect(),
                  Device::Clear | Device::Stopped(_) => Vec::new(),
            }
      }

      fn skip_reason(&self) -> String {
            format!("device-busy probe=fail pids={:?}", self.known())
      }

      fn restore(&mut self, log: &mut Log) {
            if self.restored {
                  return;
            }
            self.restored = true;
            if let Device::Stopped(units) = &self.state {
                  match find_systemctl() {
                        Some(sc) => {
                              for u in units {
                                    let (rc, err) = systemctl_run(&sc, &["--user", "start", u]);
                                    log.line(&format!(
                                          "[S] DAEMON-START unit={u} rc={rc}{}",
                                          note(rc, &err)
                                    ));
                              }
                        }
                        None => {
                              for u in units {
                                    log.line(&format!(
                                          "[S] DAEMON-START unit={u} rc=-1 err=systemctl-not-found"
                                    ));
                              }
                        }
                  }
            }
            disarm_restart();
      }
}

impl Drop for DeviceGate {
      fn drop(&mut self) {
            if self.restored {
                  return;
            }
            if let Device::Stopped(units) = &self.state {
                  if let Some(sc) = find_systemctl() {
                        for u in units {
                              let (rc, _) = systemctl_run(&sc, &["--user", "start", u]);
                              eprintln!("[RUN] daemon restart on unwind: unit={u} rc={rc}");
                        }
                  }
            }
            disarm_restart();
      }
}

fn probe_state(log: &mut Log, probe: &Path) -> Device {
      if probe_ok(probe) {
            log.line("[S] DEVICE-SHARED probe=ok holder is not a systemd --user service");
            Device::Shared
      } else {
            log.line("[S] DEVICE-UNUSABLE probe=fail holder is not a systemd --user service");
            Device::Unusable
      }
}

fn user_unit_of(pid: i32) -> Option<String> {
      let cg = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
      let path = cg.lines().find_map(|l| l.strip_prefix("0::"))?;
      if !path.contains("/user@") {
            return None;
      }
      let last = path.rsplit('/').next()?;
      if last.starts_with("user@") || !last.ends_with(".service") {
            return None;
      }
      Some(last.to_owned())
}

fn find_systemctl() -> Option<PathBuf> {
      let path = std::env::var("PATH").ok()?;
      path.split(':').map(|d| Path::new(d).join("systemctl")).find(|p| p.is_file())
}

fn systemctl_run(systemctl: &Path, args: &[&str]) -> (i32, String) {
      match Command::new(systemctl).args(args).stdin(Stdio::null()).output() {
            Ok(o) => (
                  o.status.code().unwrap_or(-1),
                  String::from_utf8_lossy(&o.stderr).split_whitespace().collect::<Vec<_>>().join(" "),
            ),
            Err(e) => (-1, e.to_string()),
      }
}

fn note(rc: i32, err: &str) -> String {
      if rc == 0 || err.is_empty() { String::new() } else { format!(" err={err}") }
}

fn wait_device_free(pids: &[i32]) -> bool {
      for p in pids {
            await_kfd_teardown(*p as u32);
      }
      let t0 = Instant::now();
      while !kfd_holders().is_empty() {
            if t0.elapsed().as_secs_f64() >= DEVICE_FREE_DEADLINE_SECS {
                  return false;
            }
            std::thread::sleep(Duration::from_millis(10));
      }
      true
}

fn proc_field(pid: i32, field: &str) -> String {
      std::fs::read_to_string(format!("/proc/{pid}/{field}"))
            .map(|s| s.trim().to_owned())
            .unwrap_or_else(|_| "?".into())
}

fn proc_cmdline(pid: i32) -> String {
      let raw = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
      let joined: String = raw
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect::<Vec<_>>()
            .join(" ");
      let flat = joined.split_whitespace().collect::<Vec<_>>().join(" ");
      if flat.is_empty() {
            "?".into()
      } else {
            flat.chars().take(MAX_CMDLINE_CHARS).collect()
      }
}

fn kfd_holders() -> Vec<i32> {
      let out = match Command::new("fuser").arg("/dev/kfd").stderr(Stdio::null()).output() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
      };
      let me = std::process::id() as i32;
      String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .filter_map(|tok| {
                  let digits: String = tok.chars().take_while(|c| c.is_ascii_digit()).collect();
                  digits.parse::<i32>().ok()
            })
            .filter(|p| *p != me && Path::new(&format!("/proc/{p}")).exists())
            .collect()
}

fn discover_binaries() -> Result<Vec<(String, String, PathBuf, PathBuf)>, String> {
      let out = Command::new("cargo")
            .args([
                  "test",
                  "--workspace",
                  "--release",
                  "--no-run",
                  "--lib",
                  "--bins",
                  "--tests",
                  "--message-format=json",
            ])
            .current_dir(ROOT)
            .stdout(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
      if !out.status.success() {
            return Err("cargo test --no-run failed (see stderr above)".into());
      }
      let mut seen = BTreeSet::new();
      let mut bins = Vec::new();
      for line in String::from_utf8_lossy(&out.stdout).lines() {
            let v: serde_json::Value = match serde_json::from_str(line) {
                  Ok(v) => v,
                  Err(_) => continue,
            };
            if v["reason"] != "compiler-artifact" || v["profile"]["test"] != true {
                  continue;
            }
            let Some(exe) = v["executable"].as_str() else { continue };
            let target = v["target"]["name"].as_str().unwrap_or_default().to_owned();
            let manifest = v["manifest_path"].as_str().unwrap_or_default();
            let pkg = package_name(v["package_id"].as_str().unwrap_or_default(), manifest);
            if pkg == "recipe" && target == "all" {
                  continue;
            }
            let cwd = Path::new(manifest).parent().unwrap_or(Path::new(ROOT)).to_path_buf();
            if seen.insert(exe.to_owned()) {
                  bins.push((pkg, target, PathBuf::from(exe), cwd));
            }
      }
      Ok(bins)
}

fn package_name(package_id: &str, manifest: &str) -> String {
      if let Some((path, tail)) = package_id.rsplit_once('#') {
            if let Some((name, _ver)) = tail.rsplit_once('@') {
                  return name.to_owned();
            }
            if let Some(dir) = path.rsplit('/').next() {
                  return dir.to_owned();
            }
      }
      if let Some(first) = package_id.split_whitespace().next() {
            if !first.is_empty() {
                  return first.to_owned();
            }
      }
      Path::new(manifest)
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into())
}

fn build_probe() -> Result<PathBuf, String> {
      let out = Command::new("cargo")
            .args(["build", "-p", "gpu-core", "--release", "--bin", "probe", "--message-format=json"])
            .current_dir(ROOT)
            .stdout(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
      if !out.status.success() {
            return Err("cargo build probe failed".into());
      }
      for line in String::from_utf8_lossy(&out.stdout).lines() {
            let v: serde_json::Value = match serde_json::from_str(line) {
                  Ok(v) => v,
                  Err(_) => continue,
            };
            if v["reason"] == "compiler-artifact"
                  && v["target"]["name"] == "probe"
                  && v["profile"]["test"] != true
            {
                  if let Some(exe) = v["executable"].as_str() {
                        return Ok(PathBuf::from(exe));
                  }
            }
      }
      Err("probe executable not found in cargo output".into())
}

fn list_tests(exe: &Path, cwd: &Path) -> Result<(Vec<String>, BTreeSet<String>), String> {
      let list = |extra: &[&str]| -> Result<Vec<String>, String> {
            let mut args = vec!["--list", "--format", "terse"];
            args.extend_from_slice(extra);
            let out = Command::new(exe)
                  .args(&args)
                  .current_dir(cwd)
                  .output()
                  .map_err(|e| e.to_string())?;
            if !out.status.success() {
                  return Err(format!("--list exited {:?}", out.status.code()));
            }
            Ok(String::from_utf8_lossy(&out.stdout)
                  .lines()
                  .filter_map(|l| l.strip_suffix(": test").map(str::to_owned))
                  .collect())
      };
      let ignored: BTreeSet<String> = list(&["--ignored"])?.into_iter().collect();
      let mut names: BTreeSet<String> = list(&[])?.into_iter().collect();
      names.extend(ignored.iter().cloned());
      Ok((names.into_iter().collect(), ignored))
}
