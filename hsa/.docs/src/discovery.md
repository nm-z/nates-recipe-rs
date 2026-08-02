# `recipe_hsa::discovery`

## Intent

```yaml
module: recipe_hsa::discovery
source: hsa/src/discovery.rs
public_entrypoint: Runtime::discover
role: exhaustive-rocr-capability-snapshot
authority: active-ROCr-system-queries
outputs: SystemDescription plus ordered DiscoveredAgent records
handles: borrowed Runtime, raw agent handles, parallel raw memory-pool handles
state: owned descriptions with no guessed values
failure: any required query, conversion, validation, callback, or reservation error aborts the snapshot
```

 `discovery.rs` is the one boundary that turns a live, initialized ROCr/HSA
 runtime into typed descriptions. It does not select a GPU, allocate memory,
 create a queue, load a code object, benchmark a device, or invent a default
 for a missing capability. It queries the system, every agent, every ISA, and
 every AMD memory pool, then keeps the raw handles needed by the later session
 and execution layers next to the corresponding description.

 The source is intentionally exhaustive. `Runtime::discover` calls the private
 `discover` function (`hsa/src/runtime.rs:52-56`, `hsa/src/discovery.rs:154-188`)
 and returns only after all agents have been described. A failed query means no
 `Discovery` value is returned. The only absence cases represented in a valid
 snapshot are capabilities that ROCr explicitly reports as absent, such as a
 non-GPU agent's AMD GPU fields, all-zero queue fields, or an optional pool
 attribute rejected with `STATUS_ERROR_INVALID_ARGUMENT`.

## Public data model

 The module is private, but these description types are re-exported from
 `hsa/src/lib.rs:21-24`. They contain copies of runtime values, so callers can
 inspect them without issuing another query. `DiscoveredAgent` and `Discovery`
 retain a borrow of the `Runtime`; the borrow is the lifetime proof that ROCr
 remains initialized while raw agent, ISA, pool, queue, and session objects are
 used.

### System snapshot

 `SystemDescription` (`hsa/src/discovery.rs:14-21`) contains:

 | Field | Source query | Meaning |
 | --- | --- | --- |
 | `hsa_version_major` | `SYSTEM_INFO_VERSION_MAJOR` | Core HSA system major version. |
 | `hsa_version_minor` | `SYSTEM_INFO_VERSION_MINOR` | Core HSA system minor version. |
 | `amd_extension_version_major` | `AMD_SYSTEM_INFO_EXT_VERSION_MAJOR` | AMD extension major version. |
 | `amd_extension_version_minor` | `AMD_SYSTEM_INFO_EXT_VERSION_MINOR` | AMD extension minor version. |
 | `timestamp_frequency_hz` | `SYSTEM_INFO_TIMESTAMP_FREQUENCY` | ROCr timestamp tick frequency, in hertz. |

 The version values are also part of the native-probe runtime identity. The
 timestamp frequency is captured again when a GPU is consumed by
 `DiscoveredAgent::into_session` so that the session's completion-signal timing
 uses the active runtime value (`hsa/src/session.rs:171-190`).

### Agent identity and queue capabilities

 `AgentIdentity` (`hsa/src/discovery.rs:23-31`) owns the fixed-width ROCr agent
 name and vendor strings, the parsed `AgentUuid`, the NUMA node, and optional
 AMD driver-node and PCI values. `driver_node_id` and `pci_address` are `Some`
 only for a GPU. `PciAddress::from_hsa` decodes the AMD domain and BDF integer
 into domain, bus, device, and function (`hsa/src/identity.rs:385-411`).

 `QueueCapabilities` (`hsa/src/discovery.rs:33-39`) records the advertised
 user-mode queue limits:

 | Field | Meaning |
 | --- | --- |
 | `maximum_queues` | Maximum number of queues ROCr advertises for the agent. |
 | `minimum_packets` | Minimum queue packet count. |
 | `maximum_packets` | Maximum queue packet count. |
 | `advertised_kind` | ROCr's queue producer discipline. |

 The field is `None` only when all three numeric queue fields are zero. Any
 partially populated record is validated and rejected rather than treated as a
 queue with missing limits.

### AMD GPU properties

 A GPU agent receives `AmdGpuProperties` (`hsa/src/discovery.rs:41-59`). The
 values are direct AMD agent queries and are not normalized or inferred:

 | Group | Fields |
 | --- | --- |
 | Identity and clocks | `chip_id`, `asic_revision`, `maximum_clock_mhz`, `maximum_memory_clock_mhz`, `product_name`, `timestamp_frequency_hz` |
 | Execution width | `compute_unit_count`, `simds_per_compute_unit`, `maximum_waves_per_compute_unit`, `xcc_count` |
 | Cache and memory | `cacheline_bytes`, `available_memory_bytes` |
 | Copy engines | `sdma_engine_count`, `xgmi_sdma_engine_count` |
 | Scratch limits | `maximum_scratch_bytes`, `current_scratch_limit_bytes` |

 `amd_gpu: None` is the deliberate result for CPU, DSP, or AIE agents. A GPU
 that fails any required AMD query fails the whole discovery operation.

### ISA descriptions

 `IsaDescription` (`hsa/src/discovery.rs:61-80`) preserves the exact ROCr name
 in `name` and separately stores a parsed `IsaTarget` for AMD GPU names. The
 `amd_target` field is `None` for non-GPU ISAs. The remaining fields are direct
 ISA limits and feature flags:

 | Field | Meaning |
 | --- | --- |
 | `supports_small_machine_model`, `supports_large_machine_model` | The two machine-model bits returned by `ISA_INFO_MACHINE_MODELS`. |
 | `supports_base_profile`, `supports_full_profile` | The two profile bits returned by `ISA_INFO_PROFILES`. |
 | `default_rounding_modes` | Default floating-point rounding modes. |
 | `base_profile_rounding_modes` | Base-profile default rounding modes. |
 | `fast_f16` | `ISA_INFO_FAST_F16_OPERATION`, converted from a checked C boolean. |
 | `maximum_workgroup_dimensions` | Per-dimension workgroup limits. |
 | `maximum_workgroup_size` | Total workgroup cardinality limit. |
 | `maximum_grid_dimensions` | Per-dimension grid limits. |
 | `maximum_grid_size` | Total grid cardinality limit. |
 | `maximum_fbarriers_per_workgroup` | Maximum fbarrier count. |

 The vector order is the order returned by `hsa_agent_iterate_isas`. No target
 is dropped merely because another target is more specific. Native-probe later
 applies its exact-target policy to this complete vector.

### Memory-pool descriptions

 `MemoryPoolDescription` (`hsa/src/discovery.rs:89-100`) describes one pool in
 the same order as the private `RawPool` handle vector:

 | Field | Meaning |
 | --- | --- |
 | `segment` | `Global`, `ReadOnly`, `Private`, or `Group`. |
 | `location` | Optional `Cpu` or `Gpu` location. `None` means this ROCr version rejected the extension attribute, not that a location was guessed. |
 | `global_flags` | Raw global-pool flags for `Global` pools only; `None` for other segments. Unknown bits are retained. |
 | `size_bytes` | Pool size. |
 | `maximum_aggregate_allocation_bytes` | Optional aggregate allocation limit. |
 | `accessible_by_all_agents` | Checked C boolean from ROCr. |
 | `runtime_allocation` | `Some(AllocationProperties)` only when runtime allocation is allowed. |

 `AllocationProperties` (`hsa/src/discovery.rs:82-87`) stores the allocation
 granule, recommended granule, and alignment, all in bytes. These values are
 queried only for an allocatable pool. A non-allocatable pool still remains in
 the description so later selection can explain `MemoryPoolNotAllocatable` or
 `NoMatchingMemoryPool` instead of silently substituting another pool.

### Complete agent and discovery records

 `AgentDescription` (`hsa/src/discovery.rs:102-117`) combines identity,
 `DeviceType`, `Profile`, raw feature bits, agent HSA version, the deprecated
 first-ISA wavefront query, optional queue and AMD GPU properties, and the
 complete ISA and memory-pool vectors. `feature_bits` is deliberately raw: bits
 introduced by a newer runtime are retained. The two capability helpers are
 exact bit tests (`hsa/src/discovery.rs:119-123`):

 ```rust
 supports_kernel_dispatch = feature_bits & AGENT_FEATURE_KERNEL_DISPATCH != 0
 supports_agent_dispatch  = feature_bits & AGENT_FEATURE_AGENT_DISPATCH  != 0
 ```

 `DiscoveredAgent` (`hsa/src/discovery.rs:125-139`) owns the raw `HsaAgent`
 handle, the parallel `Vec<RawPool>` handles, the borrowed runtime, and its
 `AgentDescription`. Only `description()` is public. `Discovery`
 (`hsa/src/discovery.rs:141-152`) owns the system snapshot and the ordered
 agent records. `system()` and `agents()` borrow them; `into_agents()` consumes
 the discovery and moves the vector without reordering it.

## Runtime-to-description call graph

 ```text
 Runtime::open/open_default
   -> dynamic ROCr loader and hsa_init
 Runtime::discover
   -> ensure_active
   -> discover
      -> system_info x5
      -> iterate_agents
      -> reserve agent descriptions
      -> discover_agent for each raw HsaAgent
         -> agent_info and agent_string for identity/features/queues
         -> GPU-only AMD identity and properties
         -> iterate_isas -> discover_isa for every HsaIsa
         -> iterate_memory_pools -> discover_memory_pool for every pool
         -> optional first-ISA wavefront query
      -> Discovery { system, agents }
 ```

 `Runtime::open` resolves every reviewed function pointer before calling
 `hsa_init` (`hsa/src/loader.rs:168-270`, `hsa/src/runtime.rs:34-49`). A failed
 initialization cannot be described, and `Runtime::discover` checks
 `active` first. Runtime shutdown is balanced by `close` or `Drop`; Rust's
 lifetime borrow prevents a caller from closing the runtime while a discovery
 record or a session still borrows it (`hsa/src/runtime.rs:10-18,58-89`).

### System queries

 `discover` constructs the `SystemDescription` before touching agents
 (`hsa/src/discovery.rs:154-174`). Every call uses `system_info`, which allocates
 typed `MaybeUninit<T>`, invokes `hsa_system_get_info`, checks the status, and
 assumes initialization only after success (`hsa/src/discovery.rs:653-662`).
 The operation label is retained if the status fails, so a status error says
 which semantic field was being read.

### Agent enumeration

 `iterate_agents` passes an `AgentCollector` to the synchronous ROCr callback
 API (`hsa/src/discovery.rs:759-800`). The callback:

 1. rejects a null `data` pointer with `STATUS_ERROR`;
 2. treats `data` as the live collector for the duration of this traversal;
 3. catches a panic from `Vec::push`, marks `panicked`, and returns `STATUS_ERROR`;
 4. otherwise appends the raw `HsaAgent` and returns `STATUS_SUCCESS`.

 The iterator checks `panicked` before checking ROCr's returned status. A panic
 therefore becomes `Error::CallbackPanicked { operation:
 "hsa_iterate_agents" }`, while a normal non-success status becomes
 `Error::Hsa`. Once the raw vector exists, `discover` uses
 `try_reserve_exact` for the final description vector. A reservation failure
 is `Error::AllocationFailed { field: "agent descriptions", ... }` and no
 partial discovery escapes.

### Per-agent query order and branches

 `discover_agent` (`hsa/src/discovery.rs:191-438`) uses this order:

 1. Read and convert `AGENT_INFO_DEVICE`, raw feature bits, and
    `AGENT_INFO_PROFILE`.
 2. Read the 21-byte AMD UUID string, parse it, and require its device prefix
    to match `HSA_AGENT_INFO_DEVICE`.
 3. Read all three queue counts. Return `queue: None` only for the all-zero
    case; otherwise read and convert `AGENT_INFO_QUEUE_TYPE`, then validate the
    complete `QueueCapabilities`.
 4. For a GPU only, read the AMD driver node, PCI domain and BDF, then all
    `AmdGpuProperties`. Non-GPUs get `None` for all three GPU-specific identity
    and property fields.
 5. Enumerate every ISA, reserve both the result vector and each name buffer,
    and call `discover_isa` in enumeration order.
 6. Enumerate every memory pool, reserve both description and raw-handle
    vectors, and call `discover_memory_pool` in the same order. The two vectors
    are kept parallel by construction.
 7. If kernel dispatch is advertised, read the deprecated
    `AGENT_INFO_WAVEFRONT_SIZE` query into `first_isa_wavefront_size`; otherwise
    leave it `None`.
 8. Read the fixed-width agent name, vendor name, NUMA node, and HSA agent
    version, then assemble `AgentDescription` and `DiscoveredAgent`.

 All `?` operators in this path abort the outer `discover` call. The source
 does not return an agent with an incomplete identity or an unvalidated queue.

## Queue validation

 `validate_queue_capabilities` (`hsa/src/discovery.rs:440-457`) accepts a
 record only when all of the following are true:

 - `maximum_queues` is nonzero;
 - `minimum_packets` and `maximum_packets` are nonzero powers of two;
 - `minimum_packets <= maximum_packets`.

 It does not impose a power-of-two rule on the queue count. A failure returns
 `Error::InvalidQueueSize` with the reported minimum and maximum and the reason
 `ROCr reported inconsistent queue limits`. The same discovered limits are
 later enforced by `Session::create_queue`, which also checks a caller's
 requested queue kind (`hsa/src/session.rs:230-263`).

## ISA query and target parsing

 `discover_isa` (`hsa/src/discovery.rs:459-570`) reads the name in two stages:

 1. Query `ISA_INFO_NAME_LENGTH` as `u32`, convert to `usize`, and reject a
    value greater than `MAX_ISA_NAME_BYTES` (4096) with
    `InvalidAttribute`.
 2. Reserve exactly that many bytes, resize to zero, and call
    `hsa_isa_get_info_alt(ISA_INFO_NAME)` when the length is nonzero.

 The HSA header describes the length as excluding the NUL, while current ROCr
 releases include one trailing NUL. The parser accepts either form. After the
 first NUL, every byte must also be NUL; nonzero data after it is an
 `InvalidAttribute` rather than silently truncated. The remaining bytes are
 decoded as UTF-8. Invalid UTF-8 is `Error::InvalidUtf8 { field: "HSA ISA
 name" }`.

 For a GPU, the exact name is passed to `IsaTarget::from_str`. A non-GPU ISA
 keeps the exact name and receives `amd_target: None`, which is intentional and
 prevents it from being used for Recipe GPU calculation placement.

 The typed ISA queries and conversions are:

 | Query | Output | Conversion |
 | --- | --- | --- |
 | `ISA_INFO_MACHINE_MODELS` | `[u8; 2]` | `c_bool` for small and large model. |
 | `ISA_INFO_PROFILES` | `[u8; 2]` | `c_bool` for base and full profile. |
 | `ISA_INFO_DEFAULT_FLOAT_ROUNDING_MODES` | `[u8; 3]` | `RoundingModes::from_c_array`. |
 | `ISA_INFO_BASE_PROFILE_DEFAULT_FLOAT_ROUNDING_MODES` | `[u8; 3]` | `RoundingModes::from_c_array`. |
 | `ISA_INFO_FAST_F16_OPERATION` | `u8` | `c_bool`. |
 | `ISA_INFO_WORKGROUP_MAX_DIM` | `[u16; 3]` | Stored unchanged. |
 | `ISA_INFO_WORKGROUP_MAX_SIZE` | `u32` | Stored unchanged. |
 | `ISA_INFO_GRID_MAX_DIM` | `HsaDim3` | Reordered into `[x, y, z]`. |
 | `ISA_INFO_GRID_MAX_SIZE` | `u64` | Stored unchanged. |
 | `ISA_INFO_FBARRIER_MAX_SIZE` | `u32` | Stored unchanged. |

 `c_bool` accepts exactly `0` or `1` (`hsa/src/identity.rs:176-187`). Any
 other byte fails with `Error::InvalidAttribute`; there is no truthiness or
 version-dependent coercion.

 `IsaTarget` parsing (`hsa/src/identity.rs:293-383`) is strict. The string must
 start with `amdgcn-amd-amdhsa--`, have a lowercase `gfx...` architecture, and
 then zero or more ordered `name+` or `name-` feature modifiers. Feature names
 contain only lowercase ASCII letters, digits, and underscores, and duplicate
 names are rejected. The `raw` string remains unchanged so identity comparisons
 and artifact targeting use exactly what ROCr reported.

## Memory-pool query and handle pairing

 `discover_memory_pool` (`hsa/src/discovery.rs:572-650`) queries each pool in
 this order:

 1. Convert `AMD_MEMORY_POOL_INFO_SEGMENT` to `MemorySegment`.
 2. Read and validate `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALLOWED`.
 3. If allocation is allowed, read granule, recommended granule, and alignment
    into `AllocationProperties`; otherwise set `runtime_allocation` to `None`.
 4. Read the optional location attribute. `STATUS_ERROR_INVALID_ARGUMENT`
    means `location: None`; any other failure remains an HSA error. A returned
    value is converted to `MemoryLocation` and invalid raw values fail.
 5. Read global flags only for `MemorySegment::Global`; non-global pools retain
    `global_flags: None`. `MemoryPoolFlags` preserves unknown bits.
 6. Read mandatory size, optional maximum aggregate allocation, and checked
    all-agent accessibility.

 `pool_info_optional` (`hsa/src/discovery.rs:707-724`) is the only status
 fallback in this module. It returns `None` for the exact invalid-argument
 status and otherwise calls `Api::check`; no other status is treated as an
 absent capability. `RawPool { handle }` is pushed immediately after its
 description (`hsa/src/discovery.rs:368-390`), so a valid `DiscoveredAgent`
 always has equal-length, equal-order `raw_pools` and `memory_pools` vectors.

 The execution layer relies on that pairing. `allocate_from` indexes both
 vectors with the same pool index, requires `runtime_allocation: Some`, rejects
 zero or over-limit sizes, and records the pool's global flags on the allocation
 (`hsa/src/execution.rs:525-578`). `allocate_coarse`, `allocate_fine`, and
 `allocate_kernarg` select the first global, allocatable pool carrying the
 corresponding flag (`hsa/src/execution.rs:580-655`). A missing matching pool
 is `NoMatchingMemoryPool`, not a fallback to a different segment.

## Identity and profile conversions

 Discovery converts raw C enums before exposing them. The conversion authority
 is `hsa/src/identity.rs`:

 | Raw value | Typed value | Invalid value |
 | --- | --- | --- |
 | `HSA_AGENT_INFO_DEVICE`: `0,1,2,3` | `Cpu`, `Gpu`, `Dsp`, `Aie` | `InvalidAttribute` at `HSA_AGENT_INFO_DEVICE`. |
 | `HSA_AGENT_INFO_PROFILE`: `0,1` | `Profile::Base`, `Profile::Full` | `InvalidAttribute` at the supplied profile field. |
 | `HSA_AGENT_INFO_QUEUE_TYPE`: `0,1,2` | `MultiProducer`, `SingleProducer`, `Cooperative` | `InvalidAttribute` at `HSA_AGENT_INFO_QUEUE_TYPE`. |
 | `HSA_AMD_MEMORY_POOL_INFO_SEGMENT`: `0,1,2,3` | `Global`, `ReadOnly`, `Private`, `Group` | `InvalidAttribute` at the segment field. |
 | `HSA_AMD_MEMORY_POOL_INFO_LOCATION`: `0,1` | `Cpu`, `Gpu` | `InvalidAttribute` at the location field. |
 | C booleans | `false`, `true` for exactly `0`, `1` | `InvalidAttribute`. |

 `AgentUuid::from_str` accepts exactly one `DEVICE-BODY` separator. The device
 prefix is `CPU`, `GPU`, `DSP`, or `AIE`; the body is either `XX` (unavailable)
 or exactly 16 hexadecimal digits. An extra separator, unknown prefix, or
 malformed body is `InvalidIdentity`. `discover_agent` additionally compares
 the parsed prefix to `HSA_AGENT_INFO_DEVICE` and rejects disagreement
 (`hsa/src/discovery.rs:204-212`). Native-probe requires
 `AgentUuidBody::Value`, so `GPU-XX` is valid HSA identity syntax but is not a
 stable native GPU descriptor (`native-probe/src/hsa.rs:168-172`).

 `MemoryPoolFlags` stores the raw `u32`. Its public constants cover kernarg
 initialization, fine-grained, coarse-grained, and extended-scope fine-grained
 bits; `unknown_bits()` exposes newer bits instead of discarding them
 (`hsa/src/identity.rs:135-157`). `RoundingModes` is three checked booleans,
 not a bitmask. `Profile` is not inferred from ISA profile support; it is the
 agent profile queried from `HSA_AGENT_INFO_PROFILE` and is later passed to
 `hsa_executable_create_alt` (`hsa/src/execution.rs:974-1022`).

## Query wrappers, status handling, and allocation safety

 `system_info`, `agent_info`, `isa_info`, and `pool_info`
 (`hsa/src/discovery.rs:653-705`) share one invariant: every call site pairs a
 known ABI attribute with the exact output type. Each uses writable
 `MaybeUninit<T>`, calls the corresponding ROCr function, routes non-success
 statuses through `Api::check`, and calls `assume_init` only after success.

 `annotate_operation` (`hsa/src/discovery.rs:726-739`) replaces the generic
 `Error::Hsa.operation` with the semantic operation label supplied by the call
 site while preserving status and optional runtime message. Non-HSA errors are
 returned unchanged. The loader's `Api::check` obtains a runtime-owned status
 string only after initialization (`hsa/src/loader.rs:272-312`).

 Fixed-width strings use `agent_string` (`hsa/src/discovery.rs:741-757`). It
 requires a NUL within the complete array, slices before the first NUL, and
 validates UTF-8. A missing NUL is `InvalidAttribute` with the array size as
 the value. This rule applies to the 21-byte UUID and the 64-byte name,
 vendor, and product fields.

 The three callback adapters have the same safety contract:

 | Callback | Iterator | Panic error operation |
 | --- | --- | --- |
 | `collect_agent` | `iterate_agents` | `hsa_iterate_agents` |
 | `collect_isa` | `iterate_isas` | `hsa_agent_iterate_isas` |
 | `collect_pool` | `iterate_memory_pools` | `hsa_amd_agent_iterate_memory_pools` |

 Each rejects null callback data, catches a panic from `Vec::push`, and records
 `panicked` before returning an error status. ROCr traversal is synchronous, so
 the stack-owned collector and its pointer remain valid for the complete call.
 A callback panic cannot unwind through the C ABI. A normal runtime failure is
 reported by `Api::check` after the panic check.

## Discovery invariants and failure surface

 The following properties are established before `Discovery` is returned:

 | Invariant | Enforced by | Result if violated |
 | --- | --- | --- |
 | Runtime is initialized and active | `Runtime::discover`, `Runtime::open` | `RuntimeClosed` or HSA initialization error. |
 | Every required system value has the reviewed output type | `system_info` | Annotated `Hsa` error. |
 | Every enumerated agent is represented once, in ROCr order | `iterate_agents`, exact reservation, loop | Callback/HSA/allocation error aborts the snapshot. |
 | UUID syntax and device prefix agree | `AgentUuid::from_str`, explicit comparison | `InvalidIdentity`. |
 | Queue limits are complete, nonzero, power-of-two packet bounds | `validate_queue_capabilities` | `InvalidQueueSize`. |
 | GPU-only fields are queried only for GPUs | `device_type == DeviceType::Gpu` branch | Non-GPU fields are `None`; GPU query errors abort. |
 | Every ISA name is bounded, NUL-clean, UTF-8, and target-parseable for GPUs | `discover_isa` | `InvalidAttribute`, `InvalidUtf8`, or `InvalidIdentity`. |
 | ISA and pool vectors preserve ROCr order | iterator collectors and exact reservations | Any callback or reservation error aborts. |
 | Raw pool and description vectors remain parallel | paired push in `discover_agent` | No partial agent is returned. |
 | Optional pool attributes are absent only for invalid-argument status | `pool_info_optional` | Other statuses remain errors. |
 | New feature and pool flag bits are not discarded | raw `feature_bits`, `MemoryPoolFlags` | Forward-compatible bits remain inspectable. |
 | No guessed location, capacity, queue limit, target, or memory pool | explicit `Option` and strict conversions | Consumer receives a typed absence or an error. |

 Errors that can originate directly in this module are:

 - `Error::Hsa` for a non-success system, agent, ISA, or pool call, with a
   semantic operation label and runtime status message when available;
 - `Error::InvalidAttribute` for unknown enum values, malformed C booleans,
   oversized ISA names, missing or dirty NUL framing, or invalid optional enum
   values;
 - `Error::InvalidUtf8` for fixed-width agent or ISA strings;
 - `Error::InvalidIdentity` for UUID or GPU ISA target syntax and UUID/device
   disagreement;
 - `Error::InvalidQueueSize` for ROCr queue limits that cannot be used safely;
 - `Error::CallbackPanicked` when a collection callback catches a panic;
 - `Error::AllocationFailed` when exact vector or ISA-name reservation fails.

 There are no retries, alternative query paths, placeholder descriptions, or
 partial-success result values. `pool_info_optional` is the sole deliberate
 compatibility rule, and it is limited to the two optional pool attributes
 described above.

## Runtime consumers

### HSA session and execution

 `DiscoveredAgent::into_session` consumes one record into a GPU-only
 `Session` (`hsa/src/session.rs:145-199`). It requires an active runtime, a GPU
 device type, the kernel-dispatch feature bit, at least one ISA, and an AMD
 target on every ISA. It then re-queries system timestamp frequency and creates
 the session's fault state and signal pool. The raw agent, raw pool handles, and
 description move into the session, so the discovery record cannot be reused.

 `Session::create_queue` consumes `AgentDescription.queue`. It requires the
 requested size to be a power of two inside the discovered packet interval and
 allows a requested kind only when the advertised kind permits it. After
 `hsa_queue_create`, it validates the returned queue pointer, base address,
 size, kernel-dispatch feature, and queue kind; a malformed successful result is
 destroyed and returned as `InvalidQueueReturned` (`hsa/src/session.rs:230-335`).

 `hsa/src/execution.rs` consumes the discovery snapshot in three ways:

 - memory allocation indexes the raw and described pool vectors together;
 - `allocate_coarse`, `allocate_fine`, and `allocate_kernarg` select pools by
   segment, allocatability, and `MemoryPoolFlags`;
 - `load_hsaco` passes the discovered `Profile` to ROCr executable creation,
   while dispatch geometry checks every discovered ISA's dimensions, total
   workgroup size, and total grid size (`hsa/src/execution.rs:974-1059,
   1909-1960`).

 The latter check is intentionally an all-ISA check. A geometry valid for one
 enumerated ISA but beyond another's exact limit is rejected; the runtime does
 not select a more permissive ISA silently.

### Public examples

 `hsa/examples/discover.rs:3-13` demonstrates the ownership boundary: open a
 runtime, borrow a discovery to print `system()` and each `description()`, end
 the borrow, then call `Runtime::close`. `hsa/examples/execute_smoke.rs:5-19`
 consumes `into_agents()`, selects the first CPU and GPU by `DeviceType`, and
 calls `into_session`. Its `allocate_fine` and `allocate_coarse` calls prove
 that pool flags and allocatability are resolved from discovery, while the
 queue uses `description().queue.minimum_packets` (`execute_smoke.rs:82-88`).

## Native-probe consumer

 `native-probe/src/hsa.rs` is the AMD backend that turns a discovery snapshot
 into a measured `GpuDescriptor` and then into `GpuMeasurement`.

### Descriptor admission

 `HsaBackend::descriptor` (`native-probe/src/hsa.rs:153-294`) ignores a
 non-GPU agent, but rejects a GPU when any identity or capability needed for
 native execution is absent:

 - kernel-dispatch support must be advertised;
 - `AgentUuid.body()` must be `Value`, not `Unavailable`;
 - `AgentIdentity.pci_address`, `amd_gpu`, and `queue` must be present;
 - `exact_target(description)` must choose one target;
 - the KFD node's `lds_size_in_kb` must exist, parse as a nonzero `u64`, and
   fit the resulting byte count.

 The descriptor key is `hsa:<agent UUID>@<canonical PCI address>`. Its target
 backend is `amd-rocr-hsa`, architecture is the exact `IsaTarget` string, and
 ABI is `elf64-amdgpu-code-object-v<configured version>`. The descriptor copies
 queue maximums, first-ISA wavefront width, the minimum workgroup limit across
 all ISAs, and KFD LDS capacity. It advertises full-duplex PCIe, one host to
 device and one device to host transfer lane, asynchronous submission, and one
 concurrent calculation. Transfer overlap is true only when
 `sdma_engine_count != 0`.

 `hsa_capacity` (`native-probe/src/hsa.rs:685-725`) first chooses the largest
 pool that is global, GPU-located, runtime-allocatable, and coarse-grained. If
 no such pool exists, it uses nonzero `amd_gpu.available_memory_bytes`. If
 neither source exists, descriptor construction fails. This capacity is a
 descriptor hint; the later benchmark records measured capacity.

### Exact target policy

 `exact_target` (`native-probe/src/hsa.rs:633-683`) requires every ISA in a
 GPU description to have an AMD target. It collects exact target strings into a
 set and partitions targets by whether `architecture().ends_with("-generic")`:

 1. One distinct non-generic target wins, even when generic targets are also
    present.
 2. With no non-generic target, one distinct generic target wins.
 3. Multiple distinct non-generic targets, multiple generic targets, or an empty
    target set is a discovery error.

 The chosen full triple is retained for the descriptor and later stripped only
 at the compiler boundary by `hsa_target_tail` (`native-probe/src/hsa.rs:727-736`).

### Re-discovery and benchmark

 `HsaBackend::discover` calls `Runtime::discover` and applies the descriptor
 admission rules to every agent (`native-probe/src/hsa.rs:608-625`).
 `NativeGpuProbe::discover_all` merges CUDA and HSA descriptors, sorts by key,
 and rejects duplicate keys (`native-probe/src/native.rs:245-264`). The normal
 `NativeGpuProbe::new` is exhaustive; `hsa_diagnostic` is deliberately
 non-exhaustive and therefore cannot produce an accepted measured profile
 (`native-probe/src/native.rs:75-85`).

 Before benchmarking, `benchmark_with_runtime` rediscovers the full topology
 and requires exactly one agent whose descriptor is byte-for-byte equal to the
 expected descriptor (`native-probe/src/hsa.rs:307-349`). A missing, changed,
 or ambiguous UUID is a benchmark failure. It then selects a CPU memory agent,
 preferring the GPU's NUMA node and using the first available fallback only
 when the same-node choice is absent.

 The benchmark exercises the discovered capabilities end to end: it allocates
 CPU fine-grained and GPU coarse-grained buffers, grants access, submits and
 verifies host-to-device, device-to-host, and device-to-device copies, lowers
 and realizes a Recipe-owned FMA HSACO for the exact target, dispatches it with
 an ISA-derived workgroup, downloads and verifies output, and returns measured
 capacity and rates (`native-probe/src/hsa.rs:347-452`). A failed query,
 allocation, queue, copy, output comparison, code-object load, or completion is
 a `ProbeError`; no estimated result is substituted.

 `HsaBackend::with_runtime` pins one library identity and one `Runtime`
 (`native-probe/src/hsa.rs:112-149`). An absent AMD PCI accelerator yields
 `None` before initialization. Once initialized, disappearance of the PCI
 surface or library, or a changed library identity, is an error. This prevents
 a later discovery from using a different ROCr runtime than the one that owns
 the raw handles.

## Native binding and executor consumers

 `native-probe/src/bindings.rs` reopens discovery during preparation, not in a
 running loop. `with_native_execution_bindings` first requires an exhaustive
 current GPU inventory and a `MeasuredProfile` whose exact machine, RAM,
 storage, and GPU origins still resolve (`bindings.rs:120-139`). It then
 reopens the retained HSA runtime and calls `realize_hsa` with owned agents
 (`bindings.rs:179-215`).

 `realize_hsa` partitions CPU host allocators from GPU agents, filters CPU
 allocators through `supports_host_allocation`, and sorts them by NUMA node,
 UUID, name, and vendor (`bindings.rs:319-345`). A host allocator must have a
 global, runtime-allocatable kernarg pool and a global, runtime-allocatable
 fine-grained or extended-scope fine-grained pool (`bindings.rs:416-435`). For
 each reopened GPU it requires:

 - a descriptor key present in the measured profile and not duplicated;
 - one exact target selected by `exact_target`;
 - a current queue capability, whose minimum packet count becomes
   `queue_packets` and whose maximum queue count becomes
   `maximum_submission_queues`;
 - one unambiguous host allocator, preferring exactly one allocator on the same
   NUMA node and otherwise allowing exactly one global fallback;
 - successful `DiscoveredAgent::into_session`.

 The resulting `HsaBinding` borrows the `Session` and host allocator and stores
 the exact target string, code-object version, queue packet size, queue count,
 and display-connector count (`native-probe/src/bindings.rs:392-413`). Missing
 or changed measured origins, duplicate agents, unsupported backends, absent
 queue capabilities, ambiguous host allocators, and failed sessions are
 `ProbeError::Discovery` values. There is no ordinal, product-name, capacity,
 or performance fallback.

 `native-executor/src/hsa.rs:31-100` exposes those binding fields to the
 execution backend. Before realizing resources, `HsaResources::realize` checks
 submission queue demand against `maximum_submission_queues` and calls
 `validate_binding` (`native-executor/src/hsa.rs:384-448`). That validator
 rechecks that the host allocator is a CPU with allocatable kernarg and fine
 pools and that the session advertises the exact `target_id`
 (`native-executor/src/hsa.rs:1692-1738`). Queue realization uses the
 discovery-derived `queue_packets` with `QueueKind::SingleProducer`
 (`native-executor/src/hsa.rs:1808-1842`). HSACO artifacts must carry the same
 target string and code-object version as the binding (`native-executor/src/hsa.rs:1904-1920`).

 `src/native_prepare.rs:705-745` repeats the artifact-facing target check and
 strips the canonical `amdgcn-amd-amdhsa--` prefix only when constructing the
 compiler's `AmdTarget`. Thus discovery owns the full identity, native-probe
 carries it into a binding, and native-executor rejects any artifact or runtime
 session that differs from it.

## Preparation and failure boundaries

 Discovery is preparation-time evidence, not model work in the immutable
 `init -> loop -> exit` lifecycle. The ownership sequence is:

 ```text
 initialized Runtime
   -> Discovery (borrowed raw topology plus owned descriptions)
   -> DiscoveredAgent::into_session (GPU session and signal pool)
   -> native-probe descriptor and measured benchmark
   -> exact NativeExecutionBindings
   -> native-executor HsaBinding
   -> pre-realized queues, allocations, executables, and pending tokens
   -> loop submits only already-realized work
 ```

 A failure at discovery, identity matching, descriptor reopening, target
 validation, or session construction stops preparation. It must not be hidden by
 an estimated profile, a different runtime, a different agent, or a synthetic
 backend. Later asynchronous failures are owned by `Session`'s queue-fault and
 completion-signal machinery, but their initial queue and geometry assumptions
 come directly from this snapshot.
