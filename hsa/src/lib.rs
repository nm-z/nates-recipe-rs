//! Reviewed ROCr/HSA discovery and queue ownership.
//!
//! It supplies exact identities and scoped runtime objects without introducing
//! a process-global runtime. Execution remains explicit and nonblocking:
//! submission returns an owned completion token.

#![cfg_attr(not(target_pointer_width = "64"), allow(dead_code))]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("recipe-hsa currently implements only the 64-bit HSA large-model ABI");

mod abi;
mod discovery;
mod error;
mod execution;
mod identity;
mod loader;
mod runtime;
mod session;

pub use discovery::{AgentDescription, AgentIdentity, AllocationProperties, AmdGpuProperties, DiscoveredAgent, Discovery, IsaDescription, MemoryPoolDescription, QueueCapabilities, SystemDescription};
pub use error::{Error, Result};
pub use execution::{Allocation, Dependency, DispatchGeometry, Executable, Kernel, KernelMetadata, Pending, PollStatus, PreparedPending, QueueProgress, RetirementReport, WaitOutcome, dependent_dispatch_packet_count};
pub use identity::{AgentUuid, AgentUuidBody, DeviceType, IsaFeature, IsaTarget, MemoryLocation, MemoryPoolFlags, MemorySegment, PciAddress, Profile, QueueKind, RoundingModes};
pub use runtime::Runtime;
pub use session::{Queue, QueueConfig, QueueFault, Session};
