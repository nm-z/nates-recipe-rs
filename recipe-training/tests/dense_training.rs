use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};
use std::collections::BTreeSet;
use std::error::Error;

use recipe_core::{AliasPermission, DType, MetricId};
use recipe_ingest::DenseMatrix;
use recipe_language::{IndexMap, PrimitiveKind, RandomDistribution};
use recipe_program::{IterationDomain, StaticCalculationProgram};
use recipe_training::{
	AdamWConfig, BinaryValidationConfig, DenseActivation, DenseBinaryDataset, DenseLayer, DensePartition,
	DenseTrainingConfig, ExternalInputRole, TemperatureScalingConfig, TrainingMetricKind,
	compile_dense_binary_training, compile_dense_binary_training_with_validation,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn dataset() -> TestResult<DenseBinaryDataset> {
	let train = DensePartition::new(
		DenseMatrix::I32 {
			rows: 5,
			columns: 2,
			values: vec![1, 10, 2, 20, 3, 30, 4, 40, 5, 50],
		},
		DenseMatrix::I32 {
			rows: 5,
			columns: 1,
			values: vec![0, 1, 0, 1, 1],
		},
	)?;
	let validation = DensePartition::new(
		DenseMatrix::I32 {
			rows: 2,
			columns: 2,
			values: vec![6, 60, 7, 70],
		},
		DenseMatrix::I32 {
			rows: 2,
			columns: 1,
			values: vec![0, 1],
		},
	)?;
	Ok(DenseBinaryDataset::new(train, Some(validation))?)
}

fn config() -> DenseTrainingConfig {
	DenseTrainingConfig {
		layers: vec![
			DenseLayer::new(NonZeroU64::new(4).unwrap(), DenseActivation::Silu),
			DenseLayer::new(NonZeroU64::new(1).unwrap(), DenseActivation::Linear),
		],
		batch_size: NonZeroUsize::new(3).unwrap(),
		epochs: NonZeroU64::new(2).unwrap(),
		warmup_epochs: 1,
		gradient_clip_norm: 1.0,
		normalization_epsilon: 1.0e-6,
		reduction_tree_lanes: 32,
		random_seed: 17,
		adamw: AdamWConfig::default(),
	}
}

#[test]
fn compiles_exact_partial_batches_and_owned_external_bytes() -> TestResult {
	let compiled = compile_dense_binary_training(&dataset()?, &config())?;
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
	let compiled = compile_dense_binary_training(&dataset()?, &config())?;
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
	let first = compile_dense_binary_training(&dataset()?, &config())?;
	let second = compile_dense_binary_training(&dataset()?, &config())?;
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
	let compiled = compile_dense_binary_training_with_validation(&dataset()?, &config(), &validation)?;
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
	assert!(
		learning_rate_kernel
			.kernel
			.inputs
			.contains(&early.initial_stopped)
	);

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
