use core::fmt::{self, Write};

use recipe_core::{ByteCount, DeviceId, LoopIterations, RunPhase, TaskId, ValueId};

pub const BACKEND_MESSAGE_CAPACITY: usize = 96;

/// Fixed-capacity backend diagnostic retained without allocating in the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendMessage {
	bytes: [u8; BACKEND_MESSAGE_CAPACITY],
	len: u16,
	truncated: bool,
}

impl BackendMessage {
	#[must_use]
	pub fn new(message: &str) -> Self {
		let mut result = Self::default();
		match result.write_str(message) {
			Ok(()) => result,
			Err(error) => {
				debug_assert!(result.truncated, "unexpected formatting error: {error}");
				result
			}
		}
	}

	#[must_use]
	pub fn as_str(&self) -> &str {
		match core::str::from_utf8(&self.bytes[..usize::from(self.len)]) {
			Ok(message) => message,
			Err(error) => unreachable!("BackendMessage accepts only valid UTF-8: {error}"),
		}
	}

	#[must_use]
	pub const fn was_truncated(&self) -> bool { self.truncated }
}

impl Default for BackendMessage {
	fn default() -> Self {
		Self {
			bytes: [0; BACKEND_MESSAGE_CAPACITY],
			len: 0,
			truncated: false,
		}
	}
}

impl Write for BackendMessage {
	fn write_str(&mut self, message: &str) -> fmt::Result {
		for character in message.chars() {
			let mut encoded = [0_u8; 4];
			let encoded = character.encode_utf8(&mut encoded).as_bytes();
			let start = usize::from(self.len);
			let end = match start.checked_add(encoded.len()) {
				Some(end) => end,
				None => {
					self.truncated = true;
					return Err(fmt::Error);
				}
			};
			match end <= BACKEND_MESSAGE_CAPACITY {
				true => {
					self.bytes[start..end].copy_from_slice(encoded);
					self.len = match u16::try_from(end) {
						Ok(len) => len,
						Err(error) => unreachable!("backend message capacity fits in u16: {error}"),
					};
				}
				false => {
					self.truncated = true;
					return Err(fmt::Error);
				}
			}
		}
		Ok(())
	}
}

impl fmt::Display for BackendMessage {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.as_str())?;
		match self.truncated {
			true => formatter.write_str("..."),
			false => Ok(()),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendOperation {
	BindResources,
	PreparePending { task: TaskId },
	AllocateArena { device: DeviceId },
	Submit { task: TaskId },
	Poll { task: TaskId },
	CollectExit { task: TaskId },
	ReleaseArena { device: DeviceId },
	DestroyResources,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutorError {
	Backend {
		operation: BackendOperation,
		message: BackendMessage,
	},
	DuplicateAdmission {
		device: DeviceId,
	},
	MissingAdmission {
		device: DeviceId,
	},
	UnexpectedAdmission {
		device: DeviceId,
	},
	AdmissionImageMismatch {
		device: DeviceId,
		expected: ValueId,
		actual: ValueId,
	},
	AdmissionSizeMismatch {
		device: DeviceId,
		expected: ByteCount,
		actual: ByteCount,
	},
	BackendProtocol {
		task: TaskId,
		detail: &'static str,
	},
	DeviceFault {
		readback: TaskId,
		value: ValueId,
		code: i32,
	},
	LifecycleInvariant {
		detail: &'static str,
	},
	WatchdogExpired {
		phase: RunPhase,
		nonprogress_polls: u32,
	},
	InvalidWatchdog {
		max_nonprogress_polls: u32,
	},
	LoopRepetitionUnsupported {
		iterations: LoopIterations,
	},
	MetricSequenceOverflow,
	ExitImageTooLarge {
		task: TaskId,
		bytes: ByteCount,
	},
	ExitImageAllocationFailed {
		task: TaskId,
		bytes: ByteCount,
	},
}

impl fmt::Display for ExecutorError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Backend { operation, message } => {
				write!(formatter, "backend {operation:?} failed: {message}")
			}
			Self::DuplicateAdmission { device } => {
				write!(
					formatter,
					"device {device} received more than one init image"
				)
			}
			Self::MissingAdmission { device } => {
				write!(formatter, "required device {device} has no init image")
			}
			Self::UnexpectedAdmission { device } => {
				write!(formatter, "init image targets unexpected device {device}")
			}
			Self::AdmissionImageMismatch {
				device,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"device {device} init image identifies value {actual}, expected {expected}"
				)
			}
			Self::AdmissionSizeMismatch {
				device,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"device {device} init image is {actual}, expected exactly {expected}"
				)
			}
			Self::BackendProtocol { task, detail } => {
				write!(
					formatter,
					"backend protocol violation for task {task}: {detail}"
				)
			}
			Self::DeviceFault {
				readback,
				value,
				code,
			} => {
				write!(
					formatter,
					"checked device work reported fault code {code} through value {value} at readback task {readback}"
				)
			}
			Self::LifecycleInvariant { detail } => {
				write!(formatter, "executor lifecycle invariant failed: {detail}")
			}
			Self::WatchdogExpired {
				phase,
				nonprogress_polls,
			} => {
				write!(
					formatter,
					"{phase:?} watchdog expired after {nonprogress_polls} nonprogress polls"
				)
			}
			Self::InvalidWatchdog {
				max_nonprogress_polls,
			} => {
				write!(
					formatter,
					"watchdog maximum must be nonzero, got {max_nonprogress_polls}"
				)
			}
			Self::LoopRepetitionUnsupported { iterations } => {
				write!(
					formatter,
					"backend does not support the finalized loop count of {}",
					iterations
						.finite()
						.map_or_else(|| "unbounded".to_owned(), |bound| bound.get().to_string())
				)
			}
			Self::MetricSequenceOverflow => formatter.write_str("metric sequence counter overflowed"),
			Self::ExitImageTooLarge { task, bytes } => {
				write!(
					formatter,
					"exit task {task} payload of {bytes} does not fit host address space"
				)
			}
			Self::ExitImageAllocationFailed { task, bytes } => {
				write!(
					formatter,
					"exit task {task} could not allocate its {bytes} result image"
				)
			}
		}
	}
}

impl std::error::Error for ExecutorError {}

pub type Result<T> = std::result::Result<T, ExecutorError>;
