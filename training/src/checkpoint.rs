use core::fmt;
use alloc::collections::BTreeMap;
use std::{
	fs::{self, File, OpenOptions},
	io::{self, Write},
	os::unix::fs::OpenOptionsExt as _,
	path::{Path, PathBuf},
	sync::atomic::{AtomicU64, Ordering},
};

use recipe_core::{
	Block, BundleIdentity, DType, Digest, DiscoveryIdentity, EXACT_USER_RESERVATION, RealizationIdentity, RunId,
	TargetIdentity, ToolchainIdentity, TopologyIdentity, ValueId,
};
use recipe_executor::{ExitImage, RunJournal};
use recipe_ingest::{
	EncodedImageFormat, ImageColorModel, ImageValueLayout, ImageValueRange, SemanticType, VectorEncoding, VectorMetadata,
	VectorRole, VectorSchema,
};
use sha2::{Digest as _, Sha256};

use crate::{
	AdamWConfig, CompiledFeatureSpan, CompiledTraining, CompletedTrainingExecution, DataNormalization,
	DecodedMulticlassClass, DenseOutputAdapter, DenseTask, FinalTrainingMetric, LearningRateSchedule, Loss,
	NativeKernelFormat, RealizedNativeKernelSet, TrainingBounds, TrainingHorizon,
};

pub type CheckpointResult<T> = Result<T, CheckpointError>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckpointPath { segments: Vec<CheckpointPathSegment> }

impl CheckpointPath {
	#[must_use]
	pub fn segments(&self) -> &[CheckpointPathSegment] { &self.segments }

	pub(crate) fn root() -> Self { Self::default() }

	pub(crate) fn field(&self, name: impl Into<String>) -> Self {
		let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Field(name.into()));
		Self { segments }
	}

	pub(crate) fn index(&self, index: usize) -> Self {
		let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Index(index));
		Self { segments }
	}
}

impl fmt::Display for CheckpointPath {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.segments.is_empty() { return formatter.write_str("<checkpoint>"); }
		let mut separator = false;
		for segment in &self.segments {
			match segment {
				CheckpointPathSegment::Field(field) => {
					if separator { formatter.write_str(".")?; }
					formatter.write_str(field)?;
					separator = true;
				}
				CheckpointPathSegment::Index(index) => write!(formatter, "[{index}]")?,
			}
		}
		Ok(())
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointPathSegment { Field(String), Index(usize) }

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointDecodeError { kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: String }

impl CheckpointDecodeError {
	pub(crate) fn new(kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: impl Into<String>) -> Self {
		Self { kind, path, detail: detail.into() }
	}

	#[must_use]
	pub const fn kind(&self) -> CheckpointDecodeErrorKind { self.kind }

	#[must_use]
	pub const fn path(&self) -> &CheckpointPath { &self.path }

	#[must_use]
	pub fn detail(&self) -> &str { &self.detail }
}

impl fmt::Display for CheckpointDecodeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { write!(formatter, "{}: {}", self.path, self.detail) }
}

impl core::error::Error for CheckpointDecodeError {}

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
		Self { source_bytes: 1 << 30, nodes: 4_000_000, vectors: 0x0001_0000, feature_spans: 0x0001_0000,
			layers: 0x4000, metadata_entries: 1_000_000, tensors: 100_000, tensor_rank: 16, tensor_bytes: 1 << 30,
			total_payload_bytes: 1 << 30 }
	}
}

#[derive(Debug)]
#[non_exhaustive]
pub enum CheckpointError {
	Decode(CheckpointDecodeError),
	InvalidManifest { detail: String },
	IncompatibleResume { detail: String },
	DuplicateOutput { value: ValueId },
	MissingOutput { value: ValueId },
	UnexpectedOutput { value: ValueId },
	OutputDTypeMismatch { value: ValueId, expected: DType, actual: DType },
	OutputSizeMismatch { value: ValueId, expected: u64, actual: u64 },
	NativeKernelUnavailable { requested: NativeKernelFormat },
	NativeKernelAmbiguous { requested: NativeKernelFormat, images: usize },
	InvalidTarget { path: PathBuf, detail: String },
	InsufficientCapacity { path: PathBuf, available: u64, checkpoint_allocation: u64, reservation: u64 },
	Io { operation: &'static str, path: PathBuf, source: io::Error },
}

impl CheckpointError {
	pub(crate) fn manifest(detail: impl Into<String>) -> Self { Self::InvalidManifest { detail: detail.into() } }

	pub(crate) fn invalid_target(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self {
		Self::InvalidTarget { path: path.into(), detail: detail.into() }
	}

	fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
		Self::Io { operation, path: path.into(), source }
	}
}

impl fmt::Display for CheckpointError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Decode(error) => write!(formatter, "decode checkpoint: {error}"),
			Self::InvalidManifest { detail } => write!(formatter, "invalid checkpoint manifest: {detail}"),
			Self::IncompatibleResume { detail } => write!(formatter, "incompatible training resume: {detail}"),
			Self::DuplicateOutput { value } => write!(formatter, "checkpoint output {value} appears more than once"),
			Self::MissingOutput { value } => write!(formatter, "checkpoint output {value} is absent"),
			Self::UnexpectedOutput { value } => write!(formatter, "execution returned unexpected checkpoint output {value}"),
			Self::OutputDTypeMismatch { value, expected, actual } => {
				write!(formatter, "checkpoint output {value} has dtype {actual:?}, expected {expected:?}")
			}
			Self::OutputSizeMismatch { value, expected, actual } => {
				write!(formatter, "checkpoint output {value} has {actual} bytes, expected {expected}")
			}
			Self::NativeKernelUnavailable { requested } => {
				write!(formatter, "the realized execution contains no .{} native kernel image", requested.extension())
			}
			Self::NativeKernelAmbiguous { requested, images } => write!(formatter,
				"the realized execution contains {images} distinct .{} images; one native file cannot represent them",
				requested.extension()),
			Self::InvalidTarget { path, detail } => write!(formatter, "invalid checkpoint target {}: {detail}", path.display()),
			Self::InsufficientCapacity { path, available, checkpoint_allocation, reservation } => write!(formatter,
				"checkpoint target {} has {available} available bytes; writing needs {checkpoint_allocation} bytes while preserving {reservation} bytes",
				path.display()),
			Self::Io { operation, path, source } => write!(formatter, "{operation} {}: {source}", path.display()),
		}
	}
}

impl core::error::Error for CheckpointError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		match self { Self::Decode(error) => Some(error), Self::Io { source, .. } => Some(source), _ => None }
	}
}

impl From<CheckpointDecodeError> for CheckpointError {
	fn from(error: CheckpointDecodeError) -> Self { Self::Decode(error) }
}

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
	pub const fn source_index(&self) -> usize { self.source_index }
	#[must_use]
	pub fn name(&self) -> &[u8] { &self.name }
	#[must_use]
	pub const fn role(&self) -> VectorRole { self.role }
	#[must_use]
	pub const fn semantic_type(&self) -> SemanticType { self.semantic_type }
	#[must_use]
	pub const fn encoding(&self) -> VectorEncoding { self.encoding }
	#[must_use]
	pub const fn dtype(&self) -> Option<DType> { self.encoding.dtype() }
	#[must_use]
	pub const fn metadata(&self) -> &VectorMetadata { &self.metadata }
}

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
	pub(crate) const fn new(format: EncodedImageFormat, width: u32, height: u32, channels: Option<u8>,
		color_model: Option<ImageColorModel>, sample_bits: Option<u8>) -> Self {
		Self { format, width, height, channels, color_model, sample_bits, value_layout: ImageValueLayout::EncodedFile,
			value_range: ImageValueRange::EncodedBytes }
	}
	#[must_use]
	pub const fn format(self) -> EncodedImageFormat { self.format }
	#[must_use]
	pub const fn width(self) -> u32 { self.width }
	#[must_use]
	pub const fn height(self) -> u32 { self.height }
	#[must_use]
	pub const fn channels(self) -> Option<u8> { self.channels }
	#[must_use]
	pub const fn color_model(self) -> Option<ImageColorModel> { self.color_model }
	#[must_use]
	pub const fn sample_bits(self) -> Option<u8> { self.sample_bits }
	#[must_use]
	pub const fn value_layout(self) -> ImageValueLayout { self.value_layout }
	#[must_use]
	pub const fn value_range(self) -> ImageValueRange { self.value_range }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointArtifactMetadata {
	None,
	Temporal { unix_seconds: i64, nanoseconds: u32 },
	Categorical { dictionary: Vec<Vec<u8>> },
	Ordinal { ordered_labels: Vec<Vec<u8>> },
	Image { encoded_variants: Vec<CheckpointImageMetadata> },
}

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
	pub(crate) fn new(source_index: usize, name: Vec<u8>, role: VectorRole, semantic_type: SemanticType,
		encoding: VectorEncoding, metadata: CheckpointArtifactMetadata) -> Self {
		Self { source_index, name, role, semantic_type, encoding, metadata }
	}

	pub(crate) fn from_schema(schema: &VectorSchema) -> Self {
		Self { source_index: schema.source_index(), name: schema.name().to_vec(), role: schema.role(),
			semantic_type: schema.semantic_type(), encoding: schema.encoding(), metadata: artifact_metadata(schema.metadata()) }
	}

	#[must_use]
	pub const fn source_index(&self) -> usize { self.source_index }
	#[must_use]
	pub fn name(&self) -> &[u8] { &self.name }
	#[must_use]
	pub const fn role(&self) -> VectorRole { self.role }
	#[must_use]
	pub const fn semantic_type(&self) -> SemanticType { self.semantic_type }
	#[must_use]
	pub const fn encoding(&self) -> VectorEncoding { self.encoding }
	#[must_use]
	pub const fn dtype(&self) -> Option<DType> { self.encoding.dtype() }
	#[must_use]
	pub const fn metadata(&self) -> &CheckpointArtifactMetadata { &self.metadata }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointTensorImage { dtype: DType, shape: Vec<u64>, bytes: Vec<u8> }

impl CheckpointTensorImage {
	#[must_use]
	pub const fn dtype(&self) -> DType { self.dtype }
	#[must_use]
	pub fn shape(&self) -> &[u64] { &self.shape }
	#[must_use]
	pub fn bytes(&self) -> &[u8] { &self.bytes }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointParameterImage {
	parameter: CheckpointTensorImage,
	first_moment: CheckpointTensorImage,
	second_moment: CheckpointTensorImage,
}

impl CheckpointParameterImage {
	#[must_use]
	pub const fn parameter(&self) -> &CheckpointTensorImage { &self.parameter }
	#[must_use]
	pub const fn first_moment(&self) -> &CheckpointTensorImage { &self.first_moment }
	#[must_use]
	pub const fn second_moment(&self) -> &CheckpointTensorImage { &self.second_moment }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointNativeKernel {
	format: NativeKernelFormat,
	target: TargetIdentity,
	toolchain: ToolchainIdentity,
	digest: Digest,
}

impl CheckpointNativeKernel {
	#[must_use]
	pub const fn format(&self) -> NativeKernelFormat { self.format }
	#[must_use]
	pub const fn target(&self) -> &TargetIdentity { &self.target }
	#[must_use]
	pub const fn toolchain(&self) -> &ToolchainIdentity { &self.toolchain }
	#[must_use]
	pub const fn digest(&self) -> Digest { self.digest }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointNativeRealization {
	program: Digest,
	realization: RealizationIdentity,
	topology: TopologyIdentity,
	discovery: DiscoveryIdentity,
	kernels: Vec<CheckpointNativeKernel>,
}

impl CheckpointNativeRealization {
	#[must_use]
	pub const fn program(&self) -> Digest { self.program }
	#[must_use]
	pub const fn realization(&self) -> RealizationIdentity { self.realization }
	#[must_use]
	pub const fn topology(&self) -> TopologyIdentity { self.topology }
	#[must_use]
	pub const fn discovery(&self) -> DiscoveryIdentity { self.discovery }
	#[must_use]
	pub fn kernels(&self) -> &[CheckpointNativeKernel] { &self.kernels }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointArtifact {
	format_version: u32,
	vectors: Vec<CheckpointArtifactVector>,
	feature_spans: Vec<CompiledFeatureSpan>,
	feature_normalization_mask: Vec<u32>,
	feature_width: usize,
	target_source_indices: Vec<usize>,
	task: DenseTask,
	output_adapter: Option<DenseOutputAdapter>,
	loss: Loss,
	data_normalization: DataNormalization,
	epochs: TrainingHorizon,
	warmup_epochs: u64,
	learning_rate_schedule: LearningRateSchedule,
	gradient_clip_norm: Option<f32>,
	normalization_epsilon: f32,
	reduction_tree_lanes: u32,
	random_seed: u64,
	adamw: AdamWConfig,
	bounds: TrainingBounds,
	blocks: Vec<Block>,
	parameters: Vec<CheckpointParameterImage>,
	state: Vec<CheckpointTensorImage>,
	temperature: Option<CheckpointTensorImage>,
	native: Option<CheckpointNativeRealization>,
}

impl CheckpointArtifact {
	#[must_use]
	pub const fn format_version(&self) -> u32 { self.format_version }
	#[must_use]
	pub fn vectors(&self) -> &[CheckpointArtifactVector] { &self.vectors }
	#[must_use]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] { &self.feature_spans }
	#[must_use]
	pub fn feature_normalization_mask(&self) -> &[u32] { &self.feature_normalization_mask }
	#[must_use]
	pub const fn feature_width(&self) -> usize { self.feature_width }
	#[must_use]
	pub fn target_source_indices(&self) -> &[usize] { &self.target_source_indices }
	#[must_use]
	pub const fn task(&self) -> DenseTask { self.task }
	#[must_use]
	pub fn target_dtype(&self) -> Option<DType> { self.target_dtypes().next() }
	#[must_use]
	pub fn target_dtypes(&self) -> impl Iterator<Item = DType> + '_ {
		self.target_source_indices.iter().filter_map(|source_index| self.vectors.iter()
			.find(|vector| vector.source_index == *source_index && vector.role == VectorRole::Target)
			.and_then(CheckpointArtifactVector::dtype))
	}
	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> { self.output_adapter }
	#[must_use]
	pub const fn data_normalization(&self) -> DataNormalization { self.data_normalization }
	#[must_use]
	pub const fn normalization_epsilon(&self) -> f32 { self.normalization_epsilon }
	#[must_use]
	pub const fn reduction_tree_lanes(&self) -> u32 { self.reduction_tree_lanes }
	#[must_use]
	pub const fn bounds(&self) -> TrainingBounds { self.bounds }
	#[must_use]
	pub fn normalization(&self) -> &[CheckpointTensorImage] { &self.state }
	#[must_use]
	pub fn blocks(&self) -> &[Block] { &self.blocks }
	#[must_use]
	pub fn parameters(&self) -> &[CheckpointParameterImage] { &self.parameters }
	#[must_use]
	pub const fn temperature(&self) -> Option<&CheckpointTensorImage> { self.temperature.as_ref() }
	#[must_use]
	pub const fn native_realization(&self) -> Option<&CheckpointNativeRealization> { self.native.as_ref() }
	#[must_use]
	pub fn decode_multiclass_class(&self, class: usize) -> Option<DecodedMulticlassClass<'_>> {
		let DenseTask::MulticlassClassification { target_vector, class_count, reserved_code } = self.task else { return None };
		if class >= class_count { return None; }
		let target = self.vectors.iter().find(|vector| vector.source_index == target_vector && vector.role == VectorRole::Target)?;
		let CheckpointArtifactMetadata::Categorical { dictionary } = &target.metadata else { return None };
		if i32::try_from(class).ok() == Some(reserved_code) { return Some(DecodedMulticlassClass::ReservedUnseen); }
		dictionary.get(class).map(|label| DecodedMulticlassClass::Label(label))
	}
	pub fn encode(&self) -> CheckpointResult<Vec<u8>> { todo!("unify checkpoint encoding on Block") }
}

pub fn decode_checkpoint(_bytes: &[u8], _limits: CheckpointDecodeLimits) -> CheckpointResult<CheckpointArtifact> {
	todo!("unify checkpoint decoding on Block")
}

pub(crate) fn decode_error(kind: CheckpointDecodeErrorKind, path: CheckpointPath,
	detail: impl Into<String>) -> CheckpointError {
	CheckpointDecodeError::new(kind, path, detail).into()
}

pub(crate) fn validate_saved_vector(_vector: &CheckpointArtifactVector,
	_path: &CheckpointPath) -> CheckpointResult<()> { Ok(()) }

#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointManifest {
	blocks: Vec<Block>,
	program_digest: Digest,
	native: Option<CheckpointNativeRealization>,
}

impl CheckpointManifest {
	pub fn from_compiled(_training: &CompiledTraining) -> CheckpointResult<Self> {
		todo!("unify: was CheckpointBlock, now Block")
	}

	#[must_use]
	pub fn blocks(&self) -> &[Block] { &self.blocks }
}

pub fn compiled_training_program_digest(training: &CompiledTraining) -> CheckpointResult<Digest> {
	let encoded = training.program().to_ogdl()
		.map_err(|error| CheckpointError::manifest(format!("encode canonical training program: {error}")))?;
	Ok(Digest::new(Sha256::digest(encoded.as_bytes()).into()))
}

pub fn apply_checkpoint_resume(_training: &mut CompiledTraining,
	_checkpoint: &CheckpointArtifact) -> CheckpointResult<()> {
	todo!("unify checkpoint resume on Block")
}

#[derive(Clone, Debug)]
pub struct CompletedTrainingCheckpoint {
	execution: CompletedTrainingExecution,
	manifest: CheckpointManifest,
	output_indices: BTreeMap<ValueId, usize>,
}

impl CompletedTrainingCheckpoint {
	pub fn new(execution: CompletedTrainingExecution, mut manifest: CheckpointManifest) -> CheckpointResult<Self> {
		manifest.native = Some(checkpoint_native_realization(execution.native_kernels(), manifest.program_digest)?);
		Ok(Self { execution, manifest, output_indices: BTreeMap::new() })
	}
	#[must_use]
	pub const fn run(&self) -> RunId { self.execution.run() }
	#[must_use]
	pub const fn bundle(&self) -> BundleIdentity { self.execution.bundle() }
	#[must_use]
	pub fn external_outputs(&self) -> &[ExitImage] { self.execution.external_outputs() }
	#[must_use]
	pub fn metrics(&self) -> &[FinalTrainingMetric] { self.execution.metrics() }
	#[must_use]
	pub const fn native_kernels(&self) -> &RealizedNativeKernelSet { self.execution.native_kernels() }
	#[must_use]
	pub const fn native_evidence(&self) -> &recipe_native_executor::NativeExecutionEvidence { self.execution.native_evidence() }
	#[must_use]
	pub const fn training_evidence(&self) -> &crate::TrainingExecutionEvidence { self.execution.training_evidence() }
	#[must_use]
	pub const fn journal(&self) -> &RunJournal { self.execution.journal() }
	#[must_use]
	pub const fn manifest(&self) -> &CheckpointManifest { &self.manifest }
	pub fn save(&self, _path: impl AsRef<Path>) -> CheckpointResult<()> {
		let _ = &self.output_indices;
		todo!("unify checkpoint saving on Block")
	}
	pub fn save_native_kernel(&self, path: impl AsRef<Path>, format: NativeKernelFormat) -> CheckpointResult<()> {
		let path = path.as_ref();
		if path.extension().and_then(|extension| extension.to_str()) != Some(format.extension()) {
			return Err(CheckpointError::invalid_target(path, format!("native kernel path must end in .{}", format.extension())));
		}
		let matching = self.native_kernels().kernels().iter().filter(|kernel| kernel.format() == format).collect::<Vec<_>>();
		let kernel = match matching.as_slice() {
			[] => return Err(CheckpointError::NativeKernelUnavailable { requested: format }),
			[kernel] => *kernel,
			kernels => return Err(CheckpointError::NativeKernelAmbiguous { requested: format, images: kernels.len() }),
		};
		let bytes = u64::try_from(kernel.bytes().len())
			.map_err(|_| CheckpointError::invalid_target(path, "native kernel size exceeds u64"))?;
		atomic_save(path, bytes, |file| file.write_all(kernel.bytes()))
	}
}

fn checkpoint_native_realization(realized: &RealizedNativeKernelSet,
	program: Digest) -> CheckpointResult<CheckpointNativeRealization> {
	let kernels = realized.kernels().iter().map(|kernel| CheckpointNativeKernel { format: kernel.format(),
		target: kernel.target().clone(), toolchain: kernel.toolchain().clone(), digest: kernel.digest() }).collect::<Vec<_>>();
	if kernels.is_empty() { return Err(CheckpointError::manifest("completed native realization contains no kernel images")); }
	Ok(CheckpointNativeRealization { program, realization: realized.realization(), topology: realized.topology(),
		discovery: realized.discovery(), kernels })
}

pub(crate) fn atomic_save(target: &Path, encoded_bytes: u64,
	write_checkpoint: impl FnOnce(&mut File) -> io::Result<()>) -> CheckpointResult<()> {
	let parent = target.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
	let statistics = rustix::fs::statvfs(parent)
		.map_err(|error| CheckpointError::io("query filesystem capacity", parent, error.into()))?;
	let block_size = statistics.f_frsize;
	let allocation = encoded_bytes.checked_add(block_size.saturating_sub(1)).and_then(|bytes| bytes.checked_div(block_size))
		.and_then(|blocks| blocks.checked_mul(block_size))
		.ok_or_else(|| CheckpointError::invalid_target(target, "checkpoint allocation size overflowed"))?;
	let available = statistics.f_bavail.saturating_mul(block_size);
	let reservation = EXACT_USER_RESERVATION.get();
	if available < allocation.saturating_add(reservation) {
		return Err(CheckpointError::InsufficientCapacity { path: target.to_path_buf(), available,
			checkpoint_allocation: allocation, reservation });
	}
	static NEXT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);
	let sequence = NEXT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
	let name = format!(".{}.recipe-{sequence}.tmp", target.file_name().and_then(|name| name.to_str()).unwrap_or("model"));
	let temporary_path = parent.join(name);
	let mut temporary = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary_path)
		.map_err(|error| CheckpointError::io("create checkpoint temporary", &temporary_path, error))?;
	if let Err(error) = write_checkpoint(&mut temporary).and_then(|_| temporary.flush()).and_then(|_| temporary.sync_all()) {
		let _ = fs::remove_file(&temporary_path);
		return Err(CheckpointError::io("write checkpoint temporary", &temporary_path, error));
	}
	drop(temporary);
	fs::rename(&temporary_path, target).map_err(|error| CheckpointError::io("atomically install checkpoint", target, error))
}

fn artifact_metadata(metadata: &VectorMetadata) -> CheckpointArtifactMetadata {
	match metadata {
		VectorMetadata::None => CheckpointArtifactMetadata::None,
		VectorMetadata::Temporal { origin } =>
			CheckpointArtifactMetadata::Temporal { unix_seconds: origin.unix_seconds, nanoseconds: origin.nanoseconds },
		VectorMetadata::Categorical { dictionary } =>
			CheckpointArtifactMetadata::Categorical { dictionary: dictionary.clone() },
		VectorMetadata::Ordinal { ordered_labels } =>
			CheckpointArtifactMetadata::Ordinal { ordered_labels: ordered_labels.clone() },
		VectorMetadata::Image { encoded_variants } => CheckpointArtifactMetadata::Image { encoded_variants: encoded_variants
			.iter().map(|variant| CheckpointImageMetadata::new(variant.format(), variant.width(), variant.height(),
				variant.channels(), variant.color_model(), variant.sample_bits())).collect() },
	}
}
