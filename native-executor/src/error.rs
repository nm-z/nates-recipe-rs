use core::fmt;

use recipe_core::{ArtifactId, DeviceId, TaskId};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
	DuplicateArtifact {
		artifact: ArtifactId,
	},
	MissingArtifact {
		artifact: ArtifactId,
	},
	UnexpectedArtifact {
		artifact: ArtifactId,
	},
	ArtifactMismatch {
		artifact: ArtifactId,
		detail: String,
	},
	ResourceContention {
		task: TaskId,
		detail: &'static str,
	},
	ArenaMismatch {
		device: DeviceId,
		detail: &'static str,
	},
	BackendState {
		backend: &'static str,
		detail: &'static str,
	},
	BackendPoisoned {
		backend: &'static str,
	},
	IntegerOverflow {
		field: &'static str,
	},
	Cuda(recipe_cuda::CudaError),
	CudaContract(&'static str),
	HsaSymbolLookup {
		artifact: ArtifactId,
		abi_entry: String,
		runtime_symbol: String,
		source: recipe_hsa::Error,
	},
	Hsa(recipe_hsa::Error),
	Kernel(recipe_kernel::LoweringError),
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DuplicateArtifact { artifact } => {
				write!(
					formatter,
					"runtime artifact {artifact} appears more than once"
				)
			}
			Self::MissingArtifact { artifact } => write!(formatter, "runtime artifact {artifact} is missing"),
			Self::UnexpectedArtifact { artifact } => {
				write!(
					formatter,
					"runtime artifact {artifact} is not present in the finalized bundle"
				)
			}
			Self::ArtifactMismatch { artifact, detail } => {
				write!(
					formatter,
					"runtime artifact {artifact} is incompatible: {detail}"
				)
			}
			Self::ResourceContention { task, detail } => {
				write!(
					formatter,
					"task {task} has native resource contention: {detail}"
				)
			}
			Self::ArenaMismatch { device, detail } => {
				write!(
					formatter,
					"arena for device {device} is incompatible: {detail}"
				)
			}
			Self::BackendState { backend, detail } => {
				write!(formatter, "{backend} backend state is invalid: {detail}")
			}
			Self::BackendPoisoned { backend } => {
				write!(formatter, "{backend} backend is poisoned")
			}
			Self::IntegerOverflow { field } => write!(formatter, "{field} does not fit the native ABI"),
			Self::Cuda(detail) => write!(formatter, "CUDA Driver operation failed: {detail}"),
			Self::CudaContract(detail) => {
				write!(formatter, "CUDA Driver resource contract failed: {detail}")
			}
			Self::HsaSymbolLookup {
				artifact,
				abi_entry,
				runtime_symbol,
				source,
			} => {
				write!(
					formatter,
					"HSACO artifact {artifact} logical ABI entry {abi_entry:?} failed ROCr lookup for descriptor symbol {runtime_symbol:?}: {source}"
				)
			}
			Self::Hsa(detail) => write!(formatter, "ROCr/HSA operation failed: {detail}"),
			Self::Kernel(detail) => write!(formatter, "kernel artifact validation failed: {detail}"),
		}
	}
}

impl std::error::Error for Error {}

impl From<recipe_cuda::CudaError> for Error {
	fn from(error: recipe_cuda::CudaError) -> Self { Self::Cuda(error) }
}

impl From<recipe_hsa::Error> for Error {
	fn from(error: recipe_hsa::Error) -> Self { Self::Hsa(error) }
}

impl From<recipe_kernel::LoweringError> for Error {
	fn from(error: recipe_kernel::LoweringError) -> Self { Self::Kernel(error) }
}
