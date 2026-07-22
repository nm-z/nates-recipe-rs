use crate::execute::ModelInner;
use pantry::encode::Dataset;
use recipe_infer::vram_estimate;
use recipe_ir::graph::{OpKind, ValueId};
use recipe_ir::{Activation, LayerDims, LayerKind, LayerSpec, SemanticGraph, pinned_vocab};
pub use recipe_ir::Work;

pub struct CatShape {
	cat_cols: usize,
	text_d: usize,
	vocab: usize,
}

pub fn plan_footprint(model: &ModelInner, ds: &Dataset) -> usize {
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let k = ds.n_targets.max(1);
	let embed_first = matches!(model.specs.first(), Some(LayerSpec::Embed(..)));
	let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
	let shape = Some(())
		.filter(|_probe| embed_cats)
		.map(|_probe| {
			let n_oh: usize = ds.onehot_groups.iter().map(|g| g.len).sum();
			CatShape {
				cat_cols: d - n_oh,
				text_d: ds.onehot_groups.len(),
				vocab: n_oh,
			}
		})
		.or_else(|| {
			Some(()).filter(|_probe| embed_first).map(|_probe| {
				let tc = ds.text_cols.len();
				let vocab = pinned_vocab(&model.specs).unwrap_or_else(|| {
					ds.x.iter().cloned().fold(0.0f64, f64::max) as usize + 1
				});
				CatShape {
					cat_cols: d - tc,
					text_d: tc,
					vocab,
				}
			})
		})
		.unwrap_or(CatShape {
			cat_cols: 0,
			text_d: d,
			vocab: 0,
		});
	vram_estimate(
		&model.specs,
		n,
		shape.text_d,
		k,
		shape.vocab,
		shape.cat_cols,
		false,
	)
}

pub const GEMM_GFLOPS: f64 = 255.0;
pub const VRAM_GBS: f64 = 432.0;

const F8: f64 = 8.0;

pub(crate) fn saves_preact(a: Activation) -> Option<()> {
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

pub fn layer_fwd(p: &LayerDims, n: usize) -> Work {
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

pub fn layer_bwd(p: &LayerDims, n: usize, first: bool) -> Work {
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

pub fn work_from_graph(g: &SemanticGraph, n: usize) -> (Vec<Work>, Vec<Work>) {
	let width = |vid: ValueId| -> usize {
		return g
			.values
			.iter()
			.find(|v| v.id == vid)
			.map_or(0, |v| v.shape.dims.iter().map(|d| d.0).product());
	};
	let base = LayerDims {
		kind: LayerKind::Dense,
		in_dim: 0,
		out_dim: 0,
		act: Activation::Linear,
		dim: 0,
		vocab: 0,
		heads: 0,
		conv_cin: 0,
		conv_k: 0,
		conv_stride: 0,
	};
	let mut dims: Vec<LayerDims> = Vec::new();
	for (i, op) in g.ops.iter().enumerate() {
		let mut ld = match op.kind {
			OpKind::Dense(act) => LayerDims {
				kind: LayerKind::Dense,
				act,
				..base
			},
			OpKind::Embed { dim, vocab } => LayerDims {
				kind: LayerKind::Embed,
				dim,
				vocab,
				..base
			},
			OpKind::Attn { dim, heads } => LayerDims {
				kind: LayerKind::Attn,
				dim,
				heads,
				..base
			},
			OpKind::Conv {
				cin,
				k,
				stride,
				act,
			} => LayerDims {
				kind: LayerKind::Conv,
				conv_cin: cin,
				conv_k: k,
				conv_stride: stride,
				act,
				..base
			},
			OpKind::Activation(_) | OpKind::LossReduce(_) => continue,
		};
		ld.in_dim = op.inputs.iter().map(|v| width(*v)).sum();
		ld.out_dim = op.outputs.first().map_or(0, |v| width(*v));
		if let Some(next) = g.ops.get(i + 1) {
			if let OpKind::Activation(a) = next.kind {
				if next.inputs.first() == op.outputs.first() {
					ld.act = a;
					ld.out_dim = next
						.outputs
						.first()
						.map_or(ld.out_dim, |v| width(*v));
				}
			}
		}
		dims.push(ld);
	}
	let fwd: Vec<Work> = dims.iter().map(|d| layer_fwd(d, n)).collect();
	let bwd: Vec<Work> = dims
		.iter()
		.enumerate()
		.map(|(l, d)| layer_bwd(d, n, l == 0))
		.collect();
	return (fwd, bwd);
}
