use core::fmt;

use recipe_core::{ArtifactId, CompletionSlotId, DeviceId, QueueSlotId, TaskId, ValueId};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
	DuplicateDevice {
		device: DeviceId,
	},
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
	MissingDevice {
		device: DeviceId,
	},
	UnexpectedDevice {
		device: DeviceId,
	},
	MissingQueue {
		task: TaskId,
		queue: QueueSlotId,
	},
	MissingCompletion {
		task: TaskId,
		completion: CompletionSlotId,
	},
	ResourceContention {
		task: TaskId,
		detail: &'static str,
	},
	CompletionBusy {
		backend: &'static str,
		task: TaskId,
		completion: CompletionSlotId,
		owner: TaskId,
	},
	ArenaMismatch {
		device: DeviceId,
		detail: &'static str,
	},
	ValueMismatch {
		value: ValueId,
		detail: &'static str,
	},
	UnsupportedTransfer {
		task: TaskId,
		detail: &'static str,
	},
	UnsupportedLoopContract {
		backend: &'static str,
		detail: &'static str,
	},
	BackendState {
		backend: &'static str,
		detail: &'static str,
	},
	BackendPoisoned {
		backend: &'static str,
	},
	SubmissionQueueLimitExceeded {
		backend: &'static str,
		device: DeviceId,
		requested: usize,
		maximum: u32,
	},
	IntegerOverflow {
		field: &'static str,
	},
	PhysicalAccountingOverflow,
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
	Protocol {
		task: TaskId,
		detail: &'static str,
	},
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DuplicateDevice { device } => {
				write!(formatter, "native device {device} appears more than once")
			}
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
			Self::MissingDevice { device } => write!(formatter, "native device {device} is missing"),
			Self::UnexpectedDevice { device } => {
				write!(formatter, "native device {device} has no finalized arena")
			}
			Self::MissingQueue { task, queue } => {
				write!(
					formatter,
					"task {task} references unavailable queue slot {queue}"
				)
			}
			Self::MissingCompletion { task, completion } => {
				write!(
					formatter,
					"task {task} references unavailable completion slot {completion}"
				)
			}
			Self::ResourceContention { task, detail } => {
				write!(
					formatter,
					"task {task} has native resource contention: {detail}"
				)
			}
			Self::CompletionBusy {
				backend,
				task,
				completion,
				owner,
			} => write!(
				formatter,
				"{backend} task {task} cannot claim completion slot {completion}; task {owner} owns it"
			),
			Self::ArenaMismatch { device, detail } => {
				write!(
					formatter,
					"arena for device {device} is incompatible: {detail}"
				)
			}
			Self::ValueMismatch { value, detail } => {
				write!(
					formatter,
					"resolved value {value} is incompatible: {detail}"
				)
			}
			Self::UnsupportedTransfer { task, detail } => {
				write!(formatter, "transfer task {task} is unsupported: {detail}")
			}
			Self::UnsupportedLoopContract { backend, detail } => {
				write!(
					formatter,
					"{backend} cannot uphold the strict loop contract: {detail}"
				)
			}
			Self::BackendState { backend, detail } => {
				write!(formatter, "{backend} backend state is invalid: {detail}")
			}
			Self::BackendPoisoned { backend } => {
				write!(formatter, "{backend} backend is poisoned")
			}
			Self::SubmissionQueueLimitExceeded {
				backend,
				device,
				requested,
				maximum,
			} => write!(
				formatter,
				"{backend} device {device} requires {requested} physical submission queues, exceeding its discovered maximum of {maximum}"
			),
			Self::IntegerOverflow { field } => write!(formatter, "{field} does not fit the native ABI"),
			Self::PhysicalAccountingOverflow => write!(
				formatter,
				"native backend exceeded the fixed physical-call accounting capacity"
			),
			Self::Cuda(detail) => write!(formatter, "CUDA Driver operation failed: {detail}"),
			Self::CudaContract(detail) => {
				write!(formatter, "CUDA Driver resource contract failed: {detail}")
			}
			Self::HsaSymbolLookup {
				artifact,
				abi_entry,
				runtime_symbol,
				source,
			} => write!(
				formatter,
				"HSACO artifact {artifact} logical ABI entry {abi_entry:?} failed ROCr lookup for descriptor symbol {runtime_symbol:?}: {source}"
			),
			Self::Hsa(detail) => write!(formatter, "ROCr/HSA operation failed: {detail}"),
			Self::Kernel(detail) => write!(formatter, "kernel artifact validation failed: {detail}"),
			Self::Protocol { task, detail } => write!(
				formatter,
				"task {task} violates the native protocol: {detail}"
			),
		}
	}
}

impl std::error::Error for Error {}

impl From<recipe_cuda::CudaError> for Error {
	fn from(error: recipe_cuda::CudaError) -> Self {
		Self::Cuda(error)
	}
}

impl From<recipe_hsa::Error> for Error {
	fn from(error: recipe_hsa::Error) -> Self {
		Self::Hsa(error)
	}
}

impl From<recipe_kernel::LoweringError> for Error {
	fn from(error: recipe_kernel::LoweringError) -> Self {
		Self::Kernel(error)
	}
}

pub(crate) fn ensure_submission_queue_capacity(
	backend: &'static str,
	device: DeviceId,
	requested: usize,
	maximum: u32,
) -> Result<()> {
	if requested <= maximum as usize {
		Ok(())
	} else {
		Err(Error::SubmissionQueueLimitExceeded {
			backend,
			device,
			requested,
			maximum,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn submission_queue_capacity_accepts_exact_limit() {
		assert_eq!(
			ensure_submission_queue_capacity("ROCr/HSA", DeviceId::new(7), 4, 4),
			Ok(())
		);
	}

	#[test]
	fn submission_queue_capacity_returns_typed_error_above_limit() {
		assert_eq!(
			ensure_submission_queue_capacity("ROCr/HSA", DeviceId::new(7), 5, 4),
			Err(Error::SubmissionQueueLimitExceeded {
				backend: "ROCr/HSA",
				device: DeviceId::new(7),
				requested: 5,
				maximum: 4,
			})
		);
	}
}
