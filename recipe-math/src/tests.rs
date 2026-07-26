use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Add, Mul, Sub};

use recipe_core::{DType, ScalarLiteral, ScalarOpcode, ScalarProgram, ScalarValueId};
use recipe_language::LanguageError;

use crate::{AlgorithmIdentity, MathFunction, exp_with_gradual_underflow_program};

#[derive(Clone, Debug, PartialEq, Eq)]
struct MathProgram {
	function: MathFunction,
	algorithm: AlgorithmIdentity,
	program: ScalarProgram,
}

fn build(function: MathFunction) -> Result<MathProgram, LanguageError> {
	ScalarProgram::try_from(function).map(|program| MathProgram {
		function,
		algorithm: function.algorithm(),
		program,
	})
}

#[derive(Clone, Copy, Debug)]
enum Value {
	F32(f32),
	I32(i32),
}

impl Value {
	fn f32(self) -> f32 {
		match self {
			Self::F32(value) => value,
			Self::I32(_) => panic!("expected f32 test-interpreter value"),
		}
	}

	fn i32(self) -> i32 {
		match self {
			Self::I32(value) => value,
			Self::F32(_) => panic!("expected i32 test-interpreter value"),
		}
	}
}

#[derive(Debug)]
struct Evaluation {
	output: f32,
	faulted: bool,
}

fn evaluate(program: &ScalarProgram, inputs: &[f32]) -> Evaluation {
	assert_eq!(program.inputs.len(), inputs.len());
	let mut values = BTreeMap::<ScalarValueId, Value>::new();
	for (input, value) in program.inputs.iter().zip(inputs) {
		assert_eq!(input.dtype, DType::F32);
		values.insert(input.id, Value::F32(*value));
	}
	for constant in &program.constants {
		let value = match constant.value {
			ScalarLiteral::F32Bits(bits) => Value::F32(f32::from_bits(bits)),
			ScalarLiteral::I32(value) => Value::I32(value),
		};
		values.insert(constant.id, value);
	}

	let mut faulted = false;
	for instruction in &program.instructions {
		let operands = instruction
			.operands
			.iter()
			.map(|operand| values[operand])
			.collect::<Vec<_>>();
		let value = evaluate_opcode(instruction.opcode, &operands, &mut faulted);
		values.insert(instruction.result, value);
	}

	assert_eq!(program.outputs.len(), 1);
	Evaluation {
		output: values[&program.outputs[0]].f32(),
		faulted,
	}
}

fn evaluate_opcode(opcode: ScalarOpcode, operands: &[Value], faulted: &mut bool) -> Value {
	type Evaluator = fn(&[Value]) -> Value;
	const EVALUATORS: &[(ScalarOpcode, Evaluator)] = &[
		(ScalarOpcode::Add, evaluate_add),
		(ScalarOpcode::Subtract, evaluate_subtract),
		(ScalarOpcode::Multiply, evaluate_multiply),
		(ScalarOpcode::Divide, evaluate_divide),
		(ScalarOpcode::Remainder, evaluate_remainder),
		(ScalarOpcode::Negate, evaluate_negate),
		(ScalarOpcode::Absolute, evaluate_absolute),
		(ScalarOpcode::Minimum, evaluate_minimum),
		(ScalarOpcode::Maximum, evaluate_maximum),
		(ScalarOpcode::Fma, evaluate_fma),
		(ScalarOpcode::Equal, evaluate_equal),
		(ScalarOpcode::NotEqual, evaluate_not_equal),
		(ScalarOpcode::LessThan, evaluate_less_than),
		(ScalarOpcode::LessThanOrEqual, evaluate_less_than_or_equal),
		(ScalarOpcode::GreaterThan, evaluate_greater_than),
		(
			ScalarOpcode::GreaterThanOrEqual,
			evaluate_greater_than_or_equal,
		),
		(ScalarOpcode::Select, evaluate_select),
		(ScalarOpcode::BitAnd, evaluate_bit_and),
		(ScalarOpcode::BitOr, evaluate_bit_or),
		(ScalarOpcode::BitXor, evaluate_bit_xor),
		(ScalarOpcode::BitNot, evaluate_bit_not),
		(ScalarOpcode::BitcastF32ToI32, evaluate_bitcast_f32_to_i32),
		(ScalarOpcode::BitcastI32ToF32, evaluate_bitcast_i32_to_f32),
		(ScalarOpcode::ShiftLeft, evaluate_shift_left),
		(
			ScalarOpcode::ShiftRightLogical,
			evaluate_shift_right_logical,
		),
		(
			ScalarOpcode::ShiftRightArithmetic,
			evaluate_shift_right_arithmetic,
		),
		(ScalarOpcode::Require, evaluate_require),
		(ScalarOpcode::IsFinite, evaluate_is_finite),
		(ScalarOpcode::IsNan, evaluate_is_nan),
		(ScalarOpcode::SquareRoot, evaluate_square_root),
		(ScalarOpcode::Floor, evaluate_floor),
		(ScalarOpcode::Ceiling, evaluate_ceiling),
		(ScalarOpcode::RoundNearestEven, evaluate_round_nearest_even),
		(ScalarOpcode::ConvertF32ToI32, evaluate_convert_f32_to_i32),
		(ScalarOpcode::ConvertI32ToF32, evaluate_convert_i32_to_f32),
	];
	let evaluator = EVALUATORS
		.iter()
		.find_map(|(candidate, evaluator)| (*candidate == opcode).then_some(*evaluator));
	let require_failed = match opcode == ScalarOpcode::Require {
		true => unary_operand(operands).i32() == 0,
		false => false,
	};
	*faulted |= require_failed;
	match evaluator {
		Some(evaluator) => evaluator(operands),
		None => panic!("test interpreter does not support {opcode:?}"),
	}
}

fn same_type_binary(operands: &[Value], float: impl Fn(f32, f32) -> f32, integer: impl Fn(i32, i32) -> i32) -> Value {
	let (left, right) = binary_operands(operands);
	match (left, right) {
		(Value::F32(left), Value::F32(right)) => Value::F32(float(left, right)),
		(Value::I32(left), Value::I32(right)) => Value::I32(integer(left, right)),
		(Value::F32(_), Value::I32(_)) | (Value::I32(_), Value::F32(_)) => {
			panic!("invalid mixed-type binary operands")
		}
	}
}

fn integer_binary(operands: &[Value], operation: impl Fn(i32, i32) -> i32) -> Value {
	let (left, right) = binary_operands(operands);
	Value::I32(operation(left.i32(), right.i32()))
}

fn comparison(operands: &[Value], float: impl Fn(f32, f32) -> bool, integer: impl Fn(i32, i32) -> bool) -> Value {
	let (left, right) = binary_operands(operands);
	let result = match (left, right) {
		(Value::F32(left), Value::F32(right)) => float(left, right),
		(Value::I32(left), Value::I32(right)) => integer(left, right),
		(Value::F32(_), Value::I32(_)) | (Value::I32(_), Value::F32(_)) => {
			panic!("invalid mixed-type comparison operands")
		}
	};
	Value::I32(i32::from(result))
}

fn unary_operand(operands: &[Value]) -> Value {
	assert_eq!(operands.len(), 1);
	operands[0]
}

fn binary_operands(operands: &[Value]) -> (Value, Value) {
	assert_eq!(operands.len(), 2);
	(operands[0], operands[1])
}

fn ternary_operands(operands: &[Value]) -> (Value, Value, Value) {
	assert_eq!(operands.len(), 3);
	(operands[0], operands[1], operands[2])
}

fn evaluate_add(operands: &[Value]) -> Value {
	same_type_binary(operands, f32::add, i32::wrapping_add)
}

fn evaluate_subtract(operands: &[Value]) -> Value {
	same_type_binary(operands, f32::sub, i32::wrapping_sub)
}

fn evaluate_multiply(operands: &[Value]) -> Value {
	same_type_binary(operands, f32::mul, i32::wrapping_mul)
}

fn evaluate_divide(operands: &[Value]) -> Value {
	let (left, right) = binary_operands(operands);
	match (left, right) {
		(Value::F32(left), Value::F32(right)) => Value::F32(left / right),
		(Value::I32(left), Value::I32(right)) => match right == 0 || (left == i32::MIN && right == -1) {
			true => Value::I32(0),
			false => Value::I32(left / right),
		},
		(Value::F32(_), Value::I32(_)) | (Value::I32(_), Value::F32(_)) => {
			panic!("invalid mixed-type Divide operands")
		}
	}
}

fn evaluate_remainder(operands: &[Value]) -> Value {
	let (left, right) = binary_operands(operands);
	match (left, right) {
		(Value::F32(left), Value::F32(right)) => Value::F32(left % right),
		(Value::I32(left), Value::I32(right)) => match right == 0 || (left == i32::MIN && right == -1) {
			true => Value::I32(0),
			false => Value::I32(left % right),
		},
		(Value::F32(_), Value::I32(_)) | (Value::I32(_), Value::F32(_)) => {
			panic!("invalid mixed-type Remainder operands")
		}
	}
}

fn evaluate_negate(operands: &[Value]) -> Value {
	match unary_operand(operands) {
		Value::F32(value) => Value::F32(-value),
		Value::I32(value) => Value::I32(value.wrapping_neg()),
	}
}

fn evaluate_absolute(operands: &[Value]) -> Value {
	match unary_operand(operands) {
		Value::F32(value) => Value::F32(value.abs()),
		Value::I32(value) => Value::I32(value.wrapping_abs()),
	}
}

fn evaluate_minimum(operands: &[Value]) -> Value {
	same_type_binary(operands, f32::min, i32::min)
}

fn evaluate_maximum(operands: &[Value]) -> Value {
	same_type_binary(operands, f32::max, i32::max)
}

fn evaluate_fma(operands: &[Value]) -> Value {
	let (left, right, addend) = ternary_operands(operands);
	Value::F32(left.f32().mul_add(right.f32(), addend.f32()))
}

fn evaluate_equal(operands: &[Value]) -> Value {
	comparison(
		operands,
		|left, right| left == right,
		|left, right| left == right,
	)
}

fn evaluate_not_equal(operands: &[Value]) -> Value {
	comparison(
		operands,
		|left, right| left != right,
		|left, right| left != right,
	)
}

fn evaluate_less_than(operands: &[Value]) -> Value {
	comparison(
		operands,
		|left, right| left < right,
		|left, right| left < right,
	)
}

fn evaluate_less_than_or_equal(operands: &[Value]) -> Value {
	comparison(
		operands,
		|left, right| left <= right,
		|left, right| left <= right,
	)
}

fn evaluate_greater_than(operands: &[Value]) -> Value {
	comparison(
		operands,
		|left, right| left > right,
		|left, right| left > right,
	)
}

fn evaluate_greater_than_or_equal(operands: &[Value]) -> Value {
	comparison(
		operands,
		|left, right| left >= right,
		|left, right| left >= right,
	)
}

fn evaluate_select(operands: &[Value]) -> Value {
	let (condition, selected, rejected) = ternary_operands(operands);
	match condition.i32() == 0 {
		true => rejected,
		false => selected,
	}
}

fn evaluate_bit_and(operands: &[Value]) -> Value {
	integer_binary(operands, |left, right| left & right)
}

fn evaluate_bit_or(operands: &[Value]) -> Value {
	integer_binary(operands, |left, right| left | right)
}

fn evaluate_bit_xor(operands: &[Value]) -> Value {
	integer_binary(operands, |left, right| left ^ right)
}

fn evaluate_bit_not(operands: &[Value]) -> Value {
	Value::I32(!unary_operand(operands).i32())
}

fn evaluate_bitcast_f32_to_i32(operands: &[Value]) -> Value {
	Value::I32(i32::from_ne_bytes(
		unary_operand(operands).f32().to_ne_bytes(),
	))
}

fn evaluate_bitcast_i32_to_f32(operands: &[Value]) -> Value {
	Value::F32(f32::from_ne_bytes(
		unary_operand(operands).i32().to_ne_bytes(),
	))
}

fn shift_count(value: i32) -> u32 {
	u32::from_ne_bytes(value.to_ne_bytes()) & 31
}

fn evaluate_shift_left(operands: &[Value]) -> Value {
	integer_binary(operands, |left, right| {
		left.wrapping_shl(shift_count(right))
	})
}

fn evaluate_shift_right_logical(operands: &[Value]) -> Value {
	integer_binary(operands, |left, right| {
		let shifted = u32::from_ne_bytes(left.to_ne_bytes()) >> shift_count(right);
		i32::from_ne_bytes(shifted.to_ne_bytes())
	})
}

fn evaluate_shift_right_arithmetic(operands: &[Value]) -> Value {
	integer_binary(operands, |left, right| left >> shift_count(right))
}

fn evaluate_require(operands: &[Value]) -> Value {
	match unary_operand(operands).i32() == 0 {
		true => Value::I32(0),
		false => Value::I32(1),
	}
}

fn evaluate_is_finite(operands: &[Value]) -> Value {
	Value::I32(i32::from(unary_operand(operands).f32().is_finite()))
}

fn evaluate_is_nan(operands: &[Value]) -> Value {
	Value::I32(i32::from(unary_operand(operands).f32().is_nan()))
}

fn evaluate_square_root(operands: &[Value]) -> Value {
	Value::F32(unary_operand(operands).f32().sqrt())
}

fn evaluate_floor(operands: &[Value]) -> Value {
	Value::F32(unary_operand(operands).f32().floor())
}

fn evaluate_ceiling(operands: &[Value]) -> Value {
	Value::F32(unary_operand(operands).f32().ceil())
}

fn evaluate_round_nearest_even(operands: &[Value]) -> Value {
	Value::F32(unary_operand(operands).f32().round_ties_even())
}

fn evaluate_convert_f32_to_i32(operands: &[Value]) -> Value {
	let value = unary_operand(operands).f32();
	Value::I32(match value.is_finite() {
		true => parse_number(value.trunc()),
		false => 0,
	})
}

fn evaluate_convert_i32_to_f32(operands: &[Value]) -> Value {
	Value::F32(parse_number(unary_operand(operands).i32()))
}

fn parse_number<Source, Target>(value: Source) -> Target
where
	Source: ToString,
	Target: std::str::FromStr,
	Target::Err: std::fmt::Debug,
{
	match value.to_string().parse() {
		Ok(parsed) => parsed,
		Err(error) => panic!("test-interpreter numeric conversion failed: {error:?}"),
	}
}

fn evaluate_function(function: MathFunction, inputs: &[f32]) -> Evaluation {
	let math = match build(function) {
		Ok(math) => math,
		Err(error) => panic!("failed to build {function:?}: {error}"),
	};
	evaluate(&math.program, inputs)
}

fn assert_conforms(function: MathFunction, inputs: &[f32], expected: f32) {
	let math = match build(function) {
		Ok(math) => math,
		Err(error) => panic!("failed to build {function:?}: {error}"),
	};
	assert_program_conforms(&math, inputs, expected);
}

fn assert_program_conforms(math: &MathProgram, inputs: &[f32], expected: f32) {
	let evaluation = evaluate(&math.program, inputs);
	let function = math.function;
	assert!(
		!evaluation.faulted,
		"{function:?}{inputs:?} unexpectedly faulted"
	);
	let contract = function.contract();
	let absolute = (evaluation.output - expected).abs();
	let scale = expected.abs().max(f32::MIN_POSITIVE);
	let relative = absolute / scale;
	assert!(
		absolute <= contract.error.maximum_absolute || relative <= contract.error.maximum_relative,
		"{function:?}{inputs:?}: output={}, expected={expected}, absolute error={absolute}, \
		 relative error={relative}, contract={:?}",
		evaluation.output,
		contract.error
	);
}

fn sweep_unary(function: MathFunction, lower: f32, upper: f32, steps: u16, oracle: impl Fn(f32) -> f32) {
	let math = match build(function) {
		Ok(math) => math,
		Err(error) => panic!("failed to build {function:?}: {error}"),
	};
	for step in 0..=steps {
		let fraction = f32::from(step) / f32::from(steps);
		let input = (upper - lower).mul_add(fraction, lower);
		assert_program_conforms(&math, &[input], oracle(input));
	}
}

#[test]
fn every_function_builds_a_valid_versioned_program() {
	let mut identities = BTreeSet::new();
	for function in MathFunction::ALL {
		let first = match build(function) {
			Ok(math) => math,
			Err(error) => panic!("failed to build {function:?}: {error}"),
		};
		let second = match build(function) {
			Ok(math) => math,
			Err(error) => panic!("failed to rebuild {function:?}: {error}"),
		};
		assert_eq!(first, second);
		assert_eq!(first.function, function);
		assert_eq!(first.algorithm, function.algorithm());
		assert_eq!(
			first.algorithm.version,
			if function == MathFunction::Pow { 3 } else { 1 }
		);
		assert!(identities.insert(first.algorithm.name));
		assert_eq!(first.program.inputs.len(), function.arity());
		assert_eq!(function.contract().domain.inputs.len(), function.arity());
		assert!(
			first.program
				.inputs
				.iter()
				.all(|input| input.dtype == DType::F32)
		);
		assert_eq!(first.program.outputs.len(), 1);
		assert_eq!(
			first.program.dtype_of(first.program.outputs[0]),
			Some(DType::F32)
		);
		assert!(first.program.validate().is_ok());
		assert!(
			first.program
				.instructions
				.iter()
				.any(|instruction| instruction.opcode == ScalarOpcode::Require)
		);
	}
}

#[test]
fn representative_finite_inputs_respect_the_error_contracts() {
	for value in [-10.0_f32, -1.0, -0.125, 0.125, 1.0, 10.0] {
		assert_conforms(MathFunction::Reciprocal, &[value], 1.0 / value);
	}
	for value in [f32::MIN_POSITIVE, 1.0e-12, 0.001, 1.0, 2.0, 100.0, f32::MAX] {
		assert_conforms(
			MathFunction::ReciprocalSquareRoot,
			&[value],
			1.0 / value.sqrt(),
		);
	}
	for value in [
		-8192.0_f32,
		-100.0,
		-core::f32::consts::PI,
		-1.0,
		-0.0,
		0.0,
		0.1,
		1.0,
		core::f32::consts::PI,
		100.0,
		8192.0,
	] {
		assert_conforms(MathFunction::Sin, &[value], value.sin());
		assert_conforms(MathFunction::Cos, &[value], value.cos());
	}
	for value in [-1.4_f32, -1.0, -0.1, -0.0, 0.0, 0.1, 1.0, 1.4] {
		assert_conforms(MathFunction::Tan, &[value], value.tan());
	}
	for [y, x] in [
		[0.0_f32, 1.0],
		[-0.0, 1.0],
		[0.0, -1.0],
		[-0.0, -1.0],
		[1.0, 0.0],
		[-1.0, 0.0],
		[1.0, 1.0],
		[-1.0, 1.0],
		[1.0, -1.0],
		[-1.0, -1.0],
		[0.001, 1000.0],
		[1000.0, 0.001],
	] {
		assert_conforms(MathFunction::Atan2, &[y, x], y.atan2(x));
	}
	for value in [-80.0_f32, -20.0, -1.0, -0.0, 0.0, 1.0, 10.0, 80.0] {
		assert_conforms(MathFunction::Exp, &[value], value.exp());
		assert_conforms(MathFunction::ExpMinusOne, &[value], value.exp_m1());
		assert_conforms(
			MathFunction::Sigmoid,
			&[value],
			match value >= 0.0 {
				true => 1.0 / (1.0 + (-value).exp()),
				false => value.exp() / (1.0 + value.exp()),
			},
		);
		assert_conforms(
			MathFunction::Softplus,
			&[value],
			value.max(0.0) + (-value.abs()).exp().ln_1p(),
		);
	}
	for value in [-0.25_f32, -1.0e-6, -0.0, 0.0, 1.0e-6, 0.25] {
		assert_conforms(MathFunction::ExpMinusOne, &[value], value.exp_m1());
	}
	for value in [
		f32::from_bits(1),
		f32::MIN_POSITIVE,
		1.0e-20,
		0.1,
		1.0,
		2.0,
		100.0,
		f32::MAX,
	] {
		assert_conforms(MathFunction::Log, &[value], value.ln());
	}
	for value in [
		-0.999_999_94_f32,
		-0.75,
		-0.1,
		-1.0e-6,
		-0.0,
		0.0,
		1.0e-6,
		0.1,
		1.0,
		100.0,
		f32::MAX,
	] {
		assert_conforms(MathFunction::LogOnePlus, &[value], value.ln_1p());
	}
	for value in [-12.75_f32, -1.5, -0.0, 0.0, 1.5, 12.75, 16_777_216.0] {
		assert_conforms(MathFunction::Floor, &[value], value.floor());
		assert_conforms(MathFunction::Ceil, &[value], value.ceil());
		assert_conforms(
			MathFunction::RoundNearestEven,
			&[value],
			value.round_ties_even(),
		);
		assert_conforms(MathFunction::Trunc, &[value], value.trunc());
		assert_conforms(
			MathFunction::Sign,
			&[value],
			match (value < 0.0, value > 0.0) {
				(true, false) => -1.0,
				(false, true) => 1.0,
				(false, false) => value,
				(true, true) => panic!("ordered value cannot be both negative and positive"),
			},
		);
	}
	for [dividend, divisor] in [
		[-10.0_f32, 3.0],
		[-0.0, 3.0],
		[0.0, -3.0],
		[1.0, 0.25],
		[10.0, -3.0],
	] {
		assert_conforms(MathFunction::Fmod, &[dividend, divisor], dividend % divisor);
	}
	for [base, exponent] in [
		[0.1_f32, -3.0],
		[0.5, 0.5],
		[1.0, -1000.0],
		[2.0, -10.0],
		[2.0, 10.0],
		[10.0, 3.0],
	] {
		assert_conforms(MathFunction::Pow, &[base, exponent], base.powf(exponent));
	}
	for value in [-40.0_f32, -10.0, -1.0, -0.0, 0.0, 1.0, 10.0, 40.0] {
		assert_conforms(MathFunction::Tanh, &[value], value.tanh());
	}
	for value in [-6.0_f32, -3.0, -1.0, -0.0, 0.0, 1.0, 3.0, 6.0] {
		assert_conforms(MathFunction::Erf, &[value], erf_reference(value));
	}
}

#[test]
fn dense_finite_sweeps_respect_the_error_contracts() {
	sweep_unary(MathFunction::Sin, -8192.0, 8192.0, 8192, f32::sin);
	sweep_unary(MathFunction::Cos, -8192.0, 8192.0, 8192, f32::cos);
	sweep_unary(MathFunction::Tan, -1.4, 1.4, 4096, f32::tan);
	sweep_unary(MathFunction::Exp, -80.0, 80.0, 4096, f32::exp);
	sweep_unary(MathFunction::ExpMinusOne, -80.0, 80.0, 4096, f32::exp_m1);
	sweep_unary(MathFunction::Tanh, -40.0, 40.0, 4096, f32::tanh);
	sweep_unary(MathFunction::Erf, -6.0, 6.0, 4096, erf_reference);
	sweep_unary(MathFunction::Sigmoid, -80.0, 80.0, 4096, |value| {
		match value >= 0.0 {
			true => 1.0 / (1.0 + (-value).exp()),
			false => value.exp() / (1.0 + value.exp()),
		}
	});
	sweep_unary(MathFunction::Softplus, -80.0, 80.0, 4096, |value| {
		value.max(0.0) + (-value.abs()).exp().ln_1p()
	});
	sweep_unary(MathFunction::Log, f32::from_bits(1), 1.0, 4096, f32::ln);
	sweep_unary(MathFunction::LogOnePlus, -0.999_99, 100.0, 8192, f32::ln_1p);

	let logarithm = match build(MathFunction::Log) {
		Ok(math) => math,
		Err(error) => panic!("failed to build Log: {error}"),
	};
	for fraction_bit in 0..=22_u32 {
		let input = f32::from_bits(1_u32 << fraction_bit);
		assert_program_conforms(&logarithm, &[input], input.ln());
	}
	for biased_exponent in 1..=254_u32 {
		let input = f32::from_bits(biased_exponent << 23);
		assert_program_conforms(&logarithm, &[input], input.ln());
	}
	let log_one_plus = match build(MathFunction::LogOnePlus) {
		Ok(math) => math,
		Err(error) => panic!("failed to build LogOnePlus: {error}"),
	};
	for exponent in -24_i32..=-1 {
		let input = 2.0_f32.powi(exponent);
		assert_program_conforms(&log_one_plus, &[input], input.ln_1p());
		assert_program_conforms(&log_one_plus, &[-input], (-input).ln_1p());
	}

	let atan2 = match build(MathFunction::Atan2) {
		Ok(math) => math,
		Err(error) => panic!("failed to build Atan2: {error}"),
	};
	for y_step in 0..=64_u16 {
		for x_step in 0..=64_u16 {
			let y = (f32::from(y_step) - 31.5) / 8.0;
			let x = (f32::from(x_step) - 31.5) / 8.0;
			assert_program_conforms(&atan2, &[y, x], y.atan2(x));
		}
	}

	let pow = match build(MathFunction::Pow) {
		Ok(math) => math,
		Err(error) => panic!("failed to build Pow: {error}"),
	};
	for base_step in 0..=64_u16 {
		for exponent_step in 0..=64_u16 {
			let logarithm = (f32::from(base_step) - 32.0) / 8.0;
			let base = logarithm.exp();
			let exponent = (f32::from(exponent_step) - 32.0) / 3.2;
			assert_program_conforms(&pow, &[base, exponent], base.powf(exponent));
		}
	}
}

#[test]
fn positive_base_pow_crosses_adam_beta_one_underflow_without_faulting() {
	let math = match build(MathFunction::Pow) {
		Ok(math) => math,
		Err(error) => panic!("failed to build Pow: {error}"),
	};
	for step in [759.0_f32, 760.0, 900.0, 1_000.0, 4_400.0] {
		let expected = 0.9_f32.powf(step);
		assert_program_conforms(&math, &[0.9, step], expected);
		let evaluation = evaluate(&math.program, &[0.9, step]);
		assert!(
			!evaluation.faulted,
			"Adam beta-one power faulted at step {step}"
		);
		match step as u32 {
			759 | 760 => assert!(
				evaluation.output.is_normal(),
				"beta-one power should remain normal at step {step}, got {}",
				evaluation.output
			),
			900 => assert!(
				evaluation.output.is_subnormal(),
				"beta-one power should be subnormal at step 900, got {}",
				evaluation.output
			),
			1_000 | 4_400 => assert_eq!(
				evaluation.output.to_bits(),
				0.0_f32.to_bits(),
				"true binary32 underflow must round to positive zero at step {step}"
			),
			_ => unreachable!("the test enumerates exact Adam steps"),
		}
	}
}

#[test]
fn nonpositive_exp_preserves_distinct_subnormal_tail_then_rounds_true_underflow_to_zero() {
	let program = exp_with_gradual_underflow_program().unwrap();
	let at_negative_ninety = evaluate(&program, &[-90.0]);
	let at_negative_one_hundred = evaluate(&program, &[-100.0]);
	let at_negative_one_hundred_three = evaluate(&program, &[-103.0]);
	for evaluation in [
		&at_negative_ninety,
		&at_negative_one_hundred,
		&at_negative_one_hundred_three,
	] {
		assert!(!evaluation.faulted);
		assert!(evaluation.output.is_subnormal());
		assert!(evaluation.output.is_sign_positive());
	}
	assert!(at_negative_ninety.output > at_negative_one_hundred.output);
	assert!(at_negative_one_hundred.output > at_negative_one_hundred_three.output);

	let stored_cutoff = -103.972_08_f32;
	let cutoff = evaluate(&program, &[stored_cutoff]);
	assert!(!cutoff.faulted);
	assert_eq!(cutoff.output.to_bits(), 1);
	let next_lower = f32::from_bits(stored_cutoff.to_bits() + 1);
	let below_cutoff = evaluate(&program, &[next_lower]);
	assert!(!below_cutoff.faulted);
	assert_eq!(below_cutoff.output.to_bits(), 0.0_f32.to_bits());

	let true_underflow = evaluate(&program, &[-104.0]);
	assert!(!true_underflow.faulted);
	assert_eq!(true_underflow.output.to_bits(), 0.0_f32.to_bits());

	assert!(evaluate(&program, &[1.0]).faulted);
	assert!(evaluate(&program, &[f32::NEG_INFINITY]).faulted);
}

#[test]
fn rejected_inputs_set_the_fault_flag() {
	for function in MathFunction::ALL {
		for input_index in 0..function.arity() {
			let mut inputs = vec![1.0; function.arity()];
			inputs[input_index] = f32::NAN;
			assert!(
				evaluate_function(function, &inputs).faulted,
				"{function:?} accepted NaN in input {input_index}"
			);
			inputs[input_index] = f32::INFINITY;
			assert!(
				evaluate_function(function, &inputs).faulted,
				"{function:?} accepted infinity in input {input_index}"
			);
			inputs[input_index] = f32::NEG_INFINITY;
			assert!(
				evaluate_function(function, &inputs).faulted,
				"{function:?} accepted negative infinity in input {input_index}"
			);
		}
	}

	for (function, inputs) in [
		(MathFunction::Reciprocal, vec![0.0]),
		(MathFunction::ReciprocalSquareRoot, vec![0.0]),
		(MathFunction::ReciprocalSquareRoot, vec![-1.0]),
		(MathFunction::Sin, vec![8193.0]),
		(MathFunction::Cos, vec![-8193.0]),
		(MathFunction::Tan, vec![1.5]),
		(MathFunction::Atan2, vec![0.0, 0.0]),
		(MathFunction::Exp, vec![81.0]),
		(MathFunction::ExpMinusOne, vec![-81.0]),
		(MathFunction::Log, vec![0.0]),
		(MathFunction::Log, vec![-1.0]),
		(MathFunction::LogOnePlus, vec![-1.0]),
		(MathFunction::Fmod, vec![1.0, 0.0]),
		(MathFunction::Pow, vec![0.0, 1.0]),
		(MathFunction::Pow, vec![2.0, 116.0]),
		(MathFunction::Sigmoid, vec![81.0]),
		(MathFunction::Tanh, vec![-41.0]),
		(MathFunction::Softplus, vec![-81.0]),
		(MathFunction::Erf, vec![7.0]),
	] {
		assert!(
			evaluate_function(function, &inputs).faulted,
			"{function:?}{inputs:?} did not fault"
		);
	}
}

#[test]
fn accepted_signed_zero_results_are_preserved_exactly() {
	for function in [
		MathFunction::Sin,
		MathFunction::Tan,
		MathFunction::ExpMinusOne,
		MathFunction::LogOnePlus,
		MathFunction::Floor,
		MathFunction::Ceil,
		MathFunction::RoundNearestEven,
		MathFunction::Trunc,
		MathFunction::Sign,
		MathFunction::Tanh,
		MathFunction::Erf,
	] {
		for zero in [0.0_f32, -0.0] {
			let evaluation = evaluate_function(function, &[zero]);
			assert!(!evaluation.faulted);
			assert_eq!(
				evaluation.output.to_bits(),
				zero.to_bits(),
				"{function:?} did not preserve {zero:?}"
			);
		}
	}

	for zero in [0.0_f32, -0.0] {
		let fmod = evaluate_function(MathFunction::Fmod, &[zero, 3.0]);
		assert!(!fmod.faulted);
		assert_eq!(fmod.output.to_bits(), zero.to_bits());
		for x in [1.0_f32, -1.0] {
			let atan2 = evaluate_function(MathFunction::Atan2, &[zero, x]);
			assert!(!atan2.faulted);
			assert_eq!(atan2.output.is_sign_negative(), zero.is_sign_negative());
		}
	}
}

fn erf_reference(value: f32) -> f32 {
	let x = f64::from(value);
	let sign = match x.is_sign_negative() {
		true => -1.0,
		false => 1.0,
	};
	let x = x.abs();
	let t = 1.0 / (1.0 + 0.5 * x);
	let exponent = -x * x - 1.265_512_23
		+ t * (1.000_023_68
			+ t * (0.374_091_96
				+ t * (0.096_784_18
					+ t * (-0.186_288_06
						+ t * (0.278_868_07
							+ t * (-1.135_203_98
								+ t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
	parse_number(sign * (1.0 - t * exponent.exp()))
}
