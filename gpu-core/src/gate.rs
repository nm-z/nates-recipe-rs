//! System-wide mutex over the one GPU: an flock(2) on
//! `/run/user/<uid>/recipe-gpu.lock`.
//!
//! Two processes driving /dev/kfd at once fault the driver, so every process
//! that touches the device takes this lock first and holds it until it is done.
//! The kernel owns the queue: first to flock wins, the rest sleep inside
//! `flock(LOCK_EX)` until the holder releases. No polling, no pid watching, no
//! `fuser`. A SIGKILLed holder releases on fd teardown, so the queue cannot
//! wedge.
//!
//! The pid written into the file names the holder. A contender that lost the
//! non-blocking attempt reads it to say who it waits behind; the winner reads it
//! to know whose teardown it must outlast. Winning the flock is not enough: the
//! kernel drops the lock when it closes the fd in `exit_files`, and amdgpu
//! reclaims the dead process's VRAM asynchronously after that. A successor that
//! starts immediately measures `claimable 0.0 KB` and refuses to claim an arena.
//! So the winner waits for the predecessor's teardown to finish — a stat on one
//! known pid, not a scan for who is busy.
//!
//! A process tree is ONE holder. The lock lives on an open file description, so
//! a child that inherits the fd inherits the lock with it: `Machine::probe`
//! re-execs itself per GPU, and those children must run under the parent's lease
//! rather than deadlock against it. The fd number rides in `RECIPE_GPU_LOCK_FD`;
//! a process that finds it set adopts the lease and never locks or unlocks.
//!
//! The fd, once opened, stays open for the life of the process, and the variable
//! is written exactly once — at the first device touch, which precedes the
//! threads that read the environment. Releasing is `LOCK_UN`, not `close`: the
//! daemon leases the card per job, and a `setenv` on every job would race the
//! beacon thread's `getenv` for the environ block.

use std::io::{Read, Seek, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

const LOCK_NAME: &str = "recipe-gpu.lock";
const INHERIT_VAR: &str = "RECIPE_GPU_LOCK_FD";
const KFD_PROC: &str = "/sys/class/kfd/kfd/proc";
const TEARDOWN_DEADLINE_SECS: f64 = 30.0;
/// Wider than the widest gap between two of amdgpu's reclaim steps (15ms).
const RECLAIM_STEP: std::time::Duration = std::time::Duration::from_millis(25);
/// The stamp is one fixed-width write at offset 0, never a truncate: a reader
/// racing the winner sees the old pid or the new one, never an empty file.
const STAMP_WIDTH: usize = 20;
/// Stamped by a voluntary release, which happens only after the releasing
/// process has handed its VRAM back. A successor reading it skips the teardown
/// wait. A holder that died instead leaves its own pid, and still owes the card.
const CLEAN: u32 = 0;

enum Lock {
	/// No fd yet — this process has never touched the device.
	Closed,
	/// Our own fd. Holding says whether the flock is currently ours.
	Own(std::fs::File),
	/// A parent's fd, inherited across exec. The lease is theirs to end.
	Adopted,
}

struct Gate {
	lock: Lock,
	holding: bool,
}

static GATE: Mutex<Gate> = Mutex::new(Gate { lock: Lock::Closed, holding: false });
/// Lock-free fast path for the millions of calls that arrive already holding.
static HOLDING: AtomicBool = AtomicBool::new(false);

fn lock_path() -> std::path::PathBuf {
	// SAFETY: getuid(2) always succeeds.
	let uid = unsafe { libc::getuid() };
	std::path::PathBuf::from(format!("/run/user/{uid}")).join(LOCK_NAME)
}

/// The fd this process inherited from a parent that already holds the GPU.
fn inherited() -> Option<RawFd> {
	let raw = std::env::var(INHERIT_VAR).ok()?;
	let fd: RawFd = raw.parse().unwrap_or_else(|e| panic!("gpu gate: {INHERIT_VAR}={raw}: {e}"));
	// SAFETY: F_GETFD reads no memory; it only reports whether fd is open.
	if unsafe { libc::fcntl(fd, libc::F_GETFD) } < 0 {
		panic!("gpu gate: {INHERIT_VAR}={fd}: {}", std::io::Error::last_os_error());
	}
	Some(fd)
}

fn flock(fd: RawFd, op: i32) -> std::io::Result<()> {
	loop {
		// SAFETY: fd stays open across the call.
		if unsafe { libc::flock(fd, op) } == 0 {
			return Ok(());
		}
		let e = std::io::Error::last_os_error();
		if e.kind() != std::io::ErrorKind::Interrupted {
			return Err(e);
		}
	}
}

/// The pid the current holder stamped. Absent on the first lock after boot,
/// when the file is still empty, and `CLEAN` after a voluntary release.
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

/// Open the lockfile and publish its fd to every process this one will exec.
fn open_lock() -> std::fs::File {
	let path = lock_path();
	let f = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(false)
		.open(&path)
		.unwrap_or_else(|e| panic!("gpu gate: open {}: {e}", path.display()));
	let fd = f.as_raw_fd();
	// The lease covers the whole process tree: children exec with this fd still
	// open and adopt it through INHERIT_VAR instead of blocking on their parent.
	// SAFETY: clearing FD_CLOEXEC on an fd this process owns.
	if unsafe { libc::fcntl(fd, libc::F_SETFD, 0) } < 0 {
		panic!("gpu gate: keep fd {fd} across exec: {}", std::io::Error::last_os_error());
	}
	// SAFETY: the first device touch precedes any thread that reads the
	// environment — the same argument hip::disable_sdma_once makes for
	// HSA_ENABLE_SDMA, and this is the only write.
	unsafe { std::env::set_var(INHERIT_VAR, fd.to_string()) };
	f
}

fn take(f: &mut std::fs::File) {
	let fd = f.as_raw_fd();
	if let Err(e) = flock(fd, libc::LOCK_EX | libc::LOCK_NB) {
		if e.kind() != std::io::ErrorKind::WouldBlock {
			panic!("gpu gate: flock: {e}");
		}
		match holder_pid(f) {
			Some(pid) => eprintln!("gpu gate: queued behind pid {pid}"),
			None => eprintln!("gpu gate: queued behind the current holder"),
		}
		flock(fd, libc::LOCK_EX).unwrap_or_else(|e| panic!("gpu gate: flock: {e}"));
	}
	// The lock is ours; the card may not be. Stamp first, so a contender blocking
	// during the teardown wait names this process and not the corpse.
	let me = std::process::id();
	let prev = holder_pid(f).filter(|p| *p != CLEAN && *p != me);
	stamp(f, me).unwrap_or_else(|e| panic!("gpu gate: stamp: {e}"));
	if let Some(pid) = prev {
		await_teardown(pid);
	}
}

/// Outlast the predecessor's device teardown, in the two stages the driver
/// actually takes.
///
/// First its kfd entry disappears. That is NOT the end: the run's arena is
/// mapped for the life of the process and amdgpu unmaps it afterwards, so
/// `mem_info_vram_used` on gfx1101 keeps falling for another ~90-110ms, in steps
/// 5-15ms apart (measured: a 12 GB stress claim drains back to the 2.0 GB
/// desktop baseline entirely after the entry is gone). A successor that claims
/// inside that window reads `claimable 0.0 KB` and refuses. So stage two waits
/// for free VRAM to stop rising, sampling slower than the widest step, so that a
/// flat sample means finished and not merely between two frees.
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

/// Claim the GPU for this process, blocking until every other holder releases.
/// Idempotent: a process already holding the lock, or running under a parent's
/// lease, returns immediately.
///
/// The device entry funnels call this, so no caller opts in.
pub fn acquire() {
	if HOLDING.load(Ordering::Acquire) {
		return;
	}
	let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
	if g.holding {
		return;
	}
	if let Lock::Closed = g.lock {
		g.lock = match inherited() {
			Some(_) => Lock::Adopted,
			None => Lock::Own(open_lock()),
		};
	}
	if let Lock::Own(f) = &mut g.lock {
		take(f);
	}
	g.holding = true;
	HOLDING.store(true, Ordering::Release);
}

/// Hand the GPU to the next process in the kernel's queue. A process running
/// under an inherited lease keeps it: the parent owns the release.
///
/// Callers must already have given the card back — the daemon trims the pool
/// before dropping a job's `Lease`. That is what the `CLEAN` stamp promises the
/// successor, and why it may start without waiting.
pub fn release() {
	let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
	if !g.holding {
		return;
	}
	if let Lock::Own(f) = &mut g.lock {
		stamp(f, CLEAN).unwrap_or_else(|e| panic!("gpu gate: clean stamp: {e}"));
		flock(f.as_raw_fd(), libc::LOCK_UN).unwrap_or_else(|e| panic!("gpu gate: unlock: {e}"));
		g.holding = false;
		HOLDING.store(false, Ordering::Release);
	}
}

/// A scoped claim for a queued job: acquire on entry, hand off on drop, even
/// when the job panics. The daemon leases the GPU per job; a one-shot binary
/// holds it from its first device touch until the kernel closes its fds.
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
