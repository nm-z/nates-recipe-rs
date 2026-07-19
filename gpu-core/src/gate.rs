use crate::hip::sysfs_vram_free;
use crate::log::Write;
use core::cmp;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU32, Ordering};
use std::env;
use std::fs;
use std::io;
use std::io::{Seek as _, Write as _};
use std::num;
use std::os::fd::{AsRawFd as _, RawFd};
use std::path::PathBuf;
use std::process;
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
	Own,
	Adopted,
}

#[derive(Clone, Copy)]
enum Grip {
	Free,
	Taken,
}

struct Gate {
	lock: Lock,
	file: Option<fs::File>,
	grip: Grip,
}

static GATE: Mutex<Gate> = Mutex::new(Gate {
	lock: Lock::Closed,
	file: None,
	grip: Grip::Free,
});
static HOLDING: AtomicU32 = AtomicU32::new(CLEAN);

fn locked() -> MutexGuard<'static, Gate> {
	match GATE.lock() {
		Ok(g) => return g,
		Err(poisoned) => return poisoned.into_inner(),
	}
}

fn flock(fd: RawFd, op: i32) -> io::Result<()> {
	loop {
		// SAFETY: flock operates on an owned fd and integer op; no pointers are dereferenced.
		match num::NonZeroI32::new(unsafe { libc::flock(fd, op) }) {
			None => return Ok(()),
			Some(_rc) => {
				let e = io::Error::last_os_error();
				if e.kind() != io::ErrorKind::Interrupted {
					return Err(e);
				}
			}
		}
	}
}

fn holder_pid(f: &mut fs::File) -> Option<u32> {
	f.rewind().ok()?;
	let s = io::read_to_string(&mut *f).ok()?;
	return s.trim().parse().ok();
}

fn stamp(f: &mut fs::File, pid: u32) -> io::Result<()> {
	f.rewind()?;
	write!(f, "{pid:<STAMP_WIDTH$}")?;
	return f.flush();
}

fn expired(t0: time::Instant) -> Option<()> {
	let waited = t0.elapsed().as_secs_f64();
	match waited.partial_cmp(&TEARDOWN_DEADLINE_SECS) {
		Some(cmp::Ordering::Less) | None => return None,
		Some(cmp::Ordering::Equal | cmp::Ordering::Greater) => return Some(()),
	}
}

fn overstayed(pid: u32, t0: time::Instant) {
	Write::error(format!(
		"gpu gate: pid {pid} still holds the device after {:.0}s — proceeding",
		t0.elapsed().as_secs_f64()
	));
}

#[inline]
pub fn acquire() {
	let None = num::NonZeroU32::new(HOLDING.load(Ordering::Acquire)) else {
		return;
	};
	let mut g = locked();
	let Grip::Free = g.grip else {
		return;
	};
	let engaged: io::Result<()> = 'engage: {
		if matches!(g.lock, Lock::Closed) {
			let adopted: Option<RawFd> = match env::var(INHERIT_VAR) {
				Err(_no_var) => None,
				Ok(raw) => {
					let Ok(fd) = raw.parse::<RawFd>() else {
						break 'engage Err(io::Error::other(format!(
							"{INHERIT_VAR}={raw}"
						)));
					};
					// SAFETY: fcntl(F_GETFD) only reads the fd's flags and dereferences no pointers.
					match unsafe { libc::fcntl(fd, libc::F_GETFD) }.cmp(&0i32) {
						cmp::Ordering::Less => {
							break 'engage Err(io::Error::other(format!(
								"{INHERIT_VAR}={fd}: {}",
								io::Error::last_os_error()
							)));
						}
						cmp::Ordering::Equal | cmp::Ordering::Greater => Some(fd),
					}
				}
			};
			if adopted.is_some() {
				g.lock = Lock::Adopted;
				g.file = None;
			} else {
				// SAFETY: getuid takes no arguments, cannot fail, and dereferences no pointers.
				let uid = unsafe { libc::getuid() };
				let dir = env::var_os("XDG_RUNTIME_DIR")
					.filter(|v| return !v.is_empty())
					.map_or_else(
						|| return PathBuf::from(format!("/run/user/{uid}")),
						PathBuf::from,
					);
				if let Err(e) = fs::create_dir_all(&dir) {
					break 'engage Err(io::Error::other(format!(
						"create {}: {e}",
						dir.display()
					)));
				}
				let path = dir.join(LOCK_NAME);
				let opened = fs::OpenOptions::new()
					.read(true)
					.write(true)
					.create(true)
					.truncate(false)
					.open(&path);
				let file = match opened {
					Ok(f) => f,
					Err(e) => {
						break 'engage Err(io::Error::other(format!(
							"open {}: {e}",
							path.display()
						)));
					}
				};
				let fd = file.as_raw_fd();
				// SAFETY: fcntl(F_SETFD, 0) clears close-on-exec on an owned fd and dereferences no pointers.
				let None =
					num::NonZeroI32::new(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) })
				else {
					break 'engage Err(io::Error::last_os_error());
				};
				// SAFETY: env is mutated under the GATE mutex during single-threaded lock setup, with no concurrent readers.
				unsafe {
					env::set_var(INHERIT_VAR, fd.to_string());
				}
				g.lock = Lock::Own;
				g.file = Some(file);
			}
		}
		let Some(f) = g.file.as_mut() else {
			break 'engage Ok(());
		};
		let fd = f.as_raw_fd();
		if let Err(e) = flock(fd, libc::LOCK_EX | libc::LOCK_NB) {
			if e.kind() != io::ErrorKind::WouldBlock {
				break 'engage Err(e);
			}
			match holder_pid(f).filter(|p| return *p != CLEAN) {
				Some(pid) => {
					Write::wait(format!("Waiting for pid {pid} to release the GPU"));
				}
				None => Write::wait("Waiting for the GPU lock"),
			}
			let got = flock(fd, libc::LOCK_EX);
			Write::unwait();
			if let Err(blocked) = got {
				break 'engage Err(blocked);
			}
		}
		let me = process::id();
		let prev = holder_pid(f).filter(|p| return *p != CLEAN && *p != me);
		if let Err(e) = stamp(f, me) {
			break 'engage Err(e);
		}
		let Some(pid) = prev else {
			break 'engage Ok(());
		};
		let path = PathBuf::from(KFD_PROC).join(pid.to_string());
		let t0 = time::Instant::now();
		let parked = path.exists();
		if parked {
			Write::wait(format!("Waiting for pid {pid} gpu teardown"));
		}
		let mut wedged = false;
		while path.exists() {
			wedged = expired(t0).is_some();
			if wedged {
				break;
			}
			thread::sleep(time::Duration::from_millis(2));
		}
		if parked {
			Write::unwait();
		}
		if wedged {
			overstayed(pid, t0);
			break 'engage Ok(());
		}
		let Some(mut free) = sysfs_vram_free() else {
			break 'engage Ok(());
		};
		Write::wait(format!("Waiting for pid {pid} vram reclaim"));
		loop {
			thread::sleep(RECLAIM_STEP);
			let Some(now) = sysfs_vram_free() else {
				Write::unwait();
				break 'engage Ok(());
			};
			match now.cmp(&free) {
				cmp::Ordering::Greater => free = now,
				cmp::Ordering::Equal | cmp::Ordering::Less => {
					Write::unwait();
					break 'engage Ok(());
				}
			}
			let None = expired(t0) else {
				Write::unwait();
				overstayed(pid, t0);
				break 'engage Ok(());
			};
		}
	};
	if engaged.is_ok() {
		g.grip = Grip::Taken;
	}
	drop(g);
	match engaged {
		Ok(()) => {
			HOLDING.store(process::id(), Ordering::Release);
		}
		Err(e) => {
			Write::error(format!("gpu gate: {e}"));
			return;
		}
	}
}

#[inline]
pub fn release() {
	let mut g = locked();
	let Grip::Taken = g.grip else {
		return;
	};
	let Some(f) = g.file.as_mut() else {
		return;
	};
	if let Some(e) = stamp(f, CLEAN).err() {
		Write::error(format!("gpu gate: clean stamp: {e}"));
	}
	if let Some(e) = flock(f.as_raw_fd(), libc::LOCK_UN).err() {
		Write::error(format!("gpu gate: unlock: {e}"));
	}
	g.grip = Grip::Free;
	drop(g);
	HOLDING.store(CLEAN, Ordering::Release);
}

pub struct Lease {
	_p: PhantomData<()>,
}

impl Default for Lease {
	#[inline]
	fn default() -> Self {
		return Self::new();
	}
}

impl Lease {
	#[must_use]
	#[inline]
	pub fn new() -> Self {
		acquire();
		return Self { _p: PhantomData };
	}
}

impl Drop for Lease {
	#[inline]
	fn drop(&mut self) {
		release();
	}
}
