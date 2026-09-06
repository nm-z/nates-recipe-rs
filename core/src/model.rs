#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activation {
	#[default]
	Linear,
	Cosine,
	Exponential,
	Logarithm,
	NaturalLogarithm,
	LegacySignedLogOnePlus,
	Huber,
	Tangent,
	Relu,
	LeakyRelu,
	Sigmoid,
	Tanh,
	Selu,
	Gelu,
	Silu,
	Elu,
	PRelu,
}

impl Activation {
	#[must_use]
	pub const fn checkpoint_tag(self) -> &'static str {
		match self {
			Self::Linear => "linear",
			Self::Cosine => "cosine",
			Self::Exponential => "exponential",
			Self::Logarithm => "signed-logarithm",
			Self::NaturalLogarithm => "natural-logarithm",
			Self::LegacySignedLogOnePlus => "logarithm",
			Self::Huber => "huber",
			Self::Tangent => "tangent",
			Self::Relu => "relu",
			Self::LeakyRelu => "leaky-relu",
			Self::Sigmoid => "sigmoid",
			Self::Tanh => "tanh",
			Self::Selu => "selu",
			Self::Gelu => "gelu",
			Self::Silu => "silu",
			Self::Elu => "elu",
			Self::PRelu => "prelu",
		}
	}

	#[must_use]
	pub const fn is_identity(self) -> bool { matches!(self, Self::Linear) }

	#[must_use]
	pub const fn learned_parameters(self) -> usize { if matches!(self, Self::PRelu) { 1 } else { 0 } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normalization {
	Layer,
	Batch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Function {
	Activation(Activation),
	Normalization(Normalization),
}

impl Function {
	#[must_use]
	pub const fn token(self) -> &'static str {
		match self {
			Self::Activation(activation) => activation.checkpoint_tag(),
			Self::Normalization(Normalization::Layer) => "layer-normalization",
			Self::Normalization(Normalization::Batch) => "batch-normalization",
		}
	}

	#[must_use]
	pub fn from_token(value: &str) -> Option<Self> {
		Some(match value {
			"linear" => Self::Activation(Activation::Linear),
			"cosine" => Self::Activation(Activation::Cosine),
			"exponential" => Self::Activation(Activation::Exponential),
			"signed-logarithm" => Self::Activation(Activation::Logarithm),
			"natural-logarithm" => Self::Activation(Activation::NaturalLogarithm),
			"logarithm" => Self::Activation(Activation::LegacySignedLogOnePlus),
			"huber" => Self::Activation(Activation::Huber),
			"tangent" => Self::Activation(Activation::Tangent),
			"relu" => Self::Activation(Activation::Relu),
			"leaky-relu" => Self::Activation(Activation::LeakyRelu),
			"sigmoid" => Self::Activation(Activation::Sigmoid),
			"tanh" => Self::Activation(Activation::Tanh),
			"selu" => Self::Activation(Activation::Selu),
			"gelu" => Self::Activation(Activation::Gelu),
			"silu" => Self::Activation(Activation::Silu),
			"elu" => Self::Activation(Activation::Elu),
			"prelu" => Self::Activation(Activation::PRelu),
			"layer-normalization" => Self::Normalization(Normalization::Layer),
			"batch-normalization" => Self::Normalization(Normalization::Batch),
			_ => return None,
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loss {
	MeanSquaredError,
	MeanAbsoluteError,
	Huber,
	BinaryCrossEntropy,
	CrossEntropy,
	Focal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Optimizer {
	AdamW,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LearningRateSchedule {
	Constant,
	Linear,
	Cosine,
	Exponential,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeFamily {
	LightGbm,
	CatBoost,
	XGBoost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataNormalization {
	Identity,
	ZScore,
	MinMax,
	L2Norm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
	Layer {
		width: usize,
		functions: Vec<Function>,
	},
	Convolution {
		filters: usize,
		kernel: usize,
		functions: Vec<Function>,
	},
	Pool {
		size: usize,
		group_to_neuron: Option<usize>,
	},
	KMeans {
		clusters: usize,
		group_to_neuron: Option<usize>,
	},
	Embedding {
		dimensions: usize,
		vocabulary: Option<usize>,
	},
	Attention {
		heads: usize,
	},
	Rnn {
		width: usize,
		functions: Vec<Function>,
	},
	Gru {
		width: usize,
		functions: Vec<Function>,
	},
	Lstm {
		width: usize,
		functions: Vec<Function>,
	},
	Tree {
		family: TreeFamily,
		trees: usize,
		depth: usize,
	},
	Knn {
		neighbors: usize,
		functions: Vec<Function>,
	},
	Residual {
		branch: Vec<Block>,
		functions: Vec<Function>,
	},
}

impl Block {
	#[must_use]
	pub const fn layer(width: usize) -> Self {
		Self::Layer {
			width,
			functions: Vec::new(),
		}
	}

	#[must_use]
	pub const fn convolution(filters: usize, kernel: usize) -> Self {
		Self::Convolution {
			filters,
			kernel,
			functions: Vec::new(),
		}
	}

	#[must_use]
	pub fn function(mut self, function: Function) -> Self {
		match &mut self {
			Self::Layer { functions, .. } | Self::Convolution { functions, .. } | Self::Rnn { functions, .. } | Self::Gru { functions, .. } | Self::Lstm { functions, .. } | Self::Knn { functions, .. } | Self::Residual { functions, .. } => functions.push(function),
			Self::Pool { .. } | Self::KMeans { .. } | Self::Embedding { .. } | Self::Attention { .. } | Self::Tree { .. } => {}
		}
		self
	}

	#[must_use]
	pub fn relu(self) -> Self { self.function(Function::Activation(Activation::Relu)) }
}
