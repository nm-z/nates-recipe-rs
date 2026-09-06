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

impl fmt::Display for TrainingCompileErrorKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		return f.write_str(match *self {
			Self::EmptyDataset => "empty dataset",
			Self::InconsistentRows => "inconsistent rows",
			Self::InvalidFeatureMatrix => "invalid feature matrix",
			Self::InvalidTargetMatrix => "invalid target matrix",
			Self::InvalidNetwork => "invalid network",
			Self::InvalidOptimizer => "invalid optimizer",
			Self::UnsupportedExtent => "unsupported extent",
			Self::ArithmeticOverflow => "arithmetic overflow",
			Self::IdentityExhausted => "identity exhausted",
			Self::Ingest => "ingest",
			Self::Language => "language",
			Self::Operation => "operation",
			Self::Program => "program",
			Self::Ogdl => "OGDL",
		});
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingCompileError {
	pub kind: TrainingCompileErrorKind,
	pub detail: String,
}

impl TrainingCompileError {
	/// Constructs a new value.
	pub(crate) fn new(kind: TrainingCompileErrorKind, detail: impl Into<String>) -> Self {
		return Self {
			kind,
			detail: detail.into(),
		};
	}
}

impl fmt::Display for TrainingCompileError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { return write!(f, "{}: {}", self.kind, self.detail); }
}

impl core::error::Error for TrainingCompileError {}

impl From<PrepareError> for TrainingCompileError {
	#[inline]
	fn from(error: PrepareError) -> Self { return Self::new(TrainingCompileErrorKind::Ingest, error.to_string()); }
}

impl From<LanguageError> for TrainingCompileError {
	#[inline]
	fn from(error: LanguageError) -> Self { return Self::new(TrainingCompileErrorKind::Language, error.to_string()); }
}

impl From<OperationError> for TrainingCompileError {
	#[inline]
	fn from(error: OperationError) -> Self { return Self::new(TrainingCompileErrorKind::Operation, error.to_string()); }
}

impl From<ProgramError> for TrainingCompileError {
	#[inline]
	fn from(error: ProgramError) -> Self { return Self::new(TrainingCompileErrorKind::Program, error.to_string()); }
}

impl From<OgdlCodecError> for TrainingCompileError {
	#[inline]
	fn from(error: OgdlCodecError) -> Self { return Self::new(TrainingCompileErrorKind::Ogdl, error.to_string()); }
}

pub type TrainingCompileResult<T> = Result<T, TrainingCompileError>;
