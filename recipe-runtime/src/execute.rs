use anyhow::Context;
use gpu_core::kernels;
use gpu_core::memory::{GpuBuffer, Stage};
use ogdl::log::{
	Errored, Flag, Opt, Write, acc, data, device, epoch, gpu, loss, lr, net, prompt, r2, save,
	set_opt, time,
};
use pantry::encode::Dataset;
use crate::memory::{
	LayerParams, PlanMode, SCRATCH_CONSTS, Scaler, Scratch, concat_layer, load_ogdl,
	load_ogdl_str, plan_layer_params,
};
use recipe_ir::{Activation, ConcatDims, LayerKind, LayerSpec, Loss, Metric, Param, Slot, pinned_vocab};
use std::cell::{Cell, RefCell};
use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::Instant;

pub struct MetricFmt {
	pub label: &'static str,
	pub width: usize,
}

pub fn metric_fmt(m: Metric) -> MetricFmt {
	match m {
		Metric::Epoch => MetricFmt {
			label: "epoch",
			width: 5,
		},
		Metric::Lr => MetricFmt {
			label: "lr",
			width: 7,
		},
		Metric::Time => MetricFmt {
			label: "time",
			width: 9,
		},
		Metric::Loss => MetricFmt {
			label: "loss",
			width: 7,
		},
		Metric::Accuracy => MetricFmt {
			label: "acc",
			width: 6,
		},
		Metric::R2 => MetricFmt {
			label: "r2",
			width: 8,
		},
	}
}

pub fn metric_pinned_slot(m: Metric) -> Option<usize> {
	return match m {
		Metric::Loss => Some(0),
		Metric::Accuracy => Some(1),
		Metric::R2 => Some(2),
		Metric::Epoch | Metric::Lr | Metric::Time => None,
	};
}

pub fn metric_render(m: Metric, v: f64) -> String {
	let w = metric_fmt(m).width;
	v.partial_cmp(&v)
		.map_or(format!("{:>w$}", "N/A"), |_finite| match m {
			Metric::Epoch => format!("{:>w$}", v as usize),
			Metric::Time => format!("{:>w$}", fmt_time(v)),
			Metric::Loss | Metric::Accuracy | Metric::Lr | Metric::R2 => {
				format!("{v:>w$.4}")
			}
		})
}

pub fn fmt_time(secs: f64) -> String {
	let s = secs as u64;
	let h = s / 3600;
	let m = (s % 3600) / 60;
	let sec = s % 60;
	match h.checked_sub(1) {
		Some(_hh) => format!("{h}h {m:02}m {sec:02}s"),
		None => match m.checked_sub(1) {
			Some(_mm) => format!("{m}m {sec:02}s"),
			None => format!("{secs:.1}s"),
		},
	}
}

#[derive(Clone, Copy)]
pub struct LossScale {
	pub sign: f64,
	pub div: f64,
}

pub fn metric_gpu_into(
	lossfn: Loss,
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
		Metric::Loss => match lossfn {
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
				std::cmp::Ordering::Equal => kernels::gpu_accuracy_into(out, ybuf, n, dst)?,
				std::cmp::Ordering::Less | std::cmp::Ordering::Greater => {
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

struct ArenaGuard {
	slab: Option<GpuBuffer>,
}

impl Drop for ArenaGuard {
	fn drop(&mut self) {
		for slab in self.slab.take().into_iter() {
			gpu_core::memory::park_run_backing(slab);
		}
	}
}

pub fn detector_forward(input: &[f64], specs: &[LayerSpec]) -> anyhow::Result<Vec<f64>> {
	let n = input.len() / pantry::CONTEXT;
	let saved = load_ogdl_str(pantry::DETECTOR_OGDL)?;
	let plan = plan_layer_params(
		specs,
		pantry::CONTEXT,
		0,
		pantry::VOCAB,
		&saved,
		PlanMode::Warm,
	)
	.map_err(|e| anyhow::anyhow!("detect plan_layer_params: {e}"))?;
	let mut stage = Stage::new();
	let w_off = stage.push(plan.host());
	let consts_off = stage.push(&SCRATCH_CONSTS);
	let x_off = stage.push(input);
	let image = stage.into_host();
	let image_floats = image.len();
	let est = crate::memory::vram_estimate(
		specs,
		n,
		pantry::CONTEXT,
		pantry::N_CLASS,
		pantry::VOCAB,
		0,
		true,
	);
	let need = est + est / 2 + (1 << 20);
	let slab = gpu_core::memory::adopt_run_backing_with_image(need, &image)
		.or_else(|| gpu_core::memory::claim_device_arena_with_image(&image))
		.ok_or_else(|| {
			anyhow::anyhow!(
				"detect: no device backing — footprint {}, claimable {}",
				pantry::data::human_bytes(need),
				pantry::data::human_bytes(gpu_core::memory::claimable_bytes()),
			)
		})?;
	let base = slab.view(0, image_floats);
	let _arena = ArenaGuard { slab: Some(slab) };
	let params = plan.materialize(&base, w_off);
	let xbuf = base.view(x_off, input.len());
	let consts_view = base.view(consts_off, 12);
	let sc = Scratch::new_infer(&params, n, &consts_view)?;
	forward_into(&params, &xbuf, None, n, &sc.acts, &sc)?;
	let last = params.len() - 1;
	let mut preds = vec![0.0f64; n * pantry::N_CLASS];
	let exit = gpu_core::memory::exit_d2h_enqueue_buf(&sc.acts[last], n * pantry::N_CLASS * 8)
		.map_err(|e| anyhow::anyhow!("detect exit d2h enqueue: {e:?}"))?;
	gpu_core::hip::device_synchronize()
		.map_err(|e| anyhow::anyhow!("detect release sync: {e:?}"))?;
	exit.finish(&mut preds);
	return Ok(preds);
}

pub fn infer_scored(
	params: &[LayerParams],
	xbuf: &GpuBuffer,
	x_cat: Option<&GpuBuffer>,
	n: usize,
	yscaler: Option<YScaler>,
	ybuf: Option<&GpuBuffer>,
	lossfn: Loss,
	_lr: f64,
	metrics: &[Metric],
	ss_tot: f64,
) -> anyhow::Result<Scored> {
	let last = params.len() - 1;
	let k = params[last].out_dim;
	let consts = {
		let b = GpuBuffer::alloc(SCRATCH_CONSTS.len())
			.context("scratch consts")?;
		b.load(&SCRATCH_CONSTS)
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
			let mut mvals = Vec::with_capacity(metrics.len());
			for &m in metrics {
				mvals.push(match m {
					Metric::Lr | Metric::Epoch | Metric::Time => f64::NAN,
					Metric::Loss | Metric::Accuracy | Metric::R2 => {
						let LossScale { sign, div } = metric_gpu_into(
							lossfn,
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
			mvals
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

pub static INTERRUPTED: AtomicUsize = AtomicUsize::new(0);

static NEXT_MODEL_ID: AtomicU32 = AtomicU32::new(0);

pub extern "C" fn on_sigint(_sig: i32) {
	let Some(_second) = Some(()).filter(|_probe| INTERRUPTED.swap(1, Ordering::SeqCst) != 0)
	else {
		return;
	};
	gpu_core::sys::exit_now(130);
}

pub struct StepScalars {
	pub neg_lr: GpuBuffer,
	pub inv_n: GpuBuffer,
	pub two_inv_n: GpuBuffer,
	pub zero: GpuBuffer,
}

pub fn loss_grad_into(
	lossfn: Loss,
	out: &GpuBuffer,
	y: &GpuBuffer,
	da: &GpuBuffer,
	n: usize,
	total: usize,
	sc: &Scratch,
	ss: &StepScalars,
) -> anyhow::Result<()> {
	match lossfn {
		Loss::Mse => {
			kernels::gpu_sub_scale_into(out, y, &ss.two_inv_n, total, da).context("mse")?;
		}
		Loss::Mae => {
			kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da).context("mae sub")?;
			kernels::gpu_sign_into(da, total, da).context("mae sign")?;
			kernels::gpu_scale_inplace(&ss.inv_n, total, da).context("mae scale")?;
		}
		Loss::Huber => {
			kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da).context("huber sub")?;
			kernels::gpu_clamp_into(da, &sc.c_neg_one, &sc.c_one, total, da)
				.context("huber clamp")?;
			kernels::gpu_scale_inplace(&ss.inv_n, total, da).context("huber scale")?;
		}
		Loss::Ce => {
			let k = total / n;
			kernels::gpu_softmax_rows_into(out, n, k, da).context("ce softmax")?;
			kernels::gpu_sub_scale_into(da, y, &ss.inv_n, total, da).context("ce grad")?;
		}
		Loss::Bce => {
			kernels::gpu_bce_grad_into(out, y, &ss.inv_n, total, da).context("bce")?;
		}
		Loss::Focal => {
			gpu_core::losses::gpu_focal_grad_into(
				out,
				y,
				&sc.c_focal_gamma,
				&sc.c_focal_alpha,
				&ss.inv_n,
				total,
				da,
			)
			.context("focal")?;
		}
	}
	Ok(())
}

pub struct TrainCfg {
	pub epochs: Slot<usize>,
	pub log_every: Slot<usize>,
	pub metrics: Vec<Metric>,
	pub device: bool,
	pub plot: Vec<Metric>,
	pub resume: Option<String>,
	pub net: Option<crate::transport::Net>,
	pub last: RefCell<LastRun>,
	pub plot_render: Option<fn(&str, &[Vec<f64>], &[recipe_ir::Metric])>,
}

impl TrainCfg {
	pub fn resolved_epochs(&self) -> usize {
		return match self.epochs {
			Slot::Given(n) => n,
			Slot::Unspecified => 1,
		};
	}

	pub fn resolved_log_every(&self) -> usize {
		return match self.log_every {
			Slot::Given(n) => n,
			Slot::Unspecified => 1,
		};
	}
}

pub struct InferCfg {
	pub flags: Vec<Flag>,
	pub last: RefCell<LastRun>,
}

pub struct RunMeta {
	pub target_names: Vec<String>,
	pub raw_test_rows: Option<Vec<Vec<String>>>,
	pub raw_test_headers: Option<Vec<String>>,
	pub fit_ok: bool,
}

pub struct LastRun {
	pub model: *const ModelInner,
	pub score: f64,
	pub loss: f64,
	pub preds: Option<Vec<f64>>,
	pub n: usize,
	pub k: usize,
	pub target_names: Vec<String>,
	pub raw_test_rows: Option<Vec<Vec<String>>>,
	pub raw_test_headers: Option<Vec<String>>,
}

impl Default for LastRun {
	fn default() -> Self {
		LastRun {
			model: ptr::null(),
			score: f64::NAN,
			loss: f64::NAN,
			preds: None,
			n: 0,
			k: 0,
			target_names: Vec::new(),
			raw_test_rows: None,
			raw_test_headers: None,
		}
	}
}

struct ScorePreds {
	score: f64,
	preds: Vec<f64>,
}

struct Rendered {
	text: String,
	neurons: usize,
}

#[doc(hidden)]
pub struct SavedWeights {
	pub text: String,
	pub neurons: usize,
	pub d: usize,
	pub c_cat: usize,
	pub vocab: usize,
}

pub struct ModelInner {
	pub specs: Vec<LayerSpec>,
	pub loss: Cell<Option<Loss>>,
	pub lr: Cell<Option<f64>>,
	pub id: recipe_ir::ObjectId,
	pub objective: recipe_ir::ObjectiveIntent,
	pub lr_intent: Option<f64>,
	pub params: RefCell<Vec<LayerParams>>,
	pub scaler: RefCell<Option<Scaler>>,
	pub yscaler: RefCell<Option<YScaler>>,
	pub fit_score: Cell<f64>,
	pub fit_loss: Cell<f64>,
	pub saved_ogdl: RefCell<Option<SavedWeights>>,
	pub arena_gen: Cell<Option<usize>>,
	pub rebuild_backing: RefCell<Option<GpuBuffer>>,
	pub forward_graph: RefCell<Option<recipe_ir::SemanticGraph>>,
	pub gguf: Option<String>,
}

impl ModelInner {
	pub fn blank() -> ModelInner {
		ModelInner {
			specs: Vec::new(),
			loss: Cell::new(None),
			lr: Cell::new(None),
			id: recipe_ir::ObjectId(NEXT_MODEL_ID.fetch_add(1, Ordering::Relaxed)),
			objective: recipe_ir::ObjectiveIntent::Unspecified,
			lr_intent: None,
			params: RefCell::new(Vec::new()),
			scaler: RefCell::new(None),
			yscaler: RefCell::new(None),
			fit_score: Cell::new(f64::NAN),
			fit_loss: Cell::new(f64::NAN),
			saved_ogdl: RefCell::new(None),
			arena_gen: Cell::new(None),
			rebuild_backing: RefCell::new(None),
			forward_graph: RefCell::new(None),
			gguf: None,
		}
	}

	pub fn resolved_loss(&self) -> Loss {
		let Some(l) = self.loss.get() else {
			Write::error("internal: model loss read before resolution");
			return Loss::Mse;
		};
		return l;
	}

	pub fn resolved_lr(&self) -> f64 {
		let Some(rate) = self.lr.get() else {
			Write::error("internal: model lr read before resolution");
			return 0.01;
		};
		return rate;
	}

	pub fn ensure_params_live(&self) -> anyhow::Result<()> {
		let Some(g) = self.arena_gen.get() else {
			return Ok(());
		};
		if gpu_core::memory::live_parked_gen() == Some(g) {
			return Ok(());
		}
		let params = {
			let mirror = self.saved_ogdl.borrow();
			let m = mirror.as_ref().ok_or_else(|| {
				anyhow::anyhow!(
					"eval: this model's device weights were freed by a later training run and \
					 there is no host mirror to restore them (pooled out-of-core arena run)"
				)
			})?;
			let saved = load_ogdl_str(&m.text).context("eval: parse host weight mirror")?;
			let plan = plan_layer_params(&self.specs, m.d, m.c_cat, m.vocab, &saved, PlanMode::Warm)
				.map_err(|e| anyhow::anyhow!("eval: rebuild weights from mirror: {e}"))?;
			let host = plan.host();
			let staged = GpuBuffer::alloc(host.len().max(1)).context("rebuild staged alloc")?;
			staged.load(host).context("rebuild staged load")?;
			let params = plan.materialize(&staged, 0);
			*self.rebuild_backing.borrow_mut() = Some(staged);
			params
		};
		*self.params.borrow_mut() = params;
		self.arena_gen.set(gpu_core::memory::live_parked_gen());
		Ok(())
	}

	pub fn begin_forward(&self) -> anyhow::Result<Option<GpuBuffer>> {
		let slab = self
			.arena_gen
			.get()
			.and_then(|_gen| gpu_core::memory::adopt_run_backing(0));
		if let Err(e) = self.ensure_params_live() {
			self.end_forward(slab);
			return Err(e);
		}
		return Ok(slab);
	}

	pub fn end_forward(&self, slab: Option<GpuBuffer>) {
		slab.map(|inner_slab| {
			gpu_core::memory::park_run_backing(inner_slab);
			self.arena_gen.set(gpu_core::memory::live_parked_gen());
		})
		.unwrap_or(());
	}
}

pub struct RunProbe {
	pub run_hip: Option<[u64; gpu_core::callspy::N]>,
	pub run_state: [u64; gpu_core::callspy::N],
}

pub fn run_begin(cfg: &TrainCfg) -> RunProbe {
	let run_hip = Some(())
		.filter(|_probe| cfg.device)
		.map(|_hip| gpu_core::callspy::snapshot());
	let run_state = gpu_core::callspy::snapshot();
	RunProbe { run_hip, run_state }
}

pub fn run_train(cfg: &TrainCfg, ds: &Dataset, model: &ModelInner, meta: RunMeta, probe: RunProbe) {
	let RunProbe { run_hip, run_state } = probe;
	set_opt(Opt {
		loss: cfg.metrics.contains(&Metric::Loss),
		acc: cfg.metrics.contains(&Metric::Accuracy),
		epoch: cfg.metrics.contains(&Metric::Epoch),
		lr: cfg.metrics.contains(&Metric::Lr),
		time: cfg.metrics.contains(&Metric::Time),
		r2: cfg.metrics.contains(&Metric::R2),
		device: run_hip.is_some(),
		save: !cfg.metrics.is_empty(),
		prompt: true,
		..Opt::default()
	});
	if !meta.fit_ok {
		Write::error(
			"run: forward-only data — Train is fit-only; use recipe.infer().run(&model).eval(&data)",
		);
		return;
	}
	let conns: Option<Arc<Vec<crate::transport::Conn>>> = match cfg.net.as_ref() {
		Some(wnet) => {
			let cs = match wnet.connect() {
				Ok(v) => v,
				Err(e) => {
					Write::error(format!("net: connect: {e:#}"));
					return;
				}
			};
			for c in &cs {
				Write::line(
					net,
					&format!(
						"net  pooled {} ({} RAM)",
						c.info.arch,
						pantry::data::human_bytes(c.info.ram as usize),
					),
				);
			}
			Some(Arc::new(cs))
		}
		None => None,
	};
	let net_ram: usize = conns.as_ref().map_or(0, |cs| {
		cs.iter()
			.map(|c| (c.info.ram as usize).saturating_sub(crate::memory::USER_GB))
			.sum()
	});
	let facts = crate::resolve::derive_facts(ds);
	match crate::resolve::resolve_model(
		&model.objective,
		model.lr_intent,
		&model.specs,
		&facts,
	) {
		Ok(res) => {
			model.loss.set(Some(res.loss));
			model.lr.set(Some(res.lr));
			for note in &res.notes {
				Write::line(
					data,
					&format!("{}  {} ({})", note.subject, note.chose, note.because),
				);
			}
			if let Some(g) = res.graph {
				model.forward_graph.borrow_mut().replace(g);
			}
		}
		Err(e) => {
			Write::error(e.message());
			return;
		}
	}
	let issues = crate::resolve::preflight(model, ds, net_ram);
	let crate::resolve::Gate::Proceed = crate::resolve::confirm_issues(&issues) else {
		Write::error("aborted");
		return;
	};
	{
		{
			run_hip
				.as_ref()
				.map(|a| {
					Write::line(device, "-- run pre-fit --");
					Write::block(
						device,
						&gpu_core::callspy::report_since(a).serialize(),
					)
				})
				.unwrap_or(());
			let __fit = run_compiled_train(cfg, ds, model);
			if !__fit.is_ok() {
				Write::error(format!(
					"run: fit: {}",
					__fit.as_ref()
						.err()
						.map(|e| format!("{e:#}"))
						.unwrap_or_default()
				));
				model.fit_score.set(f64::NAN);
				let nan_vals: Vec<f64> = vec![f64::NAN; cfg.metrics.len()];
				Write::always(metrics_line(&cfg.metrics, &nan_vals));
			}
			let post_fit = run_hip.map(|_snap| gpu_core::callspy::snapshot());
			Some(())
				.filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) != 0)
				.map(|_flag| Write::error("interrupted"))
				.unwrap_or(());
			let score = model.fit_score.get();
			post_fit
				.as_ref()
				.map(|p| {
					Write::line(device, "-- run post-fit --");
					Write::block(
						device,
						&gpu_core::callspy::report_since(p).serialize(),
					)
				})
				.unwrap_or(());
			model.arena_gen.set(gpu_core::memory::live_parked_gen());
			let mut last = cfg.last.borrow_mut();
			last.model = model as *const ModelInner;
			last.score = score;
			last.loss = model.fit_loss.get();
			last.preds = None;
			last.n = ds.x.nrows();
			last.k = ds.n_targets.max(1);
			last.target_names = meta.target_names;
			last.raw_test_rows = meta.raw_test_rows;
			last.raw_test_headers = meta.raw_test_headers;
			drop(last);
			if let Some((tree, errs)) = gpu_core::callspy::state_report(&run_state) {
				Write::block(device, &tree.serialize());
				for e in errs {
					Write::line(device, &e);
				}
			}
		}
	}
}

unsafe extern "C" {
	fn hipModuleLoadData(module: *mut *mut std::ffi::c_void, image: *const u8) -> i32;
	fn hipModuleGetFunction(
		func: *mut *mut std::ffi::c_void,
		module: *mut std::ffi::c_void,
		name: *const i8,
	) -> i32;
	fn hipModuleLaunchKernel(
		func: *mut std::ffi::c_void,
		grid_x: u32,
		grid_y: u32,
		grid_z: u32,
		block_x: u32,
		block_y: u32,
		block_z: u32,
		shared: u32,
		stream: *mut std::ffi::c_void,
		params: *mut *mut std::ffi::c_void,
		extra: *mut *mut std::ffi::c_void,
	) -> i32;
	fn hipModuleUnload(module: *mut std::ffi::c_void) -> i32;
}

pub fn run_compiled_train(cfg: &TrainCfg, ds: &Dataset, model: &ModelInner) -> anyhow::Result<()> {
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let fit_loss = model.resolved_loss();
	let fit_lr = model.resolved_lr();
	let classify = fit_loss.is_classification();
	let vocab = pinned_vocab(&model.specs).unwrap_or(0);
	let plan = plan_layer_params(&model.specs, d, 0, vocab, &[], PlanMode::Fresh)
		.map_err(|e| anyhow::anyhow!("compiled: plan_layer_params: {e}"))?;
	let dims = plan.dims();
	if dims.is_empty() {
		anyhow::bail!("compiled: model resolves to zero layers");
	}
	let k = plan.out_dim_last().max(1);
	if model.forward_graph.borrow().is_none() {
		let g = crate::graph::build_forward_graph(model.id, &dims, fit_loss);
		model.forward_graph.borrow_mut().replace(g);
	}
	let unit = {
		let gref = model.forward_graph.borrow();
		let graph = gref
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("compiled: forward graph missing"))?;
		crate::compile::generate_train_source(graph, &dims, fit_loss, n, k, fit_lr)
	};
	let target = crate::compile::detect_target()?;
	let artifact = crate::compile::compile(&unit, &target)
		.context("compiled: device program compilation")?;
	Write::line(
		device,
		&format!(
			"compiled  {:?}  {}  {} bytes",
			artifact.route,
			target.arch,
			artifact.code_object.len()
		),
	);

	let wprefix = plan.compiled_weight_prefix();
	let wp = wprefix.len();
	let arena_end = crate::compile::arena_workspace(&dims, n);
	let metric_off = arena_end;
	let x_off = metric_off + 1;
	let y_off = x_off + n * d;
	let pad_off = y_off + n * k;
	let total = pad_off + plan.host().len();

	let x_std = ds.x.as_standard_layout();
	let xsrc = x_std
		.as_slice()
		.ok_or_else(|| anyhow::anyhow!("compiled: x not contiguous"))?;
	let ysrc = ds
		.y
		.as_slice()
		.ok_or_else(|| anyhow::anyhow!("compiled: y not contiguous"))?;

	let mut image = vec![0.0f64; total];
	image[..wp].copy_from_slice(&wprefix);
	let xm = xsrc.len().min(n * d);
	image[x_off..x_off + xm].copy_from_slice(&xsrc[..xm]);
	if classify && k > 1 && ysrc.len() == n {
		for i in 0..n {
			let c = ysrc[i];
			if c.is_finite() && (c as usize) < k {
				image[y_off + i * k + c as usize] = 1.0;
			}
		}
	} else {
		let ym = ysrc.len().min(n * k);
		image[y_off..y_off + ym].copy_from_slice(&ysrc[..ym]);
	}
	image[pad_off..pad_off + plan.host().len()].copy_from_slice(plan.host());

	let need = total * size_of::<f64>();
	let slab = gpu_core::memory::adopt_run_backing_with_image(need, &image)
		.or_else(|| gpu_core::memory::claim_device_arena_with_image(&image))
		.ok_or_else(|| {
			anyhow::anyhow!(
				"compiled: arena claim failed — need {} claimable {}",
				pantry::data::human_bytes(need),
				pantry::data::human_bytes(gpu_core::memory::claimable_bytes()),
			)
		})?;
	let base = slab.view(0, total);

	let entry = std::ffi::CString::new(unit.entry_symbol.as_str())
		.context("compiled: entry symbol nul")?;
	let mut module: *mut std::ffi::c_void = ptr::null_mut();
	let mut func: *mut std::ffi::c_void = ptr::null_mut();
	// SAFETY: code_object is a valid device image and module is a live out-pointer for this call.
	let load_rc = unsafe { hipModuleLoadData(&raw mut module, artifact.code_object.as_ptr()) };
	if load_rc != 0 {
		gpu_core::memory::park_run_backing(slab);
		anyhow::bail!("compiled: hipModuleLoadData rc={load_rc}");
	}
	// SAFETY: module is a loaded module and entry is a valid nul-terminated symbol name.
	let fn_rc = unsafe { hipModuleGetFunction(&raw mut func, module, entry.as_ptr()) };
	if fn_rc != 0 {
		// SAFETY: module is a live loaded module being released after a failed symbol lookup.
		unsafe { hipModuleUnload(module) };
		gpu_core::memory::park_run_backing(slab);
		anyhow::bail!("compiled: hipModuleGetFunction({}) rc={fn_rc}", unit.entry_symbol);
	}

	let ss_tot = {
		let yreg = &image[y_off..y_off + n * k];
		let cnt = yreg.len().max(1) as f64;
		let ybar = yreg.iter().sum::<f64>() / cnt;
		yreg.iter().map(|v| (v - ybar).powi(2)).sum::<f64>()
	};
	let arena_ptr = base.ptr_raw();
	let x_ptr = base.as_ptr_offset(x_off);
	let y_ptr = base.as_ptr_offset(y_off);
	let metric_ptr = base.as_ptr_offset(metric_off);
	let total_epochs = cfg.resolved_epochs().max(1);
	let start = Instant::now();
	let mut done = 0usize;
	let mut last_metric = f64::NAN;
	while done < total_epochs {
		let chunk = (total_epochs - done).min(64);
		let mut a_ptr = arena_ptr;
		let mut xp = x_ptr;
		let mut yp = y_ptr;
		let mut mp = metric_ptr;
		let mut epochs_i = chunk as i32;
		let mut bound_i = chunk as i32;
		let mut kargs: [*mut std::ffi::c_void; 6] = [
			(&raw mut a_ptr).cast(),
			(&raw mut xp).cast(),
			(&raw mut yp).cast(),
			(&raw mut mp).cast(),
			(&raw mut epochs_i).cast(),
			(&raw mut bound_i).cast(),
		];
		// SAFETY: func is a valid kernel; kargs holds one pointer per kernel parameter for a 1x1 launch.
		let rc = unsafe {
			hipModuleLaunchKernel(
				func,
				1,
				1,
				1,
				1,
				1,
				1,
				0,
				ptr::null_mut(),
				kargs.as_mut_ptr(),
				ptr::null_mut(),
			)
		};
		if rc != 0 {
			// SAFETY: module is a live loaded module being released after a failed launch.
			unsafe { hipModuleUnload(module) };
			gpu_core::memory::park_run_backing(slab);
			anyhow::bail!("compiled: hipModuleLaunchKernel rc={rc}");
		}
		gpu_core::hip::device_synchronize().context("compiled: launch sync")?;
		let mut mo = [0.0f64; 1];
		base.view(metric_off, 1)
			.download_host(&mut mo)
			.context("compiled: metric readback")?;
		last_metric = mo[0];
		done += chunk;
		let r2_val = match ss_tot > 0.0 {
			true => 1.0 - last_metric * n as f64 / ss_tot,
			false => f64::NAN,
		};
		let elapsed = start.elapsed().as_secs_f64();
		let vals: Vec<f64> = cfg
			.metrics
			.iter()
			.map(|m| match m {
				Metric::Epoch => done as f64,
				Metric::Lr => fit_lr,
				Metric::Time => elapsed,
				Metric::Loss => last_metric,
				Metric::R2 => r2_val,
				Metric::Accuracy => r2_val,
			})
			.collect();
		metrics_emit("", &cfg.metrics, &vals);
	}

	let final_score = match ss_tot > 0.0 {
		true => 1.0 - last_metric * n as f64 / ss_tot,
		false => f64::NAN,
	};
	let key = fit_loss.score_key();
	let mut wout = vec![0.0f64; wp];
	base.view(0, wp)
		.download_host(&mut wout)
		.context("compiled: weight readback")?;
	let neurons: usize = dims.iter().map(|ld| ld.out_dim).sum();
	*model.saved_ogdl.borrow_mut() = Some(SavedWeights {
		text: plan.compiled_dump_ogdl(&wout, key, final_score)?,
		neurons,
		d,
		c_cat: 0,
		vocab,
	});
	*model.params.borrow_mut() = plan.materialize(&base, pad_off);
	model.fit_score.set(final_score);
	model.fit_loss.set(last_metric);
	// SAFETY: module is a live loaded module being released after all launches completed.
	unsafe { hipModuleUnload(module) };
	gpu_core::memory::park_run_backing(slab);
	return Ok(());
}

pub fn save_ogdl(model: &ModelInner, score: f64, filter: Option<&[Param]>, path: &str) {
	let key = model.resolved_loss().score_key();
	let path = crate::resolve::resolve_path(path);
	let allow = score.is_finite()
		&& !crate::memory::saved_score(&path, key).is_some_and(|best| score <= best);
	let Some(_ok) = Some(()).filter(|_probe| allow) else {
		return;
	};
	let mirror = model.saved_ogdl.borrow();
	let rendered = match mirror.as_ref() {
		Some(m) => Rendered {
			text: m.text.clone(),
			neurons: m.neurons,
		},
		None => {
			let params = model.params.borrow();
			if params.is_empty() {
				Write::error("save: model has no trained params");
				return;
			}
			let Ok(text) = crate::memory::dump_ogdl(&params, filter, key, score) else {
				return;
			};
			Rendered {
				text,
				neurons: params.iter().map(|p| p.out_dim).sum::<usize>(),
			}
		}
	};
	let __wr = crate::memory::write_ogdl(&path, &rendered.text);
	if !__wr.is_ok() {
		Write::error(format!(
			"write model file: {}",
			__wr.as_ref()
				.err()
				.map(|e| format!("{e:#}"))
				.unwrap_or_default()
		));
		return;
	}
	let full = fs::canonicalize(&path).unwrap_or_else(|_err| path.as_str().into());
	Write::line(
		save,
		&format!(
			"saved {} ({} neurons, {key} {score:.4})",
			full.display(),
			rendered.neurons
		),
	);
}

pub fn eval_run(cfg: &InferCfg, ds: &Dataset, model: &ModelInner) {
	let metrics: Vec<Metric> = cfg
		.flags
		.iter()
		.filter_map(|f| match f {
			Flag::loss => Some(Metric::Loss),
			Flag::acc => Some(Metric::Accuracy),
			Flag::epoch => Some(Metric::Epoch),
			Flag::lr => Some(Metric::Lr),
			Flag::time => Some(Metric::Time),
			Flag::r2 => Some(Metric::R2),
			Flag::device => None,
			_other => None,
		})
		.collect();
	let scored = ds.has_target && !metrics.is_empty();
	let eval_loss = match Some(()).filter(|_probe| scored) {
		Some(_metrics) => {
			let facts = crate::resolve::derive_facts(ds);
			match crate::resolve::resolve_model(
				&model.objective,
				model.lr_intent,
				&model.specs,
				&facts,
			) {
				Ok(res) => {
					model.loss.set(Some(res.loss));
					model.lr.set(Some(res.lr));
					res.loss
				}
				Err(e) => {
					Write::error(format!("eval: {}", e.message()));
					return;
				}
			}
		}
		None => model.loss.get().unwrap_or(Loss::Mse),
	};
	let eval_lr = model.lr.get().unwrap_or(0.0);
	let arena = match model.begin_forward() {
		Ok(a) => a,
		Err(e) => {
			Write::error(format!("eval: {e:#}"));
			return;
		}
	};
	let ei = match model.prep_eval_input(ds) {
		Ok(v) => v,
		Err(e) => {
			Write::error(format!("{e:#}"));
			model.end_forward(arena);
			return;
		}
	};
	let params = model.params.borrow();
	if params.is_empty() {
		Write::error("eval: call train first");
		model.end_forward(arena);
		return;
	}
	let eval_dims: Vec<recipe_ir::LayerDims> =
		params.iter().map(recipe_ir::LayerDims::from).collect();
	model.forward_graph.borrow_mut().replace(
		crate::graph::build_forward_graph(model.id, &eval_dims, eval_loss),
	);
	let k = params[params.len() - 1].out_dim;
	let yscaler = *model.yscaler.borrow();
	let sp = match Some(()).filter(|_probe| ds.has_target && !metrics.is_empty()) {
		Some(_scored) => {
			let ybuf = {
				let Some(__up) = ds.y.as_slice() else {
					Write::error("eval: metrics: y contig");
					model.end_forward(arena);
					return;
				};
				let __ub = match GpuBuffer::alloc(__up.len()) {
					Ok(v) => v,
					Err(e) => {
						Write::error(format!("eval: metrics: ybuf: {e:#}"));
						model.end_forward(arena);
						return;
					}
				};
				let __ld = __ub.load(__up);
				if !__ld.is_ok() {
					Write::error(format!(
						"eval: metrics: ybuf: {}",
						__ld.as_ref()
							.err()
							.map(|e| format!("{e:#}"))
							.unwrap_or_default()
					));
					model.end_forward(arena);
					return;
				}
				__ub
			};
			let total = (ei.n * k) as f64;
			let ybar = ds.y.iter().sum::<f64>() / total;
			let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
			let __sc = infer_scored(
				&params,
				&ei.x,
				ei.x_cat.as_ref(),
				ei.n,
				yscaler,
				Some(&ybuf),
				eval_loss,
				eval_lr,
				&metrics,
				ss_tot,
			);
			let sc = match __sc {
				Ok(v) => v,
				Err(e) => {
					Write::error(format!("eval: metrics: {e:#}"));
					model.end_forward(arena);
					return;
				}
			};
				metrics_emit("eval  ", &metrics, &sc.vals);
			let stop = Some(Metric::Accuracy)
				.filter(|_m| eval_loss.is_classification())
				.unwrap_or(Metric::R2);
			let score = (0..metrics.len())
				.find(|mi| metrics[*mi] == stop)
				.map_or(f64::NAN, |mi| sc.vals[mi]);
			ScorePreds {
				score,
				preds: sc.preds,
			}
		}
		None => {
			let __sc = infer_scored(
				&params,
				&ei.x,
				ei.x_cat.as_ref(),
				ei.n,
				yscaler,
				None,
				eval_loss,
				eval_lr,
				&[],
				0.0,
			);
			let sc = match __sc {
				Ok(v) => v,
				Err(e) => {
					Write::error(format!("eval: predictions: {e:#}"));
					model.end_forward(arena);
					return;
				}
			};
			Write::line(
				data,
				&format!(
					"eval: {} samples (no target column, score unavailable)",
					ei.n
				),
			);
			ScorePreds {
				score: f64::NAN,
				preds: sc.preds,
			}
		}
	};
	let mut last = cfg.last.borrow_mut();
	last.model = model as *const ModelInner;
	last.score = sp.score;
	last.preds = Some(sp.preds);
	last.n = ei.n;
	last.k = k;
	drop(last);
	drop(params);
	model.end_forward(arena);
}

pub fn setup_race_ask() -> anyhow::Result<Option<i32>> {
	let Some(it) = env::var_os("SETUP_RACE") else {
		return Ok(None);
	};
	let iters: usize = it
		.to_string_lossy()
		.parse()
		.map_err(|e| Errored::new(format!("SETUP_RACE parse: {e}")))?;
	gpu_core::hip::set_device(0)?;
	for i in 0..iters {
		let x = ndarray::Array2::<f64>::from_elem(ndarray::Ix2(45982, 768), 1.0);
		let cat = ndarray::Array2::<f64>::from_elem(ndarray::Ix2(45982, 128), 1.0);
		let mut stage = gpu_core::memory::Stage::new();
		let x_std = x.as_standard_layout();
		let x_off =
			stage.push(x_std.as_slice().ok_or_else(|| Errored::new("x contig"))?);
		let cat_std = cat.as_standard_layout();
		let cat_off = stage.push(cat_std
			.as_slice()
			.ok_or_else(|| Errored::new("cat contig"))?);
		let host = stage.into_host();
		let staged = gpu_core::memory::GpuBuffer::alloc(host.len().max(1))
			.map_err(|e| Errored::new(format!("setup-race stage: {e}")))?;
		staged.load(&host)
			.map_err(|e| Errored::new(format!("setup-race stage: {e}")))?;
		let xraw = staged.view(x_off, x.len());
		let craw = staged.view(cat_off, cat.len());
		let nn = cat.nrows();
		let cc = cat.ncols();
		let eps = {
			let e = gpu_core::memory::GpuBuffer::alloc(1)
				.map_err(|e| Errored::new(format!("eps: {e}")))?;
			e.load(&[ZSCORE_EPS])
				.map_err(|e| Errored::new(format!("eps load: {e}")))?;
			e
		};
		let mean = gpu_core::memory::GpuBuffer::alloc(cc)
			.map_err(|e| Errored::new(format!("mean: {e}")))?;
		let std = gpu_core::memory::GpuBuffer::alloc(cc)
			.map_err(|e| Errored::new(format!("std: {e}")))?;
		let xb = gpu_core::memory::GpuBuffer::alloc(nn * cc)
			.map_err(|e| Errored::new(format!("zscored: {e}")))?;
		zscore_fit_into(&craw, nn, cc, &eps, &mean, &std, &xb)?;
		let lse = gpu_core::memory::GpuBuffer::alloc(45982 * 3072)
			.map_err(|e| Errored::new(format!("lse: {e}")))?;
		let dsum = gpu_core::memory::GpuBuffer::alloc(45982 * 3072)
			.map_err(|e| Errored::new(format!("dsum: {e}")))?;
		let mut fills = Vec::new();
		while let Some(b) = gpu_core::memory::GpuBuffer::try_alloc_bytes(256 << 20) {
			fills.push(b);
		}
		let mut probe1k = vec![0u8; 1024];
		lse.download_u8(&mut probe1k)
			.map_err(|e| Errored::new(format!("d2h under pressure: {e}")))?;
		drop(xraw);
		drop(craw);
		drop(xb);
		drop(lse);
		drop(dsum);
		drop(fills);
		drop(staged);
		drop(eps);
		drop(mean);
		drop(std);
		gpu_core::memory::pool_trim();
		Write::line(gpu, &format!("setup-race iter {i}: clean"));
	}
	gpu_core::kernels::gpu_shutdown();
	Ok(Some(0))
}

pub fn generate_gguf(path: &str) {
	let prompt_text = env::args().nth(1).unwrap_or_else(|| {
		"The capital of France is".to_string()
	});
	let out = recipe_infer::llm::generate(
		Path::new(path),
		&prompt_text,
		&mut |toks| {
			Write::line(
				prompt,
				recipe_infer::llm::render_toks(toks),
			);
			true
		},
	);
	match out {
		Ok(text) => Write::line(prompt, text),
		Err(e) => Write::error(format!("infer: generate: {e:#}")),
	}
	recipe_infer::shutdown();
}

pub fn load_weights(inner: &ModelInner, weights: &str, d: usize) {
	let saved = match load_ogdl_str(weights) {
		Ok(v) => v,
		Err(e) => {
			Write::error(format!("Model::load: parse weights: {e:#}"));
			return;
		}
	};
	let Some(vocab) = pinned_vocab(&inner.specs) else {
		Write::error(
			"Model::load: first embed layer must pin a fixed vocab (embed(dim).vocab(v))",
		);
		return;
	};
	let plan = match plan_layer_params(&inner.specs, d, 0, vocab, &saved, PlanMode::Warm) {
		Ok(v) => v,
		Err(e) => {
			Write::error(format!("Model::load: plan layer params: {e:#}"));
			return;
		}
	};
	let host = plan.host();
	let staged = match GpuBuffer::alloc(host.len().max(1)) {
		Ok(v) => v,
		Err(e) => {
			Write::error(format!("Model::load staged alloc: {e:#}"));
			return;
		}
	};
	let __sl = staged.load(host);
	if !__sl.is_ok() {
		Write::error(format!(
			"Model::load staged load: {}",
			__sl.as_ref()
				.err()
				.map(|e| format!("{e:#}"))
				.unwrap_or_default()
		));
		return;
	}
	let params = plan.materialize(&staged, 0);
	*inner.rebuild_backing.borrow_mut() = Some(staged);
	*inner.params.borrow_mut() = params;
	*inner.scaler.borrow_mut() = Some(Scaler {
		mean: vec![],
		std: vec![],
	});
	*inner.yscaler.borrow_mut() = None;
}

pub(crate) struct EvalInput {
	pub x: GpuBuffer,
	pub x_cat: Option<GpuBuffer>,
	pub n: usize,
}
pub fn metrics_line(metrics: &[Metric], vals: &[f64]) -> String {
	use std::fmt::Write as _;
	let mut line = String::with_capacity(16 * metrics.len().max(1));
	let mut shown = 0usize;
	for mi in 0..metrics.len().min(vals.len()) {
		let m = metrics[mi];
		let Some(_w) = metric_fmt(m).width.checked_sub(1) else {
			continue;
		};
		let num = metric_render(m, vals[mi]);
		if !line.is_empty() {
			line.push_str("  ");
		}
		let c = ogdl::log::palette(shown);
		shown += 1;
		let _ = write!(
			line,
			"{} \x1b[38;2;{};{};{}m{num}\x1b[0m",
			metric_fmt(m).label,
			c.r,
			c.g,
			c.b
		);
	}
	line
}
pub struct Pending {
	pub epoch: usize,
	pub elapsed: f64,
}

pub fn live_vals(
	metrics: &[Metric],
	live: &[(Metric, usize, usize)],
	ring_scale: &[LossScale],
	ss_tot: f64,
	rate: f64,
	pend: &Pending,
) -> anyhow::Result<Vec<f64>> {
	let mut vals = Vec::with_capacity(metrics.len());
	for m in metrics {
		let v = match m {
			Metric::Epoch => pend.epoch as f64,
			Metric::Lr => rate,
			Metric::Time => pend.elapsed,
			Metric::Loss | Metric::Accuracy | Metric::R2 => {
				match live.iter().find(|&&(lm, _slot, _ri)| lm == *m) {
					None => f64::NAN,
					Some(&(_lm, slot, ri)) => {
						let raw = gpu_core::memory::pinned_read(slot)
							.context("read metric scalar")?;
						match m {
							Metric::R2 => 1.0 - raw / ss_tot,
							Metric::Accuracy => raw,
							_loss => {
								let s = ring_scale[ri];
								s.sign * raw / s.div
							}
						}
					}
				}
			}
		};
		vals.push(v);
	}
	return Ok(vals);
}

pub fn metric_flag(m: Metric) -> ogdl::log::Flag {
	return match m {
		Metric::Loss => loss,
		Metric::Accuracy => acc,
		Metric::Epoch => epoch,
		Metric::Lr => lr,
		Metric::Time => time,
		Metric::R2 => r2,
	};
}
pub fn metrics_emit(prefix: &str, metrics: &[Metric], vals: &[f64]) {
	let o = ogdl::log::opt();
	let shown: Vec<(Metric, f64)> = metrics
		.iter()
		.zip(vals.iter())
		.filter(|(m, _v)| match m {
			Metric::Loss => o.loss,
			Metric::Accuracy => o.acc,
			Metric::Epoch => o.epoch,
			Metric::Lr => o.lr,
			Metric::Time => o.time,
			Metric::R2 => o.r2,
		})
		.map(|(m, v)| (*m, *v))
		.collect();
	let Some((first, _v)) = shown.first() else {
		return;
	};
	let ms: Vec<Metric> = shown.iter().map(|(m, _v)| *m).collect();
	let vs: Vec<f64> = shown.iter().map(|(_m, v)| *v).collect();
	Write::line(
		metric_flag(*first),
		&format!("{prefix}{}", metrics_line(&ms, &vs)),
	);
}
impl ModelInner {
		let graph_dims = plan.dims();
		let graph = crate::graph::build_forward_graph(
			self.id,
			&graph_dims,
			self.resolved_loss(),
		);
		debug_assert!(
			graph
				.fwd_edges
				.windows(2)
				.all(|w| w[0].to == w[1].from),
			"fwd_edges must form a single chain"
		);
		debug_assert!(
			graph.fwd_edges.last().map(|e| e.to) == Some(graph.objectives[0].scalar),
			"fwd chain must end at the loss scalar"
		);
		debug_assert_eq!(
			graph.rev_edges.len(),
			graph_dims.len(),
			"rev_edges count must equal layer count"
		);
		debug_assert!(
			graph
				.params
				.iter()
				.all(|p| graph.ops.iter().any(|o| o.id == p.owner)),
			"every ParamNode owner op id must be valid"
		);
		debug_assert!(
			{
				let (gwf, gwb) = crate::plan::work_from_graph(&graph, n);
				graph_dims.iter().take(3).enumerate().all(|(l, ld)| {
					gwf[l].flop == crate::plan::layer_fwd(ld, n).flop
						&& gwb[l].flop
							== crate::plan::layer_bwd(ld, n, l == 0).flop
				})
			},
			"work_from_graph must reproduce the per-layer work table"
		);
		self.forward_graph.borrow_mut().replace(graph);
		let out_dim = plan.out_dim_last();
		let n_targets = dat.n_targets.max(1);
		let expand_ce = classify && n_targets == 1 && out_dim > 1;
		let k = Some(())
			.filter(|_probe| expand_ce)
			.map(|_probe| out_dim)
			.unwrap_or(n_targets);
		if out_dim != k {
			Write::err(format!(
				"output layer has {out_dim} units but there are {n_targets} target column(s) — make the last .layer({n_targets})"
			))?;
		}
		let yp = {
			let ys =
				dat.y.as_slice()
					.ok_or_else(|| anyhow::anyhow!("train: y contiguous"))?;
			let mut yd = ys.to_vec();
			match Some(()).filter(|_probe| !classify && !rerun) {
				Some(_fresh) => {
					let ymean = yd.iter().sum::<f64>() / yd.len() as f64;
					let yvar = yd.iter().map(|v| (v - ymean).powi(2)).sum::<f64>()
						/ yd.len() as f64;
					let ystd = (yvar + 1e-8).sqrt();
					for v in yd.iter_mut() {
						*v = (*v - ymean) / ystd;
					}
					*self.yscaler.borrow_mut() = Some(YScaler {
						mean: ymean,
						std: ystd,
					});
				}
				None => {
					let ys_opt = *self.yscaler.borrow();
					Some(())
						.filter(|_probe| !classify && rerun)
						.and(ys_opt)
						.map(|ysc| {
							for v in yd.iter_mut() {
								*v = (*v - ysc.mean) / ysc.std;
							}
						})
						.unwrap_or(());
				}
			}
			let total = yd.len() as f64;
			let ybar = yd.iter().sum::<f64>() / total;
			let ss_tot: f64 = yd.iter().map(|v| (v - ybar).powi(2)).sum();
			let y_flat = match Some(()).filter(|_probe| expand_ce) {
				Some(_oh) => {
					let mut oh = vec![0.0f64; n * out_dim];
					for i in 0..yd.len() {
						let idx = yd[i];
						let cc = idx as usize;
						Some(())
							.filter(|_probe| idx.is_finite() && cc < out_dim)
							.map(|_probe| {
								oh[i * out_dim + cc] = 1.0;
							})
							.unwrap_or(());
					}
					oh
				}
				None => yd,
			};
			YPrep { y_flat, ss_tot }
		};
		let ss_tot = yp.ss_tot;
		let epochs = cfg.resolved_epochs().max(1);
		let stop_metric = Some(())
			.filter(|_probe| classify)
			.map(|_probe| Metric::Accuracy)
			.unwrap_or(Metric::R2);
		let mut ring_row: Vec<Metric> = vec![stop_metric];
		let ckpt_loss = Some(())
			.filter(|_probe| checkpointing)
			.map(|_probe| Metric::Loss);
		for m in cfg
			.metrics
			.iter()
			.copied()
			.chain(ckpt_loss)
			.chain(plot_ys.iter().copied())
		{
			let keep = matches!(m, Metric::Loss | Metric::R2 | Metric::Accuracy)
				&& !ring_row.contains(&m);
			ring_row.extend(Some(m).filter(|_probe| keep));
		}
		let n_rows = ring_row.len();
		let live: Vec<(Metric, usize, usize)> = cfg
			.metrics
			.iter()
			.copied()
			.filter_map(|m| {
				let slot = metric_pinned_slot(m)?;
				let ri = ring_row.iter().position(|&d| d == m)?;
				Some((m, slot, ri))
			})
			.collect();
		let streaming = !plotting && !live.is_empty();
		if streaming {
			let slots = live
				.iter()
				.map(|&(_m, slot, _ri)| slot + 1)
				.max()
				.unwrap_or(0);
			gpu_core::memory::pinned_claim(slots).context("pinned metric slots")?;
		}
		let n_timed = 0usize;
		let zf: ZFit = match Some(()).filter(|_probe| d_sc == 0) {
			Some(_empty) => ZFit {
				mean: Vec::new(),
				std: Vec::new(),
				scaled: Vec::new(),
			},
			None => {
				let src = match Some(()).filter(|_probe| embed_first) {
					Some(_ef) => {
						cat.as_ref().ok_or_else(|| anyhow::anyhow!("cat matrix"))?
					}
					None => &xinput,
				};
				let src_std = src.as_standard_layout();
				let sl = src_std
					.as_slice()
					.ok_or_else(|| anyhow::anyhow!("scale src contiguous"))?;
				match Some(()).filter(|_probe| rerun) {
					Some(_re) => {
						let sc_host = self.scaler.borrow();
						let s = sc_host
							.as_ref()
							.ok_or_else(|| anyhow::anyhow!("rerun without scaler"))?;
						if s.mean.len() != d_sc {
							Write::err(format!(
								"rerun: feature count changed: {} vs {d_sc}",
								s.mean.len()
							))?;
						}
						let scaled = zscore_apply_host(
							sl, n, d_sc, &s.mean, &s.std,
						);
						ZFit {
							mean: s.mean.clone(),
							std: s.std.clone(),
							scaled,
						}
					}
					None => zscore_fit_host(sl, n, d_sc),
				}
			}
		};
		let mut stage = Stage::new();
		let w_off = stage.push(plan.host());
		let mut mean_off = 0;
		let mut std_off = 0;
		Some(())
			.filter(|_probe| d_sc != 0)
			.map(|_probe| {
				mean_off = stage.push(&zf.mean);
				std_off = stage.push(&zf.std);
			})
			.unwrap_or(());
		let ring_off = stage.reserve(n_rows * epochs);
		let end_off = stage.reserve(1);
		let w_len = plan.host().len();
		let prefix_len = stage.len_floats();
		let consts_off = stage.push(&SCRATCH_CONSTS);
		let sc_off = stage.push(&[-self.resolved_lr(), 1.0 / n as f64, 2.0 / n as f64, 0.0]);
		let scaled_off = Some(())
			.filter(|_probe| d_sc > 0)
			.map(|_probe| stage.push(&zf.scaled))
			.unwrap_or(0);
		let x_off = match Some(()).filter(|_probe| embed_first) {
			Some(_ef) => {
				let x_std = xinput.as_standard_layout();
				stage.push(x_std
					.as_slice()
					.ok_or_else(|| anyhow::anyhow!("xinput contiguous"))?)
			}
			None => 0,
		};
		let y_off = stage.push(&yp.y_flat);
		let image = stage.into_host();
		let image_floats = image.len();
		let ac_pre = recipe_ir::concat_layer_dims(&plan.dims())
			.map(|d| crate::memory::ConcatAc { a: d.a, c: d.c });
		let need = image_floats * size_of::<f64>() + crate::memory::Ooc::min_bytes(&plan.dims(), n, ac_pre);
		let slab = gpu_core::memory::adopt_run_backing_with_image(need, &image)
			.or_else(|| gpu_core::memory::claim_device_arena_with_image(&image))
			.ok_or_else(|| {
				anyhow::anyhow!(
					"arena: claim failed — image {} claimable {} free {}",
					pantry::data::human_bytes(need),
					pantry::data::human_bytes(gpu_core::memory::claimable_bytes()),
					pantry::data::human_bytes(gpu_core::memory::vram_free_base()),
				)
			})?;
		let base = slab.view(0, image_floats);
		let params = plan.materialize(&base, w_off);
		let last = params.len() - 1;
		let concat_fit_raw = concat_layer(&params);
		let ac_fit = concat_fit_raw
			.as_ref()
			.map(|d| crate::memory::ConcatAc { a: d.a, c: d.c });
		let concat_fit = concat_fit_raw.as_ref().map(|d| crate::memory::ConcatFit {
			pf: d.pf,
			a: d.a,
			c: d.c,
		});
		let consts_view = base.view(consts_off, 12);
		let sc = {
			let _t = gpu_core::memory::tag_scope("scratch");
			Scratch::carve(&params, n, &consts_view, n_timed)?
		};
		let ss = StepScalars {
			neg_lr: base.view(sc_off, 1),
			inv_n: base.view(sc_off + 1, 1),
			two_inv_n: base.view(sc_off + 2, 1),
			zero: base.view(sc_off + 3, 1),
		};
		Some(())
			.filter(|_probe| d_sc == 0 && !rerun)
			.map(|_probe| {
				*self.scaler.borrow_mut() = Some(Scaler {
					mean: vec![],
					std: vec![],
				});
			})
			.unwrap_or(());
		let x_cat = Some(())
			.filter(|_probe| d_sc > 0 && embed_first)
			.map(|_probe| base.view(scaled_off, n * d_sc));
		let xbuf = Some(())
			.filter(|_probe| d_sc > 0)
			.map(|_probe| {
				Some(())
					.filter(|_p| embed_first)
					.map(|_p| base.view(x_off, n * d))
					.unwrap_or_else(|| base.view(scaled_off, n * d_sc))
			})
			.unwrap_or_else(|| base.view(x_off, xinput.len()));
		let ybuf = base.view(y_off, n * k);
		let summary_graph = Some(())
			.filter(|_probe| !cfg.metrics.is_empty())
			.map(|_probe| {
				let neurons: usize = params.iter().map(|p| p.out_dim).sum();
				let out = params[last].out_dim;
				let kv = |name: &str, val: usize| {
					return ogdl::Node::new(
						name.to_owned(),
						vec![ogdl::Node::leaf(&val.to_string())],
					);
				};
				let sect = |name: &str, kids: Vec<ogdl::Node>| {
					return ogdl::Node::new(name.to_owned(), kids);
				};
				ogdl::Node::new(
					String::new(),
					vec![
						sect(
							"arch",
							vec![
								kv("neurons", neurons),
								kv("layers", params.len()),
								kv("samples", n),
								kv("features", d),
								kv("input_dim", d),
								kv("output_dim", out),
							],
						),
						sect(
							"data",
							vec![
								kv("rows", n + 1),
								kv("columns", d + 1),
								kv("predictors", d),
								kv("targets", out),
							],
						),
					],
				)
			});
		Some(())
			.filter(|_probe| !plotting && !rerun)
			.map(|_probe| {
				Some(())
					.filter(|_probe| matches!(did_resume, Resumed::Yes))
					.map(|_probe| {
						resume.map(|path| {
							let full = fs::canonicalize(path)
								.unwrap_or_else(|_err| path.into());
							Write::line(
								save,
								&format!("resumed: {}", full.display()),
							);
						})
						.unwrap_or(())
					})
					.unwrap_or(());
				Some(())
					.filter(|_probe| summary_graph.is_some())
					.map(|_probe| {
						for g in summary_graph.iter() {
							Write::block(data, &g.serialize());
						}
						Write::line(
							gpu,
							&format!(
								"roofline  gemm {} GF/s  vram {} GB/s",
								crate::plan::GEMM_GFLOPS,
								crate::plan::VRAM_GBS
							),
						);
					})
					.unwrap_or(());
			})
			.unwrap_or(());
		let mut ooc = {
			let _t = gpu_core::memory::tag_scope("waterfall");
			let o = crate::memory::Ooc::build(&params, n, ac_fit, conns.clone())?;
			o.report();
			o
		};
		let _guard = gpu_core::memory::AllocGuard::freeze();
		gpu_core::hw::arm_saturation_crash();
		INTERRUPTED.store(0, Ordering::SeqCst);
		gpu_core::sys::on_sigint(on_sigint);
		gpu_core::callspy::mark_loop_start();
		let hip_init = hip_snap.map(|_snap| gpu_core::callspy::snapshot());
		let led_init = hip_snap.map(|_snap| gpu_core::memory::xfer_calls());
		let mut fit_score = f64::NAN;
		let mut fit_loss = f64::NAN;
		let mut epoch_meta: Vec<EpochMeta> = Vec::new();
		let ring_every = checkpointing || plotting;
		let mut ring_scale: Vec<LossScale> = vec![
			LossScale {
				sign: 1.0,
				div: 1.0
			};
			n_rows
		];
		let mut pending: Option<Pending> = None;
		for e in 0..cfg.resolved_epochs() {
			let Some(_go) = Some(()).filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) == 0)
			else {
				break;
			};
			let log_now = cfg.resolved_log_every() > 0
				&& !cfg.metrics.is_empty()
				&& (e % cfg.resolved_log_every() == 0 || e + 1 == cfg.resolved_epochs());
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
			let Some(_go) = Some(()).filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) == 0)
			else {
				break;
			};
			ooc.backward(&params, &xbuf, &ybuf, &sc, &ss, self.resolved_loss(), concat_fit)?;
			let Some(_do) = Some(()).filter(|_probe| log_now || ring_every) else {
				continue;
			};
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
			let Some(_go) = Some(()).filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) == 0)
			else {
				break;
			};
			let out = &sc.acts[last];
			for pend in pending.take().iter() {
				sc.sync_copy_stream();
				let vals = live_vals(&cfg.metrics, &live, &ring_scale, ss_tot, self.resolved_lr(), pend)?;
				metrics_emit("", &cfg.metrics, &vals);
			}
			for mi in 0..ring_row.len() {
				let m = ring_row[mi];
				let slot = base.view(ring_off + mi * epochs + e, 1);
				ring_scale[mi] =
					metric_gpu_into(self.resolved_loss(), m, out, &ybuf, &sc, n, k, ss_tot, &slot)?;
			}
			if streaming && log_now {
				for &(_m, slot, ri) in &live {
					sc.defer_scalar(slot, &base.view(ring_off + ri * epochs + e, 1))
						.context("defer metric scalar")?;
				}
				pending = Some(Pending {
					epoch: e,
					elapsed: start.elapsed().as_secs_f64(),
				});
			}
			epoch_meta.push(EpochMeta {
				epoch: e,
				elapsed: start.elapsed().as_secs_f64(),
				logged: Some(())
					.filter(|_probe| log_now)
					.map(|_probe| Logged::Yes)
					.unwrap_or(Logged::No),
			});
		}
		for pend in pending.take().iter() {
			sc.sync_copy_stream();
			let vals = live_vals(&cfg.metrics, &live, &ring_scale, ss_tot, self.resolved_lr(), pend)?;
			metrics_emit("", &cfg.metrics, &vals);
		}
		let was_interrupted = INTERRUPTED.load(Ordering::SeqCst) != 0;
		Some(())
			.filter(|_probe| !was_interrupted)
			.map(|_probe| -> anyhow::Result<()> {
				ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
				let end_slot = base.view(end_off, 1);
				match Some(()).filter(|_p| classify) {
					Some(_cls) => match Some(()).filter(|_p| k == 1) {
						Some(_bin) => kernels::gpu_accuracy_into(
							&sc.acts[last],
							&ybuf,
							n,
							&end_slot,
						)
						.context("accuracy")?,
						None => kernels::gpu_argmax_accuracy_into(
							&sc.acts[last],
							&ybuf,
							n,
							k,
							&end_slot,
						)
						.context("argmax accuracy")?,
					},
					None => kernels::gpu_ss_res_into(
						&sc.acts[last],
						&ybuf,
						n * k,
						&end_slot,
					)
					.context("ss_res")?,
				};
				Ok(())
			})
			.transpose()?;
		gpu_core::hw::disarm_saturation_crash();
		gpu_core::callspy::mark_loop_end();
		let hip_loop = hip_snap.map(|_snap| gpu_core::callspy::snapshot());
		let led_loop = hip_snap.map(|_snap| gpu_core::memory::xfer_calls());
		drop(_guard);
		gpu_core::sys::reset_sigint();
		drop(ooc);
		let host = self.park(slab, &base, prefix_len, sc)?;
		Some(())
			.filter(|_probe| d_sc > 0 && !rerun)
			.map(|_probe| {
				*self.scaler.borrow_mut() = Some(Scaler {
					mean: host[mean_off..mean_off + d_sc].to_vec(),
					std: host[std_off..std_off + d_sc].to_vec(),
				});
			})
			.unwrap_or(());
		let val_of = |m: Metric, e: usize| -> f64 {
			let Some(mi) = ring_row.iter().position(|&d| d == m) else {
				return f64::NAN;
			};
			let raw = host[ring_off + mi * epochs + e];
			match m {
				Metric::R2 => 1.0 - raw / ss_tot,
				Metric::Accuracy => raw,
				Metric::Loss => {
					let sc = ring_scale[mi];
					sc.sign * raw / sc.div
				}
				_other => f64::NAN,
			}
		};
		let key = self.resolved_loss().score_key();
		let mut loss_prev = f64::INFINITY;
		let mut ckpt_saved: Option<()> = None;
		for meta in &epoch_meta {
			let e = meta.epoch;
			let score = val_of(stop_metric, e);
			fit_score = Some(())
				.filter(|_probe| score.is_finite())
				.map(|_probe| score)
				.unwrap_or(fit_score);
			fit_loss = Some(())
				.filter(|_probe| val_of(Metric::Loss, e).is_finite())
				.map(|_probe| val_of(Metric::Loss, e))
				.unwrap_or(fit_loss);
			let mut checkpointed: Option<()> = None;
			let nan_stop =
				match Some(()).filter(|_probe| checkpointing) {
					Some(_ckpt) => {
						let lossv = val_of(Metric::Loss, e);
						match Some(()).filter(|_probe| lossv.is_nan()) {
							Some(_nan) => {
								Write::error(&format!(
									"NaN loss at epoch {e} — stopping (weights diverged)"
								));
								Halt::Stop
							}
							None => {
								let carve = ckpt_saved.is_none()
									&& e > 0 && lossv > loss_prev;
								Some(())
									.filter(|_probe| carve)
									.map(|_probe| -> anyhow::Result<()> {
										ckpt_saved = Some(());
										let path = checkpoint_path
											.as_ref()
											.ok_or_else(|| {
												anyhow::anyhow!(
													"checkpoint path"
												)
											})?;
										let better = crate::memory::saved_score(
											path, key,
										)
										.is_none_or(|best| score > best);
										Some(())
											.filter(|_p| better)
											.map(|_p| -> anyhow::Result<()> {
												crate::memory::write_ogdl(
												path,
												&plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, score)?,
											)?;
												checkpointed = Some(());
												Ok(())
											})
											.transpose()?;
										Ok(())
									})
									.transpose()?;
								loss_prev = Some(())
									.filter(|_probe| lossv.is_finite())
									.map(|_probe| lossv)
									.unwrap_or(loss_prev);
								Halt::Continue
							}
						}
					}
					None => Halt::Continue,
				};
			let Halt::Continue = nan_stop else {
				break;
			};
			let logged_flag = matches!(meta.logged, Logged::Yes) && !streaming;
			let Some(_go) = Some(())
				.filter(|_probe| (logged_flag || checkpointed.is_some()) && !plotting)
			else {
				continue;
			};
				let vals: Vec<f64> = cfg
					.metrics
					.iter()
					.map(|m| match m {
						Metric::Epoch => e as f64,
						Metric::Lr => self.resolved_lr(),
						Metric::Time => meta.elapsed,
						_other => val_of(*m, e),
					})
					.collect();
				metrics_emit("", &cfg.metrics, &vals);
			for _ck in checkpointed.iter() {
				Write::line(save, "<- checkpoint");
			}
		}
		Some(())
			.filter(|_probe| plotting && !epoch_meta.is_empty())
			.map(|_probe| {
				let plot_rows: Vec<Vec<f64>> = epoch_meta
					.iter()
					.map(|meta| {
						let mut row = vec![meta.elapsed];
						for &m in &plot_ys {
							row.push(match m {
								Metric::Lr => self.resolved_lr(),
								_other => val_of(m, meta.epoch),
							});
						}
						row
					})
					.collect();
				let summary_text = summary_graph
					.as_ref()
					.map(|g| g.serialize())
					.unwrap_or_default();
				if let Some(render) = cfg.plot_render {
					render(&summary_text, &plot_rows, &plot_ys);
				}
			})
			.unwrap_or(());
		let end_val = Some(()).filter(|_probe| !was_interrupted).map(|_probe| {
			Some(())
				.filter(|_p| classify)
				.map(|_p| host[end_off])
				.unwrap_or_else(|| 1.0 - host[end_off] / ss_tot)
		});
		end_val
			.into_iter()
			.filter(|s| s.is_finite())
			.for_each(|s| fit_score = s);
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		*self.saved_ogdl.borrow_mut() = Some(SavedWeights {
			text: plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, fit_score)?,
			neurons,
			d,
			c_cat,
			vocab,
		});
		checkpoint_path
			.as_deref()
			.map(|path| -> anyhow::Result<()> {
				let sw = self.saved_ogdl.borrow();
				let text = &sw
					.as_ref()
					.ok_or_else(|| anyhow::anyhow!("fit leaves a mirror"))?
					.text;
				match Some(()).filter(|_probe| was_interrupted) {
					Some(_intr) => {
						let better_on_disk = crate::memory::saved_score(path, key)
							.is_some_and(|best| {
								!(fit_score.is_finite() && fit_score > best)
							});
						match Some(()).filter(|_probe| better_on_disk) {
							Some(_keep) => Write::line(
								save,
								&format!(
									"keeping {path} (better prior {key} on disk)"
								),
							),
							None => {
								crate::memory::write_ogdl(path, text)?;
								let full = fs::canonicalize(path)
									.unwrap_or_else(|_err| path.into());
								Write::line(
									save,
									&format!(
										"saved {} ({key} {fit_score:.4})",
										full.display()
									),
								);
							}
						}
					}
					None => {
						end_val
							.filter(|s| s.is_finite())
							.filter(|&s| {
								crate::memory::saved_score(path, key)
									.is_none_or(|best| s > best)
							})
							.map(|s| -> anyhow::Result<()> {
								crate::memory::write_ogdl(path, text)?;
								let full = fs::canonicalize(path)
									.unwrap_or_else(|_err| path.into());
								Write::line(
									save,
									&format!(
										"saved {} ({neurons} neurons, {key} {s:.4})",
										full.display()
									),
								);
								Ok(())
							})
							.transpose()?;
					}
				}
				Ok(())
			})
			.transpose()?;
		*self.params.borrow_mut() = params;
		self.fit_score.set(fit_score);
		self.fit_loss.set(fit_loss);
		let hip_dump = || -> Option<()> {
			let b0 = hip_snap?;
			let init = hip_init?;
			let lp = hip_loop?;
			let exit = gpu_core::callspy::snapshot();
			Write::line(device, "-- hip init --");
			Write::block(
				device,
				&gpu_core::callspy::report_between(&b0, &init).serialize(),
			);
			Write::line(device, "-- hip loop --");
			Write::block(
				device,
				&gpu_core::callspy::report_between(&init, &lp).serialize(),
			);
			Write::line(device, "-- hip exit --");
			Write::block(
				device,
				&gpu_core::callspy::report_between(&lp, &exit).serialize(),
			);
			Some(())
		};
		hip_dump();
		let led_dump = || -> Option<()> {
			let s0 = led_snap?;
			let i = led_init?;
			let l = led_loop?;
			let ee = gpu_core::memory::xfer_calls();
			Write::line(
				device,
				format!(
					"-- ledger init -- H2D calls {}  D2H calls {}",
					i.h2d - s0.h2d,
					i.d2h - s0.d2h
				),
			);
			Write::line(
				device,
				format!(
					"-- ledger loop -- H2D calls {}  D2H calls {}",
					l.h2d - i.h2d,
					l.d2h - i.d2h
				),
			);
			Write::line(
				device,
				format!(
					"-- ledger exit -- H2D calls {}  D2H calls {}",
					ee.h2d - l.h2d,
					ee.d2h - l.d2h
				),
			);
			Some(())
		};
		led_dump();
		Ok(())
	}
	fn park(
		&self,
		slab: GpuBuffer,
		src: &GpuBuffer,
		prefix_len: usize,
		sc: Scratch,
	) -> anyhow::Result<Vec<f64>> {
		let mut host = vec![0.0f64; prefix_len];
		let prefix_bytes = prefix_len * size_of::<f64>();
		let inflight = gpu_core::memory::exit_d2h_enqueue_buf(src, prefix_bytes)
			.context("exit prefix d2h enqueue")?;
		gpu_core::hip::device_synchronize().context("exit drain")?;
		drop(sc);
		inflight.finish(&mut host);
		gpu_core::memory::park_run_backing(slab);
		Ok(host)
	}
	pub(crate) fn prep_eval_input(&self, ds: &Dataset) -> anyhow::Result<EvalInput> {
		let n = ds.x.nrows();
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
		let coll = match Some(()).filter(|_probe| embed_cats) {
			Some(_p) => Some(pantry::encode::collapse_onehot(ds)?),
			None => None,
		};
		let eff_x = coll.as_ref().map_or(&ds.x, |c| &c.x);
		let eff_text = match &coll {
			Some(c) => &c.embed_cols,
			None => &ds.text_cols,
		};
		let cat_cols: Vec<usize> = Some(())
			.filter(|_probe| embed_first)
			.map(|_probe| {
				Vec::from_iter((0..eff_x.ncols()).filter(|c| !eff_text.contains(c)))
			})
			.unwrap_or_default();
		let xinput = Some(())
			.filter(|_probe| embed_first)
			.map(|_probe| eff_x.select(ndarray::Axis(1), eff_text))
			.unwrap_or_else(|| eff_x.clone());
		let d = xinput.ncols();
		let scaler = self.scaler.borrow();
		let scaler_ref = scaler
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("eval: missing scaler; train first"))?;
		let up = |m: &ndarray::Array2<f64>| -> anyhow::Result<GpuBuffer> {
			let s = m.as_standard_layout();
			let sl = s
				.as_slice()
				.ok_or_else(|| anyhow::anyhow!("eval upload: non-contiguous"))?;
			let b = GpuBuffer::alloc(sl.len()).context("eval upload")?;
			b.load(sl).context("eval upload")?;
			Ok(b)
		};
		let apply = |xraw: &GpuBuffer, rows: usize, cols: usize, sc: &Scaler| -> anyhow::Result<GpuBuffer> {
			anyhow::ensure!(
				sc.mean.len() == cols && sc.std.len() == cols,
				"eval: feature count changed: mean {} std {} vs {cols}",
				sc.mean.len(),
				sc.std.len()
			);
			let mut st = Stage::new();
			let m_off = st.push(&sc.mean);
			let s_off = st.push(&sc.std);
			let host = st.into_host();
			let img = GpuBuffer::alloc(host.len().max(1)).context("eval scaler stage")?;
			img.load(&host).context("eval scaler stage")?;
			zscore_apply_views(
				xraw,
				rows,
				cols,
				&img.view(m_off, cols),
				&img.view(s_off, cols),
			)
			.context("eval zscore")
		};
		match Some(()).filter(|_probe| embed_first) {
			Some(_ef) => {
				let xraw = up(&xinput)?;
				match Some(()).filter(|_probe| cat_cols.is_empty()) {
					Some(_nocat) => Ok(EvalInput {
						x: xraw,
						x_cat: None,
						n,
					}),
					None => {
						let cat = eff_x.select(ndarray::Axis(1), &cat_cols);
						let craw = up(&cat)?;
						let c = cat.ncols();
						Ok(EvalInput {
							x: xraw,
							x_cat: Some(apply(&craw, n, c, scaler_ref)?),
							n,
						})
					}
				}
			}
			None => {
				let xraw = up(&xinput)?;
				Ok(EvalInput {
					x: apply(&xraw, n, d, scaler_ref)?,
					x_cat: None,
					n,
				})
			}
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
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
					std::cmp::Ordering::Equal => kernels::gpu_matvec_bias_into(
						prev, &p.w, &p.b, n, p.in_dim, &acts[l],
					)?,
					std::cmp::Ordering::Less | std::cmp::Ordering::Greater => {
						kernels::gpu_linear_into(
							prev, &p.w, &p.b, n, p.out_dim, p.in_dim, &acts[l],
						)?
					}
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
