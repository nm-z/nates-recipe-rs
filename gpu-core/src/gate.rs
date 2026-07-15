use crate::log::Write;
use std::cmp;
use std::env;
use std::fs;
use std::io;
use std::io::{Read, Seek, Write as _};
use std::marker::PhantomData;
use std::num;
use std::os::fd::{AsRawFd, RawFd};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time;

const LOCK_NAME: &str = "recipe-gpu.lock";
const INHERIT_VAR: &str = "RECIPE_GPU_LOCK_FD";
const KFD_PROC: &str = "/sys/class/kfd/kfd/proc";
const TEARDOWN_DEADLINE_SECS: f64 = 30.0;
const RECLAIM_STEP: time::Duration = time::Duration::from_millis(25);
const STAMP_WIDTH: usize = 20;
const CLEAN: u32 = 0;

enum Lock {
	Closed,
	Own(fs::File),
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

static GATE: Mutex<Gate> = Mutex::new(Gate {
	lock: Lock::Closed,
	grip: Grip::Free,
});
static HOLDING: AtomicU32 = AtomicU32::new(CLEAN);

fn locked() -> MutexGuard<'static, Gate> {
	match GATE.lock() {
		Ok(g) => g,
		Err(poisoned) => poisoned.into_inner(),
	}
}

fn inherited() -> io::Result<Option<RawFd>> {
	let Ok(raw) = env::var(INHERIT_VAR) else {
		return Ok(None);
	};
	let fd: RawFd = raw
		.parse()
		.map_err(|_e| io::Error::other(format!("{INHERIT_VAR}={raw}")))?;
	match unsafe { libc::fcntl(fd, libc::F_GETFD) }.cmp(&0) {
		cmp::Ordering::Less => Err(io::Error::other(format!(
			"{INHERIT_VAR}={fd}: {}",
			io::Error::last_os_error()
		))),
		cmp::Ordering::Equal => Ok(Some(fd)),
		cmp::Ordering::Greater => Ok(Some(fd)),
	}
}

fn open_lock() -> io::Result<fs::File> {
	let uid = unsafe { libc::getuid() };
	let dir = env::var_os("XDG_RUNTIME_DIR")
		.filter(|v| !v.is_empty())
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from(format!("/run/user/{uid}")));
	fs::create_dir_all(&dir)
		.map_err(|e| io::Error::other(format!("create {}: {e}", dir.display())))?;
	let path = dir.join(LOCK_NAME);
	let f = fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(false)
		.open(&path)
		.map_err(|e| io::Error::other(format!("open {}: {e}", path.display())))?;
	let fd = f.as_raw_fd();
	let None = num::NonZeroI32::new(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }) else {
		return Err(io::Error::last_os_error());
	};
	unsafe { env::set_var(INHERIT_VAR, fd.to_string()) };
	Ok(f)
}

fn flock(fd: RawFd, op: i32) -> io::Result<()> {
	loop {
		match num::NonZeroI32::new(unsafe { libc::flock(fd, op) }) {
			None => return Ok(()),
			Some(_rc) => {
				let e = io::Error::last_os_error();
				match Some(()).filter(|_u| e.kind() == io::ErrorKind::Interrupted) {
					None => return Err(e),
					Some(()) => continue,
				}
			}
		}
	}
}

fn holder_pid(f: &mut fs::File) -> Option<u32> {
	let mut s = String::new();
	f.rewind().ok()?;
	f.read_to_string(&mut s).ok()?;
	s.trim().parse().ok()
}

fn stamp(f: &mut fs::File, pid: u32) -> io::Result<()> {
	f.rewind()?;
	write!(f, "{pid:<w$}", w = STAMP_WIDTH)?;
	f.flush()
}

fn take(f: &mut fs::File) -> io::Result<()> {
	let fd = f.as_raw_fd();
	match flock(fd, libc::LOCK_EX | libc::LOCK_NB) {
		Ok(()) => Ok(()),
		Err(e) => contend(fd, f, e),
	}?;
	let me = process::id();
	let prev = holder_pid(f).filter(|p| *p != CLEAN && *p != me);
	stamp(f, me)?;
	for pid in prev.into_iter() {
		await_teardown(pid);
	}
	Ok(())
}

fn contend(fd: RawFd, f: &mut fs::File, e: io::Error) -> io::Result<()> {
	let kind = e.kind();
	match Some(()).filter(|_u| kind == io::ErrorKind::WouldBlock) {
		None => Err(e),
		Some(()) => {
			match holder_pid(f).filter(|p| *p != CLEAN) {
				Some(pid) => {
					Write::wait(format!("Waiting for pid {pid} to release the GPU"))
				}
				None => Write::wait("Waiting for the GPU lock"),
			}
			let got = flock(fd, libc::LOCK_EX);
			Write::unwait();
			got
		}
	}
}

fn await_teardown(pid: u32) {
	let path = PathBuf::from(KFD_PROC).join(pid.to_string());
	let t0 = time::Instant::now();
	let parked = path.exists();
	match parked {
		true => Write::wait(format!("Waiting for pid {pid} gpu teardown")),
		false => {}
	}
	let mut wedged = false;
	while path.exists() {
		wedged = expired(t0).is_some();
		match wedged {
			true => break,
			false => thread::sleep(time::Duration::from_millis(2)),
		}
	}
	match parked {
		true => Write::unwait(),
		false => {}
	}
	match wedged {
		true => return overstayed(pid, t0),
		false => {}
	}
	let Some(mut free) = crate::hip::sysfs_vram_free() else {
		return;
	};
	loop {
		thread::sleep(RECLAIM_STEP);
		let Some(now) = crate::hip::sysfs_vram_free() else {
			return;
		};
		match now.cmp(&free) {
			cmp::Ordering::Greater => free = now,
			cmp::Ordering::Equal => return,
			cmp::Ordering::Less => return,
		}
		let None = expired(t0) else {
			return overstayed(pid, t0);
		};
	}
}

fn expired(t0: time::Instant) -> Option<()> {
	let waited = t0.elapsed().as_secs_f64();
	match waited.partial_cmp(&TEARDOWN_DEADLINE_SECS) {
		Some(cmp::Ordering::Less) | None => None,
		Some(cmp::Ordering::Equal) | Some(cmp::Ordering::Greater) => Some(()),
	}
}

fn overstayed(pid: u32, t0: time::Instant) {
	drop(Write::err(&format!(
		"gpu gate: pid {pid} still holds the device after {:.0}s — proceeding",
		t0.elapsed().as_secs_f64()
	)));
}

fn engage(g: &mut Gate) -> io::Result<()> {
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

fn open_or_adopt() -> io::Result<Lock> {
	match inherited()? {
		Some(_fd) => Ok(Lock::Adopted),
		None => Ok(Lock::Own(open_lock()?)),
	}
}

pub fn acquire() {
	let None = num::NonZeroU32::new(HOLDING.load(Ordering::Acquire)) else {
		return;
	};
	let mut g = locked();
	match g.grip {
		Grip::Taken => {}
		Grip::Free => mark_taken(&mut g),
	}
}

fn mark_taken(g: &mut Gate) {
	match engage(g) {
		Ok(()) => {
			g.grip = Grip::Taken;
			HOLDING.store(process::id(), Ordering::Release);
		}
		Err(e) => {
			drop(Write::err(&format!("gpu gate: {e}")));
			process::abort();
		}
	}
}

pub fn release() {
	let mut g = locked();
	match g.grip {
		Grip::Free => {}
		Grip::Taken => shutdown(&mut g),
	}
}

fn shutdown(g: &mut Gate) {
	let Lock::Own(f) = &mut g.lock else { return };
	for e in stamp(f, CLEAN).err().into_iter() {
		drop(Write::err(&format!("gpu gate: clean stamp: {e}")));
	}
	for e in flock(f.as_raw_fd(), libc::LOCK_UN).err().into_iter() {
		drop(Write::err(&format!("gpu gate: unlock: {e}")));
	}
	g.grip = Grip::Free;
	HOLDING.store(CLEAN, Ordering::Release);
}

pub struct Lease {
	_p: PhantomData<()>,
}

impl Default for Lease {
	fn default() -> Lease {
		Lease::new()
	}
}

impl Lease {
	pub fn new() -> Lease {
		acquire();
		Lease { _p: PhantomData }
	}
}

impl Drop for Lease {
	fn drop(&mut self) {
		release();
	}
}
