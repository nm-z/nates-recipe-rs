#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Preallocated asynchronous host-storage resources.
//!
//! This crate performs byte transport only. It has no payload calculation,
//! discovery, scheduling, compilation, or file-format behavior. Every worker,
//! job slot, staging buffer, RAM allocation, and disk file is created before
//! the corresponding run enters its loop.

mod arena;
mod backend;
mod error;
mod runtime;

pub use arena::{Arena, ArenaKind, DiskFileSpec};
pub use backend::{
	HostArenaLookup, HostBackend, HostBackendConfig, HostDeviceBinding, HostPending, HostPreparedResources,
	HostResources,
};
pub use error::{Error, Result};
pub use runtime::{PendingCopy, PollStatus, Runtime, RuntimeConfig, RuntimeState, SlotCapacity};
