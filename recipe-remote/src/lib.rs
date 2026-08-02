#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Bounded master/worker execution over an already connected Recipe channel.
//!
//! This crate owns no listener, socket establishment, discovery, compiler,
//! artifact loader, or file-data path. Callers provision the same finalized
//! identities on both peers and supply an already connected
//! [`recipe_transport::RuntimeChannel`].

mod codec;
mod driver;
mod error;
mod executor_driver;
mod model;
mod session;

pub use driver::{DriverFault, DriverFaultCode, DriverPoll, DriverTransferPoll, WorkerDriver};
pub use error::{RemoteError, RemoteResult};
pub use executor_driver::{ExecutorDriverBuildError, ExecutorWorkerDriver};
pub use model::{
	Capabilities, CrossTransfer, DataDirection, DuplexClaim, Manifest, ManifestArtifact, ManifestLimits,
	ProvisionedProgram, RemoteIdentity, RemoteLimits, RuntimeCapacities, RuntimeSlots,
};
pub use session::{
	Advance, CancelReason, MasterCancelling, MasterComplete, MasterExit, MasterHandshake, MasterInit, MasterRun,
	MasterRunEvent, MetricSample, RemoteChannel, RemoteMetricValue, WorkerCancelled, WorkerComplete, WorkerExit,
	WorkerExitProgress, WorkerHandshake, WorkerInit, WorkerRun, WorkerRunEvent, WorkerRunProgress,
};

/// Recipe remote-execution wire version.
pub const PROTOCOL_VERSION: u16 = 2;
