use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::{HipError, check};

#[cfg(debug_assertions)]
fn check_launch() {
      let err = unsafe { crate::hip::hipGetLastError() };
      assert!(err == 0, "HIP kernel launch failed with error code {}", err);
}
#[cfg(not(debug_assertions))]
fn check_launch() {}

fn safe_i32(v: usize) -> i32 {
      assert!(v <= i32::MAX as usize, "size {} overflows i32", v);
      v as i32
}

// rocBLAS operation constants
const ROCBLAS_OPERATION_NONE: u32 = 111;
const ROCBLAS_OPERATION_TRANSPOSE: u32 = 112;

unsafe extern "C" {
    // rocBLAS handle management
    fn rocblas_create_handle(handle: *mut *mut c_void) -> i32;
    fn rocblas_destroy_handle(handle: *mut c_void) -> i32;
    fn hipDeviceReset() -> i32;

    // rocBLAS GEMM: column-major C = alpha * op(A) * op(B) + beta * C
    fn rocblas_dgemm(
        handle: *mut c_void,
        transA: u32,
        transB: u32,
        m: i32, n: i32, k: i32,
        alpha: *const f64,
        A: *const f64, lda: i32,
        B: *const f64, ldb: i32,
        beta: *const f64,
        C: *mut f64, ldc: i32,
    ) -> i32;

    // rocBLAS daxpy: y = alpha * x + y
    fn rocblas_daxpy(
        handle: *mut c_void,
        n: i32,
        alpha: *const f64,
        x: *const f64, incx: i32,
        y: *mut f64, incy: i32,
    ) -> i32;

    // rocBLAS dscal: x = alpha * x (in-place)
    fn rocblas_dscal(
        handle: *mut c_void,
        n: i32,
        alpha: *const f64,
        x: *mut f64, incx: i32,
    ) -> i32;

    // HIP memcpy for the copy needed in gpu_scale
    fn hipMemcpy(dst: *mut c_void, src: *const c_void, size: usize, kind: i32) -> i32;

    // rocsolver Cholesky factorization: A = L L^T (in-place, lower triangle)
    fn rocsolver_dpotrf(
        handle: *mut c_void,
        uplo: u32,  // 121 = lower
        n: i32,
        A: *mut f64, lda: i32,
        info: *mut i32,
    ) -> i32;

    // rocsolver LU solve: solve A*X = B via LU pivoting (in-place, overwrites A and B)
    fn rocsolver_dgesv(
        handle: *mut c_void,
        n: i32, nrhs: i32,
        A: *mut f64, lda: i32,
        ipiv: *mut i32,
        B: *mut f64, ldb: i32,
        info: *mut i32,
    ) -> i32;

    // rocBLAS triangular solve: op(A) * X = alpha * B (in-place, overwrites B)
    fn rocblas_dtrsm(
        handle: *mut c_void,
        side: u32,   // 141 = left
        uplo: u32,   // 121 = lower, 122 = upper
        transA: u32, // 111 = none, 112 = transpose
        diag: u32,   // 131 = non-unit, 132 = unit
        m: i32, n: i32,
        alpha: *const f64,
        A: *const f64, lda: i32,
        B: *mut f64, ldb: i32,
    ) -> i32;

    // Remaining custom kernels
    fn launch_add_diag(A: *mut c_void, n: i32, val: f64, stream: *mut c_void);
    fn launch_reparameterize(mu: *const c_void, log_var: *const c_void, eps: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_kl_div(mu: *const c_void, log_var: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_vae_backward_latent(grad_z: *const c_void, mu: *const c_void, log_var: *const c_void, eps: *const c_void, grad_mu_out: *mut c_void, grad_lv_out: *mut c_void, n: i32, kl_weight: f64, stream: *mut c_void);
    fn launch_log_det_cholesky(L: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_scaled_exp(x: *const c_void, out: *mut c_void, n: i32, scale: f64, stream: *mut c_void);
    fn launch_sigmoid(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_sigmoid_backward(grad: *const c_void, act: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_tanh_act(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_tanh_backward(grad: *const c_void, act: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_relu(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_relu_backward(grad: *const c_void, act: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_add(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_add_scalar(x: *const c_void, out: *mut c_void, n: i32, s: f64, stream: *mut c_void);
    fn launch_div(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_fma(x: *const c_void, a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_reduce_sum_cols(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_reduce_sum_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_reduce_mean_cols(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_reduce_var_cols(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_pairwise_l2(query: *const c_void, train: *const c_void, out: *mut c_void, nq: i32, nt: i32, dim: i32, stream: *mut c_void);
    fn launch_partial_argsort(data: *const c_void, indices: *mut c_void, n: i32, k: i32, stream: *mut c_void);
    fn launch_bias_add(x: *const c_void, bias: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_lstm_cell(gates: *const c_void, c: *mut c_void, h: *mut c_void, n: i32, hs: i32, stream: *mut c_void);
    fn launch_gaussian_ll(x: *const c_void, means: *const c_void, vars: *const c_void, log_priors: *const c_void, out: *mut c_void, n: i32, k: i32, p: i32, stream: *mut c_void);
    fn launch_im2col_1d(x: *const c_void, patches: *mut c_void, n: i32, p: i32, ks: i32, out_len: i32, stream: *mut c_void);
    fn launch_argmax_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_mul(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_mul_inplace(a: *mut c_void, b: *const c_void, n: i32, stream: *mut c_void);
    fn launch_add_col_scaled(matrix: *mut c_void, col: *const c_void, n: i32, cols: i32, k: i32, scale: f64, stream: *mut c_void);
    fn launch_grad_hess(probs: *const c_void, targets: *const c_void, weights: *const c_void, mask: *const c_void, grad_out: *mut c_void, hess_out: *mut c_void, n: i32, nc: i32, k: i32, stream: *mut c_void);
    fn launch_softmax_ce_grad(logits: *const c_void, targets: *const c_void, weights: *const c_void, grad_out: *mut c_void, n: i32, nc: i32, scale: f64, stream: *mut c_void);
    fn launch_sub(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_softmax_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_sub_scale(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, scale: f64, stream: *mut c_void);
    fn launch_avg_pool_1d(input: *const c_void, output: *mut c_void, n: i32, out_len: i32, n_filters: i32, stream: *mut c_void);
    fn launch_pool_grad_expand(grad_pool: *const c_void, grad_out: *mut c_void, n: i32, out_len: i32, n_filters: i32, stream: *mut c_void);
    fn launch_argmin_rows(dists: *const c_void, assignments: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_centroid_update(x: *const c_void, assignments: *const c_void, centroids: *mut c_void, counts: *mut c_void, n: i32, dim: i32, k: i32, stream: *mut c_void);
    fn launch_topk_per_row(dists: *const c_void, out_indices: *mut c_void, rows: i32, cols: i32, k: i32, stream: *mut c_void);
    fn launch_leaky_relu(x: *const c_void, out: *mut c_void, n: i32, alpha: f64, stream: *mut c_void);
    fn launch_leaky_relu_backward(grad: *const c_void, act: *const c_void, out: *mut c_void, n: i32, alpha: f64, stream: *mut c_void);
    fn launch_layernorm(x: *const c_void, out: *mut c_void, gamma: *const c_void, beta: *const c_void, rows: i32, cols: i32, eps: f64, stream: *mut c_void);
    fn launch_dropout(x: *const c_void, mask: *const c_void, out: *mut c_void, n: i32, p: f64, scale: f64, stream: *mut c_void);
    fn launch_bernoulli_u8(mask: *mut c_void, n: i32, seed: u32, p: f64, stream: *mut c_void);
    fn launch_dropout_u8(x: *const c_void, mask: *const c_void, out: *mut c_void, n: i32, scale: f64, stream: *mut c_void);
    fn launch_concat(a: *const c_void, b: *const c_void, out: *mut c_void, rows: i32, d1: i32, d2: i32, stream: *mut c_void);
    fn launch_im2col_2d(x: *const c_void, patches: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, out_h: i32, out_w: i32, stream: *mut c_void);
    fn launch_exp(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_log(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_sqrt(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_abs(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_neg(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_sign(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_pow(x: *const c_void, out: *mut c_void, n: i32, p: f64, stream: *mut c_void);
    fn launch_clamp(x: *const c_void, out: *mut c_void, n: i32, lo: f64, hi: f64, stream: *mut c_void);
    fn launch_transpose(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_eye(out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_where_mask(cond: *const c_void, a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_slice_rows(src: *const c_void, dst: *mut c_void, start_row: i32, count: i32, cols: i32, stream: *mut c_void);
    fn launch_broadcast_sub(x: *const c_void, v: *const c_void, out: *mut c_void, n: i32, cols: i32, stream: *mut c_void);
    fn launch_broadcast_mul(x: *const c_void, v: *const c_void, out: *mut c_void, n: i32, cols: i32, stream: *mut c_void);
    fn launch_broadcast_div(x: *const c_void, v: *const c_void, out: *mut c_void, n: i32, cols: i32, stream: *mut c_void);

    // Softmax backward, log-softmax, cross-entropy
    fn launch_softmax_backward(grad: *const c_void, sm: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_log_softmax_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_cross_entropy(logits: *const c_void, targets: *const c_void, losses: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);

    // Gather/scatter
    fn launch_gather_rows(table: *const c_void, indices: *const c_void, out: *mut c_void, n: i32, cols: i32, stream: *mut c_void);
    fn launch_scatter_add(target: *mut c_void, indices: *const c_void, src: *const c_void, n: i32, cols: i32, stream: *mut c_void);

    // Conv backward
    fn launch_col2im_1d(patches: *const c_void, out: *mut c_void, n: i32, p: i32, ks: i32, out_len: i32, stream: *mut c_void);
    fn launch_col2im_2d(patches: *const c_void, out: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, out_h: i32, out_w: i32, stream: *mut c_void);

    // Max pool 1D
    fn launch_max_pool_1d(input: *const c_void, out_vals: *mut c_void, out_idx: *mut c_void, n: i32, out_len: i32, n_filters: i32, stream: *mut c_void);
    fn launch_max_pool_1d_backward(grad: *const c_void, indices: *const c_void, out: *mut c_void, n: i32, out_len: i32, n_filters: i32, stream: *mut c_void);

    // Pool 2D
    fn launch_avg_pool_2d(input: *const c_void, output: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, sh: i32, sw: i32, out_h: i32, out_w: i32, stream: *mut c_void);
    fn launch_avg_pool_2d_backward(grad_out: *const c_void, grad_in: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, sh: i32, sw: i32, out_h: i32, out_w: i32, stream: *mut c_void);
    fn launch_max_pool_2d(input: *const c_void, out_vals: *mut c_void, out_idx: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, sh: i32, sw: i32, out_h: i32, out_w: i32, stream: *mut c_void);
    fn launch_max_pool_2d_backward(grad_out: *const c_void, indices: *const c_void, grad_in: *mut c_void, n: i32, c: i32, out_h: i32, out_w: i32, h: i32, w: i32, stream: *mut c_void);

    // Reduce max/min
    fn launch_reduce_max_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_reduce_max_cols(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_reduce_min_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_reduce_min_cols(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);

    // Comparisons
    fn launch_gt(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_lt(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_eq(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_gt_scalar(x: *const c_void, out: *mut c_void, n: i32, val: f64, stream: *mut c_void);
    fn launch_lt_scalar(x: *const c_void, out: *mut c_void, n: i32, val: f64, stream: *mut c_void);

    // GELU / SiLU
    fn launch_gelu(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_gelu_backward(grad: *const c_void, x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_silu(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_silu_backward(grad: *const c_void, x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);

    // BatchNorm
    fn launch_batchnorm_forward(x: *const c_void, gamma: *const c_void, beta: *const c_void, out: *mut c_void, save_mean: *mut c_void, save_inv_std: *mut c_void, n: i32, c: i32, eps: f64, stream: *mut c_void);
    fn launch_batchnorm_inference(x: *const c_void, gamma: *const c_void, beta: *const c_void, run_mean: *const c_void, run_var: *const c_void, out: *mut c_void, n: i32, c: i32, eps: f64, stream: *mut c_void);
    fn launch_batchnorm_backward(grad_y: *const c_void, x: *const c_void, save_mean: *const c_void, save_inv_std: *const c_void, gamma: *const c_void, grad_x: *mut c_void, grad_gamma: *mut c_void, grad_beta: *mut c_void, n: i32, c: i32, stream: *mut c_void);

    // LayerNorm backward
    fn launch_layernorm_backward(grad_y: *const c_void, x: *const c_void, gamma: *const c_void, grad_x: *mut c_void, grad_gamma: *mut c_void, grad_beta: *mut c_void, rows: i32, cols: i32, eps: f64, stream: *mut c_void);

    // Adam/AdamW
    fn launch_adam_update(w: *mut c_void, m: *mut c_void, v: *mut c_void, g: *const c_void, lr: f64, b1: f64, b2: f64, eps: f64, t: i32, n: i32, stream: *mut c_void);
    fn launch_adamw_update(w: *mut c_void, m: *mut c_void, v: *mut c_void, g: *const c_void, lr: f64, b1: f64, b2: f64, eps: f64, wd: f64, t: i32, n: i32, stream: *mut c_void);

    // GRU
    fn launch_gru_cell(gates: *const c_void, h: *const c_void, h_new: *mut c_void, n: i32, hs: i32, stream: *mut c_void);

    // Structural
    fn launch_slice_cols(src: *const c_void, dst: *mut c_void, rows: i32, src_cols: i32, start: i32, count: i32, stream: *mut c_void);
    fn launch_tril_mask(out: *mut c_void, n: i32, fill_val: f64, stream: *mut c_void);
    fn launch_fill(out: *mut c_void, n: i32, val: f64, stream: *mut c_void);
    fn launch_repeat_rows(src: *const c_void, dst: *mut c_void, src_n: i32, total: i32, stream: *mut c_void);
    fn launch_upsample_nearest_2d(input: *const c_void, output: *mut c_void, n: i32, c: i32, h: i32, w: i32, scale_h: i32, scale_w: i32, stream: *mut c_void);

    // Reductions
    fn launch_log_sum_exp_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
    fn launch_grad_clip_norm(x: *mut c_void, tmp: *mut c_void, n: i32, max_norm: f64, stream: *mut c_void);

    // Prefix sum
    fn launch_prefix_sum_inclusive(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_prefix_sum_exclusive(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);

    // Tree
    fn launch_histogram_build(bins: *const c_void, grad: *const c_void, hess: *const c_void, mask: *const c_void, grad_hist: *mut c_void, hess_hist: *mut c_void, count_hist: *mut c_void, n: i32, p: i32, n_bins: i32, stream: *mut c_void);
    fn launch_split_eval(grad_hist: *const c_void, hess_hist: *const c_void, best_gain: *mut c_void, best_bin: *mut c_void, p: i32, n_bins: i32, lambda: f64, min_child_weight: f64, stream: *mut c_void);
    fn launch_data_partition(bins: *const c_void, node_mask: *const c_void, left_mask: *mut c_void, right_mask: *mut c_void, n: i32, p: i32, split_feat: i32, split_bin: i32, stream: *mut c_void);
    fn launch_tree_build(tr_bins: *const c_void, n_tr: i32, grad: *const c_void, hess: *const c_void, te_bins: *const c_void, n_te: i32, p: i32, n_bins: i32, max_depth: i32, lambda: f64, min_cw: f64, tr_pred: *mut c_void, te_pred: *mut c_void, stream: *mut c_void);

    // Oblivious tree kernels (u8 bins, f32 grad/hess)
    fn launch_mse_grad(pred: *const c_void, target: *const c_void, grad: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_argmax_f32(data: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_fill_f32(out: *mut c_void, val: f32, n: i32, stream: *mut c_void);
    fn launch_write_split(split_feat: *mut c_void, split_bin: *mut c_void, feat: i32, bin: u8, d: i32, stream: *mut c_void);
    fn launch_argmax_write_split(gain: *const c_void, split_feat: *mut c_void, split_bin: *mut c_void, best_idx: *mut c_void, n_features: i32, n_bins: i32, d: i32, stream: *mut c_void);
    fn launch_oblivious_histogram(bins_fm: *const c_void, node_idx: *const c_void, grad: *const c_void, hess: *const c_void, grad_hist: *mut c_void, hess_hist: *mut c_void, n_rows: i32, n_features: i32, n_bins: i32, n_nodes: i32, stream: *mut c_void);
    fn launch_oblivious_route_step(bins_rm: *const c_void, node_in: *const c_void, node_out: *mut c_void, split_feat: i32, split_bin: u8, depth: i32, n_rows: i32, n_features: i32, stream: *mut c_void);
    fn launch_oblivious_route_step_dev(bins_rm: *const c_void, node_in: *const c_void, node_out: *mut c_void, split_feat_arr: *const c_void, split_bin_arr: *const c_void, depth: i32, n_rows: i32, n_features: i32, stream: *mut c_void);
    fn launch_oblivious_route_full(bins_rm: *const c_void, split_feat: *const c_void, split_bin: *const c_void, leaf_idx: *mut c_void, n_rows: i32, n_features: i32, depth: i32, stream: *mut c_void);
    fn launch_scatter_add_by_leaf(pred: *mut c_void, leaf_idx: *const c_void, leaf_value: *const c_void, lr: f32, n_rows: i32, stream: *mut c_void);
    fn launch_leaf_reduce(leaf_idx: *const c_void, grad: *const c_void, hess: *const c_void, leaf_grad: *mut c_void, leaf_hess: *mut c_void, n_rows: i32, stream: *mut c_void);
    fn launch_leaf_finalize(leaf_grad: *const c_void, leaf_hess: *const c_void, leaf_value: *mut c_void, lambda: f32, n_leaves: i32, stream: *mut c_void);
    fn launch_oblivious_split_eval(grad_hist: *const c_void, hess_hist: *const c_void, gain_out: *mut c_void, n_nodes: i32, n_features: i32, n_bins: i32, lambda: f32, min_cw: f32, stream: *mut c_void);
    fn launch_softmax_ce_class_grad_f32(ptrs: *const c_void, targets: *const c_void, grad: *mut c_void, hess: *mut c_void, k: i32, n: i32, nc: i32, stream: *mut c_void);
    fn launch_logloss_grad_f32(pred: *const c_void, target: *const c_void, grad: *mut c_void, hess: *mut c_void, n: i32, stream: *mut c_void);
    fn launch_softmax_inplace(x: *mut c_void, n_rows: i32, n_classes: i32, stream: *mut c_void);
    fn launch_logloss_grad_mc(pred: *const c_void, tgt: *const c_void, grad: *mut c_void, hess: *mut c_void, n_rows: i32, n_classes: i32, stream: *mut c_void);
    fn launch_accuracy(pred: *const c_void, tgt: *const c_void, out: *mut c_void, n_rows: i32, n_classes: i32, stream: *mut c_void);
    fn launch_scatter_add_by_leaf_col(pred: *mut c_void, leaf_idx: *const c_void, leaf_value: *const c_void, lr: f32, n_rows: i32, n_classes: i32, col: i32, stream: *mut c_void);

    // DTW
    fn launch_dtw(cost: *const c_void, dp: *mut c_void, m: i32, n: i32, stream: *mut c_void);

    // Apriori
    fn launch_itemset_support(trans: *const c_void, cands: *const c_void, counts: *mut c_void, n_trans: i32, n_items: i32, n_cands: i32, k: i32, stream: *mut c_void);
    fn launch_candidate_generate(freq: *const c_void, out: *mut c_void, n_freq: i32, k: i32, stream: *mut c_void) -> i32;

    // Philox GPU RNG
    fn launch_rand_uniform(out: *mut c_void, n: i32, seed: u32, stream: *mut c_void);
    fn launch_randn(out: *mut c_void, n: i32, seed: u32, stream: *mut c_void);
    fn launch_bernoulli(out: *mut c_void, n: i32, p: f64, seed: u32, stream: *mut c_void);
}

// Thread-local rocBLAS handle — created once per thread, never destroyed until thread exits.
// rocBLAS handles are not Send; each rayon worker gets its own.
use std::sync::atomic::AtomicPtr;
thread_local! {
    static ROCBLAS_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
}

fn rocblas_handle() -> *mut c_void {
    ROCBLAS_HANDLE.with(|h| {
        let ptr = h.load(std::sync::atomic::Ordering::Relaxed);
        if !ptr.is_null() {
            return ptr;
        }
        let mut handle: *mut c_void = std::ptr::null_mut();
        let status = unsafe { rocblas_create_handle(&mut handle) };
        assert_eq!(status, 0, "rocblas_create_handle failed with status {}", status);
        h.store(handle, std::sync::atomic::Ordering::Relaxed);
        handle
    })
}

/// Destroy the thread-local rocBLAS handle and reset the GPU device.
/// Call once at program exit to release all GPU resources cleanly.
pub fn gpu_shutdown() {
    ROCBLAS_HANDLE.with(|h| {
        let ptr = h.swap(std::ptr::null_mut(), std::sync::atomic::Ordering::Relaxed);
        if !ptr.is_null() {
            unsafe { rocblas_destroy_handle(ptr); }
        }
    });
    unsafe { hipDeviceReset(); }
}

/// Fused linear: out = X @ W + bias. X is (m,k), W is (k,n), bias is (1,n), out is (m,n).
///
/// Pre-fills output with bias broadcast, then dgemm with beta=1.0 adds the matmul on top.
/// One output buffer, zero intermediates. The bias addition rides free in GEMM's write-back.
pub fn gpu_linear(x: &GpuBuffer, w: &GpuBuffer, bias: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
    let c = GpuBuffer::alloc(m * n)?;
    // Broadcast bias into every row of C
    unsafe { launch_repeat_rows(bias.ptr as *const c_void, c.ptr, n as i32, (m * n) as i32, std::ptr::null_mut()); }
    // C = 1.0 * X @ W + 1.0 * C_bias
    let alpha = 1.0_f64;
    let beta = 1.0_f64;
    let status = unsafe {
        rocblas_dgemm(
            rocblas_handle(),
            ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_NONE,
            n as i32, m as i32, k as i32,
            &alpha,
            w.ptr as *const f64, n as i32,
            x.ptr as *const f64, k as i32,
            &beta,
            c.ptr as *mut f64, n as i32,
        )
    };
    check(status)?;
    Ok(c)
}

/// Linear backward: returns (grad_input, grad_w, grad_b).
/// Three separate GEMM/reduce dispatches — no fusion, just API cleanliness.
pub fn gpu_linear_backward(grad: &GpuBuffer, input: &GpuBuffer, weight: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
    let grad_w = gpu_gemm_at(input, grad, k, n, m)?;
    let grad_b = gpu_reduce_sum_cols(grad, m, n)?;
    let grad_input = gpu_gemm_bt(grad, weight, m, k, n)?;
    Ok((grad_input, grad_w, grad_b))
}

/// C = A @ B, A is (m x k) row-major, B is (k x n) row-major, C is (m x n) row-major.
///
/// rocBLAS is column-major. The identity C_rm = (C_cm)^T = (B_cm @ A_cm)^T lets us call:
///   rocblas_dgemm(N, N, n, m, k, 1.0, B, n, A, k, 0.0, C, n)
pub fn gpu_gemm(a: &GpuBuffer, b: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
    let c = GpuBuffer::alloc(m * n)?;
    let alpha = 1.0_f64;
    let beta = 0.0_f64;
    let status = unsafe {
        rocblas_dgemm(
            rocblas_handle(),
            ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_NONE,
            n as i32, m as i32, k as i32,
            &alpha,
            b.ptr as *const f64, n as i32,
            a.ptr as *const f64, k as i32,
            &beta,
            c.ptr as *mut f64, n as i32,
        )
    };
    check(status)?;
    Ok(c)
}

/// C = A^T @ B, A is (k x m) row-major, B is (k x n) row-major, C is (m x n) row-major.
///
/// Column-major: C_cm = B_cm @ A_cm^T →
///   rocblas_dgemm(N, T, n, m, k, 1.0, B, n, A, m, 0.0, C, n)
pub fn gpu_gemm_at(a: &GpuBuffer, b: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
    let c = GpuBuffer::alloc(m * n)?;
    let alpha = 1.0_f64;
    let beta = 0.0_f64;
    let status = unsafe {
        rocblas_dgemm(
            rocblas_handle(),
            ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_TRANSPOSE,
            n as i32, m as i32, k as i32,
            &alpha,
            b.ptr as *const f64, n as i32,
            a.ptr as *const f64, m as i32,
            &beta,
            c.ptr as *mut f64, n as i32,
        )
    };
    check(status)?;
    Ok(c)
}

/// C = A @ B^T, A is (m x k) row-major, B is (n x k) row-major, C is (m x n) row-major.
///
/// Column-major: C_cm = B_cm^T @ A_cm →
///   rocblas_dgemm(T, N, n, m, k, 1.0, B, k, A, k, 0.0, C, n)
pub fn gpu_gemm_bt(a: &GpuBuffer, b: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
    let c = GpuBuffer::alloc(m * n)?;
    let alpha = 1.0_f64;
    let beta = 0.0_f64;
    let status = unsafe {
        rocblas_dgemm(
            rocblas_handle(),
            ROCBLAS_OPERATION_TRANSPOSE, ROCBLAS_OPERATION_NONE,
            n as i32, m as i32, k as i32,
            &alpha,
            b.ptr as *const f64, k as i32,
            a.ptr as *const f64, k as i32,
            &beta,
            c.ptr as *mut f64, n as i32,
        )
    };
    check(status)?;
    Ok(c)
}

/// GPU Cholesky solve: solve A x = b where A is symmetric positive-definite (n x n).
/// Uses rocsolver dpotrf (factorize) + rocblas dtrsm (triangular solve).
/// Copies inputs (dpotrf destroys A, dtrsm overwrites b). Returns solution on GPU.
pub fn gpu_cholesky_solve(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let a_copy = gpu_copy(a, n * n)?;
      let b_copy = gpu_copy(b, n)?;

      // Cholesky factorize: A = L L^T
      let info_buf = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
      let status = unsafe {
            rocsolver_dpotrf(
                  rocblas_handle(),
                  121, // rocblas_fill_lower
                  n as i32,
                  a_copy.ptr as *mut f64, n as i32,
                  info_buf.ptr as *mut i32,
            )
      };
      check(status)?;

      // Forward solve: L z = b
      let alpha = 1.0_f64;
      let status = unsafe {
            rocblas_dtrsm(
                  rocblas_handle(),
                  141, // left
                  121, // lower
                  111, // no transpose
                  131, // non-unit diagonal
                  n as i32, 1,
                  &alpha,
                  a_copy.ptr as *const f64, n as i32,
                  b_copy.ptr as *mut f64, n as i32,
            )
      };
      check(status)?;

      // Backward solve: L^T x = z
      let status = unsafe {
            rocblas_dtrsm(
                  rocblas_handle(),
                  141, // left
                  121, // lower
                  112, // transpose
                  131, // non-unit diagonal
                  n as i32, 1,
                  &alpha,
                  a_copy.ptr as *const f64, n as i32,
                  b_copy.ptr as *mut f64, n as i32,
            )
      };
      check(status)?;

      Ok(b_copy)
}

/// GPU matrix inversion via Cholesky: A^{-1} where A is SPD (n x n).
/// Copies A (dpotrf destroys it), creates identity on GPU. Returns A^{-1} on GPU.
pub fn gpu_cholesky_inv(a: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let a_copy = gpu_copy(a, n * n)?;
      let eye = gpu_eye(n)?;

      // Cholesky factorize: A = L L^T
      let info_buf = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
      let status = unsafe {
            rocsolver_dpotrf(
                  rocblas_handle(),
                  121, // rocblas_fill_lower
                  n as i32,
                  a_copy.ptr as *mut f64, n as i32,
                  info_buf.ptr as *mut i32,
            )
      };
      check(status)?;

      let alpha = 1.0_f64;
      // Forward solve: L Z = I
      let status = unsafe {
            rocblas_dtrsm(
                  rocblas_handle(),
                  141, 121, 111, 131,
                  n as i32, n as i32,
                  &alpha,
                  a_copy.ptr as *const f64, n as i32,
                  eye.ptr as *mut f64, n as i32,
            )
      };
      check(status)?;

      // Backward solve: L^T X = Z
      let status = unsafe {
            rocblas_dtrsm(
                  rocblas_handle(),
                  141, 121, 112, 131,
                  n as i32, n as i32,
                  &alpha,
                  a_copy.ptr as *const f64, n as i32,
                  eye.ptr as *mut f64, n as i32,
            )
      };
      check(status)?;

      Ok(eye)
}

/// GPU general linear solve via LU: solve A*X = B. A is [n,n], B is [n,nrhs].
/// Copies both (dgesv destroys A and overwrites B). Returns solution on GPU.
pub fn gpu_solve(a: &GpuBuffer, b: &GpuBuffer, n: usize, nrhs: usize) -> Result<GpuBuffer, HipError> {
      let a_copy = gpu_copy(a, n * n)?;
      let b_copy = gpu_copy(b, n * nrhs)?;
      let ipiv_buf = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
      let info_buf = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
      let status = unsafe {
            rocsolver_dgesv(
                  rocblas_handle(),
                  n as i32, nrhs as i32,
                  a_copy.ptr as *mut f64, n as i32,
                  ipiv_buf.ptr as *mut i32,
                  b_copy.ptr as *mut f64, n as i32,
                  info_buf.ptr as *mut i32,
            )
      };
      check(status)?;
      Ok(b_copy)
}

/// GPU Cholesky factorization: A = L*L^T. Returns L (lower triangular) on GPU.
/// Copies A (dpotrf destroys it).
pub fn gpu_cholesky(a: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let a_copy = gpu_copy(a, n * n)?;
      let info_buf = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
      let status = unsafe {
            rocsolver_dpotrf(
                  rocblas_handle(),
                  121, // lower
                  n as i32,
                  a_copy.ptr as *mut f64, n as i32,
                  info_buf.ptr as *mut i32,
            )
      };
      check(status)?;
      Ok(a_copy)
}

/// GPU triangular solve: L*X = B (or L^T*X = B if trans=true).
/// dtrsm does NOT destroy L, only overwrites B. Copies b only.
pub fn gpu_tri_solve(l: &GpuBuffer, b: &GpuBuffer, n: usize, nrhs: usize, trans: bool) -> Result<GpuBuffer, HipError> {
      let b_copy = gpu_copy(b, n * nrhs)?;
      let alpha = 1.0_f64;
      let trans_flag = if trans { 112u32 } else { 111u32 };
      let status = unsafe {
            rocblas_dtrsm(
                  rocblas_handle(),
                  141, // left
                  121, // lower
                  trans_flag,
                  131, // non-unit
                  n as i32, nrhs as i32,
                  &alpha,
                  l.ptr as *const f64, n as i32,
                  b_copy.ptr as *mut f64, n as i32,
            )
      };
      check(status)?;
      Ok(b_copy)
}

/// Add scalar to diagonal of n x n matrix in-place
pub fn gpu_add_diag(a: &GpuBuffer, n: usize, val: f64) {
    unsafe { launch_add_diag(a.ptr, n as i32, val, std::ptr::null_mut()); }
}

/// Reparameterize: z = mu + exp(0.5 * log_var) * eps
pub fn gpu_reparameterize(mu: &GpuBuffer, log_var: &GpuBuffer, eps: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_reparameterize(mu.ptr as *const c_void, log_var.ptr as *const c_void, eps.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// KL divergence: 0.5 * (mu^2 + exp(log_var) - log_var - 1) per element
pub fn gpu_kl_div(mu: &GpuBuffer, log_var: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_kl_div(mu.ptr as *const c_void, log_var.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// Fused VAE backward: computes grad_mu and grad_logvar in one dispatch.
/// grad_mu = grad_z + kl_weight * mu
/// grad_lv = grad_z * 0.5 * exp(0.5*lv) * eps + kl_weight * 0.5 * (exp(lv) - 1)
pub fn gpu_vae_backward_latent(grad_z: &GpuBuffer, mu: &GpuBuffer, log_var: &GpuBuffer, eps: &GpuBuffer, n: usize, kl_weight: f64) -> Result<(GpuBuffer, GpuBuffer), HipError> {
    let grad_mu = GpuBuffer::alloc(n)?;
    let grad_lv = GpuBuffer::alloc(n)?;
    unsafe { launch_vae_backward_latent(grad_z.ptr as *const c_void, mu.ptr as *const c_void, log_var.ptr as *const c_void, eps.ptr as *const c_void, grad_mu.ptr, grad_lv.ptr, n as i32, kl_weight, std::ptr::null_mut()); }
    Ok((grad_mu, grad_lv))
}

/// Log-determinant from Cholesky factor: 2 * sum(log(diag(L))).
/// L is the factorized matrix from dpotrf (n x n on GPU). Returns scalar.
pub fn gpu_log_det_cholesky(l: &GpuBuffer, n: usize) -> Result<f64, HipError> {
    let out = GpuBuffer::alloc(1)?;
    unsafe { launch_log_det_cholesky(l.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    let mut result = [0.0f64];
    out.download(&mut result)?;
    Ok(result[0])
}

pub fn gpu_scaled_exp(x: &GpuBuffer, n: usize, scale: f64) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_scaled_exp(x.ptr as *const c_void, out.ptr, n as i32, scale, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_sigmoid(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_sigmoid(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_sigmoid_backward(grad: &GpuBuffer, act: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_sigmoid_backward(grad.ptr as *const c_void, act.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_tanh(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_tanh_act(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_tanh_backward(grad: &GpuBuffer, act: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_tanh_backward(grad.ptr as *const c_void, act.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_relu(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_relu(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_relu_backward(grad: &GpuBuffer, act: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_relu_backward(grad.ptr as *const c_void, act.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_add(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_add(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_add_scalar(x: &GpuBuffer, s: f64, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_add_scalar(x.ptr as *const c_void, out.ptr, n as i32, s, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_div(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_div(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_scale(x: &GpuBuffer, scalar: f64, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    let bytes = n * std::mem::size_of::<f64>();
    // Copy x → out, then scale in-place via rocBLAS dscal
    check(unsafe { hipMemcpy(out.ptr, x.ptr as *const c_void, bytes, crate::hip::HIP_MEMCPY_D2D) })?;
    let status = unsafe {
        rocblas_dscal(rocblas_handle(), n as i32, &scalar, out.ptr as *mut f64, 1)
    };
    check(status)?;
    Ok(out)
}

/// In-place scale: x *= scalar (no alloc, no copy)
pub fn gpu_scale_inplace(x: &GpuBuffer, scalar: f64, n: usize) {
    let status = unsafe {
        rocblas_dscal(rocblas_handle(), n as i32, &scalar, x.ptr as *mut f64, 1)
    };
    assert_eq!(status, 0, "rocblas_dscal failed with status {}", status);
}

pub fn gpu_fma(x: &GpuBuffer, a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_fma(x.ptr as *const c_void, a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// In-place: y -= alpha * x (for SGD weight updates on GPU)
/// SGD weight update: Y = Y - α·X (gradient descent step).
/// Uses rocblas_daxpy with negated alpha. NOT standard axpy (Y = αX + Y).
pub fn gpu_sgd_update(weights: &GpuBuffer, grad: &GpuBuffer, lr: f64, n: usize) {
    let neg_lr = -lr;
    let status = unsafe {
        rocblas_daxpy(
            rocblas_handle(),
            n as i32,
            &neg_lr,
            grad.ptr as *const f64, 1,
            weights.ptr as *mut f64, 1,
        )
    };
    assert_eq!(status, 0, "rocblas_daxpy failed with status {}", status);
}

pub fn gpu_mul(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_mul(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_mul_inplace(a: &GpuBuffer, b: &GpuBuffer, n: usize) {
    unsafe { launch_mul_inplace(a.ptr as *mut c_void, b.ptr as *const c_void, n as i32, std::ptr::null_mut()); }
}

pub fn gpu_add_inplace(a: &GpuBuffer, b: &GpuBuffer, n: usize) {
    unsafe { launch_add(a.ptr as *const c_void, b.ptr as *const c_void, a.ptr as *mut c_void, n as i32, std::ptr::null_mut()); }
}

pub fn gpu_sub_inplace(a: &GpuBuffer, b: &GpuBuffer, n: usize) {
    unsafe { launch_sub(a.ptr as *const c_void, b.ptr as *const c_void, a.ptr as *mut c_void, n as i32, std::ptr::null_mut()); }
}

pub fn gpu_add_scalar_inplace(a: &GpuBuffer, s: f64, n: usize) {
    unsafe { launch_add_scalar(a.ptr as *const c_void, a.ptr as *mut c_void, n as i32, s, std::ptr::null_mut()); }
}

pub fn gpu_linear_into(x: &GpuBuffer, w: &GpuBuffer, bias: &GpuBuffer, out: &GpuBuffer, m: usize, n: usize, k: usize) {
    unsafe { launch_repeat_rows(bias.ptr as *const c_void, out.ptr as *mut c_void, n as i32, (m * n) as i32, std::ptr::null_mut()); }
    let alpha = 1.0_f64;
    let beta = 1.0_f64;
    unsafe {
        rocblas_dgemm(
            rocblas_handle(),
            ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_NONE,
            n as i32, m as i32, k as i32,
            &alpha,
            w.ptr as *const f64, n as i32,
            x.ptr as *const f64, k as i32,
            &beta,
            out.ptr as *mut f64, n as i32,
        );
    }
}

pub fn gpu_layernorm_into(x: &GpuBuffer, out: &GpuBuffer, gamma: Option<&GpuBuffer>, beta: Option<&GpuBuffer>, rows: usize, cols: usize) {
    let g = gamma.map(|b| b.ptr as *const c_void).unwrap_or(std::ptr::null());
    let b = beta.map(|b| b.ptr as *const c_void).unwrap_or(std::ptr::null());
    unsafe { launch_layernorm(x.ptr as *const c_void, out.ptr as *mut c_void, g, b, rows as i32, cols as i32, 1e-5, std::ptr::null_mut()); }
}

pub fn gpu_gelu_into(x: &GpuBuffer, out: &GpuBuffer, n: usize) {
    unsafe { launch_gelu(x.ptr as *const c_void, out.ptr as *mut c_void, n as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_gelu_backward_into(grad: &GpuBuffer, x: &GpuBuffer, out: &GpuBuffer, n: usize) {
    unsafe { launch_gelu_backward(grad.ptr as *const c_void, x.ptr as *const c_void, out.ptr as *mut c_void, n as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_dropout_into(x: &GpuBuffer, mask: &GpuBuffer, out: &GpuBuffer, n: usize, p: f64) {
    let scale = if p < 1.0 { 1.0 / (1.0 - p) } else { 0.0 };
    unsafe { launch_dropout(x.ptr as *const c_void, mask.ptr as *const c_void, out.ptr as *mut c_void, n as i32, p, scale, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_rand_uniform_into(out: &GpuBuffer, n: usize, seed: u32) {
    unsafe { launch_rand_uniform(out.ptr as *mut c_void, n as i32, seed, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_linear_backward_into(grad: &GpuBuffer, input: &GpuBuffer, weight: &GpuBuffer, grad_input: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
    let grad_w = gpu_gemm_at(input, grad, k, n, m)?;
    let grad_b = gpu_reduce_sum_cols(grad, m, n)?;
    let alpha = 1.0_f64;
    let beta = 0.0_f64;
    unsafe {
        rocblas_dgemm(
            rocblas_handle(),
            ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_TRANSPOSE,
            k as i32, m as i32, n as i32,
            &alpha,
            weight.ptr as *const f64, k as i32,
            grad.ptr as *const f64, n as i32,
            &beta,
            grad_input.ptr as *mut f64, k as i32,
        );
    }
    Ok((grad_w, grad_b))
}

pub fn gpu_layernorm_backward_into(grad_y: &GpuBuffer, x: &GpuBuffer, gamma: &GpuBuffer, grad_x: &GpuBuffer, rows: usize, cols: usize, eps: f64) -> Result<(GpuBuffer, GpuBuffer), HipError> {
    let grad_gamma = GpuBuffer::alloc(cols)?;
    let grad_beta = GpuBuffer::alloc(cols)?;
    unsafe { launch_layernorm_backward(grad_y.ptr as *const c_void, x.ptr as *const c_void, gamma.ptr as *const c_void, grad_x.ptr as *mut c_void, grad_gamma.ptr, grad_beta.ptr, rows as i32, cols as i32, eps, std::ptr::null_mut()); }
    Ok((grad_gamma, grad_beta))
}

pub fn gpu_softmax_ce_grad_into(logits: &GpuBuffer, targets: &GpuBuffer, weights: &GpuBuffer, grad_out: &GpuBuffer, n: usize, nc: usize, scale: f64) {
    unsafe { launch_softmax_ce_grad(logits.ptr as *const c_void, targets.ptr as *const c_void, weights.ptr as *const c_void, grad_out.ptr as *mut c_void, n as i32, nc as i32, scale, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_linear_backward_weights_only(grad: &GpuBuffer, input: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
    let grad_w = gpu_gemm_at(input, grad, k, n, m)?;
    let grad_b = gpu_reduce_sum_cols(grad, m, n)?;
    Ok((grad_w, grad_b))
}

pub fn gpu_linear_backward_weights_only_into(grad: &GpuBuffer, input: &GpuBuffer, grad_w: &GpuBuffer, grad_b: &GpuBuffer, m: usize, n: usize, k: usize) {
    let alpha = 1.0_f64;
    let beta = 0.0_f64;
    unsafe {
        rocblas_dgemm(rocblas_handle(), ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_TRANSPOSE,
            n as i32, k as i32, m as i32, &alpha,
            grad.ptr as *const f64, n as i32, input.ptr as *const f64, k as i32,
            &beta, grad_w.ptr as *mut f64, n as i32);
        launch_reduce_sum_cols(grad.ptr as *const c_void, grad_b.ptr as *mut c_void, m as i32, n as i32, std::ptr::null_mut());
    }
}

pub fn gpu_linear_backward_full_into(grad: &GpuBuffer, input: &GpuBuffer, weight: &GpuBuffer, grad_input: &GpuBuffer, grad_w: &GpuBuffer, grad_b: &GpuBuffer, m: usize, n: usize, k: usize) {
    // grad_w = input^T @ grad
    let alpha = 1.0_f64;
    let beta = 0.0_f64;
    unsafe {
        rocblas_dgemm(rocblas_handle(), ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_TRANSPOSE,
            n as i32, k as i32, m as i32, &alpha,
            grad.ptr as *const f64, n as i32, input.ptr as *const f64, k as i32,
            &beta, grad_w.ptr as *mut f64, n as i32);
    }
    // grad_b = sum_cols(grad)
    unsafe { launch_reduce_sum_cols(grad.ptr as *const c_void, grad_b.ptr as *mut c_void, m as i32, n as i32, std::ptr::null_mut()); }
    // grad_input = grad @ weight^T
    unsafe {
        rocblas_dgemm(rocblas_handle(), ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_TRANSPOSE,
            k as i32, m as i32, n as i32, &alpha,
            weight.ptr as *const f64, k as i32, grad.ptr as *const f64, n as i32,
            &beta, grad_input.ptr as *mut f64, k as i32);
    }
}

pub fn gpu_layernorm_backward_full_into(grad_y: &GpuBuffer, x: &GpuBuffer, gamma: &GpuBuffer, grad_x: &GpuBuffer, grad_gamma: &GpuBuffer, grad_beta: &GpuBuffer, rows: usize, cols: usize, eps: f64) {
    unsafe { launch_layernorm_backward(grad_y.ptr as *const c_void, x.ptr as *const c_void, gamma.ptr as *const c_void, grad_x.ptr as *mut c_void, grad_gamma.ptr as *mut c_void, grad_beta.ptr as *mut c_void, rows as i32, cols as i32, eps, std::ptr::null_mut()); }
}

pub fn gpu_softmax_rows_into(x: &GpuBuffer, out: &GpuBuffer, rows: usize, cols: usize) {
    unsafe { launch_softmax_rows(x.ptr as *const c_void, out.ptr as *mut c_void, rows as i32, cols as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_bernoulli_into(out: &GpuBuffer, n: usize, p: f64, seed: u32) {
    unsafe { launch_bernoulli(out.ptr as *mut c_void, n as i32, p, seed, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_grad_hess_into(probs: &GpuBuffer, targets: &GpuBuffer, weights: &GpuBuffer, mask: &GpuBuffer, grad_out: &GpuBuffer, hess_out: &GpuBuffer, n: usize, nc: usize, k: usize) {
    unsafe { launch_grad_hess(probs.ptr as *const c_void, targets.ptr as *const c_void, weights.ptr as *const c_void, mask.ptr as *const c_void, grad_out.ptr as *mut c_void, hess_out.ptr as *mut c_void, n as i32, nc as i32, k as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_tree_build_into(tr_bins: &GpuBuffer, te_bins: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, n_tr: usize, n_te: usize, p: usize, n_bins: usize, max_depth: usize, lambda: f64, min_cw: f64, tr_pred: &GpuBuffer, te_pred: &GpuBuffer) {
    unsafe { launch_tree_build(tr_bins.ptr as *const c_void, n_tr as i32, grad.ptr as *const c_void, hess.ptr as *const c_void, te_bins.ptr as *const c_void, n_te as i32, p as i32, n_bins as i32, max_depth as i32, lambda, min_cw, tr_pred.ptr as *mut c_void, te_pred.ptr as *mut c_void, std::ptr::null_mut()); }
    check_launch();
}

// ── Oblivious tree GPU primitives ──────────────────────────────────────────

pub fn gpu_mse_grad_into(pred: &GpuBuffer, target: &GpuBuffer, grad: &GpuBuffer, n: usize) {
    unsafe { launch_mse_grad(pred.ptr as *const c_void, target.ptr as *const c_void, grad.ptr as *mut c_void, n as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_softmax_ce_class_grad_f32(class_ptrs: &[*mut std::ffi::c_void], targets: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, k: usize, n: usize) {
    let nc = class_ptrs.len();
    let ptr_buf = GpuBuffer::upload_u8(unsafe { std::slice::from_raw_parts(class_ptrs.as_ptr() as *const u8, nc * std::mem::size_of::<*mut c_void>()) }).expect("ptr upload");
    unsafe { launch_softmax_ce_class_grad_f32(ptr_buf.ptr as *const c_void, targets.ptr as *const c_void, grad.ptr as *mut c_void, hess.ptr as *mut c_void, k as i32, n as i32, nc as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_logloss_grad_f32(pred: &GpuBuffer, target: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, n: usize) {
    unsafe { launch_logloss_grad_f32(pred.ptr as *const c_void, target.ptr as *const c_void, grad.ptr as *mut c_void, hess.ptr as *mut c_void, n as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_argmax_f32(data: &GpuBuffer, out: &GpuBuffer, n: usize) {
    unsafe { launch_argmax_f32(data.ptr as *const c_void, out.ptr as *mut c_void, n as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_fill_f32(out: &GpuBuffer, val: f32, n: usize) {
    unsafe { launch_fill_f32(out.ptr as *mut c_void, val, n as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_argmax_write_split(gain: &GpuBuffer, split_feat: &GpuBuffer, split_bin: &GpuBuffer, best_idx: &GpuBuffer, n_features: usize, n_bins: usize, d: usize) {
    unsafe { launch_argmax_write_split(gain.ptr as *const c_void, split_feat.ptr as *mut c_void, split_bin.ptr as *mut c_void, best_idx.ptr as *mut c_void, n_features as i32, n_bins as i32, d as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_write_split(split_feat: &GpuBuffer, split_bin: &GpuBuffer, feat: usize, bin: u8, d: usize) {
    unsafe { launch_write_split(split_feat.ptr as *mut c_void, split_bin.ptr as *mut c_void, feat as i32, bin, d as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_oblivious_histogram(bins_fm: &GpuBuffer, node_idx: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, grad_hist: &GpuBuffer, hess_hist: &GpuBuffer, n_rows: usize, n_features: usize, n_bins: usize, n_nodes: usize) {
    unsafe { launch_oblivious_histogram(bins_fm.ptr as *const c_void, node_idx.ptr as *const c_void, grad.ptr as *const c_void, hess.ptr as *const c_void, grad_hist.ptr as *mut c_void, hess_hist.ptr as *mut c_void, n_rows as i32, n_features as i32, n_bins as i32, n_nodes as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_oblivious_route_step(bins_rm: &GpuBuffer, node_in: &GpuBuffer, node_out: &GpuBuffer, split_feat: usize, split_bin: u8, depth: usize, n_rows: usize, n_features: usize) {
    unsafe { launch_oblivious_route_step(bins_rm.ptr as *const c_void, node_in.ptr as *const c_void, node_out.ptr as *mut c_void, split_feat as i32, split_bin, depth as i32, n_rows as i32, n_features as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_oblivious_route_step_dev(bins_rm: &GpuBuffer, node_in: &GpuBuffer, node_out: &GpuBuffer, split_feat_arr: &GpuBuffer, split_bin_arr: &GpuBuffer, depth: usize, n_rows: usize, n_features: usize) {
    unsafe { launch_oblivious_route_step_dev(bins_rm.ptr as *const c_void, node_in.ptr as *const c_void, node_out.ptr as *mut c_void, split_feat_arr.ptr as *const c_void, split_bin_arr.ptr as *const c_void, depth as i32, n_rows as i32, n_features as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_oblivious_route_full(bins_rm: &GpuBuffer, split_feat: &GpuBuffer, split_bin: &GpuBuffer, leaf_idx: &GpuBuffer, n_rows: usize, n_features: usize, depth: usize) {
    unsafe { launch_oblivious_route_full(bins_rm.ptr as *const c_void, split_feat.ptr as *const c_void, split_bin.ptr as *const c_void, leaf_idx.ptr as *mut c_void, n_rows as i32, n_features as i32, depth as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_scatter_add_by_leaf(pred: &GpuBuffer, leaf_idx: &GpuBuffer, leaf_value: &GpuBuffer, lr: f32, n_rows: usize) {
    unsafe { launch_scatter_add_by_leaf(pred.ptr as *mut c_void, leaf_idx.ptr as *const c_void, leaf_value.ptr as *const c_void, lr, n_rows as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_leaf_reduce(leaf_idx: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, leaf_grad: &GpuBuffer, leaf_hess: &GpuBuffer, n_rows: usize) {
    unsafe { launch_leaf_reduce(leaf_idx.ptr as *const c_void, grad.ptr as *const c_void, hess.ptr as *const c_void, leaf_grad.ptr as *mut c_void, leaf_hess.ptr as *mut c_void, n_rows as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_leaf_finalize(leaf_grad: &GpuBuffer, leaf_hess: &GpuBuffer, leaf_value: &GpuBuffer, lambda: f32, n_leaves: usize) {
    unsafe { launch_leaf_finalize(leaf_grad.ptr as *const c_void, leaf_hess.ptr as *const c_void, leaf_value.ptr as *mut c_void, lambda, n_leaves as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_oblivious_split_eval(grad_hist: &GpuBuffer, hess_hist: &GpuBuffer, gain_out: &GpuBuffer, n_nodes: usize, n_features: usize, n_bins: usize, lambda: f32, min_cw: f32) {
    unsafe { launch_oblivious_split_eval(grad_hist.ptr as *const c_void, hess_hist.ptr as *const c_void, gain_out.ptr as *mut c_void, n_nodes as i32, n_features as i32, n_bins as i32, lambda, min_cw, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_softmax_inplace(x: &GpuBuffer, n_rows: usize, n_classes: usize) {
    unsafe { launch_softmax_inplace(x.ptr as *mut c_void, n_rows as i32, n_classes as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_logloss_grad_mc(pred: &GpuBuffer, tgt: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, n_rows: usize, n_classes: usize) {
    unsafe { launch_logloss_grad_mc(pred.ptr as *const c_void, tgt.ptr as *const c_void, grad.ptr as *mut c_void, hess.ptr as *mut c_void, n_rows as i32, n_classes as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_accuracy(pred: &GpuBuffer, tgt: &GpuBuffer, out: &GpuBuffer, n_rows: usize, n_classes: usize) {
    unsafe { launch_accuracy(pred.ptr as *const c_void, tgt.ptr as *const c_void, out.ptr as *mut c_void, n_rows as i32, n_classes as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_scatter_add_by_leaf_col(pred: &GpuBuffer, leaf_idx: &GpuBuffer, leaf_value: &GpuBuffer, lr: f32, n_rows: usize, n_classes: usize, col: usize) {
    unsafe { launch_scatter_add_by_leaf_col(pred.ptr as *mut c_void, leaf_idx.ptr as *const c_void, leaf_value.ptr as *const c_void, lr, n_rows as i32, n_classes as i32, col as i32, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_add_col_scaled_inplace(matrix: &GpuBuffer, n: usize, cols: usize, k: usize, col: &GpuBuffer, scale: f64) {
    unsafe { launch_add_col_scaled(matrix.ptr as *mut c_void, col.ptr as *const c_void, n as i32, cols as i32, k as i32, scale, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_sub(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_sub(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// Row-wise softmax: out[i,j] = exp(x[i,j] - max_j) / sum(exp). Fully on-device.
pub fn gpu_softmax_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(rows * cols)?;
    unsafe { launch_softmax_rows(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// (a - b) * scale, element-wise. For gradient = (softmax - onehot) / n.
pub fn gpu_sub_scale(a: &GpuBuffer, b: &GpuBuffer, n: usize, scale: f64) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_sub_scale(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, scale, std::ptr::null_mut()); }
    Ok(out)
}

/// GPU 1D avg pool: (n*out_len x n_filters) → (n x n_filters)
pub fn gpu_avg_pool_1d(input: &GpuBuffer, n: usize, out_len: usize, n_filters: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n * n_filters)?;
    unsafe { launch_avg_pool_1d(input.ptr as *const c_void, out.ptr, n as i32, out_len as i32, n_filters as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// GPU pool gradient expand: (n x n_filters) → (n*out_len x n_filters), divided by out_len
pub fn gpu_pool_grad_expand(grad: &GpuBuffer, n: usize, out_len: usize, n_filters: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n * out_len * n_filters)?;
    unsafe { launch_pool_grad_expand(grad.ptr as *const c_void, out.ptr, n as i32, out_len as i32, n_filters as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// Argmin per row: returns int32 assignments (n,) — index of min column per row.
pub fn gpu_argmin_rows(dists: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc_bytes(rows * std::mem::size_of::<i32>())?;
    unsafe { launch_argmin_rows(dists.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// Download i32 assignments from GPU.
pub fn download_assignments(buf: &GpuBuffer, n: usize) -> Result<Vec<i32>, HipError> {
    let mut result = vec![0i32; n];
    let bytes = n * std::mem::size_of::<i32>();
    check(unsafe {
        hipMemcpy(result.as_mut_ptr() as *mut c_void, buf.ptr, bytes, crate::hip::HIP_MEMCPY_D2H)
    })?;
    Ok(result)
}

/// Segmented centroid update: compute new centroids from data + assignments, all on GPU.
/// Returns (centroids_buf (k*dim f64), counts_buf (k i32)).
pub fn gpu_centroid_update(x: &GpuBuffer, assignments: &GpuBuffer, n: usize, dim: usize, k: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
    let centroids = GpuBuffer::alloc(k * dim)?;
    let counts = GpuBuffer::alloc_bytes(k * std::mem::size_of::<i32>())?;
    unsafe { launch_centroid_update(x.ptr as *const c_void, assignments.ptr as *const c_void, centroids.ptr, counts.ptr, n as i32, dim as i32, k as i32, std::ptr::null_mut()); }
    Ok((centroids, counts))
}

/// Per-row top-k: for each row of (rows x cols) distance matrix, find k nearest indices.
/// Returns (rows x k) i32 buffer.
pub fn gpu_topk_per_row(dists: &GpuBuffer, rows: usize, cols: usize, k: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc_bytes(rows * k * std::mem::size_of::<i32>())?;
    unsafe { launch_topk_per_row(dists.ptr as *const c_void, out.ptr, rows as i32, cols as i32, k as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// Download (rows x k) i32 indices from GPU.
pub fn download_topk_indices(buf: &GpuBuffer, rows: usize, k: usize) -> Result<Vec<i32>, HipError> {
    let n = rows * k;
    let mut result = vec![0i32; n];
    let bytes = n * std::mem::size_of::<i32>();
    check(unsafe {
        hipMemcpy(result.as_mut_ptr() as *mut c_void, buf.ptr, bytes, crate::hip::HIP_MEMCPY_D2H)
    })?;
    Ok(result)
}

pub fn gpu_bias_add(x: &GpuBuffer, bias: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(rows * cols)?;
    unsafe { launch_bias_add(x.ptr as *const c_void, bias.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// Fused LSTM cell: apply gate activations and update C, H in-place on GPU.
/// gates: (n × 4*hs), c: (n × hs), h: (n × hs).
pub fn gpu_lstm_cell(gates: &GpuBuffer, c: &GpuBuffer, h: &GpuBuffer, n: usize, hs: usize) {
    unsafe { launch_lstm_cell(gates.ptr as *const c_void, c.ptr as *mut c_void, h.ptr as *mut c_void, n as i32, hs as i32, std::ptr::null_mut()); }
}

/// Gaussian log-likelihood matrix: out[i,c] = log_prior[c] - 0.5 * sum_j(log(var)+diff²/var)
/// x: (n×p), means: (k×p), vars: (k×p), log_priors: (k) → out: (n×k)
pub fn gpu_gaussian_ll(x: &GpuBuffer, means: &GpuBuffer, vars: &GpuBuffer, log_priors: &GpuBuffer, n: usize, k: usize, p: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n * k)?;
    unsafe { launch_gaussian_ll(x.ptr as *const c_void, means.ptr as *const c_void, vars.ptr as *const c_void, log_priors.ptr as *const c_void, out.ptr, n as i32, k as i32, p as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// im2col for 1D conv: X (n×p) → patches (n*out_len × ks), out_len = p - ks + 1
pub fn gpu_im2col_1d(x: &GpuBuffer, n: usize, p: usize, ks: usize) -> Result<GpuBuffer, HipError> {
    let out_len = p - ks + 1;
    let out = GpuBuffer::alloc(n * out_len * ks)?;
    unsafe { launch_im2col_1d(x.ptr as *const c_void, out.ptr, n as i32, p as i32, ks as i32, out_len as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// Argmax per row: out[i] = argmax_j(x[i,j]) as f64 index
pub fn gpu_argmax_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(rows)?;
    unsafe { launch_argmax_rows(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_reduce_sum_cols(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(cols)?;
    unsafe { launch_reduce_sum_cols(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_reduce_sum_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(rows)?;
    unsafe { launch_reduce_sum_rows(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_reduce_mean_cols(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(cols)?;
    unsafe { launch_reduce_mean_cols(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_reduce_var_cols(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(cols)?;
    unsafe { launch_reduce_var_cols(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_pairwise_l2(query: &GpuBuffer, train: &GpuBuffer, nq: usize, nt: usize, dim: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(nq * nt)?;
    unsafe {
        launch_pairwise_l2(query.ptr as *const c_void, train.ptr as *const c_void, out.ptr, nq as i32, nt as i32, dim as i32, std::ptr::null_mut());
    }
    Ok(out)
}

pub fn gpu_partial_argsort(data: &GpuBuffer, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
    let out_bytes = k * std::mem::size_of::<i32>();
    let out = GpuBuffer::alloc_bytes(out_bytes)?;
    unsafe {
        launch_partial_argsort(data.ptr as *const c_void, out.ptr, n as i32, k as i32, std::ptr::null_mut());
    }
    Ok(out)
}

pub fn download_indices(buf: &GpuBuffer, k: usize) -> Result<Vec<i32>, HipError> {
    let mut result = vec![0i32; k];
    let bytes = k * std::mem::size_of::<i32>();
    check(unsafe {
        hipMemcpy(result.as_mut_ptr() as *mut c_void, buf.ptr, bytes, crate::hip::HIP_MEMCPY_D2H)
    })?;
    Ok(result)
}

// ── New kernel wrappers ─────────────────────────────────────────────────────

pub fn gpu_leaky_relu(x: &GpuBuffer, n: usize, alpha: f64) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_leaky_relu(x.ptr as *const c_void, out.ptr, n as i32, alpha, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_leaky_relu_backward(grad: &GpuBuffer, act: &GpuBuffer, n: usize, alpha: f64) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    unsafe { launch_leaky_relu_backward(grad.ptr as *const c_void, act.ptr as *const c_void, out.ptr, n as i32, alpha, std::ptr::null_mut()); }
    Ok(out)
}

/// Row-wise layer normalization. Pass null GpuBuffers for gamma/beta to skip affine transform.
pub fn gpu_layernorm(x: &GpuBuffer, rows: usize, cols: usize, gamma: Option<&GpuBuffer>, beta: Option<&GpuBuffer>) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(rows * cols)?;
    let g = gamma.map(|b| b.ptr as *const c_void).unwrap_or(std::ptr::null());
    let b = beta.map(|b| b.ptr as *const c_void).unwrap_or(std::ptr::null());
    unsafe { launch_layernorm(x.ptr as *const c_void, out.ptr, g, b, rows as i32, cols as i32, 1e-5, std::ptr::null_mut()); }
    Ok(out)
}

/// Dropout with pre-generated mask (uniform [0,1) values). out[i] = mask[i] < p ? 0 : x[i] * 1/(1-p)
pub fn gpu_dropout(x: &GpuBuffer, mask: &GpuBuffer, n: usize, p: f64) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(n)?;
    let scale = if p < 1.0 { 1.0 / (1.0 - p) } else { 0.0 };
    unsafe { launch_dropout(x.ptr as *const c_void, mask.ptr as *const c_void, out.ptr, n as i32, p, scale, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_bernoulli_u8(mask: &GpuBuffer, n: usize, seed: u32, p: f64) {
    unsafe { launch_bernoulli_u8(mask.ptr as *mut c_void, n as i32, seed, p, std::ptr::null_mut()); }
    check_launch();
}

pub fn gpu_dropout_u8_into(x: &GpuBuffer, mask: &GpuBuffer, out: &GpuBuffer, n: usize, p: f64) {
    let scale = if p < 1.0 { 1.0 / (1.0 - p) } else { 0.0 };
    unsafe { launch_dropout_u8(x.ptr as *const c_void, mask.ptr as *const c_void, out.ptr as *mut c_void, n as i32, scale, std::ptr::null_mut()); }
    check_launch();
}

/// Concatenate (rows, d1) and (rows, d2) into (rows, d1+d2).
pub fn gpu_concat(a: &GpuBuffer, b: &GpuBuffer, rows: usize, d1: usize, d2: usize) -> Result<GpuBuffer, HipError> {
    let out = GpuBuffer::alloc(rows * (d1 + d2))?;
    unsafe { launch_concat(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, rows as i32, d1 as i32, d2 as i32, std::ptr::null_mut()); }
    Ok(out)
}

/// 2D im2col: extract patches from NCHW images for GEMM-based convolution.
/// Output: (N*outH*outW, C*kH*kW) patch matrix.
pub fn gpu_im2col_2d(x: &GpuBuffer, n: usize, c: usize, h: usize, w: usize, kh: usize, kw: usize) -> Result<GpuBuffer, HipError> {
    let out_h = h - kh + 1;
    let out_w = w - kw + 1;
    let out = GpuBuffer::alloc(n * out_h * out_w * c * kh * kw)?;
    unsafe { launch_im2col_2d(x.ptr as *const c_void, out.ptr, n as i32, c as i32, h as i32, w as i32, kh as i32, kw as i32, out_h as i32, out_w as i32, std::ptr::null_mut()); }
    Ok(out)
}

pub fn gpu_exp(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_exp(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_log(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_log(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_sqrt(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_sqrt(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_abs(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_abs(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_neg(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_neg(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_sign(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_sign(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_pow(x: &GpuBuffer, n: usize, p: f64) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_pow(x.ptr as *const c_void, out.ptr, n as i32, p, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_clamp(x: &GpuBuffer, n: usize, lo: f64, hi: f64) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_clamp(x.ptr as *const c_void, out.ptr, n as i32, lo, hi, std::ptr::null_mut()); }
      Ok(out)
}

/// Transpose: (rows x cols) row-major → (cols x rows) row-major.
pub fn gpu_transpose(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows * cols)?;
      unsafe { launch_transpose(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

/// Identity matrix (n x n) on device.
pub fn gpu_eye(n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n * n)?;
      unsafe { launch_eye(out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

/// Device-to-device buffer copy. n is number of f64 elements.
pub fn gpu_copy(src: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let dst = GpuBuffer::alloc(n)?;
      let bytes = n * std::mem::size_of::<f64>();
      check(unsafe { hipMemcpy(dst.ptr, src.ptr as *const c_void, bytes, crate::hip::HIP_MEMCPY_D2D) })?;
      Ok(dst)
}

/// Conditional selection: out[i] = cond[i] != 0.0 ? a[i] : b[i].
pub fn gpu_where_mask(cond: &GpuBuffer, a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_where_mask(cond.ptr as *const c_void, a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

/// Extract rows [start, start+count) from (total_rows x cols) matrix.
pub fn gpu_slice_rows(x: &GpuBuffer, start: usize, count: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let total_rows = x.n_floats() / cols;
      assert!(start + count <= total_rows, "slice_rows: start({start}) + count({count}) = {} exceeds rows({total_rows})", start + count);
      let out = GpuBuffer::alloc(count * cols)?;
      unsafe { launch_slice_rows(x.ptr as *const c_void, out.ptr, start as i32, count as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

/// Broadcast subtract: out[i] = X[i] - v[i % cols]. X is [rows, cols], v is [1, cols].
pub fn gpu_broadcast_sub(x: &GpuBuffer, v: &GpuBuffer, n: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_broadcast_sub(x.ptr as *const c_void, v.ptr as *const c_void, out.ptr, n as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

/// Broadcast multiply: out[i] = X[i] * v[i % cols]. X is [rows, cols], v is [1, cols].
pub fn gpu_broadcast_mul(x: &GpuBuffer, v: &GpuBuffer, n: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_broadcast_mul(x.ptr as *const c_void, v.ptr as *const c_void, out.ptr, n as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

/// Broadcast divide: out[i] = X[i] / v[i % cols]. X is [rows, cols], v is [1, cols].
pub fn gpu_broadcast_div(x: &GpuBuffer, v: &GpuBuffer, n: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_broadcast_div(x.ptr as *const c_void, v.ptr as *const c_void, out.ptr, n as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

// ── New ops ────────────────────────────────────────────────────────────────

pub fn gpu_softmax_backward(grad: &GpuBuffer, sm: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows * cols)?;
      unsafe { launch_softmax_backward(grad.ptr as *const c_void, sm.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_log_softmax_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows * cols)?;
      unsafe { launch_log_softmax_rows(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_cross_entropy(logits: &GpuBuffer, targets: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows)?;
      unsafe { launch_cross_entropy(logits.ptr as *const c_void, targets.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_gather_rows(table: &GpuBuffer, indices: &GpuBuffer, n: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n * cols)?;
      unsafe { launch_gather_rows(table.ptr as *const c_void, indices.ptr as *const c_void, out.ptr, n as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_scatter_add(target: &GpuBuffer, indices: &GpuBuffer, src: &GpuBuffer, n: usize, cols: usize) {
      unsafe { launch_scatter_add(target.ptr, indices.ptr as *const c_void, src.ptr as *const c_void, n as i32, cols as i32, std::ptr::null_mut()); }
}

pub fn gpu_col2im_1d(patches: &GpuBuffer, n: usize, p: usize, ks: usize) -> Result<GpuBuffer, HipError> {
      let out_len = p - ks + 1;
      let out = GpuBuffer::alloc(n * p)?;
      unsafe { launch_col2im_1d(patches.ptr as *const c_void, out.ptr, n as i32, p as i32, ks as i32, out_len as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_col2im_2d(patches: &GpuBuffer, n: usize, c: usize, h: usize, w: usize, kh: usize, kw: usize) -> Result<GpuBuffer, HipError> {
      let out_h = h - kh + 1;
      let out_w = w - kw + 1;
      let out = GpuBuffer::alloc(n * c * h * w)?;
      unsafe { launch_col2im_2d(patches.ptr as *const c_void, out.ptr, n as i32, c as i32, h as i32, w as i32, kh as i32, kw as i32, out_h as i32, out_w as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_max_pool_1d(input: &GpuBuffer, n: usize, out_len: usize, n_filters: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let vals = GpuBuffer::alloc(n * n_filters)?;
      let idx = GpuBuffer::alloc(n * n_filters)?;
      unsafe { launch_max_pool_1d(input.ptr as *const c_void, vals.ptr, idx.ptr, n as i32, out_len as i32, n_filters as i32, std::ptr::null_mut()); }
      Ok((vals, idx))
}

pub fn gpu_max_pool_1d_backward(grad: &GpuBuffer, indices: &GpuBuffer, n: usize, out_len: usize, n_filters: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n * out_len * n_filters)?;
      unsafe { launch_max_pool_1d_backward(grad.ptr as *const c_void, indices.ptr as *const c_void, out.ptr, n as i32, out_len as i32, n_filters as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_avg_pool_2d(input: &GpuBuffer, n: usize, c: usize, h: usize, w: usize, kh: usize, kw: usize, sh: usize, sw: usize) -> Result<GpuBuffer, HipError> {
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let out = GpuBuffer::alloc(n * c * out_h * out_w)?;
      unsafe { launch_avg_pool_2d(input.ptr as *const c_void, out.ptr, n as i32, c as i32, h as i32, w as i32, kh as i32, kw as i32, sh as i32, sw as i32, out_h as i32, out_w as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_avg_pool_2d_backward(grad: &GpuBuffer, n: usize, c: usize, h: usize, w: usize, kh: usize, kw: usize, sh: usize, sw: usize) -> Result<GpuBuffer, HipError> {
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let out = GpuBuffer::alloc(n * c * h * w)?;
      unsafe { launch_avg_pool_2d_backward(grad.ptr as *const c_void, out.ptr, n as i32, c as i32, h as i32, w as i32, kh as i32, kw as i32, sh as i32, sw as i32, out_h as i32, out_w as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_max_pool_2d(input: &GpuBuffer, n: usize, c: usize, h: usize, w: usize, kh: usize, kw: usize, sh: usize, sw: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let vals = GpuBuffer::alloc(n * c * out_h * out_w)?;
      let idx = GpuBuffer::alloc(n * c * out_h * out_w)?;
      unsafe { launch_max_pool_2d(input.ptr as *const c_void, vals.ptr, idx.ptr, n as i32, c as i32, h as i32, w as i32, kh as i32, kw as i32, sh as i32, sw as i32, out_h as i32, out_w as i32, std::ptr::null_mut()); }
      Ok((vals, idx))
}

pub fn gpu_max_pool_2d_backward(grad: &GpuBuffer, indices: &GpuBuffer, n: usize, c: usize, h: usize, w: usize, out_h: usize, out_w: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n * c * h * w)?;
      unsafe { launch_max_pool_2d_backward(grad.ptr as *const c_void, indices.ptr as *const c_void, out.ptr, n as i32, c as i32, out_h as i32, out_w as i32, h as i32, w as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_reduce_max_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows)?;
      unsafe { launch_reduce_max_rows(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_reduce_max_cols(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(cols)?;
      unsafe { launch_reduce_max_cols(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_reduce_min_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows)?;
      unsafe { launch_reduce_min_rows(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_reduce_min_cols(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(cols)?;
      unsafe { launch_reduce_min_cols(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

// ── Comparisons ────────────────────────────────────────────────────────────

pub fn gpu_gt(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_gt(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}
pub fn gpu_lt(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_lt(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}
pub fn gpu_eq(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_eq(a.ptr as *const c_void, b.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}
pub fn gpu_gt_scalar(x: &GpuBuffer, n: usize, val: f64) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_gt_scalar(x.ptr as *const c_void, out.ptr, n as i32, val, std::ptr::null_mut()); }
      Ok(out)
}
pub fn gpu_lt_scalar(x: &GpuBuffer, n: usize, val: f64) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_lt_scalar(x.ptr as *const c_void, out.ptr, n as i32, val, std::ptr::null_mut()); }
      Ok(out)
}

// ── GELU / SiLU ───────────────────────────────────────────────────────────

pub fn gpu_gelu(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_gelu(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}
pub fn gpu_gelu_backward(grad: &GpuBuffer, x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_gelu_backward(grad.ptr as *const c_void, x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}
pub fn gpu_silu(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_silu(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}
pub fn gpu_silu_backward(grad: &GpuBuffer, x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_silu_backward(grad.ptr as *const c_void, x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

// ── BatchNorm ──────────────────────────────────────────────────────────────

pub fn gpu_batchnorm_forward(x: &GpuBuffer, gamma: &GpuBuffer, beta: &GpuBuffer, n: usize, c: usize, eps: f64) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
      let out = GpuBuffer::alloc(n * c)?;
      let mean = GpuBuffer::alloc(c)?;
      let inv_std = GpuBuffer::alloc(c)?;
      unsafe { launch_batchnorm_forward(x.ptr as *const c_void, gamma.ptr as *const c_void, beta.ptr as *const c_void, out.ptr, mean.ptr, inv_std.ptr, n as i32, c as i32, eps, std::ptr::null_mut()); }
      Ok((out, mean, inv_std))
}

pub fn gpu_batchnorm_inference(x: &GpuBuffer, gamma: &GpuBuffer, beta: &GpuBuffer, run_mean: &GpuBuffer, run_var: &GpuBuffer, n: usize, c: usize, eps: f64) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n * c)?;
      unsafe { launch_batchnorm_inference(x.ptr as *const c_void, gamma.ptr as *const c_void, beta.ptr as *const c_void, run_mean.ptr as *const c_void, run_var.ptr as *const c_void, out.ptr, n as i32, c as i32, eps, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_batchnorm_backward(grad_y: &GpuBuffer, x: &GpuBuffer, save_mean: &GpuBuffer, save_inv_std: &GpuBuffer, gamma: &GpuBuffer, n: usize, c: usize) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
      let grad_x = GpuBuffer::alloc(n * c)?;
      let grad_gamma = GpuBuffer::alloc(c)?;
      let grad_beta = GpuBuffer::alloc(c)?;
      unsafe { launch_batchnorm_backward(grad_y.ptr as *const c_void, x.ptr as *const c_void, save_mean.ptr as *const c_void, save_inv_std.ptr as *const c_void, gamma.ptr as *const c_void, grad_x.ptr, grad_gamma.ptr, grad_beta.ptr, n as i32, c as i32, std::ptr::null_mut()); }
      Ok((grad_x, grad_gamma, grad_beta))
}

// ── LayerNorm backward ────────────────────────────────────────────────────

pub fn gpu_layernorm_backward(grad_y: &GpuBuffer, x: &GpuBuffer, gamma: &GpuBuffer, rows: usize, cols: usize, eps: f64) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
      let grad_x = GpuBuffer::alloc(rows * cols)?;
      let grad_gamma = GpuBuffer::alloc(cols)?;
      let grad_beta = GpuBuffer::alloc(cols)?;
      unsafe { launch_layernorm_backward(grad_y.ptr as *const c_void, x.ptr as *const c_void, gamma.ptr as *const c_void, grad_x.ptr, grad_gamma.ptr, grad_beta.ptr, rows as i32, cols as i32, eps, std::ptr::null_mut()); }
      Ok((grad_x, grad_gamma, grad_beta))
}

// ── Adam / AdamW ──────────────────────────────────────────────────────────

pub fn gpu_adam_update(w: &GpuBuffer, m: &GpuBuffer, v: &GpuBuffer, g: &GpuBuffer, lr: f64, b1: f64, b2: f64, eps: f64, t: usize, n: usize) {
      unsafe { launch_adam_update(w.ptr, m.ptr, v.ptr, g.ptr as *const c_void, lr, b1, b2, eps, t as i32, n as i32, std::ptr::null_mut()); }
}

pub fn gpu_adamw_update(w: &GpuBuffer, m: &GpuBuffer, v: &GpuBuffer, g: &GpuBuffer, lr: f64, b1: f64, b2: f64, eps: f64, wd: f64, t: usize, n: usize) {
      unsafe { launch_adamw_update(w.ptr, m.ptr, v.ptr, g.ptr as *const c_void, lr, b1, b2, eps, wd, t as i32, n as i32, std::ptr::null_mut()); }
}

// ── GRU ───────────────────────────────────────────────────────────────────

pub fn gpu_gru_cell(gates: &GpuBuffer, h: &GpuBuffer, n: usize, hs: usize) -> Result<GpuBuffer, HipError> {
      let h_new = GpuBuffer::alloc(n * hs)?;
      unsafe { launch_gru_cell(gates.ptr as *const c_void, h.ptr as *const c_void, h_new.ptr, n as i32, hs as i32, std::ptr::null_mut()); }
      Ok(h_new)
}

// ── Structural ────────────────────────────────────────────────────────────

pub fn gpu_vconcat(a: &GpuBuffer, b: &GpuBuffer, a_n: usize, b_n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(a_n + b_n)?;
      let a_bytes = a_n * std::mem::size_of::<f64>();
      let b_bytes = b_n * std::mem::size_of::<f64>();
      check(unsafe { hipMemcpy(out.ptr, a.ptr as *const c_void, a_bytes, crate::hip::HIP_MEMCPY_D2D) })?;
      check(unsafe { hipMemcpy((out.ptr as *mut u8).add(a_bytes) as *mut c_void, b.ptr as *const c_void, b_bytes, crate::hip::HIP_MEMCPY_D2D) })?;
      Ok(out)
}

pub fn gpu_slice_cols(x: &GpuBuffer, rows: usize, cols: usize, start: usize, count: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows * count)?;
      unsafe { launch_slice_cols(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, start as i32, count as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_tril_mask(n: usize, fill_val: f64) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n * n)?;
      unsafe { launch_tril_mask(out.ptr, n as i32, fill_val, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_fill(n: usize, val: f64) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_fill(out.ptr, n as i32, val, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_repeat_rows(src: &GpuBuffer, src_n: usize, repeats: usize) -> Result<GpuBuffer, HipError> {
      let total = src_n * repeats;
      let out = GpuBuffer::alloc(total)?;
      unsafe { launch_repeat_rows(src.ptr as *const c_void, out.ptr, src_n as i32, total as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_upsample_nearest_2d(input: &GpuBuffer, n: usize, c: usize, h: usize, w: usize, scale_h: usize, scale_w: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n * c * h * scale_h * w * scale_w)?;
      unsafe { launch_upsample_nearest_2d(input.ptr as *const c_void, out.ptr, n as i32, c as i32, h as i32, w as i32, scale_h as i32, scale_w as i32, std::ptr::null_mut()); }
      Ok(out)
}

// ── Reductions ────────────────────────────────────────────────────────────

pub fn gpu_log_sum_exp_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows)?;
      unsafe { launch_log_sum_exp_rows(x.ptr as *const c_void, out.ptr, rows as i32, cols as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_grad_clip_norm(x: &GpuBuffer, n: usize, max_norm: f64) -> Result<(), HipError> {
      let tmp = GpuBuffer::alloc(1)?;
      unsafe { launch_grad_clip_norm(x.ptr, tmp.ptr, n as i32, max_norm, std::ptr::null_mut()); }
      Ok(())
}

pub fn gpu_grad_clip_norm_with_tmp(x: &GpuBuffer, tmp: &GpuBuffer, n: usize, max_norm: f64) {
      unsafe { launch_grad_clip_norm(x.ptr as *mut c_void, tmp.ptr as *mut c_void, n as i32, max_norm, std::ptr::null_mut()); }
}

// ── Prefix sum ────────────────────────────────────────────────────────────

pub fn gpu_prefix_sum_inclusive(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_prefix_sum_inclusive(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_prefix_sum_exclusive(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_prefix_sum_exclusive(x.ptr as *const c_void, out.ptr, n as i32, std::ptr::null_mut()); }
      Ok(out)
}

// ── Tree ──────────────────────────────────────────────────────────────────

pub fn gpu_histogram_build(bins: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, mask: &GpuBuffer, n: usize, p: usize, n_bins: usize) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
      let gh = GpuBuffer::alloc(p * n_bins)?;
      let hh = GpuBuffer::alloc(p * n_bins)?;
      let ch = GpuBuffer::alloc(p * n_bins)?;
      unsafe { launch_histogram_build(bins.ptr as *const c_void, grad.ptr as *const c_void, hess.ptr as *const c_void, mask.ptr as *const c_void, gh.ptr, hh.ptr, ch.ptr, n as i32, p as i32, n_bins as i32, std::ptr::null_mut()); }
      Ok((gh, hh, ch))
}

pub fn gpu_split_eval(gh: &GpuBuffer, hh: &GpuBuffer, p: usize, n_bins: usize, lambda: f64, min_child_weight: f64) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let bg = GpuBuffer::alloc(p)?;
      let bb = GpuBuffer::alloc(p)?;
      unsafe { launch_split_eval(gh.ptr as *const c_void, hh.ptr as *const c_void, bg.ptr, bb.ptr, p as i32, n_bins as i32, lambda, min_child_weight, std::ptr::null_mut()); }
      Ok((bg, bb))
}

pub fn gpu_data_partition(bins: &GpuBuffer, mask: &GpuBuffer, n: usize, p: usize, split_feat: usize, split_bin: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let left = GpuBuffer::alloc(n)?;
      let right = GpuBuffer::alloc(n)?;
      unsafe { launch_data_partition(bins.ptr as *const c_void, mask.ptr as *const c_void, left.ptr, right.ptr, n as i32, p as i32, split_feat as i32, split_bin as i32, std::ptr::null_mut()); }
      Ok((left, right))
}

pub fn gpu_tree_build(tr_bins: &GpuBuffer, te_bins: &GpuBuffer, grad: &GpuBuffer, hess: &GpuBuffer, n_tr: usize, n_te: usize, p: usize, n_bins: usize, max_depth: usize, lambda: f64, min_cw: f64) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let tr_pred = GpuBuffer::alloc(n_tr)?;
      let te_pred = GpuBuffer::alloc(n_te)?;
      unsafe { launch_tree_build(tr_bins.ptr as *const c_void, n_tr as i32, grad.ptr as *const c_void, hess.ptr as *const c_void, te_bins.ptr as *const c_void, n_te as i32, p as i32, n_bins as i32, max_depth as i32, lambda, min_cw, tr_pred.ptr, te_pred.ptr, std::ptr::null_mut()); }
      Ok((tr_pred, te_pred))
}

/// Fused gradient + hessian for multiclass boosting, class k.
/// grad_k = (softmax_k - (target==k)) * weight * subsample_mask
/// hess_k = softmax_k * (1-softmax_k) * weight * subsample_mask, clamped [0.001, 1e6]
/// Returns (grad [n,1], hess [n,1]).
pub fn gpu_grad(probs: &GpuBuffer, targets: &GpuBuffer, weights: &GpuBuffer,
      n: usize, nc: usize, k: usize) -> Result<GpuBuffer, HipError> {
      let pk = gpu_slice_cols(probs, n, nc, k, 1)?;
      let yk = gpu_eq(targets, &gpu_fill(n, k as f64)?, n)?;
      gpu_mul(&gpu_sub(&pk, &yk, n)?, weights, n)
}

pub fn gpu_hessian(probs: &GpuBuffer, weights: &GpuBuffer,
      n: usize, nc: usize, k: usize) -> Result<GpuBuffer, HipError> {
      let pk = gpu_slice_cols(probs, n, nc, k, 1)?;
      let ones = gpu_fill(n, 1.0)?;
      gpu_clamp(&gpu_mul(&gpu_mul(&pk, &gpu_sub(&ones, &pk, n)?, n)?, weights, n)?, n, 0.001, 1e6)
}

/// Update column k of an [n, cols] matrix: out[i, k] = matrix[i, k] + col[i].
/// Returns new matrix (other columns unchanged).
pub fn gpu_add_col(matrix: &GpuBuffer, n: usize, cols: usize, k: usize, col: &GpuBuffer) -> Result<GpuBuffer, HipError> {
      let out = gpu_copy(matrix, n * cols)?;
      // Add col to the k-th column: out[i*cols + k] += col[i]
      // Use scatter_add-style approach: extract col k, add, write back via slice copy
      let old_col = gpu_slice_cols(&out, n, cols, k, 1)?;
      let new_col = gpu_add(&old_col, col, n)?;
      // Write new_col back into column k of out
      // Need a kernel for this — or do it with a D2D strided copy
      // For now: rebuild by concat of slices
      if k == 0 {
            let right = gpu_slice_cols(&out, n, cols, 1, cols - 1)?;
            gpu_concat(&new_col, &right, n, 1, cols - 1)
      } else if k == cols - 1 {
            let left = gpu_slice_cols(&out, n, cols, 0, cols - 1)?;
            gpu_concat(&left, &new_col, n, cols - 1, 1)
      } else {
            let left = gpu_slice_cols(&out, n, cols, 0, k)?;
            let right = gpu_slice_cols(&out, n, cols, k + 1, cols - k - 1)?;
            let tmp = gpu_concat(&left, &new_col, n, k, 1)?;
            gpu_concat(&tmp, &right, n, k + 1, cols - k - 1)
      }
}

/// Balanced accuracy from [n, nc] logits vs integer class labels.
/// Downloads argmax, computes ba on CPU, prints to stderr. Returns ba.
pub fn gpu_report(logits: &GpuBuffer, val_targets: &[i32], n: usize, nc: usize, round: usize) -> Result<f64, HipError> {
      let preds = gpu_argmax_rows(logits, n, nc)?;
      let mut preds_cpu = vec![0.0f64; n];
      preds.download(&mut preds_cpu)?;
      let mut correct = vec![0.0f64; nc];
      let mut total = vec![0.0f64; nc];
      for i in 0..n {
            let c = val_targets[i] as usize;
            total[c] += 1.0;
            if preds_cpu[i] as usize == c { correct[c] += 1.0; }
      }
      let ba: f64 = (0..nc).map(|k| if total[k] > 0.0 { correct[k] / total[k] } else { 0.0 }).sum::<f64>() / nc as f64;
      eprintln!("      r={:4}  val={:.4}", round + 1, ba);
      Ok(ba)
}

// ── DTW ───────────────────────────────────────────────────────────────────

pub fn gpu_dtw(cost: &GpuBuffer, m: usize, n: usize) -> Result<GpuBuffer, HipError> {
      let dp = GpuBuffer::alloc((m + 1) * (n + 1))?;
      unsafe { launch_dtw(cost.ptr as *const c_void, dp.ptr, m as i32, n as i32, std::ptr::null_mut()); }
      Ok(dp)
}

// ── Apriori ───────────────────────────────────────────────────────────────

pub fn gpu_itemset_support(trans: &GpuBuffer, cands: &GpuBuffer, n_trans: usize, n_items: usize, n_cands: usize, k: usize) -> Result<GpuBuffer, HipError> {
      let counts = GpuBuffer::alloc(n_cands)?;
      unsafe { launch_itemset_support(trans.ptr as *const c_void, cands.ptr as *const c_void, counts.ptr, n_trans as i32, n_items as i32, n_cands as i32, k as i32, std::ptr::null_mut()); }
      Ok(counts)
}

pub fn gpu_candidate_generate(freq: &GpuBuffer, n_freq: usize, k: usize) -> Result<(GpuBuffer, usize), HipError> {
      let max_cands = n_freq * (n_freq.saturating_sub(1)) / 2;
      if max_cands == 0 {
            return Ok((GpuBuffer::alloc(1)?, 0));
      }
      let out = GpuBuffer::alloc(max_cands * (k + 1))?;
      let n_generated = unsafe { launch_candidate_generate(freq.ptr as *const c_void, out.ptr, n_freq as i32, k as i32, std::ptr::null_mut()) };
      Ok((out, n_generated as usize))
}

// ── Philox GPU RNG ───────────────────────────────────────────────────────

pub fn gpu_rand_uniform(n: usize, seed: u32) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_rand_uniform(out.ptr, n as i32, seed, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_randn(n: usize, seed: u32) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_randn(out.ptr, n as i32, seed, std::ptr::null_mut()); }
      Ok(out)
}

pub fn gpu_bernoulli(n: usize, p: f64, seed: u32) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_bernoulli(out.ptr, n as i32, p, seed, std::ptr::null_mut()); }
      Ok(out)
}
