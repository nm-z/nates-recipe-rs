use core::fmt;

pub type Result<T> = core::result::Result<T, CudaError>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DriverStatus(pub i32);

impl fmt::Display for DriverStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "CUDA status {}", self.0)
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverCallError {
	pub operation: &'static str,
	pub status: DriverStatus,
	pub name: Option<String>,
	pub description: Option<String>,
}

impl fmt::Display for DriverCallError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} failed with {}", self.operation, self.status)?;
		if let Some(name) = &self.name {
			write!(f, " ({name})")?;
		}
		if let Some(description) = &self.description {
			write!(f, ": {description}")?;
		}
		Ok(())
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CudaError {
	LibraryOpen {
		attempts: Vec<String>,
	},
	InvalidLibraryPath {
		path: String,
	},
	MissingRequiredSymbol {
		symbol: &'static str,
		detail: Option<String>,
	},
	DriverCall(DriverCallError),
	InvalidDriverValue {
		operation: &'static str,
		detail: String,
	},
	InvalidInput {
		operation: &'static str,
		detail: String,
	},
	InvalidDeviceName {
		ordinal: i32,
		detail: String,
	},
	DuplicateDeviceUuid {
		first_ordinal: i32,
		second_ordinal: i32,
	},
	InvalidContextFlags {
		bits: u32,
	},
	ContextStackMismatch,
	ContextClosed,
	ResourceContextMismatch {
		operation: &'static str,
	},
}

impl fmt::Display for CudaError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::LibraryOpen { attempts } => {
				write!(f, "unable to open a CUDA Driver library")?;
				for attempt in attempts {
					write!(f, "; {attempt}")?;
				}
				Ok(())
			}
			Self::InvalidLibraryPath { path } => {
				write!(f, "CUDA Driver library path contains NUL: {path:?}")
			}
			Self::MissingRequiredSymbol { symbol, detail } => {
				write!(f, "CUDA Driver is missing required symbol {symbol}")?;
				if let Some(detail) = detail {
					write!(f, ": {detail}")?;
				}
				Ok(())
			}
			Self::DriverCall(error) => error.fmt(f),
			Self::InvalidDriverValue { operation, detail } => {
				write!(f, "{operation} returned an invalid value: {detail}")
			}
			Self::InvalidInput { operation, detail } => {
				write!(f, "invalid input for {operation}: {detail}")
			}
			Self::InvalidDeviceName { ordinal, detail } => {
				write!(
					f,
					"CUDA device {ordinal} returned an invalid name: {detail}"
				)
			}
			Self::DuplicateDeviceUuid {
				first_ordinal,
				second_ordinal,
			} => write!(
				f,
				"CUDA devices {first_ordinal} and {second_ordinal} returned the same UUID"
			),
			Self::InvalidContextFlags { bits } => {
				write!(f, "invalid R470-safe CUDA context flag set 0x{bits:08x}")
			}
			Self::ContextStackMismatch => {
				f.write_str("CUDA context stack returned a different context than Recipe pushed")
			}
			Self::ContextClosed => f.write_str("CUDA context is already closed"),
			Self::ResourceContextMismatch { operation } => {
				write!(
					f,
					"{operation} received resources from different CUDA contexts"
				)
			}
		}
	}
}

impl std::error::Error for CudaError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::DriverCall(error) => Some(error),
			_ => None,
		}
	}
}

impl std::error::Error for DriverCallError {}
