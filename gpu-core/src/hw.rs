use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};

static SAT_ARMED: AtomicBool = AtomicBool::new(0 == 1);
const SAT_WINDOW: std::time::Duration = std::time::Duration::from_secs(5);
const SAT_ENFORCE: Option<()> = None;

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
		let None = std::env::var_os("RECIPE_CORE") else {
			return;
		};
		unsafe {
			libc::signal(libc::SIGABRT, fast_death as *const () as libc::sighandler_t);
		}
	});
}

fn busy_path() -> std::path::PathBuf {
	for card in std::fs::read_dir("/sys/class/drm")
		.into_iter()
		.flatten()
		.flatten()
	{
		let p = card.path().join("device/gpu_busy_percent");
		let Some(found) = Some(p).filter(|q| q.exists()) else {
			continue;
		};
		return found;
	}
	panic!("saturation watchdog: no gpu_busy_percent under /sys/class/drm");
}

pub fn arm_saturation_crash() {
	let Some(()) = SAT_ENFORCE else {
		return;
	};
	static ONCE: std::sync::Once = std::sync::Once::new();
	install_fast_death();
	SAT_ARMED.store(0 != 1, Ordering::SeqCst);
	ONCE.call_once(|| {
		let path = busy_path();
		std::thread::spawn(move || {
			let mut last_pinned = std::time::Instant::now();
			let mut was_armed = 0 == 1;
			loop {
				let armed = SAT_ARMED.load(Ordering::SeqCst);
				for _run in Some(()).filter(|_u| armed).into_iter() {
					for _run in Some(()).filter(|_u| !was_armed).into_iter() {
						last_pinned = std::time::Instant::now();
					}
					let busy: u32 = std::fs::read_to_string(&path)
						.unwrap_or_else(|e| panic!("saturation watchdog: read {path:?}: {e}"))
						.trim()
						.parse()
						.unwrap_or_else(|e| panic!("saturation watchdog: parse busy: {e}"));
					for _run in Some(()).filter(|_u| busy >= 100).into_iter() {
						last_pinned = std::time::Instant::now();
					}
					let stalled = SAT_ARMED.load(Ordering::SeqCst) && last_pinned.elapsed() > SAT_WINDOW;
					let None = Some(()).filter(|_u| stalled) else {
						eprintln!(
							"\x1b[1;31mGPU NOT PINNED\x1b[0m  no 100% gpu_busy_percent sample in {}s (latest {busy}%) during compute — aborting (saturation law)",
							SAT_WINDOW.as_secs()
						);
						std::process::abort();
					};
				}
				was_armed = armed;
				std::thread::sleep(std::time::Duration::from_millis(10));
			}
		});
	});
}

pub fn disarm_saturation_crash() {
	SAT_ARMED.store(0 == 1, Ordering::SeqCst);
}

const AMDKFD_IOC_SMI_EVENTS: libc::c_ulong = 0xC008_4B1F;

#[repr(C)]
struct SmiArgs {
	gpuid: u32,
	anon_fd: u32,
}

enum GpuEvent {
	Thrash,
	Restored,
	Other,
}

fn gpu_id() -> Option<u32> {
	for e in std::fs::read_dir("/sys/class/kfd/kfd/topology/nodes").ok()? {
		let p = e.ok()?.path().join("gpu_id");
		let found = std::fs::read_to_string(&p)
			.ok()
			.and_then(|s| s.trim().parse::<u32>().ok())
			.filter(|id| *id != 0);
		let Some(id) = found else {
			continue;
		};
		return Some(id);
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
		let raw = unsafe { libc::open(c"/dev/kfd".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
		let kfd = match raw.cmp(&0) {
			std::cmp::Ordering::Less => {
				eprintln!("thrash watchdog: /dev/kfd: {}", std::io::Error::last_os_error());
				return;
			}
			std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
				unsafe { std::fs::File::from_raw_fd(raw) }
			}
		};
		let mut args = SmiArgs { gpuid: gpu, anon_fd: 0 };
		let rc = unsafe { libc::ioctl(kfd.as_raw_fd(), AMDKFD_IOC_SMI_EVENTS, &mut args) };
		let std::cmp::Ordering::Equal = rc.cmp(&0) else {
			eprintln!("thrash watchdog: SMI ioctl: {}", std::io::Error::last_os_error());
			return;
		};
		let mut smi = unsafe { std::fs::File::from_raw_fd(args.anon_fd as i32) };
		let mask: u64 = [1u32, 2, 5, 6, 7, 8, 9, 10, 11]
			.iter()
			.map(|i| 1u64 << (i - 1))
			.sum();
		match (&smi).write_all(&mask.to_le_bytes()) {
			Err(e) => {
				eprintln!("thrash watchdog: mask write: {e}");
			}
			Ok(()) => {
				std::thread::spawn(move || {
					let mut buf = [0u8; 1024];
					loop {
						let n = match smi.read(&mut buf) {
							Err(_e) => return,
							Ok(0) => return,
							Ok(n) => n,
						};
						for ev in String::from_utf8_lossy(&buf[..n]).split_terminator('\n') {
							let id = ev
								.split_whitespace()
								.next()
								.and_then(|t| u32::from_str_radix(t, 16).ok())
								.unwrap_or(0);
							let kind = Some(id)
								.filter(|v| *v == 9 || *v == 11)
								.map(|_v| GpuEvent::Thrash)
								.or_else(|| Some(id).filter(|v| *v == 10).map(|_v| GpuEvent::Restored))
								.unwrap_or(GpuEvent::Other);
							match kind {
								GpuEvent::Thrash => {
									eprintln!(
										"\x1b[1;31mgpu thrash\x1b[0m  {}  — driver evicted our queues/mappings; aborting per fail-clean",
										ev.trim()
									);
									std::process::abort();
								}
								GpuEvent::Restored => {
									eprintln!("\x1b[33mgpu event\x1b[0m  queue restored  {}", ev.trim())
								}
								GpuEvent::Other => {
									eprintln!("\x1b[33mgpu event\x1b[0m  {}", ev.trim())
								}
							}
						}
					}
				});
			}
		}
	});
}
