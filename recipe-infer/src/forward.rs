//! The forward path and its supporting GPU ops: host→GPU upload, train-set
//! z-score fit/apply (+ NaN impute), the layer-by-layer forward pass (dense /
//! embed / attn / conv), the KV-cache flash-attention inference path, the
//! single-scalar device→host downloads, and the fused GPU metric reductions.

use crate::enums::{Activation, LayerKind, Loss, Metric};
use crate::params::{LayerParams, Scaler, concat_layer};
use crate::scratch::Scratch;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use std::cell::RefCell;

/// One metric this epoch as a single GPU-reduced scalar, downloading only that
/// scalar (never the n predictions). `out` = output activations (n×1, on GPU);
/// `ss_tot` is precomputed once since the targets are fixed. R²/MSE/accuracy go
/// through fused single-pass kernels (`gpu_ss_res_into`/`gpu_mse_into`/
/// `gpu_accuracy_into`); MAE/Huber/CE go through `_into` variants writing into the
/// preallocated `sc.metric_t*` temporaries — so the whole path allocates nothing.
/// Matches `metric_num` exactly except accuracy differs only at the measure-zero
/// p==0.5 tie (sigmoid outputs never land there).
pub fn metric_gpu(
	loss: Loss,
	lr: f64,
	m: Metric,
	out: &GpuBuffer,
	ybuf: &GpuBuffer,
	sc: &Scratch,
	n: usize,
	k: usize,
	ss_tot: f64,
	epoch: usize,
	elapsed: f64,
) -> f64 {
	let nk = n * k; // element count: n samples × k outputs, flat row-major
	match m {
		// Hip is not a per-epoch scalar — the trainer prints the call-count
		// tree at run end and filters it out of the metric line.
		Metric::Hip => f64::NAN,
		Metric::Epoch => epoch as f64,
		Metric::Lr => lr,
		Metric::Time => elapsed,
		Metric::R2 => {
			kernels::gpu_ss_res_into(out, ybuf, nk, &sc.metric_scalar).expect("ss_res");
			1.0 - sc.read_metric_scalar() / ss_tot
		}
		Metric::Accuracy => {
			if k == 1 {
				kernels::gpu_accuracy_into(out, ybuf, n, &sc.metric_scalar).expect("accuracy");
			} else {
				kernels::gpu_argmax_accuracy_into(out, ybuf, n, k, &sc.metric_scalar).expect("argmax accuracy");
			}
			sc.read_metric_scalar()
		}
		// The Loss metric is the model's ACTUAL loss (self.loss), not hardcoded.
		Metric::Loss => {
			let nf = nk as f64;
			match loss {
				Loss::Mse => {
					kernels::gpu_mse_into(out, ybuf, nk, &sc.metric_scalar).expect("mse");
					sc.read_metric_scalar()
				}
				Loss::Mae => {
					kernels::gpu_sub_scale_into(out, ybuf, &sc.c_one, nk, &sc.metric_t0).expect("sub");
					kernels::gpu_abs_into(&sc.metric_t0, nk, &sc.metric_t0).expect("abs");
					kernels::gpu_reduce_sum_cols_into(
						&sc.metric_t0,
						&sc.reduce_ws,
						nk,
						1,
						&sc.metric_scalar,
					).expect("reduce");
					sc.read_metric_scalar() / nf
				}
				Loss::Huber => {
					// delta=1: 0.5 r² for |r|≤1 else |r|-0.5, written as
					// 0.5·clamp(r,-1,1)² + |r| - |clamp(r,-1,1)|.
					kernels::gpu_sub_scale_into(out, ybuf, &sc.c_one, nk, &sc.metric_t0).expect("sub"); // r
					kernels::gpu_clamp_into(
						&sc.metric_t0,
						&sc.c_neg_one,
						&sc.c_one,
						nk,
						&sc.metric_t1,
					).expect("clamp"); // rc
					kernels::gpu_copy_into(&sc.metric_t1, nk, &sc.metric_t2).expect("copy"); // e = rc
					kernels::gpu_mul_inplace(&sc.metric_t1, nk, &sc.metric_t2).expect("mul"); // e = rc²
					kernels::gpu_scale_inplace(&sc.c_half, nk, &sc.metric_t2).expect("scale"); // e = 0.5 rc²
					kernels::gpu_abs_into(&sc.metric_t0, nk, &sc.metric_t0).expect("abs"); // |r|
					kernels::gpu_add_inplace(&sc.metric_t0, nk, &sc.metric_t2).expect("add"); // e += |r|
					kernels::gpu_abs_into(&sc.metric_t1, nk, &sc.metric_t1).expect("abs"); // |rc|
					kernels::gpu_sub_inplace(&sc.metric_t1, nk, &sc.metric_t2).expect("sub"); // e -= |rc|
					kernels::gpu_reduce_sum_cols_into(
						&sc.metric_t2,
						&sc.reduce_ws,
						nk,
						1,
						&sc.metric_scalar,
					).expect("reduce");
					sc.read_metric_scalar() / nf
				}
				Loss::Ce => {
					// Categorical CE: p = softmax(logits); −Σ y·ln(p) / n. y is
					// one-hot so only the true class contributes per sample.
					kernels::gpu_softmax_rows_into(out, n, k, &sc.metric_t0).expect("softmax"); // p
					kernels::gpu_clamp_into(
						&sc.metric_t0,
						&sc.c_eps,
						&sc.c_one,
						nk,
						&sc.metric_t0,
					).expect("clamp"); // avoid ln(0)
					kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t0).expect("log"); // ln p
					kernels::gpu_mul_inplace(ybuf, nk, &sc.metric_t0).expect("mul"); // y·ln p
					kernels::gpu_reduce_sum_cols_into(
						&sc.metric_t0,
						&sc.reduce_ws,
						nk,
						1,
						&sc.metric_scalar,
					).expect("reduce");
					-sc.read_metric_scalar() / n as f64
				}
				Loss::Bce => {
					kernels::gpu_clamp_into(out, &sc.c_eps, &sc.c_one_minus_eps, nk, &sc.metric_t0).expect("clamp"); // pc
					kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t1).expect("log"); // ln pc
					kernels::gpu_mul_inplace(ybuf, nk, &sc.metric_t1).expect("mul"); // y·ln pc
					kernels::gpu_scale_inplace(&sc.c_neg_one, nk, &sc.metric_t0).expect("scale"); // -pc
					kernels::gpu_add_scalar_inplace(&sc.c_one, nk, &sc.metric_t0).expect("add"); // 1-pc
					kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t0).expect("log"); // ln(1-pc)
					kernels::gpu_copy_into(ybuf, nk, &sc.metric_t2).expect("copy"); // y
					kernels::gpu_scale_inplace(&sc.c_neg_one, nk, &sc.metric_t2).expect("scale"); // -y
					kernels::gpu_add_scalar_inplace(&sc.c_one, nk, &sc.metric_t2).expect("add"); // 1-y
					kernels::gpu_mul_inplace(&sc.metric_t0, nk, &sc.metric_t2).expect("mul"); // (1-y)·ln(1-pc)
					kernels::gpu_add_inplace(&sc.metric_t2, nk, &sc.metric_t1).expect("add"); // sum terms
					kernels::gpu_reduce_sum_cols_into(
						&sc.metric_t1,
						&sc.reduce_ws,
						nk,
						1,
						&sc.metric_scalar,
					).expect("reduce");
					-sc.read_metric_scalar() / nf
				}
				Loss::Focal => {
					// Per-element focal loss (already positive) → mean. t1 is a
					// throwaway sink for the grad the kernel also emits.
					gpu_core::losses::gpu_focal_into(out, ybuf, &sc.c_focal_gamma, &sc.c_focal_alpha, nk, &sc.metric_t0, &sc.metric_t1).expect("focal");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.reduce_ws, nk, 1, &sc.metric_scalar).expect("reduce");
					sc.read_metric_scalar() / nf
				}
			}
		}
	}
}

pub fn metric_gpu_into(
	loss: Loss,
	m: Metric,
	out: &GpuBuffer,
	ybuf: &GpuBuffer,
	sc: &Scratch,
	n: usize,
	k: usize,
	ss_tot: f64,
) -> (f64, f64) {
	let nk = n * k;
	match m {
		Metric::Loss => {
			match loss {
				Loss::Mse => {
					kernels::gpu_mse_into(out, ybuf, nk, &sc.metric_scalar).expect("mse");
					(1.0, 1.0)
				}
				Loss::Mae => {
					kernels::gpu_sub_scale_into(out, ybuf, &sc.c_one, nk, &sc.metric_t0).expect("sub");
					kernels::gpu_abs_into(&sc.metric_t0, nk, &sc.metric_t0).expect("abs");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.reduce_ws, nk, 1, &sc.metric_scalar).expect("reduce");
					(1.0, nk as f64)
				}
				Loss::Huber => {
					kernels::gpu_sub_scale_into(out, ybuf, &sc.c_one, nk, &sc.metric_t0).expect("sub");
					kernels::gpu_clamp_into(&sc.metric_t0, &sc.c_neg_one, &sc.c_one, nk, &sc.metric_t1).expect("clamp");
					kernels::gpu_copy_into(&sc.metric_t1, nk, &sc.metric_t2).expect("copy");
					kernels::gpu_mul_inplace(&sc.metric_t1, nk, &sc.metric_t2).expect("mul");
					kernels::gpu_scale_inplace(&sc.c_half, nk, &sc.metric_t2).expect("scale");
					kernels::gpu_abs_into(&sc.metric_t0, nk, &sc.metric_t0).expect("abs");
					kernels::gpu_add_inplace(&sc.metric_t0, nk, &sc.metric_t2).expect("add");
					kernels::gpu_abs_into(&sc.metric_t1, nk, &sc.metric_t1).expect("abs");
					kernels::gpu_sub_inplace(&sc.metric_t1, nk, &sc.metric_t2).expect("sub");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t2, &sc.reduce_ws, nk, 1, &sc.metric_scalar).expect("reduce");
					(1.0, nk as f64)
				}
				Loss::Ce => {
					kernels::gpu_softmax_rows_into(out, n, k, &sc.metric_t0).expect("softmax");
					kernels::gpu_clamp_into(&sc.metric_t0, &sc.c_eps, &sc.c_one, nk, &sc.metric_t0).expect("clamp");
					kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t0).expect("log");
					kernels::gpu_mul_inplace(ybuf, nk, &sc.metric_t0).expect("mul");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.reduce_ws, nk, 1, &sc.metric_scalar).expect("reduce");
					(-1.0, n as f64)
				}
				Loss::Bce => {
					kernels::gpu_clamp_into(out, &sc.c_eps, &sc.c_one_minus_eps, nk, &sc.metric_t0).expect("clamp");
					kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t1).expect("log");
					kernels::gpu_mul_inplace(ybuf, nk, &sc.metric_t1).expect("mul");
					kernels::gpu_scale_inplace(&sc.c_neg_one, nk, &sc.metric_t0).expect("scale");
					kernels::gpu_add_scalar_inplace(&sc.c_one, nk, &sc.metric_t0).expect("add");
					kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t0).expect("log");
					kernels::gpu_copy_into(ybuf, nk, &sc.metric_t2).expect("copy");
					kernels::gpu_scale_inplace(&sc.c_neg_one, nk, &sc.metric_t2).expect("scale");
					kernels::gpu_add_scalar_inplace(&sc.c_one, nk, &sc.metric_t2).expect("add");
					kernels::gpu_mul_inplace(&sc.metric_t0, nk, &sc.metric_t2).expect("mul");
					kernels::gpu_add_inplace(&sc.metric_t2, nk, &sc.metric_t1).expect("add");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t1, &sc.reduce_ws, nk, 1, &sc.metric_scalar).expect("reduce");
					(-1.0, nk as f64)
				}
				Loss::Focal => {
					gpu_core::losses::gpu_focal_into(out, ybuf, &sc.c_focal_gamma, &sc.c_focal_alpha, nk, &sc.metric_t0, &sc.metric_t1).expect("focal");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.reduce_ws, nk, 1, &sc.metric_scalar).expect("reduce");
					(1.0, nk as f64)
				}
			}
		}
		Metric::R2 => {
			kernels::gpu_ss_res_into(out, ybuf, nk, &sc.metric_scalar).expect("ss_res");
			(1.0, ss_tot)
		}
		_ => (1.0, 1.0),
	}
}

pub fn upload(x: &ndarray::Array2<f64>) -> (GpuBuffer, usize, usize) {
	let std = x.as_standard_layout();
	let slice = std.as_slice().expect("upload: non-contiguous");
	(
		GpuBuffer::upload(slice).expect("upload x"),
		x.nrows(),
		x.ncols(),
	)
}

/// Per-column z-score fit on the TRAIN set; store mean/std in `scaler` (reused
/// verbatim at eval, no leakage) and return the scaled [n×d] buffer.
pub fn zscore_fit(
	xraw: &GpuBuffer,
	n: usize,
	d: usize,
	scaler: &RefCell<Option<Scaler>>,
) -> GpuBuffer {
	// var_cols needs align(sum_ws,256)+d·8 bytes; mean/sum need only sum_ws — size
	// the shared workspace to the larger (var) requirement.
	let seg = kernels::gpu_reduce_sum_cols_workspace_bytes(n, d);
	let ws_bytes = ((seg + 255) & !255usize) + d * 8;
	let ws = GpuBuffer::alloc_bytes(ws_bytes).expect("zscore reduce ws");
	let mean = GpuBuffer::alloc(d).expect("mean");
	kernels::gpu_reduce_mean_cols(xraw, n, d, &ws, &mean).expect("mean");
	let var = GpuBuffer::alloc(d).expect("var");
	kernels::gpu_reduce_var_cols(xraw, n, d, &ws, &var).expect("var");
	let eps = GpuBuffer::upload(&[1e-8]).expect("var eps");
	kernels::gpu_add_scalar_inplace(&eps, d, &var).expect("var eps add");
	let std = GpuBuffer::alloc(d).expect("std");
	kernels::gpu_sqrt(&var, d, &std).expect("std");
	let xc = GpuBuffer::alloc(n * d).expect("center");
	kernels::gpu_broadcast_sub(xraw, &mean, n * d, d, &xc).expect("center");
	let xbuf = GpuBuffer::alloc(n * d).expect("scale");
	kernels::gpu_broadcast_div(&xc, &std, n * d, d, &xbuf).expect("scale");
	*scaler.borrow_mut() = Some(Scaler {
		mean: download_vec(&mean, d),
		std: download_vec(&std, d),
	});
	xbuf
}

pub fn zscore_apply(xraw: &GpuBuffer, n: usize, d: usize, scaler: &Scaler) -> GpuBuffer {
	assert_eq!(scaler.mean.len(), d, "eval: feature count changed");
	assert_eq!(scaler.std.len(), d, "eval: feature count changed");
	let mean = GpuBuffer::upload(&scaler.mean).expect("upload eval mean");
	let std = GpuBuffer::upload(&scaler.std).expect("upload eval std");
	let xc = GpuBuffer::alloc(n * d).expect("eval center");
	kernels::gpu_broadcast_sub(xraw, &mean, n * d, d, &xc).expect("eval center");
	let xbuf = GpuBuffer::alloc(n * d).expect("eval scale");
	kernels::gpu_broadcast_div(&xc, &std, n * d, d, &xbuf).expect("eval scale");
	xbuf
}

/// Forward pass writing each layer's output into the preallocated `acts`
/// (no allocation). The input `x` feeds layer 0 directly (no copy); the
/// activation is applied in place (`acts[l]` holds the pre-activation, then
/// is overwritten by its own activation). `acts[last]` ends as predictions.
pub fn forward_into(
	params: &[LayerParams],
	x: &GpuBuffer,
	x_cat: Option<&GpuBuffer>,
	n: usize,
	acts: &[GpuBuffer],
	sc: &Scratch,
) {
	let cc = concat_layer(params);
	sc.mark_fwd(0);
	for (l, p) in params.iter().enumerate() {
		// The first dense after the text prefix reads concat(attn_out, x_cat):
		// build [n×(A+C)] = [acts[l-1] | x_cat] and feed THAT instead of acts[l-1].
		if let Some((pf, a, c)) = cc
			&& l == pf
		{
			kernels::gpu_concat_into(
				&acts[l - 1],
				x_cat.expect("concat: x_cat missing"),
				n,
				a,
				c,
				&sc.concat,
			).expect("concat");
		}
		let prev = if l == 0 {
			x
		} else if Some(l) == cc.map(|t| t.0) {
			&sc.concat
		} else {
			&acts[l - 1]
		};
		match p.kind {
			LayerKind::Embed => {
				// Each of the in_dim input columns is a token id; gather its
				// dim-vector from the table → [n, in_dim*dim]. Then add the
				// positional encoding (b = -PE [out_dim], so broadcast-SUB adds it).
				kernels::gpu_gather_rows_into(
					&p.w,
					prev,
					n * p.in_dim,
					p.dim,
					&acts[l],
				).expect("gather");
				kernels::gpu_broadcast_sub_into(
					&acts[l],
					&p.b,
					n * p.out_dim,
					p.out_dim,
					&acts[l],
				).expect("pe add");
			}
			LayerKind::Attn => {
				if sc.infer {
					attn_forward_cached(p, prev, &acts[l], n, sc)
				} else {
					attn_forward(p, prev, &acts[l], n, sc)
				}
			}
			LayerKind::Conv => {
				let (cin, k, stride) = (p.conv_cin, p.conv_k, p.conv_stride);
				let lin = p.in_dim / cin;
				let cout = p.out_dim / ((lin - k) / stride + 1);
				kernels::gpu_conv1d_into(
					prev, &p.w, &p.b,
					n, cin, lin, cout, k, stride,
					&acts[l],
				).expect("conv1d");
				let m = n * p.out_dim;
				if matches!(
					p.act,
					Activation::Silu
						| Activation::Gelu | Activation::Elu
						| Activation::Selu | Activation::PRelu
				) {
					kernels::gpu_copy_into(&acts[l], m, &sc.preact[l]).expect("copy preact");
				}
				match p.act {
					Activation::Relu => kernels::gpu_relu_into(&acts[l], m, &acts[l]).expect("relu"),
					Activation::Sigmoid => kernels::gpu_sigmoid_into(&acts[l], m, &acts[l]).expect("sigmoid"),
					Activation::LeakyRelu => kernels::gpu_leaky_relu_into(&acts[l], &sc.c_leaky_alpha, m, &acts[l]).expect("leaky"),
					Activation::PRelu => {
						kernels::gpu_leaky_relu_into(&sc.preact[l], &p.palpha, m, &acts[l]).expect("prelu");
					}
					Activation::Elu => gpu_core::k_gapact::gpu_elu(&sc.preact[l], &sc.c_elu_alpha, m, &acts[l]).expect("elu"),
					Activation::Selu => gpu_core::k_gapact::gpu_selu(&sc.preact[l], &sc.c_selu_alpha, &sc.c_selu_lambda, m, &acts[l]).expect("selu"),
					Activation::Tanh => kernels::gpu_tanh_into(&acts[l], m, &acts[l]).expect("tanh"),
					Activation::Silu => kernels::gpu_silu_into(&sc.preact[l], m, &acts[l]).expect("silu"),
					Activation::Gelu => kernels::gpu_gelu_into(&sc.preact[l], m, &acts[l]).expect("gelu"),
					Activation::Linear => {}
				}
			}
			LayerKind::Dense => {
				// out_dim==1: z = X@w + b is a matrix-vector product. rocBLAS dispatches
				// a full GEMM tile (32×32) for one output column, wasting 31/32 of it —
				// dgemv reads the same operands once and is memory-bound, ~33× faster.
				if p.out_dim == 1 {
					kernels::gpu_matvec_bias_into(
						prev, &p.w, &p.b, &acts[l], n, p.in_dim,
					).expect("matvec");
				} else {
					kernels::gpu_linear_into(
						prev, &p.w, &p.b, n, p.out_dim, p.in_dim, &acts[l],
					).expect("linear");
				}
				let m = n * p.out_dim;
				// Elu/Selu/Silu/Gelu/PRelu backprop from the pre-activation z —
				// save it before the in-place activation overwrites acts[l].
				if matches!(
					p.act,
					Activation::Silu
						| Activation::Gelu | Activation::Elu
						| Activation::Selu | Activation::PRelu
				) {
					kernels::gpu_copy_into(&acts[l], m, &sc.preact[l]).expect("copy preact");
				}
				match p.act {
					Activation::Relu => {
						kernels::gpu_relu_into(&acts[l], m, &acts[l]).expect("relu")
					}
					Activation::Sigmoid => {
						kernels::gpu_sigmoid_into(&acts[l], m, &acts[l]).expect("sigmoid")
					}
					Activation::LeakyRelu => kernels::gpu_leaky_relu_into(
						&acts[l],
						&sc.c_leaky_alpha,
						m,
						&acts[l],
					).expect("leaky"),
					Activation::PRelu => {
						kernels::gpu_leaky_relu_into(
							&sc.preact[l],
							&p.palpha,
							m,
							&acts[l],
						).expect("prelu");
					}
					Activation::Elu => gpu_core::k_gapact::gpu_elu(
						&sc.preact[l],
						&sc.c_elu_alpha,
						m,
						&acts[l],
					).expect("elu"),
					Activation::Selu => gpu_core::k_gapact::gpu_selu(
						&sc.preact[l],
						&sc.c_selu_alpha,
						&sc.c_selu_lambda,
						m,
						&acts[l],
					).expect("selu"),
					Activation::Tanh => {
						kernels::gpu_tanh_into(&acts[l], m, &acts[l]).expect("tanh")
					}
					Activation::Silu => {
						kernels::gpu_silu_into(&sc.preact[l], m, &acts[l]).expect("silu")
					}
					Activation::Gelu => {
						kernels::gpu_gelu_into(&sc.preact[l], m, &acts[l]).expect("gelu")
					}
					Activation::Linear => {}
				}
			}
		}
		sc.mark_fwd(l + 1);
	}
}

/// Bare multi-head self-attention forward. Input `h` = [n, S*d] (d = p.dim,
/// S = in_dim/d, heads = p.heads, hd = d/heads). Q/K/V = H·{Wq,Wk,Wv}; per
/// head scores = Q·Kᵀ/√hd → softmax → context = scores·V; out = context·Wo.
/// Q/K/V/scores/context land in `sc.a_*` for the backward pass. Alloc-free.
pub fn attn_forward(p: &LayerParams, h: &GpuBuffer, out: &GpuBuffer, n: usize, sc: &Scratch) {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q).expect("attn q");
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k).expect("attn k");
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v).expect("attn v");
	// RoPE: rotate Q,K per head by token position before the QK dot product, so
	// scores depend on relative position (the attention op is position-aware).
	// sgn=+1 (forward rotation), theta the fixed base frequency.
	gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.a_q, &sc.a_k, &sc.c_one, &sc.c_rope_theta, m, d, heads, s).expect("rope");
	// Fused flash attention: context in one launch, no L×L buffer anywhere; the
	// per-row logsumexp lands in a_lse so backward can recompute score tiles.
	kernels::gpu_flash_attention_train_into(&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx, &sc.a_lse).expect("flash attn");
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out).expect("attn out");
}

/// Inference-only KV-cache attention. Numerically equal to `attn_forward` (same
/// Q/K/V, same scaled-dot-product softmax over all keys — full bidirectional,
/// matching training) but the score matrix is never materialized: the K,V cache
/// (`a_k`,`a_v`, O(L·d)) is streamed through shared-memory tiles inside a single
/// FlashAttention f64 kernel with an online softmax. Memory is O(L·d), not O(L²);
/// `sc.a_scores` is an unused len-1 stub on this path. No backward state saved.
pub fn attn_forward_cached(p: &LayerParams, h: &GpuBuffer, out: &GpuBuffer, n: usize, sc: &Scratch) {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	// K,V cache (and Q) — all O(L·d), built once.
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q).expect("attn q");
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k).expect("attn k");
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v).expect("attn v");
	// Same RoPE as the training path so cached inference matches it exactly.
	gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.a_q, &sc.a_k, &sc.c_one, &sc.c_rope_theta, m, d, heads, s).expect("rope");
	// Fused attention in one kernel launch — no L×L buffer anywhere.
	kernels::gpu_flash_attention_into(&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx).expect("flash attn");
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out).expect("attn out");
}

/// Copy a GPU buffer of `len` f64s back to host.
pub fn download_vec(buf: &GpuBuffer, len: usize) -> Vec<f64> {
	let mut v = vec![0.0f64; len];
	buf.download(&mut v).expect("gpu download");
	v
}

/// The single forward-only path: built params + prepared input buffers in →
/// (downloaded `n*k` preds, requested metric values) out. Applies the inverse
/// target scaler if `yscaler` is set; scores against `ybuf` on the GPU when
/// present (else returns no metric vals). The model knows how to forward — the
/// caller adapts its Dataset to tensors and chooses metrics. Both `Train::run`'s
/// inference branch and `Model::eval` call this; neither reimplements forward.
#[allow(clippy::too_many_arguments)]
pub fn infer_scored(
	params: &[LayerParams],
	xbuf: &GpuBuffer,
	x_cat: Option<&GpuBuffer>,
	n: usize,
	yscaler: Option<(f64, f64)>,
	ybuf: Option<&GpuBuffer>,
	loss: Loss,
	lr: f64,
	metrics: &[Metric],
	ss_tot: f64,
) -> (Vec<f64>, Vec<f64>) {
	let last = params.len() - 1;
	let k = params[last].out_dim;
	let sc = Scratch::new(params, n, true);
	forward_into(params, xbuf, x_cat, n, &sc.acts, &sc);
	if let Some((ymean, ystd)) = yscaler {
		// One-shot inverse target scaler (single forward, not a loop): upload the
		// two run-specific scalars as device buffers for the scale/shift ops.
		let ystd_b = GpuBuffer::upload(&[ystd]).expect("ystd");
		let ymean_b = GpuBuffer::upload(&[ymean]).expect("ymean");
		kernels::gpu_scale_inplace(&ystd_b, n * k, &sc.acts[last]).expect("y unscale");
		kernels::gpu_add_scalar_inplace(&ymean_b, n * k, &sc.acts[last]).expect("y unshift");
	}
	let out = &sc.acts[last];
	let vals: Vec<f64> = match ybuf {
		Some(yb) => metrics
			.iter()
			.map(|&m| match m {
				Metric::Lr | Metric::Epoch | Metric::Time => f64::NAN,
				_ => metric_gpu(loss, lr, m, out, yb, &sc, n, k, ss_tot, 0, 0.0),
			})
			.collect(),
		None => Vec::new(),
	};
	(download_vec(out, n * k), vals)
}

/// Download a single-element GPU buffer (a reduced scalar) to the host.
pub fn download_scalar(buf: &GpuBuffer) -> f64 {
	let mut v = [0.0f64];
	buf.download(&mut v).expect("scalar download");
	v[0]
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::params::LayerParams;
	use crate::scratch::Scratch;

	// Serialize the GPU tests: concurrent Scratch builds grow the alloc pool from
	// multiple threads, which stochastically wedges the driver (HSA spin → GPU
	// Hang SIGABRT). One test on the device at a time.
	static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

	// One attn layer built directly; helper for the KV-cache tests.
	fn randn(n: usize, seed: usize) -> GpuBuffer {
		let b = GpuBuffer::alloc(n).expect("randn alloc");
		kernels::gpu_randn(n, seed, &b).expect("randn");
		b
	}

	fn attn_layer(n: usize, heads: usize, d: usize, s: usize) -> (Vec<LayerParams>, GpuBuffer) {
		let in_dim = s * d;
		let params = vec![LayerParams {
			kind: LayerKind::Attn,
			w: randn(d * d, 1),
			b: GpuBuffer::upload(&vec![0.0f64; d]).expect("b"),
			in_dim,
			out_dim: in_dim,
			act: Activation::Linear,
			dim: d,
			vocab: 0,
			wk: randn(d * d, 2),
			wv: randn(d * d, 3),
			wo: randn(d * d, 4),
			heads,
			palpha: GpuBuffer::upload(&[0.0f64]).expect("pa"),
			conv_cin: 0, conv_k: 0, conv_stride: 0,
		}];
		(params, randn(n * in_dim, 7))
	}

	// The fused FlashAttention inference kernel must reproduce the training-path
	// full-batch attention to f64 tolerance. Same weights + input, two forwards:
	// forward_only=false uses the full L×L score buffer (attn_forward); forward_only
	// =true runs the kernel-level KV-cache path (attn_forward_cached). S spans many
	// key tiles (FA_BK=64) INCLUDING a partial final tile, exercising the online
	// softmax rescale across tiles.
	#[test]
	fn kv_cache_matches_full_attention() {
		let _g = GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		gpu_core::hip::set_device(0).expect("set_device");
		let (n, heads, d, s) = (2usize, 4usize, 16usize, 1200usize);
		let in_dim = s * d;
		let (params, h) = attn_layer(n, heads, d, s);

		let sc_ref = Scratch::new(&params, n, false);
		assert!(!sc_ref.infer, "ref must use the full-batch path");
		forward_into(&params, &h, None, n, &sc_ref.acts, &sc_ref);
		let reference = download_vec(&sc_ref.acts[0], n * in_dim);
		drop(sc_ref);

		let sc = Scratch::new(&params, n, true);
		assert!(sc.infer, "inference must use the KV-cache path");
		forward_into(&params, &h, None, n, &sc.acts, &sc);
		let cached = download_vec(&sc.acts[0], n * in_dim);

		let (mut maxdiff, mut maxabs) = (0.0f64, 0.0f64);
		for i in 0..reference.len() {
			maxdiff = maxdiff.max((reference[i] - cached[i]).abs());
			maxabs = maxabs.max(reference[i].abs());
		}
		eprintln!(
			"flash-attn equivalence: n={n} heads={heads} d={d} s={s}  maxdiff={maxdiff:e}  maxabs={maxabs:e}"
		);
		assert!(
			maxdiff <= 1e-9 * maxabs.max(1.0),
			"flash-attn output diverged from full attention: maxdiff={maxdiff:e}"
		);
	}

	// A sequence whose full O(L²) score buffer would be multiple GB runs on the
	// fused kernel, which never materializes that buffer (it streams K,V through
	// shared memory). Proves bounded memory AND speed: the full softmax path would
	// need a 2 GB a_scores alloc, the kernel path's whole Scratch is a few MB. Also
	// prints the inference wall-time over a warmed launch.
	#[test]
	fn kv_cache_bounded_memory_long_sequence() {
		let _g = GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		gpu_core::hip::set_device(0).expect("set_device");
		let (n, heads, d, s) = (2usize, 2usize, 16usize, 8192usize);
		let in_dim = s * d;
		let full_scores_bytes = n * heads * s * s * 8;
		let scratch_bytes = Scratch::vram_bytes(&attn_layer(n, heads, d, s).0, n, true);
		eprintln!(
			"flash-attn bounded: S={s}  full a_scores would be {}  whole inference Scratch is {}",
			crate::human_bytes(full_scores_bytes),
			crate::human_bytes(scratch_bytes),
		);
		assert!(full_scores_bytes > 1_000_000_000, "full buffer must be multi-GB to show the contrast");
		assert!(scratch_bytes < full_scores_bytes / 10, "kernel-path memory must be a fraction of the L² buffer");

		let (params, h) = attn_layer(n, heads, d, s);
		let sc = Scratch::new(&params, n, true);
		// Warm up (kernel JIT / allocator), then time a forward to a host sync.
		forward_into(&params, &h, None, n, &sc.acts, &sc);
		let _ = download_vec(&sc.acts[0], 1);
		let t0 = std::time::Instant::now();
		forward_into(&params, &h, None, n, &sc.acts, &sc);
		let out = download_vec(&sc.acts[0], n * in_dim);
		let ms = t0.elapsed().as_secs_f64() * 1e3;
		assert!(out.iter().all(|v| v.is_finite()), "flash-attn output not finite");
		eprintln!("flash-attn bounded: completed in {ms:.2} ms (S={s}, n={n}, heads={heads}, d={d}), out[0]={:.6}", out[0]);
	}

	// The split-K backward weight gradient (dW = inputᵀ·grad, reduced over the
	// batch rows across all CUs) must match rocBLAS's dW to f64 tolerance. It
	// reassociates the reduction (P fixed-order partial sums summed in pass 2), so
	// bit-equality isn't expected — only numerical agreement. Shapes: the profiled
	// skinny output + huge reduction, the out_dim==1 (gemv) case, and a multi-tile
	// output that exercises grid.x > 1.
	#[test]
	fn splitk_dw_matches_rocblas() {
		let _g = GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		gpu_core::hip::set_device(0).expect("set_device");
		eprintln!(
			"split-K uses device multiProcessorCount = {} (queried, not hardcoded)",
			gpu_core::hip::cu_count()
		);
		for &(m, k, n) in &[(4096usize, 42usize, 64usize), (100_000, 42, 1), (777, 130, 96)] {
			let input = randn(m * k, 11);
			let grad = randn(m * n, 22);
			let reference = GpuBuffer::alloc(k * n).expect("ref dw");
			kernels::gpu_gemm_at(&input, &grad, k, n, m, &reference).expect("ref dw");
			let partials =
				GpuBuffer::alloc(kernels::gpu_splitk_dw_partials_elems(m, k, n)).expect("partials");
			let dw = GpuBuffer::alloc(k * n).expect("dw");
			kernels::gpu_splitk_dw_into(&input, &grad, &partials, m, n, k, &dw).expect("splitk dw");
			let r = download_vec(&reference, k * n);
			let g = download_vec(&dw, k * n);
			let (mut maxdiff, mut maxabs) = (0.0f64, 0.0f64);
			for i in 0..r.len() {
				maxdiff = maxdiff.max((r[i] - g[i]).abs());
				maxabs = maxabs.max(r[i].abs());
			}
			eprintln!("split-K dW m={m} k={k} n={n}: maxdiff={maxdiff:e} maxabs={maxabs:e}");
			assert!(
				maxdiff <= 1e-8 * maxabs.max(1.0),
				"split-K dW diverged from rocBLAS: m={m} k={k} n={n} maxdiff={maxdiff:e}"
			);
		}
	}
}
