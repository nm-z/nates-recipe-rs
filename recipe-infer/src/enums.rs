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

#[allow(non_upper_case_globals)]
pub const relu: Activation = Activation::Relu;
#[allow(non_upper_case_globals)]
pub const sig: Activation = Activation::Sigmoid;
#[allow(non_upper_case_globals)]
pub const linear: Activation = Activation::Linear;
#[allow(non_upper_case_globals)]
pub const leak: Activation = Activation::LeakyRelu;
#[allow(non_upper_case_globals)]
pub const prelu: Activation = Activation::PRelu;
#[allow(non_upper_case_globals)]
pub const elu: Activation = Activation::Elu;
#[allow(non_upper_case_globals)]
pub const selu: Activation = Activation::Selu;
#[allow(non_upper_case_globals)]
pub const tanh: Activation = Activation::Tanh;
#[allow(non_upper_case_globals)]
pub const silu: Activation = Activation::Silu;
#[allow(non_upper_case_globals)]
pub const swish: Activation = Activation::Silu;
#[allow(non_upper_case_globals)]
pub const gelu: Activation = Activation::Gelu;
#[allow(non_upper_case_globals)]
pub const mse: Loss = Loss::Mse;
#[allow(non_upper_case_globals)]
pub const mae: Loss = Loss::Mae;
#[allow(non_upper_case_globals)]
pub const ce: Loss = Loss::Ce;
#[allow(non_upper_case_globals)]
pub const bce: Loss = Loss::Bce;
#[allow(non_upper_case_globals)]
pub const huber: Loss = Loss::Huber;
#[allow(non_upper_case_globals)]
pub const focal: Loss = Loss::Focal;

#[allow(non_upper_case_globals)]
pub const Loss: Metric = Metric::Loss;
#[allow(non_upper_case_globals)]
pub const Accuracy: Metric = Metric::Accuracy;
#[allow(non_upper_case_globals)]
pub const Epoch: Metric = Metric::Epoch;
#[allow(non_upper_case_globals)]
pub const Lr: Metric = Metric::Lr;
#[allow(non_upper_case_globals)]
pub const Time: Metric = Metric::Time;
#[allow(non_upper_case_globals)]
pub const R2: Metric = Metric::R2;
#[allow(non_upper_case_globals)]
pub const hip: Metric = Metric::Hip;

#[derive(Clone, Copy, PartialEq)]
pub enum Param {
	W,
	B,
}
