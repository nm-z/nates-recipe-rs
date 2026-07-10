use std::io::{Read, Seek, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicU32, Ordering};
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

#[derive(Clone, Copy)]
enum Grip {
	Free,
	Taken,
}

struct Gate {
	lock: Lock,
	grip: Grip,
}

static GATE: Mutex<Gate> = Mutex::new(Gate { lock: Lock::Closed, grip: Grip::Free });
static HOLDING: AtomicU32 = AtomicU32::new(CLEAN);

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
		.map_err(|_e| std::io::Error::other(format!("{INHERIT_VAR}={raw}")))?;
	match unsafe { libc::fcntl(fd, libc::F_GETFD) }.cmp(&0) {
		std::cmp::Ordering::Less => Err(std::io::Error::other(format!(
			"{INHERIT_VAR}={fd}: {}",
			std::io::Error::last_os_error()
		))),
		std::cmp::Ordering::Equal => Ok(Some(fd)),
		std::cmp::Ordering::Greater => Ok(Some(fd)),
	}
}

fn open_lock() -> std::io::Result<std::fs::File> {
	let uid = unsafe { libc::getuid() };
	let path = std::path::PathBuf::from(format!("/run/user/{uid}")).join(LOCK_NAME);
	let f = std::fs::OpenOptions::new()
		.read(0 == 0)
		.write(0 == 0)
		.create(0 == 0)
		.truncate(0 != 0)
		.open(&path)
		.map_err(|e| std::io::Error::other(format!("open {}: {e}", path.display())))?;
	let fd = f.as_raw_fd();
	let None = std::num::NonZeroI32::new(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }) else {
		return Err(std::io::Error::last_os_error());
	};
	unsafe { std::env::set_var(INHERIT_VAR, fd.to_string()) };
	Ok(f)
}

fn flock(fd: RawFd, op: i32) -> std::io::Result<()> {
	loop {
		match std::num::NonZeroI32::new(unsafe { libc::flock(fd, op) }) {
			None => return Ok(()),
			Some(_rc) => {
				let e = std::io::Error::last_os_error();
				match Some(()).filter(|_u| e.kind() == std::io::ErrorKind::Interrupted) {
					None => return Err(e),
					Some(()) => continue,
				}
			}
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
	match flock(fd, libc::LOCK_EX | libc::LOCK_NB) {
		Ok(()) => Ok(()),
		Err(e) => contend(fd, f, e),
	}?;
	let me = std::process::id();
	let prev = holder_pid(f).filter(|p| *p != CLEAN && *p != me);
	stamp(f, me)?;
	for pid in prev.into_iter() {
		await_teardown(pid);
	}
	Ok(())
}

fn contend(fd: RawFd, f: &mut std::fs::File, e: std::io::Error) -> std::io::Result<()> {
	let kind = e.kind();
	match Some(()).filter(|_u| kind == std::io::ErrorKind::WouldBlock) {
		None => Err(e),
		Some(()) => {
			match holder_pid(f) {
				Some(pid) => eprintln!("gpu gate: queued behind pid {pid}"),
				None => eprintln!("gpu gate: queued behind the current holder"),
			}
			flock(fd, libc::LOCK_EX)
		}
	}
}

fn await_teardown(pid: u32) {
	let path = std::path::PathBuf::from(KFD_PROC).join(pid.to_string());
	let t0 = std::time::Instant::now();
	while path.exists() {
		let None = expired(t0, pid) else { return };
		std::thread::sleep(std::time::Duration::from_millis(2));
	}
	let Some(mut free) = crate::hip::sysfs_vram_free() else { return };
	loop {
		std::thread::sleep(RECLAIM_STEP);
		let Some(now) = crate::hip::sysfs_vram_free() else { return };
		match now.cmp(&free) {
			std::cmp::Ordering::Greater => free = now,
			std::cmp::Ordering::Equal => return,
			std::cmp::Ordering::Less => return,
		}
		let None = expired(t0, pid) else { return };
	}
}

fn expired(t0: std::time::Instant, pid: u32) -> Option<()> {
	let waited = t0.elapsed().as_secs_f64();
	match waited.partial_cmp(&TEARDOWN_DEADLINE_SECS) {
		Some(std::cmp::Ordering::Less) | None => None,
		Some(std::cmp::Ordering::Equal) | Some(std::cmp::Ordering::Greater) => {
			eprintln!("gpu gate: pid {pid} still holds the device after {waited:.0}s — proceeding");
			Some(())
		}
	}
}

fn engage(g: &mut Gate) -> std::io::Result<()> {
	let fresh = match &g.lock {
		Lock::Closed => Some(open_or_adopt()?),
		Lock::Own(_own) => None,
		Lock::Adopted => None,
	};
	for lock in fresh.into_iter() {
		g.lock = lock;
	}
	match &mut g.lock {
		Lock::Own(f) => take(f),
		Lock::Adopted => Ok(()),
		Lock::Closed => Ok(()),
	}
}

fn open_or_adopt() -> std::io::Result<Lock> {
	match inherited()? {
		Some(_fd) => Ok(Lock::Adopted),
		None => Ok(Lock::Own(open_lock()?)),
	}
}

pub fn acquire() {
	let None = std::num::NonZeroU32::new(HOLDING.load(Ordering::Acquire)) else { return };
	let mut g = locked();
	match g.grip {
		Grip::Taken => return,
		Grip::Free => mark_taken(&mut g),
	}
}

fn mark_taken(g: &mut Gate) {
	match engage(g) {
		Ok(()) => {
			g.grip = Grip::Taken;
			HOLDING.store(std::process::id(), Ordering::Release);
		}
		Err(e) => panic!("gpu gate: {e}"),
	}
}

pub fn release() {
	let mut g = locked();
	match g.grip {
		Grip::Free => return,
		Grip::Taken => shutdown(&mut g),
	}
}

fn shutdown(g: &mut Gate) {
	let Lock::Own(f) = &mut g.lock else { return };
	for e in stamp(f, CLEAN).err().into_iter() {
		eprintln!("gpu gate: clean stamp: {e}");
	}
	for e in flock(f.as_raw_fd(), libc::LOCK_UN).err().into_iter() {
		eprintln!("gpu gate: unlock: {e}");
	}
	g.grip = Grip::Free;
	HOLDING.store(CLEAN, Ordering::Release);
}

pub struct Lease {
	_p: std::marker::PhantomData<()>,
}

impl Default for Lease {
	fn default() -> Lease {
		Lease::new()
	}
}

impl Lease {
	pub fn new() -> Lease {
		acquire();
		Lease { _p: std::marker::PhantomData }
	}
}

impl Drop for Lease {
	fn drop(&mut self) {
		release();
	}
}
