#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Recipe-owned scalar math composed entirely from the backend-neutral scalar
//! instruction set.
//!
//! Each generated program has a versioned algorithm identity, an explicit
//! finite input domain, stated special-value behavior, and a numerical error
//! contract. Domain checks are part of the generated program through
//! [`recipe_core::ScalarOpcode::Require`].

use recipe_core::ScalarProgram;
use recipe_language::LanguageError;

mod contract;
mod program;

pub use contract::{AlgorithmIdentity, ErrorBound, FiniteBound, FiniteDomain, FiniteInputDomain, MathContract, MathFunction, NonFiniteBehavior, SpecialValueBehavior};

/// Build Recipe's finite, nonpositive-input exponential program with gradual
/// binary32 underflow.
///
/// Unlike [`MathFunction::Exp`], this specialized program admits every finite
/// nonpositive input. It preserves subnormal results and rounds to positive
/// zero only at the binary32 half-minimum-subnormal boundary. Positive and
/// non-finite inputs report a device fault through the generated program.
///
/// # Errors
///
/// Returns a language error if the deterministic scalar program cannot be
/// constructed.
pub fn exp_with_gradual_underflow_program() -> Result<ScalarProgram, LanguageError> { ScalarProgram::try_from(MathFunction::ExpWithGradualUnderflow) }

impl TryFrom<MathFunction> for ScalarProgram {
	type Error = LanguageError;

	fn try_from(function: MathFunction) -> Result<Self, Self::Error> { program::build(function) }
}
