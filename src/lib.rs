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
      Accuracy, Epoch, Loss, Lr, Metric, Model, R2, Time, Train, hip,
      attn, bce, ce, embed, focal, huber, mae, mse,
};
#[doc(hidden)]
pub use model::{
      Activation, IntoLayer, LayerSpec, Prepared, RunData, SavePath,
      elu, gelu, leak, linear, prelu, relu, selu, sig, silu, swish, tanh,
};

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
      pub fn eval(&self, model: &Model, data: &dyn RunData) -> Vec<f64> {
            model.eval(data)
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
pub fn eval(model: &Model, data: &dyn RunData) -> Vec<f64> {
      model.eval(data)
}

pub(crate) fn ok_or_die<T, E: std::fmt::Display>(r: Result<T, E>, ctx: &str) -> T {
      assert!(
            r.is_ok(),
            "{ctx}: {}",
            r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default()
      );
      let Ok(v) = r else { std::process::abort() };
      v
}

pub(crate) fn some_or_die<T>(o: Option<T>, msg: &str) -> T {
      assert!(o.is_some(), "{msg}");
      let Some(v) = o else { std::process::abort() };
      v
}
