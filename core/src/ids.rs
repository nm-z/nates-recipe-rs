use core::fmt;

macro_rules! stable_ids {
	($($name:ident),+ $(,)?) => {
		$(
			#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
			pub struct $name(u64);

			impl $name {
				#[must_use]
				pub const fn new(value: u64) -> Self {
					Self(value)
				}

				#[must_use]
				pub const fn get(self) -> u64 {
					self.0
				}
			}

			impl fmt::Display for $name {
				fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
					self.0.fmt(formatter)
				}
			}
		)+
	};
}

stable_ids!(
	MachineId,
	NodeId,
	DeviceId,
	LinkId,
	TransportId,
	DuplexResourceId,
	ValueId,
	ScalarValueId,
	KernelTemplateId,
	KernelInputId,
	KernelOutputId,
	TaskId,
	ArtifactId,
	QueueSlotId,
	CompletionSlotId,
	MetricId,
	MetricSlotId,
	ArenaObjectId,
	RunId,
);
