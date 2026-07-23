use core::fmt;

use recipe_core::{KernelTemplateId, ValueId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LanguageErrorKind {
	EmptyShape,
	InvalidAxis,
	DuplicateAxis,
	ShapeOverflow,
	ByteSizeOverflow,
	InvalidLayout,
	DuplicateTensor,
	DuplicateKernel,
	UnknownTensor,
	DuplicateProducer,
	MissingProducer,
	Cycle,
	ArityMismatch,
	DTypeMismatch,
	ShapeMismatch,
	InvalidScalarProgram,
	InvalidPrimitive,
	WorkOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageError {
	pub kind: LanguageErrorKind,
	pub detail: String,
	pub value: Option<ValueId>,
	pub kernel: Option<KernelTemplateId>,
}

impl LanguageError {
	#[must_use]
	pub fn new(kind: LanguageErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
			value: None,
			kernel: None,
		}
	}

	#[must_use]
	pub const fn for_value(mut self, value: ValueId) -> Self {
		self.value = Some(value);
		self
	}

	#[must_use]
	pub const fn for_kernel(mut self, kernel: KernelTemplateId) -> Self {
		self.kernel = Some(kernel);
		self
	}
}

impl fmt::Display for LanguageError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		if let Some(kernel) = self.kernel {
			write!(formatter, " [kernel {kernel}]")?;
		}
		if let Some(value) = self.value {
			write!(formatter, " [value {value}]")?;
		}
		Ok(())
	}
}

impl std::error::Error for LanguageError {}

pub type LanguageResult<T> = Result<T, LanguageError>;
