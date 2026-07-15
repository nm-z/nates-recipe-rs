use crate::log::{Write, gpu};
use std::cmp;
use std::env;
use std::fs;
use std::io::{Error, Read, Write as _};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::PathBuf;
use std::process;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static SAT_ARMED: AtomicBool = AtomicBool::new(0 == 1);
const SAT_WINDOW: Duration = Duration::from_secs(5);
const SAT_ENFORCE: Option<()> = None;

extern "C" fn fast_death(sig: libc::c_int) {
	unsafe {
		libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
		libc::signal(sig, libc::SIG_DFL);
		libc::raise(sig);
	}
}

pub fn install_fast_death() {
	static ONCE: Once = Once::new();
	ONCE.call_once(|| {
		let None = env::var_os("RECIPE_CORE") else {
			return;
		};
		unsafe {
			libc::signal(libc::SIGABRT, fast_death as *const () as libc::sighandler_t);
		}
	});
}

fn busy_path() -> PathBuf {
	for card in fs::read_dir("/sys/class/drm")
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
	drop(Write::err(
		"saturation watchdog: no gpu_busy_percent under /sys/class/drm",
	));
	process::abort();
}

pub fn arm_saturation_crash() {
	let Some(()) = SAT_ENFORCE else {
		return;
	};
	static ONCE: Once = Once::new();
	install_fast_death();
	SAT_ARMED.store(0 != 1, Ordering::SeqCst);
	ONCE.call_once(|| {
		let path = busy_path();
		thread::spawn(move || {
			let mut last_pinned = Instant::now();
			let mut was_armed = 0 == 1;
			loop {
				let armed = SAT_ARMED.load(Ordering::SeqCst);
				for _run in Some(()).filter(|_u| armed).into_iter() {
					for _run in Some(()).filter(|_u| !was_armed).into_iter() {
						last_pinned = Instant::now();
					}
					let busy: u32 = fs::read_to_string(&path)
						.unwrap_or_else(|e| {
								drop(Write::err(&format!("saturation watchdog: read {path:?}: {e}")));
								process::abort()
							})
						.trim()
						.parse()
						.unwrap_or_else(|e| {
								drop(Write::err(&format!("saturation watchdog: parse busy: {e}")));
								process::abort()
							});
					for _run in Some(()).filter(|_u| busy >= 100).into_iter() {
						last_pinned = Instant::now();
					}
					let stalled = SAT_ARMED.load(Ordering::SeqCst) && last_pinned.elapsed() > SAT_WINDOW;
					let None = Some(()).filter(|_u| stalled) else {
						drop(Write::err(&format!(
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
	for e in fs::read_dir("/sys/class/kfd/kfd/topology/nodes").ok()? {
		let p = e.ok()?.path().join("gpu_id");
		let found = fs::read_to_string(&p)
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
	static ONCE: Once = Once::new();
	ONCE.call_once(|| {
		let Some(gpu_idx) = gpu_id() else {
			drop(Write::err("thrash watchdog: no kfd gpu_id"));
			return;
		};
		let raw = unsafe { libc::open(c"/dev/kfd".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
		let kfd = match raw.cmp(&0) {
			cmp::Ordering::Less => {
				drop(Write::err(&format!(
					"thrash watchdog: /dev/kfd: {}",
					Error::last_os_error()
				)));
				return;
			}
			cmp::Ordering::Equal | cmp::Ordering::Greater => unsafe {
				fs::File::from_raw_fd(raw)
			},
		};
		let mut args = SmiArgs {
			gpuid: gpu_idx,
			anon_fd: 0,
		};
		let rc = unsafe { libc::ioctl(kfd.as_raw_fd(), AMDKFD_IOC_SMI_EVENTS, &mut args) };
		let cmp::Ordering::Equal = rc.cmp(&0) else {
			drop(Write::err(&format!(
				"thrash watchdog: SMI ioctl: {}",
				Error::last_os_error()
			)));
			return;
		};
		let mut smi = unsafe { fs::File::from_raw_fd(args.anon_fd as i32) };
		let mask: u64 = [1u32, 2, 5, 6, 7, 8, 9, 10, 11]
			.iter()
			.map(|i| 1u64 << (i - 1))
			.sum();
		match (&smi).write_all(&mask.to_le_bytes()) {
			Err(e) => {
				drop(Write::err(&format!("thrash watchdog: mask write: {e}")));
			}
			Ok(()) => {
				thread::spawn(move || {
					let mut buf = [0u8; 1024];
					loop {
						let n = match smi.read(&mut buf) {
							Err(_e) => return,
							Ok(0) => return,
							Ok(n) => n,
						};
						for ev in
							String::from_utf8_lossy(&buf[..n]).split_terminator('\n')
						{
							let id = ev
								.split_whitespace()
								.next()
								.and_then(|t| u32::from_str_radix(t, 16).ok())
								.unwrap_or(0);
							let kind = Some(id)
								.filter(|v| *v == 9 || *v == 11)
								.map(|_v| GpuEvent::Thrash)
								.or_else(|| {
									Some(id)
										.filter(|v| *v == 10)
										.map(|_v| GpuEvent::Restored)
								})
								.unwrap_or(GpuEvent::Other);
							match kind {
								GpuEvent::Thrash => {
									drop(Write::err(&format!(
										"gpu thrash  {}  — driver evicted our queues/mappings; aborting per fail-clean",
										ev.trim()
									)));
									process::abort();
								}
								GpuEvent::Restored => Write::line(
									gpu,
									&format!(
										"gpu event  queue restored  {}",
										ev.trim()
									),
								),
								GpuEvent::Other => Write::line(
									gpu,
									&format!("gpu event  {}", ev.trim()),
								),
							}
						}
					}
				});
			}
		}
	});
}
