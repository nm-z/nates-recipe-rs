use crate::log::{Write, gpu};
use core::array::from_fn;
use core::cmp;
use core::mem::transmute_copy;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use std::env;
use std::fs;
use std::io::{Error, Read as _, Write as _};
use std::os::unix::io::{AsRawFd as _, FromRawFd as _};
use std::path::PathBuf;
use std::process;
use std::sync::Once;
use std::thread;
use std::time::Instant;

/// True while a compute phase is armed for the saturation-crash watchdog.
static SAT_ARMED: AtomicBool = AtomicBool::new(0i32 == 1i32);
/// Longest gap without a 100%-busy sample before the watchdog aborts.
const SAT_WINDOW: Duration = Duration::from_secs(5);
/// When `Some(())`, the saturation-crash watchdog is enabled.
const SAT_ENFORCE: Option<()> = None;

/// Signal handler: disable core dumps, then re-raise with the default disposition.
#[inline]
pub extern "C" fn fast_death(sig: libc::c_int) {
	// SAFETY: async-signal-safe; disables core dumps before re-raising.
	unsafe {
		libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
	}
	// SAFETY: async-signal-safe; restores the default disposition for the signal.
	unsafe {
		libc::signal(sig, libc::SIG_DFL);
	}
	// SAFETY: async-signal-safe; re-raises to terminate with the default action.
	unsafe {
		libc::raise(sig);
	}
}

#[inline]
pub fn install_fast_death() {
	static ONCE: Once = Once::new();
	ONCE.call_once(|| {
		let None = env::var_os("RECIPE_CORE") else {
			return;
		};
		let handler_fn: extern "C" fn(libc::c_int) = fast_death;
		// SAFETY: a C fn pointer and sighandler_t (size_t) are both pointer-width; reinterpret its address bits.
		let handler: libc::sighandler_t = unsafe {
			transmute_copy::<extern "C" fn(libc::c_int), libc::sighandler_t>(&handler_fn)
		};
		// SAFETY: registers our async-signal-safe handler for SIGABRT.
		unsafe {
			libc::signal(libc::SIGABRT, handler);
		}
	});
}

#[inline]
pub fn arm_saturation_crash() {
	static ONCE: Once = Once::new();
	let Some(()) = SAT_ENFORCE else {
		return;
	};

	install_fast_death();
	SAT_ARMED.store(0i32 != 1i32, Ordering::SeqCst);
	ONCE.call_once(|| {
		let busy_path = || -> PathBuf {
			for card in fs::read_dir("/sys/class/drm")
				.into_iter()
				.flatten()
				.flatten()
			{
				let p = card.path().join("device/gpu_busy_percent");
				let Some(found) = Some(p).filter(|q| return q.exists()) else {
					continue;
				};
				return found;
			}
			drop(Write::err(
				"saturation watchdog: no gpu_busy_percent under /sys/class/drm",
			));
			process::abort();
		};
		let path = busy_path();
		thread::spawn(move || {
			let mut last_pinned = Instant::now();
			let mut was_armed = 0i32 == 1i32;
			loop {
				let armed = SAT_ARMED.load(Ordering::SeqCst);
				if armed {
					if !was_armed {
						last_pinned = Instant::now();
					}
					let busy: u32 = fs::read_to_string(&path)
						.unwrap_or_else(|e| {
								drop(Write::err(format!("saturation watchdog: read {}: {e}", path.display())));
								process::abort()
							})
						.trim()
						.parse()
						.unwrap_or_else(|e| {
								drop(Write::err(format!("saturation watchdog: parse busy: {e}")));
								process::abort()
							});
					if busy >= 100 {
						last_pinned = Instant::now();
					}
					let stalled = SAT_ARMED.load(Ordering::SeqCst) && last_pinned.elapsed() > SAT_WINDOW;
					let None = stalled.then_some(()) else {
						drop(Write::err(format!(
							"GPU NOT PINNED  no 100% gpu_busy_percent sample in {}s (latest {busy}%) during compute — aborting (saturation law)",
							SAT_WINDOW.as_secs()
						)));
						process::abort();
					};
				}
				was_armed = armed;
				thread::sleep(Duration::from_millis(10));
			}
		});
	});
}

#[inline]
pub fn disarm_saturation_crash() {
	SAT_ARMED.store(0i32 == 1i32, Ordering::SeqCst);
}

/// KFD ioctl selector for subscribing to SMI (thermal/thrash) events.
const AMDKFD_IOC_SMI_EVENTS: libc::c_ulong = 0xC008_4B1F;

#[repr(C)]
/// Argument block for the AMDKFD SMI-events ioctl.
struct SmiArgs {
	/// KFD topology `gpu_id` to subscribe to.
	gpuid: u32,
	/// Anonymous fd the kernel returns for reading events.
	anon_fd: u32,
}

/// Classification of a decoded KFD SMI event.
enum GpuEvent {
	/// Driver evicted our queues or mappings.
	Thrash,
	/// Queues restored after an eviction.
	Restored,
	/// Any other event, logged but not acted on.
	Other,
}

#[inline]
pub fn spawn_thrash_watchdog() {
	static ONCE: Once = Once::new();
	ONCE.call_once(|| {
		let gpu_id = || -> Option<u32> {
			for e in fs::read_dir("/sys/class/kfd/kfd/topology/nodes").ok()? {
				let p = e.ok()?.path().join("gpu_id");
				let found = fs::read_to_string(&p)
					.ok()
					.and_then(|s| return s.trim().parse::<u32>().ok())
					.filter(|id| return *id != 0);
				let Some(id) = found else {
					continue;
				};
				return Some(id);
			}
			return None;
		};
		let Some(gpu_idx) = gpu_id() else {
			drop(Write::err("thrash watchdog: no kfd gpu_id"));
			return;
		};
		// SAFETY: opening a static NUL-terminated device path with standard flags; no aliasing.
		let raw = unsafe { libc::open(c"/dev/kfd".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
		let kfd = match raw.cmp(&0i32) {
			cmp::Ordering::Less => {
				drop(Write::err(format!(
					"thrash watchdog: /dev/kfd: {}",
					Error::last_os_error()
				)));
				return;
			}
			// SAFETY: this arm runs only when raw >= 0, a valid owned fd that File adopts.
			cmp::Ordering::Equal | cmp::Ordering::Greater => unsafe {
				fs::File::from_raw_fd(raw)
			},
		};
		let mut args = SmiArgs {
			gpuid: gpu_idx,
			anon_fd: 0,
		};
		// SAFETY: kfd is an open fd and args is an initialized SmiArgs for this ioctl.
		let rc = unsafe { libc::ioctl(kfd.as_raw_fd(), AMDKFD_IOC_SMI_EVENTS, &mut args) };
		let cmp::Ordering::Equal = rc.cmp(&0i32) else {
			drop(Write::err(format!(
				"thrash watchdog: SMI ioctl: {}",
				Error::last_os_error()
			)));
			return;
		};
		// SAFETY: anon_fd was just returned by the SMI ioctl and is owned by us.
		let mut smi = unsafe { fs::File::from_raw_fd(args.anon_fd.cast_signed()) };
		let mask: u64 = [1u32, 2, 5, 6, 7, 8, 9, 10, 11]
			.iter()
			.map(|i| return 1u64 << (i - 1))
			.sum();
		let smi_bytes: [u8; 8] = from_fn(|i| {
			return u8::try_from((mask >> (i * 8)) & 0xff).unwrap_or(0);
		});
		if let Err(e) = (&smi).write_all(&smi_bytes) {
			drop(Write::err(format!("thrash watchdog: mask write: {e}")));
			return;
		}
		thread::spawn(move || {
			let mut buf = [0u8; 1024];
			loop {
				let n = match smi.read(&mut buf) {
					Ok(n) if n > 0 => n,
					_ => return,
				};
				for ev in String::from_utf8_lossy(&buf[..n]).split_terminator('\n') {
					let id = ev
						.split_whitespace()
						.next()
						.and_then(|t| return u32::from_str_radix(t, 16).ok())
						.unwrap_or(0);
					let kind = match id {
						9 | 11 => GpuEvent::Thrash,
						10 => GpuEvent::Restored,
						_ => GpuEvent::Other,
					};
					match kind {
						GpuEvent::Thrash => {
							drop(Write::err(format!(
								"gpu thrash  {}  — driver evicted our queues/mappings; aborting per fail-clean",
								ev.trim()
							)));
							process::abort();
						}
						GpuEvent::Restored => Write::line(
							gpu,
							format!("gpu event  queue restored  {}", ev.trim()),
						),
						GpuEvent::Other => {
							Write::line(gpu, format!("gpu event  {}", ev.trim()));
						}
					}
				}
			}
		});
	});
}
