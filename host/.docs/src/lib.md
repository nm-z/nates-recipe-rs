# `recipe-host`

`recipe-host` is the host-storage backend crate. Its root is intentionally a
small declaration facade in [`host/src/lib.rs`](../../src/lib.rs): it forbids
unsafe code, requires every private implementation type to implement
`Debug`, states the byte-transport-only boundary, and re-exports the types
that the executor and native preparation layers use.

The crate owns preallocated RAM and disk arenas plus an asynchronous copy
runtime. It does not own payload calculations, hardware discovery, planning,
scheduling, kernel compilation, driver loading, or a model/file format. Those
operations belong to other workspace crates. A host task is therefore a
transfer or a four-byte metric readback. A calculation is rejected instead of
being silently run on the CPU. Workers, slots, staging buffers, RAM arenas,
and disk files are realized before the corresponding execution loop begins.

## Root facade and module ownership

The root contains four private modules:

| Module | Root re-exports | Responsibility |
| --- | --- | --- |
| [`arena`](../../src/arena.rs) | `Arena`, `ArenaKind`, `DiskFileSpec` | Owns byte storage, exact range I/O, disk-file allocation, and arena cleanup. |
| [`backend`](../../src/backend.rs) | `HostArenaLookup`, `HostBackend`, `HostBackendConfig`, `HostDeviceBinding`, `HostPending`, `HostPreparedResources`, `HostResources` | Adapts host copies and metric readbacks to `recipe_executor::Backend`, validates finalized task contracts, and coordinates candidate, warm, and bound resource lifecycles. |
| [`error`](../../src/error.rs) | `Error`, `Result` | Defines the non-exhaustive host error vocabulary and the result alias. |
| [`runtime`](../../src/runtime.rs) | `PendingCopy`, `PollStatus`, `Runtime`, `RuntimeConfig`, `RuntimeState`, `SlotCapacity` | Owns the fixed slot table, worker threads, per-worker staging, copy state machine, polling, and shutdown. |

The implementation modules remain private. Callers reach them through the
root names above, or through the `recipe::engine::host` re-export described in
[Workspace consumers](#workspace-consumers). No submodule path is part of the
public crate surface.

The root attributes are part of the boundary:

* `#![forbid(unsafe_code)]` applies to this crate. Host storage and worker
  copying use safe `Mutex`, `Arc`, file-offset I/O, and bounded conversions.
  The separate cross-backend bridge may use unsafe pointer reconstruction, but
  that code is in `native-executor`, not in `recipe-host`.
* `#![deny(missing_debug_implementations)]` keeps public and private runtime
  state inspectable. `Arena`, `HostBackend`, `HostPending`, `HostResources`,
  `HostPreparedResources`, `Runtime`, and `PendingCopy` provide explicit debug
  implementations where their fields include handles or synchronization
  state.

The package is Rust 2024, version `0.1.0`, and depends on `recipe-core` for
`ByteCount`, `DeviceId`, finalized plans, task and transfer identities, on
`recipe-executor` for the sealed backend ABI, and on `rustix` only for
`fallocate` and `statvfs`. The package declaration is in
[`host/Cargo.toml`](../../Cargo.toml).

## Public API

All names below are re-exported directly by the root. Methods marked
`#[doc(hidden)]` are public only so the composite native executor and bridge
can call them. They are integration hooks, not a second user-facing API.

### Arenas and disk specifications

`ArenaKind` is the two-value storage classification, `Ram` or `Disk`.

`DiskFileSpec` is a caller-resolved path. `DiskFileSpec::new(path)` converts
the input to a `PathBuf` and rejects a path with no file name with
`Error::InvalidPath`; it does not open the path or perform I/O. `path()` gives
the borrowed path used later by capacity queries and disk allocation.

`Arena` is a cloneable handle containing a `DeviceId`, an `ArenaKind`, and an
`Arc` to private backing storage.

* `Arena::ram(device, bytes)` requires nonzero bytes, converts the size to
  `usize`, reserves exactly that much host memory, zero-fills it, and returns a
  RAM arena. A size that cannot fit the host address space or cannot be
  allocated is reported as `InvalidConfiguration`.
* `Arena::disk(device, spec, bytes)` requires nonzero bytes, opens the
  caller-supplied path with `create_new`, reserves the extent with
  `fallocate`, and calls `sync_data` before returning. Open, allocation, and
  synchronization failures are `Error::Io` values with an operation label;
  failed creation paths are removed best effort.
* `device()`, `kind()`, and `bytes()` expose the immutable identity and size.
* `close(self)` requires the handle to be the last `Arc` owner. If another
  clone or an in-flight copy still owns the backing, it returns `SlotBusy`.
  RAM close is otherwise a no-op. Disk close synchronizes and removes the
  file.
* `bridge_read_exact` and `bridge_write_exact` are hidden exact-range hooks
  used by the staged CUDA/HSA bridge. They delegate to the same private
  checked read and write operations used by the host backend.

RAM backing is a mutex-protected boxed byte slice. Disk backing contains an
`std::fs::File`, its path, and its fixed byte count. Dropping disk backing
synchronizes and removes its file best effort, so normal ownership cleanup
does not leave run arena files behind. Every range is checked with checked
addition against the backing length. RAM copies lock the slice; disk copies
use positional `read_at` and `write_at` loops and reject unexpected EOF or
zero-length writes.

### Backend configuration and binding

`HostDeviceBinding` maps a Recipe device identity to one host storage origin:

* `Ram { device }` uses process RAM for arenas on that device.
* `Disk { device, arena }` uses the supplied `DiskFileSpec` as the root for
  disk arenas on that device.

`device()` returns the identity independent of storage kind. The binding is
resolved before execution. Disk paths are never discovered, selected, or
changed from inside the loop.

`HostBackendConfig::new(worker_threads, staging_bytes_per_worker, bindings)`
requires both numeric capacities to be nonzero and validates that device IDs
are unique and disk arena paths are globally unique. `worker_threads()`,
`staging_bytes_per_worker()`, and `bindings()` return the configured values.
`available_bytes(device)` checks the matching binding and then measures the
current allocatable capacity: `MemAvailable` from `/proc/meminfo` for RAM, or
available blocks from the binding's parent directory with `statvfs` for disk.
The query is observational and does not reserve capacity.

### Backend and resource handles

`HostBackend` is the sealed `recipe_executor::Backend` adapter for RAM and
disk-only task subsets. Its private state is a one-shot state machine:

```text
Ready(config) -> Prepared(resources) -> Bound
Ready(config) -> Warmed(resources)   -> Bound
Ready(config) ----------------------> Bound
```

`new(config)` creates the `Ready` form without starting workers. The hidden
`from_prepared` and `from_warmed` constructors are used by candidate warming.
`bind_resources` is the direct whole-bundle path. `bind_partition` is the
hidden composite-local path and binds only a selected host task set. Both
consume the current state, so a second bind returns
`Error::BackendState("resources may be bound only once")`.

The hidden partition methods are thin delegation points used by
`native-executor`:

* `prepare_partition` calls `HostResources::prepare_pending`.
* `allocate_partition` calls `HostResources::allocate_arena`.
* `submit_partition` and `submit_loop_partition` call `submit`, with the loop
  form first preparing or rearming the token.
* `poll_partition` and `collect_partition` delegate to the corresponding
  resource methods.
* `release_partition` checks the arena's device, then closes it.
* `destroy_partition` closes all pending state and the worker runtime.
* `prepare_candidate` realizes `HostPreparedResources` from a draft plan,
  reservation ledger, configuration, and selected host task IDs.

`HostArenaLookup` is a hidden allocation-free trait. An implementation must
return only the host `Arena` owned by the requested device. The crate provides
an implementation for `recipe_executor::ArenaSet<'_, Arena>`. The composite
local executor provides another implementation that projects a mixed
host/CUDA/HSA arena map and returns `None` for non-host arenas.

`HostPending` is the host completion token. Its only public accessor is
`task()`. Internally it owns a `PendingCopy`, optional per-task staging arena,
an optional init-image contract, an action (`None`, `Metric`, or `Egress`),
and submitted/terminal state flags.

`HostResources` is the bound resource set. It owns the `Runtime`, indexed
bindings, immutable expected work contracts, the set of currently prepared
tasks, either a deferred pending mode or a pre-final pending pool, and a
poison flag. Its hidden methods define the complete host lifecycle:

| Method | Effect |
| --- | --- |
| `prepare_pending(request)` | Validates task, phase, work class, and submission slots against the finalized contract, then takes the pre-realized token or creates one from the runtime. Admission, metric, and external-egress tasks receive a RAM staging arena. |
| `allocate_arena(layout)` | Uses the binding for `layout.device` and creates a RAM or disk arena of the exact finalized size. |
| `submit(arenas, pending, work)` | Rechecks the immutable contract and queues an exact copy. Init admission copies the image through staging, internal and exit transfers copy device arenas, and metrics copy four bytes. Calculation work returns `UnsupportedWork`. Any submission error poisons the resource. |
| `poll_pending(pending)` | Returns `BackendPoll::Pending`, or on completion returns a typed `MetricValue` for F32/I32 metrics and marks the token terminal. Worker failures poison the resource. |
| `prepare_loop_pending(pending)` / `rearm_pending(pending)` | Reuse a never-submitted token or reset a completed token's slot and staging without allocation. Active or inconsistent states are protocol errors. |
| `collect_exit(pending, work, destination)` | For a terminal external exit, validates the finalized transfer contract and copies staging bytes into caller storage. It is not valid for internal exits or non-egress tokens. |
| `recycle_pending(pending)` | Requires one terminal token, resets it, removes it from the prepared set, and returns it to the pre-final pool. This is required before warm handoff. |
| `available_bytes(device)` | Repeats the binding-specific capacity query. |
| `validate_handoff(bundle, tasks)` | Requires all warm tokens to be recycled, checks bundle devices and task identity, and replaces warm contracts with the final selected contracts. |
| `destroy()` | Drops pending arenas and closes the runtime, joining workers. |

`HostPreparedResources` is hidden because it represents the candidate/warm
handoff rather than an ordinary execution resource. Candidate realization:

1. Verifies selected task IDs exist in the draft and that bindings name draft
   arena devices.
2. Requires an enforced scheduler quota for every bound device.
3. Creates a `Runtime` with one slot per selected task (at least one), its
   workers, per-worker staging, and a pre-realized `HostPending` for every
   selected metric or transfer.
4. Records a `Candidate` handoff marker.

`validate_handoff` checks the finalized bundle, selected tasks, init-image
manifest, and every pending contract, then records the exact bundle identity
and contracts. `bind` consumes only a finalized handoff with the same bundle
identity and task set. `bind_candidate` is the warm path: it accepts only a
candidate marker, validates the final device/task set and admission manifests,
and turns the pre-final pool into `HostResources`. `destroy` drops the pool
and closes workers if preparation fails.

### Runtime and copy tokens

`SlotCapacity::new(value)` rejects zero and `get()` exposes the count.
`RuntimeConfig::new(worker_threads, slots, staging_bytes_per_worker)` rejects
zero workers or zero per-worker staging bytes and exposes all three values.
`RuntimeState` is `Ready`, `Poisoned`, or `Closed`; `PollStatus` is `Pending`
or `Complete`.

`Runtime::new(config)` allocates exactly the configured slot table, chooses
`min(worker_threads, slots)` workers, allocates one staging byte slice per
worker, and starts named `recipe-host-*` threads before returning. It does not
allocate on submission. `config()` returns the immutable configuration,
`state()` observes worker poisoning, `prepare_copy()` claims one unclaimed
slot, and `close()` stops workers and joins every thread. `Drop` performs the
same shutdown best effort. A worker join panic is returned as
`ThreadPanicked`.

`PendingCopy::submit(source, source_offset, destination, destination_offset,
bytes)` validates nonzero, in-bounds ranges on both arenas, stores the owned
backing references in its slot, transitions `Prepared -> Queued`, and wakes a
worker. `poll()` reports queued/running as `Pending`, completed as `Complete`,
or returns the worker's stored error for a failed slot. A token that has not
been submitted, has already been reset, or has an unknown state returns
`InvalidPendingState` or `Poisoned`. Reset is crate-private and is used by the
backend's loop rearm and recycle methods.

Workers scan the fixed slot table, atomically claim queued slots, and execute
one of four combinations: RAM to RAM, RAM to disk, disk to RAM, or disk to
disk. RAM overlap uses `copy_within`; disk-to-disk overlap copies backward when
needed, otherwise it streams through the worker's preallocated staging slice.
No worker allocates or discovers resources. A copy error is stored in the slot
and changes its state to `Failed`; failure to lock the error slot marks the
shared runtime poisoned.

## Work contract and lifecycle

`HostBackend` implements the sealed executor trait with
`Arena = Arena`, `Pending = HostPending`, `Resource = HostResources`, and
`Error = recipe_host::Error`. It reports at most one non-poll physical call per
operation (`MAX_NON_POLL_PHYSICAL_CALLS = 1`). The executor therefore sees the
same lifecycle as every other backend while host-specific byte operations stay
behind this adapter.

The host contract is derived from each finalized task, not from the work value
submitted later. The accepted classes are:

* `InitAdmission`: external image to a device value, with a preallocated
  staging arena sized to the image.
* `InternalTransfer`: device arena to device arena during init or loop, with
  no external staging arena.
* `Metric`: a device value readback of exactly four bytes during the loop;
  completion decodes the bytes according to the finalized F32 or I32 dtype.
* `ExitTransfer`: device arena to device arena or device arena to external
  storage during exit. External egress receives a preallocated staging arena.

The finalized contract also fixes endpoints, byte count, route, lane claims,
submission slots, init-image identity, metric identity and slot, and task
phase. Any mismatch is `Error::Protocol`. Planner-expanded routes longer than
one hop are rejected as `UnsupportedWork` because this host adapter handles
one-hop host copies; the heterogeneous bridge owns cross-backend staging.

The resulting execution order is:

1. Preparation constructs `HostBackendConfig`. Candidate or direct bind checks
   binding uniqueness, bundle device coverage, and enforced reservation quota.
2. Runtime creation and pending-token creation allocate all worker, slot, and
   staging state. `bind_resources` and `prepare_pending` complete before the
   executor enters `init`.
3. Arena allocation creates each finalized host arena. Disk arenas create and
   preallocate their exact run-scoped file at this point, never in the loop.
4. Submission validates the token and immutable task contract, then queues a
   copy. Polling is nonblocking. Completion is the only point where metric
   bytes become a `MetricValue`.
5. Repeated loop iterations call `prepare_loop_pending`, which either uses the
   untouched token or resets its completed slot. This path does not allocate.
6. Exit collection reads completed egress staging into caller-owned output.
   Arenas are released, then resources are destroyed and worker threads join.

The direct backend path binds a whole finalized bundle. The composite local
path binds a selected host partition and lets CUDA, HSA, and cross-backend
lanes own their corresponding tasks. Both paths use the same contract checks
and token state transitions.

## Failure semantics

`Result<T>` is `core::result::Result<T, Error>`. `Error` is `#[non_exhaustive]`,
so downstream matches must include a wildcard. Its variants group as follows:

| Group | Variants and meaning |
| --- | --- |
| Configuration and identity | `InvalidConfiguration`, `InvalidPath`, `DuplicateDevice`, `MissingDevice`, `WrongDeviceKind`, `BackendState`. These reject zero capacities, unusable paths, duplicate or absent bindings, storage mismatches, or invalid one-shot/candidate handoffs. |
| Immutable work contract | `Protocol { task, detail }` reports a task, phase, endpoint, slot, route, manifest, owner, or pending-state mismatch. `UnsupportedWork { task, detail }` rejects calculations and unsupported multi-hop host work. |
| Bounds and fixed reporting | `PhysicalReportOverflow` means the executor's fixed physical-call batch could not accept the host record. `RangeOverflow` means checked offset, size, or integer conversion overflowed. `OutOfBounds { device }` means a copy exceeds its arena. |
| Slot and worker state | `SlotCapacityExhausted`, `SlotBusy`, and `InvalidPendingState` describe fixed-slot ownership or token-state violations. `WorkerFailed { operation, kind }` carries a disk worker I/O failure. `ThreadSpawn`, `ThreadPanicked`, and `Poisoned` cover worker lifecycle, lock, or shared-runtime failure. |
| Direct I/O | `Io { operation, kind }` labels path creation, extent allocation, synchronization, capacity queries, and direct arena reads/writes. |

Host submission and polling fail closed. A submission mismatch or copy error
poisons `HostResources`, so later operations return `Poisoned` instead of
continuing with potentially inconsistent state. A failed worker slot itself
retains its concrete `WorkerFailed` error for `PendingCopy::poll`; a poisoned
mutex or shared wake state escalates to `Poisoned`. Runtime shutdown still
signals stopping, lets queued/running work drain, and joins workers. Disk
creation failures remove partially created files best effort, while normal
`close` and backing drop synchronize and remove disk files. Cleanup errors are
returned where the owning close operation is explicit; best-effort `Drop`
cleanup cannot report an error.

`HostBackend::poll` converts every host error variant to the executor's
`PhysicalPollStatus::Failed` record and returns the original `Error`. It emits
one physical record for bind, prepare, allocation, submit, exit collection,
release, and destroy, and exactly one poll record for each poll. Thus physical
accounting is bounded and does not change the logical task lifecycle.

## Workspace consumers

The host crate is deliberately consumed as an engine component rather than a
user declaration API.

* The root `recipe` crate depends on `recipe-host` and re-exports it as
  [`recipe::engine::host`](../../../src/facade.rs). This keeps the advanced
  engine namespace dependency-clean while the ordinary declaration facade
  remains independent of host implementation details.
* [`src/native_prepare.rs`](../../../src/native_prepare.rs) owns
  `NativeHostPlan`. It turns measured RAM device IDs and storage roots into
  `HostDeviceBinding::Ram` and deterministic run-scoped
  `HostDeviceBinding::Disk` values. `backend_config` performs no I/O; host
  candidate realization later uses `DiskFileSpec` and `create_new`.
* [`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs)
  performs the same conversion directly from a resolved local inventory.
  Probe-derived storage roots and the run ID determine the disk path, while
  this crate validates worker/staging capacities and binding uniqueness.
* [`native-executor/src/local.rs`](../../../native-executor/src/local.rs)
  stores an optional `HostBackendConfig` beside CUDA, HSA, and bridge backends.
  Candidate preparation calls `HostBackend::prepare_candidate`; warm handoff
  uses `HostPreparedResources`; final resources store `Option<HostResources>`.
  Host tasks become `LocalPending::Host(HostPending)`, host arenas become the
  `LocalArena::Host` variant, and every composite `Backend` operation dispatches
  to the host partition. `ProjectedArenas` implements `HostArenaLookup` and
  filters mixed arena maps so a host task cannot accidentally access a CUDA or
  HSA arena. Warm and candidate cleanup call `destroy` on host resources before
  preserving the original composite error.
* [`native-executor/src/bridge.rs`](../../../native-executor/src/bridge.rs)
  aliases `recipe_host::Arena` as `HostArena` for its pre-realized staged
  one-hop bridge. Its worker jobs call the hidden exact-range bridge methods
  for host reads and writes. `StagedBridgeError::Host` preserves the original
  host error as a source, so bridge failures are not converted into a second
  storage implementation.
* `native-executor/Cargo.toml` and `native-probe/Cargo.toml` declare the direct
  path dependency. The workspace root also lists `host` as a member and
  depends on it for the engine re-export. No other workspace crate imports the
  private host modules directly.

The practical ownership boundary is therefore: preparation selects origins and
configuration, the host crate realizes and owns byte storage plus fixed
asynchronous copy resources, `recipe-executor` drives the sealed lifecycle,
and `native-executor` composes the host lane with GPU and bridge lanes.
