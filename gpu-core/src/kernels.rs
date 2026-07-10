
use crate::hip::{HipError, check};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

pub(crate) fn check_launch() {
	crate::callspy::tick(&crate::callspy::LAUNCH);
	crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
	let err = unsafe { crate::hip::hipGetLastError() };
	assert!(err == 0, "HIP kernel launch failed with error code {}", err);
}

pub(crate) fn safe_i32(v: usize) -> i32 {
	assert!(v <= i32::MAX as usize, "size {} overflows i32", v);
	v as i32
}

const HIPBLAS_OP_N: u32 = 111;
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
		handle: *mut c_void, uplo: u32, n: i32, A: *mut f64, lda: i32, lwork: *mut i32,
	) -> i32;
	fn hipsolverDpotrf(
		handle: *mut c_void, uplo: u32, n: i32, A: *mut f64, lda: i32,
		work: *mut f64, lwork: i32, info: *mut i32,
	) -> i32;
	fn hipsolverDgetrf_bufferSize(
		handle: *mut c_void, m: i32, n: i32, A: *mut f64, lda: i32, lwork: *mut i32,
	) -> i32;
	fn hipsolverDgetrf(
		handle: *mut c_void, m: i32, n: i32, A: *mut f64, lda: i32,
		work: *mut f64, lwork: i32, ipiv: *mut i32, info: *mut i32,
	) -> i32;
	fn hipsolverDgetrs_bufferSize(
		handle: *mut c_void, trans: u32, n: i32, nrhs: i32, A: *mut f64, lda: i32,
		ipiv: *mut i32, B: *mut f64, ldb: i32, lwork: *mut i32,
	) -> i32;
	fn hipsolverDgetrs(
		handle: *mut c_void, trans: u32, n: i32, nrhs: i32, A: *mut f64, lda: i32,
		ipiv: *mut i32, B: *mut f64, ldb: i32, work: *mut f64, lwork: i32, info: *mut i32,
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
	fn launch_add_scalar(x: *const c_void, out: *mut c_void, n: i32, s: *const f64, stream: *mut c_void);
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
	fn launch_bernoulli_u8(mask: *mut c_void, n: i32, seed: u32, p: *const f64, stream: *mut c_void);
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
		x: *const c_void, w: *const c_void, bias: *const c_void, y: *mut c_void,
		n: i32, cin: i32, l: i32, cout: i32, k: i32, lout: i32, s: i32,
		stream: *mut c_void,
	);
	fn launch_convx_conv1d_backward_data(
		dy: *const c_void, w: *const c_void, dx: *mut c_void,
		n: i32, cin: i32, l: i32, cout: i32, k: i32, lout: i32, s: i32,
		stream: *mut c_void,
	);
	fn launch_convx_conv1d_backward_filter(
		dy: *const c_void, x: *const c_void, temp: *mut c_void,
		n: i32, cin: i32, l: i32, cout: i32, k: i32, lout: i32, s: i32,
		chunks: i32, stream: *mut c_void,
	);
	fn launch_convx_conv1d_backward_bias(
		dy: *const c_void, db: *mut c_void,
		n: i32, cout: i32, lout: i32,
		stream: *mut c_void,
	);
}

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

thread_local! {
    static HIPBLAS_HANDLE: AtomicPtr<c_void> = const { AtomicPtr::new(std::ptr::null_mut()) };
    static HIPSOLVER_HANDLE: AtomicPtr<c_void> = const { AtomicPtr::new(std::ptr::null_mut()) };
}

static ATEXIT_REGISTERED: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn atexit_gpu_shutdown() {
	crate::callspy::tick(&crate::callspy::DEVICE_SYNCHRONIZE);
	let _sync = unsafe { crate::hip::hipDeviceSynchronize() };
	crate::memory::mark_shutting_down();
	gpu_shutdown();
}

pub(crate) fn hipblas_handle() -> *mut c_void {
	crate::callspy::tick(&crate::callspy::HIPBLAS);
	HIPBLAS_HANDLE.with(|h| {
		let ptr = h.load(Ordering::Relaxed);
		match std::ptr::NonNull::new(ptr) {
			Some(existing) => existing.as_ptr(),
			None => {
				Some(ATEXIT_REGISTERED.swap(1, Ordering::Relaxed))
					.filter(|prev| *prev == 0)
					.map(|_prev| {
						unsafe extern "C" {
							fn atexit(f: unsafe extern "C" fn()) -> i32;
						}
						let _atexit = unsafe { atexit(atexit_gpu_shutdown) };
					})
					.unwrap_or(());
				let mut handle: *mut c_void = std::ptr::null_mut();
				let status = unsafe { hipblasCreate(&mut handle) };
				assert_eq!(
					status, 0,
					"hipblasCreate failed with status {}",
					status
				);
				let status = unsafe { hipblasSetStream(handle, std::ptr::null_mut()) };
				assert_eq!(status, 0, "hipblasSetStream failed with status {}", status);
				h.store(handle, Ordering::Relaxed);
				handle
			}
		}
	})
}

pub fn gpu_blas_workspace(buf: &crate::memory::GpuBuffer) {
	let status = unsafe { hipblasSetWorkspace(hipblas_handle(), buf.ptr_raw(), buf.len()) };
	assert_eq!(status, 0, "hipblasSetWorkspace failed with status {}", status);
}

pub(crate) fn hipsolver_handle() -> *mut c_void {
	HIPSOLVER_HANDLE.with(|h| {
		let ptr = h.load(Ordering::Relaxed);
		match std::ptr::NonNull::new(ptr) {
			Some(existing) => existing.as_ptr(),
			None => {
				let mut handle: *mut c_void = std::ptr::null_mut();
				let status = unsafe { hipsolverCreate(&mut handle) };
				assert_eq!(
					status, 0,
					"hipsolverCreate failed with status {}",
					status
				);
				h.store(handle, Ordering::Relaxed);
				handle
			}
		}
	})
}

pub fn gpu_shutdown() {
	crate::callspy::tick(&crate::callspy::DEVICE_SYNCHRONIZE);
	unsafe { crate::hip::hipDeviceSynchronize() };
	HIPBLAS_HANDLE.with(|h| {
		let ptr = h.swap(std::ptr::null_mut(), Ordering::Relaxed);
		std::ptr::NonNull::new(ptr)
			.map(|existing| unsafe {
				hipblasDestroy(existing.as_ptr());
			})
			.unwrap_or(());
	});
	HIPSOLVER_HANDLE.with(|h| {
		let ptr = h.swap(std::ptr::null_mut(), Ordering::Relaxed);
		std::ptr::NonNull::new(ptr)
			.map(|existing| unsafe {
				hipsolverDestroy(existing.as_ptr());
			})
			.unwrap_or(());
	});
	crate::memory::free_bounce();
	crate::memory::free_run_pin();
	let _trim = crate::hip::trim_mempool(0);
	eprint!("{}", crate::callspy::report());
}

pub fn gpu_gemm(
	a: &GpuBuffer,
	b: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0_f64;
	let beta = 0.0_f64;
	let status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_N,
			HIPBLAS_OP_N,
			n as i32,
			m as i32,
			k as i32,
			&alpha,
			b.ptr as *const f64,
			n as i32,
			a.ptr as *const f64,
			k as i32,
			&beta,
			out.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)
}

pub fn gpu_gemm_at(
	a: &GpuBuffer,
	b: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0_f64;
	let beta = 0.0_f64;
	let status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_N,
			HIPBLAS_OP_T,
			n as i32,
			m as i32,
			k as i32,
			&alpha,
			b.ptr as *const f64,
			n as i32,
			a.ptr as *const f64,
			m as i32,
			&beta,
			out.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)
}

pub fn gpu_gemm_bt_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	c: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0_f64;
	let beta = 0.0_f64;
	let status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_T,
			HIPBLAS_OP_N,
			n as i32,
			m as i32,
			k as i32,
			&alpha,
			b.ptr as *const f64,
			k as i32,
			a.ptr as *const f64,
			k as i32,
			&beta,
			c.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)
}

pub fn gpu_cholesky_solve_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			122,
			n as i32,
			std::ptr::null_mut(),
			n as i32,
			&mut lwork,
		)
	};
	(lwork.max(1) as usize) * 8
}

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
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			122,
			n as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			&mut lwork,
		)
	};
	let status = unsafe {
		hipsolverDpotrf(
			hipsolver_handle(),
			122,
			n as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			work.ptr as *mut f64,
			lwork,
			info.ptr as *mut i32,
		)
	};
	check(status)?;

	let alpha = 1.0_f64;
	let status = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			111,
			131,
			n as i32,
			1,
			&alpha,
			a_copy.ptr as *const f64,
			n as i32,
			out.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)?;

	let status = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			112,
			131,
			n as i32,
			1,
			&alpha,
			a_copy.ptr as *const f64,
			n as i32,
			out.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)
}

pub fn gpu_cholesky_inv_workspace_bytes(n: usize) -> usize {
	gpu_cholesky_solve_workspace_bytes(n)
}

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
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			122,
			n as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			&mut lwork,
		)
	};
	let status = unsafe {
		hipsolverDpotrf(
			hipsolver_handle(),
			122,
			n as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			work.ptr as *mut f64,
			lwork,
			info.ptr as *mut i32,
		)
	};
	check(status)?;

	let alpha = 1.0_f64;
	let status = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			111,
			131,
			n as i32,
			n as i32,
			&alpha,
			a_copy.ptr as *const f64,
			n as i32,
			out.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)?;

	let status = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			122,
			112,
			131,
			n as i32,
			n as i32,
			&alpha,
			a_copy.ptr as *const f64,
			n as i32,
			out.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)
}

pub fn gpu_solve_getrf_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			n as i32,
			n as i32,
			std::ptr::null_mut(),
			n as i32,
			&mut lwork,
		)
	};
	(lwork.max(1) as usize) * 8
}

pub fn gpu_solve_getrs_workspace_bytes(n: usize, nrhs: usize) -> usize {
	let mut lwork_s: i32 = 0;
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			111,
			n as i32,
			nrhs as i32,
			std::ptr::null_mut(),
			n as i32,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			n as i32,
			&mut lwork_s,
		)
	};
	(lwork_s.max(1) as usize) * 8
}

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
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			n as i32,
			n as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			&mut lwork,
		)
	};
	let status = unsafe {
		hipsolverDgetrf(
			hipsolver_handle(),
			n as i32,
			n as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			work.ptr as *mut f64,
			lwork,
			ipiv.ptr as *mut i32,
			info.ptr as *mut i32,
		)
	};
	check(status)?;

	let mut lwork_s: i32 = 0;
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			111,
			n as i32,
			nrhs as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			ipiv.ptr as *mut i32,
			out.ptr as *mut f64,
			n as i32,
			&mut lwork_s,
		)
	};
	let status = unsafe {
		hipsolverDgetrs(
			hipsolver_handle(),
			111,
			n as i32,
			nrhs as i32,
			a_copy.ptr as *mut f64,
			n as i32,
			ipiv.ptr as *mut i32,
			out.ptr as *mut f64,
			n as i32,
			work_s.ptr as *mut f64,
			lwork_s,
			info.ptr as *mut i32,
		)
	};
	check(status)
}

pub fn gpu_cholesky_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			121,
			n as i32,
			std::ptr::null_mut(),
			n as i32,
			&mut lwork,
		)
	};
	(lwork.max(1) as usize) * 8
}

pub fn gpu_cholesky(
	a: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	info: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(a, n * n, out)?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDpotrf_bufferSize(
			hipsolver_handle(),
			121,
			n as i32,
			out.ptr as *mut f64,
			n as i32,
			&mut lwork,
		)
	};
	let status = unsafe {
		hipsolverDpotrf(
			hipsolver_handle(),
			121,
			n as i32,
			out.ptr as *mut f64,
			n as i32,
			work.ptr as *mut f64,
			lwork,
			info.ptr as *mut i32,
		)
	};
	check(status)
}

pub fn gpu_tri_solve(
	l: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
	trans: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(b, n * nrhs, out)?;
	let alpha = 1.0_f64;
	let trans_flag = match trans.cmp(&0) {
		std::cmp::Ordering::Equal => 111u32,
		std::cmp::Ordering::Less | std::cmp::Ordering::Greater => 112u32,
	};
	let status = unsafe {
		hipblasDtrsm(
			hipblas_handle(),
			141,
			121,
			trans_flag,
			131,
			n as i32,
			nrhs as i32,
			&alpha,
			l.ptr as *const f64,
			n as i32,
			out.ptr as *mut f64,
			n as i32,
		)
	};
	check(status)
}

pub fn gpu_add_diag(val: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_add_diag(a.ptr, n as i32, val.ptr as *const f64, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_reparameterize(
	mu: &GpuBuffer,
	log_var: &GpuBuffer,
	eps: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_reparameterize(
			mu.ptr as *const c_void,
			log_var.ptr as *const c_void,
			eps.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_kl_div(
	mu: &GpuBuffer,
	log_var: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_kl_div(
			mu.ptr as *const c_void,
			log_var.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_vae_backward_latent(
			grad_z.ptr as *const c_void,
			mu.ptr as *const c_void,
			log_var.ptr as *const c_void,
			eps.ptr as *const c_void,
			grad_mu.ptr,
			grad_lv.ptr,
			n as i32,
			kl_weight.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_log_det_cholesky(
	l: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_log_det_cholesky(
			l.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_scaled_exp(
	x: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_scaled_exp(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			scale.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sigmoid_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_sigmoid(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sigmoid_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_sigmoid_backward(
			grad.ptr as *const c_void,
			act.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_tanh_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_tanh_act(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
pub fn gpu_tanh_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_tanh_backward(
			grad.ptr as *const c_void,
			act.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_leaky_relu_into(
	x: &GpuBuffer,
	alpha: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_leaky_relu(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			alpha.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
pub fn gpu_leaky_relu_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	alpha: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_leaky_relu_backward(
			grad.ptr as *const c_void,
			act.ptr as *const c_void,
			out.ptr,
			n as i32,
			alpha.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_silu_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_silu(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
pub fn gpu_silu_backward_into(
	grad: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_silu_backward(
			grad.ptr as *const c_void,
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_relu_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_relu(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_relu_backward_into(
	grad: &GpuBuffer,
	act: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_relu_backward(
			grad.ptr as *const c_void,
			act.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_add_into(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_add(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_add_scalar(
	x: &GpuBuffer,
	s: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_add_scalar(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			s.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_div_into(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_div(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_scale_inplace(scalar: &GpuBuffer, n: usize, x: &GpuBuffer) -> Result<(), HipError> {
	crate::infer_ops::gpu_scale_f64_inplace(scalar, n, x)
}

pub fn gpu_fma(
	x: &GpuBuffer,
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_fma(
			x.ptr as *const c_void,
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sgd_update(
	grad: &GpuBuffer,
	neg_lr: &GpuBuffer,
	n: usize,
	weights: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_sgd_update_f64(
			weights.ptr as *mut f64,
			grad.ptr as *const f64,
			neg_lr.ptr as *const f64,
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_mul(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_mul(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_mul_inplace(b: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_mul_inplace(
			a.ptr as *mut c_void,
			b.ptr as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_add_inplace(b: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_add(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			a.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sub_inplace(b: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_sub(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			a.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_add_scalar_inplace(s: &GpuBuffer, n: usize, a: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_add_scalar(
			a.ptr as *const c_void,
			a.ptr as *mut c_void,
			n as i32,
			s.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_linear_into(
	x: &GpuBuffer,
	w: &GpuBuffer,
	bias: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_repeat_rows(
			bias.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			(m * n) as i32,
			std::ptr::null_mut(),
		);
	}
	match n.cmp(&16) {
		std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
			unsafe {
				crate::math_ops::launch_tall_skinny_dgemm(
					x.ptr as *const c_void,
					w.ptr as *const c_void,
					out.ptr as *mut c_void,
					m as i32, n as i32, k as i32,
					std::ptr::null_mut(),
				);
			}
			check_launch();
			Ok(())
		}
		std::cmp::Ordering::Greater => {
			let alpha = 1.0_f64;
			let beta = 1.0_f64;
			let status = unsafe {
				hipblasDgemm(
					hipblas_handle(),
					HIPBLAS_OP_N,
					HIPBLAS_OP_N,
					n as i32,
					m as i32,
					k as i32,
					&alpha,
					w.ptr as *const f64,
					n as i32,
					x.ptr as *const f64,
					k as i32,
					&beta,
					out.ptr as *mut f64,
					n as i32,
				)
			};
			check(status)
		}
	}
}

pub fn gpu_ss_res_into(pred: &GpuBuffer, y: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		let _memset = crate::memory::memset_dev(out.ptr, 0, std::mem::size_of::<f64>(), std::ptr::null_mut());
	}
	unsafe {
		launch_ss_res(
			pred.ptr as *const c_void,
			y.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_mse_into(pred: &GpuBuffer, y: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		let _memset = crate::memory::memset_dev(out.ptr, 0, std::mem::size_of::<f64>(), std::ptr::null_mut());
	}
	unsafe {
		launch_mse(
			pred.ptr as *const c_void,
			y.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_accuracy_into(pred: &GpuBuffer, y: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		let _memset = crate::memory::memset_dev(out.ptr, 0, std::mem::size_of::<f64>(), std::ptr::null_mut());
	}
	unsafe {
		launch_accuracy_metric(
			pred.ptr as *const c_void,
			y.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_bce_grad_into(
	pred: &GpuBuffer,
	y: &GpuBuffer,
	inv_n: &GpuBuffer,
	n: usize,
	da: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bce_grad(
			pred.ptr as *const c_void,
			y.ptr as *const c_void,
			da.ptr,
			n as i32,
			inv_n.ptr as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_argmax_accuracy_into(
	pred: &GpuBuffer,
	y: &GpuBuffer,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		let _memset = crate::memory::memset_dev(out.ptr, 0, std::mem::size_of::<f64>(), std::ptr::null_mut());
	}
	unsafe {
		launch_argmax_acc(
			pred.ptr as *const c_void,
			y.ptr as *const c_void,
			out.ptr,
			n as i32,
			k as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_abs_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_abs(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_log_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_log(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_copy_into(src: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_copy_f64(
			src.ptr as *const c_void,
			out.ptr,
			n as i64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_sum_cols_into(
	x: &GpuBuffer,
	reduce_ws: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_sum_cols_workspace_bytes(
			std::ptr::null(),
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_sum_cols(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			rows as i32,
			cols as i32,
			reduce_ws.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_sum_cols_workspace_bytes(rows: usize, cols: usize) -> usize {
	unsafe {
		reduce_sum_cols_workspace_bytes(
			std::ptr::null(),
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	}
}

pub fn gpu_matvec_bias_into(
	x: &GpuBuffer,
	w: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	in_dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_repeat_rows(
			b.ptr as *const c_void,
			out.ptr as *mut c_void,
			1,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	let alpha = 1.0_f64;
	let beta = 1.0_f64;
	let status = unsafe {
		crate::hip::hipblasDgemv(
			hipblas_handle(),
			HIPBLAS_OP_T,
			in_dim as i32,
			n as i32,
			&alpha,
			x.ptr as *const f64,
			in_dim as i32,
			w.ptr as *const f64,
			1,
			&beta,
			out.ptr as *mut f64,
			1,
		)
	};
	check(status)?;
	Ok(())
}

pub fn gpu_dgemv_into(
	a: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	in_dim: usize,
	trans: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let op = match trans.cmp(&0) {
		std::cmp::Ordering::Equal => HIPBLAS_OP_T,
		std::cmp::Ordering::Less | std::cmp::Ordering::Greater => HIPBLAS_OP_N,
	};
	let alpha = 1.0_f64;
	let beta = 0.0_f64;
	let status = unsafe {
		crate::hip::hipblasDgemv(
			hipblas_handle(),
			op,
			in_dim as i32,
			n as i32,
			&alpha,
			a.ptr as *const f64,
			in_dim as i32,
			x.ptr as *const f64,
			1,
			&beta,
			out.ptr as *mut f64,
			1,
		)
	};
	check(status)?;
	Ok(())
}

pub fn gpu_dger_into(grad: &GpuBuffer, w: &GpuBuffer, n: usize, in_dim: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		let _memset = crate::memory::memset_dev(out.ptr, 0, n * in_dim * std::mem::size_of::<f64>(), std::ptr::null_mut());
	}
	let alpha = 1.0_f64;
	let status = unsafe {
		crate::hip::hipblasDger(
			hipblas_handle(),
			in_dim as i32,
			n as i32,
			&alpha,
			w.ptr as *const f64,
			1,
			grad.ptr as *const f64,
			1,
			out.ptr as *mut f64,
			in_dim as i32,
		)
	};
	check(status)?;
	Ok(())
}

pub fn gpu_layernorm_into(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	beta: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_layernorm(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			gamma.ptr as *const c_void,
			beta.ptr as *const c_void,
			rows as i32,
			cols as i32,
			eps.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_gelu_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gelu(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_gelu_backward_into(grad: &GpuBuffer, x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gelu_backward(
			grad.ptr as *const c_void,
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_dropout_into(
	x: &GpuBuffer,
	mask: &GpuBuffer,
	p: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_dropout(
			x.ptr as *const c_void,
			mask.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			p.ptr as *const f64,
			scale.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_rand_uniform_into(seed: usize, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_rand_uniform(out.ptr as *mut c_void, n as i32, seed as u32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_softmax_ce_grad_into(
	logits: &GpuBuffer,
	targets: &GpuBuffer,
	weights: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	nc: usize,
	grad_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_softmax_ce_grad(
			logits.ptr as *const c_void,
			targets.ptr as *const c_void,
			weights.ptr as *const c_void,
			grad_out.ptr as *mut c_void,
			n as i32,
			nc as i32,
			scale.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_splitk_dw_partials_elems(m: usize, k: usize, n: usize) -> usize {
	crate::math_ops::splitk_dw_partials_elems(m, k, n)
}

pub fn gpu_splitk_dw_into(
	input: &GpuBuffer,
	grad: &GpuBuffer,
	partials: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	grad_w: &GpuBuffer,
) -> Result<(), HipError> {
	let p = crate::math_ops::splitk_dw_p(m, k, n);
	unsafe {
		crate::math_ops::launch_splitk_dw(
			input.ptr as *const c_void,
			grad.ptr as *const c_void,
			partials.ptr as *mut c_void,
			grad_w.ptr as *mut c_void,
			m as i32,
			n as i32,
			k as i32,
			p as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	let ws = unsafe {
		reduce_sum_cols_workspace_bytes(
			grad.ptr as *const c_void,
			m as i32,
			n as i32,
			std::ptr::null_mut(),
		)
	};
	gpu_splitk_dw_into(input, grad, partials, m, n, k, grad_w)?;
	unsafe {
		launch_reduce_sum_cols(
			grad.ptr as *const c_void,
			grad_b.ptr as *mut c_void,
			m as i32,
			n as i32,
			reduce_ws.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	let alpha = 1.0_f64;
	let beta = 0.0_f64;
	gpu_splitk_dw_into(input, grad, partials, m, n, k, grad_w)?;
	let ws = unsafe {
		reduce_sum_cols_workspace_bytes(
			grad.ptr as *const c_void,
			m as i32,
			n as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_sum_cols(
			grad.ptr as *const c_void,
			grad_b.ptr as *mut c_void,
			m as i32,
			n as i32,
			reduce_ws.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	let gi_status = unsafe {
		hipblasDgemm(
			hipblas_handle(),
			HIPBLAS_OP_T,
			HIPBLAS_OP_N,
			k as i32,
			m as i32,
			n as i32,
			&alpha,
			weight.ptr as *const f64,
			n as i32,
			grad.ptr as *const f64,
			n as i32,
			&beta,
			grad_input.ptr as *mut f64,
			k as i32,
		)
	};
	check(gi_status)?;
	Ok(())
}

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
	unsafe {
		launch_layernorm_backward(
			grad_y.ptr as *const c_void,
			x.ptr as *const c_void,
			gamma.ptr as *const c_void,
			grad_x.ptr as *mut c_void,
			grad_gamma.ptr as *mut c_void,
			grad_beta.ptr as *mut c_void,
			rows as i32,
			cols as i32,
			eps.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_softmax_rows_into(x: &GpuBuffer, rows: usize, cols: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_softmax_rows(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_flash_attention_into(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	seq: usize,
	d: usize,
	heads: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_flash_attention_f64(
			q.ptr as *const c_void,
			k.ptr as *const c_void,
			v.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			seq as i32,
			d as i32,
			heads as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_flash_attention_train_into(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	seq: usize,
	d: usize,
	heads: usize,
	out: &GpuBuffer,
	lse: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_flash_attention_f64_train_fwd(
			q.ptr as *const c_void,
			k.ptr as *const c_void,
			v.ptr as *const c_void,
			out.ptr as *mut c_void,
			lse.ptr as *mut c_void,
			n as i32,
			seq as i32,
			d as i32,
			heads as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_flash_attention_backward_into(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
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
	unsafe {
		launch_flash_attention_f64_dsum(
			ctx.ptr as *const c_void,
			dctx.ptr as *const c_void,
			dsum.ptr as *mut c_void,
			n as i32,
			seq as i32,
			d as i32,
			heads as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	unsafe {
		launch_flash_attention_f64_bwd_dq(
			q.ptr as *const c_void,
			k.ptr as *const c_void,
			v.ptr as *const c_void,
			dctx.ptr as *const c_void,
			lse.ptr as *const c_void,
			dsum.ptr as *const c_void,
			dq.ptr as *mut c_void,
			n as i32,
			seq as i32,
			d as i32,
			heads as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	unsafe {
		launch_flash_attention_f64_bwd_dkv(
			q.ptr as *const c_void,
			k.ptr as *const c_void,
			v.ptr as *const c_void,
			dctx.ptr as *const c_void,
			lse.ptr as *const c_void,
			dsum.ptr as *const c_void,
			dk.ptr as *mut c_void,
			dv.ptr as *mut c_void,
			n as i32,
			seq as i32,
			d as i32,
			heads as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_bernoulli_into(p: &GpuBuffer, seed: usize, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_bernoulli(
			out.ptr as *mut c_void,
			n as i32,
			p.ptr as *const f64,
			seed as u32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_grad_hess(
			probs.ptr as *const c_void,
			targets.ptr as *const c_void,
			weights.ptr as *const c_void,
			mask.ptr as *const c_void,
			grad_out.ptr as *mut c_void,
			hess_out.ptr as *mut c_void,
			n as i32,
			nc as i32,
			k as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


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
	unsafe {
		launch_tb_histogram(
			tr_bins.ptr as *const c_void,
			grad.ptr as *const c_void,
			hess.ptr as *const c_void,
			node_assign.ptr as *const c_void,
			grad_hist.ptr,
			hess_hist.ptr,
			n_tr as i32,
			p as i32,
			n_bins as i32,
			level_base as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_tb_split_eval(
			grad_hist.ptr as *const c_void,
			hess_hist.ptr as *const c_void,
			split_feat.ptr,
			split_bin.ptr,
			n_level as i32,
			p as i32,
			n_bins as i32,
			lambda.ptr as *const f64,
			min_cw.ptr as *const f64,
			level_base as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_tb_repartition(
	tr_bins: &GpuBuffer,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
	n_tr: usize,
	p: usize,
	node_assign: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_tb_repartition(
			tr_bins.ptr as *const c_void,
			node_assign.ptr,
			split_feat.ptr as *const c_void,
			split_bin.ptr as *const c_void,
			n_tr as i32,
			p as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_tb_leaf_sum(
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	node_assign: &GpuBuffer,
	n_tr: usize,
	node_sum_g: &GpuBuffer,
	node_sum_h: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_tb_leaf_sum(
			grad.ptr as *const c_void,
			hess.ptr as *const c_void,
			node_assign.ptr as *const c_void,
			node_sum_g.ptr,
			node_sum_h.ptr,
			n_tr as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_tb_leaf_val(
	node_sum_g: &GpuBuffer,
	node_sum_h: &GpuBuffer,
	lambda: &GpuBuffer,
	n_leaves: usize,
	leaf_base: usize,
	leaf_val: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_tb_leaf_val(
			node_sum_g.ptr as *const c_void,
			node_sum_h.ptr as *const c_void,
			leaf_val.ptr,
			n_leaves as i32,
			leaf_base as i32,
			lambda.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_tb_scatter(
	node_assign: &GpuBuffer,
	leaf_val: &GpuBuffer,
	n_tr: usize,
	predictions: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_tb_scatter(
			node_assign.ptr as *const c_void,
			leaf_val.ptr as *const c_void,
			predictions.ptr,
			n_tr as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_tb_apply_tree(
			te_bins.ptr as *const c_void,
			split_feat.ptr as *const c_void,
			split_bin.ptr as *const c_void,
			leaf_val.ptr as *const c_void,
			predictions.ptr,
			n_te as i32,
			p as i32,
			max_depth as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_tree_build_into(
	tr_bins: &GpuBuffer,
	te_bins: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	n_tr: usize,
	n_te: usize,
	p: usize,
	n_bins: usize,
	max_depth: usize,
	lambda: &GpuBuffer,
	min_cw: &GpuBuffer,
	tr_pred: &GpuBuffer,
	te_pred: &GpuBuffer,
) -> Result<(), HipError> {
	let isz = std::mem::size_of::<i32>();
	let fsz = std::mem::size_of::<f64>();
	let max_nodes = (1usize << (max_depth + 1)) - 1;
	let max_level = match max_depth.cmp(&1) {
		std::cmp::Ordering::Less | std::cmp::Ordering::Equal => 1usize,
		std::cmp::Ordering::Greater => 1usize << (max_depth - 1),
	};
	let hist_elems = max_level * p * n_bins;

	let node_assign = GpuBuffer::alloc_bytes(n_tr * isz).expect("tb node_assign");
	node_assign.memset_zero(n_tr * isz).expect("tb node_assign zero");
	let sf = GpuBuffer::alloc_bytes(max_nodes * isz).expect("tb split_feat");
	let sb = GpuBuffer::alloc_bytes(max_nodes * isz).expect("tb split_bin");
	sf.fill_bytes(0xFF, max_nodes * isz)
		.expect("tb split_feat fill");
	sb.fill_bytes(0xFF, max_nodes * isz)
		.expect("tb split_bin fill");
	let gh = GpuBuffer::alloc(hist_elems).expect("tb grad_hist");
	let hh = GpuBuffer::alloc(hist_elems).expect("tb hess_hist");
	let sum_g = GpuBuffer::alloc(max_nodes).expect("tb node_sum_g");
	let sum_h = GpuBuffer::alloc(max_nodes).expect("tb node_sum_h");
	let lv = GpuBuffer::alloc(max_nodes).expect("tb leaf_val");

	for d in 0..max_depth {
		let level_base = (1usize << d) - 1;
		let n_level = 1usize << d;
		let level_bytes = n_level * p * n_bins * fsz;
		gh.memset_zero(level_bytes).expect("tb grad_hist zero");
		hh.memset_zero(level_bytes).expect("tb hess_hist zero");
		gpu_tb_histogram(tr_bins, grad, hess, &node_assign, n_tr, p, n_bins, level_base, &gh, &hh)?;
		gpu_tb_split_eval(&gh, &hh, &lambda, &min_cw, n_level, p, n_bins, level_base, &sf, &sb)?;
		gpu_tb_repartition(tr_bins, &sf, &sb, n_tr, p, &node_assign)?;
	}

	sum_g.memset_zero(max_nodes * fsz)
		.expect("tb node_sum_g zero");
	sum_h.memset_zero(max_nodes * fsz)
		.expect("tb node_sum_h zero");
	lv.memset_zero(max_nodes * fsz).expect("tb leaf_val zero");
	gpu_tb_leaf_sum(grad, hess, &node_assign, n_tr, &sum_g, &sum_h)?;
	gpu_tb_leaf_val(&sum_g, &sum_h, &lambda, max_nodes, 0, &lv)?;
	gpu_tb_scatter(&node_assign, &lv, n_tr, tr_pred)?;
	gpu_tb_apply_tree(te_bins, &sf, &sb, &lv, n_te, p, max_depth, te_pred)?;
	Ok(())
}


pub fn gpu_mse_grad_into(pred: &GpuBuffer, target: &GpuBuffer, n: usize, grad: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_mse_grad(
			pred.ptr as *const c_void,
			target.ptr as *const c_void,
			grad.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_softmax_ce_class_grad_f32(
	class_ptr_buf: &GpuBuffer,
	targets: &GpuBuffer,
	nc: usize,
	k: usize,
	n: usize,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_softmax_ce_class_grad_f32(
			class_ptr_buf.ptr as *const c_void,
			targets.ptr as *const c_void,
			grad.ptr as *mut c_void,
			hess.ptr as *mut c_void,
			k as i32,
			n as i32,
			nc as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_logloss_grad_f32(
	pred: &GpuBuffer,
	target: &GpuBuffer,
	n: usize,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_logloss_grad_f32(
			pred.ptr as *const c_void,
			target.ptr as *const c_void,
			grad.ptr as *mut c_void,
			hess.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_argmax_f32(data: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_argmax_f32(
			data.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_fill_f32(val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_fill_f32(out.ptr as *mut c_void, val.ptr as *const f32, n as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_argmax_write_split(
	gain: &GpuBuffer,
	n_features: usize,
	n_bins: usize,
	d: usize,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
	best_idx: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_argmax_write_split(
			gain.ptr as *const c_void,
			split_feat.ptr as *mut c_void,
			split_bin.ptr as *mut c_void,
			best_idx.ptr as *mut c_void,
			n_features as i32,
			n_bins as i32,
			d as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_write_split(
	feat: usize,
	bin: usize,
	d: usize,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_write_split(
			split_feat.ptr as *mut c_void,
			split_bin.ptr as *mut c_void,
			feat as i32,
			bin as u8,
			d as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_oblivious_histogram(
			bins_fm.ptr as *const c_void,
			node_idx.ptr as *const c_void,
			grad.ptr as *const c_void,
			hess.ptr as *const c_void,
			grad_hist.ptr as *mut c_void,
			hess_hist.ptr as *mut c_void,
			n_rows as i32,
			n_features as i32,
			n_bins as i32,
			n_nodes as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_oblivious_route_step(
			bins_rm.ptr as *const c_void,
			node_in.ptr as *const c_void,
			node_out.ptr as *mut c_void,
			split_feat as i32,
			split_bin as u8,
			depth as i32,
			n_rows as i32,
			n_features as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_oblivious_route_step_dev(
			bins_rm.ptr as *const c_void,
			node_in.ptr as *const c_void,
			node_out.ptr as *mut c_void,
			split_feat_arr.ptr as *const c_void,
			split_bin_arr.ptr as *const c_void,
			depth as i32,
			n_rows as i32,
			n_features as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_oblivious_route_full(
	bins_rm: &GpuBuffer,
	split_feat: &GpuBuffer,
	split_bin: &GpuBuffer,
	n_rows: usize,
	n_features: usize,
	depth: usize,
	leaf_idx: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_oblivious_route_full(
			bins_rm.ptr as *const c_void,
			split_feat.ptr as *const c_void,
			split_bin.ptr as *const c_void,
			leaf_idx.ptr as *mut c_void,
			n_rows as i32,
			n_features as i32,
			depth as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_scatter_add_by_leaf(
	leaf_idx: &GpuBuffer,
	leaf_value: &GpuBuffer,
	lr: &GpuBuffer,
	n_rows: usize,
	pred: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_scatter_add_by_leaf(
			pred.ptr as *mut c_void,
			leaf_idx.ptr as *const c_void,
			leaf_value.ptr as *const c_void,
			lr.ptr as *const f32,
			n_rows as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_leaf_reduce(
	leaf_idx: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	n_rows: usize,
	leaf_grad: &GpuBuffer,
	leaf_hess: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_leaf_reduce(
			leaf_idx.ptr as *const c_void,
			grad.ptr as *const c_void,
			hess.ptr as *const c_void,
			leaf_grad.ptr as *mut c_void,
			leaf_hess.ptr as *mut c_void,
			n_rows as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_leaf_finalize(
	leaf_grad: &GpuBuffer,
	leaf_hess: &GpuBuffer,
	lambda: &GpuBuffer,
	n_leaves: usize,
	leaf_value: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_leaf_finalize(
			leaf_grad.ptr as *const c_void,
			leaf_hess.ptr as *const c_void,
			leaf_value.ptr as *mut c_void,
			lambda.ptr as *const f32,
			n_leaves as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_oblivious_split_eval(
			grad_hist.ptr as *const c_void,
			hess_hist.ptr as *const c_void,
			gain_out.ptr as *mut c_void,
			n_nodes as i32,
			n_features as i32,
			n_bins as i32,
			lambda.ptr as *const f32,
			min_cw.ptr as *const f32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_softmax_inplace(n_rows: usize, n_classes: usize, x: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_softmax_inplace(
			x.ptr as *mut c_void,
			n_rows as i32,
			n_classes as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_logloss_grad_mc(
	pred: &GpuBuffer,
	tgt: &GpuBuffer,
	n_rows: usize,
	n_classes: usize,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_logloss_grad_mc(
			pred.ptr as *const c_void,
			tgt.ptr as *const c_void,
			grad.ptr as *mut c_void,
			hess.ptr as *mut c_void,
			n_rows as i32,
			n_classes as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_accuracy(
	pred: &GpuBuffer,
	tgt: &GpuBuffer,
	n_rows: usize,
	n_classes: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_accuracy(
			pred.ptr as *const c_void,
			tgt.ptr as *const c_void,
			out.ptr as *mut c_void,
			n_rows as i32,
			n_classes as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_scatter_add_by_leaf_col(
	leaf_idx: &GpuBuffer,
	leaf_value: &GpuBuffer,
	lr: &GpuBuffer,
	n_rows: usize,
	n_classes: usize,
	col: usize,
	pred: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_scatter_add_by_leaf_col(
			pred.ptr as *mut c_void,
			leaf_idx.ptr as *const c_void,
			leaf_value.ptr as *const c_void,
			lr.ptr as *const f32,
			n_rows as i32,
			n_classes as i32,
			col as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_add_col_scaled_inplace(
	col: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	cols: usize,
	k: usize,
	matrix: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_add_col_scaled(
			matrix.ptr as *mut c_void,
			col.ptr as *const c_void,
			n as i32,
			cols as i32,
			k as i32,
			scale.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sub(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_sub(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sub_scale_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_sub_scale(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			scale.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_avg_pool_1d(
	input: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_avg_pool_1d(
			input.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			out_len as i32,
			n_filters as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_pool_grad_expand(
	grad: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_pool_grad_expand(
			grad.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			out_len as i32,
			n_filters as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_argmin_rows(dists: &GpuBuffer, rows: usize, cols: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_argmin_rows(
			dists.ptr as *const c_void,
			out.ptr as *mut c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn download_assignments(buf: &GpuBuffer, n: usize) -> Result<Vec<i32>, HipError> {
	let mut result = vec![0i32; n];
	let bytes = n * std::mem::size_of::<i32>();
	unsafe {
		crate::memory::xfer(
			result.as_mut_ptr() as *mut c_void,
			buf.ptr,
			bytes,
			crate::hip::HIP_MEMCPY_D2H,
			std::ptr::null_mut(),
		)
	}?;
	crate::hip::device_synchronize()?;
	Ok(result)
}

pub fn gpu_centroid_update(
	x: &GpuBuffer,
	assignments: &GpuBuffer,
	n: usize,
	dim: usize,
	k: usize,
	centroids: &GpuBuffer,
	counts: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_centroid_update(
			x.ptr as *const c_void,
			assignments.ptr as *const c_void,
			centroids.ptr as *mut c_void,
			counts.ptr as *mut c_void,
			n as i32,
			dim as i32,
			k as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_topk_per_row(
	dists: &GpuBuffer,
	rows: usize,
	cols: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_topk_per_row(
			dists.ptr as *const c_void,
			out.ptr as *mut c_void,
			rows as i32,
			cols as i32,
			k as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn download_topk_indices(buf: &GpuBuffer, rows: usize, k: usize) -> Result<Vec<i32>, HipError> {
	let n = rows * k;
	let mut result = vec![0i32; n];
	let bytes = n * std::mem::size_of::<i32>();
	unsafe {
		crate::memory::xfer(
			result.as_mut_ptr() as *mut c_void,
			buf.ptr,
			bytes,
			crate::hip::HIP_MEMCPY_D2H,
			std::ptr::null_mut(),
		)
	}?;
	crate::hip::device_synchronize()?;
	Ok(result)
}

pub fn gpu_bias_add(
	x: &GpuBuffer,
	bias: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bias_add(
			x.ptr as *const c_void,
			bias.ptr as *const c_void,
			out.ptr as *mut c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_lstm_cell(gates: &GpuBuffer, n: usize, hs: usize, c: &GpuBuffer, h: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_lstm_cell(
			gates.ptr as *const c_void,
			c.ptr as *mut c_void,
			h.ptr as *mut c_void,
			n as i32,
			hs as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_gaussian_ll(
			x.ptr as *const c_void,
			means.ptr as *const c_void,
			vars.ptr as *const c_void,
			log_priors.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			k as i32,
			p as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_im2col_1d(x: &GpuBuffer, n: usize, p: usize, ks: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let out_len = p - ks + 1;
	unsafe {
		launch_im2col_1d(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			p as i32,
			ks as i32,
			out_len as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_argmax_rows(x: &GpuBuffer, rows: usize, cols: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_argmax_rows(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_sum_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_sum_rows_workspace_bytes(
			x.ptr as *const c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_sum_rows(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			workspace.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_mean_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_mean_cols_workspace_bytes(
			x.ptr as *const c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_mean_cols(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			workspace.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_var_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_var_cols_workspace_bytes(
			x.ptr as *const c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_var_cols(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			workspace.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_pairwise_l2(
	query: &GpuBuffer,
	train: &GpuBuffer,
	nq: usize,
	nt: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_pairwise_l2(
			query.ptr as *const c_void,
			train.ptr as *const c_void,
			out.ptr,
			nq as i32,
			nt as i32,
			dim as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_partial_argsort(
	data: &GpuBuffer,
	n: usize,
	keys_ws: &GpuBuffer,
	vals_ws: &GpuBuffer,
	radix_ws: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe { partial_argsort_workspace_bytes(n as i32) };
	unsafe {
		launch_partial_argsort(
			data.ptr as *const c_void,
			out.ptr,
			keys_ws.ptr,
			vals_ws.ptr,
			radix_ws.ptr,
			ws,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn download_indices(buf: &GpuBuffer, k: usize) -> Result<Vec<i32>, HipError> {
	let mut result = vec![0i32; k];
	let bytes = k * std::mem::size_of::<i32>();
	unsafe {
		crate::memory::xfer(
			result.as_mut_ptr() as *mut c_void,
			buf.ptr,
			bytes,
			crate::hip::HIP_MEMCPY_D2H,
			std::ptr::null_mut(),
		)
	}?;
	crate::hip::device_synchronize()?;
	Ok(result)
}


pub fn gpu_bernoulli_u8(
	p: &GpuBuffer,
	n: usize,
	seed: usize,
	mask: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bernoulli_u8(
			mask.ptr as *mut c_void,
			n as i32,
			seed as u32,
			p.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_dropout_u8_into(
	x: &GpuBuffer,
	mask: &GpuBuffer,
	scale: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_dropout_u8(
			x.ptr as *const c_void,
			mask.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			scale.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_concat_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
	rows: usize,
	d1: usize,
	d2: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_concat(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr,
			rows as i32,
			d1 as i32,
			d2 as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_slice_lead_into(
	src: &GpuBuffer,
	rows: usize,
	src_cols: usize,
	take: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_slice_lead(
			src.ptr as *const c_void,
			out.ptr,
			rows as i32,
			src_cols as i32,
			take as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_im2col_2d(
	x: &GpuBuffer,
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
	unsafe {
		launch_im2col_2d(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			kh as i32,
			kw as i32,
			out_h as i32,
			out_w as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_exp(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_exp(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sqrt(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_sqrt(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_neg(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_neg(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sign_into(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_sign(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_pow(
	x: &GpuBuffer,
	p: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_pow(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			p.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_clamp_into(
	x: &GpuBuffer,
	lo: &GpuBuffer,
	hi: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_clamp(
			x.ptr as *const c_void,
			out.ptr as *mut c_void,
			n as i32,
			lo.ptr as *const f64,
			hi.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_transpose(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_transpose(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_pack_upper_tri(
	factor: &GpuBuffer,
	m: usize,
	n: usize,
	r: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_shapex_pack_upper_tri(
			factor.ptr as *const c_void,
			r.ptr,
			safe_i32(m),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_eye(n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_eye(out.ptr, n as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_where_mask(
	cond: &GpuBuffer,
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_where_mask(
			cond.ptr as *const c_void,
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_slice_rows(
	x: &GpuBuffer,
	start: usize,
	count: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let total_rows = x.n_floats() / cols;
	assert!(
		start + count <= total_rows,
		"slice_rows: start({start}) + count({count}) = {} exceeds rows({total_rows})",
		start + count
	);
	unsafe {
		launch_slice_rows(
			x.ptr as *const c_void,
			out.ptr,
			start as i32,
			count as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_broadcast_sub_into(
	x: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_broadcast_sub(
			x.ptr as *const c_void,
			v.ptr as *const c_void,
			out.ptr,
			n as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_broadcast_mul(
	x: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_broadcast_mul(
			x.ptr as *const c_void,
			v.ptr as *const c_void,
			out.ptr,
			n as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_broadcast_div(
	x: &GpuBuffer,
	v: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_broadcast_div(
			x.ptr as *const c_void,
			v.ptr as *const c_void,
			out.ptr,
			n as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_softmax_backward_into(
	grad: &GpuBuffer,
	sm: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_softmax_backward(
			grad.ptr as *const c_void,
			sm.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_log_softmax_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_log_softmax_rows(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_cross_entropy(
	logits: &GpuBuffer,
	targets: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_cross_entropy(
			logits.ptr as *const c_void,
			targets.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_gather_rows_into(
	table: &GpuBuffer,
	indices: &GpuBuffer,
	n: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_gather_rows(
			table.ptr as *const c_void,
			indices.ptr as *const c_void,
			out.ptr,
			n as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_scatter_add(
	indices: &GpuBuffer,
	src: &GpuBuffer,
	n: usize,
	cols: usize,
	target: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_scatter_add(
			target.ptr,
			indices.ptr as *const c_void,
			src.ptr as *const c_void,
			n as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_col2im_1d(
	patches: &GpuBuffer,
	n: usize,
	p: usize,
	ks: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let out_len = p - ks + 1;
	out.memset_zero(n * p * 8)?;
	unsafe {
		launch_col2im_1d(
			patches.ptr as *const c_void,
			out.ptr,
			n as i32,
			p as i32,
			ks as i32,
			out_len as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_col2im_2d(
			patches.ptr as *const c_void,
			out.ptr,
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			kh as i32,
			kw as i32,
			out_h as i32,
			out_w as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_max_pool_1d(
	input: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	vals: &GpuBuffer,
	idx: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_max_pool_1d(
			input.ptr as *const c_void,
			vals.ptr,
			idx.ptr,
			n as i32,
			out_len as i32,
			n_filters as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_max_pool_1d_backward(
	grad: &GpuBuffer,
	indices: &GpuBuffer,
	n: usize,
	out_len: usize,
	n_filters: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	out.memset_zero(n * out_len * n_filters * 8)?;
	unsafe {
		launch_max_pool_1d_backward(
			grad.ptr as *const c_void,
			indices.ptr as *const c_void,
			out.ptr,
			n as i32,
			out_len as i32,
			n_filters as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	let out_h = (h - kh) / sh + 1;
	let out_w = (w - kw) / sw + 1;
	unsafe {
		launch_avg_pool_2d(
			input.ptr as *const c_void,
			out.ptr,
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			kh as i32,
			kw as i32,
			sh as i32,
			sw as i32,
			out_h as i32,
			out_w as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	let out_h = (h - kh) / sh + 1;
	let out_w = (w - kw) / sw + 1;
	out.memset_zero(n * c * h * w * 8)?;
	unsafe {
		launch_avg_pool_2d_backward(
			grad.ptr as *const c_void,
			out.ptr,
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			kh as i32,
			kw as i32,
			sh as i32,
			sw as i32,
			out_h as i32,
			out_w as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	let out_h = (h - kh) / sh + 1;
	let out_w = (w - kw) / sw + 1;
	unsafe {
		launch_max_pool_2d(
			input.ptr as *const c_void,
			vals.ptr,
			idx.ptr,
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			kh as i32,
			kw as i32,
			sh as i32,
			sw as i32,
			out_h as i32,
			out_w as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_max_pool_2d_backward(
			grad.ptr as *const c_void,
			indices.ptr as *const c_void,
			out.ptr,
			n as i32,
			c as i32,
			out_h as i32,
			out_w as i32,
			h as i32,
			w as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_max_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_max_rows_workspace_bytes(
			x.ptr as *const c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_max_rows(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			workspace.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_max_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_max_cols_workspace_bytes(
			x.ptr as *const c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_max_cols(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			workspace.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_min_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_min_rows_workspace_bytes(
			x.ptr as *const c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_min_rows(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			workspace.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_reduce_min_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	workspace: &GpuBuffer,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		reduce_min_cols_workspace_bytes(
			x.ptr as *const c_void,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_reduce_min_cols(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			workspace.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_gt(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gt(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
pub fn gpu_lt(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_lt(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
pub fn gpu_eq(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_eq(
			a.ptr as *const c_void,
			b.ptr as *const c_void,
			out.ptr,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
pub fn gpu_gt_scalar(x: &GpuBuffer, val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gt_scalar(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			val.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
pub fn gpu_lt_scalar(x: &GpuBuffer, val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_lt_scalar(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			val.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}



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
	unsafe {
		launch_batchnorm_forward(
			x.ptr as *const c_void,
			gamma.ptr as *const c_void,
			beta.ptr as *const c_void,
			out.ptr,
			mean.ptr,
			inv_std.ptr,
			n as i32,
			c as i32,
			eps.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_batchnorm_inference(
			x.ptr as *const c_void,
			gamma.ptr as *const c_void,
			beta.ptr as *const c_void,
			run_mean.ptr as *const c_void,
			run_var.ptr as *const c_void,
			out.ptr,
			n as i32,
			c as i32,
			eps.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_batchnorm_backward(
			grad_y.ptr as *const c_void,
			x.ptr as *const c_void,
			save_mean.ptr as *const c_void,
			save_inv_std.ptr as *const c_void,
			gamma.ptr as *const c_void,
			grad_x.ptr,
			grad_gamma.ptr,
			grad_beta.ptr,
			n as i32,
			c as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_adam_update(
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
		launch_adam_update(
			w.ptr,
			m.ptr,
			v.ptr,
			g.ptr as *const c_void,
			lr.ptr as *const f64,
			b1.ptr as *const f64,
			b2.ptr as *const f64,
			eps.ptr as *const f64,
			t as i32,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_adamw_update(
	g: &GpuBuffer,
	lr: &GpuBuffer,
	b1: &GpuBuffer,
	b2: &GpuBuffer,
	eps: &GpuBuffer,
	wd: &GpuBuffer,
	t: usize,
	n: usize,
	w: &GpuBuffer,
	m: &GpuBuffer,
	v: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_adamw_update(
			w.ptr,
			m.ptr,
			v.ptr,
			g.ptr as *const c_void,
			lr.ptr as *const f64,
			b1.ptr as *const f64,
			b2.ptr as *const f64,
			eps.ptr as *const f64,
			wd.ptr as *const f64,
			t as i32,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_gru_cell(
	gates: &GpuBuffer,
	h: &GpuBuffer,
	n: usize,
	hs: usize,
	h_new: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_gru_cell(
			gates.ptr as *const c_void,
			h.ptr as *const c_void,
			h_new.ptr,
			n as i32,
			hs as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_vconcat(
	a: &GpuBuffer,
	b: &GpuBuffer,
	a_n: usize,
	b_n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let a_bytes = a_n * std::mem::size_of::<f64>();
	let b_bytes = b_n * std::mem::size_of::<f64>();
	unsafe {
		crate::memory::xfer(out.ptr, a.ptr as *const c_void, a_bytes, crate::hip::HIP_MEMCPY_D2D, std::ptr::null_mut())
	}?;
	unsafe {
		crate::memory::xfer(
			(out.ptr as *mut u8).add(a_bytes) as *mut c_void,
			b.ptr as *const c_void,
			b_bytes,
			crate::hip::HIP_MEMCPY_D2D,
			std::ptr::null_mut(),
		)
	}?;
	crate::hip::device_synchronize()?;
	Ok(())
}

pub fn gpu_slice_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	start: usize,
	count: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_slice_cols(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			start as i32,
			count as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_tril_mask(fill_val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_tril_mask(out.ptr, n as i32, fill_val.ptr as *const f64, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_fill(val: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_fill(out.ptr, n as i32, val.ptr as *const f64, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_repeat_rows(
	src: &GpuBuffer,
	src_n: usize,
	repeats: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let total = src_n * repeats;
	unsafe {
		launch_repeat_rows(
			src.ptr as *const c_void,
			out.ptr,
			src_n as i32,
			total as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_upsample_nearest_2d(
			input.ptr as *const c_void,
			out.ptr,
			n as i32,
			c as i32,
			h as i32,
			w as i32,
			scale_h as i32,
			scale_w as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_log_sum_exp_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_log_sum_exp_rows(
			x.ptr as *const c_void,
			out.ptr,
			rows as i32,
			cols as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_grad_clip_norm(
	max_norm: &GpuBuffer,
	n: usize,
	x: &GpuBuffer,
	tmp: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_grad_clip_norm(
			x.ptr as *mut c_void,
			tmp.ptr as *mut c_void,
			n as i32,
			max_norm.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_prefix_sum_inclusive(
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
	tmp: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		prefix_sum_inclusive_workspace_bytes(
			x.ptr as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_prefix_sum_inclusive(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			tmp.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_prefix_sum_exclusive(
	x: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
	tmp: &GpuBuffer,
) -> Result<(), HipError> {
	let ws = unsafe {
		prefix_sum_exclusive_workspace_bytes(
			x.ptr as *const c_void,
			n as i32,
			std::ptr::null_mut(),
		)
	};
	unsafe {
		launch_prefix_sum_exclusive(
			x.ptr as *const c_void,
			out.ptr,
			n as i32,
			tmp.ptr,
			ws,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


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
	unsafe {
		launch_histogram_build(
			bins.ptr as *const c_void,
			grad.ptr as *const c_void,
			hess.ptr as *const c_void,
			mask.ptr as *const c_void,
			gh.ptr,
			hh.ptr,
			ch.ptr,
			n as i32,
			p as i32,
			n_bins as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_split_eval(
			gh.ptr as *const c_void,
			hh.ptr as *const c_void,
			bg.ptr,
			bb.ptr,
			p as i32,
			n_bins as i32,
			lambda.ptr as *const f64,
			min_child_weight.ptr as *const f64,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_data_partition(
			bins.ptr as *const c_void,
			mask.ptr as *const c_void,
			left.ptr,
			right.ptr,
			n as i32,
			p as i32,
			split_feat as i32,
			split_bin as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
			Ok(result)
		}
		pos => match pos.cmp(&(cols - 1)) {
			std::cmp::Ordering::Equal => {
				let left = GpuBuffer::alloc(n * (cols - 1))?;
				gpu_slice_cols(&out, n, cols, 0, cols - 1, &left)?;
				let result = GpuBuffer::alloc(n * cols)?;
				gpu_concat_into(&left, &new_col, n, cols - 1, 1, &result)?;
				Ok(result)
			}
			std::cmp::Ordering::Less | std::cmp::Ordering::Greater => {
				let left = GpuBuffer::alloc(n * k)?;
				gpu_slice_cols(&out, n, cols, 0, k, &left)?;
				let right = GpuBuffer::alloc(n * (cols - k - 1))?;
				gpu_slice_cols(&out, n, cols, k + 1, cols - k - 1, &right)?;
				let tmp = GpuBuffer::alloc(n * (k + 1))?;
				gpu_concat_into(&left, &new_col, n, k, 1, &tmp)?;
				let result = GpuBuffer::alloc(n * cols)?;
				gpu_concat_into(&tmp, &right, n, k + 1, cols - k - 1, &result)?;
				Ok(result)
			}
		},
	}
}

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
	unsafe { preds.download_async(&mut preds_cpu, std::ptr::null_mut()) }?;
	crate::hip::device_synchronize()?;
	let mut correct = vec![0.0f64; nc];
	let mut total = vec![0.0f64; nc];
	for i in 0..n {
		let c = val_targets[i] as usize;
		total[c] += 1.0;
		correct[c] += match (preds_cpu[i] as usize).cmp(&c) {
			std::cmp::Ordering::Equal => 1.0,
			std::cmp::Ordering::Less | std::cmp::Ordering::Greater => 0.0,
		};
	}
	let ba: f64 = (0..nc)
		.map(|k| match total[k].partial_cmp(&0.0) {
			Some(std::cmp::Ordering::Greater) => correct[k] / total[k],
			Some(std::cmp::Ordering::Equal)
			| Some(std::cmp::Ordering::Less)
			| None => 0.0,
		})
		.sum::<f64>()
		/ nc as f64;
	eprintln!("      r={:4}  val={:.4}", round + 1, ba);
	Ok(ba)
}


pub fn gpu_dtw(cost: &GpuBuffer, m: usize, n: usize, dp: &GpuBuffer) -> Result<(), HipError> {
	let dp_size = (m + 1) * (n + 1);
	unsafe {
		launch_dtw_init(dp.ptr, dp_size as i32, std::ptr::null_mut());
	}
	for d in 0..(m + n - 1) {
		unsafe {
			launch_dtw_antidiag(
				cost.ptr as *const c_void,
				dp.ptr,
				m as i32,
				n as i32,
				d as i32,
				std::ptr::null_mut(),
			);
		}
	}
	check_launch();
	Ok(())
}


pub fn gpu_itemset_support(
	trans: &GpuBuffer,
	cands: &GpuBuffer,
	n_trans: usize,
	n_items: usize,
	n_cands: usize,
	k: usize,
	counts: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_itemset_support(
			trans.ptr as *const c_void,
			cands.ptr as *const c_void,
			counts.ptr,
			n_trans as i32,
			n_items as i32,
			n_cands as i32,
			k as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_candidate_generate(
	freq: &GpuBuffer,
	n_freq: usize,
	k: usize,
	out: &GpuBuffer,
	n_generated: &GpuBuffer,
) -> Result<(), HipError> {
	n_generated.memset_zero(std::mem::size_of::<i32>())?;
	unsafe {
		launch_candidate_generate_write(
			freq.ptr as *const c_void,
			out.ptr,
			n_freq as i32,
			k as i32,
			n_generated.ptr,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_randn(n: usize, seed: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_randn(out.ptr, n as i32, seed as u32, std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_lgbm_histogram(
			bins_fm.ptr as *const c_void,
			node_idx.ptr as *const c_void,
			grad.ptr as *const c_void,
			hess.ptr as *const c_void,
			grad_hist.ptr as *mut c_void,
			hess_hist.ptr as *mut c_void,
			count_hist.ptr as *mut c_void,
			safe_i32(target_slot),
			safe_i32(n_rows),
			safe_i32(n_eff),
			safe_i32(n_bins),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_lgbm_hist_subtract(
	dst_slot: usize,
	src_slot: usize,
	n_eff: usize,
	n_bins: usize,
	grad_hist: &GpuBuffer,
	hess_hist: &GpuBuffer,
	count_hist: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_lgbm_hist_subtract(
			grad_hist.ptr as *mut c_void,
			hess_hist.ptr as *mut c_void,
			count_hist.ptr as *mut c_void,
			safe_i32(dst_slot),
			safe_i32(src_slot),
			safe_i32(n_eff),
			safe_i32(n_bins),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_lgbm_best_split(
			grad_hist.ptr as *const c_void,
			hess_hist.ptr as *const c_void,
			count_hist.ptr as *const c_void,
			slot_ids.ptr as *const c_void,
			best_gain.ptr as *mut c_void,
			best_feat.ptr as *mut c_void,
			best_bin.ptr as *mut c_void,
			best_left_count.ptr as *mut c_void,
			safe_i32(n_eval),
			safe_i32(n_eff),
			safe_i32(n_bins),
			lambda.ptr as *const f32,
			min_child_weight.ptr as *const f32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_lgbm_leaf_reduce(
	node_idx: &GpuBuffer,
	grad: &GpuBuffer,
	hess: &GpuBuffer,
	n_rows: usize,
	leaf_grad: &GpuBuffer,
	leaf_hess: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_lgbm_leaf_reduce(
			node_idx.ptr as *const c_void,
			grad.ptr as *const c_void,
			hess.ptr as *const c_void,
			leaf_grad.ptr as *mut c_void,
			leaf_hess.ptr as *mut c_void,
			safe_i32(n_rows),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_goss_sample(
	sorted_idx: &GpuBuffer,
	uniform_rand: &GpuBuffer,
	sample_rate: &GpuBuffer,
	keep_weight: &GpuBuffer,
	n_rows: usize,
	top_k: usize,
	weights_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_goss_sample(
			sorted_idx.ptr as *const c_void,
			weights_out.ptr as *mut c_void,
			uniform_rand.ptr as *const c_void,
			n_rows as i32,
			top_k as i32,
			sample_rate.ptr as *const f32,
			keep_weight.ptr as *const f32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

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
	unsafe {
		launch_leaf_split_apply(
			bins_fm.ptr as *const c_void,
			node_idx.ptr as *mut c_void,
			target_leaf as i32,
			new_leaf_left as i32,
			new_leaf_right as i32,
			split_feature as i32,
			split_bin as u8,
			n_rows as i32,
			n_features as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_conv1d_into(
	x: &GpuBuffer, w: &GpuBuffer, bias: &GpuBuffer,
	n: usize, cin: usize, l: usize, cout: usize, k: usize, stride: usize,
	y: &GpuBuffer,
) -> Result<(), HipError> {
	let lout = (l - k) / stride + 1;
	unsafe {
		launch_convx_conv1d(
			x.ptr as *const c_void, w.ptr as *const c_void,
			bias.ptr as *const c_void, y.ptr as *mut c_void,
			n as i32, cin as i32, l as i32, cout as i32, k as i32, lout as i32, stride as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_conv1d_backward_data_into(
	dy: &GpuBuffer, w: &GpuBuffer,
	n: usize, cin: usize, l: usize, cout: usize, k: usize, stride: usize,
	dx: &GpuBuffer,
) -> Result<(), HipError> {
	let lout = (l - k) / stride + 1;
	unsafe {
		launch_convx_conv1d_backward_data(
			dy.ptr as *const c_void, w.ptr as *const c_void, dx.ptr as *mut c_void,
			n as i32, cin as i32, l as i32, cout as i32, k as i32, lout as i32, stride as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_conv1d_backward_filter_into(
	dy: &GpuBuffer, x: &GpuBuffer,
	temp: &GpuBuffer, ws: &GpuBuffer,
	n: usize, cin: usize, l: usize, cout: usize, k: usize, stride: usize,
	chunks: usize,
	dw: &GpuBuffer,
) -> Result<(), HipError> {
	let lout = (l - k) / stride + 1;
	let fsz = cout * cin * k;
	unsafe {
		launch_convx_conv1d_backward_filter(
			dy.ptr as *const c_void, x.ptr as *const c_void, temp.ptr as *mut c_void,
			n as i32, cin as i32, l as i32, cout as i32, k as i32, lout as i32, stride as i32,
			chunks as i32, std::ptr::null_mut(),
		);
	}
	check_launch();
	gpu_reduce_sum_cols_into(temp, ws, chunks, fsz, dw)?;
	Ok(())
}

pub fn gpu_conv1d_backward_bias_into(
	dy: &GpuBuffer,
	n: usize, cout: usize, lout: usize,
	db: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_convx_conv1d_backward_bias(
			dy.ptr as *const c_void, db.ptr as *mut c_void,
			n as i32, cout as i32, lout as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}
