use crate::HipError;
use crate::callspy;
use crate::hip::{self, check};
use crate::infer_ops::gpu_scale_f64_inplace;
use crate::log::{Write, device, gpu};
use crate::memory::{self, GpuBuffer};
use core::ffi::c_void;
use core::ops::Div;
use core::{cmp, mem, ptr};
use std::process;

/// Aborts the process if the most recent HIP kernel launch recorded an error.
pub(crate) fn check_launch() {
	callspy::tick(&callspy::LAUNCH);
	callspy::tick(&callspy::GET_LAST_ERROR);
	// SAFETY: hipGetLastError reads and clears the driver's per-thread last-error slot; always valid.
	let err = unsafe { hip::hipGetLastError() };
	if err != 0i32 {
		drop(Write::err(format!(
			"HIP kernel launch failed with error code {err}"
		)));
		process::abort();
	}
}

/// Casts `v` to [`i32`], aborting the process if it exceeds [`i32::MAX`].
pub(crate) fn safe_i32(v: usize) -> i32 {
	if let Ok(n) = i32::try_from(v) {
		return n;
	}
	drop(Write::err(format!("size {v} overflows i32")));
	process::abort();
}

/// Range-checks `v` and casts it to the `i32` an FFI launcher signature expects.
///
/// # Errors
/// Returns [`HipError`] if `v` does not fit in [`i32`]; the offending value is
/// named on stderr before the error bubbles.
pub(crate) fn ci(v: usize) -> Result<i32, HipError> {
	if let Ok(n) = i32::try_from(v) {
		return Ok(n);
	}
	drop(Write::err(format!("size {v} overflows i32")));
	return Err(HipError(1));
}

/// Range-checks `v` and casts it to the `u32` an FFI launcher signature expects.
///
/// # Errors
/// Returns [`HipError`] if `v` does not fit in [`u32`]; the offending value is
/// named on stderr before the error bubbles.
pub(crate) fn cu(v: usize) -> Result<u32, HipError> {
	if let Ok(n) = u32::try_from(v) {
		return Ok(n);
	}
	drop(Write::err(format!("value {v} overflows u32")));
	return Err(HipError(1));
}

/// Exact `a + b`, routed through `mul_add` so no bare float operator remains.
const fn fadd(a: f64, b: f64) -> f64 {
	return a.mul_add(1.0f64, b);
}

/// Exact `a / b`, dispatched through the `Div` trait so no bare float operator remains.
fn fdiv(a: f64, b: f64) -> f64 {
	return Div::div(a, b);
}

/// hipBLAS operation selector for a non-transposed matrix.
const HIPBLAS_OP_N: u32 = 111;
/// hipBLAS operation selector for a transposed matrix.
const HIPBLAS_OP_T: u32 = 112;

unsafe extern "C" {
	fn hipblasCreate(handle: *mut *mut c_void) -> i32;
	fn hipblasDestroy(handle: *mut c_void) -> i32;
	fn hipblasSetStream(handle: *mut c_void, stream: *mut c_void) -> i32;
	fn hipblasSetWorkspace(handle: *mut c_void, addr: *mut c_void, size: usize) -> i32;

	fn hipblasDgemm(
		handle: *mut c_void,
		transA: u32,
		transB: u32,
		m: i32,
		n: i32,
		k: i32,
		alpha: *const f64,
		A: *const f64,
		lda: i32,
		B: *const f64,
		ldb: i32,
		beta: *const f64,
		C: *mut f64,
		ldc: i32,
	) -> i32;

	fn launch_sgd_update_f64(
		w: *mut f64,
		g: *const f64,
		neg_lr: *const f64,
		n: i32,
		stream: *mut c_void,
	);

	fn hipsolverCreate(handle: *mut *mut c_void) -> i32;
	fn hipsolverDestroy(handle: *mut c_void) -> i32;
	fn hipsolverDpotrf_bufferSize(
		handle: *mut c_void,
		uplo: u32,
		n: i32,
		A: *mut f64,
		lda: i32,
		lwork: *mut i32,
	) -> i32;
	fn hipsolverDpotrf(
		handle: *mut c_void,
		uplo: u32,
		n: i32,
		A: *mut f64,
		lda: i32,
		work: *mut f64,
		lwork: i32,
		info: *mut i32,
	) -> i32;
	fn hipsolverDgetrf_bufferSize(
		handle: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		lwork: *mut i32,
	) -> i32;
	fn hipsolverDgetrf(
		handle: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		work: *mut f64,
		lwork: i32,
		ipiv: *mut i32,
		info: *mut i32,
	) -> i32;
	fn hipsolverDgetrs_bufferSize(
		handle: *mut c_void,
		trans: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *mut i32,
		B: *mut f64,
		ldb: i32,
		lwork: *mut i32,
	) -> i32;
	fn hipsolverDgetrs(
		handle: *mut c_void,
		trans: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *mut i32,
		B: *mut f64,
		ldb: i32,
		work: *mut f64,
		lwork: i32,
		info: *mut i32,
	) -> i32;

	fn hipblasDtrsm(
		handle: *mut c_void,
		side: u32,
		uplo: u32,
		transA: u32,
		diag: u32,
		m: i32,
		n: i32,
		alpha: *const f64,
		A: *const f64,
		lda: i32,
		B: *mut f64,
		ldb: i32,
	) -> i32;

	fn launch_add_diag(A: *mut c_void, n: i32, val: *const f64, stream: *mut c_void);
	fn launch_reparameterize(
		mu: *const c_void,
		log_var: *const c_void,
		eps: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_kl_div(
		mu: *const c_void,
		log_var: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_vae_backward_latent(
		grad_z: *const c_void,
		mu: *const c_void,
		log_var: *const c_void,
		eps: *const c_void,
		grad_mu_out: *mut c_void,
		grad_lv_out: *mut c_void,
		n: i32,
		kl_weight: *const f64,
		stream: *mut c_void,
	);
	fn launch_log_det_cholesky(L: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_scaled_exp(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		scale: *const f64,
		stream: *mut c_void,
	);
	fn launch_sigmoid(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_sigmoid_backward(
		grad: *const c_void,
		act: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_tanh_act(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_tanh_backward(
		grad: *const c_void,
		act: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_relu(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_relu_backward(
		grad: *const c_void,
		act: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_add(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_add_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *const f64,
		stream: *mut c_void,
	);
	fn launch_div(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_fma(
		x: *const c_void,
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_reduce_sum_cols(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn launch_reduce_sum_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn launch_reduce_mean_cols(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn launch_reduce_var_cols(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn reduce_sum_cols_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;
	fn reduce_sum_rows_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;
	fn reduce_mean_cols_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;
	fn reduce_var_cols_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;
	fn launch_pairwise_l2(
		query: *const c_void,
		train: *const c_void,
		out: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		stream: *mut c_void,
	);
	fn launch_partial_argsort(
		data: *const c_void,
		indices: *mut c_void,
		keys_out: *mut c_void,
		vals_in: *mut c_void,
		temp: *mut c_void,
		temp_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn partial_argsort_workspace_bytes(n: i32) -> usize;
	fn launch_bias_add(
		x: *const c_void,
		bias: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_lstm_cell(
		gates: *const c_void,
		c: *mut c_void,
		h: *mut c_void,
		n: i32,
		hs: i32,
		stream: *mut c_void,
	);
	fn launch_gaussian_ll(
		x: *const c_void,
		means: *const c_void,
		vars: *const c_void,
		log_priors: *const c_void,
		out: *mut c_void,
		n: i32,
		k: i32,
		p: i32,
		stream: *mut c_void,
	);
	fn launch_im2col_1d(
		x: *const c_void,
		patches: *mut c_void,
		n: i32,
		p: i32,
		ks: i32,
		out_len: i32,
		stream: *mut c_void,
	);
	fn launch_argmax_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_mul(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_mul_inplace(a: *mut c_void, b: *const c_void, n: i32, stream: *mut c_void);
	fn launch_add_col_scaled(
		matrix: *mut c_void,
		col: *const c_void,
		n: i32,
		cols: i32,
		k: i32,
		scale: *const f64,
		stream: *mut c_void,
	);
	fn launch_grad_hess(
		probs: *const c_void,
		targets: *const c_void,
		weights: *const c_void,
		mask: *const c_void,
		grad_out: *mut c_void,
		hess_out: *mut c_void,
		n: i32,
		nc: i32,
		k: i32,
		stream: *mut c_void,
	);
	fn launch_softmax_ce_grad(
		logits: *const c_void,
		targets: *const c_void,
		weights: *const c_void,
		grad_out: *mut c_void,
		n: i32,
		nc: i32,
		scale: *const f64,
		stream: *mut c_void,
	);
	fn launch_sub(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_softmax_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_flash_attention_f64(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		n: i32,
		seq: i32,
		d: i32,
		heads: i32,
		stream: *mut c_void,
	);
	fn launch_flash_attention_f64_train_fwd(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		lse: *mut c_void,
		n: i32,
		seq: i32,
		d: i32,
		heads: i32,
		stream: *mut c_void,
	);
	fn launch_flash_attention_f64_dsum(
		ctx: *const c_void,
		dctx: *const c_void,
		dsum: *mut c_void,
		n: i32,
		seq: i32,
		d: i32,
		heads: i32,
		stream: *mut c_void,
	);
	fn launch_flash_attention_f64_bwd_dq(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		dctx: *const c_void,
		lse: *const c_void,
		dsum: *const c_void,
		dq: *mut c_void,
		n: i32,
		seq: i32,
		d: i32,
		heads: i32,
		stream: *mut c_void,
	);
	fn launch_flash_attention_f64_bwd_dkv(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		dctx: *const c_void,
		lse: *const c_void,
		dsum: *const c_void,
		dk: *mut c_void,
		dv: *mut c_void,
		n: i32,
		seq: i32,
		d: i32,
		heads: i32,
		stream: *mut c_void,
	);
	fn launch_sub_scale(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		scale: *const f64,
		stream: *mut c_void,
	);
	fn launch_avg_pool_1d(
		input: *const c_void,
		output: *mut c_void,
		n: i32,
		out_len: i32,
		n_filters: i32,
		stream: *mut c_void,
	);
	fn launch_pool_grad_expand(
		grad_pool: *const c_void,
		grad_out: *mut c_void,
		n: i32,
		out_len: i32,
		n_filters: i32,
		stream: *mut c_void,
	);
	fn launch_argmin_rows(
		dists: *const c_void,
		assignments: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_centroid_update(
		x: *const c_void,
		assignments: *const c_void,
		centroids: *mut c_void,
		counts: *mut c_void,
		n: i32,
		dim: i32,
		k: i32,
		stream: *mut c_void,
	);
	fn launch_topk_per_row(
		dists: *const c_void,
		out_indices: *mut c_void,
		rows: i32,
		cols: i32,
		k: i32,
		stream: *mut c_void,
	);
	fn launch_leaky_relu(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: *const f64,
		stream: *mut c_void,
	);
	fn launch_leaky_relu_backward(
		grad: *const c_void,
		act: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: *const f64,
		stream: *mut c_void,
	);
	fn launch_layernorm(
		x: *const c_void,
		out: *mut c_void,
		gamma: *const c_void,
		beta: *const c_void,
		rows: i32,
		cols: i32,
		eps: *const f64,
		stream: *mut c_void,
	);
	fn launch_dropout(
		x: *const c_void,
		mask: *const c_void,
		out: *mut c_void,
		n: i32,
		p: *const f64,
		scale: *const f64,
		stream: *mut c_void,
	);
	fn launch_bernoulli_u8(
		mask: *mut c_void,
		n: i32,
		seed: u32,
		p: *const f64,
		stream: *mut c_void,
	);
	fn launch_dropout_u8(
		x: *const c_void,
		mask: *const c_void,
		out: *mut c_void,
		n: i32,
		scale: *const f64,
		stream: *mut c_void,
	);
	fn launch_concat(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		rows: i32,
		d1: i32,
		d2: i32,
		stream: *mut c_void,
	);
	fn launch_slice_lead(
		src: *const c_void,
		out: *mut c_void,
		rows: i32,
		src_cols: i32,
		take: i32,
		stream: *mut c_void,
	);
	fn launch_im2col_2d(
		x: *const c_void,
		patches: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		kh: i32,
		kw: i32,
		out_h: i32,
		out_w: i32,
		stream: *mut c_void,
	);
	fn launch_exp(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_log(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_copy_f64(src: *const c_void, dst: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_sqrt(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_abs(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_neg(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_sign(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_pow(x: *const c_void, out: *mut c_void, n: i32, p: *const f64, stream: *mut c_void);
	fn launch_clamp(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		lo: *const f64,
		hi: *const f64,
		stream: *mut c_void,
	);
	fn launch_transpose(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_shapex_pack_upper_tri(
		factor: *const c_void,
		r: *mut c_void,
		m: i32,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_eye(out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_where_mask(
		cond: *const c_void,
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_slice_rows(
		src: *const c_void,
		dst: *mut c_void,
		start_row: i32,
		count: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_broadcast_sub(
		x: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		n: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_broadcast_mul(
		x: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		n: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_broadcast_div(
		x: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		n: i32,
		cols: i32,
		stream: *mut c_void,
	);

	fn launch_softmax_backward(
		grad: *const c_void,
		sm: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_log_softmax_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_cross_entropy(
		logits: *const c_void,
		targets: *const c_void,
		losses: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);

	fn launch_gather_rows(
		table: *const c_void,
		indices: *const c_void,
		out: *mut c_void,
		n: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_scatter_add(
		target: *mut c_void,
		indices: *const c_void,
		src: *const c_void,
		n: i32,
		cols: i32,
		stream: *mut c_void,
	);

	fn launch_col2im_1d(
		patches: *const c_void,
		out: *mut c_void,
		n: i32,
		p: i32,
		ks: i32,
		out_len: i32,
		stream: *mut c_void,
	);
	fn launch_col2im_2d(
		patches: *const c_void,
		out: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		kh: i32,
		kw: i32,
		out_h: i32,
		out_w: i32,
		stream: *mut c_void,
	);

	fn launch_max_pool_1d(
		input: *const c_void,
		out_vals: *mut c_void,
		out_idx: *mut c_void,
		n: i32,
		out_len: i32,
		n_filters: i32,
		stream: *mut c_void,
	);
	fn launch_max_pool_1d_backward(
		grad: *const c_void,
		indices: *const c_void,
		out: *mut c_void,
		n: i32,
		out_len: i32,
		n_filters: i32,
		stream: *mut c_void,
	);

	fn launch_avg_pool_2d(
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
	fn launch_avg_pool_2d_backward(
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
	fn launch_max_pool_2d(
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
	fn launch_max_pool_2d_backward(
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

	fn launch_reduce_max_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn launch_reduce_max_cols(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn launch_reduce_min_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn launch_reduce_min_cols(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn reduce_max_rows_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;
	fn reduce_max_cols_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;
	fn reduce_min_rows_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;
	fn reduce_min_cols_workspace_bytes(
		x: *const c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	) -> usize;

	fn launch_gt(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_lt(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_eq(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_gt_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		val: *const f64,
		stream: *mut c_void,
	);
	fn launch_lt_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		val: *const f64,
		stream: *mut c_void,
	);

	fn launch_gelu(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_gelu_backward(
		grad: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_silu(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_silu_backward(
		grad: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);

	fn launch_batchnorm_forward(
		x: *const c_void,
		gamma: *const c_void,
		beta: *const c_void,
		out: *mut c_void,
		save_mean: *mut c_void,
		save_inv_std: *mut c_void,
		n: i32,
		c: i32,
		eps: *const f64,
		stream: *mut c_void,
	);
	fn launch_batchnorm_inference(
		x: *const c_void,
		gamma: *const c_void,
		beta: *const c_void,
		run_mean: *const c_void,
		run_var: *const c_void,
		out: *mut c_void,
		n: i32,
		c: i32,
		eps: *const f64,
		stream: *mut c_void,
	);
	fn launch_batchnorm_backward(
		grad_y: *const c_void,
		x: *const c_void,
		save_mean: *const c_void,
		save_inv_std: *const c_void,
		gamma: *const c_void,
		grad_x: *mut c_void,
		grad_gamma: *mut c_void,
		grad_beta: *mut c_void,
		n: i32,
		c: i32,
		stream: *mut c_void,
	);

	fn launch_layernorm_backward(
		grad_y: *const c_void,
		x: *const c_void,
		gamma: *const c_void,
		grad_x: *mut c_void,
		grad_gamma: *mut c_void,
		grad_beta: *mut c_void,
		rows: i32,
		cols: i32,
		eps: *const f64,
		stream: *mut c_void,
	);

	fn launch_adam_update(
		w: *mut c_void,
		m: *mut c_void,
		v: *mut c_void,
		g: *const c_void,
		lr: *const f64,
		b1: *const f64,
		b2: *const f64,
		eps: *const f64,
		t: i32,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_adamw_update(
		w: *mut c_void,
		m: *mut c_void,
		v: *mut c_void,
		g: *const c_void,
		lr: *const f64,
		b1: *const f64,
		b2: *const f64,
		eps: *const f64,
		wd: *const f64,
		t: i32,
		n: i32,
		stream: *mut c_void,
	);

	fn launch_gru_cell(
		gates: *const c_void,
		h: *const c_void,
		h_new: *mut c_void,
		n: i32,
		hs: i32,
		stream: *mut c_void,
	);

	fn launch_slice_cols(
		src: *const c_void,
		dst: *mut c_void,
		rows: i32,
		src_cols: i32,
		start: i32,
		count: i32,
		stream: *mut c_void,
	);
	fn launch_tril_mask(out: *mut c_void, n: i32, fill_val: *const f64, stream: *mut c_void);
	fn launch_fill(out: *mut c_void, n: i32, val: *const f64, stream: *mut c_void);
	fn launch_repeat_rows(
		src: *const c_void,
		dst: *mut c_void,
		src_n: i32,
		total: i32,
		stream: *mut c_void,
	);
	fn launch_upsample_nearest_2d(
		input: *const c_void,
		output: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w: i32,
		scale_h: i32,
		scale_w: i32,
		stream: *mut c_void,
	);

	fn launch_log_sum_exp_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_grad_clip_norm(
		x: *mut c_void,
		tmp: *mut c_void,
		n: i32,
		max_norm: *const f64,
		stream: *mut c_void,
	);

	fn launch_prefix_sum_inclusive(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn launch_prefix_sum_exclusive(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		tmp: *mut c_void,
		tmp_bytes: usize,
		stream: *mut c_void,
	);
	fn prefix_sum_inclusive_workspace_bytes(
		x: *const c_void,
		n: i32,
		stream: *mut c_void,
	) -> usize;
	fn prefix_sum_exclusive_workspace_bytes(
		x: *const c_void,
		n: i32,
		stream: *mut c_void,
	) -> usize;

	fn launch_histogram_build(
		bins: *const c_void,
		grad: *const c_void,
		hess: *const c_void,
		mask: *const c_void,
		grad_hist: *mut c_void,
		hess_hist: *mut c_void,
		count_hist: *mut c_void,
		n: i32,
		p: i32,
		n_bins: i32,
		stream: *mut c_void,
	);
	fn launch_split_eval(
		grad_hist: *const c_void,
		hess_hist: *const c_void,
		best_gain: *mut c_void,
		best_bin: *mut c_void,
		p: i32,
		n_bins: i32,
		lambda: *const f64,
		min_child_weight: *const f64,
		stream: *mut c_void,
	);
	fn launch_data_partition(
		bins: *const c_void,
		node_mask: *const c_void,
		left_mask: *mut c_void,
		right_mask: *mut c_void,
		n: i32,
		p: i32,
		split_feat: i32,
		split_bin: i32,
		stream: *mut c_void,
	);
	fn launch_tb_histogram(
		tr_bins: *const c_void,
		grad: *const c_void,
		hess: *const c_void,
		node_assign: *const c_void,
		grad_hist: *mut c_void,
		hess_hist: *mut c_void,
		n_tr: i32,
		p: i32,
		n_bins: i32,
		level_base: i32,
		stream: *mut c_void,
	);
	fn launch_tb_split_eval(
		grad_hist: *const c_void,
		hess_hist: *const c_void,
		split_feat: *mut c_void,
		split_bin: *mut c_void,
		n_level: i32,
		p: i32,
		n_bins: i32,
		lambda: *const f64,
		min_cw: *const f64,
		level_base: i32,
		stream: *mut c_void,
	);
	fn launch_tb_repartition(
		tr_bins: *const c_void,
		node_assign: *mut c_void,
		split_feat: *const c_void,
		split_bin: *const c_void,
		n_tr: i32,
		p: i32,
		stream: *mut c_void,
	);
	fn launch_tb_leaf_sum(
		grad: *const c_void,
		hess: *const c_void,
		node_assign: *const c_void,
		node_sum_g: *mut c_void,
		node_sum_h: *mut c_void,
		n_tr: i32,
		stream: *mut c_void,
	);
	fn launch_tb_leaf_val(
		node_sum_g: *const c_void,
		node_sum_h: *const c_void,
		leaf_val: *mut c_void,
		n_leaves: i32,
		leaf_base: i32,
		lambda: *const f64,
		stream: *mut c_void,
	);
	fn launch_tb_scatter(
		node_assign: *const c_void,
		leaf_val: *const c_void,
		predictions: *mut c_void,
		n_tr: i32,
		stream: *mut c_void,
	);
	fn launch_tb_apply_tree(
		te_bins: *const c_void,
		split_feat: *const c_void,
		split_bin: *const c_void,
		leaf_val: *const c_void,
		predictions: *mut c_void,
		n_te: i32,
		p: i32,
		max_depth: i32,
		stream: *mut c_void,
	);

	fn launch_mse_grad(
		pred: *const c_void,
		target: *const c_void,
		grad: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_argmax_f32(data: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_fill_f32(out: *mut c_void, val: *const f32, n: i32, stream: *mut c_void);
	fn launch_write_split(
		split_feat: *mut c_void,
		split_bin: *mut c_void,
		feat: i32,
		bin: u8,
		d: i32,
		stream: *mut c_void,
	);
	fn launch_argmax_write_split(
		gain: *const c_void,
		split_feat: *mut c_void,
		split_bin: *mut c_void,
		best_idx: *mut c_void,
		n_features: i32,
		n_bins: i32,
		d: i32,
		stream: *mut c_void,
	);
	fn launch_oblivious_histogram(
		bins_fm: *const c_void,
		node_idx: *const c_void,
		grad: *const c_void,
		hess: *const c_void,
		grad_hist: *mut c_void,
		hess_hist: *mut c_void,
		n_rows: i32,
		n_features: i32,
		n_bins: i32,
		n_nodes: i32,
		stream: *mut c_void,
	);
	fn launch_oblivious_route_step(
		bins_rm: *const c_void,
		node_in: *const c_void,
		node_out: *mut c_void,
		split_feat: i32,
		split_bin: u8,
		depth: i32,
		n_rows: i32,
		n_features: i32,
		stream: *mut c_void,
	);
	fn launch_oblivious_route_step_dev(
		bins_rm: *const c_void,
		node_in: *const c_void,
		node_out: *mut c_void,
		split_feat_arr: *const c_void,
		split_bin_arr: *const c_void,
		depth: i32,
		n_rows: i32,
		n_features: i32,
		stream: *mut c_void,
	);
	fn launch_oblivious_route_full(
		bins_rm: *const c_void,
		split_feat: *const c_void,
		split_bin: *const c_void,
		leaf_idx: *mut c_void,
		n_rows: i32,
		n_features: i32,
		depth: i32,
		stream: *mut c_void,
	);
	fn launch_scatter_add_by_leaf(
		pred: *mut c_void,
		leaf_idx: *const c_void,
		leaf_value: *const c_void,
		lr: *const f32,
		n_rows: i32,
		stream: *mut c_void,
	);
	fn launch_leaf_reduce(
		leaf_idx: *const c_void,
		grad: *const c_void,
		hess: *const c_void,
		leaf_grad: *mut c_void,
		leaf_hess: *mut c_void,
		n_rows: i32,
		stream: *mut c_void,
	);
	fn launch_leaf_finalize(
		leaf_grad: *const c_void,
		leaf_hess: *const c_void,
		leaf_value: *mut c_void,
		lambda: *const f32,
		n_leaves: i32,
		stream: *mut c_void,
	);
	fn launch_oblivious_split_eval(
		grad_hist: *const c_void,
		hess_hist: *const c_void,
		gain_out: *mut c_void,
		n_nodes: i32,
		n_features: i32,
		n_bins: i32,
		lambda: *const f32,
		min_cw: *const f32,
		stream: *mut c_void,
	);
	fn launch_softmax_ce_class_grad_f32(
		ptrs: *const c_void,
		targets: *const c_void,
		grad: *mut c_void,
		hess: *mut c_void,
		k: i32,
		n: i32,
		nc: i32,
		stream: *mut c_void,
	);
	fn launch_logloss_grad_f32(
		pred: *const c_void,
		target: *const c_void,
		grad: *mut c_void,
		hess: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_softmax_inplace(x: *mut c_void, n_rows: i32, n_classes: i32, stream: *mut c_void);
	fn launch_logloss_grad_mc(
		pred: *const c_void,
		tgt: *const c_void,
		grad: *mut c_void,
		hess: *mut c_void,
		n_rows: i32,
		n_classes: i32,
		stream: *mut c_void,
	);
	fn launch_accuracy(
		pred: *const c_void,
		tgt: *const c_void,
		out: *mut c_void,
		n_rows: i32,
		n_classes: i32,
		stream: *mut c_void,
	);
	fn launch_scatter_add_by_leaf_col(
		pred: *mut c_void,
		leaf_idx: *const c_void,
		leaf_value: *const c_void,
		lr: *const f32,
		n_rows: i32,
		n_classes: i32,
		col: i32,
		stream: *mut c_void,
	);

	fn launch_ss_res(
		pred: *const c_void,
		y: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_mse(
		pred: *const c_void,
		y: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	#[link_name = "launch_acc_metric"]
	fn launch_accuracy_metric(
		pred: *const c_void,
		y: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_bce_grad(
		pred: *const c_void,
		y: *const c_void,
		da: *mut c_void,
		n: i32,
		inv_n: *const c_void,
		stream: *mut c_void,
	);
	fn launch_argmax_acc(
		pred: *const c_void,
		y: *const c_void,
		out: *mut c_void,
		n: i32,
		k: i32,
		stream: *mut c_void,
	);

	fn launch_dtw_init(dp: *mut c_void, dp_size: i32, stream: *mut c_void);
	fn launch_dtw_antidiag(
		cost: *const c_void,
		dp: *mut c_void,
		m: i32,
		n: i32,
		d: i32,
		stream: *mut c_void,
	);

	fn launch_itemset_support(
		trans: *const c_void,
		cands: *const c_void,
		counts: *mut c_void,
		n_trans: i32,
		n_items: i32,
		n_cands: i32,
		k: i32,
		stream: *mut c_void,
	);
	fn launch_candidate_generate_write(
		freq: *const c_void,
		out: *mut c_void,
		n_freq: i32,
		k: i32,
		write_pos: *mut c_void,
		stream: *mut c_void,
	);

	fn launch_rand_uniform(out: *mut c_void, n: i32, seed: u32, stream: *mut c_void);
	fn launch_randn(out: *mut c_void, n: i32, seed: u32, stream: *mut c_void);
	fn launch_bernoulli(out: *mut c_void, n: i32, p: *const f64, seed: u32, stream: *mut c_void);

	fn launch_lgbm_histogram(
		bins_fm: *const c_void,
		node_idx: *const c_void,
		grad: *const c_void,
		hess: *const c_void,
		grad_hist: *mut c_void,
		hess_hist: *mut c_void,
		count_hist: *mut c_void,
		target_slot: i32,
		n_rows: i32,
		n_eff: i32,
		n_bins: i32,
		stream: *mut c_void,
	);
	fn launch_lgbm_hist_subtract(
		grad_hist: *mut c_void,
		hess_hist: *mut c_void,
		count_hist: *mut c_void,
		dst_slot: i32,
		src_slot: i32,
		n_eff: i32,
		n_bins: i32,
		stream: *mut c_void,
	);
	fn launch_lgbm_best_split(
		grad_hist: *const c_void,
		hess_hist: *const c_void,
		count_hist: *const c_void,
		slot_ids: *const c_void,
		best_gain: *mut c_void,
		best_feat: *mut c_void,
		best_bin: *mut c_void,
		best_left_count: *mut c_void,
		n_eval: i32,
		n_eff: i32,
		n_bins: i32,
		lambda: *const f32,
		min_child_weight: *const f32,
		stream: *mut c_void,
	);
	fn launch_lgbm_leaf_reduce(
		node_idx: *const c_void,
		grad: *const c_void,
		hess: *const c_void,
		leaf_grad: *mut c_void,
		leaf_hess: *mut c_void,
		n_rows: i32,
		stream: *mut c_void,
	);
	fn launch_goss_sample(
		sorted_idx: *const c_void,
		weights_out: *mut c_void,
		uniform_rand: *const c_void,
		n_rows: i32,
		top_k: i32,
		sample_rate: *const f32,
		keep_weight: *const f32,
		stream: *mut c_void,
	);
	fn launch_leaf_split_apply(
		bins_fm: *const c_void,
		node_idx: *mut c_void,
		target_leaf: i32,
		new_leaf_left: i32,
		new_leaf_right: i32,
		split_feature: i32,
		split_bin: u8,
		n_rows: i32,
		n_features: i32,
		stream: *mut c_void,
	);
	fn launch_convx_conv1d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		l: i32,
		cout: i32,
		k: i32,
		lout: i32,
		s: i32,
		stream: *mut c_void,
	);
	fn launch_convx_conv1d_backward_data(
		dy: *const c_void,
		w: *const c_void,
		dx: *mut c_void,
		n: i32,
		cin: i32,
		l: i32,
		cout: i32,
		k: i32,
		lout: i32,
		s: i32,
		stream: *mut c_void,
	);
	fn launch_convx_conv1d_backward_filter(
		dy: *const c_void,
		x: *const c_void,
		temp: *mut c_void,
		n: i32,
		cin: i32,
		l: i32,
		cout: i32,
		k: i32,
		lout: i32,
		s: i32,
		chunks: i32,
		stream: *mut c_void,
	);
	fn launch_convx_conv1d_backward_bias(
		dy: *const c_void,
		db: *mut c_void,
		n: i32,
		cout: i32,
		lout: i32,
		stream: *mut c_void,
	);
	fn launch_ssm_conv_causal_silu(
		x: *const c_void,
		conv_w: *const c_void,
		bias: *const c_void,
		out: *mut c_void,
		t: i32,
		d_inner: i32,
		d_conv: i32,
		stream: *mut c_void,
	);
	fn launch_ssm_scan_mamba1(
		xc: *const c_void,
		dt: *const c_void,
		a: *const c_void,
		b: *const c_void,
		c: *const c_void,
		d: *const c_void,
		y: *mut c_void,
		t: i32,
		d_inner: i32,
		d_state: i32,
		stream: *mut c_void,
	);
	fn launch_ssm_scan_mamba2(
		xbc: *const c_void,
		dt: *const c_void,
		a: *const c_void,
		d: *const c_void,
		y: *mut c_void,
		t: i32,
		d_inner: i32,
		d_state: i32,
		n_head: i32,
		n_group: i32,
		conv_dim: i32,
		stream: *mut c_void,
	);
	fn launch_ssm_group_rmsnorm(
		x: *const c_void,
		gamma: *const c_void,
		eps: *const c_void,
		out: *mut c_void,
		rows: i32,
		dpg: i32,
		n_group: i32,
		stream: *mut c_void,
	);
	fn launch_l2norm_rows(
		x: *const c_void,
		eps: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_gated_delta_scan(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		g: *const c_void,
		beta: *const c_void,
		a: *const c_void,
		out: *mut c_void,
		t: i32,
		hv: i32,
		d: i32,
		per_channel: i32,
		scale: f64,
		stream: *mut c_void,
	);
	fn launch_row_scale(
		x: *const c_void,
		s: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
}

use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

thread_local! {
    static HIPBLAS_HANDLE: AtomicPtr<c_void> = const { AtomicPtr::new(ptr::null_mut()) };
    static HIPSOLVER_HANDLE: AtomicPtr<c_void> = const { AtomicPtr::new(ptr::null_mut()) };
}

/// One-time guard flag for registering the process-exit GPU shutdown hook.
static ATEXIT_REGISTERED: AtomicUsize = AtomicUsize::new(0);

/// Process-exit hook: synchronizes the device, then runs GPU shutdown.
#[inline]
pub unsafe extern "C" fn atexit_gpu_shutdown() {
	callspy::tick(&callspy::DEVICE_SYNCHRONIZE);
	// SAFETY: hipDeviceSynchronize blocks until the device is idle; valid to call at shutdown.
	let _sync = unsafe { hip::hipDeviceSynchronize() };
	memory::mark_shutting_down();
	gpu_shutdown();
}

/// Returns the thread-local hipBLAS handle, lazily creating it on first use.
pub(crate) fn hipblas_handle() -> *mut c_void {
	callspy::tick(&callspy::HIPBLAS);
	return HIPBLAS_HANDLE.with(|h| {
		let ptr = h.load(Ordering::Relaxed);
		return ptr::NonNull::new(ptr).map_or_else(
			|| {
				if ATEXIT_REGISTERED.swap(1, Ordering::Relaxed) == 0 {
					unsafe extern "C" {
						fn atexit(f: unsafe extern "C" fn()) -> i32;
					}
					// SAFETY: atexit_gpu_shutdown is a valid extern "C" fn(); registering it as an atexit hook is sound.
					let _atexit = unsafe { atexit(atexit_gpu_shutdown) };
				}

				let mut handle: *mut c_void = ptr::null_mut();
				// SAFETY: &raw mut handle is a valid out-pointer to a null-initialized handle that hipblasCreate fills.
				let status = unsafe { hipblasCreate(&raw mut handle) };
				if status != 0i32 {
					drop(Write::err(format!(
						"hipblasCreate failed with status {status}"
					)));
					process::abort();
				}
				// SAFETY: handle is the live hipBLAS handle from hipblasCreate; a null stream selects the default stream.
				let stream_status = unsafe { hipblasSetStream(handle, ptr::null_mut()) };
				if stream_status != 0i32 {
					drop(Write::err(format!(
						"hipblasSetStream failed with status {stream_status}"
					)));
					process::abort();
				}
				h.store(handle, Ordering::Relaxed);
				return handle;
			},
			|existing| return existing.as_ptr(),
		);
	});
}

#[inline]
pub fn gpu_blas_workspace(buf: &GpuBuffer) {
	// SAFETY: hipblas_handle() returns the live handle; buf is a valid GPU buffer of buf.len() bytes.
	let status = unsafe { hipblasSetWorkspace(hipblas_handle(), buf.ptr_raw(), buf.len()) };
	if status != 0i32 {
		drop(Write::err(format!(
			"hipblasSetWorkspace failed with status {status}"
		)));
		process::abort();
	}
}

/// Returns the thread-local hipSOLVER handle, creating it on first use.
pub(crate) fn hipsolver_handle() -> *mut c_void {
	return HIPSOLVER_HANDLE.with(|h| {
		let ptr = h.load(Ordering::Relaxed);
		return ptr::NonNull::new(ptr).map_or_else(
			|| {
				let mut handle: *mut c_void = ptr::null_mut();
				// SAFETY: &raw mut handle is a valid out-pointer to a null-initialized handle that hipsolverCreate fills.
				let status = unsafe { hipsolverCreate(&raw mut handle) };
				if status != 0i32 {
					drop(Write::err(format!(
						"hipsolverCreate failed with status {status}"
					)));
					process::abort();
				}
				h.store(handle, Ordering::Relaxed);
				return handle;
			},
			|existing| return existing.as_ptr(),
		);
	});
}

#[inline]
pub fn gpu_shutdown() {
	callspy::tick(&callspy::DEVICE_SYNCHRONIZE);
	// SAFETY: the device is live during shutdown; hipDeviceSynchronize drains pending work before teardown.
	unsafe {
		hip::hipDeviceSynchronize();
	}
	HIPBLAS_HANDLE.with(|h| {
		let ptr = h.swap(ptr::null_mut(), Ordering::Relaxed);
		if let Some(existing) = ptr::NonNull::new(ptr) {
			// SAFETY: existing is non-null (NonNull) and was returned by hipblasCreate; destroying the live handle once at shutdown is valid.
			unsafe {
				hipblasDestroy(existing.as_ptr());
			}
		}
	});
	HIPSOLVER_HANDLE.with(|h| {
		let ptr = h.swap(ptr::null_mut(), Ordering::Relaxed);
		if let Some(existing) = ptr::NonNull::new(ptr) {
			// SAFETY: existing is non-null (NonNull) and was returned by hipsolverCreate; destroying the live handle once at shutdown is valid.
			unsafe {
				hipsolverDestroy(existing.as_ptr());
			}
		}
	});
	memory::free_bounce();
	memory::free_run_pin();
	let _trim = hip::trim_mempool(0);
	Write::block(device, callspy::report().serialize());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the hipBLAS call fails.
#[inline]
pub fn gpu_gemm(
	mat_a: &GpuBuffer,
	mat_b: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	// SAFETY: mat_a/mat_b/out are valid device buffers sized for the m*n*k GEMM; hipblasDgemm reads A and B and writes C within those bounds.
	let status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_N,
			HIPBLAS_OP_N,
			ci(n)?,
			ci(m)?,
			ci(k)?,
			&raw const alpha,
			mat_b.ptr.cast::<f64>().cast_const(),
			ci(n)?,
			mat_a.ptr.cast::<f64>().cast_const(),
			ci(k)?,
			&raw const beta,
			out.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	return check(status);
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the hipBLAS call fails.
#[inline]
pub fn gpu_gemm_at(
	mat_a: &GpuBuffer,
	mat_b: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	// SAFETY: mat_a/mat_b/out are valid device buffers sized for the m*n*k GEMM; hipblasDgemm reads A and B (B transposed) and writes C within those bounds.
	let status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_N,
			HIPBLAS_OP_T,
			ci(n)?,
			ci(m)?,
			ci(k)?,
			&raw const alpha,
			mat_b.ptr.cast::<f64>(),
			ci(n)?,
			mat_a.ptr.cast::<f64>(),
			ci(m)?,
			&raw const beta,
			out.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	return check(status);
}

/// Multiplies `mat_a` by the transpose of `mat_b`, writing the result into `mat_c`.
///
/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`], or the launch fails.
#[inline]
pub fn gpu_gemm_bt_into(
	mat_a: &GpuBuffer,
	mat_b: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	mat_c: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	// SAFETY: `mat_a`, `mat_b`, and `mat_c` are valid device buffers sized for the given dims.
	let status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_T,
			HIPBLAS_OP_N,
			ci(n)?,
			ci(m)?,
			ci(k)?,
			&raw const alpha,
			mat_b.ptr.cast::<f64>(),
			ci(k)?,
			mat_a.ptr.cast::<f64>(),
			ci(k)?,
			&raw const beta,
			mat_c.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	return check(status);
}

#[must_use]
#[inline]
pub fn gpu_cholesky_solve_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: bufferSize query reads only the shape scalars and writes the workspace size into lwork.
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			122,
			safe_i32(n),
			ptr::null_mut(),
			safe_i32(n),
			&raw mut lwork,
		);
	}
	if let Ok(bytes) = usize::try_from(lwork.max(1i32)) {
		return bytes * 8;
	}
	drop(Write::err(format!("workspace size {lwork} is negative")));
	process::abort();
}

/// # Errors
/// Returns [`HipError`] if a hipSOLVER/hipBLAS call fails or `n` exceeds [`i32`].
#[inline]
pub fn gpu_cholesky_solve(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	info: &GpuBuffer,
	a_copy: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(a, n * n, a_copy)?;
	gpu_copy_into(b, n, out)?;

	let mut lwork: i32 = 0;
	// SAFETY: bufferSize query reads only the shape and a_copy pointer, writing the workspace size into lwork.
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			122,
			ci(n)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			&raw mut lwork,
		);
	}
	// SAFETY: valid hipsolver handle and device pointers; sizes and lda match the buffers.
	let status = unsafe {
		hipsolverDpotrf(
			hipsolver_handle(),
			122,
			ci(n)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			work.ptr.cast::<f64>(),
			lwork,
			info.ptr.cast::<i32>(),
		)
	};
	check(status)?;

	let alpha = 1.0f64;
	// SAFETY: valid hipblas handle and device pointers; sizes and lda match the buffers.
	let status_notrans = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			111,
			131,
			ci(n)?,
			1,
			&raw const alpha,
			a_copy.ptr.cast::<f64>().cast_const(),
			ci(n)?,
			out.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	check(status_notrans)?;

	// SAFETY: valid hipblas handle and device pointers; sizes and lda match the buffers.
	let status_trans = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			112,
			131,
			ci(n)?,
			1,
			&raw const alpha,
			a_copy.ptr.cast::<f64>().cast_const(),
			ci(n)?,
			out.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	return check(status_trans);
}

#[must_use]
#[inline]
pub fn gpu_cholesky_inv_workspace_bytes(n: usize) -> usize {
	return gpu_cholesky_solve_workspace_bytes(n);
}

/// Inverts the SPD matrix `a` of order `n` into `out` using its Cholesky factor.
///
/// # Errors
/// Returns [`HipError`] if a HIP solver/blas call fails or `n` overflows [`i32`].
#[inline]
pub fn gpu_cholesky_inv(
	a: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	info: &GpuBuffer,
	a_copy: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(a, n * n, a_copy)?;

	let mut lwork: i32 = 0;
	// SAFETY: the solver handle is initialized and `a_copy` is a valid device buffer of at least `n * n` elements.
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			122,
			ci(n)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			&raw mut lwork,
		);
	}
	// SAFETY: the solver handle is initialized and `a_copy`, `work`, and `info` are valid device buffers for the `n`-by-`n` factorization.
	let status = unsafe {
		hipsolverDpotrf(
			hipsolver_handle(),
			122,
			ci(n)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			work.ptr.cast::<f64>(),
			lwork,
			info.ptr.cast::<i32>(),
		)
	};
	check(status)?;

	let alpha = 1.0f64;
	// SAFETY: the blas handle is initialized and `a_copy` and `out` are valid device buffers of at least `n * n` elements.
	let status_lower = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			111,
			131,
			ci(n)?,
			ci(n)?,
			&raw const alpha,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			out.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	check(status_lower)?;

	// SAFETY: the hipblas handle and the a_copy/out device buffers are valid and sized n for the triangular solve.
	let status_upper = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			112,
			131,
			ci(n)?,
			ci(n)?,
			&raw const alpha,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			out.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	return check(status_upper);
}

#[must_use]
#[inline]
pub fn gpu_solve_getrf_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: the hipsolver handle is valid; a null data pointer is allowed for a buffer-size-only query.
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			safe_i32(n),
			safe_i32(n),
			ptr::null_mut(),
			safe_i32(n),
			&raw mut lwork,
		);
	}
	return usize::try_from(lwork.max(1)).unwrap_or(0) * 8;
}

#[must_use]
#[inline]
pub fn gpu_solve_getrs_workspace_bytes(n: usize, nrhs: usize) -> usize {
	let mut lwork_s: i32 = 0;
	// SAFETY: hipSOLVER buffer-size query on a valid handle; data pointers null, writes only lwork_s.
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			111,
			safe_i32(n),
			safe_i32(nrhs),
			ptr::null_mut(),
			safe_i32(n),
			ptr::null_mut(),
			ptr::null_mut(),
			safe_i32(n),
			&raw mut lwork_s,
		);
	}
	return usize::try_from(lwork_s.max(1)).unwrap_or(0) * 8;
}

/// Solves the dense linear system `A x = b` on the GPU by LU factorization.
///
/// # Errors
/// Returns [`HipError`] if a dimension overflows [`i32`] or a hipSOLVER call fails.
#[inline]
pub fn gpu_solve(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
	work: &GpuBuffer,
	work_s: &GpuBuffer,
	ipiv: &GpuBuffer,
	info: &GpuBuffer,
	a_copy: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(a, n * n, a_copy)?;
	gpu_copy_into(b, n * nrhs, out)?;

	let mut lwork: i32 = 0;
	// SAFETY: hipSOLVER buffer-size query on a valid handle and LU copy; writes only lwork.
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			ci(n)?,
			ci(n)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			&raw mut lwork,
		);
	}
	// SAFETY: a_copy/work/ipiv/info are live device buffers and the hipsolver handle is initialized; dims are range-checked.
	let status = unsafe {
		hipsolverDgetrf(
			hipsolver_handle(),
			ci(n)?,
			ci(n)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			work.ptr.cast::<f64>(),
			lwork,
			ipiv.ptr.cast::<i32>(),
			info.ptr.cast::<i32>(),
		)
	};
	check(status)?;

	let mut lwork_s: i32 = 0;
	// SAFETY: a_copy/ipiv/out are live device buffers and the hipsolver handle is initialized; dims are range-checked.
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			111,
			ci(n)?,
			ci(nrhs)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			ipiv.ptr.cast::<i32>(),
			out.ptr.cast::<f64>(),
			ci(n)?,
			&raw mut lwork_s,
		);
	}
	// SAFETY: handle is initialized; a_copy/ipiv/out/work_s/info are live device buffers sized for n and nrhs, and dims are range-checked.
	let status_s = unsafe {
		hipsolverDgetrs(
			hipsolver_handle(),
			111,
			ci(n)?,
			ci(nrhs)?,
			a_copy.ptr.cast::<f64>(),
			ci(n)?,
			ipiv.ptr.cast::<i32>(),
			out.ptr.cast::<f64>(),
			ci(n)?,
			work_s.ptr.cast::<f64>(),
			lwork_s,
			info.ptr.cast::<i32>(),
		)
	};
	return check(status_s);
}

#[must_use]
#[inline]
pub fn gpu_cholesky_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: bufferSize query computes only the workspace size and dereferences no device memory; A is intentionally null and lwork is a valid local out-param.
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			121,
			safe_i32(n),
			ptr::null_mut(),
			safe_i32(n),
			&raw mut lwork,
		);
	}
	return usize::try_from(lwork.max(1)).unwrap_or(0) * 8;
}

/// # Errors
/// Returns [`HipError`] when a dimension exceeds [`i32`] range or the hipSOLVER factorization fails.
#[inline]
pub fn gpu_cholesky(
	a: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	info: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(a, n * n, out)?;
	let mut lwork: i32 = 0;
	// SAFETY: hipSOLVER buffer-size query; the handle is live and the buffer pointers and dimensions are valid for this call.
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			121,
			ci(n)?,
			out.ptr.cast::<f64>(),
			ci(n)?,
			&raw mut lwork,
		);
	}
	// SAFETY: hipSOLVER Cholesky factorization; the handle is live and all device buffers and dimensions are valid.
	let status = unsafe {
		hipsolverDpotrf(
			hipsolver_handle(),
			121,
			ci(n)?,
			out.ptr.cast::<f64>(),
			ci(n)?,
			work.ptr.cast::<f64>(),
			lwork,
			info.ptr.cast::<i32>(),
		)
	};
	return check(status);
}

/// # Errors
/// Returns [`HipError`] when a dimension exceeds [`i32`] range or the hipBLAS solve fails.
#[inline]
pub fn gpu_tri_solve(
	l: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
	trans: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(b, n * nrhs, out)?;
	let alpha = 1.0f64;
	let trans_flag = match trans.cmp(&0) {
		cmp::Ordering::Equal => 111u32,
		cmp::Ordering::Less | cmp::Ordering::Greater => 112u32,
	};
	// SAFETY: hipBLAS triangular solve; the handle is live and all device buffers and dimensions are valid.
	let status = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			121,
			trans_flag,
			131,
			ci(n)?,
			ci(nrhs)?,
			&raw const alpha,
			l.ptr.cast::<f64>().cast_const(),
			ci(n)?,
			out.ptr.cast::<f64>(),
			ci(n)?,
		)
	};
	return check(status);
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_add_diag(val: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: a and val are valid device buffers of length n and launch_add_diag touches only those n elements.
	unsafe {
		launch_add_diag(
			a.ptr,
			ci(n)?,
			val.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_reparameterize(
	mu: &GpuBuffer,
	log_var: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: mu, log_var, eps and out are valid device buffers of length n and launch_reparameterize touches only those n elements.
	unsafe {
		launch_reparameterize(
			mu.ptr.cast_const(),
			log_var.ptr.cast_const(),
			eps.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_kl_div(
	mu: &GpuBuffer,
	log_var: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: mu, log_var and out are valid device buffers of length n and launch_kl_div touches only those n elements.
	unsafe {
		launch_kl_div(
			mu.ptr.cast_const(),
			log_var.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_vae_backward_latent(
	grad_z: &GpuBuffer,
	mu: &GpuBuffer,
	log_var: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	kl_weight: &GpuBuffer,
	grad_mu: &GpuBuffer,
	grad_lv: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: every buffer is a valid device allocation of at least `n` elements; the null stream is the default stream.
	unsafe {
		launch_vae_backward_latent(
			grad_z.ptr.cast_const(),
			mu.ptr.cast_const(),
			log_var.ptr.cast_const(),
			eps.ptr.cast_const(),
			grad_mu.ptr,
			grad_lv.ptr,
			n_i32,
			kl_weight.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_log_det_cholesky(l: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `l` and `out` are valid device buffers sized for the `n`-by-`n` factorization; the null stream is the default stream.
	unsafe {
		launch_log_det_cholesky(l.ptr.cast_const(), out.ptr, n_i32, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_scaled_exp(
	x: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x`, `scale`, and `out` are valid device buffers of at least `n` elements; the null stream is the default stream.
	unsafe {
		launch_scaled_exp(
			x.ptr.cast_const(),
			out.ptr,
			n_i32,
			scale.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_sigmoid_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: FFI call into launch_sigmoid with device pointers from live buffers and a checked length
	unsafe {
		launch_sigmoid(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` if `n` overflows `i32`.
#[inline]
pub fn gpu_sigmoid_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI call into launch_sigmoid_backward with device pointers from live buffers and a checked length
	unsafe {
		launch_sigmoid_backward(
			grad.ptr.cast_const(),
			act.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` if `n` overflows `i32`.
#[inline]
pub fn gpu_tanh_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: FFI call into launch_tanh_act with device pointers from live buffers and a checked length
	unsafe {
		launch_tanh_act(x.ptr.cast_const(), out.ptr, ci(n)?, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}
/// # Errors
/// Returns `HipError` if `n` overflows `i32`.
#[inline]
pub fn gpu_tanh_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI call into launch_tanh_backward with device pointers from live buffers and a checked length
	unsafe {
		launch_tanh_backward(
			grad.ptr.cast_const(),
			act.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if the element count overflows `i32`.
#[inline]
pub fn gpu_leaky_relu_into(
	x: &GpuBuffer,
	alpha: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the launch reads only live GpuBuffer device allocations through valid FFI pointers.
	unsafe {
		launch_leaky_relu(
			x.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			alpha.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
/// # Errors
/// Returns [`HipError`] if the element count overflows `i32`.
#[inline]
pub fn gpu_leaky_relu_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	alpha: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the launch reads only live GpuBuffer device allocations through valid FFI pointers.
	unsafe {
		launch_leaky_relu_backward(
			grad.ptr.cast_const(),
			act.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			alpha.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if the element count overflows `i32`.
#[inline]
pub fn gpu_silu_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: the launch reads only live GpuBuffer device allocations through valid FFI pointers.
	unsafe {
		launch_silu(x.ptr.cast_const(), out.ptr, ci(n)?, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}
/// # Errors
/// Returns [`HipError`] if the element count overflows `i32`.
#[inline]
pub fn gpu_silu_backward_into(
	grad: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the launch reads only live GpuBuffer device allocations through valid FFI pointers.
	unsafe {
		launch_silu_backward(
			grad.ptr.cast_const(),
			x.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns a `HipError` if `n` exceeds `i32::MAX`.
#[inline]
pub fn gpu_relu_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: the device pointers are live, ledgered allocations valid for `n` elements.
	unsafe {
		launch_relu(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns a `HipError` if `n` exceeds `i32::MAX`.
#[inline]
pub fn gpu_relu_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the device pointers are live, ledgered allocations valid for `n` elements.
	unsafe {
		launch_relu_backward(
			grad.ptr.cast_const(),
			act.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns a `HipError` if `n` exceeds `i32::MAX`.
#[inline]
pub fn gpu_add_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the device pointers are live, ledgered allocations valid for `n` elements.
	unsafe {
		launch_add(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` exceeds `i32::MAX`.
#[inline]
pub fn gpu_add_scalar(
	x: &GpuBuffer,
	s: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: x, s, out are live device buffers of n f64 elements and launch_add_scalar only enqueues a kernel over them.
	unsafe {
		launch_add_scalar(
			x.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			s.ptr.cast::<f64>(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` exceeds `i32::MAX`.
#[inline]
pub fn gpu_div_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: a, b, out are live device buffers of n f64 elements and launch_div only enqueues a kernel over them.
	unsafe {
		launch_div(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Propagates any [`HipError`] from the underlying scale kernel launch.
#[inline]
pub fn gpu_scale_inplace(scalar: &GpuBuffer, n: usize, x: &GpuBuffer) -> Result<(), HipError> {
	return gpu_scale_f64_inplace(scalar, n, x);
}

/// # Errors
/// Returns [`HipError`] if `n` exceeds `i32::MAX`.
#[inline]
pub fn gpu_fma(
	x: &GpuBuffer,
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: x, a, b, out are live device buffers of n f64 elements and launch_fma only enqueues a kernel over them.
	unsafe {
		launch_fma(
			x.ptr.cast_const(),
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_sgd_update(
	grad: &GpuBuffer,
	neg_lr: &GpuBuffer,
	n: usize,
	weights: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: `weights`, `grad`, and `neg_lr` are live device buffers of `n` f64 that outlive the launch.
	unsafe {
		launch_sgd_update_f64(
			weights.ptr.cast::<f64>(),
			grad.ptr.cast::<f64>().cast_const(),
			neg_lr.ptr.cast::<f64>().cast_const(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_mul(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: `a`, `b`, and `out` are live device buffers of `n` f64 that outlive the launch.
	unsafe {
		launch_mul(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_mul_inplace(b: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: `a` and `b` are live device buffers of `n` f64 that outlive the launch.
	unsafe {
		launch_mul_inplace(
			a.ptr.cast::<c_void>(),
			b.ptr.cast_const(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_add_inplace(b: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: `a` and `b` are live device buffers of `n` f64 that outlive the launch.
	unsafe {
		launch_add(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			a.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if `n` overflows `i32`.
#[inline]
pub fn gpu_sub_inplace(b: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: a and b are live device buffers of at least n f64 elements; launch_sub only touches memory within those bounds.
	unsafe {
		launch_sub(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			a.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if `n` overflows `i32`.
#[inline]
pub fn gpu_add_scalar_inplace(s: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: a is a live device buffer of at least n f64 elements and s a device scalar; launch_add_scalar stays within those bounds.
	unsafe {
		launch_add_scalar(
			a.ptr.cast_const(),
			a.ptr.cast::<c_void>(),
			ci(n)?,
			s.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if a dimension overflows `i32`.
#[inline]
pub fn gpu_linear_into(
	xin: &GpuBuffer,
	wgt: &GpuBuffer,
	bias: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::math_ops::launch_tall_skinny_dgemm;
	// SAFETY: bias and out are live device buffers sized for the m by n output; launch_repeat_rows writes only within out.
	unsafe {
		launch_repeat_rows(
			bias.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ci(m * n)?,
			ptr::null_mut(),
		);
	}
	match n.cmp(&16) {
		cmp::Ordering::Less | cmp::Ordering::Equal => {
			// SAFETY: device pointers and dims come from the caller's live GpuBuffers; the dgemm launcher only reads and writes within them.
			unsafe {
				launch_tall_skinny_dgemm(
					xin.ptr.cast_const(),
					wgt.ptr.cast_const(),
					out.ptr.cast::<c_void>(),
					ci(m)?,
					ci(n)?,
					ci(k)?,
					ptr::null_mut(),
				);
			}
			check_launch();
			return Ok(());
		}
		cmp::Ordering::Greater => {
			let alpha = 1.0f64;
			let beta = 1.0f64;
			// SAFETY: hipblasDgemm reads the live handle plus device pointers and dims from valid GpuBuffers.
			let status = unsafe {
				hipblasDgemm(
					hipblas_handle(),
					HIPBLAS_OP_N,
					HIPBLAS_OP_N,
					ci(n)?,
					ci(m)?,
					ci(k)?,
					&raw const alpha,
					wgt.ptr.cast::<f64>().cast_const(),
					ci(n)?,
					xin.ptr.cast::<f64>().cast_const(),
					ci(k)?,
					&raw const beta,
					out.ptr.cast::<f64>(),
					ci(n)?,
				)
			};
			return check(status);
		}
	}
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_ss_res_into(
	pred: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::memory::memset_dev;
	// SAFETY: out.ptr is a live device allocation of at least one f64; memset_dev writes size_of::<f64>() zero bytes there.
	unsafe {
		let _memset = memset_dev(out.ptr, 0, mem::size_of::<f64>(), ptr::null_mut());
	}
	// SAFETY: pred, y, and out are live device buffers of n f64 elements matching launch_ss_res's parameters.
	unsafe {
		launch_ss_res(
			pred.ptr.cast_const(),
			y.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_mse_into(
	pred: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::memory::memset_dev;
	// SAFETY: out.ptr is a live device allocation of at least one f64; memset_dev writes size_of::<f64>() zero bytes there.
	unsafe {
		let _memset = memset_dev(out.ptr, 0, mem::size_of::<f64>(), ptr::null_mut());
	}
	// SAFETY: pred, y, and out are live device buffers of n f64 elements matching launch_mse's parameters.
	unsafe {
		launch_mse(
			pred.ptr.cast_const(),
			y.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_accuracy_into(
	pred: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::memory::memset_dev;
	// SAFETY: out.ptr is a live device allocation of at least one f64; memset_dev writes size_of::<f64>() zero bytes there.
	unsafe {
		let _memset = memset_dev(out.ptr, 0, mem::size_of::<f64>(), ptr::null_mut());
	}
	// SAFETY: pred, y, and out are live device buffers of n f64 elements matching launch_accuracy_metric's parameters.
	unsafe {
		launch_accuracy_metric(
			pred.ptr.cast_const(),
			y.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_bce_grad_into(
	pred: &GpuBuffer,
	y: &GpuBuffer,
	inv_n: &GpuBuffer,
	n: usize,
	da: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pred, y, inv_n and da are live device buffers sized for n; launch_bce_grad only reads and writes within those bounds.
	unsafe {
		launch_bce_grad(
			pred.ptr.cast_const(),
			y.ptr.cast_const(),
			da.ptr,
			ci(n)?,
			inv_n.ptr.cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` or `k` overflows [`i32`].
#[inline]
pub fn gpu_argmax_accuracy_into(
	pred: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::memory::memset_dev;
	// SAFETY: out.ptr is a live device buffer of at least one f64; memset_dev zeroes eight bytes within it.
	unsafe {
		let _memset = memset_dev(out.ptr, 0, mem::size_of::<f64>(), ptr::null_mut());
	}

	// SAFETY: pred and y are live device buffers of length n*k and out holds one f64; launch_argmax_acc reads and writes only within those bounds.
	unsafe {
		launch_argmax_acc(
			pred.ptr.cast_const(),
			y.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(k)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_abs_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: x is a live device buffer of length n and out holds n f64 slots; launch_abs reads x and writes out within bounds.
	unsafe {
		launch_abs(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i32`].
#[inline]
pub fn gpu_log_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launch_log reads x and writes out, both valid device buffers of n f64 elements.
	unsafe {
		launch_log(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows [`i64`].
#[inline]
pub fn gpu_copy_into(src: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launch_copy_f64 copies n f64 elements from src to out, both valid device buffers.
	unsafe {
		launch_copy_f64(
			src.ptr.cast_const(),
			out.ptr,
			i64::try_from(n).map_err(|_e| return HipError(1))?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `rows` or `cols` overflows [`i32`].
#[inline]
pub fn gpu_reduce_sum_cols_into(
	x: &GpuBuffer,
	reduce_ws: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: reduce_sum_cols_workspace_bytes computes the byte count from the dims; its pointer args are unused.
	let ws = unsafe {
		reduce_sum_cols_workspace_bytes(ptr::null(), ci(rows)?, ci(cols)?, ptr::null_mut())
	};
	// SAFETY: launch_reduce_sum_cols reads x, writes out, and uses reduce_ws of ws bytes, all valid device buffers for the dims.
	unsafe {
		launch_reduce_sum_cols(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(rows)?,
			ci(cols)?,
			reduce_ws.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[must_use]
#[inline]
pub fn gpu_reduce_sum_cols_workspace_bytes(rows: usize, cols: usize) -> usize {
	// SAFETY: null in/out pointers request only the workspace byte count; no device memory is dereferenced.
	unsafe {
		return reduce_sum_cols_workspace_bytes(
			ptr::null(),
			safe_i32(rows),
			safe_i32(cols),
			ptr::null_mut(),
		);
	}
}

/// Computes `out = w*x + b` on device (bias broadcast then GEMV).
///
/// # Errors
/// Returns [`HipError`] if `n` or `in_dim` overflow [`i32`], or the GEMV call fails.
#[inline]
pub fn gpu_matvec_bias_into(
	x: &GpuBuffer,
	w: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	in_dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::hip::hipblasDgemv;
	// SAFETY: b and out are valid device buffers; launch_repeat_rows only writes within their lengths.
	unsafe {
		launch_repeat_rows(
			b.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			1,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	let alpha = 1.0f64;
	let beta = 1.0f64;
	// SAFETY: handle and the x/w/out device pointers are valid for the GEMV dimensions passed.
	let status = unsafe {
		hipblasDgemv(
			hipblas_handle(),
			HIPBLAS_OP_T,
			ci(in_dim)?,
			ci(n)?,
			&raw const alpha,
			x.ptr.cast::<f64>(),
			ci(in_dim)?,
			w.ptr.cast::<f64>(),
			1,
			&raw const beta,
			out.ptr.cast::<f64>(),
			1,
		)
	};
	check(status)?;
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the hipBLAS gemv call fails.
#[inline]
pub fn gpu_dgemv_into(
	a: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	in_dim: usize,
	trans: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::hip::hipblasDgemv;
	let op = match trans.cmp(&0) {
		cmp::Ordering::Equal => HIPBLAS_OP_T,
		cmp::Ordering::Less | cmp::Ordering::Greater => HIPBLAS_OP_N,
	};
	let alpha = 1.0f64;
	let beta = 0.0f64;

	// SAFETY: FFI hipBLAS dgemv over device buffers with checked dimensions.
	let status = unsafe {
		hipblasDgemv(
			hipblas_handle(),
			op,
			ci(in_dim)?,
			ci(n)?,
			&raw const alpha,
			a.ptr.cast::<f64>().cast_const(),
			ci(in_dim)?,
			x.ptr.cast::<f64>().cast_const(),
			1,
			&raw const beta,
			out.ptr.cast::<f64>(),
			1,
		)
	};
	check(status)?;
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the hipBLAS dger call fails.
#[inline]
pub fn gpu_dger_into(
	grad: &GpuBuffer,
	w: &GpuBuffer,
	n: usize,
	in_dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::hip::hipblasDger;
	use crate::memory::memset_dev;
	// SAFETY: zero the output buffer before the rank-1 update writes into it.
	unsafe {
		let _memset = memset_dev(
			out.ptr,
			0,
			n * in_dim * mem::size_of::<f64>(),
			ptr::null_mut(),
		);
	}
	let alpha = 1.0f64;

	// SAFETY: FFI hipBLAS dger over device buffers with checked dimensions.
	let status = unsafe {
		hipblasDger(
			hipblas_handle(),
			ci(in_dim)?,
			ci(n)?,
			&raw const alpha,
			w.ptr.cast::<f64>().cast_const(),
			1,
			grad.ptr.cast::<f64>().cast_const(),
			1,
			out.ptr.cast::<f64>(),
			ci(in_dim)?,
		)
	};
	check(status)?;
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] on size overflow or launch failure.
#[inline]
pub fn gpu_layernorm_into(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	beta: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pointers are live GpuBuffer allocations and the launcher touches only the rows*cols extent.
	unsafe {
		launch_layernorm(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			gamma.ptr.cast_const(),
			beta.ptr.cast_const(),
			ci(rows)?,
			ci(cols)?,
			eps.ptr.cast::<f64>(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// LayerNorm with optional affine params, mirroring llama.cpp `build_norm(x,
/// gamma_or_null, beta_or_null, LLM_NORM)`: `None` gamma runs a non-parametric
/// LayerNorm (mean-center + unit scale), a present gamma with `None` beta runs a
/// gamma-only affine (zero bias). The kernel branches on the null pointers, so the
/// runtime composes every norm variant from one op with no per-arch code.
///
/// # Errors
/// Returns [`HipError`] on size overflow or launch failure.
#[inline]
pub fn gpu_layernorm_opt_into(
	x: &GpuBuffer,
	gamma: Option<&GpuBuffer>,
	beta: Option<&GpuBuffer>,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let gp = gamma.map_or(ptr::null(), |g| g.ptr.cast_const());
	let bp = beta.map_or(ptr::null(), |b| b.ptr.cast_const());
	// SAFETY: x/out/eps are live allocations and gamma/beta are either live or null; the launcher touches only rows*cols and null-guards the affine reads.
	unsafe {
		launch_layernorm(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			gp,
			bp,
			ci(rows)?,
			ci(cols)?,
			eps.ptr.cast::<f64>(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] on size overflow or launch failure.
#[inline]
pub fn gpu_gelu_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: pointers are live GpuBuffer allocations and the launcher reads/writes only the first n elements.
	unsafe {
		launch_gelu(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] on size overflow or launch failure.
#[inline]
pub fn gpu_gelu_backward_into(
	grad: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pointers are live GpuBuffer allocations and the launcher reads/writes only the first n elements.
	unsafe {
		launch_gelu_backward(
			grad.ptr.cast_const(),
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows `i32`.
#[inline]
pub fn gpu_dropout_into(
	x: &GpuBuffer,
	mask: &GpuBuffer,
	p: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the buffer pointers are live GPU device allocations and the launch dimensions stay within their extents.
	unsafe {
		launch_dropout(
			x.ptr.cast_const(),
			mask.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			p.ptr.cast::<f64>().cast_const(),
			scale.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` or `seed` overflow the launcher integer type.
#[inline]
pub fn gpu_rand_uniform_into(seed: usize, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: the buffer pointers are live GPU device allocations and the launch dimensions stay within their extents.
	unsafe {
		launch_rand_uniform(out.ptr.cast::<c_void>(), ci(n)?, cu(seed)?, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a launch dimension overflows `i32`.
#[inline]
pub fn gpu_softmax_ce_grad_into(
	logits: &GpuBuffer,
	targets: &GpuBuffer,
	weights: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	nc: usize,
	grad_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the buffer pointers are live GPU device allocations and the launch dimensions stay within their extents.
	unsafe {
		launch_softmax_ce_grad(
			logits.ptr.cast_const(),
			targets.ptr.cast_const(),
			weights.ptr.cast_const(),
			grad_out.ptr.cast::<c_void>(),
			ci(n)?,
			ci(nc)?,
			scale.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[must_use]
#[inline]
pub fn gpu_splitk_dw_partials_elems(m: usize, k: usize, n: usize) -> usize {
	use crate::math_ops::splitk_dw_partials_elems;
	return splitk_dw_partials_elems(m, k, n);
}

/// Launches the split-K `dW` accumulation for the given shape into `grad_w`.
///
/// # Errors
/// Returns [`HipError`] if a dimension overflows [`i32`].
#[inline]
pub fn gpu_splitk_dw_into(
	input: &GpuBuffer,
	grad: &GpuBuffer,
	partials: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	grad_w: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::math_ops::{launch_splitk_dw, splitk_dw_p};
	let p = splitk_dw_p(m, k, n);

	// SAFETY: buffers outlive the launch and the dims are range-checked into i32 above.
	unsafe {
		launch_splitk_dw(
			input.ptr.cast_const(),
			grad.ptr.cast_const(),
			partials.ptr.cast::<c_void>(),
			grad_w.ptr.cast::<c_void>(),
			ci(m)?,
			ci(n)?,
			ci(k)?,
			ci(p)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32`.
#[inline]
pub fn gpu_linear_backward_weights_only_into(
	grad: &GpuBuffer,
	input: &GpuBuffer,
	reduce_ws: &GpuBuffer,
	partials: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	grad_w: &GpuBuffer,
	grad_b: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: `grad.ptr` is a valid device pointer; the call only queries the reduction workspace byte size and launches nothing.
	let ws = unsafe {
		reduce_sum_cols_workspace_bytes(grad.ptr.cast_const(), ci(m)?, ci(n)?, ptr::null_mut())
	};
	gpu_splitk_dw_into(input, grad, partials, m, n, k, grad_w)?;
	// SAFETY: `grad`/`grad_b` device pointers and the `reduce_ws` workspace are valid for this column-reduction launch on the default stream.
	unsafe {
		launch_reduce_sum_cols(
			grad.ptr.cast_const(),
			grad_b.ptr.cast::<c_void>(),
			ci(m)?,
			ci(n)?,
			reduce_ws.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the backward GEMM launch fails.
#[inline]
pub fn gpu_linear_backward_full_into(
	grad: &GpuBuffer,
	input: &GpuBuffer,
	weight: &GpuBuffer,
	reduce_ws: &GpuBuffer,
	partials: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	grad_input: &GpuBuffer,
	grad_w: &GpuBuffer,
	grad_b: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	gpu_splitk_dw_into(input, grad, partials, m, n, k, grad_w)?;
	// SAFETY: `grad.ptr` is a valid device pointer; the call only queries the reduction workspace byte size and launches nothing.
	let ws = unsafe {
		reduce_sum_cols_workspace_bytes(grad.ptr.cast_const(), ci(m)?, ci(n)?, ptr::null_mut())
	};
	// SAFETY: `grad`/`grad_b` device pointers and the `reduce_ws` workspace are valid for this column-reduction launch on the default stream.
	unsafe {
		launch_reduce_sum_cols(
			grad.ptr.cast_const(),
			grad_b.ptr.cast::<c_void>(),
			ci(m)?,
			ci(n)?,
			reduce_ws.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	// SAFETY: the device pointers are valid GPU allocations sized for the m/n/k dims; hipblas reads and writes within those bounds.
	let gi_status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_T,
			HIPBLAS_OP_N,
			ci(k)?,
			ci(m)?,
			ci(n)?,
			&raw const alpha,
			weight.ptr.cast::<f64>().cast_const(),
			ci(n)?,
			grad.ptr.cast::<f64>().cast_const(),
			ci(n)?,
			&raw const beta,
			grad_input.ptr.cast::<f64>(),
			ci(k)?,
		)
	};
	check(gi_status)?;
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds `i32` or the kernel launch fails.
#[inline]
pub fn gpu_layernorm_backward_full_into(
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
	// SAFETY: the buffers are valid device allocations sized for rows*cols; the launcher stays within those bounds.
	unsafe {
		launch_layernorm_backward(
			grad_y.ptr.cast_const(),
			x.ptr.cast_const(),
			gamma.ptr.cast_const(),
			grad_x.ptr.cast::<c_void>(),
			grad_gamma.ptr.cast::<c_void>(),
			grad_beta.ptr.cast::<c_void>(),
			ci(rows)?,
			ci(cols)?,
			eps.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` if `rows` or `cols` overflow `i32`.
#[inline]
pub fn gpu_softmax_rows_into(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the device buffer pointers are valid allocations owned by the caller for this launch.
	unsafe {
		launch_softmax_rows(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` if any dimension overflows `i32`.
#[inline]
pub fn gpu_flash_attention_into(
	query: &GpuBuffer,
	key: &GpuBuffer,
	val: &GpuBuffer,
	n: usize,
	seq: usize,
	d: usize,
	heads: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the device buffer pointers are valid allocations owned by the caller for this launch.
	unsafe {
		launch_flash_attention_f64(
			query.ptr.cast_const(),
			key.ptr.cast_const(),
			val.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ci(seq)?,
			ci(d)?,
			ci(heads)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds `i32` range.
#[inline]
pub fn gpu_flash_attention_train_into(
	query: &GpuBuffer,
	key: &GpuBuffer,
	val: &GpuBuffer,
	n: usize,
	seq: usize,
	d: usize,
	heads: usize,
	out: &GpuBuffer,
	lse: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: query/key/val/out/lse are live device buffers and dims are range-checked before this FFI kernel launch.
	unsafe {
		let n32 = ci(n)?;
		let seq32 = ci(seq)?;
		let d32 = ci(d)?;
		let heads32 = ci(heads)?;
		launch_flash_attention_f64_train_fwd(
			query.ptr.cast_const(),
			key.ptr.cast_const(),
			val.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			lse.ptr.cast::<c_void>(),
			n32,
			seq32,
			d32,
			heads32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a launch dimension does not fit in [`i32`].
#[inline]
pub fn gpu_flash_attention_backward_into(
	query: &GpuBuffer,
	key: &GpuBuffer,
	val: &GpuBuffer,
	ctx: &GpuBuffer,
	dctx: &GpuBuffer,
	lse: &GpuBuffer,
	n: usize,
	seq: usize,
	d: usize,
	heads: usize,
	dsum: &GpuBuffer,
	dq: &GpuBuffer,
	dk: &GpuBuffer,
	dv: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: ctx/dctx/dsum are live device buffers and the dims are range-checked.
	unsafe {
		launch_flash_attention_f64_dsum(
			ctx.ptr.cast_const(),
			dctx.ptr.cast_const(),
			dsum.ptr.cast::<c_void>(),
			ci(n)?,
			ci(seq)?,
			ci(d)?,
			ci(heads)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	// SAFETY: query/key/val/dctx/lse/dsum/dq are live device buffers and the dims are range-checked.
	unsafe {
		launch_flash_attention_f64_bwd_dq(
			query.ptr.cast_const(),
			key.ptr.cast_const(),
			val.ptr.cast_const(),
			dctx.ptr.cast_const(),
			lse.ptr.cast_const(),
			dsum.ptr.cast_const(),
			dq.ptr.cast::<c_void>(),
			ci(n)?,
			ci(seq)?,
			ci(d)?,
			ci(heads)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	// SAFETY: every pointer is a live GpuBuffer valid for this launch's element count and the null stream is the default stream
	unsafe {
		launch_flash_attention_f64_bwd_dkv(
			query.ptr.cast_const(),
			key.ptr.cast_const(),
			val.ptr.cast_const(),
			dctx.ptr.cast_const(),
			lse.ptr.cast_const(),
			dsum.ptr.cast_const(),
			dk.ptr.cast::<c_void>(),
			dv.ptr.cast::<c_void>(),
			ci(n)?,
			ci(seq)?,
			ci(d)?,
			ci(heads)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when `n` or `seed` overflows the launcher integer width.
#[inline]
pub fn gpu_bernoulli_into(
	p: &GpuBuffer,
	seed: usize,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n32 = ci(n)?;
	let seed32 = cu(seed)?;
	// SAFETY: p and out are valid device buffers of n f64 elements; launch_bernoulli reads p and writes out within bounds.
	unsafe {
		launch_bernoulli(
			out.ptr.cast::<c_void>(),
			n32,
			p.ptr.cast::<f64>().cast_const(),
			seed32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size or dimension exceeds `i32` range.
#[inline]
pub fn gpu_grad_hess_into(
	probs: &GpuBuffer,
	targets: &GpuBuffer,
	weights: &GpuBuffer,
	mask: &GpuBuffer,
	n: usize,
	nc: usize,
	k: usize,
	grad_out: &GpuBuffer,
	hess_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n32 = ci(n)?;
	let cls32 = ci(nc)?;
	let k32 = ci(k)?;
	// SAFETY: all buffers are valid device allocations sized per n/nc/k; launch_grad_hess reads inputs and writes grad_out and hess_out within bounds.
	unsafe {
		launch_grad_hess(
			probs.ptr.cast_const(),
			targets.ptr.cast_const(),
			weights.ptr.cast_const(),
			mask.ptr.cast_const(),
			grad_out.ptr.cast::<c_void>(),
			hess_out.ptr.cast::<c_void>(),
			n32,
			cls32,
			k32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size or dimension exceeds `i32` range.
#[inline]
pub fn gpu_tb_histogram(
	tr_bins: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	node_assign: &GpuBuffer,
	n_tr: usize,
	p: usize,
	n_bins: usize,
	level_base: usize,
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffers are valid device allocations sized per the histogram dims; launch_tb_histogram reads inputs and writes grad_hist and hess_hist within bounds.
	unsafe {
		launch_tb_histogram(
			tr_bins.ptr.cast_const(),
			grad.ptr.cast_const(),
			hess.ptr.cast_const(),
			node_assign.ptr.cast_const(),
			grad_hist.ptr,
			hess_hist.ptr,
			ci(n_tr)?,
			ci(p)?,
			ci(n_bins)?,
			ci(level_base)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a launch dimension does not fit in [`i32`].
#[inline]
pub fn gpu_tb_split_eval(
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
	lambda: &GpuBuffer,
	min_cw: &GpuBuffer,
	n_level: usize,
	p: usize,
	n_bins: usize,
	level_base: usize,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer refers to a live `GpuBuffer` allocation and the launcher reads and writes only within the passed dimensions.
	unsafe {
		launch_tb_split_eval(
			grad_hist.ptr.cast_const(),
			hess_hist.ptr.cast_const(),
			split_feat.ptr,
			split_bin.ptr,
			ci(n_level)?,
			ci(p)?,
			ci(n_bins)?,
			lambda.ptr.cast::<f64>().cast_const(),
			min_cw.ptr.cast::<f64>().cast_const(),
			ci(level_base)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when `n_tr` or `p` overflows its target width.
#[inline]
pub fn gpu_tb_repartition(
	tr_bins: &GpuBuffer,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
	n_tr: usize,
	p: usize,
	node_assign: &GpuBuffer,
) -> Result<(), HipError> {
	let n_tr32 = ci(n_tr)?;
	let p32 = ci(p)?;
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_tb_repartition(
			tr_bins.ptr.cast_const(),
			node_assign.ptr,
			split_feat.ptr.cast_const(),
			split_bin.ptr.cast_const(),
			n_tr32,
			p32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when `n_tr` overflows its target width.
#[inline]
pub fn gpu_tb_leaf_sum(
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	node_assign: &GpuBuffer,
	n_tr: usize,
	node_sum_g: &GpuBuffer,
	node_sum_h: &GpuBuffer,
) -> Result<(), HipError> {
	let n_tr32 = ci(n_tr)?;
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_tb_leaf_sum(
			grad.ptr.cast_const(),
			hess.ptr.cast_const(),
			node_assign.ptr.cast_const(),
			node_sum_g.ptr,
			node_sum_h.ptr,
			n_tr32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when `n_leaves` overflows its target width.
#[inline]
pub fn gpu_tb_leaf_val(
	node_sum_g: &GpuBuffer,
	node_sum_h: &GpuBuffer,
	lambda: &GpuBuffer,
	n_leaves: usize,
	leaf_base: usize,
	leaf_val: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_tb_leaf_val(
			node_sum_g.ptr.cast_const(),
			node_sum_h.ptr.cast_const(),
			leaf_val.ptr,
			ci(n_leaves)?,
			ci(leaf_base)?,
			lambda.ptr.cast::<f64>(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n_tr` overflows `i32`.
#[inline]
pub fn gpu_tb_scatter(
	node_assign: &GpuBuffer,
	leaf_val: &GpuBuffer,
	n_tr: usize,
	predictions: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all pointers are live GpuBuffer device allocations valid for the launch.
	unsafe {
		launch_tb_scatter(
			node_assign.ptr.cast_const(),
			leaf_val.ptr.cast_const(),
			predictions.ptr,
			ci(n_tr)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_tb_apply_tree(
	te_bins: &GpuBuffer,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
	leaf_val: &GpuBuffer,
	n_te: usize,
	p: usize,
	max_depth: usize,
	predictions: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every buffer pointer is a live device allocation sized for this launch and the size args bound the kernel's indexing.
	unsafe {
		launch_tb_apply_tree(
			te_bins.ptr.cast_const(),
			split_feat.ptr.cast_const(),
			split_bin.ptr.cast_const(),
			leaf_val.ptr.cast_const(),
			predictions.ptr,
			ci(n_te)?,
			ci(p)?,
			ci(max_depth)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if allocation, memset, or a kernel launch fails.
#[inline]
pub fn gpu_tree_build_into(
	train_bins: &GpuBuffer,
	test_bins: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	n_train: usize,
	n_test: usize,
	p: usize,
	n_bins: usize,
	max_depth: usize,
	lambda: &GpuBuffer,
	min_cw: &GpuBuffer,
	train_pred: &GpuBuffer,
	test_pred: &GpuBuffer,
) -> Result<(), HipError> {
	let isz = mem::size_of::<i32>();
	let fsz = mem::size_of::<f64>();
	let max_nodes = (1usize << (max_depth + 1)) - 1;
	let max_level = match max_depth.cmp(&1) {
		cmp::Ordering::Less | cmp::Ordering::Equal => 1usize,
		cmp::Ordering::Greater => 1usize << (max_depth - 1),
	};
	let hist_elems = max_level * p * n_bins;

	let node_assign = GpuBuffer::alloc_bytes(n_train * isz).map_err(|e| {
		drop(Write::err(format!("tb node_assign: {e}")));
		return e;
	})?;
	node_assign.memset_zero(n_train * isz).map_err(|e| {
		drop(Write::err(format!("tb node_assign zero: {e}")));
		return e;
	})?;
	let sf = GpuBuffer::alloc_bytes(max_nodes * isz).map_err(|e| {
		drop(Write::err(format!("tb split_feat: {e}")));
		return e;
	})?;
	let sb = GpuBuffer::alloc_bytes(max_nodes * isz).map_err(|e| {
		drop(Write::err(format!("tb split_bin: {e}")));
		return e;
	})?;
	sf.fill_bytes(0xFF, max_nodes * isz).map_err(|e| {
		drop(Write::err(format!("tb split_feat fill: {e}")));
		return e;
	})?;
	sb.fill_bytes(0xFF, max_nodes * isz).map_err(|e| {
		drop(Write::err(format!("tb split_bin fill: {e}")));
		return e;
	})?;
	let gh = GpuBuffer::alloc(hist_elems).map_err(|e| {
		drop(Write::err(format!("tb grad_hist: {e}")));
		return e;
	})?;
	let hh = GpuBuffer::alloc(hist_elems).map_err(|e| {
		drop(Write::err(format!("tb hess_hist: {e}")));
		return e;
	})?;
	let sum_g = GpuBuffer::alloc(max_nodes).map_err(|e| {
		drop(Write::err(format!("tb node_sum_g: {e}")));
		return e;
	})?;
	let sum_h = GpuBuffer::alloc(max_nodes).map_err(|e| {
		drop(Write::err(format!("tb node_sum_h: {e}")));
		return e;
	})?;
	let lv = GpuBuffer::alloc(max_nodes).map_err(|e| {
		drop(Write::err(format!("tb leaf_val: {e}")));
		return e;
	})?;

	for d in 0..max_depth {
		let level_base = (1usize << d) - 1;
		let n_level = 1usize << d;
		let level_bytes = n_level * p * n_bins * fsz;
		gh.memset_zero(level_bytes).map_err(|e| {
			drop(Write::err(format!("tb grad_hist zero: {e}")));
			return e;
		})?;
		hh.memset_zero(level_bytes).map_err(|e| {
			drop(Write::err(format!("tb hess_hist zero: {e}")));
			return e;
		})?;
		gpu_tb_histogram(
			train_bins,
			grad,
			hess,
			&node_assign,
			n_train,
			p,
			n_bins,
			level_base,
			&gh,
			&hh,
		)?;
		gpu_tb_split_eval(
			&gh, &hh, lambda, min_cw, n_level, p, n_bins, level_base, &sf, &sb,
		)?;
		gpu_tb_repartition(train_bins, &sf, &sb, n_train, p, &node_assign)?;
	}

	sum_g.memset_zero(max_nodes * fsz).map_err(|e| {
		drop(Write::err(format!("tb node_sum_g zero: {e}")));
		return e;
	})?;
	sum_h.memset_zero(max_nodes * fsz).map_err(|e| {
		drop(Write::err(format!("tb node_sum_h zero: {e}")));
		return e;
	})?;
	lv.memset_zero(max_nodes * fsz).map_err(|e| {
		drop(Write::err(format!("tb leaf_val zero: {e}")));
		return e;
	})?;
	gpu_tb_leaf_sum(grad, hess, &node_assign, n_train, &sum_g, &sum_h)?;
	gpu_tb_leaf_val(&sum_g, &sum_h, lambda, max_nodes, 0, &lv)?;
	gpu_tb_scatter(&node_assign, &lv, n_train, train_pred)?;
	gpu_tb_apply_tree(test_bins, &sf, &sb, &lv, n_test, p, max_depth, test_pred)?;
	return Ok(());
}

/// # Errors
/// Returns `HipError` if the kernel launch fails.
#[inline]
pub fn gpu_mse_grad_into(
	pred: &GpuBuffer,
	target: &GpuBuffer,
	n: usize,
	grad: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pred, target, and grad are valid live device allocations owned by the caller for the duration of this launch.
	unsafe {
		launch_mse_grad(
			pred.ptr.cast_const(),
			target.ptr.cast_const(),
			grad.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_softmax_ce_class_grad_f32(
	class_ptr_buf: &GpuBuffer,
	targets: &GpuBuffer,
	nc: usize,
	k: usize,
	n: usize,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
) -> Result<(), HipError> {
	let k32 = ci(k)?;
	let n32 = ci(n)?;
	let cls32 = ci(nc)?;
	// SAFETY: the four pointers are live GpuBuffer device allocations and the null stream is the valid default stream.
	unsafe {
		launch_softmax_ce_class_grad_f32(
			class_ptr_buf.ptr.cast_const(),
			targets.ptr.cast_const(),
			grad.ptr.cast::<c_void>(),
			hess.ptr.cast::<c_void>(),
			k32,
			n32,
			cls32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_logloss_grad_f32(
	pred: &GpuBuffer,
	target: &GpuBuffer,
	n: usize,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
) -> Result<(), HipError> {
	let n32 = ci(n)?;
	// SAFETY: the four pointers are live GpuBuffer device allocations and the null stream is the valid default stream.
	unsafe {
		launch_logloss_grad_f32(
			pred.ptr.cast_const(),
			target.ptr.cast_const(),
			grad.ptr.cast::<c_void>(),
			hess.ptr.cast::<c_void>(),
			n32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Errors if `n` overflows `i32`.
#[inline]
pub fn gpu_argmax_f32(data: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: HIP kernel launch over live GpuBuffer device pointers; the size arg is range-checked.
	unsafe {
		launch_argmax_f32(
			data.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Errors if `n` overflows `i32`.
#[inline]
pub fn gpu_fill_f32(val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: HIP kernel launch over live GpuBuffer device pointers; the size arg is range-checked.
	unsafe {
		launch_fill_f32(
			out.ptr.cast::<c_void>(),
			val.ptr.cast::<f32>().cast_const(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Errors if a size argument overflows `i32`.
#[inline]
pub fn gpu_argmax_write_split(
	gain: &GpuBuffer,
	n_features: usize,
	n_bins: usize,
	d: usize,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
	best_idx: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: HIP kernel launch over live GpuBuffer device pointers; the size args are range-checked.
	unsafe {
		launch_argmax_write_split(
			gain.ptr.cast_const(),
			split_feat.ptr.cast::<c_void>(),
			split_bin.ptr.cast::<c_void>(),
			best_idx.ptr.cast::<c_void>(),
			ci(n_features)?,
			ci(n_bins)?,
			ci(d)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size argument overflows the launcher's integer type.
#[inline]
pub fn gpu_write_split(
	feat: usize,
	bin: usize,
	d: usize,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: split pointers are valid caller-owned device allocations for this launch.
	unsafe {
		launch_write_split(
			split_feat.ptr.cast::<c_void>(),
			split_bin.ptr.cast::<c_void>(),
			ci(feat)?,
			u8::try_from(bin).map_err(|_e| return HipError(1))?,
			ci(d)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size argument overflows the launcher's integer type.
#[inline]
pub fn gpu_oblivious_histogram(
	bins_fm: &GpuBuffer,
	node_idx: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	n_rows: usize,
	n_features: usize,
	n_bins: usize,
	n_nodes: usize,
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: histogram in/out pointers are valid caller-owned device allocations for this launch.
	unsafe {
		launch_oblivious_histogram(
			bins_fm.ptr.cast_const(),
			node_idx.ptr.cast_const(),
			grad.ptr.cast_const(),
			hess.ptr.cast_const(),
			grad_hist.ptr.cast::<c_void>(),
			hess_hist.ptr.cast::<c_void>(),
			ci(n_rows)?,
			ci(n_features)?,
			ci(n_bins)?,
			ci(n_nodes)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if an integer argument overflows the launch integer type.
#[inline]
pub fn gpu_oblivious_route_step(
	bins_rm: &GpuBuffer,
	node_in: &GpuBuffer,
	split_feat: usize,
	split_bin: usize,
	depth: usize,
	n_rows: usize,
	n_features: usize,
	node_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pointer args reference live GpuBuffers that outlive the kernel launch.
	unsafe {
		launch_oblivious_route_step(
			bins_rm.ptr.cast_const(),
			node_in.ptr.cast_const(),
			node_out.ptr.cast::<c_void>(),
			ci(split_feat)?,
			u8::try_from(split_bin).map_err(|_e| return HipError(1))?,
			ci(depth)?,
			ci(n_rows)?,
			ci(n_features)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if an integer argument overflows the launch integer type.
#[inline]
pub fn gpu_oblivious_route_step_dev(
	bins_rm: &GpuBuffer,
	node_in: &GpuBuffer,
	split_feat_arr: &GpuBuffer,
	split_bin_arr: &GpuBuffer,
	depth: usize,
	n_rows: usize,
	n_features: usize,
	node_out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pointer args reference live GpuBuffers that outlive the kernel launch.
	unsafe {
		launch_oblivious_route_step_dev(
			bins_rm.ptr.cast_const(),
			node_in.ptr.cast_const(),
			node_out.ptr.cast::<c_void>(),
			split_feat_arr.ptr.cast_const(),
			split_bin_arr.ptr.cast_const(),
			ci(depth)?,
			ci(n_rows)?,
			ci(n_features)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_oblivious_route_full(
	bins_rm: &GpuBuffer,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
	n_rows: usize,
	n_features: usize,
	depth: usize,
	leaf_idx: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every argument pointer is a live device allocation valid for this launch.
	unsafe {
		launch_oblivious_route_full(
			bins_rm.ptr.cast_const(),
			split_feat.ptr.cast_const(),
			split_bin.ptr.cast_const(),
			leaf_idx.ptr.cast::<c_void>(),
			ci(n_rows)?,
			ci(n_features)?,
			ci(depth)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_scatter_add_by_leaf(
	leaf_idx: &GpuBuffer,
	leaf_value: &GpuBuffer,
	lr: &GpuBuffer,
	n_rows: usize,
	pred: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every argument pointer is a live device allocation valid for this launch.
	unsafe {
		launch_scatter_add_by_leaf(
			pred.ptr.cast::<c_void>(),
			leaf_idx.ptr.cast_const(),
			leaf_value.ptr.cast_const(),
			lr.ptr.cast::<f32>().cast_const(),
			ci(n_rows)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when `n_rows` overflows `i32`.
#[inline]
pub fn gpu_leaf_reduce(
	leaf_idx: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	n_rows: usize,
	leaf_grad: &GpuBuffer,
	leaf_hess: &GpuBuffer,
) -> Result<(), HipError> {
	let n_rows_i32 = ci(n_rows)?;
	// SAFETY: every buffer pointer is a live device allocation sized for this launch and `n_rows` bounds the kernel's indexing.
	unsafe {
		launch_leaf_reduce(
			leaf_idx.ptr.cast_const(),
			grad.ptr.cast_const(),
			hess.ptr.cast_const(),
			leaf_grad.ptr.cast::<c_void>(),
			leaf_hess.ptr.cast::<c_void>(),
			n_rows_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when `n_leaves` overflows `i32`.
#[inline]
pub fn gpu_leaf_finalize(
	leaf_grad: &GpuBuffer,
	leaf_hess: &GpuBuffer,
	lambda: &GpuBuffer,
	n_leaves: usize,
	leaf_value: &GpuBuffer,
) -> Result<(), HipError> {
	let n_leaves_i32 = ci(n_leaves)?;
	// SAFETY: every buffer pointer is a live device allocation sized for this launch and `n_leaves` bounds the kernel's indexing.
	unsafe {
		launch_leaf_finalize(
			leaf_grad.ptr.cast_const(),
			leaf_hess.ptr.cast_const(),
			leaf_value.ptr.cast::<c_void>(),
			lambda.ptr.cast::<f32>().cast_const(),
			n_leaves_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n_nodes`, `n_features`, or `n_bins` does not fit in [`i32`].
#[inline]
pub fn gpu_oblivious_split_eval(
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
	lambda: &GpuBuffer,
	min_cw: &GpuBuffer,
	n_nodes: usize,
	n_features: usize,
	n_bins: usize,
	gain_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_nodes_i32 = ci(n_nodes)?;
	let n_features_i32 = ci(n_features)?;
	let n_bins_i32 = ci(n_bins)?;
	// SAFETY: `grad_hist`, `hess_hist`, `lambda`, `min_cw`, and `gain_out` are valid device buffers sized for the given dims.
	unsafe {
		launch_oblivious_split_eval(
			grad_hist.ptr.cast_const(),
			hess_hist.ptr.cast_const(),
			gain_out.ptr.cast::<c_void>(),
			n_nodes_i32,
			n_features_i32,
			n_bins_i32,
			lambda.ptr.cast::<f32>(),
			min_cw.ptr.cast::<f32>(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n_rows` or `n_classes` does not fit in [`i32`].
#[inline]
pub fn gpu_softmax_inplace(n_rows: usize, n_classes: usize, x: &GpuBuffer) -> Result<(), HipError> {
	let n_rows_i32 = ci(n_rows)?;
	let n_classes_i32 = ci(n_classes)?;
	// SAFETY: `x` is a valid device buffer of `n_rows * n_classes` elements; the null stream is the default stream.
	unsafe {
		launch_softmax_inplace(
			x.ptr.cast::<c_void>(),
			n_rows_i32,
			n_classes_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n_rows` or `n_classes` does not fit in [`i32`].
#[inline]
pub fn gpu_logloss_grad_mc(
	pred: &GpuBuffer,
	tgt: &GpuBuffer,
	n_rows: usize,
	n_classes: usize,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
) -> Result<(), HipError> {
	let n_rows_i32 = ci(n_rows)?;
	let n_classes_i32 = ci(n_classes)?;
	// SAFETY: `pred`, `tgt`, `grad`, and `hess` are valid device buffers sized for the given dims.
	unsafe {
		launch_logloss_grad_mc(
			pred.ptr.cast_const(),
			tgt.ptr.cast_const(),
			grad.ptr.cast::<c_void>(),
			hess.ptr.cast::<c_void>(),
			n_rows_i32,
			n_classes_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n_rows` or `n_classes` does not fit in [`i32`].
#[inline]
pub fn gpu_accuracy(
	pred: &GpuBuffer,
	tgt: &GpuBuffer,
	n_rows: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_rows_i32 = ci(n_rows)?;
	let n_classes_i32 = ci(n_classes)?;
	// SAFETY: launch_accuracy is an FFI kernel launcher; the buffer pointers are live device allocations and the null stream selects the default queue.
	unsafe {
		launch_accuracy(
			pred.ptr.cast_const(),
			tgt.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_rows_i32,
			n_classes_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n_rows`, `n_classes`, or `col` does not fit in [`i32`].
#[inline]
pub fn gpu_scatter_add_by_leaf_col(
	leaf_idx: &GpuBuffer,
	leaf_value: &GpuBuffer,
	lr: &GpuBuffer,
	n_rows: usize,
	n_classes: usize,
	col: usize,
	pred: &GpuBuffer,
) -> Result<(), HipError> {
	let n_rows_i32 = ci(n_rows)?;
	// SAFETY: launch_scatter_add_by_leaf_col is an FFI kernel launcher; the buffer pointers are live device allocations and the null stream selects the default queue.
	unsafe {
		launch_scatter_add_by_leaf_col(
			pred.ptr.cast::<c_void>(),
			leaf_idx.ptr.cast_const(),
			leaf_value.ptr.cast_const(),
			lr.ptr.cast::<f32>().cast_const(),
			n_rows_i32,
			ci(n_classes)?,
			ci(col)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds [`i32`].
#[inline]
pub fn gpu_add_col_scaled_inplace(
	col: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	cols: usize,
	k: usize,
	matrix: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let cols_i32 = ci(cols)?;
	let k_i32 = ci(k)?;
	// SAFETY: FFI kernel launch; buffer pointers are live and dimensions fit i32.
	unsafe {
		launch_add_col_scaled(
			matrix.ptr.cast::<c_void>(),
			col.ptr.cast_const(),
			n_i32,
			cols_i32,
			k_i32,
			scale.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds [`i32`].
#[inline]
pub fn gpu_sub(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: FFI kernel launch; buffer pointers are live and dimensions fit i32.
	unsafe {
		launch_sub(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds [`i32`].
#[inline]
pub fn gpu_sub_scale_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: a, b, scale, and out are valid device allocations of at least n f64 elements and launch_sub_scale only reads and writes within them.
	unsafe {
		launch_sub_scale(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_i32,
			scale.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_avg_pool_1d(
	input: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let out_len_i32 = ci(out_len)?;
	let n_filters_i32 = ci(n_filters)?;
	// SAFETY: input and out are valid device allocations sized for the pool dims and launch_avg_pool_1d only reads and writes within them.
	unsafe {
		launch_avg_pool_1d(
			input.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_i32,
			out_len_i32,
			n_filters_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_pool_grad_expand(
	grad: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: grad and out are valid device allocations sized for the pool dims and launch_pool_grad_expand only reads and writes within them.
	unsafe {
		launch_pool_grad_expand(
			grad.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_i32,
			ci(out_len)?,
			ci(n_filters)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows [`i32`] or the launch fails.
#[inline]
pub fn gpu_argmin_rows(
	dists: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: dists and out wrap live device allocations valid for the duration of the launch.
	unsafe {
		launch_argmin_rows(
			dists.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if the device transfer or synchronize fails.
#[inline]
pub fn download_assignments(buf: &GpuBuffer, n: usize) -> Result<Vec<i32>, HipError> {
	let mut result = vec![0i32; n];
	let bytes = n * mem::size_of::<i32>();
	// SAFETY: result has room for n i32 values and buf is a live device allocation, so the D2H copy of bytes stays in bounds.
	unsafe {
		memory::xfer(
			result.as_mut_ptr().cast::<c_void>(),
			buf.ptr,
			bytes,
			hip::HIP_MEMCPY_D2H,
			ptr::null_mut(),
		)
	}?;
	hip::device_synchronize()?;
	return Ok(result);
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows [`i32`] or the launch fails.
#[inline]
pub fn gpu_centroid_update(
	x: &GpuBuffer,
	assignments: &GpuBuffer,
	n: usize,
	dim: usize,
	k: usize,
	centroids: &GpuBuffer,
	counts: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: x, assignments, centroids and counts wrap live device allocations valid for the duration of the launch.
	unsafe {
		launch_centroid_update(
			x.ptr.cast_const(),
			assignments.ptr.cast_const(),
			centroids.ptr.cast::<c_void>(),
			counts.ptr.cast::<c_void>(),
			ci(n)?,
			ci(dim)?,
			ci(k)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when a dimension exceeds `i32` range.
#[inline]
pub fn gpu_topk_per_row(
	dists: &GpuBuffer,
	rows: usize,
	cols: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI launch; all buffer pointers are live GPU allocations and dims are range-checked.
	unsafe {
		launch_topk_per_row(
			dists.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(rows)?,
			ci(cols)?,
			ci(k)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` if the device-to-host copy or synchronization fails.
#[inline]
pub fn download_topk_indices(buf: &GpuBuffer, rows: usize, k: usize) -> Result<Vec<i32>, HipError> {
	let n = rows * k;
	let mut result = vec![0i32; n];
	let bytes = n * mem::size_of::<i32>();
	// SAFETY: result holds n i32 slots and buf is a live device allocation of matching size for the D2H copy.
	unsafe {
		memory::xfer(
			result.as_mut_ptr().cast::<c_void>(),
			buf.ptr,
			bytes,
			hip::HIP_MEMCPY_D2H,
			ptr::null_mut(),
		)
	}?;
	hip::device_synchronize()?;
	return Ok(result);
}

/// # Errors
/// Returns `HipError` when a dimension exceeds `i32` range.
#[inline]
pub fn gpu_bias_add(
	x: &GpuBuffer,
	bias: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI launch; all buffer pointers are live GPU allocations and dims are range-checked.
	unsafe {
		launch_bias_add(
			x.ptr.cast_const(),
			bias.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Causal depthwise conv1d + per-channel bias + SiLU, token-major in/out.
/// `x`/`out` are `[t, d_inner]` token-major; `conv_w` is `[d_inner, d_conv]`
/// channel-major (tap-contiguous). The conv is per-channel over time with an
/// implicit `(d_conv-1)` left zero-pad, matching ggml_ssm_conv on a zeroed
/// state prefix. Fused, so the SSM conv step costs one launch and no allocation.
///
/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_ssm_conv_causal_silu(
	x: &GpuBuffer,
	conv_w: &GpuBuffer,
	bias: Option<&GpuBuffer>,
	t: usize,
	d_inner: usize,
	d_conv: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let bias_ptr = bias.map_or(ptr::null(), |b| b.ptr.cast_const());
	// SAFETY: all buffers are live device allocations sized for the passed dims; the launcher only reads/writes within them. A null bias is handled by the kernel (no per-channel add).
	unsafe {
		launch_ssm_conv_causal_silu(
			x.ptr.cast_const(),
			conv_w.ptr.cast_const(),
			bias_ptr,
			out.ptr,
			ci(t)?,
			ci(d_inner)?,
			ci(d_conv)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Fused Mamba-1 selective scan (ggml-cuda/ssm-scan.cu:83-113). One thread per
/// inner channel holds an `N=d_state` private state across the whole `t` loop,
/// folding softplus(dt), exp(dt*A), the B/x accumulate, the C readout and the D
/// skip. `xc`/`dt`/`y` are `[t, d_inner]` token-major; `a` is `[d_inner, d_state]`
/// channel-major used directly (already `-exp(A_log)`); `b`/`c` are `[t, d_state]`
/// single-group; `d` is `[d_inner]`.
///
/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn gpu_ssm_scan_mamba1(
	xc: &GpuBuffer,
	dt: &GpuBuffer,
	a: &GpuBuffer,
	b: &GpuBuffer,
	c: &GpuBuffer,
	d: &GpuBuffer,
	t: usize,
	d_inner: usize,
	d_state: usize,
	y: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffers are live device allocations sized for the passed dims; the launcher only reads/writes within them.
	unsafe {
		launch_ssm_scan_mamba1(
			xc.ptr.cast_const(),
			dt.ptr.cast_const(),
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			c.ptr.cast_const(),
			d.ptr.cast_const(),
			y.ptr,
			ci(t)?,
			ci(d_inner)?,
			ci(d_state)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Fused Mamba-2 grouped selective scan (ggml ssm-scan.cu mamba2 branch). One
/// thread per inner channel holds an `N=d_state` private state across the whole
/// `t` loop. `a`/`d` are per-head scalars `[n_head]` (used directly); `dt` is
/// `[t, n_head]` per-head; `xbc` is the conv+SiLU output `[t, conv_dim]` whose
/// first `d_inner` columns are `x` and whose grouped `B`/`C` blocks (offset
/// `d_inner`) the kernel indexes by group `g = h/(n_head/n_group)`. `y` is
/// `[t, d_inner]` token-major with the `D` skip folded in.
///
/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn gpu_ssm_scan_mamba2(
	xbc: &GpuBuffer,
	dt: &GpuBuffer,
	a: &GpuBuffer,
	d: &GpuBuffer,
	t: usize,
	d_inner: usize,
	d_state: usize,
	n_head: usize,
	n_group: usize,
	conv_dim: usize,
	y: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffers are live device allocations sized for the passed dims; the launcher only reads/writes within them.
	unsafe {
		launch_ssm_scan_mamba2(
			xbc.ptr.cast_const(),
			dt.ptr.cast_const(),
			a.ptr.cast_const(),
			d.ptr.cast_const(),
			y.ptr,
			ci(t)?,
			ci(d_inner)?,
			ci(d_state)?,
			ci(n_head)?,
			ci(n_group)?,
			ci(conv_dim)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Grouped RMSNorm with per-group gamma (Mamba-2 gated norm). The gated `[t,
/// d_inner]` buffer is flattened to `rows = t*n_group` rows of `dpg =
/// d_inner/n_group`; each row RMS-normalizes over `dpg` then scales by
/// `gamma[(row % n_group)*dpg + j]`. `gamma` is `ssm_norm [dpg, n_group]`.
/// In-place safe (`out` may alias `x`).
///
/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_ssm_group_rmsnorm(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	dpg: usize,
	n_group: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffers are live device allocations sized for the passed dims; the launcher only reads/writes within them.
	unsafe {
		launch_ssm_group_rmsnorm(
			x.ptr.cast_const(),
			gamma.ptr.cast_const(),
			eps.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(dpg)?,
			ci(n_group)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Per-row L2 normalize (ggml_l2_norm): `out[row,j] = x[row,j] /
/// max(sqrt(sum_j x^2), eps)`. Distinct from RMSNorm (no `1/cols`, no gamma; eps
/// floors the divisor rather than adding under the root). `rows = t*n_head`,
/// `cols = head_dim`, one block per row. In-place safe.
///
/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_l2norm_rows(
	x: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffers are live device allocations sized for the passed dims; the launcher only reads/writes within them.
	unsafe {
		launch_l2norm_rows(
			x.ptr.cast_const(),
			eps.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Fused gated delta-rule scan (delta-net-base.cpp `build_delta_net_autoregressive`,
/// applied sequentially over the `t` prompt tokens; state starts at zero, no
/// cross-call cache). Per head, per token: decay `S[i,j] *= exp(g*a[h])`, `sk[j]
/// = sum_i S[i,j]k[i]`, `d[j] = beta*(v[j]-sk[j])`, `S[i,j] += k[i]d[j]`, `o[j] =
/// sum_i S[i,j]q[i]` with `q` pre-scaled by `scale`. One workgroup per head, `d`
/// threads; thread `j` holds state column `S[:,j]` privately across the whole `t`
/// loop. `q`/`k`/`v`/`out` are `[t, hv*d]` token-major head-major; `g` is `[t,hv]`
/// for GDA (`per_channel=false`) or `[t,hv*d]` for KDA (`per_channel=true`);
/// `beta` is `[t,hv]`; `a` is `[hv]` (the per-head `-exp(A_log)` decay coefficient).
///
/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn gpu_gated_delta_scan(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	g: &GpuBuffer,
	beta: &GpuBuffer,
	a: &GpuBuffer,
	out: &GpuBuffer,
	t: usize,
	hv: usize,
	d: usize,
	per_channel: bool,
	scale: f64,
) -> Result<(), HipError> {
	// SAFETY: all buffers are live device allocations sized for the passed dims; the launcher only reads/writes within them.
	unsafe {
		launch_gated_delta_scan(
			q.ptr.cast_const(),
			k.ptr.cast_const(),
			v.ptr.cast_const(),
			g.ptr.cast_const(),
			beta.ptr.cast_const(),
			a.ptr.cast_const(),
			out.ptr,
			ci(t)?,
			ci(hv)?,
			ci(d)?,
			i32::from(per_channel),
			scale,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Per-row scalar multiply: `out[r, c] = x[r, c] * s[r]`, one scalar per row.
/// The gated shared-expert token gate (qwen3.5/next) scales each token's expert
/// output by a per-token sigmoid scalar. In-place safe.
///
/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_row_scale(
	x: &GpuBuffer,
	s: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffers are live device allocations sized for the passed dims; the launcher only reads/writes within them.
	unsafe {
		launch_row_scale(
			x.ptr.cast_const(),
			s.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_lstm_cell(
	gates: &GpuBuffer,
	n: usize,
	hs: usize,
	c: &GpuBuffer,
	h: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer is a live device allocation owned by a caller `GpuBuffer` that outlives this launch, and the checked casts bound the dimensions.
	unsafe {
		launch_lstm_cell(
			gates.ptr.cast_const(),
			c.ptr.cast::<c_void>(),
			h.ptr.cast::<c_void>(),
			ci(n)?,
			ci(hs)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_gaussian_ll(
	x: &GpuBuffer,
	means: &GpuBuffer,
	vars: &GpuBuffer,
	log_priors: &GpuBuffer,
	n: usize,
	k: usize,
	p: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer is a live device allocation owned by a caller `GpuBuffer` that outlives this launch, and the checked casts bound the dimensions.
	unsafe {
		launch_gaussian_ll(
			x.ptr.cast_const(),
			means.ptr.cast_const(),
			vars.ptr.cast_const(),
			log_priors.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ci(k)?,
			ci(p)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns [`HipError`] if a dimension argument overflows `i32`.
#[inline]
pub fn gpu_im2col_1d(
	x: &GpuBuffer,
	n: usize,
	p: usize,
	ks: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_len = p - ks + 1;
	// SAFETY: all pointers reference live device allocations from the passed GpuBuffers and every FFI slot type matches the launcher declaration.
	unsafe {
		launch_im2col_1d(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(n)?,
			ci(p)?,
			ci(ks)?,
			ci(out_len)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns [`HipError`] if a dimension argument overflows `i32`.
#[inline]
pub fn gpu_argmax_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all pointers reference live device allocations from the passed GpuBuffers and every FFI slot type matches the launcher declaration.
	unsafe {
		launch_argmax_rows(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `rows` or `cols` exceeds `i32::MAX`.
#[inline]
pub fn gpu_reduce_sum_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI workspace-size query; x.ptr is a live device allocation.
	let ws = unsafe {
		reduce_sum_rows_workspace_bytes(
			x.ptr.cast_const(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		)
	};
	// SAFETY: FFI launch; x/out/workspace are live device allocations for the call.
	unsafe {
		launch_reduce_sum_rows(
			x.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			workspace.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `rows` or `cols` exceeds `i32::MAX`.
#[inline]
pub fn gpu_reduce_mean_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI workspace-size query; x.ptr is a live device allocation.
	let ws = unsafe {
		reduce_mean_cols_workspace_bytes(
			x.ptr.cast_const(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		)
	};
	// SAFETY: FFI launch; x/out/workspace are live device allocations for the call.
	unsafe {
		launch_reduce_mean_cols(
			x.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			workspace.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when `rows` or `cols` overflows `i32`.
#[inline]
pub fn gpu_reduce_var_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i32 = ci(rows)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: x is a live GpuBuffer; the query reads only its pointer and sizes.
	let ws = unsafe {
		reduce_var_cols_workspace_bytes(x.ptr.cast_const(), rows_i32, cols_i32, ptr::null_mut())
	};
	// SAFETY: all buffers are live and sized for rows*cols; ws matches the query.
	unsafe {
		launch_reduce_var_cols(
			x.ptr.cast_const(),
			out.ptr,
			rows_i32,
			cols_i32,
			workspace.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when `nq`, `nt`, or `dim` overflows `i32`.
#[inline]
pub fn gpu_pairwise_l2(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let nq_i32 = ci(nq)?;
	let ntrain_i32 = ci(nt)?;
	let dim_i32 = ci(dim)?;
	// SAFETY: query/train/out are live GpuBuffers sized for nq/nt/dim.
	unsafe {
		launch_pairwise_l2(
			query.ptr.cast_const(),
			train.ptr.cast_const(),
			out.ptr,
			nq_i32,
			ntrain_i32,
			dim_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Partially argsorts `data` (`n` elements), writing the sorted indices to `out`.
///
/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_partial_argsort(
	data: &GpuBuffer,
	n: usize,
	keys_ws: &GpuBuffer,
	vals_ws: &GpuBuffer,
	radix_ws: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `partial_argsort_workspace_bytes` reads only the element count and dereferences no pointers.
	let ws = unsafe { partial_argsort_workspace_bytes(n_i32) };
	// SAFETY: `data`, `out`, `keys_ws`, `vals_ws`, and `radix_ws` are valid device buffers sized for `n`.
	unsafe {
		launch_partial_argsort(
			data.ptr.cast_const(),
			out.ptr,
			keys_ws.ptr,
			vals_ws.ptr,
			radix_ws.ptr,
			ws,
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Downloads `k` [`i32`] indices from device buffer `buf` into a host [`Vec`].
///
/// # Errors
/// Returns [`HipError`] if the device-to-host copy or synchronize fails.
#[inline]
pub fn download_indices(buf: &GpuBuffer, k: usize) -> Result<Vec<i32>, HipError> {
	let mut result = vec![0i32; k];
	let bytes = k * mem::size_of::<i32>();
	// SAFETY: `result` is a host allocation of `bytes`; `buf.ptr` is a live device buffer of at least that size.
	unsafe {
		memory::xfer(
			result.as_mut_ptr().cast::<c_void>(),
			buf.ptr,
			bytes,
			hip::HIP_MEMCPY_D2H,
			ptr::null_mut(),
		)
	}?;
	hip::device_synchronize()?;
	return Ok(result);
}

/// Fills `mask` with a Bernoulli draw of probability `p` over `n` elements.
///
/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`] or `seed` in [`u32`].
#[inline]
pub fn gpu_bernoulli_u8(
	p: &GpuBuffer,
	n: usize,
	seed: usize,
	mask: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let seed_u32 = cu(seed)?;
	// SAFETY: `mask` and `p` are valid device buffers of at least `n` elements.
	unsafe {
		launch_bernoulli_u8(
			mask.ptr.cast::<c_void>(),
			n_i32,
			seed_u32,
			p.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Applies the precomputed dropout `mask` and `scale` to `x`, writing to `out`.
///
/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_dropout_u8_into(
	x: &GpuBuffer,
	mask: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: FFI launch into a HIP kernel; the pointers are live device allocations and the length is range-checked.
	unsafe {
		launch_dropout_u8(
			x.ptr.cast_const(),
			mask.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_i32,
			scale.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Concatenates `a` and `b` column-wise into `out`.
///
/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_concat_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
	rows: usize,
	d1: usize,
	d2: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i32 = ci(rows)?;
	let d1_i32 = ci(d1)?;
	let d2_i32 = ci(d2)?;
	// SAFETY: FFI launch into a HIP kernel; the pointers are live device allocations and the dimensions are range-checked.
	unsafe {
		launch_concat(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr,
			rows_i32,
			d1_i32,
			d2_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Copies the leading `take` columns of each row of `src` into `out`.
///
/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_slice_lead_into(
	src: &GpuBuffer,
	rows: usize,
	src_cols: usize,
	take: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i32 = ci(rows)?;
	let src_cols_i32 = ci(src_cols)?;
	let take_i32 = ci(take)?;
	// SAFETY: FFI launch into a HIP kernel; the pointers are live device allocations and the dimensions are range-checked.
	unsafe {
		launch_slice_lead(
			src.ptr.cast_const(),
			out.ptr,
			rows_i32,
			src_cols_i32,
			take_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if a dimension overflows `i32`.
#[inline]
pub fn gpu_im2col_2d(
	xin: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_h = h - kh + 1;
	let out_w = w - kw + 1;
	// SAFETY: launch_im2col_2d reads xin and writes out only within the passed dims.
	unsafe {
		launch_im2col_2d(
			xin.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(kh)?,
			ci(kw)?,
			ci(out_h)?,
			ci(out_w)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if `n` overflows `i32`.
#[inline]
pub fn gpu_exp(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launch_exp reads x and writes out only for the n elements passed.
	unsafe {
		launch_exp(x.ptr.cast_const(), out.ptr, ci(n)?, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_sqrt(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_sqrt(x.ptr.cast_const(), out.ptr, n_i32, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_neg(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_neg(x.ptr.cast_const(), out.ptr, n_i32, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_sign_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_sign(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_pow(x: &GpuBuffer, p: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x`, `p`, and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_pow(
			x.ptr.cast_const(),
			out.ptr,
			n_i32,
			p.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows [`i32`].
#[inline]
pub fn gpu_clamp_into(
	x: &GpuBuffer,
	lo: &GpuBuffer,
	hi: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: the GpuBuffer pointers are valid device allocations that outlive this launch
	unsafe {
		launch_clamp(
			x.ptr.cast_const(),
			out.ptr.cast::<c_void>(),
			n_i32,
			lo.ptr.cast::<f64>().cast_const(),
			hi.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows [`i32`].
#[inline]
pub fn gpu_transpose(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i32 = ci(rows)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: the GpuBuffer pointers are valid device allocations that outlive this launch
	unsafe {
		launch_transpose(
			x.ptr.cast_const(),
			out.ptr,
			rows_i32,
			cols_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if the kernel launch fails.
#[inline]
pub fn gpu_pack_upper_tri(
	factor: &GpuBuffer,
	m: usize,
	n: usize,
	r: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: the GpuBuffer pointers are valid device allocations that outlive this launch
	unsafe {
		launch_shapex_pack_upper_tri(
			factor.ptr.cast_const(),
			r.ptr,
			safe_i32(m),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows [`i32`].
#[inline]
pub fn gpu_eye(n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: the GpuBuffer pointers are valid device allocations that outlive this launch
	unsafe {
		launch_eye(out.ptr, n_i32, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size argument overflows [`i32`].
#[inline]
pub fn gpu_where_mask(
	cond: &GpuBuffer,
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: cond, a, b, and out are valid device buffers of n elements passed directly to the launcher.
	unsafe {
		launch_where_mask(
			cond.ptr.cast_const(),
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr,
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size argument overflows [`i32`].
#[inline]
pub fn gpu_slice_rows(
	x: &GpuBuffer,
	start: usize,
	count: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let total_rows = x.n_floats().div_euclid(cols);
	if start + count > total_rows {
		drop(Write::err(format!(
			"slice_rows: start({start}) + count({count}) = {} exceeds rows({total_rows})",
			start + count
		)));
		process::abort();
	}
	let start_i32 = ci(start)?;
	let count_i32 = ci(count)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: x and out are valid device buffers; start and count were bounds-checked against total_rows above.
	unsafe {
		launch_slice_rows(
			x.ptr.cast_const(),
			out.ptr,
			start_i32,
			count_i32,
			cols_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size argument overflows [`i32`].
#[inline]
pub fn gpu_broadcast_sub_into(
	x: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: x, v, and out are valid device buffers of n elements passed directly to the launcher.
	unsafe {
		launch_broadcast_sub(
			x.ptr.cast_const(),
			v.ptr.cast_const(),
			out.ptr,
			n_i32,
			cols_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_broadcast_mul(
	x: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: `x`, `v`, and `out` are valid device buffers sized for the broadcast.
	unsafe {
		launch_broadcast_mul(
			x.ptr.cast_const(),
			v.ptr.cast_const(),
			out.ptr,
			n_i32,
			cols_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_broadcast_div(
	x: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: `x`, `v`, and `out` are valid device buffers sized for the broadcast.
	unsafe {
		launch_broadcast_div(
			x.ptr.cast_const(),
			v.ptr.cast_const(),
			out.ptr,
			n_i32,
			cols_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_softmax_backward_into(
	grad: &GpuBuffer,
	sm: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: `grad`, `sm`, and `out` are valid device buffers of `rows` * `cols` elements.
	unsafe {
		launch_softmax_backward(
			grad.ptr.cast_const(),
			sm.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if a dimension overflows `i32`.
#[inline]
pub fn gpu_log_softmax_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launch_log_softmax_rows reads x and writes out only within the passed dims.
	unsafe {
		launch_log_softmax_rows(
			x.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if a dimension overflows `i32`.
#[inline]
pub fn gpu_cross_entropy(
	logits: &GpuBuffer,
	targets: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launch_cross_entropy reads logits and targets and writes out only within the passed dims.
	unsafe {
		launch_cross_entropy(
			logits.ptr.cast_const(),
			targets.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns an error if a dimension overflows `i32`.
#[inline]
pub fn gpu_gather_rows_into(
	table: &GpuBuffer,
	indices: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launch_gather_rows reads table and indices and writes out only within the passed dims.
	unsafe {
		launch_gather_rows(
			table.ptr.cast_const(),
			indices.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` or `cols` overflows `i32`, or the kernel launch fails.
#[inline]
pub fn gpu_scatter_add(
	indices: &GpuBuffer,
	src: &GpuBuffer,
	n: usize,
	cols: usize,
	target: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: launch_scatter_add is an FFI kernel launcher; target/indices/src are live device buffers and the i32 dims are range-checked above.
	unsafe {
		launch_scatter_add(
			target.ptr,
			indices.ptr.cast_const(),
			src.ptr.cast_const(),
			n_i32,
			cols_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n`, `p`, `ks`, or `out_len` overflows `i32`, or the kernel launch fails.
#[inline]
pub fn gpu_col2im_1d(
	patches: &GpuBuffer,
	n: usize,
	p: usize,
	ks: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_len = p - ks + 1;
	out.memset_zero(n * p * 8)?;
	let n_i32 = ci(n)?;
	let p_i32 = ci(p)?;
	let ks_i32 = ci(ks)?;
	let out_len_i32 = ci(out_len)?;
	// SAFETY: launch_col2im_1d is an FFI kernel launcher; patches/out are live device buffers and the i32 dims are range-checked above.
	unsafe {
		launch_col2im_1d(
			patches.ptr.cast_const(),
			out.ptr,
			n_i32,
			p_i32,
			ks_i32,
			out_len_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows [`i32`] or the launch fails.
#[inline]
pub fn gpu_col2im_2d(
	patches: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_h = h - kh + 1;
	let out_w = w - kw + 1;
	out.memset_zero(n * c * h * w * 8)?;
	// SAFETY: buffer pointers are live device allocations and the launcher arg types match the extern decl.
	unsafe {
		launch_col2im_2d(
			patches.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(kh)?,
			ci(kw)?,
			ci(out_h)?,
			ci(out_w)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows [`i32`] or the launch fails.
#[inline]
pub fn gpu_max_pool_1d(
	input: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	vals: &GpuBuffer,
	idx: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: buffer pointers are live device allocations and the launcher arg types match the extern decl.
	unsafe {
		launch_max_pool_1d(
			input.ptr.cast_const(),
			vals.ptr,
			idx.ptr,
			ci(n)?,
			ci(out_len)?,
			ci(n_filters)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds `i32` or a device operation fails.
#[inline]
pub fn gpu_max_pool_1d_backward(
	grad: &GpuBuffer,
	indices: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	out.memset_zero(n * out_len * n_filters * 8)?;
	// SAFETY: grad, indices, and out are valid device buffers and the launch args match the kernel ABI.
	unsafe {
		launch_max_pool_1d_backward(
			grad.ptr.cast_const(),
			indices.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(out_len)?,
			ci(n_filters)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds `i32` or the kernel launch fails.
#[inline]
pub fn gpu_avg_pool_2d(
	input: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_h = (h - kh).div_euclid(sh) + 1;
	let out_w = (w - kw).div_euclid(sw) + 1;
	// SAFETY: input and out are valid device buffers and the launch args match the kernel ABI.
	unsafe {
		launch_avg_pool_2d(
			input.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(kh)?,
			ci(kw)?,
			ci(sh)?,
			ci(sw)?,
			ci(out_h)?,
			ci(out_w)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension exceeds [`i32`] or the kernel launch fails.
#[inline]
pub fn gpu_avg_pool_2d_backward(
	grad: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_h = (h - kh).div_euclid(sh) + 1;
	let out_w = (w - kw).div_euclid(sw) + 1;
	out.memset_zero(n * c * h * w * 8)?;
	// SAFETY: FFI kernel launch; buffer pointers are valid and dims are checked to i32.
	unsafe {
		launch_avg_pool_2d_backward(
			grad.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(kh)?,
			ci(kw)?,
			ci(sh)?,
			ci(sw)?,
			ci(out_h)?,
			ci(out_w)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns `HipError` when a pooling dimension overflows `i32`.
#[inline]
pub fn gpu_max_pool_2d(
	input: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	kh: usize,
	kw: usize,
	sh: usize,
	sw: usize,
	vals: &GpuBuffer,
	idx: &GpuBuffer,
) -> Result<(), HipError> {
	let out_h = (h - kh).div_euclid(sh) + 1;
	let out_w = (w - kw).div_euclid(sw) + 1;
	// SAFETY: FFI launch; all buffer pointers are live GPU allocations and dims are range-checked.
	unsafe {
		launch_max_pool_2d(
			input.ptr.cast_const(),
			vals.ptr,
			idx.ptr,
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(kh)?,
			ci(kw)?,
			ci(sh)?,
			ci(sw)?,
			ci(out_h)?,
			ci(out_w)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if the zeroing memset fails or a dimension exceeds [`i32`].
#[inline]
pub fn gpu_max_pool_2d_backward(
	grad: &GpuBuffer,
	indices: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	out_h: usize,
	out_w: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	out.memset_zero(n * c * h * w * 8)?;
	// SAFETY: grad, indices, and out are live device allocations outliving the launch; sizes are range-checked casts.
	unsafe {
		launch_max_pool_2d_backward(
			grad.ptr.cast_const(),
			indices.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(c)?,
			ci(out_h)?,
			ci(out_w)?,
			ci(h)?,
			ci(w)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `rows`/`cols` exceed `i32` range or the kernel launch fails.
#[inline]
pub fn gpu_reduce_max_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI workspace query; `x` is a live device allocation and the dims are valid.
	let ws = unsafe {
		reduce_max_rows_workspace_bytes(
			x.ptr.cast_const(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		)
	};
	// SAFETY: FFI kernel launch; `x`/`out`/`workspace` are live device allocations sized for the dims.
	unsafe {
		launch_reduce_max_rows(
			x.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			workspace.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `rows`/`cols` exceed `i32` range or the kernel launch fails.
#[inline]
pub fn gpu_reduce_max_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI workspace query; `x` is a live device allocation and the dims are valid.
	let ws = unsafe {
		reduce_max_cols_workspace_bytes(
			x.ptr.cast_const(),
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		)
	};
	// SAFETY: launch_reduce_max_cols is an FFI launcher over live GpuBuffer pointers with range-checked dims and a valid null stream.
	unsafe {
		launch_reduce_max_cols(
			x.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			workspace.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `rows` or `cols` does not fit in [`i32`].
#[inline]
pub fn gpu_reduce_min_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i32 = ci(rows)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: `x.ptr` is a live device buffer; the query only sizes the workspace.
	let ws = unsafe {
		reduce_min_rows_workspace_bytes(x.ptr.cast_const(), rows_i32, cols_i32, ptr::null_mut())
	};
	// SAFETY: all pointers are live device buffers; dims are range-checked i32.
	unsafe {
		launch_reduce_min_rows(
			x.ptr.cast_const(),
			out.ptr,
			rows_i32,
			cols_i32,
			workspace.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `rows` or `cols` does not fit in [`i32`].
#[inline]
pub fn gpu_reduce_min_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i32 = ci(rows)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: `x.ptr` is a live device buffer; the query only sizes the workspace.
	let ws = unsafe {
		reduce_min_cols_workspace_bytes(x.ptr.cast_const(), rows_i32, cols_i32, ptr::null_mut())
	};
	// SAFETY: all pointers are live device buffers; dims are range-checked i32.
	unsafe {
		launch_reduce_min_cols(
			x.ptr.cast_const(),
			out.ptr,
			rows_i32,
			cols_i32,
			workspace.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` overflows `i32`.
#[inline]
pub fn gpu_gt(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launch_gt is an FFI launcher over live GpuBuffer pointers with a range-checked length and a valid null stream.
	unsafe {
		launch_gt(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
/// # Errors
/// Returns [`HipError`] if `n` overflows `i32`.
#[inline]
pub fn gpu_lt(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launch_lt is an FFI launcher over live GpuBuffer pointers with a range-checked length and a valid null stream.
	unsafe {
		launch_lt(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
/// # Errors
/// Returns [`HipError`] if `n` overflows `i32`.
#[inline]
pub fn gpu_eq(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launch_eq is an FFI launcher over live GpuBuffer pointers with a range-checked length and a valid null stream.
	unsafe {
		launch_eq(
			a.ptr.cast_const(),
			b.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
/// # Errors
/// Returns [`HipError`] if `n` overflows `i32`.
#[inline]
pub fn gpu_gt_scalar(
	x: &GpuBuffer,
	val: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: the buffer pointers are valid for this launch and the argument types match the kernel's C ABI.
	unsafe {
		launch_gt_scalar(
			x.ptr.cast_const(),
			out.ptr,
			n_i32,
			val.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
/// # Errors
/// Returns [`HipError`] if `n` overflows `i32`.
#[inline]
pub fn gpu_lt_scalar(
	x: &GpuBuffer,
	val: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: the buffer pointers are valid for this launch and the argument types match the kernel's C ABI.
	unsafe {
		launch_lt_scalar(
			x.ptr.cast_const(),
			out.ptr,
			n_i32,
			val.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` or `c` overflows `i32`.
#[inline]
pub fn gpu_batchnorm_forward(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	beta: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	c: usize,
	out: &GpuBuffer,
	mean: &GpuBuffer,
	inv_std: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let c_i32 = ci(c)?;
	// SAFETY: the buffer pointers are valid for this launch and the argument types match the kernel's C ABI.
	unsafe {
		launch_batchnorm_forward(
			x.ptr.cast_const(),
			gamma.ptr.cast_const(),
			beta.ptr.cast_const(),
			out.ptr,
			mean.ptr,
			inv_std.ptr,
			n_i32,
			c_i32,
			eps.ptr.cast::<f64>(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` or `c` exceeds `i32::MAX`.
#[inline]
pub fn gpu_batchnorm_inference(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	beta: &GpuBuffer,
	run_mean: &GpuBuffer,
	run_var: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	c: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let c_i32 = ci(c)?;
	// SAFETY: launcher reads live GpuBuffer device pointers; n and c are range-checked above.
	unsafe {
		launch_batchnorm_inference(
			x.ptr.cast_const(),
			gamma.ptr.cast_const(),
			beta.ptr.cast_const(),
			run_mean.ptr.cast_const(),
			run_var.ptr.cast_const(),
			out.ptr,
			n_i32,
			c_i32,
			eps.ptr.cast::<f64>(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` or `c` exceeds `i32::MAX`.
#[inline]
pub fn gpu_batchnorm_backward(
	grad_y: &GpuBuffer,
	x: &GpuBuffer,
	save_mean: &GpuBuffer,
	save_inv_std: &GpuBuffer,
	gamma: &GpuBuffer,
	n: usize,
	c: usize,
	grad_x: &GpuBuffer,
	grad_gamma: &GpuBuffer,
	grad_beta: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launcher reads live GpuBuffer device pointers; size args are range-checked casts.
	unsafe {
		launch_batchnorm_backward(
			grad_y.ptr.cast_const(),
			x.ptr.cast_const(),
			save_mean.ptr.cast_const(),
			save_inv_std.ptr.cast_const(),
			gamma.ptr.cast_const(),
			grad_x.ptr,
			grad_gamma.ptr,
			grad_beta.ptr,
			ci(n)?,
			ci(c)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns `HipError` if `t` or `n` overflows `i32`.
#[inline]
pub fn gpu_adam_update(
	grad: &GpuBuffer,
	lr: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	eps: &GpuBuffer,
	t: usize,
	n: usize,
	wgt: &GpuBuffer,
	mom1: &GpuBuffer,
	mom2: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer references a live GpuBuffer allocation and the launcher stays within their extents.
	unsafe {
		launch_adam_update(
			wgt.ptr,
			mom1.ptr,
			mom2.ptr,
			grad.ptr.cast_const(),
			lr.ptr.cast::<f64>(),
			b1.ptr.cast::<f64>(),
			b2.ptr.cast::<f64>(),
			eps.ptr.cast::<f64>(),
			ci(t)?,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns `HipError` if `t` or `n` overflows `i32`.
#[inline]
pub fn gpu_adamw_update(
	grad: &GpuBuffer,
	lr: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	eps: &GpuBuffer,
	wd: &GpuBuffer,
	t: usize,
	n: usize,
	wgt: &GpuBuffer,
	mom1: &GpuBuffer,
	mom2: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every pointer references a live GpuBuffer allocation and the launcher stays within their extents.
	unsafe {
		launch_adamw_update(
			wgt.ptr,
			mom1.ptr,
			mom2.ptr,
			grad.ptr.cast_const(),
			lr.ptr.cast::<f64>(),
			b1.ptr.cast::<f64>(),
			b2.ptr.cast::<f64>(),
			eps.ptr.cast::<f64>(),
			wd.ptr.cast::<f64>(),
			ci(t)?,
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
///
/// Returns `HipError` if `n` or `hs` overflows `i32`.
#[inline]
pub fn gpu_gru_cell(
	gates: &GpuBuffer,
	h: &GpuBuffer,
	n: usize,
	hs: usize,
	h_new: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	let hs_i32 = ci(hs)?;
	// SAFETY: launch_gru_cell reads gates and h and writes h_new, each a valid device buffer for the given dims; the null stream is the default stream.
	unsafe {
		launch_gru_cell(
			gates.ptr.cast_const(),
			h.ptr.cast_const(),
			h_new.ptr,
			n_i32,
			hs_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a device-to-device copy or the device synchronize fails.
#[inline]
pub fn gpu_vconcat(
	a: &GpuBuffer,
	b: &GpuBuffer,
	a_n: usize,
	b_n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let a_bytes = a_n * mem::size_of::<f64>();
	let b_bytes = b_n * mem::size_of::<f64>();
	// SAFETY: memory::xfer copies a_bytes device-to-device from a into out; both are valid device buffers and the null stream is the default stream.
	unsafe {
		memory::xfer(
			out.ptr,
			a.ptr.cast_const(),
			a_bytes,
			hip::HIP_MEMCPY_D2D,
			ptr::null_mut(),
		)
	}?;
	// SAFETY: out is a device buffer holding at least a_bytes + b_bytes; offsetting by a_bytes stays within that allocation.
	let dst = unsafe { out.ptr.cast::<u8>().add(a_bytes) }.cast::<c_void>();
	// SAFETY: memory::xfer copies b_bytes device-to-device from b into out at byte offset a_bytes; both regions are valid and the null stream is the default stream.
	unsafe {
		memory::xfer(
			dst,
			b.ptr.cast_const(),
			b_bytes,
			hip::HIP_MEMCPY_D2D,
			ptr::null_mut(),
		)
	}?;
	hip::device_synchronize()?;
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a slice dimension does not fit in [`i32`].
#[inline]
pub fn gpu_slice_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	start: usize,
	count: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let rows_i32 = ci(rows)?;
	let cols_i32 = ci(cols)?;
	// SAFETY: launch_slice_cols reads x and writes out for the requested column slice; both are valid device buffers and the null stream is the default stream.
	unsafe {
		launch_slice_cols(
			x.ptr.cast_const(),
			out.ptr,
			rows_i32,
			cols_i32,
			ci(start)?,
			ci(count)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_tril_mask(fill_val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launcher reads valid device buffers for the passed extent.
	unsafe {
		launch_tril_mask(out.ptr, ci(n)?, fill_val.ptr.cast::<f64>(), ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_fill(val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: launcher writes a valid device buffer for the passed length.
	unsafe {
		launch_fill(out.ptr, ci(n)?, val.ptr.cast::<f64>(), ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a row count does not fit in [`i32`].
#[inline]
pub fn gpu_repeat_rows(
	src: &GpuBuffer,
	src_n: usize,
	repeats: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let total = src_n * repeats;
	// SAFETY: launcher reads and writes valid device buffers for the passed extents.
	unsafe {
		launch_repeat_rows(
			src.ptr.cast_const(),
			out.ptr,
			ci(src_n)?,
			ci(total)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a dimension does not fit in [`i32`].
#[inline]
pub fn gpu_upsample_nearest_2d(
	input: &GpuBuffer,
	n: usize,
	c: usize,
	h: usize,
	w: usize,
	scale_h: usize,
	scale_w: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launcher reads and writes valid device buffers for the passed extents.
	unsafe {
		launch_upsample_nearest_2d(
			input.ptr.cast_const(),
			out.ptr,
			ci(n)?,
			ci(c)?,
			ci(h)?,
			ci(w)?,
			ci(scale_h)?,
			ci(scale_w)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Computes the log-sum-exp of each row of `x` into `out`.
///
/// # Errors
/// Returns [`HipError`] if `rows` or `cols` does not fit in [`i32`].
#[inline]
pub fn gpu_log_sum_exp_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: `x` and `out` are valid device buffers sized `rows` by `cols`; the null stream is the default stream.
	unsafe {
		launch_log_sum_exp_rows(
			x.ptr.cast_const(),
			out.ptr,
			ci(rows)?,
			ci(cols)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Clips `x` in place so its global L2 norm does not exceed `max_norm`.
///
/// # Errors
/// Returns [`HipError`] if `n` does not fit in [`i32`].
#[inline]
pub fn gpu_grad_clip_norm(
	max_norm: &GpuBuffer,
	n: usize,
	x: &GpuBuffer,
	tmp: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: `x`, `tmp`, and `max_norm` are valid device buffers for `n` elements; the null stream is the default stream.
	unsafe {
		launch_grad_clip_norm(
			x.ptr.cast::<c_void>(),
			tmp.ptr.cast::<c_void>(),
			ci(n)?,
			max_norm.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when `n` overflows `i32`.
#[inline]
pub fn gpu_prefix_sum_inclusive(
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
	tmp: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x.ptr` is a live device buffer; the null workspace pointer only queries the byte count
	let ws = unsafe {
		prefix_sum_inclusive_workspace_bytes(x.ptr.cast_const(), n_i32, ptr::null_mut())
	};
	// SAFETY: all buffers are live device allocations sized for `n` elements
	unsafe {
		launch_prefix_sum_inclusive(
			x.ptr.cast_const(),
			out.ptr,
			n_i32,
			tmp.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when `n` overflows `i32`.
#[inline]
pub fn gpu_prefix_sum_exclusive(
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
	tmp: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x.ptr` is a live device buffer; the null workspace pointer only queries the byte count
	let ws = unsafe {
		prefix_sum_exclusive_workspace_bytes(x.ptr.cast_const(), n_i32, ptr::null_mut())
	};
	// SAFETY: all buffers are live device allocations sized for `n` elements
	unsafe {
		launch_prefix_sum_exclusive(
			x.ptr.cast_const(),
			out.ptr,
			n_i32,
			tmp.ptr,
			ws,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a dimension overflows `i32`.
#[inline]
pub fn gpu_histogram_build(
	bins: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	mask: &GpuBuffer,
	n: usize,
	p: usize,
	n_bins: usize,
	gh: &GpuBuffer,
	hh: &GpuBuffer,
	ch: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: all buffers are live device allocations sized for the histogram dims
	unsafe {
		launch_histogram_build(
			bins.ptr.cast_const(),
			grad.ptr.cast_const(),
			hess.ptr.cast_const(),
			mask.ptr.cast_const(),
			gh.ptr,
			hh.ptr,
			ch.ptr,
			ci(n)?,
			ci(p)?,
			ci(n_bins)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size or dimension overflows `i32`.
#[inline]
pub fn gpu_split_eval(
	gh: &GpuBuffer,
	hh: &GpuBuffer,
	lambda: &GpuBuffer,
	min_child_weight: &GpuBuffer,
	p: usize,
	n_bins: usize,
	bg: &GpuBuffer,
	bb: &GpuBuffer,
) -> Result<(), HipError> {
	let p_i32 = ci(p)?;
	let n_bins_i32 = ci(n_bins)?;
	// SAFETY: every pointer is a live device buffer sized for this launch; the launcher only reads and writes within those bounds.
	unsafe {
		launch_split_eval(
			gh.ptr.cast_const(),
			hh.ptr.cast_const(),
			bg.ptr,
			bb.ptr,
			p_i32,
			n_bins_i32,
			lambda.ptr.cast::<f64>().cast_const(),
			min_child_weight.ptr.cast::<f64>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size or dimension overflows `i32`.
#[inline]
pub fn gpu_data_partition(
	bins: &GpuBuffer,
	mask: &GpuBuffer,
	n: usize,
	p: usize,
	split_feat: usize,
	split_bin: usize,
	left: &GpuBuffer,
	right: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: every pointer is a live device buffer sized for this launch; the launcher only reads and writes within those bounds.
	unsafe {
		launch_data_partition(
			bins.ptr.cast_const(),
			mask.ptr.cast_const(),
			left.ptr,
			right.ptr,
			n_i32,
			ci(p)?,
			ci(split_feat)?,
			ci(split_bin)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a GPU allocation or kernel launch fails.
#[inline]
pub fn gpu_add_col(
	matrix: &GpuBuffer,
	n: usize,
	cols: usize,
	k: usize,
	col: &GpuBuffer,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n * cols)?;
	gpu_copy_into(matrix, n * cols, &out)?;
	let old_col = GpuBuffer::alloc(n)?;
	gpu_slice_cols(&out, n, cols, k, 1, &old_col)?;
	let new_col = GpuBuffer::alloc(n)?;
	gpu_add_into(&old_col, col, n, &new_col)?;
	match k {
		0 => {
			let right = GpuBuffer::alloc(n * (cols - 1))?;
			gpu_slice_cols(&out, n, cols, 1, cols - 1, &right)?;
			let result = GpuBuffer::alloc(n * cols)?;
			gpu_concat_into(&new_col, &right, n, 1, cols - 1, &result)?;
			return Ok(result);
		}
		pos => match pos.cmp(&(cols - 1)) {
			cmp::Ordering::Equal => {
				let left = GpuBuffer::alloc(n * (cols - 1))?;
				gpu_slice_cols(&out, n, cols, 0, cols - 1, &left)?;
				let result = GpuBuffer::alloc(n * cols)?;
				gpu_concat_into(&left, &new_col, n, cols - 1, 1, &result)?;
				return Ok(result);
			}
			cmp::Ordering::Less | cmp::Ordering::Greater => {
				let left = GpuBuffer::alloc(n * k)?;
				gpu_slice_cols(&out, n, cols, 0, k, &left)?;
				let right = GpuBuffer::alloc(n * (cols - k - 1))?;
				gpu_slice_cols(&out, n, cols, k + 1, cols - k - 1, &right)?;
				let tmp = GpuBuffer::alloc(n * (k + 1))?;
				gpu_concat_into(&left, &new_col, n, k, 1, &tmp)?;
				let result = GpuBuffer::alloc(n * cols)?;
				gpu_concat_into(&tmp, &right, n, k + 1, cols - k - 1, &result)?;
				return Ok(result);
			}
		},
	}
}

/// # Errors
/// Returns [`HipError`] if a GPU op, download, or a class index is out of range.
#[inline]
pub fn gpu_report(
	logits: &GpuBuffer,
	val_targets: &[i32],
	n: usize,
	nc: usize,
	round: usize,
) -> Result<f64, HipError> {
	let preds = GpuBuffer::alloc(n)?;
	gpu_argmax_rows(logits, n, nc, &preds)?;
	let mut preds_cpu = vec![0.0f64; n];
	// SAFETY: preds_cpu has length n matching preds' element count; null is the default stream.
	unsafe { preds.download_async(&mut preds_cpu, ptr::null_mut()) }?;
	hip::device_synchronize()?;
	let mut correct = vec![0.0f64; nc];
	let mut total = vec![0.0f64; nc];
	for i in 0..n {
		let c = usize::try_from(val_targets[i]).map_err(|_e| return HipError(1))?;
		total[c] = fadd(total[c], 1.0f64);
		let class_f = f64::from(cu(c)?);
		let hit = match preds_cpu[i].partial_cmp(&class_f) {
			Some(cmp::Ordering::Equal) => 1.0f64,
			Some(cmp::Ordering::Less | cmp::Ordering::Greater) | None => 0.0f64,
		};
		correct[c] = fadd(correct[c], hit);
	}
	let recall_sum: f64 = (0..nc)
		.map(|k| match total[k].partial_cmp(&0.0f64) {
			Some(cmp::Ordering::Greater) => return fdiv(correct[k], total[k]),
			Some(cmp::Ordering::Equal | cmp::Ordering::Less) | None => return 0.0f64,
		})
		.sum::<f64>();
	let ba = fdiv(recall_sum, f64::from(cu(nc)?));
	Write::line(gpu, format!("      r={:4}  val={:.4}", round + 1, ba));
	return Ok(ba);
}

/// # Errors
/// Returns [`HipError`] if a kernel launch or synchronization fails.
#[inline]
pub fn gpu_dtw(cost: &GpuBuffer, m: usize, n: usize, dp: &GpuBuffer) -> Result<(), HipError> {
	let dp_size = ci((m + 1) * (n + 1))?;
	// SAFETY: launch_dtw_init receives a valid device pointer and a range-checked size.
	unsafe {
		launch_dtw_init(dp.ptr, dp_size, ptr::null_mut());
	}
	let diags = m + n - 1;
	let m_i32 = ci(m)?;
	let n_i32 = ci(n)?;
	for d in 0..diags {
		let d_i32 = ci(d)?;
		// SAFETY: launch_dtw_antidiag receives valid device pointers and range-checked dims.
		unsafe {
			launch_dtw_antidiag(
				cost.ptr.cast_const(),
				dp.ptr,
				m_i32,
				n_i32,
				d_i32,
				ptr::null_mut(),
			);
		}
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_itemset_support(
	trans: &GpuBuffer,
	cands: &GpuBuffer,
	n_trans: usize,
	n_items: usize,
	n_cands: usize,
	k: usize,
	counts: &GpuBuffer,
) -> Result<(), HipError> {
	let n_trans_i32 = ci(n_trans)?;
	let n_items_i32 = ci(n_items)?;
	let n_cands_i32 = ci(n_cands)?;
	let k_i32 = ci(k)?;
	// SAFETY: launch_itemset_support receives valid device pointers and range-checked sizes.
	unsafe {
		launch_itemset_support(
			trans.ptr.cast_const(),
			cands.ptr.cast_const(),
			counts.ptr,
			n_trans_i32,
			n_items_i32,
			n_cands_i32,
			k_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_candidate_generate(
	freq: &GpuBuffer,
	n_freq: usize,
	k: usize,
	out: &GpuBuffer,
	n_generated: &GpuBuffer,
) -> Result<(), HipError> {
	n_generated.memset_zero(mem::size_of::<i32>())?;
	let n_freq_i32 = ci(n_freq)?;
	let k_i32 = ci(k)?;
	// SAFETY: freq/out/n_generated are valid device buffers; the launcher reads and writes only within n_freq and k.
	unsafe {
		launch_candidate_generate_write(
			freq.ptr.cast_const(),
			out.ptr,
			n_freq_i32,
			k_i32,
			n_generated.ptr,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a launch argument exceeds the integer range.
#[inline]
pub fn gpu_randn(n: usize, seed: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i = ci(n)?;
	let seed_u = cu(seed)?;
	// SAFETY: out is a valid device buffer written by the launcher for n elements.
	unsafe {
		launch_randn(out.ptr, n_i, seed_u, ptr::null_mut());
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a launch argument exceeds the integer range.
#[inline]
pub fn gpu_lgbm_histogram(
	bins_fm: &GpuBuffer,
	node_idx: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	target_slot: usize,
	n_rows: usize,
	n_eff: usize,
	n_bins: usize,
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
	count_hist: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: bins/node/grad/hess inputs and the three histogram outputs are valid device buffers sized for the launch.
	unsafe {
		launch_lgbm_histogram(
			bins_fm.ptr.cast_const(),
			node_idx.ptr.cast_const(),
			grad.ptr.cast_const(),
			hess.ptr.cast_const(),
			grad_hist.ptr.cast::<c_void>(),
			hess_hist.ptr.cast::<c_void>(),
			count_hist.ptr.cast::<c_void>(),
			safe_i32(target_slot),
			safe_i32(n_rows),
			safe_i32(n_eff),
			safe_i32(n_bins),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a launch argument exceeds the integer range.
#[inline]
pub fn gpu_lgbm_hist_subtract(
	dst_slot: usize,
	src_slot: usize,
	n_eff: usize,
	n_bins: usize,
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
	count_hist: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_lgbm_hist_subtract(
			grad_hist.ptr.cast::<c_void>(),
			hess_hist.ptr.cast::<c_void>(),
			count_hist.ptr.cast::<c_void>(),
			safe_i32(dst_slot),
			safe_i32(src_slot),
			safe_i32(n_eff),
			safe_i32(n_bins),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Picks the best-gain split across evaluated features from the `LightGBM` bin histograms.
///
/// # Errors
/// Returns [`HipError`] if a size does not fit in [`i32`] or the launch fails.
#[inline]
pub fn gpu_lgbm_best_split(
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
	count_hist: &GpuBuffer,
	slot_ids: &GpuBuffer,
	lambda: &GpuBuffer,
	min_child_weight: &GpuBuffer,
	n_eval: usize,
	n_eff: usize,
	n_bins: usize,
	best_gain: &GpuBuffer,
	best_feat: &GpuBuffer,
	best_bin: &GpuBuffer,
	best_left_count: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_lgbm_best_split(
			grad_hist.ptr.cast_const(),
			hess_hist.ptr.cast_const(),
			count_hist.ptr.cast_const(),
			slot_ids.ptr.cast_const(),
			best_gain.ptr.cast::<c_void>(),
			best_feat.ptr.cast::<c_void>(),
			best_bin.ptr.cast::<c_void>(),
			best_left_count.ptr.cast::<c_void>(),
			safe_i32(n_eval),
			safe_i32(n_eff),
			safe_i32(n_bins),
			lambda.ptr.cast::<f32>().cast_const(),
			min_child_weight.ptr.cast::<f32>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// Reduces per-row gradients and hessians into per-leaf sums.
///
/// # Errors
/// Returns [`HipError`] if `n_rows` does not fit in [`i32`] or the launch fails.
#[inline]
pub fn gpu_lgbm_leaf_reduce(
	node_idx: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	n_rows: usize,
	leaf_grad: &GpuBuffer,
	leaf_hess: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI launcher call; buffer pointers stay live for the launch and the counts are valid.
	unsafe {
		launch_lgbm_leaf_reduce(
			node_idx.ptr.cast_const(),
			grad.ptr.cast_const(),
			hess.ptr.cast_const(),
			leaf_grad.ptr.cast::<c_void>(),
			leaf_hess.ptr.cast::<c_void>(),
			safe_i32(n_rows),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_goss_sample(
	sorted_idx: &GpuBuffer,
	uniform_rand: &GpuBuffer,
	sample_rate: &GpuBuffer,
	keep_weight: &GpuBuffer,
	n_rows: usize,
	top_k: usize,
	weights_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_rows_i = ci(n_rows)?;
	let top_k_i = ci(top_k)?;
	// SAFETY: buffer pointers are live device allocations valid for the kernel launch.
	unsafe {
		launch_goss_sample(
			sorted_idx.ptr.cast_const(),
			weights_out.ptr.cast::<c_void>(),
			uniform_rand.ptr.cast_const(),
			n_rows_i,
			top_k_i,
			sample_rate.ptr.cast::<f32>().cast_const(),
			keep_weight.ptr.cast::<f32>().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if a size argument overflows `i32`.
#[inline]
pub fn gpu_leaf_split_apply(
	bins_fm: &GpuBuffer,
	target_leaf: usize,
	new_leaf_left: usize,
	new_leaf_right: usize,
	split_feature: usize,
	split_bin: usize,
	n_rows: usize,
	n_features: usize,
	node_idx: &GpuBuffer,
) -> Result<(), HipError> {
	let target_leaf_i = ci(target_leaf)?;
	let new_leaf_left_i = ci(new_leaf_left)?;
	let new_leaf_right_i = ci(new_leaf_right)?;
	let split_feature_i = ci(split_feature)?;
	// SAFETY: buffer pointers are live device allocations valid for the kernel launch.
	unsafe {
		launch_leaf_split_apply(
			bins_fm.ptr.cast_const(),
			node_idx.ptr.cast::<c_void>(),
			target_leaf_i,
			new_leaf_left_i,
			new_leaf_right_i,
			split_feature_i,
			u8::try_from(split_bin).map_err(|_e| return HipError(1))?,
			ci(n_rows)?,
			ci(n_features)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] if any conv dimension overflows `i32`.
#[inline]
pub fn gpu_conv1d_into(
	xin: &GpuBuffer,
	wgt: &GpuBuffer,
	bias: &GpuBuffer,
	n: usize,
	cin: usize,
	l: usize,
	cout: usize,
	k: usize,
	stride: usize,
	yout: &GpuBuffer,
) -> Result<(), HipError> {
	let lout = (l - k).div_euclid(stride) + 1;
	// SAFETY: launch_convx_conv1d only reads xin/wgt/bias and writes yout, each a live device buffer sized for the passed conv dims.
	unsafe {
		launch_convx_conv1d(
			xin.ptr.cast_const(),
			wgt.ptr.cast_const(),
			bias.ptr.cast_const(),
			yout.ptr.cast::<c_void>(),
			ci(n)?,
			ci(cin)?,
			ci(l)?,
			ci(cout)?,
			ci(k)?,
			ci(lout)?,
			ci(stride)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size or stride argument exceeds [`i32`].
#[inline]
pub fn gpu_conv1d_backward_data_into(
	dy: &GpuBuffer,
	w: &GpuBuffer,
	n: usize,
	cin: usize,
	l: usize,
	cout: usize,
	k: usize,
	stride: usize,
	dx: &GpuBuffer,
) -> Result<(), HipError> {
	let lout = (l - k).div_euclid(stride) + 1;
	// SAFETY: `dy`, `w`, and `dx` are valid device buffers sized for the conv1d extents.
	unsafe {
		launch_convx_conv1d_backward_data(
			dy.ptr.cast_const(),
			w.ptr.cast_const(),
			dx.ptr.cast::<c_void>(),
			ci(n)?,
			ci(cin)?,
			ci(l)?,
			ci(cout)?,
			ci(k)?,
			ci(lout)?,
			ci(stride)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

/// # Errors
/// Returns [`HipError`] when a size or stride argument exceeds [`i32`].
#[inline]
pub fn gpu_conv1d_backward_filter_into(
	dy: &GpuBuffer,
	x: &GpuBuffer,
	temp: &GpuBuffer,
	ws: &GpuBuffer,
	n: usize,
	cin: usize,
	l: usize,
	cout: usize,
	k: usize,
	stride: usize,
	chunks: usize,
	dw: &GpuBuffer,
) -> Result<(), HipError> {
	let lout = (l - k).div_euclid(stride) + 1;
	let fsz = cout * cin * k;
	let n_i = ci(n)?;
	let cin_i = ci(cin)?;
	let l_i = ci(l)?;
	let cout_i = ci(cout)?;
	let k_i = ci(k)?;
	let lout_i = ci(lout)?;
	let stride_i = ci(stride)?;
	let chunks_i = ci(chunks)?;
	// SAFETY: FFI launch reads dy/x and writes temp through live GpuBuffer pointers with range-checked dims.
	unsafe {
		launch_convx_conv1d_backward_filter(
			dy.ptr.cast_const(),
			x.ptr.cast_const(),
			temp.ptr.cast::<c_void>(),
			n_i,
			cin_i,
			l_i,
			cout_i,
			k_i,
			lout_i,
			stride_i,
			chunks_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	gpu_reduce_sum_cols_into(temp, ws, chunks, fsz, dw)?;
	return Ok(());
}

/// Accumulates the conv1d bias gradient into `db` from the upstream gradient `dy`.
///
/// # Errors
/// Returns [`HipError`] if a dimension does not fit the launcher's `i32` params.
#[inline]
pub fn gpu_conv1d_backward_bias_into(
	dy: &GpuBuffer,
	n: usize,
	cout: usize,
	lout: usize,
	db: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: FFI launch reads dy and writes db through live GpuBuffer pointers with range-checked dims.
	unsafe {
		launch_convx_conv1d_backward_bias(
			dy.ptr.cast_const(),
			db.ptr.cast::<c_void>(),
			ci(n)?,
			ci(cout)?,
			ci(lout)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
