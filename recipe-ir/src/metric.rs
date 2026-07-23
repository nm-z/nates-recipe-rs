#[derive(Clone, Copy, PartialEq)]
pub enum Metric {
	Loss,
	Accuracy,
	Epoch,
	Lr,
	Time,
	R2,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LogItem {
	Metric(Metric),
	Device,
}

mod alias {
	use super::{LogItem, Metric};
	use crate::model::{Activation, Loss};

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

pub use alias::ACCURACY as Acc;
pub use alias::{ACCURACY as Accuracy, EPOCH as Epoch, HIP as hip, LOSS as Loss, LR as Lr, R_TWO as R2, TIME as Time};
pub use alias::{ACCURACY as accuracy, EPOCH as epoch, HIP as Hip, LOSS as loss, LR as lr, R_TWO as r2, TIME as time};
pub use alias::{
	BCE as bce, CE as ce, ELU as elu, FOCAL as focal, GELU as gelu, HUBER as huber, LEAK as leak, LINEAR as linear,
	MAE as mae, MSE as mse, PRELU as prelu, RELU as relu, SELU as selu, SIG as sig, SILU as silu, SWISH as swish,
	TANH as tanh,
};
