#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]

pub type Mat = ndarray::Array2<f64>;
pub type Vec1 = ndarray::Array1<f64>;

pub use gpu_core as gpu;
pub mod lua_runtime;

#[path = "utils/data.rs"]
pub mod data;

#[cfg(test)]
#[path = "utils/tests.rs"]
mod tests;
