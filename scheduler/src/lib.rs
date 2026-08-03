#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Deterministic static scheduling and arena placement for Recipe.
//!
//! The base policy consumes only measured or explicitly overridden properties.
//! Theoretical values may seed probing, but cannot reach this crate's schedule
//! entry point because the topology and discovery validators reject them.
//! Distinct-device transfers presented to the scheduler are one-link physical
//! hops; route decomposition and intermediate allocation belong to the planner.

mod arena; mod error; mod route; mod static_schedule;

pub use arena::pack_arenas; pub use error::{ScheduleError, ScheduleErrorKind}; pub use route::{Route, shortest_route};
pub use static_schedule::{StaticSchedule, UnscheduledTask, schedule};
