#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]

#[doc(hidden)]
pub type Mat = ndarray::Array2<f64>;
#[doc(hidden)]
pub type Vec1 = ndarray::Array1<f64>;

#[doc(hidden)]
pub use gpu_core as gpu;

#[doc(hidden)]
pub use pantry::data;

#[doc(hidden)]
#[path = "utils/dataset.rs"]
pub mod dataset;
#[path = "utils/ooc.rs"]
pub mod ooc;
#[path = "utils/probe.rs"]
pub mod probe;
#[path = "utils/tui.rs"]
pub mod tui;
#[path = "utils/wire.rs"]
pub mod wire;

#[doc(hidden)]
#[path = "utils/train.rs"]
pub mod train;

#[doc(hidden)]
#[path = "utils/model.rs"]
pub mod model;

#[doc(inline)]
pub use dataset::Data;
#[doc(hidden)]
pub use dataset::Dataset;

#[doc(inline)]
pub use model::{
	Accuracy, Epoch, Infer, Loss, Lr, Metric, Model, R2, Time, Train, attn, bce, ce, embed, focal,
	hip, huber, mae, mse,
};
#[doc(hidden)]
pub use model::{
	Activation, DataHandle, IntoLayer, LayerSpec, ModelArg, ModelHandle, Prepared, RunArg,
	RunData, SavePath, elu, gelu, leak, linear, prelu, relu, selu, sig, silu, swish, tanh,
};
#[doc(hidden)]
pub use gpu_core::log::Flag;
#[doc(hidden)]
pub use gpu_core::log::{acc, chat, epoch, loss, lr, r2, time};
pub use gpu_core::log::Write;
pub use ogdl::ogdl;

pub struct Recipe;

mod alias {
	pub static RECIPE: super::Recipe = super::Recipe;
}
pub use alias::RECIPE as recipe;

impl Recipe {
	pub fn data(&self, path: &str) -> Data {
		Data::load(path)
	}
	pub fn model(&self) -> Model {
		Model::new()
	}
	pub fn train(&self) -> Train {
		Train::new()
	}
	pub fn infer(&self) -> Infer {
		Infer::new()
	}
}

pub fn data(path: &str) -> Data {
	Data::load(path)
}
pub fn model() -> Model {
	Model::new()
}
pub fn train() -> Train {
	Train::new()
}
pub fn infer() -> Infer {
	Infer::new()
}

pub(crate) fn ok_or_die<T, E: std::fmt::Display>(r: Result<T, E>, ctx: &str) -> T {
	if !r.is_ok() {
		drop(gpu_core::log::Write::err(format!(
			"{ctx}: {}",
			r.as_ref()
				.err()
				.map(|e| format!("{e:#}"))
				.unwrap_or_default()
		)));
		std::process::abort();
	}
	let Ok(v) = r else { std::process::abort() };
	v
}

pub(crate) fn some_or_die<T>(o: Option<T>, msg: &str) -> T {
	if !o.is_some() {
		drop(gpu_core::log::Write::err(msg));
		std::process::abort();
	}
	let Some(v) = o else { std::process::abort() };
	v
}
