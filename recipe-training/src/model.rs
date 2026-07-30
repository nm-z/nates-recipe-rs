use core::fmt;
use core::num::{NonZeroU32, NonZeroU64};

use recipe_core::{DType, IterationDomain, KernelTemplateId, LoopIterations, MetricId, ValueId};
use recipe_ingest::{
	DenseMatrix, PartitionKind, PreparedDataset, PreparedValues, SemanticType, VectorEncoding, VectorMetadata,
	VectorRole, VectorSchema,
};
use recipe_language::{CalculationGraph, Shape};
use recipe_program::StaticCalculationProgram;

use crate::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};

/// Semantic upper bound admitted by the Recipe reduction grammar. Physical
/// lowering selects the realizable tree width from this bound and the exact
/// tensor shape plus measured device limits.
pub const MAXIMUM_REDUCTION_TREE_LANES: u32 = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseActivation {
	Linear,
	Cosine,
	Exponential,
	/// `sign(x) * ln(abs(x))` for finite, nonzero `x`.
	Logarithm,
	/// Ordinary `ln(x)` for finite, strictly positive `x`.
	NaturalLogarithm,
	/// Decoder-only compatibility for the pre-contract
	/// `sign(x) * ln(1 + abs(x))` activation.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseNormalization {
	Layer,
	Batch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseOperation {
	Activation(DenseActivation),
	Normalization(DenseNormalization),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseBlockKind {
	Layer,
	Perc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseDataNormalization {
	/// Preserve the prepared calculation dtype and values exactly. This is the
	/// only valid input policy for integer token IDs consumed by an embedding.
	Identity,
	ZScore,
	MinMax,
	L2Norm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseLoss {
	BinaryCrossEntropy,
	Focal,
	MeanSquaredError,
	MeanAbsoluteError,
	CrossEntropy,
	Huber,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LearningRateDecay {
	Constant,
	Linear,
	Cosine,
	Exponential,
}

/// Declared duration of one training phase.
///
/// An unbounded phase has no synthetic terminal epoch. It runs until the
/// executor accepts an explicit graceful-stop request after a complete epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingHorizon {
	Finite(NonZeroU64),
	Unbounded,
}

impl TrainingHorizon {
	#[must_use]
	pub const fn finite(epochs: NonZeroU64) -> Self {
		Self::Finite(epochs)
	}

	#[must_use]
	pub const fn unbounded() -> Self {
		Self::Unbounded
	}

	#[must_use]
	pub const fn bound(self) -> Option<NonZeroU64> {
		match self {
			Self::Finite(epochs) => Some(epochs),
			Self::Unbounded => None,
		}
	}

	#[must_use]
	pub const fn is_unbounded(self) -> bool {
		matches!(self, Self::Unbounded)
	}

	#[must_use]
	pub const fn loop_iterations(self) -> LoopIterations {
		match self {
			Self::Finite(epochs) => LoopIterations::Finite(epochs),
			Self::Unbounded => LoopIterations::Unbounded,
		}
	}
}

impl From<NonZeroU64> for TrainingHorizon {
	fn from(epochs: NonZeroU64) -> Self {
		Self::Finite(epochs)
	}
}

impl fmt::Display for TrainingHorizon {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Finite(epochs) => epochs.fmt(formatter),
			Self::Unbounded => formatter.write_str("unbounded"),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseLayer {
	kind: DenseBlockKind,
	width: NonZeroU64,
	operations: Vec<DenseOperation>,
}

impl DenseLayer {
	#[must_use]
	pub fn new(width: NonZeroU64, activation: DenseActivation) -> Self {
		Self {
			kind: DenseBlockKind::Layer,
			width,
			operations: match activation {
				DenseActivation::Linear => Vec::new(),
				activation => vec![DenseOperation::Activation(activation)],
			},
		}
	}

	#[must_use]
	pub fn with_operations(width: NonZeroU64, operations: impl IntoIterator<Item = DenseOperation>) -> Self {
		Self::with_kind(DenseBlockKind::Layer, width, operations)
	}

	#[must_use]
	pub fn with_kind(
		kind: DenseBlockKind,
		width: NonZeroU64,
		operations: impl IntoIterator<Item = DenseOperation>,
	) -> Self {
		Self {
			kind,
			width,
			operations: operations.into_iter().collect(),
		}
	}

	#[must_use]
	pub const fn kind(&self) -> DenseBlockKind {
		self.kind
	}

	#[must_use]
	pub const fn width(&self) -> NonZeroU64 {
		self.width
	}

	#[must_use]
	pub fn operations(&self) -> &[DenseOperation] {
		&self.operations
	}
}

/// One valid, stride-one one-dimensional convolution block.
///
/// The logical input length and channel count are resolved from the preceding
/// block. `filters` becomes the output channel count and the kernel spans the
/// logical length while consuming every input channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseConvolution {
	filters: NonZeroU64,
	kernel: NonZeroU64,
	operations: Vec<DenseOperation>,
}

impl DenseConvolution {
	#[must_use]
	pub fn new(filters: NonZeroU64, kernel: NonZeroU64, activation: DenseActivation) -> Self {
		Self {
			filters,
			kernel,
			operations: match activation {
				DenseActivation::Linear => Vec::new(),
				activation => vec![DenseOperation::Activation(activation)],
			},
		}
	}

	#[must_use]
	pub fn with_operations(
		filters: NonZeroU64,
		kernel: NonZeroU64,
		operations: impl IntoIterator<Item = DenseOperation>,
	) -> Self {
		Self {
			filters,
			kernel,
			operations: operations.into_iter().collect(),
		}
	}

	#[must_use]
	pub const fn filters(&self) -> NonZeroU64 {
		self.filters
	}

	#[must_use]
	pub const fn kernel(&self) -> NonZeroU64 {
		self.kernel
	}

	#[must_use]
	pub fn operations(&self) -> &[DenseOperation] {
		&self.operations
	}
}

/// Resolved logical shape of one convolution block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseConvolutionGeometry {
	input_length: NonZeroU64,
	input_channels: NonZeroU64,
	output_length: NonZeroU64,
	filters: NonZeroU64,
	kernel: NonZeroU64,
}

impl DenseConvolutionGeometry {
	#[must_use]
	pub const fn new(
		input_length: NonZeroU64,
		input_channels: NonZeroU64,
		output_length: NonZeroU64,
		filters: NonZeroU64,
		kernel: NonZeroU64,
	) -> Self {
		Self {
			input_length,
			input_channels,
			output_length,
			filters,
			kernel,
		}
	}

	#[must_use]
	pub const fn input_length(self) -> NonZeroU64 {
		self.input_length
	}

	#[must_use]
	pub const fn input_channels(self) -> NonZeroU64 {
		self.input_channels
	}

	#[must_use]
	pub const fn output_length(self) -> NonZeroU64 {
		self.output_length
	}

	#[must_use]
	pub const fn filters(self) -> NonZeroU64 {
		self.filters
	}

	#[must_use]
	pub const fn kernel(self) -> NonZeroU64 {
		self.kernel
	}

	#[must_use]
	pub fn input_width(self) -> Option<NonZeroU64> {
		self.input_length
			.get()
			.checked_mul(self.input_channels.get())
			.and_then(NonZeroU64::new)
	}

	#[must_use]
	pub fn output_width(self) -> Option<NonZeroU64> {
		self.output_length
			.get()
			.checked_mul(self.filters.get())
			.and_then(NonZeroU64::new)
	}
}

/// One ordered residual branch and the operations applied after its merge.
///
/// The final branch layer determines the residual output width. The compiler
/// uses an identity skip at that width and otherwise inserts a learned,
/// weight-only linear projection from the block input to that width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseResidual {
	branch: Vec<DenseResidualOperation>,
	operations: Vec<DenseOperation>,
}

/// One channelwise, non-overlapping maximum-pooling block.
///
/// The compiler resolves the logical input shape when the block is lowered.
/// If a dense layer immediately follows the pool, `group_to_neuron` retains
/// that layer's width so the grouped connection rule cannot be lost while the
/// facade is translated into the training IR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DensePool {
	size: NonZeroU64,
	group_to_neuron: Option<NonZeroU64>,
}

impl DensePool {
	#[must_use]
	pub const fn new(size: NonZeroU64, group_to_neuron: Option<NonZeroU64>) -> Self {
		Self {
			size,
			group_to_neuron,
		}
	}

	#[must_use]
	pub const fn size(self) -> NonZeroU64 {
		self.size
	}

	#[must_use]
	pub const fn group_to_neuron(self) -> Option<NonZeroU64> {
		self.group_to_neuron
	}

	#[must_use]
	pub fn routing(self, groups: NonZeroU64) -> Option<DenseGroupToNeuronRouting> {
		self.group_to_neuron
			.map(|neurons| DenseGroupToNeuronRouting::resolve(groups, neurons))
	}
}

/// One deterministic K-means feature-reduction block.
///
/// The block emits one L2 distance per centroid. If a dense layer immediately
/// follows it, `group_to_neuron` retains that layer's width so Recipe's shared
/// divisible-or-fully-connected routing rule remains explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseKMeans {
	clusters: NonZeroU64,
	group_to_neuron: Option<NonZeroU64>,
}

/// One learned token embedding table.
///
/// The incoming feature columns are the fixed sequence positions of each row.
/// Every value is an exact int32 token ID in `0..vocabulary`; the learned table
/// maps each ID to `dimensions` binary32 channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseEmbedding {
	dimensions: NonZeroU64,
	vocabulary: NonZeroU64,
}

impl DenseEmbedding {
	#[must_use]
	pub const fn new(dimensions: NonZeroU64, vocabulary: NonZeroU64) -> Self {
		Self {
			dimensions,
			vocabulary,
		}
	}

	#[must_use]
	pub const fn dimensions(self) -> NonZeroU64 {
		self.dimensions
	}

	#[must_use]
	pub const fn vocabulary(self) -> NonZeroU64 {
		self.vocabulary
	}
}

/// One causal multi-head self-attention block over the immediately preceding
/// fixed-sequence embedding.
///
/// The embedding dimension is divided evenly across `heads`. Recipe learns
/// independent query, key, value, and output matrices and applies a causal
/// mask; no tokenizer, padding, or positional vector is implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseAttention {
	heads: NonZeroU64,
}

impl DenseAttention {
	#[must_use]
	pub const fn new(heads: NonZeroU64) -> Self {
		Self { heads }
	}

	#[must_use]
	pub const fn heads(self) -> NonZeroU64 {
		self.heads
	}
}

/// One vanilla recurrent block over the numeric feature-column sequence.
///
/// Each row starts from an all-zero hidden state. One scalar feature is
/// consumed per step in column order, `tanh` updates the hidden state, and the
/// block returns only the final hidden state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseRnn {
	width: NonZeroU64,
}

impl DenseRnn {
	#[must_use]
	pub const fn new(width: NonZeroU64) -> Self {
		Self { width }
	}

	#[must_use]
	pub const fn width(self) -> NonZeroU64 {
		self.width
	}
}

/// One reset-before gated recurrent-unit block over the numeric
/// feature-column sequence.
///
/// Each row starts from an all-zero hidden state. One scalar feature is
/// consumed per step in column order, and the block returns only the final
/// hidden state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseGru {
	width: NonZeroU64,
}

impl DenseGru {
	#[must_use]
	pub const fn new(width: NonZeroU64) -> Self {
		Self { width }
	}

	#[must_use]
	pub const fn width(self) -> NonZeroU64 {
		self.width
	}
}

/// One gated long short-term memory block over the numeric feature-column
/// sequence.
///
/// Each row starts from all-zero hidden and cell state. One scalar feature is
/// consumed per step in column order, and the block returns only the final
/// hidden state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseLstm {
	width: NonZeroU64,
}

impl DenseLstm {
	#[must_use]
	pub const fn new(width: NonZeroU64) -> Self {
		Self { width }
	}

	#[must_use]
	pub const fn width(self) -> NonZeroU64 {
		self.width
	}
}

impl DenseKMeans {
	#[must_use]
	pub const fn new(clusters: NonZeroU64, group_to_neuron: Option<NonZeroU64>) -> Self {
		Self {
			clusters,
			group_to_neuron,
		}
	}

	#[must_use]
	pub const fn clusters(self) -> NonZeroU64 {
		self.clusters
	}

	#[must_use]
	pub const fn group_to_neuron(self) -> Option<NonZeroU64> {
		self.group_to_neuron
	}

	#[must_use]
	pub fn routing(self) -> Option<DenseGroupToNeuronRouting> {
		self.group_to_neuron
			.map(|neurons| DenseGroupToNeuronRouting::resolve(self.clusters, neurons))
	}
}

/// Recipe-owned growth order for one supervised tree ensemble.
///
/// The public spellings remain `.lgbm`, `.cbst`, and `.xgbst`; this identity
/// is retained explicitly so checkpoint and inference code cannot silently
/// collapse the three construction rules into one generic tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseTreeFamily {
	LightGbm,
	CatBoost,
	XGBoost,
}

/// One terminal supervised tree ensemble.
///
/// A standalone public booster declares exactly one tree. A preceding
/// `.forest(trees)` declares exactly `trees` trees and does not add another
/// hidden estimator or boosting-round count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseTree {
	family: DenseTreeFamily,
	trees: NonZeroU64,
	depth: NonZeroU32,
}

impl DenseTree {
	#[must_use]
	pub const fn new(family: DenseTreeFamily, trees: NonZeroU64, depth: NonZeroU32) -> Self {
		Self {
			family,
			trees,
			depth,
		}
	}

	#[must_use]
	pub const fn family(self) -> DenseTreeFamily {
		self.family
	}

	#[must_use]
	pub const fn trees(self) -> NonZeroU64 {
		self.trees
	}

	#[must_use]
	pub const fn depth(self) -> NonZeroU32 {
		self.depth
	}
}

/// Recipe's shared connection rule from reduced groups to a following dense
/// layer. Divisible shapes use contiguous routing; a non-divisible shape uses
/// ordinary full connectivity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseGroupToNeuronRouting {
	Identity {
		width: NonZeroU64,
	},
	Expand {
		groups: NonZeroU64,
		neurons: NonZeroU64,
		neurons_per_group: NonZeroU64,
	},
	Contract {
		groups: NonZeroU64,
		neurons: NonZeroU64,
		groups_per_neuron: NonZeroU64,
	},
	FullyConnected {
		groups: NonZeroU64,
		neurons: NonZeroU64,
	},
}

impl DenseGroupToNeuronRouting {
	#[must_use]
	pub fn resolve(groups: NonZeroU64, neurons: NonZeroU64) -> Self {
		let groups_value = groups.get();
		let neurons_value = neurons.get();
		if groups == neurons {
			Self::Identity { width: groups }
		} else if neurons_value % groups_value == 0 {
			Self::Expand {
				groups,
				neurons,
				neurons_per_group: NonZeroU64::new(neurons_value / groups_value)
					.expect("nonzero divisible group expansion"),
			}
		} else if groups_value % neurons_value == 0 {
			Self::Contract {
				groups,
				neurons,
				groups_per_neuron: NonZeroU64::new(groups_value / neurons_value)
					.expect("nonzero divisible group contraction"),
			}
		} else {
			Self::FullyConnected { groups, neurons }
		}
	}
}

/// Stable flattening order for a pooled logical `[rows, groups, channels]`
/// tensor when it feeds a dense block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DensePoolGroupOrder {
	GroupMajorChannelMinor,
}

/// Stable winner rule used by both maximum-pool forward and backward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DensePoolWinnerContract {
	LowestLogicalIndex,
}

/// Resolved, per-example logical shape retained for checkpoint validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DensePoolState {
	input_length: NonZeroU64,
	channels: NonZeroU64,
	output_length: NonZeroU64,
	group_order: DensePoolGroupOrder,
	winner_contract: DensePoolWinnerContract,
}

impl DensePoolState {
	#[must_use]
	pub const fn new(input_length: NonZeroU64, channels: NonZeroU64, output_length: NonZeroU64) -> Self {
		Self {
			input_length,
			channels,
			output_length,
			group_order: DensePoolGroupOrder::GroupMajorChannelMinor,
			winner_contract: DensePoolWinnerContract::LowestLogicalIndex,
		}
	}

	#[must_use]
	pub const fn input_length(self) -> NonZeroU64 {
		self.input_length
	}

	#[must_use]
	pub const fn channels(self) -> NonZeroU64 {
		self.channels
	}

	#[must_use]
	pub const fn output_length(self) -> NonZeroU64 {
		self.output_length
	}

	#[must_use]
	pub const fn group_order(self) -> DensePoolGroupOrder {
		self.group_order
	}

	#[must_use]
	pub const fn winner_contract(self) -> DensePoolWinnerContract {
		self.winner_contract
	}

	#[must_use]
	pub fn input_width(self) -> Option<NonZeroU64> {
		self.input_length
			.get()
			.checked_mul(self.channels.get())
			.and_then(NonZeroU64::new)
	}

	#[must_use]
	pub fn output_width(self) -> Option<NonZeroU64> {
		self.output_length
			.get()
			.checked_mul(self.channels.get())
			.and_then(NonZeroU64::new)
	}

	#[must_use]
	pub const fn logical_input_shape(self, rows: NonZeroU64) -> [u64; 3] {
		[rows.get(), self.input_length.get(), self.channels.get()]
	}

	#[must_use]
	pub const fn logical_output_shape(self, rows: NonZeroU64) -> [u64; 3] {
		[rows.get(), self.output_length.get(), self.channels.get()]
	}
}

/// One exact, ordered operation inside a residual branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenseResidualOperation {
	Layer(DenseLayer),
	Operation(DenseOperation),
}

impl From<DenseLayer> for DenseResidualOperation {
	fn from(layer: DenseLayer) -> Self {
		Self::Layer(layer)
	}
}

impl From<DenseOperation> for DenseResidualOperation {
	fn from(operation: DenseOperation) -> Self {
		Self::Operation(operation)
	}
}

impl DenseResidual {
	#[must_use]
	pub fn new<T>(branch: impl IntoIterator<Item = T>, operations: impl IntoIterator<Item = DenseOperation>) -> Self
	where
		T: Into<DenseResidualOperation>,
	{
		Self {
			branch: branch.into_iter().map(Into::into).collect(),
			operations: operations.into_iter().collect(),
		}
	}

	#[must_use]
	pub fn branch(&self) -> &[DenseResidualOperation] {
		&self.branch
	}

	#[must_use]
	pub fn operations(&self) -> &[DenseOperation] {
		&self.operations
	}

	#[must_use]
	pub fn output_width(&self) -> Option<NonZeroU64> {
		self.branch
			.iter()
			.rev()
			.find_map(|operation| match operation {
				DenseResidualOperation::Layer(layer) => Some(layer.width()),
				DenseResidualOperation::Operation(_) => None,
			})
	}
}

/// A topology-preserving dense training block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenseBlock {
	Embedding(DenseEmbedding),
	Attention(DenseAttention),
	Rnn(DenseRnn),
	Gru(DenseGru),
	Lstm(DenseLstm),
	Layer(DenseLayer),
	Convolution(DenseConvolution),
	Pool(DensePool),
	KMeans(DenseKMeans),
	Tree(DenseTree),
	Residual(DenseResidual),
}

impl DenseBlock {
	#[must_use]
	pub fn output_width(&self) -> Option<NonZeroU64> {
		match self {
			Self::Embedding(_) => None,
			Self::Attention(_) => None,
			Self::Rnn(rnn) => Some(rnn.width()),
			Self::Gru(gru) => Some(gru.width()),
			Self::Lstm(lstm) => Some(lstm.width()),
			Self::Layer(layer) => Some(layer.width()),
			Self::Convolution(_) => None,
			Self::Pool(_) => None,
			Self::KMeans(kmeans) => Some(kmeans.clusters()),
			Self::Tree(_) => None,
			Self::Residual(residual) => residual.output_width(),
		}
	}

	#[must_use]
	pub fn output_operations(&self) -> &[DenseOperation] {
		match self {
			Self::Embedding(_) => &[],
			Self::Attention(_) => &[],
			Self::Rnn(_) => &[],
			Self::Gru(_) => &[],
			Self::Lstm(_) => &[],
			Self::Layer(layer) => layer.operations(),
			Self::Convolution(convolution) => convolution.operations(),
			Self::Pool(_) | Self::KMeans(_) | Self::Tree(_) => &[],
			Self::Residual(residual) => residual.operations(),
		}
	}
}

impl From<DenseEmbedding> for DenseBlock {
	fn from(embedding: DenseEmbedding) -> Self {
		Self::Embedding(embedding)
	}
}

impl From<DenseAttention> for DenseBlock {
	fn from(attention: DenseAttention) -> Self {
		Self::Attention(attention)
	}
}

impl From<DenseRnn> for DenseBlock {
	fn from(rnn: DenseRnn) -> Self {
		Self::Rnn(rnn)
	}
}

impl From<DenseGru> for DenseBlock {
	fn from(gru: DenseGru) -> Self {
		Self::Gru(gru)
	}
}

impl From<DenseLstm> for DenseBlock {
	fn from(lstm: DenseLstm) -> Self {
		Self::Lstm(lstm)
	}
}

impl From<DenseLayer> for DenseBlock {
	fn from(layer: DenseLayer) -> Self {
		Self::Layer(layer)
	}
}

impl From<DensePool> for DenseBlock {
	fn from(pool: DensePool) -> Self {
		Self::Pool(pool)
	}
}

impl From<DenseKMeans> for DenseBlock {
	fn from(kmeans: DenseKMeans) -> Self {
		Self::KMeans(kmeans)
	}
}

impl From<DenseConvolution> for DenseBlock {
	fn from(convolution: DenseConvolution) -> Self {
		Self::Convolution(convolution)
	}
}

impl From<DenseTree> for DenseBlock {
	fn from(tree: DenseTree) -> Self {
		Self::Tree(tree)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseTask {
	BinaryClassification {
		target_vector: usize,
		positive_code: i32,
	},
	MulticlassClassification {
		target_vector: usize,
		class_count: usize,
		reserved_code: i32,
	},
	ScalarRegression {
		target_vector: usize,
	},
	/// Multiple declared numeric 0/1 targets, retained as one ordered matrix.
	/// BCE or focal loss is evaluated independently at each matrix element.
	MultiTargetBinaryClassification {
		first_target_vector: usize,
		target_count: usize,
	},
	/// Multiple declared numeric one-hot targets forming one categorical row.
	/// Cross entropy couples the columns through one row-wise softmax.
	JointMulticlassClassification {
		first_target_vector: usize,
		target_count: usize,
	},
	/// Multiple declared numeric targets forming one ordered regression vector.
	MultiTargetRegression {
		first_target_vector: usize,
		target_count: usize,
	},
}

impl DenseTask {
	#[must_use]
	pub const fn target_vector(self) -> usize {
		match self {
			Self::BinaryClassification { target_vector, .. }
			| Self::MulticlassClassification { target_vector, .. }
			| Self::ScalarRegression { target_vector } => target_vector,
			Self::MultiTargetBinaryClassification {
				first_target_vector,
				..
			}
			| Self::JointMulticlassClassification {
				first_target_vector,
				..
			}
			| Self::MultiTargetRegression {
				first_target_vector,
				..
			} => first_target_vector,
		}
	}

	#[must_use]
	pub const fn target_count(self) -> usize {
		match self {
			Self::BinaryClassification { .. }
			| Self::MulticlassClassification { .. }
			| Self::ScalarRegression { .. } => 1,
			Self::MultiTargetBinaryClassification { target_count, .. }
			| Self::JointMulticlassClassification { target_count, .. }
			| Self::MultiTargetRegression { target_count, .. } => target_count,
		}
	}

	#[must_use]
	pub const fn uses_target_matrix(self) -> bool {
		matches!(
			self,
			Self::MultiTargetBinaryClassification { .. }
				| Self::JointMulticlassClassification { .. }
				| Self::MultiTargetRegression { .. }
		)
	}

	#[must_use]
	pub const fn output_width(self) -> usize {
		match self {
			Self::BinaryClassification { .. } | Self::ScalarRegression { .. } => 1,
			Self::MulticlassClassification { class_count, .. } => class_count,
			Self::MultiTargetBinaryClassification { target_count, .. }
			| Self::JointMulticlassClassification { target_count, .. }
			| Self::MultiTargetRegression { target_count, .. } => target_count,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseOutputAdapter {
	source_width: NonZeroU64,
	target_width: NonZeroU64,
}

impl DenseOutputAdapter {
	pub(crate) const fn new(source_width: NonZeroU64, target_width: NonZeroU64) -> Self {
		Self {
			source_width,
			target_width,
		}
	}

	#[must_use]
	pub const fn source_width(self) -> NonZeroU64 {
		self.source_width
	}

	#[must_use]
	pub const fn target_width(self) -> NonZeroU64 {
		self.target_width
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenseFeatureLowering {
	NumericScalar,
	CategoricalOneHot {
		dictionary_width: usize,
		reserved_index: usize,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledFeatureSpan {
	source_vector: usize,
	start: usize,
	width: usize,
	lowering: DenseFeatureLowering,
}

impl CompiledFeatureSpan {
	pub(crate) const fn new(
		source_vector: usize,
		start: usize,
		width: usize,
		lowering: DenseFeatureLowering,
	) -> Self {
		Self {
			source_vector,
			start,
			width,
			lowering,
		}
	}

	#[must_use]
	pub const fn source_vector(&self) -> usize {
		self.source_vector
	}

	#[must_use]
	pub const fn start(&self) -> usize {
		self.start
	}

	#[must_use]
	pub const fn width(&self) -> usize {
		self.width
	}

	#[must_use]
	pub const fn lowering(&self) -> DenseFeatureLowering {
		self.lowering
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledDatasetSchema {
	vectors: Vec<VectorSchema>,
	features: Vec<CompiledFeatureSpan>,
	targets: Vec<usize>,
	target_dtypes: Vec<DType>,
	input_width: usize,
	task: DenseTask,
}

impl CompiledDatasetSchema {
	pub(crate) fn from_prepared(
		dataset: &PreparedDataset,
		task: DenseTask,
		features: Vec<CompiledFeatureSpan>,
	) -> TrainingCompileResult<Self> {
		let vectors = dataset
			.vectors()
			.iter()
			.map(|vector| vector.schema())
			.collect::<Vec<_>>();
		let targets = dataset.target_source_indices().to_vec();
		if targets.len() != task.target_count() || targets.first().copied() != Some(task.target_vector()) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!(
					"declared target order {targets:?} disagrees with task primary target {} and count {}",
					task.target_vector(),
					task.target_count()
				),
			));
		}
		let target_dtypes = targets
			.iter()
			.copied()
			.map(|target| {
				let target_schema = vectors
					.iter()
					.find(|vector| vector.source_index() == target && vector.role() == VectorRole::Target)
					.ok_or_else(|| {
						TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidTargetMatrix,
							format!("dense target vector {target} is absent from the compiled schema"),
						)
					})?;
				target_schema.encoding().dtype().ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidTargetMatrix,
						format!(
							"dense target vector {target} uses variable-width {:?} storage without a typed target lowering",
							target_schema.encoding()
						),
					)
				})
			})
			.collect::<TrainingCompileResult<Vec<_>>>()?;
		let input_width = features.iter().map(|feature| feature.width).sum();
		Ok(Self {
			vectors,
			features,
			targets,
			target_dtypes,
			input_width,
			task,
		})
	}

	#[must_use]
	pub fn vectors(&self) -> &[VectorSchema] {
		&self.vectors
	}

	#[must_use]
	pub fn features(&self) -> &[CompiledFeatureSpan] {
		&self.features
	}

	#[must_use]
	pub fn target(&self) -> usize {
		self.targets[0]
	}

	/// Target source identities in the user's declaration order.
	#[must_use]
	pub fn targets(&self) -> &[usize] {
		&self.targets
	}

	/// Smallest fixed-width dtype selected by semantic distillation for the
	/// target vector. Loss calculations may consume an explicit conversion,
	/// but this source contract never changes.
	#[must_use]
	pub fn target_dtype(&self) -> DType {
		self.target_dtypes[0]
	}

	/// Calculation-facing source dtypes in declared-target order.
	#[must_use]
	pub fn target_dtypes(&self) -> &[DType] {
		&self.target_dtypes
	}

	#[must_use]
	pub const fn input_width(&self) -> usize {
		self.input_width
	}

	#[must_use]
	pub const fn task(&self) -> DenseTask {
		self.task
	}

	#[must_use]
	pub const fn output_width(&self) -> usize {
		self.task.output_width()
	}

	/// Decode one multiclass output index under the fitted target dictionary.
	///
	/// The final index is the explicit checkpoint-v5 route for a nonempty label
	/// absent from the fit partition. An out-of-range index, or a non-multiclass
	/// schema, has no categorical decoding.
	#[must_use]
	pub fn decode_multiclass_class(&self, class: usize) -> Option<DecodedMulticlassClass<'_>> {
		let DenseTask::MulticlassClassification {
			target_vector,
			class_count,
			reserved_code,
		} = self.task
		else {
			return None;
		};
		if class >= class_count {
			return None;
		}
		let target = self
			.vectors
			.iter()
			.find(|vector| vector.source_index() == target_vector && vector.role() == VectorRole::Target)?;
		let VectorMetadata::Categorical { dictionary } = target.metadata() else {
			return None;
		};
		if i32::try_from(class).ok() == Some(reserved_code) {
			return Some(DecodedMulticlassClass::ReservedUnseen);
		}
		dictionary
			.get(class)
			.map(|label| DecodedMulticlassClass::Label(label))
	}
}

/// Semantic decoding of one multiclass output index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodedMulticlassClass<'a> {
	Label(&'a [u8]),
	ReservedUnseen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DenseFeaturePlan {
	spans: Vec<CompiledFeatureSpan>,
	input_width: usize,
	normalization_mask: Option<Vec<u32>>,
}

impl DenseFeaturePlan {
	pub(crate) fn from_prepared(dataset: &PreparedDataset) -> TrainingCompileResult<Self> {
		let mut spans = Vec::new();
		let mut normalization_mask = Vec::new();
		let mut input_width = 0usize;
		let mut has_categorical = false;
		for feature in dataset
			.vectors()
			.iter()
			.filter(|vector| vector.role() == VectorRole::Feature)
		{
			let (width, lowering, normalize) = match (
				feature.semantic_type(),
				feature.encoding(),
				feature.metadata(),
				feature.values(),
			) {
				(SemanticType::Numeric, VectorEncoding::I32, VectorMetadata::None, PreparedValues::I32(_))
				| (
					SemanticType::Numeric,
					VectorEncoding::F32,
					VectorMetadata::None,
					PreparedValues::F32Bits(_),
				) => (1, DenseFeatureLowering::NumericScalar, true),
				(
					SemanticType::Categorical,
					VectorEncoding::DictionaryI32,
					VectorMetadata::Categorical { dictionary },
					PreparedValues::I32(_),
				) => {
					validate_categorical_dictionary(feature.name(), dictionary)?;
					has_categorical = true;
					let reserved_index = dictionary.len();
					(
						reserved_index.checked_add(1).ok_or_else(|| {
							TrainingCompileError::new(
								TrainingCompileErrorKind::ArithmeticOverflow,
								"categorical one-hot width overflowed usize",
							)
						})?,
						DenseFeatureLowering::CategoricalOneHot {
							dictionary_width: dictionary.len(),
							reserved_index,
						},
						false,
					)
				}
				_ => {
					return Err(TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidFeatureMatrix,
						format!(
							"feature {:?} classified {:?}/{:?} does not yet have a dedicated semantic dense lowering",
							String::from_utf8_lossy(feature.name()),
							feature.semantic_type(),
							feature.encoding()
						),
					));
				}
			};
			let start = input_width;
			input_width = input_width.checked_add(width).ok_or_else(|| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::ArithmeticOverflow,
					"lowered dense feature width overflowed usize",
				)
			})?;
			spans.push(CompiledFeatureSpan::new(
				feature.source_index(),
				start,
				width,
				lowering,
			));
			normalization_mask.resize(
				input_width,
				if normalize { 1.0f32 } else { 0.0f32 }.to_bits(),
			);
		}
		if spans.is_empty() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				"dense feature lowering requires at least one feature vector",
			));
		}
		Ok(Self {
			spans,
			input_width,
			normalization_mask: has_categorical.then_some(normalization_mask),
		})
	}

	#[must_use]
	pub(crate) fn spans(&self) -> &[CompiledFeatureSpan] {
		&self.spans
	}

	#[must_use]
	pub(crate) fn normalization_mask(&self) -> Option<&[u32]> {
		self.normalization_mask.as_deref()
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DensePartition {
	features: DenseMatrix,
	targets: DenseMatrix,
	target_observations: Vec<TargetObservation>,
}

impl DensePartition {
	fn new(
		features: DenseMatrix,
		targets: DenseMatrix,
		target_observations: Vec<TargetObservation>,
	) -> TrainingCompileResult<Self> {
		validate_partition(&features, &targets)?;
		if target_observations.len() != targets.rows() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InconsistentRows,
				format!(
					"target matrix has {} rows but target observation state has {} rows",
					targets.rows(),
					target_observations.len()
				),
			));
		}
		Ok(Self {
			features,
			targets,
			target_observations,
		})
	}

	#[must_use]
	pub const fn features(&self) -> &DenseMatrix {
		&self.features
	}

	#[must_use]
	pub const fn targets(&self) -> &DenseMatrix {
		&self.targets
	}

	#[must_use]
	pub const fn rows(&self) -> usize {
		self.features.rows()
	}

	#[must_use]
	pub const fn feature_columns(&self) -> usize {
		self.features.columns()
	}

	#[must_use]
	pub fn all_targets_known(&self) -> bool {
		self.target_observations
			.iter()
			.all(|observation| *observation == TargetObservation::Known)
	}

	pub(crate) fn accepted_updates_per_epoch(&self) -> usize {
		usize::from(self.target_observations.contains(&TargetObservation::Known))
	}

	pub(crate) fn target_supervision(&self) -> DenseMatrix {
		DenseMatrix::F32Bits {
			rows: self.rows(),
			columns: 1,
			values: self
				.target_observations
				.iter()
				.map(|observation| match observation {
					TargetObservation::Known => 1.0_f32.to_bits(),
					TargetObservation::Missing | TargetObservation::Unseen => 0.0_f32.to_bits(),
				})
				.collect(),
		}
	}

	fn known_only(self) -> TrainingCompileResult<Option<Self>> {
		if self.all_targets_known() {
			return Ok(Some(self));
		}
		let retained_rows = self
			.target_observations
			.iter()
			.enumerate()
			.filter_map(|(row, observation)| (*observation == TargetObservation::Known).then_some(row))
			.collect::<Vec<_>>();
		if retained_rows.is_empty() {
			return Ok(None);
		}
		let features = compact_dense_rows(&self.features, &retained_rows, "validation feature")?;
		let targets = compact_dense_rows(&self.targets, &retained_rows, "validation target")?;
		Self::new(
			features,
			targets,
			vec![TargetObservation::Known; retained_rows.len()],
		)
		.map(Some)
	}
}

/// Whether one prepared target value is supervised by the declared task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetObservation {
	Known,
	Missing,
	Unseen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoweredDenseTargets {
	matrix: DenseMatrix,
	observations: Vec<TargetObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoweredDenseDataset {
	train: DensePartition,
	validation: Option<DensePartition>,
	validation_split_rows: usize,
}

impl LoweredDenseDataset {
	fn new(
		train: DensePartition,
		validation: Option<DensePartition>,
		validation_split_rows: usize,
	) -> TrainingCompileResult<Self> {
		if let Some(validation) = &validation
			&& validation.feature_columns() != train.feature_columns()
		{
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!(
					"validation has {} feature columns, training has {}",
					validation.feature_columns(),
					train.feature_columns()
				),
			));
		}
		Ok(Self {
			train,
			validation,
			validation_split_rows,
		})
	}

	pub(crate) fn from_prepared(
		dataset: &PreparedDataset,
		feature_plan: &DenseFeaturePlan,
		task: DenseTask,
	) -> TrainingCompileResult<Self> {
		let train_targets = lower_dense_targets(dataset, task, PartitionKind::Train)?;
		let train = DensePartition::new(
			lower_dense_features(dataset, feature_plan, PartitionKind::Train)?,
			train_targets.matrix,
			train_targets.observations,
		)?;
		let validation_split_rows = dataset.validation().len();
		let validation = if dataset.validation().is_empty() {
			None
		} else {
			let validation_targets = lower_dense_targets(dataset, task, PartitionKind::Validation)?;
			DensePartition::new(
				lower_dense_features(dataset, feature_plan, PartitionKind::Validation)?,
				validation_targets.matrix,
				validation_targets.observations,
			)?
			.known_only()?
		};
		Self::new(train, validation, validation_split_rows)
	}

	#[must_use]
	pub const fn train(&self) -> &DensePartition {
		&self.train
	}

	#[must_use]
	pub const fn validation(&self) -> Option<&DensePartition> {
		self.validation.as_ref()
	}

	#[must_use]
	pub const fn validation_split_rows(&self) -> usize {
		self.validation_split_rows
	}
}

fn lower_dense_targets(
	dataset: &PreparedDataset,
	task: DenseTask,
	partition_kind: PartitionKind,
) -> TrainingCompileResult<LoweredDenseTargets> {
	if task.uses_target_matrix() {
		return lower_multi_dense_targets(dataset, task, partition_kind);
	}
	let target_vector = task.target_vector();
	let target = dataset
		.vectors()
		.iter()
		.find(|vector| vector.source_index() == target_vector && vector.role() == VectorRole::Target)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("dense target vector {target_vector} is absent"),
			)
		})?;
	let partition = match partition_kind {
		PartitionKind::Train => dataset.train(),
		PartitionKind::Validation => dataset.validation(),
	};
	match target.values() {
		PreparedValues::I32(values) => {
			let mut lowered = Vec::with_capacity(partition.len());
			let mut observations = Vec::with_capacity(partition.len());
			for (retained_position, source_row) in partition
				.retained_positions()
				.iter()
				.copied()
				.zip(partition.source_rows().iter().copied())
			{
				let value = values.get(retained_position).ok_or_else(|| {
					target_lowering_error(target.name(), source_row, "target value is absent")
				})?;
				let (value, observation) = lower_i32_target(target, task, *value, source_row)?;
				lowered.push(value);
				observations.push(observation);
			}
			Ok(LoweredDenseTargets {
				matrix: DenseMatrix::I32 {
					rows: partition.len(),
					columns: 1,
					values: lowered,
				},
				observations,
			})
		}
		PreparedValues::F32Bits(values) => {
			if matches!(task, DenseTask::MulticlassClassification { .. }) {
				return Err(target_lowering_error(
					target.name(),
					0,
					"dictionary target does not retain int32 category codes",
				));
			}
			let mut lowered = Vec::with_capacity(partition.len());
			let mut observations = Vec::with_capacity(partition.len());
			for (retained_position, source_row) in partition
				.retained_positions()
				.iter()
				.copied()
				.zip(partition.source_rows().iter().copied())
			{
				let value = values.get(retained_position).ok_or_else(|| {
					target_lowering_error(target.name(), source_row, "target value is absent")
				})?;
				let (bits, observation) = lower_f32_target(target.name(), task, *value, source_row)?;
				lowered.push(bits);
				observations.push(observation);
			}
			Ok(LoweredDenseTargets {
				matrix: DenseMatrix::F32Bits {
					rows: partition.len(),
					columns: 1,
					values: lowered,
				},
				observations,
			})
		}
		PreparedValues::VariableWidth(_) => Err(target_lowering_error(
			target.name(),
			0,
			"dense target cannot use variable-width storage",
		)),
	}
}

fn lower_multi_dense_targets(
	dataset: &PreparedDataset,
	task: DenseTask,
	partition_kind: PartitionKind,
) -> TrainingCompileResult<LoweredDenseTargets> {
	let target_count = task.target_count();
	if target_count < 2 || dataset.target_source_indices().len() != target_count {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!(
				"multi-target task requires at least two declared targets and exact order width {target_count}, found {:?}",
				dataset.target_source_indices()
			),
		));
	}
	let targets = dataset
		.target_source_indices()
		.iter()
		.copied()
		.map(|source_index| {
			dataset
				.vectors()
				.iter()
				.find(|vector| vector.source_index() == source_index && vector.role() == VectorRole::Target)
				.ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidTargetMatrix,
						format!("declared target vector {source_index} is absent"),
					)
				})
		})
		.collect::<TrainingCompileResult<Vec<_>>>()?;
	let partition = match partition_kind {
		PartitionKind::Train => dataset.train(),
		PartitionKind::Validation => dataset.validation(),
	};
	let capacity = partition.len().checked_mul(target_count).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"multi-target matrix element count overflowed usize",
		)
	})?;
	let mut lowered = Vec::with_capacity(capacity);
	let mut observations = Vec::with_capacity(partition.len());
	for (retained_position, source_row) in partition
		.retained_positions()
		.iter()
		.copied()
		.zip(partition.source_rows().iter().copied())
	{
		let row_start = lowered.len();
		let mut row_known = true;
		for target in &targets {
			let value = match target.values() {
				PreparedValues::I32(values) => {
					let value = values.get(retained_position).ok_or_else(|| {
						target_lowering_error(target.name(), source_row, "target value is absent")
					})?;
					match value {
						Some(value) => {
							let converted = *value as f32;
							if f64::from(converted) != f64::from(*value) {
								return Err(target_lowering_error(
									target.name(),
									source_row,
									format!(
										"int32 target {value} is not exactly representable as binary32"
									),
								));
							}
							Some(converted)
						}
						None => None,
					}
				}
				PreparedValues::F32Bits(values) => {
					let bits = values.get(retained_position).ok_or_else(|| {
						target_lowering_error(target.name(), source_row, "target value is absent")
					})?;
					bits.map(f32::from_bits)
				}
				PreparedValues::VariableWidth(_) => {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						"multi-target dense objectives require fixed numeric storage",
					));
				}
			};
			match value {
				Some(value) if value.is_finite() => lowered.push(value.to_bits()),
				Some(value) => {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						format!("target contains non-finite value {value}"),
					));
				}
				None => {
					row_known = false;
					lowered.push(0.0_f32.to_bits());
				}
			}
		}
		let row = &mut lowered[row_start..];
		if !row_known {
			row.fill(0.0_f32.to_bits());
			observations.push(TargetObservation::Missing);
			continue;
		}
		match task {
			DenseTask::MultiTargetBinaryClassification { .. } => {
				if let Some((column, value)) = row
					.iter()
					.copied()
					.map(f32::from_bits)
					.enumerate()
					.find(|(_, value)| *value != 0.0 && *value != 1.0)
				{
					return Err(target_lowering_error(
						targets[column].name(),
						source_row,
						format!(
							"binary target matrix contains {value}; only exact zero and one are accepted"
						),
					));
				}
			}
			DenseTask::JointMulticlassClassification { .. } => {
				let mut ones = 0_usize;
				for (column, value) in row.iter().copied().map(f32::from_bits).enumerate() {
					if value == 1.0 {
						ones += 1;
					} else if value != 0.0 {
						return Err(target_lowering_error(
							targets[column].name(),
							source_row,
							format!(
								"joint cross-entropy target contains {value}; rows must be exact one-hot vectors"
							),
						));
					}
				}
				if ones != 1 {
					return Err(target_lowering_error(
						targets[0].name(),
						source_row,
						format!(
							"joint cross-entropy row has {ones} active targets; exactly one is required"
						),
					));
				}
			}
			DenseTask::MultiTargetRegression { .. } => {}
			DenseTask::BinaryClassification { .. }
			| DenseTask::MulticlassClassification { .. }
			| DenseTask::ScalarRegression { .. } => {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					"singular task reached multi-target lowering",
				));
			}
		}
		observations.push(TargetObservation::Known);
	}
	Ok(LoweredDenseTargets {
		matrix: DenseMatrix::F32Bits {
			rows: partition.len(),
			columns: target_count,
			values: lowered,
		},
		observations,
	})
}

fn lower_i32_target(
	target: &recipe_ingest::PreparedVector,
	task: DenseTask,
	value: Option<i32>,
	source_row: usize,
) -> TrainingCompileResult<(i32, TargetObservation)> {
	let Some(value) = value else {
		return Ok((0, TargetObservation::Missing));
	};
	match task {
		DenseTask::ScalarRegression { .. } => Ok((value, TargetObservation::Known)),
		DenseTask::BinaryClassification { positive_code, .. } => {
			if let VectorMetadata::Categorical { dictionary } = target.metadata() {
				let expected_positive_code = match dictionary.len() {
					0 => -1,
					1 => 0,
					2 => 1,
					count => {
						return Err(target_lowering_error(
							target.name(),
							source_row,
							format!("categorical BCE target has unsupported label count {count}"),
						));
					}
				};
				if positive_code != expected_positive_code {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						format!(
							"categorical BCE positive code {positive_code} disagrees with expected binding {expected_positive_code}"
						),
					));
				}
				let category = usize::try_from(value).map_err(|_| {
					target_lowering_error(
						target.name(),
						source_row,
						format!("negative category code {value}"),
					)
				})?;
				if category == dictionary.len() {
					return Ok((0, TargetObservation::Unseen));
				}
				if category > dictionary.len() {
					return Err(target_lowering_error(
						target.name(),
						source_row,
						format!(
							"category code {value} exceeds reserved code {}",
							dictionary.len()
						),
					));
				}
				return Ok((i32::from(value == positive_code), TargetObservation::Known));
			}
			if !matches!(value, 0 | 1) {
				return Err(target_lowering_error(
					target.name(),
					source_row,
					format!("binary target matrix contains {value}; only exact zero and one are accepted"),
				));
			}
			Ok((value, TargetObservation::Known))
		}
		DenseTask::MulticlassClassification {
			class_count,
			reserved_code,
			..
		} => {
			let reserved_index = usize::try_from(reserved_code).map_err(|_| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					format!("multiclass reserved code {reserved_code} is negative"),
				)
			})?;
			if class_count == 0 || reserved_index.checked_add(1) != Some(class_count) {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					"multiclass class count does not include exactly one reserved unseen-label route",
				));
			}
			let category = usize::try_from(value).map_err(|_| {
				target_lowering_error(
					target.name(),
					source_row,
					format!("negative category code {value}"),
				)
			})?;
			if category == reserved_index {
				return Ok((0, TargetObservation::Unseen));
			}
			if category > reserved_index {
				return Err(target_lowering_error(
					target.name(),
					source_row,
					format!("category code {value} exceeds reserved code {reserved_code}"),
				));
			}
			Ok((value, TargetObservation::Known))
		}
		DenseTask::MultiTargetBinaryClassification { .. }
		| DenseTask::JointMulticlassClassification { .. }
		| DenseTask::MultiTargetRegression { .. } => Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"multi-target task reached singular int32 target lowering",
		)),
	}
}

fn lower_f32_target(
	name: &[u8],
	task: DenseTask,
	bits: Option<u32>,
	source_row: usize,
) -> TrainingCompileResult<(u32, TargetObservation)> {
	let Some(bits) = bits else {
		return Ok((0.0_f32.to_bits(), TargetObservation::Missing));
	};
	let value = f32::from_bits(bits);
	if !value.is_finite() {
		return Err(target_lowering_error(
			name,
			source_row,
			format!("target contains non-finite f32 bits {bits:#010x}"),
		));
	}
	if matches!(task, DenseTask::BinaryClassification { .. }) && value != 0.0 && value != 1.0 {
		return Err(target_lowering_error(
			name,
			source_row,
			format!("binary target matrix contains {value}; only exact zero and one are accepted"),
		));
	}
	Ok((bits, TargetObservation::Known))
}

fn compact_dense_rows(matrix: &DenseMatrix, rows: &[usize], role: &str) -> TrainingCompileResult<DenseMatrix> {
	let columns = matrix.columns();
	let capacity = rows.len().checked_mul(columns).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			format!("{role} compaction element count overflowed usize"),
		)
	})?;
	match matrix {
		DenseMatrix::I32 { values, .. } => {
			let mut compacted = Vec::with_capacity(capacity);
			for row in rows.iter().copied() {
				let start = row.checked_mul(columns).ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row offset overflowed usize"),
					)
				})?;
				let end = start.checked_add(columns).ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row end overflowed usize"),
					)
				})?;
				compacted.extend_from_slice(values.get(start..end).ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InconsistentRows,
						format!("{role} row {row} is absent from dense storage"),
					)
				})?);
			}
			Ok(DenseMatrix::I32 {
				rows: rows.len(),
				columns,
				values: compacted,
			})
		}
		DenseMatrix::F32Bits { values, .. } => {
			let mut compacted = Vec::with_capacity(capacity);
			for row in rows.iter().copied() {
				let start = row.checked_mul(columns).ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row offset overflowed usize"),
					)
				})?;
				let end = start.checked_add(columns).ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::ArithmeticOverflow,
						format!("{role} row end overflowed usize"),
					)
				})?;
				compacted.extend_from_slice(values.get(start..end).ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InconsistentRows,
						format!("{role} row {row} is absent from dense storage"),
					)
				})?);
			}
			Ok(DenseMatrix::F32Bits {
				rows: rows.len(),
				columns,
				values: compacted,
			})
		}
	}
}

fn target_lowering_error(name: &[u8], source_row: usize, detail: impl core::fmt::Display) -> TrainingCompileError {
	TrainingCompileError::new(
		TrainingCompileErrorKind::InvalidTargetMatrix,
		format!(
			"target {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(name)
		),
	)
}

pub(crate) fn lower_dense_features(
	dataset: &PreparedDataset,
	plan: &DenseFeaturePlan,
	partition_kind: PartitionKind,
) -> TrainingCompileResult<DenseMatrix> {
	if plan.normalization_mask.is_none() {
		return Ok(dataset.fixed_dense_matrix(VectorRole::Feature, partition_kind)?);
	}
	let partition = match partition_kind {
		PartitionKind::Train => dataset.train(),
		PartitionKind::Validation => dataset.validation(),
	};
	let capacity = partition
		.len()
		.checked_mul(plan.input_width)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"lowered dense feature element count overflowed usize",
			)
		})?;
	let features = dataset
		.vectors()
		.iter()
		.filter(|vector| vector.role() == VectorRole::Feature)
		.collect::<Vec<_>>();
	if features.len() != plan.spans.len() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			"dense feature plan no longer matches its prepared vectors",
		));
	}
	let mut output = Vec::with_capacity(capacity);
	for (retained_position, source_row) in partition
		.retained_positions()
		.iter()
		.copied()
		.zip(partition.source_rows().iter().copied())
	{
		for (feature, span) in features.iter().copied().zip(&plan.spans) {
			if feature.source_index() != span.source_vector {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidFeatureMatrix,
					"dense feature plan source identity no longer matches its prepared vector",
				));
			}
			match span.lowering {
				DenseFeatureLowering::NumericScalar => {
					output.push(lower_numeric_feature(
						feature,
						retained_position,
						source_row,
					)?);
				}
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width,
					reserved_index,
				} => {
					let PreparedValues::I32(values) = feature.values() else {
						return Err(feature_lowering_error(
							feature.name(),
							source_row,
							"dictionary encoding does not contain int32 category codes",
						));
					};
					let code = values.get(retained_position).ok_or_else(|| {
						feature_lowering_error(
							feature.name(),
							source_row,
							"category code is absent from the prepared vector",
						)
					})?;
					let expected_width = dictionary_width.checked_add(1).ok_or_else(|| {
						feature_lowering_error(
							feature.name(),
							source_row,
							"categorical span omitted its reserved route",
						)
					})?;
					if span.width != expected_width || reserved_index != dictionary_width {
						return Err(feature_lowering_error(
							feature.name(),
							source_row,
							"categorical span reserved-route identity is inconsistent",
						));
					}
					let category = match code {
						Some(code) => {
							categorical_code_index(*code, dictionary_width, feature.name(), source_row)?
						}
						None => reserved_index,
					};
					let start = output.len();
					output.resize(start + span.width, 0.0f32.to_bits());
					output[start + category] = 1.0f32.to_bits();
				}
			}
		}
	}
	Ok(DenseMatrix::F32Bits {
		rows: partition.len(),
		columns: plan.input_width,
		values: output,
	})
}

fn lower_numeric_feature(
	feature: &recipe_ingest::PreparedVector,
	retained_position: usize,
	source_row: usize,
) -> TrainingCompileResult<u32> {
	match feature.values() {
		PreparedValues::I32(values) => {
			let value = values
				.get(retained_position)
				.ok_or_else(|| {
					feature_lowering_error(
						feature.name(),
						source_row,
						"numeric value is absent from the prepared vector",
					)
				})?
				.ok_or_else(|| {
					feature_lowering_error(feature.name(), source_row, "numeric value is missing")
				})?;
			let converted = value as f32;
			if f64::from(converted) != f64::from(value) {
				return Err(feature_lowering_error(
					feature.name(),
					source_row,
					format!("int32 value {value} cannot be represented exactly by dense f32 calculation"),
				));
			}
			Ok(converted.to_bits())
		}
		PreparedValues::F32Bits(values) => values
			.get(retained_position)
			.ok_or_else(|| {
				feature_lowering_error(
					feature.name(),
					source_row,
					"numeric value is absent from the prepared vector",
				)
			})?
			.ok_or_else(|| feature_lowering_error(feature.name(), source_row, "numeric value is missing")),
		PreparedValues::VariableWidth(_) => Err(feature_lowering_error(
			feature.name(),
			source_row,
			"numeric feature unexpectedly contains variable-width values",
		)),
	}
}

fn validate_categorical_dictionary(name: &[u8], dictionary: &[Vec<u8>]) -> TrainingCompileResult<()> {
	let mut labels = std::collections::BTreeSet::new();
	if dictionary
		.iter()
		.any(|label| label.is_empty() || !labels.insert(label))
	{
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			format!(
				"categorical feature {:?} has an empty or duplicate dictionary label",
				String::from_utf8_lossy(name)
			),
		));
	}
	Ok(())
}

fn categorical_code_index(
	code: i32,
	known_width: usize,
	name: &[u8],
	source_row: usize,
) -> TrainingCompileResult<usize> {
	let index = usize::try_from(code).map_err(|_| {
		feature_lowering_error(
			name,
			source_row,
			format!("negative category code {code} violates dictionary encoding"),
		)
	})?;
	if index <= known_width {
		Ok(index)
	} else {
		Err(feature_lowering_error(
			name,
			source_row,
			format!("category code {code} exceeds dictionary and reserved index {known_width}"),
		))
	}
}

fn feature_lowering_error(name: &[u8], source_row: usize, detail: impl core::fmt::Display) -> TrainingCompileError {
	TrainingCompileError::new(
		TrainingCompileErrorKind::InvalidFeatureMatrix,
		format!(
			"feature {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(name)
		),
	)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdamWConfig {
	pub learning_rate: f32,
	pub beta_one: f32,
	pub beta_two: f32,
	pub epsilon: f32,
	pub weight_decay: f32,
}

impl Default for AdamWConfig {
	fn default() -> Self {
		Self {
			learning_rate: 1.0e-4,
			beta_one: 0.9,
			beta_two: 0.999,
			epsilon: 1.0e-8,
			weight_decay: 0.01,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct DenseTrainingConfig {
	pub layers: Vec<DenseLayer>,
	pub loss: DenseLoss,
	pub data_normalization: DenseDataNormalization,
	pub epochs: TrainingHorizon,
	pub warmup_epochs: u64,
	pub learning_rate_decay: LearningRateDecay,
	pub gradient_clip_norm: Option<f32>,
	pub normalization_epsilon: f32,
	pub reduction_tree_lanes: u32,
	pub random_seed: u64,
	pub adamw: AdamWConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryValidationConfig {
	calibration_bins: NonZeroU32,
	recall_threshold_bits: Vec<u32>,
	temperature_scaling: Option<TemperatureScalingConfig>,
}

/// Request epoch-bound categorical cross-entropy and top-one accuracy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MulticlassValidationConfig;

/// Request epoch-bound coefficient of determination over the complete
/// prepared regression validation partition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegressionValidationConfig;

impl BinaryValidationConfig {
	#[must_use]
	pub fn new(calibration_bins: NonZeroU32, recall_thresholds: impl IntoIterator<Item = f32>) -> Self {
		Self {
			calibration_bins,
			recall_threshold_bits: recall_thresholds.into_iter().map(f32::to_bits).collect(),
			temperature_scaling: None,
		}
	}

	#[must_use]
	pub fn with_temperature_scaling(mut self, config: TemperatureScalingConfig) -> Self {
		self.temperature_scaling = Some(config);
		self
	}

	#[must_use]
	pub const fn calibration_bins(&self) -> NonZeroU32 {
		self.calibration_bins
	}

	#[must_use]
	pub fn recall_thresholds(&self) -> impl ExactSizeIterator<Item = f32> + '_ {
		self.recall_threshold_bits
			.iter()
			.copied()
			.map(f32::from_bits)
	}

	#[must_use]
	pub const fn temperature_scaling(&self) -> Option<TemperatureScalingConfig> {
		self.temperature_scaling
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemperatureScalingConfig {
	pub iterations: NonZeroU64,
	pub learning_rate: f32,
	pub minimum_temperature: f32,
	pub maximum_temperature: f32,
}

impl Default for TemperatureScalingConfig {
	fn default() -> Self {
		Self {
			iterations: NonZeroU64::new(64).expect("64 is nonzero"),
			learning_rate: 0.01,
			minimum_temperature: 0.05,
			maximum_temperature: 20.0,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingBounds {
	pub train_rows: u64,
	pub epochs: TrainingHorizon,
	pub training_iterations: LoopIterations,
	pub calibration_iterations: u64,
	pub iterations: LoopIterations,
	pub warmup_iterations: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedExternalInput {
	role: ExternalInputRole,
	value: ValueId,
	dtype: DType,
	shape: Shape,
	bytes: Vec<u8>,
}

impl OwnedExternalInput {
	#[must_use]
	pub const fn role(&self) -> ExternalInputRole {
		self.role
	}

	#[must_use]
	pub const fn value(&self) -> ValueId {
		self.value
	}

	#[must_use]
	pub const fn dtype(&self) -> DType {
		self.dtype
	}

	#[must_use]
	pub const fn shape(&self) -> &Shape {
		&self.shape
	}

	#[must_use]
	pub fn bytes(&self) -> &[u8] {
		&self.bytes
	}

	pub(crate) fn new(role: ExternalInputRole, value: ValueId, dtype: DType, shape: Shape, bytes: Vec<u8>) -> Self {
		Self {
			role,
			value,
			dtype,
			shape,
			bytes,
		}
	}

	pub(crate) fn replace_bytes(&mut self, bytes: &[u8]) {
		self.bytes.clear();
		self.bytes.extend_from_slice(bytes);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalInputRole {
	TrainFeatures,
	TrainTargets,
	TrainTargetSupervision,
	ResumeEnabled,
	ResumeParameter { ordinal: usize },
	ResumeFirstMoment { ordinal: usize },
	ResumeSecondMoment { ordinal: usize },
	ResumeKMeansCentroids { block: usize },
	ResumeTreeSplitFeatures { block: usize },
	ResumeTreeSplitThresholds { block: usize },
	ResumeTreeLeafValues { block: usize },
	TrainingPoolWindowIndices { block: usize },
	TrainingPoolWinnerBases { block: usize },
	TrainingPoolGradientBatchIndices { block: usize },
	TrainingConvolutionWindowIndices { block: usize },
	TrainingConvolutionInputGradientIndices { block: usize },
	TrainingConvolutionInputGradientValidity { block: usize },
	ValidationFeatures,
	ValidationTargets,
	ValidationPoolWindowIndices { block: usize },
	ValidationPoolWinnerBases { block: usize },
	ValidationConvolutionWindowIndices { block: usize },
	FeatureNormalizationMask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParameterState {
	pub initial_parameter: ValueId,
	pub updated_parameter: ValueId,
	pub initial_first_moment: ValueId,
	pub updated_first_moment: ValueId,
	pub initial_second_moment: ValueId,
	pub updated_second_moment: ValueId,
	pub update_kernel: KernelTemplateId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseLayerState {
	pub weight: ParameterState,
	pub bias: ParameterState,
	/// Learned scalar state in ordered `.prelu()` occurrence order.
	pub prelu: Vec<ParameterState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseConvolutionState {
	pub geometry: DenseConvolutionGeometry,
	pub weight: ParameterState,
	pub bias: ParameterState,
	/// Learned scalar state for the optional convolution PReLU activation.
	pub prelu: Vec<ParameterState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseKMeansState {
	pub input_width: NonZeroU64,
	pub clusters: NonZeroU64,
	pub initial_centroids: ValueId,
	pub updated_centroids: ValueId,
	pub update_kernel: KernelTemplateId,
}

/// Persistable state of one learned embedding table and its resolved fixed
/// sequence length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseEmbeddingState {
	pub sequence_length: NonZeroU64,
	pub dimensions: NonZeroU64,
	pub vocabulary: NonZeroU64,
	pub table: ParameterState,
}

/// Persistable learned state and resolved geometry of one causal self-attention
/// block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseAttentionState {
	pub sequence_length: NonZeroU64,
	pub dimensions: NonZeroU64,
	pub heads: NonZeroU64,
	pub head_dimension: NonZeroU64,
	pub query: ParameterState,
	pub key: ParameterState,
	pub value: ParameterState,
	pub output: ParameterState,
}

/// Persistable parameter state and fixed row-sequence geometry of one vanilla
/// recurrent block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseRnnState {
	pub sequence_length: NonZeroU64,
	pub width: NonZeroU64,
	pub input_weight: ParameterState,
	pub recurrent_weight: ParameterState,
	pub bias: ParameterState,
}

/// Persistable parameter state and fixed row-sequence geometry of one
/// reset-before gated recurrent-unit block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseGruState {
	pub sequence_length: NonZeroU64,
	pub width: NonZeroU64,
	pub reset_input_weight: ParameterState,
	pub reset_recurrent_weight: ParameterState,
	pub reset_bias: ParameterState,
	pub update_input_weight: ParameterState,
	pub update_recurrent_weight: ParameterState,
	pub update_bias: ParameterState,
	pub candidate_input_weight: ParameterState,
	pub candidate_recurrent_weight: ParameterState,
	pub candidate_bias: ParameterState,
}

/// Persistable parameter state and fixed row-sequence geometry of one LSTM
/// block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseLstmState {
	pub sequence_length: NonZeroU64,
	pub width: NonZeroU64,
	pub input_gate_input_weight: ParameterState,
	pub input_gate_recurrent_weight: ParameterState,
	pub input_gate_bias: ParameterState,
	pub forget_gate_input_weight: ParameterState,
	pub forget_gate_recurrent_weight: ParameterState,
	pub forget_gate_bias: ParameterState,
	pub output_gate_input_weight: ParameterState,
	pub output_gate_recurrent_weight: ParameterState,
	pub output_gate_bias: ParameterState,
	pub candidate_input_weight: ParameterState,
	pub candidate_recurrent_weight: ParameterState,
	pub candidate_bias: ParameterState,
}

/// Persistable state of one terminal supervised tree ensemble.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseTreeState {
	pub declaration: DenseTree,
	pub input_width: NonZeroU64,
	pub output_width: NonZeroU64,
	pub internal_nodes_per_tree: NonZeroU64,
	pub leaves_per_tree: NonZeroU64,
	pub split_features: ValueId,
	pub split_thresholds: ValueId,
	pub leaf_values: ParameterState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseResidualState {
	pub branch: Vec<DenseLayerState>,
	/// Learned scalars owned by free activation steps in branch order.
	pub branch_prelu: Vec<ParameterState>,
	pub projection: Option<ParameterState>,
	/// Learned scalars owned by post-merge operations in declaration order.
	pub prelu: Vec<ParameterState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenseBlockState {
	Embedding(DenseEmbeddingState),
	Attention(DenseAttentionState),
	Rnn(DenseRnnState),
	Gru(DenseGruState),
	Lstm(DenseLstmState),
	Layer(DenseLayerState),
	Convolution(DenseConvolutionState),
	Pool(DensePoolState),
	KMeans(DenseKMeansState),
	Tree(DenseTreeState),
	Residual(DenseResidualState),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZScoreState {
	pub mean: ValueId,
	pub variance: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinMaxState {
	pub minimum: ValueId,
	pub maximum: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataNormalizationState {
	Identity,
	ZScore(ZScoreState),
	MinMax(MinMaxState),
	L2Norm,
}

/// GPU-resident optimizer transition gate and accepted-update progress.
///
/// Optimizer state advances once for a complete training partition only when
/// that partition has at least one known target. The accepted-update counter
/// and beta powers are bounded execution state rather than public artifacts;
/// semantic resume restores model and optimizer tensors under the newly
/// declared training schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OptimizerProgressState {
	pub apply_update: ValueId,
	pub initial_accepted_updates: ValueId,
	pub updated_accepted_updates: ValueId,
	pub update_kernel: KernelTemplateId,
	pub initial_beta_one_power: ValueId,
	pub updated_beta_one_power: ValueId,
	pub initial_beta_two_power: ValueId,
	pub updated_beta_two_power: ValueId,
	pub beta_update_kernel: KernelTemplateId,
	pub accepted_updates_per_epoch: u64,
	pub maximum_accepted_updates: Option<u64>,
	pub accepted_update_counter_limit: u64,
	pub warmup_accepted_updates: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationMetricFamily {
	Binary,
	Multiclass,
	Regression,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationUnavailableReason {
	NoKnownTargets,
	SingleKnownClass { known_rows: NonZeroU64 },
}

/// Whether requested validation metrics have a supervised population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationMetricStatus {
	NotRequested,
	Available {
		family: ValidationMetricFamily,
		known_rows: NonZeroU64,
	},
	Unavailable {
		family: ValidationMetricFamily,
		reason: ValidationUnavailableReason,
		split_rows: u64,
	},
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingOutputs {
	pub training_loss: ValueId,
	pub training_loss_domain: IterationDomain,
	pub normalization: DataNormalizationState,
	pub optimizer_progress: Option<OptimizerProgressState>,
	pub blocks: Vec<DenseBlockState>,
	pub layers: Vec<DenseLayerState>,
	pub validation: Option<BinaryValidationOutputs>,
	pub multiclass_validation: Option<MulticlassValidationOutputs>,
	pub regression_validation: Option<RegressionValidationOutputs>,
	pub validation_status: ValidationMetricStatus,
	pub metric_bindings: Vec<TrainingMetricBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingMetricKind {
	TrainingLoss,
	LearningRate,
	ValidationMeanBce,
	ValidationMeanCrossEntropy,
	Accuracy,
	R2,
	AuRoc,
	AuPrc,
	BrierScore,
	ExpectedCalibrationError,
	RecallAt { threshold_bits: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingMetricBinding {
	pub kind: TrainingMetricKind,
	pub metric: MetricId,
	pub value: ValueId,
	pub domain: IterationDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecallMetricOutput {
	pub threshold_bits: u32,
	pub value: ValueId,
}

impl RecallMetricOutput {
	#[must_use]
	pub const fn threshold(self) -> f32 {
		f32::from_bits(self.threshold_bits)
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryMetricOutputs {
	pub mean_bce: ValueId,
	pub accuracy: ValueId,
	pub auroc: ValueId,
	pub auprc: ValueId,
	pub brier_score: ValueId,
	pub expected_calibration_error: ValueId,
	pub recall_at: Vec<RecallMetricOutput>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MulticlassMetricOutputs {
	pub mean_cross_entropy: ValueId,
	pub accuracy: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegressionMetricOutputs {
	pub r2: ValueId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegressionValidationOutputs {
	pub predictions: ValueId,
	pub metrics: RegressionMetricOutputs,
	pub metric_domain: IterationDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemperatureScalingState {
	pub initial_temperature: ValueId,
	pub updated_temperature: ValueId,
	pub update_kernel: KernelTemplateId,
	pub iterations: NonZeroU64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryValidationOutputs {
	pub logits: ValueId,
	pub metrics: BinaryMetricOutputs,
	pub metric_domain: IterationDomain,
	pub temperature_scaling: Option<TemperatureScalingState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MulticlassValidationOutputs {
	pub logits: ValueId,
	pub metrics: MulticlassMetricOutputs,
	pub metric_domain: IterationDomain,
}

#[derive(Clone, Debug)]
pub struct CompiledTraining {
	pub(crate) program: StaticCalculationProgram,
	pub(crate) external_inputs: Vec<OwnedExternalInput>,
	pub(crate) bounds: TrainingBounds,
	pub(crate) outputs: TrainingOutputs,
	pub(crate) dataset_schema: CompiledDatasetSchema,
	pub(crate) config: DenseTrainingConfig,
	pub(crate) blocks: Vec<DenseBlock>,
	pub(crate) layers: Vec<DenseLayer>,
	pub(crate) output_adapter: Option<DenseOutputAdapter>,
}

impl CompiledTraining {
	#[must_use]
	pub const fn graph(&self) -> &CalculationGraph {
		self.program.graph()
	}

	#[must_use]
	pub const fn program(&self) -> &StaticCalculationProgram {
		&self.program
	}

	#[must_use]
	pub fn external_inputs(&self) -> &[OwnedExternalInput] {
		&self.external_inputs
	}

	#[must_use]
	pub const fn bounds(&self) -> TrainingBounds {
		self.bounds
	}

	#[must_use]
	pub const fn outputs(&self) -> &TrainingOutputs {
		&self.outputs
	}

	#[must_use]
	pub const fn dataset_schema(&self) -> &CompiledDatasetSchema {
		&self.dataset_schema
	}

	#[must_use]
	pub const fn config(&self) -> &DenseTrainingConfig {
		&self.config
	}

	#[must_use]
	pub fn layers(&self) -> &[DenseLayer] {
		&self.layers
	}

	#[must_use]
	pub fn blocks(&self) -> &[DenseBlock] {
		&self.blocks
	}

	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> {
		self.output_adapter
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedTrainingFeature {
	DynamicLoopShortening,
	ExactOptimizerResume,
}

pub const REMAINING_UNSUPPORTED: &[UnsupportedTrainingFeature] = &[
	UnsupportedTrainingFeature::DynamicLoopShortening,
	UnsupportedTrainingFeature::ExactOptimizerResume,
];

fn validate_partition(features: &DenseMatrix, targets: &DenseMatrix) -> TrainingCompileResult<()> {
	if features.rows() == 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::EmptyDataset,
			"dense training partition has no rows",
		));
	}
	if features.columns() == 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			"dense feature matrix has no columns",
		));
	}
	if features.rows() != targets.rows() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InconsistentRows,
			format!(
				"feature matrix has {} rows, target matrix has {}",
				features.rows(),
				targets.rows()
			),
		));
	}
	if targets.columns() == 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"dense training partition has no target columns",
		));
	}
	validate_matrix_storage(features, "feature")?;
	validate_matrix_storage(targets, "target")
}

fn validate_matrix_storage(matrix: &DenseMatrix, role: &str) -> TrainingCompileResult<()> {
	let expected = matrix.rows().checked_mul(matrix.columns()).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			format!("{role} element count overflowed usize"),
		)
	})?;
	let actual = match matrix {
		DenseMatrix::I32 { values, .. } => values.len(),
		DenseMatrix::F32Bits { values, .. } => {
			if let Some(bits) = values
				.iter()
				.copied()
				.find(|bits| !f32::from_bits(*bits).is_finite())
			{
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidFeatureMatrix,
					format!("{role} matrix contains non-finite f32 bits {bits:#010x}"),
				));
			}
			values.len()
		}
	};
	if actual != expected {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidFeatureMatrix,
			format!("{role} matrix stores {actual} values, expected {expected}"),
		));
	}
	Ok(())
}

pub(crate) fn validate_binary_targets(targets: &DenseMatrix) -> TrainingCompileResult<()> {
	let invalid = match targets {
		DenseMatrix::I32 { values, .. } => values
			.iter()
			.copied()
			.find(|value| !matches!(value, 0 | 1))
			.map(|value| value.to_string()),
		DenseMatrix::F32Bits { values, .. } => values
			.iter()
			.copied()
			.map(f32::from_bits)
			.find(|value| *value != 0.0 && *value != 1.0)
			.map(|value| value.to_string()),
	};
	if let Some(value) = invalid {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!("binary target matrix contains {value}; only exact zero and one are accepted"),
		));
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn nonzero(value: u64) -> NonZeroU64 {
		NonZeroU64::new(value).expect("test value is nonzero")
	}

	#[test]
	fn group_to_neuron_routing_uses_divisible_ranges_and_full_fallback() {
		assert_eq!(
			DenseGroupToNeuronRouting::resolve(nonzero(4), nonzero(4)),
			DenseGroupToNeuronRouting::Identity { width: nonzero(4) }
		);
		assert_eq!(
			DenseGroupToNeuronRouting::resolve(nonzero(4), nonzero(8)),
			DenseGroupToNeuronRouting::Expand {
				groups: nonzero(4),
				neurons: nonzero(8),
				neurons_per_group: nonzero(2),
			}
		);
		assert_eq!(
			DenseGroupToNeuronRouting::resolve(nonzero(8), nonzero(4)),
			DenseGroupToNeuronRouting::Contract {
				groups: nonzero(8),
				neurons: nonzero(4),
				groups_per_neuron: nonzero(2),
			}
		);
		assert_eq!(
			DenseGroupToNeuronRouting::resolve(nonzero(5), nonzero(8)),
			DenseGroupToNeuronRouting::FullyConnected {
				groups: nonzero(5),
				neurons: nonzero(8),
			}
		);
	}

	#[test]
	fn pool_state_preserves_logical_shape_order_and_winner_contract() {
		let state = DensePoolState::new(nonzero(7), nonzero(3), nonzero(3));
		assert_eq!(state.input_width(), Some(nonzero(21)));
		assert_eq!(state.output_width(), Some(nonzero(9)));
		assert_eq!(state.logical_input_shape(nonzero(2)), [2, 7, 3]);
		assert_eq!(state.logical_output_shape(nonzero(2)), [2, 3, 3]);
		assert_eq!(
			state.group_order(),
			DensePoolGroupOrder::GroupMajorChannelMinor
		);
		assert_eq!(
			state.winner_contract(),
			DensePoolWinnerContract::LowestLogicalIndex
		);
	}

	#[test]
	fn categorical_codes_preserve_the_reserved_route_and_reject_corruption() {
		assert_eq!(categorical_code_index(0, 3, b"color", 7).unwrap(), 0);
		assert_eq!(categorical_code_index(2, 3, b"color", 7).unwrap(), 2);
		assert_eq!(categorical_code_index(3, 3, b"color", 7).unwrap(), 3);

		let negative = categorical_code_index(-1, 3, b"color", 7).unwrap_err();
		assert_eq!(
			negative.kind,
			TrainingCompileErrorKind::InvalidFeatureMatrix
		);
		assert!(negative.detail.contains("negative category code -1"));
		let overflow = categorical_code_index(4, 3, b"color", 7).unwrap_err();
		assert_eq!(
			overflow.kind,
			TrainingCompileErrorKind::InvalidFeatureMatrix
		);
		assert!(overflow.detail.contains("reserved index 3"));
	}
}
