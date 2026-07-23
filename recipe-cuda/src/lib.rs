#![allow(
	unsafe_code,
	reason = "reviewed CUDA Driver API and dlopen/dlsym FFI boundary"
)]
#![cfg_attr(
	not(target_os = "linux"),
	allow(dead_code, unused_imports),
	reason = "the production loader is Linux-only"
)]

#[cfg(not(target_os = "linux"))]
compile_error!("recipe-cuda currently supports Linux libcuda.so deployments only");
#[cfg(not(target_pointer_width = "64"))]
compile_error!("recipe-cuda requires a 64-bit CUDA Driver ABI");

mod artifact;
mod context;
mod discovery;
mod driver;
mod error;
mod ffi;
mod runtime;

pub use artifact::{
	ArtifactCompatibilityError, ArtifactField, ArtifactIdentity, ArtifactIssue, DeploymentIdentity,
	ToolchainIdentity, validate_artifact_compatibility,
};
pub use context::{Context, ContextFlags, ContextGuard, SchedulingPolicy};
pub use discovery::{
	ComputeCapability, DeviceAttributes, DeviceInfo, DeviceOrdinal, DeviceUuid, Discovery, DriverVersion,
};
pub use driver::{Driver, ModuleLoadingMode};
pub use error::{CudaError, DriverCallError, DriverStatus, Result};
pub use ffi::{DriverCapabilities, DriverSymbol, OPTIONAL_DRIVER_SYMBOLS, REQUIRED_DRIVER_SYMBOLS};
pub use runtime::{
	CompletionStatus, DeviceBuffer, Dim3, Event, Function, LaunchConfig, Module, Pending, PinnedHostBuffer, Stream,
	WaitOutcome,
};
