#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]

pub type Mat = ndarray::Array2<f64>;
pub type Vec1 = ndarray::Array1<f64>;

pub use gpu_core as gpu;
pub mod lua_runtime;

#[path = "utils/data.rs"]
pub mod data;

#[path = "utils/dataset.rs"]
pub mod dataset;

#[path = "utils/model.rs"]
pub mod model;

pub use dataset::{Data, Dataset};
pub use model::{
      Accuracy, Activation, Epoch, IntoLayer, Loss, Lr, Metric, Model, Param, R2, Time, Train, ce,
      huber, linear, mae, mse, relu, sigmoid,
};

// `save` selectors. Defined here in the crate root — NOT in `model`, where bare
// `w`/`b` consts would be parsed as constant-patterns and break `let w`/`let b`.
#[allow(non_upper_case_globals)]
pub const w: Param = Param::W;
#[allow(non_upper_case_globals)]
pub const b: Param = Param::B;

#[cfg(test)]
#[path = "utils/tests.rs"]
mod tests;
