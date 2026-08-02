use recipe_core::{AliasPermission, DType, ScalarOpcode, ScalarProgram, ValueId};
use recipe_language::{
	LanguageError, LanguageErrorKind, LanguageResult, PrimitiveAliasRule, ScalarProgramBuilder, Shape,
};
use recipe_ops::{
	OperationError, canonical_elu_program, canonical_leaky_relu_program, canonical_prelu_program,
	canonical_selu_program,
};

use crate::DenseActivation;

pub(crate) enum ForwardActivation {
	Identity,
	Owned(&'static str),
	Program(ScalarProgram),
	SignedMagnitude(&'static str),
	PRelu(ScalarProgram),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RecurrentGateParameters {
	pub(crate) input_weight: ValueId,
	pub(crate) recurrent_weight: ValueId,
	pub(crate) bias: ValueId,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GruForwardParameters {
	pub(crate) reset: RecurrentGateParameters,
	pub(crate) update: RecurrentGateParameters,
	pub(crate) candidate: RecurrentGateParameters,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LstmForwardParameters {
	pub(crate) input: RecurrentGateParameters,
	pub(crate) forget: RecurrentGateParameters,
	pub(crate) output: RecurrentGateParameters,
	pub(crate) candidate: RecurrentGateParameters,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RnnStepValues {
	pub(crate) input: ValueId,
	pub(crate) previous_hidden: ValueId,
	pub(crate) preactivation: ValueId,
	pub(crate) hidden: ValueId,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GruStepValues {
	pub(crate) input: ValueId,
	pub(crate) previous_hidden: ValueId,
	pub(crate) reset_preactivation: ValueId,
	pub(crate) reset: ValueId,
	pub(crate) update_preactivation: ValueId,
	pub(crate) update: ValueId,
	pub(crate) reset_hidden: ValueId,
	pub(crate) candidate_preactivation: ValueId,
	pub(crate) candidate: ValueId,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LstmStepValues {
	pub(crate) input: ValueId,
	pub(crate) previous_hidden: ValueId,
	pub(crate) previous_cell: ValueId,
	pub(crate) input_gate_preactivation: ValueId,
	pub(crate) input_gate: ValueId,
	pub(crate) forget_gate_preactivation: ValueId,
	pub(crate) forget_gate: ValueId,
	pub(crate) output_gate_preactivation: ValueId,
	pub(crate) output_gate: ValueId,
	pub(crate) candidate_preactivation: ValueId,
	pub(crate) candidate: ValueId,
	pub(crate) cell: ValueId,
	pub(crate) cell_activation: ValueId,
}

/// Minimal graph-emission boundary shared by training, validation, and loaded
/// model inference. Parameter sourcing and gradient construction stay outside
/// this interface; it owns only the exact forward recurrent equations.
pub(crate) trait RecurrentForwardGraph {
	type Error: From<LanguageError>;

	fn zero_f32(&mut self, shape: Shape) -> Result<ValueId, Self::Error>;

	fn gather_matrix_column(
		&mut self,
		matrix: ValueId,
		rows: u64,
		columns: u64,
		column: u64,
	) -> Result<ValueId, Self::Error>;

	fn bias_free_linear(
		&mut self,
		input: ValueId,
		weight: ValueId,
		rows: u64,
		output_width: u64,
	) -> Result<ValueId, Self::Error>;

	fn elementwise_f32(
		&mut self,
		shape: Shape,
		inputs: Vec<ValueId>,
		program: ScalarProgram,
	) -> Result<ValueId, Self::Error>;

	fn activate(&mut self, input: ValueId, activation: DenseActivation, shape: Shape)
	-> Result<ValueId, Self::Error>;
}

pub(crate) fn lower_rnn_sequence<G: RecurrentForwardGraph>(
	graph: &mut G,
	input: ValueId,
	rows: u64,
	sequence: u64,
	width: u64,
	parameters: RecurrentGateParameters,
) -> Result<(ValueId, Vec<RnnStepValues>), G::Error> {
	let mut hidden = graph.zero_f32(Shape::new(vec![rows, width])?)?;
	let mut steps = Vec::new();
	for step in 0..sequence {
		let step_input = graph.gather_matrix_column(input, rows, sequence, step)?;
		let (next_hidden, values) = lower_rnn_step(graph, step_input, hidden, rows, width, parameters)?;
		steps.push(values);
		hidden = next_hidden;
	}
	Ok((hidden, steps))
}

pub(crate) fn lower_gru_sequence<G: RecurrentForwardGraph>(
	graph: &mut G,
	input: ValueId,
	rows: u64,
	sequence: u64,
	width: u64,
	parameters: GruForwardParameters,
) -> Result<(ValueId, Vec<GruStepValues>), G::Error> {
	let mut hidden = graph.zero_f32(Shape::new(vec![rows, width])?)?;
	let mut steps = Vec::new();
	for step in 0..sequence {
		let step_input = graph.gather_matrix_column(input, rows, sequence, step)?;
		let (next_hidden, values) = lower_gru_step(graph, step_input, hidden, rows, width, parameters)?;
		steps.push(values);
		hidden = next_hidden;
	}
	Ok((hidden, steps))
}

pub(crate) fn lower_lstm_sequence<G: RecurrentForwardGraph>(
	graph: &mut G,
	input: ValueId,
	rows: u64,
	sequence: u64,
	width: u64,
	parameters: LstmForwardParameters,
) -> Result<(ValueId, Vec<LstmStepValues>), G::Error> {
	let state_shape = Shape::new(vec![rows, width])?;
	let mut hidden = graph.zero_f32(state_shape.clone())?;
	let mut cell = graph.zero_f32(state_shape)?;
	let mut steps = Vec::new();
	for step in 0..sequence {
		let step_input = graph.gather_matrix_column(input, rows, sequence, step)?;
		let (next_hidden, next_cell, values) =
			lower_lstm_step(graph, step_input, hidden, cell, rows, width, parameters)?;
		steps.push(values);
		hidden = next_hidden;
		cell = next_cell;
	}
	Ok((hidden, steps))
}

pub(crate) fn lower_rnn_step<G: RecurrentForwardGraph>(
	graph: &mut G,
	input: ValueId,
	previous_hidden: ValueId,
	rows: u64,
	width: u64,
	parameters: RecurrentGateParameters,
) -> Result<(ValueId, RnnStepValues), G::Error> {
	let (preactivation, hidden) = lower_recurrent_gate(
		graph,
		input,
		previous_hidden,
		rows,
		width,
		parameters,
		DenseActivation::Tanh,
	)?;
	Ok((hidden, RnnStepValues {
		input,
		previous_hidden,
		preactivation,
		hidden,
	}))
}

pub(crate) fn lower_gru_step<G: RecurrentForwardGraph>(
	graph: &mut G,
	input: ValueId,
	previous_hidden: ValueId,
	rows: u64,
	width: u64,
	parameters: GruForwardParameters,
) -> Result<(ValueId, GruStepValues), G::Error> {
	let hidden_shape = Shape::new(vec![rows, width])?;
	let (reset_preactivation, reset) = lower_recurrent_gate(
		graph,
		input,
		previous_hidden,
		rows,
		width,
		parameters.reset,
		DenseActivation::Sigmoid,
	)?;
	let (update_preactivation, update) = lower_recurrent_gate(
		graph,
		input,
		previous_hidden,
		rows,
		width,
		parameters.update,
		DenseActivation::Sigmoid,
	)?;
	let reset_hidden = graph.elementwise_f32(
		hidden_shape.clone(),
		vec![reset, previous_hidden],
		multiply_program()?,
	)?;
	let candidate_input_projection = graph.bias_free_linear(input, parameters.candidate.input_weight, rows, width)?;
	let candidate_recurrent_projection = graph.bias_free_linear(
		reset_hidden,
		parameters.candidate.recurrent_weight,
		rows,
		width,
	)?;
	let candidate_preactivation = graph.elementwise_f32(
		hidden_shape.clone(),
		vec![
			candidate_input_projection,
			candidate_recurrent_projection,
			parameters.candidate.bias,
		],
		sum_program(3)?,
	)?;
	let candidate = graph.activate(
		candidate_preactivation,
		DenseActivation::Tanh,
		hidden_shape.clone(),
	)?;
	let next_hidden = graph.elementwise_f32(
		hidden_shape,
		vec![candidate, update, previous_hidden],
		gru_hidden_program()?,
	)?;
	Ok((next_hidden, GruStepValues {
		input,
		previous_hidden,
		reset_preactivation,
		reset,
		update_preactivation,
		update,
		reset_hidden,
		candidate_preactivation,
		candidate,
	}))
}

pub(crate) fn lower_lstm_step<G: RecurrentForwardGraph>(
	graph: &mut G,
	input: ValueId,
	previous_hidden: ValueId,
	previous_cell: ValueId,
	rows: u64,
	width: u64,
	parameters: LstmForwardParameters,
) -> Result<(ValueId, ValueId, LstmStepValues), G::Error> {
	let (input_gate_preactivation, input_gate) = lower_recurrent_gate(
		graph,
		input,
		previous_hidden,
		rows,
		width,
		parameters.input,
		DenseActivation::Sigmoid,
	)?;
	let (forget_gate_preactivation, forget_gate) = lower_recurrent_gate(
		graph,
		input,
		previous_hidden,
		rows,
		width,
		parameters.forget,
		DenseActivation::Sigmoid,
	)?;
	let (output_gate_preactivation, output_gate) = lower_recurrent_gate(
		graph,
		input,
		previous_hidden,
		rows,
		width,
		parameters.output,
		DenseActivation::Sigmoid,
	)?;
	let (candidate_preactivation, candidate) = lower_recurrent_gate(
		graph,
		input,
		previous_hidden,
		rows,
		width,
		parameters.candidate,
		DenseActivation::Tanh,
	)?;
	let state_shape = Shape::new(vec![rows, width])?;
	let cell = graph.elementwise_f32(
		state_shape.clone(),
		vec![forget_gate, previous_cell, input_gate, candidate],
		lstm_cell_program()?,
	)?;
	let cell_activation = graph.activate(cell, DenseActivation::Tanh, state_shape.clone())?;
	let hidden = graph.elementwise_f32(
		state_shape,
		vec![output_gate, cell_activation],
		multiply_program()?,
	)?;
	Ok((hidden, cell, LstmStepValues {
		input,
		previous_hidden,
		previous_cell,
		input_gate_preactivation,
		input_gate,
		forget_gate_preactivation,
		forget_gate,
		output_gate_preactivation,
		output_gate,
		candidate_preactivation,
		candidate,
		cell,
		cell_activation,
	}))
}

fn lower_recurrent_gate<G: RecurrentForwardGraph>(
	graph: &mut G,
	input: ValueId,
	previous_hidden: ValueId,
	rows: u64,
	width: u64,
	parameters: RecurrentGateParameters,
	activation: DenseActivation,
) -> Result<(ValueId, ValueId), G::Error> {
	let input_projection = graph.bias_free_linear(input, parameters.input_weight, rows, width)?;
	let recurrent_projection = graph.bias_free_linear(previous_hidden, parameters.recurrent_weight, rows, width)?;
	let shape = Shape::new(vec![rows, width])?;
	let preactivation = graph.elementwise_f32(
		shape.clone(),
		vec![input_projection, recurrent_projection, parameters.bias],
		sum_program(3)?,
	)?;
	let activated = graph.activate(preactivation, activation, shape)?;
	Ok((preactivation, activated))
}

pub(crate) fn lower_activation<E>(activation: DenseActivation) -> Result<ForwardActivation, E>
where E: From<LanguageError> + From<OperationError> {
	Ok(match activation {
		DenseActivation::Linear => ForwardActivation::Identity,
		DenseActivation::Cosine => ForwardActivation::Owned("gpu_cos"),
		DenseActivation::Exponential => ForwardActivation::Owned("gpu_exp"),
		DenseActivation::Logarithm => ForwardActivation::SignedMagnitude("gpu_log_into"),
		DenseActivation::NaturalLogarithm => ForwardActivation::Owned("gpu_log_into"),
		DenseActivation::LegacySignedLogOnePlus => ForwardActivation::SignedMagnitude("gpu_log1p"),
		DenseActivation::Huber => ForwardActivation::Program(huber_activation_program()?),
		DenseActivation::Tangent => ForwardActivation::Owned("gpu_tan"),
		DenseActivation::Relu => ForwardActivation::Owned("gpu_relu_into"),
		DenseActivation::LeakyRelu => ForwardActivation::Program(canonical_leaky_relu_program()?),
		DenseActivation::Sigmoid => ForwardActivation::Owned("gpu_sigmoid_into"),
		DenseActivation::Tanh => ForwardActivation::Owned("gpu_tanh_into"),
		DenseActivation::Selu => ForwardActivation::Program(canonical_selu_program()?),
		DenseActivation::Gelu => ForwardActivation::Owned("gpu_gelu_into"),
		DenseActivation::Silu => ForwardActivation::Owned("gpu_silu_into"),
		DenseActivation::Elu => ForwardActivation::Program(canonical_elu_program()?),
		DenseActivation::PRelu => ForwardActivation::PRelu(canonical_prelu_program()?),
	})
}

pub(crate) fn causal_mask_program(sequence_length: u64) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let sequence = builder.i32(attention_extent(sequence_length, "sequence length")?)?;
	let key = builder.binary(ScalarOpcode::Remainder, position, sequence)?;
	let row = builder.binary(ScalarOpcode::Divide, position, sequence)?;
	let query = builder.binary(ScalarOpcode::Remainder, row, sequence)?;
	let visible = builder.binary(ScalarOpcode::LessThanOrEqual, key, query)?;
	builder.finish(&[visible])
}

pub(crate) fn head_major_source_index_program(
	sequence_length: u64,
	heads: u64,
	head_dimension: u64,
) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let position = builder.input(DType::I32)?;
	let sequence = builder.i32(attention_extent(sequence_length, "sequence length")?)?;
	let heads = builder.i32(attention_extent(heads, "head count")?)?;
	let dimension = builder.i32(attention_extent(head_dimension, "head dimension")?)?;
	let channel = builder.binary(ScalarOpcode::Remainder, position, dimension)?;
	let packed = builder.binary(ScalarOpcode::Divide, position, dimension)?;
	let head = builder.binary(ScalarOpcode::Remainder, packed, heads)?;
	let sequence_packed = builder.binary(ScalarOpcode::Divide, packed, heads)?;
	let token = builder.binary(ScalarOpcode::Remainder, sequence_packed, sequence)?;
	let row = builder.binary(ScalarOpcode::Divide, sequence_packed, sequence)?;
	let source = builder.binary(ScalarOpcode::Multiply, row, heads)?;
	let source = builder.binary(ScalarOpcode::Add, source, head)?;
	let source = builder.binary(ScalarOpcode::Multiply, source, sequence)?;
	let source = builder.binary(ScalarOpcode::Add, source, token)?;
	let source = builder.binary(ScalarOpcode::Multiply, source, dimension)?;
	let source = builder.binary(ScalarOpcode::Add, source, channel)?;
	builder.finish(&[source])
}

fn attention_extent(value: u64, name: &str) -> LanguageResult<i32> {
	i32::try_from(value).map_err(|error| {
		LanguageError::new(
			LanguageErrorKind::InvalidScalarProgram,
			format!("attention {name} {value} cannot be represented by int32: {error}"),
		)
	})
}

pub(crate) fn sum_program(inputs: usize) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let mut values = (0..inputs)
		.map(|_| builder.input(DType::F32))
		.collect::<LanguageResult<Vec<_>>>()?
		.into_iter();
	let mut total = values.next().ok_or_else(|| {
		LanguageError::new(
			LanguageErrorKind::InvalidScalarProgram,
			"scalar sum requires at least one input",
		)
	})?;
	for value in values {
		total = builder.binary(ScalarOpcode::Add, total, value)?;
	}
	builder.finish(&[total])
}

pub(crate) fn forbidden_aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> {
	(0..inputs)
		.flat_map(|input| {
			(0..outputs).map(move |output| {
				PrimitiveAliasRule {
					input,
					output,
					permission: AliasPermission::Forbidden,
				}
			})
		})
		.collect()
}

pub(crate) fn add_program() -> LanguageResult<ScalarProgram> { binary_program(ScalarOpcode::Add) }

pub(crate) fn subtract_program() -> LanguageResult<ScalarProgram> { binary_program(ScalarOpcode::Subtract) }

pub(crate) fn multiply_program() -> LanguageResult<ScalarProgram> { binary_program(ScalarOpcode::Multiply) }

pub(crate) fn divide_program() -> LanguageResult<ScalarProgram> { binary_program(ScalarOpcode::Divide) }

fn binary_program(opcode: ScalarOpcode) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let left = builder.input(DType::F32)?;
	let right = builder.input(DType::F32)?;
	let result = builder.binary(opcode, left, right)?;
	builder.finish(&[result])
}

pub(crate) fn square_program() -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let result = builder.binary(ScalarOpcode::Multiply, value, value)?;
	builder.finish(&[result])
}

pub(crate) fn multiply_constant_program(constant: f32) -> LanguageResult<ScalarProgram> {
	constant_binary_program(constant, ScalarOpcode::Multiply)
}

pub(crate) fn divide_constant_program(divisor: f32) -> LanguageResult<ScalarProgram> {
	constant_binary_program(divisor, ScalarOpcode::Divide)
}

fn constant_binary_program(constant: f32, opcode: ScalarOpcode) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let constant = builder.f32(constant)?;
	let result = builder.binary(opcode, value, constant)?;
	builder.finish(&[result])
}

pub(crate) fn gru_hidden_program() -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let candidate = builder.input(DType::F32)?;
	let update = builder.input(DType::F32)?;
	let previous = builder.input(DType::F32)?;
	let difference = builder.binary(ScalarOpcode::Subtract, previous, candidate)?;
	let retained = builder.binary(ScalarOpcode::Multiply, update, difference)?;
	let hidden = builder.binary(ScalarOpcode::Add, candidate, retained)?;
	builder.finish(&[hidden])
}

pub(crate) fn lstm_cell_program() -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let forget = builder.input(DType::F32)?;
	let previous_cell = builder.input(DType::F32)?;
	let input = builder.input(DType::F32)?;
	let candidate = builder.input(DType::F32)?;
	let retained = builder.binary(ScalarOpcode::Multiply, forget, previous_cell)?;
	let admitted = builder.binary(ScalarOpcode::Multiply, input, candidate)?;
	let cell = builder.binary(ScalarOpcode::Add, retained, admitted)?;
	builder.finish(&[cell])
}

pub(crate) fn z_score_program(epsilon: f32, masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let mean = builder.input(DType::F32)?;
	let variance = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let epsilon = builder.f32(epsilon)?;
	let variance = builder.binary(ScalarOpcode::Maximum, variance, epsilon)?;
	let standard_deviation = builder.unary(ScalarOpcode::SquareRoot, variance)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, mean)?;
	let normalized = builder.binary(ScalarOpcode::Divide, centered, standard_deviation)?;
	let normalized = select_normalized(&mut builder, value, normalized, mask)?;
	builder.finish(&[normalized])
}

pub(crate) fn min_max_program(epsilon: f32, masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let minimum = builder.input(DType::F32)?;
	let maximum = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let range = builder.binary(ScalarOpcode::Subtract, maximum, minimum)?;
	let epsilon = builder.f32(epsilon)?;
	let denominator = builder.binary(ScalarOpcode::Maximum, range, epsilon)?;
	let centered = builder.binary(ScalarOpcode::Subtract, value, minimum)?;
	let normalized = builder.binary(ScalarOpcode::Divide, centered, denominator)?;
	let normalized = select_normalized(&mut builder, value, normalized, mask)?;
	builder.finish(&[normalized])
}

pub(crate) fn l2_square_program(masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let square = builder.binary(ScalarOpcode::Multiply, value, value)?;
	let square = match mask {
		Some(mask) => builder.binary(ScalarOpcode::Multiply, square, mask)?,
		None => square,
	};
	builder.finish(&[square])
}

pub(crate) fn l2_norm_program(epsilon: f32, masked: bool) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?;
	let value = builder.input(DType::F32)?;
	let norm_squared = builder.input(DType::F32)?;
	let mask = masked.then(|| builder.input(DType::F32)).transpose()?;
	let epsilon = builder.f32(epsilon)?;
	let norm_squared = builder.binary(ScalarOpcode::Maximum, norm_squared, epsilon)?;
	let norm = builder.unary(ScalarOpcode::SquareRoot, norm_squared)?;
	let normalized = builder.binary(ScalarOpcode::Divide, value, norm)?;
	let normalized = select_normalized(&mut builder, value, normalized, mask)?;
	builder.finish(&[normalized])
}

fn select_normalized(
	builder: &mut ScalarProgramBuilder,
	value: recipe_language::ScalarExpression,
	normalized: recipe_language::ScalarExpression,
	mask: Option<recipe_language::ScalarExpression>,
) -> LanguageResult<recipe_language::ScalarExpression> {
	let Some(mask) = mask else {
		return Ok(normalized);
	};
	let zero = builder.f32(0.0)?;
	let normalize = builder.binary(ScalarOpcode::GreaterThan, mask, zero)?;
	builder.ternary(ScalarOpcode::Select, normalize, normalized, value)
}

pub(crate) fn huber_activation_program() -> LanguageResult<ScalarProgram> {
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
	builder.finish(&[result])
}
