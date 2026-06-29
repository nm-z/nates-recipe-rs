use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

// ── FFI: ropex.hip ───────────────────────────────────────────────────────────
// C: launch_ropex_qk(q, k, pos, qo, ko, n_rows, d, theta, stream)
//    const double*, const double*, const double*, double*, double*, int, int, double, hipStream_t
unsafe extern "C" {
	fn launch_ropex_qk(
		q: *const c_void,
		k: *const c_void,
		pos: *const c_void,
		qo: *mut c_void,
		ko: *mut c_void,
		n_rows: i32,
		d: i32,
		theta: f64,
		stream: *mut c_void,
	);
	fn launch_ropex_qk_heads(
		q: *mut c_void,
		k: *mut c_void,
		m: i32,
		d: i32,
		heads: i32,
		seq: i32,
		theta: f64,
		sgn: f64,
		stream: *mut c_void,
	);
}

/// Default RoPE base frequency (the value from the RoPE paper).
pub const ROPE_THETA: f64 = 10000.0;

/// Multi-head rotary position embedding applied IN-PLACE to Q and K, each
/// `(m, d)` row-major with `d = heads*hd`. Rotates per head by the token's
/// sequence position (`row % seq`). `sgn`: `+1.0` forward (Q_rot = R(angle)·Q),
/// `-1.0` backward (un-rotate a gradient — a rotation's inverse is itself negated).
pub fn gpu_rope_qk_heads_inplace(
	q: &GpuBuffer,
	k: &GpuBuffer,
	m: usize,
	d: usize,
	heads: usize,
	seq: usize,
	sgn: f64,
) {
	unsafe {
		launch_ropex_qk_heads(
			q.ptr_raw(),
			k.ptr_raw(),
			m as i32,
			d as i32,
			heads as i32,
			seq as i32,
			ROPE_THETA,
			sgn,
			std::ptr::null_mut(),
		);
	}
	check_launch();
}

// ── gpu_rope_qk ──────────────────────────────────────────────────────────────
// Q, K: f64 (n_rows, dim) row-major. positions: f64 (n_rows,), cast to int in-kernel.
// NeoX half-split rotary: pair (i, i+dim/2) rotated by angle = pos / theta^(2i/dim).
// Same rotation applied to both Q and K. theta: base frequency (e.g. 10000.0).
// Returns freshly-allocated rotated (Q, K) of the same (n_rows, dim) shape.
pub fn gpu_rope_qk(
	q: &GpuBuffer,
	k: &GpuBuffer,
	positions: &GpuBuffer,
	n_rows: usize,
	dim: usize,
	theta: f64,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
	let qo = GpuBuffer::alloc(n_rows * dim)?;
	let ko = GpuBuffer::alloc(n_rows * dim)?;
	unsafe {
		launch_ropex_qk(
			q.ptr_raw() as *const c_void,
			k.ptr_raw() as *const c_void,
			positions.ptr_raw() as *const c_void,
			qo.ptr_raw(),
			ko.ptr_raw(),
			n_rows as i32,
			dim as i32,
			theta,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok((qo, ko))
}

#[cfg(test)]
mod tests {
	use super::*;

	// The multi-head RoPE backward (sgn=-1) must be the exact inverse rotation of
	// the forward (sgn=+1) — i.e. for L = Σ g·rope(x,+1), dL/dx = rope(g,-1).
	// This is precisely the chain-rule step attn_backward relies on (un-rotating
	// the Q,K gradients), checked against a finite difference of the forward.
	#[test]
	fn rope_heads_backward_is_inverse_rotation() {
		crate::hip::set_device(0).expect("set_device");
		let (m, d, heads, seq) = (6usize, 8usize, 2usize, 3usize); // hd=4, half=2
		let xq: Vec<f64> = (0..m * d).map(|i| ((i * 7 % 13) as f64 - 6.0) * 0.1).collect();
		let xk: Vec<f64> = (0..m * d).map(|i| ((i * 5 % 11) as f64 - 5.0) * 0.1).collect();
		let g: Vec<f64> = (0..m * d).map(|i| ((i * 3 % 17) as f64 - 8.0) * 0.1).collect();

		// analytic: dL/dq = R(-angle)·g = rope(g, -1) on the q slot
		let gq = GpuBuffer::upload(&g).expect("g");
		let gk = GpuBuffer::upload(&vec![0.0f64; m * d]).expect("gk");
		gpu_rope_qk_heads_inplace(&gq, &gk, m, d, heads, seq, -1.0);
		let analytic = {
			let mut v = vec![0.0f64; m * d];
			gq.download(&mut v).expect("dl");
			v
		};

		let eps = 1e-6;
		let loss = |x: &[f64]| -> f64 {
			let q = GpuBuffer::upload(x).expect("q");
			let k = GpuBuffer::upload(&xk).expect("k");
			gpu_rope_qk_heads_inplace(&q, &k, m, d, heads, seq, 1.0);
			let mut o = vec![0.0f64; m * d];
			q.download(&mut o).expect("o");
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
