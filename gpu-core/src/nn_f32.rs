use crate::hip::{HipError, check};
use crate::log::Write;
use crate::memory::GpuBuffer;
use std::ffi::c_void;
use std::process;
use std::ptr;

const HIPBLAS_OP_N: u32 = 111;

unsafe extern "C" {
	fn hipblasSgemm(
		handle: *mut c_void,
		transA: u32,
		transB: u32,
		m: i32,
		n: i32,
		k: i32,
		alpha: *const f32,
		A: *const f32,
		lda: i32,
		B: *const f32,
		ldb: i32,
		beta: *const f32,
		C: *mut f32,
		ldc: i32,
	) -> i32;

	fn launch_relu_f32(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_relu_backward_f32(
		grad: *const c_void,
		act: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_gelu_f32(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_gelu_backward_f32(
		grad: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_bias_add_f32(
		x: *const c_void,
		bias: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_repeat_rows_f32(
		src: *const c_void,
		dst: *mut c_void,
		cols: i32,
		total: i32,
		stream: *mut c_void,
	);
	fn launch_layernorm_f32(
		x: *const c_void,
		out: *mut c_void,
		gamma: *const c_void,
		beta: *const c_void,
		rows: i32,
		cols: i32,
		eps: *const c_void,
		stream: *mut c_void,
	);
	fn launch_layernorm_backward_f32(
		grad_y: *const c_void,
		x: *const c_void,
		gamma: *const c_void,
		grad_x: *mut c_void,
		grad_gamma: *mut c_void,
		grad_beta: *mut c_void,
		rows: i32,
		cols: i32,
		eps: *const c_void,
		stream: *mut c_void,
	);
	fn launch_avg_pool_2d_f32(
		input: *const c_void,
		output: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		kh: i32,
		kw: i32,
		sh: i32,
		sw: i32,
		out_h: i32,
		out_w: i32,
		stream: *mut c_void,
	);
	fn launch_avg_pool_2d_backward_f32(
		grad_out: *const c_void,
		grad_in: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		kh: i32,
		kw: i32,
		sh: i32,
		sw: i32,
		out_h: i32,
		out_w: i32,
		stream: *mut c_void,
	);
	fn launch_max_pool_2d_f32(
		input: *const c_void,
		out_vals: *mut c_void,
		out_idx: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		kh: i32,
		kw: i32,
		sh: i32,
		sw: i32,
		out_h: i32,
		out_w: i32,
		stream: *mut c_void,
	);
	fn launch_max_pool_2d_backward_f32(
		grad_out: *const c_void,
		indices: *const c_void,
		grad_in: *mut c_void,
		n: i32,
		c: i32,
		out_h: i32,
		out_w: i32,
		h: i32,
		w: i32,
		stream: *mut c_void,
	);
	fn launch_lstm_cell_f32(
		gates: *const c_void,
		c: *mut c_void,
		h: *mut c_void,
		n: i32,
		hs: i32,
		stream: *mut c_void,
	);
	fn launch_gru_cell_f32(
		gates: *const c_void,
		h: *const c_void,
		h_new: *mut c_void,
		n: i32,
		hs: i32,
		stream: *mut c_void,
	);

	fn launch_relu_f16(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_gelu_f16(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_add_f16(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_mul_f16(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_sgd_update_f32(
		grad: *const c_void,
		lr: *const c_void,
		weights: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
}

fn check_launch() -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::LAUNCH);
	crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
	let err = unsafe { crate::hip::hipGetLastError() };
	check(err)
}

fn safe_i32(v: usize) -> i32 {
	if !(v <= i32::MAX as usize) {
		drop(Write::err(&format!("size {v} overflows i32")));
		process::abort();
	}
	v as i32
}

pub fn gpu_linear_f32(
	x: &GpuBuffer,
	w: &GpuBuffer,
	bias: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_repeat_rows_f32(
			bias.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			safe_i32(m * n),
			ptr::null_mut(),
		);
	}
	check_launch()?;
	let alpha = 1.0_f32;
	let beta = 1.0_f32;
	let status = unsafe {
		hipblasSgemm(
			crate::kernels::hipblas_handle(),
			HIPBLAS_OP_N,
			HIPBLAS_OP_N,
			safe_i32(n),
			safe_i32(m),
			safe_i32(k),
			&alpha,
			w.ptr_raw() as *const f32,
			safe_i32(n),
			x.ptr_raw() as *const f32,
			safe_i32(k),
			&beta,
			out.ptr_raw() as *mut f32,
			safe_i32(n),
		)
	};
	check(status)
}

pub fn gpu_relu_f32(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_relu_f32(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_relu_backward_f32(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_relu_backward_f32(
			grad.ptr_raw() as *const c_void,
			act.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_gelu_f32(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gelu_f32(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_gelu_backward_f32(
	grad: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_gelu_backward_f32(
			grad.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_layernorm_f32(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	beta: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_layernorm_f32(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			gamma.ptr_raw() as *const c_void,
			beta.ptr_raw() as *const c_void,
			safe_i32(rows),
			safe_i32(cols),
			eps.ptr_raw() as *const c_void,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_layernorm_backward_f32(
	grad_y: &GpuBuffer,
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	grad_x: &GpuBuffer,
	grad_gamma: &GpuBuffer,
	grad_beta: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_layernorm_backward_f32(
			grad_y.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			gamma.ptr_raw() as *const c_void,
			grad_x.ptr_raw(),
			grad_gamma.ptr_raw(),
			grad_beta.ptr_raw(),
			safe_i32(rows),
			safe_i32(cols),
			eps.ptr_raw() as *const c_void,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_bias_add_f32(
	x: &GpuBuffer,
	bias: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bias_add_f32(
			x.ptr_raw() as *const c_void,
			bias.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(rows),
			safe_i32(cols),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_avg_pool_2d_f32(
	input: &GpuBuffer,
	n_batch: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_h = (h - kh) / sh + 1;
	let out_w = (w - kw) / sw + 1;
	unsafe {
		launch_avg_pool_2d_f32(
			input.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n_batch),
			safe_i32(c),
			safe_i32(h),
			safe_i32(w),
			safe_i32(kh),
			safe_i32(kw),
			safe_i32(sh),
			safe_i32(sw),
			safe_i32(out_h),
			safe_i32(out_w),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_avg_pool_2d_backward_f32(
	grad_out: &GpuBuffer,
	n_batch: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	out_h: usize,
	out_w: usize,
	grad_in: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_avg_pool_2d_backward_f32(
			grad_out.ptr_raw() as *const c_void,
			grad_in.ptr_raw(),
			safe_i32(n_batch),
			safe_i32(c),
			safe_i32(h),
			safe_i32(w),
			safe_i32(kh),
			safe_i32(kw),
			safe_i32(sh),
			safe_i32(sw),
			safe_i32(out_h),
			safe_i32(out_w),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_max_pool_2d_f32(
	input: &GpuBuffer,
	n_batch: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	out_vals: &GpuBuffer,
	out_idx: &GpuBuffer,
) -> Result<(), HipError> {
	let out_h = (h - kh) / sh + 1;
	let out_w = (w - kw) / sw + 1;
	unsafe {
		launch_max_pool_2d_f32(
			input.ptr_raw() as *const c_void,
			out_vals.ptr_raw(),
			out_idx.ptr_raw(),
			safe_i32(n_batch),
			safe_i32(c),
			safe_i32(h),
			safe_i32(w),
			safe_i32(kh),
			safe_i32(kw),
			safe_i32(sh),
			safe_i32(sw),
			safe_i32(out_h),
			safe_i32(out_w),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_max_pool_2d_backward_f32(
	grad_out: &GpuBuffer,
	indices: &GpuBuffer,
	n_batch: usize,
	c: usize,
	h: usize,
	w: usize,
	out_h: usize,
	out_w: usize,
	grad_in: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_max_pool_2d_backward_f32(
			grad_out.ptr_raw() as *const c_void,
			indices.ptr_raw() as *const c_void,
			grad_in.ptr_raw(),
			safe_i32(n_batch),
			safe_i32(c),
			safe_i32(out_h),
			safe_i32(out_w),
			safe_i32(h),
			safe_i32(w),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_lstm_cell_f32(
	gates: &GpuBuffer,
	n: usize,
	hs: usize,
	c: &GpuBuffer,
	h: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_lstm_cell_f32(
			gates.ptr_raw() as *const c_void,
			c.ptr_raw(),
			h.ptr_raw(),
			safe_i32(n),
			safe_i32(hs),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_gru_cell_f32(
	gates: &GpuBuffer,
	h: &GpuBuffer,
	n: usize,
	hs: usize,
	h_new: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_gru_cell_f32(
			gates.ptr_raw() as *const c_void,
			h.ptr_raw() as *const c_void,
			h_new.ptr_raw(),
			safe_i32(n),
			safe_i32(hs),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_relu_f16(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_relu_f16(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_gelu_f16(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gelu_f16(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_add_f16(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_add_f16(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_mul_f16(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_mul_f16(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_sgd_update_f32(
	grad: &GpuBuffer,
	lr: &GpuBuffer,
	n: usize,
	weights: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_sgd_update_f32(
			grad.ptr_raw() as *const c_void,
			lr.ptr_raw() as *const c_void,
			weights.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch()
}
