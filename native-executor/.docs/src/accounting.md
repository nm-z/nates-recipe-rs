# Native executor accounting

## Contract boundary

`native-executor/src/accounting.rs` is the small adapter between a native
backend method and the backend-neutral `recipe_executor::PhysicalCallBatch`.
It does not measure memory, reserve a device, allocate a resource, or validate
a task. It performs only these two operations:

1. append one already-classified physical record to the caller-owned fixed
   batch, and
2. translate one closed `BackendWork` value, or one poll result, to its
   corresponding `PhysicalCall`.

The batch is consumed by `recipe_executor::RunJournal` after the backend method
returns. Logical lifecycle events are a separate journal stream. A physical
record is evidence of one adapter action, not a replacement for a logical task
event. In particular, one logical admission is allowed to have multiple
`AdmissionChunk` records, although the native adapters currently emit exactly
one chunk with index `0`.

Source anchors:

- `native-executor/src/accounting.rs:5-35`
- `executor/src/backend.rs:127-239`
- `executor/src/executor.rs:479-574`

## Units and field ownership

| value | type or unit | source of truth | accounting behavior |
| --- | --- | --- | --- |
| task identity | `TaskId` | finalized bundle | copied into submission and poll records |
| device identity | `DeviceId` | resolved value location or finalized arena layout | copied into admission, allocation, and release records |
| bytes | `ByteCount`, exact decimal `u64` bytes | finalized work or arena layout | copied without conversion |
| admission chunk index | `u32` | physical ABI | native accounting always writes `0` |
| metric mailbox slot | `MetricSlotId` | finalized metric work | copied into `SubmitMetric` |
| poll status | `PhysicalPollStatus` | backend result | one of `Pending`, `Complete`, or `Failed` |
| per-operation batch length | `usize` view over an internal `u8` | `PhysicalCallBatch` | bounded by `MAX_PHYSICAL_CALLS_PER_OPERATION = 16` |
| exact pending-poll total | `u128` | `RunJournal.pending_polls` | all pending polls are counted, only selected markers are retained in order |

`ByteCount` is the `recipe_core` unit from `core/src/units.rs:3-38`. It stores
an exact decimal byte count in a `u64`; checked addition and subtraction are
used by capacity accounting. Queue limits are a different unit: requested
queues are counted as `usize` and compared with a binding's discovered or
configured `u32` maximum.

## `accounting.rs` functions

### `record`

```text
record(batch, call) -> Ok(())
                         or Error::PhysicalAccountingOverflow
```

`record` calls `PhysicalCallBatch::try_push`. A successful call writes the
record at the current prefix and increments the batch length. If the prefix
already contains 16 records, `try_push` returns `PhysicalCallBatchOverflow`;
the helper discards that source error and returns the native
`Error::PhysicalAccountingOverflow` (`native-executor/src/error.rs:82,
196-200`). The failed append does not increase the batch length.

The helper is crate-private and is used directly by `CudaBackend` and
`HsaBackend`. `LocalBackend` has a same-shaped local helper at
`native-executor/src/local.rs:3642-3646` because its error type is
`LocalError<BridgeError>`; that helper maps the same batch overflow to
`LocalError::PhysicalAccountingOverflow` (`local.rs:341-377`). The local
composite reports at its boundary, so a host, CUDA, HSA, or bridge child does
not append a second record to the same batch.

`record` performs no driver operation and allocates no storage. Callers use
`?`, so a full batch normally prevents the operation that follows the record
from running. Polling is the one ordering exception described below.
For the by-value `destroy_resources` methods, an early accounting error returns
before the explicit child-destroy routine; normal Rust drop still runs when
the by-value resource argument leaves the method, but no successful
`DestroyResources` operation is reported.

### `submission_call`

`submission_call` is an exhaustive match over the closed `BackendWork` enum.
It trusts the immutable work contract and copies only the identity or byte
fields needed for a physical report. It does not inspect routes, queue slots,
arena bounds, ABI arguments, or reservation state.

| `BackendWork` variant | emitted `PhysicalCall` | copied fields |
| --- | --- | --- |
| `InitAdmission(work)` | `AdmissionChunk` | `task = work.task`, `device = work.destination.device`, `bytes = work.bytes`, `chunk_index = 0` |
| `Calculation(work)` | `SubmitCalculation` | `task = work.task` |
| `InternalTransfer(work)` | `SubmitInternalTransfer` | `task = work.task` |
| `Metric(work)` | `SubmitMetric` | `task = work.task`, `slot = work.slot` |
| `ExitTransfer(work)` | `SubmitExitTransfer` | `task = work.task` |

The admission device comes from the resolved destination location, not from a
separately supplied device argument. `MetricWork` contributes a slot but no
byte count because a metric is represented by a scheduled device readback,
not by an independently accounted payload transfer.

### `completion_poll_call`

`completion_poll_call(task, status)` is a `const fn` that constructs exactly
`PhysicalCall::Poll { task, status }`. It performs no status normalization and
does not validate that the task belongs to the finalized bundle. That protocol
validation belongs to `RunJournal::record_physical` and the executor's fixed
completion ledger.

## Physical-call ABI and operation callers

`PhysicalCallBatch` is defined in `executor/src/backend.rs:185-239`:

- storage is `[Option<PhysicalCall>; 16]` plus a `u8` prefix length;
- `new` creates an empty batch and `single` creates a one-record batch;
- `try_push` rejects exactly the 17th record;
- `iter` exposes only the initialized prefix and treats an empty prefix slot as
  unreachable;
- `try_from_array` rejects arrays longer than 16 before pushing records.

The backend ABI-wide bound is `MAX_PHYSICAL_CALLS_PER_OPERATION = 16`.
`Backend::MAX_NON_POLL_PHYSICAL_CALLS` must be in `1..=16`; the executor
rejects a backend declaration outside that range while deriving journal
capacity (`executor/src/executor.rs:258-265`). The three native backend
implementations all declare `1` (`cuda.rs:1198-1200`, `hsa.rs:1387-1389`,
`local.rs:1647-1655`). Their normal reports therefore contain at most one
record for every non-poll call, despite the ABI allowing 16.

The following table is the complete native call surface. Each row names the
record written before or after the real operation and the native owner.

| backend method | record | order relative to real operation | owner |
| --- | --- | --- | --- |
| `bind_resources` | `BindResources` | before state transition and resource binding | CUDA, HSA, Local |
| `prepare_pending` | `PreparePending { task }` | before pending-token preparation | CUDA, HSA, Local |
| `allocate_arena` | `AllocateArena { device, bytes }` | before arena allocation | CUDA, HSA, Local |
| `submit` | `submission_call(work)` | before native submit | CUDA, HSA, Local |
| `submit_loop_iteration` in CUDA/HSA | `submission_call(work)` | before loop-token rearm and submit | CUDA, HSA |
| `submit_loop_iteration` in Local | `submission_call(work)` through `submit` | after child loop-token rearm, immediately before child submit | Local |
| `poll` | `Poll { task, status }` | after child poll result is known | CUDA, HSA, Local |
| `collect_exit` | `CollectExit { task, bytes }` | before copying the completed external exit image | CUDA, HSA, Local |
| `release_arena` | `ReleaseArena { device }` | before health, ownership, and release checks | CUDA, HSA, Local |
| `destroy_resources` | `DestroyResources` | before child destruction | CUDA, HSA, Local |

CUDA and HSA call sites are at `cuda.rs:1204-1366` and `hsa.rs:1390-1573`.
The Local composite call sites are at `local.rs:1657-2069`. Local warm
candidate execution (`local.rs:2977-3160`) calls child resources directly and
does not use a `PhysicalCallBatch`; accounting begins when the prepared
resources cross into the finalized `Backend` interface.

The following `PhysicalCall` variants are part of the backend-neutral ABI but
are not emitted by `native-executor/src/accounting.rs` or the native call
sites: `PrepareExternal`, `SubmitExternalIngress`, `SubmitExternalEgress`,
`AcknowledgeExternalEgress`, and `QuiesceWorker`. Cross-backend bridge work is
reported as `SubmitInternalTransfer` or `SubmitExitTransfer` at the Local
boundary. The separate host backend has its own equivalent mapper at
`host/src/backend.rs:1818-1847`; Local does not invoke that mapper while
composing host resources.

### Poll ordering and status invariant

`Backend::poll` requires exactly one `PhysicalCall::Poll` and no other
physical record for the operation (`executor/src/backend.rs:330-349`). CUDA,
HSA, and Local first poll the real pending token, map the result, append the
poll record, and then return the original result:

- `Ok(BackendPoll::Pending)` maps to `PhysicalPollStatus::Pending`;
- `Ok(BackendPoll::Complete { .. })` maps to `Complete`;
- every enumerated backend failure maps to `Failed`.

If appending the poll record itself overflows, `record` returns the accounting
overflow after the driver or child operation has already been attempted. This
can mask an underlying poll error. With the executor's fresh empty batch and
the native `MAX_NON_POLL_PHYSICAL_CALLS = 1` contract, a 16-record poll batch
is a protocol violation rather than an expected runtime path.

The Local loop-iteration method re-arms the selected child pending token before
calling its ordinary `submit`; therefore an accounting overflow from that
ordinary submission record can occur after rearm. CUDA and HSA append before
their loop-token rearm. This ordering is observable failure behavior and is
why accounting is kept as a direct precondition at each adapter boundary.

## Run-journal accounting and capacities

The executor creates one fresh `PhysicalCallBatch` for each backend operation
and passes it to `backend_value` (`executor/src/executor.rs:2430-2474` and
`2693-2716`). `backend_value` records the batch before it interprets the
backend result. Consequently, calls emitted by a failed backend operation are
still journal evidence, unless journal ingestion itself fails.
This intentionally permits physical and logical streams to diverge on a
failure: a failed submission can leave `SubmitCalculation` or
`SubmitInternalTransfer` without a logical `TaskSubmitted`, a failed poll can
leave `Poll { status: Failed }` without `TaskCompleted`, and a failed arena
allocation can leave `AllocateArena` without `ArenaAllocated`.

`RunJournal::record_physical` (`executor/src/executor.rs:479-574`) performs
four fixed-capacity actions:

1. iterate the batch's initialized prefix;
2. retain ordered records for non-repeating lifecycle work and the retained
   loop detail;
3. count every `Poll::Pending` in the task-indexed `u128` counter table,
   retaining only the first pending marker selected by the current retention
   policy; and
4. check the declared physical journal bound before extending the ordered
   vector.

`physical_calls_observed` and `physical_calls_compacted` in
`JournalSummary` are saturating `u128` totals. `physical_calls()` therefore
returns ordered non-pending calls, terminal polls, and retained pending
markers, while `pending_poll_counts()` is the exact source for all pending
poll totals. Repeated loop calls are compacted after iteration zero by default;
`JournalCapacity::for_bundle` asks for one retained loop iteration
(`executor/src/executor.rs:248-255`).

### Declared physical journal formula

`JournalCapacity::for_bundle_retaining` computes the bound from the finalized
bundle and the backend's declared non-poll bound (`executor/src/executor.rs:258-347`).
Use these symbols:

```text
B = Backend::MAX_NON_POLL_PHYSICAL_CALLS
T = total finalized task count
L = finalized loop-task count
A = arena-layout count
E = non-loop task count + active loop-task executions in retained iterations
X = exit tasks with a device-to-external destination
F = 2 + 2*A + T + E + X
P = B*F + E + T
```

The declared physical journal capacity is `P`. `F` accounts for bind and
destroy, arena allocate and release, pending-token preparation, task
submission, and external-exit collection. `E` adds one terminal poll per task
execution. `T` reserves one ordered pending marker per finalized task; extra
pending polls are compacted into the task's `u128` counter and do not grow the
ordered vector. All sums and products are checked and report
`ExecutorError::PreparationCapacityOverflow` on host-size overflow.

If retained detail or a backend's actual reports exceed the declared bound,
`record_physical` returns `ExecutorError::JournalCapacityExceeded` for the
physical stream. A pending poll naming a task absent from the finalized bundle
is `ExecutorError::BackendProtocol`; a `u128` pending counter overflow is
`ExecutorError::PendingPollCountOverflow`. These errors are distinct from the
per-operation `Error::PhysicalAccountingOverflow` raised by `record`.
Capacity derivation or a failure before the run journal is created can produce
a `RunFailure` with no journal at all; once the journal exists, failed backend
and teardown paths retain it for inspection.

Teardown uses the same path. Every arena release and the final
`DestroyResources` call gets a new batch, is journaled before its result is
interpreted, and is attempted in order even after an earlier release failure
(`executor/src/executor.rs:1489-1541`). The first teardown error is retained as
the primary failure and the next as `cleanup_error`; physical records already
emitted by those failed calls remain available in the retained `RunJournal`.
The Local composite destroys a bridge first, then HSA, CUDA, and host child
resources (`native-executor/src/local.rs:2064-2096`). CUDA and HSA resource
destruction first checks the poisoned flag, so a poisoned child can reject its
`DestroyResources` call with `BackendPoisoned` after the corresponding
physical attempt has been recorded (`cuda.rs:988-998`, `hsa.rs:1146-1156`).
`DestroyResources` is therefore an attempted action, not proof of successful
teardown. Local publishes `NativeExecutionEvidence::completed` only after all
children succeed; that separate evidence carries `teardown_completed` and
zero live resources (`native-executor/src/local.rs:2079-2108`,
`native-executor/src/evidence.rs:23-57`).

## Reservation ledger

Reservations are scheduler policy, not physical accounting records. The core
types are in `core/src/plan.rs:23-132`:

```text
EXACT_USER_RESERVATION = ByteCount(1_000_000_000)  # decimal bytes
ReservationMechanism = HeldAllocation | EnforcedQuota
ReservationEvidence = NonGpu | GpuDisplay { enabled_connectors: u32 }
ReservationEntry = { device, name, bytes, mechanism, evidence }
```

`ReservationEvidence::required_bytes()` requires exactly one billion bytes for
`NonGpu` and for a GPU with one or more enabled display connectors. A headless
GPU (`GpuDisplay { enabled_connectors: 0 }`) has an explicit zero-byte
exemption. `ReservationLedger::validate` requires all of the following:

- every entry names a topology device;
- no device appears twice;
- RAM and disk use `NonGpu`, while GPU memory uses `GpuDisplay`;
- `entry.bytes` equals the evidence-derived value exactly; and
- every topology device has one entry.

The production native preparation path constructs one entry per topology
device from the driver's mechanism and evidence, names it
`recipe-user-{device_id}`, derives `bytes` from evidence, and validates the
result (`prepare/src/production.rs:1076-1115`). `NativeExecutorDriver` always
chooses `EnforcedQuota` (`prepare/src/production.rs:827-843`). The core type
still serializes `HeldAllocation`, but all native runtime adapters require
`EnforcedQuota` when they realize a device:

- CUDA: `cuda.rs:1648-1662`, error `ArenaMismatch` if the mechanism differs;
- HSA: `hsa.rs:1740-1754`, error `ArenaMismatch` if it differs; and
- host: `host/src/backend.rs:1340-1353`, error `InvalidConfiguration` if it
  differs.

Local evidence is derived from exactly one configured owner. Host RAM/disk
bindings return `NonGpu`; CUDA and HSA bindings return `GpuDisplay` with their
enabled connector count. A second binding for the same device is
`LocalError::DuplicateDevice`; no binding is `LocalError::MissingDevice`
(`native-executor/src/local.rs:2145-2185`).

## Byte-capacity ledger

`CapacityLedger` is post-stabilization scheduler accounting, not a physical
call stream. Each required device has one `CapacityLedgerEntry`:

| field | unit | meaning |
| --- | --- | --- |
| `total` | `ByteCount` | measured total available baseline for the realization |
| `runtime_overhead` | `ByteCount` | bytes consumed by persistent runtime/native resources |
| `fragmentation` | `ByteCount` | measured allocator fragmentation allowance |
| `safety_headroom` | `ByteCount` | measured policy headroom beyond the user reservation |
| `recipe_usable` | `ByteCount` | bytes available to Recipe arena planning |

Do not conflate the three byte baselines: `Topology::Device.capacity` is the
nominal topology property, `DiscoveryProfile::DiscoveredDevice.total_capacity`
seeds optimistic planning, and the native local ledger's final `total` is the
live pre-realization available counter captured for this session. The latter
is deliberately allowed to be lower than either nominal value.

Each field carries `PropertyProvenance`. A realized capacity must be
`Measured` or `Override`, not `Estimated`; `Property::is_schedulable` defines
that rule (`core/src/topology.rs:10-37`). `CapacityLedger::validate`
(`core/src/plan.rs:150-229`) additionally requires one entry for every
topology device, rejects unknown and duplicate devices, and checks with
checked `ByteCount` addition that:

```text
reservation.bytes
+ runtime_overhead.value
+ fragmentation.value
+ safety_headroom.value
+ recipe_usable.value
<= total.value
```

The inequality intentionally permits unclassified slack. Overflow of the
checked sum is `ValidationCode::CapacityOverflow`. A finalized arena layout
must satisfy `layout.size <= recipe_usable.value`
(`core/src/plan.rs:2717-2723`); planner and scheduler perform the same
capacity check before finalization.

### Local native capacity lifecycle

The local candidate factory owns the only native byte-capacity state:

1. **Initial snapshot.** `capture_initial_capacity` queries the current
   available counter for every topology device before candidate resources are
   realized (`native-executor/src/local.rs:2187-2244`). Host RAM uses
   `MemAvailable`, host disk uses `statvfs`, CUDA uses the Driver free-memory
   counter, and HSA uses the ROCr available-memory counter. Missing owners,
   missing bindings, duplicate device keys, driver conversion failures, or an
   unowned device produce `CapacityMismatch` or the underlying backend error.
2. **Initial headroom gate.** `validate_initial_headroom` requires the live
   available count to be at least the exact reservation for every device
   (`local.rs:2246-2265`). This is a quota/headroom check; it does not create a
   dummy allocation.
3. **Warm provisional capacity.** Before a warm pass, the local factory builds
   a provisional bundle with discovery `total_capacity - reservation.bytes`,
   zero runtime overhead, fragmentation, and safety headroom, and the
   discovery provenance (`local.rs:2598-2653`). This optimistic value exists
   only to address and execute the warm candidate.
4. **Post-warm observation.** After a complete maximum-concurrency warm pass,
   the factory releases warm candidate arenas before querying live available
   bytes (`local.rs:1223-1259`). Persistent modules, queues, staging, driver
   allocations, and other retained resources remain. For each device,
   `observe_capacity_ledger` uses the immutable initial baseline and exact
   reservation (`local.rs:3279-3358`):

   ```text
   capped_live = min(live_available, initial_available)
   runtime_overhead = initial_available - capped_live
   recipe_usable = capped_live - reservation.bytes
   fragmentation = 0 B
   safety_headroom = 0 B
   ```

   The subtractions are checked. If live availability falls below the
   reservation, `recipe_usable` underflows and the result is
   `LocalError::CapacityMismatch` with detail `live available bytes fell below
   required user headroom`. Capping live availability prevents a later free
   counter increase from creating negative overhead or changing the original
   scheduler baseline.
5. **Anchor.** `anchor_capacity_snapshot` stores the first successful observed
   ledger and returns a clone for later calls (`local.rs:3262-3277`). System or
   display drift after the first complete observation cannot rewrite the
   finalized contract. The factory rejects a snapshot requested before a new
   complete warm pass, or for a different topology/discovery identity
   (`local.rs:1549-1604`).
6. **Validation and handoff.** `ValidatedCandidateFactory` validates every
   returned snapshot against topology and reservations before preparation
   accepts it (`candidate.rs:382-406`). Preparation requires the configured
   number of snapshots and a stable final tail, then validates a
   `RealizationProfile` (`prepare/src/lib.rs:608-670`). Finalize uses the
   resulting capacity to validate exact arena layouts. `FinalizedBundle` keeps
   the reservation ledger but not the capacity ledger
   (`core/src/plan.rs:2375-2516`); capacity is a pre-final scheduler contract,
   while reservations remain available to runtime resource realization.

`LocalPreparedSession` retains `initial_capacity`, `anchored_capacity`, and the
reservation clone until handoff. `into_backend` requires observed capacity,
released warm arenas, matching candidate identities, and matching reservations
before moving the already-warmed resources into the one-shot runtime backend
(`native-executor/src/local.rs:1262-1374`). It then drops the capacity snapshot,
because the finalized runtime must not re-plan from changing live counters.

## Other fixed capacities

Byte capacity and physical-call capacity are not the only bounded resources:

- CUDA and HSA count the distinct queue IDs required by the finalized tasks on
  each device (`requested_submission_queue_count` in `cuda.rs:1620-1631` and
  `hsa.rs:2264-2275`).
- `ensure_submission_queue_capacity` compares that `usize` count with the
  binding's `u32` maximum and returns `Error::SubmissionQueueLimitExceeded`
  (`native-executor/src/error.rs:243-258`). CUDA has a native-executor ceiling
  constant of 32 because the CUDA Driver API exposes no finite stream-count
  attribute (`cuda.rs:30-33`); HSA uses the maximum supplied by its binding.
- Pending-token pools, arena maps, completion objects, staging, and scratch
  are all realized before the loop. They are not represented as additional
  `PhysicalCall` variants, and their allocation failures are backend or local
  resource errors rather than accounting overflow.

## Failure precedence and invariants

The following ordering is normative for diagnosing a failure:

1. A malformed reservation is rejected before realization, and a malformed
   capacity snapshot is rejected before finalization accepts the evidence.
2. A missing or duplicate native owner, insufficient initial headroom, queue
   limit, or reservation mechanism mismatch rejects realization before the
   finalized runtime can submit work.
3. At a finalized backend boundary, `record` is attempted at the source order
   in the operation table. A full batch returns the native or Local accounting
   overflow and normally skips the subsequent operation.
4. The executor journals the returned batch before converting a backend error
   into `ExecutorError::Backend`. Journal-capacity or protocol failures can
   therefore replace the backend result as the reported executor error.
5. Poll records describe the result observed from the real child operation;
   `Failed` is used for all enumerated child errors. A poll-record append
   overflow occurs after the child operation and takes precedence over its
   result.
6. Teardown still attempts every arena release and resource destruction. Its
   first failure is primary and the next is retained as cleanup evidence; the
   physical records from attempted teardown calls remain in the run journal.

The invariants that must hold for a successful native run are:

- every native operation reports only records from the closed
  `PhysicalCall` vocabulary;
- no operation exceeds 16 records, and each native non-poll operation emits
  at most one;
- every backend poll appends exactly one poll record with a status matching its
  returned result;
- every `AdmissionChunk`, allocation, collection, and release byte field is
  copied from the exact finalized `ByteCount` contract, with no unit conversion;
- pending poll counts are exact even when repeated loop markers are compacted;
- every topology device has one validated reservation and one validated
  capacity entry before finalization;
- every realized capacity is measured or explicitly overridden and its checked
  accounting sum does not exceed total; and
- final arenas fit `recipe_usable`, while runtime resources consume the exact
  reservation mechanism accepted by the native adapter.

These constraints keep physical accounting as observation of the real native
boundary. It cannot create capacity, substitute for reservation validation,
or hide a resource or scheduler failure behind a synthetic record.
