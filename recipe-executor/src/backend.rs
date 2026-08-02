use std::{collections::BTreeMap, error::Error, fmt::Debug};

use recipe_core::{
	ArenaLayout, ArtifactId, ByteCount, DeviceId, FinalizedBundle, KernelTemplateId, LinkId, LoopIteration, MetricId,
	MetricPurpose, MetricSlotId, ResolvedTransferEndpoint, ResolvedValueLocation, RunId, RunPhase, SubmissionSlots,
	TaskId, TransferLaneClaim,
};

use crate::metrics::MetricValue;

#[doc(hidden)]
pub mod sealed {
	pub trait Sealed {}
}

/// Closed work classes understood by every Recipe backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkClass {
	InitAdmission,
	Calculation,
	InternalTransfer,
	Metric,
	ExitTransfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InitAdmissionWork<'a> {
	pub task: TaskId,
	pub destination: ResolvedValueLocation,
	pub bytes: ByteCount,
	pub submission: SubmissionSlots,
	pub image: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalculationWork<'a> {
	pub task: TaskId,
	pub run: RunId,
	/// Zero-based finalized loop iteration for recurrent-state and schedule addressing.
	pub iteration: LoopIteration,
	pub device: DeviceId,
	pub kernel_template: KernelTemplateId,
	pub artifact: ArtifactId,
	pub submission: SubmissionSlots,
	pub inputs: &'a [ResolvedValueLocation],
	pub outputs: &'a [ResolvedValueLocation],
	pub fault_flag: Option<ResolvedValueLocation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransferWork<'a> {
	pub task: TaskId,
	pub source: ResolvedTransferEndpoint,
	pub destination: ResolvedTransferEndpoint,
	pub bytes: ByteCount,
	pub route: &'a [LinkId],
	pub lane_claims: &'a [TransferLaneClaim],
	pub submission: SubmissionSlots,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetricWork {
	pub task: TaskId,
	/// Zero-based finalized loop iteration whose value is being emitted.
	pub iteration: LoopIteration,
	pub purpose: MetricPurpose,
	pub metric: MetricId,
	pub slot: MetricSlotId,
	pub value: ResolvedValueLocation,
	pub submission: SubmissionSlots,
}

/// One immutable, fully resolved operation handed to a backend.
///
/// There is intentionally no compiler, loader, allocator, discovery, or
/// topology-mutation variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendWork<'a> {
	InitAdmission(InitAdmissionWork<'a>),
	Calculation(CalculationWork<'a>),
	InternalTransfer(TransferWork<'a>),
	Metric(MetricWork),
	ExitTransfer(TransferWork<'a>),
}

impl BackendWork<'_> {
	#[must_use]
	pub const fn task(&self) -> TaskId {
		match self {
			Self::InitAdmission(work) => work.task,
			Self::Calculation(work) => work.task,
			Self::InternalTransfer(work) | Self::ExitTransfer(work) => work.task,
			Self::Metric(work) => work.task,
		}
	}

	#[must_use]
	pub const fn class(&self) -> WorkClass {
		match self {
			Self::InitAdmission(_) => WorkClass::InitAdmission,
			Self::Calculation(_) => WorkClass::Calculation,
			Self::InternalTransfer(_) => WorkClass::InternalTransfer,
			Self::Metric(_) => WorkClass::Metric,
			Self::ExitTransfer(_) => WorkClass::ExitTransfer,
		}
	}
}

/// One pre-init request for a backend-owned completion/submission token.
///
/// Backends must allocate, load, warm, and otherwise realize every resource
/// reachable from the returned token before this method returns. [`Backend::submit`]
/// and [`Backend::poll`] are then required to mutate only this pre-realized
/// token and other preallocated resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingRequest {
	pub task: TaskId,
	pub phase: RunPhase,
	pub class: WorkClass,
	pub submission: Option<SubmissionSlots>,
}

/// Read-only access to the exact arenas allocated from a finalized bundle.
#[derive(Debug)]
pub struct ArenaSet<'a, A> {
	arenas: &'a BTreeMap<DeviceId, A>,
}

impl<'a, A> ArenaSet<'a, A> {
	/// Constructs a read-only arena view for a pre-realized executor.
	///
	/// This is public only for composite backends and pre-final warm
	/// executors. Callers cannot mutate or extend the arena map through it.
	#[doc(hidden)]
	pub const fn new(arenas: &'a BTreeMap<DeviceId, A>) -> Self { Self { arenas } }

	#[must_use]
	pub fn get(&self, device: DeviceId) -> Option<&A> { self.arenas.get(&device) }

	pub fn iter(&self) -> impl ExactSizeIterator<Item = (DeviceId, &A)> {
		self.arenas.iter().map(|(device, arena)| (*device, arena))
	}

	#[must_use]
	pub fn len(&self) -> usize { self.arenas.len() }

	#[must_use]
	pub fn is_empty(&self) -> bool { self.arenas.is_empty() }
}

/// Nonblocking result of querying one backend completion token.
#[derive(Clone, Debug, PartialEq)]
pub enum BackendPoll {
	Pending,
	Complete { metric: Option<MetricValue> },
}

/// Status attached to a physical poll record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalPollStatus {
	Pending,
	Complete,
	Failed,
}

/// A backend-reported physical action.
///
/// Logical lifecycle accounting is kept in a separate [`crate::RunJournal`]
/// stream. In particular, one logical admission may report multiple physical
/// chunks here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalCall {
	BindResources,
	PreparePending {
		task: TaskId,
	},
	PrepareExternal {
		task: TaskId,
	},
	AllocateArena {
		device: DeviceId,
		bytes: ByteCount,
	},
	AdmissionChunk {
		task: TaskId,
		device: DeviceId,
		bytes: ByteCount,
		chunk_index: u32,
	},
	SubmitCalculation {
		task: TaskId,
	},
	SubmitInternalTransfer {
		task: TaskId,
	},
	SubmitMetric {
		task: TaskId,
		slot: MetricSlotId,
	},
	SubmitExitTransfer {
		task: TaskId,
	},
	SubmitExternalIngress {
		task: TaskId,
		bytes: ByteCount,
	},
	SubmitExternalEgress {
		task: TaskId,
		bytes: ByteCount,
	},
	AcknowledgeExternalEgress {
		task: TaskId,
	},
	QuiesceWorker,
	CollectExit {
		task: TaskId,
		bytes: ByteCount,
	},
	Poll {
		task: TaskId,
		status: PhysicalPollStatus,
	},
	ReleaseArena {
		device: DeviceId,
	},
	DestroyResources,
}

/// Maximum physical records one backend operation may report inline.
///
/// The bound is deliberately part of the backend ABI: returning an owned
/// collection from a loop operation would permit hidden allocation or growth.
pub const MAX_PHYSICAL_CALLS_PER_OPERATION: usize = 16;

/// Fixed-capacity physical-call report for one backend operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalCallBatch {
	calls: [Option<PhysicalCall>; MAX_PHYSICAL_CALLS_PER_OPERATION],
	len: u8,
}

impl PhysicalCallBatch {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			calls: [None; MAX_PHYSICAL_CALLS_PER_OPERATION],
			len: 0,
		}
	}

	#[must_use]
	pub const fn single(call: PhysicalCall) -> Self {
		let mut calls = [None; MAX_PHYSICAL_CALLS_PER_OPERATION];
		calls[0] = Some(call);
		Self { calls, len: 1 }
	}

	pub fn try_push(&mut self, call: PhysicalCall) -> Result<(), PhysicalCallBatchOverflow> {
		let index = usize::from(self.len);
		match index == MAX_PHYSICAL_CALLS_PER_OPERATION {
			true => Err(PhysicalCallBatchOverflow),
			false => {
				self.calls[index] = Some(call);
				self.len += 1;
				Ok(())
			}
		}
	}

	pub fn try_from_array<const N: usize>(calls: [PhysicalCall; N]) -> Result<Self, PhysicalCallBatchOverflow> {
		match N > MAX_PHYSICAL_CALLS_PER_OPERATION {
			true => Err(PhysicalCallBatchOverflow),
			false => {
				let mut batch = Self::new();
				for call in calls {
					batch.try_push(call)?;
				}
				Ok(batch)
			}
		}
	}

	pub fn iter(&self) -> impl Iterator<Item = PhysicalCall> + '_ {
		self.calls[..self.len()].iter().map(|call| {
			match call {
				Some(call) => *call,
				None => unreachable!("the initialized physical-call prefix contains records"),
			}
		})
	}

	#[must_use]
	pub fn len(&self) -> usize { usize::from(self.len) }

	#[must_use]
	pub const fn is_empty(&self) -> bool { self.len == 0 }
}

impl Default for PhysicalCallBatch {
	fn default() -> Self { Self::new() }
}

/// A backend attempted to exceed its fixed per-operation reporting bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalCallBatchOverflow;

impl std::fmt::Display for PhysicalCallBatchOverflow {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			formatter,
			"backend physical-call batch exceeds the fixed capacity of {MAX_PHYSICAL_CALLS_PER_OPERATION}"
		)
	}
}

impl Error for PhysicalCallBatchOverflow {}

/// Minimal backend surface required by the typestate executor.
///
/// This trait is sealed to Recipe-owned native adapter crates. Implementations
/// translate these closed operations onto pre-realized driver resources. The
/// executor never exposes a backend reference after it has been moved into a
/// run.
///
/// `submit` and `poll`, including formatting their errors, must not allocate,
/// grow collections, load code, or lazily initialize driver state. All such
/// work belongs in `bind_resources` or `prepare_pending`, both of which finish
/// before `init`.
pub trait Backend: sealed::Sealed {
	type Arena: Debug;
	type Resource: Debug;
	type Pending: Debug;
	type Error: Error + Send + Sync + 'static;

	/// Maximum records emitted by any one operation other than [`Self::poll`].
	///
	/// Recipe's executor uses this closed backend contract to preallocate an
	/// exact finalized-run journal. Implementations may choose a smaller bound
	/// than the ABI-wide [`MAX_PHYSICAL_CALLS_PER_OPERATION`], but must never
	/// exceed the declared value.
	const MAX_NON_POLL_PHYSICAL_CALLS: usize = MAX_PHYSICAL_CALLS_PER_OPERATION;

	fn bind_resources(
		&mut self,
		bundle: &FinalizedBundle,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Resource, Self::Error>;

	/// Realizes one reusable pending token before `init`.
	fn prepare_pending(
		&mut self,
		resource: &mut Self::Resource,
		request: PendingRequest,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Pending, Self::Error>;

	fn allocate_arena(
		&mut self,
		resource: &mut Self::Resource,
		layout: &ArenaLayout,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<Self::Arena, Self::Error>;

	/// Declares that loop pending tokens can be submitted, completed, and
	/// allocation-free rearmed for another finalized iteration.
	///
	/// The compatibility default is one-shot only. Executors reject bundles
	/// whose loop count exceeds one unless the backend opts in.
	#[must_use]
	fn supports_loop_repetition(&self) -> bool { false }

	/// Reports whether one loop task may be submitted behind an incomplete
	/// predecessor on the same native submission queue.
	///
	/// The default preserves completion-token ownership for backends whose
	/// pending objects must reach terminal state before another submission can
	/// reuse the queue. Backends opting in must make queue ordering sufficient
	/// and keep every submitted task's resources live until the queue is idle.
	#[must_use]
	fn supports_same_queue_pipelining(&self, _resource: &Self::Resource, _task: TaskId) -> bool { false }

	fn submit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: BackendWork<'_>,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error>;

	/// Submits one task in an exact zero-based loop iteration.
	///
	/// A sparse task's first active iteration may be nonzero. Repeatable
	/// backends must therefore decide whether to use the freshly prepared
	/// token or rearm a terminal token from the token's own activation state,
	/// never from the global iteration index. The default preserves one-shot
	/// backend behavior by forwarding to [`Self::submit`].
	fn submit_loop_iteration(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		iteration: LoopIteration,
		work: BackendWork<'_>,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error> {
		let _ = iteration;
		self.submit(resource, arenas, pending, work, physical_calls)
	}

	/// Poll one pending task.
	///
	/// Implementations must append exactly one [`PhysicalCall::Poll`] for the
	/// pending task, with a status matching the returned result, and no other
	/// physical record. Pending records are counted exactly in a fixed per-task
	/// journal table; complete and failed records remain ordered calls.
	fn poll(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::Pending,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<BackendPoll, Self::Error>;

	/// Copies one completed external-exit payload into caller-owned storage.
	///
	/// This method is invoked only during `exit`, never during the live loop.
	fn collect_exit(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: TransferWork<'_>,
		destination: &mut [u8],
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error>;

	fn release_arena(
		&mut self,
		resource: &mut Self::Resource,
		device: DeviceId,
		arena: Self::Arena,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error>;

	fn destroy_resources(
		&mut self,
		resource: Self::Resource,
		physical_calls: &mut PhysicalCallBatch,
	) -> std::result::Result<(), Self::Error>;
}
