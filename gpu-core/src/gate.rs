use std::io::{Read, Seek, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

const LOCK_NAME: &str = "recipe-gpu.lock";
const INHERIT_VAR: &str = "RECIPE_GPU_LOCK_FD";
const KFD_PROC: &str = "/sys/class/kfd/kfd/proc";
const TEARDOWN_DEADLINE_SECS: f64 = 30.0;
const RECLAIM_STEP: std::time::Duration = std::time::Duration::from_millis(25);
const STAMP_WIDTH: usize = 20;
const CLEAN: u32 = 0;

enum Lock {
	Closed,
	Own(std::fs::File),
	Adopted,
}

struct Gate {
	lock: Lock,
	holding: bool,
}

static GATE: Mutex<Gate> = Mutex::new(Gate { lock: Lock::Closed, holding: false });
static HOLDING: AtomicBool = AtomicBool::new(false);

fn locked() -> MutexGuard<'static, Gate> {
	match GATE.lock() {
		Ok(g) => g,
		Err(poisoned) => poisoned.into_inner(),
	}
}

fn inherited() -> std::io::Result<Option<RawFd>> {
	let Ok(raw) = std::env::var(INHERIT_VAR) else { return Ok(None) };
	let fd: RawFd = raw
		.parse()
		.map_err(|_| std::io::Error::other(format!("{INHERIT_VAR}={raw}")))?;
	if unsafe { libc::fcntl(fd, libc::F_GETFD) } < 0 {
		return Err(std::io::Error::other(format!(
			"{INHERIT_VAR}={fd}: {}",
			std::io::Error::last_os_error()
		)));
	}
	Ok(Some(fd))
}

fn open_lock() -> std::io::Result<std::fs::File> {
	let uid = unsafe { libc::getuid() };
	let path = std::path::PathBuf::from(format!("/run/user/{uid}")).join(LOCK_NAME);
	let f = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(false)
		.open(&path)
		.map_err(|e| std::io::Error::other(format!("open {}: {e}", path.display())))?;
	let fd = f.as_raw_fd();
	if unsafe { libc::fcntl(fd, libc::F_SETFD, 0) } < 0 {
		return Err(std::io::Error::last_os_error());
	}
	unsafe { std::env::set_var(INHERIT_VAR, fd.to_string()) };
	Ok(f)
}

fn flock(fd: RawFd, op: i32) -> std::io::Result<()> {
	loop {
		if unsafe { libc::flock(fd, op) } == 0 {
			return Ok(());
		}
		let e = std::io::Error::last_os_error();
		if e.kind() != std::io::ErrorKind::Interrupted {
			return Err(e);
		}
	}
}

fn holder_pid(f: &mut std::fs::File) -> Option<u32> {
	let mut s = String::new();
	f.rewind().ok()?;
	f.read_to_string(&mut s).ok()?;
	s.trim().parse().ok()
}

fn stamp(f: &mut std::fs::File, pid: u32) -> std::io::Result<()> {
	f.rewind()?;
	write!(f, "{pid:<w$}", w = STAMP_WIDTH)?;
	f.flush()
}

fn take(f: &mut std::fs::File) -> std::io::Result<()> {
	let fd = f.as_raw_fd();
	if let Err(e) = flock(fd, libc::LOCK_EX | libc::LOCK_NB) {
		if e.kind() != std::io::ErrorKind::WouldBlock {
			return Err(e);
		}
		match holder_pid(f) {
			Some(pid) => eprintln!("gpu gate: queued behind pid {pid}"),
			None => eprintln!("gpu gate: queued behind the current holder"),
		}
		flock(fd, libc::LOCK_EX)?;
	}
	let me = std::process::id();
	let prev = holder_pid(f).filter(|p| *p != CLEAN && *p != me);
	stamp(f, me)?;
	if let Some(pid) = prev {
		await_teardown(pid);
	}
	Ok(())
}

fn await_teardown(pid: u32) {
	let path = std::path::PathBuf::from(KFD_PROC).join(pid.to_string());
	let t0 = std::time::Instant::now();
	while path.exists() {
		if expired(t0, pid) {
			return;
		}
		std::thread::sleep(std::time::Duration::from_millis(2));
	}
	let Some(mut free) = crate::hip::sysfs_vram_free() else { return };
	loop {
		std::thread::sleep(RECLAIM_STEP);
		match crate::hip::sysfs_vram_free() {
			Some(now) if now > free => free = now,
			_ => return,
		}
		if expired(t0, pid) {
			return;
		}
	}
}

fn expired(t0: std::time::Instant, pid: u32) -> bool {
	let waited = t0.elapsed().as_secs_f64();
	if waited >= TEARDOWN_DEADLINE_SECS {
		eprintln!("gpu gate: pid {pid} still holds the device after {waited:.0}s — proceeding");
		return true;
	}
	false
}

fn engage(g: &mut Gate) -> std::io::Result<()> {
	if let Lock::Closed = g.lock {
		g.lock = match inherited()? {
			Some(_) => Lock::Adopted,
			None => Lock::Own(open_lock()?),
		};
	}
	if let Lock::Own(f) = &mut g.lock {
		take(f)?;
	}
	Ok(())
}

pub fn acquire() {
	if HOLDING.load(Ordering::Acquire) {
		return;
	}
	let mut g = locked();
	if g.holding {
		return;
	}
	if let Err(e) = engage(&mut g) {
		panic!("gpu gate: {e}");
	}
	g.holding = true;
	HOLDING.store(true, Ordering::Release);
}

pub fn release() {
	let mut g = locked();
	if !g.holding {
		return;
	}
	if let Lock::Own(f) = &mut g.lock {
		if let Err(e) = stamp(f, CLEAN) {
			eprintln!("gpu gate: clean stamp: {e}");
		}
		if let Err(e) = flock(f.as_raw_fd(), libc::LOCK_UN) {
			eprintln!("gpu gate: unlock: {e}");
		}
		g.holding = false;
		HOLDING.store(false, Ordering::Release);
	}
}

pub struct Lease(());

impl Default for Lease {
	fn default() -> Lease {
		Lease::new()
	}
}

impl Lease {
	pub fn new() -> Lease {
		acquire();
		Lease(())
	}
}

impl Drop for Lease {
	fn drop(&mut self) {
		release();
	}
}
