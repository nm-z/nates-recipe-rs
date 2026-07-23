use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use recipe_core::{
	AliasPermission, DType, KernelTemplateId, ScalarInput, ScalarInstruction, ScalarLiteral, ScalarOpcode,
	ScalarProgram, ScalarValueId, ValueId,
};
use recipe_language::{
	AtomicOperation, AtomicOrdering, AxisSet, Contraction, Elementwise, Gather, Histogram, IndexBounds,
	PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce,
	ReduceOperator, ReduceResult, Scan, ScanMode, Scatter, ScatterConflict, Shape, Sort, SortDirection, Tensor,
};
use recipe_primitives::{
	BufferLifetime, FaultReason, HistogramBinMapping, OwnedAtomicOperation, ProgramDigest, ScratchPurpose, StageKind,
	lower,
};

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug)]
struct TestFailure(String);

impl fmt::Display for TestFailure {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(formatter)
	}
}

impl Error for TestFailure {}

fn tensor(id: u64, dtype: DType, shape: &[u64]) -> Result<Tensor, recipe_language::LanguageError> {
	Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(shape.to_vec())?,
		true,
		false,
	)
}

fn index(tensors: &[Tensor]) -> BTreeMap<ValueId, &Tensor> {
	tensors.iter().map(|tensor| (tensor.id, tensor)).collect()
}

fn aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> {
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

fn kernel(id: u64, inputs: &[u64], outputs: &[u64], kind: PrimitiveKind) -> PrimitiveKernel {
	PrimitiveKernel {
		id: KernelTemplateId::new(id),
		inputs: inputs.iter().copied().map(ValueId::new).collect(),
		outputs: outputs.iter().copied().map(ValueId::new).collect(),
		alias_rules: aliases(inputs.len(), outputs.len()),
		kind,
	}
}

fn add_program() -> ScalarProgram {
	let left = ScalarValueId::new(1);
	let right = ScalarValueId::new(2);
	let result = ScalarValueId::new(3);
	ScalarProgram {
		inputs: vec![
			ScalarInput {
				id: left,
				dtype: DType::F32,
			},
			ScalarInput {
				id: right,
				dtype: DType::F32,
			},
		],
		constants: Vec::new(),
		instructions: vec![ScalarInstruction {
			result,
			dtype: DType::F32,
			opcode: ScalarOpcode::Add,
			operands: vec![left, right],
		}],
		outputs: vec![result],
	}
}

fn validate(program: &recipe_primitives::LoweredProgram) -> TestResult {
	program.validate().map_err(|errors| {
		TestFailure(
			errors.iter()
				.map(ToString::to_string)
				.collect::<Vec<_>>()
				.join("; "),
		)
	})?;
	Ok(())
}

#[test]
fn elementwise_retains_the_validated_kernel_template_and_broadcast_access() -> TestResult {
	let tensors = [
		tensor(1, DType::F32, &[2, 1])?,
		tensor(2, DType::F32, &[1, 3])?,
		tensor(3, DType::F32, &[2, 3])?,
	];
	let primitive = kernel(
		1,
		&[1, 2],
		&[3],
		PrimitiveKind::Elementwise(Elementwise {
			program: add_program(),
		}),
	);
	let program = lower(&primitive, &index(&tensors))?;
	validate(&program)?;
	assert_eq!(program.stages.len(), 1);
	let StageKind::ScalarMap { template } = &program.stages[0].kind else {
		panic!("expected scalar-map stage");
	};
	assert_eq!(template.program, add_program());
	assert_eq!(template.inputs[0].access.strides, [1, 0]);
	assert_eq!(template.inputs[1].access.strides, [0, 1]);
	assert_eq!(program.resources.total_flops.get(), 6);
	Ok(())
}

#[test]
fn every_reduction_operator_and_result_family_has_a_fixed_tree() -> TestResult {
	for (case, operator) in [
		ReduceOperator::Sum,
		ReduceOperator::Product,
		ReduceOperator::Minimum,
		ReduceOperator::Maximum,
		ReduceOperator::Any,
		ReduceOperator::All,
	]
	.into_iter()
	.enumerate()
	{
		let dtype = match matches!(operator, ReduceOperator::Any | ReduceOperator::All) {
			true => DType::I32,
			false => DType::F32,
		};
		let tensors = [tensor(1, dtype, &[2, 9])?, tensor(2, dtype, &[2])?];
		let primitive = kernel(
			u64::try_from(case + 10)?,
			&[1],
			&[2],
			PrimitiveKind::Reduce(Reduce {
				operator,
				axes: AxisSet::new(vec![1])?,
				keep_dimensions: false,
				result: ReduceResult::Value,
				tree_lanes: 4,
			}),
		);
		let program = lower(&primitive, &index(&tensors))?;
		assert!(program.stages.len() >= 2);
		assert!(
			program
				.stages
				.iter()
				.all(|stage| matches!(stage.kind, StageKind::FixedTreeReduce(_)))
		);
		validate(&program)?;
	}

	for result in [ReduceResult::Index, ReduceResult::ValueAndIndex] {
		let tensors = [
			tensor(1, DType::F32, &[2, 9])?,
			tensor(2, DType::F32, &[2])?,
			tensor(3, DType::I32, &[2])?,
		];
		let outputs: &[u64] = match result == ReduceResult::Index {
			true => &[3],
			false => &[2, 3],
		};
		let primitive = kernel(
			30 + u64::from(result == ReduceResult::ValueAndIndex),
			&[1],
			outputs,
			PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Maximum,
				axes: AxisSet::new(vec![1])?,
				keep_dimensions: false,
				result,
				tree_lanes: 4,
			}),
		);
		let program = lower(&primitive, &index(&tensors))?;
		assert!(program.buffers.iter().any(|buffer| matches!(
			buffer.origin,
			recipe_primitives::BufferOrigin::Scratch {
				purpose: ScratchPurpose::ReductionIndices,
				..
			}
		)));
		validate(&program)?;
	}
	Ok(())
}

#[test]
fn scan_is_a_hierarchical_blelloch_program_for_every_mode() -> TestResult {
	for (case, mode, reverse) in [
		(0_u64, ScanMode::Inclusive, false),
		(
			1,
			ScanMode::Exclusive {
				identity: ScalarLiteral::F32Bits(0),
			},
			false,
		),
		(2, ScanMode::Inclusive, true),
	] {
		let tensors = [
			tensor(1, DType::F32, &[2, 35])?,
			tensor(2, DType::F32, &[2, 35])?,
		];
		let primitive = kernel(
			40 + case,
			&[1],
			&[2],
			PrimitiveKind::Scan(Scan {
				operator: ReduceOperator::Sum,
				axis: 1,
				mode,
				reverse,
				tree_lanes: 4,
			}),
		);
		let program = lower(&primitive, &index(&tensors))?;
		assert!(
			program
				.stages
				.iter()
				.any(|stage| matches!(stage.kind, StageKind::ScanUniformCombine(_)))
		);
		assert!(
			program
				.stages
				.iter()
				.filter(|stage| matches!(stage.kind, StageKind::FixedTreeScanLocal(_)))
				.count() >= 2
		);
		validate(&program)?;
	}
	Ok(())
}

#[test]
fn contraction_gather_and_all_index_policies_are_explicit() -> TestResult {
	let contraction_tensors = [
		tensor(1, DType::F32, &[2, 3, 5])?,
		tensor(2, DType::F32, &[2, 5, 7])?,
		tensor(3, DType::F32, &[2, 3, 7])?,
	];
	let contraction = kernel(
		50,
		&[1, 2],
		&[3],
		PrimitiveKind::Contraction(Contraction {
			batch_axes: vec![(0, 0)],
			contract_axes: vec![(2, 1)],
		}),
	);
	let program = lower(&contraction, &index(&contraction_tensors))?;
	assert!(matches!(
		program.stages[0].kind,
		StageKind::TiledContraction(_)
	));
	assert_eq!(program.resources.total_flops.get(), 420);

	for (case, bounds) in [IndexBounds::Reject, IndexBounds::Clamp, IndexBounds::Wrap]
		.into_iter()
		.enumerate()
	{
		let tensors = [
			tensor(1, DType::F32, &[4, 9])?,
			tensor(2, DType::I32, &[3])?,
			tensor(3, DType::F32, &[4, 3])?,
		];
		let gather = kernel(
			51 + u64::try_from(case)?,
			&[1, 2],
			&[3],
			PrimitiveKind::Gather(Gather { axis: 1, bounds }),
		);
		let program = lower(&gather, &index(&tensors))?;
		assert_eq!(
			program.stages[0].fault.is_some(),
			bounds == IndexBounds::Reject
		);
		match program.stages[0].fault {
			Some(fault) => {
				assert_eq!(fault.reason, FaultReason::IndexOutOfBounds);
				assert!(fault.guard_before_address);
			}
			None => assert_ne!(bounds, IndexBounds::Reject),
		}
		validate(&program)?;
	}
	Ok(())
}

#[test]
fn scatter_preserves_every_atomic_operation_and_ordering() -> TestResult {
	let operations = [
		AtomicOperation::Exchange,
		AtomicOperation::Add,
		AtomicOperation::Minimum,
		AtomicOperation::Maximum,
	];
	let orderings = [
		AtomicOrdering::Relaxed,
		AtomicOrdering::Acquire,
		AtomicOrdering::Release,
		AtomicOrdering::AcquireRelease,
		AtomicOrdering::SequentiallyConsistent,
	];
	for (operation_index, operation) in operations.into_iter().enumerate() {
		for (ordering_index, ordering) in orderings.into_iter().enumerate() {
			let tensors = [
				tensor(1, DType::F32, &[4, 9])?,
				tensor(2, DType::I32, &[3])?,
				tensor(3, DType::F32, &[4, 3])?,
				tensor(4, DType::F32, &[4, 9])?,
			];
			let scatter = kernel(
				60 + u64::try_from(operation_index * orderings.len() + ordering_index)?,
				&[1, 2, 3],
				&[4],
				PrimitiveKind::Scatter(Scatter {
					axis: 1,
					bounds: IndexBounds::Reject,
					conflict: ScatterConflict::Atomic {
						operation,
						ordering,
					},
				}),
			);
			let program = lower(&scatter, &index(&tensors))?;
			let Some(stage) = program.stages.last() else {
				return Err(Box::new(TestFailure(String::from(
					"scatter emitted no stages",
				))));
			};
			assert!(stage.atomics.iter().any(|atomic| {
				atomic.domain == recipe_primitives::AtomicAddressDomain::TensorElements
					&& atomic.operation == OwnedAtomicOperation::from(operation)
			}));
			assert!(stage.fault.is_some());
			validate(&program)?;
		}
	}

	let tensors = [
		tensor(1, DType::I32, &[9])?,
		tensor(2, DType::I32, &[3])?,
		tensor(3, DType::I32, &[3])?,
		tensor(4, DType::I32, &[9])?,
	];
	let unique = kernel(
		81,
		&[1, 2, 3],
		&[4],
		PrimitiveKind::Scatter(Scatter {
			axis: 0,
			bounds: IndexBounds::Wrap,
			conflict: ScatterConflict::UniqueIndices,
		}),
	);
	let program = lower(&unique, &index(&tensors))?;
	let Some(stage) = program.stages.last() else {
		return Err(Box::new(TestFailure(String::from(
			"unique scatter emitted no stages",
		))));
	};
	assert!(stage.atomics.is_empty());
	Ok(())
}

#[test]
fn histogram_has_fixed_clear_atomic_and_checked_bin_stages() -> TestResult {
	for (weighted, input_dtype) in [(false, DType::I32), (true, DType::F32)] {
		let mut tensors = vec![
			tensor(1, input_dtype, &[33])?,
			tensor(
				3,
				match weighted {
					true => DType::F32,
					false => DType::I32,
				},
				&[7],
			)?,
		];
		let inputs: &[u64] = match weighted {
			true => {
				tensors.push(tensor(2, DType::F32, &[33])?);
				&[1, 2]
			}
			false => &[1],
		};
		let histogram = kernel(
			90 + u64::from(weighted),
			inputs,
			&[3],
			PrimitiveKind::Histogram(Histogram {
				bins: 7,
				weighted,
				ordering: AtomicOrdering::SequentiallyConsistent,
			}),
		);
		let program = lower(&histogram, &index(&tensors))?;
		assert!(matches!(program.stages[0].kind, StageKind::HistogramClear));
		let StageKind::HistogramAccumulate { bin_mapping, .. } = program.stages[1].kind else {
			panic!("expected histogram accumulate");
		};
		assert_eq!(
			bin_mapping,
			match input_dtype == DType::I32 {
				true => HistogramBinMapping::I32Direct,
				false => HistogramBinMapping::F32TruncateTowardZero,
			}
		);
		let Some(fault) = program.stages[1].fault else {
			return Err(Box::new(TestFailure(String::from(
				"histogram omitted its checked fault path",
			))));
		};
		assert_eq!(fault.reason, FaultReason::HistogramBinOutOfBounds);
		validate(&program)?;
	}
	Ok(())
}

#[test]
fn sort_is_always_stable_total_ordered_and_fixed_network() -> TestResult {
	for (case, direction, stable, emit_indices) in [
		(0_u64, SortDirection::Ascending, true, false),
		(1, SortDirection::Descending, false, true),
	] {
		let tensors = [
			tensor(1, DType::F32, &[3, 7])?,
			tensor(2, DType::F32, &[3, 7])?,
			tensor(3, DType::I32, &[3, 7])?,
		];
		let outputs: &[u64] = match emit_indices {
			true => &[2, 3],
			false => &[2],
		};
		let sort = kernel(
			100 + case,
			&[1],
			outputs,
			PrimitiveKind::Sort(Sort {
				axis: 1,
				direction,
				stable,
				emit_indices,
			}),
		);
		let program = lower(&sort, &index(&tensors))?;
		let compare_stages = program
			.stages
			.iter()
			.filter(|stage| matches!(stage.kind, StageKind::StableSortCompareExchange(_)))
			.count();
		assert_eq!(compare_stages, 6);
		assert_eq!(
			program
				.buffers
				.iter()
				.filter(|buffer| buffer.lifetime == BufferLifetime::ProgramScratch)
				.count(),
			2
		);
		validate(&program)?;
	}
	Ok(())
}

#[test]
fn every_random_distribution_fixes_philox10_and_counter_semantics() -> TestResult {
	for (case, distribution, dtype) in [
		(0_u64, RandomDistribution::UniformF32, DType::F32),
		(1, RandomDistribution::NormalF32, DType::F32),
		(
			2,
			RandomDistribution::BernoulliI32 {
				probability_bits: 0.25_f32.to_bits(),
			},
			DType::I32,
		),
		(
			3,
			RandomDistribution::UniformI32 {
				low: -7,
				high_exclusive: 19,
			},
			DType::I32,
		),
	] {
		let tensors = [tensor(1, dtype, &[1024])?];
		let random = kernel(
			110 + case,
			&[],
			&[1],
			PrimitiveKind::Random(RandomMap {
				distribution,
				key: RandomKey {
					seed_low: 1,
					seed_high: 2,
					stream: 3,
				},
				philox_rounds: 10,
			}),
		);
		let program = lower(&random, &index(&tensors))?;
		let StageKind::Philox4x32_10(contract) = program.stages[0].kind else {
			panic!("expected Philox stage");
		};
		assert_eq!(contract.rounds, 10);
		assert!(contract.fold_kernel_id_into_key);
		assert_eq!(program.resources.total_flops.get(), 40_960);
		validate(&program)?;
	}
	Ok(())
}

#[test]
fn canonical_digest_is_deterministic_sensitive_and_tamper_evident() -> TestResult {
	let tensors = [tensor(1, DType::I32, &[17])?, tensor(2, DType::I32, &[17])?];
	let ascending = kernel(
		120,
		&[1],
		&[2],
		PrimitiveKind::Sort(Sort {
			axis: 0,
			direction: SortDirection::Ascending,
			stable: true,
			emit_indices: false,
		}),
	);
	let descending = kernel(
		120,
		&[1],
		&[2],
		PrimitiveKind::Sort(Sort {
			axis: 0,
			direction: SortDirection::Descending,
			stable: true,
			emit_indices: false,
		}),
	);
	let first = lower(&ascending, &index(&tensors))?;
	let second = lower(&ascending, &index(&tensors))?;
	let changed = lower(&descending, &index(&tensors))?;
	assert_eq!(first.digest, second.digest);
	assert_ne!(first.digest, changed.digest);

	let mut tampered = first;
	tampered.digest = ProgramDigest::ZERO;
	let errors = match tampered.validate() {
		Ok(()) => {
			return Err(Box::new(TestFailure(String::from(
				"tampered digest unexpectedly validated",
			))));
		}
		Err(errors) => errors,
	};
	assert!(errors.iter().any(|error| error.path == "digest"));
	Ok(())
}
