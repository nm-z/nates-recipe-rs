use std::{collections::BTreeSet, fmt, num::NonZeroU64};

use crate::{ error::{ValidationCode, ValidationResult, Validator}, ids::{
		ArenaObjectId, ArtifactId, CompletionSlotId, DeviceId, KernelTemplateId, LinkId, MetricId, MetricSlotId,
		QueueSlotId, TaskId, ValueId, }, scalar::{AliasPermission, DType}, topology::Topology,
	units::{ByteCount, ByteOffset, FlopCount, Nanoseconds}, };

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunPhase { Init, Loop, Exit, }

/// Declared lifetime of the immutable loop task graph.
///
/// A finite loop executes an exact nonzero number of iterations. An unbounded
/// loop has no invented terminal iteration and may leave the loop only through
/// an explicit graceful-stop boundary or failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoopIterations { Finite(NonZeroU64), Unbounded, }

impl LoopIterations { pub const ONE: Self = Self::Finite(NonZeroU64::MIN); pub const UNBOUNDED: Self = Self::Unbounded;

	#[must_use]
	pub const fn new(iterations: u64) -> Option<Self> { match NonZeroU64::new(iterations) {
			Some(iterations) => Some(Self::Finite(iterations)), None => None, } }

	#[must_use]
	pub const fn finite(self) -> Option<NonZeroU64> { match self { Self::Finite(iterations) => Some(iterations),
			Self::Unbounded => None, } }

	#[must_use]
	pub const fn is_unbounded(self) -> bool { matches!(self, Self::Unbounded) }

	#[must_use]
	pub const fn from_nonzero(iterations: NonZeroU64) -> Self { Self::Finite(iterations) }

	#[must_use]
	pub const fn iteration(self, index: u64) -> Option<LoopIteration> { match self {
			Self::Finite(iterations) if index >= iterations.get() => None,
			Self::Finite(_) | Self::Unbounded => Some(LoopIteration { index, total: self }), } } }

impl Default for LoopIterations { fn default() -> Self { Self::ONE } }

impl From<NonZeroU64> for LoopIterations { fn from(iterations: NonZeroU64) -> Self { Self::from_nonzero(iterations) } }

impl fmt::Display for LoopIterations {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self { Self::Finite(iterations) => iterations.fmt(formatter),
			Self::Unbounded => formatter.write_str("unbounded"),
		} } }

/// One zero-based execution of a finalized loop task graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoopIteration { index: u64, total: LoopIterations, }

impl LoopIteration {
	#[must_use]
	pub const fn index(self) -> u64 { self.index }

	#[must_use]
	pub const fn total(self) -> LoopIterations { self.total } }

/// One nonempty arithmetic progression within a finite or unbounded loop.
///
/// Domains are zero-based and half-open. They select submissions without
/// unrolling the immutable task graph or allocating per iteration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IterationDomain { first: u64, end: IterationEnd, stride: NonZeroU64, }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum IterationEnd { Exclusive(u64), Unbounded, }

impl IterationDomain {
	#[must_use]
	pub const fn new(first: u64, end_exclusive: u64, stride: u64) -> Option<Self> {
		let Some(stride) = NonZeroU64::new(stride) else { return None; }; if first >= end_exclusive { return None; }
		Some(Self { first, end: IterationEnd::Exclusive(end_exclusive), stride, }) }

	#[must_use]
	pub const fn unbounded(first: u64, stride: u64) -> Option<Self> { let Some(stride) = NonZeroU64::new(stride) else {
			return None; }; Some(Self { first, end: IterationEnd::Unbounded, stride, }) }

	#[must_use]
	pub const fn every(iterations: LoopIterations) -> Self { match iterations { LoopIterations::Finite(iterations) => {
				Self { first: 0, end: IterationEnd::Exclusive(iterations.get()), stride: NonZeroU64::MIN, } }
			LoopIterations::Unbounded => { Self { first: 0, end: IterationEnd::Unbounded, stride: NonZeroU64::MIN, } } } }

	#[must_use]
	pub const fn first() -> Self { Self { first: 0, end: IterationEnd::Exclusive(1), stride: NonZeroU64::MIN, } }


	#[must_use]
	pub const fn first_iteration(self) -> u64 { self.first }

	#[must_use]
	pub const fn end_exclusive(self) -> Option<u64> { match self.end { IterationEnd::Exclusive(end) => Some(end),
			IterationEnd::Unbounded => None, } }

	#[must_use]
	pub const fn is_unbounded(self) -> bool { matches!(self.end, IterationEnd::Unbounded) }

	#[must_use]
	pub const fn stride(self) -> NonZeroU64 { self.stride }

	#[must_use]
	pub const fn contains(self, iteration: u64) -> bool { iteration >= self.first && match self.end {
				IterationEnd::Exclusive(end) => iteration < end, IterationEnd::Unbounded => true,
			} && (iteration - self.first).is_multiple_of(self.stride.get()) }

	#[must_use]
	pub const fn is_within(self, iterations: LoopIterations) -> bool { match (self.end, iterations) {
			(IterationEnd::Exclusive(end), LoopIterations::Finite(iterations)) => { self.first < end && end <= iterations.get() }
			(IterationEnd::Exclusive(end), LoopIterations::Unbounded) => self.first < end,
			(IterationEnd::Unbounded, LoopIterations::Unbounded) => true,
			(IterationEnd::Unbounded, LoopIterations::Finite(_)) => false, } } }

/// Exact activation domain assigned to one immutable loop task.
///
/// This schedule sidecar preserves the long-standing [`Task`] layout while
/// making the task/domain association complete and independently validated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoopTaskDomain { pub task: TaskId, pub domain: IterationDomain, }

/// Complete activation schedule supplied to Finalize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopSchedule { iterations: LoopIterations, domains: Vec<LoopTaskDomain>, }

impl LoopSchedule {
	#[must_use]
	pub const fn new(iterations: LoopIterations, domains: Vec<LoopTaskDomain>) -> Self { Self { iterations, domains, } }

	#[must_use]
	pub const fn iterations(&self) -> LoopIterations { self.iterations }

	#[must_use]
	pub fn domains(&self) -> &[LoopTaskDomain] { &self.domains }

	#[must_use]
	pub fn into_parts(self) -> (LoopIterations, Vec<LoopTaskDomain>) { (self.iterations, self.domains) } }

/// Half-open interval in the deterministic schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleWindow { pub start: Nanoseconds, pub end: Nanoseconds, }

impl ScheduleWindow {
	#[must_use]
	pub fn is_valid(self) -> bool { self.start < self.end }

	#[must_use]
	pub fn overlaps(self, other: Self) -> bool { self.start < other.end && other.start < self.end } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmissionSlots { pub queue: QueueSlotId, pub completion: CompletionSlotId, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueSpec { pub id: ValueId, pub dtype: DType, pub bytes: ByteCount, pub device: DeviceId,
	pub producer: Option<TaskId>, }

/// Draft-time logical placement of a value within an arena object.
///
/// `object_offset` is relative to the object and therefore does not choose a
/// physical arena offset. Finalize combines it with the validated
/// [`ArenaAllocation`] selected for the object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueBinding { pub value: ValueId, pub object: ArenaObjectId, pub object_offset: ByteOffset, }

/// Exact storage relationship selected for one primitive source alias pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueAliasContract { pub input: ValueId, pub output: ValueId, pub permission: AliasPermission, }

/// One graph-level external input embedded into a device's single logical
/// init image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InitDataImageMember {
	/// Stable value identity from the backend-neutral calculation graph.
	pub logical: ValueId,
	/// Candidate-specific resident copy addressed by the finalized bundle.
	pub physical: ValueId, pub dtype: DType, pub bytes: ByteCount,
	/// Offset relative to the beginning of this device's logical init image.
	pub image_offset: ByteOffset, }

/// Exact packing manifest for one required device's single init admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitDataImage { pub device: DeviceId,
	/// Resident value spanning the complete packed image.
	pub image: ValueId, pub bytes: ByteCount, pub members: Vec<InitDataImageMember>, }

/// Immutable value address resolved by Finalize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedValueLocation { pub value: ValueId, pub dtype: DType, pub device: DeviceId, pub bytes: ByteCount,
	pub object: ArenaObjectId, pub object_offset: ByteOffset, pub arena_offset: ByteOffset, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalculationTask { pub device: DeviceId, pub kernel_template: KernelTemplateId, pub artifact: ArtifactId,
	pub inputs: Vec<ValueId>, pub outputs: Vec<ValueId>,
	/// Preallocated int32 device fault flag required by checked scalar
	/// operations. It is absent only when the kernel cannot report a fault.
	pub fault_flag: Option<ValueId>, pub work: FlopCount, pub submission: SubmissionSlots, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferEndpoint { External, Device { device: DeviceId, value: ValueId }, }

impl TransferEndpoint {
	#[must_use]
	pub(crate) const fn value(self) -> Option<ValueId> { match self { Self::External => None,
			Self::Device { value, .. } => Some(value), } } }

/// One exact zero-based transfer lane selected during static scheduling.
///
/// An internal transfer is one executor-visible hop and therefore claims
/// exactly one lane on its single directed link. A same-device internal copy
/// has no link lane. Logical external admission or egress claims one lane on
/// its device transfer capability. An unscheduled transfer carries an empty
/// list; a scheduled Draft carries the complete canonical claim list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferLaneClaim { Link { link: LinkId, lane: u32 }, External { device: DeviceId, lane: u32 }, }

/// One physical transfer submission.
///
/// A device-to-device task contains either one directed link or, for an
/// explicit same-device copy, no link. Multi-link paths are represented by
/// dependency-chained tasks with resident intermediate values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferTask { pub source: TransferEndpoint, pub destination: TransferEndpoint, pub bytes: ByteCount,
	pub route: Vec<LinkId>, pub lane_claims: Vec<TransferLaneClaim>, pub submission: SubmissionSlots, }

/// A transfer endpoint after Finalize has resolved every logical value to its
/// immutable arena location.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedTransferEndpoint { External, Device(ResolvedValueLocation), }

/// Both resolved endpoints of one finalized transfer task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedTransferEndpoints { pub source: ResolvedTransferEndpoint, pub destination: ResolvedTransferEndpoint,
}

/// Machine-readable reason for copying one four-byte device value to fixed
/// host-visible metric staging.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetricPurpose {
	/// User-facing, nonblocking, newest-value-wins telemetry.
	User,
	/// Mandatory fail-closed readback for every checked calculation sharing
	/// this metric's preallocated device fault flag.
	FaultReadback, }

/// Asynchronous four-byte readback into a preallocated metric slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetricTask { pub purpose: MetricPurpose, pub metric: MetricId, pub value: ValueId, pub slot: MetricSlotId,
	pub submission: SubmissionSlots, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskKind { Calculation(CalculationTask), Transfer(TransferTask), Metric(MetricTask), }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task { pub id: TaskId, pub phase: RunPhase, pub window: ScheduleWindow, pub dependencies: Vec<TaskId>,
	pub kind: TaskKind, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueSlot { pub id: QueueSlotId, pub device: DeviceId, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionSlot { pub id: CompletionSlotId, pub device: DeviceId, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetricSlot { pub id: MetricSlotId, pub metric: MetricId, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceBytes { pub device: DeviceId, pub bytes: ByteCount, }

/// Exact preallocated resource choices fixed by Draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceManifest { pub queues: Vec<QueueSlot>, pub completions: Vec<CompletionSlot>,
	pub metrics: Vec<MetricSlot>, pub pinned_staging: Vec<DeviceBytes>, pub scratch: Vec<DeviceBytes>, }

impl ResourceManifest { pub fn validate(&self, topology: &Topology) -> ValidationResult {
		let mut validator = Validator::default(); let devices = topology .devices .iter() .map(|device| device.id)
			.collect::<BTreeSet<_>>(); let mut queues = BTreeSet::new(); for (index, slot) in self.queues.iter().enumerate() {
			validator.require( queues.insert(slot.id), ValidationCode::DuplicateId,
				format!("queues[{index}].id"),
				format!("queue slot {} appears more than once", slot.id),
			); validator.require( devices.contains(&slot.device), ValidationCode::UnknownReference,
				format!("queues[{index}].device"),
				format!("unknown device {}", slot.device),
			); }
		let mut completions = BTreeSet::new(); for (index, slot) in self.completions.iter().enumerate() { validator.require(
				completions.insert(slot.id), ValidationCode::DuplicateId,
				format!("completions[{index}].id"),
				format!("completion slot {} appears more than once", slot.id),
			); validator.require( devices.contains(&slot.device), ValidationCode::UnknownReference,
				format!("completions[{index}].device"),
				format!("unknown device {}", slot.device),
			); }
		let mut metrics = BTreeSet::new(); for (index, slot) in self.metrics.iter().enumerate() { validator.require(
				metrics.insert(slot.id), ValidationCode::DuplicateId,
				format!("metrics[{index}].id"),
				format!("metric slot {} appears more than once", slot.id),
			); }
		for (field, entries) in [
			("pinned_staging", &self.pinned_staging),
			("scratch", &self.scratch),
		] { let mut seen = BTreeSet::new(); for (index, entry) in entries.iter().enumerate() { validator.require(
					seen.insert(entry.device), ValidationCode::DuplicateId,
					format!("{field}[{index}].device"),
					format!("device {} appears more than once", entry.device),
				); validator.require( devices.contains(&entry.device), ValidationCode::UnknownReference,
					format!("{field}[{index}].device"),
					format!("unknown device {}", entry.device),
				); } }
		validator.finish() }

	#[must_use]
	pub fn queue(&self, id: QueueSlotId) -> Option<&QueueSlot> { self.queues.iter().find(|slot| slot.id == id) }

	#[must_use]
	pub fn completion(&self, id: CompletionSlotId) -> Option<&CompletionSlot> { self.completions.iter().find(|slot| slot.id == id) }

	#[must_use]
	pub fn metric(&self, id: MetricSlotId) -> Option<&MetricSlot> { self.metrics.iter().find(|slot| slot.id == id) } }

/// Logical object whose physical offset is deliberately absent from Draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaObject { pub id: ArenaObjectId, pub device: DeviceId, pub bytes: ByteCount, pub alignment: ByteCount,
	pub lifetime: ScheduleWindow, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaRelease { pub device: DeviceId, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaAllocation { pub object: ArenaObjectId, pub offset: ByteOffset, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaLayout { pub device: DeviceId, pub size: ByteCount, pub allocations: Vec<ArenaAllocation>, }
