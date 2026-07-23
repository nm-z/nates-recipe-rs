#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Backend-neutral, statically shaped calculation language for Recipe.
//!
//! This layer replaces backend source strings and vendor-library operation
//! calls. It describes calculation payloads and a small set of Recipe-owned
//! parallel primitives. Placement, transfers, drivers, and lifecycle state do
//! not belong here.

mod error;
mod graph;
mod primitive;
mod scalar_builder;
mod shape;
mod tensor;

pub use error::{LanguageError, LanguageErrorKind, LanguageResult};
pub use graph::{CalculationGraph, CalculationNode};
pub use primitive::{
	AtomicOperation, AtomicOrdering, Contraction, Elementwise, Gather, Histogram, IndexBounds, PrimitiveAliasRule,
	PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce, ReduceOperator, ReduceResult,
	Scan, ScanMode, Scatter, ScatterConflict, Sort, SortDirection,
};
pub use scalar_builder::{ScalarExpression, ScalarProgramBuilder};
pub use shape::{AxisSet, Shape};
pub use tensor::{ContiguousOrder, Tensor, TensorLayout};
