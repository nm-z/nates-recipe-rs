use recipe_core::{AliasPermission, DType, ScalarOpcode, ScalarProgram, ValueId}; use recipe_language::{
	LanguageError, LanguageErrorKind, LanguageResult, PrimitiveAliasRule, ScalarProgramBuilder, Shape, }; use recipe_ops::{
	OperationError, canonical_elu_program, canonical_leaky_relu_program, canonical_prelu_program, canonical_selu_program,
};

use crate::DenseActivation;

/// Lowered implementation selected for one dense activation.
pub enum ForwardActivation {
	/// Preserve the input value unchanged.
	Identity,
	/// Invoke the named Recipe-owned operation.
	Owned(&'static str),
	/// Execute the supplied scalar program.
	Program(ScalarProgram),
	/// Invoke an operation that preserves the input sign separately.
	SignedMagnitude(&'static str),
	/// Execute the canonical parametric `ReLU` scalar program.
	PRelu(ScalarProgram), }

/// Parameter values for one recurrent gate.
#[derive(Clone, Copy, Debug)]
pub struct RecurrentGateParameters {
	/// Input projection weight.
	pub input_weight: ValueId,
	/// Recurrent hidden-state projection weight.
	pub recurrent_weight: ValueId,
	/// Gate bias.
	pub bias: ValueId, }

/// Parameter values for all GRU gates.
#[derive(Clone, Copy, Debug)]
pub struct GruForwardParameters {
	/// Reset-gate parameters.
	pub reset: RecurrentGateParameters,
	/// Update-gate parameters.
	pub update: RecurrentGateParameters,
	/// Candidate-state parameters.
	pub candidate: RecurrentGateParameters, }

/// Parameter values for all LSTM gates.
#[derive(Clone, Copy, Debug)]
pub struct LstmForwardParameters {
	/// Input-gate parameters.
	pub input: RecurrentGateParameters,
	/// Forget-gate parameters.
	pub forget: RecurrentGateParameters,
	/// Output-gate parameters.
	pub output: RecurrentGateParameters,
	/// Candidate-cell parameters.
	pub candidate: RecurrentGateParameters, }

/// Intermediate values produced by one simple RNN step.
#[derive(Clone, Copy, Debug)]
pub struct RnnStepValues {
	/// Input value for this step.
	pub input: ValueId,
	/// Hidden state from the preceding step.
	pub previous_hidden: ValueId,
	/// Combined affine projection before activation.
	pub preactivation: ValueId,
	/// Activated hidden state produced by this step.
	pub hidden: ValueId, }

/// Intermediate values produced by one GRU step.
#[derive(Clone, Copy, Debug)]
pub struct GruStepValues {
	/// Input value for this step.
	pub input: ValueId,
	/// Hidden state from the preceding step.
	pub previous_hidden: ValueId,
	/// Reset-gate value before activation.
	pub reset_preactivation: ValueId,
	/// Activated reset gate.
	pub reset: ValueId,
	/// Update-gate value before activation.
	pub update_preactivation: ValueId,
	/// Activated update gate.
	pub update: ValueId,
	/// Previous hidden state gated by the reset value.
	pub reset_hidden: ValueId,
	/// Candidate-state value before activation.
	pub candidate_preactivation: ValueId,
	/// Activated candidate state.
	pub candidate: ValueId, }

/// Intermediate values produced by one LSTM step.
#[derive(Clone, Copy, Debug)]
pub struct LstmStepValues {
	/// Input value for this step.
	pub input: ValueId,
	/// Hidden state from the preceding step.
	pub previous_hidden: ValueId,
	/// Cell state from the preceding step.
	pub previous_cell: ValueId,
	/// Input-gate value before activation.
	pub input_gate_preactivation: ValueId,
	/// Activated input gate.
	pub input_gate: ValueId,
	/// Forget-gate value before activation.
	pub forget_gate_preactivation: ValueId,
	/// Activated forget gate.
	pub forget_gate: ValueId,
	/// Output-gate value before activation.
	pub output_gate_preactivation: ValueId,
	/// Activated output gate.
	pub output_gate: ValueId,
	/// Candidate-cell value before activation.
	pub candidate_preactivation: ValueId,
	/// Activated candidate cell.
	pub candidate: ValueId,
	/// Updated cell state.
	pub cell: ValueId,
	/// Activated updated cell state.
	pub cell_activation: ValueId, }

/// Minimal graph-emission boundary shared by training, validation, and loaded
/// model inference. Parameter sourcing and gradient construction stay outside
/// this interface; it owns only the exact forward recurrent equations.
pub trait RecurrentForwardGraph {
	/// Error returned while emitting a forward graph.
	type Error: From<LanguageError>;

	/// Emit a zero-filled `f32` value with `shape`.
	fn zero_f32(&mut self, shape: Shape) -> Result<ValueId, Self::Error>;

	/// Gather one column from a row-major matrix.
	fn gather_matrix_column( &mut self, matrix: ValueId, rows: u64, columns: u64, column: u64,
	) -> Result<ValueId, Self::Error>;

	/// Emit a bias-free linear projection.
	fn bias_free_linear( &mut self, input: ValueId, weight: ValueId, rows: u64, output_width: u64,
	) -> Result<ValueId, Self::Error>;

	/// Apply a scalar program elementwise to the supplied inputs.
	fn elementwise_f32( &mut self, shape: Shape, inputs: Vec<ValueId>, program: ScalarProgram,
	) -> Result<ValueId, Self::Error>;

	/// Apply one dense activation to an input value.
	fn activate(&mut self, input: ValueId, activation: DenseActivation, shape: Shape)
	-> Result<ValueId, Self::Error>; }

/// Emit a complete simple RNN sequence and retain each step's intermediate values.
///
/// # Errors
///
/// Returns an error when the graph cannot emit a required value or operation.
pub fn lower_rnn_sequence<G: RecurrentForwardGraph>( graph: &mut G, input: ValueId, rows: u64, sequence: u64,
	width: u64, parameters: RecurrentGateParameters, ) -> Result<(ValueId, Vec<RnnStepValues>), G::Error> {
	let mut hidden = graph.zero_f32(Shape::new(vec![rows, width])?)?; let mut steps = Vec::new(); for step in 0..sequence {
		let step_input = graph.gather_matrix_column(input, rows, sequence, step)?; let previous_hidden = hidden;
		let (preactivation, next_hidden) = lower_recurrent_gate( graph, step_input, previous_hidden, rows, width, parameters,
			DenseActivation::Tanh, )?; steps.push(RnnStepValues { input: step_input, previous_hidden, preactivation,
			hidden: next_hidden, }); hidden = next_hidden; }
	return Ok((hidden, steps)); }

/// Emit a complete GRU sequence and retain each step's intermediate values.
///
/// # Errors
///
/// Returns an error when the graph cannot emit a required value or operation.
pub fn lower_gru_sequence<G: RecurrentForwardGraph>( graph: &mut G, input: ValueId, rows: u64, sequence: u64,
	width: u64, parameters: GruForwardParameters, ) -> Result<(ValueId, Vec<GruStepValues>), G::Error> {
	let mut hidden = graph.zero_f32(Shape::new(vec![rows, width])?)?; let mut steps = Vec::new(); for step in 0..sequence {
		let step_input = graph.gather_matrix_column(input, rows, sequence, step)?; let previous_hidden = hidden;
		let hidden_shape = Shape::new(vec![rows, width])?; let (reset_preactivation, reset) = lower_recurrent_gate( graph,
			step_input, previous_hidden, rows, width, parameters.reset, DenseActivation::Sigmoid, )?;
		let (update_preactivation, update) = lower_recurrent_gate( graph, step_input, previous_hidden, rows, width,
			parameters.update, DenseActivation::Sigmoid, )?; let reset_hidden = graph.elementwise_f32( hidden_shape.clone(),
			vec![reset, previous_hidden], multiply_program()?, )?; let candidate_input_projection =
			graph.bias_free_linear(step_input, parameters.candidate.input_weight, rows, width)?;
		let candidate_recurrent_projection = graph.bias_free_linear( reset_hidden, parameters.candidate.recurrent_weight,
			rows, width, )?; let candidate_preactivation = graph.elementwise_f32( hidden_shape.clone(), vec![
				candidate_input_projection, candidate_recurrent_projection, parameters.candidate.bias, ], sum_program(3)?, )?;
		let candidate = graph.activate( candidate_preactivation, DenseActivation::Tanh, hidden_shape.clone(), )?;
		let hidden_program = { let mut program_builder = ScalarProgramBuilder::new()?;
			let program_candidate = program_builder.input(DType::F32)?; let program_update = program_builder.input(DType::F32)?;
			let program_previous = program_builder.input(DType::F32)?; let difference =
				program_builder.binary(ScalarOpcode::Subtract, program_previous, program_candidate)?;
			let retained = program_builder.binary(ScalarOpcode::Multiply, program_update, difference)?;
			let interpolated = program_builder.binary(ScalarOpcode::Add, program_candidate, retained)?;
			program_builder.finish(&[interpolated])? }; let next_hidden = graph.elementwise_f32( hidden_shape,
			vec![candidate, update, previous_hidden], hidden_program, )?; steps.push(GruStepValues { input: step_input,
			previous_hidden, reset_preactivation, reset, update_preactivation, update, reset_hidden, candidate_preactivation,
			candidate, }); hidden = next_hidden; }
	return Ok((hidden, steps)); }

/// Emit a complete LSTM sequence and retain each step's intermediate values.
///
/// # Errors
///
/// Returns an error when the graph cannot emit a required value or operation.
pub fn lower_lstm_sequence<G: RecurrentForwardGraph>( graph: &mut G, input: ValueId, rows: u64, sequence: u64,
	width: u64, parameters: LstmForwardParameters, ) -> Result<(ValueId, Vec<LstmStepValues>), G::Error> {
	let state_shape = Shape::new(vec![rows, width])?; let mut hidden = graph.zero_f32(state_shape.clone())?;
	let mut cell = graph.zero_f32(state_shape)?; let mut steps = Vec::new(); for step in 0..sequence {
		let step_input = graph.gather_matrix_column(input, rows, sequence, step)?; let previous_hidden = hidden;
		let previous_cell = cell; let (input_gate_preactivation, input_gate) = lower_recurrent_gate( graph, step_input,
			previous_hidden, rows, width, parameters.input, DenseActivation::Sigmoid, )?;
		let (forget_gate_preactivation, forget_gate) = lower_recurrent_gate( graph, step_input, previous_hidden, rows, width,
			parameters.forget, DenseActivation::Sigmoid, )?; let (output_gate_preactivation, output_gate) = lower_recurrent_gate(
			graph, step_input, previous_hidden, rows, width, parameters.output, DenseActivation::Sigmoid, )?;
		let (candidate_preactivation, candidate) = lower_recurrent_gate( graph, step_input, previous_hidden, rows, width,
			parameters.candidate, DenseActivation::Tanh, )?; let step_shape = Shape::new(vec![rows, width])?;
		let cell_program = { let mut program_builder = ScalarProgramBuilder::new()?;
			let program_forget = program_builder.input(DType::F32)?;
			let program_previous_cell = program_builder.input(DType::F32)?;
			let program_input = program_builder.input(DType::F32)?; let program_candidate = program_builder.input(DType::F32)?;
			let retained = program_builder.binary(ScalarOpcode::Multiply, program_forget, program_previous_cell)?; let admitted =
				program_builder.binary(ScalarOpcode::Multiply, program_input, program_candidate)?;
			let updated_cell = program_builder.binary(ScalarOpcode::Add, retained, admitted)?;
			program_builder.finish(&[updated_cell])? }; let next_cell = graph.elementwise_f32( step_shape.clone(),
			vec![forget_gate, previous_cell, input_gate, candidate], cell_program, )?;
		let cell_activation = graph.activate(next_cell, DenseActivation::Tanh, step_shape.clone())?;
		let next_hidden = graph.elementwise_f32( step_shape, vec![output_gate, cell_activation], multiply_program()?, )?;
		steps.push(LstmStepValues { input: step_input, previous_hidden, previous_cell, input_gate_preactivation, input_gate,
			forget_gate_preactivation, forget_gate, output_gate_preactivation, output_gate, candidate_preactivation, candidate,
			cell: next_cell, cell_activation, }); hidden = next_hidden; cell = next_cell; }
	return Ok((hidden, steps)); }

/// Emit the affine projection and activation shared by recurrent gates.
///
/// # Errors
///
/// Returns an error when the graph cannot emit a projection, sum, or activation.
fn lower_recurrent_gate<G: RecurrentForwardGraph>( graph: &mut G, input: ValueId, previous_hidden: ValueId, rows: u64,
	width: u64, parameters: RecurrentGateParameters, activation: DenseActivation,
) -> Result<(ValueId, ValueId), G::Error> {
	let input_projection = graph.bias_free_linear(input, parameters.input_weight, rows, width)?;
	let recurrent_projection = graph.bias_free_linear(previous_hidden, parameters.recurrent_weight, rows, width)?;
	let shape = Shape::new(vec![rows, width])?; let preactivation = graph.elementwise_f32( shape.clone(),
		vec![input_projection, recurrent_projection, parameters.bias], sum_program(3)?, )?;
	let activated = graph.activate(preactivation, activation, shape)?; return Ok((preactivation, activated)); }

/// Select the exact forward implementation for a declared dense activation.
///
/// # Errors
///
/// Returns an error when a required scalar or canonical operation program cannot
/// be constructed.
impl DenseActivation { pub(crate) fn forward_kernel<E>(self) -> Result<ForwardActivation, E> where
		E: From<LanguageError> + From<OperationError>, { Ok(match self {
		DenseActivation::Linear => ForwardActivation::Identity,
		DenseActivation::Cosine => ForwardActivation::Owned("gpu_cos"),
		DenseActivation::Exponential => ForwardActivation::Owned("gpu_exp"),
		DenseActivation::Logarithm => ForwardActivation::SignedMagnitude("gpu_log_into"),
		DenseActivation::NaturalLogarithm => ForwardActivation::Owned("gpu_log_into"),
		DenseActivation::LegacySignedLogOnePlus => ForwardActivation::SignedMagnitude("gpu_log1p"),
		DenseActivation::Huber => { let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
			let absolute = builder.unary(ScalarOpcode::Absolute, value)?; let one = builder.f32(1.0)?;
			let quadratic_domain = builder.binary(ScalarOpcode::LessThanOrEqual, absolute, one)?;
			let square = builder.binary(ScalarOpcode::Multiply, value, value)?; let half = builder.f32(0.5)?;
			let quadratic = builder.binary(ScalarOpcode::Multiply, half, square)?;
			let linear = builder.binary(ScalarOpcode::Subtract, absolute, half)?;
			let result = builder.ternary(ScalarOpcode::Select, quadratic_domain, quadratic, linear)?;
			ForwardActivation::Program(builder.finish(&[result])?) }
		DenseActivation::Tangent => ForwardActivation::Owned("gpu_tan"),
		DenseActivation::Relu => ForwardActivation::Owned("gpu_relu_into"),
		DenseActivation::LeakyRelu => ForwardActivation::Program(canonical_leaky_relu_program()?),
		DenseActivation::Sigmoid => ForwardActivation::Owned("gpu_sigmoid_into"),
		DenseActivation::Tanh => ForwardActivation::Owned("gpu_tanh_into"),
		DenseActivation::Selu => ForwardActivation::Program(canonical_selu_program()?),
		DenseActivation::Gelu => ForwardActivation::Owned("gpu_gelu_into"),
		DenseActivation::Silu => ForwardActivation::Owned("gpu_silu_into"),
		DenseActivation::Elu => ForwardActivation::Program(canonical_elu_program()?),
		DenseActivation::PRelu => ForwardActivation::PRelu(canonical_prelu_program()?), }) } }

/// Build a scalar program that marks causally visible attention positions.
///
/// # Errors
///
/// Returns an error when the sequence extent or scalar program is invalid.
pub fn causal_mask_program(sequence_length: u64) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let position = builder.input(DType::I32)?;
	let sequence = builder.i32(attention_extent(sequence_length, "sequence length")?)?;
	let key = builder.binary(ScalarOpcode::Remainder, position, sequence)?;
	let row = builder.binary(ScalarOpcode::Divide, position, sequence)?;
	let query = builder.binary(ScalarOpcode::Remainder, row, sequence)?;
	let visible = builder.binary(ScalarOpcode::LessThanOrEqual, key, query)?; return builder.finish(&[visible]); }

/// Build the source-index transform from head-major packed coordinates.
///
/// # Errors
///
/// Returns an error when an extent exceeds `i32` or the scalar program is invalid.
pub fn head_major_source_index_program( sequence_length: u64, head_count: u64, head_dimension: u64,
) -> LanguageResult<ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let sequence = builder.i32(attention_extent(sequence_length, "sequence length")?)?;
	let encoded_head_count = builder.i32(attention_extent(head_count, "head count")?)?;
	let dimension = builder.i32(attention_extent(head_dimension, "head dimension")?)?;
	let channel = builder.binary(ScalarOpcode::Remainder, position, dimension)?;
	let packed = builder.binary(ScalarOpcode::Divide, position, dimension)?;
	let head = builder.binary(ScalarOpcode::Remainder, packed, encoded_head_count)?;
	let sequence_packed = builder.binary(ScalarOpcode::Divide, packed, encoded_head_count)?;
	let token = builder.binary(ScalarOpcode::Remainder, sequence_packed, sequence)?;
	let row = builder.binary(ScalarOpcode::Divide, sequence_packed, sequence)?;
	let row_head_base = builder.binary(ScalarOpcode::Multiply, row, encoded_head_count)?;
	let selected_head = builder.binary(ScalarOpcode::Add, row_head_base, head)?;
	let token_base = builder.binary(ScalarOpcode::Multiply, selected_head, sequence)?;
	let selected_token = builder.binary(ScalarOpcode::Add, token_base, token)?;
	let channel_base = builder.binary(ScalarOpcode::Multiply, selected_token, dimension)?;
	let source_index = builder.binary(ScalarOpcode::Add, channel_base, channel)?; return builder.finish(&[source_index]); }

/// Convert one attention extent to the scalar program's `i32` coordinate type.
///
/// # Errors
///
/// Returns an error when `value` exceeds `i32`.
fn attention_extent(value: u64, name: &str) -> LanguageResult<i32> { return i32::try_from(value).map_err(|error| {
		return LanguageError::new( LanguageErrorKind::InvalidScalarProgram,
			format!("attention {name} {value} cannot be represented by int32: {error}"),
		); }); }

/// Build an elementwise sum over `inputs` scalar values.
///
/// # Errors
///
/// Returns an error when `inputs` is zero or the scalar program is invalid.
pub fn sum_program(inputs: usize) -> LanguageResult<ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let mut input_values = Vec::with_capacity(inputs); for _ in 0..inputs { input_values.push(builder.input(DType::F32)?);
	}
	let mut values = input_values.into_iter(); let mut total = values.next().ok_or_else(|| { return LanguageError::new(
			LanguageErrorKind::InvalidScalarProgram,
			"scalar sum requires at least one input",
		); })?; for value in values { total = builder.binary(ScalarOpcode::Add, total, value)?; }
	return builder.finish(&[total]); }

/// Declare every input-to-output alias pair forbidden.
pub fn forbidden_aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> { return (0..inputs)
		.flat_map(|input| { return (0..outputs).map(move |output| { return PrimitiveAliasRule { input, output,
					permission: AliasPermission::Forbidden, }; }); }) .collect(); }

/// Build an elementwise addition program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn add_program() -> LanguageResult<ScalarProgram> { return binary_program(ScalarOpcode::Add); }

/// Build an elementwise subtraction program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn subtract_program() -> LanguageResult<ScalarProgram> { return binary_program(ScalarOpcode::Subtract); }

/// Build an elementwise multiplication program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn multiply_program() -> LanguageResult<ScalarProgram> { return binary_program(ScalarOpcode::Multiply); }

/// Build an elementwise division program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn divide_program() -> LanguageResult<ScalarProgram> { return binary_program(ScalarOpcode::Divide); }

/// Build a two-input scalar program for `opcode`.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
fn binary_program(opcode: ScalarOpcode) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?; let result = builder.binary(opcode, left, right)?;
	return builder.finish(&[result]); }

/// Build an elementwise square program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn square_program() -> LanguageResult<ScalarProgram> { let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?; let result = builder.binary(ScalarOpcode::Multiply, value, value)?;
	return builder.finish(&[result]); }

/// Build an elementwise multiplication by `constant`.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn multiply_constant_program(constant: f32) -> LanguageResult<ScalarProgram> {
	return constant_binary_program(constant, ScalarOpcode::Multiply); }

/// Build an elementwise division by `divisor`.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn divide_constant_program(divisor: f32) -> LanguageResult<ScalarProgram> {
	return constant_binary_program(divisor, ScalarOpcode::Divide); }

/// Build a binary scalar program with one input and one embedded constant.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
fn constant_binary_program(constant_value: f32, opcode: ScalarOpcode) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let constant_expression = builder.f32(constant_value)?;
	let result = builder.binary(opcode, value, constant_expression)?; return builder.finish(&[result]); }

/// Build a masked or unmasked z-score normalization program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn z_score_program(epsilon_value: f32, masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let mean = builder.input(DType::F32)?; let variance = builder.input(DType::F32)?; let mask = masked .then(|| {
			return builder.input(DType::F32); }) .transpose()?; let epsilon_floor = builder.f32(epsilon_value)?;
	let stabilized_variance = builder.binary(ScalarOpcode::Maximum, variance, epsilon_floor)?;
	let standard_deviation = builder.unary(ScalarOpcode::SquareRoot, stabilized_variance)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, mean)?;
	let standardized = builder.binary(ScalarOpcode::Divide, centered, standard_deviation)?;
	let selected_normalization = select_normalized(&mut builder, value, standardized, mask)?;
	return builder.finish(&[selected_normalization]); }

/// Build a masked or unmasked min-max normalization program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn min_max_program(epsilon_value: f32, masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let minimum = builder.input(DType::F32)?; let maximum = builder.input(DType::F32)?; let mask = masked .then(|| {
			return builder.input(DType::F32); }) .transpose()?;
	let range = builder.binary(ScalarOpcode::Subtract, maximum, minimum)?; let epsilon_floor = builder.f32(epsilon_value)?;
	let denominator = builder.binary(ScalarOpcode::Maximum, range, epsilon_floor)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, minimum)?;
	let scaled = builder.binary(ScalarOpcode::Divide, centered, denominator)?;
	let selected_normalization = select_normalized(&mut builder, value, scaled, mask)?;
	return builder.finish(&[selected_normalization]); }

/// Build the squared contribution used by L2 normalization.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn l2_square_program(masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?; let mask = masked .then(|| {
			return builder.input(DType::F32); }) .transpose()?;
	let squared_value = builder.binary(ScalarOpcode::Multiply, value, value)?; let masked_square = match mask {
		Some(active_mask) => builder.binary(ScalarOpcode::Multiply, squared_value, active_mask)?, None => squared_value, };
	return builder.finish(&[masked_square]); }

/// Build a masked or unmasked L2 normalization program.
///
/// # Errors
///
/// Returns an error when the scalar program cannot be constructed.
pub fn l2_norm_program(epsilon_value: f32, masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let input_norm_squared = builder.input(DType::F32)?; let mask = masked .then(|| { return builder.input(DType::F32); })
		.transpose()?; let epsilon_floor = builder.f32(epsilon_value)?;
	let stabilized_norm_squared = builder.binary(ScalarOpcode::Maximum, input_norm_squared, epsilon_floor)?;
	let norm = builder.unary(ScalarOpcode::SquareRoot, stabilized_norm_squared)?;
	let unit_value = builder.binary(ScalarOpcode::Divide, value, norm)?;
	let selected_normalization = select_normalized(&mut builder, value, unit_value, mask)?;
	return builder.finish(&[selected_normalization]); }

/// Select a normalized value only where an optional mask is active.
///
/// # Errors
///
/// Returns an error when the scalar expressions cannot be constructed.
fn select_normalized( builder: &mut ScalarProgramBuilder, value: recipe_language::ScalarExpression,
	normalized: recipe_language::ScalarExpression, mask: Option<recipe_language::ScalarExpression>,
) -> LanguageResult<recipe_language::ScalarExpression> { let Some(active_mask) = mask else { return Ok(normalized); };
	let zero = builder.f32(0.0)?; let normalize = builder.binary(ScalarOpcode::GreaterThan, active_mask, zero)?;
	return builder.ternary(ScalarOpcode::Select, normalize, normalized, value); }
