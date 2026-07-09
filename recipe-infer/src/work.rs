use crate::enums::{Activation, LayerKind};
use crate::params::LayerParams;

pub const GEMM_GFLOPS: f64 = 255.0;
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

fn saves_preact(a: Activation) -> bool {
	matches!(
		a,
		Activation::Silu | Activation::Gelu | Activation::Elu | Activation::Selu | Activation::PRelu
	)
}

fn act_fwd(a: Activation, m: f64) -> Work {
	match a {
		Activation::Linear => Work::default(),
		_ => Work { flop: 4.0 * m, bytes: 2.0 * F8 * m },
	}
}

fn act_bwd(a: Activation, m: f64) -> Work {
	match a {
		Activation::Linear => Work::default(),
		_ => Work { flop: 4.0 * m, bytes: 3.0 * F8 * m },
	}
}

fn sgd(e: f64) -> Work {
	Work { flop: 2.0 * e, bytes: 3.0 * F8 * e }
}

pub fn layer_fwd(p: &LayerParams, n: usize) -> Work {
	let (nf, i, o) = (n as f64, p.in_dim as f64, p.out_dim as f64);
	let mut w = Work::default();
	match p.kind {
		LayerKind::Dense => {
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
			w.add(4.0 * 2.0 * m * d * d, 4.0 * F8 * (2.0 * m * d + d * d + d));
			w.add(12.0 * m * d, 4.0 * F8 * m * d);
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
			w.add(0.0, F8 * (nf * i + 2.0 * nf * i * dim));
			w.add(nf * o, F8 * (2.0 * nf * o + o));
		}
	}
	w
}

pub fn layer_bwd(p: &LayerParams, n: usize, first: bool) -> Work {
	let (nf, i, o) = (n as f64, p.in_dim as f64, p.out_dim as f64);
	let mut w = Work::default();
	match p.kind {
		LayerKind::Dense => {
			w = w.plus(act_bwd(p.act, nf * o));
			w.add(2.0 * nf * i * o, F8 * (nf * i + nf * o + i * o));
			w.add(nf * o, F8 * (nf * o + o));
			if !first {
				w.add(2.0 * nf * i * o, F8 * (nf * o + i * o + nf * i));
			}
			w = w.plus(sgd(i * o)).plus(sgd(o));
		}
		LayerKind::Attn => {
			let d = p.dim as f64;
			let s = i / d;
			let m = nf * s;
			let h = p.heads as f64;
			w.add(4.0 * 4.0 * m * d * d, 4.0 * F8 * (3.0 * m * d + 2.0 * d * d));
			w.add(10.0 * nf * s * s * d + 8.0 * nf * h * s * s, F8 * (9.0 * m * d + 2.0 * nf * h * s));
			w.add(12.0 * m * d + 2.0 * 2.0 * m * d, F8 * (4.0 * m * d + 2.0 * 3.0 * m * d));
			w = w.plus(sgd(4.0 * d * d));
		}
		LayerKind::Conv => {
			let (cin, k) = (p.conv_cin as f64, p.conv_k as f64);
			let lout = ((p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1) as f64;
			let cout = o / lout;
			w = w.plus(act_bwd(p.act, nf * o));
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
			w.add(0.0, F8 * v * dim);
			w.add(nf * o, F8 * (nf * i + 3.0 * nf * o));
			w = w.plus(sgd(v * dim));
		}
	}
	w
}
