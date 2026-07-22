use crate::enums::{Activation, LayerKind};
use crate::params::LayerParams;
pub use recipe_ir::Work;

pub const GEMM_GFLOPS: f64 = 255.0;
pub const VRAM_GBS: f64 = 432.0;

const F8: f64 = 8.0;

fn saves_preact(a: Activation) -> Option<()> {
	Some(()).filter(|_u| {
		matches!(
			a,
			Activation::Silu
				| Activation::Gelu | Activation::Elu
				| Activation::Selu | Activation::PRelu
		)
	})
}

fn act_fwd(a: Activation, m: f64) -> Work {
	match a {
		Activation::Linear => Work::default(),
		Activation::Relu
		| Activation::Sigmoid
		| Activation::LeakyRelu
		| Activation::PRelu
		| Activation::Elu
		| Activation::Selu
		| Activation::Tanh
		| Activation::Silu
		| Activation::Gelu => Work {
			flop: 4.0 * m,
			bytes: 2.0 * F8 * m,
		},
	}
}

fn act_bwd(a: Activation, m: f64) -> Work {
	match a {
		Activation::Linear => Work::default(),
		Activation::Relu
		| Activation::Sigmoid
		| Activation::LeakyRelu
		| Activation::PRelu
		| Activation::Elu
		| Activation::Selu
		| Activation::Tanh
		| Activation::Silu
		| Activation::Gelu => Work {
			flop: 4.0 * m,
			bytes: 3.0 * F8 * m,
		},
	}
}

fn sgd(e: f64) -> Work {
	Work {
		flop: 2.0 * e,
		bytes: 3.0 * F8 * e,
	}
}

pub fn layer_fwd(p: &LayerParams, n: usize) -> Work {
	let nf = n as f64;
	let i = p.in_dim as f64;
	let o = p.out_dim as f64;
	let mut w = Work::default();
	match p.kind {
		LayerKind::Dense => {
			w.add(2.0 * nf * i * o, F8 * (nf * i + i * o + o + nf * o));
			for _u in saves_preact(p.act).into_iter() {
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
			w.add(
				4.0 * nf * s * s * d + 4.0 * nf * h * s * s,
				F8 * (4.0 * m * d + nf * h * s),
			);
		}
		LayerKind::Conv => {
			let cin = p.conv_cin as f64;
			let k = p.conv_k as f64;
			let lout = ((p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1) as f64;
			let cout = o / lout;
			w.add(
				2.0 * nf * cout * lout * cin * k,
				F8 * (nf * i + cout * cin * k + cout + nf * o),
			);
			for _u in saves_preact(p.act).into_iter() {
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
	let nf = n as f64;
	let i = p.in_dim as f64;
	let o = p.out_dim as f64;
	let mut w = Work::default();
	match p.kind {
		LayerKind::Dense => {
			w = w.plus(act_bwd(p.act, nf * o));
			w.add(2.0 * nf * i * o, F8 * (nf * i + nf * o + i * o));
			w.add(nf * o, F8 * (nf * o + o));
			for _u in Some(()).filter(|_g| !first).into_iter() {
				w.add(2.0 * nf * i * o, F8 * (nf * o + i * o + nf * i));
			}
			w = w.plus(sgd(i * o)).plus(sgd(o));
		}
		LayerKind::Attn => {
			let d = p.dim as f64;
			let s = i / d;
			let m = nf * s;
			let h = p.heads as f64;
			w.add(
				4.0 * 4.0 * m * d * d,
				4.0 * F8 * (3.0 * m * d + 2.0 * d * d),
			);
			w.add(
				10.0 * nf * s * s * d + 8.0 * nf * h * s * s,
				F8 * (9.0 * m * d + 2.0 * nf * h * s),
			);
			w.add(
				12.0 * m * d + 2.0 * 2.0 * m * d,
				F8 * (4.0 * m * d + 2.0 * 3.0 * m * d),
			);
			w = w.plus(sgd(4.0 * d * d));
		}
		LayerKind::Conv => {
			let cin = p.conv_cin as f64;
			let k = p.conv_k as f64;
			let lout = ((p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1) as f64;
			let cout = o / lout;
			w = w.plus(act_bwd(p.act, nf * o));
			w.add(
				2.0 * nf * cout * lout * cin * k,
				F8 * (nf * o + nf * i + cout * cin * k),
			);
			w.add(nf * o, F8 * (nf * o + cout));
			for _u in Some(()).filter(|_g| !first).into_iter() {
				w.add(
					2.0 * nf * cout * lout * cin * k,
					F8 * (nf * o + cout * cin * k + nf * i),
				);
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
