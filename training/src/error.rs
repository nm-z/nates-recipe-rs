use core::fmt;

use recipe_ingest::PrepareError;
use recipe_language::{LanguageError, OgdlCodecError};
use recipe_ops::OperationError;
use recipe_program::ProgramError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrainingCompileErrorKind {
	EmptyDataset,
	InconsistentRows,
	InvalidFeatureMatrix,
	InvalidTargetMatrix,
	InvalidNetwork,
	InvalidOptimizer,
	UnsupportedExtent,
	ArithmeticOverflow,
	IdentityExhausted,
	Ingest,
	Language,
	Operation,
	Program,
	Ogdl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingCompileError {
	pub kind: TrainingCompileErrorKind,
	pub detail: String,
}

impl TrainingCompileError {
	pub(crate) fn new(kind: TrainingCompileErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for TrainingCompileError {
	#[inline]
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	}
}

impl core::error::Error for TrainingCompileError {}

impl From<PrepareError> for TrainingCompileError {
	#[inline]
	fn from(error: PrepareError) -> Self {
		Self::new(TrainingCompileErrorKind::Ingest, error.to_string())
	}
}

impl From<LanguageError> for TrainingCompileError {
	#[inline]
	fn from(error: LanguageError) -> Self {
		Self::new(TrainingCompileErrorKind::Language, error.to_string())
	}
}

impl From<OperationError> for TrainingCompileError {
	#[inline]
	fn from(error: OperationError) -> Self {
		Self::new(TrainingCompileErrorKind::Operation, error.to_string())
	}
}

impl From<ProgramError> for TrainingCompileError {
	#[inline]
	fn from(error: ProgramError) -> Self {
		Self::new(TrainingCompileErrorKind::Program, error.to_string())
	}
}

impl From<OgdlCodecError> for TrainingCompileError {
	#[inline]
	fn from(error: OgdlCodecError) -> Self {
		Self::new(TrainingCompileErrorKind::Ogdl, error.to_string())
	}
}

pub type TrainingCompileResult<T> = Result<T, TrainingCompileError>;
