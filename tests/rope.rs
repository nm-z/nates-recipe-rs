#![allow(unsafe_code)]
use core::ptr;
use gpu_core::memory::GpuBuffer;
use gpu_core::rope::{ROPE_THETA, gpu_rope_qk_heads_inplace};

#[test]
fn rope_heads_backward_is_inverse_rotation() {
	gpu_core::hip::set_device(0).expect("set_device");
	let (m, d, heads, seq) = (6usize, 8usize, 2usize, 3usize);
	let xq: Vec<f64> = (0..m * d)
		.map(|i| ((i * 7 % 13) as f64 - 6.0) * 0.1)
		.collect();
	let xk: Vec<f64> = (0..m * d)
		.map(|i| ((i * 5 % 11) as f64 - 5.0) * 0.1)
		.collect();
	let g: Vec<f64> = (0..m * d)
		.map(|i| ((i * 3 % 17) as f64 - 8.0) * 0.1)
		.collect();

	let theta = GpuBuffer::alloc(1).expect("theta");
	theta.load(&[ROPE_THETA]).expect("theta load");
	let sgn_bwd = GpuBuffer::alloc(1).expect("sgn_bwd");
	sgn_bwd.load(&[-1.0f64]).expect("sgn_bwd load");
	let sgn_fwd = GpuBuffer::alloc(1).expect("sgn_fwd");
	sgn_fwd.load(&[1.0f64]).expect("sgn_fwd load");
	let gq = GpuBuffer::alloc(g.len()).expect("g");
	gq.load(&g).expect("g load");
	let gk = GpuBuffer::alloc(m * d).expect("gk");
	gk.load(&vec![0.0f64; m * d]).expect("gk load");
	gpu_rope_qk_heads_inplace(&sgn_bwd, &theta, m, d, heads, seq, &gq, &gk).expect("rope bwd");
	let analytic = {
		let mut v = vec![0.0f64; m * d];
		unsafe { gq.download_async(&mut v, ptr::null_mut()) }.expect("dl");
		gpu_core::hip::device_synchronize().expect("dl sync");
		v
	};

	let eps = 1e-6;
	let loss = |x: &[f64]| -> f64 {
		let q = GpuBuffer::alloc(x.len()).expect("q");
		q.load(x).expect("q load");
		let k = GpuBuffer::alloc(xk.len()).expect("k");
		k.load(&xk).expect("k load");
		gpu_rope_qk_heads_inplace(&sgn_fwd, &theta, m, d, heads, seq, &q, &k)
			.expect("rope fwd");
		let mut o = vec![0.0f64; m * d];
		unsafe { q.download_async(&mut o, ptr::null_mut()) }.expect("o");
		gpu_core::hip::device_synchronize().expect("o sync");
		o.iter().zip(&g).map(|(a, b)| a * b).sum()
	};
	let mut maxdiff = 0.0f64;
	for i in 0..m * d {
		let mut xp = xq.clone();
		xp[i] += eps;
		let mut xm = xq.clone();
		xm[i] -= eps;
		let num = (loss(&xp) - loss(&xm)) / (2.0 * eps);
		maxdiff = maxdiff.max((num - analytic[i]).abs());
	}
	eprintln!("rope-heads backward vs finite-diff: maxdiff = {maxdiff:e}");
	assert!(
		maxdiff < 1e-6,
		"rope backward != inverse rotation: {maxdiff:e}"
	);
}
