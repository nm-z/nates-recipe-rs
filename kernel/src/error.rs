use core::fmt;

use recipe_core::ScalarValueId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoweringErrorKind { InvalidKernel, InvalidStageContract, InvalidEntrySymbol, InvalidTarget,
	InvalidWorkgroupSize, UnknownScalarValue, UnsupportedOperation, ArithmeticOverflow, ProhibitedInterface,
	ArtifactFormat, ArtifactMismatch, ToolchainFailed, Io, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweringError { pub kind: LoweringErrorKind, pub scalar: Option<ScalarValueId>, pub message: String, }

impl LoweringError {
	#[must_use]
	pub fn new(kind: LoweringErrorKind, message: impl Into<String>) -> Self { Self { kind, scalar: None,
			message: message.into(), } }

	#[must_use]
	pub fn for_scalar(mut self, scalar: ScalarValueId) -> Self { self.scalar = Some(scalar); self } }

impl fmt::Display for LoweringError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}", self.kind)?;
		match self.scalar {
			Some(scalar) => write!(formatter, " for scalar {scalar}"),
			None => Ok(()), }?;
		write!(formatter, ": {}", self.message)
	} }

impl std::error::Error for LoweringError {}
