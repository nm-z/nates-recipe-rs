#![allow(unsafe_code)]
use std::any;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::panic;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const TEST_DEADLINE_SECS: f64 = 60.0;
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
	hash: Option<u64>,
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
	fn failed_total(&self) -> usize {
		self.failed + self.unattempted
	}

	fn green(&self) -> bool {
		self.discovered > 0 && self.passed == self.discovered
	}
}

fn out(s: &str) {
	let _ = writeln!(io::stdout(), "{s}");
}

fn errline(s: &str) {
	let _ = writeln!(io::stderr(), "{s}");
}

fn tty(fd: i32) -> bool {
	unsafe { libc::isatty(fd) == 1 }
}

fn paint(code: u8, s: &str, on: bool) -> String {
	if on {
		format!("\x1b[{code}m{s}\x1b[0m")
	} else {
		s.to_owned()
	}
}

const GREEN: u8 = 32;
const RED: u8 = 31;
const CYAN: u8 = 36;

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

	fn verdict(&mut self, v: &str, tail: &str) {
		self.file(&format!("{v} {tail}"));
		let code = if v == "PASS" { GREEN } else { RED };
		errline(&format!("{} {tail}", paint(code, v, tty(2))));
	}

	fn status(&mut self, s: &str) {
		self.file(s);
		errline(&paint(CYAN, s, tty(2)));
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
	let code = match panic::catch_unwind(run) {
		Ok(c) => c,
		Err(p) => fatal(&format!("suite panicked: {}", panic_msg(&*p))),
	};
	let pgid = CHILD_PGID.load(Ordering::SeqCst);
	if pgid > 0 {
		unsafe {
			libc::kill(-pgid, libc::SIGKILL);
		}
	}
	process::exit(code);
}

fn acquire_lock() -> Option<File> {
	let f = File::create(Path::new(ROOT).join("target").join(".suite.lock")).ok()?;
	let rc = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
	if rc == 0 { Some(f) } else { None }
}

fn panic_msg(p: &(dyn any::Any + Send)) -> String {
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
	let args: Vec<String> = env::args().skip(1).collect();
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

	errline(&paint(CYAN, "[S] building test binaries (release)", tty(2)));
	let binaries = match discover_binaries() {
		Ok(b) => b,
		Err(e) => return fatal(&format!("discovery failed: {e}")),
	};
	errline(&paint(CYAN, "[S] building probe", tty(2)));
	let probe = match build_probe() {
		Ok(p) => p,
		Err(e) => return fatal(&format!("probe build failed: {e}")),
	};
	errline(&paint(
		CYAN,
		&format!("[S] enumerating {} binaries", binaries.len()),
		tty(2),
	));
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
				hash: None,
			});
		}
	}
	let mut crate_srcs: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
	for t in &mut tests {
		let srcs = crate_srcs
			.entry(t.cwd.clone())
			.or_insert_with(|| crate_source_texts(&t.cwd));
		t.hash = test_fn_hash(srcs, &t.name);
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

	let mut log = Log {
		f: File::create(Path::new(ROOT).join("suite.log")).expect("create suite.log"),
	};
	let head = format!("[S] discovered={} binaries={}", discovered, binaries.len());
	log.file(&head);
	out(&paint(CYAN, &head, tty(1)));
	let mut tally = Tally {
		discovered,
		passed: 0,
		failed: 0,
		unattempted: 0,
	};
	if discovered == 0 {
		return finish(&mut log, &tally, t0, 0.0);
	}

	let waited = Instant::now();
	gpu_core::gate::acquire();
	log.status(&format!(
		"[S] GPU-LOCK acquired wait={:.1}s",
		waited.elapsed().as_secs_f64()
	));
	let known = kfd_holders();
	let mut done = vec![false; tests.len()];
	let mut test_secs = 0.0f64;
	let cache = load_cache();
	let mut passes: Vec<(String, u64)> = Vec::new();

	let ended = panic::catch_unwind(AssertUnwindSafe(|| {
		dispatch(
			&mut log,
			&probe,
			&tests,
			&known,
			&mut done,
			&mut tally,
			&mut test_secs,
			&cache,
			&mut passes,
		)
	}));
	let abort = match ended {
		Ok(Some(reason)) => reason,
		Ok(None) => "suite ended early".to_owned(),
		Err(p) => {
			let reason = oneline(&format!("suite panicked: {}", panic_msg(&*p)));
			log.status(&format!("[S] SUITE-PANIC {reason}"));
			reason
		}
	};
	fail_rest(&mut log, &mut tally, &tests, &done, &oneline(&abort));
	store_cache(cache, &passes);
	finish(&mut log, &tally, t0, test_secs)
}

fn dispatch(
	log: &mut Log,
	probe: &Path,
	tests: &[Test],
	known: &[i32],
	done: &mut [bool],
	tally: &mut Tally,
	test_secs: &mut f64,
	cache: &BTreeMap<String, u64>,
	passes: &mut Vec<(String, u64)>,
) -> Option<String> {
	let mut prev_violent: Option<String> = None;
	let mut rerun: Vec<(usize, Attempt, String)> = Vec::new();
	let mut abort: Option<String> = None;

	for (i, t) in tests.iter().enumerate() {
		if t.ignored {
			log.verdict("FAIL", &test_tail(&t.id, 0.0));
			log.line("     ignored");
			tally.failed += 1;
			done[i] = true;
			prev_violent = None;
			continue;
		}
		if let Some(h) = t.hash
			&& cache.get(&t.id) == Some(&h)
		{
			log.verdict("PASS", &format!("{} cached", test_tail(&t.id, 0.0)));
			tally.passed += 1;
			passes.push((t.id.clone(), h));
			done[i] = true;
			continue;
		}
		let leaked = settled_holders(log, known);
		if !leaked.is_empty() {
			log.status(&format!("[S] DEVICE-BUSY before={} pids={leaked:?}", t.id));
			abort = Some(format!("device-busy pids={leaked:?}"));
			break;
		}
		let att = run_test(t);
		let violent = matches!(att.outcome, Outcome::Signal(_) | Outcome::Deadline);
		if let Some(culprit) = prev_violent
			.clone()
			.filter(|_| att.outcome != Outcome::Pass)
		{
			rerun.push((i, att, culprit));
			prev_violent = if violent { Some(t.id.clone()) } else { None };
			if violent && !probe_ok(probe) {
				abort = Some(poison(log, &t.id));
				break;
			}
			continue;
		}
		record(log, t, &att, tally, test_secs, None);
		if att.outcome == Outcome::Pass
			&& let Some(h) = t.hash
		{
			passes.push((t.id.clone(), h));
		}
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
				log.status(&format!("[S] DEVICE-BUSY before={} pids={leaked:?}", t.id));
				abort = Some(format!("device-busy pids={leaked:?}"));
				break;
			}
			let att = run_test(t);
			let violent = matches!(att.outcome, Outcome::Signal(_) | Outcome::Deadline);
			record(
				log,
				t,
				&att,
				tally,
				test_secs,
				Some((first, culprit.as_str())),
			);
			if att.outcome == Outcome::Pass
				&& let Some(h) = t.hash
			{
				passes.push((t.id.clone(), h));
			}
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
	log.status(&format!("[S] DEVICE-POISONED after={after}"));
	format!("device-poisoned after={after}")
}

fn oneline(s: &str) -> String {
	s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fail_rest(log: &mut Log, tally: &mut Tally, tests: &[Test], done: &[bool], abort: &str) {
	for (i, t) in tests.iter().enumerate() {
		if !done.get(i).copied().unwrap_or(false) {
			log.verdict("FAIL", &test_tail(&t.id, 0.0));
			log.line(&format!("     not attempted: {abort}"));
			tally.unattempted += 1;
		}
	}
}

fn fatal(msg: &str) -> i32 {
	out(&format!("{} {msg}", paint(RED, "FAIL:", tty(1))));
	errline(&format!("{} {msg}", paint(RED, "FAIL:", tty(2))));
	1
}

fn new_holders(known: &[i32]) -> Vec<i32> {
	kfd_holders()
		.into_iter()
		.filter(|p| !known.contains(p))
		.collect()
}

fn settled_holders(log: &mut Log, known: &[i32]) -> Vec<i32> {
	let first = new_holders(known);
	if first.is_empty() {
		return first;
	}
	log.status(&format!("[S] DEVICE-WAIT pids={first:?}"));
	let t0 = Instant::now();
	loop {
		let leaked = new_holders(known);
		if leaked.is_empty() {
			log.status(&format!(
				"[S] DEVICE-CLEAR waited={:.1}s",
				t0.elapsed().as_secs_f64()
			));
			return leaked;
		}
		if t0.elapsed().as_secs_f64() >= DEVICE_FREE_DEADLINE_SECS {
			return leaked;
		}
		thread::sleep(Duration::from_millis(50));
	}
}

fn record(
	log: &mut Log,
	t: &Test,
	att: &Attempt,
	tally: &mut Tally,
	test_secs: &mut f64,
	first_try: Option<(&Attempt, &str)>,
) {
	*test_secs += att.secs;
	match &att.outcome {
		Outcome::Pass => {
			log.verdict("PASS", &test_tail(&t.id, att.secs));
			tally.passed += 1;
		}
		_ => {
			log.verdict("FAIL", &test_tail(&t.id, att.secs));
			tally.failed += 1;
			for line in details(att, &t.name) {
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
		for line in scrub(&first.capture, &t.name) {
			log.line(&format!("     first try: {line}"));
		}
	}
}

fn test_tail(id: &str, secs: f64) -> String {
	format!("{id} {secs:.1}s")
}

fn scrub(capture: &[String], name: &str) -> Vec<String> {
	let bare = name.rsplit("::").next().unwrap_or(name);
	let mut lines: Vec<String> = Vec::new();
	for raw in capture {
		let l = raw.trim_end();
		let t = l.trim_start();
		let ceremony = (t.starts_with("running ")
			&& (t.ends_with(" test") || t.ends_with(" tests")))
			|| t == "failures:"
			|| t.starts_with("test result: ")
			|| t.starts_with("note: run with `RUST_BACKTRACE=1`")
			|| (t.starts_with("test ")
				&& (t.ends_with("... FAILED") || t.ends_with("... ok") || t.ends_with("... ignored")))
			|| t == bare
			|| t == name;
		if ceremony {
			continue;
		}
		if l.is_empty() && lines.last().is_none_or(|p| p.is_empty()) {
			continue;
		}
		lines.push(l.to_owned());
	}
	while lines.last().is_some_and(|l| l.is_empty()) {
		lines.pop();
	}
	lines
}

fn details(att: &Attempt, name: &str) -> Vec<String> {
	let mut lines = scrub(&att.capture, name);
	match &att.outcome {
		Outcome::Signal(_) | Outcome::Deadline => lines.insert(0, reason(&att.outcome)),
		_ if lines.is_empty() => lines.push(reason(&att.outcome)),
		_ => {}
	}
	lines
}

fn reason(o: &Outcome) -> String {
	match o {
		Outcome::Pass => String::new(),
		Outcome::ExitFail(c) => format!("exit={c}"),
		Outcome::Signal(s) => format!("signal={s}"),
		Outcome::Deadline => "deadline".into(),
	}
}

fn finish(log: &mut Log, tally: &Tally, t0: Instant, test_secs: f64) -> i32 {
	let wall = t0.elapsed().as_secs_f64();
	let code = if tally.green() { 0 } else { 1 };
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
	let code_color = if tally.green() { GREEN } else { RED };
	out(&paint(code_color, &summary, tty(1)));
	code
}

fn await_kfd_teardown(pid: u32) {
	let path = format!("/sys/class/kfd/kfd/proc/{pid}");
	let t0 = Instant::now();
	while Path::new(&path).exists() {
		if t0.elapsed().as_secs_f64() >= KFD_TEARDOWN_DEADLINE_SECS {
			errline(&paint(
				CYAN,
				&format!(
					"[RUN] kfd teardown of pid {pid} still live after {:.0}s — proceeding",
					t0.elapsed().as_secs_f64()
				),
				tty(2),
			));
			return;
		}
		thread::sleep(Duration::from_millis(3));
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
					errline(&paint(CYAN, &format!("[RUN] {} {}s", t.id, next_tick), tty(2)));
					next_tick += 10;
				}
				if el >= TEST_DEADLINE_SECS {
					unsafe {
						libc::kill(-(child_pid as i32), libc::SIGKILL);
					}
					child.wait().expect("waitpid after kill");
					break (TEST_DEADLINE_SECS, None);
				}
				thread::sleep(Duration::from_millis(3));
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
		String::from_utf8_lossy(&buf)
			.lines()
			.map(str::to_owned)
			.collect()
	};
	Attempt {
		secs,
		outcome,
		capture,
	}
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
		Err(e) => {
			errline(&format!("spawn test binary: {e}"));
			return false;
		}
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
				thread::sleep(Duration::from_millis(3));
			}
			Err(e) => {
				errline(&format!("wait on test child: {e}"));
				break false;
			}
		}
	};
	CHILD_PGID.store(0, Ordering::SeqCst);
	await_kfd_teardown(pid);
	ok
}

fn kfd_holders() -> Vec<i32> {
	let out = match Command::new("fuser")
		.arg("/dev/kfd")
		.stderr(Stdio::null())
		.output()
	{
		Ok(o) => o,
		Err(e) => {
			errline(&format!("fuser /dev/kfd: {e}"));
			return Vec::new();
		}
	};
	let me = process::id() as i32;
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
		let Some(exe) = v["executable"].as_str() else {
			continue;
		};
		let target = v["target"]["name"].as_str().unwrap_or_default().to_owned();
		let manifest = v["manifest_path"].as_str().unwrap_or_default();
		let pkg = package_name(v["package_id"].as_str().unwrap_or_default(), manifest);
		if pkg == "recipe" && target == "all" {
			continue;
		}
		let cwd = Path::new(manifest)
			.parent()
			.unwrap_or(Path::new(ROOT))
			.to_path_buf();
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
	if let Some(first) = package_id.split_whitespace().next()
		&& !first.is_empty()
	{
		return first.to_owned();
	}
	Path::new(manifest)
		.parent()
		.and_then(|p| p.file_name())
		.map(|s| s.to_string_lossy().into_owned())
		.unwrap_or_else(|| "unknown".into())
}

fn build_probe() -> Result<PathBuf, String> {
	let out = Command::new("cargo")
		.args([
			"build",
			"-p",
			"gpu-core",
			"--release",
			"--bin",
			"probe",
			"--message-format=json",
		])
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
			&& let Some(exe) = v["executable"].as_str()
		{
			return Ok(PathBuf::from(exe));
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

fn cache_path() -> PathBuf {
	Path::new(ROOT).join("target").join(".suite_cache")
}

fn load_cache() -> BTreeMap<String, u64> {
	let mut map = BTreeMap::new();
	if let Ok(text) = fs::read_to_string(cache_path()) {
		for line in text.lines() {
			if let Some((hex, id)) = line.split_once('\t')
				&& let Ok(h) = u64::from_str_radix(hex, 16)
			{
				map.insert(id.to_owned(), h);
			}
		}
	}
	map
}

fn store_cache(mut cache: BTreeMap<String, u64>, passes: &[(String, u64)]) {
	for (id, h) in passes {
		cache.insert(id.clone(), *h);
	}
	let mut text = String::new();
	for (id, h) in &cache {
		text.push_str(&format!("{h:016x}\t{id}\n"));
	}
	fs::write(cache_path(), text).expect("write suite cache");
}

fn crate_source_texts(cwd: &Path) -> Vec<String> {
	let mut files: Vec<PathBuf> = Vec::new();
	collect_rs(cwd, &mut files);
	files.sort();
	files.iter()
		.filter_map(|p| fs::read_to_string(p).ok())
		.collect()
}

fn collect_rs(dir: &Path, files: &mut Vec<PathBuf>) {
	let Ok(rd) = fs::read_dir(dir) else {
		return;
	};
	for e in rd.flatten() {
		let p = e.path();
		let name = e.file_name().to_string_lossy().into_owned();
		if p.is_dir() {
			if name.starts_with('.')
				|| name == "target" || name == "datasets"
				|| p.join("Cargo.toml").exists()
			{
				continue;
			}
			collect_rs(&p, files);
		} else if name.ends_with(".rs") {
			files.push(p);
		}
	}
}

fn test_fn_hash(srcs: &[String], test_name: &str) -> Option<u64> {
	let fn_name = test_name.rsplit("::").next().unwrap_or(test_name);
	let mut h: u64 = 0xcbf2_9ce4_8422_2325;
	let mut found = false;
	for src in srcs {
		for span in fn_spans(src, fn_name) {
			found = true;
			for b in span.as_bytes() {
				h ^= u64::from(*b);
				h = h.wrapping_mul(0x0100_0000_01b3);
			}
		}
	}
	if found { Some(h) } else { None }
}

fn is_ident(b: u8) -> bool {
	b == b'_' || b.is_ascii_alphanumeric()
}

fn fn_spans<'a>(src: &'a str, name: &str) -> Vec<&'a str> {
	let bytes = src.as_bytes();
	let mut spans = Vec::new();
	for (pos, _) in src.match_indices(name) {
		let end = pos + name.len();
		if end < bytes.len() && is_ident(bytes[end]) {
			continue;
		}
		let mut j = pos;
		while j > 0 && bytes[j - 1].is_ascii_whitespace() {
			j -= 1;
		}
		if j < 2 || &src[j - 2..j] != "fn" || j == pos {
			continue;
		}
		if j > 2 && is_ident(bytes[j - 3]) {
			continue;
		}
		if let Some(close) = fn_close(src, end) {
			spans.push(&src[j - 2..close]);
		}
	}
	spans
}

fn fn_close(src: &str, from: usize) -> Option<usize> {
	let cs: Vec<(usize, char)> = src[from..].char_indices().collect();
	let mut i = 0usize;
	let mut depth = 0usize;
	while i < cs.len() {
		match cs[i].1 {
			'{' => {
				depth += 1;
				i += 1;
			}
			'}' => {
				if depth > 0 {
					depth -= 1;
					if depth == 0 {
						return Some(from + cs[i].0 + cs[i].1.len_utf8());
					}
				}
				i += 1;
			}
			'/' if i + 1 < cs.len() && cs[i + 1].1 == '/' => {
				while i < cs.len() && cs[i].1 != '\n' {
					i += 1;
				}
			}
			'/' if i + 1 < cs.len() && cs[i + 1].1 == '*' => {
				let mut lvl = 1usize;
				i += 2;
				while i < cs.len() && lvl > 0 {
					if cs[i].1 == '/' && i + 1 < cs.len() && cs[i + 1].1 == '*' {
						lvl += 1;
						i += 2;
					} else if cs[i].1 == '*' && i + 1 < cs.len() && cs[i + 1].1 == '/' {
						lvl -= 1;
						i += 2;
					} else {
						i += 1;
					}
				}
			}
			'"' => i = skip_str(&cs, i),
			'r' => match raw_str_hashes(&cs, i) {
				Some(n) => i = skip_raw(&cs, i, n),
				None => i += 1,
			},
			'\'' => i = skip_char_or_lifetime(&cs, i),
			_ => i += 1,
		}
	}
	None
}

fn skip_str(cs: &[(usize, char)], start: usize) -> usize {
	let mut i = start + 1;
	while i < cs.len() {
		match cs[i].1 {
			'\\' => i += 2,
			'"' => return i + 1,
			_ => i += 1,
		}
	}
	i
}

fn raw_str_hashes(cs: &[(usize, char)], i: usize) -> Option<usize> {
	if i > 0 {
		let prev = cs[i - 1].1;
		if prev == '_' || prev.is_alphanumeric() {
			return None;
		}
	}
	let mut j = i + 1;
	let mut n = 0usize;
	while j < cs.len() && cs[j].1 == '#' {
		n += 1;
		j += 1;
	}
	if j < cs.len() && cs[j].1 == '"' {
		Some(n)
	} else {
		None
	}
}

fn skip_raw(cs: &[(usize, char)], start: usize, n: usize) -> usize {
	let mut i = start + 1 + n + 1;
	while i < cs.len() {
		if cs[i].1 == '"' && cs[i + 1..].iter().take(n).filter(|c| c.1 == '#').count() == n {
			return i + 1 + n;
		}
		i += 1;
	}
	i
}

fn skip_char_or_lifetime(cs: &[(usize, char)], i: usize) -> usize {
	if i + 1 >= cs.len() {
		return i + 1;
	}
	if cs[i + 1].1 == '\\' {
		match cs.get(i + 2).map(|c| c.1) {
			Some('x') => i + 6,
			Some('u') => {
				let mut j = i + 3;
				while j < cs.len() && cs[j].1 != '\'' {
					j += 1;
				}
				j + 1
			}
			_ => i + 4,
		}
	} else if i + 2 < cs.len() && cs[i + 2].1 == '\'' {
		i + 3
	} else {
		i + 1
	}
}
