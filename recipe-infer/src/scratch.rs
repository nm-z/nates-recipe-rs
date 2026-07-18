use crate::enums::{Activation, LayerKind, LayerSpec};
use crate::params::{ConcatDims, LayerDims, LayerParams, concat_layer};
use anyhow::Context;
use gpu_core::kernels;
use gpu_core::log::Write;
use gpu_core::memory::GpuBuffer;
use std::cell::Cell;
use std::mem;
use std::ptr;

pub const SCRATCH_CONSTS: [f64; 12] = [
	1.0,
	-1.0,
	0.5,
	1e-7,
	1.0 - 1e-7,
	crate::params::LEAKY_ALPHA,
	crate::params::ELU_ALPHA,
	gpu_core::k_gapact::SELU_ALPHA,
	gpu_core::k_gapact::SELU_LAMBDA,
	crate::params::FOCAL_GAMMA,
	crate::params::FOCAL_ALPHA,
	gpu_core::rope::ROPE_THETA,
];

pub struct Scratch {
	pub acts: Vec<GpuBuffer>,
	pub preact: Vec<GpuBuffer>,
	pub da_a: GpuBuffer,
	pub da_b: GpuBuffer,
	pub dz: GpuBuffer,
	pub dw: GpuBuffer,
	pub dw_partials: GpuBuffer,
	pub db: GpuBuffer,
	pub metric_t0: GpuBuffer,
	pub metric_t1: GpuBuffer,
	pub metric_t2: GpuBuffer,
	pub metric_scalar: GpuBuffer,
	pub metric_scalar_b: GpuBuffer,
	pub c_one: GpuBuffer,
	pub c_neg_one: GpuBuffer,
	pub c_half: GpuBuffer,
	pub c_eps: GpuBuffer,
	pub c_one_minus_eps: GpuBuffer,
	pub c_leaky_alpha: GpuBuffer,
	pub c_elu_alpha: GpuBuffer,
	pub c_selu_alpha: GpuBuffer,
	pub c_selu_lambda: GpuBuffer,
	pub c_focal_gamma: GpuBuffer,
	pub c_focal_alpha: GpuBuffer,
	pub c_rope_theta: GpuBuffer,
	pub reduce_ws: GpuBuffer,
	pub embed_grad: GpuBuffer,
	pub a_q: GpuBuffer,
	pub a_k: GpuBuffer,
	pub a_v: GpuBuffer,
	pub a_ctx: GpuBuffer,
	pub a_lse: GpuBuffer,
	pub a_dctx: GpuBuffer,
	pub a_dq: GpuBuffer,
	pub a_dk: GpuBuffer,
	pub a_dv: GpuBuffer,
	pub a_dsum: GpuBuffer,
	pub a_gw: GpuBuffer,
	pub a_dbias: GpuBuffer,
	pub prelu_t0: GpuBuffer,
	pub prelu_t1: GpuBuffer,
	pub prelu_scalar: GpuBuffer,
	pub concat: GpuBuffer,
	pub concat_dgrad: GpuBuffer,
	pub conv_temp: GpuBuffer,
	pub conv_wg: usize,
	pub infer: bool,
	pub copy_stream: gpu_core::hip::Stream,
	ev_fwd: Vec<Vec<gpu_core::hip::Event>>,
	ev_bwd: Vec<Vec<gpu_core::hip::Event>>,
	timing: Cell<bool>,
	timing_slot: Cell<usize>,
}

pub struct LayerMs {
	pub fwd: Vec<f64>,
	pub bwd: Vec<f64>,
}

pub struct LayerEvents {
	pub fwd: Vec<Vec<gpu_core::hip::Event>>,
	pub bwd: Vec<Vec<gpu_core::hip::Event>>,
}

pub fn layer_ms_from(
	ev_fwd: &[gpu_core::hip::Event],
	ev_bwd: &[gpu_core::hip::Event],
	layers: usize,
) -> LayerMs {
	let f = (0..layers)
		.map(|l| {
			let e = gpu_core::hip::elapsed_ms(&ev_fwd[l], &ev_fwd[l + 1]);
			if !e.is_ok() {
				Write::error(format!(
					"fwd elapsed: {}",
					e.as_ref().err().map(|x| x.to_string()).unwrap_or_default()
				));
				return 0.0;
			}
			e.unwrap_or(0.0) as f64
		})
		.collect();
	let b = (0..layers)
		.map(|l| {
			let e = gpu_core::hip::elapsed_ms(&ev_bwd[l + 1], &ev_bwd[l]);
			if !e.is_ok() {
				Write::error(format!(
					"bwd elapsed: {}",
					e.as_ref().err().map(|x| x.to_string()).unwrap_or_default()
				));
				return 0.0;
			}
			e.unwrap_or(0.0) as f64
		})
		.collect();
	LayerMs { fwd: f, bwd: b }
}

impl Scratch {
	pub fn new_full(
		params: &[LayerParams],
		n: usize,
		consts: &GpuBuffer,
	) -> anyhow::Result<Scratch> {
		Self::new_inner(params, n, false, false, consts, 1)
	}

	pub fn carve(
		params: &[LayerParams],
		n: usize,
		consts: &GpuBuffer,
		n_timed: usize,
	) -> anyhow::Result<Scratch> {
		Self::new_inner(params, n, false, true, consts, n_timed)
	}

	pub fn new_infer(
		params: &[LayerParams],
		n: usize,
		consts: &GpuBuffer,
	) -> anyhow::Result<Scratch> {
		Self::new_inner(params, n, true, false, consts, 0)
	}

	fn new_inner(
		params: &[LayerParams],
		n: usize,
		forward_only: bool,
		light: bool,
		consts: &GpuBuffer,
		n_timed: usize,
	) -> anyhow::Result<Scratch> {
		let bw = move |sz: usize| if forward_only { 1 } else { sz };
		let lt = move |sz: usize| if light { 1 } else { sz };
		let alloc = |sz: usize, label: &str| -> anyhow::Result<GpuBuffer> {
			GpuBuffer::alloc(sz).map_err(|e| {
				anyhow::anyhow!(
					"{label}: GPU alloc of {} ({sz} × f64) failed — {e:?}",
					crate::human_bytes(sz * size_of::<f64>())
				)
			})
		};
		let mut max_ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1);
		let mut max_act = 0usize;
		let mut max_wt = 0usize;
		let mut max_bias = 0usize;
		let mut max_embed_grad = 1usize;
		let mut max_seqd = 1usize;
		let mut max_lse = 1usize;
		let mut max_dd = 1usize;
		let mut max_dw_partials = 1usize;
		let mut has_prelu = false;
		for p in params {
			let dw_dp = match p.kind {
				LayerKind::Dense => {
					kernels::gpu_splitk_dw_partials_elems(n, p.in_dim, p.out_dim)
				}
				LayerKind::Attn => {
					let s = p.in_dim / p.dim;
					kernels::gpu_splitk_dw_partials_elems(n * s, p.dim, p.dim)
				}
				_ => 0,
			};
			if dw_dp > max_dw_partials {
				max_dw_partials = dw_dp;
			}
			if p.act == Activation::PRelu {
				has_prelu = true;
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * p.out_dim, 1);
				if ws > max_ws {
					max_ws = ws;
				}
			}
			let w = kernels::gpu_reduce_sum_cols_workspace_bytes(n, p.out_dim);
			if w > max_ws {
				max_ws = w;
			}
			let a = n * p.out_dim.max(p.in_dim);
			if a > max_act {
				max_act = a;
			}
			let wt = match p.kind {
				LayerKind::Conv => {
					let lin = p.in_dim / p.conv_cin;
					let lout = (lin - p.conv_k) / p.conv_stride + 1;
					let cout = p.out_dim / lout;
					cout * p.conv_cin * p.conv_k
				}
				LayerKind::Dense => p.in_dim * p.out_dim,
				_ => 0,
			};
			if wt > max_wt {
				max_wt = wt;
			}
			let bias_sz = if p.kind == LayerKind::Conv {
				let lin = p.in_dim / p.conv_cin;
				let lout = (lin - p.conv_k) / p.conv_stride + 1;
				p.out_dim / lout
			} else {
				p.out_dim
			};
			if bias_sz > max_bias {
				max_bias = bias_sz;
			}
			if p.kind == LayerKind::Embed && p.vocab * p.dim > max_embed_grad {
				max_embed_grad = p.vocab * p.dim;
			}
			if p.kind == LayerKind::Attn {
				let s = p.in_dim / p.dim;
				if n * p.in_dim > max_seqd {
					max_seqd = n * p.in_dim;
				}
				if n * p.heads * s > max_lse {
					max_lse = n * p.heads * s;
				}
				if p.dim * p.dim > max_dd {
					max_dd = p.dim * p.dim;
				}
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * s, p.dim);
				if ws > max_ws {
					max_ws = ws;
				}
			}
		}
		let mut acts = Vec::with_capacity(params.len());
		let mut preact = Vec::with_capacity(params.len());
		for p in params {
			acts.push(alloc(
				if light && acts.len() + 1 != params.len() {
					1
				} else {
					n * p.out_dim
				},
				"scratch acts",
			)?);
			let needs_pre = matches!(
				p.act,
				Activation::Silu
					| Activation::Gelu | Activation::Elu
					| Activation::Selu | Activation::PRelu
			);
			preact.push(alloc(
				if needs_pre { lt(n * p.out_dim) } else { 1 },
				"scratch preact",
			)?);
		}
		let out_elems = n * params.last().map_or(1, |p| p.out_dim);
		let (concat_sz, concat_grad_sz) = match concat_layer(params) {
			Some(ConcatDims { a, c, .. }) => (n * (a + c), n * a),
			None => (1, 1),
		};
		let mut max_conv_fsz = 0usize;
		for p in params {
			if p.kind == LayerKind::Conv {
				let lin = p.in_dim / p.conv_cin;
				let lout = (lin - p.conv_k) / p.conv_stride + 1;
				let cout = p.out_dim / lout;
				let fsz = cout * p.conv_cin * p.conv_k;
				if fsz > max_conv_fsz {
					max_conv_fsz = fsz;
				}
			}
		}
		let (conv_temp_buf, conv_wg_count) = if !forward_only && !light && max_conv_fsz > 0 {
			let free = gpu_core::hip::mem_info().map(|m| m.free).unwrap_or(0);
			let usable = free / 2;
			let chunks = (usable / (max_conv_fsz * size_of::<f64>())).min(n).max(1);
			let buf = alloc(max_conv_fsz * chunks, "conv_temp")?;
			let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(chunks, max_conv_fsz);
			if ws > max_ws {
				max_ws = ws;
			}
			(buf, chunks)
		} else {
			(alloc(1, "conv_temp")?, 0)
		};
		let cbuf = |i: usize| -> GpuBuffer { consts.view(i, 1) };
		let copy_stream = gpu_core::hip::Stream::new().context("copy stream")?;
		let mut ev_fwd = Vec::with_capacity(n_timed);
		let mut ev_bwd = Vec::with_capacity(n_timed);
		for _ in 0..n_timed {
			let mut f = Vec::with_capacity(params.len() + 1);
			let mut b = Vec::with_capacity(params.len() + 1);
			for _ in 0..=params.len() {
				f.push(gpu_core::hip::Event::new().context("layer timing event")?);
				b.push(gpu_core::hip::Event::new().context("layer timing event")?);
			}
			ev_fwd.push(f);
			ev_bwd.push(b);
		}
		Ok(Scratch {
			acts,
			preact,
			da_a: alloc(lt(bw(max_act)), "da_a")?,
			da_b: alloc(lt(bw(max_act)), "da_b")?,
			dz: alloc(lt(bw(max_act)), "dz")?,
			dw: alloc(lt(bw(max_wt)), "dw")?,
			dw_partials: alloc(lt(bw(max_dw_partials)), "dw_partials")?,
			db: alloc(lt(bw(max_bias)), "db")?,
			metric_t0: alloc(out_elems, "metric_t0")?,
			metric_t1: alloc(out_elems, "metric_t1")?,
			metric_t2: alloc(out_elems, "metric_t2")?,
			metric_scalar: alloc(1, "metric_scalar")?,
			metric_scalar_b: alloc(1, "metric_scalar_b")?,
			c_one: cbuf(0),
			c_neg_one: cbuf(1),
			c_half: cbuf(2),
			c_eps: cbuf(3),
			c_one_minus_eps: cbuf(4),
			c_leaky_alpha: cbuf(5),
			c_elu_alpha: cbuf(6),
			c_selu_alpha: cbuf(7),
			c_selu_lambda: cbuf(8),
			c_focal_gamma: cbuf(9),
			c_focal_alpha: cbuf(10),
			c_rope_theta: cbuf(11),
			reduce_ws: GpuBuffer::alloc_bytes(max_ws).map_err(|e| {
				anyhow::anyhow!(
					"reduce_ws: GPU alloc of {} failed — {e:?}",
					crate::human_bytes(max_ws)
				)
			})?,
			embed_grad: alloc(bw(max_embed_grad), "embed_grad")?,
			a_q: alloc(lt(max_seqd), "a_q")?,
			a_k: alloc(lt(max_seqd), "a_k")?,
			a_v: alloc(lt(max_seqd), "a_v")?,
			a_ctx: alloc(lt(max_seqd), "a_ctx")?,
			a_lse: alloc(if forward_only || light { 1 } else { max_lse }, "a_lse")?,
			a_dctx: alloc(lt(bw(max_seqd)), "a_dctx")?,
			a_dq: alloc(lt(bw(max_seqd)), "a_dq")?,
			a_dk: alloc(lt(bw(max_seqd)), "a_dk")?,
			a_dv: alloc(lt(bw(max_seqd)), "a_dv")?,
			a_dsum: alloc(lt(bw(max_lse)), "a_dsum")?,
			a_gw: alloc(bw(max_dd), "a_gw")?,
			a_dbias: alloc(bw(max_dd), "a_dbias")?,
			prelu_t0: alloc(lt(bw(if has_prelu { max_act } else { 1 })), "prelu_t0")?,
			prelu_t1: alloc(lt(bw(if has_prelu { max_act } else { 1 })), "prelu_t1")?,
			prelu_scalar: alloc(1, "prelu_scalar")?,
			concat: alloc(lt(concat_sz), "concat")?,
			concat_dgrad: alloc(lt(bw(concat_grad_sz)), "concat_dgrad")?,
			conv_temp: conv_temp_buf,
			conv_wg: conv_wg_count,
			infer: forward_only,
			copy_stream,
			ev_fwd,
			ev_bwd,
			timing: Cell::new(false),
			timing_slot: Cell::new(0),
		})
	}

	pub fn set_timing(&self, on: bool) {
		self.timing.set(on);
	}

	pub fn set_timing_slot(&self, slot: usize) {
		self.timing_slot.set(slot);
	}

	pub fn mark_fwd(&self, i: usize) {
		if self.timing.get() {
			let r = self.ev_fwd[self.timing_slot.get()][i].record_default();
			if !r.is_ok() {
				Write::error(format!(
					"record fwd event: {}",
					r.err().map(|e| e.to_string()).unwrap_or_default()
				));
				return;
			}
		}
	}

	pub fn mark_bwd(&self, i: usize) {
		if self.timing.get() {
			let r = self.ev_bwd[self.timing_slot.get()][i].record_default();
			if !r.is_ok() {
				Write::error(format!(
					"record bwd event: {}",
					r.err().map(|e| e.to_string()).unwrap_or_default()
				));
				return;
			}
		}
	}

	pub fn layer_ms(&self, layers: usize) -> LayerMs {
		let s = self.timing_slot.get();
		let r = self.ev_bwd[s][0].synchronize();
		if !r.is_ok() {
			Write::error(format!(
				"sync bwd event: {}",
				r.err().map(|e| e.to_string()).unwrap_or_default()
			));
			return LayerMs {
				fwd: vec![0.0; layers],
				bwd: vec![0.0; layers],
			};
		}
		layer_ms_from(&self.ev_fwd[s], &self.ev_bwd[s], layers)
	}

	pub fn take_events(&mut self) -> LayerEvents {
		LayerEvents {
			fwd: mem::take(&mut self.ev_fwd),
			bwd: mem::take(&mut self.ev_bwd),
		}
	}

	pub fn download_scalar_deferred(&self) {
		gpu_core::memory::pinned_download(0, &self.metric_scalar, &self.copy_stream);
	}

	pub fn download_scalar_b_deferred(&self) {
		gpu_core::memory::pinned_download(1, &self.metric_scalar_b, &self.copy_stream);
	}

	pub fn sync_deferred_scalar(&self) -> f64 {
		let r = self.copy_stream.synchronize();
		if !r.is_ok() {
			Write::error(format!(
				"sync copy stream: {}",
				r.err().map(|e| e.to_string()).unwrap_or_default()
			));
		}
		return gpu_core::memory::pinned_read(0);
	}

	pub fn read_metric_scalar(&self) -> f64 {
		self.download_scalar_deferred();
		return self.sync_deferred_scalar();
	}

	pub fn sync_copy_stream(&self) {
		let r = self.copy_stream.synchronize();
		if !r.is_ok() {
			Write::error(format!(
				"sync copy stream: {}",
				r.err().map(|e| e.to_string()).unwrap_or_default()
			));
			return;
		}
	}

	pub fn deferred_scalar(&self) -> f64 {
		return gpu_core::memory::pinned_read(0);
	}
	pub fn deferred_scalar_b(&self) -> f64 {
		return gpu_core::memory::pinned_read(1);
	}
}

impl Drop for Scratch {
	fn drop(&mut self) {
		let owned_any = self.acts.iter().any(GpuBuffer::is_pool_owned)
			|| self.preact.iter().any(GpuBuffer::is_pool_owned)
			|| [
				&self.da_a,
				&self.da_b,
				&self.dz,
				&self.dw,
				&self.dw_partials,
				&self.db,
				&self.metric_t0,
				&self.metric_t1,
				&self.metric_t2,
				&self.metric_scalar,
				&self.metric_scalar_b,
				&self.c_one,
				&self.c_neg_one,
				&self.c_half,
				&self.c_eps,
				&self.c_one_minus_eps,
				&self.c_leaky_alpha,
				&self.c_elu_alpha,
				&self.c_selu_alpha,
				&self.c_selu_lambda,
				&self.c_focal_gamma,
				&self.c_focal_alpha,
				&self.c_rope_theta,
				&self.reduce_ws,
				&self.embed_grad,
				&self.a_q,
				&self.a_k,
				&self.a_v,
				&self.a_ctx,
				&self.a_lse,
				&self.a_dctx,
				&self.a_dq,
				&self.a_dk,
				&self.a_dv,
				&self.a_dsum,
				&self.a_gw,
				&self.a_dbias,
				&self.prelu_t0,
				&self.prelu_t1,
				&self.prelu_scalar,
				&self.concat,
				&self.concat_dgrad,
				&self.conv_temp,
			]
			.iter()
			.any(|b| b.is_pool_owned());
		if owned_any {
			if let Err(e) = gpu_core::hip::device_synchronize() {
				Write::error(format!("Scratch::drop device sync: {e}"));
			}
		}
	}
}

impl Scratch {
	pub fn vram_bytes(params: &[LayerParams], n: usize, forward_only: bool) -> usize {
		let dims: Vec<LayerDims> = params.iter().map(LayerDims::from).collect();
		Self::vram_bytes_dims(&dims, n, forward_only)
	}

	pub fn vram_bytes_dims(params: &[LayerDims], n: usize, forward_only: bool) -> usize {
		let bw = |sz: usize| if forward_only { 1 } else { sz };
		let mut max_ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1);
		let (mut max_act, mut max_wt, mut max_bias) = (0usize, 0usize, 0usize);
		let (mut max_embed_grad, mut max_seqd, mut max_lse, mut max_dd) =
			(1usize, 1usize, 1usize, 1usize);
		let mut max_dw_partials = 1usize;
		let mut has_prelu = false;
		let mut floats = 0usize;
		for p in params {
			floats += n * p.out_dim;
			let dw_dp = match p.kind {
				LayerKind::Dense => {
					kernels::gpu_splitk_dw_partials_elems(n, p.in_dim, p.out_dim)
				}
				LayerKind::Attn => {
					let s = p.in_dim / p.dim;
					kernels::gpu_splitk_dw_partials_elems(n * s, p.dim, p.dim)
				}
				_ => 0,
			};
			if dw_dp > max_dw_partials {
				max_dw_partials = dw_dp;
			}
			let needs_pre = matches!(
				p.act,
				Activation::Silu
					| Activation::Gelu | Activation::Elu
					| Activation::Selu | Activation::PRelu
			);
			floats += if needs_pre { n * p.out_dim } else { 1 };
			if p.act == Activation::PRelu {
				has_prelu = true;
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * p.out_dim, 1);
				if ws > max_ws {
					max_ws = ws;
				}
			}
			let w = kernels::gpu_reduce_sum_cols_workspace_bytes(n, p.out_dim);
			if w > max_ws {
				max_ws = w;
			}
			if n * p.out_dim.max(p.in_dim) > max_act {
				max_act = n * p.out_dim.max(p.in_dim);
			}
			let wt = match p.kind {
				LayerKind::Conv => {
					let lin = p.in_dim / p.conv_cin;
					let lout = (lin - p.conv_k) / p.conv_stride + 1;
					let cout = p.out_dim / lout;
					cout * p.conv_cin * p.conv_k
				}
				LayerKind::Dense => p.in_dim * p.out_dim,
				_ => 0,
			};
			if wt > max_wt {
				max_wt = wt;
			}
			if p.out_dim > max_bias {
				max_bias = p.out_dim;
			}
			if p.kind == LayerKind::Embed && p.vocab * p.dim > max_embed_grad {
				max_embed_grad = p.vocab * p.dim;
			}
			if p.kind == LayerKind::Attn {
				let s = p.in_dim / p.dim;
				if n * p.in_dim > max_seqd {
					max_seqd = n * p.in_dim;
				}
				if n * p.heads * s > max_lse {
					max_lse = n * p.heads * s;
				}
				if p.dim * p.dim > max_dd {
					max_dd = p.dim * p.dim;
				}
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * s, p.dim);
				if ws > max_ws {
					max_ws = ws;
				}
			}
		}
		let out_elems = n * params.last().map_or(1, |p| p.out_dim);
		floats += 3 * bw(max_act);
		floats += bw(max_wt) + bw(max_dw_partials) + bw(max_bias);
		floats += 3 * out_elems + 2;
		floats += 12;
		floats += bw(max_embed_grad);
		floats += 4 * max_seqd;
		floats += 4 * bw(max_seqd);
		if forward_only {
			floats += 2;
		} else {
			floats += 2 * max_lse;
		}
		floats += 2 * bw(max_dd);
		floats += 2 * bw(if has_prelu { max_act } else { 1 }) + 1;
		match crate::params::concat_layer_dims(params) {
			Some(ConcatDims { a, c, .. }) => {
				floats += n * (a + c) + bw(n * a);
			}
			None => {
				floats += 1 + bw(1);
			}
		}
		floats * mem::size_of::<f64>() + max_ws
	}
}

pub fn vram_estimate(
	specs: &[LayerSpec],
	n: usize,
	d: usize,
	k: usize,
	vocab: usize,
	c_cat: usize,
	forward_only: bool,
) -> usize {
	let f8 = mem::size_of::<f64>();
	let mut bytes = 0usize;
	bytes += 2 * n * d * f8;
	if c_cat > 0 {
		bytes += 2 * n * c_cat * f8;
	}
	bytes += 3 * d * f8;
	bytes += n * k * f8;
	let mut in_dim = d;
	let mut embed_dim = 0usize;
	let mut fake_params: Vec<(usize, usize, LayerKind, usize, usize, Activation, usize)> =
		Vec::new();
	for spec in specs {
		match *spec {
			LayerSpec::Embed(dim, _) => {
				let seq = in_dim;
				bytes += vocab * dim * f8;
				bytes += seq * dim * f8;
				let out = seq * dim;
				embed_dim = dim;
				fake_params.push((
					in_dim,
					out,
					LayerKind::Embed,
					dim,
					vocab,
					Activation::Linear,
					0,
				));
				in_dim = out;
			}
			LayerSpec::Attn(heads) => {
				let d_tok = if embed_dim > 0 { embed_dim } else { in_dim };
				bytes += 4 * d_tok * d_tok * f8;
				bytes += 4 * d_tok * f8;
				fake_params.push((
					in_dim,
					in_dim,
					LayerKind::Attn,
					d_tok,
					0,
					Activation::Linear,
					heads,
				));
			}
			LayerSpec::Conv(filters, kernel, stride, act) => {
				let cin = fake_params.last().map_or(1, |(_, _, kind, ..)| {
					if *kind == LayerKind::Conv { 0 } else { 1 }
				});
				let cin = if cin == 0 {
					match fake_params.last() {
						Some(prev) => {
							prev.1 / ((prev.0 / prev.3.max(1) - prev.4)
								/ prev.6.max(1) + 1)
								.max(1)
						}
						None => {
							Write::error("conv cin");
							return bytes;
						}
					}
				} else {
					cin
				};
				let lin = in_dim / cin;
				let lout = (lin - kernel) / stride + 1;
				bytes += filters * cin * kernel * f8;
				bytes += filters * f8;
				if act == Activation::PRelu {
					bytes += f8;
				}
				let out = filters * lout;
				fake_params.push((in_dim, out, LayerKind::Conv, cin, kernel, act, stride));
				in_dim = out;
			}
			LayerSpec::Dense(units, act) => {
				let actual_in = if c_cat > 0
					&& !fake_params.is_empty()
					&& matches!(
						fake_params.last(),
						Some((_, _, LayerKind::Embed | LayerKind::Attn, ..))
					) {
					in_dim + c_cat
				} else {
					in_dim
				};
				bytes += actual_in * units * f8;
				bytes += units * f8;
				if act == Activation::PRelu {
					bytes += f8;
				}
				fake_params.push((actual_in, units, LayerKind::Dense, 0, 0, act, 0));
				in_dim = units;
			}
		}
	}
	let dummy_params: Vec<LayerParams> = fake_params
		.iter()
		.map(|&(i, o, kind, dim, vocab, act, heads)| {
			let dummy = || GpuBuffer::borrow(ptr::null_mut(), 0);
			let (cc, ck, cs) = if kind == LayerKind::Conv {
				(dim, vocab, heads)
			} else {
				(0, 0, 0)
			};
			LayerParams {
				kind,
				w: dummy(),
				b: dummy(),
				in_dim: i,
				out_dim: o,
				act,
				dim: dim.max(1),
				vocab,
				wk: dummy(),
				wv: dummy(),
				wo: dummy(),
				heads,
				palpha: dummy(),
				conv_cin: cc,
				conv_k: ck,
				conv_stride: cs,
			}
		})
		.collect();
	if !dummy_params.is_empty() {
		bytes += Scratch::vram_bytes(&dummy_params, n, forward_only);
	}
	bytes
}
