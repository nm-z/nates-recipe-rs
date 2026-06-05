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

pub use dataset::{Data, DataSplit, Dataset};
pub use model::{
      Accuracy, Activation, Epoch, IntoLayer, Loss, Lr, Metric, Model, R2, Time, ce, huber, linear,
      mae, mse, relu, sigmoid,
};

#[cfg(test)]
#[path = "utils/tests.rs"]
mod tests;
