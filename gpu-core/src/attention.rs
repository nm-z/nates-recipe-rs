use crate::HipError;
use crate::callspy::{GET_LAST_ERROR, LAUNCH, tick};
use crate::hip::{check, hipGetLastError};
use crate::kernels::ci;
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

fn check_launch() -> Result<(), HipError> {
	tick(&LAUNCH);
	tick(&GET_LAST_ERROR);
	// SAFETY: nullary HIP error query with no preconditions.
	let err = unsafe { hipGetLastError() };
	return check(err);
}

#[derive(Clone, Copy)]
pub struct ConvOut {
	pub h: usize,
	pub w: usize,
}

#[must_use]
#[inline]
pub const fn conv_out_hw(
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	pad_h: usize,
	pad_w: usize,
	dil_h: usize,
	dil_w: usize,
) -> ConvOut {
	let out_h = (h + 2 * pad_h - dil_h * (kh - 1) - 1).div_euclid(sh) + 1;
	let out_w = (w + 2 * pad_w - dil_w * (kw - 1) - 1).div_euclid(sw) + 1;
	return ConvOut { h: out_h, w: out_w };
}

unsafe extern "C" {
	fn launch_scaled_dot_product_attention(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		n_rows: i32,
		seq: i32,
		dim: i32,
		causal: i32,
		stream: *mut c_void,
	);
	fn launch_causal_softmax_rows(x: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
	fn launch_mha_split(
		x: *const c_void,
		out: *mut c_void,
		seq: i32,
		n_heads: i32,
		head_dim: i32,
		stream: *mut c_void,
	);
	fn launch_mha_merge(
		x: *const c_void,
		out: *mut c_void,
		seq: i32,
		n_heads: i32,
		head_dim: i32,
		stream: *mut c_void,
	);
	fn launch_rope(
		x: *const c_void,
		out: *mut c_void,
		seq: i32,
		dim: i32,
		base: *const c_void,
		stream: *mut c_void,
	);
	fn launch_positional_encoding(out: *mut c_void, seq: i32, dim: i32, stream: *mut c_void);
	fn launch_rmsnorm(
		x: *const c_void,
		gamma: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		eps: *const c_void,
		stream: *mut c_void,
	);
	fn launch_rmsnorm_backward(
		grad_out: *const c_void,
		x: *const c_void,
		gamma: *const c_void,
		grad_x: *mut c_void,
		grad_gamma: *mut c_void,
		rows: i32,
		cols: i32,
		eps: *const c_void,
		stream: *mut c_void,
	);
	fn launch_im2col_2d_ext(
		x: *const c_void,
		patches: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		kh: i32,
		kw: i32,
		sh: i32,
		sw: i32,
		pad_h: i32,
		pad_w: i32,
		dil_h: i32,
		dil_w: i32,
		out_h: i32,
		out_w: i32,
		stream: *mut c_void,
	);
	fn launch_col2im_2d_ext(
		patches: *const c_void,
		x: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		kh: i32,
		kw: i32,
		sh: i32,
		sw: i32,
		pad_h: i32,
		pad_w: i32,
		dil_h: i32,
		dil_w: i32,
		out_h: i32,
		out_w: i32,
		stream: *mut c_void,
	);
	fn launch_embedding_backward(
		grad_out: *const c_void,
		indices: *const c_void,
		grad_table: *mut c_void,
		n: i32,
		cols: i32,
		vocab: i32,
		stream: *mut c_void,
	);
	fn launch_bn_update_running(
		run_mean: *mut c_void,
		run_var: *mut c_void,
		save_mean: *const c_void,
		save_var: *const c_void,
		momentum: *const c_void,
		c: i32,
		stream: *mut c_void,
	);
}

#[inline]
pub fn gpu_scaled_dot_product_attn(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	n_rows: usize,
	seq: usize,
	dim: usize,
	causal: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_rows_i = ci(n_rows)?;
	let seq_i = ci(seq)?;
	let dim_i = ci(dim)?;
	let causal_i = ci(causal)?;
	// SAFETY: FFI launch; pointers are live device buffers and the i32 sizes are range-checked above.
	unsafe {
		launch_scaled_dot_product_attention(
			q.ptr_raw().cast_const(),
			k.ptr_raw().cast_const(),
			v.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_rows_i,
			seq_i,
			dim_i,
			causal_i,
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_causal_softmax_rows(rows: usize, cols: usize, x: &GpuBuffer) -> Result<(), HipError> {
	let rows_i = ci(rows)?;
	let cols_i = ci(cols)?;
	// SAFETY: the buffer pointer is valid for the launch and dims fit i32.
	unsafe {
		launch_causal_softmax_rows(x.ptr_raw(), rows_i, cols_i, ptr::null_mut());
	}
	return check_launch();
}

#[inline]
pub fn gpu_mha_split(
	x: &GpuBuffer,
	seq: usize,
	n_heads: usize,
	head_dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let seq_i = ci(seq)?;
	let n_heads_i = ci(n_heads)?;
	let head_dim_i = ci(head_dim)?;
	// SAFETY: buffer pointers are valid for the launch and dims fit i32.
	unsafe {
		launch_mha_split(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			seq_i,
			n_heads_i,
			head_dim_i,
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_mha_merge(
	x: &GpuBuffer,
	seq: usize,
	n_heads: usize,
	head_dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let seq_i = ci(seq)?;
	let n_heads_i = ci(n_heads)?;
	let head_dim_i = ci(head_dim)?;
	// SAFETY: buffer pointers are valid for the launch and dims fit i32.
	unsafe {
		launch_mha_merge(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			seq_i,
			n_heads_i,
			head_dim_i,
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_rope(
	x: &GpuBuffer,
	seq: usize,
	dim: usize,
	base: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let seq_i = ci(seq)?;
	let dim_i = ci(dim)?;
	// SAFETY: launch args are valid device pointers and checked i32 extents.
	unsafe {
		launch_rope(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			seq_i,
			dim_i,
			base.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_positional_encoding(seq: usize, dim: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let seq_i = ci(seq)?;
	let dim_i = ci(dim)?;
	// SAFETY: launch args are valid device pointers and checked i32 extents.
	unsafe {
		launch_positional_encoding(out.ptr_raw(), seq_i, dim_i, ptr::null_mut());
	}
	return check_launch();
}

#[inline]
pub fn gpu_rmsnorm(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i = ci(rows)?;
	let cols_i = ci(cols)?;
	// SAFETY: launch args are valid device pointers and checked i32 extents.
	unsafe {
		launch_rmsnorm(
			x.ptr_raw().cast_const(),
			gamma.ptr_raw().cast_const(),
			out.ptr_raw(),
			rows_i,
			cols_i,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_rmsnorm_backward(
	grad_out: &GpuBuffer,
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	grad_x: &GpuBuffer,
	grad_gamma: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the GpuBuffer pointers are live device allocations for the launch duration.
	unsafe {
		launch_rmsnorm_backward(
			grad_out.ptr_raw().cast_const(),
			x.ptr_raw().cast_const(),
			gamma.ptr_raw().cast_const(),
			grad_x.ptr_raw(),
			grad_gamma.ptr_raw(),
			ci(rows)?,
			ci(cols)?,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_im2col_2d_ext(
	img: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	pad_h: usize,
	pad_w: usize,
	dil_h: usize,
	dil_w: usize,
	patches: &GpuBuffer,
) -> Result<(), HipError> {
	let dims = conv_out_hw(h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w);
	// SAFETY: the GpuBuffer pointers are live device allocations for the launch duration.
	unsafe {
		launch_im2col_2d_ext(
			img.ptr_raw().cast_const(),
			patches.ptr_raw(),
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(kh)?,
			ci(kw)?,
			ci(sh)?,
			ci(sw)?,
			ci(pad_h)?,
			ci(pad_w)?,
			ci(dil_h)?,
			ci(dil_w)?,
			ci(dims.h)?,
			ci(dims.w)?,
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_col2im_2d_ext(
	patches: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	pad_h: usize,
	pad_w: usize,
	dil_h: usize,
	dil_w: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let dims = conv_out_hw(h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w);
	// SAFETY: buffer pointers are valid for this launch and the extern signature matches launch_col2im_2d_ext.
	unsafe {
		launch_col2im_2d_ext(
			patches.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(kh)?,
			ci(kw)?,
			ci(sh)?,
			ci(sw)?,
			ci(pad_h)?,
			ci(pad_w)?,
			ci(dil_h)?,
			ci(dil_w)?,
			ci(dims.h)?,
			ci(dims.w)?,
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_embedding_backward(
	grad_out: &GpuBuffer,
	indices: &GpuBuffer,
	n: usize,
	cols: usize,
	vocab: usize,
	grad_table: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	let cols_i = ci(cols)?;
	let vocab_i = ci(vocab)?;
	// SAFETY: device pointers are valid allocations that outlive the launch.
	unsafe {
		launch_embedding_backward(
			grad_out.ptr_raw().cast_const(),
			indices.ptr_raw().cast_const(),
			grad_table.ptr_raw(),
			n_i,
			cols_i,
			vocab_i,
			ptr::null_mut(),
		);
	}
	return check_launch();
}

#[inline]
pub fn gpu_bn_update_running(
	save_mean: &GpuBuffer,
	save_var: &GpuBuffer,
	momentum: &GpuBuffer,
	c: usize,
	run_mean: &GpuBuffer,
	run_var: &GpuBuffer,
) -> Result<(), HipError> {
	let c_i = ci(c)?;
	// SAFETY: every pointer is a live device allocation and c is a checked i32.
	unsafe {
		launch_bn_update_running(
			run_mean.ptr_raw(),
			run_var.ptr_raw(),
			save_mean.ptr_raw().cast_const(),
			save_var.ptr_raw().cast_const(),
			momentum.ptr_raw().cast_const(),
			c_i,
			ptr::null_mut(),
		);
	}
	return check_launch();
}
