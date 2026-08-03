use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlannerErrorKind { InvalidGraph, InvalidTopology, InvalidDiscovery, InvalidReservation, InvalidCapacity,
	InvalidArtifact, InvalidDraft, MissingArtifact, NoCalculationDevice, NoRoute, DependencyConflict, CandidateInfeasible,
	NoViableCandidate, Schedule, ArithmeticOverflow, IdentityCollision, UnknownCandidate, AlreadyRejected, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerError { pub kind: PlannerErrorKind, pub message: String, }

impl PlannerError {
	#[must_use]
	pub fn new(kind: PlannerErrorKind, message: impl Into<String>) -> Self { Self { kind, message: message.into(), } } }

impl fmt::Display for PlannerError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.message)
	} }

impl std::error::Error for PlannerError {}

pub type PlannerResult<T> = Result<T, PlannerError>;
