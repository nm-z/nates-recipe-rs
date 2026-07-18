#![allow(unsafe_code, reason = "FFI to HIP runtime")]
use core::cmp::Ordering;
use core::ptr;
use gpu_core::log::{Errored, Opt, Write, gpu, set_opt};
use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, math_ops};
use std::process;

fn main() -> Result<(), Errored> {
	let n = 8usize << 20i32;
	let host = vec![4.0f64; n];
	let x = GpuBuffer::alloc(n).map_err(|e| return Errored::new(format!("probe: alloc: {e}")))?;
	x.load(&host)
		.map_err(|e| return Errored::new(format!("probe: upload: {e}")))?;
	math_ops::gpu_rsqrt(&x, n, &x)
		.map_err(|e| return Errored::new(format!("probe: kernel: {e}")))?;
	let mut back = vec![0.0f64; n];
	// SAFETY: back is a live host slice of n f64 and x is a device buffer of n elements; the null stream is the default stream.
	unsafe { x.download_async(&mut back, ptr::null_mut()) }
		.map_err(|e| return Errored::new(format!("probe: download: {e}")))?;
	hip::device_synchronize()
		.map_err(|e| return Errored::new(format!("probe: download sync: {e}")))?;
	for (i, v) in back.iter().enumerate() {
		if (v - 0.5f64).abs().partial_cmp(&1e-12f64) == Some(Ordering::Greater) {
			Write::error(format!("probe: mismatch at {i}: {v}"));
			process::exit(1);
		}
	}
	hip::device_synchronize()
		.map_err(|e| return Errored::new(format!("probe: device sync: {e}")))?;
	set_opt(Opt {
		gpu: true,
		..Opt::default()
	});
	Write::line(gpu, "probe: ok");
	return Ok(());
}
