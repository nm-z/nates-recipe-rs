//! Execution-description enums: what math each layer/loss/metric computes. These
//! describe the computation, not the data — so they live in the inference crate
//! alongside the engine that interprets them.

/// Activation function for a dense layer: `.layer(64).relu()`.
#[derive(Clone, Copy, PartialEq)]
pub enum Activation {
	Relu,
	Sigmoid,
	Linear,
	// Preserve negative signal: LeakyRelu (×0.01 for x<0), PRelu (learnable slope),
	// Elu/Selu (exponential for x<0), Tanh (→[-1,1]), Silu/Swish (x·σ(x)), Gelu
	// (transformer default). Tanh/Leaky/PRelu backprop from the output; Elu/Selu/
	// Silu/Gelu backprop from the pre-activation (saved in forward).
	LeakyRelu,
	PRelu,
	Elu,
	Selu,
	Tanh,
	Silu,
	Gelu,
}

/// A layer in the stack: a dense layer (`units`, activation), or a learned token
/// `Embed`ding lookup (each input column is a token id → `dim`-vector).
#[derive(Clone, Copy)]
pub enum LayerSpec {
	Dense(usize, Activation),
	/// `(dim, fixed_vocab)`. `fixed_vocab = Some(v)` pins the token table to `v`
	/// rows verbatim (used by the char-level type detector, whose alphabet is the
	/// fixed 257-symbol byte set); `None` derives vocab from the data (`max id + 1`).
	Embed(usize, Option<usize>),
	Attn(usize),
	Conv(usize, usize, usize, Activation),
}

/// What a layer computes. `Dense`: z = act(X·W + b). `Embed`: each of the
/// `in_dim` input columns is a token id, looked up in the `w` table ([vocab×dim],
/// row-major) → output `in_dim×dim` wide (the flattened token-vector sequence).
#[derive(Clone, Copy, PartialEq)]
pub enum LayerKind {
	Dense,
	Embed,
	Attn,
	Conv,
}

/// Loss function: `.loss(mse)`, `.loss(ce)`, etc.
#[derive(Clone, Copy, PartialEq)]
pub enum Loss {
	Mse,
	Mae,
	/// Softmax cross-entropy (multi-class).
	Ce,
	/// Binary cross-entropy.
	Bce,
	Huber,
	/// Focal loss (binary, down-weights easy examples for class imbalance).
	Focal,
}

impl Loss {
	pub fn is_classification(self) -> bool {
		matches!(self, Loss::Ce | Loss::Bce | Loss::Focal)
	}
	pub fn score_key(self) -> &'static str {
		if self.is_classification() {
			"acc"
		} else {
			"r2"
		}
	}
}

/// What to log or plot each epoch: `.log([Loss, R2, Lr])`.
#[derive(Clone, Copy, PartialEq)]
pub enum Metric {
	Loss,
	Accuracy,
	Epoch,
	Lr,
	Time,
	R2,
	/// Not a per-epoch number: prints the run-scoped HIP call-count tree when
	/// the run finishes. `.log([Loss, R2, hip])`.
	Hip,
}

#[allow(non_upper_case_globals)]
pub const relu: Activation = Activation::Relu;
#[allow(non_upper_case_globals)]
pub const sig: Activation = Activation::Sigmoid;
#[allow(non_upper_case_globals)]
pub const linear: Activation = Activation::Linear;
#[allow(non_upper_case_globals)]
pub const leak: Activation = Activation::LeakyRelu;
#[allow(non_upper_case_globals)]
pub const prelu: Activation = Activation::PRelu;
#[allow(non_upper_case_globals)]
pub const elu: Activation = Activation::Elu;
#[allow(non_upper_case_globals)]
pub const selu: Activation = Activation::Selu;
#[allow(non_upper_case_globals)]
pub const tanh: Activation = Activation::Tanh;
#[allow(non_upper_case_globals)]
pub const silu: Activation = Activation::Silu;
#[allow(non_upper_case_globals)]
pub const swish: Activation = Activation::Silu;
#[allow(non_upper_case_globals)]
pub const gelu: Activation = Activation::Gelu;
#[allow(non_upper_case_globals)]
pub const mse: Loss = Loss::Mse;
#[allow(non_upper_case_globals)]
pub const mae: Loss = Loss::Mae;
#[allow(non_upper_case_globals)]
pub const ce: Loss = Loss::Ce;
#[allow(non_upper_case_globals)]
pub const bce: Loss = Loss::Bce;
#[allow(non_upper_case_globals)]
pub const huber: Loss = Loss::Huber;
#[allow(non_upper_case_globals)]
pub const focal: Loss = Loss::Focal;

#[allow(non_upper_case_globals)]
pub const Loss: Metric = Metric::Loss;
#[allow(non_upper_case_globals)]
pub const Accuracy: Metric = Metric::Accuracy;
#[allow(non_upper_case_globals)]
pub const Epoch: Metric = Metric::Epoch;
#[allow(non_upper_case_globals)]
pub const Lr: Metric = Metric::Lr;
#[allow(non_upper_case_globals)]
pub const Time: Metric = Metric::Time;
#[allow(non_upper_case_globals)]
pub const R2: Metric = Metric::R2;
#[allow(non_upper_case_globals)]
pub const hip: Metric = Metric::Hip;

/// Which parameters an OGDL save writes — `W` (weights) and/or `B` (biases). The
/// crate-root `w`/`b` consts in `recipe` map onto these; lives here beside
/// the checkpoint codec since it gates `dump_ogdl`'s per-layer emission.
#[derive(Clone, Copy, PartialEq)]
pub enum Param {
	W,
	B,
}
