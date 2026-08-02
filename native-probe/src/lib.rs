//! Bare-metal GPU discovery and measurement for `recipe probe`.
//!
//! Backend libraries and compiler tools are selected through explicit paths.
//! Theoretical seed values only bound benchmark work; every returned property
//! is obtained from current discovery or a completed native submission.
//! A separate preparation-scoped entry point reopens measured GPU origins by
//! exact key and lends CUDA/HSA bindings whose lifetimes cannot escape that
//! realized scope.

mod benchmark;
mod bindings;
mod config;
mod cuda;
mod hsa;
mod identity;
mod native;

pub use bindings::{NativeExecutionBindings, host_backend_config_from_inventory, with_native_execution_bindings};
pub use config::{BackendLibrary, CudaProbeConfig, HsaProbeConfig, KernelBuildConfig, NativeProbeConfig};
pub use native::NativeGpuProbe;
