use core::fmt;

use crate::OperationId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationErrorKind { UnknownOperation, AmbiguousSymbol, UnsupportedLowering, WrongLoweringKind,
	PrimitiveRecipeMismatch, InvalidScalarProgram, PrimitiveLoweringFailed, InvalidCompositionRecipe,
	InvalidMaterializationRequest, MissingPreparedParameter, PreparedParameterTypeMismatch, IterationBoundUnresolved,
	CompositionExpansionOverflow, MissingConcreteFormula, UnsupportedConcreteShape, IdentityNamespaceOverlap,
	IdentityNamespaceExhausted, WorkspaceLimitExceeded, GraphMaterializationFailed, WorkspaceFormulaMismatch,
	WorkspaceArithmeticOverflow, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationError { pub kind: OperationErrorKind, pub detail: String, pub operation: Option<OperationId>, }

impl OperationError {
	#[must_use]
	pub fn new(kind: OperationErrorKind, detail: impl Into<String>) -> Self { Self { kind, detail: detail.into(),
			operation: None, } }

	#[must_use]
	pub const fn for_operation(mut self, operation: OperationId) -> Self { self.operation = Some(operation); self } }

impl fmt::Display for OperationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		if let Some(operation) = self.operation {
			write!(formatter, " [operation {}]", operation.ordinal())?;
		}
		Ok(()) } }

impl std::error::Error for OperationError {}

pub type OperationResult<T> = Result<T, OperationError>;
