//! OS-level unsafe boundary: the single named home for libc/process/signal
//! calls that the rest of the workspace would otherwise reach for inline.
//! Every function here is SAFE to call; the unsafe is contained.

use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::io::AsRawFd as _;
use std::path::Path;
use std::process::Command;

/// Bytes free (available to an unprivileged writer) on the filesystem holding
/// `path`. Returns 0 when the path cannot be encoded or the query fails.
#[inline]
#[must_use]
pub fn disk_free_bytes(path: &Path) -> usize {
	let Ok(c) = CString::new(path.as_os_str().as_bytes()) else {
		return 0;
	};
	// SAFETY: statvfs is plain-old-data with no invalid bit patterns; zeroed is a valid initial state.
	let mut st: libc::statvfs = unsafe { core::mem::zeroed() };
	// SAFETY: c is a valid NUL-terminated path; st is a valid writable statvfs slot.
	if unsafe { libc::statvfs(c.as_ptr(), &raw mut st) } != 0i32 {
		return 0;
	}
	return usize::try_from(st.f_bavail)
		.unwrap_or(0)
		.saturating_mul(usize::try_from(st.f_frsize).unwrap_or(0));
}

/// Total bytes of the filesystem holding `path`, or `None` if the path cannot
/// be encoded or the `statvfs` call fails.
#[inline]
#[must_use]
pub fn disk_total_bytes(path: &Path) -> Option<u64> {
	let c = CString::new(path.as_os_str().as_bytes()).ok()?;
	// SAFETY: statvfs is plain-old-data with no invalid bit patterns; zeroed is a valid initial state.
	let mut st: libc::statvfs = unsafe { core::mem::zeroed() };
	// SAFETY: c is a valid NUL-terminated path; st is a valid writable statvfs slot.
	if unsafe { libc::statvfs(c.as_ptr(), &raw mut st) } != 0i32 {
		return None;
	}
	return Some((st.f_blocks as u64).saturating_mul(st.f_frsize as u64));
}

/// Arrange for `cmd`'s child to disable core dumps between fork and exec.
/// Used by the size-probe spawns so a deliberate OOM/commit failure leaves no
/// multi-GB core behind.
pub fn disable_core_dumps(cmd: &mut Command) {
	use std::os::unix::process::CommandExt as _;
	// SAFETY: setrlimit and Ok are async-signal-safe; the child only lowers its
	// own RLIMIT_CORE before exec and touches no shared state.
	unsafe {
		cmd.pre_exec(|| {
			let z = libc::rlimit {
				rlim_cur: 0,
				rlim_max: 0,
			};
			libc::setrlimit(libc::RLIMIT_CORE, &z);
			return Ok(());
		});
	}
}

/// Advise the kernel to drop the `[off, off+len)` byte range of `f` from the
/// page cache. Best-effort; failures are ignored (cache advice is a hint).
pub fn advise_dontneed(f: &File, off: u64, len: usize) {
	// SAFETY: f owns a live fd; posix_fadvise reads no memory and cannot corrupt state.
	unsafe {
		libc::posix_fadvise(f.as_raw_fd(), off as i64, len as i64, libc::POSIX_FADV_DONTNEED);
	}
}

/// Flush the `[off, off+len)` range of `f` to disk and then drop it from the
/// page cache (write-behind eviction). Best-effort.
pub fn evict_range(f: &File, off: u64, len: usize) {
	// SAFETY: f owns a live fd; both calls only reference kernel-side state for that fd.
	unsafe {
		libc::sync_file_range(
			f.as_raw_fd(),
			off as i64,
			len as i64,
			libc::SYNC_FILE_RANGE_WAIT_BEFORE
				| libc::SYNC_FILE_RANGE_WRITE
				| libc::SYNC_FILE_RANGE_WAIT_AFTER,
		);
	}
	advise_dontneed(f, off, len);
}

/// Sum every `stride`-th byte of `buf` with volatile reads, so the compiler
/// cannot elide the loads. The single primitive for RAM read-bandwidth probes;
/// takes a slice so the bounds are the caller's borrow, not a raw address.
#[must_use]
pub fn volatile_read_sum(buf: &[u8], stride: usize) -> u64 {
	let step = stride.max(1);
	let mut acc = 0u64;
	let mut i = 0usize;
	while i < buf.len() {
		// SAFETY: i < buf.len() holds each iteration, so the offset pointer is
		// in-bounds for buf; volatile only forbids the optimizer eliding the load.
		acc = acc.wrapping_add(u64::from(unsafe { core::ptr::read_volatile(buf.as_ptr().add(i)) }));
		i += step;
	}
	return acc;
}

/// Terminate the process immediately with `code`, skipping atexit handlers and
/// buffer flushes. Async-signal-safe: the only call permitted from a signal
/// handler that must not run Rust destructors.
#[inline]
pub fn exit_now(code: i32) -> ! {
	// SAFETY: _exit is async-signal-safe and simply terminates the process.
	unsafe { libc::_exit(code) };
}

/// Install `handler` as the SIGINT disposition. `handler` must be an
/// `extern "C"` function that only calls async-signal-safe routines.
pub fn on_sigint(handler: extern "C" fn(i32)) {
	// SAFETY: handler is a valid extern "C" fn pointer; signal() just records the disposition.
	unsafe {
		libc::signal(libc::SIGINT, handler as *const () as libc::sighandler_t);
	}
}

/// Restore SIGINT to the default disposition.
pub fn reset_sigint() {
	// SAFETY: SIG_DFL is the well-defined default disposition sentinel.
	unsafe {
		libc::signal(libc::SIGINT, libc::SIG_DFL);
	}
}

/// Redirect fd 2 (stderr) to `file`, returning a dup of the original fd 2 to
/// pass back to [`restore_stderr`], or `-1` if the redirect could not be set up.
pub fn redirect_stderr(file: &File) -> i32 {
	// SAFETY: dup(2) then dup2 onto fd 2; on success fd 2 aliases file and the
	// returned fd keeps the original stderr open for later restore.
	unsafe {
		let saved = libc::dup(2);
		if saved >= 0 {
			libc::dup2(file.as_raw_fd(), 2);
		}
		return saved;
	}
}

/// Restore fd 2 from a `saved` dup produced by [`redirect_stderr`], closing the
/// dup. A negative `saved` is a no-op.
pub fn restore_stderr(saved: i32) {
	if saved < 0 {
		return;
	}
	// SAFETY: saved is a live dup of the original fd 2; dup2 reinstates it and close releases the dup.
	unsafe {
		libc::dup2(saved, 2);
		libc::close(saved);
	}
}

/// One up, non-loopback, IPv4 interface: its name and address/netmask as host
/// order `u32`s. The single `getifaddrs` boundary.
pub struct Ipv4Iface {
	pub name: String,
	pub ip: u32,
	pub mask: u32,
}

/// Enumerate up, non-loopback IPv4 interfaces. Returns an empty vec if the
/// `getifaddrs` query fails.
#[must_use]
pub fn ipv4_interfaces() -> Vec<Ipv4Iface> {
	let mut out = Vec::new();
	// SAFETY: getifaddrs yields a NULL-terminated list we only read, and every
	// pointer is null-checked before deref; freeifaddrs releases it on exit.
	unsafe {
		let mut ifap: *mut libc::ifaddrs = core::ptr::null_mut();
		if libc::getifaddrs(&raw mut ifap) != 0i32 {
			return out;
		}
		let mut p = ifap;
		while !p.is_null() {
			let a = &*p;
			let flags = a.ifa_flags;
			let up = flags & (libc::IFF_UP | libc::IFF_RUNNING) as u32
				== (libc::IFF_UP | libc::IFF_RUNNING) as u32;
			let lo = flags & libc::IFF_LOOPBACK as u32 != 0;
			let want = up
				&& !lo && !a.ifa_addr.is_null()
				&& (*a.ifa_addr).sa_family as i32 == libc::AF_INET;
			if want {
				let sin = &*a.ifa_addr.cast::<libc::sockaddr_in>();
				let ip = u32::from_be(sin.sin_addr.s_addr);
				let mask = match a.ifa_netmask.cast::<libc::sockaddr_in>().as_ref() {
					Some(nm) => u32::from_be(nm.sin_addr.s_addr),
					None => 0,
				};
				let name = CStr::from_ptr(a.ifa_name).to_string_lossy().into_owned();
				out.push(Ipv4Iface { name, ip, mask });
			}
			p = a.ifa_next;
		}
		libc::freeifaddrs(ifap);
	}
	return out;
}
