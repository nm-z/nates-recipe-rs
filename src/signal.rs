#![allow(unsafe_code, reason = "POSIX signal installation requires libc FFI")]

use core::fmt;
use std::{
	io,
	sync::{
		Mutex, MutexGuard,
		atomic::{AtomicBool, Ordering},
	},
};

static SIGINT_REQUESTED: AtomicBool = AtomicBool::new(false);
static SIGINT_INSTALLATION: Mutex<()> = Mutex::new(());

extern "C" fn record_sigint(_signal: libc::c_int) {
	// AtomicBool exists only on targets with native atomic byte operations. The
	// handler performs no allocation, locking, formatting, or system call.
	SIGINT_REQUESTED.store(true, Ordering::Release);
}

/// Process-scoped SIGINT handler serialized across Recipe operations.
pub struct SigintGuard {
	previous: libc::sigaction,
	requested: &'static AtomicBool,
	_installation: MutexGuard<'static, ()>,
}

impl fmt::Debug for SigintGuard {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		return f
			.debug_struct("SigintGuard")
			.field("requested", &self.requested())
			.finish_non_exhaustive();
	}
}

impl SigintGuard {
	pub(crate) fn install() -> io::Result<Self> {
		let installation = SIGINT_INSTALLATION
			.lock()
			.map_err(|_error| return io::Error::other("Recipe SIGINT installation lock is poisoned"))?;
		SIGINT_REQUESTED.store(false, Ordering::Release);
		// SAFETY: both sigaction values are fully initialized before use. The
		// installed handler has the required C ABI and performs only one atomic
		// store. sigemptyset and sigaction receive valid writable pointers.
		let mut action: libc::sigaction = unsafe { core::mem::zeroed() };
		action.sa_sigaction = record_sigint as *const () as libc::sighandler_t;
		action.sa_flags = 0;
		// SAFETY: `action.sa_mask` is initialized and its raw pointer is valid for this call.
		if unsafe { libc::sigemptyset(&raw mut action.sa_mask) } != 0 {
			return Err(io::Error::last_os_error());
		}
		// SAFETY: zero is a valid initial representation for the output `sigaction` value.
		let mut previous: libc::sigaction = unsafe { core::mem::zeroed() };
		// SAFETY: both pointers refer to initialized `sigaction` values that remain live for the call.
		if unsafe { libc::sigaction(libc::SIGINT, &raw const action, &raw mut previous) } != 0 {
			return Err(io::Error::last_os_error());
		}
		return Ok(Self {
			previous,
			requested: &SIGINT_REQUESTED,
			_installation: installation,
		});
	}

	pub(crate) fn requested(&self) -> bool { return self.requested.load(Ordering::Acquire); }

	pub(crate) const fn request_flag(&self) -> &'static AtomicBool { return self.requested; }
}

impl Drop for SigintGuard {
	fn drop(&mut self) {
		// SAFETY: `_previous` was returned by a successful sigaction call for
		// SIGINT and remains initialized for the guard's lifetime.
		unsafe {
			libc::sigaction(
				libc::SIGINT,
				&raw const self.previous,
				core::ptr::null_mut(),
			);
		}
	}
}

pub fn send_sigint(process_id: u32) -> io::Result<()> {
	let process_id = libc::pid_t::try_from(process_id).map_err(|_error| return io::Error::new(io::ErrorKind::InvalidInput, "process id does not fit pid_t"))?;
	// SAFETY: kill does not dereference process memory; the validated positive
	// child process id and SIGINT value are passed directly to the OS.
	if unsafe { libc::kill(process_id, libc::SIGINT) } == 0 {
		return Ok(());
	}
	return Err(io::Error::last_os_error());
}
