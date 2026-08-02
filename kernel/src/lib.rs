#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Recipe-owned lowering from the backend-neutral scalar IR to target LLVM IR.
//!
//! This crate emits direct kernel definitions rather than rewriting aliases in
//! compiler-produced text. LLVM then lowers the module to inspected AMDGPU ISA
//! or PTX, and the offline artifact builder packages an HSACO or cubin.

mod artifact;
mod audit;
mod builder;
mod error;
mod llvm;
mod stage;
mod target;

pub use artifact::{
	ArtifactDigest, HsaKernelArgument, HsaKernelMetadata, InspectedCubin, InspectedHsaco, inspect_cubin,
	inspect_hsaco, inspect_hsaco_bundle,
};
pub use audit::{AuditFinding, AuditKind, audit_llvm_ir};
pub use builder::{
	ArtifactBuilder, BuildPhase, BuildProvenance, BuiltArtifact, BuiltCubinBundle, BuiltHsacoBundle,
	CubinBundleProvenance, HsacoBundleProvenance, OfflineToolchain, PinnedTool, ToolInvocation,
};
pub use error::{LoweringError, LoweringErrorKind};
pub use llvm::{BufferAccess, KernelAbi, KernelArgument, LoweredKernel, LoweringOptions, lower_elementwise};
pub use target::{AmdTarget, KernelTarget, NvidiaTarget};

/// Recompute the planner's canonical deferred-artifact contract digest.
#[must_use]
pub fn artifact_build_contract_digest(build: &recipe_core::ArtifactBuildRecipe) -> recipe_core::Digest {
	stage::artifact_build_contract_digest(build)
}

/// Realize one exact primitive stage into target LLVM IR.
pub fn lower_stage(
	program: &recipe_primitives::LoweredProgram,
	build: &recipe_core::ArtifactBuildRecipe,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<LoweredKernel, LoweringError> {
	stage::lower_stage(program, build, target, options)
}
