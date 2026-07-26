use core::fmt;
use core::num::{NonZeroU64, NonZeroUsize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use recipe_core::{BundleIdentity, DType, EXACT_USER_RESERVATION, RunId, ValueId};
use recipe_executor::{ExitImage, RunJournal};
use recipe_ingest::{
	EncodedImageFormat, ImageColorModel, ImageValueLayout, ImageValueRange, SemanticType, VectorEncoding,
	VectorMetadata, VectorRole,
};
use recipe_ogdl::{Graph as OgdlGraph, NodeId as OgdlNodeId};

use crate::{
	AdamWConfig, CompiledFeatureSpan, CompiledTraining, CompletedTrainingExecution, DataNormalizationState,
	DecodedMulticlassClass, DenseActivation, DenseBlock, DenseBlockKind, DenseBlockState, DenseDataNormalization,
	DenseFeatureLowering, DenseLayer, DenseLayerState, DenseLoss, DenseOperation, DenseOutputAdapter, DenseResidual,
	DenseResidualOperation, DenseResidualState, DenseTask, DenseTrainingConfig, FinalTrainingMetric, LearningRateDecay,
	ParameterState, TrainingBounds,
};

const FLAT_CHECKPOINT_FORMAT_VERSION: u32 = 5;
const STRUCTURED_CHECKPOINT_FORMAT_VERSION: u32 = 6;
const HEX_CHUNK_BYTES: usize = 64;
const TEMP_CREATE_ATTEMPTS: u64 = 64;

static NEXT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// A stable location inside a Recipe checkpoint.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckpointPath {
	segments: Vec<CheckpointPathSegment>,
}

impl CheckpointPath {
	#[must_use]
	pub fn segments(&self) -> &[CheckpointPathSegment] {
		&self.segments
	}

	fn root() -> Self {
		Self::default()
	}

	fn field(&self, name: impl Into<String>) -> Self {
		let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Field(name.into()));
		Self { segments }
	}

	fn index(&self, index: usize) -> Self {
		let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Index(index));
		Self { segments }
	}
}

impl fmt::Display for CheckpointPath {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.segments.is_empty() {
			return formatter.write_str("<checkpoint>");
		}
		let mut needs_separator = false;
		for segment in &self.segments {
			match segment {
				CheckpointPathSegment::Field(field) => {
					if needs_separator {
						formatter.write_str(".")?;
					}
					formatter.write_str(field)?;
					needs_separator = true;
				}
				CheckpointPathSegment::Index(index) => write!(formatter, "[{index}]")?,
			}
		}
		Ok(())
	}
}

/// One typed component of a checkpoint path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointPathSegment {
	Field(String),
	Index(usize),
}

/// Stable class of a checkpoint decoding failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckpointDecodeErrorKind {
	LimitExceeded,
	InvalidUtf8,
	InvalidSyntax,
	MissingField,
	DuplicateField,
	UnknownField,
	InvalidValue,
	InconsistentValue,
}

/// A typed, path-addressed checkpoint decoding failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointDecodeError {
	kind: CheckpointDecodeErrorKind,
	path: CheckpointPath,
	detail: String,
}

impl CheckpointDecodeError {
	fn new(kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: impl Into<String>) -> Self {
		Self {
			kind,
			path,
			detail: detail.into(),
		}
	}

	#[must_use]
	pub const fn kind(&self) -> CheckpointDecodeErrorKind {
		self.kind
	}

	#[must_use]
	pub const fn path(&self) -> &CheckpointPath {
		&self.path
	}

	#[must_use]
	pub fn detail(&self) -> &str {
		&self.detail
	}
}

impl fmt::Display for CheckpointDecodeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{}: {}", self.path, self.detail)
	}
}

impl std::error::Error for CheckpointDecodeError {}

/// Finite allocation and structural bounds for decoding one checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointDecodeLimits {
	pub source_bytes: usize,
	pub nodes: usize,
	pub vectors: usize,
	pub feature_spans: usize,
	pub layers: usize,
	pub metadata_entries: usize,
	pub tensors: usize,
	pub tensor_rank: usize,
	pub tensor_bytes: usize,
	pub total_payload_bytes: usize,
}

impl Default for CheckpointDecodeLimits {
	fn default() -> Self {
		Self {
			source_bytes: 1 << 30,
			nodes: 4_000_000,
			vectors: 65_536,
			feature_spans: 65_536,
			layers: 16_384,
			metadata_entries: 1_000_000,
			tensors: 100_000,
			tensor_rank: 16,
			tensor_bytes: 1 << 30,
			total_payload_bytes: 1 << 30,
		}
	}
}

/// Saving or validating a completed Recipe checkpoint failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum CheckpointError {
	Decode(CheckpointDecodeError),
	InvalidManifest {
		detail: String,
	},
	DuplicateOutput {
		value: ValueId,
	},
	MissingOutput {
		value: ValueId,
	},
	UnexpectedOutput {
		value: ValueId,
	},
	OutputDTypeMismatch {
		value: ValueId,
		expected: DType,
		actual: DType,
	},
	OutputSizeMismatch {
		value: ValueId,
		expected: u64,
		actual: u64,
	},
	InvalidTarget {
		path: PathBuf,
		detail: String,
	},
	InsufficientCapacity {
		path: PathBuf,
		available: u64,
		checkpoint_allocation: u64,
		reservation: u64,
	},
	Io {
		operation: &'static str,
		path: PathBuf,
		source: io::Error,
	},
}

impl CheckpointError {
	fn manifest(detail: impl Into<String>) -> Self {
		Self::InvalidManifest {
			detail: detail.into(),
		}
	}

	fn invalid_target(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self {
		Self::InvalidTarget {
			path: path.into(),
			detail: detail.into(),
		}
	}

	fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
		Self::Io {
			operation,
			path: path.into(),
			source,
		}
	}
}

impl fmt::Display for CheckpointError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Decode(error) => write!(formatter, "decode checkpoint: {error}"),
			Self::InvalidManifest { detail } => write!(formatter, "invalid checkpoint manifest: {detail}"),
			Self::DuplicateOutput { value } => write!(
				formatter,
				"checkpoint output {value} appears more than once"
			),
			Self::MissingOutput { value } => write!(formatter, "checkpoint output {value} is absent"),
			Self::UnexpectedOutput { value } => {
				write!(
					formatter,
					"execution returned unexpected checkpoint output {value}"
				)
			}
			Self::OutputDTypeMismatch {
				value,
				expected,
				actual,
			} => write!(
				formatter,
				"checkpoint output {value} has dtype {actual:?}, expected {expected:?}"
			),
			Self::OutputSizeMismatch {
				value,
				expected,
				actual,
			} => write!(
				formatter,
				"checkpoint output {value} has {actual} bytes, expected {expected}"
			),
			Self::InvalidTarget { path, detail } => {
				write!(
					formatter,
					"invalid checkpoint target {}: {detail}",
					path.display()
				)
			}
			Self::InsufficientCapacity {
				path,
				available,
				checkpoint_allocation,
				reservation,
			} => write!(
				formatter,
				"checkpoint target {} has {available} available bytes; writing needs \
				 {checkpoint_allocation} bytes while preserving {reservation} bytes",
				path.display()
			),
			Self::Io {
				operation,
				path,
				source,
			} => write!(formatter, "{operation} {}: {source}", path.display()),
		}
	}
}

impl std::error::Error for CheckpointError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Decode(error) => Some(error),
			Self::Io { source, .. } => Some(source),
			_ => None,
		}
	}
}

/// Dataset semantics retained by a model without retaining source rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointVectorSchema {
	source_index: usize,
	name: Vec<u8>,
	role: VectorRole,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
	metadata: VectorMetadata,
}

impl CheckpointVectorSchema {
	#[must_use]
	pub const fn source_index(&self) -> usize {
		self.source_index
	}

	#[must_use]
	pub fn name(&self) -> &[u8] {
		&self.name
	}

	#[must_use]
	pub const fn role(&self) -> VectorRole {
		self.role
	}

	#[must_use]
	pub const fn semantic_type(&self) -> SemanticType {
		self.semantic_type
	}

	#[must_use]
	pub const fn encoding(&self) -> VectorEncoding {
		self.encoding
	}

	#[must_use]
	pub const fn metadata(&self) -> &VectorMetadata {
		&self.metadata
	}
}

/// Image header facts decoded from a checkpoint without widening ingestion's
/// construction API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckpointImageMetadata {
	format: EncodedImageFormat,
	width: u32,
	height: u32,
	channels: Option<u8>,
	color_model: Option<ImageColorModel>,
	sample_bits: Option<u8>,
	value_layout: ImageValueLayout,
	value_range: ImageValueRange,
}

impl CheckpointImageMetadata {
	#[must_use]
	pub const fn format(self) -> EncodedImageFormat {
		self.format
	}

	#[must_use]
	pub const fn width(self) -> u32 {
		self.width
	}

	#[must_use]
	pub const fn height(self) -> u32 {
		self.height
	}

	#[must_use]
	pub const fn channels(self) -> Option<u8> {
		self.channels
	}

	#[must_use]
	pub const fn color_model(self) -> Option<ImageColorModel> {
		self.color_model
	}

	#[must_use]
	pub const fn sample_bits(self) -> Option<u8> {
		self.sample_bits
	}

	#[must_use]
	pub const fn value_layout(self) -> ImageValueLayout {
		self.value_layout
	}

	#[must_use]
	pub const fn value_range(self) -> ImageValueRange {
		self.value_range
	}
}

/// Encoding metadata retained by a decoded checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointArtifactMetadata {
	None,
	Temporal {
		unix_seconds: i64,
		nanoseconds: u32,
	},
	Categorical {
		dictionary: Vec<Vec<u8>>,
	},
	Ordinal {
		ordered_labels: Vec<Vec<u8>>,
	},
	Image {
		encoded_variants: Vec<CheckpointImageMetadata>,
	},
}

/// Full row-free vector schema decoded from a checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointArtifactVector {
	source_index: usize,
	name: Vec<u8>,
	role: VectorRole,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
	metadata: CheckpointArtifactMetadata,
}

impl CheckpointArtifactVector {
	#[must_use]
	pub const fn source_index(&self) -> usize {
		self.source_index
	}

	#[must_use]
	pub fn name(&self) -> &[u8] {
		&self.name
	}

	#[must_use]
	pub const fn role(&self) -> VectorRole {
		self.role
	}

	#[must_use]
	pub const fn semantic_type(&self) -> SemanticType {
		self.semantic_type
	}

	#[must_use]
	pub const fn encoding(&self) -> VectorEncoding {
		self.encoding
	}

	#[must_use]
	pub const fn metadata(&self) -> &CheckpointArtifactMetadata {
		&self.metadata
	}
}

/// One exact tensor image decoded from a checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointTensorImage {
	dtype: DType,
	shape: Vec<u64>,
	bytes: Vec<u8>,
}

impl CheckpointTensorImage {
	#[must_use]
	pub const fn dtype(&self) -> DType {
		self.dtype
	}

	#[must_use]
	pub fn shape(&self) -> &[u64] {
		&self.shape
	}

	#[must_use]
	pub fn bytes(&self) -> &[u8] {
		&self.bytes
	}
}

/// Parameter, first moment, and second moment for one weight or bias.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointParameterImage {
	parameter: CheckpointTensorImage,
	first_moment: CheckpointTensorImage,
	second_moment: CheckpointTensorImage,
}

impl CheckpointParameterImage {
	#[must_use]
	pub const fn parameter(&self) -> &CheckpointTensorImage {
		&self.parameter
	}

	#[must_use]
	pub const fn first_moment(&self) -> &CheckpointTensorImage {
		&self.first_moment
	}

	#[must_use]
	pub const fn second_moment(&self) -> &CheckpointTensorImage {
		&self.second_moment
	}
}

/// One effective dense block and its exact final optimizer state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointLayerImage {
	declaration: DenseLayer,
	weight: CheckpointParameterImage,
	bias: CheckpointParameterImage,
}

impl CheckpointLayerImage {
	#[must_use]
	pub const fn declaration(&self) -> &DenseLayer {
		&self.declaration
	}

	#[must_use]
	pub const fn weight(&self) -> &CheckpointParameterImage {
		&self.weight
	}

	#[must_use]
	pub const fn bias(&self) -> &CheckpointParameterImage {
		&self.bias
	}
}

/// One exact ordered step in a saved residual branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointResidualBranchImage {
	Layer(CheckpointLayerImage),
	Operation(DenseOperation),
}

/// The saved residual skip route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointResidualSkipImage {
	Identity,
	/// A learned weight-only linear projection. Residual projections never
	/// carry a bias parameter.
	Projection(CheckpointParameterImage),
}

/// One topology-preserving residual block and its complete optimizer state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointResidualImage {
	branch: Vec<CheckpointResidualBranchImage>,
	output_width: NonZeroU64,
	skip: CheckpointResidualSkipImage,
	operations: Vec<DenseOperation>,
}

impl CheckpointResidualImage {
	#[must_use]
	pub fn branch(&self) -> &[CheckpointResidualBranchImage] {
		&self.branch
	}

	#[must_use]
	pub const fn output_width(&self) -> NonZeroU64 {
		self.output_width
	}

	#[must_use]
	pub const fn skip(&self) -> &CheckpointResidualSkipImage {
		&self.skip
	}

	#[must_use]
	pub fn operations(&self) -> &[DenseOperation] {
		&self.operations
	}
}

/// One effective saved model block. Checkpoint v5 contains only `Layer`;
/// checkpoint v6 additionally retains residual topology.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointBlockImage {
	Layer(CheckpointLayerImage),
	Residual(CheckpointResidualImage),
}

impl CheckpointBlockImage {
	#[must_use]
	pub fn output_width(&self) -> NonZeroU64 {
		match self {
			Self::Layer(layer) => layer.declaration.width(),
			Self::Residual(residual) => residual.output_width,
		}
	}

	#[must_use]
	pub fn output_operations(&self) -> &[DenseOperation] {
		match self {
			Self::Layer(layer) => layer.declaration.operations(),
			Self::Residual(residual) => &residual.operations,
		}
	}
}

/// Fully decoded, validated, native-handle-free checkpoint artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointArtifact {
	format_version: u32,
	vectors: Vec<CheckpointArtifactVector>,
	feature_spans: Vec<CompiledFeatureSpan>,
	feature_normalization_mask: Vec<u32>,
	feature_width: usize,
	task: DenseTask,
	output_adapter: Option<DenseOutputAdapter>,
	config: DenseTrainingConfig,
	bounds: TrainingBounds,
	normalization: Vec<CheckpointTensorImage>,
	layers: Vec<CheckpointLayerImage>,
	blocks: Vec<CheckpointBlockImage>,
	temperature: Option<CheckpointTensorImage>,
}

impl CheckpointArtifact {
	#[must_use]
	pub const fn format_version(&self) -> u32 {
		self.format_version
	}

	#[must_use]
	pub fn vectors(&self) -> &[CheckpointArtifactVector] {
		&self.vectors
	}

	#[must_use]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] {
		&self.feature_spans
	}

	#[must_use]
	pub fn feature_normalization_mask(&self) -> &[u32] {
		&self.feature_normalization_mask
	}

	#[must_use]
	pub const fn feature_width(&self) -> usize {
		self.feature_width
	}

	#[must_use]
	pub const fn task(&self) -> DenseTask {
		self.task
	}

	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> {
		self.output_adapter
	}

	#[must_use]
	pub const fn config(&self) -> &DenseTrainingConfig {
		&self.config
	}

	#[must_use]
	pub const fn bounds(&self) -> TrainingBounds {
		self.bounds
	}

	#[must_use]
	pub fn normalization(&self) -> &[CheckpointTensorImage] {
		&self.normalization
	}

	#[must_use]
	pub fn layers(&self) -> &[CheckpointLayerImage] {
		&self.layers
	}

	/// Effective topology. For a flat v5 artifact this contains the same layers
	/// exposed by [`Self::layers`]. A structured v6 artifact leaves the legacy
	/// flat layer view empty and retains its complete topology here.
	#[must_use]
	pub fn blocks(&self) -> &[CheckpointBlockImage] {
		&self.blocks
	}

	#[must_use]
	pub const fn temperature(&self) -> Option<&CheckpointTensorImage> {
		self.temperature.as_ref()
	}

	/// Decode one predicted multiclass index using only the saved target schema.
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
			.find(|vector| vector.source_index == target_vector && vector.role == VectorRole::Target)?;
		let CheckpointArtifactMetadata::Categorical { dictionary } = &target.metadata else {
			return None;
		};
		if i32::try_from(class).ok() == Some(reserved_code) {
			return Some(DecodedMulticlassClass::ReservedUnseen);
		}
		dictionary
			.get(class)
			.map(|label| DecodedMulticlassClass::Label(label))
	}

	/// Encode this validated artifact in its canonical versioned form.
	pub fn encode(&self) -> CheckpointResult<Vec<u8>> {
		validate_artifact(self)?;
		let mut output = Vec::new();
		encode_artifact(self, &mut output)
			.map_err(|error| CheckpointError::io("encode checkpoint", Path::new("<memory>"), error))?;
		Ok(output)
	}
}

/// Decode and fully validate one bounded checkpoint-v5 document.
pub fn decode_checkpoint(bytes: &[u8], limits: CheckpointDecodeLimits) -> CheckpointResult<CheckpointArtifact> {
	if bytes.len() > limits.source_bytes {
		return Err(decode_error(
			CheckpointDecodeErrorKind::LimitExceeded,
			CheckpointPath::root(),
			format!(
				"source contains {} bytes, limit is {}",
				bytes.len(),
				limits.source_bytes
			),
		));
	}
	let text = core::str::from_utf8(bytes).map_err(|error| {
		decode_error(
			CheckpointDecodeErrorKind::InvalidUtf8,
			CheckpointPath::root(),
			format!("source is not UTF-8: {error}"),
		)
	})?;
	let graph = OgdlGraph::parse(text).map_err(|error| {
		decode_error(
			CheckpointDecodeErrorKind::InvalidSyntax,
			CheckpointPath::root(),
			error.to_string(),
		)
	})?;
	if graph.len() > limits.nodes {
		return Err(decode_error(
			CheckpointDecodeErrorKind::LimitExceeded,
			CheckpointPath::root(),
			format!(
				"document contains {} nodes, limit is {}",
				graph.len(),
				limits.nodes
			),
		));
	}
	Decoder {
		graph: &graph,
		limits,
		block_count: 0,
		tensor_count: 0,
		payload_bytes: 0,
		metadata_entries: 0,
	}
	.decode()
}

fn decode_error(kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: impl Into<String>) -> CheckpointError {
	CheckpointError::Decode(CheckpointDecodeError::new(kind, path, detail))
}

struct Decoder<'a> {
	graph: &'a OgdlGraph,
	limits: CheckpointDecodeLimits,
	block_count: usize,
	tensor_count: usize,
	payload_bytes: usize,
	metadata_entries: usize,
}

struct FieldSet {
	fields: BTreeMap<String, OgdlNodeId>,
}

impl FieldSet {
	fn require(&self, name: &str, path: &CheckpointPath) -> CheckpointResult<OgdlNodeId> {
		self.fields.get(name).copied().ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::MissingField,
				path.field(name),
				format!("required field {name:?} is absent"),
			)
		})
	}

	fn optional(&self, name: &str) -> Option<OgdlNodeId> {
		self.fields.get(name).copied()
	}
}

impl Decoder<'_> {
	fn node(&self, id: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<&recipe_ogdl::Node> {
		self.graph.node(id).ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax,
				path.clone(),
				"OGDL node identity is absent",
			)
		})
	}

	fn fields(
		&self,
		parent: OgdlNodeId,
		path: &CheckpointPath,
		required: &[&str],
		optional: &[&str],
	) -> CheckpointResult<FieldSet> {
		let allowed = required
			.iter()
			.chain(optional)
			.copied()
			.collect::<BTreeSet<_>>();
		let mut fields = BTreeMap::new();
		for child in self.node(parent, path)?.children() {
			let child_node = self.node(*child, path)?;
			let name = child_node.text();
			if !allowed.contains(name) {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					path.field(name),
					format!("field {name:?} is not valid here"),
				));
			}
			if fields.insert(name.to_owned(), *child).is_some() {
				return Err(decode_error(
					CheckpointDecodeErrorKind::DuplicateField,
					path.field(name),
					format!("field {name:?} appears more than once"),
				));
			}
		}
		let fields = FieldSet { fields };
		for name in required {
			fields.require(name, path)?;
		}
		Ok(fields)
	}

	fn scalar<'a>(&'a self, field: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<&'a str> {
		let children = self.node(field, path)?.children();
		if children.len() != 1 {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax,
				path.clone(),
				format!("scalar field has {} values, expected one", children.len()),
			));
		}
		let value = self.node(children[0], path)?;
		if !value.children().is_empty() {
			return Err(decode_error(
				CheckpointDecodeErrorKind::UnknownField,
				path.field(value.children().len().to_string()),
				"scalar value has unexpected descendants",
			));
		}
		Ok(value.text())
	}

	fn tagged<'a>(
		&'a self,
		field: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<(&'a str, &'a [OgdlNodeId])> {
		let children = self.node(field, path)?.children();
		let tag_id = children.first().copied().ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::MissingField,
				path.clone(),
				"tagged field has no tag value",
			)
		})?;
		let tag = self.node(tag_id, path)?;
		if !tag.children().is_empty() {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax,
				path.clone(),
				"tag value has unexpected descendants",
			));
		}
		Ok((tag.text(), &children[1..]))
	}

	fn fields_from_children(
		&self,
		children: &[OgdlNodeId],
		path: &CheckpointPath,
		required: &[&str],
		optional: &[&str],
	) -> CheckpointResult<FieldSet> {
		let allowed = required
			.iter()
			.chain(optional)
			.copied()
			.collect::<BTreeSet<_>>();
		let mut fields = BTreeMap::new();
		for child in children {
			let name = self.node(*child, path)?.text();
			if !allowed.contains(name) {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					path.field(name),
					format!("field {name:?} is not valid here"),
				));
			}
			if fields.insert(name.to_owned(), *child).is_some() {
				return Err(decode_error(
					CheckpointDecodeErrorKind::DuplicateField,
					path.field(name),
					format!("field {name:?} appears more than once"),
				));
			}
		}
		let fields = FieldSet { fields };
		for name in required {
			fields.require(name, path)?;
		}
		Ok(fields)
	}
}

impl Decoder<'_> {
	fn decode(mut self) -> CheckpointResult<CheckpointArtifact> {
		let root_path = CheckpointPath::root().field("recipe");
		if self.graph.roots().len() != 1 {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax,
				CheckpointPath::root(),
				format!(
					"document has {} roots, expected one",
					self.graph.roots().len()
				),
			));
		}
		let root = self.graph.roots()[0];
		if self.node(root, &root_path)?.text() != "recipe" {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				CheckpointPath::root(),
				"root must be `recipe`",
			));
		}
		let fields = self.fields(
			root,
			&root_path,
			&[
				"format",
				"version",
				"semantics",
				"dataset",
				"training",
				"model",
			],
			&[],
		)?;
		self.expect_scalar(
			fields.require("format", &root_path)?,
			&root_path.field("format"),
			"dense-training-checkpoint",
		)?;
		let version = self.parse_u32(
			self.scalar(
				fields.require("version", &root_path)?,
				&root_path.field("version"),
			)?,
			&root_path.field("version"),
		)?;
		if !matches!(version, FLAT_CHECKPOINT_FORMAT_VERSION | STRUCTURED_CHECKPOINT_FORMAT_VERSION) {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				root_path.field("version"),
				format!(
					"checkpoint version is {version}, expected {FLAT_CHECKPOINT_FORMAT_VERSION} or \
					 {STRUCTURED_CHECKPOINT_FORMAT_VERSION}"
				),
			));
		}
		let (loss, data_normalization) = self.parse_semantics(
			fields.require("semantics", &root_path)?,
			&root_path.field("semantics"),
		)?;
		let dataset = self.parse_dataset(
			fields.require("dataset", &root_path)?,
			&root_path.field("dataset"),
		)?;
		let (mut config, bounds) = self.parse_training(
			fields.require("training", &root_path)?,
			&root_path.field("training"),
			loss,
			data_normalization,
		)?;
		let model = self.parse_model(
			fields.require("model", &root_path)?,
			&root_path.field("model"),
			data_normalization,
			dataset.feature_width,
			dataset.task.output_width(),
			version,
		)?;
		if version == FLAT_CHECKPOINT_FORMAT_VERSION {
			config.layers = match model.output_adapter {
				Some(_) => model.layers[..model.layers.len().saturating_sub(1)]
					.iter()
					.map(|layer| layer.declaration.clone())
					.collect(),
				None => model
					.layers
					.iter()
					.map(|layer| layer.declaration.clone())
					.collect(),
			};
		}
		let artifact = CheckpointArtifact {
			format_version: version,
			vectors: dataset.vectors,
			feature_spans: dataset.feature_spans,
			feature_normalization_mask: dataset.feature_normalization_mask,
			feature_width: dataset.feature_width,
			task: dataset.task,
			output_adapter: model.output_adapter,
			config,
			bounds,
			normalization: model.normalization,
			layers: model.layers,
			blocks: model.blocks,
			temperature: model.temperature,
		};
		validate_artifact(&artifact)?;
		Ok(artifact)
	}

	fn parse_semantics(
		&self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<(DenseLoss, DenseDataNormalization)> {
		let fields = self.fields(
			node,
			path,
			&["objective", "normalization", "optimizer"],
			&[],
		)?;
		let loss = self.parse_loss(
			self.scalar(fields.require("objective", path)?, &path.field("objective"))?,
			&path.field("objective"),
		)?;
		let normalization = self.parse_data_normalization(
			self.scalar(
				fields.require("normalization", path)?,
				&path.field("normalization"),
			)?,
			&path.field("normalization"),
		)?;
		self.expect_scalar(
			fields.require("optimizer", path)?,
			&path.field("optimizer"),
			"adamw",
		)?;
		Ok((loss, normalization))
	}
}

impl Decoder<'_> {
	fn expect_scalar(&self, field: OgdlNodeId, path: &CheckpointPath, expected: &str) -> CheckpointResult<()> {
		let actual = self.scalar(field, path)?;
		if actual != expected {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("value is {actual:?}, expected {expected:?}"),
			));
		}
		Ok(())
	}

	fn parse_unsigned<T>(&self, value: &str, path: &CheckpointPath, type_name: &str) -> CheckpointResult<T>
	where
		T: core::str::FromStr,
		T::Err: fmt::Display,
	{
		if value.is_empty()
			|| !value.bytes().all(|byte| byte.is_ascii_digit())
			|| value.len() > 1 && value.starts_with('0')
		{
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("{value:?} is not a canonical unsigned {type_name}"),
			));
		}
		value.parse::<T>().map_err(|error| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("{value:?} cannot be represented as {type_name}: {error}"),
			)
		})
	}

	fn parse_signed<T>(&self, value: &str, path: &CheckpointPath, type_name: &str) -> CheckpointResult<T>
	where
		T: core::str::FromStr,
		T::Err: fmt::Display,
	{
		let digits = value.strip_prefix('-').unwrap_or(value);
		if digits.is_empty()
			|| !digits.bytes().all(|byte| byte.is_ascii_digit())
			|| digits.len() > 1 && digits.starts_with('0')
			|| value == "-0"
		{
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("{value:?} is not a canonical signed {type_name}"),
			));
		}
		value.parse::<T>().map_err(|error| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("{value:?} cannot be represented as {type_name}: {error}"),
			)
		})
	}

	fn parse_u8(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u8> {
		self.parse_unsigned(value, path, "u8")
	}

	fn parse_u32(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u32> {
		self.parse_unsigned(value, path, "u32")
	}

	fn parse_u64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u64> {
		self.parse_unsigned(value, path, "u64")
	}

	fn parse_usize(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<usize> {
		self.parse_unsigned(value, path, "usize")
	}

	fn parse_i32(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<i32> {
		self.parse_signed(value, path, "i32")
	}

	fn parse_i64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<i64> {
		self.parse_signed(value, path, "i64")
	}

	fn parse_nonzero_u64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<NonZeroU64> {
		NonZeroU64::new(self.parse_u64(value, path)?).ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				"value must be nonzero",
			)
		})
	}

	fn parse_nonzero_usize(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<NonZeroUsize> {
		NonZeroUsize::new(self.parse_usize(value, path)?).ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				"value must be nonzero",
			)
		})
	}

	fn parse_f32_bits(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<f32> {
		if value.len() != 10
			|| !value.starts_with("0x")
			|| !value[2..]
				.bytes()
				.all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
		{
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("{value:?} is not one canonical 32-bit lowercase hexadecimal value"),
			));
		}
		let bits = u32::from_str_radix(&value[2..], 16).map_err(|error| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("invalid f32 bit pattern: {error}"),
			)
		})?;
		Ok(f32::from_bits(bits))
	}

	fn parse_hex_bytes(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<Vec<u8>> {
		let Some(hex) = value.strip_prefix("0x") else {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				"byte string must start with `0x`",
			));
		};
		if hex.len() % 2 != 0
			|| !hex
				.bytes()
				.all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
		{
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				path.clone(),
				format!("{value:?} is not canonical lowercase byte hexadecimal"),
			));
		}
		let mut bytes = Vec::with_capacity(hex.len() / 2);
		for pair in hex.as_bytes().chunks_exact(2) {
			let pair = core::str::from_utf8(pair).expect("hexadecimal digits are ASCII");
			bytes.push(u8::from_str_radix(pair, 16).expect("validated hexadecimal pair"));
		}
		Ok(bytes)
	}

	fn parse_loss(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<DenseLoss> {
		match value {
			"binary-cross-entropy-with-logits" => Ok(DenseLoss::BinaryCrossEntropy),
			"mean-squared-error" => Ok(DenseLoss::MeanSquaredError),
			"mean-absolute-error" => Ok(DenseLoss::MeanAbsoluteError),
			"cross-entropy" => Ok(DenseLoss::CrossEntropy),
			"huber-unit-delta" => Ok(DenseLoss::Huber),
			_ => Err(self.invalid_enum(path, "objective", value)),
		}
	}

	fn parse_data_normalization(
		&self,
		value: &str,
		path: &CheckpointPath,
	) -> CheckpointResult<DenseDataNormalization> {
		match value {
			"z-score" => Ok(DenseDataNormalization::ZScore),
			"min-max" => Ok(DenseDataNormalization::MinMax),
			"l2-norm" => Ok(DenseDataNormalization::L2Norm),
			_ => Err(self.invalid_enum(path, "data normalization", value)),
		}
	}

	fn parse_learning_rate_decay(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<LearningRateDecay> {
		match value {
			"linear" => Ok(LearningRateDecay::Linear),
			"cosine" => Ok(LearningRateDecay::Cosine),
			"exponential" => Ok(LearningRateDecay::Exponential),
			_ => Err(self.invalid_enum(path, "learning-rate decay", value)),
		}
	}

	fn invalid_enum(&self, path: &CheckpointPath, description: &str, value: &str) -> CheckpointError {
		decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			format!("unknown {description} {value:?}"),
		)
	}
}

impl Decoder<'_> {
	fn parse_dataset(&mut self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<ParsedDataset> {
		let fields = self.fields(
			node,
			path,
			&[
				"feature-width",
				"target",
				"vectors",
				"feature-spans",
				"feature-normalization-mask",
			],
			&[],
		)?;
		let feature_width = self.parse_usize(
			self.scalar(
				fields.require("feature-width", path)?,
				&path.field("feature-width"),
			)?,
			&path.field("feature-width"),
		)?;
		let task = self.parse_target(fields.require("target", path)?, &path.field("target"))?;
		let vectors = self.parse_vectors(fields.require("vectors", path)?, &path.field("vectors"))?;
		let feature_spans = self.parse_feature_spans(
			fields.require("feature-spans", path)?,
			&path.field("feature-spans"),
		)?;
		let feature_normalization_mask = self.parse_feature_normalization_mask(
			fields.require("feature-normalization-mask", path)?,
			&path.field("feature-normalization-mask"),
		)?;
		Ok(ParsedDataset {
			vectors,
			feature_spans,
			feature_normalization_mask,
			feature_width,
			task,
		})
	}

	fn parse_target(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<DenseTask> {
		let fields = self.fields(
			node,
			path,
			&["source-index", "task"],
			&["positive-code", "class-count", "reserved-unseen-code"],
		)?;
		let source_index = self.parse_usize(
			self.scalar(
				fields.require("source-index", path)?,
				&path.field("source-index"),
			)?,
			&path.field("source-index"),
		)?;
		let task = self.scalar(fields.require("task", path)?, &path.field("task"))?;
		match task {
			"binary-classification" => {
				self.reject_fields(&fields, path, &["class-count", "reserved-unseen-code"])?;
				let positive_code = self.parse_i32(
					self.scalar(
						fields.require("positive-code", path)?,
						&path.field("positive-code"),
					)?,
					&path.field("positive-code"),
				)?;
				Ok(DenseTask::BinaryClassification {
					target_vector: source_index,
					positive_code,
				})
			}
			"multiclass-classification" => {
				self.reject_fields(&fields, path, &["positive-code"])?;
				let class_count = self.parse_usize(
					self.scalar(
						fields.require("class-count", path)?,
						&path.field("class-count"),
					)?,
					&path.field("class-count"),
				)?;
				let reserved_code = self.parse_i32(
					self.scalar(
						fields.require("reserved-unseen-code", path)?,
						&path.field("reserved-unseen-code"),
					)?,
					&path.field("reserved-unseen-code"),
				)?;
				Ok(DenseTask::MulticlassClassification {
					target_vector: source_index,
					class_count,
					reserved_code,
				})
			}
			"scalar-regression" => {
				self.reject_fields(
					&fields,
					path,
					&["positive-code", "class-count", "reserved-unseen-code"],
				)?;
				Ok(DenseTask::ScalarRegression {
					target_vector: source_index,
				})
			}
			_ => Err(self.invalid_enum(&path.field("task"), "dense task", task)),
		}
	}

	fn reject_fields(&self, fields: &FieldSet, path: &CheckpointPath, names: &[&str]) -> CheckpointResult<()> {
		if let Some(name) = names.iter().find(|name| fields.optional(name).is_some()) {
			return Err(decode_error(
				CheckpointDecodeErrorKind::UnknownField,
				path.field(*name),
				format!("field {name:?} is not valid for this tagged variant"),
			));
		}
		Ok(())
	}

	fn parse_vectors(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointArtifactVector>> {
		let children = self.node(node, path)?.children().to_vec();
		if children.len() > self.limits.vectors {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"vector count is {}, limit is {}",
					children.len(),
					self.limits.vectors
				),
			));
		}
		let mut vectors = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let vector_path = path.index(index);
			if self.node(child, &vector_path)?.text() != "vector" {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					vector_path,
					format!(
						"expected `vector`, found {:?}",
						self.node(child, path)?.text()
					),
				));
			}
			vectors.push(self.parse_vector(child, &path.index(index))?);
		}
		Ok(vectors)
	}

	fn parse_vector(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<CheckpointArtifactVector> {
		let fields = self.fields(
			node,
			path,
			&[
				"source-index",
				"name-bytes",
				"role",
				"semantic-type",
				"encoding",
				"metadata",
			],
			&[],
		)?;
		let source_index = self.parse_usize(
			self.scalar(
				fields.require("source-index", path)?,
				&path.field("source-index"),
			)?,
			&path.field("source-index"),
		)?;
		let name = self.parse_hex_bytes(
			self.scalar(
				fields.require("name-bytes", path)?,
				&path.field("name-bytes"),
			)?,
			&path.field("name-bytes"),
		)?;
		let role = self.parse_vector_role(
			self.scalar(fields.require("role", path)?, &path.field("role"))?,
			&path.field("role"),
		)?;
		let semantic_type = self.parse_semantic_type(
			self.scalar(
				fields.require("semantic-type", path)?,
				&path.field("semantic-type"),
			)?,
			&path.field("semantic-type"),
		)?;
		let encoding = self.parse_vector_encoding(
			self.scalar(fields.require("encoding", path)?, &path.field("encoding"))?,
			&path.field("encoding"),
		)?;
		let metadata = self.parse_vector_metadata(fields.require("metadata", path)?, &path.field("metadata"))?;
		Ok(CheckpointArtifactVector {
			source_index,
			name,
			role,
			semantic_type,
			encoding,
			metadata,
		})
	}

	fn parse_vector_role(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<VectorRole> {
		match value {
			"feature" => Ok(VectorRole::Feature),
			"target" => Ok(VectorRole::Target),
			_ => Err(self.invalid_enum(path, "vector role", value)),
		}
	}

	fn parse_semantic_type(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<SemanticType> {
		match value {
			"numeric" => Ok(SemanticType::Numeric),
			"temporal" => Ok(SemanticType::Temporal),
			"categorical" => Ok(SemanticType::Categorical),
			"ordinal" => Ok(SemanticType::Ordinal),
			"text" => Ok(SemanticType::Text),
			"image" => Ok(SemanticType::Image),
			"binary" => Ok(SemanticType::Binary),
			_ => Err(self.invalid_enum(path, "semantic type", value)),
		}
	}

	fn parse_vector_encoding(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<VectorEncoding> {
		match value {
			"f32" => Ok(VectorEncoding::F32),
			"int32" => Ok(VectorEncoding::I32),
			"relative-seconds-int32" => Ok(VectorEncoding::RelativeSecondsI32),
			"dictionary-int32" => Ok(VectorEncoding::DictionaryI32),
			"ordinal-int32" => Ok(VectorEncoding::OrdinalI32),
			"utf8" => Ok(VectorEncoding::Utf8),
			"bytes" => Ok(VectorEncoding::Bytes),
			_ => Err(self.invalid_enum(path, "vector encoding", value)),
		}
	}

	fn parse_vector_metadata(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<CheckpointArtifactMetadata> {
		let (tag, payload) = self.tagged(node, path)?;
		let tag = tag.to_owned();
		let payload = payload.to_vec();
		match tag.as_str() {
			"none" => {
				self.require_empty(&payload, path)?;
				Ok(CheckpointArtifactMetadata::None)
			}
			"temporal" => {
				let fields =
					self.fields_from_children(&payload, path, &["unix-seconds", "nanoseconds"], &[])?;
				let unix_seconds = self.parse_i64(
					self.scalar(
						fields.require("unix-seconds", path)?,
						&path.field("unix-seconds"),
					)?,
					&path.field("unix-seconds"),
				)?;
				let nanoseconds = self.parse_u32(
					self.scalar(
						fields.require("nanoseconds", path)?,
						&path.field("nanoseconds"),
					)?,
					&path.field("nanoseconds"),
				)?;
				Ok(CheckpointArtifactMetadata::Temporal {
					unix_seconds,
					nanoseconds,
				})
			}
			"categorical" => Ok(CheckpointArtifactMetadata::Categorical {
				dictionary: self.parse_byte_entries(&payload, path, "value-bytes")?,
			}),
			"ordinal" => Ok(CheckpointArtifactMetadata::Ordinal {
				ordered_labels: self.parse_byte_entries(&payload, path, "value-bytes")?,
			}),
			"image" => Ok(CheckpointArtifactMetadata::Image {
				encoded_variants: self.parse_image_variants(&payload, path)?,
			}),
			_ => Err(self.invalid_enum(path, "vector metadata", &tag)),
		}
	}

	fn require_empty(&self, children: &[OgdlNodeId], path: &CheckpointPath) -> CheckpointResult<()> {
		if let Some(child) = children.first() {
			let name = self.node(*child, path)?.text();
			return Err(decode_error(
				CheckpointDecodeErrorKind::UnknownField,
				path.field(name),
				format!("tagged value has unexpected field {name:?}"),
			));
		}
		Ok(())
	}

	fn reserve_metadata_entries(&mut self, added: usize, path: &CheckpointPath) -> CheckpointResult<()> {
		self.metadata_entries = self.metadata_entries.checked_add(added).ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				"metadata entry count overflowed usize",
			)
		})?;
		if self.metadata_entries > self.limits.metadata_entries {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"metadata entry count is {}, limit is {}",
					self.metadata_entries, self.limits.metadata_entries
				),
			));
		}
		Ok(())
	}

	fn parse_byte_entries(
		&mut self,
		children: &[OgdlNodeId],
		path: &CheckpointPath,
		expected_name: &str,
	) -> CheckpointResult<Vec<Vec<u8>>> {
		self.reserve_metadata_entries(children.len(), path)?;
		let mut entries = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let entry_path = path.index(index);
			let node = self.node(child, &entry_path)?;
			if node.text() != expected_name {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					entry_path,
					format!("expected {expected_name:?}, found {:?}", node.text()),
				));
			}
			entries.push(self.parse_hex_bytes(self.scalar(child, &path.index(index))?, &path.index(index))?);
		}
		Ok(entries)
	}

	fn parse_image_variants(
		&mut self,
		children: &[OgdlNodeId],
		path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointImageMetadata>> {
		self.reserve_metadata_entries(children.len(), path)?;
		let mut variants = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let variant_path = path.index(index);
			if self.node(child, &variant_path)?.text() != "variant" {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					variant_path,
					format!(
						"expected `variant`, found {:?}",
						self.node(child, path)?.text()
					),
				));
			}
			let fields = self.fields(
				child,
				&variant_path,
				&[
					"format",
					"width",
					"height",
					"channels",
					"color-model",
					"sample-bits",
					"value-layout",
					"value-range",
				],
				&[],
			)?;
			let format_path = variant_path.field("format");
			let format = self.parse_image_format(
				self.scalar(fields.require("format", &variant_path)?, &format_path)?,
				&format_path,
			)?;
			let width = self.parse_u32(
				self.scalar(
					fields.require("width", &variant_path)?,
					&variant_path.field("width"),
				)?,
				&variant_path.field("width"),
			)?;
			let height = self.parse_u32(
				self.scalar(
					fields.require("height", &variant_path)?,
					&variant_path.field("height"),
				)?,
				&variant_path.field("height"),
			)?;
			let channels = self.parse_optional_u8(
				self.scalar(
					fields.require("channels", &variant_path)?,
					&variant_path.field("channels"),
				)?,
				&variant_path.field("channels"),
			)?;
			let color_model = self.parse_optional_image_color_model(
				self.scalar(
					fields.require("color-model", &variant_path)?,
					&variant_path.field("color-model"),
				)?,
				&variant_path.field("color-model"),
			)?;
			let sample_bits = self.parse_optional_u8(
				self.scalar(
					fields.require("sample-bits", &variant_path)?,
					&variant_path.field("sample-bits"),
				)?,
				&variant_path.field("sample-bits"),
			)?;
			self.expect_scalar(
				fields.require("value-layout", &variant_path)?,
				&variant_path.field("value-layout"),
				"encoded-file",
			)?;
			self.expect_scalar(
				fields.require("value-range", &variant_path)?,
				&variant_path.field("value-range"),
				"encoded-bytes",
			)?;
			variants.push(CheckpointImageMetadata {
				format,
				width,
				height,
				channels,
				color_model,
				sample_bits,
				value_layout: ImageValueLayout::EncodedFile,
				value_range: ImageValueRange::EncodedBytes,
			});
		}
		Ok(variants)
	}

	fn parse_optional_u8(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<Option<u8>> {
		if value == "none" {
			Ok(None)
		} else {
			self.parse_u8(value, path).map(Some)
		}
	}

	fn parse_image_format(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<EncodedImageFormat> {
		match value {
			"png" => Ok(EncodedImageFormat::Png),
			"jpeg" => Ok(EncodedImageFormat::Jpeg),
			"gif87a" => Ok(EncodedImageFormat::Gif87a),
			"gif89a" => Ok(EncodedImageFormat::Gif89a),
			"bmp" => Ok(EncodedImageFormat::Bmp),
			"webp" => Ok(EncodedImageFormat::WebP),
			_ => Err(self.invalid_enum(path, "encoded image format", value)),
		}
	}

	fn parse_optional_image_color_model(
		&self,
		value: &str,
		path: &CheckpointPath,
	) -> CheckpointResult<Option<ImageColorModel>> {
		let model = match value {
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
		};
		Ok(Some(model))
	}

	fn parse_feature_spans(
		&self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<Vec<CompiledFeatureSpan>> {
		let children = self.node(node, path)?.children();
		if children.len() > self.limits.feature_spans {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"feature-span count is {}, limit is {}",
					children.len(),
					self.limits.feature_spans
				),
			));
		}
		let mut spans = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let span_path = path.index(index);
			if self.node(child, &span_path)?.text() != "span" {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					span_path,
					format!(
						"expected `span`, found {:?}",
						self.node(child, path)?.text()
					),
				));
			}
			let fields = self.fields(
				child,
				&span_path,
				&["source-index", "start", "width", "lowering"],
				&[],
			)?;
			let source_vector = self.parse_usize(
				self.scalar(
					fields.require("source-index", &span_path)?,
					&span_path.field("source-index"),
				)?,
				&span_path.field("source-index"),
			)?;
			let start = self.parse_usize(
				self.scalar(
					fields.require("start", &span_path)?,
					&span_path.field("start"),
				)?,
				&span_path.field("start"),
			)?;
			let width = self.parse_usize(
				self.scalar(
					fields.require("width", &span_path)?,
					&span_path.field("width"),
				)?,
				&span_path.field("width"),
			)?;
			let lowering = self.parse_feature_lowering(
				fields.require("lowering", &span_path)?,
				&span_path.field("lowering"),
			)?;
			spans.push(CompiledFeatureSpan::new(
				source_vector,
				start,
				width,
				lowering,
			));
		}
		Ok(spans)
	}

	fn parse_feature_lowering(
		&self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<DenseFeatureLowering> {
		let (tag, payload) = self.tagged(node, path)?;
		match tag {
			"numeric-scalar" => {
				self.require_empty(payload, path)?;
				Ok(DenseFeatureLowering::NumericScalar)
			}
			"categorical-one-hot" => {
				let fields =
					self.fields_from_children(payload, path, &["dictionary-width", "reserved-index"], &[])?;
				let dictionary_width = self.parse_usize(
					self.scalar(
						fields.require("dictionary-width", path)?,
						&path.field("dictionary-width"),
					)?,
					&path.field("dictionary-width"),
				)?;
				let reserved_index = self.parse_usize(
					self.scalar(
						fields.require("reserved-index", path)?,
						&path.field("reserved-index"),
					)?,
					&path.field("reserved-index"),
				)?;
				Ok(DenseFeatureLowering::CategoricalOneHot {
					dictionary_width,
					reserved_index,
				})
			}
			_ => Err(self.invalid_enum(path, "feature lowering", tag)),
		}
	}

	fn parse_feature_normalization_mask(
		&self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<Vec<u32>> {
		let children = self.node(node, path)?.children();
		let mut mask = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let value_path = path.index(index);
			if self.node(child, &value_path)?.text() != "value-bits" {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					value_path,
					format!(
						"expected `value-bits`, found {:?}",
						self.node(child, path)?.text()
					),
				));
			}
			mask.push(self
				.parse_f32_bits(self.scalar(child, &path.index(index))?, &path.index(index))?
				.to_bits());
		}
		Ok(mask)
	}
}

impl Decoder<'_> {
	fn parse_training(
		&self,
		node: OgdlNodeId,
		path: &CheckpointPath,
		loss: DenseLoss,
		data_normalization: DenseDataNormalization,
	) -> CheckpointResult<(DenseTrainingConfig, TrainingBounds)> {
		let fields = self.fields(
			node,
			path,
			&[
				"batch-size",
				"epochs",
				"warmup-epochs",
				"learning-rate-decay",
				"gradient-clip-norm",
				"normalization-epsilon",
				"reduction-tree-lanes",
				"random-seed",
				"adamw",
				"bounds",
			],
			&[],
		)?;
		let batch_size = self.parse_nonzero_usize(
			self.scalar(
				fields.require("batch-size", path)?,
				&path.field("batch-size"),
			)?,
			&path.field("batch-size"),
		)?;
		let epochs = self.parse_nonzero_u64(
			self.scalar(fields.require("epochs", path)?, &path.field("epochs"))?,
			&path.field("epochs"),
		)?;
		let warmup_epochs = self.parse_u64(
			self.scalar(
				fields.require("warmup-epochs", path)?,
				&path.field("warmup-epochs"),
			)?,
			&path.field("warmup-epochs"),
		)?;
		let learning_rate_decay = self.parse_learning_rate_decay(
			self.scalar(
				fields.require("learning-rate-decay", path)?,
				&path.field("learning-rate-decay"),
			)?,
			&path.field("learning-rate-decay"),
		)?;
		let gradient_clip_norm = self.parse_f32_bits(
			self.scalar(
				fields.require("gradient-clip-norm", path)?,
				&path.field("gradient-clip-norm"),
			)?,
			&path.field("gradient-clip-norm"),
		)?;
		let normalization_epsilon = self.parse_f32_bits(
			self.scalar(
				fields.require("normalization-epsilon", path)?,
				&path.field("normalization-epsilon"),
			)?,
			&path.field("normalization-epsilon"),
		)?;
		let reduction_tree_lanes = self.parse_u32(
			self.scalar(
				fields.require("reduction-tree-lanes", path)?,
				&path.field("reduction-tree-lanes"),
			)?,
			&path.field("reduction-tree-lanes"),
		)?;
		let random_seed = self.parse_u64(
			self.scalar(
				fields.require("random-seed", path)?,
				&path.field("random-seed"),
			)?,
			&path.field("random-seed"),
		)?;
		let adamw = self.parse_adamw(fields.require("adamw", path)?, &path.field("adamw"))?;
		let bounds = self.parse_bounds(fields.require("bounds", path)?, &path.field("bounds"))?;
		Ok((
			DenseTrainingConfig {
				layers: Vec::new(),
				loss,
				data_normalization,
				batch_size,
				epochs,
				warmup_epochs,
				learning_rate_decay,
				gradient_clip_norm,
				normalization_epsilon,
				reduction_tree_lanes,
				random_seed,
				adamw,
			},
			bounds,
		))
	}

	fn parse_adamw(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<AdamWConfig> {
		let fields = self.fields(
			node,
			path,
			&[
				"learning-rate",
				"beta-one",
				"beta-two",
				"epsilon",
				"weight-decay",
			],
			&[],
		)?;
		Ok(AdamWConfig {
			learning_rate: self.parse_f32_bits(
				self.scalar(
					fields.require("learning-rate", path)?,
					&path.field("learning-rate"),
				)?,
				&path.field("learning-rate"),
			)?,
			beta_one: self.parse_f32_bits(
				self.scalar(fields.require("beta-one", path)?, &path.field("beta-one"))?,
				&path.field("beta-one"),
			)?,
			beta_two: self.parse_f32_bits(
				self.scalar(fields.require("beta-two", path)?, &path.field("beta-two"))?,
				&path.field("beta-two"),
			)?,
			epsilon: self.parse_f32_bits(
				self.scalar(fields.require("epsilon", path)?, &path.field("epsilon"))?,
				&path.field("epsilon"),
			)?,
			weight_decay: self.parse_f32_bits(
				self.scalar(
					fields.require("weight-decay", path)?,
					&path.field("weight-decay"),
				)?,
				&path.field("weight-decay"),
			)?,
		})
	}

	fn parse_bounds(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<TrainingBounds> {
		let fields = self.fields(
			node,
			path,
			&[
				"train-rows",
				"batch-size",
				"batches-per-epoch",
				"padded-rows-per-epoch",
				"epochs",
				"training-iterations",
				"calibration-iterations",
				"iterations",
				"warmup-iterations",
			],
			&[],
		)?;
		let u64_field = |name: &str| -> CheckpointResult<u64> {
			self.parse_u64(
				self.scalar(fields.require(name, path)?, &path.field(name))?,
				&path.field(name),
			)
		};
		Ok(TrainingBounds {
			train_rows: u64_field("train-rows")?,
			batch_size: u64_field("batch-size")?,
			batches_per_epoch: u64_field("batches-per-epoch")?,
			padded_rows_per_epoch: u64_field("padded-rows-per-epoch")?,
			epochs: self.parse_nonzero_u64(
				self.scalar(fields.require("epochs", path)?, &path.field("epochs"))?,
				&path.field("epochs"),
			)?,
			training_iterations: self.parse_nonzero_u64(
				self.scalar(
					fields.require("training-iterations", path)?,
					&path.field("training-iterations"),
				)?,
				&path.field("training-iterations"),
			)?,
			calibration_iterations: u64_field("calibration-iterations")?,
			iterations: self.parse_nonzero_u64(
				self.scalar(
					fields.require("iterations", path)?,
					&path.field("iterations"),
				)?,
				&path.field("iterations"),
			)?,
			warmup_iterations: u64_field("warmup-iterations")?,
		})
	}
}

impl Decoder<'_> {
	fn parse_model(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
		data_normalization: DenseDataNormalization,
		expected_input_width: usize,
		expected_output_width: usize,
		format_version: u32,
	) -> CheckpointResult<ParsedModel> {
		let fields = self.fields(
			node,
			path,
			&["input-width", "output-width", "normalization", "blocks"],
			&["output-adapter", "calibration"],
		)?;
		let input_width = self.parse_usize(
			self.scalar(
				fields.require("input-width", path)?,
				&path.field("input-width"),
			)?,
			&path.field("input-width"),
		)?;
		if input_width != expected_input_width {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("input-width"),
				format!("model input width is {input_width}, dataset feature width is {expected_input_width}"),
			));
		}
		let output_width = self.parse_usize(
			self.scalar(
				fields.require("output-width", path)?,
				&path.field("output-width"),
			)?,
			&path.field("output-width"),
		)?;
		if output_width != expected_output_width {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("output-width"),
				format!("model output width is {output_width}, task output width is {expected_output_width}"),
			));
		}
		let output_adapter = fields
			.optional("output-adapter")
			.map(|adapter| self.parse_output_adapter(adapter, &path.field("output-adapter")))
			.transpose()?;
		let normalization = self.parse_model_normalization(
			fields.require("normalization", path)?,
			&path.field("normalization"),
			data_normalization,
		)?;
		let blocks = self.parse_blocks(
			fields.require("blocks", path)?,
			&path.field("blocks"),
			format_version,
		)?;
		let layers = if format_version == FLAT_CHECKPOINT_FORMAT_VERSION {
			blocks
				.iter()
				.map(|block| match block {
					CheckpointBlockImage::Layer(layer) => Ok(layer.clone()),
					CheckpointBlockImage::Residual(_) => Err(decode_error(
						CheckpointDecodeErrorKind::InvalidValue,
						path.field("blocks"),
						"checkpoint v5 cannot contain a residual block",
					)),
				})
				.collect::<CheckpointResult<Vec<_>>>()?
		} else {
			Vec::new()
		};
		let temperature = fields
			.optional("calibration")
			.map(|calibration| self.parse_calibration(calibration, &path.field("calibration")))
			.transpose()?
			.flatten();
		Ok(ParsedModel {
			output_adapter,
			normalization,
			layers,
			blocks,
			temperature,
		})
	}

	fn parse_output_adapter(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<DenseOutputAdapter> {
		let (tag, payload) = self.tagged(node, path)?;
		if tag != "linear-projection" {
			return Err(self.invalid_enum(path, "output adapter", tag));
		}
		let fields = self.fields_from_children(payload, path, &["source-width", "target-width"], &[])?;
		let source_width = self.parse_nonzero_u64(
			self.scalar(
				fields.require("source-width", path)?,
				&path.field("source-width"),
			)?,
			&path.field("source-width"),
		)?;
		let target_width = self.parse_nonzero_u64(
			self.scalar(
				fields.require("target-width", path)?,
				&path.field("target-width"),
			)?,
			&path.field("target-width"),
		)?;
		Ok(DenseOutputAdapter::new(source_width, target_width))
	}

	fn parse_model_normalization(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
		expected: DenseDataNormalization,
	) -> CheckpointResult<Vec<CheckpointTensorImage>> {
		let (tag, payload) = self.tagged(node, path)?;
		let tag = tag.to_owned();
		let payload = payload.to_vec();
		let actual = self.parse_data_normalization(&tag, path)?;
		if actual != expected {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.clone(),
				format!(
					"model normalization {tag:?} differs from semantic normalization {:?}",
					data_normalization(expected)
				),
			));
		}
		let names: &[&str] = match actual {
			DenseDataNormalization::ZScore => &["mean", "variance"],
			DenseDataNormalization::MinMax => &["minimum", "maximum"],
			DenseDataNormalization::L2Norm => &[],
		};
		let fields = self.fields_from_children(&payload, path, names, &[])?;
		let mut tensors = Vec::with_capacity(names.len());
		for name in names {
			tensors.push(self.parse_tensor(fields.require(name, path)?, &path.field(*name))?);
		}
		Ok(tensors)
	}

	fn parse_blocks(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
		format_version: u32,
	) -> CheckpointResult<Vec<CheckpointBlockImage>> {
		let children = self.node(node, path)?.children().to_vec();
		if children.len() > self.limits.layers {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"block count is {}, limit is {}",
					children.len(),
					self.limits.layers
				),
			));
		}
		let mut blocks = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let block_path = path.index(index);
			self.claim_block(&block_path)?;
			let block = match self.node(child, &block_path)?.text() {
				"layer" | "perc" => CheckpointBlockImage::Layer(self.parse_layer(child, &block_path, index)?),
				"residual" if format_version == STRUCTURED_CHECKPOINT_FORMAT_VERSION => {
					CheckpointBlockImage::Residual(self.parse_residual(child, &block_path, index)?)
				}
				"residual" => {
					return Err(decode_error(
						CheckpointDecodeErrorKind::InvalidValue,
						block_path,
						"checkpoint v5 cannot contain a residual block",
					));
				}
				value => return Err(self.invalid_enum(&block_path, "dense block", value)),
			};
			blocks.push(block);
		}
		Ok(blocks)
	}

	fn claim_block(&mut self, path: &CheckpointPath) -> CheckpointResult<()> {
		self.block_count = self.block_count.checked_add(1).ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				"block and branch-step count overflowed usize",
			)
		})?;
		if self.block_count > self.limits.layers {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"block and branch-step count is {}, limit is {}",
					self.block_count, self.limits.layers
				),
			));
		}
		Ok(())
	}

	fn parse_layer(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
		expected_index: usize,
	) -> CheckpointResult<CheckpointLayerImage> {
		let kind = match self.node(node, path)?.text() {
			"layer" => DenseBlockKind::Layer,
			"perc" => DenseBlockKind::Perc,
			value => return Err(self.invalid_enum(path, "dense block", value)),
		};
		let (width, payload) = self.tagged(node, path)?;
		let width = width.to_owned();
		let payload = payload.to_vec();
		let width = self.parse_nonzero_u64(&width, &path.field("width"))?;
		let fields = self.fields_from_children(
			&payload,
			path,
			&["index", "operations", "weight", "bias"],
			&[],
		)?;
		let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?;
		if index != expected_index {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			));
		}
		let operations = self.parse_operations(
			fields.require("operations", path)?,
			&path.field("operations"),
		)?;
		let weight = self.parse_parameter(fields.require("weight", path)?, &path.field("weight"))?;
		let bias = self.parse_parameter(fields.require("bias", path)?, &path.field("bias"))?;
		Ok(CheckpointLayerImage {
			declaration: DenseLayer::with_kind(kind, width, operations),
			weight,
			bias,
		})
	}

	fn parse_residual(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
		expected_index: usize,
	) -> CheckpointResult<CheckpointResidualImage> {
		let fields = self.fields(
			node,
			path,
			&["index", "output-width", "branch", "skip", "operations"],
			&[],
		)?;
		let index = self.parse_usize(
			self.scalar(fields.require("index", path)?, &path.field("index"))?,
			&path.field("index"),
		)?;
		if index != expected_index {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("index"),
				format!("serialized block index is {index}, expected {expected_index}"),
			));
		}
		let output_width = self.parse_nonzero_u64(
			self.scalar(
				fields.require("output-width", path)?,
				&path.field("output-width"),
			)?,
			&path.field("output-width"),
		)?;
		let branch = self.parse_residual_branch(fields.require("branch", path)?, &path.field("branch"))?;
		let declared_output = branch.iter().rev().find_map(|step| match step {
			CheckpointResidualBranchImage::Layer(layer) => Some(layer.declaration.width()),
			CheckpointResidualBranchImage::Operation(_) => None,
		});
		if declared_output != Some(output_width) {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("output-width"),
				format!(
					"residual output width is {}, but the last branch layer yields {:?}",
					output_width,
					declared_output.map(NonZeroU64::get)
				),
			));
		}
		let skip = self.parse_residual_skip(fields.require("skip", path)?, &path.field("skip"))?;
		let operations = self.parse_operations(
			fields.require("operations", path)?,
			&path.field("operations"),
		)?;
		Ok(CheckpointResidualImage {
			branch,
			output_width,
			skip,
			operations,
		})
	}

	fn parse_residual_branch(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointResidualBranchImage>> {
		let (count, payload) = self.tagged(node, path)?;
		let count = self.parse_usize(count, &path.field("count"))?;
		let payload = payload.to_vec();
		if count != payload.len() {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("count"),
				format!("branch step count is {count}, but {} steps follow", payload.len()),
			));
		}
		let mut branch = Vec::with_capacity(count);
		for (index, step) in payload.into_iter().enumerate() {
			let step_path = path.index(index);
			self.claim_block(&step_path)?;
			match self.node(step, &step_path)?.text() {
				"layer" | "perc" => branch.push(CheckpointResidualBranchImage::Layer(
					self.parse_layer(step, &step_path, index)?,
				)),
				"operation" => branch.push(CheckpointResidualBranchImage::Operation(
					self.parse_dense_operation(self.scalar(step, &step_path)?, &step_path)?,
				)),
				value => return Err(self.invalid_enum(&step_path, "residual branch step", value)),
			}
		}
		Ok(branch)
	}

	fn parse_residual_skip(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<CheckpointResidualSkipImage> {
		let (tag, payload) = self.tagged(node, path)?;
		let payload = payload.to_vec();
		match tag {
			"identity" => {
				if !payload.is_empty() {
					return Err(decode_error(
						CheckpointDecodeErrorKind::UnknownField,
						path.clone(),
						"identity skip has unexpected descendants",
					));
				}
				Ok(CheckpointResidualSkipImage::Identity)
			}
			"linear-projection" => {
				let fields = self.fields_from_children(&payload, path, &["weight"], &[])?;
				Ok(CheckpointResidualSkipImage::Projection(
					self.parse_parameter(fields.require("weight", path)?, &path.field("weight"))?,
				))
			}
			value => Err(self.invalid_enum(path, "residual skip", value)),
		}
	}

	fn parse_operations(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Vec<DenseOperation>> {
		let (count, payload) = self.tagged(node, path)?;
		let count = self.parse_usize(count, &path.field("count"))?;
		if count != payload.len() {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InconsistentValue,
				path.field("count"),
				format!(
					"operation count is {count}, but {} operations follow",
					payload.len()
				),
			));
		}
		let mut operations = Vec::with_capacity(count);
		for (index, operation) in payload.iter().copied().enumerate() {
			let operation_path = path.index(index);
			if self.node(operation, &operation_path)?.text() != "operation" {
				return Err(decode_error(
					CheckpointDecodeErrorKind::UnknownField,
					operation_path,
					format!(
						"expected `operation`, found {:?}",
						self.node(operation, path)?.text()
					),
				));
			}
			operations.push(self.parse_dense_operation(
				self.scalar(operation, &path.index(index))?,
				&path.index(index),
			)?);
		}
		Ok(operations)
	}

	fn parse_dense_operation(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<DenseOperation> {
		let operation = match value {
			"linear" => DenseOperation::Activation(DenseActivation::Linear),
			"cosine" => DenseOperation::Activation(DenseActivation::Cosine),
			"exponential" => DenseOperation::Activation(DenseActivation::Exponential),
			"logarithm" => DenseOperation::Activation(DenseActivation::Logarithm),
			"huber" => DenseOperation::Activation(DenseActivation::Huber),
			"tangent" => DenseOperation::Activation(DenseActivation::Tangent),
			"relu" => DenseOperation::Activation(DenseActivation::Relu),
			"gelu" => DenseOperation::Activation(DenseActivation::Gelu),
			"silu" => DenseOperation::Activation(DenseActivation::Silu),
			"layer-normalization" => DenseOperation::Normalization(crate::DenseNormalization::Layer),
			"batch-normalization" => DenseOperation::Normalization(crate::DenseNormalization::Batch),
			_ => return Err(self.invalid_enum(path, "dense operation", value)),
		};
		Ok(operation)
	}

	fn parse_parameter(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<CheckpointParameterImage> {
		let fields = self.fields(
			node,
			path,
			&["parameter", "first-moment", "second-moment"],
			&[],
		)?;
		Ok(CheckpointParameterImage {
			parameter: self.parse_tensor(fields.require("parameter", path)?, &path.field("parameter"))?,
			first_moment: self.parse_tensor(
				fields.require("first-moment", path)?,
				&path.field("first-moment"),
			)?,
			second_moment: self.parse_tensor(
				fields.require("second-moment", path)?,
				&path.field("second-moment"),
			)?,
		})
	}

	fn parse_tensor(&mut self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<CheckpointTensorImage> {
		self.tensor_count = self.tensor_count.checked_add(1).ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				"tensor count overflowed usize",
			)
		})?;
		if self.tensor_count > self.limits.tensors {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"tensor count is {}, limit is {}",
					self.tensor_count, self.limits.tensors
				),
			));
		}
		let fields = self.fields(node, path, &["dtype", "shape", "payload"], &[])?;
		let dtype = self.parse_dtype(
			self.scalar(fields.require("dtype", path)?, &path.field("dtype"))?,
			&path.field("dtype"),
		)?;
		let shape = self.parse_shape(fields.require("shape", path)?, &path.field("shape"))?;
		let bytes = self.parse_payload(fields.require("payload", path)?, &path.field("payload"))?;
		Ok(CheckpointTensorImage {
			dtype,
			shape,
			bytes,
		})
	}

	fn parse_dtype(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<DType> {
		match value {
			"f32" => Ok(DType::F32),
			"int32" => Ok(DType::I32),
			_ => Err(self.invalid_enum(path, "tensor dtype", value)),
		}
	}

	fn parse_shape(&self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Vec<u64>> {
		let mut shape = Vec::new();
		let mut children = self.node(node, path)?.children();
		while let Some(extent) = children.first().copied() {
			if children.len() != 1 {
				return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidSyntax,
					path.index(shape.len()),
					"shape chain branches instead of containing one ordered extent",
				));
			}
			if shape.len() == self.limits.tensor_rank {
				return Err(decode_error(
					CheckpointDecodeErrorKind::LimitExceeded,
					path.clone(),
					format!("tensor rank exceeds limit {}", self.limits.tensor_rank),
				));
			}
			let extent_node = self.node(extent, &path.index(shape.len()))?;
			shape.push(self.parse_u64(extent_node.text(), &path.index(shape.len()))?);
			children = extent_node.children();
		}
		Ok(shape)
	}

	fn parse_payload(&mut self, node: OgdlNodeId, path: &CheckpointPath) -> CheckpointResult<Vec<u8>> {
		let (tag, chunks) = self.tagged(node, path)?;
		if tag != "raw-bytes-hex" {
			return Err(self.invalid_enum(path, "tensor payload encoding", tag));
		}
		let chunks = chunks.to_vec();
		if chunks.is_empty() {
			return Err(decode_error(
				CheckpointDecodeErrorKind::MissingField,
				path.clone(),
				"raw hexadecimal payload has no chunks",
			));
		}
		let mut decoded_size = 0usize;
		for (index, chunk) in chunks.iter().copied().enumerate() {
			let chunk_path = path.index(index);
			let chunk = self.node(chunk, &chunk_path)?;
			if !chunk.children().is_empty() {
				return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidSyntax,
					chunk_path,
					"payload chunk has unexpected descendants",
				));
			}
			let Some(hex) = chunk.text().strip_prefix("0x") else {
				return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidValue,
					chunk_path,
					"payload chunk must start with `0x`",
				));
			};
			if hex.len() % 2 != 0 || hex.len() > HEX_CHUNK_BYTES * 2 {
				return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidValue,
					chunk_path,
					format!("payload chunk contains {} hexadecimal digits", hex.len()),
				));
			}
			let chunk_bytes = hex.len() / 2;
			if index + 1 != chunks.len() && chunk_bytes != HEX_CHUNK_BYTES {
				return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidValue,
					chunk_path,
					format!("nonfinal payload chunk has {chunk_bytes} bytes, expected {HEX_CHUNK_BYTES}"),
				));
			}
			if chunk_bytes == 0 && chunks.len() != 1 {
				return Err(decode_error(
					CheckpointDecodeErrorKind::InvalidValue,
					chunk_path,
					"empty payload chunk must be the only chunk",
				));
			}
			decoded_size = decoded_size.checked_add(chunk_bytes).ok_or_else(|| {
				decode_error(
					CheckpointDecodeErrorKind::LimitExceeded,
					path.clone(),
					"tensor payload size overflowed usize",
				)
			})?;
		}
		if decoded_size > self.limits.tensor_bytes {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"tensor payload has {decoded_size} bytes, limit is {}",
					self.limits.tensor_bytes
				),
			));
		}
		self.payload_bytes = self
			.payload_bytes
			.checked_add(decoded_size)
			.ok_or_else(|| {
				decode_error(
					CheckpointDecodeErrorKind::LimitExceeded,
					path.clone(),
					"total tensor payload size overflowed usize",
				)
			})?;
		if self.payload_bytes > self.limits.total_payload_bytes {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				path.clone(),
				format!(
					"decoded payload total is {} bytes, limit is {}",
					self.payload_bytes, self.limits.total_payload_bytes
				),
			));
		}
		let mut bytes = Vec::with_capacity(decoded_size);
		for (index, chunk) in chunks.iter().copied().enumerate() {
			bytes.extend(self.parse_hex_bytes(self.node(chunk, path)?.text(), &path.index(index))?);
		}
		Ok(bytes)
	}

	fn parse_calibration(
		&mut self,
		node: OgdlNodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<Option<CheckpointTensorImage>> {
		let fields = self.fields(node, path, &["temperature"], &[])?;
		Ok(Some(self.parse_tensor(
			fields.require("temperature", path)?,
			&path.field("temperature"),
		)?))
	}
}

struct ParsedDataset {
	vectors: Vec<CheckpointArtifactVector>,
	feature_spans: Vec<CompiledFeatureSpan>,
	feature_normalization_mask: Vec<u32>,
	feature_width: usize,
	task: DenseTask,
}

struct ParsedModel {
	output_adapter: Option<DenseOutputAdapter>,
	normalization: Vec<CheckpointTensorImage>,
	layers: Vec<CheckpointLayerImage>,
	blocks: Vec<CheckpointBlockImage>,
	temperature: Option<CheckpointTensorImage>,
}

fn validate_artifact(artifact: &CheckpointArtifact) -> CheckpointResult<()> {
	let root = CheckpointPath::root().field("recipe");
	validate_artifact_topology(artifact, &root.field("model"))?;
	validate_training_config(
		&artifact.config,
		&root.field("training"),
		artifact.format_version == FLAT_CHECKPOINT_FORMAT_VERSION,
	)?;
	validate_training_bounds(
		&artifact.config,
		artifact.bounds,
		&root.field("training").field("bounds"),
	)?;
	validate_vector_schema(artifact, &root.field("dataset"))?;
	validate_effective_model(artifact, &root.field("model"))?;
	Ok(())
}

fn validate_artifact_topology(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	match artifact.format_version {
		FLAT_CHECKPOINT_FORMAT_VERSION => {
			if artifact.layers.is_empty() || artifact.blocks.len() != artifact.layers.len() {
				return Err(validation_error(
					path.field("blocks"),
					"checkpoint v5 requires matching nonempty flat layer and block views",
				));
			}
			for (index, (block, layer)) in artifact.blocks.iter().zip(&artifact.layers).enumerate() {
				if block != &CheckpointBlockImage::Layer(layer.clone()) {
					return Err(validation_error(
						path.field("blocks").index(index),
						"checkpoint v5 block differs from its flat layer view",
					));
				}
			}
		}
		STRUCTURED_CHECKPOINT_FORMAT_VERSION => {
			if !artifact.layers.is_empty() {
				return Err(validation_error(
					path.field("blocks"),
					"checkpoint v6 must not mix a legacy flat layer view with structured blocks",
				));
			}
			if artifact.blocks.is_empty()
				|| !artifact
					.blocks
					.iter()
					.any(|block| matches!(block, CheckpointBlockImage::Residual(_)))
			{
				return Err(validation_error(
					path.field("blocks"),
					"checkpoint v6 requires a nonempty topology containing a residual block",
				));
			}
			if !artifact.config.layers.is_empty() {
				return Err(validation_error(
					path.field("blocks"),
					"checkpoint v6 cannot mix legacy configuration layers with structured blocks",
				));
			}
		}
		version => {
			return Err(invalid_value(
				CheckpointPath::root().field("recipe").field("version"),
				format!("unsupported checkpoint version {version}"),
			));
		}
	}
	Ok(())
}

fn validation_error(path: CheckpointPath, detail: impl Into<String>) -> CheckpointError {
	decode_error(CheckpointDecodeErrorKind::InconsistentValue, path, detail)
}

fn invalid_value(path: CheckpointPath, detail: impl Into<String>) -> CheckpointError {
	decode_error(CheckpointDecodeErrorKind::InvalidValue, path, detail)
}

fn validate_training_config(
	config: &DenseTrainingConfig,
	path: &CheckpointPath,
	require_legacy_layers: bool,
) -> CheckpointResult<()> {
	if require_legacy_layers && config.layers.is_empty() {
		return Err(validation_error(
			path.field("layers"),
			"effective model does not retain any declared layer",
		));
	}
	if config.warmup_epochs > config.epochs.get() {
		return Err(validation_error(
			path.field("warmup-epochs"),
			format!(
				"warmup epoch count {} exceeds total epoch count {}",
				config.warmup_epochs, config.epochs
			),
		));
	}
	if config.reduction_tree_lanes == 0
		|| config.reduction_tree_lanes > 1024
		|| !config.reduction_tree_lanes.is_power_of_two()
	{
		return Err(invalid_value(
			path.field("reduction-tree-lanes"),
			"reduction tree lanes must be a power of two in 1..=1024",
		));
	}
	for (name, value, allow_zero) in [
		("gradient-clip-norm", config.gradient_clip_norm, false),
		("normalization-epsilon", config.normalization_epsilon, false),
		("adamw.learning-rate", config.adamw.learning_rate, false),
		("adamw.epsilon", config.adamw.epsilon, false),
		("adamw.weight-decay", config.adamw.weight_decay, true),
	] {
		if !value.is_finite() || value < 0.0 || !allow_zero && value == 0.0 {
			let field_path = name
				.split('.')
				.fold(path.clone(), |path, field| path.field(field));
			return Err(invalid_value(
				field_path,
				format!(
					"value must be finite and {}",
					if allow_zero {
						"nonnegative"
					} else {
						"positive"
					}
				),
			));
		}
	}
	for (name, value) in [
		("beta-one", config.adamw.beta_one),
		("beta-two", config.adamw.beta_two),
	] {
		if !value.is_finite() || !(0.0..1.0).contains(&value) {
			return Err(invalid_value(
				path.field("adamw").field(name),
				"value must be finite and in [0, 1)",
			));
		}
	}
	Ok(())
}

fn validate_training_bounds(
	config: &DenseTrainingConfig,
	bounds: TrainingBounds,
	path: &CheckpointPath,
) -> CheckpointResult<()> {
	if bounds.train_rows == 0 {
		return Err(invalid_value(
			path.field("train-rows"),
			"training row count must be nonzero",
		));
	}
	let config_batch = u64::try_from(config.batch_size.get()).map_err(|error| {
		validation_error(
			path.field("batch-size"),
			format!("configuration batch size cannot be represented by u64: {error}"),
		)
	})?;
	expect_u64(
		bounds.batch_size,
		config_batch,
		path.field("batch-size"),
		"configuration batch size",
	)?;
	if bounds.epochs != config.epochs {
		return Err(validation_error(
			path.field("epochs"),
			format!(
				"bound epoch count {} differs from configuration epoch count {}",
				bounds.epochs, config.epochs
			),
		));
	}
	let batches_per_epoch = bounds
		.train_rows
		.checked_add(bounds.batch_size - 1)
		.and_then(|value| value.checked_div(bounds.batch_size))
		.ok_or_else(|| validation_error(path.field("batches-per-epoch"), "ceil division overflowed"))?;
	expect_u64(
		bounds.batches_per_epoch,
		batches_per_epoch,
		path.field("batches-per-epoch"),
		"ceil(train rows / batch size)",
	)?;
	let padded_rows = batches_per_epoch
		.checked_mul(bounds.batch_size)
		.ok_or_else(|| {
			validation_error(
				path.field("padded-rows-per-epoch"),
				"padded row count overflowed",
			)
		})?;
	expect_u64(
		bounds.padded_rows_per_epoch,
		padded_rows,
		path.field("padded-rows-per-epoch"),
		"batches per epoch times batch size",
	)?;
	if padded_rows > i32::MAX as u64 {
		return Err(invalid_value(
			path.field("padded-rows-per-epoch"),
			"padded rows exceed the int32 IndexMap domain",
		));
	}
	let training_iterations = batches_per_epoch
		.checked_mul(config.epochs.get())
		.ok_or_else(|| {
			validation_error(
				path.field("training-iterations"),
				"training iteration count overflowed",
			)
		})?;
	expect_u64(
		bounds.training_iterations.get(),
		training_iterations,
		path.field("training-iterations"),
		"batches per epoch times epochs",
	)?;
	if training_iterations > i32::MAX as u64 {
		return Err(invalid_value(
			path.field("training-iterations"),
			"training iterations exceed the int32 schedule domain",
		));
	}
	let iterations = training_iterations
		.checked_add(bounds.calibration_iterations)
		.ok_or_else(|| validation_error(path.field("iterations"), "total iteration count overflowed"))?;
	expect_u64(
		bounds.iterations.get(),
		iterations,
		path.field("iterations"),
		"training plus calibration iterations",
	)?;
	let warmup_iterations = batches_per_epoch
		.checked_mul(config.warmup_epochs)
		.ok_or_else(|| {
			validation_error(
				path.field("warmup-iterations"),
				"warmup iteration count overflowed",
			)
		})?;
	expect_u64(
		bounds.warmup_iterations,
		warmup_iterations,
		path.field("warmup-iterations"),
		"batches per epoch times warmup epochs",
	)
}

fn expect_u64(actual: u64, expected: u64, path: CheckpointPath, derivation: &str) -> CheckpointResult<()> {
	if actual != expected {
		return Err(validation_error(
			path,
			format!("value is {actual}, expected {expected} from {derivation}"),
		));
	}
	Ok(())
}

fn validate_vector_schema(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	let vectors_path = path.field("vectors");
	if artifact.vectors.is_empty() {
		return Err(validation_error(
			vectors_path,
			"dataset vector schema is empty",
		));
	}
	let mut source_indices = BTreeSet::new();
	let mut names = BTreeSet::new();
	let mut targets = Vec::new();
	let mut previous_source_index = None;
	for (index, vector) in artifact.vectors.iter().enumerate() {
		let vector_path = path.field("vectors").index(index);
		if !source_indices.insert(vector.source_index) {
			return Err(validation_error(
				vector_path.field("source-index"),
				format!(
					"source index {} appears more than once",
					vector.source_index
				),
			));
		}
		if previous_source_index.is_some_and(|previous| previous >= vector.source_index) {
			return Err(validation_error(
				vector_path.field("source-index"),
				"vector source indices must retain strictly increasing source order",
			));
		}
		previous_source_index = Some(vector.source_index);
		if vector.name.is_empty() {
			return Err(invalid_value(
				vector_path.field("name-bytes"),
				"vector name is empty",
			));
		}
		if !names.insert(vector.name.clone()) {
			return Err(validation_error(
				vector_path.field("name-bytes"),
				"vector name appears more than once",
			));
		}
		validate_vector_metadata(vector, &vector_path)?;
		if vector.role == VectorRole::Target {
			targets.push(vector);
		}
	}
	if targets.len() != 1 {
		return Err(validation_error(
			path.field("target"),
			format!(
				"dense dataset requires exactly one target vector, found {}",
				targets.len()
			),
		));
	}
	let target = targets[0];
	if target.source_index != artifact.task.target_vector() {
		return Err(validation_error(
			path.field("target").field("source-index"),
			format!(
				"declared target source {} differs from target vector source {}",
				artifact.task.target_vector(),
				target.source_index
			),
		));
	}
	validate_task(
		artifact.config.loss,
		artifact.task,
		target,
		&path.field("target"),
	)?;
	validate_feature_spans(artifact, path)
}

fn validate_vector_metadata(vector: &CheckpointArtifactVector, path: &CheckpointPath) -> CheckpointResult<()> {
	let valid = match (&vector.semantic_type, &vector.encoding, &vector.metadata) {
		(SemanticType::Numeric, VectorEncoding::F32 | VectorEncoding::I32, CheckpointArtifactMetadata::None) => {
			true
		}
		(
			SemanticType::Temporal,
			VectorEncoding::RelativeSecondsI32,
			CheckpointArtifactMetadata::Temporal { nanoseconds, .. },
		) => {
			if *nanoseconds >= 1_000_000_000 {
				return Err(invalid_value(
					path.field("metadata").field("nanoseconds"),
					"temporal nanoseconds must be below one billion",
				));
			}
			true
		}
		(
			SemanticType::Categorical,
			VectorEncoding::DictionaryI32,
			CheckpointArtifactMetadata::Categorical { dictionary },
		) => {
			validate_byte_dictionary(dictionary, &path.field("metadata"), true)?;
			true
		}
		(
			SemanticType::Ordinal,
			VectorEncoding::OrdinalI32,
			CheckpointArtifactMetadata::Ordinal { ordered_labels },
		) => {
			validate_byte_dictionary(ordered_labels, &path.field("metadata"), false)?;
			true
		}
		(SemanticType::Text, VectorEncoding::Utf8, CheckpointArtifactMetadata::None) => true,
		(SemanticType::Image, VectorEncoding::Bytes, CheckpointArtifactMetadata::Image { encoded_variants }) => {
			validate_image_metadata(encoded_variants, &path.field("metadata"))?;
			true
		}
		(SemanticType::Binary, VectorEncoding::Bytes, CheckpointArtifactMetadata::None) => true,
		_ => false,
	};
	if !valid {
		return Err(validation_error(
			path.field("metadata"),
			format!(
				"metadata is incompatible with {:?}/{:?}",
				vector.semantic_type, vector.encoding
			),
		));
	}
	Ok(())
}

fn validate_byte_dictionary(values: &[Vec<u8>], path: &CheckpointPath, require_sorted: bool) -> CheckpointResult<()> {
	if values.is_empty() {
		return Err(invalid_value(path.clone(), "dictionary is empty"));
	}
	let mut distinct = BTreeSet::new();
	for (index, value) in values.iter().enumerate() {
		if value.is_empty() {
			return Err(invalid_value(
				path.index(index),
				"dictionary label is empty",
			));
		}
		if !distinct.insert(value) {
			return Err(validation_error(
				path.index(index),
				"dictionary label appears more than once",
			));
		}
		if require_sorted && index > 0 && values[index - 1] >= *value {
			return Err(validation_error(
				path.index(index),
				"categorical dictionary is not in canonical ascending byte order",
			));
		}
	}
	Ok(())
}

fn validate_image_metadata(values: &[CheckpointImageMetadata], path: &CheckpointPath) -> CheckpointResult<()> {
	if values.is_empty() {
		return Err(invalid_value(path.clone(), "image variant set is empty"));
	}
	let mut distinct = BTreeSet::new();
	for (index, value) in values.iter().copied().enumerate() {
		let value_path = path.index(index);
		if value.width == 0 || value.height == 0 {
			return Err(invalid_value(
				value_path,
				"image dimensions must be nonzero",
			));
		}
		if value.channels == Some(0) {
			return Err(invalid_value(
				value_path.field("channels"),
				"image channel count must be nonzero",
			));
		}
		if value.sample_bits == Some(0) {
			return Err(invalid_value(
				value_path.field("sample-bits"),
				"image sample width must be nonzero",
			));
		}
		if !image_metadata_could_be_ingested(value) {
			return Err(validation_error(
				value_path.clone(),
				"image header facts cannot be produced by the declared encoded format",
			));
		}
		if !distinct.insert(value) {
			return Err(validation_error(
				value_path,
				"image variant appears more than once",
			));
		}
		if index > 0 && values[index - 1] >= value {
			return Err(validation_error(
				path.index(index),
				"image variants are not in canonical ascending header order",
			));
		}
	}
	Ok(())
}

fn image_metadata_could_be_ingested(value: CheckpointImageMetadata) -> bool {
	match value.format {
		EncodedImageFormat::Png => {
			value.width <= i32::MAX as u32
				&& value.height <= i32::MAX as u32
				&& matches!(
					(value.channels, value.color_model, value.sample_bits),
					(
						Some(1),
						Some(ImageColorModel::Grayscale),
						Some(1 | 2 | 4 | 8 | 16)
					) | (Some(3), Some(ImageColorModel::Rgb), Some(8 | 16))
						| (
							Some(1),
							Some(ImageColorModel::IndexedRgb),
							Some(1 | 2 | 4 | 8)
						) | (Some(2), Some(ImageColorModel::GrayscaleAlpha), Some(8 | 16))
						| (Some(4), Some(ImageColorModel::Rgba), Some(8 | 16))
				)
		}
		EncodedImageFormat::Jpeg => {
			value.width <= u32::from(u16::MAX)
				&& value.height <= u32::from(u16::MAX)
				&& value.sample_bits.is_some_and(|bits| bits > 0)
				&& match (value.channels, value.color_model) {
					(Some(1), Some(ImageColorModel::Grayscale)) => true,
					(Some(3), None | Some(ImageColorModel::Rgb | ImageColorModel::YCbCr)) => true,
					(Some(4), None | Some(ImageColorModel::Cmyk | ImageColorModel::Ycck)) => true,
					(Some(channels), None) => channels > 0 && !matches!(channels, 1 | 3 | 4),
					_ => false,
				}
		}
		EncodedImageFormat::Gif87a | EncodedImageFormat::Gif89a => {
			value.width <= u32::from(u16::MAX)
				&& value.height <= u32::from(u16::MAX)
				&& value.channels == Some(1)
				&& value.color_model == Some(ImageColorModel::IndexedRgb)
				&& value.sample_bits.is_none_or(|bits| (1..=8).contains(&bits))
		}
		EncodedImageFormat::Bmp => matches!(
			(value.channels, value.color_model, value.sample_bits),
			(None, None, None)
				| (
					Some(1),
					Some(ImageColorModel::IndexedRgb),
					Some(1 | 2 | 4 | 8)
				) | (Some(3), Some(ImageColorModel::Bgr), Some(8 | 16))
		),
		EncodedImageFormat::WebP => match (value.channels, value.color_model, value.sample_bits) {
			(Some(3), Some(ImageColorModel::YCbCr), Some(8)) => value.width <= 0x3fff && value.height <= 0x3fff,
			(Some(3), Some(ImageColorModel::Rgb), Some(8)) | (Some(4), Some(ImageColorModel::Rgba), Some(8)) => {
				value.width <= 1 << 24 && value.height <= 1 << 24
			}
			_ => false,
		},
	}
}

fn validate_task(
	loss: DenseLoss,
	task: DenseTask,
	target: &CheckpointArtifactVector,
	path: &CheckpointPath,
) -> CheckpointResult<()> {
	match task {
		DenseTask::BinaryClassification { positive_code, .. } => {
			if loss != DenseLoss::BinaryCrossEntropy {
				return Err(validation_error(
					path.field("task"),
					"binary task requires binary cross entropy",
				));
			}
			if positive_code != 1 {
				return Err(validation_error(
					path.field("positive-code"),
					"checkpoint v5 binary targets reserve code 1 as positive",
				));
			}
			let compatible = matches!(
				(target.semantic_type, target.encoding, &target.metadata),
				(
					SemanticType::Numeric,
					VectorEncoding::I32 | VectorEncoding::F32,
					CheckpointArtifactMetadata::None
				)
			) || matches!(
				(target.semantic_type, target.encoding, &target.metadata),
				(
					SemanticType::Categorical,
					VectorEncoding::DictionaryI32,
					CheckpointArtifactMetadata::Categorical { dictionary }
				) if dictionary.len() == 2
			);
			if !compatible {
				return Err(validation_error(
					path.clone(),
					"target schema is incompatible with checkpoint-v5 BCE",
				));
			}
		}
		DenseTask::MulticlassClassification {
			class_count,
			reserved_code,
			..
		} => {
			if loss != DenseLoss::CrossEntropy {
				return Err(validation_error(
					path.field("task"),
					"multiclass task requires cross entropy",
				));
			}
			let CheckpointArtifactMetadata::Categorical { dictionary } = &target.metadata else {
				return Err(validation_error(
					path.clone(),
					"multiclass target does not have categorical metadata",
				));
			};
			if target.semantic_type != SemanticType::Categorical
				|| target.encoding != VectorEncoding::DictionaryI32
			{
				return Err(validation_error(
					path.clone(),
					"multiclass target is not categorical dictionary-int32",
				));
			}
			let expected_count = dictionary
				.len()
				.checked_add(1)
				.ok_or_else(|| validation_error(path.field("class-count"), "class count overflowed usize"))?;
			if class_count != expected_count {
				return Err(validation_error(
					path.field("class-count"),
					format!("class count is {class_count}, expected {expected_count}"),
				));
			}
			let expected_reserved = i32::try_from(dictionary.len()).map_err(|error| {
				validation_error(
					path.field("reserved-unseen-code"),
					format!("dictionary length cannot be represented as i32: {error}"),
				)
			})?;
			if reserved_code != expected_reserved {
				return Err(validation_error(
					path.field("reserved-unseen-code"),
					format!("reserved code is {reserved_code}, expected {expected_reserved}"),
				));
			}
		}
		DenseTask::ScalarRegression { .. } => {
			if !matches!(
				loss,
				DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber
			) {
				return Err(validation_error(
					path.field("task"),
					"scalar regression requires MSE, MAE, or Huber",
				));
			}
			if target.semantic_type != SemanticType::Numeric
				|| !matches!(target.encoding, VectorEncoding::I32 | VectorEncoding::F32)
			{
				return Err(validation_error(
					path.clone(),
					"scalar regression target is not numeric",
				));
			}
		}
	}
	Ok(())
}

fn validate_feature_spans(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	let features = artifact
		.vectors
		.iter()
		.filter(|vector| vector.role == VectorRole::Feature)
		.collect::<Vec<_>>();
	if features.is_empty() {
		return Err(validation_error(
			path.field("feature-spans"),
			"dense dataset has no feature vectors",
		));
	}
	if artifact.feature_spans.len() != features.len() {
		return Err(validation_error(
			path.field("feature-spans"),
			format!(
				"{} spans describe {} feature vectors",
				artifact.feature_spans.len(),
				features.len()
			),
		));
	}
	let mut start = 0usize;
	for (index, (span, vector)) in artifact.feature_spans.iter().zip(features).enumerate() {
		let span_path = path.field("feature-spans").index(index);
		if span.source_vector() != vector.source_index {
			return Err(validation_error(
				span_path.field("source-index"),
				format!(
					"span source is {}, expected {}",
					span.source_vector(),
					vector.source_index
				),
			));
		}
		if span.start() != start {
			return Err(validation_error(
				span_path.field("start"),
				format!(
					"span starts at {}, expected contiguous start {start}",
					span.start()
				),
			));
		}
		match (
			span.lowering(),
			vector.semantic_type,
			vector.encoding,
			&vector.metadata,
		) {
			(
				DenseFeatureLowering::NumericScalar,
				SemanticType::Numeric,
				VectorEncoding::I32 | VectorEncoding::F32,
				CheckpointArtifactMetadata::None,
			) if span.width() == 1 => {}
			(
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width,
					reserved_index,
				},
				SemanticType::Categorical,
				VectorEncoding::DictionaryI32,
				CheckpointArtifactMetadata::Categorical { dictionary },
			) if dictionary_width == dictionary.len()
				&& reserved_index == dictionary.len()
				&& span.width() == dictionary.len().saturating_add(1) => {}
			_ => {
				return Err(validation_error(
					span_path.field("lowering"),
					"feature lowering, width, and vector schema are inconsistent",
				));
			}
		}
		start = start
			.checked_add(span.width())
			.ok_or_else(|| validation_error(span_path.field("width"), "feature width overflowed usize"))?;
	}
	if start != artifact.feature_width || artifact.feature_width == 0 {
		return Err(validation_error(
			path.field("feature-width"),
			format!(
				"feature width is {}, contiguous spans cover {start}",
				artifact.feature_width
			),
		));
	}
	let expected_mask = feature_normalization_mask(&artifact.feature_spans, artifact.feature_width);
	if artifact.feature_normalization_mask.len() != expected_mask.len() {
		return Err(validation_error(
			path.field("feature-normalization-mask"),
			format!(
				"normalization mask has {} values, expected {}",
				artifact.feature_normalization_mask.len(),
				expected_mask.len()
			),
		));
	}
	for (index, (actual, expected)) in artifact
		.feature_normalization_mask
		.iter()
		.zip(expected_mask)
		.enumerate()
	{
		if *actual != expected {
			return Err(validation_error(
				path.field("feature-normalization-mask").index(index),
				format!("mask bits are 0x{actual:08x}, expected 0x{expected:08x}"),
			));
		}
	}
	Ok(())
}

fn validate_effective_model(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	match artifact.format_version {
		FLAT_CHECKPOINT_FORMAT_VERSION => validate_effective_flat_model(artifact, path),
		STRUCTURED_CHECKPOINT_FORMAT_VERSION => validate_effective_structured_model(artifact, path),
		version => Err(invalid_value(
			CheckpointPath::root().field("recipe").field("version"),
			format!("unsupported checkpoint version {version}"),
		)),
	}
}

fn validate_effective_flat_model(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	if artifact.layers.is_empty() {
		return Err(validation_error(
			path.field("blocks"),
			"effective model has no blocks",
		));
	}
	let target_width = artifact.task.output_width();
	match artifact.output_adapter {
		Some(adapter) => {
			if artifact.layers.len() < 2 || artifact.config.layers.len() + 1 != artifact.layers.len() {
				return Err(validation_error(
					path.field("output-adapter"),
					"adapter requires at least one declared block and one final synthetic block",
				));
			}
			let source = artifact
				.config
				.layers
				.last()
				.expect("validated nonempty layers");
			let final_layer = artifact
				.layers
				.last()
				.expect("validated nonempty effective layers");
			if adapter.source_width() != source.width()
				|| usize::try_from(adapter.target_width().get()).ok() != Some(target_width)
			{
				return Err(validation_error(
					path.field("output-adapter"),
					"adapter widths do not join the declared output block to the task output",
				));
			}
			if final_layer.declaration.kind() != DenseBlockKind::Layer
				|| final_layer.declaration.width() != adapter.target_width()
				|| !final_layer.declaration.operations().is_empty()
			{
				return Err(validation_error(
					path.field("blocks").index(artifact.layers.len() - 1),
					"final adapter block must be an operation-free linear layer at the target width",
				));
			}
			let classification_requires_logits = matches!(
				artifact.task,
				DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. }
			);
			let source_is_logits = source
				.operations()
				.iter()
				.all(|operation| *operation == DenseOperation::Activation(DenseActivation::Linear));
			if source.width() == adapter.target_width() && (!classification_requires_logits || source_is_logits) {
				return Err(validation_error(
					path.field("output-adapter"),
					"serialized adapter is redundant under the checkpoint-v5 effective-layer rule",
				));
			}
		}
		None => {
			if artifact.config.layers.len() != artifact.layers.len() {
				return Err(validation_error(
					path.field("blocks"),
					"declared and effective block counts differ",
				));
			}
			let output = artifact
				.config
				.layers
				.last()
				.expect("validated nonempty layers");
			if usize::try_from(output.width().get()).ok() != Some(target_width) {
				return Err(validation_error(
					path.field("blocks").index(artifact.layers.len() - 1),
					"final block width differs from the task output without an adapter",
				));
			}
			if matches!(
				artifact.task,
				DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. }
			) && !output
				.operations()
				.iter()
				.all(|operation| *operation == DenseOperation::Activation(DenseActivation::Linear))
			{
				return Err(validation_error(
					path.field("blocks")
						.index(artifact.layers.len() - 1)
						.field("operations"),
					"classification output is not logits and has no linear output adapter",
				));
			}
		}
	}
	validate_normalization_state(artifact, &path.field("normalization"))?;
	let mut input_width = u64::try_from(artifact.feature_width).map_err(|error| {
		validation_error(
			path.field("input-width"),
			format!("feature width cannot be represented by u64: {error}"),
		)
	})?;
	for (index, layer) in artifact.layers.iter().enumerate() {
		let layer_path = path.field("blocks").index(index);
		let output_width = layer.declaration.width().get();
		validate_parameter(
			&layer.weight,
			&layer_path.field("weight"),
			&[input_width, output_width],
		)?;
		validate_parameter(&layer.bias, &layer_path.field("bias"), &[output_width])?;
		input_width = output_width;
	}
	validate_temperature(artifact, &path.field("calibration"))
}

fn validate_effective_structured_model(
	artifact: &CheckpointArtifact,
	path: &CheckpointPath,
) -> CheckpointResult<()> {
	if artifact.blocks.is_empty() {
		return Err(validation_error(
			path.field("blocks"),
			"effective model has no blocks",
		));
	}
	let target_width = artifact.task.output_width();
	match artifact.output_adapter {
		Some(adapter) => {
			if artifact.blocks.len() < 2 {
				return Err(validation_error(
					path.field("output-adapter"),
					"adapter requires at least one declared block and one final synthetic block",
				));
			}
			let source = &artifact.blocks[artifact.blocks.len() - 2];
			let final_block = artifact.blocks.last().expect("validated nonempty blocks");
			if adapter.source_width() != source.output_width()
				|| usize::try_from(adapter.target_width().get()).ok() != Some(target_width)
			{
				return Err(validation_error(
					path.field("output-adapter"),
					"adapter widths do not join the declared output block to the task output",
				));
			}
			let CheckpointBlockImage::Layer(final_layer) = final_block else {
				return Err(validation_error(
					path.field("blocks").index(artifact.blocks.len() - 1),
					"final adapter block must be an ordinary layer",
				));
			};
			if final_layer.declaration.kind() != DenseBlockKind::Layer
				|| final_layer.declaration.width() != adapter.target_width()
				|| !final_layer.declaration.operations().is_empty()
			{
				return Err(validation_error(
					path.field("blocks").index(artifact.blocks.len() - 1),
					"final adapter block must be an operation-free linear layer at the target width",
				));
			}
			let classification_requires_logits = matches!(
				artifact.task,
				DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. }
			);
			let source_is_logits = source
				.output_operations()
				.iter()
				.all(|operation| *operation == DenseOperation::Activation(DenseActivation::Linear));
			if source.output_width() == adapter.target_width()
				&& (!classification_requires_logits || source_is_logits)
			{
				return Err(validation_error(
					path.field("output-adapter"),
					"serialized adapter is redundant under the checkpoint-v6 effective-block rule",
				));
			}
		}
		None => {
			let output = artifact.blocks.last().expect("validated nonempty blocks");
			if usize::try_from(output.output_width().get()).ok() != Some(target_width) {
				return Err(validation_error(
					path.field("blocks").index(artifact.blocks.len() - 1),
					"final block width differs from the task output without an adapter",
				));
			}
			if matches!(
				artifact.task,
				DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. }
			) && !output
				.output_operations()
				.iter()
				.all(|operation| *operation == DenseOperation::Activation(DenseActivation::Linear))
			{
				return Err(validation_error(
					path.field("blocks")
						.index(artifact.blocks.len() - 1)
						.field("operations"),
					"classification output is not logits and has no linear output adapter",
				));
			}
		}
	}
	validate_normalization_state(artifact, &path.field("normalization"))?;
	let mut input_width = u64::try_from(artifact.feature_width).map_err(|error| {
		validation_error(
			path.field("input-width"),
			format!("feature width cannot be represented by u64: {error}"),
		)
	})?;
	for (index, block) in artifact.blocks.iter().enumerate() {
		let block_path = path.field("blocks").index(index);
		match block {
			CheckpointBlockImage::Layer(layer) => {
				let output_width = layer.declaration.width().get();
				validate_parameter(
					&layer.weight,
					&block_path.field("weight"),
					&[input_width, output_width],
				)?;
				validate_parameter(&layer.bias, &block_path.field("bias"), &[output_width])?;
				input_width = output_width;
			}
			CheckpointBlockImage::Residual(residual) => {
				let residual_input_width = input_width;
				let mut branch_width = input_width;
				let mut retained_layer = false;
				for (step_index, step) in residual.branch.iter().enumerate() {
					if let CheckpointResidualBranchImage::Layer(layer) = step {
						retained_layer = true;
						let output_width = layer.declaration.width().get();
						let step_path = block_path.field("branch").index(step_index);
						validate_parameter(
							&layer.weight,
							&step_path.field("weight"),
							&[branch_width, output_width],
						)?;
						validate_parameter(&layer.bias, &step_path.field("bias"), &[output_width])?;
						branch_width = output_width;
					}
				}
				if !retained_layer || branch_width != residual.output_width.get() {
					return Err(validation_error(
						block_path.field("output-width"),
						"residual output width must equal its last branch layer width",
					));
				}
				match (&residual.skip, residual_input_width == branch_width) {
					(CheckpointResidualSkipImage::Identity, true) => {}
					(CheckpointResidualSkipImage::Projection(projection), false) => {
						validate_parameter(
							projection,
							&block_path.field("skip").field("weight"),
							&[residual_input_width, branch_width],
						)?;
					}
					(CheckpointResidualSkipImage::Identity, false) => {
						return Err(validation_error(
							block_path.field("skip"),
							"residual width mismatch requires a weight-only linear projection",
						));
					}
					(CheckpointResidualSkipImage::Projection(_), true) => {
						return Err(validation_error(
							block_path.field("skip"),
							"equal residual widths require an identity skip",
						));
					}
				}
				input_width = branch_width;
			}
		}
	}
	validate_temperature(artifact, &path.field("calibration"))
}

fn validate_normalization_state(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	let shape = [u64::try_from(artifact.feature_width).map_err(|error| {
		validation_error(
			path.clone(),
			format!("feature width cannot be represented by u64: {error}"),
		)
	})?];
	match artifact.config.data_normalization {
		DenseDataNormalization::ZScore => {
			if artifact.normalization.len() != 2 {
				return Err(validation_error(
					path.clone(),
					"z-score state requires mean and variance tensors",
				));
			}
			validate_f32_tensor(
				&artifact.normalization[0],
				&path.field("mean"),
				&shape,
				false,
			)?;
			validate_f32_tensor(
				&artifact.normalization[1],
				&path.field("variance"),
				&shape,
				true,
			)
		}
		DenseDataNormalization::MinMax => {
			if artifact.normalization.len() != 2 {
				return Err(validation_error(
					path.clone(),
					"min-max state requires minimum and maximum tensors",
				));
			}
			validate_f32_tensor(
				&artifact.normalization[0],
				&path.field("minimum"),
				&shape,
				false,
			)?;
			validate_f32_tensor(
				&artifact.normalization[1],
				&path.field("maximum"),
				&shape,
				false,
			)?;
			for (index, (minimum, maximum)) in f32_values(&artifact.normalization[0])
				.zip(f32_values(&artifact.normalization[1]))
				.enumerate()
			{
				if minimum > maximum {
					return Err(validation_error(
						path.field("maximum").field("payload").index(index),
						format!("maximum {maximum} is below minimum {minimum}"),
					));
				}
			}
			Ok(())
		}
		DenseDataNormalization::L2Norm => {
			if !artifact.normalization.is_empty() {
				return Err(validation_error(
					path.clone(),
					"l2 normalization must not retain fitted tensors",
				));
			}
			Ok(())
		}
	}
}

fn validate_parameter(
	parameter: &CheckpointParameterImage,
	path: &CheckpointPath,
	shape: &[u64],
) -> CheckpointResult<()> {
	validate_f32_tensor(&parameter.parameter, &path.field("parameter"), shape, false)?;
	validate_f32_tensor(
		&parameter.first_moment,
		&path.field("first-moment"),
		shape,
		false,
	)?;
	validate_f32_tensor(
		&parameter.second_moment,
		&path.field("second-moment"),
		shape,
		true,
	)
}

fn validate_f32_tensor(
	tensor: &CheckpointTensorImage,
	path: &CheckpointPath,
	expected_shape: &[u64],
	require_nonnegative: bool,
) -> CheckpointResult<()> {
	if tensor.dtype != DType::F32 {
		return Err(validation_error(
			path.field("dtype"),
			format!("tensor dtype is {:?}, expected F32", tensor.dtype),
		));
	}
	if tensor.shape != expected_shape {
		return Err(validation_error(
			path.field("shape"),
			format!(
				"tensor shape is {:?}, expected {expected_shape:?}",
				tensor.shape
			),
		));
	}
	let elements = expected_shape
		.iter()
		.try_fold(1u64, |elements, extent| {
			if *extent == 0 {
				None
			} else {
				elements.checked_mul(*extent)
			}
		})
		.ok_or_else(|| {
			validation_error(
				path.field("shape"),
				"tensor extent product is zero or overflowed",
			)
		})?;
	let bytes = elements
		.checked_mul(4)
		.and_then(|bytes| usize::try_from(bytes).ok())
		.ok_or_else(|| validation_error(path.field("payload"), "tensor byte size overflowed usize"))?;
	if tensor.bytes.len() != bytes {
		return Err(validation_error(
			path.field("payload"),
			format!(
				"tensor payload has {} bytes, expected {bytes}",
				tensor.bytes.len()
			),
		));
	}
	for (index, value) in f32_values(tensor).enumerate() {
		if !value.is_finite() {
			return Err(invalid_value(
				path.field("payload").index(index),
				format!("tensor state contains nonfinite value {value}"),
			));
		}
		if require_nonnegative && value < 0.0 {
			return Err(invalid_value(
				path.field("payload").index(index),
				format!("tensor state contains negative value {value}"),
			));
		}
	}
	Ok(())
}

fn f32_values(tensor: &CheckpointTensorImage) -> impl Iterator<Item = f32> + '_ {
	tensor.bytes
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("exact four-byte chunk")))
}

fn validate_temperature(artifact: &CheckpointArtifact, path: &CheckpointPath) -> CheckpointResult<()> {
	match (
		&artifact.temperature,
		artifact.bounds.calibration_iterations,
	) {
		(None, 0) => Ok(()),
		(Some(_), 0) => Err(validation_error(
			path.clone(),
			"temperature tensor exists while calibration iteration count is zero",
		)),
		(None, _) => Err(validation_error(
			path.clone(),
			"calibration iterations exist without a final temperature tensor",
		)),
		(Some(temperature), _) => {
			if artifact.config.loss != DenseLoss::BinaryCrossEntropy
				|| !matches!(artifact.task, DenseTask::BinaryClassification { .. })
			{
				return Err(validation_error(
					path.clone(),
					"temperature scaling is retained only for binary BCE training",
				));
			}
			validate_f32_tensor(temperature, &path.field("temperature"), &[1], true)?;
			let temperature_value = f32_values(temperature)
				.next()
				.expect("validated scalar tensor");
			if temperature_value == 0.0 {
				return Err(invalid_value(
					path.field("temperature").field("payload").index(0),
					"temperature must be positive",
				));
			}
			Ok(())
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckpointTensor {
	value: ValueId,
	dtype: DType,
	shape: Vec<u64>,
	bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckpointParameter {
	parameter: CheckpointTensor,
	first_moment: CheckpointTensor,
	second_moment: CheckpointTensor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckpointLayer {
	declaration: DenseLayer,
	weight: CheckpointParameter,
	bias: CheckpointParameter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CheckpointResidualBranch {
	Layer(CheckpointLayer),
	Operation(DenseOperation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CheckpointResidualSkip {
	Identity,
	Projection(CheckpointParameter),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckpointResidual {
	branch: Vec<CheckpointResidualBranch>,
	output_width: NonZeroU64,
	skip: CheckpointResidualSkip,
	operations: Vec<DenseOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CheckpointBlock {
	Layer(CheckpointLayer),
	Residual(CheckpointResidual),
}

/// Immutable semantic description used to interpret a completed execution's
/// egress images. It contains no dataset rows, executable artifacts, devices,
/// queues, contexts, or native handles.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointManifest {
	format_version: u32,
	vectors: Vec<CheckpointVectorSchema>,
	feature_spans: Vec<CompiledFeatureSpan>,
	feature_width: usize,
	task: DenseTask,
	output_adapter: Option<DenseOutputAdapter>,
	config: DenseTrainingConfig,
	bounds: TrainingBounds,
	normalization: Vec<CheckpointTensor>,
	layers: Vec<CheckpointLayer>,
	blocks: Vec<CheckpointBlock>,
	temperature: Option<CheckpointTensor>,
}

impl CheckpointManifest {
	/// Capture only the declaration and no-row schema needed to interpret the
	/// final model state.
	pub fn from_compiled(training: &CompiledTraining) -> CheckpointResult<Self> {
		let structured = training
			.blocks()
			.iter()
			.any(|block| matches!(block, DenseBlock::Residual(_)));
		if !structured && training.layers().len() != training.outputs().layers.len() {
			return Err(CheckpointError::manifest(format!(
				"{} effective layers produced {} layer states",
				training.layers().len(),
				training.outputs().layers.len()
			)));
		}
		let feature_width = training.dataset_schema().input_width();
		let feature_spans = training.dataset_schema().features().to_vec();
		let vectors = training
			.dataset_schema()
			.vectors()
			.iter()
			.map(|vector| CheckpointVectorSchema {
				source_index: vector.source_index(),
				name: vector.name().to_vec(),
				role: vector.role(),
				semantic_type: vector.semantic_type(),
				encoding: vector.encoding(),
				metadata: vector.metadata().clone(),
			})
			.collect();
		let output = training.outputs();
		let normalization = match output.normalization {
			DataNormalizationState::ZScore(state) => vec![
				checkpoint_tensor(training, state.mean)?,
				checkpoint_tensor(training, state.variance)?,
			],
			DataNormalizationState::MinMax(state) => vec![
				checkpoint_tensor(training, state.minimum)?,
				checkpoint_tensor(training, state.maximum)?,
			],
			DataNormalizationState::L2Norm => Vec::new(),
		};
		let (format_version, config, layers, blocks) = if structured {
			if training.blocks().len() != output.blocks.len() {
				return Err(CheckpointError::manifest(format!(
					"{} effective blocks produced {} block states",
					training.blocks().len(),
					output.blocks.len()
				)));
			}
			let mut config = training.config().clone();
			config.layers.clear();
			(
				STRUCTURED_CHECKPOINT_FORMAT_VERSION,
				config,
				Vec::new(),
				checkpoint_blocks(training, training.blocks(), &output.blocks)?,
			)
		} else {
			let layers = training
				.layers()
				.iter()
				.cloned()
				.zip(&output.layers)
				.map(|(declaration, state)| checkpoint_layer(training, declaration, *state))
				.collect::<CheckpointResult<Vec<_>>>()?;
			(
				FLAT_CHECKPOINT_FORMAT_VERSION,
				training.config().clone(),
				layers,
				Vec::new(),
			)
		};
		let temperature = output
			.validation
			.as_ref()
			.and_then(|validation| validation.temperature_scaling)
			.map(|state| checkpoint_tensor(training, state.updated_temperature))
			.transpose()?;
		let manifest = Self {
			format_version,
			vectors,
			feature_spans,
			feature_width,
			task: training.dataset_schema().task(),
			output_adapter: training.output_adapter(),
			config,
			bounds: training.bounds(),
			normalization,
			layers,
			blocks,
			temperature,
		};
		manifest.validate_external_boundary(training)?;
		Ok(manifest)
	}

	#[must_use]
	pub const fn format_version(&self) -> u32 {
		self.format_version
	}

	#[must_use]
	pub fn vectors(&self) -> &[CheckpointVectorSchema] {
		&self.vectors
	}

	#[must_use]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] {
		&self.feature_spans
	}

	#[must_use]
	pub const fn feature_width(&self) -> usize {
		self.feature_width
	}

	#[must_use]
	pub const fn task(&self) -> DenseTask {
		self.task
	}

	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> {
		self.output_adapter
	}

	fn tensors(&self) -> impl Iterator<Item = &CheckpointTensor> {
		let mut tensors = self.normalization.iter().collect::<Vec<_>>();
		for layer in &self.layers {
			push_checkpoint_parameter_tensors(&mut tensors, &layer.weight);
			push_checkpoint_parameter_tensors(&mut tensors, &layer.bias);
		}
		for block in &self.blocks {
			match block {
				CheckpointBlock::Layer(layer) => {
					push_checkpoint_parameter_tensors(&mut tensors, &layer.weight);
					push_checkpoint_parameter_tensors(&mut tensors, &layer.bias);
				}
				CheckpointBlock::Residual(residual) => {
					for step in &residual.branch {
						if let CheckpointResidualBranch::Layer(layer) = step {
							push_checkpoint_parameter_tensors(&mut tensors, &layer.weight);
							push_checkpoint_parameter_tensors(&mut tensors, &layer.bias);
						}
					}
					if let CheckpointResidualSkip::Projection(projection) = &residual.skip {
						push_checkpoint_parameter_tensors(&mut tensors, projection);
					}
				}
			}
		}
		tensors.extend(self.temperature.iter());
		tensors.into_iter()
	}

	fn validate_external_boundary(&self, training: &CompiledTraining) -> CheckpointResult<()> {
		let expected = self
			.tensors()
			.map(|tensor| tensor.value)
			.collect::<BTreeSet<_>>();
		if expected.len() != self.tensors().count() {
			return Err(CheckpointError::manifest(
				"one logical value is assigned to multiple checkpoint roles",
			));
		}
		let actual = training
			.graph()
			.tensors
			.iter()
			.filter(|tensor| tensor.external_output)
			.map(|tensor| tensor.id)
			.collect::<BTreeSet<_>>();
		if expected != actual {
			let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
			let unexpected = actual.difference(&expected).copied().collect::<Vec<_>>();
			return Err(CheckpointError::manifest(format!(
				"checkpoint/output boundary differs (missing {missing:?}, unexpected {unexpected:?})"
			)));
		}
		Ok(())
	}
}

fn checkpoint_layer(
	training: &CompiledTraining,
	declaration: DenseLayer,
	state: DenseLayerState,
) -> CheckpointResult<CheckpointLayer> {
	Ok(CheckpointLayer {
		declaration,
		weight: checkpoint_parameter(training, state.weight)?,
		bias: checkpoint_parameter(training, state.bias)?,
	})
}

fn checkpoint_blocks(
	training: &CompiledTraining,
	declarations: &[DenseBlock],
	states: &[DenseBlockState],
) -> CheckpointResult<Vec<CheckpointBlock>> {
	if declarations.len() != states.len() {
		return Err(CheckpointError::manifest(format!(
			"{} structured declarations produced {} block states",
			declarations.len(),
			states.len()
		)));
	}
	declarations
		.iter()
		.zip(states)
		.enumerate()
		.map(|(index, (declaration, state))| match (declaration, state) {
			(DenseBlock::Layer(layer), DenseBlockState::Layer(state)) => {
				checkpoint_layer(training, layer.clone(), *state).map(CheckpointBlock::Layer)
			}
			(DenseBlock::Residual(residual), DenseBlockState::Residual(state)) => {
				checkpoint_residual(training, residual, state).map(CheckpointBlock::Residual)
			}
			_ => Err(CheckpointError::manifest(format!(
				"structured block {index} declaration and optimizer state have different kinds"
			))),
		})
		.collect()
}

fn checkpoint_residual(
	training: &CompiledTraining,
	declaration: &DenseResidual,
	state: &DenseResidualState,
) -> CheckpointResult<CheckpointResidual> {
	let output_width = declaration.output_width().ok_or_else(|| {
		CheckpointError::manifest("residual checkpoint declaration has no width-producing branch layer")
	})?;
	let declared_layer_count = declaration
		.branch()
		.iter()
		.filter(|step| matches!(step, DenseResidualOperation::Layer(_)))
		.count();
	if declared_layer_count != state.branch.len() {
		return Err(CheckpointError::manifest(format!(
			"residual branch declares {declared_layer_count} layers but produced {} layer states",
			state.branch.len()
		)));
	}
	let mut state_index = 0usize;
	let mut branch = Vec::with_capacity(declaration.branch().len());
	for step in declaration.branch() {
		match step {
			DenseResidualOperation::Layer(layer) => {
				let layer_state = state.branch.get(state_index).copied().ok_or_else(|| {
					CheckpointError::manifest("residual branch layer state disappeared during traversal")
				})?;
				state_index += 1;
				branch.push(CheckpointResidualBranch::Layer(checkpoint_layer(
					training,
					layer.clone(),
					layer_state,
				)?));
			}
			DenseResidualOperation::Operation(operation) => {
				branch.push(CheckpointResidualBranch::Operation(*operation));
			}
		}
	}
	let skip = match state.projection {
		Some(projection) => CheckpointResidualSkip::Projection(checkpoint_parameter(training, projection)?),
		None => CheckpointResidualSkip::Identity,
	};
	Ok(CheckpointResidual {
		branch,
		output_width,
		skip,
		operations: declaration.operations().to_vec(),
	})
}

fn push_checkpoint_parameter_tensors<'a>(
	tensors: &mut Vec<&'a CheckpointTensor>,
	parameter: &'a CheckpointParameter,
) {
	tensors.extend([
		&parameter.parameter,
		&parameter.first_moment,
		&parameter.second_moment,
	]);
}

/// A model checkpoint that can exist only after native execution has completed
/// its exit lifecycle and destroyed its native resources.
#[derive(Clone, Debug)]
pub struct CompletedTrainingCheckpoint {
	execution: CompletedTrainingExecution,
	manifest: CheckpointManifest,
	output_indices: BTreeMap<ValueId, usize>,
}

impl CompletedTrainingCheckpoint {
	pub fn new(execution: CompletedTrainingExecution, manifest: CheckpointManifest) -> CheckpointResult<Self> {
		let output_indices = map_outputs(
			&manifest,
			execution.external_outputs(),
			execution.external_output_values(),
		)?;
		Ok(Self {
			execution,
			manifest,
			output_indices,
		})
	}

	#[must_use]
	pub const fn run(&self) -> RunId {
		self.execution.run()
	}

	#[must_use]
	pub const fn bundle(&self) -> BundleIdentity {
		self.execution.bundle()
	}

	#[must_use]
	pub fn external_outputs(&self) -> &[ExitImage] {
		self.execution.external_outputs()
	}

	#[must_use]
	pub fn metrics(&self) -> &[FinalTrainingMetric] {
		self.execution.metrics()
	}

	#[must_use]
	pub const fn journal(&self) -> &RunJournal {
		self.execution.journal()
	}

	#[must_use]
	pub const fn manifest(&self) -> &CheckpointManifest {
		&self.manifest
	}

	#[must_use]
	pub fn into_execution(self) -> CompletedTrainingExecution {
		self.execution
	}

	/// Persist the semantic model and exact final tensor bits without retaining
	/// or serializing any native artifact.
	pub fn save(&self, path: impl AsRef<Path>) -> CheckpointResult<()> {
		let path = path.as_ref();
		let outputs = self.output_bytes();
		let encoded_bytes = encoded_size(&self.manifest, &outputs)?;
		atomic_save(path, encoded_bytes, |file| {
			encode_checkpoint(&self.manifest, &outputs, file)
		})
	}

	fn output_bytes(&self) -> BTreeMap<ValueId, &[u8]> {
		self.output_indices
			.iter()
			.map(|(value, index)| {
				(
					*value,
					self.execution.external_outputs()[*index].bytes.as_slice(),
				)
			})
			.collect()
	}
}

fn checkpoint_parameter(training: &CompiledTraining, state: ParameterState) -> CheckpointResult<CheckpointParameter> {
	Ok(CheckpointParameter {
		parameter: checkpoint_tensor(training, state.updated_parameter)?,
		first_moment: checkpoint_tensor(training, state.updated_first_moment)?,
		second_moment: checkpoint_tensor(training, state.updated_second_moment)?,
	})
}

fn checkpoint_tensor(training: &CompiledTraining, value: ValueId) -> CheckpointResult<CheckpointTensor> {
	let tensor = training
		.graph()
		.tensors
		.iter()
		.find(|tensor| tensor.id == value)
		.ok_or_else(|| {
			CheckpointError::manifest(format!(
				"checkpoint value {value} has no tensor declaration"
			))
		})?;
	if !tensor.external_output {
		return Err(CheckpointError::manifest(format!(
			"checkpoint value {value} is not an external output"
		)));
	}
	Ok(CheckpointTensor {
		value,
		dtype: tensor.dtype,
		shape: tensor.shape.extents().to_vec(),
		bytes: tensor.storage_bytes.get(),
	})
}

fn map_outputs(
	manifest: &CheckpointManifest,
	outputs: &[ExitImage],
	logical_values: &[ValueId],
) -> CheckpointResult<BTreeMap<ValueId, usize>> {
	if outputs.len() != logical_values.len() {
		return Err(CheckpointError::manifest(format!(
			"execution returned {} physical checkpoint images but {} logical output identities",
			outputs.len(),
			logical_values.len()
		)));
	}
	let expected = manifest
		.tensors()
		.map(|tensor| (tensor.value, tensor))
		.collect::<BTreeMap<_, _>>();
	let mut mapped = BTreeMap::new();
	for (index, (output, value)) in outputs.iter().zip(logical_values).enumerate() {
		let value = *value;
		let Some(tensor) = expected.get(&value) else {
			return Err(CheckpointError::UnexpectedOutput { value });
		};
		if mapped.insert(value, index).is_some() {
			return Err(CheckpointError::DuplicateOutput { value });
		}
		if output.source.dtype != tensor.dtype {
			return Err(CheckpointError::OutputDTypeMismatch {
				value,
				expected: tensor.dtype,
				actual: output.source.dtype,
			});
		}
		let actual = u64::try_from(output.bytes.len()).unwrap_or(u64::MAX);
		if output.source.bytes.get() != tensor.bytes || actual != tensor.bytes {
			return Err(CheckpointError::OutputSizeMismatch {
				value,
				expected: tensor.bytes,
				actual,
			});
		}
	}
	for value in expected.keys() {
		if !mapped.contains_key(value) {
			return Err(CheckpointError::MissingOutput { value: *value });
		}
	}
	Ok(mapped)
}

fn encoded_size(manifest: &CheckpointManifest, outputs: &BTreeMap<ValueId, &[u8]>) -> CheckpointResult<u64> {
	let mut counter = CountingWriter::default();
	encode_checkpoint(manifest, outputs, &mut counter)
		.map_err(|error| CheckpointError::io("measure checkpoint", Path::new("<memory>"), error))?;
	Ok(counter.bytes)
}

fn encode_checkpoint(
	manifest: &CheckpointManifest,
	outputs: &BTreeMap<ValueId, &[u8]>,
	writer: &mut impl Write,
) -> io::Result<()> {
	validate_manifest_topology_for_encoding(manifest)?;
	encode_artifact(&artifact_from_manifest(manifest, outputs)?, writer)
}

fn validate_manifest_topology_for_encoding(manifest: &CheckpointManifest) -> io::Result<()> {
	let valid = match manifest.format_version {
		FLAT_CHECKPOINT_FORMAT_VERSION => !manifest.layers.is_empty() && manifest.blocks.is_empty(),
		STRUCTURED_CHECKPOINT_FORMAT_VERSION => {
			manifest.layers.is_empty()
				&& !manifest.blocks.is_empty()
				&& manifest
					.blocks
					.iter()
					.any(|block| matches!(block, CheckpointBlock::Residual(_)))
		}
		_ => false,
	};
	if !valid {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"checkpoint manifest mixes incompatible flat and structured topology",
		));
	}
	Ok(())
}

fn artifact_from_manifest(
	manifest: &CheckpointManifest,
	outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointArtifact> {
	let layers = manifest
		.layers
		.iter()
		.map(|layer| artifact_layer(layer, outputs))
		.collect::<io::Result<Vec<_>>>()?;
	let blocks = if manifest.format_version == FLAT_CHECKPOINT_FORMAT_VERSION {
		layers
			.iter()
			.cloned()
			.map(CheckpointBlockImage::Layer)
			.collect()
	} else {
		manifest
			.blocks
			.iter()
			.map(|block| artifact_block(block, outputs))
			.collect::<io::Result<Vec<_>>>()?
	};
	Ok(CheckpointArtifact {
		format_version: manifest.format_version,
		vectors: manifest
			.vectors
			.iter()
			.map(|vector| CheckpointArtifactVector {
				source_index: vector.source_index,
				name: vector.name.clone(),
				role: vector.role,
				semantic_type: vector.semantic_type,
				encoding: vector.encoding,
				metadata: artifact_metadata(&vector.metadata),
			})
			.collect(),
		feature_spans: manifest.feature_spans.clone(),
		feature_normalization_mask: feature_normalization_mask(&manifest.feature_spans, manifest.feature_width),
		feature_width: manifest.feature_width,
		task: manifest.task,
		output_adapter: manifest.output_adapter,
		config: manifest.config.clone(),
		bounds: manifest.bounds,
		normalization: manifest
			.normalization
			.iter()
			.map(|tensor| artifact_tensor(tensor, outputs))
			.collect::<io::Result<_>>()?,
		layers,
		blocks,
		temperature: manifest
			.temperature
			.as_ref()
			.map(|tensor| artifact_tensor(tensor, outputs))
			.transpose()?,
	})
}

fn artifact_layer(
	layer: &CheckpointLayer,
	outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointLayerImage> {
	Ok(CheckpointLayerImage {
		declaration: layer.declaration.clone(),
		weight: artifact_parameter(&layer.weight, outputs)?,
		bias: artifact_parameter(&layer.bias, outputs)?,
	})
}

fn artifact_block(
	block: &CheckpointBlock,
	outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointBlockImage> {
	match block {
		CheckpointBlock::Layer(layer) => artifact_layer(layer, outputs).map(CheckpointBlockImage::Layer),
		CheckpointBlock::Residual(residual) => {
			let branch = residual
				.branch
				.iter()
				.map(|step| match step {
					CheckpointResidualBranch::Layer(layer) => {
						artifact_layer(layer, outputs).map(CheckpointResidualBranchImage::Layer)
					}
					CheckpointResidualBranch::Operation(operation) => {
						Ok(CheckpointResidualBranchImage::Operation(*operation))
					}
				})
				.collect::<io::Result<Vec<_>>>()?;
			let skip = match &residual.skip {
				CheckpointResidualSkip::Identity => CheckpointResidualSkipImage::Identity,
				CheckpointResidualSkip::Projection(projection) => {
					CheckpointResidualSkipImage::Projection(artifact_parameter(projection, outputs)?)
				}
			};
			Ok(CheckpointBlockImage::Residual(CheckpointResidualImage {
				branch,
				output_width: residual.output_width,
				skip,
				operations: residual.operations.clone(),
			}))
		}
	}
}

fn artifact_metadata(metadata: &VectorMetadata) -> CheckpointArtifactMetadata {
	match metadata {
		VectorMetadata::None => CheckpointArtifactMetadata::None,
		VectorMetadata::Temporal { origin } => CheckpointArtifactMetadata::Temporal {
			unix_seconds: origin.unix_seconds,
			nanoseconds: origin.nanoseconds,
		},
		VectorMetadata::Categorical { dictionary } => CheckpointArtifactMetadata::Categorical {
			dictionary: dictionary.clone(),
		},
		VectorMetadata::Ordinal { ordered_labels } => CheckpointArtifactMetadata::Ordinal {
			ordered_labels: ordered_labels.clone(),
		},
		VectorMetadata::Image { encoded_variants } => CheckpointArtifactMetadata::Image {
			encoded_variants: encoded_variants
				.iter()
				.map(|variant| CheckpointImageMetadata {
					format: variant.format(),
					width: variant.width(),
					height: variant.height(),
					channels: variant.channels(),
					color_model: variant.color_model(),
					sample_bits: variant.sample_bits(),
					value_layout: variant.value_layout(),
					value_range: variant.value_range(),
				})
				.collect(),
		},
	}
}

fn artifact_parameter(
	parameter: &CheckpointParameter,
	outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<CheckpointParameterImage> {
	Ok(CheckpointParameterImage {
		parameter: artifact_tensor(&parameter.parameter, outputs)?,
		first_moment: artifact_tensor(&parameter.first_moment, outputs)?,
		second_moment: artifact_tensor(&parameter.second_moment, outputs)?,
	})
}

fn artifact_tensor(tensor: &CheckpointTensor, outputs: &BTreeMap<ValueId, &[u8]>) -> io::Result<CheckpointTensorImage> {
	Ok(CheckpointTensorImage {
		dtype: tensor.dtype,
		shape: tensor.shape.clone(),
		bytes: required_output(outputs, tensor.value)?.to_vec(),
	})
}

fn encode_artifact(artifact: &CheckpointArtifact, writer: &mut impl Write) -> io::Result<()> {
	writeln!(writer, "recipe")?;
	writeln!(writer, "\tformat\tdense-training-checkpoint")?;
	writeln!(writer, "\tversion\t{}", artifact.format_version)?;
	writeln!(writer, "\tsemantics")?;
	writeln!(
		writer,
		"\t\tobjective\t{}",
		dense_loss(artifact.config.loss)
	)?;
	writeln!(
		writer,
		"\t\tnormalization\t{}",
		data_normalization(artifact.config.data_normalization)
	)?;
	writeln!(writer, "\t\toptimizer\tadamw")?;
	writeln!(writer, "\tdataset")?;
	writeln!(writer, "\t\tfeature-width\t{}", artifact.feature_width)?;
	writeln!(writer, "\t\ttarget")?;
	match artifact.task {
		DenseTask::BinaryClassification {
			target_vector,
			positive_code,
		} => {
			writeln!(writer, "\t\t\tsource-index\t{target_vector}")?;
			writeln!(writer, "\t\t\ttask\tbinary-classification")?;
			writeln!(writer, "\t\t\tpositive-code\t{positive_code}")?;
		}
		DenseTask::MulticlassClassification {
			target_vector,
			class_count,
			reserved_code,
		} => {
			writeln!(writer, "\t\t\tsource-index\t{target_vector}")?;
			writeln!(writer, "\t\t\ttask\tmulticlass-classification")?;
			writeln!(writer, "\t\t\tclass-count\t{class_count}")?;
			writeln!(writer, "\t\t\treserved-unseen-code\t{reserved_code}")?;
		}
		DenseTask::ScalarRegression { target_vector } => {
			writeln!(writer, "\t\t\tsource-index\t{target_vector}")?;
			writeln!(writer, "\t\t\ttask\tscalar-regression")?;
		}
	}
	writeln!(writer, "\t\tvectors")?;
	for vector in &artifact.vectors {
		writeln!(writer, "\t\t\tvector")?;
		writeln!(writer, "\t\t\t\tsource-index\t{}", vector.source_index)?;
		write!(writer, "\t\t\t\tname-bytes\t")?;
		write_hex(writer, &vector.name)?;
		writeln!(writer)?;
		writeln!(writer, "\t\t\t\trole\t{}", vector_role(vector.role))?;
		writeln!(
			writer,
			"\t\t\t\tsemantic-type\t{}",
			semantic_type(vector.semantic_type)
		)?;
		writeln!(
			writer,
			"\t\t\t\tencoding\t{}",
			vector_encoding(vector.encoding)
		)?;
		encode_artifact_vector_metadata(writer, &vector.metadata)?;
	}
	writeln!(writer, "\t\tfeature-spans")?;
	for span in &artifact.feature_spans {
		writeln!(writer, "\t\t\tspan")?;
		writeln!(writer, "\t\t\t\tsource-index\t{}", span.source_vector())?;
		writeln!(writer, "\t\t\t\tstart\t{}", span.start())?;
		writeln!(writer, "\t\t\t\twidth\t{}", span.width())?;
		match span.lowering() {
			DenseFeatureLowering::NumericScalar => {
				writeln!(writer, "\t\t\t\tlowering\tnumeric-scalar")?;
			}
			DenseFeatureLowering::CategoricalOneHot {
				dictionary_width,
				reserved_index,
			} => {
				writeln!(writer, "\t\t\t\tlowering\tcategorical-one-hot")?;
				writeln!(writer, "\t\t\t\t\tdictionary-width\t{dictionary_width}")?;
				writeln!(writer, "\t\t\t\t\treserved-index\t{reserved_index}")?;
			}
		}
	}
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
		for (index, layer) in artifact.layers.iter().enumerate() {
			writeln!(
				writer,
				"\t\t\t{}\t{}",
				dense_block_kind(layer.declaration.kind()),
				layer.declaration.width()
			)?;
			writeln!(writer, "\t\t\t\tindex\t{index}")?;
			writeln!(
				writer,
				"\t\t\t\toperations\t{}",
				layer.declaration.operations().len()
			)?;
			for operation in layer.declaration.operations() {
				writeln!(
					writer,
					"\t\t\t\t\toperation\t{}",
					dense_operation(*operation)
				)?;
			}
			writeln!(writer, "\t\t\t\tweight")?;
			encode_artifact_parameter(writer, 5, &layer.weight)?;
			writeln!(writer, "\t\t\t\tbias")?;
			encode_artifact_parameter(writer, 5, &layer.bias)?;
		}
	} else {
		for (index, block) in artifact.blocks.iter().enumerate() {
			encode_artifact_block(writer, 3, index, block)?;
		}
	}
	if let Some(temperature) = &artifact.temperature {
		writeln!(writer, "\t\tcalibration")?;
		encode_artifact_tensor(writer, 3, "temperature", temperature)?;
	}
	Ok(())
}

fn encode_artifact_block(
	writer: &mut impl Write,
	depth: usize,
	index: usize,
	block: &CheckpointBlockImage,
) -> io::Result<()> {
	match block {
		CheckpointBlockImage::Layer(layer) => encode_artifact_layer(writer, depth, index, layer),
		CheckpointBlockImage::Residual(residual) => {
			write_tabs(writer, depth)?;
			writeln!(writer, "residual")?;
			write_tabs(writer, depth + 1)?;
			writeln!(writer, "index\t{index}")?;
			write_tabs(writer, depth + 1)?;
			writeln!(writer, "output-width\t{}", residual.output_width)?;
			write_tabs(writer, depth + 1)?;
			writeln!(writer, "branch\t{}", residual.branch.len())?;
			for (step_index, step) in residual.branch.iter().enumerate() {
				match step {
					CheckpointResidualBranchImage::Layer(layer) => {
						encode_artifact_layer(writer, depth + 2, step_index, layer)?;
					}
					CheckpointResidualBranchImage::Operation(operation) => {
						write_tabs(writer, depth + 2)?;
						writeln!(writer, "operation\t{}", dense_operation(*operation))?;
					}
				}
			}
			write_tabs(writer, depth + 1)?;
			match &residual.skip {
				CheckpointResidualSkipImage::Identity => writeln!(writer, "skip\tidentity")?,
				CheckpointResidualSkipImage::Projection(projection) => {
					writeln!(writer, "skip\tlinear-projection")?;
					write_tabs(writer, depth + 2)?;
					writeln!(writer, "weight")?;
					encode_artifact_parameter(writer, depth + 3, projection)?;
				}
			}
			encode_artifact_operations(writer, depth + 1, &residual.operations)
		}
	}
}

fn encode_artifact_layer(
	writer: &mut impl Write,
	depth: usize,
	index: usize,
	layer: &CheckpointLayerImage,
) -> io::Result<()> {
	write_tabs(writer, depth)?;
	writeln!(
		writer,
		"{}\t{}",
		dense_block_kind(layer.declaration.kind()),
		layer.declaration.width()
	)?;
	write_tabs(writer, depth + 1)?;
	writeln!(writer, "index\t{index}")?;
	encode_artifact_operations(writer, depth + 1, layer.declaration.operations())?;
	write_tabs(writer, depth + 1)?;
	writeln!(writer, "weight")?;
	encode_artifact_parameter(writer, depth + 2, &layer.weight)?;
	write_tabs(writer, depth + 1)?;
	writeln!(writer, "bias")?;
	encode_artifact_parameter(writer, depth + 2, &layer.bias)
}

fn encode_artifact_operations(
	writer: &mut impl Write,
	depth: usize,
	operations: &[DenseOperation],
) -> io::Result<()> {
	write_tabs(writer, depth)?;
	writeln!(writer, "operations\t{}", operations.len())?;
	for operation in operations {
		write_tabs(writer, depth + 1)?;
		writeln!(writer, "operation\t{}", dense_operation(*operation))?;
	}
	Ok(())
}

fn encode_artifact_vector_metadata(writer: &mut impl Write, metadata: &CheckpointArtifactMetadata) -> io::Result<()> {
	match metadata {
		CheckpointArtifactMetadata::None => writeln!(writer, "\t\t\t\tmetadata\tnone"),
		CheckpointArtifactMetadata::Temporal {
			unix_seconds,
			nanoseconds,
		} => {
			writeln!(writer, "\t\t\t\tmetadata\ttemporal")?;
			writeln!(writer, "\t\t\t\t\tunix-seconds\t{unix_seconds}")?;
			writeln!(writer, "\t\t\t\t\tnanoseconds\t{nanoseconds}")
		}
		CheckpointArtifactMetadata::Categorical { dictionary } => {
			writeln!(writer, "\t\t\t\tmetadata\tcategorical")?;
			for value in dictionary {
				write!(writer, "\t\t\t\t\tvalue-bytes\t")?;
				write_hex(writer, value)?;
				writeln!(writer)?;
			}
			Ok(())
		}
		CheckpointArtifactMetadata::Ordinal { ordered_labels } => {
			writeln!(writer, "\t\t\t\tmetadata\tordinal")?;
			for value in ordered_labels {
				write!(writer, "\t\t\t\t\tvalue-bytes\t")?;
				write_hex(writer, value)?;
				writeln!(writer)?;
			}
			Ok(())
		}
		CheckpointArtifactMetadata::Image { encoded_variants } => {
			writeln!(writer, "\t\t\t\tmetadata\timage")?;
			for variant in encoded_variants {
				writeln!(writer, "\t\t\t\t\tvariant")?;
				writeln!(
					writer,
					"\t\t\t\t\t\tformat\t{}",
					image_format(variant.format)
				)?;
				writeln!(writer, "\t\t\t\t\t\twidth\t{}", variant.width)?;
				writeln!(writer, "\t\t\t\t\t\theight\t{}", variant.height)?;
				writeln!(
					writer,
					"\t\t\t\t\t\tchannels\t{}",
					variant
						.channels
						.map_or_else(|| "none".to_owned(), |value| value.to_string())
				)?;
				writeln!(
					writer,
					"\t\t\t\t\t\tcolor-model\t{}",
					variant.color_model.map_or("none", image_color_model)
				)?;
				writeln!(
					writer,
					"\t\t\t\t\t\tsample-bits\t{}",
					variant
						.sample_bits
						.map_or_else(|| "none".to_owned(), |value| value.to_string())
				)?;
				writeln!(
					writer,
					"\t\t\t\t\t\tvalue-layout\t{}",
					image_value_layout(variant.value_layout)
				)?;
				writeln!(
					writer,
					"\t\t\t\t\t\tvalue-range\t{}",
					image_value_range(variant.value_range)
				)?;
			}
			Ok(())
		}
	}
}

fn encode_training_config(
	writer: &mut impl Write,
	config: &DenseTrainingConfig,
	bounds: TrainingBounds,
) -> io::Result<()> {
	writeln!(writer, "\ttraining")?;
	writeln!(writer, "\t\tbatch-size\t{}", config.batch_size)?;
	writeln!(writer, "\t\tepochs\t{}", config.epochs)?;
	writeln!(writer, "\t\twarmup-epochs\t{}", config.warmup_epochs)?;
	writeln!(
		writer,
		"\t\tlearning-rate-decay\t{}",
		learning_rate_decay(config.learning_rate_decay)
	)?;
	write_f32_bits(writer, 2, "gradient-clip-norm", config.gradient_clip_norm)?;
	write_f32_bits(
		writer,
		2,
		"normalization-epsilon",
		config.normalization_epsilon,
	)?;
	writeln!(
		writer,
		"\t\treduction-tree-lanes\t{}",
		config.reduction_tree_lanes
	)?;
	writeln!(writer, "\t\trandom-seed\t{}", config.random_seed)?;
	encode_adamw(writer, config.adamw)?;
	writeln!(writer, "\t\tbounds")?;
	writeln!(writer, "\t\t\ttrain-rows\t{}", bounds.train_rows)?;
	writeln!(writer, "\t\t\tbatch-size\t{}", bounds.batch_size)?;
	writeln!(
		writer,
		"\t\t\tbatches-per-epoch\t{}",
		bounds.batches_per_epoch
	)?;
	writeln!(
		writer,
		"\t\t\tpadded-rows-per-epoch\t{}",
		bounds.padded_rows_per_epoch
	)?;
	writeln!(writer, "\t\t\tepochs\t{}", bounds.epochs)?;
	writeln!(
		writer,
		"\t\t\ttraining-iterations\t{}",
		bounds.training_iterations
	)?;
	writeln!(
		writer,
		"\t\t\tcalibration-iterations\t{}",
		bounds.calibration_iterations
	)?;
	writeln!(writer, "\t\t\titerations\t{}", bounds.iterations)?;
	writeln!(
		writer,
		"\t\t\twarmup-iterations\t{}",
		bounds.warmup_iterations
	)
}

fn encode_artifact_data_normalization(writer: &mut impl Write, artifact: &CheckpointArtifact) -> io::Result<()> {
	writeln!(
		writer,
		"\t\tnormalization\t{}",
		data_normalization(artifact.config.data_normalization)
	)?;
	let names: &[&str] = match artifact.config.data_normalization {
		DenseDataNormalization::ZScore => &["mean", "variance"],
		DenseDataNormalization::MinMax => &["minimum", "maximum"],
		DenseDataNormalization::L2Norm => &[],
	};
	if names.len() != artifact.normalization.len() {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"checkpoint normalization tensor count differs from its declaration",
		));
	}
	for (name, tensor) in names.iter().zip(&artifact.normalization) {
		encode_artifact_tensor(writer, 3, name, tensor)?;
	}
	Ok(())
}

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

fn encode_artifact_parameter(
	writer: &mut impl Write,
	depth: usize,
	parameter: &CheckpointParameterImage,
) -> io::Result<()> {
	encode_artifact_tensor(writer, depth, "parameter", &parameter.parameter)?;
	encode_artifact_tensor(writer, depth, "first-moment", &parameter.first_moment)?;
	encode_artifact_tensor(writer, depth, "second-moment", &parameter.second_moment)
}

fn encode_artifact_tensor(
	writer: &mut impl Write,
	depth: usize,
	name: &str,
	tensor: &CheckpointTensorImage,
) -> io::Result<()> {
	write_tabs(writer, depth)?;
	writeln!(writer, "{name}")?;
	write_tabs(writer, depth + 1)?;
	writeln!(writer, "dtype\t{}", dtype(tensor.dtype))?;
	write_tabs(writer, depth + 1)?;
	write!(writer, "shape")?;
	for extent in &tensor.shape {
		write!(writer, "\t{extent}")?;
	}
	writeln!(writer)?;
	write_tabs(writer, depth + 1)?;
	writeln!(writer, "payload\traw-bytes-hex")?;
	if tensor.bytes.is_empty() {
		write_tabs(writer, depth + 2)?;
		writeln!(writer, "0x")?;
	} else {
		for chunk in tensor.bytes.chunks(HEX_CHUNK_BYTES) {
			write_tabs(writer, depth + 2)?;
			write_hex(writer, chunk)?;
			writeln!(writer)?;
		}
	}
	Ok(())
}

fn required_output<'a>(outputs: &'a BTreeMap<ValueId, &[u8]>, value: ValueId) -> io::Result<&'a [u8]> {
	outputs.get(&value).copied().ok_or_else(|| {
		io::Error::new(
			io::ErrorKind::InvalidData,
			format!("validated checkpoint output {value} disappeared"),
		)
	})
}

fn feature_normalization_mask(spans: &[CompiledFeatureSpan], width: usize) -> Vec<u32> {
	let mut mask = vec![0.0f32.to_bits(); width];
	for span in spans {
		if span.lowering() == DenseFeatureLowering::NumericScalar {
			let end = span.start().saturating_add(span.width()).min(width);
			mask[span.start().min(width)..end].fill(1.0f32.to_bits());
		}
	}
	mask
}

fn write_tabs(writer: &mut impl Write, depth: usize) -> io::Result<()> {
	for _ in 0..depth {
		writer.write_all(b"\t")?;
	}
	Ok(())
}

fn write_hex(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
	const HEX: &[u8; 16] = b"0123456789abcdef";
	writer.write_all(b"0x")?;
	for byte in bytes {
		writer.write_all(&[HEX[usize::from(*byte >> 4)], HEX[usize::from(*byte & 0x0f)]])?;
	}
	Ok(())
}

const fn dtype(dtype: DType) -> &'static str {
	match dtype {
		DType::F32 => "f32",
		DType::I32 => "int32",
	}
}

const fn vector_role(role: VectorRole) -> &'static str {
	match role {
		VectorRole::Feature => "feature",
		VectorRole::Target => "target",
	}
}

const fn semantic_type(semantic_type: SemanticType) -> &'static str {
	match semantic_type {
		SemanticType::Numeric => "numeric",
		SemanticType::Temporal => "temporal",
		SemanticType::Categorical => "categorical",
		SemanticType::Ordinal => "ordinal",
		SemanticType::Text => "text",
		SemanticType::Image => "image",
		SemanticType::Binary => "binary",
	}
}

const fn image_format(format: EncodedImageFormat) -> &'static str {
	match format {
		EncodedImageFormat::Png => "png",
		EncodedImageFormat::Jpeg => "jpeg",
		EncodedImageFormat::Gif87a => "gif87a",
		EncodedImageFormat::Gif89a => "gif89a",
		EncodedImageFormat::Bmp => "bmp",
		EncodedImageFormat::WebP => "webp",
	}
}

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
	}
}

const fn image_value_layout(layout: ImageValueLayout) -> &'static str {
	match layout {
		ImageValueLayout::EncodedFile => "encoded-file",
	}
}

const fn image_value_range(range: ImageValueRange) -> &'static str {
	match range {
		ImageValueRange::EncodedBytes => "encoded-bytes",
	}
}

const fn vector_encoding(encoding: VectorEncoding) -> &'static str {
	match encoding {
		VectorEncoding::F32 => "f32",
		VectorEncoding::I32 => "int32",
		VectorEncoding::RelativeSecondsI32 => "relative-seconds-int32",
		VectorEncoding::DictionaryI32 => "dictionary-int32",
		VectorEncoding::OrdinalI32 => "ordinal-int32",
		VectorEncoding::Utf8 => "utf8",
		VectorEncoding::Bytes => "bytes",
	}
}

const fn dense_activation(activation: DenseActivation) -> &'static str {
	match activation {
		DenseActivation::Linear => "linear",
		DenseActivation::Cosine => "cosine",
		DenseActivation::Exponential => "exponential",
		DenseActivation::Logarithm => "logarithm",
		DenseActivation::Huber => "huber",
		DenseActivation::Tangent => "tangent",
		DenseActivation::Relu => "relu",
		DenseActivation::Gelu => "gelu",
		DenseActivation::Silu => "silu",
	}
}

const fn dense_block_kind(kind: DenseBlockKind) -> &'static str {
	match kind {
		DenseBlockKind::Layer => "layer",
		DenseBlockKind::Perc => "perc",
	}
}

const fn dense_operation(operation: DenseOperation) -> &'static str {
	match operation {
		DenseOperation::Activation(activation) => dense_activation(activation),
		DenseOperation::Normalization(crate::DenseNormalization::Layer) => "layer-normalization",
		DenseOperation::Normalization(crate::DenseNormalization::Batch) => "batch-normalization",
	}
}

const fn data_normalization(normalization: DenseDataNormalization) -> &'static str {
	match normalization {
		DenseDataNormalization::ZScore => "z-score",
		DenseDataNormalization::MinMax => "min-max",
		DenseDataNormalization::L2Norm => "l2-norm",
	}
}

const fn dense_loss(loss: DenseLoss) -> &'static str {
	match loss {
		DenseLoss::BinaryCrossEntropy => "binary-cross-entropy-with-logits",
		DenseLoss::MeanSquaredError => "mean-squared-error",
		DenseLoss::MeanAbsoluteError => "mean-absolute-error",
		DenseLoss::CrossEntropy => "cross-entropy",
		DenseLoss::Huber => "huber-unit-delta",
	}
}

const fn learning_rate_decay(decay: LearningRateDecay) -> &'static str {
	match decay {
		LearningRateDecay::Linear => "linear",
		LearningRateDecay::Cosine => "cosine",
		LearningRateDecay::Exponential => "exponential",
	}
}

#[derive(Debug, Default)]
struct CountingWriter {
	bytes: u64,
}

impl Write for CountingWriter {
	fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
		let length = u64::try_from(bytes.len()).map_err(|_| io::Error::other("checkpoint size exceeds u64"))?;
		self.bytes = self
			.bytes
			.checked_add(length)
			.ok_or_else(|| io::Error::other("checkpoint size exceeds u64"))?;
		Ok(bytes.len())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

fn atomic_save(
	target: &Path,
	encoded_bytes: u64,
	write_checkpoint: impl FnOnce(&mut File) -> io::Result<()>,
) -> CheckpointResult<()> {
	let parent = normalized_parent(target)?;
	validate_target(target, parent)?;
	let statistics = rustix::fs::statvfs(parent)
		.map_err(|error| CheckpointError::io("query filesystem capacity", parent, error.into()))?;
	let allocation = allocation_bytes(encoded_bytes, statistics.f_frsize)?;
	require_capacity(
		target,
		statistics.f_bavail,
		statistics.f_frsize,
		allocation,
		EXACT_USER_RESERVATION.get(),
	)?;
	let (mut temporary, mut guard) = create_private_temporary(target, parent)?;
	write_checkpoint(&mut temporary)
		.map_err(|error| CheckpointError::io("write checkpoint temporary", guard.path.clone(), error))?;
	temporary
		.flush()
		.map_err(|error| CheckpointError::io("flush checkpoint temporary", guard.path.clone(), error))?;
	temporary
		.sync_all()
		.map_err(|error| CheckpointError::io("sync checkpoint temporary", guard.path.clone(), error))?;
	let actual = temporary
		.metadata()
		.map_err(|error| CheckpointError::io("inspect checkpoint temporary", guard.path.clone(), error))?
		.len();
	if actual != encoded_bytes {
		return Err(CheckpointError::invalid_target(
			target,
			format!("encoder wrote {actual} bytes after measuring {encoded_bytes}"),
		));
	}
	drop(temporary);
	fs::rename(&guard.path, target)
		.map_err(|error| CheckpointError::io("atomically install checkpoint", target, error))?;
	guard.armed = false;
	let directory =
		File::open(parent).map_err(|error| CheckpointError::io("open checkpoint parent", parent, error))?;
	directory
		.sync_all()
		.map_err(|error| CheckpointError::io("sync checkpoint parent", parent, error))
}

fn normalized_parent(target: &Path) -> CheckpointResult<&Path> {
	if target.as_os_str().is_empty() || target.file_name().is_none() {
		return Err(CheckpointError::invalid_target(
			target,
			"target must name a file",
		));
	}
	let parent = target.parent().unwrap_or_else(|| Path::new("."));
	Ok(if parent.as_os_str().is_empty() {
		Path::new(".")
	} else {
		parent
	})
}

fn validate_target(target: &Path, parent: &Path) -> CheckpointResult<()> {
	let parent_metadata =
		fs::metadata(parent).map_err(|error| CheckpointError::io("inspect checkpoint parent", parent, error))?;
	if !parent_metadata.is_dir() {
		return Err(CheckpointError::invalid_target(
			target,
			"parent is not a directory",
		));
	}
	match fs::symlink_metadata(target) {
		Ok(metadata) if metadata.file_type().is_symlink() => Err(CheckpointError::invalid_target(
			target,
			"existing target is a symbolic link",
		)),
		Ok(metadata) if !metadata.is_file() => Err(CheckpointError::invalid_target(
			target,
			"existing target is not a regular file",
		)),
		Ok(_) => Ok(()),
		Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
		Err(error) => Err(CheckpointError::io(
			"inspect checkpoint target",
			target,
			error,
		)),
	}
}

fn allocation_bytes(encoded_bytes: u64, fragment_bytes: u64) -> CheckpointResult<u64> {
	if fragment_bytes == 0 {
		return Err(CheckpointError::manifest(
			"filesystem reports a zero fragment size",
		));
	}
	if encoded_bytes == 0 {
		return Ok(0);
	}
	let fragments = encoded_bytes
		.checked_add(fragment_bytes - 1)
		.and_then(|rounded| rounded.checked_div(fragment_bytes))
		.ok_or_else(|| CheckpointError::manifest("checkpoint allocation size overflowed"))?;
	fragments
		.checked_mul(fragment_bytes)
		.ok_or_else(|| CheckpointError::manifest("checkpoint allocation size overflowed"))
}

fn require_capacity(
	target: &Path,
	available_fragments: u64,
	fragment_bytes: u64,
	checkpoint_allocation: u64,
	reservation: u64,
) -> CheckpointResult<()> {
	let available = available_fragments
		.checked_mul(fragment_bytes)
		.ok_or_else(|| CheckpointError::manifest("available filesystem capacity overflowed"))?;
	let required = checkpoint_allocation
		.checked_add(reservation)
		.ok_or_else(|| CheckpointError::manifest("checkpoint capacity requirement overflowed"))?;
	if available < required {
		return Err(CheckpointError::InsufficientCapacity {
			path: target.to_path_buf(),
			available,
			checkpoint_allocation,
			reservation,
		});
	}
	Ok(())
}

fn create_private_temporary(target: &Path, parent: &Path) -> CheckpointResult<(File, TemporaryGuard)> {
	let filename = target
		.file_name()
		.ok_or_else(|| CheckpointError::invalid_target(target, "target must name a file"))?;
	for _ in 0..TEMP_CREATE_ATTEMPTS {
		let sequence = NEXT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
		let mut temporary_name = OsString::from(".");
		temporary_name.push(filename);
		temporary_name.push(format!(".recipe-tmp-{}-{sequence}", std::process::id()));
		let temporary_path = parent.join(temporary_name);
		match OpenOptions::new()
			.write(true)
			.create_new(true)
			.mode(0o600)
			.open(&temporary_path)
		{
			Ok(file) => {
				return Ok((
					file,
					TemporaryGuard {
						path: temporary_path,
						armed: true,
					},
				));
			}
			Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
			Err(error) => {
				return Err(CheckpointError::io(
					"create checkpoint temporary",
					temporary_path,
					error,
				));
			}
		}
	}
	Err(CheckpointError::invalid_target(
		target,
		"could not create a unique private temporary file",
	))
}

#[derive(Debug)]
struct TemporaryGuard {
	path: PathBuf,
	armed: bool,
}

impl Drop for TemporaryGuard {
	fn drop(&mut self) {
		if self.armed {
			let _ = fs::remove_file(&self.path);
		}
	}
}

#[cfg(test)]
pub(crate) fn encoded_test_checkpoint_fixture() -> Vec<u8> {
	tests::encoded_test_checkpoint()
}

#[cfg(test)]
pub(crate) fn encoded_categorical_feature_test_checkpoint_fixture() -> Vec<u8> {
	tests::encoded_categorical_feature_test_checkpoint()
}

#[cfg(test)]
mod tests {
	use core::num::{NonZeroU64, NonZeroUsize};
	use std::os::unix::fs::PermissionsExt;

	use recipe_core::{ArenaObjectId, ByteCount, ByteOffset, DeviceId, ResolvedValueLocation, TaskId};

	use super::*;

	#[test]
	fn checkpoint_v5_decodes_and_reencodes_canonical_bytes_identically() {
		let manifest = test_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let expected_artifact = artifact_from_manifest(&manifest, &outputs).unwrap();
		validate_artifact(&expected_artifact).unwrap();
		let mut encoded = Vec::new();
		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let decoded = decode_checkpoint(&encoded, CheckpointDecodeLimits::default()).unwrap();

		assert_eq!(decoded, expected_artifact);
		assert_eq!(decoded.encode().unwrap(), encoded);
	}

	#[test]
	fn checkpoint_v6_roundtrips_ordered_residual_projection_and_every_optimizer_tensor() {
		let manifest = projected_residual_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let expected_artifact = artifact_from_manifest(&manifest, &outputs).unwrap();
		validate_artifact(&expected_artifact).unwrap();
		let mut encoded = Vec::new();
		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let decoded = decode_checkpoint(&encoded, CheckpointDecodeLimits::default()).unwrap();

		assert_eq!(decoded.format_version(), STRUCTURED_CHECKPOINT_FORMAT_VERSION);
		assert!(decoded.layers().is_empty());
		assert_eq!(decoded, expected_artifact);
		let [CheckpointBlockImage::Residual(residual), CheckpointBlockImage::Layer(adapter)] =
			decoded.blocks()
		else {
			panic!("expected residual plus effective adapter topology");
		};
		assert_eq!(
			residual.branch(),
			[
				CheckpointResidualBranchImage::Operation(DenseOperation::Activation(DenseActivation::Relu)),
				CheckpointResidualBranchImage::Layer(CheckpointLayerImage {
					declaration: DenseLayer::with_operations(
						NonZeroU64::new(2).unwrap(),
						[DenseOperation::Activation(DenseActivation::Silu)],
					),
					weight: artifact_parameter(&test_parameter(3, &[1, 2]), &outputs).unwrap(),
					bias: artifact_parameter(&test_parameter(6, &[2]), &outputs).unwrap(),
				}),
				CheckpointResidualBranchImage::Operation(DenseOperation::Activation(DenseActivation::Gelu)),
			]
		);
		assert_eq!(residual.output_width().get(), 2);
		let CheckpointResidualSkipImage::Projection(projection) = residual.skip() else {
			panic!("one-to-two residual requires a projection");
		};
		assert_eq!(projection.parameter().shape(), [1, 2]);
		assert_eq!(projection.first_moment().shape(), [1, 2]);
		assert_eq!(projection.second_moment().shape(), [1, 2]);
		assert_eq!(
			residual.operations(),
			[DenseOperation::Normalization(crate::DenseNormalization::Layer)]
		);
		assert_eq!(adapter.declaration().width().get(), 1);
		assert_eq!(decoded.encode().unwrap(), encoded);
	}

	#[test]
	fn checkpoint_v6_roundtrips_identity_skip_without_inventing_projection_state() {
		let manifest = identity_residual_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();
		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let decoded = decode_checkpoint(&encoded, CheckpointDecodeLimits::default()).unwrap();

		let [CheckpointBlockImage::Residual(residual)] = decoded.blocks() else {
			panic!("expected one identity residual");
		};
		assert_eq!(residual.skip(), &CheckpointResidualSkipImage::Identity);
		assert_eq!(decoded.encode().unwrap(), encoded);
	}

	#[test]
	fn checkpoint_v6_rejects_branch_projection_and_shape_corruption_with_typed_paths() {
		let encoded = encoded_projected_residual_checkpoint();
		let text = String::from_utf8(encoded).unwrap();

		let bad_count = text.replacen("\t\t\t\tbranch\t3", "\t\t\t\tbranch\t4", 1);
		assert_decode_error(
			bad_count.as_bytes(),
			CheckpointDecodeErrorKind::InconsistentValue,
			"recipe.model.blocks[0].branch.count",
		);

		let bad_width = text.replacen("\t\t\t\toutput-width\t2", "\t\t\t\toutput-width\t3", 1);
		assert_decode_error(
			bad_width.as_bytes(),
			CheckpointDecodeErrorKind::InconsistentValue,
			"recipe.model.blocks[0].output-width",
		);

		let skip_start = text.find("\t\t\t\tskip\tlinear-projection\n").unwrap();
		let operations_start = text[skip_start..]
			.find("\t\t\t\toperations\t")
			.map(|offset| skip_start + offset)
			.unwrap();
		let mut missing_projection = text.clone();
		missing_projection.replace_range(
			skip_start..operations_start,
			"\t\t\t\tskip\tidentity\n",
		);
		assert_decode_error(
			missing_projection.as_bytes(),
			CheckpointDecodeErrorKind::InconsistentValue,
			"recipe.model.blocks[0].skip",
		);

		let projection_shape = "\t\t\t\t\t\t\tshape\t1\t2";
		let shape_start = text[skip_start..]
			.find(projection_shape)
			.map(|offset| skip_start + offset)
			.unwrap();
		let mut bad_shape = text.clone();
		bad_shape.replace_range(
			shape_start..shape_start + projection_shape.len(),
			"\t\t\t\t\t\t\tshape\t1\t3",
		);
		assert_decode_error(
			bad_shape.as_bytes(),
			CheckpointDecodeErrorKind::InconsistentValue,
			"recipe.model.blocks[0].skip.weight.parameter.shape",
		);
	}

	#[test]
	fn checkpoint_encoder_rejects_mixed_legacy_and_structured_topology() {
		let mut manifest = projected_residual_manifest();
		manifest.layers = test_manifest().layers;
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let error = encode_checkpoint(&manifest, &outputs, &mut Vec::new()).unwrap_err();

		assert_eq!(error.kind(), io::ErrorKind::InvalidData);
		assert!(error.to_string().contains("mixes incompatible"));
	}

	#[test]
	fn checkpoint_v5_roundtrips_multiclass_reserved_route_perc_operations_and_adapter() {
		let manifest = multiclass_adapter_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();
		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let decoded = decode_checkpoint(&encoded, CheckpointDecodeLimits::default()).unwrap();

		assert_eq!(
			decoded.task(),
			DenseTask::MulticlassClassification {
				target_vector: 1,
				class_count: 4,
				reserved_code: 3,
			}
		);
		assert_eq!(
			decoded.layers()[0].declaration().kind(),
			DenseBlockKind::Perc
		);
		assert_eq!(
			decoded.layers()[0].declaration().operations(),
			[
				DenseOperation::Normalization(crate::DenseNormalization::Layer),
				DenseOperation::Activation(DenseActivation::Relu),
			]
		);
		assert_eq!(
			decoded.decode_multiclass_class(0),
			Some(DecodedMulticlassClass::Label(&[0x00]))
		);
		assert_eq!(
			decoded.decode_multiclass_class(3),
			Some(DecodedMulticlassClass::ReservedUnseen)
		);
		assert_eq!(decoded.decode_multiclass_class(4), None);
		assert_eq!(decoded.encode().unwrap(), encoded);
	}

	#[test]
	fn checkpoint_v5_roundtrips_calibration_and_all_data_normalization_modes() {
		let mut calibrated = test_manifest();
		calibrated.bounds.calibration_iterations = 1;
		calibrated.bounds.iterations = NonZeroU64::new(2).unwrap();
		calibrated.temperature = Some(test_tensor(9, DType::F32));
		let mut calibrated_owned = owned_outputs(&calibrated);
		calibrated_owned.insert(ValueId::new(9), 1.0f32.to_le_bytes().to_vec());
		let calibrated_outputs = borrowed_outputs(&calibrated_owned);
		let mut encoded = Vec::new();
		encode_checkpoint(&calibrated, &calibrated_outputs, &mut encoded).unwrap();
		let decoded = decode_checkpoint(&encoded, CheckpointDecodeLimits::default()).unwrap();
		assert_eq!(decoded.temperature().unwrap().bytes(), 1.0f32.to_le_bytes());
		assert_eq!(decoded.encode().unwrap(), encoded);

		let mut min_max = test_manifest();
		min_max.config.data_normalization = DenseDataNormalization::MinMax;
		assert_manifest_roundtrip(&min_max);

		let mut l2 = test_manifest();
		l2.config.data_normalization = DenseDataNormalization::L2Norm;
		l2.normalization.clear();
		assert_manifest_roundtrip(&l2);
	}

	#[test]
	fn checkpoint_v5_image_metadata_accepts_only_ingestible_canonical_header_facts() {
		let path = CheckpointPath::root()
			.field("recipe")
			.field("dataset")
			.field("vectors")
			.index(0)
			.field("metadata");
		let png = CheckpointImageMetadata {
			format: EncodedImageFormat::Png,
			width: 16,
			height: 8,
			channels: Some(4),
			color_model: Some(ImageColorModel::Rgba),
			sample_bits: Some(8),
			value_layout: ImageValueLayout::EncodedFile,
			value_range: ImageValueRange::EncodedBytes,
		};
		validate_image_metadata(&[png], &path).unwrap();

		let impossible_png = CheckpointImageMetadata {
			channels: Some(3),
			..png
		};
		let error = validate_image_metadata(&[impossible_png], &path).unwrap_err();
		let CheckpointError::Decode(error) = error else {
			panic!("expected typed decode error, got {error}");
		};
		assert_eq!(error.kind(), CheckpointDecodeErrorKind::InconsistentValue);
		assert_eq!(
			error.path().to_string(),
			"recipe.dataset.vectors[0].metadata[0]"
		);

		let jpeg = CheckpointImageMetadata {
			format: EncodedImageFormat::Jpeg,
			width: 16,
			height: 8,
			channels: Some(3),
			color_model: Some(ImageColorModel::YCbCr),
			sample_bits: Some(8),
			value_layout: ImageValueLayout::EncodedFile,
			value_range: ImageValueRange::EncodedBytes,
		};
		let error = validate_image_metadata(&[jpeg, png], &path).unwrap_err();
		let CheckpointError::Decode(error) = error else {
			panic!("expected typed decode error, got {error}");
		};
		assert_eq!(error.kind(), CheckpointDecodeErrorKind::InconsistentValue);
		assert_eq!(
			error.path().to_string(),
			"recipe.dataset.vectors[0].metadata[1]"
		);
	}

	#[test]
	fn checkpoint_v5_decoder_rejects_missing_duplicate_and_unknown_fields_with_typed_paths() {
		let encoded = encoded_test_checkpoint();
		let text = String::from_utf8(encoded).unwrap();

		let missing = text.replacen("\tversion\t5\n", "", 1);
		assert_decode_error(
			missing.as_bytes(),
			CheckpointDecodeErrorKind::MissingField,
			"recipe.version",
		);

		let duplicate = text.replacen("\tversion\t5\n", "\tversion\t5\n\tversion\t5\n", 1);
		assert_decode_error(
			duplicate.as_bytes(),
			CheckpointDecodeErrorKind::DuplicateField,
			"recipe.version",
		);

		let unknown = text.replacen("recipe\n", "recipe\n\tunknown\t0\n", 1);
		assert_decode_error(
			unknown.as_bytes(),
			CheckpointDecodeErrorKind::UnknownField,
			"recipe.unknown",
		);
	}

	#[test]
	fn checkpoint_v5_decoder_rejects_noncanonical_values_and_enforces_limits() {
		let encoded = encoded_test_checkpoint();
		let text = String::from_utf8(encoded.clone()).unwrap();

		let version = text.replacen("\tversion\t5\n", "\tversion\t05\n", 1);
		assert_decode_error(
			version.as_bytes(),
			CheckpointDecodeErrorKind::InvalidValue,
			"recipe.version",
		);

		let bad_hex = text.replacen("name-bytes\t0x", "name-bytes\t0X", 1);
		assert_decode_error(
			bad_hex.as_bytes(),
			CheckpointDecodeErrorKind::InvalidValue,
			"recipe.dataset.vectors[0].name-bytes",
		);

		let mut source_limits = CheckpointDecodeLimits::default();
		source_limits.source_bytes = encoded.len() - 1;
		assert_decode_error_with_limits(
			&encoded,
			source_limits,
			CheckpointDecodeErrorKind::LimitExceeded,
			"<checkpoint>",
		);

		let mut payload_limits = CheckpointDecodeLimits::default();
		payload_limits.total_payload_bytes = 31;
		assert_decode_error_with_limits(
			&encoded,
			payload_limits,
			CheckpointDecodeErrorKind::LimitExceeded,
			"recipe.model.blocks[0].bias.second-moment.payload",
		);
	}

	#[test]
	fn checkpoint_v5_decoder_rejects_width_mask_shape_and_nonfinite_state_tampering() {
		let encoded = encoded_test_checkpoint();
		let text = String::from_utf8(encoded).unwrap();

		let input_width = text.replacen("\t\tinput-width\t1\n", "\t\tinput-width\t2\n", 1);
		assert_decode_error(
			input_width.as_bytes(),
			CheckpointDecodeErrorKind::InconsistentValue,
			"recipe.model.input-width",
		);

		let mask = text.replacen(
			"\t\t\tvalue-bits\t0x3f800000\n",
			"\t\t\tvalue-bits\t0x00000000\n",
			1,
		);
		assert_decode_error(
			mask.as_bytes(),
			CheckpointDecodeErrorKind::InconsistentValue,
			"recipe.dataset.feature-normalization-mask[0]",
		);

		let shape = text.replacen("\t\t\t\tshape\t1\n", "\t\t\t\tshape\t2\n", 1);
		assert_decode_error(
			shape.as_bytes(),
			CheckpointDecodeErrorKind::InconsistentValue,
			"recipe.model.normalization.mean.shape",
		);

		let nonfinite = text.replacen("\t\t\t\t0x00000000\n", "\t\t\t\t0x0000c07f\n", 1);
		assert_decode_error(
			nonfinite.as_bytes(),
			CheckpointDecodeErrorKind::InvalidValue,
			"recipe.model.normalization.mean.payload[0]",
		);
	}

	#[test]
	fn codec_is_deterministic_valid_ogdl_and_preserves_raw_f32_and_i32_bits() {
		let mut manifest = test_manifest();
		manifest.normalization[0].dtype = DType::I32;
		let mut owned = owned_outputs(&manifest);
		owned.insert(
			manifest.normalization[0].value,
			vec![0xff, 0xff, 0xff, 0xff],
		);
		owned.insert(
			manifest.normalization[1].value,
			vec![0x01, 0x00, 0xc0, 0x7f],
		);
		let outputs = borrowed_outputs(&owned);
		let mut first = Vec::new();
		let mut second = Vec::new();

		encode_checkpoint(&manifest, &outputs, &mut first).unwrap();
		encode_checkpoint(&manifest, &outputs, &mut second).unwrap();

		assert_eq!(first, second);
		let text = String::from_utf8(first).unwrap();
		recipe_ogdl::Graph::parse(&text).unwrap();
		assert!(text.contains("dtype\tint32"));
		assert!(text.contains("0xffffffff"));
		assert!(text.contains("dtype\tf32"));
		assert!(text.contains("0x0100c07f"));
		assert!(!text.contains("cuda"));
		assert!(!text.contains("hsaco"));
		assert!(!text.contains("artifact"));
	}

	#[test]
	fn codec_reports_the_declared_normalization_in_every_schema_location() {
		let mut manifest = test_manifest();
		manifest.config.data_normalization = DenseDataNormalization::MinMax;
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();

		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let text = String::from_utf8(encoded).unwrap();
		assert_eq!(text.matches("normalization\tmin-max").count(), 2);
		assert!(!text.contains("normalization\tz-score"));
	}

	#[test]
	fn checkpoint_preserves_perc_instead_of_relabeling_it_as_layer() {
		let mut manifest = test_manifest();
		let declaration = DenseLayer::with_kind(DenseBlockKind::Perc, NonZeroU64::new(1).unwrap(), []);
		manifest.config.layers[0] = declaration.clone();
		manifest.layers[0].declaration = declaration;
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();

		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let text = String::from_utf8(encoded).unwrap();
		assert!(text.contains("\t\t\tperc\t1\n"));
		assert!(!text.contains("\t\t\tlayer\t1\n"));
	}

	#[test]
	fn checkpoint_v5_records_categorical_dictionary_and_reserved_span_identity() {
		let manifest = categorical_feature_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();

		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let text = String::from_utf8(encoded).unwrap();
		recipe_ogdl::Graph::parse(&text).unwrap();
		decode_checkpoint(text.as_bytes(), CheckpointDecodeLimits::default()).unwrap();
		assert!(text.contains("version\t5"));
		assert!(text.contains("lowering\tcategorical-one-hot"));
		assert!(text.contains("dictionary-width\t2"));
		assert!(text.contains("reserved-index\t2"));
	}

	#[test]
	fn checkpoint_v5_records_multiclass_target_and_effective_output_adapter() {
		let mut manifest = test_manifest();
		manifest.task = DenseTask::MulticlassClassification {
			target_vector: 7,
			class_count: 4,
			reserved_code: 3,
		};
		manifest.output_adapter = Some(DenseOutputAdapter::new(
			NonZeroU64::new(2).unwrap(),
			NonZeroU64::new(4).unwrap(),
		));
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();

		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();

		let text = String::from_utf8(encoded).unwrap();
		recipe_ogdl::Graph::parse(&text).unwrap();
		assert!(text.contains("task\tmulticlass-classification"));
		assert!(text.contains("class-count\t4"));
		assert!(text.contains("reserved-unseen-code\t3"));
		assert!(text.contains("output-adapter\tlinear-projection"));
		assert!(text.contains("source-width\t2"));
		assert!(text.contains("target-width\t4"));
	}

	#[test]
	fn output_mapping_rejects_missing_duplicate_unexpected_dtype_and_size() {
		let manifest = test_manifest();
		let complete = exit_outputs(&manifest);
		let complete_values = exit_output_values(&manifest);

		let mut missing = complete.clone();
		let mut missing_values = complete_values.clone();
		missing.pop().unwrap();
		let missing_value = missing_values.pop().unwrap();
		assert!(matches!(
			map_outputs(&manifest, &missing, &missing_values),
			Err(CheckpointError::MissingOutput { value }) if value == missing_value
		));

		let mut duplicate = complete.clone();
		let mut duplicate_values = complete_values.clone();
		let duplicate_value = duplicate_values[0];
		duplicate.push(duplicate[0].clone());
		duplicate_values.push(duplicate_value);
		assert!(matches!(
			map_outputs(&manifest, &duplicate, &duplicate_values),
			Err(CheckpointError::DuplicateOutput { value }) if value == duplicate_value
		));

		let unexpected = complete.clone();
		let mut unexpected_values = complete_values.clone();
		unexpected_values[0] = ValueId::new(999);
		assert!(matches!(
			map_outputs(&manifest, &unexpected, &unexpected_values),
			Err(CheckpointError::UnexpectedOutput { value }) if value == ValueId::new(999)
		));

		let mut wrong_dtype = complete.clone();
		let dtype_value = complete_values[0];
		wrong_dtype[0].source.dtype = DType::I32;
		assert!(matches!(
			map_outputs(&manifest, &wrong_dtype, &complete_values),
			Err(CheckpointError::OutputDTypeMismatch { value, .. }) if value == dtype_value
		));

		let mut wrong_size = complete;
		let size_value = complete_values[0];
		wrong_size[0].bytes.pop();
		assert!(matches!(
			map_outputs(&manifest, &wrong_size, &complete_values),
			Err(CheckpointError::OutputSizeMismatch { value, .. }) if value == size_value
		));
	}

	#[test]
	fn output_mapping_keeps_logical_identity_separate_from_physical_arena_identity() {
		let manifest = test_manifest();
		let mut outputs = exit_outputs(&manifest);
		let logical_values = exit_output_values(&manifest);
		for (index, output) in outputs.iter_mut().enumerate() {
			output.source.value = ValueId::new(47 + u64::try_from(index).unwrap());
		}

		let mapped = map_outputs(&manifest, &outputs, &logical_values).unwrap();

		assert_eq!(mapped.len(), logical_values.len());
		for logical in logical_values {
			assert!(mapped.contains_key(&logical));
		}
	}

	#[test]
	fn quota_rejection_precedes_any_file_creation() {
		let directory = TestDirectory::create();
		let target = directory.path.join("model.ogdl");

		let error = require_capacity(&target, 10, 4, 8, 33).unwrap_err();

		assert!(matches!(
			error,
			CheckpointError::InsufficientCapacity {
				available: 40,
				checkpoint_allocation: 8,
				reservation: 33,
				..
			}
		));
		assert!(!target.exists());
		assert!(fs::read_dir(&directory.path).unwrap().next().is_none());
	}

	#[test]
	fn atomic_save_replaces_target_privately_and_leaves_no_temporary() {
		let directory = TestDirectory::create();
		let target = directory.path.join("model.ogdl");
		fs::write(&target, b"old").unwrap();

		atomic_save(&target, 3, |file| file.write_all(b"new")).unwrap();

		assert_eq!(fs::read(&target).unwrap(), b"new");
		assert_eq!(
			fs::metadata(&target).unwrap().permissions().mode() & 0o777,
			0o600
		);
		assert_no_temporary(&directory.path);
	}

	#[test]
	fn failed_atomic_write_cleans_temporary_and_preserves_target() {
		let directory = TestDirectory::create();
		let target = directory.path.join("model.ogdl");
		fs::write(&target, b"old").unwrap();

		let error = atomic_save(&target, 3, |file| {
			file.write_all(b"ne")?;
			Err(io::Error::other("injected writer failure"))
		})
		.unwrap_err();

		assert!(matches!(
			error,
			CheckpointError::Io {
				operation: "write checkpoint temporary",
				..
			}
		));
		assert_eq!(fs::read(&target).unwrap(), b"old");
		assert_no_temporary(&directory.path);
	}

	fn test_manifest() -> CheckpointManifest {
		let declaration = DenseLayer::new(NonZeroU64::new(1).unwrap(), DenseActivation::Linear);
		CheckpointManifest {
			format_version: FLAT_CHECKPOINT_FORMAT_VERSION,
			vectors: vec![
				CheckpointVectorSchema {
					source_index: 0,
					name: b"feature\nbytes".to_vec(),
					role: VectorRole::Feature,
					semantic_type: SemanticType::Numeric,
					encoding: VectorEncoding::F32,
					metadata: VectorMetadata::None,
				},
				CheckpointVectorSchema {
					source_index: 1,
					name: b"target".to_vec(),
					role: VectorRole::Target,
					semantic_type: SemanticType::Numeric,
					encoding: VectorEncoding::F32,
					metadata: VectorMetadata::None,
				},
			],
			feature_spans: vec![CompiledFeatureSpan::new(
				0,
				0,
				1,
				DenseFeatureLowering::NumericScalar,
			)],
			feature_width: 1,
			task: DenseTask::BinaryClassification {
				target_vector: 1,
				positive_code: 1,
			},
			output_adapter: None,
			config: DenseTrainingConfig {
				layers: vec![declaration.clone()],
				loss: DenseLoss::BinaryCrossEntropy,
				data_normalization: DenseDataNormalization::ZScore,
				batch_size: NonZeroUsize::new(1).unwrap(),
				epochs: NonZeroU64::new(1).unwrap(),
				warmup_epochs: 0,
				learning_rate_decay: LearningRateDecay::Cosine,
				gradient_clip_norm: f32::from_bits(0x3f80_0000),
				normalization_epsilon: f32::from_bits(0x3586_37bd),
				reduction_tree_lanes: 1,
				random_seed: 7,
				adamw: AdamWConfig::default(),
			},
			bounds: TrainingBounds {
				train_rows: 1,
				batch_size: 1,
				batches_per_epoch: 1,
				padded_rows_per_epoch: 1,
				epochs: NonZeroU64::new(1).unwrap(),
				training_iterations: NonZeroU64::new(1).unwrap(),
				calibration_iterations: 0,
				iterations: NonZeroU64::new(1).unwrap(),
				warmup_iterations: 0,
			},
			normalization: vec![test_tensor(1, DType::F32), test_tensor(2, DType::F32)],
			layers: vec![CheckpointLayer {
				declaration,
				weight: test_parameter(3, &[1, 1]),
				bias: test_parameter(6, &[1]),
			}],
			blocks: Vec::new(),
			temperature: None,
		}
	}

	fn projected_residual_manifest() -> CheckpointManifest {
		let mut manifest = test_manifest();
		manifest.format_version = STRUCTURED_CHECKPOINT_FORMAT_VERSION;
		manifest.config.layers.clear();
		manifest.layers.clear();
		manifest.output_adapter = Some(DenseOutputAdapter::new(
			NonZeroU64::new(2).unwrap(),
			NonZeroU64::new(1).unwrap(),
		));
		manifest.blocks = vec![
			CheckpointBlock::Residual(CheckpointResidual {
				branch: vec![
					CheckpointResidualBranch::Operation(DenseOperation::Activation(DenseActivation::Relu)),
					CheckpointResidualBranch::Layer(CheckpointLayer {
						declaration: DenseLayer::with_operations(
							NonZeroU64::new(2).unwrap(),
							[DenseOperation::Activation(DenseActivation::Silu)],
						),
						weight: test_parameter(3, &[1, 2]),
						bias: test_parameter(6, &[2]),
					}),
					CheckpointResidualBranch::Operation(DenseOperation::Activation(DenseActivation::Gelu)),
				],
				output_width: NonZeroU64::new(2).unwrap(),
				skip: CheckpointResidualSkip::Projection(test_parameter(9, &[1, 2])),
				operations: vec![DenseOperation::Normalization(crate::DenseNormalization::Layer)],
			}),
			CheckpointBlock::Layer(CheckpointLayer {
				declaration: DenseLayer::new(NonZeroU64::new(1).unwrap(), DenseActivation::Linear),
				weight: test_parameter(12, &[2, 1]),
				bias: test_parameter(15, &[1]),
			}),
		];
		manifest
	}

	fn identity_residual_manifest() -> CheckpointManifest {
		let mut manifest = test_manifest();
		manifest.format_version = STRUCTURED_CHECKPOINT_FORMAT_VERSION;
		manifest.config.layers.clear();
		manifest.layers.clear();
		manifest.blocks = vec![CheckpointBlock::Residual(CheckpointResidual {
			branch: vec![
				CheckpointResidualBranch::Operation(DenseOperation::Activation(DenseActivation::Relu)),
				CheckpointResidualBranch::Layer(CheckpointLayer {
					declaration: DenseLayer::new(NonZeroU64::new(1).unwrap(), DenseActivation::Linear),
					weight: test_parameter(3, &[1, 1]),
					bias: test_parameter(6, &[1]),
				}),
			],
			output_width: NonZeroU64::new(1).unwrap(),
			skip: CheckpointResidualSkip::Identity,
			operations: Vec::new(),
		})];
		manifest
	}

	fn encoded_projected_residual_checkpoint() -> Vec<u8> {
		let manifest = projected_residual_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();
		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();
		encoded
	}

	fn multiclass_adapter_manifest() -> CheckpointManifest {
		let mut manifest = test_manifest();
		manifest.vectors[1] = CheckpointVectorSchema {
			source_index: 1,
			name: vec![0xff, b't', b'a', b'r', b'g', b'e', b't'],
			role: VectorRole::Target,
			semantic_type: SemanticType::Categorical,
			encoding: VectorEncoding::DictionaryI32,
			metadata: VectorMetadata::Categorical {
				dictionary: vec![vec![0x00], vec![0x80], vec![0xff]],
			},
		};
		manifest.task = DenseTask::MulticlassClassification {
			target_vector: 1,
			class_count: 4,
			reserved_code: 3,
		};
		manifest.config.loss = DenseLoss::CrossEntropy;
		let source = DenseLayer::with_kind(
			DenseBlockKind::Perc,
			NonZeroU64::new(2).unwrap(),
			[
				DenseOperation::Normalization(crate::DenseNormalization::Layer),
				DenseOperation::Activation(DenseActivation::Relu),
			],
		);
		let adapter = DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Linear);
		manifest.config.layers = vec![source.clone()];
		manifest.output_adapter = Some(DenseOutputAdapter::new(
			NonZeroU64::new(2).unwrap(),
			NonZeroU64::new(4).unwrap(),
		));
		manifest.layers = vec![
			CheckpointLayer {
				declaration: source,
				weight: test_parameter(3, &[1, 2]),
				bias: test_parameter(6, &[2]),
			},
			CheckpointLayer {
				declaration: adapter,
				weight: test_parameter(9, &[2, 4]),
				bias: test_parameter(12, &[4]),
			},
		];
		manifest
	}

	fn categorical_feature_manifest() -> CheckpointManifest {
		let mut manifest = test_manifest();
		manifest.vectors[0].semantic_type = SemanticType::Categorical;
		manifest.vectors[0].encoding = VectorEncoding::DictionaryI32;
		manifest.vectors[0].metadata = VectorMetadata::Categorical {
			dictionary: vec![b"blue".to_vec(), b"red".to_vec()],
		};
		manifest.feature_spans = vec![CompiledFeatureSpan::new(
			0,
			0,
			3,
			DenseFeatureLowering::CategoricalOneHot {
				dictionary_width: 2,
				reserved_index: 2,
			},
		)];
		manifest.feature_width = 3;
		manifest.normalization = vec![
			test_tensor_with_shape(1, DType::F32, &[3]),
			test_tensor_with_shape(2, DType::F32, &[3]),
		];
		manifest.layers[0].weight = test_parameter(3, &[3, 1]);
		manifest
	}

	fn test_tensor(value: u64, dtype: DType) -> CheckpointTensor {
		CheckpointTensor {
			value: ValueId::new(value),
			dtype,
			shape: vec![1],
			bytes: 4,
		}
	}

	fn test_parameter(first_value: u64, shape: &[u64]) -> CheckpointParameter {
		CheckpointParameter {
			parameter: test_tensor_with_shape(first_value, DType::F32, shape),
			first_moment: test_tensor_with_shape(first_value + 1, DType::F32, shape),
			second_moment: test_tensor_with_shape(first_value + 2, DType::F32, shape),
		}
	}

	fn test_tensor_with_shape(value: u64, dtype: DType, shape: &[u64]) -> CheckpointTensor {
		let elements = shape.iter().product::<u64>();
		CheckpointTensor {
			value: ValueId::new(value),
			dtype,
			shape: shape.to_vec(),
			bytes: elements * 4,
		}
	}

	fn owned_outputs(manifest: &CheckpointManifest) -> BTreeMap<ValueId, Vec<u8>> {
		manifest
			.tensors()
			.map(|tensor| {
				(
					tensor.value,
					vec![0; usize::try_from(tensor.bytes).unwrap()],
				)
			})
			.collect()
	}

	fn borrowed_outputs(outputs: &BTreeMap<ValueId, Vec<u8>>) -> BTreeMap<ValueId, &[u8]> {
		outputs
			.iter()
			.map(|(value, bytes)| (*value, bytes.as_slice()))
			.collect()
	}

	pub(super) fn encoded_test_checkpoint() -> Vec<u8> {
		let manifest = test_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();
		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();
		encoded
	}

	pub(super) fn encoded_categorical_feature_test_checkpoint() -> Vec<u8> {
		let manifest = categorical_feature_manifest();
		let owned = owned_outputs(&manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();
		encode_checkpoint(&manifest, &outputs, &mut encoded).unwrap();
		encoded
	}

	fn assert_manifest_roundtrip(manifest: &CheckpointManifest) {
		let owned = owned_outputs(manifest);
		let outputs = borrowed_outputs(&owned);
		let mut encoded = Vec::new();
		encode_checkpoint(manifest, &outputs, &mut encoded).unwrap();
		let decoded = decode_checkpoint(&encoded, CheckpointDecodeLimits::default()).unwrap();
		assert_eq!(decoded.encode().unwrap(), encoded);
	}

	fn assert_decode_error(bytes: &[u8], kind: CheckpointDecodeErrorKind, path: &str) {
		assert_decode_error_with_limits(bytes, CheckpointDecodeLimits::default(), kind, path);
	}

	fn assert_decode_error_with_limits(
		bytes: &[u8],
		limits: CheckpointDecodeLimits,
		kind: CheckpointDecodeErrorKind,
		path: &str,
	) {
		let error = decode_checkpoint(bytes, limits).unwrap_err();
		let CheckpointError::Decode(error) = error else {
			panic!("expected typed decode error, got {error}");
		};
		assert_eq!(error.kind(), kind);
		assert_eq!(error.path().to_string(), path);
	}

	fn exit_outputs(manifest: &CheckpointManifest) -> Vec<ExitImage> {
		manifest
			.tensors()
			.map(|tensor| ExitImage {
				task: TaskId::new(tensor.value.get()),
				source: ResolvedValueLocation {
					value: tensor.value,
					dtype: tensor.dtype,
					device: DeviceId::new(1),
					bytes: ByteCount::new(tensor.bytes),
					object: ArenaObjectId::new(tensor.value.get()),
					object_offset: ByteOffset::new(0),
					arena_offset: ByteOffset::new(0),
				},
				bytes: vec![0; usize::try_from(tensor.bytes).unwrap()],
			})
			.collect()
	}

	fn exit_output_values(manifest: &CheckpointManifest) -> Vec<ValueId> {
		manifest.tensors().map(|tensor| tensor.value).collect()
	}

	fn assert_no_temporary(directory: &Path) {
		let temporary = fs::read_dir(directory)
			.unwrap()
			.filter_map(Result::ok)
			.any(|entry| entry.file_name().to_string_lossy().contains(".recipe-tmp-"));
		assert!(!temporary);
	}

	#[derive(Debug)]
	struct TestDirectory {
		path: PathBuf,
	}

	impl TestDirectory {
		fn create() -> Self {
			for _ in 0..64 {
				let sequence = NEXT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
				let path = std::env::temp_dir().join(format!(
					"recipe-checkpoint-test-{}-{sequence}",
					std::process::id()
				));
				match fs::create_dir(&path) {
					Ok(()) => return Self { path },
					Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
					Err(error) => panic!("create test directory {}: {error}", path.display()),
				}
			}
			panic!("could not create unique checkpoint test directory");
		}
	}

	impl Drop for TestDirectory {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.path);
		}
	}
}
