//! Production artifact materialization and candidate stabilization.
//!
//! This module owns the complete pre-finalization boundary. Deferred primitive
//! stages are lowered and built here in [`BuildPhase::Realize`]. The resulting
//! immutable images are then handed to a candidate-scoped driver which must
//! load, reserve, warm, and observe the exact [`PlannedCandidate`] before
//! `Finalize`. Neither the compiler nor the driver is retained in the
//! [`PreparedNativeSession`] that crosses into the run lifecycle.

use std::collections::BTreeSet;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use recipe_core::{
	ArtifactBuildRecipe, ArtifactId, ArtifactIdentity, CapacityLedger, Device, Digest, DiscoveredDevice,
	DiscoveryProfile, EXACT_USER_RESERVATION, Label, ReservationEntry, ReservationLedger, ReservationMechanism,
	TargetIdentity, ToolchainIdentity, Topology,
};
use recipe_cuda::{ComputeCapability, DriverSymbol, DriverVersion, ToolchainIdentity as CudaToolchainIdentity};
use recipe_kernel::{
	ArtifactBuilder, ArtifactDigest, BuildPhase, BuiltArtifact, KernelTarget, LoweringError, LoweringOptions,
};
use recipe_language::CalculationGraph;
use recipe_native_executor::{
	CandidateArtifact as ExecutorCandidateArtifact, CandidateFailure as ExecutorCandidateFailure,
	CandidateRealizationRequest as ExecutorCandidateRequest, CandidateSessionFactory, RuntimeArtifact,
	RuntimeArtifactKind, ValidatedCandidateFactory, ValidatedCandidateSession,
};
use recipe_planner::PlannedCandidate;
use recipe_probe::{MeasuredProfile, validate_profile};

use crate::{
	ArtifactProvider, CandidateRealizer, PreparationPolicy, RealizationObservation, RealizeFailure, RealizedCandidate,
};

const CUDA_BACKEND: &str = "nvidia-cuda-driver";
const CUDA_ABI: &str = "elf64-cubin";
const HSA_BACKEND: &str = "amd-rocr-hsa";
const HSA_ABI_PREFIX: &str = "elf64-amdgpu-code-object-v";

fn require<E>(condition: bool, error: E) -> Result<(), E> {
	match condition {
		true => Ok(()),
		false => Err(error),
	}
}

/// One immutable binary and its two matching identity domains.
///
/// `identity` is serialized into the finalized Recipe bundle. `runtime` is the
/// native executor's byte image and launch ABI. Construction validates their
/// digest, entry point, target, and format relationship before either value can
/// enter a catalog or candidate driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeArtifact {
	identity: ArtifactIdentity,
	runtime: RuntimeArtifact,
}

impl NativeArtifact {
	pub fn new(identity: ArtifactIdentity, runtime: RuntimeArtifact) -> Result<Self, NativePrepareError<Infallible>> {
		validate_native_artifact(&identity, &runtime)?;
		Ok(Self { identity, runtime })
	}

	#[must_use]
	pub const fn identity(&self) -> &ArtifactIdentity {
		&self.identity
	}

	#[must_use]
	pub const fn runtime(&self) -> &RuntimeArtifact {
		&self.runtime
	}
}

/// Validated precompiled inputs. It deliberately contains no builder, loader,
/// context, allocator, or driver handle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeArtifactCatalog {
	artifacts: Vec<NativeArtifact>,
	identities: Vec<ArtifactIdentity>,
}

impl NativeArtifactCatalog {
	pub fn new(mut artifacts: Vec<NativeArtifact>) -> Result<Self, NativePrepareError<Infallible>> {
		artifacts.sort_by_key(|artifact| artifact.identity.id);
		let mut ids = BTreeSet::new();
		for artifact in &artifacts {
			validate_native_artifact(&artifact.identity, &artifact.runtime)?;
			require(
				ids.insert(artifact.identity.id),
				NativePrepareError::InvalidCatalog(format!(
					"artifact {} appears more than once",
					artifact.identity.id
				)),
			)?;
		}
		let identities = artifacts
			.iter()
			.map(|artifact| artifact.identity.clone())
			.collect();
		Ok(Self {
			artifacts,
			identities,
		})
	}

	#[must_use]
	pub fn artifacts(&self) -> &[NativeArtifact] {
		&self.artifacts
	}

	#[must_use]
	pub fn identities(&self) -> &[ArtifactIdentity] {
		&self.identities
	}

	fn artifact(&self, id: ArtifactId) -> Option<&NativeArtifact> {
		let index = match self
			.artifacts
			.binary_search_by_key(&id, |artifact| artifact.identity.id)
		{
			Ok(index) => index,
			Err(search_error) => {
				debug_assert!(search_error <= self.artifacts.len());
				return None;
			}
		};
		self.artifacts.get(index)
	}
}

/// Production catalog source for fixed, already-built artifacts.
#[derive(Clone, Debug)]
pub struct NativeArtifactProvider {
	catalog: NativeArtifactCatalog,
}

impl NativeArtifactProvider {
	#[must_use]
	pub const fn new(catalog: NativeArtifactCatalog) -> Self {
		Self { catalog }
	}
}

impl ArtifactProvider for NativeArtifactProvider {
	type Catalog = NativeArtifactCatalog;
	type Error = NativePrepareError<Infallible>;

	fn resolve(
		&mut self,
		graph: &CalculationGraph,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<Self::Catalog, Self::Error> {
		graph.validate()
			.map_err(|error| NativePrepareError::InvalidCandidate(error.to_string()))?;
		discovery
			.validate(topology)
			.map_err(|errors| NativePrepareError::InvalidProfile(errors.to_string()))?;
		for identity in &self.catalog.identities {
			let target_exists = discovery.devices.iter().any(|device| {
				device.calculation
					.as_ref()
					.is_some_and(|calculation| calculation.target == identity.target)
			});
			require(
				target_exists,
				NativePrepareError::InvalidCatalog(format!(
					"artifact {} targets {:?}, which is absent from the measured discovery profile",
					identity.id, identity.target
				)),
			)?;
		}
		Ok(self.catalog.clone())
	}

	fn identities<'a>(&self, catalog: &'a Self::Catalog) -> &'a [ArtifactIdentity] {
		catalog.identities()
	}
}

/// CUDA compatibility data that cannot be inferred from a cubin alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaArtifactPolicy {
	pub toolchain: CudaToolchainIdentity,
	pub minimum_driver: DriverVersion,
	pub maximum_driver: Option<DriverVersion>,
	pub required_driver_symbols: BTreeSet<DriverSymbol>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeArtifactPolicy {
	Cuda(CudaArtifactPolicy),
	Hsa,
}

/// Exact mapping from one measured calculation target to its pinned Realize
/// toolchain and native runtime identity policy.
#[derive(Clone, Debug)]
pub struct TargetBuildSpec {
	pub target_identity: TargetIdentity,
	pub target: KernelTarget,
	pub toolchain_identity: ToolchainIdentity,
	pub builder: ArtifactBuilder,
	pub scratch_parent: PathBuf,
	pub runtime_policy: RuntimeArtifactPolicy,
}

impl TargetBuildSpec {
	fn validate(&self) -> Result<(), NativePrepareError<Infallible>> {
		require(
			!self.toolchain_identity.digest.is_zero(),
			NativePrepareError::InvalidConfiguration("artifact toolchain digest is zero".to_owned()),
		)?;
		match (&self.target, &self.runtime_policy) {
			(KernelTarget::Nvidia(target), RuntimeArtifactPolicy::Cuda(policy)) => {
				let expected = target.architecture();
				require(
					self.target_identity.backend.as_str() == CUDA_BACKEND
						&& self.target_identity.abi.as_str() == CUDA_ABI
						&& self.target_identity.architecture.as_str() == expected
						&& policy.toolchain.cubin_format == CUDA_ABI
						&& !policy
							.maximum_driver
							.is_some_and(|maximum| maximum < policy.minimum_driver),
					NativePrepareError::InvalidConfiguration(format!(
						"CUDA build target {:?} and runtime policy do not match measured identity {:?}",
						self.target, self.target_identity
					)),
				)?;
			}
			(KernelTarget::Amd(target), RuntimeArtifactPolicy::Hsa) => {
				let expected_abi = format!("{HSA_ABI_PREFIX}{}", target.code_object_version);
				let architecture = self.target_identity.architecture.as_str();
				let full_architecture = format!("amdgcn-amd-amdhsa--{}", target.target_id);
				let architecture_matches =
					architecture == target.target_id || architecture == full_architecture;
				require(
					self.target_identity.backend.as_str() == HSA_BACKEND
						&& self.target_identity.abi.as_str() == expected_abi
						&& architecture_matches,
					NativePrepareError::InvalidConfiguration(format!(
						"HSA build target {:?} does not match measured identity {:?}",
						self.target, self.target_identity
					)),
				)?;
			}
			(KernelTarget::Nvidia(_), RuntimeArtifactPolicy::Hsa)
			| (KernelTarget::Amd(_), RuntimeArtifactPolicy::Cuda(_)) => {
				return Err(NativePrepareError::InvalidConfiguration(
					"kernel target and runtime artifact policy name different backends".to_owned(),
				));
			}
		}
		Ok(())
	}
}

/// Pinned compiler set used only during candidate realization.
#[derive(Clone, Debug)]
pub struct DeferredArtifactCompiler {
	targets: Vec<TargetBuildSpec>,
}

impl DeferredArtifactCompiler {
	pub fn new(mut targets: Vec<TargetBuildSpec>) -> Result<Self, NativePrepareError<Infallible>> {
		targets.sort_by(|left, right| left.target_identity.cmp(&right.target_identity));
		let mut identities = BTreeSet::new();
		for target in &targets {
			target.validate()?;
			require(
				identities.insert(target.target_identity.clone()),
				NativePrepareError::InvalidConfiguration(format!(
					"target {:?} has more than one build specification",
					target.target_identity
				)),
			)?;
		}
		Ok(Self { targets })
	}

	fn materialize(
		&self,
		candidate: &PlannedCandidate,
		catalog: &NativeArtifactCatalog,
		discovery: &DiscoveryProfile,
	) -> Result<Vec<NativeArtifact>, NativePrepareError<Infallible>> {
		let mut artifacts = Vec::with_capacity(
			candidate
				.draft
				.artifacts
				.len()
				.checked_add(candidate.draft.artifact_builds.len())
				.ok_or_else(|| {
					NativePrepareError::InvalidCandidate("artifact count overflowed usize".to_owned())
				})?,
		);
		for identity in &candidate.draft.artifacts {
			let artifact = catalog.artifact(identity.id).ok_or_else(|| {
				NativePrepareError::InvalidCatalog(format!(
					"candidate selected artifact {}, but its runtime image is absent",
					identity.id
				))
			})?;
			require(
				artifact.identity == *identity,
				NativePrepareError::InvalidCatalog(format!(
					"catalog identity for artifact {} differs from the exact candidate identity",
					identity.id
				)),
			)?;
			artifacts.push(artifact.clone());
		}
		for build in &candidate.draft.artifact_builds {
			artifacts.push(self.build_deferred(candidate, discovery, build)?);
		}
		artifacts.sort_by_key(|artifact| artifact.identity.id);
		for pair in artifacts.windows(2) {
			require(
				pair[0].identity.id != pair[1].identity.id,
				NativePrepareError::InvalidCandidate(format!(
					"candidate resolves artifact {} more than once",
					pair[0].identity.id
				)),
			)?;
		}
		Ok(artifacts)
	}

	fn build_deferred(
		&self,
		candidate: &PlannedCandidate,
		discovery: &DiscoveryProfile,
		build: &ArtifactBuildRecipe,
	) -> Result<NativeArtifact, NativePrepareError<Infallible>> {
		let target_identity = candidate_target(candidate, discovery, build.artifact)?;
		let spec_index = match self
			.targets
			.binary_search_by(|candidate| candidate.target_identity.cmp(&target_identity))
		{
			Ok(index) => index,
			Err(search_error) => {
				debug_assert!(search_error <= self.targets.len());
				return Err(NativePrepareError::MissingBuildTarget(target_identity));
			}
		};
		let spec = self.targets.get(spec_index).ok_or_else(|| {
			NativePrepareError::InvalidConfiguration(
				"binary-search target index is outside the build specification table".to_owned(),
			)
		})?;
		let program = candidate
			.lowered_programs
			.iter()
			.find(|program| {
				program.source_kernel == build.source_kernel
					&& Digest::new(program.digest.bytes()) == build.provenance.program_digest
			})
			.ok_or_else(|| {
				NativePrepareError::InvalidCandidate(format!(
					"artifact {} has no exact lowered program",
					build.artifact
				))
			})?;
		let entry_symbol = format!("recipe_stage_{}", build.artifact.get());
		let lowered = recipe_kernel::lower_stage(
			program,
			build,
			&spec.target,
			&LoweringOptions {
				entry_symbol: entry_symbol.clone(),
				workgroup_lanes: build.dispatch.workgroup_lanes,
			},
		)
		.map_err(NativePrepareError::Lowering)?;
		let built = spec
			.builder
			.build(
				BuildPhase::Realize,
				&lowered,
				&spec.target,
				&spec.scratch_parent,
			)
			.map_err(NativePrepareError::Lowering)?;
		let (bytes, phase) = match &built {
			BuiltArtifact::Hsaco {
				bytes, provenance, ..
			}
			| BuiltArtifact::Cubin {
				bytes, provenance, ..
			} => (bytes.as_slice(), provenance.phase),
		};
		require(
			phase == BuildPhase::Realize,
			NativePrepareError::InvalidArtifact(format!(
				"artifact {} was not built in Realize",
				build.artifact
			)),
		)?;
		let digest = ArtifactDigest::of(bytes).bytes();
		let identity = ArtifactIdentity {
			id: build.artifact,
			digest: Digest::new(digest),
			format: spec.target_identity.abi.clone(),
			target: spec.target_identity.clone(),
			toolchain: spec.toolchain_identity.clone(),
			entry_symbol: Label::new(entry_symbol).map_err(|error| {
				NativePrepareError::InvalidArtifact(format!(
					"artifact {} entry symbol is invalid: {error}",
					build.artifact
				))
			})?,
			kernel_template: build.kernel_template,
			resources: build.resources.clone(),
			build: Some(build.provenance),
		};
		let runtime_kind = runtime_kind(spec, digest)?;
		let bytes: Arc<[u8]> = match built {
			BuiltArtifact::Hsaco { bytes, .. } | BuiltArtifact::Cubin { bytes, .. } => bytes.into(),
		};
		let runtime = RuntimeArtifact::new(build.artifact, bytes, lowered.abi, runtime_kind);
		NativeArtifact::new(identity, runtime).map_err(erase_never)
	}
}

fn runtime_kind(
	spec: &TargetBuildSpec,
	digest: [u8; 32],
) -> Result<RuntimeArtifactKind, NativePrepareError<Infallible>> {
	match (&spec.target, &spec.runtime_policy) {
		(KernelTarget::Nvidia(target), RuntimeArtifactPolicy::Cuda(policy)) => Ok(RuntimeArtifactKind::Cuda {
			identity: recipe_cuda::ArtifactIdentity {
				sha256: digest,
				target: ComputeCapability::new(u32::from(target.sm_major), u32::from(target.sm_minor)),
				toolchain: policy.toolchain.clone(),
				minimum_driver: policy.minimum_driver,
				maximum_driver: policy.maximum_driver,
				required_driver_symbols: policy.required_driver_symbols.clone(),
			},
		}),
		(KernelTarget::Amd(target), RuntimeArtifactPolicy::Hsa) => Ok(RuntimeArtifactKind::Hsa {
			target_id: spec.target_identity.architecture.as_str().to_owned(),
			code_object_version: target.code_object_version,
		}),
		(KernelTarget::Nvidia(_), RuntimeArtifactPolicy::Hsa)
		| (KernelTarget::Amd(_), RuntimeArtifactPolicy::Cuda(_)) => Err(NativePrepareError::InvalidConfiguration(
			"validated target/runtime policy pair changed before artifact construction".to_owned(),
		)),
	}
}

fn candidate_target(
	candidate: &PlannedCandidate,
	discovery: &DiscoveryProfile,
	artifact: ArtifactId,
) -> Result<TargetIdentity, NativePrepareError<Infallible>> {
	let mut targets = BTreeSet::new();
	for task in &candidate.draft.tasks {
		let recipe_core::TaskKind::Calculation(calculation) = &task.kind else {
			continue;
		};
		let calculation = match calculation.artifact == artifact {
			true => calculation,
			false => continue,
		};
		let capability = match discovery.device(calculation.device) {
			Some(device) => device.calculation.as_ref(),
			None => None,
		}
		.ok_or_else(|| {
			NativePrepareError::InvalidCandidate(format!(
				"artifact {artifact} is assigned to device {} without a calculation target",
				calculation.device
			))
		})?;
		targets.insert(capability.target.clone());
	}
	match targets.len() {
		1 => targets
			.into_iter()
			.next()
			.ok_or_else(|| NativePrepareError::InvalidCandidate("target set became empty".to_owned())),
		0 => Err(NativePrepareError::InvalidCandidate(format!(
			"deferred artifact {artifact} is unused by the candidate"
		))),
		2.. => Err(NativePrepareError::InvalidCandidate(format!(
			"artifact {artifact} is assigned to more than one native target"
		))),
	}
}

fn validate_native_artifact(
	identity: &ArtifactIdentity,
	runtime: &RuntimeArtifact,
) -> Result<(), NativePrepareError<Infallible>> {
	identity
		.validate()
		.map_err(|errors| NativePrepareError::InvalidArtifact(errors.to_string()))?;
	require(
		runtime.id() == identity.id,
		NativePrepareError::InvalidArtifact(format!(
			"runtime artifact {} differs from identity {}",
			runtime.id(),
			identity.id
		)),
	)?;
	let digest = ArtifactDigest::of(runtime.bytes()).bytes();
	require(
		digest == identity.digest.bytes(),
		NativePrepareError::InvalidArtifact(format!(
			"artifact {} byte digest differs from its identity",
			identity.id
		)),
	)?;
	require(
		runtime.abi().entry_symbol == identity.entry_symbol.as_str(),
		NativePrepareError::InvalidArtifact(format!(
			"artifact {} runtime entry differs from its identity",
			identity.id
		)),
	)?;
	require(
		identity.format == identity.target.abi,
		NativePrepareError::InvalidArtifact(format!(
			"artifact {} format differs from its target ABI",
			identity.id
		)),
	)?;
	match runtime.kind() {
		RuntimeArtifactKind::Cuda { identity: cuda } => {
			require(
				identity.target.backend.as_str() == CUDA_BACKEND
					&& identity.target.abi.as_str() == CUDA_ABI
					&& identity.target.architecture.as_str() == cuda.target.to_string()
					&& cuda.sha256 == digest,
				NativePrepareError::InvalidArtifact(format!(
					"artifact {} has inconsistent CUDA identities",
					identity.id
				)),
			)?;
		}
		RuntimeArtifactKind::Hsa {
			target_id,
			code_object_version,
		} => {
			let abi = format!("{HSA_ABI_PREFIX}{code_object_version}");
			require(
				identity.target.backend.as_str() == HSA_BACKEND
					&& identity.target.abi.as_str() == abi
					&& identity.target.architecture.as_str() == target_id,
				NativePrepareError::InvalidArtifact(format!(
					"artifact {} has inconsistent HSA identities",
					identity.id
				)),
			)?;
		}
	}
	Ok(())
}

/// Candidate-scoped input to a native/host realization implementation.
#[derive(Clone, Copy, Debug)]
pub struct CandidateRealizationRequest<'a> {
	pub topology: &'a Topology,
	pub discovery: &'a DiscoveryProfile,
	pub candidate: &'a PlannedCandidate,
	pub artifacts: &'a [NativeArtifact],
	pub reservations: &'a ReservationLedger,
}

/// Failure classification required from a production candidate driver.
///
/// A rejection returned before a `Session` exists guarantees that every
/// partially created native object has already been destroyed.
#[derive(Debug)]
pub enum CandidateDriverFailure<E> {
	CandidateRejected {
		detail: String,
	},
	Fatal(E),
	/// The backend has only a post-Finalize binding path and therefore cannot
	/// satisfy the ahead-of-run fixed-point contract.
	PreFinalRealizationUnavailable {
		backend: String,
	},
}

/// Dependency-neutral pre-final realization interface.
///
/// Implementations own native contexts, loaded modules, queues, completion
/// objects, exact one-GB reservations, staging, and warmup resources in
/// `Session`. `warm_maximum_concurrency` must execute the candidate's maximum
/// concurrent schedule without changing any Draft choice. A successful
/// session must be reusable by the finalized runtime; silently loading or
/// allocating the same resources again after Finalize is prohibited.
pub trait CandidateDriver {
	type Session: fmt::Debug;
	type Error: Error + Send + Sync + 'static;

	fn reservation_mechanism(
		&self,
		device: &Device,
		discovered: &DiscoveredDevice,
	) -> Result<ReservationMechanism, Self::Error>;

	fn realize_candidate(
		&mut self,
		request: CandidateRealizationRequest<'_>,
	) -> Result<Self::Session, CandidateDriverFailure<Self::Error>>;

	fn warm_maximum_concurrency(
		&mut self,
		session: &mut Self::Session,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateDriverFailure<Self::Error>>;

	fn capacity_snapshot(
		&mut self,
		session: &mut Self::Session,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateDriverFailure<Self::Error>>;

	fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error>;
}

/// Concrete bridge to the native executor's dependency-neutral, validated
/// pre-final candidate lifecycle.
///
/// The native factory remains the sole owner of loading, allocation, warmup,
/// observation, and deterministic destruction. The adapter supplies a held
/// allocation on every storage device and converts the already validated
/// Recipe/native artifact pairs without exposing a second realization path.
#[derive(Debug)]
pub struct NativeExecutorDriver<F> {
	factory: ValidatedCandidateFactory<F>,
}

impl<F> NativeExecutorDriver<F> {
	#[must_use]
	pub const fn new(factory: F) -> Self {
		Self {
			factory: ValidatedCandidateFactory::new(factory),
		}
	}

	#[must_use]
	pub const fn factory(&self) -> &F {
		self.factory.inner()
	}
}

impl<F> CandidateDriver for NativeExecutorDriver<F>
where
	F: CandidateSessionFactory,
{
	type Session = ValidatedCandidateSession<F::Session>;
	type Error = F::Error;

	fn reservation_mechanism(
		&self,
		device: &Device,
		discovered: &DiscoveredDevice,
	) -> Result<ReservationMechanism, Self::Error> {
		debug_assert_eq!(device.id, discovered.device);
		Ok(ReservationMechanism::HeldAllocation)
	}

	fn realize_candidate(
		&mut self,
		request: CandidateRealizationRequest<'_>,
	) -> Result<Self::Session, CandidateDriverFailure<Self::Error>> {
		let artifacts = request
			.artifacts
			.iter()
			.map(|artifact| ExecutorCandidateArtifact::new(artifact.identity.clone(), artifact.runtime.clone()))
			.collect::<Vec<_>>();
		self.factory
			.realize_candidate(ExecutorCandidateRequest {
				topology: request.topology,
				discovery: request.discovery,
				candidate: request.candidate,
				artifacts: &artifacts,
				reservations: request.reservations,
			})
			.map_err(map_executor_failure)
	}

	fn warm_maximum_concurrency(
		&mut self,
		session: &mut Self::Session,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateDriverFailure<Self::Error>> {
		self.factory
			.warm_maximum_concurrency(session, candidate, pass)
			.map_err(map_executor_failure)
	}

	fn capacity_snapshot(
		&mut self,
		session: &mut Self::Session,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateDriverFailure<Self::Error>> {
		self.factory
			.capacity_snapshot(session, topology, discovery)
			.map_err(map_executor_failure)
	}

	fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error> {
		self.factory.destroy_candidate(session)
	}
}

fn map_executor_failure<E>(failure: ExecutorCandidateFailure<E>) -> CandidateDriverFailure<E> {
	match failure {
		ExecutorCandidateFailure::CandidateRejected { detail } => {
			CandidateDriverFailure::CandidateRejected { detail }
		}
		ExecutorCandidateFailure::Fatal(error) => CandidateDriverFailure::Fatal(error),
		ExecutorCandidateFailure::PreFinalRealizationUnavailable { detail } => {
			CandidateDriverFailure::PreFinalRealizationUnavailable {
				backend: detail.to_owned(),
			}
		}
	}
}

/// Successful driver session plus the immutable images that produced its
/// loaded modules. No compiler, loader, allocator, or mutable driver is
/// retained here.
#[derive(Debug)]
pub struct PreparedNativeSession<S> {
	driver_session: S,
	artifacts: Vec<NativeArtifact>,
}

impl<S> PreparedNativeSession<S> {
	#[must_use]
	pub const fn driver_session(&self) -> &S {
		&self.driver_session
	}

	#[must_use]
	pub fn artifacts(&self) -> &[NativeArtifact] {
		&self.artifacts
	}

	#[must_use]
	pub fn runtime_artifacts(&self) -> impl ExactSizeIterator<Item = &RuntimeArtifact> {
		self.artifacts.iter().map(NativeArtifact::runtime)
	}

	/// Consumes the preparation wrapper without destroying its pre-final
	/// physical session. The returned session is the same value realized and
	/// warmed by the candidate driver; callers may hand it to the finalized
	/// local backend exactly once.
	#[must_use]
	pub fn into_parts(self) -> (S, Vec<NativeArtifact>) {
		(self.driver_session, self.artifacts)
	}
}

/// Concrete `CandidateRealizer` which compiles every deferred stage, invokes
/// one candidate-scoped driver, runs the bounded stabilization trace, and
/// retains only the successful opaque session and immutable images.
#[derive(Debug)]
pub struct NativeCandidateRealizer<D> {
	topology: Topology,
	discovery: DiscoveryProfile,
	compiler: DeferredArtifactCompiler,
	driver: D,
}

impl<D> NativeCandidateRealizer<D>
where
	D: CandidateDriver,
{
	pub fn new(
		profile: &MeasuredProfile,
		compiler: DeferredArtifactCompiler,
		driver: D,
	) -> Result<Self, NativePrepareError<D::Error>> {
		validate_profile(profile).map_err(|errors| NativePrepareError::InvalidProfile(errors.to_string()))?;
		for device in &profile.discovery.devices {
			let Some(calculation) = &device.calculation else {
				continue;
			};
			require(
				compiler
					.targets
					.binary_search_by(|target| target.target_identity.cmp(&calculation.target))
					.is_ok(),
				NativePrepareError::MissingBuildTarget(calculation.target.clone()),
			)?;
		}
		Ok(Self {
			topology: profile.topology.clone(),
			discovery: profile.discovery.clone(),
			compiler,
			driver,
		})
	}

	#[must_use]
	pub const fn driver(&self) -> &D {
		&self.driver
	}
}

impl<D> CandidateRealizer<NativeArtifactCatalog> for NativeCandidateRealizer<D>
where
	D: CandidateDriver,
{
	type Session = PreparedNativeSession<D::Session>;
	type Error = NativePrepareError<D::Error>;

	fn reservation_plan(
		&self,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<ReservationLedger, Self::Error> {
		self.require_profile(topology, discovery)?;
		exact_reservation_plan(&self.driver, topology, discovery)
	}

	fn realize(
		&mut self,
		candidate: &PlannedCandidate,
		catalog: &NativeArtifactCatalog,
		reservations: &ReservationLedger,
		policy: PreparationPolicy,
	) -> Result<RealizedCandidate<Self::Session>, RealizeFailure<Self::Error>> {
		require(
			candidate.draft.topology == self.topology.identity
				&& candidate.draft.discovery == self.discovery.identity,
			RealizeFailure::Fatal(NativePrepareError::ProfileMismatch),
		)?;
		reservations.validate(&self.topology).map_err(|errors| {
			RealizeFailure::Fatal(NativePrepareError::InvalidReservation(errors.to_string()))
		})?;
		let artifacts = self
			.compiler
			.materialize(candidate, catalog, &self.discovery)
			.map_err(|error| RealizeFailure::Fatal(error.erase_driver()))?;
		let request = CandidateRealizationRequest {
			topology: &self.topology,
			discovery: &self.discovery,
			candidate,
			artifacts: &artifacts,
			reservations,
		};
		let mut session = self
			.driver
			.realize_candidate(request)
			.map_err(map_driver_failure)?;
		let mut snapshots = Vec::with_capacity(usize::try_from(policy.stabilization_passes.get()).map_err(
			|conversion_error| {
				RealizeFailure::Fatal(NativePrepareError::InvalidConfiguration(format!(
					"stabilization pass count does not fit usize: {conversion_error}"
				)))
			},
		)?);
		for pass in 1..=policy.stabilization_passes.get() {
			match self
				.driver
				.warm_maximum_concurrency(&mut session, candidate, pass)
			{
				Ok(completed) => completed,
				Err(failure) => return self.reject_after_session(session, failure),
			}
			match self
				.driver
				.capacity_snapshot(&mut session, &self.topology, &self.discovery)
			{
				Ok(snapshot) => snapshots.push(snapshot),
				Err(failure) => return self.reject_after_session(session, failure),
			}
		}
		let identities = artifacts
			.iter()
			.map(|artifact| artifact.identity.clone())
			.collect();
		Ok(RealizedCandidate {
			session: PreparedNativeSession {
				driver_session: session,
				artifacts,
			},
			observation: RealizationObservation {
				artifacts: identities,
				resources: candidate.draft.resources.clone(),
				reservations: reservations.clone(),
				capacity_snapshots: snapshots,
			},
		})
	}

	fn destroy(&mut self, session: Self::Session) -> Result<(), Self::Error> {
		let PreparedNativeSession {
			driver_session,
			artifacts: _,
		} = session;
		self.driver
			.destroy_candidate(driver_session)
			.map_err(NativePrepareError::Driver)
	}
}

fn exact_reservation_plan<D>(
	driver: &D,
	topology: &Topology,
	discovery: &DiscoveryProfile,
) -> Result<ReservationLedger, NativePrepareError<D::Error>>
where
	D: CandidateDriver,
{
	let mut entries = Vec::with_capacity(topology.devices.len());
	for device in &topology.devices {
		let discovered = discovery.device(device.id).ok_or_else(|| {
			NativePrepareError::InvalidProfile(format!(
				"device {} is absent from measured discovery",
				device.id
			))
		})?;
		let mechanism = driver
			.reservation_mechanism(device, discovered)
			.map_err(NativePrepareError::Driver)?;
		entries.push(ReservationEntry {
			device: device.id,
			name: Label::new(format!("recipe-user-{}", device.id.get())).map_err(|error| {
				NativePrepareError::InvalidConfiguration(format!(
					"reservation name for device {} is invalid: {error}",
					device.id
				))
			})?,
			bytes: EXACT_USER_RESERVATION,
			mechanism,
		});
	}
	let reservations = ReservationLedger { entries };
	reservations
		.validate(topology)
		.map_err(|errors| NativePrepareError::InvalidReservation(errors.to_string()))?;
	Ok(reservations)
}

impl<D> NativeCandidateRealizer<D>
where
	D: CandidateDriver,
{
	fn require_profile(
		&self,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<(), NativePrepareError<D::Error>> {
		require(
			topology == &self.topology && discovery == &self.discovery,
			NativePrepareError::ProfileMismatch,
		)
	}

	fn reject_after_session<T>(
		&mut self,
		session: D::Session,
		failure: CandidateDriverFailure<D::Error>,
	) -> Result<T, RealizeFailure<NativePrepareError<D::Error>>> {
		let failure_detail = driver_failure_detail(&failure);
		match self.driver.destroy_candidate(session) {
			Ok(()) => Err(map_driver_failure(failure)),
			Err(error) => Err(RealizeFailure::Fatal(NativePrepareError::Teardown {
				operation: failure_detail,
				source: error,
			})),
		}
	}
}

fn map_driver_failure<E>(failure: CandidateDriverFailure<E>) -> RealizeFailure<NativePrepareError<E>> {
	match failure {
		CandidateDriverFailure::CandidateRejected { detail } => RealizeFailure::CandidateRejected { detail },
		CandidateDriverFailure::Fatal(error) => RealizeFailure::Fatal(NativePrepareError::Driver(error)),
		CandidateDriverFailure::PreFinalRealizationUnavailable { backend } => {
			RealizeFailure::Fatal(NativePrepareError::PreFinalRealizationUnavailable { backend })
		}
	}
}

fn driver_failure_detail<E>(failure: &CandidateDriverFailure<E>) -> String {
	match failure {
		CandidateDriverFailure::CandidateRejected { detail } => {
			format!("candidate rejection with a live session: {detail}")
		}
		CandidateDriverFailure::Fatal(_) => "fatal driver failure with a live session".to_owned(),
		CandidateDriverFailure::PreFinalRealizationUnavailable { backend } => {
			format!("{backend} lacks pre-final candidate realization with a live session")
		}
	}
}

#[derive(Debug)]
pub enum NativePrepareError<E> {
	InvalidConfiguration(String),
	InvalidProfile(String),
	ProfileMismatch,
	InvalidCatalog(String),
	InvalidCandidate(String),
	InvalidArtifact(String),
	InvalidReservation(String),
	MissingBuildTarget(TargetIdentity),
	Lowering(LoweringError),
	Driver(E),
	Teardown { operation: String, source: E },
	PreFinalRealizationUnavailable { backend: String },
}

impl NativePrepareError<Infallible> {
	fn erase_driver<F>(self) -> NativePrepareError<F> {
		match self {
			Self::InvalidConfiguration(detail) => NativePrepareError::InvalidConfiguration(detail),
			Self::InvalidProfile(detail) => NativePrepareError::InvalidProfile(detail),
			Self::ProfileMismatch => NativePrepareError::ProfileMismatch,
			Self::InvalidCatalog(detail) => NativePrepareError::InvalidCatalog(detail),
			Self::InvalidCandidate(detail) => NativePrepareError::InvalidCandidate(detail),
			Self::InvalidArtifact(detail) => NativePrepareError::InvalidArtifact(detail),
			Self::InvalidReservation(detail) => NativePrepareError::InvalidReservation(detail),
			Self::MissingBuildTarget(target) => NativePrepareError::MissingBuildTarget(target),
			Self::Lowering(error) => NativePrepareError::Lowering(error),
			Self::Driver(error) => match error {},
			Self::Teardown { source, .. } => match source {},
			Self::PreFinalRealizationUnavailable { backend } => {
				NativePrepareError::PreFinalRealizationUnavailable { backend }
			}
		}
	}
}

impl<E: fmt::Display> fmt::Display for NativePrepareError<E> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidConfiguration(detail) => {
				write!(
					formatter,
					"invalid production preparation configuration: {detail}"
				)
			}
			Self::InvalidProfile(detail) => write!(formatter, "invalid measured profile: {detail}"),
			Self::ProfileMismatch => {
				formatter.write_str("preparer inputs differ from the validated measured profile")
			}
			Self::InvalidCatalog(detail) => write!(formatter, "invalid native artifact catalog: {detail}"),
			Self::InvalidCandidate(detail) => write!(formatter, "invalid planned candidate: {detail}"),
			Self::InvalidArtifact(detail) => write!(formatter, "invalid realized artifact: {detail}"),
			Self::InvalidReservation(detail) => write!(formatter, "invalid reservation plan: {detail}"),
			Self::MissingBuildTarget(target) => {
				write!(
					formatter,
					"no pinned artifact builder exists for target {target:?}"
				)
			}
			Self::Lowering(error) => write!(formatter, "artifact realization failed: {error}"),
			Self::Driver(error) => write!(formatter, "candidate driver failed: {error}"),
			Self::Teardown { operation, source } => {
				write!(
					formatter,
					"candidate teardown failed after {operation}: {source}"
				)
			}
			Self::PreFinalRealizationUnavailable { backend } => {
				write!(
					formatter,
					"{backend} has no candidate-scoped pre-Finalize realization path"
				)
			}
		}
	}
}

impl<E> Error for NativePrepareError<E>
where
	E: Error + 'static,
{
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Lowering(error) => Some(error),
			Self::Driver(error) => Some(error),
			Self::Teardown { source, .. } => Some(source),
			Self::InvalidConfiguration(_)
			| Self::InvalidProfile(_)
			| Self::ProfileMismatch
			| Self::InvalidCatalog(_)
			| Self::InvalidCandidate(_)
			| Self::InvalidArtifact(_)
			| Self::InvalidReservation(_)
			| Self::MissingBuildTarget(_)
			| Self::PreFinalRealizationUnavailable { .. } => None,
		}
	}
}

fn erase_never<E>(error: NativePrepareError<Infallible>) -> NativePrepareError<E> {
	error.erase_driver()
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;
	use std::sync::Arc;

	use recipe_core::{
		ByteCount, BytesPerSecond, DeviceId, DeviceKind, DiscoveryIdentity, FlopsPerSecond, KernelResourceBounds,
		KernelTemplateId, Machine, MachineId, Property, PropertyProvenance, TopologyIdentity, TransferCapability,
		TransferLaneCount,
	};
	use recipe_kernel::{KernelAbi, KernelArgument, OfflineToolchain, PinnedTool};

	use super::*;

	fn label(value: &str) -> Label {
		match Label::new(value) {
			Ok(label) => label,
			Err(error) => panic!("test label is invalid: {error}"),
		}
	}

	fn measured<T>(value: T) -> Property<T> {
		Property::new(value, PropertyProvenance::Measured)
	}

	fn topology_and_discovery() -> (Topology, DiscoveryProfile) {
		let machine = MachineId::new(1);
		let topology_id = TopologyIdentity::new(Digest::new([1; 32]));
		let devices = [
			(DeviceId::new(1), DeviceKind::GpuMemory),
			(DeviceId::new(2), DeviceKind::Ram),
			(DeviceId::new(3), DeviceKind::Disk),
		]
		.into_iter()
		.map(|(id, kind)| Device {
			id,
			machine,
			kind,
			capacity: measured(ByteCount::new(4_000_000_000)),
			transfer_rate: measured(
				BytesPerSecond::new(1_000_000_000)
					.unwrap_or_else(|error| panic!("test rate is invalid: {error}")),
			),
			calculation_rate: match kind {
				DeviceKind::GpuMemory => Some(measured(
					FlopsPerSecond::new(1_000_000_000)
						.unwrap_or_else(|error| panic!("test FLOP rate is invalid: {error}")),
				)),
				DeviceKind::Ram | DeviceKind::Disk => None,
			},
		})
		.collect::<Vec<_>>();
		let topology = Topology {
			identity: topology_id,
			machines: vec![Machine {
				id: machine,
				name: label("machine"),
			}],
			nodes: Vec::new(),
			devices,
			links: Vec::new(),
		};
		let discovery = DiscoveryProfile {
			identity: DiscoveryIdentity::new(Digest::new([2; 32])),
			topology: topology_id,
			devices: topology
				.devices
				.iter()
				.map(|device| DiscoveredDevice {
					device: device.id,
					available: true,
					total_capacity: measured(ByteCount::new(4_000_000_000)),
					transfer: TransferCapability {
						rate: measured(
							BytesPerSecond::new(1_000_000_000)
								.unwrap_or_else(|error| panic!("test rate is invalid: {error}")),
						),
						maximum_inflight_transfers: measured(
							TransferLaneCount::new(1)
								.unwrap_or_else(|error| panic!("test lane count is invalid: {error}")),
						),
						asynchronous_submission: true,
						overlaps_calculation: true,
					},
					calculation: None,
				})
				.collect(),
			links: Vec::new(),
		};
		(topology, discovery)
	}

	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	struct TestSession;

	#[derive(Debug, Default)]
	struct TestDriver {
		destroyed: usize,
	}

	impl CandidateDriver for TestDriver {
		type Session = TestSession;
		type Error = Infallible;

		fn reservation_mechanism(
			&self,
			device: &Device,
			discovered: &DiscoveredDevice,
		) -> Result<ReservationMechanism, Self::Error> {
			debug_assert_eq!(device.id, discovered.device);
			Ok(ReservationMechanism::HeldAllocation)
		}

		fn realize_candidate(
			&mut self,
			request: CandidateRealizationRequest<'_>,
		) -> Result<Self::Session, CandidateDriverFailure<Self::Error>> {
			debug_assert_eq!(request.topology.identity, request.candidate.draft.topology);
			Err(CandidateDriverFailure::PreFinalRealizationUnavailable {
				backend: "test".to_owned(),
			})
		}

		fn warm_maximum_concurrency(
			&mut self,
			session: &mut Self::Session,
			candidate: &PlannedCandidate,
			pass: u32,
		) -> Result<(), CandidateDriverFailure<Self::Error>> {
			debug_assert_eq!(*session, TestSession);
			debug_assert_ne!(candidate.draft.candidate.digest(), Digest::ZERO);
			debug_assert_ne!(pass, 0);
			Err(CandidateDriverFailure::CandidateRejected {
				detail: "test".to_owned(),
			})
		}

		fn capacity_snapshot(
			&mut self,
			session: &mut Self::Session,
			topology: &Topology,
			discovery: &DiscoveryProfile,
		) -> Result<CapacityLedger, CandidateDriverFailure<Self::Error>> {
			debug_assert_eq!(*session, TestSession);
			debug_assert_eq!(topology.identity, discovery.topology);
			Err(CandidateDriverFailure::CandidateRejected {
				detail: "test".to_owned(),
			})
		}

		fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error> {
			debug_assert_eq!(session, TestSession);
			self.destroyed += 1;
			Ok(())
		}
	}

	#[test]
	fn exact_reservation_plan_covers_every_storage_device() {
		let (topology, discovery) = topology_and_discovery();
		let reservations = exact_reservation_plan(&TestDriver::default(), &topology, &discovery)
			.unwrap_or_else(|error| panic!("reservation plan failed: {error}"));
		assert_eq!(reservations.entries.len(), 3);
		for device in &topology.devices {
			let entry = reservations
				.entry(device.id)
				.unwrap_or_else(|| panic!("device {} has no reservation", device.id));
			assert_eq!(entry.bytes, EXACT_USER_RESERVATION);
			assert_eq!(entry.mechanism, ReservationMechanism::HeldAllocation);
			assert_eq!(
				entry.name.as_str(),
				format!("recipe-user-{}", device.id.get())
			);
		}
	}

	fn native_artifact() -> NativeArtifact {
		let bytes: Arc<[u8]> = Arc::from(&b"recipe-cubin"[..]);
		let digest = ArtifactDigest::of(&bytes).bytes();
		let target = TargetIdentity {
			backend: label(CUDA_BACKEND),
			architecture: label("sm_52"),
			abi: label(CUDA_ABI),
		};
		let runtime = RuntimeArtifact::new(
			ArtifactId::new(7),
			bytes,
			KernelAbi {
				entry_symbol: "recipe_stage_7".to_owned(),
				arguments: vec![KernelArgument::ElementCount],
				argument_bytes: 8,
				argument_alignment: 8,
				elements: recipe_core::ElementCount::new(1)
					.unwrap_or_else(|error| panic!("test element count is invalid: {error}")),
				workgroup_lanes: 1,
			},
			RuntimeArtifactKind::Cuda {
				identity: recipe_cuda::ArtifactIdentity {
					sha256: digest,
					target: ComputeCapability::new(5, 2),
					toolchain: CudaToolchainIdentity {
						zig_version: "0.17".to_owned(),
						llvm_version: "22".to_owned(),
						ptx_isa_version: "6.4".to_owned(),
						ptxas_version: "11.4".to_owned(),
						cuda_toolkit_version: "11.4".to_owned(),
						cubin_format: CUDA_ABI.to_owned(),
					},
					minimum_driver: DriverVersion::new(11, 0)
						.unwrap_or_else(|error| panic!("test driver version is invalid: {error}")),
					maximum_driver: None,
					required_driver_symbols: BTreeSet::new(),
				},
			},
		);
		NativeArtifact::new(
			ArtifactIdentity {
				id: ArtifactId::new(7),
				digest: Digest::new(digest),
				format: target.abi.clone(),
				target,
				toolchain: ToolchainIdentity {
					name: label("recipe-owned"),
					version: label("1"),
					digest: Digest::new([3; 32]),
				},
				entry_symbol: label("recipe_stage_7"),
				kernel_template: KernelTemplateId::new(7),
				resources: KernelResourceBounds {
					private_bytes_per_lane: ByteCount::ZERO,
					shared_bytes_per_workgroup: ByteCount::ZERO,
					scratch_bytes_per_dispatch: ByteCount::ZERO,
					maximum_workgroup_lanes: 1,
				},
				build: None,
			},
			runtime,
		)
		.unwrap_or_else(|error| panic!("native artifact fixture is invalid: {error}"))
	}

	#[test]
	fn catalog_rejects_duplicate_native_identity() {
		let artifact = native_artifact();
		let error = NativeArtifactCatalog::new(vec![artifact.clone(), artifact])
			.expect_err("duplicate native artifacts must fail");
		assert!(matches!(error, NativePrepareError::InvalidCatalog(_)));
	}

	#[test]
	fn target_build_spec_rejects_runtime_backend_mismatch() {
		let executable = std::env::current_exe()
			.unwrap_or_else(|error| panic!("test executable path is unavailable: {error}"));
		let tool = PinnedTool::inspect(executable).unwrap_or_else(|error| panic!("pin test executable: {error}"));
		let builder = ArtifactBuilder::new(OfflineToolchain {
			verifier: tool.clone(),
			llvm_codegen: tool.clone(),
			elf_linker: Some(tool),
			ptx_assembler: None,
		})
		.unwrap_or_else(|error| panic!("construct test artifact builder: {error}"));
		let error = DeferredArtifactCompiler::new(vec![TargetBuildSpec {
			target_identity: TargetIdentity {
				backend: label(HSA_BACKEND),
				architecture: label("amdgcn-amd-amdhsa--gfx1101"),
				abi: label("elf64-amdgpu-code-object-v5"),
			},
			target: KernelTarget::Amd(recipe_kernel::AmdTarget {
				target_id: "gfx1101".to_owned(),
				code_object_version: 6,
			}),
			toolchain_identity: ToolchainIdentity {
				name: label("recipe-owned"),
				version: label("1"),
				digest: Digest::new([4; 32]),
			},
			builder,
			scratch_parent: std::env::temp_dir(),
			runtime_policy: RuntimeArtifactPolicy::Hsa,
		}])
		.expect_err("mismatched HSA code-object version must fail");
		assert!(matches!(error, NativePrepareError::InvalidConfiguration(_)));
	}

	#[test]
	fn live_session_failure_is_destroyed_before_rejection() {
		let (topology, discovery) = topology_and_discovery();
		let mut realizer = NativeCandidateRealizer {
			topology,
			discovery,
			compiler: DeferredArtifactCompiler {
				targets: Vec::new(),
			},
			driver: TestDriver::default(),
		};
		let result: Result<(), RealizeFailure<NativePrepareError<Infallible>>> = realizer.reject_after_session(
			TestSession,
			CandidateDriverFailure::CandidateRejected {
				detail: "warm trace rejected".to_owned(),
			},
		);
		assert!(matches!(
			result,
			Err(RealizeFailure::CandidateRejected { .. })
		));
		assert_eq!(realizer.driver.destroyed, 1);
	}

	#[test]
	fn executor_unavailable_factory_maps_to_typed_pre_final_failure() {
		let failure: CandidateDriverFailure<recipe_native_executor::UnavailableCandidateError> =
			map_executor_failure(ExecutorCandidateFailure::PreFinalRealizationUnavailable {
				detail: "no physical candidate factory",
			});
		assert!(matches!(
			failure,
			CandidateDriverFailure::PreFinalRealizationUnavailable { .. }
		));
	}
}
