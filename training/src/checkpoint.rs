use core::{ fmt, num::{NonZeroU32, NonZeroU64}, sync::atomic::{AtomicU64, Ordering}, };
use alloc::collections::{BTreeMap, BTreeSet}; use std::{ ffi::OsString, fs::{self, File, OpenOptions},
	io::{self, Write}, os::unix::fs::OpenOptionsExt as _, path::{Path, PathBuf}, };

use recipe_core::{ BundleIdentity, DType, Digest, DiscoveryIdentity, EXACT_USER_RESERVATION, Label, LoopIterations,
	RealizationIdentity, RunId, TargetIdentity, ToolchainIdentity, TopologyIdentity, ValueId, };
use recipe_executor::{ExitImage, RunJournal}; use recipe_ingest::{
	EncodedImageFormat, ImageColorModel, ImageValueLayout, ImageValueRange, SemanticType, VectorEncoding,
	VectorMetadata, VectorRole, VectorSchema, }; use recipe_ogdl::{Graph as OgdlGraph, NodeId as OgdlNodeId};
use sha2::{Digest as _, Sha256};

use crate::{ AdamWConfig, CompiledFeatureSpan, CompiledTraining, CompletedTrainingExecution, DataNormalizationState,
	DecodedMulticlassClass, DenseAttention, DenseAttentionState, DenseBlockKind,
	DenseBlockState, DenseConvolution, DenseConvolutionGeometry, DenseConvolutionState, DenseDataNormalization,
	DenseEmbedding, DenseEmbeddingState, DenseFeatureLowering, DenseGroupToNeuronRouting, DenseGru, DenseGruState,
	DenseKMeans, DenseKMeansState, DenseLayer, DenseLayerState, DenseLoss, DenseLstm, DenseLstmState, DenseOperation,
	DenseOutputAdapter, DensePool, DensePoolGroupOrder, DensePoolState, DensePoolWinnerContract, DenseResidual,
	DenseResidualOperation, DenseResidualState, DenseRnn, DenseRnnState, DenseTask, DenseTrainingConfig, DenseTree,
	DenseTreeFamily, DenseTreeState, ExternalInputRole, FinalTrainingMetric, LearningRateDecay,
	MAXIMUM_REDUCTION_TREE_LANES, NativeKernelFormat, OwnedExternalInput, ParameterState, RealizedNativeKernelSet,
	TrainingBounds, TrainingHorizon, };

const FLAT_CHECKPOINT_FORMAT_VERSION: u32 = 5; const STRUCTURED_CHECKPOINT_FORMAT_VERSION: u32 = 6;
const POOL_CHECKPOINT_FORMAT_VERSION: u32 = 7; const LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION: u32 = 8;
const NATIVE_CHECKPOINT_FORMAT_VERSION: u32 = 9; const KMEANS_CHECKPOINT_FORMAT_VERSION: u32 = 10;
const MULTI_TARGET_CHECKPOINT_FORMAT_VERSION: u32 = 11; const TREE_CHECKPOINT_FORMAT_VERSION: u32 = 12;
const EMBEDDING_CHECKPOINT_FORMAT_VERSION: u32 = 13; const RNN_CHECKPOINT_FORMAT_VERSION: u32 = 14;
const GRU_CHECKPOINT_FORMAT_VERSION: u32 = 15; const LSTM_CHECKPOINT_FORMAT_VERSION: u32 = 16;
const LEGACY_CHECKPOINT_FORMAT: &str = "dense-training-checkpoint";
const SEMANTIC_MODEL_FORMAT: &str = "recipe-semantic-model";
const HEX_CHUNK_BYTES: usize = 64; const TEMP_CREATE_ATTEMPTS: u64 = 64;

static NEXT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

const fn is_semantic_model_version(version: u32) -> bool { matches!( version, LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION
			| NATIVE_CHECKPOINT_FORMAT_VERSION
			| KMEANS_CHECKPOINT_FORMAT_VERSION
			| MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
			| TREE_CHECKPOINT_FORMAT_VERSION
			| EMBEDDING_CHECKPOINT_FORMAT_VERSION
			| RNN_CHECKPOINT_FORMAT_VERSION
			| GRU_CHECKPOINT_FORMAT_VERSION
			| LSTM_CHECKPOINT_FORMAT_VERSION ) }

pub type CheckpointResult<T> = Result<T, CheckpointError>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckpointPath { segments: Vec<CheckpointPathSegment>, }

impl CheckpointPath {
	#[must_use]
	#[inline]
	pub fn segments(&self) -> &[CheckpointPathSegment] { &self.segments }

	pub(crate) fn root() -> Self { Self::default() }

	pub(crate) fn field(&self, name: impl Into<String>) -> Self { let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Field(name.into())); Self { segments } }

	pub(crate) fn index(&self, index: usize) -> Self { let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Index(index)); Self { segments } } }

impl fmt::Display for CheckpointPath {
	#[inline]
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.segments.is_empty() {
			return formatter.write_str("<checkpoint>");
		}
		let mut needs_separator = false; for segment in &self.segments { match segment {
				CheckpointPathSegment::Field(field) => { if needs_separator {
						formatter.write_str(".")?;
					}
					formatter.write_str(field)?; needs_separator = true; }
				CheckpointPathSegment::Index(index) => write!(formatter, "[{index}]")?,
			} }
		Ok(()) } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointPathSegment { Field(String), Index(usize), }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckpointDecodeErrorKind { LimitExceeded, InvalidUtf8, InvalidSyntax, MissingField, DuplicateField,
	UnknownField, InvalidValue, InconsistentValue, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointDecodeError { kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: String, }

impl CheckpointDecodeError {
	pub(crate) fn new(kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: impl Into<String>) -> Self { Self {
			kind, path, detail: detail.into(), } }

	#[must_use]
	#[inline]
	pub const fn kind(&self) -> CheckpointDecodeErrorKind { self.kind }

	#[must_use]
	#[inline]
	pub const fn path(&self) -> &CheckpointPath { &self.path }

	#[must_use]
	#[inline]
	pub fn detail(&self) -> &str { &self.detail } }

impl fmt::Display for CheckpointDecodeError {
	#[inline]
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{}: {}", self.path, self.detail)
	} }

impl core::error::Error for CheckpointDecodeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointDecodeLimits { pub source_bytes: usize, pub nodes: usize, pub vectors: usize,
	pub feature_spans: usize, pub layers: usize, pub metadata_entries: usize, pub tensors: usize, pub tensor_rank: usize,
	pub tensor_bytes: usize, pub total_payload_bytes: usize, }

impl Default for CheckpointDecodeLimits {
	#[inline]
	fn default() -> Self { Self { source_bytes: 1 << 30, nodes: 4_000_000, vectors: 0x0001_0000,
			feature_spans: 0x0001_0000, layers: 0x4000, metadata_entries: 1_000_000, tensors: 100_000, tensor_rank: 16,
			tensor_bytes: 1 << 30, total_payload_bytes: 1 << 30, } } }

#[derive(Debug)]
#[non_exhaustive]
pub enum CheckpointError { Decode(CheckpointDecodeError), InvalidManifest { detail: String, }, IncompatibleResume {
		detail: String, }, DuplicateOutput { value: ValueId, }, MissingOutput { value: ValueId, }, UnexpectedOutput {
		value: ValueId, }, OutputDTypeMismatch { value: ValueId, expected: DType, actual: DType, }, OutputSizeMismatch {
		value: ValueId, expected: u64, actual: u64, }, NativeKernelUnavailable { requested: NativeKernelFormat, },
	NativeKernelAmbiguous { requested: NativeKernelFormat, images: usize, }, InvalidTarget { path: PathBuf, detail: String,
	}, InsufficientCapacity { path: PathBuf, available: u64, checkpoint_allocation: u64, reservation: u64, }, Io {
		operation: &'static str,
		path: PathBuf, source: io::Error, }, }

impl CheckpointError { pub(crate) fn manifest(detail: impl Into<String>) -> Self { Self::InvalidManifest {
			detail: detail.into(), } }

	pub(crate) fn invalid_target(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self { Self::InvalidTarget {
			path: path.into(), detail: detail.into(), } }

	fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
		Self::Io { operation, path: path.into(), source, } } }

impl fmt::Display for CheckpointError {
	#[inline]
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Decode(error) => write!(formatter, "decode checkpoint: {error}"),
			Self::InvalidManifest { detail } => write!(formatter, "invalid checkpoint manifest: {detail}"),
			Self::IncompatibleResume { detail } => write!(formatter, "incompatible training resume: {detail}"),
			Self::DuplicateOutput { value } => { write!( formatter,
					"checkpoint output {value} appears more than once"
				) }
			Self::MissingOutput { value } => write!(formatter, "checkpoint output {value} is absent"),
			Self::UnexpectedOutput { value } => { write!( formatter,
					"execution returned unexpected checkpoint output {value}"
				) }
			Self::OutputDTypeMismatch { value, expected, actual, } => { write!( formatter,
					"checkpoint output {value} has dtype {actual:?}, expected {expected:?}"
				) }
			Self::OutputSizeMismatch { value, expected, actual, } => { write!( formatter,
					"checkpoint output {value} has {actual} bytes, expected {expected}"
				) }
			Self::NativeKernelUnavailable { requested } => { write!( formatter,
					"the realized execution contains no .{} native kernel image",
					requested.extension() ) }
			Self::NativeKernelAmbiguous { requested, images } => { write!( formatter,
					"the realized execution contains {images} distinct .{} images; one native file cannot represent them",
					requested.extension() ) }
			Self::InvalidTarget { path, detail } => { write!( formatter,
					"invalid checkpoint target {}: {detail}",
					path.display() ) }
			Self::InsufficientCapacity { path, available, checkpoint_allocation, reservation, } => { write!( formatter,
					"checkpoint target {} has {available} available bytes; writing needs \
				 {checkpoint_allocation} bytes while preserving {reservation} bytes",
					path.display() ) }
			Self::Io { operation, path, source,
			} => write!(formatter, "{operation} {}: {source}", path.display()),
		} } }

impl core::error::Error for CheckpointError {
	#[inline]
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		match self { Self::Decode(error) => Some(error), Self::Io { source, .. } => Some(source), _ => None, } } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointVectorSchema { source_index: usize, name: Vec<u8>, role: VectorRole, semantic_type: SemanticType,
	encoding: VectorEncoding, metadata: VectorMetadata, }

impl CheckpointVectorSchema {
	#[must_use]
	#[inline]
	pub const fn source_index(&self) -> usize { self.source_index }

	#[must_use]
	#[inline]
	pub fn name(&self) -> &[u8] { &self.name }

	#[must_use]
	#[inline]
	pub const fn role(&self) -> VectorRole { self.role }

	#[must_use]
	#[inline]
	pub const fn semantic_type(&self) -> SemanticType { self.semantic_type }

	#[must_use]
	#[inline]
	pub const fn encoding(&self) -> VectorEncoding { self.encoding }

	#[must_use]
	#[inline]
	pub const fn dtype(&self) -> Option<DType> { self.encoding.dtype() }

	#[must_use]
	#[inline]
	pub const fn metadata(&self) -> &VectorMetadata { &self.metadata } }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckpointImageMetadata { format: EncodedImageFormat, width: u32, height: u32, channels: Option<u8>,
	color_model: Option<ImageColorModel>, sample_bits: Option<u8>, value_layout: ImageValueLayout,
	value_range: ImageValueRange, }

impl CheckpointImageMetadata { pub(crate) const fn new( format: EncodedImageFormat, width: u32, height: u32,
		channels: Option<u8>, color_model: Option<ImageColorModel>, sample_bits: Option<u8>, ) -> Self { Self { format, width,
			height, channels, color_model, sample_bits, value_layout: ImageValueLayout::EncodedFile,
			value_range: ImageValueRange::EncodedBytes, } }

	#[must_use]
	#[inline]
	pub const fn format(self) -> EncodedImageFormat { self.format }

	#[must_use]
	#[inline]
	pub const fn width(self) -> u32 { self.width }

	#[must_use]
	#[inline]
	pub const fn height(self) -> u32 { self.height }

	#[must_use]
	#[inline]
	pub const fn channels(self) -> Option<u8> { self.channels }

	#[must_use]
	#[inline]
	pub const fn color_model(self) -> Option<ImageColorModel> { self.color_model }

	#[must_use]
	#[inline]
	pub const fn sample_bits(self) -> Option<u8> { self.sample_bits }

	#[must_use]
	#[inline]
	pub const fn value_layout(self) -> ImageValueLayout { self.value_layout }

	#[must_use]
	#[inline]
	pub const fn value_range(self) -> ImageValueRange { self.value_range } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointArtifactMetadata { None, Temporal { unix_seconds: i64, nanoseconds: u32, }, Categorical {
		dictionary: Vec<Vec<u8>>, }, Ordinal { ordered_labels: Vec<Vec<u8>>, }, Image {
		encoded_variants: Vec<CheckpointImageMetadata>, }, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointArtifactVector { source_index: usize, name: Vec<u8>, role: VectorRole, semantic_type: SemanticType,
	encoding: VectorEncoding, metadata: CheckpointArtifactMetadata, }

impl CheckpointArtifactVector { pub(crate) fn new( source_index: usize, name: Vec<u8>, role: VectorRole,
		semantic_type: SemanticType, encoding: VectorEncoding, metadata: CheckpointArtifactMetadata, ) -> Self { Self {
			source_index, name, role, semantic_type, encoding, metadata, } }

	pub(crate) fn from_schema(schema: &VectorSchema) -> Self { Self { source_index: schema.source_index(),
			name: schema.name().to_vec(), role: schema.role(), semantic_type: schema.semantic_type(),
			encoding: schema.encoding(), metadata: artifact_metadata(schema.metadata()), } }

	#[must_use]
	#[inline]
	pub const fn source_index(&self) -> usize { self.source_index }

	#[must_use]
	#[inline]
	pub fn name(&self) -> &[u8] { &self.name }

	#[must_use]
	#[inline]
	pub const fn role(&self) -> VectorRole { self.role }

	#[must_use]
	#[inline]
	pub const fn semantic_type(&self) -> SemanticType { self.semantic_type }

	#[must_use]
	#[inline]
	pub const fn encoding(&self) -> VectorEncoding { self.encoding }

	#[must_use]
	#[inline]
	pub const fn dtype(&self) -> Option<DType> { self.encoding.dtype() }

	#[must_use]
	#[inline]
	pub const fn metadata(&self) -> &CheckpointArtifactMetadata { &self.metadata } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointTensorImage { dtype: DType, shape: Vec<u64>, bytes: Vec<u8>, }

impl CheckpointTensorImage {
	#[must_use]
	#[inline]
	pub const fn dtype(&self) -> DType { self.dtype }

	#[must_use]
	#[inline]
	pub fn shape(&self) -> &[u64] { &self.shape }

	#[must_use]
	#[inline]
	pub fn bytes(&self) -> &[u8] { &self.bytes } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointParameterImage { parameter: CheckpointTensorImage, first_moment: CheckpointTensorImage,
	second_moment: CheckpointTensorImage, }

impl CheckpointParameterImage {
	#[must_use]
	#[inline]
	pub const fn parameter(&self) -> &CheckpointTensorImage { &self.parameter }

	#[must_use]
	#[inline]
	pub const fn first_moment(&self) -> &CheckpointTensorImage { &self.first_moment }

	#[must_use]
	#[inline]
	pub const fn second_moment(&self) -> &CheckpointTensorImage { &self.second_moment } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointLayerImage { declaration: DenseLayer, weight: CheckpointParameterImage,
	bias: CheckpointParameterImage, prelu: Vec<CheckpointParameterImage>, }

impl CheckpointLayerImage {
	#[must_use]
	#[inline]
	pub const fn declaration(&self) -> &DenseLayer { &self.declaration }

	#[must_use]
	#[inline]
	pub const fn weight(&self) -> &CheckpointParameterImage { &self.weight }

	#[must_use]
	#[inline]
	pub const fn bias(&self) -> &CheckpointParameterImage { &self.bias }

	#[must_use]
	#[inline]
	pub fn prelu(&self) -> &[CheckpointParameterImage] { &self.prelu } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointConvolutionImage { declaration: DenseConvolution, geometry: DenseConvolutionGeometry,
	weight: CheckpointParameterImage, bias: CheckpointParameterImage, prelu: Vec<CheckpointParameterImage>, }

impl CheckpointConvolutionImage {
	#[must_use]
	#[inline]
	pub const fn declaration(&self) -> &DenseConvolution { &self.declaration }

	#[must_use]
	#[inline]
	pub const fn geometry(&self) -> DenseConvolutionGeometry { self.geometry }

	#[must_use]
	#[inline]
	pub const fn weight(&self) -> &CheckpointParameterImage { &self.weight }

	#[must_use]
	#[inline]
	pub const fn bias(&self) -> &CheckpointParameterImage { &self.bias }

	#[must_use]
	#[inline]
	pub fn prelu(&self) -> &[CheckpointParameterImage] { &self.prelu } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointResidualBranchImage { Layer(CheckpointLayerImage), Operation(DenseOperation), }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointResidualSkipImage { Identity, Projection(CheckpointParameterImage), }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointResidualImage { branch: Vec<CheckpointResidualBranchImage>,
	branch_prelu: Vec<CheckpointParameterImage>, output_width: NonZeroU64, skip: CheckpointResidualSkipImage,
	operations: Vec<DenseOperation>, prelu: Vec<CheckpointParameterImage>, }

impl CheckpointResidualImage {
	#[must_use]
	#[inline]
	pub fn branch(&self) -> &[CheckpointResidualBranchImage] { &self.branch }

	#[must_use]
	#[inline]
	pub fn branch_prelu(&self) -> &[CheckpointParameterImage] { &self.branch_prelu }

	#[must_use]
	#[inline]
	pub const fn output_width(&self) -> NonZeroU64 { self.output_width }

	#[must_use]
	#[inline]
	pub const fn skip(&self) -> &CheckpointResidualSkipImage { &self.skip }

	#[must_use]
	#[inline]
	pub fn operations(&self) -> &[DenseOperation] { &self.operations }

	#[must_use]
	#[inline]
	pub fn prelu(&self) -> &[CheckpointParameterImage] { &self.prelu } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointPoolImage { size: NonZeroU64, group_to_neuron: Option<NonZeroU64>, input_length: NonZeroU64,
	channels: NonZeroU64, output_length: NonZeroU64, input_width: NonZeroU64, output_width: NonZeroU64,
	group_order: DensePoolGroupOrder, winner_contract: DensePoolWinnerContract, }

impl CheckpointPoolImage {
	#[must_use]
	#[inline]
	pub const fn size(&self) -> NonZeroU64 { self.size }

	#[must_use]
	#[inline]
	pub const fn group_to_neuron(&self) -> Option<NonZeroU64> { self.group_to_neuron }

	#[must_use]
	#[inline]
	pub const fn input_length(&self) -> NonZeroU64 { self.input_length }

	#[must_use]
	#[inline]
	pub const fn channels(&self) -> NonZeroU64 { self.channels }

	#[must_use]
	#[inline]
	pub const fn output_length(&self) -> NonZeroU64 { self.output_length }

	#[must_use]
	#[inline]
	pub const fn input_width(&self) -> NonZeroU64 { self.input_width }

	#[must_use]
	#[inline]
	pub const fn output_width(&self) -> NonZeroU64 { self.output_width }

	#[must_use]
	#[inline]
	pub const fn group_order(&self) -> DensePoolGroupOrder { self.group_order }

	#[must_use]
	#[inline]
	pub const fn winner_contract(&self) -> DensePoolWinnerContract { self.winner_contract } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointKMeansImage { clusters: NonZeroU64, group_to_neuron: Option<NonZeroU64>, input_width: NonZeroU64,
	centroids: CheckpointTensorImage, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointEmbeddingImage { dimensions: NonZeroU64, vocabulary: NonZeroU64, sequence_length: NonZeroU64,
	table: CheckpointParameterImage, }

impl CheckpointEmbeddingImage {
	#[must_use]
	#[inline]
	pub const fn dimensions(&self) -> NonZeroU64 { self.dimensions }

	#[must_use]
	#[inline]
	pub const fn vocabulary(&self) -> NonZeroU64 { self.vocabulary }

	#[must_use]
	#[inline]
	pub const fn sequence_length(&self) -> NonZeroU64 { self.sequence_length }

	#[must_use]
	#[inline]
	pub const fn table(&self) -> &CheckpointParameterImage { &self.table } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointAttentionImage { sequence_length: NonZeroU64, dimensions: NonZeroU64, heads: NonZeroU64,
	head_dimension: NonZeroU64, query: CheckpointParameterImage, key: CheckpointParameterImage,
	value: CheckpointParameterImage, output: CheckpointParameterImage, }

impl CheckpointAttentionImage {
	#[must_use]
	#[inline]
	pub const fn sequence_length(&self) -> NonZeroU64 { self.sequence_length }

	#[must_use]
	#[inline]
	pub const fn dimensions(&self) -> NonZeroU64 { self.dimensions }

	#[must_use]
	#[inline]
	pub const fn heads(&self) -> NonZeroU64 { self.heads }

	#[must_use]
	#[inline]
	pub const fn head_dimension(&self) -> NonZeroU64 { self.head_dimension }

	#[must_use]
	#[inline]
	pub const fn query(&self) -> &CheckpointParameterImage { &self.query }

	#[must_use]
	#[inline]
	pub const fn key(&self) -> &CheckpointParameterImage { &self.key }

	#[must_use]
	#[inline]
	pub const fn value(&self) -> &CheckpointParameterImage { &self.value }

	#[must_use]
	#[inline]
	pub const fn output(&self) -> &CheckpointParameterImage { &self.output } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointRnnImage { sequence_length: NonZeroU64, width: NonZeroU64, input_weight: CheckpointParameterImage,
	recurrent_weight: CheckpointParameterImage, bias: CheckpointParameterImage, }

impl CheckpointRnnImage {
	#[must_use]
	#[inline]
	pub const fn sequence_length(&self) -> NonZeroU64 { self.sequence_length }

	#[must_use]
	#[inline]
	pub const fn width(&self) -> NonZeroU64 { self.width }

	#[must_use]
	#[inline]
	pub const fn input_weight(&self) -> &CheckpointParameterImage { &self.input_weight }

	#[must_use]
	#[inline]
	pub const fn recurrent_weight(&self) -> &CheckpointParameterImage { &self.recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn bias(&self) -> &CheckpointParameterImage { &self.bias } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointGruImage { sequence_length: NonZeroU64, width: NonZeroU64,
	reset_input_weight: CheckpointParameterImage, reset_recurrent_weight: CheckpointParameterImage,
	reset_bias: CheckpointParameterImage, update_input_weight: CheckpointParameterImage,
	update_recurrent_weight: CheckpointParameterImage, update_bias: CheckpointParameterImage,
	candidate_input_weight: CheckpointParameterImage, candidate_recurrent_weight: CheckpointParameterImage,
	candidate_bias: CheckpointParameterImage, }

impl CheckpointGruImage {
	#[must_use]
	#[inline]
	pub const fn sequence_length(&self) -> NonZeroU64 { self.sequence_length }

	#[must_use]
	#[inline]
	pub const fn width(&self) -> NonZeroU64 { self.width }

	#[must_use]
	#[inline]
	pub const fn reset_input_weight(&self) -> &CheckpointParameterImage { &self.reset_input_weight }

	#[must_use]
	#[inline]
	pub const fn reset_recurrent_weight(&self) -> &CheckpointParameterImage { &self.reset_recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn reset_bias(&self) -> &CheckpointParameterImage { &self.reset_bias }

	#[must_use]
	#[inline]
	pub const fn update_input_weight(&self) -> &CheckpointParameterImage { &self.update_input_weight }

	#[must_use]
	#[inline]
	pub const fn update_recurrent_weight(&self) -> &CheckpointParameterImage { &self.update_recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn update_bias(&self) -> &CheckpointParameterImage { &self.update_bias }

	#[must_use]
	#[inline]
	pub const fn candidate_input_weight(&self) -> &CheckpointParameterImage { &self.candidate_input_weight }

	#[must_use]
	#[inline]
	pub const fn candidate_recurrent_weight(&self) -> &CheckpointParameterImage { &self.candidate_recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn candidate_bias(&self) -> &CheckpointParameterImage { &self.candidate_bias } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointLstmImage { sequence_length: NonZeroU64, width: NonZeroU64,
	input_gate_input_weight: CheckpointParameterImage, input_gate_recurrent_weight: CheckpointParameterImage,
	input_gate_bias: CheckpointParameterImage, forget_gate_input_weight: CheckpointParameterImage,
	forget_gate_recurrent_weight: CheckpointParameterImage, forget_gate_bias: CheckpointParameterImage,
	output_gate_input_weight: CheckpointParameterImage, output_gate_recurrent_weight: CheckpointParameterImage,
	output_gate_bias: CheckpointParameterImage, candidate_input_weight: CheckpointParameterImage,
	candidate_recurrent_weight: CheckpointParameterImage, candidate_bias: CheckpointParameterImage, }

impl CheckpointLstmImage {
	#[must_use]
	#[inline]
	pub const fn sequence_length(&self) -> NonZeroU64 { self.sequence_length }

	#[must_use]
	#[inline]
	pub const fn width(&self) -> NonZeroU64 { self.width }

	#[must_use]
	#[inline]
	pub const fn input_gate_input_weight(&self) -> &CheckpointParameterImage { &self.input_gate_input_weight }

	#[must_use]
	#[inline]
	pub const fn input_gate_recurrent_weight(&self) -> &CheckpointParameterImage { &self.input_gate_recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn input_gate_bias(&self) -> &CheckpointParameterImage { &self.input_gate_bias }

	#[must_use]
	#[inline]
	pub const fn forget_gate_input_weight(&self) -> &CheckpointParameterImage { &self.forget_gate_input_weight }

	#[must_use]
	#[inline]
	pub const fn forget_gate_recurrent_weight(&self) -> &CheckpointParameterImage { &self.forget_gate_recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn forget_gate_bias(&self) -> &CheckpointParameterImage { &self.forget_gate_bias }

	#[must_use]
	#[inline]
	pub const fn output_gate_input_weight(&self) -> &CheckpointParameterImage { &self.output_gate_input_weight }

	#[must_use]
	#[inline]
	pub const fn output_gate_recurrent_weight(&self) -> &CheckpointParameterImage { &self.output_gate_recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn output_gate_bias(&self) -> &CheckpointParameterImage { &self.output_gate_bias }

	#[must_use]
	#[inline]
	pub const fn candidate_input_weight(&self) -> &CheckpointParameterImage { &self.candidate_input_weight }

	#[must_use]
	#[inline]
	pub const fn candidate_recurrent_weight(&self) -> &CheckpointParameterImage { &self.candidate_recurrent_weight }

	#[must_use]
	#[inline]
	pub const fn candidate_bias(&self) -> &CheckpointParameterImage { &self.candidate_bias } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointTreeImage { declaration: DenseTree, input_width: NonZeroU64, output_width: NonZeroU64,
	internal_nodes_per_tree: NonZeroU64, leaves_per_tree: NonZeroU64, split_features: CheckpointTensorImage,
	split_thresholds: CheckpointTensorImage, leaf_values: CheckpointParameterImage, }

impl CheckpointTreeImage {
	#[must_use]
	#[inline]
	pub const fn declaration(&self) -> DenseTree { self.declaration }

	#[must_use]
	#[inline]
	pub const fn input_width(&self) -> NonZeroU64 { self.input_width }

	#[must_use]
	#[inline]
	pub const fn output_width(&self) -> NonZeroU64 { self.output_width }

	#[must_use]
	#[inline]
	pub const fn internal_nodes_per_tree(&self) -> NonZeroU64 { self.internal_nodes_per_tree }

	#[must_use]
	#[inline]
	pub const fn leaves_per_tree(&self) -> NonZeroU64 { self.leaves_per_tree }

	#[must_use]
	#[inline]
	pub const fn split_features(&self) -> &CheckpointTensorImage { &self.split_features }

	#[must_use]
	#[inline]
	pub const fn split_thresholds(&self) -> &CheckpointTensorImage { &self.split_thresholds }

	#[must_use]
	#[inline]
	pub const fn leaf_values(&self) -> &CheckpointParameterImage { &self.leaf_values } }

impl CheckpointKMeansImage {
	#[must_use]
	#[inline]
	pub const fn clusters(&self) -> NonZeroU64 { self.clusters }

	#[must_use]
	#[inline]
	pub const fn group_to_neuron(&self) -> Option<NonZeroU64> { self.group_to_neuron }

	#[must_use]
	#[inline]
	pub const fn input_width(&self) -> NonZeroU64 { self.input_width }

	#[must_use]
	#[inline]
	pub const fn centroids(&self) -> &CheckpointTensorImage { &self.centroids } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointBlockImage { Embedding(CheckpointEmbeddingImage), Attention(CheckpointAttentionImage),
	Rnn(CheckpointRnnImage), Gru(CheckpointGruImage), Lstm(CheckpointLstmImage), Layer(CheckpointLayerImage),
	Convolution(CheckpointConvolutionImage), Pool(CheckpointPoolImage), KMeans(CheckpointKMeansImage),
	Tree(CheckpointTreeImage), Residual(CheckpointResidualImage), }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointNativeKernel { format: NativeKernelFormat, target: TargetIdentity, toolchain: ToolchainIdentity,
	digest: Digest, }

impl CheckpointNativeKernel {
	#[must_use]
	#[inline]
	pub const fn format(&self) -> NativeKernelFormat { self.format }

	#[must_use]
	#[inline]
	pub const fn target(&self) -> &TargetIdentity { &self.target }

	#[must_use]
	#[inline]
	pub const fn toolchain(&self) -> &ToolchainIdentity { &self.toolchain }

	#[must_use]
	#[inline]
	pub const fn digest(&self) -> Digest { self.digest } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointNativeRealization { program: Digest, realization: RealizationIdentity, topology: TopologyIdentity,
	discovery: DiscoveryIdentity, kernels: Vec<CheckpointNativeKernel>, }

impl CheckpointNativeRealization {
	#[must_use]
	#[inline]
	pub const fn program(&self) -> Digest { self.program }

	#[must_use]
	#[inline]
	pub const fn realization(&self) -> RealizationIdentity { self.realization }

	#[must_use]
	#[inline]
	pub const fn topology(&self) -> TopologyIdentity { self.topology }

	#[must_use]
	#[inline]
	pub const fn discovery(&self) -> DiscoveryIdentity { self.discovery }

	#[must_use]
	#[inline]
	pub fn kernels(&self) -> &[CheckpointNativeKernel] { &self.kernels } }

impl CheckpointBlockImage {
	#[must_use]
	#[inline]
	pub fn output_width(&self) -> NonZeroU64 { crate::inference::inference_block(self).output_width() }

	#[must_use]
	#[inline]
	pub fn output_operations(&self) -> &[DenseOperation] { crate::inference::inference_block(self).output_operations() } }

#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointArtifact { format_version: u32, vectors: Vec<CheckpointArtifactVector>,
	feature_spans: Vec<CompiledFeatureSpan>, feature_normalization_mask: Vec<u32>, feature_width: usize,
	target_source_indices: Vec<usize>, task: DenseTask, output_adapter: Option<DenseOutputAdapter>,
	config: DenseTrainingConfig, bounds: TrainingBounds, normalization: Vec<CheckpointTensorImage>,
	layers: Vec<CheckpointLayerImage>, blocks: Vec<CheckpointBlockImage>, temperature: Option<CheckpointTensorImage>,
	native: Option<CheckpointNativeRealization>, }

impl CheckpointArtifact {
	#[must_use]
	#[inline]
	pub const fn format_version(&self) -> u32 { self.format_version }

	#[must_use]
	#[inline]
	pub fn vectors(&self) -> &[CheckpointArtifactVector] { &self.vectors }

	#[must_use]
	#[inline]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] { &self.feature_spans }

	#[must_use]
	#[inline]
	pub fn feature_normalization_mask(&self) -> &[u32] { &self.feature_normalization_mask }

	#[must_use]
	#[inline]
	pub const fn feature_width(&self) -> usize { self.feature_width }

	#[must_use]
	#[inline]
	pub fn target_source_indices(&self) -> &[usize] { &self.target_source_indices }

	#[must_use]
	#[inline]
	pub const fn task(&self) -> DenseTask { self.task }

	#[must_use]
	#[inline]
	pub fn target_dtype(&self) -> Option<DType> { self.target_dtypes().next() }

	#[must_use]
	#[inline]
	pub fn target_dtypes(&self) -> impl Iterator<Item = DType> + '_ {
		self.target_source_indices .iter() .filter_map(|source_index| { self.vectors .iter()
					.find(|vector| vector.source_index == *source_index && vector.role == VectorRole::Target)
					.and_then(CheckpointArtifactVector::dtype) }) }

	#[must_use]
	#[inline]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> { self.output_adapter }

	#[must_use]
	#[inline]
	pub const fn config(&self) -> &DenseTrainingConfig { &self.config }

	#[must_use]
	#[inline]
	pub const fn bounds(&self) -> TrainingBounds { self.bounds }

	#[must_use]
	#[inline]
	pub fn normalization(&self) -> &[CheckpointTensorImage] { &self.normalization }

	#[must_use]
	#[inline]
	pub fn layers(&self) -> &[CheckpointLayerImage] { &self.layers }

	#[must_use]
	#[inline]
	pub fn blocks(&self) -> &[CheckpointBlockImage] { &self.blocks }

	#[must_use]
	#[inline]
	pub const fn temperature(&self) -> Option<&CheckpointTensorImage> { self.temperature.as_ref() }

	#[must_use]
	#[inline]
	pub const fn native_realization(&self) -> Option<&CheckpointNativeRealization> { self.native.as_ref() }

	#[must_use]
	#[inline]
	pub fn decode_multiclass_class(&self, class: usize) -> Option<DecodedMulticlassClass<'_>> {
		let DenseTask::MulticlassClassification { target_vector, class_count, reserved_code, } = self.task
		else { return None; }; if class >= class_count { return None; }
		let target = self .vectors .iter()
			.find(|vector| vector.source_index == target_vector && vector.role == VectorRole::Target)?;
		let CheckpointArtifactMetadata::Categorical { dictionary } = &target.metadata else { return None; };
		if i32::try_from(class).ok() == Some(reserved_code) { return Some(DecodedMulticlassClass::ReservedUnseen); }
		dictionary .get(class) .map(|label| DecodedMulticlassClass::Label(label)) }

	#[inline]
	pub fn encode(&self) -> CheckpointResult<Vec<u8>> { validate_artifact(self)?; let mut output = Vec::new();
		encode_artifact(self, &mut output)
			.map_err(|error| CheckpointError::io("encode checkpoint", Path::new("<memory>"), error))?;
		Ok(output) } }

#[inline]
pub fn decode_checkpoint(bytes: &[u8], limits: CheckpointDecodeLimits) -> CheckpointResult<CheckpointArtifact> {
	if bytes.len() > limits.source_bytes { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
			CheckpointPath::root(), format!(
				"source contains {} bytes, limit is {}",
				bytes.len(), limits.source_bytes ), )); }
	let text = core::str::from_utf8(bytes).map_err(|error| { decode_error( CheckpointDecodeErrorKind::InvalidUtf8,
			CheckpointPath::root(),
			format!("source is not UTF-8: {error}"),
		) })?; let graph = OgdlGraph::parse(text).map_err(|error| { decode_error( CheckpointDecodeErrorKind::InvalidSyntax,
			CheckpointPath::root(), error.to_string(), ) })?; if graph.len() > limits.nodes { return Err(decode_error(
			CheckpointDecodeErrorKind::LimitExceeded, CheckpointPath::root(), format!(
				"document contains {} nodes, limit is {}",
				graph.len(), limits.nodes ), )); }
	Decoder { graph: &graph, limits, block_count: 0, tensor_count: 0, payload_bytes: 0, metadata_entries: 0, } .decode() }

pub(crate) fn decode_error( kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: impl Into<String>,
) -> CheckpointError { CheckpointError::Decode(CheckpointDecodeError::new(kind, path, detail)) }

struct Decoder<'a> {
	graph: &'a OgdlGraph,
	limits: CheckpointDecodeLimits, block_count: usize, tensor_count: usize, payload_bytes: usize, metadata_entries: usize,
}

struct FieldSet { fields: BTreeMap<String, OgdlNodeId>, }

impl FieldSet { fn require(&self, name: &str, path: &CheckpointPath) -> CheckpointResult<OgdlNodeId> {
		self.fields.get(name).copied().ok_or_else(|| { decode_error( CheckpointDecodeErrorKind::MissingField,
				path.field(name),
				format!("required field {name:?} is absent"),
			) }) }

	fn optional(&self, name: &str) -> Option<OgdlNodeId> { self.fields.get(name).copied() } }

impl Decoder<'_> {
	fn node(&self, id: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<&recipe_ogdl::Node> {
		self.graph.node(id).ok_or_else(|| { decode_error( CheckpointDecodeErrorKind::InvalidSyntax, path.clone(),
				"OGDL node identity is absent",
			) }) }

	fn fields( &self, parent: OgdlNodeId, path: &CheckpointPath, required: &[&str], optional: &[&str],
	) -> CheckpointResult<FieldSet> { let allowed = required .iter() .chain(optional) .copied() .collect::<BTreeSet<_>>();
		let mut fields = BTreeMap::new(); for child in self.node(parent, path)?.children() {
			let child_node = self.node(*child, path)?; let name = child_node.text(); if !allowed.contains(name) {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, path.field(name),
					format!("field {name:?} is not valid here"),
				)); }
			if fields.insert(name.to_owned(), *child).is_some() { return Err(decode_error(
					CheckpointDecodeErrorKind::DuplicateField, path.field(name),
					format!("field {name:?} appears more than once"),
				)); } }
		let fields = FieldSet { fields }; for name in required { fields.require(name, path)?; }
		Ok(fields) }

	fn scalar<'a>(&'a self, field: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<&'a str> {
		let children = self.node(field, path)?.children(); if children.len() != 1 { return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax, path.clone(),
				format!("scalar field has {} values, expected one", children.len()),
			)); }
		let value = self.node(children[0], path)?; if !value.children().is_empty() { return Err(decode_error(
				CheckpointDecodeErrorKind::UnknownField, path.field(value.children().len().to_string()),
				"scalar value has unexpected descendants",
			)); }
		Ok(value.text()) }

	fn required_scalar<'a>(&'a self, fields: &FieldSet, name: &str, path: &CheckpointPath) -> CheckpointResult<&'a str> { self.scalar(fields.require(name, path)?, &path.field(name)) }

	fn tagged<'a>(
		&'a self,
		field: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<(&'a str, &'a [OgdlNodeId])> {
		let children = self.node(field, path)?.children(); let tag_id = children.first().copied().ok_or_else(|| {
			decode_error( CheckpointDecodeErrorKind::MissingField, path.clone(),
				"tagged field has no tag value",
			) })?; let tag = self.node(tag_id, path)?; if !tag.children().is_empty() { return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax, path.clone(),
				"tag value has unexpected descendants",
			)); }
		Ok((tag.text(), &children[1..])) }

	fn fields_from_children( &self, children: &[OgdlNodeId], path: &CheckpointPath, required: &[&str], optional: &[&str],
	) -> CheckpointResult<FieldSet> { let allowed = required .iter() .chain(optional) .copied() .collect::<BTreeSet<_>>();
		let mut fields = BTreeMap::new(); for child in children { let name = self.node(*child, path)?.text();
			if !allowed.contains(name) { return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, path.field(name),
					format!("field {name:?} is not valid here"),
				)); }
			if fields.insert(name.to_owned(), *child).is_some() { return Err(decode_error(
					CheckpointDecodeErrorKind::DuplicateField, path.field(name),
					format!("field {name:?} appears more than once"),
				)); } }
		let fields = FieldSet { fields }; for name in required { fields.require(name, path)?; }
		Ok(fields) } }

impl Decoder<'_> {
	fn decode(mut self) -> CheckpointResult<CheckpointArtifact> {
		let root_path = CheckpointPath::root().field("recipe");
		if self.graph.roots().len() != 1 { return Err(decode_error( CheckpointDecodeErrorKind::InvalidSyntax,
				CheckpointPath::root(), format!(
					"document has {} roots, expected one",
					self.graph.roots().len() ), )); }
		let root = self.graph.roots()[0];
		if self.node(root, &root_path)?.text() != "recipe" {
			return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, CheckpointPath::root(),
				"root must be `recipe`",
			)); }
		let fields = self.fields( root, &root_path, &[
				"format",
				"version",
				"semantics",
				"dataset",
				"training",
				"model",
			],
			&["native"],
		)?;
		let format = self.required_scalar(&fields, "format", &root_path)?;
		let version = self.parse_u32(
			self.required_scalar(&fields, "version", &root_path)?,
			&root_path.field("version"),
		)?; if !matches!( version, FLAT_CHECKPOINT_FORMAT_VERSION | STRUCTURED_CHECKPOINT_FORMAT_VERSION
				| POOL_CHECKPOINT_FORMAT_VERSION
				| LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION
				| NATIVE_CHECKPOINT_FORMAT_VERSION
				| KMEANS_CHECKPOINT_FORMAT_VERSION
				| MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
				| TREE_CHECKPOINT_FORMAT_VERSION
				| EMBEDDING_CHECKPOINT_FORMAT_VERSION
				| RNN_CHECKPOINT_FORMAT_VERSION
				| GRU_CHECKPOINT_FORMAT_VERSION
				| LSTM_CHECKPOINT_FORMAT_VERSION ) { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
				root_path.field("version"),
				format!(
					"checkpoint version is {version}, expected {FLAT_CHECKPOINT_FORMAT_VERSION}, \
						 {STRUCTURED_CHECKPOINT_FORMAT_VERSION}, {POOL_CHECKPOINT_FORMAT_VERSION}, \
						 {LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION}, {NATIVE_CHECKPOINT_FORMAT_VERSION}, \
						 {KMEANS_CHECKPOINT_FORMAT_VERSION}, {MULTI_TARGET_CHECKPOINT_FORMAT_VERSION}, \
						 {TREE_CHECKPOINT_FORMAT_VERSION}, {EMBEDDING_CHECKPOINT_FORMAT_VERSION}, \
						 {RNN_CHECKPOINT_FORMAT_VERSION}, {GRU_CHECKPOINT_FORMAT_VERSION}, or \
						 {LSTM_CHECKPOINT_FORMAT_VERSION}"
				), )); }
		let expected_format = if is_semantic_model_version(version) { SEMANTIC_MODEL_FORMAT } else { LEGACY_CHECKPOINT_FORMAT
		}; if format != expected_format { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
				root_path.field("format"),
				format!("model format is {format:?}, expected {expected_format:?} for version {version}"),
			)); }
		let (loss, data_normalization) = self.parse_semantics(
			fields.require("semantics", &root_path)?,
			&root_path.field("semantics"),
		)?; let dataset = self.parse_dataset(
			fields.require("dataset", &root_path)?,
			&root_path.field("dataset"),
		)?; let (mut config, bounds) = self.parse_training(
			fields.require("training", &root_path)?,
			&root_path.field("training"),
			loss, data_normalization, version, )?; let model = self.parse_model(
			fields.require("model", &root_path)?,
			&root_path.field("model"),
			data_normalization, dataset.feature_width, dataset.task.output_width(), version, )?;
		if version == FLAT_CHECKPOINT_FORMAT_VERSION { config.layers = match model.output_adapter {
				Some(_) => model.layers[..model.layers.len().saturating_sub(1)] .iter() .map(|layer| layer.declaration.clone())
					.collect(), None => model .layers .iter() .map(|layer| layer.declaration.clone()) .collect(), }; }
		let native = match ( is_semantic_model_version(version),
			fields.optional("native"),
		) {
			(true, Some(node)) => Some(self.parse_native(node, &root_path.field("native"))?),
			(true, None) => { return Err(decode_error( CheckpointDecodeErrorKind::MissingField,
					root_path.field("native"),
					format!("v{version} semantic models require native realization metadata"),
				)); }
			(false, Some(_)) => { return Err(decode_error( CheckpointDecodeErrorKind::UnknownField,
					root_path.field("native"),
					"native realization metadata requires semantic-model version 8 or newer",
				)); }
			(false, None) => None, }; let artifact = CheckpointArtifact { format_version: version, vectors: dataset.vectors,
			feature_spans: dataset.feature_spans, feature_normalization_mask: dataset.feature_normalization_mask,
			feature_width: dataset.feature_width, target_source_indices: dataset.target_source_indices, task: dataset.task,
			output_adapter: model.output_adapter, config, bounds, normalization: model.normalization, layers: model.layers,
			blocks: model.blocks, temperature: model.temperature, native, }; validate_artifact(&artifact)?; Ok(artifact) }

	fn parse_semantics( &self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<(DenseLoss, DenseDataNormalization)> { let fields = self.fields( node, path,
			&["objective", "normalization", "optimizer"],
			&[], )?; let loss = self.parse_loss(
			self.scalar(fields.require("objective", path)?, &path.field("objective"))?,
			&path.field("objective"),
		)?; let normalization = self.parse_data_normalization(
			self.required_scalar(&fields, "normalization", path)?,
			&path.field("normalization"),
		)?; self.expect_scalar(
			fields.require("optimizer", path)?,
			&path.field("optimizer"),
			"adamw",
		)?; Ok((loss, normalization)) }

	fn parse_native( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<CheckpointNativeRealization> { let fields = self.fields( node, path,
			&["program", "realization", "topology", "discovery", "kernels"],
			&[], )?;
		let program = self.parse_digest_field(fields.require("program", path)?, &path.field("program"))?;
		let realization = RealizationIdentity::new(self.parse_digest_field(
			fields.require("realization", path)?,
			&path.field("realization"),
		)?); let topology = TopologyIdentity::new(
			self.parse_digest_field(fields.require("topology", path)?, &path.field("topology"))?,
		); let discovery = DiscoveryIdentity::new(
			self.parse_digest_field(fields.require("discovery", path)?, &path.field("discovery"))?,
		);
		let kernels_node = fields.require("kernels", path)?;
		let children = self
			.node(kernels_node, &path.field("kernels"))?
			.children() .to_vec(); if children.is_empty() { return Err(decode_error( CheckpointDecodeErrorKind::MissingField,
				path.field("kernels"),
				"native realization contains no kernels",
			)); }
		self.metadata_entries = self .metadata_entries .checked_add(children.len()) .ok_or_else(|| { decode_error(
					CheckpointDecodeErrorKind::LimitExceeded,
					path.field("kernels"),
					"native kernel count overflowed",
				) })?; if self.metadata_entries > self.limits.metadata_entries { return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.field("kernels"),
				"native kernel count exceeds the metadata-entry limit",
			)); }
		let mut kernels = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let kernel_path = path.field("kernels").index(index);
			if self.node(child, &kernel_path)?.text() != "kernel" {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, kernel_path,
					"native kernel list accepts only `kernel` entries",
				)); }
			let fields = self.fields( child, &kernel_path,
				&["format", "target", "toolchain", "digest"],
				&[], )?;
			let format = match self.required_scalar(&fields, "format", &kernel_path)? {
				"cubin" => NativeKernelFormat::Cubin,
				"hsaco" => NativeKernelFormat::Hsaco,
				value => { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
						kernel_path.field("format"),
						format!("unknown native kernel format {value:?}"),
					)); } };
			let target_path = kernel_path.field("target");
			let target_fields = self.fields(
				fields.require("target", &kernel_path)?,
				&target_path,
				&["backend", "architecture", "abi"],
				&[], )?; let target = TargetIdentity { backend: self.parse_label_field(
					target_fields.require("backend", &target_path)?,
					&target_path.field("backend"),
				)?, architecture: self.parse_label_field(
					target_fields.require("architecture", &target_path)?,
					&target_path.field("architecture"),
				)?, abi: self.parse_label_field(
					target_fields.require("abi", &target_path)?,
					&target_path.field("abi"),
				)?, };
			let toolchain_path = kernel_path.field("toolchain");
			let toolchain_fields = self.fields(
				fields.require("toolchain", &kernel_path)?,
				&toolchain_path,
				&["name", "version", "digest"],
				&[], )?; let toolchain = ToolchainIdentity { name: self.parse_label_field(
					toolchain_fields.require("name", &toolchain_path)?,
					&toolchain_path.field("name"),
				)?, version: self.parse_label_field(
					toolchain_fields.require("version", &toolchain_path)?,
					&toolchain_path.field("version"),
				)?, digest: self.parse_digest_field(
					toolchain_fields.require("digest", &toolchain_path)?,
					&toolchain_path.field("digest"),
				)?, }; kernels.push(CheckpointNativeKernel { format, target, toolchain, digest: self.parse_digest_field(
					fields.require("digest", &kernel_path)?,
					&kernel_path.field("digest"),
				)?, }); }
		Ok(CheckpointNativeRealization { program, realization, topology, discovery, kernels, }) }

	fn parse_label_field(&self, field: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Label> {
		let value = self.scalar(field, path)?; Label::new(value).map_err(|error| { decode_error(
				CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("invalid native identity label: {error}"),
			) }) }

	fn parse_digest_field(&self, field: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Digest> {
		let bytes = self.parse_hex_bytes(self.scalar(field, path)?, path)?;
		let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| { decode_error(
				CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("digest contains {} bytes, expected 32", bytes.len()),
			) })?; let digest = Digest::new(bytes); if digest.is_zero() { return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				"digest must not be zero",
			)); }
		Ok(digest) } }

impl Decoder<'_> {
	fn expect_scalar(&self, field: OgdlNodeId, path: &CheckpointPath, expected: &str) -> CheckpointResult<()> {
		let actual = self.scalar(field, path)?; if actual != expected { return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("value is {actual:?}, expected {expected:?}"),
			)); }
		Ok(()) }

	fn parse_unsigned<T>(&self, value: &str, path: &CheckpointPath, type_name: &str) -> CheckpointResult<T>
	where T: core::str::FromStr, T::Err: fmt::Display, { if value.is_empty()
			|| !value.bytes().all(|byte| byte.is_ascii_digit())
			|| value.len() > 1 && value.starts_with('0')
		{ return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("{value:?} is not a canonical unsigned {type_name}"),
			)); }
		value.parse::<T>().map_err(|error| { decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("{value:?} cannot be represented as {type_name}: {error}"),
			) }) }

	fn parse_signed<T>(&self, value: &str, path: &CheckpointPath, type_name: &str) -> CheckpointResult<T>
	where T: core::str::FromStr, T::Err: fmt::Display, {
		let digits = value.strip_prefix('-').unwrap_or(value);
		if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit())
			|| digits.len() > 1 && digits.starts_with('0')
			|| value == "-0"
		{ return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("{value:?} is not a canonical signed {type_name}"),
			)); }
		value.parse::<T>().map_err(|error| { decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("{value:?} cannot be represented as {type_name}: {error}"),
			) }) }

	fn parse_u8(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u8> { self.parse_unsigned(value, path, "u8") }

	fn parse_u32(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u32> { self.parse_unsigned(value, path, "u32") }

	fn parse_u64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u64> { self.parse_unsigned(value, path, "u64") }

	fn parse_usize(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<usize> { self.parse_unsigned(value, path, "usize") }

	fn parse_i32(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<i32> { self.parse_signed(value, path, "i32") }

	fn parse_i64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<i64> { self.parse_signed(value, path, "i64") }

	fn parse_nonzero_u64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<NonZeroU64> {
		NonZeroU64::new(self.parse_u64(value, path)?).ok_or_else(|| { decode_error( CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				"value must be nonzero",
			) }) }

	fn parse_f32_bits(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<f32> { if value.len() != 10
			|| !value.starts_with("0x")
			|| !value[2..] .bytes()
				.all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
		{ return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("{value:?} is not one canonical 32-bit lowercase hexadecimal value"),
			)); }
		let bits = u32::from_str_radix(&value[2..], 16).map_err(|error| { decode_error(
				CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("invalid f32 bit pattern: {error}"),
			) })?; Ok(f32::from_bits(bits)) }

	fn parse_hex_bytes(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<Vec<u8>> {
		let Some(hex) = value.strip_prefix("0x") else {
			return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				"byte string must start with `0x`",
			)); }; if hex.len() % 2 != 0
			|| !hex .bytes()
				.all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
		{ return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
				format!("{value:?} is not canonical lowercase byte hexadecimal"),
			)); }
		let mut bytes = Vec::with_capacity(hex.len() / 2); for pair in hex.as_bytes().chunks_exact(2) {
			let pair = core::str::from_utf8(pair).expect("hexadecimal digits are ASCII");
			bytes.push(u8::from_str_radix(pair, 16).expect("validated hexadecimal pair"));
		}
		Ok(bytes) }

	fn parse_loss(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<DenseLoss> { match value {
			"binary-cross-entropy-with-logits" => Ok(DenseLoss::BinaryCrossEntropy),
			"binary-focal-with-logits-alpha-0.25-gamma-2" => Ok(DenseLoss::Focal),
			"mean-squared-error" => Ok(DenseLoss::MeanSquaredError),
			"mean-absolute-error" => Ok(DenseLoss::MeanAbsoluteError),
			"cross-entropy" => Ok(DenseLoss::CrossEntropy),
			"huber-unit-delta" => Ok(DenseLoss::Huber),
			_ => Err(self.invalid_enum(path, "objective", value)),
		} }

	fn parse_data_normalization( &self, value: &str, path: &CheckpointPath, ) -> CheckpointResult<DenseDataNormalization> {
		match value {
			"identity" => Ok(DenseDataNormalization::Identity),
			"z-score" => Ok(DenseDataNormalization::ZScore),
			"min-max" => Ok(DenseDataNormalization::MinMax),
			"l2-norm" => Ok(DenseDataNormalization::L2Norm),
			_ => Err(self.invalid_enum(path, "data normalization", value)),
		} }

	fn parse_learning_rate_decay(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<LearningRateDecay> {
		match value {
			"constant" => Ok(LearningRateDecay::Constant),
			"linear" => Ok(LearningRateDecay::Linear),
			"cosine" => Ok(LearningRateDecay::Cosine),
			"exponential" => Ok(LearningRateDecay::Exponential),
			_ => Err(self.invalid_enum(path, "learning-rate decay", value)),
		} }

	fn invalid_enum(&self, path: &CheckpointPath, description: &str, value: &str) -> CheckpointError { decode_error(
			CheckpointDecodeErrorKind::InvalidValue, path.clone(),
			format!("unknown {description} {value:?}"),
		) } }

impl Decoder<'_> {
	fn parse_dataset(&mut self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<ParsedDataset> {
		let fields = self.fields( node, path, &[
				"feature-width",
				"target",
				"vectors",
				"feature-spans",
				"feature-normalization-mask",
			], &[], )?; let feature_width = self.parse_usize(
			self.required_scalar(&fields, "feature-width", path)?,
			&path.field("feature-width"),
		)?;
		let target = self.parse_target(fields.require("target", path)?, &path.field("target"))?;
		let vectors = self.parse_vectors(fields.require("vectors", path)?, &path.field("vectors"))?;
		let feature_spans = self.parse_feature_spans(
			fields.require("feature-spans", path)?,
			&path.field("feature-spans"),
		)?; let feature_normalization_mask = self.parse_feature_normalization_mask(
			fields.require("feature-normalization-mask", path)?,
			&path.field("feature-normalization-mask"),
		)?; Ok(ParsedDataset { vectors, feature_spans, feature_normalization_mask, feature_width,
			target_source_indices: target.source_indices, task: target.task, }) }

	fn parse_target(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<ParsedTarget> {
		let fields = self.fields( node, path,
			&["task"],
			&[
				"source-index",
				"source-indices",
				"positive-code",
				"class-count",
				"reserved-unseen-code",
			], )?;
		let task = self.scalar(fields.require("task", path)?, &path.field("task"))?;
		let parse_source_index = || { self.parse_usize(
				self.required_scalar(&fields, "source-index", path)?,
				&path.field("source-index"),
			) }; match task {
			"binary-classification" => {
				self.reject_fields( &fields, path,
					&["source-indices", "class-count", "reserved-unseen-code"],
				)?; let source_index = parse_source_index()?; let positive_code = self.parse_i32(
					self.required_scalar(&fields, "positive-code", path)?,
					&path.field("positive-code"),
				)?; Ok(ParsedTarget { source_indices: vec![source_index], task: DenseTask::BinaryClassification {
						target_vector: source_index, positive_code, }, }) }
			"multiclass-classification" => {
				self.reject_fields(&fields, path, &["source-indices", "positive-code"])?;
				let source_index = parse_source_index()?; let class_count = self.parse_usize(
					self.required_scalar(&fields, "class-count", path)?,
					&path.field("class-count"),
				)?; let reserved_code = self.parse_i32(
					self.required_scalar(&fields, "reserved-unseen-code", path)?,
					&path.field("reserved-unseen-code"),
				)?; Ok(ParsedTarget { source_indices: vec![source_index], task: DenseTask::MulticlassClassification {
						target_vector: source_index, class_count, reserved_code, }, }) }
			"scalar-regression" => {
				self.reject_fields( &fields, path, &[
						"source-indices",
						"positive-code",
						"class-count",
						"reserved-unseen-code",
					], )?; let source_index = parse_source_index()?; Ok(ParsedTarget { source_indices: vec![source_index],
					task: DenseTask::ScalarRegression { target_vector: source_index, }, }) }
			"multi-target-binary-classification"
			| "joint-multiclass-classification"
			| "multi-target-regression" => {
				self.reject_fields( &fields, path, &[
						"source-index",
						"positive-code",
						"class-count",
						"reserved-unseen-code",
					], )?;
				let source_indices_node = fields.require("source-indices", path)?;
				let children = self
					.node(source_indices_node, &path.field("source-indices"))?
					.children(); if children.len() > self.limits.vectors { return Err(decode_error(
						CheckpointDecodeErrorKind::LimitExceeded,
						path.field("source-indices"),
						format!(
							"target source-index count is {}, limit is {}",
							children.len(), self.limits.vectors ), )); }
				let mut source_indices = Vec::with_capacity(children.len());
				for (index, child) in children.iter().copied().enumerate() {
					let child_path = path.field("source-indices").index(index);
					let node = self.node(child, &child_path)?; if !node.children().is_empty() { return Err(decode_error(
							CheckpointDecodeErrorKind::InvalidValue, child_path,
							"target source index must be one scalar",
						)); }
					source_indices.push(self.parse_usize(node.text(), &child_path)?); }
				if source_indices.len() < 2 { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
						path.field("source-indices"),
						"multi-target task requires at least two source indices",
					)); }
				let first_target_vector = source_indices[0]; let target_count = source_indices.len(); let task = match task {
					"multi-target-binary-classification" => DenseTask::MultiTargetBinaryClassification {
						first_target_vector, target_count, },
					"joint-multiclass-classification" => DenseTask::JointMulticlassClassification {
						first_target_vector, target_count, },
					"multi-target-regression" => DenseTask::MultiTargetRegression {
						first_target_vector, target_count, }, _ => unreachable!(), }; Ok(ParsedTarget { source_indices, task, }) }
			_ => Err(self.invalid_enum(&path.field("task"), "dense task", task)),
		} }

	fn reject_fields(&self, fields: &FieldSet, path: &CheckpointPath, names: &[&str]) -> CheckpointResult<()> {
		if let Some(name) = names.iter().find(|name| fields.optional(name).is_some()) { return Err(decode_error(
				CheckpointDecodeErrorKind::UnknownField, path.field(*name),
				format!("field {name:?} is not valid for this tagged variant"),
			)); }
		Ok(()) }

	fn parse_vectors( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointArtifactVector>> { let children = self.node(node, path)?.children().to_vec();
		if children.len() > self.limits.vectors { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(), format!(
					"vector count is {}, limit is {}",
					children.len(), self.limits.vectors ), )); }
		let mut vectors = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let vector_path = path.index(index);
			if self.node(child, &vector_path)?.text() != "vector" {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, vector_path, format!(
						"expected `vector`, found {:?}",
						self.node(child, path)?.text() ), )); }
			vectors.push(self.parse_vector(child, &path.index(index))?); }
		Ok(vectors) }

	fn parse_vector( &mut self, node: OgdlNodeId, path: &CheckpointPath, ) -> CheckpointResult<CheckpointArtifactVector> {
		let fields = self.fields( node, path, &[
				"source-index",
				"name-bytes",
				"role",
				"semantic-type",
				"encoding",
				"metadata",
			], &[], )?; let source_index = self.parse_usize(
			self.required_scalar(&fields, "source-index", path)?,
			&path.field("source-index"),
		)?; let name = self.parse_hex_bytes(
			self.required_scalar(&fields, "name-bytes", path)?,
			&path.field("name-bytes"),
		)?; let role = self.parse_vector_role(
			self.scalar(fields.require("role", path)?, &path.field("role"))?,
			&path.field("role"),
		)?; let semantic_type = self.parse_semantic_type(
			self.required_scalar(&fields, "semantic-type", path)?,
			&path.field("semantic-type"),
		)?; let encoding = self.parse_vector_encoding(
			self.scalar(fields.require("encoding", path)?, &path.field("encoding"))?,
			&path.field("encoding"),
		)?;
		let metadata = self.parse_vector_metadata(fields.require("metadata", path)?, &path.field("metadata"))?;
		Ok(CheckpointArtifactVector { source_index, name, role, semantic_type, encoding, metadata, }) }

	fn parse_vector_role(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<VectorRole> { match value {
			"feature" => Ok(VectorRole::Feature),
			"target" => Ok(VectorRole::Target),
			_ => Err(self.invalid_enum(path, "vector role", value)),
		} }

	fn parse_semantic_type(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<SemanticType> { match value {
			"numeric" => Ok(SemanticType::Numeric),
			"temporal" => Ok(SemanticType::Temporal),
			"categorical" => Ok(SemanticType::Categorical),
			"ordinal" => Ok(SemanticType::Ordinal),
			"text" => Ok(SemanticType::Text),
			"image" => Ok(SemanticType::Image),
			"binary" => Ok(SemanticType::Binary),
			_ => Err(self.invalid_enum(path, "semantic type", value)),
		} }

	fn parse_vector_encoding(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<VectorEncoding> { match value {
			"f32" => Ok(VectorEncoding::F32),
			"int32" => Ok(VectorEncoding::I32),
			"relative-seconds-int32" => Ok(VectorEncoding::RelativeSecondsI32),
			"dictionary-int32" => Ok(VectorEncoding::DictionaryI32),
			"ordinal-int32" => Ok(VectorEncoding::OrdinalI32),
			"utf8" => Ok(VectorEncoding::Utf8),
			"bytes" => Ok(VectorEncoding::Bytes),
			_ => Err(self.invalid_enum(path, "vector encoding", value)),
		} }

	fn parse_vector_metadata( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<CheckpointArtifactMetadata> { let (tag, payload) = self.tagged(node, path)?;
		let tag = tag.to_owned(); let payload = payload.to_vec(); match tag.as_str() {
			"none" => {
				self.require_empty(&payload, path)?; Ok(CheckpointArtifactMetadata::None) }
			"temporal" => {
				let fields =
					self.fields_from_children(&payload, path, &["unix-seconds", "nanoseconds"], &[])?;
				let unix_seconds = self.parse_i64(
					self.required_scalar(&fields, "unix-seconds", path)?,
					&path.field("unix-seconds"),
				)?; let nanoseconds = self.parse_u32(
					self.required_scalar(&fields, "nanoseconds", path)?,
					&path.field("nanoseconds"),
				)?; Ok(CheckpointArtifactMetadata::Temporal { unix_seconds, nanoseconds, }) }
			"categorical" => Ok(CheckpointArtifactMetadata::Categorical {
				dictionary: self.parse_byte_entries(&payload, path, "value-bytes")?,
			}),
			"ordinal" => Ok(CheckpointArtifactMetadata::Ordinal {
				ordered_labels: self.parse_byte_entries(&payload, path, "value-bytes")?,
			}),
			"image" => Ok(CheckpointArtifactMetadata::Image {
				encoded_variants: self.parse_image_variants(&payload, path)?, }),
			_ => Err(self.invalid_enum(path, "vector metadata", &tag)),
		} }

	fn require_empty(&self, children: &[OgdlNodeId], path: &CheckpointPath) -> CheckpointResult<()> {
		if let Some(child) = children.first() { let name = self.node(*child, path)?.text(); return Err(decode_error(
				CheckpointDecodeErrorKind::UnknownField, path.field(name),
				format!("tagged value has unexpected field {name:?}"),
			)); }
		Ok(()) }

	fn reserve_metadata_entries(&mut self, added: usize, path: &CheckpointPath) -> CheckpointResult<()> {
		self.metadata_entries = self.metadata_entries.checked_add(added).ok_or_else(|| { decode_error(
				CheckpointDecodeErrorKind::LimitExceeded, path.clone(),
				"metadata entry count overflowed usize",
			) })?; if self.metadata_entries > self.limits.metadata_entries { return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded, path.clone(), format!(
					"metadata entry count is {}, limit is {}",
					self.metadata_entries, self.limits.metadata_entries ), )); }
		Ok(()) }

	fn parse_byte_entries( &mut self, children: &[OgdlNodeId], path: &CheckpointPath, expected_name: &str,
	) -> CheckpointResult<Vec<Vec<u8>>> { self.reserve_metadata_entries(children.len(), path)?;
		let mut entries = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let entry_path = path.index(index); let node = self.node(child, &entry_path)?; if node.text() != expected_name {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, entry_path,
					format!("expected {expected_name:?}, found {:?}", node.text()),
				)); }
			entries.push(self.parse_hex_bytes(self.scalar(child, &path.index(index))?, &path.index(index))?); }
		Ok(entries) }

	fn parse_image_variants( &mut self, children: &[OgdlNodeId], path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointImageMetadata>> { self.reserve_metadata_entries(children.len(), path)?;
		let mut variants = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let variant_path = path.index(index);
			if self.node(child, &variant_path)?.text() != "variant" {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, variant_path, format!(
						"expected `variant`, found {:?}",
						self.node(child, path)?.text() ), )); }
			let fields = self.fields( child, &variant_path, &[
					"format",
					"width",
					"height",
					"channels",
					"color-model",
					"sample-bits",
					"value-layout",
					"value-range",
				], &[], )?;
			let format_path = variant_path.field("format");
			let format = self.parse_image_format(
				self.scalar(fields.require("format", &variant_path)?, &format_path)?,
				&format_path, )?; let width = self.parse_u32(
				self.required_scalar(&fields, "width", &variant_path)?,
				&variant_path.field("width"),
			)?; let height = self.parse_u32(
				self.required_scalar(&fields, "height", &variant_path)?,
				&variant_path.field("height"),
			)?; let channels = self.parse_optional_u8(
				self.required_scalar(&fields, "channels", &variant_path)?,
				&variant_path.field("channels"),
			)?; let color_model = self.parse_optional_image_color_model(
				self.required_scalar(&fields, "color-model", &variant_path)?,
				&variant_path.field("color-model"),
			)?; let sample_bits = self.parse_optional_u8(
				self.required_scalar(&fields, "sample-bits", &variant_path)?,
				&variant_path.field("sample-bits"),
			)?; self.expect_scalar(
				fields.require("value-layout", &variant_path)?,
				&variant_path.field("value-layout"),
				"encoded-file",
			)?; self.expect_scalar(
				fields.require("value-range", &variant_path)?,
				&variant_path.field("value-range"),
				"encoded-bytes",
			)?; variants.push(CheckpointImageMetadata { format, width, height, channels, color_model, sample_bits,
				value_layout: ImageValueLayout::EncodedFile, value_range: ImageValueRange::EncodedBytes, }); }
		Ok(variants) }

	fn parse_optional_u8(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<Option<u8>> {
		if value == "none" {
			Ok(None) } else { self.parse_u8(value, path).map(Some) } }

	fn parse_image_format(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<EncodedImageFormat> {
		match value {
			"png" => Ok(EncodedImageFormat::Png),
			"jpeg" => Ok(EncodedImageFormat::Jpeg),
			"gif87a" => Ok(EncodedImageFormat::Gif87a),
			"gif89a" => Ok(EncodedImageFormat::Gif89a),
			"bmp" => Ok(EncodedImageFormat::Bmp),
			"webp" => Ok(EncodedImageFormat::WebP),
			_ => Err(self.invalid_enum(path, "encoded image format", value)),
		} }

	fn parse_optional_image_color_model( &self, value: &str, path: &CheckpointPath,
	) -> CheckpointResult<Option<ImageColorModel>> { let model = match value {
			"none" => return Ok(None),
			"grayscale" => ImageColorModel::Grayscale,
			"grayscale-alpha" => ImageColorModel::GrayscaleAlpha,
			"rgb" => ImageColorModel::Rgb,
			"rgba" => ImageColorModel::Rgba,
			"bgr" => ImageColorModel::Bgr,
			"indexed-rgb" => ImageColorModel::IndexedRgb,
			"y-cb-cr" => ImageColorModel::YCbCr,
			"cmyk" => ImageColorModel::Cmyk,
			"ycck" => ImageColorModel::Ycck,
			_ => return Err(self.invalid_enum(path, "image color model", value)),
		}; Ok(Some(model)) }

	fn parse_feature_spans( &self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<Vec<CompiledFeatureSpan>> { let children = self.node(node, path)?.children();
		if children.len() > self.limits.feature_spans { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(), format!(
					"feature-span count is {}, limit is {}",
					children.len(), self.limits.feature_spans ), )); }
		let mut spans = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let span_path = path.index(index);
			if self.node(child, &span_path)?.text() != "span" {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, span_path, format!(
						"expected `span`, found {:?}",
						self.node(child, path)?.text() ), )); }
			let fields = self.fields( child, &span_path,
				&["source-index", "start", "width", "lowering"],
				&[], )?; let source_vector = self.parse_usize(
				self.required_scalar(&fields, "source-index", &span_path)?,
				&span_path.field("source-index"),
			)?; let start = self.parse_usize(
				self.required_scalar(&fields, "start", &span_path)?,
				&span_path.field("start"),
			)?; let width = self.parse_usize(
				self.required_scalar(&fields, "width", &span_path)?,
				&span_path.field("width"),
			)?; let lowering = self.parse_feature_lowering(
				fields.require("lowering", &span_path)?,
				&span_path.field("lowering"),
			)?; spans.push(CompiledFeatureSpan::new( source_vector, start, width, lowering, )); }
		Ok(spans) }

	fn parse_feature_lowering( &self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<DenseFeatureLowering> { let (tag, payload) = self.tagged(node, path)?; match tag {
			"numeric-scalar" => {
				self.require_empty(payload, path)?; Ok(DenseFeatureLowering::NumericScalar) }
			"categorical-one-hot" => {
				let fields =
					self.fields_from_children(payload, path, &["dictionary-width", "reserved-index"], &[])?;
				let dictionary_width = self.parse_usize(
					self.required_scalar(&fields, "dictionary-width", path)?,
					&path.field("dictionary-width"),
				)?; let reserved_index = self.parse_usize(
					self.required_scalar(&fields, "reserved-index", path)?,
					&path.field("reserved-index"),
				)?; Ok(DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, }) }
			_ => Err(self.invalid_enum(path, "feature lowering", tag)),
		} }

	fn parse_feature_normalization_mask( &self, node: OgdlNodeId, path: &CheckpointPath, ) -> CheckpointResult<Vec<u32>> {
		let children = self.node(node, path)?.children(); let mut mask = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() { let value_path = path.index(index);
			if self.node(child, &value_path)?.text() != "value-bits" {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, value_path, format!(
						"expected `value-bits`, found {:?}",
						self.node(child, path)?.text() ), )); }
			mask.push(self .parse_f32_bits(self.scalar(child, &path.index(index))?, &path.index(index))? .to_bits()); }
		Ok(mask) } }

impl Decoder<'_> {
	fn parse_training( &self, node: OgdlNodeId, path: &CheckpointPath, loss: DenseLoss,
		data_normalization: DenseDataNormalization, format_version: u32,
	) -> CheckpointResult<(DenseTrainingConfig, TrainingBounds)> { let fields = self.fields( node, path, &[
				"epochs",
				"warmup-epochs",
				"learning-rate-decay",
				"gradient-clip-norm",
				"normalization-epsilon",
				"reduction-tree-lanes",
				"random-seed",
				"adamw",
				"bounds",
			], &[], )?;
		let epochs_value = self.scalar(fields.require("epochs", path)?, &path.field("epochs"))?;
		let epochs = self.parse_training_horizon(epochs_value, &path.field("epochs"), format_version)?;
		let warmup_epochs = self.parse_u64(
			self.required_scalar(&fields, "warmup-epochs", path)?,
			&path.field("warmup-epochs"),
		)?; let learning_rate_decay = self.parse_learning_rate_decay(
			self.required_scalar(&fields, "learning-rate-decay", path)?,
			&path.field("learning-rate-decay"),
		)?;
		let gradient_clip_value = self.required_scalar(&fields, "gradient-clip-norm", path)?;
		let gradient_clip_norm = if gradient_clip_value == "none" {
			if !is_semantic_model_version(format_version) { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
					path.field("gradient-clip-norm"),
					"optional gradient clipping requires semantic-model version 8 or newer",
				)); }
			None } else {
			Some(self.parse_f32_bits(gradient_clip_value, &path.field("gradient-clip-norm"))?)
		}; let normalization_epsilon = self.parse_f32_bits(
			self.required_scalar(&fields, "normalization-epsilon", path)?,
			&path.field("normalization-epsilon"),
		)?; let reduction_tree_lanes = self.parse_u32(
			self.required_scalar(&fields, "reduction-tree-lanes", path)?,
			&path.field("reduction-tree-lanes"),
		)?; let random_seed = self.parse_u64(
			self.required_scalar(&fields, "random-seed", path)?,
			&path.field("random-seed"),
		)?;
		let adamw = self.parse_adamw(fields.require("adamw", path)?, &path.field("adamw"))?;
		let bounds = self.parse_bounds(
			fields.require("bounds", path)?,
			&path.field("bounds"),
			format_version, )?; Ok(( DenseTrainingConfig { layers: Vec::new(), loss, data_normalization, epochs, warmup_epochs,
				learning_rate_decay, gradient_clip_norm, normalization_epsilon, reduction_tree_lanes, random_seed, adamw, }, bounds,
		)) }

	fn parse_adamw(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<AdamWConfig> {
		let fields = self.fields( node, path, &[
				"learning-rate",
				"beta-one",
				"beta-two",
				"epsilon",
				"weight-decay",
			], &[], )?; Ok(AdamWConfig { learning_rate: self.parse_f32_bits(
				self.required_scalar(&fields, "learning-rate", path)?,
				&path.field("learning-rate"),
			)?, beta_one: self.parse_f32_bits(
				self.scalar(fields.require("beta-one", path)?, &path.field("beta-one"))?,
				&path.field("beta-one"),
			)?, beta_two: self.parse_f32_bits(
				self.scalar(fields.require("beta-two", path)?, &path.field("beta-two"))?,
				&path.field("beta-two"),
			)?, epsilon: self.parse_f32_bits(
				self.scalar(fields.require("epsilon", path)?, &path.field("epsilon"))?,
				&path.field("epsilon"),
			)?, weight_decay: self.parse_f32_bits(
				self.required_scalar(&fields, "weight-decay", path)?,
				&path.field("weight-decay"),
			)?, }) }

	fn parse_bounds( &self, node: OgdlNodeId, path: &CheckpointPath, format_version: u32,
	) -> CheckpointResult<TrainingBounds> { let fields = self.fields( node, path, &[
				"train-rows",
				"epochs",
				"training-iterations",
				"calibration-iterations",
				"iterations",
				"warmup-iterations",
			], &[], )?; let u64_field = |name: &str| -> CheckpointResult<u64> { self.parse_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) }; Ok(TrainingBounds {
			train_rows: u64_field("train-rows")?,
			epochs: self.parse_training_horizon(
				self.scalar(fields.require("epochs", path)?, &path.field("epochs"))?,
				&path.field("epochs"),
				format_version, )?, training_iterations: self.parse_loop_iterations(
				self.required_scalar(&fields, "training-iterations", path)?,
				&path.field("training-iterations"),
				format_version, )?,
			calibration_iterations: u64_field("calibration-iterations")?,
			iterations: self.parse_loop_iterations(
				self.required_scalar(&fields, "iterations", path)?,
				&path.field("iterations"),
				format_version, )?,
			warmup_iterations: u64_field("warmup-iterations")?,
		}) }

	fn parse_training_horizon( &self, value: &str, path: &CheckpointPath, format_version: u32,
	) -> CheckpointResult<TrainingHorizon> {
		if value == "unbounded" {
			if !is_semantic_model_version(format_version) { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
					path.clone(),
					"unbounded training requires semantic-model version 8 or newer",
				)); }
			return Ok(TrainingHorizon::Unbounded); }
		self.parse_nonzero_u64(value, path) .map(TrainingHorizon::Finite) }

	fn parse_loop_iterations( &self, value: &str, path: &CheckpointPath, format_version: u32,
	) -> CheckpointResult<LoopIterations> {
		if value == "unbounded" {
			if !is_semantic_model_version(format_version) { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
					path.clone(),
					"unbounded loop bounds require semantic-model version 8 or newer",
				)); }
			return Ok(LoopIterations::Unbounded); }
		self.parse_nonzero_u64(value, path) .map(LoopIterations::Finite) } }

impl Decoder<'_> {
	fn parse_model( &mut self, node: OgdlNodeId, path: &CheckpointPath, data_normalization: DenseDataNormalization,
		expected_input_width: usize, expected_output_width: usize, format_version: u32, ) -> CheckpointResult<ParsedModel> {
		let fields = self.fields( node, path,
			&["input-width", "output-width", "normalization", "blocks"],
			&["output-adapter", "calibration"],
		)?; let input_width = self.parse_usize(
			self.required_scalar(&fields, "input-width", path)?,
			&path.field("input-width"),
		)?; if input_width != expected_input_width { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("input-width"),
				format!("model input width is {input_width}, dataset feature width is {expected_input_width}"),
			)); }
		let output_width = self.parse_usize(
			self.required_scalar(&fields, "output-width", path)?,
			&path.field("output-width"),
		)?; if output_width != expected_output_width { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("output-width"),
				format!("model output width is {output_width}, task output width is {expected_output_width}"),
			)); }
		let output_adapter = fields
			.optional("output-adapter")
			.map(|adapter| self.parse_output_adapter(adapter, &path.field("output-adapter")))
			.transpose()?; let normalization = self.parse_model_normalization(
			fields.require("normalization", path)?,
			&path.field("normalization"),
			data_normalization, )?; let blocks = self.parse_blocks(
			fields.require("blocks", path)?,
			&path.field("blocks"),
			format_version, )?; let layers = if format_version == FLAT_CHECKPOINT_FORMAT_VERSION { blocks.iter()
				.map(|block| match block { CheckpointBlockImage::Layer(layer) => Ok(layer.clone()),
					CheckpointBlockImage::Embedding(_)
					| CheckpointBlockImage::Attention(_)
					| CheckpointBlockImage::Rnn(_)
					| CheckpointBlockImage::Gru(_)
					| CheckpointBlockImage::Lstm(_)
					| CheckpointBlockImage::Convolution(_)
					| CheckpointBlockImage::Pool(_)
					| CheckpointBlockImage::KMeans(_)
					| CheckpointBlockImage::Tree(_)
					| CheckpointBlockImage::Residual(_) => Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
						path.field("blocks"),
						"checkpoint v5 cannot contain a structured block",
					)), }) .collect::<CheckpointResult<Vec<_>>>()? } else { Vec::new() }; let temperature = fields
			.optional("calibration")
			.map(|calibration| self.parse_calibration(calibration, &path.field("calibration")))
			.transpose()? .flatten(); Ok(ParsedModel { output_adapter, normalization, layers, blocks, temperature, }) }

	fn parse_output_adapter(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<DenseOutputAdapter> {
		let (tag, payload) = self.tagged(node, path)?;
		if tag != "linear-projection" {
			return Err(self.invalid_enum(path, "output adapter", tag));
		}
		let fields = self.fields_from_children(payload, path, &["source-width", "target-width"], &[])?;
		let source_width = self.parse_nonzero_u64(
			self.required_scalar(&fields, "source-width", path)?,
			&path.field("source-width"),
		)?; let target_width = self.parse_nonzero_u64(
			self.required_scalar(&fields, "target-width", path)?,
			&path.field("target-width"),
		)?; Ok(DenseOutputAdapter::new(source_width, target_width)) }

	fn parse_model_normalization( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected: DenseDataNormalization,
	) -> CheckpointResult<Vec<CheckpointTensorImage>> { let (tag, payload) = self.tagged(node, path)?;
		let tag = tag.to_owned(); let payload = payload.to_vec(); let actual = self.parse_data_normalization(&tag, path)?;
		if actual != expected { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue, path.clone(), format!(
					"model normalization {tag:?} differs from semantic normalization {:?}",
					data_normalization(expected) ), )); }
		let names: &[&str] = match actual { DenseDataNormalization::Identity => &[],
			DenseDataNormalization::ZScore => &["mean", "variance"],
			DenseDataNormalization::MinMax => &["minimum", "maximum"],
			DenseDataNormalization::L2Norm => &[], }; let fields = self.fields_from_children(&payload, path, names, &[])?;
		let mut tensors = Vec::with_capacity(names.len()); for name in names {
			tensors.push(self.parse_tensor(fields.require(name, path)?, &path.field(*name))?); }
		Ok(tensors) }

	fn parse_blocks( &mut self, node: OgdlNodeId, path: &CheckpointPath, format_version: u32,
	) -> CheckpointResult<Vec<CheckpointBlockImage>> { let children = self.node(node, path)?.children().to_vec();
		if children.len() > self.limits.layers { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(), format!(
					"block count is {}, limit is {}",
					children.len(), self.limits.layers ), )); }
		let mut blocks = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let block_path = path.index(index); self.claim_block(&block_path)?;
			let block = match self.node(child, &block_path)?.text() {
				"embedding"
					if matches!( format_version, EMBEDDING_CHECKPOINT_FORMAT_VERSION
							| RNN_CHECKPOINT_FORMAT_VERSION | GRU_CHECKPOINT_FORMAT_VERSION
							| LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Embedding(self.parse_embedding(child, &block_path, index)?) }
				"embedding" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain an embedding block"),
					)); }
				"attention"
					if matches!( format_version, EMBEDDING_CHECKPOINT_FORMAT_VERSION
							| RNN_CHECKPOINT_FORMAT_VERSION | GRU_CHECKPOINT_FORMAT_VERSION
							| LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Attention(self.parse_attention(child, &block_path, index)?) }
				"attention" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain an attention block"),
					)); }
				"rnn" if matches!(
					format_version, RNN_CHECKPOINT_FORMAT_VERSION | GRU_CHECKPOINT_FORMAT_VERSION | LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Rnn(self.parse_rnn(child, &block_path, index)?) }
				"rnn" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain an RNN block"),
					)); }
				"gru" if matches!(
					format_version, GRU_CHECKPOINT_FORMAT_VERSION | LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Gru(self.parse_gru(child, &block_path, index)?) }
				"gru" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain a GRU block"),
					)); }
				"lstm" if format_version == LSTM_CHECKPOINT_FORMAT_VERSION => {
					CheckpointBlockImage::Lstm(self.parse_lstm(child, &block_path, index)?) }
				"lstm" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain an LSTM block"),
					)); }
				"layer" | "perc" => CheckpointBlockImage::Layer(self.parse_layer(child, &block_path, index)?),
				"convolution"
					if matches!( format_version, NATIVE_CHECKPOINT_FORMAT_VERSION
							| KMEANS_CHECKPOINT_FORMAT_VERSION | MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
							| TREE_CHECKPOINT_FORMAT_VERSION | EMBEDDING_CHECKPOINT_FORMAT_VERSION
							| RNN_CHECKPOINT_FORMAT_VERSION | GRU_CHECKPOINT_FORMAT_VERSION
							| LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Convolution(self.parse_convolution(child, &block_path, index)?) }
				"convolution" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain a convolution block"),
					)); }
				"pool" if matches!(
					format_version, POOL_CHECKPOINT_FORMAT_VERSION | LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION
						| NATIVE_CHECKPOINT_FORMAT_VERSION
						| KMEANS_CHECKPOINT_FORMAT_VERSION
						| MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
						| TREE_CHECKPOINT_FORMAT_VERSION
						| EMBEDDING_CHECKPOINT_FORMAT_VERSION
						| RNN_CHECKPOINT_FORMAT_VERSION
						| GRU_CHECKPOINT_FORMAT_VERSION
						| LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Pool(self.parse_pool(child, &block_path, index)?) }
				"pool" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain a pool block"),
					)); }
				"kmeans"
					if matches!( format_version, KMEANS_CHECKPOINT_FORMAT_VERSION | MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
							| TREE_CHECKPOINT_FORMAT_VERSION | EMBEDDING_CHECKPOINT_FORMAT_VERSION
							| RNN_CHECKPOINT_FORMAT_VERSION | GRU_CHECKPOINT_FORMAT_VERSION
							| LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::KMeans(self.parse_kmeans(child, &block_path, index)?) }
				"kmeans" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain a K-means block"),
					)); }
				"tree" if matches!(
					format_version, TREE_CHECKPOINT_FORMAT_VERSION | EMBEDDING_CHECKPOINT_FORMAT_VERSION
						| RNN_CHECKPOINT_FORMAT_VERSION
						| GRU_CHECKPOINT_FORMAT_VERSION
						| LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Tree(self.parse_tree(child, &block_path, index)?) }
				"tree" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain a supervised tree block"),
					)); }
				"residual"
					if matches!( format_version, STRUCTURED_CHECKPOINT_FORMAT_VERSION
							| POOL_CHECKPOINT_FORMAT_VERSION | LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION
							| NATIVE_CHECKPOINT_FORMAT_VERSION | KMEANS_CHECKPOINT_FORMAT_VERSION
							| MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
							| TREE_CHECKPOINT_FORMAT_VERSION | EMBEDDING_CHECKPOINT_FORMAT_VERSION
							| RNN_CHECKPOINT_FORMAT_VERSION | GRU_CHECKPOINT_FORMAT_VERSION
							| LSTM_CHECKPOINT_FORMAT_VERSION ) =>
				{ CheckpointBlockImage::Residual(self.parse_residual(child, &block_path, index)?) }
				"residual" => {
					return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, block_path,
						format!("checkpoint v{format_version} cannot contain a residual block"),
					)); }
				value => return Err(self.invalid_enum(&block_path, "dense block", value)),
			}; blocks.push(block); }
		Ok(blocks) }

	fn claim_block(&mut self, path: &CheckpointPath) -> CheckpointResult<()> {
		self.block_count = self.block_count.checked_add(1).ok_or_else(|| { decode_error(
				CheckpointDecodeErrorKind::LimitExceeded, path.clone(),
				"block and branch-step count overflowed usize",
			) })?; if self.block_count > self.limits.layers { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(), format!(
					"block and branch-step count is {}, limit is {}",
					self.block_count, self.limits.layers ), )); }
		Ok(()) }

	fn parse_embedding( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointEmbeddingImage> { let fields = self.fields( node, path, &[
				"index",
				"dimensions",
				"vocabulary",
				"sequence-length",
				"table",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let parse_extent = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) };
		let dimensions = parse_extent("dimensions")?;
		let vocabulary = parse_extent("vocabulary")?;
		let sequence_length = parse_extent("sequence-length")?;
		let table = self.parse_parameter(fields.require("table", path)?, &path.field("table"))?;
		Ok(CheckpointEmbeddingImage { dimensions, vocabulary, sequence_length, table, }) }

	fn parse_attention( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointAttentionImage> { let fields = self.fields( node, path, &[
				"index",
				"sequence-length",
				"dimensions",
				"heads",
				"head-dimension",
				"query",
				"key",
				"value",
				"output",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let parse_extent = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) }; Ok(CheckpointAttentionImage {
			sequence_length: parse_extent("sequence-length")?,
			dimensions: parse_extent("dimensions")?,
			heads: parse_extent("heads")?,
			head_dimension: parse_extent("head-dimension")?,
			query: self.parse_parameter(fields.require("query", path)?, &path.field("query"))?,
			key: self.parse_parameter(fields.require("key", path)?, &path.field("key"))?,
			value: self.parse_parameter(fields.require("value", path)?, &path.field("value"))?,
			output: self.parse_parameter(fields.require("output", path)?, &path.field("output"))?,
		}) }

	fn parse_rnn( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointRnnImage> { let fields = self.fields( node, path, &[
				"index",
				"sequence-length",
				"width",
				"input-weight",
				"recurrent-weight",
				"bias",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let parse_extent = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) }; Ok(CheckpointRnnImage {
			sequence_length: parse_extent("sequence-length")?,
			width: parse_extent("width")?,
			input_weight: self.parse_parameter(
				fields.require("input-weight", path)?,
				&path.field("input-weight"),
			)?, recurrent_weight: self.parse_parameter(
				fields.require("recurrent-weight", path)?,
				&path.field("recurrent-weight"),
			)?,
			bias: self.parse_parameter(fields.require("bias", path)?, &path.field("bias"))?,
		}) }

	fn parse_gru( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointGruImage> { let fields = self.fields( node, path, &[
				"index",
				"sequence-length",
				"width",
				"reset-input-weight",
				"reset-recurrent-weight",
				"reset-bias",
				"update-input-weight",
				"update-recurrent-weight",
				"update-bias",
				"candidate-input-weight",
				"candidate-recurrent-weight",
				"candidate-bias",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let parse_extent = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) };
		let sequence_length = parse_extent("sequence-length")?;
		let width = parse_extent("width")?;
		let mut parameter = |name: &str| self.parse_parameter(fields.require(name, path)?, &path.field(name));
		Ok(CheckpointGruImage { sequence_length, width,
			reset_input_weight: parameter("reset-input-weight")?,
			reset_recurrent_weight: parameter("reset-recurrent-weight")?,
			reset_bias: parameter("reset-bias")?,
			update_input_weight: parameter("update-input-weight")?,
			update_recurrent_weight: parameter("update-recurrent-weight")?,
			update_bias: parameter("update-bias")?,
			candidate_input_weight: parameter("candidate-input-weight")?,
			candidate_recurrent_weight: parameter("candidate-recurrent-weight")?,
			candidate_bias: parameter("candidate-bias")?,
		}) }

	fn parse_lstm( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointLstmImage> { let fields = self.fields( node, path, &[
				"index",
				"sequence-length",
				"width",
				"input-gate-input-weight",
				"input-gate-recurrent-weight",
				"input-gate-bias",
				"forget-gate-input-weight",
				"forget-gate-recurrent-weight",
				"forget-gate-bias",
				"output-gate-input-weight",
				"output-gate-recurrent-weight",
				"output-gate-bias",
				"candidate-input-weight",
				"candidate-recurrent-weight",
				"candidate-bias",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let parse_extent = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) };
		let sequence_length = parse_extent("sequence-length")?;
		let width = parse_extent("width")?;
		let mut parameter = |name: &str| self.parse_parameter(fields.require(name, path)?, &path.field(name));
		Ok(CheckpointLstmImage { sequence_length, width,
			input_gate_input_weight: parameter("input-gate-input-weight")?,
			input_gate_recurrent_weight: parameter("input-gate-recurrent-weight")?,
			input_gate_bias: parameter("input-gate-bias")?,
			forget_gate_input_weight: parameter("forget-gate-input-weight")?,
			forget_gate_recurrent_weight: parameter("forget-gate-recurrent-weight")?,
			forget_gate_bias: parameter("forget-gate-bias")?,
			output_gate_input_weight: parameter("output-gate-input-weight")?,
			output_gate_recurrent_weight: parameter("output-gate-recurrent-weight")?,
			output_gate_bias: parameter("output-gate-bias")?,
			candidate_input_weight: parameter("candidate-input-weight")?,
			candidate_recurrent_weight: parameter("candidate-recurrent-weight")?,
			candidate_bias: parameter("candidate-bias")?,
		}) }

	fn parse_layer( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointLayerImage> { let kind = match self.node(node, path)?.text() {
			"layer" => DenseBlockKind::Layer,
			"perc" => DenseBlockKind::Perc,
			value => return Err(self.invalid_enum(path, "dense block", value)),
		}; let (width, payload) = self.tagged(node, path)?; let width = width.to_owned(); let payload = payload.to_vec();
		let width = self.parse_nonzero_u64(&width, &path.field("width"))?;
		let fields = self.fields_from_children( &payload, path,
			&["index", "operations", "weight", "bias"],
			&["prelu"],
		)?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let operations = self.parse_operations(
			fields.require("operations", path)?,
			&path.field("operations"),
		)?;
		let weight = self.parse_parameter(fields.require("weight", path)?, &path.field("weight"))?;
		let bias = self.parse_parameter(fields.require("bias", path)?, &path.field("bias"))?;
		let prelu = fields
			.optional("prelu")
			.map(|node| self.parse_parameter_list(node, &path.field("prelu")))
			.transpose()? .unwrap_or_default(); Ok(CheckpointLayerImage {
			declaration: DenseLayer::with_kind(kind, width, operations), weight, bias, prelu, }) }

	fn parse_convolution( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointConvolutionImage> { let fields = self.fields( node, path, &[
				"index",
				"filters",
				"kernel",
				"input-length",
				"input-channels",
				"output-length",
				"operations",
				"weight",
				"bias",
			],
			&["prelu"],
		)?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let nonzero_field = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) };
		let filters = nonzero_field("filters")?;
		let kernel = nonzero_field("kernel")?;
		let input_length = nonzero_field("input-length")?;
		let input_channels = nonzero_field("input-channels")?;
		let output_length = nonzero_field("output-length")?;
		let operations = self.parse_operations(
			fields.require("operations", path)?,
			&path.field("operations"),
		)?;
		let weight = self.parse_parameter(fields.require("weight", path)?, &path.field("weight"))?;
		let bias = self.parse_parameter(fields.require("bias", path)?, &path.field("bias"))?;
		let prelu = fields
			.optional("prelu")
			.map(|node| self.parse_parameter_list(node, &path.field("prelu")))
			.transpose()? .unwrap_or_default(); Ok(CheckpointConvolutionImage {
			declaration: DenseConvolution::with_operations(filters, kernel, operations),
			geometry: DenseConvolutionGeometry::new(input_length, input_channels, output_length, filters, kernel), weight, bias,
			prelu, }) }

	fn parse_pool( &self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointPoolImage> { let fields = self.fields( node, path, &[
				"index",
				"size",
				"group-to-neuron",
				"input-length",
				"channels",
				"output-length",
				"group-order",
				"winner-contract",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let nonzero_field = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) };
		let size = nonzero_field("size")?;
		let group_to_neuron = match self.required_scalar(&fields, "group-to-neuron", path)? {
			"none" => None,
			value => Some(self.parse_nonzero_u64(value, &path.field("group-to-neuron"))?),
		};
		let input_length = nonzero_field("input-length")?;
		let channels = nonzero_field("channels")?;
		let output_length = nonzero_field("output-length")?;
		self.expect_scalar(
			fields.require("group-order", path)?,
			&path.field("group-order"),
			"group-major-channel-minor",
		)?; self.expect_scalar(
			fields.require("winner-contract", path)?,
			&path.field("winner-contract"),
			"lowest-logical-index",
		)?; checkpoint_pool_image( DensePool::new(size, group_to_neuron),
			DensePoolState::new(input_length, channels, output_length), path, ) }

	fn parse_kmeans( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointKMeansImage> { let fields = self.fields( node, path, &[
				"index",
				"clusters",
				"group-to-neuron",
				"input-width",
				"centroids",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let clusters = self.parse_nonzero_u64(
			self.scalar(fields.require("clusters", path)?, &path.field("clusters"))?,
			&path.field("clusters"),
		)?;
		let group_to_neuron = match self.required_scalar(&fields, "group-to-neuron", path)? {
			"none" => None,
			value => Some(self.parse_nonzero_u64(value, &path.field("group-to-neuron"))?),
		}; let input_width = self.parse_nonzero_u64(
			self.required_scalar(&fields, "input-width", path)?,
			&path.field("input-width"),
		)?;
		let centroids = self.parse_tensor(fields.require("centroids", path)?, &path.field("centroids"))?;
		Ok(CheckpointKMeansImage { clusters, group_to_neuron, input_width, centroids, }) }

	fn parse_tree( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointTreeImage> { let fields = self.fields( node, path, &[
				"index",
				"family",
				"trees",
				"depth",
				"input-width",
				"output-width",
				"internal-nodes-per-tree",
				"leaves-per-tree",
				"split-features",
				"split-thresholds",
				"leaf-values",
			], &[], )?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let family = match self.scalar(fields.require("family", path)?, &path.field("family"))? {
			"lightgbm" => DenseTreeFamily::LightGbm,
			"catboost" => DenseTreeFamily::CatBoost,
			"xgboost" => DenseTreeFamily::XGBoost,
			value => return Err(self.invalid_enum(&path.field("family"), "tree family", value)),
		}; let nonzero_u64_field = |name: &str| { self.parse_nonzero_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?, &path.field(name), ) };
		let trees = nonzero_u64_field("trees")?;
		let depth_u64 = nonzero_u64_field("depth")?;
		let depth = NonZeroU32::new(u32::try_from(depth_u64.get()).map_err(|error| { decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.field("depth"),
				format!("tree depth cannot be represented by u32: {error}"),
			) })?)
		.expect("parsed nonzero tree depth");
		Ok(CheckpointTreeImage { declaration: DenseTree::new(family, trees, depth),
			input_width: nonzero_u64_field("input-width")?,
			output_width: nonzero_u64_field("output-width")?,
			internal_nodes_per_tree: nonzero_u64_field("internal-nodes-per-tree")?,
			leaves_per_tree: nonzero_u64_field("leaves-per-tree")?,
			split_features: self.parse_tensor(
				fields.require("split-features", path)?,
				&path.field("split-features"),
			)?, split_thresholds: self.parse_tensor(
				fields.require("split-thresholds", path)?,
				&path.field("split-thresholds"),
			)?, leaf_values: self.parse_parameter(
				fields.require("leaf-values", path)?,
				&path.field("leaf-values"),
			)?, }) }

	fn parse_residual( &mut self, node: OgdlNodeId, path: &CheckpointPath, expected_index: usize,
	) -> CheckpointResult<CheckpointResidualImage> { let fields = self.fields( node, path,
			&["index", "output-width", "branch", "skip", "operations"],
			&["branch-prelu", "prelu"],
		)?; let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?; if index != expected_index { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			)); }
		let output_width = self.parse_nonzero_u64(
			self.required_scalar(&fields, "output-width", path)?,
			&path.field("output-width"),
		)?;
		let branch = self.parse_residual_branch(fields.require("branch", path)?, &path.field("branch"))?;
		let branch_prelu = fields
			.optional("branch-prelu")
			.map(|node| self.parse_parameter_list(node, &path.field("branch-prelu")))
			.transpose()? .unwrap_or_default(); let declared_output = branch.iter().rev().find_map(|step| match step {
			CheckpointResidualBranchImage::Layer(layer) => Some(layer.declaration.width()),
			CheckpointResidualBranchImage::Operation(_) => None, }); if declared_output != Some(output_width) {
			return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("output-width"),
				format!(
					"residual output width is {}, but the last branch layer yields {:?}",
					output_width, declared_output.map(NonZeroU64::get) ), )); }
		let skip = self.parse_residual_skip(fields.require("skip", path)?, &path.field("skip"))?;
		let operations = self.parse_operations(
			fields.require("operations", path)?,
			&path.field("operations"),
		)?; let prelu = fields
			.optional("prelu")
			.map(|node| self.parse_parameter_list(node, &path.field("prelu")))
			.transpose()? .unwrap_or_default(); Ok(CheckpointResidualImage { branch, branch_prelu, output_width, skip,
			operations, prelu, }) }

	fn parse_parameter_list( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointParameterImage>> { let (count, payload) = self.tagged(node, path)?;
		let count = self.parse_usize(count, &path.field("count"))?;
		let payload = payload.to_vec(); if count != payload.len() { return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("count"),
				format!(
					"PReLU parameter count is {count}, but {} parameters follow",
					payload.len() ), )); }
		payload .into_iter() .enumerate() .map(|(index, parameter)| { let parameter_path = path.index(index);
				if self.node(parameter, &parameter_path)?.text() != "parameter" {
					return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, parameter_path,
						"expected `parameter` in PReLU state list",
					)); }
				self.parse_parameter(parameter, &path.index(index)) }) .collect() }

	fn parse_residual_branch( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointResidualBranchImage>> { let (count, payload) = self.tagged(node, path)?;
		let count = self.parse_usize(count, &path.field("count"))?;
		let payload = payload.to_vec(); if count != payload.len() { return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("count"),
				format!(
					"branch step count is {count}, but {} steps follow",
					payload.len() ), )); }
		let mut branch = Vec::with_capacity(count); for (index, step) in payload.into_iter().enumerate() {
			let step_path = path.index(index); self.claim_block(&step_path)?; match self.node(step, &step_path)?.text() {
				"layer" | "perc" => branch.push(CheckpointResidualBranchImage::Layer(
					self.parse_layer(step, &step_path, index)?, )),
				"operation" => branch.push(CheckpointResidualBranchImage::Operation(
					self.parse_dense_operation(self.scalar(step, &step_path)?, &step_path)?, )),
				value => return Err(self.invalid_enum(&step_path, "residual branch step", value)),
			} }
		Ok(branch) }

	fn parse_residual_skip( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<CheckpointResidualSkipImage> { let (tag, payload) = self.tagged(node, path)?;
		let payload = payload.to_vec(); match tag {
			"identity" => {
				if !payload.is_empty() { return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, path.clone(),
						"identity skip has unexpected descendants",
					)); }
				Ok(CheckpointResidualSkipImage::Identity) }
			"linear-projection" => {
				let fields = self.fields_from_children(&payload, path, &["weight"], &[])?;
				Ok(CheckpointResidualSkipImage::Projection(
					self.parse_parameter(fields.require("weight", path)?, &path.field("weight"))?,
				)) }
			value => Err(self.invalid_enum(path, "residual skip", value)),
		} }

	fn parse_operations(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Vec<DenseOperation>> {
		let (count, payload) = self.tagged(node, path)?;
		let count = self.parse_usize(count, &path.field("count"))?;
		if count != payload.len() { return Err(decode_error( CheckpointDecodeErrorKind::InconsistentValue,
				path.field("count"),
				format!(
					"operation count is {count}, but {} operations follow",
					payload.len() ), )); }
		let mut operations = Vec::with_capacity(count); for (index, operation) in payload.iter().copied().enumerate() {
			let operation_path = path.index(index);
			if self.node(operation, &operation_path)?.text() != "operation" {
				return Err(decode_error( CheckpointDecodeErrorKind::UnknownField, operation_path, format!(
						"expected `operation`, found {:?}",
						self.node(operation, path)?.text() ), )); }
			operations.push(self.parse_dense_operation( self.scalar(operation, &path.index(index))?, &path.index(index), )?); }
		Ok(operations) }

	fn parse_dense_operation(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<DenseOperation> { DenseOperation::from_token(value).ok_or_else(|| self.invalid_enum(path, "dense operation", value)) }

	fn parse_parameter( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<CheckpointParameterImage> { let fields = self.fields( node, path,
			&["parameter", "first-moment", "second-moment"],
			&[], )?; Ok(CheckpointParameterImage {
			parameter: self.parse_tensor(fields.require("parameter", path)?, &path.field("parameter"))?,
			first_moment: self.parse_tensor(
				fields.require("first-moment", path)?,
				&path.field("first-moment"),
			)?, second_moment: self.parse_tensor(
				fields.require("second-moment", path)?,
				&path.field("second-moment"),
			)?, }) }

	fn parse_tensor(&mut self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<CheckpointTensorImage> {
		self.tensor_count = self.tensor_count.checked_add(1).ok_or_else(|| { decode_error(
				CheckpointDecodeErrorKind::LimitExceeded, path.clone(),
				"tensor count overflowed usize",
			) })?; if self.tensor_count > self.limits.tensors { return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded, path.clone(), format!(
					"tensor count is {}, limit is {}",
					self.tensor_count, self.limits.tensors ), )); }
		let fields = self.fields(node, path, &["dtype", "shape", "payload"], &[])?;
		let dtype = self.parse_dtype(
			self.scalar(fields.require("dtype", path)?, &path.field("dtype"))?,
			&path.field("dtype"),
		)?;
		let shape = self.parse_shape(fields.require("shape", path)?, &path.field("shape"))?;
		let bytes = self.parse_payload(fields.require("payload", path)?, &path.field("payload"))?;
		Ok(CheckpointTensorImage { dtype, shape, bytes, }) }

	fn parse_dtype(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<DType> { match value {
			"f32" => Ok(DType::F32),
			"int32" => Ok(DType::I32),
			_ => Err(self.invalid_enum(path, "tensor dtype", value)),
		} }

	fn parse_shape(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Vec<u64>> {
		let mut shape = Vec::new(); let mut children = self.node(node, path)?.children();
		while let Some(extent) = children.first().copied() { if children.len() != 1 { return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidSyntax, path.index(shape.len()),
					"shape chain branches instead of containing one ordered extent",
				)); }
			if shape.len() == self.limits.tensor_rank { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
					path.clone(),
					format!("tensor rank exceeds limit {}", self.limits.tensor_rank),
				)); }
			let extent_node = self.node(extent, &path.index(shape.len()))?;
			shape.push(self.parse_u64(extent_node.text(), &path.index(shape.len()))?); children = extent_node.children(); }
		Ok(shape) }

	fn parse_payload(&mut self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Vec<u8>> {
		let (tag, chunks) = self.tagged(node, path)?;
		if tag != "raw-bytes-hex" {
			return Err(self.invalid_enum(path, "tensor payload encoding", tag));
		}
		let chunks = chunks.to_vec(); if chunks.is_empty() { return Err(decode_error( CheckpointDecodeErrorKind::MissingField,
				path.clone(),
				"raw hexadecimal payload has no chunks",
			)); }
		let mut decoded_size = 0usize; for (index, chunk) in chunks.iter().copied().enumerate() {
			let chunk_path = path.index(index); let chunk = self.node(chunk, &chunk_path)?; if !chunk.children().is_empty() {
				return Err(decode_error( CheckpointDecodeErrorKind::InvalidSyntax, chunk_path,
					"payload chunk has unexpected descendants",
				)); }
			let Some(hex) = chunk.text().strip_prefix("0x") else {
				return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, chunk_path,
					"payload chunk must start with `0x`",
				)); }; if hex.len() % 2 != 0 || hex.len() > HEX_CHUNK_BYTES * 2 { return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidValue, chunk_path,
					format!("payload chunk contains {} hexadecimal digits", hex.len()),
				)); }
			let chunk_bytes = hex.len() / 2; if index + 1 != chunks.len() && chunk_bytes != HEX_CHUNK_BYTES {
				return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, chunk_path,
					format!("nonfinal payload chunk has {chunk_bytes} bytes, expected {HEX_CHUNK_BYTES}"),
				)); }
			if chunk_bytes == 0 && chunks.len() != 1 { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
					chunk_path,
					"empty payload chunk must be the only chunk",
				)); }
			decoded_size = decoded_size.checked_add(chunk_bytes).ok_or_else(|| { decode_error(
					CheckpointDecodeErrorKind::LimitExceeded, path.clone(),
					"tensor payload size overflowed usize",
				) })?; }
		if decoded_size > self.limits.tensor_bytes { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(), format!(
					"tensor payload has {decoded_size} bytes, limit is {}",
					self.limits.tensor_bytes ), )); }
		self.payload_bytes = self .payload_bytes .checked_add(decoded_size) .ok_or_else(|| { decode_error(
					CheckpointDecodeErrorKind::LimitExceeded, path.clone(),
					"total tensor payload size overflowed usize",
				) })?; if self.payload_bytes > self.limits.total_payload_bytes { return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded, path.clone(), format!(
					"decoded payload total is {} bytes, limit is {}",
					self.payload_bytes, self.limits.total_payload_bytes ), )); }
		let mut bytes = Vec::with_capacity(decoded_size); for (index, chunk) in chunks.iter().copied().enumerate() {
			bytes.extend(self.parse_hex_bytes(self.node(chunk, path)?.text(), &path.index(index))?); }
		Ok(bytes) }

	fn parse_calibration( &mut self, node: OgdlNodeId, path: &CheckpointPath,
	) -> CheckpointResult<Option<CheckpointTensorImage>> {
		let fields = self.fields(node, path, &["temperature"], &[])?;
		Ok(Some(self.parse_tensor(
			fields.require("temperature", path)?,
			&path.field("temperature"),
		)?)) } }

struct ParsedDataset { vectors: Vec<CheckpointArtifactVector>, feature_spans: Vec<CompiledFeatureSpan>,
	feature_normalization_mask: Vec<u32>, feature_width: usize, target_source_indices: Vec<usize>, task: DenseTask, }

struct ParsedTarget { source_indices: Vec<usize>, task: DenseTask, }

struct ParsedModel { output_adapter: Option<DenseOutputAdapter>, normalization: Vec<CheckpointTensorImage>,
	layers: Vec<CheckpointLayerImage>, blocks: Vec<CheckpointBlockImage>, temperature: Option<CheckpointTensorImage>, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckpointTopologyStorage { Manifest, Artifact, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckpointTensorValidation { Declaration, Payload, }

fn validate_artifact(artifact: &CheckpointArtifact) -> CheckpointResult<()> { validate_checkpoint_semantic_invariants(
		artifact, CheckpointTopologyStorage::Artifact, CheckpointTensorValidation::Payload, ) }

fn validate_checkpoint_semantic_invariants( artifact: &CheckpointArtifact, topology_storage: CheckpointTopologyStorage,
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> {
	let root = CheckpointPath::root().field("recipe");
	if artifact.task.uses_target_matrix() && !matches!( artifact.format_version, MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
				| TREE_CHECKPOINT_FORMAT_VERSION
				| EMBEDDING_CHECKPOINT_FORMAT_VERSION
				| RNN_CHECKPOINT_FORMAT_VERSION
				| GRU_CHECKPOINT_FORMAT_VERSION
				| LSTM_CHECKPOINT_FORMAT_VERSION ) { return Err(invalid_value(
			root.field("version"),
			format!("multi-target task requires semantic-model version {MULTI_TARGET_CHECKPOINT_FORMAT_VERSION}"),
		)); }
	validate_checkpoint_topology(artifact, &root.field("model"), topology_storage)?;
	if !is_semantic_model_version(artifact.format_version) && artifact.config.gradient_clip_norm.is_none() {
		return Err(invalid_value(
			root.field("training").field("gradient-clip-norm"),
			"legacy checkpoint versions require an explicit gradient clip norm",
		)); }
	validate_training_config( &artifact.config,
		&root.field("training"),
		artifact.format_version == FLAT_CHECKPOINT_FORMAT_VERSION, )?; validate_training_bounds( &artifact.config,
		artifact.bounds,
		&root.field("training").field("bounds"),
	)?;
	validate_vector_schema(artifact, &root.field("dataset"))?;
	validate_effective_model(artifact, &root.field("model"), tensor_validation)?;
	validate_native_metadata(artifact, &root.field("native"), topology_storage)?;
	Ok(()) }

fn validate_checkpoint_topology( artifact: &CheckpointArtifact, path: &CheckpointPath,
	storage: CheckpointTopologyStorage, ) -> CheckpointResult<()> { if artifact .blocks .iter()
		.any(|block| matches!(block, CheckpointBlockImage::Rnn(_)))
		&& !matches!( artifact.format_version,
			RNN_CHECKPOINT_FORMAT_VERSION | GRU_CHECKPOINT_FORMAT_VERSION | LSTM_CHECKPOINT_FORMAT_VERSION ) {
		return Err(validation_error(
			path.field("blocks"),
			format!("vanilla RNN topology requires semantic-model version {RNN_CHECKPOINT_FORMAT_VERSION}"),
		)); }
	if artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Gru(_))) && !matches!(
			artifact.format_version, GRU_CHECKPOINT_FORMAT_VERSION | LSTM_CHECKPOINT_FORMAT_VERSION ) {
		return Err(validation_error(
			path.field("blocks"),
			format!("GRU topology requires semantic-model version {GRU_CHECKPOINT_FORMAT_VERSION}"),
		)); }
	if artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Lstm(_)))
		&& artifact.format_version != LSTM_CHECKPOINT_FORMAT_VERSION
	{ return Err(validation_error(
			path.field("blocks"),
			format!("LSTM topology requires semantic-model version {LSTM_CHECKPOINT_FORMAT_VERSION}"),
		)); }
	if storage == CheckpointTopologyStorage::Manifest { let valid = match artifact.format_version {
				FLAT_CHECKPOINT_FORMAT_VERSION => !artifact.layers.is_empty() && artifact.blocks.is_empty(),
				STRUCTURED_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && artifact
						.blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Residual(_)))
						&& !artifact.blocks.iter().any(|block| { matches!( block, CheckpointBlockImage::Convolution(_)
									| CheckpointBlockImage::Pool(_) | CheckpointBlockImage::KMeans(_)
									| CheckpointBlockImage::Tree(_) ) }) && artifact.config.layers.is_empty() }
				POOL_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && artifact .blocks
						.iter() .any(|block| matches!(block, CheckpointBlockImage::Pool(_)))
						&& !artifact.blocks.iter().any(|block| { matches!( block, CheckpointBlockImage::Convolution(_)
									| CheckpointBlockImage::KMeans(_) | CheckpointBlockImage::Tree(_) ) }) && artifact.config.layers.is_empty() }
				LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty()
						&& !artifact.blocks.is_empty() && !artifact.blocks.iter().any(|block| { matches!( block,
							CheckpointBlockImage::Convolution(_) | CheckpointBlockImage::KMeans(_) | CheckpointBlockImage::Tree(_) )
					}) && artifact.config.layers.is_empty() }
				NATIVE_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && !artifact .blocks
						.iter() .any(|block| matches!(block, CheckpointBlockImage::KMeans(_)))
						&& !artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
						&& artifact.config.layers.is_empty() }
				KMEANS_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && !artifact .blocks
						.iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
						&& artifact.config.layers.is_empty() }
				MULTI_TARGET_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && !artifact
						.blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
						&& artifact.config.layers.is_empty() }
				TREE_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && artifact .blocks
						.iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
						&& artifact.config.layers.is_empty() }
				EMBEDDING_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && artifact
						.blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Embedding(_)))
						&& artifact.config.layers.is_empty() }
				RNN_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && artifact .blocks
						.iter() .any(|block| matches!(block, CheckpointBlockImage::Rnn(_)))
						&& artifact.config.layers.is_empty() }
				GRU_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && artifact .blocks
						.iter() .any(|block| matches!(block, CheckpointBlockImage::Gru(_)))
						&& artifact.config.layers.is_empty() }
				LSTM_CHECKPOINT_FORMAT_VERSION => { artifact.layers.is_empty() && !artifact.blocks.is_empty() && artifact .blocks
						.iter() .any(|block| matches!(block, CheckpointBlockImage::Lstm(_)))
						&& artifact.config.layers.is_empty() }
				_ => false, }; if !valid { return Err(validation_error(
				path.field("blocks"),
				"checkpoint manifest mixes incompatible flat and structured topology",
			)); }
		return Ok(()); }
	match artifact.format_version { FLAT_CHECKPOINT_FORMAT_VERSION => {
			if artifact.layers.is_empty() || artifact.blocks.len() != artifact.layers.len() { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v5 requires matching nonempty flat layer and block views",
				)); }
			for (index, (block, layer)) in artifact.blocks.iter().zip(&artifact.layers).enumerate() {
				if block != &CheckpointBlockImage::Layer(layer.clone()) { return Err(validation_error(
						path.field("blocks").index(index),
						"checkpoint v5 block differs from its flat layer view",
					)); } } }
		STRUCTURED_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v6 must not mix a legacy flat layer view with structured blocks",
				)); }
			if artifact.blocks.iter().any(|block| { matches!( block, CheckpointBlockImage::Convolution(_)
						| CheckpointBlockImage::Pool(_)
						| CheckpointBlockImage::KMeans(_)
						| CheckpointBlockImage::Tree(_) ) }) { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v6 cannot contain a pool block",
				)); }
			if artifact.blocks.is_empty() || !artifact .blocks .iter()
					.any(|block| matches!(block, CheckpointBlockImage::Residual(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v6 requires a nonempty topology containing a residual block",
				)); }
			if !artifact.config.layers.is_empty() { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v6 cannot mix legacy configuration layers with structured blocks",
				)); } }
		POOL_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v7 must not mix a legacy flat layer view with structured blocks",
				)); }
			if artifact.blocks.is_empty() || !artifact .blocks .iter()
					.any(|block| matches!(block, CheckpointBlockImage::Pool(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v7 requires a nonempty topology containing a pool block",
				)); }
			if artifact.blocks.iter().any(|block| { matches!( block, CheckpointBlockImage::Convolution(_)
						| CheckpointBlockImage::KMeans(_)
						| CheckpointBlockImage::Tree(_) ) }) { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v7 cannot contain a convolution block",
				)); }
			if !artifact.config.layers.is_empty() { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v7 cannot mix legacy configuration layers with structured blocks",
				)); } }
		LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| artifact.blocks.iter().any(|block| { matches!( block, CheckpointBlockImage::Convolution(_)
							| CheckpointBlockImage::KMeans(_) | CheckpointBlockImage::Tree(_) ) }) { return Err(validation_error(
					path.field("blocks"),
					"checkpoint v8 requires one nonempty convolution-free canonical block view without legacy layers",
				)); } }
		NATIVE_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::KMeans(_)))
				|| artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					format!(
						"checkpoint v{} requires one nonempty canonical block view without legacy layers",
						artifact.format_version ), )); } }
		KMEANS_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v10 requires one nonempty canonical block view without legacy layers",
				)); } }
		MULTI_TARGET_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v11 requires one nonempty canonical block view without legacy layers",
				)); } }
		TREE_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| !artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Tree(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v12 requires one nonempty canonical block view containing a tree",
				)); } }
		EMBEDDING_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| !artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Embedding(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v13 requires one nonempty canonical block view containing a leading embedding",
				)); } }
		RNN_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| !artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Rnn(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v14 requires one nonempty canonical block view containing a leading RNN",
				)); } }
		GRU_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| !artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Gru(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v15 requires one nonempty canonical block view containing a leading GRU",
				)); } }
		LSTM_CHECKPOINT_FORMAT_VERSION => { if !artifact.layers.is_empty() || artifact.blocks.is_empty()
				|| !artifact.config.layers.is_empty()
				|| !artifact .blocks .iter() .any(|block| matches!(block, CheckpointBlockImage::Lstm(_)))
			{ return Err(validation_error(
					path.field("blocks"),
					"checkpoint v16 requires one nonempty canonical block view containing a leading LSTM",
				)); } }
		version => { return Err(invalid_value(
				CheckpointPath::root().field("recipe").field("version"),
				format!("unsupported checkpoint version {version}"),
			)); } }
	Ok(()) }

fn validate_native_metadata( artifact: &CheckpointArtifact, path: &CheckpointPath, storage: CheckpointTopologyStorage,
) -> CheckpointResult<()> { if !is_semantic_model_version(artifact.format_version) { return match artifact.native {
			None => Ok(()), Some(_) => Err(validation_error( path.clone(),
				"native realization metadata requires semantic-model version 8 or newer",
			)), }; }
	let Some(native) = &artifact.native else { return if storage == CheckpointTopologyStorage::Manifest { Ok(()) } else {
			Err(validation_error( path.clone(), format!(
					"checkpoint v{} artifact omits native realization metadata",
					artifact.format_version ), )) }; }; if native.program.is_zero()
		|| native.realization.is_zero()
		|| native.topology.is_zero()
		|| native.discovery.is_zero()
	{ return Err(invalid_value( path.clone(),
			"program, native realization, topology, and discovery identities must be nonzero",
		)); }
	if native.kernels.is_empty() { return Err(validation_error(
			path.field("kernels"),
			"native realization contains no kernel identities",
		)); }
	let mut identities = BTreeSet::new(); for (index, kernel) in native.kernels.iter().enumerate() {
		let kernel_path = path.field("kernels").index(index);
		for (name, label) in [
			("target.backend", &kernel.target.backend),
			("target.architecture", &kernel.target.architecture),
			("target.abi", &kernel.target.abi),
			("toolchain.name", &kernel.toolchain.name),
			("toolchain.version", &kernel.toolchain.version),
		] {
			if label.as_str().contains(['\t', '\r', '\n']) {
				return Err(invalid_value( kernel_path.clone(),
					format!("native identity field {name} contains an OGDL delimiter"),
				)); } }
		if kernel.digest.is_zero() || kernel.toolchain.digest.is_zero() { return Err(invalid_value(
				kernel_path.field("digest"),
				"native image and toolchain digests must be nonzero",
			)); }
		let format_matches = match kernel.format { NativeKernelFormat::Cubin => {
				kernel.target.backend.as_str() == "nvidia-cuda-driver"
					&& kernel.target.abi.as_str() == "elf64-cubin"
			}
			NativeKernelFormat::Hsaco => {
				kernel.target.backend.as_str() == "amd-rocr-hsa"
					&& kernel .target .abi .as_str()
						.starts_with("elf64-amdgpu-code-object-v")
			} }; if !format_matches { return Err(validation_error(
				kernel_path.field("format"),
				"native file format disagrees with its target backend and ABI",
			)); }
		if !identities.insert(( kernel.format, kernel.target.clone(), kernel.toolchain.clone(), kernel.digest, )) {
			return Err(validation_error( kernel_path,
				"native kernel identity is duplicated",
			)); } }
	Ok(()) }

fn validation_error(path: CheckpointPath, detail: impl Into<String>) -> CheckpointError { decode_error(CheckpointDecodeErrorKind::InconsistentValue, path, detail) }

fn invalid_value(path: CheckpointPath, detail: impl Into<String>) -> CheckpointError { decode_error(CheckpointDecodeErrorKind::InvalidValue, path, detail) }

fn checkpoint_pool_image( declaration: DensePool, state: DensePoolState, path: &CheckpointPath,
) -> CheckpointResult<CheckpointPoolImage> { let expected_output_length = NonZeroU64::new( state.input_length() .get()
			.div_ceil(declaration.size().get()), )
	.expect("nonzero pool extents have a nonzero output length");
	if state.output_length() != expected_output_length { return Err(validation_error(
			path.field("output-length"),
			format!(
				"pool output length is {}, expected {} for input length {} and size {}",
				state.output_length(), expected_output_length, state.input_length(), declaration.size() ), )); }
	let input_width = state.input_width().ok_or_else(|| { validation_error(
			path.field("channels"),
			"pool input length and channel count overflow u64",
		) })?; let output_width = state.output_width().ok_or_else(|| { validation_error(
			path.field("channels"),
			"pool output length and channel count overflow u64",
		) })?; Ok(CheckpointPoolImage { size: declaration.size(), group_to_neuron: declaration.group_to_neuron(),
		input_length: state.input_length(), channels: state.channels(), output_length: state.output_length(), input_width,
		output_width, group_order: state.group_order(), winner_contract: state.winner_contract(), }) }

fn validate_training_config( config: &DenseTrainingConfig, path: &CheckpointPath, require_legacy_layers: bool,
) -> CheckpointResult<()> { if require_legacy_layers && config.layers.is_empty() { return Err(validation_error(
			path.field("layers"),
			"effective model does not retain any declared layer",
		)); }
	match config.epochs { TrainingHorizon::Finite(epochs) if config.warmup_epochs > epochs.get() => {
			return Err(validation_error(
				path.field("warmup-epochs"),
				format!(
					"warmup epoch count {} exceeds total epoch count {}",
					config.warmup_epochs, epochs ), )); }
		TrainingHorizon::Unbounded if config.learning_rate_decay != LearningRateDecay::Constant => {
			return Err(validation_error(
				path.field("learning-rate-decay"),
				"unbounded training requires a constant post-warmup learning rate",
			)); }
		TrainingHorizon::Unbounded if config.warmup_epochs > i32::MAX as u64 => { return Err(invalid_value(
				path.field("warmup-epochs"),
				"unbounded training warmup exceeds the int32 schedule domain",
			)); }
		TrainingHorizon::Finite(_) | TrainingHorizon::Unbounded => {} }
	if config.reduction_tree_lanes == 0 || config.reduction_tree_lanes > MAXIMUM_REDUCTION_TREE_LANES
		|| !config.reduction_tree_lanes.is_power_of_two()
	{ return Err(invalid_value(
			path.field("reduction-tree-lanes"),
			"reduction tree lanes must be a power of two in 1..=1024",
		)); }
	if let Some(value) = config.gradient_clip_norm && (!value.is_finite() || value <= 0.0)
	{ return Err(invalid_value(
			path.field("gradient-clip-norm"),
			"value must be finite and positive",
		)); }
	for (name, value, allow_zero) in [
		("normalization-epsilon", config.normalization_epsilon, false),
		("adamw.learning-rate", config.adamw.learning_rate, false),
		("adamw.epsilon", config.adamw.epsilon, false),
		("adamw.weight-decay", config.adamw.weight_decay, true),
	] { if !value.is_finite() || value < 0.0 || !allow_zero && value == 0.0 { let field_path = name
				.split('.')
				.fold(path.clone(), |path, field| path.field(field)); return Err(invalid_value( field_path, format!(
					"value must be finite and {}",
					if allow_zero {
						"nonnegative"
					} else {
						"positive"
					} ), )); } }
	for (name, value) in [
		("beta-one", config.adamw.beta_one),
		("beta-two", config.adamw.beta_two),
	] { if !value.is_finite() || !(0.0..1.0).contains(&value) { return Err(invalid_value(
				path.field("adamw").field(name),
				"value must be finite and in [0, 1)",
			)); } }
	Ok(()) }

fn validate_training_bounds( config: &DenseTrainingConfig, bounds: TrainingBounds, path: &CheckpointPath,
) -> CheckpointResult<()> { if bounds.train_rows == 0 { return Err(invalid_value(
			path.field("train-rows"),
			"training row count must be nonzero",
		)); }
	if bounds.train_rows > i32::MAX as u64 { return Err(invalid_value(
			path.field("train-rows"),
			"training rows exceed the int32 IndexMap domain",
		)); }
	if bounds.epochs != config.epochs { return Err(validation_error(
			path.field("epochs"),
			format!(
				"bound epoch count {} differs from configuration epoch count {}",
				bounds.epochs, config.epochs ), )); }
	let expected_training_iterations = config.epochs.loop_iterations();
	if bounds.training_iterations != expected_training_iterations { return Err(validation_error(
			path.field("training-iterations"),
			format!(
				"value is {}, expected {} from one full-partition update per epoch",
				loop_iterations_text(bounds.training_iterations), loop_iterations_text(expected_training_iterations) ), )); }
	match config.epochs { TrainingHorizon::Finite(training_epochs) => { let training_iterations = training_epochs.get();
			if training_iterations > i32::MAX as u64 { return Err(invalid_value(
					path.field("training-iterations"),
					"training iterations exceed the int32 schedule domain",
				)); }
			let iterations = training_iterations .checked_add(bounds.calibration_iterations) .ok_or_else(|| {
					validation_error(path.field("iterations"), "total iteration count overflowed")
				})?; expect_u64( bounds.iterations .finite() .ok_or_else(|| { validation_error(
							path.field("iterations"),
							"finite training cannot declare unbounded total iterations",
						) })? .get(), iterations,
				path.field("iterations"),
				"training plus calibration iterations",
			)?; }
		TrainingHorizon::Unbounded => { if bounds.calibration_iterations != 0 { return Err(validation_error(
					path.field("calibration-iterations"),
					"unbounded training cannot declare a post-training calibration phase",
				)); }
			if !bounds.iterations.is_unbounded() { return Err(validation_error(
					path.field("iterations"),
					"unbounded training requires unbounded total iterations",
				)); } } }
	expect_u64( bounds.warmup_iterations, config.warmup_epochs,
		path.field("warmup-iterations"),
		"one full-partition update per warmup epoch",
	) }

fn expect_u64(actual: u64, expected: u64, path: CheckpointPath, derivation: &str) -> CheckpointResult<()> {
	if actual != expected { return Err(validation_error( path,
			format!("value is {actual}, expected {expected} from {derivation}"),
		)); }
	Ok(()) }

fn validate_vector_schema(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	let vectors_path = path.field("vectors");
	if artifact.vectors.is_empty() { return Err(validation_error( vectors_path,
			"dataset vector schema is empty",
		)); }
	let mut source_indices = BTreeSet::new(); let mut names = BTreeSet::new(); let mut targets = Vec::new();
	let mut previous_source_index = None; for (index, vector) in artifact.vectors.iter().enumerate() {
		let vector_path = path.field("vectors").index(index);
		if !source_indices.insert(vector.source_index) { return Err(validation_error(
				vector_path.field("source-index"),
				format!(
					"source index {} appears more than once",
					vector.source_index ), )); }
		if previous_source_index.is_some_and(|previous| previous >= vector.source_index) { return Err(validation_error(
				vector_path.field("source-index"),
				"vector source indices must retain strictly increasing source order",
			)); }
		previous_source_index = Some(vector.source_index); if vector.name.is_empty() { return Err(invalid_value(
				vector_path.field("name-bytes"),
				"vector name is empty",
			)); }
		if !names.insert(vector.name.clone()) { return Err(validation_error(
				vector_path.field("name-bytes"),
				"vector name appears more than once",
			)); }
		validate_vector_metadata(vector, &vector_path)?; if vector.role == VectorRole::Target { targets.push(vector); } }
	if artifact.target_source_indices.is_empty() || artifact.target_source_indices.len() != artifact.task.target_count()
		|| artifact.target_source_indices.first().copied() != Some(artifact.task.target_vector())
	{ return Err(validation_error(
			path.field("target"),
			format!(
				"target order {:?} disagrees with task primary source {} and count {}",
				artifact.target_source_indices, artifact.task.target_vector(), artifact.task.target_count() ), )); }
	let declared = artifact .target_source_indices .iter() .copied() .collect::<BTreeSet<_>>();
	if declared.len() != artifact.target_source_indices.len() { return Err(validation_error(
			path.field("target").field("source-indices"),
			"declared target order repeats a source index",
		)); }
	if targets.len() != declared.len() || targets .iter() .any(|target| !declared.contains(&target.source_index))
	{ return Err(validation_error(
			path.field("target"),
			format!(
				"saved vector schema has {} target-role vectors but declared order is {:?}",
				targets.len(), artifact.target_source_indices ), )); }
	let ordered_targets = artifact .target_source_indices .iter() .copied() .map(|source_index| { targets .iter() .copied()
				.find(|target| target.source_index == source_index)
				.expect("validated target source identity exists")
		}) .collect::<Vec<_>>(); if artifact.task.uses_target_matrix() { validate_multi_target_task( artifact.config.loss,
			artifact.task, &ordered_targets,
			&path.field("target"),
		)?; } else { validate_task( artifact.config.loss, artifact.task, ordered_targets[0],
			&path.field("target"),
		)?; }
	validate_feature_spans(artifact, path) }

fn validate_vector_metadata(vector: &CheckpointArtifactVector, path: &CheckpointPath) -> CheckpointResult<()> {
	let valid = match (&vector.semantic_type, &vector.encoding, &vector.metadata) {
		(SemanticType::Numeric, VectorEncoding::F32 | VectorEncoding::I32, CheckpointArtifactMetadata::None) => { true }
		( SemanticType::Temporal, VectorEncoding::RelativeSecondsI32,
			CheckpointArtifactMetadata::Temporal { nanoseconds, .. }, ) => { if *nanoseconds >= 1_000_000_000 {
				return Err(invalid_value(
					path.field("metadata").field("nanoseconds"),
					"temporal nanoseconds must be below one billion",
				)); }
			true }
		( SemanticType::Categorical, VectorEncoding::DictionaryI32, CheckpointArtifactMetadata::Categorical { dictionary },
		) => {
			// An empty fitted dictionary is canonical when every fit-partition
			// observation was missing. Its one saved dense span is the reserved
			// missing/unseen route, for both features and targets.
			if !dictionary.is_empty() {
				validate_byte_dictionary(dictionary, &path.field("metadata"), true)?;
			}
			true }
		( SemanticType::Ordinal, VectorEncoding::OrdinalI32, CheckpointArtifactMetadata::Ordinal { ordered_labels }, ) => {
			validate_byte_dictionary(ordered_labels, &path.field("metadata"), false)?;
			true }
		(SemanticType::Text, VectorEncoding::Utf8, CheckpointArtifactMetadata::None) => true,
		(SemanticType::Image, VectorEncoding::Bytes, CheckpointArtifactMetadata::Image { encoded_variants }) => {
			validate_image_metadata(encoded_variants, &path.field("metadata"))?;
			true }
		(SemanticType::Binary, VectorEncoding::Bytes, CheckpointArtifactMetadata::None) => true, _ => false, }; if !valid {
		return Err(validation_error(
			path.field("metadata"),
			format!(
				"metadata is incompatible with {:?}/{:?}",
				vector.semantic_type, vector.encoding ), )); }
	Ok(()) }

pub(crate) fn validate_saved_vector(vector: &CheckpointArtifactVector, path: &CheckpointPath) -> CheckpointResult<()> { validate_vector_metadata(vector, path) }

fn validate_byte_dictionary(values: &[Vec<u8>], path: &CheckpointPath, require_sorted: bool) -> CheckpointResult<()> {
	if values.is_empty() {
		return Err(invalid_value(path.clone(), "dictionary is empty"));
	}
	let mut distinct = BTreeSet::new(); for (index, value) in values.iter().enumerate() { if value.is_empty() {
			return Err(invalid_value( path.index(index),
				"dictionary label is empty",
			)); }
		if !distinct.insert(value) { return Err(validation_error( path.index(index),
				"dictionary label appears more than once",
			)); }
		if require_sorted && index > 0 && values[index - 1] >= *value { return Err(validation_error( path.index(index),
				"categorical dictionary is not in canonical ascending byte order",
			)); } }
	Ok(()) }

fn validate_image_metadata(values: &[CheckpointImageMetadata], path: &CheckpointPath) -> CheckpointResult<()> {
	if values.is_empty() {
		return Err(invalid_value(path.clone(), "image variant set is empty"));
	}
	let mut distinct = BTreeSet::new(); for (index, value) in values.iter().copied().enumerate() {
		let value_path = path.index(index); if value.width == 0 || value.height == 0 { return Err(invalid_value( value_path,
				"image dimensions must be nonzero",
			)); }
		if value.channels == Some(0) { return Err(invalid_value(
				value_path.field("channels"),
				"image channel count must be nonzero",
			)); }
		if value.sample_bits == Some(0) { return Err(invalid_value(
				value_path.field("sample-bits"),
				"image sample width must be nonzero",
			)); }
		if !image_metadata_could_be_ingested(value) { return Err(validation_error( value_path.clone(),
				"image header facts cannot be produced by the declared encoded format",
			)); }
		if !distinct.insert(value) { return Err(validation_error( value_path,
				"image variant appears more than once",
			)); }
		if index > 0 && values[index - 1] >= value { return Err(validation_error( path.index(index),
				"image variants are not in canonical ascending header order",
			)); } }
	Ok(()) }

fn image_metadata_could_be_ingested(value: CheckpointImageMetadata) -> bool { match value.format {
		EncodedImageFormat::Png => { value.width <= i32::MAX as u32 && value.height <= i32::MAX as u32 && matches!(
					(value.channels, value.color_model, value.sample_bits), ( Some(1), Some(ImageColorModel::Grayscale),
						Some(1 | 2 | 4 | 8 | 16) ) | (Some(3), Some(ImageColorModel::Rgb), Some(8 | 16))
						| ( Some(1), Some(ImageColorModel::IndexedRgb), Some(1 | 2 | 4 | 8)
						) | (Some(2), Some(ImageColorModel::GrayscaleAlpha), Some(8 | 16))
						| (Some(4), Some(ImageColorModel::Rgba), Some(8 | 16)) ) }
		EncodedImageFormat::Jpeg => { value.width <= u32::from(u16::MAX) && value.height <= u32::from(u16::MAX)
				&& value.sample_bits.is_some_and(|bits| bits > 0)
				&& match (value.channels, value.color_model) { (Some(1), Some(ImageColorModel::Grayscale)) => true,
					(Some(3), None | Some(ImageColorModel::Rgb | ImageColorModel::YCbCr)) => true,
					(Some(4), None | Some(ImageColorModel::Cmyk | ImageColorModel::Ycck)) => true,
					(Some(channels), None) => channels > 0 && !matches!(channels, 1 | 3 | 4), _ => false, } }
		EncodedImageFormat::Gif87a | EncodedImageFormat::Gif89a => { value.width <= u32::from(u16::MAX)
				&& value.height <= u32::from(u16::MAX)
				&& value.channels == Some(1)
				&& value.color_model == Some(ImageColorModel::IndexedRgb)
				&& value.sample_bits.is_none_or(|bits| (1..=8).contains(&bits)) }
		EncodedImageFormat::Bmp => { matches!( (value.channels, value.color_model, value.sample_bits), (None, None, None) | (
						Some(1), Some(ImageColorModel::IndexedRgb), Some(1 | 2 | 4 | 8)
					) | (Some(3), Some(ImageColorModel::Bgr), Some(8 | 16)) ) }
		EncodedImageFormat::WebP => match (value.channels, value.color_model, value.sample_bits) {
			(Some(3), Some(ImageColorModel::YCbCr), Some(8)) => value.width <= 0x3fff && value.height <= 0x3fff,
			(Some(3), Some(ImageColorModel::Rgb), Some(8)) | (Some(4), Some(ImageColorModel::Rgba), Some(8)) => {
				value.width <= 1 << 24 && value.height <= 1 << 24 }
			_ => false, }, } }

fn validate_task( loss: DenseLoss, task: DenseTask, target: &CheckpointArtifactVector, path: &CheckpointPath,
) -> CheckpointResult<()> { match task { DenseTask::BinaryClassification { positive_code, .. } => {
			if !matches!(loss, DenseLoss::BinaryCrossEntropy | DenseLoss::Focal) { return Err(validation_error(
					path.field("task"),
					"binary task requires binary cross entropy or focal loss",
				)); }
			let expected_positive_code = match (target.semantic_type, target.encoding, &target.metadata) { (
					SemanticType::Numeric, VectorEncoding::I32 | VectorEncoding::F32, CheckpointArtifactMetadata::None, ) => Some(1), (
					SemanticType::Categorical, VectorEncoding::DictionaryI32, CheckpointArtifactMetadata::Categorical { dictionary },
				) if dictionary.len() <= 2 => Some(i32::try_from(dictionary.len()).expect("length at most two") - 1),
				_ => None, }; let Some(expected_positive_code) = expected_positive_code else { return Err(validation_error(
					path.clone(),
					"target schema is incompatible with checkpoint categorical BCE",
				)); }; if positive_code != expected_positive_code { return Err(validation_error(
					path.field("positive-code"),
					format!(
						"binary positive code is {positive_code}, expected {expected_positive_code} from the saved target schema"
					), )); } }
		DenseTask::MulticlassClassification { class_count, reserved_code, .. } => { if loss != DenseLoss::CrossEntropy {
				return Err(validation_error(
					path.field("task"),
					"multiclass task requires cross entropy",
				)); }
			let CheckpointArtifactMetadata::Categorical { dictionary } = &target.metadata else { return Err(validation_error(
					path.clone(),
					"multiclass target does not have categorical metadata",
				)); }; if target.semantic_type != SemanticType::Categorical
				|| target.encoding != VectorEncoding::DictionaryI32
			{ return Err(validation_error( path.clone(),
					"multiclass target is not categorical dictionary-int32",
				)); }
			let expected_count = dictionary .len() .checked_add(1)
				.ok_or_else(|| validation_error(path.field("class-count"), "class count overflowed usize"))?;
			if class_count != expected_count { return Err(validation_error(
					path.field("class-count"),
					format!("class count is {class_count}, expected {expected_count}"),
				)); }
			let expected_reserved = i32::try_from(dictionary.len()).map_err(|error| { validation_error(
					path.field("reserved-unseen-code"),
					format!("dictionary length cannot be represented as i32: {error}"),
				) })?; if reserved_code != expected_reserved { return Err(validation_error(
					path.field("reserved-unseen-code"),
					format!("reserved code is {reserved_code}, expected {expected_reserved}"),
				)); } }
		DenseTask::ScalarRegression { .. } => { if !matches!( loss,
				DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber ) { return Err(validation_error(
					path.field("task"),
					"scalar regression requires MSE, MAE, or Huber",
				)); }
			if target.semantic_type != SemanticType::Numeric
				|| !matches!(target.encoding, VectorEncoding::I32 | VectorEncoding::F32)
			{ return Err(validation_error( path.clone(),
					"scalar regression target is not numeric",
				)); } }
		DenseTask::MultiTargetBinaryClassification { .. }
		| DenseTask::JointMulticlassClassification { .. }
		| DenseTask::MultiTargetRegression { .. } => { return Err(validation_error(
				path.field("task"),
				"multi-target task requires ordered multi-target validation",
			)); } }
	Ok(()) }

fn validate_multi_target_task( loss: DenseLoss, task: DenseTask, targets: &[&CheckpointArtifactVector],
	path: &CheckpointPath, ) -> CheckpointResult<()> { if targets.len() < 2 || targets.len() != task.target_count() {
		return Err(validation_error(
			path.field("source-indices"),
			format!(
				"multi-target task declares {} columns but {} target schemas were saved",
				task.target_count(), targets.len() ), )); }
	for (index, target) in targets.iter().copied().enumerate() { if target.semantic_type != SemanticType::Numeric
			|| !matches!(target.encoding, VectorEncoding::I32 | VectorEncoding::F32)
			|| target.metadata != CheckpointArtifactMetadata::None
		{ return Err(validation_error(
				path.field("source-indices").index(index),
				"multi-target objectives require numeric int32 or binary32 target schemas without categorical metadata",
			)); } }
	let compatible = match task { DenseTask::MultiTargetBinaryClassification { .. } => {
			matches!(loss, DenseLoss::BinaryCrossEntropy | DenseLoss::Focal) }
		DenseTask::JointMulticlassClassification { .. } => loss == DenseLoss::CrossEntropy,
		DenseTask::MultiTargetRegression { .. } => { matches!( loss,
				DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber ) }
		DenseTask::BinaryClassification { .. }
		| DenseTask::MulticlassClassification { .. }
		| DenseTask::ScalarRegression { .. } => false, }; if !compatible { return Err(validation_error(
			path.field("task"),
			format!("multi-target task {task:?} is incompatible with objective {loss:?}"),
		)); }
	Ok(()) }

fn validate_feature_spans(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	let features = artifact .vectors .iter() .filter(|vector| vector.role == VectorRole::Feature) .collect::<Vec<_>>();
	if features.is_empty() { return Err(validation_error(
			path.field("feature-spans"),
			"dense dataset has no feature vectors",
		)); }
	if artifact.feature_spans.len() != features.len() { return Err(validation_error(
			path.field("feature-spans"),
			format!(
				"{} spans describe {} feature vectors",
				artifact.feature_spans.len(), features.len() ), )); }
	let mut start = 0usize; for (index, (span, vector)) in artifact.feature_spans.iter().zip(features).enumerate() {
		let span_path = path.field("feature-spans").index(index);
		if span.source_vector() != vector.source_index { return Err(validation_error(
				span_path.field("source-index"),
				format!(
					"span source is {}, expected {}",
					span.source_vector(), vector.source_index ), )); }
		if span.start() != start { return Err(validation_error(
				span_path.field("start"),
				format!(
					"span starts at {}, expected contiguous start {start}",
					span.start() ), )); }
		match ( span.lowering(), vector.semantic_type, vector.encoding, &vector.metadata, ) { (
				DenseFeatureLowering::NumericScalar, SemanticType::Numeric, VectorEncoding::I32 | VectorEncoding::F32,
				CheckpointArtifactMetadata::None, ) if span.width() == 1 => {}
			( DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, }, SemanticType::Categorical,
				VectorEncoding::DictionaryI32, CheckpointArtifactMetadata::Categorical { dictionary },
			) if dictionary_width == dictionary.len() && reserved_index == dictionary.len()
				&& span.width() == dictionary.len().saturating_add(1) => {}
			_ => { return Err(validation_error(
					span_path.field("lowering"),
					"feature lowering, width, and vector schema are inconsistent",
				)); } }
		start = start .checked_add(span.width())
			.ok_or_else(|| validation_error(span_path.field("width"), "feature width overflowed usize"))?;
	}
	if start != artifact.feature_width || artifact.feature_width == 0 { return Err(validation_error(
			path.field("feature-width"),
			format!(
				"feature width is {}, contiguous spans cover {start}",
				artifact.feature_width ), )); }
	let expected_mask = feature_normalization_mask(&artifact.feature_spans, artifact.feature_width);
	if artifact.feature_normalization_mask.len() != expected_mask.len() { return Err(validation_error(
			path.field("feature-normalization-mask"),
			format!(
				"normalization mask has {} values, expected {}",
				artifact.feature_normalization_mask.len(), expected_mask.len() ), )); }
	for (index, (actual, expected)) in artifact .feature_normalization_mask .iter() .zip(expected_mask) .enumerate()
	{ if *actual != expected { return Err(validation_error(
				path.field("feature-normalization-mask").index(index),
				format!("mask bits are 0x{actual:08x}, expected 0x{expected:08x}"),
			)); } }
	Ok(()) }

fn validate_effective_model( artifact: &CheckpointArtifact, path: &CheckpointPath,
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> { match artifact.format_version {
		FLAT_CHECKPOINT_FORMAT_VERSION => validate_effective_flat_model(artifact, path, tensor_validation),
		STRUCTURED_CHECKPOINT_FORMAT_VERSION
		| POOL_CHECKPOINT_FORMAT_VERSION
		| LEGACY_NATIVE_CHECKPOINT_FORMAT_VERSION
		| NATIVE_CHECKPOINT_FORMAT_VERSION
		| KMEANS_CHECKPOINT_FORMAT_VERSION
		| MULTI_TARGET_CHECKPOINT_FORMAT_VERSION
		| TREE_CHECKPOINT_FORMAT_VERSION
		| EMBEDDING_CHECKPOINT_FORMAT_VERSION
		| RNN_CHECKPOINT_FORMAT_VERSION
		| GRU_CHECKPOINT_FORMAT_VERSION
		| LSTM_CHECKPOINT_FORMAT_VERSION => validate_effective_structured_model(artifact, path, tensor_validation),
		version => Err(invalid_value(
			CheckpointPath::root().field("recipe").field("version"),
			format!("unsupported checkpoint version {version}"),
		)), } }

fn validate_effective_flat_model( artifact: &CheckpointArtifact, path: &CheckpointPath,
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> { if artifact.layers.is_empty() {
		return Err(validation_error(
			path.field("blocks"),
			"effective model has no blocks",
		)); }
	let target_width = artifact.task.output_width(); match artifact.output_adapter { Some(adapter) => {
			if artifact.layers.len() < 2 || artifact.config.layers.len() + 1 != artifact.layers.len() {
				return Err(validation_error(
					path.field("output-adapter"),
					"adapter requires at least one declared block and one final synthetic block",
				)); }
			let source = artifact .config .layers .last()
				.expect("validated nonempty layers");
			let final_layer = artifact .layers .last()
				.expect("validated nonempty effective layers");
			if adapter.source_width() != source.width()
				|| usize::try_from(adapter.target_width().get()).ok() != Some(target_width)
			{ return Err(validation_error(
					path.field("output-adapter"),
					"adapter widths do not join the declared output block to the task output",
				)); }
			if final_layer.declaration.kind() != DenseBlockKind::Layer
				|| final_layer.declaration.width() != adapter.target_width()
				|| !final_layer.declaration.operations().is_empty()
			{ return Err(validation_error(
					path.field("blocks").index(artifact.layers.len() - 1),
					"final adapter block must be an operation-free linear layer at the target width",
				)); }
			let classification_requires_logits = matches!( artifact.task,
				DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. } ); let source_is_logits = source
				.operations() .iter()
				.all(|operation| matches!(operation, DenseOperation::Activation(activation) if activation.is_identity()));
			let source_width = source.width(); let target_width = adapter.target_width();
			if source_width == target_width && (!classification_requires_logits || source_is_logits) {
				return Err(validation_error(
					path.field("output-adapter"),
					"serialized adapter is redundant under the checkpoint-v5 effective-layer rule",
				)); } }
		None => { if artifact.config.layers.len() != artifact.layers.len() { return Err(validation_error(
					path.field("blocks"),
					"declared and effective block counts differ",
				)); }
			let output = artifact .config .layers .last()
				.expect("validated nonempty layers");
			if usize::try_from(output.width().get()).ok() != Some(target_width) { return Err(validation_error(
					path.field("blocks").index(artifact.layers.len() - 1),
					"final block width differs from the task output without an adapter",
				)); }
			if matches!( artifact.task, DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. }
			) && !output .operations() .iter()
				.all(|operation| matches!(operation, DenseOperation::Activation(activation) if activation.is_identity()))
			{ return Err(validation_error(
					path.field("blocks")
						.index(artifact.layers.len() - 1)
						.field("operations"),
					"classification output is not logits and has no linear output adapter",
				)); } } }
	validate_normalization_state(artifact, &path.field("normalization"), tensor_validation)?;
	let mut input_width = u64::try_from(artifact.feature_width).map_err(|error| { validation_error(
			path.field("input-width"),
			format!("feature width cannot be represented by u64: {error}"),
		) })?; for (index, layer) in artifact.layers.iter().enumerate() {
		let layer_path = path.field("blocks").index(index);
		let output_width = layer.declaration.width().get(); validate_parameter( &layer.weight,
			&layer_path.field("weight"),
			&[input_width, output_width], tensor_validation, )?; validate_parameter( &layer.bias,
			&layer_path.field("bias"),
			&[output_width], tensor_validation, )?; validate_prelu_parameters( layer.declaration.operations().iter().copied(),
			&layer.prelu,
			&layer_path.field("prelu"),
			tensor_validation, )?; input_width = output_width; }
	validate_temperature(artifact, &path.field("calibration"), tensor_validation)
}

fn validate_effective_structured_model( artifact: &CheckpointArtifact, path: &CheckpointPath,
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> { if artifact.blocks.is_empty() {
		return Err(validation_error(
			path.field("blocks"),
			"effective model has no blocks",
		)); }
	validate_structured_pool_declarations(artifact, path)?; validate_structured_kmeans_declarations(artifact, path)?;
	validate_structured_tree_declarations(artifact, path)?; validate_structured_embedding_declarations(artifact, path)?;
	let target_width = artifact.task.output_width(); match artifact.output_adapter { Some(adapter) => {
			if artifact.blocks.len() < 2 { return Err(validation_error(
					path.field("output-adapter"),
					"adapter requires at least one declared block and one final synthetic block",
				)); }
			let source = &artifact.blocks[artifact.blocks.len() - 2];
			let final_block = artifact.blocks.last().expect("validated nonempty blocks");
			if adapter.source_width() != source.output_width()
				|| usize::try_from(adapter.target_width().get()).ok() != Some(target_width)
			{ return Err(validation_error(
					path.field("output-adapter"),
					"adapter widths do not join the declared output block to the task output",
				)); }
			let CheckpointBlockImage::Layer(final_layer) = final_block else { return Err(validation_error(
					path.field("blocks").index(artifact.blocks.len() - 1),
					"final adapter block must be an ordinary layer",
				)); }; if final_layer.declaration.kind() != DenseBlockKind::Layer
				|| final_layer.declaration.width() != adapter.target_width()
				|| !final_layer.declaration.operations().is_empty()
			{ return Err(validation_error(
					path.field("blocks").index(artifact.blocks.len() - 1),
					"final adapter block must be an operation-free linear layer at the target width",
				)); }
			let classification_requires_logits = matches!( artifact.task,
				DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. } ); let source_is_logits = source
				.output_operations() .iter()
				.all(|operation| matches!(operation, DenseOperation::Activation(activation) if activation.is_identity()));
			let source_width = source.output_width(); let target_width = adapter.target_width();
			if source_width == target_width && (!classification_requires_logits || source_is_logits) {
				return Err(validation_error(
					path.field("output-adapter"),
					format!(
						"serialized adapter is redundant under the checkpoint-v{} effective-block rule",
						artifact.format_version ), )); } }
		None => {
			let output = artifact.blocks.last().expect("validated nonempty blocks");
			if usize::try_from(output.output_width().get()).ok() != Some(target_width) { return Err(validation_error(
					path.field("blocks").index(artifact.blocks.len() - 1),
					"final block width differs from the task output without an adapter",
				)); }
			if matches!( artifact.task, DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. }
			) && !output .output_operations() .iter()
				.all(|operation| matches!(operation, DenseOperation::Activation(activation) if activation.is_identity()))
			{ return Err(validation_error(
					path.field("blocks")
						.index(artifact.blocks.len() - 1)
						.field("operations"),
					"classification output is not logits and has no linear output adapter",
				)); } } }
	validate_normalization_state(artifact, &path.field("normalization"), tensor_validation)?;
	let mut input_width = u64::try_from(artifact.feature_width).map_err(|error| { validation_error(
			path.field("input-width"),
			format!("feature width cannot be represented by u64: {error}"),
		) })?; let mut logical_length = input_width; let mut logical_channels = 1u64;
	for (index, block) in artifact.blocks.iter().enumerate() {
		let block_path = path.field("blocks").index(index);
		match block { CheckpointBlockImage::Embedding(embedding) => { if embedding.sequence_length.get() != input_width {
					return Err(validation_error(
						block_path.field("sequence-length"),
						format!(
							"embedding sequence length is {}, prepared feature width is {input_width}",
							embedding.sequence_length ), )); }
				validate_parameter( &embedding.table,
					&block_path.field("table"),
					&[embedding.vocabulary.get(), embedding.dimensions.get()], tensor_validation, )?; input_width = embedding
					.sequence_length .get() .checked_mul(embedding.dimensions.get()) .ok_or_else(|| {
						validation_error(block_path.clone(), "embedding output width overflowed u64")
					})?; logical_length = embedding.sequence_length.get(); logical_channels = embedding.dimensions.get(); }
			CheckpointBlockImage::Attention(attention) => { if index != 1 || !matches!( artifact.blocks.first(),
						Some(CheckpointBlockImage::Embedding(_)) ) { return Err(validation_error( block_path.clone(),
						"causal attention must immediately follow the leading embedding block",
					)); }
				if attention.sequence_length.get() != logical_length || attention.dimensions.get() != logical_channels
				{ return Err(validation_error( block_path.clone(),
						"attention geometry differs from the preceding embedding sequence",
					)); }
				let reconstructed_dimensions = attention .heads .get() .checked_mul(attention.head_dimension.get()) .ok_or_else(|| {
						validation_error(block_path.clone(), "attention head geometry overflowed u64")
					})?; if reconstructed_dimensions != attention.dimensions.get() { return Err(validation_error(
						block_path.field("head-dimension"),
						"attention heads times head dimension differs from the embedding dimension",
					)); }
				for (name, parameter) in [
					("query", &attention.query),
					("key", &attention.key),
					("value", &attention.value),
					("output", &attention.output),
				] { validate_parameter( parameter, &block_path.field(name),
						&[attention.dimensions.get(), attention.dimensions.get()], tensor_validation, )?; }
				input_width = attention .sequence_length .get() .checked_mul(attention.dimensions.get())
					.ok_or_else(|| validation_error(block_path, "attention output width overflowed u64"))?;
			}
			CheckpointBlockImage::Rnn(rnn) => { if index != 0 { return Err(validation_error( block_path.clone(),
						"the first vanilla RNN case must be the leading model block",
					)); }
				for (feature_index, span) in artifact.feature_spans.iter().enumerate() {
					let Some(vector) = artifact.vectors.iter().find(|vector| {
						vector.source_index == span.source_vector() && vector.role == VectorRole::Feature }) else {
						return Err(validation_error( block_path.clone(),
							format!("RNN feature span {feature_index} has no source vector"),
						)); }; if span.lowering() != DenseFeatureLowering::NumericScalar
						|| span.width() != 1 || vector.semantic_type != SemanticType::Numeric
						|| !matches!(vector.encoding, VectorEncoding::I32 | VectorEncoding::F32)
					{ return Err(validation_error( block_path.clone(),
							"the first vanilla RNN case requires one numeric scalar per feature-column time step",
						)); } }
				if rnn.sequence_length.get() != input_width { return Err(validation_error(
						block_path.field("sequence-length"),
						format!(
							"RNN sequence length is {}, prepared feature width is {input_width}",
							rnn.sequence_length ), )); }
				validate_parameter( &rnn.input_weight,
					&block_path.field("input-weight"),
					&[1, rnn.width.get()], tensor_validation, )?; validate_parameter( &rnn.recurrent_weight,
					&block_path.field("recurrent-weight"),
					&[rnn.width.get(), rnn.width.get()], tensor_validation, )?; validate_parameter( &rnn.bias,
					&block_path.field("bias"),
					&[rnn.width.get()], tensor_validation, )?; input_width = rnn.width.get(); logical_length = input_width;
				logical_channels = 1; }
			CheckpointBlockImage::Gru(gru) => { if index != 0 { return Err(validation_error( block_path.clone(),
						"the first GRU case must be the leading model block",
					)); }
				for (feature_index, span) in artifact.feature_spans.iter().enumerate() {
					let Some(vector) = artifact.vectors.iter().find(|vector| {
						vector.source_index == span.source_vector() && vector.role == VectorRole::Feature }) else {
						return Err(validation_error( block_path.clone(),
							format!("GRU feature span {feature_index} has no source vector"),
						)); }; if span.lowering() != DenseFeatureLowering::NumericScalar
						|| span.width() != 1 || vector.semantic_type != SemanticType::Numeric
						|| !matches!(vector.encoding, VectorEncoding::I32 | VectorEncoding::F32)
					{ return Err(validation_error( block_path.clone(),
							"the first GRU case requires one numeric scalar per feature-column time step",
						)); } }
				if gru.sequence_length.get() != input_width { return Err(validation_error(
						block_path.field("sequence-length"),
						format!(
							"GRU sequence length is {}, prepared feature width is {input_width}",
							gru.sequence_length ), )); }
				for (name, parameter) in [
					("reset-input-weight", &gru.reset_input_weight),
					("update-input-weight", &gru.update_input_weight),
					("candidate-input-weight", &gru.candidate_input_weight),
				] { validate_parameter( parameter, &block_path.field(name), &[1, gru.width.get()], tensor_validation, )?; }
				for (name, parameter) in [
					("reset-recurrent-weight", &gru.reset_recurrent_weight),
					("update-recurrent-weight", &gru.update_recurrent_weight),
					(
						"candidate-recurrent-weight",
						&gru.candidate_recurrent_weight, ), ] { validate_parameter( parameter, &block_path.field(name),
						&[gru.width.get(), gru.width.get()], tensor_validation, )?; }
				for (name, parameter) in [
					("reset-bias", &gru.reset_bias),
					("update-bias", &gru.update_bias),
					("candidate-bias", &gru.candidate_bias),
				] { validate_parameter( parameter, &block_path.field(name), &[gru.width.get()], tensor_validation, )?; }
				input_width = gru.width.get(); logical_length = input_width; logical_channels = 1; }
			CheckpointBlockImage::Lstm(lstm) => { if index != 0 { return Err(validation_error( block_path.clone(),
						"the first LSTM case must be the leading model block",
					)); }
				for (feature_index, span) in artifact.feature_spans.iter().enumerate() {
					let Some(vector) = artifact.vectors.iter().find(|vector| {
						vector.source_index == span.source_vector() && vector.role == VectorRole::Feature }) else {
						return Err(validation_error( block_path.clone(),
							format!("LSTM feature span {feature_index} has no source vector"),
						)); }; if span.lowering() != DenseFeatureLowering::NumericScalar
						|| span.width() != 1 || vector.semantic_type != SemanticType::Numeric
						|| !matches!(vector.encoding, VectorEncoding::I32 | VectorEncoding::F32)
					{ return Err(validation_error( block_path.clone(),
							"the first LSTM case requires one numeric scalar per feature-column time step",
						)); } }
				if lstm.sequence_length.get() != input_width { return Err(validation_error(
						block_path.field("sequence-length"),
						format!(
							"LSTM sequence length is {}, prepared feature width is {input_width}",
							lstm.sequence_length ), )); }
				for (name, parameter) in [
					("input-gate-input-weight", &lstm.input_gate_input_weight),
					("forget-gate-input-weight", &lstm.forget_gate_input_weight),
					("output-gate-input-weight", &lstm.output_gate_input_weight),
					("candidate-input-weight", &lstm.candidate_input_weight),
				] { validate_parameter( parameter, &block_path.field(name), &[1, lstm.width.get()], tensor_validation, )?; }
				for (name, parameter) in [ (
						"input-gate-recurrent-weight",
						&lstm.input_gate_recurrent_weight, ), (
						"forget-gate-recurrent-weight",
						&lstm.forget_gate_recurrent_weight, ), (
						"output-gate-recurrent-weight",
						&lstm.output_gate_recurrent_weight, ), (
						"candidate-recurrent-weight",
						&lstm.candidate_recurrent_weight, ), ] { validate_parameter( parameter, &block_path.field(name),
						&[lstm.width.get(), lstm.width.get()], tensor_validation, )?; }
				for (name, parameter) in [
					("input-gate-bias", &lstm.input_gate_bias),
					("forget-gate-bias", &lstm.forget_gate_bias),
					("output-gate-bias", &lstm.output_gate_bias),
					("candidate-bias", &lstm.candidate_bias),
				] { validate_parameter( parameter, &block_path.field(name), &[lstm.width.get()], tensor_validation, )?; }
				input_width = lstm.width.get(); logical_length = input_width; logical_channels = 1; }
			CheckpointBlockImage::Layer(layer) => { let output_width = layer.declaration.width().get(); validate_parameter(
					&layer.weight,
					&block_path.field("weight"),
					&[input_width, output_width], tensor_validation, )?; validate_parameter( &layer.bias,
					&block_path.field("bias"),
					&[output_width], tensor_validation, )?; validate_prelu_parameters( layer.declaration.operations().iter().copied(),
					&layer.prelu,
					&block_path.field("prelu"),
					tensor_validation, )?; if let Some(CheckpointBlockImage::Pool(pool)) = index .checked_sub(1)
					.and_then(|previous| artifact.blocks.get(previous))
				{ validate_pool_routed_weight(pool, layer, &block_path, tensor_validation)?; }
				if let Some(CheckpointBlockImage::KMeans(kmeans)) = index .checked_sub(1)
					.and_then(|previous| artifact.blocks.get(previous))
				{ validate_kmeans_routed_weight(kmeans, layer, &block_path, tensor_validation)?; }
				input_width = output_width; logical_length = output_width; logical_channels = 1; }
			CheckpointBlockImage::Convolution(convolution) => { let geometry = convolution.geometry;
				if convolution.declaration.filters() != geometry.filters() || convolution.declaration.kernel() != geometry.kernel()
				{ return Err(validation_error( block_path.clone(),
						"convolution declaration differs from its resolved geometry",
					)); }
				if convolution.declaration.operations().len() > 1 || convolution .declaration .operations() .iter()
						.any(|operation| !matches!(operation, DenseOperation::Activation(_)))
				{ return Err(validation_error(
						block_path.field("operations"),
						"convolution accepts at most one activation operation",
					)); }
				if geometry.input_length().get() != logical_length || geometry.input_channels().get() != logical_channels
					|| geometry.input_width().map(NonZeroU64::get) != Some(input_width)
				{ return Err(validation_error(
						block_path.field("input-length"),
						"convolution input geometry differs from the preceding logical shape",
					)); }
				if geometry.kernel().get() > logical_length { return Err(validation_error(
						block_path.field("kernel"),
						"convolution kernel exceeds its logical input length",
					)); }
				let expected_output_length = logical_length - geometry.kernel().get() + 1;
				if geometry.output_length().get() != expected_output_length { return Err(validation_error(
						block_path.field("output-length"),
						format!(
							"convolution output length is {}, expected {expected_output_length}",
							geometry.output_length() ), )); }
				validate_parameter( &convolution.weight,
					&block_path.field("weight"),
					&[ geometry.kernel().get(), geometry.input_channels().get(), geometry.filters().get(), ], tensor_validation, )?;
				validate_parameter( &convolution.bias,
					&block_path.field("bias"),
					&[geometry.filters().get()], tensor_validation, )?; validate_prelu_parameters(
					convolution.declaration.operations().iter().copied(), &convolution.prelu,
					&block_path.field("prelu"),
					tensor_validation, )?; input_width = geometry .output_width()
					.expect("validated convolution output width is nonzero")
					.get(); logical_length = geometry.output_length().get(); logical_channels = geometry.filters().get(); }
			CheckpointBlockImage::Pool(pool) => { if pool.input_width.get() != input_width { return Err(validation_error(
						block_path.field("input-length"),
						format!(
							"pool input width is {}, preceding block width is {input_width}",
							pool.input_width ), )); }
				if pool.input_length.get() != logical_length { return Err(validation_error(
						block_path.field("input-length"),
						format!(
							"pool input length is {}, expected {logical_length} from the preceding logical shape",
							pool.input_length ), )); }
				if pool.channels.get() != logical_channels { return Err(validation_error(
						block_path.field("channels"),
						format!(
							"pool channel count is {}, expected {logical_channels} from the preceding logical shape",
							pool.channels ), )); }
				input_width = pool.output_width.get(); logical_length = pool.output_length.get();
				logical_channels = pool.channels.get(); }
			CheckpointBlockImage::KMeans(kmeans) => { if kmeans.input_width.get() != input_width { return Err(validation_error(
						block_path.field("input-width"),
						format!(
							"K-means input width is {}, preceding block width is {input_width}",
							kmeans.input_width ), )); }
				validate_f32_tensor( &kmeans.centroids,
					&block_path.field("centroids"),
					&[kmeans.clusters.get(), kmeans.input_width.get()], false, tensor_validation, )?;
				input_width = kmeans.clusters.get(); logical_length = input_width; logical_channels = 1; }
			CheckpointBlockImage::Tree(tree) => { if tree.input_width.get() != input_width { return Err(validation_error(
						block_path.field("input-width"),
						format!(
							"tree input width is {}, preceding block width is {input_width}",
							tree.input_width ), )); }
				let split_elements = tree .declaration .trees() .get() .checked_mul(tree.internal_nodes_per_tree.get())
					.ok_or_else(|| { validation_error( block_path.clone(),
							"tree split tensor extent overflowed u64",
						) })?; let leaf_elements = tree .declaration .trees() .get() .checked_mul(tree.leaves_per_tree.get())
					.and_then(|elements| elements.checked_mul(tree.output_width.get())) .ok_or_else(|| {
						validation_error(block_path.clone(), "tree leaf tensor extent overflowed u64")
					})?; validate_i32_tensor( &tree.split_features,
					&block_path.field("split-features"),
					&[split_elements], tensor_validation, )?; if tensor_validation == CheckpointTensorValidation::Payload {
					for (split, bytes) in tree.split_features.bytes.chunks_exact(4).enumerate() { let feature =
							i32::from_le_bytes(bytes.try_into().expect("exact four-byte tensor chunk"));
						if feature < 0 || u64::try_from(feature) .ok() .is_none_or(|feature| feature >= input_width)
						{ return Err(invalid_value( block_path
									.field("split-features")
									.field("payload")
									.index(split), format!(
									"tree split feature {feature} is outside input width {input_width}"
								), )); } } }
				validate_f32_tensor( &tree.split_thresholds,
					&block_path.field("split-thresholds"),
					&[split_elements], false, tensor_validation, )?; validate_parameter( &tree.leaf_values,
					&block_path.field("leaf-values"),
					&[leaf_elements], tensor_validation, )?; input_width = tree.output_width.get(); logical_length = input_width;
				logical_channels = 1; }
			CheckpointBlockImage::Residual(residual) => { let residual_input_width = input_width;
				let mut branch_width = input_width; let mut retained_layer = false;
				for (step_index, step) in residual.branch.iter().enumerate() {
					if let CheckpointResidualBranchImage::Layer(layer) = step { retained_layer = true;
						let output_width = layer.declaration.width().get();
						let step_path = block_path.field("branch").index(step_index);
						validate_parameter( &layer.weight,
							&step_path.field("weight"),
							&[branch_width, output_width], tensor_validation, )?; validate_parameter( &layer.bias,
							&step_path.field("bias"),
							&[output_width], tensor_validation, )?; validate_prelu_parameters(
							layer.declaration.operations().iter().copied(), &layer.prelu,
							&step_path.field("prelu"),
							tensor_validation, )?; branch_width = output_width; } }
				let branch_operations = residual.branch.iter().filter_map(|step| match step {
					CheckpointResidualBranchImage::Operation(operation) => Some(*operation),
					CheckpointResidualBranchImage::Layer(_) => None, }); validate_prelu_parameters( branch_operations,
					&residual.branch_prelu,
					&block_path.field("branch-prelu"),
					tensor_validation, )?; if !retained_layer || branch_width != residual.output_width.get() {
					return Err(validation_error(
						block_path.field("output-width"),
						"residual output width must equal its last branch layer width",
					)); }
				match (&residual.skip, residual_input_width == branch_width) { (CheckpointResidualSkipImage::Identity, true) => {}
					(CheckpointResidualSkipImage::Projection(projection), false) => { validate_parameter( projection,
							&block_path.field("skip").field("weight"),
							&[residual_input_width, branch_width], tensor_validation, )?; }
					(CheckpointResidualSkipImage::Identity, false) => { return Err(validation_error(
							block_path.field("skip"),
							"residual width mismatch requires a weight-only linear projection",
						)); }
					(CheckpointResidualSkipImage::Projection(_), true) => { return Err(validation_error(
							block_path.field("skip"),
							"equal residual widths require an identity skip",
						)); } }
				validate_prelu_parameters( residual.operations.iter().copied(), &residual.prelu,
					&block_path.field("prelu"),
					tensor_validation, )?; input_width = branch_width; logical_length = branch_width; logical_channels = 1; } } }
	validate_temperature(artifact, &path.field("calibration"), tensor_validation)
}

fn validate_structured_pool_declarations(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	for (index, block) in artifact.blocks.iter().enumerate() { let CheckpointBlockImage::Pool(pool) = block else {
			continue; };
		let block_path = path.field("blocks").index(index);
		let expected = checkpoint_pool_image( DensePool::new(pool.size, pool.group_to_neuron),
			DensePoolState::new(pool.input_length, pool.channels, pool.output_length), &block_path, )?; if *pool != expected {
			return Err(validation_error( block_path.clone(),
				"pool cached widths or semantic contracts differ from its saved shape",
			)); }
		let Some(neurons) = pool.group_to_neuron else { continue; };
		let Some(CheckpointBlockImage::Layer(layer)) = artifact.blocks.get(index + 1) else { return Err(validation_error(
				block_path.field("group-to-neuron"),
				"pool group-to-neuron routing requires an immediately following ordinary layer",
			)); }; if layer.declaration.kind() != DenseBlockKind::Layer || layer.declaration.width() != neurons {
			return Err(validation_error(
				block_path.field("group-to-neuron"),
				format!(
					"pool routes to {neurons} neurons, but the immediately following block is not an ordinary layer of that width"
				), )); } }
	Ok(()) }

fn validate_structured_kmeans_declarations( artifact: &CheckpointArtifact, path: &CheckpointPath,
) -> CheckpointResult<()> { for (index, block) in artifact.blocks.iter().enumerate() {
		let CheckpointBlockImage::KMeans(kmeans) = block else { continue; };
		let block_path = path.field("blocks").index(index);
		if kmeans.clusters.get() > i32::MAX as u64 { return Err(validation_error(
				block_path.field("clusters"),
				"K-means cluster count exceeds Recipe's int32 assignment-index domain",
			)); }
		let Some(neurons) = kmeans.group_to_neuron else { continue; };
		let Some(CheckpointBlockImage::Layer(layer)) = artifact.blocks.get(index + 1) else { return Err(validation_error(
				block_path.field("group-to-neuron"),
				"K-means group-to-neuron routing requires an immediately following ordinary layer",
			)); }; if layer.declaration.kind() != DenseBlockKind::Layer || layer.declaration.width() != neurons {
			return Err(validation_error(
				block_path.field("group-to-neuron"),
				format!(
					"K-means routes to {neurons} neurons, but the immediately following block is not an ordinary layer of that width"
				), )); } }
	Ok(()) }

fn validate_structured_tree_declarations(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	let trees = artifact .blocks .iter() .enumerate() .filter_map(|(index, block)| match block {
			CheckpointBlockImage::Tree(tree) => Some((index, tree)), _ => None, }) .collect::<Vec<_>>(); if trees.is_empty() {
		return Ok(()); }
	if artifact.blocks.len() != 1 || trees.len() != 1 || artifact.output_adapter.is_some() { return Err(validation_error(
			path.field("blocks"),
			"a supervised tree is one terminal model block and cannot use a dense output adapter",
		)); }
	let (index, tree) = trees[0];
	let block_path = path.field("blocks").index(index);
	let depth = tree.declaration.depth().get(); if depth > 30 { return Err(validation_error(
			block_path.field("depth"),
			"tree depth exceeds Recipe's checked int32 traversal domain",
		)); }
	let leaves = 1u64 .checked_shl(depth)
		.ok_or_else(|| validation_error(block_path.field("depth"), "tree leaf count overflowed u64"))?;
	let internal_nodes = leaves - 1;
	if tree.leaves_per_tree.get() != leaves || tree.internal_nodes_per_tree.get() != internal_nodes {
		return Err(validation_error( block_path.clone(),
			"tree cached node or leaf count differs from its declared complete depth",
		)); }
	if usize::try_from(tree.output_width.get()).ok() != Some(artifact.task.output_width()) { return Err(validation_error(
			block_path.field("output-width"),
			"tree output width differs from the saved task",
		)); }
	Ok(()) }

fn validate_structured_embedding_declarations( artifact: &CheckpointArtifact, path: &CheckpointPath,
) -> CheckpointResult<()> { let embeddings = artifact .blocks .iter() .enumerate()
		.filter_map(|(index, block)| match block { CheckpointBlockImage::Embedding(embedding) => Some((index, embedding)),
			_ => None, }) .collect::<Vec<_>>(); if embeddings.is_empty() {
		if artifact.config.data_normalization == DenseDataNormalization::Identity { return Err(validation_error(
				path.field("normalization"),
				"identity input handling requires one leading embedding block",
			)); }
		return Ok(()); }
	if embeddings.len() != 1 || embeddings[0].0 != 0 { return Err(validation_error(
			path.field("blocks"),
			"the fixed-token model requires exactly one leading embedding block",
		)); }
	if artifact.config.data_normalization != DenseDataNormalization::Identity { return Err(validation_error(
			path.field("normalization"),
			"embedding token IDs require identity input handling",
		)); }
	let embedding = embeddings[0].1; if embedding.vocabulary.get() > i32::MAX as u64 { return Err(validation_error(
			path.field("blocks").index(0).field("vocabulary"),
			"embedding vocabulary exceeds Recipe's checked int32 token-index domain",
		)); }
	let feature_vectors = artifact .vectors .iter() .filter(|vector| vector.role == VectorRole::Feature)
		.collect::<Vec<_>>(); if feature_vectors.len() != artifact.feature_width
		|| feature_vectors.iter().any(|vector| { vector.semantic_type != SemanticType::Numeric
				|| vector.encoding != VectorEncoding::I32
				|| !matches!(vector.metadata, CheckpointArtifactMetadata::None) }) || artifact .feature_spans .iter()
		.any(|span| span.width() != 1 || span.lowering() != DenseFeatureLowering::NumericScalar)
	{ return Err(validation_error(
			path.field("dataset").field("feature-spans"),
			"embedding requires one exact numeric int32 feature per fixed sequence position",
		)); }
	Ok(()) }

fn validate_pool_routed_weight( pool: &CheckpointPoolImage, layer: &CheckpointLayerImage, layer_path: &CheckpointPath,
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> {
	let Some(neurons) = pool.group_to_neuron else { return Ok(()); };
	if tensor_validation == CheckpointTensorValidation::Declaration { return Ok(()); }
	let routing = DenseGroupToNeuronRouting::resolve(pool.output_length, neurons); for (name, tensor) in [
		("parameter", &layer.weight.parameter),
		("first-moment", &layer.weight.first_moment),
		("second-moment", &layer.weight.second_moment),
	] { let neurons = usize::try_from(neurons.get()).map_err(|error| { validation_error(
				layer_path.field("weight").field(name).field("shape"),
				format!("routed neuron width cannot be represented by usize: {error}"),
			) })?; let channels = usize::try_from(pool.channels.get()).map_err(|error| { validation_error(
				layer_path.field("weight").field(name).field("shape"),
				format!("pool channel count cannot be represented by usize: {error}"),
			) })?; for (entry, bytes) in tensor.bytes.chunks_exact(4).enumerate() { let input = entry / neurons;
			let neuron = entry % neurons; let group = input / channels; let allowed = match routing {
				DenseGroupToNeuronRouting::Identity { .. } => neuron == group, DenseGroupToNeuronRouting::Expand {
					neurons_per_group, .. } => {
					neuron / usize::try_from(neurons_per_group.get()).expect("routing divisor fits usize")
						== group }
				DenseGroupToNeuronRouting::Contract { groups_per_neuron, .. } => {
					group / usize::try_from(groups_per_neuron.get()).expect("routing divisor fits usize")
						== neuron }
				DenseGroupToNeuronRouting::FullyConnected { .. } => true, };
			if !allowed && u32::from_le_bytes(bytes.try_into().expect("exact four-byte tensor chunk")) != 0 {
				return Err(validation_error( layer_path
						.field("weight")
						.field(name)
						.field("payload")
						.index(entry),
					"pool-routed dense state requires every disallowed weight entry to be exact +0.0",
				)); } } }
	Ok(()) }

fn validate_kmeans_routed_weight( kmeans: &CheckpointKMeansImage, layer: &CheckpointLayerImage,
	layer_path: &CheckpointPath, tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> {
	let Some(neurons) = kmeans.group_to_neuron else { return Ok(()); };
	if tensor_validation == CheckpointTensorValidation::Declaration { return Ok(()); }
	let routing = DenseGroupToNeuronRouting::resolve(kmeans.clusters, neurons); for (name, tensor) in [
		("parameter", &layer.weight.parameter),
		("first-moment", &layer.weight.first_moment),
		("second-moment", &layer.weight.second_moment),
	] { let neurons = usize::try_from(neurons.get()).map_err(|error| { validation_error(
				layer_path.field("weight").field(name).field("shape"),
				format!("routed neuron width cannot be represented by usize: {error}"),
			) })?; for (entry, bytes) in tensor.bytes.chunks_exact(4).enumerate() { let group = entry / neurons;
			let neuron = entry % neurons; let allowed = match routing {
				DenseGroupToNeuronRouting::Identity { .. } => neuron == group, DenseGroupToNeuronRouting::Expand {
					neurons_per_group, .. } => {
					neuron / usize::try_from(neurons_per_group.get()).expect("routing divisor fits usize")
						== group }
				DenseGroupToNeuronRouting::Contract { groups_per_neuron, .. } => {
					group / usize::try_from(groups_per_neuron.get()).expect("routing divisor fits usize")
						== neuron }
				DenseGroupToNeuronRouting::FullyConnected { .. } => true, };
			if !allowed && u32::from_le_bytes(bytes.try_into().expect("exact four-byte tensor chunk")) != 0 {
				return Err(validation_error( layer_path
						.field("weight")
						.field(name)
						.field("payload")
						.index(entry),
					"K-means-routed dense state requires every disallowed weight entry to be exact +0.0",
				)); } } }
	Ok(()) }

fn validate_normalization_state( artifact: &CheckpointArtifact, path: &CheckpointPath,
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> {
	let shape = [u64::try_from(artifact.feature_width).map_err(|error| { validation_error( path.clone(),
			format!("feature width cannot be represented by u64: {error}"),
		) })?]; match artifact.config.data_normalization { DenseDataNormalization::Identity => {
			if !artifact.normalization.is_empty() { return Err(validation_error( path.clone(),
					"identity input handling must not retain fitted tensors",
				)); }
			Ok(()) }
		DenseDataNormalization::ZScore => { if artifact.normalization.len() != 2 { return Err(validation_error( path.clone(),
					"z-score state requires mean and variance tensors",
				)); }
			validate_f32_tensor( &artifact.normalization[0],
				&path.field("mean"),
				&shape, false, tensor_validation, )?; validate_f32_tensor( &artifact.normalization[1],
				&path.field("variance"),
				&shape, true, tensor_validation, ) }
		DenseDataNormalization::MinMax => { if artifact.normalization.len() != 2 { return Err(validation_error( path.clone(),
					"min-max state requires minimum and maximum tensors",
				)); }
			validate_f32_tensor( &artifact.normalization[0],
				&path.field("minimum"),
				&shape, false, tensor_validation, )?; validate_f32_tensor( &artifact.normalization[1],
				&path.field("maximum"),
				&shape, false, tensor_validation, )?; if tensor_validation == CheckpointTensorValidation::Payload {
				for (index, (minimum, maximum)) in f32_values(&artifact.normalization[0])
					.zip(f32_values(&artifact.normalization[1])) .enumerate()
				{ if minimum > maximum { return Err(validation_error(
							path.field("maximum").field("payload").index(index),
							format!("maximum {maximum} is below minimum {minimum}"),
						)); } } }
			Ok(()) }
		DenseDataNormalization::L2Norm => { if !artifact.normalization.is_empty() { return Err(validation_error( path.clone(),
					"l2 normalization must not retain fitted tensors",
				)); }
			Ok(()) } } }

fn validate_parameter( parameter: &CheckpointParameterImage, path: &CheckpointPath, shape: &[u64],
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> { validate_f32_tensor( &parameter.parameter,
		&path.field("parameter"),
		shape, false, tensor_validation, )?; validate_f32_tensor( &parameter.first_moment,
		&path.field("first-moment"),
		shape, false, tensor_validation, )?; validate_f32_tensor( &parameter.second_moment,
		&path.field("second-moment"),
		shape, true, tensor_validation, ) }

fn validate_prelu_parameters( operations: impl IntoIterator<Item = DenseOperation>,
	parameters: &[CheckpointParameterImage], path: &CheckpointPath, tensor_validation: CheckpointTensorValidation,
) -> CheckpointResult<()> { let expected = operations .into_iter()
		.filter(|operation| matches!(operation, DenseOperation::Activation(activation) if activation.learned_parameters() == 1))
		.count(); if parameters.len() != expected { return Err(validation_error( path.clone(), format!(
				"{} learned PReLU scalar parameters are saved, but {expected} PReLU operations are declared",
				parameters.len() ), )); }
	for (index, parameter) in parameters.iter().enumerate() {
		validate_parameter(parameter, &path.index(index), &[1], tensor_validation)?; }
	Ok(()) }

fn validate_f32_tensor( tensor: &CheckpointTensorImage, path: &CheckpointPath, expected_shape: &[u64],
	require_nonnegative: bool, tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> {
	if tensor.dtype != DType::F32 { return Err(validation_error(
			path.field("dtype"),
			format!("tensor dtype is {:?}, expected F32", tensor.dtype),
		)); }
	if tensor.shape != expected_shape { return Err(validation_error(
			path.field("shape"),
			format!(
				"tensor shape is {:?}, expected {expected_shape:?}",
				tensor.shape ), )); }
	let elements = expected_shape .iter() .try_fold(1u64, |elements, extent| { if *extent == 0 { None } else {
				elements.checked_mul(*extent) } }) .ok_or_else(|| { validation_error(
				path.field("shape"),
				"tensor extent product is zero or overflowed",
			) })?; let bytes = elements .checked_mul(4) .and_then(|bytes| usize::try_from(bytes).ok())
		.ok_or_else(|| validation_error(path.field("payload"), "tensor byte size overflowed usize"))?;
	if tensor_validation == CheckpointTensorValidation::Payload && tensor.bytes.len() != bytes {
		return Err(validation_error(
			path.field("payload"),
			format!(
				"tensor payload has {} bytes, expected {bytes}",
				tensor.bytes.len() ), )); }
	if tensor_validation == CheckpointTensorValidation::Payload { for (index, value) in f32_values(tensor).enumerate() {
			if !value.is_finite() { return Err(invalid_value(
					path.field("payload").index(index),
					format!("tensor state contains nonfinite value {value}"),
				)); }
			if require_nonnegative && value < 0.0 { return Err(invalid_value(
					path.field("payload").index(index),
					format!("tensor state contains negative value {value}"),
				)); } } }
	Ok(()) }

fn validate_i32_tensor( tensor: &CheckpointTensorImage, path: &CheckpointPath, expected_shape: &[u64],
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> { if tensor.dtype != DType::I32 {
		return Err(validation_error(
			path.field("dtype"),
			format!("tensor dtype is {:?}, expected I32", tensor.dtype),
		)); }
	if tensor.shape != expected_shape { return Err(validation_error(
			path.field("shape"),
			format!(
				"tensor shape is {:?}, expected {expected_shape:?}",
				tensor.shape ), )); }
	let bytes = expected_shape .iter() .try_fold(1u64, |elements, extent| { if *extent == 0 { None } else {
				elements.checked_mul(*extent) } }) .and_then(|elements| elements.checked_mul(4))
		.and_then(|bytes| usize::try_from(bytes).ok())
		.ok_or_else(|| validation_error(path.field("payload"), "tensor byte size overflowed usize"))?;
	if tensor_validation == CheckpointTensorValidation::Payload && tensor.bytes.len() != bytes {
		return Err(validation_error(
			path.field("payload"),
			format!(
				"tensor payload has {} bytes, expected {bytes}",
				tensor.bytes.len() ), )); }
	Ok(()) }

fn f32_values(tensor: &CheckpointTensorImage) -> impl Iterator<Item = f32> + '_ {
	tensor.bytes .chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("exact four-byte chunk")))
}

fn validate_temperature( artifact: &CheckpointArtifact, path: &CheckpointPath,
	tensor_validation: CheckpointTensorValidation, ) -> CheckpointResult<()> { match ( &artifact.temperature,
		artifact.bounds.calibration_iterations, ) { (None, 0) => Ok(()), (Some(_), 0) => Err(validation_error( path.clone(),
			"temperature tensor exists while calibration iteration count is zero",
		)), (None, _) => Err(validation_error( path.clone(),
			"calibration iterations exist without a final temperature tensor",
		)), (Some(temperature), _) => { if !matches!( artifact.config.loss, DenseLoss::BinaryCrossEntropy | DenseLoss::Focal
			) || !matches!(artifact.task, DenseTask::BinaryClassification { .. })
			{ return Err(validation_error( path.clone(),
					"temperature scaling is retained only for binary BCE or focal training",
				)); }
			validate_f32_tensor( temperature,
				&path.field("temperature"),
				&[1], true, tensor_validation, )?; if tensor_validation == CheckpointTensorValidation::Declaration { return Ok(());
			}
			let temperature_value = f32_values(temperature) .next()
				.expect("validated scalar tensor");
			if temperature_value == 0.0 { return Err(invalid_value(
					path.field("temperature").field("payload").index(0),
					"temperature must be positive",
				)); }
			Ok(()) } } }

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckpointTensor { value: ValueId, dtype: DType, shape: Vec<u64>, bytes: u64, }

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckpointParameter { parameter: CheckpointTensor, first_moment: CheckpointTensor,
	second_moment: CheckpointTensor, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointLayer { declaration: DenseLayer, weight: CheckpointParameter, bias: CheckpointParameter,
	prelu: Vec<CheckpointParameter>, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointConvolution { declaration: DenseConvolution, geometry: DenseConvolutionGeometry,
	weight: CheckpointParameter, bias: CheckpointParameter, prelu: Vec<CheckpointParameter>, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointKMeans { declaration: DenseKMeans, input_width: NonZeroU64, centroids: CheckpointTensor, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointEmbedding { declaration: DenseEmbedding, sequence_length: NonZeroU64,
	table: CheckpointParameter, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointAttention { declaration: DenseAttention, sequence_length: NonZeroU64,
	dimensions: NonZeroU64, head_dimension: NonZeroU64, query: CheckpointParameter, key: CheckpointParameter,
	value: CheckpointParameter, output: CheckpointParameter, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointRnn { declaration: DenseRnn, sequence_length: NonZeroU64, input_weight: CheckpointParameter,
	recurrent_weight: CheckpointParameter, bias: CheckpointParameter, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointGru { declaration: DenseGru, sequence_length: NonZeroU64,
	reset_input_weight: CheckpointParameter, reset_recurrent_weight: CheckpointParameter, reset_bias: CheckpointParameter,
	update_input_weight: CheckpointParameter, update_recurrent_weight: CheckpointParameter,
	update_bias: CheckpointParameter, candidate_input_weight: CheckpointParameter,
	candidate_recurrent_weight: CheckpointParameter, candidate_bias: CheckpointParameter, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointLstm { declaration: DenseLstm, sequence_length: NonZeroU64,
	input_gate_input_weight: CheckpointParameter, input_gate_recurrent_weight: CheckpointParameter,
	input_gate_bias: CheckpointParameter, forget_gate_input_weight: CheckpointParameter,
	forget_gate_recurrent_weight: CheckpointParameter, forget_gate_bias: CheckpointParameter,
	output_gate_input_weight: CheckpointParameter, output_gate_recurrent_weight: CheckpointParameter,
	output_gate_bias: CheckpointParameter, candidate_input_weight: CheckpointParameter,
	candidate_recurrent_weight: CheckpointParameter, candidate_bias: CheckpointParameter, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointTree { declaration: DenseTree, input_width: NonZeroU64, output_width: NonZeroU64,
	internal_nodes_per_tree: NonZeroU64, leaves_per_tree: NonZeroU64, split_features: CheckpointTensor,
	split_thresholds: CheckpointTensor, leaf_values: CheckpointParameter, }

#[derive(Clone, Debug, PartialEq, Eq)]
enum CheckpointResidualBranch { Layer(CheckpointLayer), Operation(DenseOperation), }

#[derive(Clone, Debug, PartialEq, Eq)]
enum CheckpointResidualSkip { Identity, Projection(CheckpointParameter), }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckpointResidual { branch: Vec<CheckpointResidualBranch>, branch_prelu: Vec<CheckpointParameter>,
	output_width: NonZeroU64, skip: CheckpointResidualSkip, operations: Vec<DenseOperation>,
	prelu: Vec<CheckpointParameter>, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckpointBlock { Embedding(CheckpointEmbedding), Attention(CheckpointAttention), Rnn(CheckpointRnn),
	Gru(CheckpointGru), Lstm(CheckpointLstm), Layer(CheckpointLayer), Convolution(CheckpointConvolution),
	Pool(CheckpointPoolImage), KMeans(CheckpointKMeans), Tree(CheckpointTree), Residual(CheckpointResidual), }

impl CheckpointBlock { const fn format_version(&self) -> u32 { match self {
			Self::Lstm(_) => LSTM_CHECKPOINT_FORMAT_VERSION, Self::Gru(_) => GRU_CHECKPOINT_FORMAT_VERSION,
			Self::Rnn(_) => RNN_CHECKPOINT_FORMAT_VERSION, Self::Embedding(_) => EMBEDDING_CHECKPOINT_FORMAT_VERSION,
			Self::Tree(_) => TREE_CHECKPOINT_FORMAT_VERSION, Self::KMeans(_) => KMEANS_CHECKPOINT_FORMAT_VERSION,
			_ => NATIVE_CHECKPOINT_FORMAT_VERSION, } } }

#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointManifest { format_version: u32, vectors: Vec<CheckpointVectorSchema>,
	feature_spans: Vec<CompiledFeatureSpan>, feature_width: usize, target_source_indices: Vec<usize>, task: DenseTask,
	output_adapter: Option<DenseOutputAdapter>, config: DenseTrainingConfig, bounds: TrainingBounds,
	normalization: Vec<CheckpointTensor>, layers: Vec<CheckpointLayer>, blocks: Vec<CheckpointBlock>,
	temperature: Option<CheckpointTensor>, program_digest: Digest, native: Option<CheckpointNativeRealization>, }

impl CheckpointManifest {
	#[inline]
	pub fn from_compiled(training: &CompiledTraining) -> CheckpointResult<Self> {
		let feature_width = training.dataset_schema().input_width();
		let feature_spans = training.dataset_schema().features().to_vec(); let vectors = training .dataset_schema() .vectors()
			.iter() .map(|vector| CheckpointVectorSchema { source_index: vector.source_index(), name: vector.name().to_vec(),
				role: vector.role(), semantic_type: vector.semantic_type(), encoding: vector.encoding(),
				metadata: vector.metadata().clone(), }) .collect(); let output = training.outputs();
		let normalization = match output.normalization { DataNormalizationState::Identity => Vec::new(),
			DataNormalizationState::ZScore(state) => { vec![ checkpoint_tensor(training, state.mean)?,
					checkpoint_tensor(training, state.variance)?, ] }
			DataNormalizationState::MinMax(state) => { vec![ checkpoint_tensor(training, state.minimum)?,
					checkpoint_tensor(training, state.maximum)?, ] }
			DataNormalizationState::L2Norm => Vec::new(), }; let mut config = training.config().clone(); config.layers.clear();
		let blocks = checkpoint_blocks(training, &output.blocks)?; let format_version = blocks .iter()
			.map(CheckpointBlock::format_version)
			.chain(training.dataset_schema().task().uses_target_matrix().then_some(MULTI_TARGET_CHECKPOINT_FORMAT_VERSION))
			.max() .unwrap_or(NATIVE_CHECKPOINT_FORMAT_VERSION); let temperature = output .validation .as_ref()
			.and_then(|validation| validation.temperature_scaling)
			.map(|state| checkpoint_tensor(training, state.updated_temperature)) .transpose()?; let manifest = Self {
			format_version, vectors, feature_spans, feature_width,
			target_source_indices: training.dataset_schema().targets().to_vec(), task: training.dataset_schema().task(),
			output_adapter: training.output_adapter(), config, bounds: training.bounds(), normalization, layers: Vec::new(),
			blocks, temperature, program_digest: compiled_training_program_digest(training)?, native: None, };
		validate_manifest_semantic_invariants(&manifest).map_err(manifest_semantic_error)?;
		manifest.validate_external_boundary(training)?; Ok(manifest) }

	#[must_use]
	#[inline]
	pub const fn format_version(&self) -> u32 { self.format_version }

	#[must_use]
	#[inline]
	pub fn vectors(&self) -> &[CheckpointVectorSchema] { &self.vectors }

	#[must_use]
	#[inline]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] { &self.feature_spans }

	#[must_use]
	#[inline]
	pub const fn feature_width(&self) -> usize { self.feature_width }

	#[must_use]
	#[inline]
	pub fn target_source_indices(&self) -> &[usize] { &self.target_source_indices }

	#[must_use]
	#[inline]
	pub const fn task(&self) -> DenseTask { self.task }

	#[must_use]
	#[inline]
	pub fn target_dtype(&self) -> Option<DType> { self.target_dtypes().next() }

	#[must_use]
	#[inline]
	pub fn target_dtypes(&self) -> impl Iterator<Item = DType> + '_ {
		self.target_source_indices .iter() .filter_map(|source_index| { self.vectors .iter()
					.find(|vector| vector.source_index == *source_index && vector.role == VectorRole::Target)
					.and_then(CheckpointVectorSchema::dtype) }) }

	#[must_use]
	#[inline]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> { self.output_adapter }

	fn tensors(&self) -> impl Iterator<Item = &CheckpointTensor> {
		let mut tensors = self.normalization.iter().collect::<Vec<_>>(); for parameter in manifest_resume_parameters(self) {
			push_checkpoint_parameter_tensors(&mut tensors, parameter); }
		for block in &self.blocks { match block { CheckpointBlock::KMeans(kmeans) => tensors.push(&kmeans.centroids),
				CheckpointBlock::Tree(tree) => tensors.extend([&tree.split_features, &tree.split_thresholds]),
				CheckpointBlock::Embedding(_)
				| CheckpointBlock::Attention(_)
				| CheckpointBlock::Rnn(_)
				| CheckpointBlock::Gru(_)
				| CheckpointBlock::Lstm(_)
				| CheckpointBlock::Layer(_)
				| CheckpointBlock::Convolution(_)
				| CheckpointBlock::Pool(_)
				| CheckpointBlock::Residual(_) => {} } }
		tensors.extend(self.temperature.iter()); tensors.into_iter() }

	fn validate_external_boundary(&self, training: &CompiledTraining) -> CheckpointResult<()> { let expected = self
			.tensors() .map(|tensor| tensor.value) .collect::<BTreeSet<_>>(); if expected.len() != self.tensors().count() {
			return Err(CheckpointError::manifest(
				"one logical value is assigned to multiple checkpoint roles",
			)); }
		let actual = training .graph() .tensors .iter() .filter(|tensor| tensor.external_output) .map(|tensor| tensor.id)
			.collect::<BTreeSet<_>>(); if expected != actual {
			let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
			let unexpected = actual.difference(&expected).copied().collect::<Vec<_>>();
			return Err(CheckpointError::manifest(format!(
				"checkpoint/output boundary differs (missing {missing:?}, unexpected {unexpected:?})"
			))); }
		Ok(()) } }

#[inline]
pub fn compiled_training_program_digest(training: &CompiledTraining) -> CheckpointResult<Digest> {
	let encoded = training .program() .to_ogdl()
		.map_err(|error| CheckpointError::manifest(format!("encode canonical training program: {error}")))?;
	Ok(Digest::new(Sha256::digest(encoded.as_bytes()).into())) }

#[inline]
pub fn apply_checkpoint_resume( training: &mut CompiledTraining, checkpoint: &CheckpointArtifact,
) -> CheckpointResult<()> { validate_artifact(checkpoint)?; let manifest = CheckpointManifest::from_compiled(training)?;
	validate_resume_compatibility(&manifest, checkpoint)?; let parameters = artifact_resume_parameters(checkpoint);
	let mut admitted = vec![0u8; parameters.len()]; let kmeans_centroids = checkpoint .blocks .iter() .enumerate()
		.filter_map(|(block, image)| match image { CheckpointBlockImage::KMeans(kmeans) => Some((block, &kmeans.centroids)),
			_ => None, }) .collect::<BTreeMap<_, _>>(); let tree_splits = checkpoint .blocks .iter() .enumerate()
		.filter_map(|(block, image)| match image {
			CheckpointBlockImage::Tree(tree) => Some((block, (tree.split_features(), tree.split_thresholds()))), _ => None, })
		.collect::<BTreeMap<_, _>>(); let mut admitted_kmeans = BTreeSet::new();
	let mut admitted_tree_features = BTreeSet::new(); let mut admitted_tree_thresholds = BTreeSet::new();
	let mut enabled_inputs = 0usize; for input in training.external_inputs_mut() {
		let (ordinal, component) = match input.role() { ExternalInputRole::ResumeEnabled => { enabled_inputs = enabled_inputs
						.checked_add(1) .ok_or_else(|| CheckpointError::IncompatibleResume {
							detail: "resume-enable input count overflowed".to_owned(),
						})?; if input.dtype() != DType::I32 || input.shape().extents() != [1] {
					return Err(CheckpointError::IncompatibleResume {
						detail: "compiled resume-enable input is not an int32 scalar".to_owned(),
					}); }
				input.replace_bytes(&1i32.to_le_bytes()); continue; }
			ExternalInputRole::ResumeKMeansCentroids { block } => { let image = kmeans_centroids .get(&block)
						.ok_or_else(|| CheckpointError::IncompatibleResume { detail: format!(
								"compiled resume input refers to absent K-means block {block}"
							), })?; if !admitted_kmeans.insert(block) { return Err(CheckpointError::IncompatibleResume { detail: format!(
							"compiled K-means centroid resume input for block {block} is duplicated"
						), }); }
				validate_resume_tensor(input, image, &format!("K-means block {block} centroids"))?;
				input.replace_bytes(image.bytes()); continue; }
			ExternalInputRole::ResumeTreeSplitFeatures { block } => { let (image, _) = tree_splits .get(&block)
						.ok_or_else(|| CheckpointError::IncompatibleResume {
							detail: format!("compiled resume input refers to absent tree block {block}"),
						})?; if !admitted_tree_features.insert(block) { return Err(CheckpointError::IncompatibleResume { detail: format!(
							"compiled tree split-feature resume input for block {block} is duplicated"
						), }); }
				validate_resume_tensor(input, image, &format!("tree block {block} split features"))?;
				input.replace_bytes(image.bytes()); continue; }
			ExternalInputRole::ResumeTreeSplitThresholds { block } => { let (_, image) = tree_splits .get(&block)
						.ok_or_else(|| CheckpointError::IncompatibleResume {
							detail: format!("compiled resume input refers to absent tree block {block}"),
						})?; if !admitted_tree_thresholds.insert(block) { return Err(CheckpointError::IncompatibleResume {
						detail: format!(
							"compiled tree split-threshold resume input for block {block} is duplicated"
						), }); }
				validate_resume_tensor( input, image,
					&format!("tree block {block} split thresholds"),
				)?; input.replace_bytes(image.bytes()); continue; }
			ExternalInputRole::ResumeParameter { ordinal } => (ordinal, 0u8),
			ExternalInputRole::ResumeFirstMoment { ordinal } => (ordinal, 1u8),
			ExternalInputRole::ResumeSecondMoment { ordinal } => (ordinal, 2u8), _ => continue, }; let parameter = parameters
			.get(ordinal) .ok_or_else(|| CheckpointError::IncompatibleResume {
				detail: format!("compiled resume input refers to absent parameter ordinal {ordinal}"),
			})?; let image = match component { 0 => parameter.parameter(), 1 => parameter.first_moment(),
			2 => parameter.second_moment(),
			_ => unreachable!("resume component is one of three fixed values"),
		}; let bit = 1u8 << component; if admitted[ordinal] & bit != 0 { return Err(CheckpointError::IncompatibleResume {
				detail: format!(
					"compiled resume input for parameter {ordinal} component {component} is duplicated"
				), }); }
		validate_resume_input(input, image, ordinal, component)?; input.replace_bytes(image.bytes());
		admitted[ordinal] |= bit; }
	if enabled_inputs != 1 { return Err(CheckpointError::IncompatibleResume {
			detail: format!("compiled program has {enabled_inputs} resume-enable inputs, expected one"),
		}); }
	if let Some((ordinal, mask)) = admitted .iter() .copied() .enumerate() .find(|(_, mask)| *mask != 0b111)
	{ return Err(CheckpointError::IncompatibleResume { detail: format!(
				"compiled parameter {ordinal} admits resume component mask {mask:#05b}, expected 0b111"
			), }); }
	let expected_kmeans = kmeans_centroids.keys().copied().collect::<BTreeSet<_>>();
	if admitted_kmeans != expected_kmeans { return Err(CheckpointError::IncompatibleResume { detail: format!(
				"compiled K-means centroid resume inputs are {admitted_kmeans:?}, expected {expected_kmeans:?}"
			), }); }
	let expected_trees = tree_splits.keys().copied().collect::<BTreeSet<_>>();
	if admitted_tree_features != expected_trees || admitted_tree_thresholds != expected_trees {
		return Err(CheckpointError::IncompatibleResume { detail: format!(
				"compiled tree split resume inputs are features {admitted_tree_features:?}, thresholds \
				 {admitted_tree_thresholds:?}, expected {expected_trees:?}"
			), }); }
	Ok(()) }

fn validate_resume_tensor( input: &OwnedExternalInput, image: &CheckpointTensorImage, role: &str,
) -> CheckpointResult<()> { let actual_bytes = u64::try_from(image.bytes().len()).unwrap_or(u64::MAX);
	let expected_bytes = input .shape() .bytes(input.dtype()) .map_err(|error| CheckpointError::IncompatibleResume {
			detail: format!("compiled {role} has invalid tensor shape: {error}"),
		})?; if input.dtype() != image.dtype()
		|| input.shape().extents() != image.shape()
		|| actual_bytes != expected_bytes.get()
	{ return Err(CheckpointError::IncompatibleResume { detail: format!(
				"{role} is {:?}{:?}/{} bytes, expected {:?}{:?}/{} bytes",
				image.dtype(), image.shape(), actual_bytes, input.dtype(), input.shape().extents(), expected_bytes.get() ), }); }
	Ok(()) }

fn validate_resume_input( input: &OwnedExternalInput, image: &CheckpointTensorImage, ordinal: usize, component: u8,
) -> CheckpointResult<()> { validate_resume_tensor( input, image,
		&format!("parameter {ordinal} component {component}"),
	) }

fn validate_resume_compatibility( manifest: &CheckpointManifest, checkpoint: &CheckpointArtifact,
) -> CheckpointResult<()> { let incompatible = |detail: String| CheckpointError::IncompatibleResume { detail };
	if manifest.feature_width != checkpoint.feature_width || manifest.feature_spans != checkpoint.feature_spans
		|| feature_normalization_mask(&manifest.feature_spans, manifest.feature_width)
			!= checkpoint.feature_normalization_mask
		|| manifest.target_source_indices != checkpoint.target_source_indices
		|| manifest.task != checkpoint.task
		|| manifest.output_adapter != checkpoint.output_adapter
	{ return Err(incompatible(
			"saved and current typed dataset shape, target semantics, or output adapter differ".to_owned(),
		)); }
	if manifest.vectors.len() != checkpoint.vectors.len() || manifest .vectors .iter() .zip(&checkpoint.vectors)
			.any(|(expected, actual)| { expected.source_index != actual.source_index || expected.name != actual.name
					|| expected.role != actual.role
					|| expected.semantic_type != actual.semantic_type
					|| expected.encoding != actual.encoding
					|| artifact_metadata(&expected.metadata) != actual.metadata }) { return Err(incompatible(
			"saved and current row-free vector schemas differ".to_owned(),
		)); }
	let expected = &manifest.config; let actual = &checkpoint.config; if expected.loss != actual.loss
		|| expected.data_normalization != actual.data_normalization
		|| expected.normalization_epsilon.to_bits() != actual.normalization_epsilon.to_bits()
		|| expected.adamw.beta_one.to_bits() != actual.adamw.beta_one.to_bits()
		|| expected.adamw.beta_two.to_bits() != actual.adamw.beta_two.to_bits()
		|| expected.adamw.epsilon.to_bits() != actual.adamw.epsilon.to_bits()
		|| expected.adamw.weight_decay.to_bits() != actual.adamw.weight_decay.to_bits()
	{ return Err(incompatible(
			"saved and current objective, normalization, or AdamW state semantics differ".to_owned(),
		)); }
	if !resume_topology_matches(manifest, checkpoint) { return Err(incompatible(
			"saved and current effective model topologies differ".to_owned(),
		)); }
	let expected_parameters = manifest_resume_parameters(manifest);
	let actual_parameters = artifact_resume_parameters(checkpoint);
	if expected_parameters.len() != actual_parameters.len() { return Err(incompatible(format!(
			"saved model has {} parameters, current topology has {}",
			actual_parameters.len(), expected_parameters.len() ))); }
	for (ordinal, (expected, actual)) in expected_parameters .into_iter() .zip(actual_parameters) .enumerate()
	{ for (component, (expected, actual)) in [ (&expected.parameter, actual.parameter()),
			(&expected.first_moment, actual.first_moment()), (&expected.second_moment, actual.second_moment()), ] .into_iter()
		.enumerate()
		{ if expected.dtype != actual.dtype() || expected.shape != actual.shape()
				|| expected.bytes != u64::try_from(actual.bytes().len()).unwrap_or(u64::MAX)
			{ return Err(incompatible(format!(
					"saved parameter {ordinal} component {component} tensor contract differs from the current topology"
				))); } } }
	for (block, (expected, actual)) in manifest.blocks.iter().zip(&checkpoint.blocks).enumerate() {
		let (CheckpointBlock::KMeans(expected), CheckpointBlockImage::KMeans(actual)) = (expected, actual) else { continue; };
		if expected.centroids.dtype != actual.centroids.dtype() || expected.centroids.shape != actual.centroids.shape()
			|| expected.centroids.bytes != u64::try_from(actual.centroids.bytes().len()).unwrap_or(u64::MAX)
		{ return Err(incompatible(format!(
				"saved K-means block {block} centroid tensor contract differs from the current topology"
			))); } }
	for (block, (expected, actual)) in manifest.blocks.iter().zip(&checkpoint.blocks).enumerate() {
		let (CheckpointBlock::Tree(expected), CheckpointBlockImage::Tree(actual)) = (expected, actual) else { continue; };
		for (name, expected, actual) in [ (
				"split-feature",
				&expected.split_features, &actual.split_features, ), (
				"split-threshold",
				&expected.split_thresholds, &actual.split_thresholds, ), ] { if expected.dtype != actual.dtype()
				|| expected.shape != actual.shape()
				|| expected.bytes != u64::try_from(actual.bytes().len()).unwrap_or(u64::MAX)
			{ return Err(incompatible(format!(
					"saved tree block {block} {name} tensor contract differs from the current topology"
				))); } } }
	Ok(()) }

fn resume_topology_matches(manifest: &CheckpointManifest, checkpoint: &CheckpointArtifact) -> bool {
	if manifest.format_version == FLAT_CHECKPOINT_FORMAT_VERSION { return manifest.layers.len() == checkpoint.layers.len()
			&& manifest .layers .iter() .zip(&checkpoint.layers)
				.all(|(expected, actual)| expected.declaration == actual.declaration); }
	manifest.blocks.len() == checkpoint.blocks.len() && manifest .blocks .iter() .zip(&checkpoint.blocks)
			.all(|(expected, actual)| match (expected, actual) {
				(CheckpointBlock::Embedding(expected), CheckpointBlockImage::Embedding(actual)) => {
					expected.declaration.dimensions() == actual.dimensions && expected.declaration.vocabulary() == actual.vocabulary
						&& expected.sequence_length == actual.sequence_length }
				(CheckpointBlock::Attention(expected), CheckpointBlockImage::Attention(actual)) => {
					expected.declaration.heads() == actual.heads && expected.sequence_length == actual.sequence_length
						&& expected.dimensions == actual.dimensions
						&& expected.head_dimension == actual.head_dimension }
				(CheckpointBlock::Rnn(expected), CheckpointBlockImage::Rnn(actual)) => {
					expected.declaration.width() == actual.width && expected.sequence_length == actual.sequence_length }
				(CheckpointBlock::Gru(expected), CheckpointBlockImage::Gru(actual)) => {
					expected.declaration.width() == actual.width && expected.sequence_length == actual.sequence_length }
				(CheckpointBlock::Lstm(expected), CheckpointBlockImage::Lstm(actual)) => {
					expected.declaration.width() == actual.width && expected.sequence_length == actual.sequence_length }
				(CheckpointBlock::Layer(expected), CheckpointBlockImage::Layer(actual)) => {
					expected.declaration == actual.declaration }
				(CheckpointBlock::Convolution(expected), CheckpointBlockImage::Convolution(actual)) => {
					expected.declaration == actual.declaration && expected.geometry == actual.geometry }
				(CheckpointBlock::Pool(expected), CheckpointBlockImage::Pool(actual)) => expected == actual,
				(CheckpointBlock::KMeans(expected), CheckpointBlockImage::KMeans(actual)) => {
					expected.declaration.clusters() == actual.clusters
						&& expected.declaration.group_to_neuron() == actual.group_to_neuron
						&& expected.input_width == actual.input_width }
				(CheckpointBlock::Tree(expected), CheckpointBlockImage::Tree(actual)) => {
					expected.declaration == actual.declaration && expected.input_width == actual.input_width
						&& expected.output_width == actual.output_width
						&& expected.internal_nodes_per_tree == actual.internal_nodes_per_tree
						&& expected.leaves_per_tree == actual.leaves_per_tree }
				(CheckpointBlock::Residual(expected), CheckpointBlockImage::Residual(actual)) => {
					expected.output_width == actual.output_width && expected.operations == actual.operations
						&& expected.branch.len() == actual.branch.len()
						&& expected .branch .iter() .zip(&actual.branch) .all(|(expected, actual)| match (expected, actual) { (
									CheckpointResidualBranch::Layer(expected), CheckpointResidualBranchImage::Layer(actual),
								) => expected.declaration == actual.declaration, ( CheckpointResidualBranch::Operation(expected),
									CheckpointResidualBranchImage::Operation(actual), ) => expected == actual, _ => false, }) && matches!(
						(&expected.skip, &actual.skip), ( CheckpointResidualSkip::Identity, CheckpointResidualSkipImage::Identity ) | (
							CheckpointResidualSkip::Projection(_), CheckpointResidualSkipImage::Projection(_) ) ) }
				_ => false, }) }

macro_rules! extend_block_parameters { ($parameters:ident, $blocks:expr, $block:ident, $branch:ident, $skip:ident) => {
		for block in $blocks { match block { $block::Embedding(embedding) => $parameters.push(&embedding.table),
				$block::Attention(attention) => $parameters.extend([ &attention.query, &attention.key, &attention.value,
					&attention.output, ]), $block::Rnn(rnn) => {
					$parameters.extend([&rnn.input_weight, &rnn.recurrent_weight, &rnn.bias]); }
				$block::Gru(gru) => $parameters.extend([ &gru.reset_input_weight, &gru.reset_recurrent_weight, &gru.reset_bias,
					&gru.update_input_weight, &gru.update_recurrent_weight, &gru.update_bias, &gru.candidate_input_weight,
					&gru.candidate_recurrent_weight, &gru.candidate_bias, ]), $block::Lstm(lstm) => $parameters.extend([
					&lstm.input_gate_input_weight, &lstm.input_gate_recurrent_weight, &lstm.input_gate_bias,
					&lstm.forget_gate_input_weight, &lstm.forget_gate_recurrent_weight, &lstm.forget_gate_bias,
					&lstm.output_gate_input_weight, &lstm.output_gate_recurrent_weight, &lstm.output_gate_bias,
					&lstm.candidate_input_weight, &lstm.candidate_recurrent_weight, &lstm.candidate_bias, ]), $block::Layer(layer) => {
					$parameters.extend([&layer.weight, &layer.bias]); $parameters.extend(&layer.prelu); }
				$block::Convolution(convolution) => { $parameters.extend([&convolution.weight, &convolution.bias]);
					$parameters.extend(&convolution.prelu); }
				$block::Pool(_) | $block::KMeans(_) => {}
				$block::Tree(tree) => $parameters.push(&tree.leaf_values), $block::Residual(residual) => {
					let mut branch_prelu = residual.branch_prelu.iter(); for step in &residual.branch { match step {
							$branch::Layer(layer) => { $parameters.extend([&layer.weight, &layer.bias]); $parameters.extend(&layer.prelu); }
							$branch::Operation(DenseOperation::Activation(activation)) if activation.learned_parameters() == 1 => {
								$parameters.extend(branch_prelu.next()); }
							$branch::Operation(_) => {} } }
					if let $skip::Projection(projection) = &residual.skip { $parameters.push(projection); }
					$parameters.extend(&residual.prelu); } } } }; }

fn manifest_resume_parameters(manifest: &CheckpointManifest) -> Vec<&CheckpointParameter> {
	let mut parameters = Vec::new(); if manifest.format_version == FLAT_CHECKPOINT_FORMAT_VERSION {
		for layer in &manifest.layers { parameters.extend([&layer.weight, &layer.bias]); parameters.extend(&layer.prelu); }
	} else { extend_block_parameters!( parameters, &manifest.blocks, CheckpointBlock, CheckpointResidualBranch,
			CheckpointResidualSkip ); }
	parameters }

fn artifact_resume_parameters(checkpoint: &CheckpointArtifact) -> Vec<&CheckpointParameterImage> {
	let mut parameters = Vec::new(); if checkpoint.format_version == FLAT_CHECKPOINT_FORMAT_VERSION {
		for layer in &checkpoint.layers { parameters.extend([&layer.weight, &layer.bias]); parameters.extend(&layer.prelu); }
	} else { extend_block_parameters!( parameters, &checkpoint.blocks, CheckpointBlockImage, CheckpointResidualBranchImage,
			CheckpointResidualSkipImage ); }
	parameters }

fn validate_manifest_semantic_invariants(manifest: &CheckpointManifest) -> CheckpointResult<()> {
	let artifact = declaration_artifact_from_manifest(manifest); validate_checkpoint_semantic_invariants( &artifact,
		CheckpointTopologyStorage::Manifest, CheckpointTensorValidation::Declaration, ) }

fn manifest_semantic_error(error: CheckpointError) -> CheckpointError { match error {
		CheckpointError::Decode(error) => CheckpointError::manifest(error.to_string()), other => other, } }

pub(crate) fn checkpoint_layer( training: &CompiledTraining, declaration: DenseLayer, state: DenseLayerState,
) -> CheckpointResult<CheckpointLayer> { Ok(CheckpointLayer { declaration,
		weight: checkpoint_parameter(training, state.weight)?, bias: checkpoint_parameter(training, state.bias)?, prelu: state
			.prelu .into_iter() .map(|parameter| checkpoint_parameter(training, parameter))
			.collect::<CheckpointResult<Vec<_>>>()?, }) }

pub(crate) fn checkpoint_convolution( training: &CompiledTraining, declaration: DenseConvolution,
	state: DenseConvolutionState, ) -> CheckpointResult<CheckpointConvolution> { Ok(CheckpointConvolution { declaration,
		geometry: state.geometry, weight: checkpoint_parameter(training, state.weight)?,
		bias: checkpoint_parameter(training, state.bias)?, prelu: state .prelu .into_iter()
			.map(|parameter| checkpoint_parameter(training, parameter)) .collect::<CheckpointResult<Vec<_>>>()?, }) }

fn checkpoint_blocks( training: &CompiledTraining, states: &[DenseBlockState],
) -> CheckpointResult<Vec<CheckpointBlock>> { states.iter() .enumerate()
		.map(|(index, state)| state.realized().checkpoint(training, index)) .collect() }

pub(crate) fn checkpoint_pool( declaration: DensePool, state: DensePoolState, index: usize,
) -> CheckpointResult<CheckpointBlock> { checkpoint_pool_image( declaration, state, &CheckpointPath::root()
			.field("recipe")
			.field("model")
			.field("blocks")
			.index(index), ) .map(CheckpointBlock::Pool) .map_err(manifest_semantic_error) }

pub(crate) fn checkpoint_embedding( training: &CompiledTraining, declaration: DenseEmbedding,
	state: DenseEmbeddingState, ) -> CheckpointResult<CheckpointEmbedding> {
	if declaration.dimensions() != state.dimensions || declaration.vocabulary() != state.vocabulary {
		return Err(CheckpointError::manifest(
			"embedding declaration differs from its learned table state",
		)); }
	Ok(CheckpointEmbedding { declaration, sequence_length: state.sequence_length,
		table: checkpoint_parameter(training, state.table)?, }) }

pub(crate) fn checkpoint_attention( training: &CompiledTraining, declaration: DenseAttention,
	state: DenseAttentionState, ) -> CheckpointResult<CheckpointAttention> { if declaration.heads() != state.heads
		|| state.heads.get().checked_mul(state.head_dimension.get()) != Some(state.dimensions.get())
	{ return Err(CheckpointError::manifest(
			"attention declaration differs from its learned projection state",
		)); }
	Ok(CheckpointAttention { declaration, sequence_length: state.sequence_length, dimensions: state.dimensions,
		head_dimension: state.head_dimension, query: checkpoint_parameter(training, state.query)?,
		key: checkpoint_parameter(training, state.key)?, value: checkpoint_parameter(training, state.value)?,
		output: checkpoint_parameter(training, state.output)?, }) }

pub(crate) fn checkpoint_rnn( training: &CompiledTraining, declaration: DenseRnn, state: DenseRnnState,
) -> CheckpointResult<CheckpointRnn> { if declaration.width() != state.width { return Err(CheckpointError::manifest(
			"RNN declaration differs from its learned parameter state",
		)); }
	Ok(CheckpointRnn { declaration, sequence_length: state.sequence_length,
		input_weight: checkpoint_parameter(training, state.input_weight)?,
		recurrent_weight: checkpoint_parameter(training, state.recurrent_weight)?,
		bias: checkpoint_parameter(training, state.bias)?, }) }

pub(crate) fn checkpoint_gru( training: &CompiledTraining, declaration: DenseGru, state: DenseGruState,
) -> CheckpointResult<CheckpointGru> { if declaration.width() != state.width { return Err(CheckpointError::manifest(
			"GRU declaration differs from its learned parameter state",
		)); }
	Ok(CheckpointGru { declaration, sequence_length: state.sequence_length,
		reset_input_weight: checkpoint_parameter(training, state.reset_input_weight)?,
		reset_recurrent_weight: checkpoint_parameter(training, state.reset_recurrent_weight)?,
		reset_bias: checkpoint_parameter(training, state.reset_bias)?,
		update_input_weight: checkpoint_parameter(training, state.update_input_weight)?,
		update_recurrent_weight: checkpoint_parameter(training, state.update_recurrent_weight)?,
		update_bias: checkpoint_parameter(training, state.update_bias)?,
		candidate_input_weight: checkpoint_parameter(training, state.candidate_input_weight)?,
		candidate_recurrent_weight: checkpoint_parameter(training, state.candidate_recurrent_weight)?,
		candidate_bias: checkpoint_parameter(training, state.candidate_bias)?, }) }

pub(crate) fn checkpoint_lstm( training: &CompiledTraining, declaration: DenseLstm, state: DenseLstmState,
) -> CheckpointResult<CheckpointLstm> { if declaration.width() != state.width { return Err(CheckpointError::manifest(
			"LSTM declaration differs from its learned parameter state",
		)); }
	Ok(CheckpointLstm { declaration, sequence_length: state.sequence_length,
		input_gate_input_weight: checkpoint_parameter(training, state.input_gate_input_weight)?,
		input_gate_recurrent_weight: checkpoint_parameter(training, state.input_gate_recurrent_weight)?,
		input_gate_bias: checkpoint_parameter(training, state.input_gate_bias)?,
		forget_gate_input_weight: checkpoint_parameter(training, state.forget_gate_input_weight)?,
		forget_gate_recurrent_weight: checkpoint_parameter(training, state.forget_gate_recurrent_weight)?,
		forget_gate_bias: checkpoint_parameter(training, state.forget_gate_bias)?,
		output_gate_input_weight: checkpoint_parameter(training, state.output_gate_input_weight)?,
		output_gate_recurrent_weight: checkpoint_parameter(training, state.output_gate_recurrent_weight)?,
		output_gate_bias: checkpoint_parameter(training, state.output_gate_bias)?,
		candidate_input_weight: checkpoint_parameter(training, state.candidate_input_weight)?,
		candidate_recurrent_weight: checkpoint_parameter(training, state.candidate_recurrent_weight)?,
		candidate_bias: checkpoint_parameter(training, state.candidate_bias)?, }) }

pub(crate) fn checkpoint_kmeans( training: &CompiledTraining, declaration: DenseKMeans, state: DenseKMeansState,
) -> CheckpointResult<CheckpointKMeans> { if declaration.clusters() != state.clusters {
		return Err(CheckpointError::manifest(
			"K-means declaration cluster count differs from its saved state",
		)); }
	Ok(CheckpointKMeans { declaration, input_width: state.input_width,
		centroids: checkpoint_tensor(training, state.updated_centroids)?, }) }

pub(crate) fn checkpoint_tree( training: &CompiledTraining, declaration: DenseTree, state: DenseTreeState,
) -> CheckpointResult<CheckpointTree> { if declaration != state.declaration { return Err(CheckpointError::manifest(
			"tree declaration differs from its saved state",
		)); }
	Ok(CheckpointTree { declaration, input_width: state.input_width, output_width: state.output_width,
		internal_nodes_per_tree: state.internal_nodes_per_tree, leaves_per_tree: state.leaves_per_tree,
		split_features: checkpoint_tensor(training, state.split_features)?,
		split_thresholds: checkpoint_tensor(training, state.split_thresholds)?,
		leaf_values: checkpoint_parameter(training, state.leaf_values)?, }) }

pub(crate) fn checkpoint_residual( training: &CompiledTraining, declaration: &DenseResidual, state: &DenseResidualState,
) -> CheckpointResult<CheckpointResidual> { let output_width = declaration.output_width().ok_or_else(|| {
		CheckpointError::manifest("residual checkpoint declaration has no width-producing branch layer")
	})?; let declared_layer_count = declaration .branch() .iter()
		.filter(|step| matches!(step, DenseResidualOperation::Layer(_))) .count();
	if declared_layer_count != state.branch.len() { return Err(CheckpointError::manifest(format!(
			"residual branch declares {declared_layer_count} layers but produced {} layer states",
			state.branch.len() ))); }
	let mut state_index = 0usize; let mut branch = Vec::with_capacity(declaration.branch().len());
	for step in declaration.branch() { match step { DenseResidualOperation::Layer(layer) => {
				let layer_state = state.branch.get(state_index).cloned().ok_or_else(|| {
					CheckpointError::manifest("residual branch layer state disappeared during traversal")
				})?; state_index += 1; branch.push(CheckpointResidualBranch::Layer(checkpoint_layer( training, layer.clone(),
					layer_state, )?)); }
			DenseResidualOperation::Operation(operation) => { branch.push(CheckpointResidualBranch::Operation(*operation)); } } }
	let skip = match state.projection {
		Some(projection) => CheckpointResidualSkip::Projection(checkpoint_parameter(training, projection)?),
		None => CheckpointResidualSkip::Identity, }; Ok(CheckpointResidual { branch, branch_prelu: state .branch_prelu .iter()
			.copied() .map(|parameter| checkpoint_parameter(training, parameter)) .collect::<CheckpointResult<Vec<_>>>()?,
		output_width, skip, operations: declaration.operations().to_vec(), prelu: state .prelu .iter() .copied()
			.map(|parameter| checkpoint_parameter(training, parameter)) .collect::<CheckpointResult<Vec<_>>>()?, }) }

fn push_checkpoint_parameter_tensors<'a>(tensors: &mut Vec<&'a CheckpointTensor>, parameter: &'a CheckpointParameter) {
	tensors.extend([ &parameter.parameter, &parameter.first_moment, &parameter.second_moment, ]); }

#[derive(Clone, Debug)]
pub struct CompletedTrainingCheckpoint { execution: CompletedTrainingExecution, manifest: CheckpointManifest,
	output_indices: BTreeMap<ValueId, usize>, }

impl CompletedTrainingCheckpoint {
	#[inline]
	pub fn new(execution: CompletedTrainingExecution, mut manifest: CheckpointManifest) -> CheckpointResult<Self> {
		manifest.native = Some(checkpoint_native_realization( execution.native_kernels(), manifest.program_digest, )?);
		validate_manifest_semantic_invariants(&manifest).map_err(manifest_semantic_error)?; let output_indices = map_outputs(
			&manifest, execution.external_outputs(), execution.external_output_values(), )?; Ok(Self { execution, manifest,
			output_indices, }) }

	#[must_use]
	#[inline]
	pub const fn run(&self) -> RunId { self.execution.run() }

	#[must_use]
	#[inline]
	pub const fn bundle(&self) -> BundleIdentity { self.execution.bundle() }

	#[must_use]
	#[inline]
	pub fn external_outputs(&self) -> &[ExitImage] { self.execution.external_outputs() }

	#[must_use]
	#[inline]
	pub fn metrics(&self) -> &[FinalTrainingMetric] { self.execution.metrics() }

	#[must_use]
	#[inline]
	pub const fn native_kernels(&self) -> &RealizedNativeKernelSet { self.execution.native_kernels() }

	#[must_use]
	#[inline]
	pub const fn native_evidence(&self) -> &recipe_native_executor::NativeExecutionEvidence { self.execution.native_evidence() }

	#[must_use]
	#[inline]
	pub const fn training_evidence(&self) -> &crate::TrainingExecutionEvidence { self.execution.training_evidence() }

	#[must_use]
	#[inline]
	pub const fn journal(&self) -> &RunJournal { self.execution.journal() }

	#[must_use]
	#[inline]
	pub const fn manifest(&self) -> &CheckpointManifest { &self.manifest }


	#[inline]
	pub fn save(&self, path: impl AsRef<Path>) -> CheckpointResult<()> { let path = path.as_ref();
		let outputs = self.output_bytes(); let encoded_bytes = encoded_size(&self.manifest, &outputs)?;
		atomic_save(path, encoded_bytes, |file| { encode_checkpoint(&self.manifest, &outputs, file) }) }

	#[inline]
	pub fn save_native_kernel(&self, path: impl AsRef<Path>, format: NativeKernelFormat) -> CheckpointResult<()> {
		let path = path.as_ref(); if path.extension().and_then(|extension| extension.to_str()) != Some(format.extension()) {
			return Err(CheckpointError::invalid_target( path,
				format!("native kernel path must end in .{}", format.extension()),
			)); }
		let matching = self .native_kernels() .kernels() .iter() .filter(|kernel| kernel.format() == format)
			.collect::<Vec<_>>(); let kernel = match matching.as_slice() {
			[] => return Err(CheckpointError::NativeKernelUnavailable { requested: format }), [kernel] => *kernel, kernels => {
				return Err(CheckpointError::NativeKernelAmbiguous { requested: format, images: kernels.len(), }); } };
		let encoded_bytes = u64::try_from(kernel.bytes().len())
			.map_err(|_| CheckpointError::invalid_target(path, "native kernel size exceeds u64"))?;
		atomic_save(path, encoded_bytes, |file| file.write_all(kernel.bytes())) }

	fn output_bytes(&self) -> BTreeMap<ValueId, &[u8]> { self.output_indices .iter() .map(|(value, index)| { ( *value,
					self.execution.external_outputs()[*index].bytes.as_slice(), ) }) .collect() } }

fn checkpoint_native_realization( realized: &RealizedNativeKernelSet, program: Digest,
) -> CheckpointResult<CheckpointNativeRealization> { let kernels = realized .kernels() .iter()
		.map(|kernel| CheckpointNativeKernel { format: kernel.format(), target: kernel.target().clone(),
			toolchain: kernel.toolchain().clone(), digest: kernel.digest(), }) .collect::<Vec<_>>(); if kernels.is_empty() {
		return Err(CheckpointError::manifest(
			"completed native realization contains no kernel images",
		)); }
	Ok(CheckpointNativeRealization { program, realization: realized.realization(), topology: realized.topology(),
		discovery: realized.discovery(), kernels, }) }

fn checkpoint_parameter(training: &CompiledTraining, state: ParameterState) -> CheckpointResult<CheckpointParameter> {
	Ok(CheckpointParameter { parameter: checkpoint_tensor(training, state.updated_parameter)?,
		first_moment: checkpoint_tensor(training, state.updated_first_moment)?,
		second_moment: checkpoint_tensor(training, state.updated_second_moment)?, }) }

fn checkpoint_tensor(training: &CompiledTraining, value: ValueId) -> CheckpointResult<CheckpointTensor> {
	let tensor = training .graph() .tensors .iter() .find(|tensor| tensor.id == value) .ok_or_else(|| {
			CheckpointError::manifest(format!(
				"checkpoint value {value} has no tensor declaration"
			)) })?; if !tensor.external_output { return Err(CheckpointError::manifest(format!(
			"checkpoint value {value} is not an external output"
		))); }
	Ok(CheckpointTensor { value, dtype: tensor.dtype, shape: tensor.shape.extents().to_vec(),
		bytes: tensor.storage_bytes.get(), }) }

fn map_outputs( manifest: &CheckpointManifest, outputs: &[ExitImage], logical_values: &[ValueId],
) -> CheckpointResult<BTreeMap<ValueId, usize>> { if outputs.len() != logical_values.len() {
		return Err(CheckpointError::manifest(format!(
			"execution returned {} physical checkpoint images but {} logical output identities",
			outputs.len(), logical_values.len() ))); }
	let expected = manifest .tensors() .map(|tensor| (tensor.value, tensor)) .collect::<BTreeMap<_, _>>();
	let mut mapped = BTreeMap::new(); for (index, (output, value)) in outputs.iter().zip(logical_values).enumerate() {
		let value = *value; let Some(tensor) = expected.get(&value) else {
			return Err(CheckpointError::UnexpectedOutput { value }); }; if mapped.insert(value, index).is_some() {
			return Err(CheckpointError::DuplicateOutput { value }); }
		if output.source.dtype != tensor.dtype { return Err(CheckpointError::OutputDTypeMismatch { value,
				expected: tensor.dtype, actual: output.source.dtype, }); }
		let actual = u64::try_from(output.bytes.len()).unwrap_or(u64::MAX);
		if output.source.bytes.get() != tensor.bytes || actual != tensor.bytes {
			return Err(CheckpointError::OutputSizeMismatch { value, expected: tensor.bytes, actual, }); } }
	for value in expected.keys() { if !mapped.contains_key(value) {
			return Err(CheckpointError::MissingOutput { value: *value }); } }
	Ok(mapped) }

fn encoded_size(manifest: &CheckpointManifest, outputs: &BTreeMap<ValueId, &[u8]>) -> CheckpointResult<u64> {
	let mut counter = CountingWriter::default(); encode_checkpoint(manifest, outputs, &mut counter)
		.map_err(|error| CheckpointError::io("measure checkpoint", Path::new("<memory>"), error))?;
	Ok(counter.bytes) }

fn encode_checkpoint( manifest: &CheckpointManifest, outputs: &BTreeMap<ValueId, &[u8]>, writer: &mut impl Write,
) -> io::Result<()> { validate_manifest_semantic_invariants(manifest).map_err(checkpoint_validation_io_error)?;
	let artifact = artifact_from_manifest(manifest, outputs)?;
	validate_artifact(&artifact).map_err(checkpoint_validation_io_error)?; encode_artifact(&artifact, writer) }

fn checkpoint_validation_io_error(error: CheckpointError) -> io::Error { io::Error::new(io::ErrorKind::InvalidData, error.to_string()) }

fn declaration_artifact_from_manifest(manifest: &CheckpointManifest) -> CheckpointArtifact { CheckpointArtifact {
		format_version: manifest.format_version, vectors: manifest .vectors .iter() .map(|vector| CheckpointArtifactVector {
				source_index: vector.source_index, name: vector.name.clone(), role: vector.role,
				semantic_type: vector.semantic_type, encoding: vector.encoding, metadata: artifact_metadata(&vector.metadata), })
			.collect(), feature_spans: manifest.feature_spans.clone(),
		feature_normalization_mask: feature_normalization_mask(&manifest.feature_spans, manifest.feature_width),
		feature_width: manifest.feature_width, target_source_indices: manifest.target_source_indices.clone(),
		task: manifest.task, output_adapter: manifest.output_adapter, config: manifest.config.clone(),
		bounds: manifest.bounds, normalization: manifest .normalization .iter() .map(declaration_tensor_image) .collect(),
		layers: manifest .layers .iter() .map(declaration_layer_image) .collect(), blocks: manifest .blocks .iter()
			.map(declaration_block_image) .collect(), temperature: manifest.temperature.as_ref().map(declaration_tensor_image),
		native: manifest.native.clone(), } }

fn declaration_tensor_image(tensor: &CheckpointTensor) -> CheckpointTensorImage { CheckpointTensorImage {
		dtype: tensor.dtype, shape: tensor.shape.clone(), bytes: Vec::new(), } }

fn declaration_parameter_image(parameter: &CheckpointParameter) -> CheckpointParameterImage { CheckpointParameterImage {
		parameter: declaration_tensor_image(&parameter.parameter),
		first_moment: declaration_tensor_image(&parameter.first_moment),
		second_moment: declaration_tensor_image(&parameter.second_moment), } }

fn declaration_layer_image(layer: &CheckpointLayer) -> CheckpointLayerImage { CheckpointLayerImage {
		declaration: layer.declaration.clone(), weight: declaration_parameter_image(&layer.weight),
		bias: declaration_parameter_image(&layer.bias), prelu: layer .prelu .iter() .map(declaration_parameter_image)
			.collect(), } }

fn declaration_convolution_image(convolution: &CheckpointConvolution) -> CheckpointConvolutionImage {
	CheckpointConvolutionImage { declaration: convolution.declaration.clone(), geometry: convolution.geometry,
		weight: declaration_parameter_image(&convolution.weight), bias: declaration_parameter_image(&convolution.bias),
		prelu: convolution .prelu .iter() .map(declaration_parameter_image) .collect(), } }

fn declaration_kmeans_image(kmeans: &CheckpointKMeans) -> CheckpointKMeansImage { CheckpointKMeansImage {
		clusters: kmeans.declaration.clusters(), group_to_neuron: kmeans.declaration.group_to_neuron(),
		input_width: kmeans.input_width, centroids: declaration_tensor_image(&kmeans.centroids), } }

fn declaration_embedding_image(embedding: &CheckpointEmbedding) -> CheckpointEmbeddingImage { CheckpointEmbeddingImage {
		dimensions: embedding.declaration.dimensions(), vocabulary: embedding.declaration.vocabulary(),
		sequence_length: embedding.sequence_length, table: declaration_parameter_image(&embedding.table), } }

fn declaration_attention_image(attention: &CheckpointAttention) -> CheckpointAttentionImage { CheckpointAttentionImage {
		sequence_length: attention.sequence_length, dimensions: attention.dimensions, heads: attention.declaration.heads(),
		head_dimension: attention.head_dimension, query: declaration_parameter_image(&attention.query),
		key: declaration_parameter_image(&attention.key), value: declaration_parameter_image(&attention.value),
		output: declaration_parameter_image(&attention.output), } }

fn declaration_rnn_image(rnn: &CheckpointRnn) -> CheckpointRnnImage { CheckpointRnnImage {
		sequence_length: rnn.sequence_length, width: rnn.declaration.width(),
		input_weight: declaration_parameter_image(&rnn.input_weight),
		recurrent_weight: declaration_parameter_image(&rnn.recurrent_weight), bias: declaration_parameter_image(&rnn.bias), }
}

fn declaration_gru_image(gru: &CheckpointGru) -> CheckpointGruImage { CheckpointGruImage {
		sequence_length: gru.sequence_length, width: gru.declaration.width(),
		reset_input_weight: declaration_parameter_image(&gru.reset_input_weight),
		reset_recurrent_weight: declaration_parameter_image(&gru.reset_recurrent_weight),
		reset_bias: declaration_parameter_image(&gru.reset_bias),
		update_input_weight: declaration_parameter_image(&gru.update_input_weight),
		update_recurrent_weight: declaration_parameter_image(&gru.update_recurrent_weight),
		update_bias: declaration_parameter_image(&gru.update_bias),
		candidate_input_weight: declaration_parameter_image(&gru.candidate_input_weight),
		candidate_recurrent_weight: declaration_parameter_image(&gru.candidate_recurrent_weight),
		candidate_bias: declaration_parameter_image(&gru.candidate_bias), } }

fn declaration_lstm_image(lstm: &CheckpointLstm) -> CheckpointLstmImage { CheckpointLstmImage {
		sequence_length: lstm.sequence_length, width: lstm.declaration.width(),
		input_gate_input_weight: declaration_parameter_image(&lstm.input_gate_input_weight),
		input_gate_recurrent_weight: declaration_parameter_image(&lstm.input_gate_recurrent_weight),
		input_gate_bias: declaration_parameter_image(&lstm.input_gate_bias),
		forget_gate_input_weight: declaration_parameter_image(&lstm.forget_gate_input_weight),
		forget_gate_recurrent_weight: declaration_parameter_image(&lstm.forget_gate_recurrent_weight),
		forget_gate_bias: declaration_parameter_image(&lstm.forget_gate_bias),
		output_gate_input_weight: declaration_parameter_image(&lstm.output_gate_input_weight),
		output_gate_recurrent_weight: declaration_parameter_image(&lstm.output_gate_recurrent_weight),
		output_gate_bias: declaration_parameter_image(&lstm.output_gate_bias),
		candidate_input_weight: declaration_parameter_image(&lstm.candidate_input_weight),
		candidate_recurrent_weight: declaration_parameter_image(&lstm.candidate_recurrent_weight),
		candidate_bias: declaration_parameter_image(&lstm.candidate_bias), } }

fn declaration_tree_image(tree: &CheckpointTree) -> CheckpointTreeImage { CheckpointTreeImage {
		declaration: tree.declaration, input_width: tree.input_width, output_width: tree.output_width,
		internal_nodes_per_tree: tree.internal_nodes_per_tree, leaves_per_tree: tree.leaves_per_tree,
		split_features: declaration_tensor_image(&tree.split_features),
		split_thresholds: declaration_tensor_image(&tree.split_thresholds),
		leaf_values: declaration_parameter_image(&tree.leaf_values), } }

fn declaration_block_image(block: &CheckpointBlock) -> CheckpointBlockImage { match block {
		CheckpointBlock::Embedding(embedding) => { CheckpointBlockImage::Embedding(declaration_embedding_image(embedding)) }
		CheckpointBlock::Attention(attention) => { CheckpointBlockImage::Attention(declaration_attention_image(attention)) }
		CheckpointBlock::Rnn(rnn) => CheckpointBlockImage::Rnn(declaration_rnn_image(rnn)),
		CheckpointBlock::Gru(gru) => CheckpointBlockImage::Gru(declaration_gru_image(gru)),
		CheckpointBlock::Lstm(lstm) => CheckpointBlockImage::Lstm(declaration_lstm_image(lstm)),
		CheckpointBlock::Layer(layer) => CheckpointBlockImage::Layer(declaration_layer_image(layer)),
		CheckpointBlock::Convolution(convolution) => {
			CheckpointBlockImage::Convolution(declaration_convolution_image(convolution)) }
		CheckpointBlock::Pool(pool) => CheckpointBlockImage::Pool(*pool),
		CheckpointBlock::KMeans(kmeans) => CheckpointBlockImage::KMeans(declaration_kmeans_image(kmeans)),
		CheckpointBlock::Tree(tree) => CheckpointBlockImage::Tree(declaration_tree_image(tree)),
		CheckpointBlock::Residual(residual) => CheckpointBlockImage::Residual(CheckpointResidualImage { branch: residual
				.branch .iter() .map(|step| match step { CheckpointResidualBranch::Layer(layer) => {
						CheckpointResidualBranchImage::Layer(declaration_layer_image(layer)) }
					CheckpointResidualBranch::Operation(operation) => { CheckpointResidualBranchImage::Operation(*operation) } })
				.collect(), branch_prelu: residual .branch_prelu .iter() .map(declaration_parameter_image) .collect(),
			output_width: residual.output_width, skip: match &residual.skip {
				CheckpointResidualSkip::Identity => CheckpointResidualSkipImage::Identity,
				CheckpointResidualSkip::Projection(projection) => {
					CheckpointResidualSkipImage::Projection(declaration_parameter_image(projection)) } },
			operations: residual.operations.clone(), prelu: residual .prelu .iter() .map(declaration_parameter_image) .collect(),
		}), } }

fn artifact_from_manifest( manifest: &CheckpointManifest, outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointArtifact> { let layers = manifest .layers .iter() .map(|layer| artifact_layer(layer, outputs))
		.collect::<io::Result<Vec<_>>>()?; let blocks = if manifest.format_version == FLAT_CHECKPOINT_FORMAT_VERSION {
		layers.iter() .cloned() .map(CheckpointBlockImage::Layer) .collect() } else { manifest .blocks .iter()
			.map(|block| artifact_block(block, outputs)) .collect::<io::Result<Vec<_>>>()? }; Ok(CheckpointArtifact {
		format_version: manifest.format_version, vectors: manifest .vectors .iter() .map(|vector| CheckpointArtifactVector {
				source_index: vector.source_index, name: vector.name.clone(), role: vector.role,
				semantic_type: vector.semantic_type, encoding: vector.encoding, metadata: artifact_metadata(&vector.metadata), })
			.collect(), feature_spans: manifest.feature_spans.clone(),
		feature_normalization_mask: feature_normalization_mask(&manifest.feature_spans, manifest.feature_width),
		feature_width: manifest.feature_width, target_source_indices: manifest.target_source_indices.clone(),
		task: manifest.task, output_adapter: manifest.output_adapter, config: manifest.config.clone(),
		bounds: manifest.bounds, normalization: manifest .normalization .iter()
			.map(|tensor| artifact_tensor(tensor, outputs)) .collect::<io::Result<_>>()?, layers, blocks, temperature: manifest
			.temperature .as_ref() .map(|tensor| artifact_tensor(tensor, outputs)) .transpose()?,
		native: manifest.native.clone(), }) }

fn artifact_layer(layer: &CheckpointLayer, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointLayerImage> {
	Ok(CheckpointLayerImage { declaration: layer.declaration.clone(), weight: artifact_parameter(&layer.weight, outputs)?,
		bias: artifact_parameter(&layer.bias, outputs)?, prelu: layer .prelu .iter()
			.map(|parameter| artifact_parameter(parameter, outputs)) .collect::<io::Result<Vec<_>>>()?, }) }

fn artifact_convolution( convolution: &CheckpointConvolution, outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointConvolutionImage> { Ok(CheckpointConvolutionImage {
		declaration: convolution.declaration.clone(), geometry: convolution.geometry,
		weight: artifact_parameter(&convolution.weight, outputs)?, bias: artifact_parameter(&convolution.bias, outputs)?,
		prelu: convolution .prelu .iter() .map(|parameter| artifact_parameter(parameter, outputs))
			.collect::<io::Result<Vec<_>>>()?, }) }

fn artifact_kmeans(kmeans: &CheckpointKMeans, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointKMeansImage> {
	Ok(CheckpointKMeansImage { clusters: kmeans.declaration.clusters(),
		group_to_neuron: kmeans.declaration.group_to_neuron(), input_width: kmeans.input_width,
		centroids: artifact_tensor(&kmeans.centroids, outputs)?, }) }

fn artifact_embedding( embedding: &CheckpointEmbedding, outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointEmbeddingImage> { Ok(CheckpointEmbeddingImage {
		dimensions: embedding.declaration.dimensions(), vocabulary: embedding.declaration.vocabulary(),
		sequence_length: embedding.sequence_length, table: artifact_parameter(&embedding.table, outputs)?, }) }

fn artifact_attention( attention: &CheckpointAttention, outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointAttentionImage> { Ok(CheckpointAttentionImage { sequence_length: attention.sequence_length,
		dimensions: attention.dimensions, heads: attention.declaration.heads(), head_dimension: attention.head_dimension,
		query: artifact_parameter(&attention.query, outputs)?, key: artifact_parameter(&attention.key, outputs)?,
		value: artifact_parameter(&attention.value, outputs)?, output: artifact_parameter(&attention.output, outputs)?, }) }

fn artifact_rnn(rnn: &CheckpointRnn, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointRnnImage> {
	Ok(CheckpointRnnImage { sequence_length: rnn.sequence_length, width: rnn.declaration.width(),
		input_weight: artifact_parameter(&rnn.input_weight, outputs)?,
		recurrent_weight: artifact_parameter(&rnn.recurrent_weight, outputs)?, bias: artifact_parameter(&rnn.bias, outputs)?,
	}) }

fn artifact_gru(gru: &CheckpointGru, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointGruImage> {
	Ok(CheckpointGruImage { sequence_length: gru.sequence_length, width: gru.declaration.width(),
		reset_input_weight: artifact_parameter(&gru.reset_input_weight, outputs)?,
		reset_recurrent_weight: artifact_parameter(&gru.reset_recurrent_weight, outputs)?,
		reset_bias: artifact_parameter(&gru.reset_bias, outputs)?,
		update_input_weight: artifact_parameter(&gru.update_input_weight, outputs)?,
		update_recurrent_weight: artifact_parameter(&gru.update_recurrent_weight, outputs)?,
		update_bias: artifact_parameter(&gru.update_bias, outputs)?,
		candidate_input_weight: artifact_parameter(&gru.candidate_input_weight, outputs)?,
		candidate_recurrent_weight: artifact_parameter(&gru.candidate_recurrent_weight, outputs)?,
		candidate_bias: artifact_parameter(&gru.candidate_bias, outputs)?, }) }

fn artifact_lstm(lstm: &CheckpointLstm, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointLstmImage> {
	Ok(CheckpointLstmImage { sequence_length: lstm.sequence_length, width: lstm.declaration.width(),
		input_gate_input_weight: artifact_parameter(&lstm.input_gate_input_weight, outputs)?,
		input_gate_recurrent_weight: artifact_parameter(&lstm.input_gate_recurrent_weight, outputs)?,
		input_gate_bias: artifact_parameter(&lstm.input_gate_bias, outputs)?,
		forget_gate_input_weight: artifact_parameter(&lstm.forget_gate_input_weight, outputs)?,
		forget_gate_recurrent_weight: artifact_parameter(&lstm.forget_gate_recurrent_weight, outputs)?,
		forget_gate_bias: artifact_parameter(&lstm.forget_gate_bias, outputs)?,
		output_gate_input_weight: artifact_parameter(&lstm.output_gate_input_weight, outputs)?,
		output_gate_recurrent_weight: artifact_parameter(&lstm.output_gate_recurrent_weight, outputs)?,
		output_gate_bias: artifact_parameter(&lstm.output_gate_bias, outputs)?,
		candidate_input_weight: artifact_parameter(&lstm.candidate_input_weight, outputs)?,
		candidate_recurrent_weight: artifact_parameter(&lstm.candidate_recurrent_weight, outputs)?,
		candidate_bias: artifact_parameter(&lstm.candidate_bias, outputs)?, }) }

fn artifact_tree(tree: &CheckpointTree, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointTreeImage> {
	Ok(CheckpointTreeImage { declaration: tree.declaration, input_width: tree.input_width, output_width: tree.output_width,
		internal_nodes_per_tree: tree.internal_nodes_per_tree, leaves_per_tree: tree.leaves_per_tree,
		split_features: artifact_tensor(&tree.split_features, outputs)?,
		split_thresholds: artifact_tensor(&tree.split_thresholds, outputs)?,
		leaf_values: artifact_parameter(&tree.leaf_values, outputs)?, }) }

fn artifact_block(block: &CheckpointBlock, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointBlockImage> {
	match block { CheckpointBlock::Embedding(embedding) => {
			artifact_embedding(embedding, outputs).map(CheckpointBlockImage::Embedding) }
		CheckpointBlock::Attention(attention) => { artifact_attention(attention, outputs).map(CheckpointBlockImage::Attention)
		}
		CheckpointBlock::Rnn(rnn) => artifact_rnn(rnn, outputs).map(CheckpointBlockImage::Rnn),
		CheckpointBlock::Gru(gru) => artifact_gru(gru, outputs).map(CheckpointBlockImage::Gru),
		CheckpointBlock::Lstm(lstm) => artifact_lstm(lstm, outputs).map(CheckpointBlockImage::Lstm),
		CheckpointBlock::Layer(layer) => artifact_layer(layer, outputs).map(CheckpointBlockImage::Layer),
		CheckpointBlock::Convolution(convolution) => {
			artifact_convolution(convolution, outputs).map(CheckpointBlockImage::Convolution) }
		CheckpointBlock::Pool(pool) => Ok(CheckpointBlockImage::Pool(*pool)),
		CheckpointBlock::KMeans(kmeans) => artifact_kmeans(kmeans, outputs).map(CheckpointBlockImage::KMeans),
		CheckpointBlock::Tree(tree) => artifact_tree(tree, outputs).map(CheckpointBlockImage::Tree),
		CheckpointBlock::Residual(residual) => { let branch = residual .branch .iter() .map(|step| match step {
					CheckpointResidualBranch::Layer(layer) => {
						artifact_layer(layer, outputs).map(CheckpointResidualBranchImage::Layer) }
					CheckpointResidualBranch::Operation(operation) => { Ok(CheckpointResidualBranchImage::Operation(*operation)) } })
				.collect::<io::Result<Vec<_>>>()?; let skip = match &residual.skip {
				CheckpointResidualSkip::Identity => CheckpointResidualSkipImage::Identity,
				CheckpointResidualSkip::Projection(projection) => {
					CheckpointResidualSkipImage::Projection(artifact_parameter(projection, outputs)?) } };
			Ok(CheckpointBlockImage::Residual(CheckpointResidualImage { branch, branch_prelu: residual .branch_prelu .iter()
					.map(|parameter| artifact_parameter(parameter, outputs)) .collect::<io::Result<Vec<_>>>()?,
				output_width: residual.output_width, skip, operations: residual.operations.clone(), prelu: residual .prelu .iter()
					.map(|parameter| artifact_parameter(parameter, outputs)) .collect::<io::Result<Vec<_>>>()?, })) } } }

fn artifact_metadata(metadata: &VectorMetadata) -> CheckpointArtifactMetadata { match metadata {
		VectorMetadata::None => CheckpointArtifactMetadata::None,
		VectorMetadata::Temporal { origin } => CheckpointArtifactMetadata::Temporal { unix_seconds: origin.unix_seconds,
			nanoseconds: origin.nanoseconds, },
		VectorMetadata::Categorical { dictionary } => CheckpointArtifactMetadata::Categorical {
			dictionary: dictionary.clone(), },
		VectorMetadata::Ordinal { ordered_labels } => CheckpointArtifactMetadata::Ordinal {
			ordered_labels: ordered_labels.clone(), },
		VectorMetadata::Image { encoded_variants } => CheckpointArtifactMetadata::Image { encoded_variants: encoded_variants
				.iter() .map(|variant| CheckpointImageMetadata { format: variant.format(), width: variant.width(),
					height: variant.height(), channels: variant.channels(), color_model: variant.color_model(),
					sample_bits: variant.sample_bits(), value_layout: variant.value_layout(), value_range: variant.value_range(), })
				.collect(), }, } }

fn artifact_parameter( parameter: &CheckpointParameter, outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointParameterImage> { Ok(CheckpointParameterImage {
		parameter: artifact_tensor(&parameter.parameter, outputs)?,
		first_moment: artifact_tensor(&parameter.first_moment, outputs)?,
		second_moment: artifact_tensor(&parameter.second_moment, outputs)?, }) }

fn artifact_tensor(tensor: &CheckpointTensor, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointTensorImage> {
	Ok(CheckpointTensorImage { dtype: tensor.dtype, shape: tensor.shape.clone(),
		bytes: required_output(outputs, tensor.value)?.to_vec(), }) }

fn encode_artifact(artifact: &CheckpointArtifact, writer: &mut impl Write) -> io::Result<()> {
	writeln!(writer, "recipe")?;
	writeln!( writer,
		"\tformat\t{}",
		if is_semantic_model_version(artifact.format_version) { SEMANTIC_MODEL_FORMAT } else { LEGACY_CHECKPOINT_FORMAT } )?;
	writeln!(writer, "\tversion\t{}", artifact.format_version)?;
	writeln!(writer, "\tsemantics")?;
	writeln!( writer,
		"\t\tobjective\t{}",
		dense_loss(artifact.config.loss) )?; writeln!( writer,
		"\t\tnormalization\t{}",
		data_normalization(artifact.config.data_normalization) )?;
	writeln!(writer, "\t\toptimizer\tadamw")?;
	writeln!(writer, "\tdataset")?;
	writeln!(writer, "\t\tfeature-width\t{}", artifact.feature_width)?;
	writeln!(writer, "\t\ttarget")?;
	match artifact.task { DenseTask::BinaryClassification { target_vector, positive_code, } => {
			writeln!(writer, "\t\t\tsource-index\t{target_vector}")?;
			writeln!(writer, "\t\t\ttask\tbinary-classification")?;
			writeln!(writer, "\t\t\tpositive-code\t{positive_code}")?;
		}
		DenseTask::MulticlassClassification { target_vector, class_count, reserved_code, } => {
			writeln!(writer, "\t\t\tsource-index\t{target_vector}")?;
			writeln!(writer, "\t\t\ttask\tmulticlass-classification")?;
			writeln!(writer, "\t\t\tclass-count\t{class_count}")?;
			writeln!(writer, "\t\t\treserved-unseen-code\t{reserved_code}")?;
		}
		DenseTask::ScalarRegression { target_vector } => {
			writeln!(writer, "\t\t\tsource-index\t{target_vector}")?;
			writeln!(writer, "\t\t\ttask\tscalar-regression")?;
		}
		DenseTask::MultiTargetBinaryClassification { .. }
		| DenseTask::JointMulticlassClassification { .. }
		| DenseTask::MultiTargetRegression { .. } => {
			writeln!(writer, "\t\t\tsource-indices")?;
			for source_index in &artifact.target_source_indices {
				writeln!(writer, "\t\t\t\t{source_index}")?;
			}
			let task = match artifact.task {
				DenseTask::MultiTargetBinaryClassification { .. } => "multi-target-binary-classification",
				DenseTask::JointMulticlassClassification { .. } => "joint-multiclass-classification",
				DenseTask::MultiTargetRegression { .. } => "multi-target-regression",
				DenseTask::BinaryClassification { .. }
				| DenseTask::MulticlassClassification { .. }
				| DenseTask::ScalarRegression { .. } => unreachable!(), };
			writeln!(writer, "\t\t\ttask\t{task}")?;
		} }
	writeln!(writer, "\t\tvectors")?;
	for vector in &artifact.vectors {
		writeln!(writer, "\t\t\tvector")?;
		writeln!(writer, "\t\t\t\tsource-index\t{}", vector.source_index)?;
		write!(writer, "\t\t\t\tname-bytes\t")?;
		write_hex(writer, &vector.name)?; writeln!(writer)?;
		writeln!(writer, "\t\t\t\trole\t{}", vector_role(vector.role))?;
		writeln!( writer,
			"\t\t\t\tsemantic-type\t{}",
			semantic_type(vector.semantic_type) )?; writeln!( writer,
			"\t\t\t\tencoding\t{}",
			vector_encoding(vector.encoding) )?; encode_artifact_vector_metadata(writer, &vector.metadata)?; }
	writeln!(writer, "\t\tfeature-spans")?;
	for span in &artifact.feature_spans {
		writeln!(writer, "\t\t\tspan")?;
		writeln!(writer, "\t\t\t\tsource-index\t{}", span.source_vector())?;
		writeln!(writer, "\t\t\t\tstart\t{}", span.start())?;
		writeln!(writer, "\t\t\t\twidth\t{}", span.width())?;
		match span.lowering() { DenseFeatureLowering::NumericScalar => {
				writeln!(writer, "\t\t\t\tlowering\tnumeric-scalar")?;
			}
			DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, } => {
				writeln!(writer, "\t\t\t\tlowering\tcategorical-one-hot")?;
				writeln!(writer, "\t\t\t\t\tdictionary-width\t{dictionary_width}")?;
				writeln!(writer, "\t\t\t\t\treserved-index\t{reserved_index}")?;
			} } }
	writeln!(writer, "\t\tfeature-normalization-mask")?;
	for bits in &artifact.feature_normalization_mask {
		writeln!(writer, "\t\t\tvalue-bits\t0x{bits:08x}")?;
	}
	encode_training_config(writer, &artifact.config, artifact.bounds)?;
	writeln!(writer, "\tmodel")?;
	writeln!(writer, "\t\tinput-width\t{}", artifact.feature_width)?;
	writeln!(writer, "\t\toutput-width\t{}", artifact.task.output_width())?;
	if let Some(adapter) = artifact.output_adapter {
		writeln!(writer, "\t\toutput-adapter\tlinear-projection")?;
		writeln!(writer, "\t\t\tsource-width\t{}", adapter.source_width())?;
		writeln!(writer, "\t\t\ttarget-width\t{}", adapter.target_width())?;
	}
	encode_artifact_data_normalization(writer, artifact)?;
	writeln!(writer, "\t\tblocks")?;
	if artifact.format_version == FLAT_CHECKPOINT_FORMAT_VERSION {
		for (index, layer) in artifact.layers.iter().enumerate() { writeln!( writer,
				"\t\t\t{}\t{}",
				dense_block_kind(layer.declaration.kind()), layer.declaration.width() )?;
			writeln!(writer, "\t\t\t\tindex\t{index}")?;
			writeln!( writer,
				"\t\t\t\toperations\t{}",
				layer.declaration.operations().len() )?; for operation in layer.declaration.operations() { writeln!( writer,
					"\t\t\t\t\toperation\t{}",
					dense_operation(*operation) )?; }
			encode_artifact_parameter_list(writer, 4, "prelu", &layer.prelu)?;
			writeln!(writer, "\t\t\t\tweight")?;
			encode_artifact_parameter(writer, 5, &layer.weight)?;
			writeln!(writer, "\t\t\t\tbias")?;
			encode_artifact_parameter(writer, 5, &layer.bias)?; } } else {
		for (index, block) in artifact.blocks.iter().enumerate() { encode_artifact_block(writer, 3, index, block)?; } }
	if let Some(temperature) = &artifact.temperature {
		writeln!(writer, "\t\tcalibration")?;
		encode_artifact_tensor(writer, 3, "temperature", temperature)?;
	}
	if let Some(native) = &artifact.native { encode_native_metadata(writer, native)?; }
	Ok(()) }

fn encode_native_metadata(writer: &mut impl Write, native: &CheckpointNativeRealization) -> io::Result<()> {
	writeln!(writer, "\tnative")?;
	write!(writer, "\t\tprogram\t")?;
	write_hex(writer, &native.program.bytes())?; writeln!(writer)?;
	write!(writer, "\t\trealization\t")?;
	write_hex(writer, &native.realization.digest().bytes())?; writeln!(writer)?;
	write!(writer, "\t\ttopology\t")?;
	write_hex(writer, &native.topology.digest().bytes())?; writeln!(writer)?;
	write!(writer, "\t\tdiscovery\t")?;
	write_hex(writer, &native.discovery.digest().bytes())?; writeln!(writer)?;
	writeln!(writer, "\t\tkernels")?;
	for kernel in &native.kernels {
		writeln!(writer, "\t\t\tkernel")?;
		writeln!(writer, "\t\t\t\tformat\t{}", kernel.format.extension())?;
		writeln!(writer, "\t\t\t\ttarget")?;
		writeln!(writer, "\t\t\t\t\tbackend\t{}", kernel.target.backend)?;
		writeln!( writer,
			"\t\t\t\t\tarchitecture\t{}",
			kernel.target.architecture )?;
		writeln!(writer, "\t\t\t\t\tabi\t{}", kernel.target.abi)?;
		writeln!(writer, "\t\t\t\ttoolchain")?;
		writeln!(writer, "\t\t\t\t\tname\t{}", kernel.toolchain.name)?;
		writeln!(writer, "\t\t\t\t\tversion\t{}", kernel.toolchain.version)?;
		write!(writer, "\t\t\t\t\tdigest\t")?;
		write_hex(writer, &kernel.toolchain.digest.bytes())?; writeln!(writer)?;
		write!(writer, "\t\t\t\tdigest\t")?;
		write_hex(writer, &kernel.digest.bytes())?; writeln!(writer)?; }
	Ok(()) }

fn encode_artifact_block( writer: &mut impl Write, depth: usize, index: usize, block: &CheckpointBlockImage,
) -> io::Result<()> { match block { CheckpointBlockImage::Embedding(embedding) => {
			write_line(writer, depth, format_args!("embedding"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			write_line(writer, depth + 1, format_args!("dimensions\t{}", embedding.dimensions))?;
			write_line(writer, depth + 1, format_args!("vocabulary\t{}", embedding.vocabulary))?;
			write_line(writer, depth + 1, format_args!("sequence-length\t{}", embedding.sequence_length))?;
			write_line(writer, depth + 1, format_args!("table"))?;
			encode_artifact_parameter(writer, depth + 2, &embedding.table) }
		CheckpointBlockImage::Attention(attention) => {
			write_line(writer, depth, format_args!("attention"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			for (name, value) in [
				("sequence-length", attention.sequence_length.get()),
				("dimensions", attention.dimensions.get()),
				("heads", attention.heads.get()),
				("head-dimension", attention.head_dimension.get()),
			] {
				write_line(writer, depth + 1, format_args!("{name}\t{value}"))?;
			}
			for (name, parameter) in [
				("query", &attention.query),
				("key", &attention.key),
				("value", &attention.value),
				("output", &attention.output),
			] {
				write_line(writer, depth + 1, format_args!("{name}"))?;
				encode_artifact_parameter(writer, depth + 2, parameter)?; }
			Ok(()) }
		CheckpointBlockImage::Rnn(rnn) => {
			write_line(writer, depth, format_args!("rnn"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			for (name, value) in [
				("sequence-length", rnn.sequence_length.get()),
				("width", rnn.width.get()),
			] {
				write_line(writer, depth + 1, format_args!("{name}\t{value}"))?;
			}
			for (name, parameter) in [
				("input-weight", &rnn.input_weight),
				("recurrent-weight", &rnn.recurrent_weight),
				("bias", &rnn.bias),
			] {
				write_line(writer, depth + 1, format_args!("{name}"))?;
				encode_artifact_parameter(writer, depth + 2, parameter)?; }
			Ok(()) }
		CheckpointBlockImage::Gru(gru) => {
			write_line(writer, depth, format_args!("gru"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			for (name, value) in [
				("sequence-length", gru.sequence_length.get()),
				("width", gru.width.get()),
			] {
				write_line(writer, depth + 1, format_args!("{name}\t{value}"))?;
			}
			for (name, parameter) in [
				("reset-input-weight", &gru.reset_input_weight),
				("reset-recurrent-weight", &gru.reset_recurrent_weight),
				("reset-bias", &gru.reset_bias),
				("update-input-weight", &gru.update_input_weight),
				("update-recurrent-weight", &gru.update_recurrent_weight),
				("update-bias", &gru.update_bias),
				("candidate-input-weight", &gru.candidate_input_weight),
				(
					"candidate-recurrent-weight",
					&gru.candidate_recurrent_weight, ),
				("candidate-bias", &gru.candidate_bias),
			] {
				write_line(writer, depth + 1, format_args!("{name}"))?;
				encode_artifact_parameter(writer, depth + 2, parameter)?; }
			Ok(()) }
		CheckpointBlockImage::Lstm(lstm) => {
			write_line(writer, depth, format_args!("lstm"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			for (name, value) in [
				("sequence-length", lstm.sequence_length.get()),
				("width", lstm.width.get()),
			] {
				write_line(writer, depth + 1, format_args!("{name}\t{value}"))?;
			}
			for (name, parameter) in [
				("input-gate-input-weight", &lstm.input_gate_input_weight),
				(
					"input-gate-recurrent-weight",
					&lstm.input_gate_recurrent_weight, ),
				("input-gate-bias", &lstm.input_gate_bias),
				("forget-gate-input-weight", &lstm.forget_gate_input_weight),
				(
					"forget-gate-recurrent-weight",
					&lstm.forget_gate_recurrent_weight, ),
				("forget-gate-bias", &lstm.forget_gate_bias),
				("output-gate-input-weight", &lstm.output_gate_input_weight),
				(
					"output-gate-recurrent-weight",
					&lstm.output_gate_recurrent_weight, ),
				("output-gate-bias", &lstm.output_gate_bias),
				("candidate-input-weight", &lstm.candidate_input_weight),
				(
					"candidate-recurrent-weight",
					&lstm.candidate_recurrent_weight, ),
				("candidate-bias", &lstm.candidate_bias),
			] {
				write_line(writer, depth + 1, format_args!("{name}"))?;
				encode_artifact_parameter(writer, depth + 2, parameter)?; }
			Ok(()) }
		CheckpointBlockImage::Layer(layer) => encode_artifact_layer(writer, depth, index, layer),
		CheckpointBlockImage::Convolution(convolution) => {
			write_line(writer, depth, format_args!("convolution"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			write_line(writer, depth + 1, format_args!("filters\t{}", convolution.declaration.filters()))?;
			write_line(writer, depth + 1, format_args!("kernel\t{}", convolution.declaration.kernel()))?;
			write_tabs(writer, depth + 1)?; writeln!( writer,
				"input-length\t{}",
				convolution.geometry.input_length() )?; write_tabs(writer, depth + 1)?; writeln!( writer,
				"input-channels\t{}",
				convolution.geometry.input_channels() )?; write_tabs(writer, depth + 1)?; writeln!( writer,
				"output-length\t{}",
				convolution.geometry.output_length() )?;
			encode_artifact_operations(writer, depth + 1, convolution.declaration.operations())?;
			encode_artifact_parameter_list(writer, depth + 1, "prelu", &convolution.prelu)?;
			write_line(writer, depth + 1, format_args!("weight"))?;
			encode_artifact_parameter(writer, depth + 2, &convolution.weight)?;
			write_line(writer, depth + 1, format_args!("bias"))?;
			encode_artifact_parameter(writer, depth + 2, &convolution.bias) }
		CheckpointBlockImage::Pool(pool) => {
			write_line(writer, depth, format_args!("pool"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			write_line(writer, depth + 1, format_args!("size\t{}", pool.size))?;
			write_tabs(writer, depth + 1)?; match pool.group_to_neuron {
				Some(neurons) => writeln!(writer, "group-to-neuron\t{neurons}")?,
				None => writeln!(writer, "group-to-neuron\tnone")?,
			}
			write_line(writer, depth + 1, format_args!("input-length\t{}", pool.input_length))?;
			write_line(writer, depth + 1, format_args!("channels\t{}", pool.channels))?;
			write_line(writer, depth + 1, format_args!("output-length\t{}", pool.output_length))?;
			write_line(writer, depth + 1, format_args!("group-order\tgroup-major-channel-minor"))?;
			write_tabs(writer, depth + 1)?;
			writeln!(writer, "winner-contract\tlowest-logical-index")
		}
		CheckpointBlockImage::KMeans(kmeans) => {
			write_line(writer, depth, format_args!("kmeans"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			write_line(writer, depth + 1, format_args!("clusters\t{}", kmeans.clusters))?;
			write_tabs(writer, depth + 1)?; match kmeans.group_to_neuron {
				Some(neurons) => writeln!(writer, "group-to-neuron\t{neurons}")?,
				None => writeln!(writer, "group-to-neuron\tnone")?,
			}
			write_line(writer, depth + 1, format_args!("input-width\t{}", kmeans.input_width))?;
			encode_artifact_tensor(writer, depth + 1, "centroids", &kmeans.centroids)
		}
		CheckpointBlockImage::Tree(tree) => {
			write_line(writer, depth, format_args!("tree"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			write_tabs(writer, depth + 1)?; writeln!( writer,
				"family\t{}",
				dense_tree_family(tree.declaration.family()) )?;
			write_line(writer, depth + 1, format_args!("trees\t{}", tree.declaration.trees()))?;
			write_line(writer, depth + 1, format_args!("depth\t{}", tree.declaration.depth()))?;
			write_line(writer, depth + 1, format_args!("input-width\t{}", tree.input_width))?;
			write_line(writer, depth + 1, format_args!("output-width\t{}", tree.output_width))?;
			write_tabs(writer, depth + 1)?; writeln!( writer,
				"internal-nodes-per-tree\t{}",
				tree.internal_nodes_per_tree )?;
			write_line(writer, depth + 1, format_args!("leaves-per-tree\t{}", tree.leaves_per_tree))?;
			encode_artifact_tensor(writer, depth + 1, "split-features", &tree.split_features)?;
			encode_artifact_tensor( writer, depth + 1,
				"split-thresholds",
				&tree.split_thresholds, )?;
			write_line(writer, depth + 1, format_args!("leaf-values"))?;
			encode_artifact_parameter(writer, depth + 2, &tree.leaf_values) }
		CheckpointBlockImage::Residual(residual) => {
			write_line(writer, depth, format_args!("residual"))?;
			write_line(writer, depth + 1, format_args!("index\t{index}"))?;
			write_line(writer, depth + 1, format_args!("output-width\t{}", residual.output_width))?;
			write_line(writer, depth + 1, format_args!("branch\t{}", residual.branch.len()))?;
			for (step_index, step) in residual.branch.iter().enumerate() { match step {
					CheckpointResidualBranchImage::Layer(layer) => { encode_artifact_layer(writer, depth + 2, step_index, layer)?; }
					CheckpointResidualBranchImage::Operation(operation) => {
						write_line(writer, depth + 2, format_args!("operation\t{}", dense_operation(*operation)))?;
					} } }
			encode_artifact_parameter_list(writer, depth + 1, "branch-prelu", &residual.branch_prelu)?;
			write_tabs(writer, depth + 1)?; match &residual.skip {
				CheckpointResidualSkipImage::Identity => writeln!(writer, "skip\tidentity")?,
				CheckpointResidualSkipImage::Projection(projection) => {
					writeln!(writer, "skip\tlinear-projection")?;
					write_line(writer, depth + 2, format_args!("weight"))?;
					encode_artifact_parameter(writer, depth + 3, projection)?; } }
			encode_artifact_operations(writer, depth + 1, &residual.operations)?;
			encode_artifact_parameter_list(writer, depth + 1, "prelu", &residual.prelu)
		} } }

fn encode_artifact_layer( writer: &mut impl Write, depth: usize, index: usize, layer: &CheckpointLayerImage,
) -> io::Result<()> { write_tabs(writer, depth)?; writeln!( writer,
		"{}\t{}",
		dense_block_kind(layer.declaration.kind()), layer.declaration.width() )?;
	write_line(writer, depth + 1, format_args!("index\t{index}"))?;
	encode_artifact_operations(writer, depth + 1, layer.declaration.operations())?;
	encode_artifact_parameter_list(writer, depth + 1, "prelu", &layer.prelu)?;
	write_line(writer, depth + 1, format_args!("weight"))?;
	encode_artifact_parameter(writer, depth + 2, &layer.weight)?;
	write_line(writer, depth + 1, format_args!("bias"))?;
	encode_artifact_parameter(writer, depth + 2, &layer.bias) }

fn encode_artifact_operations(writer: &mut impl Write, depth: usize, operations: &[DenseOperation]) -> io::Result<()> {
	write_line(writer, depth, format_args!("operations\t{}", operations.len()))?;
	for operation in operations {
		write_line(writer, depth + 1, format_args!("operation\t{}", dense_operation(*operation)))?;
	}
	Ok(()) }

fn encode_artifact_parameter_list( writer: &mut impl Write, depth: usize, name: &str,
	parameters: &[CheckpointParameterImage], ) -> io::Result<()> { if parameters.is_empty() { return Ok(()); }
	write_line(writer, depth, format_args!("{name}\t{}", parameters.len()))?;
	for parameter in parameters {
		write_line(writer, depth + 1, format_args!("parameter"))?;
		encode_artifact_parameter(writer, depth + 2, parameter)?; }
	Ok(()) }

fn encode_artifact_vector_metadata(writer: &mut impl Write, metadata: &CheckpointArtifactMetadata) -> io::Result<()> {
	match metadata {
		CheckpointArtifactMetadata::None => writeln!(writer, "\t\t\t\tmetadata\tnone"),
		CheckpointArtifactMetadata::Temporal { unix_seconds, nanoseconds, } => {
			writeln!(writer, "\t\t\t\tmetadata\ttemporal")?;
			writeln!(writer, "\t\t\t\t\tunix-seconds\t{unix_seconds}")?;
			writeln!(writer, "\t\t\t\t\tnanoseconds\t{nanoseconds}")
		}
		CheckpointArtifactMetadata::Categorical { dictionary } => {
			writeln!(writer, "\t\t\t\tmetadata\tcategorical")?;
			for value in dictionary {
				write!(writer, "\t\t\t\t\tvalue-bytes\t")?;
				write_hex(writer, value)?; writeln!(writer)?; }
			Ok(()) }
		CheckpointArtifactMetadata::Ordinal { ordered_labels } => {
			writeln!(writer, "\t\t\t\tmetadata\tordinal")?;
			for value in ordered_labels {
				write!(writer, "\t\t\t\t\tvalue-bytes\t")?;
				write_hex(writer, value)?; writeln!(writer)?; }
			Ok(()) }
		CheckpointArtifactMetadata::Image { encoded_variants } => {
			writeln!(writer, "\t\t\t\tmetadata\timage")?;
			for variant in encoded_variants {
				writeln!(writer, "\t\t\t\t\tvariant")?;
				writeln!( writer,
					"\t\t\t\t\t\tformat\t{}",
					image_format(variant.format) )?;
				writeln!(writer, "\t\t\t\t\t\twidth\t{}", variant.width)?;
				writeln!(writer, "\t\t\t\t\t\theight\t{}", variant.height)?;
				writeln!( writer,
					"\t\t\t\t\t\tchannels\t{}",
					variant .channels
						.map_or_else(|| "none".to_owned(), |value| value.to_string())
				)?; writeln!( writer,
					"\t\t\t\t\t\tcolor-model\t{}",
					variant.color_model.map_or("none", image_color_model)
				)?; writeln!( writer,
					"\t\t\t\t\t\tsample-bits\t{}",
					variant .sample_bits
						.map_or_else(|| "none".to_owned(), |value| value.to_string())
				)?; writeln!( writer,
					"\t\t\t\t\t\tvalue-layout\t{}",
					image_value_layout(variant.value_layout) )?; writeln!( writer,
					"\t\t\t\t\t\tvalue-range\t{}",
					image_value_range(variant.value_range) )?; }
			Ok(()) } } }

fn encode_training_config( writer: &mut impl Write, config: &DenseTrainingConfig, bounds: TrainingBounds,
) -> io::Result<()> {
	writeln!(writer, "\ttraining")?;
	writeln!(writer, "\t\tepochs\t{}", config.epochs)?;
	writeln!(writer, "\t\twarmup-epochs\t{}", config.warmup_epochs)?;
	writeln!( writer,
		"\t\tlearning-rate-decay\t{}",
		learning_rate_decay(config.learning_rate_decay) )?; match config.gradient_clip_norm {
		Some(value) => write_f32_bits(writer, 2, "gradient-clip-norm", value)?,
		None => writeln!(writer, "\t\tgradient-clip-norm\tnone")?,
	}
	write_f32_bits( writer, 2,
		"normalization-epsilon",
		config.normalization_epsilon, )?; writeln!( writer,
		"\t\treduction-tree-lanes\t{}",
		config.reduction_tree_lanes )?;
	writeln!(writer, "\t\trandom-seed\t{}", config.random_seed)?;
	encode_adamw(writer, config.adamw)?;
	writeln!(writer, "\t\tbounds")?;
	writeln!(writer, "\t\t\ttrain-rows\t{}", bounds.train_rows)?;
	writeln!(writer, "\t\t\tepochs\t{}", bounds.epochs)?;
	writeln!( writer,
		"\t\t\ttraining-iterations\t{}",
		loop_iterations_text(bounds.training_iterations) )?; writeln!( writer,
		"\t\t\tcalibration-iterations\t{}",
		bounds.calibration_iterations )?; writeln!( writer,
		"\t\t\titerations\t{}",
		loop_iterations_text(bounds.iterations) )?; writeln!( writer,
		"\t\t\twarmup-iterations\t{}",
		bounds.warmup_iterations ) }

fn encode_artifact_data_normalization(writer: &mut impl Write, artifact: &CheckpointArtifact) -> io::Result<()> {
	writeln!( writer,
		"\t\tnormalization\t{}",
		data_normalization(artifact.config.data_normalization) )?;
	let names: &[&str] = match artifact.config.data_normalization { DenseDataNormalization::Identity => &[],
		DenseDataNormalization::ZScore => &["mean", "variance"],
		DenseDataNormalization::MinMax => &["minimum", "maximum"],
		DenseDataNormalization::L2Norm => &[], }; if names.len() != artifact.normalization.len() { return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"checkpoint normalization tensor count differs from its declaration",
		)); }
	for (name, tensor) in names.iter().zip(&artifact.normalization) { encode_artifact_tensor(writer, 3, name, tensor)?; }
	Ok(()) }

fn encode_adamw(writer: &mut impl Write, adamw: AdamWConfig) -> io::Result<()> {
	writeln!(writer, "\t\tadamw")?;
	write_f32_bits(writer, 3, "learning-rate", adamw.learning_rate)?;
	write_f32_bits(writer, 3, "beta-one", adamw.beta_one)?;
	write_f32_bits(writer, 3, "beta-two", adamw.beta_two)?;
	write_f32_bits(writer, 3, "epsilon", adamw.epsilon)?;
	write_f32_bits(writer, 3, "weight-decay", adamw.weight_decay)
}

fn write_f32_bits(writer: &mut impl Write, depth: usize, name: &str, value: f32) -> io::Result<()> {
	write_tabs(writer, depth)?;
	writeln!(writer, "{name}\t0x{:08x}", value.to_bits())
}

fn encode_artifact_parameter( writer: &mut impl Write, depth: usize, parameter: &CheckpointParameterImage,
) -> io::Result<()> {
	encode_artifact_tensor(writer, depth, "parameter", &parameter.parameter)?;
	encode_artifact_tensor(writer, depth, "first-moment", &parameter.first_moment)?;
	encode_artifact_tensor(writer, depth, "second-moment", &parameter.second_moment)
}

fn encode_artifact_tensor( writer: &mut impl Write, depth: usize, name: &str, tensor: &CheckpointTensorImage,
) -> io::Result<()> {
	write_line(writer, depth, format_args!("{name}"))?;
	write_line(writer, depth + 1, format_args!("dtype\t{}", dtype(tensor.dtype)))?;
	write_tabs(writer, depth + 1)?;
	write!(writer, "shape")?;
	for extent in &tensor.shape {
		write!(writer, "\t{extent}")?;
	}
	writeln!(writer)?;
	write_line(writer, depth + 1, format_args!("payload\traw-bytes-hex"))?;
	if tensor.bytes.is_empty() {
		write_line(writer, depth + 2, format_args!("0x"))?;
	} else { for chunk in tensor.bytes.chunks(HEX_CHUNK_BYTES) { write_tabs(writer, depth + 2)?; write_hex(writer, chunk)?;
			writeln!(writer)?; } }
	Ok(()) }

fn required_output<'a>(outputs: &'a BTreeMap<ValueId, &[u8]>, value: ValueId) -> io::Result<&'a [u8]> {
	outputs.get(&value).copied().ok_or_else(|| { io::Error::new( io::ErrorKind::InvalidData,
			format!("validated checkpoint output {value} disappeared"),
		) }) }

fn feature_normalization_mask(spans: &[CompiledFeatureSpan], width: usize) -> Vec<u32> {
	let mut mask = vec![0.0f32.to_bits(); width]; for span in spans {
		if span.lowering() == DenseFeatureLowering::NumericScalar {
			let end = span.start().saturating_add(span.width()).min(width);
			mask[span.start().min(width)..end].fill(1.0f32.to_bits()); } }
	mask }

fn write_tabs(writer: &mut impl Write, depth: usize) -> io::Result<()> { for _ in 0..depth {
		writer.write_all(b"\t")?;
	}
	Ok(()) }

fn write_line(writer: &mut impl Write, depth: usize, line: fmt::Arguments<'_>) -> io::Result<()> {
	write_tabs(writer, depth)?; writer.write_fmt(line)?; writeln!(writer) }

fn write_hex(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
	const HEX: &[u8; 16] = b"0123456789abcdef";
	writer.write_all(b"0x")?;
	for byte in bytes { writer.write_all(&[HEX[usize::from(*byte >> 4)], HEX[usize::from(*byte & 0x0f)]])?; }
	Ok(()) }

const fn dtype(dtype: DType) -> &'static str {
	match dtype {
		DType::F32 => "f32",
		DType::I32 => "int32",
	} }

const fn vector_role(role: VectorRole) -> &'static str {
	match role {
		VectorRole::Feature => "feature",
		VectorRole::Target => "target",
	} }

const fn semantic_type(semantic_type: SemanticType) -> &'static str {
	match semantic_type {
		SemanticType::Numeric => "numeric",
		SemanticType::Temporal => "temporal",
		SemanticType::Categorical => "categorical",
		SemanticType::Ordinal => "ordinal",
		SemanticType::Text => "text",
		SemanticType::Image => "image",
		SemanticType::Binary => "binary",
	} }

const fn image_format(format: EncodedImageFormat) -> &'static str {
	match format {
		EncodedImageFormat::Png => "png",
		EncodedImageFormat::Jpeg => "jpeg",
		EncodedImageFormat::Gif87a => "gif87a",
		EncodedImageFormat::Gif89a => "gif89a",
		EncodedImageFormat::Bmp => "bmp",
		EncodedImageFormat::WebP => "webp",
	} }

const fn image_color_model(model: ImageColorModel) -> &'static str {
	match model {
		ImageColorModel::Grayscale => "grayscale",
		ImageColorModel::GrayscaleAlpha => "grayscale-alpha",
		ImageColorModel::Rgb => "rgb",
		ImageColorModel::Rgba => "rgba",
		ImageColorModel::Bgr => "bgr",
		ImageColorModel::IndexedRgb => "indexed-rgb",
		ImageColorModel::YCbCr => "y-cb-cr",
		ImageColorModel::Cmyk => "cmyk",
		ImageColorModel::Ycck => "ycck",
	} }

const fn image_value_layout(layout: ImageValueLayout) -> &'static str {
	match layout {
		ImageValueLayout::EncodedFile => "encoded-file",
	} }

const fn image_value_range(range: ImageValueRange) -> &'static str {
	match range {
		ImageValueRange::EncodedBytes => "encoded-bytes",
	} }

const fn vector_encoding(encoding: VectorEncoding) -> &'static str {
	match encoding {
		VectorEncoding::F32 => "f32",
		VectorEncoding::I32 => "int32",
		VectorEncoding::RelativeSecondsI32 => "relative-seconds-int32",
		VectorEncoding::DictionaryI32 => "dictionary-int32",
		VectorEncoding::OrdinalI32 => "ordinal-int32",
		VectorEncoding::Utf8 => "utf8",
		VectorEncoding::Bytes => "bytes",
	} }

const fn dense_block_kind(kind: DenseBlockKind) -> &'static str {
	match kind {
		DenseBlockKind::Layer => "layer",
		DenseBlockKind::Perc => "perc",
	} }

const fn dense_tree_family(family: DenseTreeFamily) -> &'static str {
	match family {
		DenseTreeFamily::LightGbm => "lightgbm",
		DenseTreeFamily::CatBoost => "catboost",
		DenseTreeFamily::XGBoost => "xgboost",
	} }

const fn dense_operation(operation: DenseOperation) -> &'static str {
	match operation { DenseOperation::Activation(activation) => activation.checkpoint_tag(),
		DenseOperation::Normalization(crate::DenseNormalization::Layer) => "layer-normalization",
		DenseOperation::Normalization(crate::DenseNormalization::Batch) => "batch-normalization",
	} }

const fn data_normalization(normalization: DenseDataNormalization) -> &'static str {
	match normalization {
		DenseDataNormalization::Identity => "identity",
		DenseDataNormalization::ZScore => "z-score",
		DenseDataNormalization::MinMax => "min-max",
		DenseDataNormalization::L2Norm => "l2-norm",
	} }

const fn dense_loss(loss: DenseLoss) -> &'static str {
	match loss {
		DenseLoss::BinaryCrossEntropy => "binary-cross-entropy-with-logits",
		DenseLoss::Focal => "binary-focal-with-logits-alpha-0.25-gamma-2",
		DenseLoss::MeanSquaredError => "mean-squared-error",
		DenseLoss::MeanAbsoluteError => "mean-absolute-error",
		DenseLoss::CrossEntropy => "cross-entropy",
		DenseLoss::Huber => "huber-unit-delta",
	} }

const fn learning_rate_decay(decay: LearningRateDecay) -> &'static str {
	match decay {
		LearningRateDecay::Constant => "constant",
		LearningRateDecay::Linear => "linear",
		LearningRateDecay::Cosine => "cosine",
		LearningRateDecay::Exponential => "exponential",
	} }

fn loop_iterations_text(iterations: LoopIterations) -> String { iterations .finite()
		.map_or_else(|| "unbounded".to_owned(), |finite| finite.get().to_string())
}

#[derive(Debug, Default)]
struct CountingWriter { bytes: u64, }

impl Write for CountingWriter { fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
		let length = u64::try_from(bytes.len()).map_err(|_| io::Error::other("checkpoint size exceeds u64"))?;
		self.bytes = self .bytes .checked_add(length)
			.ok_or_else(|| io::Error::other("checkpoint size exceeds u64"))?;
		Ok(bytes.len()) }

	fn flush(&mut self) -> io::Result<()> { Ok(()) } }

pub(crate) fn atomic_save( target: &Path, encoded_bytes: u64,
	write_checkpoint: impl FnOnce(&mut File) -> io::Result<()>, ) -> CheckpointResult<()> {
	let parent = normalized_parent(target)?; validate_target(target, parent)?; let statistics = rustix::fs::statvfs(parent)
		.map_err(|error| CheckpointError::io("query filesystem capacity", parent, error.into()))?;
	let allocation = allocation_bytes(encoded_bytes, statistics.f_frsize)?; require_capacity( target, statistics.f_bavail,
		statistics.f_frsize, allocation, EXACT_USER_RESERVATION.get(), )?;
	let (mut temporary, mut guard) = create_private_temporary(target, parent)?; write_checkpoint(&mut temporary)
		.map_err(|error| CheckpointError::io("write checkpoint temporary", guard.path.clone(), error))?;
	temporary .flush()
		.map_err(|error| CheckpointError::io("flush checkpoint temporary", guard.path.clone(), error))?;
	temporary .sync_all()
		.map_err(|error| CheckpointError::io("sync checkpoint temporary", guard.path.clone(), error))?;
	let actual = temporary .metadata()
		.map_err(|error| CheckpointError::io("inspect checkpoint temporary", guard.path.clone(), error))?
		.len(); if actual != encoded_bytes { return Err(CheckpointError::invalid_target( target,
			format!("encoder wrote {actual} bytes after measuring {encoded_bytes}"),
		)); }
	drop(temporary); fs::rename(&guard.path, target)
		.map_err(|error| CheckpointError::io("atomically install checkpoint", target, error))?;
	guard.armed = false; let directory =
		File::open(parent).map_err(|error| CheckpointError::io("open checkpoint parent", parent, error))?;
	directory .sync_all()
		.map_err(|error| CheckpointError::io("sync checkpoint parent", parent, error))
}

fn normalized_parent(target: &Path) -> CheckpointResult<&Path> {
	if target.as_os_str().is_empty() || target.file_name().is_none() { return Err(CheckpointError::invalid_target( target,
			"target must name a file",
		)); }
	let parent = target.parent().unwrap_or_else(|| Path::new("."));
	Ok(if parent.as_os_str().is_empty() {
		Path::new(".")
	} else { parent }) }

fn validate_target(target: &Path, parent: &Path) -> CheckpointResult<()> { let parent_metadata =
		fs::metadata(parent).map_err(|error| CheckpointError::io("inspect checkpoint parent", parent, error))?;
	if !parent_metadata.is_dir() { return Err(CheckpointError::invalid_target( target,
			"parent is not a directory",
		)); }
	match fs::symlink_metadata(target) {
		Ok(metadata) if metadata.file_type().is_symlink() => Err(CheckpointError::invalid_target( target,
			"existing target is a symbolic link",
		)), Ok(metadata) if !metadata.is_file() => Err(CheckpointError::invalid_target( target,
			"existing target is not a regular file",
		)), Ok(_) => Ok(()), Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
		Err(error) => Err(CheckpointError::io(
			"inspect checkpoint target",
			target, error, )), } }

fn allocation_bytes(encoded_bytes: u64, fragment_bytes: u64) -> CheckpointResult<u64> { if fragment_bytes == 0 {
		return Err(CheckpointError::manifest(
			"filesystem reports a zero fragment size",
		)); }
	if encoded_bytes == 0 { return Ok(0); }
	let fragments = encoded_bytes .checked_add(fragment_bytes - 1) .and_then(|rounded| rounded.checked_div(fragment_bytes))
		.ok_or_else(|| CheckpointError::manifest("checkpoint allocation size overflowed"))?;
	fragments .checked_mul(fragment_bytes)
		.ok_or_else(|| CheckpointError::manifest("checkpoint allocation size overflowed"))
}

fn require_capacity( target: &Path, available_fragments: u64, fragment_bytes: u64, checkpoint_allocation: u64,
	reservation: u64, ) -> CheckpointResult<()> { let available = available_fragments .checked_mul(fragment_bytes)
		.ok_or_else(|| CheckpointError::manifest("available filesystem capacity overflowed"))?;
	let required = checkpoint_allocation .checked_add(reservation)
		.ok_or_else(|| CheckpointError::manifest("checkpoint capacity requirement overflowed"))?;
	if available < required { return Err(CheckpointError::InsufficientCapacity { path: target.to_path_buf(), available,
			checkpoint_allocation, reservation, }); }
	Ok(()) }

fn create_private_temporary(target: &Path, parent: &Path) -> CheckpointResult<(File, TemporaryGuard)> {
	let filename = target .file_name()
		.ok_or_else(|| CheckpointError::invalid_target(target, "target must name a file"))?;
	for _ in 0..TEMP_CREATE_ATTEMPTS { let sequence = NEXT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
		let mut temporary_name = OsString::from(".");
		temporary_name.push(filename);
		temporary_name.push(format!(".recipe-tmp-{}-{sequence}", std::process::id()));
		let temporary_path = parent.join(temporary_name); match OpenOptions::new() .write(true) .create_new(true) .mode(0o600)
			.open(&temporary_path)
		{ Ok(file) => { return Ok(( file, TemporaryGuard { path: temporary_path, armed: true, }, )); }
			Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
			Err(error) => { return Err(CheckpointError::io(
					"create checkpoint temporary",
					temporary_path, error, )); } } }
	Err(CheckpointError::invalid_target( target,
		"could not create a unique private temporary file",
	)) }

#[derive(Debug)]
struct TemporaryGuard { path: PathBuf, armed: bool, }

impl Drop for TemporaryGuard { fn drop(&mut self) { if self.armed { let _ = fs::remove_file(&self.path); } } }
