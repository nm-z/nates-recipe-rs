#![allow(unsafe_code, reason = "FFI to HIP runtime")]
use gpu_core::log::{Errored, Opt, Write, gpu, set_opt};
use gpu_core::memory::GpuBuffer;
use std::cmp::Ordering;
use std::process;
use std::ptr;

fn main() -> Result<(), Errored> {
	let n = (64usize << 20) / 8;
	let host = vec![4.0f64; n];
	let x = GpuBuffer::alloc(n).map_err(|e| Errored::new(format!("probe: alloc: {e}")))?;
	x.load(&host).map_err(|e| Errored::new(format!("probe: upload: {e}")))?;
	gpu_core::math_ops::gpu_rsqrt(&x, n, &x).map_err(|e| Errored::new(format!("probe: kernel: {e}")))?;
	let mut back = vec![0.0f64; n];
	unsafe { x.download_async(&mut back, ptr::null_mut()) }
		.map_err(|e| Errored::new(format!("probe: download: {e}")))?;
	gpu_core::hip::device_synchronize().map_err(|e| Errored::new(format!("probe: download sync: {e}")))?;
	for i in 0..back.len() {
		let v = back[i];
		match (v - 0.5).abs().partial_cmp(&1e-12) {
			Some(Ordering::Greater) => {
				drop(Write::err(&format!("probe: mismatch at {i}: {v}")));
				process::exit(1);
			}
			Some(Ordering::Less) | Some(Ordering::Equal) | None => {
				continue;
			}
		}
	}
	gpu_core::hip::device_synchronize().map_err(|e| Errored::new(format!("probe: device sync: {e}")))?;
	set_opt(Opt {
		gpu: true,
		..Opt::default()
	});
	Write::line(gpu, "probe: ok");
	Ok(())
}
