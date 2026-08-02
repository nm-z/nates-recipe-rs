//! Raw declarations for the subset of the public ROCr/HSA ABI used here.
//!
//! Constants and layouts were reviewed against `hsa.h` and `hsa_ext_amd.h`
//! from ROCm 7.2. The C enums used by these APIs have `int` representation;
//! fixed-width queue fields use their explicit header types.

use core::ffi::{c_char, c_void};

pub(crate) type HsaStatus = i32;
pub(crate) type HsaInfo = i32;

pub(crate) const STATUS_SUCCESS: HsaStatus = 0;
pub(crate) const STATUS_ERROR: HsaStatus = 0x1000;
pub(crate) const STATUS_ERROR_INVALID_ARGUMENT: HsaStatus = 0x1001;

pub(crate) const SYSTEM_INFO_VERSION_MAJOR: HsaInfo = 0;
pub(crate) const SYSTEM_INFO_VERSION_MINOR: HsaInfo = 1;
pub(crate) const SYSTEM_INFO_TIMESTAMP_FREQUENCY: HsaInfo = 3;
pub(crate) const AMD_SYSTEM_INFO_EXT_VERSION_MAJOR: HsaInfo = 0x207;
pub(crate) const AMD_SYSTEM_INFO_EXT_VERSION_MINOR: HsaInfo = 0x208;

pub(crate) const AGENT_INFO_NAME: HsaInfo = 0;
pub(crate) const AGENT_INFO_VENDOR_NAME: HsaInfo = 1;
pub(crate) const AGENT_INFO_FEATURE: HsaInfo = 2;
pub(crate) const AGENT_INFO_PROFILE: HsaInfo = 4;
pub(crate) const AGENT_INFO_WAVEFRONT_SIZE: HsaInfo = 6;
/// `HSA_AGENT_INFO_QUEUES_MAX` from the HSA runtime ABI.
pub(crate) const AGENT_INFO_QUEUES_MAX: HsaInfo = 12;
pub(crate) const AGENT_INFO_QUEUE_MIN_SIZE: HsaInfo = 13;
pub(crate) const AGENT_INFO_QUEUE_MAX_SIZE: HsaInfo = 14;
pub(crate) const AGENT_INFO_QUEUE_TYPE: HsaInfo = 15;
pub(crate) const AGENT_INFO_NODE: HsaInfo = 16;
pub(crate) const AGENT_INFO_DEVICE: HsaInfo = 17;
pub(crate) const AGENT_INFO_VERSION_MAJOR: HsaInfo = 21;
pub(crate) const AGENT_INFO_VERSION_MINOR: HsaInfo = 22;

pub(crate) const AMD_AGENT_INFO_CHIP_ID: HsaInfo = 0xA000;
pub(crate) const AMD_AGENT_INFO_CACHELINE_SIZE: HsaInfo = 0xA001;
pub(crate) const AMD_AGENT_INFO_COMPUTE_UNIT_COUNT: HsaInfo = 0xA002;
pub(crate) const AMD_AGENT_INFO_MAX_CLOCK_FREQUENCY: HsaInfo = 0xA003;
pub(crate) const AMD_AGENT_INFO_DRIVER_NODE_ID: HsaInfo = 0xA004;
pub(crate) const AMD_AGENT_INFO_BDFID: HsaInfo = 0xA006;
pub(crate) const AMD_AGENT_INFO_MEMORY_MAX_FREQUENCY: HsaInfo = 0xA008;
pub(crate) const AMD_AGENT_INFO_PRODUCT_NAME: HsaInfo = 0xA009;
pub(crate) const AMD_AGENT_INFO_MAX_WAVES_PER_CU: HsaInfo = 0xA00A;
pub(crate) const AMD_AGENT_INFO_NUM_SIMDS_PER_CU: HsaInfo = 0xA00B;
pub(crate) const AMD_AGENT_INFO_DOMAIN: HsaInfo = 0xA00F;
pub(crate) const AMD_AGENT_INFO_UUID: HsaInfo = 0xA011;
pub(crate) const AMD_AGENT_INFO_ASIC_REVISION: HsaInfo = 0xA012;
pub(crate) const AMD_AGENT_INFO_MEMORY_AVAIL: HsaInfo = 0xA015;
pub(crate) const AMD_AGENT_INFO_TIMESTAMP_FREQUENCY: HsaInfo = 0xA016;
pub(crate) const AMD_AGENT_INFO_NUM_SDMA_ENG: HsaInfo = 0xA10A;
pub(crate) const AMD_AGENT_INFO_NUM_SDMA_XGMI_ENG: HsaInfo = 0xA10B;
pub(crate) const AMD_AGENT_INFO_NUM_XCC: HsaInfo = 0xA111;
pub(crate) const AMD_AGENT_INFO_SCRATCH_LIMIT_MAX: HsaInfo = 0xA116;
pub(crate) const AMD_AGENT_INFO_SCRATCH_LIMIT_CURRENT: HsaInfo = 0xA117;

pub(crate) const AGENT_FEATURE_KERNEL_DISPATCH: u32 = 1;
pub(crate) const AGENT_FEATURE_AGENT_DISPATCH: u32 = 2;

pub(crate) const ISA_INFO_NAME_LENGTH: HsaInfo = 0;
pub(crate) const ISA_INFO_NAME: HsaInfo = 1;
pub(crate) const ISA_INFO_MACHINE_MODELS: HsaInfo = 5;
pub(crate) const ISA_INFO_PROFILES: HsaInfo = 6;
pub(crate) const ISA_INFO_DEFAULT_FLOAT_ROUNDING_MODES: HsaInfo = 7;
pub(crate) const ISA_INFO_BASE_PROFILE_DEFAULT_FLOAT_ROUNDING_MODES: HsaInfo = 8;
pub(crate) const ISA_INFO_FAST_F16_OPERATION: HsaInfo = 9;
pub(crate) const ISA_INFO_WORKGROUP_MAX_DIM: HsaInfo = 12;
pub(crate) const ISA_INFO_WORKGROUP_MAX_SIZE: HsaInfo = 13;
pub(crate) const ISA_INFO_GRID_MAX_DIM: HsaInfo = 14;
pub(crate) const ISA_INFO_GRID_MAX_SIZE: HsaInfo = 16;
pub(crate) const ISA_INFO_FBARRIER_MAX_SIZE: HsaInfo = 17;

pub(crate) const EXECUTABLE_SYMBOL_INFO_TYPE: HsaInfo = 0;
pub(crate) const EXECUTABLE_SYMBOL_INFO_KERNEL_OBJECT: HsaInfo = 22;
pub(crate) const EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_SIZE: HsaInfo = 11;
pub(crate) const EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_ALIGNMENT: HsaInfo = 12;
pub(crate) const EXECUTABLE_SYMBOL_INFO_KERNEL_GROUP_SEGMENT_SIZE: HsaInfo = 13;
pub(crate) const EXECUTABLE_SYMBOL_INFO_KERNEL_PRIVATE_SEGMENT_SIZE: HsaInfo = 14;
pub(crate) const EXECUTABLE_SYMBOL_INFO_KERNEL_DYNAMIC_CALLSTACK: HsaInfo = 15;

pub(crate) const AMD_MEMORY_POOL_INFO_SEGMENT: HsaInfo = 0;
pub(crate) const AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS: HsaInfo = 1;
pub(crate) const AMD_MEMORY_POOL_INFO_SIZE: HsaInfo = 2;
pub(crate) const AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALLOWED: HsaInfo = 5;
pub(crate) const AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_GRANULE: HsaInfo = 6;
pub(crate) const AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALIGNMENT: HsaInfo = 7;
pub(crate) const AMD_MEMORY_POOL_INFO_ACCESSIBLE_BY_ALL: HsaInfo = 15;
pub(crate) const AMD_MEMORY_POOL_INFO_ALLOC_MAX_SIZE: HsaInfo = 16;
pub(crate) const AMD_MEMORY_POOL_INFO_LOCATION: HsaInfo = 17;
pub(crate) const AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_REC_GRANULE: HsaInfo = 18;

pub(crate) const MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT: u32 = 1;
pub(crate) const MEMORY_POOL_GLOBAL_FLAG_FINE_GRAINED: u32 = 2;
pub(crate) const MEMORY_POOL_GLOBAL_FLAG_COARSE_GRAINED: u32 = 4;
pub(crate) const MEMORY_POOL_GLOBAL_FLAG_EXTENDED_FINE_GRAINED: u32 = 8;

pub(crate) const SYMBOL_KIND_KERNEL: i32 = 1;
pub(crate) const DEFAULT_FLOAT_ROUNDING_MODE_NEAR: i32 = 2;
pub(crate) const SIGNAL_CONDITION_LT: i32 = 2;
pub(crate) const WAIT_STATE_ACTIVE: i32 = 1;

pub(crate) const PACKET_TYPE_INVALID: u16 = 1;
pub(crate) const PACKET_TYPE_KERNEL_DISPATCH: u16 = 2;
pub(crate) const PACKET_TYPE_BARRIER_AND: u16 = 3;
pub(crate) const PACKET_HEADER_ACQUIRE_FENCE_SCOPE: u16 = 9;
pub(crate) const PACKET_HEADER_RELEASE_FENCE_SCOPE: u16 = 11;
pub(crate) const FENCE_SCOPE_SYSTEM: u16 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaAgent {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaIsa {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaSignal {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaMemoryPool {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaCodeObjectReader {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaExecutable {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaLoadedCodeObject {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HsaExecutableSymbol {
	pub(crate) handle: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HsaDim3 {
	pub(crate) x: u32,
	pub(crate) y: u32,
	pub(crate) z: u32,
}

/// Public large-model layout from `hsa_queue_t`.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct HsaQueue {
	pub(crate) kind: u32,
	pub(crate) features: u32,
	pub(crate) base_address: *mut c_void,
	pub(crate) doorbell_signal: HsaSignal,
	pub(crate) size: u32,
	pub(crate) reserved1: u32,
	pub(crate) id: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct HsaKernelDispatchPacket {
	pub(crate) header: u16,
	pub(crate) setup: u16,
	pub(crate) workgroup_size_x: u16,
	pub(crate) workgroup_size_y: u16,
	pub(crate) workgroup_size_z: u16,
	pub(crate) reserved0: u16,
	pub(crate) grid_size_x: u32,
	pub(crate) grid_size_y: u32,
	pub(crate) grid_size_z: u32,
	pub(crate) private_segment_size: u32,
	pub(crate) group_segment_size: u32,
	pub(crate) kernel_object: u64,
	pub(crate) kernarg_address: *mut c_void,
	pub(crate) reserved2: u64,
	pub(crate) completion_signal: HsaSignal,
}

/// Public large-model layout from `hsa_barrier_and_packet_t`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct HsaBarrierAndPacket {
	pub(crate) header: u16,
	pub(crate) reserved0: u16,
	pub(crate) reserved1: u32,
	pub(crate) dep_signal: [HsaSignal; 5],
	pub(crate) reserved2: u64,
	pub(crate) completion_signal: HsaSignal,
}

const _: [(); 64] = [(); core::mem::size_of::<HsaKernelDispatchPacket>()];
const _: [(); 64] = [(); core::mem::size_of::<HsaBarrierAndPacket>()];

pub(crate) type AgentCallback = unsafe extern "C" fn(agent: HsaAgent, data: *mut c_void) -> HsaStatus;
pub(crate) type IsaCallback = unsafe extern "C" fn(isa: HsaIsa, data: *mut c_void) -> HsaStatus;
pub(crate) type MemoryPoolCallback = unsafe extern "C" fn(pool: HsaMemoryPool, data: *mut c_void) -> HsaStatus;
pub(crate) type QueueErrorCallback = unsafe extern "C" fn(status: HsaStatus, source: *mut HsaQueue, data: *mut c_void);

pub(crate) type HsaInit = unsafe extern "C" fn() -> HsaStatus;
pub(crate) type HsaShutDown = unsafe extern "C" fn() -> HsaStatus;
pub(crate) type HsaStatusString = unsafe extern "C" fn(HsaStatus, *mut *const c_char) -> HsaStatus;
pub(crate) type HsaSystemGetInfo = unsafe extern "C" fn(attribute: HsaInfo, value: *mut c_void) -> HsaStatus;
pub(crate) type HsaIterateAgents = unsafe extern "C" fn(callback: AgentCallback, data: *mut c_void) -> HsaStatus;
pub(crate) type HsaAgentGetInfo =
	unsafe extern "C" fn(agent: HsaAgent, attribute: HsaInfo, value: *mut c_void) -> HsaStatus;
pub(crate) type HsaAgentIterateIsas =
	unsafe extern "C" fn(agent: HsaAgent, callback: IsaCallback, data: *mut c_void) -> HsaStatus;
pub(crate) type HsaIsaGetInfoAlt =
	unsafe extern "C" fn(isa: HsaIsa, attribute: HsaInfo, value: *mut c_void) -> HsaStatus;
pub(crate) type HsaQueueCreate = unsafe extern "C" fn(
	agent: HsaAgent,
	size: u32,
	kind: u32,
	callback: Option<QueueErrorCallback>,
	data: *mut c_void,
	private_segment_size: u32,
	group_segment_size: u32,
	queue: *mut *mut HsaQueue,
) -> HsaStatus;
pub(crate) type HsaQueueDestroy = unsafe extern "C" fn(queue: *mut HsaQueue) -> HsaStatus;
pub(crate) type HsaAmdAgentIterateMemoryPools =
	unsafe extern "C" fn(agent: HsaAgent, callback: MemoryPoolCallback, data: *mut c_void) -> HsaStatus;
pub(crate) type HsaAmdMemoryPoolGetInfo =
	unsafe extern "C" fn(pool: HsaMemoryPool, attribute: HsaInfo, value: *mut c_void) -> HsaStatus;
pub(crate) type HsaCodeObjectReaderCreateFromMemory =
	unsafe extern "C" fn(*const c_void, usize, *mut HsaCodeObjectReader) -> HsaStatus;
pub(crate) type HsaCodeObjectReaderDestroy = unsafe extern "C" fn(HsaCodeObjectReader) -> HsaStatus;
pub(crate) type HsaExecutableCreateAlt = unsafe extern "C" fn(i32, i32, *const c_char, *mut HsaExecutable) -> HsaStatus;
pub(crate) type HsaExecutableDestroy = unsafe extern "C" fn(HsaExecutable) -> HsaStatus;
pub(crate) type HsaExecutableLoadAgentCodeObject = unsafe extern "C" fn(
	HsaExecutable,
	HsaAgent,
	HsaCodeObjectReader,
	*const c_char,
	*mut HsaLoadedCodeObject,
) -> HsaStatus;
pub(crate) type HsaExecutableFreeze = unsafe extern "C" fn(HsaExecutable, *const c_char) -> HsaStatus;
pub(crate) type HsaExecutableGetSymbolByName =
	unsafe extern "C" fn(HsaExecutable, *const c_char, *const HsaAgent, *mut HsaExecutableSymbol) -> HsaStatus;
pub(crate) type HsaExecutableSymbolGetInfo =
	unsafe extern "C" fn(HsaExecutableSymbol, HsaInfo, *mut c_void) -> HsaStatus;
pub(crate) type HsaAmdMemoryPoolAllocate =
	unsafe extern "C" fn(HsaMemoryPool, usize, u32, *mut *mut c_void) -> HsaStatus;
pub(crate) type HsaAmdMemoryPoolFree = unsafe extern "C" fn(*mut c_void) -> HsaStatus;
pub(crate) type HsaAmdAgentsAllowAccess =
	unsafe extern "C" fn(u32, *const HsaAgent, *const u32, *const c_void) -> HsaStatus;
pub(crate) type HsaAmdMemoryAsyncCopy = unsafe extern "C" fn(
	*mut c_void,
	HsaAgent,
	*const c_void,
	HsaAgent,
	usize,
	u32,
	*const HsaSignal,
	HsaSignal,
) -> HsaStatus;
pub(crate) type HsaSignalCreate = unsafe extern "C" fn(i64, u32, *const HsaAgent, *mut HsaSignal) -> HsaStatus;
pub(crate) type HsaSignalDestroy = unsafe extern "C" fn(HsaSignal) -> HsaStatus;
pub(crate) type HsaSignalLoadScAcquire = unsafe extern "C" fn(HsaSignal) -> i64;
pub(crate) type HsaSignalStoreScRelease = unsafe extern "C" fn(HsaSignal, i64);
pub(crate) type HsaSignalWaitScAcquire = unsafe extern "C" fn(HsaSignal, i32, i64, u64, i32) -> i64;
pub(crate) type HsaQueueLoadReadIndexScAcquire = unsafe extern "C" fn(*const HsaQueue) -> u64;
pub(crate) type HsaQueueLoadWriteIndexRelaxed = unsafe extern "C" fn(*const HsaQueue) -> u64;
pub(crate) type HsaQueueStoreWriteIndexScRelease = unsafe extern "C" fn(*const HsaQueue, u64);
