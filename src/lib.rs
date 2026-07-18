#![allow(unsafe_code, reason = "FFI to HIP runtime and libc")]
#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]

use std::fmt;

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
#[path = "utils/probe.rs"]
pub mod machine;
#[path = "utils/ooc.rs"]
pub mod ooc;
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
	Accuracy, Epoch, Infer, Loss, Lr, Metric, Model, R2, Time, Train, attn, bce, ce, embed,
	focal, hip, huber, mae, mse,
};
#[doc(hidden)]
pub use model::{
	Activation, DataHandle, IntoLayer, LayerSpec, ModelArg, ModelHandle, Prepared, RunArg,
	RunData, SavePath, elu, gelu, leak, linear, prelu, relu, selu, sig, silu, swish, tanh,
};
pub use ogdl::ogdl;

pub fn block(content: impl fmt::Display) {
	gpu_core::log::Write::always(content);
}

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

pub(crate) fn ok_or_err<T, E: fmt::Display>(r: Result<T, E>, ctx: &str) -> anyhow::Result<T> {
	r.map_err(|e| anyhow::anyhow!("{ctx}: {e:#}"))
}

pub(crate) fn some_or_err<T>(o: Option<T>, msg: &str) -> anyhow::Result<T> {
	o.ok_or_else(|| anyhow::anyhow!("{msg}"))
}
