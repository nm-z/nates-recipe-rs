use core::fmt;

use recipe_core::{DeviceId, TaskId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScheduleErrorKind { InvalidTopology, InvalidDiscovery, DuplicateTask, UnknownDependency, DependencyCycle,
	InvalidLifecycleDependency, UnavailableCapability, InvalidCalculationPlacement, InvalidTransfer, NoRoute,
	ArithmeticOverflow, InsufficientCapacity, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleError { pub kind: ScheduleErrorKind, pub task: Option<TaskId>, pub device: Option<DeviceId>,
	pub message: String, }

impl ScheduleError {
	#[must_use]
	pub fn new(kind: ScheduleErrorKind, message: impl Into<String>) -> Self { Self { kind, task: None, device: None,
			message: message.into(), } }

	#[must_use]
	pub fn for_task(mut self, task: TaskId) -> Self { self.task = Some(task); self }

	#[must_use]
	pub fn for_device(mut self, device: DeviceId) -> Self { self.device = Some(device); self } }

impl fmt::Display for ScheduleError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}", self.kind)?;
		if let Some(task) = self.task {
			write!(formatter, " for task {task}")?;
		}
		if let Some(device) = self.device {
			write!(formatter, " on device {device}")?;
		}
		write!(formatter, ": {}", self.message)
	} }

impl std::error::Error for ScheduleError {}
