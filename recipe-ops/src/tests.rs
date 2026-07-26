use std::collections::BTreeMap;

use recipe_core::{AliasPermission, DType, KernelTemplateId, ScalarLiteral, ValueId};
use recipe_language::{
	AtomicOperation, AtomicOrdering, AxisSet, Contraction, Gather, IndexBounds, IndexMap, PrimitiveAliasRule,
	PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce, Scan, ScanMode, Scatter,
	ScatterConflict, Shape, Sort, Tensor,
};
use recipe_primitives::{INDEX_MAP_INTEGER_OPERATIONS_PER_LANE, StageKind};

use super::*;

#[test]
fn inventory_matches_every_normative_line_in_order() {
	let expected = include_str!("../../operation-surface.txt")
		.lines()
		.enumerate()
		.filter_map(|(index, raw)| {
			let line = raw.trim_end();
			(!line.is_empty() && !line.starts_with('#')).then(|| {
				let fields = line.split('\t').collect::<Vec<_>>();
				assert_eq!(fields.len(), 2, "surface line {} is malformed", index + 1);
				(index + 1, fields[0], fields[1])
			})
		})
		.collect::<Vec<_>>();
	let registry = operation_registry();
	let actual = registry.surface_iter().collect::<Vec<_>>();

	assert_eq!(expected.len(), 421);
	assert_eq!(registry.surface_len(), expected.len());
	assert_eq!(actual.len(), expected.len());
	for (ordinal, (descriptor, (line, symbol, source))) in actual.iter().zip(expected).enumerate() {
		assert_eq!(descriptor.id.ordinal(), ordinal);
		assert_eq!(descriptor.id.surface_line(), line);
		assert_eq!(descriptor.symbol, symbol);
		assert_eq!(descriptor.source, source);
		assert!(!descriptor.definition.is_empty());
	}
}

#[test]
fn recipe_owned_extensions_follow_the_surface_and_resolve_canonically() {
	let registry = operation_registry();
	let owned = registry.owned_iter().collect::<Vec<_>>();
	assert_eq!(registry.surface_len(), 421);
	assert_eq!(registry.owned_len(), 2);
	assert_eq!(registry.len(), 423);
	assert_eq!(owned.len(), registry.owned_len());
	assert_eq!(
		owned.iter()
			.map(|descriptor| descriptor.symbol)
			.collect::<Vec<_>>(),
		["recipe_max_pool_1d", "recipe_max_pool_1d_backward"]
	);
	for (owned_index, descriptor) in owned.into_iter().enumerate() {
		assert!(descriptor.id.is_recipe_owned());
		assert_eq!(descriptor.id.surface_line(), 0);
		assert_eq!(
			descriptor.id.ordinal(),
			registry.surface_len() + owned_index
		);
		assert_eq!(descriptor.family, OperationFamily::Pooling);
		assert!(matches!(
			descriptor.lowering,
			LoweringAvailability::Composition(_)
		));
		assert_eq!(
			registry.named(descriptor.symbol).collect::<Vec<_>>(),
			[descriptor]
		);
		assert_eq!(
			registry.resolve_unique(descriptor.symbol).unwrap(),
			descriptor
		);
		assert_eq!(
			registry
				.resolve_exact(descriptor.symbol, descriptor.source)
				.unwrap(),
			descriptor
		);
	}
	assert_eq!(registry.iter().count(), registry.len());
	assert!(
		registry
			.surface_iter()
			.all(|descriptor| !descriptor.id.is_recipe_owned())
	);
	assert_eq!(
		channelwise_max_pool_1d_descriptor(),
		registry.resolve_unique("recipe_max_pool_1d").unwrap()
	);
	assert_eq!(
		channelwise_max_pool_1d_backward_descriptor(),
		registry
			.resolve_unique("recipe_max_pool_1d_backward")
			.unwrap()
	);
}

#[test]
fn duplicate_symbols_are_preserved_and_never_resolved_implicitly() {
	let registry = operation_registry();
	for (symbol, expected) in [
		("predict", 3),
		("predict_proba", 2),
		("train", 3),
		("train_multiclass", 2),
	] {
		let descriptors = registry.named(symbol).collect::<Vec<_>>();
		assert_eq!(descriptors.len(), expected);
		assert!(descriptors.iter().all(|descriptor| {
			usize::from(descriptor.id.occurrences()) == expected && descriptor.id.is_duplicate_symbol()
		}));
		assert_eq!(
			descriptors
				.iter()
				.map(|descriptor| descriptor.id.occurrence())
				.collect::<Vec<_>>(),
			(1..=u16::try_from(expected).unwrap()).collect::<Vec<_>>()
		);
		assert_eq!(
			registry.resolve_unique(symbol).unwrap_err().kind,
			OperationErrorKind::AmbiguousSymbol
		);
		for descriptor in descriptors {
			assert_eq!(
				registry
					.resolve_exact(descriptor.symbol, descriptor.source)
					.unwrap(),
				descriptor
			);
		}
	}
	assert_eq!(
		registry
			.resolve_unique("not_an_operation")
			.unwrap_err()
			.kind,
		OperationErrorKind::UnknownOperation
	);
}

#[test]
fn canonical_dtype_contracts_do_not_reintroduce_legacy_calculation_types() {
	let registry = operation_registry();
	assert_eq!(
		registry.resolve_unique("gpu_eq").unwrap().dtypes,
		CanonicalDTypeContract::Exact {
			inputs: &[DType::F32, DType::F32],
			outputs: &[DType::I32],
		}
	);
	assert_eq!(
		registry.resolve_unique("gpu_argmax_f32").unwrap().dtypes,
		CanonicalDTypeContract::Exact {
			inputs: &[DType::F32],
			outputs: &[DType::I32],
		}
	);
	for (symbol, legacy) in [
		("gpu_add_f16", LegacyDType::F16),
		("gpu_rmsnorm_f64", LegacyDType::F64),
		("gpu_bernoulli_u8", LegacyDType::U8),
		("gpu_dgemv_into", LegacyDType::F64),
	] {
		let descriptor = registry.resolve_unique(symbol).unwrap();
		assert_eq!(descriptor.legacy_dtype, Some(legacy));
		assert!(!matches!(
			descriptor.lowering,
			LoweringAvailability::Unsupported(_)
		));
		assert!(!matches!(
			descriptor.dtypes,
			CanonicalDTypeContract::LegacyExcluded { .. } | CanonicalDTypeContract::NonNumericHostData
		));
	}
	for descriptor in registry.iter().filter(|entry| {
		matches!(
			entry.lowering,
			LoweringAvailability::Scalar(_)
				| LoweringAvailability::Primitive(_)
				| LoweringAvailability::Composition(_)
		)
	}) {
		assert!(
			!matches!(
				descriptor.dtypes,
				CanonicalDTypeContract::LegacyExcluded { .. } | CanonicalDTypeContract::NonNumericHostData
			),
			"{} reintroduced a noncanonical calculation dtype",
			descriptor.symbol
		);
	}
	assert_eq!(
		registry.resolve_unique("dequant_f32").unwrap().dtypes,
		CanonicalDTypeContract::F32OrI32Payload
	);
	assert_eq!(
		registry.resolve_unique("gpu_relu_into").unwrap().family,
		OperationFamily::Activation
	);
	assert_eq!(
		registry.resolve_unique("gpu_mae_grad").unwrap().family,
		OperationFamily::Loss
	);
}

#[test]
fn every_scalar_recipe_lowers_to_the_same_valid_program_repeatedly() {
	let descriptors = operation_registry()
		.iter()
		.filter(|descriptor| matches!(descriptor.lowering, LoweringAvailability::Scalar(_)))
		.collect::<Vec<_>>();
	assert!(
		descriptors.len() >= 60,
		"only {} scalar operations were classified",
		descriptors.len()
	);
	for descriptor in descriptors {
		let first = lower_scalar(descriptor)
			.unwrap_or_else(|error| panic!("{} failed scalar lowering: {error}", descriptor.symbol));
		let second = lower_scalar(descriptor).unwrap();
		assert_eq!(first, second, "{} was nondeterministic", descriptor.symbol);
		first.validate().unwrap();
		let expected = match descriptor.dtypes {
			CanonicalDTypeContract::Exact { inputs, outputs } => (inputs, outputs),
			other => panic!(
				"{} scalar recipe has non-exact dtype contract {other:?}",
				descriptor.symbol
			),
		};
		assert_eq!(
			first.inputs
				.iter()
				.map(|input| input.dtype)
				.collect::<Vec<_>>(),
			expected.0
		);
		assert_eq!(
			first.outputs
				.iter()
				.map(|output| first.dtype_of(*output).unwrap())
				.collect::<Vec<_>>(),
			expected.1
		);
	}
}

#[test]
fn scalar_argument_order_and_in_place_aliases_match_the_public_entrypoints() {
	let registry = operation_registry();
	let subtraction = registry.resolve_unique("gpu_sub_inplace").unwrap();
	let subtraction_program = lower_scalar(subtraction).unwrap();
	assert_eq!(
		subtraction_program.instructions[0].operands,
		[
			subtraction_program.inputs[1].id,
			subtraction_program.inputs[0].id
		]
	);
	assert_eq!(
		subtraction.alias,
		AliasContract::OutputMustAliasInput { input: 1 }
	);

	let clip = registry.resolve_unique("gpu_clip_value").unwrap();
	let clip_program = lower_scalar(clip).unwrap();
	assert_eq!(
		clip_program.instructions[0].operands,
		[clip_program.inputs[2].id, clip_program.inputs[0].id]
	);
	assert_eq!(
		clip_program.instructions[1].operands[1],
		clip_program.inputs[1].id
	);
	assert_eq!(clip.alias, AliasContract::OutputMustAliasInput { input: 2 });

	let divergence = lower_scalar(registry.resolve_unique("gpu_kl_div").unwrap()).unwrap();
	assert_eq!(divergence.inputs.len(), 2);
	assert!(matches!(
		registry.resolve_unique("gpu_add_col").unwrap().lowering,
		LoweringAvailability::Composition(_)
	));
}

#[test]
fn every_primitive_recipe_performs_real_deterministic_lowering() {
	let descriptors = operation_registry()
		.iter()
		.filter(|descriptor| matches!(descriptor.lowering, LoweringAvailability::Primitive(_)))
		.collect::<Vec<_>>();
	assert!(
		descriptors.len() >= 25,
		"only {} primitive operations were classified",
		descriptors.len()
	);
	for descriptor in descriptors {
		let recipe = match descriptor.lowering {
			LoweringAvailability::Primitive(recipe) => recipe,
			LoweringAvailability::Scalar(_)
			| LoweringAvailability::Composition(_)
			| LoweringAvailability::Workspace(_)
			| LoweringAvailability::NonCalculation(_)
			| LoweringAvailability::Unsupported(_) => {
				unreachable!()
			}
		};
		let case = primitive_case(recipe, descriptor.id.ordinal());
		let tensors = case
			.tensors
			.iter()
			.map(|tensor| (tensor.id, tensor))
			.collect::<BTreeMap<_, _>>();
		let request = PrimitiveRequest {
			kernel: &case.kernel,
			tensors: &tensors,
		};
		let first = lower_primitive(descriptor, request)
			.unwrap_or_else(|error| panic!("{} failed primitive lowering: {error}", descriptor.symbol));
		let second = lower_primitive(descriptor, request).unwrap();
		assert_eq!(first, second, "{} was nondeterministic", descriptor.symbol);
		first.validate().unwrap();
	}
}

#[test]
fn index_map_lowers_zero_inputs_into_the_exact_int32_output_shape_and_contract() {
	let case = primitive_case(PrimitiveRecipe::IndexMap, 10_000);
	let tensors = case
		.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	let request = PrimitiveRequest {
		kernel: &case.kernel,
		tensors: &tensors,
	};

	assert_eq!(
		PrimitiveRecipe::IndexMap.family(),
		PrimitiveFamily::IndexMap
	);
	assert_eq!(
		PrimitiveRecipe::IndexMap.dtype_contract(),
		CanonicalDTypeContract::Exact {
			inputs: &[],
			outputs: &[DType::I32],
		}
	);
	let first = lower_index_map(request).unwrap();
	let second = lower_index_map(request).unwrap();
	assert_eq!(first, second);
	first.validate().unwrap();
	assert_eq!(first.source_input_count, 0);
	assert_eq!(first.source_output_count, 1);
	assert!(first.source_aliases.is_empty());

	let output = first
		.buffers
		.iter()
		.find(|buffer| {
			matches!(
				buffer.origin,
				recipe_primitives::BufferOrigin::Tensor(value) if value == ValueId::new(1)
			)
		})
		.unwrap();
	assert_eq!(output.dtype, DType::I32);
	assert_eq!(output.shape, [2, 3]);
	assert_eq!(output.access.logical_extents, [2, 3]);

	assert_eq!(first.stages.len(), 1);
	let stage = &first.stages[0];
	assert_eq!(stage.geometry.logical_lanes, 6);
	assert_eq!(
		stage.kind,
		StageKind::IndexMap(IndexMap {
			start: -3,
			element_step: 2,
			iteration_step: 64,
			modulus: Some(101),
		})
	);
	assert_eq!(
		stage.resources.integer_operations,
		6 * INDEX_MAP_INTEGER_OPERATIONS_PER_LANE
	);
	assert_eq!(first.resources.total_flops.get(), 0);
	assert_eq!(
		first.resources.total_integer_operations,
		6 * INDEX_MAP_INTEGER_OPERATIONS_PER_LANE
	);
	let fault = stage
		.fault
		.expect("checked affine arithmetic requires a fault path");
	assert_eq!(
		fault.reason,
		recipe_primitives::FaultReason::ArithmeticDomain
	);
	assert!(fault.guard_before_address);
}

#[test]
fn index_map_surface_rejects_inputs_non_int32_outputs_and_nonpositive_moduli() {
	let mut input_case = primitive_case(PrimitiveRecipe::IndexMap, 10_010);
	input_case
		.tensors
		.push(tensor(2, DType::I32, &[1], true, false));
	input_case.kernel.inputs.push(ValueId::new(2));
	input_case.kernel.alias_rules = forbidden_aliases(1, 1);
	assert_eq!(
		lower_index_map_case(&input_case).unwrap_err().kind,
		OperationErrorKind::PrimitiveLoweringFailed
	);

	let mut dtype_case = primitive_case(PrimitiveRecipe::IndexMap, 10_011);
	dtype_case.tensors[0] = tensor(1, DType::F32, &[2, 3], false, true);
	assert_eq!(
		lower_index_map_case(&dtype_case).unwrap_err().kind,
		OperationErrorKind::PrimitiveLoweringFailed
	);

	let mut modulus_case = primitive_case(PrimitiveRecipe::IndexMap, 10_012);
	let PrimitiveKind::IndexMap(spec) = &mut modulus_case.kernel.kind else {
		unreachable!();
	};
	spec.modulus = Some(0);
	assert_eq!(
		lower_index_map_case(&modulus_case).unwrap_err().kind,
		OperationErrorKind::PrimitiveLoweringFailed
	);
}

#[test]
fn direct_primitives_reject_semantically_different_valid_shapes_and_policies() {
	let mut wrong_gemm = contraction_case(KernelTemplateId::new(10_001), ContractionClass::Matrix);
	wrong_gemm.kernel.id = KernelTemplateId::new(10_002);
	assert_primitive_mismatch("gpu_gemm_at", &wrong_gemm);

	let mut reverse_scan = primitive_case(
		PrimitiveRecipe::Scan {
			operator: recipe_language::ReduceOperator::Sum,
			exclusive: false,
			axis: AxisRequirement::Last,
		},
		10_003,
	);
	let PrimitiveKind::Scan(spec) = &mut reverse_scan.kernel.kind else {
		unreachable!();
	};
	spec.reverse = true;
	assert_primitive_mismatch("gpu_cumsum_rows", &reverse_scan);

	let mut clamping_gather = gather_case(KernelTemplateId::new(10_004), AxisRequirement::First);
	let PrimitiveKind::Gather(spec) = &mut clamping_gather.kernel.kind else {
		unreachable!();
	};
	spec.bounds = IndexBounds::Clamp;
	assert_primitive_mismatch("gpu_gather_rows_into", &clamping_gather);

	let mut relaxed_scatter = scatter_case(KernelTemplateId::new(10_005), AxisRequirement::First);
	let PrimitiveKind::Scatter(spec) = &mut relaxed_scatter.kernel.kind else {
		unreachable!();
	};
	spec.conflict = ScatterConflict::Atomic {
		operation: AtomicOperation::Add,
		ordering: AtomicOrdering::Relaxed,
	};
	assert_primitive_mismatch("gpu_scatter_add", &relaxed_scatter);
}

#[test]
fn wrong_and_unsupported_lowering_requests_fail_closed() {
	let primitive = operation_registry().resolve_unique("gpu_sum_all").unwrap();
	let case = primitive_case(PrimitiveRecipe::Random(RandomRecipe::UniformF32), 999);
	let tensors = case
		.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	let request = PrimitiveRequest {
		kernel: &case.kernel,
		tensors: &tensors,
	};
	for descriptor in operation_registry().iter() {
		match descriptor.lowering {
			LoweringAvailability::Scalar(_) => assert_eq!(
				lower_primitive(descriptor, request).unwrap_err().kind,
				OperationErrorKind::WrongLoweringKind,
				"{} accepted primitive lowering",
				descriptor.symbol
			),
			LoweringAvailability::Primitive(_) => assert_eq!(
				lower_scalar(descriptor).unwrap_err().kind,
				OperationErrorKind::WrongLoweringKind,
				"{} accepted scalar lowering",
				descriptor.symbol
			),
			LoweringAvailability::Composition(_)
			| LoweringAvailability::Workspace(_)
			| LoweringAvailability::NonCalculation(_) => {
				assert_eq!(
					lower_scalar(descriptor).unwrap_err().kind,
					OperationErrorKind::WrongLoweringKind
				);
				assert_eq!(
					lower_primitive(descriptor, request).unwrap_err().kind,
					OperationErrorKind::WrongLoweringKind
				);
			}
			LoweringAvailability::Unsupported(_) => {
				assert_eq!(
					lower_scalar(descriptor).unwrap_err().kind,
					OperationErrorKind::UnsupportedLowering,
					"{} did not fail closed in scalar lowering",
					descriptor.symbol
				);
				assert_eq!(
					lower_primitive(descriptor, request).unwrap_err().kind,
					OperationErrorKind::UnsupportedLowering,
					"{} did not fail closed in primitive lowering",
					descriptor.symbol
				);
			}
		}
	}
	assert_eq!(
		lower_primitive(primitive, request).unwrap_err().kind,
		OperationErrorKind::PrimitiveRecipeMismatch
	);
}

#[test]
fn every_surface_row_has_one_explicit_lowering_state_and_no_residual_gap() {
	let mut scalar = 0_usize;
	let mut primitive = 0_usize;
	let mut composition = 0_usize;
	let mut workspace = 0_usize;
	let mut non_calculation = 0_usize;
	let mut unsupported = 0_usize;
	for descriptor in operation_registry().surface_iter() {
		match descriptor.lowering {
			LoweringAvailability::Scalar(_) => scalar += 1,
			LoweringAvailability::Primitive(_) => primitive += 1,
			LoweringAvailability::Composition(_) => composition += 1,
			LoweringAvailability::Workspace(_) => workspace += 1,
			LoweringAvailability::NonCalculation(_) => non_calculation += 1,
			LoweringAvailability::Unsupported(_) => unsupported += 1,
		}
	}
	assert_eq!(
		[
			scalar,
			primitive,
			composition,
			workspace,
			non_calculation,
			unsupported
		],
		[94, 29, 264, 24, 10, 0]
	);
	assert_eq!(
		scalar + primitive + composition + workspace + non_calculation + unsupported,
		operation_registry().surface_len()
	);
}

#[test]
fn every_structured_recipe_is_finite_specific_and_valid() {
	let descriptors = operation_registry()
		.iter()
		.filter(|descriptor| matches!(descriptor.lowering, LoweringAvailability::Composition(_)))
		.collect::<Vec<_>>();
	assert_eq!(descriptors.len(), 266);
	for descriptor in descriptors {
		let recipe = match descriptor.lowering {
			LoweringAvailability::Composition(recipe) => recipe,
			_ => unreachable!(),
		};
		validate_composition(descriptor)
			.unwrap_or_else(|error| panic!("{} has invalid composition: {error}", descriptor.symbol));
		assert!(!recipe.name().is_empty());
		assert!(!recipe.definition().contains("pending"));
		assert!(!recipe.definition().contains("generic"));
		assert!(!recipe.steps().is_empty());
	}
	for symbol in [
		"gpu_fft_c2c_1d",
		"gpu_cholesky",
		"gpu_svd",
		"gpu_union_find_cc",
		"gpu_viterbi",
	] {
		let descriptor = operation_registry().resolve_unique(symbol).unwrap();
		let recipe = match descriptor.lowering {
			LoweringAvailability::Composition(recipe) => recipe,
			other => panic!("{symbol} has unexpected lowering {other:?}"),
		};
		assert!(
			composition_contains_repeat(recipe.steps()),
			"{symbol} omitted its static iteration structure"
		);
	}
}

#[test]
fn structured_losses_and_metrics_preserve_output_granularity() {
	for (symbol, expected) in [
		(
			"gpu_accuracy",
			&[
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
			][..],
		),
		(
			"gpu_argmax_accuracy_into",
			&[
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
			],
		),
		(
			"gpu_accuracy_into",
			&[
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
			],
		),
		(
			"gpu_cross_entropy",
			&[
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Gather,
				PrimitiveFamily::Elementwise,
			],
		),
		("gpu_bce_with_logits", &[PrimitiveFamily::Elementwise]),
		("gpu_focal_into", &[PrimitiveFamily::Elementwise]),
		("gpu_hinge_loss", &[PrimitiveFamily::Elementwise]),
		("gpu_kl_div_loss", &[PrimitiveFamily::Elementwise]),
		(
			"gpu_mse_into",
			&[
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
			],
		),
		(
			"gpu_ss_res_into",
			&[PrimitiveFamily::Elementwise, PrimitiveFamily::Reduce],
		),
		(
			"gpu_contrastive_loss",
			&[
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
			],
		),
		(
			"gpu_cosine_embedding_loss",
			&[
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
			],
		),
		(
			"gpu_triplet_loss",
			&[
				PrimitiveFamily::Elementwise,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Reduce,
				PrimitiveFamily::Elementwise,
			],
		),
	] {
		let descriptor = operation_registry().resolve_unique(symbol).unwrap();
		let recipe = match descriptor.lowering {
			LoweringAvailability::Composition(recipe) => recipe,
			other => panic!("{symbol} has unexpected lowering {other:?}"),
		};
		assert_eq!(
			composition_primitive_families(recipe.steps()),
			expected,
			"{symbol} changed calculation or output granularity"
		);
	}
}

#[test]
fn workspace_queries_use_owned_static_formulas() {
	let registry = operation_registry();
	let workspace = registry
		.iter()
		.filter(|descriptor| matches!(descriptor.lowering, LoweringAvailability::Workspace(_)))
		.collect::<Vec<_>>();
	assert_eq!(workspace.len(), 24);
	for descriptor in workspace {
		assert!(!descriptor.definition.contains("vendor"));
	}

	let evaluate = |symbol: &str, dimensions: &[u64]| {
		evaluate_workspace(registry.resolve_unique(symbol).unwrap(), dimensions).unwrap()
	};
	assert_eq!(
		evaluate("gpu_sum_all_workspace_bytes", &[4096]),
		WorkspaceValue {
			amount: 256,
			unit: WorkspaceUnit::Bytes,
		}
	);
	assert_eq!(
		evaluate("gpu_cumprod_workspace_bytes", &[4096]),
		WorkspaceValue {
			amount: 512,
			unit: WorkspaceUnit::Bytes,
		}
	);
	assert_eq!(
		evaluate("gpu_random_permutation_workspace_bytes", &[1000]),
		WorkspaceValue {
			amount: 12_192,
			unit: WorkspaceUnit::Bytes,
		}
	);
	assert_eq!(
		evaluate("gpu_dot_workspace_bytes", &[4096]),
		WorkspaceValue {
			amount: 0,
			unit: WorkspaceUnit::Bytes,
		}
	);
	assert_eq!(
		evaluate("gpu_l2_norm_workspace_bytes", &[4096]),
		WorkspaceValue {
			amount: 16_640,
			unit: WorkspaceUnit::Bytes,
		}
	);
	assert_eq!(
		evaluate("gpu_count_distinct_workspace_bytes", &[1000]),
		WorkspaceValue {
			amount: 12_320,
			unit: WorkspaceUnit::Bytes,
		}
	);
	assert_eq!(
		evaluate("gpu_cholesky_workspace_bytes", &[100]),
		WorkspaceValue {
			amount: 12_804,
			unit: WorkspaceUnit::Bytes,
		}
	);
	assert_eq!(
		evaluate("gpu_splitk_dw_partials_elems", &[1024, 16, 8]),
		WorkspaceValue {
			amount: 512,
			unit: WorkspaceUnit::F32Elements,
		}
	);
	assert_eq!(
		evaluate_workspace(
			registry.resolve_unique("gpu_qr_workspace_bytes").unwrap(),
			&[5],
		)
		.unwrap_err()
		.kind,
		OperationErrorKind::WorkspaceFormulaMismatch
	);
	assert_eq!(
		evaluate_workspace(
			registry.resolve_unique("gpu_svd_workspace_bytes").unwrap(),
			&[u64::MAX, u64::MAX],
		)
		.unwrap_err()
		.kind,
		OperationErrorKind::WorkspaceArithmeticOverflow
	);
}

#[test]
fn non_calculation_entries_are_exactly_host_metadata_or_lifecycle() {
	let actual = operation_registry()
		.iter()
		.filter_map(|descriptor| {
			matches!(descriptor.lowering, LoweringAvailability::NonCalculation(_)).then_some(descriptor.symbol)
		})
		.collect::<Vec<_>>();
	assert_eq!(
		actual,
		[
			"Data",
			"Model",
			"Train",
			"Infer",
			"encode",
			"gpu_blas_workspace",
			"gpu_shutdown",
			"parse_safetensors",
			"render_chat",
			"render_template",
		]
	);
	for descriptor in operation_registry()
		.iter()
		.filter(|descriptor| matches!(descriptor.lowering, LoweringAvailability::NonCalculation(_)))
	{
		assert_eq!(
			descriptor.dtypes,
			CanonicalDTypeContract::NonNumericHostData
		);
	}
}

fn composition_contains_repeat(steps: &[CompositionStep]) -> bool {
	steps.iter().any(|step| match step {
		CompositionStep::Primitive { .. } => false,
		CompositionStep::Repeat { .. } => true,
	})
}

fn composition_primitive_families(steps: &[CompositionStep]) -> Vec<PrimitiveFamily> {
	steps.iter()
		.map(|step| match step {
			CompositionStep::Primitive { family, .. } => *family,
			CompositionStep::Repeat { .. } => panic!("expected an unrolled top-level recipe"),
		})
		.collect()
}

fn assert_primitive_mismatch(symbol: &str, case: &PrimitiveCase) {
	let tensors = tensor_index(case);
	let request = PrimitiveRequest {
		kernel: &case.kernel,
		tensors: &tensors,
	};
	assert_eq!(
		lower_primitive(
			operation_registry().resolve_unique(symbol).unwrap(),
			request,
		)
		.unwrap_err()
		.kind,
		OperationErrorKind::PrimitiveRecipeMismatch,
		"{symbol} accepted a primitive with different semantics"
	);
}

fn tensor_index(case: &PrimitiveCase) -> BTreeMap<ValueId, &Tensor> {
	case.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect()
}

fn lower_index_map_case(case: &PrimitiveCase) -> OperationResult<LoweredProgram> {
	let tensors = tensor_index(case);
	lower_index_map(PrimitiveRequest {
		kernel: &case.kernel,
		tensors: &tensors,
	})
}

#[derive(Debug)]
struct PrimitiveCase {
	kernel: PrimitiveKernel,
	tensors: Vec<Tensor>,
}

fn primitive_case(recipe: PrimitiveRecipe, ordinal: usize) -> PrimitiveCase {
	let kernel_id = KernelTemplateId::new(u64::try_from(ordinal).unwrap().saturating_add(1));
	match recipe {
		PrimitiveRecipe::Reduce {
			operator,
			result,
			axes,
		} => {
			let (shape, axes) = reduction_shape(axes);
			let output_shape = shape.reduced(&axes, false).unwrap();
			let input = tensor(1, DType::F32, shape.extents(), true, false);
			let mut tensors = vec![input];
			let mut outputs = Vec::new();
			match result {
				recipe_language::ReduceResult::Value => {
					tensors.push(tensor(2, DType::F32, output_shape.extents(), false, true));
					outputs.push(ValueId::new(2));
				}
				recipe_language::ReduceResult::Index => {
					tensors.push(tensor(2, DType::I32, output_shape.extents(), false, true));
					outputs.push(ValueId::new(2));
				}
				recipe_language::ReduceResult::ValueAndIndex => {
					tensors.push(tensor(2, DType::F32, output_shape.extents(), false, true));
					tensors.push(tensor(3, DType::I32, output_shape.extents(), false, true));
					outputs.extend([ValueId::new(2), ValueId::new(3)]);
				}
			}
			let inputs = vec![ValueId::new(1)];
			let alias_rules = forbidden_aliases(inputs.len(), outputs.len());
			PrimitiveCase {
				kernel: PrimitiveKernel {
					id: kernel_id,
					inputs,
					outputs,
					alias_rules,
					kind: PrimitiveKind::Reduce(Reduce {
						operator,
						axes,
						keep_dimensions: false,
						result,
						tree_lanes: 64,
					}),
				},
				tensors,
			}
		}
		PrimitiveRecipe::Scan {
			operator,
			exclusive,
			axis,
		} => {
			let (shape, axis) = scan_shape(axis);
			let inputs = vec![ValueId::new(1)];
			let outputs = vec![ValueId::new(2)];
			let mode = match exclusive {
				true => ScanMode::Exclusive {
					identity: match operator {
						recipe_language::ReduceOperator::Product => {
							ScalarLiteral::F32Bits(1.0_f32.to_bits())
						}
						recipe_language::ReduceOperator::Sum
						| recipe_language::ReduceOperator::Minimum
						| recipe_language::ReduceOperator::Maximum
						| recipe_language::ReduceOperator::Any
						| recipe_language::ReduceOperator::All => ScalarLiteral::F32Bits(0.0_f32.to_bits()),
					},
				},
				false => ScanMode::Inclusive,
			};
			PrimitiveCase {
				kernel: PrimitiveKernel {
					id: kernel_id,
					inputs: inputs.clone(),
					outputs: outputs.clone(),
					alias_rules: forbidden_aliases(inputs.len(), outputs.len()),
					kind: PrimitiveKind::Scan(Scan {
						operator,
						axis,
						mode,
						reverse: false,
						tree_lanes: 64,
					}),
				},
				tensors: vec![
					tensor(1, DType::F32, shape.extents(), true, false),
					tensor(2, DType::F32, shape.extents(), false, true),
				],
			}
		}
		PrimitiveRecipe::Contraction(class) => contraction_case(kernel_id, class),
		PrimitiveRecipe::Gather { axis } => gather_case(kernel_id, axis),
		PrimitiveRecipe::ScatterAdd { axis } => scatter_case(kernel_id, axis),
		PrimitiveRecipe::Sort {
			direction,
			stable,
			axis,
		} => {
			let inputs = vec![ValueId::new(1)];
			let outputs = vec![ValueId::new(2)];
			PrimitiveCase {
				kernel: PrimitiveKernel {
					id: kernel_id,
					inputs: inputs.clone(),
					outputs: outputs.clone(),
					alias_rules: forbidden_aliases(inputs.len(), outputs.len()),
					kind: PrimitiveKind::Sort(Sort {
						axis: axis_for(axis, 1),
						direction,
						stable,
						emit_indices: false,
					}),
				},
				tensors: vec![
					tensor(1, DType::F32, &[8], true, false),
					tensor(2, DType::F32, &[8], false, true),
				],
			}
		}
		PrimitiveRecipe::IndexMap => PrimitiveCase {
			kernel: PrimitiveKernel {
				id: kernel_id,
				inputs: Vec::new(),
				outputs: vec![ValueId::new(1)],
				alias_rules: Vec::new(),
				kind: PrimitiveKind::IndexMap(IndexMap {
					start: -3,
					element_step: 2,
					iteration_step: 64,
					modulus: Some(101),
				}),
			},
			tensors: vec![tensor(1, DType::I32, &[2, 3], false, true)],
		},
		PrimitiveRecipe::Random(distribution) => {
			let distribution = match distribution {
				RandomRecipe::UniformF32 => RandomDistribution::UniformF32,
				RandomRecipe::NormalF32 => RandomDistribution::NormalF32,
			};
			PrimitiveCase {
				kernel: PrimitiveKernel {
					id: kernel_id,
					inputs: Vec::new(),
					outputs: vec![ValueId::new(1)],
					alias_rules: Vec::new(),
					kind: PrimitiveKind::Random(RandomMap {
						distribution,
						key: RandomKey {
							seed_low: 11,
							seed_high: 29,
							stream: 7,
						},
						philox_rounds: 10,
					}),
				},
				tensors: vec![tensor(1, DType::F32, &[16], false, true)],
			}
		}
	}
}

fn contraction_case(kernel_id: KernelTemplateId, class: ContractionClass) -> PrimitiveCase {
	let (left_shape, right_shape, output_shape, batch_axes, contract_axes) = match class {
		ContractionClass::Vector => (vec![4], vec![4], vec![1], Vec::new(), vec![(0, 0)]),
		ContractionClass::Matrix => (vec![2, 3], vec![3, 4], vec![2, 4], Vec::new(), vec![(1, 0)]),
		ContractionClass::LeftTransposedMatrix => (vec![3, 2], vec![3, 4], vec![2, 4], Vec::new(), vec![(0, 0)]),
		ContractionClass::RightTransposedMatrix => (vec![2, 3], vec![4, 3], vec![2, 4], Vec::new(), vec![(1, 1)]),
		ContractionClass::Batched => (
			vec![2, 3, 4],
			vec![2, 4, 5],
			vec![2, 3, 5],
			vec![(0, 0)],
			vec![(2, 1)],
		),
	};
	let inputs = vec![ValueId::new(1), ValueId::new(2)];
	let outputs = vec![ValueId::new(3)];
	PrimitiveCase {
		kernel: PrimitiveKernel {
			id: kernel_id,
			inputs: inputs.clone(),
			outputs: outputs.clone(),
			alias_rules: forbidden_aliases(inputs.len(), outputs.len()),
			kind: PrimitiveKind::Contraction(Contraction {
				batch_axes,
				contract_axes,
			}),
		},
		tensors: vec![
			tensor(1, DType::F32, &left_shape, true, false),
			tensor(2, DType::F32, &right_shape, true, false),
			tensor(3, DType::F32, &output_shape, false, true),
		],
	}
}

fn gather_case(kernel_id: KernelTemplateId, requirement: AxisRequirement) -> PrimitiveCase {
	let axis = axis_for(requirement, 2);
	let source_shape = Shape::new(vec![4, 3]).unwrap();
	let indices_shape = Shape::new(vec![2]).unwrap();
	let output_shape = source_shape.gather_result(axis, &indices_shape).unwrap();
	let inputs = vec![ValueId::new(1), ValueId::new(2)];
	let outputs = vec![ValueId::new(3)];
	PrimitiveCase {
		kernel: PrimitiveKernel {
			id: kernel_id,
			inputs: inputs.clone(),
			outputs: outputs.clone(),
			alias_rules: forbidden_aliases(inputs.len(), outputs.len()),
			kind: PrimitiveKind::Gather(Gather {
				axis,
				bounds: IndexBounds::Reject,
			}),
		},
		tensors: vec![
			tensor(1, DType::F32, source_shape.extents(), true, false),
			tensor(2, DType::I32, indices_shape.extents(), true, false),
			tensor(3, DType::F32, output_shape.extents(), false, true),
		],
	}
}

fn scatter_case(kernel_id: KernelTemplateId, requirement: AxisRequirement) -> PrimitiveCase {
	let axis = axis_for(requirement, 2);
	let target_shape = Shape::new(vec![4, 3]).unwrap();
	let indices_shape = Shape::new(vec![2]).unwrap();
	let updates_shape = target_shape.gather_result(axis, &indices_shape).unwrap();
	let inputs = vec![ValueId::new(1), ValueId::new(2), ValueId::new(3)];
	let outputs = vec![ValueId::new(1)];
	PrimitiveCase {
		kernel: PrimitiveKernel {
			id: kernel_id,
			inputs: inputs.clone(),
			outputs: outputs.clone(),
			alias_rules: vec![
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
				PrimitiveAliasRule {
					input: 2,
					output: 0,
					permission: AliasPermission::Forbidden,
				},
			],
			kind: PrimitiveKind::Scatter(Scatter {
				axis,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::Atomic {
					operation: AtomicOperation::Add,
					ordering: AtomicOrdering::SequentiallyConsistent,
				},
			}),
		},
		tensors: vec![
			tensor(1, DType::F32, target_shape.extents(), true, true),
			tensor(2, DType::I32, indices_shape.extents(), true, false),
			tensor(3, DType::F32, updates_shape.extents(), true, false),
		],
	}
}

fn reduction_shape(requirement: AxisRequirement) -> (Shape, AxisSet) {
	match requirement {
		AxisRequirement::Any => (Shape::new(vec![4]).unwrap(), AxisSet::new(vec![0]).unwrap()),
		AxisRequirement::First => (
			Shape::new(vec![2, 3]).unwrap(),
			AxisSet::new(vec![0]).unwrap(),
		),
		AxisRequirement::Last => (
			Shape::new(vec![2, 3]).unwrap(),
			AxisSet::new(vec![1]).unwrap(),
		),
		AxisRequirement::All => (
			Shape::new(vec![2, 3]).unwrap(),
			AxisSet::new(vec![0, 1]).unwrap(),
		),
	}
}

fn scan_shape(requirement: AxisRequirement) -> (Shape, usize) {
	match requirement {
		AxisRequirement::Any | AxisRequirement::All => (Shape::new(vec![4]).unwrap(), 0),
		AxisRequirement::First => (Shape::new(vec![2, 3]).unwrap(), 0),
		AxisRequirement::Last => (Shape::new(vec![2, 3]).unwrap(), 1),
	}
}

fn axis_for(requirement: AxisRequirement, rank: usize) -> usize {
	match requirement {
		AxisRequirement::Any | AxisRequirement::First => 0,
		AxisRequirement::Last => rank.saturating_sub(1),
		AxisRequirement::All => {
			assert_eq!(rank, 1);
			0
		}
	}
}

fn forbidden_aliases(input_count: usize, output_count: usize) -> Vec<PrimitiveAliasRule> {
	(0..input_count)
		.flat_map(|input| {
			(0..output_count).map(move |output| PrimitiveAliasRule {
				input,
				output,
				permission: AliasPermission::Forbidden,
			})
		})
		.collect()
}

fn tensor(id: u64, dtype: DType, extents: &[u64], external_input: bool, external_output: bool) -> Tensor {
	Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(extents.to_vec()).unwrap(),
		external_input,
		external_output,
	)
	.unwrap()
}
