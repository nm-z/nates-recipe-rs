use core::fmt;
use std::{
	io::{self, Write},
	path::Path,
	time::Duration,
};

use recipe_core::{BundleIdentity, RunId};
use recipe_executor::RunJournal;
use recipe_ingest::GgufLimits;
use recipe_native_executor::{LocalCandidateFactory, StagedCrossBackend};
use recipe_prepare::{
	NativeArtifactCatalog, NativeArtifactProvider, NativeCandidateRealizer, NativeExecutorDriver, Preparer,
};
use recipe_training::{
	BayesModelArtifact, CheckpointArtifact, CheckpointDecodeLimits, CompiledInference, CompiledKnnInference,
	CompletedInferenceExecution, CompletedKnnInferenceExecution, DecodedMulticlassClass, GgufLlamaArtifact,
	InferenceCompileError, InferenceExecutionError, InferenceExecutionLimits, InferencePrediction,
	InferencePredictionKind, InferencePreparationError, KnnInferencePrediction, KnnInferencePredictionKind,
	KnnLabelValue, KnnModelArtifact, KnnModelDecodeLimits, SemanticModelArtifact, compile_prepared_bayes_inference,
	compile_prepared_gguf_llama_inference, compile_prepared_inference, compile_prepared_knn_inference,
	load_gguf_llama_model_file, load_semantic_model_file, prepare_and_execute_local_inference,
	prepare_and_execute_local_knn_inference, prepare_bayes_inference_table, prepare_checkpoint_inference_table,
	prepare_gguf_llama_inference_table, prepare_knn_inference_table,
};

use crate::{
	api::{Data, DeclarationError, Infer, InferenceDeclaration, Metric, Model},
	data_prepare::{DataPreparationError, distill_data, select_target_free_data},
	native_prepare::{NativePreparationError, with_current_native_preparation},
	training::{derive_native_runtime_tuning, next_run_id},
};

/// The public inference boundary failed before native execution.
#[derive(Debug)]
#[non_exhaustive]
pub enum InferenceError {
	Declaration(DeclarationError),
	Data(DataPreparationError),
	Model(InferencePreparationError),
	Compile(InferenceCompileError),
	Native(NativePreparationError),
	Execute(InferenceExecutionError),
	Runtime { stage: &'static str, detail: String },
	Unsupported { detail: String },
}

impl InferenceError {
	fn unsupported(detail: impl Into<String>) -> Self {
		Self::Unsupported {
			detail: detail.into(),
		}
	}

	fn runtime(stage: &'static str, detail: impl Into<String>) -> Self {
		Self::Runtime {
			stage,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for InferenceError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Declaration(error) => write!(formatter, "invalid inference declaration: {error}"),
			Self::Data(error) => write!(formatter, "prepare inference data: {error}"),
			Self::Model(error) => write!(formatter, "load inference model: {error}"),
			Self::Compile(error) => write!(formatter, "compile inference graph: {error}"),
			Self::Native(error) => write!(formatter, "prepare current native system: {error}"),
			Self::Execute(error) => write!(formatter, "execute native inference: {error}"),
			Self::Runtime { stage, detail } => write!(formatter, "{stage}: {detail}"),
			Self::Unsupported { detail } => write!(formatter, "unsupported inference declaration: {detail}"),
		}
	}
}

impl std::error::Error for InferenceError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Declaration(error) => Some(error),
			Self::Data(error) => Some(error),
			Self::Model(error) => Some(error),
			Self::Compile(error) => Some(error),
			Self::Native(error) => Some(error),
			Self::Execute(error) => Some(error),
			Self::Runtime { .. } | Self::Unsupported { .. } => None,
		}
	}
}

impl From<DeclarationError> for InferenceError {
	fn from(error: DeclarationError) -> Self { Self::Declaration(error) }
}

impl From<DataPreparationError> for InferenceError {
	fn from(error: DataPreparationError) -> Self { Self::Data(error) }
}

impl From<InferencePreparationError> for InferenceError {
	fn from(error: InferencePreparationError) -> Self { Self::Model(error) }
}

impl From<InferenceCompileError> for InferenceError {
	fn from(error: InferenceCompileError) -> Self { Self::Compile(error) }
}

impl From<NativePreparationError> for InferenceError {
	fn from(error: NativePreparationError) -> Self { Self::Native(error) }
}

impl From<InferenceExecutionError> for InferenceError {
	fn from(error: InferenceExecutionError) -> Self { Self::Execute(error) }
}

pub type InferenceResult<T> = Result<T, InferenceError>;

/// Recipe-owned inference instrument selected from a strict model root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceModelKind {
	Dense,
	Knn,
	Bayes,
	GgufLlama,
}

/// One immutable native inference program selected from a semantic model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompiledModelInference {
	Dense(CompiledInference),
	Knn(CompiledKnnInference),
	Bayes(CompiledInference),
	GgufLlama(CompiledInference),
}

impl CompiledModelInference {
	#[must_use]
	pub const fn kind(&self) -> InferenceModelKind {
		match self {
			Self::Dense(_) => InferenceModelKind::Dense,
			Self::Knn(_) => InferenceModelKind::Knn,
			Self::Bayes(_) => InferenceModelKind::Bayes,
			Self::GgufLlama(_) => InferenceModelKind::GgufLlama,
		}
	}
}

#[derive(Clone, Debug)]
enum InferenceReportPayload {
	Dense {
		checkpoint: CheckpointArtifact,
		execution: CompletedInferenceExecution,
	},
	Knn {
		artifact: KnnModelArtifact,
		execution: CompletedKnnInferenceExecution,
	},
	Bayes {
		artifact: BayesModelArtifact,
		execution: CompletedInferenceExecution,
	},
	GgufLlama {
		artifact: GgufLlamaArtifact,
		execution: CompletedInferenceExecution,
	},
}

#[derive(Clone, Debug)]
enum CompletedModelInference {
	Dense(CompletedInferenceExecution),
	Knn(CompletedKnnInferenceExecution),
	Bayes(CompletedInferenceExecution),
	GgufLlama(CompletedInferenceExecution),
}

impl CompletedModelInference {
	const fn elapsed(&self) -> Duration {
		match self {
			Self::Dense(execution) | Self::Bayes(execution) | Self::GgufLlama(execution) => execution.elapsed(),
			Self::Knn(execution) => execution.elapsed(),
		}
	}
}

#[derive(Clone, Debug)]
enum LoadedInferenceModel {
	Semantic(SemanticModelArtifact),
	GgufLlama(GgufLlamaArtifact),
}

/// Fully exited target-free inference result.
#[derive(Clone, Debug)]
pub struct InferenceReport {
	payload: InferenceReportPayload,
	elapsed: Duration,
	devices: Vec<String>,
}

impl InferenceReport {
	#[must_use]
	pub const fn kind(&self) -> InferenceModelKind {
		match self.payload {
			InferenceReportPayload::Dense { .. } => InferenceModelKind::Dense,
			InferenceReportPayload::Knn { .. } => InferenceModelKind::Knn,
			InferenceReportPayload::Bayes { .. } => InferenceModelKind::Bayes,
			InferenceReportPayload::GgufLlama { .. } => InferenceModelKind::GgufLlama,
		}
	}

	#[must_use]
	pub const fn run(&self) -> RunId {
		match &self.payload {
			InferenceReportPayload::Dense { execution, .. } => execution.run(),
			InferenceReportPayload::Knn { execution, .. } => execution.run(),
			InferenceReportPayload::Bayes { execution, .. } => execution.run(),
			InferenceReportPayload::GgufLlama { execution, .. } => execution.run(),
		}
	}

	#[must_use]
	pub const fn bundle(&self) -> BundleIdentity {
		match &self.payload {
			InferenceReportPayload::Dense { execution, .. } => execution.bundle(),
			InferenceReportPayload::Knn { execution, .. } => execution.bundle(),
			InferenceReportPayload::Bayes { execution, .. } => execution.bundle(),
			InferenceReportPayload::GgufLlama { execution, .. } => execution.bundle(),
		}
	}

	/// The singular dense, Bayesian, or GGUF-logit prediction, or `None` for an
	/// all-output KNN report.
	#[must_use]
	pub const fn prediction(&self) -> Option<&InferencePrediction> {
		match &self.payload {
			InferenceReportPayload::Dense { execution, .. } => Some(execution.prediction()),
			InferenceReportPayload::Knn { .. } => None,
			InferenceReportPayload::Bayes { execution, .. } => Some(execution.prediction()),
			InferenceReportPayload::GgufLlama { execution, .. } => Some(execution.prediction()),
		}
	}

	/// Independently typed KNN predictions in declared-target order, or `None`
	/// for a dense report.
	#[must_use]
	pub fn knn_predictions(&self) -> Option<&[KnnInferencePrediction]> {
		match &self.payload {
			InferenceReportPayload::Dense { .. }
			| InferenceReportPayload::Bayes { .. }
			| InferenceReportPayload::GgufLlama { .. } => None,
			InferenceReportPayload::Knn { execution, .. } => Some(execution.predictions()),
		}
	}

	#[must_use]
	pub const fn journal(&self) -> &RunJournal {
		match &self.payload {
			InferenceReportPayload::Dense { execution, .. } => execution.journal(),
			InferenceReportPayload::Knn { execution, .. } => execution.journal(),
			InferenceReportPayload::Bayes { execution, .. } => execution.journal(),
			InferenceReportPayload::GgufLlama { execution, .. } => execution.journal(),
		}
	}

	/// Exact bundled native images built, loaded, warmed, and torn down by this
	/// inference run. KNN currently exposes its typed predictions but not its
	/// retained realization image.
	#[must_use]
	pub const fn native_kernels(&self) -> Option<&recipe_training::RealizedNativeKernelSet> {
		match &self.payload {
			InferenceReportPayload::Dense { execution, .. }
			| InferenceReportPayload::Bayes { execution, .. }
			| InferenceReportPayload::GgufLlama { execution, .. } => Some(execution.native_kernels()),
			InferenceReportPayload::Knn { .. } => None,
		}
	}

	/// Driver-derived realization and teardown evidence for the completed run.
	#[must_use]
	pub const fn native_evidence(&self) -> &recipe_native_executor::NativeExecutionEvidence {
		match &self.payload {
			InferenceReportPayload::Dense { execution, .. }
			| InferenceReportPayload::Bayes { execution, .. }
			| InferenceReportPayload::GgufLlama { execution, .. } => execution.native_evidence(),
			InferenceReportPayload::Knn { execution, .. } => execution.native_evidence(),
		}
	}

	/// One complete checked native inference loop. Lifecycle setup, output
	/// collection, and teardown are excluded from this throughput interval and
	/// remain covered by the report's native lifecycle evidence.
	#[must_use]
	pub const fn elapsed(&self) -> Duration { self.elapsed }

	#[must_use]
	pub fn devices(&self) -> &[String] { &self.devices }

	/// Exact dense, Bayesian, or GGUF-logit payload interpreted as little-endian
	/// binary32 values. KNN reports return an empty iterator because their outputs
	/// may be mixed F32/I32; use [`Self::knn_predictions`] instead.
	/// The native exit boundary has already validated dtype, shape, and byte
	/// count before this iterator can be constructed.
	pub fn values(&self) -> impl ExactSizeIterator<Item = f32> + '_ {
		self.prediction()
			.map_or(&[][..], InferencePrediction::bytes)
			.chunks_exact(4)
			.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one validated f32")))
	}

	/// Decode one saved multiclass identity without refitting or host-side label
	/// inference.
	#[must_use]
	pub fn decode_multiclass_class(&self, class: usize) -> Option<DecodedMulticlassClass<'_>> {
		match &self.payload {
			InferenceReportPayload::Dense { checkpoint, .. } => checkpoint.decode_multiclass_class(class),
			InferenceReportPayload::Knn { .. }
			| InferenceReportPayload::Bayes { .. }
			| InferenceReportPayload::GgufLlama { .. } => None,
		}
	}

	/// Decode one categorical Bayesian child class through the saved dictionary.
	/// This compatibility accessor addresses the first repeated declaration.
	#[must_use]
	pub fn decode_bayes_class(&self, class: usize) -> Option<&[u8]> { self.decode_bayes_output_class(0, class) }

	/// Number of observed categorical Bayesian target outputs in declaration
	/// order, or zero for another model family.
	#[must_use]
	pub fn bayes_output_count(&self) -> usize {
		match &self.payload {
			InferenceReportPayload::Bayes { artifact, .. } => artifact.conditionals().len(),
			InferenceReportPayload::Dense { .. }
			| InferenceReportPayload::Knn { .. }
			| InferenceReportPayload::GgufLlama { .. } => 0,
		}
	}

	/// Saved target name for one Bayesian output declaration.
	#[must_use]
	pub fn bayes_output_name(&self, output: usize) -> Option<&[u8]> {
		match &self.payload {
			InferenceReportPayload::Bayes { artifact, .. } => {
				artifact
					.conditionals()
					.get(output)
					.map(|conditional| conditional.child().name())
			}
			InferenceReportPayload::Dense { .. }
			| InferenceReportPayload::Knn { .. }
			| InferenceReportPayload::GgufLlama { .. } => None,
		}
	}

	/// Saved class count for one Bayesian output declaration.
	#[must_use]
	pub fn bayes_output_classes(&self, output: usize) -> Option<usize> {
		match &self.payload {
			InferenceReportPayload::Bayes { artifact, .. } => {
				artifact
					.conditionals()
					.get(output)
					.map(|conditional| conditional.child_classes())
			}
			InferenceReportPayload::Dense { .. }
			| InferenceReportPayload::Knn { .. }
			| InferenceReportPayload::GgufLlama { .. } => None,
		}
	}

	/// Column range occupied by one Bayesian target inside each packed
	/// probability row.
	#[must_use]
	pub fn bayes_output_range(&self, output: usize) -> Option<core::ops::Range<usize>> {
		let InferenceReportPayload::Bayes { artifact, .. } = &self.payload else {
			return None;
		};
		let conditional = artifact.conditionals().get(output)?;
		let start = artifact
			.conditionals()
			.iter()
			.take(output)
			.try_fold(0_usize, |offset, conditional| {
				offset.checked_add(conditional.child_classes())
			})?;
		Some(start..start.checked_add(conditional.child_classes())?)
	}

	/// Decode one class for one repeated Bayesian target declaration.
	#[must_use]
	pub fn decode_bayes_output_class(&self, output: usize, class: usize) -> Option<&[u8]> {
		match &self.payload {
			InferenceReportPayload::Bayes { artifact, .. } => {
				artifact
					.conditionals()
					.get(output)?
					.child()
					.dictionary()
					.get(class)
					.map(Vec::as_slice)
			}
			InferenceReportPayload::Dense { .. }
			| InferenceReportPayload::Knn { .. }
			| InferenceReportPayload::GgufLlama { .. } => None,
		}
	}

	/// Decode one discrete KNN class code through the saved target dictionary.
	/// Numeric outputs and dense reports return `None`.
	#[must_use]
	pub fn decode_knn_class(&self, output: usize, code: i32) -> Option<&KnnLabelValue> {
		match &self.payload {
			InferenceReportPayload::Dense { .. }
			| InferenceReportPayload::Bayes { .. }
			| InferenceReportPayload::GgufLlama { .. } => None,
			InferenceReportPayload::Knn { artifact, .. } => {
				artifact
					.references()
					.outputs()
					.get(output)
					.and_then(|output| output.decode_class(code))
			}
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) struct CompiledInferencePackage {
	model: LoadedInferenceModel,
	inference: CompiledModelInference,
}

pub(crate) fn evaluate_inference_declaration(declaration: &InferenceDeclaration) -> InferenceResult<InferenceReport> {
	let package = compile_inference_declaration(declaration)?;
	let (execution, devices) = execute_current_inference_native(&package.inference)?;
	let elapsed = execution.elapsed();
	let payload = match (package.model, execution) {
		(
			LoadedInferenceModel::Semantic(SemanticModelArtifact::Dense(checkpoint)),
			CompletedModelInference::Dense(execution),
		) => {
			InferenceReportPayload::Dense {
				checkpoint,
				execution,
			}
		}
		(
			LoadedInferenceModel::Semantic(SemanticModelArtifact::Knn(artifact)),
			CompletedModelInference::Knn(execution),
		) => {
			InferenceReportPayload::Knn {
				artifact,
				execution,
			}
		}
		(
			LoadedInferenceModel::Semantic(SemanticModelArtifact::Bayes(artifact)),
			CompletedModelInference::Bayes(execution),
		) => {
			InferenceReportPayload::Bayes {
				artifact,
				execution,
			}
		}
		(LoadedInferenceModel::GgufLlama(artifact), CompletedModelInference::GgufLlama(execution)) => {
			InferenceReportPayload::GgufLlama {
				artifact,
				execution,
			}
		}
		_ => unreachable!("compiled and completed model families remain identical"),
	};
	let report = InferenceReport {
		payload,
		elapsed,
		devices,
	};
	write_inference_report(&mut io::stdout().lock(), declaration.policy(), &report)
		.map_err(|error| InferenceError::runtime("write inference report", error.to_string()))?;
	Ok(report)
}

/// Translate public target-free data, checkpoint model, and inference policy
/// declarations into one immutable Recipe inference program.
///
/// This boundary performs bounded filesystem reads and schema preparation, but
/// no native probing, code generation, allocation, or execution.
pub fn compile_inference(policy: &Infer, data: &Data, model: &Model) -> InferenceResult<CompiledModelInference> {
	compile_inference_package(policy, data, model).map(|package| package.inference)
}

pub(crate) fn compile_inference_declaration(
	declaration: &InferenceDeclaration,
) -> InferenceResult<CompiledInferencePackage> {
	let data = declaration
		.data()
		.ok_or_else(|| InferenceError::unsupported("inference evaluation requires one data declaration"))?;
	compile_inference_package(declaration.policy(), data, declaration.model())
}

fn compile_inference_package(policy: &Infer, data: &Data, model: &Model) -> InferenceResult<CompiledInferencePackage> {
	policy.validate()?;
	data.validate()?;
	model.validate()?;
	require_target_free_data_policy(data)?;
	let source = require_inference_model_source(model)?;
	let model = match source.extension().and_then(|extension| extension.to_str()) {
		Some("ogdl") => {
			LoadedInferenceModel::Semantic(load_semantic_model_file(
				source,
				CheckpointDecodeLimits::default(),
				KnnModelDecodeLimits::default(),
			)?)
		}
		Some("gguf") => {
			LoadedInferenceModel::GgufLlama(load_gguf_llama_model_file(
				source,
				gguf_limits_for_file(source)?,
			)?)
		}
		_ => unreachable!("model extension was validated before loading"),
	};
	let distilled = distill_data(data)?;
	let selected = select_target_free_data(data, &distilled)?;
	let inference = match &model {
		LoadedInferenceModel::Semantic(SemanticModelArtifact::Dense(checkpoint)) => {
			let prepared = prepare_checkpoint_inference_table(checkpoint.clone(), &selected)?;
			CompiledModelInference::Dense(compile_prepared_inference(&prepared)?)
		}
		LoadedInferenceModel::Semantic(SemanticModelArtifact::Knn(artifact)) => {
			let prepared = prepare_knn_inference_table(artifact.clone(), &selected)?;
			CompiledModelInference::Knn(compile_prepared_knn_inference(&prepared)?)
		}
		LoadedInferenceModel::Semantic(SemanticModelArtifact::Bayes(artifact)) => {
			let prepared = prepare_bayes_inference_table(artifact.clone(), &selected)?;
			CompiledModelInference::Bayes(compile_prepared_bayes_inference(&prepared)?)
		}
		LoadedInferenceModel::GgufLlama(artifact) => {
			let prepared = prepare_gguf_llama_inference_table(artifact.clone(), &selected)?;
			CompiledModelInference::GgufLlama(compile_prepared_gguf_llama_inference(&prepared)?)
		}
	};
	Ok(CompiledInferencePackage { model, inference })
}

fn require_target_free_data_policy(data: &Data) -> InferenceResult<()> {
	if !data.targets().is_empty() {
		return Err(InferenceError::unsupported(
			"target-free inference does not consume `.target(...)`; the saved model supplies target interpretation",
		));
	}
	if data.split_fraction().is_some() {
		return Err(InferenceError::unsupported(
			"target-free inference evaluates every declared row and does not accept `.split(...)`",
		));
	}
	if data.normalization().is_some() {
		return Err(InferenceError::unsupported(
			"inference normalization is fixed by the semantic model and cannot be redeclared on data",
		));
	}
	Ok(())
}

fn require_inference_model_source(model: &Model) -> InferenceResult<&Path> {
	let source = model
		.weights_source()
		.ok_or_else(|| InferenceError::unsupported("inference requires `recipe.model().load(MODEL_PATH)`"))?;
	let path = Path::new(source);
	match path.extension().and_then(|extension| extension.to_str()) {
		Some("ogdl" | "gguf") => Ok(path),
		Some(extension) => {
			Err(InferenceError::unsupported(format!(
				"inference model extension .{extension} is neither .ogdl nor .gguf"
			)))
		}
		None => {
			Err(InferenceError::unsupported(
				"inference model path has no .ogdl or .gguf extension",
			))
		}
	}
}

fn gguf_limits_for_file(path: &Path) -> InferenceResult<GgufLimits> {
	let file_bytes = std::fs::metadata(path)
		.map_err(|error| {
			InferenceError::runtime(
				"inspect GGUF inference model length",
				format!("{}: {error}", path.display()),
			)
		})?
		.len();
	let bound = file_bytes.max(1);
	GgufLimits::new(bound, bound, bound, 4, bound, bound, bound).map_err(|error| {
		InferenceError::runtime(
			"bound GGUF inference model",
			format!("{}: {error}", path.display()),
		)
	})
}

fn execute_current_inference_native(
	inference: &CompiledModelInference,
) -> InferenceResult<(CompletedModelInference, Vec<String>)> {
	let run = next_run_id();
	let result = with_current_native_preparation(|profile, _config, scope| {
		let devices = scope
			.targets()
			.devices()
			.iter()
			.map(|target| target.origin().as_str().to_owned())
			.collect::<Vec<_>>();
		let (bindings, host, targets) = scope.into_parts();
		let graph = match inference {
			CompiledModelInference::Dense(inference) => inference.graph(),
			CompiledModelInference::Knn(inference) => inference.graph(),
			CompiledModelInference::Bayes(inference) => inference.graph(),
			CompiledModelInference::GgufLlama(inference) => inference.graph(),
		};
		let tuning = derive_native_runtime_tuning(graph, profile, host.machine())
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
		let limits = InferenceExecutionLimits::new(tuning.watchdog);
		let host = host
			.backend_config(run, tuning.worker_threads, tuning.staging_bytes_per_worker)
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
		let (cuda, hsa) = bindings.into_parts();
		let bridge = StagedCrossBackend::new(cuda.clone(), hsa.clone());
		let factory = LocalCandidateFactory::production(Some(host), cuda, hsa, bridge);
		let driver = NativeExecutorDriver::new(factory);
		let compiler = targets
			.deferred_compiler()
			.map_err(NativePreparationError::TargetSpecification)?;
		let realizer = NativeCandidateRealizer::new(profile, compiler, driver)
			.map_err(|error| NativePreparationError::LocalConfiguration(error.to_string()))?;
		let provider = NativeArtifactProvider::new(NativeArtifactCatalog::default());
		let mut preparer = Preparer::new(provider, realizer);
		let execution = match inference {
			CompiledModelInference::Dense(inference) => {
				prepare_and_execute_local_inference(inference, profile, &mut preparer, run, limits)
					.map(CompletedModelInference::Dense)
			}
			CompiledModelInference::Knn(inference) => {
				prepare_and_execute_local_knn_inference(inference, profile, &mut preparer, run, limits)
					.map(CompletedModelInference::Knn)
			}
			CompiledModelInference::Bayes(inference) => {
				prepare_and_execute_local_inference(inference, profile, &mut preparer, run, limits)
					.map(CompletedModelInference::Bayes)
			}
			CompiledModelInference::GgufLlama(inference) => {
				prepare_and_execute_local_inference(inference, profile, &mut preparer, run, limits)
					.map(CompletedModelInference::GgufLlama)
			}
		};
		Ok((execution, devices))
	})?;
	let (execution, devices) = result;
	Ok((execution?, devices))
}

fn write_inference_report(writer: &mut impl Write, policy: &Infer, report: &InferenceReport) -> io::Result<()> {
	if policy
		.log_items()
		.iter()
		.any(|item| item.metric() == Metric::Time)
	{
		writeln!(writer, "time\t{:.6}s", report.elapsed().as_secs_f64())?;
	}
	if policy
		.log_items()
		.iter()
		.any(|item| item.metric() == Metric::Device)
	{
		writeln!(writer, "device\t{}", report.devices().join(","))?;
	}
	write_prediction_rows(writer, report)
}

fn write_prediction_rows(writer: &mut impl Write, report: &InferenceReport) -> io::Result<()> {
	match &report.payload {
		InferenceReportPayload::Dense { .. } => write_dense_prediction_rows(writer, report),
		InferenceReportPayload::Knn {
			artifact,
			execution,
		} => write_knn_prediction_rows(writer, artifact, execution),
		InferenceReportPayload::Bayes {
			artifact,
			execution,
		} => write_bayes_prediction_rows(writer, artifact, execution),
		InferenceReportPayload::GgufLlama {
			artifact,
			execution,
		} => write_gguf_llama_prediction_rows(writer, artifact, execution),
	}
}

fn write_dense_prediction_rows(writer: &mut impl Write, report: &InferenceReport) -> io::Result<()> {
	let checkpoint = match &report.payload {
		InferenceReportPayload::Dense { checkpoint, .. } => checkpoint,
		InferenceReportPayload::Knn { .. } => unreachable!("dense writer received a KNN report"),
		InferenceReportPayload::Bayes { .. } => unreachable!("dense writer received a Bayesian report"),
		InferenceReportPayload::GgufLlama { .. } => unreachable!("dense writer received a GGUF llama report"),
	};
	let prediction = report
		.prediction()
		.expect("dense report has one singular prediction");
	let contract = prediction.contract();
	let shape = contract.shape().extents();
	let [rows, width] = shape else {
		unreachable!("validated inference prediction is rank two");
	};
	let width = usize::try_from(*width).expect("validated prediction width fits usize");
	let row_bytes = width
		.checked_mul(core::mem::size_of::<f32>())
		.expect("validated prediction row byte count fits usize");
	let bytes = prediction.bytes();
	debug_assert_eq!(bytes.len(), usize::try_from(*rows).unwrap() * row_bytes);
	for (row, row_image) in bytes.chunks_exact(row_bytes).enumerate() {
		match contract.kind() {
			InferencePredictionKind::BinaryProbability => {
				writeln!(
					writer,
					"prediction\t{row}\tprobability\t{}",
					decode_prediction_f32(&row_image[..4])
				)?;
			}
			InferencePredictionKind::Regression => {
				writeln!(
					writer,
					"prediction\t{row}\tvalue\t{}",
					decode_prediction_f32(&row_image[..4])
				)?;
			}
			InferencePredictionKind::MulticlassProbabilities => {
				let class = multiclass_argmax(row_image);
				write!(writer, "prediction\t{row}\tclass\t{class}\tlabel\t")?;
				write_multiclass_label(writer, report.decode_multiclass_class(class))?;
				write!(writer, "\tprobabilities\t[")?;
				for (index, value) in row_image
					.chunks_exact(4)
					.map(decode_prediction_f32)
					.enumerate()
				{
					if index != 0 {
						writer.write_all(b",")?;
					}
					write!(writer, "{value}")?;
				}
				writeln!(writer, "]")?;
			}
			InferencePredictionKind::MultiTargetBinaryProbabilities => {
				write!(writer, "prediction\t{row}\tprobabilities\t")?;
				write_named_dense_values(writer, checkpoint, row_image)?;
				writeln!(writer)?;
			}
			InferencePredictionKind::JointTargetProbabilities => {
				let class = multiclass_argmax(row_image);
				write!(writer, "prediction\t{row}\tclass\t{class}\ttarget\t")?;
				write_dense_target_name(writer, checkpoint, class)?;
				write!(writer, "\tprobabilities\t")?;
				write_named_dense_values(writer, checkpoint, row_image)?;
				writeln!(writer)?;
			}
			InferencePredictionKind::MultiTargetRegression => {
				write!(writer, "prediction\t{row}\tvalues\t")?;
				write_named_dense_values(writer, checkpoint, row_image)?;
				writeln!(writer)?;
			}
			InferencePredictionKind::TokenLogits => {
				unreachable!("dense semantic models cannot emit GGUF token logits")
			}
			InferencePredictionKind::BayesProbabilities => {
				unreachable!("dense semantic models cannot emit Bayesian probability blocks")
			}
		}
	}
	Ok(())
}

fn write_gguf_llama_prediction_rows(
	writer: &mut impl Write,
	artifact: &GgufLlamaArtifact,
	execution: &CompletedInferenceExecution,
) -> io::Result<()> {
	let prediction = execution.prediction();
	let contract = prediction.contract();
	if contract.kind() != InferencePredictionKind::TokenLogits {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"GGUF llama prediction is not a raw token-logit matrix",
		));
	}
	let [positions, vocabulary] = contract.shape().extents() else {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"GGUF llama prediction is not rank two",
		));
	};
	if *vocabulary != artifact.vocabulary() {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"GGUF llama prediction width differs from the model vocabulary",
		));
	}
	let vocabulary = usize::try_from(*vocabulary).map_err(|error| {
		io::Error::new(
			io::ErrorKind::InvalidData,
			format!("GGUF llama vocabulary width: {error}"),
		)
	})?;
	let row_bytes = vocabulary
		.checked_mul(core::mem::size_of::<f32>())
		.ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				"GGUF llama logit row size overflowed",
			)
		})?;
	let expected_bytes = usize::try_from(*positions)
		.ok()
		.and_then(|positions| positions.checked_mul(row_bytes))
		.ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				"GGUF llama logit size overflowed",
			)
		})?;
	if prediction.bytes().len() != expected_bytes {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"GGUF llama prediction byte count differs from its matrix shape",
		));
	}
	for (position, row_image) in prediction.bytes().chunks_exact(row_bytes).enumerate() {
		let token = multiclass_argmax(row_image);
		let logit = decode_prediction_f32(&row_image[token * 4..token * 4 + 4]);
		writeln!(
			writer,
			"prediction\t{position}\ttoken\t{token}\tlogit\t{logit}"
		)?;
	}
	Ok(())
}

fn write_named_dense_values(
	writer: &mut impl Write,
	checkpoint: &CheckpointArtifact,
	row_image: &[u8],
) -> io::Result<()> {
	let values = row_image.chunks_exact(4).map(decode_prediction_f32);
	if values.len() != checkpoint.target_source_indices().len() {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"dense prediction width differs from saved target order",
		));
	}
	writer.write_all(b"[")?;
	for (column, value) in values.enumerate() {
		if column != 0 {
			writer.write_all(b",")?;
		}
		write_dense_target_name(writer, checkpoint, column)?;
		write!(writer, "={value}")?;
	}
	writer.write_all(b"]")
}

fn write_dense_target_name(writer: &mut impl Write, checkpoint: &CheckpointArtifact, column: usize) -> io::Result<()> {
	let source_index = checkpoint
		.target_source_indices()
		.get(column)
		.copied()
		.ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				"dense target column is out of range",
			)
		})?;
	let vector = checkpoint
		.vectors()
		.iter()
		.find(|vector| vector.source_index() == source_index && vector.role() == recipe_ingest::VectorRole::Target)
		.ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				"saved dense target schema is absent",
			)
		})?;
	write_quoted_bytes(writer, vector.name())
}

fn write_bayes_prediction_rows(
	writer: &mut impl Write,
	artifact: &BayesModelArtifact,
	execution: &CompletedInferenceExecution,
) -> io::Result<()> {
	let prediction = execution.prediction();
	let contract = prediction.contract();
	if contract.kind() != InferencePredictionKind::BayesProbabilities {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"Bayesian prediction is not a categorical probability matrix",
		));
	}
	let [rows, width] = contract.shape().extents() else {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"Bayesian prediction is not rank two",
		));
	};
	let width = usize::try_from(*width).map_err(|error| {
		io::Error::new(
			io::ErrorKind::InvalidData,
			format!("Bayesian class width: {error}"),
		)
	})?;
	let expected_width = artifact
		.conditionals()
		.iter()
		.try_fold(0_usize, |width, conditional| {
			width.checked_add(conditional.child_classes())
		})
		.ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				"Bayesian saved output width overflowed",
			)
		})?;
	if width != expected_width {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			format!("Bayesian prediction has width {width} for {expected_width} saved child labels"),
		));
	}
	let row_bytes = width
		.checked_mul(core::mem::size_of::<f32>())
		.ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				"Bayesian prediction row size overflowed",
			)
		})?;
	let expected_bytes = usize::try_from(*rows)
		.ok()
		.and_then(|rows| rows.checked_mul(row_bytes))
		.ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				"Bayesian prediction size overflowed",
			)
		})?;
	if prediction.bytes().len() != expected_bytes {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"Bayesian prediction byte count differs from its matrix shape",
		));
	}
	for (row, row_image) in prediction.bytes().chunks_exact(row_bytes).enumerate() {
		let mut start = 0_usize;
		for (output, conditional) in artifact.conditionals().iter().enumerate() {
			let labels = conditional.child().dictionary();
			let end = start + labels.len();
			let output_image = &row_image[start * 4..end * 4];
			let class = multiclass_argmax(output_image);
			write!(writer, "prediction\t{row}")?;
			if artifact.conditionals().len() > 1 {
				write!(writer, "\toutput\t{output}\ttarget\t")?;
				write_quoted_bytes(writer, conditional.child().name())?;
			}
			write!(writer, "\tclass\t{class}\tlabel\t")?;
			write_quoted_bytes(writer, &labels[class])?;
			write!(writer, "\tprobabilities\t[")?;
			for (index, value) in output_image
				.chunks_exact(4)
				.map(decode_prediction_f32)
				.enumerate()
			{
				if index != 0 {
					writer.write_all(b",")?;
				}
				write!(writer, "{value}")?;
			}
			writeln!(writer, "]")?;
			start = end;
		}
	}
	Ok(())
}

fn write_knn_prediction_rows(
	writer: &mut impl Write,
	artifact: &KnnModelArtifact,
	execution: &CompletedKnnInferenceExecution,
) -> io::Result<()> {
	let predictions = execution.predictions();
	let outputs = artifact.references().outputs();
	if predictions.len() != outputs.len() {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			format!(
				"KNN execution returned {} outputs for {} saved targets",
				predictions.len(),
				outputs.len()
			),
		));
	}
	let rows = predictions
		.first()
		.map(|prediction| {
			let [rows, 1] = prediction.contract().shape().extents() else {
				unreachable!("validated KNN prediction shape is [rows, 1]");
			};
			usize::try_from(*rows).expect("validated KNN row count fits usize")
		})
		.unwrap_or(0);
	for row in 0..rows {
		for (output_index, (prediction, output)) in predictions.iter().zip(outputs).enumerate() {
			if prediction.contract().source_vector() != output.schema().source_index() {
				return Err(io::Error::new(
					io::ErrorKind::InvalidData,
					format!("KNN prediction {output_index} has the wrong saved target identity"),
				));
			}
			let begin = row
				.checked_mul(4)
				.expect("validated KNN prediction byte offset fits usize");
			let scalar = &prediction.bytes()[begin..begin + 4];
			write!(writer, "prediction\t{row}\toutput\t")?;
			write_quoted_bytes(writer, output.schema().name())?;
			match prediction.contract().kind() {
				KnnInferencePredictionKind::NumericMean => {
					writeln!(writer, "\tvalue\t{}", decode_prediction_f32(scalar))?;
				}
				KnnInferencePredictionKind::DiscreteMode => {
					let code =
						i32::from_le_bytes(scalar.try_into().expect("one validated KNN i32 prediction"));
					let label = output.decode_class(code).ok_or_else(|| {
						io::Error::new(
							io::ErrorKind::InvalidData,
							format!("KNN prediction {output_index} emitted unknown class code {code}"),
						)
					})?;
					write!(writer, "\tclass\t{code}\tlabel\t")?;
					write_knn_label(writer, label)?;
					writeln!(writer)?;
				}
			}
		}
	}
	Ok(())
}

fn write_knn_label(writer: &mut impl Write, label: &KnnLabelValue) -> io::Result<()> {
	if let Some(value) = label.as_i32() {
		return write!(writer, "{value}");
	}
	write_quoted_bytes(writer, label.as_bytes().unwrap())
}

fn decode_prediction_f32(bytes: &[u8]) -> f32 {
	f32::from_le_bytes(bytes.try_into().expect("one validated prediction f32"))
}

fn multiclass_argmax(row_image: &[u8]) -> usize {
	let mut values = row_image
		.chunks_exact(4)
		.map(decode_prediction_f32)
		.enumerate();
	let (mut best_class, mut best_value) = values
		.next()
		.expect("validated multiclass prediction width is nonzero");
	for (class, value) in values {
		if value.total_cmp(&best_value).is_gt() {
			best_class = class;
			best_value = value;
		}
	}
	best_class
}

fn write_multiclass_label(writer: &mut impl Write, label: Option<DecodedMulticlassClass<'_>>) -> io::Result<()> {
	match label {
		Some(DecodedMulticlassClass::ReservedUnseen) => writer.write_all(b"<reserved-unseen>"),
		Some(DecodedMulticlassClass::Label(bytes)) => write_quoted_bytes(writer, bytes),
		None => writer.write_all(b"<unavailable>"),
	}
}

fn write_quoted_bytes(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
	writer.write_all(b"\"")?;
	for byte in bytes {
		match *byte {
			b'\\' => writer.write_all(b"\\\\")?,
			b'\"' => writer.write_all(b"\\\"")?,
			b'\n' => writer.write_all(b"\\n")?,
			b'\r' => writer.write_all(b"\\r")?,
			b'\t' => writer.write_all(b"\\t")?,
			0x20..=0x7e => writer.write_all(&[*byte])?,
			byte => write!(writer, "\\x{byte:02x}")?,
		}
	}
	writer.write_all(b"\"")
}
