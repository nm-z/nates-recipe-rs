#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]
//! GPU-native neural network training.
//!
//! ```rust,no_run
//! use recipe::*;
//!
//! let data = Data::load()
//!     .set("train.csv")
//!     .split(0.8)
//!     .target("Price");
//!
//! let model = Model::new()
//!     .layer(64).leak()
//!     .layer(32).leak()
//!     .layer(1)
//!     .loss(mse)
//!     .lr(0.001);
//!
//! let train = Train::new()
//!     .epochs(100)
//!     .log([Loss, R2]);
//!
//! train.run(&model, &data);
//! train.save();
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
	Accuracy, Epoch, Loss, Lr, Metric, Model, R2, Time, Train,
	attn, bce, ce, embed, focal, huber, mae, mse,
};
#[doc(hidden)]
pub use model::{
	Activation, IntoLayer, LayerSpec, Param, RunData, SaveItem,
	elu, gelu, leak, linear, prelu, relu, selu, sig, silu, swish, tanh,
};

/// Save weights: `train.save()` (→ model.ogdl) or `train.save_as([w, b], path)`.
#[allow(non_upper_case_globals)]
pub const w: Param = Param::W;
/// Save biases via `train.save()` / `train.save_as([w, b], path)`.
#[allow(non_upper_case_globals)]
pub const b: Param = Param::B;
