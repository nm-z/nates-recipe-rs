use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use recipe_core::{BundleIdentity, DType, EXACT_USER_RESERVATION, RunId, ValueId};
use recipe_executor::{ExitImage, RunJournal};
use recipe_ingest::{PreparedDataset, SemanticType, VectorEncoding, VectorMetadata, VectorRole};

use crate::{
	AdamWConfig, CompiledTraining, CompletedTrainingExecution, DenseActivation, DenseLayer, DenseTrainingConfig,
	FinalTrainingMetric, ParameterState, TrainingBounds,
};

const CHECKPOINT_FORMAT_VERSION: u32 = 1;
const HEX_CHUNK_BYTES: usize = 64;
const TEMP_CREATE_ATTEMPTS: u64 = 64;

static NEXT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// Saving or validating a completed Recipe checkpoint failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum CheckpointError {
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

/// Immutable semantic description used to interpret a completed execution's
/// egress images. It contains no dataset rows, executable artifacts, devices,
/// queues, contexts, or native handles.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointManifest {
	vectors: Vec<CheckpointVectorSchema>,
	feature_width: usize,
	config: DenseTrainingConfig,
	bounds: TrainingBounds,
	normalization_mean: CheckpointTensor,
	normalization_variance: CheckpointTensor,
	layers: Vec<CheckpointLayer>,
	temperature: Option<CheckpointTensor>,
}

impl CheckpointManifest {
	/// Capture only the declaration and no-row schema needed to interpret the
	/// final model state.
	pub fn from_compiled(
		prepared: &PreparedDataset,
		config: &DenseTrainingConfig,
		training: &CompiledTraining,
	) -> CheckpointResult<Self> {
		if config.layers.len() != training.outputs().layers.len() {
			return Err(CheckpointError::manifest(format!(
				"{} declared layers produced {} layer states",
				config.layers.len(),
				training.outputs().layers.len()
			)));
		}
		let feature_width = prepared
			.vectors()
			.iter()
			.filter(|vector| vector.role() == VectorRole::Feature)
			.count();
		let vectors = prepared
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
		let normalization_mean = checkpoint_tensor(training, output.normalization.mean)?;
		let normalization_variance = checkpoint_tensor(training, output.normalization.variance)?;
		let layers = config
			.layers
			.iter()
			.copied()
			.zip(&output.layers)
			.map(|(declaration, state)| -> CheckpointResult<_> {
				Ok(CheckpointLayer {
					declaration,
					weight: checkpoint_parameter(training, state.weight)?,
					bias: checkpoint_parameter(training, state.bias)?,
				})
			})
			.collect::<CheckpointResult<Vec<_>>>()?;
		let temperature = output
			.validation
			.as_ref()
			.and_then(|validation| validation.temperature_scaling)
			.map(|state| checkpoint_tensor(training, state.updated_temperature))
			.transpose()?;
		let manifest = Self {
			vectors,
			feature_width,
			config: config.clone(),
			bounds: training.bounds(),
			normalization_mean,
			normalization_variance,
			layers,
			temperature,
		};
		manifest.validate_external_boundary(training)?;
		Ok(manifest)
	}

	#[must_use]
	pub fn vectors(&self) -> &[CheckpointVectorSchema] {
		&self.vectors
	}

	#[must_use]
	pub const fn feature_width(&self) -> usize {
		self.feature_width
	}

	fn tensors(&self) -> impl Iterator<Item = &CheckpointTensor> {
		std::iter::once(&self.normalization_mean)
			.chain(std::iter::once(&self.normalization_variance))
			.chain(self.layers.iter().flat_map(|layer| {
				[
					&layer.weight.parameter,
					&layer.weight.first_moment,
					&layer.weight.second_moment,
					&layer.bias.parameter,
					&layer.bias.first_moment,
					&layer.bias.second_moment,
				]
			}))
			.chain(self.temperature.iter())
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
	writeln!(writer, "recipe")?;
	writeln!(writer, "\tformat\tdense-training-checkpoint")?;
	writeln!(writer, "\tversion\t{CHECKPOINT_FORMAT_VERSION}")?;
	writeln!(writer, "\tsemantics")?;
	writeln!(writer, "\t\tobjective\tbinary-cross-entropy-with-logits")?;
	writeln!(writer, "\t\tnormalization\tz-score")?;
	writeln!(writer, "\t\toptimizer\tadamw")?;
	writeln!(writer, "\tdataset")?;
	writeln!(writer, "\t\tfeature-width\t{}", manifest.feature_width)?;
	writeln!(writer, "\t\tvectors")?;
	for vector in &manifest.vectors {
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
		encode_vector_metadata(writer, &vector.metadata)?;
	}
	encode_training_config(writer, &manifest.config, manifest.bounds)?;
	writeln!(writer, "\tmodel")?;
	writeln!(writer, "\t\tinput-width\t{}", manifest.feature_width)?;
	writeln!(writer, "\t\tnormalization")?;
	encode_tensor(
		writer,
		3,
		"mean",
		&manifest.normalization_mean,
		required_output(outputs, manifest.normalization_mean.value)?,
	)?;
	encode_tensor(
		writer,
		3,
		"variance",
		&manifest.normalization_variance,
		required_output(outputs, manifest.normalization_variance.value)?,
	)?;
	writeln!(writer, "\t\tlayers")?;
	for (index, layer) in manifest.layers.iter().enumerate() {
		writeln!(writer, "\t\t\tlayer")?;
		writeln!(writer, "\t\t\t\tindex\t{index}")?;
		writeln!(writer, "\t\t\t\twidth\t{}", layer.declaration.width())?;
		writeln!(
			writer,
			"\t\t\t\tactivation\t{}",
			dense_activation(layer.declaration.activation())
		)?;
		writeln!(writer, "\t\t\t\tweight")?;
		encode_parameter(writer, 5, &layer.weight, outputs)?;
		writeln!(writer, "\t\t\t\tbias")?;
		encode_parameter(writer, 5, &layer.bias, outputs)?;
	}
	if let Some(temperature) = &manifest.temperature {
		writeln!(writer, "\t\tcalibration")?;
		encode_tensor(
			writer,
			3,
			"temperature",
			temperature,
			required_output(outputs, temperature.value)?,
		)?;
	}
	Ok(())
}

fn encode_vector_metadata(writer: &mut impl Write, metadata: &VectorMetadata) -> io::Result<()> {
	match metadata {
		VectorMetadata::None => writeln!(writer, "\t\t\t\tmetadata\tnone"),
		VectorMetadata::Temporal { origin } => {
			writeln!(writer, "\t\t\t\tmetadata\ttemporal")?;
			writeln!(writer, "\t\t\t\t\tunix-seconds\t{}", origin.unix_seconds)?;
			writeln!(writer, "\t\t\t\t\tnanoseconds\t{}", origin.nanoseconds)
		}
		VectorMetadata::Categorical { dictionary } => {
			writeln!(writer, "\t\t\t\tmetadata\tcategorical")?;
			for value in dictionary {
				write!(writer, "\t\t\t\t\tvalue-bytes\t")?;
				write_hex(writer, value)?;
				writeln!(writer)?;
			}
			Ok(())
		}
		VectorMetadata::Ordinal { ordered_labels } => {
			writeln!(writer, "\t\t\t\tmetadata\tordinal")?;
			for value in ordered_labels {
				write!(writer, "\t\t\t\t\tvalue-bytes\t")?;
				write_hex(writer, value)?;
				writeln!(writer)?;
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

fn encode_parameter(
	writer: &mut impl Write,
	depth: usize,
	parameter: &CheckpointParameter,
	outputs: &BTreeMap<ValueId, &[u8]>,
) -> io::Result<()> {
	encode_tensor(
		writer,
		depth,
		"parameter",
		&parameter.parameter,
		required_output(outputs, parameter.parameter.value)?,
	)?;
	encode_tensor(
		writer,
		depth,
		"first-moment",
		&parameter.first_moment,
		required_output(outputs, parameter.first_moment.value)?,
	)?;
	encode_tensor(
		writer,
		depth,
		"second-moment",
		&parameter.second_moment,
		required_output(outputs, parameter.second_moment.value)?,
	)
}

fn encode_tensor(
	writer: &mut impl Write,
	depth: usize,
	name: &str,
	tensor: &CheckpointTensor,
	bytes: &[u8],
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
	if bytes.is_empty() {
		write_tabs(writer, depth + 2)?;
		writeln!(writer, "0x")?;
	} else {
		for chunk in bytes.chunks(HEX_CHUNK_BYTES) {
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
		DenseActivation::Silu => "silu",
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
mod tests {
	use core::num::{NonZeroU64, NonZeroUsize};
	use std::os::unix::fs::PermissionsExt;

	use recipe_core::{ArenaObjectId, ByteCount, ByteOffset, DeviceId, ResolvedValueLocation, TaskId};

	use super::*;

	#[test]
	fn codec_is_deterministic_valid_ogdl_and_preserves_raw_f32_and_i32_bits() {
		let mut manifest = test_manifest();
		manifest.normalization_mean.dtype = DType::I32;
		let mut owned = owned_outputs(&manifest);
		owned.insert(
			manifest.normalization_mean.value,
			vec![0xff, 0xff, 0xff, 0xff],
		);
		owned.insert(
			manifest.normalization_variance.value,
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
			vectors: vec![CheckpointVectorSchema {
				source_index: 0,
				name: b"feature\nbytes".to_vec(),
				role: VectorRole::Feature,
				semantic_type: SemanticType::Numeric,
				encoding: VectorEncoding::F32,
				metadata: VectorMetadata::None,
			}],
			feature_width: 1,
			config: DenseTrainingConfig {
				layers: vec![declaration],
				batch_size: NonZeroUsize::new(1).unwrap(),
				epochs: NonZeroU64::new(1).unwrap(),
				warmup_epochs: 0,
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
			normalization_mean: test_tensor(1, DType::F32),
			normalization_variance: test_tensor(2, DType::F32),
			layers: vec![CheckpointLayer {
				declaration,
				weight: test_parameter(3),
				bias: test_parameter(6),
			}],
			temperature: None,
		}
	}

	fn test_tensor(value: u64, dtype: DType) -> CheckpointTensor {
		CheckpointTensor {
			value: ValueId::new(value),
			dtype,
			shape: vec![1],
			bytes: 4,
		}
	}

	fn test_parameter(first_value: u64) -> CheckpointParameter {
		CheckpointParameter {
			parameter: test_tensor(first_value, DType::F32),
			first_moment: test_tensor(first_value + 1, DType::F32),
			second_moment: test_tensor(first_value + 2, DType::F32),
		}
	}

	fn owned_outputs(manifest: &CheckpointManifest) -> BTreeMap<ValueId, Vec<u8>> {
		manifest
			.tensors()
			.map(|tensor| (tensor.value, vec![0, 0, 0, 0]))
			.collect()
	}

	fn borrowed_outputs(outputs: &BTreeMap<ValueId, Vec<u8>>) -> BTreeMap<ValueId, &[u8]> {
		outputs
			.iter()
			.map(|(value, bytes)| (*value, bytes.as_slice()))
			.collect()
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
