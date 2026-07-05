//! Kernel-side HW truth: the KFD SMI thrash watchdog. Subscribes to the
//! driver's per-process event stream (/dev/kfd, kfd_ioctl.h:528-547) and turns
//! silent driver pathology into loud lines: queue eviction of OUR compute
//! queues aborts (fail clean beats limping through a thrash storm), migrations
//! and throttles print. The HSA memory-fault autopsy lives in hip.rs; this
//! file is everything the kernel can tell us that HIP cannot.

use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};

// ── Saturation crash ───────────────────────────────────────────────────────
// The saturation invariant, executable: while armed (the fit's compute loop),
// the GPU must hit 100% busy at least once in every 5-second window; a window
// that never pins aborts the process. Momentary dips (the one scalar download
// at an epoch boundary) are not serialization; five straight seconds without
// a single pinned sample is.

static SAT_ARMED: AtomicBool = AtomicBool::new(false);
const SAT_WINDOW: std::time::Duration = std::time::Duration::from_secs(5);

// ── Fast death ──────────────────────────────────────────────────────────────
// A GPU-scale process holds tens of GB; SIGABRT's default core dump pipes the
// whole image through systemd-coredump, which drains it even past its size
// cap — the terminal sits minutes in an uninterruptible dump where ^C is
// undeliverable. The autopsy + ledger on stderr are the real forensics; the
// giant core is not. RLIMIT_CORE is NOT enforced for piped core_patterns
// (core(5)), so the working switch is PR_SET_DUMPABLE=0: do_coredump refuses
// non-dumpable processes outright and the re-raise dies instantly, dump-free.
// RECIPE_CORE=1 skips installation for a session where a core is wanted.

extern "C" fn fast_death(sig: libc::c_int) {
	unsafe {
		libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
		libc::signal(sig, libc::SIG_DFL);
		libc::raise(sig);
	}
}

pub fn install_fast_death() {
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		if std::env::var_os("RECIPE_CORE").is_some() {
			return;
		}
		unsafe {
			libc::signal(libc::SIGABRT, fast_death as *const () as libc::sighandler_t);
		}
	});
}

fn busy_path() -> std::path::PathBuf {
	for card in std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten() {
		let p = card.path().join("device/gpu_busy_percent");
		if p.exists() {
			return p;
		}
	}
	panic!("saturation watchdog: no gpu_busy_percent under /sys/class/drm");
}

/// Arm for the duration of a compute phase. First call spawns the sampler.
pub fn arm_saturation_crash() {
	static ONCE: std::sync::Once = std::sync::Once::new();
	install_fast_death();
	SAT_ARMED.store(true, Ordering::SeqCst);
	ONCE.call_once(|| {
		let path = busy_path();
		std::thread::spawn(move || {
			let mut last_pinned = std::time::Instant::now();
			let mut was_armed = false;
			loop {
				let armed = SAT_ARMED.load(Ordering::SeqCst);
				if armed {
					if !was_armed {
						// Fresh compute phase gets a fresh window.
						last_pinned = std::time::Instant::now();
					}
					let busy: u32 = std::fs::read_to_string(&path)
						.unwrap_or_else(|e| panic!("saturation watchdog: read {path:?}: {e}"))
						.trim()
						.parse()
						.unwrap_or_else(|e| panic!("saturation watchdog: parse busy: {e}"));
					if busy >= 100 {
						last_pinned = std::time::Instant::now();
					}
					if SAT_ARMED.load(Ordering::SeqCst) && last_pinned.elapsed() > SAT_WINDOW {
						eprintln!(
							"\x1b[1;31mGPU NOT PINNED\x1b[0m  no 100% gpu_busy_percent sample in {}s (latest {busy}%) during compute — aborting (saturation law)",
							SAT_WINDOW.as_secs()
						);
						std::process::abort();
					}
				}
				was_armed = armed;
				std::thread::sleep(std::time::Duration::from_millis(10));
			}
		});
	});
}

pub fn disarm_saturation_crash() {
	SAT_ARMED.store(false, Ordering::SeqCst);
}

/// 1 Hz trivial device op for the life of the process. The GPU has a ~5 s
/// runtime-PM autosuspend; a multi-minute host-only phase (CSV parse,
/// tokenize) lets it suspend, and the wake races the first heavy transfer —
/// faults/wedges cluster exactly there (gemma4 fights the same disease with
/// a load-scoped keepalive; the recipe fit's crashes sit right after the
/// data-load gap, writing runtime-internal VAs no allocation of ours owns).
pub fn spawn_gpu_keepalive() {
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		let Ok(buf) = crate::memory::GpuBuffer::alloc(1) else {
			eprintln!("keepalive: alloc failed");
			return;
		};
		std::thread::spawn(move || {
			loop {
				if buf.memset_zero(8).is_err() {
					return;
				}
				std::thread::sleep(std::time::Duration::from_secs(1));
			}
		});
	});
}

// _IOWR('K', 0x1F, struct kfd_ioctl_smi_events_args{u32,u32}) — kfd_ioctl.h:1735.
const AMDKFD_IOC_SMI_EVENTS: libc::c_ulong = 0xC008_4B1F;

#[repr(C)]
struct SmiArgs {
	gpuid: u32,
	anon_fd: u32,
}

/// First non-zero gpu_id under the KFD topology (0 = CPU node).
fn gpu_id() -> Option<u32> {
	for e in std::fs::read_dir("/sys/class/kfd/kfd/topology/nodes").ok()? {
		let p = e.ok()?.path().join("gpu_id");
		if let Some(id) = std::fs::read_to_string(&p).ok().and_then(|s| s.trim().parse::<u32>().ok())
			&& id != 0
		{
			return Some(id);
		}
	}
	None
}

/// Crash-on-thrash: QUEUE_EVICTION(9) / UNMAP_FROM_GPU(11) of this process
/// means the driver suspended our queues to shuffle memory out from under us —
/// print the raw event and abort. VMFAULT/THROTTLE/MIGRATE/PAGE_FAULT print
/// only. Failures to arm are reported once and non-fatal (the watchdog is
/// diagnosis, not a dependency).
pub fn spawn_thrash_watchdog() {
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		let Some(gpu) = gpu_id() else {
			eprintln!("thrash watchdog: no kfd gpu_id");
			return;
		};
		let kfd = match std::fs::OpenOptions::new().read(true).write(true).open("/dev/kfd") {
			Ok(f) => f,
			Err(e) => {
				eprintln!("thrash watchdog: /dev/kfd: {e}");
				return;
			}
		};
		let mut args = SmiArgs { gpuid: gpu, anon_fd: 0 };
		// SAFETY: ioctl with an owned struct, layout per kfd_ioctl.h:575.
		if unsafe { libc::ioctl(kfd.as_raw_fd(), AMDKFD_IOC_SMI_EVENTS, &mut args) } != 0 {
			eprintln!("thrash watchdog: SMI ioctl: {}", std::io::Error::last_os_error());
			return;
		}
		// SAFETY: the ioctl hands us ownership of anon_fd.
		let mut smi = unsafe { std::fs::File::from_raw_fd(args.anon_fd as i32) };
		// Subscribe mask: bit i-1 = event i (VMFAULT, THROTTLE, MIGRATE_START/END,
		// PAGE_FAULT_START/END, QUEUE_EVICTION, QUEUE_RESTORE, UNMAP_FROM_GPU).
		let mask: u64 = [1u32, 2, 5, 6, 7, 8, 9, 10, 11]
			.iter()
			.map(|i| 1u64 << (i - 1))
			.sum();
		if let Err(e) = (&smi).write_all(&mask.to_le_bytes()) {
			eprintln!("thrash watchdog: mask write: {e}");
			return;
		}
		std::thread::spawn(move || {
			let mut buf = [0u8; 1024];
			loop {
				let n = match smi.read(&mut buf) {
					Ok(0) | Err(_) => return,
					Ok(n) => n,
				};
				for ev in String::from_utf8_lossy(&buf[..n]).split_terminator('\n') {
					// Line starts with the event id in hex (kfd_smi_events.c).
					let id = ev
						.split_whitespace()
						.next()
						.and_then(|t| u32::from_str_radix(t, 16).ok())
						.unwrap_or(0);
					match id {
						9 | 11 => {
							eprintln!(
								"\x1b[1;31mgpu thrash\x1b[0m  {}  — driver evicted our queues/mappings; aborting per fail-clean",
								ev.trim()
							);
							std::process::abort();
						}
						10 => eprintln!("\x1b[33mgpu event\x1b[0m  queue restored  {}", ev.trim()),
						_ => eprintln!("\x1b[33mgpu event\x1b[0m  {}", ev.trim()),
					}
				}
			}
		});
	});
}
