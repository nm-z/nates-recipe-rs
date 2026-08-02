/// Stable identity for one deterministic implementation and coefficient set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlgorithmIdentity {
	pub name: &'static str,
	pub version: u32,
}

/// One endpoint of a finite input interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FiniteBound {
	Unbounded,
	Inclusive(f32),
	Exclusive(f32),
}

/// Domain of one finite scalar input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiniteInputDomain {
	pub name: &'static str,
	pub lower: FiniteBound,
	pub upper: FiniteBound,
	pub nonzero: bool,
}

/// Complete finite-input domain, including any relation between arguments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiniteDomain {
	pub inputs: &'static [FiniteInputDomain],
	pub relation: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonFiniteBehavior {
	/// NaN and either infinity set the kernel fault flag through `Require`.
	RejectWithRequire,
}

/// Behavior not fully described by the ordinary finite interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecialValueBehavior {
	pub non_finite: NonFiniteBehavior,
	pub signed_zero: &'static str,
	pub domain_violation: &'static str,
}

/// Error claim over the declared finite domain.
///
/// A result conforms when either the absolute or relative bound holds. Bounds
/// of zero denote an exact composition of the corresponding scalar primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorBound {
	pub maximum_absolute: f32,
	pub maximum_relative: f32,
	pub note: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MathContract {
	pub domain: FiniteDomain,
	pub special_values: SpecialValueBehavior,
	pub error: ErrorBound,
	pub algorithm: AlgorithmIdentity,
}

/// Shipped Recipe scalar-math surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MathFunction {
	Reciprocal,
	ReciprocalSquareRoot,
	Sin,
	Cos,
	Tan,
	Atan2,
	Exp,
	ExpWithGradualUnderflow,
	ExpMinusOne,
	Log,
	LogOnePlus,
	Floor,
	Ceil,
	RoundNearestEven,
	Trunc,
	Fmod,
	Pow,
	Sign,
	Sigmoid,
	Tanh,
	Softplus,
	Erf,
}

const ANY_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Unbounded,
	upper: FiniteBound::Unbounded,
	nonzero: false,
}];
const NONZERO_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Unbounded,
	upper: FiniteBound::Unbounded,
	nonzero: true,
}];
const POSITIVE_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Exclusive(0.0),
	upper: FiniteBound::Unbounded,
	nonzero: true,
}];
const TRIG_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Inclusive(-8192.0),
	upper: FiniteBound::Inclusive(8192.0),
	nonzero: false,
}];
const TAN_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Inclusive(-1.4),
	upper: FiniteBound::Inclusive(1.4),
	nonzero: false,
}];
const ATAN2_INPUTS: &[FiniteInputDomain] = &[
	FiniteInputDomain {
		name: "y",
		lower: FiniteBound::Unbounded,
		upper: FiniteBound::Unbounded,
		nonzero: false,
	},
	FiniteInputDomain {
		name: "x",
		lower: FiniteBound::Unbounded,
		upper: FiniteBound::Unbounded,
		nonzero: false,
	},
];
const EXP_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Inclusive(-80.0),
	upper: FiniteBound::Inclusive(80.0),
	nonzero: false,
}];
const NONPOSITIVE_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Unbounded,
	upper: FiniteBound::Inclusive(0.0),
	nonzero: false,
}];
const LOG1P_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Exclusive(-1.0),
	upper: FiniteBound::Unbounded,
	nonzero: false,
}];
const FMOD_INPUTS: &[FiniteInputDomain] = &[
	FiniteInputDomain {
		name: "dividend",
		lower: FiniteBound::Unbounded,
		upper: FiniteBound::Unbounded,
		nonzero: false,
	},
	FiniteInputDomain {
		name: "divisor",
		lower: FiniteBound::Unbounded,
		upper: FiniteBound::Unbounded,
		nonzero: true,
	},
];
const POW_INPUTS: &[FiniteInputDomain] = &[
	FiniteInputDomain {
		name: "base",
		lower: FiniteBound::Exclusive(0.0),
		upper: FiniteBound::Unbounded,
		nonzero: true,
	},
	FiniteInputDomain {
		name: "exponent",
		lower: FiniteBound::Unbounded,
		upper: FiniteBound::Unbounded,
		nonzero: false,
	},
];
const TANH_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Inclusive(-40.0),
	upper: FiniteBound::Inclusive(40.0),
	nonzero: false,
}];
const ERF_X: &[FiniteInputDomain] = &[FiniteInputDomain {
	name: "x",
	lower: FiniteBound::Inclusive(-6.0),
	upper: FiniteBound::Inclusive(6.0),
	nonzero: false,
}];

impl MathFunction {
	pub const ALL: [Self; 22] = [
		Self::Reciprocal,
		Self::ReciprocalSquareRoot,
		Self::Sin,
		Self::Cos,
		Self::Tan,
		Self::Atan2,
		Self::Exp,
		Self::ExpWithGradualUnderflow,
		Self::ExpMinusOne,
		Self::Log,
		Self::LogOnePlus,
		Self::Floor,
		Self::Ceil,
		Self::RoundNearestEven,
		Self::Trunc,
		Self::Fmod,
		Self::Pow,
		Self::Sign,
		Self::Sigmoid,
		Self::Tanh,
		Self::Softplus,
		Self::Erf,
	];

	#[must_use]
	pub const fn arity(self) -> usize {
		match self {
			Self::Atan2 | Self::Fmod | Self::Pow => 2,
			Self::Reciprocal
			| Self::ReciprocalSquareRoot
			| Self::Sin
			| Self::Cos
			| Self::Tan
			| Self::Exp
			| Self::ExpWithGradualUnderflow
			| Self::ExpMinusOne
			| Self::Log
			| Self::LogOnePlus
			| Self::Floor
			| Self::Ceil
			| Self::RoundNearestEven
			| Self::Trunc
			| Self::Sign
			| Self::Sigmoid
			| Self::Tanh
			| Self::Softplus
			| Self::Erf => 1,
		}
	}

	#[must_use]
	pub const fn contract(self) -> MathContract {
		let domain = self.domain();
		let special_values = SpecialValueBehavior {
			non_finite: NonFiniteBehavior::RejectWithRequire,
			signed_zero: self.signed_zero_behavior(),
			domain_violation: "sets the preallocated fault flag through Require",
		};
		MathContract {
			domain,
			special_values,
			error: self.error_bound(),
			algorithm: self.algorithm(),
		}
	}

	#[must_use]
	pub const fn algorithm(self) -> AlgorithmIdentity {
		let name = match self {
			Self::Reciprocal => "recipe.math.reciprocal.ieee-divide-f32",
			Self::ReciprocalSquareRoot => "recipe.math.rsqrt.sqrt-divide-f32",
			Self::Sin => "recipe.math.sin.cody-waite-s11-f32",
			Self::Cos => "recipe.math.cos.cody-waite-c12-f32",
			Self::Tan => "recipe.math.tan.cody-waite-ratio-f32",
			Self::Atan2 => "recipe.math.atan2.octant-atan15-f32",
			Self::Exp => "recipe.math.exp.bitpow2-taylor7-f32",
			Self::ExpWithGradualUnderflow => "recipe.math.exp.split-scale-subnormal-f32",
			Self::ExpMinusOne => "recipe.math.expm1.hybrid-taylor8-f32",
			Self::Log => "recipe.math.log.bitdecompose-atanh13-f32",
			Self::LogOnePlus => "recipe.math.log1p.hybrid-series12-f32",
			Self::Floor => "recipe.math.floor.scalar-primitive-f32",
			Self::Ceil => "recipe.math.ceil.scalar-primitive-f32",
			Self::RoundNearestEven => "recipe.math.rne.scalar-primitive-f32",
			Self::Trunc => "recipe.math.trunc.floor-ceil-select-f32",
			Self::Fmod => "recipe.math.fmod.ieee-remainder-f32",
			Self::Pow => "recipe.math.pow.log-exp-subnormal-v3-f32",
			Self::Sign => "recipe.math.sign.ordered-select-f32",
			Self::Sigmoid => "recipe.math.sigmoid.stable-exp-f32",
			Self::Tanh => "recipe.math.tanh.stable-expm1-f32",
			Self::Softplus => "recipe.math.softplus.stable-log1p-f32",
			Self::Erf => "recipe.math.erf.as-7.1.26-f32",
		};
		let version = match self {
			Self::Pow => 3,
			_ => 1,
		};
		AlgorithmIdentity { name, version }
	}

	const fn domain(self) -> FiniteDomain {
		match self {
			Self::Reciprocal => {
				FiniteDomain {
					inputs: NONZERO_X,
					relation: "x is finite and nonzero",
				}
			}
			Self::ReciprocalSquareRoot => {
				FiniteDomain {
					inputs: POSITIVE_X,
					relation: "x is finite and strictly positive",
				}
			}
			Self::Sin | Self::Cos => {
				FiniteDomain {
					inputs: TRIG_X,
					relation: "Cody-Waite reduction is bounded to |x| <= 8192",
				}
			}
			Self::Tan => {
				FiniteDomain {
					inputs: TAN_X,
					relation: "|x| <= 1.4 excludes every tangent pole",
				}
			}
			Self::Atan2 => {
				FiniteDomain {
					inputs: ATAN2_INPUTS,
					relation: "y and x are finite and not both zero",
				}
			}
			Self::Exp | Self::ExpMinusOne | Self::Sigmoid | Self::Softplus => {
				FiniteDomain {
					inputs: EXP_X,
					relation: "range reduction exponent remains a normal binary32 value",
				}
			}
			Self::ExpWithGradualUnderflow => {
				FiniteDomain {
					inputs: NONPOSITIVE_X,
					relation: "every finite x <= 0 is accepted; binary32 underflow uses round-to-nearest-even",
				}
			}
			Self::Log => {
				FiniteDomain {
					inputs: POSITIVE_X,
					relation: "all positive finite normals and subnormals are accepted",
				}
			}
			Self::LogOnePlus => {
				FiniteDomain {
					inputs: LOG1P_X,
					relation: "1 + x is strictly positive",
				}
			}
			Self::Floor | Self::Ceil | Self::RoundNearestEven | Self::Trunc | Self::Sign => {
				FiniteDomain {
					inputs: ANY_X,
					relation: "every finite binary32 value is accepted",
				}
			}
			Self::Fmod => {
				FiniteDomain {
					inputs: FMOD_INPUTS,
					relation: "the finite divisor is nonzero",
				}
			}
			Self::Pow => {
				FiniteDomain {
					inputs: POW_INPUTS,
					relation: "base > 0 and exponent * log(base) <= 80; binary32 underflow rounds to positive zero",
				}
			}
			Self::Tanh => {
				FiniteDomain {
					inputs: TANH_X,
					relation: "|2*x| remains inside the exp range-reduction domain",
				}
			}
			Self::Erf => {
				FiniteDomain {
					inputs: ERF_X,
					relation: "the shipped approximation is contracted over |x| <= 6",
				}
			}
		}
	}

	const fn signed_zero_behavior(self) -> &'static str {
		match self {
			Self::Reciprocal | Self::ReciprocalSquareRoot | Self::Log => {
				"positive and negative zero are rejected"
			}
			Self::Sin
			| Self::Tan
			| Self::ExpMinusOne
			| Self::LogOnePlus
			| Self::Floor
			| Self::Ceil
			| Self::RoundNearestEven
			| Self::Trunc
			| Self::Sign
			| Self::Tanh
			| Self::Erf => "an input signed zero is preserved exactly",
			Self::Cos | Self::Exp | Self::ExpWithGradualUnderflow | Self::Pow => {
				"either signed zero maps to positive one when it is in-domain"
			}
			Self::Atan2 => "signed y zero selects the signed x-axis result; the (zero, zero) pair is rejected",
			Self::Fmod => "a zero dividend preserves its sign",
			Self::Sigmoid => "either signed zero maps to positive 0.5",
			Self::Softplus => "either signed zero maps to positive ln(2)",
		}
	}

	const fn error_bound(self) -> ErrorBound {
		match self {
			Self::Reciprocal
			| Self::ReciprocalSquareRoot
			| Self::Floor
			| Self::Ceil
			| Self::RoundNearestEven
			| Self::Trunc
			| Self::Fmod
			| Self::Sign => {
				ErrorBound {
					maximum_absolute: 0.0,
					maximum_relative: 0.0,
					note: "exact composition of shipped IEEE scalar primitives",
				}
			}
			Self::Sin | Self::Cos => {
				ErrorBound {
					maximum_absolute: 2.0e-4,
					maximum_relative: 2.0e-4,
					note: "Cody-Waite reduction plus odd/even polynomial over the declared domain",
				}
			}
			Self::Tan => {
				ErrorBound {
					maximum_absolute: 5.0e-4,
					maximum_relative: 5.0e-4,
					note: "sine/cosine ratio away from poles",
				}
			}
			Self::Atan2 => {
				ErrorBound {
					maximum_absolute: 3.0e-5,
					maximum_relative: 3.0e-5,
					note: "octant reduction with degree-15 odd atan polynomial",
				}
			}
			Self::Exp => {
				ErrorBound {
					maximum_absolute: 2.0e-6,
					maximum_relative: 5.0e-6,
					note: "normal-power-of-two reconstruction with degree-7 residual polynomial",
				}
			}
			Self::ExpWithGradualUnderflow => {
				ErrorBound {
					maximum_absolute: 2.0e-6,
					maximum_relative: 5.0e-6,
					note: "split-scale power-of-two reconstruction preserves binary32 subnormals and rounds true underflow",
				}
			}
			Self::ExpMinusOne => {
				ErrorBound {
					maximum_absolute: 3.0e-6,
					maximum_relative: 7.0e-6,
					note: "degree-8 local series and range-reduced exp elsewhere",
				}
			}
			Self::Log => {
				ErrorBound {
					maximum_absolute: 6.0e-6,
					maximum_relative: 6.0e-6,
					note: "bit decomposition with an atanh series through y^13",
				}
			}
			Self::LogOnePlus => {
				ErrorBound {
					maximum_absolute: 8.0e-6,
					maximum_relative: 8.0e-6,
					note: "degree-12 local series and bit-decomposed log elsewhere",
				}
			}
			Self::Pow => {
				ErrorBound {
					maximum_absolute: 3.0e-5,
					maximum_relative: 2.0e-4,
					note: "positive-base log plus split-scale exp reconstruction through binary32 subnormals",
				}
			}
			Self::Sigmoid => {
				ErrorBound {
					maximum_absolute: 4.0e-6,
					maximum_relative: 6.0e-6,
					note: "sign-stable exp quotient",
				}
			}
			Self::Tanh => {
				ErrorBound {
					maximum_absolute: 8.0e-6,
					maximum_relative: 1.0e-5,
					note: "sign-stable expm1 quotient",
				}
			}
			Self::Softplus => {
				ErrorBound {
					maximum_absolute: 1.0e-5,
					maximum_relative: 1.0e-5,
					note: "max(x,0) plus log1p(exp(-abs(x)))",
				}
			}
			Self::Erf => {
				ErrorBound {
					maximum_absolute: 2.0e-6,
					maximum_relative: 2.0e-6,
					note: "Abramowitz-Stegun 7.1.26 with Recipe exp",
				}
			}
		}
	}
}
