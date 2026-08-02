# `recipe_host::backend`

```yaml
document: recipe_host.backend
source: host/src/backend.rs
kind: preallocated-host-backend
authority:
  - host/src/backend.rs
  - host/src/runtime.rs
  - host/src/arena.rs
  - host/src/error.rs
  - executor/src/backend.rs
  - executor/src/executor.rs
  - native-executor/src/local.rs
  - native-executor/src/bridge.rs
  - core/src/schedule.rs
  - core/src/plan.rs
  - scheduler/src/static_schedule.rs
```

This document follows the host backend from a caller-resolved RAM or disk
binding to a finalized Recipe task, a preallocated pending copy, worker
completion, and final output or teardown. The implementation is a byte
transport adapter. It owns host storage, staging, and asynchronous copy
completion, but it does not own graph semantics, scheduling, kernel loading,
GPU calculation, discovery, or dynamic route selection.

The host backend is deliberately narrower than the executor's closed backend
ABI. It accepts init admission, internal transfer, metric readback, and exit
transfer work that is assigned to host-owned devices. A `Calculation` work
item is rejected with `UnsupportedWork`; calculation payloads belong to the
CUDA or HSA adapters. In a heterogeneous local run, `LocalBackend` partitions
tasks before dispatch, so host work and GPU calculation work share one
executor lifecycle without sharing an implementation.

Source line references below anchor the current tree. The type and branch
names are the contract. The line numbers identify the checks that enforce it.

## 1. Position in the execution architecture

The full production path is:

```text
measured host inventory and tuning
  -> HostBackendConfig { worker count, staging capacity, bindings }
  -> LocalCandidateFactory::realize_candidate
  -> HostBackend::prepare_candidate (optional pre-final path)
  -> HostPreparedResources { Runtime, pending tokens, handoff = Candidate }
  -> bind_candidate against a provisional finalized bundle
  -> maximum-concurrency warm trace
  -> recycle every terminal host token and release warm arenas
  -> final handoff validation
  -> HostBackend::from_warmed
  -> LocalBackend::bind_resources
  -> executor::PreparedRun::prepare
     -> host bind partition
     -> host pending preparation for every host task
  -> InitializedRun::initialize
     -> host Arena allocation
     -> init admission copies
  -> RunningRun loop
     -> host internal copies and metric readbacks
     -> terminal token rearm for the next iteration
  -> ExitedLoop::exit
     -> host exit transfers and external collection
     -> arena release
     -> Runtime shutdown and disk-file removal
```

The ordinary direct `HostBackend::new` path skips candidate warming. It still
binds resources and prepares all pending tokens before `init`, and it still
allocates every arena before the loop. The candidate path exists so native
preparation can realize and measure the same host resources before Finalize.

The executor's typestate handles are the outer authority:

```text
PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop -> ExitedRun
```

`PreparedRun::prepare` binds one `Resource`, realizes one pending token per
finalized task, and records `Prepared` only after all three phase token pools
exist (`executor/src/executor.rs:795-977`, `2085-2122`). `initialize` allocates
the finalized arena layouts and drives `Init` (`executor/src/executor.rs:980-1035`).
`RunningRun::poll` submits and polls only the active loop phase, while
`ExitedLoop::exit` drives `Exit`, collects external images, then releases
arenas and destroys resources (`executor/src/executor.rs:1072-1370`). The host
implementation is therefore never asked to discover a path or allocate an
executor-visible object from the live loop.

## 2. Public host backend surface

### 2.1 Caller-resolved bindings and configuration

`HostDeviceBinding` is a closed association between one `DeviceId` and one
host storage origin (`host/src/backend.rs:30-53`):

```text
HostDeviceBinding::Ram  { device }
HostDeviceBinding::Disk { device, arena: DiskFileSpec }
```

The binding does not probe the machine and does not choose a path. A disk path
is selected by preparation. `DiskFileSpec::new` rejects a path without a file
name, and the arena later opens that exact path with `create_new`
(`host/src/arena.rs:20-36`, `196-223`). `HostDeviceBinding::device()` is a
constant-time projection used by every binding and ownership check.

`HostBackendConfig` stores three policy values (`host/src/backend.rs:55-93`):

| field | meaning | validation |
| --- | --- | --- |
| `worker_threads` | upper bound on host copy workers | nonzero |
| `staging_bytes_per_worker` | one fixed staging byte buffer per runtime worker | nonzero |
| `bindings` | one caller-resolved RAM or disk origin per host device | device IDs unique, disk paths globally unique |

`HostBackendConfig::new` performs all three checks before the config can enter
a backend. `available_bytes(device)` requires a configured device and queries
the current capacity without realizing a candidate. RAM uses the numeric
`MemAvailable` field in `/proc/meminfo`, multiplying KiB by 1024 with checked
arithmetic. Disk uses `statvfs` on the configured path's parent and returns
`f_bavail * f_frsize` with checked arithmetic (`backend.rs:95-107`,
`1355-1388`). These observations are capacity evidence, not a replacement for
the finalized reservation and arena contract.

The production configuration producers are intentionally outside this crate:

* `native-probe/src/bindings.rs:87-118` maps the exact resolved RAM and
  storage inventory to `HostDeviceBinding` values. Storage paths are
  deterministic run-scoped children of the measured benchmark root. Creating
  the `DiskFileSpec` performs no I/O; `create_new` is deferred to resource or
  arena realization.
* `src/native_prepare.rs:141-163` performs the same mapping for an owned
  `NativeHostPlan`.
* `src/training.rs:1011-1275` derives worker count and staging capacity from
  measured host lanes, graph opportunities, maximum transfer bytes, local RAM,
  and the measured RAM or storage buffer scale. `src/training.rs:1303-1313`
  passes those values into `HostBackendConfig` and `LocalCandidateFactory`.
* `src/inference.rs:602-629` follows the same measured path for inference.

A binding can be valid as a config and still be rejected at candidate or final
handoff if its device is absent from the candidate or finalized arena layouts,
its reservation is missing, or its reservation mechanism is not an enforced
quota.

### 2.2 Backend typestate

`HostBackend` contains exactly one `BackendState` (`backend.rs:110-126`):

```text
Ready(HostBackendConfig)
Prepared(HostPreparedResources)
Warmed(HostResources)
Bound
```

Constructors select the first three states (`backend.rs:128-150`). The
partition bind path replaces the state with `Bound` before realization
(`backend.rs:158-168`). The direct trait implementation records its
`BindResources` physical call first, then replaces the state before resource
realization (`1130-1148`); only a physical-report overflow can fail before that
replacement. This makes a successfully entered bind one-shot even when
realization fails. The state transition is intentional: a failed bind cannot
be retried against a different bundle while retaining partly consumed config
or prepared native resources.

`HostBackend` has two layers of methods:

1. The hidden partition methods (`backend.rs:171-248`) are used by
   `LocalBackend`. They forward to `HostResources` and deliberately do not
   append physical-call records. The composite local backend emits one record
   for the enclosing operation, avoiding duplicate accounting.
2. The `recipe_executor::Backend` implementation (`backend.rs:1121-1284`)
   records one physical call and then invokes the same resource operation.
   This is the direct host-only executor path.

`HostArenaLookup` is the allocation-free read-only arena projection
(`backend.rs:253-263`). The direct implementation accepts
`ArenaSet<'_, Arena>`. The heterogeneous local implementation uses
`ProjectedArenas` (`native-executor/src/local.rs:2113-2125`), which returns an
arena only when the requested device is a `LocalArena::Host`. A CUDA or HSA
arena therefore cannot be accidentally passed to a host copy.

### 2.3 Resource and token records

`ExpectedWork` is the immutable per-task contract retained by finalized host
resources (`backend.rs:265-345`):

```text
Init {
    image: ValueId,
    destination: ResolvedValueLocation,
    bytes: ByteCount,
    submission: SubmissionSlots,
}
Transfer {
    class: WorkClass,
    source: ResolvedTransferEndpoint,
    destination: ResolvedTransferEndpoint,
    bytes: ByteCount,
    route: Vec<LinkId>,
    lane_claims: Vec<TransferLaneClaim>,
    submission: SubmissionSlots,
}
Metric {
    metric: MetricId,
    slot: MetricSlotId,
    value: ResolvedValueLocation,
    submission: SubmissionSlots,
}
```

Each variant derives its `WorkClass`, optional submission slots, optional init
image identity, and staging requirement. `Init` always gets a RAM staging
arena on the destination device. A device-to-external `ExitTransfer` gets a
RAM staging arena on the source device. A `Metric` gets a four-byte RAM
staging arena on the value's device. Device-to-device transfers use no host
staging at this layer. `PendingAction` records whether completion does
nothing, decodes a metric, or leaves a staged egress payload for explicit
collection (`backend.rs:347-359`).

`HostPending` is one reusable completion/submission token (`backend.rs:361-391`):

```text
task: TaskId
class: WorkClass
copy: PendingCopy
staging: Option<Arena>
admission: Option<InitImageContract>
action: PendingAction
submitted: bool
terminal: bool
```

`submitted` and `terminal` are host-side lifecycle bits around the runtime
slot. A token can be submitted only when both are false. A successful poll
changes it to `(true, true)`. A loop rearm or warm recycle resets it to
`(false, false)` after resetting the runtime slot from `Complete` to
`Prepared`. `HostPending::task()` is the only public identity projection.

`HostResources` owns the live runtime and finalized contracts
(`backend.rs:393-406`):

```text
runtime: Runtime
bindings: BTreeMap<DeviceId, HostDeviceBinding>
contracts: BTreeMap<TaskId, ExpectedWork>
prepared: BTreeSet<TaskId>
pending: Deferred | Prepared(BTreeMap<TaskId, HostPending>)
poisoned: bool
```

`Deferred` is the direct-final state. `Prepared` is the warm or finalized
candidate state and contains one pre-realized token per selected host task.
`prepared` tracks tokens checked out of that pool. `poisoned` is a fail-closed
resource bit set by submit or runtime poll failures.

`HostPreparedResources` is the pre-final owner (`backend.rs:408-424`). It has
one preallocated pending map and a handoff state:

```text
Candidate
Finalized {
    bundle: BundleIdentity,
    tasks: BTreeSet<TaskId>,
    contracts: BTreeMap<TaskId, ExpectedWork>,
}
```

A candidate can be converted to warm `HostResources`, or it can be validated
once for a finalized bundle and then consumed by `HostPreparedResources::bind`.
A candidate cannot be bound as finalized without this handoff evidence.

## 3. Contract construction and validation

The host backend repeats the relevant bundle checks at its own boundary. This
is not a second scheduler. It protects a backend token from mismatched work
if a caller bypasses or misuses a composite boundary.

### 3.1 Device, reservation, and ownership checks

`validate_bindings` rejects duplicate device IDs and duplicate disk paths
(`backend.rs:1286-1303`). `index_bindings` repeats that validation while
consuming the vector into an ordered map (`1305-1311`).

`validate_bundle_devices` compares binding keys with finalized arena-layout
device IDs (`1313-1338`):

* direct full-bundle realization requires every finalized layout device to have
  a host binding and rejects any extra binding;
* a selected heterogeneous partition requires every configured host binding to
  appear in the bundle, but it does not require host bindings for GPU-owned
  layouts.

`validate_reservations` requires one reservation ledger entry for every host
binding and then calls `require_enforced_quota`. `HeldAllocation` is rejected;
the host backend requires `ReservationMechanism::EnforcedQuota`
(`1340-1353`). The scheduler's reservation contract is therefore checked
before host runtime threads, pending slots, or disk arenas become part of a
finalized resource set.

### 3.2 Final task contracts

`task_contracts(bundle, selected)` walks the immutable finalized task list and
optionally keeps only the selected partition (`backend.rs:1596-1691`). It
rejects a `TaskKind::Calculation` immediately with
`UnsupportedWork`. For a metric it resolves the value location and retains
metric ID, slot, dtype-bearing location, and submission slots.

For transfers it obtains `FinalizedBundle::transfer_endpoints(task)`, maps the
phase and resolved endpoint pair through `transfer_class`, and then checks the
exact init or transfer contract:

| finalized shape | host class | retained contract |
| --- | --- | --- |
| `Init`, `External -> Device` | `InitAdmission` | destination location, init image value, exact bytes, submission slots |
| `Init` or `Loop`, `Device -> Device` | `InternalTransfer` | both resolved locations, bytes, route, lane claims, submission slots |
| `Exit`, `Device -> Device` | `ExitTransfer` | both resolved locations, bytes, route, lane claims, submission slots |
| `Exit`, `Device -> External` | `ExitTransfer` | source location, bytes, route, lane claims, submission slots |

An init task must have a finalized manifest for the destination device, and
`manifest.image == destination.value` and `manifest.bytes == transfer.bytes`
must both hold (`1629-1655`). Internal and exit routes longer than one link
return `UnsupportedWork` (`1657-1672`). A duplicate task ID is a protocol
failure (`1683-1688`).

The phase matrix is repeated by `phase_accepts` (`1717-1726`) when a pending
request is prepared:

```text
InitAdmission: Init only
InternalTransfer: Init or Loop
Calculation and Metric: Loop only
ExitTransfer: Exit only
```

The core plan and scheduler already enforce these shapes. Core validation
rejects external admission outside `Init`, external egress outside `Exit`,
external routes that contain internal links, internal routes longer than one
directed link, mismatched value devices or byte counts, and incomplete lane
claims (`core/src/plan.rs:634-766`, `1022-1140`). The scheduler expands longer
paths into dependency-chained hop tasks and persists one exact claim per
selected link or external lane (`scheduler/src/static_schedule.rs:14-23`,
`245-447`). The host checks remain necessary because the backend receives
resolved values and can fail closed without trusting an unchecked caller.

### 3.3 Candidate contracts

Before final addresses exist, `prepare_candidate_pending` derives the same
shape from `DraftPlan` (`backend.rs:1390-1500`). It builds a value map from
`ValueSpec` records, selects only the caller-provided task IDs, and rejects a
calculation in the host partition. A metric gets four-byte staging using the
value's dtype. A transfer is classified by the draft's logical endpoint pair
and phase by `candidate_transfer_class` (`1513-1550`).

`candidate_transfer_staging` mirrors the finalized staging matrix
(`1552-1594`). An init admission must be external to device and must match the
draft's `InitDataImage` manifest. Device-to-device internal and exit transfers
do not allocate a host staging arena. Invalid phase or endpoint combinations
are protocol failures before a copy slot is claimed.

Every accepted candidate task then calls `Runtime::prepare_copy`, allocates any
required RAM staging arena, and stores a `HostPending` with both lifecycle bits
clear. Thus a warm candidate's pending map contains real copy slots and real
staging allocations, not placeholders.

## 4. Resource realization and handoff

### 4.1 Direct finalized realization

`HostResources::realize` calls `realize_scoped` with no task filter
(`backend.rs:638-674`). The function indexes bindings, requires complete
device coverage, validates enforced quotas, derives all host task contracts,
creates a `RuntimeConfig`, and starts the runtime workers. The slot capacity is
`contracts.len().max(1)`, so an empty host partition still has a valid runtime
with one slot. `pending` starts as `Deferred`; the per-task copy slot and
staging allocation are created by `prepare_pending`, not by bind.

`HostResources::realize_partition` uses the same steps with a selected task set
and permits the rest of the bundle to be owned by CUDA, HSA, or a bridge
(`643-674`). This is the path used by `LocalBackend::bind_resources` after
immutable task ownership has been classified.

### 4.2 Candidate realization

`HostPreparedResources::realize` performs candidate-only checks and then
realizes all selected host pending tokens (`backend.rs:452-500`):

1. every selected task ID must occur in `DraftPlan.tasks`;
2. each host binding device must occur in `draft.arena_objects`;
3. each host binding must have an enforced-quota reservation;
4. runtime slots are sized to the selected task count, with a minimum of one;
5. `prepare_candidate_pending` validates draft task shape, allocates runtime
   copy slots, and allocates init, metric, or egress staging arenas.

If candidate pending preparation fails, the newly created runtime is closed
before the error is returned (`488-493`). The returned state is
`HostPreparedHandoff::Candidate`.

### 4.3 Warm activation and final validation

`LocalCandidateFactory::realize_candidate` calls
`HostBackend::prepare_candidate` for `partitions.host`
(`native-executor/src/local.rs:1422-1547`). The host prepared resource is
retained alongside CUDA, HSA, and cross-backend prepared resources.

The production warm path constructs a provisional `FinalizedBundle`, then
calls `HostPreparedResources::bind_candidate` (`local.rs:1073-1221`). This
consumes the candidate wrapper into `HostResources` with a `Prepared` pending
pool. It rejects a previously finalized handoff, mismatched bundle device
coverage, mismatched task keys, a host calculation, or an init admission whose
prepared image contract differs from the provisional finalized manifest
(`backend.rs:542-588`).

`run_warm_trace` executes `Init`, one loop iteration, and `Exit` using these
same host operations (`native-executor/src/local.rs:2844-3023`). For each
runnable host task it checks out a token from the prepared pool, submits it,
polls it to terminal state, collects an external exit when present, and calls
`recycle_pending` to return the token to the exact pool. Warm arenas are
allocated once per pass and released after the pass, so capacity observation
includes a production-shaped allocate, copy, poll, collect, and free cycle
without leaving candidate arenas alive into Finalize.

`HostResources::validate_handoff` requires that all warm tokens have been
recycled, that the pending pool's task keys equal the final host partition,
that finalized contracts can be rebuilt, and that every prepared init contract
still matches the final manifest (`backend.rs:1076-1109`). The host resources
then retain the final contracts for execution.

`HostPreparedResources::validate_handoff` is the parallel one-shot handoff for
callers that keep a `HostPreparedResources` wrapper instead of converting it
with `bind_candidate` (`backend.rs:590-625`). It changes `Candidate` to
`Finalized { bundle identity, selected task IDs, contracts }`. The consuming
`bind` method accepts only that finalized state and exact bundle/task identity;
trying to bind a candidate directly or to a different final bundle returns a
`BackendState` error (`503-539`).

In the native local production flow, `LocalPreparedSession::into_backend`
moves the warmed `HostResources` into `HostBackend::from_warmed`
(`native-executor/src/local.rs:1262-1314`). The later composite
`LocalBackend::bind_resources` call invokes `HostBackend::bind_partition`,
which validates the final handoff and returns the same resources without
reallocating host runtime state (`local.rs:1657-1701`,
`backend.rs:158-168`).

## 5. Pending token lifecycle

### 5.1 Preparation before `init`

`HostResources::prepare_pending` is called by the executor's
`realize_phase` for every task in `Init`, `Loop`, and `Exit`
(`executor/src/executor.rs:2085-2122`). It first checks health, finds the
finalized contract, compares task class and submission slots, and checks that
the requested phase accepts the contract (`backend.rs:684-705`). The same task
cannot be prepared twice.

In a prepared candidate pool, the method removes the exact pre-realized token
by task ID and verifies task, class, and init admission identity
(`706-723`). In direct finalized resources, it calls `Runtime::prepare_copy`,
derives staging from `ExpectedWork::staging`, and creates a new token with
clear lifecycle bits (`725-740`). Staging allocations therefore complete
before the executor enters `init`; no staging `Arena` is created by a loop
submission.

### 5.2 Submission guard and work dispatch

`HostResources::submit` is the single host submission operation
(`backend.rs:758-902`). It checks health, token identity, token state, task
contract identity, and submitted class before dispatch. The successful path
sets `pending.submitted = true`. Any dispatch error sets `self.poisoned = true`
and returns the original error. A failed submission therefore cannot be
retried through the same resource set.

`validate_transfer` compares every immutable transfer field, not only source
and destination (`1759-1775`): class, resolved endpoints, byte count, route,
lane claims, and `SubmissionSlots` must all match. `checked_arena` obtains only
the host arena for the resolved device and checks `arena_offset + bytes` with
checked arithmetic before the runtime sees the copy (`1728-1747`).

The work variants are dispatched as follows.

#### Init admission

`ExpectedWork::Init` must pair with `BackendWork::InitAdmission`
(`784-822`). The backend checks task destination value and full resolved
location, exact byte count, submission slots, image length as `u64`, and the
stored init manifest contract. It writes the caller-provided packed image to
the token's preallocated staging arena, then submits one copy from staging
offset zero to the resolved destination arena offset.

The caller's image is not retained after `submit`. The worker job owns cloned
`Arc<Backing>` values, while the bytes themselves are copied into the staging
arena before the job is queued. The physical report is one
`AdmissionChunk { chunk_index: 0 }`, because host admission is one complete
packed image rather than a lazily selected stream of chunks.

#### Internal and exit device-to-device transfers

An `ExpectedWork::Transfer` paired with `BackendWork::InternalTransfer` or
`BackendWork::ExitTransfer` is checked by `validate_transfer` and then passed
to `submit_transfer` (`823-846`, `1777-1816`). For two device endpoints the
runtime copies directly from the source arena offset to the destination arena
offset. No per-token staging arena is allocated. An exit transfer whose
destination is another host device remains an ordinary direct copy; only an
external destination requires egress collection.

#### External exit transfer

A device-to-external exit transfer is submitted as a source-arena-to-staging
copy. The preallocated staging arena is required and the destination offset is
always zero (`1794-1806`). Completion does not copy into caller memory yet;
`PendingAction::Egress` records that the later `collect_exit` call must read
the staging arena.

#### Metric readback

`ExpectedWork::Metric` must pair with `BackendWork::Metric`
(`847-877`). The backend checks metric ID, metric slot, resolved value
location, and submission slots. It bounds-checks exactly four bytes at the
device value's arena offset, copies those bytes to the preallocated metric
staging arena, and marks the action with the value's `DType`.

#### Calculation

Any `BackendWork::Calculation` is rejected with
`UnsupportedWork { detail: "payload calculations require a CUDA or HSA GPU
adapter" }` (`878-883`). This check is fail closed even if a caller reaches a
host backend with a calculation work value. The host adapter never interprets
kernel templates, artifacts, inputs, outputs, or fault flags.

### 5.3 Polling and completion

`HostResources::poll_pending` is nonblocking (`backend.rs:904-939`). It first
requires a submitted, nonterminal token. `PendingCopy::poll` maps queued or
running runtime states to `BackendPoll::Pending`, and a complete runtime slot
enters the action-specific completion branch.

* `PendingAction::None` is a completed admission or internal transfer and
  yields `BackendPoll::Complete { metric: None }`.
* `PendingAction::Egress` is a completed source-to-staging copy and also yields
  no metric. The staging bytes remain available for `collect_exit`.
* `PendingAction::Metric` reads exactly four bytes from staging and decodes
  little-endian `f32` or `i32` according to the finalized value dtype. The
  result is `MetricValue::F32` or `MetricValue::I32`.

On successful completion `terminal` becomes true. A runtime poll failure sets
`poisoned = true` and returns the worker or runtime error. A staging read
failure or malformed action is returned directly; it does not invent a
second completion path.

The executor maps this result to one physical `Poll` record with
`Pending`, `Complete`, or `Failed` status (`backend.rs:1204-1243`). Its journal
retains the first pending marker and exact per-task pending count, while
terminal polls remain ordered (`executor/src/executor.rs:479-573`). Thus a
slow host disk does not cause an unbounded journal allocation.

### 5.4 Loop repetition and warm recycling

`rearm_pending` is the only in-place loop reset (`backend.rs:941-954`). It
requires a terminal submitted token, resets the runtime slot from `Complete` to
`Prepared`, and clears the host lifecycle bits. `prepare_loop_pending` accepts
either a never-submitted token or a terminal token, and rejects active or
inconsistent states (`956-971`). The global loop iteration is intentionally
ignored by the host resource because token reuse is based on the token's own
terminal state.

The partition wrapper `submit_loop_partition` and the `Backend` implementation
both call `prepare_loop_pending` before the loop submission
(`197-207`, `1190-1202`). A sparse loop task whose first activation is not
iteration zero still receives the same zero-based `LoopIteration` in its
`BackendWork`; host rearming depends only on local token state.

Warm execution uses `recycle_pending` rather than leaving tokens in phase
slots (`backend.rs:1035-1062`). It requires a terminal token and a matching
checked-out task in `prepared`, resets the runtime slot, clears lifecycle bits,
and reinserts the token into `PendingResources::Prepared`. Recycling into
direct `Deferred` resources is rejected because direct finalized runs never
own a warm pending pool.

### 5.5 Exit collection

`collect_exit` is read-only with respect to the pending token and runs only in
`Exit` (`backend.rs:973-1033`). It checks host health, destination slice length,
task identity, terminal state, `PendingAction::Egress`, and byte count. It
reconstructs the finalized transfer expectation and validates every transfer
field again. The work destination must be `External`, a staging arena must
exist, and the staging bytes are read exactly into the caller-owned slice.

The executor allocates the output vector before invoking the backend and stores
the resulting `ExitImage { task, source, bytes }` in its fixed result slots
(`executor/src/executor.rs:2596-2665`). The host backend never chooses the
output path and never publishes an exit image by itself.

## 6. Host memory, disk, and copy behavior

The backend delegates physical storage to `Arena` and asynchronous byte copies
to `Runtime`, but its contracts determine when those resources are created and
which ranges are legal.

### 6.1 Arena realization

`HostResources::allocate_arena` looks up the finalized layout device in the
binding map (`backend.rs:745-756`). A RAM binding calls `Arena::ram`; a disk
binding clones its configured `DiskFileSpec` and calls `Arena::disk`.

`Arena::ram` rejects zero bytes, converts the requested count to `usize`,
reserves exactly that many bytes, zero-fills a boxed slice, and protects it
with a `Mutex` behind an `Arc` (`host/src/arena.rs:88-117`).

`Arena::disk` creates the exact configured path with `OpenOptions::create_new`,
allocates the requested extent with `fallocate`, and calls `sync_data`. A
failed extent or sync removes the newly created file before returning the
error (`arena.rs:119-129`, `196-223`). This prevents an overlapping run from
overwriting an existing arena file.

An arena carries its immutable `DeviceId`, `ArenaKind`, and backing length.
`Arena::close` requires unique ownership of the backing `Arc`; if pending jobs
or another arena clone still retain it, `Arc::try_unwrap` returns `SlotBusy`.
Disk close syncs and removes the file. `Backing::Drop` also attempts sync and
removal, but deliberately ignores cleanup errors (`arena.rs:79-85`,
`141-144`, `225-234`).

`Arena::read_exact` and `write_exact` validate `offset + byte_count <= backing
length` with checked arithmetic. RAM accesses lock the storage and copy the
exact slice. Disk accesses use `FileExt::read_at` and `write_at` loops that
reject unexpected EOF or zero-byte writes (`arena.rs:146-182`, `236-291`).
These methods expose `bridge_read_exact` and `bridge_write_exact` for the
staged cross-backend bridge. Those bridge calls use the same host backing but
are not host backend pending tokens.

### 6.2 Runtime slot ownership

`Runtime::new` preallocates a fixed `JobSlot` table and one staging buffer per
worker (`host/src/runtime.rs:128-187`). The worker count is
`min(worker_threads, slots)`. Every worker is named `recipe-host-{index}` and
receives one fixed `staging_bytes_per_worker` buffer. Thread-spawn failure
stops and joins already-created workers before returning `ThreadSpawn`.

`Runtime::prepare_copy` requires a ready, non-poisoned runtime, atomically
claims the next unclaimed slot, and returns a `PendingCopy` holding `Arc`
references to the slot and shared worker state (`runtime.rs:190-223`). The
slot index is monotonic. There is no allocation-free general slot search or
implicit replacement: a capacity miss is `SlotCapacityExhausted`, and a slot
that is no longer unclaimed is `SlotBusy`.

Each `PendingCopy` transitions through this fixed state machine:

```text
UNCLAIMED -> PREPARED -> QUEUED -> RUNNING -> COMPLETE
                                     \-> FAILED
```

`submit` requires `PREPARED`, rejects zero-byte copies, checks both ranges,
stores a job with cloned source and destination backings, changes the state to
`QUEUED`, and wakes one worker (`runtime.rs:259-300`). `poll` reports queued or
running as pending, complete as complete, and failed as the stored worker
error. Polling an unclaimed or merely prepared slot is `InvalidPendingState`
(`302-313`). `reset` is legal only from `COMPLETE`; it clears the job and
failure cells and returns the slot to `PREPARED` (`315-325`).

The backend's `submitted` and `terminal` bits are therefore a semantic layer
over the runtime slot state. The runtime owns physical queueing and worker
execution. The host backend owns task identity, finalized contract checks,
staging action, and executor-facing completion values.

### 6.3 Worker copy cases

A worker scans queued slots, atomically claims each one as `RUNNING`, takes its
job, and executes it. Success stores `COMPLETE`; a copy or lock failure stores
the error and stores `FAILED` (`runtime.rs:374-427`). Workers wait on a
generation counter and condition variable when no slot progressed. During
shutdown they finish all queued or running jobs before exiting. A panic while
joining workers is surfaced as `ThreadPanicked` by `Runtime::shutdown`.

`execute_copy` covers all four backing combinations (`runtime.rs:429-478`):

| source | destination | operation |
| --- | --- | --- |
| RAM | RAM | lock both backings and copy slices; same backing uses overlap-safe `copy_within` |
| RAM | disk | lock source, then `write_at` the source range |
| disk | RAM | lock destination, then `read_at` into the destination range |
| disk | disk | chunk through one worker staging buffer |

Distinct RAM backings are locked by the address order of their `Arc` pointers,
so two opposite copies cannot deadlock on lock acquisition. A same-backing
RAM copy uses `copy_within`, preserving overlap semantics.

Disk-to-disk copies chunk through the worker buffer. For an overlapping copy
within one disk backing where destination starts inside the source at a higher
offset, chunks are read and written backward. All other copies run forward.
Each chunk uses checked offsets and the same exact-read and full-write loops as
arena I/O (`runtime.rs:480-599`). The worker staging buffer is never exposed as
a Recipe arena and is allocated once before the run.

## 7. Direct Backend ABI implementation

`impl recipe_executor::Backend for HostBackend` binds the host types and
declares `MAX_NON_POLL_PHYSICAL_CALLS = 1` (`backend.rs:1121-1128`):

```text
Arena    = recipe_host::Arena
Resource = HostResources
Pending  = HostPending
Error    = recipe_host::Error
```

The executor ABI requires bind, pending preparation, arena allocation,
submission, polling, exit collection, release, and destroy to operate on
pre-realized resources. The trait is sealed and its non-poll operation bound
is part of journal capacity (`executor/src/backend.rs:318-450`). Host methods
satisfy that boundary by allocating in bind, pending preparation, or arena
allocation and keeping submit and poll to fixed-size fields, fixed staging,
and existing runtime jobs.

 Host advertises `supports_loop_repetition() == true` (`backend.rs:1176`), so
 one prepared token and one staging allocation can serve every finalized loop
 iteration. It does not override the trait's
 `supports_same_queue_pipelining` default (`executor/src/backend.rs:372-380`),
 and therefore host tasks are not submitted behind incomplete predecessors on
 the same queue. The executor's scheduler waits for the ordinary completion
 dependency or schedule-window condition. In a composite local run, only CUDA
 owners opt into same-queue pipelining (`native-executor/src/local.rs:1797-1801`);
 a host token never inherits that GPU exception.

### Physical records

The implementation records exactly one physical call before each non-poll
operation (`backend.rs:1130-1284`):

| operation | record |
| --- | --- |
| bind | `BindResources` |
| prepare pending | `PreparePending { task }` |
| allocate | `AllocateArena { device, bytes }` |
| submit init | `AdmissionChunk { task, device, bytes, chunk_index: 0 }` |
| submit calculation | `SubmitCalculation { task }`, although the resource then rejects it |
| submit internal | `SubmitInternalTransfer { task }` |
| submit metric | `SubmitMetric { task, slot }` |
| submit exit | `SubmitExitTransfer { task }` |
| poll | one `Poll { task, status }` |
| collect | `CollectExit { task, bytes }` |
| release | `ReleaseArena { device }` |
| destroy | `DestroyResources` |

`record` uses `PhysicalCallBatch::try_push`. If a caller supplies a full
batch, the host returns `PhysicalReportOverflow` instead of growing or
allocating a report (`backend.rs:1840-1848`). Poll maps every host error to
`PhysicalPollStatus::Failed` and still attempts to append the one poll record.
The fixed report and `MAX_NON_POLL_PHYSICAL_CALLS = 1` let
`JournalCapacity::for_bundle` preallocate exact physical journal capacity
(`executor/src/executor.rs:248-347`). Repeated loop calls and pending polls
are compacted by task-indexed counters rather than by unbounded vectors.

Partition methods intentionally omit these records. `LocalBackend` records
one composite `BindResources`, `PreparePending`, submission, poll, collect,
release, or destroy action and then forwards to the host child
(`native-executor/src/local.rs:1657-2110`). A host child must not double-count
the same logical local operation.

## 8. Composite local callers and ownership boundary

`LocalCandidateFactory` classifies every candidate device as Host, Cuda, or
HSA and every task as Host, Cuda, HSA, or Bridge
(`native-executor/src/local.rs:2291-2393`). The task-owner rules are:

* a calculation is Cuda or HSA only; a host-owned calculation is rejected;
* a metric follows the owner of its value device;
* a host-to-host device transfer is Host;
* a same-device CUDA or same-class HSA transfer remains native;
* a transfer crossing host, CUDA, HSA, or distinct CUDA devices is Bridge;
* an external admission or egress follows its endpoint device owner.

A route with more than one link is rejected before ownership partitioning. A
bridge task must contain exactly one candidate link. Queue and completion slots
cannot be shared by different local owners (`local.rs:2512-2558`). This is why
a host `ExpectedWork::Transfer` can retain the route and lane claims without
implementing a route walker.

At final bind, `LocalBackend::bind_resources` records one composite bind,
classifies the finalized bundle, binds the selected host task set through
`HostBackend::bind_partition`, and binds CUDA, HSA, and bridge partitions
separately (`local.rs:1647-1701`). `LocalPending::Host` carries the
`HostPending`; owner checks prevent a host token from being submitted for a
GPU or bridge task (`local.rs:1703-1867`).

The host projection used by the composite is deliberately narrower than the
complete `LocalArena` map. `ProjectedArenas::host_arena` returns `None` for
CUDA, HSA, or unknown devices. A host transfer therefore fails with
`MissingDevice` or `OutOfBounds` instead of silently touching another backend's
memory.

The cross-backend `StagedCrossBackend` is separate from `HostBackend`. It may
read or write a host `Arena` through `bridge_read_exact` and
`bridge_write_exact`, but it owns its own native staging allocations,
per-task worker, and source or destination completion tokens
(`native-executor/src/bridge.rs:775-867`, `1066-1291`). A bridge task cannot
have an external endpoint, so host egress collection never receives a bridge
pending token (`local.rs:1979-2023`). Host backend documentation must not
treat bridge worker staging as host backend `PendingAction` staging.

## 9. Outputs and externally visible state

### 9.1 Metrics

The host backend returns a `MetricValue` only for a completed metric task. The
executor's `complete_slot` publishes that value to the metric mailbox for
`MetricPurpose::User`, or interprets an `I32` fault readback as a checked
device-fault result (`executor/src/executor.rs:2496-2594`). A zero fault code
records `FaultChecked`; a nonzero code becomes `DeviceFault`; an F32 fault
readback is a backend protocol failure. The host layer only transports and
decodes the four bytes. It does not decide whether the metric is user-facing
or a fault check.

### 9.2 External outputs

A completed host egress first exists in the per-token staging arena. The
executor allocates the exact output vector, calls `collect_exit`, and stores
one `ExitImage` per finalized external exit task. `ExitedRun::exit_images()`
exposes those vectors to the caller. No path, file, or output name is selected
by `HostBackend`.

### 9.3 Journaling

Logical events (`InitAdmission`, `TaskSubmitted`, `TaskCompleted`, metric
publication, `ArenaAllocated`, `ArenaReleased`, and `Exited`) are emitted by
the executor. The host backend contributes only physical records through the
ABI. This separation preserves one logical admission even though the backend
may use a staging write plus an asynchronous copy, and it preserves one
logical egress even though collection is a second host read.

## 10. Failure and cleanup semantics

### 10.1 Configuration and handoff failures

These failures occur before live work and leave the resource unbound or make
the one-shot state unusable:

* `InvalidConfiguration` covers zero worker or staging capacity, zero arena
  allocation, address-space conversion failure, malformed `/proc/meminfo`,
  and a non-enforced reservation mechanism.
* `InvalidPath` comes from a disk path without a usable file name.
* `DuplicateDevice` and duplicate disk-path checks protect binding maps and
  file ownership.
* `MissingDevice` and the invalid-binding configuration detail identify a host
  binding or required arena device with no corresponding layout, reservation,
  or arena projection.
* `BackendState` covers one-shot binds, candidate versus finalized handoff
  order, warm pool loss, token recycling into deferred resources, and resource
  destruction in an invalid state.
* `UnsupportedWork` covers calculations and finalized transfers whose route
  exceeds the host one-hop contract.
* `Protocol` covers task identity, class, phase, manifest, endpoint, route,
  lane claim, submission slot, arena range, staging, and pending-state
  mismatches.

The outer executor wraps any host error in a fixed `BackendMessage` with the
corresponding `BackendOperation`. The message is bounded to 96 bytes and may
be marked truncated; error formatting does not allocate in the live loop
(`executor/src/error.rs:5-89`, `executor/src/executor.rs:2693-2716`).

### 10.2 Range, storage, and worker failures

`RangeOverflow` protects all checked offset and byte conversions. `OutOfBounds`
is the host backend's resolved-arena range failure; `Arena` uses
`RangeOverflow` for backing-range arithmetic. `SlotCapacityExhausted`,
`SlotBusy`, and `InvalidPendingState` identify runtime token misuse.

`Io` identifies caller-thread arena writes, reads, disk extent allocation,
sync, and capacity queries. A worker's disk read, disk write, unexpected EOF,
or write-zero is wrapped as `WorkerFailed` so the caller can distinguish an
asynchronous copy failure from a submission-thread failure. `ThreadSpawn` is
returned while constructing the worker set. `ThreadPanicked` is returned when
runtime shutdown joins a panicked worker. `Poisoned` identifies a mutex or
shared runtime state that can no longer be trusted.

### 10.3 Poisoning and recovery boundary

`HostResources::ensure_healthy` rejects all later resource operations once the
local `poisoned` bit is set (`backend.rs:676-682`). The bit is set by any
`submit` result error and by any `PendingCopy::poll` error
(`891-900`, `934-938`). It is not set by an ordinary contract failure during
`prepare_pending`, `collect_exit`, `rearm_pending`, or `recycle_pending`; those
methods return their direct error and preserve the observed transition.

Runtime shared poisoning is separate. A worker failure normally stores
`SLOT_FAILED` and the concrete `WorkerFailed` error. If the failure cell or
wake lock is itself poisoned, the runtime marks its shared state poisoned;
subsequent copy preparation or submission then returns `Poisoned`
(`runtime.rs:144-151`, `201-218`, `374-425`).

The executor never retries a failed host submission or poll through another
host implementation. `RunFailure` retains the primary backend error, bounded
journal evidence, and the backend value for recovery. Teardown still releases
every arena in order and then destroys resources; the first later teardown
failure is stored separately as `cleanup_error`
(`executor/src/executor.rs:693-758`, `1489-1541`).

`HostResources::destroy` drops the pending pool before closing the runtime, so
no `HostPending` retains a job-slot reference while workers are joined
(`backend.rs:1111-1118`). `HostPreparedResources::destroy` follows the same
order (`628-635`). In a composite local teardown, bridge, HSA, CUDA, and host
resources are each attempted and the first error is retained
(`native-executor/src/local.rs:2064-2110`, `3231-3259`). Disk backing cleanup
then syncs and removes the exact run-scoped files.

## 11. Invariants that define correctness

The following properties are enforced by the backend or by the immediate
executor and plan boundary. They are the invariants a caller may rely on.

| invariant | enforcement |
| --- | --- |
| Host owns only caller-resolved RAM or disk devices | `HostDeviceBinding`, binding uniqueness, finalized layout ownership, and `HostArenaLookup` (`backend.rs:30-53`, `1286-1338`) |
| Host reservations are scheduler-enforced quotas | `validate_reservations` and `require_enforced_quota` (`1340-1353`) |
| Every host task has one immutable expected contract | `task_contracts` keyed by `TaskId`, duplicate rejection (`1596-1691`) |
| Init image identity and size are exact | finalized or draft `InitDataImage` comparison, image-length check, and staging write (`794-821`, `1434-1455`) |
| Executor-visible host transfers are one-hop copies | route length check and complete route and lane claim comparison (`1657-1672`, `1760-1774`) |
| Calculations never execute on host | task-contract rejection and explicit `BackendWork::Calculation` rejection (`1605-1611`, `878-883`) |
| Every copy is preallocated before `init` | runtime worker set at resource realization, token and staging at pending preparation, arena at initialize |
| No live operation grows a collection | fixed runtime slots and worker staging, fixed `HostPending`, fixed physical-call batch, fixed executor journal bound |
| A token is owned by exactly one task and one host resource | `HostPending.task`, contract lookup, submitted and terminal guards, `prepared` set and pool keys |
| A token is submitted only once per activation | `submitted || terminal` check and runtime `PREPARED` state (`768-773`, `276-296`) |
| Loop reuse follows terminal state, not global index | `prepare_loop_pending` and `rearm_pending` (`941-971`) |
| Metrics are exactly four bytes with finalized dtype | `ExpectedWork::Metric::staging`, `checked_arena(..., 4)`, little-endian decode (`337-341`, `865-876`, `923-929`) |
| External egress is collected only after terminal staging completion | `PendingAction::Egress`, `collect_exit` checks, executor exit vector allocation |
| Host cannot touch another backend's arena | projected lookup returns only `LocalArena::Host`; checked device and range validation |
| Warm resources cannot change after final observation | candidate handoff state, exact bundle and task identity, empty checked-out set, and final contract rebuild |
| A backend bind is one-shot | `mem::replace(&mut state, Bound)` before realization (`158-168`, `1135-1148`) |
| Physical accounting remains bounded and nonduplicated | `MAX_NON_POLL_PHYSICAL_CALLS = 1`, `record`, and composite partition methods that do not record |
| Failure is visible and fail closed | no retry or fallback; submit and runtime poll poison resources, executor retains primary and cleanup errors |

## 12. Boundary summary

`HostBackend` is the concrete bridge between immutable Recipe transfer
contracts and preallocated host byte movement. Its authority ends at a
resolved host arena range, a four-byte metric representation, or a caller-owned
exit slice. `recipe_core` supplies value locations, phases, routes, lane
claims, manifests, and reservations. `recipe_executor` supplies the typestate
lifecycle, pending request ABI, physical accounting, metric publication, and
teardown order. `native-executor` supplies heterogeneous ownership and the
separate cross-backend bridge. `recipe-host` itself supplies only the runtime
workers, RAM or disk backing, staging, copy semantics, and the exact host-side
validation needed to keep those layers consistent.
