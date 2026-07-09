use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_ropex_qk(
		q: *const c_void,
		k: *const c_void,
		pos: *const c_void,
		qo: *mut c_void,
		ko: *mut c_void,
		n_rows: i32,
		d: i32,
		theta: *const c_void,
		stream: *mut c_void,
	);
	fn launch_ropex_qk_heads(
		q: *mut c_void,
		k: *mut c_void,
		m: i32,
		d: i32,
		heads: i32,
		seq: i32,
		theta: *const c_void,
		sgn: *const c_void,
		stream: *mut c_void,
	);
}

pub const ROPE_THETA: f64 = 10000.0;

pub fn gpu_rope_qk_heads_inplace(
	sgn: &GpuBuffer,
	theta: &GpuBuffer,
	m: usize,
	d: usize,
	heads: usize,
	seq: usize,
	q: &GpuBuffer,
	k: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_ropex_qk_heads(
			q.ptr_raw(),
			k.ptr_raw(),
			m as i32,
			d as i32,
			heads as i32,
			seq as i32,
			theta.ptr_raw() as *const c_void,
			sgn.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_rope_qk(
	q: &GpuBuffer,
	k: &GpuBuffer,
	positions: &GpuBuffer,
	theta: &GpuBuffer,
	n_rows: usize,
	dim: usize,
	qo: &GpuBuffer,
	ko: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_ropex_qk(
			q.ptr_raw() as *const c_void,
			k.ptr_raw() as *const c_void,
			positions.ptr_raw() as *const c_void,
			qo.ptr_raw(),
			ko.ptr_raw(),
			n_rows as i32,
			dim as i32,
			theta.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rope_heads_backward_is_inverse_rotation() {
		crate::hip::set_device(0).expect("set_device");
		let (m, d, heads, seq) = (6usize, 8usize, 2usize, 3usize);
		let xq: Vec<f64> = (0..m * d).map(|i| ((i * 7 % 13) as f64 - 6.0) * 0.1).collect();
		let xk: Vec<f64> = (0..m * d).map(|i| ((i * 5 % 11) as f64 - 5.0) * 0.1).collect();
		let g: Vec<f64> = (0..m * d).map(|i| ((i * 3 % 17) as f64 - 8.0) * 0.1).collect();

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
			unsafe { gq.download_async(&mut v, std::ptr::null_mut()) }.expect("dl");
			crate::hip::device_synchronize().expect("dl sync");
			v
		};

		let eps = 1e-6;
		let loss = |x: &[f64]| -> f64 {
			let q = GpuBuffer::alloc(x.len()).expect("q");
			q.load(x).expect("q load");
			let k = GpuBuffer::alloc(xk.len()).expect("k");
			k.load(&xk).expect("k load");
			gpu_rope_qk_heads_inplace(&sgn_fwd, &theta, m, d, heads, seq, &q, &k).expect("rope fwd");
			let mut o = vec![0.0f64; m * d];
			unsafe { q.download_async(&mut o, std::ptr::null_mut()) }.expect("o");
			crate::hip::device_synchronize().expect("o sync");
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
		assert!(maxdiff < 1e-6, "rope backward != inverse rotation: {maxdiff:e}");
	}
}
