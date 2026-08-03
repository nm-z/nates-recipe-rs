use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{
	AliasPermission, DType, FlopCount, KernelTemplateId, ScalarLiteral, ScalarOpcode, ScalarProgram, ValueId, };

use crate::{AxisSet, LanguageError, LanguageErrorKind, LanguageResult, Shape, Tensor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicOrdering { Relaxed, Acquire, Release, AcquireRelease, SequentiallyConsistent, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicOperation { Exchange, Add, Minimum, Maximum, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexBounds {
	/// An out-of-range index reports a device fault through the preallocated
	/// fault channel. No unchecked memory access is emitted.
	Reject, Clamp, Wrap, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReduceOperator { Sum, Product, Minimum, Maximum, Any, All, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReduceResult { Value, Index, ValueAndIndex, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elementwise { pub program: ScalarProgram, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reduce { pub operator: ReduceOperator, pub axes: AxisSet, pub keep_dimensions: bool,
	pub result: ReduceResult,
	/// Power-of-two lane count fixes the reduction tree and therefore its
	/// floating-point operation order.
	pub tree_lanes: u32, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanMode { Inclusive, Exclusive { identity: ScalarLiteral }, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scan { pub operator: ReduceOperator, pub axis: usize, pub mode: ScanMode, pub reverse: bool,
	pub tree_lanes: u32, }

/// Generalized tensor contraction. Axis pairs are ordered and form the leading
/// batch dimensions followed by the left and right free dimensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contraction { pub batch_axes: Vec<(usize, usize)>, pub contract_axes: Vec<(usize, usize)>, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gather { pub axis: usize, pub bounds: IndexBounds, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScatterConflict { UniqueIndices, Atomic { operation: AtomicOperation, ordering: AtomicOrdering, }, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scatter { pub axis: usize, pub bounds: IndexBounds, pub conflict: ScatterConflict, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Histogram { pub bins: u32, pub weighted: bool, pub ordering: AtomicOrdering, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection { Ascending, Descending, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sort { pub axis: usize, pub direction: SortDirection, pub stable: bool, pub emit_indices: bool, }

/// Iteration-aware affine int32 source.
///
/// Lane `element_index` evaluates
/// `start + element_index * element_step + loop_iteration * iteration_step`
/// through checked signed-64 intermediates. Without a modulus, the result must
/// also fit int32. With a modulus, Recipe applies Euclidean remainder using the
/// strictly positive modulus before the checked int32 result is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexMap { pub start: i32, pub element_step: i32, pub iteration_step: i32, pub modulus: Option<i32>, }

/// Counter key material. The runtime additionally folds in the immutable
/// `RunId`, loop iteration, kernel ID, and linear element index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RandomKey { pub seed_low: u64, pub seed_high: u64, pub stream: u64, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomDistribution { UniformF32, NormalF32, BernoulliI32 { probability_bits: u32 },
	UniformI32 { low: i32, high_exclusive: i32 }, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RandomMap { pub distribution: RandomDistribution, pub key: RandomKey,
	/// Recipe-owned Philox uses ten rounds. This is explicit so an artifact
	/// cannot silently select a backend RNG implementation.
	pub philox_rounds: u8, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveKind { Elementwise(Elementwise), Reduce(Reduce), Scan(Scan), Contraction(Contraction), Gather(Gather),
	Scatter(Scatter), Histogram(Histogram), Sort(Sort), IndexMap(IndexMap), Random(RandomMap), }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveAliasRule { pub input: usize, pub output: usize, pub permission: AliasPermission, }

/// One placement-free primitive calculation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitiveKernel { pub id: KernelTemplateId, pub inputs: Vec<ValueId>, pub outputs: Vec<ValueId>,
	pub alias_rules: Vec<PrimitiveAliasRule>, pub kind: PrimitiveKind, }

impl PrimitiveKernel { pub fn validate(&self, tensors: &BTreeMap<ValueId, &Tensor>) -> LanguageResult<()> {
		let inputs = self .inputs .iter() .map(|id| { tensors.get(id).copied().ok_or_else(|| { LanguageError::new(
						LanguageErrorKind::UnknownTensor,
						format!("input tensor {id} does not exist"),
					) .for_kernel(self.id) .for_value(*id) }) }) .collect::<LanguageResult<Vec<_>>>()?; let outputs = self .outputs
			.iter() .map(|id| { tensors.get(id).copied().ok_or_else(|| { LanguageError::new( LanguageErrorKind::UnknownTensor,
						format!("output tensor {id} does not exist"),
					) .for_kernel(self.id) .for_value(*id) }) }) .collect::<LanguageResult<Vec<_>>>()?;
		self.validate_alias_matrix(inputs.len(), outputs.len())?;

		match &self.kind { PrimitiveKind::Elementwise(spec) => validate_elementwise(self.id, spec, &inputs, &outputs),
			PrimitiveKind::Reduce(spec) => validate_reduce(self.id, spec, &inputs, &outputs),
			PrimitiveKind::Scan(spec) => validate_scan(self.id, spec, &inputs, &outputs),
			PrimitiveKind::Contraction(spec) => validate_contraction(self.id, spec, &inputs, &outputs),
			PrimitiveKind::Gather(spec) => validate_gather(self.id, *spec, &inputs, &outputs),
			PrimitiveKind::Scatter(spec) => validate_scatter(self.id, *spec, &inputs, &outputs),
			PrimitiveKind::Histogram(spec) => validate_histogram(self.id, *spec, &inputs, &outputs),
			PrimitiveKind::Sort(spec) => validate_sort(self.id, *spec, &inputs, &outputs),
			PrimitiveKind::IndexMap(spec) => validate_index_map(self.id, *spec, &inputs, &outputs),
			PrimitiveKind::Random(spec) => validate_random(self.id, *spec, &inputs, &outputs), } }

	pub fn work(&self, tensors: &BTreeMap<ValueId, &Tensor>) -> LanguageResult<FlopCount> { self.validate(tensors)?;
		let tensor = |id: ValueId| { tensors .get(&id) .copied()
				.expect("validation established every tensor")
		}; let work = match &self.kind { PrimitiveKind::Elementwise(spec) => {
				let elements = tensor(self.outputs[0]).shape.elements(); let per_element = spec .program .instructions .iter()
					.try_fold(0_u64, |total, instruction| { total.checked_add(scalar_work(instruction.opcode)) })
					.ok_or_else(|| work_overflow(self.id))?; elements.checked_mul(per_element) }
			PrimitiveKind::Reduce(spec) => { let input = tensor(self.inputs[0]).shape.elements();
				let output = tensor(self.outputs[0]).shape.elements(); let combine = input.saturating_sub(output);
				let multiplier = if spec.result == ReduceResult::ValueAndIndex { 2 } else { 1 }; combine.checked_mul(multiplier) }
			PrimitiveKind::Scan(_) => tensor(self.inputs[0]).shape.elements().checked_mul(2),
			PrimitiveKind::Contraction(spec) => { let left = tensor(self.inputs[0]); let contracted = spec .contract_axes .iter()
					.try_fold(1_u64, |count, (axis, _)| { count.checked_mul(left.shape.extents()[*axis]) }); contracted
					.and_then(|count| count.checked_mul(tensor(self.outputs[0]).shape.elements()))
					.and_then(|count| count.checked_mul(2)) }
			PrimitiveKind::Gather(_) | PrimitiveKind::Scatter(_) => { Some(tensor(self.outputs[0]).shape.elements()) }
			PrimitiveKind::Histogram(_) => Some(tensor(self.inputs[0]).shape.elements()), PrimitiveKind::Sort(spec) => {
				let input = tensor(self.inputs[0]); let axis = input.shape.extents()[spec.axis]; let comparisons = if axis <= 1 { 0
				} else { u64::from(u64::BITS - (axis - 1).leading_zeros()) .checked_mul(axis)
						.ok_or_else(|| work_overflow(self.id))? }; let slices = input.shape.elements().checked_div(axis).unwrap_or(0);
				comparisons.checked_mul(slices) }
			PrimitiveKind::IndexMap(_) => Some(0), PrimitiveKind::Random(spec) => { let rounds = u64::from(spec.philox_rounds);
				tensor(self.outputs[0]) .shape .elements() .checked_mul(rounds) .and_then(|value| value.checked_mul(4)) } }
		.ok_or_else(|| work_overflow(self.id))?; Ok(FlopCount::new(work)) }

	fn validate_alias_matrix(&self, input_count: usize, output_count: usize) -> LanguageResult<()> {
		let mut pairs = BTreeSet::new(); for rule in &self.alias_rules {
			if rule.input >= input_count || rule.output >= output_count { return Err(LanguageError::new(
					LanguageErrorKind::InvalidPrimitive, format!(
						"alias pair ({}, {}) is outside {input_count} inputs and {output_count} outputs",
						rule.input, rule.output ), ) .for_kernel(self.id)); }
			if !pairs.insert((rule.input, rule.output)) { return Err(LanguageError::new( LanguageErrorKind::InvalidPrimitive,
					format!(
						"alias pair ({}, {}) appears more than once",
						rule.input, rule.output ), ) .for_kernel(self.id)); } }
		if pairs.len() != input_count.saturating_mul(output_count) { return Err(LanguageError::new(
				LanguageErrorKind::InvalidPrimitive,
				"every input/output pair requires an explicit alias rule",
			) .for_kernel(self.id)); }
		Ok(()) } }

fn validate_elementwise( id: KernelTemplateId, spec: &Elementwise, inputs: &[&Tensor], outputs: &[&Tensor],
) -> LanguageResult<()> { spec.program.validate().map_err(|errors| {
		LanguageError::new(LanguageErrorKind::InvalidScalarProgram, errors.to_string()).for_kernel(id) })?; require_arity( id,
		"elementwise input",
		inputs.len(), spec.program.inputs.len(), )?; require_arity( id,
		"elementwise output",
		outputs.len(), spec.program.outputs.len(), )?; if inputs.is_empty() { return Err(LanguageError::new(
			LanguageErrorKind::ArityMismatch,
			"constant elementwise maps use Random or an explicit filled input",
		) .for_kernel(id)); }
	for (index, (tensor, scalar)) in inputs.iter().zip(&spec.program.inputs).enumerate() {
		if tensor.dtype != scalar.dtype { return Err(dtype_error( id, format!(
					"input {index} is {:?}, scalar input is {:?}",
					tensor.dtype, scalar.dtype ), )); } }
	let shape = Shape::broadcast_result( &inputs .iter() .map(|tensor| &tensor.shape) .collect::<Vec<_>>(), )
	.map_err(|error| error.for_kernel(id))?;
	for (index, (tensor, scalar)) in outputs.iter().zip(&spec.program.outputs).enumerate() {
		let expected_dtype = spec.program.dtype_of(*scalar); if expected_dtype != Some(tensor.dtype) { return Err(dtype_error(
				id, format!(
					"output {index} is {:?}, scalar output is {expected_dtype:?}",
					tensor.dtype ), )); }
		require_shape( id,
			&format!("elementwise output {index}"),
			&shape, &tensor.shape, )?; }
	Ok(()) }

fn validate_reduce(id: KernelTemplateId, spec: &Reduce, inputs: &[&Tensor], outputs: &[&Tensor]) -> LanguageResult<()> {
	require_arity(id, "reduction input", inputs.len(), 1)?;
	let expected_outputs = match spec.result { ReduceResult::Value | ReduceResult::Index => 1,
		ReduceResult::ValueAndIndex => 2, };
	require_arity(id, "reduction output", outputs.len(), expected_outputs)?;
	validate_tree(id, spec.tree_lanes)?; spec.axes .validate_rank(inputs[0].shape.rank())
		.map_err(|error| error.for_kernel(id))?;
	if matches!(spec.operator, ReduceOperator::Any | ReduceOperator::All) && inputs[0].dtype != DType::I32 {
		return Err(dtype_error(id, "Any and All require int32 truth values"));
	}
	if matches!( spec.result, ReduceResult::Index | ReduceResult::ValueAndIndex ) && !matches!( spec.operator,
		ReduceOperator::Minimum | ReduceOperator::Maximum ) { return Err(LanguageError::new(
			LanguageErrorKind::InvalidPrimitive,
			"index reductions require Minimum or Maximum",
		) .for_kernel(id)); }
	let reduced = inputs[0] .shape .reduced(&spec.axes, spec.keep_dimensions) .map_err(|error| error.for_kernel(id))?;
	match spec.result { ReduceResult::Value => {
			require_dtype(id, "reduction value", inputs[0].dtype, outputs[0].dtype)?;
			require_shape(id, "reduction value", &reduced, &outputs[0].shape)?;
		}
		ReduceResult::Index => {
			require_dtype(id, "reduction index", DType::I32, outputs[0].dtype)?;
			require_shape(id, "reduction index", &reduced, &outputs[0].shape)?;
		}
		ReduceResult::ValueAndIndex => {
			require_dtype(id, "reduction value", inputs[0].dtype, outputs[0].dtype)?;
			require_dtype(id, "reduction index", DType::I32, outputs[1].dtype)?;
			require_shape(id, "reduction value", &reduced, &outputs[0].shape)?;
			require_shape(id, "reduction index", &reduced, &outputs[1].shape)?;
		} }
	let empty_axis = spec .axes .as_slice() .iter() .any(|axis| inputs[0].shape.extents()[*axis] == 0); if empty_axis
		&& matches!( spec.operator, ReduceOperator::Minimum | ReduceOperator::Maximum ) { return Err(LanguageError::new(
			LanguageErrorKind::InvalidPrimitive,
			"Minimum and Maximum have no implicit empty-domain identity",
		) .for_kernel(id)); }
	Ok(()) }

fn validate_scan(id: KernelTemplateId, spec: &Scan, inputs: &[&Tensor], outputs: &[&Tensor]) -> LanguageResult<()> {
	require_arity(id, "scan input", inputs.len(), 1)?;
	require_arity(id, "scan output", outputs.len(), 1)?;
	if spec.axis >= inputs[0].shape.rank() { return Err(LanguageError::new( LanguageErrorKind::InvalidAxis,
			format!("scan axis {} is outside input rank", spec.axis),
		) .for_kernel(id)); }
	validate_tree(id, spec.tree_lanes)?;
	if matches!(spec.operator, ReduceOperator::Any | ReduceOperator::All) && inputs[0].dtype != DType::I32 {
		return Err(dtype_error(id, "Any and All scans require int32"));
	}
	if let ScanMode::Exclusive { identity } = spec.mode && identity.dtype() != inputs[0].dtype
	{ return Err(dtype_error( id,
			"exclusive-scan identity type differs from its input",
		)); }
	require_dtype(id, "scan output", inputs[0].dtype, outputs[0].dtype)?;
	require_shape(id, "scan output", &inputs[0].shape, &outputs[0].shape)
}

fn validate_contraction( id: KernelTemplateId, spec: &Contraction, inputs: &[&Tensor], outputs: &[&Tensor],
) -> LanguageResult<()> {
	require_arity(id, "contraction input", inputs.len(), 2)?;
	require_arity(id, "contraction output", outputs.len(), 1)?;
	if spec.contract_axes.is_empty() { return Err(LanguageError::new( LanguageErrorKind::InvalidPrimitive,
			"contraction requires at least one contracted axis pair",
		) .for_kernel(id)); }
	require_dtype(id, "contraction operands", inputs[0].dtype, inputs[1].dtype)?;
	require_dtype(id, "contraction output", inputs[0].dtype, outputs[0].dtype)?;
	let left_rank = inputs[0].shape.rank(); let right_rank = inputs[1].shape.rank(); let mut used_left = BTreeSet::new();
	let mut used_right = BTreeSet::new(); let mut output_extents = Vec::new(); for (class, pairs) in [
		("batch", &spec.batch_axes),
		("contract", &spec.contract_axes),
	] { for (left, right) in pairs { if *left >= left_rank || *right >= right_rank { return Err(LanguageError::new(
					LanguageErrorKind::InvalidAxis,
					format!("{class} axis pair ({left}, {right}) exceeds ranks ({left_rank}, {right_rank})"),
				) .for_kernel(id)); }
			if !used_left.insert(*left) || !used_right.insert(*right) { return Err(LanguageError::new(
					LanguageErrorKind::DuplicateAxis,
					format!("{class} axis pair reuses an operand axis"),
				) .for_kernel(id)); }
			let left_extent = inputs[0].shape.extents()[*left]; let right_extent = inputs[1].shape.extents()[*right];
			if left_extent != right_extent { return Err(LanguageError::new( LanguageErrorKind::ShapeMismatch,
					format!("{class} axes ({left}, {right}) have extents {left_extent} and {right_extent}"),
				) .for_kernel(id)); }
			if class == "batch" {
				output_extents.push(left_extent); } } }
	for (axis, extent) in inputs[0].shape.extents().iter().copied().enumerate() { if !used_left.contains(&axis) {
			output_extents.push(extent); } }
	for (axis, extent) in inputs[1].shape.extents().iter().copied().enumerate() { if !used_right.contains(&axis) {
			output_extents.push(extent); } }
	if output_extents.is_empty() { output_extents.push(1); }
	let expected = Shape::new(output_extents).map_err(|error| error.for_kernel(id))?;
	require_shape(id, "contraction output", &expected, &outputs[0].shape)
}

fn validate_gather(id: KernelTemplateId, spec: Gather, inputs: &[&Tensor], outputs: &[&Tensor]) -> LanguageResult<()> {
	require_arity(id, "gather input", inputs.len(), 2)?;
	require_arity(id, "gather output", outputs.len(), 1)?;
	require_dtype(id, "gather indices", DType::I32, inputs[1].dtype)?;
	require_dtype(id, "gather output", inputs[0].dtype, outputs[0].dtype)?;
	let expected = inputs[0] .shape .gather_result(spec.axis, &inputs[1].shape) .map_err(|error| error.for_kernel(id))?;
	require_shape(id, "gather output", &expected, &outputs[0].shape)
}

fn validate_scatter( id: KernelTemplateId, spec: Scatter, inputs: &[&Tensor], outputs: &[&Tensor],
) -> LanguageResult<()> {
	require_arity(id, "scatter input", inputs.len(), 3)?;
	require_arity(id, "scatter output", outputs.len(), 1)?;
	require_dtype(id, "scatter indices", DType::I32, inputs[1].dtype)?;
	require_dtype(id, "scatter updates", inputs[0].dtype, inputs[2].dtype)?;
	require_dtype(id, "scatter output", inputs[0].dtype, outputs[0].dtype)?;
	require_shape(id, "scatter output", &inputs[0].shape, &outputs[0].shape)?;
	let updates = inputs[0] .shape .gather_result(spec.axis, &inputs[1].shape) .map_err(|error| error.for_kernel(id))?;
	require_shape(id, "scatter updates", &updates, &inputs[2].shape)?;
	// The selected lowering must preserve this exact conflict operation and
	// ordering. All five orderings are meaningful for an atomic RMW.
	Ok(()) }

fn validate_histogram( id: KernelTemplateId, spec: Histogram, inputs: &[&Tensor], outputs: &[&Tensor],
) -> LanguageResult<()> { let expected_inputs = if spec.weighted { 2 } else { 1 };
	require_arity(id, "histogram input", inputs.len(), expected_inputs)?;
	require_arity(id, "histogram output", outputs.len(), 1)?;
	if spec.bins == 0 || spec.bins > i32::MAX as u32 { return Err(LanguageError::new( LanguageErrorKind::InvalidPrimitive,
			"histogram bin count must be in 1..=i32::MAX",
		) .for_kernel(id)); }
	let expected_dtype = if spec.weighted {
		require_dtype(id, "histogram weights", DType::F32, inputs[1].dtype)?;
		require_shape(id, "histogram weights", &inputs[0].shape, &inputs[1].shape)?;
		DType::F32 } else { DType::I32 };
	require_dtype(id, "histogram output", expected_dtype, outputs[0].dtype)?;
	let expected = Shape::new(vec![u64::from(spec.bins)]).expect("nonzero bin count");
	require_shape(id, "histogram output", &expected, &outputs[0].shape)
}

fn validate_sort(id: KernelTemplateId, spec: Sort, inputs: &[&Tensor], outputs: &[&Tensor]) -> LanguageResult<()> {
	require_arity(id, "sort input", inputs.len(), 1)?;
	require_arity( id,
		"sort output",
		outputs.len(), if spec.emit_indices { 2 } else { 1 }, )?; if spec.axis >= inputs[0].shape.rank() {
		return Err(LanguageError::new( LanguageErrorKind::InvalidAxis,
			format!("sort axis {} is outside input rank", spec.axis),
		) .for_kernel(id)); }
	if inputs[0].shape.extents()[spec.axis] > i32::MAX as u64 { return Err(LanguageError::new(
			LanguageErrorKind::InvalidPrimitive,
			"sort axis cannot be represented by int32 result indices",
		) .for_kernel(id)); }
	require_dtype(id, "sort values", inputs[0].dtype, outputs[0].dtype)?;
	require_shape(id, "sort values", &inputs[0].shape, &outputs[0].shape)?;
	if spec.emit_indices {
		require_dtype(id, "sort indices", DType::I32, outputs[1].dtype)?;
		require_shape(id, "sort indices", &inputs[0].shape, &outputs[1].shape)?;
	}
	Ok(()) }

fn validate_index_map( id: KernelTemplateId, spec: IndexMap, inputs: &[&Tensor], outputs: &[&Tensor],
) -> LanguageResult<()> {
	require_arity(id, "index-map input", inputs.len(), 0)?;
	require_arity(id, "index-map output", outputs.len(), 1)?;
	require_dtype(id, "index-map output", DType::I32, outputs[0].dtype)?;
	if spec.modulus.is_some_and(|modulus| modulus <= 0) { return Err(LanguageError::new(
			LanguageErrorKind::InvalidPrimitive,
			"index-map modulus must be strictly positive when present",
		) .for_kernel(id)); }
	Ok(()) }

fn validate_random( id: KernelTemplateId, spec: RandomMap, inputs: &[&Tensor], outputs: &[&Tensor],
) -> LanguageResult<()> {
	require_arity(id, "random input", inputs.len(), 0)?;
	require_arity(id, "random output", outputs.len(), 1)?;
	if spec.philox_rounds != 10 { return Err(LanguageError::new( LanguageErrorKind::InvalidPrimitive,
			"Recipe random maps require exactly Philox4x32-10",
		) .for_kernel(id)); }
	let dtype = match spec.distribution { RandomDistribution::UniformF32 | RandomDistribution::NormalF32 => DType::F32,
		RandomDistribution::BernoulliI32 { probability_bits } => { let probability = f32::from_bits(probability_bits);
			if !(probability.is_finite() && (0.0..=1.0).contains(&probability)) { return Err(LanguageError::new(
					LanguageErrorKind::InvalidPrimitive,
					"Bernoulli probability must be a finite f32 in [0, 1]",
				) .for_kernel(id)); }
			DType::I32 }
		RandomDistribution::UniformI32 { low, high_exclusive, } => { if low >= high_exclusive { return Err(LanguageError::new(
					LanguageErrorKind::InvalidPrimitive,
					"uniform int32 range must have low < high_exclusive",
				) .for_kernel(id)); }
			DType::I32 } };
	require_dtype(id, "random output", dtype, outputs[0].dtype)
}

fn validate_tree(id: KernelTemplateId, lanes: u32) -> LanguageResult<()> {
	if lanes == 0 || lanes > 1024 || !lanes.is_power_of_two() { return Err(LanguageError::new(
			LanguageErrorKind::InvalidPrimitive,
			"fixed tree lane count must be a power of two in 1..=1024",
		) .for_kernel(id)); }
	Ok(()) }

fn require_arity(id: KernelTemplateId, name: &str, actual: usize, expected: usize) -> LanguageResult<()> {
	if actual == expected { Ok(()) } else { Err(LanguageError::new( LanguageErrorKind::ArityMismatch,
			format!("{name} arity is {actual}, expected {expected}"),
		) .for_kernel(id)) } }

fn require_dtype(id: KernelTemplateId, name: &str, expected: DType, actual: DType) -> LanguageResult<()> {
	if expected == actual { Ok(()) } else { Err(dtype_error( id,
			format!("{name} is {actual:?}, expected {expected:?}"),
		)) } }

fn require_shape(id: KernelTemplateId, name: &str, expected: &Shape, actual: &Shape) -> LanguageResult<()> {
	if expected == actual { Ok(()) } else { Err(LanguageError::new( LanguageErrorKind::ShapeMismatch, format!(
				"{name} has shape {:?}, expected {:?}",
				actual.extents(), expected.extents() ), ) .for_kernel(id)) } }

fn dtype_error(id: KernelTemplateId, detail: impl Into<String>) -> LanguageError { LanguageError::new(LanguageErrorKind::DTypeMismatch, detail).for_kernel(id) }

fn work_overflow(id: KernelTemplateId) -> LanguageError { LanguageError::new( LanguageErrorKind::WorkOverflow,
		"primitive work count overflowed u64",
	) .for_kernel(id) }

const fn scalar_work(opcode: ScalarOpcode) -> u64 { opcode.flops() }
