use crate::discovery::{ComputeCapability, DeviceInfo, DeviceUuid, Discovery, DriverVersion};
use crate::ffi::{DriverCapabilities, DriverSymbol};
use core::fmt;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolchainIdentity {
	pub zig_version: String,
	pub llvm_version: String,
	pub ptx_isa_version: String,
	pub ptxas_version: String,
	pub cuda_toolkit_version: String,
	pub cubin_format: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactIdentity {
	pub sha256: [u8; 32],
	pub target: ComputeCapability,
	pub toolchain: ToolchainIdentity,
	pub minimum_driver: DriverVersion,
	pub maximum_driver: Option<DriverVersion>,
	pub required_driver_symbols: BTreeSet<DriverSymbol>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeploymentIdentity {
	pub driver_version: DriverVersion,
	pub device_uuid: DeviceUuid,
	pub target: ComputeCapability,
	pub driver_capabilities: DriverCapabilities,
}

impl DeploymentIdentity {
	pub fn from_discovery(discovery: &Discovery, device: &DeviceInfo) -> Option<Self> {
		if !discovery
			.devices
			.iter()
			.any(|candidate| candidate.ordinal == device.ordinal && candidate.uuid == device.uuid)
		{
			return None;
		}
		Some(Self {
			driver_version: discovery.driver_version,
			device_uuid: device.uuid,
			target: device.compute_capability,
			driver_capabilities: discovery.capabilities.clone(),
		})
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArtifactField {
	Sha256,
	Target,
	ZigVersion,
	LlvmVersion,
	PtxIsaVersion,
	PtxasVersion,
	CudaToolkitVersion,
	CubinFormat,
	MinimumDriver,
	MaximumDriver,
	RequiredDriverSymbols,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactIssue {
	EmptyArtifact,
	InvalidCubinMagic,
	DigestMismatch {
		declared: [u8; 32],
		computed: [u8; 32],
	},
	IdentityFieldMismatch {
		field: ArtifactField,
	},
	EmptyIdentityField {
		field: ArtifactField,
	},
	InvalidDriverRange {
		minimum: DriverVersion,
		maximum: DriverVersion,
	},
	DeviceTargetMismatch {
		artifact: ComputeCapability,
		device: ComputeCapability,
	},
	DriverTooOld {
		artifact_minimum: DriverVersion,
		deployed: DriverVersion,
	},
	DriverTooNew {
		artifact_maximum: DriverVersion,
		deployed: DriverVersion,
	},
	MissingDriverSymbol {
		symbol: DriverSymbol,
	},
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactCompatibilityError {
	pub issues: Vec<ArtifactIssue>,
}

impl fmt::Display for ArtifactCompatibilityError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"CUDA artifact identity is incompatible ({} issue(s))",
			self.issues.len()
		)
	}
}

impl std::error::Error for ArtifactCompatibilityError {}

pub fn validate_artifact_compatibility(
	cubin: &[u8],
	expected: &ArtifactIdentity,
	observed: &ArtifactIdentity,
	deployment: &DeploymentIdentity,
) -> core::result::Result<(), ArtifactCompatibilityError> {
	let mut issues = Vec::new();

	if cubin.is_empty() {
		issues.push(ArtifactIssue::EmptyArtifact);
	}
	if !cubin.starts_with(b"\x7fELF") {
		issues.push(ArtifactIssue::InvalidCubinMagic);
	}
	let computed: [u8; 32] = Sha256::digest(cubin).into();
	if observed.sha256 != computed {
		issues.push(ArtifactIssue::DigestMismatch {
			declared: observed.sha256,
			computed,
		});
	}

	compare_identity(expected, observed, &mut issues);
	validate_nonempty(observed, &mut issues);

	if let Some(maximum) = observed.maximum_driver
		&& observed.minimum_driver > maximum
	{
		issues.push(ArtifactIssue::InvalidDriverRange {
			minimum: observed.minimum_driver,
			maximum,
		});
	}
	if observed.target != deployment.target {
		issues.push(ArtifactIssue::DeviceTargetMismatch {
			artifact: observed.target,
			device: deployment.target,
		});
	}
	if deployment.driver_version < observed.minimum_driver {
		issues.push(ArtifactIssue::DriverTooOld {
			artifact_minimum: observed.minimum_driver,
			deployed: deployment.driver_version,
		});
	}
	if let Some(maximum) = observed.maximum_driver
		&& deployment.driver_version > maximum
	{
		issues.push(ArtifactIssue::DriverTooNew {
			artifact_maximum: maximum,
			deployed: deployment.driver_version,
		});
	}
	for symbol in &observed.required_driver_symbols {
		if !deployment.driver_capabilities.supports(*symbol) {
			issues.push(ArtifactIssue::MissingDriverSymbol { symbol: *symbol });
		}
	}

	if issues.is_empty() {
		Ok(())
	} else {
		Err(ArtifactCompatibilityError { issues })
	}
}

fn compare_identity(expected: &ArtifactIdentity, observed: &ArtifactIdentity, issues: &mut Vec<ArtifactIssue>) {
	for (field, matches) in [
		(ArtifactField::Sha256, expected.sha256 == observed.sha256),
		(ArtifactField::Target, expected.target == observed.target),
		(
			ArtifactField::ZigVersion,
			expected.toolchain.zig_version == observed.toolchain.zig_version,
		),
		(
			ArtifactField::LlvmVersion,
			expected.toolchain.llvm_version == observed.toolchain.llvm_version,
		),
		(
			ArtifactField::PtxIsaVersion,
			expected.toolchain.ptx_isa_version == observed.toolchain.ptx_isa_version,
		),
		(
			ArtifactField::PtxasVersion,
			expected.toolchain.ptxas_version == observed.toolchain.ptxas_version,
		),
		(
			ArtifactField::CudaToolkitVersion,
			expected.toolchain.cuda_toolkit_version == observed.toolchain.cuda_toolkit_version,
		),
		(
			ArtifactField::CubinFormat,
			expected.toolchain.cubin_format == observed.toolchain.cubin_format,
		),
		(
			ArtifactField::MinimumDriver,
			expected.minimum_driver == observed.minimum_driver,
		),
		(
			ArtifactField::MaximumDriver,
			expected.maximum_driver == observed.maximum_driver,
		),
		(
			ArtifactField::RequiredDriverSymbols,
			expected.required_driver_symbols == observed.required_driver_symbols,
		),
	] {
		if !matches {
			issues.push(ArtifactIssue::IdentityFieldMismatch { field });
		}
	}
}

fn validate_nonempty(identity: &ArtifactIdentity, issues: &mut Vec<ArtifactIssue>) {
	for (field, text) in [
		(ArtifactField::ZigVersion, &identity.toolchain.zig_version),
		(ArtifactField::LlvmVersion, &identity.toolchain.llvm_version),
		(
			ArtifactField::PtxIsaVersion,
			&identity.toolchain.ptx_isa_version,
		),
		(
			ArtifactField::PtxasVersion,
			&identity.toolchain.ptxas_version,
		),
		(
			ArtifactField::CudaToolkitVersion,
			&identity.toolchain.cuda_toolkit_version,
		),
		(ArtifactField::CubinFormat, &identity.toolchain.cubin_format),
	] {
		if text.trim().is_empty() {
			issues.push(ArtifactIssue::EmptyIdentityField { field });
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Driver;
	use crate::ffi::{DriverSymbol, test_support};

	fn identities() -> (
		Vec<u8>,
		ArtifactIdentity,
		DeploymentIdentity,
		crate::Discovery,
	) {
		let driver = Driver::from_test_api(test_support::api(false));
		let discovery = driver.discover().unwrap();
		let deployment = DeploymentIdentity::from_discovery(&discovery, &discovery.devices[1]).unwrap();
		let cubin = b"\x7fELFfake-cubin".to_vec();
		let identity = ArtifactIdentity {
			sha256: Sha256::digest(&cubin).into(),
			target: ComputeCapability::new(5, 2),
			toolchain: ToolchainIdentity {
				zig_version: "0.17.0-dev".to_owned(),
				llvm_version: "21.1.8".to_owned(),
				ptx_isa_version: "7.8".to_owned(),
				ptxas_version: "11.4".to_owned(),
				cuda_toolkit_version: "11.4".to_owned(),
				cubin_format: "elf64-cuda".to_owned(),
			},
			minimum_driver: DriverVersion::new(11, 0).unwrap(),
			maximum_driver: Some(DriverVersion::new(11, 9).unwrap()),
			required_driver_symbols: BTreeSet::from([DriverSymbol::Init, DriverSymbol::CtxCreateV2]),
		};
		(cubin, identity, deployment, discovery)
	}

	#[test]
	fn exact_r470_compatible_identity_passes() {
		let (cubin, identity, deployment, _discovery) = identities();
		validate_artifact_compatibility(&cubin, &identity, &identity, &deployment)
			.expect("exact artifact should pass");
	}

	#[test]
	fn near_match_sm_is_rejected() {
		let (cubin, identity, mut deployment, _discovery) = identities();
		deployment.target = ComputeCapability::new(8, 6);
		let error = validate_artifact_compatibility(&cubin, &identity, &identity, &deployment).unwrap_err();
		assert!(error.issues.contains(&ArtifactIssue::DeviceTargetMismatch {
			artifact: ComputeCapability::new(5, 2),
			device: ComputeCapability::new(8, 6),
		}));
	}

	#[test]
	fn optional_loading_mode_is_not_assumed() {
		let (cubin, mut identity, deployment, _discovery) = identities();
		identity
			.required_driver_symbols
			.insert(DriverSymbol::ModuleGetLoadingMode);
		let error = validate_artifact_compatibility(&cubin, &identity, &identity, &deployment).unwrap_err();
		assert!(error.issues.contains(&ArtifactIssue::MissingDriverSymbol {
			symbol: DriverSymbol::ModuleGetLoadingMode,
		}));
	}

	#[test]
	fn content_hash_and_full_manifest_are_fail_closed() {
		let (mut cubin, identity, deployment, _discovery) = identities();
		cubin.push(0);
		let mut observed = identity.clone();
		observed.toolchain.ptxas_version = "13.3".to_owned();
		let error = validate_artifact_compatibility(&cubin, &identity, &observed, &deployment).unwrap_err();
		assert!(
			error.issues
				.iter()
				.any(|issue| matches!(issue, ArtifactIssue::DigestMismatch { .. }))
		);
		assert!(
			error.issues
				.contains(&ArtifactIssue::IdentityFieldMismatch {
					field: ArtifactField::PtxasVersion,
				})
		);
	}
}
