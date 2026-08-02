# `recipe-host`

`recipe-host` is the host-storage backend for Recipe. It owns byte transport
between pre-realized RAM and disk arenas. The crate is deliberately below model
semantics: it does not discover devices, choose a placement, schedule tasks,
compile or load kernels, perform payload calculations, or interpret a file
format. A finalized plan supplies the devices, arena layouts, transfer
contracts, routes, lane claims, and init-image manifests. The host crate turns
those immutable inputs into preallocated copy tokens and then executes only
the byte operations allowed by those contracts.

The implementation has one important temporal rule: every worker, slot,
staging arena, RAM allocation, and disk file reachable from a run is created
before that run enters its loop. `submit`, `poll`, and loop rearm only mutate
already-realized state. Preparation failures remain visible; there is no CPU
calculation fallback, path search, retry path, or lazy resource creation.

## Manifest and module graph

The [manifest](../Cargo.toml) declares package `recipe-host` version `0.1.0`,
Rust edition 2024, MIT licensing, and the description "Preallocated
asynchronous RAM and disk transfer resources for Recipe". Its direct
dependencies are intentionally small:

| Dependency | Boundary role |
| --- | --- |
| [`recipe-core`](../../core) | Typed IDs, byte units, draft/finalized plans, task kinds, phases, resolved arena locations, init images, reservations, and transfer contracts. |
| [`recipe-executor`](../../executor) | The sealed `Backend` trait, closed `BackendWork` variants, arena views, pending requests, metric values, backend poll results, and physical-call accounting. |
| `rustix` with the `fs` feature | The Linux `fallocate` and `statvfs` calls used to allocate disk extents and observe available disk capacity. |

The crate forbids unsafe Rust and denies missing `Debug` implementations. The
implementation dependency graph is a small ownership graph rather than a
layered runtime:

```text
lib.rs
├── arena.rs    Arena, RAM/disk Backing, exact range I/O, disk-file lifetime
├── backend.rs  HostBackend, contracts, plan validation, Backend adapter
├── error.rs    Error and Result vocabulary, Display formatting
└── runtime.rs  fixed job slots, worker threads, asynchronous copy state

arena.rs  ─────┐
runtime.rs ────┼──> backend.rs ──> recipe_executor::Backend
error.rs  ─────┘       │
                       └──────────> recipe_core finalized plans and IDs
```

`lib.rs` declares all four modules privately and reexports the supported
surface. The root exports `Arena`, `ArenaKind`, and `DiskFileSpec`; host backend
configuration, bindings, resources, pending tokens, and the lookup trait;
`Error` and `Result`; and `Runtime`, `RuntimeConfig`, `RuntimeState`,
`SlotCapacity`, `PendingCopy`, and `PollStatus`. `Backing`, `Job`, `JobSlot`,
`ExpectedWork`, contract builders, and low-level copy helpers remain private.
The module files under `host/.docs/src/` are per-source placeholders; this
README is the crate-level implementation map.

## Ownership boundary

The host crate owns the following state:

| State | Owner and lifetime | Meaning |
| --- | --- | --- |
| `Arena` | Cloneable `Arc<Backing>` | One finalized device arena backed by zero-filled RAM or one newly created, preallocated disk file. |
| `Runtime` | `HostResources` or `HostPreparedResources` | Fixed slot table, per-worker staging buffers, worker handles, wake state, and poison state. |
| `HostPending` | One finalized task slot | A `PendingCopy`, optional task-local RAM staging arena, init-image contract, action kind, and submit/terminal flags. |
| `HostResources` | Executor resource state | Device bindings, immutable `ExpectedWork` contracts, prepared task set, pending pool/deferred mode, and host poison state. |
| `HostPreparedResources` | Candidate or pre-final handoff | A runtime and all candidate pending tokens before finalized addresses are known, plus a candidate/finalized handoff marker. |
| `HostBackend` | Moved into `PreparedRun` or a composite local backend | The one-shot backend state machine that owns the configuration or prepared/warmed resources until the executor binds it. |

The host crate does not own the finalized bundle, arena layout policy, task
dependencies, graph values, reservations, native GPU objects, or executor
journal. `recipe-core` owns the immutable declarations. `recipe-executor`
owns the typestate lifecycle, dependency scheduler, logical journal, metric
mailbox, external exit-image vectors, and teardown ordering. Native GPU crates
own CUDA/HSA contexts, queues, modules, buffers, and native completion tokens.

The resulting separation is strict:

```text
Draft/FinalizedBundle + HostBackendConfig
              |
              v
       recipe-host preparation
       (bindings, contracts, slots, staging)
              |
              v
       recipe-executor lifecycle
       (allocate arenas, admit images, run loop, collect exits)
```

The backend accepts only the closed executor work vocabulary. `TaskKind::Metric`
is treated as a four-byte device-to-host readback. Init admission and external
exit egress are transfer operations. `BackendWork::Calculation` is rejected
because payload arithmetic belongs to CUDA or HSA adapters.

## Inputs and outputs

### Configuration inputs

`HostBackendConfig::new(worker_threads, staging_bytes_per_worker, bindings)`
requires a nonzero worker count and nonzero per-worker disk staging capacity.
Each `HostDeviceBinding` identifies exactly one `DeviceId` and one storage
kind:

* `HostDeviceBinding::Ram { device }` selects an in-memory arena.
* `HostDeviceBinding::Disk { device, arena }` selects a caller-resolved
  `DiskFileSpec` path. The path is not opened by configuration.

Bindings must have unique devices, and all disk paths must be globally unique.
`DiskFileSpec::new` rejects only a path without a usable file name. It does not
create a parent directory, resolve a path, or probe capacity. Preparation later
checks the finalized layout devices and reservation ledger. `available_bytes`
reads `/proc/meminfo` (`MemAvailable`, converted from KiB to bytes) for RAM or
uses `statvfs` on the path parent (`f_bavail * f_frsize`) for disk.

The native preparation layers construct this configuration from measured
inventory. `NativeHostPlan::backend_config` and
`native-probe::host_backend_config_from_inventory` create deterministic
run-scoped disk names such as `.recipe-run-{run}-device-{device}-arena` but do
not perform I/O. Candidate realization is the point at which `create_new` and
`fallocate` run, so a path collision is an explicit failure instead of an
overwrite.

### Plan inputs

The direct full-backend path consumes a `FinalizedBundle`. Candidate
preparation consumes a `DraftPlan`, `ReservationLedger`, a config, and a
selected task set. The plan supplies:

| Plan field | Host use |
| --- | --- |
| `arena_layouts` / draft `arena_objects` | Device ownership and exact size for each physical arena. |
| `tasks` | Closed task kinds and run phases. Calculations are rejected; metrics and legal transfers become host contracts. |
| `value_location` and `transfer_endpoints` | Exact device, object, arena offset, dtype, route, lane claims, and byte count checked at submit. |
| `init_images` | One packed image identity and byte size per destination device. Host admission requires an exact manifest match. |
| `reservations` | One entry per configured host device, with `ReservationMechanism::EnforcedQuota`. Held allocations are not accepted as host capacity proof. |
| `BundleIdentity` and selected task IDs | Candidate-to-final handoff identity and task-set equality. |

### Runtime outputs

The public and executor-facing results are bounded and typed:

| Operation | Result |
| --- | --- |
| Arena creation | `Arena` with device, kind, and exact `ByteCount`. Disk creation also owns the file path and file descriptor. |
| Pending preparation | `HostPending` containing one preallocated runtime token and any required task-local staging. |
| Submit | `()` after the copy has been queued, or an `Error` that poisons `HostResources`. |
| Poll | `BackendPoll::Pending` or `BackendPoll::Complete { metric }`; a metric completion returns one little-endian F32 or I32 value. |
| Exit collection | Caller-owned bytes filled from an egress staging arena. The executor wraps these bytes in an `ExitImage`. |
| Physical accounting | One fixed-capacity `PhysicalCall` record per host backend operation, with `MAX_NON_POLL_PHYSICAL_CALLS = 1` and exactly one poll record per `poll`. |
| Teardown | Disk files are synced and removed, worker threads are joined, and the first teardown error is returned. |

## Arena ownership and disk lifecycle

`Arena` is a cloneable handle containing a `DeviceId`, an `ArenaKind`, and an
`Arc<Backing>`. RAM backing stores a zero-filled `Box<[u8]>` behind a `Mutex`
and records its byte length. Disk backing stores a read/write `File`, the
caller-selected path, and its byte length. `Arena::bytes` reports the backing
length, not the current file size.

`Arena::ram` rejects zero bytes, converts the `ByteCount` to `usize`, reserves
exactly that many bytes, and fills the boxed slice with zeroes. Allocation or
address-space conversion failures are `InvalidConfiguration`. `Arena::disk`
rejects zero bytes, opens the path with `create_new`, allocates the exact extent
with `fallocate`, and calls `sync_data` before returning. If extent allocation
or the initial sync fails, the file is closed and removed before the error is
returned. Existing files are never opened or truncated.

All range operations use checked `u64` addition and then checked `usize`
conversion. `write_exact` and `read_exact` are crate-private operations used by
the backend; hidden `bridge_write_exact` and `bridge_read_exact` expose the same
exact-range behavior to the pre-realized native staged bridge. RAM operations
lock the backing mutex and copy one validated slice. Disk operations use
`FileExt::read_at` and `write_at` until the complete range has transferred;
zero-byte progress is reported as `UnexpectedEof` or `WriteZero`.

Dropping the last `Arc<Backing>` for a disk arena performs best-effort
`sync_data` and `remove_file`, ignoring errors. Explicit `Arena::close` is
stronger: it first requires unique ownership with `Arc::try_unwrap`, returning
`SlotBusy` if any clone, pending job, staging arena, or bridge task still holds
the backing, then syncs and removes the disk file. RAM close only consumes the
unique backing. The executor's release step therefore passes the arena by
value only after all pending users have reached terminal state.

## Runtime and copy state machine

`RuntimeConfig` contains a nonzero worker count, a nonzero `SlotCapacity`, and a
nonzero per-worker staging byte count. `Runtime::new` allocates the complete
slot vector, shared atomics/condition variable, one staging buffer per
`min(worker_threads, slots)`, and the worker handle vector before spawning any
thread. Workers are named `recipe-host-{index}`. If a later thread spawn fails,
the runtime sets its stopping flag, wakes existing workers, joins them, and
returns `ThreadSpawn`.

Each `JobSlot` has an atomic state, a mutex-protected optional `Job`, and a
mutex-protected optional failure. The state values are private but form this
closed sequence:

```text
UNCLAIMED -> PREPARED -> QUEUED -> RUNNING -> COMPLETE
                                      \-> FAILED
COMPLETE -> PREPARED                 (reset/rearm only)
```

`Runtime::prepare_copy` claims the next preallocated slot with an atomic
counter and transitions it to `PREPARED`. The counter is monotonic; when the
fixed table is exhausted it returns `SlotCapacityExhausted`. Host loop reuse
does not allocate another slot. It resets the same terminal token after
completion. A `PendingCopy` retains an `Arc<JobSlot>` and the shared runtime
state, so all source and destination backings remain live while a queued job
is running.

`PendingCopy::submit` requires runtime and slot readiness, rejects zero-byte or
out-of-bounds ranges, obtains the job mutex without blocking, stores cloned
backings and offsets, changes the state to `QUEUED`, and wakes one worker.
`poll` is nonblocking: queued/running returns `Pending`, complete returns
`Complete`, failed returns the stored worker error, and unsubmitted states are
`InvalidPendingState`. `reset` is crate-private and accepts only `COMPLETE`; it
clears the job and failure, then returns the slot to `PREPARED`.

Workers repeatedly scan all slots and claim queued work with
`QUEUED -> RUNNING`. They execute one copy, publish `COMPLETE` or `FAILED`, and
continue until `stopping` is set and no slot is queued or running. When no work
progresses they wait on a condition variable with a ten-millisecond timeout,
guarded by a wake-generation counter. A poisoned wake or failure mutex sets
the shared poison flag. `Runtime::state` exposes `Poisoned` while the local
state is otherwise `Ready`; `close` sets stopping, wakes all workers, joins
every handle, and reports `ThreadPanicked` if a join fails. `Drop` invokes the
same shutdown path as a best-effort cleanup.

### Copy implementations

`execute_copy` dispatches only on the four backing pairs:

| Source | Destination | Implementation |
| --- | --- | --- |
| RAM | RAM | Mutex-protected slice copy. The same backing uses `copy_within`; distinct backings lock in `Arc` address order to avoid lock-order deadlock. |
| RAM | Disk | Lock source and issue exact `write_at` calls. |
| Disk | RAM | Lock destination and issue exact `read_at` calls. |
| Disk | Disk | Chunk through the worker's private staging buffer. Same-file overlapping moves where destination starts later copy backward; all other moves copy forward. |

The worker staging buffer is used only for disk-to-disk movement. Task-local
staging arenas in `HostPending` are separate and hold init images, metric
readbacks, or external egress payloads.

## Host backend contracts

### Backend states

`HostBackend` is a one-shot adapter with four internal states:

```text
Ready(config)
   | bind_resources / bind_partition
   v
Prepared(HostPreparedResources) -> Bound
Warmed(HostResources)            -> Bound
Ready(config) -------------------> Bound
```

Every binding operation replaces the state with `Bound` before attempting the
handoff. A second bind therefore fails with `BackendState`, including after a
failed handoff. The convenience methods `prepare_partition`,
`allocate_partition`, `submit_partition`, `submit_loop_partition`,
`poll_partition`, `collect_partition`, `release_partition`, and
`destroy_partition` are used by the composite native backend. They deliberately
do not add a second execution implementation. `bind_partition` is the
task-subset form; direct `Backend::bind_resources` uses all bundle tasks.

### Expected work

Before any token is prepared, `task_contracts` converts each selected finalized
task into an immutable `ExpectedWork` entry:

* `ExpectedWork::Init` stores the packed `ValueId`, resolved destination,
  transfer bytes, and submission slots. The destination must be a device and
  the bundle's init-image manifest must have the same image and byte count.
* `ExpectedWork::Transfer` stores the `WorkClass`, resolved endpoints, byte
  count, one-hop route, lane claims, and submission slots. A route longer than
  one link is rejected as `UnsupportedWork`; the planner must expand a
  multi-link path into dependency-chained tasks.
* `ExpectedWork::Metric` stores metric and slot IDs, the resolved four-byte
  value location, and submission slots.

The contract's class is derived from phase and endpoint class. Only these
combinations are legal:

| Phase | Resolved endpoints | Class |
| --- | --- | --- |
| Init | External -> Device | `InitAdmission` |
| Init or Loop | Device -> Device | `InternalTransfer` |
| Exit | Device -> Device or Device -> External | `ExitTransfer` |

Any external-to-external, host-to-external in init/loop, external-to-device in
loop/exit, or other phase mismatch is a `Protocol` error. `phase_accepts`
enforces the same relationship when the executor asks for a pending token.

### Pending resources

`HostPending` is the host's per-task completion token. It contains:

| Field | Purpose |
| --- | --- |
| `task`, `class` | Identity and closed work class checked at every boundary. |
| `copy` | One preallocated runtime slot. |
| `staging` | Optional task-local RAM arena. Init uses image bytes, egress uses transfer bytes, and metrics use exactly four bytes. Device-to-device copies use no task-local staging. |
| `admission` | Exact init image contract, if this is an init task. |
| `action` | `None`, `Metric { dtype }`, or `Egress`, controlling completion output. |
| `submitted`, `terminal` | Host-side lifecycle flags preventing duplicate submit, poll-before-submit, and collection-before-completion. |

`HostResources::prepare_pending` first checks health and the finalized task
contract, then records that the task has been prepared exactly once. In a
warm/pre-final pool it removes the already-realized token and checks task,
class, and admission identity. In deferred mode it calls `Runtime::prepare_copy`
and creates any task-local staging arena. There is no allocation in submit or
poll.

`HostPreparedResources` uses the same token shape but stores every selected
task in a `BTreeMap`. Candidate realization builds this map from the draft,
rejecting calculations, invalid endpoint classes, missing metric values,
missing/mismatched init manifests, duplicate task IDs, and any slot or staging
allocation failure. It also checks that configured host devices appear in draft
arena objects and that each has an enforced reservation.

## Submission and completion paths

The host submission path is contract-first. `HostResources::submit` requires a
healthy resource, an unsubmitted/nonterminal token, matching task ID and work
class, and the exact finalized contract. A successful branch sets
`pending.submitted = true`; every error sets `HostResources::poisoned = true` so
the next operation fails closed.

### Init admission

For `BackendWork::InitAdmission`, the image destination, value, byte count,
submission slots, and image length must match `ExpectedWork::Init`. The packed
image is written into the token's preallocated RAM staging arena. The resolved
destination arena and offset are checked against the finalized arena map, then
the runtime copies staging to the device-owned host arena. The logical
executor event records one init admission; no host calculation occurs.

### Internal and exit transfers

For internal or exit transfer work, `validate_transfer` compares task class,
resolved endpoints, bytes, route, lane claims, and submission slots. The only
host-resolved forms are:

* Device -> Device: checked source and destination arenas are passed directly
  to the runtime copy token.
* Device -> External during exit: checked source is copied into the token's
  preallocated egress staging arena. The executor later invokes `collect_exit`
  to copy that staging into caller-owned output bytes.

External -> Device and External -> External are not representable by this
backend and fail with `Protocol`. Init External -> Device is admitted through
the separate init-image branch, which supplies the caller's image bytes.

### Metric readback

For `BackendWork::Metric`, metric ID, slot ID, resolved value location, and
submission slots must match. Exactly four bytes are copied from the device arena
location into task-local RAM staging. On completion, the bytes are decoded as
little-endian `f32` or `i32` using the finalized dtype. The returned
`MetricValue` is consumed by `recipe-executor`: user metrics publish into the
capacity-one mailbox, while fault-readback metrics check for zero or return a
device-fault error. A host metric never allocates a mailbox or owns metric
policy.

### Calculation rejection

`BackendWork::Calculation` always returns `UnsupportedWork` with the explicit
message that payload calculations require a CUDA or HSA GPU adapter. The host
backend does not inspect kernel templates, artifacts, input lists, output lists,
or fault flags beyond rejecting the variant.

### Poll, collect, rearm, and recycle

`poll_pending` requires a submitted, nonterminal token. Pending runtime state
maps to `BackendPoll::Pending`. Complete state marks the token terminal and,
for metrics, reads the four-byte staging value. A runtime failure poisons the
resource and is returned unchanged. `rearm_pending` accepts only a submitted
terminal token, resets its runtime slot, and clears the two host flags.
`prepare_loop_pending` accepts a never-submitted token or rearms a completed
one; active or inconsistent flags are protocol errors.

`collect_exit` requires terminal completion, the same task ID, the `Egress`
action, and an output slice whose length equals the contract byte count. It
revalidates the finalized transfer and requires an external destination before
reading staging. `recycle_pending` is used by warm preparation after a terminal
warm pass. It requires the task to be present in the prepared set, resets the
token, removes the task from the active set, and inserts it into the pre-final
pool. Deferred resources cannot recycle because they have no warm pool.

## Candidate, warm, and finalized handoff

Candidate preparation and runtime binding are separate states because a draft
does not yet contain physical arena offsets. `HostPreparedResources::realize`
validates selected task IDs against the draft, validates host bindings against
draft arena objects, requires enforced reservation entries, creates a runtime
with exactly `max(selected_tasks, 1)` slots, and pre-realizes every pending
token. A failure closes the runtime before returning.

The native local factory uses this path as follows:

```text
LocalCandidateFactory::realize_candidate
  -> HostBackend::prepare_candidate(DraftPlan, ReservationLedger, config, host_tasks)
  -> HostPreparedResources (Candidate, all host tokens ready)
  -> activate_warm_resources
  -> HostPreparedResources::bind_candidate(provisional bundle, host tasks)
  -> HostResources (Prepared pending pool)
  -> warm passes and capacity observation
  -> HostResources::validate_handoff(final bundle, host tasks)
  -> LocalBackend::from_warmed / HostBackend::from_warmed
```

`bind_candidate` rejects a finalized handoff marker, checks bundle device
coverage in partition mode, requires exact pending-pool task equality, builds
finalized contracts, and verifies every pre-realized init admission. After
warm passes, `HostResources::validate_handoff` requires no active prepared
tokens, an intact pre-final pending pool, matching final task IDs, and matching
admission contracts before replacing its contracts with final ones.

The alternate `HostPreparedResources::validate_handoff` path transitions a
candidate object to a `Finalized` marker containing `BundleIdentity`, task IDs,
and contracts. A later `HostPreparedResources::bind` consumes only that exact
marker and rejects a different bundle or task partition. Attempting to bind a
candidate without validation, validate twice, or rebind a finalized object is
an explicit `BackendState` error.

## `recipe-executor` integration

`HostBackend` implements the sealed `recipe_executor::Backend` trait:

```text
type Arena    = recipe_host::Arena
type Resource = recipe_host::HostResources
type Pending  = recipe_host::HostPending
type Error    = recipe_host::Error
MAX_NON_POLL_PHYSICAL_CALLS = 1
```

The executor's `PreparedRun::prepare` calls `bind_resources`, then
`realize_phase` calls `prepare_pending` once for every init, loop, and exit
task. Thus a direct host run has all runtime slots and task-local staging before
`initialize`. `PreparedRun::initialize` invokes `allocate_arena` for each
finalized layout and then submits/polls init admissions. `InitializedRun` starts
the loop. Each loop iteration enters `submit_loop_iteration`, where host
rearm/preparation happens before the exact `BackendWork` is submitted. The
executor polls until every task is complete, publishes any metric, and starts
the next finalized iteration without changing the task graph. Exit tasks submit
device-to-device or device-to-external transfers; completed external exits call
`collect_exit`. Finally the executor releases every arena and calls
`destroy_resources`.

The host adapter records physical calls at the trait boundary. Every non-poll
operation records one `PhysicalCall`: bind, prepare pending, allocate arena,
the mapped submit call, collect exit, release arena, or destroy resources.
`poll` maps `Pending`, `Complete`, and every host error to
`PhysicalPollStatus::{Pending,Complete,Failed}` and appends exactly one poll
record. `record` uses the executor's fixed 16-record batch and translates a
batch overflow into `PhysicalReportOverflow`. Host physical records are
accounting only. Logical task completion, phase ordering, watchdog handling,
metric publication, and exit-image ownership remain executor responsibilities.

The executor's read-only `ArenaSet` is adapted by `HostArenaLookup`. The host
implementation returns only the arena keyed by the requested device, so a
wrong-class local arena is not accidentally used. `checked_arena` additionally
checks device identity and `arena_offset + bytes <= Arena::bytes()` for every
submission.

## Composite local and staged bridge integration

`recipe-native-executor::LocalBackend` composes host, CUDA, HSA, and staged
cross-backend partitions. Its `LocalResources` stores an optional
`HostResources`, and its `LocalPending` has a `Host(HostPending)` variant. During
bind it classifies each finalized device and task, passes the host task subset
to `HostBackend::bind_partition`, and rejects host tasks when no host config is
present. During preparation, allocation, submit, loop rearm, poll, exit
collection, release, and destruction it verifies the task owner is `Host` and
delegates directly to the corresponding host method. Host arenas are wrapped
as `LocalArena::Host`; `ProjectedArenas` implements `HostArenaLookup` by
returning only that variant.

`StagedCrossBackend` uses the host crate for one-hop transfers that cross local
backend ownership. It pre-realizes one host staging worker per staged task and
native leg resources before final handoff. A host endpoint itself has no
separate staging buffer: the bridge's `HostStageJob::Read` and `Write` call the
hidden `Arena::bridge_read_exact` and `bridge_write_exact` methods to move bytes
between an `Arena` and a retained CUDA/HSA staging allocation. The host crate
still owns only the exact arena I/O and error conversion; native-executor owns
the pointer lifetime proof, worker message, native completion, and staged bridge
state. Bridge shutdown joins its workers before host resources are destroyed.

This bridge is not a host-backend calculation path. It accepts only a finalized
one-hop device-to-device transfer crossing ownership classes, preserves route,
lane claims, and submission slots, and leaves task scheduling and logical
completion to `recipe-executor`.

## Resource teardown

The normal executor teardown order is:

```text
completed exit tasks
  -> collect external exit bytes
  -> release each Arena by device
       -> Arena::close, sync disk and remove disk path
  -> destroy HostResources
       -> drop pending pool/staging arenas
       -> Runtime::close, stop and join workers
```

`HostResources::destroy` drops its pending pool before closing the runtime. A
queued job can still retain its backing through the runtime's shared slot, so
`Runtime::close` then drains queued/running work and joins every worker before
returning. `HostPreparedResources::destroy` follows the same order for
candidate resources.
The local composite backend destroys bridge resources, HSA, CUDA, then host
resources and retains the first error while attempting every remaining cleanup.
If an arena still has an `Arc` clone held by a pending token or bridge job,
explicit release returns `SlotBusy`; the caller must preserve the failure and
not invent another release path. Drop remains a best-effort disk-file cleanup
for abandoned handles.

## Invariants and failure catalog

The following invariants are enforced in the host source and are part of the
executor ABI:

1. Worker count, slot capacity, runtime staging capacity, arena sizes, and copy
   sizes are nonzero.
2. Device bindings and disk paths are unique. Every configured host device has
   a finalized layout and an enforced reservation; full binding requires
   complete layout coverage, while a local partition checks only its own
   binding set.
3. A host task is exactly one metric or one legal transfer. Payload calculation
   tasks fail closed with `UnsupportedWork`.
4. Init admissions match the exact image `ValueId`, destination, byte count,
   submission slots, and packed image length in the finalized manifest.
5. Transfer submissions match endpoints, byte count, route, lane claims,
   submission slots, phase, and class. Multi-hop routes are planner-expanded
   before the host backend sees them.
6. Every resolved arena range uses checked arithmetic and a device ownership
   check. Zero-byte copies, integer conversion overflow, and out-of-bounds
   ranges do not reach a worker.
7. A pending token is prepared once, submitted once per activation, polled only
   while active, collected only after terminal completion, and rearmed only from
   its own terminal state. Warm recycling is terminal-only and task-unique.
8. Runtime worker failures poison the pending/resource path. Errors are not
   retried and a poisoned resource does not accept later operations.
9. Disk files are created with `create_new`, extent-allocated, synced before
   use, and removed only by explicit unique close or last-backing drop.
10. Physical reporting is fixed-capacity and one-record-per-operation; logical
    lifecycle accounting is owned by the executor journal.

`Error` is `#[non_exhaustive]` and groups failures by boundary:

| Error family | Examples and meaning |
| --- | --- |
| Configuration/path | `InvalidConfiguration`, `InvalidPath`, `DuplicateDevice`, `MissingDevice`, `WrongDeviceKind`, `BackendState`. Inputs or handoff state are impossible for the selected plan. |
| Contract/protocol | `Protocol { task, detail }`, `UnsupportedWork { task, detail }`. The caller supplied a task, work variant, endpoint, phase, route, or manifest that differs from finalized state. |
| Capacity/arithmetic | `RangeOverflow`, `OutOfBounds`, `SlotCapacityExhausted`, `SlotBusy`, `PhysicalReportOverflow`. Checked arithmetic or fixed tables prevent hidden growth. |
| I/O/worker | `Io`, `WorkerFailed`, `ThreadSpawn`, `ThreadPanicked`. The underlying operation or worker lifecycle failed. |
| Token/runtime state | `InvalidPendingState`, `Poisoned`. A token was used in the wrong state or a mutex/worker failure made the runtime unsafe to continue. |

`poll` reports any of these host failures as a failed physical poll before
returning the original error. The executor wraps the error as a backend
operation failure, records the bounded journal evidence, and performs its
best-effort release and destruction sequence.

## Verification boundary

`cargo check -p recipe-host` and workspace formatting/linting prove Rust shape,
forbidden unsafe code, and trait compatibility only. They do not prove disk
capacity, worker scheduling, or data correctness. Runtime evidence must enter
through the production executor or native local backend, use a finalized plan
and real RAM/disk origins, inspect resulting bytes independently, and verify
that all arenas, files, pending tokens, and worker threads are gone after
teardown. A successful host acceptance path therefore demonstrates the
complete `bind -> prepare -> allocate -> submit -> poll -> collect -> release ->
destroy` boundary, not merely a status or physical-call count.
