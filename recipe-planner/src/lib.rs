#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Finite, deterministic Draft candidate enumeration over measured topology.

mod error;
mod hash;
mod planner;

pub use error::{PlannerError, PlannerErrorKind, PlannerResult};
pub use planner::{
	KernelPlacement, LogicalValueCopy, PlannedCandidate, PlannerSearch, StagePlacement, plan_candidates,
};
