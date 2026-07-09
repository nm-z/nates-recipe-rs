
use crate::hip::{HipError, check};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

fn cl() -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::LAUNCH);
	crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
	check(unsafe { crate::hip::hipGetLastError() })
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
	fn launch_gelu_mul(a: *const c_void, b: *const c_void, out: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_glu_gelu(input: *const c_void, out: *mut c_void, rows: i32, half: i32, stream: *mut c_void);
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

pub fn gpu_rope_partial(
	theta: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	buf: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			rows as i32,
			head_dim as i32,
			rotary_dim as i32,
			heads_per_tok as i32,
			theta.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	cl()
}

pub fn gpu_widen_bf16(raw: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_widen_bf16_f64(raw.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	cl()
}

pub fn gpu_rmsnorm_f64(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			gamma.ptr_raw() as *const c_void,
			rows as i32,
			cols as i32,
			eps.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	cl()
}

pub fn gpu_rmsnorm_f64_nogamma(
	x: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			std::ptr::null(),
			rows as i32,
			cols as i32,
			eps.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	cl()
}

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
	unsafe {
		launch_gqa_masked_attn(
			q.ptr_raw() as *const c_void,
			k.ptr_raw() as *const c_void,
			v.ptr_raw() as *const c_void,
			out.ptr_raw(),
			t as i32,
			nqh as i32,
			nkv as i32,
			hd as i32,
			prefix as i32,
			std::ptr::null_mut(),
		);
	}
	cl()
}

pub fn gpu_gelu_mul(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gelu_mul(a.ptr_raw() as *const c_void, b.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	cl()
}

pub fn gpu_glu_gelu(input: &GpuBuffer, rows: usize, half: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_glu_gelu(input.ptr_raw() as *const c_void, out.ptr_raw(), rows as i32, half as i32, std::ptr::null_mut());
	}
	cl()
}

pub fn gpu_gemm_bt_f64(a: &GpuBuffer, b: &GpuBuffer, m: usize, n: usize, k: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gemm_bt_f64(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			m as i32,
			n as i32,
			k as i32,
			std::ptr::null_mut(),
		);
	}
	cl()
}

pub fn gpu_scale_f64_inplace(scalar: &GpuBuffer, n: usize, x: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_scale_f64(x.ptr_raw(), scalar.ptr_raw() as *const c_void, n as i64, std::ptr::null_mut());
	}
	cl()
}
