use crate::HipError;
use crate::callspy;
use crate::hip::{check, hipGetLastError};
use crate::kernels::ci;
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

/// Checks the last HIP launch for an error, ticking the launch callspy counters.
fn cl() -> Result<(), HipError> {
	callspy::tick(&callspy::LAUNCH);
	callspy::tick(&callspy::GET_LAST_ERROR);
	// SAFETY: hipGetLastError only reads and clears the thread-local HIP error state; it dereferences no memory.
	return check(unsafe { hipGetLastError() });
}

unsafe extern "C" {
	fn launch_widen_bf16_f64(input: *const c_void, out: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_normx_rmsnorm(
		x: *const c_void,
		out: *mut c_void,
		gamma: *const c_void,
		rows: i32,
		cols: i32,
		eps: *const c_void,
		stream: *mut c_void,
	);
	fn launch_gqa_masked_attn(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		t: i32,
		nqh: i32,
		nkv: i32,
		hd: i32,
		prefix: i32,
		stream: *mut c_void,
	);
	fn launch_gelu_mul(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i64,
		stream: *mut c_void,
	);
	fn launch_glu_gelu(
		input: *const c_void,
		out: *mut c_void,
		rows: i32,
		half: i32,
		stream: *mut c_void,
	);
	fn launch_rope_partial(
		buf: *mut c_void,
		rows: i32,
		head_dim: i32,
		rotary_dim: i32,
		heads_per_tok: i32,
		theta: *const c_void,
		stream: *mut c_void,
	);
	fn launch_gemm_bt_f64(
		a: *const c_void,
		b: *const c_void,
		c: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		stream: *mut c_void,
	);
	fn launch_scale_f64(x: *mut c_void, scalar: *const c_void, n: i64, stream: *mut c_void);
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the kernel launch fails.
#[inline]
pub fn gpu_rope_partial(
	theta: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	buf: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: buf and theta are live GpuBuffer allocations and the dims are range-checked i32; the launcher only reads them.
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			ci(rows)?,
			ci(head_dim)?,
			ci(rotary_dim)?,
			ci(heads_per_tok)?,
			theta.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if `n` overflows `i64` or the kernel launch fails.
#[inline]
pub fn gpu_widen_bf16(raw: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: raw and out are live GpuBuffer allocations sized for n f64 elements; the launcher only reads/writes within them.
	unsafe {
		launch_widen_bf16_f64(
			raw.ptr_raw().cast_const(),
			out.ptr_raw(),
			i64::try_from(n).map_err(|_e| return HipError(1))?,
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the kernel launch fails.
#[inline]
pub fn gpu_rmsnorm_f64(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: x, gamma, eps, and out are live GpuBuffer allocations and the launcher only reads/writes within them.
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			gamma.ptr_raw().cast_const(),
			ci(rows)?,
			ci(cols)?,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the launch fails.
#[inline]
pub fn gpu_rmsnorm_f64_nogamma(
	x: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launcher reads valid device buffers with matching dims on the default stream.
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			ptr::null(),
			ci(rows)?,
			ci(cols)?,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the launch fails.
#[inline]
pub fn gpu_gqa_attn(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	t: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	prefix: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launcher reads valid device buffers with matching dims on the default stream.
	unsafe {
		launch_gqa_masked_attn(
			q.ptr_raw().cast_const(),
			k.ptr_raw().cast_const(),
			v.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(t)?,
			ci(nqh)?,
			ci(nkv)?,
			ci(hd)?,
			ci(prefix)?,
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if `n` overflows `i64` or the kernel launch fails.
#[inline]
pub fn gpu_gelu_mul(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: a, b, and out outlive this synchronous launch and n matches their element counts.
	unsafe {
		launch_gelu_mul(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			out.ptr_raw(),
			i64::try_from(n).map_err(|_e| return HipError(1))?,
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if `rows` or `half` overflows `i32` or the launch fails.
#[inline]
pub fn gpu_glu_gelu(
	input: &GpuBuffer,
	rows: usize,
	half: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: input and out outlive this synchronous launch and rows and half match their sizes.
	unsafe {
		launch_glu_gelu(
			input.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(rows)?,
			ci(half)?,
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if `m`, `n`, or `k` overflows `i32` or the launch fails.
#[inline]
pub fn gpu_gemm_bt_f64(
	lhs: &GpuBuffer,
	rhs: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: lhs, rhs, and out outlive this synchronous launch and m, n, k match their shapes.
	unsafe {
		launch_gemm_bt_f64(
			lhs.ptr_raw().cast_const(),
			rhs.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(m)?,
			ci(n)?,
			ci(k)?,
			ptr::null_mut(),
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if the kernel launch fails.
#[inline]
pub fn gpu_scale_f64_inplace(scalar: &GpuBuffer, n: usize, x: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: all pointers reference live GpuBuffers valid for the kernel launch.
	unsafe {
		launch_scale_f64(
			x.ptr_raw(),
			scalar.ptr_raw().cast_const(),
			i64::try_from(n).map_err(|_e| return HipError(1))?,
			ptr::null_mut(),
		);
	}
	return cl();
}
