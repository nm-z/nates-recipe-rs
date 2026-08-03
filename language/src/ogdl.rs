use core::fmt;

use recipe_core::{ AliasPermission, ByteCount, DType, KernelTemplateId, ScalarConstant, ScalarInput, ScalarInstruction,
	ScalarLiteral, ScalarOpcode, ScalarProgram, ScalarValueId, ValueId, };
use recipe_ogdl::{Graph, GraphError, NodeId, ParseError};

use crate::{
	AtomicOperation, AtomicOrdering, AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather,
	Histogram, IndexBounds, IndexMap, LanguageError, PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind,
	RandomDistribution, RandomKey, RandomMap, Reduce, ReduceOperator, ReduceResult, Scan, ScanMode, Scatter,
	ScatterConflict, Shape, Sort, SortDirection, Tensor, TensorLayout, };

const ROOT: &str = "RecipeIR";
const SCHEMA: &str = "CalculationGraph";
const VERSION: &str = "1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OgdlDocumentErrorKind { InvalidRoot, MissingField, DuplicateField, UnknownField, UnknownVariant, InvalidNumber,
	InvalidBoolean, UnexpectedChildren, UnsupportedValue, }

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OgdlCodecError { Syntax(ParseError), Document { kind: OgdlDocumentErrorKind, path: String, detail: String, },
	InvalidGraph(LanguageError), Build(GraphError), }

impl OgdlCodecError {
	fn document(kind: OgdlDocumentErrorKind, path: impl Into<String>, detail: impl Into<String>) -> Self { Self::Document {
			kind, path: path.into(), detail: detail.into(), } } }

impl fmt::Display for OgdlCodecError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Syntax(error) => write!(formatter, "invalid OGDL syntax: {error}"),
			Self::Document { kind, path, detail } => {
				write!(formatter, "invalid Recipe IR at {path}: {kind:?}: {detail}")
			}
			Self::InvalidGraph(error) => write!(formatter, "invalid calculation graph: {error}"),
			Self::Build(error) => write!(formatter, "cannot encode Recipe IR graph: {error}"),
		} } }

impl std::error::Error for OgdlCodecError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self { Self::Syntax(error) => Some(error), Self::InvalidGraph(error) => Some(error),
			Self::Build(error) => Some(error), Self::Document { .. } => None, } } }

impl From<ParseError> for OgdlCodecError { fn from(error: ParseError) -> Self { Self::Syntax(error) } }

impl From<GraphError> for OgdlCodecError { fn from(error: GraphError) -> Self { Self::Build(error) } }

impl From<LanguageError> for OgdlCodecError { fn from(error: LanguageError) -> Self { Self::InvalidGraph(error) } }

impl CalculationGraph {
	/// Encode this typed graph as the canonical Recipe `.ogdl` representation.
	///
	/// Encoding validates the complete calculation graph first. Consequently,
	/// emitted text is both syntactically canonical and semantically valid.
	pub fn to_ogdl(&self) -> Result<String, OgdlCodecError> { Ok(self.to_ogdl_graph()?.to_canonical_string()) }

	/// Encode this typed graph into the ordered graph representation.
	pub fn to_ogdl_graph(&self) -> Result<Graph, OgdlCodecError> { self.validate()?; encode_graph(self) }

	/// Parse and strictly decode a Recipe `.ogdl` calculation graph.
	///
	/// The version, every record field, enum spelling, number, and collection
	/// item is checked before [`CalculationGraph::validate`] is called.
	pub fn from_ogdl(input: &str) -> Result<Self, OgdlCodecError> { let graph = Graph::parse(input)?;
		Self::from_ogdl_graph(&graph) }

	/// Strictly decode an already parsed ordered graph.
	pub fn from_ogdl_graph(graph: &Graph) -> Result<Self, OgdlCodecError> { decode_graph(graph) } }

fn encode_graph(source: &CalculationGraph) -> Result<Graph, OgdlCodecError> { let mut graph = Graph::new();
	let root = graph.append_root(ROOT)?;
	field_value(&mut graph, root, "schema", SCHEMA)?;
	field_value(&mut graph, root, "version", VERSION)?;
	let tensors = graph.append_child(root, "tensors")?;
	for tensor in &source.tensors { encode_tensor(&mut graph, tensors, tensor)?; }
	let nodes = graph.append_child(root, "nodes")?;
	for node in &source.nodes {
		let target = graph.append_child(nodes, "node")?;
		let kernel = graph.append_child(target, "kernel")?;
		encode_kernel(&mut graph, kernel, &node.kernel)?; }
	Ok(graph) }

fn field_value( graph: &mut Graph, parent: NodeId, name: &str, value: impl Into<String>,
) -> Result<NodeId, OgdlCodecError> { let field = graph.append_child(parent, name)?;
	Ok(graph.append_child(field, value.into())?) }

fn encode_tensor(graph: &mut Graph, parent: NodeId, tensor: &Tensor) -> Result<(), OgdlCodecError> {
	let target = graph.append_child(parent, "tensor")?;
	field_value(graph, target, "id", tensor.id.get().to_string())?;
	field_value(graph, target, "dtype", dtype_name(tensor.dtype))?;
	let shape = graph.append_child(target, "shape")?;
	for extent in tensor.shape.extents() {
		field_value(graph, shape, "extent", extent.to_string())?;
	}
	let layout = graph.append_child(target, "layout")?;
	field_value( graph, layout,
		"offset_elements",
		tensor.layout.offset_elements.to_string(), )?;
	let strides = graph.append_child(layout, "strides")?;
	for stride in &tensor.layout.strides {
		field_value(graph, strides, "stride", stride.to_string())?;
	}
	field_value( graph, target,
		"storage_bytes",
		tensor.storage_bytes.get().to_string(), )?; field_value( graph, target,
		"external_input",
		bool_name(tensor.external_input), )?; field_value( graph, target,
		"external_output",
		bool_name(tensor.external_output), )?; Ok(()) }

fn encode_kernel(graph: &mut Graph, parent: NodeId, kernel: &PrimitiveKernel) -> Result<(), OgdlCodecError> {
	field_value(graph, parent, "id", kernel.id.get().to_string())?;
	let inputs = graph.append_child(parent, "inputs")?;
	for input in &kernel.inputs {
		field_value(graph, inputs, "value", input.get().to_string())?;
	}
	let outputs = graph.append_child(parent, "outputs")?;
	for output in &kernel.outputs {
		field_value(graph, outputs, "value", output.get().to_string())?;
	}
	let aliases = graph.append_child(parent, "alias_rules")?;
	for rule in &kernel.alias_rules {
		let target = graph.append_child(aliases, "alias_rule")?;
		field_value(graph, target, "input", rule.input.to_string())?;
		field_value(graph, target, "output", rule.output.to_string())?;
		field_value( graph, target,
			"permission",
			alias_permission_name(rule.permission), )?; }
	let kind = graph.append_child(parent, "kind")?;
	encode_primitive(graph, kind, &kernel.kind) }

fn encode_primitive(graph: &mut Graph, parent: NodeId, kind: &PrimitiveKind) -> Result<(), OgdlCodecError> {
	match kind { PrimitiveKind::Elementwise(spec) => {
			let variant = graph.append_child(parent, "Elementwise")?;
			let program = graph.append_child(variant, "program")?;
			encode_scalar_program(graph, program, &spec.program) }
		PrimitiveKind::Reduce(spec) => {
			let variant = graph.append_child(parent, "Reduce")?;
			field_value( graph, variant,
				"operator",
				reduce_operator_name(spec.operator), )?;
			let axes = graph.append_child(variant, "axes")?;
			for axis in spec.axes.as_slice() {
				field_value(graph, axes, "axis", axis.to_string())?;
			}
			field_value( graph, variant,
				"keep_dimensions",
				bool_name(spec.keep_dimensions), )?;
			field_value(graph, variant, "result", reduce_result_name(spec.result))?;
			field_value(graph, variant, "tree_lanes", spec.tree_lanes.to_string())?;
			Ok(()) }
		PrimitiveKind::Scan(spec) => {
			let variant = graph.append_child(parent, "Scan")?;
			field_value( graph, variant,
				"operator",
				reduce_operator_name(spec.operator), )?;
			field_value(graph, variant, "axis", spec.axis.to_string())?;
			let mode = graph.append_child(variant, "mode")?;
			match spec.mode { ScanMode::Inclusive => {
					graph.append_child(mode, "Inclusive")?;
				}
				ScanMode::Exclusive { identity } => {
					let exclusive = graph.append_child(mode, "Exclusive")?;
					let identity_node = graph.append_child(exclusive, "identity")?;
					encode_literal(graph, identity_node, identity)?; } }
			field_value(graph, variant, "reverse", bool_name(spec.reverse))?;
			field_value(graph, variant, "tree_lanes", spec.tree_lanes.to_string())?;
			Ok(()) }
		PrimitiveKind::Contraction(spec) => {
			let variant = graph.append_child(parent, "Contraction")?;
			encode_axis_pairs(graph, variant, "batch_axes", &spec.batch_axes)?;
			encode_axis_pairs(graph, variant, "contract_axes", &spec.contract_axes)
		}
		PrimitiveKind::Gather(spec) => {
			let variant = graph.append_child(parent, "Gather")?;
			field_value(graph, variant, "axis", spec.axis.to_string())?;
			field_value(graph, variant, "bounds", index_bounds_name(spec.bounds))?;
			Ok(()) }
		PrimitiveKind::Scatter(spec) => {
			let variant = graph.append_child(parent, "Scatter")?;
			field_value(graph, variant, "axis", spec.axis.to_string())?;
			field_value(graph, variant, "bounds", index_bounds_name(spec.bounds))?;
			let conflict = graph.append_child(variant, "conflict")?;
			match spec.conflict { ScatterConflict::UniqueIndices => {
					graph.append_child(conflict, "UniqueIndices")?;
				}
				ScatterConflict::Atomic { operation, ordering, } => {
					let atomic = graph.append_child(conflict, "Atomic")?;
					field_value(graph, atomic, "operation", atomic_operation_name(operation))?;
					field_value(graph, atomic, "ordering", atomic_ordering_name(ordering))?;
				} }
			Ok(()) }
		PrimitiveKind::Histogram(spec) => {
			let variant = graph.append_child(parent, "Histogram")?;
			field_value(graph, variant, "bins", spec.bins.to_string())?;
			field_value(graph, variant, "weighted", bool_name(spec.weighted))?;
			field_value( graph, variant,
				"ordering",
				atomic_ordering_name(spec.ordering), )?; Ok(()) }
		PrimitiveKind::Sort(spec) => {
			let variant = graph.append_child(parent, "Sort")?;
			field_value(graph, variant, "axis", spec.axis.to_string())?;
			field_value( graph, variant,
				"direction",
				sort_direction_name(spec.direction), )?;
			field_value(graph, variant, "stable", bool_name(spec.stable))?;
			field_value(graph, variant, "emit_indices", bool_name(spec.emit_indices))?;
			Ok(()) }
		PrimitiveKind::IndexMap(spec) => {
			let variant = graph.append_child(parent, "IndexMap")?;
			field_value(graph, variant, "start", spec.start.to_string())?;
			field_value( graph, variant,
				"element_step",
				spec.element_step.to_string(), )?; field_value( graph, variant,
				"iteration_step",
				spec.iteration_step.to_string(), )?;
			let modulus = graph.append_child(variant, "modulus")?;
			match spec.modulus { Some(value) => {
					field_value(graph, modulus, "Some", value.to_string())?;
				}
				None => {
					graph.append_child(modulus, "None")?;
				} }
			Ok(()) }
		PrimitiveKind::Random(spec) => {
			let variant = graph.append_child(parent, "Random")?;
			let distribution = graph.append_child(variant, "distribution")?;
			encode_random_distribution(graph, distribution, spec.distribution)?;
			let key = graph.append_child(variant, "key")?;
			field_value(graph, key, "seed_low", spec.key.seed_low.to_string())?;
			field_value(graph, key, "seed_high", spec.key.seed_high.to_string())?;
			field_value(graph, key, "stream", spec.key.stream.to_string())?;
			field_value( graph, variant,
				"philox_rounds",
				spec.philox_rounds.to_string(), )?; Ok(()) } } }

fn encode_axis_pairs( graph: &mut Graph, parent: NodeId, name: &str, pairs: &[(usize, usize)],
) -> Result<(), OgdlCodecError> { let list = graph.append_child(parent, name)?; for (left, right) in pairs {
		let pair = graph.append_child(list, "pair")?;
		field_value(graph, pair, "left", left.to_string())?;
		field_value(graph, pair, "right", right.to_string())?;
	}
	Ok(()) }

fn encode_random_distribution( graph: &mut Graph, parent: NodeId, distribution: RandomDistribution,
) -> Result<(), OgdlCodecError> { match distribution { RandomDistribution::UniformF32 => {
			graph.append_child(parent, "UniformF32")?;
		}
		RandomDistribution::NormalF32 => {
			graph.append_child(parent, "NormalF32")?;
		}
		RandomDistribution::BernoulliI32 { probability_bits } => {
			let variant = graph.append_child(parent, "BernoulliI32")?;
			field_value( graph, variant,
				"probability_bits",
				probability_bits.to_string(), )?; }
		RandomDistribution::UniformI32 { low, high_exclusive, } => {
			let variant = graph.append_child(parent, "UniformI32")?;
			field_value(graph, variant, "low", low.to_string())?;
			field_value(graph, variant, "high_exclusive", high_exclusive.to_string())?;
		} }
	Ok(()) }

fn encode_scalar_program(graph: &mut Graph, parent: NodeId, program: &ScalarProgram) -> Result<(), OgdlCodecError> {
	let inputs = graph.append_child(parent, "inputs")?;
	for input in &program.inputs {
		let target = graph.append_child(inputs, "input")?;
		field_value(graph, target, "id", input.id.get().to_string())?;
		field_value(graph, target, "dtype", dtype_name(input.dtype))?;
	}
	let constants = graph.append_child(parent, "constants")?;
	for constant in &program.constants {
		let target = graph.append_child(constants, "constant")?;
		field_value(graph, target, "id", constant.id.get().to_string())?;
		let literal = graph.append_child(target, "literal")?;
		encode_literal(graph, literal, constant.value)?; }
	let instructions = graph.append_child(parent, "instructions")?;
	for instruction in &program.instructions {
		let target = graph.append_child(instructions, "instruction")?;
		field_value( graph, target,
			"result",
			instruction.result.get().to_string(), )?;
		field_value(graph, target, "dtype", dtype_name(instruction.dtype))?;
		let opcode = scalar_opcode_name(instruction.opcode).ok_or_else(|| { OgdlCodecError::document(
				OgdlDocumentErrorKind::UnsupportedValue,
				"nodes.node.kernel.kind.Elementwise.program.instructions.instruction.opcode",
				"scalar opcode is newer than Recipe IR schema version 1",
			) })?;
		field_value(graph, target, "opcode", opcode)?;
		let operands = graph.append_child(target, "operands")?;
		for operand in &instruction.operands {
			field_value(graph, operands, "value", operand.get().to_string())?;
		} }
	let outputs = graph.append_child(parent, "outputs")?;
	for output in &program.outputs {
		field_value(graph, outputs, "value", output.get().to_string())?;
	}
	Ok(()) }

fn encode_literal(graph: &mut Graph, parent: NodeId, literal: ScalarLiteral) -> Result<(), OgdlCodecError> {
	match literal { ScalarLiteral::F32Bits(bits) => {
			field_value(graph, parent, "F32Bits", bits.to_string())?;
		}
		ScalarLiteral::I32(value) => {
			field_value(graph, parent, "I32", value.to_string())?;
		} }
	Ok(()) }

fn decode_graph(source: &Graph) -> Result<CalculationGraph, OgdlCodecError> { if source.roots().len() != 1 {
		return Err(OgdlCodecError::document( OgdlDocumentErrorKind::InvalidRoot,
			"$",
			format!(
				"Recipe IR requires exactly one {ROOT} root; found {}",
				source.roots().len() ), )); }
	let root = source.roots()[0]; if node_text(source, root) != ROOT { return Err(OgdlCodecError::document(
			OgdlDocumentErrorKind::InvalidRoot,
			"$",
			format!("expected root {ROOT}, found {}", node_text(source, root)),
		)); }
	let fields = required_fields(source, root, ROOT, &[
		"schema", "version", "tensors", "nodes",
	])?;
	let schema = value_text(source, fields[0], "RecipeIR.schema")?;
	if schema != SCHEMA { return Err(OgdlCodecError::document( OgdlDocumentErrorKind::UnknownVariant,
			"RecipeIR.schema",
			format!("expected {SCHEMA}, found {schema}"),
		)); }
	let version = value_text(source, fields[1], "RecipeIR.version")?;
	if version != VERSION { return Err(OgdlCodecError::document( OgdlDocumentErrorKind::UnknownVariant,
			"RecipeIR.version",
			format!("supported version is {VERSION}, found {version}"),
		)); }
	let tensors = decode_list( source, fields[2],
		"RecipeIR.tensors",
		"tensor",
		decode_tensor, )?; let nodes = decode_list( source, fields[3],
		"RecipeIR.nodes",
		"node",
		decode_calculation_node, )?; let graph = CalculationGraph { tensors, nodes }; graph.validate()?; Ok(graph) }

fn decode_calculation_node(source: &Graph, node: NodeId, path: &str) -> Result<CalculationNode, OgdlCodecError> {
	let fields = required_fields(source, node, path, &["kernel"])?;
	Ok(CalculationNode {
		kernel: decode_kernel(source, fields[0], &format!("{path}.kernel"))?,
	}) }

fn decode_tensor(source: &Graph, node: NodeId, path: &str) -> Result<Tensor, OgdlCodecError> {
	let fields = required_fields(source, node, path, &[
		"id",
		"dtype",
		"shape",
		"layout",
		"storage_bytes",
		"external_input",
		"external_output",
	])?;
	let id = ValueId::new(parse_number(source, fields[0], &format!("{path}.id"))?);
	let dtype = parse_dtype(
		value_text(source, fields[1], &format!("{path}.dtype"))?,
		&format!("{path}.dtype"),
	)?;
	let shape_path = format!("{path}.shape");
	let extents = decode_values(source, fields[2], &shape_path, "extent")?
		.into_iter() .enumerate()
		.map(|(index, value)| parse_number_text(value, &format!("{shape_path}.extent[{index}]")))
		.collect::<Result<Vec<u64>, _>>()?; let shape = Shape::new(extents)?;
	let layout_path = format!("{path}.layout");
	let layout_fields = required_fields(source, fields[3], &layout_path, &[
		"offset_elements",
		"strides",
	])?; let offset_elements = parse_number( source, layout_fields[0],
		&format!("{layout_path}.offset_elements"),
	)?;
	let strides_path = format!("{layout_path}.strides");
	let strides = decode_values(source, layout_fields[1], &strides_path, "stride")?
		.into_iter() .enumerate()
		.map(|(index, value)| parse_number_text(value, &format!("{strides_path}.stride[{index}]")))
		.collect::<Result<Vec<u64>, _>>()?; Ok(Tensor { id, dtype, shape, layout: TensorLayout { offset_elements, strides, },
		storage_bytes: ByteCount::new(parse_number( source, fields[4],
			&format!("{path}.storage_bytes"),
		)?), external_input: parse_bool(
			value_text(source, fields[5], &format!("{path}.external_input"))?,
			&format!("{path}.external_input"),
		)?, external_output: parse_bool(
			value_text(source, fields[6], &format!("{path}.external_output"))?,
			&format!("{path}.external_output"),
		)?, }) }

fn decode_kernel(source: &Graph, node: NodeId, path: &str) -> Result<PrimitiveKernel, OgdlCodecError> {
	let fields = required_fields(source, node, path, &[
		"id",
		"inputs",
		"outputs",
		"alias_rules",
		"kind",
	])?;
	let id = KernelTemplateId::new(parse_number(source, fields[0], &format!("{path}.id"))?);
	let inputs_path = format!("{path}.inputs");
	let inputs = decode_values(source, fields[1], &inputs_path, "value")?
		.into_iter() .enumerate()
		.map(|(index, value)| parse_number_text(value, &format!("{inputs_path}.value[{index}]")).map(ValueId::new))
		.collect::<Result<Vec<_>, _>>()?;
	let outputs_path = format!("{path}.outputs");
	let outputs =
		decode_values(source, fields[2], &outputs_path, "value")?
			.into_iter() .enumerate() .map(|(index, value)| {
				parse_number_text(value, &format!("{outputs_path}.value[{index}]")).map(ValueId::new)
			}) .collect::<Result<Vec<_>, _>>()?; let alias_rules = decode_list( source, fields[3],
		&format!("{path}.alias_rules"),
		"alias_rule",
		decode_alias_rule, )?;
	let kind = decode_primitive(source, fields[4], &format!("{path}.kind"))?;
	Ok(PrimitiveKernel { id, inputs, outputs, alias_rules, kind, }) }

fn decode_alias_rule(source: &Graph, node: NodeId, path: &str) -> Result<PrimitiveAliasRule, OgdlCodecError> {
	let fields = required_fields(source, node, path, &["input", "output", "permission"])?;
	Ok(PrimitiveAliasRule {
		input: parse_number(source, fields[0], &format!("{path}.input"))?,
		output: parse_number(source, fields[1], &format!("{path}.output"))?,
		permission: parse_alias_permission(
			value_text(source, fields[2], &format!("{path}.permission"))?,
			&format!("{path}.permission"),
		)?, }) }

fn decode_primitive(source: &Graph, node: NodeId, path: &str) -> Result<PrimitiveKind, OgdlCodecError> {
	let variant = only_child(source, node, path)?;
	let variant_path = format!("{path}.{}", node_text(source, variant));
	match node_text(source, variant) {
		"Elementwise" => {
			let fields = required_fields(source, variant, &variant_path, &["program"])?;
			Ok(PrimitiveKind::Elementwise(Elementwise {
				program: decode_scalar_program(source, fields[0], &format!("{variant_path}.program"))?,
			})) }
		"Reduce" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"operator",
				"axes",
				"keep_dimensions",
				"result",
				"tree_lanes",
			])?;
			let axes_path = format!("{variant_path}.axes");
			let axes = decode_values(source, fields[1], &axes_path, "axis")?
				.into_iter() .enumerate()
				.map(|(index, value)| parse_number_text(value, &format!("{axes_path}.axis[{index}]")))
				.collect::<Result<Vec<usize>, _>>()?; Ok(PrimitiveKind::Reduce(Reduce { operator: parse_reduce_operator(
					value_text(source, fields[0], &format!("{variant_path}.operator"))?,
					&format!("{variant_path}.operator"),
				)?, axes: AxisSet::new(axes)?, keep_dimensions: parse_bool( value_text( source, fields[2],
						&format!("{variant_path}.keep_dimensions"),
					)?,
					&format!("{variant_path}.keep_dimensions"),
				)?, result: parse_reduce_result(
					value_text(source, fields[3], &format!("{variant_path}.result"))?,
					&format!("{variant_path}.result"),
				)?,
				tree_lanes: parse_number(source, fields[4], &format!("{variant_path}.tree_lanes"))?,
			})) }
		"Scan" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"operator",
				"axis",
				"mode",
				"reverse",
				"tree_lanes",
			])?; Ok(PrimitiveKind::Scan(Scan { operator: parse_reduce_operator(
					value_text(source, fields[0], &format!("{variant_path}.operator"))?,
					&format!("{variant_path}.operator"),
				)?,
				axis: parse_number(source, fields[1], &format!("{variant_path}.axis"))?,
				mode: decode_scan_mode(source, fields[2], &format!("{variant_path}.mode"))?,
				reverse: parse_bool(
					value_text(source, fields[3], &format!("{variant_path}.reverse"))?,
					&format!("{variant_path}.reverse"),
				)?,
				tree_lanes: parse_number(source, fields[4], &format!("{variant_path}.tree_lanes"))?,
			})) }
		"Contraction" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"batch_axes",
				"contract_axes",
			])?; Ok(PrimitiveKind::Contraction(Contraction {
				batch_axes: decode_axis_pairs(source, fields[0], &format!("{variant_path}.batch_axes"))?,
				contract_axes: decode_axis_pairs(source, fields[1], &format!("{variant_path}.contract_axes"))?,
			})) }
		"Gather" => {
			let fields = required_fields(source, variant, &variant_path, &["axis", "bounds"])?;
			Ok(PrimitiveKind::Gather(Gather {
				axis: parse_number(source, fields[0], &format!("{variant_path}.axis"))?,
				bounds: parse_index_bounds(
					value_text(source, fields[1], &format!("{variant_path}.bounds"))?,
					&format!("{variant_path}.bounds"),
				)?, })) }
		"Scatter" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"axis", "bounds", "conflict",
			])?; Ok(PrimitiveKind::Scatter(Scatter {
				axis: parse_number(source, fields[0], &format!("{variant_path}.axis"))?,
				bounds: parse_index_bounds(
					value_text(source, fields[1], &format!("{variant_path}.bounds"))?,
					&format!("{variant_path}.bounds"),
				)?,
				conflict: decode_scatter_conflict(source, fields[2], &format!("{variant_path}.conflict"))?,
			})) }
		"Histogram" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"bins", "weighted", "ordering",
			])?; Ok(PrimitiveKind::Histogram(Histogram {
				bins: parse_number(source, fields[0], &format!("{variant_path}.bins"))?,
				weighted: parse_bool(
					value_text(source, fields[1], &format!("{variant_path}.weighted"))?,
					&format!("{variant_path}.weighted"),
				)?, ordering: parse_atomic_ordering(
					value_text(source, fields[2], &format!("{variant_path}.ordering"))?,
					&format!("{variant_path}.ordering"),
				)?, })) }
		"Sort" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"axis",
				"direction",
				"stable",
				"emit_indices",
			])?; Ok(PrimitiveKind::Sort(Sort {
				axis: parse_number(source, fields[0], &format!("{variant_path}.axis"))?,
				direction: parse_sort_direction(
					value_text(source, fields[1], &format!("{variant_path}.direction"))?,
					&format!("{variant_path}.direction"),
				)?, stable: parse_bool(
					value_text(source, fields[2], &format!("{variant_path}.stable"))?,
					&format!("{variant_path}.stable"),
				)?, emit_indices: parse_bool(
					value_text(source, fields[3], &format!("{variant_path}.emit_indices"))?,
					&format!("{variant_path}.emit_indices"),
				)?, })) }
		"IndexMap" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"start",
				"element_step",
				"iteration_step",
				"modulus",
			])?; Ok(PrimitiveKind::IndexMap(IndexMap {
				start: parse_number(source, fields[0], &format!("{variant_path}.start"))?,
				element_step: parse_number(source, fields[1], &format!("{variant_path}.element_step"))?,
				iteration_step: parse_number(source, fields[2], &format!("{variant_path}.iteration_step"))?,
				modulus: decode_positive_modulus(source, fields[3], &format!("{variant_path}.modulus"))?,
			})) }
		"Random" => {
			let fields = required_fields(source, variant, &variant_path, &[
				"distribution",
				"key",
				"philox_rounds",
			])?;
			let key_path = format!("{variant_path}.key");
			let key_fields = required_fields(source, fields[1], &key_path, &[
				"seed_low",
				"seed_high",
				"stream",
			])?; Ok(PrimitiveKind::Random(RandomMap { distribution: decode_random_distribution( source, fields[0],
					&format!("{variant_path}.distribution"),
				)?, key: RandomKey {
					seed_low: parse_number(source, key_fields[0], &format!("{key_path}.seed_low"))?,
					seed_high: parse_number(source, key_fields[1], &format!("{key_path}.seed_high"))?,
					stream: parse_number(source, key_fields[2], &format!("{key_path}.stream"))?,
				},
				philox_rounds: parse_number(source, fields[2], &format!("{variant_path}.philox_rounds"))?,
			})) }
		name => Err(unknown_variant(path, name)), } }

fn decode_positive_modulus(source: &Graph, node: NodeId, path: &str) -> Result<Option<i32>, OgdlCodecError> {
	let variant = only_child(source, node, path)?; match node_text(source, variant) {
		"None" => {
			require_leaf(source, variant, &format!("{path}.None"))?;
			Ok(None) }
		"Some" => {
			let value = value_text(source, variant, &format!("{path}.Some"))?;
			Ok(Some(parse_number_text(value, &format!("{path}.Some"))?))
		}
		name => Err(unknown_variant(path, name)), } }

fn decode_scan_mode(source: &Graph, node: NodeId, path: &str) -> Result<ScanMode, OgdlCodecError> {
	let variant = only_child(source, node, path)?; match node_text(source, variant) {
		"Inclusive" => {
			require_leaf(source, variant, &format!("{path}.Inclusive"))?;
			Ok(ScanMode::Inclusive) }
		"Exclusive" => {
			let variant_path = format!("{path}.Exclusive");
			let fields = required_fields(source, variant, &variant_path, &["identity"])?;
			Ok(ScanMode::Exclusive {
				identity: decode_literal(source, fields[0], &format!("{variant_path}.identity"))?,
			}) }
		name => Err(unknown_variant(path, name)), } }

fn decode_scatter_conflict(source: &Graph, node: NodeId, path: &str) -> Result<ScatterConflict, OgdlCodecError> {
	let variant = only_child(source, node, path)?; match node_text(source, variant) {
		"UniqueIndices" => {
			require_leaf(source, variant, &format!("{path}.UniqueIndices"))?;
			Ok(ScatterConflict::UniqueIndices) }
		"Atomic" => {
			let variant_path = format!("{path}.Atomic");
			let fields = required_fields(source, variant, &variant_path, &["operation", "ordering"])?;
			Ok(ScatterConflict::Atomic { operation: parse_atomic_operation(
					value_text(source, fields[0], &format!("{variant_path}.operation"))?,
					&format!("{variant_path}.operation"),
				)?, ordering: parse_atomic_ordering(
					value_text(source, fields[1], &format!("{variant_path}.ordering"))?,
					&format!("{variant_path}.ordering"),
				)?, }) }
		name => Err(unknown_variant(path, name)), } }

fn decode_random_distribution(source: &Graph, node: NodeId, path: &str) -> Result<RandomDistribution, OgdlCodecError> {
	let variant = only_child(source, node, path)?; match node_text(source, variant) {
		"UniformF32" => {
			require_leaf(source, variant, &format!("{path}.UniformF32"))?;
			Ok(RandomDistribution::UniformF32) }
		"NormalF32" => {
			require_leaf(source, variant, &format!("{path}.NormalF32"))?;
			Ok(RandomDistribution::NormalF32) }
		"BernoulliI32" => {
			let variant_path = format!("{path}.BernoulliI32");
			let fields = required_fields(source, variant, &variant_path, &["probability_bits"])?;
			Ok(RandomDistribution::BernoulliI32 { probability_bits: parse_number( source, fields[0],
					&format!("{variant_path}.probability_bits"),
				)?, }) }
		"UniformI32" => {
			let variant_path = format!("{path}.UniformI32");
			let fields = required_fields(source, variant, &variant_path, &["low", "high_exclusive"])?;
			Ok(RandomDistribution::UniformI32 {
				low: parse_number(source, fields[0], &format!("{variant_path}.low"))?,
				high_exclusive: parse_number(source, fields[1], &format!("{variant_path}.high_exclusive"))?,
			}) }
		name => Err(unknown_variant(path, name)), } }

fn decode_axis_pairs(source: &Graph, node: NodeId, path: &str) -> Result<Vec<(usize, usize)>, OgdlCodecError> {
	decode_list(source, node, path, "pair", |source, pair, pair_path| {
		let fields = required_fields(source, pair, pair_path, &["left", "right"])?;
		Ok((
			parse_number(source, fields[0], &format!("{pair_path}.left"))?,
			parse_number(source, fields[1], &format!("{pair_path}.right"))?,
		)) }) }

fn decode_scalar_program(source: &Graph, node: NodeId, path: &str) -> Result<ScalarProgram, OgdlCodecError> {
	let fields = required_fields(source, node, path, &[
		"inputs",
		"constants",
		"instructions",
		"outputs",
	])?; let inputs = decode_list( source, fields[0],
		&format!("{path}.inputs"),
		"input",
		|source, input, input_path| {
			let fields = required_fields(source, input, input_path, &["id", "dtype"])?;
			Ok(ScalarInput { id: ScalarValueId::new(parse_number( source, fields[0],
					&format!("{input_path}.id"),
				)?), dtype: parse_dtype(
					value_text(source, fields[1], &format!("{input_path}.dtype"))?,
					&format!("{input_path}.dtype"),
				)?, }) }, )?; let constants = decode_list( source, fields[1],
		&format!("{path}.constants"),
		"constant",
		|source, constant, constant_path| {
			let fields = required_fields(source, constant, constant_path, &["id", "literal"])?;
			Ok(ScalarConstant { id: ScalarValueId::new(parse_number( source, fields[0],
					&format!("{constant_path}.id"),
				)?),
				value: decode_literal(source, fields[1], &format!("{constant_path}.literal"))?,
			}) }, )?; let instructions = decode_list( source, fields[2],
		&format!("{path}.instructions"),
		"instruction",
		decode_scalar_instruction, )?;
	let outputs_path = format!("{path}.outputs");
	let outputs = decode_values(source, fields[3], &outputs_path, "value")?
		.into_iter() .enumerate() .map(|(index, value)| {
			parse_number_text(value, &format!("{outputs_path}.value[{index}]")).map(ScalarValueId::new)
		}) .collect::<Result<Vec<_>, _>>()?; Ok(ScalarProgram { inputs, constants, instructions, outputs, }) }

fn decode_scalar_instruction(source: &Graph, node: NodeId, path: &str) -> Result<ScalarInstruction, OgdlCodecError> {
	let fields = required_fields(source, node, path, &[
		"result", "dtype", "opcode", "operands",
	])?;
	let operands_path = format!("{path}.operands");
	let operands = decode_values(source, fields[3], &operands_path, "value")?
		.into_iter() .enumerate() .map(|(index, value)| {
			parse_number_text(value, &format!("{operands_path}.value[{index}]")).map(ScalarValueId::new)
		}) .collect::<Result<Vec<_>, _>>()?; Ok(ScalarInstruction {
		result: ScalarValueId::new(parse_number(source, fields[0], &format!("{path}.result"))?),
		dtype: parse_dtype(
			value_text(source, fields[1], &format!("{path}.dtype"))?,
			&format!("{path}.dtype"),
		)?, opcode: parse_scalar_opcode(
			value_text(source, fields[2], &format!("{path}.opcode"))?,
			&format!("{path}.opcode"),
		)?, operands, }) }

fn decode_literal(source: &Graph, node: NodeId, path: &str) -> Result<ScalarLiteral, OgdlCodecError> {
	let variant = only_child(source, node, path)?; match node_text(source, variant) {
		"F32Bits" => {
			Ok(ScalarLiteral::F32Bits(parse_number( source, variant,
				&format!("{path}.F32Bits"),
			)?)) }
		"I32" => {
			Ok(ScalarLiteral::I32(parse_number( source, variant,
				&format!("{path}.I32"),
			)?)) }
		name => Err(unknown_variant(path, name)), } }

fn node_text(graph: &Graph, node: NodeId) -> &str { graph.node(node)
		.expect("NodeId obtained from this immutable graph")
		.text() }

fn node_children(graph: &Graph, node: NodeId) -> &[NodeId] { graph.node(node)
		.expect("NodeId obtained from this immutable graph")
		.children() }

fn required_fields(graph: &Graph, parent: NodeId, path: &str, names: &[&str]) -> Result<Vec<NodeId>, OgdlCodecError> {
	let mut found = vec![None; names.len()]; for child in node_children(graph, parent) {
		let name = node_text(graph, *child); let Some(index) = names.iter().position(|expected| *expected == name) else {
			return Err(OgdlCodecError::document( OgdlDocumentErrorKind::UnknownField,
				format!("{path}.{name}"),
				format!("field {name} is not part of this record"),
			)); }; if found[index].replace(*child).is_some() { return Err(OgdlCodecError::document(
				OgdlDocumentErrorKind::DuplicateField,
				format!("{path}.{name}"),
				format!("field {name} appears more than once"),
			)); } }
	for (index, name) in names.iter().enumerate() { if found[index].is_none() { return Err(OgdlCodecError::document(
				OgdlDocumentErrorKind::MissingField,
				format!("{path}.{name}"),
				format!("required field {name} is absent"),
			)); } }
	Ok(found .into_iter()
		.map(|field| field.expect("presence checked above"))
		.collect()) }

fn only_child(graph: &Graph, parent: NodeId, path: &str) -> Result<NodeId, OgdlCodecError> {
	let children = node_children(graph, parent); match children { [child] => Ok(*child), [] => {
			Err(OgdlCodecError::document( OgdlDocumentErrorKind::MissingField, path,
				"exactly one variant/value child is required",
			)) }
		_ => { Err(OgdlCodecError::document( OgdlDocumentErrorKind::UnexpectedChildren, path, format!(
					"expected exactly one variant/value child, found {}",
					children.len() ), )) } } }

fn require_leaf(graph: &Graph, node: NodeId, path: &str) -> Result<(), OgdlCodecError> {
	let count = node_children(graph, node).len(); if count == 0 { Ok(()) } else { Err(OgdlCodecError::document(
			OgdlDocumentErrorKind::UnexpectedChildren, path,
			format!("leaf node has {count} unexpected children"),
		)) } }

fn value_text<'a>(graph: &'a Graph, field: NodeId, path: &str) -> Result<&'a str, OgdlCodecError> {
	let value = only_child(graph, field, path)?; require_leaf(graph, value, path)?; Ok(node_text(graph, value)) }

fn decode_values<'a>(
	graph: &'a Graph,
	parent: NodeId, path: &str, item_name: &str,
) -> Result<Vec<&'a str>, OgdlCodecError> {
	node_children(graph, parent) .iter() .enumerate() .map(|(index, child)| { let actual = node_text(graph, *child);
			if actual != item_name { return Err(OgdlCodecError::document( OgdlDocumentErrorKind::UnknownField,
					format!("{path}.{actual}"),
					format!("expected collection item {item_name}, found {actual}"),
				)); }
			value_text(graph, *child, &format!("{path}.{item_name}[{index}]"))
		}) .collect() }

fn decode_list<T>( graph: &Graph, parent: NodeId, path: &str, item_name: &str,
	decode: impl Fn(&Graph, NodeId, &str) -> Result<T, OgdlCodecError>, ) -> Result<Vec<T>, OgdlCodecError> {
	node_children(graph, parent) .iter() .enumerate() .map(|(index, child)| { let actual = node_text(graph, *child);
			if actual != item_name { return Err(OgdlCodecError::document( OgdlDocumentErrorKind::UnknownField,
					format!("{path}.{actual}"),
					format!("expected collection item {item_name}, found {actual}"),
				)); }
			decode(graph, *child, &format!("{path}.{item_name}[{index}]"))
		}) .collect() }

fn parse_number<T>(graph: &Graph, field: NodeId, path: &str) -> Result<T, OgdlCodecError>
where T: core::str::FromStr { parse_number_text(value_text(graph, field, path)?, path) }

fn parse_number_text<T>(text: &str, path: &str) -> Result<T, OgdlCodecError>
where T: core::str::FromStr { text.parse().map_err(|_| { OgdlCodecError::document( OgdlDocumentErrorKind::InvalidNumber,
			path,
			format!("{text:?} is not a valid in-range integer"),
		) }) }

fn parse_bool(text: &str, path: &str) -> Result<bool, OgdlCodecError> { match text {
		"true" => Ok(true),
		"false" => Ok(false),
		_ => { Err(OgdlCodecError::document( OgdlDocumentErrorKind::InvalidBoolean, path,
				format!("expected exact true or false, found {text}"),
			)) } } }

fn unknown_variant(path: &str, name: &str) -> OgdlCodecError { OgdlCodecError::document(
		OgdlDocumentErrorKind::UnknownVariant, path,
		format!("unknown exact enum variant {name}"),
	) }

const fn bool_name(value: bool) -> &'static str { if value { "true" } else { "false" } }

const fn dtype_name(value: DType) -> &'static str {
	match value {
		DType::F32 => "F32",
		DType::I32 => "I32",
	} }

fn parse_dtype(text: &str, path: &str) -> Result<DType, OgdlCodecError> { match text {
		"F32" => Ok(DType::F32),
		"I32" => Ok(DType::I32),
		name => Err(unknown_variant(path, name)), } }

const fn alias_permission_name(value: AliasPermission) -> &'static str {
	match value {
		AliasPermission::Forbidden => "Forbidden",
		AliasPermission::MayAliasExact => "MayAliasExact",
		AliasPermission::MustAliasExact => "MustAliasExact",
	} }

fn parse_alias_permission(text: &str, path: &str) -> Result<AliasPermission, OgdlCodecError> { match text {
		"Forbidden" => Ok(AliasPermission::Forbidden),
		"MayAliasExact" => Ok(AliasPermission::MayAliasExact),
		"MustAliasExact" => Ok(AliasPermission::MustAliasExact),
		name => Err(unknown_variant(path, name)), } }

const fn atomic_ordering_name(value: AtomicOrdering) -> &'static str {
	match value {
		AtomicOrdering::Relaxed => "Relaxed",
		AtomicOrdering::Acquire => "Acquire",
		AtomicOrdering::Release => "Release",
		AtomicOrdering::AcquireRelease => "AcquireRelease",
		AtomicOrdering::SequentiallyConsistent => "SequentiallyConsistent",
	} }

fn parse_atomic_ordering(text: &str, path: &str) -> Result<AtomicOrdering, OgdlCodecError> { match text {
		"Relaxed" => Ok(AtomicOrdering::Relaxed),
		"Acquire" => Ok(AtomicOrdering::Acquire),
		"Release" => Ok(AtomicOrdering::Release),
		"AcquireRelease" => Ok(AtomicOrdering::AcquireRelease),
		"SequentiallyConsistent" => Ok(AtomicOrdering::SequentiallyConsistent),
		name => Err(unknown_variant(path, name)), } }

const fn atomic_operation_name(value: AtomicOperation) -> &'static str {
	match value {
		AtomicOperation::Exchange => "Exchange",
		AtomicOperation::Add => "Add",
		AtomicOperation::Minimum => "Minimum",
		AtomicOperation::Maximum => "Maximum",
	} }

fn parse_atomic_operation(text: &str, path: &str) -> Result<AtomicOperation, OgdlCodecError> { match text {
		"Exchange" => Ok(AtomicOperation::Exchange),
		"Add" => Ok(AtomicOperation::Add),
		"Minimum" => Ok(AtomicOperation::Minimum),
		"Maximum" => Ok(AtomicOperation::Maximum),
		name => Err(unknown_variant(path, name)), } }

const fn index_bounds_name(value: IndexBounds) -> &'static str {
	match value {
		IndexBounds::Reject => "Reject",
		IndexBounds::Clamp => "Clamp",
		IndexBounds::Wrap => "Wrap",
	} }

fn parse_index_bounds(text: &str, path: &str) -> Result<IndexBounds, OgdlCodecError> { match text {
		"Reject" => Ok(IndexBounds::Reject),
		"Clamp" => Ok(IndexBounds::Clamp),
		"Wrap" => Ok(IndexBounds::Wrap),
		name => Err(unknown_variant(path, name)), } }

const fn reduce_operator_name(value: ReduceOperator) -> &'static str {
	match value {
		ReduceOperator::Sum => "Sum",
		ReduceOperator::Product => "Product",
		ReduceOperator::Minimum => "Minimum",
		ReduceOperator::Maximum => "Maximum",
		ReduceOperator::Any => "Any",
		ReduceOperator::All => "All",
	} }

fn parse_reduce_operator(text: &str, path: &str) -> Result<ReduceOperator, OgdlCodecError> { match text {
		"Sum" => Ok(ReduceOperator::Sum),
		"Product" => Ok(ReduceOperator::Product),
		"Minimum" => Ok(ReduceOperator::Minimum),
		"Maximum" => Ok(ReduceOperator::Maximum),
		"Any" => Ok(ReduceOperator::Any),
		"All" => Ok(ReduceOperator::All),
		name => Err(unknown_variant(path, name)), } }

const fn reduce_result_name(value: ReduceResult) -> &'static str {
	match value {
		ReduceResult::Value => "Value",
		ReduceResult::Index => "Index",
		ReduceResult::ValueAndIndex => "ValueAndIndex",
	} }

fn parse_reduce_result(text: &str, path: &str) -> Result<ReduceResult, OgdlCodecError> { match text {
		"Value" => Ok(ReduceResult::Value),
		"Index" => Ok(ReduceResult::Index),
		"ValueAndIndex" => Ok(ReduceResult::ValueAndIndex),
		name => Err(unknown_variant(path, name)), } }

const fn sort_direction_name(value: SortDirection) -> &'static str {
	match value {
		SortDirection::Ascending => "Ascending",
		SortDirection::Descending => "Descending",
	} }

fn parse_sort_direction(text: &str, path: &str) -> Result<SortDirection, OgdlCodecError> { match text {
		"Ascending" => Ok(SortDirection::Ascending),
		"Descending" => Ok(SortDirection::Descending),
		name => Err(unknown_variant(path, name)), } }

fn scalar_opcode_name(value: ScalarOpcode) -> Option<&'static str> {
	match value {
		ScalarOpcode::Add => Some("Add"),
		ScalarOpcode::Subtract => Some("Subtract"),
		ScalarOpcode::Multiply => Some("Multiply"),
		ScalarOpcode::Divide => Some("Divide"),
		ScalarOpcode::Remainder => Some("Remainder"),
		ScalarOpcode::Negate => Some("Negate"),
		ScalarOpcode::Absolute => Some("Absolute"),
		ScalarOpcode::Minimum => Some("Minimum"),
		ScalarOpcode::Maximum => Some("Maximum"),
		ScalarOpcode::Fma => Some("Fma"),
		ScalarOpcode::Equal => Some("Equal"),
		ScalarOpcode::NotEqual => Some("NotEqual"),
		ScalarOpcode::LessThan => Some("LessThan"),
		ScalarOpcode::LessThanOrEqual => Some("LessThanOrEqual"),
		ScalarOpcode::GreaterThan => Some("GreaterThan"),
		ScalarOpcode::GreaterThanOrEqual => Some("GreaterThanOrEqual"),
		ScalarOpcode::Select => Some("Select"),
		ScalarOpcode::BitAnd => Some("BitAnd"),
		ScalarOpcode::BitOr => Some("BitOr"),
		ScalarOpcode::BitXor => Some("BitXor"),
		ScalarOpcode::BitNot => Some("BitNot"),
		ScalarOpcode::BitcastF32ToI32 => Some("BitcastF32ToI32"),
		ScalarOpcode::BitcastI32ToF32 => Some("BitcastI32ToF32"),
		ScalarOpcode::ShiftLeft => Some("ShiftLeft"),
		ScalarOpcode::ShiftRightLogical => Some("ShiftRightLogical"),
		ScalarOpcode::ShiftRightArithmetic => Some("ShiftRightArithmetic"),
		ScalarOpcode::Require => Some("Require"),
		ScalarOpcode::IsFinite => Some("IsFinite"),
		ScalarOpcode::IsNan => Some("IsNan"),
		ScalarOpcode::SquareRoot => Some("SquareRoot"),
		ScalarOpcode::Floor => Some("Floor"),
		ScalarOpcode::Ceiling => Some("Ceiling"),
		ScalarOpcode::RoundNearestEven => Some("RoundNearestEven"),
		ScalarOpcode::ConvertF32ToI32 => Some("ConvertF32ToI32"),
		ScalarOpcode::ConvertI32ToF32 => Some("ConvertI32ToF32"),
		_ => None, } }

fn parse_scalar_opcode(text: &str, path: &str) -> Result<ScalarOpcode, OgdlCodecError> { match text {
		"Add" => Ok(ScalarOpcode::Add),
		"Subtract" => Ok(ScalarOpcode::Subtract),
		"Multiply" => Ok(ScalarOpcode::Multiply),
		"Divide" => Ok(ScalarOpcode::Divide),
		"Remainder" => Ok(ScalarOpcode::Remainder),
		"Negate" => Ok(ScalarOpcode::Negate),
		"Absolute" => Ok(ScalarOpcode::Absolute),
		"Minimum" => Ok(ScalarOpcode::Minimum),
		"Maximum" => Ok(ScalarOpcode::Maximum),
		"Fma" => Ok(ScalarOpcode::Fma),
		"Equal" => Ok(ScalarOpcode::Equal),
		"NotEqual" => Ok(ScalarOpcode::NotEqual),
		"LessThan" => Ok(ScalarOpcode::LessThan),
		"LessThanOrEqual" => Ok(ScalarOpcode::LessThanOrEqual),
		"GreaterThan" => Ok(ScalarOpcode::GreaterThan),
		"GreaterThanOrEqual" => Ok(ScalarOpcode::GreaterThanOrEqual),
		"Select" => Ok(ScalarOpcode::Select),
		"BitAnd" => Ok(ScalarOpcode::BitAnd),
		"BitOr" => Ok(ScalarOpcode::BitOr),
		"BitXor" => Ok(ScalarOpcode::BitXor),
		"BitNot" => Ok(ScalarOpcode::BitNot),
		"BitcastF32ToI32" => Ok(ScalarOpcode::BitcastF32ToI32),
		"BitcastI32ToF32" => Ok(ScalarOpcode::BitcastI32ToF32),
		"ShiftLeft" => Ok(ScalarOpcode::ShiftLeft),
		"ShiftRightLogical" => Ok(ScalarOpcode::ShiftRightLogical),
		"ShiftRightArithmetic" => Ok(ScalarOpcode::ShiftRightArithmetic),
		"Require" => Ok(ScalarOpcode::Require),
		"IsFinite" => Ok(ScalarOpcode::IsFinite),
		"IsNan" => Ok(ScalarOpcode::IsNan),
		"SquareRoot" => Ok(ScalarOpcode::SquareRoot),
		"Floor" => Ok(ScalarOpcode::Floor),
		"Ceiling" => Ok(ScalarOpcode::Ceiling),
		"RoundNearestEven" => Ok(ScalarOpcode::RoundNearestEven),
		"ConvertF32ToI32" => Ok(ScalarOpcode::ConvertF32ToI32),
		"ConvertI32ToF32" => Ok(ScalarOpcode::ConvertI32ToF32),
		name => Err(unknown_variant(path, name)), } }
