#[derive(Clone, Copy, PartialEq)]
pub enum Activation {
	Relu,
	Sigmoid,
	Linear,
	LeakyRelu,
	PRelu,
	Elu,
	Selu,
	Tanh,
	Silu,
	Gelu,
}

#[derive(Clone, Copy)]
pub enum LayerSpec {
	Dense(usize, Activation),
	Embed(usize, Option<usize>),
	Attn(usize),
	Conv(usize, usize, usize, Activation),
}

#[derive(Clone, Copy, PartialEq)]
pub enum LayerKind {
	Dense,
	Embed,
	Attn,
	Conv,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Loss {
	Mse,
	Mae,
	Ce,
	Bce,
	Huber,
	Focal,
}

impl Loss {
	pub fn is_classification(self) -> bool {
		matches!(self, Loss::Ce | Loss::Bce | Loss::Focal)
	}
	pub fn score_key(self) -> &'static str {
		match self {
			Loss::Ce | Loss::Bce | Loss::Focal => "acc",
			Loss::Mse | Loss::Mae | Loss::Huber => "r2",
		}
	}
	pub fn name(self) -> &'static str {
		match self {
			Loss::Mse => "mse",
			Loss::Mae => "mae",
			Loss::Ce => "ce",
			Loss::Bce => "bce",
			Loss::Huber => "huber",
			Loss::Focal => "focal",
		}
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum Param {
	W,
	B,
}

#[derive(Clone, Copy)]
pub struct ConcatDims {
	pub pf: usize,
	pub a: usize,
	pub c: usize,
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

pub fn concat_layer_dims(dims: &[LayerDims]) -> Option<ConcatDims> {
	concat_layer_dims_iter(dims.iter().map(|d| (d.kind, d.in_dim, d.out_dim)))
}

fn concat_layer_dims_iter(it: impl Iterator<Item = (LayerKind, usize, usize)>) -> Option<ConcatDims> {
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

pub fn pinned_vocab(specs: &[LayerSpec]) -> Option<usize> {
	specs.iter().find_map(|s| match s {
		LayerSpec::Embed(_, v) => *v,
		_ => None,
	})
}
