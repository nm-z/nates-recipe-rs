use core::fmt;

use recipe_core::{KernelTemplateId, ValueId};
use recipe_language::LanguageError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoweringErrorKind {
	InvalidLanguage,
	MissingTensor,
	ArithmeticOverflow,
	InvalidStaticAccess,
	InvalidLoweredProgram,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweringError {
	pub kind: LoweringErrorKind,
	pub detail: String,
	pub kernel: Option<KernelTemplateId>,
	pub value: Option<ValueId>,
}

impl LoweringError {
	#[must_use]
	pub fn new(kind: LoweringErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
			kernel: None,
			value: None,
		}
	}

	#[must_use]
	pub const fn for_kernel(mut self, kernel: KernelTemplateId) -> Self {
		self.kernel = Some(kernel);
		self
	}

	#[must_use]
	pub const fn for_value(mut self, value: ValueId) -> Self {
		self.value = Some(value);
		self
	}
}

impl From<LanguageError> for LoweringError {
	fn from(error: LanguageError) -> Self {
		Self {
			kind: LoweringErrorKind::InvalidLanguage,
			detail: error.detail,
			kernel: error.kernel,
			value: error.value,
		}
	}
}

impl fmt::Display for LoweringError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		match self.kernel {
			Some(kernel) => write!(formatter, " [kernel {kernel}]")?,
			None => formatter.write_str("")?,
		}
		match self.value {
			Some(value) => write!(formatter, " [value {value}]")?,
			None => formatter.write_str("")?,
		}
		Ok(())
	}
}

impl std::error::Error for LoweringError {}

pub type LoweringResult<T> = Result<T, LoweringError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramValidationError {
	pub path: String,
	pub detail: String,
}

impl ProgramValidationError {
	#[must_use]
	pub fn new(path: impl Into<String>, detail: impl Into<String>) -> Self {
		Self {
			path: path.into(),
			detail: detail.into(),
		}
	}
}

impl fmt::Display for ProgramValidationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{}: {}", self.path, self.detail)
	}
}
