#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Deterministic, backend-neutral AOT lowering for Recipe-owned GPU
//! primitives.
//!
//! This crate deliberately stops before target realization. It fixes dispatch
//! geometry, buffer views, collective trees, synchronization, atomics, fault
//! behavior, random-number semantics, and resource bounds without selecting a
//! machine, driver API, or vendor library.

mod error;
mod hash;
mod lower;
mod model;
mod validate;

pub use error::{LoweringError, LoweringErrorKind, LoweringResult, ProgramValidationError};
pub use lower::lower;
pub use model::*;
