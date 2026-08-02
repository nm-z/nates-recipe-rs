---
document: recipe_native_executor.local
source: native-executor/src/local.rs
kind: heterogeneous-local-backend-and-candidate-lifecycle
authority:
  - native-executor/src/local.rs
  - native-executor/src/candidate.rs
  - native-executor/src/bridge.rs
  - native-executor/src/cuda.rs
  - native-executor/src/hsa.rs
  - host/src/backend.rs
  - executor/src/backend.rs
  - executor/src/executor.rs
  - core/src/plan.rs
  - core/src/schedule.rs
---

# Local execution over heterogeneous partitions

`native-executor/src/local.rs` is the composite backend for one finalized local
Recipe plan. It composes an optional host partition, a CUDA partition, an HSA
partition, and one pre-realized cross-backend bridge behind the sealed
`recipe_executor::Backend` trait. The file also owns the pre-final candidate
factory used to realize, warm, measure, and hand off those resources.

The important boundary is ownership. A task is assigned to exactly one
`TaskOwner`, every finalized arena is assigned to exactly one
`LocalDeviceClass`, and the pending token variant must agree with that owner.
The executor supplies immutable `BackendWork` values and read-only arena views;
the composite only routes those values to the owner that was derived from the
finalized bundle.

## Source map

The following anchors refer to the current implementation in
`native-executor/src/local.rs`.

| Area | Source anchors | Responsibility |
| --- | --- | --- |
| Device and arena views | `L32-L123` | Device classes, owned arenas, borrowed arena projection |
| Bridge contracts | `L125-L212` | Runtime and candidate-time cross-backend bridge APIs |
| Rejecting bridge and errors | `L214-L494` | Homogeneous bridge policy and error wrappers |
| Prepared bridge handoff | `L496-L612` | Exact once-only bridge resource transfer |
| Composite resource and warm data | `L614-L878` | Resource maps, pending tokens, warm task representation |
| Candidate session and stabilizers | `L880-L1071` | Pre-final state and stabilizer implementations |
| Warm execution and handoff | `L1073-L1420` | Warm passes, capacity observation, backend conversion, cleanup |
| Candidate factory | `L1422-L1608` | Candidate validation, realization, pass ordering, snapshots |
| `LocalBackend` implementation | `L1610-L2110` | Runtime composition and every `Backend` method |
| Arena lookup projection | `L2113-L2143` | Type-specific views for child backends |
| Capacity and partition helpers | `L2145-L2596` | Device evidence, ownership, artifacts, routes, slot checks |
| Provisional warm bundle and work | `L2598-L2842` | Warm bundle, task conversion, images, arena allocation |
| Warm scheduler | `L2844-L3260` | Maximum-concurrency init, loop, and exit trace |
| Capacity accounting and teardown | `L3262-L3479` | Snapshot anchoring, capacity ledger, cleanup, identity checks |
| Final classification and small helpers | `L3481-L3658` | Final route ownership, owner checks, accounting, first-error policy |

Line numbers are navigation aids, not an additional contract. The type and
state invariants below are the contract implemented by those regions.

## Type model

### Device classes and arenas

`LocalDeviceClass` has exactly three values: `Host`, `Cuda`, and `Hsa`. The
class is the immutable owner of a finalized device. `LocalArena` owns one
concrete allocation and preserves the class and `DeviceId` supplied by the
child backend. `LocalArenaRef` is the corresponding borrowed, read-only form.
Both expose `class()` and `device()` by delegating to the contained arena.

`LocalArenaSet` wraps the executor's `ArenaSet<LocalArena>` and never extends or
mutates it. `get` and `iter` project each enum variant into `LocalArenaRef`.
This view is passed to the bridge as `LocalArenaSet` and is converted into a
type-specific `ProjectedArenas` view for host, CUDA, and HSA child calls.

### Bridge contracts

`CrossBackendTransfer` is the runtime bridge for planner-expanded one-hop
transfers that cross backend ownership boundaries. Its associated `Resource`,
`Pending`, and `Error` types remain owned by the bridge implementation. The
methods have these phases:

| Method | Allowed work and inputs |
| --- | --- |
| `bind` | Receives the finalized bundle, bridge task IDs, and device ownership map. This is the bridge bind point. |
| `prepare_pending` | Creates or obtains one pre-realized bridge pending token for a `PendingRequest`. |
| `submit` | Receives a borrowed `LocalArenaSet`, a bridge token, a transfer class, and immutable transfer work. It must be nonblocking and allocation-free. |
| `poll` | Polls one bridge token and returns `BackendPoll`. It must be nonblocking and allocation-free. |
| `supports_loop_repetition` | Declares whether terminal loop transfer tokens can be reused. The default is `false`. |
| `rearm_loop_pending` | Resets a never-submitted or terminal loop token without allocating. The default is a no-op. |
| `destroy` | Consumes the bridge resource. |

The trait documentation makes `bind` and `prepare_pending` the only bridge
methods allowed to allocate, register memory, create queues, or grow storage.
The composite does not add a lazy bridge path.

`CandidateCrossBackendTransfer` extends this trait and `Clone` for the pre-final
phase. `realize_candidate` must create every registration, staging allocation,
queue, and completion object. `validate_handoff` can inspect finalized
addresses, but cannot allocate or replace the resource. `recycle_candidate_pending`
returns one terminal warm-pass token to the exact resource that created it.

`RejectCrossBackend` is the closed homogeneous policy. It accepts an empty
bridge task set and otherwise returns `CrossBackendUnavailable` from
`bind` and `realize_candidate`; its pending, submit, poll, validation, and
recycle methods also fail. Its `poll` and recycle failures use a zero task ID
because no bridge token exists. Its `supports_loop_repetition()` is `true`
because it has no physical token to rearm. Consequently, a `LocalBackend` using
this bridge can run repeated homogeneous plans, while any actual cross-backend
task fails closed during bind.

### Runtime resources and tokens

`LocalResources` is the resource object returned by `bind_resources`:

```text
devices: BTreeMap<DeviceId, LocalDeviceClass>
tasks:   BTreeMap<TaskId, TaskOwner>
host:    Option<HostResources>
cuda:    CudaResources
hsa:     HsaResources
bridge:  BridgeResource
```

The maps are the authoritative dispatch table for the lifetime of one
finalized run. They are copied from `Partitions` during warm activation or
created from `classify` during a direct bind. `LocalPending` keeps the concrete
token in the same owner-shaped enum:

| Pending variant | Child token | Owner required |
| --- | --- | --- |
| `Host` | `HostPending` | `TaskOwner::Host` |
| `Cuda` | `CudaPending` | `TaskOwner::Cuda` |
| `Hsa` | `HsaPending` | `TaskOwner::Hsa` |
| `Bridge { task, pending }` | `Bridge::Pending` plus an explicit task ID | `TaskOwner::Bridge` |

`LocalPending::task()` reads the task ID from the child token, except for the
bridge variant where it reads the explicit field. `ensure_owner` compares that
owner with `LocalResources.tasks`; a missing task is `TaskNotAssigned`, and a
different owner is `PendingOwnerMismatch`.

`PreparedBridge` is the bridge state used by the production handoff. Its
`bind` method compares the finalized bridge task set and device map with the
candidate copies, requires `handoff_validated`, and changes
`PreparedBridgeResource::Available` to `Consumed` while moving the exact
resource out. A second bind, a bind with different partitions, or a bind before
handoff validation returns `PreparedBridgeError::State`. All later bridge
operations delegate through the wrapper and convert bridge errors to
`PreparedBridgeError::Bridge`.

### Errors

`LocalError<BridgeError>` is the composite error boundary. Structural failures
have no source error: `DuplicateDevice`, `MissingDevice`, `UnexpectedDevice`,
`UnsupportedCalculation`, `UnsupportedRoute`, `TaskNotAssigned`,
`ArenaOwnerMismatch`, `PendingOwnerMismatch`, `CapacityMismatch`,
`BackendState`, and `PhysicalAccountingOverflow`. Child failures are wrapped as
`Host(recipe_host::Error)`, `Native(crate::Error)`, or `Bridge(BridgeError)` and
are returned as their `StdError::source()`.

The display text distinguishes ownership, route, capacity, state, and physical
accounting failures. `PreparedBridgeError` is narrower: `State` describes a
bad once-only handoff and `Bridge` wraps an underlying bridge error.

## Ownership and route classification

The same ownership rules are applied twice. `classify_candidate` works on a
`PlannedCandidate` before finalized addresses exist. `classify` works on a
`FinalizedBundle` and resolves transfer endpoints first. Both produce a
`Partitions` value containing one device map, one task map, and four owner task
sets.

### Device ownership

`declared_devices` concatenates host bindings, CUDA bindings, and HSA bindings
into `(DeviceId, LocalDeviceClass)` pairs. `validate_device_owners` then enforces
an exact set relation:

1. A declared device may appear only once. A second binding, even from a
   different class, is `DuplicateDevice`.
2. Every device named by a candidate or finalized arena layout must be
   declared, otherwise `MissingDevice`.
3. Every declared device must have a required arena layout, otherwise
   `UnexpectedDevice`.

The resulting `BTreeMap` is the only device class map used by classification,
capacity reads, arena allocation, and arena release.

### Task ownership

The owner rules are:

| Task or endpoint shape | Owner |
| --- | --- |
| Calculation on a CUDA device | `Cuda` |
| Calculation on an HSA device | `Hsa` |
| Calculation on a host device or unknown device | `UnsupportedCalculation` |
| Metric | Class of the value's device (`Host`, `Cuda`, or `Hsa`) |
| External to device | Class of the destination device |
| Device to external | Class of the source device |
| Host to host | `Host` |
| HSA to HSA | `Hsa` |
| CUDA to the same CUDA device | `Cuda` |
| CUDA to a different CUDA device | `Bridge` |
| Any host/CUDA/HSA mixed device pair | `Bridge` |
| External to external | `UnsupportedRoute` |

The device-to-device matrix is implemented by `device_transfer_owner`. Host
and HSA transfers stay in their child backend even when the two device IDs
differ. CUDA transfers stay local only when source and destination IDs are
equal. All mixed-class transfers and inter-device CUDA transfers require a
bridge.

An owner of `Bridge` is legal only for a transfer. It must have exactly one
candidate or finalized link in `route`. A route with two or more links is
rejected before dispatch because executor-visible transfers must already be
planner-expanded to one hop. A bridge owner with zero links is also rejected.

### Submission-slot isolation

`validate_partition_slots` associates every task's queue slot and completion
slot with its owner. Reusing a slot by tasks of the same owner is allowed. A
queue or completion slot shared by different owners is `BackendState`, because
the composite cannot safely dispatch one slot through multiple child backends.
The check runs for both candidate and finalized partitions.

### Artifact partitioning

`partition_candidate_artifacts` scans calculation tasks and collects artifact IDs
by `TaskOwner::Cuda` and `TaskOwner::Hsa`. A calculation assigned to host or
bridge is rejected. An artifact ID cannot occur in both GPU sets. Each supplied
`CandidateArtifact` must remove exactly one ID from one set, and its runtime
image is sent to the matching prepared child resource. An unused supplied
artifact is `Error::UnexpectedArtifact`; an unprovided required ID is
`Error::MissingArtifact`. The identity vector retained by the session preserves
the supplied artifact identities for final handoff comparison.

## Pre-final candidate lifecycle

`LocalCandidateFactory` implements `CandidateSessionFactory`. It stores the
optional `HostBackendConfig`, CUDA and HSA bindings, a cloneable bridge, a
stabilizer, and one optional `InitialCapacitySnapshot`. `new` accepts an
explicit stabilizer. `fail_closed` selects `UnavailableLocalStabilizer`, whose
warm and capacity methods return `CandidateFailure::PreFinalRealizationUnavailable`.
`production` selects `NativeLocalStabilizer`, which calls the session's native
warm and capacity methods.

### Realization

`realize_candidate` proceeds in this order:

1. Validate the request's topology, discovery, draft, reservations, and exact
   artifact set with `CandidateRealizationRequest::validate`.
2. Build the declared device list and classify the candidate. This checks exact
   device ownership, task routes, task owners, and queue/completion isolation.
3. Capture initial available bytes once per topology and discovery identity.
   A later request with different identities is a fatal `BackendState`; the
   first snapshot is reused for matching identities.
4. Require a reservation entry for every device and require initial available
   bytes to be at least the held reservation (`validate_initial_headroom`).
5. Partition runtime artifacts between CUDA and HSA and preserve their static
   identities.
6. Prepare the optional host partition, realize CUDA prepared resources, and
   realize HSA prepared resources. These child realizations load the exact
   runtime artifacts and create their pre-final queues, completion objects,
   staging, scratch, and pending pools.
7. Clone the bridge and call `realize_candidate` with the candidate bridge task
   set and exact device map.
8. Return a `LocalPreparedSession` in `LocalPreparedPhysical::Candidate` with
   `StabilizationState::Realized`.

If any later child realization fails, already prepared resources are destroyed
in the cleanup helpers. The original error is retained; errors from cleanup are
discarded by `cleanup_error` after the first error has been selected.

### Stabilization state machine

The session's `stabilization` field has three states:

```text
Realized --warm(pass 1)--> Warmed { pass: 1 }
Warmed { pass } --capacity_snapshot--> Observed { pass }
Observed { pass } --warm(pass + 1)--> Warmed { pass: pass + 1 }
```

`LocalCandidateFactory::warm_maximum_concurrency` requires the supplied
candidate to equal the realized candidate. `Realized` accepts only pass `1`.
`Observed` accepts only the checked `pass + 1`. Calling warm while already
`Warmed`, skipping a pass, or changing the candidate returns
`CandidateRejected`. `capacity_snapshot` requires matching topology and
discovery identities and requires a new `Warmed` state. On success it records
`Observed { pass }`.

### Warm activation

The first warm pass calls `activate_warm_resources`:

1. A provisional finalized bundle is built from the candidate draft, topology,
   discovery, loaded artifact identities, and reservations. Its capacity entries
   use discovered total capacity minus the exact reservation as usable bytes,
   with zero runtime overhead, fragmentation, and safety headroom. Address
   resolution errors reject the candidate.
2. The physical state moves to `Transition`, then the `Candidate` resources are
   consumed. Host prepared resources call `bind_candidate`; CUDA and HSA call
   their corresponding bind methods. A host, CUDA, or HSA bind failure destroys
   the resources already moved and marks the session `Destroyed`. If warm task,
   image, or exit metadata preparation fails after the children have been
   assembled, the error is returned while the physical state remains in its
   `Transition` state.
3. The bound children and bridge resource are assembled into `LocalResources`.
4. `prepare_warm_tasks` converts every bundle task to owned warm metadata;
   `prepare_warm_images` creates zero-filled host images for every init manifest;
   `prepare_warm_exits` creates zero-filled host destinations only for external
   exit transfers.
5. The state becomes `LocalPreparedPhysical::Warm`, with empty warm arenas,
   immutable task metadata, images, and exit buffers.

On later passes, activation is a no-op and the same warmed resources and arena
map are reused.

### Warm task representation

`WarmTask` retains task ID, phase, schedule window, dependency IDs, and a
`WarmWork` value. The four `WarmWork` forms map directly to executor work:

| `WarmWork` | Phase and conversion |
| --- | --- |
| `Init` | An init transfer from external input to a device. `backend_work` looks up that device's zero-filled image and emits `BackendWork::InitAdmission`. |
| `Calculation` | A loop calculation with resolved input, output, and optional fault-flag locations. Emits `BackendWork::Calculation` with the warm pass `RunId` and one loop iteration. |
| `Transfer` | An init or loop internal transfer, or an exit transfer. The stored class must be `InternalTransfer` or `ExitTransfer`; any other class is `BackendState`. |
| `Metric` | A loop metric with resolved value location and submission slot. Emits `BackendWork::Metric`. |

Calculations and metrics in init or exit phases are invalid. Missing value
locations or missing transfer endpoint resolutions are `TaskNotAssigned`.
`external_exit_bytes` selects only `ExitTransfer` values whose destination is
`External`; a bridge can therefore never be placed in the warm exit collection
path.

### Maximum-concurrency warm scheduler

`run_warm_trace` executes all warm tasks through the same child-resource calls
used by the runtime backend. It uses `RunId::new(pass)` and exactly one
zero-based loop iteration. Phases are strictly ordered: `Init`, then `Loop`,
then `Exit`.

Within one phase, a remaining task is runnable only when:

- its phase matches the current phase;
- its dependencies are in the completed set; and
- every currently pending task has an overlapping schedule window.

For each runnable task the scheduler creates a `PendingRequest`, calls
`prepare_warm_pending`, builds `BackendWork`, and calls `submit_warm_work`.
Pending tokens are keyed by task ID. A duplicate key is
`PendingOwnerMismatch`.

The scheduler then polls every pending task in the current phase. `Pending`
leaves the token in place. `Complete` removes the token, collects an external
exit into its preallocated host buffer when applicable, recycles the terminal
token, marks the task complete, and adds its ID to the dependency set. A bridge
exit reaches `collect_warm_exit` and fails with `UnsupportedRoute` because
cross-backend transfers cannot have an external endpoint.

If a phase still has work but no pending token and no progress, the scheduler
returns `BackendState("maximum-concurrency warm scheduler stalled")`. If work
is pending but no progress occurs, it backs off from 50 microseconds up to 2
milliseconds and fails after 10,000,000 idle polls with
`BackendState("maximum-concurrency warm trace did not reach terminal completion")`.
Progress resets the backoff. A phase completes only after every task in that
phase is terminal.

### Capacity observation

After a complete warm pass, `observe_capacity` takes the warm arena map and
releases every arena before reading live availability. The first successful
observation is stored by `anchor_capacity_snapshot`; subsequent calls return a
clone of that anchored ledger and cannot rewrite the scheduler contract.

`observe_capacity_ledger` requires topology and discovery identities to match
the immutable initial snapshot. For every topology device it requires:

- a measured discovery capacity;
- an initial available-byte entry;
- an exact held reservation;
- a live resource owner and a successful child `available_bytes` read.

The accounting function is:

```text
capped_live = min(live_available, initial_available)
runtime_overhead = initial_available - capped_live
recipe_usable = capped_live - reservation_bytes
```

Underflow is `CapacityMismatch`. The resulting ledger marks total,
runtime-overhead, fragmentation, safety headroom, and recipe-usable values as
measured; fragmentation and safety headroom are zero in this observation.

### Final handoff

`LocalPreparedSession::into_backend` first calls `validate_handoff`. The
session must be `Observed`, must contain one `Warm` physical resource set, and
must have an empty arena map because capacity observation already released the
warm arenas. `validate_prepared_identity` compares all of the following with
the finalized bundle:

- topology identity and discovery identity;
- draft, candidate, task, kernel, and artifact-build identities;
- resource declarations and init-image manifests;
- reservation ledger;
- every loaded artifact identity.

The bridge, optional host resources, CUDA resources, and HSA resources each
validate their own handoff and pending pools. On success, the session is
consumed and converted to a `LocalBackend` whose children are built with
`from_warmed`. The bridge is wrapped in `PreparedBridge` with its exact bridge
task set, device map, and `handoff_validated: true`.

On validation failure, `into_backend` calls `destroy`. If teardown also fails,
the teardown error replaces the validation error. This is the one place where
the session's validation and destruction errors are intentionally ordered.

### Session destruction

`LocalPreparedSession::destroy` consumes the session after replacing its
physical state with `Transition`:

| Physical state | Destruction |
| --- | --- |
| `Candidate` | Destroy bridge resource, HSA prepared resources, CUDA prepared resources, then optional host prepared resources. |
| `Warm` | Release all warm arenas, then destroy bridge, HSA, CUDA, and optional host warm resources. |
| `Transition` | Return `BackendState` because ownership is already moving. |
| `Destroyed` | Return success. |

`retain_first` preserves the first error and drops later teardown errors. The
state is not restored after a failed destruction attempt.

## `LocalBackend` composition

`LocalBackend` stores:

```text
host:             Option<HostBackend>
cuda:             CudaBackend
hsa:              HsaBackend
bridge:           Bridge
declared_devices: Vec<(DeviceId, LocalDeviceClass)>
native_evidence:  NativeExecutionEvidence
```

`LocalBackend::new` creates `HostBackend` from the optional host config,
`CudaBackend` and `HsaBackend` from their bindings and runtime artifacts, and
the bridge from the caller. It computes `declared_devices` immediately. This
constructor leaves child backends in their ready states; their partition bind
methods perform the direct finalized realization when the executor calls
`bind_resources`.

The production candidate path does not call this constructor. It creates the
backend in `LocalPreparedSession::into_backend`, using `from_warmed` child
states and a `PreparedBridge`, so the exact resources warmed before finalize are
the resources used by the finalized run.

The backend is sealed with `recipe_executor::sealed::Sealed`. Its associated
types are `LocalArena`, `LocalError<Bridge::Error>`,
`LocalPending<Bridge::Pending>`, and `LocalResources<Bridge::Resource>`.
`MAX_NON_POLL_PHYSICAL_CALLS` is `1`.

## Runtime call trace through `recipe_executor::Backend`

The executor invokes the composite in the following lifecycle. The executor
creates a fresh `PhysicalCallBatch` for each operation; local methods append at
most one record per non-poll operation and exactly one poll record.

```text
PreparedRun::prepare
  supports_loop_repetition
  bind_resources
  prepare_pending for Init tasks
  prepare_pending for Loop tasks
  prepare_pending for Exit tasks

PreparedRun::initialize
  allocate_arena for every finalized arena layout
  submit / poll Init tasks until complete

InitializedRun::start_loop
  submit_loop_iteration / poll Loop tasks for each active iteration

ExitedLoop::exit
  submit / poll Exit tasks
  collect_exit for each external exit
  release_arena for every arena
  destroy_resources
```

The outer executor rejects a finite loop count greater than one or an unbounded
loop when `supports_loop_repetition` is false. It prepares all phase tokens
before init, allocates arenas only during initialize, submits loop work only with
an active zero-based iteration, and calls `collect_exit` only after loop
completion. On failures it releases arenas before consuming the resource for
`destroy_resources`.

### Physical-call accounting

`record` maps a full `PhysicalCallBatch` to
`LocalError::PhysicalAccountingOverflow` when the fixed batch is full. The local
backend emits these records:

| Backend method | Record |
| --- | --- |
| `bind_resources` | `BindResources` |
| `prepare_pending(task)` | `PreparePending { task }` |
| `allocate_arena(layout)` | `AllocateArena { device, bytes }` |
| `submit(work)` and successful loop submission | One record from `accounting::submission_call`: `AdmissionChunk` for init, `SubmitCalculation`, `SubmitInternalTransfer`, `SubmitMetric`, or `SubmitExitTransfer`. |
| `poll(task)` | Exactly one `Poll { task, status }`, with `Pending`, `Complete`, or `Failed` derived from the returned result. |
| `collect_exit(work)` | `CollectExit { task, bytes }` |
| `release_arena(device)` | `ReleaseArena { device }` |
| `destroy_resources` | `DestroyResources` |

`submit_loop_iteration` prepares or rearms the token and then calls `submit`, so
it emits only the one submission record from `submit`. If the batch cannot
accept a record, the operation returns `PhysicalAccountingOverflow`.

### `bind_resources`

1. Append `BindResources`.
2. Call `classify(bundle, declared_devices)`.
3. If a host child exists, bind `partitions.host` through
   `HostBackend::bind_partition`. If no host child exists, any host task is a
   `BackendState` failure.
4. Bind `partitions.cuda` through `CudaBackend::bind_partition`.
5. Bind `partitions.hsa` through `HsaBackend::bind_partition`.
6. Bind the bridge with the finalized bundle, bridge task set, and device map.
7. Return `LocalResources` containing the maps and all child resources.

Each child error is wrapped in `LocalError`. The local method has no alternate
bind path and no lazy bridge realization. Child backends enforce their own
single-bind state, while `PreparedBridge` additionally enforces exact
candidate partition identity and one-time resource consumption. The local
method does not perform a rollback between child bind calls; once a child bind
has transitioned its own state, a later child or bridge error is returned to the
executor as the bind failure.

### `prepare_pending`

The method records `PreparePending` and routes by `resource.tasks[task]`:

| Owner | Local operation |
| --- | --- |
| Host | Require `resource.host` and `self.host`, then `HostBackend::prepare_partition`. |
| CUDA | `CudaResources::prepare_pending`. |
| HSA | `HsaResources::prepare_pending`. |
| Bridge | `Bridge::prepare_pending`, wrapped with the request task ID in `LocalPending::Bridge`. |
| Missing map entry | `TaskNotAssigned`. |

Missing child state is `BackendState`. The executor performs this operation for
every task in each phase before any phase runs, so a returned token must already
contain every completion object, queue reference, staging buffer, and other
resource needed by later submit and poll calls.

### `allocate_arena`

The method records `AllocateArena` and routes by the finalized device map:

- Host requires both bound host resources and the configured host child, then
  calls `HostBackend::allocate_partition`.
- CUDA calls `CudaResources::allocate_arena`.
- HSA calls `HsaResources::allocate_arena`.
- An absent device map entry is `MissingDevice`.

The returned concrete arena is wrapped in the corresponding `LocalArena` enum.
The outer executor holds these arenas in its immutable `ArenaSet` for all later
submissions and releases them only after exit.

### Repetition and same-queue pipelining

`supports_loop_repetition` returns exactly `bridge.supports_loop_repetition()`.
The composite does not infer repetition support from the absence of bridge
tasks. The bridge implementation must opt in for the whole local backend.

`supports_same_queue_pipelining` returns true only when the task map says
`TaskOwner::Cuda`. Host, HSA, and bridge tasks retain the executor's default
completion-token ordering. For CUDA tasks the outer scheduler may treat a
pending predecessor on the same queue as a valid dependency and may overlap
tasks with non-overlapping schedule windows according to the executor's
pipeline rules.

### `submit`

The method records the work-class-specific submission call, creates a
`ProjectedArenas` view, and dispatches by the pending token variant. Every
variant first calls `ensure_owner`.

| Pending token | Delegated method | Additional checks |
| --- | --- | --- |
| `Host` | `HostBackend::submit_partition(resource, &ProjectedArenas, pending, work)` | Host resource and child must exist. |
| `Cuda` | `CudaResources::submit(&ProjectedArenas, pending, work)` | Owner must be CUDA. |
| `Hsa` | `HsaResources::submit(&ProjectedArenas, pending, work)` | Owner must be HSA. |
| `Bridge` | `Bridge::submit(&mut bridge_resource, LocalArenaSet { arenas }, pending, class, transfer)` | Stored pending task must equal `work.task`; only internal and exit transfers are accepted. |

The bridge arm converts `BackendWork::InternalTransfer` and
`BackendWork::ExitTransfer` into the bridge's explicit `WorkClass`. Init,
calculation, and metric variants cannot be submitted through a bridge token and
return `PendingOwnerMismatch`.

The child backends then validate the immutable task contract, route, lane
claims, submission slots, arena locations, and work class. A child submission
failure is wrapped without a substitute implementation.

### `submit_loop_iteration`

The `_iteration` parameter is intentionally not used for token state. The
executor already embeds the exact iteration in `BackendWork::Calculation` and
`BackendWork::Metric`, and the child token tracks whether it is freshly prepared
or terminal and ready for rearm.

The method first checks ownership, then calls:

| Pending token | Rearm preparation |
| --- | --- |
| Host | `HostResources::prepare_loop_pending` |
| CUDA | `CudaResources::prepare_loop_pending` |
| HSA | `HsaResources::prepare_loop_pending` |
| Bridge | `CrossBackendTransfer::rearm_loop_pending` after matching the stored task ID |

It then calls `submit`, which emits the one submission record and delegates the
actual work. A mismatched bridge task is `PendingOwnerMismatch`; child token
state errors are wrapped as host or native errors.

### `poll`

The task ID comes from `LocalPending::task()`. Poll dispatch is direct:

- Host: `HostBackend::poll_partition`.
- CUDA: `CudaResources::poll`.
- HSA: `HsaResources::poll_pending`.
- Bridge: `CrossBackendTransfer::poll`.

The result is translated to a physical status before recording one poll call.
`BackendPoll::Pending` maps to `PhysicalPollStatus::Pending`, complete maps to
`Complete`, and every `LocalError` maps to `Failed`. The original child result
is returned after recording. A batch overflow while writing the poll record
returns `PhysicalAccountingOverflow` instead.

### `collect_exit`

The method records `CollectExit`, builds a `ProjectedArenas` view, and delegates
external result copying to the Host, CUDA, or HSA child according to the
pending token. The executor has already verified that the transfer is an exit
transfer with an external destination. A bridge pending token is always
rejected with `UnsupportedRoute`, because bridge-owned transfers are
device-to-device and cannot expose an external endpoint.

The child validates that the token is terminal, the source location and bytes
match the finalized contract, and the destination buffer has the exact size.

### `release_arena`

The method records `ReleaseArena`, looks up the expected class, and checks both
`arena.device() == device` and `arena.class() == expected`. A mismatch is
`ArenaOwnerMismatch`. It then releases through:

- `HostBackend::release_partition`, after requiring host resources and child;
- `CudaArena::release`, after `CudaResources::ensure_healthy`;
- `HsaArena::release`, after `HsaResources::ensure_healthy`.

The executor invokes this once per arena after exit or during failure teardown.
The arena is consumed by the selected child release operation.

### `destroy_resources`

The method records `DestroyResources` and captures CUDA and HSA execution
evidence before consuming the resource object. It destroys in this fixed order:

```text
bridge -> HSA -> CUDA -> optional Host
```

`retain_first` keeps the first error and drops later errors. If a bound host
resource exists but `self.host` is absent, the host step is a
`BackendState` error. Only when every child destruction succeeds does the
backend store `NativeExecutionEvidence::completed(device_evidence)`, which
records the native devices and marks teardown complete with zero live resources.
On any error the evidence remains at its previous default or completed value.

## Arena lookup projection

`ProjectedArenas` implements three child lookup traits over one
`ArenaSet<LocalArena>`:

| Trait | Returns |
| --- | --- |
| `HostArenaLookup::host_arena` | Only `LocalArena::Host` for the requested device |
| `CudaArenaLookup::arena` | Only `LocalArena::Cuda` for the requested device |
| `HsaArenaLookup::arena` | Only `LocalArena::Hsa` for the requested device |

For a different class or absent device, the lookup returns `None`. This keeps
child backends from seeing or interpreting another backend's allocation.

## Capacity and reservation evidence

`reservation_evidence_for_device` scans exactly the configured binding lists for
one device. A host binding yields `ReservationEvidence::NonGpu`. CUDA and HSA
bindings yield `GpuDisplay` with their enabled display connector count. A
second matching binding of any class is `DuplicateDevice`; no matching binding
is `MissingDevice`.

`capture_initial_capacity` reads the initial available bytes from the child
binding or host configuration selected by the device map. A missing binding or
owner is `CapacityMismatch`; duplicate insertion is `DuplicateDevice`. The
snapshot retains topology and discovery identities so it cannot be reused for a
different hardware description.

`validate_initial_headroom` requires every snapshot device to have a
reservation policy and enough available bytes to cover its reservation. This is
checked before any candidate resources are realized.

## Failure and cleanup matrix

The following table summarizes where failures originate and how the composite
reports them.

| Failure | Detection point | Report |
| --- | --- | --- |
| Duplicate, missing, or extra binding/device | `validate_device_owners`, reservation evidence, capacity capture | `DuplicateDevice`, `MissingDevice`, `UnexpectedDevice`, or `CapacityMismatch` |
| Host calculation | Candidate or finalized classification | `UnsupportedCalculation` |
| Multi-hop route | Candidate or finalized classification | `UnsupportedRoute` |
| External-to-external route | Candidate or finalized route owner | `UnsupportedRoute` |
| Bridge with zero or multiple links | Candidate or finalized bridge partition | `UnsupportedRoute` |
| Queue/completion slot crosses owners | Partition validation | `BackendState` |
| Runtime artifact assigned to both GPU classes | Candidate artifact partition | `BackendState` |
| Missing or unexpected runtime artifact | Candidate artifact partition or child bind | `Native(Error::...)` |
| Task absent from the immutable map | Prepare, warm conversion, owner checks | `TaskNotAssigned` |
| Token and task owner disagree | Submit or loop preparation | `PendingOwnerMismatch` |
| Arena device or class disagrees | Release | `ArenaOwnerMismatch` |
| Child resource absent | Any routed operation | `BackendState` |
| Child driver or host operation fails | Delegated child method | `Host`, `Native`, or `Bridge` wrapper |
| Fixed physical batch is full | `record` | `PhysicalAccountingOverflow` |
| Warm scheduling stalls | `run_warm_trace` | `BackendState` with scheduler detail |
| Warm pass order or candidate changes | Candidate factory | `CandidateRejected` |
| Handoff identity or state differs | `validate_handoff` | `BackendState` |
| Teardown step fails | `destroy`, `destroy_resources`, or arena release | First error, later errors dropped |

Cleanup is deterministic and one-way. Candidate realization cleans up already
prepared children if a later child or bridge fails. Warm activation destroys
already-bound children on a later bind failure. Session destruction moves to
`Transition` before consuming resources. Runtime teardown consumes all arenas,
then consumes the resource object. No fallback owner, substitute bridge, retry,
or lazy resource creation exists in `local.rs`.

## End-to-end invariants

These invariants are the checks that must remain true for a valid local run:

1. Every finalized arena device has exactly one declared local owner, and every
   declared owner appears in the finalized arena set.
2. Every task has exactly one `TaskOwner`; calculations are GPU-only, metrics
   follow their value device, and mixed device-class transfers use exactly one
   bridge link.
3. Every queue and completion slot is owned by one backend class for the whole
   partition.
4. Every pending token variant agrees with the task owner and task ID stored in
   `LocalResources`.
5. Candidate, warm, and finalized identities match before resources cross the
   pre-final handoff.
6. All native, host, and bridge resources used by the finalized backend were
   created before loop submission. Loop submit and poll only reuse or rearm
   those resources.
7. The warm pass executes init, loop, and exit through the same child resource
   paths as the finalized backend, with one zero-based loop iteration and the
   candidate's maximum-concurrency windows.
8. Capacity is measured after warm arena allocation and release, then anchored
   so later allocator or display drift cannot alter the finalized contract.
9. External exit collection is handled only by host, CUDA, or HSA owners. A
   bridge never receives an external endpoint.
10. Destruction order is bridge, HSA, CUDA, host, and only a completely
    successful teardown publishes completed native evidence.

These invariants explain why routing is explicit rather than inferred from the
pending token at runtime: the maps and token shapes are checked before work is
submitted, and every later operation is a direct dispatch through that fixed
ownership decision.
