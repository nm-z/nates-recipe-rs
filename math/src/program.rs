use recipe_core::{DType, ScalarOpcode, ScalarProgram};
use recipe_language::{LanguageResult, ScalarExpression, ScalarProgramBuilder};

use crate::MathFunction;

const LOG_TWO_HIGH: f32 = 0.693_145_75; const LOG_TWO_LOW: f32 = 1.428_606_8e-6;
const LOG_TWO_RECIPROCAL: f32 = core::f32::consts::LOG2_E;
// ln(2^-150): binary32 exp results at or below this half-min-subnormal
// tie round to positive zero under round-to-nearest-even.
const EXP_HALF_MIN_SUBNORMAL_LOG: f32 = -103.972_08; const EXP_SPLIT_SCALE_BITS: i32 = 64;
const PI_OVER_TWO_HIGH: f32 = f32::from_bits(0x3fc9_0fda); const PI_OVER_TWO_LOW: f32 = 7.549_789_4e-8;
const TWO_OVER_PI: f32 = core::f32::consts::FRAC_2_PI;

/// Build one deterministic, backend-neutral f32 scalar math program.
pub(crate) fn build(function: MathFunction) -> LanguageResult<ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let mut inputs = Vec::with_capacity(function.arity());
	for _ in 0..function.arity() { inputs.push(builder.input(DType::F32)?); }
	require_finite_inputs(&mut builder, &inputs)?; let output = emit_function(&mut builder, function, &inputs)?;
	let program = builder.finish(&[output])?; Ok(program) }

fn emit_function( builder: &mut ScalarProgramBuilder, function: MathFunction, inputs: &[ScalarExpression],
) -> LanguageResult<ScalarExpression> { let x = inputs[0]; match function { MathFunction::Reciprocal => {
			require_nonzero(builder, x)?; let one = builder.f32(1.0)?; builder.binary(ScalarOpcode::Divide, one, x) }
		MathFunction::ReciprocalSquareRoot => { require_positive(builder, x)?;
			let root = builder.unary(ScalarOpcode::SquareRoot, x)?; let one = builder.f32(1.0)?;
			builder.binary(ScalarOpcode::Divide, one, root) }
		MathFunction::Sin => { require_closed_interval(builder, x, -8192.0, 8192.0)?;
			let (sine, _) = emit_sine_cosine_core(builder, x)?; preserve_zero(builder, x, sine) }
		MathFunction::Cos => { require_closed_interval(builder, x, -8192.0, 8192.0)?;
			let (_, cosine) = emit_sine_cosine_core(builder, x)?; Ok(cosine) }
		MathFunction::Tan => { require_closed_interval(builder, x, -1.4, 1.4)?;
			let (sine, cosine) = emit_sine_cosine_core(builder, x)?;
			let tangent = builder.binary(ScalarOpcode::Divide, sine, cosine)?; preserve_zero(builder, x, tangent) }
		MathFunction::Atan2 => { let y = x; let x = inputs[1]; require_atan2_nonzero_pair(builder, y, x)?;
			emit_atan2_core(builder, y, x) }
		MathFunction::Exp => { require_closed_interval(builder, x, -80.0, 80.0)?; emit_exp_core(builder, x) }
		MathFunction::ExpWithGradualUnderflow => { let zero = builder.f32(0.0)?;
			let nonpositive = builder.binary(ScalarOpcode::LessThanOrEqual, x, zero)?; require_condition(builder, nonpositive)?;
			emit_exp_with_underflow(builder, x) }
		MathFunction::ExpMinusOne => { require_closed_interval(builder, x, -80.0, 80.0)?;
			let result = emit_expm1_core(builder, x)?; preserve_zero(builder, x, result) }
		MathFunction::Log => { require_positive(builder, x)?; emit_log_core(builder, x) }
		MathFunction::LogOnePlus => { let negative_one = builder.f32(-1.0)?;
			let valid = builder.binary(ScalarOpcode::GreaterThan, x, negative_one)?; require_condition(builder, valid)?;
			let result = emit_log1p_core(builder, x)?; preserve_zero(builder, x, result) }
		MathFunction::Floor => builder.unary(ScalarOpcode::Floor, x),
		MathFunction::Ceil => builder.unary(ScalarOpcode::Ceiling, x),
		MathFunction::RoundNearestEven => builder.unary(ScalarOpcode::RoundNearestEven, x),
		MathFunction::Trunc => emit_trunc(builder, x), MathFunction::Fmod => { let divisor = inputs[1];
			require_nonzero(builder, divisor)?; let result = builder.binary(ScalarOpcode::Remainder, x, divisor)?;
			preserve_zero(builder, x, result) }
		MathFunction::Pow => { let exponent = inputs[1]; require_positive(builder, x)?;
			let logarithm = emit_log_core(builder, x)?; let power = builder.binary(ScalarOpcode::Multiply, exponent, logarithm)?;
			let limit = builder.f32(80.0)?; let in_range = builder.binary(ScalarOpcode::LessThanOrEqual, power, limit)?;
			require_condition(builder, in_range)?; emit_exp_with_underflow(builder, power) }
		MathFunction::Sign => emit_sign(builder, x), MathFunction::Sigmoid => {
			require_closed_interval(builder, x, -80.0, 80.0)?; emit_sigmoid_core(builder, x) }
		MathFunction::Tanh => { require_closed_interval(builder, x, -40.0, 40.0)?; let result = emit_tanh_core(builder, x)?;
			preserve_zero(builder, x, result) }
		MathFunction::Softplus => { require_closed_interval(builder, x, -80.0, 80.0)?; emit_softplus_core(builder, x) }
		MathFunction::Erf => { require_closed_interval(builder, x, -6.0, 6.0)?; let result = emit_erf_core(builder, x)?;
			preserve_zero(builder, x, result) } } }

fn require_finite_inputs(builder: &mut ScalarProgramBuilder, inputs: &[ScalarExpression]) -> LanguageResult<()> {
	for input in inputs { let finite = builder.unary(ScalarOpcode::IsFinite, *input)?; require_condition(builder, finite)?;
	}
	Ok(()) }

fn require_nonzero(builder: &mut ScalarProgramBuilder, value: ScalarExpression) -> LanguageResult<()> {
	let zero = builder.f32(0.0)?; let nonzero = builder.binary(ScalarOpcode::NotEqual, value, zero)?;
	require_condition(builder, nonzero) }

fn require_positive(builder: &mut ScalarProgramBuilder, value: ScalarExpression) -> LanguageResult<()> {
	let zero = builder.f32(0.0)?; let positive = builder.binary(ScalarOpcode::GreaterThan, value, zero)?;
	require_condition(builder, positive) }

fn require_closed_interval( builder: &mut ScalarProgramBuilder, value: ScalarExpression, lower: f32, upper: f32,
) -> LanguageResult<()> { let lower = builder.f32(lower)?;
	let at_least_lower = builder.binary(ScalarOpcode::GreaterThanOrEqual, value, lower)?;
	require_condition(builder, at_least_lower)?; let upper = builder.f32(upper)?;
	let at_most_upper = builder.binary(ScalarOpcode::LessThanOrEqual, value, upper)?;
	require_condition(builder, at_most_upper) }

fn require_condition(builder: &mut ScalarProgramBuilder, condition: ScalarExpression) -> LanguageResult<()> {
	builder.unary(ScalarOpcode::Require, condition)?; Ok(()) }

fn require_atan2_nonzero_pair( builder: &mut ScalarProgramBuilder, y: ScalarExpression, x: ScalarExpression,
) -> LanguageResult<()> { let zero = builder.f32(0.0)?;
	let y_nonzero = builder.binary(ScalarOpcode::NotEqual, y, zero)?;
	let x_nonzero = builder.binary(ScalarOpcode::NotEqual, x, zero)?;
	let either_nonzero = builder.binary(ScalarOpcode::BitOr, y_nonzero, x_nonzero)?;
	require_condition(builder, either_nonzero) }

fn preserve_zero( builder: &mut ScalarProgramBuilder, input: ScalarExpression, result: ScalarExpression,
) -> LanguageResult<ScalarExpression> { let zero = builder.f32(0.0)?;
	let input_is_zero = builder.binary(ScalarOpcode::Equal, input, zero)?;
	builder.ternary(ScalarOpcode::Select, input_is_zero, input, result) }

fn horner( builder: &mut ScalarProgramBuilder, x: ScalarExpression, first: f32, remaining: &[f32],
) -> LanguageResult<ScalarExpression> { let mut accumulator = builder.f32(first)?; for coefficient in remaining {
		let coefficient = builder.f32(*coefficient)?;
		accumulator = builder.ternary(ScalarOpcode::Fma, accumulator, x, coefficient)?; }
	Ok(accumulator) }

fn emit_sine_cosine_core( builder: &mut ScalarProgramBuilder, x: ScalarExpression,
) -> LanguageResult<(ScalarExpression, ScalarExpression)> { let reciprocal = builder.f32(TWO_OVER_PI)?;
	let scaled = builder.binary(ScalarOpcode::Multiply, x, reciprocal)?;
	let nearest = builder.unary(ScalarOpcode::RoundNearestEven, scaled)?;
	let negative_high = builder.f32(-PI_OVER_TWO_HIGH)?;
	let reduced_high = builder.ternary(ScalarOpcode::Fma, nearest, negative_high, x)?;
	let negative_low = builder.f32(-PI_OVER_TWO_LOW)?;
	let reduced = builder.ternary(ScalarOpcode::Fma, nearest, negative_low, reduced_high)?;
	let squared = builder.binary(ScalarOpcode::Multiply, reduced, reduced)?;

	let sine_polynomial = horner(builder, squared, 1.589_691e-10, &[ -2.505_076e-8, 2.755_731_4e-6, -1.984_127e-4,
		8.333_334e-3, -1.666_666_7e-1, ])?; let reduced_cubed = builder.binary(ScalarOpcode::Multiply, reduced, squared)?;
	let sine = builder.ternary(ScalarOpcode::Fma, reduced_cubed, sine_polynomial, reduced)?;

	let cosine_polynomial = horner(builder, squared, -1.135_965e-11, &[ 2.087_572_3e-9, -2.755_731_4e-7, 2.480_158_8e-5,
		-1.388_888_9e-3, 4.166_666_8e-2, ])?; let negative_half = builder.f32(-0.5)?;
	let cosine_inner = builder.ternary(ScalarOpcode::Fma, squared, cosine_polynomial, negative_half)?;
	let one = builder.f32(1.0)?; let cosine = builder.ternary(ScalarOpcode::Fma, squared, cosine_inner, one)?;

	let quadrant = builder.unary(ScalarOpcode::ConvertF32ToI32, nearest)?; let three = builder.i32(3)?;
	let quadrant = builder.binary(ScalarOpcode::BitAnd, quadrant, three)?; let zero = builder.i32(0)?;
	let two = builder.i32(2)?; let is_zero = builder.binary(ScalarOpcode::Equal, quadrant, zero)?;
	let is_two = builder.binary(ScalarOpcode::Equal, quadrant, two)?;
	let is_lower_half = builder.binary(ScalarOpcode::LessThan, quadrant, two)?;
	let negative_sine = builder.unary(ScalarOpcode::Negate, sine)?;
	let negative_cosine = builder.unary(ScalarOpcode::Negate, cosine)?;

	let sine_lower = builder.ternary(ScalarOpcode::Select, is_zero, sine, cosine)?;
	let sine_upper = builder.ternary(ScalarOpcode::Select, is_two, negative_sine, negative_cosine)?;
	let selected_sine = builder.ternary(ScalarOpcode::Select, is_lower_half, sine_lower, sine_upper)?;
	let cosine_lower = builder.ternary(ScalarOpcode::Select, is_zero, cosine, negative_sine)?;
	let cosine_upper = builder.ternary(ScalarOpcode::Select, is_two, negative_cosine, sine)?;
	let selected_cosine = builder.ternary( ScalarOpcode::Select, is_lower_half, cosine_lower, cosine_upper, )?;
	Ok((selected_sine, selected_cosine)) }

fn emit_atan_polynomial(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let squared = builder.binary(ScalarOpcode::Multiply, x, x)?; let polynomial = horner(builder, squared, -1.0 / 15.0, &[
		1.0 / 13.0, -1.0 / 11.0, 1.0 / 9.0, -1.0 / 7.0, 1.0 / 5.0, -1.0 / 3.0, ])?;
	let cubed = builder.binary(ScalarOpcode::Multiply, x, squared)?;
	builder.ternary(ScalarOpcode::Fma, cubed, polynomial, x) }

fn emit_atan2_core( builder: &mut ScalarProgramBuilder, y: ScalarExpression, x: ScalarExpression,
) -> LanguageResult<ScalarExpression> { let absolute_y = builder.unary(ScalarOpcode::Absolute, y)?;
	let absolute_x = builder.unary(ScalarOpcode::Absolute, x)?;
	let swap = builder.binary(ScalarOpcode::GreaterThan, absolute_y, absolute_x)?;
	let minimum = builder.ternary(ScalarOpcode::Select, swap, absolute_x, absolute_y)?;
	let maximum = builder.ternary(ScalarOpcode::Select, swap, absolute_y, absolute_x)?;
	let ratio = builder.binary(ScalarOpcode::Divide, minimum, maximum)?;

	let threshold = builder.f32(0.414_213_57)?;
	let transformed_region = builder.binary(ScalarOpcode::GreaterThan, ratio, threshold)?; let one = builder.f32(1.0)?;
	let numerator = builder.binary(ScalarOpcode::Subtract, ratio, one)?;
	let denominator = builder.binary(ScalarOpcode::Add, ratio, one)?;
	let transformed = builder.binary(ScalarOpcode::Divide, numerator, denominator)?;
	let direct_angle = emit_atan_polynomial(builder, ratio)?;
	let transformed_angle = emit_atan_polynomial(builder, transformed)?;
	let quarter_pi = builder.f32(core::f32::consts::FRAC_PI_4)?;
	let transformed_angle = builder.binary(ScalarOpcode::Add, quarter_pi, transformed_angle)?;
	let reduced_angle = builder.ternary( ScalarOpcode::Select, transformed_region, transformed_angle, direct_angle, )?;

	let half_pi = builder.f32(core::f32::consts::FRAC_PI_2)?;
	let swapped_angle = builder.binary(ScalarOpcode::Subtract, half_pi, reduced_angle)?;
	let first_quadrant = builder.ternary(ScalarOpcode::Select, swap, swapped_angle, reduced_angle)?;
	let zero = builder.f32(0.0)?; let x_negative = builder.binary(ScalarOpcode::LessThan, x, zero)?;
	let pi = builder.f32(core::f32::consts::PI)?;
	let reflected = builder.binary(ScalarOpcode::Subtract, pi, first_quadrant)?;
	let unsigned_angle = builder.ternary(ScalarOpcode::Select, x_negative, reflected, first_quadrant)?;
	copy_sign_from(builder, unsigned_angle, y) }

fn emit_exp_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> { emit_exp_reconstruction(builder, x, false) }

/// Reconstruct exp across the binary32 subnormal range and round values at or
/// below half of the minimum subnormal to positive zero.
fn emit_exp_with_underflow( builder: &mut ScalarProgramBuilder, x: ScalarExpression,
) -> LanguageResult<ScalarExpression> { let cutoff = builder.f32(EXP_HALF_MIN_SUBNORMAL_LOG)?;
	let underflows = builder.binary(ScalarOpcode::LessThan, x, cutoff)?;
	// Clamp before f32-to-i32 range reduction so a finite multiplication that
	// overflowed toward negative infinity cannot enter numeric conversion.
	let bounded = builder.binary(ScalarOpcode::Maximum, x, cutoff)?;
	let reconstructed = emit_exp_reconstruction(builder, bounded, true)?; let zero = builder.f32(0.0)?;
	builder.ternary(ScalarOpcode::Select, underflows, zero, reconstructed) }

fn emit_exp_reconstruction( builder: &mut ScalarProgramBuilder, x: ScalarExpression, support_subnormals: bool,
) -> LanguageResult<ScalarExpression> { let log_two_reciprocal = builder.f32(LOG_TWO_RECIPROCAL)?;
	let scaled = builder.binary(ScalarOpcode::Multiply, x, log_two_reciprocal)?;
	let nearest = builder.unary(ScalarOpcode::RoundNearestEven, scaled)?; let negative_high = builder.f32(-LOG_TWO_HIGH)?;
	let reduced_high = builder.ternary(ScalarOpcode::Fma, nearest, negative_high, x)?;
	let negative_low = builder.f32(-LOG_TWO_LOW)?;
	let reduced = builder.ternary(ScalarOpcode::Fma, nearest, negative_low, reduced_high)?;
	let polynomial = horner(builder, reduced, 1.0 / 5040.0, &[ 1.0 / 720.0, 1.0 / 120.0, 1.0 / 24.0, 1.0 / 6.0, 0.5, 1.0,
		1.0, ])?;

	let exponent = builder.unary(ScalarOpcode::ConvertF32ToI32, nearest)?; let bias = builder.i32(127)?;
	let shift = builder.i32(23)?; if !support_subnormals { let biased = builder.binary(ScalarOpcode::Add, exponent, bias)?;
		let exponent_bits = builder.binary(ScalarOpcode::ShiftLeft, biased, shift)?;
		let power_of_two = builder.unary(ScalarOpcode::BitcastI32ToF32, exponent_bits)?;
		return builder.binary(ScalarOpcode::Multiply, polynomial, power_of_two); }

	// A direct 2^k bit construction is valid only for normal k in
	// [-126, 127]. Split lower exponents into 2^(k+64) * 2^-64 so the first
	// product stays normal and only the final IEEE multiplication rounds into
	// subnormal storage.
	let minimum_normal_exponent = builder.i32(-126)?;
	let needs_lower_split = builder.binary(ScalarOpcode::LessThan, exponent, minimum_normal_exponent)?;
	let split = builder.i32(EXP_SPLIT_SCALE_BITS)?;
	let adjusted_lower = builder.binary(ScalarOpcode::Add, exponent, split)?; let reconstructed_exponent = builder.ternary(
		ScalarOpcode::Select, needs_lower_split, adjusted_lower, exponent, )?;
	let biased = builder.binary(ScalarOpcode::Add, reconstructed_exponent, bias)?;
	let exponent_bits = builder.binary(ScalarOpcode::ShiftLeft, biased, shift)?;
	let power_of_two = builder.unary(ScalarOpcode::BitcastI32ToF32, exponent_bits)?;
	let normal_result = builder.binary(ScalarOpcode::Multiply, polynomial, power_of_two)?;
	let lower_scale = builder.f32(f32::from_bits((127 - EXP_SPLIT_SCALE_BITS as u32) << 23))?; let one = builder.f32(1.0)?;
	let tail_scale = builder.ternary(ScalarOpcode::Select, needs_lower_split, lower_scale, one)?;
	builder.binary(ScalarOpcode::Multiply, normal_result, tail_scale) }

fn emit_expm1_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let absolute = builder.unary(ScalarOpcode::Absolute, x)?; let cutoff = builder.f32(0.25)?;
	let use_series = builder.binary(ScalarOpcode::LessThanOrEqual, absolute, cutoff)?;
	let series_polynomial = horner(builder, x, 1.0 / 40320.0, &[ 1.0 / 5040.0, 1.0 / 720.0, 1.0 / 120.0, 1.0 / 24.0,
		1.0 / 6.0, 0.5, 1.0, ])?; let series = builder.binary(ScalarOpcode::Multiply, x, series_polynomial)?;
	let exponential = emit_exp_core(builder, x)?; let one = builder.f32(1.0)?;
	let regular = builder.binary(ScalarOpcode::Subtract, exponential, one)?;
	builder.ternary(ScalarOpcode::Select, use_series, series, regular) }

fn emit_log_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let minimum_normal = builder.f32(f32::MIN_POSITIVE)?;
	let subnormal = builder.binary(ScalarOpcode::LessThan, x, minimum_normal)?;
	let subnormal_scale = builder.f32(8_388_608.0)?;
	let scaled_subnormal = builder.binary(ScalarOpcode::Multiply, x, subnormal_scale)?;
	let normalized_input = builder.ternary(ScalarOpcode::Select, subnormal, scaled_subnormal, x)?;

	let bits = builder.unary(ScalarOpcode::BitcastF32ToI32, normalized_input)?; let exponent_shift = builder.i32(23)?;
	let exponent = builder.binary(ScalarOpcode::ShiftRightLogical, bits, exponent_shift)?;
	let exponent_mask = builder.i32(0xff)?; let exponent = builder.binary(ScalarOpcode::BitAnd, exponent, exponent_mask)?;
	let exponent_bias = builder.i32(127)?; let exponent = builder.binary(ScalarOpcode::Subtract, exponent, exponent_bias)?;
	let subnormal_adjustment = builder.i32(-23)?; let no_adjustment = builder.i32(0)?; let adjustment = builder.ternary(
		ScalarOpcode::Select, subnormal, subnormal_adjustment, no_adjustment, )?;
	let exponent = builder.binary(ScalarOpcode::Add, exponent, adjustment)?;

	let mantissa_mask = builder.i32(0x007f_ffff)?;
	let mantissa = builder.binary(ScalarOpcode::BitAnd, bits, mantissa_mask)?; let one_bits = builder.i32(0x3f80_0000)?;
	let mantissa = builder.binary(ScalarOpcode::BitOr, mantissa, one_bits)?;
	let mantissa = builder.unary(ScalarOpcode::BitcastI32ToF32, mantissa)?;
	let square_root_two = builder.f32(core::f32::consts::SQRT_2)?;
	let reduce_mantissa = builder.binary(ScalarOpcode::GreaterThan, mantissa, square_root_two)?;
	let half = builder.f32(0.5)?; let halved_mantissa = builder.binary(ScalarOpcode::Multiply, mantissa, half)?;
	let mantissa = builder.ternary( ScalarOpcode::Select, reduce_mantissa, halved_mantissa, mantissa, )?;
	let one_integer = builder.i32(1)?; let zero_integer = builder.i32(0)?; let exponent_increment = builder.ternary(
		ScalarOpcode::Select, reduce_mantissa, one_integer, zero_integer, )?;
	let exponent = builder.binary(ScalarOpcode::Add, exponent, exponent_increment)?;

	let one = builder.f32(1.0)?; let numerator = builder.binary(ScalarOpcode::Subtract, mantissa, one)?;
	let denominator = builder.binary(ScalarOpcode::Add, mantissa, one)?;
	let y = builder.binary(ScalarOpcode::Divide, numerator, denominator)?;
	let y_squared = builder.binary(ScalarOpcode::Multiply, y, y)?; let series = horner(builder, y_squared, 1.0 / 13.0, &[
		1.0 / 11.0, 1.0 / 9.0, 1.0 / 7.0, 1.0 / 5.0, 1.0 / 3.0, 1.0, ])?;
	let twice_y = builder.binary(ScalarOpcode::Add, y, y)?;
	let mantissa_log = builder.binary(ScalarOpcode::Multiply, twice_y, series)?;
	let exponent = builder.unary(ScalarOpcode::ConvertI32ToF32, exponent)?; let log_two_high = builder.f32(LOG_TWO_HIGH)?;
	let high = builder.ternary(ScalarOpcode::Fma, exponent, log_two_high, mantissa_log)?;
	let log_two_low = builder.f32(LOG_TWO_LOW)?; builder.ternary(ScalarOpcode::Fma, exponent, log_two_low, high) }

fn emit_log1p_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let absolute = builder.unary(ScalarOpcode::Absolute, x)?; let cutoff = builder.f32(0.25)?;
	let use_series = builder.binary(ScalarOpcode::LessThanOrEqual, absolute, cutoff)?;
	let series_polynomial = horner(builder, x, -1.0 / 12.0, &[ 1.0 / 11.0, -1.0 / 10.0, 1.0 / 9.0, -1.0 / 8.0, 1.0 / 7.0,
		-1.0 / 6.0, 1.0 / 5.0, -0.25, 1.0 / 3.0, -0.5, 1.0, ])?;
	let series = builder.binary(ScalarOpcode::Multiply, x, series_polynomial)?; let one = builder.f32(1.0)?;
	let shifted = builder.binary(ScalarOpcode::Add, one, x)?; let regular = emit_log_core(builder, shifted)?;
	builder.ternary(ScalarOpcode::Select, use_series, series, regular) }

fn emit_trunc(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let floor = builder.unary(ScalarOpcode::Floor, x)?; let ceiling = builder.unary(ScalarOpcode::Ceiling, x)?;
	let zero = builder.f32(0.0)?; let negative = builder.binary(ScalarOpcode::LessThan, x, zero)?;
	builder.ternary(ScalarOpcode::Select, negative, ceiling, floor) }

fn emit_sign(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let zero = builder.f32(0.0)?; let negative = builder.binary(ScalarOpcode::LessThan, x, zero)?;
	let positive = builder.binary(ScalarOpcode::GreaterThan, x, zero)?; let one = builder.f32(1.0)?;
	let negative_one = builder.f32(-1.0)?; let positive_or_zero = builder.ternary(ScalarOpcode::Select, positive, one, x)?;
	builder.ternary( ScalarOpcode::Select, negative, negative_one, positive_or_zero, ) }

fn emit_sigmoid_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let zero = builder.f32(0.0)?; let nonnegative = builder.binary(ScalarOpcode::GreaterThanOrEqual, x, zero)?;
	let negative_x = builder.unary(ScalarOpcode::Negate, x)?; let exp_negative = emit_exp_core(builder, negative_x)?;
	let exp_positive = emit_exp_core(builder, x)?; let one = builder.f32(1.0)?;
	let positive_denominator = builder.binary(ScalarOpcode::Add, one, exp_negative)?;
	let positive = builder.binary(ScalarOpcode::Divide, one, positive_denominator)?;
	let negative_denominator = builder.binary(ScalarOpcode::Add, one, exp_positive)?;
	let negative = builder.binary(ScalarOpcode::Divide, exp_positive, negative_denominator)?;
	builder.ternary(ScalarOpcode::Select, nonnegative, positive, negative) }

fn emit_tanh_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let absolute = builder.unary(ScalarOpcode::Absolute, x)?; let two = builder.f32(2.0)?;
	let doubled = builder.binary(ScalarOpcode::Multiply, absolute, two)?;
	let numerator = emit_expm1_core(builder, doubled)?;
	let denominator = builder.binary(ScalarOpcode::Add, numerator, two)?;
	let magnitude = builder.binary(ScalarOpcode::Divide, numerator, denominator)?; copy_sign_from(builder, magnitude, x) }

fn emit_softplus_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let absolute = builder.unary(ScalarOpcode::Absolute, x)?;
	let negative_absolute = builder.unary(ScalarOpcode::Negate, absolute)?;
	let exponential = emit_exp_core(builder, negative_absolute)?; let correction = emit_log1p_core(builder, exponential)?;
	let zero = builder.f32(0.0)?; let positive = builder.binary(ScalarOpcode::GreaterThan, x, zero)?;
	let maximum = builder.ternary(ScalarOpcode::Select, positive, x, zero)?;
	builder.binary(ScalarOpcode::Add, maximum, correction) }

fn emit_erf_core(builder: &mut ScalarProgramBuilder, x: ScalarExpression) -> LanguageResult<ScalarExpression> {
	let absolute = builder.unary(ScalarOpcode::Absolute, x)?; let p = builder.f32(0.327_591_1)?;
	let scaled = builder.binary(ScalarOpcode::Multiply, p, absolute)?; let one = builder.f32(1.0)?;
	let denominator = builder.binary(ScalarOpcode::Add, one, scaled)?;
	let t = builder.binary(ScalarOpcode::Divide, one, denominator)?; let polynomial = horner(builder, t, 1.061_405_4, &[
		-1.453_152_1, 1.421_413_8, -0.284_496_72, 0.254_829_6, ])?;
	let polynomial = builder.binary(ScalarOpcode::Multiply, polynomial, t)?;
	let squared = builder.binary(ScalarOpcode::Multiply, absolute, absolute)?;
	let negative_squared = builder.unary(ScalarOpcode::Negate, squared)?;
	let exponential = emit_exp_core(builder, negative_squared)?;
	let tail = builder.binary(ScalarOpcode::Multiply, polynomial, exponential)?;
	let magnitude = builder.binary(ScalarOpcode::Subtract, one, tail)?; copy_sign_from(builder, magnitude, x) }

fn copy_sign_from( builder: &mut ScalarProgramBuilder, magnitude: ScalarExpression, sign_source: ScalarExpression,
) -> LanguageResult<ScalarExpression> { let bits = builder.unary(ScalarOpcode::BitcastF32ToI32, sign_source)?;
	let shift = builder.i32(31)?; let sign = builder.binary(ScalarOpcode::ShiftRightLogical, bits, shift)?;
	let negative = builder.unary(ScalarOpcode::Negate, magnitude)?;
	builder.ternary(ScalarOpcode::Select, sign, negative, magnitude) }
