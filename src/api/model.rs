use crate::api::IntoLayer;
use ogdl::log::Write;
use recipe_runtime::execute::ModelInner;
use std::cell::RefCell;
use std::mem;

pub use recipe_infer::{
	Acc, Accuracy, Activation, Epoch, LayerSpec, Loss, Lr, Metric, R2, Time, bce, ce, elu,
	focal, gelu, hip, huber, leak, linear, mae, mse, prelu, relu, selu, sig, silu, swish, tanh,
};

pub use recipe_infer::Param;

impl IntoLayer for usize {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense(self, Activation::Linear)
	}
}

pub struct DenseSpec {
	pub units: usize,
	pub act: Activation,
}
impl IntoLayer for DenseSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense(self.units, self.act)
	}
}

pub struct EmbedSpec {
	dim: usize,
	vocab: Option<usize>,
}
pub fn embed(dim: usize) -> EmbedSpec {
	EmbedSpec { dim, vocab: None }
}
impl EmbedSpec {
	pub fn vocab(mut self, v: usize) -> EmbedSpec {
		self.vocab = Some(v);
		self
	}
}
impl IntoLayer for EmbedSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Embed(self.dim, self.vocab)
	}
}

pub struct AttnSpec {
	heads: usize,
}
pub fn attn(heads: usize) -> AttnSpec {
	AttnSpec { heads }
}
impl IntoLayer for AttnSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Attn(self.heads)
	}
}

pub struct Model {
	pub(crate) inner: Box<ModelInner>,
}

thread_local! {
	static PARKED_MODEL: RefCell<Option<Box<ModelInner>>> =
		const { RefCell::new(None) };
}

impl Drop for Model {
	fn drop(&mut self) {
		let inner = mem::replace(&mut self.inner, Box::new(ModelInner::blank()));
		PARKED_MODEL.with(|slot| slot.borrow_mut().replace(inner));
	}
}

pub(crate) fn parked_model() -> Model {
	let inner = PARKED_MODEL.with(|slot| slot.borrow_mut().take());
	let inner = inner.unwrap_or_else(|| {
		Write::error(
			"run: no model configured — chain recipe.model().layer(…) before run(…, model)",
		);
		Box::new(ModelInner::blank())
	});
	Model { inner }
}

impl Model {
	pub fn new() -> Model {
		Model {
			inner: Box::new(ModelInner::blank()),
		}
	}

	pub fn load(weights: &str, proto: Model, d: usize) -> Model {
		if weights.ends_with(".gguf") {
			let mut proto = proto;
			proto.inner.gguf = Some(weights.to_string());
			return proto;
		}
		recipe_runtime::execute::load_weights(&proto.inner, weights, d);
		proto
	}

	pub fn layer(mut self, spec: impl IntoLayer) -> Model {
		self.inner.specs.push(spec.into_layer());
		self
	}

	fn last_activation_slot(&mut self) -> Option<&mut Activation> {
		match self.inner.specs.last_mut() {
			Some(LayerSpec::Dense(_units, a)) => Some(a),
			Some(LayerSpec::Conv(_filters, _kernel, _stride, a)) => Some(a),
			_other => None,
		}
	}

	fn set_last_activation(mut self, act: Activation) -> Model {
		match self.last_activation_slot() {
			Some(slot) => *slot = act,
			None => Write::error(
				"activation method called but last layer is not dense or conv",
			),
		}
		self
	}

	pub fn relu(self) -> Model {
		self.set_last_activation(Activation::Relu)
	}
	pub fn leak(self) -> Model {
		self.set_last_activation(Activation::LeakyRelu)
	}
	pub fn sigmoid(self) -> Model {
		self.set_last_activation(Activation::Sigmoid)
	}
	pub fn tanh(self) -> Model {
		self.set_last_activation(Activation::Tanh)
	}
	pub fn selu(self) -> Model {
		self.set_last_activation(Activation::Selu)
	}
	pub fn gelu(self) -> Model {
		self.set_last_activation(Activation::Gelu)
	}
	pub fn silu(self) -> Model {
		self.set_last_activation(Activation::Silu)
	}
	pub fn elu(self) -> Model {
		self.set_last_activation(Activation::Elu)
	}
	pub fn prelu(self) -> Model {
		self.set_last_activation(Activation::PRelu)
	}

	pub fn conv(mut self, filters: usize, kernel: usize, stride: usize) -> Model {
		self.inner
			.specs
			.push(LayerSpec::Conv(filters, kernel, stride, Activation::Linear));
		self
	}

	pub fn loss(self, obj: impl IntoObjective) -> Model {
		return obj.record(self);
	}

	pub fn lr(mut self, rate: f64) -> Model {
		self.inner.lr.set(rate);
		self.inner.lr_intent = Some(rate);
		return self;
	}
}

pub trait IntoObjective {
	fn record(self, m: Model) -> Model;
}

impl IntoObjective for recipe_ir::Loss {
	fn record(self, mut m: Model) -> Model {
		m.inner.objective = recipe_ir::ObjectiveIntent::Builtin(self);
		m.inner.loss.set(self);
		return m;
	}
}

impl IntoObjective for &Model {
	fn record(self, mut m: Model) -> Model {
		m.inner.objective =
			recipe_ir::ObjectiveIntent::Reference(recipe_ir::ObjectRef::Object(self.inner.id));
		return m;
	}
}

impl Default for Model {
	fn default() -> Self {
		Self::new()
	}
}
