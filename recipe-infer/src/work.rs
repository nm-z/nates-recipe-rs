//! Analytic per-layer work accounting: FLOPs executed and VRAM bytes touched by
//! one full-batch forward or backward pass of a layer, computed from the actual
//! layer dims — the numerator for every roofline line the fit loop prints.
//! Convention: matrix contractions counted exactly (2·M·N·K), elementwise ops at
//! their per-element cost, every logical kernel operand read or written once;
//! kernel-internal temporaries (split-K partials, reduce workspaces, bounce
//! staging) are not counted.

use crate::enums::{Activation, LayerKind};
use crate::params::LayerParams;

/// Measured ceiling of our f64 GEMM kernels on gfx1101 — GFLOP/s.
pub const GEMM_GFLOPS: f64 = 255.0;
/// gfx1101 VRAM bandwidth — GB/s.
pub const VRAM_GBS: f64 = 432.0;

#[derive(Clone, Copy, Default)]
pub struct Work {
	pub flop: f64,
	pub bytes: f64,
}

impl Work {
	fn add(&mut self, flop: f64, bytes: f64) {
		self.flop += flop;
		self.bytes += bytes;
	}
	pub fn plus(mut self, o: Work) -> Work {
		self.add(o.flop, o.bytes);
		self
	}
}

const F8: f64 = 8.0;

/// Whether the activation's backward reads a saved pre-activation (forward pays
/// an extra copy of z).
fn saves_preact(a: Activation) -> bool {
	matches!(
		a,
		Activation::Silu | Activation::Gelu | Activation::Elu | Activation::Selu | Activation::PRelu
	)
}

/// In-place activation over m elements: ~4 FLOP/elem, one read + one write.
fn act_fwd(a: Activation, m: f64) -> Work {
	match a {
		Activation::Linear => Work::default(),
		_ => Work { flop: 4.0 * m, bytes: 2.0 * F8 * m },
	}
}

/// Activation backward dz = act'·da over m elements: reads da + saved act (or
/// preact), writes dz.
fn act_bwd(a: Activation, m: f64) -> Work {
	match a {
		Activation::Linear => Work::default(),
		_ => Work { flop: 4.0 * m, bytes: 3.0 * F8 * m },
	}
}

/// SGD update p -= lr·g over e elements: read p + g, write p.
fn sgd(e: f64) -> Work {
	Work { flop: 2.0 * e, bytes: 3.0 * F8 * e }
}

/// One full-batch FORWARD pass of layer `p` over `n` samples.
pub fn layer_fwd(p: &LayerParams, n: usize) -> Work {
	let (nf, i, o) = (n as f64, p.in_dim as f64, p.out_dim as f64);
	let mut w = Work::default();
	match p.kind {
		LayerKind::Dense => {
			// z = X·W + b (matvec when o==1 — same operand counts).
			w.add(2.0 * nf * i * o, F8 * (nf * i + i * o + o + nf * o));
			if saves_preact(p.act) {
				w.add(0.0, 2.0 * F8 * nf * o);
			}
			w = w.plus(act_fwd(p.act, nf * o));
		}
		LayerKind::Attn => {
			let d = p.dim as f64;
			let s = i / d;
			let m = nf * s;
			let h = p.heads as f64;
			// Wq/Wk/Wv/Wo projections.
			w.add(4.0 * 2.0 * m * d * d, 4.0 * F8 * (2.0 * m * d + d * d + d));
			// RoPE on Q,K.
			w.add(12.0 * m * d, 4.0 * F8 * m * d);
			// Flash: QKᵀ + PV per head (2·2·S²·hd over n·heads = 4·n·S²·d) plus
			// the online softmax (~4 FLOP per score); streams Q,K,V, writes ctx+lse.
			w.add(4.0 * nf * s * s * d + 4.0 * nf * h * s * s, F8 * (4.0 * m * d + nf * h * s));
		}
		LayerKind::Conv => {
			let (cin, k) = (p.conv_cin as f64, p.conv_k as f64);
			let lout = ((p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1) as f64;
			let cout = o / lout;
			w.add(2.0 * nf * cout * lout * cin * k, F8 * (nf * i + cout * cin * k + cout + nf * o));
			if saves_preact(p.act) {
				w.add(0.0, 2.0 * F8 * nf * o);
			}
			w = w.plus(act_fwd(p.act, nf * o));
		}
		LayerKind::Embed => {
			let dim = p.dim as f64;
			// Gather (ids + table rows read, out written) + positional add.
			w.add(0.0, F8 * (nf * i + 2.0 * nf * i * dim));
			w.add(nf * o, F8 * (2.0 * nf * o + o));
		}
	}
	w
}

/// One full-batch BACKWARD + SGD pass of layer `p` over `n` samples. `first`
/// = layer 0 (no input-gradient GEMM below it).
pub fn layer_bwd(p: &LayerParams, n: usize, first: bool) -> Work {
	let (nf, i, o) = (n as f64, p.in_dim as f64, p.out_dim as f64);
	let mut w = Work::default();
	match p.kind {
		LayerKind::Dense => {
			w = w.plus(act_bwd(p.act, nf * o));
			// dW = aᵀ·dz, db = Σdz.
			w.add(2.0 * nf * i * o, F8 * (nf * i + nf * o + i * o));
			w.add(nf * o, F8 * (nf * o + o));
			if !first {
				// da_below = dz·Wᵀ.
				w.add(2.0 * nf * i * o, F8 * (nf * o + i * o + nf * i));
			}
			w = w.plus(sgd(i * o)).plus(sgd(o));
		}
		LayerKind::Attn => {
			let d = p.dim as f64;
			let s = i / d;
			let m = nf * s;
			let h = p.heads as f64;
			// Wo backward (dctx + dWo), then Wq/Wk/Wv backwards (dH + dW each).
			w.add(4.0 * 4.0 * m * d * d, 4.0 * F8 * (3.0 * m * d + 2.0 * d * d));
			// Flash backward: recompute P, dS, dQ, dK, dV — five S²·hd contractions
			// per head; reads q/k/v/ctx/dctx/lse, writes dsum/dq/dk/dv.
			w.add(10.0 * nf * s * s * d + 8.0 * nf * h * s * s, F8 * (9.0 * m * d + 2.0 * nf * h * s));
			// Un-RoPE dQ,dK + the two dH accumulations.
			w.add(12.0 * m * d + 2.0 * 2.0 * m * d, F8 * (4.0 * m * d + 2.0 * 3.0 * m * d));
			w = w.plus(sgd(4.0 * d * d));
		}
		LayerKind::Conv => {
			let (cin, k) = (p.conv_cin as f64, p.conv_k as f64);
			let lout = ((p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1) as f64;
			let cout = o / lout;
			w = w.plus(act_bwd(p.act, nf * o));
			// Filter grad + bias grad + (data grad below).
			w.add(2.0 * nf * cout * lout * cin * k, F8 * (nf * o + nf * i + cout * cin * k));
			w.add(nf * o, F8 * (nf * o + cout));
			if !first {
				w.add(2.0 * nf * cout * lout * cin * k, F8 * (nf * o + cout * cin * k + nf * i));
			}
			w = w.plus(sgd(cout * cin * k)).plus(sgd(cout));
		}
		LayerKind::Embed => {
			let dim = p.dim as f64;
			let v = p.vocab as f64;
			// Zero the table grad, scatter-add da by token id, SGD the table.
			w.add(0.0, F8 * v * dim);
			w.add(nf * o, F8 * (nf * i + 3.0 * nf * o));
			w = w.plus(sgd(v * dim));
		}
	}
	w
}
