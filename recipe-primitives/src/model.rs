use core::fmt;

use recipe_core::{
	AliasPermission, ByteCount, DType, FlopCount, KernelTemplate, KernelTemplateId, ScalarLiteral, ValueId,
};
use recipe_language::{
	AtomicOperation, AtomicOrdering, Contraction, IndexBounds, RandomDistribution, RandomKey, ReduceOperator,
	ReduceResult, ScatterConflict, SortDirection,
};

use crate::error::ProgramValidationError;

pub const LOWERED_PROGRAM_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramDigest([u8; 32]);

impl ProgramDigest {
	pub const ZERO: Self = Self([0; 32]);

	#[must_use]
	pub const fn new(bytes: [u8; 32]) -> Self {
		Self(bytes)
	}

	#[must_use]
	pub const fn bytes(self) -> [u8; 32] {
		self.0
	}
}

impl fmt::Debug for ProgramDigest {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("ProgramDigest(")?;
		for byte in &self.0[..6] {
			write!(formatter, "{byte:02x}")?;
		}
		formatter.write_str("...)")
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(u32);

impl BufferId {
	#[must_use]
	pub const fn new(value: u32) -> Self {
		Self(value)
	}

	#[must_use]
	pub const fn get(self) -> u32 {
		self.0
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageId(u32);

impl StageId {
	#[must_use]
	pub const fn new(value: u32) -> Self {
		Self(value)
	}

	#[must_use]
	pub const fn get(self) -> u32 {
		self.0
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferOrigin {
	Tensor(ValueId),
	Scratch {
		ordinal: u32,
		purpose: ScratchPurpose,
	},
	FaultFlag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchPurpose {
	ReductionValues,
	ReductionIndices,
	ScanValues,
	ScanBlockTotals,
	SortValues,
	SortIndices,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferLifetime {
	ExternalValue,
	ProgramScratch,
	ProgramFaultFlag,
}

/// Complete, immutable affine view into a typed buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticAccess {
	pub logical_extents: Vec<u64>,
	pub offset_elements: u64,
	pub strides: Vec<u64>,
	pub storage_bytes: ByteCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramBuffer {
	pub id: BufferId,
	pub origin: BufferOrigin,
	pub lifetime: BufferLifetime,
	pub dtype: DType,
	pub shape: Vec<u64>,
	pub access: StaticAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessMode {
	Read,
	Write,
	ReadWrite,
	ReadWriteAtomic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferBinding {
	pub buffer: BufferId,
	pub dtype: DType,
	pub mode: AccessMode,
	pub view: StaticAccess,
}

/// Immutable one-dimensional launch. Higher-rank tensor addressing remains in
/// each binding and stage contract, so launch geometry never depends on a
/// backend's grid-rank limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchGeometry {
	pub logical_lanes: u64,
	pub workgroup_lanes: u32,
	pub workgroups: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynchronizationScope {
	Workgroup,
	DispatchBoundary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemorySemantics {
	SharedAcquireRelease,
	GlobalAcquireRelease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SynchronizationPoint {
	pub after_step: u32,
	pub scope: SynchronizationScope,
	pub memory: MemorySemantics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicAddressDomain {
	SingleFaultFlag,
	TensorElements,
	HistogramBins,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnedAtomicOperation {
	Exchange,
	Add,
	Minimum,
	Maximum,
}

impl From<AtomicOperation> for OwnedAtomicOperation {
	fn from(value: AtomicOperation) -> Self {
		match value {
			AtomicOperation::Exchange => Self::Exchange,
			AtomicOperation::Add => Self::Add,
			AtomicOperation::Minimum => Self::Minimum,
			AtomicOperation::Maximum => Self::Maximum,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnedAtomicOrdering {
	Relaxed,
	Acquire,
	Release,
	AcquireRelease,
	SequentiallyConsistent,
}

impl From<AtomicOrdering> for OwnedAtomicOrdering {
	fn from(value: AtomicOrdering) -> Self {
		match value {
			AtomicOrdering::Relaxed => Self::Relaxed,
			AtomicOrdering::Acquire => Self::Acquire,
			AtomicOrdering::Release => Self::Release,
			AtomicOrdering::AcquireRelease => Self::AcquireRelease,
			AtomicOrdering::SequentiallyConsistent => Self::SequentiallyConsistent,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicContract {
	pub buffer: BufferId,
	pub dtype: DType,
	pub operation: OwnedAtomicOperation,
	pub ordering: OwnedAtomicOrdering,
	pub domain: AtomicAddressDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultReason {
	ArithmeticDomain,
	IndexOutOfBounds,
	HistogramBinOutOfBounds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaultContract {
	pub flag: BufferId,
	pub reason: FaultReason,
	pub code: i32,
	/// Rejected lanes must publish the fault before returning and may not form
	/// or issue the invalid payload address.
	pub guard_before_address: bool,
	pub publish: AtomicContract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreePhase {
	Reduction,
	ScanUpsweep,
	ScanDownsweep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeStep {
	pub phase: TreePhase,
	pub stride: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedTree {
	pub lanes: u32,
	pub steps: Vec<TreeStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReductionPadding {
	OperatorIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReductionTieBreak {
	LowestLogicalIndex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReductionStage {
	pub pass: u32,
	pub operator: ReduceOperator,
	pub result: ReduceResult,
	pub dtype: DType,
	pub sequences: u64,
	pub input_width: u64,
	pub output_width: u64,
	pub reduced_axes: Vec<usize>,
	pub tree: FixedTree,
	pub padding: ReductionPadding,
	pub tie_break: ReductionTieBreak,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanStageMode {
	UserInclusive,
	UserExclusive { identity: ScalarLiteral },
	HierarchyExclusiveIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanLocalStage {
	pub level: u32,
	pub operator: ReduceOperator,
	pub dtype: DType,
	pub sequences: u64,
	pub input_width: u64,
	pub output_width: u64,
	pub axis: usize,
	pub reverse: bool,
	pub mode: ScanStageMode,
	pub tree: FixedTree,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanUniformStage {
	pub level: u32,
	pub operator: ReduceOperator,
	pub dtype: DType,
	pub sequences: u64,
	pub width: u64,
	pub block_lanes: u32,
	pub reverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractionTile {
	pub output_x: u32,
	pub output_y: u32,
	pub reduction: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractionStage {
	pub spec: Contraction,
	pub dtype: DType,
	pub output_elements: u64,
	pub contracted_elements: u64,
	pub tile: ContractionTile,
	/// Contracted coordinates are visited in row-major axis-pair order. Each
	/// output accumulator therefore has one backend-independent operation
	/// order.
	pub canonical_contracted_order: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyReason {
	ScatterBase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistogramBinMapping {
	I32Direct,
	F32TruncateTowardZero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatSortOrder {
	Ieee754TotalOrder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortPadding {
	AfterAllValidElements,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortTieBreak {
	OriginalAxisIndexAscending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortNetwork {
	pub axis: usize,
	pub axis_length: u64,
	pub padded_axis_length: u64,
	pub slices: u64,
	pub direction: SortDirection,
	pub requested_stable: bool,
	pub tie_break: SortTieBreak,
	pub float_order: FloatSortOrder,
	pub padding: SortPadding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortCompareStage {
	pub network: SortNetwork,
	pub merge_width: u64,
	pub compare_distance: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhiloxCounterWord {
	ElementLow,
	ElementHigh,
	RunXorStreamLow,
	RunXorStreamHigh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformI32Mapping {
	UnbiasedMultiplyHighWithCounterRejection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalF32Mapping {
	OwnedBoxMullerV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Philox10Contract {
	pub key: RandomKey,
	pub distribution: RandomDistribution,
	pub rounds: u8,
	pub multiplier_0: u32,
	pub multiplier_1: u32,
	pub weyl_0: u32,
	pub weyl_1: u32,
	pub counter: [PhiloxCounterWord; 4],
	pub fold_kernel_id_into_key: bool,
	pub uniform_i32: UniformI32Mapping,
	pub normal_f32: NormalF32Mapping,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageKind {
	ScalarMap {
		template: KernelTemplate,
	},
	Fill {
		value: ScalarLiteral,
	},
	Copy {
		reason: CopyReason,
	},
	FixedTreeReduce(ReductionStage),
	FixedTreeScanLocal(ScanLocalStage),
	ScanUniformCombine(ScanUniformStage),
	TiledContraction(ContractionStage),
	Gather {
		axis: usize,
		bounds: IndexBounds,
	},
	Scatter {
		axis: usize,
		bounds: IndexBounds,
		conflict: ScatterConflict,
	},
	HistogramClear,
	HistogramAccumulate {
		bins: u32,
		weighted: bool,
		bin_mapping: HistogramBinMapping,
		ordering: OwnedAtomicOrdering,
	},
	StableSortInitialize {
		network: SortNetwork,
	},
	StableSortCompareExchange(SortCompareStage),
	StableSortFinalize {
		network: SortNetwork,
		emit_indices: bool,
	},
	Philox4x32_10(Philox10Contract),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageResourceBounds {
	pub flops: FlopCount,
	pub integer_operations: u64,
	pub atomic_operations: u64,
	pub shared_bytes_per_workgroup: ByteCount,
	pub private_bytes_per_lane: ByteCount,
	pub maximum_workgroup_lanes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramStage {
	pub id: StageId,
	pub dependencies: Vec<StageId>,
	pub geometry: DispatchGeometry,
	pub bindings: Vec<BufferBinding>,
	pub synchronization: Vec<SynchronizationPoint>,
	pub atomics: Vec<AtomicContract>,
	pub fault: Option<FaultContract>,
	pub resources: StageResourceBounds,
	pub kind: StageKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramResourceBounds {
	pub total_flops: FlopCount,
	pub total_integer_operations: u64,
	pub total_atomic_operations: u64,
	pub persistent_scratch_bytes: ByteCount,
	pub fault_bytes: ByteCount,
	pub peak_shared_bytes_per_workgroup: ByteCount,
	pub peak_private_bytes_per_lane: ByteCount,
	pub maximum_workgroup_lanes: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceAliasContract {
	pub input: usize,
	pub output: usize,
	pub permission: AliasPermission,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredProgram {
	pub schema_version: u32,
	pub source_kernel: KernelTemplateId,
	pub source_input_count: usize,
	pub source_output_count: usize,
	pub source_aliases: Vec<SourceAliasContract>,
	pub buffers: Vec<ProgramBuffer>,
	pub stages: Vec<ProgramStage>,
	pub resources: ProgramResourceBounds,
	pub digest: ProgramDigest,
}

impl LoweredProgram {
	/// Revalidate all structural, synchronization, resource, and digest
	/// invariants without consulting runtime state.
	pub fn validate(&self) -> Result<(), Vec<ProgramValidationError>> {
		crate::validate::validate(self)
	}

	/// Recompute the domain-separated canonical program digest.
	#[must_use]
	pub fn canonical_digest(&self) -> ProgramDigest {
		crate::hash::program_digest(self)
	}
}
