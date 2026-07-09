//! Layer parameters and their construction: the per-layer GPU weight buffers,
//! the resume-checkpoint block type, the positional-encoding table, the
//! two-branch concat detector, and `plan_layer_params` (host-composed init or resume).

use crate::enums::{Activation, LayerKind, LayerSpec};
use crate::ogdl::Saved;
use gpu_core::memory::GpuBuffer;

/// Leaky-ReLU negative slope, and PReLU's initial (then learned) slope.
pub const LEAKY_ALPHA: f64 = 0.01;
pub const PRELU_INIT: f64 = 0.25;
/// ELU negative-saturation scale (SELU's fixed constants live in gpu-core's selu).
pub const ELU_ALPHA: f64 = 1.0;
pub const FOCAL_GAMMA: f64 = 2.0;
pub const FOCAL_ALPHA: f64 = 0.25;

/// Sinusoidal positional encoding table [seq*dim], row-major: PE[s,2i]=sin(s/10000^(2i/dim)),
/// PE[s,2i+1]=cos(...). `negate` returns -PE (so a broadcast-SUB adds it). Built on host
/// once (no GPU PE kernel); added per row in the embed forward.
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
	// Dense: weight [in_dim×out_dim]. Embed: token table [vocab×dim]. Attn: Wq [d×d].
	pub w: GpuBuffer,
	// Dense: bias [out_dim]. Embed: negated positional encoding [in_dim*dim]. Attn: zero bias [d].
	pub b: GpuBuffer,
	pub in_dim: usize,
	pub out_dim: usize,
	pub act: Activation,
	// Embed: embedding width / table rows. Attn: model dim d (per token) / heads.
	pub dim: usize,
	pub vocab: usize,
	// Attn only: K/V/output projections [d×d] each, and head count (else dummy len-1 / 0).
	pub wk: GpuBuffer,
	pub wv: GpuBuffer,
	pub wo: GpuBuffer,
	pub heads: usize,
	// PRelu only: the learnable negative slope (a single [1] scalar, SGD-updated).
	// Dummy len-1 for every other activation.
	pub palpha: GpuBuffer,
	// Conv only: input channels, kernel size, stride. Dense/Embed/Attn: all 0.
	pub conv_cin: usize,
	pub conv_k: usize,
	pub conv_stride: usize,
}

/// If the network is an embed/attn text prefix followed by a dense head, return
/// `(first_dense_index, attn_out_dim A, categorical_dim C)` — the dense at that
/// index reads `concat(prefix_output[A], x_cat[C])`. None when there's no prefix
/// or no extra categorical features (C==0, e.g. all columns are text).
pub fn concat_layer(params: &[LayerParams]) -> Option<(usize, usize, usize)> {
	concat_layer_dims_iter(params.iter().map(|p| (p.kind, p.in_dim, p.out_dim)))
}

/// `concat_layer` over the host-only dims mirror (plan pass, no GPU buffers).
pub fn concat_layer_dims(dims: &[LayerDims]) -> Option<(usize, usize, usize)> {
	concat_layer_dims_iter(dims.iter().map(|d| (d.kind, d.in_dim, d.out_dim)))
}

fn concat_layer_dims_iter(
	it: impl Iterator<Item = (LayerKind, usize, usize)>,
) -> Option<(usize, usize, usize)> {
	let layers: Vec<(LayerKind, usize, usize)> = it.collect();
	for l in 1..layers.len() {
		let (prev_kind, _, prev_out) = layers[l - 1];
		let (kind, in_dim, _) = layers[l];
		if kind == LayerKind::Dense && matches!(prev_kind, LayerKind::Embed | LayerKind::Attn) {
			let a = prev_out;
			let c = in_dim.saturating_sub(a);
			return (c > 0).then_some((l, a, c));
		}
	}
	None
}

/// Per-feature standardizer fit on the train set, reused verbatim on eval so
/// train and eval see the same scaling (no leakage, no drift).
pub struct Scaler {
	pub mean: Vec<f64>,
	pub std: Vec<f64>,
}

/// The fixed vocab pinned on the first `embed` layer, if any. When `Some`, the
/// embed token table is sized to this verbatim and the `max id + 1` data
/// derivation is bypassed everywhere (fit, resume, preflight).
pub fn pinned_vocab(specs: &[LayerSpec]) -> Option<usize> {
	specs.iter().find_map(|s| match s {
		LayerSpec::Embed(_, v) => *v,
		_ => None,
	})
}

/// Dims-only mirror of `LayerParams` — everything sizing/preflight needs,
/// available from the host-only plan pass before any GPU work.
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

/// Philox-2x32 (10 rounds) — the exact host mirror of the `philox2x32` device
/// kernel, so the host-composed randn init draws from the same stream the old
/// device `gpu_randn` did (bit-identical up to libm transcendental ULPs).
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

/// Host randn matching the device `randn_kernel` op-for-op: Box-Muller over two
/// philox uniforms, then scaled by the per-layer-kind init scale (He / 0.1 /
/// 1/√d_tok) — the same scale the device `gpu_scale_inplace` applied. Fills the
/// block host-side so the init runs ZERO device kernels.
fn host_randn(seed: u32, scale: f64, out: &mut [f64]) {
	for (i, o) in out.iter_mut().enumerate() {
		let u1 = philox_uniform((2 * i) as u32, seed).max(1e-30);
		let u2 = philox_uniform((2 * i + 1) as u32, seed);
		*o = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * scale;
	}
}

/// A planned block's location in the plan's host image. Every block is composed
/// host-side (resumed weights, PE tables, biases, zeros, and now randn init too),
/// so `materialize` only carves views — no device init kernels remain.
struct BlockPlan {
	off: usize, // f64 offset into the plan's host image
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

/// The AOT weight plan: every layer's block sizes, offsets, and initial host
/// bytes composed on the host with ZERO GPU calls. The run pushes `host()`
/// into its one init image (the run's persist prefix — the exit block
/// downloads it back in the same single D2H), then `materialize` carves views
/// and runs the randn init kernels.
pub struct LayerPlan {
	entries: Vec<PlanEntry>,
	host: Vec<f64>,
}

const PLAN_ALIGN_F64: usize = 32; // 256-byte blocks, kernel-clean views

impl LayerPlan {
	fn pad(&mut self) {
		let rem = self.host.len() % PLAN_ALIGN_F64;
		if rem != 0 {
			self.host.resize(self.host.len() + PLAN_ALIGN_F64 - rem, 0.0);
		}
	}

	fn push(&mut self, data: &[f64]) -> BlockPlan {
		self.pad();
		let off = self.host.len();
		self.host.extend_from_slice(data);
		BlockPlan { off, len: data.len() }
	}

	fn zeros(&mut self, len: usize) -> BlockPlan {
		self.pad();
		let off = self.host.len();
		self.host.resize(off + len, 0.0);
		BlockPlan { off, len }
	}

	/// Compose a randn-init block directly into the host image (host Box-Muller +
	/// scale), returning a plain host-composed block — the device runs no init kernel.
	fn randn(&mut self, len: usize, seed: usize, scale: f64) -> BlockPlan {
		self.pad();
		let off = self.host.len();
		self.host.resize(off + len, 0.0);
		host_randn(seed as u32, scale, &mut self.host[off..off + len]);
		BlockPlan { off, len }
	}

	/// The composed host image — push this (whole) into the run's init stage.
	pub fn host(&self) -> &[f64] {
		&self.host
	}

	pub fn dims(&self) -> Vec<LayerDims> {
		self.entries.iter().map(|e| e.dims).collect()
	}

	pub fn out_dim_last(&self) -> usize {
		self.entries.last().map_or(0, |e| e.dims.out_dim)
	}

	/// Carve every block as a view of the uploaded image (`base_off` = where the
	/// image landed in the init image buffer, in f64s). The image already holds every
	/// block's contents (weights, PE tables, biases, host-composed randn init), so
	/// this is pure pointer arithmetic — ZERO device kernels, ZERO transfers.
	pub fn materialize(&self, staged: &GpuBuffer, base_off: usize) -> Vec<LayerParams> {
		let view = |bp: &BlockPlan| -> GpuBuffer {
			staged.view(base_off + bp.off, bp.len.max(1))
		};
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

	/// Byte-identical `ogdl::dump_ogdl` from a HOST image of the (trained) weights
	/// in this plan's layout — the write-only save mirror. The run's single exit
	/// D2H brings the weight prefix home; this formats it to OGDL with ZERO device
	/// transfers, field-for-field matching the per-buffer device dump (same
	/// z-numbering, same row/col order, same `f64::to_string` precision). `image`
	/// begins at the weight block (plan-image offset 0).
	pub fn dump_ogdl_host(&self, image: &[f64], key: &str, score: f64) -> String {
		let blk = |bp: &BlockPlan| image[bp.off..bp.off + bp.len].to_vec();
		ogdl_text(|g| {
			g.add(score, key); // metric header: `{key} {score}`
			let mut z = 1;
			for e in &self.entries {
				let d = &e.dims;
				match d.kind {
					LayerKind::Embed => {
						let table = blk(&e.w);
						for id in 0..d.vocab {
							g.add(table[id * d.dim..(id + 1) * d.dim].to_vec(), &format!("embed.{id}"));
						}
					}
					LayerKind::Attn => {
						g.add(blk(&e.w), "attn.wq");
						g.add(blk(&e.wk), "attn.wk");
						g.add(blk(&e.wv), "attn.wv");
						g.add(blk(&e.wo), "attn.wo");
						let bias = blk(&e.b);
						for nm in ["bq", "bk", "bv", "bo"] {
							g.add(bias.clone(), &format!("attn.{nm}"));
						}
					}
					LayerKind::Conv => {
						let lin = d.in_dim / d.conv_cin;
						let lout = (lin - d.conv_k) / d.conv_stride + 1;
						let cout = d.out_dim / lout;
						g.add(vec![cout as f64, d.conv_cin as f64, d.conv_k as f64, d.conv_stride as f64], "conv");
						g.add(blk(&e.w), "conv.w");
						g.add(blk(&e.b), "conv.b");
					}
					LayerKind::Dense => {
						let w = blk(&e.w);
						let b = blk(&e.b);
						let slope = (d.act == Activation::PRelu).then(|| image[e.palpha.off]);
						for j in 0..d.out_dim {
							let row: Vec<f64> = (0..d.in_dim).map(|i| w[i * d.out_dim + j]).collect();
							g.add(row, &format!("z{z}.w"));
							if let Some(a) = slope {
								g.add(a, &format!("z{z}.a"));
							}
							g.add(b[j], &format!("z{z}.b"));
							z += 1;
						}
					}
				}
			}
		})
	}
}

// Serialize an ogdl graph built by `build` to text through the crate's own
// four-method API (build → `file` → read). The crate writes to files, so this
// round-trips a private temp — the checkpoint codec is now just `add` calls.
pub(crate) fn ogdl_text(build: impl FnOnce(ogdl::Graph)) -> String {
	static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
	let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let tmp = std::env::temp_dir().join(format!("nrs_dump_{}_{seq}.ogdl", std::process::id()));
	let tp = tmp.to_str().expect("utf8 tmp");
	let _ = std::fs::remove_file(tp);
	let g = ogdl::file(tp); // fresh empty graph (tmp absent)
	build(g);
	g.file(tp); // populated graph → write out
	let text = std::fs::read_to_string(tp).unwrap_or_default();
	let _ = std::fs::remove_file(tp);
	text
}

/// Host-only plan pass: same walk, same shapes, same seeds, same resume
/// validation the fit builder does, but composes every block into a host
/// image instead of touching the GPU. The fit path plans → sizes scratch →
/// claims the arena → uploads ONE init image → materializes.
pub fn plan_layer_params(
	specs: &[LayerSpec],
	d: usize,
	c_cat: usize,
	vocab: usize,
	resumed: &[Saved],
	try_resume: bool,
) -> Result<LayerPlan, String> {
	let mut plan = LayerPlan { entries: Vec::new(), host: Vec::new() };
	// One shared zero scalar backs every never-touched dummy slot (non-attn
	// wk/wv/wo, non-PRelu palpha) — they are read-only placeholders.
	let dummy_off = plan.zeros(1).off;
	let dummy = || BlockPlan { off: dummy_off, len: 1 };
	let mut si = 0usize;
	let mut in_dim = d;
	for (li, spec) in specs.iter().enumerate() {
		if let LayerSpec::Embed(dim, _) = *spec {
			let w = if try_resume {
				let t = match resumed.get(si) {
					Some(Saved::Embed(t)) => t,
					_ => return Err(format!("layer {li}: checkpoint has no embed block here")),
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
				if e.dims.kind == LayerKind::Embed { e.dims.dim } else { e.dims.out_dim }
			});
			assert!(
				in_dim % d_tok == 0,
				"attn: input {in_dim} not a multiple of token dim {d_tok}"
			);
			assert!(
				d_tok.is_multiple_of(heads),
				"attn: token dim {d_tok} not divisible by {heads} heads"
			);
			let need = d_tok * d_tok;
			let (w, wk, wv, wo) = if try_resume {
				let (sq, sk, sv, so) = match resumed.get(si) {
					Some(Saved::Attn { wq, wk, wv, wo, .. }) => (wq, wk, wv, wo),
					_ => return Err(format!("layer {li}: checkpoint has no attn block here")),
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
					let prev_lout = (prev.dims.in_dim / prev.dims.conv_cin - prev.dims.conv_k)
						/ prev.dims.conv_stride + 1;
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
					_ => return Err(format!("layer {li}: checkpoint has no conv block here")),
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
		let (units, act) = match *spec {
			LayerSpec::Dense(u, a) => (u, a),
			_ => unreachable!(),
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
			(plan.randn(in_dim * units, li, scale), plan.zeros(units), None)
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
