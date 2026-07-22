pub use recipe_ir::{Activation, LayerKind, LayerSpec, LogItem, Loss, Metric, Param};

pub struct MetricFmt {
	pub label: &'static str,
	pub width: usize,
}

pub fn metric_fmt(m: Metric) -> MetricFmt {
	match m {
		Metric::Epoch => MetricFmt {
			label: "epoch",
			width: 5,
		},
		Metric::Lr => MetricFmt {
			label: "lr",
			width: 7,
		},
		Metric::Time => MetricFmt {
			label: "time",
			width: 9,
		},
		Metric::Loss => MetricFmt {
			label: "loss",
			width: 7,
		},
		Metric::Accuracy => MetricFmt {
			label: "acc",
			width: 6,
		},
		Metric::R2 => MetricFmt {
			label: "r2",
			width: 8,
		},
	}
}

pub fn metric_pinned_slot(m: Metric) -> Option<usize> {
	return match m {
		Metric::Loss => Some(0),
		Metric::Accuracy => Some(1),
		Metric::R2 => Some(2),
		Metric::Epoch | Metric::Lr | Metric::Time => None,
	};
}

pub fn metric_render(m: Metric, v: f64) -> String {
	let w = metric_fmt(m).width;
	v.partial_cmp(&v)
		.map_or(format!("{:>w$}", "N/A"), |_finite| match m {
			Metric::Epoch => format!("{:>w$}", v as usize),
			Metric::Time => format!("{:>w$}", fmt_time(v)),
			Metric::Loss | Metric::Accuracy | Metric::Lr | Metric::R2 => {
				format!("{v:>w$.4}")
			}
		})
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
	use super::{Activation, LogItem, Loss, Metric};

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

	pub const LOSS: LogItem = LogItem::Metric(Metric::Loss);
	pub const ACCURACY: LogItem = LogItem::Metric(Metric::Accuracy);
	pub const EPOCH: LogItem = LogItem::Metric(Metric::Epoch);
	pub const LR: LogItem = LogItem::Metric(Metric::Lr);
	pub const TIME: LogItem = LogItem::Metric(Metric::Time);
	pub const R_TWO: LogItem = LogItem::Metric(Metric::R2);
	pub const HIP: LogItem = LogItem::Device;
}

pub use alias::{
	ACCURACY as Accuracy, EPOCH as Epoch, HIP as hip, LOSS as Loss, LR as Lr, R_TWO as R2,
	TIME as Time,
};
pub use alias::{
	ACCURACY as accuracy, EPOCH as epoch, HIP as Hip, LOSS as loss, LR as lr, R_TWO as r2,
	TIME as time,
};
pub use alias::ACCURACY as Acc;
pub use alias::{
	BCE as bce, CE as ce, ELU as elu, FOCAL as focal, GELU as gelu, HUBER as huber, LEAK as leak,
	LINEAR as linear, MAE as mae, MSE as mse, PRELU as prelu, RELU as relu, SELU as selu,
	SIG as sig, SILU as silu, SWISH as swish, TANH as tanh,
};
