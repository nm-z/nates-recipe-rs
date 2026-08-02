# Worker execution

`executor/src/worker.rs` is the executor-side implementation of one remote
worker. It turns a finalized bundle and a measured topology into an immutable
projection for one worker node, binds a caller-supplied native backend to that
projection, and exposes the nonblocking operations needed by the remote
worker protocol. It does not discover hardware, compile kernels, choose a
schedule, own a transport connection, or implement a native driver. Those
concerns are supplied by the finalized bundle, the topology, the
`WorkerBackend` implementation, and (in the repository's concrete caller) the
remote crate.

The source is deliberately split into two layers:

1. `WorkerProjection` validates and closes the part of the bundle visible to
   one machine/node assignment. It resolves values, transfers, queues,
   completion slots, arenas, init images, dependencies, and artifacts before
   execution starts.
2. `WorkerExecutionSession` owns the pre-realized backend resource, arenas,
   pending tokens, image buffers, task state, lifecycle, watchdog, and
   `RunJournal`. Every live operation validates the run, phase, role,
   dependencies, schedule window, and task state before calling the backend.

The public types are re-exported from `executor/src/lib.rs`, so users of the
crate normally import them from `recipe_executor`. The only in-tree adapter
that constructs a projection and session is
`remote/src/executor_driver.rs::ExecutorWorkerDriver`; its `WorkerDriver`
implementation forwards remote protocol commands to this module.

## Architectural boundary

The worker is an execution boundary for an already finalized plan. A
`FinalizedBundle` owns the authoritative task graph and resource manifests.
`Topology` supplies measured machine, node, device, and directed-link facts.
The worker accepts an exact `WorkerAssignment { machine, node }` and keeps only
the tasks whose device or transfer endpoint belongs to that worker.

The worker backend is required to bind and realize all resources before init:

- `bind_worker_resources` receives the bundle and immutable projection and
  returns the backend resource.
- `prepare_pending` realizes a reusable local completion/submission token for
  each projected local task.
- `prepare_external` realizes a token for each projected cross-machine
  transfer.
- `allocate_arena` is called when the corresponding init image begins, before
  any image bytes are submitted.
- `submit`, `poll`, and all external-transfer methods operate on those already
  realized resources. They must not compile, load, allocate, or discover
  during live execution.
- `quiesce_worker` is the required native barrier before arena release or
  resource destruction.

`WorkerBackend` has no concrete implementation in this workspace. Native
adapter code supplies `B` to the generic `ExecutorWorkerDriver<B>` in the
remote crate. Consequently this module owns the sequencing and contracts,
while the adapter owns the driver calls represented by each hook.

The session uses `ArenaSet` as a read-only view of its `BTreeMap` of arenas.
The backend cannot add or replace an arena through that view. The session is
also the only owner of the backend after preparation. On a terminal success or
failure, `into_parts` can return the backend and journal only after the native
resource has been destroyed.

## Projection data model

### Assignment, roles, and transfers

`WorkerAssignment` is a copyable pair of `MachineId` and `NodeId`. Its
constructor and accessors do no validation. `WorkerProjection::derive` is the
validation boundary that proves the pair names an existing worker node on the
specified machine.

`WorkerTaskRole` is closed and covers every projected task:

| Role | Meaning | Backend path |
| --- | --- | --- |
| `InitAdmission` | One exact external-to-local init image admission for one device | `BackendWork::InitAdmission` |
| `Local` | A local calculation, local metric readback, or local device-to-device transfer | `BackendWork::Calculation`, `InternalTransfer`, or `Metric` |
| `ExternalIngress` | A cross-machine transfer whose local endpoint receives bytes | `WorkerBackend::begin_external_ingress` |
| `ExternalEgress` | A cross-machine transfer whose local endpoint produces bytes | `WorkerBackend::begin_external_egress` |

`ExternalTransferDirection` distinguishes `Ingress` and `Egress` relative to
the worker. `WorkerExternalTransfer` stores the complete preprovisioned
contract:

- task id and phase;
- direction;
- the local resolved value location;
- exact byte count;
- one-hop measured route (`LinkId` list);
- exact transfer lane claims; and
- queue/completion submission slots.

Its accessors expose these fields without allowing mutation. The route and
lane-claim accessors return slices into the immutable boxed storage.

### Internal prepared work

`PreparedWorkerWork` is private and is the projection's normalized work form:

- `InitAdmission` stores device, destination location, packed-image byte
  count, and submission slots.
- `Calculation` stores device, kernel template, artifact, submission slots,
  resolved local inputs and outputs, and an optional resolved fault flag.
- `InternalTransfer` stores resolved source and destination endpoints, byte
  count, route, lane claims, submission slots, and either
  `WorkClass::InternalTransfer` or `WorkClass::ExitTransfer`.
- `Metric` stores metric purpose and id, metric slot, resolved value, and
  submission slots.
- `External` wraps `WorkerExternalTransfer` and therefore has no
  `BackendWork` value. It is submitted through the extra `WorkerBackend`
  hooks.

`role`, `class`, and `submission` derive the closed role, backend work class,
and submission slots. `backend_work` translates local variants into the
backend-neutral `BackendWork` enum. Init admission requires an image slice;
calculation and metric work require a loop iteration; internal transfers map
their stored class to `InternalTransfer` or `ExitTransfer`. An invalid class
returns `None`, which the session reports as a role/projection error rather
than inventing another backend path.

`ProjectedTask` retains the original task id, phase, schedule window, full
dependency list, dependencies that remain inside this projection, and the
prepared work. The projection sorts these records by task id so binary search
and deterministic hashing are available during execution.

## Building `WorkerProjection`

`WorkerProjection::derive(bundle, topology, assignment)` is the complete
pre-execution projection algorithm.

### Topology and ownership checks

1. It calls both `Topology::validate` and
   `Topology::validate_scheduling_properties`. Any validation detail is
   intentionally collapsed to `WorkerProjectionError::InvalidTopology`.
   Production scheduling therefore cannot be based on an estimated device or
   link property.
2. The bundle topology identity must equal the supplied topology identity.
   A mismatch reports both identities.
3. The assignment machine must exist.
4. The assignment node must exist, have `NodeRole::Worker`, belong to that
   machine, and own at least one device.
5. Node devices are sorted and collected into a `BTreeSet` used as the local
   ownership set.
6. Every local device must have a finalized arena layout, a reservation ledger
   entry, and one finalized init image. Missing objects report
   `MissingArena`, `MissingReservation`, or `MissingInitImage`.

### Task classification

Every bundle task is passed to `classify_task`. The result is either a
prepared local/external work record or `None` when the task belongs entirely
to another worker. A duplicate task id in the bundle is rejected. The
projected task id set is then used to filter each task's dependency list, so a
worker waits only for dependencies that are also represented locally; the
original full dependency list remains available for auditing and projection
identity hashing.

The classification matrix is:

| Finalized task | Local ownership | Phase/route rule | Projection result |
| --- | --- | --- | --- |
| Calculation | Calculation device local | Every input, output, and optional fault flag must resolve to a local device | `Calculation` / `Local` |
| Metric | Metric value's device local | Value location and later metric slot must be finalized | `Metric` / `Local` |
| External to local device | Destination local | Legal only in `RunPhase::Init` | `InitAdmission` |
| Local device to `External` | Source local | Legal only in `RunPhase::Exit` | `External` / `Egress` |
| Foreign device to local device | Destination local | Planner-expanded one-hop loop or exit transfer | `External` / `Ingress` |
| Local device to foreign device | Source local | Planner-expanded one-hop loop or exit transfer | `External` / `Egress` |
| Local device to local device | Both local | Init/loop uses internal-transfer class, exit uses exit-transfer class | `InternalTransfer` / `Local` |
| Both endpoints foreign | Neither local | No local execution | `None` |

An external endpoint paired with a foreign device is not a local admission and
is therefore omitted. Endpoint combinations that cannot be represented by the
resolved transfer ABI return `InvalidTask`.

For local calculations, `resolve_local_values` and
`resolve_local_value` require every referenced value to have a finalized
location on a local device. A foreign input, output, or fault flag is an
invalid task, not a reason to create a hidden transfer.

### Cross-machine transfer validation

`external_transfer` enforces the planner's cross-machine contract:

- init is rejected because the external-to-device init form is represented by
  `InitAdmission`, not a live external token;
- the route must contain exactly one measured link;
- that link must exist in the supplied topology;
- ingress requires `link.to == local.device`, while egress requires
  `link.from == local.device`;
- lane claims must contain exactly one `TransferLaneClaim::Link` for that link;
- no `TransferLaneClaim::External` is allowed on a cross-machine link; and
- the selected lane must be below the measured link's maximum in-flight count.

The original route, claims, byte count, and submission slots are copied into
the immutable `WorkerExternalTransfer`. A missing link claim or any mismatch
with the measured route reports `InvalidResource`.

### Resource validation

`validate_task_resources` checks the submission resources selected by the
finalized plan:

1. Local work uses its stored submission slots. External work uses the slots
   in its `WorkerExternalTransfer`.
2. The queue slot and completion slot must both exist and name the same
   device.
3. Local work's queue device must be in the worker's local device set.
4. External ingress uses the source device from the original transfer's
   `TransferEndpoint::Device` and requires the queue on that source device.
   An external source is invalid for this form.
5. External egress uses the transfer's local source device.
6. A metric slot must exist and refer to the same `MetricId` as the metric
   task. The source uses the `InvalidResource` detail "metric slot is absent"
   for both absence and id mismatch.

### Admission and projection identity

After classification, every local device must have exactly one
`InitAdmission`. Its destination value and byte count must equal the device's
finalized `InitDataImage` packing. A missing or duplicate admission is
rejected. No other init-phase task is allowed in the remote projection.

The artifact list is the sorted set of artifacts used by projected
calculation tasks. The projection digest is SHA-256 over a version tag,
bundle identity, topology identity, assignment machine and node, sorted
device ids, and each task's id, phase, role, and original dependency ids. It
does not hash mutable runtime state. The resulting digest is the stable
identity returned by `WorkerProjection::identity`.

The projection accessors expose bundle and topology identities, assignment,
devices, arena layouts, task `(TaskId, RunPhase, WorkerTaskRole)` records,
artifact ids, init-image byte counts, external transfer contracts, and binary
searched task roles/phases. They do not expose prepared backend tokens or
allow projection mutation.

## Projection failures

`WorkerProjectionError` is non-exhaustive and implements `Display` and
`Error`. The variants are the complete rejection surface of projection:

| Variant | Condition |
| --- | --- |
| `InvalidTopology` | Topology structure or schedulability validation failed |
| `TopologyMismatch` | Bundle and supplied topology identities differ |
| `UnknownMachine` | Assignment names no machine |
| `UnknownNode` | Assignment names no node |
| `NodeIsNotWorker` | Node exists but is not a worker node |
| `NodeMachineMismatch` | Node belongs to a different machine |
| `EmptyWorker` | Worker node owns no devices |
| `MissingArena` | Local device has no finalized arena layout |
| `MissingReservation` | Local device has no finalized reservation ledger entry |
| `MissingInitImage` | Local device has no finalized init image |
| `MissingInitAdmission` | No exact init admission was projected for a local device |
| `DuplicateInitAdmission` | More than one init admission targets a local device |
| `InvalidTask` | Task identity, phase, endpoint, value, route, or admission contract is not representable |
| `InvalidResource` | Queue, completion, metric slot, lane claim, or endpoint ownership is invalid |
| `CapacityOverflow` | Projection-related capacity arithmetic overflowed |

## Backend extension surface

`WorkerBackend` extends the sealed `Backend` trait. It is intentionally the
smallest addition needed for worker-only cross-machine execution:

| Hook | When called | Required result |
| --- | --- | --- |
| `bind_worker_resources` | Once during `prepare` | Backend resource bound to the exact bundle/projection |
| `prepare_external` | Once for each projected external transfer during `prepare` | Pre-realized external pending token |
| `begin_external_ingress` | On an ingress dispatch | Begin receiving the exact byte slice into worker-owned resources |
| `begin_external_egress` | On an egress dispatch | Begin producing exactly into the caller's stable destination slice |
| `poll_external` | On every external poll | `Pending` or completed byte count |
| `acknowledge_external_egress` | After the remote consumer acknowledges produced bytes | Release/retire the egress pending token |
| `quiesce_worker` | Cancel, fatal cleanup, failed preparation cleanup, or explicit worker quiesce | Return only after no native queue can access an arena |

All hooks receive a mutable `PhysicalCallBatch`. The session records that
batch whether the backend succeeds or fails. `WorkerBackendOperation` names
the physical operation used when wrapping backend errors:

`BindProjection`, `PrepareLocal(TaskId)`, `PrepareExternal(TaskId)`,
`AllocateArena(DeviceId)`, `SubmitLocal(TaskId)`, `PollLocal(TaskId)`,
`SubmitIngress(TaskId)`, `SubmitEgress(TaskId)`, `PollExternal(TaskId)`,
`AcknowledgeEgress(TaskId)`, `Quiesce`, `ReleaseArena(DeviceId)`, and
`DestroyResources`.

`ExternalTransferPoll` is the allocation-free external result type. A pending
poll carries no byte count. A complete poll carries the backend-reported byte
count, which the session converts to `ByteCount` and checks against the
finalized contract.

## Session state and ownership

`WorkerExecutionSession<B>` contains:

- the nonzero active `RunId`;
- `loop_iteration`, initialized to finalized iteration zero;
- the immutable `WorkerProjection`;
- the backend value and optional backend resource;
- a device-to-arena map;
- one `WorkerTaskSlot` per projected task;
- one `WorkerImage` per local device;
- `WorkerLifecycle`;
- the configured `Watchdog`; and
- the fixed-capacity `RunJournal`.

The session's task state is private:

| State | Meaning | Outgoing transitions |
| --- | --- | --- |
| `Idle` | Prepared but not submitted | `Active` by dispatch |
| `Active` | Native operation submitted | `Complete` by successful poll, `AwaitingAck` for completed egress, or `Quiesced` during cleanup |
| `AwaitingAck` | Egress bytes completed and waiting for consumer acknowledgment | `Complete` by `acknowledge_external_egress`, or `Quiesced` during cleanup |
| `Complete` | Contractually finished | No further dispatch |
| `Quiesced` | Native work was quiesced by cancellation or fatal cleanup | Not considered complete and cannot satisfy a dependency |

Each slot stores either a local pending token or an external pending token,
the projected task contract, its state, and a nonprogress poll counter.

Image state is independent of task state:

`Needed -> Receiving -> Submitted -> Complete -> Released`.

The image buffer is allocated and zeroed during session preparation, filled in
strict offset order, hashed on submission, and zeroed again on release or
fatal cleanup.

`WorkerLifecycle` is the closed session lifecycle:

`Prepared`, `Init`, `Loop`, `Exit`, `Cancelling`, `Finished`, and `Failed`.

The normal transitions are:

```text
Prepared --begin_init_image--> Init
Init --all init image polls complete--> Loop
Loop --begin_exit after all loop tasks complete--> Exit
Loop or Exit --cancel/quiesce--> Cancelling
Exit --all arenas released, finish--> Finished
Cancelling --all arenas released, finish--> Finished
any live session --fatal_cleanup--> Failed
```

`into_parts` succeeds only from `Finished` or `Failed` when the resource is
`None`. A session in any other lifecycle is returned in a boxed `Err` so the
caller retains ownership and can continue cleanup.

## Preparation flow

`WorkerExecutionSession::prepare(run, bundle, projection, backend, watchdog)`
is the only constructor for a live session.

1. A zero run id is rejected as `InvalidRun`, preserving the backend without
   attempting cleanup.
2. Projection bundle and topology identities must equal the supplied bundle,
   otherwise `BundleMismatch` is returned with the backend preserved.
3. `worker_journal_capacity` calls `JournalCapacity::for_bundle::<B>` and adds
   bounded capacity for two logical events per external transfer plus two
   lifecycle events, and bounded non-poll physical calls for each projected
   task plus one extra backend-operation bound. Checked arithmetic reports
   `CapacityOverflow`.
4. A `RunJournal` is allocated from that exact capacity.
5. `bind_worker_resources` binds the backend resource. Its physical calls are
   recorded even when it fails. A backend error is wrapped as
   `Backend { operation: BindProjection, ... }`; a journal-recording failure
   is retained as a separate cleanup error when applicable.
6. Each projected task is prepared in task-id order. External work calls
   `prepare_external`; local work constructs a `PendingRequest` containing the
   task id, phase, closed work class, and optional submission slots, then calls
   `prepare_pending`. Every operation's physical batch is recorded.
7. If any pending preparation or physical journal record fails,
   `preparation_failure` quiesces and destroys the already-bound resource,
   preserves the backend, and returns both the primary error and the first
   cleanup error if cleanup also fails.
8. `prepare_images` allocates one zeroed boxed buffer per projection layout,
   using the exact init admission byte count. Each image starts `Needed` with a
   zero expected digest and zero received count.
9. A `Prepared { run, bundle }` logical event is recorded. Failure here also
   invokes preparation cleanup.
10. The session stores iteration zero from the finalized loop schedule,
    resource ownership, empty arenas, idle task slots, prepared images, and
    `WorkerLifecycle::Prepared`.

Preparation realizes all local and external pending tokens before the first
init image is accepted. No live dispatch is allowed to lazily create a token.

The session's read-only accessors are deliberately small: `run_id` returns the
active run, `projection` returns the immutable projection reference,
`lifecycle` returns the closed lifecycle value, and `journal` returns the
journal reference. None exposes the backend, resource, pending token, arena,
task slot, or image buffer while the session is live.

## Init image flow

The remote protocol serializes one logical image at a time, while the session
validates each device independently.

### `begin_init_image`

`begin_init_image(run, device, bytes, digest)`:

1. checks the run and permits only `Prepared` or `Init` lifecycle;
2. finds the device image and requires state `Needed`;
3. requires the caller byte count to equal the finalized image byte count;
4. finds the matching finalized arena layout and rejects a duplicate arena;
5. calls `Backend::allocate_arena`, records the physical batch, and inserts
   the returned arena;
6. stores the expected digest, resets the received count, zeroes the image
   buffer, changes image state to `Receiving`, and changes lifecycle to
   `Init`; and
7. records `LogicalEvent::ArenaAllocated`.

The allocation is backend-owned, but the session owns the returned arena until
`release_arena` or fatal cleanup.

### `write_init_chunk`

`write_init_chunk(run, device, offset, bytes)` has no backend call. It requires
the image to be `Receiving`, requires `offset` to equal the current received
count, checks checked range arithmetic against the finalized image size, copies
the chunk into the preallocated image buffer, advances `received`, and returns
the input length. Out-of-order offsets, overflow, and overrun return an error
without inventing a second buffer or retry path.

### `submit_init_image`

`submit_init_image(run, device)` requires a receiving image whose received
count equals its exact byte count. It hashes the complete buffer with
SHA-256, compares it to the digest supplied at begin, finds the admission task,
checks its projected dependencies, constructs `BackendWork::InitAdmission`,
and calls `Backend::submit` with a read-only `ArenaSet`. The task becomes
`Active`, the image becomes `Submitted`, and
`LogicalEvent::InitAdmission` records the admission.

The work must be a local pending token with `InitAdmission` role. A metric
result or external pending token cannot be used for init admission.

### `poll_init_image`

`poll_init_image(run, device)` polls the admission's local token:

- `BackendPoll::Pending` returns `ExternalTransferPoll::Pending` and advances
  the watchdog through the common local poll helper.
- `BackendPoll::Complete { metric: None }` marks the task and image complete,
  records `TaskCompleted` for `RunPhase::Init`, and returns a complete transfer
  result with the exact image byte count.
- A complete admission carrying a metric is rejected as `MetricContract`.

When every local image is complete, the session changes lifecycle to `Loop`
and records `Initialized` followed by `LoopStarted`. The image buffers remain
resident until explicit arena release or cleanup.

## Dispatch and polling

### Common dispatch gate

`ensure_dispatch(run, phase, task, role)` is shared by local and external
dispatch methods. It:

1. checks the active run;
2. maps the lifecycle to an active phase (`Loop` or `Exit`);
3. requires the caller phase to equal the active phase and the projected task
   phase to equal it;
4. requires the projected role to equal the requested role;
5. requires task state `Idle`;
6. requires every projected dependency to be `Complete`; and
7. rejects a task if an active or acknowledgement-waiting task has a schedule
   window that does not overlap the new task's window.

The final rule is implemented exactly as
`active.state in {Active, AwaitingAck} && !active.window.overlaps(new.window)`.
It is not a backend queue heuristic and does not infer a new schedule.

### Local tasks

`submit_task(run, phase, task)` uses the common gate with `Local` role, obtains
the stored class, constructs backend work with the session's fixed iteration
zero, and calls `Backend::submit` against the read-only arena set. On success
the slot becomes `Active` and `TaskSubmitted { phase, task, class }` is
recorded.

`poll_task(run, task)` requires a local active slot, calls `Backend::poll`,
records the physical poll, and returns either `WorkerTaskPoll::Pending` or a
complete result. Pending increments the per-task watchdog counter. Completion
resets it, validates the metric contract, marks the task `Complete`, and
records `TaskCompleted`.

Metric validation is fail-closed:

- `MetricPurpose::User` must return a value whose `F32` or `I32` variant
  matches the finalized value dtype, and the value is returned to the caller.
- `MetricPurpose::FaultReadback` with `I32(0)` is successful and yields no
  user metric.
- A nonzero fault readback becomes `DeviceFault { readback, code }`.
- A metric with an incompatible type, a missing metric where one is required,
  or a metric attached to a non-metric task becomes `MetricContract`.
- Non-metric local tasks may complete only with `None`.

### External ingress

`begin_external_ingress(run, phase, task, bytes)` uses the common gate with
`ExternalIngress`, checks the supplied slice length against the immutable
transfer byte count, and calls `WorkerBackend::begin_external_ingress` with
the read-only arena set, external pending token, transfer contract, and stable
byte slice. Success marks the slot `Active` and records
`ExternalTransferSubmitted` with direction `Ingress`.

`poll_external_ingress` calls the shared external polling helper. Pending
increments the watchdog. Completion converts and checks the backend byte count,
marks the slot `Complete`, resets the watchdog counter, and records
`ExternalTransferCompleted`.

### External egress

`begin_external_egress(run, phase, task, destination)` is the egress analogue:
the destination length must equal the finalized byte count, the backend fills
the caller-owned stable slice, the slot becomes `Active`, and
`ExternalTransferSubmitted` records direction `Egress`.

`poll_external_egress` shares the same byte-count and watchdog checks. On a
complete poll, the byte count is returned and the slot becomes
`AwaitingAck`, not `Complete`; no completed logical event is recorded until
the consumer acknowledges the bytes.

`acknowledge_external_egress(run, task)` requires an egress contract in
`AwaitingAck`, calls `acknowledge_external_egress`, marks the slot `Complete`,
and records `ExternalTransferCompleted`. A second acknowledgment or an
ingress acknowledgment is rejected.

## Exit, cancellation, and resource teardown

### Starting exit

`begin_exit(run)` requires lifecycle `Loop` and requires every projected loop
task, including projected cross-machine tasks, to be `Complete`. It then
changes lifecycle to `Exit` and records `LoopCompleted`. Exit task dispatches
use the same dependency, role, schedule-window, and backend-submit rules as
loop dispatches, but use `RunPhase::Exit` and `ExitTransfer` work for local
exit transfers.

### Cancellation

`cancel(run)` accepts only `Loop` or `Exit`. It calls `quiesce_worker`, records
the physical calls and `WorkerQuiesced`, changes every `Active` or
`AwaitingAck` task to `Quiesced`, then changes lifecycle to `Cancelling`.
Idle and already complete tasks are left unchanged. Cancellation does not
release arenas or destroy the backend resource; the caller must still invoke
`release_arena` for every device and then `finish`.

### Releasing an arena

`release_arena(run, device)` accepts `Exit` only after every projected exit
task is complete, or accepts any device while `Cancelling`. It rejects other
lifecycle states. `release_one_arena` requires an unreleased image and an
existing arena, calls `Backend::release_arena`, records the physical batch,
marks the image `Released`, zeroes its buffer, removes the arena, and records
`ArenaReleased`. A repeated release reports `ArenaAlreadyReleased`.

### Normal finish

`finish(run)` accepts only `Exit` or `Cancelling` and requires the arena map to
be empty. It takes the backend resource, calls `Backend::destroy_resources`,
records the physical calls, changes lifecycle to `Finished`, and records
`Exited`. If any arena remains or the resource was already destroyed, finish
returns an invalid-lifecycle error.

### Fatal cleanup

`fatal_cleanup()` is the terminal cleanup path used by the remote adapter's
fault handler and `Drop` implementation. Its order is intentionally strict:

1. If a resource exists, call `quiesce_worker` and record its physical calls.
   A backend quiesce failure changes lifecycle to `Failed` and returns
   immediately, preserving every arena because native work may still refer to
   it.
2. On successful quiesce, mark active and acknowledgement-waiting tasks
   `Quiesced` and record `WorkerQuiesced`. A journal error is retained as the
   first error while cleanup continues.
3. Release every remaining arena exactly once. Continue after an individual
   release failure, retaining the first error.
4. Zero every init image buffer.
5. Take and destroy the resource, again retaining the first error.
6. Set lifecycle `Failed` and return the first error, or `Ok(())` if every
   cleanup operation and journal record succeeded.

`destroy_prepared_resource` implements the same quiesce-then-destroy order for
preparation failures. It records physical calls for both operations and
returns only the first backend or journal error.

## Journal and output contract

The worker produces no files and performs no direct network I/O. Its outputs
are owned runtime objects and journal records:

- `WorkerProjection` exposes deterministic identity, selected devices,
  layouts, artifacts, task roles/phases, init sizes, and external contracts.
- The backend receives immutable `BackendWork` values, read-only arena views,
  and worker-only external hooks.
- `WorkerTaskPoll` and `ExternalTransferPoll` expose nonblocking completion
  and validated metric/byte results.
- `WorkerExecutionError` preserves the exact validation, lifecycle, capacity,
  watchdog, device-fault, journal, and backend failure category.
- `RunJournal` records logical contract events separately from bounded
  backend-reported `PhysicalCall` records. Worker-specific logical events are
  `Prepared`, `ArenaAllocated`, `InitAdmission`, `TaskSubmitted`,
  `TaskCompleted`, `ExternalTransferSubmitted`,
  `ExternalTransferCompleted`, `Initialized`, `LoopStarted`,
  `LoopCompleted`, `WorkerQuiesced`, `ArenaReleased`, and `Exited`.

`backend_result` always records the supplied physical call batch before
returning the backend result. If physical journal recording fails, the
journal error is returned instead of masking it with a backend error.
`journal_result` maps the executor journal result into
`WorkerExecutionError::Journal` for logical events.

The watchdog counts only nonprogress polls. A pending local or external poll
increments the task counter. A completion resets it. Reaching the configured
`Watchdog::max_nonprogress_polls` returns `WatchdogExpired`; the session does
not retry or silently detach the pending token.

The remaining private helpers provide the single validation and accounting
path used by the public operations:

- `poll_local_pending` selects a local pending token, calls `Backend::poll`,
  records the physical batch, and applies the watchdog result.
- `poll_external_direction` performs the same work for an external token,
  verifies direction and exact byte count, and applies the ingress-complete or
  egress-awaiting-ack transition.
- `validate_metric_completion` is the only place that interprets a backend
  metric and implements the user/fault-readback rules described above.
- `ensure_run`, `active_phase`, `task_index`, `image_index`, and
  `external_contract` centralize id and lifecycle lookups. A failed binary
  search is `UnknownTask` or `UnknownDevice`, never a synthesized slot.
- `quiesce` performs the live-session quiesce call, marks active slots
  `Quiesced`, and records `WorkerQuiesced`.
- `release_one_arena` is the one arena removal path used by normal release and
  fatal cleanup.
- `byte_count` is the checked `usize` to `ByteCount` conversion used for
  caller and backend transfer lengths.

## Worker execution failures

`WorkerExecutionError<E>` is non-exhaustive, implements `Display` and
`Error`, and preserves a backend error type `E` when the failure originated in
the backend. Its variants are:

| Variant | Meaning |
| --- | --- |
| `Projection` | Projection validation failed while preparing or validating a task |
| `InvalidRun` | Run id is zero at preparation |
| `RunMismatch` | Operation used a run id different from the active session |
| `BundleMismatch` | Session projection identities differ from the supplied bundle |
| `UnknownTask` / `UnknownDevice` | Identifier is outside the projection |
| `WrongRole` | Operation requested a role different from the projected role |
| `WrongPhase` | Operation phase differs from lifecycle or task phase |
| `InvalidLifecycle` | Operation is not valid in the current lifecycle, with a static detail |
| `DuplicateDispatch` | Image or task was dispatched after leaving `Needed`/`Idle` |
| `TaskNotActive` | Poll or acknowledgment has no active matching operation |
| `DependencyIncomplete` | A projected dependency is not `Complete` |
| `PhaseIncomplete` | Exit or release was attempted while a required phase task remains incomplete |
| `ScheduleConflict` | A live task has a non-overlapping schedule window |
| `ByteCountMismatch` | Caller or backend byte count differs from the finalized contract |
| `InitOffsetMismatch` | Init chunk offset is not the next expected contiguous offset |
| `InitDigestMismatch` | Received init bytes hash differently from the announced digest |
| `MetricContract` | Metric presence or dtype violates the finalized task contract |
| `DeviceFault` | Checked device work read a nonzero fault flag |
| `WatchdogExpired` | A task made no progress for the configured poll bound |
| `ArenaAlreadyReleased` | Requested arena is absent or already marked released |
| `Backend` | A `WorkerBackend` or base `Backend` operation returned `E` |
| `Journal` | Logical or physical journal accounting failed |
| `CapacityOverflow` | Checked size, count, or byte conversion overflowed |

The `source` method exposes projection, backend, and journal causes. Other
variants are contract errors with no nested source.

Preparation has an additional ownership-preserving error object:
`WorkerPrepareFailure<B>` stores the boxed primary error, an optional boxed
cleanup error, and the recovered backend. `error()` and `cleanup_error()`
borrow those values. `into_parts()` consumes the failure and returns
`WorkerPrepareFailureParts { error, cleanup_error, backend }`, allowing the
caller to map both faults while retaining the backend for reuse or disposal.

## Concrete remote caller

`remote/src/executor_driver.rs` is the repository's direct caller of this
module. `ExecutorWorkerDriver<B>` stores the finalized bundle, projection,
expected provisioned-program digest, watchdog, an optional backend, an
optional worker session, the active run, and one pending init-chunk record.

### Construction and program proof

`ExecutorWorkerDriver::new` first derives the projection. It then validates
the provisioned wire program against that projection and topology:

- program manifest bundle identity must equal projection bundle identity;
- program device count and every image byte count must match;
- the program task list must equal the projection's non-init tasks in order;
- local projected tasks must be `ProgramTaskKind::Driver`;
- external ingress and egress tasks must be `ProgramTaskKind::CrossTransfer`;
- init admissions must not leak into the runtime task manifest;
- cross-transfer count, task id, direction, byte count, and the single
  topology link's capacity resource and duplex mode must match.

Any mismatch becomes `ExecutorDriverBuildError::ProgramMismatch` (or its
wrapped projection error). No session is prepared until this proof succeeds.

### `WorkerDriver` forwarding

The adapter's `WorkerDriver` implementation is a direct, state-checked
forwarder:

| Remote driver method | Session call |
| --- | --- |
| `prepare` | `WorkerExecutionSession::prepare` after program digest and empty-session checks |
| `begin_init_image` | `begin_init_image` |
| `begin_init_chunk` | `write_init_chunk`, then store the copied length as one pending chunk |
| `poll_init_chunk` | Return the stored copied length and clear the pending chunk |
| `finish_init_image` | `submit_init_image` |
| `poll_init_image` | `poll_init_image`, mapped to `DriverTransferPoll` |
| `submit_task` | `submit_task` using the session's active phase |
| `poll_task` | `poll_task`, mapping `MetricValue` to the remote metric enum |
| `begin_receive_user_data` / `poll_receive_user_data` | External ingress begin/poll |
| `begin_produce_user_data` / `poll_produce_user_data` | External egress begin/poll |
| `user_data_acked` | `acknowledge_external_egress` |
| `cancel` | `cancel` (the reason is intentionally not interpreted by the executor) |
| `begin_exit` | `begin_exit` |
| `release_arena` | `release_arena` |
| `finish` | `finish`, then recover the backend with `into_parts` |
| `cleanup_after_fault` | `fatal_cleanup`, then recover the backend |

`ExecutorWorkerDriver::prepare` takes the backend out of its option before
calling the session constructor. A successful session owns it. A preparation
failure consumes `WorkerPrepareFailure`, puts the recovered backend back, and
maps the primary and cleanup errors to distinct `DriverFaultCode` values.

Every session error is mapped to `DriverFaultCode::EXECUTOR_OPERATION_FAILED`
with a bounded detail number. The mapping preserves useful identifiers for
tasks/devices/runs, lifecycle codes for invalid lifecycle errors, backend
operation codes for backend failures, and reserved high values for journal or
capacity failures. `Drop` calls `fatal_cleanup` when a session is still live.

The adapter uses these stable operation codes when encoding a backend error:

| Code | `WorkerBackendOperation` |
| --- | --- |
| 1 | `BindProjection` |
| 2 | `PrepareLocal` |
| 3 | `PrepareExternal` |
| 4 | `AllocateArena` |
| 5 | `SubmitLocal` |
| 6 | `PollLocal` |
| 7 | `SubmitIngress` |
| 8 | `SubmitEgress` |
| 9 | `PollExternal` |
| 10 | `AcknowledgeEgress` |
| 11 | `Quiesce` |
| 12 | `ReleaseArena` |
| 13 | `DestroyResources` |

Lifecycle details are encoded as `Prepared=1`, `Init=2`, `Loop=3`, `Exit=4`,
`Cancelling=5`, `Finished=6`, and `Failed=7`. Projection and bundle mismatch
use detail `1`; journal and capacity errors use the reserved high values.

### Remote handshake and init

The remote worker state machine calls the adapter as follows:

1. `WorkerHandshake` validates the master's hello and manifest. On `Prepare`,
   it calls `ExecutorWorkerDriver::prepare`, which invokes session preparation
   and pre-realizes all local and external pending tokens. The worker sends
   `PrepareAck` only after that succeeds.
2. `WorkerInit` receives one `InitBegin`, calls `begin_init_image`, receives
   ordered `InitChunk` messages, calls `begin_init_chunk`, waits for the
   matching `poll_init_chunk`, then receives `InitEnd` and calls
   `finish_init_image`.
3. `poll_init_image` drives the asynchronous admission. Once complete, the
   remote state records the device complete and sends `InitAck`.
4. After every device image is complete and `InitComplete` is received, the
   worker sends `InitCompleteAck` and enters `WorkerRun`. The session has
   already changed to lifecycle `Loop` when the final image admission polled
   complete.

The adapter's init chunk method is intentionally a synchronous copy into the
session's preallocated image buffer. Its remote `poll_init_chunk` reports that
copy complete; no native backend call occurs for a chunk. Native admission
starts only at `finish_init_image`.

### Remote loop and external data

In `WorkerRun`, an `Execute` message validates the remote static task state,
calls `submit_task`, and places the task in a remote active slot. Progress
round-robins those slots through `poll_task`. A completed user metric is sent
as `Metric` before `TaskComplete`; a fault or executor error becomes a
`TaskFailed`/driver fault path.

For a master-to-worker `UserData` message, the remote state checks the static
cross-transfer contract and schedule, copies into stable scratch storage,
calls `begin_receive_user_data`, and polls until the session reports the exact
byte count. For a worker-to-master `DataRequest`, it calls
`begin_produce_user_data` with stable scratch storage, polls until complete,
sends the produced bytes, and waits for `DataAck` before calling
`user_data_acked`. This ordering corresponds exactly to the session's
`Active`, `Complete`, and `AwaitingAck` external task states.

### Remote exit and cancellation

`BeginExit` is accepted only when remote loop task states are complete. The
adapter calls `begin_exit`, which independently rechecks projected loop task
completion and changes the session to `Exit`. Exit `Execute`, `UserData`, and
`DataRequest` messages use the same forwarding methods with exit phase.

The master then sends one `Release` per device after exit tasks complete. The
worker calls `release_arena`, sends `ReleaseAck`, and only accepts
`ExitComplete` after every release is acknowledged. The adapter's `finish`
call destroys backend resources and returns the backend to the completed
driver. A cancel message instead calls session `cancel`, enters the remote
`WorkerCancelled` release loop, releases every arena, calls `finish`, and
sends `CancelAck`.

If any adapter call returns a driver fault, the remote session invokes
`cleanup_after_fault`. That calls `fatal_cleanup` before the terminal fault is
reported. A cleanup failure is combined with the primary fault using
`DriverFault::cleanup_failed`; the protocol never reports a supposedly
terminal fault while native work may still reference an arena.

## Method-to-backend trace

The following compact trace covers every native call made by the session:

```text
prepare
  bind_worker_resources(bundle, projection)
  for each external task: prepare_external(resource, transfer)
  for each local task: prepare_pending(resource, PendingRequest)

init image
  begin_init_image: allocate_arena(resource, layout)
  write_init_chunk: no backend call, copy into image buffer
  submit_init_image: submit(resource, ArenaSet, local pending, InitAdmissionWork)
  poll_init_image: poll(resource, local pending)

local loop/exit task
  submit_task: submit(resource, ArenaSet, local pending, BackendWork)
  poll_task: poll(resource, local pending)

external ingress
  begin_external_ingress: begin_external_ingress(resource, arenas, pending, transfer, bytes)
  poll_external_ingress: poll_external(resource, pending)

external egress
  begin_external_egress: begin_external_egress(resource, arenas, pending, transfer, destination)
  poll_external_egress: poll_external(resource, pending)
  acknowledge_external_egress: acknowledge_external_egress(resource, pending, transfer)

terminal paths
  cancel/fatal cleanup/preparation cleanup: quiesce_worker(resource)
  release_arena: release_arena(resource, device, arena)
  finish/fatal cleanup: destroy_resources(resource)
```

Every listed call receives a bounded physical-call batch, and every result is
validated against the immutable projection before the session publishes a
logical completion or state transition. This is the complete worker-side
execution boundary; no alternate implementation, fallback backend, or hidden
resource source exists in `executor/src/worker.rs`.
