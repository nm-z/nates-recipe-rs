use recipe_core::{
	AliasPermission, DType, KernelTemplateId, ScalarConstant, ScalarInput, ScalarInstruction, ScalarLiteral,
	ScalarOpcode, ScalarProgram, ScalarValueId, ValueId,
};
use recipe_language::{
	AtomicOperation, AtomicOrdering, AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather,
	Histogram, IndexBounds, IndexMap, OgdlCodecError, OgdlDocumentErrorKind, PrimitiveAliasRule, PrimitiveKernel,
	PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce, ReduceOperator, ReduceResult, Scan, ScanMode,
	Scatter, ScatterConflict, Shape, Sort, SortDirection, Tensor,
};

fn make_graph(
	kind: PrimitiveKind,
	inputs: Vec<(DType, Vec<u64>)>,
	outputs: Vec<(DType, Vec<u64>)>,
) -> CalculationGraph {
	let input_count = inputs.len();
	let output_count = outputs.len();
	let mut tensors = Vec::with_capacity(input_count + output_count);
	let mut input_ids = Vec::with_capacity(input_count);
	let mut output_ids = Vec::with_capacity(output_count);
	for (index, (dtype, extents)) in inputs.into_iter().enumerate() {
		let id = ValueId::new(index as u64 + 1);
		input_ids.push(id);
		tensors.push(Tensor::contiguous(id, dtype, Shape::new(extents).unwrap(), true, false).unwrap());
	}
	for (index, (dtype, extents)) in outputs.into_iter().enumerate() {
		let id = ValueId::new((input_count + index) as u64 + 1);
		output_ids.push(id);
		tensors.push(Tensor::contiguous(id, dtype, Shape::new(extents).unwrap(), false, true).unwrap());
	}
	let alias_rules = (0..input_count)
		.flat_map(|input| {
			(0..output_count).map(move |output| PrimitiveAliasRule {
				input,
				output,
				permission: AliasPermission::Forbidden,
			})
		})
		.collect();
	CalculationGraph {
		tensors,
		nodes: vec![CalculationNode {
			kernel: PrimitiveKernel {
				id: KernelTemplateId::new(1),
				inputs: input_ids,
				outputs: output_ids,
				alias_rules,
				kind,
			},
		}],
	}
}

fn assert_roundtrip(graph: &CalculationGraph) {
	graph.validate().unwrap();
	let text = graph.to_ogdl().unwrap();
	assert!(text.starts_with("RecipeIR\tschema\tCalculationGraph\n\tversion\t1"));
	assert!(!text.contains('"'));
	let decoded = CalculationGraph::from_ogdl(&text).unwrap();
	assert_eq!(&decoded, graph);
	assert_eq!(decoded.to_ogdl().unwrap(), text);
}

fn scalar_program() -> ScalarProgram {
	let input = ScalarValueId::new(1);
	let constant = ScalarValueId::new(2);
	let product = ScalarValueId::new(3);
	ScalarProgram {
		inputs: vec![ScalarInput {
			id: input,
			dtype: DType::F32,
		}],
		constants: vec![ScalarConstant {
			id: constant,
			value: ScalarLiteral::F32Bits(0x7fc0_1234),
		}],
		instructions: vec![ScalarInstruction {
			result: product,
			dtype: DType::F32,
			opcode: ScalarOpcode::Multiply,
			operands: vec![input, constant],
		}],
		outputs: vec![product],
	}
}

#[test]
fn roundtrips_every_primitive_family_without_debug_serialization() {
	let graphs = [
		make_graph(
			PrimitiveKind::Elementwise(Elementwise {
				program: scalar_program(),
			}),
			vec![(DType::F32, vec![4])],
			vec![(DType::F32, vec![4])],
		),
		make_graph(
			PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Maximum,
				axes: AxisSet::new(vec![1]).unwrap(),
				keep_dimensions: false,
				result: ReduceResult::ValueAndIndex,
				tree_lanes: 64,
			}),
			vec![(DType::F32, vec![2, 4])],
			vec![(DType::F32, vec![2]), (DType::I32, vec![2])],
		),
		make_graph(
			PrimitiveKind::Scan(Scan {
				operator: ReduceOperator::Sum,
				axis: 0,
				mode: ScanMode::Exclusive {
					identity: ScalarLiteral::I32(0),
				},
				reverse: true,
				tree_lanes: 32,
			}),
			vec![(DType::I32, vec![8])],
			vec![(DType::I32, vec![8])],
		),
		make_graph(
			PrimitiveKind::Contraction(Contraction {
				batch_axes: vec![(0, 0)],
				contract_axes: vec![(2, 1)],
			}),
			vec![(DType::F32, vec![2, 3, 5]), (DType::F32, vec![2, 5, 7])],
			vec![(DType::F32, vec![2, 3, 7])],
		),
		make_graph(
			PrimitiveKind::Gather(Gather {
				axis: 1,
				bounds: IndexBounds::Clamp,
			}),
			vec![(DType::F32, vec![4, 9]), (DType::I32, vec![3])],
			vec![(DType::F32, vec![4, 3])],
		),
		make_graph(
			PrimitiveKind::Scatter(Scatter {
				axis: 1,
				bounds: IndexBounds::Wrap,
				conflict: ScatterConflict::Atomic {
					operation: AtomicOperation::Add,
					ordering: AtomicOrdering::AcquireRelease,
				},
			}),
			vec![
				(DType::F32, vec![4, 9]),
				(DType::I32, vec![3]),
				(DType::F32, vec![4, 3]),
			],
			vec![(DType::F32, vec![4, 9])],
		),
		make_graph(
			PrimitiveKind::Histogram(Histogram {
				bins: 5,
				weighted: true,
				ordering: AtomicOrdering::SequentiallyConsistent,
			}),
			vec![(DType::I32, vec![8]), (DType::F32, vec![8])],
			vec![(DType::F32, vec![5])],
		),
		make_graph(
			PrimitiveKind::Sort(Sort {
				axis: 1,
				direction: SortDirection::Descending,
				stable: true,
				emit_indices: true,
			}),
			vec![(DType::F32, vec![2, 4])],
			vec![(DType::F32, vec![2, 4]), (DType::I32, vec![2, 4])],
		),
		make_graph(
			PrimitiveKind::IndexMap(IndexMap {
				start: -7,
				element_step: 3,
				iteration_step: 64,
				modulus: Some(101),
			}),
			Vec::new(),
			vec![(DType::I32, vec![64])],
		),
		make_graph(
			PrimitiveKind::Random(RandomMap {
				distribution: RandomDistribution::BernoulliI32 {
					probability_bits: 0.25_f32.to_bits(),
				},
				key: RandomKey {
					seed_low: 11,
					seed_high: 22,
					stream: 33,
				},
				philox_rounds: 10,
			}),
			Vec::new(),
			vec![(DType::I32, vec![8])],
		),
	];

	for graph in graphs {
		assert_roundtrip(&graph);
	}
}

#[test]
fn roundtrips_empty_collections_and_all_random_distribution_shapes() {
	let empty = CalculationGraph {
		tensors: Vec::new(),
		nodes: Vec::new(),
	};
	assert_roundtrip(&empty);

	for (distribution, dtype) in [
		(RandomDistribution::UniformF32, DType::F32),
		(RandomDistribution::NormalF32, DType::F32),
		(
			RandomDistribution::BernoulliI32 {
				probability_bits: 0.75_f32.to_bits(),
			},
			DType::I32,
		),
		(
			RandomDistribution::UniformI32 {
				low: -4,
				high_exclusive: 9,
			},
			DType::I32,
		),
	] {
		assert_roundtrip(&make_graph(
			PrimitiveKind::Random(RandomMap {
				distribution,
				key: RandomKey {
					seed_low: u64::MAX,
					seed_high: 0,
					stream: 7,
				},
				philox_rounds: 10,
			}),
			Vec::new(),
			vec![(dtype, vec![16])],
		));
	}
}

#[test]
fn index_map_preserves_explicit_optional_modulus_and_signed_steps() {
	for spec in [
		IndexMap {
			start: i32::MIN,
			element_step: -1,
			iteration_step: i32::MAX,
			modulus: None,
		},
		IndexMap {
			start: -17,
			element_step: 1,
			iteration_step: 2048,
			modulus: Some(i32::MAX),
		},
	] {
		assert_roundtrip(&make_graph(
			PrimitiveKind::IndexMap(spec),
			Vec::new(),
			vec![(DType::I32, vec![8])],
		));
	}
}

#[test]
fn roundtrips_alternate_modes_and_output_arities() {
	let graphs = [
		make_graph(
			PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Sum,
				axes: AxisSet::new(vec![0]).unwrap(),
				keep_dimensions: false,
				result: ReduceResult::Value,
				tree_lanes: 8,
			}),
			vec![(DType::F32, vec![8])],
			vec![(DType::F32, vec![1])],
		),
		make_graph(
			PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Minimum,
				axes: AxisSet::new(vec![0]).unwrap(),
				keep_dimensions: true,
				result: ReduceResult::Index,
				tree_lanes: 8,
			}),
			vec![(DType::F32, vec![8])],
			vec![(DType::I32, vec![1])],
		),
		make_graph(
			PrimitiveKind::Scan(Scan {
				operator: ReduceOperator::Product,
				axis: 0,
				mode: ScanMode::Inclusive,
				reverse: false,
				tree_lanes: 8,
			}),
			vec![(DType::F32, vec![8])],
			vec![(DType::F32, vec![8])],
		),
		make_graph(
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
			vec![
				(DType::I32, vec![9]),
				(DType::I32, vec![3]),
				(DType::I32, vec![3]),
			],
			vec![(DType::I32, vec![9])],
		),
		make_graph(
			PrimitiveKind::Histogram(Histogram {
				bins: 4,
				weighted: false,
				ordering: AtomicOrdering::Relaxed,
			}),
			vec![(DType::I32, vec![8])],
			vec![(DType::I32, vec![4])],
		),
		make_graph(
			PrimitiveKind::Sort(Sort {
				axis: 0,
				direction: SortDirection::Ascending,
				stable: false,
				emit_indices: false,
			}),
			vec![(DType::I32, vec![8])],
			vec![(DType::I32, vec![8])],
		),
	];
	for graph in graphs {
		assert_roundtrip(&graph);
	}
}

#[test]
fn preserves_noncontiguous_layout_offsets_and_storage_contracts() {
	let mut tensor = Tensor::contiguous(
		ValueId::new(99),
		DType::F32,
		Shape::new(vec![2, 3]).unwrap(),
		true,
		true,
	)
	.unwrap();
	tensor.layout.offset_elements = 2;
	tensor.layout.strides = vec![10, 2];
	tensor.storage_bytes = recipe_core::ByteCount::new(68);
	let graph = CalculationGraph {
		tensors: vec![tensor],
		nodes: Vec::new(),
	};
	assert_roundtrip(&graph);
}

fn assert_document_kind(error: OgdlCodecError, expected: OgdlDocumentErrorKind) {
	assert!(
		matches!(
			error,
			OgdlCodecError::Document {
				kind,
				..
			} if kind == expected
		),
		"unexpected error: {error}"
	);
}

#[test]
fn rejects_missing_duplicate_unknown_and_misspelled_schema_fields() {
	let missing = "RecipeIR\tschema\tCalculationGraph\n\ttensors\n\tnodes";
	assert_document_kind(
		CalculationGraph::from_ogdl(missing).unwrap_err(),
		OgdlDocumentErrorKind::MissingField,
	);

	let duplicate = "RecipeIR\tschema\tCalculationGraph\n\tversion\t1\n\tversion\t1\n\ttensors\n\tnodes";
	assert_document_kind(
		CalculationGraph::from_ogdl(duplicate).unwrap_err(),
		OgdlDocumentErrorKind::DuplicateField,
	);

	let unknown = "RecipeIR\tschema\tCalculationGraph\n\tversion\t1\n\tfuture\tvalue\n\ttensors\n\tnodes";
	assert_document_kind(
		CalculationGraph::from_ogdl(unknown).unwrap_err(),
		OgdlDocumentErrorKind::UnknownField,
	);

	let wrong_schema = "RecipeIR\tschema\tcalculationgraph\n\tversion\t1\n\ttensors\n\tnodes";
	assert_document_kind(
		CalculationGraph::from_ogdl(wrong_schema).unwrap_err(),
		OgdlDocumentErrorKind::UnknownVariant,
	);

	let nested_value = "RecipeIR\tschema\tCalculationGraph\textra\n\tversion\t1\n\ttensors\n\tnodes";
	assert_document_kind(
		CalculationGraph::from_ogdl(nested_value).unwrap_err(),
		OgdlDocumentErrorKind::UnexpectedChildren,
	);
}

#[test]
fn rejects_unknown_variants_invalid_numbers_and_noncanonical_booleans() {
	let graph = make_graph(
		PrimitiveKind::Elementwise(Elementwise {
			program: scalar_program(),
		}),
		vec![(DType::F32, vec![4])],
		vec![(DType::F32, vec![4])],
	);
	let text = graph.to_ogdl().unwrap();

	let unknown_variant = text.replacen("\tElementwise", "\tElementWise", 1);
	assert_document_kind(
		CalculationGraph::from_ogdl(&unknown_variant).unwrap_err(),
		OgdlDocumentErrorKind::UnknownVariant,
	);

	let invalid_number = text.replacen(
		"storage_bytes\t16",
		"storage_bytes\t18446744073709551616",
		1,
	);
	assert_document_kind(
		CalculationGraph::from_ogdl(&invalid_number).unwrap_err(),
		OgdlDocumentErrorKind::InvalidNumber,
	);

	let invalid_bool = text.replacen("external_input\ttrue", "external_input\tTrue", 1);
	assert_document_kind(
		CalculationGraph::from_ogdl(&invalid_bool).unwrap_err(),
		OgdlDocumentErrorKind::InvalidBoolean,
	);
}

#[test]
fn validates_decoded_tensor_and_calculation_semantics() {
	let graph = make_graph(
		PrimitiveKind::Elementwise(Elementwise {
			program: scalar_program(),
		}),
		vec![(DType::F32, vec![4])],
		vec![(DType::F32, vec![4])],
	);
	let text = graph
		.to_ogdl()
		.unwrap()
		.replacen("storage_bytes\t16", "storage_bytes\t1", 1);
	let error = CalculationGraph::from_ogdl(&text).unwrap_err();
	assert!(matches!(error, OgdlCodecError::InvalidGraph(_)));

	let index_map = make_graph(
		PrimitiveKind::IndexMap(IndexMap {
			start: 0,
			element_step: 1,
			iteration_step: 8,
			modulus: Some(17),
		}),
		Vec::new(),
		vec![(DType::I32, vec![8])],
	);
	let text = index_map
		.to_ogdl()
		.unwrap()
		.replacen("Some\t17", "Some\t0", 1);
	assert!(matches!(
		CalculationGraph::from_ogdl(&text).unwrap_err(),
		OgdlCodecError::InvalidGraph(_)
	));
}
