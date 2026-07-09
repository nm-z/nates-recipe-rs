use crate::enums::{Activation, LayerKind, Loss, Metric};
use crate::params::{LayerParams, concat_layer};
use crate::scratch::Scratch;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;

pub fn metric_gpu_into(
	loss: Loss,
	m: Metric,
	out: &GpuBuffer,
	ybuf: &GpuBuffer,
	sc: &Scratch,
	n: usize,
	k: usize,
	ss_tot: f64,
	dst: &GpuBuffer,
) -> (f64, f64) {
	let nk = n * k;
	match m {
		Metric::Loss => {
			match loss {
				Loss::Mse => {
					kernels::gpu_mse_into(out, ybuf, nk, dst).expect("mse");
					(1.0, 1.0)
				}
				Loss::Mae => {
					kernels::gpu_sub_scale_into(out, ybuf, &sc.c_one, nk, &sc.metric_t0).expect("sub");
					kernels::gpu_abs_into(&sc.metric_t0, nk, &sc.metric_t0).expect("abs");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.reduce_ws, nk, 1, dst).expect("reduce");
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
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t2, &sc.reduce_ws, nk, 1, dst).expect("reduce");
					(1.0, nk as f64)
				}
				Loss::Ce => {
					kernels::gpu_softmax_rows_into(out, n, k, &sc.metric_t0).expect("softmax");
					kernels::gpu_clamp_into(&sc.metric_t0, &sc.c_eps, &sc.c_one, nk, &sc.metric_t0).expect("clamp");
					kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t0).expect("log");
					kernels::gpu_mul_inplace(ybuf, nk, &sc.metric_t0).expect("mul");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.reduce_ws, nk, 1, dst).expect("reduce");
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
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t1, &sc.reduce_ws, nk, 1, dst).expect("reduce");
					(-1.0, nk as f64)
				}
				Loss::Focal => {
					gpu_core::losses::gpu_focal_into(out, ybuf, &sc.c_focal_gamma, &sc.c_focal_alpha, nk, &sc.metric_t0, &sc.metric_t1).expect("focal");
					kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.reduce_ws, nk, 1, dst).expect("reduce");
					(1.0, nk as f64)
				}
			}
		}
		Metric::R2 => {
			kernels::gpu_ss_res_into(out, ybuf, nk, dst).expect("ss_res");
			(1.0, ss_tot)
		}
		Metric::Accuracy => {
			if k == 1 {
				kernels::gpu_accuracy_into(out, ybuf, n, dst).expect("accuracy");
			} else {
				kernels::gpu_argmax_accuracy_into(out, ybuf, n, k, dst).expect("argmax accuracy");
			}
			(1.0, 1.0)
		}
		_ => (1.0, 1.0),
	}
}

pub const ZSCORE_EPS: f64 = 1e-8;

pub fn zscore_apply_views(
	xraw: &GpuBuffer,
	n: usize,
	d: usize,
	mean: &GpuBuffer,
	std: &GpuBuffer,
) -> GpuBuffer {
	let xc = GpuBuffer::alloc(n * d).expect("center");
	kernels::gpu_broadcast_sub_into(xraw, mean, n * d, d, &xc).expect("center");
	let xbuf = GpuBuffer::alloc(n * d).expect("scale");
	kernels::gpu_broadcast_div(&xc, std, n * d, d, &xbuf).expect("scale");
	xbuf
}

pub fn zscore_fit_into(
	xraw: &GpuBuffer,
	n: usize,
	d: usize,
	eps: &GpuBuffer,
	mean: &GpuBuffer,
	std: &GpuBuffer,
	out: &GpuBuffer,
) {
	let seg = kernels::gpu_reduce_sum_cols_workspace_bytes(n, d);
	let ws = GpuBuffer::alloc_bytes(((seg + 255) & !255usize) + d * 8).expect("zscore ws");
	kernels::gpu_reduce_mean_cols(xraw, n, d, &ws, mean).expect("mean");
	let var = GpuBuffer::alloc(d).expect("var");
	kernels::gpu_reduce_var_cols(xraw, n, d, &ws, &var).expect("var");
	kernels::gpu_add_scalar_inplace(eps, d, &var).expect("var eps");
	kernels::gpu_sqrt(&var, d, std).expect("std");
	let xc = GpuBuffer::alloc(n * d).expect("center");
	kernels::gpu_broadcast_sub_into(xraw, mean, n * d, d, &xc).expect("center");
	kernels::gpu_broadcast_div(&xc, std, n * d, d, out).expect("scale");
}

pub fn zscore_apply_into(
	xraw: &GpuBuffer,
	n: usize,
	d: usize,
	mean: &GpuBuffer,
	std: &GpuBuffer,
	out: &GpuBuffer,
) {
	let xc = GpuBuffer::alloc(n * d).expect("center");
	kernels::gpu_broadcast_sub_into(xraw, mean, n * d, d, &xc).expect("center");
	kernels::gpu_broadcast_div(&xc, std, n * d, d, out).expect("scale");
}

pub fn zscore_fit_host(x: &[f64], n: usize, d: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
	let nf = n as f64;
	let mut mean = vec![0.0f64; d];
	for i in 0..n {
		for j in 0..d {
			mean[j] += x[i * d + j];
		}
	}
	for m in mean.iter_mut() {
		*m /= nf;
	}
	let mut std = vec![0.0f64; d];
	for i in 0..n {
		for j in 0..d {
			let dv = x[i * d + j] - mean[j];
			std[j] += dv * dv;
		}
	}
	for s in std.iter_mut() {
		*s = (*s / nf + ZSCORE_EPS).sqrt();
	}
	let mut scaled = vec![0.0f64; n * d];
	for i in 0..n {
		for j in 0..d {
			scaled[i * d + j] = (x[i * d + j] - mean[j]) / std[j];
		}
	}
	(mean, std, scaled)
}

pub fn zscore_apply_host(x: &[f64], n: usize, d: usize, mean: &[f64], std: &[f64]) -> Vec<f64> {
	let mut scaled = vec![0.0f64; n * d];
	for i in 0..n {
		for j in 0..d {
			scaled[i * d + j] = (x[i * d + j] - mean[j]) / std[j];
		}
	}
	scaled
}

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
				if p.out_dim == 1 {
					kernels::gpu_matvec_bias_into(
	prev,
	&p.w,
	&p.b,
	n,
	p.in_dim,
	&acts[l],
).expect("matvec");
				} else {
					kernels::gpu_linear_into(
						prev, &p.w, &p.b, n, p.out_dim, p.in_dim, &acts[l],
					).expect("linear");
				}
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

pub fn attn_forward(p: &LayerParams, h: &GpuBuffer, out: &GpuBuffer, n: usize, sc: &Scratch) {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q).expect("attn q");
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k).expect("attn k");
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v).expect("attn v");
	gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.c_one, &sc.c_rope_theta, m, d, heads, s, &sc.a_q, &sc.a_k).expect("rope");
	kernels::gpu_flash_attention_train_into(&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx, &sc.a_lse).expect("flash attn");
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out).expect("attn out");
}

pub fn attn_forward_cached(p: &LayerParams, h: &GpuBuffer, out: &GpuBuffer, n: usize, sc: &Scratch) {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q).expect("attn q");
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k).expect("attn k");
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v).expect("attn v");
	gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.c_one, &sc.c_rope_theta, m, d, heads, s, &sc.a_q, &sc.a_k).expect("rope");
	kernels::gpu_flash_attention_into(&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx).expect("flash attn");
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out).expect("attn out");
}

pub fn download_vec(buf: &GpuBuffer, len: usize) -> Vec<f64> {
	let mut v = vec![0.0f64; len];
	unsafe { buf.download_async(&mut v, std::ptr::null_mut()) }.expect("gpu download");
	gpu_core::hip::device_synchronize().expect("gpu download sync");
	v
}

#[allow(clippy::too_many_arguments)]
pub fn infer_scored(
	params: &[LayerParams],
	xbuf: &GpuBuffer,
	x_cat: Option<&GpuBuffer>,
	n: usize,
	yscaler: Option<(f64, f64)>,
	ybuf: Option<&GpuBuffer>,
	loss: Loss,
	_lr: f64,
	metrics: &[Metric],
	ss_tot: f64,
) -> (Vec<f64>, Vec<f64>) {
	let last = params.len() - 1;
	let k = params[last].out_dim;
	let consts = {
		let b = GpuBuffer::alloc(crate::scratch::SCRATCH_CONSTS.len()).expect("scratch consts");
		b.load(&crate::scratch::SCRATCH_CONSTS).expect("scratch consts");
		b
	};
	let sc = Scratch::new_infer(params, n, &consts);
	forward_into(params, xbuf, x_cat, n, &sc.acts, &sc);
	if let Some((ymean, ystd)) = yscaler {
		let ystd_b = { let __up = &[ystd]; let __ub = GpuBuffer::alloc(__up.len()).expect("ystd"); __ub.load(__up).expect("ystd"); __ub };
		let ymean_b = { let __up = &[ymean]; let __ub = GpuBuffer::alloc(__up.len()).expect("ymean"); __ub.load(__up).expect("ymean"); __ub };
		kernels::gpu_scale_inplace(&ystd_b, n * k, &sc.acts[last]).expect("y unscale");
		kernels::gpu_add_scalar_inplace(&ymean_b, n * k, &sc.acts[last]).expect("y unshift");
	}
	let out = &sc.acts[last];
	let vals: Vec<f64> = match ybuf {
		Some(yb) => metrics
			.iter()
			.map(|&m| match m {
				Metric::Lr | Metric::Epoch | Metric::Time | Metric::Hip => f64::NAN,
				_ => {
					let (sign, div) =
						metric_gpu_into(loss, m, out, yb, &sc, n, k, ss_tot, &sc.metric_scalar);
					let v = sign * download_scalar(&sc.metric_scalar) / div;
					if m == Metric::R2 { 1.0 - v } else { v }
				}
			})
			.collect(),
		None => Vec::new(),
	};
	(download_vec(out, n * k), vals)
}

pub fn download_scalar(buf: &GpuBuffer) -> f64 {
	let mut v = [0.0f64];
	unsafe { buf.download_async(&mut v, std::ptr::null_mut()) }.expect("scalar download");
	gpu_core::hip::device_synchronize().expect("scalar download sync");
	v[0]
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::params::LayerParams;
	use crate::scratch::Scratch;

	static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

	fn randn(n: usize, seed: usize) -> GpuBuffer {
		let b = GpuBuffer::alloc(n).expect("randn alloc");
		kernels::gpu_randn(n, seed, &b).expect("randn");
		b
	}

	fn consts_buf() -> GpuBuffer {
		let b = GpuBuffer::alloc(crate::scratch::SCRATCH_CONSTS.len()).expect("consts");
		b.load(&crate::scratch::SCRATCH_CONSTS).expect("consts");
		b
	}

	fn attn_layer(n: usize, heads: usize, d: usize, s: usize) -> (Vec<LayerParams>, GpuBuffer) {
		let in_dim = s * d;
		let params = vec![LayerParams {
			kind: LayerKind::Attn,
			w: randn(d * d, 1),
			b: { let __up = &vec![0.0f64; d]; let __ub = GpuBuffer::alloc(__up.len()).expect("b"); __ub.load(__up).expect("b"); __ub },
			in_dim,
			out_dim: in_dim,
			act: Activation::Linear,
			dim: d,
			vocab: 0,
			wk: randn(d * d, 2),
			wv: randn(d * d, 3),
			wo: randn(d * d, 4),
			heads,
			palpha: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("pa"); __ub.load(__up).expect("pa"); __ub },
			conv_cin: 0, conv_k: 0, conv_stride: 0,
		}];
		(params, randn(n * in_dim, 7))
	}

	#[test]
	fn kv_cache_matches_full_attention() {
		let _g = GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		gpu_core::hip::set_device(0).expect("set_device");
		let (n, heads, d, s) = (2usize, 4usize, 16usize, 1200usize);
		let in_dim = s * d;
		let (params, h) = attn_layer(n, heads, d, s);

		let consts = consts_buf();
		let sc_ref = Scratch::new_full(&params, n, &consts);
		assert!(!sc_ref.infer, "ref must use the full-batch path");
		forward_into(&params, &h, None, n, &sc_ref.acts, &sc_ref);
		let reference = download_vec(&sc_ref.acts[0], n * in_dim);
		drop(sc_ref);

		let sc = Scratch::new_infer(&params, n, &consts);
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
		let consts = consts_buf();
		let sc = Scratch::new_infer(&params, n, &consts);
		forward_into(&params, &h, None, n, &sc.acts, &sc);
		let _ = download_vec(&sc.acts[0], 1);
		let t0 = std::time::Instant::now();
		forward_into(&params, &h, None, n, &sc.acts, &sc);
		let out = download_vec(&sc.acts[0], n * in_dim);
		let ms = t0.elapsed().as_secs_f64() * 1e3;
		assert!(out.iter().all(|v| v.is_finite()), "flash-attn output not finite");
		eprintln!("flash-attn bounded: completed in {ms:.2} ms (S={s}, n={n}, heads={heads}, d={d}), out[0]={:.6}", out[0]);
	}

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
