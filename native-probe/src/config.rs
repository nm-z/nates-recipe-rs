use std::path::PathBuf;

use recipe_core::Label; use recipe_kernel::OfflineToolchain;

/// Ordered, explicit shared-library candidates for one native backend.
///
/// Missing candidates mean that backend is absent. Once any candidate exists,
/// inability to hash, load, or exhaustively query it is a hard error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendLibrary { pub candidates: Vec<PathBuf>, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaProbeConfig { pub library: BackendLibrary,
	/// PTX ISA encoded as major * 10 + minor.
	pub ptx_isa: u16, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HsaProbeConfig { pub library: BackendLibrary, pub code_object_version: u8, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelBuildConfig { pub toolchain: OfflineToolchain,
	/// Human-readable release identity paired with the exact binary digest.
	pub release: Label, pub scratch_parent: PathBuf,
	/// Number of dependent Recipe-owned f32 FMA operations per element.
	pub fma_chain_length: u16, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeProbeConfig {
	/// Host RAM domain emitted by the host-discovery half of `recipe probe`.
	pub host_memory_key: Label,
	/// Explicit PCI sysfs root, normally `/sys/bus/pci/devices`.
	pub pci_sysfs_root: PathBuf, pub cuda: CudaProbeConfig, pub hsa: HsaProbeConfig, pub kernels: KernelBuildConfig, }
