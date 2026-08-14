//! Compile-time lowering for the scalar, predictor, route, and normalization
//! pieces of a concrete model.
//!
//! The caller supplies the already selected value type and pointer spelling.
//! These routines return LLVM text containing only fixed SSA and direct memory
//! operations. They intentionally do not emit descriptors, instruction arrays,
//! opcode switches, or runtime graph traversal.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ScalarOpcode {
	Add = 0,
	Constant = 1,
	Parameter = 2,
	Subtract = 3,
	Multiply = 4,
	Divide = 5,
	Absolute = 6,
	Exp = 7,
	Log = 8,
	Sin = 10,
	Cos = 11,
	Tanh = 12,
	Greater = 13,
	Rat = 14,
}

impl ScalarOpcode {
	fn from_i32(value: i32) -> Result<Self, EmitError> {
		match value {
			0 => Ok(Self::Add),
			1 => Ok(Self::Constant),
			2 => Ok(Self::Parameter),
			3 => Ok(Self::Subtract),
			4 => Ok(Self::Multiply),
			5 => Ok(Self::Divide),
			6 => Ok(Self::Absolute),
			7 => Ok(Self::Exp),
			8 => Ok(Self::Log),
			10 => Ok(Self::Sin),
			11 => Ok(Self::Cos),
			12 => Ok(Self::Tanh),
			13 => Ok(Self::Greater),
			14 => Ok(Self::Rat),
			_ => Err(EmitError::InvalidOpcode { kind: "scalar", value }),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredictorOpcode {
	Feature = 0,
	Row = 1,
	Constant = 2,
	Load = 3,
	Store = 4,
	Duplicate = 5,
	Add = 6,
	Subtract = 7,
	Multiply = 8,
	Divide = 9,
	Greater = 10,
	Choose = 11,
}

impl PredictorOpcode {
	fn from_i32(value: i32) -> Result<Self, EmitError> {
		match value {
			0 => Ok(Self::Feature),
			1 => Ok(Self::Row),
			2 => Ok(Self::Constant),
			3 => Ok(Self::Load),
			4 => Ok(Self::Store),
			5 => Ok(Self::Duplicate),
			6 => Ok(Self::Add),
			7 => Ok(Self::Subtract),
			8 => Ok(Self::Multiply),
			9 => Ok(Self::Divide),
			10 => Ok(Self::Greater),
			11 => Ok(Self::Choose),
			_ => Err(EmitError::InvalidOpcode { kind: "predictor", value }),
		}
	}
}

#[derive(Debug)]
pub enum EmitError {
	WrongWidth { kind: &'static str, width: usize },
	InvalidOpcode { kind: &'static str, value: i32 },
	InvalidOperand { kind: &'static str, value: f64 },
	InvalidReference { kind: &'static str, index: i32 },
	StackUnderflow { kind: &'static str },
	StackDepth { kind: &'static str, depth: usize },
	LocalIndex { index: usize, locals: usize },
	EmptyRoute,
}

impl fmt::Display for EmitError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::WrongWidth { kind, width } => write!(f, "{kind} program has invalid width {width}"),
			Self::InvalidOpcode { kind, value } => write!(f, "{kind} program has invalid opcode {value}"),
			Self::InvalidOperand { kind, value } => write!(f, "{kind} program has invalid operand {value}"),
			Self::InvalidReference { kind, index } => write!(f, "{kind} program references unavailable value {index}"),
			Self::StackUnderflow { kind } => write!(f, "{kind} program stack underflows"),
			Self::StackDepth { kind, depth } => write!(f, "{kind} program ends at stack depth {depth}"),
			Self::LocalIndex { index, locals } => write!(f, "predictor local {index} is outside {locals} locals"),
			Self::EmptyRoute => f.write_str("route has no fields"),
		}
	}
}

impl std::error::Error for EmitError {}

pub type LiteralFn<'a> = dyn Fn(f64, &str) -> String + 'a;

#[derive(Clone, Copy)]
pub struct ScalarContext<'a> {
	pub value_type: &'a str,
	pub pointer_type: &'a str,
	pub first: &'a str,
	pub second: &'a str,
	pub weights: &'a str,
	pub parameter_base: &'a str,
	pub prefix: &'a str,
	pub literal: &'a LiteralFn<'a>,
}

#[derive(Clone)]
pub struct ScalarForward {
	pub code: String,
	pub value: String,
	pub instructions: usize,
	pub parameter_registers: Vec<(usize, usize)>,
}

#[derive(Clone)]
pub struct ScalarReverse {
	pub code: String,
	pub first_adjoint: String,
	pub second_adjoint: String,
	pub parameter_adjoint: BTreeMap<usize, String>,
}

#[derive(Clone)]
struct ScalarInstruction {
	opcode: ScalarOpcode,
	left: f64,
	right: f64,
}

fn integer(value: f64, kind: &'static str) -> Result<i32, EmitError> {
	if !value.is_finite() || value.fract() != 0.0 || value < i32::MIN as f64 || value > i32::MAX as f64 {
		return Err(EmitError::InvalidOperand { kind, value });
	}
	Ok(value as i32)
}

fn binary(code: &mut String, value_type: &str, name: &str, operation: &str, left: &str, right: &str) -> String {
	let _ = writeln!(code, "{name} = call {value_type} @recipe.{operation}({value_type} {left}, {value_type} {right})");
	name.to_owned()
}

fn predicate(code: &mut String, value_type: &str, name: &str, operation: &str, left: &str, right: &str) -> String {
	let _ = writeln!(code, "{name}.condition = call i1 @recipe.{operation}({value_type} {left}, {value_type} {right})");
	let _ = writeln!(code, "{name} = call {value_type} @recipe.from.u1(i1 {name}.condition)");
	name.to_owned()
}

fn parse_scalar(code: &[f64]) -> Result<Vec<ScalarInstruction>, EmitError> {
	if code.len() % 3 != 0 {
		return Err(EmitError::WrongWidth { kind: "scalar", width: code.len() });
	}
	code.chunks_exact(3)
		.map(|instruction| Ok(ScalarInstruction { opcode: ScalarOpcode::from_i32(integer(instruction[0], "scalar opcode")?)?, left: instruction[1], right: instruction[2] }))
		.collect()
}

fn scalar_operand(value: f64, values: &[String], first: &str, second: &str) -> Result<String, EmitError> {
	let index = integer(value, "scalar reference")?;
	match index {
		-2 => Ok(second.to_owned()),
		-1 => Ok(first.to_owned()),
		0.. => values.get(index as usize).cloned().ok_or(EmitError::InvalidReference { kind: "scalar", index }),
		_ => Err(EmitError::InvalidReference { kind: "scalar", index }),
	}
}

/// Emit a scalar program as straight-line SSA. The returned value is the
/// program result. Scalar `Rat` deliberately returns its left operand in the
/// forward path, matching the real block's inference semantics.
pub fn emit_scalar_forward(code: &[f64], context: ScalarContext<'_>) -> Result<ScalarForward, EmitError> {
	let instructions = parse_scalar(code)?;
	let mut output = String::new();
	let mut values = Vec::with_capacity(instructions.len());
	let mut parameters = Vec::new();
	for (index, instruction) in instructions.iter().enumerate() {
		let name = format!("%{}.scalar.{index}", context.prefix);
		let value = match instruction.opcode {
			ScalarOpcode::Constant => (context.literal)(instruction.left, context.value_type),
			ScalarOpcode::Parameter => {
				let parameter = integer(instruction.left, "scalar parameter")?;
				if parameter < 0 {
					return Err(EmitError::InvalidOperand { kind: "scalar parameter", value: instruction.left });
				}
				let base = format!("{name}.base");
				let pointer = format!("{name}.ptr");
				let _ = writeln!(output, "{base} = getelementptr inbounds {ty}, {ptrty} {weights}, i32 {offset}", ty = context.value_type, ptrty = context.pointer_type, weights = context.weights, offset = context.parameter_base);
				let _ = writeln!(output, "{pointer} = getelementptr inbounds {ty}, {ptrty} {base}, i32 {parameter}", ty = context.value_type, ptrty = context.pointer_type, base = base, parameter = parameter);
				let _ = writeln!(output, "{name} = load {ty}, {ptrty} {pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type, pointer = pointer, align = if context.value_type == "double" { 8 } else { 4 });
				parameters.push((index, parameter as usize));
				name
			},
			ScalarOpcode::Rat => scalar_operand(instruction.left, &values, context.first, context.second)?,
			ScalarOpcode::Add | ScalarOpcode::Subtract | ScalarOpcode::Multiply | ScalarOpcode::Divide | ScalarOpcode::Greater => {
				let left = scalar_operand(instruction.left, &values, context.first, context.second)?;
				let right = scalar_operand(instruction.right, &values, context.first, context.second)?;
				match instruction.opcode {
					ScalarOpcode::Add => binary(&mut output, context.value_type, &name, "add", &left, &right),
					ScalarOpcode::Subtract => binary(&mut output, context.value_type, &name, "sub", &left, &right),
					ScalarOpcode::Multiply => binary(&mut output, context.value_type, &name, "mul", &left, &right),
					ScalarOpcode::Divide => binary(&mut output, context.value_type, &name, "div", &left, &right),
					ScalarOpcode::Greater => predicate(&mut output, context.value_type, &name, "ogt", &left, &right),
					_ => unreachable!(),
				}
			}
			ScalarOpcode::Absolute | ScalarOpcode::Exp | ScalarOpcode::Log | ScalarOpcode::Sin | ScalarOpcode::Cos | ScalarOpcode::Tanh => {
				let left = scalar_operand(instruction.left, &values, context.first, context.second)?;
				let operation = match instruction.opcode {
					ScalarOpcode::Absolute => "abs",
					ScalarOpcode::Exp => "exp",
					ScalarOpcode::Log => "log",
					ScalarOpcode::Sin => "sin",
					ScalarOpcode::Cos => "cos",
					ScalarOpcode::Tanh => "tanh",
					_ => unreachable!(),
				};
				let _ = writeln!(output, "{name} = call {ty} @recipe.{operation}({ty} {left})", ty = context.value_type);
				name
			},
		};
		values.push(value);
	}
	let value = values.last().cloned().ok_or(EmitError::WrongWidth { kind: "scalar", width: 0 })?;
	Ok(ScalarForward { code: output, value, instructions: instructions.len(), parameter_registers: parameters })
}

fn add_adjoint(code: &mut String, value_type: &str, prefix: &str, old: &mut String, contribution: &str, sequence: &mut usize) {
	let name = format!("%{prefix}.adjoint.{}", *sequence);
	*sequence += 1;
	*old = binary(code, value_type, &name, "add", old, contribution);
}

fn negate(code: &mut String, value_type: &str, prefix: &str, value: &str, sequence: &mut usize) -> String {
	let name = format!("%{prefix}.neg.{}", *sequence);
	*sequence += 1;
	let _ = writeln!(code, "{name} = call {value_type} @recipe.neg({value_type} {value})");
	name
}

/// Emit the reverse of a scalar program using the forward SSA values. The
/// result contains expressions for the two input adjoints and parameter
/// adjoints. The caller owns the flat adjoint and gradient arenas and stores
/// these expressions at the node's fixed element/parameter offsets.
pub fn emit_scalar_reverse(code: &[f64], forward: &ScalarForward, context: ScalarContext<'_>, incoming: &str) -> Result<ScalarReverse, EmitError> {
	let instructions = parse_scalar(code)?;
	if forward.instructions != instructions.len() {
		return Err(EmitError::WrongWidth { kind: "scalar forward", width: forward.instructions });
	}
	let mut output = String::new();
	let mut values = Vec::with_capacity(instructions.len());
	let mut parameter_for = vec![None; instructions.len()];
	for (index, instruction) in instructions.iter().enumerate() {
		let value = match instruction.opcode {
			ScalarOpcode::Constant => (context.literal)(instruction.left, context.value_type),
			ScalarOpcode::Parameter => {
				let parameter = integer(instruction.left, "scalar parameter")?;
				if parameter < 0 {
					return Err(EmitError::InvalidOperand { kind: "scalar parameter", value: instruction.left });
				}
				parameter_for[index] = Some(parameter as usize);
				format!("%{}.scalar.{index}", context.prefix)
			}
			ScalarOpcode::Rat => scalar_operand(instruction.left, &values, context.first, context.second)?,
			ScalarOpcode::Add | ScalarOpcode::Subtract | ScalarOpcode::Multiply | ScalarOpcode::Divide | ScalarOpcode::Greater | ScalarOpcode::Absolute | ScalarOpcode::Exp | ScalarOpcode::Log | ScalarOpcode::Sin | ScalarOpcode::Cos | ScalarOpcode::Tanh => format!("%{}.scalar.{index}", context.prefix),
		};
		values.push(value);
	}
	let mut adjoints = vec![(context.literal)(0.0, context.value_type); instructions.len()];
	let mut first = (context.literal)(0.0, context.value_type);
	let mut second = (context.literal)(0.0, context.value_type);
	let mut parameters = BTreeMap::new();
	let mut sequence = 0;
	if let Some(last) = adjoints.last_mut() {
		*last = incoming.to_owned();
	}
	let operand = |value: f64, values: &[String]| scalar_operand(value, values, context.first, context.second);
	let add_operand = |code: &mut String, value: f64, contribution: &str, adjoints: &mut [String], first: &mut String, second: &mut String, sequence: &mut usize| -> Result<(), EmitError> {
		let index = integer(value, "scalar reference")?;
		match index {
			-2 => add_adjoint(code, context.value_type, context.prefix, second, contribution, sequence),
			-1 => add_adjoint(code, context.value_type, context.prefix, first, contribution, sequence),
			0.. => {
				let slot = usize::try_from(index).map_err(|_| EmitError::InvalidReference { kind: "scalar", index })?;
				let target = adjoints.get_mut(slot).ok_or(EmitError::InvalidReference { kind: "scalar", index })?;
				add_adjoint(code, context.value_type, context.prefix, target, contribution, sequence)
			}
			_ => return Err(EmitError::InvalidReference { kind: "scalar", index }),
		}
		Ok(())
	};
	for (index, instruction) in instructions.iter().enumerate().rev() {
		let adjoint = adjoints[index].clone();
		let left = if matches!(instruction.opcode, ScalarOpcode::Constant | ScalarOpcode::Parameter) { String::new() } else { operand(instruction.left, &values)? };
		let right = if matches!(instruction.opcode, ScalarOpcode::Add | ScalarOpcode::Subtract | ScalarOpcode::Multiply | ScalarOpcode::Divide | ScalarOpcode::Greater | ScalarOpcode::Rat) { operand(instruction.right, &values)? } else { String::new() };
		match instruction.opcode {
			ScalarOpcode::Add => {
				add_operand(&mut output, instruction.left, &adjoint, &mut adjoints, &mut first, &mut second, &mut sequence)?;
				add_operand(&mut output, instruction.right, &adjoint, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Rat => {
				add_operand(&mut output, instruction.right, &adjoint, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Subtract => {
				add_operand(&mut output, instruction.left, &adjoint, &mut adjoints, &mut first, &mut second, &mut sequence)?;
				let negative = negate(&mut output, context.value_type, context.prefix, &adjoint, &mut sequence);
				add_operand(&mut output, instruction.right, &negative, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Multiply => {
				let left_contribution = binary(&mut output, context.value_type, &format!("%{}.mul.left.{sequence}", context.prefix), "mul", &adjoint, &right);
				sequence += 1;
				let right_contribution = binary(&mut output, context.value_type, &format!("%{}.mul.right.{sequence}", context.prefix), "mul", &adjoint, &left);
				sequence += 1;
				add_operand(&mut output, instruction.left, &left_contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
				add_operand(&mut output, instruction.right, &right_contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Divide => {
				let left_contribution = binary(&mut output, context.value_type, &format!("%{}.div.left.{sequence}", context.prefix), "div", &adjoint, &right);
				sequence += 1;
				let square = binary(&mut output, context.value_type, &format!("%{}.div.square.{sequence}", context.prefix), "mul", &right, &right);
				sequence += 1;
				let numerator = binary(&mut output, context.value_type, &format!("%{}.div.numerator.{sequence}", context.prefix), "mul", &adjoint, &left);
				sequence += 1;
				let raw = binary(&mut output, context.value_type, &format!("%{}.div.raw.{sequence}", context.prefix), "div", &numerator, &square);
				sequence += 1;
				let right_contribution = negate(&mut output, context.value_type, context.prefix, &raw, &mut sequence);
				add_operand(&mut output, instruction.left, &left_contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
				add_operand(&mut output, instruction.right, &right_contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Absolute => {
				let negative = format!("%{}.abs.negative.{sequence}", context.prefix);
				sequence += 1;
				let positive = format!("%{}.abs.positive.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{negative} = call i1 @recipe.olt({ty} {left}, {ty} {zero})", ty = context.value_type, zero = (context.literal)(0.0, context.value_type));
				let _ = writeln!(output, "{positive} = call i1 @recipe.ogt({ty} {left}, {ty} {zero})", ty = context.value_type, zero = (context.literal)(0.0, context.value_type));
				let negated = negate(&mut output, context.value_type, context.prefix, &adjoint, &mut sequence);
				let upper = format!("%{}.abs.upper.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{upper} = select i1 {positive}, {ty} {adjoint}, {ty} {zero}", ty = context.value_type, adjoint = adjoint, zero = (context.literal)(0.0, context.value_type));
				let contribution = format!("%{}.abs.contribution.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{contribution} = select i1 {negative}, {ty} {negated}, {ty} {upper}", ty = context.value_type, negated = negated, upper = upper);
				add_operand(&mut output, instruction.left, &contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Exp => {
				let contribution = binary(&mut output, context.value_type, &format!("%{}.exp.{sequence}", context.prefix), "mul", &adjoint, &values[index]);
				sequence += 1;
				add_operand(&mut output, instruction.left, &contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Log => {
				let contribution = binary(&mut output, context.value_type, &format!("%{}.log.{sequence}", context.prefix), "div", &adjoint, &left);
				sequence += 1;
				add_operand(&mut output, instruction.left, &contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Sin => {
				let cosine = format!("%{}.sin.cosine.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{cosine} = call {ty} @recipe.cos({ty} {left})", ty = context.value_type);
				let contribution = binary(&mut output, context.value_type, &format!("%{}.sin.{sequence}", context.prefix), "mul", &adjoint, &cosine);
				sequence += 1;
				add_operand(&mut output, instruction.left, &contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Cos => {
				let sine = format!("%{}.cos.sine.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{sine} = call {ty} @recipe.sin({ty} {left})", ty = context.value_type);
				let raw = binary(&mut output, context.value_type, &format!("%{}.cos.raw.{sequence}", context.prefix), "mul", &adjoint, &sine);
				sequence += 1;
				let contribution = negate(&mut output, context.value_type, context.prefix, &raw, &mut sequence);
				add_operand(&mut output, instruction.left, &contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Tanh => {
				let square = binary(&mut output, context.value_type, &format!("%{}.tanh.square.{sequence}", context.prefix), "mul", &values[index], &values[index]);
				sequence += 1;
				let one = (context.literal)(1.0, context.value_type);
				let base = binary(&mut output, context.value_type, &format!("%{}.tanh.base.{sequence}", context.prefix), "sub", &one, &square);
				sequence += 1;
				let contribution = binary(&mut output, context.value_type, &format!("%{}.tanh.{sequence}", context.prefix), "mul", &adjoint, &base);
				sequence += 1;
				add_operand(&mut output, instruction.left, &contribution, &mut adjoints, &mut first, &mut second, &mut sequence)?;
			}
			ScalarOpcode::Greater | ScalarOpcode::Constant | ScalarOpcode::Parameter => {}
		}
	}
	for (index, parameter) in parameter_for.into_iter().enumerate() {
		if let Some(parameter) = parameter {
			parameters.entry(parameter).and_modify(|value: &mut String| add_adjoint(&mut output, context.value_type, context.prefix, value, &adjoints[index], &mut sequence)).or_insert_with(|| adjoints[index].clone());
		}
	}
	Ok(ScalarReverse { code: output, first_adjoint: first, second_adjoint: second, parameter_adjoint: parameters })
}

#[derive(Clone, Copy)]
pub struct PredictorContext<'a> {
	pub value_type: &'a str,
	pub pointer_type: &'a str,
	pub input: &'a str,
	pub row: &'a str,
	pub features: usize,
	pub prefix: &'a str,
	pub literal: &'a LiteralFn<'a>,
}

pub struct PredictorForward {
	pub code: String,
	pub value: String,
}

fn parse_predictor(code: &[f64]) -> Result<Vec<(PredictorOpcode, f64)>, EmitError> {
	if code.len() % 2 != 0 {
		return Err(EmitError::WrongWidth { kind: "predictor", width: code.len() });
	}
	code.chunks_exact(2).map(|instruction| Ok((PredictorOpcode::from_i32(integer(instruction[0], "predictor opcode")?)?, instruction[1]))).collect()
}

/// Emit a predictor without a runtime stack, local array, or opcode switch.
/// Stores and loads are resolved at compile time into SSA values.
pub fn emit_predictor_forward(code: &[f64], locals: usize, context: PredictorContext<'_>) -> Result<PredictorForward, EmitError> {
	let instructions = parse_predictor(code)?;
	let mut output = String::new();
	let mut stack = Vec::new();
	let mut local_values = vec![(context.literal)(0.0, context.value_type); locals];
	let mut sequence = 0;
	let pop = |stack: &mut Vec<String>| stack.pop().ok_or(EmitError::StackUnderflow { kind: "predictor" });
	let push_binary = |output: &mut String, stack: &mut Vec<String>, operation: &str, value_type: &str, prefix: &str, sequence: &mut usize| -> Result<(), EmitError> {
		let right = pop(stack)?;
		let left = pop(stack)?;
		let name = format!("%{prefix}.predictor.{sequence}");
		*sequence += 1;
		stack.push(binary(output, value_type, &name, operation, &left, &right));
		Ok(())
	};
	for (opcode, argument) in instructions {
		match opcode {
			PredictorOpcode::Feature => {
				let feature = integer(argument, "predictor feature")?;
				if feature < 0 || feature as usize >= context.features {
					return Err(EmitError::InvalidOperand { kind: "predictor feature", value: argument });
				}
				let row_base = format!("%{}.predictor.row.base.{sequence}", context.prefix);
				sequence += 1;
				let index = format!("%{}.predictor.feature.index.{sequence}", context.prefix);
				sequence += 1;
				let pointer = format!("%{}.predictor.feature.ptr.{sequence}", context.prefix);
				sequence += 1;
				let value = format!("%{}.predictor.feature.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{row_base} = mul i32 {row}, {features}", row = context.row, features = context.features);
				let _ = writeln!(output, "{index} = add i32 {row_base}, {feature}");
				let _ = writeln!(output, "{pointer} = getelementptr inbounds {ty}, {ptrty} {input}, i32 {index}", ty = context.value_type, ptrty = context.pointer_type, input = context.input);
				let _ = writeln!(output, "{value} = load {ty}, {ptrty} {pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type, pointer = pointer, align = if context.value_type == "double" { 8 } else { 4 });
				stack.push(value);
			}
			PredictorOpcode::Row => {
				let value = format!("%{}.predictor.row.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{value} = call {ty} @recipe.from.u32(i32 {row})", ty = context.value_type, row = context.row);
				stack.push(value);
			}
			PredictorOpcode::Constant => stack.push((context.literal)(argument, context.value_type)),
			PredictorOpcode::Load => {
				let local = integer(argument, "predictor local")?;
				if local < 0 || local as usize >= locals {
					return Err(EmitError::LocalIndex { index: local.max(0) as usize, locals });
				}
				stack.push(local_values[local as usize].clone());
			}
			PredictorOpcode::Store => {
				let local = integer(argument, "predictor local")?;
				if local < 0 || local as usize >= locals {
					return Err(EmitError::LocalIndex { index: local.max(0) as usize, locals });
				}
				local_values[local as usize] = pop(&mut stack)?;
			}
			PredictorOpcode::Duplicate => stack.push(stack.last().cloned().ok_or(EmitError::StackUnderflow { kind: "predictor" })?),
			PredictorOpcode::Add => push_binary(&mut output, &mut stack, "add", context.value_type, context.prefix, &mut sequence)?,
			PredictorOpcode::Subtract => push_binary(&mut output, &mut stack, "sub", context.value_type, context.prefix, &mut sequence)?,
			PredictorOpcode::Multiply => push_binary(&mut output, &mut stack, "mul", context.value_type, context.prefix, &mut sequence)?,
			PredictorOpcode::Divide => push_binary(&mut output, &mut stack, "div", context.value_type, context.prefix, &mut sequence)?,
			PredictorOpcode::Greater => {
				let right = pop(&mut stack)?;
				let left = pop(&mut stack)?;
				let name = format!("%{}.predictor.greater.{sequence}", context.prefix);
				sequence += 1;
				stack.push(predicate(&mut output, context.value_type, &name, "ogt", &left, &right));
			}
			PredictorOpcode::Choose => {
				let no = pop(&mut stack)?;
				let yes = pop(&mut stack)?;
				let condition = pop(&mut stack)?;
				let condition_true = format!("%{}.predictor.condition.{sequence}", context.prefix);
				sequence += 1;
				let value = format!("%{}.predictor.choose.{sequence}", context.prefix);
				sequence += 1;
				let _ = writeln!(output, "{condition_true} = call i1 @recipe.one({ty} {condition}, {ty} {zero})", ty = context.value_type, zero = (context.literal)(0.0, context.value_type));
				let _ = writeln!(output, "{value} = select i1 {condition_true}, {ty} {yes}, {ty} {no}", ty = context.value_type);
				stack.push(value);
			}
		}
	}
	if stack.len() != 1 {
		return Err(EmitError::StackDepth { kind: "predictor", depth: stack.len() });
	}
	Ok(PredictorForward { code: output, value: stack.pop().unwrap_or_default() })
}

#[derive(Clone, Copy)]
pub struct RouteField<'a> {
	pub source: &'a str,
	pub stride: usize,
	pub index: usize,
}

#[derive(Clone, Copy)]
pub struct RouteContext<'a> {
	pub value_type: &'a str,
	pub pointer_type: &'a str,
	pub row: &'a str,
	pub output: &'a str,
	pub prefix: &'a str,
}

/// Emit a route as one fixed load/store per selected field. The fields are
/// unrolled, so the output contains no runtime source lookup or field switch.
pub fn emit_route(fields: &[RouteField<'_>], context: RouteContext<'_>) -> Result<String, EmitError> {
	if fields.is_empty() {
		return Err(EmitError::EmptyRoute);
	}
	let mut output = String::new();
	let output_base = format!("%{}.route.output.base", context.prefix);
	let _ = writeln!(output, "{output_base} = mul i32 {row}, {}", fields.len(), row = context.row);
	for (column, field) in fields.iter().enumerate() {
		let source_base = format!("%{}.route.source.base.{column}", context.prefix);
		let source_index = format!("%{}.route.source.index.{column}", context.prefix);
		let source_pointer = format!("%{}.route.source.ptr.{column}", context.prefix);
		let value = format!("%{}.route.value.{column}", context.prefix);
		let output_index = format!("%{}.route.output.index.{column}", context.prefix);
		let output_pointer = format!("%{}.route.output.ptr.{column}", context.prefix);
		let _ = writeln!(output, "{source_base} = mul i32 {row}, {stride}", row = context.row, stride = field.stride);
		let _ = writeln!(output, "{source_index} = add i32 {source_base}, {field}", field = field.index);
		let _ = writeln!(output, "{source_pointer} = getelementptr inbounds {ty}, {ptrty} {source}, i32 {source_index}", ty = context.value_type, ptrty = context.pointer_type, source = field.source);
		let _ = writeln!(output, "{value} = load {ty}, {ptrty} {source_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type, source_pointer = source_pointer, align = if context.value_type == "double" { 8 } else { 4 });
		let _ = writeln!(output, "{output_index} = add i32 {output_base}, {column}");
		let _ = writeln!(output, "{output_pointer} = getelementptr inbounds {ty}, {ptrty} {output}, i32 {output_index}", ty = context.value_type, ptrty = context.pointer_type, output = context.output);
		let _ = writeln!(output, "store {ty} {value}, {ptrty} {output_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type, value = value, output_pointer = output_pointer, align = if context.value_type == "double" { 8 } else { 4 });
	}
	Ok(output)
}

#[derive(Clone, Copy)]
pub struct RouteReverseContext<'a> {
	pub value_type: &'a str,
	pub pointer_type: &'a str,
	pub row: &'a str,
	pub output_adjoint: &'a str,
	pub prefix: &'a str,
}

/// Emit route reverse accumulation. `source_adjoint` is one fixed pointer per
/// field, already resolved by the caller to either the input adjoint arena or
/// a concrete preceding node's adjoint arena.
pub fn emit_route_reverse(fields: &[RouteField<'_>], source_adjoint: &[&str], context: RouteReverseContext<'_>) -> Result<String, EmitError> {
	if fields.is_empty() {
		return Err(EmitError::EmptyRoute);
	}
	if source_adjoint.len() != fields.len() {
		return Err(EmitError::WrongWidth { kind: "route adjoint", width: source_adjoint.len() });
	}
	let mut output = String::new();
	let output_base = format!("%{}.route.reverse.output.base", context.prefix);
	let _ = writeln!(output, "{output_base} = mul i32 {row}, {}", fields.len(), row = context.row);
	for (column, (field, source)) in fields.iter().zip(source_adjoint).enumerate() {
		let output_index = format!("%{}.route.reverse.output.index.{column}", context.prefix);
		let output_pointer = format!("%{}.route.reverse.output.ptr.{column}", context.prefix);
		let delta = format!("%{}.route.reverse.delta.{column}", context.prefix);
		let source_base = format!("%{}.route.reverse.source.base.{column}", context.prefix);
		let source_index = format!("%{}.route.reverse.source.index.{column}", context.prefix);
		let source_pointer = format!("%{}.route.reverse.source.ptr.{column}", context.prefix);
		let _ = writeln!(output, "{output_index} = add i32 {output_base}, {column}");
		let _ = writeln!(output, "{output_pointer} = getelementptr inbounds {ty}, {ptrty} {output_adjoint}, i32 {output_index}", ty = context.value_type, ptrty = context.pointer_type, output_adjoint = context.output_adjoint);
		let _ = writeln!(output, "{delta} = load {ty}, {ptrty} {output_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type, output_pointer = output_pointer, align = if context.value_type == "double" { 8 } else { 4 });
		let _ = writeln!(output, "{source_base} = mul i32 {row}, {stride}", row = context.row, stride = field.stride);
		let _ = writeln!(output, "{source_index} = add i32 {source_base}, {field}", field = field.index);
		let _ = writeln!(output, "{source_pointer} = getelementptr inbounds {ty}, {ptrty} {source}, i32 {source_index}", ty = context.value_type, ptrty = context.pointer_type, source = source);
		let _ = writeln!(output, "call {ty} @recipe.atomic.add({ptrty} {source_pointer}, {ty} {delta})", ty = context.value_type, ptrty = context.pointer_type);
	}
	Ok(output)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizeMode {
	Batch,
	Layer,
	/// Root-mean-square statistics use layer-shaped groups with a zero mean.
	Rms,
	/// Stored batch statistics used by evaluation and inference.
	Evaluation,
}

#[derive(Clone, Copy)]
pub struct NormalizeContext<'a> {
	pub value_type: &'a str,
	pub pointer_type: &'a str,
	pub source_value: &'a str,
	pub context: &'a str,
	pub row: &'a str,
	pub rows: &'a str,
	pub channels: usize,
	pub length: usize,
	pub mode: NormalizeMode,
	pub prefix: &'a str,
}

pub struct NormalizeFragment {
	pub code: String,
	pub value: String,
	pub group: String,
	pub groups: String,
}

/// Emit one normalized element from the fixed statistics arena. The arena is
/// laid out as `mean[group]` followed by `scale[group]`, with `groups` fixed by
/// the normalization mode. A training caller must run a separate fixed stats
/// pass before this fragment; evaluation and inference reuse the stored arena.
pub fn emit_normalize(context: NormalizeContext<'_>, element: &str) -> NormalizeFragment {
	let mut output = String::new();
	let elements = context.channels * context.length;
	let length = context.length;
	let local = format!("%{}.normalize.local", context.prefix);
	let row = format!("%{}.normalize.row", context.prefix);
	let position = format!("%{}.normalize.position", context.prefix);
	let group = format!("%{}.normalize.group", context.prefix);
	let groups = format!("%{}.normalize.groups", context.prefix);
	let scale_index = format!("%{}.normalize.scale.index", context.prefix);
	let mean_pointer = format!("%{}.normalize.mean.ptr", context.prefix);
	let scale_pointer = format!("%{}.normalize.scale.ptr", context.prefix);
	let mean = format!("%{}.normalize.mean", context.prefix);
	let scale = format!("%{}.normalize.scale", context.prefix);
	let centered = format!("%{}.normalize.centered", context.prefix);
	let value = format!("%{}.normalize.value", context.prefix);
	let _ = writeln!(output, "{row} = udiv i32 {element}, {elements}");
	let _ = writeln!(output, "{local} = urem i32 {element}, {elements}");
	let _ = writeln!(output, "{position} = urem i32 {local}, {length}");
	match context.mode {
		NormalizeMode::Batch | NormalizeMode::Evaluation => {
			let channel = format!("%{}.normalize.channel", context.prefix);
			let _ = writeln!(output, "{channel} = udiv i32 {local}, {length}");
			let _ = writeln!(output, "{group} = add i32 {channel}, 0");
			let _ = writeln!(output, "{groups} = add i32 0, {channels}", channels = context.channels);
		}
		NormalizeMode::Layer | NormalizeMode::Rms => {
			let row_base = format!("%{}.normalize.layer.row.base", context.prefix);
			let _ = writeln!(output, "{row_base} = mul i32 {row}, {length}");
			let _ = writeln!(output, "{group} = add i32 {row_base}, {position}");
			let _ = writeln!(output, "{groups} = mul i32 {rows}, {length}", rows = context.rows);
		}
	}
	let _ = writeln!(output, "{scale_index} = add i32 {groups}, {group}");
	let _ = writeln!(output, "{mean_pointer} = getelementptr inbounds {ty}, {ptrty} {context_ptr}, i32 {group}", ty = context.value_type, ptrty = context.pointer_type, context_ptr = context.context);
	let _ = writeln!(output, "{scale_pointer} = getelementptr inbounds {ty}, {ptrty} {context_ptr}, i32 {scale_index}", ty = context.value_type, ptrty = context.pointer_type, context_ptr = context.context);
	let _ = writeln!(output, "{mean} = load {ty}, {ptrty} {mean_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type, mean_pointer = mean_pointer, align = if context.value_type == "double" { 8 } else { 4 });
	let _ = writeln!(output, "{scale} = load {ty}, {ptrty} {scale_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type, scale_pointer = scale_pointer, align = if context.value_type == "double" { 8 } else { 4 });
	let _ = writeln!(output, "{centered} = call {ty} @recipe.sub({ty} {source}, {ty} {mean})", ty = context.value_type, source = context.source_value);
	let _ = writeln!(output, "{value} = call {ty} @recipe.mul({ty} {centered}, {ty} {scale})", ty = context.value_type);
	NormalizeFragment { code: output, value, group, groups }
}

#[derive(Clone, Copy)]
pub struct NormalizeReverseContext<'a> {
	pub value_type: &'a str,
	pub pointer_type: &'a str,
	pub context: &'a str,
	pub rows: &'a str,
	pub channels: usize,
	pub length: usize,
	pub mode: NormalizeMode,
	pub prefix: &'a str,
}

pub struct NormalizeReverseFragment {
	pub code: String,
	pub contribution: String,
}

/// Emit the fixed reverse formula for a normalized element. In training modes
/// the stats pass must have populated `sum[group]` and `projected[group]` in
/// the context arena. Evaluation uses stored scale directly because its stats
/// are not differentiated.
pub fn emit_normalize_reverse(context: NormalizeReverseContext<'_>, element: &str, delta: &str, output_value: &str) -> NormalizeReverseFragment {
	let mut code = String::new();
	let elements = context.channels * context.length;
	let length = context.length;
	let row = format!("%{}.normalize.reverse.row", context.prefix);
	let local = format!("%{}.normalize.reverse.local", context.prefix);
	let position = format!("%{}.normalize.reverse.position", context.prefix);
	let group = format!("%{}.normalize.reverse.group", context.prefix);
	let groups = format!("%{}.normalize.reverse.groups", context.prefix);
	let items = format!("%{}.normalize.reverse.items", context.prefix);
	let scale_index = format!("%{}.normalize.reverse.scale.index", context.prefix);
	let sum_base = format!("%{}.normalize.reverse.sum.base", context.prefix);
	let projected_base = format!("%{}.normalize.reverse.projected.base", context.prefix);
	let sum_index = format!("%{}.normalize.reverse.sum.index", context.prefix);
	let projected_index = format!("%{}.normalize.reverse.projected.index", context.prefix);
	let scale_pointer = format!("%{}.normalize.reverse.scale.ptr", context.prefix);
	let sum_pointer = format!("%{}.normalize.reverse.sum.ptr", context.prefix);
	let projected_pointer = format!("%{}.normalize.reverse.projected.ptr", context.prefix);
	let scale = format!("%{}.normalize.reverse.scale", context.prefix);
	let sum = format!("%{}.normalize.reverse.sum", context.prefix);
	let projected = format!("%{}.normalize.reverse.projected", context.prefix);
	let _ = writeln!(code, "{row} = udiv i32 {element}, {elements}");
	let _ = writeln!(code, "{local} = urem i32 {element}, {elements}");
	let _ = writeln!(code, "{position} = urem i32 {local}, {length}");
	match context.mode {
		NormalizeMode::Batch | NormalizeMode::Evaluation => {
			let channel = format!("%{}.normalize.reverse.channel", context.prefix);
			let _ = writeln!(code, "{channel} = udiv i32 {local}, {length}");
			let _ = writeln!(code, "{group} = add i32 {channel}, 0");
			let _ = writeln!(code, "{groups} = add i32 0, {channels}", channels = context.channels);
			let _ = writeln!(code, "{items} = mul i32 {rows}, {length}", rows = context.rows);
		}
		NormalizeMode::Layer | NormalizeMode::Rms => {
			let row_base = format!("%{}.normalize.reverse.row.base", context.prefix);
			let _ = writeln!(code, "{row_base} = mul i32 {row}, {length}");
			let _ = writeln!(code, "{group} = add i32 {row_base}, {position}");
			let _ = writeln!(code, "{groups} = mul i32 {rows}, {length}", rows = context.rows);
			let _ = writeln!(code, "{items} = add i32 0, {channels}", channels = context.channels);
		}
	}
	let _ = writeln!(code, "{scale_index} = add i32 {groups}, {group}");
	let _ = writeln!(code, "{sum_base} = mul i32 {groups}, 2");
	let _ = writeln!(code, "{projected_base} = mul i32 {groups}, 3");
	let _ = writeln!(code, "{sum_index} = add i32 {sum_base}, {group}");
	let _ = writeln!(code, "{projected_index} = add i32 {projected_base}, {group}");
	let _ = writeln!(code, "{scale_pointer} = getelementptr inbounds {ty}, {ptrty} {context_ptr}, i32 {scale_index}", ty = context.value_type, ptrty = context.pointer_type, context_ptr = context.context);
	let _ = writeln!(code, "{sum_pointer} = getelementptr inbounds {ty}, {ptrty} {context_ptr}, i32 {sum_index}", ty = context.value_type, ptrty = context.pointer_type, context_ptr = context.context);
	let _ = writeln!(code, "{projected_pointer} = getelementptr inbounds {ty}, {ptrty} {context_ptr}, i32 {projected_index}", ty = context.value_type, ptrty = context.pointer_type, context_ptr = context.context);
	let align = if context.value_type == "double" { 8 } else { 4 };
	let _ = writeln!(code, "{scale} = load {ty}, {ptrty} {scale_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type);
	if context.mode == NormalizeMode::Evaluation {
		let contribution = format!("%{}.normalize.reverse.fixed", context.prefix);
		let _ = writeln!(code, "{contribution} = call {ty} @recipe.mul({ty} {delta}, {ty} {scale})", ty = context.value_type);
		return NormalizeReverseFragment { code, contribution };
	}
	let _ = writeln!(code, "{sum} = load {ty}, {ptrty} {sum_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type);
	let _ = writeln!(code, "{projected} = load {ty}, {ptrty} {projected_pointer}, align {align}", ty = context.value_type, ptrty = context.pointer_type);
	let items_value = format!("%{}.normalize.reverse.items.value", context.prefix);
	let scaled_delta = format!("%{}.normalize.reverse.scaled.delta", context.prefix);
	let output_projection = format!("%{}.normalize.reverse.output.projection", context.prefix);
	let centered = format!("%{}.normalize.reverse.centered", context.prefix);
	let numerator = format!("%{}.normalize.reverse.numerator", context.prefix);
	let scaled = format!("%{}.normalize.reverse.scaled", context.prefix);
	let contribution = format!("%{}.normalize.reverse.contribution", context.prefix);
	let _ = writeln!(code, "{items_value} = call {ty} @recipe.from.u32(i32 {items})", ty = context.value_type);
	let _ = writeln!(code, "{scaled_delta} = call {ty} @recipe.mul({ty} {items_value}, {ty} {delta})", ty = context.value_type);
	let _ = writeln!(code, "{output_projection} = call {ty} @recipe.mul({ty} {output_value}, {ty} {projected})", ty = context.value_type);
	let _ = writeln!(code, "{centered} = call {ty} @recipe.sub({ty} {scaled_delta}, {ty} {sum})", ty = context.value_type);
	let _ = writeln!(code, "{numerator} = call {ty} @recipe.sub({ty} {centered}, {ty} {output_projection})", ty = context.value_type);
	let _ = writeln!(code, "{scaled} = call {ty} @recipe.mul({ty} {scale}, {ty} {numerator})", ty = context.value_type);
	let _ = writeln!(code, "{contribution} = call {ty} @recipe.div({ty} {scaled}, {ty} {items_value})", ty = context.value_type);
	NormalizeReverseFragment { code, contribution }
}
