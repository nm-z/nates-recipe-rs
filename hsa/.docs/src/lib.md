# `recipe-hsa` crate facade

`recipe-hsa` is the reviewed ROCr/HSA boundary used by Recipe.  The crate
loads the ROCr shared library at runtime, records the exact topology exposed by
that runtime, and provides lifetime-scoped allocation, executable, signal, and
queue objects.  It does not create a process-global runtime, scheduler, or
operation library.  Submission is explicit and asynchronous: a copy or kernel
dispatch returns an owned completion token whose signal can also be used as a
device-side dependency.

This page documents the public facade in `hsa/src/lib.rs`.  The implementation
is split into private modules.  Their detailed contracts are documented in the
corresponding pages in this directory, while this page records the public names
and the ownership boundaries that connect them.

## Compilation and loading boundary

The crate is implemented only for the 64-bit large-model HSA ABI.  On another
pointer width, compilation stops with the crate-level `compile_error!` in
`src/lib.rs`; the `allow(dead_code)` attribute on non-64-bit targets does not
make the implementation available there.

ROCr is not linked at build time.  `Runtime::open` uses the private loader to
open a caller-selected path or soname and resolve the complete function-pointer
surface used by this crate.  `Runtime::open_default` tries, in order,
`libhsa-runtime64.so.1` and `libhsa-runtime64.so`.  A failed library open moves
to the next default candidate; a resolved-library error is returned immediately.
After all symbols are resolved, `hsa_init` is called once for that `Runtime`.
The `live-hsa` Cargo feature is required by the standalone `execute_smoke`
example; the library itself always uses the dynamic loader when opened.

## Private module graph

`src/lib.rs` declares every implementation module as private.  Consumers use
the root re-exports below, not paths such as `recipe_hsa::execution::Pending`.

| Module | Boundary owned by the module | Root names it contributes |
| --- | --- | --- |
| `abi` | Reviewed 64-bit ROCr/HSA constants, handle layouts, packet layouts, and C function-pointer types.  It contains no public Rust API. | None directly |
| `loader` | `dlopen`/`dlsym`, library lifetime, symbol conversion, and status conversion through the private `Api`. | None directly |
| `discovery` | Synchronous system, agent, ISA, queue-capability, and memory-pool enumeration.  It validates raw attributes before exposing owned descriptions and retains the raw agent/pool handles needed by later operations. | `SystemDescription`, `AgentIdentity`, `QueueCapabilities`, `AmdGpuProperties`, `IsaDescription`, `AllocationProperties`, `MemoryPoolDescription`, `AgentDescription`, `DiscoveredAgent`, `Discovery` |
| `identity` | Typed device, queue, memory, profile, UUID, ISA-target, PCI, feature, and rounding values.  Parsers retain the exact source spelling while rejecting malformed identities. | `DeviceType`, `Profile`, `QueueKind`, `MemorySegment`, `MemoryLocation`, `MemoryPoolFlags`, `RoundingModes`, `AgentUuidBody`, `AgentUuid`, `IsaFeature`, `IsaTarget`, `PciAddress` |
| `runtime` | One balanced `hsa_init`/`hsa_shut_down` reference and the root borrow lifetime for all child objects. | `Runtime` |
| `session` | GPU-only admission, queue creation and destruction, queue callback poisoning, queue configuration, and the session borrow carried by execution objects. | `Session`, `Queue`, `QueueConfig`, `QueueFault` |
| `execution` | Memory allocation and access grants, in-memory HSACO loading and symbol metadata, signal pooling and deferred retirement, asynchronous copies, AQL packet publication, dependency barriers, and completion-token state. | `Allocation`, `Executable`, `Kernel`, `KernelMetadata`, `DispatchGeometry`, `Dependency`, `Pending`, `PreparedPending`, `PollStatus`, `WaitOutcome`, `QueueProgress`, `RetirementReport`, `dependent_dispatch_packet_count` |
| `error` | The non-exhaustive error enum and the crate-wide `Result<T>` alias. | `Error`, `Result` |

The ABI and loader remain deliberately hidden.  No raw ROCr handle, function
pointer, packet, or `Api` field is part of the public root contract.

## Root re-export surface

The exact `pub use` surface in `src/lib.rs` is:

```text
recipe_hsa::discovery::{
    AgentDescription, AgentIdentity, AllocationProperties, AmdGpuProperties,
    DiscoveredAgent, Discovery, IsaDescription, MemoryPoolDescription,
    QueueCapabilities, SystemDescription,
}
recipe_hsa::{Error, Result}
recipe_hsa::execution::{
    Allocation, Dependency, DispatchGeometry, Executable, Kernel,
    KernelMetadata, Pending, PollStatus, PreparedPending, QueueProgress,
    RetirementReport, WaitOutcome, dependent_dispatch_packet_count,
}
recipe_hsa::identity::{
    AgentUuid, AgentUuidBody, DeviceType, IsaFeature, IsaTarget,
    MemoryLocation, MemoryPoolFlags, MemorySegment, PciAddress, Profile,
    QueueKind, RoundingModes,
}
recipe_hsa::Runtime
recipe_hsa::session::{Queue, QueueConfig, QueueFault, Session}
```

The braces above show source-module grouping only.  In Rust all of these names
are available at the crate root, for example `recipe_hsa::Runtime` and
`recipe_hsa::Queue`, and the private source modules are not addressable by a
consumer.

## Runtime root and lifetime hierarchy

`Runtime` is the sole public owner of the loaded ROCr `Api` and of the active
initialization reference.  It is intentionally not a singleton and is not
cloned.  Its public operations are:

| Method | Contract |
| --- | --- |
| `Runtime::open_default() -> Result<Runtime>` | Tries the two normal 64-bit ROCr sonames in order and initializes the first one that opens and accepts `hsa_init`. |
| `Runtime::open(path: impl AsRef<OsStr>) -> Result<Runtime>` | Opens the exact path or soname, resolves all required symbols, calls `hsa_init`, and returns one active runtime.  Interior NULs and loader or HSA errors are typed. |
| `Runtime::discover(&self) -> Result<Discovery<'_>>` | Requires an active runtime and performs a complete synchronous topology pass.  The returned `Discovery` borrows this runtime. |
| `Runtime::close(self) -> Result<()>` | Explicitly balances `hsa_init`.  The borrow checker prevents this consuming call while a discovery record, session, queue, allocation, executable, kernel, dependency, or pending token still carries a runtime borrow.  It reports the shutdown status. |

Dropping an active `Runtime` performs the same shutdown call but cannot report a
failure.  Calling `close` marks the object inactive before invoking
`hsa_shut_down`, so a second close is not possible.  Any operation reached
through a child first checks the active flag and returns `Error::RuntimeClosed`
if the runtime has already been consumed or otherwise marked inactive.

The lifetime hierarchy is therefore a compile-time ownership boundary, not a
global registry:

```text
Runtime
  | borrows 'runtime
  +-- Discovery<'runtime>
        +-- DiscoveredAgent<'runtime>
              +-- Session<'runtime>       (GPU admission only)
                    +-- Queue<'session, 'runtime>
                    +-- Allocation<'runtime>
                    +-- Executable<'session, 'runtime>
                          +-- Kernel<'session, 'runtime>
                    +-- Pending<'session, 'runtime>
                    +-- PreparedPending<'session, 'runtime>
                    +-- Dependency<'session, 'runtime>
```

The arrows mean borrowed lifetime requirements.  They do not imply that a
`Session` owns the `Runtime`; the runtime must outlive every child object.  A
`DiscoveredAgent` can also allocate and grant access without becoming a
session, which is how a CPU agent participates in host-visible memory copies.

## Discovery API

`Runtime::discover` reads system attributes, enumerates agents, then enumerates
each agent's ISAs and AMD memory pools.  It materializes owned Rust values, so
the public descriptions remain readable after the synchronous traversal has
returned.  Raw handles needed for allocation and later session creation stay
inside each `DiscoveredAgent`.

### `Discovery<'runtime>`

| Method | Result and ownership |
| --- | --- |
| `system(&self) -> &SystemDescription` | Returns the system version and timestamp-frequency record. |
| `agents(&self) -> &[DiscoveredAgent<'runtime>]` | Borrows all discovered agent records without changing ownership. |
| `into_agents(self) -> Vec<DiscoveredAgent<'runtime>>` | Consumes the discovery container and transfers its records to the caller. |

`SystemDescription` has public fields `hsa_version_major`,
`hsa_version_minor`, `amd_extension_version_major`,
`amd_extension_version_minor`, and `timestamp_frequency_hz`.

### `DiscoveredAgent<'runtime>`

`description(&self) -> &AgentDescription` is the read-only identity and
capability view.  The record itself retains the borrowed runtime and exact raw
agent and memory-pool handles, but it does not own a queue, allocation,
executable, or signal.

`AgentDescription` contains:

| Field | Meaning |
| --- | --- |
| `identity: AgentIdentity` | Names, vendor, parsed UUID, NUMA node, and optional AMD driver node and PCI address. |
| `device_type: DeviceType` | CPU, GPU, DSP, or AIE as reported by ROCr. |
| `profile: Profile` | Base or full HSA profile. |
| `feature_bits: u32` | Raw feature bits, including bits unknown to this crate. |
| `hsa_version_major`, `hsa_version_minor` | Agent HSA version. |
| `first_isa_wavefront_size: Option<u32>` | Deprecated first-ISA wavefront query when kernel dispatch is reported. |
| `queue: Option<QueueCapabilities>` | Validated queue count, packet bounds, and advertised queue kind, or `None` when all queue attributes are zero. |
| `amd_gpu: Option<AmdGpuProperties>` | AMD GPU-only properties, otherwise `None`. |
| `isas: Vec<IsaDescription>` | Every enumerated ISA, including exact target text and limits. |
| `memory_pools: Vec<MemoryPoolDescription>` | Every enumerated memory pool and optional allocation metadata. |

`supports_kernel_dispatch()` and `supports_agent_dispatch()` test the
corresponding raw feature bits without discarding unknown bits.

`AgentIdentity` exposes `name`, `vendor_name`, `uuid: AgentUuid`,
`numa_node_id`, `driver_node_id: Option<u32>`, and
`pci_address: Option<PciAddress>`.  AMD-only fields are absent (`None`) for
non-GPU agents.

`QueueCapabilities` exposes `maximum_queues`, `minimum_packets`,
`maximum_packets`, and `advertised_kind`.  Discovery rejects zero,
non-power-of-two, or inverted reported bounds before producing this value.

`AmdGpuProperties` exposes `chip_id`, `asic_revision`, `cacheline_bytes`,
`compute_unit_count`, `simds_per_compute_unit`,
`maximum_waves_per_compute_unit`, `maximum_clock_mhz`,
`maximum_memory_clock_mhz`, `product_name`, `available_memory_bytes`,
`timestamp_frequency_hz`, `sdma_engine_count`, `xgmi_sdma_engine_count`,
`xcc_count`, `maximum_scratch_bytes`, and `current_scratch_limit_bytes`.

`IsaDescription` exposes the exact `name`, optional parsed `amd_target`, model
and profile support booleans, `default_rounding_modes`,
`base_profile_rounding_modes`, `fast_f16`,
`maximum_workgroup_dimensions`, `maximum_workgroup_size`,
`maximum_grid_dimensions`, `maximum_grid_size`, and
`maximum_fbarriers_per_workgroup`.  GPU ISA names must parse as exact AMD HSA
target identities.  A non-GPU ISA has `amd_target == None`.

`MemoryPoolDescription` exposes `segment`, optional `location`, optional
`global_flags`, `size_bytes`, optional
`maximum_aggregate_allocation_bytes`, `accessible_by_all_agents`, and
optional `runtime_allocation: AllocationProperties`.  `location == None` or
`maximum_aggregate_allocation_bytes == None` records an extension attribute
that the runtime rejected; callers must preserve that absence rather than
guessing a value.  `AllocationProperties` contains `granule_bytes`,
`recommended_granule_bytes`, and `alignment_bytes`.

### Admission and agent-owned memory

`DiscoveredAgent::into_session(self) -> Result<Session<'runtime>>` consumes a
record and admits it only when all of the following are true:

1. The borrowed runtime is active.
2. The agent is `DeviceType::Gpu`.
3. The agent reports kernel-dispatch support.
4. Every discovered ISA has an exact AMD target and at least one ISA exists.

The method captures the current system timestamp frequency for the session's
signal waits and creates the session-local signal pool.  CPU, DSP, AIE, and
GPU records that fail these checks are not admitted as sessions: the consuming
call returns `Error::UnsupportedAgent` and never creates a partially realized
session.

All discovered agents, including CPU agents, expose these allocation methods:

| Method | Contract |
| --- | --- |
| `allocate(&self, pool_index: usize, size: usize) -> Result<Allocation<'runtime>>` | Allocates from the exact discovered pool index.  The index must exist, the pool must report runtime allocation, the size must be nonzero, and any reported maximum must contain it. |
| `allocate_coarse(&self, size: usize) -> Result<Allocation<'runtime>>` | Selects the first allocatable global pool containing `MemoryPoolFlags::COARSE_GRAINED`. |
| `allocate_fine(&self, size: usize) -> Result<Allocation<'runtime>>` | Selects the first allocatable global pool containing `FINE_GRAINED` or `EXTENDED_SCOPE_FINE_GRAINED`. |
| `allocate_kernarg(&self, size: usize) -> Result<Allocation<'runtime>>` | Selects the first allocatable global pool containing `KERNARG_INITIALIZATION`. |
| `grant_access(&self, allocation: &Allocation<'_>) -> Result<()>` | Replaces the allocation's direct access set with this agent plus the allocation's pool owner, as defined by ROCr. |

These methods do not infer a pool from a location or from a caller's intent.
They use the exact descriptions captured during discovery.

## Identity values

The identity re-exports are plain, cloneable value types.  They have no runtime
handle and therefore do not participate in the `Runtime` lifetime hierarchy.

| Type | Public shape and behavior |
| --- | --- |
| `DeviceType` | `Cpu`, `Gpu`, `Dsp`, `Aie`. |
| `Profile` | `Base`, `Full`. |
| `QueueKind` | `MultiProducer`, `SingleProducer`, `Cooperative`. |
| `MemorySegment` | `Global`, `ReadOnly`, `Private`, `Group`. |
| `MemoryLocation` | `Cpu`, `Gpu`. |
| `MemoryPoolFlags` | Opaque raw `u32` flags.  Constants are `KERNARG_INITIALIZATION`, `FINE_GRAINED`, `COARSE_GRAINED`, and `EXTENDED_SCOPE_FINE_GRAINED`; methods are `bits()`, `contains(flag)`, and `unknown_bits()`.  Unknown bits are retained. |
| `RoundingModes` | Public booleans `default`, `toward_zero`, and `nearest_even`. |
| `AgentUuidBody` | `Unavailable` for the `XX` body, or `Value([u8; 8])` for exactly 16 hexadecimal digits. |
| `AgentUuid` | `as_str()`, `device_type()`, and `body()`; implements `FromStr` and `Display`.  The accepted spelling is `CPU-...`, `GPU-...`, `DSP-...`, or `AIE-...` followed by `XX` or one 16-digit hexadecimal body. |
| `IsaFeature` | `name()` and `enabled()`.  Features preserve source order. |
| `IsaTarget` | `as_str()`, `architecture()`, and `features()`; implements `FromStr` and `Display`.  The parser requires `amdgcn-amd-amdhsa--gfx...` and rejects malformed or duplicate `name+`/`name-` modifiers. |
| `PciAddress` | Public `domain`, `bus`, `device`, and `function` fields; `Display` uses `dddd:bb:dd.f`. |

The raw-to-typed conversions used during discovery are private.  Consumers can
parse `AgentUuid` and `IsaTarget` from text, but cannot manufacture a raw ROCr
handle or bypass discovery validation through a public constructor.

## GPU session and queue ownership

`Session<'runtime>` is a realized ownership scope for one admitted AMD GPU
agent.  It stores the exact agent and pool handles from the consumed discovery
record, the immutable description, a permanent shared-fault record, and the
session-local signal pool.  Its public methods are:

| Method | Contract |
| --- | --- |
| `description(&self) -> &AgentDescription` | Returns the immutable discovery description used to validate this session. |
| `available_memory_bytes(&self) -> Result<u64>` | Queries ROCr's current AMD available-memory counter after checking runtime activity and session health. |
| `fault(&self) -> Option<QueueFault>` | Returns the newest asynchronous queue fault, if the queue callback has poisoned the session. |
| `ensure_healthy(&self) -> Result<()>` | Returns `Ok(())` until poison, then returns `Error::SessionPoisoned` for every subsequent logical operation. |
| `create_queue(&self, config: QueueConfig) -> Result<Queue<'_, '_>>` | Validates discovered bounds, power-of-two size, advertised queue kind, and kernel-dispatch capability, then creates one callback-owned queue. |
| `grant_access_exact_set(&self, allocation: &Allocation<'_>, sessions: &[&Session<'runtime>], agents: &[&DiscoveredAgent<'runtime>]) -> Result<()>` | Replaces the allocation's direct access set with a deduplicated exact set of same-runtime session and agent handles, while ROCr retains the pool owner. |
| `poll_retirements(&self) -> RetirementReport` | Performs one nonblocking pass over the explicit deferred-retirement set. |
| `drain_retirements(&self, timeout: Duration) -> Result<RetirementReport>` | Polls and waits in bounded chunks until the set drains, a timeout occurs, poison is observed, or a negative completion signal is found.  Unresolved entries remain owned for a later call. |
| `prepare_pending(&self, allocation_capacity, dependency_capacity) -> Result<PreparedPending<'_, 'runtime>>` | Realizes one completion signal and fixed keepalive-vector capacities before the live loop. |
| `copy_async_prepared(&self, destination, destination_offset, source, source_offset, size, pending) -> Result<()>` | Submits a direct AMD asynchronous copy using an already realized token.  It requires two allocation keepalive slots and no dependency slots. |
| `copy_async(&self, destination, destination_offset, source, source_offset, size) -> Result<Pending<'_, 'runtime>>` | Acquires a signal, retains both allocation owners, and submits one nonzero-byte asynchronous copy. |
| `load_hsaco(&self, hsaco: &[u8]) -> Result<Executable<'_, 'runtime>>` | Retains the in-memory HSACO, creates a code-object reader and executable using the session profile, loads it for this exact GPU agent, freezes it, and returns the executable. |
| `allocate`, `allocate_coarse`, `allocate_fine`, `allocate_kernarg`, `grant_access` | Session-owned spellings of the discovered-agent allocation/access methods, using this GPU's exact pools and handle. |

The session methods that establish execution lifetimes have these signatures:

```rust
pub fn prepare_pending<'session>(
    &'session self,
    allocation_capacity: usize,
    dependency_capacity: usize,
) -> Result<PreparedPending<'session, 'runtime>>;

pub fn copy_async<'session>(
    &'session self,
    destination: &Allocation<'runtime>,
    destination_offset: usize,
    source: &Allocation<'runtime>,
    source_offset: usize,
    size: usize,
) -> Result<Pending<'session, 'runtime>>;

pub fn copy_async_prepared(
    &self,
    destination: &Allocation<'runtime>,
    destination_offset: usize,
    source: &Allocation<'runtime>,
    source_offset: usize,
    size: usize,
    pending: &mut PreparedPending<'_, 'runtime>,
) -> Result<()>;

pub fn load_hsaco<'session>(
    &'session self,
    hsaco: &[u8],
) -> Result<Executable<'session, 'runtime>>;
```

`QueueConfig` is a copyable value with public fields `size_packets`, `kind`,
`private_segment_size`, and `group_segment_size`.  `QueueConfig::new(size,
kind)` sets both segment-size fields to `u32::MAX`, which asks ROCr for its
normal limits.  A requested queue size must be inside the discovered range and
power of two.  The requested producer discipline is retained even if ROCr's
read-only queue field reports the broader multi-producer protocol.

`QueueFault` contains `status: i32`, `source_queue_id: Option<u64>`, and an
ever-increasing `epoch: u64`.  The callback path records these fields without a
lock or allocation, wakes bounded waiters, and permanently poisons the owning
session.  Poison does not release live signals or allocations; pending
operations move to deferred retirement so the device can finish safely.

`Queue<'session, 'runtime>` is host-thread-confined.  Its public methods are:

| Method | Contract |
| --- | --- |
| `id() -> u64` | Reads the realized ROCr queue ID. |
| `size_packets() -> u32` | Reads the realized ring size. |
| `kind() -> QueueKind` | Returns the requested producer discipline enforced by this wrapper. |
| `session() -> &Session<'_>` | Returns the borrowed owning session. |
| `close(self) -> Result<()>` | Destroys the queue only when its `QueueCore` has no pending-token keepalive.  If a pending operation still retains the core, returns `Error::ResourceBusy`.  Drop attempts the same destruction but cannot report an error. |
| `dispatch(kernel, kernarg, geometry) -> Result<Pending<'session, 'runtime>>` | Convenience form of `dispatch_after` with no dependencies. |
| `dispatch_after(kernel, kernarg, geometry, dependencies) -> Result<Pending<'session, 'runtime>>` | Publishes a dependency barrier tree followed by a kernel packet.  Dependencies must belong to this session; the host does not wait for them. |
| `dispatch_prepared(kernel, kernarg, geometry, pending) -> Result<()>` | Dependency-free dispatch through a `PreparedPending` token realized before loop initialization.  It performs no signal acquisition or host collection growth. |
| `progress_capacity(required_packets, probe_budget: NonZeroU32) -> Result<QueueProgress>` | Performs at most the requested number of nonblocking occupancy probes without publishing or advancing the queue write index. |

All queue publication is single-producer safe only.  `dispatch`,
`dispatch_after`, and `dispatch_prepared` reject a queue whose configured kind
is not `QueueKind::SingleProducer`.  Queue ring slots are populated while the
header is invalid, then the body, valid header, write index, and doorbell are
published with release ordering.

The lifetime-bearing queue signatures are:

```rust
pub fn dispatch_prepared(
    &self,
    kernel: &Kernel<'session, 'runtime>,
    kernarg: Option<&Allocation<'runtime>>,
    geometry: DispatchGeometry,
    pending: &mut PreparedPending<'session, 'runtime>,
) -> Result<()>;

pub fn dispatch(
    &self,
    kernel: &Kernel<'session, 'runtime>,
    kernarg: Option<&Allocation<'runtime>>,
    geometry: DispatchGeometry,
) -> Result<Pending<'session, 'runtime>>;

pub fn dispatch_after(
    &self,
    kernel: &Kernel<'session, 'runtime>,
    kernarg: Option<&Allocation<'runtime>>,
    geometry: DispatchGeometry,
    dependencies: &[Dependency<'session, 'runtime>],
) -> Result<Pending<'session, 'runtime>>;
```

## Allocation and access ownership

`Allocation<'runtime>` owns one pointer returned by an exactly discovered ROCr
memory pool.  Its `Rc` inner owner also remembers the pool owner, selected pool
index, global flags, and the exact agents most recently granted direct access.
The public methods are:

| Method | Contract |
| --- | --- |
| `as_ptr() -> *mut c_void` | Returns the live device-visible pointer.  The allocation must still own its pointer. |
| `len() -> usize` | Returns the requested byte length. |
| `is_empty() -> bool` | Tests `len() == 0`; allocation itself rejects a zero-size request. |
| `pool_index() -> usize` | Returns the discovered pool index used for allocation. |
| `unsafe copy_from_host(offset, source) -> Result<()>` | Bounds-checks and copies host bytes into a host-accessible, coherent allocation.  The caller must ensure host-write accessibility and no concurrent device access. |
| `unsafe copy_to_host(offset, destination) -> Result<()>` | Bounds-checks and copies from a host-accessible allocation after a system-scope producing operation has completed. |
| `close(self) -> Result<()>` | Frees the pointer only when no pending token or other `Rc` owner retains it; otherwise returns `Error::ResourceBusy`. |

The copy methods are direct host operations, not asynchronous HSA copies.  The
session `copy_async` methods require both endpoint owners to have direct access
to both allocations and retain the endpoint allocations until the completion
signal is terminal.

## Executables, kernels, and dispatch geometry

`Session::load_hsaco` returns `Executable<'session, 'runtime>`, which retains an
`Arc<[u8]>` backing buffer, a code-object reader, a frozen executable handle,
and the exact GPU agent.  An empty byte slice is rejected.  `Executable::kernel
(&self, name: &str) -> Result<Kernel<'session, 'runtime>>` resolves a symbol
for that agent, rejects non-kernel symbols, reads all kernel metadata, and
rejects a zero or non-power-of-two kernarg alignment.  `Executable::close(self)`
requires unique ownership; a live `Kernel` or pending dispatch yields
`Error::ResourceBusy`.

`Kernel` has `name() -> &str` and `metadata() -> &KernelMetadata`.  It retains
the executable inner owner, so dropping the `Executable` value itself does not
invalidate a kernel that is still referenced.  `KernelMetadata` exposes
`kernarg_segment_size`, `kernarg_segment_alignment`, `group_segment_size`,
`private_segment_size`, and `dynamic_callstack`.

`DispatchGeometry` has public fields `dimensions: u8`, `grid: [u32; 3]`,
`workgroup: [u16; 3]`, `dynamic_group_bytes: u32`,
`dynamic_private_bytes: u32`, and `barrier: bool`.  The constructor
`one_dimensional(grid, workgroup)` sets dimensions to one, unused coordinates
to one, dynamic byte counts to zero, and `barrier` to false.  Dispatch accepts
only one, two, or three dimensions; active dimensions must be nonzero, unused
dimensions must equal one, workgroups may not exceed grids, products must not
overflow, and every exact discovered ISA limit must contain the request.

If a kernel has a nonzero kernarg segment, dispatch requires an allocation from
a discovered kernarg-capable pool, sufficient size, required alignment, and
queue-agent access.  A dynamic-callstack kernel requires a nonzero explicit
`dynamic_private_bytes`; static and dynamic group/private sizes are checked for
addition overflow before packet construction.

## Completion tokens and dependency ownership

The execution module has one session-local signal pool.  It recycles a signal
only after an acquire load observes zero, retains queue, executable, allocation,
and dependency resources through terminal completion, and moves incomplete
tokens into the explicit deferred-retirement set.  A negative signal is a
terminal asynchronous failure and poisons future signal acquisition.

### `Pending<'session, 'runtime>`

`Pending` is a `#[must_use]` owned token for one submitted copy or dispatch.
It is not a mere status value.  It owns the completion signal and every device
resource that the operation may still reference.

| Method | Contract |
| --- | --- |
| `dependency(&self) -> Dependency<'session, 'runtime>` | Clones a session-scoped signal record for later device-side dependency use without consuming the token. |
| `poll(&mut self) -> Result<PollStatus>` | Observes the signal without blocking.  Positive is `Pending`; zero is `Complete`; negative is `Error::AsyncSignal`.  A poisoned session also returns an error while a signal remains positive. |
| `wait(&mut self, timeout: Duration) -> Result<WaitOutcome>` | Repeatedly polls and uses bounded active-wait hints of at most one millisecond.  Returns `Complete` or `TimedOut`; ROCr timeout hints may overshoot slightly but no unbounded wait is requested. |

Dropping an incomplete token does not free its signal, queue, executable, or
allocations immediately.  It transfers them to the session retirement set.
Dropping a terminal token releases them in dependency-safe order.

`Dependency<'session, 'runtime>` is clonable and has `poll() ->
Result<PollStatus>`.  Every clone keeps the signal handle alive and prevents
recycling until all submitted consumers and clones have completed.  A
dependency can be supplied only to a queue and session that share its exact
signal pool.

`PollStatus` is `Pending | Complete`.  `WaitOutcome` is `Complete | TimedOut`.

### `PreparedPending<'session, 'runtime>`

`Session::prepare_pending(allocation_capacity, dependency_capacity)` creates a
token in the `Ready` state.  Signal creation and vector reservation occur before
the loop.  Submission mutates the token to `Active`, and only `Active` tokens
can be polled or borrowed as dependencies.  The states and public methods are:

| Method | Contract |
| --- | --- |
| `poll(&mut self) -> Result<PollStatus>` | Delegates to an active `Pending`; ready and consumed states return a typed error. |
| `reset(&mut self) -> Result<()>` | Re-arms a terminal active token with the same signal and keepalive storage.  A still-pending token returns `Error::ResourceBusy`; an asynchronous failure remains active and visible. |
| `dependency(&self) -> Result<Dependency<'session, 'runtime>>` | Borrows only an active token. |

`Queue::dispatch_prepared` requires a ready token with enough allocation
capacity for the optional kernarg (zero or one allocation) and no dependency
capacity.  `Session::copy_async_prepared` requires two allocation slots.  Both
methods validate the token's signal pool against the session before publishing.
After successful submission the token is active.  Dropping a ready token
returns its signal to the pool; dropping an active token follows `Pending`
retirement; dropping a consumed token does nothing.

### Deferred retirement

`RetirementReport` is the result of one explicit retirement pass.  Its fields
are `reclaimed`, `pending`, `failed_signal: Option<i64>`, and `poisoned`.
`is_drained()` is true exactly when `pending == 0`.  `Session::poll_retirements`
never sleeps.  `Session::drain_retirements` preserves unresolved records after a
timeout or poison and reports `Error::DeferredRetirement`; a drained negative
signal returns `Error::AsyncSignal`.

## Queue capacity and dependency lowering

`QueueProgress` reports a bounded capacity probe:

```text
Ready { available_packets: u32, probes: u32 }
Backpressured {
    available_packets: u32,
    required_packets: u32,
    probes: u32,
}
```

`required_packets` must be nonzero and no larger than the realized queue size.
The probe budget is a `NonZeroU32`; no call sleeps, blocks, publishes a packet,
or advances the write index.

`dependent_dispatch_packet_count(dependency_count: usize) -> Result<u32>`
returns one kernel packet plus the number of barrier-AND packets needed by the
deterministic reduction tree.  Each barrier has fan-in five.  Zero dependencies
therefore require one packet.  Very large counts that overflow the tree or a
`u32` result return `Error::InvalidDispatch`.

`Queue::dispatch_after` lowers dependencies into that five-input tree, retains
the input and intermediate signal records in the pending keepalive, then
publishes the terminal barrier and kernel packets in order.  The host does not
wait for dependencies.  If the required tree is larger than the realized ring,
the call returns `Error::InvalidProgressRequest`; if the ring is large enough
but currently occupied, the capacity check occurs before any packet is
published and returns `Error::QueueFull` with the observed read and write
indices.

## Error and failure boundary

All operations that cross the loader, ROCr, allocation, queue, executable,
signal, or dispatch boundary return the root `Result<T>` alias.  `Error` is
`#[non_exhaustive]`; callers must retain a wildcard arm when matching it.
Important groups are:

| Group | Variants |
| --- | --- |
| Loader and ABI | `PathContainsNul`, `LibraryOpen`, `MissingSymbol`, `Hsa`, `InvalidUtf8`, `InvalidAttribute`, `CallbackPanicked`, `AllocationFailed` |
| Identity and admission | `InvalidIdentity`, `UnsupportedAgent`, `RuntimeClosed` |
| Queue creation and publication | `InvalidQueueSize`, `UnsupportedQueueKind`, `NullQueue`, `InvalidQueueReturned`, `QueueFull`, `InvalidProgressRequest` |
| Memory | `InvalidMemoryPoolIndex`, `MemoryPoolNotAllocatable`, `NoMatchingMemoryPool`, `InvalidAllocationSize`, `NullAllocation`, `CopyOutOfBounds`, `ResourceBusy` |
| Code objects and dispatch | `EmptyCodeObject`, `NameContainsNul`, `SymbolNotKernel`, `InvalidKernel`, `InvalidDispatch` |
| Asynchronous completion | `NullSignal`, `AsyncSignal`, `SessionPoisoned`, `DeferredRetirement` |

`Hsa` carries the operation name, numeric status, and an optional runtime-owned
status message.  Realized loop submission uses the allocation-free numeric
status path, while discovery and realization retain richer diagnostics.
`SessionPoisoned` is permanent for that session and includes the callback
status, optional source queue ID, and fault epoch.  `DeferredRetirement` means
the crate deliberately retained unresolved device resources, not that they
were freed or replaced by a fallback object.

## End-to-end ownership sequence

The supported public sequence is:

1. Open one `Runtime` with `open_default` or `open`.
2. Call `discover`, inspect exact descriptions, and move records out with
   `into_agents` or borrow them through `agents`.
3. Keep CPU or other host agents as `DiscoveredAgent` values when their pools
   are needed for host-visible allocations.  Consume one qualifying GPU record
   with `into_session`.
4. Allocate from exact discovered pools, then replace access sets with
   `grant_access` or `grant_access_exact_set` before an asynchronous copy or
   dispatch.
5. Optionally load and resolve an in-memory HSACO, create a validated
   single-producer queue, and choose ordinary or pre-realized completion
   tokens.
6. Submit copies with `copy_async` or `copy_async_prepared`, and kernels with
   `dispatch`, `dispatch_after`, or `dispatch_prepared`.  Poll or bounded-wait
   each token, borrow dependencies only while the signal remains owned, and
   explicitly poll or drain deferred retirements.
7. Close executable, allocations, and queues when no pending keepalive retains
   them.  Drop the session and discovery records, then call `Runtime::close`.

The reverse order is not an alternate implementation.  It is rejected by
lifetimes or by `Error::ResourceBusy` so that ROCr objects are never destroyed
while a submitted operation can still reference them.
