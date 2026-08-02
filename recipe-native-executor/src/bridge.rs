//! Pre-realized staged transfers between local Host, CUDA, and HSA owners.

use core::fmt;
use std::{
	collections::{BTreeMap, BTreeSet},
	error::Error as StdError,
	io,
	num::TryFromIntError,
	sync::{
		Arc, Mutex,
		atomic::{AtomicU8, Ordering},
		mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
	},
	thread,
};

use recipe_core::{
	ByteCount, DeviceId, FinalizedBundle, ResolvedTransferEndpoint, RunPhase, SubmissionSlots, Task, TaskId,
	TaskKind, TransferEndpoint, TransferLaneClaim, ValueId,
};
use recipe_cuda::{
	CompletionStatus as CudaCompletionStatus, DeviceBuffer, Event, Pending as CudaPending, PinnedHostBuffer, Stream,
};
use recipe_executor::{BackendPoll, PendingRequest, TransferWork, WorkClass};
use recipe_host::Arena as HostArena;
use recipe_hsa::{
	Allocation, PollStatus as HsaPollStatus, PreparedPending as HsaPreparedPending, Session as HsaSession,
};
use recipe_planner::PlannedCandidate;

use crate::{
	cuda::CudaBinding,
	hsa::HsaBinding,
	local::{CandidateCrossBackendTransfer, CrossBackendTransfer, LocalArenaRef, LocalArenaSet, LocalDeviceClass},
};

const WORKER_IDLE: u8 = 0;
const WORKER_PENDING: u8 = 1;
const WORKER_COMPLETE: u8 = 2;
const WORKER_FAILED: u8 = 3;

#[derive(Debug)]
#[non_exhaustive]
pub enum StagedBridgeError {
	DuplicateBinding {
		device: DeviceId,
	},
	MissingBinding {
		device: DeviceId,
		class: LocalDeviceClass,
	},
	MissingTask {
		task: TaskId,
	},
	Contract {
		task: TaskId,
		detail: &'static str,
	},
	State {
		task: TaskId,
		detail: &'static str,
	},
	Host(recipe_host::Error),
	Cuda(recipe_cuda::CudaError),
	Hsa(recipe_hsa::Error),
	ThreadSpawn(io::ErrorKind),
	ThreadPanicked,
	WorkerDisconnected,
	WorkerState {
		task: TaskId,
		status: u8,
	},
	IntegerConversion {
		task: TaskId,
		detail: &'static str,
		source: TryFromIntError,
	},
}

impl fmt::Display for StagedBridgeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DuplicateBinding { device } => {
				write!(
					formatter,
					"cross-backend device {device} is bound more than once"
				)
			}
			Self::MissingBinding { device, class } => {
				write!(
					formatter,
					"cross-backend {class:?} device {device} has no native binding"
				)
			}
			Self::MissingTask { task } => {
				write!(formatter, "cross-backend task {task} was not pre-realized")
			}
			Self::Contract { task, detail } => {
				write!(
					formatter,
					"cross-backend task {task} violates its immutable contract: {detail}"
				)
			}
			Self::State { task, detail } => {
				write!(
					formatter,
					"cross-backend task {task} is in an invalid state: {detail}"
				)
			}
			Self::Host(error) => write!(formatter, "cross-backend host staging failed: {error}"),
			Self::Cuda(error) => write!(formatter, "cross-backend CUDA staging failed: {error}"),
			Self::Hsa(error) => write!(formatter, "cross-backend HSA staging failed: {error}"),
			Self::ThreadSpawn(kind) => {
				write!(
					formatter,
					"cross-backend staging worker could not start: {kind:?}"
				)
			}
			Self::ThreadPanicked => formatter.write_str("cross-backend staging worker panicked"),
			Self::WorkerDisconnected => formatter.write_str("cross-backend staging worker disconnected"),
			Self::WorkerState { task, status } => {
				write!(
					formatter,
					"cross-backend staging worker for task {task} reported invalid state {status}"
				)
			}
			Self::IntegerConversion { task, detail, .. } => {
				write!(
					formatter,
					"cross-backend task {task} cannot represent {detail} on this host"
				)
			}
		}
	}
}

impl StdError for StagedBridgeError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Host(error) => Some(error),
			Self::Cuda(error) => Some(error),
			Self::Hsa(error) => Some(error),
			Self::IntegerConversion { source, .. } => Some(source),
			Self::DuplicateBinding { .. }
			| Self::MissingBinding { .. }
			| Self::MissingTask { .. }
			| Self::Contract { .. }
			| Self::State { .. }
			| Self::ThreadSpawn(_)
			| Self::ThreadPanicked
			| Self::WorkerDisconnected
			| Self::WorkerState { .. } => None,
		}
	}
}

impl From<recipe_host::Error> for StagedBridgeError {
	fn from(error: recipe_host::Error) -> Self { Self::Host(error) }
}

impl From<recipe_cuda::CudaError> for StagedBridgeError {
	fn from(error: recipe_cuda::CudaError) -> Self { Self::Cuda(error) }
}

impl From<recipe_hsa::Error> for StagedBridgeError {
	fn from(error: recipe_hsa::Error) -> Self { Self::Hsa(error) }
}

/// Production one-hop bridge using pre-realized host staging workers and
/// native asynchronous copy tokens.
#[derive(Clone, Debug)]
pub struct StagedCrossBackend<'cuda, 'hsa> {
	cuda_bindings: Vec<CudaBinding<'cuda>>,
	hsa_bindings: Vec<HsaBinding<'hsa>>,
}

impl<'cuda, 'hsa> StagedCrossBackend<'cuda, 'hsa> {
	#[must_use]
	pub fn new(cuda_bindings: Vec<CudaBinding<'cuda>>, hsa_bindings: Vec<HsaBinding<'hsa>>) -> Self {
		Self {
			cuda_bindings,
			hsa_bindings,
		}
	}

	fn validate_bindings(&self, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<(), StagedBridgeError> {
		for (index, binding) in self.cuda_bindings.iter().enumerate() {
			let device = binding.device();
			match self.cuda_bindings[..index]
				.iter()
				.any(|prior| prior.device() == device)
			{
				true => Err(StagedBridgeError::DuplicateBinding { device }),
				false => Ok(()),
			}?;
			match devices.get(&device) == Some(&LocalDeviceClass::Cuda) {
				true => Ok(()),
				false => {
					Err(StagedBridgeError::MissingBinding {
						device,
						class: LocalDeviceClass::Cuda,
					})
				}
			}?;
		}
		for (index, binding) in self.hsa_bindings.iter().enumerate() {
			let device = binding.device();
			match self
				.cuda_bindings
				.iter()
				.any(|prior| prior.device() == device)
				|| self.hsa_bindings[..index]
					.iter()
					.any(|prior| prior.device() == device)
			{
				true => Err(StagedBridgeError::DuplicateBinding { device }),
				false => Ok(()),
			}?;
			match devices.get(&device) == Some(&LocalDeviceClass::Hsa) {
				true => Ok(()),
				false => {
					Err(StagedBridgeError::MissingBinding {
						device,
						class: LocalDeviceClass::Hsa,
					})
				}
			}?;
		}
		Ok(())
	}

	fn cuda_context(&self, device: DeviceId) -> Result<&'cuda recipe_cuda::Context, StagedBridgeError> {
		self.cuda_bindings
			.iter()
			.find(|binding| binding.device() == device)
			.map(CudaBinding::context)
			.ok_or(StagedBridgeError::MissingBinding {
				device,
				class: LocalDeviceClass::Cuda,
			})
	}

	fn hsa_binding(&self, device: DeviceId) -> Result<&HsaBinding<'hsa>, StagedBridgeError> {
		self.hsa_bindings
			.iter()
			.find(|binding| binding.device() == device)
			.ok_or(StagedBridgeError::MissingBinding {
				device,
				class: LocalDeviceClass::Hsa,
			})
	}

	fn realize_tasks<'tasks>(
		&self,
		tasks: impl Iterator<Item = &'tasks Task>,
		selected: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<StagedBridgeResources<'cuda, 'hsa>, StagedBridgeError> {
		self.validate_bindings(devices)?;
		let mut resources = BTreeMap::new();
		for task in tasks.filter(|task| selected.contains(&task.id)) {
			let contract = TransferContract::from_task(task, devices)?;
			let task_id = contract.task;
			let resource = self.realize_task(contract)?;
			match resources.insert(task_id, resource) {
				Some(_) => {
					Err(StagedBridgeError::Contract {
						task: task_id,
						detail: "task identity appears more than once",
					})
				}
				None => Ok(()),
			}?;
		}
		match resources.keys().copied().eq(selected.iter().copied()) {
			true => Ok(StagedBridgeResources { tasks: resources }),
			false => {
				Err(StagedBridgeError::MissingTask {
					task: first_missing_task(selected, &resources),
				})
			}
		}
	}

	fn realize_task(
		&self,
		contract: TransferContract,
	) -> Result<StagedTaskResources<'cuda, 'hsa>, StagedBridgeError> {
		let bytes = byte_count_to_usize(contract.bytes, contract.task)?;
		let (source, source_token) = self.realize_leg(contract.source, bytes)?;
		let (destination, destination_token) = self.realize_leg(contract.destination, bytes)?;
		let worker = HostStageWorker::new(contract.task)?;
		Ok(StagedTaskResources {
			contract,
			source,
			destination,
			tokens: PreparedTokenState::Available(Box::new(PreparedTokens {
				source: source_token,
				destination: destination_token,
			})),
			worker,
		})
	}

	fn realize_leg(
		&self,
		endpoint: EndpointContract,
		bytes: usize,
	) -> Result<(LegResources<'cuda, 'hsa>, PreparedLegToken<'cuda, 'hsa>), StagedBridgeError> {
		match endpoint.class {
			LocalDeviceClass::Host => Ok((LegResources::Host, PreparedLegToken::Host)),
			LocalDeviceClass::Cuda => {
				let context = self.cuda_context(endpoint.device)?;
				let staging = PinnedHostBuffer::allocate(context, bytes)?;
				let stream = Stream::create_nonblocking(context)?;
				let event = Event::create_completion(context)?;
				Ok((
					LegResources::Cuda { stream, staging },
					PreparedLegToken::Cuda(event),
				))
			}
			LocalDeviceClass::Hsa => {
				let binding = self.hsa_binding(endpoint.device)?;
				let session = binding.session();
				let staging = binding.allocate_host_fine(bytes)?;
				let pending = session.prepare_pending(2, 0)?;
				Ok((
					LegResources::Hsa { session, staging },
					PreparedLegToken::Hsa(pending),
				))
			}
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EndpointContract {
	device: DeviceId,
	value: ValueId,
	class: LocalDeviceClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TransferContract {
	task: TaskId,
	phase: RunPhase,
	class: WorkClass,
	source: EndpointContract,
	destination: EndpointContract,
	bytes: ByteCount,
	route: Vec<recipe_core::LinkId>,
	lane_claims: Vec<TransferLaneClaim>,
	submission: SubmissionSlots,
}

impl TransferContract {
	fn from_task(task: &Task, devices: &BTreeMap<DeviceId, LocalDeviceClass>) -> Result<Self, StagedBridgeError> {
		let TaskKind::Transfer(transfer) = &task.kind else {
			return Err(StagedBridgeError::Contract {
				task: task.id,
				detail: "only transfers may use the staged bridge",
			});
		};
		let (
			TransferEndpoint::Device {
				device: source_device,
				value: source_value,
			},
			TransferEndpoint::Device {
				device: destination_device,
				value: destination_value,
			},
		) = (transfer.source, transfer.destination)
		else {
			return Err(StagedBridgeError::Contract {
				task: task.id,
				detail: "staged bridge endpoints must both be devices",
			});
		};
		let source_class = devices
			.get(&source_device)
			.copied()
			.ok_or(StagedBridgeError::MissingBinding {
				device: source_device,
				class: LocalDeviceClass::Host,
			})?;
		let destination_class =
			devices
				.get(&destination_device)
				.copied()
				.ok_or(StagedBridgeError::MissingBinding {
					device: destination_device,
					class: LocalDeviceClass::Host,
				})?;
		let class = transfer_class(task.phase);
		match transfer.route.len() == 1 {
			true => Ok(()),
			false => {
				Err(StagedBridgeError::Contract {
					task: task.id,
					detail: "staged transfers require exactly one finalized link",
				})
			}
		}?;
		match source_class == destination_class
			&& !(source_class == LocalDeviceClass::Cuda && source_device != destination_device)
		{
			true => {
				Err(StagedBridgeError::Contract {
					task: task.id,
					detail: "transfer does not cross local backend ownership",
				})
			}
			false => Ok(()),
		}?;
		Ok(Self {
			task: task.id,
			phase: task.phase,
			class,
			source: EndpointContract {
				device: source_device,
				value: source_value,
				class: source_class,
			},
			destination: EndpointContract {
				device: destination_device,
				value: destination_value,
				class: destination_class,
			},
			bytes: transfer.bytes,
			route: transfer.route.clone(),
			lane_claims: transfer.lane_claims.clone(),
			submission: transfer.submission,
		})
	}

	fn validate_finalized(
		&self,
		bundle: &FinalizedBundle,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<(), StagedBridgeError> {
		let task = bundle
			.tasks()
			.iter()
			.find(|task| task.id == self.task)
			.ok_or(StagedBridgeError::MissingTask { task: self.task })?;
		let TaskKind::Transfer(transfer) = &task.kind else {
			return Err(StagedBridgeError::Contract {
				task: self.task,
				detail: "finalized bridge task is not a transfer",
			});
		};
		let source = TransferEndpoint::Device {
			device: self.source.device,
			value: self.source.value,
		};
		let destination = TransferEndpoint::Device {
			device: self.destination.device,
			value: self.destination.value,
		};
		let exact = task.phase == self.phase
			&& transfer_class(task.phase) == self.class
			&& transfer.source == source
			&& transfer.destination == destination
			&& transfer.bytes == self.bytes
			&& transfer.route == self.route
			&& transfer.lane_claims == self.lane_claims
			&& transfer.submission == self.submission
			&& devices.get(&self.source.device) == Some(&self.source.class)
			&& devices.get(&self.destination.device) == Some(&self.destination.class);
		match exact {
			true => Ok(()),
			false => {
				Err(StagedBridgeError::Contract {
					task: self.task,
					detail: "finalized transfer differs from its pre-realized candidate",
				})
			}
		}?;
		let endpoints = bundle
			.transfer_endpoints(self.task)
			.ok_or(StagedBridgeError::Contract {
				task: self.task,
				detail: "finalized transfer has no resolved endpoints",
			})?;
		match (endpoints.source, endpoints.destination) {
			(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => {
				match source.device == self.source.device
					&& source.value == self.source.value
					&& destination.device == self.destination.device
					&& destination.value == self.destination.value
				{
					true => Ok(()),
					false => {
						Err(StagedBridgeError::Contract {
							task: self.task,
							detail: "resolved transfer endpoints differ from the staged values",
						})
					}
				}
			}
			(
				ResolvedTransferEndpoint::External | ResolvedTransferEndpoint::Device(_),
				ResolvedTransferEndpoint::External | ResolvedTransferEndpoint::Device(_),
			) => {
				Err(StagedBridgeError::Contract {
					task: self.task,
					detail: "resolved transfer endpoints differ from the staged values",
				})
			}
		}
	}

	fn validate_work(&self, class: WorkClass, work: &TransferWork<'_>) -> Result<(), StagedBridgeError> {
		match (work.source, work.destination) {
			(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) => {
				match work.task == self.task
					&& class == self.class && source.device == self.source.device
					&& source.value == self.source.value
					&& destination.device == self.destination.device
					&& destination.value == self.destination.value
					&& work.bytes == self.bytes
					&& work.route == self.route
					&& work.lane_claims == self.lane_claims
					&& work.submission == self.submission
				{
					true => Ok(()),
					false => {
						Err(StagedBridgeError::Contract {
							task: self.task,
							detail: "submitted transfer differs from the immutable staged contract",
						})
					}
				}
			}
			(
				ResolvedTransferEndpoint::External | ResolvedTransferEndpoint::Device(_),
				ResolvedTransferEndpoint::External | ResolvedTransferEndpoint::Device(_),
			) => {
				Err(StagedBridgeError::Contract {
					task: self.task,
					detail: "submitted transfer differs from the immutable staged contract",
				})
			}
		}
	}
}

const fn transfer_class(phase: RunPhase) -> WorkClass {
	match phase {
		RunPhase::Init | RunPhase::Loop => WorkClass::InternalTransfer,
		RunPhase::Exit => WorkClass::ExitTransfer,
	}
}

enum LegResources<'cuda, 'hsa> {
	Host,
	Cuda {
		stream: Stream<'cuda>,
		staging: PinnedHostBuffer<'cuda>,
	},
	Hsa {
		session: &'hsa HsaSession<'hsa>,
		staging: Allocation<'hsa>,
	},
}

impl fmt::Debug for LegResources<'_, '_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Host => formatter.write_str("Host"),
			Self::Cuda { staging, .. } => {
				formatter
					.debug_struct("Cuda")
					.field("staging_bytes", &staging.len())
					.finish()
			}
			Self::Hsa { staging, .. } => {
				formatter
					.debug_struct("Hsa")
					.field("staging_bytes", &staging.len())
					.finish()
			}
		}
	}
}

impl LegResources<'_, '_> {
	fn host_address(&mut self, task: TaskId) -> Result<usize, StagedBridgeError> {
		match self {
			Self::Host => {
				Err(StagedBridgeError::State {
					task,
					detail: "host endpoint has no separate staging buffer",
				})
			}
			Self::Cuda { staging, .. } => Ok(staging.as_mut_slice().as_mut_ptr().addr()),
			Self::Hsa { staging, .. } => Ok(staging.as_ptr().cast::<u8>().addr()),
		}
	}

	fn destroy(self) -> Result<(), StagedBridgeError> {
		match self {
			Self::Host => Ok(()),
			Self::Cuda { stream, staging } => {
				let stream_result = stream.destroy().map_err(StagedBridgeError::from);
				let staging_result = staging.free().map_err(StagedBridgeError::from);
				first_error(stream_result, staging_result)
			}
			Self::Hsa { staging, .. } => staging.close().map_err(StagedBridgeError::from),
		}
	}
}

enum PreparedLegToken<'cuda, 'hsa> {
	Host,
	Cuda(Event<'cuda>),
	Hsa(HsaPreparedPending<'hsa, 'hsa>),
}

impl fmt::Debug for PreparedLegToken<'_, '_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Host => formatter.write_str("Host"),
			Self::Cuda(_) => formatter.write_str("CudaEvent"),
			Self::Hsa(_) => formatter.write_str("HsaSignal"),
		}
	}
}

#[derive(Debug)]
struct PreparedTokens<'cuda, 'hsa> {
	source: PreparedLegToken<'cuda, 'hsa>,
	destination: PreparedLegToken<'cuda, 'hsa>,
}

#[derive(Debug)]
enum PreparedTokenState<'cuda, 'hsa> {
	Available(Box<PreparedTokens<'cuda, 'hsa>>),
	Consumed,
}

#[derive(Debug)]
struct StagedTaskResources<'cuda, 'hsa> {
	contract: TransferContract,
	source: LegResources<'cuda, 'hsa>,
	destination: LegResources<'cuda, 'hsa>,
	tokens: PreparedTokenState<'cuda, 'hsa>,
	worker: HostStageWorker,
}

/// Physical resources for every selected staged transfer.
pub struct StagedBridgeResources<'cuda, 'hsa> {
	tasks: BTreeMap<TaskId, StagedTaskResources<'cuda, 'hsa>>,
}

impl fmt::Debug for StagedBridgeResources<'_, '_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("StagedBridgeResources")
			.field("tasks", &self.tasks)
			.finish()
	}
}

enum ActiveLeg<'cuda, 'hsa> {
	Host,
	CudaReady(Event<'cuda>),
	CudaActive(CudaPending<'static, 'cuda>),
	Hsa(HsaPreparedPending<'hsa, 'hsa>),
	Transition,
}

impl fmt::Debug for ActiveLeg<'_, '_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Host => formatter.write_str("Host"),
			Self::CudaReady(_) => formatter.write_str("CudaReady"),
			Self::CudaActive(_) => formatter.write_str("CudaActive"),
			Self::Hsa(_) => formatter.write_str("Hsa"),
			Self::Transition => formatter.write_str("Transition"),
		}
	}
}

impl<'cuda, 'hsa> From<PreparedLegToken<'cuda, 'hsa>> for ActiveLeg<'cuda, 'hsa> {
	fn from(token: PreparedLegToken<'cuda, 'hsa>) -> Self {
		match token {
			PreparedLegToken::Host => Self::Host,
			PreparedLegToken::Cuda(event) => Self::CudaReady(event),
			PreparedLegToken::Hsa(pending) => Self::Hsa(pending),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingState {
	Ready,
	Source,
	Middle,
	Destination,
	Complete,
}

enum DestinationTarget<'cuda, 'hsa> {
	Host,
	Cuda {
		buffer: *const DeviceBuffer<'cuda>,
		offset: usize,
	},
	Hsa {
		allocation: *const Allocation<'hsa>,
		offset: usize,
	},
}

impl fmt::Debug for DestinationTarget<'_, '_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Host => formatter.write_str("Host"),
			Self::Cuda { offset, .. } => {
				formatter
					.debug_struct("Cuda")
					.field("offset", offset)
					.finish()
			}
			Self::Hsa { offset, .. } => {
				formatter
					.debug_struct("Hsa")
					.field("offset", offset)
					.finish()
			}
		}
	}
}

#[derive(Debug)]
enum MiddleJobState {
	Uninitialized,
	Ready(HostStageJob),
	Submitted,
}

impl MiddleJobState {
	fn initialize(&mut self, task: TaskId, job: HostStageJob) -> Result<(), StagedBridgeError> {
		match self {
			Self::Uninitialized => {
				*self = Self::Ready(job);
				Ok(())
			}
			Self::Ready(_) | Self::Submitted => {
				Err(StagedBridgeError::State {
					task,
					detail: "staged transfer middle job was initialized more than once",
				})
			}
		}
	}

	fn submit(&mut self, task: TaskId, worker: &HostStageWorker) -> Result<(), StagedBridgeError> {
		let prior = core::mem::replace(self, Self::Submitted);
		match prior {
			Self::Ready(job) => worker.submit(job),
			Self::Uninitialized | Self::Submitted => {
				*self = prior;
				Err(StagedBridgeError::State {
					task,
					detail: "staged transfer has no ready host middle job",
				})
			}
		}
	}
}

/// One exact, pre-realized staged transfer token.
pub struct StagedBridgePending<'cuda, 'hsa> {
	task: TaskId,
	state: PendingState,
	source: ActiveLeg<'cuda, 'hsa>,
	destination: ActiveLeg<'cuda, 'hsa>,
	middle: MiddleJobState,
	target: DestinationTarget<'cuda, 'hsa>,
}

impl fmt::Debug for StagedBridgePending<'_, '_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("StagedBridgePending")
			.field("task", &self.task)
			.field("state", &self.state)
			.field("source", &self.source)
			.field("destination", &self.destination)
			.field("middle", &self.middle)
			.field("target", &self.target)
			.finish()
	}
}

#[derive(Debug)]
enum HostStageJob {
	Read {
		arena: HostArena,
		offset: u64,
		destination: usize,
		bytes: usize,
	},
	Write {
		source: usize,
		arena: HostArena,
		offset: u64,
		bytes: usize,
	},
	Copy {
		source: usize,
		destination: usize,
		bytes: usize,
	},
}

impl HostStageJob {
	fn execute(self) -> Result<(), recipe_host::Error> {
		match self {
			Self::Read {
				arena,
				offset,
				destination,
				bytes,
			} => {
				// SAFETY: the bridge owns this preallocated destination until
				// the worker publishes terminal status.
				let pointer = core::ptr::with_exposed_provenance_mut::<u8>(destination);
				// SAFETY: the pointer and byte count satisfy the ownership
				// conditions stated above.
				let destination = unsafe { core::slice::from_raw_parts_mut(pointer, bytes) };
				arena.bridge_read_exact(offset, destination)
			}
			Self::Write {
				source,
				arena,
				offset,
				bytes,
			} => {
				// SAFETY: the preceding native completion and worker state keep
				// this source allocation live and immutable for the job.
				let pointer = core::ptr::with_exposed_provenance::<u8>(source);
				// SAFETY: the pointer and byte count satisfy the lifetime and
				// immutability conditions stated above.
				let source = unsafe { core::slice::from_raw_parts(pointer, bytes) };
				arena.bridge_write_exact(offset, source)
			}
			Self::Copy {
				source,
				destination,
				bytes,
			} => {
				// SAFETY: source and destination are distinct per-endpoint
				// staging allocations retained by the task resource.
				let source = core::ptr::with_exposed_provenance::<u8>(source);
				let destination = core::ptr::with_exposed_provenance_mut::<u8>(destination);
				// SAFETY: justified above.
				let copy = || unsafe { core::ptr::copy_nonoverlapping(source, destination, bytes) };
				copy();
				Ok(())
			}
		}
	}
}

enum WorkerMessage {
	Run(HostStageJob),
}

enum WorkerSenderState {
	Active(SyncSender<WorkerMessage>),
	Closed,
}

enum WorkerCompletionState {
	Waiting(Receiver<()>),
	Observed,
}

enum WorkerFailureState {
	Empty,
	Available(recipe_host::Error),
	Consumed,
}

struct HostStageWorker {
	task: TaskId,
	sender: WorkerSenderState,
	status: Arc<AtomicU8>,
	error: Arc<Mutex<WorkerFailureState>>,
	completion: WorkerCompletionState,
}

impl fmt::Debug for HostStageWorker {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HostStageWorker")
			.field("task", &self.task)
			.field("status", &self.status.load(Ordering::Acquire))
			.finish_non_exhaustive()
	}
}

impl HostStageWorker {
	fn new(task: TaskId) -> Result<Self, StagedBridgeError> {
		let (sender, receiver) = sync_channel(1);
		let status = Arc::new(AtomicU8::new(WORKER_IDLE));
		let error = Arc::new(Mutex::new(WorkerFailureState::Empty));
		let worker_status = Arc::clone(&status);
		let worker_error = Arc::clone(&error);
		let (completion_sender, completion_receiver) = sync_channel(1);
		let handle = thread::Builder::new()
			.name(format!("recipe-bridge-{}", task.get()))
			.spawn(move || {
				worker_loop(receiver, &worker_status, &worker_error);
				let completion = completion_sender.send(());
				debug_assert!(completion.is_ok(), "bridge completion receiver was dropped");
			})
			.map_err(|error| StagedBridgeError::ThreadSpawn(error.kind()))?;
		drop(handle);
		Ok(Self {
			task,
			sender: WorkerSenderState::Active(sender),
			status,
			error,
			completion: WorkerCompletionState::Waiting(completion_receiver),
		})
	}

	fn submit(&self, job: HostStageJob) -> Result<(), StagedBridgeError> {
		self.status
			.compare_exchange(
				WORKER_IDLE,
				WORKER_PENDING,
				Ordering::AcqRel,
				Ordering::Acquire,
			)
			.map_err(|status| {
				StagedBridgeError::WorkerState {
					task: self.task,
					status,
				}
			})?;
		let WorkerSenderState::Active(sender) = &self.sender else {
			return Err(StagedBridgeError::WorkerDisconnected);
		};
		match sender.try_send(WorkerMessage::Run(job)) {
			Ok(()) => Ok(()),
			Err(TrySendError::Full(message)) | Err(TrySendError::Disconnected(message)) => {
				drop(message);
				self.status.store(WORKER_IDLE, Ordering::Release);
				Err(StagedBridgeError::WorkerDisconnected)
			}
		}
	}

	fn poll(&self) -> Result<BackendPoll, StagedBridgeError> {
		match self.status.load(Ordering::Acquire) {
			WORKER_PENDING => Ok(BackendPoll::Pending),
			WORKER_COMPLETE => Ok(BackendPoll::Complete { metric: None }),
			WORKER_FAILED => {
				let mut failure = self.error.lock().map_err(|poisoned| {
					drop(poisoned);
					StagedBridgeError::WorkerDisconnected
				})?;
				let state = core::mem::replace(&mut *failure, WorkerFailureState::Consumed);
				match state {
					WorkerFailureState::Available(error) => Err(StagedBridgeError::Host(error)),
					WorkerFailureState::Empty | WorkerFailureState::Consumed => {
						Err(StagedBridgeError::WorkerDisconnected)
					}
				}
			}
			WORKER_IDLE => {
				Err(StagedBridgeError::State {
					task: self.task,
					detail: "host staging worker has no submitted job",
				})
			}
			status => {
				Err(StagedBridgeError::WorkerState {
					task: self.task,
					status,
				})
			}
		}
	}

	fn reset(&self) -> Result<(), StagedBridgeError> {
		self.status
			.compare_exchange(
				WORKER_COMPLETE,
				WORKER_IDLE,
				Ordering::AcqRel,
				Ordering::Acquire,
			)
			.map_err(|status| {
				StagedBridgeError::WorkerState {
					task: self.task,
					status,
				}
			})
			.map(|_| ())
	}

	fn close(mut self) -> Result<(), StagedBridgeError> { self.shutdown() }

	fn shutdown(&mut self) -> Result<(), StagedBridgeError> {
		let sender = core::mem::replace(&mut self.sender, WorkerSenderState::Closed);
		drop(sender);
		let completion = core::mem::replace(&mut self.completion, WorkerCompletionState::Observed);
		match completion {
			WorkerCompletionState::Waiting(receiver) => {
				let result = std::iter::repeat_with(|| {
					match receiver.try_recv() {
						Ok(()) => Some(Ok(())),
						Err(TryRecvError::Empty) => {
							thread::yield_now();
							None
						}
						Err(TryRecvError::Disconnected) => Some(Err(StagedBridgeError::ThreadPanicked)),
					}
				})
				.find_map(core::convert::identity);
				match result {
					Some(result) => result,
					None => Err(StagedBridgeError::WorkerDisconnected),
				}
			}
			WorkerCompletionState::Observed => Ok(()),
		}
	}
}

impl Drop for HostStageWorker {
	fn drop(&mut self) { drop(self.shutdown()); }
}

fn worker_loop(receiver: Receiver<WorkerMessage>, status: &AtomicU8, error: &Mutex<WorkerFailureState>) {
	for message in receiver {
		match message {
			WorkerMessage::Run(job) => {
				match job.execute() {
					Ok(()) => status.store(WORKER_COMPLETE, Ordering::Release),
					Err(failure) => {
						match error.lock() {
							Ok(mut slot) => {
								*slot = WorkerFailureState::Available(failure);
								status.store(WORKER_FAILED, Ordering::Release);
							}
							Err(poisoned) => {
								drop(poisoned);
								status.store(WORKER_FAILED, Ordering::Release);
							}
						}
					}
				}
			}
		}
	}
}

impl<'cuda, 'hsa> CrossBackendTransfer<'cuda, 'hsa> for StagedCrossBackend<'cuda, 'hsa> {
	type Error = StagedBridgeError;
	type Pending = StagedBridgePending<'cuda, 'hsa>;
	type Resource = StagedBridgeResources<'cuda, 'hsa>;

	fn bind(
		&mut self,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<Self::Resource, Self::Error> {
		self.realize_tasks(bundle.tasks().iter(), tasks, devices)
	}

	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
	) -> Result<Self::Pending, Self::Error> {
		let task = resource
			.tasks
			.get_mut(&request.task)
			.ok_or(StagedBridgeError::MissingTask { task: request.task })?;
		match request.phase != task.contract.phase
			|| request.class != task.contract.class
			|| request.submission != Some(task.contract.submission)
		{
			true => {
				Err(StagedBridgeError::Contract {
					task: request.task,
					detail: "pending request differs from the staged transfer",
				})
			}
			false => Ok(()),
		}?;
		let token_state = core::mem::replace(&mut task.tokens, PreparedTokenState::Consumed);
		let PreparedTokenState::Available(tokens) = token_state else {
			return Err(StagedBridgeError::State {
				task: request.task,
				detail: "pending token was prepared more than once",
			});
		};
		let tokens = *tokens;
		Ok(StagedBridgePending {
			task: request.task,
			state: PendingState::Ready,
			source: ActiveLeg::from(tokens.source),
			destination: ActiveLeg::from(tokens.destination),
			middle: MiddleJobState::Uninitialized,
			target: DestinationTarget::Host,
		})
	}

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: LocalArenaSet<'_, '_, 'cuda, 'hsa>,
		pending: &mut Self::Pending,
		class: WorkClass,
		work: TransferWork<'_>,
	) -> Result<(), Self::Error> {
		let task = resource
			.tasks
			.get_mut(&work.task)
			.ok_or(StagedBridgeError::MissingTask { task: work.task })?;
		match pending.task == work.task && pending.state == PendingState::Ready {
			true => Ok(()),
			false => {
				Err(StagedBridgeError::State {
					task: work.task,
					detail: "staged pending token is not ready for this transfer",
				})
			}
		}?;
		task.contract.validate_work(class, &work)?;
		let source = arenas
			.get(task.contract.source.device)
			.ok_or(StagedBridgeError::MissingBinding {
				device: task.contract.source.device,
				class: task.contract.source.class,
			})?;
		let destination =
			arenas.get(task.contract.destination.device)
				.ok_or(StagedBridgeError::MissingBinding {
					device: task.contract.destination.device,
					class: task.contract.destination.class,
				})?;
		validate_arena(task.contract.source, source, work.task)?;
		validate_arena(task.contract.destination, destination, work.task)?;
		let source_offset = endpoint_offset(work.source, work.task)?;
		let destination_offset = endpoint_offset(work.destination, work.task)?;
		let bytes = byte_count_to_usize(work.bytes, work.task)?;
		let (source_leg, destination_leg) = (&mut task.source, &mut task.destination);
		let job = middle_job(
			work.task,
			source_leg,
			destination_leg,
			source,
			destination,
			StagedCopyRange {
				source_offset,
				destination_offset,
				bytes,
			},
		)?;
		pending.middle.initialize(work.task, job)?;
		pending.target = destination_target(destination, destination_offset);
		match source_leg {
			LegResources::Host => {
				pending.middle.submit(work.task, &task.worker)?;
				pending.state = PendingState::Middle;
			}
			LegResources::Cuda { .. } | LegResources::Hsa { .. } => {
				submit_source(
					work.task,
					source_leg,
					&mut pending.source,
					source,
					source_offset,
					bytes,
				)?;
				pending.state = PendingState::Source;
			}
		}
		Ok(())
	}

	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
	) -> Result<BackendPoll, Self::Error> {
		let task = resource
			.tasks
			.get_mut(&pending.task)
			.ok_or(StagedBridgeError::MissingTask { task: pending.task })?;
		match pending.state {
			PendingState::Source => {
				match poll_leg(pending.task, &mut pending.source)? {
					BackendPoll::Pending => Ok(BackendPoll::Pending),
					BackendPoll::Complete { .. } => {
						pending.middle.submit(pending.task, &task.worker)?;
						pending.state = PendingState::Middle;
						Ok(BackendPoll::Pending)
					}
				}
			}
			PendingState::Middle => {
				match task.worker.poll()? {
					BackendPoll::Pending => Ok(BackendPoll::Pending),
					BackendPoll::Complete { .. } => {
						match pending.target {
							DestinationTarget::Host => {
								pending.state = PendingState::Complete;
								Ok(BackendPoll::Complete { metric: None })
							}
							DestinationTarget::Cuda { .. } | DestinationTarget::Hsa { .. } => {
								submit_destination(
									pending.task,
									&task.destination,
									&mut pending.destination,
									&pending.target,
									byte_count_to_usize(task.contract.bytes, task.contract.task)?,
								)?;
								pending.state = PendingState::Destination;
								Ok(BackendPoll::Pending)
							}
						}
					}
				}
			}
			PendingState::Destination => {
				match poll_leg(pending.task, &mut pending.destination)? {
					BackendPoll::Pending => Ok(BackendPoll::Pending),
					BackendPoll::Complete { .. } => {
						pending.state = PendingState::Complete;
						Ok(BackendPoll::Complete { metric: None })
					}
				}
			}
			PendingState::Ready => {
				Err(StagedBridgeError::State {
					task: pending.task,
					detail: "staged transfer was polled before submission",
				})
			}
			PendingState::Complete => {
				Err(StagedBridgeError::State {
					task: pending.task,
					detail: "staged transfer was polled after completion",
				})
			}
		}
	}

	fn supports_loop_repetition(&self) -> bool { true }

	fn rearm_loop_pending(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
	) -> Result<(), Self::Error> {
		if pending.state == PendingState::Ready {
			return Ok(());
		}
		let task = resource
			.tasks
			.get_mut(&pending.task)
			.ok_or(StagedBridgeError::MissingTask { task: pending.task })?;
		if pending.state != PendingState::Complete {
			return Err(StagedBridgeError::State {
				task: pending.task,
				detail: "only a terminal staged loop token may be rearmed",
			});
		}
		task.worker.reset()?;
		rearm_leg(pending.task, &mut pending.source)?;
		rearm_leg(pending.task, &mut pending.destination)?;
		pending.middle = MiddleJobState::Uninitialized;
		pending.target = DestinationTarget::Host;
		pending.state = PendingState::Ready;
		Ok(())
	}

	fn destroy(&mut self, resource: Self::Resource) -> Result<(), Self::Error> { destroy_resources(resource) }
}

impl<'cuda, 'hsa> CandidateCrossBackendTransfer<'cuda, 'hsa> for StagedCrossBackend<'cuda, 'hsa> {
	fn realize_candidate(
		&mut self,
		candidate: &PlannedCandidate,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<Self::Resource, Self::Error> {
		self.realize_tasks(candidate.draft.tasks.iter(), tasks, devices)
	}

	fn validate_handoff(
		&mut self,
		resource: &Self::Resource,
		bundle: &FinalizedBundle,
		tasks: &BTreeSet<TaskId>,
		devices: &BTreeMap<DeviceId, LocalDeviceClass>,
	) -> Result<(), Self::Error> {
		self.validate_bindings(devices)?;
		match resource.tasks.keys().copied().eq(tasks.iter().copied()) {
			true => Ok(()),
			false => {
				Err(StagedBridgeError::MissingTask {
					task: first_missing_task(tasks, &resource.tasks),
				})
			}
		}?;
		for task in resource.tasks.values() {
			task.contract.validate_finalized(bundle, devices)?;
		}
		Ok(())
	}

	fn recycle_candidate_pending(
		&mut self,
		resource: &mut Self::Resource,
		pending: Self::Pending,
	) -> Result<(), Self::Error> {
		let task = resource
			.tasks
			.get_mut(&pending.task)
			.ok_or(StagedBridgeError::MissingTask { task: pending.task })?;
		match pending.state {
			PendingState::Complete => Ok(()),
			PendingState::Ready | PendingState::Source | PendingState::Middle | PendingState::Destination => {
				Err(StagedBridgeError::State {
					task: pending.task,
					detail: "only a terminal staged token may be recycled",
				})
			}
		}?;
		let source = recycle_leg(pending.task, pending.source)?;
		let destination = recycle_leg(pending.task, pending.destination)?;
		task.worker.reset()?;
		let prior = core::mem::replace(
			&mut task.tokens,
			PreparedTokenState::Available(Box::new(PreparedTokens {
				source,
				destination,
			})),
		);
		match prior {
			PreparedTokenState::Consumed => Ok(()),
			PreparedTokenState::Available(_) => {
				Err(StagedBridgeError::State {
					task: pending.task,
					detail: "staged resource already contains a pending token",
				})
			}
		}
	}
}

fn recycle_leg<'cuda, 'hsa>(
	task: TaskId,
	leg: ActiveLeg<'cuda, 'hsa>,
) -> Result<PreparedLegToken<'cuda, 'hsa>, StagedBridgeError> {
	match leg {
		ActiveLeg::Host => Ok(PreparedLegToken::Host),
		ActiveLeg::CudaReady(event) => Ok(PreparedLegToken::Cuda(event)),
		ActiveLeg::CudaActive(pending) => {
			pending
				.recycle_event()
				.map(PreparedLegToken::Cuda)
				.map_err(StagedBridgeError::from)
		}
		ActiveLeg::Hsa(mut pending) => {
			pending.reset()?;
			Ok(PreparedLegToken::Hsa(pending))
		}
		ActiveLeg::Transition => {
			Err(StagedBridgeError::State {
				task,
				detail: "staged native completion token is in transition",
			})
		}
	}
}

fn rearm_leg(task: TaskId, leg: &mut ActiveLeg<'_, '_>) -> Result<(), StagedBridgeError> {
	let active = core::mem::replace(leg, ActiveLeg::Transition);
	let ready = match active {
		ActiveLeg::Host => ActiveLeg::Host,
		ActiveLeg::CudaReady(event) => ActiveLeg::CudaReady(event),
		ActiveLeg::CudaActive(pending) => {
			ActiveLeg::CudaReady(pending.recycle_event().map_err(StagedBridgeError::from)?)
		}
		ActiveLeg::Hsa(mut pending) => {
			match pending.reset() {
				Ok(()) => ActiveLeg::Hsa(pending),
				Err(error) => {
					*leg = ActiveLeg::Hsa(pending);
					return Err(StagedBridgeError::from(error));
				}
			}
		}
		ActiveLeg::Transition => {
			return Err(StagedBridgeError::State {
				task,
				detail: "staged native completion token is in transition",
			});
		}
	};
	*leg = ready;
	Ok(())
}

#[derive(Clone, Copy, Debug)]
struct StagedCopyRange {
	source_offset: usize,
	destination_offset: usize,
	bytes: usize,
}

fn middle_job(
	task: TaskId,
	source_leg: &mut LegResources<'_, '_>,
	destination_leg: &mut LegResources<'_, '_>,
	source: LocalArenaRef<'_, '_, '_>,
	destination: LocalArenaRef<'_, '_, '_>,
	range: StagedCopyRange,
) -> Result<HostStageJob, StagedBridgeError> {
	match (source_leg, destination_leg, source, destination) {
		(
			LegResources::Host,
			destination_leg @ (LegResources::Cuda { .. } | LegResources::Hsa { .. }),
			LocalArenaRef::Host(source),
			LocalArenaRef::Cuda(_) | LocalArenaRef::Hsa(_),
		) => {
			Ok(HostStageJob::Read {
				arena: source.clone(),
				offset: usize_to_u64(range.source_offset, task)?,
				destination: destination_leg.host_address(task)?,
				bytes: range.bytes,
			})
		}
		(
			source_leg @ (LegResources::Cuda { .. } | LegResources::Hsa { .. }),
			LegResources::Host,
			LocalArenaRef::Cuda(_) | LocalArenaRef::Hsa(_),
			LocalArenaRef::Host(destination),
		) => {
			Ok(HostStageJob::Write {
				source: source_leg.host_address(task)?,
				arena: destination.clone(),
				offset: usize_to_u64(range.destination_offset, task)?,
				bytes: range.bytes,
			})
		}
		(
			source_leg @ (LegResources::Cuda { .. } | LegResources::Hsa { .. }),
			destination_leg @ (LegResources::Cuda { .. } | LegResources::Hsa { .. }),
			LocalArenaRef::Cuda(_) | LocalArenaRef::Hsa(_),
			LocalArenaRef::Cuda(_) | LocalArenaRef::Hsa(_),
		) => {
			Ok(HostStageJob::Copy {
				source: source_leg.host_address(task)?,
				destination: destination_leg.host_address(task)?,
				bytes: range.bytes,
			})
		}
		(
			LegResources::Host | LegResources::Cuda { .. } | LegResources::Hsa { .. },
			LegResources::Host | LegResources::Cuda { .. } | LegResources::Hsa { .. },
			LocalArenaRef::Host(_) | LocalArenaRef::Cuda(_) | LocalArenaRef::Hsa(_),
			LocalArenaRef::Host(_) | LocalArenaRef::Cuda(_) | LocalArenaRef::Hsa(_),
		) => {
			Err(StagedBridgeError::Contract {
				task,
				detail: "arena classes differ from their staged endpoint resources",
			})
		}
	}
}

fn destination_target<'cuda, 'hsa>(
	destination: LocalArenaRef<'_, 'cuda, 'hsa>,
	offset: usize,
) -> DestinationTarget<'cuda, 'hsa> {
	match destination {
		LocalArenaRef::Host(_) => DestinationTarget::Host,
		LocalArenaRef::Cuda(arena) => {
			DestinationTarget::Cuda {
				buffer: arena.buffer(),
				offset,
			}
		}
		LocalArenaRef::Hsa(arena) => {
			DestinationTarget::Hsa {
				allocation: arena.allocation(),
				offset,
			}
		}
	}
}

fn submit_source<'cuda, 'hsa>(
	task: TaskId,
	resources: &mut LegResources<'cuda, 'hsa>,
	pending: &mut ActiveLeg<'cuda, 'hsa>,
	arena: LocalArenaRef<'_, 'cuda, 'hsa>,
	offset: usize,
	bytes: usize,
) -> Result<(), StagedBridgeError> {
	match (resources, pending, arena) {
		(LegResources::Cuda { stream, staging }, pending @ ActiveLeg::CudaReady(_), LocalArenaRef::Cuda(arena)) => {
			let event = consume_cuda_event(task, pending, "CUDA source event was already consumed")?;
			// SAFETY: the staged pending token is retained until terminal
			// completion, and both arena and staging outlive the executor phase.
			let native = unsafe { stream.copy_d2h(staging, 0, arena.buffer(), offset, bytes, event) }?;
			*pending = ActiveLeg::CudaActive(erase_cuda_pending_lifetime(native));
			Ok(())
		}
		(LegResources::Hsa { session, staging }, ActiveLeg::Hsa(pending), LocalArenaRef::Hsa(arena)) => {
			session
				.copy_async_prepared(staging, 0, arena.allocation(), offset, bytes, pending)
				.map_err(StagedBridgeError::from)
		}
		(
			LegResources::Host | LegResources::Cuda { .. } | LegResources::Hsa { .. },
			ActiveLeg::Host
			| ActiveLeg::CudaReady(_)
			| ActiveLeg::CudaActive(_)
			| ActiveLeg::Hsa(_)
			| ActiveLeg::Transition,
			LocalArenaRef::Host(_) | LocalArenaRef::Cuda(_) | LocalArenaRef::Hsa(_),
		) => {
			Err(StagedBridgeError::State {
				task,
				detail: "source arena, staging resource, and completion token disagree",
			})
		}
	}
}

fn submit_destination<'cuda, 'hsa>(
	task: TaskId,
	resources: &LegResources<'cuda, 'hsa>,
	pending: &mut ActiveLeg<'cuda, 'hsa>,
	target: &DestinationTarget<'cuda, 'hsa>,
	bytes: usize,
) -> Result<(), StagedBridgeError> {
	match (resources, pending, target) {
		(
			LegResources::Cuda { stream, staging },
			pending @ ActiveLeg::CudaReady(_),
			DestinationTarget::Cuda { buffer, offset },
		) => {
			let event = consume_cuda_event(task, pending, "CUDA destination event was already consumed")?;
			// SAFETY: `buffer` points into the executor arena map, which owns
			// every arena until all phase pending tokens are terminal.
			let buffer = unsafe { &**buffer };
			// SAFETY: bounds are checked by the CUDA wrapper and the returned
			// token remains owned through terminal polling.
			let native = unsafe { stream.copy_h2d(buffer, *offset, staging, 0, bytes, event) }?;
			*pending = ActiveLeg::CudaActive(erase_cuda_pending_lifetime(native));
			Ok(())
		}
		(
			LegResources::Hsa { session, staging },
			ActiveLeg::Hsa(pending),
			DestinationTarget::Hsa { allocation, offset },
		) => {
			// SAFETY: `allocation` points into the executor arena map retained
			// until this pending token reaches terminal completion.
			let allocation = unsafe { &**allocation };
			session
				.copy_async_prepared(allocation, *offset, staging, 0, bytes, pending)
				.map_err(StagedBridgeError::from)
		}
		(
			LegResources::Host | LegResources::Cuda { .. } | LegResources::Hsa { .. },
			ActiveLeg::Host
			| ActiveLeg::CudaReady(_)
			| ActiveLeg::CudaActive(_)
			| ActiveLeg::Hsa(_)
			| ActiveLeg::Transition,
			DestinationTarget::Host | DestinationTarget::Cuda { .. } | DestinationTarget::Hsa { .. },
		) => {
			Err(StagedBridgeError::State {
				task,
				detail: "destination arena, staging resource, and completion token disagree",
			})
		}
	}
}

fn consume_cuda_event<'cuda, 'hsa>(
	task: TaskId,
	pending: &mut ActiveLeg<'cuda, 'hsa>,
	detail: &'static str,
) -> Result<Event<'cuda>, StagedBridgeError> {
	let prior = core::mem::replace(pending, ActiveLeg::Transition);
	match prior {
		ActiveLeg::CudaReady(event) => Ok(event),
		other @ (ActiveLeg::Host | ActiveLeg::CudaActive(_) | ActiveLeg::Hsa(_) | ActiveLeg::Transition) => {
			*pending = other;
			Err(StagedBridgeError::State { task, detail })
		}
	}
}

fn poll_leg(task: TaskId, pending: &mut ActiveLeg<'_, '_>) -> Result<BackendPoll, StagedBridgeError> {
	match pending {
		ActiveLeg::CudaActive(native) => {
			match native.poll()? {
				CudaCompletionStatus::Pending => Ok(BackendPoll::Pending),
				CudaCompletionStatus::Complete => Ok(BackendPoll::Complete { metric: None }),
			}
		}
		ActiveLeg::Hsa(native) => {
			match native.poll()? {
				HsaPollStatus::Pending => Ok(BackendPoll::Pending),
				HsaPollStatus::Complete => Ok(BackendPoll::Complete { metric: None }),
			}
		}
		ActiveLeg::Host | ActiveLeg::CudaReady(_) | ActiveLeg::Transition => {
			Err(StagedBridgeError::State {
				task,
				detail: "native staged operation was polled before submission",
			})
		}
	}
}

fn validate_arena(
	endpoint: EndpointContract,
	arena: LocalArenaRef<'_, '_, '_>,
	task: TaskId,
) -> Result<(), StagedBridgeError> {
	match arena.device() == endpoint.device && arena.class() == endpoint.class {
		true => Ok(()),
		false => {
			Err(StagedBridgeError::Contract {
				task,
				detail: "executor arena differs from the staged endpoint owner",
			})
		}
	}
}

fn endpoint_offset(endpoint: ResolvedTransferEndpoint, task: TaskId) -> Result<usize, StagedBridgeError> {
	let ResolvedTransferEndpoint::Device(location) = endpoint else {
		return Err(StagedBridgeError::Contract {
			task,
			detail: "staged endpoint is external",
		});
	};
	match usize::try_from(location.arena_offset.get()) {
		Ok(offset) => Ok(offset),
		Err(source) => {
			Err(StagedBridgeError::IntegerConversion {
				task,
				detail: "the staged arena offset",
				source,
			})
		}
	}
}

fn byte_count_to_usize(bytes: ByteCount, task: TaskId) -> Result<usize, StagedBridgeError> {
	match usize::try_from(bytes.get()) {
		Ok(bytes) => Ok(bytes),
		Err(source) => {
			Err(StagedBridgeError::IntegerConversion {
				task,
				detail: "the staged transfer size",
				source,
			})
		}
	}
}

fn usize_to_u64(value: usize, task: TaskId) -> Result<u64, StagedBridgeError> {
	match u64::try_from(value) {
		Ok(value) => Ok(value),
		Err(source) => {
			Err(StagedBridgeError::IntegerConversion {
				task,
				detail: "the staged host offset",
				source,
			})
		}
	}
}

fn first_missing_task<T>(selected: &BTreeSet<TaskId>, resources: &BTreeMap<TaskId, T>) -> TaskId {
	let missing = selected
		.iter()
		.find(|task| !resources.contains_key(task))
		.copied()
		.or_else(|| {
			resources
				.keys()
				.find(|task| !selected.contains(task))
				.copied()
		});
	match missing {
		Some(task) => task,
		None => TaskId::new(0),
	}
}

fn first_error(
	first: Result<(), StagedBridgeError>,
	second: Result<(), StagedBridgeError>,
) -> Result<(), StagedBridgeError> {
	match first {
		Err(error) => Err(error),
		Ok(()) => second,
	}
}

fn destroy_resources(resource: StagedBridgeResources<'_, '_>) -> Result<(), StagedBridgeError> {
	let mut first = None;
	for (_, task) in resource.tasks {
		let StagedTaskResources {
			source,
			destination,
			tokens,
			worker,
			..
		} = task;
		drop(tokens);
		retain_first(&mut first, worker.close());
		retain_first(&mut first, destination.destroy());
		retain_first(&mut first, source.destroy());
	}
	match first {
		Some(error) => Err(error),
		None => Ok(()),
	}
}

fn retain_first(first: &mut Option<StagedBridgeError>, result: Result<(), StagedBridgeError>) {
	let Err(error) = result else {
		return;
	};
	match first {
		None => *first = Some(error),
		Some(_) => drop(error),
	}
}

fn erase_cuda_pending_lifetime<'operation, 'context>(
	pending: CudaPending<'operation, 'context>,
) -> CudaPending<'static, 'context> {
	// SAFETY: `CudaPending` carries only a phantom operation borrow. The bridge
	// retains both arena and staging allocations until terminal polling.
	let widen = |value| unsafe {
		core::mem::transmute::<CudaPending<'operation, 'context>, CudaPending<'static, 'context>>(value)
	};
	widen(pending)
}
