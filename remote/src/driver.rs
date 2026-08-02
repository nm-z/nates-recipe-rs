use recipe_core::{ByteCount, DeviceId, Digest, RunId, TaskId};

use crate::{model::ProvisionedProgram, session::RemoteMetricValue};

/// Allocation-free worker fault payload suitable for propagation in-loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DriverFault {
	pub code: u32,
	pub detail: u64,
}

impl DriverFault {
	#[must_use]
	pub const fn new(code: u32, detail: u64) -> Self { Self { code, detail } }

	#[must_use]
	pub fn cleanup_failed(primary: Self, cleanup: Self) -> Self {
		Self {
			code: DriverFaultCode::FATAL_CLEANUP_FAILED.get(),
			detail: u64::from(primary.code) << 32 | u64::from(cleanup.code),
		}
	}
}

/// Recipe-reserved worker-driver fault namespace.
///
/// Backend-native faults may use any other code. These values describe faults
/// raised by the remote/executor integration boundary itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DriverFaultCode(u32);

impl DriverFaultCode {
	pub const PROGRAM_IDENTITY_MISMATCH: Self = Self(0x5245_0002);
	pub const INVALID_LIFECYCLE: Self = Self(0x5245_0003);
	pub const FATAL_CLEANUP_FAILED: Self = Self(0x5245_0004);
	pub const PROGRAM_PROJECTION_MISMATCH: Self = Self(0x5245_0005);
	pub const EXECUTOR_PREPARE_FAILED: Self = Self(0x5245_0006);
	pub const EXECUTOR_OPERATION_FAILED: Self = Self(0x5245_0007);
	pub const EXECUTOR_CLEANUP_FAILED: Self = Self(0x5245_0008);

	#[must_use]
	pub const fn get(self) -> u32 { self.0 }
}

/// Nonblocking state of one already-submitted native worker task.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DriverPoll {
	Pending,
	Complete { metric: Option<RemoteMetricValue> },
}

/// Nonblocking state of one driver-owned byte transfer.
///
/// The byte count is checked against the finalized transfer before Recipe
/// advances the corresponding protocol task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverTransferPoll {
	Pending,
	Complete { bytes: usize },
}

/// Local prepared-executor boundary used by a remote worker.
///
/// The protocol supplies only finalized task/device identifiers and bounded
/// data slices. There is deliberately no host calculation callback, compiler,
/// discovery hook, file operation, or arbitrary closure surface.
pub trait WorkerDriver: core::fmt::Debug {
	/// Bind and pre-realize the local executor before any init image is admitted.
	fn prepare(&mut self, run: RunId, program: &ProvisionedProgram) -> Result<(), DriverFault>;

	fn begin_init_image(&mut self, device: DeviceId, bytes: ByteCount, digest: Digest) -> Result<(), DriverFault>;

	/// Submit one image chunk from storage that remains stable until the
	/// matching poll reports completion.
	fn begin_init_chunk(&mut self, device: DeviceId, offset: u64, bytes: &[u8]) -> Result<(), DriverFault>;

	fn poll_init_chunk(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault>;

	fn finish_init_image(&mut self, device: DeviceId) -> Result<(), DriverFault>;

	/// Poll the final asynchronous device admission submitted by
	/// [`Self::finish_init_image`].
	fn poll_init_image(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault>;

	/// Submit one provisioned GPU-native task without host-side calculation.
	fn submit_task(&mut self, task: TaskId) -> Result<(), DriverFault>;

	fn poll_task(&mut self, task: TaskId) -> Result<DriverPoll, DriverFault>;

	/// Submit a master-to-worker transfer from storage that remains stable
	/// until the matching poll reports completion.
	fn begin_receive_user_data(&mut self, task: TaskId, bytes: &[u8]) -> Result<(), DriverFault>;

	fn poll_receive_user_data(&mut self, task: TaskId) -> Result<DriverTransferPoll, DriverFault>;

	/// Submit a worker-to-master transfer into caller-owned storage that
	/// remains stable until the matching poll reports completion.
	fn begin_produce_user_data(&mut self, task: TaskId, destination: &mut [u8]) -> Result<(), DriverFault>;

	fn poll_produce_user_data(&mut self, task: TaskId) -> Result<DriverTransferPoll, DriverFault>;

	fn user_data_acked(&mut self, task: TaskId) -> Result<(), DriverFault>;

	fn cancel(&mut self, reason: u32) -> Result<(), DriverFault>;

	fn begin_exit(&mut self) -> Result<(), DriverFault>;

	fn release_arena(&mut self, device: DeviceId) -> Result<(), DriverFault>;

	fn finish(&mut self) -> Result<(), DriverFault>;

	/// Quiesce every active operation and release every locally realized
	/// resource after a terminal driver fault.
	///
	/// This path has no protocol-level per-arena acknowledgment: the worker
	/// sends its terminal `DriverFault` only after this method succeeds. The
	/// implementation must therefore be deterministic, idempotent with respect
	/// to resources it has already released, and return only after native work
	/// can no longer access an arena.
	fn cleanup_after_fault(&mut self, primary: DriverFault) -> Result<(), DriverFault>;
}
