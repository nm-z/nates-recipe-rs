
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};


static SAT_ARMED: AtomicBool = AtomicBool::new(false);
const SAT_WINDOW: std::time::Duration = std::time::Duration::from_secs(5);
const SAT_ENFORCE: bool = false;


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

pub fn arm_saturation_crash() {
	if !SAT_ENFORCE {
		return;
	}
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

const AMDKFD_IOC_SMI_EVENTS: libc::c_ulong = 0xC008_4B1F;

#[repr(C)]
struct SmiArgs {
	gpuid: u32,
	anon_fd: u32,
}

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
		if unsafe { libc::ioctl(kfd.as_raw_fd(), AMDKFD_IOC_SMI_EVENTS, &mut args) } != 0 {
			eprintln!("thrash watchdog: SMI ioctl: {}", std::io::Error::last_os_error());
			return;
		}
		let mut smi = unsafe { std::fs::File::from_raw_fd(args.anon_fd as i32) };
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
