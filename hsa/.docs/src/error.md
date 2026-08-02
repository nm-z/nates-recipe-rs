# `hsa::Error` contract

This document is the source-level contract for `hsa/src/error.rs`. It records
the typed failure values, their exact rendering, every construction and
transformation currently present in the HSA crate, and the runtime consequence
of returning each value. Source references use the line numbers in the current
tree.

## Parseable intent and structure

```toml
[module]
path = "hsa/src/error.rs"
kind = "public-non-exhaustive-error-contract"
intent = "Represent every loader, ROCr status, identity, capability, ownership, dispatch, signal, and retirement failure without guessing or hiding the cause."
purpose = "Provide the single Error and Result boundary re-exported by recipe-hsa, with exact fields, Display rendering, and runtime consequence."
result_alias = "std::result::Result<T, Error>"
error_enum = "Error"
non_exhaustive = true
traits = ["Clone", "Debug", "Eq", "PartialEq", "std::error::Error"]
rendering = "std::fmt::Display"
status_type = "i32"
status_success = 0
status_error = 0x1000
status_error_invalid_argument = 0x1001
variant_count = 33
failure_policy = "return the typed value; no implicit retry, fallback, substitute state, or alternate implementation"
```

`Result<T>` is the crate-wide public alias for `std::result::Result<T, Error>`
(`error.rs:1-4`). `Error` is `#[non_exhaustive]`, so code outside the crate
must retain a wildcard arm when matching it. There is no source-error chain:
the `std::error::Error` implementation is empty. All context that callers can
render is carried by the variant fields below. The enum derives value
comparison, so the fields are also the equality identity.

The enum has 33 variants. No variant is a status-code alias: HSA statuses are
kept as raw `i32` values in `Hsa` and `SessionPoisoned`, while local validation
and lifecycle failures use the other variants.

`lib.rs` publicly re-exports both `Error` and `Result` (`lib.rs:21-35`). The
error type is therefore the single failure boundary for runtime loading,
topology discovery, session ownership, memory allocation, executable loading,
queue publication, and explicit asynchronous completion. It does not own
policy such as retries or fallback implementations; those decisions remain at
the calling API boundary.

## Exact fields and `Display` output

The following table is intentionally literal. A template is the exact string
written by `fmt::Display`; `:?` and `:#x` have their Rust formatter meanings.

| Variant | Fields | Exact display template |
| --- | --- | --- |
| `PathContainsNul` | `path: String` | `shared-library path contains NUL: {path:?}` |
| `NameContainsNul` | `field: &'static str` | `{field} contains an interior NUL` |
| `LibraryOpen` | `path: String`, `detail: String` | `could not open shared library {path:?}: {detail}` |
| `MissingSymbol` | `library: String`, `symbol: &'static str`, `detail: String` | `shared library {library:?} lacks required symbol {symbol}: {detail}` |
| `Hsa` | `operation: &'static str`, `status: i32`, `message: Option<String>` | `{operation} failed with HSA status {status:#x}`; append ` ({message})` only when `message` is `Some` |
| `InvalidUtf8` | `field: &'static str` | `{field} returned by ROCr is not valid UTF-8` |
| `InvalidIdentity` | `kind: &'static str`, `value: String`, `reason: &'static str` | `invalid {kind} identity {value:?}: {reason}` |
| `InvalidAttribute` | `field: &'static str`, `value: u64` | `invalid ROCr value {value:#x} for {field}` |
| `CallbackPanicked` | `operation: &'static str` | `{operation} callback panicked` |
| `AllocationFailed` | `field: &'static str`, `requested: usize` | `could not reserve {requested} bytes while reading {field}` |
| `RuntimeClosed` | none | `the HSA runtime is already closed` |
| `UnsupportedAgent` | `reason: &'static str` | `agent cannot back a Recipe GPU session: {reason}` |
| `InvalidQueueSize` | `requested: u32`, `minimum: u32`, `maximum: u32`, `reason: &'static str` | `queue size {requested} is invalid for [{minimum}, {maximum}]: {reason}` |
| `UnsupportedQueueKind` | `requested: crate::QueueKind`, `advertised: crate::QueueKind` | `queue kind {requested:?} is incompatible with advertised kind {advertised:?}` |
| `NullQueue` | none | `ROCr reported success but returned a null queue` |
| `InvalidQueueReturned` | `kind: u32`, `features: u32`, `size: u32`, `base_is_null: bool` | `ROCr returned invalid queue fields: kind={kind}, features={features:#x}, size={size}, base_is_null={base_is_null}` |
| `EmptyCodeObject` | none | `HSACO code object is empty` |
| `InvalidMemoryPoolIndex` | `index: usize`, `pool_count: usize` | `memory-pool index {index} is outside the discovered pool count {pool_count}` |
| `MemoryPoolNotAllocatable` | `index: usize` | `memory pool {index} is not runtime allocatable` |
| `NoMatchingMemoryPool` | `kind: &'static str` | `no allocatable {kind} memory pool was discovered` |
| `InvalidAllocationSize` | `requested: usize`, `maximum: Option<usize>` | `allocation size {requested} is invalid for maximum {maximum:?}` |
| `NullAllocation` | none | `ROCr reported success but returned a null allocation` |
| `ResourceBusy` | `resource: &'static str` | `{resource} still has live dependent resources` |
| `SymbolNotKernel` | `name: String`, `kind: i32` | `executable symbol {name:?} has non-kernel kind {kind}` |
| `InvalidKernel` | `reason: &'static str` | `invalid HSA kernel: {reason}` |
| `InvalidDispatch` | `reason: &'static str` | `invalid HSA dispatch: {reason}` |
| `QueueFull` | `write_index: u64`, `read_index: u64`, `size: u32` | `HSA queue is full at write index {write_index}, read index {read_index}, size {size}` |
| `InvalidProgressRequest` | `required_packets: u32`, `queue_size: u32` | `capacity request for {required_packets} packets is invalid for queue size {queue_size}` |
| `CopyOutOfBounds` | `buffer: &'static str`, `offset: usize`, `size: usize`, `capacity: usize` | `{buffer} copy range offset {offset} plus size {size} exceeds capacity {capacity}` |
| `NullSignal` | none | `ROCr returned an invalid zero signal handle` |
| `AsyncSignal` | `value: i64` | `HSA completion signal reported asynchronous failure {value}` |
| `SessionPoisoned` | `status: i32`, `source_queue_id: Option<u64>`, `epoch: u64` | `HSA session is poisoned by async status {status:#x} from queue {source_queue_id:?} at epoch {epoch}` |
| `DeferredRetirement` | `pending: usize`, `poisoned: bool` | `{pending} deferred HSA operation(s) remain unresolved (session_poisoned={poisoned}); their signals and referenced resources were retained` |

`Hsa` writes the operation/status portion first and appends the parenthesized
message only when `message` is present (`error.rs:150-160`). Every other
variant has one unconditional format arm (`error.rs:128-314`).

## Status conversion rules

The ABI defines `HsaStatus = i32`, `STATUS_SUCCESS = 0`,
`STATUS_ERROR = 0x1000`, and `STATUS_ERROR_INVALID_ARGUMENT = 0x1001`
(`abi.rs:9-15`). The conversion rules are deliberately narrow:

1. `Api::check(operation, status)` returns `Ok(())` only for
   `STATUS_SUCCESS`; every other value becomes `Error::Hsa` with the raw
   status and `Api::status_message(status)` (`loader.rs:272-281`). The message
   query calls `hsa_status_string` while the runtime is initialized. A failed
   query or null returned pointer produces `message: None`; otherwise the
   runtime-owned NUL-terminated text is copied into `Some(String)`
   (`loader.rs:299-312`).
2. `Api::check_status_only` has the same success boundary but never queries or
   allocates a message, so failures always use `message: None`
   (`loader.rs:283-297`). The live prepared async-copy path is the current
   caller (`execution.rs:1549-1568`).
3. Discovery query helpers call `Api::check` and then
   `annotate_operation`. That helper reconstructs only `Error::Hsa`, replacing
   the generic ABI operation with the field-specific label while preserving
   `status` and `message`; all other errors pass through unchanged
   (`discovery.rs:653-705`, `discovery.rs:726-739`).
4. `pool_info_optional` treats exactly `0x1001` as an unsupported optional
   attribute and returns `Ok(None)`. Other non-success statuses become
   `Error::Hsa` through the normal annotated path (`discovery.rs:707-724`).
5. `Runtime::open` handles `hsa_init` directly. A failed initialization cannot
   safely call `hsa_status_string`, so it creates `Hsa { operation: "hsa_init",
   message: None }` (`runtime.rs:35-49`).
6. Queue, allocation, executable, and teardown calls use `Api::check` unless
   explicitly noted below. Their raw statuses therefore retain optional ROCr
   text. The queue error callback and completion signals do not pass through
   `Api::check`: they produce `SessionPoisoned` and `AsyncSignal` respectively.

The operation labels reaching `Hsa` are:

- `hsa_init`
- `hsa_shut_down`
- `hsa_system_get_info`
- `hsa_system_get_info(TIMESTAMP_FREQUENCY)`
- `hsa_agent_get_info`
- `hsa_agent_get_info(AMD_MEMORY_AVAIL)`
- `hsa_isa_get_info_alt`
- `hsa_isa_get_info_alt(NAME)`
- `hsa_amd_memory_pool_get_info`
- `hsa_iterate_agents`
- `hsa_agent_iterate_isas`
- `hsa_amd_agent_iterate_memory_pools`
- `hsa_queue_create`
- `hsa_queue_destroy`
- `hsa_signal_create`
- `hsa_amd_memory_pool_allocate`
- `hsa_amd_memory_pool_free`
- `hsa_amd_agents_allow_access`
- `hsa_code_object_reader_create_from_memory`
- `hsa_code_object_reader_destroy`
- `hsa_executable_create_alt`
- `hsa_executable_destroy`
- `hsa_executable_load_agent_code_object`
- `hsa_executable_freeze`
- `hsa_executable_get_symbol_by_name`
- `hsa_amd_memory_async_copy`

`executable_symbol_info` may replace the ABI call name with these exact
metadata labels before constructing `Hsa`:

```text
HSA executable symbol type
HSA kernel object
HSA kernel kernarg size
HSA kernel kernarg alignment
HSA kernel group segment size
HSA kernel private segment size
HSA kernel dynamic callstack
```

Discovery replaces the generic labels with the exact field labels supplied to
`system_info`, `agent_info`, `isa_info`, and `pool_info`. Those labels cover
system versions and timestamp, every HSA and AMD agent property, queue limits
and kind, UUID and identity fields, ISA name and limits, memory-pool segment,
allocation, location, flags, size, accessibility and maximum, and all GPU
properties (`discovery.rs:154-650`).

The field-specific operation labels supplied to those helpers are:

```text
HSA system major version
HSA system minor version
AMD HSA extension major version
AMD HSA extension minor version
HSA timestamp frequency
HSA agent device type
HSA agent features
HSA agent profile
AMD agent UUID
HSA minimum queue size
HSA maximum queue size
HSA maximum queue count
HSA queue type
AMD driver node ID
AMD PCI domain
AMD PCI BDF ID
AMD chip ID
AMD ASIC revision
AMD cacheline size
AMD compute-unit count
AMD SIMDs per compute unit
AMD maximum waves per compute unit
AMD maximum GPU clock
AMD maximum memory clock
AMD product name
AMD available memory
AMD timestamp frequency
AMD SDMA engine count
AMD XGMI SDMA engine count
AMD XCC count
AMD maximum scratch limit
AMD current scratch limit
HSA first-ISA wavefront size
HSA agent name
HSA agent vendor
HSA NUMA node ID
HSA agent major version
HSA agent minor version
HSA ISA name length
HSA ISA machine models
HSA ISA profiles
HSA ISA maximum grid dimensions
HSA ISA small machine model
HSA ISA large machine model
HSA ISA base profile
HSA ISA full profile
HSA ISA default rounding modes
HSA ISA base-profile rounding modes
HSA ISA fast f16
HSA ISA maximum workgroup dimensions
HSA ISA maximum workgroup size
HSA ISA maximum grid size
HSA ISA maximum fbarriers
AMD memory-pool segment
AMD memory-pool runtime allocation
AMD memory-pool allocation granule
AMD memory-pool recommended allocation granule
AMD memory-pool allocation alignment
AMD memory-pool location
AMD memory-pool global flags
AMD memory-pool size
AMD memory-pool maximum allocation
AMD memory-pool all-agent accessibility
```

## Construction and propagation map

Every `Result`-returning public operation uses the alias above. A `?` or an
explicit `return Err` preserves the exact variant and fields unless a mapping
listed here intentionally replaces context. Cleanup in `Drop` implementations
cannot return an error; those paths discard `Hsa` results after making the
underlying resource safe to drop.

### Loader and runtime

| Variant | Construction | Propagation and runtime consequence |
| --- | --- | --- |
| `PathContainsNul` | `Library::open` maps `CString::new(path.as_bytes())` failure and stores the lossy display path (`loader.rs:27-34`). | No `dlopen` occurs. `Runtime::open` propagates it. `open_default` only supplies static NUL-free sonames, so it cannot produce this value. |
| `LibraryOpen` | A null `dlopen` handle captures the thread-local `dlerror`, or the literal fallback `dynamic loader returned a null handle` (`loader.rs:38-55`). | `Api::load` and `Runtime::open` stop before initialization. `Runtime::open_default` retries the second soname only for this variant, then returns the last open error if both candidates fail (`runtime.rs:22-32`). |
| `MissingSymbol` | `Library::symbol` returns it when `dlerror` reports a lookup failure or when a symbol address is null despite no loader diagnostic (`loader.rs:58-88`). `Api::load` resolves each required ROCr function in order (`loader.rs:169-269`). | The first missing symbol aborts loading. The library handle is dropped, so no partially usable API escapes. |
| `Hsa` | `Runtime::open` constructs the pre-init `hsa_init` case. `Api::check` and `check_status_only` construct all post-load ROCr call failures. | The operation fails at the caller boundary with raw status and optional text. `Runtime::close` sets `active = false` before checking shutdown, so a shutdown error still leaves the object closed (`runtime.rs:61-68`). `Drop` invokes shutdown but discards its status (`runtime.rs:79-89`). |
| `RuntimeClosed` | `Runtime::ensure_active` returns it after explicit close (`runtime.rs:70-76`). Every public operation that first checks the runtime propagates it. | No ROCr call is attempted. This is a local lifetime guard, not an HSA status. |

### Identity normalization and discovery

| Variant | Construction | Propagation and runtime consequence |
| --- | --- | --- |
| `InvalidAttribute` | Raw enum and boolean decoders reject unknown values: `DeviceType`, `Profile`, `QueueKind`, `MemorySegment`, `MemoryLocation`, and `c_bool` (`identity.rs:13-187`). Discovery also rejects an ISA name length above 4096, non-zero data after the first NUL, an agent string with no NUL, and the same malformed metadata through `agent_string` (`discovery.rs:459-499`, `741-757`). | The current discovery record is rejected rather than guessed. The error retains the HSA field label and raw value; all enclosing discovery/session calls propagate it. |
| `InvalidIdentity` | `AgentUuid::from_str` rejects missing separator, extra separator, unknown device prefix, and an invalid body (`identity.rs:210-264`). `IsaTarget::from_str` rejects missing AMD prefix, invalid architecture, malformed feature modifiers, and duplicate feature names (`identity.rs:309-377`). Discovery separately rejects a UUID device prefix that disagrees with `HSA_AGENT_INFO_DEVICE` (`discovery.rs:204-211`). | Agent or ISA discovery aborts before an inexact identity can become a session. The original input is retained in `value`. |
| `InvalidUtf8` | `discover_isa` maps invalid ISA bytes, and `agent_string` maps invalid agent/product/vendor strings (`discovery.rs:495-499`, `741-757`). | The affected discovery operation fails. No replacement or lossy identity is admitted. |
| `CallbackPanicked` | Each synchronous ROCr iterator records a panic in its collector, then returns this value before examining the final ABI status: `hsa_iterate_agents`, `hsa_agent_iterate_isas`, and `hsa_amd_agent_iterate_memory_pools` (`discovery.rs:764-800`, `802-842`, `844-884`). | The panic is contained at the C boundary, collection stops, and discovery fails with the operation name. A callback panic takes precedence over a generic `Hsa` status from the same call. A null callback data pointer returns ABI `STATUS_ERROR` and is handled as `Hsa` because the collector panic flag remains false. |
| `AllocationFailed` | Discovery reserves vectors for agent, ISA, memory-pool descriptions and handles, and ISA name bytes (`discovery.rs:176-183`, `356-386`, `468-474`). | Discovery aborts before returning a partial topology. The `requested` value is the attempted element count or byte length, despite the display wording `bytes`. |
| `InvalidQueueSize` | Discovery rejects inconsistent advertised limits: nonzero queue count and power-of-two min/max with `min <= max` are required (`discovery.rs:440-457`). | The agent has no valid queue capability and cannot proceed to a session. |

The `Hsa` values produced during discovery come from every
`system_info`/`agent_info`/`isa_info`/`pool_info` call. `pool_info_optional`
is the one deliberate status exception: `STATUS_ERROR_INVALID_ARGUMENT` means
the runtime does not expose that optional extension attribute and becomes
`None`, not an error.

### Session and queue ownership

| Variant | Construction | Propagation and runtime consequence |
| --- | --- | --- |
| `SessionPoisoned` | `SharedFault::record` stores the latest queue callback status, optional source queue ID, and an incremented epoch; `SharedFault::check` turns a snapshot into this value (`session.rs:29-91`). The callback is registered by `Session::create_queue` (`session.rs:94-112`). | Poisoning is permanent. `Session::ensure_healthy`, allocation/queue/dispatch entry points, dependency polling, pending completion checks, and retirement reports observe it. A pending operation remains nonterminal when a callback poisons the session, so its signal and keepalive resources are deferred rather than released. |
| `UnsupportedAgent` | `DiscoveredAgent::into_session` rejects non-GPU agents, GPUs without kernel-dispatch support, and GPUs with no exact AMD ISA. `Session::create_queue` also rejects a GPU with no user-mode queue capability (`session.rs:145-169`, `230-235`). | No session or queue is created. The `reason` identifies the missing capability and is rendered verbatim. |
| `InvalidQueueSize` | `Session::create_queue` rejects a requested size outside discovered bounds or not a power of two (`session.rs:230-246`). | Queue creation is not called. The discovered min/max are reported in the value. |
| `UnsupportedQueueKind` | `Session::create_queue` checks the requested producer discipline against the advertised kind. A `MultiProducer` advertisement permits multi or single producer; single and cooperative advertisements require an exact match (`session.rs:248-263`). | No queue is created. |
| `NullQueue` | After `hsa_queue_create` reports success, the returned pointer is checked for null (`session.rs:274-289`). | The queue handle is unusable, so creation fails before reading fields. |
| `InvalidQueueReturned` | A successful queue is rejected if its base address is null, size is zero or non-power-of-two, kernel-dispatch support is absent, or the raw kind is unknown (`session.rs:290-319`). The newly created queue is destroyed first. | The value reports all observed fields. If cleanup fails, callback storage is leaked intentionally and that cleanup status is not substituted for the primary invalid-return error. |
| `ResourceBusy` | `Queue::close` returns it when `Rc::try_unwrap` finds pending-token owners (`session.rs:397-407`). | The explicit close fails without destroying the queue. Dropping the returned `Rc` still follows normal deferred ownership. Queue `Drop` later attempts destruction and discards any status. |
| `Hsa` | Session timestamp and available-memory queries, queue create, and queue destroy use `Api::check` (`session.rs:171-220`, `274-286`, `346-364`). | The call fails with the corresponding operation/status. Explicit queue close returns the teardown `Hsa`; queue drop ignores it. |

### Allocation, memory access, and code objects

| Variant | Construction | Propagation and runtime consequence |
| --- | --- | --- |
| `CopyOutOfBounds` | `check_range` rejects overflow or an end beyond capacity for host copies and async copies (`execution.rs:512-523`). `buffer` is `destination` or `source`. | No pointer arithmetic or device submission occurs. The exact offset, requested size, and capacity remain available to the caller. |
| `InvalidMemoryPoolIndex` | `allocate_from` independently indexes raw pool handles and descriptions, reporting the corresponding vector length (`execution.rs:525-545`). | Allocation stops before touching ROCr. This catches mismatched or stale discovery indices. |
| `MemoryPoolNotAllocatable` | `allocate_from` rejects a discovered pool whose `runtime_allocation` is `None` (`execution.rs:546-548`). | No allocation call occurs. |
| `NoMatchingMemoryPool` | `matching_pool` is used by coarse, fine, and kernarg allocation helpers when no global, runtime-allocatable pool has the required flags (`execution.rs:580-655`, `667-699`). | The helper fails before selecting an index, with `kind` equal to `coarse-grained`, `fine-grained`, or `kernarg`. |
| `InvalidAllocationSize` | `allocate_from` rejects zero and sizes above the optional discovered maximum (`execution.rs:549-558`). | No allocation call occurs. `maximum` remains `None` when ROCr advertised no maximum. |
| `NullAllocation` | A successful `hsa_amd_memory_pool_allocate` with a null output pointer is rejected (`execution.rs:560-577`). | The typed allocation is not returned. |
| `ResourceBusy` | `Allocation::close` returns `HSA memory allocation` when clones remain (`execution.rs:499-508`). | The explicit close cannot free the pointer; `AllocationInner::Drop` later attempts the free and ignores its status. |
| `InvalidDispatch` | Access-grant validation rejects an agent count above the public ABI range, an empty set, or objects from another runtime (`execution.rs:595-620`, `710-739`). | The access set is not submitted. Any preceding allocation remains owned by the caller. |
| `Hsa` | Memory allocate/free and `hsa_amd_agents_allow_access` use `Api::check` (`execution.rs:560-620`). | Explicit operations return the status error. Allocation drop/free errors are intentionally discarded. |
| `EmptyCodeObject` | `Session::load_hsaco` rejects an empty byte slice before creating a reader (`execution.rs:974-980`). | No executable or reader is created. |
| `NameContainsNul` | `Executable::kernel` maps `CString::new(name)` failure (`execution.rs:845-851`). | Symbol lookup is not attempted. |
| `SymbolNotKernel` | A looked-up executable symbol whose type differs from `SYMBOL_KIND_KERNEL` is rejected (`execution.rs:872-883`). | Kernel metadata is not queried and no `Kernel` value is returned. |
| `InvalidKernel` | `Executable::kernel` rejects a zero kernel object and a zero or non-power-of-two kernarg alignment (`execution.rs:885-935`). | The executable remains valid, but this symbol cannot be dispatched. |
| `ResourceBusy` | `Executable::close` returns `HSA executable` when a `Kernel` or other clone remains (`execution.rs:946-955`). | Explicit executable destruction is deferred. `ExecutableInner::Drop` attempts destruction and discards any status. |
| `InvalidAttribute` | Kernel metadata's dynamic-callstack flag is decoded through `c_bool`; malformed ROCr data therefore uses the identity decoder's field/value form (`execution.rs:921-929`). | Kernel creation fails before a dispatch object is returned. |
| `Hsa` | Code-object reader creation/destruction, executable creation/load/freeze, symbol lookup, and symbol metadata queries all use `Api::check` (`execution.rs:959-1053`). | Partial executable cleanup is attempted on creation/load/freeze failure. If executable destruction fails, reader backing is leaked; if reader destruction fails, HSACO backing is forgotten. The original `Hsa` remains the returned value. |

### Signals, pending operations, and retirement

| Variant | Construction | Propagation and runtime consequence |
| --- | --- | --- |
| `AllocationFailed` | `SignalPool::acquire` and `rearm` reject retirement-count arithmetic overflow (`requested: usize::MAX`) or failed vector reservation (`field: deferred HSA retirement set`). `Session::prepare_pending` reports failed exact reservations for allocation/dependency keepalive slots (`execution.rs:187-209`, `247-268`, `1460-1498`). | A signal reservation is released when setup fails. No operation is submitted. |
| `AsyncSignal` | A negative completion signal is reported by `Dependency::poll`, `Pending::poll`, or retirement draining. `SignalPool::acquire`/`rearm` also report the first fatal negative signal retained by the pool (`execution.rs:187-192`, `247-251`, `1133-1154`, `1188-1228`, `749-757`). | The completion failure is terminal for that signal. `Pending::poll` stores `Err(AsyncSignal)` as its terminal result, releases keepalives, and the pool remembers the first negative value so later submissions fail. `drain_retirements` returns it only after all retired records have been reclaimed. |
| `NullSignal` | `SignalPool::acquire` rejects a successful `hsa_signal_create` that returns a zero handle (`execution.rs:215-230`). | The reservation is released and no signal record escapes. |
| `SessionPoisoned` | `Dependency::poll` and `Pending::poll` call `SharedFault::check` while a signal is pending or complete. Submission methods check health after enqueue and retire the operation if a queue callback raced with publication (`execution.rs:1143-1154`, `1188-1229`, `2078-2087`, `2280-2296`). | The operation remains retained when its device completion is uncertain. Callers must resolve the fault and drain retirement resources; no later session operation is admitted. |
| `DeferredRetirement` | `Session::drain_retirements` returns it when unresolved records remain at timeout or when the session is already poisoned (`execution.rs:741-769`). | The pending signals, referenced allocations, queues, executables, and dependencies remain retained. A later drain can continue ordered teardown. Dropping the signal pool also retains unresolved records and emits a root stderr leak diagnostic. |
| `ResourceBusy` | Prepared-token state transitions return it for a non-ready token, a consumed token, a nonterminal reset, or missing completed signal/keepalive (`execution.rs:1318-1347`, `1370-1427`). | The state is restored where documented, so callers can retry after satisfying the state precondition. |
| `InvalidDispatch` | Prepared-token capacity/state checks use it for insufficient keepalive capacity, polling an unsubmitted token, or requesting a dependency from a non-active token (`execution.rs:1335-1341`, `1370-1439`). | No packet is published and the prepared token remains ready when the capacity check fails. |

### Queue publication and dispatch validation

`InvalidDispatch` is the local shape and ownership validator. The exact reason
strings currently constructed are listed here by source region:

```text
execution.rs:597-604
  HSA access-grant agent count exceeds the public ABI
  HSA access-grant set must not be empty
execution.rs:727-729
  HSA access-grant objects belong to different runtimes
execution.rs:1339-1341
  preallocated HSA pending token has insufficient keepalive capacity
execution.rs:1375-1377
  preallocated HSA pending token has not been submitted
execution.rs:1436-1438
  only an active HSA pending token can be a dependency
execution.rs:1514-1530 and 1588-1599
  preallocated completion token belongs to another HSA session
  zero-byte async copies have ambiguous completion semantics
  copy endpoint agents do not both have direct access to both allocations
execution.rs:1903-1905
  dependency barrier tree packet count overflows
execution.rs:1911-1956
  dispatch dimensions must be 1, 2, or 3
  grid and workgroup dimensions must be nonzero
  unused grid and workgroup dimensions must equal one
  workgroup dimensions cannot exceed grid dimensions
  workgroup cardinality overflows
  grid cardinality overflows
  geometry exceeds an exact discovered ISA limit
execution.rs:1979-2050 and 2128-2210
  preallocated completion token belongs to another HSA session
  safe AQL publication requires a single-producer queue
  kernel executable belongs to a different agent
  dependency belongs to a different HSA session
  kernarg must use a discovered kernarg-capable pool
  queue agent has not been granted access to kernarg memory
  kernarg allocation is smaller than kernel metadata
  kernarg allocation does not meet kernel alignment
  kernel requires a kernarg allocation
  dynamic-callstack kernel requires an explicit private-byte allowance
  private segment size overflows
  group segment size overflows
```

The first three dispatch reasons above apply to both `dispatch_prepared` and
`dispatch_after`; the dependency-session reason applies only to
`dispatch_after`. The kernarg, dynamic-callstack, and segment-overflow checks
are duplicated in the two submission methods with the same text. All of these
checks happen before packet publication, except the post-publication health
check, which returns `SessionPoisoned` and moves the active token to retirement.

`QueueFull` is separate from `InvalidProgressRequest`. `enqueue_packets`
computes write/read indices and occupied slots, and returns `QueueFull` when the
packet batch cannot fit (`execution.rs:1683-1715`). It does not spin or publish
part of the batch. `progress_capacity` rejects only zero required packets or a
requirement larger than the ring (`execution.rs:1731-1760`), returning a
non-error `QueueProgress::Ready` or `Backpressured` result for valid bounded
probes. `dispatch_after` maps a dependency tree whose packet count exceeds the
queue size to `InvalidProgressRequest` (`execution.rs:2148-2153`).

`CopyOutOfBounds` remains the range error for all byte copies. `Hsa` is used for
the ROCr async-copy status itself: `copy_async_prepared` uses
`check_status_only` to avoid live-loop allocation (`execution.rs:1549-1568`),
while `copy_async` uses rich `Api::check` (`execution.rs:1623-1642`).

## Lifecycle consequences that are easy to miss

- Explicit `close` methods return teardown errors, but `Drop` cannot. Queue,
  allocation, executable, and runtime drops therefore ignore an `Hsa` status.
- Queue creation validates the returned object after a successful status. A
  malformed object is reported as `InvalidQueueReturned`, not as a generic
  status error. Failed cleanup intentionally leaks callback context to avoid a
  callback use-after-free.
- `SessionPoisoned` is not a snapshot-only diagnostic. `SharedFault::record`
  sets a permanent poisoned bit, and every future `ensure_healthy` fails with
  the newest status, source queue ID (when ROCr supplied one), and epoch.
- Negative completion values are not converted to `Hsa`; they are
  `AsyncSignal`. The first such value is retained in the signal pool and blocks
  new signal acquisition and rearming.
- Incomplete `Pending` and active prepared tokens are `#[must_use]` resources.
  Dropping one moves its signal and every referenced resource into explicit
  deferred retirement. `DeferredRetirement` reports that retention rather than
  releasing objects whose device use is unknown.
- `Error` carries no retry policy. Callers may retry only when the surrounding
  API state permits it, such as a queue-capacity probe returning
  `Backpressured`, a later retirement drain, or a prepared token restored to
  `Ready`. No error arm silently substitutes another implementation or state.

## Literal reason and field dictionaries

The summaries above identify each constructor. This final dictionary keeps the
static strings that become user-visible fields or reasons without relying on a
paraphrase.

### `InvalidIdentity`

```text
AgentUuid::from_str
  expected DEVICE-BODY
  contains an extra separator
  unknown device prefix
  body must be XX or exactly 16 hexadecimal digits
IsaTarget::from_str
  expected amdgcn-amd-amdhsa-- prefix
  architecture must be a lowercase gfx target identity
  feature must have a name followed by + or -
  malformed feature modifier
  duplicate feature modifier
discover_agent UUID cross-check
  device prefix disagrees with HSA_AGENT_INFO_DEVICE
```

`AgentUuid` accepts the four exact prefixes `CPU`, `GPU`, `DSP`, and `AIE`.
`XX` is the only unavailable body; otherwise the body must contain exactly 16
ASCII hexadecimal digits. `IsaTarget` requires the exact
`amdgcn-amd-amdhsa--` prefix, a lowercase `gfx` architecture, and unique
feature names ending in `+` or `-` (`identity.rs:210-264`, `309-377`).

### `InvalidAttribute`

The raw enum decoders use these fixed ABI fields:

```text
HSA_AGENT_INFO_DEVICE
HSA_AGENT_INFO_PROFILE (the caller supplies this field)
HSA_AGENT_INFO_QUEUE_TYPE
HSA_AMD_MEMORY_POOL_INFO_SEGMENT
HSA_AMD_MEMORY_POOL_INFO_LOCATION
```

Discovery adds `HSA_ISA_INFO_NAME_LENGTH`, `HSA_ISA_INFO_NAME`, and the exact
field labels passed to `agent_string`: `AMD agent UUID`, `AMD product name`,
`HSA agent name`, and `HSA agent vendor`. Invalid C booleans use the field label
passed to `c_bool`, including `HSA ISA small machine model`, `HSA ISA large
machine model`, `HSA ISA base profile`, `HSA ISA full profile`, `HSA ISA
default rounding modes`, `HSA ISA base-profile rounding modes`, `HSA ISA fast
f16`, `AMD memory-pool runtime allocation`, `AMD memory-pool all-agent
accessibility`, and `HSA kernel dynamic callstack` (`identity.rs:13-187`,
`discovery.rs:459-650`, `execution.rs:921-929`). The `value` is the raw
integer widened to `u64`; no value is clamped or replaced.

### Capability and kernel reasons

`UnsupportedAgent` currently uses exactly:

```text
the agent is not a GPU
the GPU does not report kernel-dispatch support
the GPU lacks an exact AMD ISA identity
the GPU reports no user-mode queue capability
```

`InvalidQueueSize` uses `ROCr reported inconsistent queue limits` during
discovery and `requested size must be a power of two inside the discovered
range` during queue creation (`discovery.rs:440-457`, `session.rs:236-246`).

`InvalidKernel` uses exactly:

```text
frozen executable returned a zero kernel object
ROCr returned an invalid kernarg alignment
```

The `InvalidQueueReturned` predicate is also exact: `base_is_null`,
`size == 0`, a non-power-of-two `size`, missing
`AGENT_FEATURE_KERNEL_DISPATCH`, or an unknown raw queue kind causes the value;
the four observed fields are never normalized (`session.rs:290-319`).

### Allocation fields

`AllocationFailed.field` has these current values:

```text
agent descriptions
ISA descriptions
memory-pool descriptions
memory-pool handles
HSA ISA name
deferred HSA retirement set
preallocated HSA allocation keepalive slots
preallocated HSA dependency keepalive slots
```

`NoMatchingMemoryPool.kind` is one of `coarse-grained`, `fine-grained`, or
`kernarg`. `CopyOutOfBounds.buffer` is one of `destination` or `source`.
`ResourceBusy.resource` is one of `HSA queue`, `HSA memory allocation`,
`HSA executable`, `preallocated HSA pending token`, `consumed HSA pending
token`, `nonterminal prepared HSA pending token`, `completed HSA pending
signal`, or `completed HSA pending keepalive`.

## Public boundary propagation matrix

This matrix names the public entry points and the error families reachable at
each boundary. A family entry means the exact variants documented above,
including variants propagated through `?` from a helper.

| Public boundary | Reachable error families |
| --- | --- |
| `Runtime::open_default` | `LibraryOpen` retry/return, `MissingSymbol`, `Hsa` from `hsa_init`; static candidates cannot produce `PathContainsNul` |
| `Runtime::open` | `PathContainsNul`, `LibraryOpen`, `MissingSymbol`, pre-init `Hsa` |
| `Runtime::discover` | `RuntimeClosed`; all discovery `Hsa`, `InvalidAttribute`, `InvalidIdentity`, `InvalidUtf8`, `CallbackPanicked`, `AllocationFailed`, and `InvalidQueueSize` values |
| `Runtime::close` | `RuntimeClosed`, teardown `Hsa` |
| `DiscoveredAgent::into_session` | `RuntimeClosed`, `UnsupportedAgent`, timestamp-query `Hsa` |
| `Session::available_memory_bytes` | `RuntimeClosed`, `SessionPoisoned`, query `Hsa` |
| `Session::ensure_healthy` | `SessionPoisoned` |
| `Session::create_queue` | `RuntimeClosed`, `SessionPoisoned`, `UnsupportedAgent`, `InvalidQueueSize`, `UnsupportedQueueKind`, queue-create `Hsa`, `NullQueue`, `InvalidQueueReturned` |
| `Queue::close` | `ResourceBusy`, queue-destroy `Hsa` |
| `Allocation::copy_from_host` and `copy_to_host` | `CopyOutOfBounds` |
| `Allocation::close` | `ResourceBusy`, memory-free `Hsa` |
| `DiscoveredAgent` and `Session` allocation helpers | `RuntimeClosed`, `NoMatchingMemoryPool`, `InvalidMemoryPoolIndex`, `MemoryPoolNotAllocatable`, `InvalidAllocationSize`, allocation `Hsa`, `NullAllocation` |
| `grant_access` and `grant_access_exact_set` | `RuntimeClosed`, access-grant `InvalidDispatch`, access-grant `Hsa` |
| `Session::poll_retirements` | no `Result`; report carries `failed_signal` and `poisoned` state instead of converting it |
| `Session::drain_retirements` | `AsyncSignal`, `DeferredRetirement` |
| `Session::load_hsaco` | `RuntimeClosed`, `SessionPoisoned`, `EmptyCodeObject`, reader/executable `Hsa` |
| `Executable::kernel` | name `NameContainsNul`, symbol-query `Hsa`, `SymbolNotKernel`, `InvalidKernel`, metadata `InvalidAttribute` |
| `Executable::close` | `ResourceBusy`, executable/reader-destroy `Hsa` |
| `Dependency::poll` | `AsyncSignal`, `SessionPoisoned` |
| `Pending::poll` and `Pending::wait` | terminal `AsyncSignal` or `SessionPoisoned`, plus signal-derived `AsyncSignal` |
| `PreparedPending::poll`, `reset`, and `dependency` | `InvalidDispatch`, `ResourceBusy`, and propagated `SessionPoisoned`, `AsyncSignal`, or retirement allocation errors from reset |
| `Session::prepare_pending` | `RuntimeClosed`, `SessionPoisoned`, signal `Hsa`, `NullSignal`, signal-pool `AsyncSignal`/`AllocationFailed`, keepalive `AllocationFailed` |
| `copy_async` and `copy_async_prepared` | `RuntimeClosed`, `SessionPoisoned`, `InvalidDispatch`, `CopyOutOfBounds`, signal `Hsa`, `NullSignal`, signal-pool `AsyncSignal`/`AllocationFailed`, copy `Hsa` |
| `dependent_dispatch_packet_count` | dependency-tree `InvalidDispatch` |
| `Queue::progress_capacity` | `RuntimeClosed`, `SessionPoisoned`, `InvalidProgressRequest` |
| `dispatch_prepared` | `RuntimeClosed`, `SessionPoisoned`, all prepared-dispatch `InvalidDispatch` reasons, `ResourceBusy`, `QueueFull` |
| `dispatch_after` | `RuntimeClosed`, `SessionPoisoned`, dependency `AsyncSignal`, all dependent-dispatch `InvalidDispatch` reasons, `InvalidProgressRequest`, signal `Hsa`, `NullSignal`, signal-pool `AsyncSignal`/`AllocationFailed`, `QueueFull` |

The matrix is a propagation map, not a promise that every listed family can be
triggered on every hardware implementation. Discovery and runtime state decide
which valid branch is reachable; impossible events have no separate defensive
error branch in the implementation.
