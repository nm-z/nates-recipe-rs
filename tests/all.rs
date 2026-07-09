use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

const TEST_DEADLINE_SECS: f64 = 60.0;
const WALL_BUDGET_SECS: f64 = 600.0;
const PROBE_DEADLINE_SECS: f64 = 10.0;
const KFD_TEARDOWN_DEADLINE_SECS: f64 = 30.0;
const DEVICE_FREE_DEADLINE_SECS: f64 = 30.0;

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

struct Tally {
      discovered: usize,
      passed: usize,
      failed: usize,
      unattempted: usize,
}

impl Tally {
      fn ran(&self) -> usize {
            self.passed + self.failed
      }

      fn failed_total(&self) -> usize {
            self.failed + self.unattempted
      }

      fn green(&self) -> bool {
            self.discovered > 0 && self.passed == self.discovered
      }
}

fn out(s: &str) {
      let _ = writeln!(std::io::stdout(), "{s}");
}

fn errline(s: &str) {
      let _ = writeln!(std::io::stderr(), "{s}");
}

struct Log {
      f: File,
}

impl Log {
      fn file(&mut self, s: &str) {
            writeln!(self.f, "{s}").expect("suite.log write");
            self.f.flush().expect("suite.log flush");
      }

      fn line(&mut self, s: &str) {
            self.file(s);
            errline(s);
      }
}

static CHILD_PGID: AtomicI32 = AtomicI32::new(0);

extern "C" fn kill_child_group(_sig: i32) {
      let pgid = CHILD_PGID.load(Ordering::SeqCst);
      if pgid > 0 {
            unsafe {
                  libc::kill(-pgid, libc::SIGKILL);
            }
      }
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
      let code = match std::panic::catch_unwind(run) {
            Ok(c) => c,
            Err(p) => fatal(&format!("suite panicked: {}", panic_msg(&*p))),
      };
      let pgid = CHILD_PGID.load(Ordering::SeqCst);
      if pgid > 0 {
            unsafe {
                  libc::kill(-pgid, libc::SIGKILL);
            }
      }
      std::process::exit(code);
}

fn acquire_lock() -> Option<File> {
      let f = File::create(Path::new(ROOT).join("target").join(".suite.lock")).ok()?;
      let rc = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
      if rc == 0 { Some(f) } else { None }
}

fn panic_msg(p: &(dyn std::any::Any + Send)) -> String {
      if let Some(s) = p.downcast_ref::<&str>() {
            return (*s).to_owned();
      }
      if let Some(s) = p.downcast_ref::<String>() {
            return s.clone();
      }
      "unknown payload".to_owned()
}

fn run() -> i32 {
      let t0 = Instant::now();
      let args: Vec<String> = std::env::args().skip(1).collect();
      let filter: Option<String> = match args.as_slice() {
            [] => None,
            [a] if a == "all" => None,
            [a] => Some(a.clone()),
            _ => {
                  return fatal(&format!(
                        "suite takes at most one arg (\"all\" or an id substring), got {args:?}"
                  ));
            }
      };

      let _lock = match acquire_lock() {
            Some(f) => f,
            None => return fatal("another suite run already holds target/.suite.lock"),
      };

      let binaries = match discover_binaries() {
            Ok(b) => b,
            Err(e) => return fatal(&format!("discovery failed: {e}")),
      };
      let probe = match build_probe() {
            Ok(p) => p,
            Err(e) => return fatal(&format!("probe build failed: {e}")),
      };
      let mut tests: Vec<Test> = Vec::new();
      for (pkg, target, exe, cwd) in &binaries {
            let (names, ignored) = match list_tests(exe, cwd) {
                  Ok(v) => v,
                  Err(e) => return fatal(&format!("--list failed for {}: {e}", exe.display())),
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
      let colliding: Vec<&str> = tests
            .iter()
            .filter(|t| t.id.starts_with("recipe/") && t.id.contains("all"))
            .map(|t| t.id.as_str())
            .collect();
      if !colliding.is_empty() {
            return fatal(&format!(
                  "root-package test ids may not contain \"all\" (cargo filter collision): {}",
                  colliding.join(" ")
            ));
      }
      if let Some(f) = &filter {
            tests.retain(|t| t.id.contains(f.as_str()));
      }
      tests.sort_by(|a, b| a.id.cmp(&b.id));
      let discovered = tests.len();
      let width = tests.iter().map(|t| t.id.len()).max().unwrap_or(0);

      let mut log = Log {
            f: File::create(Path::new(ROOT).join("suite.log")).expect("create suite.log"),
      };
      let head = format!("[S] discovered={} binaries={}", discovered, binaries.len());
      log.file(&head);
      out(&head);
      let mut tally = Tally { discovered, passed: 0, failed: 0, unattempted: 0 };
      if discovered == 0 {
            return finish(&mut log, &tally, t0, Instant::now(), 0.0);
      }

      // One lease over the whole suite. Every test process links gpu-core and
      // would take this lock at its first device touch; instead they inherit
      // ours through RECIPE_GPU_LOCK_FD and no daemon job can slip between two
      // tests. A daemon that wants the GPU queues in the kernel until we exit,
      // so the suite no longer stops anyone's service to get the card.
      let waited = Instant::now();
      gpu_core::gate::acquire();
      log.line(&format!("[S] GPU-LOCK acquired wait={:.1}s", waited.elapsed().as_secs_f64()));
      // Processes already holding /dev/kfd open cannot use it while we hold the
      // lease; only a NEW holder means a test leaked a GPU child.
      let known = kfd_holders();
      let mut done = vec![false; tests.len()];
      let dispatch_t0 = Instant::now();
      let mut test_secs = 0.0f64;

      let ended = std::panic::catch_unwind(AssertUnwindSafe(|| {
            dispatch(&mut log, &probe, &tests, &known, &mut done, &mut tally, &mut test_secs, width)
      }));
      let abort = match ended {
            Ok(Some(reason)) => reason,
            Ok(None) => "suite ended early".to_owned(),
            Err(p) => {
                  let reason = oneline(&format!("suite panicked: {}", panic_msg(&*p)));
                  log.line(&format!("[S] SUITE-PANIC {reason}"));
                  reason
            }
      };
      fail_rest(&mut log, &mut tally, &tests, &done, &oneline(&abort), width);
      finish(&mut log, &tally, t0, dispatch_t0, test_secs)
}

fn dispatch(
      log: &mut Log,
      probe: &Path,
      tests: &[Test],
      known: &[i32],
      done: &mut [bool],
      tally: &mut Tally,
      test_secs: &mut f64,
      width: usize,
) -> Option<String> {
      let mut prev_violent: Option<String> = None;
      let mut rerun: Vec<(usize, Attempt, String)> = Vec::new();
      let mut abort: Option<String> = None;

      for (i, t) in tests.iter().enumerate() {
            if t.ignored {
                  log.line(&test_line("FAIL", &t.id, 0.0, width));
                  log.line("     ignored");
                  tally.failed += 1;
                  done[i] = true;
                  prev_violent = None;
                  continue;
            }
            let leaked = settled_holders(log, known);
            if !leaked.is_empty() {
                  log.line(&format!("[S] DEVICE-BUSY before={} pids={leaked:?}", t.id));
                  abort = Some(format!("device-busy pids={leaked:?}"));
                  break;
            }
            let att = run_test(t);
            let violent = matches!(att.outcome, Outcome::Signal(_) | Outcome::Deadline);
            if let Some(culprit) = prev_violent.clone().filter(|_| att.outcome != Outcome::Pass) {
                  rerun.push((i, att, culprit));
                  prev_violent = if violent { Some(t.id.clone()) } else { None };
                  if violent && !probe_ok(probe) {
                        abort = Some(poison(log, &t.id));
                        break;
                  }
                  continue;
            }
            record(log, t, width, &att, tally, test_secs, None);
            done[i] = true;
            prev_violent = if violent { Some(t.id.clone()) } else { None };
            if violent && !probe_ok(probe) {
                  abort = Some(poison(log, &t.id));
                  break;
            }
      }

      if abort.is_none() {
            for (i, first, culprit) in &rerun {
                  let t = &tests[*i];
                  let leaked = settled_holders(log, known);
                  if !leaked.is_empty() {
                        log.line(&format!("[S] DEVICE-BUSY before={} pids={leaked:?}", t.id));
                        abort = Some(format!("device-busy pids={leaked:?}"));
                        break;
                  }
                  let att = run_test(t);
                  let violent = matches!(att.outcome, Outcome::Signal(_) | Outcome::Deadline);
                  record(log, t, width, &att, tally, test_secs, Some((first, culprit.as_str())));
                  done[*i] = true;
                  if violent && !probe_ok(probe) {
                        abort = Some(poison(log, &t.id));
                        break;
                  }
            }
      }
      abort
}

fn poison(log: &mut Log, after: &str) -> String {
      log.line(&format!("[S] DEVICE-POISONED after={after}"));
      format!("device-poisoned after={after}")
}

fn oneline(s: &str) -> String {
      s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fail_rest(log: &mut Log, tally: &mut Tally, tests: &[Test], done: &[bool], abort: &str, width: usize) {
      for (i, t) in tests.iter().enumerate() {
            if !done.get(i).copied().unwrap_or(false) {
                  log.line(&test_line("FAIL", &t.id, 0.0, width));
                  log.line(&format!("     not attempted: {abort}"));
                  tally.unattempted += 1;
            }
      }
}

fn fatal(msg: &str) -> i32 {
      out(&format!("FAIL: {msg}"));
      errline(&format!("FAIL: {msg}"));
      1
}

fn new_holders(known: &[i32]) -> Vec<i32> {
      kfd_holders().into_iter().filter(|p| !known.contains(p)).collect()
}

fn settled_holders(log: &mut Log, known: &[i32]) -> Vec<i32> {
      let first = new_holders(known);
      if first.is_empty() {
            return first;
      }
      log.line(&format!("[S] DEVICE-WAIT pids={first:?}"));
      let t0 = Instant::now();
      loop {
            let leaked = new_holders(known);
            if leaked.is_empty() {
                  log.line(&format!("[S] DEVICE-CLEAR waited={:.1}s", t0.elapsed().as_secs_f64()));
                  return leaked;
            }
            if t0.elapsed().as_secs_f64() >= DEVICE_FREE_DEADLINE_SECS {
                  return leaked;
            }
            std::thread::sleep(Duration::from_millis(50));
      }
}

fn record(
      log: &mut Log,
      t: &Test,
      width: usize,
      att: &Attempt,
      tally: &mut Tally,
      test_secs: &mut f64,
      first_try: Option<(&Attempt, &str)>,
) {
      *test_secs += att.secs;
      match &att.outcome {
            Outcome::Pass => {
                  log.line(&test_line("PASS", &t.id, att.secs, width));
                  tally.passed += 1;
            }
            _ => {
                  log.line(&test_line("FAIL", &t.id, att.secs, width));
                  tally.failed += 1;
                  for line in details(&t.name, att) {
                        log.line(&format!("     {line}"));
                  }
            }
      }
      if let Some((first, culprit)) = first_try {
            log.line(&format!(
                  "     first try: FAIL {:.1}s {} suspect-contamination culprit={culprit}",
                  first.secs,
                  reason(&first.outcome)
            ));
            for line in capture_lines(&t.name, &first.capture) {
                  log.line(&format!("     first try: {line}"));
            }
      }
}

fn test_line(verdict: &str, id: &str, secs: f64, width: usize) -> String {
      format!("{verdict} {id:<width$} {dur:>5}", dur = format!("{secs:.1}s"))
}

fn details(name: &str, att: &Attempt) -> Vec<String> {
      let mut lines = capture_lines(name, &att.capture);
      match &att.outcome {
            Outcome::Signal(_) | Outcome::Deadline => lines.insert(0, reason(&att.outcome)),
            _ if lines.is_empty() => lines.push(reason(&att.outcome)),
            _ => {}
      }
      lines
}

fn capture_lines(name: &str, capture: &[String]) -> Vec<String> {
      capture
            .iter()
            .filter(|l| !boilerplate(name, l))
            .map(|l| l.trim_end().to_owned())
            .collect()
}

fn boilerplate(name: &str, line: &str) -> bool {
      let t = line.trim();
      t.is_empty()
            || t == name
            || t == "failures:"
            || t.starts_with("running ")
            || (t.starts_with("test ") && t.contains(" ... "))
            || t.starts_with("test result:")
            || t.starts_with("note: run with `RUST_BACKTRACE")
            || (t.starts_with("---- ") && t.ends_with(" ----"))
            || (t.starts_with("thread '") && t.contains("panicked at"))
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
      tally: &Tally,
      t0: Instant,
      dispatch_t0: Instant,
      test_secs: f64,
) -> i32 {
      let wall = t0.elapsed().as_secs_f64();
      let dispatch_secs = dispatch_t0.elapsed().as_secs_f64();
      if dispatch_secs > WALL_BUDGET_SECS {
            log.line(&format!(
                  "[S] WALL-OVER dispatch={dispatch_secs:.1}s budget={WALL_BUDGET_SECS:.1}s"
            ));
      }
      let mut faults: Vec<String> = Vec::new();
      if tally.discovered == 0 {
            faults.push("no tests discovered, nothing ran".to_owned());
      }
      if tally.passed + tally.failed + tally.unattempted != tally.discovered {
            faults.push(format!(
                  "accounting bug: passed={} + failed={} + unattempted={} != discovered={}",
                  tally.passed, tally.failed, tally.unattempted, tally.discovered
            ));
      }
      if tally.unattempted > 0 {
            faults.push(format!(
                  "{} tests not attempted (discovered={}, ran={})",
                  tally.unattempted,
                  tally.discovered,
                  tally.ran()
            ));
      }
      match std::fs::read_to_string(Path::new(ROOT).join("suite.log")) {
            Ok(text) => {
                  if let Some(bad) = text.lines().find(|l| !grammar_ok(l)) {
                        faults.push(format!("log grammar violation: {bad:?}"));
                  }
            }
            Err(e) => faults.push(format!("cannot reread suite.log: {e}")),
      }
      let code = if faults.is_empty() && tally.green() { 0 } else { 1 };
      for f in &faults {
            log.file(&format!("[S] FAIL: {f}"));
            out(&format!("FAIL: {f}"));
      }
      log.file(&format!(
            "[S] passed={} failed={} unattempted={} discovered={} wall={wall:.1}s overhead={:.1}s exit={code}",
            tally.passed,
            tally.failed,
            tally.unattempted,
            tally.discovered,
            wall - test_secs
      ));
      let mut summary = format!("{}/{} passed.", tally.passed, tally.discovered);
      if tally.failed_total() > 0 {
            summary.push_str(&format!(" {} FAILED.", tally.failed_total()));
      }
      log.file("");
      log.file(&summary);
      out("");
      out(&summary);
      code
}

fn grammar_ok(line: &str) -> bool {
      if line.is_empty()
            || line.starts_with("[S] ")
            || line.starts_with("PASS ")
            || line.starts_with("FAIL ")
      {
            return true;
      }
      if let Some(rest) = line.strip_prefix("     ") {
            return !rest.trim().is_empty();
      }
      summary_ok(line)
}

fn summary_ok(line: &str) -> bool {
      let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
      let Some((frac, rest)) = line.split_once(" passed.") else {
            return false;
      };
      let Some((p, d)) = frac.split_once('/') else {
            return false;
      };
      if !digits(p) || !digits(d) {
            return false;
      }
      match rest.strip_prefix(' ').and_then(|r| r.strip_suffix(" FAILED.")) {
            None => rest.is_empty(),
            Some(n) => digits(n),
      }
}

fn await_kfd_teardown(pid: u32) {
      let path = format!("/sys/class/kfd/kfd/proc/{pid}");
      let t0 = Instant::now();
      while Path::new(&path).exists() {
            if t0.elapsed().as_secs_f64() >= KFD_TEARDOWN_DEADLINE_SECS {
                  errline(&format!("[RUN] kfd teardown of pid {pid} still live after {:.0}s — proceeding", t0.elapsed().as_secs_f64()));
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
                              errline(&format!("[RUN] {} {}s", t.id, next_tick));
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
