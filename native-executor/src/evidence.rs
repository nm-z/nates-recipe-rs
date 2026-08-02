use recipe_core::DeviceId;

/// Native driver family that realized one participating device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeBackendKind {
	Cuda,
	Hsa,
}

/// Bounded evidence derived from the real resources retained immediately
/// before native teardown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeDeviceExecutionEvidence {
	pub device: DeviceId,
	pub backend: NativeBackendKind,
	pub image_loads: usize,
	pub entry_lookups: usize,
	pub queues: usize,
	pub completion_objects: usize,
	pub persistent_allocations: usize,
}

/// Complete production-native realization and teardown evidence for one run.
///
/// The running backend exposes no compiler, loader, queue creator, completion
/// creator, or allocator operation. `loop_realization_calls` therefore counts
/// any such call admitted after handoff; it must remain zero. A successful
/// report is returned only after every retained native resource was destroyed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeExecutionEvidence {
	devices: Vec<NativeDeviceExecutionEvidence>,
	loop_realization_calls: u64,
	teardown_completed: bool,
	live_resources_after_teardown: usize,
}

impl NativeExecutionEvidence {
	pub(crate) fn completed(devices: Vec<NativeDeviceExecutionEvidence>) -> Self {
		Self {
			devices,
			loop_realization_calls: 0,
			teardown_completed: true,
			live_resources_after_teardown: 0,
		}
	}

	#[must_use]
	pub fn devices(&self) -> &[NativeDeviceExecutionEvidence] { &self.devices }

	#[must_use]
	pub const fn loop_realization_calls(&self) -> u64 { self.loop_realization_calls }

	#[must_use]
	pub const fn teardown_completed(&self) -> bool { self.teardown_completed }

	#[must_use]
	pub const fn live_resources_after_teardown(&self) -> usize { self.live_resources_after_teardown }
}
