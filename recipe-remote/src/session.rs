use recipe_core::{ByteCount, DeviceId, Digest, DuplexMode, DuplexResourceId, RunId, RunPhase, TaskId};
use recipe_transport::{CompletionToken, FrameKind, ReceivedFrame, RuntimeChannel, RuntimeLane, ScheduleStamp};
use sha2::{Digest as _, Sha256};

use crate::{
	RemoteError, RemoteResult,
	codec::{self, Decoded, Message, PeerRole},
	driver::{DriverFault, DriverPoll, DriverTransferPoll, WorkerDriver},
	model::{
		DataDirection, ProgramTaskKind, ProvisionedProgram, RemoteIdentity, RemoteLimits, RuntimeCapacities,
		init_chunk_count, validate_run_id,
	},
};

/// A connected transport plus Recipe's connection-global run epoch.
///
/// Reusing the wrapper returned by a completed session preserves monotonic
/// `RunId` and user-data schedule positions across repeated lifecycles.
#[derive(Debug)]
pub struct RemoteChannel {
	channel: RuntimeChannel,
	last_run: Option<RunId>,
}

impl RemoteChannel {
	#[must_use]
	pub const fn new(channel: RuntimeChannel) -> Self {
		Self {
			channel,
			last_run: None,
		}
	}

	#[must_use]
	pub const fn identity(&self) -> recipe_transport::SessionIdentity { self.channel.identity() }
}

impl From<RuntimeChannel> for RemoteChannel {
	fn from(channel: RuntimeChannel) -> Self { Self::new(channel) }
}

#[derive(Debug)]
pub enum Advance<P, N> {
	Pending(P),
	Ready(N),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RemoteMetricValue {
	F32(f32),
	I32(i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricSample {
	pub task: TaskId,
	pub value: RemoteMetricValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelReason(u32);

impl CancelReason {
	#[must_use]
	pub const fn new(code: u32) -> Self { Self(code) }

	#[must_use]
	pub const fn code(self) -> u32 { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DriverFaultSendState {
	NeedSend,
	Waiting(CompletionToken),
}

#[derive(Clone, Copy, Debug)]
struct DriverFaultReport {
	fault: DriverFault,
	state: DriverFaultSendState,
}

fn begin_driver_fault<D: WorkerDriver>(
	driver: &mut D,
	report: &mut Option<DriverFaultReport>,
	fault: DriverFault,
) -> RemoteResult<()> {
	if report.is_some() {
		return Err(RemoteError::Protocol {
			detail: "worker attempted to report more than one terminal driver fault",
		});
	}
	let fault = match driver.cleanup_after_fault(fault) {
		Ok(()) => fault,
		Err(cleanup) => DriverFault::cleanup_failed(fault, cleanup),
	};
	*report = Some(DriverFaultReport {
		fault,
		state: DriverFaultSendState::NeedSend,
	});
	Ok(())
}

fn progress_driver_fault(
	core: &mut SessionCore,
	report: &mut Option<DriverFaultReport>,
	transmitted: Option<CompletionToken>,
) -> RemoteResult<Option<DriverFault>> {
	let Some(mut active) = *report else {
		return Ok(None);
	};
	match active.state {
		DriverFaultSendState::NeedSend => {
			if let Some(token) = core.try_send_tracked(
				Message::DriverFault {
					fault: active.fault,
				},
				None,
			)? {
				active.state = DriverFaultSendState::Waiting(token);
				*report = Some(active);
			}
			Ok(None)
		}
		DriverFaultSendState::Waiting(token) if transmitted == Some(token) => {
			*report = None;
			Ok(Some(active.fault))
		}
		DriverFaultSendState::Waiting(_) => Ok(None),
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MasterRunEvent {
	TaskComplete(TaskId),
	TaskFailed {
		task: TaskId,
		fault: DriverFault,
	},
	DataReady {
		task: TaskId,
		slot: usize,
		bytes: usize,
	},
	MetricReady {
		slot: usize,
		sample: MetricSample,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerRunEvent {
	TaskAccepted(TaskId),
	TaskReported(TaskId),
	DataAccepted(TaskId),
	DataAcknowledged(TaskId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskStatus {
	Idle,
	Active,
	ReceivedAwaitAck,
	Complete,
	Failed,
}

#[derive(Clone, Copy, Debug)]
struct MasterTaskState {
	task: TaskId,
	phase: RunPhase,
	kind: ProgramTaskKind,
	status: TaskStatus,
}

#[derive(Debug)]
struct DataSlot {
	task: Option<TaskId>,
	bytes: Box<[u8]>,
	length: usize,
}

impl DataSlot {
	fn new(bytes: usize) -> Self {
		Self {
			task: None,
			bytes: vec![0; bytes].into_boxed_slice(),
			length: 0,
		}
	}

	fn reset(&mut self) {
		self.task = None;
		self.bytes[..self.length].fill(0);
		self.length = 0;
	}
}

#[derive(Clone, Copy, Debug)]
struct MetricSlot {
	sample: Option<MetricSample>,
}

#[derive(Clone, Copy, Debug)]
struct HalfDuplexToken {
	resource: DuplexResourceId,
	owner: Option<TaskId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReleaseStatus {
	Needed,
	Requested,
	Complete,
}

#[derive(Clone, Copy, Debug)]
struct ReleaseState {
	device: DeviceId,
	status: ReleaseStatus,
}

#[derive(Debug)]
struct MasterStorage {
	tasks: Box<[MasterTaskState]>,
	transfers: Box<[crate::CrossTransfer]>,
	data: Box<[DataSlot]>,
	metrics: Box<[MetricSlot]>,
	half_duplex: Box<[HalfDuplexToken]>,
	releases: Box<[ReleaseState]>,
	pending_data_ack: Option<TaskId>,
}

impl MasterStorage {
	fn new(program: &ProvisionedProgram, limits: RemoteLimits) -> Self {
		let tasks = program
			.tasks()
			.iter()
			.map(|task| {
				MasterTaskState {
					task: task.id,
					phase: task.phase,
					kind: task.kind,
					status: TaskStatus::Idle,
				}
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let data = (0..limits.data_slots())
			.map(|_| DataSlot::new(limits.max_message_bytes()))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let metrics = vec![MetricSlot { sample: None }; limits.metric_slots()].into_boxed_slice();
		let half_duplex = program
			.half_duplex_resources()
			.iter()
			.map(|resource| {
				HalfDuplexToken {
					resource: *resource,
					owner: None,
				}
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let releases = program
			.devices()
			.iter()
			.map(|device| {
				ReleaseState {
					device: device.id,
					status: ReleaseStatus::Needed,
				}
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		Self {
			tasks,
			transfers: program.transfers().to_vec().into_boxed_slice(),
			data,
			metrics,
			half_duplex,
			releases,
			pending_data_ack: None,
		}
	}

	fn task_mut(&mut self, task: TaskId) -> RemoteResult<&mut MasterTaskState> {
		let index = self
			.tasks
			.binary_search_by_key(&task, |candidate| candidate.task)
			.map_err(|_| RemoteError::UnknownTask(task))?;
		Ok(&mut self.tasks[index])
	}

	fn transfer(&self, task: TaskId) -> RemoteResult<&crate::CrossTransfer> {
		self.transfers
			.iter()
			.find(|transfer| transfer.task == task)
			.ok_or(RemoteError::UnknownTask(task))
	}

	fn acquire_transfer(&mut self, task: TaskId) -> RemoteResult<()> {
		let transfer_index = self
			.transfers
			.iter()
			.position(|transfer| transfer.task == task)
			.ok_or(RemoteError::UnknownTask(task))?;
		for claim in self.transfers[transfer_index]
			.claims
			.iter()
			.filter(|claim| claim.mode == DuplexMode::Half)
		{
			let token = self
				.half_duplex
				.iter()
				.find(|token| token.resource == claim.resource)
				.ok_or(RemoteError::Protocol {
					detail: "half-duplex resource is absent from fixed storage",
				})?;
			if token.owner.is_some() {
				return Err(RemoteError::Backpressured("half-duplex capacity token"));
			}
		}
		for claim in self.transfers[transfer_index]
			.claims
			.iter()
			.filter(|claim| claim.mode == DuplexMode::Half)
		{
			let token = self
				.half_duplex
				.iter_mut()
				.find(|token| token.resource == claim.resource)
				.ok_or(RemoteError::Protocol {
					detail: "half-duplex resource disappeared",
				})?;
			token.owner = Some(task);
		}
		Ok(())
	}

	fn release_transfer(&mut self, task: TaskId) -> RemoteResult<()> {
		let transfer_index = self
			.transfers
			.iter()
			.position(|transfer| transfer.task == task)
			.ok_or(RemoteError::UnknownTask(task))?;
		for claim in self.transfers[transfer_index]
			.claims
			.iter()
			.filter(|claim| claim.mode == DuplexMode::Half)
		{
			let token = self
				.half_duplex
				.iter_mut()
				.find(|token| token.resource == claim.resource)
				.ok_or(RemoteError::Protocol {
					detail: "half-duplex resource disappeared",
				})?;
			if token.owner != Some(task) {
				return Err(RemoteError::Protocol {
					detail: "half-duplex capacity token belongs to another task",
				});
			}
			token.owner = None;
		}
		Ok(())
	}

	fn store_data(&mut self, task: TaskId, bytes: &[u8]) -> RemoteResult<usize> {
		let (index, slot) = self
			.data
			.iter_mut()
			.enumerate()
			.find(|(_, slot)| slot.task.is_none())
			.ok_or(RemoteError::Backpressured("master data inbox"))?;
		if bytes.len() > slot.bytes.len() {
			return Err(RemoteError::CapacityExhausted("master data inbox payload"));
		}
		slot.bytes[..bytes.len()].copy_from_slice(bytes);
		slot.length = bytes.len();
		slot.task = Some(task);
		Ok(index)
	}

	fn store_metric(&mut self, sample: MetricSample) -> RemoteResult<usize> {
		let (index, slot) = self
			.metrics
			.iter_mut()
			.enumerate()
			.find(|(_, slot)| slot.sample.is_none())
			.ok_or(RemoteError::Backpressured("master metric inbox"))?;
		slot.sample = Some(sample);
		Ok(index)
	}

	fn capacities(&self, scratch_bytes: usize) -> RuntimeCapacities {
		RuntimeCapacities {
			task_slots: self.tasks.len(),
			data_slots: self.data.len(),
			metric_slots: self.metrics.len(),
			scratch_bytes,
			half_duplex_tokens: self.half_duplex.len(),
		}
	}
}

#[derive(Debug)]
struct SessionCore {
	channel: RuntimeChannel,
	identity: RemoteIdentity,
	limits: RemoteLimits,
	program: ProvisionedProgram,
	run: RunId,
	scratch: Box<[u8]>,
	tx_sequence: [u64; 3],
	rx_sequence: [u64; 3],
	tx_schedule_base: u64,
	rx_schedule_base: u64,
	last_run: Option<RunId>,
	poisoned: bool,
}

impl SessionCore {
	fn new(
		remote_channel: RemoteChannel,
		identity: RemoteIdentity,
		limits: RemoteLimits,
		program: ProvisionedProgram,
		run: RunId,
	) -> RemoteResult<Self> {
		validate_run_id(run)?;
		program.manifest().validate()?;
		validate_init_schedule(&program, limits)?;
		let RemoteChannel { channel, last_run } = remote_channel;
		let channel_identity = channel.identity();
		if channel_identity.local() != identity.local() || channel_identity.remote() != identity.remote() {
			return Err(RemoteError::InvalidConfiguration(
				"remote identity differs from the connected transport",
			));
		}
		if last_run.is_some_and(|previous| run <= previous) {
			return Err(RemoteError::InvalidConfiguration(
				"run ids must increase strictly on a reused remote channel",
			));
		}
		let tx_schedule_base = channel.next_outbound_schedule_position()?;
		let rx_schedule_base = channel.next_inbound_schedule_position()?;
		Ok(Self {
			channel,
			identity,
			limits,
			program,
			run,
			scratch: vec![0; limits.max_message_bytes()].into_boxed_slice(),
			tx_sequence: [0; 3],
			rx_sequence: [0; 3],
			tx_schedule_base,
			rx_schedule_base,
			last_run: Some(run),
			poisoned: false,
		})
	}

	fn into_channel(self) -> RemoteChannel {
		RemoteChannel {
			channel: self.channel,
			last_run: self.last_run,
		}
	}

	fn ensure_healthy(&self) -> RemoteResult<()> {
		if self.poisoned {
			Err(RemoteError::Poisoned)
		} else {
			Ok(())
		}
	}

	fn poison<T>(&mut self, error: RemoteError) -> RemoteResult<T> {
		self.poisoned = true;
		Err(error)
	}

	fn progress_transport(&mut self) -> RemoteResult<Option<CompletionToken>> {
		self.ensure_healthy()?;
		let progress = match self.channel.progress() {
			Ok(progress) => progress,
			Err(error) => return self.poison(RemoteError::from(error)),
		};
		if let Some(token) = progress.transmitted
			&& let Err(error) = self.channel.release_completion(token)
		{
			return self.poison(RemoteError::from(error));
		}
		Ok(progress.transmitted)
	}

	fn try_send(&mut self, message: Message<'_>, schedule: Option<u64>) -> RemoteResult<bool> {
		Ok(self.try_send_tracked(message, schedule)?.is_some())
	}

	fn try_send_tracked(
		&mut self,
		message: Message<'_>,
		schedule: Option<u64>,
	) -> RemoteResult<Option<CompletionToken>> {
		self.ensure_healthy()?;
		let lane = message.lane();
		let index = lane_index(lane);
		let length = codec::encode(
			&mut self.scratch,
			self.tx_sequence[index],
			self.run,
			message,
		)?;
		let result = match lane {
			RuntimeLane::Control => self.channel.submit_control(&self.scratch[..length]),
			RuntimeLane::Metrics => self.channel.submit_metrics(&self.scratch[..length]),
			RuntimeLane::UserData => {
				let stamp = schedule.ok_or(RemoteError::Protocol {
					detail: "user-data message has no static schedule stamp",
				})?;
				let stamp = self
					.tx_schedule_base
					.checked_add(stamp)
					.ok_or(RemoteError::Protocol {
						detail: "connection-global user-data schedule exhausted",
					})?;
				self.channel.submit_user_data(
					ScheduleStamp::new(stamp).map_err(RemoteError::from)?,
					&self.scratch[..length],
				)
			}
		};
		match result {
			Ok(submission) => {
				self.tx_sequence[index] =
					self.tx_sequence[index]
						.checked_add(1)
						.ok_or(RemoteError::Protocol {
							detail: "remote transmit sequence exhausted",
						})?;
				Ok(Some(submission.token))
			}
			Err(recipe_transport::TransportError::CapacityExhausted(_)) => Ok(None),
			Err(error) => self.poison(RemoteError::from(error)),
		}
	}

	fn try_send_hello(&mut self, role: PeerRole) -> RemoteResult<bool> {
		let index = lane_index(RuntimeLane::Control);
		let length = codec::encode_hello(
			&mut self.scratch,
			self.tx_sequence[index],
			self.run,
			role,
			self.identity,
			self.limits,
		)?;
		match self.channel.submit_control(&self.scratch[..length]) {
			Ok(_) => {
				self.tx_sequence[index] =
					self.tx_sequence[index]
						.checked_add(1)
						.ok_or(RemoteError::Protocol {
							detail: "remote transmit sequence exhausted",
						})?;
				Ok(true)
			}
			Err(recipe_transport::TransportError::CapacityExhausted(_)) => Ok(false),
			Err(error) => self.poison(RemoteError::from(error)),
		}
	}

	fn try_send_manifest(&mut self) -> RemoteResult<bool> {
		let index = lane_index(RuntimeLane::Control);
		let length = codec::encode_manifest(
			&mut self.scratch,
			self.tx_sequence[index],
			self.run,
			self.program.manifest(),
			self.program.digest(),
		)?;
		match self.channel.submit_control(&self.scratch[..length]) {
			Ok(_) => {
				self.tx_sequence[index] =
					self.tx_sequence[index]
						.checked_add(1)
						.ok_or(RemoteError::Protocol {
							detail: "remote transmit sequence exhausted",
						})?;
				Ok(true)
			}
			Err(recipe_transport::TransportError::CapacityExhausted(_)) => Ok(false),
			Err(error) => self.poison(RemoteError::from(error)),
		}
	}

	fn received(&mut self) -> RemoteResult<Option<ReceivedMessage<'_>>> {
		self.ensure_healthy()?;
		let Some(frame) = self.channel.received() else {
			return Ok(None);
		};
		let lane = frame_lane(frame)?;
		let decoded = match codec::decode(frame.payload, self.limits) {
			Ok(decoded) => decoded,
			Err(error) => {
				self.poisoned = true;
				return Err(error);
			}
		};
		if decoded.run != self.run {
			self.poisoned = true;
			return Err(RemoteError::RunMismatch {
				expected: self.run,
				actual: decoded.run,
			});
		}
		if decoded.message.lane() != lane {
			self.poisoned = true;
			return Err(RemoteError::Protocol {
				detail: "message type arrived on the wrong runtime lane",
			});
		}
		let index = lane_index(lane);
		if decoded.sequence != self.rx_sequence[index] {
			self.poisoned = true;
			return Err(RemoteError::Protocol {
				detail: "remote per-lane sequence is replayed or out of order",
			});
		}
		self.rx_sequence[index] = self.rx_sequence[index]
			.checked_add(1)
			.ok_or(RemoteError::Protocol {
				detail: "remote receive sequence exhausted",
			})?;
		let schedule = frame
			.metadata
			.schedule
			.map(|schedule| {
				schedule
					.checked_sub(self.rx_schedule_base)
					.ok_or(RemoteError::Protocol {
						detail: "user-data schedule predates the active run",
					})
			})
			.transpose()?;
		Ok(Some(ReceivedMessage {
			token: frame.metadata.token,
			schedule,
			decoded,
		}))
	}

	fn release_received(&mut self, token: CompletionToken) -> RemoteResult<()> {
		match self.channel.release_received(token) {
			Ok(()) => Ok(()),
			Err(error) => self.poison(RemoteError::from(error)),
		}
	}

	fn validate_hello(&self, hello: codec::WireHello, remote_role: PeerRole) -> RemoteResult<()> {
		if hello.role != remote_role {
			return Err(RemoteError::Protocol {
				detail: "peer advertised the wrong remote role",
			});
		}
		if !hello.capabilities.contains(crate::Capabilities::REQUIRED) {
			return Err(RemoteError::Protocol {
				detail: "peer lacks a required remote capability",
			});
		}
		if hello.local_machine != self.identity.remote().machine()
			|| hello.local_profile != self.identity.remote().profile()
			|| hello.remote_machine != self.identity.local().machine()
			|| hello.remote_profile != self.identity.local().profile()
		{
			return Err(RemoteError::Protocol {
				detail: "peer hello profile identities differ from the prevalidated endpoints",
			});
		}
		if hello.limits != self.limits.wire_tuple() {
			return Err(RemoteError::Protocol {
				detail: "peer fixed remote limits differ",
			});
		}
		Ok(())
	}
}

#[derive(Debug)]
struct ReceivedMessage<'a> {
	token: CompletionToken,
	schedule: Option<u64>,
	decoded: Decoded<'a>,
}

fn lane_index(lane: RuntimeLane) -> usize {
	match lane {
		RuntimeLane::Control => 0,
		RuntimeLane::Metrics => 1,
		RuntimeLane::UserData => 2,
	}
}

fn frame_lane(frame: ReceivedFrame<'_>) -> RemoteResult<RuntimeLane> {
	match frame.metadata.kind {
		FrameKind::Control => Ok(RuntimeLane::Control),
		FrameKind::Metrics => Ok(RuntimeLane::Metrics),
		FrameKind::UserData => Ok(RuntimeLane::UserData),
		FrameKind::ProbeBegin | FrameKind::ProbeReady | FrameKind::ProbeData | FrameKind::ProbeComplete => {
			Err(RemoteError::Protocol {
				detail: "probe frame reached the remote runtime",
			})
		}
	}
}

fn validate_init_schedule(program: &ProvisionedProgram, limits: RemoteLimits) -> RemoteResult<()> {
	let chunks = init_chunk_count(program.devices(), limits)?;
	if program
		.transfers()
		.iter()
		.filter(|transfer| transfer.direction == DataDirection::MasterToWorker)
		.any(|transfer| transfer.schedule < chunks)
	{
		return Err(RemoteError::InvalidConfiguration(
			"master-to-worker schedule overlaps reserved init chunk positions",
		));
	}
	Ok(())
}

#[derive(Debug)]
pub struct MasterHandshake {
	core: SessionCore,
	storage: MasterStorage,
	hello_sent: bool,
	hello_received: bool,
	manifest_sent: bool,
	manifest_acked: bool,
	prepare_sent: bool,
}

impl MasterHandshake {
	pub fn new<C: Into<RemoteChannel>>(
		channel: C,
		identity: RemoteIdentity,
		limits: RemoteLimits,
		program: ProvisionedProgram,
		run: RunId,
	) -> RemoteResult<Self> {
		let core = SessionCore::new(channel.into(), identity, limits, program, run)?;
		let storage = MasterStorage::new(&core.program, limits);
		Ok(Self {
			core,
			storage,
			hello_sent: false,
			hello_received: false,
			manifest_sent: false,
			manifest_acked: false,
			prepare_sent: false,
		})
	}

	pub fn progress(mut self) -> RemoteResult<Advance<Self, MasterInit>> {
		self.core.progress_transport()?;
		if self.core.channel.received().is_some() {
			let (token, message) = {
				let received = self.core.received()?.ok_or(RemoteError::Protocol {
					detail: "received frame disappeared",
				})?;
				(received.token, received.decoded.message)
			};
			match message {
				Message::Hello(hello) if !self.hello_received => {
					self.core.validate_hello(hello, PeerRole::Worker)?;
					self.hello_received = true;
				}
				Message::ManifestAck { manifest, program } if self.manifest_sent && !self.manifest_acked => {
					if manifest != self.core.program.manifest().digest
						|| program != self.core.program.digest()
					{
						return self.core.poison(RemoteError::ManifestMismatch(
							"worker acknowledged different manifest identities",
						));
					}
					self.manifest_acked = true;
				}
				Message::PrepareAck if self.prepare_sent => {
					self.core.release_received(token)?;
					return Ok(Advance::Ready(MasterInit::new(self.core, self.storage)));
				}
				Message::DriverFault { fault } => {
					return self.core.poison(RemoteError::Driver(fault));
				}
				_ => {
					return self.core.poison(RemoteError::Protocol {
						detail: "master received a duplicate or out-of-order handshake message",
					});
				}
			}
			self.core.release_received(token)?;
		}
		if !self.hello_sent {
			self.hello_sent = self.core.try_send_hello(PeerRole::Master)?;
		} else if self.hello_received && !self.manifest_sent {
			self.manifest_sent = self.core.try_send_manifest()?;
		} else if self.manifest_acked && !self.prepare_sent {
			self.prepare_sent = self.core.try_send(Message::Prepare, None)?;
		}
		Ok(Advance::Pending(self))
	}
}

#[derive(Debug)]
pub struct WorkerHandshake<D: WorkerDriver> {
	core: SessionCore,
	driver: D,
	hello_sent: bool,
	hello_received: bool,
	manifest_received: bool,
	manifest_ack_sent: bool,
	prepared: bool,
	fault: Option<DriverFaultReport>,
}

impl<D: WorkerDriver> WorkerHandshake<D> {
	pub fn new<C: Into<RemoteChannel>>(
		channel: C,
		identity: RemoteIdentity,
		limits: RemoteLimits,
		program: ProvisionedProgram,
		run: RunId,
		driver: D,
	) -> RemoteResult<Self> {
		Ok(Self {
			core: SessionCore::new(channel.into(), identity, limits, program, run)?,
			driver,
			hello_sent: false,
			hello_received: false,
			manifest_received: false,
			manifest_ack_sent: false,
			prepared: false,
			fault: None,
		})
	}

	pub fn progress(mut self) -> RemoteResult<Advance<Self, WorkerInit<D>>> {
		let transmitted = self.core.progress_transport()?;
		if self.fault.is_some() {
			if let Some(fault) = progress_driver_fault(&mut self.core, &mut self.fault, transmitted)? {
				return Err(RemoteError::Driver(fault));
			}
			return Ok(Advance::Pending(self));
		}
		if self.core.channel.received().is_some() {
			let manifest_proof = (
				self.core.program.manifest().bundle,
				self.core.program.manifest().draft,
				self.core.program.manifest().realization,
				self.core.program.manifest().digest,
				self.core.program.digest(),
				self.core.program.manifest().artifacts.len(),
			);
			let (token, message) = {
				let received = self.core.received()?.ok_or(RemoteError::Protocol {
					detail: "received frame disappeared",
				})?;
				(received.token, received.decoded.message)
			};
			match message {
				Message::Hello(hello) if !self.hello_received => {
					self.core.validate_hello(hello, PeerRole::Master)?;
					self.hello_received = true;
				}
				Message::Manifest(manifest) if self.hello_received && !self.manifest_received => {
					if !manifest.matches_proof(
						manifest_proof.0,
						manifest_proof.1,
						manifest_proof.2,
						manifest_proof.3,
						manifest_proof.4,
						manifest_proof.5,
					) {
						return self.core.poison(RemoteError::ManifestMismatch(
							"master manifest differs from worker provisioning",
						));
					}
					self.manifest_received = true;
				}
				Message::Prepare if self.manifest_ack_sent && !self.prepared => {
					match self.driver.prepare(self.core.run, &self.core.program) {
						Ok(()) => self.prepared = true,
						Err(fault) => begin_driver_fault(&mut self.driver, &mut self.fault, fault)?,
					}
				}
				_ => {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker received a duplicate or out-of-order handshake message",
					});
				}
			}
			self.core.release_received(token)?;
		}
		if !self.hello_sent {
			self.hello_sent = self.core.try_send_hello(PeerRole::Worker)?;
		} else if self.manifest_received && !self.manifest_ack_sent {
			self.manifest_ack_sent = self.core.try_send(
				Message::ManifestAck {
					manifest: self.core.program.manifest().digest,
					program: self.core.program.digest(),
				},
				None,
			)?;
		} else if self.prepared && self.core.try_send(Message::PrepareAck, None)? {
			return Ok(Advance::Ready(WorkerInit::new(self.core, self.driver)));
		}
		Ok(Advance::Pending(self))
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeviceImageStatus {
	Needed,
	Active,
	Complete,
}

#[derive(Clone, Copy, Debug)]
struct DeviceImageState {
	device: DeviceId,
	bytes: ByteCount,
	status: DeviceImageStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MasterImagePhase {
	Begin,
	Chunks,
	End,
	WaitAck,
}

#[derive(Debug)]
struct MasterImage {
	device: DeviceId,
	bytes: Box<[u8]>,
	digest: Digest,
	offset: usize,
	pending_chunk: Option<(CompletionToken, usize)>,
	phase: MasterImagePhase,
}

#[derive(Debug)]
pub struct MasterInit {
	core: SessionCore,
	storage: MasterStorage,
	devices: Box<[DeviceImageState]>,
	active: Option<MasterImage>,
	next_schedule: u64,
	complete_sent: bool,
}

impl MasterInit {
	fn new(core: SessionCore, storage: MasterStorage) -> Self {
		let devices = core
			.program
			.devices()
			.iter()
			.map(|device| {
				DeviceImageState {
					device: device.id,
					bytes: device.image_bytes,
					status: DeviceImageStatus::Needed,
				}
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		Self {
			core,
			storage,
			devices,
			active: None,
			next_schedule: 0,
			complete_sent: false,
		}
	}

	pub fn queue_image(&mut self, device: DeviceId, bytes: Box<[u8]>) -> RemoteResult<()> {
		self.core.ensure_healthy()?;
		if self.active.is_some() {
			return Err(RemoteError::Backpressured("logical init image"));
		}
		let state = self
			.devices
			.iter_mut()
			.find(|state| state.device == device)
			.ok_or(RemoteError::UnknownDevice(device))?;
		if state.status != DeviceImageStatus::Needed {
			return Err(RemoteError::DuplicateDevice(device));
		}
		if u64::try_from(bytes.len()).ok() != Some(state.bytes.get()) {
			return Err(RemoteError::InvalidConfiguration(
				"init image size differs from the finalized arena",
			));
		}
		let digest = Digest::new(Sha256::digest(&bytes).into());
		state.status = DeviceImageStatus::Active;
		self.active = Some(MasterImage {
			device,
			bytes,
			digest,
			offset: 0,
			pending_chunk: None,
			phase: MasterImagePhase::Begin,
		});
		Ok(())
	}

	pub fn progress(mut self) -> RemoteResult<Advance<Self, MasterRun>> {
		let transmitted = self.core.progress_transport()?;
		if self.core.channel.received().is_some() {
			let (token, message) = {
				let received = self.core.received()?.ok_or(RemoteError::Protocol {
					detail: "received frame disappeared",
				})?;
				(received.token, received.decoded.message)
			};
			match message {
				Message::InitAck { device } => {
					let active = self.active.as_ref().ok_or(RemoteError::Protocol {
						detail: "init acknowledgment has no active image",
					})?;
					if active.device != device || active.phase != MasterImagePhase::WaitAck {
						return self.core.poison(RemoteError::Protocol {
							detail: "init acknowledgment names the wrong image",
						});
					}
					let state = self
						.devices
						.iter_mut()
						.find(|state| state.device == device)
						.ok_or(RemoteError::UnknownDevice(device))?;
					state.status = DeviceImageStatus::Complete;
					self.active = None;
				}
				Message::InitCompleteAck if self.complete_sent => {
					self.core.release_received(token)?;
					return Ok(Advance::Ready(MasterRun::new(self.core, self.storage)));
				}
				Message::DriverFault { fault } => {
					return self.core.poison(RemoteError::Driver(fault));
				}
				_ => {
					return self.core.poison(RemoteError::Protocol {
						detail: "master received an out-of-order init message",
					});
				}
			}
			self.core.release_received(token)?;
		}
		self.progress_image(transmitted)?;
		if self.active.is_none()
			&& self
				.devices
				.iter()
				.all(|state| state.status == DeviceImageStatus::Complete)
			&& !self.complete_sent
		{
			self.complete_sent = self.core.try_send(Message::InitComplete, None)?;
		}
		Ok(Advance::Pending(self))
	}

	fn progress_image(&mut self, transmitted: Option<CompletionToken>) -> RemoteResult<()> {
		let Some(active) = self.active.as_mut() else {
			return Ok(());
		};
		if let Some((token, end)) = active.pending_chunk {
			if transmitted != Some(token) {
				return Ok(());
			}
			active.offset = end;
			active.pending_chunk = None;
		}
		match active.phase {
			MasterImagePhase::Begin => {
				if self.core.try_send(
					Message::InitBegin {
						device: active.device,
						bytes: ByteCount::new(u64::try_from(active.bytes.len()).map_err(|_| {
							RemoteError::InvalidConfiguration("init image does not fit the wire ABI")
						})?),
						digest: active.digest,
					},
					None,
				)? {
					active.phase = MasterImagePhase::Chunks;
				}
			}
			MasterImagePhase::Chunks => {
				let maximum = self
					.core
					.scratch
					.len()
					.checked_sub(crate::model::INIT_CHUNK_OVERHEAD)
					.ok_or(RemoteError::CapacityExhausted("init chunk codec"))?;
				let end = active
					.offset
					.saturating_add(maximum)
					.min(active.bytes.len());
				if active.offset == end {
					active.phase = MasterImagePhase::End;
				} else if let Some(token) = self.core.try_send_tracked(
					Message::InitChunk {
						device: active.device,
						offset: u64::try_from(active.offset).map_err(|_| {
							RemoteError::InvalidConfiguration("init offset does not fit the wire ABI")
						})?,
						bytes: &active.bytes[active.offset..end],
					},
					Some(self.next_schedule),
				)? {
					active.pending_chunk = Some((token, end));
					self.next_schedule = self
						.next_schedule
						.checked_add(1)
						.ok_or(RemoteError::Protocol {
							detail: "init schedule sequence exhausted",
						})?;
				}
			}
			MasterImagePhase::End => {
				if self.core.try_send(
					Message::InitEnd {
						device: active.device,
						bytes: ByteCount::new(u64::try_from(active.bytes.len()).map_err(|_| {
							RemoteError::InvalidConfiguration("init image does not fit the wire ABI")
						})?),
					},
					None,
				)? {
					active.phase = MasterImagePhase::WaitAck;
				}
			}
			MasterImagePhase::WaitAck => {}
		}
		Ok(())
	}
}

#[derive(Debug)]
struct WorkerImage {
	device: DeviceId,
	bytes: ByteCount,
	digest: Digest,
	received: u64,
	hash: Sha256,
}

#[derive(Clone, Copy, Debug)]
struct PendingInitChunk {
	device: DeviceId,
	offset: u64,
	length: usize,
	end: u64,
}

#[derive(Clone, Copy, Debug)]
struct PendingInitImage {
	device: DeviceId,
	length: usize,
}

#[derive(Debug)]
pub struct WorkerInit<D: WorkerDriver> {
	core: SessionCore,
	driver: D,
	devices: Box<[DeviceImageState]>,
	active: Option<WorkerImage>,
	chunk_buffer: Box<[u8]>,
	pending_chunk: Option<PendingInitChunk>,
	pending_image: Option<PendingInitImage>,
	next_schedule: u64,
	pending_ack: Option<DeviceId>,
	complete_received: bool,
	fault: Option<DriverFaultReport>,
}

impl<D: WorkerDriver> WorkerInit<D> {
	fn new(core: SessionCore, driver: D) -> Self {
		let chunk_buffer = vec![0; core.limits.max_message_bytes()].into_boxed_slice();
		let devices = core
			.program
			.devices()
			.iter()
			.map(|device| {
				DeviceImageState {
					device: device.id,
					bytes: device.image_bytes,
					status: DeviceImageStatus::Needed,
				}
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		Self {
			core,
			driver,
			devices,
			active: None,
			chunk_buffer,
			pending_chunk: None,
			pending_image: None,
			next_schedule: 0,
			pending_ack: None,
			complete_received: false,
			fault: None,
		}
	}

	pub fn progress(mut self) -> RemoteResult<Advance<Self, WorkerRun<D>>> {
		let transmitted = self.core.progress_transport()?;
		if self.fault.is_some() {
			if let Some(fault) = progress_driver_fault(&mut self.core, &mut self.fault, transmitted)? {
				return Err(RemoteError::Driver(fault));
			}
			return Ok(Advance::Pending(self));
		}
		let chunk_active = self.progress_chunk()?;
		let transfer_active = match chunk_active {
			true => true,
			false => self.progress_image()?,
		};
		if !transfer_active && self.core.channel.received().is_some() {
			let token = self.process_received()?;
			self.core.release_received(token)?;
		}
		if let Some(device) = self.pending_ack
			&& self.core.try_send(Message::InitAck { device }, None)?
		{
			self.pending_ack = None;
		}
		if self.complete_received
			&& self.pending_ack.is_none()
			&& self.core.try_send(Message::InitCompleteAck, None)?
		{
			return Ok(Advance::Ready(WorkerRun::new(self.core, self.driver)));
		}
		Ok(Advance::Pending(self))
	}

	fn progress_chunk(&mut self) -> RemoteResult<bool> {
		let Some(pending) = self.pending_chunk else {
			return Ok(false);
		};
		let polled = match self.driver.poll_init_chunk(pending.device) {
			Ok(polled) => polled,
			Err(fault) => {
				begin_driver_fault(&mut self.driver, &mut self.fault, fault)?;
				return Ok(true);
			}
		};
		match polled {
			DriverTransferPoll::Pending => Ok(true),
			DriverTransferPoll::Complete { bytes } => {
				if bytes != pending.length {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker driver completed the wrong init chunk length",
					});
				}
				let active = self.active.as_mut().ok_or(RemoteError::Protocol {
					detail: "completed init chunk has no active image",
				})?;
				if active.device != pending.device || active.received != pending.offset {
					return self.core.poison(RemoteError::Protocol {
						detail: "completed init chunk differs from active image state",
					});
				}
				active.hash.update(&self.chunk_buffer[..pending.length]);
				active.received = pending.end;
				self.chunk_buffer[..pending.length].fill(0);
				self.pending_chunk = None;
				self.next_schedule = self
					.next_schedule
					.checked_add(1)
					.ok_or(RemoteError::Protocol {
						detail: "init schedule sequence exhausted",
					})?;
				Ok(false)
			}
		}
	}

	fn progress_image(&mut self) -> RemoteResult<bool> {
		let Some(pending) = self.pending_image else {
			return Ok(false);
		};
		let polled = match self.driver.poll_init_image(pending.device) {
			Ok(polled) => polled,
			Err(fault) => {
				begin_driver_fault(&mut self.driver, &mut self.fault, fault)?;
				return Ok(true);
			}
		};
		match polled {
			DriverTransferPoll::Pending => Ok(true),
			DriverTransferPoll::Complete { bytes } => {
				if bytes != pending.length {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker driver completed the wrong init image length",
					});
				}
				let state = self
					.devices
					.iter_mut()
					.find(|state| state.device == pending.device)
					.ok_or(RemoteError::UnknownDevice(pending.device))?;
				if state.status != DeviceImageStatus::Active {
					return self.core.poison(RemoteError::Protocol {
						detail: "completed init image is not active",
					});
				}
				state.status = DeviceImageStatus::Complete;
				self.pending_image = None;
				self.pending_ack = Some(pending.device);
				Ok(false)
			}
		}
	}

	fn process_received(&mut self) -> RemoteResult<CompletionToken> {
		let received = self.core.received()?.ok_or(RemoteError::Protocol {
			detail: "received frame disappeared",
		})?;
		let token = received.token;
		match received.decoded.message {
			Message::InitBegin {
				device,
				bytes,
				digest,
			} => {
				if self.active.is_some() || self.pending_image.is_some() || self.pending_ack.is_some() {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker received overlapping init images",
					});
				}
				let state = self
					.devices
					.iter_mut()
					.find(|state| state.device == device)
					.ok_or(RemoteError::UnknownDevice(device))?;
				if state.status != DeviceImageStatus::Needed || state.bytes != bytes {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker init image identity or size differs",
					});
				}
				match self.driver.begin_init_image(device, bytes, digest) {
					Ok(()) => {
						state.status = DeviceImageStatus::Active;
						self.active = Some(WorkerImage {
							device,
							bytes,
							digest,
							received: 0,
							hash: Sha256::new(),
						});
					}
					Err(fault) => begin_driver_fault(&mut self.driver, &mut self.fault, fault)?,
				}
			}
			Message::InitChunk {
				device,
				offset,
				bytes,
			} => {
				if self.pending_chunk.is_some() {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker received overlapping init chunks",
					});
				}
				if received.schedule != Some(self.next_schedule) {
					return self.core.poison(RemoteError::Protocol {
						detail: "init chunk schedule is replayed or out of order",
					});
				}
				let active = self.active.as_ref().ok_or(RemoteError::Protocol {
					detail: "init chunk has no active image",
				})?;
				if active.device != device || active.received != offset {
					return self.core.poison(RemoteError::Protocol {
						detail: "init chunk device or offset is out of order",
					});
				}
				let length = u64::try_from(bytes.len()).map_err(|_| {
					RemoteError::Protocol {
						detail: "init chunk length does not fit the wire ABI",
					}
				})?;
				let end = offset.checked_add(length).ok_or(RemoteError::Protocol {
					detail: "init chunk range overflowed",
				})?;
				if end > active.bytes.get() {
					return self.core.poison(RemoteError::Protocol {
						detail: "init chunk exceeds the finalized image",
					});
				}
				if bytes.len() > self.chunk_buffer.len() {
					return self
						.core
						.poison(RemoteError::CapacityExhausted("worker init chunk buffer"));
				}
				self.chunk_buffer[..bytes.len()].copy_from_slice(bytes);
				match self
					.driver
					.begin_init_chunk(device, offset, &self.chunk_buffer[..bytes.len()])
				{
					Ok(()) => {
						self.pending_chunk = Some(PendingInitChunk {
							device,
							offset,
							length: bytes.len(),
							end,
						});
					}
					Err(fault) => {
						self.chunk_buffer[..bytes.len()].fill(0);
						begin_driver_fault(&mut self.driver, &mut self.fault, fault)?;
					}
				}
			}
			Message::InitEnd { device, bytes } => {
				if self.pending_image.is_some() {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker received overlapping final init admissions",
					});
				}
				let active = self.active.take().ok_or(RemoteError::Protocol {
					detail: "init end has no active image",
				})?;
				if active.device != device || active.bytes != bytes || active.received != bytes.get() {
					return self.core.poison(RemoteError::Protocol {
						detail: "init end differs from the complete logical image",
					});
				}
				let actual = Digest::new(active.hash.finalize().into());
				if actual != active.digest {
					return self.core.poison(RemoteError::ManifestMismatch(
						"init image content digest differs",
					));
				}
				match self.driver.finish_init_image(device) {
					Ok(()) => {
						self.pending_image = Some(PendingInitImage {
							device,
							length: usize::try_from(bytes.get()).map_err(|_| {
								RemoteError::CapacityExhausted("worker final init admission")
							})?,
						});
					}
					Err(fault) => begin_driver_fault(&mut self.driver, &mut self.fault, fault)?,
				}
			}
			Message::InitComplete => {
				if self.active.is_some()
					|| self.pending_image.is_some()
					|| self.pending_ack.is_some()
					|| !self
						.devices
						.iter()
						.all(|state| state.status == DeviceImageStatus::Complete)
				{
					return self.core.poison(RemoteError::Protocol {
						detail: "init completed before every device image",
					});
				}
				self.complete_received = true;
			}
			_ => {
				return self.core.poison(RemoteError::Protocol {
					detail: "worker received an out-of-order init message",
				});
			}
		}
		Ok(token)
	}
}

#[derive(Debug)]
struct MasterRuntime {
	core: SessionCore,
	storage: MasterStorage,
}

impl MasterRuntime {
	fn submit_task(&mut self, phase: RunPhase, task: TaskId) -> RemoteResult<()> {
		{
			let state = self.storage.task_mut(task)?;
			if state.phase != phase || state.kind != ProgramTaskKind::Driver || state.status != TaskStatus::Idle {
				return Err(RemoteError::Protocol {
					detail: "driver task is not idle in the active remote phase",
				});
			}
		}
		if !self.core.try_send(Message::Execute { task }, None)? {
			return Err(RemoteError::Backpressured("control lane"));
		}
		self.storage.task_mut(task)?.status = TaskStatus::Active;
		Ok(())
	}

	fn send_user_data(&mut self, phase: RunPhase, task: TaskId, bytes: &[u8]) -> RemoteResult<()> {
		let (direction, expected_bytes, schedule) = {
			let transfer = self.storage.transfer(task)?;
			(transfer.direction, transfer.bytes, transfer.schedule)
		};
		if direction != DataDirection::MasterToWorker
			|| expected_bytes.get() != u64::try_from(bytes.len()).unwrap_or(u64::MAX)
		{
			return Err(RemoteError::Protocol {
				detail: "master-to-worker data differs from the provisioned transfer",
			});
		}
		{
			let state = self.storage.task_mut(task)?;
			if state.phase != phase
				|| state.kind != ProgramTaskKind::CrossTransfer
				|| state.status != TaskStatus::Idle
			{
				return Err(RemoteError::Protocol {
					detail: "cross transfer is not idle in the active remote phase",
				});
			}
		}
		self.storage.acquire_transfer(task)?;
		let submitted = self
			.core
			.try_send(Message::UserData { task, bytes }, Some(schedule));
		match submitted {
			Ok(true) => {
				self.storage.task_mut(task)?.status = TaskStatus::Active;
				Ok(())
			}
			Ok(false) => {
				self.storage.release_transfer(task)?;
				Err(RemoteError::Backpressured("user-data lane"))
			}
			Err(error) => {
				let release = self.storage.release_transfer(task);
				debug_assert!(release.is_ok());
				Err(error)
			}
		}
	}

	fn request_user_data(&mut self, phase: RunPhase, task: TaskId) -> RemoteResult<()> {
		if self
			.storage
			.tasks
			.iter()
			.any(|state| state.status == TaskStatus::Active && self.is_worker_to_master(state.task))
		{
			return Err(RemoteError::Backpressured(
				"worker-to-master transfer request",
			));
		}
		let transfer = self.storage.transfer(task)?;
		if transfer.direction != DataDirection::WorkerToMaster {
			return Err(RemoteError::Protocol {
				detail: "data request names the wrong transfer direction",
			});
		}
		{
			let state = self.storage.task_mut(task)?;
			if state.phase != phase
				|| state.kind != ProgramTaskKind::CrossTransfer
				|| state.status != TaskStatus::Idle
			{
				return Err(RemoteError::Protocol {
					detail: "data request is not idle in the active remote phase",
				});
			}
		}
		self.storage.acquire_transfer(task)?;
		let submitted = self.core.try_send(Message::DataRequest { task }, None);
		match submitted {
			Ok(true) => {
				self.storage.task_mut(task)?.status = TaskStatus::Active;
				Ok(())
			}
			Ok(false) => {
				self.storage.release_transfer(task)?;
				Err(RemoteError::Backpressured("control lane"))
			}
			Err(error) => {
				let release = self.storage.release_transfer(task);
				debug_assert!(release.is_ok());
				Err(error)
			}
		}
	}

	fn progress(&mut self, phase: RunPhase) -> RemoteResult<Option<MasterRunEvent>> {
		self.core.progress_transport()?;
		if let Some(task) = self.storage.pending_data_ack
			&& self.core.try_send(Message::DataAck { task }, None)?
		{
			self.storage.pending_data_ack = None;
			self.storage.task_mut(task)?.status = TaskStatus::Complete;
			self.storage.release_transfer(task)?;
			return Ok(Some(MasterRunEvent::TaskComplete(task)));
		}
		let Some(received) = self.core.received()? else {
			return Ok(None);
		};
		let token = received.token;
		let schedule = received.schedule;
		let event = match received.decoded.message {
			Message::TaskComplete { task } => {
				let state = self.storage.task_mut(task)?;
				if state.phase != phase || state.status != TaskStatus::Active {
					return self.core.poison(RemoteError::Protocol {
						detail: "task completion is duplicate or out of phase",
					});
				}
				state.status = TaskStatus::Complete;
				if state.kind == ProgramTaskKind::CrossTransfer {
					self.storage.release_transfer(task)?;
				}
				Some(MasterRunEvent::TaskComplete(task))
			}
			Message::TaskFailed { task, fault } => {
				let state = self.storage.task_mut(task)?;
				if state.phase != phase || state.status != TaskStatus::Active {
					return self.core.poison(RemoteError::Protocol {
						detail: "task failure is duplicate or out of phase",
					});
				}
				state.status = TaskStatus::Failed;
				if state.kind == ProgramTaskKind::CrossTransfer {
					self.storage.release_transfer(task)?;
				}
				Some(MasterRunEvent::TaskFailed { task, fault })
			}
			Message::UserData { task, bytes } => {
				let transfer = self.storage.transfer(task)?;
				if transfer.direction != DataDirection::WorkerToMaster
					|| transfer.schedule
						!= schedule.ok_or(RemoteError::Protocol {
							detail: "worker-to-master data has no schedule stamp",
						})? || transfer.bytes.get() != u64::try_from(bytes.len()).unwrap_or(u64::MAX)
				{
					return self.core.poison(RemoteError::Protocol {
						detail: "worker-to-master data differs from the static transfer",
					});
				}
				let state = self.storage.task_mut(task)?;
				if state.phase != phase || state.status != TaskStatus::Active {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker-to-master data is duplicate or out of phase",
					});
				}
				let slot = self.storage.store_data(task, bytes)?;
				self.storage.task_mut(task)?.status = TaskStatus::ReceivedAwaitAck;
				if self.storage.pending_data_ack.replace(task).is_some() {
					return self.core.poison(RemoteError::Protocol {
						detail: "more than one data acknowledgment is pending",
					});
				}
				Some(MasterRunEvent::DataReady {
					task,
					slot,
					bytes: bytes.len(),
				})
			}
			Message::Metric { task, value } => {
				let state = self.storage.task_mut(task)?;
				if state.phase != phase
					|| state.kind != ProgramTaskKind::Driver
					|| state.status != TaskStatus::Active
				{
					return self.core.poison(RemoteError::Protocol {
						detail: "metric is duplicate or belongs to an inactive task",
					});
				}
				let sample = MetricSample { task, value };
				let slot = self.storage.store_metric(sample)?;
				Some(MasterRunEvent::MetricReady { slot, sample })
			}
			Message::DriverFault { fault } => {
				return self.core.poison(RemoteError::Driver(fault));
			}
			_ => {
				return self.core.poison(RemoteError::Protocol {
					detail: "master received a message invalid for active execution",
				});
			}
		};
		self.core.release_received(token)?;
		Ok(event)
	}

	fn is_worker_to_master(&self, task: TaskId) -> bool {
		self.storage
			.transfer(task)
			.is_ok_and(|transfer| transfer.direction == DataDirection::WorkerToMaster)
	}

	fn phase_complete(&self, phase: RunPhase) -> bool {
		self.storage
			.tasks
			.iter()
			.filter(|task| task.phase == phase)
			.all(|task| task.status == TaskStatus::Complete)
	}

	fn capacities(&self) -> RuntimeCapacities { self.storage.capacities(self.core.scratch.len()) }

	fn data(&self, slot: usize) -> Option<(TaskId, &[u8])> {
		let slot = self.storage.data.get(slot)?;
		Some((slot.task?, &slot.bytes[..slot.length]))
	}

	fn release_data(&mut self, slot: usize) -> RemoteResult<()> {
		let slot = self
			.storage
			.data
			.get_mut(slot)
			.ok_or(RemoteError::InvalidConfiguration("unknown data inbox slot"))?;
		if slot.task.is_none() {
			return Err(RemoteError::Protocol {
				detail: "data inbox slot is already free",
			});
		}
		slot.reset();
		Ok(())
	}

	fn take_metric(&mut self, slot: usize) -> Option<MetricSample> {
		self.storage.metrics.get_mut(slot)?.sample.take()
	}
}

#[derive(Debug)]
pub struct MasterRun {
	runtime: MasterRuntime,
}

impl MasterRun {
	fn new(core: SessionCore, storage: MasterStorage) -> Self {
		Self {
			runtime: MasterRuntime { core, storage },
		}
	}

	pub fn submit_task(&mut self, task: TaskId) -> RemoteResult<()> { self.runtime.submit_task(RunPhase::Loop, task) }

	pub fn send_user_data(&mut self, task: TaskId, bytes: &[u8]) -> RemoteResult<()> {
		self.runtime.send_user_data(RunPhase::Loop, task, bytes)
	}

	pub fn request_user_data(&mut self, task: TaskId) -> RemoteResult<()> {
		self.runtime.request_user_data(RunPhase::Loop, task)
	}

	pub fn progress(&mut self) -> RemoteResult<Option<MasterRunEvent>> { self.runtime.progress(RunPhase::Loop) }

	#[must_use]
	pub fn capacities(&self) -> RuntimeCapacities { self.runtime.capacities() }

	#[must_use]
	pub fn data(&self, slot: usize) -> Option<(TaskId, &[u8])> { self.runtime.data(slot) }

	pub fn release_data(&mut self, slot: usize) -> RemoteResult<()> { self.runtime.release_data(slot) }

	pub fn take_metric(&mut self, slot: usize) -> Option<MetricSample> { self.runtime.take_metric(slot) }

	pub fn begin_exit(self) -> RemoteResult<MasterExit> {
		if !self.runtime.phase_complete(RunPhase::Loop) {
			return Err(RemoteError::Protocol {
				detail: "cannot exit before all remote loop tasks complete",
			});
		}
		Ok(MasterExit {
			runtime: self.runtime,
			state: MasterExitState::NeedBegin,
		})
	}

	#[must_use]
	pub fn cancel(self, reason: CancelReason) -> MasterCancelling {
		MasterCancelling {
			runtime: self.runtime,
			reason,
			sent: false,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WorkerTaskSlot {
	Free,
	Running {
		task: TaskId,
	},
	Complete {
		task: TaskId,
		metric: Option<RemoteMetricValue>,
		metric_state: MetricSendState,
	},
	Failed {
		task: TaskId,
		fault: DriverFault,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetricSendState {
	NotNeeded,
	NeedSend,
	Waiting(CompletionToken),
	Flushed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutgoingDataState {
	Producing,
	NeedSend,
	WaitAck,
}

#[derive(Clone, Copy, Debug)]
struct IncomingData {
	task: TaskId,
	length: usize,
}

#[derive(Clone, Copy, Debug)]
struct OutgoingData {
	task: TaskId,
	length: usize,
	schedule: u64,
	state: OutgoingDataState,
}

#[derive(Debug)]
struct WorkerStorage {
	tasks: Box<[MasterTaskState]>,
	transfers: Box<[crate::CrossTransfer]>,
	active: Box<[WorkerTaskSlot]>,
	half_duplex: Box<[HalfDuplexToken]>,
	releases: Box<[ReleaseState]>,
	data_scratch: Box<[u8]>,
	incoming_scratch: Box<[u8]>,
	incoming: Option<IncomingData>,
	outgoing: Option<OutgoingData>,
	poll_cursor: usize,
}

impl WorkerStorage {
	fn new(program: &ProvisionedProgram, limits: RemoteLimits) -> Self {
		Self {
			tasks: program
				.tasks()
				.iter()
				.map(|task| {
					MasterTaskState {
						task: task.id,
						phase: task.phase,
						kind: task.kind,
						status: TaskStatus::Idle,
					}
				})
				.collect::<Vec<_>>()
				.into_boxed_slice(),
			transfers: program.transfers().to_vec().into_boxed_slice(),
			active: vec![WorkerTaskSlot::Free; limits.task_slots()].into_boxed_slice(),
			half_duplex: program
				.half_duplex_resources()
				.iter()
				.map(|resource| {
					HalfDuplexToken {
						resource: *resource,
						owner: None,
					}
				})
				.collect::<Vec<_>>()
				.into_boxed_slice(),
			releases: program
				.devices()
				.iter()
				.map(|device| {
					ReleaseState {
						device: device.id,
						status: ReleaseStatus::Needed,
					}
				})
				.collect::<Vec<_>>()
				.into_boxed_slice(),
			data_scratch: vec![0; limits.max_message_bytes()].into_boxed_slice(),
			incoming_scratch: vec![0; limits.max_message_bytes()].into_boxed_slice(),
			incoming: None,
			outgoing: None,
			poll_cursor: 0,
		}
	}

	fn task_mut(&mut self, task: TaskId) -> RemoteResult<&mut MasterTaskState> {
		let index = self
			.tasks
			.binary_search_by_key(&task, |candidate| candidate.task)
			.map_err(|_| RemoteError::UnknownTask(task))?;
		Ok(&mut self.tasks[index])
	}

	fn transfer(&self, task: TaskId) -> RemoteResult<&crate::CrossTransfer> {
		self.transfers
			.iter()
			.find(|transfer| transfer.task == task)
			.ok_or(RemoteError::UnknownTask(task))
	}

	fn acquire_transfer(&mut self, task: TaskId) -> RemoteResult<()> {
		let index = self
			.transfers
			.iter()
			.position(|transfer| transfer.task == task)
			.ok_or(RemoteError::UnknownTask(task))?;
		for claim in self.transfers[index]
			.claims
			.iter()
			.filter(|claim| claim.mode == DuplexMode::Half)
		{
			let token = self
				.half_duplex
				.iter()
				.find(|token| token.resource == claim.resource)
				.ok_or(RemoteError::Protocol {
					detail: "worker half-duplex token is absent",
				})?;
			if token.owner.is_some() {
				return Err(RemoteError::Protocol {
					detail: "worker observed overlapping half-duplex transfers",
				});
			}
		}
		for claim in self.transfers[index]
			.claims
			.iter()
			.filter(|claim| claim.mode == DuplexMode::Half)
		{
			let token = self
				.half_duplex
				.iter_mut()
				.find(|token| token.resource == claim.resource)
				.ok_or(RemoteError::Protocol {
					detail: "worker half-duplex token disappeared",
				})?;
			token.owner = Some(task);
		}
		Ok(())
	}

	fn release_transfer(&mut self, task: TaskId) -> RemoteResult<()> {
		let index = self
			.transfers
			.iter()
			.position(|transfer| transfer.task == task)
			.ok_or(RemoteError::UnknownTask(task))?;
		for claim in self.transfers[index]
			.claims
			.iter()
			.filter(|claim| claim.mode == DuplexMode::Half)
		{
			let token = self
				.half_duplex
				.iter_mut()
				.find(|token| token.resource == claim.resource)
				.ok_or(RemoteError::Protocol {
					detail: "worker half-duplex token disappeared",
				})?;
			if token.owner != Some(task) {
				return Err(RemoteError::Protocol {
					detail: "worker half-duplex token belongs to another task",
				});
			}
			token.owner = None;
		}
		Ok(())
	}

	fn phase_complete(&self, phase: RunPhase) -> bool {
		self.tasks
			.iter()
			.filter(|task| task.phase == phase)
			.all(|task| task.status == TaskStatus::Complete)
	}
}

#[derive(Debug)]
struct WorkerRuntime<D: WorkerDriver> {
	core: SessionCore,
	driver: D,
	storage: WorkerStorage,
	fault: Option<DriverFaultReport>,
}

impl<D: WorkerDriver> WorkerRuntime<D> {
	fn new(core: SessionCore, driver: D) -> Self {
		let storage = WorkerStorage::new(&core.program, core.limits);
		Self {
			core,
			driver,
			storage,
			fault: None,
		}
	}

	fn begin_fault(&mut self, fault: DriverFault) -> RemoteResult<()> {
		begin_driver_fault(&mut self.driver, &mut self.fault, fault)
	}

	fn progress_fault(&mut self, transmitted: Option<CompletionToken>) -> RemoteResult<Option<DriverFault>> {
		progress_driver_fault(&mut self.core, &mut self.fault, transmitted)
	}

	fn progress_active(&mut self, transmitted: Option<CompletionToken>) -> RemoteResult<Option<WorkerRunEvent>> {
		if let Some(event) = self.progress_incoming()? {
			return Ok(Some(event));
		}
		for offset in 0..self.storage.active.len() {
			let index = (self.storage.poll_cursor + offset) % self.storage.active.len();
			let WorkerTaskSlot::Running { task } = self.storage.active[index] else {
				continue;
			};
			self.storage.poll_cursor = (index + 1) % self.storage.active.len();
			match self.driver.poll_task(task) {
				Ok(DriverPoll::Pending) => {}
				Ok(DriverPoll::Complete { metric }) => {
					self.storage.active[index] = WorkerTaskSlot::Complete {
						task,
						metric,
						metric_state: match metric {
							Some(_) => MetricSendState::NeedSend,
							None => MetricSendState::NotNeeded,
						},
					};
					break;
				}
				Err(fault) => {
					self.storage.active[index] = WorkerTaskSlot::Failed { task, fault };
					break;
				}
			}
		}
		for index in 0..self.storage.active.len() {
			match self.storage.active[index] {
				WorkerTaskSlot::Complete {
					task,
					metric,
					mut metric_state,
				} => {
					if metric_state == MetricSendState::NeedSend {
						let value = metric.ok_or(RemoteError::Protocol {
							detail: "worker metric completion lost its scalar",
						})?;
						let Some(token) = self
							.core
							.try_send_tracked(Message::Metric { task, value }, None)?
						else {
							return Ok(None);
						};
						metric_state = MetricSendState::Waiting(token);
						self.storage.active[index] = WorkerTaskSlot::Complete {
							task,
							metric,
							metric_state,
						};
						return Ok(None);
					}
					if let MetricSendState::Waiting(token) = metric_state {
						if transmitted != Some(token) {
							return Ok(None);
						}
						metric_state = MetricSendState::Flushed;
						self.storage.active[index] = WorkerTaskSlot::Complete {
							task,
							metric,
							metric_state,
						};
					}
					if self.core.try_send(Message::TaskComplete { task }, None)? {
						self.storage.active[index] = WorkerTaskSlot::Free;
						self.storage.task_mut(task)?.status = TaskStatus::Complete;
						return Ok(Some(WorkerRunEvent::TaskReported(task)));
					}
					return Ok(None);
				}
				WorkerTaskSlot::Failed { task, fault } => {
					if self
						.core
						.try_send(Message::TaskFailed { task, fault }, None)?
					{
						self.storage.active[index] = WorkerTaskSlot::Free;
						self.storage.task_mut(task)?.status = TaskStatus::Failed;
						return Ok(Some(WorkerRunEvent::TaskReported(task)));
					}
					return Ok(None);
				}
				WorkerTaskSlot::Free | WorkerTaskSlot::Running { .. } => {}
			}
		}
		for index in 0..self.storage.tasks.len() {
			let task = self.storage.tasks[index];
			if task.kind == ProgramTaskKind::CrossTransfer
				&& task.status == TaskStatus::ReceivedAwaitAck
				&& self
					.core
					.try_send(Message::TaskComplete { task: task.task }, None)?
			{
				self.storage.tasks[index].status = TaskStatus::Complete;
				self.storage.release_transfer(task.task)?;
				return Ok(Some(WorkerRunEvent::TaskReported(task.task)));
			}
		}
		self.progress_outgoing()
	}

	fn progress_incoming(&mut self) -> RemoteResult<Option<WorkerRunEvent>> {
		let Some(incoming) = self.storage.incoming else {
			return Ok(None);
		};
		match self
			.driver
			.poll_receive_user_data(incoming.task)
			.map_err(RemoteError::Driver)?
		{
			DriverTransferPoll::Pending => Ok(None),
			DriverTransferPoll::Complete { bytes } => {
				if bytes != incoming.length {
					return self.core.poison(RemoteError::Protocol {
						detail: "worker driver completed the wrong inbound data length",
					});
				}
				self.storage.incoming_scratch[..incoming.length].fill(0);
				self.storage.incoming = None;
				self.storage.task_mut(incoming.task)?.status = TaskStatus::ReceivedAwaitAck;
				Ok(Some(WorkerRunEvent::DataAccepted(incoming.task)))
			}
		}
	}

	fn progress_outgoing(&mut self) -> RemoteResult<Option<WorkerRunEvent>> {
		let Some(mut outgoing) = self.storage.outgoing else {
			return Ok(None);
		};
		if outgoing.state == OutgoingDataState::Producing {
			match self
				.driver
				.poll_produce_user_data(outgoing.task)
				.map_err(RemoteError::Driver)?
			{
				DriverTransferPoll::Pending => return Ok(None),
				DriverTransferPoll::Complete { bytes } => {
					if bytes != outgoing.length {
						return self.core.poison(RemoteError::Protocol {
							detail: "worker driver completed the wrong outbound data length",
						});
					}
					outgoing.state = OutgoingDataState::NeedSend;
					self.storage.outgoing = Some(outgoing);
				}
			}
		}
		if outgoing.state == OutgoingDataState::NeedSend
			&& self.core.try_send(
				Message::UserData {
					task: outgoing.task,
					bytes: &self.storage.data_scratch[..outgoing.length],
				},
				Some(outgoing.schedule),
			)? {
			self.storage.outgoing = Some(OutgoingData {
				state: OutgoingDataState::WaitAck,
				..outgoing
			});
		}
		Ok(None)
	}

	fn handle_execution_message(
		driver: &mut D,
		storage: &mut WorkerStorage,
		phase: RunPhase,
		message: Message<'_>,
		schedule: Option<u64>,
	) -> RemoteResult<Option<WorkerRunEvent>> {
		match message {
			Message::Execute { task } => {
				let task_state = storage.task_mut(task)?;
				if task_state.phase != phase
					|| task_state.kind != ProgramTaskKind::Driver
					|| task_state.status != TaskStatus::Idle
				{
					return Err(RemoteError::Protocol {
						detail: "worker execute command is duplicate or out of phase",
					});
				}
				let slot = storage
					.active
					.iter_mut()
					.find(|slot| **slot == WorkerTaskSlot::Free)
					.ok_or(RemoteError::CapacityExhausted("worker task slots"))?;
				driver.submit_task(task).map_err(RemoteError::Driver)?;
				*slot = WorkerTaskSlot::Running { task };
				storage.task_mut(task)?.status = TaskStatus::Active;
				Ok(Some(WorkerRunEvent::TaskAccepted(task)))
			}
			Message::UserData { task, bytes } => {
				if storage.incoming.is_some() {
					return Err(RemoteError::Protocol {
						detail: "worker received overlapping inbound data",
					});
				}
				let transfer = storage.transfer(task)?;
				if transfer.direction != DataDirection::MasterToWorker
					|| transfer.schedule
						!= schedule.ok_or(RemoteError::Protocol {
							detail: "master-to-worker data has no schedule stamp",
						})? || transfer.bytes.get() != u64::try_from(bytes.len()).unwrap_or(u64::MAX)
				{
					return Err(RemoteError::Protocol {
						detail: "master-to-worker data differs from the static transfer",
					});
				}
				let state = storage.task_mut(task)?;
				if state.phase != phase || state.status != TaskStatus::Idle {
					return Err(RemoteError::Protocol {
						detail: "master-to-worker transfer is duplicate or out of phase",
					});
				}
				if bytes.len() > storage.incoming_scratch.len() {
					return Err(RemoteError::CapacityExhausted("worker incoming data"));
				}
				storage.acquire_transfer(task)?;
				storage.incoming_scratch[..bytes.len()].copy_from_slice(bytes);
				driver.begin_receive_user_data(task, &storage.incoming_scratch[..bytes.len()])
					.map_err(RemoteError::Driver)?;
				storage.task_mut(task)?.status = TaskStatus::Active;
				storage.incoming = Some(IncomingData {
					task,
					length: bytes.len(),
				});
				Ok(Some(WorkerRunEvent::TaskAccepted(task)))
			}
			Message::DataRequest { task } => {
				if storage.outgoing.is_some() {
					return Err(RemoteError::Protocol {
						detail: "worker received overlapping data requests",
					});
				}
				let (direction, transfer_bytes, transfer_schedule) = {
					let transfer = storage.transfer(task)?;
					(transfer.direction, transfer.bytes, transfer.schedule)
				};
				if direction != DataDirection::WorkerToMaster {
					return Err(RemoteError::Protocol {
						detail: "worker data request has the wrong direction",
					});
				}
				let length = usize::try_from(transfer_bytes.get())
					.map_err(|_| RemoteError::CapacityExhausted("worker outgoing data"))?;
				if length > storage.data_scratch.len() {
					return Err(RemoteError::CapacityExhausted("worker outgoing data"));
				}
				let state = storage.task_mut(task)?;
				if state.phase != phase || state.status != TaskStatus::Idle {
					return Err(RemoteError::Protocol {
						detail: "worker data request is duplicate or out of phase",
					});
				}
				storage.acquire_transfer(task)?;
				driver.begin_produce_user_data(task, &mut storage.data_scratch[..length])
					.map_err(RemoteError::Driver)?;
				storage.task_mut(task)?.status = TaskStatus::Active;
				storage.outgoing = Some(OutgoingData {
					task,
					length,
					schedule: transfer_schedule,
					state: OutgoingDataState::Producing,
				});
				Ok(Some(WorkerRunEvent::TaskAccepted(task)))
			}
			Message::DataAck { task } => {
				let outgoing = storage.outgoing.ok_or(RemoteError::Protocol {
					detail: "worker data acknowledgment has no outgoing transfer",
				})?;
				if outgoing.task != task || outgoing.state != OutgoingDataState::WaitAck {
					return Err(RemoteError::Protocol {
						detail: "worker data acknowledgment is duplicate or out of order",
					});
				}
				driver.user_data_acked(task).map_err(RemoteError::Driver)?;
				storage.data_scratch[..outgoing.length].fill(0);
				storage.outgoing = None;
				storage.task_mut(task)?.status = TaskStatus::Complete;
				storage.release_transfer(task)?;
				Ok(Some(WorkerRunEvent::DataAcknowledged(task)))
			}
			_ => {
				Err(RemoteError::Protocol {
					detail: "worker received a message invalid for active execution",
				})
			}
		}
	}
}

#[derive(Debug)]
pub struct WorkerRun<D: WorkerDriver> {
	runtime: WorkerRuntime<D>,
}

impl<D: WorkerDriver> WorkerRun<D> {
	fn new(core: SessionCore, driver: D) -> Self {
		Self {
			runtime: WorkerRuntime::new(core, driver),
		}
	}

	pub fn progress(mut self) -> RemoteResult<WorkerRunProgress<D>> {
		let transmitted = self.runtime.core.progress_transport()?;
		if self.runtime.fault.is_some() {
			if let Some(fault) = self.runtime.progress_fault(transmitted)? {
				return Err(RemoteError::Driver(fault));
			}
			return Ok(WorkerRunProgress::Running {
				session: self,
				event: None,
			});
		}
		match self.runtime.progress_active(transmitted) {
			Ok(Some(event)) => {
				return Ok(WorkerRunProgress::Running {
					session: self,
					event: Some(event),
				});
			}
			Ok(None) => {}
			Err(RemoteError::Driver(fault)) => {
				self.runtime.begin_fault(fault)?;
				return Ok(WorkerRunProgress::Running {
					session: self,
					event: None,
				});
			}
			Err(error) => return Err(error),
		}
		let Some(received) = self.runtime.core.received()? else {
			return Ok(WorkerRunProgress::Running {
				session: self,
				event: None,
			});
		};
		let token = received.token;
		let schedule = received.schedule;
		match received.decoded.message {
			Message::Cancel { reason } => {
				let cancelled = self.runtime.driver.cancel(reason.code());
				self.runtime.core.release_received(token)?;
				if let Err(fault) = cancelled {
					self.runtime.begin_fault(fault)?;
					return Ok(WorkerRunProgress::Running {
						session: self,
						event: None,
					});
				}
				Ok(WorkerRunProgress::Cancelled(WorkerCancelled::new(
					self.runtime,
				)))
			}
			Message::BeginExit => {
				if !self.runtime.storage.phase_complete(RunPhase::Loop) {
					return self.runtime.core.poison(RemoteError::Protocol {
						detail: "worker received exit before all loop work completed",
					});
				}
				let begun = self.runtime.driver.begin_exit();
				self.runtime.core.release_received(token)?;
				if let Err(fault) = begun {
					self.runtime.begin_fault(fault)?;
					return Ok(WorkerRunProgress::Running {
						session: self,
						event: None,
					});
				}
				Ok(WorkerRunProgress::Exit(WorkerExit {
					runtime: self.runtime,
					ready_sent: false,
					exit_complete_received: false,
					finished: false,
					exit_ack: TerminalAck::NeedSend,
				}))
			}
			message => {
				let event = WorkerRuntime::handle_execution_message(
					&mut self.runtime.driver,
					&mut self.runtime.storage,
					RunPhase::Loop,
					message,
					schedule,
				);
				let event = match event {
					Ok(event) => event,
					Err(RemoteError::Driver(fault)) => {
						self.runtime.core.release_received(token)?;
						self.runtime.begin_fault(fault)?;
						return Ok(WorkerRunProgress::Running {
							session: self,
							event: None,
						});
					}
					Err(error) => return self.runtime.core.poison(error),
				};
				self.runtime.core.release_received(token)?;
				Ok(WorkerRunProgress::Running {
					session: self,
					event,
				})
			}
		}
	}

	#[must_use]
	pub fn capacities(&self) -> RuntimeCapacities {
		RuntimeCapacities {
			task_slots: self.runtime.storage.active.len(),
			data_slots: 1,
			metric_slots: self.runtime.core.limits.metric_slots(),
			scratch_bytes: self.runtime.core.scratch.len()
				+ self.runtime.storage.data_scratch.len()
				+ self.runtime.storage.incoming_scratch.len(),
			half_duplex_tokens: self.runtime.storage.half_duplex.len(),
		}
	}
}

#[derive(Debug)]
pub enum WorkerRunProgress<D: WorkerDriver> {
	Running {
		session: WorkerRun<D>,
		event: Option<WorkerRunEvent>,
	},
	Exit(WorkerExit<D>),
	Cancelled(WorkerCancelled<D>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MasterExitState {
	NeedBegin,
	WaitReady,
	Active,
	Releasing,
	NeedComplete,
	WaitComplete,
	Done,
}

#[derive(Debug)]
pub struct MasterExit {
	runtime: MasterRuntime,
	state: MasterExitState,
}

impl MasterExit {
	pub fn submit_task(&mut self, task: TaskId) -> RemoteResult<()> {
		self.ensure_active()?;
		self.runtime.submit_task(RunPhase::Exit, task)
	}

	pub fn send_user_data(&mut self, task: TaskId, bytes: &[u8]) -> RemoteResult<()> {
		self.ensure_active()?;
		self.runtime.send_user_data(RunPhase::Exit, task, bytes)
	}

	pub fn request_user_data(&mut self, task: TaskId) -> RemoteResult<()> {
		self.ensure_active()?;
		self.runtime.request_user_data(RunPhase::Exit, task)
	}

	pub fn progress(&mut self) -> RemoteResult<Option<MasterRunEvent>> {
		match self.state {
			MasterExitState::NeedBegin => {
				self.runtime.core.progress_transport()?;
				if self.runtime.core.try_send(Message::BeginExit, None)? {
					self.state = MasterExitState::WaitReady;
				}
				Ok(None)
			}
			MasterExitState::WaitReady => {
				self.runtime.core.progress_transport()?;
				let Some(received) = self.runtime.core.received()? else {
					return Ok(None);
				};
				let token = received.token;
				match received.decoded.message {
					Message::ExitReady => self.state = MasterExitState::Active,
					Message::DriverFault { fault } => {
						return self.runtime.core.poison(RemoteError::Driver(fault));
					}
					_ => {
						return self.runtime.core.poison(RemoteError::Protocol {
							detail: "master expected the worker exit-ready acknowledgment",
						});
					}
				}
				self.runtime.core.release_received(token)?;
				Ok(None)
			}
			MasterExitState::Active => {
				let event = self.runtime.progress(RunPhase::Exit)?;
				if self.runtime.phase_complete(RunPhase::Exit)
					&& self.runtime.storage.pending_data_ack.is_none()
				{
					self.state = MasterExitState::Releasing;
				}
				Ok(event)
			}
			MasterExitState::Releasing => self.progress_releases(),
			MasterExitState::NeedComplete => {
				self.runtime.core.progress_transport()?;
				if self.runtime.core.try_send(Message::ExitComplete, None)? {
					self.state = MasterExitState::WaitComplete;
				}
				Ok(None)
			}
			MasterExitState::WaitComplete => {
				self.runtime.core.progress_transport()?;
				let Some(received) = self.runtime.core.received()? else {
					return Ok(None);
				};
				let token = received.token;
				match received.decoded.message {
					Message::ExitAck => self.state = MasterExitState::Done,
					Message::DriverFault { fault } => {
						return self.runtime.core.poison(RemoteError::Driver(fault));
					}
					_ => {
						return self.runtime.core.poison(RemoteError::Protocol {
							detail: "master expected the terminal exit acknowledgment",
						});
					}
				}
				self.runtime.core.release_received(token)?;
				Ok(None)
			}
			MasterExitState::Done => Ok(None),
		}
	}

	fn progress_releases(&mut self) -> RemoteResult<Option<MasterRunEvent>> {
		self.runtime.core.progress_transport()?;
		if let Some(received) = self.runtime.core.received()? {
			let token = received.token;
			match received.decoded.message {
				Message::ReleaseAck { device } => {
					let state = self
						.runtime
						.storage
						.releases
						.iter_mut()
						.find(|state| state.device == device)
						.ok_or(RemoteError::UnknownDevice(device))?;
					if state.status != ReleaseStatus::Requested {
						return self.runtime.core.poison(RemoteError::Protocol {
							detail: "arena release acknowledgment is duplicate or out of order",
						});
					}
					state.status = ReleaseStatus::Complete;
				}
				Message::DriverFault { fault } => {
					return self.runtime.core.poison(RemoteError::Driver(fault));
				}
				_ => {
					return self.runtime.core.poison(RemoteError::Protocol {
						detail: "master expected an arena release acknowledgment",
					});
				}
			}
			self.runtime.core.release_received(token)?;
		}
		if self
			.runtime
			.storage
			.releases
			.iter()
			.all(|state| state.status == ReleaseStatus::Complete)
		{
			self.state = MasterExitState::NeedComplete;
			return Ok(None);
		}
		if self
			.runtime
			.storage
			.releases
			.iter()
			.any(|state| state.status == ReleaseStatus::Requested)
		{
			return Ok(None);
		}
		let state = self
			.runtime
			.storage
			.releases
			.iter_mut()
			.find(|state| state.status == ReleaseStatus::Needed)
			.ok_or(RemoteError::Protocol {
				detail: "release state has no next device",
			})?;
		if self.runtime.core.try_send(
			Message::Release {
				device: state.device,
			},
			None,
		)? {
			state.status = ReleaseStatus::Requested;
		}
		Ok(None)
	}

	fn ensure_active(&self) -> RemoteResult<()> {
		if self.state == MasterExitState::Active {
			Ok(())
		} else {
			Err(RemoteError::Protocol {
				detail: "remote exit work is not yet active",
			})
		}
	}

	#[must_use]
	pub const fn is_complete(&self) -> bool { matches!(self.state, MasterExitState::Done) }

	pub fn into_complete(self) -> RemoteResult<MasterComplete> {
		if self.state != MasterExitState::Done {
			return Err(RemoteError::Protocol {
				detail: "remote exit is not complete",
			});
		}
		Ok(MasterComplete {
			core: self.runtime.core,
			cancelled: false,
		})
	}

	#[must_use]
	pub fn capacities(&self) -> RuntimeCapacities { self.runtime.capacities() }

	#[must_use]
	pub fn data(&self, slot: usize) -> Option<(TaskId, &[u8])> { self.runtime.data(slot) }

	pub fn release_data(&mut self, slot: usize) -> RemoteResult<()> { self.runtime.release_data(slot) }

	pub fn take_metric(&mut self, slot: usize) -> Option<MetricSample> { self.runtime.take_metric(slot) }

	#[must_use]
	pub fn cancel(self, reason: CancelReason) -> MasterCancelling {
		MasterCancelling {
			runtime: self.runtime,
			reason,
			sent: false,
		}
	}
}

#[derive(Debug)]
pub struct MasterCancelling {
	runtime: MasterRuntime,
	reason: CancelReason,
	sent: bool,
}

impl MasterCancelling {
	pub fn progress(mut self) -> RemoteResult<Advance<Self, MasterComplete>> {
		self.runtime.core.progress_transport()?;
		if let Some(received) = self.runtime.core.received()? {
			let token = received.token;
			match received.decoded.message {
				Message::ReleaseAck { device } if self.sent => {
					let state = self
						.runtime
						.storage
						.releases
						.iter_mut()
						.find(|state| state.device == device)
						.ok_or(RemoteError::UnknownDevice(device))?;
					if state.status == ReleaseStatus::Complete {
						return self.runtime.core.poison(RemoteError::Protocol {
							detail: "cancel release acknowledgment is duplicate",
						});
					}
					state.status = ReleaseStatus::Complete;
				}
				Message::CancelAck
					if self.sent
						&& self
							.runtime
							.storage
							.releases
							.iter()
							.all(|state| state.status == ReleaseStatus::Complete) =>
				{
					self.runtime.core.release_received(token)?;
					return Ok(Advance::Ready(MasterComplete {
						core: self.runtime.core,
						cancelled: true,
					}));
				}
				Message::TaskComplete { .. }
				| Message::TaskFailed { .. }
				| Message::Metric { .. }
				| Message::UserData { .. }
				| Message::DataAck { .. }
				| Message::ExitReady
					if self.sent => {}
				Message::DriverFault { fault } => {
					return self.runtime.core.poison(RemoteError::Driver(fault));
				}
				_ => {
					return self.runtime.core.poison(RemoteError::Protocol {
						detail: "master received an invalid message while cancelling",
					});
				}
			}
			self.runtime.core.release_received(token)?;
		}
		if !self.sent {
			self.sent = self.runtime.core.try_send(
				Message::Cancel {
					reason: self.reason,
				},
				None,
			)?;
		}
		Ok(Advance::Pending(self))
	}
}

#[derive(Debug)]
pub struct MasterComplete {
	core: SessionCore,
	cancelled: bool,
}

impl MasterComplete {
	#[must_use]
	pub const fn was_cancelled(&self) -> bool { self.cancelled }

	#[must_use]
	pub fn run_id(&self) -> RunId { self.core.run }

	#[must_use]
	pub fn into_channel(self) -> RemoteChannel { self.core.into_channel() }
}

#[derive(Debug)]
pub struct WorkerCancelled<D: WorkerDriver> {
	core: SessionCore,
	driver: D,
	releases: Box<[ReleaseState]>,
	release_ack: Option<(DeviceId, CompletionToken)>,
	fault: Option<DriverFaultReport>,
	finished: bool,
	ack: TerminalAck,
}

impl<D: WorkerDriver> WorkerCancelled<D> {
	fn new(runtime: WorkerRuntime<D>) -> Self {
		let WorkerRuntime {
			core,
			driver,
			storage,
			fault,
		} = runtime;
		Self {
			core,
			driver,
			releases: storage.releases,
			release_ack: None,
			fault,
			finished: false,
			ack: TerminalAck::NeedSend,
		}
	}

	pub fn progress(mut self) -> RemoteResult<Advance<Self, WorkerComplete<D>>> {
		let transmitted = self.core.progress_transport()?;
		if self.fault.is_some() {
			if let Some(fault) = progress_driver_fault(&mut self.core, &mut self.fault, transmitted)? {
				return Err(RemoteError::Driver(fault));
			}
			return Ok(Advance::Pending(self));
		}
		if let Some((device, token)) = self.release_ack {
			if transmitted != Some(token) {
				return Ok(Advance::Pending(self));
			}
			let state = self
				.releases
				.iter_mut()
				.find(|state| state.device == device)
				.ok_or(RemoteError::UnknownDevice(device))?;
			if state.status != ReleaseStatus::Requested {
				return self.core.poison(RemoteError::Protocol {
					detail: "cancel release acknowledgment has invalid state",
				});
			}
			state.status = ReleaseStatus::Complete;
			self.release_ack = None;
		}
		if self.release_ack.is_none() {
			if let Some(state) = self
				.releases
				.iter()
				.find(|state| state.status == ReleaseStatus::Requested)
			{
				if let Some(token) = self.core.try_send_tracked(
					Message::ReleaseAck {
						device: state.device,
					},
					None,
				)? {
					self.release_ack = Some((state.device, token));
				}
				return Ok(Advance::Pending(self));
			}
			if let Some(state) = self
				.releases
				.iter_mut()
				.find(|state| state.status == ReleaseStatus::Needed)
			{
				match self.driver.release_arena(state.device) {
					Ok(()) => state.status = ReleaseStatus::Requested,
					Err(fault) => begin_driver_fault(&mut self.driver, &mut self.fault, fault)?,
				}
				return Ok(Advance::Pending(self));
			}
		}
		if !self
			.releases
			.iter()
			.all(|state| state.status == ReleaseStatus::Complete)
		{
			return Ok(Advance::Pending(self));
		}
		if !self.finished {
			match self.driver.finish() {
				Ok(()) => self.finished = true,
				Err(fault) => {
					begin_driver_fault(&mut self.driver, &mut self.fault, fault)?;
					return Ok(Advance::Pending(self));
				}
			}
		}
		match self.ack {
			TerminalAck::NeedSend => {
				if let Some(token) = self.core.try_send_tracked(Message::CancelAck, None)? {
					self.ack = TerminalAck::Waiting(token);
				}
			}
			TerminalAck::Waiting(token) if transmitted == Some(token) => {
				self.ack = TerminalAck::Flushed;
			}
			TerminalAck::Waiting(_) | TerminalAck::Flushed => {}
		}
		if self.ack == TerminalAck::Flushed {
			return Ok(Advance::Ready(WorkerComplete {
				core: self.core,
				driver: self.driver,
				cancelled: true,
			}));
		}
		Ok(Advance::Pending(self))
	}
}

#[derive(Debug)]
pub struct WorkerExit<D: WorkerDriver> {
	runtime: WorkerRuntime<D>,
	ready_sent: bool,
	exit_complete_received: bool,
	finished: bool,
	exit_ack: TerminalAck,
}

impl<D: WorkerDriver> WorkerExit<D> {
	pub fn progress(mut self) -> RemoteResult<WorkerExitProgress<D>> {
		let transmitted = self.runtime.core.progress_transport()?;
		if self.runtime.fault.is_some() {
			if let Some(fault) = self.runtime.progress_fault(transmitted)? {
				return Err(RemoteError::Driver(fault));
			}
			return Ok(WorkerExitProgress::Exiting {
				session: self,
				event: None,
			});
		}
		if !self.ready_sent {
			self.ready_sent = self.runtime.core.try_send(Message::ExitReady, None)?;
			return Ok(WorkerExitProgress::Exiting {
				session: self,
				event: None,
			});
		}
		if let Some(event) = self.flush_release_ack()? {
			return Ok(WorkerExitProgress::Exiting {
				session: self,
				event: Some(event),
			});
		}
		if self.exit_complete_received {
			if !self.finished {
				match self.runtime.driver.finish() {
					Ok(()) => self.finished = true,
					Err(fault) => {
						self.runtime.begin_fault(fault)?;
						return Ok(WorkerExitProgress::Exiting {
							session: self,
							event: None,
						});
					}
				}
			}
			match self.exit_ack {
				TerminalAck::NeedSend => {
					if let Some(token) = self.runtime.core.try_send_tracked(Message::ExitAck, None)? {
						self.exit_ack = TerminalAck::Waiting(token);
					}
				}
				TerminalAck::Waiting(token) if transmitted == Some(token) => {
					self.exit_ack = TerminalAck::Flushed;
				}
				TerminalAck::Waiting(_) | TerminalAck::Flushed => {}
			}
			if self.exit_ack == TerminalAck::Flushed {
				return Ok(WorkerExitProgress::Complete(WorkerComplete {
					core: self.runtime.core,
					driver: self.runtime.driver,
					cancelled: false,
				}));
			}
			return Ok(WorkerExitProgress::Exiting {
				session: self,
				event: None,
			});
		}
		match self.runtime.progress_active(transmitted) {
			Ok(Some(event)) => {
				return Ok(WorkerExitProgress::Exiting {
					session: self,
					event: Some(event),
				});
			}
			Ok(None) => {}
			Err(RemoteError::Driver(fault)) => {
				self.runtime.begin_fault(fault)?;
				return Ok(WorkerExitProgress::Exiting {
					session: self,
					event: None,
				});
			}
			Err(error) => return Err(error),
		}
		let Some(received) = self.runtime.core.received()? else {
			return Ok(WorkerExitProgress::Exiting {
				session: self,
				event: None,
			});
		};
		let token = received.token;
		let schedule = received.schedule;
		match received.decoded.message {
			Message::Cancel { reason } => {
				let cancelled = self.runtime.driver.cancel(reason.code());
				self.runtime.core.release_received(token)?;
				if let Err(fault) = cancelled {
					self.runtime.begin_fault(fault)?;
					return Ok(WorkerExitProgress::Exiting {
						session: self,
						event: None,
					});
				}
				Ok(WorkerExitProgress::Cancelled(WorkerCancelled::new(
					self.runtime,
				)))
			}
			Message::Release { device } => {
				if !self.runtime.storage.phase_complete(RunPhase::Exit) {
					return self.runtime.core.poison(RemoteError::Protocol {
						detail: "worker received arena release before exit tasks completed",
					});
				}
				let state = self
					.runtime
					.storage
					.releases
					.iter_mut()
					.find(|state| state.device == device)
					.ok_or(RemoteError::UnknownDevice(device))?;
				if state.status != ReleaseStatus::Needed {
					return self.runtime.core.poison(RemoteError::Protocol {
						detail: "worker arena release is duplicate or out of order",
					});
				}
				let released = self.runtime.driver.release_arena(device);
				self.runtime.core.release_received(token)?;
				match released {
					Ok(()) => state.status = ReleaseStatus::Requested,
					Err(fault) => {
						self.runtime.begin_fault(fault)?;
						return Ok(WorkerExitProgress::Exiting {
							session: self,
							event: None,
						});
					}
				}
				Ok(WorkerExitProgress::Exiting {
					session: self,
					event: None,
				})
			}
			Message::ExitComplete => {
				if !self.runtime.storage.phase_complete(RunPhase::Exit)
					|| !self
						.runtime
						.storage
						.releases
						.iter()
						.all(|state| state.status == ReleaseStatus::Complete)
				{
					return self.runtime.core.poison(RemoteError::Protocol {
						detail: "worker received exit-complete before exact arena release",
					});
				}
				self.exit_complete_received = true;
				self.runtime.core.release_received(token)?;
				Ok(WorkerExitProgress::Exiting {
					session: self,
					event: None,
				})
			}
			message => {
				let event = WorkerRuntime::handle_execution_message(
					&mut self.runtime.driver,
					&mut self.runtime.storage,
					RunPhase::Exit,
					message,
					schedule,
				);
				let event = match event {
					Ok(event) => event,
					Err(RemoteError::Driver(fault)) => {
						self.runtime.core.release_received(token)?;
						self.runtime.begin_fault(fault)?;
						return Ok(WorkerExitProgress::Exiting {
							session: self,
							event: None,
						});
					}
					Err(error) => return self.runtime.core.poison(error),
				};
				self.runtime.core.release_received(token)?;
				Ok(WorkerExitProgress::Exiting {
					session: self,
					event,
				})
			}
		}
	}

	fn flush_release_ack(&mut self) -> RemoteResult<Option<WorkerRunEvent>> {
		let Some(state) = self
			.runtime
			.storage
			.releases
			.iter_mut()
			.find(|state| state.status == ReleaseStatus::Requested)
		else {
			return Ok(None);
		};
		if self.runtime.core.try_send(
			Message::ReleaseAck {
				device: state.device,
			},
			None,
		)? {
			state.status = ReleaseStatus::Complete;
		}
		Ok(None)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalAck {
	NeedSend,
	Waiting(CompletionToken),
	Flushed,
}

#[derive(Debug)]
pub enum WorkerExitProgress<D: WorkerDriver> {
	Exiting {
		session: WorkerExit<D>,
		event: Option<WorkerRunEvent>,
	},
	Complete(WorkerComplete<D>),
	Cancelled(WorkerCancelled<D>),
}

#[derive(Debug)]
pub struct WorkerComplete<D: WorkerDriver> {
	core: SessionCore,
	driver: D,
	cancelled: bool,
}

impl<D: WorkerDriver> WorkerComplete<D> {
	#[must_use]
	pub const fn was_cancelled(&self) -> bool { self.cancelled }

	#[must_use]
	pub fn run_id(&self) -> RunId { self.core.run }

	#[must_use]
	pub fn into_parts(self) -> (RemoteChannel, D) { (self.core.into_channel(), self.driver) }
}
