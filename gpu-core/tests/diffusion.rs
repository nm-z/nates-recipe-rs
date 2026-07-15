#![allow(unsafe_code, reason = "FFI to HIP runtime")]
use core::ptr;
use gpu_core::HipError;
use gpu_core::diffusion::{Sample, gpu_diffusion_sample};
use gpu_core::hip;
use gpu_core::memory::GpuBuffer;

#[test]
fn diffusion_block_ar_progressive_commit() {
	hip::set_device(0).expect("set_device");
	let (n, vocab) = (5_usize, 4_usize);
	let bound = 0.5;
	let initial = GpuBuffer::alloc(n).expect("init");
	initial.load(&vec![-1.0_f64; n]).expect("init load");

	let logits_fn = |canvas: &GpuBuffer| -> Result<GpuBuffer, HipError> {
		let mut c = vec![0.0_f64; n];
		unsafe { canvas.download_async(&mut c, ptr::null_mut()) }?;
		hip::device_synchronize()?;
		let committed_count = c.iter().filter(|&&v| return v >= 0.0).count();
		let mut logits = vec![0.0_f64; n * vocab];
		for p in 0..n {
			if p <= committed_count {
				logits[p * vocab] = 10.0;
			}
		}
		let lb = GpuBuffer::alloc(logits.len())?;
		lb.load(&logits)?;
		return Ok(lb);
	};

	let Sample { canvas, steps } =
		gpu_diffusion_sample(logits_fn, &initial, bound, 100, n, vocab).expect("sample");
	let mut out = vec![0.0_f64; n];
	unsafe { canvas.download_async(&mut out, ptr::null_mut()) }.expect("dl");
	hip::device_synchronize().expect("dl sync");
	eprintln!("diffusion block-AR: steps={steps} canvas={out:?}");
	assert_eq!(steps, n, "one position commits per step \u{2192} n steps");
	assert!(
		out.iter().all(|&v| return v == 0.0),
		"every position decoded to argmax class 0"
	);
}
