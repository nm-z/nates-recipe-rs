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

#[derive(Clone, Copy, PartialEq)]
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
		if self.is_classification() {
			"acc"
		} else {
			"r2"
		}
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum Metric {
	Loss,
	Accuracy,
	Epoch,
	Lr,
	Time,
	R2,
	Hip,
}

mod alias {
	use super::{Activation, Loss, Metric};

	pub const RELU: Activation = Activation::Relu;
	pub const SIG: Activation = Activation::Sigmoid;
	pub const LINEAR: Activation = Activation::Linear;
	pub const LEAK: Activation = Activation::LeakyRelu;
	pub const PRELU: Activation = Activation::PRelu;
	pub const ELU: Activation = Activation::Elu;
	pub const SELU: Activation = Activation::Selu;
	pub const TANH: Activation = Activation::Tanh;
	pub const SILU: Activation = Activation::Silu;
	pub const SWISH: Activation = Activation::Silu;
	pub const GELU: Activation = Activation::Gelu;

	pub const MSE: Loss = Loss::Mse;
	pub const MAE: Loss = Loss::Mae;
	pub const CE: Loss = Loss::Ce;
	pub const BCE: Loss = Loss::Bce;
	pub const HUBER: Loss = Loss::Huber;
	pub const FOCAL: Loss = Loss::Focal;

	pub const LOSS: Metric = Metric::Loss;
	pub const ACCURACY: Metric = Metric::Accuracy;
	pub const EPOCH: Metric = Metric::Epoch;
	pub const LR: Metric = Metric::Lr;
	pub const TIME: Metric = Metric::Time;
	pub const R_TWO: Metric = Metric::R2;
	pub const HIP: Metric = Metric::Hip;
}

pub use alias::{
	BCE as bce, CE as ce, ELU as elu, FOCAL as focal, GELU as gelu, HUBER as huber, LEAK as leak,
	LINEAR as linear, MAE as mae, MSE as mse, PRELU as prelu, RELU as relu, SELU as selu,
	SIG as sig, SILU as silu, SWISH as swish, TANH as tanh,
};
pub use alias::{
	ACCURACY as Accuracy, EPOCH as Epoch, HIP as hip, LOSS as Loss, LR as Lr, R_TWO as R2,
	TIME as Time,
};

#[derive(Clone, Copy, PartialEq)]
pub enum Param {
	W,
	B,
}
