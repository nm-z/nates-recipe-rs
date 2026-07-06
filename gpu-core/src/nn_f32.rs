use crate::hip::{HipError, check};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

// ── hipBLAS f32 ────────────────────────────────────────────────────────────
// Same column-major conventions as kernels.rs dgemm section.
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

	// f32 kernels
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

	// f16 kernels — buffers are alloc_bytes(n*2), cast as raw u16 bit patterns
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
	assert!(v <= i32::MAX as usize, "size {} overflows i32", v);
	v as i32
}

// ── gpu_linear_f32 ─────────────────────────────────────────────────────────
// out = X @ W + bias. X is (m,k) f32, W is (k,n) f32, bias is (n,) f32.
// Prefills out with bias broadcast, then sgemm with beta=1.0 adds the matmul.
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
			std::ptr::null_mut(),
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

// ── gpu_relu_f32 / backward ────────────────────────────────────────────────
pub fn gpu_relu_f32(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_relu_f32(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			std::ptr::null_mut(),
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_gelu_f32 / backward ────────────────────────────────────────────────
// backward takes pre-activation x (not the output), matching the f64 convention
pub fn gpu_gelu_f32(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gelu_f32(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			std::ptr::null_mut(),
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_layernorm_f32 / backward ───────────────────────────────────────────
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
			std::ptr::null_mut(),
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_bias_add_f32 ───────────────────────────────────────────────────────
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_avg_pool_2d_f32 / backward ────────────────────────────────────────
// Input is NCHW layout. outH = (H-kH)/sH+1, outW = (W-kW)/sW+1.
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
			std::ptr::null_mut(),
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_max_pool_2d_f32 / backward ────────────────────────────────────────
// Returns (pooled_values, argmax_indices) both as f32 GpuBuffers.
// out_idx stores the flat intra-channel index (ih*W+iw) as f32.
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
			std::ptr::null_mut(),
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_lstm_cell_f32 ─────────────────────────────────────────────────────
// gates: (n, 4*hs) f32, layout [forget|input|cell_cand|output] per sample.
// c and h are updated in-place (n, hs).
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_gru_cell_f32 ──────────────────────────────────────────────────────
// gates: (n, 4*hs) f32, layout [z_pre|r_pre|n_x|n_h] per sample.
// h: previous hidden (n, hs). Returns new hidden (n, hs).
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── f16 kernels ───────────────────────────────────────────────────────────
// Buffers hold raw __half bit patterns. Allocate with alloc_bytes(n * 2).
// Upload via upload_u8 (reinterpret &[u16] as &[u8]) or via half::f16 helpers.

pub fn gpu_relu_f16(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_relu_f16(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			std::ptr::null_mut(),
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_add_f16(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_add_f16(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_mul_f16(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_mul_f16(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch()
}

// ── gpu_sgd_update_f32 ────────────────────────────────────────────────────
// In-place: weights -= lr * grad. lr rides as a 1-elem f32 device buffer.
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
			std::ptr::null_mut(),
		);
	}
	check_launch()
}
