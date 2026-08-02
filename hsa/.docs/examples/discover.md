# `discover` example

## Purpose

[`hsa/examples/discover.rs`](../../examples/discover.rs) (lines 1-14) is the
smallest live demonstration of the public `recipe-hsa` discovery boundary. It
does not create a queue, allocate memory, load a code object, or submit work. Its
one job is to open ROCr, read the complete visible system and agent descriptions,
print those descriptions, and perform the matching shutdown.

The example is deliberately a real runtime probe. The values are read from the
installed ROCr driver at process start, so agent count, names, PCI addresses,
ISA limits, memory-pool sizes, and clock or availability counters are
machine-dependent. The example has no command-line arguments, environment
variables, configuration file, synthetic data, or fallback description.

## Source shape

The complete example is structurally four operations:

```rust
use recipe_hsa::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let runtime = Runtime::open_default()?;
	{
		let discovery = runtime.discover()?;
		println!("system: {:#?}", discovery.system());
		for (index, agent) in discovery.agents().iter().enumerate() {
			println!("agent {index}: {:#?}", agent.description());
		}
	}
	runtime.close()?;
	Ok(())
}
```

`main` returns a boxed standard error, so each `recipe_hsa::Error` produced by
the `?` operator reaches the process-level `Result` termination path. On a
successful run the only intentional output is two kinds of pretty `Debug`
records on standard output. On failure, Rust reports the error on standard
error and exits unsuccessfully. A panic while formatting or writing standard
output is not converted into a `recipe-hsa` error.

The inner scope is required by the public lifetime API. `Discovery<'runtime>`
and every `DiscoveredAgent<'runtime>` borrow `Runtime`; the borrow must end
before the consuming `Runtime::close` call. Dropping the discovery record drops
ordinary Rust vectors and strings, not an owned ROCr agent or memory-pool
handle. The runtime owns one balanced initialization reference and performs the
one shutdown; `recipe-hsa` intentionally does not hide a process-global runtime
behind this example.

## Invocation and preconditions

The example is an ordinary package example and does not require the
`live-hsa` feature. The direct production invocation is:

```text
cargo run -p recipe-hsa --example discover
```

The crate currently implements only the 64-bit HSA large-model ABI. A build for
another pointer width stops at the crate-level compile error before this
program can run. At runtime, a usable ROCr installation must expose the
library and every symbol that the crate resolves while opening it. The example
binary has no link-time `libhsa-runtime64` dependency: `ldd` shows only the
normal C runtime libraries, and the two ROCr sonames are looked up by `dlopen`
after process start.

## Public calls made by the example

| Source operation | Public API | Effect |
| --- | --- | --- |
| Open | `Runtime::open_default()` | Tries the normal ROCr sonames, loads the reviewed ABI, and calls `hsa_init`. |
| Discover | `Runtime::discover()` | Checks that the runtime is active, reads system attributes, enumerates agents, and builds owned descriptions that borrow the runtime. |
| System print | `Discovery::system()` | Borrows the `SystemDescription` for pretty `Debug` formatting. |
| Agent print | `Discovery::agents()` and `DiscoveredAgent::description()` | Borrows the ordered agent slice and each immutable `AgentDescription` for pretty `Debug` formatting. |
| Close | `Runtime::close()` | Consumes the runtime, calls `hsa_shut_down`, and reports its status. |

`Discovery::into_agents()` is public but is not used here. This source path makes
no execution API calls.

The public re-exports are defined in [`hsa/src/lib.rs`](../../src/lib.rs),
runtime lifetime and shutdown behavior in [`hsa/src/runtime.rs`](../../src/runtime.rs)
(lines 20-89), the discovery record construction and callback collectors in
[`hsa/src/discovery.rs`](../../src/discovery.rs) (lines 154-884), dynamic
loading and status conversion in [`hsa/src/loader.rs`](../../src/loader.rs)
(lines 168-313), and raw enum and identity validation in
[`hsa/src/identity.rs`](../../src/identity.rs). The raw attribute numbers and
function-pointer signatures used below are in [`hsa/src/abi.rs`](../../src/abi.rs);
the typed failure variants and their process-visible formatting are in
[`hsa/src/error.rs`](../../src/error.rs).

## End-to-end runtime trace

### 1. Loading and initialization

`Runtime::open_default` tries these candidates in order:

1. `libhsa-runtime64.so.1`
2. `libhsa-runtime64.so`

It retries the second candidate only when the first `dlopen` returns
`Error::LibraryOpen`. Any other error from the first candidate, including a
missing symbol or an `hsa_init` failure, is returned immediately. If both
libraries cannot be opened, the last `LibraryOpen` error is returned.

For each candidate, `Runtime::open` calls `Api::load`, which uses `dlopen` with
`RTLD_NOW | RTLD_LOCAL`. The path is converted to a NUL-terminated C string;
an interior NUL would return `PathContainsNul` (the fixed default candidates do
not contain one). `Api::load` resolves all of the following symbols before
`hsa_init`, even though this example exercises only the discovery subset:

```text
hsa_init
hsa_shut_down
hsa_status_string
hsa_system_get_info
hsa_iterate_agents
hsa_agent_get_info
hsa_agent_iterate_isas
hsa_isa_get_info_alt
hsa_queue_create
hsa_queue_destroy
hsa_amd_agent_iterate_memory_pools
hsa_amd_memory_pool_get_info
hsa_code_object_reader_create_from_memory
hsa_code_object_reader_destroy
hsa_executable_create_alt
hsa_executable_destroy
hsa_executable_load_agent_code_object
hsa_executable_freeze
hsa_executable_get_symbol_by_name
hsa_executable_symbol_get_info
hsa_amd_memory_pool_allocate
hsa_amd_memory_pool_free
hsa_amd_agents_allow_access
hsa_amd_memory_async_copy
hsa_signal_create
hsa_signal_destroy
hsa_signal_load_scacquire
hsa_signal_store_screlease
hsa_signal_wait_scacquire
hsa_queue_load_read_index_scacquire
hsa_queue_load_write_index_relaxed
hsa_queue_store_write_index_screlease
```

There are 32 required symbols in this list. All 32 must resolve before the
initialization call, so a library that implements discovery but omits an
execution symbol still fails at `Runtime::open_default`.

The loader clears the thread-local `dlerror` state before each `dlopen` or
`dlsym`, copies any resulting diagnostic immediately, and treats both a
reported loader error and a null function address as `MissingSymbol`. The
resolved addresses are transmuted to the reviewed function-pointer aliases in
`abi.rs`; no runtime call is made through an untyped address. If symbol loading
fails partway through the list, the already opened library is dropped and
`dlclose` is performed before the error leaves `Api::load`.

`dlsym` failures become `MissingSymbol` with the library, symbol name, and
loader detail. A successfully loaded `Api` is held in an `Arc`, and its
`Library` keeps the defining shared object loaded while all runtime objects
exist.

After symbol resolution, `Runtime::open` calls the resolved `hsa_init` pointer.
A non-success status becomes `Error::Hsa { operation: "hsa_init", ... }`.
Because the runtime is not initialized at that boundary, the error intentionally
has no status-string message. On success, `Runtime` stores the `Arc<Api>` and an
active flag. Each successful `Runtime::open` owns one balanced ROCr
initialization reference; this API does not assume or create a singleton, so a
separate caller may hold another `Runtime` at the same time.

### 2. Entering discovery

`Runtime::discover` first checks the active flag. An inactive object would
return `RuntimeClosed`; the safe consuming `close` API normally makes such an
object unavailable to a caller. It then calls the crate's discovery engine with
the runtime borrow, establishing the lifetime that prevents shutdown while
descriptions are live.

The discovery engine reads the following five system attributes in order with
`hsa_system_get_info`:

| HSA attribute | Output type | `SystemDescription` field |
| --- | --- | --- |
| `SYSTEM_INFO_VERSION_MAJOR` (`0`) | `u16` | `hsa_version_major` |
| `SYSTEM_INFO_VERSION_MINOR` (`1`) | `u16` | `hsa_version_minor` |
| `AMD_SYSTEM_INFO_EXT_VERSION_MAJOR` (`0x207`) | `u16` | `amd_extension_version_major` |
| `AMD_SYSTEM_INFO_EXT_VERSION_MINOR` (`0x208`) | `u16` | `amd_extension_version_minor` |
| `SYSTEM_INFO_TIMESTAMP_FREQUENCY` (`3`) | `u64` | `timestamp_frequency_hz` |

Every failed query is checked through the loaded `hsa_status_string` function
and then annotated with its field-specific operation text, such as `HSA system
major version`. A non-success runtime status therefore stops discovery before
agent enumeration and returns `Error::Hsa` with the numeric status and, when
ROCr supplies one, its human-readable message.

The raw agent, ISA, and memory-pool attribute numbers used by the subsequent
queries are the reviewed constants in `abi.rs`, not values supplied by the
example:

| Query family | Attributes and numeric IDs |
| --- | --- |
| Common agent | `NAME=0`, `VENDOR_NAME=1`, `FEATURE=2`, `PROFILE=4`, `WAVEFRONT_SIZE=6`, `QUEUES_MAX=12`, `QUEUE_MIN_SIZE=13`, `QUEUE_MAX_SIZE=14`, `QUEUE_TYPE=15`, `NODE=16`, `DEVICE=17`, `VERSION_MAJOR=21`, `VERSION_MINOR=22`. |
| AMD agent | `CHIP_ID=0xA000`, `CACHELINE_SIZE=0xA001`, `COMPUTE_UNIT_COUNT=0xA002`, `MAX_CLOCK_FREQUENCY=0xA003`, `DRIVER_NODE_ID=0xA004`, `BDFID=0xA006`, `MEMORY_MAX_FREQUENCY=0xA008`, `PRODUCT_NAME=0xA009`, `MAX_WAVES_PER_CU=0xA00A`, `NUM_SIMDS_PER_CU=0xA00B`, `DOMAIN=0xA00F`, `UUID=0xA011`, `ASIC_REVISION=0xA012`, `MEMORY_AVAIL=0xA015`, `TIMESTAMP_FREQUENCY=0xA016`, `NUM_SDMA_ENG=0xA10A`, `NUM_SDMA_XGMI_ENG=0xA10B`, `NUM_XCC=0xA111`, `SCRATCH_LIMIT_MAX=0xA116`, `SCRATCH_LIMIT_CURRENT=0xA117`. |
| ISA | `NAME_LENGTH=0`, `NAME=1`, `MACHINE_MODELS=5`, `PROFILES=6`, `DEFAULT_FLOAT_ROUNDING_MODES=7`, `BASE_PROFILE_DEFAULT_FLOAT_ROUNDING_MODES=8`, `FAST_F16_OPERATION=9`, `WORKGROUP_MAX_DIM=12`, `WORKGROUP_MAX_SIZE=13`, `GRID_MAX_DIM=14`, `GRID_MAX_SIZE=16`, `FBARRIER_MAX_SIZE=17`. |
| AMD memory pool | `SEGMENT=0`, `GLOBAL_FLAGS=1`, `SIZE=2`, `RUNTIME_ALLOC_ALLOWED=5`, `RUNTIME_ALLOC_GRANULE=6`, `RUNTIME_ALLOC_ALIGNMENT=7`, `ACCESSIBLE_BY_ALL=15`, `ALLOC_MAX_SIZE=16`, `LOCATION=17`, `RUNTIME_ALLOC_REC_GRANULE=18`. |

The discovery calls use these raw function-pointer shapes from `abi.rs`:

| Function pointer | Shape and callback |
| --- | --- |
| `HsaStatusString` | `unsafe extern "C" fn(HsaStatus, *mut *const c_char) -> HsaStatus`, used only to enrich a failed status. |
| `HsaSystemGetInfo` | `unsafe extern "C" fn(HsaInfo, *mut c_void) -> HsaStatus`. |
| `HsaIterateAgents` | `unsafe extern "C" fn(AgentCallback, *mut c_void) -> HsaStatus`, where `AgentCallback` receives `HsaAgent`. |
| `HsaAgentGetInfo` | `unsafe extern "C" fn(HsaAgent, HsaInfo, *mut c_void) -> HsaStatus`. |
| `HsaAgentIterateIsas` | `unsafe extern "C" fn(HsaAgent, IsaCallback, *mut c_void) -> HsaStatus`, where `IsaCallback` receives `HsaIsa`. |
| `HsaIsaGetInfoAlt` | `unsafe extern "C" fn(HsaIsa, HsaInfo, *mut c_void) -> HsaStatus`. |
| `HsaAmdAgentIterateMemoryPools` | `unsafe extern "C" fn(HsaAgent, MemoryPoolCallback, *mut c_void) -> HsaStatus`, where `MemoryPoolCallback` receives `HsaMemoryPool`. |
| `HsaAmdMemoryPoolGetInfo` | `unsafe extern "C" fn(HsaMemoryPool, HsaInfo, *mut c_void) -> HsaStatus`. |

### 3. Enumerating raw agents

`hsa_iterate_agents` is called once with a synchronous C callback and a pointer
to an `AgentCollector`. Each callback appends the raw `HsaAgent` handle to a
Rust vector. A null callback data pointer returns the generic HSA error status.
The vector push is wrapped in `catch_unwind`; an allocation panic is recorded
and the callback returns an error status rather than unwinding across the C ABI.
The reviewed ABI represents status and attribute selectors as signed 32-bit
integers. Success is `0`, the generic error returned by a bad callback pointer
is `0x1000`, and the optional-pool compatibility status is `0x1001`.
ROCr invokes the callback serially on the calling thread and stops the
traversal when a callback returns a non-success status, so a captured panic
cannot leave a later raw handle silently appended.
After ROCr returns, a recorded callback panic has priority and becomes
`CallbackPanicked { operation: "hsa_iterate_agents" }`. Otherwise the traversal
status is checked as `hsa_iterate_agents` and the collected handles are
returned.

The description vector then reserves exactly the raw-agent count. An explicit
reserve failure is `AllocationFailed { field: "agent descriptions", ... }`.
Each raw handle is expanded completely before the next handle is processed; the
first per-agent error aborts the whole discovery result. Agent order is the
order ROCr supplied and is preserved by the final `agents` slice. The raw
`HsaAgent`, `HsaIsa`, and `HsaMemoryPool` values are `#[repr(C)]` handle records
copied into Rust vectors. Discovery does not create or destroy any of them, so
there is no per-agent teardown call hidden in this example.

The same explicit reserve boundary is used for `ISA descriptions`,
`memory-pool descriptions`, and `memory-pool handles`. The ISA name buffer has
its own `AllocationFailed { field: "HSA ISA name", ... }` boundary. These are
the allocation failures that are intentionally converted to `recipe-hsa::Error`
instead of being hidden behind a partial record.

### 4. Expanding one agent

For each raw handle, `discover_agent` uses `hsa_agent_get_info` for the base
identity and capabilities. The generic query helper pairs each attribute with
the exact Rust output type, checks the HSA status, and annotates an error with a
field description.

#### Base identity and feature data

The following values are read for every agent:

* `HSA_AGENT_INFO_DEVICE` maps raw `0`, `1`, `2`, and `3` to `Cpu`, `Gpu`,
  `Dsp`, and `Aie`. Any other value is `InvalidAttribute`.
* `HSA_AGENT_INFO_FEATURE` is retained as raw `feature_bits`, so unknown bits
  from a newer runtime are not discarded. `supports_kernel_dispatch` and
  `supports_agent_dispatch` later test bits `1` and `2` respectively.
* `HSA_AGENT_INFO_PROFILE` maps `0` to `Base` and `1` to `Full`; other values
  are invalid.
* `AMD_AGENT_INFO_UUID` is read as 21 bytes. The value must be a NUL-terminated
  UTF-8 string in the form `DEVICE-BODY`, where `DEVICE` is `CPU`, `GPU`,
  `DSP`, or `AIE`, and `BODY` is either `XX` or exactly 16 hexadecimal digits.
  Extra separators, unknown prefixes, malformed bodies, missing NUL, or invalid
  UTF-8 are typed identity or attribute errors. The parsed UUID device prefix
  must agree with `HSA_AGENT_INFO_DEVICE`.

Queue limits are then read as `u32` values from
`HSA_AGENT_INFO_QUEUE_MIN_SIZE`, `HSA_AGENT_INFO_QUEUE_MAX_SIZE`, and
`HSA_AGENT_INFO_QUEUES_MAX`. If all three are zero, `queue` is `None` and the
queue kind is not queried. Otherwise `HSA_AGENT_INFO_QUEUE_TYPE` is mapped to
`MultiProducer`, `SingleProducer`, or `Cooperative`, and the limits must have a
nonzero maximum queue count, nonzero power-of-two packet bounds, and
`minimum_packets <= maximum_packets`. An unknown queue-kind value is an
`InvalidAttribute`; inconsistent limits return `InvalidQueueSize` rather than
being normalized.

#### GPU-only properties

For a `Gpu` agent, discovery additionally queries:

* `AMD_AGENT_INFO_DRIVER_NODE_ID` as `driver_node_id`;
* `AMD_AGENT_INFO_DOMAIN` and `AMD_AGENT_INFO_BDFID`, combined into
  `PciAddress { domain, bus, device, function }` by the HSA BDF bit layout;
* `AMD_AGENT_INFO_CHIP_ID`;
* `AMD_AGENT_INFO_ASIC_REVISION`;
* `AMD_AGENT_INFO_CACHELINE_SIZE`;
* `AMD_AGENT_INFO_COMPUTE_UNIT_COUNT`;
* `AMD_AGENT_INFO_NUM_SIMDS_PER_CU`;
* `AMD_AGENT_INFO_MAX_WAVES_PER_CU`;
* `AMD_AGENT_INFO_MAX_CLOCK_FREQUENCY`;
* `AMD_AGENT_INFO_MEMORY_MAX_FREQUENCY`;
* `AMD_AGENT_INFO_PRODUCT_NAME` as a 64-byte NUL-terminated UTF-8 string;
* `AMD_AGENT_INFO_MEMORY_AVAIL`;
* `AMD_AGENT_INFO_TIMESTAMP_FREQUENCY`;
* `AMD_AGENT_INFO_NUM_SDMA_ENG`;
* `AMD_AGENT_INFO_NUM_SDMA_XGMI_ENG`;
* `AMD_AGENT_INFO_NUM_XCC`;
* `AMD_AGENT_INFO_SCRATCH_LIMIT_MAX`; and
* `AMD_AGENT_INFO_SCRATCH_LIMIT_CURRENT`.

These become `AmdGpuProperties`. CPU, DSP, and AIE agents leave
`driver_node_id`, `pci_address`, and `amd_gpu` as `None`; no AMD GPU attribute
is queried for them.

#### ISA enumeration and validation

`hsa_agent_iterate_isas` is called for every agent, including agents that have
no kernel-dispatch feature. Its callback collector has the same null-pointer,
panic capture, and post-callback status handling as the agent collector, with
`hsa_agent_iterate_isas` as the operation name. The ISA vector reserves exactly
its reported count. ROCr specifies a deterministic order for an agent's ISA
traversal, and discovery preserves that order.

Each raw ISA is expanded with `hsa_isa_get_info_alt`:

1. `ISA_INFO_NAME_LENGTH` is read as `u32`. Values over the hard safety limit
   of 4096 bytes return `InvalidAttribute` before allocation.
2. Exactly that many bytes are allocated and `ISA_INFO_NAME` is queried when
   the length is nonzero. ROCr variants that report a length excluding or
   including a trailing NUL are both accepted. After the first NUL, every byte
   must also be NUL; otherwise the name is invalid. The remaining bytes must be
   UTF-8.
3. A GPU ISA name is parsed as an `IsaTarget` and must begin with
   `amdgcn-amd-amdhsa--`, followed by a lowercase `gfx` architecture and
   optional unique feature modifiers ending in `+` or `-`. Empty, malformed, or
   duplicate feature modifiers are invalid. Non-GPU ISA names are retained
   exactly and have no parsed AMD target.
4. `ISA_INFO_MACHINE_MODELS` (`[u8; 2]`) and `ISA_INFO_PROFILES` (`[u8; 2]`)
   are converted through strict C-boolean validation into the small or large
   machine-model and base or full-profile flags.
5. The default and base-profile rounding-mode attributes are each read as
   `[u8; 3]` and converted to strict boolean `RoundingModes` values.
6. `ISA_INFO_FAST_F16_OPERATION` is another strict boolean.
7. Workgroup limits are read from `ISA_INFO_WORKGROUP_MAX_DIM`
   (`[u16; 3]`) and `ISA_INFO_WORKGROUP_MAX_SIZE` (`u32`).
8. Grid limits are read from `ISA_INFO_GRID_MAX_DIM` (`HsaDim3`) and
   `ISA_INFO_GRID_MAX_SIZE` (`u64`). The three `HsaDim3` fields become
   `maximum_grid_dimensions`.
9. `ISA_INFO_FBARRIER_MAX_SIZE` becomes
   `maximum_fbarriers_per_workgroup`.

Any HSA status, invalid enum or boolean, malformed target identity, invalid UTF-8,
or allocation failure aborts the complete discovery result. No ISA is silently
dropped.

#### Memory-pool enumeration and validation

`hsa_amd_agent_iterate_memory_pools` is called for every agent. Its collector
uses the same synchronous callback contract and panic handling, with
`hsa_amd_agent_iterate_memory_pools` as the operation name. The code reserves
both the public memory-pool descriptions and the parallel raw pool handles
before expanding each pool.

For each pool, `hsa_amd_memory_pool_get_info` supplies:

* `AMD_MEMORY_POOL_INFO_SEGMENT`, mapped to `Global`, `ReadOnly`, `Private`, or
  `Group`;
* `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALLOWED`, a strict C boolean;
* when runtime allocation is allowed, granule, recommended granule, and
  alignment from `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_GRANULE`,
  `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_REC_GRANULE`, and
  `AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALIGNMENT`;
* `AMD_MEMORY_POOL_INFO_LOCATION`, optionally mapped to `Cpu` or `Gpu`;
* `AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS` only for a `Global` segment, retaining
  unknown flag bits;
* `AMD_MEMORY_POOL_INFO_SIZE`;
* `AMD_MEMORY_POOL_INFO_ALLOC_MAX_SIZE`, optionally; and
* `AMD_MEMORY_POOL_INFO_ACCESSIBLE_BY_ALL`, a strict C boolean.

The two optional queries have one deliberate compatibility boundary:
`STATUS_ERROR_INVALID_ARGUMENT` means that the installed ROCr does not expose
that extension attribute, so the result is `None`. Every other non-success
status remains an `Error::Hsa`. An unsupported or malformed enum is an
`InvalidAttribute`, not a guessed location or segment. The public description
contains the metadata, while the parallel raw handle is retained for later
session or allocation APIs.

Finally, if the feature bits include kernel dispatch, discovery reads
`HSA_AGENT_INFO_WAVEFRONT_SIZE` as `first_isa_wavefront_size`. Agents without
that feature get `None` and do not incur that query. The final identity strings
are then read from `HSA_AGENT_INFO_NAME` and `HSA_AGENT_INFO_VENDOR_NAME` as
64-byte NUL-terminated UTF-8 values, and the NUMA and agent version fields come
from `HSA_AGENT_INFO_NODE`, `HSA_AGENT_INFO_VERSION_MAJOR`, and
`HSA_AGENT_INFO_VERSION_MINOR`.

The resulting `AgentDescription` contains the identity, device and profile,
raw features, HSA version, optional first wavefront size, queue capabilities,
optional AMD GPU properties, every ISA description, and every memory-pool
description. The associated `DiscoveredAgent` additionally retains the raw
agent and pool handles plus its borrow of `Runtime`.

The fields printed by the example therefore have this provenance:

| Printed field | Provenance |
| --- | --- |
| `identity.name`, `identity.vendor_name` | 64-byte `HSA_AGENT_INFO_NAME` and `HSA_AGENT_INFO_VENDOR_NAME` strings. |
| `identity.uuid` | 21-byte `AMD_AGENT_INFO_UUID`, parsed and checked against `device_type`. |
| `identity.numa_node_id` | `HSA_AGENT_INFO_NODE`. |
| `identity.driver_node_id`, `identity.pci_address` | GPU-only AMD driver-node, domain, and BDF queries. |
| `device_type`, `profile`, `feature_bits` | `HSA_AGENT_INFO_DEVICE`, `HSA_AGENT_INFO_PROFILE`, and `HSA_AGENT_INFO_FEATURE`. |
| `hsa_version_major`, `hsa_version_minor` | `HSA_AGENT_INFO_VERSION_MAJOR` and `HSA_AGENT_INFO_VERSION_MINOR`. |
| `first_isa_wavefront_size` | Conditional `HSA_AGENT_INFO_WAVEFRONT_SIZE` when kernel-dispatch bit 1 is set. |
| `queue` | Queue limit and kind attributes, or `None` when all three limits are zero. |
| `amd_gpu` | GPU-only AMD property bundle, or `None` for non-GPU agents. |
| `isas` | Every ISA handle from `hsa_agent_iterate_isas`, in traversal order. |
| `memory_pools` | Every pool handle from `hsa_amd_agent_iterate_memory_pools`, in traversal order. |

### 5. Printing

After all system and agent work succeeds, the example prints:

```text
system: SystemDescription {
    hsa_version_major: ...,
    hsa_version_minor: ...,
    amd_extension_version_major: ...,
    amd_extension_version_minor: ...,
    timestamp_frequency_hz: ...,
}
agent 0: AgentDescription {
    identity: AgentIdentity { ... },
    device_type: ...,
    profile: ...,
    feature_bits: ...,
    hsa_version_major: ...,
    hsa_version_minor: ...,
    first_isa_wavefront_size: ...,
    queue: ...,
    amd_gpu: ...,
    isas: [...],
    memory_pools: [...],
}
```

`{:#?}` is pretty `Debug`, so nested structs, vectors, enum variants, and
strings are expanded across multiple lines. The index is zero-based and is
assigned by the example's `enumerate`, not read from ROCr. One `agent` block is
printed for every successfully described raw agent. There is no summary count,
filter, sorting, or GPU-only selection.

On the live host used for this trace, the command completed successfully and
began with:

```text
system: SystemDescription {
    hsa_version_major: 1,
    hsa_version_minor: 18,
    amd_extension_version_major: 1,
    amd_extension_version_minor: 15,
    timestamp_frequency_hz: 1000000000,
}
```

It printed two agent records. Agent 0 was the CPU agent (`device_type: Cpu`,
`profile: Full`, no queue or ISA records) with four CPU global memory pools.
Agent 1 was an AMD GPU (`device_type: Gpu`, `profile: Base`) with feature bit
`1`, a `MultiProducer` queue range of 64 through 131072 packets, a `gfx1101`
ISA, a `gfx11-generic` ISA, AMD GPU properties for an AMD Radeon RX 7700 XT,
and three GPU or group memory pools. The exact counters and names are driver
state, not example constants.

### 6. Shutdown

Leaving the inner scope drops `Discovery` and ends all runtime borrows. The
example then consumes `Runtime` with `close`. `Runtime::close` checks its active
flag, marks it inactive before the FFI call, invokes `hsa_shut_down`, and routes
the status through `Api::check("hsa_shut_down", status)`. A non-success status
is an `Error::Hsa` with a numeric code and an optional status string. Because the
object is already marked inactive, `Drop` will not call `hsa_shut_down` a second
time if `close` returns an error. On success, the `Api` and its dynamic-library
handle are released exactly once at the end of `close`.

If any `?` before the explicit close returns, Rust first drops the live local
values and then runs `Runtime::drop`. The drop fallback calls `hsa_shut_down`
but cannot report its status. This is why the successful path uses the explicit
close and why the discovery borrow is scoped before it.

## Failure boundaries

The example has no recovery branch after discovery starts. The first failing
boundary returns immediately, and no partial `Discovery` is exposed:

| Boundary | Typed failure or behavior | What the user observes |
| --- | --- | --- |
| Non-64-bit build | Crate compile error | The example cannot be built. |
| `dlopen` of both default sonames | `LibraryOpen` | Process-level error with the attempted path and loader detail; no HSA call is made. |
| Interior NUL in an explicit `Runtime::open` path | `PathContainsNul` | Not reachable from this example's fixed sonames. |
| Any required `dlsym` | `MissingSymbol` | Process-level error naming the first missing symbol; default-soname fallback does not continue. |
| `hsa_init` | `Hsa` with operation `hsa_init` and no message | Process-level error; the failed `Api` unloads. |
| System query | `Hsa` annotated with its system field | No `system:` line is printed because discovery is incomplete. |
| Agent, ISA, or pool callback | `CallbackPanicked` for a caught callback panic, otherwise `Hsa` | No partial agent line is printed for the failing discovery call. |
| Typed metadata or identity | `InvalidAttribute`, `InvalidUtf8`, or `InvalidIdentity` | Process-level error naming the invalid field or identity. |
| Queue limits | `InvalidQueueSize` | Inconsistent limits are rejected, not clamped. |
| ISA name length or vector reservation | `InvalidAttribute` or `AllocationFailed` for `HSA ISA name`, `ISA descriptions`, `memory-pool descriptions`, or `memory-pool handles` | Discovery aborts before returning a record. |
| Optional pool extension attribute | `STATUS_ERROR_INVALID_ARGUMENT` becomes `None`; other statuses are `Hsa` | Missing optional metadata is visible as `None`, not a guessed value. |
| `println!` | Standard-output I/O panic | A Rust panic, rather than a boxed `recipe-hsa` error, terminates the process. |
| `hsa_shut_down` | `Hsa` from `Runtime::close` | All discovery output may already be present, followed by the process-level close error. |

Discovery-side `Api::check` asks `hsa_status_string` for rich text while the
runtime is initialized. If that status-string query fails or returns a null
pointer, the same typed HSA error remains valid with `message: None`. The only
intentional compatibility fallback in discovery is the two optional memory-pool
attributes; there is no alternate discovery implementation or synthetic
replacement for a failed required query.

The `Display` text surfaced by `main` follows the error variant: loader failures
read as `could not open shared library ...` or `shared library ... lacks required
symbol ...`, HSA statuses read as `<operation> failed with HSA status 0x...`
with an optional parenthesized ROCr message, and typed metadata failures identify
the field or identity value. `main` adds the standard `Error: ` prefix when the
returned boxed error is rendered by Rust's `Result` termination implementation.

## What this example proves, and what it does not

The run proves that the current process can load the reviewed ROCr ABI,
initialize it, enumerate the visible system, parse all returned agent, ISA, and
pool metadata, print the resulting descriptions, and shut the runtime down.
It does not prove queue creation, allocation, access grants, executable loading,
kernel dispatch, asynchronous copies, or completion handling. Those operations
belong to the separate execution API and the `execute_smoke` example.
Accordingly, a successful `discover` run is a live ROCr discovery diagnostic,
not a complete Recipe workload acceptance result.
