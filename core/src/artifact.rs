use crate::{
	error::{ValidationCode, ValidationResult, Validator},
	identity::{Digest, Label},
	ids::{ArtifactId, KernelTemplateId, ValueId},
	scalar::DType,
	units::{ByteCount, FlopCount},
};

/// Opaque calculation-target identity selected through discovered capability.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TargetIdentity {
	pub backend: Label,
	pub architecture: Label,
	pub abi: Label,
}

/// Exact toolchain identity used to produce an artifact.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolchainIdentity {
	pub name: Label,
	pub version: Label,
	pub digest: Digest,
}

/// Conservative resource envelope fixed during Draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelResourceBounds {
	pub private_bytes_per_lane: ByteCount,
	pub shared_bytes_per_workgroup: ByteCount,
	pub scratch_bytes_per_dispatch: ByteCount,
	pub maximum_workgroup_lanes: u32,
}

/// Stable provenance that binds a realized image to one immutable stage
/// recipe without pretending that its target or toolchain existed at Draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactBuildProvenance {
	pub program_digest: Digest,
	pub stage_ordinal: u32,
	pub contract_digest: Digest,
}

/// Access contract for one ordered stage argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactBuildAccess {
	Read,
	Write,
	ReadWrite,
	ReadWriteAtomic,
}

impl ArtifactBuildAccess {
	#[must_use]
	pub const fn reads(self) -> bool { matches!(self, Self::Read | Self::ReadWrite | Self::ReadWriteAtomic) }

	#[must_use]
	pub const fn writes(self) -> bool { matches!(self, Self::Write | Self::ReadWrite | Self::ReadWriteAtomic) }
}

/// Complete immutable affine view compiled into one stage argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactBuildView {
	pub logical_extents: Vec<u64>,
	pub offset_elements: u64,
	pub strides: Vec<u64>,
	pub storage_bytes: ByteCount,
}

/// Candidate-local value bound to one ordered stage argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactBuildBinding {
	pub value: ValueId,
	pub dtype: DType,
	pub access: ArtifactBuildAccess,
	pub view: ArtifactBuildView,
}

/// Fixed launch geometry selected before target realization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactDispatchGeometry {
	pub logical_lanes: u64,
	pub workgroup_lanes: u32,
	pub workgroups: u64,
}

/// Exact operation bounds retained even when the measured scheduler currently
/// prices only floating-point work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactWorkBounds {
	pub flops: FlopCount,
	pub integer_operations: u64,
	pub atomic_operations: u64,
}

/// Target-independent request to build one stage artifact during Realize.
///
/// `artifact` reserves the ID ultimately carried by [`ArtifactIdentity`].
/// The recipe deliberately contains no target, format, entry symbol, or
/// toolchain identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactBuildRecipe {
	pub artifact: ArtifactId,
	/// Collision-checked stage-scoped identity, not the source primitive ID.
	pub kernel_template: KernelTemplateId,
	pub source_kernel: KernelTemplateId,
	pub provenance: ArtifactBuildProvenance,
	pub bindings: Vec<ArtifactBuildBinding>,
	pub dispatch: ArtifactDispatchGeometry,
	pub work: ArtifactWorkBounds,
	pub fault_flag: Option<ValueId>,
	pub resources: KernelResourceBounds,
}

impl ArtifactBuildRecipe {
	pub fn validate(&self) -> ValidationResult {
		let mut validator = Validator::default();
		validator.require(
			self.artifact.get() != 0,
			ValidationCode::InvalidIdentity,
			"artifact",
			"reserved artifact ID must not be zero",
		);
		validator.require(
			self.kernel_template.get() != 0,
			ValidationCode::InvalidIdentity,
			"kernel_template",
			"stage-scoped kernel identity must not be zero",
		);
		validator.require(
			self.source_kernel.get() != 0,
			ValidationCode::InvalidIdentity,
			"source_kernel",
			"source kernel identity must not be zero",
		);
		validator.require(
			!self.provenance.program_digest.is_zero(),
			ValidationCode::InvalidIdentity,
			"provenance.program_digest",
			"lowered-program digest must not be zero",
		);
		validator.require(
			!self.provenance.contract_digest.is_zero(),
			ValidationCode::InvalidIdentity,
			"provenance.contract_digest",
			"stage contract digest must not be zero",
		);
		validator.require(
			self.dispatch.logical_lanes != 0,
			ValidationCode::MissingRequiredObject,
			"dispatch.logical_lanes",
			"a deferred stage must dispatch at least one logical lane",
		);
		validator.require(
			self.dispatch.workgroup_lanes != 0,
			ValidationCode::MissingRequiredObject,
			"dispatch.workgroup_lanes",
			"workgroup lane count must not be zero",
		);
		let expected_workgroups = self
			.dispatch
			.logical_lanes
			.checked_add(u64::from(self.dispatch.workgroup_lanes).saturating_sub(1))
			.map(|rounded| rounded / u64::from(self.dispatch.workgroup_lanes));
		validator.require(
			expected_workgroups == Some(self.dispatch.workgroups),
			ValidationCode::ResourceMismatch,
			"dispatch.workgroups",
			"workgroup count differs from the exact ceiling division",
		);
		validator.require(
			self.resources.maximum_workgroup_lanes == self.dispatch.workgroup_lanes,
			ValidationCode::ResourceMismatch,
			"resources.maximum_workgroup_lanes",
			"resource envelope must retain the exact drafted workgroup size",
		);
		let mut values = std::collections::BTreeSet::new();
		for (index, binding) in self.bindings.iter().enumerate() {
			let path = format!("bindings[{index}]");
			validator.require(
				values.insert(binding.value),
				ValidationCode::DuplicateId,
				format!("{path}.value"),
				format!("value {} is bound more than once", binding.value),
			);
			validator.require(
				!binding.view.logical_extents.is_empty(),
				ValidationCode::MissingRequiredObject,
				format!("{path}.view.logical_extents"),
				"stage binding view must have an explicit rank",
			);
			validator.require(
				binding.view.logical_extents.len() == binding.view.strides.len(),
				ValidationCode::ResourceMismatch,
				format!("{path}.view.strides"),
				"stage binding extent and stride ranks differ",
			);
		}
		if let Some(fault_flag) = self.fault_flag {
			validator.require(
				self.bindings
					.iter()
					.any(|binding| binding.value == fault_flag && binding.dtype == DType::I32 && binding.access == ArtifactBuildAccess::ReadWriteAtomic && binding.view.storage_bytes == ByteCount::new(4)),
				ValidationCode::ResourceMismatch,
				"fault_flag",
				"fault flag must name its exact int32 atomic binding",
			);
		}
		validator.finish()
	}
}

/// Complete identity and resource contract for one drafted kernel artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactIdentity {
	pub id: ArtifactId,
	pub digest: Digest,
	pub format: Label,
	pub target: TargetIdentity,
	pub toolchain: ToolchainIdentity,
	pub entry_symbol: Label,
	pub kernel_template: KernelTemplateId,
	pub resources: KernelResourceBounds,
	/// Present exactly when this image realizes a deferred Draft recipe.
	pub build: Option<ArtifactBuildProvenance>,
}

impl ArtifactIdentity {
	pub fn validate(&self) -> ValidationResult {
		let mut validator = Validator::default();
		validator.require(
			!self.digest.is_zero(),
			ValidationCode::InvalidIdentity,
			"digest",
			"artifact digest must not be zero",
		);
		validator.require(
			!self.toolchain.digest.is_zero(),
			ValidationCode::InvalidIdentity,
			"toolchain.digest",
			"toolchain digest must not be zero",
		);
		validator.require(
			self.resources.maximum_workgroup_lanes != 0,
			ValidationCode::MissingRequiredObject,
			"resources.maximum_workgroup_lanes",
			"maximum workgroup size must be nonzero",
		);
		if let Some(build) = self.build {
			validator.require(
				!build.program_digest.is_zero(),
				ValidationCode::InvalidIdentity,
				"build.program_digest",
				"deferred-build program digest must not be zero",
			);
			validator.require(
				!build.contract_digest.is_zero(),
				ValidationCode::InvalidIdentity,
				"build.contract_digest",
				"deferred-build contract digest must not be zero",
			);
		}
		validator.finish()
	}
}
