// Suite orchestrator (SUITE SPEC v3). `cargo test all` at repo root runs every
// test in every workspace crate, each as its own OS process, 60s deadline per
// test, one structured log at <root>/suite.log. See spec: R1-R9.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

const TEST_DEADLINE_SECS: f64 = 60.0;
const WALL_BUDGET_SECS: f64 = 600.0;
const PROBE_DEADLINE_SECS: f64 = 10.0;
const KFD_TEARDOWN_DEADLINE_SECS: f64 = 30.0;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

struct Test {
      id: String, // crate/target/name
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
      // every log line also streams to stderr the moment it is written
      fn line(&mut self, s: &str) {
            writeln!(self.f, "{s}").expect("suite.log write");
            self.f.flush().expect("suite.log flush");
            eprintln!("{s}");
      }
}

// pgid of the currently-running test child (== its pid via setsid); 0 = none.
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
      let code = run();
      // EXIT trap: no child survives the suite, whatever the exit path
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

      // ── discovery ──────────────────────────────────────────────────────────
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
            return finish(&mut log, 0, 0, t0, 0.0, 3);
      }

      // ── dispatch ───────────────────────────────────────────────────────────
      let (mut passed, mut failed) = (0usize, 0usize);
      let mut test_secs = 0.0f64;
      let mut prev_violent: Option<String> = None; // id of the crash/kill just before
      let mut rerun: Vec<(usize, Attempt, String)> = Vec::new(); // (test idx, first try, culprit)
      let mut poisoned: Option<String> = None;
      let mut busy_abort = false;

      for (i, t) in tests.iter().enumerate() {
            if t.ignored {
                  log.line(&format!("[FAIL] {} 0.0 ignored", t.id));
                  failed += 1;
                  prev_violent = None;
                  continue;
            }
            let holders = kfd_holders();
            if !holders.is_empty() {
                  log.line(&format!("[S] DEVICE-BUSY before={} pids={holders:?}", t.id));
                  busy_abort = true;
                  break;
            }
            let att = run_test(t);
            let violent = matches!(att.outcome, Outcome::Signal(_) | Outcome::Deadline);
            if let Some(culprit) = prev_violent.clone().filter(|_| att.outcome != Outcome::Pass) {
                  // any FAIL right after a crash/kill is deferred and rerun at suite end
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
            return finish(&mut log, passed, failed, t0, test_secs, 4);
      }

      // ── contamination reruns (fresh process, suite end) ────────────────────
      if poisoned.is_none() {
            for (i, first, culprit) in &rerun {
                  let t = &tests[*i];
                  let holders = kfd_holders();
                  if !holders.is_empty() {
                        log.line(&format!("[S] DEVICE-BUSY before={} pids={holders:?}", t.id));
                        return finish(&mut log, passed, failed, t0, test_secs, 4);
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
            return finish(&mut log, passed, failed, t0, test_secs, 4);
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
      finish(&mut log, passed, failed, t0, test_secs, code)
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
      t0: Instant,
      test_secs: f64,
      code: i32,
) -> i32 {
      let wall = t0.elapsed().as_secs_f64();
      let tail = format!(
            "[S] passed={passed} failed={failed} total={} wall={wall:.1}s overhead={:.1}s exit={code}",
            passed + failed,
            wall - test_secs
      );
      log.line(&tail);
      println!("{tail}");
      // grammar assert: every line matches ^\[(S|PASS|FAIL|E[0-9]+)\]
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
      if line.starts_with("[S] ") || line.starts_with("[PASS] ") || line.starts_with("[FAIL] ") {
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

// ── subprocess execution ────────────────────────────────────────────────────

// Driver-truth spawn gate: a fresh process's first hipMallocAsync spins in HSA
// if it races the predecessor's kernel-side GPU teardown (coredump-proven:
// alloc_bytes_inner → libamdhip64 → libhsa-runtime64 spin, GPU 100%). Wait for
// the dead child's /sys/class/kfd/kfd/proc/<pid> entry to vanish — ~27ms after
// a clean exit, unbounded after SIGKILL, hence the deadline + loud note. This
// is a condition wait on driver state, not sleep-settling.
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
      // children run WITH the 1 GiB pool warm — it commits pages so async
      // copies never fault "page not present" (proven 6/6 vs 1/6 without);
      // load-bearing until the one-claim arena replaces pool growth.
      cmd.args(["--exact", &t.name, "--nocapture"])
            .current_dir(&t.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::from(cap))
            .stderr(Stdio::from(cap2));
      // own session + process group per test: deadline kill reaches grandchildren
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
      let mut next_tick = 10u64; // live timer on stderr for anything slow (log grammar keeps suite.log clean)
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
                              // SIGKILL the whole group, then reap and confirm
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

// item 2: /dev/kfd must be free of any holder before a GPU test spawns; a
// holder here means a previous kill failed to fully clear the device.
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
            .filter(|p| *p != me)
            .collect()
}

// ── discovery plumbing ──────────────────────────────────────────────────────

// (package, target, executable, package dir) for every test binary in the
// workspace, release profile, excluding this orchestrator's own target.
fn discover_binaries() -> Result<Vec<(String, String, PathBuf, PathBuf)>, String> {
      let out = Command::new("cargo")
            .args(["test", "--workspace", "--release", "--no-run", "--message-format=json"])
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
                  continue; // this orchestrator
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

// all test names + which of them are #[ignore], via libtest --list.
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
