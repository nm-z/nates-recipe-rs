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

pub use contract::{
	AlgorithmIdentity, ErrorBound, FiniteBound, FiniteDomain, FiniteInputDomain, MathContract, MathFunction,
	NonFiniteBehavior, SpecialValueBehavior,
};

impl TryFrom<MathFunction> for ScalarProgram {
	type Error = LanguageError;

	fn try_from(function: MathFunction) -> Result<Self, Self::Error> {
		program::build(function)
	}
}

#[cfg(test)]
mod tests;
