use gpu_core::memory::GpuBuffer;
use std::process::Command;

// Repro of the cookbook "continue anyway? y" crash: allocations fill VRAM and
// the first ask past the mappable ceiling dies in the uncatchable
// VmHeap::MapPhysMemory assert (SIGABRT) instead of returning a clean OOM.
// The child fills VRAM in 512 MB steps holding every buffer; the only
// acceptable outcome is try_alloc_bytes -> None (exit 0). A signal 6 exit is
// the bug.
#[test]
fn oversize_alloc_is_clean_oom_not_abort() {
	if std::env::var("OVERSIZE_OOM_CHILD").is_ok() {
		let mut held = Vec::new();
		loop {
			match GpuBuffer::try_alloc_bytes(1usize << 29) {
				Some(b) => held.push(b),
				None => std::process::exit(0),
			}
		}
	}
	let exe = std::env::current_exe().expect("current_exe");
	let mut cmd = Command::new(exe);
	cmd.args([
		"--exact",
		"oversize_oom::oversize_alloc_is_clean_oom_not_abort",
		"--nocapture",
		"--test-threads=1",
	])
	.env("OVERSIZE_OOM_CHILD", "1");
	// No core dump from the expected-abort child.
	unsafe {
		use std::os::unix::process::CommandExt;
		cmd.pre_exec(|| {
			let lim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
			libc::setrlimit(libc::RLIMIT_CORE, &lim);
			Ok(())
		});
	}
	let out = cmd.output().expect("spawn child");
	assert!(
		out.status.success(),
		"oversize alloc did not fail cleanly: {:?}\nchild stderr tail: {}",
		out.status,
		String::from_utf8_lossy(&out.stderr)
			.lines()
			.rev()
			.take(6)
			.collect::<Vec<_>>()
			.join(" | ")
	);
}
