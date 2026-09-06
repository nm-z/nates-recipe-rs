use core::fmt;

/// Machine-readable category for a contract validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ValidationCode {
	EmptyName,
	InvalidIdentity,
	DuplicateId,
	DuplicateName,
	UnknownReference,
	WrongKind,
	MissingMaster,
	MultipleMasters,
	UnownedDevice,
	DeviceOwnedMultipleTimes,
	MachineMismatch,
	MissingRequiredObject,
	UnavailableRequiredObject,
	UnsupportedCapability,
	UnmeasuredProperty,
	InvalidRoute,
	InvalidLaneClaim,
	InvalidTransport,
	InvalidDuplex,
	RouteEndpointMismatch,
	MissingAliasRule,
	DuplicateAliasRule,
	UnknownScalarValue,
	ScalarUseBeforeDefinition,
	ScalarArity,
	ScalarTypeMismatch,
	DuplicateScalarValue,
	MissingScalarOutput,
	InvalidMemoryAccess,
	DependencyCycle,
	InvalidPhase,
	DependencyPhaseOrder,
	DependencyScheduleOrder,
	InvalidCalculationPlacement,
	InvalidIterationDomain,
	InvalidFaultReadback,
	InvalidExternalTransfer,
	InvalidDataImage,
	MissingUpload,
	DuplicateUpload,
	MissingRelease,
	DuplicateRelease,
	DuplicateReservation,
	MissingReservation,
	WrongReservationSize,
	InvalidReservationName,
	CapacityOverflow,
	InsufficientCapacity,
	IdentityMismatch,
	ArtifactMismatch,
	ResourceMismatch,
	ResourceContention,
	CapabilityConcurrencyExceeded,
	DuplicateAllocation,
	MissingAllocation,
	AllocationOutOfBounds,
	AllocationMisaligned,
	DuplicateValueBinding,
	MissingValueBinding,
	ValueBindingOutOfBounds,
	ValueBindingMisaligned,
	ValueLifetimeMismatch,
	AliasViolation,
	AddressOverflow,
	LiveAllocationOverlap,
	InvalidLifetime,
}

/// One precise validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
	pub code: ValidationCode,
	pub path: String,
	pub message: String,
}

impl ValidationError {
	#[must_use]
	pub fn new(code: ValidationCode, path: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			code,
			path: path.into(),
			message: message.into(),
		}
	}
}

/// All failures found during one validation pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationErrors {
	errors: Vec<ValidationError>,
}

impl ValidationErrors {
	#[must_use]
	pub fn as_slice(&self) -> &[ValidationError] { &self.errors }

	#[must_use]
	pub fn into_vec(self) -> Vec<ValidationError> { self.errors }

	#[must_use]
	pub fn contains(&self, code: ValidationCode) -> bool { self.errors.iter().any(|error| error.code == code) }
}

impl fmt::Display for ValidationErrors {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		for (index, error) in self.errors.iter().enumerate() {
			if index != 0 {
				formatter.write_str("; ")?;
			}
			write!(
				formatter,
				"{:?} at {}: {}",
				error.code, error.path, error.message
			)?;
		}
		Ok(())
	}
}

impl std::error::Error for ValidationErrors {}

pub type ValidationResult<T = ()> = Result<T, ValidationErrors>;

#[derive(Debug, Default)]
pub(crate) struct Validator {
	errors: Vec<ValidationError>,
}

impl Validator {
	pub(crate) fn require(&mut self, condition: bool, code: ValidationCode, path: impl Into<String>, message: impl Into<String>) {
		if !condition {
			self.errors.push(ValidationError::new(code, path, message));
		}
	}

	pub(crate) fn error(&mut self, code: ValidationCode, path: impl Into<String>, message: impl Into<String>) { self.errors.push(ValidationError::new(code, path, message)); }

	pub(crate) fn append(&mut self, prefix: &str, errors: ValidationErrors) {
		for mut error in errors.into_vec() {
			error.path = if error.path.is_empty() {
				prefix.to_owned()
			} else {
				format!("{prefix}.{}", error.path)
			};
			self.errors.push(error);
		}
	}

	pub(crate) fn finish(self) -> ValidationResult {
		if self.errors.is_empty() {
			Ok(())
		} else {
			Err(ValidationErrors {
				errors: self.errors,
			})
		}
	}

	pub(crate) fn into_errors(self) -> Option<ValidationErrors> {
		if self.errors.is_empty() {
			None
		} else {
			Some(ValidationErrors {
				errors: self.errors,
			})
		}
	}
}
