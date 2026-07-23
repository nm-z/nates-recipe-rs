use core::fmt;

use recipe_core::{DeviceId, RunId, TaskId};

pub type RemoteResult<T> = Result<T, RemoteError>;

#[derive(Debug)]
#[non_exhaustive]
pub enum RemoteError {
	InvalidConfiguration(&'static str),
	ManifestMismatch(&'static str),
	Protocol { detail: &'static str },
	RunMismatch { expected: RunId, actual: RunId },
	UnknownTask(TaskId),
	DuplicateTask(TaskId),
	UnknownDevice(DeviceId),
	DuplicateDevice(DeviceId),
	CapacityExhausted(&'static str),
	Backpressured(&'static str),
	Codec(&'static str),
	Driver(crate::DriverFault),
	Transport(recipe_transport::TransportError),
	Poisoned,
}

impl fmt::Display for RemoteError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidConfiguration(detail) => {
				write!(formatter, "invalid remote configuration: {detail}")
			}
			Self::ManifestMismatch(detail) => write!(formatter, "remote manifest mismatch: {detail}"),
			Self::Protocol { detail } => write!(formatter, "remote protocol violation: {detail}"),
			Self::RunMismatch { expected, actual } => {
				write!(
					formatter,
					"remote run {actual} differs from expected run {expected}"
				)
			}
			Self::UnknownTask(task) => write!(formatter, "remote task {task} is not provisioned"),
			Self::DuplicateTask(task) => write!(formatter, "remote task {task} appears more than once"),
			Self::UnknownDevice(device) => write!(formatter, "remote device {device} is not provisioned"),
			Self::DuplicateDevice(device) => {
				write!(formatter, "remote device {device} appears more than once")
			}
			Self::CapacityExhausted(resource) => write!(formatter, "remote {resource} capacity is exhausted"),
			Self::Backpressured(resource) => write!(formatter, "remote {resource} is backpressured"),
			Self::Codec(detail) => write!(formatter, "remote codec rejected a message: {detail}"),
			Self::Driver(fault) => write!(
				formatter,
				"worker driver fault {} with detail {}",
				fault.code, fault.detail
			),
			Self::Transport(error) => write!(formatter, "remote transport failed: {error}"),
			Self::Poisoned => formatter.write_str("remote session is poisoned"),
		}
	}
}

impl std::error::Error for RemoteError {}

impl From<recipe_transport::TransportError> for RemoteError {
	fn from(error: recipe_transport::TransportError) -> Self {
		Self::Transport(error)
	}
}
