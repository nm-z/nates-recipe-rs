use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_momentum_update(
		w: *mut c_void,
		v: *mut c_void,
		g: *const c_void,
		lr: *const c_void,
		momentum: *const c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_rmsprop_update(
		w: *mut c_void,
		cache: *mut c_void,
		g: *const c_void,
		lr: *const c_void,
		decay: *const c_void,
		eps: *const c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_adagrad_update(
		w: *mut c_void,
		accum: *mut c_void,
		g: *const c_void,
		lr: *const c_void,
		eps: *const c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_lamb_phase1(
		w: *mut c_void,
		m: *mut c_void,
		v: *mut c_void,
		g: *const c_void,
		b1: *const c_void,
		b2: *const c_void,
		eps: *const c_void,
		wd: *const c_void,
		t: i32,
		n: i32,
		tmp_upd: *mut c_void,
		w_norm_sq: *mut c_void,
		u_norm_sq: *mut c_void,
		stream: *mut c_void,
	);
	fn launch_lamb_phase2(
		w: *mut c_void,
		tmp_upd: *const c_void,
		lr: *const c_void,
		w_norm_sq: *const c_void,
		u_norm_sq: *const c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_lion_update(
		w: *mut c_void,
		m: *mut c_void,
		g: *const c_void,
		lr: *const c_void,
		b1: *const c_void,
		b2: *const c_void,
		wd: *const c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_nadam_update(
		w: *mut c_void,
		m: *mut c_void,
		v: *mut c_void,
		g: *const c_void,
		lr: *const c_void,
		b1: *const c_void,
		b2: *const c_void,
		eps: *const c_void,
		t: i32,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_clip_value(
		x: *mut c_void,
		lo: *const c_void,
		hi: *const c_void,
		n: i32,
		stream: *mut c_void,
	);
}

/// Momentum SGD: v = momentum*v - lr*g; w += v (in-place, updates both w and v).
pub fn gpu_momentum_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	momentum: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
	v: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_momentum_update(
			w.ptr_raw(),
			v.ptr_raw(),
			g.ptr_raw() as *const c_void,
			lr.ptr_raw() as *const c_void,
			momentum.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

/// RMSProp: cache = decay*cache + (1-decay)*g^2; w -= lr*g/(sqrt(cache)+eps) (in-place).
pub fn gpu_rmsprop_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	decay: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
	cache: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_rmsprop_update(
			w.ptr_raw(),
			cache.ptr_raw(),
			g.ptr_raw() as *const c_void,
			lr.ptr_raw() as *const c_void,
			decay.ptr_raw() as *const c_void,
			eps.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

/// Adagrad: accum += g^2; w -= lr*g/(sqrt(accum)+eps) (in-place).
pub fn gpu_adagrad_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
	accum: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_adagrad_update(
			w.ptr_raw(),
			accum.ptr_raw(),
			g.ptr_raw() as *const c_void,
			lr.ptr_raw() as *const c_void,
			eps.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

/// LAMB phase 1: Adam moments + per-element update into tmp_upd, accumulating ||w||^2
/// and ||update||^2 on device (norm buffers zeroed here before the atomicAdd pass).
pub fn gpu_lamb_phase1(
	g: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	eps: &GpuBuffer,
	wd: &GpuBuffer,
	t: usize,
	n: usize,
	w: &GpuBuffer,
	m: &GpuBuffer,
	v: &GpuBuffer,
	tmp_upd: &GpuBuffer,
	w_norm_sq: &GpuBuffer,
	u_norm_sq: &GpuBuffer,
) -> Result<(), HipError> {
	w_norm_sq.memset_zero(8)?;
	u_norm_sq.memset_zero(8)?;
	unsafe {
		launch_lamb_phase1(
			w.ptr_raw(),
			m.ptr_raw(),
			v.ptr_raw(),
			g.ptr_raw() as *const c_void,
			b1.ptr_raw() as *const c_void,
			b2.ptr_raw() as *const c_void,
			eps.ptr_raw() as *const c_void,
			wd.ptr_raw() as *const c_void,
			t as i32,
			n as i32,
			tmp_upd.ptr_raw(),
			w_norm_sq.ptr_raw(),
			u_norm_sq.ptr_raw(),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

/// LAMB phase 2: w -= lr * (||w||/||update||) * tmp_upd (trust ratio 1.0 if a norm is 0).
/// Norm buffers are the device-side squared norms produced by phase1 — no D2H roundtrip.
pub fn gpu_lamb_phase2(
	tmp_upd: &GpuBuffer,
	lr: &GpuBuffer,
	w_norm_sq: &GpuBuffer,
	u_norm_sq: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_lamb_phase2(
			w.ptr_raw(),
			tmp_upd.ptr_raw() as *const c_void,
			lr.ptr_raw() as *const c_void,
			w_norm_sq.ptr_raw() as *const c_void,
			u_norm_sq.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// not-an-op: driver — LAMB orchestration; allocates workspace, uploads scalars, composes
// gpu_lamb_phase1 + gpu_lamb_phase2 with norms staying device-side (no D2H roundtrip).
pub fn gpu_lamb_update(
	w: &GpuBuffer,
	m: &GpuBuffer,
	v: &GpuBuffer,
	g: &GpuBuffer,
	lr: f64,
	b1: f64,
	b2: f64,
	eps: f64,
	wd: f64,
	t: i32,
	n: usize,
) -> Result<(), HipError> {
	let tmp_upd = GpuBuffer::alloc(n)?;
	let w_norm_sq = GpuBuffer::alloc(1)?;
	let u_norm_sq = GpuBuffer::alloc(1)?;
	let lr_b = GpuBuffer::upload(&[lr])?;
	let b1_b = GpuBuffer::upload(&[b1])?;
	let b2_b = GpuBuffer::upload(&[b2])?;
	let eps_b = GpuBuffer::upload(&[eps])?;
	let wd_b = GpuBuffer::upload(&[wd])?;

	gpu_lamb_phase1(
		g, &b1_b, &b2_b, &eps_b, &wd_b, t as usize, n, w, m, v, &tmp_upd, &w_norm_sq,
		&u_norm_sq,
	)?;
	gpu_lamb_phase2(&tmp_upd, &lr_b, &w_norm_sq, &u_norm_sq, n, w)?;
	Ok(())
}

/// Lion: update = sign(b1*m + (1-b1)*g); w -= lr*(update + wd*w); m = b2*m + (1-b2)*g (in-place).
pub fn gpu_lion_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	wd: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
	m: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_lion_update(
			w.ptr_raw(),
			m.ptr_raw(),
			g.ptr_raw() as *const c_void,
			lr.ptr_raw() as *const c_void,
			b1.ptr_raw() as *const c_void,
			b2.ptr_raw() as *const c_void,
			wd.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

/// Nadam (Nesterov-accelerated Adam): uses next-step bias-corrected first moment.
pub fn gpu_nadam_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	eps: &GpuBuffer,
	t: usize,
	n: usize,
	w: &GpuBuffer,
	m: &GpuBuffer,
	v: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_nadam_update(
			w.ptr_raw(),
			m.ptr_raw(),
			v.ptr_raw(),
			g.ptr_raw() as *const c_void,
			lr.ptr_raw() as *const c_void,
			b1.ptr_raw() as *const c_void,
			b2.ptr_raw() as *const c_void,
			eps.ptr_raw() as *const c_void,
			t as i32,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

/// In-place elementwise clamp: x[i] = clamp(x[i], lo, hi).
pub fn gpu_clip_value(
	lo: &GpuBuffer,
	hi: &GpuBuffer,
	n: usize,
	x: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_clip_value(
			x.ptr_raw(),
			lo.ptr_raw() as *const c_void,
			hi.ptr_raw() as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
