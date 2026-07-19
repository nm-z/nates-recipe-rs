use gpu_core::memory::GpuBuffer;
use std::env;
use std::process::{self, Command};

#[test]
fn oversize_request_is_clean_oom_not_abort() {
	if env::var("OVERSIZE_OOM_CHILD").is_ok() {
		let mut held = Vec::new();
		loop {
			eprintln!("fill: holding {} x 512MB, asking next", held.len());
			match GpuBuffer::try_alloc_bytes(1usize << 29) {
				Some(b) => held.push(b),
				None => process::exit(0),
			}
		}
	}
	let exe = env::current_exe().expect("current_exe");
	let mut cmd = Command::new(exe);
	cmd.args([
		"--exact",
		"oversize_oom::oversize_request_is_clean_oom_not_abort",
		"--nocapture",
		"--test-threads=1",
	])
	.env("OVERSIZE_OOM_CHILD", "1")
	.env_remove("RECIPE_SKIP_POOL_WARM");
	unsafe {
		use std::os::unix::process::CommandExt;
		cmd.pre_exec(|| {
			let lim = libc::rlimit {
				rlim_cur: 0,
				rlim_max: 0,
			};
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
