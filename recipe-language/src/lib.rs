#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Backend-neutral, statically shaped calculation language for Recipe.
//!
//! This layer replaces backend source strings and vendor-library operation
//! calls. It describes calculation payloads and a small set of Recipe-owned
//! parallel primitives. Placement, transfers, drivers, and lifecycle state do
//! not belong here.
//!
//! [`CalculationGraph`] is the typed in-memory contract. Its canonical textual
//! representation is the versioned, OGDL-derived format produced by
//! [`CalculationGraph::to_ogdl`] and accepted by
//! [`CalculationGraph::from_ogdl`]. Decoding is strict: fields and enum
//! spellings are exact, collections are explicit, and no missing value receives
//! a default.

mod error;
mod graph;
mod ogdl;
mod primitive;
mod scalar_builder;
mod shape;
mod tensor;

pub use error::{LanguageError, LanguageErrorKind, LanguageResult};
pub use graph::{CalculationGraph, CalculationNode};
pub use ogdl::{OgdlCodecError, OgdlDocumentErrorKind};
pub use primitive::{
	AtomicOperation, AtomicOrdering, Contraction, Elementwise, Gather, Histogram, IndexBounds, IndexMap,
	PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, RandomDistribution, RandomKey, RandomMap, Reduce,
	ReduceOperator, ReduceResult, Scan, ScanMode, Scatter, ScatterConflict, Sort, SortDirection,
};
pub use scalar_builder::{ScalarExpression, ScalarProgramBuilder};
pub use shape::{AxisSet, Shape};
pub use tensor::{ContiguousOrder, Tensor, TensorLayout};
