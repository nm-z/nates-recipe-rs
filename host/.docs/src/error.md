# `recipe_host::Error`

This page is the error contract for `host/src/error.rs`. The public crate re-exports
`Error` and the alias `Result<T> = core::result::Result<T, Error>` from
`host/src/lib.rs`. The enum is `Clone`, `Debug`, `PartialEq`, and `Eq`, is
`#[non_exhaustive]`, implements `Display` and `std::error::Error`, and carries no
source error. `#[non_exhaustive]` means downstream matches must include a wildcard
even though the current implementation lists every variant below.

The payloads deliberately retain stable host context rather than the original
error object. `DeviceId` and `TaskId` identify the affected resource or task.
`io::ErrorKind` retains the operating-system category, while each static
`operation` or `detail` string identifies the host operation that rejected or
failed. `error::io(operation, &io::Error)` is the crate-private normalizer: it
constructs `Error::Io { operation, kind: error.kind() }` and discards the original
`io::Error` value.

## Rendering

`Display` is the only human-readable rendering supplied by this crate. The exact
formats are:

| Variant | Display text |
| --- | --- |
| `InvalidConfiguration(detail)` | `invalid host runtime configuration: {detail}` |
| `InvalidPath` | `host storage path has no usable file name` |
| `DuplicateDevice(device)` | `host device {device} appears more than once` |
| `MissingDevice(device)` | `host device {device} is not bound` |
| `WrongDeviceKind(device)` | `host device {device} has the wrong storage kind` |
| `BackendState(detail)` | `invalid host backend state: {detail}` |
| `Protocol { task, detail }` | `host backend protocol violation for task {task}: {detail}` |
| `UnsupportedWork { task, detail }` | `host backend cannot execute task {task}: {detail}` |
| `PhysicalReportOverflow` | `host physical-call report exceeded its fixed capacity` |
| `RangeOverflow` | `host transfer range overflowed` |
| `OutOfBounds { device }` | `host transfer exceeds arena {device}` |
| `SlotCapacityExhausted` | `preallocated host job slots are exhausted` |
| `SlotBusy` | `preallocated host job slot is busy` |
| `InvalidPendingState` | `host pending token is in the wrong state` |
| `WorkerFailed { operation, kind }` | `host worker {operation} failed with {kind:?}` |
| `Io { operation, kind }` | `host {operation} failed with {kind:?}` |
| `ThreadSpawn(kind)` | `host worker creation failed with {kind:?}` |
| `ThreadPanicked` | `host worker thread panicked` |
| `Poisoned` | `host runtime is poisoned` |

`Debug` is the derived enum representation. There is no `source()` chain from a
host error, including for `Io` and `WorkerFailed`, because the enum stores only
`ErrorKind`.

## Propagation and state transitions

1. Functions in `host/src/arena.rs`, `host/src/runtime.rs`, and
   `host/src/backend.rs` return the alias directly. Their `?`, `map_err`, and
   `ok_or` paths preserve the `Error` value unchanged unless a lower-level
   `io::Error` is normalized into `Io` or a worker failure is normalized into
   `WorkerFailed`.
2. `HostResources::submit` (`backend.rs:759-902`) marks its resource
   `poisoned = true` for every error returned by validation, arena access, or
   `PendingCopy::submit`. `HostResources::poll_pending`
   (`backend.rs:905-939`) does the same for an error returned by
   `PendingCopy::poll`, including a worker failure. Protocol checks made before
   that poll, or while decoding a completed metric, return directly without
   setting the boolean. After the poison bit is set, `ensure_healthy` returns
   `Poisoned` from subsequent resource operations. Errors while configuration,
   realization, handoff, release, or destruction are returned directly and do
   not set this boolean.
3. The asynchronous `Runtime` has a separate shared poison flag. A poisoned
   worker mutex or wake mutex sets that flag; `Runtime::state()` then reports
   `RuntimeState::Poisoned`, and `Runtime::prepare_copy` or
   `PendingCopy::submit` rejects with `Poisoned`. A normal worker I/O or range
   failure is stored in the slot and is returned by `PendingCopy::poll`; it does
   not set the shared flag by itself. The host resource sees that poll error and
   then sets its own poison boolean as described above.
4. `HostBackend` implements `recipe_executor::Backend` with `type Error = Error`.
   Every trait operation first calls `record` to append its physical call. A
   `PhysicalReportOverflow` returned by `record` prevents the operation from
   reaching the host resource, except in `poll`, where the resource is polled
   first and a report overflow can replace the original poll result. In
   `Backend::poll` (`backend.rs:1204-1243`), every current `Error` variant is
   classified as `PhysicalPollStatus::Failed`, a `PhysicalCall::Poll` is
   recorded when capacity permits, and the original `Result` is returned.
5. `Runtime::close` joins every worker and reports `ThreadPanicked` if a join
   fails, but still transitions the runtime to `Closed`. `Drop` calls the same
   shutdown path and intentionally discards its result, so a panic discovered
   during automatic drop is not observable by the caller. `Backing::drop` also
   ignores disk `sync_data` and file-removal failures; explicit `Arena::close`
   is the path that reports those failures as `Io`.

At the workspace boundaries, host errors remain typed only where the boundary
allows it:

- `NativeHostPlan::backend_config` (`src/native_prepare.rs:143-163`) and
  `host_backend_config_from_inventory` (`native-probe/src/bindings.rs:94-118`)
  return `recipe_host::Result<HostBackendConfig>` directly. The inference and
  training entry points call those functions and convert an error with
  `error.to_string()` into `NativePreparationError::LocalConfiguration`, so
  configuration failures at those two entry points are rendered text rather
  than retained as a typed source.
- `native-executor/src/local.rs` wraps host results with
  `LocalError::Host`. Its `Display` prefix is `host partition failed: {error}`
  and its `std::error::Error::source()` returns the contained host error. This
  wrapper is used for candidate preparation, handoff, binding, pending-token
  preparation, arena allocation, submission, loop rearming, polling, exit
  collection, arena release, capacity observation, recycling, and resource
  destruction.
- `native-executor/src/bridge.rs` wraps staging-worker host results with
  `StagedBridgeError::Host`, whose `Display` prefix is
  `cross-backend host staging failed: {error}` and whose `source()` returns the
  host error. `From<recipe_host::Error>` constructs this wrapper. A host-stage
  worker stores its `Error` until `HostStageWorker::poll` observes a failed
  worker, then returns it through `StagedBridgeError::Host`.

## Variant inventory

The lists below include every current constructor in `host/src`, grouped by the
function that constructs it. A listed detail is part of the rendered contract;
the line numbers identify the current checkout and are not an additional API.

### `InvalidConfiguration(&'static str)`

This is a caller or plan/configuration invariant failure, allocation refusal, or
unusable host-capacity report. It is constructed at:

- `arena.rs:96-107`: `"RAM allocation must be nonzero"`,
  `"RAM arena size does not fit the host address space"`, and
  `"RAM arena allocation failed"`.
- `arena.rs:196-201`: `"disk allocation must be nonzero"`.
- `runtime.rs:27-37`: `"job slot capacity must be nonzero"`.
- `runtime.rs:50-67`: `"worker thread count must be nonzero"` and
  `"per-worker disk staging capacity must be nonzero"`.
- `runtime.rs:137-163`: `"host slot table allocation failed"`,
  `"host staging table allocation failed"`, and
  `"host worker table allocation failed"` when preallocating runtime tables.
- `runtime.rs:346-350`: `"host copy size must be nonzero"`.
- `runtime.rs:365-371`: `"host staging allocation failed"`.
- `backend.rs:62-78`: `"host backend worker count must be nonzero"` and
  `"host backend staging capacity must be nonzero"`.
- `backend.rs:474-479`: `"host binding names a device absent from the candidate arena objects"`.
- `backend.rs:1286-1300`: `"host disk arena paths must be globally unique"`.
- `backend.rs:1313-1337`: `"host binding names a device absent from the finalized bundle"`.
- `backend.rs:1348-1353`: `"host backend requires a scheduler-enforced quota"`.
- `backend.rs:1355-1372`: `"/proc/meminfo has no numeric MemAvailable field"`.

The allocation forms fail before the corresponding arena, runtime, or pending
resource is returned. If an `InvalidConfiguration` reaches
`HostResources::submit`, that active resource is poisoned; configuration or
capacity queries before an active resource exists simply return the error to
their caller.

### `InvalidPath`

- `arena.rs:25-31`, `DiskFileSpec::new`: the supplied path has no
  `file_name()`.
- `backend.rs:1375-1377`, `available_disk_bytes`: the disk path has no
  parent directory.

The operation stops before file creation or `statvfs`. It is also classified as
`PhysicalPollStatus::Failed` if it reaches the backend poll boundary.

### `DuplicateDevice(DeviceId)`

`backend.rs:1286-1293`, `validate_bindings`, returns the duplicate device while
validating `HostBackendConfig` and while re-indexing bindings for candidate or
finalized realization. No duplicate binding is inserted. The error is returned
before runtime workers or arenas are created.

### `MissingDevice(DeviceId)`

The variant identifies a required device for which no binding or arena is
available:

- `backend.rs:97-106`, `HostBackendConfig::available_bytes`, unknown configured
  device.
- `backend.rs:745-755`, `HostResources::allocate_arena`, missing binding for a
  finalized layout.
- `backend.rs:1064-1073`, `HostResources::available_bytes`, missing binding.
- `backend.rs:1313-1328`, `validate_bundle_devices`, missing binding when full
  bundle coverage is required.
- `backend.rs:1340-1344`, `validate_reservations`, missing reservation-ledger
  entry for a binding.
- `backend.rs:1728-1746`, `checked_arena`, missing host arena in the lookup.

The operation returns before accessing the absent arena. If a missing-device
error reaches `HostResources::submit`, the enclosing method poisons that active
resource.

### `WrongDeviceKind(DeviceId)`

There is no constructor for this variant in the current repository. The only
uses are the `Display` arm (`error.rs:55`) and the exhaustive `Backend::poll`
classification (`backend.rs:1220`). It remains part of the public,
non-exhaustive enum and must be handled by downstream wildcard matches.

### `BackendState(&'static str)`

This denotes an invalid host-resource lifecycle or handoff, not a device or
operating-system failure. Current detail strings are:

- `backend.rs:158-168` and `1129-1148`: `"resources may be bound only once"`.
- `backend.rs:510-529`: `"finalized host partition differs from its prepared handoff"` and
  `"candidate host resources were not validated for finalized handoff"`.
- `backend.rs:541-565`: `"finalized host resources cannot be rebound as a warm candidate"` and
  `"warm host partition differs from its pre-final pending pool"`.
- `backend.rs:590-608`: `"candidate host handoff was validated more than once"` and
  `"finalized host partition differs from its pre-final pending pool"`.
- `backend.rs:1049-1060`: `"warm host resources have no pre-final pending pool"`.
- `backend.rs:1076-1095`: `"warm host pending tokens were not all recycled"`,
  `"warm host resources lost their pre-final pending pool"`, and
  `"final host partition differs from its warm pending pool"`.

The state transition is rejected and the caller retains ownership of the
resource unless the surrounding state machine is already consuming it. These
errors are failed poll statuses when observed through `Backend::poll`; they do
not themselves set `HostResources::poisoned` unless returned by `submit` or
`poll_pending`.

### `Protocol { task: TaskId, detail: &'static str }`

Protocol errors mean that a finalized task contract, pending-token state, arena
ownership, endpoint class, or lifecycle phase disagrees with the request. The
`task` field is the task being checked. `TaskId::new(0)` is used for the two
release-arena ownership checks because no task is available at that boundary.

Every current detail string is listed here, grouped by the validation phase.

Candidate and handoff validation (`backend.rs:452-626`, `1390-1593`):

- `"host candidate partition names an absent task"`
- `"warm host task is absent from the candidate contract"`
- `"warm init-image manifest differs from prepared host admission"`
- `"finalized host contract is absent from the prepared pending pool"`
- `"finalized init-image manifest differs from the prepared host admission"`
- `"host candidate partition contains a calculation"`
- `"host candidate metric value is absent"`
- `"host candidate admission has no init-image manifest"`
- `"host candidate admission differs from its init-image manifest"`
- `"host candidate admission has no device image destination"`
- `"host candidate transfer has a non-transfer class"`
- `"host candidate task appears more than once"`
- `"host candidate transfer phase or endpoint class is invalid"`
- `"host candidate transfer staging contract is invalid"`

Pending preparation and submission (`backend.rs:685-902`):

- `"pending request names no finalized host task"`
- `"pending request differs from the finalized host task"`
- `"host pending token was prepared more than once"`
- `"pre-final host pending resource is absent"`
- `"pre-final host pending resource differs from finalized request"`
- `"host pending token is not ready for this task"`
- `"submitted work names no finalized host task"`
- `"submitted work class differs from the finalized host task"`
- `"host admission differs from its finalized contract"`
- `"host admission has no preallocated staging arena"`
- `"host metric differs from its finalized contract"`
- `"host metric has no preallocated staging arena"`
- `"submitted host work variant differs from its finalized contract"`

Polling, loop rearming, exit collection, and recycling (`backend.rs:905-1108`):

- `"host pending token is not active"`
- `"completed host metric has no staging arena"`
- `"only a terminal host loop token may be rearmed"`
- `"an active or inconsistent host loop token may not be submitted again"`
- `"host exit collection differs from its completed task"`
- `"collected host exit has no finalized contract"`
- `"collected host exit is not a finalized transfer"`
- `"host exit collection has no external destination"`
- `"completed host exit has no staging arena"`
- `"only one terminal host pending token may be recycled"`
- `"host pending pool already contains the recycled task"`
- `"final host task is absent from its warm pending pool"`
- `"final host admission differs from its warm manifest"`

Finalized contract and transfer checks (`backend.rs:1596-1815`):

- `"metric value has no finalized location"`
- `"transfer has no finalized endpoints"`
- `"host admission has no device destination"`
- `"host admission device has no finalized init-image manifest"`
- `"host admission differs from the finalized init-image manifest"`
- `"host transfer received a non-transfer work class"`
- `"host task appears more than once"`
- `"host transfer phase or endpoint class is invalid"`
- `"host transfer differs from its finalized contract"`
- `"host egress has no preallocated staging arena"`
- `"host transfer has unsupported resolved endpoints"`

Release ownership (`backend.rs:226-234` and `1262-1278`):

- `"released host arena belongs to another device"`

The failed check prevents the associated host operation. A protocol error from
`HostResources::submit` sets the resource poison bit. A protocol error from
`poll_pending` is set on that bit only when it came from `PendingCopy::poll`;
the token-state and completed-metric checks return directly. Candidate/handoff
checks return during realization or binding and do not run a loop operation.

### `UnsupportedWork { task: TaskId, detail: &'static str }`

This is a deliberate fail-closed result for work that the host adapter cannot
own. The three constructors are:

- `backend.rs:878-883`, submitting `BackendWork::Calculation`: `"payload calculations require a CUDA or HSA GPU adapter"`.
- `backend.rs:1605-1611`, realizing a host-only contract for a calculation:
  `"host-only backend cannot bind a GPU calculation task"`.
- `backend.rs:1657-1663`, a transfer route longer than one planner-expanded hop:
  `"host backend requires planner-expanded one-hop transfers"`.

The first form marks an active `HostResources` value poisoned because it is a
submission error. The latter two fail realization before a usable host contract
is returned. All forms render as `host backend cannot execute task {task}: ...`.

### `PhysicalReportOverflow`

`backend.rs:1840-1846`, `record`, converts a failed
`PhysicalCallBatch::try_push` into this unit variant after consuming the batch's
overflow value. It is reachable from every `recipe_executor::Backend` callback
that records a physical call: bind, prepare, arena allocation, submit, loop
submit, poll, exit collection, arena release, and resource destruction. The
record is fixed-capacity; no retry or alternate record path exists. The variant
is returned unchanged and is classified as `PhysicalPollStatus::Failed` when it
is the result of a resource poll.

### `RangeOverflow`

This means an offset, length, byte-count conversion, or checked arithmetic could
not be represented safely. It is constructed at:

- `arena.rs:156-181`, source/destination slice length conversion and host-range
  conversion for RAM or disk arena reads and writes.
- `arena.rs:236-288`, backing-range checks, `usize` range conversion, and file
  offset increments in exact disk reads/writes.
- `runtime.rs:480-555`, overlapping RAM copies, disk staging chunk arithmetic,
  host-range conversion, and copied-byte accumulation.
- `runtime.rs:558-598`, worker disk read/write offset increments.
- `backend.rs:794-797`, init-image length conversion; `backend.rs:976-979`,
  exit-buffer length conversion; `backend.rs:1368-1387`, RAM and disk capacity
  multiplication; and `backend.rs:1736-1745`, checked finalized-arena offsets.

`validate_range` uses `OutOfBounds` for a copy whose checked end is outside its
arena, including an offset-plus-length overflow at that specific pre-submit
check. Other helper paths use `RangeOverflow` for the same arithmetic class when
the failure is representability or checked addition. No slice or file operation
is attempted after the error. Worker-originated range errors are stored in the
pending slot and then returned by `poll`.

### `OutOfBounds { device: DeviceId }`

- `runtime.rs:346-357`, `validate_range`, rejects a checked end outside the
  source or destination arena after the separate zero-byte configuration check.
- `backend.rs:1728-1746`, `checked_arena`, rejects a resolved location whose
  arena device differs or whose checked end exceeds that arena.

The device identifies the arena that could not contain the transfer. The copy is
not queued. An active host submit that receives this value poisons its resource.

### `SlotCapacityExhausted`

`runtime.rs:201-223`, `Runtime::prepare_copy`, reports that the monotonic
`next_slot` index has no preallocated slot (`slots.get(index)` returned `None`).
No token is returned and no new allocation is attempted. Candidate pending
preparation propagates the error and closes the partially prepared runtime.

### `SlotBusy`

The variant means a preallocated backing or job slot cannot be exclusively
claimed now:

- `arena.rs:141-144`, `Arena::close`, because `Arc::try_unwrap` found other
  references to the backing.
- `runtime.rs:201-223`, `Runtime::prepare_copy`, because the slot CAS did not
  observe `SLOT_UNCLAIMED`.
- `runtime.rs:280-287`, `PendingCopy::submit`, because the job mutex is busy or
  already contains a job.

The runtime does not retry. A failed active submit poisons its host resource;
an explicit arena close or preparation call returns the error directly.

### `InvalidPendingState`

This variant is used for a pending token or slot that is not in the state
required by the operation:

- `runtime.rs:272-279`, `PendingCopy::submit`, slot is not `SLOT_PREPARED`.
- `runtime.rs:302-312`, `PendingCopy::poll`, slot is `SLOT_UNCLAIMED` or
  `SLOT_PREPARED`.
- `runtime.rs:315-324`, `PendingCopy::reset`, slot is not `SLOT_COMPLETE`.
- `runtime.rs:429-436`, `execute_slot`, a queued slot has no job to take.

The host backend also propagates this value from `reset` during loop rearming or
pending recycling. No state is advanced by a rejected operation. As with other
active submit or poll errors, the enclosing `HostResources` method poisons its
resource where applicable.

### `WorkerFailed { operation: &'static str, kind: io::ErrorKind }`

Only the asynchronous runtime's worker disk helpers construct this variant:

- `runtime.rs:558-577`, `read_exact_at`, operation `"disk read"`, preserving a
  `read_at` error kind or using `UnexpectedEof` when the file returns zero bytes.
- `runtime.rs:580-598`, `write_all_at`, operation `"disk write"`, preserving a
  `write_at` error kind or using `WriteZero` when the file returns zero bytes.

The worker stores the full `Error` in the slot's failure mutex and publishes
`SLOT_FAILED`. `PendingCopy::poll` clones and returns that value. A normal worker
I/O failure does not set the runtime's shared poison flag, but
`HostResources::poll_pending` sets its resource poison bit after receiving it.

### `Io { operation: &'static str, kind: io::ErrorKind }`

`Io` is the synchronous host I/O category. Constructors and operation strings
are:

- `arena.rs:196-222`, `create_allocated_file`: `error::io("create disk arena", ...)`,
  direct `"allocate disk extent"`, and `error::io("sync disk allocation", ...)`.
- `arena.rs:225-232`, explicit disk close: `error::io("sync disk file", ...)`
  and `error::io("remove disk file", ...)`.
- `arena.rs:250-291`, exact disk arena reads and writes: `"read host arena"`
  and `"write host arena"`, with `UnexpectedEof` and `WriteZero` for zero-byte
  OS results.
- `backend.rs:1355-1361`, `/proc/meminfo` read: `"read /proc/meminfo"`.
- `backend.rs:1375-1382`, `statvfs`: `"query disk capacity"`.

The original `io::Error` is not retained. File creation and extent allocation
clean up the just-created path before returning. Explicit close reports the
failure; automatic `Backing::drop` suppresses its cleanup errors.

### `ThreadSpawn(io::ErrorKind)`

`runtime.rs:164-178`, `Runtime::new`, constructs this variant when a configured
worker thread cannot be spawned. It sets the shared stopping flag, wakes workers,
joins every worker already started, and returns the spawn kind. No partially
constructed `Runtime` is returned.

### `ThreadPanicked`

`runtime.rs:225-240`, `Runtime::shutdown`, sets this variant when any worker join
returns an error. Shutdown continues joining the remaining workers and sets the
runtime state to `Closed` even when the result is an error. `Drop` discards this
result. The backend poll classifier treats it as a failed poll status.

### `Poisoned`

`Poisoned` is the terminal safety result for a lock, worker, resource, or runtime
state that can no longer be trusted. Current constructors are:

- `arena.rs:156-181`, RAM backing mutex lock failure during exact read/write.
- `runtime.rs:194-218`, a runtime whose shared poison flag is set while its
  public state is otherwise `Ready`.
- `runtime.rs:272-311`, shared poison, poisoned job/failure mutexes, and an
  unknown slot state while submitting or polling.
- `runtime.rs:315-324` and `429-436`, poisoned job/failure mutexes during reset
  or execution.
- `runtime.rs:374-425`, worker-loop wake or failure mutex poisoning sets the
  shared flag; the next public observation returns `Poisoned`.
- `backend.rs:676-681`, `HostResources::ensure_healthy` after its own poison
  bit was set by a submit or poll error.

Once `HostResources::poisoned` is true, every operation guarded by
`ensure_healthy` fails without touching arenas or pending tokens. A runtime's
shared poison instead changes `Runtime::state()` to `RuntimeState::Poisoned` and
blocks new pending-copy preparation and submission. `Poisoned` is classified as
`PhysicalPollStatus::Failed` at the host backend poll boundary.
