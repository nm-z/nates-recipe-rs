use crate::hip::{HipError, check};
use crate::memory::GpuBuffer;
use std::ffi::c_void;
use std::ptr;

fn check_launch() -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::LAUNCH);
	crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
	let err = unsafe { crate::hip::hipGetLastError() };
	check(err)
}

#[derive(Clone, Copy)]
pub struct ConvOut {
	pub h: usize,
	pub w: usize,
}

pub fn conv_out_hw(
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
	let out_h = (h + 2 * pad_h - dil_h * (kh - 1) - 1) / sh + 1;
	let out_w = (w + 2 * pad_w - dil_w * (kw - 1) - 1) / sw + 1;
	ConvOut { h: out_h, w: out_w }
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

pub fn gpu_scaled_dot_product_attention(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	n_rows: usize,
	seq: usize,
	dim: usize,
	causal: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_scaled_dot_product_attention(
			q.ptr_raw() as *const c_void,
			k.ptr_raw() as *const c_void,
			v.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n_rows as i32,
			seq as i32,
			dim as i32,
			causal as i32,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_causal_softmax_rows(rows: usize, cols: usize, x: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_causal_softmax_rows(x.ptr_raw(), rows as i32, cols as i32, ptr::null_mut());
	}
	check_launch()
}

pub fn gpu_mha_split(
	x: &GpuBuffer,
	seq: usize,
	n_heads: usize,
	head_dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_mha_split(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			seq as i32,
			n_heads as i32,
			head_dim as i32,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_mha_merge(
	x: &GpuBuffer,
	seq: usize,
	n_heads: usize,
	head_dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_mha_merge(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			seq as i32,
			n_heads as i32,
			head_dim as i32,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_rope(
	x: &GpuBuffer,
	seq: usize,
	dim: usize,
	base: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_rope(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			seq as i32,
			dim as i32,
			base.ptr_raw() as *const c_void,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_positional_encoding(seq: usize, dim: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_positional_encoding(out.ptr_raw(), seq as i32, dim as i32, ptr::null_mut());
	}
	check_launch()
}

pub fn gpu_rmsnorm(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_rmsnorm(
			x.ptr_raw() as *const c_void,
			gamma.ptr_raw() as *const c_void,
			out.ptr_raw(),
			rows as i32,
			cols as i32,
			eps.ptr_raw() as *const c_void,
			ptr::null_mut(),
		);
	}
	check_launch()
}

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
	unsafe {
		launch_rmsnorm_backward(
			grad_out.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			gamma.ptr_raw() as *const c_void,
			grad_x.ptr_raw(),
			grad_gamma.ptr_raw(),
			rows as i32,
			cols as i32,
			eps.ptr_raw() as *const c_void,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_im2col_2d_ext(
	x: &GpuBuffer,
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
	unsafe {
		launch_im2col_2d_ext(
			x.ptr_raw() as *const c_void,
			patches.ptr_raw(),
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			kh as i32,
			kw as i32,
			sh as i32,
			sw as i32,
			pad_h as i32,
			pad_w as i32,
			dil_h as i32,
			dil_w as i32,
			dims.h as i32,
			dims.w as i32,
			ptr::null_mut(),
		);
	}
	check_launch()
}

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
	unsafe {
		launch_col2im_2d_ext(
			patches.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			kh as i32,
			kw as i32,
			sh as i32,
			sw as i32,
			pad_h as i32,
			pad_w as i32,
			dil_h as i32,
			dil_w as i32,
			dims.h as i32,
			dims.w as i32,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_embedding_backward(
	grad_out: &GpuBuffer,
	indices: &GpuBuffer,
	n: usize,
	cols: usize,
	vocab: usize,
	grad_table: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_embedding_backward(
			grad_out.ptr_raw() as *const c_void,
			indices.ptr_raw() as *const c_void,
			grad_table.ptr_raw(),
			n as i32,
			cols as i32,
			vocab as i32,
			ptr::null_mut(),
		);
	}
	check_launch()
}

pub fn gpu_bn_update_running(
	save_mean: &GpuBuffer,
	save_var: &GpuBuffer,
	momentum: &GpuBuffer,
	c: usize,
	run_mean: &GpuBuffer,
	run_var: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bn_update_running(
			run_mean.ptr_raw(),
			run_var.ptr_raw(),
			save_mean.ptr_raw() as *const c_void,
			save_var.ptr_raw() as *const c_void,
			momentum.ptr_raw() as *const c_void,
			c as i32,
			ptr::null_mut(),
		);
	}
	check_launch()
}
