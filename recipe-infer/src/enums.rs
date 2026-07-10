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
pub enum Metric {
	Loss,
	Accuracy,
	Epoch,
	Lr,
	Time,
	R2,
	Hip,
}

pub struct MetricFmt {
	pub label: &'static str,
	pub width: usize,
}

impl Metric {
	pub fn fmt(self) -> MetricFmt {
		match self {
			Metric::Epoch => MetricFmt { label: "epoch", width: 5 },
			Metric::Lr => MetricFmt { label: "lr", width: 7 },
			Metric::Time => MetricFmt { label: "time", width: 9 },
			Metric::Loss => MetricFmt { label: "loss", width: 7 },
			Metric::Accuracy => MetricFmt { label: "acc", width: 6 },
			Metric::R2 => MetricFmt { label: "r2", width: 8 },
			Metric::Hip => MetricFmt { label: "hip", width: 0 },
		}
	}

	pub fn render(self, v: f64) -> String {
		let w = self.fmt().width;
		v.partial_cmp(&v).map_or(
			format!("{:>w$}", "N/A"),
			|_finite| match self {
				Metric::Epoch => format!("{:>w$}", v as usize),
				Metric::Time => format!("{:>w$}", fmt_time(v)),
				Metric::Hip => String::new(),
				Metric::Loss | Metric::Accuracy | Metric::Lr | Metric::R2 => format!("{v:>w$.4}"),
			},
		)
	}
}

pub fn fmt_time(secs: f64) -> String {
	let s = secs as u64;
	let h = s / 3600;
	let m = (s % 3600) / 60;
	let sec = s % 60;
	match h.checked_sub(1) {
		Some(_hh) => format!("{h}h {m:02}m {sec:02}s"),
		None => match m.checked_sub(1) {
			Some(_mm) => format!("{m}m {sec:02}s"),
			None => format!("{secs:.1}s"),
		},
	}
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
