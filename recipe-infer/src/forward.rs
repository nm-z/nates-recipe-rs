use crate::enums::{Activation, LayerKind, Loss, Metric};
use crate::params::{ConcatDims, LayerParams, concat_layer};
use crate::scratch::Scratch;
use anyhow::Context;
use gpu_core::kernels;
use ogdl::log::Write;
use gpu_core::memory::GpuBuffer;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub struct LossScale {
	pub sign: f64,
	pub div: f64,
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
	dst: &GpuBuffer,
) -> anyhow::Result<LossScale> {
	let nk = n * k;
	Ok(match m {
		Metric::Loss => match loss {
			Loss::Mse => {
				kernels::gpu_mse_into(out, ybuf, nk, dst)?;
				LossScale {
					sign: 1.0,
					div: 1.0,
				}
			}
			Loss::Mae => {
				kernels::gpu_sub_scale_into(out, ybuf, &sc.c_one, nk, &sc.metric_t0)?;
				kernels::gpu_abs_into(&sc.metric_t0, nk, &sc.metric_t0)?;
				kernels::gpu_reduce_sum_cols_into(
					&sc.metric_t0,
					&sc.reduce_ws,
					nk,
					1,
					dst,
				)?;
				LossScale {
					sign: 1.0,
					div: nk as f64,
				}
			}
			Loss::Huber => {
				kernels::gpu_sub_scale_into(out, ybuf, &sc.c_one, nk, &sc.metric_t0)?;
				kernels::gpu_clamp_into(
					&sc.metric_t0,
					&sc.c_neg_one,
					&sc.c_one,
					nk,
					&sc.metric_t1,
				)?;
				kernels::gpu_copy_into(&sc.metric_t1, nk, &sc.metric_t2)?;
				kernels::gpu_mul_inplace(&sc.metric_t1, nk, &sc.metric_t2)?;
				kernels::gpu_scale_inplace(&sc.c_half, nk, &sc.metric_t2)?;
				kernels::gpu_abs_into(&sc.metric_t0, nk, &sc.metric_t0)?;
				kernels::gpu_add_inplace(&sc.metric_t0, nk, &sc.metric_t2)?;
				kernels::gpu_abs_into(&sc.metric_t1, nk, &sc.metric_t1)?;
				kernels::gpu_sub_inplace(&sc.metric_t1, nk, &sc.metric_t2)?;
				kernels::gpu_reduce_sum_cols_into(
					&sc.metric_t2,
					&sc.reduce_ws,
					nk,
					1,
					dst,
				)?;
				LossScale {
					sign: 1.0,
					div: nk as f64,
				}
			}
			Loss::Ce => {
				kernels::gpu_softmax_rows_into(out, n, k, &sc.metric_t0)?;
				kernels::gpu_clamp_into(
					&sc.metric_t0,
					&sc.c_eps,
					&sc.c_one,
					nk,
					&sc.metric_t0,
				)?;
				kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t0)?;
				kernels::gpu_mul_inplace(ybuf, nk, &sc.metric_t0)?;
				kernels::gpu_reduce_sum_cols_into(
					&sc.metric_t0,
					&sc.reduce_ws,
					nk,
					1,
					dst,
				)?;
				LossScale {
					sign: -1.0,
					div: n as f64,
				}
			}
			Loss::Bce => {
				kernels::gpu_clamp_into(
					out,
					&sc.c_eps,
					&sc.c_one_minus_eps,
					nk,
					&sc.metric_t0,
				)?;
				kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t1)?;
				kernels::gpu_mul_inplace(ybuf, nk, &sc.metric_t1)?;
				kernels::gpu_scale_inplace(&sc.c_neg_one, nk, &sc.metric_t0)?;
				kernels::gpu_add_scalar_inplace(&sc.c_one, nk, &sc.metric_t0)?;
				kernels::gpu_log_into(&sc.metric_t0, nk, &sc.metric_t0)?;
				kernels::gpu_copy_into(ybuf, nk, &sc.metric_t2)?;
				kernels::gpu_scale_inplace(&sc.c_neg_one, nk, &sc.metric_t2)?;
				kernels::gpu_add_scalar_inplace(&sc.c_one, nk, &sc.metric_t2)?;
				kernels::gpu_mul_inplace(&sc.metric_t0, nk, &sc.metric_t2)?;
				kernels::gpu_add_inplace(&sc.metric_t2, nk, &sc.metric_t1)?;
				kernels::gpu_reduce_sum_cols_into(
					&sc.metric_t1,
					&sc.reduce_ws,
					nk,
					1,
					dst,
				)?;
				LossScale {
					sign: -1.0,
					div: nk as f64,
				}
			}
			Loss::Focal => {
				gpu_core::losses::gpu_focal_into(
					out,
					ybuf,
					&sc.c_focal_gamma,
					&sc.c_focal_alpha,
					nk,
					&sc.metric_t0,
					&sc.metric_t1,
				)?;
				kernels::gpu_reduce_sum_cols_into(
					&sc.metric_t0,
					&sc.reduce_ws,
					nk,
					1,
					dst,
				)?;
				LossScale {
					sign: 1.0,
					div: nk as f64,
				}
			}
		},
		Metric::R2 => {
			kernels::gpu_ss_res_into(out, ybuf, nk, dst)?;
			LossScale {
				sign: 1.0,
				div: ss_tot,
			}
		}
		Metric::Accuracy => {
			match k.cmp(&1) {
				Ordering::Equal => kernels::gpu_accuracy_into(out, ybuf, n, dst)?,
				Ordering::Less | Ordering::Greater => {
					kernels::gpu_argmax_accuracy_into(out, ybuf, n, k, dst)?
				}
			}
			LossScale {
				sign: 1.0,
				div: 1.0,
			}
		}
		Metric::Epoch | Metric::Lr | Metric::Time => LossScale {
			sign: 1.0,
			div: 1.0,
		},
	})
}

pub const ZSCORE_EPS: f64 = 1e-8;

pub fn zscore_apply_views(
	xraw: &GpuBuffer,
	n: usize,
	d: usize,
	mean: &GpuBuffer,
	std: &GpuBuffer,
) -> anyhow::Result<GpuBuffer> {
	let xc = GpuBuffer::alloc(n * d).context("center")?;
	kernels::gpu_broadcast_sub_into(xraw, mean, n * d, d, &xc)?;
	let xbuf = GpuBuffer::alloc(n * d).context("scale")?;
	kernels::gpu_broadcast_div(&xc, std, n * d, d, &xbuf)?;
	Ok(xbuf)
}

pub fn zscore_fit_into(
	xraw: &GpuBuffer,
	n: usize,
	d: usize,
	eps: &GpuBuffer,
	mean: &GpuBuffer,
	std: &GpuBuffer,
	out: &GpuBuffer,
) -> anyhow::Result<()> {
	let seg = kernels::gpu_reduce_sum_cols_workspace_bytes(n, d);
	let ws = GpuBuffer::alloc_bytes(((seg + 255) & !255usize) + d * size_of::<f64>()).context("zscore ws")?;
	kernels::gpu_reduce_mean_cols(xraw, n, d, &ws, mean)?;
	let var = GpuBuffer::alloc(d).context("var")?;
	kernels::gpu_reduce_var_cols(xraw, n, d, &ws, &var)?;
	kernels::gpu_add_scalar_inplace(eps, d, &var)?;
	kernels::gpu_sqrt(&var, d, std)?;
	let xc = GpuBuffer::alloc(n * d).context("center")?;
	kernels::gpu_broadcast_sub_into(xraw, mean, n * d, d, &xc)?;
	kernels::gpu_broadcast_div(&xc, std, n * d, d, out)?;
	Ok(())
}

pub fn zscore_apply_into(
	xraw: &GpuBuffer,
	n: usize,
	d: usize,
	mean: &GpuBuffer,
	std: &GpuBuffer,
	out: &GpuBuffer,
) -> anyhow::Result<()> {
	let xc = GpuBuffer::alloc(n * d).context("center")?;
	kernels::gpu_broadcast_sub_into(xraw, mean, n * d, d, &xc)?;
	kernels::gpu_broadcast_div(&xc, std, n * d, d, out)?;
	Ok(())
}

pub struct ZFit {
	pub mean: Vec<f64>,
	pub std: Vec<f64>,
	pub scaled: Vec<f64>,
}

pub fn zscore_fit_host(x: &[f64], n: usize, d: usize) -> ZFit {
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
		let var = *s / nf;
		*s = match var == 0.0 {
			true => 1.0,
			false => (var + ZSCORE_EPS).sqrt(),
		};
	}
	let mut scaled = vec![0.0f64; n * d];
	for i in 0..n {
		for j in 0..d {
			scaled[i * d + j] = (x[i * d + j] - mean[j]) / std[j];
		}
	}
	ZFit { mean, std, scaled }
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
) -> anyhow::Result<()> {
	let cc = concat_layer(params);
	sc.mark_fwd(0);
	for l in 0..params.len() {
		let p = &params[l];
		for ConcatDims { a, c, .. } in cc.filter(|cd| cd.pf == l).into_iter() {
			kernels::gpu_concat_into(
				&acts[l - 1],
				x_cat.ok_or_else(|| anyhow::anyhow!("concat: x_cat missing"))?,
				n,
				a,
				c,
				&sc.concat,
			)?;
		}
		let prev = match l.checked_sub(1) {
			None => x,
			Some(lm1) => match cc.filter(|cd| cd.pf == l) {
				Some(_cd) => &sc.concat,
				None => &acts[lm1],
			},
		};
		match p.kind {
			LayerKind::Embed => {
				kernels::gpu_gather_rows_into(&p.w, prev, n * p.in_dim, p.dim, &acts[l])?;
				kernels::gpu_broadcast_sub_into(
					&acts[l],
					&p.b,
					n * p.out_dim,
					p.out_dim,
					&acts[l],
				)?;
			}
			LayerKind::Attn => match Some(()).filter(|_u| sc.infer) {
				Some(()) => attn_forward_cached(p, prev, &acts[l], n, sc)?,
				None => attn_forward(p, prev, &acts[l], n, sc)?,
			},
			LayerKind::Conv => {
				let cin = p.conv_cin;
				let k = p.conv_k;
				let stride = p.conv_stride;
				let lin = p.in_dim / cin;
				let cout = p.out_dim / ((lin - k) / stride + 1);
				kernels::gpu_conv1d_into(
					prev, &p.w, &p.b, n, cin, lin, cout, k, stride, &acts[l],
				)?;
				let m = n * p.out_dim;
				match Some(()).filter(|_u| {
					matches!(
						p.act,
						Activation::Silu
							| Activation::Gelu | Activation::Elu
							| Activation::Selu | Activation::PRelu
					)
				}) {
					Some(()) => kernels::gpu_copy_into(&acts[l], m, &sc.preact[l]),
					None => Ok(()),
				}?;
				match p.act {
					Activation::Relu => kernels::gpu_relu_into(&acts[l], m, &acts[l]),
					Activation::Sigmoid => {
						kernels::gpu_sigmoid_into(&acts[l], m, &acts[l])
					}
					Activation::LeakyRelu => kernels::gpu_leaky_relu_into(
						&acts[l],
						&sc.c_leaky_alpha,
						m,
						&acts[l],
					),
					Activation::PRelu => kernels::gpu_leaky_relu_into(
						&sc.preact[l],
						&p.palpha,
						m,
						&acts[l],
					),
					Activation::Elu => gpu_core::k_gapact::gpu_elu(
						&sc.preact[l],
						&sc.c_elu_alpha,
						m,
						&acts[l],
					),
					Activation::Selu => gpu_core::k_gapact::gpu_selu(
						&sc.preact[l],
						&sc.c_selu_alpha,
						&sc.c_selu_lambda,
						m,
						&acts[l],
					),
					Activation::Tanh => kernels::gpu_tanh_into(&acts[l], m, &acts[l]),
					Activation::Silu => {
						kernels::gpu_silu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Gelu => {
						kernels::gpu_gelu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Linear => Ok(()),
				}?;
			}
			LayerKind::Dense => {
				match p.out_dim.cmp(&1) {
					Ordering::Equal => kernels::gpu_matvec_bias_into(
						prev, &p.w, &p.b, n, p.in_dim, &acts[l],
					)?,
					Ordering::Less | Ordering::Greater => kernels::gpu_linear_into(
						prev, &p.w, &p.b, n, p.out_dim, p.in_dim, &acts[l],
					)?,
				}
				let m = n * p.out_dim;
				match Some(()).filter(|_u| {
					matches!(
						p.act,
						Activation::Silu
							| Activation::Gelu | Activation::Elu
							| Activation::Selu | Activation::PRelu
					)
				}) {
					Some(()) => kernels::gpu_copy_into(&acts[l], m, &sc.preact[l]),
					None => Ok(()),
				}?;
				match p.act {
					Activation::Relu => kernels::gpu_relu_into(&acts[l], m, &acts[l]),
					Activation::Sigmoid => {
						kernels::gpu_sigmoid_into(&acts[l], m, &acts[l])
					}
					Activation::LeakyRelu => kernels::gpu_leaky_relu_into(
						&acts[l],
						&sc.c_leaky_alpha,
						m,
						&acts[l],
					),
					Activation::PRelu => kernels::gpu_leaky_relu_into(
						&sc.preact[l],
						&p.palpha,
						m,
						&acts[l],
					),
					Activation::Elu => gpu_core::k_gapact::gpu_elu(
						&sc.preact[l],
						&sc.c_elu_alpha,
						m,
						&acts[l],
					),
					Activation::Selu => gpu_core::k_gapact::gpu_selu(
						&sc.preact[l],
						&sc.c_selu_alpha,
						&sc.c_selu_lambda,
						m,
						&acts[l],
					),
					Activation::Tanh => kernels::gpu_tanh_into(&acts[l], m, &acts[l]),
					Activation::Silu => {
						kernels::gpu_silu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Gelu => {
						kernels::gpu_gelu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Linear => Ok(()),
				}?;
			}
		}
		sc.mark_fwd(l + 1);
	}
	Ok(())
}

pub fn attn_forward(
	p: &LayerParams,
	h: &GpuBuffer,
	out: &GpuBuffer,
	n: usize,
	sc: &Scratch,
) -> anyhow::Result<()> {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q)?;
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k)?;
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v)?;
	gpu_core::rope::gpu_rope_qk_heads_inplace(
		&sc.c_one,
		&sc.c_rope_theta,
		m,
		d,
		heads,
		s,
		&sc.a_q,
		&sc.a_k,
	)?;
	kernels::gpu_flash_attention_train_into(
		&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx, &sc.a_lse,
	)?;
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out)?;
	Ok(())
}

pub fn attn_forward_cached(
	p: &LayerParams,
	h: &GpuBuffer,
	out: &GpuBuffer,
	n: usize,
	sc: &Scratch,
) -> anyhow::Result<()> {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q)?;
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k)?;
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v)?;
	gpu_core::rope::gpu_rope_qk_heads_inplace(
		&sc.c_one,
		&sc.c_rope_theta,
		m,
		d,
		heads,
		s,
		&sc.a_q,
		&sc.a_k,
	)?;
	kernels::gpu_flash_attention_into(&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx)?;
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out)?;
	Ok(())
}

pub fn download_vec(buf: &GpuBuffer, len: usize) -> anyhow::Result<Vec<f64>> {
	let mut v = vec![0.0f64; len];
	let dl = buf.download(&mut v);
	if let Err(e) = dl {
		Write::err(format!("gpu download: {e}"))?;
	}
	if let Err(e) = gpu_core::hip::device_synchronize() {
		Write::err(format!("gpu download sync: {e}"))?;
	}
	return Ok(v);
}

pub fn infer_scored(
	params: &[LayerParams],
	xbuf: &GpuBuffer,
	x_cat: Option<&GpuBuffer>,
	n: usize,
	yscaler: Option<YScaler>,
	ybuf: Option<&GpuBuffer>,
	loss: Loss,
	_lr: f64,
	metrics: &[Metric],
	ss_tot: f64,
) -> anyhow::Result<Scored> {
	let last = params.len() - 1;
	let k = params[last].out_dim;
	let consts = {
		let b = GpuBuffer::alloc(crate::scratch::SCRATCH_CONSTS.len())
			.context("scratch consts")?;
		b.load(&crate::scratch::SCRATCH_CONSTS)
			.context("scratch consts")?;
		b
	};
	let sc = Scratch::new_infer(params, n, &consts)?;
	forward_into(params, xbuf, x_cat, n, &sc.acts, &sc)?;
	for YScaler {
		mean: ymean,
		std: ystd,
	} in yscaler.into_iter()
	{
		let ystd_b = {
			let __up = &[ystd];
			let __ub = GpuBuffer::alloc(__up.len()).context("ystd")?;
			__ub.load(__up).context("ystd")?;
			__ub
		};
		let ymean_b = {
			let __up = &[ymean];
			let __ub = GpuBuffer::alloc(__up.len()).context("ymean")?;
			__ub.load(__up).context("ymean")?;
			__ub
		};
		kernels::gpu_scale_inplace(&ystd_b, n * k, &sc.acts[last])?;
		kernels::gpu_add_scalar_inplace(&ymean_b, n * k, &sc.acts[last])?;
	}
	let out = &sc.acts[last];
	let vals: Vec<f64> = match ybuf {
		Some(yb) => {
			let mut acc = Vec::with_capacity(metrics.len());
			for &m in metrics {
				acc.push(match m {
					Metric::Lr | Metric::Epoch | Metric::Time => f64::NAN,
					Metric::Loss | Metric::Accuracy | Metric::R2 => {
						let LossScale { sign, div } = metric_gpu_into(
							loss,
							m,
							out,
							yb,
							&sc,
							n,
							k,
							ss_tot,
							&sc.metric_scalar,
						)?;
						let v = sign * download_scalar(&sc.metric_scalar)? / div;
						match m {
							Metric::R2 => 1.0 - v,
							Metric::Loss
							| Metric::Accuracy
							| Metric::Lr
							| Metric::Epoch
							| Metric::Time => v,
						}
					}
				});
			}
			acc
		}
		None => Vec::new(),
	};
	Ok(Scored {
		preds: download_vec(out, n * k)?,
		vals,
	})
}

#[derive(Clone, Copy)]
pub struct YScaler {
	pub mean: f64,
	pub std: f64,
}

pub struct Scored {
	pub preds: Vec<f64>,
	pub vals: Vec<f64>,
}

pub fn download_scalar(buf: &GpuBuffer) -> anyhow::Result<f64> {
	let mut v = [0.0f64];
	let dl = buf.download(&mut v);
	if let Err(e) = dl {
		Write::err(format!("scalar download: {e}"))?;
	}
	if let Err(e) = gpu_core::hip::device_synchronize() {
		Write::err(format!("scalar download sync: {e}"))?;
	}
	return Ok(v[0]);
}
