use std::{error::Error, fmt};

use recipe_core::{ByteCount, DeviceId, Digest, FinalizedBundle, RunId, RunPhase, TaskId, Topology};
use recipe_executor::{
	ExternalTransferDirection, ExternalTransferPoll, MetricValue, Watchdog, WorkerAssignment, WorkerBackend,
	WorkerBackendOperation, WorkerExecutionError, WorkerExecutionSession, WorkerLifecycle, WorkerProjection,
	WorkerProjectionError, WorkerTaskPoll, WorkerTaskRole,
};

use crate::{
	DataDirection, DriverFault, DriverFaultCode, DriverPoll, DriverTransferPoll, ProvisionedProgram,
	RemoteMetricValue, WorkerDriver, model::ProgramTaskKind,
};

/// Static construction failure for the concrete executor-backed worker driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutorDriverBuildError {
	Projection(WorkerProjectionError),
	ProgramMismatch(&'static str),
}

impl fmt::Display for ExecutorDriverBuildError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Projection(error) => write!(formatter, "{error}"),
			Self::ProgramMismatch(detail) => {
				write!(
					formatter,
					"remote program differs from worker projection: {detail}"
				)
			}
		}
	}
}

impl Error for ExecutorDriverBuildError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Projection(error) => Some(error),
			Self::ProgramMismatch(_) => None,
		}
	}
}

impl From<WorkerProjectionError> for ExecutorDriverBuildError {
	fn from(error: WorkerProjectionError) -> Self { Self::Projection(error) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingInitChunk {
	device: DeviceId,
	bytes: usize,
}

/// Concrete remote worker adapter over Recipe's projected executor session.
///
/// Construction proves that the provisioned wire program is the exact
/// non-init view of the immutable worker projection. Preparation then
/// pre-realizes every local and external pending token before accepting an
/// init image. Live methods can only dispatch or poll task IDs in that
/// projection.
#[derive(Debug)]
pub struct ExecutorWorkerDriver<B: WorkerBackend + fmt::Debug> {
	bundle: FinalizedBundle,
	projection: WorkerProjection,
	expected_program: Digest,
	watchdog: Watchdog,
	backend: Option<B>,
	session: Option<WorkerExecutionSession<B>>,
	run: Option<RunId>,
	pending_init_chunk: Option<PendingInitChunk>,
}

impl<B: WorkerBackend + fmt::Debug> ExecutorWorkerDriver<B> {
	pub fn new(
		bundle: FinalizedBundle,
		topology: &Topology,
		assignment: WorkerAssignment,
		program: &ProvisionedProgram,
		backend: B,
		watchdog: Watchdog,
	) -> Result<Self, ExecutorDriverBuildError> {
		let projection = WorkerProjection::derive(&bundle, topology, assignment)?;
		validate_program(program, &projection, topology)?;
		Ok(Self {
			bundle,
			projection,
			expected_program: program.digest(),
			watchdog,
			backend: Some(backend),
			session: None,
			run: None,
			pending_init_chunk: None,
		})
	}

	#[must_use]
	pub const fn projection(&self) -> &WorkerProjection { &self.projection }

	#[must_use]
	pub const fn expected_program(&self) -> Digest { self.expected_program }

	#[must_use]
	pub const fn active_run(&self) -> Option<RunId> { self.run }

	fn session_mut(&mut self) -> Result<&mut WorkerExecutionSession<B>, DriverFault> {
		self.session.as_mut().ok_or_else(invalid_lifecycle)
	}

	fn current_run(&self) -> Result<RunId, DriverFault> { self.run.ok_or_else(invalid_lifecycle) }

	fn active_phase(&self) -> Result<RunPhase, DriverFault> {
		match self.session.as_ref().map(WorkerExecutionSession::lifecycle) {
			Some(WorkerLifecycle::Loop) => Ok(RunPhase::Loop),
			Some(WorkerLifecycle::Exit) => Ok(RunPhase::Exit),
			_ => Err(invalid_lifecycle()),
		}
	}

	fn recover_finished_session(&mut self) -> Result<(), DriverFault> {
		let session = self.session.take().ok_or_else(invalid_lifecycle)?;
		match session.into_parts() {
			Ok((backend, _journal)) => {
				self.backend = Some(backend);
				self.run = None;
				self.pending_init_chunk = None;
				Ok(())
			}
			Err(session) => {
				self.session = Some(*session);
				Err(invalid_lifecycle())
			}
		}
	}
}

impl<B: WorkerBackend + fmt::Debug> Drop for ExecutorWorkerDriver<B> {
	fn drop(&mut self) {
		if let Some(session) = self.session.as_mut() {
			let _cleanup = session.fatal_cleanup();
		}
	}
}

impl<B: WorkerBackend + fmt::Debug> WorkerDriver for ExecutorWorkerDriver<B> {
	fn prepare(&mut self, run: RunId, program: &ProvisionedProgram) -> Result<(), DriverFault> {
		if program.digest() != self.expected_program {
			return Err(DriverFault::new(
				DriverFaultCode::PROGRAM_IDENTITY_MISMATCH.get(),
				digest_prefix(program.digest()),
			));
		}
		if self.session.is_some() || self.run.is_some() {
			return Err(invalid_lifecycle());
		}
		let backend = self.backend.take().ok_or_else(invalid_lifecycle)?;
		match WorkerExecutionSession::prepare(
			run,
			&self.bundle,
			self.projection.clone(),
			backend,
			self.watchdog,
		) {
			Ok(session) => {
				self.session = Some(session);
				self.run = Some(run);
				Ok(())
			}
			Err(failure) => {
				let parts = failure.into_parts();
				self.backend = Some(parts.backend);
				let primary = executor_fault(&parts.error, DriverFaultCode::EXECUTOR_PREPARE_FAILED);
				match parts.cleanup_error {
					Some(cleanup) => {
						Err(DriverFault::cleanup_failed(
							primary,
							executor_fault(&cleanup, DriverFaultCode::EXECUTOR_CLEANUP_FAILED),
						))
					}
					None => Err(primary),
				}
			}
		}
	}

	fn begin_init_image(&mut self, device: DeviceId, bytes: ByteCount, digest: Digest) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.begin_init_image(run, device, bytes, digest)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn begin_init_chunk(&mut self, device: DeviceId, offset: u64, bytes: &[u8]) -> Result<(), DriverFault> {
		if self.pending_init_chunk.is_some() {
			return Err(invalid_lifecycle());
		}
		let run = self.current_run()?;
		let copied = self
			.session_mut()?
			.write_init_chunk(run, device, offset, bytes)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))?;
		self.pending_init_chunk = Some(PendingInitChunk {
			device,
			bytes: copied,
		});
		Ok(())
	}

	fn poll_init_chunk(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault> {
		let pending = self.pending_init_chunk.ok_or_else(invalid_lifecycle)?;
		if pending.device != device {
			return Err(invalid_lifecycle());
		}
		self.pending_init_chunk = None;
		Ok(DriverTransferPoll::Complete {
			bytes: pending.bytes,
		})
	}

	fn finish_init_image(&mut self, device: DeviceId) -> Result<(), DriverFault> {
		if self.pending_init_chunk.is_some() {
			return Err(invalid_lifecycle());
		}
		let run = self.current_run()?;
		self.session_mut()?
			.submit_init_image(run, device)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn poll_init_image(&mut self, device: DeviceId) -> Result<DriverTransferPoll, DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.poll_init_image(run, device)
			.map(transfer_poll)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn submit_task(&mut self, task: TaskId) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		let phase = self.active_phase()?;
		self.session_mut()?
			.submit_task(run, phase, task)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn poll_task(&mut self, task: TaskId) -> Result<DriverPoll, DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.poll_task(run, task)
			.map(|poll| {
				match poll {
					WorkerTaskPoll::Pending => DriverPoll::Pending,
					WorkerTaskPoll::Complete { metric } => {
						DriverPoll::Complete {
							metric: metric.map(|value| {
								match value {
									MetricValue::F32(value) => RemoteMetricValue::F32(value),
									MetricValue::I32(value) => RemoteMetricValue::I32(value),
								}
							}),
						}
					}
				}
			})
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn begin_receive_user_data(&mut self, task: TaskId, bytes: &[u8]) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		let phase = self.active_phase()?;
		self.session_mut()?
			.begin_external_ingress(run, phase, task, bytes)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn poll_receive_user_data(&mut self, task: TaskId) -> Result<DriverTransferPoll, DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.poll_external_ingress(run, task)
			.map(transfer_poll)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn begin_produce_user_data(&mut self, task: TaskId, destination: &mut [u8]) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		let phase = self.active_phase()?;
		self.session_mut()?
			.begin_external_egress(run, phase, task, destination)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn poll_produce_user_data(&mut self, task: TaskId) -> Result<DriverTransferPoll, DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.poll_external_egress(run, task)
			.map(transfer_poll)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn user_data_acked(&mut self, task: TaskId) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.acknowledge_external_egress(run, task)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn cancel(&mut self, reason: u32) -> Result<(), DriverFault> {
		let _reason = reason;
		let run = self.current_run()?;
		self.session_mut()?
			.cancel(run)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn begin_exit(&mut self) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.begin_exit(run)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn release_arena(&mut self, device: DeviceId) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.release_arena(run, device)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))
	}

	fn finish(&mut self) -> Result<(), DriverFault> {
		let run = self.current_run()?;
		self.session_mut()?
			.finish(run)
			.map_err(|error| executor_fault(&error, DriverFaultCode::EXECUTOR_OPERATION_FAILED))?;
		self.recover_finished_session()
	}

	fn cleanup_after_fault(&mut self, primary: DriverFault) -> Result<(), DriverFault> {
		let _primary = primary;
		let cleanup_error = match self.session.as_mut() {
			Some(session) => session.fatal_cleanup().err(),
			None => return Ok(()),
		};
		let recovery = self.recover_finished_session();
		match (cleanup_error, recovery) {
			(Some(error), _) => {
				Err(executor_fault(
					&error,
					DriverFaultCode::EXECUTOR_CLEANUP_FAILED,
				))
			}
			(None, Err(error)) => Err(error),
			(None, Ok(())) => Ok(()),
		}
	}
}

fn validate_program(
	program: &ProvisionedProgram,
	projection: &WorkerProjection,
	topology: &Topology,
) -> Result<(), ExecutorDriverBuildError> {
	if program.manifest().bundle != projection.bundle().digest() {
		return Err(ExecutorDriverBuildError::ProgramMismatch(
			"bundle identity differs",
		));
	}
	if program.devices().len() != projection.devices().len() {
		return Err(ExecutorDriverBuildError::ProgramMismatch(
			"device count differs",
		));
	}
	for device in program.devices() {
		if !projection.devices().contains(&device.id)
			|| projection.init_image_bytes(device.id) != Some(device.image_bytes)
		{
			return Err(ExecutorDriverBuildError::ProgramMismatch(
				"device image contract differs",
			));
		}
	}
	let projected_tasks = projection
		.tasks()
		.filter(|(_, phase, _)| *phase != RunPhase::Init)
		.collect::<Vec<_>>();
	if program.tasks().len() != projected_tasks.len() {
		return Err(ExecutorDriverBuildError::ProgramMismatch(
			"task count differs",
		));
	}
	for (program_task, (task, phase, role)) in program.tasks().iter().zip(projected_tasks) {
		let kind = match role {
			WorkerTaskRole::Local => ProgramTaskKind::Driver,
			WorkerTaskRole::ExternalIngress | WorkerTaskRole::ExternalEgress => ProgramTaskKind::CrossTransfer,
			WorkerTaskRole::InitAdmission => {
				return Err(ExecutorDriverBuildError::ProgramMismatch(
					"init admission leaked into runtime task manifest",
				));
			}
		};
		if program_task.id != task || program_task.phase != phase || program_task.kind != kind {
			return Err(ExecutorDriverBuildError::ProgramMismatch(
				"task identity, phase, or role differs",
			));
		}
	}
	let external = projection.external_transfers().collect::<Vec<_>>();
	if program.transfers().len() != external.len() {
		return Err(ExecutorDriverBuildError::ProgramMismatch(
			"cross-transfer count differs",
		));
	}
	for transfer in external {
		let Some(program_transfer) = program
			.transfers()
			.iter()
			.find(|candidate| candidate.task == transfer.task())
		else {
			return Err(ExecutorDriverBuildError::ProgramMismatch(
				"cross-transfer identity is absent",
			));
		};
		let direction = match transfer.direction() {
			ExternalTransferDirection::Ingress => DataDirection::MasterToWorker,
			ExternalTransferDirection::Egress => DataDirection::WorkerToMaster,
		};
		let Some(link) = transfer
			.route()
			.first()
			.and_then(|link| topology.link(*link))
		else {
			return Err(ExecutorDriverBuildError::ProgramMismatch(
				"cross-transfer route is unavailable",
			));
		};
		if program_transfer.task != transfer.task()
			|| program_transfer.direction != direction
			|| program_transfer.bytes != transfer.bytes()
			|| program_transfer.claims.len() != 1
			|| program_transfer.claims[0].resource != link.capacity_resource
			|| program_transfer.claims[0].mode != link.duplex
		{
			return Err(ExecutorDriverBuildError::ProgramMismatch(
				"cross-transfer contract differs",
			));
		}
	}
	Ok(())
}

const fn invalid_lifecycle() -> DriverFault { DriverFault::new(DriverFaultCode::INVALID_LIFECYCLE.get(), 0) }

const fn transfer_poll(poll: ExternalTransferPoll) -> DriverTransferPoll {
	match poll {
		ExternalTransferPoll::Pending => DriverTransferPoll::Pending,
		ExternalTransferPoll::Complete { bytes } => DriverTransferPoll::Complete { bytes },
	}
}

fn executor_fault<E: Error + Send + Sync + 'static>(
	error: &WorkerExecutionError<E>,
	code: DriverFaultCode,
) -> DriverFault {
	let detail = match error {
		WorkerExecutionError::Projection(_) | WorkerExecutionError::BundleMismatch => 1,
		WorkerExecutionError::InvalidRun(run) => run.get(),
		WorkerExecutionError::RunMismatch { actual, .. } => actual.get(),
		WorkerExecutionError::UnknownTask(task)
		| WorkerExecutionError::DuplicateDispatch(task)
		| WorkerExecutionError::TaskNotActive(task)
		| WorkerExecutionError::MetricContract { task, .. }
		| WorkerExecutionError::WatchdogExpired { task, .. } => task.get(),
		WorkerExecutionError::UnknownDevice(device)
		| WorkerExecutionError::InitDigestMismatch(device)
		| WorkerExecutionError::ArenaAlreadyReleased(device) => device.get(),
		WorkerExecutionError::WrongRole { task, .. }
		| WorkerExecutionError::DependencyIncomplete { task, .. }
		| WorkerExecutionError::ScheduleConflict { task, .. }
		| WorkerExecutionError::DeviceFault { readback: task, .. } => task.get(),
		WorkerExecutionError::WrongPhase { task, .. } | WorkerExecutionError::ByteCountMismatch { task, .. } => {
			task.map_or(0, TaskId::get)
		}
		WorkerExecutionError::PhaseIncomplete { task, .. } => task.get(),
		WorkerExecutionError::InvalidLifecycle { state, .. } => lifecycle_code(*state),
		WorkerExecutionError::InitOffsetMismatch { device, .. } => device.get(),
		WorkerExecutionError::Backend { operation, .. } => backend_operation_code(*operation),
		WorkerExecutionError::Journal(_) => u64::MAX - 1,
		WorkerExecutionError::CapacityOverflow => u64::MAX,
		_ => u64::MAX - 2,
	};
	DriverFault::new(code.get(), detail)
}

const fn lifecycle_code(lifecycle: WorkerLifecycle) -> u64 {
	match lifecycle {
		WorkerLifecycle::Prepared => 1,
		WorkerLifecycle::Init => 2,
		WorkerLifecycle::Loop => 3,
		WorkerLifecycle::Exit => 4,
		WorkerLifecycle::Cancelling => 5,
		WorkerLifecycle::Finished => 6,
		WorkerLifecycle::Failed => 7,
	}
}

const fn backend_operation_code(operation: WorkerBackendOperation) -> u64 {
	match operation {
		WorkerBackendOperation::BindProjection => 1,
		WorkerBackendOperation::PrepareLocal(_) => 2,
		WorkerBackendOperation::PrepareExternal(_) => 3,
		WorkerBackendOperation::AllocateArena(_) => 4,
		WorkerBackendOperation::SubmitLocal(_) => 5,
		WorkerBackendOperation::PollLocal(_) => 6,
		WorkerBackendOperation::SubmitIngress(_) => 7,
		WorkerBackendOperation::SubmitEgress(_) => 8,
		WorkerBackendOperation::PollExternal(_) => 9,
		WorkerBackendOperation::AcknowledgeEgress(_) => 10,
		WorkerBackendOperation::Quiesce => 11,
		WorkerBackendOperation::ReleaseArena(_) => 12,
		WorkerBackendOperation::DestroyResources => 13,
	}
}

fn digest_prefix(digest: Digest) -> u64 {
	let mut prefix = [0_u8; core::mem::size_of::<u64>()];
	prefix.copy_from_slice(&digest.bytes()[..core::mem::size_of::<u64>()]);
	u64::from_le_bytes(prefix)
}
