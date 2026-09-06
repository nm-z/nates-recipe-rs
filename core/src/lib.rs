#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Dependency-free shared types for Recipe's static execution pipeline.
//!
//! This crate contains no runtime or driver integration. It models the values
//! exchanged by discovery, drafting, realization, and finalization, and
//! validates their cross-stage invariants.

pub mod artifact;
pub mod discovery;
pub mod error;
pub mod identity;
pub mod ids;
pub mod model;
pub mod plan;
pub mod scalar;
pub mod schedule;
pub mod topology;
pub mod units;

pub use artifact::*;
pub use discovery::*;
pub use error::{ValidationCode, ValidationError, ValidationErrors, ValidationResult};
pub use identity::{BundleIdentity, CandidateIdentity, Digest, DiscoveryIdentity, DraftIdentity, Label, LabelError, RealizationIdentity, TopologyIdentity};
pub use ids::*;
pub use model::*;
pub use plan::*;
pub use scalar::*;
pub use schedule::*;
pub use topology::*;
pub use units::*;
