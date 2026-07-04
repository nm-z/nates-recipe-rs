//! Kernel-side HW truth: the KFD SMI thrash watchdog. Subscribes to the
//! driver's per-process event stream (/dev/kfd, kfd_ioctl.h:528-547) and turns
//! silent driver pathology into loud lines: queue eviction of OUR compute
//! queues aborts (fail clean beats limping through a thrash storm), migrations
//! and throttles print. The HSA memory-fault autopsy lives in hip.rs; this
//! file is everything the kernel can tell us that HIP cannot.

use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};

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
