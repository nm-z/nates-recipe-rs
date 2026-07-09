#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]
//! GPU-native neural network training.
//!
//! Three import styles, one crate, the same methods. Only the door differs —
//! everything after the constructor is dot chaining.
//!
//! ```rust,no_run
//! // style 1: static (dot syntax)
//! use recipe::*;
//!
//! let data  = recipe.data("train.csv").split(0.8).target("Price");
//! let model = recipe.model().layer(64).leak().layer(1).loss(mse).lr(0.001);
//! recipe.train().epochs(100).log([Loss, R2]).run(data, model).save("model.ogdl");
//! ```
//!
//! ```rust,no_run
//! // style 2: struct (associated function)
//! use recipe::{Data, Model, Train, mse};
//!
//! let data  = Data::load("train.csv").split(0.8).target("Price");
//! let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.001);
//! Train::new().epochs(100).run(data, model);
//! model.eval(data);
//! ```
//!
//! ```rust,no_run
//! // style 3: crate path (free function)
//! let data  = recipe::data("train.csv").split(0.8).target("Price");
//! let model = recipe::model().layer(64).leak().layer(1).loss(recipe::mse);
//! recipe::train().epochs(100).run(data, model);
//! recipe::eval(model, data);
//! ```

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
#[path = "utils/wire.rs"]
pub mod wire;

#[doc(hidden)]
#[path = "utils/train.rs"]
mod train;

#[doc(hidden)]
#[path = "utils/model.rs"]
pub mod model;

#[doc(inline)]
pub use dataset::Data;
#[doc(hidden)]
pub use dataset::Dataset;

#[doc(inline)]
pub use model::{
	Accuracy, Epoch, Loss, Lr, Metric, Model, R2, Time, Train, hip,
	attn, bce, ce, embed, focal, huber, mae, mse,
};
#[doc(hidden)]
pub use model::{
	Activation, IntoLayer, LayerSpec, Prepared, RunData, SavePath,
	elu, gelu, leak, linear, prelu, relu, selu, sig, silu, swish, tanh,
};

// ── the three doors ─────────────────────────────────────────────────────────
// `Data::load` / `Model::new` / `Train::new` are style 2 (associated function).
// Style 1 goes through the `recipe` static, style 3 through the free functions
// below. All three land on the same constructors, so the chain that follows is
// identical whichever door you came in by.

/// The unit type behind the [`recipe`] static — style 1's `recipe.data(…)`.
pub struct Recipe;

/// Style 1's entry point: `use recipe::*;` then `recipe.data("train.csv")`.
/// Lives in the value namespace, so `recipe::data(…)` (style 3) still resolves
/// through the crate.
#[allow(non_upper_case_globals)]
pub static recipe: Recipe = Recipe;

impl Recipe {
	pub fn data(&self, path: &str) -> &'static mut Data {
		Data::load(path)
	}
	pub fn model(&self) -> &'static mut Model {
		Model::new()
	}
	pub fn train(&self) -> &'static mut Train {
		Train::new()
	}
	pub fn eval(&self, model: &Model, data: &dyn RunData) -> Vec<f64> {
		model.eval(data)
	}
}

/// Style 3: `recipe::data("train.csv")`.
pub fn data(path: &str) -> &'static mut Data {
	Data::load(path)
}
/// Style 3: `recipe::model()`.
pub fn model() -> &'static mut Model {
	Model::new()
}
/// Style 3: `recipe::train()`.
pub fn train() -> &'static mut Train {
	Train::new()
}
/// Style 3: `recipe::eval(model, data)`.
pub fn eval(model: &Model, data: &dyn RunData) -> Vec<f64> {
	model.eval(data)
}
