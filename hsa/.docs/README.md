# `recipe-hsa`

`recipe-hsa` is Recipe's reviewed ROCr/HSA boundary. It loads the ROCr shared
library at runtime, reads the public large-model HSA ABI, discovers the exact
system, agents, ISAs, queues, and AMD memory pools, and turns one discovered
GPU agent into a scoped `Session`. The session owns the HSA-facing allocation,
signal, executable, queue, asynchronous-copy, and AQL-dispatch operations used
by the native probe and native executor.

The crate is intentionally below Recipe scheduling and execution policy. It
does not choose a device, compile a kernel, build a topology, create an arena
layout, or implement a global runtime. It accepts an already selected
discovered agent and explicit operation inputs, then returns an owned resource
or an owned completion token. `recipe-native-probe` and
`recipe-native-executor` are the only production callers in this workspace.

The implementation has one hard platform boundary: it compiles only for a
64-bit target. The ABI declarations mirror the HSA large model and use
`repr(C)` layouts for the queue and 64-byte AQL packets. Rust type checking is
structural evidence only. A successful Recipe workload on a live ROCr runtime
is required for a runtime claim.

## Position in the runtime

The HSA path has two distinct lifetimes. Discovery is a borrowed snapshot of a
live `Runtime`; realization consumes one `DiscoveredAgent` into a GPU-only
`Session`; operations created from the session borrow that session and keep
their device resources alive until terminal completion.

```text
NativeProbeConfig
        |
        v
HsaBackend::with_runtime
        |
        +--> Runtime::open(path) -> Api + hsa_init
        |
        +--> Runtime::discover()
        |      system + every DiscoveredAgent
        |
        +--> descriptor / measured benchmark
        |
        +--> exact binding reopen
               DiscoveredAgent::into_session()
                       |
                       v
                    Session
             +---------+----------+
             |                    |
        allocations          load_hsaco
             |                    |
             +--------+-----------+
                      v
              copy_async / AQL queue
                      |
                      v
              Pending or PreparedPending
                      |
          poll, wait, reset, retirement
```

`Runtime` is not a process singleton. Rust borrows prevent `Runtime::close`
from being called while a discovery record, session, queue, allocation,
executable, kernel, or pending token still borrows it. The native probe keeps
one `Runtime` in a `RefCell<Option<HsaRuntimeState>>` and reuses it for
discovery and benchmarks. The preparation binding callback lends the resulting
sessions only for that callback's scope.

At the asynchronous boundary, a completion signal is initialized to `1` and
reaches `0` on successful completion or a negative value on device failure.
`Pending` owns that signal and all resources referenced by the submitted
operation. Dropping an incomplete token does not destroy device-visible state;
it moves the signal and keepalives into the session's explicit deferred
retirement set. A terminal unresolved drop is reported as a diagnostic rather
than guessed safe destruction.

This crate contains no HIP path, CUDA Runtime API, vendor operation library,
scheduler policy, global queue registry, background worker, or implicit retry
path. It only exposes ROCr operations that have an exact discovered owner and
an explicit lifetime.

## Manifest and build contract

The [manifest](../Cargo.toml) declares:

| Field | Value |
| --- | --- |
| Package | `recipe-hsa` |
| Version | `0.1.0` |
| Edition | Rust 2024 |
| License | MIT |
| Description | `Reviewed ROCr/HSA discovery and queue ownership for Recipe` |
| Runtime dependency | `libc = "0.2"` |
| Feature | `live-hsa` (empty feature used by the live smoke example) |
| Library target | `recipe_hsa` from `src/lib.rs` |
| Examples | `discover` (always available), `execute_smoke` (`live-hsa` required) |

`libc` is used only for Linux/POSIX dynamic loading and the corresponding C
FFI types. There is no static ROCr link input, generated binding step, build
script, compiler dependency, or test target in the HSA manifest. Workspace
lint settings apply, with package-local denial of unsafe operations inside
unsafe functions and undocumented unsafe blocks.

The crate root [lib.rs](../src/lib.rs#L1-L35) emits a compile error when
`target_pointer_width` is not `64`; the ABI is deliberately not made to look
portable by silently selecting a different layout. [`hsa/README.md`](../README.md) gives
the short user-facing smoke description; this document is the source-traced
architecture and contract.

The root `recipe` facade re-exports the crate as
[`recipe::engine::hsa`](../../src/facade.rs#L17-L41). Direct workspace
consumers are `native-probe` and `native-executor`; the examples are diagnostic
entry points and are not acceptance harnesses.

## Module graph

`src/lib.rs` declares all implementation modules privately and re-exports the
safe public values. Raw C handles and function pointers remain crate-private.

```text
lib.rs
├── abi.rs         reviewed ROCr/HSA constants, repr(C) handles and packets
├── loader.rs      dlopen/dlsym Api owner and status conversion
├── runtime.rs     Runtime open, hsa_init, discover entry, and shutdown
├── discovery.rs   system/agent/ISA/pool enumeration and descriptions
├── identity.rs    typed HSA enum and text identity parsing
├── session.rs     GPU session, queue callback poisoning, queue ownership
├── execution.rs   allocations, signals, HSACO, copies, packets, dispatch
└── error.rs       non-exhaustive public error vocabulary and Result alias
```

The implementation dependency direction is one way toward the FFI owner:

```text
abi
  |
loader ---> runtime ---> discovery ---> identity
   |          |             |
   +----------+-------------+-------> session
                                      |
                                      +--> execution
error and identity are shared value/error boundaries
```

| Module | Main responsibility | Important ownership rule |
| --- | --- | --- |
| `abi.rs` | C constants, handles, `HsaQueue`, kernel/barrier packets, function-pointer aliases | No raw ABI type is public outside the crate. |
| `loader.rs` | `Library`, `Api`, symbol resolution, rich and allocation-free status checks | `_library` outlives every resolved function pointer. |
| `runtime.rs` | Dynamic library selection, balanced initialization, discovery and shutdown | `active` is one balanced `hsa_init` reference. |
| `discovery.rs` | Immutable descriptions plus borrowed raw agent/pool handles | Descriptions never guess unsupported optional attributes. |
| `identity.rs` | Device, queue, pool, UUID, ISA, and PCI value parsing | Unknown enum values and malformed identities fail immediately. |
| `session.rs` | GPU admission, shared queue-fault state, queues and queue callback context | Queue callback data stays alive through `hsa_queue_destroy`. |
| `execution.rs` | Signal pool, deferred retirement, memory, executable, copy, AQL and pending state | A pending token retains every native object the device can still reference. |
| `error.rs` | `Result<T>` and `#[non_exhaustive] Error` | Callers must match a future-proof error set. |

## Public surface and input/output ownership

The root exports are listed in [lib.rs](../src/lib.rs#L21-L35). The following
table describes the production boundary rather than private implementation
helpers.

| Entry point or value | Inputs | Output and ownership |
| --- | --- | --- |
| `Runtime::open_default` | No input; tries the two normal ROCr sonames in order | Active `Runtime`, owning `Arc<Api>` and one successful `hsa_init` reference. |
| `Runtime::open` | `AsRef<OsStr>` path or soname | Active `Runtime`; interior NUL paths and loader or `hsa_init` errors are returned. |
| `Runtime::discover` | Borrowed active runtime | `Discovery<'runtime>` containing system data and borrowed `DiscoveredAgent` records. |
| `Discovery::system`, `agents`, `into_agents` | Borrow or ownership of discovery | Borrowed descriptions or owned agent records that still borrow `Runtime`. |
| `DiscoveredAgent::into_session` | Ownership of one discovered agent | GPU-only `Session<'runtime>` with a fresh fault state and signal pool. |
| `DiscoveredAgent::allocate*` | Pool index and nonzero byte count, or a pool category | `Allocation<'runtime>` with one ROCr pointer and its pool metadata. |
| `Session::allocate*` | Same pool inputs on the GPU session | GPU-owned `Allocation` with the session agent as owner. |
| `grant_access` / `grant_access_exact_set` | Allocation plus exact discovered agents/sessions | Replaces ROCr direct access and records the exact handles for later validation. |
| `Session::load_hsaco` | Nonempty in-memory HSACO bytes | `Executable` retaining bytes, reader, executable and agent. |
| `Executable::kernel` | NUL-free symbol name | `Kernel` retaining its executable and validated kernel metadata. |
| `Session::copy_async` | Two allocations, checked ranges and nonzero size | `Pending` owning signal and both allocation keepalives. |
| `Session::prepare_pending` | Allocation and dependency capacities | `PreparedPending` with signal and pre-reserved keepalive vectors. |
| `copy_async_prepared` / `Queue::dispatch_prepared` | A ready prepared token and operation inputs | `()`; token becomes active and is later polled/reset in place. |
| `Session::create_queue` | Discovered queue config | Host-thread-confined `Queue` owning a ROCr queue and callback context. |
| `Queue::dispatch` / `dispatch_after` | Kernel, optional kernarg, geometry, optional dependencies | `Pending` retaining queue, executable, kernarg, dependencies and barriers. |
| `Pending::poll` / `wait` | Mutable token, optional bounded duration for `wait` | `PollStatus`, `WaitOutcome`, or an error; token remains owned by caller. |
| `Pending::dependency` | Borrowed active token | Clonable dependency handle; it does not consume or recycle the signal. |
| `Session::poll_retirements` / `drain_retirements` | None, or a bounded duration | Retirement report, or unresolved/negative-signal error with resources retained. |
| `Allocation::copy_from_host` / `copy_to_host` | Unsafe host-access promise, checked range and host slice | Byte copy only; no HSA operation or allocation is created. |
| `close` methods | Consuming owner | Explicit native destruction, or `ResourceBusy` if a dependent `Rc` remains. `Drop` is best effort. |

The lifetimes are part of the contract. `Allocation<'runtime>` borrows the
runtime through a phantom lifetime. `Executable<'session, 'runtime>` and
`Kernel<'session, 'runtime>` borrow the session and runtime. `Queue` borrows
the session and is host-thread-confined through its `Rc<QueueCore>`. A
`Pending` or `Dependency` carries the session/runtime lifetimes, so a live
device operation cannot outlive the owner scope in safe Rust.

## ABI boundary

### Raw declarations

[`abi.rs`](../src/abi.rs#L1-L284) is the only module that declares C layouts and
function-pointer types. It records the HSA and AMD extension attribute numbers
used by discovery and execution. HSA status and info values are C `int`
representations (`i32`), while large-model fixed-width packet and queue fields
retain the types specified by the public header.

The opaque handles are one `u64` field with `repr(C)`:

```text
HsaAgent, HsaIsa, HsaSignal, HsaMemoryPool,
HsaCodeObjectReader, HsaExecutable, HsaLoadedCodeObject,
HsaExecutableSymbol
```

`HsaDim3` is three `u32` dimensions. `HsaQueue` contains the queue kind and
features, a packet-ring base pointer, doorbell signal, ring size, reserved
field, and queue ID. `HsaKernelDispatchPacket` and `HsaBarrierAndPacket` are
both asserted at compile time to be exactly 64 bytes, matching one AQL ring
slot. Their packet header starts invalid and is published only after the body
has been written.

### Resolved symbol inventory

`loader::Api` resolves every symbol below from one `dlopen` handle before it
returns. Missing symbols are `Error::MissingSymbol`; there is no partial API
object.

| Group | Resolved ROCr symbols |
| --- | --- |
| Lifecycle and status | `hsa_init`, `hsa_shut_down`, `hsa_status_string` |
| System and agents | `hsa_system_get_info`, `hsa_iterate_agents`, `hsa_agent_get_info`, `hsa_agent_iterate_isas`, `hsa_isa_get_info_alt` |
| Queues | `hsa_queue_create`, `hsa_queue_destroy`, `hsa_queue_load_read_index_scacquire`, `hsa_queue_load_write_index_relaxed`, `hsa_queue_store_write_index_screlease` |
| AMD pools and access | `hsa_amd_agent_iterate_memory_pools`, `hsa_amd_memory_pool_get_info`, `hsa_amd_memory_pool_allocate`, `hsa_amd_memory_pool_free`, `hsa_amd_agents_allow_access`, `hsa_amd_memory_async_copy` |
| Code objects | `hsa_code_object_reader_create_from_memory`, `hsa_code_object_reader_destroy`, `hsa_executable_create_alt`, `hsa_executable_destroy`, `hsa_executable_load_agent_code_object`, `hsa_executable_freeze`, `hsa_executable_get_symbol_by_name`, `hsa_executable_symbol_get_info` |
| Signals | `hsa_signal_create`, `hsa_signal_destroy`, `hsa_signal_load_scacquire`, `hsa_signal_store_screlease`, `hsa_signal_wait_scacquire` |

The callback aliases cover agent, ISA, memory-pool, and queue-error traversal.
The API uses no process-global function table. `Library` owns the `dlopen`
handle, `Api` stores all typed function pointers and the library, and
`Arc<Api>` keeps both alive while any runtime or native resource exists.

### Dynamic loading

`Library::open` converts the supplied `OsStr` through Unix bytes, rejects an
embedded NUL, clears and reads thread-local `dlerror`, and uses
`dlopen(path, RTLD_NOW | RTLD_LOCAL)`. `Library::symbol` clears `dlerror`, calls
`dlsym`, copies any diagnostic, and rejects both a loader error and a null
address. The static symbol-name byte slices are NUL terminated. `Drop` calls
the matching `dlclose` after the function-pointer fields have been dropped.

`Runtime::open_default` is the only soname selection helper. It attempts
`libhsa-runtime64.so.1`, then `libhsa-runtime64.so`, and continues only when
the previous attempt was specifically `LibraryOpen`. A symbol error, ABI
error, or initialization error is returned immediately. `Runtime::open(path)`
uses exactly the supplied path.

`Api::check` converts nonzero HSA status to `Error::Hsa` and asks
`hsa_status_string` for a copied message when the runtime is initialized.
`Api::check_status_only` returns the numeric status without asking for text or
allocating. Native submission paths use the latter after realization so a
failure remains visible without introducing a loop-time diagnostic allocation.

## Identity types

[`identity.rs`](../src/identity.rs#L1-L412) converts raw enum values and exact
text identities at the discovery boundary. Unknown values are errors, not
future guesses.

| Type | Accepted representation and behavior |
| --- | --- |
| `DeviceType` | Raw `0..=3` maps to `Cpu`, `Gpu`, `Dsp`, `Aie`; other values are invalid. |
| `Profile` | Raw `0` or `1` maps to `Base` or `Full`; `as_raw` is used for executable creation. |
| `QueueKind` | Raw `0`, `1`, `2` maps to `MultiProducer`, `SingleProducer`, `Cooperative`; `as_raw` is the queue-create value. |
| `MemorySegment` | Raw `0..=3` maps to `Global`, `ReadOnly`, `Private`, `Group`. |
| `MemoryLocation` | Raw `0` or `1` maps to `Cpu` or `Gpu`; unsupported locations fail. |
| `MemoryPoolFlags` | Retains every raw bit. `bits`, `contains`, and `unknown_bits` expose known and newer flags without rewriting them. |
| `RoundingModes` | Converts three C boolean bytes for default, toward-zero, and nearest-even modes; values other than `0` or `1` fail. |
| `AgentUuidBody` | `Unavailable` for the literal `XX` body, otherwise the parsed eight-byte hexadecimal value. |
| `IsaFeature` | One ordered ISA feature modifier, exposing its name and enabled (`+`) state. |
| `PciAddress` | Splits HSA domain and BDF ID into domain, bus, device, function and formats as `dddd:bb:dd.f`. |

`AgentUuid::from_str` requires `DEVICE-BODY`, with one separator and a known
uppercase prefix (`CPU`, `GPU`, `DSP`, or `AIE`). `XX` means the body is
unavailable; otherwise the body must be exactly 16 ASCII hexadecimal digits.
The original text is retained, and `device_type` and `body` are available as
typed values. Discovery rejects a UUID whose prefix disagrees with
`HSA_AGENT_INFO_DEVICE`.

`IsaTarget::from_str` requires the exact
`amdgcn-amd-amdhsa--gfx...` prefix. The architecture is a lowercase `gfx`
identity with lowercase letters, digits, or `-`; subsequent feature modifiers
have a lowercase name ending in `+` or `-`, and duplicate names are rejected.
The original target, architecture, and ordered feature list are retained.
Only GPU ISAs are parsed as `IsaTarget`; CPU and other agent ISAs have
`amd_target: None`.

## Runtime lifecycle

[`runtime.rs`](../src/runtime.rs#L10-L90) owns one initialized ROCr reference:

1. `Runtime::open_default` selects a soname, or `Runtime::open` accepts an
   explicit path.
2. `Api::load` opens the library and resolves the complete function table.
3. `hsa_init` is called through the resolved pointer. A failure returns
   `Error::Hsa { operation: "hsa_init", message: None }`, because status text
   is not safe before successful initialization.
4. The resulting `Runtime { api, active: true }` is the owner of the balanced
   initialization reference.
5. `discover` requires `active` and delegates to `discovery::discover`.
6. `close(self)` marks `active` false, calls `hsa_shut_down`, and reports the
   status. `Drop` performs the same shutdown when explicit close is not used,
   but cannot return an error.

`ensure_active` is called by every operation that can touch ROCr. Calling an
 operation after `close` returns `RuntimeClosed`. Borrowing prevents a caller
 from consuming `Runtime` while child objects still exist. There is no
 reference-counted shutdown policy or hidden process-global lease.

## Discovery data flow

[`discovery.rs`](../src/discovery.rs#L14-L884) performs exhaustive synchronous
enumeration. It first queries:

```text
SystemDescription
  hsa_version_major/minor
  amd_extension_version_major/minor
  timestamp_frequency_hz
```

It then iterates every agent, builds an `AgentDescription`, and preserves the
raw agent and memory-pool handles in a borrowed `DiscoveredAgent` for later
realization. `Discovery::agents()` borrows the complete vector;
`Discovery::into_agents()` moves it while retaining the runtime lifetime.

### Agent description

Every `AgentDescription` contains:

| Field | Source and use |
| --- | --- |
| `identity` | Name, vendor, typed UUID, NUMA node, and GPU-only driver node and PCI address. |
| `device_type`, `profile`, `feature_bits` | Raw HSA agent attributes converted to typed values; feature bits retain unknown future bits. |
| `hsa_version_major/minor` | Agent version attributes. |
| `first_isa_wavefront_size` | Deprecated agent wavefront query, read only when kernel-dispatch support is advertised. |
| `queue` | `None` when all queue limits are zero, otherwise validated queue count, packet range, and advertised kind. |
| `amd_gpu` | GPU-only AMD chip, clocks, CU/SIMD, scratch, memory, SDMA/XGMI, XCC, product, and timestamp properties. |
| `isas` | Every enumerated ISA, including exact text and machine/profile/rounding/geometry limits. |
| `memory_pools` | Every AMD pool and its segment, optional location/flags/maximum, accessibility, and runtime-allocation properties. |

For a GPU, discovery additionally requires the AMD driver node, PCI domain and
BDF ID, and all `AmdGpuProperties` attributes. Non-GPU agents have no GPU
properties, driver node, or PCI address. `AgentDescription::supports_kernel_dispatch`
and `supports_agent_dispatch` test the raw feature bits without discarding
unknown bits.

Queue metadata is considered absent only when maximum queues, minimum packets,
and maximum packets are all zero. Otherwise, `validate_queue_capabilities`
requires a nonzero queue count, nonzero power-of-two minimum and maximum
packet sizes, and `minimum_packets <= maximum_packets`. The queue kind must
also be one of the three known raw values.

### ISA description

`discover_isa` reads the reported name length and rejects values above the
4,096-byte bound. It accepts ROCr implementations that report the length
including or excluding the trailing NUL, truncates at the first NUL, and
rejects nonzero bytes after that NUL. The bytes must be UTF-8. GPU names must
parse as `IsaTarget`; non-GPU names are retained as text only.

The result records:

```text
name, amd_target
small/large machine model support
base/full profile support
default and base-profile rounding modes
fast f16 support
maximum workgroup dimensions and cardinality
maximum grid dimensions and cardinality
maximum fbarrier count
```

All C boolean fields pass through `identity::c_bool`, so an ABI value other
than `0` or `1` is `InvalidAttribute`.

### Memory-pool description

`discover_memory_pool` reads the segment, runtime-allocation flag, size,
all-agent accessibility, and optional extension attributes. If runtime
allocation is allowed, granule, recommended granule, and alignment are all
required and form `AllocationProperties`. Global pools also expose
`MemoryPoolFlags`; non-global pools leave `global_flags` as `None`.

`location` and `maximum_aggregate_allocation_bytes` are optional because newer
ROCr versions expose them while older versions may return
`STATUS_ERROR_INVALID_ARGUMENT`. `pool_info_optional` converts exactly that
status to `None`; any other non-success status is an error. The absence of
location is preserved and never guessed as CPU or GPU.

### Callback and allocation behavior

The three synchronous iterators use stack collectors passed through `*mut
c_void`. Each callback checks for a null data pointer and wraps `Vec::push` in
`catch_unwind`. A panic becomes `CallbackPanicked` and the C callback returns a
generic error, so a Rust panic cannot unwind through ROCr. Collector allocation
failures are represented by `AllocationFailed` where the enclosing vector is
reserved explicitly; ordinary HSA status failures retain the operation name.

## Session, queue, and asynchronous fault ownership

[`session.rs`](../src/session.rs#L22-L409) converts a discovery record to a
GPU-only execution scope.

### Session admission

`DiscoveredAgent::into_session` consumes the record and requires:

1. The borrowed runtime is active.
2. The device type is `Gpu`.
3. Kernel-dispatch support is present in `feature_bits`.
4. At least one ISA exists and every ISA has an exact `amd_target`.
5. The system timestamp frequency can be read.

Failure at any gate is `UnsupportedAgent`, `RuntimeClosed`, or the underlying
HSA query error. A successful session copies the agent description and raw
pool handles, creates one `SharedFault`, and creates one `SignalPool` using
the system timestamp frequency.

### Queue fault callback

`SharedFault` is shared by every queue and every signal pool operation in one
session. `queue_error_callback` performs no lock acquisition and no allocation:
it records the status and optional source queue ID, increments an epoch, marks
the session permanently poisoned with release ordering, and wakes the
condition variable. `Session::fault` returns the newest `QueueFault`; it
contains status, optional queue ID, and epoch. `ensure_healthy` returns
`SessionPoisoned` after the first callback. Poisoning is permanent for that
session, including after all pending signals reach terminal values.

The callback data is a boxed `QueueCallbackContext` retained by `QueueCore`
until `hsa_queue_destroy` returns. If queue destruction fails, the callback
context is leaked rather than risking a use-after-free from a late ROCr
callback. The condition variable is not used by the public wait methods; the
current bounded waits use HSA signal waits directly.

### Queue configuration and ownership

`QueueConfig::new(size_packets, kind)` sets both private and group segment
size fields to `u32::MAX`, which asks ROCr to select its normal limits. Session
queue creation validates the discovered capability range and requires a
power-of-two packet count. The requested kind must be compatible with the
advertised kind:

| Advertised | Accepted request |
| --- | --- |
| `SingleProducer` | `SingleProducer` only |
| `MultiProducer` | `MultiProducer` or confined `SingleProducer` |
| `Cooperative` | `Cooperative` only |

After `hsa_queue_create`, the returned pointer must be non-null, have a
non-null power-of-two ring base, a nonzero power-of-two size, a known queue
kind, and kernel-dispatch support in its feature bits. An invalid successful
return is destroyed immediately and reported as `InvalidQueueReturned`.

`QueueCore` owns the one raw queue pointer and callback box. `Queue` exposes the
queue ID, realized ring size, requested producer kind, and parent session. Its
`Rc` makes the queue host-thread-confined and lets active pending keepalives
retain it. `Queue::close(self)` succeeds only when the core has no other `Rc`
owners; otherwise it returns `ResourceBusy`. `Drop` calls `hsa_queue_destroy`
best effort.

## Memory allocation and access

### Allocation selection

`DiscoveredAgent` and `Session` share the same allocation methods:

| Method | Selection predicate |
| --- | --- |
| `allocate(pool_index, size)` | Exact discovered index, runtime allocation enabled, nonzero size, and optional maximum respected. |
| `allocate_coarse(size)` | First global, runtime-allocatable pool with `COARSE_GRAINED`. |
| `allocate_fine(size)` | First global, runtime-allocatable pool with `FINE_GRAINED` or `EXTENDED_SCOPE_FINE_GRAINED`. |
| `allocate_kernarg(size)` | First global, runtime-allocatable pool with `KERNARG_INITIALIZATION`. |

No category method uses location as a substitute for flags. The allocator
checks the exact pool vector/index pair, calls
`hsa_amd_memory_pool_allocate(pool, size, 0, &pointer)`, rejects a null
pointer after success, and records the pool owner, pool index, global flags,
size, and an initial direct-access set containing the owner. Zero-byte or
over-limit requests are `InvalidAllocationSize`; unknown indices and
non-allocatable pools have dedicated errors.

`AllocationInner` owns the one pointer and calls
`hsa_amd_memory_pool_free` exactly once. `Allocation::close(self)` requires
the allocation's `Rc` to be unique. A pending copy or dispatch holds an
`Rc<AllocationInner>`, so explicit close while that operation is live returns
`ResourceBusy`. `Drop` frees best effort.

### Access grants

`grant_access` is a replacement operation, not an additive operation. It sends
the exact discovered agent handles to `hsa_amd_agents_allow_access`; ROCr
retains the allocation owner in addition to the supplied set. The allocation
records the owner and every supplied unique handle only after the call
succeeds. An empty set and a count that does not fit the ABI are rejected.

`Session::grant_access_exact_set` accepts session and discovered-agent
references from the same runtime, sorts and deduplicates their raw handles,
and replaces the direct-access set with that exact set. It rejects an
allocation or agent from a different runtime. This is the operation used by
the native executor when sharing one device arena with all HSA sessions and
the selected host allocator.

### Host byte copies

`copy_from_host` and `copy_to_host` are unsafe because the caller must establish
host accessibility, coherence, nonoverlap, and operation ordering. Both use
checked `offset + size` arithmetic and return `CopyOutOfBounds` before pointer
arithmetic. The write path requires no concurrent device reader or writer; the
read path requires a system-scope producer to have completed. They perform a
plain nonoverlapping byte copy and do not create a signal.

## Loader, HSACO, executable, and kernel ownership

`Session::load_hsaco` in [execution.rs](../src/execution.rs#L974-L1060) takes a
nonempty byte slice and copies it into an `Arc<[u8]>`. The operation sequence
is fixed:

1. Require an active, healthy session and reject `EmptyCodeObject`.
2. Create an HSA code-object reader from the in-memory bytes. The `Arc` is
   retained in `ExecutableInner` before the C call, so ROCr never observes a
   dangling input buffer.
3. Create an executable using the discovered session profile and
   `DEFAULT_FLOAT_ROUNDING_MODE_NEAR`.
4. Load the reader for the exact session agent.
5. Freeze the executable.
6. Check the session fault state and return an `Executable` that owns the
   bytes, reader, executable handle, and agent identity.

Any failure after reader creation runs the same ordered destructor: executable
first, then reader, then backing bytes. If executable destruction fails, the
reader and bytes are leaked rather than released while ROCr could still use
them. If reader destruction fails, its backing bytes are also leaked. The
normal `Drop` path is best effort; explicit `Executable::close` reports the
status and requires no live `Kernel` or pending keepalive.

`Executable::kernel(name)` rejects an interior NUL, looks up the symbol for the
exact executable agent, and reads symbol kind, kernel object, kernarg size and
alignment, group/private segment sizes, and dynamic-callstack state. The symbol
must be a kernel, its object must be nonzero, and alignment must be nonzero and
power of two. The returned `Kernel` owns an `Rc` reference to the executable,
so the executable cannot be explicitly closed while the kernel is live.

The public `KernelMetadata` is:

```text
kernarg_segment_size: u32
kernarg_segment_alignment: u32
group_segment_size: u32
private_segment_size: u32
dynamic_callstack: bool
```

The native executor cross-checks these values against inspected HSACO ABI
metadata and finalized resource bounds before any dispatch.

## Signals, pending tokens, and retirement

### Signal pool

`SignalPool` is session-owned and not `Send` because its state uses `Rc` and
`RefCell`. It maintains:

```text
available                 terminal signals reset to 1 for reuse
retired                   submitted signals awaiting <= 0
retirement_reservations   capacity reserved before submission
fatal_signal              first negative signal value
```

`acquire` first collects terminal retired records, rejects an already recorded
negative signal, reserves retirement capacity, reuses an available signal or
calls `hsa_signal_create(1, 0, null, &signal)`, and returns an `Rc<SignalRecord>`.
The reservation is held until submission ownership is known. `rearm` performs
the same capacity reservation, clears terminal state, and stores `1` with
release ordering only after a previous terminal completion.

`SignalRecord::Drop` inspects the signal with acquire ordering. Positive values
outside the explicit retired set are an ownership invariant violation; the
signal is deliberately not destroyed and a terminal leak diagnostic is
printed. Negative values are terminal failures and are destroyed. Zero values
are returned to `available` after reserving vector capacity, or destroyed if
the pool cannot retain them.

`SignalPool::collect_retired` scans the explicit set for values `<= 0`, marks
each terminal value, records the first negative value as fatal, drops the
operation keepalive before the signal, and reports reclaimed/pending/failure/
poison state in `RetirementReport`. `wait_one_retired` uses a bounded
`hsa_signal_wait_scacquire` with `SIGNAL_CONDITION_LT` and the discovered
timestamp frequency.

When `SignalPoolInner` is dropped, available signals are destroyed. Retired
records that are already terminal are reclaimed; positive records are
forgotten with their keepalives and a terminal diagnostic asks the caller to
drain retirements before dropping the session. This is an intentional leak of
device-visible state rather than unsafe release.

### `Dependency`

`Dependency` clones the `Rc<SignalRecord>` and pool. `poll` loads the signal
with acquire ordering and classifies it as `Pending` (`> 0`), `Complete` (`0`),
or `AsyncSignal` (`< 0`). Pending and complete values also check shared queue
poisoning. A dependency is never recycled by polling and can be cloned for
multiple barrier consumers.

### `Pending`

`Pending` is `#[must_use]` and contains the pool, optional signal, optional
`PendingKeepalive`, and a cached terminal result. The keepalive may retain:

```text
queue core, executable, allocation Rc values, dependency Rc values
```

`poll` is nonblocking. It keeps a positive signal nonterminal, transitions a
zero signal to terminal after releasing the retirement reservation and all
keepalives, and converts a negative signal to `AsyncSignal` while recording
the pool's fatal signal. A queue callback can poison the session before the
operation signal reaches zero; in that case `poll` returns `SessionPoisoned`
but leaves the token nonterminal so `Drop` still retires the device references.

`wait(timeout)` repeatedly polls and requests at most one millisecond of
active wait at a time. ROCr timeout arguments are hints, so the method checks
elapsed host time and returns `TimedOut` while retaining the live token. It
never requests an unbounded wait.

`Drop` behavior is state-sensitive:

| Token state | Drop action |
| --- | --- |
| Nonterminal signal and keepalive | Move both to `SignalPool::retired`. |
| Cached terminal result | Drop keepalive and terminal signal normally. |
| Partially absent internals | Drop whichever pieces remain. |

### `PreparedPending`

`Session::prepare_pending(allocation_capacity, dependency_capacity)` realizes
one signal and reserves both keepalive vectors before the run loop. It fails
with `AllocationFailed` if either reservation cannot be made. A ready token
contains no queue, executable, allocation, or dependency references.

`copy_async_prepared` and `dispatch_prepared` call `begin`, which consumes the
ready state, checks vector capacities, clears stale keepalives, and performs no
signal acquisition or collection growth. A failed native submission restores
the ready state. A successful submission changes the token to active. `poll`
on a ready token is invalid; `dependency` is valid only while active.

`reset` polls an active token, refuses to reset while pending, and after
terminal success rearms the same signal to `1` and restores ready storage.
`Drop` releases an unsubmitted ready signal or drops an active `Pending`.
This is the no-loop-time-realization path used by `native-executor`.

## Asynchronous copies

`Session::copy_async` is the general path used by diagnostics and probing. It
requires an active healthy session, a nonzero byte count, checked source and
destination ranges, and mutual direct access by both allocation owner agents.
It acquires a signal, retains both allocation `Rc`s, calls
`hsa_amd_memory_async_copy` with no dependency array, and returns `Pending`.
Submission failure releases the unsubmitted signal. A post-submit queue fault
retires the active signal and keepalives instead of freeing them early.

`copy_async_prepared` enforces the same checks plus same-session prepared-token
ownership. It appends exactly two allocation keepalives to the pre-reserved
vector and uses `check_status_only` for the native call. It restores ready state
on a submit failure and retires active state if the session becomes poisoned
immediately afterward.

The operation is asynchronous for both host/device and device/device cases.
The caller must poll or wait before reading a host fine-grained allocation, and
must retain the token or explicitly drain its deferred retirement set before
tearing down the session.

## AQL queue publication and capacity

`Queue` exposes only a host-confined single-producer publication path. The
internal `QueueIo` sequence is:

1. Load the relaxed write index and acquire read index.
2. Compute wrapped occupancy. If the packet batch does not fit, return
   `QueueFull` without changing the write index or any slot header.
3. For each packet, write the complete 64-byte body while its header is
   `PACKET_TYPE_INVALID`.
4. Store the final packet header with release ordering, publishing all prior
   body writes.
5. Store the next write index with the ROCr sequentially consistent release
   function.
6. Ring the queue doorbell with a release store.

The ring slot is selected with `index & (size - 1)`, relying on the validated
power-of-two ring size and 64-byte slot layout. The packet body is never
visible before its valid header. A multi-producer queue can be requested only
when ROCr advertises it, but safe packet publication methods still require the
confined `QueueKind::SingleProducer` request.

`Queue::progress_capacity(required_packets, probe_budget)` is explicitly
bounded and nonblocking. It rejects zero or over-sized requests and reads
occupancy at most `probe_budget` times. It returns `QueueProgress::Ready` with
available packets and probe count when capacity appears, or
`Backpressured` without publishing anything. Native execution uses a budget of
one probe for the strict loop and maps backpressure to resource contention.

## Dependency barriers and dispatch geometry

`Queue::dispatch` is `dispatch_after` with no dependencies. `dispatch_after`
validates session identity, queue producer kind, executable agent identity,
all dependency pools, and each dependency's current signal state before
publishing anything. It computes the required packet count with
`dependent_dispatch_packet_count`.

Barrier lowering uses a fixed fan-in of five:

```text
input dependency signals
       | groups of at most five
       v
barrier-and completion signals
       | repeat groups of at most five
       v
one terminal barrier signal
       |
       v
kernel dispatch packet
```

Every barrier packet has system acquire and release fence scopes. The terminal
barrier completion signal is retained in the kernel keepalive, and the kernel
packet is published immediately after the barrier packet tree. The host never
waits for dependencies; the device queue enforces the order. A zero-dependency
dispatch needs one kernel packet. The public count function adds one kernel
packet to the checked barrier-tree count and rejects a `u32` overflow.

`DispatchGeometry` contains dimensions, grid, workgroup, dynamic group/private
bytes, and the optional kernel barrier bit. `one_dimensional(grid, workgroup)`
sets inactive axes to one and dynamic sizes to zero. `validate_geometry` then
requires:

* dimensions in `1..=3`;
* every grid/workgroup axis nonzero;
* inactive axes exactly one;
* workgroup axis no larger than its grid axis;
* checked workgroup and grid cardinalities;
* every axis and cardinality within every exact discovered ISA limit.

Before an AQL kernel packet is built, the queue path additionally checks:

* a zero-size kernarg kernel receives no kernarg allocation;
* a nonzero kernarg kernel receives a kernarg-capable allocation with enough
  bytes, exact queue-agent access, and the metadata alignment;
* dynamic-callstack kernels receive a nonzero explicit private-byte allowance;
* static plus dynamic private/group segment sizes do not overflow.

The packet's kernel object, kernarg pointer, geometry, segment sizes, and
completion signal are all populated before the header publication sequence.
`dispatch_prepared` performs the same checks but uses one pre-realized signal
and one fixed keepalive token, and it accepts no dependency list.

## Native probe caller

`recipe-native-probe` owns the policy that turns HSA discovery into measured
GPU evidence. The HSA-specific implementation is
[`native-probe/src/hsa.rs`](../../native-probe/src/hsa.rs#L67-L875); its parent
module selects it through the `NativeBackend::Hsa` variant in
[`native-probe/src/native.rs`](../../native-probe/src/native.rs#L11-L298).

### Runtime retention and discovery

`HsaBackend::new` stores the configured library identity, host-memory key, PCI
sysfs root, code-object version, and kernel build configuration. A code-object
version of zero is rejected before opening ROCr. `with_runtime` checks for an
AMD PCI accelerator, selects the configured ROCr library, rejects a library
identity change after initialization, opens one `Runtime`, stores it in
`RefCell<Option<HsaRuntimeState>>`, and invokes a callback with borrowed
`PinnedLibrary` and `Runtime` values.

If no AMD PCI accelerator is present before initialization, `with_runtime`
returns `None`. If the accelerator or library disappears after a runtime has
been retained, it returns a discovery error rather than silently reopening a
different runtime. A normal `NativeGpuProbe::new` includes both CUDA and HSA
backends and marks discovery exhaustive; `hsa_diagnostic` includes only HSA
and is diagnostic, not a complete profile path.

`HsaBackend::discover` calls `Runtime::discover`, iterates every agent, and
retains only GPU descriptors. A descriptor requires kernel dispatch, a stable
UUID body, exact PCI identity, one exact artifact target, AMD properties, queue
limits, a KFD LDS value, and a matching configured runtime identity. The
resulting `GpuDescriptor` records HSA target backend `amd-rocr-hsa`, ELF64
code-object ABI/version, measured-capacity hint, PCI/KFD/link identities,
queue limits, wavefront width, workgroup limits, and whether SDMA can overlap
transfers.

`exact_target` rejects a non-AMDGPU ISA, no target, multiple distinct specific
targets, or ambiguous generic targets. A single non-generic target wins; a
single identical generic target is accepted. `hsa_capacity` prefers the
largest allocatable GPU-located coarse global pool and falls back to the AMD
available-memory counter only when no such pool exists.

### Measured benchmark

`HsaBackend::benchmark` re-discovers the expected descriptor and requires one
unique matching UUID. It selects that GPU and a CPU memory agent on the same
NUMA node when possible, then consumes the GPU agent into a `Session`.

The benchmark path allocates fine CPU source/destination and coarse GPU
source/destination buffers, grants both directions of access, initializes the
source through the unsafe host copy, and measures:

```text
host -> device asynchronous copy
device -> host asynchronous copy
device -> device asynchronous copy
```

Each operation uses `copy_async`, polls with a bounded exponential backoff,
and verifies returned bytes independently. It then lowers and builds a
Recipe-owned F32 FMA HSACO for the exact target and code-object version,
loads it with `Session::load_hsaco`, resolves the inspected symbol, checks
kernarg metadata, allocates CPU kernarg memory, writes explicit pointer and
element arguments, creates a single-producer queue at the discovered minimum
size, dispatches one-dimensional geometry, waits for completion, downloads,
and verifies the computed bytes. Queue, executable, kernelarg, and allocations
are explicitly closed after terminal completion.

The benchmark returns measured capacity, calculation rate, device memory rate,
host-to-device rate, and device-to-host rate. Timeout cleanup continues polling
the still-live token at a capped low rate so a benchmark deadline never causes
unsafe release of a live signal or allocation.

## Preparation-scoped native bindings

`recipe-native-probe/src/bindings.rs` reopens measured origins by exact key and
lends HSA state to one preparation/execution callback. `with_native_execution_bindings`
first rediscoveries the complete local inventory and resolves it against the
measured profile. It partitions expected GPU descriptors by the exact target
backend `amd-rocr-hsa`, calls `HsaBackend::with_runtime`, and performs another
exact HSA discovery.

`realize_hsa` partitions CPU and GPU agents, retains only CPU agents with both
allocatable fine and kernarg global pools, sorts them deterministically, and
selects exactly one same-NUMA allocator per GPU. Missing or ambiguous host
allocators are errors. Each expected GPU must reopen exactly once, retain its
exact target string and queue minimum, and successfully pass
`DiscoveredAgent::into_session`.

The callback receives an `HsaBinding` containing:

```text
measured DeviceId
borrowed Session
borrowed exact host allocator DiscoveredAgent
exact target identity
code-object version
queue packet size and maximum queue count
display connector count
```

The binding borrows both runtime-owned objects. It cannot escape the callback,
so the retained `Runtime` stays alive through all preparation resources and is
not a dynamic placement fallback. `HsaBinding::allocate_host_fine` allocates
from the exact CPU allocator and grants the bound GPU access in one operation.

## Native executor caller

`recipe-native-executor/src/hsa.rs` is the production HSA bridge below the
backend-neutral executor. It imports only the public HSA values needed for
allocations, queue creation, HSACO loading, prepared copies, prepared kernel
dispatch, signal polling, and retirement. The root native-executor module
re-exports `HsaBackend` and `HsaBinding`.

### Binding and resource ownership

`HsaBinding` is a preparation-scoped borrowed view. `HsaResources` owns the
immutable `ExecutionPlan`, one `DeviceResources` per planned device, task
contracts, a set of prepared task IDs, one `PreparedPending` per candidate
task, and a backend poison bit. Each `DeviceResources` owns:

```text
Session and exact host allocator borrows
realized Queue values by QueueSlotId
completion-slot ownership state
LoadedArtifact kernels and shared Executable values
host kernarg allocations and host byte staging vectors
four-byte metric buffers
fine-grained staging allocation
init-image contract and exit output vectors
optional coarse scratch allocation
```

`HsaBackend` has one typestate-like state enum:

```text
Ready(bindings, artifacts)
Prepared(HsaPreparedResources)   [retained direct handoff path]
Warmed(HsaResources)
Bound
```

`bind_resources` consumes `Ready`, validates the full execution bundle and
runtime artifacts, realizes resources, and returns `HsaResources`. A second
bind fails with `BackendState`. `bind_partition` performs the same operation
for a selected task set. The production preparation flow normally hands off
warmed resources; the direct finalized prepared state remains explicit and
fails closed when its bundle identity or task set differs.

### Resource realization

`HsaResources::realize` requires exact one-to-one device bindings and an
enforced scheduler quota for every device. It validates the CPU host allocator
type, allocatable kernarg and fine pools, and the exact target advertised by
the GPU session. It creates one single-producer queue for every planned queue
slot using the binding's measured minimum packet size, and creates matching
available completion-slot state.

For every device it then realizes:

1. Fine-grained host staging sized by the finalized resource manifest.
2. The required init-image contract and staging capacity check.
3. Optional coarse scratch memory.
4. One `Executable` per distinct HSACO content digest.
5. One `Kernel` per logical artifact entry resolved from the shared executable.
6. One host kernarg allocation and byte vector per completion slot, sized to
   the maximum inspected kernel ABI on that slot.
7. One four-byte fine-grained metric allocation per metric task.
8. One host output vector per exit transfer to an external destination.

Before loading, `inspect_hsaco_bundle` checks each byte image against the exact
target, code-object version, and logical artifact ABI. The HSA loader then
checks the runtime symbol and metadata again. Finalized kernel resource bounds
become the `HsaArtifactResourceEnvelope`; a dynamic-callstack kernel requires
a nonzero finalized private-byte bound that fits the AQL field. Any mismatch
is an artifact, value, arena, or protocol error before loop submission.

The pending pool is created after resource realization. Every candidate task
gets one `Session::prepare_pending(2, 0)` token, so the common copy path can
retain two allocations without loop-time vector growth. Candidate task device
selection uses the calculation device, metric value device, or the device end
point of a transfer; a transfer with no device endpoint is invalid.

### Task contract and phase mapping

The HSA bridge turns finalized task kinds into `HsaTaskContract` values and
rejects any runtime work that differs from those values. The only valid HSA
classes are:

| Finalized task | Required phase | HSA operation |
| --- | --- | --- |
| Init transfer from external to device | `Init` | Copy init image from fine staging to the device arena. |
| Device-to-device transfer in init or loop | `Init` or `Loop` | Direct asynchronous copy between checked HSA arenas. |
| Calculation | `Loop` | Fill preallocated kernarg bytes, then `dispatch_prepared`. |
| Metric | `Loop` | Four-byte device readback to a preallocated fine buffer. |
| Exit transfer from device to device or external | `Exit` | Device copy, or copy to fine staging followed by host collection. |

External-to-external, external-to-device in loop, device-to-external in loop,
and external endpoints in exit are rejected by `transfer_work_class`. Init and
exit are therefore transfers, not additional model work, and metrics are
specialized four-byte readback transfers.

### Submission path

The executor calls `prepare_pending` once per task, then `submit` or
`submit_loop_iteration` with backend-neutral `BackendWork`. The bridge checks
the task contract, immutable submission queue/completion slots, arena identity,
byte ranges, route and lane claims, and completion-slot ownership before
calling HSA.

The operation-specific paths are:

* `submit_admission`: copy the finalized init image into fine staging and call
  `copy_async_prepared` into the destination arena.
* `submit_internal_transfer`: resolve two device endpoints and call
  `copy_async_prepared` for the planned offsets and byte count.
* `submit_exit_transfer`: copy to fine staging for an external destination or
  directly to another device arena. The completed external path records an
  egress action.
* `submit_calculation`: fill host kernarg bytes with arena pointers, run ID,
  loop iteration, element count, and optional I32 fault flag; probe one packet
  of queue capacity; compute padded one-dimensional grid; add finalized dynamic
  private bytes; then call `Queue::dispatch_prepared`.
* `submit_metric`: check one four-byte F32/I32 value, then copy it into the
  task's preallocated metric buffer with `copy_async_prepared`.

Completion slots transition from `Available` to `Active { task }` before each
submission and back to `Available` only after terminal polling. A failed native
submission releases the claim. A busy or missing slot is a protocol or
resource-contention error, not an implicit new signal.

`submit_loop_iteration` calls `prepare_loop_pending` before the operation. A
ready token is used as-is; a terminal token is reset and rearmed; an active
token cannot be submitted again. This preserves one pre-realized signal and
fixed keepalive storage across loop iterations.

### Poll, metrics, egress, and poison

`poll_pending` validates that the HSA completion slot is owned by the pending
task, then delegates to `PreparedPending::poll`. A pending signal maps to
`BackendPoll::Pending`. Terminal completion releases the slot and performs the
pending action:

* metric action reads exactly four system-scope bytes from the fine buffer and
  decodes the finalized F32 or I32 type;
* egress action reads the preallocated fine staging vector;
* ordinary copy/calculation action has no host result.

Negative HSA signals, queue callback poisoning, deferred retirement with a
poisoned session, and backend health errors mark the native backend poisoned.
Once poisoned, later operations fail with `BackendPoisoned` or the underlying
HSA error, and no fallback submission is attempted.

### Teardown

`destroy_resources` first drops the pending pool so its prepared tokens release
any executable keepalives. `destroy_devices` requires every completion slot to
be `Available`, drains each session's deferred retirements for a bounded ten
millisecond pass, then closes queues, drops kernels, closes shared
executables, closes kernarg and metric allocations, closes fine staging, and
frees optional scratch. Active completion slots, unresolved retirements,
queue-close failures, executable-close failures, and allocation-close failures
remain errors. The order is explicit so no queue, executable, allocation, or
signal is destroyed while a device-visible operation can still reference it.

The native executor records physical call accounting for bind, pending
preparation, arena allocation/release, submissions, polls, exit collection,
and destruction. Accounting is observation of the real calls; it does not
replace HSA completion or authoritative state.

## Failure vocabulary

[`error.rs`](../src/error.rs#L3-L316) defines `Result<T>` and a non-exhaustive
`Error`. The variants are grouped below by the boundary that can produce
them.

| Group | Variants and observed trigger |
| --- | --- |
| Loader and names | `PathContainsNul`, `NameContainsNul`, `LibraryOpen`, `MissingSymbol`: invalid C strings, failed `dlopen`, or unresolved required symbol. |
| HSA status and ABI values | `Hsa`, `InvalidUtf8`, `InvalidIdentity`, `InvalidAttribute`, `CallbackPanicked`: non-success runtime status, malformed text or identity, invalid enum/boolean/optional value, or panic inside a synchronous callback. |
| Runtime and agent admission | `RuntimeClosed`, `UnsupportedAgent`: operation after shutdown, non-GPU session request, missing kernel dispatch, or missing exact GPU ISA. |
| Queue creation | `InvalidQueueSize`, `UnsupportedQueueKind`, `NullQueue`, `InvalidQueueReturned`: request outside discovered limits, incompatible producer kind, null successful return, or malformed returned queue fields. |
| Allocation and access | `AllocationFailed`, `InvalidMemoryPoolIndex`, `MemoryPoolNotAllocatable`, `NoMatchingMemoryPool`, `InvalidAllocationSize`, `NullAllocation`, `CopyOutOfBounds`: host/vector reservation, pool selection, size, pointer, or range failure. Access-runtime status is `Hsa`. |
| Explicit lifetime | `ResourceBusy`: a queue, allocation, executable, or prepared token still has dependent `Rc` ownership. |
| Executable and dispatch inputs | `EmptyCodeObject`, `SymbolNotKernel`, `InvalidKernel`, `InvalidDispatch`: empty HSACO, wrong symbol kind, invalid metadata, wrong session/queue/agent, bad geometry, bad kernarg, overflow, or dependency ownership. |
| Queue capacity | `QueueFull`: an actual packet batch does not fit and no write index was advanced. `InvalidProgressRequest`: a bounded probe requested zero or more packets than the ring. |
| Completion state | `NullSignal`, `AsyncSignal`: zero signal handle or a negative terminal signal. |
| Session poison and teardown | `SessionPoisoned`: asynchronous queue callback failure. `DeferredRetirement`: unresolved deferred signals remain at timeout or poison; referenced resources are retained. |

`Error` implements `Display` with operation names, status numbers, identity
values, queue indices, signal values, and resource counts. It implements
`std::error::Error`, derives clone/debug/equality, and is `#[non_exhaustive]`.
Callers must not assume the listed variants are permanently complete.

## Examples and diagnostic commands

The [discover example](../examples/discover.rs#L1-L14) opens the default ROCr
runtime, prints the system and every agent description, explicitly drops the
discovery borrow, calls `Runtime::close`, and exits. It requires a live ROCr
installation and is diagnostic only.

The [execute smoke example](../examples/execute_smoke.rs#L1-L136) demonstrates
the complete fine-to-coarse-to-fine path:

1. Open and discover ROCr, selecting one CPU and one GPU agent.
2. Consume the GPU into a session.
3. Allocate fine CPU source/destination and coarse GPU storage.
4. Grant access in both directions.
5. Initialize host bytes, submit upload and download copies, wait, and verify.
6. Optionally read `RECIPE_HSA_SMOKE_COPY_HSACO`, require
   `RECIPE_HSA_SMOKE_SYMBOL`, load and resolve the HSACO, allocate kernarg and
   output, create a minimum-size single-producer queue, submit a dependent
   copy kernel, wait, download, verify, and explicitly close queue/executable
   and allocations.
7. Close all remaining allocations, drop the session and CPU agent, and close
   the runtime.

Run the diagnostic entry points as:

```text
cargo run --example discover -p recipe-hsa
cargo run --features live-hsa --example execute_smoke -p recipe-hsa
RECIPE_HSA_SMOKE_COPY_HSACO=/path/to/copy.hsaco \
RECIPE_HSA_SMOKE_SYMBOL=kernel_name \
cargo run --features live-hsa --example execute_smoke -p recipe-hsa
```

The optional HSACO must accept destination and source pointers in the first
16 kernarg bytes, and the smoke geometry uses one-dimensional byte-count/4
elements with a workgroup of 64. The examples compare actual destination bytes
to the input bytes; printing a success line or receiving an exit status is not
itself correctness evidence.

## Source map and validation boundary

| Concern | Source |
| --- | --- |
| Public exports and 64-bit gate | [`hsa/src/lib.rs`](../src/lib.rs#L1-L35) |
| Raw ROCr constants, structs, and function pointers | [`hsa/src/abi.rs`](../src/abi.rs#L1-L284) |
| Dynamic library and API table | [`hsa/src/loader.rs`](../src/loader.rs#L22-L313) |
| Runtime lifetime and shutdown | [`hsa/src/runtime.rs`](../src/runtime.rs#L10-L90) |
| Identity parsers and typed flags | [`hsa/src/identity.rs`](../src/identity.rs#L5-L412) |
| System, agent, ISA, pool discovery | [`hsa/src/discovery.rs`](../src/discovery.rs#L14-L884) |
| Session, queue callback, queue ownership | [`hsa/src/session.rs`](../src/session.rs#L22-L409) |
| Signals, allocation, executable, copies, AQL, dispatch | [`hsa/src/execution.rs`](../src/execution.rs#L29-L2305) |
| Error vocabulary | [`hsa/src/error.rs`](../src/error.rs#L3-L316) |
| Probe runtime, identity, and benchmark caller | [`native-probe/src/hsa.rs`](../../native-probe/src/hsa.rs#L67-L875) |
| Exact preparation binding and session lending | [`native-probe/src/bindings.rs`](../../native-probe/src/bindings.rs#L22-L482) |
| Native executor HSA realization and bridge | [`native-executor/src/hsa.rs`](../../native-executor/src/hsa.rs#L31-L2679) |
| Root advanced facade | [`src/facade.rs`](../../src/facade.rs#L17-L41) |

`cargo check -p recipe-hsa` validates Rust and FFI shape. It does not prove
that a ROCr library is installed, that the ABI matches the deployed headers,
that a queue can execute, or that an HSACO computes correctly. Those claims
require the real probe or a complete public workload on the matching AMD
runtime and hardware.
