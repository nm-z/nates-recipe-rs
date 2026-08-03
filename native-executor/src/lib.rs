//! Native GPU resources below Recipe's backend-neutral executor boundary.
//!
//! This crate validates an immutable [`ExecutionPlan`], realizes CUDA Driver
//! API or ROCr/HSA resources before the loop, owns exact device arenas, and
//! translates closed [`recipe_executor::BackendWork`] values into native
//! asynchronous operations. It deliberately provides no compiler, discovery,
//! allocator, module loader, or topology mutation through its running API.
//!
//! CUDA and HSA adapters implement `recipe_executor::Backend` over those
//! realized resources. Remaining shared strict-loop constraints are recorded
//! in `INTEGRATION_REQUIRED.md`.

#![deny(missing_debug_implementations)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod accounting;
mod bridge;
pub mod candidate;
mod cuda_ffi;
mod error;
mod evidence;
mod local;
mod plan;

pub mod cuda;
pub mod hsa;

pub use bridge::{StagedBridgeError, StagedBridgePending, StagedBridgeResources, StagedCrossBackend};
pub use candidate::{
	CandidateArtifact, CandidateFailure, CandidateRealizationRequest, CandidateRequestError, CandidateSessionFactory,
	UnavailableCandidateError, UnavailableCandidateFactory, ValidatedCandidateFactory, ValidatedCandidateSession,
};
pub use cuda::{CUDA_MAXIMUM_SUBMISSION_QUEUES, CudaBackend, CudaBinding};
pub use error::{Error, Result};
pub use evidence::{NativeBackendKind, NativeDeviceExecutionEvidence, NativeExecutionEvidence};
pub use hsa::{HsaBackend, HsaBinding, HsaBindingProperties};
pub use local::{
	CandidateCrossBackendTransfer, CrossBackendTransfer, CrossBackendUnavailable, LocalArena, LocalArenaRef,
	LocalArenaSet, LocalBackend, LocalCandidateFactory, LocalCandidateStabilizer, LocalDeviceClass, LocalError,
	LocalPending, LocalPreparedSession, LocalResources, PreparedBridge, PreparedBridgeError, RejectCrossBackend,
	UnavailableLocalStabilizer,
};
pub(crate) use plan::{ExecutionPlan, PlannedSubmission};
pub use plan::{RuntimeArtifact, RuntimeArtifactKind, RuntimeImage};
