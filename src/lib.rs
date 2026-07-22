#![allow(unsafe_code)]
use std::fmt;

#[doc(hidden)]
pub type Mat = ndarray::Array2<f64>;
#[doc(hidden)]
pub type Vec1 = ndarray::Array1<f64>;

#[doc(hidden)]
pub use gpu_core as gpu;

#[doc(hidden)]
pub use pantry::data;

pub mod api;

pub mod cli;

#[doc(inline)]
pub use api::data::Data;
#[doc(hidden)]
pub use api::data::{Dataset, Datasets};

#[doc(inline)]
pub use api::infer::Infer;
#[doc(inline)]
pub use api::model::{
	Acc, Accuracy, Epoch, Lr, Model, R2, Time, attn, bce, ce, embed, focal, hip, huber, mae,
	mse,
};
#[doc(inline)]
pub use api::train::Train;
#[doc(inline)]
pub use recipe_infer::loss as Loss;
#[doc(hidden)]
pub use api::{
	DataHandle, IntoLayer, ModelArg, ModelHandle, Prepared, RunArg, RunData, SavePath,
};
#[doc(hidden)]
pub use api::model::IntoObjective;
#[doc(hidden)]
pub use api::model::{
	elu, gelu, leak, linear, prelu, relu, selu, sig, silu, swish, tanh,
};
pub use ogdl::ogdl;
pub use recipe_ir::{Activation, LayerKind, LayerSpec, LogItem, Loss, Metric, Param};

pub fn block(content: impl fmt::Display) {
	ogdl::log::Write::always(content);
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
