use core::num::{NonZeroU64, NonZeroUsize};
use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, MetricId, ScalarOpcode, ValueId};
use recipe_ingest::{DenseMatrix, PreparedDataset, SemanticType, VectorEncoding, VectorRole};
use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather, IndexBounds, IndexMap,
	PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce,
	ReduceOperator, ReduceResult, ScalarExpression, ScalarProgramBuilder, Scatter, ScatterConflict, Shape, Tensor,
};
use recipe_ops::{
	BinaryClassificationMetricRequest, ChannelwiseMaxPool1dPreparation, IdentityNamespace, MaterializationRequest,
	NamedTensor, PreparedParameter, PreparedParameters, RecallAtOutput, binary_metric_requirements, lower_scalar,
	materialize_binary_classification_metrics, materialize_composition, operation_registry,
	prepare_channelwise_max_pool_1d,
};
use recipe_program::{IterationDomain, KernelIterationDomain, MetricEmission, StaticCalculationProgram};

use crate::model::{DenseFeaturePlan, LoweredDenseDataset, validate_binary_targets};
use crate::{
	BinaryMetricOutputs, BinaryValidationConfig, BinaryValidationOutputs, CompiledDatasetSchema, CompiledTraining,
	DataNormalizationState, DenseActivation, DenseBlock, DenseBlockState, DenseDataNormalization,
	DenseGroupToNeuronRouting, DenseLayer, DenseLayerState, DenseLoss, DenseNormalization, DenseOperation,
	DenseOutputAdapter, DensePool, DensePoolState, DenseResidualOperation, DenseResidualState, DenseTask,
	DenseTrainingConfig, EarlyStoppingState, ExternalInputRole, LearningRateDecay, MinMaxState,
	MulticlassMetricOutputs, MulticlassValidationConfig, MulticlassValidationOutputs, OptimizerProgressState,
	OwnedExternalInput, ParameterState, RecallMetricOutput, TemperatureScalingConfig, TemperatureScalingState,
	TrainingBounds, TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult, TrainingMetricBinding,
	TrainingMetricKind, TrainingOutputs, ValidationMetricFamily, ValidationMetricStatus, ValidationUnavailableReason,
	ZScoreState,
};

const MATERIALIZATION_RESERVATION: u64 = 64;
const WORKSPACE_LIMIT: ByteCount = ByteCount::new(u64::MAX);

#[derive(Clone, Debug)]
struct LayerValues {
	input: ValueId,
	weight: InitialParameter,
	forward_weight: ValueId,
	routing_mask: Option<ValueId>,
	bias: InitialParameter,
	operations: Vec<OperationValues>,
}

#[derive(Clone, Debug)]
enum BlockValues {
	Layer(LayerValues),
	Pool(PoolValues),
	Residual(ResidualValues),
}

#[derive(Clone, Debug)]
struct PoolValues {
	state: DensePoolState,
	preparation: ChannelwiseMaxPool1dPreparation,
	winners: ValueId,
	gradient_batch_indices: ValueId,
	input_matrix_indices: ValueId,
	output_group_indices: ValueId,
}

#[derive(Clone, Debug)]
struct ResidualValues {
	input: ValueId,
	branch: Vec<ResidualBranchValues>,
	projection: Option<InitialParameter>,
	operations: Vec<OperationValues>,
}

#[derive(Clone, Debug)]
enum ResidualBranchValues {
	Layer(LayerValues),
	Operation(OperationValues),
}

#[derive(Clone, Copy, Debug)]
struct OperationValues {
	operation: DenseOperation,
	input: ValueId,
	output: ValueId,
	variance: Option<ValueId>,
}

#[derive(Clone, Copy, Debug)]
struct NormalizedValues {
	normalized: ValueId,
	variance: ValueId,
}

#[derive(Clone, Copy, Debug)]
struct InitialParameter {
	value: ValueId,
	first_moment: ValueId,
	second_moment: ValueId,
}

#[derive(Clone, Copy, Debug)]
struct GradientPair {
	weight: ValueId,
	bias: ValueId,
}

#[derive(Clone, Debug)]
enum BlockGradients {
	Layer(GradientPair),
	Pool,
	Residual {
		branch: Vec<GradientPair>,
		projection: Option<ValueId>,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LogicalFeatureShape {
	length: NonZeroU64,
	channels: NonZeroU64,
}

impl LogicalFeatureShape {
	fn from_width(width: u64) -> TrainingCompileResult<Self> {
		let length = NonZeroU64::new(width).ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"logical feature width must be nonzero",
			)
		})?;
		Ok(Self {
			length,
			channels: NonZeroU64::MIN,
		})
	}

	fn width(self) -> TrainingCompileResult<u64> {
		self.length
			.get()
			.checked_mul(self.channels.get())
			.ok_or_else(|| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::ArithmeticOverflow,
					"logical feature width overflowed u64",
				)
			})
	}

	fn pooled(self, pool: DensePool) -> TrainingCompileResult<(Self, DensePoolState)> {
		let output_length = NonZeroU64::new(self.length.get().div_ceil(pool.size().get())).ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"maximum pool produced zero groups",
			)
		})?;
		let state = DensePoolState::new(self.length, self.channels, output_length);
		Ok((
			Self {
				length: output_length,
				channels: self.channels,
			},
			state,
		))
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParameterRole {
	LayerWeight,
	LayerBias,
	ResidualProjectionWeight,
}

#[derive(Clone, Copy, Debug)]
struct ParameterGradient {
	role: ParameterRole,
	value: ValueId,
}

#[derive(Clone, Copy, Debug)]
struct ZScoreValues {
	normalized: ValueId,
	mean: ValueId,
	variance: ValueId,
}

#[derive(Clone, Copy, Debug)]
struct MinMaxValues {
	normalized: ValueId,
	minimum: ValueId,
	maximum: ValueId,
}

#[derive(Clone, Copy, Debug)]
struct ValidationValues {
	features: ValueId,
	targets: ValueId,
	rows: u64,
}

#[derive(Clone, Copy, Debug)]
struct AcceptedUpdatePlan {
	per_epoch: u64,
	maximum: u64,
	warmup: u64,
}

#[derive(Clone, Copy, Debug)]
struct InitialEarlyStoppingState {
	best_auprc: ValueId,
	stale_epochs: ValueId,
	stopped: ValueId,
}

/// Compile one semantically typed fixed-width dataset and dense model into a
/// bounded static Recipe program.
///
/// The final partial batch is represented by clamped gathers plus an exact GPU
/// validity mask. Padded lanes contribute zero loss and zero gradient, so no
/// source row is repeated and no host-side batch calculation is required.
pub fn compile_dense_training(
	dataset: &PreparedDataset,
	config: &DenseTrainingConfig,
) -> TrainingCompileResult<CompiledTraining> {
	let blocks = config
		.layers
		.iter()
		.cloned()
		.map(DenseBlock::Layer)
		.collect::<Vec<_>>();
	compile_dense_training_impl(dataset, config, &blocks, None, None)
}

/// Compile a topology-preserving dense model containing ordinary and residual
/// blocks. Existing flat compilation remains a strict wrapper over this path.
/// The explicit `blocks` are the network declaration; `config.layers` remains
/// only the legacy declaration used by the flat entrypoints.
pub fn compile_dense_training_with_blocks(
	dataset: &PreparedDataset,
	config: &DenseTrainingConfig,
	blocks: &[DenseBlock],
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_training_impl(dataset, config, blocks, None, None)
}

/// Compile topology-preserving blocks with binary validation.
pub fn compile_dense_training_with_blocks_and_binary_validation(
	dataset: &PreparedDataset,
	config: &DenseTrainingConfig,
	blocks: &[DenseBlock],
	validation: &BinaryValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_training_impl(dataset, config, blocks, Some(validation), None)
}

/// Compile topology-preserving blocks with multiclass validation.
pub fn compile_dense_training_with_blocks_and_multiclass_validation(
	dataset: &PreparedDataset,
	config: &DenseTrainingConfig,
	blocks: &[DenseBlock],
	validation: &MulticlassValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_training_impl(dataset, config, blocks, None, Some(validation))
}

/// Compile the dense training program plus epoch-bound validation, a
/// GPU-resident AUPRC early-stop latch, and optional post-training temperature
/// scaling.
pub fn compile_dense_training_with_binary_validation(
	dataset: &PreparedDataset,
	config: &DenseTrainingConfig,
	validation: &BinaryValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	let blocks = config
		.layers
		.iter()
		.cloned()
		.map(DenseBlock::Layer)
		.collect::<Vec<_>>();
	compile_dense_training_impl(dataset, config, &blocks, Some(validation), None)
}

/// Compile categorical cross-entropy training plus epoch-bound mean loss and
/// top-one accuracy over the prepared validation partition.
pub fn compile_dense_training_with_multiclass_validation(
	dataset: &PreparedDataset,
	config: &DenseTrainingConfig,
	validation: &MulticlassValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	let blocks = config
		.layers
		.iter()
		.cloned()
		.map(DenseBlock::Layer)
		.collect::<Vec<_>>();
	compile_dense_training_impl(dataset, config, &blocks, None, Some(validation))
}

fn compile_dense_training_impl(
	prepared: &PreparedDataset,
	config: &DenseTrainingConfig,
	declared_blocks: &[DenseBlock],
	validation_config: Option<&BinaryValidationConfig>,
	multiclass_validation_config: Option<&MulticlassValidationConfig>,
) -> TrainingCompileResult<CompiledTraining> {
	let task = resolve_dense_task(prepared, config.loss)?;
	validate_config(config, declared_blocks)?;
	let feature_plan = DenseFeaturePlan::from_prepared(prepared)?;
	let dataset = LoweredDenseDataset::from_prepared(prepared, &feature_plan, task)?;
	validate_lowered_dataset(&dataset, task)?;
	let feature_width = u64::try_from(dataset.train().feature_columns()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("dense feature width cannot be represented by u64: {error}"),
		)
	})?;
	let (blocks, output_adapter) = effective_blocks(declared_blocks, task, feature_width)?;
	let layers = blocks
		.iter()
		.map(|block| match block {
			DenseBlock::Layer(layer) => Some(layer.clone()),
			DenseBlock::Pool(_) | DenseBlock::Residual(_) => None,
		})
		.collect::<Option<Vec<_>>>()
		.unwrap_or_default();
	let dataset_schema = CompiledDatasetSchema::from_prepared(prepared, task, feature_plan.spans().to_vec());
	let validation_status = validate_validation_config(
		&dataset,
		task,
		config.loss,
		validation_config,
		multiclass_validation_config,
	)?;
	let validation_available = matches!(validation_status, ValidationMetricStatus::Available { .. });
	let retain_validation_inputs = !matches!(
		validation_status,
		ValidationMetricStatus::Unavailable { .. }
	);
	let calibration_iterations = if validation_available {
		validation_config
			.and_then(BinaryValidationConfig::temperature_scaling)
			.map_or(0, |calibration| calibration.iterations.get())
	} else {
		0
	};
	let bounds = training_bounds(
		dataset.train().rows(),
		config.batch_size,
		config.epochs,
		config.warmup_epochs,
		calibration_iterations,
	)?;
	let masked_targets = !dataset.train().all_targets_known();
	let optimizer_stops_early = validation_available
		&& validation_config
			.and_then(BinaryValidationConfig::early_stopping_patience)
			.is_some();
	let accepted_update_plan = (masked_targets || optimizer_stops_early)
		.then(|| {
			accepted_update_plan(
				if masked_targets {
					dataset
						.train()
						.accepted_updates_per_epoch(config.batch_size.get())
				} else {
					usize::try_from(bounds.batches_per_epoch).map_err(|error| {
						TrainingCompileError::new(
							TrainingCompileErrorKind::UnsupportedExtent,
							format!("physical batches per epoch cannot be represented by usize: {error}"),
						)
					})?
				},
				config,
				&bounds,
			)
		})
		.transpose()?;
	let mut compiler = GraphCompiler::new(bounds.iterations, bounds.training_iterations)?;

	let train_features = compiler.external_matrix(ExternalInputRole::TrainFeatures, dataset.train().features())?;
	let train_targets = compiler.external_matrix(ExternalInputRole::TrainTargets, dataset.train().targets())?;
	let train_target_supervision = masked_targets
		.then(|| dataset.train().target_supervision())
		.as_ref()
		.map(|supervision| compiler.external_matrix(ExternalInputRole::TrainTargetSupervision, supervision))
		.transpose()?;
	let validation_inputs = retain_validation_inputs
		.then(|| dataset.validation())
		.flatten()
		.map(|validation| -> TrainingCompileResult<_> {
			Ok((
				compiler.external_matrix(ExternalInputRole::ValidationFeatures, validation.features())?,
				compiler.external_matrix(ExternalInputRole::ValidationTargets, validation.targets())?,
				u64::try_from(validation.rows()).map_err(|error| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::UnsupportedExtent,
						format!("validation rows cannot be represented by u64: {error}"),
					)
				})?,
			))
		})
		.transpose()?;
	let feature_normalization_mask = feature_plan
		.normalization_mask()
		.map(|mask| compiler.external_f32_vector(ExternalInputRole::FeatureNormalizationMask, mask))
		.transpose()?;

	let feature_columns = u64::try_from(dataset.train().feature_columns()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("feature width cannot be represented by u64: {error}"),
		)
	})?;
	let target_code_columns = u64::try_from(dataset.train().targets().columns()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("target code width cannot be represented by u64: {error}"),
		)
	})?;
	let output_columns = u64::try_from(task.output_width()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("task output width cannot be represented by u64: {error}"),
		)
	})?;
	let train_rows = bounds.train_rows;
	let batch = bounds.batch_size;
	let full_feature_shape = shape(&[train_rows, feature_columns])?;
	let full_target_shape = shape(&[train_rows, target_code_columns])?;
	let batch_feature_shape = shape(&[batch, feature_columns])?;
	let batch_target_shape = shape(&[batch, target_code_columns])?;
	let batch_output_shape = shape(&[batch, output_columns])?;
	let batch_row_shape = shape(&[batch, 1])?;
	let scalar_shape = shape(&[1])?;

	let converted_features = compiler.convert_matrix_if_i32(
		train_features,
		full_feature_shape.clone(),
		IterationDomain::first(),
	)?;
	let prepared_targets = match task {
		DenseTask::MulticlassClassification { .. } => train_targets,
		DenseTask::BinaryClassification { .. } | DenseTask::ScalarRegression { .. } => {
			compiler.convert_matrix_if_i32(train_targets, full_target_shape, IterationDomain::first())?
		}
	};
	let validation_features = validation_inputs
		.as_ref()
		.map(|(features, _, rows)| -> TrainingCompileResult<_> {
			let feature_shape = shape(&[*rows, feature_columns])?;
			Ok((
				compiler.convert_matrix_if_i32(*features, feature_shape.clone(), IterationDomain::first())?,
				feature_shape,
			))
		})
		.transpose()?;
	let (normalized_features, normalization, normalized_validation_features) = match config.data_normalization {
		DenseDataNormalization::ZScore => {
			let training = compiler.z_score(
				converted_features,
				full_feature_shape,
				feature_columns,
				train_rows,
				config.normalization_epsilon,
				config.reduction_tree_lanes,
				feature_normalization_mask,
			)?;
			let validation = validation_features
				.map(|(features, feature_shape)| {
					compiler.apply_z_score(
						features,
						training.mean,
						training.variance,
						feature_shape,
						config.normalization_epsilon,
						IterationDomain::first(),
						feature_normalization_mask,
					)
				})
				.transpose()?;
			(
				training.normalized,
				DataNormalizationState::ZScore(ZScoreState {
					mean: training.mean,
					variance: training.variance,
				}),
				validation,
			)
		}
		DenseDataNormalization::MinMax => {
			let training = compiler.min_max(
				converted_features,
				full_feature_shape,
				feature_columns,
				config.normalization_epsilon,
				config.reduction_tree_lanes,
				feature_normalization_mask,
			)?;
			let validation = validation_features
				.map(|(features, feature_shape)| {
					compiler.apply_min_max(
						features,
						training.minimum,
						training.maximum,
						feature_shape,
						config.normalization_epsilon,
						IterationDomain::first(),
						feature_normalization_mask,
					)
				})
				.transpose()?;
			(
				training.normalized,
				DataNormalizationState::MinMax(MinMaxState {
					minimum: training.minimum,
					maximum: training.maximum,
				}),
				validation,
			)
		}
		DenseDataNormalization::L2Norm => {
			let training = compiler.l2_norm(
				converted_features,
				full_feature_shape,
				config.normalization_epsilon,
				config.reduction_tree_lanes,
				IterationDomain::first(),
				feature_normalization_mask,
			)?;
			let validation = validation_features
				.map(|(features, feature_shape)| {
					compiler.l2_norm(
						features,
						feature_shape,
						config.normalization_epsilon,
						config.reduction_tree_lanes,
						IterationDomain::first(),
						feature_normalization_mask,
					)
				})
				.transpose()?;
			(training, DataNormalizationState::L2Norm, validation)
		}
	};
	let validation_values = match (validation_inputs, normalized_validation_features) {
		(Some((_, targets, rows)), Some(features)) => {
			let target_shape = shape(&[rows, target_code_columns])?;
			let targets = match task {
				DenseTask::MulticlassClassification { .. } => targets,
				DenseTask::BinaryClassification { .. } | DenseTask::ScalarRegression { .. } => {
					compiler.convert_matrix_if_i32(targets, target_shape, IterationDomain::first())?
				}
			};
			Some(ValidationValues {
				features,
				targets,
				rows,
			})
		}
		(None, None) => None,
		_ => {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				"validation feature normalization did not preserve the validation partition",
			));
		}
	};

	let index = compiler.tensor(DType::I32, shape(&[batch])?)?;
	compiler.emit(
		Vec::new(),
		vec![index],
		PrimitiveKind::IndexMap(IndexMap {
			start: 0,
			element_step: 1,
			iteration_step: checked_i32(batch, "batch size")?,
			modulus: Some(checked_i32(
				bounds.padded_rows_per_epoch,
				"padded rows per epoch",
			)?),
		}),
		Vec::new(),
		compiler.training_domain,
	)?;
	let mask_index = compiler.tensor(DType::I32, batch_row_shape.clone())?;
	compiler.emit(
		Vec::new(),
		vec![mask_index],
		PrimitiveKind::IndexMap(IndexMap {
			start: 0,
			element_step: 1,
			iteration_step: checked_i32(batch, "batch size")?,
			modulus: Some(checked_i32(
				bounds.padded_rows_per_epoch,
				"padded rows per epoch",
			)?),
		}),
		Vec::new(),
		compiler.training_domain,
	)?;
	let validity = compiler.tensor(DType::F32, batch_row_shape.clone())?;
	compiler.emit_elementwise(
		vec![mask_index],
		vec![validity],
		validity_program(checked_i32(train_rows, "train rows")?)?,
		compiler.training_domain,
	)?;
	let valid_count = compiler.tensor(DType::F32, scalar_shape.clone())?;
	compiler.reduce_sum(
		validity,
		valid_count,
		&[0, 1],
		config.reduction_tree_lanes,
		compiler.training_domain,
	)?;
	let (supervision, supervised_count, update_signal) =
		if let Some(train_target_supervision) = train_target_supervision {
			let batch_target_supervision = compiler.tensor(DType::F32, batch_row_shape.clone())?;
			compiler.emit(
				vec![train_target_supervision, index],
				vec![batch_target_supervision],
				PrimitiveKind::Gather(Gather {
					axis: 0,
					bounds: IndexBounds::Clamp,
				}),
				forbidden_aliases(2, 1),
				compiler.training_domain,
			)?;
			let supervision = compiler.tensor(DType::F32, batch_row_shape.clone())?;
			compiler.emit_elementwise(
				vec![validity, batch_target_supervision],
				vec![supervision],
				multiply_program()?,
				compiler.training_domain,
			)?;
			let known_count = compiler.tensor(DType::F32, scalar_shape.clone())?;
			compiler.reduce_sum(
				supervision,
				known_count,
				&[0, 1],
				config.reduction_tree_lanes,
				compiler.training_domain,
			)?;
			let safe_known_count = compiler.tensor(DType::F32, scalar_shape.clone())?;
			compiler.emit_elementwise(
				vec![known_count],
				vec![safe_known_count],
				safe_count_program()?,
				compiler.training_domain,
			)?;
			(supervision, safe_known_count, Some(known_count))
		} else {
			(
				validity,
				valid_count,
				accepted_update_plan.map(|_| valid_count),
			)
		};

	let batch_features = compiler.tensor(DType::F32, batch_feature_shape.clone())?;
	compiler.emit(
		vec![normalized_features, index],
		vec![batch_features],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Clamp,
		}),
		forbidden_aliases(2, 1),
		compiler.training_domain,
	)?;
	let batch_targets = compiler.tensor(
		if matches!(task, DenseTask::MulticlassClassification { .. }) {
			DType::I32
		} else {
			DType::F32
		},
		batch_target_shape,
	)?;
	compiler.emit(
		vec![prepared_targets, index],
		vec![batch_targets],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Clamp,
		}),
		forbidden_aliases(2, 1),
		compiler.training_domain,
	)?;

	let (current, _, block_values) = compiler.compile_training_blocks(
		&blocks,
		batch_features,
		feature_columns,
		batch,
		supervision,
		supervised_count,
		config,
	)?;
	let safe_current = compiler.mask_f32_with_zero(current, supervision, compiler.training_domain)?;
	let safe_batch_targets = if matches!(task, DenseTask::MulticlassClassification { .. }) {
		compiler.mask_i32_with_zero(batch_targets, supervision, compiler.training_domain)?
	} else {
		compiler.mask_f32_with_zero(batch_targets, supervision, compiler.training_domain)?
	};

	let losses = compiler.tensor(DType::F32, batch_output_shape.clone())?;
	let loss_gradient = compiler.tensor(DType::F32, batch_output_shape.clone())?;
	match config.loss {
		DenseLoss::BinaryCrossEntropy => {
			compiler.materialize(
				"gpu_bce_with_logits",
				&[("logits", safe_current), ("targets", safe_batch_targets)],
				&[("losses", losses), ("gradients", loss_gradient)],
				"logits",
				&PreparedParameters::new(),
				compiler.training_domain,
			)?;
		}
		DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber => {
			compiler.emit_elementwise(
				vec![safe_current, safe_batch_targets],
				vec![losses, loss_gradient],
				pointwise_loss_program(config.loss)?,
				compiler.training_domain,
			)?;
		}
		DenseLoss::CrossEntropy => {
			compiler.cross_entropy_with_logits(
				safe_current,
				safe_batch_targets,
				losses,
				loss_gradient,
				batch,
				output_columns,
				config.reduction_tree_lanes,
				compiler.training_domain,
			)?;
		}
	}
	let normalized_gradient = compiler.tensor(DType::F32, batch_output_shape.clone())?;
	compiler.emit_elementwise(
		vec![loss_gradient, supervision, supervised_count],
		vec![normalized_gradient],
		masked_mean_program()?,
		compiler.training_domain,
	)?;
	let masked_losses = compiler.tensor(DType::F32, batch_output_shape)?;
	compiler.emit_elementwise(
		vec![losses, supervision],
		vec![masked_losses],
		masked_zero_f32_program()?,
		compiler.training_domain,
	)?;
	let loss_sum = compiler.tensor(DType::F32, scalar_shape.clone())?;
	compiler.reduce_sum(
		masked_losses,
		loss_sum,
		&[0, 1],
		config.reduction_tree_lanes,
		compiler.training_domain,
	)?;
	let batch_loss = compiler.tensor(DType::F32, scalar_shape.clone())?;
	compiler.emit_elementwise(
		vec![loss_sum, supervised_count],
		vec![batch_loss],
		divide_program()?,
		compiler.training_domain,
	)?;
	let gradients = compiler.backward_blocks(
		&block_values,
		normalized_gradient,
		batch,
		supervision,
		supervised_count,
		config.normalization_epsilon,
		config.reduction_tree_lanes,
	)?;
	let parameter_gradients = flatten_parameter_gradients(&gradients);
	let flattened_gradients = parameter_gradients
		.iter()
		.map(|gradient| gradient.value)
		.collect::<Vec<_>>();
	let clipped = compiler.global_clip(
		&flattened_gradients,
		config.gradient_clip_norm,
		config.reduction_tree_lanes,
	)?;

	let early_stopping_initial = validation_available
		.then_some(validation_config)
		.flatten()
		.and_then(BinaryValidationConfig::early_stopping_patience)
		.map(|_| compiler.initialize_early_stopping_state())
		.transpose()?;
	let (learning_rate, beta_one_power, beta_two_power, optimizer_progress) = compiler.dynamic_adam_scalars(
		config,
		&bounds,
		early_stopping_initial.map(|state| state.stopped),
		update_signal.zip(accepted_update_plan),
	)?;
	let block_states = compiler.update_blocks(
		&block_values,
		&parameter_gradients,
		&clipped,
		learning_rate,
		beta_one_power,
		beta_two_power,
		optimizer_progress.map(|progress| progress.apply_update),
		config,
	)?;
	let layer_states = block_states
		.iter()
		.map(|state| match state {
			DenseBlockState::Layer(state) => Some(*state),
			DenseBlockState::Pool(_) | DenseBlockState::Residual(_) => None,
		})
		.collect::<Option<Vec<_>>>()
		.unwrap_or_default();
	let validation = match (validation_config, validation_values) {
		(Some(validation_config), Some(validation_values)) => Some(compiler.compile_validation(
			validation_values,
			config,
			&blocks,
			validation_config,
			&block_states,
			early_stopping_initial,
			&bounds,
		)?),
		(Some(_), None)
			if matches!(
				validation_status,
				ValidationMetricStatus::Unavailable { .. }
			) =>
		{
			None
		}
		(Some(_), None) => {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				"validation was requested without validation values",
			));
		}
		(None, _) => None,
	};
	let multiclass_validation = match (multiclass_validation_config, validation_values) {
		(Some(_), Some(validation_values)) => Some(compiler.compile_multiclass_validation(
			validation_values,
			config,
			&blocks,
			&block_states,
			&bounds,
		)?),
		(Some(_), None)
			if matches!(
				validation_status,
				ValidationMetricStatus::Unavailable { .. }
			) =>
		{
			None
		}
		(Some(_), None) => {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				"multiclass validation was requested without validation values",
			));
		}
		(None, _) => None,
	};

	let batch_loss_domain = compiler.training_domain;
	let metric_bindings = training_metric_bindings(
		batch_loss,
		batch_loss_domain,
		validation.as_ref(),
		multiclass_validation.as_ref(),
	)?;
	match normalization {
		DataNormalizationState::ZScore(state) => {
			compiler
				.external_outputs
				.extend([state.mean, state.variance]);
		}
		DataNormalizationState::MinMax(state) => {
			compiler
				.external_outputs
				.extend([state.minimum, state.maximum]);
		}
		DataNormalizationState::L2Norm => {}
	}
	compiler.finish(
		bounds,
		TrainingOutputs {
			batch_loss,
			batch_loss_domain,
			normalization,
			optimizer_progress,
			blocks: block_states,
			layers: layer_states,
			validation,
			multiclass_validation,
			validation_status,
			metric_bindings,
		},
		dataset_schema,
		config.clone(),
		blocks,
		layers,
		output_adapter,
	)
}

fn resolve_dense_task(dataset: &PreparedDataset, loss: DenseLoss) -> TrainingCompileResult<DenseTask> {
	let targets = dataset
		.vectors()
		.iter()
		.filter(|vector| vector.role() == VectorRole::Target)
		.collect::<Vec<_>>();
	if targets.len() != 1 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			format!(
				"dense training requires exactly one typed target vector, got {}",
				targets.len()
			),
		));
	}
	let target = targets[0];
	let classified = format!("{:?}/{:?}", target.semantic_type(), target.encoding());
	match loss {
		DenseLoss::BinaryCrossEntropy => {
			let positive_code = match (target.semantic_type(), target.encoding(), target.metadata()) {
				(SemanticType::Numeric, VectorEncoding::I32 | VectorEncoding::F32, _) => Some(1),
				(
					SemanticType::Categorical,
					VectorEncoding::DictionaryI32,
					recipe_ingest::VectorMetadata::Categorical { dictionary },
				) if dictionary.len() <= 2 => Some(match dictionary.len() {
					0 => -1,
					1 => 0,
					2 => 1,
					_ => unreachable!(),
				}),
				_ => None,
			};
			let Some(positive_code) = positive_code else {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					format!(
						"target {:?} classified {classified} is not an explicit numeric 0/1 or at-most-two-category BCE target",
						String::from_utf8_lossy(target.name())
					),
				));
			};
			Ok(DenseTask::BinaryClassification {
				target_vector: target.source_index(),
				positive_code,
			})
		}
		DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber => {
			if target.semantic_type() != SemanticType::Numeric
				|| !matches!(target.encoding(), VectorEncoding::I32 | VectorEncoding::F32)
			{
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					format!(
						"target {:?} classified {classified} is incompatible with scalar regression",
						String::from_utf8_lossy(target.name())
					),
				));
			}
			Ok(DenseTask::ScalarRegression {
				target_vector: target.source_index(),
			})
		}
		DenseLoss::CrossEntropy => {
			let (
				SemanticType::Categorical,
				VectorEncoding::DictionaryI32,
				recipe_ingest::VectorMetadata::Categorical { dictionary },
			) = (target.semantic_type(), target.encoding(), target.metadata())
			else {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidTargetMatrix,
					format!(
						"target {:?} classified {classified} is incompatible with categorical cross entropy",
						String::from_utf8_lossy(target.name())
					),
				));
			};
			let class_count = dictionary.len().checked_add(1).ok_or_else(|| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::ArithmeticOverflow,
					"categorical class count overflowed while adding the reserved unseen-label route",
				)
			})?;
			let reserved_code = i32::try_from(dictionary.len()).map_err(|error| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::UnsupportedExtent,
					format!("categorical reserved code cannot be represented by i32: {error}"),
				)
			})?;
			Ok(DenseTask::MulticlassClassification {
				target_vector: target.source_index(),
				class_count,
				reserved_code,
			})
		}
	}
}

fn validate_lowered_dataset(dataset: &LoweredDenseDataset, task: DenseTask) -> TrainingCompileResult<()> {
	for partition in core::iter::once(dataset.train()).chain(dataset.validation()) {
		if partition.targets().columns() != 1 {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!(
					"dense task requires one classified target-code column, got {}",
					partition.targets().columns(),
				),
			));
		}
		validate_exact_i32_to_f32(partition.features(), "feature")?;
		match task {
			DenseTask::BinaryClassification { .. } => {
				validate_exact_i32_to_f32(partition.targets(), "target")?;
				validate_binary_targets(partition.targets())?;
			}
			DenseTask::ScalarRegression { .. } => {
				validate_exact_i32_to_f32(partition.targets(), "target")?;
			}
			DenseTask::MulticlassClassification { .. } => {
				if !matches!(partition.targets(), DenseMatrix::I32 { .. }) {
					return Err(TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidTargetMatrix,
						"multiclass categorical targets must retain dictionary int32 codes",
					));
				}
			}
		}
	}
	Ok(())
}

fn validate_exact_i32_to_f32(matrix: &DenseMatrix, role: &str) -> TrainingCompileResult<()> {
	let DenseMatrix::I32 { values, .. } = matrix else {
		return Ok(());
	};
	if let Some(value) = values
		.iter()
		.copied()
		.find(|value| f64::from(*value as f32) != f64::from(*value))
	{
		return Err(TrainingCompileError::new(
			if role == "feature" {
				TrainingCompileErrorKind::InvalidFeatureMatrix
			} else {
				TrainingCompileErrorKind::InvalidTargetMatrix
			},
			format!("{role} int32 value {value} cannot be represented exactly by dense f32 calculation"),
		));
	}
	Ok(())
}

fn validate_config(config: &DenseTrainingConfig, blocks: &[DenseBlock]) -> TrainingCompileResult<()> {
	if blocks.is_empty() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"dense network requires at least one layer",
		));
	}
	for (block_index, block) in blocks.iter().enumerate() {
		match block {
			DenseBlock::Layer(_) => {}
			DenseBlock::Pool(pool) => {
				if let Some(neurons) = pool.group_to_neuron()
					&& !matches!(
						blocks.get(block_index + 1),
						Some(DenseBlock::Layer(layer)) if layer.width() == neurons
					) {
					return Err(TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidNetwork,
						format!(
							"pool block {block_index} routes to {} neurons but has no matching immediate layer",
							neurons.get()
						),
					));
				}
			}
			DenseBlock::Residual(residual) if residual.output_width().is_none() => {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidNetwork,
					format!("residual block {block_index} requires at least one layer operation"),
				));
			}
			DenseBlock::Residual(_) => {}
		}
	}
	if config.warmup_epochs > config.epochs.get() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"warmup epochs cannot exceed total epochs",
		));
	}
	if config.reduction_tree_lanes == 0
		|| config.reduction_tree_lanes > 1024
		|| !config.reduction_tree_lanes.is_power_of_two()
	{
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"reduction tree lanes must be a power of two in 1..=1024",
		));
	}
	for (name, value, allow_zero) in [
		("gradient clip norm", config.gradient_clip_norm, false),
		("normalization epsilon", config.normalization_epsilon, false),
		("learning rate", config.adamw.learning_rate, false),
		("AdamW epsilon", config.adamw.epsilon, false),
		("weight decay", config.adamw.weight_decay, true),
	] {
		if !value.is_finite() || value < 0.0 || (!allow_zero && value == 0.0) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				format!(
					"{name} must be finite and {}",
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
		("beta one", config.adamw.beta_one),
		("beta two", config.adamw.beta_two),
	] {
		if !value.is_finite() || !(0.0..1.0).contains(&value) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				format!("{name} must be finite and in [0, 1)"),
			));
		}
	}
	Ok(())
}

fn effective_blocks(
	declared: &[DenseBlock],
	task: DenseTask,
	input_width: u64,
) -> TrainingCompileResult<(Vec<DenseBlock>, Option<DenseOutputAdapter>)> {
	let mut blocks = declared.to_vec();
	let output = blocks.last().ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"dense network requires at least one layer",
		)
	})?;
	let target_width = NonZeroU64::new(u64::try_from(task.output_width()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("task output width cannot be represented by u64: {error}"),
		)
	})?)
	.ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"task output width must be nonzero",
		)
	})?;
	let mut logical = LogicalFeatureShape::from_width(input_width)?;
	for block in &blocks {
		logical = match block {
			DenseBlock::Layer(layer) => LogicalFeatureShape::from_width(layer.width().get())?,
			DenseBlock::Pool(pool) => logical.pooled(*pool)?.0,
			DenseBlock::Residual(residual) => LogicalFeatureShape::from_width(
				residual
					.output_width()
					.ok_or_else(|| {
						TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"final residual block requires at least one branch layer",
						)
					})?
					.get(),
			)?,
		};
	}
	let source_width = NonZeroU64::new(logical.width()?).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"effective model produced zero output width",
		)
	})?;
	let output_is_logits = !matches!(output, DenseBlock::Pool(_))
		&& output
			.output_operations()
			.iter()
			.all(|operation| *operation == DenseOperation::Activation(DenseActivation::Linear));
	let requires_logits = matches!(
		task,
		DenseTask::BinaryClassification { .. } | DenseTask::MulticlassClassification { .. }
	);
	let needs_adapter = matches!(output, DenseBlock::Pool(_))
		|| source_width != target_width
		|| (requires_logits && !output_is_logits);
	if !needs_adapter {
		return Ok((blocks, None));
	}
	let adapter = DenseOutputAdapter::new(source_width, target_width);
	blocks.push(DenseBlock::Layer(DenseLayer::new(
		target_width,
		DenseActivation::Linear,
	)));
	Ok((blocks, Some(adapter)))
}

fn flatten_parameter_gradients(blocks: &[BlockGradients]) -> Vec<ParameterGradient> {
	let mut flattened = Vec::new();
	for block in blocks {
		match block {
			BlockGradients::Layer(gradient) => flattened.extend([
				ParameterGradient {
					role: ParameterRole::LayerWeight,
					value: gradient.weight,
				},
				ParameterGradient {
					role: ParameterRole::LayerBias,
					value: gradient.bias,
				},
			]),
			BlockGradients::Pool => {}
			BlockGradients::Residual { branch, projection } => {
				for gradient in branch {
					flattened.extend([
						ParameterGradient {
							role: ParameterRole::LayerWeight,
							value: gradient.weight,
						},
						ParameterGradient {
							role: ParameterRole::LayerBias,
							value: gradient.bias,
						},
					]);
				}
				if let Some(projection) = projection {
					flattened.push(ParameterGradient {
						role: ParameterRole::ResidualProjectionWeight,
						value: *projection,
					});
				}
			}
		}
	}
	flattened
}

fn take_clipped_gradient(
	parameter_gradients: &[ParameterGradient],
	clipped: &[ValueId],
	cursor: &mut usize,
	expected: ParameterRole,
) -> TrainingCompileResult<ValueId> {
	let parameter = parameter_gradients.get(*cursor).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"optimizer parameter traversal ended before the model tape",
		)
	})?;
	if parameter.role != expected {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			format!(
				"optimizer parameter role {:?} disagrees with expected {expected:?}",
				parameter.role
			),
		));
	}
	let gradient = clipped.get(*cursor).copied().ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"clipped gradient traversal ended before the model tape",
		)
	})?;
	*cursor = cursor.checked_add(1).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"optimizer parameter cursor overflowed usize",
		)
	})?;
	Ok(gradient)
}

fn validate_validation_config(
	dataset: &LoweredDenseDataset,
	task: DenseTask,
	loss: DenseLoss,
	binary: Option<&BinaryValidationConfig>,
	multiclass: Option<&MulticlassValidationConfig>,
) -> TrainingCompileResult<ValidationMetricStatus> {
	if binary.is_some() && multiclass.is_some() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"binary and multiclass validation cannot be requested for one target",
		));
	}
	if multiclass.is_some() {
		if loss != DenseLoss::CrossEntropy || !matches!(task, DenseTask::MulticlassClassification { .. }) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"multiclass validation requires a categorical cross-entropy target",
			));
		}
		if dataset.validation_split_rows() == 0 {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				"multiclass validation metrics require a prepared validation partition",
			));
		}
		return validation_metric_status(dataset, ValidationMetricFamily::Multiclass);
	}
	let Some(config) = binary else {
		return Ok(ValidationMetricStatus::NotRequested);
	};
	if loss != DenseLoss::BinaryCrossEntropy || !matches!(task, DenseTask::BinaryClassification { .. }) {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"binary classification validation is available only with BCE",
		));
	}
	if dataset.validation_split_rows() == 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"validation metrics require a prepared validation partition",
		));
	}
	let thresholds = config.recall_thresholds().collect::<Vec<_>>();
	if let Some((index, threshold)) = thresholds
		.iter()
		.copied()
		.enumerate()
		.find(|(_, threshold)| !threshold.is_finite() || !(0.0..=1.0).contains(threshold))
	{
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			format!("validation recall threshold {index} ({threshold}) must be finite and in [0, 1]"),
		));
	}
	let distinct = thresholds
		.iter()
		.map(|threshold| threshold.to_bits())
		.collect::<BTreeSet<_>>();
	if distinct.len() != thresholds.len() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"validation recall thresholds must have distinct f32 bit patterns",
		));
	}
	if let Some(validation) = dataset.validation() {
		binary_metric_requirements(
			u64::try_from(validation.rows()).map_err(|error| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::UnsupportedExtent,
					format!("validation rows cannot be represented by u64: {error}"),
				)
			})?,
			thresholds.len(),
			config.calibration_bins().get(),
		)?;
	}
	if let Some(temperature) = config.temperature_scaling() {
		if !temperature.learning_rate.is_finite() || temperature.learning_rate <= 0.0 {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				"temperature-scaling learning rate must be finite and positive",
			));
		}
		if !temperature.minimum_temperature.is_finite()
			|| !temperature.maximum_temperature.is_finite()
			|| temperature.minimum_temperature <= 0.0
			|| temperature.minimum_temperature >= temperature.maximum_temperature
		{
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				"temperature bounds must be finite, positive, and strictly increasing",
			));
		}
	}
	binary_validation_metric_status(dataset)
}

fn binary_validation_metric_status(dataset: &LoweredDenseDataset) -> TrainingCompileResult<ValidationMetricStatus> {
	let status = validation_metric_status(dataset, ValidationMetricFamily::Binary)?;
	let ValidationMetricStatus::Available { known_rows, .. } = status else {
		return Ok(status);
	};
	let validation = dataset.validation().ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"available binary validation omitted its known target rows",
		)
	})?;
	let (has_zero, has_one) = match validation.targets() {
		DenseMatrix::I32 { values, .. } => (
			values.iter().any(|value| *value == 0),
			values.iter().any(|value| *value == 1),
		),
		DenseMatrix::F32Bits { values, .. } => (
			values.iter().any(|bits| f32::from_bits(*bits) == 0.0),
			values.iter().any(|bits| f32::from_bits(*bits) == 1.0),
		),
	};
	if has_zero && has_one {
		return Ok(status);
	}
	let split_rows = u64::try_from(dataset.validation_split_rows()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("validation split rows cannot be represented by u64: {error}"),
		)
	})?;
	Ok(ValidationMetricStatus::Unavailable {
		family: ValidationMetricFamily::Binary,
		reason: ValidationUnavailableReason::SingleKnownClass { known_rows },
		split_rows,
	})
}

fn validation_metric_status(
	dataset: &LoweredDenseDataset,
	family: ValidationMetricFamily,
) -> TrainingCompileResult<ValidationMetricStatus> {
	let split_rows = u64::try_from(dataset.validation_split_rows()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("validation split rows cannot be represented by u64: {error}"),
		)
	})?;
	let Some(validation) = dataset.validation() else {
		return Ok(ValidationMetricStatus::Unavailable {
			family,
			reason: ValidationUnavailableReason::NoKnownTargets,
			split_rows,
		});
	};
	let known_rows = u64::try_from(validation.rows())
		.ok()
		.and_then(NonZeroU64::new)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				"known validation target rows cannot be represented as nonzero u64",
			)
		})?;
	Ok(ValidationMetricStatus::Available { family, known_rows })
}

fn training_bounds(
	rows: usize,
	batch_size: NonZeroUsize,
	epochs: NonZeroU64,
	warmup_epochs: u64,
	calibration_iterations: u64,
) -> TrainingCompileResult<TrainingBounds> {
	let train_rows = u64::try_from(rows).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("training row count cannot be represented by u64: {error}"),
		)
	})?;
	let batch_size = u64::try_from(batch_size.get()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("batch size cannot be represented by u64: {error}"),
		)
	})?;
	let batches_per_epoch = train_rows
		.checked_add(batch_size - 1)
		.and_then(|value| value.checked_div(batch_size))
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"batches-per-epoch calculation overflowed",
			)
		})?;
	let padded_rows_per_epoch = batches_per_epoch.checked_mul(batch_size).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"padded epoch extent overflowed",
		)
	})?;
	let training_iterations = batches_per_epoch.checked_mul(epochs.get()).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"training iteration count overflowed",
		)
	})?;
	let training_iterations = NonZeroU64::new(training_iterations).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::EmptyDataset,
			"training requires at least one batch",
		)
	})?;
	if training_iterations.get() > i32::MAX as u64 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			"GPU-derived training schedule currently requires at most i32::MAX training iterations",
		));
	}
	let iterations = training_iterations
		.get()
		.checked_add(calibration_iterations)
		.and_then(NonZeroU64::new)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"training plus calibration iteration count overflowed",
			)
		})?;
	if padded_rows_per_epoch > i32::MAX as u64 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			"padded rows per epoch exceed the int32 IndexMap domain",
		));
	}
	let warmup_iterations = batches_per_epoch
		.checked_mul(warmup_epochs)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"warmup iteration count overflowed",
			)
		})?;
	Ok(TrainingBounds {
		train_rows,
		batch_size,
		batches_per_epoch,
		padded_rows_per_epoch,
		epochs,
		training_iterations,
		calibration_iterations,
		iterations,
		warmup_iterations,
	})
}

fn accepted_update_plan(
	accepted_updates_per_epoch: usize,
	config: &DenseTrainingConfig,
	bounds: &TrainingBounds,
) -> TrainingCompileResult<AcceptedUpdatePlan> {
	let per_epoch = u64::try_from(accepted_updates_per_epoch).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("accepted updates per epoch cannot be represented by u64: {error}"),
		)
	})?;
	if per_epoch > bounds.batches_per_epoch {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"accepted updates per epoch exceed physical batches per epoch",
		));
	}
	let maximum = per_epoch.checked_mul(config.epochs.get()).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"maximum accepted optimizer update count overflowed u64",
		)
	})?;
	let warmup = per_epoch.checked_mul(config.warmup_epochs).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"warmup accepted optimizer update count overflowed u64",
		)
	})?;
	if maximum > bounds.training_iterations.get() || warmup > maximum {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"accepted optimizer update bounds exceed physical training bounds",
		));
	}
	Ok(AcceptedUpdatePlan {
		per_epoch,
		maximum,
		warmup,
	})
}

#[derive(Debug)]
struct GraphCompiler {
	tensors: BTreeMap<ValueId, Tensor>,
	nodes: Vec<CalculationNode>,
	domains: Vec<KernelIterationDomain>,
	next_value: u64,
	next_kernel: u64,
	next_weight_stream: u64,
	iterations: recipe_core::LoopIterations,
	training_domain: IterationDomain,
	external_inputs: Vec<OwnedExternalInput>,
	external_input_ids: BTreeSet<ValueId>,
	external_outputs: BTreeSet<ValueId>,
}

impl GraphCompiler {
	fn new(iterations: NonZeroU64, training_iterations: NonZeroU64) -> TrainingCompileResult<Self> {
		let training_domain = IterationDomain::new(0, training_iterations.get(), 1).ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				"training iteration domain must be nonempty",
			)
		})?;
		Ok(Self {
			tensors: BTreeMap::new(),
			nodes: Vec::new(),
			domains: Vec::new(),
			next_value: 1,
			next_kernel: 1,
			next_weight_stream: 0,
			iterations: recipe_core::LoopIterations::from(iterations),
			training_domain,
			external_inputs: Vec::new(),
			external_input_ids: BTreeSet::new(),
			external_outputs: BTreeSet::new(),
		})
	}

	fn tensor(&mut self, dtype: DType, shape: Shape) -> TrainingCompileResult<ValueId> {
		let value = self.next_value()?;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false)?;
		self.tensors.insert(value, tensor);
		Ok(value)
	}

	fn external_matrix(&mut self, role: ExternalInputRole, matrix: &DenseMatrix) -> TrainingCompileResult<ValueId> {
		let rows = u64::try_from(matrix.rows()).map_err(|error| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} rows cannot be represented by u64: {error}"),
			)
		})?;
		let columns = u64::try_from(matrix.columns()).map_err(|error| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} columns cannot be represented by u64: {error}"),
			)
		})?;
		let shape = shape(&[rows, columns])?;
		let (dtype, bytes) = matrix_bytes(matrix);
		let expected_bytes = shape.bytes(dtype)?.get();
		if u64::try_from(bytes.len()).ok() != Some(expected_bytes) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!(
					"{role:?} encodes {} bytes, tensor contract requires {expected_bytes}",
					bytes.len()
				),
			));
		}
		let value = self.tensor(dtype, shape.clone())?;
		self.external_input_ids.insert(value);
		self.external_inputs
			.push(OwnedExternalInput::new(role, value, dtype, shape, bytes));
		Ok(value)
	}

	fn external_f32_vector(&mut self, role: ExternalInputRole, values: &[u32]) -> TrainingCompileResult<ValueId> {
		if values.is_empty() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!("{role:?} cannot be empty"),
			));
		}
		if let Some(bits) = values
			.iter()
			.copied()
			.find(|bits| !f32::from_bits(*bits).is_finite())
		{
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				format!("{role:?} contains non-finite f32 bits {bits:#010x}"),
			));
		}
		let columns = u64::try_from(values.len()).map_err(|error| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("{role:?} width cannot be represented by u64: {error}"),
			)
		})?;
		let shape = shape(&[columns])?;
		let bytes = values
			.iter()
			.flat_map(|bits| bits.to_le_bytes())
			.collect::<Vec<_>>();
		let value = self.tensor(DType::F32, shape.clone())?;
		self.external_input_ids.insert(value);
		self.external_inputs.push(OwnedExternalInput::new(
			role,
			value,
			DType::F32,
			shape,
			bytes,
		));
		Ok(value)
	}

	fn external_i32_tensor(
		&mut self,
		role: ExternalInputRole,
		tensor_shape: Shape,
		values: &[i32],
	) -> TrainingCompileResult<ValueId> {
		if u64::try_from(values.len()).ok() != Some(tensor_shape.elements()) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				format!(
					"{role:?} contains {} values, tensor shape requires {}",
					values.len(),
					tensor_shape.elements()
				),
			));
		}
		let bytes = values
			.iter()
			.flat_map(|value| value.to_le_bytes())
			.collect::<Vec<_>>();
		let value = self.tensor(DType::I32, tensor_shape.clone())?;
		self.external_input_ids.insert(value);
		self.external_inputs.push(OwnedExternalInput::new(
			role,
			value,
			DType::I32,
			tensor_shape,
			bytes,
		));
		Ok(value)
	}

	fn convert_matrix_if_i32(
		&mut self,
		input: ValueId,
		output_shape: Shape,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		if self.tensor_ref(input)?.dtype == DType::F32 {
			return Ok(input);
		}
		let output = self.tensor(DType::F32, output_shape)?;
		self.emit_elementwise(vec![input], vec![output], convert_i32_program()?, domain)?;
		Ok(output)
	}

	fn z_score(
		&mut self,
		input: ValueId,
		matrix_shape: Shape,
		columns: u64,
		rows: u64,
		epsilon: f32,
		tree_lanes: u32,
		normalization_mask: Option<ValueId>,
	) -> TrainingCompileResult<ZScoreValues> {
		let column_shape = shape(&[columns])?;
		let sums = self.tensor(DType::F32, column_shape.clone())?;
		self.reduce_sum(input, sums, &[0], tree_lanes, IterationDomain::first())?;
		let means = self.tensor(DType::F32, column_shape.clone())?;
		self.emit_elementwise(
			vec![sums],
			vec![means],
			divide_constant_program(rows as f32)?,
			IterationDomain::first(),
		)?;
		let centered = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![input, means],
			vec![centered],
			subtract_program()?,
			IterationDomain::first(),
		)?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![centered],
			vec![squares],
			square_program()?,
			IterationDomain::first(),
		)?;
		let variance_sums = self.tensor(DType::F32, column_shape.clone())?;
		self.reduce_sum(
			squares,
			variance_sums,
			&[0],
			tree_lanes,
			IterationDomain::first(),
		)?;
		let variances = self.tensor(DType::F32, column_shape)?;
		self.emit_elementwise(
			vec![variance_sums],
			vec![variances],
			divide_constant_program(rows as f32)?,
			IterationDomain::first(),
		)?;
		let normalized = self.apply_z_score(
			input,
			means,
			variances,
			matrix_shape,
			epsilon,
			IterationDomain::first(),
			normalization_mask,
		)?;
		Ok(ZScoreValues {
			normalized,
			mean: means,
			variance: variances,
		})
	}

	fn apply_z_score(
		&mut self,
		input: ValueId,
		mean: ValueId,
		variance: ValueId,
		output_shape: Shape,
		epsilon: f32,
		domain: IterationDomain,
		normalization_mask: Option<ValueId>,
	) -> TrainingCompileResult<ValueId> {
		let normalized = self.tensor(DType::F32, output_shape)?;
		let mut inputs = vec![input, mean, variance];
		if let Some(mask) = normalization_mask {
			inputs.push(mask);
		}
		self.emit_elementwise(
			inputs,
			vec![normalized],
			z_score_program(epsilon, normalization_mask.is_some())?,
			domain,
		)?;
		Ok(normalized)
	}

	fn min_max(
		&mut self,
		input: ValueId,
		matrix_shape: Shape,
		columns: u64,
		epsilon: f32,
		tree_lanes: u32,
		normalization_mask: Option<ValueId>,
	) -> TrainingCompileResult<MinMaxValues> {
		let column_shape = shape(&[columns])?;
		let minimum = self.tensor(DType::F32, column_shape.clone())?;
		self.reduce_value(
			input,
			minimum,
			ReduceOperator::Minimum,
			&[0],
			false,
			tree_lanes,
			IterationDomain::first(),
		)?;
		let maximum = self.tensor(DType::F32, column_shape)?;
		self.reduce_value(
			input,
			maximum,
			ReduceOperator::Maximum,
			&[0],
			false,
			tree_lanes,
			IterationDomain::first(),
		)?;
		let normalized = self.apply_min_max(
			input,
			minimum,
			maximum,
			matrix_shape,
			epsilon,
			IterationDomain::first(),
			normalization_mask,
		)?;
		Ok(MinMaxValues {
			normalized,
			minimum,
			maximum,
		})
	}

	fn apply_min_max(
		&mut self,
		input: ValueId,
		minimum: ValueId,
		maximum: ValueId,
		output_shape: Shape,
		epsilon: f32,
		domain: IterationDomain,
		normalization_mask: Option<ValueId>,
	) -> TrainingCompileResult<ValueId> {
		let normalized = self.tensor(DType::F32, output_shape)?;
		let mut inputs = vec![input, minimum, maximum];
		if let Some(mask) = normalization_mask {
			inputs.push(mask);
		}
		self.emit_elementwise(
			inputs,
			vec![normalized],
			min_max_program(epsilon, normalization_mask.is_some())?,
			domain,
		)?;
		Ok(normalized)
	}

	fn l2_norm(
		&mut self,
		input: ValueId,
		matrix_shape: Shape,
		epsilon: f32,
		tree_lanes: u32,
		domain: IterationDomain,
		normalization_mask: Option<ValueId>,
	) -> TrainingCompileResult<ValueId> {
		let rows = matrix_shape.extents().first().copied().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidFeatureMatrix,
				"L2 normalization requires a matrix",
			)
		})?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?;
		let mut square_inputs = vec![input];
		if let Some(mask) = normalization_mask {
			square_inputs.push(mask);
		}
		self.emit_elementwise(
			square_inputs,
			vec![squares],
			l2_square_program(normalization_mask.is_some())?,
			domain,
		)?;
		let norms_squared = self.tensor(DType::F32, shape(&[rows, 1])?)?;
		self.reduce_value(
			squares,
			norms_squared,
			ReduceOperator::Sum,
			&[1],
			true,
			tree_lanes,
			domain,
		)?;
		let normalized = self.tensor(DType::F32, matrix_shape)?;
		let mut normalize_inputs = vec![input, norms_squared];
		if let Some(mask) = normalization_mask {
			normalize_inputs.push(mask);
		}
		self.emit_elementwise(
			normalize_inputs,
			vec![normalized],
			l2_norm_program(epsilon, normalization_mask.is_some())?,
			domain,
		)?;
		Ok(normalized)
	}

	fn apply_activation(
		&mut self,
		input: ValueId,
		activation: DenseActivation,
		output_shape: Shape,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		if activation == DenseActivation::Linear {
			return Ok(input);
		}
		let output = self.tensor(DType::F32, output_shape.clone())?;
		match activation {
			DenseActivation::Linear => {}
			DenseActivation::Cosine => {
				self.emit_owned_scalar("gpu_cos", vec![input], vec![output], domain)?;
			}
			DenseActivation::Exponential => {
				self.emit_owned_scalar("gpu_exp", vec![input], vec![output], domain)?;
			}
			DenseActivation::Logarithm => {
				let absolute = self.tensor(DType::F32, output_shape.clone())?;
				self.emit_owned_scalar("gpu_abs_into", vec![input], vec![absolute], domain)?;
				let magnitude = self.tensor(DType::F32, output_shape.clone())?;
				self.emit_owned_scalar("gpu_log1p", vec![absolute], vec![magnitude], domain)?;
				let sign = self.tensor(DType::F32, output_shape)?;
				self.emit_owned_scalar("gpu_sign_into", vec![input], vec![sign], domain)?;
				self.emit_elementwise(
					vec![sign, magnitude],
					vec![output],
					multiply_program()?,
					domain,
				)?;
			}
			DenseActivation::Huber => {
				self.emit_elementwise(
					vec![input],
					vec![output],
					huber_activation_program()?,
					domain,
				)?;
			}
			DenseActivation::Tangent => {
				self.emit_owned_scalar("gpu_tan", vec![input], vec![output], domain)?;
			}
			DenseActivation::Relu => {
				self.emit_owned_scalar("gpu_relu_into", vec![input], vec![output], domain)?;
			}
			DenseActivation::Gelu => {
				self.emit_owned_scalar("gpu_gelu_into", vec![input], vec![output], domain)?;
			}
			DenseActivation::Silu => {
				self.emit_owned_scalar("gpu_silu_into", vec![input], vec![output], domain)?;
			}
		}
		Ok(output)
	}

	fn mask_f32_with_zero(
		&mut self,
		input: ValueId,
		validity: ValueId,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		let input_shape = {
			let input_tensor = self.tensor_ref(input)?;
			if input_tensor.dtype != DType::F32 {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidNetwork,
					"floating supervision masking requires an f32 input",
				));
			}
			input_tensor.shape.clone()
		};
		let output = self.tensor(DType::F32, input_shape)?;
		self.emit_elementwise(
			vec![input, validity],
			vec![output],
			masked_zero_f32_program()?,
			domain,
		)?;
		Ok(output)
	}

	fn mask_i32_with_zero(
		&mut self,
		input: ValueId,
		validity: ValueId,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		let input_shape = {
			let input_tensor = self.tensor_ref(input)?;
			if input_tensor.dtype != DType::I32 {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidNetwork,
					"integer supervision masking requires an i32 input",
				));
			}
			input_tensor.shape.clone()
		};
		let output = self.tensor(DType::I32, input_shape)?;
		self.emit_elementwise(
			vec![input, validity],
			vec![output],
			masked_zero_i32_program()?,
			domain,
		)?;
		Ok(output)
	}

	#[allow(clippy::too_many_arguments)]
	fn cross_entropy_with_logits(
		&mut self,
		logits: ValueId,
		targets: ValueId,
		losses: ValueId,
		gradients: ValueId,
		rows: u64,
		classes: u64,
		tree_lanes: u32,
		domain: IterationDomain,
	) -> TrainingCompileResult<()> {
		let matrix_shape = shape(&[rows, classes])?;
		let row_shape = shape(&[rows, 1])?;
		let classes_i32 = checked_i32(classes, "categorical class count")?;
		let class_indices = self.tensor(DType::I32, shape(&[1, classes])?)?;
		self.emit(
			Vec::new(),
			vec![class_indices],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let maximum = self.tensor(DType::F32, row_shape.clone())?;
		self.reduce_value(
			logits,
			maximum,
			ReduceOperator::Maximum,
			&[1],
			true,
			tree_lanes,
			domain,
		)?;
		let shifted = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![logits, maximum],
			vec![shifted],
			subtract_program()?,
			domain,
		)?;
		let exponentials = self.tensor(DType::F32, matrix_shape)?;
		self.emit_owned_scalar("gpu_exp", vec![shifted], vec![exponentials], domain)?;
		let exponential_sum = self.tensor(DType::F32, row_shape.clone())?;
		self.reduce_value(
			exponentials,
			exponential_sum,
			ReduceOperator::Sum,
			&[1],
			true,
			tree_lanes,
			domain,
		)?;
		let logarithmic_sum = self.tensor(DType::F32, row_shape)?;
		self.emit_owned_scalar(
			"gpu_log_into",
			vec![exponential_sum],
			vec![logarithmic_sum],
			domain,
		)?;
		self.emit_elementwise(
			vec![
				logits,
				targets,
				class_indices,
				maximum,
				logarithmic_sum,
				exponentials,
				exponential_sum,
			],
			vec![losses, gradients],
			cross_entropy_with_logits_program(classes_i32)?,
			domain,
		)?;
		Ok(())
	}

	fn normalize_training(
		&mut self,
		input: ValueId,
		normalization: DenseNormalization,
		rows: u64,
		columns: u64,
		validity: ValueId,
		valid_count: ValueId,
		epsilon: f32,
		tree_lanes: u32,
	) -> TrainingCompileResult<NormalizedValues> {
		match normalization {
			DenseNormalization::Layer => self.normalize_unmasked(
				input,
				rows,
				columns,
				1,
				columns as f32,
				true,
				epsilon,
				tree_lanes,
				self.training_domain,
			),
			DenseNormalization::Batch => self.normalize_masked_batch(
				input,
				rows,
				columns,
				validity,
				valid_count,
				epsilon,
				tree_lanes,
			),
		}
	}

	fn normalize_validation(
		&mut self,
		input: ValueId,
		normalization: DenseNormalization,
		rows: u64,
		columns: u64,
		epsilon: f32,
		tree_lanes: u32,
		domain: IterationDomain,
	) -> TrainingCompileResult<NormalizedValues> {
		match normalization {
			DenseNormalization::Layer => self.normalize_unmasked(
				input,
				rows,
				columns,
				1,
				columns as f32,
				true,
				epsilon,
				tree_lanes,
				domain,
			),
			DenseNormalization::Batch => self.normalize_unmasked(
				input,
				rows,
				columns,
				0,
				rows as f32,
				false,
				epsilon,
				tree_lanes,
				domain,
			),
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn normalize_unmasked(
		&mut self,
		input: ValueId,
		rows: u64,
		columns: u64,
		axis: usize,
		population: f32,
		keep_dimensions: bool,
		epsilon: f32,
		tree_lanes: u32,
		domain: IterationDomain,
	) -> TrainingCompileResult<NormalizedValues> {
		let matrix_shape = shape(&[rows, columns])?;
		let statistic_shape = if keep_dimensions {
			shape(&[rows, 1])?
		} else {
			shape(&[columns])?
		};
		let sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce_value(
			input,
			sums,
			ReduceOperator::Sum,
			&[axis],
			keep_dimensions,
			tree_lanes,
			domain,
		)?;
		let means = self.tensor(DType::F32, statistic_shape.clone())?;
		self.emit_elementwise(
			vec![sums],
			vec![means],
			divide_constant_program(population)?,
			domain,
		)?;
		let centered = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![input, means],
			vec![centered],
			subtract_program()?,
			domain,
		)?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(vec![centered], vec![squares], square_program()?, domain)?;
		let variance_sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce_value(
			squares,
			variance_sums,
			ReduceOperator::Sum,
			&[axis],
			keep_dimensions,
			tree_lanes,
			domain,
		)?;
		let variance = self.tensor(DType::F32, statistic_shape)?;
		self.emit_elementwise(
			vec![variance_sums],
			vec![variance],
			divide_constant_program(population)?,
			domain,
		)?;
		let normalized = self.apply_z_score(input, means, variance, matrix_shape, epsilon, domain, None)?;
		Ok(NormalizedValues {
			normalized,
			variance,
		})
	}

	#[allow(clippy::too_many_arguments)]
	fn normalize_masked_batch(
		&mut self,
		input: ValueId,
		rows: u64,
		columns: u64,
		validity: ValueId,
		valid_count: ValueId,
		epsilon: f32,
		tree_lanes: u32,
	) -> TrainingCompileResult<NormalizedValues> {
		let matrix_shape = shape(&[rows, columns])?;
		let statistic_shape = shape(&[columns])?;
		let safe_input = self.mask_f32_with_zero(input, validity, self.training_domain)?;
		let sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce_sum(safe_input, sums, &[0], tree_lanes, self.training_domain)?;
		let means = self.tensor(DType::F32, statistic_shape.clone())?;
		self.emit_elementwise(
			vec![sums, valid_count],
			vec![means],
			divide_program()?,
			self.training_domain,
		)?;
		let unmasked_centered = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![safe_input, means],
			vec![unmasked_centered],
			subtract_program()?,
			self.training_domain,
		)?;
		let centered = self.mask_f32_with_zero(unmasked_centered, validity, self.training_domain)?;
		let squares = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![centered],
			vec![squares],
			square_program()?,
			self.training_domain,
		)?;
		let masked_squares = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![squares, validity],
			vec![masked_squares],
			masked_zero_f32_program()?,
			self.training_domain,
		)?;
		let variance_sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce_sum(
			masked_squares,
			variance_sums,
			&[0],
			tree_lanes,
			self.training_domain,
		)?;
		let variance = self.tensor(DType::F32, statistic_shape)?;
		self.emit_elementwise(
			vec![variance_sums, valid_count],
			vec![variance],
			divide_program()?,
			self.training_domain,
		)?;
		let normalized = self.apply_z_score(
			safe_input,
			means,
			variance,
			matrix_shape,
			epsilon,
			self.training_domain,
			None,
		)?;
		let normalized = self.mask_f32_with_zero(normalized, validity, self.training_domain)?;
		Ok(NormalizedValues {
			normalized,
			variance,
		})
	}

	fn backward_activation(
		&mut self,
		gradient: ValueId,
		input: ValueId,
		output: ValueId,
		activation: DenseActivation,
	) -> TrainingCompileResult<ValueId> {
		if activation == DenseActivation::Linear {
			return Ok(gradient);
		}
		let output_shape = self.tensor_ref(gradient)?.shape.clone();
		let activation_gradient = self.tensor(DType::F32, output_shape.clone())?;
		match activation {
			DenseActivation::Linear => {}
			DenseActivation::Cosine => {
				let sine = self.tensor(DType::F32, output_shape)?;
				self.emit_owned_scalar("gpu_sin", vec![input], vec![sine], self.training_domain)?;
				self.emit_elementwise(
					vec![gradient, sine],
					vec![activation_gradient],
					negative_multiply_program()?,
					self.training_domain,
				)?;
			}
			DenseActivation::Exponential => {
				self.emit_elementwise(
					vec![gradient, output],
					vec![activation_gradient],
					multiply_program()?,
					self.training_domain,
				)?;
			}
			DenseActivation::Logarithm => {
				self.emit_elementwise(
					vec![gradient, input],
					vec![activation_gradient],
					signed_log_one_plus_backward_program()?,
					self.training_domain,
				)?;
			}
			DenseActivation::Huber => {
				self.emit_elementwise(
					vec![gradient, input],
					vec![activation_gradient],
					huber_activation_backward_program()?,
					self.training_domain,
				)?;
			}
			DenseActivation::Tangent => {
				self.emit_elementwise(
					vec![gradient, output],
					vec![activation_gradient],
					tangent_activation_backward_program()?,
					self.training_domain,
				)?;
			}
			DenseActivation::Relu => {
				self.emit_owned_scalar(
					"gpu_relu_backward_into",
					vec![gradient, input],
					vec![activation_gradient],
					self.training_domain,
				)?;
			}
			DenseActivation::Gelu => {
				self.emit_owned_scalar(
					"gpu_gelu_backward_into",
					vec![gradient, input],
					vec![activation_gradient],
					self.training_domain,
				)?;
			}
			DenseActivation::Silu => {
				self.emit_owned_scalar(
					"gpu_silu_backward_into",
					vec![gradient, input],
					vec![activation_gradient],
					self.training_domain,
				)?;
			}
		}
		Ok(activation_gradient)
	}

	#[allow(clippy::too_many_arguments)]
	fn backward_normalization(
		&mut self,
		gradient: ValueId,
		normalized: ValueId,
		variance: ValueId,
		normalization: DenseNormalization,
		validity: ValueId,
		valid_count: ValueId,
		epsilon: f32,
		tree_lanes: u32,
	) -> TrainingCompileResult<ValueId> {
		let matrix_shape = self.tensor_ref(gradient)?.shape.clone();
		let gradient = self.mask_f32_with_zero(gradient, validity, self.training_domain)?;
		let normalized = self.mask_f32_with_zero(normalized, validity, self.training_domain)?;
		let extents = matrix_shape.extents();
		let rows = extents.first().copied().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"normalization gradient is not rank two",
			)
		})?;
		let columns = extents.get(1).copied().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"normalization gradient is not rank two",
			)
		})?;
		let (axis, keep_dimensions, statistic_shape) = match normalization {
			DenseNormalization::Layer => (1, true, shape(&[rows, 1])?),
			DenseNormalization::Batch => (0, false, shape(&[columns])?),
		};
		let gradient_sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce_value(
			gradient,
			gradient_sums,
			ReduceOperator::Sum,
			&[axis],
			keep_dimensions,
			tree_lanes,
			self.training_domain,
		)?;
		let gradient_means = self.tensor(DType::F32, statistic_shape.clone())?;
		match normalization {
			DenseNormalization::Layer => {
				self.emit_elementwise(
					vec![gradient_sums],
					vec![gradient_means],
					divide_constant_program(columns as f32)?,
					self.training_domain,
				)?;
			}
			DenseNormalization::Batch => {
				self.emit_elementwise(
					vec![gradient_sums, valid_count],
					vec![gradient_means],
					divide_program()?,
					self.training_domain,
				)?;
			}
		}
		let gradient_products = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![gradient, normalized],
			vec![gradient_products],
			multiply_program()?,
			self.training_domain,
		)?;
		let product_sums = self.tensor(DType::F32, statistic_shape.clone())?;
		self.reduce_value(
			gradient_products,
			product_sums,
			ReduceOperator::Sum,
			&[axis],
			keep_dimensions,
			tree_lanes,
			self.training_domain,
		)?;
		let product_means = self.tensor(DType::F32, statistic_shape.clone())?;
		match normalization {
			DenseNormalization::Layer => {
				self.emit_elementwise(
					vec![product_sums],
					vec![product_means],
					divide_constant_program(columns as f32)?,
					self.training_domain,
				)?;
			}
			DenseNormalization::Batch => {
				self.emit_elementwise(
					vec![product_sums, valid_count],
					vec![product_means],
					divide_program()?,
					self.training_domain,
				)?;
			}
		}
		let inverse_standard_deviation = self.tensor(DType::F32, statistic_shape)?;
		self.emit_elementwise(
			vec![variance],
			vec![inverse_standard_deviation],
			inverse_standard_deviation_program(epsilon)?,
			self.training_domain,
		)?;
		let unmasked = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![
				gradient,
				gradient_means,
				normalized,
				product_means,
				inverse_standard_deviation,
			],
			vec![unmasked],
			normalization_backward_program()?,
			self.training_domain,
		)?;
		if normalization == DenseNormalization::Layer {
			return Ok(unmasked);
		}
		let masked = self.tensor(DType::F32, matrix_shape)?;
		self.emit_elementwise(
			vec![unmasked, validity],
			vec![masked],
			masked_zero_f32_program()?,
			self.training_domain,
		)?;
		Ok(masked)
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_training_blocks(
		&mut self,
		blocks: &[DenseBlock],
		input: ValueId,
		input_width: u64,
		batch: u64,
		validity: ValueId,
		valid_count: ValueId,
		config: &DenseTrainingConfig,
	) -> TrainingCompileResult<(ValueId, u64, Vec<BlockValues>)> {
		let mut current = input;
		let mut current_width = input_width;
		let mut logical = LogicalFeatureShape::from_width(input_width)?;
		let mut preceding_pool = None;
		let mut values = Vec::with_capacity(blocks.len());
		for (block_index, block) in blocks.iter().enumerate() {
			match block {
				DenseBlock::Layer(layer) => {
					let routing = preceding_pool
						.and_then(|pool: DensePool| pool.routing(logical.length))
						.map(|routing| (routing, logical.channels));
					let (output, layer_values) = self.compile_training_layer(
						layer,
						current,
						current_width,
						batch,
						validity,
						valid_count,
						routing,
						config,
					)?;
					current = output;
					current_width = layer.width().get();
					logical = LogicalFeatureShape::from_width(current_width)?;
					preceding_pool = None;
					values.push(BlockValues::Layer(layer_values));
				}
				DenseBlock::Pool(pool) => {
					let (pooled_logical, state) = logical.pooled(*pool)?;
					let (output, pool_values) = self.compile_training_pool(
						current,
						batch,
						state,
						pool.size(),
						block_index,
						config.reduction_tree_lanes,
					)?;
					current = output;
					logical = pooled_logical;
					current_width = logical.width()?;
					preceding_pool = Some(*pool);
					values.push(BlockValues::Pool(pool_values));
				}
				DenseBlock::Residual(residual) => {
					preceding_pool = None;
					let residual_input = current;
					let residual_input_width = current_width;
					let mut branch_current = residual_input;
					let mut branch_width = residual_input_width;
					let mut branch_values = Vec::with_capacity(residual.branch().len());
					for branch_operation in residual.branch() {
						match branch_operation {
							DenseResidualOperation::Layer(layer) => {
								let (output, layer_values) = self.compile_training_layer(
									layer,
									branch_current,
									branch_width,
									batch,
									validity,
									valid_count,
									None,
									config,
								)?;
								branch_current = output;
								branch_width = layer.width().get();
								branch_values.push(ResidualBranchValues::Layer(layer_values));
							}
							DenseResidualOperation::Operation(operation) => {
								let (output, operation_values) = self.compile_training_operation(
									branch_current,
									*operation,
									batch,
									branch_width,
									validity,
									valid_count,
									config,
								)?;
								branch_current = output;
								branch_values.push(ResidualBranchValues::Operation(operation_values));
							}
						}
					}
					let output_width = residual.output_width().ok_or_else(|| {
						TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"residual branch requires at least one layer operation",
						)
					})?;
					if branch_width != output_width.get() {
						return Err(TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"residual branch output disagrees with its final layer width",
						));
					}
					let (skip, projection) = if residual_input_width == output_width.get() {
						(residual_input, None)
					} else {
						let projection = self.initialize_weight(
							shape(&[residual_input_width, output_width.get()])?,
							residual_input_width,
							config.random_seed,
						)?;
						let skip = self.bias_free_linear(
							residual_input,
							projection.value,
							batch,
							output_width.get(),
							self.training_domain,
						)?;
						(skip, Some(projection))
					};
					current = self.exact_add(branch_current, skip, self.training_domain)?;
					current_width = output_width.get();
					logical = LogicalFeatureShape::from_width(current_width)?;
					let mut operation_values = Vec::with_capacity(residual.operations().len());
					for operation in residual.operations().iter().copied() {
						let (output, values) = self.compile_training_operation(
							current,
							operation,
							batch,
							current_width,
							validity,
							valid_count,
							config,
						)?;
						current = output;
						operation_values.push(values);
					}
					values.push(BlockValues::Residual(ResidualValues {
						input: residual_input,
						branch: branch_values,
						projection,
						operations: operation_values,
					}));
				}
			}
		}
		Ok((current, current_width, values))
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_training_layer(
		&mut self,
		layer: &DenseLayer,
		input: ValueId,
		input_width: u64,
		batch: u64,
		validity: ValueId,
		valid_count: ValueId,
		routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>,
		config: &DenseTrainingConfig,
	) -> TrainingCompileResult<(ValueId, LayerValues)> {
		let output_width = layer.width().get();
		let output_shape = shape(&[batch, output_width])?;
		let routing_mask = routing
			.map(|(routing, channels)| self.group_routing_mask(input_width, output_width, channels, routing))
			.transpose()?;
		let mut weight = self.initialize_weight(
			shape(&[input_width, output_width])?,
			input_width,
			config.random_seed,
		)?;
		if let Some(mask) = routing_mask {
			let masked = self.tensor(DType::F32, shape(&[input_width, output_width])?)?;
			self.emit_elementwise(
				vec![weight.value, mask],
				vec![masked],
				masked_zero_f32_program()?,
				IterationDomain::first(),
			)?;
			weight.value = masked;
		}
		let forward_weight = weight.value;
		let bias = self.initialize_zero_parameter(shape(&[output_width])?)?;
		let preactivation = self.tensor(DType::F32, output_shape)?;
		self.materialize(
			"gpu_linear_into",
			&[
				("input", input),
				("weight", forward_weight),
				("bias", bias.value),
			],
			&[("output", preactivation)],
			"input",
			&PreparedParameters::new(),
			self.training_domain,
		)?;
		let mut current = preactivation;
		let mut operations = Vec::with_capacity(layer.operations().len());
		for operation in layer.operations().iter().copied() {
			let (output, values) = self.compile_training_operation(
				current,
				operation,
				batch,
				output_width,
				validity,
				valid_count,
				config,
			)?;
			current = output;
			operations.push(values);
		}
		Ok((
			current,
			LayerValues {
				input,
				weight,
				forward_weight,
				routing_mask,
				bias,
				operations,
			},
		))
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_training_pool(
		&mut self,
		input: ValueId,
		batch: u64,
		state: DensePoolState,
		pool_size: NonZeroU64,
		block_index: usize,
		tree_lanes: u32,
	) -> TrainingCompileResult<(ValueId, PoolValues)> {
		let input_width = state.input_width().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"maximum-pool input width overflowed u64",
			)
		})?;
		let output_width = state.output_width().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"maximum-pool output width overflowed u64",
			)
		})?;
		self.require_tensor(
			input,
			DType::F32,
			&[batch, input_width.get()],
			"maximum-pool input",
		)?;
		let preparation = prepare_channelwise_max_pool_1d(
			batch,
			state.input_length().get(),
			state.channels().get(),
			pool_size.get(),
		)?;
		if preparation.groups() != state.output_length().get() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"maximum-pool preparation disagrees with the resolved logical shape",
			));
		}
		let (flat_input, input_matrix_indices) =
			self.pack_matrix_to_flat(input, batch, input_width.get(), self.training_domain)?;
		let window_indices = self.external_i32_tensor(
			ExternalInputRole::TrainingPoolWindowIndices { block: block_index },
			shape(&preparation.window_indices_shape())?,
			preparation.window_indices(),
		)?;
		let winner_bases = self.external_i32_tensor(
			ExternalInputRole::TrainingPoolWinnerBases { block: block_index },
			shape(&preparation.output_shape())?,
			preparation.winner_bases(),
		)?;
		let gradient_batch_indices = self.external_i32_tensor(
			ExternalInputRole::TrainingPoolGradientBatchIndices { block: block_index },
			shape(&[batch])?,
			preparation.gradient_batch_indices(),
		)?;
		let pooled = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		let winners = self.tensor(DType::I32, shape(&preparation.output_shape())?)?;
		self.materialize(
			"recipe_max_pool_1d",
			&[
				("values", flat_input),
				("window_indices", window_indices),
				("winner_bases", winner_bases),
			],
			&[("pooled", pooled), ("winning_indices", winners)],
			"values",
			&preparation.forward_parameters(u64::from(tree_lanes)),
			self.training_domain,
		)?;
		let (output, output_group_indices) = self.unpack_pool_to_matrix(
			pooled,
			batch,
			state.output_length().get(),
			state.channels().get(),
			output_width.get(),
			self.training_domain,
		)?;
		Ok((
			output,
			PoolValues {
				state,
				preparation,
				winners,
				gradient_batch_indices,
				input_matrix_indices,
				output_group_indices,
			},
		))
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_training_operation(
		&mut self,
		input: ValueId,
		operation: DenseOperation,
		batch: u64,
		width: u64,
		validity: ValueId,
		valid_count: ValueId,
		config: &DenseTrainingConfig,
	) -> TrainingCompileResult<(ValueId, OperationValues)> {
		let output_shape = shape(&[batch, width])?;
		let safe_input = self.mask_f32_with_zero(input, validity, self.training_domain)?;
		let (raw_output, variance) = match operation {
			DenseOperation::Activation(activation) => (
				self.apply_activation(safe_input, activation, output_shape, self.training_domain)?,
				None,
			),
			DenseOperation::Normalization(normalization) => {
				let normalized = self.normalize_training(
					safe_input,
					normalization,
					batch,
					width,
					validity,
					valid_count,
					config.normalization_epsilon,
					config.reduction_tree_lanes,
				)?;
				(normalized.normalized, Some(normalized.variance))
			}
		};
		let output = self.mask_f32_with_zero(raw_output, validity, self.training_domain)?;
		Ok((
			output,
			OperationValues {
				operation,
				input: safe_input,
				output,
				variance,
			},
		))
	}

	fn bias_free_linear(
		&mut self,
		input: ValueId,
		weight: ValueId,
		rows: u64,
		output_width: u64,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		let output = self.tensor(DType::F32, shape(&[rows, output_width])?)?;
		self.emit(
			vec![input, weight],
			vec![output],
			PrimitiveKind::Contraction(Contraction {
				batch_axes: Vec::new(),
				contract_axes: vec![(1, 0)],
			}),
			forbidden_aliases(2, 1),
			domain,
		)?;
		Ok(output)
	}

	fn exact_add(
		&mut self,
		left: ValueId,
		right: ValueId,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		let left_dtype = self.tensor_ref(left)?.dtype;
		let left_shape = self.tensor_ref(left)?.shape.clone();
		let right_tensor = self.tensor_ref(right)?;
		if left_dtype != right_tensor.dtype || left_shape != right_tensor.shape {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"residual addition requires exactly equal dtypes and shapes",
			));
		}
		let output = self.tensor(left_dtype, left_shape)?;
		self.emit_elementwise(vec![left, right], vec![output], sum_program(2)?, domain)?;
		Ok(output)
	}

	fn require_tensor(&self, value: ValueId, dtype: DType, extents: &[u64], role: &str) -> TrainingCompileResult<()> {
		let tensor = self.tensor_ref(value)?;
		if tensor.dtype != dtype || tensor.shape.extents() != extents {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				format!(
					"{role} has {:?} {:?}, expected {dtype:?} {extents:?}",
					tensor.dtype,
					tensor.shape.extents()
				),
			));
		}
		Ok(())
	}

	fn identity_indices(&mut self, tensor_shape: Shape) -> TrainingCompileResult<ValueId> {
		checked_i32(tensor_shape.elements(), "identity index element count")?;
		let indices = self.tensor(DType::I32, tensor_shape)?;
		self.emit(
			Vec::new(),
			vec![indices],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		Ok(indices)
	}

	fn zero_f32_tensor(&mut self, tensor_shape: Shape) -> TrainingCompileResult<ValueId> {
		let seed = self.tensor(DType::I32, tensor_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![seed],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let output = self.tensor(DType::F32, tensor_shape)?;
		self.emit_elementwise(
			vec![seed],
			vec![output],
			zero_outputs_program(1)?,
			IterationDomain::first(),
		)?;
		Ok(output)
	}

	fn pack_matrix_to_flat(
		&mut self,
		input: ValueId,
		rows: u64,
		width: u64,
		domain: IterationDomain,
	) -> TrainingCompileResult<(ValueId, ValueId)> {
		self.require_tensor(input, DType::F32, &[rows, width], "pool matrix pack input")?;
		let elements = rows.checked_mul(width).ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"pool matrix pack element count overflowed u64",
			)
		})?;
		let matrix_indices = self.identity_indices(shape(&[rows, width])?)?;
		let base = self.zero_f32_tensor(shape(&[elements])?)?;
		let flat = self.tensor(DType::F32, shape(&[elements])?)?;
		self.emit(
			vec![base, matrix_indices, input],
			vec![flat],
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
			domain,
		)?;
		Ok((flat, matrix_indices))
	}

	fn unpack_pool_to_matrix(
		&mut self,
		pooled: ValueId,
		batch: u64,
		groups: u64,
		channels: u64,
		output_width: u64,
		domain: IterationDomain,
	) -> TrainingCompileResult<(ValueId, ValueId)> {
		self.require_tensor(
			pooled,
			DType::F32,
			&[batch, groups, channels],
			"pool grouped output",
		)?;
		if groups.checked_mul(channels) != Some(output_width) {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"pool grouped output width disagrees with its logical shape",
			));
		}
		let group_indices = self.identity_indices(shape(&[groups, channels])?)?;
		let base = self.zero_f32_tensor(shape(&[batch, output_width])?)?;
		let output = self.tensor(DType::F32, shape(&[batch, output_width])?)?;
		self.emit(
			vec![base, group_indices, pooled],
			vec![output],
			PrimitiveKind::Scatter(Scatter {
				axis: 1,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			forbidden_aliases(3, 1),
			domain,
		)?;
		Ok((output, group_indices))
	}

	fn group_routing_mask(
		&mut self,
		input_width: u64,
		output_width: u64,
		channels: NonZeroU64,
		routing: DenseGroupToNeuronRouting,
	) -> TrainingCompileResult<ValueId> {
		let (groups, neurons) = routing_extents(routing);
		if groups.get().checked_mul(channels.get()) != Some(input_width) || neurons.get() != output_width {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"group-to-neuron routing disagrees with the dense weight shape",
			));
		}
		let weight_shape = shape(&[input_width, output_width])?;
		let positions = self.identity_indices(weight_shape.clone())?;
		let mask = self.tensor(DType::F32, weight_shape)?;
		self.emit_elementwise(
			vec![positions],
			vec![mask],
			group_routing_mask_program(channels, routing)?,
			IterationDomain::first(),
		)?;
		Ok(mask)
	}

	fn initialize_weight(
		&mut self,
		parameter_shape: Shape,
		fan_in: u64,
		seed: u64,
	) -> TrainingCompileResult<InitialParameter> {
		let stream = self.next_weight_stream;
		self.next_weight_stream = stream.checked_add(1).ok_or_else(identity_exhausted)?;
		let random = self.tensor(DType::F32, parameter_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![random],
			PrimitiveKind::Random(RandomMap {
				distribution: RandomDistribution::NormalF32,
				key: RandomKey {
					seed_low: seed,
					seed_high: seed.rotate_left(29) ^ 0x9e37_79b9_7f4a_7c15,
					stream,
				},
				philox_rounds: 10,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let value = self.tensor(DType::F32, parameter_shape.clone())?;
		let scale = (2.0f32 / fan_in as f32).sqrt();
		self.emit_elementwise(
			vec![random],
			vec![value],
			multiply_constant_program(scale)?,
			IterationDomain::first(),
		)?;
		let (first_moment, second_moment) = self.initialize_zero_pair(parameter_shape)?;
		Ok(InitialParameter {
			value,
			first_moment,
			second_moment,
		})
	}

	fn initialize_zero_parameter(&mut self, parameter_shape: Shape) -> TrainingCompileResult<InitialParameter> {
		let seed = self.tensor(DType::I32, parameter_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![seed],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let value = self.tensor(DType::F32, parameter_shape.clone())?;
		let first_moment = self.tensor(DType::F32, parameter_shape.clone())?;
		let second_moment = self.tensor(DType::F32, parameter_shape)?;
		self.emit_elementwise(
			vec![seed],
			vec![value, first_moment, second_moment],
			zero_outputs_program(3)?,
			IterationDomain::first(),
		)?;
		Ok(InitialParameter {
			value,
			first_moment,
			second_moment,
		})
	}

	fn initialize_zero_pair(&mut self, parameter_shape: Shape) -> TrainingCompileResult<(ValueId, ValueId)> {
		let seed = self.tensor(DType::I32, parameter_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![seed],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let first = self.tensor(DType::F32, parameter_shape.clone())?;
		let second = self.tensor(DType::F32, parameter_shape)?;
		self.emit_elementwise(
			vec![seed],
			vec![first, second],
			zero_outputs_program(2)?,
			IterationDomain::first(),
		)?;
		Ok((first, second))
	}

	fn initialize_early_stopping_state(&mut self) -> TrainingCompileResult<InitialEarlyStoppingState> {
		let scalar_shape = shape(&[1])?;
		let seed = self.tensor(DType::I32, scalar_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![seed],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let best_auprc = self.tensor(DType::F32, scalar_shape.clone())?;
		let stale_epochs = self.tensor(DType::I32, scalar_shape.clone())?;
		let stopped = self.tensor(DType::I32, scalar_shape)?;
		self.emit_elementwise(
			vec![seed],
			vec![best_auprc, stale_epochs, stopped],
			early_stopping_initial_program()?,
			IterationDomain::first(),
		)?;
		Ok(InitialEarlyStoppingState {
			best_auprc,
			stale_epochs,
			stopped,
		})
	}

	fn compile_validation_blocks(
		&mut self,
		blocks: &[DenseBlock],
		states: &[DenseBlockState],
		input: ValueId,
		rows: u64,
		config: &DenseTrainingConfig,
		domain: IterationDomain,
	) -> TrainingCompileResult<(ValueId, u64)> {
		if blocks.len() != states.len() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"validation declaration and block-state counts disagree",
			));
		}
		let mut current = input;
		let mut current_width = self.matrix_width(input, "validation input")?;
		let mut logical = LogicalFeatureShape::from_width(current_width)?;
		let mut preceding_pool = None;
		for (block_index, (block, state)) in blocks.iter().zip(states).enumerate() {
			match (block, state) {
				(DenseBlock::Layer(layer), DenseBlockState::Layer(state)) => {
					let routing = preceding_pool
						.and_then(|pool: DensePool| pool.routing(logical.length))
						.map(|routing| (routing, logical.channels));
					current = self
						.compile_validation_layer(layer, state, current, rows, routing, config, domain)?;
					current_width = layer.width().get();
					logical = LogicalFeatureShape::from_width(current_width)?;
					preceding_pool = None;
				}
				(DenseBlock::Pool(pool), DenseBlockState::Pool(state)) => {
					let (pooled_logical, expected_state) = logical.pooled(*pool)?;
					if *state != expected_state {
						return Err(TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"validation maximum-pool state differs from the training logical shape",
						));
					}
					current = self.compile_validation_pool(
						current,
						rows,
						*state,
						pool.size(),
						block_index,
						config.reduction_tree_lanes,
						domain,
					)?;
					logical = pooled_logical;
					current_width = logical.width()?;
					preceding_pool = Some(*pool);
				}
				(DenseBlock::Residual(residual), DenseBlockState::Residual(state)) => {
					preceding_pool = None;
					let residual_input = current;
					let residual_input_width = current_width;
					let mut branch = residual_input;
					let mut branch_width = residual_input_width;
					let mut layer_states = state.branch.iter();
					for operation in residual.branch() {
						match operation {
							DenseResidualOperation::Layer(layer) => {
								let layer_state = layer_states.next().ok_or_else(|| {
									TrainingCompileError::new(
										TrainingCompileErrorKind::InvalidNetwork,
										"residual validation state omitted a branch layer",
									)
								})?;
								branch = self.compile_validation_layer(
									layer,
									layer_state,
									branch,
									rows,
									None,
									config,
									domain,
								)?;
								branch_width = layer.width().get();
							}
							DenseResidualOperation::Operation(operation) => {
								branch = self.apply_validation_operation(
									branch,
									*operation,
									rows,
									branch_width,
									config,
									domain,
								)?;
							}
						}
					}
					if layer_states.next().is_some() {
						return Err(TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"residual validation state has an extra branch layer",
						));
					}
					let output_width = residual.output_width().ok_or_else(|| {
						TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"residual validation requires a branch layer",
						)
					})?;
					if branch_width != output_width.get() {
						return Err(TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"residual validation branch width disagrees with its final layer",
						));
					}
					let skip = match (residual_input_width == output_width.get(), state.projection) {
						(true, None) => residual_input,
						(false, Some(projection)) => self.bias_free_linear(
							residual_input,
							projection.updated_parameter,
							rows,
							output_width.get(),
							domain,
						)?,
						(true, Some(_)) => {
							return Err(TrainingCompileError::new(
								TrainingCompileErrorKind::InvalidNetwork,
								"equal-width residual unexpectedly retained a projection",
							));
						}
						(false, None) => {
							return Err(TrainingCompileError::new(
								TrainingCompileErrorKind::InvalidNetwork,
								"mismatched residual omitted its projection",
							));
						}
					};
					current = self.exact_add(branch, skip, domain)?;
					current_width = output_width.get();
					logical = LogicalFeatureShape::from_width(current_width)?;
					for operation in residual.operations().iter().copied() {
						current = self.apply_validation_operation(
							current,
							operation,
							rows,
							current_width,
							config,
							domain,
						)?;
					}
				}
				_ => {
					return Err(TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidNetwork,
						"validation block declaration and state variants disagree",
					));
				}
			}
		}
		Ok((current, current_width))
	}

	#[allow(clippy::too_many_arguments)]
	fn compile_validation_pool(
		&mut self,
		input: ValueId,
		rows: u64,
		state: DensePoolState,
		pool_size: NonZeroU64,
		block_index: usize,
		tree_lanes: u32,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		let input_width = state.input_width().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"validation maximum-pool input width overflowed u64",
			)
		})?;
		let output_width = state.output_width().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"validation maximum-pool output width overflowed u64",
			)
		})?;
		let preparation = prepare_channelwise_max_pool_1d(
			rows,
			state.input_length().get(),
			state.channels().get(),
			pool_size.get(),
		)?;
		if preparation.groups() != state.output_length().get() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"validation maximum-pool preparation disagrees with training",
			));
		}
		let (flat_input, _) = self.pack_matrix_to_flat(input, rows, input_width.get(), domain)?;
		let window_indices = self.external_i32_tensor(
			ExternalInputRole::ValidationPoolWindowIndices { block: block_index },
			shape(&preparation.window_indices_shape())?,
			preparation.window_indices(),
		)?;
		let winner_bases = self.external_i32_tensor(
			ExternalInputRole::ValidationPoolWinnerBases { block: block_index },
			shape(&preparation.output_shape())?,
			preparation.winner_bases(),
		)?;
		let pooled = self.tensor(DType::F32, shape(&preparation.output_shape())?)?;
		let unused_winners = self.tensor(DType::I32, shape(&preparation.output_shape())?)?;
		self.materialize(
			"recipe_max_pool_1d",
			&[
				("values", flat_input),
				("window_indices", window_indices),
				("winner_bases", winner_bases),
			],
			&[("pooled", pooled), ("winning_indices", unused_winners)],
			"values",
			&preparation.forward_parameters(u64::from(tree_lanes)),
			domain,
		)?;
		let (output, _) = self.unpack_pool_to_matrix(
			pooled,
			rows,
			state.output_length().get(),
			state.channels().get(),
			output_width.get(),
			domain,
		)?;
		Ok(output)
	}

	fn compile_validation_layer(
		&mut self,
		layer: &DenseLayer,
		state: &DenseLayerState,
		input: ValueId,
		rows: u64,
		routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>,
		config: &DenseTrainingConfig,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		let output_width = layer.width().get();
		let input_width = self.matrix_width(input, "validation dense input")?;
		let routing_mask = routing
			.map(|(routing, channels)| self.group_routing_mask(input_width, output_width, channels, routing))
			.transpose()?;
		let forward_weight = if let Some(mask) = routing_mask {
			let masked = self.tensor(DType::F32, shape(&[input_width, output_width])?)?;
			self.emit_elementwise(
				vec![state.weight.updated_parameter, mask],
				vec![masked],
				masked_zero_f32_program()?,
				domain,
			)?;
			masked
		} else {
			state.weight.updated_parameter
		};
		let output_shape = shape(&[rows, output_width])?;
		let preactivation = self.tensor(DType::F32, output_shape)?;
		self.materialize(
			"gpu_linear_into",
			&[
				("input", input),
				("weight", forward_weight),
				("bias", state.bias.updated_parameter),
			],
			&[("output", preactivation)],
			"input",
			&PreparedParameters::new(),
			domain,
		)?;
		let mut current = preactivation;
		for operation in layer.operations().iter().copied() {
			current = self.apply_validation_operation(current, operation, rows, output_width, config, domain)?;
		}
		Ok(current)
	}

	fn apply_validation_operation(
		&mut self,
		input: ValueId,
		operation: DenseOperation,
		rows: u64,
		width: u64,
		config: &DenseTrainingConfig,
		domain: IterationDomain,
	) -> TrainingCompileResult<ValueId> {
		match operation {
			DenseOperation::Activation(activation) => {
				self.apply_activation(input, activation, shape(&[rows, width])?, domain)
			}
			DenseOperation::Normalization(normalization) => Ok(self
				.normalize_validation(
					input,
					normalization,
					rows,
					width,
					config.normalization_epsilon,
					config.reduction_tree_lanes,
					domain,
				)?
				.normalized),
		}
	}

	fn compile_validation(
		&mut self,
		validation: ValidationValues,
		training_config: &DenseTrainingConfig,
		block_declarations: &[DenseBlock],
		validation_config: &BinaryValidationConfig,
		block_states: &[DenseBlockState],
		early_stopping_initial: Option<InitialEarlyStoppingState>,
		bounds: &TrainingBounds,
	) -> TrainingCompileResult<BinaryValidationOutputs> {
		let validation_domain = IterationDomain::new(
			bounds.batches_per_epoch - 1,
			bounds.training_iterations.get(),
			bounds.batches_per_epoch,
		)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				"validation domain must contain each epoch's final training iteration",
			)
		})?;
		let (current, _) = self.compile_validation_blocks(
			block_declarations,
			block_states,
			validation.features,
			validation.rows,
			training_config,
			validation_domain,
		)?;
		let logits = current;
		let matrix_shape = shape(&[validation.rows, 1])?;
		let probabilities_matrix = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_owned_scalar(
			"gpu_sigmoid_into",
			vec![logits],
			vec![probabilities_matrix],
			validation_domain,
		)?;
		let losses_matrix = self.tensor(DType::F32, matrix_shape.clone())?;
		let unused_gradient = self.tensor(DType::F32, matrix_shape)?;
		self.materialize(
			"gpu_bce_with_logits",
			&[("logits", logits), ("targets", validation.targets)],
			&[("losses", losses_matrix), ("gradients", unused_gradient)],
			"logits",
			&PreparedParameters::new(),
			validation_domain,
		)?;

		let vector_shape = shape(&[validation.rows])?;
		let probabilities = self.tensor(DType::F32, vector_shape.clone())?;
		self.reduce_sum(
			probabilities_matrix,
			probabilities,
			&[1],
			training_config.reduction_tree_lanes,
			validation_domain,
		)?;
		let targets = self.tensor(DType::F32, vector_shape.clone())?;
		self.reduce_sum(
			validation.targets,
			targets,
			&[1],
			training_config.reduction_tree_lanes,
			validation_domain,
		)?;
		let losses = self.tensor(DType::F32, vector_shape)?;
		self.reduce_sum(
			losses_matrix,
			losses,
			&[1],
			training_config.reduction_tree_lanes,
			validation_domain,
		)?;
		let metrics = self.materialize_binary_metrics(
			probabilities,
			targets,
			losses,
			validation.rows,
			training_config.reduction_tree_lanes,
			validation_config,
			validation_domain,
		)?;
		let early_stopping = match (
			validation_config.early_stopping_patience(),
			early_stopping_initial,
		) {
			(Some(patience), Some(initial)) => {
				Some(self.update_early_stopping(metrics.auprc, initial, patience, validation_domain)?)
			}
			(None, None) => None,
			_ => {
				return Err(TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidOptimizer,
					"early-stopping state and validation declaration disagree",
				));
			}
		};
		let temperature_scaling = validation_config
			.temperature_scaling()
			.map(|temperature| {
				self.compile_temperature_scaling(
					logits,
					validation.targets,
					validation.rows,
					training_config.reduction_tree_lanes,
					temperature,
					bounds,
				)
			})
			.transpose()?;
		Ok(BinaryValidationOutputs {
			logits,
			metrics,
			metric_domain: validation_domain,
			early_stopping,
			temperature_scaling,
		})
	}

	fn compile_multiclass_validation(
		&mut self,
		validation: ValidationValues,
		training_config: &DenseTrainingConfig,
		block_declarations: &[DenseBlock],
		block_states: &[DenseBlockState],
		bounds: &TrainingBounds,
	) -> TrainingCompileResult<MulticlassValidationOutputs> {
		let validation_domain = IterationDomain::new(
			bounds.batches_per_epoch - 1,
			bounds.training_iterations.get(),
			bounds.batches_per_epoch,
		)
		.ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidOptimizer,
				"validation domain must contain each epoch's final training iteration",
			)
		})?;
		let (current, classes) = self.compile_validation_blocks(
			block_declarations,
			block_states,
			validation.features,
			validation.rows,
			training_config,
			validation_domain,
		)?;
		let logits = current;
		let matrix_shape = shape(&[validation.rows, classes])?;
		let losses = self.tensor(DType::F32, matrix_shape.clone())?;
		let unused_gradient = self.tensor(DType::F32, matrix_shape)?;
		self.cross_entropy_with_logits(
			logits,
			validation.targets,
			losses,
			unused_gradient,
			validation.rows,
			classes,
			training_config.reduction_tree_lanes,
			validation_domain,
		)?;
		let scalar_shape = shape(&[1])?;
		let loss_sum = self.tensor(DType::F32, scalar_shape.clone())?;
		self.reduce_sum(
			losses,
			loss_sum,
			&[0, 1],
			training_config.reduction_tree_lanes,
			validation_domain,
		)?;
		let mean_cross_entropy = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_elementwise(
			vec![loss_sum],
			vec![mean_cross_entropy],
			divide_constant_program(validation.rows as f32)?,
			validation_domain,
		)?;

		let target_vector = self.tensor(DType::I32, shape(&[validation.rows])?)?;
		self.reduce_sum(
			validation.targets,
			target_vector,
			&[1],
			training_config.reduction_tree_lanes,
			validation_domain,
		)?;
		let correct_count = self.tensor(DType::F32, scalar_shape.clone())?;
		let parameters = [(
			"tree_lanes".to_owned(),
			PreparedParameter::U64(u64::from(training_config.reduction_tree_lanes)),
		)]
		.into_iter()
		.collect::<PreparedParameters>();
		self.materialize(
			"gpu_accuracy",
			&[("predictions", logits), ("targets", target_vector)],
			&[("correct_count", correct_count)],
			"predictions",
			&parameters,
			validation_domain,
		)?;
		let accuracy = self.tensor(DType::F32, scalar_shape)?;
		self.emit_elementwise(
			vec![correct_count],
			vec![accuracy],
			divide_constant_program(validation.rows as f32)?,
			validation_domain,
		)?;
		Ok(MulticlassValidationOutputs {
			logits,
			metrics: MulticlassMetricOutputs {
				mean_cross_entropy,
				accuracy,
			},
			metric_domain: validation_domain,
		})
	}

	fn materialize_binary_metrics(
		&mut self,
		probabilities: ValueId,
		targets: ValueId,
		losses: ValueId,
		examples: u64,
		tree_lanes: u32,
		config: &BinaryValidationConfig,
		domain: IterationDomain,
	) -> TrainingCompileResult<BinaryMetricOutputs> {
		let scalar_shape = shape(&[1])?;
		let mean_bce = self.tensor(DType::F32, scalar_shape.clone())?;
		let auroc = self.tensor(DType::F32, scalar_shape.clone())?;
		let auprc = self.tensor(DType::F32, scalar_shape.clone())?;
		let brier_score = self.tensor(DType::F32, scalar_shape.clone())?;
		let expected_calibration_error = self.tensor(DType::F32, scalar_shape.clone())?;
		let recall_at = config
			.recall_thresholds()
			.map(|threshold| -> TrainingCompileResult<_> {
				Ok(RecallMetricOutput {
					threshold_bits: threshold.to_bits(),
					value: self.tensor(DType::F32, scalar_shape.clone())?,
				})
			})
			.collect::<TrainingCompileResult<Vec<_>>>()?;
		let requirements = binary_metric_requirements(examples, recall_at.len(), config.calibration_bins().get())?;
		let first_value = self.next_value;
		let first_kernel = self.next_kernel;
		self.next_value = self
			.next_value
			.checked_add(requirements.intermediate_values)
			.ok_or_else(identity_exhausted)?;
		self.next_kernel = self
			.next_kernel
			.checked_add(requirements.kernels)
			.ok_or_else(identity_exhausted)?;
		let request = BinaryClassificationMetricRequest {
			probabilities: self.tensor_ref(probabilities)?.clone(),
			targets: self.tensor_ref(targets)?.clone(),
			per_element_bce: self.tensor_ref(losses)?.clone(),
			mean_bce: self.tensor_ref(mean_bce)?.clone(),
			auroc: self.tensor_ref(auroc)?.clone(),
			auprc: self.tensor_ref(auprc)?.clone(),
			brier_score: self.tensor_ref(brier_score)?.clone(),
			expected_calibration_error: self.tensor_ref(expected_calibration_error)?.clone(),
			recall_at: recall_at
				.iter()
				.map(|recall| {
					Ok(RecallAtOutput::new(
						recall.threshold(),
						self.tensor_ref(recall.value)?.clone(),
					))
				})
				.collect::<TrainingCompileResult<Vec<_>>>()?,
			calibration_bins: config.calibration_bins().get(),
			tree_lanes,
			identity_namespace: IdentityNamespace::new(
				ValueId::new(first_value),
				requirements.intermediate_values,
				KernelTemplateId::new(first_kernel),
				requirements.kernels,
			),
			workspace_limit: requirements.workspace_bytes,
		};
		let materialized = materialize_binary_classification_metrics(&request)?;
		for tensor in &materialized.graph.tensors {
			self.insert_tensor_contract(tensor.clone())?;
		}
		for node in &materialized.graph.nodes {
			self.nodes.push(node.clone());
			self.domains.push(KernelIterationDomain {
				kernel: node.kernel.id,
				domain,
			});
		}
		Ok(BinaryMetricOutputs {
			mean_bce,
			auroc,
			auprc,
			brier_score,
			expected_calibration_error,
			recall_at,
		})
	}

	fn update_early_stopping(
		&mut self,
		auprc: ValueId,
		initial: InitialEarlyStoppingState,
		patience: NonZeroU64,
		domain: IterationDomain,
	) -> TrainingCompileResult<EarlyStoppingState> {
		let patience = checked_i32(patience.get(), "early-stopping patience")?;
		let scalar_shape = shape(&[1])?;
		let updated_best_auprc = self.tensor(DType::F32, scalar_shape.clone())?;
		let updated_stale_epochs = self.tensor(DType::I32, scalar_shape.clone())?;
		let updated_stopped = self.tensor(DType::I32, scalar_shape)?;
		let update_kernel = self.emit(
			vec![
				auprc,
				initial.best_auprc,
				initial.stale_epochs,
				initial.stopped,
			],
			vec![updated_best_auprc, updated_stale_epochs, updated_stopped],
			PrimitiveKind::Elementwise(Elementwise {
				program: early_stopping_update_program(patience)?,
			}),
			exact_early_stopping_aliases(),
			domain,
		)?;
		Ok(EarlyStoppingState {
			initial_best_auprc: initial.best_auprc,
			updated_best_auprc,
			initial_stale_epochs: initial.stale_epochs,
			updated_stale_epochs,
			initial_stopped: initial.stopped,
			updated_stopped,
			update_kernel,
		})
	}

	fn compile_temperature_scaling(
		&mut self,
		logits: ValueId,
		targets: ValueId,
		examples: u64,
		tree_lanes: u32,
		config: TemperatureScalingConfig,
		bounds: &TrainingBounds,
	) -> TrainingCompileResult<TemperatureScalingState> {
		let domain = IterationDomain::new(bounds.training_iterations.get(), bounds.iterations.get(), 1)
			.ok_or_else(|| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::InvalidOptimizer,
					"temperature-scaling domain must be nonempty",
				)
			})?;
		let scalar_shape = shape(&[1])?;
		let matrix_shape = shape(&[examples, 1])?;
		let seed = self.tensor(DType::I32, scalar_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![seed],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let initial_temperature = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_elementwise(
			vec![seed],
			vec![initial_temperature],
			constant_f32_from_i32_program(1.0)?,
			IterationDomain::first(),
		)?;
		let scaled_logits = self.tensor(DType::F32, matrix_shape.clone())?;
		self.emit_elementwise(
			vec![logits, initial_temperature],
			vec![scaled_logits],
			divide_program()?,
			domain,
		)?;
		let unused_losses = self.tensor(DType::F32, matrix_shape.clone())?;
		let scaled_gradient = self.tensor(DType::F32, matrix_shape.clone())?;
		self.materialize(
			"gpu_bce_with_logits",
			&[("logits", scaled_logits), ("targets", targets)],
			&[("losses", unused_losses), ("gradients", scaled_gradient)],
			"logits",
			&PreparedParameters::new(),
			domain,
		)?;
		let contributions = self.tensor(DType::F32, matrix_shape)?;
		self.emit_elementwise(
			vec![scaled_gradient, logits, initial_temperature],
			vec![contributions],
			temperature_gradient_program(examples as f32)?,
			domain,
		)?;
		let temperature_gradient = self.tensor(DType::F32, scalar_shape.clone())?;
		self.reduce_sum(
			contributions,
			temperature_gradient,
			&[0, 1],
			tree_lanes,
			domain,
		)?;
		let updated_temperature = self.tensor(DType::F32, scalar_shape)?;
		let update_kernel = self.emit(
			vec![initial_temperature, temperature_gradient],
			vec![updated_temperature],
			PrimitiveKind::Elementwise(Elementwise {
				program: temperature_update_program(config)?,
			}),
			vec![
				PrimitiveAliasRule {
					input: 0,
					output: 0,
					permission: AliasPermission::MustAliasExact,
				},
				PrimitiveAliasRule {
					input: 1,
					output: 0,
					permission: AliasPermission::Forbidden,
				},
			],
			domain,
		)?;
		self.external_outputs.insert(updated_temperature);
		Ok(TemperatureScalingState {
			initial_temperature,
			updated_temperature,
			update_kernel,
			iterations: config.iterations,
		})
	}

	#[allow(clippy::too_many_arguments)]
	fn backward_blocks(
		&mut self,
		blocks: &[BlockValues],
		mut gradient: ValueId,
		batch: u64,
		validity: ValueId,
		valid_count: ValueId,
		epsilon: f32,
		tree_lanes: u32,
	) -> TrainingCompileResult<Vec<BlockGradients>> {
		let mut gradients = vec![None; blocks.len()];
		for block_index in (0..blocks.len()).rev() {
			let need_input_gradient = block_index != 0;
			match &blocks[block_index] {
				BlockValues::Layer(layer) => {
					let (parameter_gradient, input_gradient) = self.backward_layer(
						layer,
						gradient,
						need_input_gradient,
						batch,
						validity,
						valid_count,
						epsilon,
						tree_lanes,
					)?;
					if let Some(input_gradient) = input_gradient {
						gradient = input_gradient;
					}
					gradients[block_index] = Some(BlockGradients::Layer(parameter_gradient));
				}
				BlockValues::Pool(pool) => {
					gradient = self.backward_pool(pool, gradient, batch, validity)?;
					gradients[block_index] = Some(BlockGradients::Pool);
				}
				BlockValues::Residual(residual) => {
					for operation in residual.operations.iter().rev() {
						gradient = self.backward_operation(
							gradient,
							*operation,
							validity,
							valid_count,
							epsilon,
							tree_lanes,
						)?;
					}
					let merge_gradient = gradient;
					let layer_count = residual
						.branch
						.iter()
						.filter(|operation| matches!(operation, ResidualBranchValues::Layer(_)))
						.count();
					let mut branch_gradients = vec![None; layer_count];
					let mut branch_gradient = Some(merge_gradient);
					let mut layer_index = layer_count;
					for operation in residual.branch.iter().rev() {
						match operation {
							ResidualBranchValues::Operation(operation) => {
								if let Some(current) = branch_gradient {
									branch_gradient = Some(self.backward_operation(
										current,
										*operation,
										validity,
										valid_count,
										epsilon,
										tree_lanes,
									)?);
								}
							}
							ResidualBranchValues::Layer(layer) => {
								layer_index = layer_index.checked_sub(1).ok_or_else(|| {
									TrainingCompileError::new(
										TrainingCompileErrorKind::InvalidNetwork,
										"residual branch layer tape underflowed",
									)
								})?;
								let current = branch_gradient.ok_or_else(|| {
									TrainingCompileError::new(
										TrainingCompileErrorKind::InvalidNetwork,
										"residual branch gradient ended before all parameters",
									)
								})?;
								let (parameter_gradient, input_gradient) = self.backward_layer(
									layer,
									current,
									true,
									batch,
									validity,
									valid_count,
									epsilon,
									tree_lanes,
								)?;
								branch_gradients[layer_index] = Some(parameter_gradient);
								branch_gradient = input_gradient;
							}
						}
					}
					let branch_gradients = branch_gradients
						.into_iter()
						.map(|gradient| {
							gradient.ok_or_else(|| {
								TrainingCompileError::new(
									TrainingCompileErrorKind::InvalidNetwork,
									"residual backward omitted a branch parameter gradient",
								)
							})
						})
						.collect::<TrainingCompileResult<Vec<_>>>()?;
					let (projection_gradient, skip_gradient) = match residual.projection {
						Some(projection) => {
							let safe_residual_input = self.mask_f32_with_zero(
								residual.input,
								validity,
								self.training_domain,
							)?;
							let safe_merge_gradient = self.mask_f32_with_zero(
								merge_gradient,
								validity,
								self.training_domain,
							)?;
							let weight_shape = self.tensor_ref(projection.value)?.shape.clone();
							let weight_gradient = self.tensor(DType::F32, weight_shape)?;
							self.emit(
								vec![safe_residual_input, safe_merge_gradient],
								vec![weight_gradient],
								PrimitiveKind::Contraction(Contraction {
									batch_axes: Vec::new(),
									contract_axes: vec![(0, 0)],
								}),
								forbidden_aliases(2, 1),
								self.training_domain,
							)?;
							let input_shape = self.tensor_ref(residual.input)?.shape.clone();
							let input_gradient = self.tensor(DType::F32, input_shape)?;
							self.emit(
								vec![safe_merge_gradient, projection.value],
								vec![input_gradient],
								PrimitiveKind::Contraction(Contraction {
									batch_axes: Vec::new(),
									contract_axes: vec![(1, 1)],
								}),
								forbidden_aliases(2, 1),
								self.training_domain,
							)?;
							let skip_gradient = Some(input_gradient);
							(Some(weight_gradient), skip_gradient)
						}
						None => (None, Some(merge_gradient)),
					};
					let branch_gradient = branch_gradient.ok_or_else(|| {
						TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"residual branch did not produce its input gradient",
						)
					})?;
					let skip_gradient = skip_gradient.ok_or_else(|| {
						TrainingCompileError::new(
							TrainingCompileErrorKind::InvalidNetwork,
							"residual skip did not produce its input gradient",
						)
					})?;
					gradient = self.exact_add(branch_gradient, skip_gradient, self.training_domain)?;
					gradients[block_index] = Some(BlockGradients::Residual {
						branch: branch_gradients,
						projection: projection_gradient,
					});
				}
			}
		}
		gradients
			.into_iter()
			.map(|gradient| {
				gradient.ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidNetwork,
						"dense backward failed to produce one gradient structure per block",
					)
				})
			})
			.collect()
	}

	fn backward_pool(
		&mut self,
		pool: &PoolValues,
		gradient: ValueId,
		batch: u64,
		validity: ValueId,
	) -> TrainingCompileResult<ValueId> {
		let input_width = pool.state.input_width().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"maximum-pool backward input width overflowed u64",
			)
		})?;
		let output_width = pool.state.output_width().ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"maximum-pool backward output width overflowed u64",
			)
		})?;
		self.require_tensor(
			gradient,
			DType::F32,
			&[batch, output_width.get()],
			"maximum-pool output gradient",
		)?;
		let gradient = self.mask_f32_with_zero(gradient, validity, self.training_domain)?;
		let grouped_gradient = self.tensor(
			DType::F32,
			shape(&[
				batch,
				pool.state.output_length().get(),
				pool.state.channels().get(),
			])?,
		)?;
		self.emit(
			vec![gradient, pool.output_group_indices],
			vec![grouped_gradient],
			PrimitiveKind::Gather(Gather {
				axis: 1,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
			self.training_domain,
		)?;
		let flat_shape = shape(&[pool.preparation.input_elements()])?;
		let input_gradient_base = self.zero_f32_tensor(flat_shape.clone())?;
		let flat_gradient = self.tensor(DType::F32, flat_shape)?;
		self.materialize(
			"recipe_max_pool_1d_backward",
			&[
				("output_gradient", grouped_gradient),
				("winning_indices", pool.winners),
				("gradient_batch_indices", pool.gradient_batch_indices),
				("input_gradient_base", input_gradient_base),
			],
			&[("input_gradient", flat_gradient)],
			"output_gradient",
			&pool.preparation.backward_parameters(),
			self.training_domain,
		)?;
		let input_gradient = self.tensor(DType::F32, shape(&[batch, input_width.get()])?)?;
		self.emit(
			vec![flat_gradient, pool.input_matrix_indices],
			vec![input_gradient],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
			forbidden_aliases(2, 1),
			self.training_domain,
		)?;
		Ok(input_gradient)
	}

	#[allow(clippy::too_many_arguments)]
	fn backward_layer(
		&mut self,
		layer: &LayerValues,
		mut gradient: ValueId,
		need_input_gradient: bool,
		batch: u64,
		validity: ValueId,
		valid_count: ValueId,
		epsilon: f32,
		tree_lanes: u32,
	) -> TrainingCompileResult<(GradientPair, Option<ValueId>)> {
		for operation in layer.operations.iter().rev() {
			gradient = self.backward_operation(
				gradient,
				*operation,
				validity,
				valid_count,
				epsilon,
				tree_lanes,
			)?;
		}
		let gradient = self.mask_f32_with_zero(gradient, validity, self.training_domain)?;
		let safe_input = self.mask_f32_with_zero(layer.input, validity, self.training_domain)?;
		let weight_shape = self.tensor_ref(layer.weight.value)?.shape.clone();
		let bias_shape = self.tensor_ref(layer.bias.value)?.shape.clone();
		let raw_weight_gradient = self.tensor(DType::F32, weight_shape.clone())?;
		let bias_gradient = self.tensor(DType::F32, bias_shape)?;
		let prepared = [(
			"tree_lanes".to_owned(),
			PreparedParameter::U64(u64::from(tree_lanes)),
		)]
		.into_iter()
		.collect::<PreparedParameters>();
		let input_gradient = if need_input_gradient {
			let input_width = self
				.tensor_ref(layer.input)?
				.shape
				.extents()
				.get(1)
				.copied()
				.ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidNetwork,
						"dense backward input is not rank two",
					)
				})?;
			let input_gradient = self.tensor(DType::F32, shape(&[batch, input_width])?)?;
			self.materialize(
				"gpu_linear_backward_full_into",
				&[
					("output_gradient", gradient),
					("input", safe_input),
					("weight", layer.forward_weight),
				],
				&[
					("input_gradient", input_gradient),
					("weight_gradient", raw_weight_gradient),
					("bias_gradient", bias_gradient),
				],
				"output_gradient",
				&prepared,
				self.training_domain,
			)?;
			Some(input_gradient)
		} else {
			self.materialize(
				"gpu_linear_backward_weights_only_into",
				&[("output_gradient", gradient), ("input", safe_input)],
				&[
					("weight_gradient", raw_weight_gradient),
					("bias_gradient", bias_gradient),
				],
				"output_gradient",
				&prepared,
				self.training_domain,
			)?;
			None
		};
		let weight_gradient = if let Some(mask) = layer.routing_mask {
			let masked = self.tensor(DType::F32, weight_shape)?;
			self.emit_elementwise(
				vec![raw_weight_gradient, mask],
				vec![masked],
				masked_zero_f32_program()?,
				self.training_domain,
			)?;
			masked
		} else {
			raw_weight_gradient
		};
		Ok((
			GradientPair {
				weight: weight_gradient,
				bias: bias_gradient,
			},
			input_gradient,
		))
	}

	fn backward_operation(
		&mut self,
		gradient: ValueId,
		operation: OperationValues,
		validity: ValueId,
		valid_count: ValueId,
		epsilon: f32,
		tree_lanes: u32,
	) -> TrainingCompileResult<ValueId> {
		let gradient = self.mask_f32_with_zero(gradient, validity, self.training_domain)?;
		let result = match operation.operation {
			DenseOperation::Activation(activation) => {
				self.backward_activation(gradient, operation.input, operation.output, activation)
			}
			DenseOperation::Normalization(normalization) => self.backward_normalization(
				gradient,
				operation.output,
				operation.variance.ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidNetwork,
						"normalization forward state omitted its variance",
					)
				})?,
				normalization,
				validity,
				valid_count,
				epsilon,
				tree_lanes,
			),
		}?;
		self.mask_f32_with_zero(result, validity, self.training_domain)
	}

	#[allow(clippy::too_many_arguments)]
	fn update_blocks(
		&mut self,
		blocks: &[BlockValues],
		parameter_gradients: &[ParameterGradient],
		clipped: &[ValueId],
		learning_rate: ValueId,
		beta_one_power: ValueId,
		beta_two_power: ValueId,
		apply_update: Option<ValueId>,
		config: &DenseTrainingConfig,
	) -> TrainingCompileResult<Vec<DenseBlockState>> {
		if parameter_gradients.len() != clipped.len() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"global clipping changed the parameter-gradient count",
			));
		}
		let mut cursor = 0usize;
		let mut states = Vec::with_capacity(blocks.len());
		for block in blocks {
			match block {
				BlockValues::Layer(layer) => {
					states.push(DenseBlockState::Layer(self.update_layer_state(
						layer,
						parameter_gradients,
						clipped,
						&mut cursor,
						learning_rate,
						beta_one_power,
						beta_two_power,
						apply_update,
						config,
					)?));
				}
				BlockValues::Pool(pool) => states.push(DenseBlockState::Pool(pool.state)),
				BlockValues::Residual(residual) => {
					let mut branch = Vec::new();
					for operation in &residual.branch {
						if let ResidualBranchValues::Layer(layer) = operation {
							branch.push(self.update_layer_state(
								layer,
								parameter_gradients,
								clipped,
								&mut cursor,
								learning_rate,
								beta_one_power,
								beta_two_power,
								apply_update,
								config,
							)?);
						}
					}
					let projection = residual
						.projection
						.map(|projection| -> TrainingCompileResult<_> {
							let gradient = take_clipped_gradient(
								parameter_gradients,
								clipped,
								&mut cursor,
								ParameterRole::ResidualProjectionWeight,
							)?;
							self.adamw_update(
								gradient,
								projection,
								learning_rate,
								beta_one_power,
								beta_two_power,
								apply_update,
								config,
							)
						})
						.transpose()?;
					states.push(DenseBlockState::Residual(DenseResidualState {
						branch,
						projection,
					}));
				}
			}
		}
		if cursor != parameter_gradients.len() {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"optimizer parameter traversal retained unused gradients",
			));
		}
		Ok(states)
	}

	#[allow(clippy::too_many_arguments)]
	fn update_layer_state(
		&mut self,
		layer: &LayerValues,
		parameter_gradients: &[ParameterGradient],
		clipped: &[ValueId],
		cursor: &mut usize,
		learning_rate: ValueId,
		beta_one_power: ValueId,
		beta_two_power: ValueId,
		apply_update: Option<ValueId>,
		config: &DenseTrainingConfig,
	) -> TrainingCompileResult<DenseLayerState> {
		let weight_gradient = take_clipped_gradient(
			parameter_gradients,
			clipped,
			cursor,
			ParameterRole::LayerWeight,
		)?;
		let bias_gradient = take_clipped_gradient(
			parameter_gradients,
			clipped,
			cursor,
			ParameterRole::LayerBias,
		)?;
		let weight = self.adamw_update(
			weight_gradient,
			layer.weight,
			learning_rate,
			beta_one_power,
			beta_two_power,
			apply_update,
			config,
		)?;
		let bias = self.adamw_update(
			bias_gradient,
			layer.bias,
			learning_rate,
			beta_one_power,
			beta_two_power,
			apply_update,
			config,
		)?;
		Ok(DenseLayerState { weight, bias })
	}

	fn global_clip(
		&mut self,
		gradients: &[ValueId],
		maximum_norm: f32,
		tree_lanes: u32,
	) -> TrainingCompileResult<Vec<ValueId>> {
		let scalar_shape = shape(&[1])?;
		let mut squared_norms = Vec::with_capacity(gradients.len());
		for gradient in gradients.iter().copied() {
			let gradient_shape = self.tensor_ref(gradient)?.shape.clone();
			let squares = self.tensor(DType::F32, gradient_shape.clone())?;
			self.emit_elementwise(
				vec![gradient],
				vec![squares],
				square_program()?,
				self.training_domain,
			)?;
			let norm = self.tensor(DType::F32, scalar_shape.clone())?;
			let axes = (0..gradient_shape.rank()).collect::<Vec<_>>();
			self.reduce_sum(squares, norm, &axes, tree_lanes, self.training_domain)?;
			squared_norms.push(norm);
		}
		let global_squared_norm = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_elementwise(
			squared_norms,
			vec![global_squared_norm],
			sum_program(gradients.len())?,
			self.training_domain,
		)?;
		let scale = self.tensor(DType::F32, scalar_shape)?;
		self.emit_elementwise(
			vec![global_squared_norm],
			vec![scale],
			clip_scale_program(maximum_norm)?,
			self.training_domain,
		)?;
		gradients
			.iter()
			.copied()
			.map(|gradient| {
				let clipped = self.tensor(DType::F32, self.tensor_ref(gradient)?.shape.clone())?;
				self.emit_elementwise(
					vec![gradient, scale],
					vec![clipped],
					multiply_program()?,
					self.training_domain,
				)?;
				Ok(clipped)
			})
			.collect()
	}

	fn dynamic_adam_scalars(
		&mut self,
		config: &DenseTrainingConfig,
		bounds: &TrainingBounds,
		stopped: Option<ValueId>,
		accepted: Option<(ValueId, AcceptedUpdatePlan)>,
	) -> TrainingCompileResult<(ValueId, ValueId, ValueId, Option<OptimizerProgressState>)> {
		let scalar_shape = shape(&[1])?;
		let (step_i32, update_state, schedule_warmup, schedule_updates) =
			if let Some((known_count, plan)) = accepted {
				let apply_update = self.tensor(DType::I32, scalar_shape.clone())?;
				let mut gate_inputs = vec![known_count];
				if let Some(stopped) = stopped {
					gate_inputs.push(stopped);
				}
				self.emit_elementwise(
					gate_inputs,
					vec![apply_update],
					optimizer_update_gate_program(stopped.is_some())?,
					self.training_domain,
				)?;
				let initial_accepted_updates = self.tensor(DType::I32, scalar_shape.clone())?;
				self.emit(
					Vec::new(),
					vec![initial_accepted_updates],
					PrimitiveKind::IndexMap(IndexMap {
						start: 0,
						element_step: 0,
						iteration_step: 0,
						modulus: None,
					}),
					Vec::new(),
					IterationDomain::first(),
				)?;
				let updated_accepted_updates = self.tensor(DType::I32, scalar_shape.clone())?;
				let update_kernel = self.emit(
					vec![initial_accepted_updates, apply_update],
					vec![updated_accepted_updates],
					PrimitiveKind::Elementwise(Elementwise {
						program: accepted_update_program()?,
					}),
					exact_single_recurrence_aliases(2),
					self.training_domain,
				)?;
				(
					updated_accepted_updates,
					Some((
						apply_update,
						initial_accepted_updates,
						updated_accepted_updates,
						update_kernel,
						plan,
					)),
					plan.warmup,
					plan.maximum.max(1),
				)
			} else {
				let step_i32 = self.tensor(DType::I32, scalar_shape.clone())?;
				self.emit(
					Vec::new(),
					vec![step_i32],
					PrimitiveKind::IndexMap(IndexMap {
						start: 1,
						element_step: 0,
						iteration_step: 1,
						modulus: None,
					}),
					Vec::new(),
					self.training_domain,
				)?;
				(
					step_i32,
					None,
					bounds.warmup_iterations,
					bounds.training_iterations.get(),
				)
			};
		let warmup = self.tensor(DType::F32, scalar_shape.clone())?;
		let progress = self.tensor(DType::F32, scalar_shape.clone())?;
		let remaining_fraction = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_elementwise(
			vec![step_i32],
			vec![warmup, progress, remaining_fraction],
			schedule_inputs_program(schedule_warmup, schedule_updates)?,
			self.training_domain,
		)?;
		let decay = match config.learning_rate_decay {
			LearningRateDecay::Linear => remaining_fraction,
			LearningRateDecay::Cosine => {
				let angle = self.tensor(DType::F32, scalar_shape.clone())?;
				self.emit_elementwise(
					vec![remaining_fraction],
					vec![angle],
					cosine_angle_program()?,
					self.training_domain,
				)?;
				let sine = self.tensor(DType::F32, scalar_shape.clone())?;
				self.emit_owned_scalar("gpu_sin", vec![angle], vec![sine], self.training_domain)?;
				let decay = self.tensor(DType::F32, scalar_shape.clone())?;
				self.emit_elementwise(
					vec![sine, remaining_fraction],
					vec![decay],
					cosine_decay_program()?,
					self.training_domain,
				)?;
				decay
			}
			LearningRateDecay::Exponential => {
				let argument = self.tensor(DType::F32, scalar_shape.clone())?;
				self.emit_elementwise(
					vec![remaining_fraction],
					vec![argument],
					exponential_decay_argument_program()?,
					self.training_domain,
				)?;
				let curve = self.tensor(DType::F32, scalar_shape.clone())?;
				self.emit_owned_scalar(
					"gpu_expm1",
					vec![argument],
					vec![curve],
					self.training_domain,
				)?;
				let decay = self.tensor(DType::F32, scalar_shape.clone())?;
				self.emit_elementwise(
					vec![curve, remaining_fraction],
					vec![decay],
					exponential_decay_program()?,
					self.training_domain,
				)?;
				decay
			}
		};
		let learning_rate = self.tensor(DType::F32, scalar_shape.clone())?;
		if let Some((apply_update, ..)) = update_state {
			self.emit_elementwise(
				vec![decay, warmup, apply_update],
				vec![learning_rate],
				gated_learning_rate_program(config.adamw.learning_rate)?,
				self.training_domain,
			)?;
		} else {
			let mut learning_rate_inputs = vec![decay, warmup];
			if let Some(stopped) = stopped {
				learning_rate_inputs.push(stopped);
			}
			self.emit_elementwise(
				learning_rate_inputs,
				vec![learning_rate],
				learning_rate_program(config.adamw.learning_rate, stopped.is_some())?,
				self.training_domain,
			)?;
		}
		let beta_seed = self.tensor(DType::I32, scalar_shape.clone())?;
		self.emit(
			Vec::new(),
			vec![beta_seed],
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 0,
				iteration_step: 0,
				modulus: None,
			}),
			Vec::new(),
			IterationDomain::first(),
		)?;
		let initial_beta_one_power = self.tensor(DType::F32, scalar_shape.clone())?;
		let initial_beta_two_power = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_elementwise(
			vec![beta_seed],
			vec![initial_beta_one_power, initial_beta_two_power],
			adam_beta_power_initial_program()?,
			IterationDomain::first(),
		)?;
		let beta_one_power = self.tensor(DType::F32, scalar_shape.clone())?;
		let beta_two_power = self.tensor(DType::F32, scalar_shape)?;
		let mut beta_inputs = vec![initial_beta_one_power, initial_beta_two_power];
		if let Some((apply_update, ..)) = update_state {
			beta_inputs.push(apply_update);
		}
		let beta_update_kernel = self.emit(
			beta_inputs,
			vec![beta_one_power, beta_two_power],
			PrimitiveKind::Elementwise(Elementwise {
				program: if update_state.is_some() {
					gated_adam_beta_power_update_program(config.adamw.beta_one, config.adamw.beta_two)?
				} else {
					adam_beta_power_update_program(config.adamw.beta_one, config.adamw.beta_two)?
				},
			}),
			if update_state.is_some() {
				exact_gated_pair_recurrence_aliases()
			} else {
				exact_pair_recurrence_aliases()
			},
			self.training_domain,
		)?;
		let optimizer_progress = update_state.map(
			|(apply_update, initial_accepted_updates, updated_accepted_updates, update_kernel, plan)| {
				OptimizerProgressState {
					apply_update,
					initial_accepted_updates,
					updated_accepted_updates,
					update_kernel,
					initial_beta_one_power,
					updated_beta_one_power: beta_one_power,
					initial_beta_two_power,
					updated_beta_two_power: beta_two_power,
					beta_update_kernel,
					accepted_updates_per_epoch: plan.per_epoch,
					maximum_accepted_updates: plan.maximum,
					warmup_accepted_updates: plan.warmup,
				}
			},
		);
		Ok((
			learning_rate,
			beta_one_power,
			beta_two_power,
			optimizer_progress,
		))
	}

	fn adamw_update(
		&mut self,
		gradient: ValueId,
		initial: InitialParameter,
		learning_rate: ValueId,
		beta_one_power: ValueId,
		beta_two_power: ValueId,
		apply_update: Option<ValueId>,
		config: &DenseTrainingConfig,
	) -> TrainingCompileResult<ParameterState> {
		let parameter_shape = self.tensor_ref(initial.value)?.shape.clone();
		let updated_first_moment = self.tensor(DType::F32, parameter_shape.clone())?;
		let updated_second_moment = self.tensor(DType::F32, parameter_shape.clone())?;
		let updated_parameter = self.tensor(DType::F32, parameter_shape)?;
		let inputs = vec![
			gradient,
			initial.value,
			initial.first_moment,
			initial.second_moment,
			learning_rate,
			beta_one_power,
			beta_two_power,
		];
		let mut inputs = inputs;
		if let Some(apply_update) = apply_update {
			inputs.push(apply_update);
		}
		let outputs = vec![
			updated_first_moment,
			updated_second_moment,
			updated_parameter,
		];
		let aliases = exact_adam_aliases(inputs.len());
		let update_kernel = self.emit(
			inputs,
			outputs,
			PrimitiveKind::Elementwise(Elementwise {
				program: if apply_update.is_some() {
					gated_adamw_program(
						config.adamw.beta_one,
						config.adamw.beta_two,
						config.adamw.epsilon,
						config.adamw.weight_decay,
					)?
				} else {
					adamw_program(
						config.adamw.beta_one,
						config.adamw.beta_two,
						config.adamw.epsilon,
						config.adamw.weight_decay,
					)?
				},
			}),
			aliases,
			self.training_domain,
		)?;
		self.external_outputs.extend([
			updated_parameter,
			updated_first_moment,
			updated_second_moment,
		]);
		Ok(ParameterState {
			initial_parameter: initial.value,
			updated_parameter,
			initial_first_moment: initial.first_moment,
			updated_first_moment,
			initial_second_moment: initial.second_moment,
			updated_second_moment,
			update_kernel,
		})
	}

	fn reduce_sum(
		&mut self,
		input: ValueId,
		output: ValueId,
		axes: &[usize],
		tree_lanes: u32,
		domain: IterationDomain,
	) -> TrainingCompileResult<KernelTemplateId> {
		self.reduce_value(
			input,
			output,
			ReduceOperator::Sum,
			axes,
			false,
			tree_lanes,
			domain,
		)
	}

	#[allow(clippy::too_many_arguments)]
	fn reduce_value(
		&mut self,
		input: ValueId,
		output: ValueId,
		operator: ReduceOperator,
		axes: &[usize],
		keep_dimensions: bool,
		tree_lanes: u32,
		domain: IterationDomain,
	) -> TrainingCompileResult<KernelTemplateId> {
		self.emit(
			vec![input],
			vec![output],
			PrimitiveKind::Reduce(Reduce {
				operator,
				axes: AxisSet::new(axes.to_vec())?,
				keep_dimensions,
				result: ReduceResult::Value,
				tree_lanes,
			}),
			forbidden_aliases(1, 1),
			domain,
		)
	}

	fn emit_owned_scalar(
		&mut self,
		symbol: &str,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		domain: IterationDomain,
	) -> TrainingCompileResult<KernelTemplateId> {
		let descriptor = operation_registry().resolve_unique(symbol)?;
		let program = lower_scalar(descriptor)?;
		self.emit_elementwise(inputs, outputs, program, domain)
	}

	fn emit_elementwise(
		&mut self,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		program: recipe_core::ScalarProgram,
		domain: IterationDomain,
	) -> TrainingCompileResult<KernelTemplateId> {
		let aliases = forbidden_aliases(inputs.len(), outputs.len());
		self.emit(
			inputs,
			outputs,
			PrimitiveKind::Elementwise(Elementwise { program }),
			aliases,
			domain,
		)
	}

	fn emit(
		&mut self,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		kind: PrimitiveKind,
		alias_rules: Vec<PrimitiveAliasRule>,
		domain: IterationDomain,
	) -> TrainingCompileResult<KernelTemplateId> {
		let id = self.next_kernel()?;
		self.nodes.push(CalculationNode {
			kernel: PrimitiveKernel {
				id,
				inputs,
				outputs,
				alias_rules,
				kind,
			},
		});
		self.domains
			.push(KernelIterationDomain { kernel: id, domain });
		Ok(id)
	}

	fn materialize(
		&mut self,
		symbol: &str,
		inputs: &[(&'static str, ValueId)],
		outputs: &[(&'static str, ValueId)],
		iteration_shape_input: &'static str,
		parameters: &PreparedParameters,
		domain: IterationDomain,
	) -> TrainingCompileResult<()> {
		let mut input_tensors = inputs
			.iter()
			.map(|(_, value)| self.tensor_ref(*value).cloned())
			.collect::<TrainingCompileResult<Vec<_>>>()?;
		for tensor in &mut input_tensors {
			tensor.external_input = true;
			tensor.external_output = false;
		}
		let mut output_tensors = outputs
			.iter()
			.map(|(_, value)| self.tensor_ref(*value).cloned())
			.collect::<TrainingCompileResult<Vec<_>>>()?;
		for tensor in &mut output_tensors {
			tensor.external_input = false;
			tensor.external_output = true;
		}
		let named_inputs = inputs
			.iter()
			.zip(&input_tensors)
			.map(|((name, _), tensor)| NamedTensor::new(name, tensor))
			.collect::<Vec<_>>();
		let named_outputs = outputs
			.iter()
			.zip(&output_tensors)
			.map(|((name, _), tensor)| NamedTensor::new(name, tensor))
			.collect::<Vec<_>>();
		let first_value = self.next_value;
		let first_kernel = self.next_kernel;
		self.next_value = self
			.next_value
			.checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?;
		self.next_kernel = self
			.next_kernel
			.checked_add(MATERIALIZATION_RESERVATION)
			.ok_or_else(identity_exhausted)?;
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&named_inputs,
			&named_outputs,
			iteration_shape_input,
			parameters,
			IdentityNamespace::new(
				ValueId::new(first_value),
				MATERIALIZATION_RESERVATION,
				KernelTemplateId::new(first_kernel),
				MATERIALIZATION_RESERVATION,
			),
			WORKSPACE_LIMIT,
		))?;
		for tensor in &materialized.graph().tensors {
			self.insert_tensor_contract(tensor.clone())?;
		}
		for node in &materialized.graph().nodes {
			self.nodes.push(node.clone());
			self.domains.push(KernelIterationDomain {
				kernel: node.kernel.id,
				domain,
			});
		}
		Ok(())
	}

	fn insert_tensor_contract(&mut self, mut tensor: Tensor) -> TrainingCompileResult<()> {
		tensor.external_input = false;
		tensor.external_output = false;
		match self.tensors.get(&tensor.id) {
			Some(existing)
				if existing.dtype == tensor.dtype
					&& existing.shape == tensor.shape
					&& existing.layout == tensor.layout
					&& existing.storage_bytes == tensor.storage_bytes =>
			{
				Ok(())
			}
			Some(_) => Err(TrainingCompileError::new(
				TrainingCompileErrorKind::Language,
				format!(
					"materialized tensor {} conflicts with an existing contract",
					tensor.id
				),
			)),
			None => {
				self.tensors.insert(tensor.id, tensor);
				Ok(())
			}
		}
	}

	fn tensor_ref(&self, value: ValueId) -> TrainingCompileResult<&Tensor> {
		self.tensors.get(&value).ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::Language,
				format!("compiler tensor {value} is absent"),
			)
		})
	}

	fn matrix_width(&self, value: ValueId, role: &str) -> TrainingCompileResult<u64> {
		let extents = self.tensor_ref(value)?.shape.extents();
		if extents.len() != 2 {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				format!("{role} must be a rank-two matrix"),
			));
		}
		Ok(extents[1])
	}

	fn next_value(&mut self) -> TrainingCompileResult<ValueId> {
		let value = self.next_value;
		self.next_value = value.checked_add(1).ok_or_else(identity_exhausted)?;
		Ok(ValueId::new(value))
	}

	fn next_kernel(&mut self) -> TrainingCompileResult<KernelTemplateId> {
		let kernel = self.next_kernel;
		self.next_kernel = kernel.checked_add(1).ok_or_else(identity_exhausted)?;
		Ok(KernelTemplateId::new(kernel))
	}

	fn finish(
		mut self,
		bounds: TrainingBounds,
		outputs: TrainingOutputs,
		dataset_schema: CompiledDatasetSchema,
		config: DenseTrainingConfig,
		blocks: Vec<DenseBlock>,
		layers: Vec<crate::DenseLayer>,
		output_adapter: Option<DenseOutputAdapter>,
	) -> TrainingCompileResult<CompiledTraining> {
		for tensor in self.tensors.values_mut() {
			tensor.external_input = self.external_input_ids.contains(&tensor.id);
			tensor.external_output = self.external_outputs.contains(&tensor.id);
		}
		let metrics = outputs
			.metric_bindings
			.iter()
			.map(|binding| MetricEmission {
				metric: binding.metric,
				value: binding.value,
				domain: binding.domain,
			})
			.collect();
		let graph = CalculationGraph {
			tensors: self.tensors.into_values().collect(),
			nodes: self.nodes,
		};
		graph.validate()?;
		let canonical = graph.to_ogdl()?;
		let graph = CalculationGraph::from_ogdl(&canonical)?;
		let program = StaticCalculationProgram::new_with_metrics(graph, self.iterations, self.domains, metrics)?;
		let program_text = program.to_ogdl()?;
		let program = StaticCalculationProgram::from_ogdl(&program_text)?;
		Ok(CompiledTraining {
			program,
			external_inputs: self.external_inputs,
			bounds,
			outputs,
			dataset_schema,
			config,
			blocks,
			layers,
			output_adapter,
		})
	}
}

fn matrix_bytes(matrix: &DenseMatrix) -> (DType, Vec<u8>) {
	match matrix {
		DenseMatrix::I32 { values, .. } => (
			DType::I32,
			values.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		),
		DenseMatrix::F32Bits { values, .. } => (
			DType::F32,
			values.iter()
				.flat_map(|value| value.to_le_bytes())
				.collect(),
		),
	}
}

fn shape(extents: &[u64]) -> TrainingCompileResult<Shape> {
	Ok(Shape::new(extents.to_vec())?)
}

fn checked_i32(value: u64, name: &str) -> TrainingCompileResult<i32> {
	i32::try_from(value).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("{name} {value} cannot be represented by int32: {error}"),
		)
	})
}

fn training_metric_bindings(
	batch_loss: ValueId,
	batch_loss_domain: IterationDomain,
	validation: Option<&BinaryValidationOutputs>,
	multiclass_validation: Option<&MulticlassValidationOutputs>,
) -> TrainingCompileResult<Vec<TrainingMetricBinding>> {
	let mut bindings = vec![TrainingMetricBinding {
		kind: TrainingMetricKind::BatchLoss,
		metric: MetricId::new(1),
		value: batch_loss,
		domain: batch_loss_domain,
	}];
	if let Some(validation) = validation {
		for (kind, value) in [
			(
				TrainingMetricKind::ValidationMeanBce,
				validation.metrics.mean_bce,
			),
			(TrainingMetricKind::AuRoc, validation.metrics.auroc),
			(TrainingMetricKind::AuPrc, validation.metrics.auprc),
			(
				TrainingMetricKind::BrierScore,
				validation.metrics.brier_score,
			),
			(
				TrainingMetricKind::ExpectedCalibrationError,
				validation.metrics.expected_calibration_error,
			),
		] {
			push_metric_binding(&mut bindings, kind, value, validation.metric_domain)?;
		}
		for recall in &validation.metrics.recall_at {
			push_metric_binding(
				&mut bindings,
				TrainingMetricKind::RecallAt {
					threshold_bits: recall.threshold_bits,
				},
				recall.value,
				validation.metric_domain,
			)?;
		}
	}
	if let Some(validation) = multiclass_validation {
		for (kind, value) in [
			(
				TrainingMetricKind::ValidationMeanCrossEntropy,
				validation.metrics.mean_cross_entropy,
			),
			(TrainingMetricKind::Accuracy, validation.metrics.accuracy),
		] {
			push_metric_binding(&mut bindings, kind, value, validation.metric_domain)?;
		}
	}
	Ok(bindings)
}

fn push_metric_binding(
	bindings: &mut Vec<TrainingMetricBinding>,
	kind: TrainingMetricKind,
	value: ValueId,
	domain: IterationDomain,
) -> TrainingCompileResult<()> {
	let ordinal = u64::try_from(bindings.len())
		.ok()
		.and_then(|index| index.checked_add(1))
		.ok_or_else(identity_exhausted)?;
	bindings.push(TrainingMetricBinding {
		kind,
		metric: MetricId::new(ordinal),
		value,
		domain,
	});
	Ok(())
}

fn identity_exhausted() -> TrainingCompileError {
	TrainingCompileError::new(
		TrainingCompileErrorKind::IdentityExhausted,
		"deterministic graph identity space exhausted",
	)
}

fn forbidden_aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> {
	(0..inputs)
		.flat_map(|input| {
			(0..outputs).map(move |output| PrimitiveAliasRule {
				input,
				output,
				permission: AliasPermission::Forbidden,
			})
		})
		.collect()
}

fn exact_adam_aliases(inputs: usize) -> Vec<PrimitiveAliasRule> {
	(0..inputs)
		.flat_map(|input| {
			(0..3).map(move |output| {
				let exact = matches!((input, output), (1, 2) | (2, 0) | (3, 1));
				PrimitiveAliasRule {
					input,
					output,
					permission: if exact {
						AliasPermission::MustAliasExact
					} else {
						AliasPermission::Forbidden
					},
				}
			})
		})
		.collect()
}

fn exact_single_recurrence_aliases(inputs: usize) -> Vec<PrimitiveAliasRule> {
	(0..inputs)
		.map(|input| PrimitiveAliasRule {
			input,
			output: 0,
			permission: if input == 0 {
				AliasPermission::MustAliasExact
			} else {
				AliasPermission::Forbidden
			},
		})
		.collect()
}

fn exact_early_stopping_aliases() -> Vec<PrimitiveAliasRule> {
	(0..4).flat_map(|input| {
		(0..3).map(move |output| PrimitiveAliasRule {
			input,
			output,
			permission: if matches!((input, output), (1, 0) | (2, 1) | (3, 2)) {
				AliasPermission::MustAliasExact
			} else {
				AliasPermission::Forbidden
			},
		})
	})
	.collect()
}

fn exact_pair_recurrence_aliases() -> Vec<PrimitiveAliasRule> {
	(0..2).flat_map(|input| {
		(0..2).map(move |output| PrimitiveAliasRule {
			input,
			output,
			permission: if input == output {
				AliasPermission::MustAliasExact
			} else {
				AliasPermission::Forbidden
			},
		})
	})
	.collect()
}

fn exact_gated_pair_recurrence_aliases() -> Vec<PrimitiveAliasRule> {
	(0..3).flat_map(|input| {
		(0..2).map(move |output| PrimitiveAliasRule {
			input,
			output,
			permission: if input == output {
				AliasPermission::MustAliasExact
			} else {
				AliasPermission::Forbidden
			},
		})
	})
	.collect()
}

fn convert_i32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::I32)?;
	let converted = builder.unary(ScalarOpcode::ConvertI32ToF32, value)?;
	Ok(builder.finish(&[converted])?)
}

fn constant_f32_from_i32_program(value: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let _seed = builder.input(DType::I32)?;
	let value = builder.f32(value)?;
	Ok(builder.finish(&[value])?)
}

fn early_stopping_initial_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let _seed = builder.input(DType::I32)?;
	let best = builder.f32(0.0)?;
	let stale = builder.i32(0)?;
	let stopped = builder.i32(0)?;
	Ok(builder.finish(&[best, stale, stopped])?)
}

fn early_stopping_update_program(patience: i32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let auprc = builder.input(DType::F32)?;
	let best = builder.input(DType::F32)?;
	let stale = builder.input(DType::I32)?;
	let stopped = builder.input(DType::I32)?;
	let improved = builder.binary(ScalarOpcode::GreaterThan, auprc, best)?;
	let updated_best = builder.ternary(ScalarOpcode::Select, improved, auprc, best)?;
	let zero = builder.i32(0)?;
	let one = builder.i32(1)?;
	let incremented = builder.binary(ScalarOpcode::Add, stale, one)?;
	let updated_stale = builder.ternary(ScalarOpcode::Select, improved, zero, incremented)?;
	let patience = builder.i32(patience)?;
	let reached = builder.binary(ScalarOpcode::GreaterThanOrEqual, updated_stale, patience)?;
	let updated_stopped = builder.binary(ScalarOpcode::BitOr, stopped, reached)?;
	Ok(builder.finish(&[updated_best, updated_stale, updated_stopped])?)
}

fn validity_program(rows: i32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let row_count = builder.i32(rows)?;
	let valid = builder.binary(ScalarOpcode::LessThan, position, row_count)?;
	let valid = builder.unary(ScalarOpcode::ConvertI32ToF32, valid)?;
	Ok(builder.finish(&[valid])?)
}

fn zero_outputs_program(outputs: usize) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let _seed = builder.input(DType::I32)?;
	let zeros = (0..outputs)
		.map(|_| builder.f32(0.0))
		.collect::<Result<Vec<_>, _>>()?;
	Ok(builder.finish(&zeros)?)
}

fn routing_extents(routing: DenseGroupToNeuronRouting) -> (NonZeroU64, NonZeroU64) {
	match routing {
		DenseGroupToNeuronRouting::Identity { width } => (width, width),
		DenseGroupToNeuronRouting::Expand {
			groups, neurons, ..
		}
		| DenseGroupToNeuronRouting::Contract {
			groups, neurons, ..
		}
		| DenseGroupToNeuronRouting::FullyConnected { groups, neurons } => (groups, neurons),
	}
}

fn group_routing_mask_program(
	channels: NonZeroU64,
	routing: DenseGroupToNeuronRouting,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let (_, neurons) = routing_extents(routing);
	let group_stride = channels.get().checked_mul(neurons.get()).ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::ArithmeticOverflow,
			"group routing row stride overflowed u64",
		)
	})?;
	let group_stride = checked_i32(group_stride, "group routing row stride")?;
	let neurons_i32 = checked_i32(neurons.get(), "group routing neuron count")?;
	let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let group_stride = builder.i32(group_stride)?;
	let neurons_value = builder.i32(neurons_i32)?;
	let group = builder.binary(ScalarOpcode::Divide, position, group_stride)?;
	let neuron = builder.binary(ScalarOpcode::Remainder, position, neurons_value)?;
	let allowed = match routing {
		DenseGroupToNeuronRouting::Identity { .. } => builder.binary(ScalarOpcode::Equal, group, neuron)?,
		DenseGroupToNeuronRouting::Expand {
			neurons_per_group, ..
		} => {
			let divisor = builder.i32(checked_i32(
				neurons_per_group.get(),
				"neurons per routed group",
			)?)?;
			let neuron_group = builder.binary(ScalarOpcode::Divide, neuron, divisor)?;
			builder.binary(ScalarOpcode::Equal, group, neuron_group)?
		}
		DenseGroupToNeuronRouting::Contract {
			groups_per_neuron, ..
		} => {
			let divisor = builder.i32(checked_i32(
				groups_per_neuron.get(),
				"groups per routed neuron",
			)?)?;
			let neuron_group = builder.binary(ScalarOpcode::Divide, group, divisor)?;
			builder.binary(ScalarOpcode::Equal, neuron, neuron_group)?
		}
		DenseGroupToNeuronRouting::FullyConnected { .. } => builder.i32(1)?,
	};
	let allowed = builder.unary(ScalarOpcode::ConvertI32ToF32, allowed)?;
	Ok(builder.finish(&[allowed])?)
}

fn multiply_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Multiply, left, right)?;
	Ok(builder.finish(&[result])?)
}

fn masked_zero_f32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let validity = builder.input(DType::F32)?;
	let zero = builder.f32(0.0)?;
	let supervised = builder.binary(ScalarOpcode::GreaterThan, validity, zero)?;
	let masked = builder.ternary(ScalarOpcode::Select, supervised, value, zero)?;
	Ok(builder.finish(&[masked])?)
}

fn masked_zero_i32_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::I32)?;
	let validity = builder.input(DType::F32)?;
	let zero_f32 = builder.f32(0.0)?;
	let supervised = builder.binary(ScalarOpcode::GreaterThan, validity, zero_f32)?;
	let zero_i32 = builder.i32(0)?;
	let masked = builder.ternary(ScalarOpcode::Select, supervised, value, zero_i32)?;
	Ok(builder.finish(&[masked])?)
}

fn negative_multiply_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let product = builder.binary(ScalarOpcode::Multiply, left, right)?;
	let result = builder.unary(ScalarOpcode::Negate, product)?;
	Ok(builder.finish(&[result])?)
}

fn pointwise_loss_program(loss: DenseLoss) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let prediction = builder.input(DType::F32)?;
	let target = builder.input(DType::F32)?;
	let prediction_is_finite = builder.unary(ScalarOpcode::IsFinite, prediction)?;
	let target_is_finite = builder.unary(ScalarOpcode::IsFinite, target)?;
	let _ = builder.unary(ScalarOpcode::Require, prediction_is_finite)?;
	let _ = builder.unary(ScalarOpcode::Require, target_is_finite)?;
	let difference = builder.binary(ScalarOpcode::Subtract, prediction, target)?;

	let (loss, gradient) = match loss {
		DenseLoss::MeanSquaredError => {
			let loss = builder.binary(ScalarOpcode::Multiply, difference, difference)?;
			let two = builder.f32(2.0)?;
			let gradient = builder.binary(ScalarOpcode::Multiply, two, difference)?;
			(loss, gradient)
		}
		DenseLoss::MeanAbsoluteError => {
			let loss = builder.unary(ScalarOpcode::Absolute, difference)?;
			let zero = builder.f32(0.0)?;
			let one = builder.f32(1.0)?;
			let negative_one = builder.f32(-1.0)?;
			let positive = builder.binary(ScalarOpcode::GreaterThan, difference, zero)?;
			let negative = builder.binary(ScalarOpcode::LessThan, difference, zero)?;
			let positive_or_zero = builder.ternary(ScalarOpcode::Select, positive, one, zero)?;
			let gradient = builder.ternary(
				ScalarOpcode::Select,
				negative,
				negative_one,
				positive_or_zero,
			)?;
			(loss, gradient)
		}
		DenseLoss::CrossEntropy => {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"categorical cross entropy uses its stable logits program",
			));
		}
		DenseLoss::Huber => {
			let absolute = builder.unary(ScalarOpcode::Absolute, difference)?;
			let one = builder.f32(1.0)?;
			let quadratic_domain = builder.binary(ScalarOpcode::LessThan, absolute, one)?;
			let square = builder.binary(ScalarOpcode::Multiply, difference, difference)?;
			let half = builder.f32(0.5)?;
			let quadratic = builder.binary(ScalarOpcode::Multiply, half, square)?;
			let linear = builder.binary(ScalarOpcode::Subtract, absolute, half)?;
			let loss = builder.ternary(ScalarOpcode::Select, quadratic_domain, quadratic, linear)?;
			let negative_one = builder.f32(-1.0)?;
			let lower_bounded = builder.binary(ScalarOpcode::Maximum, difference, negative_one)?;
			let gradient = builder.binary(ScalarOpcode::Minimum, lower_bounded, one)?;
			(loss, gradient)
		}
		DenseLoss::BinaryCrossEntropy => {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidNetwork,
				"BCE-with-logits uses its stable dedicated loss program",
			));
		}
	};
	Ok(builder.finish(&[loss, gradient])?)
}

fn cross_entropy_with_logits_program(classes: i32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	if classes <= 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"categorical class count must be positive",
		));
	}
	let mut builder = ScalarProgramBuilder::new()?;
	let logit = builder.input(DType::F32)?;
	let target = builder.input(DType::I32)?;
	let class_index = builder.input(DType::I32)?;
	let maximum = builder.input(DType::F32)?;
	let logarithmic_sum = builder.input(DType::F32)?;
	let exponential = builder.input(DType::F32)?;
	let exponential_sum = builder.input(DType::F32)?;
	for value in [
		logit,
		maximum,
		logarithmic_sum,
		exponential,
		exponential_sum,
	] {
		let finite = builder.unary(ScalarOpcode::IsFinite, value)?;
		let _ = builder.unary(ScalarOpcode::Require, finite)?;
	}
	let zero = builder.i32(0)?;
	let class_count = builder.i32(classes)?;
	let target_nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, target, zero)?;
	let target_below_class_count = builder.binary(ScalarOpcode::LessThan, target, class_count)?;
	let target_in_range = builder.binary(
		ScalarOpcode::BitAnd,
		target_nonnegative,
		target_below_class_count,
	)?;
	let _ = builder.unary(ScalarOpcode::Require, target_in_range)?;
	let target_indicator = builder.binary(ScalarOpcode::Equal, target, class_index)?;
	let target_indicator = builder.unary(ScalarOpcode::ConvertI32ToF32, target_indicator)?;
	let log_partition = builder.binary(ScalarOpcode::Add, maximum, logarithmic_sum)?;
	let negative_log_probability = builder.binary(ScalarOpcode::Subtract, log_partition, logit)?;
	let loss = builder.binary(
		ScalarOpcode::Multiply,
		target_indicator,
		negative_log_probability,
	)?;
	let probability = builder.binary(ScalarOpcode::Divide, exponential, exponential_sum)?;
	let gradient = builder.binary(ScalarOpcode::Subtract, probability, target_indicator)?;
	Ok(builder.finish(&[loss, gradient])?)
}

fn signed_log_one_plus_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let upstream = builder.input(DType::F32)?;
	let value = builder.input(DType::F32)?;
	let absolute = builder.unary(ScalarOpcode::Absolute, value)?;
	let one = builder.f32(1.0)?;
	let denominator = builder.binary(ScalarOpcode::Add, one, absolute)?;
	let result = builder.binary(ScalarOpcode::Divide, upstream, denominator)?;
	Ok(builder.finish(&[result])?)
}

fn huber_activation_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let absolute = builder.unary(ScalarOpcode::Absolute, value)?;
	let one = builder.f32(1.0)?;
	let quadratic_domain = builder.binary(ScalarOpcode::LessThanOrEqual, absolute, one)?;
	let square = builder.binary(ScalarOpcode::Multiply, value, value)?;
	let half = builder.f32(0.5)?;
	let quadratic = builder.binary(ScalarOpcode::Multiply, half, square)?;
	let linear = builder.binary(ScalarOpcode::Subtract, absolute, half)?;
	let result = builder.ternary(ScalarOpcode::Select, quadratic_domain, quadratic, linear)?;
	Ok(builder.finish(&[result])?)
}

fn huber_activation_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let upstream = builder.input(DType::F32)?;
	let value = builder.input(DType::F32)?;
	let negative_one = builder.f32(-1.0)?;
	let one = builder.f32(1.0)?;
	let lower_bounded = builder.binary(ScalarOpcode::Maximum, value, negative_one)?;
	let derivative = builder.binary(ScalarOpcode::Minimum, lower_bounded, one)?;
	let result = builder.binary(ScalarOpcode::Multiply, upstream, derivative)?;
	Ok(builder.finish(&[result])?)
}

fn tangent_activation_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let upstream = builder.input(DType::F32)?;
	let tangent = builder.input(DType::F32)?;
	let square = builder.binary(ScalarOpcode::Multiply, tangent, tangent)?;
	let one = builder.f32(1.0)?;
	let derivative = builder.binary(ScalarOpcode::Add, one, square)?;
	let result = builder.binary(ScalarOpcode::Multiply, upstream, derivative)?;
	Ok(builder.finish(&[result])?)
}

fn divide_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let numerator = builder.input(DType::F32)?;
	let denominator = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Divide, numerator, denominator)?;
	Ok(builder.finish(&[result])?)
}

fn safe_count_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let count = builder.input(DType::F32)?;
	let one = builder.f32(1.0)?;
	let safe = builder.binary(ScalarOpcode::Maximum, count, one)?;
	Ok(builder.finish(&[safe])?)
}

fn subtract_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Subtract, left, right)?;
	Ok(builder.finish(&[result])?)
}

fn square_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Multiply, value, value)?;
	Ok(builder.finish(&[result])?)
}

fn multiply_constant_program(constant: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let constant = builder.f32(constant)?;
	let result = builder.binary(ScalarOpcode::Multiply, value, constant)?;
	Ok(builder.finish(&[result])?)
}

fn divide_constant_program(divisor: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let divisor = builder.f32(divisor)?;
	let result = builder.binary(ScalarOpcode::Divide, value, divisor)?;
	Ok(builder.finish(&[result])?)
}

fn z_score_program(epsilon: f32, masked: bool) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let mean = builder.input(DType::F32)?;
	let variance = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let epsilon = builder.f32(epsilon)?;
	let variance = builder.binary(ScalarOpcode::Maximum, variance, epsilon)?;
	let standard_deviation = builder.unary(ScalarOpcode::SquareRoot, variance)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, mean)?;
	let mut normalized = builder.binary(ScalarOpcode::Divide, centered, standard_deviation)?;
	if let Some(mask) = mask {
		let zero = builder.f32(0.0)?;
		let normalize = builder.binary(ScalarOpcode::GreaterThan, mask, zero)?;
		normalized = builder.ternary(ScalarOpcode::Select, normalize, normalized, value)?;
	}
	Ok(builder.finish(&[normalized])?)
}

fn min_max_program(epsilon: f32, masked: bool) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let minimum = builder.input(DType::F32)?;
	let maximum = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let range = builder.binary(ScalarOpcode::Subtract, maximum, minimum)?;
	let epsilon = builder.f32(epsilon)?;
	let denominator = builder.binary(ScalarOpcode::Maximum, range, epsilon)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, minimum)?;
	let mut normalized = builder.binary(ScalarOpcode::Divide, centered, denominator)?;
	if let Some(mask) = mask {
		let zero = builder.f32(0.0)?;
		let normalize = builder.binary(ScalarOpcode::GreaterThan, mask, zero)?;
		normalized = builder.ternary(ScalarOpcode::Select, normalize, normalized, value)?;
	}
	Ok(builder.finish(&[normalized])?)
}

fn l2_square_program(masked: bool) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let mut square = builder.binary(ScalarOpcode::Multiply, value, value)?;
	if let Some(mask) = mask {
		square = builder.binary(ScalarOpcode::Multiply, square, mask)?;
	}
	Ok(builder.finish(&[square])?)
}

fn l2_norm_program(epsilon: f32, masked: bool) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let norm_squared = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let epsilon = builder.f32(epsilon)?;
	let norm_squared = builder.binary(ScalarOpcode::Maximum, norm_squared, epsilon)?;
	let norm = builder.unary(ScalarOpcode::SquareRoot, norm_squared)?;
	let mut normalized = builder.binary(ScalarOpcode::Divide, value, norm)?;
	if let Some(mask) = mask {
		let zero = builder.f32(0.0)?;
		let normalize = builder.binary(ScalarOpcode::GreaterThan, mask, zero)?;
		normalized = builder.ternary(ScalarOpcode::Select, normalize, normalized, value)?;
	}
	Ok(builder.finish(&[normalized])?)
}

fn inverse_standard_deviation_program(epsilon: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let variance = builder.input(DType::F32)?;
	let epsilon = builder.f32(epsilon)?;
	let variance = builder.binary(ScalarOpcode::Maximum, variance, epsilon)?;
	let standard_deviation = builder.unary(ScalarOpcode::SquareRoot, variance)?;
	let one = builder.f32(1.0)?;
	let inverse = builder.binary(ScalarOpcode::Divide, one, standard_deviation)?;
	Ok(builder.finish(&[inverse])?)
}

fn normalization_backward_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let gradient = builder.input(DType::F32)?;
	let gradient_mean = builder.input(DType::F32)?;
	let normalized = builder.input(DType::F32)?;
	let gradient_normalized_mean = builder.input(DType::F32)?;
	let inverse_standard_deviation = builder.input(DType::F32)?;
	let centered_gradient = builder.binary(ScalarOpcode::Subtract, gradient, gradient_mean)?;
	let projection = builder.binary(ScalarOpcode::Multiply, normalized, gradient_normalized_mean)?;
	let adjusted = builder.binary(ScalarOpcode::Subtract, centered_gradient, projection)?;
	let result = builder.binary(ScalarOpcode::Multiply, adjusted, inverse_standard_deviation)?;
	Ok(builder.finish(&[result])?)
}

fn masked_mean_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let valid = builder.input(DType::F32)?;
	let valid_count = builder.input(DType::F32)?;
	let zero = builder.f32(0.0)?;
	let supervised = builder.binary(ScalarOpcode::GreaterThan, valid, zero)?;
	let safe_value = builder.ternary(ScalarOpcode::Select, supervised, value, zero)?;
	let normalized = builder.binary(ScalarOpcode::Divide, safe_value, valid_count)?;
	let masked = builder.ternary(ScalarOpcode::Select, supervised, normalized, zero)?;
	Ok(builder.finish(&[masked])?)
}

fn sum_program(inputs: usize) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let mut values = (0..inputs)
		.map(|_| builder.input(DType::F32))
		.collect::<Result<Vec<_>, _>>()?
		.into_iter();
	let mut total = values.next().ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"global clipping requires at least one parameter gradient",
		)
	})?;
	for value in values {
		total = builder.binary(ScalarOpcode::Add, total, value)?;
	}
	Ok(builder.finish(&[total])?)
}

fn clip_scale_program(maximum_norm: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let squared_norm = builder.input(DType::F32)?;
	let norm = builder.unary(ScalarOpcode::SquareRoot, squared_norm)?;
	let epsilon = builder.f32(f32::EPSILON)?;
	let denominator = builder.binary(ScalarOpcode::Maximum, norm, epsilon)?;
	let maximum = builder.f32(maximum_norm)?;
	let ratio = builder.binary(ScalarOpcode::Divide, maximum, denominator)?;
	let one = builder.f32(1.0)?;
	let scale = builder.binary(ScalarOpcode::Minimum, one, ratio)?;
	Ok(builder.finish(&[scale])?)
}

fn schedule_inputs_program(
	warmup_iterations: u64,
	total_iterations: u64,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	if total_iterations == 0 || total_iterations > i32::MAX as u64 || warmup_iterations > total_iterations {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"training schedule requires 1..=i32::MAX updates and warmup no longer than training",
		));
	}
	let warmup_iterations_i32 = warmup_iterations as i32;
	let mut builder = ScalarProgramBuilder::new()?;
	let step = builder.input(DType::I32)?;
	let one = builder.f32(1.0)?;
	let zero_i32 = builder.i32(0)?;
	let warmup = if warmup_iterations == 0 {
		one
	} else {
		let warmup_end = builder.i32(warmup_iterations_i32)?;
		let warmup_step = builder.binary(ScalarOpcode::Minimum, step, warmup_end)?;
		exact_nonnegative_i32_ratio(&mut builder, warmup_step, warmup_iterations_i32)?
	};
	let decay_start = warmup_iterations.max(1);
	let (progress, remaining_fraction) = if total_iterations <= decay_start {
		(builder.f32(0.0)?, one)
	} else {
		let decay_start = builder.i32(decay_start as i32)?;
		let elapsed = builder.binary(ScalarOpcode::Subtract, step, decay_start)?;
		let elapsed = builder.binary(ScalarOpcode::Maximum, elapsed, zero_i32)?;
		let decay_iterations = total_iterations.saturating_sub(warmup_iterations.max(1));
		let decay_end = builder.i32(decay_iterations as i32)?;
		let elapsed = builder.binary(ScalarOpcode::Minimum, elapsed, decay_end)?;
		let remaining = builder.binary(ScalarOpcode::Subtract, decay_end, elapsed)?;
		(
			exact_nonnegative_i32_ratio(&mut builder, elapsed, decay_iterations as i32)?,
			exact_nonnegative_i32_ratio(&mut builder, remaining, decay_iterations as i32)?,
		)
	};
	Ok(builder.finish(&[warmup, progress, remaining_fraction])?)
}

fn exact_nonnegative_i32_ratio(
	builder: &mut ScalarProgramBuilder,
	numerator: ScalarExpression,
	denominator: i32,
) -> TrainingCompileResult<ScalarExpression> {
	const LIMB_BITS: i32 = 12;
	const LIMB_MASK: i32 = (1 << LIMB_BITS) - 1;
	const LIMB_SCALE: f32 = (1 << LIMB_BITS) as f32;
	if denominator <= 0 {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidOptimizer,
			"schedule ratio denominator must be positive",
		));
	}

	if denominator <= LIMB_MASK {
		let terminal = builder.i32(denominator)?;
		let terminal = builder.binary(ScalarOpcode::Equal, numerator, terminal)?;
		let numerator = builder.unary(ScalarOpcode::ConvertI32ToF32, numerator)?;
		let denominator = builder.f32(denominator as f32)?;
		let ratio = builder.binary(ScalarOpcode::Divide, numerator, denominator)?;
		let one = builder.f32(1.0)?;
		return Ok(builder.ternary(ScalarOpcode::Select, terminal, one, ratio)?);
	}

	let terminal = builder.i32(denominator)?;
	let terminal = builder.binary(ScalarOpcode::Equal, numerator, terminal)?;
	let shift = builder.i32(LIMB_BITS)?;
	let mask = builder.i32(LIMB_MASK)?;
	let numerator_high = builder.binary(ScalarOpcode::ShiftRightLogical, numerator, shift)?;
	let numerator_low = builder.binary(ScalarOpcode::BitAnd, numerator, mask)?;
	let numerator_high = builder.unary(ScalarOpcode::ConvertI32ToF32, numerator_high)?;
	let numerator_low = builder.unary(ScalarOpcode::ConvertI32ToF32, numerator_low)?;
	let limb_scale = builder.f32(LIMB_SCALE)?;
	let numerator_low = builder.binary(ScalarOpcode::Divide, numerator_low, limb_scale)?;

	let denominator_high = builder.f32((denominator >> LIMB_BITS) as f32)?;
	let denominator_low = builder.f32((denominator & LIMB_MASK) as f32 / LIMB_SCALE)?;
	let quotient = builder.binary(ScalarOpcode::Divide, numerator_high, denominator_high)?;
	let negative_quotient = builder.unary(ScalarOpcode::Negate, quotient)?;
	let residual = builder.ternary(
		ScalarOpcode::Fma,
		negative_quotient,
		denominator_high,
		numerator_high,
	)?;
	let residual = builder.binary(ScalarOpcode::Add, residual, numerator_low)?;
	let residual = builder.ternary(
		ScalarOpcode::Fma,
		negative_quotient,
		denominator_low,
		residual,
	)?;
	let denominator = builder.binary(ScalarOpcode::Add, denominator_high, denominator_low)?;
	let correction = builder.binary(ScalarOpcode::Divide, residual, denominator)?;
	let ratio = builder.binary(ScalarOpcode::Add, quotient, correction)?;
	let zero = builder.f32(0.0)?;
	let ratio = builder.binary(ScalarOpcode::Maximum, zero, ratio)?;
	let one = builder.f32(1.0)?;
	let ratio = builder.binary(ScalarOpcode::Minimum, one, ratio)?;
	Ok(builder.ternary(ScalarOpcode::Select, terminal, one, ratio)?)
}

fn cosine_angle_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let remaining_fraction = builder.input(DType::F32)?;
	let half_pi = builder.f32(core::f32::consts::FRAC_PI_2)?;
	let angle = builder.binary(ScalarOpcode::Multiply, remaining_fraction, half_pi)?;
	Ok(builder.finish(&[angle])?)
}

fn cosine_decay_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let sine = builder.input(DType::F32)?;
	let remaining_fraction = builder.input(DType::F32)?;
	let decay = builder.binary(ScalarOpcode::Multiply, sine, sine)?;
	let zero = builder.f32(0.0)?;
	let one = builder.f32(1.0)?;
	let at_start = builder.binary(ScalarOpcode::Equal, remaining_fraction, one)?;
	let at_end = builder.binary(ScalarOpcode::Equal, remaining_fraction, zero)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_start, one, decay)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_end, zero, decay)?;
	Ok(builder.finish(&[decay])?)
}

fn exponential_decay_argument_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let remaining_fraction = builder.input(DType::F32)?;
	let rate = builder.f32(5.0)?;
	let argument = builder.binary(ScalarOpcode::Multiply, rate, remaining_fraction)?;
	Ok(builder.finish(&[argument])?)
}

fn exponential_decay_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let curve = builder.input(DType::F32)?;
	let remaining_fraction = builder.input(DType::F32)?;
	let end = builder.f32((-5.0_f32).exp())?;
	let one = builder.f32(1.0)?;
	let denominator = builder.binary(ScalarOpcode::Subtract, one, end)?;
	let scale = builder.binary(ScalarOpcode::Divide, end, denominator)?;
	let decay = builder.binary(ScalarOpcode::Multiply, scale, curve)?;
	let zero = builder.f32(0.0)?;
	let at_start = builder.binary(ScalarOpcode::Equal, remaining_fraction, one)?;
	let at_end = builder.binary(ScalarOpcode::Equal, remaining_fraction, zero)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_start, one, decay)?;
	let decay = builder.ternary(ScalarOpcode::Select, at_end, zero, decay)?;
	Ok(builder.finish(&[decay])?)
}

fn learning_rate_program(
	base_learning_rate: f32,
	gated_by_early_stopping: bool,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let decay = builder.input(DType::F32)?;
	let warmup = builder.input(DType::F32)?;
	let stopped = gated_by_early_stopping
		.then(|| builder.input(DType::I32))
		.transpose()?;
	let factor = builder.binary(ScalarOpcode::Multiply, warmup, decay)?;
	let base = builder.f32(base_learning_rate)?;
	let mut learning_rate = builder.binary(ScalarOpcode::Multiply, base, factor)?;
	if let Some(stopped) = stopped {
		let zero_i32 = builder.i32(0)?;
		let running = builder.binary(ScalarOpcode::Equal, stopped, zero_i32)?;
		let zero_f32 = builder.f32(0.0)?;
		learning_rate = builder.ternary(ScalarOpcode::Select, running, learning_rate, zero_f32)?;
	}
	Ok(builder.finish(&[learning_rate])?)
}

fn optimizer_update_gate_program(gated_by_early_stopping: bool) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let known_count = builder.input(DType::F32)?;
	let zero_f32 = builder.f32(0.0)?;
	let mut enabled = builder.binary(ScalarOpcode::GreaterThan, known_count, zero_f32)?;
	if gated_by_early_stopping {
		let stopped = builder.input(DType::I32)?;
		let zero_i32 = builder.i32(0)?;
		let running = builder.binary(ScalarOpcode::Equal, stopped, zero_i32)?;
		enabled = builder.binary(ScalarOpcode::BitAnd, enabled, running)?;
	}
	Ok(builder.finish(&[enabled])?)
}

fn accepted_update_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let accepted = builder.input(DType::I32)?;
	let enabled = builder.input(DType::I32)?;
	let one = builder.i32(1)?;
	let candidate = builder.binary(ScalarOpcode::Add, accepted, one)?;
	let updated = builder.ternary(ScalarOpcode::Select, enabled, candidate, accepted)?;
	Ok(builder.finish(&[updated])?)
}

fn gated_learning_rate_program(base_learning_rate: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let decay = builder.input(DType::F32)?;
	let warmup = builder.input(DType::F32)?;
	let enabled = builder.input(DType::I32)?;
	let factor = builder.binary(ScalarOpcode::Multiply, warmup, decay)?;
	let base = builder.f32(base_learning_rate)?;
	let candidate = builder.binary(ScalarOpcode::Multiply, base, factor)?;
	let zero = builder.f32(0.0)?;
	let learning_rate = builder.ternary(ScalarOpcode::Select, enabled, candidate, zero)?;
	Ok(builder.finish(&[learning_rate])?)
}

fn adam_beta_power_initial_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let _seed = builder.input(DType::I32)?;
	let beta_one_power = builder.f32(1.0)?;
	let beta_two_power = builder.f32(1.0)?;
	Ok(builder.finish(&[beta_one_power, beta_two_power])?)
}

fn adam_beta_power_update_program(beta_one: f32, beta_two: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let beta_one_power = builder.input(DType::F32)?;
	let beta_two_power = builder.input(DType::F32)?;
	let beta_one = builder.f32(beta_one)?;
	let beta_two = builder.f32(beta_two)?;
	let beta_one_power = builder.binary(ScalarOpcode::Multiply, beta_one_power, beta_one)?;
	let beta_two_power = builder.binary(ScalarOpcode::Multiply, beta_two_power, beta_two)?;
	Ok(builder.finish(&[beta_one_power, beta_two_power])?)
}

fn gated_adam_beta_power_update_program(
	beta_one: f32,
	beta_two: f32,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let beta_one_power = builder.input(DType::F32)?;
	let beta_two_power = builder.input(DType::F32)?;
	let enabled = builder.input(DType::I32)?;
	let beta_one = builder.f32(beta_one)?;
	let beta_two = builder.f32(beta_two)?;
	let candidate_one = builder.binary(ScalarOpcode::Multiply, beta_one_power, beta_one)?;
	let candidate_two = builder.binary(ScalarOpcode::Multiply, beta_two_power, beta_two)?;
	let updated_one = builder.ternary(ScalarOpcode::Select, enabled, candidate_one, beta_one_power)?;
	let updated_two = builder.ternary(ScalarOpcode::Select, enabled, candidate_two, beta_two_power)?;
	Ok(builder.finish(&[updated_one, updated_two])?)
}

fn adamw_program(
	beta_one: f32,
	beta_two: f32,
	epsilon: f32,
	weight_decay: f32,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let gradient = builder.input(DType::F32)?;
	let weight = builder.input(DType::F32)?;
	let first_moment = builder.input(DType::F32)?;
	let second_moment = builder.input(DType::F32)?;
	let learning_rate = builder.input(DType::F32)?;
	let beta_one_power = builder.input(DType::F32)?;
	let beta_two_power = builder.input(DType::F32)?;

	let one = builder.f32(1.0)?;
	let beta_one = builder.f32(beta_one)?;
	let beta_two = builder.f32(beta_two)?;
	let one_minus_beta_one = builder.binary(ScalarOpcode::Subtract, one, beta_one)?;
	let one_minus_beta_two = builder.binary(ScalarOpcode::Subtract, one, beta_two)?;

	let retained_first = builder.binary(ScalarOpcode::Multiply, beta_one, first_moment)?;
	let gradient_first = builder.binary(ScalarOpcode::Multiply, one_minus_beta_one, gradient)?;
	let updated_first = builder.binary(ScalarOpcode::Add, retained_first, gradient_first)?;

	let retained_second = builder.binary(ScalarOpcode::Multiply, beta_two, second_moment)?;
	let gradient_squared = builder.binary(ScalarOpcode::Multiply, gradient, gradient)?;
	let gradient_second = builder.binary(ScalarOpcode::Multiply, one_minus_beta_two, gradient_squared)?;
	let updated_second = builder.binary(ScalarOpcode::Add, retained_second, gradient_second)?;

	let first_correction = builder.binary(ScalarOpcode::Subtract, one, beta_one_power)?;
	let second_correction = builder.binary(ScalarOpcode::Subtract, one, beta_two_power)?;
	let corrected_first = builder.binary(ScalarOpcode::Divide, updated_first, first_correction)?;
	let corrected_second = builder.binary(ScalarOpcode::Divide, updated_second, second_correction)?;
	let root_second = builder.unary(ScalarOpcode::SquareRoot, corrected_second)?;
	let epsilon = builder.f32(epsilon)?;
	let denominator = builder.binary(ScalarOpcode::Add, root_second, epsilon)?;
	let normalized = builder.binary(ScalarOpcode::Divide, corrected_first, denominator)?;
	let adaptive_step = builder.binary(ScalarOpcode::Multiply, learning_rate, normalized)?;

	let weight_decay = builder.f32(weight_decay)?;
	let decay = builder.binary(ScalarOpcode::Multiply, learning_rate, weight_decay)?;
	let decay = builder.binary(ScalarOpcode::Multiply, decay, weight)?;
	let decayed_weight = builder.binary(ScalarOpcode::Subtract, weight, decay)?;
	let updated_weight = builder.binary(ScalarOpcode::Subtract, decayed_weight, adaptive_step)?;
	Ok(builder.finish(&[updated_first, updated_second, updated_weight])?)
}

fn gated_adamw_program(
	beta_one: f32,
	beta_two: f32,
	epsilon: f32,
	weight_decay: f32,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let gradient = builder.input(DType::F32)?;
	let weight = builder.input(DType::F32)?;
	let first_moment = builder.input(DType::F32)?;
	let second_moment = builder.input(DType::F32)?;
	let learning_rate = builder.input(DType::F32)?;
	let beta_one_power = builder.input(DType::F32)?;
	let beta_two_power = builder.input(DType::F32)?;
	let enabled = builder.input(DType::I32)?;

	let one = builder.f32(1.0)?;
	let beta_one = builder.f32(beta_one)?;
	let beta_two = builder.f32(beta_two)?;
	let one_minus_beta_one = builder.binary(ScalarOpcode::Subtract, one, beta_one)?;
	let one_minus_beta_two = builder.binary(ScalarOpcode::Subtract, one, beta_two)?;

	let retained_first = builder.binary(ScalarOpcode::Multiply, beta_one, first_moment)?;
	let gradient_first = builder.binary(ScalarOpcode::Multiply, one_minus_beta_one, gradient)?;
	let candidate_first = builder.binary(ScalarOpcode::Add, retained_first, gradient_first)?;

	let retained_second = builder.binary(ScalarOpcode::Multiply, beta_two, second_moment)?;
	let gradient_squared = builder.binary(ScalarOpcode::Multiply, gradient, gradient)?;
	let gradient_second = builder.binary(ScalarOpcode::Multiply, one_minus_beta_two, gradient_squared)?;
	let candidate_second = builder.binary(ScalarOpcode::Add, retained_second, gradient_second)?;

	let safe_beta_one_power = builder.ternary(ScalarOpcode::Select, enabled, beta_one_power, beta_one)?;
	let safe_beta_two_power = builder.ternary(ScalarOpcode::Select, enabled, beta_two_power, beta_two)?;
	let first_correction = builder.binary(ScalarOpcode::Subtract, one, safe_beta_one_power)?;
	let second_correction = builder.binary(ScalarOpcode::Subtract, one, safe_beta_two_power)?;
	let corrected_first = builder.binary(ScalarOpcode::Divide, candidate_first, first_correction)?;
	let corrected_second = builder.binary(ScalarOpcode::Divide, candidate_second, second_correction)?;
	let root_second = builder.unary(ScalarOpcode::SquareRoot, corrected_second)?;
	let epsilon = builder.f32(epsilon)?;
	let denominator = builder.binary(ScalarOpcode::Add, root_second, epsilon)?;
	let normalized = builder.binary(ScalarOpcode::Divide, corrected_first, denominator)?;
	let adaptive_step = builder.binary(ScalarOpcode::Multiply, learning_rate, normalized)?;

	let weight_decay = builder.f32(weight_decay)?;
	let decay = builder.binary(ScalarOpcode::Multiply, learning_rate, weight_decay)?;
	let decay = builder.binary(ScalarOpcode::Multiply, decay, weight)?;
	let decayed_weight = builder.binary(ScalarOpcode::Subtract, weight, decay)?;
	let candidate_weight = builder.binary(ScalarOpcode::Subtract, decayed_weight, adaptive_step)?;
	let updated_first = builder.ternary(ScalarOpcode::Select, enabled, candidate_first, first_moment)?;
	let updated_second = builder.ternary(
		ScalarOpcode::Select,
		enabled,
		candidate_second,
		second_moment,
	)?;
	let updated_weight = builder.ternary(ScalarOpcode::Select, enabled, candidate_weight, weight)?;
	Ok(builder.finish(&[updated_first, updated_second, updated_weight])?)
}

fn temperature_gradient_program(population: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let scaled_logit_gradient = builder.input(DType::F32)?;
	let logit = builder.input(DType::F32)?;
	let temperature = builder.input(DType::F32)?;
	let product = builder.binary(ScalarOpcode::Multiply, scaled_logit_gradient, logit)?;
	let numerator = builder.unary(ScalarOpcode::Negate, product)?;
	let temperature_squared = builder.binary(ScalarOpcode::Multiply, temperature, temperature)?;
	let population = builder.f32(population)?;
	let denominator = builder.binary(ScalarOpcode::Multiply, temperature_squared, population)?;
	let contribution = builder.binary(ScalarOpcode::Divide, numerator, denominator)?;
	Ok(builder.finish(&[contribution])?)
}

fn temperature_update_program(config: TemperatureScalingConfig) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let temperature = builder.input(DType::F32)?;
	let gradient = builder.input(DType::F32)?;
	let learning_rate = builder.f32(config.learning_rate)?;
	let step = builder.binary(ScalarOpcode::Multiply, learning_rate, gradient)?;
	let candidate = builder.binary(ScalarOpcode::Subtract, temperature, step)?;
	let minimum = builder.f32(config.minimum_temperature)?;
	let bounded = builder.binary(ScalarOpcode::Maximum, candidate, minimum)?;
	let maximum = builder.f32(config.maximum_temperature)?;
	let bounded = builder.binary(ScalarOpcode::Minimum, bounded, maximum)?;
	Ok(builder.finish(&[bounded])?)
}

#[cfg(test)]
mod tests {
	use super::*;
	use recipe_core::{ScalarLiteral, ScalarProgram, ScalarValueId};

	#[derive(Clone, Copy, Debug)]
	enum TestValue {
		F32(f32),
		I32(i32),
	}

	impl TestValue {
		fn f32(self) -> f32 {
			match self {
				Self::F32(value) => value,
				Self::I32(_) => panic!("expected f32"),
			}
		}

		fn i32(self) -> i32 {
			match self {
				Self::I32(value) => value,
				Self::F32(_) => panic!("expected i32"),
			}
		}
	}

	fn evaluate(program: &ScalarProgram, inputs: &[f32]) -> Vec<f32> {
		let inputs = inputs
			.iter()
			.copied()
			.map(TestValue::F32)
			.collect::<Vec<_>>();
		evaluate_values(program, &inputs)
			.into_iter()
			.map(TestValue::f32)
			.collect()
	}

	fn evaluate_i32(program: &ScalarProgram, inputs: &[i32]) -> Vec<f32> {
		let inputs = inputs
			.iter()
			.copied()
			.map(TestValue::I32)
			.collect::<Vec<_>>();
		evaluate_values(program, &inputs)
			.into_iter()
			.map(TestValue::f32)
			.collect()
	}

	fn evaluate_values(program: &ScalarProgram, inputs: &[TestValue]) -> Vec<TestValue> {
		assert_eq!(program.inputs.len(), inputs.len());
		let mut values = BTreeMap::<ScalarValueId, TestValue>::new();
		for (input, value) in program.inputs.iter().zip(inputs) {
			assert_eq!(
				input.dtype,
				match value {
					TestValue::F32(_) => DType::F32,
					TestValue::I32(_) => DType::I32,
				}
			);
			values.insert(input.id, *value);
		}
		for constant in &program.constants {
			let value = match constant.value {
				ScalarLiteral::F32Bits(bits) => TestValue::F32(f32::from_bits(bits)),
				ScalarLiteral::I32(value) => TestValue::I32(value),
			};
			values.insert(constant.id, value);
		}
		for instruction in &program.instructions {
			let operands = instruction
				.operands
				.iter()
				.map(|operand| values[operand])
				.collect::<Vec<_>>();
			let value = match instruction.opcode {
				ScalarOpcode::Add => match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => TestValue::F32(left + right),
					[TestValue::I32(left), TestValue::I32(right)] => TestValue::I32(left + right),
					_ => panic!("mixed Add operands"),
				},
				ScalarOpcode::Subtract => match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => TestValue::F32(left - right),
					[TestValue::I32(left), TestValue::I32(right)] => TestValue::I32(left - right),
					_ => panic!("mixed Subtract operands"),
				},
				ScalarOpcode::Multiply => match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => TestValue::F32(left * right),
					[TestValue::I32(left), TestValue::I32(right)] => TestValue::I32(left * right),
					_ => panic!("mixed Multiply operands"),
				},
				ScalarOpcode::Divide => match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => TestValue::F32(left / right),
					[TestValue::I32(left), TestValue::I32(right)] => TestValue::I32(left / right),
					_ => panic!("mixed Divide operands"),
				},
				ScalarOpcode::Remainder => match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => TestValue::F32(left % right),
					[TestValue::I32(left), TestValue::I32(right)] => TestValue::I32(left % right),
					_ => panic!("mixed Remainder operands"),
				},
				ScalarOpcode::Negate => match operands[0] {
					TestValue::F32(value) => TestValue::F32(-value),
					TestValue::I32(value) => TestValue::I32(-value),
				},
				ScalarOpcode::Absolute => match operands[0] {
					TestValue::F32(value) => TestValue::F32(value.abs()),
					TestValue::I32(value) => TestValue::I32(value.abs()),
				},
				ScalarOpcode::Fma => TestValue::F32(
					operands[0]
						.f32()
						.mul_add(operands[1].f32(), operands[2].f32()),
				),
				ScalarOpcode::SquareRoot => TestValue::F32(operands[0].f32().sqrt()),
				ScalarOpcode::Minimum => match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => TestValue::F32(left.min(*right)),
					[TestValue::I32(left), TestValue::I32(right)] => TestValue::I32((*left).min(*right)),
					_ => panic!("mixed Minimum operands"),
				},
				ScalarOpcode::Maximum => match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => TestValue::F32(left.max(*right)),
					[TestValue::I32(left), TestValue::I32(right)] => TestValue::I32((*left).max(*right)),
					_ => panic!("mixed Maximum operands"),
				},
				ScalarOpcode::Equal => TestValue::I32(i32::from(match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => left == right,
					[TestValue::I32(left), TestValue::I32(right)] => left == right,
					_ => panic!("mixed Equal operands"),
				})),
				ScalarOpcode::NotEqual => TestValue::I32(i32::from(match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => left != right,
					[TestValue::I32(left), TestValue::I32(right)] => left != right,
					_ => panic!("mixed NotEqual operands"),
				})),
				ScalarOpcode::LessThan => TestValue::I32(i32::from(match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => left < right,
					[TestValue::I32(left), TestValue::I32(right)] => left < right,
					_ => panic!("mixed LessThan operands"),
				})),
				ScalarOpcode::LessThanOrEqual => TestValue::I32(i32::from(match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => left <= right,
					[TestValue::I32(left), TestValue::I32(right)] => left <= right,
					_ => panic!("mixed LessThanOrEqual operands"),
				})),
				ScalarOpcode::GreaterThan => TestValue::I32(i32::from(match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => left > right,
					[TestValue::I32(left), TestValue::I32(right)] => left > right,
					_ => panic!("mixed GreaterThan operands"),
				})),
				ScalarOpcode::GreaterThanOrEqual => TestValue::I32(i32::from(match operands.as_slice() {
					[TestValue::F32(left), TestValue::F32(right)] => left >= right,
					[TestValue::I32(left), TestValue::I32(right)] => left >= right,
					_ => panic!("mixed GreaterThanOrEqual operands"),
				})),
				ScalarOpcode::Select => {
					if operands[0].i32() != 0 {
						operands[1]
					} else {
						operands[2]
					}
				}
				ScalarOpcode::BitAnd => TestValue::I32(operands[0].i32() & operands[1].i32()),
				ScalarOpcode::BitOr => TestValue::I32(operands[0].i32() | operands[1].i32()),
				ScalarOpcode::BitXor => TestValue::I32(operands[0].i32() ^ operands[1].i32()),
				ScalarOpcode::BitNot => TestValue::I32(!operands[0].i32()),
				ScalarOpcode::ShiftLeft => {
					TestValue::I32(operands[0].i32().wrapping_shl(operands[1].i32() as u32))
				}
				ScalarOpcode::ShiftRightLogical => {
					TestValue::I32(((operands[0].i32() as u32).wrapping_shr(operands[1].i32() as u32)) as i32)
				}
				ScalarOpcode::ShiftRightArithmetic => {
					TestValue::I32(operands[0].i32().wrapping_shr(operands[1].i32() as u32))
				}
				ScalarOpcode::ConvertI32ToF32 => TestValue::F32(operands[0].i32() as f32),
				ScalarOpcode::IsFinite => TestValue::I32(i32::from(operands[0].f32().is_finite())),
				ScalarOpcode::Require => {
					assert_ne!(operands[0].i32(), 0);
					operands[0]
				}
				other => panic!("unsupported test opcode {other:?}"),
			};
			values.insert(instruction.result, value);
		}
		program
			.outputs
			.iter()
			.map(|output| values[output])
			.collect()
	}

	fn assert_close(actual: &[f32], expected: [f32; 2]) {
		for (actual, expected) in actual.iter().zip(expected) {
			assert!(
				(actual - expected).abs() <= 0.000_001,
				"{actual} != {expected}"
			);
		}
	}

	fn assert_table_close(actual: &[f32], expected: &[f32]) {
		assert_eq!(actual.len(), expected.len());
		for (actual, expected) in actual.iter().zip(expected) {
			assert!(
				(actual - expected).abs() <= 0.000002,
				"{actual} != {expected}"
			);
		}
	}

	fn learning_rate_from_coordinates(coordinates: &[f32], schedule: LearningRateDecay) -> f32 {
		let warmup = coordinates[0];
		let remaining_fraction = coordinates[2];
		let decay = match schedule {
			LearningRateDecay::Linear => remaining_fraction,
			LearningRateDecay::Cosine => {
				let angle = evaluate(&cosine_angle_program().unwrap(), &[remaining_fraction])[0];
				evaluate(
					&cosine_decay_program().unwrap(),
					&[angle.sin(), remaining_fraction],
				)[0]
			}
			LearningRateDecay::Exponential => {
				let argument = evaluate(
					&exponential_decay_argument_program().unwrap(),
					&[remaining_fraction],
				)[0];
				evaluate(
					&exponential_decay_program().unwrap(),
					&[argument.exp_m1(), remaining_fraction],
				)[0]
			}
		};
		evaluate(
			&learning_rate_program(1.0, false).unwrap(),
			&[decay, warmup],
		)[0]
	}

	fn schedule_learning_rates(total: u64, warmup: u64, decay: LearningRateDecay) -> Vec<f32> {
		let schedule = schedule_inputs_program(warmup, total).unwrap();
		(1..=total)
			.map(|step| {
				let coordinates = evaluate_i32(&schedule, &[step as i32]);
				learning_rate_from_coordinates(&coordinates, decay)
			})
			.collect()
	}

	#[test]
	fn group_routing_masks_follow_contiguous_division_rules() {
		let nonzero = |value| NonZeroU64::new(value).unwrap();
		for (routing, channels, input_width, output_width) in [
			(
				DenseGroupToNeuronRouting::Identity { width: nonzero(3) },
				2,
				6,
				3,
			),
			(
				DenseGroupToNeuronRouting::Expand {
					groups: nonzero(4),
					neurons: nonzero(8),
					neurons_per_group: nonzero(2),
				},
				2,
				8,
				8,
			),
			(
				DenseGroupToNeuronRouting::Contract {
					groups: nonzero(8),
					neurons: nonzero(4),
					groups_per_neuron: nonzero(2),
				},
				1,
				8,
				4,
			),
			(
				DenseGroupToNeuronRouting::FullyConnected {
					groups: nonzero(5),
					neurons: nonzero(8),
				},
				1,
				5,
				8,
			),
		] {
			let program = group_routing_mask_program(nonzero(channels), routing).unwrap();
			let (groups, _) = routing_extents(routing);
			for row in 0..input_width {
				let group = row / channels;
				for neuron in 0..output_width {
					let position = row * output_width + neuron;
					let actual = evaluate_i32(&program, &[position as i32])[0];
					let expected = match routing {
						DenseGroupToNeuronRouting::Identity { .. } => neuron == group,
						DenseGroupToNeuronRouting::Expand {
							neurons_per_group, ..
						} => neuron / neurons_per_group.get() == group,
						DenseGroupToNeuronRouting::Contract {
							groups_per_neuron, ..
						} => group / groups_per_neuron.get() == neuron,
						DenseGroupToNeuronRouting::FullyConnected { .. } => true,
					};
					assert_eq!(
						actual,
						if expected { 1.0 } else { 0.0 },
						"routing {routing:?}, group {group}/{}, neuron {neuron}",
						groups.get()
					);
				}
			}
		}
	}

	#[test]
	fn learning_rate_tables_cover_the_full_training_interval() {
		assert_table_close(
			&schedule_learning_rates(5, 0, LearningRateDecay::Linear),
			&[1.0, 0.75, 0.5, 0.25, 0.0],
		);
		assert_table_close(
			&schedule_learning_rates(5, 0, LearningRateDecay::Cosine),
			&[1.0, 0.8535534, 0.5, 0.1464466, 0.0],
		);
		assert_table_close(
			&schedule_learning_rates(5, 0, LearningRateDecay::Exponential),
			&[1.0, 0.2816647, 0.07585818, 0.01689363, 0.0],
		);
	}

	#[test]
	fn warmup_reaches_the_base_rate_before_decay() {
		assert_table_close(
			&schedule_learning_rates(6, 2, LearningRateDecay::Linear),
			&[0.5, 1.0, 0.75, 0.5, 0.25, 0.0],
		);
		assert_table_close(
			&schedule_learning_rates(6, 2, LearningRateDecay::Cosine),
			&[0.5, 1.0, 0.8535534, 0.5, 0.1464466, 0.0],
		);
		assert_table_close(
			&schedule_learning_rates(6, 2, LearningRateDecay::Exponential),
			&[0.5, 1.0, 0.2816647, 0.07585818, 0.01689363, 0.0],
		);
	}

	#[test]
	fn one_update_uses_the_full_base_learning_rate() {
		for warmup in [0, 1] {
			for decay in [
				LearningRateDecay::Linear,
				LearningRateDecay::Cosine,
				LearningRateDecay::Exponential,
			] {
				assert_eq!(schedule_learning_rates(1, warmup, decay), [1.0]);
			}
		}
		let coordinates = evaluate_i32(&schedule_inputs_program(0, 1).unwrap(), &[1]);
		let learning_rate = evaluate(
			&learning_rate_program(0.1, false).unwrap(),
			&[coordinates[2], coordinates[0]],
		)[0];
		let update = evaluate(
			&adamw_program(0.9, 0.999, 0.00000001, 0.0).unwrap(),
			&[1.0, 1.0, 0.0, 0.0, learning_rate, 0.9, 0.999],
		);
		assert!(update[2].is_finite());
		assert!(update[2] < 1.0);
	}

	#[test]
	fn schedule_keeps_exact_integer_limbs_past_the_f32_integer_boundary() {
		let schedule = schedule_inputs_program(0, 16777218).unwrap();
		assert!(
			schedule
				.instructions
				.iter()
				.any(|instruction| instruction.opcode == ScalarOpcode::ShiftRightLogical)
		);
		assert!(
			schedule
				.instructions
				.iter()
				.any(|instruction| instruction.opcode == ScalarOpcode::BitAnd)
		);
		let before_boundary = evaluate_i32(&schedule, &[16777216])[1];
		let after_boundary = evaluate_i32(&schedule, &[16777217])[1];
		assert!(before_boundary < after_boundary);
		assert_eq!(after_boundary, (16777216.0_f64 / 16777217.0_f64) as f32);
		assert_eq!(evaluate_i32(&schedule, &[16777218])[1], 1.0);
	}

	#[test]
	fn every_decay_keeps_a_nonzero_penultimate_rate_at_the_i32_limit() {
		let schedule = schedule_inputs_program(0, i32::MAX as u64).unwrap();
		let penultimate = evaluate_i32(&schedule, &[i32::MAX - 1]);
		let final_update = evaluate_i32(&schedule, &[i32::MAX]);
		for decay in [
			LearningRateDecay::Linear,
			LearningRateDecay::Cosine,
			LearningRateDecay::Exponential,
		] {
			assert!(learning_rate_from_coordinates(&penultimate, decay) > 0.0);
			assert_eq!(learning_rate_from_coordinates(&final_update, decay), 0.0);
		}
	}

	#[test]
	fn large_warmup_does_not_round_the_penultimate_step_to_completion() {
		let schedule = schedule_inputs_program(16777217, 16777218).unwrap();
		let penultimate = evaluate_i32(&schedule, &[16777216]);
		let warmup_end = evaluate_i32(&schedule, &[16777217]);
		assert_eq!(penultimate[0], 0.99999994);
		assert_eq!(warmup_end[0], 1.0);
	}

	#[test]
	fn zero_adam_betas_use_the_finite_recurrent_update() {
		for (beta_one, beta_two, expected) in [
			(0.0, 0.999, [0.0, 0.999]),
			(0.9, 0.0, [0.9, 0.0]),
			(0.0, 0.0, [0.0, 0.0]),
		] {
			let program = adam_beta_power_update_program(beta_one, beta_two).unwrap();
			assert_eq!(evaluate(&program, &[1.0, 1.0]), expected);
			assert!(
				program
					.instructions
					.iter()
					.all(|instruction| instruction.opcode != ScalarOpcode::Require)
			);
		}
		let update = evaluate(
			&adamw_program(0.0, 0.0, 0.00000001, 0.0).unwrap(),
			&[2.0, 1.0, 0.0, 0.0, 0.1, 0.0, 0.0],
		);
		assert!(update.iter().all(|value| value.is_finite()));
		assert_eq!(update[0], 2.0);
		assert_eq!(update[1], 4.0);
	}

	#[test]
	fn rejected_batches_preserve_optimizer_state_bit_for_bit() {
		let gate = optimizer_update_gate_program(false).unwrap();
		let disabled = evaluate_values(&gate, &[TestValue::F32(0.0)])[0].i32();
		assert_eq!(disabled, 0);
		let enabled = evaluate_values(&gate, &[TestValue::F32(2.0)])[0].i32();
		assert_eq!(enabled, 1);
		let stopped_gate = optimizer_update_gate_program(true).unwrap();
		assert_eq!(
			evaluate_values(&stopped_gate, &[TestValue::F32(2.0), TestValue::I32(1)],)[0].i32(),
			0
		);

		let weight = f32::from_bits(0xbf9a_4c21);
		let first = f32::from_bits(0x3e91_7ac3);
		let second = f32::from_bits(0x3f23_d70a);
		let update = evaluate_values(
			&gated_adamw_program(0.9, 0.999, 0.00000001, 0.75).unwrap(),
			&[
				TestValue::F32(7.0),
				TestValue::F32(weight),
				TestValue::F32(first),
				TestValue::F32(second),
				TestValue::F32(0.5),
				TestValue::F32(1.0),
				TestValue::F32(1.0),
				TestValue::I32(0),
			],
		);
		assert_eq!(update[0].f32().to_bits(), first.to_bits());
		assert_eq!(update[1].f32().to_bits(), second.to_bits());
		assert_eq!(update[2].f32().to_bits(), weight.to_bits());
		assert!(update.iter().all(|value| value.f32().is_finite()));

		let unusual = evaluate_values(
			&gated_adamw_program(0.9, 0.999, 0.00000001, 0.75).unwrap(),
			&[
				TestValue::F32(7.0),
				TestValue::F32(f32::from_bits(0x8000_0000)),
				TestValue::F32(f32::from_bits(0x7fc1_2345)),
				TestValue::F32(f32::from_bits(0x0000_0001)),
				TestValue::F32(0.5),
				TestValue::F32(1.0),
				TestValue::F32(1.0),
				TestValue::I32(0),
			],
		);
		assert_eq!(unusual[0].f32().to_bits(), 0x7fc1_2345);
		assert_eq!(unusual[1].f32().to_bits(), 0x0000_0001);
		assert_eq!(unusual[2].f32().to_bits(), 0x8000_0000);
	}

	#[test]
	fn accepted_clock_and_beta_powers_ignore_rejected_batches() {
		let clock = accepted_update_program().unwrap();
		let beta = gated_adam_beta_power_update_program(0.9, 0.999).unwrap();
		let mut step = 0;
		let mut powers = [1.0, 1.0];
		let mut steps = Vec::new();
		let mut powers_one = Vec::new();
		for enabled in [0, 1, 0, 1] {
			step = evaluate_values(&clock, &[TestValue::I32(step), TestValue::I32(enabled)])[0].i32();
			let next = evaluate_values(
				&beta,
				&[
					TestValue::F32(powers[0]),
					TestValue::F32(powers[1]),
					TestValue::I32(enabled),
				],
			);
			powers = [next[0].f32(), next[1].f32()];
			steps.push(step);
			powers_one.push(powers[0]);
		}
		assert_eq!(steps, [0, 1, 1, 2]);
		assert_eq!(powers_one[0].to_bits(), 1.0_f32.to_bits());
		assert_eq!(powers_one[1].to_bits(), 0.9_f32.to_bits());
		assert_eq!(powers_one[2].to_bits(), 0.9_f32.to_bits());
		assert_eq!(powers_one[3].to_bits(), (0.9_f32 * 0.9).to_bits());
	}

	#[test]
	fn zero_supervision_uses_a_safe_denominator_and_exact_zero_loss() {
		let safe = evaluate(&safe_count_program().unwrap(), &[0.0])[0];
		assert_eq!(safe, 1.0);
		for ignored in [13.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
			let masked = evaluate(&masked_zero_f32_program().unwrap(), &[ignored, 0.0])[0];
			assert_eq!(masked.to_bits(), 0.0_f32.to_bits());
			let gradient = evaluate(&masked_mean_program().unwrap(), &[ignored, 0.0, safe])[0];
			assert_eq!(gradient.to_bits(), 0.0_f32.to_bits());
			for loss in [
				DenseLoss::MeanSquaredError,
				DenseLoss::MeanAbsoluteError,
				DenseLoss::Huber,
			] {
				let result = evaluate(&pointwise_loss_program(loss).unwrap(), &[masked, masked]);
				assert!(result.iter().all(|value| value.is_finite()));
				assert!(
					result.iter()
						.all(|value| value.to_bits() == 0.0_f32.to_bits())
				);
			}
		}
		let categorical = evaluate_values(
			&masked_zero_i32_program().unwrap(),
			&[TestValue::I32(i32::MAX), TestValue::F32(0.0)],
		)[0]
		.i32();
		assert_eq!(categorical, 0);
		let loss = evaluate(&divide_program().unwrap(), &[0.0, safe])[0];
		assert_eq!(loss.to_bits(), 0.0_f32.to_bits());
	}

	#[test]
	fn pointwise_losses_and_gradients_match_the_declared_formulas() {
		assert_close(
			&evaluate(
				&pointwise_loss_program(DenseLoss::MeanSquaredError).unwrap(),
				&[3.0, 1.0],
			),
			[4.0, 4.0],
		);
		assert_close(
			&evaluate(
				&pointwise_loss_program(DenseLoss::MeanAbsoluteError).unwrap(),
				&[-2.0, 1.0],
			),
			[3.0, -1.0],
		);
		assert_close(
			&evaluate(
				&pointwise_loss_program(DenseLoss::Huber).unwrap(),
				&[0.5, 0.0],
			),
			[0.125, 0.5],
		);
		assert_close(
			&evaluate(
				&pointwise_loss_program(DenseLoss::Huber).unwrap(),
				&[-2.0, 0.0],
			),
			[1.5, -1.0],
		);
	}

	#[test]
	fn categorical_cross_entropy_program_consumes_logits_and_returns_softmax_gradient() {
		let logits = [2.0f32, 1.0, -1.0];
		let target_code = 2i32;
		let maximum = 2.0f32;
		let exponentials = logits.map(|logit| (logit - maximum).exp());
		let exponential_sum = exponentials.iter().sum::<f32>();
		let logarithmic_sum = exponential_sum.ln();
		for (index, (logit, exponential)) in logits.into_iter().zip(exponentials).enumerate() {
			let target_indicator = f32::from(index == usize::try_from(target_code).unwrap());
			let actual = evaluate_values(
				&cross_entropy_with_logits_program(3).unwrap(),
				&[
					TestValue::F32(logit),
					TestValue::I32(target_code),
					TestValue::I32(i32::try_from(index).unwrap()),
					TestValue::F32(maximum),
					TestValue::F32(logarithmic_sum),
					TestValue::F32(exponential),
					TestValue::F32(exponential_sum),
				],
			)
			.into_iter()
			.map(TestValue::f32)
			.collect::<Vec<_>>();
			let expected_loss = target_indicator * (maximum + logarithmic_sum - logit);
			let expected_gradient = exponential / exponential_sum - target_indicator;
			assert_close(&actual, [expected_loss, expected_gradient]);
			if index == usize::try_from(target_code).unwrap() {
				assert!(actual[0] > 0.0);
			}
		}
	}

	#[test]
	fn data_normalization_programs_leave_categorical_spans_unchanged() {
		assert_eq!(
			evaluate(
				&z_score_program(0.000001, true).unwrap(),
				&[1.0, 0.5, 0.25, 0.0]
			),
			[1.0]
		);
		assert_eq!(
			evaluate(
				&min_max_program(0.000001, true).unwrap(),
				&[1.0, 0.25, 0.75, 0.0]
			),
			[1.0]
		);
		assert_eq!(
			evaluate(&l2_square_program(true).unwrap(), &[1.0, 0.0]),
			[0.0]
		);
		assert_eq!(
			evaluate(&l2_norm_program(0.000001, true).unwrap(), &[1.0, 9.0, 0.0]),
			[1.0]
		);
		assert_eq!(
			evaluate(&l2_square_program(true).unwrap(), &[3.0, 1.0]),
			[9.0]
		);
		assert_eq!(
			evaluate(&l2_norm_program(0.000001, true).unwrap(), &[6.0, 9.0, 1.0]),
			[2.0]
		);
	}
}
