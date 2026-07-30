#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! The one public fixed-point boundary between a measured profile and an
//! immutable, fully realized Recipe execution bundle.
//!
//! Artifact selection, finite Draft enumeration, backend realization,
//! stabilization, capacity rejection, and Finalize are deliberately owned by
//! this crate. A caller cannot finalize an optimistic Draft and later mutate it
//! to match whatever a backend happened to allocate.

use std::error::Error;
use std::fmt;
use std::num::NonZeroU32;

use recipe_core::{
	ArtifactIdentity, BundleIdentity, ByteCount, CandidateIdentity, CapacityLedger, CapacityLedgerEntry, DeviceId,
	DiscoveryProfile, DraftPlan, EXACT_USER_RESERVATION, FinalizedBundle, LoopIterations, LoopTaskDomain, Property,
	PropertyProvenance, RealizationIdentity, RealizationProfile, ReservationLedger, ResourceManifest, TaskId,
	Topology, ValueId,
};
use recipe_language::CalculationGraph;
#[cfg(test)]
use recipe_planner::plan_candidates;
use recipe_planner::{
	PlannedCandidate, PlannedExternalOutput, PlannedProgramCandidate, ProgramPlannerSearch, plan_program_candidates,
};
use recipe_probe::{MeasuredProfile, validate_profile};
use recipe_program::StaticCalculationProgram;
use recipe_scheduler::pack_arenas;
use sha2::{Digest as _, Sha256};

mod production;

pub use production::{
	CandidateDriver, CandidateDriverFailure, CandidateRealizationRequest, CudaArtifactPolicy,
	DeferredArtifactCompiler, NativeArtifact, NativeArtifactCatalog, NativeArtifactProvider, NativeCandidateRealizer,
	NativeExecutorDriver, NativePrepareError, PreparedNativeSession, RuntimeArtifactPolicy, TargetBuildSpec,
};

/// Default bounded stabilization contract. The backend performs three complete
/// maximum-concurrency traces and the last two capacity snapshots must match.
pub const DEFAULT_STABILIZATION_PASSES: NonZeroU32 = NonZeroU32::new(3).expect("three is nonzero");
pub const DEFAULT_STABLE_TAIL: NonZeroU32 = NonZeroU32::new(2).expect("two is nonzero");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparationPolicy {
	pub stabilization_passes: NonZeroU32,
	pub stable_tail: NonZeroU32,
}

impl Default for PreparationPolicy {
	fn default() -> Self {
		Self {
			stabilization_passes: DEFAULT_STABILIZATION_PASSES,
			stable_tail: DEFAULT_STABLE_TAIL,
		}
	}
}

impl PreparationPolicy {
	fn validate(self) -> Result<(), PrepareError> {
		if self.stable_tail > self.stabilization_passes {
			return Err(PrepareError::new(
				PrepareErrorKind::InvalidPolicy,
				"stable-tail length exceeds the bounded stabilization pass count",
			));
		}
		Ok(())
	}
}

/// Recipe-owned artifact source used inside public preparation.
///
/// The provider may read an exact precompiled catalog or perform compilation
/// in an allowed offline/Realize phase. It must return final binary identities;
/// no compiler entry point is reachable after this call returns.
pub trait ArtifactProvider {
	type Catalog: fmt::Debug;
	type Error: Error + Send + Sync + 'static;

	fn resolve(
		&mut self,
		graph: &CalculationGraph,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<Self::Catalog, Self::Error>;

	fn identities<'a>(&self, catalog: &'a Self::Catalog) -> &'a [ArtifactIdentity];
}

/// Candidate-local realization failure.
///
/// `CandidateRejected` is retryable and means every partially created native
/// object has already been destroyed. `Fatal` aborts the complete preparation.
#[derive(Debug)]
pub enum RealizeFailure<E> {
	CandidateRejected { detail: String },
	Fatal(E),
}

/// Capacity and identity evidence emitted after the exact Draft was loaded,
/// warmed at its maximum concurrent schedule, and stabilized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealizationObservation {
	pub artifacts: Vec<ArtifactIdentity>,
	pub resources: ResourceManifest,
	pub reservations: ReservationLedger,
	/// One post-trace snapshot per bounded stabilization pass.
	pub capacity_snapshots: Vec<CapacityLedger>,
}

/// Native objects that keep one successful realization alive.
#[derive(Debug)]
pub struct RealizedCandidate<S> {
	pub session: S,
	pub observation: RealizationObservation,
}

/// Backend realization boundary.
///
/// `Session` must own every reservation, context, module, queue, signal/event,
/// staging allocation, warmed graph executable, and other object required by
/// the candidate. Dropping it must be safe. `destroy` is used for explicit
/// deterministic rejection before the next candidate is attempted.
pub trait CandidateRealizer<C> {
	type Session: fmt::Debug;
	type Error: Error + Send + Sync + 'static;

	/// Select an implementation mechanism for the mandatory exact reservation
	/// on every automatically discovered storage device.
	fn reservation_plan(
		&self,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<ReservationLedger, Self::Error>;

	fn realize(
		&mut self,
		candidate: &PlannedCandidate,
		catalog: &C,
		reservations: &ReservationLedger,
		policy: PreparationPolicy,
	) -> Result<RealizedCandidate<Self::Session>, RealizeFailure<Self::Error>>;

	fn destroy(&mut self, session: Self::Session) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateRejectionStage {
	Realize,
	Stabilize,
	FinalCapacity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateRejection {
	pub candidate: CandidateIdentity,
	pub stage: CandidateRejectionStage,
	pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrepareErrorKind {
	InvalidPolicy,
	InvalidMeasuredProfile,
	ArtifactCatalog,
	InvalidReservation,
	Planning,
	Realization,
	Teardown,
	Finalization,
	CandidateExhaustion,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrepareError {
	pub kind: PrepareErrorKind,
	pub message: String,
	pub rejections: Vec<CandidateRejection>,
}

impl PrepareError {
	#[must_use]
	pub fn new(kind: PrepareErrorKind, message: impl Into<String>) -> Self {
		Self {
			kind,
			message: message.into(),
			rejections: Vec::new(),
		}
	}

	fn with_rejections(mut self, rejections: Vec<CandidateRejection>) -> Self {
		self.rejections = rejections;
		self
	}
}

impl fmt::Display for PrepareError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.message)?;
		if !self.rejections.is_empty() {
			write!(
				formatter,
				" ({} candidate rejection(s): ",
				self.rejections.len()
			)?;
			for (index, rejection) in self.rejections.iter().enumerate() {
				if index != 0 {
					write!(formatter, "; ")?;
				}
				write!(
					formatter,
					"{:?} at {:?}: {}",
					rejection.candidate, rejection.stage, rejection.detail
				)?;
			}
			write!(formatter, ")")?;
		}
		Ok(())
	}
}

impl Error for PrepareError {}

/// Successful fixed-point product. The realization session stays alive beside
/// the immutable bundle; neither can be substituted independently.
#[derive(Debug)]
pub struct PreparedSystem<C, S> {
	bundle: FinalizedBundle,
	realization: RealizationProfile,
	catalog: C,
	session: S,
	external_outputs: Vec<PlannedExternalOutput>,
	attempted_candidates: usize,
	rejections: Vec<CandidateRejection>,
}

impl<C, S> PreparedSystem<C, S> {
	#[must_use]
	pub const fn bundle(&self) -> &FinalizedBundle {
		&self.bundle
	}

	#[must_use]
	pub const fn realization(&self) -> &RealizationProfile {
		&self.realization
	}

	#[must_use]
	pub const fn catalog(&self) -> &C {
		&self.catalog
	}

	#[must_use]
	pub const fn session(&self) -> &S {
		&self.session
	}

	/// Exact logical-to-physical egress identities retained from the candidate
	/// that produced this immutable bundle.
	///
	/// Finalized exit tasks address physical arena values. This evidence
	/// preserves the one logical tensor deliberately selected for each task
	/// without reverse-matching against unrelated resident copies.
	pub fn external_outputs(&self) -> impl ExactSizeIterator<Item = (TaskId, ValueId, DeviceId, ValueId)> + '_ {
		self.external_outputs
			.iter()
			.map(|output| (output.task, output.logical, output.device, output.physical))
	}

	#[must_use]
	pub const fn attempted_candidates(&self) -> usize {
		self.attempted_candidates
	}

	#[must_use]
	pub fn rejections(&self) -> &[CandidateRejection] {
		&self.rejections
	}

	#[must_use]
	pub fn into_parts(self) -> (FinalizedBundle, RealizationProfile, C, S) {
		(self.bundle, self.realization, self.catalog, self.session)
	}
}

#[derive(Debug)]
pub struct Preparer<A, R> {
	artifact_provider: A,
	realizer: R,
	policy: PreparationPolicy,
}

impl<A, R> Preparer<A, R> {
	#[must_use]
	pub const fn new(artifact_provider: A, realizer: R) -> Self {
		Self {
			artifact_provider,
			realizer,
			policy: PreparationPolicy {
				stabilization_passes: DEFAULT_STABILIZATION_PASSES,
				stable_tail: DEFAULT_STABLE_TAIL,
			},
		}
	}

	pub fn with_policy(mut self, policy: PreparationPolicy) -> Result<Self, PrepareError> {
		policy.validate()?;
		self.policy = policy;
		Ok(self)
	}

	#[must_use]
	pub const fn artifact_provider(&self) -> &A {
		&self.artifact_provider
	}

	#[must_use]
	pub const fn realizer(&self) -> &R {
		&self.realizer
	}

	#[must_use]
	pub const fn realizer_mut(&mut self) -> &mut R {
		&mut self.realizer
	}
}

impl<A, R> Preparer<A, R>
where
	A: ArtifactProvider,
	R: CandidateRealizer<A::Catalog>,
{
	/// Run the complete fixed-point pipeline from a validated, hashed measured
	/// profile. The theoretical seed contract cannot enter this API.
	pub fn prepare(
		&mut self,
		graph: &CalculationGraph,
		profile: &MeasuredProfile,
	) -> Result<PreparedSystem<A::Catalog, R::Session>, PrepareError> {
		let program = StaticCalculationProgram::every_iteration(graph.clone(), LoopIterations::ONE)
			.map_err(|error| PrepareError::new(PrepareErrorKind::Planning, error.to_string()))?;
		self.prepare_program(&program, profile)
	}

	/// Prepares a bounded calculation program decoded from Recipe OGDL or
	/// constructed directly.
	pub fn prepare_program(
		&mut self,
		program: &StaticCalculationProgram,
		profile: &MeasuredProfile,
	) -> Result<PreparedSystem<A::Catalog, R::Session>, PrepareError> {
		self.policy.validate()?;
		validate_profile(profile)
			.map_err(|error| PrepareError::new(PrepareErrorKind::InvalidMeasuredProfile, error.to_string()))?;
		self.prepare_program_validated(program, &profile.topology, &profile.discovery)
	}

	#[cfg(test)]
	fn prepare_validated(
		&mut self,
		graph: &CalculationGraph,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<PreparedSystem<A::Catalog, R::Session>, PrepareError> {
		let program = StaticCalculationProgram::every_iteration(graph.clone(), LoopIterations::ONE)
			.map_err(|error| PrepareError::new(PrepareErrorKind::Planning, error.to_string()))?;
		self.prepare_program_validated(&program, topology, discovery)
	}

	fn prepare_program_validated(
		&mut self,
		program: &StaticCalculationProgram,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<PreparedSystem<A::Catalog, R::Session>, PrepareError> {
		let reservations = self
			.realizer
			.reservation_plan(topology, discovery)
			.map_err(|error| {
				PrepareError::new(
					PrepareErrorKind::InvalidReservation,
					format!("reservation planning failed: {error}"),
				)
			})?;
		reservations
			.validate(topology)
			.map_err(|errors| PrepareError::new(PrepareErrorKind::InvalidReservation, errors.to_string()))?;

		let planning_capacity = optimistic_planning_capacity(topology, discovery, &reservations)?;
		let catalog = self
			.artifact_provider
			.resolve(program.graph(), topology, discovery)
			.map_err(|error| {
				PrepareError::new(
					PrepareErrorKind::ArtifactCatalog,
					format!("artifact resolution failed: {error}"),
				)
			})?;
		let mut search = plan_program_candidates(
			program,
			topology,
			discovery,
			self.artifact_provider.identities(&catalog),
			&reservations,
			&planning_capacity,
		)
		.map_err(|error| PrepareError::new(PrepareErrorKind::Planning, error.to_string()))?;

		let mut attempts = 0usize;
		let mut rejections = Vec::new();
		while let Some(program_candidate) = next_owned_program_candidate(&mut search) {
			let PlannedProgramCandidate {
				planned: candidate,
				loop_iterations,
				loop_domains,
			} = program_candidate;
			attempts = attempts.checked_add(1).ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::ArithmeticOverflow,
					"candidate attempt count overflowed usize",
				)
			})?;
			let candidate_identity = candidate.draft.candidate;
			let realized = match self
				.realizer
				.realize(&candidate, &catalog, &reservations, self.policy)
			{
				Ok(realized) => realized,
				Err(RealizeFailure::CandidateRejected { detail }) => {
					reject_candidate(
						&mut search,
						&mut rejections,
						candidate_identity,
						CandidateRejectionStage::Realize,
						detail,
					)?;
					continue;
				}
				Err(RealizeFailure::Fatal(error)) => {
					return Err(PrepareError::new(
						PrepareErrorKind::Realization,
						format!("candidate {candidate_identity:?} realization failed: {error}"),
					)
					.with_rejections(rejections));
				}
			};

			let capacity = match validate_observation(
				topology,
				discovery,
				&candidate.draft,
				&realized.observation,
				self.policy,
			) {
				Ok(capacity) => capacity,
				Err(detail) => {
					self.destroy_rejected(realized.session, &rejections)?;
					reject_candidate(
						&mut search,
						&mut rejections,
						candidate_identity,
						CandidateRejectionStage::Stabilize,
						detail,
					)?;
					continue;
				}
			};

			let layouts = match pack_arenas(topology, &candidate.draft.arena_objects, &capacity) {
				Ok(layouts) => layouts,
				Err(error) => {
					self.destroy_rejected(realized.session, &rejections)?;
					reject_candidate(
						&mut search,
						&mut rejections,
						candidate_identity,
						CandidateRejectionStage::FinalCapacity,
						error.to_string(),
					)?;
					continue;
				}
			};

			let realization_identity = hash_realization(&candidate.draft, &realized.observation, self.policy);
			let realization = RealizationProfile {
				identity: realization_identity,
				draft: candidate.draft.identity,
				candidate: candidate_identity,
				discovery: discovery.identity,
				topology: topology.identity,
				artifacts: realized.observation.artifacts,
				resources: realized.observation.resources,
				reservations: realized.observation.reservations,
				capacity,
			};
			let bundle_identity = hash_bundle_with_loop_domains(
				topology,
				discovery,
				&candidate.draft,
				&realization,
				&layouts,
				loop_iterations,
				&loop_domains,
			);
			let external_outputs = candidate.external_outputs.clone();
			let bundle = match FinalizedBundle::finalize_with_loop_schedule(
				bundle_identity,
				topology,
				discovery,
				candidate.draft,
				realization.clone(),
				layouts,
				recipe_core::LoopSchedule::new(loop_iterations, loop_domains),
			) {
				Ok(bundle) => bundle,
				Err(errors) => {
					self.destroy_rejected(realized.session, &rejections)?;
					return Err(PrepareError::new(
						PrepareErrorKind::Finalization,
						format!("unchanged candidate {candidate_identity:?} failed Finalize: {errors}"),
					)
					.with_rejections(rejections));
				}
			};
			return Ok(PreparedSystem {
				bundle,
				realization,
				catalog,
				session: realized.session,
				external_outputs,
				attempted_candidates: attempts,
				rejections,
			});
		}

		Err(PrepareError::new(
			PrepareErrorKind::CandidateExhaustion,
			format!("all {attempts} finite candidates were rejected"),
		)
		.with_rejections(rejections))
	}

	fn destroy_rejected(
		&mut self,
		session: R::Session,
		rejections: &[CandidateRejection],
	) -> Result<(), PrepareError> {
		self.realizer.destroy(session).map_err(|error| {
			PrepareError::new(
				PrepareErrorKind::Teardown,
				format!("candidate teardown failed: {error}"),
			)
			.with_rejections(rejections.to_vec())
		})
	}
}

fn next_owned_program_candidate(search: &mut ProgramPlannerSearch) -> Option<PlannedProgramCandidate> {
	search.next_candidate().cloned()
}

fn reject_candidate(
	search: &mut ProgramPlannerSearch,
	rejections: &mut Vec<CandidateRejection>,
	candidate: CandidateIdentity,
	stage: CandidateRejectionStage,
	detail: String,
) -> Result<(), PrepareError> {
	search.reject(candidate).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::Planning,
			format!("candidate rejection bookkeeping failed: {error}"),
		)
		.with_rejections(rejections.clone())
	})?;
	rejections.push(CandidateRejection {
		candidate,
		stage,
		detail,
	});
	Ok(())
}

/// Draft enumeration uses the measured physical total minus only the mandatory
/// reservation as an optimistic upper bound. It is not a realization snapshot.
/// Finalize always repacks against the post-warm capacity returned by the
/// backend, and rejects a candidate that no longer fits.
fn optimistic_planning_capacity(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	reservations: &ReservationLedger,
) -> Result<CapacityLedger, PrepareError> {
	let mut entries = Vec::with_capacity(topology.devices.len());
	for device in &topology.devices {
		let discovered = discovery.device(device.id).ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::InvalidMeasuredProfile,
				format!("device {} has no discovery capacity", device.id),
			)
		})?;
		let reservation = reservations.entry(device.id).ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::InvalidReservation,
				format!("device {} has no reservation plan", device.id),
			)
		})?;
		let usable = discovered
			.total_capacity
			.value
			.checked_sub(reservation.bytes)
			.ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::InvalidReservation,
					format!(
						"device {} capacity {} is smaller than the exact {}-byte reservation",
						device.id,
						discovered.total_capacity.value.get(),
						EXACT_USER_RESERVATION.get()
					),
				)
			})?;
		let provenance = discovered.total_capacity.provenance;
		entries.push(CapacityLedgerEntry {
			device: device.id,
			total: discovered.total_capacity,
			runtime_overhead: Property::new(ByteCount::ZERO, provenance),
			fragmentation: Property::new(ByteCount::ZERO, provenance),
			safety_headroom: Property::new(ByteCount::ZERO, provenance),
			recipe_usable: Property::new(usable, provenance),
		});
	}
	let ledger = CapacityLedger { entries };
	ledger.validate(topology, reservations)
		.map_err(|errors| PrepareError::new(PrepareErrorKind::InvalidMeasuredProfile, errors.to_string()))?;
	Ok(ledger)
}

fn validate_observation(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	draft: &DraftPlan,
	observation: &RealizationObservation,
	policy: PreparationPolicy,
) -> Result<CapacityLedger, String> {
	let expected_passes = usize::try_from(policy.stabilization_passes.get())
		.map_err(|_| "stabilization pass count does not fit usize".to_owned())?;
	if observation.capacity_snapshots.len() != expected_passes {
		return Err(format!(
			"backend returned {} capacity snapshots, expected exactly {expected_passes}",
			observation.capacity_snapshots.len()
		));
	}
	for (index, snapshot) in observation.capacity_snapshots.iter().enumerate() {
		snapshot
			.validate(topology, &observation.reservations)
			.map_err(|errors| format!("capacity snapshot {index} is invalid: {errors}"))?;
	}
	let stable_tail = usize::try_from(policy.stable_tail.get())
		.map_err(|_| "stable-tail length does not fit usize".to_owned())?;
	let stable_start = observation
		.capacity_snapshots
		.len()
		.checked_sub(stable_tail)
		.ok_or_else(|| "stable-tail length exceeds observations".to_owned())?;
	let final_snapshot = observation
		.capacity_snapshots
		.last()
		.cloned()
		.ok_or_else(|| "realization returned no capacity snapshot".to_owned())?;
	if observation.capacity_snapshots[stable_start..]
		.iter()
		.any(|snapshot| snapshot != &final_snapshot)
	{
		let deltas = capacity_stability_deltas(
			topology,
			&observation.capacity_snapshots,
			stable_start,
			&final_snapshot,
		);
		return Err(format!(
			"the final {stable_tail} post-warm capacity snapshots did not stabilize: {}",
			deltas.join("; ")
		));
	}

	let profile = RealizationProfile {
		identity: RealizationIdentity::new(recipe_core::Digest::new([1; 32])),
		draft: draft.identity,
		candidate: draft.candidate,
		discovery: discovery.identity,
		topology: topology.identity,
		artifacts: observation.artifacts.clone(),
		resources: observation.resources.clone(),
		reservations: observation.reservations.clone(),
		capacity: final_snapshot.clone(),
	};
	profile
		.validate(topology, discovery, draft)
		.map_err(|errors| format!("realization differs from its unchanged Draft: {errors}"))?;
	Ok(final_snapshot)
}

fn capacity_stability_deltas(
	topology: &Topology,
	snapshots: &[CapacityLedger],
	stable_start: usize,
	final_snapshot: &CapacityLedger,
) -> Vec<String> {
	let final_pass = snapshots.len();
	let mut deltas = Vec::new();
	for (index, snapshot) in snapshots
		.iter()
		.enumerate()
		.skip(stable_start)
		.take(final_pass.saturating_sub(stable_start + 1))
	{
		if snapshot == final_snapshot {
			continue;
		}
		for device in &topology.devices {
			let Some(observed) = snapshot.entry(device.id) else {
				deltas.push(format!(
					"pass {} device {} is absent but present in final pass {final_pass}",
					index + 1,
					device.id
				));
				continue;
			};
			let Some(final_entry) = final_snapshot.entry(device.id) else {
				deltas.push(format!(
					"pass {} device {} is present but absent from final pass {final_pass}",
					index + 1,
					device.id
				));
				continue;
			};
			for (field, observed, final_value) in [
				("total", observed.total.value, final_entry.total.value),
				(
					"runtime_overhead",
					observed.runtime_overhead.value,
					final_entry.runtime_overhead.value,
				),
				(
					"fragmentation",
					observed.fragmentation.value,
					final_entry.fragmentation.value,
				),
				(
					"safety_headroom",
					observed.safety_headroom.value,
					final_entry.safety_headroom.value,
				),
				(
					"recipe_usable",
					observed.recipe_usable.value,
					final_entry.recipe_usable.value,
				),
			] {
				if observed != final_value {
					deltas.push(format!(
						"pass {} device {} {field}={} vs final pass {final_pass}={} ({})",
						index + 1,
						device.id,
						observed.get(),
						final_value.get(),
						byte_delta(observed, final_value)
					));
				}
			}
		}
	}
	if deltas.is_empty() {
		deltas.push("snapshot ordering or metadata differed without a byte-field delta".to_owned());
	}
	deltas
}

fn byte_delta(observed: ByteCount, final_value: ByteCount) -> String {
	match final_value.get().cmp(&observed.get()) {
		std::cmp::Ordering::Greater => format!("final increased by {}", final_value.get() - observed.get()),
		std::cmp::Ordering::Less => format!("final decreased by {}", observed.get() - final_value.get()),
		std::cmp::Ordering::Equal => "no byte delta".to_owned(),
	}
}

fn hash_realization(
	draft: &DraftPlan,
	observation: &RealizationObservation,
	policy: PreparationPolicy,
) -> RealizationIdentity {
	let mut hash = CanonicalHash::new("recipe-realization-v3");
	hash.digest(draft.identity.digest());
	hash.digest(draft.candidate.digest());
	hash.digest(draft.discovery.digest());
	hash.digest(draft.topology.digest());
	hash.artifact_builds(&draft.artifact_builds);
	hash.value_aliases(&draft.value_aliases);
	hash.tasks(&draft.tasks);
	hash.u64(u64::from(policy.stabilization_passes.get()));
	hash.u64(u64::from(policy.stable_tail.get()));
	hash.artifacts(&observation.artifacts);
	hash.resources(&observation.resources);
	hash.reservations(&observation.reservations);
	hash.u64(observation.capacity_snapshots.len() as u64);
	for snapshot in &observation.capacity_snapshots {
		hash.capacity(snapshot);
	}
	RealizationIdentity::new(hash.finish())
}

#[cfg(test)]
fn hash_bundle(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	draft: &DraftPlan,
	realization: &RealizationProfile,
	layouts: &[recipe_core::ArenaLayout],
) -> BundleIdentity {
	let loop_domains = draft
		.tasks
		.iter()
		.filter(|task| task.phase == recipe_core::RunPhase::Loop)
		.map(|task| LoopTaskDomain {
			task: task.id,
			domain: recipe_core::IterationDomain::every(LoopIterations::ONE),
		})
		.collect::<Vec<_>>();
	hash_bundle_with_loop_domains(
		topology,
		discovery,
		draft,
		realization,
		layouts,
		LoopIterations::ONE,
		&loop_domains,
	)
}

fn hash_bundle_with_loop_domains(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	draft: &DraftPlan,
	realization: &RealizationProfile,
	layouts: &[recipe_core::ArenaLayout],
	loop_iterations: LoopIterations,
	loop_domains: &[LoopTaskDomain],
) -> BundleIdentity {
	let mut hash = CanonicalHash::new("recipe-finalized-bundle-v5");
	hash.digest(topology.identity.digest());
	hash.digest(discovery.identity.digest());
	hash.digest(draft.identity.digest());
	hash.digest(realization.identity.digest());
	hash.digest(draft.candidate.digest());
	hash.artifacts(&realization.artifacts);
	hash.artifact_builds(&draft.artifact_builds);
	hash.value_aliases(&draft.value_aliases);
	hash.tasks(&draft.tasks);
	hash.u64(u64::from(loop_iterations.is_unbounded()));
	hash.u64(loop_iterations.finite().map_or(0, |finite| finite.get()));
	hash.u64(loop_domains.len() as u64);
	for assignment in loop_domains {
		hash.u64(assignment.task.get());
		hash.u64(assignment.domain.first_iteration());
		hash.u64(u64::from(assignment.domain.is_unbounded()));
		hash.u64(assignment.domain.end_exclusive().unwrap_or(0));
		hash.u64(assignment.domain.stride().get());
	}
	hash.resources(&realization.resources);
	hash.reservations(&realization.reservations);
	hash.capacity(&realization.capacity);
	let mut layouts = layouts.to_vec();
	layouts.sort_by_key(|layout| layout.device);
	hash.u64(layouts.len() as u64);
	for layout in layouts {
		hash.u64(layout.device.get());
		hash.u64(layout.size.get());
		let mut allocations = layout.allocations;
		allocations.sort_by_key(|allocation| allocation.object);
		hash.u64(allocations.len() as u64);
		for allocation in allocations {
			hash.u64(allocation.object.get());
			hash.u64(allocation.offset.get());
		}
	}
	BundleIdentity::new(hash.finish())
}

#[derive(Debug)]
struct CanonicalHash {
	hasher: Sha256,
}

impl CanonicalHash {
	fn new(domain: &str) -> Self {
		let mut result = Self {
			hasher: Sha256::new(),
		};
		result.bytes(domain.as_bytes());
		result
	}

	fn bytes(&mut self, bytes: &[u8]) {
		self.hasher.update((bytes.len() as u64).to_le_bytes());
		self.hasher.update(bytes);
	}

	fn u64(&mut self, value: u64) {
		self.hasher.update(value.to_le_bytes());
	}

	fn digest(&mut self, digest: recipe_core::Digest) {
		self.hasher.update(digest.bytes());
	}

	fn artifacts(&mut self, artifacts: &[ArtifactIdentity]) {
		let mut artifacts = artifacts.iter().collect::<Vec<_>>();
		artifacts.sort_by_key(|artifact| artifact.id);
		self.u64(artifacts.len() as u64);
		for artifact in artifacts {
			self.u64(artifact.id.get());
			self.digest(artifact.digest);
			self.bytes(artifact.format.as_str().as_bytes());
			self.bytes(artifact.target.backend.as_str().as_bytes());
			self.bytes(artifact.target.architecture.as_str().as_bytes());
			self.bytes(artifact.target.abi.as_str().as_bytes());
			self.bytes(artifact.toolchain.name.as_str().as_bytes());
			self.bytes(artifact.toolchain.version.as_str().as_bytes());
			self.digest(artifact.toolchain.digest);
			self.bytes(artifact.entry_symbol.as_str().as_bytes());
			self.u64(artifact.kernel_template.get());
			self.kernel_resources(&artifact.resources);
			match artifact.build {
				Some(build) => {
					self.u64(1);
					self.build_provenance(build);
				}
				None => self.u64(0),
			}
		}
	}

	fn artifact_builds(&mut self, builds: &[recipe_core::ArtifactBuildRecipe]) {
		let mut builds = builds.iter().collect::<Vec<_>>();
		builds.sort_by_key(|build| build.artifact);
		self.u64(builds.len() as u64);
		for build in builds {
			self.u64(build.artifact.get());
			self.u64(build.kernel_template.get());
			self.u64(build.source_kernel.get());
			self.build_provenance(build.provenance);
			self.u64(build.bindings.len() as u64);
			for binding in &build.bindings {
				self.u64(binding.value.get());
				self.u64(match binding.dtype {
					recipe_core::DType::F32 => 0,
					recipe_core::DType::I32 => 1,
				});
				self.u64(match binding.access {
					recipe_core::ArtifactBuildAccess::Read => 0,
					recipe_core::ArtifactBuildAccess::Write => 1,
					recipe_core::ArtifactBuildAccess::ReadWrite => 2,
					recipe_core::ArtifactBuildAccess::ReadWriteAtomic => 3,
				});
				self.u64(binding.view.logical_extents.len() as u64);
				for extent in &binding.view.logical_extents {
					self.u64(*extent);
				}
				self.u64(binding.view.offset_elements);
				self.u64(binding.view.strides.len() as u64);
				for stride in &binding.view.strides {
					self.u64(*stride);
				}
				self.u64(binding.view.storage_bytes.get());
			}
			self.u64(build.dispatch.logical_lanes);
			self.u64(u64::from(build.dispatch.workgroup_lanes));
			self.u64(build.dispatch.workgroups);
			self.u64(build.work.flops.get());
			self.u64(build.work.integer_operations);
			self.u64(build.work.atomic_operations);
			match build.fault_flag {
				Some(fault_flag) => {
					self.u64(1);
					self.u64(fault_flag.get());
				}
				None => self.u64(0),
			}
			self.kernel_resources(&build.resources);
		}
	}

	fn build_provenance(&mut self, provenance: recipe_core::ArtifactBuildProvenance) {
		self.digest(provenance.program_digest);
		self.u64(u64::from(provenance.stage_ordinal));
		self.digest(provenance.contract_digest);
	}

	fn kernel_resources(&mut self, resources: &recipe_core::KernelResourceBounds) {
		self.u64(resources.private_bytes_per_lane.get());
		self.u64(resources.shared_bytes_per_workgroup.get());
		self.u64(resources.scratch_bytes_per_dispatch.get());
		self.u64(u64::from(resources.maximum_workgroup_lanes));
	}

	fn value_aliases(&mut self, aliases: &[recipe_core::ValueAliasContract]) {
		let mut aliases = aliases.to_vec();
		aliases.sort_by_key(|alias| (alias.input, alias.output));
		self.u64(aliases.len() as u64);
		for alias in aliases {
			self.u64(alias.input.get());
			self.u64(alias.output.get());
			self.u64(match alias.permission {
				recipe_core::AliasPermission::Forbidden => 0,
				recipe_core::AliasPermission::MayAliasExact => 1,
				recipe_core::AliasPermission::MustAliasExact => 2,
			});
		}
	}

	fn tasks(&mut self, tasks: &[recipe_core::Task]) {
		let mut tasks = tasks.iter().collect::<Vec<_>>();
		tasks.sort_by_key(|task| task.id);
		self.u64(tasks.len() as u64);
		for task in tasks {
			self.u64(task.id.get());
			self.u64(match task.phase {
				recipe_core::RunPhase::Init => 0,
				recipe_core::RunPhase::Loop => 1,
				recipe_core::RunPhase::Exit => 2,
			});
			self.u64(task.window.start.get());
			self.u64(task.window.end.get());
			let mut dependencies = task.dependencies.clone();
			dependencies.sort();
			self.u64(dependencies.len() as u64);
			for dependency in dependencies {
				self.u64(dependency.get());
			}
			match &task.kind {
				recipe_core::TaskKind::Calculation(calculation) => {
					self.u64(0);
					self.u64(calculation.device.get());
					self.u64(calculation.kernel_template.get());
					self.u64(calculation.artifact.get());
					self.u64(calculation.inputs.len() as u64);
					for input in &calculation.inputs {
						self.u64(input.get());
					}
					self.u64(calculation.outputs.len() as u64);
					for output in &calculation.outputs {
						self.u64(output.get());
					}
					match calculation.fault_flag {
						Some(fault_flag) => {
							self.u64(1);
							self.u64(fault_flag.get());
						}
						None => self.u64(0),
					}
					self.u64(calculation.work.get());
					self.u64(calculation.submission.queue.get());
					self.u64(calculation.submission.completion.get());
				}
				recipe_core::TaskKind::Transfer(transfer) => {
					self.u64(1);
					self.transfer_endpoint(transfer.source);
					self.transfer_endpoint(transfer.destination);
					self.u64(transfer.bytes.get());
					self.u64(transfer.route.len() as u64);
					for link in &transfer.route {
						self.u64(link.get());
					}
					self.u64(transfer.lane_claims.len() as u64);
					for claim in &transfer.lane_claims {
						match claim {
							recipe_core::TransferLaneClaim::Link { link, lane } => {
								self.u64(0);
								self.u64(link.get());
								self.u64(u64::from(*lane));
							}
							recipe_core::TransferLaneClaim::External { device, lane } => {
								self.u64(1);
								self.u64(device.get());
								self.u64(u64::from(*lane));
							}
						}
					}
					self.u64(transfer.submission.queue.get());
					self.u64(transfer.submission.completion.get());
				}
				recipe_core::TaskKind::Metric(metric) => {
					self.u64(2);
					self.u64(metric.metric.get());
					self.u64(metric.value.get());
					self.u64(metric.slot.get());
					self.u64(metric.submission.queue.get());
					self.u64(metric.submission.completion.get());
				}
			}
		}
	}

	fn transfer_endpoint(&mut self, endpoint: recipe_core::TransferEndpoint) {
		match endpoint {
			recipe_core::TransferEndpoint::External => self.u64(0),
			recipe_core::TransferEndpoint::Device { device, value } => {
				self.u64(1);
				self.u64(device.get());
				self.u64(value.get());
			}
		}
	}

	fn resources(&mut self, resources: &ResourceManifest) {
		let mut queues = resources.queues.clone();
		queues.sort_by_key(|queue| queue.id);
		self.u64(queues.len() as u64);
		for queue in queues {
			self.u64(queue.id.get());
			self.u64(queue.device.get());
		}
		let mut completions = resources.completions.clone();
		completions.sort_by_key(|completion| completion.id);
		self.u64(completions.len() as u64);
		for completion in completions {
			self.u64(completion.id.get());
			self.u64(completion.device.get());
		}
		let mut metrics = resources.metrics.clone();
		metrics.sort_by_key(|metric| metric.id);
		self.u64(metrics.len() as u64);
		for metric in metrics {
			self.u64(metric.id.get());
			self.u64(metric.metric.get());
		}
		let mut pinned_staging = resources.pinned_staging.clone();
		pinned_staging.sort_by_key(|entry| entry.device);
		self.u64(pinned_staging.len() as u64);
		for entry in pinned_staging {
			self.u64(entry.device.get());
			self.u64(entry.bytes.get());
		}
		let mut scratch = resources.scratch.clone();
		scratch.sort_by_key(|entry| entry.device);
		self.u64(scratch.len() as u64);
		for entry in scratch {
			self.u64(entry.device.get());
			self.u64(entry.bytes.get());
		}
	}

	fn reservations(&mut self, reservations: &ReservationLedger) {
		let mut entries = reservations.entries.clone();
		entries.sort_by_key(|entry| entry.device);
		self.u64(entries.len() as u64);
		for entry in entries {
			self.u64(entry.device.get());
			self.bytes(entry.name.as_str().as_bytes());
			self.u64(entry.bytes.get());
			self.u64(match entry.mechanism {
				recipe_core::ReservationMechanism::HeldAllocation => 0,
				recipe_core::ReservationMechanism::EnforcedQuota => 1,
			});
			match entry.evidence {
				recipe_core::ReservationEvidence::NonGpu => self.u64(0),
				recipe_core::ReservationEvidence::GpuDisplay { enabled_connectors } => {
					self.u64(1);
					self.u64(u64::from(enabled_connectors));
				}
			}
		}
	}

	fn capacity(&mut self, capacity: &CapacityLedger) {
		let mut entries = capacity.entries.clone();
		entries.sort_by_key(|entry| entry.device);
		self.u64(entries.len() as u64);
		for entry in entries {
			self.u64(entry.device.get());
			self.property(entry.total);
			self.property(entry.runtime_overhead);
			self.property(entry.fragmentation);
			self.property(entry.safety_headroom);
			self.property(entry.recipe_usable);
		}
	}

	fn property(&mut self, property: Property<ByteCount>) {
		self.u64(property.value.get());
		self.u64(match property.provenance {
			PropertyProvenance::Estimated => 0,
			PropertyProvenance::Measured => 1,
			PropertyProvenance::Override => 2,
		});
	}

	fn finish(self) -> recipe_core::Digest {
		let bytes: [u8; 32] = self.hasher.finalize().into();
		recipe_core::Digest::new(bytes)
	}
}

#[cfg(test)]
mod tests;
