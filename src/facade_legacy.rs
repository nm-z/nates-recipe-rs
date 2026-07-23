use std::fmt;

#[doc(hidden)]
pub type Mat = ndarray::Array2<f64>;
#[doc(hidden)]
pub type Vec1 = ndarray::Array1<f64>;

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
#[doc(hidden)]
pub use api::model::IntoObjective;
#[doc(inline)]
pub use api::model::{Acc, Accuracy, Epoch, Lr, Model, R2, Time, attn, bce, ce, embed, focal, hip, huber, mae, mse};
#[doc(hidden)]
pub use api::model::{elu, gelu, leak, linear, prelu, relu, selu, sig, silu, swish, tanh};
#[doc(inline)]
pub use api::train::Train;
#[doc(hidden)]
pub use api::{DataHandle, IntoLayer, ModelArg, ModelHandle, Prepared, RunArg, RunData, SavePath};
pub use ogdl::ogdl;
#[doc(inline)]
pub use recipe_ir::metric::loss as Loss;
pub use recipe_ir::{Activation, LayerKind, LayerSpec, LogItem, Loss, Metric, Param};

pub fn block(content: impl fmt::Display) {
	ogdl::log::Write::always(content);
}

pub struct Recipe;

#[expect(non_upper_case_globals, reason = "preserves the public Recipe builder value")]
pub static recipe: Recipe = Recipe;

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
