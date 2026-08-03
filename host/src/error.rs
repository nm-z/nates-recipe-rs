use core::fmt; use std::io;

use recipe_core::{DeviceId, TaskId};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
	InvalidConfiguration(&'static str),
	InvalidPath, DuplicateDevice(DeviceId), MissingDevice(DeviceId), WrongDeviceKind(DeviceId),
	BackendState(&'static str),
	Protocol { task: TaskId,
		detail: &'static str,
	}, UnsupportedWork { task: TaskId,
		detail: &'static str,
	}, PhysicalReportOverflow, RangeOverflow, OutOfBounds { device: DeviceId, }, SlotCapacityExhausted, SlotBusy,
	InvalidPendingState, WorkerFailed {
		operation: &'static str,
		kind: io::ErrorKind, }, Io {
		operation: &'static str,
		kind: io::ErrorKind, }, ThreadSpawn(io::ErrorKind), ThreadPanicked, Poisoned, }

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self { Self::InvalidConfiguration(detail) => {
				write!(formatter, "invalid host runtime configuration: {detail}")
			}
			Self::InvalidPath => formatter.write_str("host storage path has no usable file name"),
			Self::DuplicateDevice(device) => write!(formatter, "host device {device} appears more than once"),
			Self::MissingDevice(device) => write!(formatter, "host device {device} is not bound"),
			Self::WrongDeviceKind(device) => write!(formatter, "host device {device} has the wrong storage kind"),
			Self::BackendState(detail) => write!(formatter, "invalid host backend state: {detail}"),
			Self::Protocol { task, detail } => { write!( formatter,
					"host backend protocol violation for task {task}: {detail}"
				) }
			Self::UnsupportedWork { task, detail } => { write!( formatter,
					"host backend cannot execute task {task}: {detail}"
				) }
			Self::PhysicalReportOverflow => {
				formatter.write_str("host physical-call report exceeded its fixed capacity")
			}
			Self::RangeOverflow => formatter.write_str("host transfer range overflowed"),
			Self::OutOfBounds { device } => write!(formatter, "host transfer exceeds arena {device}"),
			Self::SlotCapacityExhausted => formatter.write_str("preallocated host job slots are exhausted"),
			Self::SlotBusy => formatter.write_str("preallocated host job slot is busy"),
			Self::InvalidPendingState => formatter.write_str("host pending token is in the wrong state"),
			Self::WorkerFailed { operation, kind } => {
				write!(formatter, "host worker {operation} failed with {kind:?}")
			}
			Self::Io { operation, kind } => write!(formatter, "host {operation} failed with {kind:?}"),
			Self::ThreadSpawn(kind) => write!(formatter, "host worker creation failed with {kind:?}"),
			Self::ThreadPanicked => formatter.write_str("host worker thread panicked"),
			Self::Poisoned => formatter.write_str("host runtime is poisoned"),
		} } }

impl std::error::Error for Error {}

pub(crate) fn io(operation: &'static str, error: &io::Error) -> Error {
	Error::Io { operation, kind: error.kind(), } }
