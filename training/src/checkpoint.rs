use core::fmt;
use std::{
	fs::{self, File, OpenOptions},
	io::{self, Write as _},
	os::unix::fs::OpenOptionsExt as _,
	path::{Path, PathBuf},
	sync::atomic::{AtomicU64, Ordering},
};

use recipe_core::{Block, BundleIdentity, DType, Digest, DiscoveryIdentity, EXACT_USER_RESERVATION, RealizationIdentity, RunId, TargetIdentity, ToolchainIdentity, TopologyIdentity, ValueId};
use recipe_executor::{ExitImage, RunJournal};
use recipe_ingest::{VectorRole, VectorSchema};
use sha2::{Digest as _, Sha256};

use crate::{AdamWConfig, CompiledFeatureSpan, CompiledTraining, CompletedTrainingExecution, DataNormalization, DecodedMulticlassClass, DenseOutputAdapter, DenseTask, FinalTrainingMetric, LearningRateSchedule, Loss, NativeKernelFormat, RealizedNativeKernelSet, TrainingBounds, TrainingHorizon};

pub type CheckpointResult<T> = Result<T, CheckpointError>;

static NEXT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckpointPath {
	segments: Vec<CheckpointPathSegment>,
}

impl CheckpointPath {
	#[must_use]
	pub fn segments(&self) -> &[CheckpointPathSegment] { return &self.segments; }

	pub(crate) fn root() -> Self { return Self::default(); }

	pub(crate) fn field(&self, name: impl Into<String>) -> Self {
		let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Field(name.into()));
		return Self { segments };
	}

	pub(crate) fn index(&self, index: usize) -> Self {
		let mut segments = self.segments.clone();
		segments.push(CheckpointPathSegment::Index(index));
		return Self { segments };
	}
}

impl fmt::Display for CheckpointPath {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.segments.is_empty() {
			return f.write_str("<checkpoint>");
		}
		let mut separator = false;
		for segment in &self.segments {
			match segment {
				CheckpointPathSegment::Field(field) => {
					if separator {
						f.write_str(".")?;
					}
					f.write_str(field)?;
					separator = true;
				}
				CheckpointPathSegment::Index(index) => write!(f, "[{index}]")?,
			}
		}
		return Ok(());
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointPathSegment {
	Field(String),
	Index(usize),
}

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
pub struct CheckpointDecodeError {
	kind: CheckpointDecodeErrorKind,
	path: CheckpointPath,
	detail: String,
}

impl CheckpointDecodeError {
	pub(crate) fn new(kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: impl Into<String>) -> Self {
		return Self {
			kind,
			path,
			detail: detail.into(),
		};
	}

	#[must_use]
	pub const fn kind(&self) -> CheckpointDecodeErrorKind { return self.kind; }

	#[must_use]
	pub const fn path(&self) -> &CheckpointPath { return &self.path; }

	#[must_use]
	pub fn detail(&self) -> &str { return &self.detail; }
}

impl fmt::Display for CheckpointDecodeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { return write!(f, "{}: {}", self.path, self.detail); }
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
		return Self {
			source_bytes: 1 << 30,
			nodes: 4_000_000,
			vectors: 0x0001_0000,
			feature_spans: 0x0001_0000,
			layers: 0x4000,
			metadata_entries: 1_000_000,
			tensors: 100_000,
			tensor_rank: 16,
			tensor_bytes: 1 << 30,
			total_payload_bytes: 1 << 30,
		};
	}
}

#[derive(Debug)]
#[non_exhaustive]
pub enum CheckpointError {
	Decode(CheckpointDecodeError),
	InvalidManifest {
		detail: String,
	},
	IncompatibleResume {
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
	NativeKernelUnavailable {
		requested: NativeKernelFormat,
	},
	NativeKernelAmbiguous {
		requested: NativeKernelFormat,
		images: usize,
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
	pub(crate) fn manifest(detail: impl Into<String>) -> Self {
		return Self::InvalidManifest {
			detail: detail.into(),
		};
	}

	pub(crate) fn invalid_target(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self {
		return Self::InvalidTarget {
			path: path.into(),
			detail: detail.into(),
		};
	}

	fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
		return Self::Io {
			operation,
			path: path.into(),
			source,
		};
	}
}

impl fmt::Display for CheckpointError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Decode(error) => return write!(f, "decode checkpoint: {error}"),
			Self::InvalidManifest { detail } => {
				return write!(f, "invalid checkpoint manifest: {detail}");
			}
			Self::IncompatibleResume { detail } => {
				return write!(f, "incompatible training resume: {detail}");
			}
			Self::DuplicateOutput { value } => {
				return write!(f, "checkpoint output {value} appears more than once");
			}
			Self::MissingOutput { value } => return write!(f, "checkpoint output {value} is absent"),
			Self::UnexpectedOutput { value } => {
				return write!(f, "execution returned unexpected checkpoint output {value}");
			}
			Self::OutputDTypeMismatch {
				value,
				expected,
				actual,
			} => {
				return write!(
					f,
					"checkpoint output {value} has dtype {actual}, expected {expected}"
				);
			}
			Self::OutputSizeMismatch {
				value,
				expected,
				actual,
			} => {
				return write!(
					f,
					"checkpoint output {value} has {actual} bytes, expected {expected}"
				);
			}
			Self::NativeKernelUnavailable { requested } => {
				return write!(
					f,
					"the realized execution contains no .{} native kernel image",
					requested.extension()
				);
			}
			Self::NativeKernelAmbiguous { requested, images } => {
				return write!(
					f,
					"the realized execution contains {images} distinct .{} images; one native file cannot represent them",
					requested.extension()
				);
			}
			Self::InvalidTarget { path, detail } => {
				return write!(f, "invalid checkpoint target {}: {detail}", path.display());
			}
			Self::InsufficientCapacity {
				path,
				available,
				checkpoint_allocation,
				reservation,
			} => {
				return write!(
					f,
					"checkpoint target {} has {available} available bytes; writing needs {checkpoint_allocation} bytes while preserving {reservation} bytes",
					path.display()
				);
			}
			Self::Io {
				operation,
				path,
				source,
			} => return write!(f, "{operation} {}: {source}", path.display()),
		}
	}
}

impl core::error::Error for CheckpointError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		match self {
			Self::Decode(error) => return Some(error),
			Self::Io { source, .. } => return Some(source),
			Self::InvalidManifest { .. } | Self::IncompatibleResume { .. } | Self::DuplicateOutput { .. } | Self::MissingOutput { .. } | Self::UnexpectedOutput { .. } | Self::OutputDTypeMismatch { .. } | Self::OutputSizeMismatch { .. } | Self::NativeKernelUnavailable { .. } | Self::NativeKernelAmbiguous { .. } | Self::InvalidTarget { .. } | Self::InsufficientCapacity { .. } => return None,
		}
	}
}

impl From<CheckpointDecodeError> for CheckpointError {
	fn from(error: CheckpointDecodeError) -> Self { return Self::Decode(error); }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointTensorImage {
	dtype: DType,
	shape: Vec<u64>,
	bytes: Vec<u8>,
}

impl CheckpointTensorImage {
	#[must_use]
	pub const fn dtype(&self) -> DType { return self.dtype; }
	#[must_use]
	pub fn shape(&self) -> &[u64] { return &self.shape; }
	#[must_use]
	pub fn bytes(&self) -> &[u8] { return &self.bytes; }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointParameterImage {
	parameter: CheckpointTensorImage,
	first_moment: CheckpointTensorImage,
	second_moment: CheckpointTensorImage,
}

impl CheckpointParameterImage {
	#[must_use]
	pub const fn parameter(&self) -> &CheckpointTensorImage { return &self.parameter; }
	#[must_use]
	pub const fn first_moment(&self) -> &CheckpointTensorImage { return &self.first_moment; }
	#[must_use]
	pub const fn second_moment(&self) -> &CheckpointTensorImage { return &self.second_moment; }
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
	pub const fn format(&self) -> NativeKernelFormat { return self.format; }
	#[must_use]
	pub const fn target(&self) -> &TargetIdentity { return &self.target; }
	#[must_use]
	pub const fn toolchain(&self) -> &ToolchainIdentity { return &self.toolchain; }
	#[must_use]
	pub const fn digest(&self) -> Digest { return self.digest; }
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
	pub const fn program(&self) -> Digest { return self.program; }
	#[must_use]
	pub const fn realization(&self) -> RealizationIdentity { return self.realization; }
	#[must_use]
	pub const fn topology(&self) -> TopologyIdentity { return self.topology; }
	#[must_use]
	pub const fn discovery(&self) -> DiscoveryIdentity { return self.discovery; }
	#[must_use]
	pub fn kernels(&self) -> &[CheckpointNativeKernel] { return &self.kernels; }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointArtifact {
	format_version: u32,
	vectors: Vec<VectorSchema>,
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
	pub const fn format_version(&self) -> u32 { return self.format_version; }
	#[must_use]
	pub fn vectors(&self) -> &[VectorSchema] { return &self.vectors; }
	#[must_use]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] { return &self.feature_spans; }
	#[must_use]
	pub fn feature_normalization_mask(&self) -> &[u32] { return &self.feature_normalization_mask; }
	#[must_use]
	pub const fn feature_width(&self) -> usize { return self.feature_width; }
	#[must_use]
	pub fn target_source_indices(&self) -> &[usize] { return &self.target_source_indices; }
	#[must_use]
	pub const fn task(&self) -> DenseTask { return self.task; }
	#[must_use]
	pub fn target_dtype(&self) -> Option<DType> { return self.target_dtypes().next(); }
	pub fn target_dtypes(&self) -> impl Iterator<Item = DType> + '_ {
		return self
			.target_source_indices
			.iter()
			.filter_map(|source_index| {
				return self
					.vectors
					.iter()
					.find(|vector| {
						return vector.source_index() == *source_index && vector.role() == VectorRole::Target;
					})
					.and_then(|vector| return vector.encoding().dtype());
			});
	}
	#[must_use]
	pub const fn output_adapter(&self) -> Option<DenseOutputAdapter> { return self.output_adapter; }
	#[must_use]
	pub const fn data_normalization(&self) -> DataNormalization { return self.data_normalization; }
	#[must_use]
	pub const fn normalization_epsilon(&self) -> f32 { return self.normalization_epsilon; }
	#[must_use]
	pub const fn reduction_tree_lanes(&self) -> u32 { return self.reduction_tree_lanes; }
	#[must_use]
	pub const fn bounds(&self) -> TrainingBounds { return self.bounds; }
	#[must_use]
	pub fn normalization(&self) -> &[CheckpointTensorImage] { return &self.state; }
	#[must_use]
	pub fn blocks(&self) -> &[Block] { return &self.blocks; }
	#[must_use]
	pub fn parameters(&self) -> &[CheckpointParameterImage] { return &self.parameters; }
	#[must_use]
	pub const fn temperature(&self) -> Option<&CheckpointTensorImage> { return self.temperature.as_ref(); }
	#[must_use]
	pub const fn native_realization(&self) -> Option<&CheckpointNativeRealization> { return self.native.as_ref(); }
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
		let target = self.vectors.iter().find(|vector| {
			return vector.source_index() == target_vector && vector.role() == VectorRole::Target;
		})?;
		let dictionary = target.metadata().categorical_dictionary()?;
		if i32::try_from(class).ok() == Some(reserved_code) {
			return Some(DecodedMulticlassClass::ReservedUnseen);
		}
		return dictionary
			.get(class)
			.map(|label| return DecodedMulticlassClass::Label(label));
	}
	pub fn encode(&self) -> CheckpointResult<Vec<u8>> { todo!("unify checkpoint encoding on Block") }
}

pub fn decode_checkpoint(_bytes: &[u8], _limits: CheckpointDecodeLimits) -> CheckpointResult<CheckpointArtifact> { todo!("unify checkpoint decoding on Block") }

pub fn decode_error(kind: CheckpointDecodeErrorKind, path: CheckpointPath, detail: impl Into<String>) -> CheckpointError { return CheckpointDecodeError::new(kind, path, detail).into(); }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointManifest {
	blocks: Vec<Block>,
	program_digest: Digest,
	native: Option<CheckpointNativeRealization>,
}

impl CheckpointManifest {
	pub fn from_compiled(_training: &CompiledTraining) -> CheckpointResult<Self> { todo!("unify: was CheckpointBlock, now Block") }

	#[must_use]
	pub fn blocks(&self) -> &[Block] { return &self.blocks; }
}

pub fn compiled_training_program_digest(training: &CompiledTraining) -> CheckpointResult<Digest> {
	let encoded = training
		.program()
		.to_ogdl()
		.map_err(|error| return CheckpointError::manifest(format!("encode canonical training program: {error}")))?;
	return Ok(Digest::new(Sha256::digest(encoded.as_bytes()).into()));
}

pub fn apply_checkpoint_resume(_training: &mut CompiledTraining, _checkpoint: &CheckpointArtifact) -> CheckpointResult<()> { todo!("unify checkpoint resume on Block") }

#[derive(Clone, Debug)]
pub struct CompletedTrainingCheckpoint {
	execution: CompletedTrainingExecution,
	manifest: CheckpointManifest,
}

impl CompletedTrainingCheckpoint {
	pub fn new(execution: CompletedTrainingExecution, mut manifest: CheckpointManifest) -> CheckpointResult<Self> {
		manifest.native = Some(checkpoint_native_realization(
			execution.native_kernels(),
			manifest.program_digest,
		)?);
		return Ok(Self {
			execution,
			manifest,
		});
	}
	#[must_use]
	pub const fn run(&self) -> RunId { return self.execution.run(); }
	#[must_use]
	pub const fn bundle(&self) -> BundleIdentity { return self.execution.bundle(); }
	#[must_use]
	pub fn external_outputs(&self) -> &[ExitImage] { return self.execution.external_outputs(); }
	#[must_use]
	pub fn metrics(&self) -> &[FinalTrainingMetric] { return self.execution.metrics(); }
	#[must_use]
	pub const fn native_kernels(&self) -> &RealizedNativeKernelSet { return self.execution.native_kernels(); }
	#[must_use]
	pub const fn native_evidence(&self) -> &recipe_native_executor::NativeExecutionEvidence { return self.execution.native_evidence(); }
	#[must_use]
	pub const fn training_evidence(&self) -> &crate::TrainingExecutionEvidence { return self.execution.training_evidence(); }
	#[must_use]
	pub const fn journal(&self) -> &RunJournal { return self.execution.journal(); }
	#[must_use]
	pub const fn manifest(&self) -> &CheckpointManifest { return &self.manifest; }
	pub fn save(&self, _path: impl AsRef<Path>) -> CheckpointResult<()> { todo!("unify checkpoint saving on Block") }
	pub fn save_native_kernel(&self, path: impl AsRef<Path>, format: NativeKernelFormat) -> CheckpointResult<()> {
		let path = path.as_ref();
		if path
			.extension()
			.and_then(|extension| return extension.to_str())
			!= Some(format.extension())
		{
			return Err(CheckpointError::invalid_target(
				path,
				format!("native kernel path must end in .{}", format.extension()),
			));
		}
		let matching = self
			.native_kernels()
			.kernels()
			.iter()
			.filter(|kernel| return kernel.format() == format)
			.collect::<Vec<_>>();
		let kernel = match matching.as_slice() {
			[] => return Err(CheckpointError::NativeKernelUnavailable { requested: format }),
			[kernel] => *kernel,
			kernels => {
				return Err(CheckpointError::NativeKernelAmbiguous {
					requested: format,
					images: kernels.len(),
				});
			}
		};
		let bytes = u64::try_from(kernel.bytes().len()).map_err(|_error| return CheckpointError::invalid_target(path, "native kernel size exceeds u64"))?;
		return atomic_save(path, bytes, |file| return file.write_all(kernel.bytes()));
	}
}

fn checkpoint_native_realization(realized: &RealizedNativeKernelSet, program: Digest) -> CheckpointResult<CheckpointNativeRealization> {
	let kernels = realized
		.kernels()
		.iter()
		.map(|kernel| {
			return CheckpointNativeKernel {
				format: kernel.format(),
				target: kernel.target().clone(),
				toolchain: kernel.toolchain().clone(),
				digest: kernel.digest(),
			};
		})
		.collect::<Vec<_>>();
	if kernels.is_empty() {
		return Err(CheckpointError::manifest(
			"completed native realization contains no kernel images",
		));
	}
	return Ok(CheckpointNativeRealization {
		program,
		realization: realized.realization(),
		topology: realized.topology(),
		discovery: realized.discovery(),
		kernels,
	});
}

pub fn atomic_save(target: &Path, encoded_bytes: u64, write_checkpoint: impl FnOnce(&mut File) -> io::Result<()>) -> CheckpointResult<()> {
	let parent = target
		.parent()
		.filter(|path| return !path.as_os_str().is_empty())
		.unwrap_or_else(|| return Path::new("."));
	let statistics = rustix::fs::statvfs(parent).map_err(|error| return CheckpointError::io("query filesystem capacity", parent, error.into()))?;
	let block_size = statistics.f_frsize;
	let allocation = encoded_bytes
		.checked_add(block_size.saturating_sub(1))
		.and_then(|bytes| return bytes.checked_div(block_size))
		.and_then(|blocks| return blocks.checked_mul(block_size))
		.ok_or_else(|| return CheckpointError::invalid_target(target, "checkpoint allocation size overflowed"))?;
	let available = statistics.f_bavail.saturating_mul(block_size);
	let reservation = EXACT_USER_RESERVATION.get();
	if available < allocation.saturating_add(reservation) {
		return Err(CheckpointError::InsufficientCapacity {
			path: target.to_path_buf(),
			available,
			checkpoint_allocation: allocation,
			reservation,
		});
	}
	let sequence = NEXT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
	let name = format!(
		".{}.recipe-{sequence}.tmp",
		target.file_name()
			.and_then(|name| return name.to_str())
			.unwrap_or("model")
	);
	let temporary_path = parent.join(name);
	let mut temporary = OpenOptions::new()
		.write(true)
		.create_new(true)
		.mode(0o600)
		.open(&temporary_path)
		.map_err(|error| return CheckpointError::io("create checkpoint temporary", &temporary_path, error))?;
	if let Err(error) = write_checkpoint(&mut temporary)
		.and_then(|()| return temporary.flush())
		.and_then(|()| return temporary.sync_all())
	{
		let _ignored = fs::remove_file(&temporary_path);
		return Err(CheckpointError::io(
			"write checkpoint temporary",
			&temporary_path,
			error,
		));
	}
	drop(temporary);
	return fs::rename(&temporary_path, target).map_err(|error| return CheckpointError::io("atomically install checkpoint", target, error));
}
