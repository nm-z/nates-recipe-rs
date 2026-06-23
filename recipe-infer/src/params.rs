//! Layer parameters and their construction: the per-layer GPU weight buffers,
//! the resume-checkpoint block type, the positional-encoding table, the
//! two-branch concat detector, and `build_layer_params` (random init or resume).

use crate::enums::{Activation, LayerKind, LayerSpec};
use crate::ogdl::Saved;
use gpu_core::kernels;
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
	for l in 1..params.len() {
		let prev = &params[l - 1];
		if params[l].kind == LayerKind::Dense
			&& matches!(prev.kind, LayerKind::Embed | LayerKind::Attn)
		{
			let a = prev.out_dim;
			let c = params[l].in_dim.saturating_sub(a);
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

pub fn build_layer_params(
	specs: &[LayerSpec],
	d: usize,
	c_cat: usize,
	vocab: usize,
	resumed: &[Saved],
	try_resume: bool,
) -> Result<Vec<LayerParams>, String> {
	let mut si = 0usize;
	let mut params: Vec<LayerParams> = Vec::new();
	let mut in_dim = d;
	let dummy = || GpuBuffer::upload(&[0.0f64]).expect("dummy buf");
	for (li, spec) in specs.iter().enumerate() {
		if let LayerSpec::Embed(dim, _) = *spec {
			// Token table [vocab×dim]. On resume, upload the saved table;
			// else small-random init (embeddings want O(0.1) scale, not He).
			// in_dim columns of token ids → in_dim*dim wide output. `b` holds
			// the NEGATED sinusoidal positional encoding [in_dim*dim], always
			// recomputed (deterministic, never saved). No activation.
			let table = if try_resume {
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
				GpuBuffer::upload(t).expect("upload embed table")
			} else {
				let table = kernels::gpu_randn(
					vocab * dim,
					4242 + (li as u32) * 7919,
				)
				.expect("randn embed");
				kernels::gpu_scale_inplace(&table, 0.1, vocab * dim);
				table
			};
			let neg_pe = sinusoidal_pe(in_dim, dim, true);
			let b = GpuBuffer::upload(&neg_pe).expect("upload pe");
			params.push(LayerParams {
				kind: LayerKind::Embed,
				w: table,
				b,
				in_dim,
				out_dim: in_dim * dim,
				act: Activation::Linear,
				dim,
				vocab,
				wk: dummy(),
				wv: dummy(),
				wo: dummy(),
				heads: 0,
				palpha: dummy(),
				conv_cin: 0, conv_k: 0, conv_stride: 0,
			});
			in_dim *= dim;
			continue;
		}
		if let LayerSpec::Attn(heads) = *spec {
			// Bare multi-head self-attention; input is [n, S*d] with d = in_dim/S.
			// d (the per-token width) = the previous embed dim. We recover it from
			// the embed layer: in_dim here = S*d, and d = embed dim. heads | d.
			let d_tok = params.last().map_or(in_dim, |p| {
				if p.kind == LayerKind::Embed {
					p.dim
				} else {
					p.out_dim
				}
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
					Some(Saved::Attn { wq, wk, wv, wo, .. }) => {
						(wq, wk, wv, wo)
					}
					_ => {
						return Err(format!(
							"layer {li}: checkpoint has no attn block here"
						));
					}
				};
				for (nm, v) in [("wq", sq), ("wk", sk), ("wv", sv), ("wo", so)]
				{
					if v.len() != need {
						return Err(format!(
							"layer {li} attn {nm}: checkpoint has {} values, model needs {need} (token dim {d_tok}²)",
							v.len()
						));
					}
				}
				si += 1;
				(
					GpuBuffer::upload(sq).expect("upload wq"),
					GpuBuffer::upload(sk).expect("upload wk"),
					GpuBuffer::upload(sv).expect("upload wv"),
					GpuBuffer::upload(so).expect("upload wo"),
				)
			} else {
				let mk = |seed: u32| {
					let w =
						kernels::gpu_randn(need, seed).expect("randn attn");
					kernels::gpu_scale_inplace(
						&w,
						(1.0 / d_tok as f64).sqrt(),
						need,
					);
					w
				};
				(
					mk(7001 + li as u32 * 13),
					mk(7002 + li as u32 * 13),
					mk(7003 + li as u32 * 13),
					mk(7004 + li as u32 * 13),
				)
			};
			params.push(LayerParams {
				kind: LayerKind::Attn,
				w,
				b: GpuBuffer::upload(&vec![0.0f64; d_tok]).expect("attn bias"),
				in_dim,
				out_dim: in_dim,
				act: Activation::Linear,
				dim: d_tok,
				vocab: 0,
				wk,
				wv,
				wo,
				heads,
				palpha: dummy(),
				conv_cin: 0, conv_k: 0, conv_stride: 0,
			});
			continue;
		}
		if let LayerSpec::Conv(filters, kernel, stride, act) = *spec {
			let cin = if let Some(prev) = params.last() {
				if prev.kind == LayerKind::Conv {
					let prev_lout = (prev.in_dim / prev.conv_cin - prev.conv_k) / prev.conv_stride + 1;
					prev.out_dim / prev_lout
				} else {
					1
				}
			} else {
				1
			};
			let lin = in_dim / cin;
			let lout = (lin - kernel) / stride + 1;
			let w_count = filters * cin * kernel;
			let (w, b) = if !try_resume {
				let scale = (2.0 / (cin * kernel) as f64).sqrt();
				let w = kernels::gpu_randn(w_count, 5678 + (li as u32) * 7919)
					.expect("randn conv w");
				kernels::gpu_scale_inplace(&w, scale, w_count);
				let b = GpuBuffer::upload(&vec![0.0f64; filters]).expect("upload conv b");
				(w, b)
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
				(
					GpuBuffer::upload(ws).expect("upload conv w"),
					GpuBuffer::upload(bs).expect("upload conv b"),
				)
			};
			let palpha = if act == Activation::PRelu {
				GpuBuffer::upload(&[PRELU_INIT]).expect("prelu alpha")
			} else {
				dummy()
			};
			params.push(LayerParams {
				kind: LayerKind::Conv,
				w,
				b,
				in_dim,
				out_dim: filters * lout,
				act,
				dim: 0,
				vocab: 0,
				wk: dummy(),
				wv: dummy(),
				wo: dummy(),
				heads: 0,
				palpha,
				conv_cin: cin,
				conv_k: kernel,
				conv_stride: stride,
			});
			in_dim = filters * lout;
			continue;
		}
		let (units, act) = match *spec {
			LayerSpec::Dense(u, a) => (u, a),
			_ => unreachable!(),
		};
		// First dense after the embed/attn prefix: its input is
		// concat(prefix_out, x_cat), so widen in_dim by the categorical
		// count exactly once (fires only when the prior layer is prefix).
		if c_cat > 0
			&& matches!(
				params.last().map(|p| p.kind),
				Some(LayerKind::Embed | LayerKind::Attn)
			) {
			in_dim += c_cat;
		}
		let (w, b, slope) = if !try_resume {
			let scale = (2.0 / in_dim as f64).sqrt();
			let w = kernels::gpu_randn(in_dim * units, 1234 + (li as u32) * 7919)
				.expect("randn w");
			kernels::gpu_scale_inplace(&w, scale, in_dim * units);
			let b = GpuBuffer::upload(&vec![0.0f64; units]).expect("upload b");
			(w, b, None)
		} else {
			// Distribute saved neurons back into this layer's W (in_dim×units,
			// row-major index i*units+j) and bias[j], matching dump_ogdl's layout.
			// A PReLU layer shares one slope across neurons → take the first `a`.
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
			(
				GpuBuffer::upload(&wh).expect("upload w"),
				GpuBuffer::upload(&bh).expect("upload b"),
				slope,
			)
		};
		let palpha = if act == Activation::PRelu {
			GpuBuffer::upload(&[slope.unwrap_or(PRELU_INIT)])
				.expect("prelu alpha")
		} else {
			dummy()
		};
		params.push(LayerParams {
			kind: LayerKind::Dense,
			w,
			b,
			in_dim,
			out_dim: units,
			act,
			dim: 0,
			vocab: 0,
			wk: dummy(),
			wv: dummy(),
			wo: dummy(),
			heads: 0,
			palpha,
			conv_cin: 0, conv_k: 0, conv_stride: 0,
		});
		in_dim = units;
	}
	// Every saved block must be consumed: a leftover means the checkpoint has
	// more layers/neurons than this architecture (wrong file or changed arch).
	if try_resume && si != resumed.len() {
		return Err(format!(
			"checkpoint has {} saved blocks, this architecture consumed {si}",
			resumed.len()
		));
	}
	Ok(params)
}
