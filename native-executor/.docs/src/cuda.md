# CUDA native executor

This document describes the CUDA Driver adapter in
`native-executor/src/cuda.rs`. The adapter is a closed implementation of
`recipe_executor::Backend`: it receives an immutable finalized task and
translates it to already-realized CUDA Driver objects. Compilation, artifact
materialization, discovery, queue creation, event creation, memory allocation,
and topology changes are preparation work. They are not performed by a live
`submit` or `poll` operation.

The public native path is:

```text
NativeGpuProbe / with_native_execution_bindings
    -> CudaBinding<'context>
    -> LocalCandidateFactory::realize_candidate
    -> CudaPreparedResources::realize
    -> maximum-concurrency warm passes
    -> CudaPreparedResources::bind_candidate
    -> CudaBackend::from_warmed
    -> LocalBackend::bind_resources
    -> CudaBackend::Backend methods
    -> CudaResources::submit / poll / collect_exit
    -> explicit arena release and resource destruction
```

`CudaResources` is the authoritative owner of CUDA objects after handoff.
`CudaPending` is the one reusable token for one finalized task. The executor
owns the run lifecycle and scheduling decisions; this module owns only the
CUDA realization and operation contract.

Source anchors:

- `native-executor/src/cuda.rs:1-146`
- `native-executor/src/cuda.rs:221-421`
- `native-executor/src/cuda.rs:438-999`
- `native-executor/src/cuda.rs:1001-1367`
- `native-executor/src/cuda.rs:1382-2266`
- `native-executor/src/cuda_ffi.rs:1-54`
- `native-executor/src/local.rs:1422-2095`
- `native-executor/src/bridge.rs:169-333`, `1066-1635`
- `executor/src/backend.rs:327-470`

## Bindings and arenas

### `CudaBinding`

`CudaBinding<'context>` (`cuda.rs`, `CudaBinding`) borrows one open
`recipe_cuda::Context` and carries:

| Field | Meaning | Validation or consumer |
| --- | --- | --- |
| `device` | Recipe device identity | Keys all resource maps and arena ownership checks. |
| `context` | Exact CUDA context for that device | Borrowed for the complete binding/resource lifetime. |
| `deployment` | Reopened device UUID, compute capability, driver version, and driver capabilities | `validate_binding` requires the context UUID and compute capability to match. Artifact compatibility is checked against this identity. |
| `maximum_submission_queues` | Bound on distinct queue slots used by the selected tasks | `CudaResources::realize` checks it with `ensure_submission_queue_capacity`. |
| `enabled_display_connectors` | Measured display reservation evidence | Read by the local candidate factory when producing reservation evidence. |

`CudaBinding::new` stores these values and performs no driver operation.
`available_bytes` calls `Context::memory_info` on this exact context and maps
the driver's `usize` free-byte count into Recipe's `u64` byte unit. A failed
conversion is `Error::ArenaMismatch`, not a truncated value. The native probe
sets `maximum_submission_queues` to `CUDA_MAXIMUM_SUBMISSION_QUEUES` (32),
because the CUDA Driver API does not expose a finite stream-count attribute.
This is an executor ceiling, not a hardware measurement.

### Driver and context prerequisite

`CudaBinding` is produced only after `native-probe` has reopened the exact
measured CUDA deployment. `native-probe/src/cuda.rs` checks for an NVIDIA PCI
accelerator, opens the configured `libcuda.so.1` or `libcuda.so`, loads every
required Driver symbol, and runs exhaustive discovery. A descriptor includes
the stable UUID and PCI identity, compute capability, total memory, driver and
toolchain identities, one host-to-device and one device-to-host transfer lane,
the measured workgroup limit, asynchronous-submission capability, and the
fixed queue ceiling. A current device that is absent from the measured profile
or has a changed descriptor is rejected before a binding can reach this crate.

`recipe-cuda::Context::create` validates the context flag bitset, creates the
context with `cuCtxCreate_v2`, immediately pops it, and requires that the
popped handle is exactly the created handle. `Context` is deliberately
`!Send` and `!Sync`; all CUDA resources in this module borrow the same context
object. The runtime enters and leaves that context around every Driver call,
and resource operations reject pointer-distinct contexts even when they name
the same GPU. The binding callback retains the context for the complete
candidate and run scope, so a `CudaArena`, module, stream, event, pinned
buffer, or pending token cannot outlive it.

The Driver loader requires the symbols used by discovery, context management,
module loading, memory, streams, events, copies, and launches. UUID-v2,
module-loading-mode, and rich error-name/error-string symbols are optional.
Missing required symbols are a discovery failure, not a deferred runtime
fallback. Live submit and poll use allocation-free numeric status conversion;
rich Driver text is collected only during discovery and realization.

`CudaArena<'context>` wraps one `recipe_cuda::DeviceBuffer` and its Recipe
`DeviceId`. `CudaArena::release` consumes the wrapper and calls
`DeviceBuffer::free`. Every operation obtains an arena through
`checked_arena` before passing a pointer to CUDA. That check requires:

1. an arena for the resolved value's device;
2. matching arena and resolved-value device identities;
3. checked `arena_offset + bytes`; and
4. the resulting range to fit within the device allocation.

The byte and offset conversions are checked against the host `usize` and
return `Error::IntegerOverflow` when the native ABI cannot represent them.

After realization, `CudaResources::available_bytes(device)` performs the same
exact-context memory observation through that device's retained
`DeviceResources`. The local candidate stabilizer uses this post-warm query
after releasing warm arenas to derive capacity evidence; it does not mutate the
finalized scheduler contract on later observations.

## Backend state and one-shot binding

`CudaBackend` is a state machine:

| State | Contents | Entry |
| --- | --- | --- |
| `Ready` | Borrowed bindings and runtime artifacts | `CudaBackend::new` |
| `Prepared` | Candidate resources plus a prepared handoff marker | `from_prepared`; retained for a direct finalized handoff |
| `Warmed` | `CudaResources` produced by a complete warm trace | `from_warmed`; this is the production handoff |
| `Bound` | No backend-owned payload | Set before every bind attempt |

`bind_resources` records `PhysicalCall::BindResources`, replaces the state
with `Bound`, and then dispatches:

- `Ready` validates the complete `FinalizedBundle` with `ExecutionPlan::validate`
  and realizes all selected CUDA resources.
- `Prepared` requires its handoff to have been validated for the exact bundle
  identity and task set, then consumes it.
- `Warmed` calls `CudaResources::validate_handoff` for all finalized tasks.
- `Bound` returns `Error::BackendState` saying that resources may be bound only
  once.

`bind_partition` is the same transition for `LocalBackend`: it validates a
partition plan when starting from `Ready`, binds a prepared handoff, or
validates a warmed handoff. Since the state is replaced before validation, a
failed bind is fail-closed and cannot be retried through the same backend.

## Candidate realization and warm handoff

`CudaPreparedResources::realize` is the pre-final candidate path used by
`LocalCandidateFactory::realize_candidate`:

1. CUDA bindings are keyed by device. Duplicate bindings are rejected.
2. Every selected calculation device must have a binding.
3. Runtime artifact IDs must be exactly the selected calculation artifact IDs,
   with no duplicate, missing, or unexpected artifact.
4. Each bound device is checked with `validate_binding` and `realize_device`.
5. The handoff stores `CudaPreparedHandoff::Candidate` with the exact runtime
   artifact images and the realized per-device objects.

The candidate draft has no finalized arena offsets. Native objects are created
from its resource manifest and task set; the warm trace uses a provisional
bundle with candidate-resolved locations solely to exercise those objects.
`bind_candidate` and the enclosing local handoff then validate the finalized
bundle's exact locations, init images, artifact identities, reservations, and
task set before any finalized run can use the warmed resources.

`bind_candidate` may consume only a `Candidate` handoff. It compares every
device's realized `InitImageContract` with the finalized bundle's init-image
manifest, validates the partition `ExecutionPlan` and task contracts, and
returns `CudaResources` with `poisoned == false`. A `Finalized` handoff cannot
be rebound as a warm candidate. The retained `validate_handoff` method upgrades
a candidate to `Finalized` after the same init-image, plan, and contract checks;
`bind` then requires exact bundle identity and exact task-set equality.

The production local preparation flow is implemented in
`native-executor/src/local.rs`:

```text
LocalCandidateFactory::realize_candidate
    -> CudaPreparedResources::realize
    -> LocalPreparedSession::activate_warm_resources
    -> CudaPreparedResources::bind_candidate
    -> run_warm_trace (prepare, submit, poll, recycle each token)
    -> observe capacity after releasing warm arenas
    -> LocalPreparedSession::into_backend
    -> CudaBackend::from_warmed
```

Every warm task is submitted through the same CUDA operation methods used by a
real run. `CudaResources::recycle_pending` accepts only a terminal token in the
`Terminal` native state and removes its task from `prepared_tasks`, allowing the
next warm pass or final handoff to prepare exactly one token again. It does not
allocate a replacement event or queue.

The enclosing `LocalPreparedSession` has an explicit physical state transition:

```text
Candidate resources
    -> Transition
    -> Warm resources + temporary arenas
    -> (warm pass, recycle all tokens)
    -> release temporary arenas
    -> Observed capacity
    -> into_backend consumes Warm resources
```

Activation creates a provisional finalized bundle from the immutable candidate
and reservation ledger, then calls each child `bind_candidate`. If CUDA binding
or a later HSA, host, or bridge bind fails, the local cleanup path destroys the
already-realized native resources and marks the physical session `Destroyed`.
The session cannot silently recreate them. A warm pass must use the same
candidate identity and the expected monotonically increasing pass number. The
warm scheduler respects task dependencies, schedule windows, and phase; it
stalls if no task is runnable and no pending token remains, and fails after its
bounded idle-poll limit if pending work never reaches terminal state.

The first initial-capacity snapshot comes from each exact CUDA binding before
candidate realization. After a complete warm pass, temporary CUDA arenas are
released and `CudaResources::available_bytes` observes the live free-memory
counter with modules, queues, staging, metric buffers, scratch, and other
persistent objects still resident. Capacity accounting caps live bytes at the
initial observation, computes runtime overhead as `initial - capped_live`, and
subtracts the held reservation headroom to obtain Recipe-usable bytes. The
first successful snapshot is anchored. Later display or allocator counter
drift cannot rewrite the candidate's finalized capacity contract.

`into_backend` is legal only after the final warm pass has been observed, all
temporary arenas have been released, the finalized bundle identity and every
candidate resource/artifact/reservation field match exactly, and CUDA's own
`validate_handoff` has confirmed that every warm pending token was recycled.

## Per-device realization

`CudaResources::realize` first validates the binding set against the plan:

- duplicate binding devices produce `DuplicateDevice`;
- every planned device must have a binding, otherwise `MissingDevice`;
- a binding for a device absent from the plan produces `UnexpectedDevice`;
- the selected task list is the complete bundle when called by
  `bind_resources`, or the exact partition task set for `bind_partition` and
  warm handoff;
- the distinct queue IDs used by selected tasks are counted per device and
  compared with the binding's maximum queue value;
- each device reservation must exist and use
  `ReservationMechanism::EnforcedQuota`. Any other mechanism is an
  `ArenaMismatch`.

`ExecutionPlan::validate` and `validate_partition` are the immutable admission
gate immediately before this realization. They authenticate runtime artifact
IDs, SHA-256 bytes, ABI entry symbols, target backend/architecture, workgroup
limits, and every calculation ABI mapping. They also derive a submission map
for every bundle task and require each selected task's planned device to be in
the partition device set. A partition retains only the runtime images named by
its selected calculations, but its submission map still describes the complete
finalized bundle. CUDA therefore never infers a queue, completion, artifact, or
device assignment from the incoming `BackendWork` value.

`realize_device` then creates all driver objects needed by that device before
the run can enter `init`:

| Resource | Creation rule and runtime role |
| --- | --- |
| `queues` | One nonblocking `Stream` for every resource queue slot on this device that a selected task uses. |
| `completions` | One timing-disabled `Event` per used completion slot, initially `Available`. Event slots are checked out only by event-backed H2D/D2H operations. |
| `staging` | One pinned host buffer of the manifest's `pinned_staging` size. The device must have an init-image manifest and its image bytes must fit this buffer. |
| `scratch` | One device buffer when the manifest declares a nonzero scratch size; zero or absent scratch means no allocation. |
| `modules` | One boxed `Module` per distinct runtime cubin content digest. |
| `artifacts` | One logical `LoadedArtifact` per artifact ID, retaining its ABI and a function handle resolved from the shared module. |
| `invocations` | One `ParameterBlock` per calculation completion slot, sized to the maximum ABI argument count of calculations using that slot. |
| `metric_buffers` | One four-byte pinned host buffer for each selected metric whose value belongs to this device. |
| `egress` | One zero-filled host `Vec<u8>` for each selected exit transfer from this device to an external endpoint. |

For every calculation artifact assigned to the device, realization requires a
`RuntimeArtifactKind::Cuda`. It then:

1. calls `validate_artifact_compatibility` with the exact cubin bytes,
   deployment identity, and artifact identity;
2. computes `sm = 10 * major + minor`, checking arithmetic and conversion to
   the inspector's `u8` domain;
3. calls `inspect_cubin` for that SM and the immutable ABI entry symbol;
4. rejects a reported entry symbol that differs from the ABI; and
5. groups artifacts by `ArtifactDigest`, requiring identical bytes when a
   digest is shared.

`validate_artifact_compatibility` checks the cubin's nonempty ELF magic,
declared SHA-256, expected identity fields, target compute capability, driver
version range, and every required Driver symbol. `inspect_cubin` independently
parses the CUDA ELF machine and accepted CUDA OS-ABI tags, decodes the SM field
for the observed ELF layout, audits symbols, requires the exact function
symbol and matching executable `.text.<entry>` section, and rejects an empty
or non-executable entry body. A successful identity check therefore does not
replace structural cubin inspection, and structural inspection does not replace
deployment compatibility.

Each distinct digest is loaded once with `Module::load_cubin`. Every logical
artifact then resolves its ABI entry with `Module::function`. The module is
stored in a stable `Box` so function handles remain valid. The explicit
`Function` lifetime widening in `realize_device` is safe only under the local
ownership invariant: `artifacts` is dropped before `modules`, modules never
escape `CudaResources`, and teardown unloads modules only after function
handles have been dropped.

No allocation, module load, function lookup, queue creation, or completion
creation occurs in a live submit or poll path.

## Task contracts

`task_contracts` derives an immutable `TaskContract` for each selected task.
The contract stores phase, `WorkClass`, submission slots, the init-image
contract when applicable, and exact transfer route and lane claims.

Transfer classification is closed:

| Finalized phase and endpoints | Work class |
| --- | --- |
| `Init`, external to device | `InitAdmission` |
| `Init` or `Loop`, device to device | `InternalTransfer` |
| `Exit`, device to device or external | `ExitTransfer` |
| Any other endpoint/phase combination | `Protocol` error |

For init admission, the finalized transfer endpoints must resolve to a device
image. The image value and byte count must equal that device's
`InitDataImage` manifest. A repeated task ID is a protocol error. Calculation
and metric contracts contain no route or lane claims.

`prepare_pending` verifies the request against this contract, looks up the
immutable `PlannedSubmission`, verifies that its queue and completion slots
were realized for the target device, and inserts the task into
`prepared_tasks`. A second preparation of the same task is rejected. The
returned `CudaPending` is ready but has not submitted anything.

## Pending token state

`CudaPending<'context>` records the task, phase, class, planned device, queue,
completion slot, native state, post-completion action, and a terminal flag.

```text
Ready --submit--> Active(Event | Stream) --poll complete--> Terminal
  ^                                         |
  |-------------- prepare_loop_pending -----|
                    (loop only, rearm)
```

`validate_ready` requires exact task/class/device/queue/completion/submission
identity, native `Ready`, and `terminal == false`. `activate` changes only a
validated ready token to `Active` and stores its `PendingAction`.

The loop path chooses its next transition from the token itself, not from the
global iteration number:

- `Ready, terminal == false` means `UsePrepared` for the first active loop
  iteration;
- `Terminal, terminal == true` means `Rearm`, which resets native state to
  `Ready`, clears the action, and clears `terminal`;
- `Active`, or any inconsistent state, is rejected as an active token being
  submitted again.

`CudaBackend::supports_loop_repetition` returns `true`, so the executor may
reuse a loop token for all finalized iterations. The adapter also returns
`true` from `supports_same_queue_pipelining`: stream-backed CUDA operations
use queue ordering and `cuStreamQuery` rather than one event per task. The
executor may therefore submit a later task behind an incomplete predecessor on
the same queue. Resource and arena ownership still lasts until the queue is
idle.

`CudaPending::Drop` is deliberately conservative. A nonterminal active native
token is forgotten rather than dropped, because CUDA has no cancellation
operation and dropping the token could release an operation borrow while the
driver is still accessing a resource. This is not cancellation. The associated
stream or completion slot can remain live, and normal teardown will expose the
unfinished operation as a stream or completion ownership error. A terminal
token is safe to drop.

## Submission operations

`CudaResources::submit` performs these checks in order: healthy backend, task
submission in the immutable plan, pending-token identity, and task contract.
It then dispatches the closed `BackendWork` variant. Errors from the actual
CUDA submission helpers poison the resource set; the next operation returns
`Error::BackendPoisoned`. Errors from the pre-dispatch plan, pending, or
contract checks are returned without changing `poisoned`.

The operation table below describes the exact driver call and pending form.

| `BackendWork` | Checks and native operation | Pending form and completion action |
| --- | --- | --- |
| `InitAdmission` | Destination device must equal the planned device. The destination arena, finalized image/value/bytes, image slice length, and pinned staging capacity are checked. The image is copied into pinned staging. A completion event is checked out with `take_event`, then `Stream::copy_h2d` enqueues `cuMemcpyHtoDAsync_v2` and records the event. | `PendingSubmission::Event`; `PendingAction::None`. |
| `Calculation` | Work device must equal the plan. The logical artifact and completion-slot `ParameterBlock` must exist. `fill_invocation` validates all ABI arguments and arena ranges. Grid size is `ceil(elements / workgroup_lanes)` with checked arithmetic; launch dimensions are nonzero `Dim3` values. | `PendingSubmission::Stream`; `PendingAction::None`. `ParameterBlock::enqueue` uses `cuLaunchKernel` without recording an event. |
| `InternalTransfer` | `device_endpoints` requires two resolved device endpoints. Source, destination, and planned device must be the same device and context. Both ranges are checked. `Stream::enqueue_d2d` enqueues `cuMemcpyDtoDAsync_v2` without an event. | `PendingSubmission::Stream`; `PendingAction::None`. |
| `Metric` | The value must be exactly four bytes and belong to the planned device. The resolved source range and pre-realized four-byte pinned metric buffer are checked. `Stream::enqueue_d2h` enqueues `cuMemcpyDtoHAsync_v2` without an event. | `PendingSubmission::Stream`; `PendingAction::Metric { dtype }`. |
| `ExitTransfer` | Source must be a device endpoint, destination must be external, and the source device must equal the planned device. The source range, staging capacity, and preallocated egress vector are checked. An event is checked out, then `Stream::copy_d2h` enqueues `cuMemcpyDtoHAsync_v2` and records it. Device-to-device exit is rejected by this CUDA bridge. | `PendingSubmission::Event`; `PendingAction::Egress { bytes }`. |

`route` and `lane_claims` are checked for exact equality with the finalized
transfer contract. The direct CUDA copy does not reinterpret those values:
the planner and local partitioner have already selected the one-hop route and
the CUDA operation is limited to a same-context copy. A cross-device or
cross-backend route is owned by `StagedCrossBackend`, described below.

### Kernel argument realization

`fill_invocation` resets the `ParameterBlock` keepalive list and walks the
immutable `KernelAbi` in order. A `Buffer` consumes the next input/output
location, checks its arena range, computes `device_ptr + arena_offset`, and
retains the arena buffer as a keepalive. A `FaultFlag` requires exactly one
resolved location, which must have been validated as the device int32 flag by
the execution plan. `RunId`, `LoopIteration`, and `ElementCount` are copied as
64-bit by-value arguments from the work item and ABI. At the end, every operand
and the optional fault flag must have been consumed exactly once.

`ParameterBlock` stores boxed 64-bit values and a parallel pointer array. Its
keepalive pointers are reconstructed only for the CUDA launch call. The
`recipe_cuda::Stream::enqueue_launch` safety contract requires the cubin ABI,
parameter pointer types, launch geometry, function/module, stream, and every
referenced device allocation to remain valid until stream completion. Arena
ownership and the pending token provide those lifetimes.

## Polling and completion actions

`CudaResources::poll` first checks `ensure_healthy` and obtains the pending
device. `validate_active` distinguishes native forms:

- a stream token requires its queue slot to still exist;
- an event token requires its completion slot to be `Active` and owned by the
  same task;
- `Ready` and `Terminal` are rejected as not active.

An event token calls `Pending::poll`, which maps `cuEventQuery` to
`CompletionStatus`. A stream token calls `Stream::poll_idle`, which maps
`cuStreamQuery`. A driver error from either native poll poisons the resource
set. `Pending` returns `BackendPoll::Pending`; `Complete` enters
`finish_pending`.

`finish_pending` replaces the token's native state with `Terminal`. For an
event token it calls `Pending::wait(Duration::ZERO)`:

- `WaitOutcome::TimedOut` puts the event token back into `Active` and returns
  `BackendPoll::Pending`;
- `WaitOutcome::Complete` extracts the event and returns it to its completion
  slot as `Available`, after asserting that the slot was owned by this task.

Stream tokens have no event to return. The stream itself remains available for
queue-ordered submissions.

The post-completion action is then applied:

- `None` returns `Complete { metric: None }`;
- `Metric` reads exactly four bytes from the task's pinned host buffer and
  decodes little-endian `F32` or `I32` into `MetricValue`;
- `Egress` checks the preallocated egress vector and staging sizes, copies the
  completed staging prefix into the vector, and returns no metric.

Only after this action succeeds is `pending.terminal` set to `true`. A
protocol or size error while finishing is returned directly; it is not routed
through the native-poll poison branch.

The `Backend` implementation appends exactly one physical accounting record
for every adapter operation. `MAX_NON_POLL_PHYSICAL_CALLS` is `1`. Polling
records one `PhysicalCall::Poll` with `Pending`, `Complete`, or `Failed`, and
never reports another physical operation in the same poll call. The local
backend and the executor preserve this status when wrapping CUDA errors.

## Exit collection

`collect_exit` is called only after a terminal exit token. It checks:

- the backend is healthy;
- the source is a resolved device endpoint and the destination is external;
- task, class, device, queue, completion, and terminal state match the
  completed pending token; and
- caller storage length equals the finalized byte count.

The per-task preallocated egress vector must exist and match that length. The
vector is copied into caller-owned storage. No CUDA call occurs during
collection. `CudaBackend::collect_exit` consumes the unused arena view and
records `PhysicalCall::CollectExit` before delegating. A bridge pending token
cannot have an external endpoint and is rejected by `LocalBackend`.

`CudaResources::take_egress(task)` is a separate crate-visible extraction
helper that removes and returns a task's stored egress vector from the first
device map containing it. The normal executor path uses `collect_exit`, which
retains the vector in the resource map until the caller-owned copy succeeds.

## Cross-backend transfers

`LocalBackend` classifies a device transfer by its endpoint owners:

| Endpoints | Owner |
| --- | --- |
| Same CUDA device to itself | `Cuda` |
| Different CUDA devices | `Bridge` |
| CUDA to HSA/Host, or HSA/Host to CUDA | `Bridge` |
| External to CUDA or CUDA to external | `Cuda` (admission or egress) |

This is why `CudaResources::submit_internal_transfer` rejects a different
device even though the planner may describe a multi-device route. Planner
expansion gives the bridge one-hop tasks; the direct CUDA adapter performs only
same-context D2D.

`StagedCrossBackend` (`native-executor/src/bridge.rs`) realizes a CUDA leg
before warm execution with one pinned staging buffer, one nonblocking stream,
and one completion event. For a CUDA source it submits `copy_d2h` and polls the
CUDA `Pending`; after the host staging worker completes, a CUDA destination
submits `copy_h2d` and polls its event. The bridge owns these event tokens and
recycles them with `Pending::recycle_event` after a terminal warm pass. Bridge
contract validation requires device endpoints, one finalized link, exact
values/bytes/route/lane claims/submission slots, and an owner class matching
the pre-realized leg. Bridge destroy closes workers, staging, streams, events,
and other legs separately from `CudaResources`.

## Loop and lifecycle callers

The executor's `Backend` state machine invokes the CUDA implementation through
these methods:

| Executor phase | CUDA entry point | Effect |
| --- | --- | --- |
| Bind | `CudaBackend::bind_resources` or `bind_partition` | Validates the immutable plan and consumes the backend state once. |
| Prepare | `CudaBackend::prepare_pending` | Records and obtains one ready token per task before `init`. |
| Arena allocation | `CudaBackend::allocate_arena` | Allocates one exact `DeviceBuffer` for each finalized arena layout. |
| Init, loop, exit submission | `submit` or `submit_loop_iteration` | Records one physical call, optionally rearms a loop token, and invokes `CudaResources::submit`. |
| Poll | `poll` | Drives event or queue completion and records exactly one poll status. |
| Exit result | `collect_exit` | Copies the precompleted egress vector into caller storage. |
| Arena release | `release_arena` | Requires a healthy backend and exact device ownership, then frees the buffer. |
| Teardown | `destroy_resources` | Requires a healthy resource set and runs `destroy_devices`. |

`LocalBackend` forwards `LocalPending::Cuda` directly to these methods. It
also checks the task owner before each dispatch and projects the local arena
map into the CUDA-only `CudaArenaLookup`. `LocalBackend::supports_loop_repetition`
comes from the bridge, while its `supports_same_queue_pipelining` returns true
only for tasks owned by CUDA. A cross-backend pending token is never routed to
`CudaResources`.

At the root-library boundary, `src/native_prepare.rs` reopens exact measured
devices and creates `CudaBinding` values. `src/inference.rs` and
`src/training.rs` construct `StagedCrossBackend`, `LocalCandidateFactory`,
`NativeExecutorDriver`, and `NativeCandidateRealizer`. The preparation driver
materializes and validates artifacts, invokes the CUDA candidate realization,
runs each maximum-concurrency warm pass, observes capacity, and hands the same
warmed session to the finalized local backend. The running declaration never
stores a CUDA context or mutable loader.

### Executor phase ordering

`recipe_executor::PreparedRun::prepare` rejects a loop count greater than one
unless the backend reports loop repetition support. It binds the CUDA backend,
then calls `prepare_pending` once for every init, loop, and exit task. The three
phase vectors and all CUDA pending tokens therefore exist before initialization
starts. `PreparedTask` construction also rejects invalid phase combinations
before a `BackendWork` value can reach this module: calculations and metrics
are loop-only, init transfers are admission or device-to-device movement, and
loop transfers must be internal.

`PreparedRun::initialize` validates one external `DeviceImage` per finalized
device, allocates every exact arena layout through `CudaBackend::allocate_arena`,
and runs the init phase. The init `BackendWork` borrows its validated image
slice; CUDA copies it into the device arena through pinned staging and does not
retain the caller's image after submission. Fault-flag bytes in the image are
zeroed by executor image validation before the first admission.

`InitializedRun::start_loop` begins iteration zero. Each scheduler pass calls
`submit_loop_iteration` for runnable CUDA tasks, then calls `poll` for every
pending slot. A later task may be considered runnable while an earlier task is
pending only when both tasks are on a queue for which the backend reports
same-queue pipelining. All other dependency and schedule-window constraints
remain executor decisions. A loop iteration is complete only after every
active slot is terminal; the next iteration resets the executor completion
ledger while the CUDA token itself is rearmed from its own terminal state.

When loop work is complete, `ExitedLoop::exit` runs the exit phase. For each
external exit, executor `complete_slot` allocates caller-owned result storage,
calls `collect_exit` on the already-terminal CUDA pending token, and records an
`ExitImage`. Only after every exit task is complete does executor teardown
release arenas and call `destroy_resources`. A failed scheduler, watchdog,
device-fault readback, backend operation, or teardown still attempts ordered
arena release and resource destruction and preserves the first cleanup error
separately.

## Teardown and failure behavior

### Explicit teardown order

`CudaResources::destroy` first calls `ensure_healthy`; a poisoned backend
returns `Error::BackendPoisoned` without attempting the explicit teardown
routine. A healthy resource set is passed to `destroy_devices`, which handles
each device in this order:

1. query every stream with `poll_idle`; a non-complete result is
   `Error::CudaContract("CUDA stream remained active during teardown")`;
2. destroy every `Available` completion event. An `Active { task }` event is
   `Error::ResourceContention` because a pending token still owns it;
3. destroy streams;
4. drop logical `LoadedArtifact` function handles;
5. unload boxed modules;
6. free metric pinned buffers and the shared staging buffer; and
7. free optional scratch.

The order is deliberate: function handles are dropped before their modules,
and all asynchronous work is required to be terminal before either streams or
buffers are released. `CudaPreparedResources::destroy` uses the same device
teardown routine for a candidate that never reaches finalized handoff.

`release_arena` records its physical call, refuses a poisoned resource set,
checks the arena's device identity against the requested device, and consumes
the arena. The local wrapper also checks the arena's owner class.

### Failure matrix

| Boundary | Observable failures |
| --- | --- |
| Binding and plan shape | `DuplicateDevice`, `MissingDevice`, `UnexpectedDevice`, `SubmissionQueueLimitExceeded`, `BackendState`, `Protocol`. |
| Reservation | Missing ledger entry or any mechanism other than `EnforcedQuota` produces `MissingDevice` or `ArenaMismatch`. |
| Artifact realization | `MissingArtifact`, `DuplicateArtifact`, `UnexpectedArtifact`, `ArtifactMismatch`, `Kernel` from cubin inspection, `IntegerOverflow`, or wrapped `recipe_cuda::CudaError`. |
| Resource lookup | `MissingQueue`, `MissingCompletion`, `MissingDevice`, `ArenaMismatch`, `ValueMismatch`, or `ResourceContention` for a too-small invocation block. |
| Contract validation | `Protocol` for a task, phase, class, submission slots, init image, endpoint class, route, lane claims, ABI operands, or pending state that differs from the finalized contract. |
| Completion ownership | `CompletionBusy` when an event slot is checked out by another task; `Protocol` when a pending token is not the registered owner. |
| Unsupported native route | `UnsupportedTransfer` for external internal-transfer endpoints, cross-device direct D2D, or non-external exit destinations. Cross-backend one-hop routes must use `StagedCrossBackend`. |
| Live native operation | A `recipe_cuda::CudaError` from submit or native poll poisons the resource set. Later operations return `BackendPoisoned`. |
| Completion action | `Protocol` for missing or wrongly sized metric/egress storage, or for an invalid four-byte metric buffer. These finish-time errors are returned directly. |
| Teardown | `CudaContract` for an active stream, `ResourceContention` for an active completion event, or wrapped CUDA free/unload/destroy errors. |

`recipe_cuda::CudaError` retains the Driver operation and numeric status for
live-loop failures. It also covers library loading, missing symbols, invalid
inputs, closed contexts, context-stack mismatches, and resource-context
mismatches encountered during discovery or realization. Post-realization
status checks intentionally keep the live path allocation-free and report the
numeric Driver status without formatting rich error text.

There are two distinct partial-operation cases worth preserving when diagnosing
a failure:

1. `take_event` marks an event slot `Active` before the asynchronous copy is
   enqueued. If the subsequent Driver call fails, `submit` poisons the CUDA
   resource set and the slot still records the failed task owner. The event
   handle itself is dropped by normal Rust unwinding, while explicit teardown
   is refused by the poisoned-state guard.
2. `finish_pending` moves the native state to `Terminal` before reading a
   metric buffer or copying egress. If that post-completion action reports a
   protocol or size error, the backend is not automatically poisoned, but the
   token is no longer a valid active token and its caller must fail the run.

These are deliberate ownership boundaries, not retry points. CUDA has no
cancellation or safe replacement operation for a partially submitted token.

## `recipe-cuda` runtime boundary used by this adapter

The native executor delegates raw Driver calls to `cuda/src/runtime.rs`; the
following wrapper behavior is part of the CUDA adapter's contract:

| Wrapper | Driver call behavior | Why `cuda.rs` depends on it |
| --- | --- | --- |
| `with_current` | Pushes the borrowed context with `cuCtxPushCurrent_v2`, runs one operation, and pops it. An operation error is retained if the pop also fails; a successful operation reports a pop failure. | Every allocation, module, stream, event, copy, launch, poll, and explicit free runs on the binding's exact context. |
| `same_context` | Compares `Context` pointer identity, not just device UUID. | Prevents a stream, event, arena, staging buffer, or function from being combined across separately created contexts. |
| `Module::load_cubin` | Requires ELF magic, calls `cuModuleLoadData`, and rejects a null success handle. | `realize_device` has already inspected the cubin's CUDA ABI and target before loading. |
| `Module::function` | Rejects empty or NUL-containing names, calls `cuModuleGetFunction`, and rejects a null success handle. | `LoadedArtifact` retains one ABI entry lookup per logical artifact. |
| `DeviceBuffer` | Rejects zero-byte allocation, calls `cuMemAlloc_v2`, checks every offset range, and frees with `cuMemFree_v2`. | Arenas and optional scratch are nonzero, bounded device allocations. |
| `PinnedHostBuffer` | Rejects zero-byte allocation, calls `cuMemHostAlloc`, zeroes the returned bytes, and frees with `cuMemFreeHost`. | Shared admission/egress staging and four-byte metric readback are pinned host memory. |
| `Stream::copy_h2d`, `copy_d2h`, `copy_d2d` | Checks contexts and ranges, enqueues the asynchronous copy, records a supplied event, and returns an event-backed `Pending`. | Init and external-exit transfers use an event because their completion slot must be reusable after the copy. |
| `Stream::enqueue_d2h`, `enqueue_d2d`, `enqueue_launch` | Checks contexts and ranges, enqueues without recording an event, and returns only a Driver result. | Calculation, metric, and internal D2D work use queue-idle completion and permit same-queue pipelining. |
| `Stream::launch` | Enqueues a kernel, records an event, and returns a `Pending`; `enqueue_launch` omits the event. | The probe benchmark uses the event form, while the production CUDA backend uses `ParameterBlock::enqueue` and queue polling. |
| `Pending` | `poll` maps `cuEventQuery`; `wait(Duration::ZERO)` either extracts the event or returns the same token; `recycle_event` extracts only after a terminal poll. | `finish_pending` returns event slots to `Available`; bridge warm passes recycle their own events. |

`Pending<'operation, 'context>` stores the completion event and a phantom
operation borrow. `erase_operation_lifetime` widens only the operation
lifetime to `'static`; the context lifetime remains tied to the binding. The
caller must retain all stream, module, argument, arena, and staging owners
until the token reaches terminal completion. The no-event enqueue methods carry
the same requirement through their caller's stream-idle poll.

The wrapper maps only `CUDA_SUCCESS` to complete and
`CUDA_ERROR_NOT_READY` to pending. Any other Driver status is a
`CudaError::DriverCall`; post-realization submit and poll omit optional name and
description allocations, preserving the numeric status for error reporting.

## Native execution evidence

`CudaResources::execution_evidence` reports one
`NativeDeviceExecutionEvidence` per retained device:

- `image_loads` is the number of distinct cubin modules in `modules`;
- `entry_lookups` is the number of logical loaded artifacts in `artifacts`;
- `queues` and `completion_objects` are the realized stream and event counts;
- `persistent_allocations` is the number of metric buffers, plus one when
  scratch exists, plus one for the shared staging allocation.

`LocalBackend::destroy_resources` captures this evidence before destruction and
publishes completed `NativeExecutionEvidence` only when bridge, HSA, CUDA, and
host teardown all succeed. A successful report has zero live resources after
teardown and zero loop-time realization calls. These counters describe real
retained resources; they are not substitutes for the executor's observed
correctness or performance measurements.

## Source map

The implementation regions covered here are:

| Concern | Source region |
| --- | --- |
| Binding, arena, backend state | `native-executor/src/cuda.rs`: `CudaBinding`, `CudaArena`, `CudaBackendState`, `CudaBackend` |
| Finalized and candidate realization | `native-executor/src/cuda.rs`: `CudaResources::realize`, `CudaPreparedResources::{realize,bind,bind_candidate,validate_handoff}` |
| Device resources and cubin loading | `native-executor/src/cuda.rs`: `realize_device`, `invocation_sizes` |
| Contracts and transfer classification | `native-executor/src/cuda.rs`: `task_contracts`, `transfer_work_class`, `validate_work_contract` |
| Work submission and completion | `native-executor/src/cuda.rs`: `submit_*`, `poll`, `finish_pending`, `finish_action` |
| Pending state and unsafe lifetime boundary | `native-executor/src/cuda.rs`: `CudaPending`, `erase_operation_lifetime`, `Drop for CudaPending` |
| ABI argument storage | `native-executor/src/cuda_ffi.rs`: `ParameterBlock` |
| CUDA Driver wrappers | `cuda/src/runtime.rs`: `Stream`, `Pending`, `Module`, `DeviceBuffer`, `PinnedHostBuffer`, `Event` |
| Backend-neutral lifecycle | `executor/src/backend.rs`, `executor/src/executor.rs` |
| Local owner partitioning and warm trace | `native-executor/src/local.rs` |
| Cross-backend CUDA staging | `native-executor/src/bridge.rs`: `StagedCrossBackend`, `submit_source`, `submit_destination`, `poll_leg` |
| Exact binding creation | `native-probe/src/bindings.rs` |
| Root training and inference callers | `src/training.rs`, `src/inference.rs`, `src/native_prepare.rs` |
