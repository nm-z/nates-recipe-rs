# `hsa/src/abi.rs`

`abi` is the private, raw FFI boundary for the HSA implementation. It declares
the small ROCr and AMD extension surface that the crate actually uses. It does
not link to ROCr at compile time and it does not implement any policy. The
runtime loader in [`loader.rs`](../../src/loader.rs) resolves every function
pointer from a shared object and stores those pointers in `loader::Api`.
Discovery, session management, and execution then call the typed pointers
declared here.

The declarations were reviewed against the public `hsa.h` and
`hsa_ext_amd.h` headers from ROCm 7.2. The C enums used by this surface have
`int` representation, so enum and attribute values are represented as `i32`
unless the header explicitly uses another fixed-width type. The queue and AQL
packet declarations are the 64-bit large-model ABI. [`lib.rs`](../../src/lib.rs)
rejects non-64-bit targets before this module can be used.

## Boundary rules

* Every ABI type, constant, and function-pointer alias is `pub(crate)`. Raw
  handles and function pointers never cross the public crate API.
* C-facing structs use `#[repr(C)]`, and C callbacks and functions use
  `unsafe extern "C"` function-pointer types. Rust callers must establish the
  lifetime, pointer, alignment, initialization, and thread-safety guarantees
  required by the corresponding ROCr contract.
* `HsaStatus` is the status type returned by ROCr. `HsaInfo` is the integer
  attribute type accepted by the `*_get_info` functions. A non-success status
  means that an output pointer was not consumed as an initialized value.
* Opaque HSA objects are represented by one `u64` handle. The Rust structs are
  `Copy` values and therefore do not own an object. Ownership is provided by
  `Runtime`, `Session`, `QueueCore`, `AllocationInner`, `ExecutableInner`, and
  the signal pool, which retain the `Api` and perform the matching destroy
  operation.
* The two AQL packet structs have compile-time 64-byte size assertions. The
  queue writer relies on that exact slot size and on the first `u16` being the
  packet header.

The raw declarations are intentionally narrow. If an operation is not listed
below, this crate does not call it through this ABI module.

## Status and attribute constants

The numeric values are part of the foreign ABI, not configurable Recipe
values. The caller tables below name the concrete Rust operation that consumes
each value.

### Status values

| Constant | Value | Use |
| --- | ---: | --- |
| `STATUS_SUCCESS` | `0` | `loader::Api::check` and `check_status_only`; successful initialization and teardown; successful discovery callbacks; successful queue-destroy checks; identity flag exports. |
| `STATUS_ERROR` | `0x1000` | Returned by `collect_agent`, `collect_isa`, and `collect_pool` when callback data is null or a `Vec::push` panics. It tells ROCr to stop the synchronous traversal. |
| `STATUS_ERROR_INVALID_ARGUMENT` | `0x1001` | Recognized only by `discovery::pool_info_optional` as the supported way for a runtime to report an unavailable optional AMD pool attribute. |

`Api::check` turns every other status into `Error::Hsa` and asks
`hsa_status_string` for a diagnostic while the runtime is initialized.
`check_status_only` preserves the numeric status without allocating a message
for the realized submission path. Destructor-only signal destroys return a
status that cannot be observed from `Drop`. `Runtime::open` handles a failed
`hsa_init` directly because `hsa_status_string` is not safe before
initialization succeeds.

### System and agent information attributes

The generic HSA attributes are queried by `discovery::discover_agent` and
`discovery::discover`; the expected output type is the type passed to the
generic helper (`system_info` or `agent_info`). Fixed-size string attributes
are copied into arrays and decoded only after the terminating NUL is checked.

| Constant | Value | Output and caller |
| --- | ---: | --- |
| `SYSTEM_INFO_VERSION_MAJOR` | `0` | `u16`, `discover` -> `SystemDescription::hsa_version_major`. |
| `SYSTEM_INFO_VERSION_MINOR` | `1` | `u16`, `discover` -> `SystemDescription::hsa_version_minor`. |
| `SYSTEM_INFO_TIMESTAMP_FREQUENCY` | `3` | `u64`, `discover` and `DiscoveredAgent::into_session`; the latter supplies the signal wait tick frequency. |
| `AMD_SYSTEM_INFO_EXT_VERSION_MAJOR` | `0x207` | `u16`, `discover` -> AMD extension major version. |
| `AMD_SYSTEM_INFO_EXT_VERSION_MINOR` | `0x208` | `u16`, `discover` -> AMD extension minor version. |
| `AGENT_INFO_NAME` | `0` | `[u8; 64]`, `agent_string::<64>` -> `AgentIdentity::name`. |
| `AGENT_INFO_VENDOR_NAME` | `1` | `[u8; 64]`, `agent_string::<64>` -> `AgentIdentity::vendor_name`. |
| `AGENT_INFO_FEATURE` | `2` | `u32`, `AgentDescription::feature_bits`; bit tests use the feature constants below. |
| `AGENT_INFO_PROFILE` | `4` | `i32`, `Profile::from_raw` -> `AgentDescription::profile`. |
| `AGENT_INFO_WAVEFRONT_SIZE` | `6` | `u32`, only queried for a kernel-dispatch agent and stored as `first_isa_wavefront_size`. |
| `AGENT_INFO_QUEUES_MAX` | `12` | `u32`, `QueueCapabilities::maximum_queues`. |
| `AGENT_INFO_QUEUE_MIN_SIZE` | `13` | `u32`, `QueueCapabilities::minimum_packets`; validated as a nonzero power of two. |
| `AGENT_INFO_QUEUE_MAX_SIZE` | `14` | `u32`, `QueueCapabilities::maximum_packets`; validated as a nonzero power of two not below the minimum. |
| `AGENT_INFO_QUEUE_TYPE` | `15` | `u32`, `QueueKind::from_raw` -> advertised queue protocol. |
| `AGENT_INFO_NODE` | `16` | `u32`, `AgentIdentity::numa_node_id`. |
| `AGENT_INFO_DEVICE` | `17` | `i32`, `DeviceType::from_raw`; determines GPU-only AMD queries and session admission. |
| `AGENT_INFO_VERSION_MAJOR` | `21` | `u16`, `AgentDescription::hsa_version_major`. |
| `AGENT_INFO_VERSION_MINOR` | `22` | `u16`, `AgentDescription::hsa_version_minor`. |

AMD agent attributes are queried only for a `DeviceType::Gpu`, except for the
memory-availability query which is also exposed later by
`Session::available_memory_bytes`.

| Constant | Value | Output and caller |
| --- | ---: | --- |
| `AMD_AGENT_INFO_CHIP_ID` | `0xA000` | `u32`, `AmdGpuProperties::chip_id`. |
| `AMD_AGENT_INFO_CACHELINE_SIZE` | `0xA001` | `u32`, `cacheline_bytes`. |
| `AMD_AGENT_INFO_COMPUTE_UNIT_COUNT` | `0xA002` | `u32`, `compute_unit_count`. |
| `AMD_AGENT_INFO_MAX_CLOCK_FREQUENCY` | `0xA003` | `u32`, `maximum_clock_mhz`. |
| `AMD_AGENT_INFO_DRIVER_NODE_ID` | `0xA004` | `u32`, optional GPU `AgentIdentity::driver_node_id`. |
| `AMD_AGENT_INFO_BDFID` | `0xA006` | `u32`, combined with the domain by `PciAddress::from_hsa`. |
| `AMD_AGENT_INFO_MEMORY_MAX_FREQUENCY` | `0xA008` | `u32`, `maximum_memory_clock_mhz`. |
| `AMD_AGENT_INFO_PRODUCT_NAME` | `0xA009` | `[u8; 64]`, `agent_string::<64>` -> `product_name`. |
| `AMD_AGENT_INFO_MAX_WAVES_PER_CU` | `0xA00A` | `u32`, `maximum_waves_per_compute_unit`. |
| `AMD_AGENT_INFO_NUM_SIMDS_PER_CU` | `0xA00B` | `u32`, `simds_per_compute_unit`. |
| `AMD_AGENT_INFO_DOMAIN` | `0xA00F` | `u32`, PCI domain input to `PciAddress::from_hsa`. |
| `AMD_AGENT_INFO_UUID` | `0xA011` | `[u8; 21]`, parsed by `AgentUuid::from_str`; its device prefix must agree with `AGENT_INFO_DEVICE`. |
| `AMD_AGENT_INFO_ASIC_REVISION` | `0xA012` | `u32`, `asic_revision`. |
| `AMD_AGENT_INFO_MEMORY_AVAIL` | `0xA015` | `u64`, `available_memory_bytes` during discovery and on-demand in `Session::available_memory_bytes`. |
| `AMD_AGENT_INFO_TIMESTAMP_FREQUENCY` | `0xA016` | `u64`, GPU-specific `AmdGpuProperties::timestamp_frequency_hz`. |
| `AMD_AGENT_INFO_NUM_SDMA_ENG` | `0xA10A` | `u32`, `sdma_engine_count`. |
| `AMD_AGENT_INFO_NUM_SDMA_XGMI_ENG` | `0xA10B` | `u32`, `xgmi_sdma_engine_count`. |
| `AMD_AGENT_INFO_NUM_XCC` | `0xA111` | `u32`, `xcc_count`. |
| `AMD_AGENT_INFO_SCRATCH_LIMIT_MAX` | `0xA116` | `u64`, `maximum_scratch_bytes`. |
| `AMD_AGENT_INFO_SCRATCH_LIMIT_CURRENT` | `0xA117` | `u64`, `current_scratch_limit_bytes`. |

### Feature bits

| Constant | Value | Use |
| --- | ---: | --- |
| `AGENT_FEATURE_KERNEL_DISPATCH` | `1` (`1 << 0`) | `AgentDescription::supports_kernel_dispatch`, GPU session admission, first-ISA wavefront query, and returned-queue validation. |
| `AGENT_FEATURE_AGENT_DISPATCH` | `2` (`1 << 1`) | `AgentDescription::supports_agent_dispatch`; the raw bit is retained even when a newer runtime adds more bits. |

### ISA information attributes

`discover_isa` uses `HsaIsaGetInfoAlt` for every attribute in this table. A
GPU ISA name is parsed as an exact AMD target; non-GPU names remain raw text
and do not receive an AMD target.

| Constant | Value | Output and interpretation |
| --- | ---: | --- |
| `ISA_INFO_NAME_LENGTH` | `0` | `u32`; bounds the allocation used for the name and is rejected above 4096 bytes. |
| `ISA_INFO_NAME` | `1` | The reported byte buffer; either a header length excluding NUL or the current ROCr trailing-NUL form is accepted, but bytes after the first NUL must also be NUL. |
| `ISA_INFO_MACHINE_MODELS` | `5` | `[u8; 2]`; entries are validated as booleans for small and large machine model support. |
| `ISA_INFO_PROFILES` | `6` | `[u8; 2]`; entries are validated as booleans for base and full profiles. |
| `ISA_INFO_DEFAULT_FLOAT_ROUNDING_MODES` | `7` | `[u8; 3]`; converted to `RoundingModes`. |
| `ISA_INFO_BASE_PROFILE_DEFAULT_FLOAT_ROUNDING_MODES` | `8` | `[u8; 3]`; converted to base-profile `RoundingModes`. |
| `ISA_INFO_FAST_F16_OPERATION` | `9` | `u8`; converted to `IsaDescription::fast_f16`. |
| `ISA_INFO_WORKGROUP_MAX_DIM` | `12` | `[u16; 3]`; per-dimension workgroup limit. |
| `ISA_INFO_WORKGROUP_MAX_SIZE` | `13` | `u32`; total workgroup limit used by dispatch validation. |
| `ISA_INFO_GRID_MAX_DIM` | `14` | `HsaDim3`; copied into the three grid-dimension limits. |
| `ISA_INFO_GRID_MAX_SIZE` | `16` | `u64`; total grid limit used by dispatch validation. |
| `ISA_INFO_FBARRIER_MAX_SIZE` | `17` | `u32`; maximum workgroup fbarrier count. |

### Executable-symbol attributes

`Executable::kernel` looks up one symbol and queries every kernel attribute
needed to form `KernelMetadata`.

| Constant | Value | Output and caller |
| --- | ---: | --- |
| `EXECUTABLE_SYMBOL_INFO_TYPE` | `0` | `i32`; must equal `SYMBOL_KIND_KERNEL`, otherwise `Error::SymbolNotKernel`. |
| `EXECUTABLE_SYMBOL_INFO_KERNEL_OBJECT` | `22` | `u64`; zero is rejected as `Error::InvalidKernel`. |
| `EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_SIZE` | `11` | `u32`, required kernarg allocation size. |
| `EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_ALIGNMENT` | `12` | `u32`; must be nonzero and a power of two. |
| `EXECUTABLE_SYMBOL_INFO_KERNEL_GROUP_SEGMENT_SIZE` | `13` | `u32`, static group segment bytes. |
| `EXECUTABLE_SYMBOL_INFO_KERNEL_PRIVATE_SEGMENT_SIZE` | `14` | `u32`, static private segment bytes. |
| `EXECUTABLE_SYMBOL_INFO_KERNEL_DYNAMIC_CALLSTACK` | `15` | `u8`; validated as a C boolean and stored in `dynamic_callstack`. |

### AMD memory-pool attributes

`discover_memory_pool` uses the required attributes in the normal path and
`pool_info_optional` for extension attributes that older runtimes may reject
with `STATUS_ERROR_INVALID_ARGUMENT`.

| Constant | Value | Output and caller |
| --- | ---: | --- |
| `AMD_MEMORY_POOL_INFO_SEGMENT` | `0` | `i32`; `MemorySegment::from_raw` selects global, read-only, private, or group. |
| `AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS` | `1` | `u32`, queried only for global pools and retained as `MemoryPoolFlags`. |
| `AMD_MEMORY_POOL_INFO_SIZE` | `2` | `usize`, total pool bytes. |
| `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALLOWED` | `5` | `u8`; controls whether the three runtime allocation properties are queried. |
| `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_GRANULE` | `6` | `usize`, allocation granule. |
| `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALIGNMENT` | `7` | `usize`, allocation alignment. |
| `AMD_MEMORY_POOL_INFO_ACCESSIBLE_BY_ALL` | `15` | `u8`; converted to `accessible_by_all_agents`. |
| `AMD_MEMORY_POOL_INFO_ALLOC_MAX_SIZE` | `16` | Optional `usize`, maximum aggregate allocation. |
| `AMD_MEMORY_POOL_INFO_LOCATION` | `17` | Optional `i32`, converted to `MemoryLocation`; `None` records an unsupported extension attribute without guessing. |
| `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_REC_GRANULE` | `18` | `usize`, recommended allocation granule. |

The global-pool flag bits are exported through `identity::MemoryPoolFlags` and
are used to select allocation pools:

| Constant | Value | Meaning and caller |
| --- | ---: | --- |
| `MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT` | `1` | Kernarg initialization capability; `allocate_kernarg` and dispatch validation. |
| `MEMORY_POOL_GLOBAL_FLAG_FINE_GRAINED` | `2` | Fine-grained pool selection. |
| `MEMORY_POOL_GLOBAL_FLAG_COARSE_GRAINED` | `4` | Coarse-grained pool selection. |
| `MEMORY_POOL_GLOBAL_FLAG_EXTENDED_FINE_GRAINED` | `8` | Alternate fine-grained capability accepted by `allocate_fine`. |

### Enum-like execution values

| Constant | Value | Use |
| --- | ---: | --- |
| `SYMBOL_KIND_KERNEL` | `1` | Kernel symbol kind returned by `hsa_executable_symbol_get_info`. |
| `DEFAULT_FLOAT_ROUNDING_MODE_NEAR` | `2` | The nearest-even/default mode argument to `hsa_executable_create_alt`; the profile comes from discovery. |
| `SIGNAL_CONDITION_LT` | `2` | `hsa_signal_wait_scacquire` condition, paired with compare value `1` to wait for zero or a negative signal. |
| `WAIT_STATE_ACTIVE` | `1` | Active-wait hint passed to bounded signal waits. |

### AQL packet header values

| Constant | Value | Use |
| --- | ---: | --- |
| `PACKET_TYPE_INVALID` | `1` | Every packet body is initialized with this header. `enqueue_packets` asserts it before writing and publishes a valid header only after the body is complete. |
| `PACKET_TYPE_KERNEL_DISPATCH` | `2` | Low packet-type bits of a kernel dispatch header. |
| `PACKET_TYPE_BARRIER_AND` | `3` | Low packet-type bits of each dependency-reduction barrier header. |
| `PACKET_HEADER_ACQUIRE_FENCE_SCOPE` | `9` | Header bit offset used by `kernel_packet_header` and `barrier_and_packet_header`. |
| `PACKET_HEADER_RELEASE_FENCE_SCOPE` | `11` | Header bit offset used by both packet-header builders. |
| `FENCE_SCOPE_SYSTEM` | `2` | System-scope acquire and release fields, required for host/device visibility and completion-token ownership. |

## C-compatible handles and layouts

### Opaque handles

All of these structs are `#[repr(C)]` with one `pub(crate) handle: u64`
field. They are value representations only. The caller must not manufacture a
nonzero handle or call a destroy function twice.

| Type | Produced by | Retained and consumed by |
| --- | --- | --- |
| `HsaAgent` | `hsa_iterate_agents` callback | `Discovery`, `DiscoveredAgent`, `Session`, queue creation, memory operations, executable loading, and queue callbacks. |
| `HsaIsa` | `hsa_agent_iterate_isas` callback | `discover_isa` queries; it is not independently destroyed. |
| `HsaSignal` | `hsa_signal_create`, or an available signal reused by the signal pool | `SignalRecord`, completion/dependency tokens, barrier packets, and a queue's doorbell. Destroyed or rearmed only after terminal ownership is established. |
| `HsaMemoryPool` | `hsa_amd_agent_iterate_memory_pools` callback | `RawPool`, pool discovery, and `hsa_amd_memory_pool_allocate`; the runtime owns the pool object. |
| `HsaCodeObjectReader` | `hsa_code_object_reader_create_from_memory` | `ExecutableInner::reader`; destroyed after its executable. |
| `HsaExecutable` | `hsa_executable_create_alt` | `ExecutableInner::executable`; loaded, frozen, symbol-queried, then destroyed before the reader. |
| `HsaLoadedCodeObject` | Output of `hsa_executable_load_agent_code_object` | A `MaybeUninit` output in `Session::load_hsaco`; the loaded handle is intentionally not used or separately destroyed because executable destruction owns that relationship. |
| `HsaExecutableSymbol` | `hsa_executable_get_symbol_by_name` | Temporary value in `Executable::kernel`, queried immediately and not destroyed separately. |

### `HsaDim3`

`HsaDim3` is a `#[repr(C)]`, three-`u32` value with fields `x`, `y`, and `z`.
`hsa_isa_get_info_alt(ISA_INFO_GRID_MAX_DIM)` writes it. Discovery copies the
fields into `[u32; 3]` in `IsaDescription`; it is not passed back to ROCr.

### `HsaQueue`

`HsaQueue` is the public large-model `hsa_queue_t` prefix used by this crate:

| Field | Type | Meaning and caller |
| --- | --- | --- |
| `kind` | `u32` | Actual queue protocol reported by ROCr. `Session::create_queue` converts it with `QueueKind::from_raw` and rejects unknown values. |
| `features` | `u32` | Runtime feature bits. Queue creation requires `AGENT_FEATURE_KERNEL_DISPATCH`. |
| `base_address` | `*mut c_void` | Start of the 64-byte AQL ring. It must be non-null before `HsaQueueIo` can calculate a slot. |
| `doorbell_signal` | `HsaSignal` | Signal written by `HsaQueueIo::ring_doorbell_screlease` after publishing a packet. |
| `size` | `u32` | Ring capacity. Creation validates nonzero and power-of-two; occupancy and slot masking use it. |
| `reserved1` | `u32` | ABI padding/reserved field; never modified. |
| `id` | `u64` | Read by `Queue::id` and by `queue_error_callback` when ROCr supplies a non-null source queue. |

`Session::create_queue` treats a successful status with a null pointer as
`Error::NullQueue`. It also validates `base_address`, `size`, `kind`, and
`features` before constructing `QueueCore`. On an invalid result it calls
`hsa_queue_destroy`; callback context is freed only when that destroy succeeds.
Unlike the packet types, `HsaQueue` has no local size assertion because callers
only read the reviewed prefix fields, but its field order and pointer width are
the public large-model layout.
The returned `kind` can report the runtime's multi-producer protocol even when
the request was single-producer; `QueueCore` retains the requested producer
discipline and only exposes the safe single-producer publication path.

### AQL packet layouts

`HsaKernelDispatchPacket` is the 64-byte `hsa_kernel_dispatch_packet_t`
layout. `header` starts as `PACKET_TYPE_INVALID` and is release-published as
the final header. `setup` stores the dispatch dimensionality and the barrier
bit is encoded by `kernel_packet_header`. The three `workgroup_size_*` fields
are `u16`; the three `grid_size_*` fields are `u32`. Static and dynamic private
and group segment bytes are carried in the two `u32` segment fields. The
`u64 kernel_object` comes from executable symbol metadata, `kernarg_address`
is either null or a validated aligned allocation pointer, `reserved2` is zero,
and `completion_signal` is the token's `HsaSignal`.

`HsaBarrierAndPacket` is the 64-byte `hsa_barrier_and_packet_t` layout.
`header` also starts invalid and is later published as a barrier header.
`dep_signal` contains five `HsaSignal` values. `lower_barrier_packets` fills
unused entries with the zero handle, stores a completion signal in each packet,
and builds a deterministic reduction tree when more than five dependencies are
present. The two reserved fields are zero.

The compile-time assertions
`[(); 64] = [(); size_of::<HsaKernelDispatchPacket>()]` and the corresponding
barrier assertion make a layout drift a compile error. `HsaQueueIo::slot`
therefore masks an index by `size - 1`, casts the ring to `[u8; 64]`, writes one
packet body, release-stores the header, release-stores the write index, and
release-stores the doorbell signal. No consumer can observe a packet before
its body is fully written.

## Callback contracts

| Alias | C signature | Implementation and invocation |
| --- | --- | --- |
| `AgentCallback` | `unsafe extern "C" fn(HsaAgent, *mut c_void) -> HsaStatus` | `discovery::collect_agent` receives the agent handle and a pointer to `AgentCollector`; `iterate_agents` supplies the pointer for the synchronous traversal. |
| `IsaCallback` | `unsafe extern "C" fn(HsaIsa, *mut c_void) -> HsaStatus` | `discovery::collect_isa` appends an ISA to `IsaCollector`; invoked by `iterate_isas`. |
| `MemoryPoolCallback` | `unsafe extern "C" fn(HsaMemoryPool, *mut c_void) -> HsaStatus` | `discovery::collect_pool` appends a pool to `PoolCollector`; invoked by `iterate_memory_pools`. |
| `QueueErrorCallback` | `unsafe extern "C" fn(HsaStatus, *mut HsaQueue, *mut c_void)` | `session::queue_error_callback` records status and, when supplied, the source queue ID in `SharedFault`; passed to `hsa_queue_create`. |

The three discovery callbacks reject null data, cast only the pointer supplied
by their own synchronous wrapper, and wrap `Vec::push` in
`catch_unwind(AssertUnwindSafe(...))`. A panic never unwinds through C. The
wrapper checks the `panicked` flag before interpreting the returned status and
reports `Error::CallbackPanicked` when needed. The callback data is valid only
until the foreign iterator returns.

The queue callback has a different lifetime. `Session::create_queue` places a
`QueueCallbackContext` in a `Box` and passes its stable address to ROCr.
`QueueCore` retains the box until `hsa_queue_destroy` returns. If destruction
fails, the context is leaked because ROCr may still invoke the callback. The
callback performs only atomic stores and a condition-variable notification,
with no allocation and no lock acquisition.

## Function-pointer aliases and callers

Every alias in this section is loaded eagerly by `Api::load`. The symbol name
shown is the literal name passed to `dlsym`. A non-null address and no
`dlerror` diagnostic are required before `Api` is constructed. All calls are
made through the matching alias, so the argument and return layout is checked
at the one `transmute` in the loader.

### Library lifetime and symbol loading

`loader::Library::open` converts the requested `OsStr` to a NUL-terminated
`CString`, calls `dlopen(..., RTLD_NOW | RTLD_LOCAL)`, and stores a checked
`NonNull<c_void>` handle. Interior NULs become `Error::PathContainsNul`; a null
handle becomes `Error::LibraryOpen` with a copied `dlerror` diagnostic.

`Library::symbol` receives a static ASCII name and static NUL-terminated byte
slice. It clears the thread-local loader diagnostic, calls `dlsym`, copies any
diagnostic before another loader call, and rejects both an error diagnostic and
a null function address as `Error::MissingSymbol`.

`Api::load` resolves all required symbols below in order. A failure drops the
partially built `Library`, so no function pointer escapes. On success, the
library is stored in the first field of `Api`; all function pointers are
therefore dropped before the handle is closed. `Library` has explicit `Send`
and `Sync` implementations because POSIX permits immutable lookup and calls
through one handle from multiple threads. `Runtime` stores `Arc<Api>`, which
keeps the handle alive for every queue, allocation, executable, and signal
record.

`Runtime::open_default` supplies the normal soname search policy around this
loader: it tries `libhsa-runtime64.so.1` and then `libhsa-runtime64.so`,
continuing only after a `LibraryOpen` failure. A missing required symbol,
failed initialization, or any other typed error stops the search rather than
silently selecting a different ABI.

### Runtime and information functions

| Alias and symbol | ABI inputs and output | Callers, postconditions, and failures |
| --- | --- | --- |
| `HsaInit` (`hsa_init`) | No arguments; returns `HsaStatus`. | `Runtime::open` calls it once after `Api::load`. Non-success is `Error::Hsa { operation: "hsa_init", message: None }`; no status-string query is attempted before initialization. |
| `HsaShutDown` (`hsa_shut_down`) | No arguments; returns `HsaStatus`. | `Runtime::close` and `Drop` call the one balanced shutdown. `close` reports a status through `Api::check`; `Drop` intentionally discards the status. Borrowing and `active` prevent shutdown while child objects are live. |
| `HsaStatusString` (`hsa_status_string`) | `HsaStatus`, `*mut *const c_char`; returns a status and writes runtime-owned NUL-terminated text. | `Api::status_message`, reached by `Api::check` after initialization. Null output or a non-success conversion returns `None`, leaving the numeric `Error::Hsa` status intact. |
| `HsaSystemGetInfo` (`hsa_system_get_info`) | `HsaInfo`, writable `*mut c_void`; returns `HsaStatus`. | `discovery::system_info` pairs each attribute with its exact `MaybeUninit<T>`, and `DiscoveredAgent::into_session` queries the timestamp frequency again. A failed query is annotated with the field-specific operation and the output is never read. |
| `HsaIterateAgents` (`hsa_iterate_agents`) | `AgentCallback`, opaque `*mut c_void`; returns `HsaStatus`. | `discovery::iterate_agents` supplies a live `AgentCollector`, checks callback panic state, then checks the status. The collected handles become `DiscoveredAgent::raw_agent` values. |
| `HsaAgentGetInfo` (`hsa_agent_get_info`) | `HsaAgent`, `HsaInfo`, writable `*mut c_void`; returns `HsaStatus`. | `discovery::agent_info` reads all generic and AMD agent attributes listed above; `Session::available_memory_bytes` reads `AMD_AGENT_INFO_MEMORY_AVAIL` on demand. The generic helper assumes initialization only after `Api::check` succeeds. |
| `HsaAgentIterateIsas` (`hsa_agent_iterate_isas`) | `HsaAgent`, `IsaCallback`, opaque data pointer; returns `HsaStatus`. | `discovery::iterate_isas` owns `IsaCollector` for the synchronous call and maps callback panic or status failure to `Error`. |
| `HsaIsaGetInfoAlt` (`hsa_isa_get_info_alt`) | `HsaIsa`, `HsaInfo`, writable `*mut c_void`; returns `HsaStatus`. | `discovery::discover_isa` first obtains a bounded name length, then the name bytes and all ISA capability values. Name buffers are writable for exactly the reported length; `MaybeUninit<T>` values are assumed initialized only after success. |
| `HsaAmdAgentIterateMemoryPools` (`hsa_amd_agent_iterate_memory_pools`) | `HsaAgent`, `MemoryPoolCallback`, opaque data pointer; returns `HsaStatus`. | `discovery::iterate_memory_pools` collects the exact pool handles for a `DiscoveredAgent`; the same synchronous callback and panic rules apply. |
| `HsaAmdMemoryPoolGetInfo` (`hsa_amd_memory_pool_get_info`) | `HsaMemoryPool`, `HsaInfo`, writable `*mut c_void`; returns `HsaStatus`. | `discovery::pool_info` handles required attributes. `pool_info_optional` maps only `STATUS_ERROR_INVALID_ARGUMENT` to `None`, checks every other status, and never reads an uninitialized optional output. |

### Queue creation and destruction

| Alias and symbol | ABI inputs and output | Caller and invariants |
| --- | --- | --- |
| `HsaQueueCreate` (`hsa_queue_create`) | Agent, packet count `u32`, raw queue kind `u32`, optional `QueueErrorCallback`, callback data, private and group segment limits, and `*mut *mut HsaQueue` output; returns `HsaStatus`. | `Session::create_queue` validates discovered limits, kind compatibility, and kernel-dispatch support before calling. `QueueConfig::new` supplies `u32::MAX` for both segment hints, which asks ROCr to choose its normal limit. It checks status, non-null output, ring address, power-of-two size, returned kind, and feature bits. Callback data points to a boxed context retained by `QueueCore`. |
| `HsaQueueDestroy` (`hsa_queue_destroy`) | `*mut HsaQueue`; returns `HsaStatus`. | Used by the invalid-return cleanup in `Session::create_queue`, by `QueueCore::destroy`, and by `Queue::close`. A failed destroy causes callback-context leakage rather than a use-after-free and is returned as `Error::Hsa` for explicit close. |

### Code-object and executable functions

| Alias and symbol | ABI inputs and output | Caller and failures |
| --- | --- | --- |
| `HsaCodeObjectReaderCreateFromMemory` (`hsa_code_object_reader_create_from_memory`) | Non-null code-object bytes, `usize` length, `*mut HsaCodeObjectReader` output; returns `HsaStatus`. | `Session::load_hsaco` rejects an empty slice, copies it into an `Arc<[u8]>`, and retains that Arc in `ExecutableInner` for the reader lifetime. Creation failure is checked and no handle is assumed. |
| `HsaCodeObjectReaderDestroy` (`hsa_code_object_reader_destroy`) | `HsaCodeObjectReader`; returns `HsaStatus`. | `ExecutableInner::destroy` calls it only after the executable is destroyed. A failure keeps the backing code-object Arc alive by forgetting it, because the reader may still refer to the bytes. |
| `HsaExecutableCreateAlt` (`hsa_executable_create_alt`) | Profile `i32`, float-rounding mode `i32`, nullable options string, `*mut HsaExecutable` output; returns `HsaStatus`. | `Session::load_hsaco` passes the discovered profile, `DEFAULT_FLOAT_ROUNDING_MODE_NEAR`, and null options. If creation fails, the reader is destroyed through `ExecutableInner::destroy`. |
| `HsaExecutableDestroy` (`hsa_executable_destroy`) | `HsaExecutable`; returns `HsaStatus`. | `ExecutableInner::destroy` calls it before reader destruction. A failed destroy leaks the reader and its backing bytes and returns `Error::Hsa`. |
| `HsaExecutableLoadAgentCodeObject` (`hsa_executable_load_agent_code_object`) | Executable, agent, code-object reader, nullable options string, `*mut HsaLoadedCodeObject` output; returns `HsaStatus`. | `Session::load_hsaco` passes mutually live objects and a `MaybeUninit<HsaLoadedCodeObject>`. The output is intentionally not retained; executable destruction owns the loaded object. Failure destroys the partially built executable and reader. |
| `HsaExecutableFreeze` (`hsa_executable_freeze`) | Executable and nullable options string; returns `HsaStatus`. | `Session::load_hsaco` freezes after loading and before returning `Executable`. Failure destroys all partial resources. A final session-health check prevents returning an executable after an asynchronous queue fault. |
| `HsaExecutableGetSymbolByName` (`hsa_executable_get_symbol_by_name`) | Executable, NUL-terminated symbol name, optional agent pointer, `*mut HsaExecutableSymbol` output; returns `HsaStatus`. | `Executable::kernel` converts the Rust name to `CString`, rejects interior NULs, supplies the executable's agent pointer, and assumes the output only after success. Lookup errors become `Error::Hsa`. |
| `HsaExecutableSymbolGetInfo` (`hsa_executable_symbol_get_info`) | Symbol, `HsaInfo`, writable output pointer; returns `HsaStatus`. | `executable_symbol_info` pairs each metadata attribute with its exact output type. Kernel kind, object, alignment, and C-boolean checks map malformed successful responses to `SymbolNotKernel` or `InvalidKernel`. |

### AMD memory operations

| Alias and symbol | ABI inputs and output | Caller and invariants |
| --- | --- | --- |
| `HsaAmdMemoryPoolAllocate` (`hsa_amd_memory_pool_allocate`) | Pool handle, byte count `usize`, allocation flags `u32`, `*mut *mut c_void` output; returns `HsaStatus`. | `allocate_from` accepts only a discovered runtime-allocatable pool, rejects zero or over-limit sizes, passes flags zero, checks status, and rejects a null successful pointer as `Error::NullAllocation`. `AllocationInner` owns the resulting pointer. |
| `HsaAmdMemoryPoolFree` (`hsa_amd_memory_pool_free`) | Allocation pointer; returns `HsaStatus`. | `AllocationInner::destroy` performs exactly one matching free and reports status. `Drop` calls it while ignoring the result because it cannot return an error; explicit `Allocation::close` reports it when it has unique ownership. |
| `HsaAmdAgentsAllowAccess` (`hsa_amd_agents_allow_access`) | Agent count `u32`, pointer to discovered agents, nullable reserved flags pointer, allocation pointer; returns `HsaStatus`. | `grant_access` rejects an empty or overflowing set, passes null reserved flags, and requires handles from the same runtime. ROCr replaces the set, so the exact handles are recorded only after success for later copy validation. |
| `HsaAmdMemoryAsyncCopy` (`hsa_amd_memory_async_copy`) | Destination pointer and agent, source pointer and agent, byte count `usize`, dependency count `u32`, dependency signal pointer, completion `HsaSignal`; returns `HsaStatus`. | `Session::copy_async` and `copy_async_prepared` validate nonzero size, bounds, mutual access, live allocations, and an initialized completion signal. They pass no dependency array, retain both allocations until terminal completion, and use `check` or allocation-free `check_status_only` according to the path. |

### Signals and queue indices

| Alias and symbol | ABI inputs and output | Caller and invariants |
| --- | --- | --- |
| `HsaSignalCreate` (`hsa_signal_create`) | Initial `i64`, consumer count `u32`, optional consumer-agent array, `*mut HsaSignal` output; returns `HsaStatus`. | `SignalPool::acquire` creates value `1`, zero consumers, and a null agent list. A successful call must return a nonzero handle. Recycled signals are restored to one by `HsaSignalStoreScRelease`. |
| `HsaSignalDestroy` (`hsa_signal_destroy`) | `HsaSignal`; returns `HsaStatus`. | `SignalRecord::drop` destroys negative or otherwise terminal signals; `SignalPoolInner::drop` destroys available signals. The destructor paths intentionally ignore this return because they cannot report an error. Positive signals are never destroyed outside deferred retirement. |
| `HsaSignalLoadScAcquire` (`hsa_signal_load_scacquire`) | `HsaSignal`; returns current `i64` value. | Signal polling, retirement collection, bounded waits, and `SignalRecord::drop` use acquire loads. Positive means pending, zero means complete, and negative means asynchronous failure. |
| `HsaSignalStoreScRelease` (`hsa_signal_store_screlease`) | `HsaSignal`, new `i64`; returns nothing. | Signal rearming writes `1` only after terminal completion. Queue doorbells write the packet index with release ordering. The signal record must be the sole submission owner when rearmed. |
| `HsaSignalWaitScAcquire` (`hsa_signal_wait_scacquire`) | Signal, condition `i32`, compare `i64`, timeout ticks `u64`, wait-state hint `i32`; returns observed `i64`. | `SignalPool::wait_one_retired` and `Pending::wait` pass `SIGNAL_CONDITION_LT`, compare `1`, bounded ticks, and `WAIT_STATE_ACTIVE`. `duration_to_ticks` converts the timestamp frequency with saturating arithmetic and clamps to at least one tick; `Pending::wait` requests no more than one millisecond per call. The call is a hint only; the surrounding loop rechecks the signal and host deadline. |
| `HsaQueueLoadReadIndexScAcquire` (`hsa_queue_load_read_index_scacquire`) | `*const HsaQueue`; returns `u64`. | `HsaQueueIo::load_read_index_scacquire` obtains consumer progress for occupancy and queue-full checks. The queue core owns a live readable pointer. |
| `HsaQueueLoadWriteIndexRelaxed` (`hsa_queue_load_write_index_relaxed`) | `*const HsaQueue`; returns `u64`. | `HsaQueueIo::load_write_index_relaxed` obtains the producer position for packet placement. It is used only by the host-confined queue writer. |
| `HsaQueueStoreWriteIndexScRelease` (`hsa_queue_store_write_index_screlease`) | `*const HsaQueue`, next `u64`; returns nothing. | `HsaQueueIo::publish_write_index_screlease` publishes each packet after its body and header release store. The queue API is exposed only to the single-producer publication path. |

## Caller trace by module

The following is the complete in-repository call trace for the ABI module.
Examples call only public `Runtime`, `Discovery`, `Session`, `Queue`,
`Allocation`, `Executable`, and token methods; they do not access `abi`
directly.

### `loader.rs`

`loader::Api` imports every function-pointer alias in this file, resolves the
matching symbol, and stores it as a field. It is the only location that turns
a `*mut c_void` from `dlsym` into a typed function pointer. `Library` keeps the
shared object open for the entire `Api` lifetime. `Api::check` and
`check_status_only` are the common status boundary for all callers except the
special pre-initialization `hsa_init` path and status-free signal destruction.

### `runtime.rs`

`Runtime::open` calls `HsaInit`; `Runtime::close` and `Drop` call
`HsaShutDown`. `Runtime::discover` delegates to `discovery::discover`, and
`ensure_active` prevents any child operation from using the ABI after explicit
close. The `Arc<Api>` held by `Runtime` is cloned into sessions, queues,
allocations, executables, and signals.

### `discovery.rs`

`discover` calls `HsaSystemGetInfo` for the system versions and timestamp,
then `HsaIterateAgents`. For each agent, `discover_agent` calls
`HsaAgentGetInfo` for generic identity, feature, profile, queue, version, and
AMD GPU attributes; `HsaAgentIterateIsas` followed by `HsaIsaGetInfoAlt` for
each ISA; and `HsaAmdAgentIterateMemoryPools` followed by
`HsaAmdMemoryPoolGetInfo` for each pool. `pool_info_optional` is the sole
caller that intentionally accepts `STATUS_ERROR_INVALID_ARGUMENT`.

`system_info`, `agent_info`, `isa_info`, and `pool_info` all use
`MaybeUninit<T>`, cast only the writable storage to `c_void`, check the status,
and then assume initialization. Their call sites determine `T`, so an
attribute/value-size mismatch is a source-level ABI review error rather than
a runtime conversion. `agent_string` validates a NUL and UTF-8; ISA-name
allocation is bounded and accepts the two observed ROCr length conventions.

The three iterator wrappers keep callback data live for the complete foreign
call and detect both callback panic and returned status. No callback may
retain the opaque data pointer after the wrapper returns.

### `identity.rs`

`MemoryPoolFlags` re-exports the four memory-pool flag constants as public bit
values. `DeviceType`, `Profile`, `QueueKind`, `MemorySegment`, and
`MemoryLocation` decode the raw `i32` or `u32` values returned by the ABI and
return `Error::InvalidAttribute` for unknown values. `PciAddress::from_hsa`
decodes the raw AMD domain and BDF ID. No function pointer is called here.

### `session.rs`

`DiscoveredAgent::into_session` calls `HsaSystemGetInfo` for the timestamp
frequency used by signal waits. `Session::available_memory_bytes` calls
`HsaAgentGetInfo` for the live AMD memory counter.

`Session::create_queue` validates discovered queue capabilities, calls
`HsaQueueCreate`, inspects `HsaQueue`, and either constructs `QueueCore` or
destroys and rejects the returned queue. `QueueCore::destroy`, `Queue::close`,
and the invalid-return cleanup call `HsaQueueDestroy`. The queue-error callback
reads the optional source queue's `id` and permanently poisons the shared
session fault state.

### `execution.rs`

Signal pooling calls `HsaSignalCreate`, `HsaSignalLoadScAcquire`,
`HsaSignalStoreScRelease`, `HsaSignalWaitScAcquire`, and `HsaSignalDestroy`.
`Pending`, `Dependency`, retirement collection, and prepared-token reset all
use the same signal ownership rules. `SignalRecord` never destroys a positive
signal; unresolved records stay in the explicit retirement set with their
queue, executable, allocation, and dependency keepalives.

When the signal pool itself is dropped, available terminal signals are
destroyed. Retired signals are loaded once: terminal records release their
keepalives and are reclaimed, while positive records are forgotten together
with their keepalives and a root log entry reports the unresolved operation
count. `Session::drain_retirements` is therefore the explicit teardown path
when results or resource reclamation must be observed.

Allocation creation and teardown call `HsaAmdMemoryPoolAllocate` and
`HsaAmdMemoryPoolFree`; exact access replacement calls
`HsaAmdAgentsAllowAccess`. Both `grant_access` entry points pass discovered
handles, and `Session::grant_access_exact_set` rejects allocations, sessions,
or discovered agents from another `Runtime` before it reaches the ABI call.
Asynchronous copies call
`HsaAmdMemoryAsyncCopy`. The keepalive vectors ensure foreign operations cannot
observe a freed allocation.

`Session::load_hsaco` calls the code-object reader and executable functions in
the required order: create reader, create executable, load code object for the
session agent, freeze, then expose `Executable`. `Executable::kernel` resolves
the symbol and queries all executable-symbol attributes. Destruction reverses
the dependency order, executable before reader, and preserves backing bytes if
either destroy operation fails.

Queue dispatch constructs `HsaKernelDispatchPacket` and, when dependencies are
present, `HsaBarrierAndPacket`. `HsaQueueIo` reads both queue indices, writes
the 64-byte ring slot, release-publishes the header and write index, and rings
the doorbell with `HsaSignalStoreScRelease`. It uses the packet and fence
constants above. Geometry, queue capacity, signal ownership, kernarg pool,
pointer alignment, and agent identity are validated before any packet is
published.

## Safety and failure invariants

The ABI declarations themselves cannot enforce the following conditions; each
condition is established by the named caller before entering `unsafe` code.

1. **Library and pointer lifetime.** `Api` owns `Library`, and every owner of a
   function pointer holds an `Arc<Api>`. `Library::drop` runs only after all
   pointers and foreign objects are gone.
2. **Initialization.** Except for `Api::load`, all status conversion and HSA
   object operations occur after successful `HsaInit`. `Runtime::close` marks
   the object inactive before calling shutdown and borrows prevent live child
   references.
3. **Output initialization.** Information and handle outputs use
   `MaybeUninit`; successful status is checked before `assume_init`. Fixed-size
   strings additionally require a NUL and valid UTF-8. Dynamic ISA names are
   bounded before allocation.
4. **Callback boundaries.** Iterator callbacks are synchronous, carry a live
   collector pointer, and cannot unwind through C. Queue callback data remains
   boxed until queue destruction succeeds; a failed destroy leaks that context
   instead of permitting a callback use-after-free.
5. **Handle ownership.** Opaque `Copy` handles do not imply ownership.
   `QueueCore`, `AllocationInner`, `ExecutableInner`, and `SignalRecord` each
   perform at most one matching destroy. Reader destruction follows executable
   destruction, and loaded code objects are owned by the executable.
6. **Signal terminal state.** A positive signal is device-pending and remains
   in deferred retirement. Zero is complete and can be recycled only after all
   keepalives are released. A negative value is terminal failure and poisons
   the session or token. Waits are bounded and always followed by a fresh load.
7. **Memory and asynchronous copies.** Allocation sizes and copy ranges are
   checked with checked arithmetic. Both endpoint agents must have direct
   access to both allocations. Keepalives retain allocations until the
   completion signal is terminal. `Allocation::copy_from_host` and
   `copy_to_host` are direct host pointer copies rather than HSA ABI calls; the
   caller must additionally prove host accessibility, coherency, completion of
   prior device work, and non-overlap before invoking those `unsafe` methods.
8. **Queue publication.** Queue creation proves a non-null, power-of-two ring
   and kernel-dispatch capability. The single-producer path writes a complete
   invalid-header body before release-publishing a valid header, write index,
   and doorbell. Queue occupancy is checked before every enqueue.
9. **Executable metadata.** Kernel names are NUL-terminated; the returned
   symbol must be a kernel with a nonzero object and power-of-two alignment.
   Kernarg pointers are from a discovered kernarg-capable pool, accessible to
   the queue agent, large enough, and aligned.
10. **Status propagation.** Foreign failures become typed `Error` values with
    the operation and numeric status. Optional pool attributes are the only
    deliberate status-to-`None` conversion. Destructor paths that cannot return
    an error either retain/leak the foreign resource safely or report through
    an explicit `close` method.

### Header-to-call-site review checklist

The declarations and callers form one reviewed chain rather than independent
FFI fragments:

| Header contract | Local proof before the unsafe call |
| --- | --- |
| C enum or fixed-width scalar | The alias uses `i32`, `u32`, `u16`, `u64`, `i64`, `usize`, or `u8` according to the attribute or large-model header type. Unknown enum values are decoded as `InvalidAttribute`, not silently reinterpreted. |
| `void *` information output | `system_info`, `agent_info`, `isa_info`, `pool_info`, and `executable_symbol_info` allocate `MaybeUninit<T>` of the reviewed size, pass a writable cast, check status, and only then assume initialization. |
| C string input/output | `Library` names and executable symbol names are static or `CString`-owned and NUL terminated. Status text, agent strings, ISA names, and product names are copied before the foreign owner can change them. |
| Opaque handle | The handle originates from a successful creation or synchronous iterator. Rust ownership wrappers retain the runtime and enforce exactly one matching destroy. |
| Synchronous iterator callback | The collector is stack-owned for the complete foreign call, the callback ABI matches, null data is rejected, and panic is caught before returning a status. |
| Asynchronous queue callback | The callback context is boxed, held by `QueueCore`, and leaked on ambiguous destruction. It records only atomics and a wake notification. |
| AQL queue and packet | Queue creation proves a non-null power-of-two ring and kernel-dispatch feature. Packet size assertions, invalid-header initialization, release publication, and system fences establish visibility. |
| Signal operation | Signal value and ownership state are checked before loading, waiting, rearming, or destroying. Positive values stay in deferred retirement with all resource keepalives. |
| AMD memory operation | Pool capability, allocation size, pointer result, endpoint ranges, and direct access are validated. Foreign copies retain allocations until the completion signal reaches a terminal value. |

The resulting boundary has no fallback ABI, mock implementation, or alternate
symbol path. A missing library or symbol, an incompatible layout, a foreign
status, malformed attribute, invalid pointer result, queue fault, failed
signal, or unsafe lifetime condition remains visible through the corresponding
typed error or permanent resource-retirement report.

### Implementation-level failure map

Foreign failures always retain their numeric `HsaStatus`; the following
crate-level failures are raised before or immediately after the ABI call:

| ABI stage | Additional typed failures |
| --- | --- |
| `Library::open` and `Api::load` | `PathContainsNul`, `LibraryOpen`, and `MissingSymbol`. No partial `Api` is returned. |
| `HsaInit` and `HsaShutDown` | `Hsa` with no status text before initialization, `RuntimeClosed`, or a teardown `Hsa` from explicit `Runtime::close`. |
| System, agent, ISA, and pool queries | `CallbackPanicked`, `AllocationFailed`, `InvalidUtf8`, `InvalidIdentity`, `InvalidAttribute`, and annotated `Hsa`. Optional pool attributes alone become `None` for `STATUS_ERROR_INVALID_ARGUMENT`. |
| `HsaQueueCreate` and `HsaQueueDestroy` | `UnsupportedAgent`, `InvalidQueueSize`, `UnsupportedQueueKind`, `NullQueue`, `InvalidQueueReturned`, `ResourceBusy`, and `Hsa`. A failed destroy leaks callback context intentionally. |
| Code-object and executable functions | `EmptyCodeObject`, `NameContainsNul`, `SymbolNotKernel`, `InvalidKernel`, `ResourceBusy`, and `Hsa`. Reader backing bytes are retained if destruction is ambiguous. |
| AMD allocation and access functions | `InvalidMemoryPoolIndex`, `MemoryPoolNotAllocatable`, `InvalidAllocationSize`, `NullAllocation`, `NoMatchingMemoryPool`, `CopyOutOfBounds`, `InvalidDispatch`, and `Hsa`. |
| Async copy and signal functions | `NullSignal`, `AsyncSignal`, `SessionPoisoned`, `DeferredRetirement`, `AllocationFailed`, and `Hsa`. Positive completion signals remain owned by deferred retirement. |
| Queue index and AQL publication | `QueueFull`, `InvalidProgressRequest`, and `InvalidDispatch`; the raw index functions themselves have no status return, so pointer and publication invariants are established before the call. |
