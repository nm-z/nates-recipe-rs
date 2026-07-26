use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};
use std::collections::BTreeSet;
use std::error::Error;

use recipe_core::{AliasPermission, DType, MetricId, ScalarOpcode, ValueId};
use recipe_ingest::{
	CategoricalEncodingModel, Delimiter, HeaderMode, IngestLimits, PreparationRequest, PreparedDataset, SemanticType,
	TableRequest, TrainFraction, VectorEncoding, VectorMetadata, VectorRole, parse_table, prepare_table,
};
use recipe_language::{
	CalculationNode, IndexMap, PrimitiveKind, RandomDistribution, ReduceOperator, ReduceResult, ScatterConflict,
	Tensor,
};
use recipe_program::{IterationDomain, StaticCalculationProgram};
use recipe_training::{
	AdamWConfig, BinaryValidationConfig, CheckpointManifest, CompiledTraining, DecodedMulticlassClass,
	DenseActivation, DenseBlock, DenseBlockState, DenseDataNormalization, DenseFeatureLowering, DenseLayer,
	DenseLayerState, DenseLoss, DenseNormalization, DenseOperation, DensePool, DensePoolGroupOrder,
	DensePoolWinnerContract, DenseResidual, DenseResidualOperation, DenseTask, DenseTrainingConfig,
	ExternalInputRole, LearningRateDecay, MulticlassValidationConfig, TemperatureScalingConfig,
	TrainingCompileErrorKind, TrainingMetricKind, ValidationMetricFamily, ValidationMetricStatus,
	ValidationUnavailableReason, compile_dense_training, compile_dense_training_with_binary_validation,
	compile_dense_training_with_blocks, compile_dense_training_with_blocks_and_binary_validation,
	compile_dense_training_with_multiclass_validation,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn prepare_csv(source: &[u8]) -> TestResult<PreparedDataset> {
	let table = parse_table(
		source,
		TableRequest::new(
			Delimiter::Comma,
			HeaderMode::Present,
			IngestLimits::new(1_048_576, 64, 8, 1_024)?,
		),
	)?;
	let fraction = TrainFraction::new(5, 7)?;
	Ok(prepare_table(
		&table,
		&PreparationRequest::new(["target"], fraction),
		&CategoricalEncodingModel,
	)?)
}

fn prepared_dataset(targets: &[i32]) -> TestResult<PreparedDataset> {
	let mut source = b"feature_one,feature_two,target\n".to_vec();
	for (index, target) in targets.iter().enumerate() {
		let feature = index + 1;
		source.extend_from_slice(format!("{feature},{},{target}\n", feature * 10).as_bytes());
	}
	prepare_csv(&source)
}

fn categorical_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,no\n\
		2,20,yes\n\
		3,30,no\n\
		4,40,yes\n\
		5,50,yes\n\
		6,60,no\n\
		7,70,yes\n",
	)
}

fn multiclass_dataset_with_validation_only_label() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,red\n\
		2,20,blue\n\
		3,30,green\n\
		4,40,red\n\
		5,50,blue\n\
		6,60,yellow\n\
		7,70,green\n",
	)
}

fn binary_dataset() -> TestResult<PreparedDataset> {
	prepared_dataset(&[0, 1, 0, 1, 1, 0, 1])
}

fn four_feature_binary_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,feature_three,feature_four,target\n\
		1,10,100,1000,0\n\
		2,20,200,2000,1\n\
		3,30,300,3000,0\n\
		4,40,400,4000,1\n\
		5,50,500,5000,1\n\
		6,60,600,6000,0\n\
		7,70,700,7000,1\n",
	)
}

fn partially_missing_binary_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,0\n\
		2,20,1\n\
		3,30,\n\
		4,40,\n\
		5,50,1\n\
		6,60,0\n\
		7,70,1\n",
	)
}

fn unknown_validation_target_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,0\n\
		2,20,1\n\
		3,30,0\n\
		4,40,1\n\
		5,50,1\n\
		6,60,\n\
		7,70,\n",
	)
}

fn entirely_unsupervised_training_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,\n\
		2,20,\n\
		3,30,\n\
		4,40,\n\
		5,50,\n\
		6,60,0\n\
		7,70,1\n",
	)
}

fn entirely_unsupervised_categorical_training_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,\n\
		2,20,\n\
		3,30,\n\
		4,40,\n\
		5,50,\n\
		6,60,red\n\
		7,70,blue\n",
	)
}

fn one_known_categorical_training_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,known\n\
		2,20,\n\
		3,30,\n\
		4,40,\n\
		5,50,\n\
		6,60,known\n\
		7,70,unseen\n",
	)
}

fn binary_single_class_validation_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,no\n\
		2,20,yes\n\
		3,30,no\n\
		4,40,yes\n\
		5,50,no\n\
		6,60,no\n\
		7,70,no\n",
	)
}

fn missing_multiclass_target_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,red\n\
		2,20,blue\n\
		3,30,\n\
		4,40,green\n\
		5,50,\n\
		6,60,yellow\n\
		7,70,red\n",
	)
}

fn missing_f32_regression_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"feature_one,feature_two,target\n\
		1,10,1.25\n\
		2,20,2.5\n\
		3,30,\n\
		4,40,4.75\n\
		5,50,\n\
		6,60,6.5\n\
		7,70,7.25\n",
	)
}

fn regression_dataset() -> TestResult<PreparedDataset> {
	prepared_dataset(&[
		208_500, 181_500, 223_500, 140_000, 250_000, 307_000, 129_900,
	])
}

fn categorical_feature_dataset() -> TestResult<PreparedDataset> {
	prepare_csv(
		b"amount,color,target\n\
		10,red,0\n\
		20,blue,1\n\
		30,green,0\n\
		40,red,1\n\
		50,blue,1\n\
		60,green,0\n\
		70,red,1\n",
	)
}

fn config() -> DenseTrainingConfig {
	DenseTrainingConfig {
		layers: vec![
			DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Silu),
			DenseLayer::new(NonZeroU64::new(1).unwrap(), DenseActivation::Linear),
		],
		loss: DenseLoss::BinaryCrossEntropy,
		data_normalization: DenseDataNormalization::ZScore,
		batch_size: NonZeroUsize::new(3).unwrap(),
		epochs: NonZeroU64::new(2).unwrap(),
		warmup_epochs: 1,
		learning_rate_decay: LearningRateDecay::Cosine,
		gradient_clip_norm: 1.0,
		normalization_epsilon: 1.0e-6,
		reduction_tree_lanes: 32,
		random_seed: 17,
		adamw: AdamWConfig::default(),
	}
}

fn training_schedule_program(compiled: &CompiledTraining) -> &recipe_core::ScalarProgram {
	let step = compiled
		.graph()
		.nodes
		.iter()
		.find_map(|node| match node.kernel.kind {
			PrimitiveKind::IndexMap(IndexMap {
				start: 1,
				element_step: 0,
				iteration_step: 1,
				modulus: None,
			}) => node.kernel.outputs.first().copied(),
			_ => None,
		})
		.expect("training step map exists");
	compiled
		.graph()
		.nodes
		.iter()
		.find_map(|node| (node.kernel.inputs == [step]).then_some(&node.kernel.kind))
		.and_then(|kind| match kind {
			PrimitiveKind::Elementwise(elementwise) => Some(&elementwise.program),
			_ => None,
		})
		.expect("training schedule program exists")
}

#[test]
fn compiles_every_data_normalizer_and_both_layer_operation_orders() -> TestResult {
	let validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), []);
	for data_normalization in [
		DenseDataNormalization::ZScore,
		DenseDataNormalization::MinMax,
		DenseDataNormalization::L2Norm,
	] {
		for operations in [
			vec![
				DenseOperation::Normalization(DenseNormalization::Layer),
				DenseOperation::Activation(DenseActivation::Silu),
			],
			vec![
				DenseOperation::Activation(DenseActivation::Silu),
				DenseOperation::Normalization(DenseNormalization::Layer),
			],
			vec![
				DenseOperation::Normalization(DenseNormalization::Batch),
				DenseOperation::Activation(DenseActivation::Silu),
			],
			vec![
				DenseOperation::Activation(DenseActivation::Silu),
				DenseOperation::Normalization(DenseNormalization::Batch),
			],
		] {
			let mut config = config();
			config.data_normalization = data_normalization;
			config.layers[0] = DenseLayer::with_operations(NonZeroU64::new(4).unwrap(), operations);
			let compiled =
				compile_dense_training_with_binary_validation(&binary_dataset()?, &config, &validation)?;
			compiled.graph().validate()?;
			compiled.program().validate()?;
		}
	}
	Ok(())
}

#[test]
fn compiles_every_pointwise_loss_and_its_backward_seed() -> TestResult {
	for loss in [
		DenseLoss::BinaryCrossEntropy,
		DenseLoss::MeanSquaredError,
		DenseLoss::MeanAbsoluteError,
		DenseLoss::Huber,
	] {
		let mut config = config();
		config.loss = loss;
		let dataset = match loss {
			DenseLoss::BinaryCrossEntropy => binary_dataset()?,
			DenseLoss::MeanSquaredError | DenseLoss::MeanAbsoluteError | DenseLoss::Huber => {
				regression_dataset()?
			}
			DenseLoss::CrossEntropy => unreachable!(),
		};
		let compiled = compile_dense_training(&dataset, &config)?;
		compiled.graph().validate()?;
		compiled.program().validate()?;
		assert_eq!(
			compiled.dataset_schema().task(),
			match loss {
				DenseLoss::BinaryCrossEntropy => DenseTask::BinaryClassification {
					target_vector: 2,
					positive_code: 1,
				},
				_ => DenseTask::ScalarRegression { target_vector: 2 },
			}
		);
	}
	Ok(())
}

#[test]
fn compiled_schema_retains_numeric_regression_semantics_and_spans() -> TestResult {
	let mut config = config();
	config.loss = DenseLoss::MeanSquaredError;
	let compiled = compile_dense_training(&regression_dataset()?, &config)?;
	let schema = compiled.dataset_schema();
	assert_eq!(schema.input_width(), 2);
	assert_eq!(schema.target(), 2);
	assert_eq!(
		schema.task(),
		DenseTask::ScalarRegression { target_vector: 2 }
	);
	assert_eq!(schema.features().len(), 2);
	assert_eq!(schema.features()[0].source_vector(), 0);
	assert_eq!(schema.features()[0].start(), 0);
	assert_eq!(schema.features()[0].width(), 1);
	let target = schema
		.vectors()
		.iter()
		.find(|vector| vector.role() == VectorRole::Target)
		.expect("compiled target schema exists");
	assert_eq!(target.name(), b"target");
	assert_eq!(target.semantic_type(), SemanticType::Numeric);
	assert_eq!(target.encoding(), VectorEncoding::I32);
	assert_eq!(target.metadata(), &VectorMetadata::None);
	let checkpoint = CheckpointManifest::from_compiled(&compiled)?;
	assert_eq!(checkpoint.feature_width(), schema.input_width());
	assert_eq!(checkpoint.vectors().len(), schema.vectors().len());
	assert_eq!(
		checkpoint.vectors()[2].semantic_type(),
		SemanticType::Numeric
	);
	assert_eq!(checkpoint.vectors()[2].encoding(), VectorEncoding::I32);
	Ok(())
}

#[test]
fn categorical_features_lower_in_dictionary_order_with_exact_spans() -> TestResult {
	let compiled = compile_dense_training(&categorical_feature_dataset()?, &config())?;
	let schema = compiled.dataset_schema();
	assert_eq!(schema.input_width(), 5);
	assert_eq!(schema.features().len(), 2);
	assert_eq!(schema.features()[0].source_vector(), 0);
	assert_eq!(schema.features()[0].start(), 0);
	assert_eq!(schema.features()[0].width(), 1);
	assert_eq!(
		schema.features()[0].lowering(),
		DenseFeatureLowering::NumericScalar
	);
	assert_eq!(schema.features()[1].source_vector(), 1);
	assert_eq!(schema.features()[1].start(), 1);
	assert_eq!(schema.features()[1].width(), 4);
	assert_eq!(
		schema.features()[1].lowering(),
		DenseFeatureLowering::CategoricalOneHot {
			dictionary_width: 3,
			reserved_index: 3,
		}
	);
	let color = schema
		.vectors()
		.iter()
		.find(|vector| vector.name() == b"color")
		.expect("color schema exists");
	assert_eq!(
		color.metadata(),
		&VectorMetadata::Categorical {
			dictionary: vec![b"blue".to_vec(), b"green".to_vec(), b"red".to_vec()],
		}
	);

	let train_features = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainFeatures)
		.expect("lowered train features exist");
	assert_eq!(train_features.dtype(), DType::F32);
	assert_eq!(train_features.shape().extents(), [5, 5]);
	let values = train_features
		.bytes()
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one f32")))
		.collect::<Vec<_>>();
	assert_eq!(
		values,
		[
			10.0, 0.0, 0.0, 1.0, 0.0, 20.0, 1.0, 0.0, 0.0, 0.0, 30.0, 0.0, 1.0, 0.0, 0.0, 40.0, 0.0, 0.0, 1.0,
			0.0, 50.0, 1.0, 0.0, 0.0, 0.0,
		]
	);
	let checkpoint = CheckpointManifest::from_compiled(&compiled)?;
	assert_eq!(checkpoint.feature_spans(), schema.features());
	assert_eq!(
		checkpoint.feature_spans()[1].lowering(),
		DenseFeatureLowering::CategoricalOneHot {
			dictionary_width: 3,
			reserved_index: 3,
		}
	);
	Ok(())
}

#[test]
fn data_normalizers_apply_only_to_numeric_feature_spans() -> TestResult {
	for normalization in [
		DenseDataNormalization::ZScore,
		DenseDataNormalization::MinMax,
		DenseDataNormalization::L2Norm,
	] {
		let mut config = config();
		config.data_normalization = normalization;
		let compiled = compile_dense_training(&categorical_feature_dataset()?, &config)?;
		compiled.graph().validate()?;
		compiled.program().validate()?;
		let mask = compiled
			.external_inputs()
			.iter()
			.find(|input| input.role() == ExternalInputRole::FeatureNormalizationMask)
			.expect("mixed semantic features require a normalization mask");
		assert_eq!(mask.dtype(), DType::F32);
		assert_eq!(mask.shape().extents(), [5]);
		let values = mask
			.bytes()
			.chunks_exact(4)
			.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one f32")))
			.collect::<Vec<_>>();
		assert_eq!(values, [1.0, 0.0, 0.0, 0.0, 0.0]);
	}
	Ok(())
}

#[test]
fn categorical_feature_lowering_routes_missing_values_and_rejects_unsupported_semantics() -> TestResult {
	let missing = prepare_csv(
		b"amount,color,target\n\
		10,red,0\n\
		20,,1\n\
		30,green,0\n\
		40,red,1\n\
		50,blue,1\n\
		60,green,0\n\
		70,red,1\n",
	)?;
	let compiled = compile_dense_training(&missing, &config())?;
	let train_features = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainFeatures)
		.expect("lowered train features exist");
	let values = train_features
		.bytes()
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one f32")))
		.collect::<Vec<_>>();
	assert_eq!(&values[5..10], [20.0, 0.0, 0.0, 0.0, 1.0]);

	let ordinal = prepare_csv(
		b"amount,priority,target\n\
		10,low,0\n\
		20,medium,1\n\
		30,high,0\n\
		40,low,1\n\
		50,medium,1\n\
		60,high,0\n\
		70,low,1\n",
	)?;
	let error = compile_dense_training(&ordinal, &config()).unwrap_err();
	assert_eq!(error.kind, TrainingCompileErrorKind::InvalidFeatureMatrix);
	assert!(error.detail.contains("Ordinal/OrdinalI32"));
	Ok(())
}

#[test]
fn bce_accepts_two_label_categorical_targets_and_records_the_dictionary() -> TestResult {
	let compiled = compile_dense_training(&categorical_dataset()?, &config())?;
	assert_eq!(
		compiled.dataset_schema().task(),
		DenseTask::BinaryClassification {
			target_vector: 2,
			positive_code: 1,
		}
	);
	let target = compiled
		.dataset_schema()
		.vectors()
		.iter()
		.find(|vector| vector.role() == VectorRole::Target)
		.expect("compiled target schema exists");
	assert_eq!(target.semantic_type(), SemanticType::Categorical);
	assert_eq!(target.encoding(), VectorEncoding::DictionaryI32);
	assert_eq!(
		target.metadata(),
		&VectorMetadata::Categorical {
			dictionary: vec![b"no".to_vec(), b"yes".to_vec()],
		}
	);
	Ok(())
}

#[test]
fn multiclass_cross_entropy_uses_training_dictionary_reserved_route_and_output_adapter() -> TestResult {
	let dataset = multiclass_dataset_with_validation_only_label()?;
	let mut config = config();
	config.loss = DenseLoss::CrossEntropy;
	config.layers[1] = DenseLayer::new(NonZeroU64::new(2).unwrap(), DenseActivation::Silu);
	let compiled = compile_dense_training(&dataset, &config)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	assert_eq!(
		compiled.dataset_schema().task(),
		DenseTask::MulticlassClassification {
			target_vector: 2,
			class_count: 4,
			reserved_code: 3,
		}
	);
	assert_eq!(compiled.dataset_schema().output_width(), 4);
	assert_eq!(
		compiled.dataset_schema().decode_multiclass_class(0),
		Some(DecodedMulticlassClass::Label(b"blue"))
	);
	assert_eq!(
		compiled.dataset_schema().decode_multiclass_class(3),
		Some(DecodedMulticlassClass::ReservedUnseen)
	);
	assert_eq!(compiled.dataset_schema().decode_multiclass_class(4), None);
	let target = compiled
		.dataset_schema()
		.vectors()
		.iter()
		.find(|vector| vector.role() == VectorRole::Target)
		.expect("compiled target schema exists");
	assert_eq!(
		target.metadata(),
		&VectorMetadata::Categorical {
			dictionary: vec![b"blue".to_vec(), b"green".to_vec(), b"red".to_vec()],
		}
	);
	let adapter = compiled
		.output_adapter()
		.expect("mismatched nonlinear output is adapted");
	assert_eq!(adapter.source_width().get(), 2);
	assert_eq!(adapter.target_width().get(), 4);
	assert_eq!(compiled.layers().len(), config.layers.len() + 1);
	assert_eq!(
		compiled
			.layers()
			.last()
			.expect("adapter layer")
			.width()
			.get(),
		4
	);
	assert!(
		compiled
			.layers()
			.last()
			.expect("adapter layer")
			.operations()
			.is_empty()
	);

	let train_targets = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainTargets)
		.expect("classified training target codes exist");
	assert_eq!(train_targets.dtype(), DType::I32);
	assert_eq!(train_targets.shape().extents(), [5, 1]);
	let train_values = train_targets
		.bytes()
		.chunks_exact(4)
		.map(|bytes| i32::from_le_bytes(bytes.try_into().expect("one i32")))
		.collect::<Vec<_>>();
	assert_eq!(train_values, [2, 0, 1, 2, 0]);
	let validation_targets = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::ValidationTargets)
		.expect("classified validation target codes exist");
	assert_eq!(validation_targets.dtype(), DType::I32);
	assert_eq!(validation_targets.shape().extents(), [1, 1]);
	let validation_values = validation_targets
		.bytes()
		.chunks_exact(4)
		.map(|bytes| i32::from_le_bytes(bytes.try_into().expect("one i32")))
		.collect::<Vec<_>>();
	assert_eq!(validation_values, [1]);

	let cross_entropy = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			matches!(
				&node.kernel.kind,
				PrimitiveKind::Elementwise(elementwise)
					if elementwise.program.inputs.iter().map(|input| input.dtype).collect::<Vec<_>>()
						== [DType::F32, DType::I32, DType::I32, DType::F32, DType::F32, DType::F32, DType::F32]
						&& node.kernel.outputs.len() == 2
			)
		})
		.expect("device categorical cross-entropy node exists");
	for output in &cross_entropy.kernel.outputs {
		let tensor = compiled
			.graph()
			.tensors
			.iter()
			.find(|tensor| tensor.id == *output)
			.expect("cross-entropy output tensor exists");
		assert_eq!(tensor.dtype, DType::F32);
		assert_eq!(tensor.shape.extents(), [3, 4]);
	}

	let manifest = CheckpointManifest::from_compiled(&compiled)?;
	assert_eq!(manifest.task(), compiled.dataset_schema().task());
	assert_eq!(manifest.output_adapter(), Some(adapter));
	assert_eq!(manifest.vectors()[2].metadata(), target.metadata());
	Ok(())
}

#[test]
fn multiclass_validation_materializes_device_cross_entropy_and_top_one_accuracy() -> TestResult {
	let mut config = config();
	config.loss = DenseLoss::CrossEntropy;
	config.layers[1] = DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Linear);
	let compiled = compile_dense_training_with_multiclass_validation(
		&multiclass_dataset_with_validation_only_label()?,
		&config,
		&MulticlassValidationConfig,
	)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	assert_eq!(compiled.outputs().validation, None);
	let validation = compiled
		.outputs()
		.multiclass_validation
		.expect("multiclass validation outputs exist");
	assert_eq!(
		validation.metric_domain,
		IterationDomain::new(
			compiled.bounds().batches_per_epoch - 1,
			compiled.bounds().training_iterations.get(),
			compiled.bounds().batches_per_epoch,
		)
		.expect("epoch-bound validation domain")
	);
	assert_eq!(
		compiled
			.outputs()
			.metric_bindings
			.iter()
			.map(|binding| binding.kind)
			.collect::<Vec<_>>(),
		[
			TrainingMetricKind::BatchLoss,
			TrainingMetricKind::ValidationMeanCrossEntropy,
			TrainingMetricKind::Accuracy,
		]
	);
	for value in [
		validation.metrics.mean_cross_entropy,
		validation.metrics.accuracy,
	] {
		let tensor = compiled
			.graph()
			.tensors
			.iter()
			.find(|tensor| tensor.id == value)
			.expect("multiclass metric tensor exists");
		assert_eq!(tensor.dtype, DType::F32);
		assert_eq!(tensor.shape.extents(), [1]);
	}
	let validation_targets = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::ValidationTargets)
		.expect("validation target codes exist");
	assert_eq!(validation_targets.dtype(), DType::I32);
	assert_eq!(validation_targets.shape().extents(), [1, 1]);
	Ok(())
}

#[test]
fn defined_output_projection_adapts_unusual_widths_instead_of_rejecting_training() -> TestResult {
	for (loss, dataset) in [
		(DenseLoss::BinaryCrossEntropy, binary_dataset()?),
		(DenseLoss::MeanSquaredError, regression_dataset()?),
		(
			DenseLoss::CrossEntropy,
			multiclass_dataset_with_validation_only_label()?,
		),
	] {
		let mut config = config();
		config.loss = loss;
		config.layers[1] = DenseLayer::new(NonZeroU64::new(5).unwrap(), DenseActivation::Silu);
		let compiled = compile_dense_training(&dataset, &config)?;
		let adapter = compiled
			.output_adapter()
			.expect("unusual width has a defined projection");
		assert_eq!(adapter.source_width().get(), 5);
		assert_eq!(
			adapter.target_width().get(),
			compiled.dataset_schema().output_width() as u64
		);
		assert_eq!(compiled.layers().len(), config.layers.len() + 1);
		compiled.graph().validate()?;
		compiled.program().validate()?;
	}

	let mut matched = config();
	matched.loss = DenseLoss::CrossEntropy;
	matched.layers[1] = DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Linear);
	let compiled = compile_dense_training(&multiclass_dataset_with_validation_only_label()?, &matched)?;
	assert_eq!(compiled.output_adapter(), None);
	assert_eq!(compiled.layers(), matched.layers);
	Ok(())
}

#[test]
fn loss_contract_rejects_semantically_incompatible_targets() -> TestResult {
	let binary_error = compile_dense_training(&prepared_dataset(&[0, 1, 2, 0, 1, 0, 1])?, &config()).unwrap_err();
	assert_eq!(
		binary_error.kind,
		TrainingCompileErrorKind::InvalidTargetMatrix
	);
	assert!(
		binary_error
			.detail
			.contains("binary target matrix contains 2")
	);

	let mut regression = config();
	regression.loss = DenseLoss::Huber;
	let regression_error = compile_dense_training(&categorical_dataset()?, &regression).unwrap_err();
	assert_eq!(
		regression_error.kind,
		TrainingCompileErrorKind::InvalidTargetMatrix
	);
	assert!(
		regression_error
			.detail
			.contains("Categorical/DictionaryI32")
	);

	let mut cross_entropy = config();
	cross_entropy.loss = DenseLoss::CrossEntropy;
	let cross_entropy_error = compile_dense_training(&regression_dataset()?, &cross_entropy).unwrap_err();
	assert_eq!(
		cross_entropy_error.kind,
		TrainingCompileErrorKind::InvalidTargetMatrix
	);
	assert!(
		cross_entropy_error
			.detail
			.contains("incompatible with categorical cross entropy")
	);

	let validation_error = compile_dense_training_with_multiclass_validation(
		&binary_dataset()?,
		&config(),
		&MulticlassValidationConfig,
	)
	.unwrap_err();
	assert_eq!(
		validation_error.kind,
		TrainingCompileErrorKind::InvalidNetwork
	);
	assert!(
		validation_error
			.detail
			.contains("multiclass validation requires a categorical cross-entropy target")
	);
	Ok(())
}

#[test]
fn missing_targets_use_safe_placeholders_and_gate_the_entire_optimizer_transition() -> TestResult {
	let mut training = config();
	training.batch_size = NonZeroUsize::new(2).unwrap();
	training.layers[0] = DenseLayer::with_operations(
		NonZeroU64::new(4).unwrap(),
		[DenseOperation::Normalization(DenseNormalization::Batch)],
	);
	let compiled = compile_dense_training(&partially_missing_binary_dataset()?, &training)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;

	let targets = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainTargets)
		.expect("training targets exist");
	let target_values = targets
		.bytes()
		.chunks_exact(4)
		.map(|bytes| i32::from_le_bytes(bytes.try_into().expect("one i32")))
		.collect::<Vec<_>>();
	assert_eq!(target_values, [0, 1, 0, 0, 1]);

	let target_supervision = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainTargetSupervision)
		.expect("target supervision exists");
	let supervision_values = target_supervision
		.bytes()
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one f32")))
		.collect::<Vec<_>>();
	assert_eq!(supervision_values, [1.0, 1.0, 0.0, 0.0, 1.0]);

	let progress = compiled
		.outputs()
		.optimizer_progress
		.expect("masked targets declare accepted-update progress");
	assert_eq!(progress.accepted_updates_per_epoch, 2);
	assert_eq!(progress.maximum_accepted_updates, 4);
	assert_eq!(progress.warmup_accepted_updates, 2);
	let accepted = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == progress.update_kernel)
		.expect("accepted-update recurrence exists");
	assert_eq!(
		accepted.kernel.inputs,
		[progress.initial_accepted_updates, progress.apply_update]
	);
	assert_eq!(accepted.kernel.outputs, [progress.updated_accepted_updates]);
	assert_eq!(accepted.kernel.alias_rules.len(), 2);
	assert_eq!(
		accepted.kernel.alias_rules[0].permission,
		AliasPermission::MustAliasExact
	);
	assert_eq!(
		accepted.kernel.alias_rules[1].permission,
		AliasPermission::Forbidden
	);

	let beta = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == progress.beta_update_kernel)
		.expect("gated beta-power recurrence exists");
	assert_eq!(
		beta.kernel.inputs,
		[
			progress.initial_beta_one_power,
			progress.initial_beta_two_power,
			progress.apply_update,
		]
	);
	assert_eq!(beta.kernel.alias_rules.len(), 6);
	assert_eq!(
		beta.kernel.alias_rules[0].permission,
		AliasPermission::MustAliasExact
	);
	assert_eq!(
		beta.kernel.alias_rules[3].permission,
		AliasPermission::MustAliasExact
	);

	let adam = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == compiled.outputs().layers[0].weight.update_kernel)
		.expect("AdamW transition exists");
	assert_eq!(adam.kernel.inputs.len(), 8);
	assert_eq!(adam.kernel.inputs.last(), Some(&progress.apply_update));
	assert_eq!(adam.kernel.alias_rules.len(), 24);
	assert_eq!(
		adam.kernel.alias_rules[5].permission,
		AliasPermission::MustAliasExact
	);
	assert_eq!(
		adam.kernel.alias_rules[6].permission,
		AliasPermission::MustAliasExact
	);
	assert_eq!(
		adam.kernel.alias_rules[10].permission,
		AliasPermission::MustAliasExact
	);

	let known_count = producer(&compiled, progress.apply_update).kernel.inputs[0];
	let supervision = producer(&compiled, known_count).kernel.inputs[0];
	let batch_normalization_masks = compiled
		.graph()
		.nodes
		.iter()
		.filter(|node| {
			node.kernel.inputs.get(1) == Some(&supervision)
				&& node
					.kernel
					.outputs
					.iter()
					.any(|output| graph_tensor(&compiled, *output).shape.extents() == [2, 4])
		})
		.collect::<Vec<_>>();
	assert!(
		batch_normalization_masks.len() >= 4,
		"BatchNorm did not use target supervision"
	);
	assert!(batch_normalization_masks.iter().all(|node| {
		let PrimitiveKind::Elementwise(elementwise) = &node.kernel.kind else {
			return false;
		};
		elementwise
			.program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Select)
			&& elementwise
				.program
				.instructions
				.iter()
				.all(|instruction| instruction.opcode != ScalarOpcode::Multiply)
	}));
	let _manifest = CheckpointManifest::from_compiled(&compiled)?;
	Ok(())
}

#[test]
fn all_known_targets_keep_the_legacy_optimizer_graph() -> TestResult {
	let compiled = compile_dense_training(&binary_dataset()?, &config())?;
	assert_eq!(compiled.outputs().optimizer_progress, None);
	assert!(
		compiled
			.external_inputs()
			.iter()
			.all(|input| input.role() != ExternalInputRole::TrainTargetSupervision)
	);
	let adam = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == compiled.outputs().layers[0].weight.update_kernel)
		.expect("legacy AdamW transition exists");
	assert_eq!(adam.kernel.inputs.len(), 7);
	assert_eq!(adam.kernel.alias_rules.len(), 21);
	assert!(compiled.graph().nodes.iter().any(|node| {
		matches!(
			node.kernel.kind,
			PrimitiveKind::IndexMap(IndexMap {
				start: 1,
				element_step: 0,
				iteration_step: 1,
				modulus: None,
			})
		)
	}));
	Ok(())
}

#[test]
fn every_numeric_and_categorical_loss_accepts_unsupervised_target_rows() -> TestResult {
	for loss in [
		DenseLoss::MeanSquaredError,
		DenseLoss::MeanAbsoluteError,
		DenseLoss::Huber,
	] {
		let mut training = config();
		training.loss = loss;
		training.batch_size = NonZeroUsize::new(2).unwrap();
		let compiled = compile_dense_training(&missing_f32_regression_dataset()?, &training)?;
		compiled.graph().validate()?;
		assert!(compiled.outputs().optimizer_progress.is_some());
	}

	let mut training = config();
	training.loss = DenseLoss::CrossEntropy;
	training.batch_size = NonZeroUsize::new(2).unwrap();
	training.layers[1] = DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Linear);
	let compiled = compile_dense_training_with_multiclass_validation(
		&missing_multiclass_target_dataset()?,
		&training,
		&MulticlassValidationConfig,
	)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	let supervision = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainTargetSupervision)
		.expect("categorical target supervision exists");
	let values = supervision
		.bytes()
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one f32")))
		.collect::<Vec<_>>();
	assert_eq!(values, [1.0, 1.0, 0.0, 1.0, 0.0]);
	let validation_targets = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::ValidationTargets)
		.expect("known validation target remains");
	assert_eq!(validation_targets.shape().extents(), [1, 1]);
	assert_eq!(
		compiled.outputs().validation_status,
		ValidationMetricStatus::Available {
			family: ValidationMetricFamily::Multiclass,
			known_rows: NonZeroU64::new(1).unwrap(),
		}
	);
	Ok(())
}

#[test]
fn every_loss_sanitizes_ignored_logits_and_targets_before_checked_math() -> TestResult {
	let mut cases = Vec::new();
	for loss in [
		DenseLoss::BinaryCrossEntropy,
		DenseLoss::MeanSquaredError,
		DenseLoss::MeanAbsoluteError,
		DenseLoss::Huber,
	] {
		let mut training = config();
		training.loss = loss;
		training.batch_size = NonZeroUsize::new(2).unwrap();
		let dataset = if loss == DenseLoss::BinaryCrossEntropy {
			partially_missing_binary_dataset()?
		} else {
			missing_f32_regression_dataset()?
		};
		cases.push(compile_dense_training(&dataset, &training)?);
	}
	let mut cross_entropy = config();
	cross_entropy.loss = DenseLoss::CrossEntropy;
	cross_entropy.batch_size = NonZeroUsize::new(2).unwrap();
	cross_entropy.layers[1] = DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Linear);
	cases.push(compile_dense_training(
		&missing_multiclass_target_dataset()?,
		&cross_entropy,
	)?);

	for compiled in &cases {
		let supervision = compiled
			.external_inputs()
			.iter()
			.find(|input| input.role() == ExternalInputRole::TrainTargetSupervision)
			.expect("target supervision exists")
			.value();
		let checked_loss =
			compiled
				.graph()
				.nodes
				.iter()
				.find(|node| {
					let PrimitiveKind::Elementwise(elementwise) = &node.kernel.kind else {
						return false;
					};
					node.kernel.outputs.len() == 2
						&& node.kernel.outputs.iter().all(|output| {
							graph_tensor(compiled, *output).shape.extents().first() == Some(&2)
						}) && elementwise
						.program
						.instructions
						.iter()
						.any(|instruction| instruction.opcode == ScalarOpcode::Require)
				})
				.expect("checked loss program exists");
		for input in &checked_loss.kernel.inputs[..2] {
			assert!(select_zero_producer_uses(compiled, *input, supervision));
		}

		if matches!(
			compiled.dataset_schema().task(),
			DenseTask::MulticlassClassification { .. }
		) {
			let row_maximum =
				compiled
					.graph()
					.nodes
					.iter()
					.find(|node| {
						matches!(
							&node.kernel.kind,
							PrimitiveKind::Reduce(reduce)
								if reduce.operator == recipe_language::ReduceOperator::Maximum
						) && node.kernel.inputs.iter().any(|input| {
							graph_tensor(compiled, *input).shape.extents().first() == Some(&2)
						})
					})
					.expect("cross-entropy row maximum exists");
			assert!(select_zero_producer_uses(
				compiled,
				row_maximum.kernel.inputs[0],
				supervision,
			));
		}
	}
	Ok(())
}

#[test]
fn entirely_unsupervised_training_is_a_finite_planned_no_op() -> TestResult {
	let mut training = config();
	training.batch_size = NonZeroUsize::new(2).unwrap();
	let compiled = compile_dense_training(&entirely_unsupervised_training_dataset()?, &training)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	let progress = compiled
		.outputs()
		.optimizer_progress
		.expect("all-unsupervised training retains explicit optimizer progress");
	assert_eq!(progress.accepted_updates_per_epoch, 0);
	assert_eq!(progress.maximum_accepted_updates, 0);
	assert_eq!(progress.warmup_accepted_updates, 0);
	let supervision = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainTargetSupervision)
		.expect("all-unsupervised target mask exists");
	assert!(
		supervision.bytes().chunks_exact(4).all(|bytes| {
			f32::from_le_bytes(bytes.try_into().expect("one f32")).to_bits() == 0.0_f32.to_bits()
		})
	);
	let _manifest = CheckpointManifest::from_compiled(&compiled)?;
	Ok(())
}

#[test]
fn zero_and_one_known_categorical_dictionaries_form_bounded_bce_and_ce_tasks() -> TestResult {
	for loss in [DenseLoss::BinaryCrossEntropy, DenseLoss::CrossEntropy] {
		let mut training = config();
		training.loss = loss;
		training.batch_size = NonZeroUsize::new(2).unwrap();

		let zero_known = compile_dense_training(
			&entirely_unsupervised_categorical_training_dataset()?,
			&training,
		)?;
		zero_known.graph().validate()?;
		zero_known.program().validate()?;
		let zero_progress = zero_known
			.outputs()
			.optimizer_progress
			.expect("zero-known categorical task retains the optimizer gate");
		assert_eq!(zero_progress.accepted_updates_per_epoch, 0);
		assert_eq!(zero_progress.maximum_accepted_updates, 0);
		assert_eq!(zero_known.dataset_schema().output_width(), 1);
		match loss {
			DenseLoss::BinaryCrossEntropy => assert_eq!(
				zero_known.dataset_schema().task(),
				DenseTask::BinaryClassification {
					target_vector: 2,
					positive_code: -1,
				}
			),
			DenseLoss::CrossEntropy => assert_eq!(
				zero_known.dataset_schema().task(),
				DenseTask::MulticlassClassification {
					target_vector: 2,
					class_count: 1,
					reserved_code: 0,
				}
			),
			_ => unreachable!(),
		}
		if loss == DenseLoss::CrossEntropy {
			assert_eq!(
				zero_known.dataset_schema().decode_multiclass_class(0),
				Some(DecodedMulticlassClass::ReservedUnseen)
			);
		}
		let zero_target = zero_known
			.dataset_schema()
			.vectors()
			.iter()
			.find(|vector| vector.role() == VectorRole::Target)
			.expect("zero-known categorical schema exists");
		assert_eq!(
			zero_target.metadata(),
			&VectorMetadata::Categorical {
				dictionary: Vec::new()
			}
		);
		assert!(
			zero_known
				.external_inputs()
				.iter()
				.find(|input| input.role() == ExternalInputRole::TrainTargetSupervision)
				.expect("zero-known supervision exists")
				.bytes()
				.chunks_exact(4)
				.all(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()) == 0.0_f32.to_bits())
		);
		assert!(
			zero_known
				.external_inputs()
				.iter()
				.find(|input| input.role() == ExternalInputRole::TrainTargets)
				.expect("zero-known target placeholders exist")
				.bytes()
				.chunks_exact(4)
				.all(|bytes| bytes == [0, 0, 0, 0])
		);
		let _zero_manifest = CheckpointManifest::from_compiled(&zero_known)?;

		let one_known = compile_dense_training(&one_known_categorical_training_dataset()?, &training)?;
		one_known.graph().validate()?;
		one_known.program().validate()?;
		let one_progress = one_known
			.outputs()
			.optimizer_progress
			.expect("one-known categorical task retains the optimizer gate");
		assert_eq!(one_progress.accepted_updates_per_epoch, 1);
		assert_eq!(one_progress.maximum_accepted_updates, 2);
		assert_eq!(
			one_known.dataset_schema().output_width(),
			if loss == DenseLoss::CrossEntropy {
				2
			} else {
				1
			}
		);
		let one_supervision = one_known
			.external_inputs()
			.iter()
			.find(|input| input.role() == ExternalInputRole::TrainTargetSupervision)
			.expect("one-known supervision exists")
			.bytes()
			.chunks_exact(4)
			.map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
			.collect::<Vec<_>>();
		assert_eq!(one_supervision, [1.0, 0.0, 0.0, 0.0, 0.0]);
		if loss == DenseLoss::BinaryCrossEntropy {
			assert_eq!(
				one_known.dataset_schema().task(),
				DenseTask::BinaryClassification {
					target_vector: 2,
					positive_code: 0,
				}
			);
			let targets = one_known
				.external_inputs()
				.iter()
				.find(|input| input.role() == ExternalInputRole::TrainTargets)
				.expect("one-label BCE targets exist")
				.bytes()
				.chunks_exact(4)
				.map(|bytes| i32::from_le_bytes(bytes.try_into().unwrap()))
				.collect::<Vec<_>>();
			assert_eq!(targets, [1, 0, 0, 0, 0]);
		}
		if loss == DenseLoss::CrossEntropy {
			assert_eq!(
				one_known.dataset_schema().decode_multiclass_class(0),
				Some(DecodedMulticlassClass::Label(b"known"))
			);
			assert_eq!(
				one_known.dataset_schema().decode_multiclass_class(1),
				Some(DecodedMulticlassClass::ReservedUnseen)
			);
		}
		let _one_manifest = CheckpointManifest::from_compiled(&one_known)?;
	}
	Ok(())
}

#[test]
fn binary_validation_requires_both_known_classes_without_rejecting_training() -> TestResult {
	let validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), []);
	for (dataset, known_rows) in [
		(one_known_categorical_training_dataset()?, 1),
		(binary_single_class_validation_dataset()?, 2),
	] {
		let compiled = compile_dense_training_with_binary_validation(&dataset, &config(), &validation)?;
		assert_eq!(
			compiled.outputs().validation_status,
			ValidationMetricStatus::Unavailable {
				family: ValidationMetricFamily::Binary,
				reason: ValidationUnavailableReason::SingleKnownClass {
					known_rows: NonZeroU64::new(known_rows).unwrap(),
				},
				split_rows: 2,
			}
		);
		assert!(compiled.outputs().validation.is_none());
	}

	let mut cross_entropy = config();
	cross_entropy.loss = DenseLoss::CrossEntropy;
	let compiled = compile_dense_training_with_multiclass_validation(
		&one_known_categorical_training_dataset()?,
		&cross_entropy,
		&MulticlassValidationConfig,
	)?;
	assert_eq!(
		compiled.outputs().validation_status,
		ValidationMetricStatus::Available {
			family: ValidationMetricFamily::Multiclass,
			known_rows: NonZeroU64::MIN,
		}
	);
	Ok(())
}

#[test]
fn ignored_activation_lanes_are_selected_before_and_after_checked_nonlinearity() -> TestResult {
	let mut training = config();
	training.batch_size = NonZeroUsize::new(2).unwrap();
	training.layers[0] = DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Exponential);
	let compiled = compile_dense_training(
		&entirely_unsupervised_categorical_training_dataset()?,
		&training,
	)?;
	let supervision = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == ExternalInputRole::TrainTargetSupervision)
		.expect("target supervision exists")
		.value();
	let activation = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			let PrimitiveKind::Elementwise(elementwise) = &node.kernel.kind else {
				return false;
			};
			node.kernel.inputs.len() == 1
				&& node.kernel.outputs.len() == 1
				&& graph_tensor(&compiled, node.kernel.outputs[0])
					.shape
					.extents() == [2, 4]
				&& elementwise
					.program
					.instructions
					.iter()
					.any(|instruction| instruction.opcode == ScalarOpcode::Require)
				&& select_zero_producer_uses(&compiled, node.kernel.inputs[0], supervision)
		})
		.expect("checked exponential activation exists");
	assert!(compiled.graph().nodes.iter().any(|node| {
		node.kernel.inputs.first() == activation.kernel.outputs.first()
			&& node
				.kernel
				.outputs
				.first()
				.copied()
				.is_some_and(|output| select_zero_producer_uses(&compiled, output, supervision))
	}));
	Ok(())
}

#[test]
fn zero_known_validation_targets_are_explicitly_unavailable() -> TestResult {
	let validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), [])
		.with_auprc_early_stopping(NonZeroU64::new(2).unwrap())
		.with_temperature_scaling(TemperatureScalingConfig {
			iterations: NonZeroU64::new(3).unwrap(),
			..TemperatureScalingConfig::default()
		});
	let compiled = compile_dense_training_with_binary_validation(
		&unknown_validation_target_dataset()?,
		&config(),
		&validation,
	)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	assert_eq!(
		compiled.outputs().validation_status,
		ValidationMetricStatus::Unavailable {
			family: ValidationMetricFamily::Binary,
			reason: ValidationUnavailableReason::NoKnownTargets,
			split_rows: 2,
		}
	);
	assert_eq!(compiled.outputs().validation, None);
	assert_eq!(compiled.outputs().multiclass_validation, None);
	assert_eq!(compiled.bounds().calibration_iterations, 0);
	assert_eq!(
		compiled.bounds().iterations,
		compiled.bounds().training_iterations
	);
	assert_eq!(compiled.outputs().metric_bindings.len(), 1);
	assert!(compiled.external_inputs().iter().all(|input| {
		!matches!(
			input.role(),
			ExternalInputRole::ValidationFeatures | ExternalInputRole::ValidationTargets
		)
	}));
	Ok(())
}

#[test]
fn dense_calculation_rejects_inexact_i32_to_f32_conversion() -> TestResult {
	let dataset = prepare_csv(
		b"feature_one,feature_two,target\n\
		16777217,10,1\n\
		2,20,2\n\
		3,30,3\n\
		4,40,4\n\
		5,50,5\n\
		6,60,6\n\
		7,70,7\n",
	)?;
	let mut config = config();
	config.loss = DenseLoss::MeanAbsoluteError;
	let error = compile_dense_training(&dataset, &config).unwrap_err();
	assert_eq!(error.kind, TrainingCompileErrorKind::InvalidFeatureMatrix);
	assert!(error.detail.contains("16777217"));
	Ok(())
}

#[test]
fn compiles_contextual_activations_and_learning_rate_decays() -> TestResult {
	for activation in [
		DenseActivation::Linear,
		DenseActivation::Cosine,
		DenseActivation::Exponential,
		DenseActivation::Logarithm,
		DenseActivation::Huber,
		DenseActivation::Tangent,
		DenseActivation::Relu,
		DenseActivation::Gelu,
		DenseActivation::Silu,
	] {
		for decay in [
			LearningRateDecay::Linear,
			LearningRateDecay::Cosine,
			LearningRateDecay::Exponential,
		] {
			let mut config = config();
			config.layers[0] = DenseLayer::new(NonZeroU64::new(4).unwrap(), activation);
			config.learning_rate_decay = decay;
			let compiled = compile_dense_training(&binary_dataset()?, &config)?;
			compiled.graph().validate()?;
			compiled.program().validate()?;
		}
	}
	Ok(())
}

#[test]
fn temperature_calibration_does_not_change_the_training_schedule() -> TestResult {
	let validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), []);
	let calibrated_validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), [])
		.with_temperature_scaling(TemperatureScalingConfig {
			iterations: NonZeroU64::new(3).unwrap(),
			..TemperatureScalingConfig::default()
		});
	let dataset = binary_dataset()?;
	let without_calibration = compile_dense_training_with_binary_validation(&dataset, &config(), &validation)?;
	let with_calibration =
		compile_dense_training_with_binary_validation(&dataset, &config(), &calibrated_validation)?;
	assert_eq!(without_calibration.bounds().training_iterations.get(), 4);
	assert_eq!(with_calibration.bounds().training_iterations.get(), 4);
	assert_eq!(without_calibration.bounds().iterations.get(), 4);
	assert_eq!(with_calibration.bounds().iterations.get(), 7);
	assert_eq!(
		training_schedule_program(&without_calibration),
		training_schedule_program(&with_calibration),
	);
	Ok(())
}

#[test]
fn zero_adam_betas_compile_as_one_shared_recurrent_state() -> TestResult {
	let mut config = config();
	config.adamw.beta_one = 0.0;
	config.adamw.beta_two = 0.0;
	config.learning_rate_decay = LearningRateDecay::Linear;
	let compiled = compile_dense_training(&binary_dataset()?, &config)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	let adam = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == compiled.outputs().layers[0].weight.update_kernel)
		.expect("AdamW update exists");
	let beta_powers = [adam.kernel.inputs[5], adam.kernel.inputs[6]];
	let recurrence = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			beta_powers
				.iter()
				.all(|power| node.kernel.outputs.contains(power))
		})
		.expect("shared beta-power recurrence exists");
	assert_eq!(
		compiled.program().domain(recurrence.kernel.id),
		Some(IterationDomain::new(0, 4, 1).unwrap()),
	);
	assert_eq!(
		recurrence
			.kernel
			.alias_rules
			.iter()
			.filter(|rule| rule.permission == AliasPermission::MustAliasExact)
			.map(|rule| (rule.input, rule.output))
			.collect::<BTreeSet<_>>(),
		BTreeSet::from([(0, 0), (1, 1)]),
	);
	let PrimitiveKind::Elementwise(elementwise) = &recurrence.kernel.kind else {
		panic!("beta-power recurrence is not elementwise");
	};
	assert_eq!(elementwise.program.instructions.len(), 2);
	assert!(
		elementwise
			.program
			.instructions
			.iter()
			.all(|instruction| instruction.opcode == ScalarOpcode::Multiply)
	);
	for initial_power in &recurrence.kernel.inputs {
		let initializer = compiled
			.graph()
			.nodes
			.iter()
			.find(|node| node.kernel.outputs.contains(initial_power))
			.expect("beta-power initializer exists");
		assert_eq!(
			compiled.program().domain(initializer.kernel.id),
			Some(IterationDomain::first()),
		);
	}
	Ok(())
}

#[test]
fn compiles_exact_partial_batches_and_owned_external_bytes() -> TestResult {
	let compiled = compile_dense_training(&binary_dataset()?, &config())?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	assert_eq!(compiled.bounds().train_rows, 5);
	assert_eq!(compiled.bounds().batch_size, 3);
	assert_eq!(compiled.bounds().batches_per_epoch, 2);
	assert_eq!(compiled.bounds().padded_rows_per_epoch, 6);
	assert_eq!(compiled.bounds().iterations.get(), 4);
	assert_eq!(compiled.bounds().warmup_iterations, 2);
	assert_eq!(compiled.outputs().metric_bindings.len(), 1);
	let batch_metric = compiled.outputs().metric_bindings[0];
	assert_eq!(batch_metric.kind, TrainingMetricKind::BatchLoss);
	assert_eq!(batch_metric.metric, MetricId::new(1));
	assert_eq!(batch_metric.value, compiled.outputs().batch_loss);
	assert_eq!(batch_metric.domain, compiled.outputs().batch_loss_domain);
	assert_eq!(
		compiled.program().metric(batch_metric.metric),
		Some(recipe_program::MetricEmission {
			metric: batch_metric.metric,
			value: batch_metric.value,
			domain: batch_metric.domain,
		})
	);

	let manifest = compiled.external_inputs();
	assert_eq!(manifest.len(), 4);
	assert_eq!(
		manifest
			.iter()
			.map(|input| input.role())
			.collect::<Vec<_>>(),
		[
			ExternalInputRole::TrainFeatures,
			ExternalInputRole::TrainTargets,
			ExternalInputRole::ValidationFeatures,
			ExternalInputRole::ValidationTargets,
		]
	);
	for input in manifest {
		assert_eq!(input.dtype(), DType::I32);
		assert_eq!(
			u64::try_from(input.bytes().len())?,
			input.shape().bytes(input.dtype())?.get()
		);
		assert!(
			compiled
				.graph()
				.tensors
				.iter()
				.any(|tensor| tensor.id == input.value() && tensor.external_input)
		);
	}
	Ok(())
}

#[test]
fn emits_gpu_iteration_batching_initialization_and_recurrent_adamw() -> TestResult {
	let compiled = compile_dense_training(&binary_dataset()?, &config())?;
	let mut batch_map = false;
	let mut step_map = false;
	let mut normal_initializers = 0;
	for node in &compiled.graph().nodes {
		match node.kernel.kind {
			PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 3,
				modulus: Some(6),
			}) => batch_map = true,
			PrimitiveKind::IndexMap(IndexMap {
				start: 1,
				element_step: 0,
				iteration_step: 1,
				modulus: None,
			}) => step_map = true,
			PrimitiveKind::Random(random) if random.distribution == RandomDistribution::NormalF32 => {
				normal_initializers += 1;
				assert_eq!(
					compiled.program().domain(node.kernel.id),
					Some(IterationDomain::first())
				);
			}
			_ => {}
		}
	}
	assert!(batch_map);
	assert!(step_map);
	assert_eq!(normal_initializers, 2);

	for layer in &compiled.outputs().layers {
		for parameter in [layer.weight, layer.bias] {
			let update = compiled
				.graph()
				.nodes
				.iter()
				.find(|node| node.kernel.id == parameter.update_kernel)
				.expect("reported AdamW kernel exists");
			let must_alias = update
				.kernel
				.alias_rules
				.iter()
				.filter(|rule| rule.permission == AliasPermission::MustAliasExact)
				.map(|rule| (rule.input, rule.output))
				.collect::<BTreeSet<_>>();
			assert_eq!(must_alias, BTreeSet::from([(1, 2), (2, 0), (3, 1)]));
			assert_eq!(
				compiled.program().domain(parameter.update_kernel),
				Some(IterationDomain::every(compiled.program().iterations()))
			);
		}
	}
	Ok(())
}

#[test]
fn graph_and_program_ogdl_round_trip_deterministically() -> TestResult {
	let first = compile_dense_training(&binary_dataset()?, &config())?;
	let second = compile_dense_training(&binary_dataset()?, &config())?;
	assert_eq!(first.graph(), second.graph());
	assert_eq!(first.program(), second.program());

	let graph_text = first.graph().to_ogdl()?;
	assert_eq!(
		recipe_language::CalculationGraph::from_ogdl(&graph_text)?,
		*first.graph()
	);
	let program_text = first.program().to_ogdl()?;
	assert_eq!(
		StaticCalculationProgram::from_ogdl(&program_text)?,
		*first.program()
	);
	Ok(())
}

#[test]
fn validation_metrics_early_stop_and_temperature_have_exact_domains() -> TestResult {
	let validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), [0.1, 0.2, 0.3])
		.with_auprc_early_stopping(NonZeroU64::new(2).unwrap())
		.with_temperature_scaling(TemperatureScalingConfig {
			iterations: NonZeroU64::new(3).unwrap(),
			..TemperatureScalingConfig::default()
		});
	let compiled = compile_dense_training_with_binary_validation(&binary_dataset()?, &config(), &validation)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	assert_eq!(compiled.bounds().training_iterations.get(), 4);
	assert_eq!(compiled.bounds().calibration_iterations, 3);
	assert_eq!(compiled.bounds().iterations.get(), 7);
	let metric_bindings = &compiled.outputs().metric_bindings;
	assert_eq!(metric_bindings.len(), 9);
	assert_eq!(
		metric_bindings
			.iter()
			.map(|binding| binding.metric.get())
			.collect::<Vec<_>>(),
		(1_u64..=9).collect::<Vec<_>>()
	);
	assert_eq!(
		metric_bindings
			.iter()
			.map(|binding| binding.kind)
			.collect::<Vec<_>>(),
		[
			TrainingMetricKind::BatchLoss,
			TrainingMetricKind::ValidationMeanBce,
			TrainingMetricKind::AuRoc,
			TrainingMetricKind::AuPrc,
			TrainingMetricKind::BrierScore,
			TrainingMetricKind::ExpectedCalibrationError,
			TrainingMetricKind::RecallAt {
				threshold_bits: 0.1_f32.to_bits(),
			},
			TrainingMetricKind::RecallAt {
				threshold_bits: 0.2_f32.to_bits(),
			},
			TrainingMetricKind::RecallAt {
				threshold_bits: 0.3_f32.to_bits(),
			},
		]
	);
	assert_eq!(compiled.program().metrics().len(), metric_bindings.len());
	for binding in metric_bindings {
		let emission = compiled
			.program()
			.metric(binding.metric)
			.expect("training metric is attached to the static program");
		assert_eq!(emission.value, binding.value);
		assert_eq!(emission.domain, binding.domain);
	}

	let outputs = compiled
		.outputs()
		.validation
		.as_ref()
		.expect("validation outputs are declared");
	assert_eq!(outputs.metrics.recall_at.len(), 3);
	assert_eq!(
		outputs
			.metrics
			.recall_at
			.iter()
			.map(|output| output.threshold())
			.collect::<Vec<_>>(),
		[0.1, 0.2, 0.3]
	);
	let validation_domain = IterationDomain::new(1, 4, 2).unwrap();
	for metric in [
		outputs.metrics.mean_bce,
		outputs.metrics.auroc,
		outputs.metrics.auprc,
		outputs.metrics.brier_score,
		outputs.metrics.expected_calibration_error,
	] {
		let producer = compiled
			.graph()
			.nodes
			.iter()
			.find(|node| node.kernel.outputs.contains(&metric))
			.expect("metric has a producer");
		assert_eq!(
			compiled.program().domain(producer.kernel.id),
			Some(validation_domain)
		);
		assert!(
			!compiled
				.graph()
				.tensors
				.iter()
				.find(|tensor| tensor.id == metric)
				.expect("metric tensor exists")
				.external_output
		);
	}

	let early = outputs.early_stopping.expect("early stopping state exists");
	assert_eq!(
		compiled.program().domain(early.update_kernel),
		Some(validation_domain)
	);
	let early_kernel = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == early.update_kernel)
		.expect("early-stop update exists");
	assert_eq!(
		early_kernel
			.kernel
			.alias_rules
			.iter()
			.filter(|rule| rule.permission == AliasPermission::MustAliasExact)
			.map(|rule| (rule.input, rule.output))
			.collect::<BTreeSet<_>>(),
		BTreeSet::from([(1, 0), (2, 1), (3, 2)])
	);
	let first_adam = compiled.outputs().layers[0].weight.update_kernel;
	assert_eq!(
		compiled.program().domain(first_adam),
		Some(IterationDomain::new(0, 4, 1).unwrap())
	);
	let learning_rate = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == first_adam)
		.expect("AdamW update exists")
		.kernel
		.inputs[4];
	let learning_rate_kernel = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.outputs.contains(&learning_rate))
		.expect("learning-rate producer exists");
	let progress = compiled
		.outputs()
		.optimizer_progress
		.expect("early stopping gates complete optimizer progress");
	assert!(
		learning_rate_kernel
			.kernel
			.inputs
			.contains(&progress.apply_update)
	);
	let update_gate = producer(&compiled, progress.apply_update);
	assert!(update_gate.kernel.inputs.contains(&early.initial_stopped));

	let temperature = outputs
		.temperature_scaling
		.expect("temperature-scaling state exists");
	assert_eq!(
		compiled.program().domain(temperature.update_kernel),
		Some(IterationDomain::new(4, 7, 1).unwrap())
	);
	let temperature_kernel = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == temperature.update_kernel)
		.expect("temperature update exists");
	assert_eq!(
		temperature_kernel.kernel.alias_rules[0].permission,
		AliasPermission::MustAliasExact
	);
	assert!(
		compiled
			.graph()
			.tensors
			.iter()
			.find(|tensor| tensor.id == temperature.updated_temperature)
			.expect("updated temperature exists")
			.external_output
	);
	Ok(())
}

fn graph_tensor(compiled: &CompiledTraining, value: ValueId) -> &Tensor {
	compiled
		.graph()
		.tensors
		.iter()
		.find(|tensor| tensor.id == value)
		.expect("graph tensor exists")
}

fn producer(compiled: &CompiledTraining, value: ValueId) -> &CalculationNode {
	compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.outputs.contains(&value))
		.expect("value producer exists")
}

fn value_depends_on(compiled: &CompiledTraining, value: ValueId, ancestor: ValueId) -> bool {
	let mut pending = vec![value];
	let mut visited = BTreeSet::new();
	while let Some(current) = pending.pop() {
		if current == ancestor {
			return true;
		}
		if !visited.insert(current) {
			continue;
		}
		if let Some(producer) = compiled
			.graph()
			.nodes
			.iter()
			.find(|node| node.kernel.outputs.contains(&current))
		{
			pending.extend(producer.kernel.inputs.iter().copied());
		}
	}
	false
}

fn select_zero_producer_uses(compiled: &CompiledTraining, value: ValueId, supervision: ValueId) -> bool {
	let producer = producer(compiled, value);
	let PrimitiveKind::Elementwise(elementwise) = &producer.kernel.kind else {
		return false;
	};
	producer
		.kernel
		.inputs
		.get(1)
		.copied()
		.is_some_and(|validity| value_depends_on(compiled, validity, supervision))
		&& elementwise
			.program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Select)
}

fn selected_zero_source(compiled: &CompiledTraining, value: ValueId) -> Option<ValueId> {
	let producer = producer(compiled, value);
	let PrimitiveKind::Elementwise(elementwise) = &producer.kernel.kind else {
		return None;
	};
	(producer.kernel.inputs.len() == 2
		&& elementwise
			.program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::GreaterThan)
		&& elementwise
			.program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Select))
	.then_some(producer.kernel.inputs[0])
}

fn strip_selected_zero_producers(compiled: &CompiledTraining, mut value: ValueId) -> ValueId {
	while let Some(source) = selected_zero_source(compiled, value) {
		value = source;
	}
	value
}

fn forward_contraction<'a>(compiled: &'a CompiledTraining, state: &DenseLayerState) -> &'a CalculationNode {
	compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.get(1) == Some(&state.weight.initial_parameter)
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction)
						if contraction.batch_axes.is_empty() && contraction.contract_axes == [(1, 0)]
				)
		})
		.expect("layer forward contraction exists")
}

fn affine_output(compiled: &CompiledTraining, state: &DenseLayerState) -> ValueId {
	let contracted = forward_contraction(compiled, state).kernel.outputs[0];
	compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.inputs == [contracted, state.bias.initial_parameter])
		.and_then(|node| node.kernel.outputs.first().copied())
		.expect("layer bias addition exists")
}

fn exact_add<'a>(compiled: &'a CompiledTraining, left: ValueId, right: ValueId) -> &'a CalculationNode {
	compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs == [left, right]
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Elementwise(elementwise)
						if elementwise.program.inputs.len() == 2
							&& elementwise.program.outputs.len() == 1
							&& elementwise.program.instructions.len() == 1
							&& elementwise.program.instructions[0].opcode == ScalarOpcode::Add
				)
		})
		.expect("exact binary add exists")
}

fn regression_config() -> DenseTrainingConfig {
	let mut config = config();
	config.loss = DenseLoss::MeanSquaredError;
	config.epochs = NonZeroU64::new(1).unwrap();
	config.warmup_epochs = 0;
	config
}

#[test]
fn channelwise_pool_retains_shape_winners_backward_and_validation_topology() -> TestResult {
	let blocks = vec![
		DenseBlock::Pool(DensePool::new(
			NonZeroU64::new(2).unwrap(),
			NonZeroU64::new(4),
		)),
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(4).unwrap(),
			DenseActivation::Silu,
		)),
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(1).unwrap(),
			DenseActivation::Linear,
		)),
	];
	let validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), []);
	let compiled = compile_dense_training_with_blocks_and_binary_validation(
		&four_feature_binary_dataset()?,
		&config(),
		&blocks,
		&validation,
	)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	assert_eq!(
		CheckpointManifest::from_compiled(&compiled)?.format_version(),
		7
	);
	let [
		DenseBlockState::Pool(pool),
		DenseBlockState::Layer(first),
		DenseBlockState::Layer(_),
	] = compiled.outputs().blocks.as_slice()
	else {
		panic!("expected pool, routed layer, output layer state topology");
	};
	assert_eq!(pool.input_length().get(), 4);
	assert_eq!(pool.channels().get(), 1);
	assert_eq!(pool.output_length().get(), 2);
	assert_eq!(
		pool.logical_input_shape(NonZeroU64::new(3).unwrap()),
		[3, 4, 1]
	);
	assert_eq!(
		pool.logical_output_shape(NonZeroU64::new(3).unwrap()),
		[3, 2, 1]
	);
	assert_eq!(
		pool.group_order(),
		DensePoolGroupOrder::GroupMajorChannelMinor
	);
	assert_eq!(
		pool.winner_contract(),
		DensePoolWinnerContract::LowestLogicalIndex
	);
	assert_eq!(
		graph_tensor(&compiled, first.weight.initial_parameter)
			.shape
			.extents(),
		[2, 4]
	);
	let maximum_with_winners = compiled
		.graph()
		.nodes
		.iter()
		.filter(|node| {
			matches!(
				&node.kernel.kind,
				PrimitiveKind::Reduce(recipe_language::Reduce {
					operator: ReduceOperator::Maximum,
					result: ReduceResult::ValueAndIndex,
					..
				})
			)
		})
		.count();
	assert_eq!(
		maximum_with_winners, 2,
		"training and validation each pool once"
	);
	assert!(compiled.graph().nodes.iter().any(|node| {
		matches!(
			&node.kernel.kind,
			PrimitiveKind::Scatter(recipe_language::Scatter {
				conflict: ScatterConflict::UniqueIndices,
				..
			})
		) && node.kernel.inputs.iter().any(|input| {
			graph_tensor(&compiled, *input).dtype == DType::I32
				&& graph_tensor(&compiled, *input).shape.extents() == [3, 2, 1]
		})
	}));
	for role in [
		ExternalInputRole::TrainingPoolWindowIndices { block: 0 },
		ExternalInputRole::TrainingPoolWinnerBases { block: 0 },
		ExternalInputRole::TrainingPoolGradientBatchIndices { block: 0 },
		ExternalInputRole::ValidationPoolWindowIndices { block: 0 },
		ExternalInputRole::ValidationPoolWinnerBases { block: 0 },
	] {
		assert!(
			compiled
				.external_inputs()
				.iter()
				.any(|input| input.role() == role),
			"missing prepared pool input {role:?}"
		);
	}
	Ok(())
}

#[test]
fn repeated_pool_blocks_preserve_logical_length_until_the_dense_boundary() -> TestResult {
	let blocks = vec![
		DenseBlock::Pool(DensePool::new(NonZeroU64::new(2).unwrap(), None)),
		DenseBlock::Pool(DensePool::new(
			NonZeroU64::new(2).unwrap(),
			NonZeroU64::new(1),
		)),
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(1).unwrap(),
			DenseActivation::Linear,
		)),
	];
	let compiled = compile_dense_training_with_blocks(&four_feature_binary_dataset()?, &config(), &blocks)?;
	compiled.graph().validate()?;
	compiled.program().validate()?;
	let [
		DenseBlockState::Pool(first),
		DenseBlockState::Pool(second),
		DenseBlockState::Layer(_),
	] = compiled.outputs().blocks.as_slice()
	else {
		panic!("expected two pool states followed by one layer state");
	};
	assert_eq!(
		(first.input_length().get(), first.output_length().get()),
		(4, 2)
	);
	assert_eq!(
		(second.input_length().get(), second.output_length().get()),
		(2, 1)
	);
	assert_eq!(
		CheckpointManifest::from_compiled(&compiled)?.format_version(),
		7
	);
	assert_eq!(
		compiled
			.graph()
			.nodes
			.iter()
			.filter(|node| matches!(
				&node.kernel.kind,
				PrimitiveKind::Reduce(recipe_language::Reduce {
					operator: ReduceOperator::Maximum,
					result: ReduceResult::ValueAndIndex,
					..
				})
			))
			.count(),
		2
	);
	Ok(())
}

#[test]
fn flat_entrypoint_and_structured_flat_blocks_are_identical() -> TestResult {
	let dataset = binary_dataset()?;
	let config = config();
	let blocks = config
		.layers
		.iter()
		.cloned()
		.map(DenseBlock::Layer)
		.collect::<Vec<_>>();
	let legacy = compile_dense_training(&dataset, &config)?;
	let structured = compile_dense_training_with_blocks(&dataset, &config, &blocks)?;
	assert_eq!(legacy.graph(), structured.graph());
	assert_eq!(legacy.program(), structured.program());
	assert_eq!(legacy.layers(), structured.layers());
	assert_eq!(legacy.outputs().layers, structured.outputs().layers);
	assert_eq!(legacy.blocks(), structured.blocks());
	assert_eq!(legacy.outputs().blocks, structured.outputs().blocks);
	assert_eq!(
		CheckpointManifest::from_compiled(&legacy)?,
		CheckpointManifest::from_compiled(&structured)?
	);
	assert!(
		structured
			.outputs()
			.blocks
			.iter()
			.all(|state| matches!(state, DenseBlockState::Layer(_)))
	);
	Ok(())
}

#[test]
fn equal_width_residual_forms_an_exact_identity_diamond() -> TestResult {
	let blocks = vec![
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(2).unwrap(),
			DenseActivation::Linear,
		)),
		DenseBlock::Residual(DenseResidual::new(
			[DenseLayer::new(
				NonZeroU64::new(2).unwrap(),
				DenseActivation::Linear,
			)],
			[],
		)),
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(1).unwrap(),
			DenseActivation::Linear,
		)),
	];
	let compiled = compile_dense_training_with_blocks(&regression_dataset()?, &regression_config(), &blocks)?;
	let checkpoint = CheckpointManifest::from_compiled(&compiled)?;
	assert_eq!(checkpoint.format_version(), 6);
	let [
		DenseBlockState::Layer(first),
		DenseBlockState::Residual(residual),
		DenseBlockState::Layer(last),
	] = compiled.outputs().blocks.as_slice()
	else {
		panic!("expected layer, residual, layer state topology");
	};
	assert_eq!(residual.branch.len(), 1);
	assert_eq!(residual.projection, None);
	let input = affine_output(&compiled, first);
	let branch_contraction = forward_contraction(&compiled, &residual.branch[0]);
	assert_eq!(branch_contraction.kernel.inputs[0], input);
	let branch = affine_output(&compiled, &residual.branch[0]);
	let merge = exact_add(&compiled, branch, input);
	let merged = merge.kernel.outputs[0];
	for value in [branch, input, merged] {
		assert_eq!(graph_tensor(&compiled, value).shape.extents(), [3, 2]);
	}
	assert!(
		merge.kernel
			.alias_rules
			.iter()
			.all(|rule| rule.permission == AliasPermission::Forbidden)
	);
	assert_eq!(
		forward_contraction(&compiled, last).kernel.inputs[0],
		merged
	);
	assert_eq!(
		graph_tensor(&compiled, last.weight.initial_parameter)
			.shape
			.extents(),
		[2, 1]
	);
	Ok(())
}

#[test]
fn projected_residual_is_weight_only_and_backpropagates_both_paths() -> TestResult {
	let blocks = vec![
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(3).unwrap(),
			DenseActivation::Linear,
		)),
		DenseBlock::Residual(DenseResidual::new(
			[DenseLayer::new(
				NonZeroU64::new(5).unwrap(),
				DenseActivation::Linear,
			)],
			[],
		)),
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(1).unwrap(),
			DenseActivation::Linear,
		)),
	];
	let compiled = compile_dense_training_with_blocks(&regression_dataset()?, &regression_config(), &blocks)?;
	let checkpoint = CheckpointManifest::from_compiled(&compiled)?;
	assert_eq!(checkpoint.format_version(), 6);
	let [
		DenseBlockState::Layer(first),
		DenseBlockState::Residual(residual),
		DenseBlockState::Layer(last),
	] = compiled.outputs().blocks.as_slice()
	else {
		panic!("expected projected residual state topology");
	};
	let projection = residual
		.projection
		.expect("width mismatch creates projection");
	let residual_input = affine_output(&compiled, first);
	let branch = affine_output(&compiled, &residual.branch[0]);
	let projection_forward = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs == [residual_input, projection.initial_parameter]
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction) if contraction.contract_axes == [(1, 0)]
				)
		})
		.expect("weight-only skip projection exists");
	let projected = projection_forward.kernel.outputs[0];
	assert_eq!(
		graph_tensor(&compiled, projection.initial_parameter)
			.shape
			.extents(),
		[3, 5]
	);
	assert_eq!(graph_tensor(&compiled, projected).shape.extents(), [3, 5]);
	let merge = exact_add(&compiled, branch, projected);
	let merge_gradient = forward_contraction(&compiled, last)
		.kernel
		.inputs
		.first()
		.copied()
		.and_then(|_| {
			compiled.graph().nodes.iter().find_map(|node| {
				matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction)
						if contraction.contract_axes == [(1, 1)]
							&& node.kernel.inputs.get(1) == Some(&last.weight.initial_parameter)
				)
				.then(|| node.kernel.outputs[0])
			})
		})
		.expect("gradient arriving at residual merge exists");
	let projection_weight_gradient = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.len() == 2
				&& strip_selected_zero_producers(&compiled, node.kernel.inputs[0]) == residual_input
				&& strip_selected_zero_producers(&compiled, node.kernel.inputs[1]) == merge_gradient
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction) if contraction.contract_axes == [(0, 0)]
				)
		})
		.expect("projection weight gradient exists");
	assert_eq!(
		graph_tensor(&compiled, projection_weight_gradient.kernel.outputs[0])
			.shape
			.extents(),
		[3, 5]
	);
	let skip_dx = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.len() == 2
				&& strip_selected_zero_producers(&compiled, node.kernel.inputs[0]) == merge_gradient
				&& node.kernel.inputs[1] == projection.initial_parameter
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction) if contraction.contract_axes == [(1, 1)]
				)
		})
		.expect("projection input gradient exists")
		.kernel
		.outputs[0];
	let branch_dx = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.get(1) == Some(&residual.branch[0].weight.initial_parameter)
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction) if contraction.contract_axes == [(1, 1)]
				)
		})
		.expect("branch input gradient exists")
		.kernel
		.outputs[0];
	let backward_merge = exact_add(&compiled, branch_dx, skip_dx);
	assert_eq!(
		graph_tensor(&compiled, backward_merge.kernel.outputs[0])
			.shape
			.extents(),
		[3, 3]
	);
	for value in [
		projection.initial_parameter,
		projection.updated_parameter,
		projection.initial_first_moment,
		projection.updated_first_moment,
		projection.initial_second_moment,
		projection.updated_second_moment,
	] {
		assert_eq!(graph_tensor(&compiled, value).shape.extents(), [3, 5]);
	}
	let update = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.id == projection.update_kernel)
		.expect("projection AdamW update exists");
	assert_eq!(update.kernel.inputs[1], projection.initial_parameter);
	assert_eq!(update.kernel.inputs[2], projection.initial_first_moment);
	assert_eq!(update.kernel.inputs[3], projection.initial_second_moment);
	assert_eq!(
		update.kernel.outputs,
		[
			projection.updated_first_moment,
			projection.updated_second_moment,
			projection.updated_parameter,
		]
	);
	for value in [
		projection.updated_parameter,
		projection.updated_first_moment,
		projection.updated_second_moment,
	] {
		assert!(graph_tensor(&compiled, value).external_output);
	}
	assert_eq!(
		forward_contraction(&compiled, last).kernel.inputs[0],
		merge.kernel.outputs[0]
	);
	Ok(())
}

#[test]
fn residual_keeps_unique_streams_and_exact_pre_and_post_operation_order() -> TestResult {
	let branch = vec![
		DenseResidualOperation::Operation(DenseOperation::Activation(DenseActivation::Relu)),
		DenseResidualOperation::Layer(DenseLayer::new(
			NonZeroU64::new(5).unwrap(),
			DenseActivation::Linear,
		)),
		DenseResidualOperation::Operation(DenseOperation::Activation(DenseActivation::Relu)),
	];
	let blocks = vec![
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(3).unwrap(),
			DenseActivation::Linear,
		)),
		DenseBlock::Residual(DenseResidual::new(
			branch.clone(),
			[DenseOperation::Activation(DenseActivation::Silu)],
		)),
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(1).unwrap(),
			DenseActivation::Linear,
		)),
	];
	let compiled = compile_dense_training_with_blocks(&regression_dataset()?, &regression_config(), &blocks)?;
	let [
		DenseBlockState::Layer(first),
		DenseBlockState::Residual(residual),
		DenseBlockState::Layer(last),
	] = compiled.outputs().blocks.as_slice()
	else {
		panic!("expected ordered residual state topology");
	};
	let DenseBlock::Residual(declaration) = &compiled.blocks()[1] else {
		panic!("residual declaration is retained");
	};
	assert_eq!(declaration.branch(), branch);
	assert_eq!(
		declaration.operations(),
		[DenseOperation::Activation(DenseActivation::Silu)]
	);
	let residual_input = affine_output(&compiled, first);
	let branch_contraction = forward_contraction(&compiled, &residual.branch[0]);
	let leading_activation = producer(
		&compiled,
		strip_selected_zero_producers(&compiled, branch_contraction.kernel.inputs[0]),
	);
	assert_eq!(
		strip_selected_zero_producers(&compiled, leading_activation.kernel.inputs[0]),
		residual_input
	);
	let branch_affine = affine_output(&compiled, &residual.branch[0]);
	let projection = residual
		.projection
		.expect("three-to-five projection exists");
	let projected = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| node.kernel.inputs == [residual_input, projection.initial_parameter])
		.expect("projection forward exists")
		.kernel
		.outputs[0];
	let merge = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.get(1) == Some(&projected)
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Elementwise(elementwise)
						if elementwise.program.instructions.len() == 1
							&& elementwise.program.instructions[0].opcode == ScalarOpcode::Add
				)
		})
		.expect("residual forward merge exists");
	let branch_activation = producer(
		&compiled,
		strip_selected_zero_producers(&compiled, merge.kernel.inputs[0]),
	);
	assert_eq!(
		strip_selected_zero_producers(&compiled, branch_activation.kernel.inputs[0]),
		branch_affine
	);
	let final_input = forward_contraction(&compiled, last).kernel.inputs[0];
	let post_activation = producer(
		&compiled,
		strip_selected_zero_producers(&compiled, final_input),
	);
	assert_eq!(
		strip_selected_zero_producers(&compiled, post_activation.kernel.inputs[0]),
		merge.kernel.outputs[0]
	);

	let weights = [
		first.weight,
		residual.branch[0].weight,
		projection,
		last.weight,
	];
	let streams = weights
		.iter()
		.map(|parameter| {
			let scaled = producer(&compiled, parameter.initial_parameter);
			let random = producer(&compiled, scaled.kernel.inputs[0]);
			match &random.kernel.kind {
				PrimitiveKind::Random(random) => {
					assert_eq!(random.distribution, RandomDistribution::NormalF32);
					random.key.stream
				}
				_ => panic!("weight seed is produced by Random"),
			}
		})
		.collect::<BTreeSet<_>>();
	assert_eq!(streams, BTreeSet::from([0, 1, 2, 3]));
	let repeated = compile_dense_training_with_blocks(&regression_dataset()?, &regression_config(), &blocks)?;
	assert_eq!(compiled.graph(), repeated.graph());
	assert_eq!(compiled.program(), repeated.program());
	Ok(())
}

#[test]
fn residual_validation_reuses_updated_branch_and_projection_topology() -> TestResult {
	let blocks = vec![
		DenseBlock::Residual(DenseResidual::new(
			[DenseLayer::new(
				NonZeroU64::new(4).unwrap(),
				DenseActivation::Relu,
			)],
			[],
		)),
		DenseBlock::Layer(DenseLayer::new(
			NonZeroU64::new(1).unwrap(),
			DenseActivation::Linear,
		)),
	];
	let validation = BinaryValidationConfig::new(NonZeroU32::new(4).unwrap(), []);
	let compiled = compile_dense_training_with_blocks_and_binary_validation(
		&binary_dataset()?,
		&config(),
		&blocks,
		&validation,
	)?;
	let [
		DenseBlockState::Residual(residual),
		DenseBlockState::Layer(last),
	] = compiled.outputs().blocks.as_slice()
	else {
		panic!("expected residual validation state topology");
	};
	let projection = residual.projection.expect("two-to-four projection exists");
	let validation_domain = compiled
		.outputs()
		.validation
		.as_ref()
		.expect("binary validation exists")
		.metric_domain;
	let validation_projection = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.get(1) == Some(&projection.updated_parameter)
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction) if contraction.contract_axes == [(1, 0)]
				)
		})
		.expect("validation projection consumes updated weight");
	assert_eq!(
		compiled.program().domain(validation_projection.kernel.id),
		Some(validation_domain)
	);
	let validation_branch = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.get(1) == Some(&residual.branch[0].weight.updated_parameter)
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction) if contraction.contract_axes == [(1, 0)]
				)
		})
		.expect("validation branch consumes updated weight");
	assert_eq!(
		compiled.program().domain(validation_branch.kernel.id),
		Some(validation_domain)
	);
	let validation_last = compiled
		.graph()
		.nodes
		.iter()
		.find(|node| {
			node.kernel.inputs.get(1) == Some(&last.weight.updated_parameter)
				&& matches!(
					&node.kernel.kind,
					PrimitiveKind::Contraction(contraction) if contraction.contract_axes == [(1, 0)]
				)
		})
		.expect("validation continuation consumes updated weight");
	assert_eq!(
		compiled.program().domain(validation_last.kernel.id),
		Some(validation_domain)
	);
	Ok(())
}
