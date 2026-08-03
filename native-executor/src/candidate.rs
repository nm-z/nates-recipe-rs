//! Pre-final native candidate realization.
//!
//! This boundary is intentionally earlier than [`crate::LocalBackend`] run
//! binding. A successful session owns the exact loaded artifacts,
//! reservations, queues, completion objects, staging, scratch, and warmed
//! driver state during final capacity stabilization and arena repacking.
//! It contains no finalized arena offsets.

use core::fmt;
use std::{
	collections::{BTreeMap, BTreeSet},
	error::Error as StdError,
};

use recipe_core::{
	ArtifactBuildRecipe, ArtifactId, ArtifactIdentity, CapacityLedger, DeviceId, DiscoveryIdentity, DiscoveryProfile,
	ReservationEvidence, ReservationLedger, Topology, TopologyIdentity, ValidationErrors,
};
use recipe_planner::PlannedCandidate;

use crate::{RuntimeArtifact, plan::validate_runtime_artifact};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateArtifact {
	identity: ArtifactIdentity,
	runtime: RuntimeArtifact,
}

impl CandidateArtifact {
	#[must_use]
	pub const fn new(identity: ArtifactIdentity, runtime: RuntimeArtifact) -> Self { Self { identity, runtime } }

	#[must_use]
	pub const fn identity(&self) -> &ArtifactIdentity { &self.identity }

	#[must_use]
	pub const fn runtime(&self) -> &RuntimeArtifact { &self.runtime }

	#[must_use]
	pub fn into_parts(self) -> (ArtifactIdentity, RuntimeArtifact) { (self.identity, self.runtime) }
}

#[derive(Clone, Copy, Debug)]
pub struct CandidateRealizationRequest<'a> {
	pub topology: &'a Topology,
	pub discovery: &'a DiscoveryProfile,
	pub candidate: &'a PlannedCandidate,
	pub artifacts: &'a [CandidateArtifact],
	pub reservations: &'a ReservationLedger,
}

impl CandidateRealizationRequest<'_> {
	pub fn validate(&self) -> Result<(), CandidateRequestError> {
		self.topology
			.validate()
			.map_err(CandidateRequestError::InvalidTopology)?;
		self.discovery
			.validate(self.topology)
			.map_err(CandidateRequestError::InvalidDiscovery)?;
		self.candidate
			.draft
			.validate(self.topology, self.discovery)
			.map_err(CandidateRequestError::InvalidDraft)?;
		self.reservations
			.validate(self.topology)
			.map_err(CandidateRequestError::InvalidReservations)?;
		validate_artifacts(self)
	}
}

#[derive(Debug)]
pub enum CandidateRequestError {
	InvalidTopology(ValidationErrors),
	InvalidDiscovery(ValidationErrors),
	InvalidDraft(ValidationErrors),
	InvalidReservations(ValidationErrors),
	ArtifactSetOverlap,
	InvalidArtifact {
		artifact: ArtifactId,
		errors: ValidationErrors,
	},
	DuplicateArtifact {
		artifact: ArtifactId,
	},
	StaticArtifactMismatch {
		artifact: ArtifactId,
	},
	UnexpectedArtifact {
		artifact: ArtifactId,
	},
	RuntimeArtifact {
		artifact: ArtifactId,
		source: Box<crate::Error>,
	},
	MissingRuntimeArtifact {
		artifact: ArtifactId,
	},
	RuntimeArtifactSetMismatch,
	DeferredArtifactMismatch {
		artifact: ArtifactId,
		build: ArtifactId,
	},
	MissingStagePlacement {
		artifact: ArtifactId,
	},
	MissingGpuCapability {
		artifact: ArtifactId,
	},
	TargetMismatch {
		artifact: ArtifactId,
	},
}

impl fmt::Display for CandidateRequestError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidTopology(errors) => write!(formatter, "topology is invalid: {errors}"),
			Self::InvalidDiscovery(errors) => write!(formatter, "discovery is invalid: {errors}"),
			Self::InvalidDraft(errors) => write!(formatter, "candidate Draft is invalid: {errors}"),
			Self::InvalidReservations(errors) => {
				write!(formatter, "reservation ledger is invalid: {errors}")
			}
			Self::ArtifactSetOverlap => {
				formatter.write_str("candidate artifact and deferred-build identities overlap")
			}
			Self::InvalidArtifact { artifact, errors } => {
				write!(formatter, "artifact {artifact} is invalid: {errors}")
			}
			Self::DuplicateArtifact { artifact } => {
				write!(formatter, "artifact {artifact} appears more than once")
			}
			Self::StaticArtifactMismatch { artifact } => {
				write!(
					formatter,
					"artifact {artifact} differs from its immutable Draft identity"
				)
			}
			Self::UnexpectedArtifact { artifact } => {
				write!(
					formatter,
					"artifact {artifact} is not required exactly once by the candidate"
				)
			}
			Self::RuntimeArtifact { artifact, source } => {
				write!(
					formatter,
					"runtime artifact {artifact} is invalid: {source}"
				)
			}
			Self::MissingRuntimeArtifact { artifact } => {
				write!(
					formatter,
					"required artifact {artifact} has no exact runtime image"
				)
			}
			Self::RuntimeArtifactSetMismatch => {
				formatter.write_str("runtime artifact set differs from the candidate requirement")
			}
			Self::DeferredArtifactMismatch { artifact, build } => {
				write!(
					formatter,
					"realized artifact {artifact} differs from deferred build {build}"
				)
			}
			Self::MissingStagePlacement { artifact } => {
				write!(
					formatter,
					"deferred artifact {artifact} has no immutable stage placement"
				)
			}
			Self::MissingGpuCapability { artifact } => {
				write!(
					formatter,
					"deferred artifact {artifact} placement has no GPU capability"
				)
			}
			Self::TargetMismatch { artifact } => {
				write!(
					formatter,
					"deferred artifact {artifact} target differs from its measured placement"
				)
			}
		}
	}
}

impl StdError for CandidateRequestError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::InvalidTopology(errors)
			| Self::InvalidDiscovery(errors)
			| Self::InvalidDraft(errors)
			| Self::InvalidReservations(errors) => Some(errors),
			Self::RuntimeArtifact { source, .. } => Some(source),
			Self::ArtifactSetOverlap
			| Self::InvalidArtifact { .. }
			| Self::DuplicateArtifact { .. }
			| Self::StaticArtifactMismatch { .. }
			| Self::UnexpectedArtifact { .. }
			| Self::MissingRuntimeArtifact { .. }
			| Self::RuntimeArtifactSetMismatch
			| Self::DeferredArtifactMismatch { .. }
			| Self::MissingStagePlacement { .. }
			| Self::MissingGpuCapability { .. }
			| Self::TargetMismatch { .. } => None,
		}
	}
}

#[derive(Debug)]
pub enum CandidateFailure<E> {
	CandidateRejected { detail: String },
	Fatal(E),
	PreFinalRealizationUnavailable { detail: &'static str },
}

impl<E: fmt::Display> fmt::Display for CandidateFailure<E> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::CandidateRejected { detail } => write!(formatter, "candidate rejected: {detail}"),
			Self::Fatal(error) => write!(formatter, "candidate realization failed: {error}"),
			Self::PreFinalRealizationUnavailable { detail } => {
				write!(
					formatter,
					"pre-final candidate realization is unavailable: {detail}"
				)
			}
		}
	}
}

impl<E> StdError for CandidateFailure<E>
where E: StdError + 'static
{
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Fatal(error) => Some(error),
			Self::CandidateRejected { .. } | Self::PreFinalRealizationUnavailable { .. } => None,
		}
	}
}

/// Dependency-neutral native realization lifecycle used by
/// `recipe-prepare`'s production driver adapter.
///
/// Implementations must load and inspect every exact artifact, allocate every
/// exact reservation, create all candidate resource-manifest objects, and
/// perform no deferred initialization after `realize_candidate`. Each warm
/// pass must exercise the candidate's maximum-concurrency trace. Capacity
/// snapshots are taken only after a complete warm pass. `destroy_candidate`
/// must deterministically release every partially or fully realized object.
pub trait CandidateSessionFactory {
	type Session: fmt::Debug;
	type Error: StdError + Send + Sync + 'static;

	fn reservation_evidence(&self, device: DeviceId) -> Result<ReservationEvidence, Self::Error>;

	fn realize_candidate(
		&mut self,
		request: CandidateRealizationRequest<'_>,
	) -> Result<Self::Session, CandidateFailure<Self::Error>>;

	fn warm_maximum_concurrency(
		&mut self,
		session: &mut Self::Session,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<Self::Error>>;

	fn capacity_snapshot(
		&mut self,
		session: &mut Self::Session,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<Self::Error>>;

	fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error>;
}

/// Validation wrapper for a physical candidate factory.
///
/// It rejects mismatched topology, Draft, artifact, reservation, warm-pass, and
/// capacity evidence before that evidence can cross the preparation boundary.
#[derive(Debug)]
pub struct ValidatedCandidateFactory<F> {
	inner: F,
}

impl<F> ValidatedCandidateFactory<F> {
	#[must_use]
	pub const fn new(inner: F) -> Self { Self { inner } }

	#[must_use]
	pub const fn inner(&self) -> &F { &self.inner }

	#[must_use]
	pub const fn inner_mut(&mut self) -> &mut F { &mut self.inner }

	#[must_use]
	pub fn into_inner(self) -> F { self.inner }
}

#[derive(Debug)]
pub struct ValidatedCandidateSession<S> {
	inner: S,
	candidate: PlannedCandidate,
	topology: TopologyIdentity,
	discovery: DiscoveryIdentity,
	reservations: ReservationLedger,
}

impl<S> ValidatedCandidateSession<S> {
	#[must_use]
	pub const fn inner(&self) -> &S { &self.inner }

	#[must_use]
	pub const fn inner_mut(&mut self) -> &mut S { &mut self.inner }

	#[must_use]
	pub fn into_inner(self) -> S { self.inner }
}

impl<F> CandidateSessionFactory for ValidatedCandidateFactory<F>
where F: CandidateSessionFactory
{
	type Error = F::Error;
	type Session = ValidatedCandidateSession<F::Session>;

	fn reservation_evidence(&self, device: DeviceId) -> Result<ReservationEvidence, Self::Error> {
		self.inner.reservation_evidence(device)
	}

	fn realize_candidate(
		&mut self,
		request: CandidateRealizationRequest<'_>,
	) -> Result<Self::Session, CandidateFailure<Self::Error>> {
		request.validate().map_err(|error| {
			CandidateFailure::CandidateRejected {
				detail: error.to_string(),
			}
		})?;
		let candidate = request.candidate.clone();
		let topology = request.topology.identity;
		let discovery = request.discovery.identity;
		let reservations = request.reservations.clone();
		let inner = self.inner.realize_candidate(request)?;
		Ok(ValidatedCandidateSession {
			inner,
			candidate,
			topology,
			discovery,
			reservations,
		})
	}

	fn warm_maximum_concurrency(
		&mut self,
		session: &mut Self::Session,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<Self::Error>> {
		match pass {
			0 => {
				Err(CandidateFailure::CandidateRejected {
					detail: "warm pass numbering must start at one".to_owned(),
				})
			}
			1..=u32::MAX => Ok(()),
		}?;
		match *candidate == session.candidate {
			true => Ok(()),
			false => {
				Err(CandidateFailure::CandidateRejected {
					detail: "warm trace names a different candidate".to_owned(),
				})
			}
		}?;
		self.inner
			.warm_maximum_concurrency(&mut session.inner, candidate, pass)
	}

	fn capacity_snapshot(
		&mut self,
		session: &mut Self::Session,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<Self::Error>> {
		match topology.identity == session.topology && discovery.identity == session.discovery {
			true => Ok(()),
			false => {
				Err(CandidateFailure::CandidateRejected {
					detail: "capacity snapshot topology or discovery identity changed".to_owned(),
				})
			}
		}?;
		let snapshot = self
			.inner
			.capacity_snapshot(&mut session.inner, topology, discovery)?;
		snapshot
			.validate(topology, &session.reservations)
			.map_err(|errors| {
				CandidateFailure::CandidateRejected {
					detail: format!("native capacity snapshot is invalid: {errors}"),
				}
			})?;
		Ok(snapshot)
	}

	fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error> {
		self.inner.destroy_candidate(session.inner)
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableCandidateFactory;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnavailableCandidateError;

impl fmt::Display for UnavailableCandidateError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("pre-final native candidate realization is unavailable")
	}
}

impl StdError for UnavailableCandidateError {}

fn unavailable_candidate<T>(context: T) -> CandidateFailure<UnavailableCandidateError> {
	drop(context);
	CandidateFailure::PreFinalRealizationUnavailable {
		detail: "no native candidate factory was configured",
	}
}

impl CandidateSessionFactory for UnavailableCandidateFactory {
	type Error = UnavailableCandidateError;
	type Session = ();

	fn reservation_evidence(&self, device: DeviceId) -> Result<ReservationEvidence, Self::Error> {
		let _ = device;
		Err(UnavailableCandidateError)
	}

	fn realize_candidate(
		&mut self,
		request: CandidateRealizationRequest<'_>,
	) -> Result<Self::Session, CandidateFailure<Self::Error>> {
		Err(unavailable_candidate(request))
	}

	fn warm_maximum_concurrency(
		&mut self,
		session: &mut Self::Session,
		candidate: &PlannedCandidate,
		pass: u32,
	) -> Result<(), CandidateFailure<Self::Error>> {
		Err(unavailable_candidate((session, candidate, pass)))
	}

	fn capacity_snapshot(
		&mut self,
		session: &mut Self::Session,
		topology: &Topology,
		discovery: &DiscoveryProfile,
	) -> Result<CapacityLedger, CandidateFailure<Self::Error>> {
		Err(unavailable_candidate((session, topology, discovery)))
	}

	fn destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error> {
		let () = session;
		Ok(())
	}
}

fn validate_artifacts(request: &CandidateRealizationRequest<'_>) -> Result<(), CandidateRequestError> {
	let static_artifacts = request
		.candidate
		.draft
		.artifacts
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	let builds = request
		.candidate
		.draft
		.artifact_builds
		.iter()
		.map(|build| (build.artifact, build))
		.collect::<BTreeMap<_, _>>();
	let expected = static_artifacts
		.keys()
		.chain(builds.keys())
		.copied()
		.collect::<BTreeSet<_>>();
	match expected.len() == static_artifacts.len() + builds.len() {
		true => Ok(()),
		false => Err(CandidateRequestError::ArtifactSetOverlap),
	}?;

	let mut observed = BTreeSet::new();
	for artifact in request.artifacts {
		let identity = artifact.identity();
		identity.validate().map_err(|errors| {
			CandidateRequestError::InvalidArtifact {
				artifact: identity.id,
				errors,
			}
		})?;
		match observed.insert(identity.id) {
			true => Ok(()),
			false => {
				Err(CandidateRequestError::DuplicateArtifact {
					artifact: identity.id,
				})
			}
		}?;
		let identity_result = match (static_artifacts.get(&identity.id), builds.get(&identity.id)) {
			(Some(expected), None) => {
				match *expected == identity {
					true => Ok(()),
					false => {
						Err(CandidateRequestError::StaticArtifactMismatch {
							artifact: identity.id,
						})
					}
				}
			}
			(None, Some(build)) => validate_built_identity(request, build, identity),
			(None, None) | (Some(_), Some(_)) => {
				Err(CandidateRequestError::UnexpectedArtifact {
					artifact: identity.id,
				})
			}
		};
		identity_result?;
		validate_runtime_artifact(identity, artifact.runtime()).map_err(|source| {
			CandidateRequestError::RuntimeArtifact {
				artifact: identity.id,
				source: Box::new(source),
			}
		})?;
	}
	match observed == expected {
		true => Ok(()),
		false => {
			match expected.difference(&observed).next() {
				Some(missing) => Err(CandidateRequestError::MissingRuntimeArtifact { artifact: *missing }),
				None => Err(CandidateRequestError::RuntimeArtifactSetMismatch),
			}
		}
	}
}

fn validate_built_identity(
	request: &CandidateRealizationRequest<'_>,
	build: &ArtifactBuildRecipe,
	identity: &ArtifactIdentity,
) -> Result<(), CandidateRequestError> {
	match identity.kernel_template == build.kernel_template
		&& identity.resources == build.resources
		&& identity.build == Some(build.provenance)
	{
		true => Ok(()),
		false => {
			Err(CandidateRequestError::DeferredArtifactMismatch {
				artifact: identity.id,
				build: build.artifact,
			})
		}
	}?;
	let placement = match request
		.candidate
		.stage_placements
		.iter()
		.find(|placement| placement.artifact == identity.id)
	{
		Some(placement) => placement,
		None => {
			return Err(CandidateRequestError::MissingStagePlacement {
				artifact: identity.id,
			});
		}
	};
	let discovered_device = match request
		.discovery
		.devices
		.iter()
		.find(|device| device.device == placement.device)
	{
		Some(device) => device,
		None => {
			return Err(CandidateRequestError::MissingGpuCapability {
				artifact: identity.id,
			});
		}
	};
	let discovered = match discovered_device.calculation.as_ref() {
		Some(calculation) => calculation,
		None => {
			return Err(CandidateRequestError::MissingGpuCapability {
				artifact: identity.id,
			});
		}
	};
	match identity.target == discovered.target {
		true => Ok(()),
		false => {
			Err(CandidateRequestError::TargetMismatch {
				artifact: identity.id,
			})
		}
	}
}
