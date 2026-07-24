use core::num::{NonZeroU64, NonZeroUsize};
use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, MetricId, ScalarOpcode, ValueId};
use recipe_ingest::DenseMatrix;
use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Elementwise, Gather, IndexBounds, IndexMap, PrimitiveAliasRule,
	PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce, ReduceOperator, ReduceResult,
	ScalarProgramBuilder, Shape, Tensor,
};
use recipe_ops::{
	BinaryClassificationMetricRequest, IdentityNamespace, MaterializationRequest, NamedTensor, PreparedParameter,
	PreparedParameters, RecallAtOutput, binary_metric_requirements, lower_scalar,
	materialize_binary_classification_metrics, materialize_composition, operation_registry,
};
use recipe_program::{IterationDomain, KernelIterationDomain, MetricEmission, StaticCalculationProgram};

use crate::{
	BinaryMetricOutputs, BinaryValidationConfig, BinaryValidationOutputs, CompiledTraining, DenseActivation,
	DenseBinaryDataset, DenseLayerState, DenseTrainingConfig, EarlyStoppingState, ExternalInputRole,
	OwnedExternalInput, ParameterState, RecallMetricOutput, TemperatureScalingConfig, TemperatureScalingState,
	TrainingBounds, TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult, TrainingMetricBinding,
	TrainingMetricKind, TrainingOutputs, ZScoreState,
};

const MATERIALIZATION_RESERVATION: u64 = 64;
const WORKSPACE_LIMIT: ByteCount = ByteCount::new(u64::MAX);

#[derive(Clone, Copy, Debug)]
struct LayerValues {
	input: ValueId,
	preactivation: ValueId,
	weight: InitialParameter,
	bias: InitialParameter,
	activation: DenseActivation,
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

#[derive(Clone, Copy, Debug)]
struct ZScoreValues {
	normalized: ValueId,
	mean: ValueId,
	variance: ValueId,
}

#[derive(Clone, Copy, Debug)]
struct ValidationValues {
	features: ValueId,
	targets: ValueId,
	rows: u64,
}

#[derive(Clone, Copy, Debug)]
struct InitialEarlyStoppingState {
	best_auprc: ValueId,
	stale_epochs: ValueId,
	stopped: ValueId,
}

/// Compile one fixed-width binary-classification dataset and dense model into
/// a bounded static Recipe program.
///
/// The final partial batch is represented by clamped gathers plus an exact GPU
/// validity mask. Padded lanes contribute zero loss and zero gradient, so no
/// source row is repeated and no host-side batch calculation is required.
pub fn compile_dense_binary_training(
	dataset: &DenseBinaryDataset,
	config: &DenseTrainingConfig,
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_binary_training_impl(dataset, config, None)
}

/// Compile the dense training program plus epoch-bound validation, a
/// GPU-resident AUPRC early-stop latch, and optional post-training temperature
/// scaling.
pub fn compile_dense_binary_training_with_validation(
	dataset: &DenseBinaryDataset,
	config: &DenseTrainingConfig,
	validation: &BinaryValidationConfig,
) -> TrainingCompileResult<CompiledTraining> {
	compile_dense_binary_training_impl(dataset, config, Some(validation))
}

fn compile_dense_binary_training_impl(
	dataset: &DenseBinaryDataset,
	config: &DenseTrainingConfig,
	validation_config: Option<&BinaryValidationConfig>,
) -> TrainingCompileResult<CompiledTraining> {
	validate_config(config)?;
	validate_validation_config(dataset, validation_config)?;
	let calibration_iterations = validation_config
		.and_then(BinaryValidationConfig::temperature_scaling)
		.map_or(0, |calibration| calibration.iterations.get());
	let bounds = training_bounds(
		dataset.train().rows(),
		config.batch_size,
		config.epochs,
		config.warmup_epochs,
		calibration_iterations,
	)?;
	let mut compiler = GraphCompiler::new(bounds.iterations, bounds.training_iterations)?;

	let train_features = compiler.external_matrix(ExternalInputRole::TrainFeatures, dataset.train().features())?;
	let train_targets = compiler.external_matrix(ExternalInputRole::TrainTargets, dataset.train().targets())?;
	let validation_inputs = dataset
		.validation()
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

	let feature_columns = u64::try_from(dataset.train().feature_columns()).map_err(|error| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("feature width cannot be represented by u64: {error}"),
		)
	})?;
	let train_rows = bounds.train_rows;
	let batch = bounds.batch_size;
	let full_feature_shape = shape(&[train_rows, feature_columns])?;
	let full_target_shape = shape(&[train_rows, 1])?;
	let batch_feature_shape = shape(&[batch, feature_columns])?;
	let batch_target_shape = shape(&[batch, 1])?;
	let scalar_shape = shape(&[1])?;

	let converted_features = compiler.convert_matrix_if_i32(
		train_features,
		full_feature_shape.clone(),
		IterationDomain::first(),
	)?;
	let training_z_score = compiler.z_score(
		converted_features,
		full_feature_shape,
		feature_columns,
		train_rows,
		config.normalization_epsilon,
		config.reduction_tree_lanes,
	)?;
	let converted_targets =
		compiler.convert_matrix_if_i32(train_targets, full_target_shape, IterationDomain::first())?;
	let validation_values = match validation_inputs {
		Some((features, targets, rows)) => {
			let feature_shape = shape(&[rows, feature_columns])?;
			let target_shape = shape(&[rows, 1])?;
			let features =
				compiler.convert_matrix_if_i32(features, feature_shape.clone(), IterationDomain::first())?;
			let features = compiler.apply_z_score(
				features,
				training_z_score.mean,
				training_z_score.variance,
				feature_shape,
				config.normalization_epsilon,
				IterationDomain::first(),
			)?;
			let targets = compiler.convert_matrix_if_i32(targets, target_shape, IterationDomain::first())?;
			Some(ValidationValues {
				features,
				targets,
				rows,
			})
		}
		None => None,
	};
	let normalized_features = training_z_score.normalized;

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
	let mask_index = compiler.tensor(DType::I32, batch_target_shape.clone())?;
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
	let validity = compiler.tensor(DType::F32, batch_target_shape.clone())?;
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
	let batch_targets = compiler.tensor(DType::F32, batch_target_shape.clone())?;
	compiler.emit(
		vec![converted_targets, index],
		vec![batch_targets],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Clamp,
		}),
		forbidden_aliases(2, 1),
		compiler.training_domain,
	)?;

	let mut layer_values = Vec::with_capacity(config.layers.len());
	let mut current = batch_features;
	let mut current_width = feature_columns;
	for (layer_index, layer) in config.layers.iter().copied().enumerate() {
		let output_width = layer.width().get();
		let parameter_shape = shape(&[current_width, output_width])?;
		let bias_shape = shape(&[output_width])?;
		let weight = compiler.initialize_weight(
			parameter_shape,
			current_width,
			config.random_seed,
			u64::try_from(layer_index).map_err(|error| {
				TrainingCompileError::new(
					TrainingCompileErrorKind::ArithmeticOverflow,
					format!("layer index cannot be represented by u64: {error}"),
				)
			})?,
		)?;
		let bias = compiler.initialize_zero_parameter(bias_shape)?;
		let preactivation = compiler.tensor(DType::F32, shape(&[batch, output_width])?)?;
		compiler.materialize(
			"gpu_linear_into",
			&[
				("input", current),
				("weight", weight.value),
				("bias", bias.value),
			],
			&[("output", preactivation)],
			"input",
			&PreparedParameters::new(),
			compiler.training_domain,
		)?;
		let output = match layer.activation() {
			DenseActivation::Linear => preactivation,
			DenseActivation::Silu => {
				let output = compiler.tensor(DType::F32, shape(&[batch, output_width])?)?;
				compiler.emit_owned_scalar(
					"gpu_silu_into",
					vec![preactivation],
					vec![output],
					compiler.training_domain,
				)?;
				output
			}
		};
		layer_values.push(LayerValues {
			input: current,
			preactivation,
			weight,
			bias,
			activation: layer.activation(),
		});
		current = output;
		current_width = output_width;
	}

	let losses = compiler.tensor(DType::F32, batch_target_shape.clone())?;
	let loss_gradient = compiler.tensor(DType::F32, batch_target_shape.clone())?;
	compiler.materialize(
		"gpu_bce_with_logits",
		&[("logits", current), ("targets", batch_targets)],
		&[("losses", losses), ("gradients", loss_gradient)],
		"logits",
		&PreparedParameters::new(),
		compiler.training_domain,
	)?;
	let normalized_gradient = compiler.tensor(DType::F32, batch_target_shape.clone())?;
	compiler.emit_elementwise(
		vec![loss_gradient, validity, valid_count],
		vec![normalized_gradient],
		masked_mean_program()?,
		compiler.training_domain,
	)?;
	let masked_losses = compiler.tensor(DType::F32, batch_target_shape)?;
	compiler.emit_elementwise(
		vec![losses, validity],
		vec![masked_losses],
		multiply_program()?,
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
		vec![loss_sum, valid_count],
		vec![batch_loss],
		divide_program()?,
		compiler.training_domain,
	)?;
	let gradients = compiler.backward(
		&layer_values,
		normalized_gradient,
		batch,
		config.reduction_tree_lanes,
	)?;
	let flattened_gradients = gradients
		.iter()
		.flat_map(|gradient| [gradient.weight, gradient.bias])
		.collect::<Vec<_>>();
	let clipped = compiler.global_clip(
		&flattened_gradients,
		config.gradient_clip_norm,
		config.reduction_tree_lanes,
	)?;

	let early_stopping_initial = validation_config
		.and_then(BinaryValidationConfig::early_stopping_patience)
		.map(|_| compiler.initialize_early_stopping_state())
		.transpose()?;
	let (learning_rate, beta_one_power, beta_two_power) = compiler.dynamic_adam_scalars(
		config,
		&bounds,
		early_stopping_initial.map(|state| state.stopped),
	)?;
	let mut layer_states = Vec::with_capacity(layer_values.len());
	for (layer_index, layer) in layer_values.iter().copied().enumerate() {
		let gradient_offset = layer_index.checked_mul(2).ok_or_else(|| {
			TrainingCompileError::new(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"gradient index overflowed usize",
			)
		})?;
		let weight = compiler.adamw_update(
			clipped[gradient_offset],
			layer.weight,
			learning_rate,
			beta_one_power,
			beta_two_power,
			config,
		)?;
		let bias = compiler.adamw_update(
			clipped[gradient_offset + 1],
			layer.bias,
			learning_rate,
			beta_one_power,
			beta_two_power,
			config,
		)?;
		layer_states.push(DenseLayerState { weight, bias });
	}
	let validation = match (validation_config, validation_values) {
		(Some(validation_config), Some(validation_values)) => Some(compiler.compile_validation(
			validation_values,
			config,
			validation_config,
			&layer_states,
			early_stopping_initial,
			&bounds,
		)?),
		(Some(_), None) => {
			return Err(TrainingCompileError::new(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				"validation was requested without validation values",
			));
		}
		(None, _) => None,
	};

	let batch_loss_domain = compiler.training_domain;
	let metric_bindings = training_metric_bindings(batch_loss, batch_loss_domain, validation.as_ref())?;
	compiler
		.external_outputs
		.extend([training_z_score.mean, training_z_score.variance]);
	compiler.finish(
		bounds,
		TrainingOutputs {
			batch_loss,
			batch_loss_domain,
			normalization: ZScoreState {
				mean: training_z_score.mean,
				variance: training_z_score.variance,
			},
			layers: layer_states,
			validation,
			metric_bindings,
		},
	)
}

fn validate_config(config: &DenseTrainingConfig) -> TrainingCompileResult<()> {
	if config.layers.is_empty() {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"dense network requires at least one layer",
		));
	}
	let output = config.layers.last().copied().expect("nonempty was checked");
	if output.width().get() != 1 || output.activation() != DenseActivation::Linear {
		return Err(TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidNetwork,
			"binary BCE-with-logits requires a final width-one Linear layer",
		));
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

fn validate_validation_config(
	dataset: &DenseBinaryDataset,
	config: Option<&BinaryValidationConfig>,
) -> TrainingCompileResult<()> {
	let Some(config) = config else {
		return Ok(());
	};
	let validation = dataset.validation().ok_or_else(|| {
		TrainingCompileError::new(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"validation metrics require a prepared validation partition",
		)
	})?;
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
	Ok(())
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
			"GPU-derived Adam step currently requires at most i32::MAX training iterations",
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

#[derive(Debug)]
struct GraphCompiler {
	tensors: BTreeMap<ValueId, Tensor>,
	nodes: Vec<CalculationNode>,
	domains: Vec<KernelIterationDomain>,
	next_value: u64,
	next_kernel: u64,
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
	) -> TrainingCompileResult<ValueId> {
		let normalized = self.tensor(DType::F32, output_shape)?;
		self.emit_elementwise(
			vec![input, mean, variance],
			vec![normalized],
			z_score_program(epsilon)?,
			domain,
		)?;
		Ok(normalized)
	}

	fn initialize_weight(
		&mut self,
		parameter_shape: Shape,
		fan_in: u64,
		seed: u64,
		stream: u64,
	) -> TrainingCompileResult<InitialParameter> {
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

	fn compile_validation(
		&mut self,
		validation: ValidationValues,
		training_config: &DenseTrainingConfig,
		validation_config: &BinaryValidationConfig,
		layers: &[DenseLayerState],
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
		let mut current = validation.features;
		for (layer, state) in training_config.layers.iter().copied().zip(layers) {
			let output_shape = shape(&[validation.rows, layer.width().get()])?;
			let preactivation = self.tensor(DType::F32, output_shape.clone())?;
			self.materialize(
				"gpu_linear_into",
				&[
					("input", current),
					("weight", state.weight.updated_parameter),
					("bias", state.bias.updated_parameter),
				],
				&[("output", preactivation)],
				"input",
				&PreparedParameters::new(),
				validation_domain,
			)?;
			current = match layer.activation() {
				DenseActivation::Linear => preactivation,
				DenseActivation::Silu => {
					let activated = self.tensor(DType::F32, output_shape)?;
					self.emit_owned_scalar(
						"gpu_silu_into",
						vec![preactivation],
						vec![activated],
						validation_domain,
					)?;
					activated
				}
			};
		}
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

	fn backward(
		&mut self,
		layers: &[LayerValues],
		mut gradient: ValueId,
		batch: u64,
		tree_lanes: u32,
	) -> TrainingCompileResult<Vec<GradientPair>> {
		let mut gradients = vec![None; layers.len()];
		for layer_index in (0..layers.len()).rev() {
			let layer = layers[layer_index];
			let parameter_gradient = match layer.activation {
				DenseActivation::Linear => gradient,
				DenseActivation::Silu => {
					let activation_gradient =
						self.tensor(DType::F32, self.tensor_ref(gradient)?.shape.clone())?;
					self.emit_owned_scalar(
						"gpu_silu_backward_into",
						vec![gradient, layer.preactivation],
						vec![activation_gradient],
						self.training_domain,
					)?;
					activation_gradient
				}
			};
			let weight_shape = self.tensor_ref(layer.weight.value)?.shape.clone();
			let bias_shape = self.tensor_ref(layer.bias.value)?.shape.clone();
			let weight_gradient = self.tensor(DType::F32, weight_shape)?;
			let bias_gradient = self.tensor(DType::F32, bias_shape)?;
			let prepared = [(
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(tree_lanes)),
			)]
			.into_iter()
			.collect::<PreparedParameters>();
			if layer_index == 0 {
				self.materialize(
					"gpu_linear_backward_weights_only_into",
					&[
						("output_gradient", parameter_gradient),
						("input", layer.input),
					],
					&[
						("weight_gradient", weight_gradient),
						("bias_gradient", bias_gradient),
					],
					"output_gradient",
					&prepared,
					self.training_domain,
				)?;
			} else {
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
						("output_gradient", parameter_gradient),
						("input", layer.input),
						("weight", layer.weight.value),
					],
					&[
						("input_gradient", input_gradient),
						("weight_gradient", weight_gradient),
						("bias_gradient", bias_gradient),
					],
					"output_gradient",
					&prepared,
					self.training_domain,
				)?;
				gradient = input_gradient;
			}
			gradients[layer_index] = Some(GradientPair {
				weight: weight_gradient,
				bias: bias_gradient,
			});
		}
		gradients
			.into_iter()
			.map(|gradient| {
				gradient.ok_or_else(|| {
					TrainingCompileError::new(
						TrainingCompileErrorKind::InvalidNetwork,
						"dense backward failed to produce one gradient pair per layer",
					)
				})
			})
			.collect()
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
	) -> TrainingCompileResult<(ValueId, ValueId, ValueId)> {
		let scalar_shape = shape(&[1])?;
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
		let step_f32 = self.tensor(DType::F32, scalar_shape.clone())?;
		let warmup = self.tensor(DType::F32, scalar_shape.clone())?;
		let cosine_angle = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_elementwise(
			vec![step_i32],
			vec![step_f32, warmup, cosine_angle],
			schedule_inputs_program(bounds.warmup_iterations, bounds.iterations.get())?,
			self.training_domain,
		)?;
		let cosine = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_owned_scalar(
			"gpu_cos",
			vec![cosine_angle],
			vec![cosine],
			self.training_domain,
		)?;
		let learning_rate = self.tensor(DType::F32, scalar_shape.clone())?;
		let mut learning_rate_inputs = vec![cosine, warmup];
		if let Some(stopped) = stopped {
			learning_rate_inputs.push(stopped);
		}
		self.emit_elementwise(
			learning_rate_inputs,
			vec![learning_rate],
			learning_rate_program(config.adamw.learning_rate, stopped.is_some())?,
			self.training_domain,
		)?;
		let beta_one = self.tensor(DType::F32, scalar_shape.clone())?;
		let beta_two = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_elementwise(
			vec![step_f32],
			vec![beta_one, beta_two],
			adam_beta_program(config.adamw.beta_one, config.adamw.beta_two)?,
			self.training_domain,
		)?;
		let beta_one_power = self.tensor(DType::F32, scalar_shape.clone())?;
		self.emit_owned_scalar(
			"gpu_pow",
			vec![beta_one, step_f32],
			vec![beta_one_power],
			self.training_domain,
		)?;
		let beta_two_power = self.tensor(DType::F32, scalar_shape)?;
		self.emit_owned_scalar(
			"gpu_pow",
			vec![beta_two, step_f32],
			vec![beta_two_power],
			self.training_domain,
		)?;
		Ok((learning_rate, beta_one_power, beta_two_power))
	}

	fn adamw_update(
		&mut self,
		gradient: ValueId,
		initial: InitialParameter,
		learning_rate: ValueId,
		beta_one_power: ValueId,
		beta_two_power: ValueId,
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
		let outputs = vec![
			updated_first_moment,
			updated_second_moment,
			updated_parameter,
		];
		let aliases = exact_adam_aliases();
		let update_kernel = self.emit(
			inputs,
			outputs,
			PrimitiveKind::Elementwise(Elementwise {
				program: adamw_program(
					config.adamw.beta_one,
					config.adamw.beta_two,
					config.adamw.epsilon,
					config.adamw.weight_decay,
				)?,
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
		self.emit(
			vec![input],
			vec![output],
			PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Sum,
				axes: AxisSet::new(axes.to_vec())?,
				keep_dimensions: false,
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

	fn finish(mut self, bounds: TrainingBounds, outputs: TrainingOutputs) -> TrainingCompileResult<CompiledTraining> {
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
) -> TrainingCompileResult<Vec<TrainingMetricBinding>> {
	let mut bindings = vec![TrainingMetricBinding {
		kind: TrainingMetricKind::BatchLoss,
		metric: MetricId::new(1),
		value: batch_loss,
		domain: batch_loss_domain,
	}];
	let Some(validation) = validation else {
		return Ok(bindings);
	};
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
		let ordinal = u64::try_from(bindings.len())
			.ok()
			.and_then(|index| index.checked_add(1))
			.ok_or_else(identity_exhausted)?;
		bindings.push(TrainingMetricBinding {
			kind,
			metric: MetricId::new(ordinal),
			value,
			domain: validation.metric_domain,
		});
	}
	for recall in &validation.metrics.recall_at {
		let ordinal = u64::try_from(bindings.len())
			.ok()
			.and_then(|index| index.checked_add(1))
			.ok_or_else(identity_exhausted)?;
		bindings.push(TrainingMetricBinding {
			kind: TrainingMetricKind::RecallAt {
				threshold_bits: recall.threshold_bits,
			},
			metric: MetricId::new(ordinal),
			value: recall.value,
			domain: validation.metric_domain,
		});
	}
	Ok(bindings)
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

fn exact_adam_aliases() -> Vec<PrimitiveAliasRule> {
	(0..7).flat_map(|input| {
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

fn multiply_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Multiply, left, right)?;
	Ok(builder.finish(&[result])?)
}

fn divide_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let numerator = builder.input(DType::F32)?;
	let denominator = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Divide, numerator, denominator)?;
	Ok(builder.finish(&[result])?)
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

fn z_score_program(epsilon: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let mean = builder.input(DType::F32)?;
	let variance = builder.input(DType::F32)?;
	let epsilon = builder.f32(epsilon)?;
	let variance = builder.binary(ScalarOpcode::Maximum, variance, epsilon)?;
	let standard_deviation = builder.unary(ScalarOpcode::SquareRoot, variance)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, mean)?;
	let normalized = builder.binary(ScalarOpcode::Divide, centered, standard_deviation)?;
	Ok(builder.finish(&[normalized])?)
}

fn masked_mean_program() -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let valid = builder.input(DType::F32)?;
	let valid_count = builder.input(DType::F32)?;
	let masked = builder.binary(ScalarOpcode::Multiply, value, valid)?;
	let normalized = builder.binary(ScalarOpcode::Divide, masked, valid_count)?;
	Ok(builder.finish(&[normalized])?)
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
	let mut builder = ScalarProgramBuilder::new()?;
	let step = builder.input(DType::I32)?;
	let step = builder.unary(ScalarOpcode::ConvertI32ToF32, step)?;
	let one = builder.f32(1.0)?;
	let zero = builder.f32(0.0)?;
	let warmup = if warmup_iterations == 0 {
		one
	} else {
		let denominator = builder.f32(warmup_iterations as f32)?;
		let fraction = builder.binary(ScalarOpcode::Divide, step, denominator)?;
		builder.binary(ScalarOpcode::Minimum, one, fraction)?
	};
	let warmup_start = builder.f32(warmup_iterations as f32)?;
	let elapsed = builder.binary(ScalarOpcode::Subtract, step, warmup_start)?;
	let elapsed = builder.binary(ScalarOpcode::Maximum, elapsed, zero)?;
	let decay_iterations = total_iterations.saturating_sub(warmup_iterations).max(1);
	let decay_denominator = builder.f32(decay_iterations as f32)?;
	let progress = builder.binary(ScalarOpcode::Divide, elapsed, decay_denominator)?;
	let progress = builder.binary(ScalarOpcode::Minimum, one, progress)?;
	let pi = builder.f32(core::f32::consts::PI)?;
	let angle = builder.binary(ScalarOpcode::Multiply, progress, pi)?;
	Ok(builder.finish(&[step, warmup, angle])?)
}

fn learning_rate_program(
	base_learning_rate: f32,
	gated_by_early_stopping: bool,
) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let cosine = builder.input(DType::F32)?;
	let warmup = builder.input(DType::F32)?;
	let stopped = gated_by_early_stopping
		.then(|| builder.input(DType::I32))
		.transpose()?;
	let one = builder.f32(1.0)?;
	let shifted = builder.binary(ScalarOpcode::Add, one, cosine)?;
	let half = builder.f32(0.5)?;
	let cosine_factor = builder.binary(ScalarOpcode::Multiply, half, shifted)?;
	let factor = builder.binary(ScalarOpcode::Multiply, warmup, cosine_factor)?;
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

fn adam_beta_program(beta_one: f32, beta_two: f32) -> TrainingCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let _step = builder.input(DType::F32)?;
	let beta_one = builder.f32(beta_one)?;
	let beta_two = builder.f32(beta_two)?;
	Ok(builder.finish(&[beta_one, beta_two])?)
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
