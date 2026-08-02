---
module: native-executor/src/bridge.rs
public_types: [StagedCrossBackend, StagedBridgeResources, StagedBridgePending, StagedBridgeError]
implements: [CrossBackendTransfer, CandidateCrossBackendTransfer]
scope: pre-realized one-hop device-to-device transfers
---

# Staged cross-backend bridge

## Purpose

`StagedCrossBackend` is the production composite backend for one-hop transfers
whose source and destination are owned by different local execution owners.
The owners may be Host, CUDA, HSA, or two distinct CUDA devices. The bridge
turns one immutable `TransferWork` into a staged pipeline:

1. If the source is native, issue a native device-to-host copy into a
   preallocated staging allocation.
2. Run one host-stage job that reads, writes, or copies the staging bytes.
3. If the destination is native, issue a native host-to-device copy from its
   preallocated staging allocation.

The bridge owns the staging allocations, native completion tokens, and one
bounded host worker for each selected transfer. It does not discover devices,
choose routes, allocate finalized arenas, compile code, or handle external
endpoints. Planner route expansion and `LocalBackend` task ownership must have
already selected this implementation before the bridge is called.

`submit` and `poll` are nonblocking and allocation-free at the bridge boundary.
All streams, staging memory, completion objects, worker threads, and worker
channels are created during pre-loop realization, either by
`realize_candidate` or by direct finalized `bind`. `prepare_pending` consumes
an already prepared token; it does not create a new driver object.

## Source anchors

The implementation is deliberately kept in one bridge module. Current source
regions are:

| Region | Symbols and intent |
| --- | --- |
| `bridge.rs:44-166` | `StagedBridgeError`, formatting, error sources, and lower-level conversions |
| `bridge.rs:172-334` | Binding validation, selected-task realization, and per-leg allocation |
| `bridge.rs:336-554` | `EndpointContract`, `TransferContract`, finalized/work validation |
| `bridge.rs:556-694` | Leg resources, prepared tokens, task resources, active legs |
| `bridge.rs:696-817` | Pending states, destination targets, middle jobs, pending token |
| `bridge.rs:820-867` | Host `Read`, `Write`, and `Copy` job execution |
| `bridge.rs:869-1064` | Bounded host worker, status publication, polling, reset, shutdown |
| `bridge.rs:1066-1363` | Runtime and candidate bridge trait implementations |
| `bridge.rs:1365-1417` | Native token recycling and loop rearming |
| `bridge.rs:1419-1506` | Route-to-host-job construction and destination capture |
| `bridge.rs:1508-1635` | CUDA/HSA native submission and completion polling |
| `bridge.rs:1637-1765` | Arena/range checks, integer conversion, cleanup, lifetime erasure |
| `local.rs:125-212` | `CrossBackendTransfer` and `CandidateCrossBackendTransfer` contracts |
| `local.rs:502-612` | `PreparedBridge` one-shot candidate handoff |
| `local.rs:1611-2111` | Composite `LocalBackend` bind, prepare, submit, poll, exit, and teardown |
| `local.rs:2291-2596` | Candidate/final owner classification and one-hop route checks |
| `local.rs:2844-3202` | Real warm trace, bridge pending recycle, and warm arena release |
| `cuda/src/runtime.rs:238-327,405-480,699-821` | Pinned buffers, asynchronous copies, events, and CUDA pending reuse |
| `hsa/src/execution.rs:1291-1575` | Prepared HSA signal/keepalive state, reset, and async copy |
| `host/src/arena.rs:88-182` | Host arena backing, exact bridge range I/O, and backing ownership |
| `executor/src/backend.rs:318-450` | Closed backend ABI and fixed physical-call contract |
| `executor/src/executor.rs:1489-1541,2085-2494` | Teardown, phase token preparation, scheduling, submit, and poll |

## Boundary and ownership model

The bridge is used through the two local traits in `native-executor/src/local.rs`:

| Trait operation | Bridge responsibility | Allowed resource change |
| --- | --- | --- |
| `realize_candidate` | Build resources from a `PlannedCandidate` draft | Allocate and create all bridge objects |
| `validate_handoff` | Prove the realized candidate exactly matches the finalized bundle | Inspect only; do not replace resources |
| `bind` | Realize from a finalized bundle, or move an already validated prepared resource | May realize the pre-loop bridge resource; prepared handoff moves it without replacement |
| `prepare_pending` | Move one pair of prepared source and destination tokens into a pending token | Consume the per-task token pair |
| `submit` | Validate immutable work and enqueue the first stage | No allocation or waiting |
| `poll` | Advance one native or host stage | No allocation or waiting |
| `rearm_loop_pending` | Reset a terminal loop token for the next iteration | Reuse the same worker, staging, and native completion objects |
| `recycle_candidate_pending` | Return one terminal warm token to candidate resources | Reuse the same prepared objects |
| `destroy` | Close workers and release all bridge resources | Deterministic teardown, first error retained |

`LocalBackend` classifies a device-to-device transfer as `TaskOwner::Bridge`
when the endpoint classes differ, or when two distinct CUDA devices are used.
Host-to-host, HSA-to-HSA, and same-device CUDA transfers remain with their
native owner. `LocalArenaSet` is a read-only view of the executor-owned arena
map. The bridge borrows that view during submission and never owns or mutates
the finalized arena map. The local backend does not opt bridge tasks into
same-queue pipelining, so a bridge pending token reaches terminal completion
before another submission can reuse its queue or completion ownership.

## Resource and contract types

### `StagedCrossBackend`

The bridge contains cloned `CudaBinding` and `HsaBinding` vectors. A binding
provides the exact CUDA context or HSA session and the allocator used to make a
bridge staging allocation. `new` only stores these bindings. `validate_bindings`
rejects duplicate binding devices, rejects a binding whose class disagrees with
the finalized device map, and rejects an HSA binding that duplicates either a
CUDA or an earlier HSA binding.

### `TransferContract`

`TransferContract` is the immutable copy of the transfer facts that every later
stage checks:

| Field | Meaning |
| --- | --- |
| `task` | Task identity used to index resources and pending tokens |
| `phase` | Finalized `RunPhase` |
| `class` | `InternalTransfer` for `Init` or `Loop`, `ExitTransfer` for `Exit` |
| `source`, `destination` | Device, value, and `LocalDeviceClass` for each endpoint |
| `bytes` | Exact `ByteCount` requested by the task |
| `route` | Cloned planner-expanded link list, which must contain exactly one link |
| `lane_claims` | Cloned finalized lane claims |
| `submission` | Exact queue and completion slots |

`from_task` accepts only `TaskKind::Transfer` with two
`TransferEndpoint::Device` endpoints. It requires one route link and requires
the endpoint owners to cross local ownership. Consequently, a host-to-host,
HSA-to-HSA, same-device CUDA, or otherwise same-owner transfer is a contract
error. A same-class CUDA transfer is accepted only when the device IDs differ.

`validate_finalized` compares every contract field with the matching finalized
task, checks the finalized device class map, and then checks
`FinalizedBundle::transfer_endpoints`. Both resolved endpoints must still be
the exact device and value recorded before finalization. Any candidate/final
mismatch is rejected rather than rebuilding resources.

`validate_work` performs the corresponding comparison for a submitted
`TransferWork`. It rejects external resolved endpoints, a changed work class,
task, value, device, byte count, route, lane claim, or submission slot.
Route and lane claims are carried as immutable proof from the planner and
scheduler. The bridge validates them but does not select a different route or
claim another lane during staging.

### Per-task resources

`StagedTaskResources` contains the contract, a source and destination
`LegResources`, one `PreparedTokenState`, and one `HostStageWorker`.
`StagedBridgeResources` stores these records in a `BTreeMap<TaskId, ...>`.
`realize_tasks` filters the supplied task list by the selected set, realizes
each record, and requires the resulting map keys to equal the selected set in
order. Duplicate identities, missing selected tasks, and unexpected resources
are errors.

`LegResources` is one of:

| Leg | Pre-realized objects | Host-visible staging address |
| --- | --- | --- |
| `Host` | None | Not applicable; a host arena is read or written by the worker |
| `Cuda` | A nonblocking CUDA `Stream` and `PinnedHostBuffer` of `bytes` | `PinnedHostBuffer::as_mut_slice().as_mut_ptr()` |
| `Hsa` | Bound `HsaSession` and fine-grained host `Allocation` of `bytes` | `Allocation::as_ptr()` |

For HSA, `HsaBinding::allocate_host_fine` allocates through the selected CPU
agent and grants access to the exact bound GPU. Each HSA leg also gets a
`Session::prepare_pending(2, 0)` token. The allocation capacity of two matches
the two allocation keepalives needed by one asynchronous copy.

For CUDA, realization creates one pinned allocation, one nonblocking stream,
and one completion `Event` per leg. Zero-byte or otherwise invalid sizes are
left to the native allocation wrappers and become `StagedBridgeError::Cuda` or
`StagedBridgeError::Hsa`.

## Candidate and finalized lifecycle

### Pre-final realization

`LocalCandidateFactory::realize_candidate` clones the bridge and calls
`realize_candidate` with the candidate draft, bridge task set, and device-class
map. The bridge validates bindings, creates every selected task's two legs and
tokens, and starts one named `recipe-bridge-{task}` host worker per task. No
finalized arena address is captured at this stage.

### Warm execution and recycling

The local candidate session activates the realized host, CUDA, HSA, and bridge
resources and runs the real maximum-concurrency warm trace. For a bridge task,
the trace calls `prepare_pending`, `submit`, repeatedly `poll`, and after a
terminal result calls `recycle_candidate_pending`. Recycling requires
`PendingState::Complete`, polls or resets native tokens to prove completion,
resets the host worker, and puts a new `PreparedTokens` pair wrapping those
reused native tokens back in `PreparedTokenState::Available`. The warm trace therefore exercises the exact
same staged route without allocating replacement resources. Warm arenas are
released before capacity observation and final handoff.

### Final handoff

`validate_handoff` repeats binding validation, requires the resource task set to
equal the finalized bridge partition, and validates every `TransferContract`
against the finalized bundle and device map. `LocalPreparedSession::into_backend`
then wraps the resources in `PreparedBridge` with `handoff_validated = true`.
`PreparedBridge::bind` checks the unchanged task and device partitions and
consumes the prepared bridge resource exactly once. A second bind, a changed
partition, or an unvalidated handoff fails closed.

If another child partition fails while candidate resources are being activated,
the local session destroys the bridge resource along with the native and host
children. Candidate destruction is best effort in bridge, HSA, CUDA, then Host
order. A failed final handoff first releases any warm arenas, then destroys the
bridge and child resources while returning the first teardown error.

### Runtime execution

At finalized run bind, `LocalBackend` records one `BindResources` physical call.
Before `init`, the executor's phase realization calls `prepare_pending` once
for each bridge task, records `PreparePending`, and wraps the result in
`LocalPending::Bridge`. The first submission must use the exact
`PendingRequest` contract and `TransferWork` contract. The executor then polls
the one bridge pending token until `BackendPoll::Complete`.

### Loop repetition

`supports_loop_repetition` returns `true`. `rearm_loop_pending` is a terminal
token reset, not a new preparation path:

* `Ready` is already reusable and returns success.
* `Source`, `Middle`, or `Destination` is invalid because the operation is not
  terminal.
* `Complete` resets the host worker, recycles an active CUDA event or HSA
  prepared signal, clears the middle job, resets the destination target, and
  returns to `Ready`.

The executor then submits the next loop iteration through the same bridge
resource. No loop-time allocation, worker creation, stream creation, or signal
acquisition is allowed.

## Staged route matrix

`middle_job` is the single route constructor. It matches both staged leg kinds
and both executor arena classes before producing one host job:

| Source endpoint | Destination endpoint | Host-stage job | Native stages |
| --- | --- | --- | --- |
| Host | CUDA or HSA | `Read`: `HostArena::bridge_read_exact` into destination staging | Destination H2D or HSA copy |
| CUDA or HSA | Host | `Write`: source staging into `HostArena::bridge_write_exact` | Source D2H or HSA copy |
| CUDA or HSA | CUDA or HSA | `Copy`: nonoverlapping pointer copy between the two staging allocations | Source D2H or HSA copy, then destination H2D or HSA copy |

The source and destination ranges use the resolved arena offsets and the exact
contract byte count. Host `Read` and `Write` clone the `HostArena`, retaining
its `Arc` backing while the worker runs. The `Copy` job uses distinct
per-endpoint staging allocations. A class/resource disagreement is a contract
error, not a fallback route.

The bridge never handles an external endpoint. `endpoint_offset` rejects
`ResolvedTransferEndpoint::External`; `LocalBackend::collect_exit` likewise
rejects a bridge pending token because bridge tasks are device-to-device.
Longer topology paths are not hidden here: the planner lowers them to
dependency-chained one-link tasks with resident intermediate values before
this bridge sees them.

## Pending state machine

`StagedBridgePending` has a task ID, `PendingState`, source and destination
`ActiveLeg`s, a `MiddleJobState`, and a `DestinationTarget`.

```text
prepare_pending
    -> Ready

Ready --submit, native source--> Source
Ready --submit, host source--> Middle
Source --native poll Pending--> Source
Source --native poll Complete / submit worker--> Middle
Middle --worker poll Pending--> Middle
Middle --worker poll Complete, host destination--> Complete
Middle --worker poll Complete, native destination--> Destination
Destination --native poll Pending--> Destination
Destination --native poll Complete--> Complete
```

`poll` returns `BackendPoll::Pending` for every nonterminal transition and
`BackendPoll::Complete { metric: None }` only on the transition to
`Complete`. Polling `Ready` or polling `Complete` again is a state error.

`MiddleJobState` prevents duplicate construction or submission of a host job:
`Uninitialized -> Ready(job) -> Submitted`. Initialization more than once, or
submission without a ready job, is rejected. A failed worker submission leaves
the token on the terminal failure path rather than attempting a second job.

`ActiveLeg` states are `Host`, `CudaReady(Event)`, `CudaActive(Pending)`,
`Hsa(PreparedPending)`, and the internal `Transition` sentinel. CUDA event
consumption and rearming use `Transition` to make a temporary move explicit.
An invalid token kind, a second event consumption, or an observed transition
produces a state error. The HSA rearm path restores its active token if reset
reports an error; native failure is returned to the executor.

`DestinationTarget` stores `Host`, or a raw pointer to the exact executor-owned
CUDA `DeviceBuffer` or HSA `Allocation` plus its offset. The pointer is created
only after `validate_arena`, and its owning arena map is retained by the
executor until the pending token is terminal.

## Native and host operations

### CUDA legs

`submit_source` consumes a `CudaReady` event, calls the stream's unsafe
`copy_d2h` with the source arena and leg staging buffer, and stores the returned
`CudaPending`. `submit_destination` consumes the destination event and calls
`copy_h2d` from destination staging into the captured device buffer. Both calls
record the event after the asynchronous copy. `poll_leg` calls
`CudaPending::poll` and maps `Pending` or `Complete` to the bridge poll result.

The CUDA wrapper requires the event, stream, source or destination buffer, and
pinned buffer to share the context and checks every range. The bridge keeps the
`CudaPending` token until terminal polling and retains all referenced arenas
and staging objects. `erase_cuda_pending_lifetime` is the single reviewed
lifetime widening: `CudaPending` carries only a phantom operation borrow, and
the bridge's resource ownership provides the actual lifetime guarantee. An
active `CudaPending` dropped before completion intentionally leaks the native
operation through the CUDA adapter rather than releasing a referenced object.

### HSA legs

`submit_source` calls `Session::copy_async_prepared` from a device arena into
the HSA leg's fine-grained staging allocation. `submit_destination` calls the
same prepared operation from staging into the captured destination allocation.
`poll_leg` calls `PreparedPending::poll` and maps HSA signal completion. The
prepared HSA token owns one signal and fixed keepalive vectors, so submission
can retain both allocations without acquiring a signal or growing a vector.
`PreparedPending::reset` is used only after terminal completion; an in-flight
token remains nonterminal and is retired safely by its drop behavior.

### Host worker

Each task's `HostStageWorker` owns a bounded `sync_channel(1)`, an atomic status,
an error slot, and a completion receiver. `new` starts a detached named thread
and initially sets `WORKER_IDLE`. `submit` performs an `IDLE -> PENDING`
compare-and-exchange and uses `try_send`, so the bridge never waits for the
worker. A full or disconnected channel restores `IDLE` and reports
`WorkerDisconnected`.

The worker executes exactly one `HostStageJob` at a time. Success publishes
`WORKER_COMPLETE` with release ordering. Host failure stores the
`recipe_host::Error` and publishes `WORKER_FAILED`; a poisoned error mutex
publishes failure without an available error, which later becomes
`WorkerDisconnected`. `poll` reads with acquire ordering and consumes a stored
failure exactly once. `reset` accepts only `WORKER_COMPLETE` and returns to
`WORKER_IDLE`.

`shutdown` closes the sender and yields until the worker sends its completion
message. A disconnected completion channel is `ThreadPanicked`; a missing
sender or other invalid status is reported as worker failure. `Drop` invokes
the same shutdown and discards its error, while explicit bridge destruction
retains the first shutdown error. A worker job's cloned `HostArena` and native
staging pointer remain valid until terminal worker status is observed.
Host job errors are surfaced by `poll`; `close` waits for thread termination and
does not re-read an already published job failure.

## Accounting and executor integration

The bridge does not emit `PhysicalCall` records itself. `LocalBackend` owns the
composite accounting boundary:

| Bridge action | Composite record |
| --- | --- |
| Final bind | `BindResources` |
| Prepare one task | `PreparePending { task }` |
| Submit internal bridge transfer | `SubmitInternalTransfer { task }` |
| Submit exit-class bridge transfer | `SubmitExitTransfer { task }` |
| Any poll result | `Poll { task, Pending\|Complete\|Failed }` |
| Destroy local resources | `DestroyResources` |

One bridge submission may perform a source native copy, a host copy, and a
destination native copy, but it remains one backend operation and one
submission record. This preserves the fixed executor journal contract and does
not expose staging implementation details as additional model work. The local
backend declares `MAX_NON_POLL_PHYSICAL_CALLS = 1`; a fixed-capacity physical
call overflow is reported by `LocalBackend` before the bridge result is
accepted. Each bridge poll appends exactly one `Poll` record with `Pending`,
`Complete`, or `Failed`; the executor compacts repeated pending polls into the
fixed per-task pending-poll counter while retaining terminal records in order.

## Validation and invariants

The following conditions are deliberate fail-closed checks rather than
fallbacks:

1. Every CUDA or HSA binding is unique and agrees with the finalized device
   class map.
2. Every selected bridge task appears exactly once, and no unselected task is
   realized.
3. A bridge task is a device-to-device transfer with exactly one route link and
   crosses local backend ownership.
4. The finalized task and resolved endpoint values are byte-for-byte equivalent
   to the pre-realized `TransferContract`.
5. The pending request, submitted work, endpoint values, offsets, bytes, route,
   lane claims, and submission slots match that contract.
6. The executor arena for each endpoint has the expected device and class.
7. All `ByteCount` and arena offsets fit the host `usize` address space, and
   host staging offsets fit `u64` before a host API call.
8. Exactly one prepared source/destination token pair exists per task. Preparing
   twice, recycling before completion, or polling outside the pending state
   machine is invalid.
9. A native operation uses the matching leg resource and token kind. A CUDA
   token must belong to the resource's context, and an HSA token must belong to
   the resource's session.
10. Arena and staging ownership outlives native completion and host worker
    execution. No resource is released while its pending operation can still
    reference it.
11. Loop rearming and candidate recycling operate only on terminal tokens and
    reuse pre-realized objects.
12. No bridge pending token can be collected as an external exit transfer.

## Errors and failure behavior

`StagedBridgeError` is `non_exhaustive` and preserves the failure layer:

| Variant | Failure condition |
| --- | --- |
| `DuplicateBinding` | A CUDA or HSA device is bound more than once |
| `MissingBinding` | A required device/class has no matching native binding or arena |
| `MissingTask` | A request, resource, or finalized bundle lacks the task identity |
| `Contract` | Immutable candidate, finalized, arena, endpoint, route, or work contract differs |
| `State` | Pending, active-leg, middle-job, token, or worker transition is invalid |
| `Host`, `Cuda`, `Hsa` | Underlying host, CUDA Driver, or ROCr/HSA operation failed |
| `ThreadSpawn` | The per-task worker could not be created |
| `ThreadPanicked` | The detached worker ended without sending completion |
| `WorkerDisconnected` | Worker channel, error state, completion state, or poisoned mutex cannot provide a valid result |
| `WorkerState` | Atomic worker status is not one of the known states for the requested operation |
| `IntegerConversion` | A `ByteCount`, arena offset, or host offset cannot be represented on this host |

Underlying host, CUDA, and HSA errors are available through `source()`. Bridge
state and contract errors intentionally have no lower-level source because
they identify an invalid caller or finalized plan. Cleanup attempts every task
in `BTreeMap` order. For each task it drops the remaining prepared token state, closes
the worker, destroys destination leg resources, then destroys source leg
resources. `first_error` and `retain_first` return the first error while still
attempting all later cleanup operations.

## Safety-critical facts

* `HostStageJob::Read` reconstructs a mutable destination slice from a stored
  address. The destination is a bridge-owned CUDA pinned buffer or HSA staging
  allocation, and the worker status is terminally published before that
  allocation can be destroyed.
* `HostStageJob::Write` reconstructs an immutable source slice only after the
  preceding native operation completed. The source staging allocation remains
  retained by the task resource.
* `HostStageJob::Copy` uses nonoverlapping pointers from two distinct endpoint
  staging allocations.
* CUDA destination raw pointers point into the executor arena map. The executor
  retains every arena until all phase pending tokens are terminal.
* HSA destination raw pointers have the same executor-owned arena guarantee;
  the HSA wrapper additionally checks mutual agent accessibility and ranges.
* The bridge uses no wait call in `submit` or `poll`. Waiting is limited to
  worker shutdown, after the sender has been closed, so destruction cannot
  leave a worker using released staging memory.
* `LocalBackend::destroy_resources` destroys the bridge before HSA, CUDA, and
  Host child resources. This keeps bridge staging, streams, events, and worker
  jobs alive until every cross-owner operation has released its child arena and
  native references. CUDA and HSA child teardown can therefore enforce their
  own terminal-queue and deferred-retirement checks.

## Implementation map

| Symbol | Role |
| --- | --- |
| `StagedBridgeError` | Public error taxonomy and lower-level source mapping |
| `StagedCrossBackend` | Binding lookup, candidate/final resource realization, trait implementation |
| `TransferContract` | Immutable task, route, endpoint, and submission validation |
| `LegResources`, `PreparedLegToken`, `PreparedTokenState` | Per-endpoint staging and reusable completion ownership |
| `StagedBridgePending`, `PendingState`, `ActiveLeg`, `MiddleJobState` | Runtime state machine for one staged transfer |
| `HostStageJob`, `HostStageWorker` | Bounded host copy execution and completion publication |
| `middle_job`, `submit_source`, `submit_destination`, `poll_leg` | Route construction and native/host stage transitions |
| `recycle_leg`, `rearm_leg` | Terminal token reuse for warm passes and loop iterations |
| `destroy_resources` | Ordered, best-effort resource teardown |
