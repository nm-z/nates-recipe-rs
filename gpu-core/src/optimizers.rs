use crate::HipError;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

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

#[inline]
pub fn gpu_momentum_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	momentum: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
	v: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	// SAFETY: all buffer pointers are live device allocations sized for n elements.
	unsafe {
		launch_momentum_update(
			w.ptr_raw(),
			v.ptr_raw(),
			g.ptr_raw().cast_const(),
			lr.ptr_raw().cast_const(),
			momentum.ptr_raw().cast_const(),
			n_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_rmsprop_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	decay: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
	cache: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	// SAFETY: all buffer pointers are live device allocations sized for n elements.
	unsafe {
		launch_rmsprop_update(
			w.ptr_raw(),
			cache.ptr_raw(),
			g.ptr_raw().cast_const(),
			lr.ptr_raw().cast_const(),
			decay.ptr_raw().cast_const(),
			eps.ptr_raw().cast_const(),
			n_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_adagrad_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
	accum: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffer pointers are live device allocations sized for n elements.
	unsafe {
		launch_adagrad_update(
			w.ptr_raw(),
			accum.ptr_raw(),
			g.ptr_raw().cast_const(),
			lr.ptr_raw().cast_const(),
			eps.ptr_raw().cast_const(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_lamb_phase1(
	grad: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	eps: &GpuBuffer,
	wd: &GpuBuffer,
	step: usize,
	count: usize,
	weight: &GpuBuffer,
	moment: &GpuBuffer,
	velocity: &GpuBuffer,
	tmp_upd: &GpuBuffer,
	w_norm_sq: &GpuBuffer,
	u_norm_sq: &GpuBuffer,
) -> Result<(), HipError> {
	w_norm_sq.memset_zero(8)?;
	u_norm_sq.memset_zero(8)?;
	let step_i = ci(step)?;
	let count_i = ci(count)?;
	// SAFETY: buffer pointers are valid for the launch and the step/count bounds are checked above.
	unsafe {
		launch_lamb_phase1(
			weight.ptr_raw(),
			moment.ptr_raw(),
			velocity.ptr_raw(),
			grad.ptr_raw().cast_const(),
			b1.ptr_raw().cast_const(),
			b2.ptr_raw().cast_const(),
			eps.ptr_raw().cast_const(),
			wd.ptr_raw().cast_const(),
			step_i,
			count_i,
			tmp_upd.ptr_raw(),
			w_norm_sq.ptr_raw(),
			u_norm_sq.ptr_raw(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_lamb_phase2(
	tmp_upd: &GpuBuffer,
	lr: &GpuBuffer,
	w_norm_sq: &GpuBuffer,
	u_norm_sq: &GpuBuffer,
	n: usize,
	w: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	// SAFETY: buffer pointers are valid for the launch and the element count is checked above.
	unsafe {
		launch_lamb_phase2(
			w.ptr_raw(),
			tmp_upd.ptr_raw().cast_const(),
			lr.ptr_raw().cast_const(),
			w_norm_sq.ptr_raw().cast_const(),
			u_norm_sq.ptr_raw().cast_const(),
			n_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
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
	let n_i = ci(n)?;
	// SAFETY: every pointer is a live GpuBuffer device allocation sized for n elements.
	unsafe {
		launch_lion_update(
			w.ptr_raw(),
			m.ptr_raw(),
			g.ptr_raw().cast_const(),
			lr.ptr_raw().cast_const(),
			b1.ptr_raw().cast_const(),
			b2.ptr_raw().cast_const(),
			wd.ptr_raw().cast_const(),
			n_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_nadam_update(
	grad: &GpuBuffer,
	lr: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	eps: &GpuBuffer,
	step: usize,
	count: usize,
	weight: &GpuBuffer,
	moment: &GpuBuffer,
	velocity: &GpuBuffer,
) -> Result<(), HipError> {
	let step_i = ci(step)?;
	let count_i = ci(count)?;
	// SAFETY: every pointer is a live GpuBuffer device allocation sized for count elements.
	unsafe {
		launch_nadam_update(
			weight.ptr_raw(),
			moment.ptr_raw(),
			velocity.ptr_raw(),
			grad.ptr_raw().cast_const(),
			lr.ptr_raw().cast_const(),
			b1.ptr_raw().cast_const(),
			b2.ptr_raw().cast_const(),
			eps.ptr_raw().cast_const(),
			step_i,
			count_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_clip_value(
	lo: &GpuBuffer,
	hi: &GpuBuffer,
	n: usize,
	x: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	// SAFETY: every pointer is a live GpuBuffer device allocation sized for n elements.
	unsafe {
		launch_clip_value(
			x.ptr_raw(),
			lo.ptr_raw().cast_const(),
			hi.ptr_raw().cast_const(),
			n_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
