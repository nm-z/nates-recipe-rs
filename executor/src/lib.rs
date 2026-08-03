#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Backend-neutral execution of immutable [`recipe_core::FinalizedBundle`]s.
//!
//! The public lifecycle is deliberately expressed as owned typestate handles:
//! [`PreparedRun`] -> [`InitializedRun`] -> [`RunningRun`] -> [`ExitedLoop`] ->
//! [`ExitedRun`]. A running handle exposes progress and metric operations only;
//! allocation, compilation, module loading, and external data ingress are not
//! part of its API.

mod backend; mod error; mod executor; mod metrics;

#[doc(hidden)]
pub use backend::sealed; pub use backend::{
	ArenaSet, Backend, BackendPoll, BackendWork, CalculationWork, InitAdmissionWork,
	MAX_PHYSICAL_CALLS_PER_OPERATION, MetricWork, PendingRequest, PhysicalCall, PhysicalCallBatch,
	PhysicalCallBatchOverflow, PhysicalPollStatus, TransferWork, WorkClass, };
pub use error::{BACKEND_MESSAGE_CAPACITY, BackendMessage, BackendOperation, ExecutorError, Result}; pub use executor::{
	DeviceImage, ExitImage, ExitedLoop, ExitedRun, InitializedRun, JournalCapacity, JournalSummary, LogicalEvent,
	LoopStatus, PendingPollCount, PreparedRun, RunFailure, RunFailureParts, RunJournal, RunningRun, Watchdog, };
pub use metrics::{MetricMailbox, MetricSample, MetricValue};
