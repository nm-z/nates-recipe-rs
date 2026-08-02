use std::collections::BTreeMap;

use recipe_core::{
	DType, ScalarConstant, ScalarInput, ScalarInstruction, ScalarLiteral, ScalarOpcode, ScalarProgram, ScalarValueId,
};
use recipe_math::MathFunction;

use crate::{
	CanonicalDTypeContract, LoweringAvailability, OperationDescriptor, OperationError, OperationErrorKind,
	OperationResult,
};

const F32_1: &[DType] = &[DType::F32];
const F32_2: &[DType] = &[DType::F32, DType::F32];
const F32_3: &[DType] = &[DType::F32, DType::F32, DType::F32];
const F32_4: &[DType] = &[DType::F32, DType::F32, DType::F32, DType::F32];
const I32_F32_F32: &[DType] = &[DType::I32, DType::F32, DType::F32];
const F32_OUTPUT: &[DType] = &[DType::F32];
const I32_OUTPUT: &[DType] = &[DType::I32];

/// Fixed class weight used by Recipe's parameterless focal objective.
pub const FOCAL_ALPHA: f32 = 0.25;
/// Fixed focusing exponent used by Recipe's parameterless focal objective.
pub const FOCAL_GAMMA: f32 = 2.0;

/// A canonical scalar recipe. Tensor broadcasting and view selection are
/// described by the surrounding elementwise primitive, while this value fixes
/// the per-output-element calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarRecipe {
	Opcode {
		opcode: ScalarOpcode,
		inputs: &'static [DType],
	},
	Math(MathFunction),
	Composite(CompositeScalar),
}

/// Multi-instruction elementwise calculations owned by Recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositeScalar {
	Identity,
	ReverseAdd,
	ReverseMultiply,
	DifferenceScaled,
	ReverseSubtract,
	ReverseDivide,
	Clamp,
	ClampLegacyOrder,
	BinaryCrossEntropyGradient,
	Dropout,
	Relu,
	ReluBackward,
	LeakyRelu,
	LeakyReluBackward,
	Elu,
	EluBackward,
	Selu,
	SeluBackward,
	SigmoidBackward,
	TanhBackward,
	Silu,
	SiluBackward,
	GeluTanh,
	GeluTanhBackward,
	GeluTanhMultiply,
	GluGeluTanh,
	GluSilu,
	ScaledExp,
	Reparameterize,
	KullbackLeiblerElement,
	MeanAbsoluteErrorGradient,
	HuberGradient,
	SgdAdd,
	SgdSubtract,
}

impl ScalarRecipe {
	pub(crate) fn for_symbol(symbol: &str) -> Option<Self> {
		let recipe = match symbol {
			"gpu_abs_into" => Self::opcode(ScalarOpcode::Absolute, F32_1),
			"gpu_add_into" | "gpu_add_scalar" | "gpu_bias_add" | "gpu_bias_add_f32" | "gpu_add_f16" => {
				Self::opcode(ScalarOpcode::Add, F32_2)
			}
			"gpu_add_inplace" | "gpu_add_scalar_inplace" => Self::Composite(CompositeScalar::ReverseAdd),
			"gpu_atan2" => Self::Math(MathFunction::Atan2),
			"gpu_bce_grad_into" => Self::Composite(CompositeScalar::BinaryCrossEntropyGradient),
			"gpu_broadcast_div" | "gpu_div_into" | "gpu_div_scalar" => Self::opcode(ScalarOpcode::Divide, F32_2),
			"gpu_broadcast_mul" | "gpu_mul" | "gpu_mul_f16" | "gpu_row_scale" => {
				Self::opcode(ScalarOpcode::Multiply, F32_2)
			}
			"gpu_mul_inplace" | "gpu_scale_inplace" | "gpu_scale_f64_inplace" => {
				Self::Composite(CompositeScalar::ReverseMultiply)
			}
			"gpu_broadcast_sub_into" | "gpu_sub" | "gpu_sub_scalar" => {
				Self::opcode(ScalarOpcode::Subtract, F32_2)
			}
			"gpu_sub_inplace" => Self::Composite(CompositeScalar::ReverseSubtract),
			"gpu_ceil" => Self::Math(MathFunction::Ceil),
			"gpu_clamp_into" => Self::Composite(CompositeScalar::Clamp),
			"gpu_clip_value" => Self::Composite(CompositeScalar::ClampLegacyOrder),
			"gpu_copy_into" | "gpu_fill" | "gpu_fill_f32" => Self::Composite(CompositeScalar::Identity),
			"gpu_cos" => Self::Math(MathFunction::Cos),
			"gpu_dropout_into" => Self::Composite(CompositeScalar::Dropout),
			"gpu_elu" => Self::Composite(CompositeScalar::Elu),
			"gpu_elu_backward" => Self::Composite(CompositeScalar::EluBackward),
			"gpu_eq" => Self::opcode(ScalarOpcode::Equal, F32_2),
			"gpu_exp" => Self::Math(MathFunction::Exp),
			"gpu_expm1" => Self::Math(MathFunction::ExpMinusOne),
			"gpu_floor" => Self::Math(MathFunction::Floor),
			"gpu_fma" => Self::opcode(ScalarOpcode::Fma, F32_3),
			"gpu_fmod" => Self::Math(MathFunction::Fmod),
			"gpu_gelu_backward_f32" | "gpu_gelu_backward_into" => {
				Self::Composite(CompositeScalar::GeluTanhBackward)
			}
			"gpu_gelu_f16" | "gpu_gelu_f32" | "gpu_gelu_into" => Self::Composite(CompositeScalar::GeluTanh),
			"gpu_gelu_mul" => Self::Composite(CompositeScalar::GeluTanhMultiply),
			"gpu_glu_gelu" => Self::Composite(CompositeScalar::GluGeluTanh),
			"gpu_glu_silu" => Self::Composite(CompositeScalar::GluSilu),
			"gpu_gt" | "gpu_gt_scalar" => Self::opcode(ScalarOpcode::GreaterThan, F32_2),
			"gpu_huber_grad" => Self::Composite(CompositeScalar::HuberGradient),
			"gpu_kl_div" => Self::Composite(CompositeScalar::KullbackLeiblerElement),
			"gpu_leaky_relu_backward_into" => Self::Composite(CompositeScalar::LeakyReluBackward),
			"gpu_leaky_relu_into" => Self::Composite(CompositeScalar::LeakyRelu),
			"gpu_log_into" => Self::Math(MathFunction::Log),
			"gpu_log1p" => Self::Math(MathFunction::LogOnePlus),
			"gpu_lt" | "gpu_lt_scalar" => Self::opcode(ScalarOpcode::LessThan, F32_2),
			"gpu_mae_grad" => Self::Composite(CompositeScalar::MeanAbsoluteErrorGradient),
			"gpu_max" => Self::opcode(ScalarOpcode::Maximum, F32_2),
			"gpu_min" => Self::opcode(ScalarOpcode::Minimum, F32_2),
			"gpu_mse_grad_into" => Self::opcode(ScalarOpcode::Subtract, F32_2),
			"gpu_neg" => Self::opcode(ScalarOpcode::Negate, F32_1),
			"gpu_pow" => Self::Math(MathFunction::Pow),
			"gpu_rdiv_scalar" => Self::Composite(CompositeScalar::ReverseDivide),
			"gpu_reciprocal" => Self::Math(MathFunction::Reciprocal),
			"gpu_relu_backward_f32" | "gpu_relu_backward_into" => Self::Composite(CompositeScalar::ReluBackward),
			"gpu_relu_f16" | "gpu_relu_f32" | "gpu_relu_into" => Self::Composite(CompositeScalar::Relu),
			"gpu_reparameterize" => Self::Composite(CompositeScalar::Reparameterize),
			"gpu_round" => Self::Math(MathFunction::RoundNearestEven),
			"gpu_rsqrt" => Self::Math(MathFunction::ReciprocalSquareRoot),
			"gpu_rsub_scalar" => Self::Composite(CompositeScalar::ReverseSubtract),
			"gpu_scaled_exp" => Self::Composite(CompositeScalar::ScaledExp),
			"gpu_sgd_update" => Self::Composite(CompositeScalar::SgdAdd),
			"gpu_sgd_update_f32" => Self::Composite(CompositeScalar::SgdSubtract),
			"gpu_selu" => Self::Composite(CompositeScalar::Selu),
			"gpu_selu_backward" => Self::Composite(CompositeScalar::SeluBackward),
			"gpu_sigmoid_backward_into" => Self::Composite(CompositeScalar::SigmoidBackward),
			"gpu_sigmoid_into" => Self::Math(MathFunction::Sigmoid),
			"gpu_sign_into" => Self::Math(MathFunction::Sign),
			"gpu_silu_backward_into" => Self::Composite(CompositeScalar::SiluBackward),
			"gpu_silu_into" => Self::Composite(CompositeScalar::Silu),
			"gpu_sin" => Self::Math(MathFunction::Sin),
			"gpu_softplus" => Self::Math(MathFunction::Softplus),
			"gpu_sqrt" => Self::opcode(ScalarOpcode::SquareRoot, F32_1),
			"gpu_sub_scale_into" => Self::Composite(CompositeScalar::DifferenceScaled),
			"gpu_tan" => Self::Math(MathFunction::Tan),
			"gpu_tanh_backward_into" => Self::Composite(CompositeScalar::TanhBackward),
			"gpu_tanh_into" => Self::Math(MathFunction::Tanh),
			"gpu_trunc" => Self::Math(MathFunction::Trunc),
			"gpu_where_mask" => Self::opcode(ScalarOpcode::Select, I32_F32_F32),
			_ => return None,
		};
		Some(recipe)
	}

	const fn opcode(opcode: ScalarOpcode, inputs: &'static [DType]) -> Self { Self::Opcode { opcode, inputs } }

	#[must_use]
	pub const fn dtype_contract(self) -> CanonicalDTypeContract {
		match self {
			Self::Opcode { opcode, inputs } => {
				CanonicalDTypeContract::Exact {
					inputs,
					outputs: if matches!(
						opcode,
						ScalarOpcode::Equal
							| ScalarOpcode::NotEqual | ScalarOpcode::LessThan
							| ScalarOpcode::LessThanOrEqual | ScalarOpcode::GreaterThan
							| ScalarOpcode::GreaterThanOrEqual
					) {
						I32_OUTPUT
					} else {
						F32_OUTPUT
					},
				}
			}
			Self::Math(function) => {
				CanonicalDTypeContract::Exact {
					inputs: if function.arity() == 1 { F32_1 } else { F32_2 },
					outputs: F32_OUTPUT,
				}
			}
			Self::Composite(composite) => {
				CanonicalDTypeContract::Exact {
					inputs: composite.inputs(),
					outputs: F32_OUTPUT,
				}
			}
		}
	}

	#[must_use]
	pub const fn definition(self) -> &'static str {
		match self {
			Self::Opcode {
				opcode: ScalarOpcode::Add,
				..
			} => "per-element typed addition",
			Self::Opcode {
				opcode: ScalarOpcode::Subtract,
				..
			} => "per-element typed subtraction",
			Self::Opcode {
				opcode: ScalarOpcode::Multiply,
				..
			} => "per-element typed multiplication",
			Self::Opcode {
				opcode: ScalarOpcode::Divide,
				..
			} => "per-element IEEE binary32 division",
			Self::Opcode {
				opcode: ScalarOpcode::Absolute,
				..
			} => "per-element binary32 absolute value",
			Self::Opcode {
				opcode: ScalarOpcode::Negate,
				..
			} => "per-element binary32 negation",
			Self::Opcode {
				opcode: ScalarOpcode::Minimum,
				..
			} => "per-element binary32 minimum",
			Self::Opcode {
				opcode: ScalarOpcode::Maximum,
				..
			} => "per-element binary32 maximum",
			Self::Opcode {
				opcode: ScalarOpcode::Fma,
				..
			} => "per-element one-rounding binary32 fused multiply-add",
			Self::Opcode {
				opcode: ScalarOpcode::Equal,
				..
			} => "per-element ordered equality yielding an int32 mask",
			Self::Opcode {
				opcode: ScalarOpcode::LessThan,
				..
			} => "per-element ordered less-than yielding an int32 mask",
			Self::Opcode {
				opcode: ScalarOpcode::GreaterThan,
				..
			} => "per-element ordered greater-than yielding an int32 mask",
			Self::Opcode {
				opcode: ScalarOpcode::Select,
				..
			} => "per-element int32-mask selection",
			Self::Opcode {
				opcode: ScalarOpcode::SquareRoot,
				..
			} => "per-element binary32 square root",
			Self::Opcode { .. } => "typed scalar opcode",
			Self::Math(function) => function.algorithm().name,
			Self::Composite(composite) => composite.definition(),
		}
	}
}

impl CompositeScalar {
	const fn inputs(self) -> &'static [DType] {
		match self {
			Self::Identity | Self::Relu | Self::Silu | Self::GeluTanh => F32_1,
			Self::ReverseAdd
			| Self::ReverseMultiply
			| Self::ReverseSubtract
			| Self::ReverseDivide
			| Self::ReluBackward
			| Self::LeakyRelu
			| Self::Elu
			| Self::SigmoidBackward
			| Self::TanhBackward
			| Self::SiluBackward
			| Self::GeluTanhBackward
			| Self::GeluTanhMultiply
			| Self::GluGeluTanh
			| Self::GluSilu
			| Self::ScaledExp
			| Self::KullbackLeiblerElement
			| Self::MeanAbsoluteErrorGradient => F32_2,
			Self::DifferenceScaled
			| Self::Clamp
			| Self::ClampLegacyOrder
			| Self::BinaryCrossEntropyGradient
			| Self::LeakyReluBackward
			| Self::EluBackward
			| Self::Selu
			| Self::Reparameterize
			| Self::HuberGradient
			| Self::SgdAdd
			| Self::SgdSubtract => F32_3,
			Self::Dropout | Self::SeluBackward => F32_4,
		}
	}

	const fn definition(self) -> &'static str {
		match self {
			Self::Identity => "per-element identity",
			Self::ReverseAdd => "second input + first input",
			Self::ReverseMultiply => "second input * first input",
			Self::DifferenceScaled => "(left - right) * scale",
			Self::ReverseSubtract => "scalar - element",
			Self::ReverseDivide => "scalar / element",
			Self::Clamp => "min(max(value, lower), upper)",
			Self::ClampLegacyOrder => "min(max(third input value, first input lower), second input upper)",
			Self::BinaryCrossEntropyGradient => {
				"((clamp(prediction) - target) / (prediction * (1 - prediction))) * inverse_count"
			}
			Self::Dropout => "mask < probability selects zero, otherwise value * scale",
			Self::Relu => "max(value, 0)",
			Self::ReluBackward => "gradient when activation > 0, otherwise 0",
			Self::LeakyRelu => "value when value > 0, otherwise alpha * value",
			Self::LeakyReluBackward => "gradient when activation > 0, otherwise alpha * gradient",
			Self::Elu => "value when value > 0, otherwise alpha * (exp(value) - 1)",
			Self::EluBackward => "gradient * (1 when value > 0, otherwise alpha * exp(value))",
			Self::Selu => "lambda * ELU(value, alpha)",
			Self::SeluBackward => "gradient * lambda * (1 when value > 0, otherwise alpha * exp(value))",
			Self::SigmoidBackward => "gradient * activation * (1 - activation)",
			Self::TanhBackward => "gradient * (1 - activation * activation)",
			Self::Silu => "value * sigmoid(value)",
			Self::SiluBackward => "gradient * sigmoid(value) * (1 + value * (1 - sigmoid(value)))",
			Self::GeluTanh => "0.5 * value * (1 + tanh(sqrt(2/pi) * (value + 0.044715 * value^3)))",
			Self::GeluTanhBackward => "gradient times the analytic derivative of tanh-approximated GELU",
			Self::GeluTanhMultiply => "tanh-approximated GELU(left) * right",
			Self::GluGeluTanh => "tanh-approximated GELU(first split value) * second split value",
			Self::GluSilu => "SiLU(first split value) * second split value",
			Self::ScaledExp => "exp(value * scale)",
			Self::Reparameterize => "mean + exp(0.5 * log_variance) * epsilon",
			Self::KullbackLeiblerElement => "0.5 * (mean^2 + exp(log_variance) - log_variance - 1)",
			Self::MeanAbsoluteErrorGradient => "sign(prediction - target)",
			Self::HuberGradient => "clamp(prediction - target, -delta, delta)",
			Self::SgdAdd => "weights + signed_learning_rate * gradient",
			Self::SgdSubtract => "weights - learning_rate * gradient",
		}
	}
}

/// Lower a scalar descriptor into an owned, validated SSA program.
pub fn lower_scalar(descriptor: OperationDescriptor) -> OperationResult<ScalarProgram> {
	let recipe = match descriptor.lowering {
		LoweringAvailability::Scalar(recipe) => recipe,
		LoweringAvailability::Primitive(_)
		| LoweringAvailability::Composition(_)
		| LoweringAvailability::Workspace(_)
		| LoweringAvailability::NonCalculation(_) => {
			return Err(wrong_kind(
				descriptor,
				"operation does not have a scalar elementwise recipe",
			));
		}
		LoweringAvailability::Unsupported(reason) => {
			return Err(OperationError::new(
				OperationErrorKind::UnsupportedLowering,
				format!("{}: {reason:?}", descriptor.definition),
			)
			.for_operation(descriptor.id));
		}
	};
	let program = match recipe {
		ScalarRecipe::Opcode { opcode, inputs } => {
			let mut composer = Composer::new();
			let inputs = composer.inputs(inputs)?;
			let output = composer.apply(opcode, &inputs)?;
			composer.finish(&[output])?
		}
		ScalarRecipe::Math(function) => {
			ScalarProgram::try_from(function).map_err(|error| invalid_program(descriptor, error.to_string()))?
		}
		ScalarRecipe::Composite(composite) => {
			lower_composite(composite).map_err(|error| error.for_operation(descriptor.id))?
		}
	};
	program
		.validate()
		.map_err(|errors| invalid_program(descriptor, errors.to_string()))?;
	Ok(program)
}

/// Canonical parameterless leaky-ReLU forward program used by Recipe models.
pub fn canonical_leaky_relu_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let value = composer.input(DType::F32)?;
	let alpha = composer.f32(0.01)?;
	let output = leaky_relu(&mut composer, value, alpha)?;
	composer.finish(&[output])
}

/// Canonical parameterless leaky-ReLU backward program used by Recipe models.
pub fn canonical_leaky_relu_backward_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let gradient = composer.input(DType::F32)?;
	let value = composer.input(DType::F32)?;
	let alpha = composer.f32(0.01)?;
	let output = leaky_relu_backward(&mut composer, gradient, value, alpha)?;
	composer.finish(&[output])
}

/// Canonical PReLU forward program. The second input is one learned scalar
/// broadcast over the activation tensor by the enclosing elementwise node.
pub fn canonical_prelu_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let value = composer.input(DType::F32)?;
	let alpha = composer.input(DType::F32)?;
	let output = leaky_relu(&mut composer, value, alpha)?;
	composer.finish(&[output])
}

/// Canonical PReLU backward program.
///
/// The first output is the input gradient. The second is the per-element
/// contribution to the learned scalar gradient; the model compiler reduces it
/// over the complete logical training partition before optimizer clipping.
pub fn canonical_prelu_backward_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let gradient = composer.input(DType::F32)?;
	let value = composer.input(DType::F32)?;
	let alpha = composer.input(DType::F32)?;
	let input_gradient = leaky_relu_backward(&mut composer, gradient, value, alpha)?;
	let zero = composer.f32(0.0)?;
	let positive = composer.binary(ScalarOpcode::GreaterThan, value, zero)?;
	let negative_value = composer.ternary(ScalarOpcode::Select, positive, zero, value)?;
	let alpha_gradient = composer.binary(ScalarOpcode::Multiply, gradient, negative_value)?;
	composer.finish(&[input_gradient, alpha_gradient])
}

/// Canonical parameterless ELU forward program with alpha equal to one.
pub fn canonical_elu_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let value = composer.input(DType::F32)?;
	let alpha = composer.f32(1.0)?;
	let output = elu(&mut composer, value, alpha)?;
	composer.finish(&[output])
}

/// Canonical parameterless ELU backward program with alpha equal to one.
pub fn canonical_elu_backward_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let gradient = composer.input(DType::F32)?;
	let value = composer.input(DType::F32)?;
	let alpha = composer.f32(1.0)?;
	let output = elu_backward(&mut composer, gradient, value, alpha)?;
	composer.finish(&[output])
}

/// Canonical self-normalizing SELU forward program.
pub fn canonical_selu_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let value = composer.input(DType::F32)?;
	let alpha = composer.f32(1.673_263_2)?;
	let lambda = composer.f32(1.050_701)?;
	let activated = elu(&mut composer, value, alpha)?;
	let output = composer.binary(ScalarOpcode::Multiply, lambda, activated)?;
	composer.finish(&[output])
}

/// Canonical self-normalizing SELU backward program.
pub fn canonical_selu_backward_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let gradient = composer.input(DType::F32)?;
	let value = composer.input(DType::F32)?;
	let alpha = composer.f32(1.673_263_2)?;
	let lambda = composer.f32(1.050_701)?;
	let derivative = elu_derivative(&mut composer, value, alpha)?;
	let scaled = composer.binary(ScalarOpcode::Multiply, lambda, derivative)?;
	let output = composer.binary(ScalarOpcode::Multiply, gradient, scaled)?;
	composer.finish(&[output])
}

pub(crate) fn binary_cross_entropy_with_logits_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let logit = composer.input(DType::F32)?;
	let target = composer.input(DType::F32)?;

	let target_is_finite = composer.unary(ScalarOpcode::IsFinite, target)?;
	composer.unary(ScalarOpcode::Require, target_is_finite)?;
	let zero = composer.f32(0.0)?;
	let one = composer.f32(1.0)?;
	let target_is_nonnegative = composer.binary(ScalarOpcode::GreaterThanOrEqual, target, zero)?;
	composer.unary(ScalarOpcode::Require, target_is_nonnegative)?;
	let target_is_at_most_one = composer.binary(ScalarOpcode::LessThanOrEqual, target, one)?;
	composer.unary(ScalarOpcode::Require, target_is_at_most_one)?;

	// softplus(logit) is Recipe's owned stable
	// max(logit, 0) + log1p(exp(-abs(logit))) implementation. Both math
	// programs are inlined into this program so every scalar value remains in
	// one owned SSA identity space.
	let softplus = composer.inline_math(MathFunction::Softplus, &[logit])?;
	let weighted_logit = composer.binary(ScalarOpcode::Multiply, logit, target)?;
	let loss = composer.binary(ScalarOpcode::Subtract, softplus, weighted_logit)?;
	let sigmoid = composer.inline_math(MathFunction::Sigmoid, &[logit])?;
	let gradient = composer.binary(ScalarOpcode::Subtract, sigmoid, target)?;
	composer.finish(&[loss, gradient])
}

/// Canonical binary focal loss evaluated directly from logits.
///
/// Recipe's public focal objective is parameterless, so it retains the
/// historical alpha of 0.25 and gamma of 2.0. Targets must be exact zero or
/// one. The program uses `sigmoid(-signed_logit)` and
/// `softplus(-signed_logit)` so it never forms `log(sigmoid(logit))` by
/// subtracting a rounded probability from one.
pub fn canonical_focal_with_logits_program() -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let logit = composer.input(DType::F32)?;
	let target = composer.input(DType::F32)?;

	let target_is_finite = composer.unary(ScalarOpcode::IsFinite, target)?;
	composer.unary(ScalarOpcode::Require, target_is_finite)?;
	let zero = composer.f32(0.0)?;
	let one = composer.f32(1.0)?;
	let target_is_zero = composer.binary(ScalarOpcode::Equal, target, zero)?;
	let target_is_one = composer.binary(ScalarOpcode::Equal, target, one)?;
	let target_is_binary = composer.binary(ScalarOpcode::BitOr, target_is_zero, target_is_one)?;
	composer.unary(ScalarOpcode::Require, target_is_binary)?;

	let negative_logit = composer.unary(ScalarOpcode::Negate, logit)?;
	let signed_logit = composer.ternary(ScalarOpcode::Select, target_is_one, logit, negative_logit)?;
	let negative_signed_logit = composer.unary(ScalarOpcode::Negate, signed_logit)?;
	let target_probability = composer.inline_math(MathFunction::Sigmoid, &[signed_logit])?;
	let focusing_weight = composer.inline_math(MathFunction::Sigmoid, &[negative_signed_logit])?;
	let negative_log_probability = composer.inline_math(MathFunction::Softplus, &[negative_signed_logit])?;

	let focusing_weight_squared = composer.binary(ScalarOpcode::Multiply, focusing_weight, focusing_weight)?;
	let alpha = composer.f32(FOCAL_ALPHA)?;
	let weighted_focus = composer.binary(ScalarOpcode::Multiply, alpha, focusing_weight_squared)?;
	let loss = composer.binary(
		ScalarOpcode::Multiply,
		weighted_focus,
		negative_log_probability,
	)?;

	let probability_log = composer.binary(
		ScalarOpcode::Multiply,
		target_probability,
		negative_log_probability,
	)?;
	let gamma = composer.f32(FOCAL_GAMMA)?;
	let focused_probability_log = composer.binary(ScalarOpcode::Multiply, gamma, probability_log)?;
	let gradient_factor = composer.binary(ScalarOpcode::Add, focused_probability_log, focusing_weight)?;
	let gradient_magnitude = composer.binary(ScalarOpcode::Multiply, weighted_focus, gradient_factor)?;
	let negative_gradient = composer.unary(ScalarOpcode::Negate, gradient_magnitude)?;
	let gradient = composer.ternary(
		ScalarOpcode::Select,
		target_is_one,
		negative_gradient,
		gradient_magnitude,
	)?;
	composer.finish(&[loss, gradient])
}

fn lower_composite(composite: CompositeScalar) -> OperationResult<ScalarProgram> {
	let mut composer = Composer::new();
	let values = composer.inputs(composite.inputs())?;
	let output = match composite {
		CompositeScalar::Identity => values[0],
		CompositeScalar::ReverseAdd => composer.binary(ScalarOpcode::Add, values[1], values[0])?,
		CompositeScalar::ReverseMultiply => composer.binary(ScalarOpcode::Multiply, values[1], values[0])?,
		CompositeScalar::DifferenceScaled => {
			let difference = composer.binary(ScalarOpcode::Subtract, values[0], values[1])?;
			composer.binary(ScalarOpcode::Multiply, difference, values[2])?
		}
		CompositeScalar::ReverseSubtract => composer.binary(ScalarOpcode::Subtract, values[1], values[0])?,
		CompositeScalar::ReverseDivide => composer.binary(ScalarOpcode::Divide, values[1], values[0])?,
		CompositeScalar::Clamp => {
			let lower = composer.binary(ScalarOpcode::Maximum, values[0], values[1])?;
			composer.binary(ScalarOpcode::Minimum, lower, values[2])?
		}
		CompositeScalar::ClampLegacyOrder => {
			let lower = composer.binary(ScalarOpcode::Maximum, values[2], values[0])?;
			composer.binary(ScalarOpcode::Minimum, lower, values[1])?
		}
		CompositeScalar::BinaryCrossEntropyGradient => {
			binary_cross_entropy_gradient(&mut composer, values[0], values[1], values[2])?
		}
		CompositeScalar::Dropout => {
			let drop = composer.binary(ScalarOpcode::LessThan, values[1], values[2])?;
			let zero = composer.f32(0.0)?;
			let scaled = composer.binary(ScalarOpcode::Multiply, values[0], values[3])?;
			composer.ternary(ScalarOpcode::Select, drop, zero, scaled)?
		}
		CompositeScalar::Relu => relu(&mut composer, values[0])?,
		CompositeScalar::ReluBackward => positive_gradient(&mut composer, values[0], values[1])?,
		CompositeScalar::LeakyRelu => leaky_relu(&mut composer, values[0], values[1])?,
		CompositeScalar::LeakyReluBackward => leaky_relu_backward(&mut composer, values[0], values[1], values[2])?,
		CompositeScalar::Elu => elu(&mut composer, values[0], values[1])?,
		CompositeScalar::EluBackward => elu_backward(&mut composer, values[0], values[1], values[2])?,
		CompositeScalar::Selu => {
			let activated = elu(&mut composer, values[0], values[1])?;
			composer.binary(ScalarOpcode::Multiply, values[2], activated)?
		}
		CompositeScalar::SeluBackward => {
			let derivative = elu_derivative(&mut composer, values[1], values[2])?;
			let scaled = composer.binary(ScalarOpcode::Multiply, values[3], derivative)?;
			composer.binary(ScalarOpcode::Multiply, values[0], scaled)?
		}
		CompositeScalar::SigmoidBackward => {
			let one = composer.f32(1.0)?;
			let complement = composer.binary(ScalarOpcode::Subtract, one, values[1])?;
			let slope = composer.binary(ScalarOpcode::Multiply, values[1], complement)?;
			composer.binary(ScalarOpcode::Multiply, values[0], slope)?
		}
		CompositeScalar::TanhBackward => {
			let square = composer.binary(ScalarOpcode::Multiply, values[1], values[1])?;
			let one = composer.f32(1.0)?;
			let slope = composer.binary(ScalarOpcode::Subtract, one, square)?;
			composer.binary(ScalarOpcode::Multiply, values[0], slope)?
		}
		CompositeScalar::Silu => silu(&mut composer, values[0])?,
		CompositeScalar::SiluBackward => silu_backward(&mut composer, values[0], values[1])?,
		CompositeScalar::GeluTanh => gelu_tanh(&mut composer, values[0])?,
		CompositeScalar::GeluTanhBackward => gelu_tanh_backward(&mut composer, values[0], values[1])?,
		CompositeScalar::GeluTanhMultiply | CompositeScalar::GluGeluTanh => {
			let activated = gelu_tanh(&mut composer, values[0])?;
			composer.binary(ScalarOpcode::Multiply, activated, values[1])?
		}
		CompositeScalar::GluSilu => {
			let activated = silu(&mut composer, values[0])?;
			composer.binary(ScalarOpcode::Multiply, activated, values[1])?
		}
		CompositeScalar::ScaledExp => {
			let scaled = composer.binary(ScalarOpcode::Multiply, values[0], values[1])?;
			composer.inline_math(MathFunction::Exp, &[scaled])?
		}
		CompositeScalar::Reparameterize => {
			let half = composer.f32(0.5)?;
			let half_variance = composer.binary(ScalarOpcode::Multiply, half, values[1])?;
			let deviation = composer.inline_math(MathFunction::Exp, &[half_variance])?;
			let noise = composer.binary(ScalarOpcode::Multiply, deviation, values[2])?;
			composer.binary(ScalarOpcode::Add, values[0], noise)?
		}
		CompositeScalar::KullbackLeiblerElement => kl_divergence(&mut composer, values[0], values[1])?,
		CompositeScalar::MeanAbsoluteErrorGradient => {
			let difference = composer.binary(ScalarOpcode::Subtract, values[0], values[1])?;
			composer.inline_math(MathFunction::Sign, &[difference])?
		}
		CompositeScalar::HuberGradient => {
			let difference = composer.binary(ScalarOpcode::Subtract, values[0], values[1])?;
			let negative_delta = composer.unary(ScalarOpcode::Negate, values[2])?;
			let lower = composer.binary(ScalarOpcode::Maximum, difference, negative_delta)?;
			composer.binary(ScalarOpcode::Minimum, lower, values[2])?
		}
		CompositeScalar::SgdAdd => {
			let update = composer.binary(ScalarOpcode::Multiply, values[1], values[0])?;
			composer.binary(ScalarOpcode::Add, values[2], update)?
		}
		CompositeScalar::SgdSubtract => {
			let update = composer.binary(ScalarOpcode::Multiply, values[1], values[0])?;
			composer.binary(ScalarOpcode::Subtract, values[2], update)?
		}
	};
	composer.finish(&[output])
}

fn binary_cross_entropy_gradient(
	composer: &mut Composer,
	prediction: Value,
	target: Value,
	inverse_count: Value,
) -> OperationResult<Value> {
	let epsilon = composer.f32(1.0e-7)?;
	let one = composer.f32(1.0)?;
	let upper = composer.binary(ScalarOpcode::Subtract, one, epsilon)?;
	let lower_bounded = composer.binary(ScalarOpcode::Maximum, prediction, epsilon)?;
	let bounded = composer.binary(ScalarOpcode::Minimum, lower_bounded, upper)?;
	let numerator = composer.binary(ScalarOpcode::Subtract, bounded, target)?;
	let complement = composer.binary(ScalarOpcode::Subtract, one, bounded)?;
	let denominator = composer.binary(ScalarOpcode::Multiply, bounded, complement)?;
	let quotient = composer.binary(ScalarOpcode::Divide, numerator, denominator)?;
	composer.binary(ScalarOpcode::Multiply, quotient, inverse_count)
}

fn relu(composer: &mut Composer, value: Value) -> OperationResult<Value> {
	let zero = composer.f32(0.0)?;
	composer.binary(ScalarOpcode::Maximum, value, zero)
}

fn positive_gradient(composer: &mut Composer, gradient: Value, activation: Value) -> OperationResult<Value> {
	let zero = composer.f32(0.0)?;
	let positive = composer.binary(ScalarOpcode::GreaterThan, activation, zero)?;
	composer.ternary(ScalarOpcode::Select, positive, gradient, zero)
}

fn leaky_relu(composer: &mut Composer, value: Value, alpha: Value) -> OperationResult<Value> {
	let zero = composer.f32(0.0)?;
	let positive = composer.binary(ScalarOpcode::GreaterThan, value, zero)?;
	let negative = composer.binary(ScalarOpcode::Multiply, alpha, value)?;
	composer.ternary(ScalarOpcode::Select, positive, value, negative)
}

fn leaky_relu_backward(
	composer: &mut Composer,
	gradient: Value,
	activation: Value,
	alpha: Value,
) -> OperationResult<Value> {
	let zero = composer.f32(0.0)?;
	let positive = composer.binary(ScalarOpcode::GreaterThan, activation, zero)?;
	let negative = composer.binary(ScalarOpcode::Multiply, alpha, gradient)?;
	composer.ternary(ScalarOpcode::Select, positive, gradient, negative)
}

fn elu(composer: &mut Composer, value: Value, alpha: Value) -> OperationResult<Value> {
	let zero = composer.f32(0.0)?;
	let positive = composer.binary(ScalarOpcode::GreaterThan, value, zero)?;
	let exponential = composer.inline_math(MathFunction::Exp, &[value])?;
	let one = composer.f32(1.0)?;
	let shifted = composer.binary(ScalarOpcode::Subtract, exponential, one)?;
	let negative = composer.binary(ScalarOpcode::Multiply, alpha, shifted)?;
	composer.ternary(ScalarOpcode::Select, positive, value, negative)
}

fn elu_derivative(composer: &mut Composer, value: Value, alpha: Value) -> OperationResult<Value> {
	let zero = composer.f32(0.0)?;
	let positive = composer.binary(ScalarOpcode::GreaterThan, value, zero)?;
	let one = composer.f32(1.0)?;
	let exponential = composer.inline_math(MathFunction::Exp, &[value])?;
	let negative = composer.binary(ScalarOpcode::Multiply, alpha, exponential)?;
	composer.ternary(ScalarOpcode::Select, positive, one, negative)
}

fn elu_backward(composer: &mut Composer, gradient: Value, value: Value, alpha: Value) -> OperationResult<Value> {
	let derivative = elu_derivative(composer, value, alpha)?;
	composer.binary(ScalarOpcode::Multiply, gradient, derivative)
}

fn silu(composer: &mut Composer, value: Value) -> OperationResult<Value> {
	let sigmoid = composer.inline_math(MathFunction::Sigmoid, &[value])?;
	composer.binary(ScalarOpcode::Multiply, value, sigmoid)
}

fn silu_backward(composer: &mut Composer, gradient: Value, value: Value) -> OperationResult<Value> {
	let sigmoid = composer.inline_math(MathFunction::Sigmoid, &[value])?;
	let one = composer.f32(1.0)?;
	let complement = composer.binary(ScalarOpcode::Subtract, one, sigmoid)?;
	let correction = composer.binary(ScalarOpcode::Multiply, value, complement)?;
	let slope = composer.binary(ScalarOpcode::Add, one, correction)?;
	let slope = composer.binary(ScalarOpcode::Multiply, sigmoid, slope)?;
	composer.binary(ScalarOpcode::Multiply, gradient, slope)
}

fn gelu_tanh(composer: &mut Composer, value: Value) -> OperationResult<Value> {
	let square = composer.binary(ScalarOpcode::Multiply, value, value)?;
	let cube = composer.binary(ScalarOpcode::Multiply, square, value)?;
	let cubic_coefficient = composer.f32(0.044_715)?;
	let cubic = composer.binary(ScalarOpcode::Multiply, cubic_coefficient, cube)?;
	let polynomial = composer.binary(ScalarOpcode::Add, value, cubic)?;
	let coefficient = composer.f32(0.797_884_6)?;
	let inner = composer.binary(ScalarOpcode::Multiply, coefficient, polynomial)?;
	let hyperbolic = composer.inline_math(MathFunction::Tanh, &[inner])?;
	let one = composer.f32(1.0)?;
	let shifted = composer.binary(ScalarOpcode::Add, one, hyperbolic)?;
	let half = composer.f32(0.5)?;
	let half_value = composer.binary(ScalarOpcode::Multiply, half, value)?;
	composer.binary(ScalarOpcode::Multiply, half_value, shifted)
}

fn gelu_tanh_backward(composer: &mut Composer, gradient: Value, value: Value) -> OperationResult<Value> {
	let square = composer.binary(ScalarOpcode::Multiply, value, value)?;
	let cube = composer.binary(ScalarOpcode::Multiply, square, value)?;
	let cubic_coefficient = composer.f32(0.044_715)?;
	let cubic = composer.binary(ScalarOpcode::Multiply, cubic_coefficient, cube)?;
	let polynomial = composer.binary(ScalarOpcode::Add, value, cubic)?;
	let coefficient = composer.f32(0.797_884_6)?;
	let inner = composer.binary(ScalarOpcode::Multiply, coefficient, polynomial)?;
	let hyperbolic = composer.inline_math(MathFunction::Tanh, &[inner])?;
	let one = composer.f32(1.0)?;
	let half = composer.f32(0.5)?;
	let shifted = composer.binary(ScalarOpcode::Add, one, hyperbolic)?;
	let first = composer.binary(ScalarOpcode::Multiply, half, shifted)?;

	let hyperbolic_square = composer.binary(ScalarOpcode::Multiply, hyperbolic, hyperbolic)?;
	let tanh_slope = composer.binary(ScalarOpcode::Subtract, one, hyperbolic_square)?;
	let three_cubic = composer.f32(3.0 * 0.044_715)?;
	let quadratic = composer.binary(ScalarOpcode::Multiply, three_cubic, square)?;
	let inner_derivative = composer.binary(ScalarOpcode::Add, one, quadratic)?;
	let inner_derivative = composer.binary(ScalarOpcode::Multiply, coefficient, inner_derivative)?;
	let second = composer.binary(ScalarOpcode::Multiply, value, tanh_slope)?;
	let second = composer.binary(ScalarOpcode::Multiply, second, inner_derivative)?;
	let second = composer.binary(ScalarOpcode::Multiply, half, second)?;
	let derivative = composer.binary(ScalarOpcode::Add, first, second)?;
	composer.binary(ScalarOpcode::Multiply, gradient, derivative)
}

fn kl_divergence(composer: &mut Composer, mean: Value, log_variance: Value) -> OperationResult<Value> {
	let mean_square = composer.binary(ScalarOpcode::Multiply, mean, mean)?;
	let variance = composer.inline_math(MathFunction::Exp, &[log_variance])?;
	let sum = composer.binary(ScalarOpcode::Add, mean_square, variance)?;
	let difference = composer.binary(ScalarOpcode::Subtract, sum, log_variance)?;
	let one = composer.f32(1.0)?;
	let shifted = composer.binary(ScalarOpcode::Subtract, difference, one)?;
	let half = composer.f32(0.5)?;
	composer.binary(ScalarOpcode::Multiply, half, shifted)
}

fn wrong_kind(descriptor: OperationDescriptor, detail: &'static str) -> OperationError {
	OperationError::new(OperationErrorKind::WrongLoweringKind, detail).for_operation(descriptor.id)
}

fn invalid_program(descriptor: OperationDescriptor, detail: String) -> OperationError {
	OperationError::new(OperationErrorKind::InvalidScalarProgram, detail).for_operation(descriptor.id)
}

#[derive(Clone, Copy, Debug)]
struct Value {
	id: ScalarValueId,
	dtype: DType,
}

#[derive(Debug)]
struct Composer {
	next_value: u64,
	inputs: Vec<ScalarInput>,
	constants: Vec<ScalarConstant>,
	instructions: Vec<ScalarInstruction>,
}

impl Composer {
	const fn new() -> Self {
		Self {
			next_value: 1,
			inputs: Vec::new(),
			constants: Vec::new(),
			instructions: Vec::new(),
		}
	}

	fn inputs(&mut self, dtypes: &[DType]) -> OperationResult<Vec<Value>> {
		dtypes.iter()
			.copied()
			.map(|dtype| self.input(dtype))
			.collect()
	}

	fn input(&mut self, dtype: DType) -> OperationResult<Value> {
		let value = self.next(dtype)?;
		self.inputs.push(ScalarInput {
			id: value.id,
			dtype,
		});
		Ok(value)
	}

	fn f32(&mut self, value: f32) -> OperationResult<Value> { self.constant(ScalarLiteral::F32Bits(value.to_bits())) }

	fn constant(&mut self, literal: ScalarLiteral) -> OperationResult<Value> {
		let value = self.next(literal.dtype())?;
		self.constants.push(ScalarConstant {
			id: value.id,
			value: literal,
		});
		Ok(value)
	}

	fn unary(&mut self, opcode: ScalarOpcode, operand: Value) -> OperationResult<Value> {
		self.apply(opcode, &[operand])
	}

	fn binary(&mut self, opcode: ScalarOpcode, left: Value, right: Value) -> OperationResult<Value> {
		self.apply(opcode, &[left, right])
	}

	fn ternary(&mut self, opcode: ScalarOpcode, first: Value, second: Value, third: Value) -> OperationResult<Value> {
		self.apply(opcode, &[first, second, third])
	}

	fn apply(&mut self, opcode: ScalarOpcode, operands: &[Value]) -> OperationResult<Value> {
		let dtypes = operands
			.iter()
			.map(|operand| operand.dtype)
			.collect::<Vec<_>>();
		let dtype = opcode.result_dtype(&dtypes).ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::InvalidScalarProgram,
				format!("{opcode:?} does not accept {dtypes:?}"),
			)
		})?;
		let result = self.next(dtype)?;
		self.instructions.push(ScalarInstruction {
			result: result.id,
			dtype,
			opcode,
			operands: operands.iter().map(|operand| operand.id).collect(),
		});
		Ok(result)
	}

	fn inline_math(&mut self, function: MathFunction, arguments: &[Value]) -> OperationResult<Value> {
		let program = ScalarProgram::try_from(function)
			.map_err(|error| OperationError::new(OperationErrorKind::InvalidScalarProgram, error.to_string()))?;
		if program.inputs.len() != arguments.len() {
			return Err(OperationError::new(
				OperationErrorKind::InvalidScalarProgram,
				format!(
					"{function:?} has {} inputs, but {} arguments were supplied",
					program.inputs.len(),
					arguments.len()
				),
			));
		}
		let mut values = BTreeMap::new();
		for (input, argument) in program.inputs.iter().zip(arguments) {
			if input.dtype != argument.dtype {
				return Err(OperationError::new(
					OperationErrorKind::InvalidScalarProgram,
					format!(
						"{function:?} input requires {:?}, received {:?}",
						input.dtype, argument.dtype
					),
				));
			}
			values.insert(input.id, *argument);
		}
		for constant in program.constants {
			let replacement = self.constant(constant.value)?;
			values.insert(constant.id, replacement);
		}
		for instruction in program.instructions {
			let operands = instruction
				.operands
				.iter()
				.map(|operand| {
					values.get(operand).copied().ok_or_else(|| {
						OperationError::new(
							OperationErrorKind::InvalidScalarProgram,
							format!("{function:?} references unknown scalar value {operand}"),
						)
					})
				})
				.collect::<OperationResult<Vec<_>>>()?;
			let replacement = self.apply(instruction.opcode, &operands)?;
			if replacement.dtype != instruction.dtype {
				return Err(OperationError::new(
					OperationErrorKind::InvalidScalarProgram,
					format!("{function:?} changed an instruction result dtype"),
				));
			}
			values.insert(instruction.result, replacement);
		}
		let output = program.outputs.first().ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::InvalidScalarProgram,
				format!("{function:?} exposes no scalar output"),
			)
		})?;
		values.get(output).copied().ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::InvalidScalarProgram,
				format!("{function:?} exposes an unknown scalar output"),
			)
		})
	}

	fn finish(self, outputs: &[Value]) -> OperationResult<ScalarProgram> {
		let program = ScalarProgram {
			inputs: self.inputs,
			constants: self.constants,
			instructions: self.instructions,
			outputs: outputs.iter().map(|output| output.id).collect(),
		};
		program.validate().map_err(|errors| {
			OperationError::new(OperationErrorKind::InvalidScalarProgram, errors.to_string())
		})?;
		Ok(program)
	}

	fn next(&mut self, dtype: DType) -> OperationResult<Value> {
		let id = self.next_value;
		self.next_value = self.next_value.checked_add(1).ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::InvalidScalarProgram,
				"scalar value identity space exhausted",
			)
		})?;
		Ok(Value {
			id: ScalarValueId::new(id),
			dtype,
		})
	}
}
