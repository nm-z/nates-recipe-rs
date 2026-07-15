use crate::enums::{Activation, LayerKind, LayerSpec};
use crate::{Param, download_scalar, download_vec};
use anyhow::Context;
use gpu_core::log::{Errored, Write};
use gpu_core::memory::GpuBuffer;
use std::env;
use std::f64::consts;
use std::fs;
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

pub const LEAKY_ALPHA: f64 = 0.01;
pub const PRELU_INIT: f64 = 0.25;
pub const ELU_ALPHA: f64 = 1.0;
pub const FOCAL_GAMMA: f64 = 2.0;
pub const FOCAL_ALPHA: f64 = 0.25;

pub fn sinusoidal_pe(seq: usize, dim: usize, negate: bool) -> Vec<f64> {
	let sign = if negate { -1.0 } else { 1.0 };
	let mut pe = vec![0.0f64; seq * dim];
	for s in 0..seq {
		for j in 0..dim {
			let i2 = (j / 2) * 2;
			let freq = 1.0 / 10000f64.powf(i2 as f64 / dim as f64);
			let ang = s as f64 * freq;
			pe[s * dim + j] = sign * if j % 2 == 0 { ang.sin() } else { ang.cos() };
		}
	}
	pe
}

pub struct LayerParams {
	pub kind: LayerKind,
	pub w: GpuBuffer,
	pub b: GpuBuffer,
	pub in_dim: usize,
	pub out_dim: usize,
	pub act: Activation,
	pub dim: usize,
	pub vocab: usize,
	pub wk: GpuBuffer,
	pub wv: GpuBuffer,
	pub wo: GpuBuffer,
	pub heads: usize,
	pub palpha: GpuBuffer,
	pub conv_cin: usize,
	pub conv_k: usize,
	pub conv_stride: usize,
}

#[derive(Clone, Copy)]
pub struct ConcatDims {
	pub pf: usize,
	pub a: usize,
	pub c: usize,
}

pub fn concat_layer(params: &[LayerParams]) -> Option<ConcatDims> {
	concat_layer_dims_iter(params.iter().map(|p| (p.kind, p.in_dim, p.out_dim)))
}

pub fn concat_layer_dims(dims: &[LayerDims]) -> Option<ConcatDims> {
	concat_layer_dims_iter(dims.iter().map(|d| (d.kind, d.in_dim, d.out_dim)))
}

fn concat_layer_dims_iter(
	it: impl Iterator<Item = (LayerKind, usize, usize)>,
) -> Option<ConcatDims> {
	let layers: Vec<(LayerKind, usize, usize)> = it.collect();
	for l in 1..layers.len() {
		let (prev_kind, _, prev_out) = layers[l - 1];
		let (kind, in_dim, _) = layers[l];
		if kind == LayerKind::Dense && matches!(prev_kind, LayerKind::Embed | LayerKind::Attn) {
			let a = prev_out;
			let c = in_dim.saturating_sub(a);
			return (c > 0).then_some(ConcatDims { pf: l, a, c });
		}
	}
	None
}

pub struct Scaler {
	pub mean: Vec<f64>,
	pub std: Vec<f64>,
}

pub fn pinned_vocab(specs: &[LayerSpec]) -> Option<usize> {
	specs.iter().find_map(|s| match s {
		LayerSpec::Embed(_, v) => *v,
		_ => None,
	})
}

#[derive(Clone, Copy)]
pub struct LayerDims {
	pub kind: LayerKind,
	pub in_dim: usize,
	pub out_dim: usize,
	pub act: Activation,
	pub dim: usize,
	pub vocab: usize,
	pub heads: usize,
	pub conv_cin: usize,
	pub conv_k: usize,
	pub conv_stride: usize,
}

impl From<&LayerParams> for LayerDims {
	fn from(p: &LayerParams) -> Self {
		LayerDims {
			kind: p.kind,
			in_dim: p.in_dim,
			out_dim: p.out_dim,
			act: p.act,
			dim: p.dim,
			vocab: p.vocab,
			heads: p.heads,
			conv_cin: p.conv_cin,
			conv_k: p.conv_k,
			conv_stride: p.conv_stride,
		}
	}
}

fn philox2x32(counter: u32, key: u32) -> u32 {
	let (mut x, mut y) = (counter, key);
	for i in 0..10u32 {
		let lo = x.wrapping_mul(0xD2511F53);
		let hi = ((x as u64 * 0xD2511F53u64) >> 32) as u32;
		x = hi ^ y ^ key.wrapping_mul(i + 1);
		y = lo;
	}
	x
}

fn philox_uniform(idx: u32, seed: u32) -> f64 {
	philox2x32(idx, seed) as f64 / 4294967296.0
}

fn host_randn(seed: u32, scale: f64, out: &mut [f64]) {
	for (i, o) in out.iter_mut().enumerate() {
		let u1 = philox_uniform((2 * i) as u32, seed).max(1e-30);
		let u2 = philox_uniform((2 * i + 1) as u32, seed);
		*o = (-2.0 * u1.ln()).sqrt() * (2.0 * consts::PI * u2).cos() * scale;
	}
}

struct BlockPlan {
	off: usize,
	len: usize,
}

struct PlanEntry {
	dims: LayerDims,
	w: BlockPlan,
	b: BlockPlan,
	wk: BlockPlan,
	wv: BlockPlan,
	wo: BlockPlan,
	palpha: BlockPlan,
}

pub struct LayerPlan {
	entries: Vec<PlanEntry>,
	host: Vec<f64>,
}

const PLAN_ALIGN_F64: usize = 32;

impl LayerPlan {
	fn pad(&mut self) {
		let rem = self.host.len() % PLAN_ALIGN_F64;
		if rem != 0 {
			self.host
				.resize(self.host.len() + PLAN_ALIGN_F64 - rem, 0.0);
		}
	}

	fn push(&mut self, data: &[f64]) -> BlockPlan {
		self.pad();
		let off = self.host.len();
		self.host.extend_from_slice(data);
		BlockPlan {
			off,
			len: data.len(),
		}
	}

	fn zeros(&mut self, len: usize) -> BlockPlan {
		self.pad();
		let off = self.host.len();
		self.host.resize(off + len, 0.0);
		BlockPlan { off, len }
	}

	fn randn(&mut self, len: usize, seed: usize, scale: f64) -> BlockPlan {
		self.pad();
		let off = self.host.len();
		self.host.resize(off + len, 0.0);
		host_randn(seed as u32, scale, &mut self.host[off..off + len]);
		BlockPlan { off, len }
	}

	pub fn host(&self) -> &[f64] {
		&self.host
	}

	pub fn dims(&self) -> Vec<LayerDims> {
		self.entries.iter().map(|e| e.dims).collect()
	}

	pub fn out_dim_last(&self) -> usize {
		self.entries.last().map_or(0, |e| e.dims.out_dim)
	}

	pub fn materialize(&self, staged: &GpuBuffer, base_off: usize) -> Vec<LayerParams> {
		let view =
			|bp: &BlockPlan| -> GpuBuffer { staged.view(base_off + bp.off, bp.len.max(1)) };
		self.entries
			.iter()
			.map(|e| LayerParams {
				kind: e.dims.kind,
				w: view(&e.w),
				b: view(&e.b),
				in_dim: e.dims.in_dim,
				out_dim: e.dims.out_dim,
				act: e.dims.act,
				dim: e.dims.dim,
				vocab: e.dims.vocab,
				wk: view(&e.wk),
				wv: view(&e.wv),
				wo: view(&e.wo),
				heads: e.dims.heads,
				palpha: view(&e.palpha),
				conv_cin: e.dims.conv_cin,
				conv_k: e.dims.conv_k,
				conv_stride: e.dims.conv_stride,
			})
			.collect()
	}

	pub fn dump_ogdl_host(&self, image: &[f64], key: &str, score: f64) -> String {
		let blk = |bp: &BlockPlan| image[bp.off..bp.off + bp.len].to_vec();
		ogdl_text(|g| {
			drop(g.add(score, key));
			let mut z = 1;
			let mut at = 1;
			let mut cv = 1;
			for e in &self.entries {
				let d = &e.dims;
				match d.kind {
					LayerKind::Embed => {
						let table = blk(&e.w);
						for id in 0..d.vocab {
							drop(g.add(
								table[id * d.dim..(id + 1) * d.dim].to_vec(),
								&format!("embed.{id}"),
							));
						}
					}
					LayerKind::Attn => {
						drop(g.add(blk(&e.w), &format!("attn{at}.wq")));
						drop(g.add(blk(&e.wk), &format!("attn{at}.wk")));
						drop(g.add(blk(&e.wv), &format!("attn{at}.wv")));
						drop(g.add(blk(&e.wo), &format!("attn{at}.wo")));
						let bias = blk(&e.b);
						for nm in ["bq", "bk", "bv", "bo"] {
							drop(g.add(bias.clone(), &format!("attn{at}.{nm}")));
						}
						at += 1;
					}
					LayerKind::Conv => {
						let lin = d.in_dim / d.conv_cin;
						let lout = (lin - d.conv_k) / d.conv_stride + 1;
						let cout = d.out_dim / lout;
						drop(g.add(
							vec![
								cout as f64,
								d.conv_cin as f64,
								d.conv_k as f64,
								d.conv_stride as f64,
							],
							&format!("conv{cv}"),
						));
						drop(g.add(blk(&e.w), &format!("conv{cv}.w")));
						drop(g.add(blk(&e.b), &format!("conv{cv}.b")));
						cv += 1;
					}
					LayerKind::Dense => {
						let w = blk(&e.w);
						let b = blk(&e.b);
						let slope =
							(d.act == Activation::PRelu).then(|| image[e.palpha.off]);
						for j in 0..d.out_dim {
							let row: Vec<f64> = (0..d.in_dim)
								.map(|i| w[i * d.out_dim + j])
								.collect();
							drop(g.add(row, &format!("z{z}.w")));
							if let Some(a) = slope {
								drop(g.add(a, &format!("z{z}.a")));
							}
							drop(g.add(b[j], &format!("z{z}.b")));
							z += 1;
						}
					}
				}
			}
		})
	}
}

pub fn ogdl_text(build: impl FnOnce(ogdl::Graph)) -> String {
	static SEQ: AtomicU64 = AtomicU64::new(0);
	let seq = SEQ.fetch_add(1, Ordering::Relaxed);
	let tmp = env::temp_dir().join(format!("nrs_dump_{}_{seq}.ogdl", process::id()));
	let Some(tp) = tmp.to_str() else {
		drop(Write::err("utf8 tmp"));
		process::abort();
	};
	let _ = fs::remove_file(tp);
	let g = ogdl::file(tp);
	build(g.clone());
	drop(g.file(tp));
	let text = fs::read_to_string(tp).unwrap_or_default();
	let _ = fs::remove_file(tp);
	text
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlanMode {
	Fresh,
	Warm,
}

pub fn plan_layer_params(
	specs: &[LayerSpec],
	d: usize,
	c_cat: usize,
	vocab: usize,
	resumed: &[Saved],
	mode: PlanMode,
) -> Result<LayerPlan, String> {
	let try_resume = mode == PlanMode::Warm;
	let mut plan = LayerPlan {
		entries: Vec::new(),
		host: Vec::new(),
	};
	let dummy_off = plan.zeros(1).off;
	let dummy = || BlockPlan {
		off: dummy_off,
		len: 1,
	};
	let mut si = 0usize;
	let mut in_dim = d;
	for (li, spec) in specs.iter().enumerate() {
		if let LayerSpec::Embed(dim, _) = *spec {
			let w = if try_resume {
				let t = match resumed.get(si) {
					Some(Saved::Embed(t)) => t,
					_ => {
						return Err(format!(
							"layer {li}: checkpoint has no embed block here"
						));
					}
				};
				if t.len() != vocab * dim {
					return Err(format!(
						"layer {li} embed: checkpoint table has {} values, model needs {} (vocab {vocab} × dim {dim})",
						t.len(),
						vocab * dim
					));
				}
				si += 1;
				plan.push(t)
			} else {
				plan.randn(vocab * dim, 4242 + li * 7919, 0.1)
			};
			let b = plan.push(&sinusoidal_pe(in_dim, dim, true));
			plan.entries.push(PlanEntry {
				dims: LayerDims {
					kind: LayerKind::Embed,
					in_dim,
					out_dim: in_dim * dim,
					act: Activation::Linear,
					dim,
					vocab,
					heads: 0,
					conv_cin: 0,
					conv_k: 0,
					conv_stride: 0,
				},
				w,
				b,
				wk: dummy(),
				wv: dummy(),
				wo: dummy(),
				palpha: dummy(),
			});
			in_dim *= dim;
			continue;
		}
		if let LayerSpec::Attn(heads) = *spec {
			let d_tok = plan.entries.last().map_or(in_dim, |e| {
				if e.dims.kind == LayerKind::Embed {
					e.dims.dim
				} else {
					e.dims.out_dim
				}
			});
			if !in_dim.is_multiple_of(d_tok) {
				return Err(Errored::new(format!(
					"attn: input {in_dim} not a multiple of token dim {d_tok}"
				))
				.to_string());
			}
			if !d_tok.is_multiple_of(heads) {
				return Err(Errored::new(format!(
					"attn: token dim {d_tok} not divisible by {heads} heads"
				))
				.to_string());
			}
			let need = d_tok * d_tok;
			let (w, wk, wv, wo) = if try_resume {
				let (sq, sk, sv, so) = match resumed.get(si) {
					Some(Saved::Attn { wq, wk, wv, wo, .. }) => (wq, wk, wv, wo),
					_ => {
						return Err(format!(
							"layer {li}: checkpoint has no attn block here"
						));
					}
				};
				for (nm, v) in [("wq", sq), ("wk", sk), ("wv", sv), ("wo", so)] {
					if v.len() != need {
						return Err(format!(
							"layer {li} attn {nm}: checkpoint has {} values, model needs {need} (token dim {d_tok}²)",
							v.len()
						));
					}
				}
				si += 1;
				(plan.push(sq), plan.push(sk), plan.push(sv), plan.push(so))
			} else {
				let scale = (1.0 / d_tok as f64).sqrt();
				(
					plan.randn(need, 7001 + li * 13, scale),
					plan.randn(need, 7002 + li * 13, scale),
					plan.randn(need, 7003 + li * 13, scale),
					plan.randn(need, 7004 + li * 13, scale),
				)
			};
			let b = plan.zeros(d_tok);
			plan.entries.push(PlanEntry {
				dims: LayerDims {
					kind: LayerKind::Attn,
					in_dim,
					out_dim: in_dim,
					act: Activation::Linear,
					dim: d_tok,
					vocab: 0,
					heads,
					conv_cin: 0,
					conv_k: 0,
					conv_stride: 0,
				},
				w,
				b,
				wk,
				wv,
				wo,
				palpha: dummy(),
			});
			continue;
		}
		if let LayerSpec::Conv(filters, kernel, stride, act) = *spec {
			let cin = if let Some(prev) = plan.entries.last() {
				if prev.dims.kind == LayerKind::Conv {
					let prev_lout = (prev.dims.in_dim / prev.dims.conv_cin
						- prev.dims.conv_k) / prev.dims.conv_stride
						+ 1;
					prev.dims.out_dim / prev_lout
				} else {
					1
				}
			} else {
				1
			};
			let lin = in_dim / cin;
			let lout = (lin - kernel) / stride + 1;
			let w_count = filters * cin * kernel;
			let (w, b, slope) = if !try_resume {
				let scale = (2.0 / (cin * kernel) as f64).sqrt();
				(plan.randn(w_count, li, scale), plan.zeros(filters), None)
			} else {
				let (ws, bs) = match resumed.get(si) {
					Some(Saved::Conv { w, b }) => (w, b),
					_ => {
						return Err(format!(
							"layer {li}: checkpoint has no conv block here"
						));
					}
				};
				if ws.len() != w_count {
					return Err(format!(
						"layer {li} conv: checkpoint has {} weights, model needs {w_count} ({filters}×{cin}×{kernel})",
						ws.len()
					));
				}
				if bs.len() != filters {
					return Err(format!(
						"layer {li} conv: checkpoint has {} biases, model needs {filters}",
						bs.len()
					));
				}
				si += 1;
				(plan.push(ws), plan.push(bs), None)
			};
			let palpha = if act == Activation::PRelu {
				plan.push(&[slope.unwrap_or(PRELU_INIT)])
			} else {
				dummy()
			};
			plan.entries.push(PlanEntry {
				dims: LayerDims {
					kind: LayerKind::Conv,
					in_dim,
					out_dim: filters * lout,
					act,
					dim: 0,
					vocab: 0,
					heads: 0,
					conv_cin: cin,
					conv_k: kernel,
					conv_stride: stride,
				},
				w,
				b,
				wk: dummy(),
				wv: dummy(),
				wo: dummy(),
				palpha,
			});
			in_dim = filters * lout;
			continue;
		}
		let LayerSpec::Dense(units, act) = *spec else {
			return Err(
				"plan_layer_params: layer spec not handled by an earlier arm".to_owned(),
			);
		};
		if c_cat > 0
			&& matches!(
				plan.entries.last().map(|e| e.dims.kind),
				Some(LayerKind::Embed | LayerKind::Attn)
			) {
			in_dim += c_cat;
		}
		let (w, b, slope) = if !try_resume {
			let scale = (2.0 / in_dim as f64).sqrt();
			(
				plan.randn(in_dim * units, li, scale),
				plan.zeros(units),
				None,
			)
		} else {
			let mut wh = vec![0.0f64; in_dim * units];
			let mut bh = vec![0.0f64; units];
			let mut slope = None;
			for j in 0..units {
				let (ws, bias, a) = match resumed.get(si) {
					Some(Saved::Dense { w, b, a }) => (w, *b, *a),
					_ => {
						return Err(format!(
							"layer {li} neuron {j}: checkpoint has no dense (z) block here"
						));
					}
				};
				if ws.len() != in_dim {
					return Err(format!(
						"layer {li} neuron {j}: checkpoint has {} weights, model needs {in_dim} (data feature count differs?)",
						ws.len()
					));
				}
				for i in 0..in_dim {
					wh[i * units + j] = ws[i];
				}
				bh[j] = bias;
				if j == 0 {
					slope = a;
				}
				si += 1;
			}
			(plan.push(&wh), plan.push(&bh), slope)
		};
		let palpha = if act == Activation::PRelu {
			plan.push(&[slope.unwrap_or(PRELU_INIT)])
		} else {
			dummy()
		};
		plan.entries.push(PlanEntry {
			dims: LayerDims {
				kind: LayerKind::Dense,
				in_dim,
				out_dim: units,
				act,
				dim: 0,
				vocab: 0,
				heads: 0,
				conv_cin: 0,
				conv_k: 0,
				conv_stride: 0,
			},
			w,
			b,
			wk: dummy(),
			wv: dummy(),
			wo: dummy(),
			palpha,
		});
		in_dim = units;
	}
	if try_resume && si != resumed.len() {
		return Err(format!(
			"checkpoint has {} saved blocks, this architecture consumed {si}",
			resumed.len()
		));
	}
	Ok(plan)
}

#[derive(Debug, PartialEq)]
pub enum Saved {
	Embed(Vec<f64>),
	Attn {
		wq: Vec<f64>,
		wk: Vec<f64>,
		wv: Vec<f64>,
		wo: Vec<f64>,
		bq: Vec<f64>,
		bk: Vec<f64>,
		bv: Vec<f64>,
		bo: Vec<f64>,
	},
	Dense {
		w: Vec<f64>,
		b: f64,
		a: Option<f64>,
	},
	Conv {
		w: Vec<f64>,
		b: Vec<f64>,
	},
}

impl Saved {
	pub fn len(&self) -> usize {
		match self {
			Saved::Embed(t) => t.len(),
			Saved::Attn {
				wq,
				wk,
				wv,
				wo,
				bq,
				bk,
				bv,
				bo,
			} => {
				wq.len()
					+ wk.len() + wv.len() + wo.len()
					+ bq.len() + bk.len() + bv.len()
					+ bo.len()
			}
			Saved::Dense { w, .. } => w.len() + 1,
			Saved::Conv { w, b } => w.len() + b.len(),
		}
	}
}

pub fn load_ogdl(path: &str) -> anyhow::Result<Vec<Saved>> {
	let text = match fs::read_to_string(path) {
		Ok(t) => t,
		Err(_) => {
			drop(Write::err(&format!(
				"no data in {path}, initialized random weights and biases"
			)));
			return Ok(Vec::new());
		}
	};
	load_ogdl_str(&text)
}

pub fn load_ogdl_str(text: &str) -> anyhow::Result<Vec<Saved>> {
	let root = ogdl::text(text).itnl("");
	let vals = |n: &ogdl::Node| -> Vec<f64> {
		n.children
			.iter()
			.filter_map(|g| g.name.parse::<f64>().ok())
			.collect()
	};
	let mut out: Vec<Saved> = Vec::new();
	for block in &root.children {
		if !block.children.is_empty() && block.children.iter().all(|c| c.children.is_empty()) {
			continue;
		}
		let field = |name: &str| -> Vec<f64> {
			block.children
				.iter()
				.find(|c| c.name == name)
				.map_or_else(Vec::new, &vals)
		};
		match block.name.as_str() {
			"embed" => {
				let mut rows: Vec<(usize, Vec<f64>)> =
					Vec::with_capacity(block.children.len());
				for c in &block.children {
					let id = c.name.parse().context("resume: embed row id")?;
					rows.push((id, vals(c)));
				}
				rows.sort_by_key(|(id, _)| *id);
				out.push(Saved::Embed(
					rows.into_iter().flat_map(|(_, v)| v).collect(),
				));
			}
			name if name.starts_with("attn") => out.push(Saved::Attn {
				wq: field("wq"),
				wk: field("wk"),
				wv: field("wv"),
				wo: field("wo"),
				bq: field("bq"),
				bk: field("bk"),
				bv: field("bv"),
				bo: field("bo"),
			}),
			name if name.starts_with("conv") => out.push(Saved::Conv {
				w: field("w"),
				b: field("b"),
			}),
			_ => {
				let mut w = Vec::new();
				let mut b = 0.0;
				let mut a = None;
				for c in &block.children {
					match c.name.as_str() {
						"w" => w = vals(c),
						"b" => {
							b = vals(c)
								.first()
								.copied()
								.ok_or_else(|| anyhow::anyhow!("resume: dense b"))?
						}
						"a" => a = vals(c).first().copied(),
						key if key.starts_with('w')
							&& key.len() > 1 && key[1..]
							.chars()
							.all(|ch| ch.is_ascii_digit()) =>
						{
							w.push(vals(c).first().copied().ok_or_else(|| {
								anyhow::anyhow!("resume: dense w{{n}}")
							})?);
						}
						key => anyhow::bail!(
							"resume: unrecognized key '{key}' — incompatible checkpoint; rm the .ogdl to start fresh"
						),
					}
				}
				out.push(Saved::Dense { w, b, a });
			}
		}
	}
	Ok(out)
}

pub fn dump_ogdl(
	params: &[LayerParams],
	filter: Option<&[Param]>,
	key: &str,
	score: f64,
) -> String {
	let want_w = filter.is_none_or(|f| f.contains(&Param::W));
	let want_b = filter.is_none_or(|f| f.contains(&Param::B));
	crate::params::ogdl_text(|g| {
		drop(g.add(score, key));
		let mut z = 1;
		let mut at = 1;
		let mut cv = 1;
		for p in params.iter() {
			match p.kind {
				LayerKind::Embed => {
					if want_w {
						let table = download_vec(&p.w, p.vocab * p.dim);
						for id in 0..p.vocab {
							drop(g.add(
								table[id * p.dim..(id + 1) * p.dim].to_vec(),
								&format!("embed.{id}"),
							));
						}
					}
				}
				LayerKind::Attn => {
					let dd = p.dim * p.dim;
					if want_w {
						drop(g.add(download_vec(&p.w, dd), &format!("attn{at}.wq")));
						drop(g.add(download_vec(&p.wk, dd), &format!("attn{at}.wk")));
						drop(g.add(download_vec(&p.wv, dd), &format!("attn{at}.wv")));
						drop(g.add(download_vec(&p.wo, dd), &format!("attn{at}.wo")));
					}
					if want_b {
						let bias = download_vec(&p.b, p.dim);
						for nm in ["bq", "bk", "bv", "bo"] {
							drop(g.add(bias.clone(), &format!("attn{at}.{nm}")));
						}
					}
					at += 1;
				}
				LayerKind::Conv => {
					let lin = p.in_dim / p.conv_cin;
					let lout = (lin - p.conv_k) / p.conv_stride + 1;
					let cout = p.out_dim / lout;
					let w_count = cout * p.conv_cin * p.conv_k;
					drop(g.add(
						vec![
							cout as f64,
							p.conv_cin as f64,
							p.conv_k as f64,
							p.conv_stride as f64,
						],
						&format!("conv{cv}"),
					));
					if want_w {
						drop(g.add(
							download_vec(&p.w, w_count),
							&format!("conv{cv}.w"),
						));
					}
					if want_b {
						drop(g.add(download_vec(&p.b, cout), &format!("conv{cv}.b")));
					}
					cv += 1;
				}
				LayerKind::Dense => {
					let w = download_vec(&p.w, p.in_dim * p.out_dim);
					let b = download_vec(&p.b, p.out_dim);
					let slope = (p.act == Activation::PRelu)
						.then(|| download_scalar(&p.palpha));
					for j in 0..p.out_dim {
						if want_w {
							let row: Vec<f64> = (0..p.in_dim)
								.map(|i| w[i * p.out_dim + j])
								.collect();
							drop(g.add(row, &format!("z{z}.w")));
							if let Some(a) = slope {
								drop(g.add(a, &format!("z{z}.a")));
							}
						}
						if want_b {
							drop(g.add(b[j], &format!("z{z}.b")));
						}
						z += 1;
					}
				}
			}
		}
	})
}

pub fn write_ogdl(path: &str, out: &str) -> anyhow::Result<()> {
	if let Some(parent) = Path::new(path).parent()
		&& !parent.as_os_str().is_empty()
	{
		fs::create_dir_all(parent)
			.map_err(|e| anyhow::anyhow!("save: mkdir {}: {e}", parent.display()))?;
	}
	fs::write(path, out).map_err(|e| anyhow::anyhow!("save: write {path}: {e}"))
}

pub fn saved_score(path: &str, key: &str) -> Option<f64> {
	let text = fs::read_to_string(path).ok()?;
	for line in text.lines() {
		let line = line.trim();
		if let Some((k, v)) = line
			.split_once('=')
			.or_else(|| line.split_once(char::is_whitespace))
			&& k.trim() == key
		{
			return v.trim().parse().ok();
		}
	}
	None
}
