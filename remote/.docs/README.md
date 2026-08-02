# `recipe-remote`

`recipe-remote` is the bounded, already-connected master/worker execution
protocol for one finalized Recipe program. It turns a `ProvisionedProgram`
derived from a `recipe_core::FinalizedBundle` and measured `Topology` into a
typed session that admits one worker's packed device images, dispatches the
worker portion of `init`, `loop`, and `exit`, carries cross-machine data, and
closes with exact arena release acknowledgments.

The crate is deliberately a protocol layer, not a connection manager or an
executor. It does not listen, open a socket, resolve an address, discover
hardware, compile or load artifacts, read files, choose a route, or calculate
on the CPU. The caller supplies an already connected
`recipe_transport::RuntimeChannel`, the exact local and remote endpoint
identities, the same finalized `ProvisionedProgram` on both peers, and a
`WorkerDriver` implementation on the worker.

## Position in the workspace

`remote/Cargo.toml` defines package `recipe-remote`, Rust 2024, MIT licensed.
Its direct dependencies are:

| Dependency | Boundary used by `recipe-remote` |
| --- | --- |
| `recipe-core` | Finalized bundle, topology, task, device, transfer, digest, phase, and ID contracts. |
| `recipe-executor` | Worker projection and the prepared native worker execution session used by `ExecutorWorkerDriver`. |
| `recipe-transport` | Already connected nonblocking channel, endpoint identities, fixed lanes, completion tokens, and schedule stamps. |
| `sha2` | Canonical manifest/program identities and streamed init-image checksums. |

The root crate includes the package in the workspace and re-exports it as
`recipe::engine::remote` from `src/facade.rs`. There are no other in-tree
callers of a remote session. A process that uses this crate therefore creates
the transport and provisions the program outside this crate, then drives the
public typestate handles below.

The crate root (`src/lib.rs`) forbids unsafe code and denies missing `Debug`
implementations. All implementation modules are private; the root re-exports
only the protocol data, errors, worker-driver boundary, executor adapter, and
session typestates.

## Module graph

```text
recipe_core::FinalizedBundle + Topology
                |
                v
          model.rs  ---- canonical Manifest and ProvisionedProgram
                |
                +---- codec.rs  ---- bounded recipe-remote payloads
                |
recipe_transport::RuntimeChannel
                |
                v
          session.rs  ---- master and worker typestate state machines
                |
                +---- driver.rs  ---- closed WorkerDriver ABI
                |
                +---- executor_driver.rs  ---- recipe-executor adapter
                |
                +---- error.rs  ---- one protocol error vocabulary
```

The dependency direction is one way. `model` owns the static facts consumed by
the session. `codec` only serializes those facts and live commands. `session`
owns all protocol state, fixed storage, and transitions. `driver` describes
what a worker may ask a native executor to do. `executor_driver` implements
that boundary over `recipe_executor::WorkerExecutionSession`.

The root API is intentionally the union of those boundaries:

| Module | Re-exported API |
| --- | --- |
| `model` | `Capabilities`, `CrossTransfer`, `DataDirection`, `DuplexClaim`, `Manifest`, `ManifestArtifact`, `ManifestLimits`, `ProvisionedProgram`, `RemoteIdentity`, `RemoteLimits`, `RuntimeCapacities`, `RuntimeSlots`. |
| `driver` | `DriverFault`, `DriverFaultCode`, `DriverPoll`, `DriverTransferPoll`, `WorkerDriver`. |
| `executor_driver` | `ExecutorDriverBuildError`, `ExecutorWorkerDriver`. |
| `session` | `Advance`, `CancelReason`, `MasterHandshake`, `MasterInit`, `MasterRun`, `MasterExit`, `MasterCancelling`, `MasterComplete`, `WorkerHandshake`, `WorkerInit`, `WorkerRun`, `WorkerExit`, `WorkerCancelled`, `WorkerComplete`, all progress/event types, `MetricSample`, `RemoteChannel`, and `RemoteMetricValue`. |
| `error` | `RemoteError` and `RemoteResult`. |

## Static inputs and identity boundary

### `RemoteIdentity`

`RemoteIdentity::new(local, remote)` wraps two transport
`EndpointIdentity` values. It rejects endpoints whose machine digests are the
same. `local`, `remote`, and `reversed` expose the two directions. Session
construction later checks that these values exactly equal the identities bound
to the supplied `RuntimeChannel`; a caller cannot use a channel for a
different endpoint pair.

### `RemoteLimits`

`ManifestLimits::new(max_artifacts, max_tasks, max_devices, max_transfers)`
and `RuntimeSlots::new(task_slots, data_slots, metric_slots)` reject every
zero capacity. `RemoteLimits::new(max_message_bytes, manifest, runtime)` also
requires at least 256 bytes and requires the size to fit the wire `u32` length
fields. The limits are immutable after construction and are sent in the hello
proof as eight little-endian `u64` values, in this order:

```text
max_message_bytes,
max_artifacts,
max_tasks,
max_devices,
max_transfers,
task_slots,
data_slots,
metric_slots
```

The two peers must advertise exactly the same tuple. `max_message_bytes`
controls the remote codec scratch buffer and every bounded payload. The three
slot values bound submitted worker tasks, master data inbox entries, and
master metric entries. They are protocol capacities, not tuning hints.

### Capabilities

`Capabilities` is a private-bit-set wrapper with `from_bits`, `bits`,
`contains`, and `BitOr`. The required set sent by this protocol is:

| Capability | Meaning |
| --- | --- |
| `CHUNKED_INIT` | Device images may be admitted as ordered bounded chunks. |
| `BIDIRECTIONAL_DATA` | Master-to-worker ingress and worker-to-master egress are both valid. |
| `PREPARED_EXECUTION` | The worker must bind and pre-realize its executor before init. |
| `METRICS` | Worker task metrics may be sent on the metrics lane. |
| `CANCELLATION` | A master may enter the cancellation cleanup path. |
| `EXACT_RELEASE_ACK` | Every worker arena has one release request and one acknowledgment. |

Unknown extra bits are tolerated, but a peer lacking any required bit fails the
handshake.

## Manifest and provisioned program

### `Manifest`

`Manifest::from_bundle(bundle, limits)` admits only a bounded artifact list.
It sorts artifact IDs, rejects duplicate IDs, and stores only artifact IDs and
their digests. The manifest also records:

* protocol version `PROTOCOL_VERSION` (currently `2`);
* finalized bundle identity digest;
* finalized draft digest;
* finalized realization digest; and
* a canonical SHA-256 digest over the domain string
  `recipe-remote-manifest-v1`, all identities, and sorted artifact pairs.

`Manifest::validate` requires the current protocol, nonzero plan identities,
strictly increasing artifact IDs, nonzero artifact digests, and a digest that
recomputes exactly. The encoded manifest size is checked before construction
and before sending, so artifact metadata cannot exceed either the manifest
limit or the message limit. The wire manifest carries the artifact bytes as a
borrowed slice and never carries artifact contents.

### `ProvisionedProgram::from_bundle`

`ProvisionedProgram` is the immutable remote view of one worker assignment. Its
private contents are:

* the validated `Manifest`;
* a program digest;
* sorted worker task records containing `TaskId`, `RunPhase`, and either
  `Driver` or `CrossTransfer` kind;
* sorted worker device records containing `DeviceId` and exact init-image
  byte count;
* cross-machine transfer contracts; and
* the unique half-duplex capacity resources referenced by those transfers.

Construction performs all of the following checks before a session exists:

1. The bundle topology identity equals the supplied topology identity, and the
   topology passes both structural and scheduling-property validation.
2. The worker device list is nonempty, no larger than `max_devices`, sorted,
   nonzero, and duplicate-free. Every device has a finalized init image.
3. Every bundle task ID is nonzero. Calculations are worker driver work only
   when their finalized device belongs to this worker. Metrics are classified
   by the finalized device holding the metric value.
4. Transfers are classified from finalized endpoints. External-to-worker init
   admission is represented by the device image and is omitted from runtime
   tasks. Worker-to-worker transfers remain worker driver work. Master-owned
   transfers are omitted. A transfer crossing the boundary becomes a
   `CrossTransfer` with a direction and exact byte count.
5. No worker driver or cross-boundary task may remain in `RunPhase::Init`; init
   work is the one packed image per worker device.
6. Every cross-boundary transfer is a planner-expanded one-hop transfer. Its
   route has exactly one measured link, the link endpoints match the finalized
   device endpoints, and its link lane claim names exactly that link. External
   lane claims are rejected here.
7. Cross transfers are sorted by direction, schedule-window start, and task
   ID. Init chunk positions are reserved first. Master-to-worker user data
   starts after those init positions, while worker-to-master data starts at
   logical position zero. Each direction has strictly increasing schedules,
   and no schedule may equal the transport unscheduled sentinel `u64::MAX`.
8. Transfer IDs, byte counts, and duplex claims are checked. Byte counts are
   nonzero, claim resources are nonzero and unique within a transfer, and a
   half-duplex resource appears in one fixed token table. User data must fit
   in `max_message_bytes - USER_DATA_OVERHEAD`.
9. Task, device, transfer, and artifact counts remain within the configured
   limits.

The program digest is SHA-256 over `recipe-remote-program-v1`, the manifest
digest, canonical device records, canonical task records, every transfer's
direction/schedule/byte count, and every duplex claim. `manifest()`, `digest()`,
`worker_devices()`, and `transfers()` expose the stable public view. The
handshake compares this digest and the manifest identities, while the worker
executor adapter additionally compares every projected task and transfer
record.

### Duplex and capacity semantics

`DataDirection` has `MasterToWorker` and `WorkerToMaster`. A `DuplexClaim`
binds one measured `DuplexResourceId` to `DuplexMode::Half` or `Full`.
Full-duplex claims never consume a protocol token. Every half-duplex resource
gets one fixed token in each peer's storage. A cross transfer must acquire all
of its half-duplex tokens before submission and releases the same tokens only
when the transfer reaches its protocol completion state. The acquisition is
all-or-nothing, so a failed submission cannot leak a token.

`RuntimeCapacities` reports the effective task, data, metric, scratch, and
half-duplex-token capacities. The master scratch is its codec buffer. The
worker reports its codec, ingress, and egress scratch combined; its data slot
capacity is one because only one incoming and one outgoing transfer state is
held by the worker runtime.

## Wire codec

`codec.rs` is a manual, little-endian, allocation-free-after-construction
codec. Every remote payload begins with:

```text
8 bytes  magic: RCPREM01
2 bytes  remote protocol version
1 byte   message tag
1 byte   reserved, must be zero
8 bytes  per-lane sequence
8 bytes  nonzero RunId
```

The remote header is 28 bytes. `decode` rejects payloads above the configured
limit, a wrong magic or version, nonzero reserved byte, unknown tags, truncated
fixed-width fields, length overflow, unknown metric scalar types, and trailing
bytes. Manifest and user-data lengths are checked before slices are taken.
`Writer` and `Reader` operate on caller-owned buffers and return a bounded
codec or capacity error instead of growing storage.

The message tag and transport lane map is:

| Tag | Message | Lane | Direction and use |
| ---: | --- | --- | --- |
| 1 | `Hello` | Control | Symmetric role, capabilities, identities, and limits. |
| 2 | `Manifest` | Control | Master proves bundle, plan, artifact, and program identities. |
| 3 | `ManifestAck` | Control | Worker acknowledges the exact manifest and program digest. |
| 4 | `Prepare` | Control | Master asks the worker to bind and pre-realize the executor. |
| 5 | `PrepareAck` | Control | Worker has prepared successfully. |
| 6 | `InitBegin` | Control | Begin one device image with byte count and SHA-256 digest. |
| 7 | `InitChunk` | UserData | Ordered image bytes with a static schedule position. |
| 8 | `InitEnd` | Control | Declare that the complete logical image has arrived. |
| 9 | `InitAck` | Control | Worker completed final image admission for one device. |
| 10 | `InitComplete` | Control | Master has received every device image acknowledgment. |
| 11 | `InitCompleteAck` | Control | Worker is ready for runtime task dispatch. |
| 12 | `Execute` | Control | Dispatch one provisioned driver task. |
| 13 | `TaskComplete` | Control | Report a driver or transfer task completion. |
| 14 | `TaskFailed` | Control | Report a poll-time task fault without terminating the session. |
| 15 | `DataRequest` | Control | Request one worker-to-master transfer. |
| 16 | `UserData` | UserData | Master ingress or worker egress bytes with static schedule. |
| 17 | `DataAck` | Control | Master accepted worker egress bytes. |
| 18 | `Metric` | Metrics | One f32 or i32 scalar for a completed worker task. |
| 19 | `Cancel` | Control | Enter cancellation with an opaque `CancelReason` code. |
| 20 | `CancelAck` | Control | Worker completed cancellation cleanup and release acknowledgments. |
| 21 | `BeginExit` | Control | Enter the worker's exit phase after loop completion. |
| 22 | `ExitReady` | Control | Worker accepted exit and called `begin_exit`. |
| 23 | `Release` | Control | Release one exact worker arena. |
| 24 | `ReleaseAck` | Control | A requested arena has been released. |
| 25 | `ExitComplete` | Control | All exit tasks and arena releases are complete. |
| 26 | `ExitAck` | Control | Worker finished native teardown. |
| 27 | `DriverFault` | Control | Terminal worker driver fault after local cleanup. |

`encode_hello` always advertises `Capabilities::REQUIRED`. `encode_manifest`
emits the full sorted artifact proof. `WireManifest::matches_proof` checks the
bundle, draft, realization, manifest digest, program digest, artifact count,
strict artifact ordering, nonzero artifact digests, and the canonical manifest
hash before the worker accepts it.

## Transport integration and session core

`RemoteChannel` owns a `RuntimeChannel` plus the last run ID used on that
connection. It can be built from a `RuntimeChannel` or passed directly to
either handshake. A completed master returns it through
`MasterComplete::into_channel`; a completed worker returns it together with
the driver through `WorkerComplete::into_parts`. Reusing the wrapper preserves
strictly increasing run IDs and lets `SessionCore` translate each new run's
logical user-data schedules onto the transport's connection-global schedule
positions.

`SessionCore::new` validates the nonzero run ID, manifest, init schedule, exact
transport identity, and strict run increase. It obtains outbound and inbound
schedule bases from `RuntimeChannel`, allocates one codec scratch buffer, and
initializes independent transmit and receive sequence counters for Control,
Metrics, and UserData.

Every `progress` implementation first calls `RuntimeChannel::progress`. A
transmitted completion token is released before another send can reuse that
fixed slot. At most one received transport frame is held at a time; after
decoding and checking it, the session releases the matching received token.
Transport capacity exhaustion is surfaced as a nonfatal `Backpressured` result
from a public send operation or as `false` from an internal `try_send`, so the
caller must progress and try the same operation again. Other transport errors,
codec errors, wrong runs, wrong lanes, and replayed or out-of-order per-lane
sequences poison the session. A poisoned session rejects all later progress.

The transport itself remains responsible for TCP framing, endpoint/profile
identity headers, payload integrity, nonblocking reads and writes, and fixed
lane slot ownership. `recipe-remote` adds its own payload magic, protocol
version, run epoch, per-lane sequence, static schedule translation, and
message state machine on top.

## Public typestate lifecycle

The only legal lifecycle is:

```text
MasterHandshake -> MasterInit -> MasterRun -> MasterExit -> MasterComplete
       |                |            |        \-> MasterCancelling
       |                |            +----------> MasterCancelling
       |                +----------------------> error
       +--------------------------------------> error

WorkerHandshake<D> -> WorkerInit<D> -> WorkerRun<D>
                                      |       |
                                      |       +-> WorkerExit<D> -> WorkerComplete<D>
                                      +----------> WorkerCancelled<D> -> WorkerComplete<D>
```

`Advance<P, N>` is either `Pending(P)` or `Ready(N)`. Master states keep their
owned session in place and expose mutable progress. Worker states consume the
current handle and return it in a progress enum, which prevents using a stale
worker phase after a transition.

### Symmetric handshake

`MasterHandshake::new(channel, identity, limits, program, run)` and
`WorkerHandshake::new(..., driver)` perform static construction only. Their
`progress` methods drive this order:

1. Both peers send `Hello` and validate the opposite role, required
   capabilities, reversed endpoint/profile identities, and exact limits.
2. The master sends `Manifest`. The worker validates the complete proof against
   its own program and sends `ManifestAck`.
3. The master checks the acknowledgment digests and sends `Prepare`.
4. The worker calls `WorkerDriver::prepare`. A successful worker sends
   `PrepareAck`, yielding `WorkerInit`; the master receives that acknowledgment
   and yields `MasterInit`.

Any duplicate, missing, wrong-role, or out-of-order handshake message poisons
the session. A worker driver error enters the terminal driver-fault path before
the master can observe it.

### Init image admission

The master owns one `MasterImage` at a time. `MasterInit::queue_image(device,
bytes)` requires a known device that is still `Needed`, and the byte length must
equal the finalized image contract. It computes SHA-256 over the caller-owned
boxed image, then `progress` sends:

```text
InitBegin(device, bytes, digest)
InitChunk(device, offset, chunk) ...
InitEnd(device, bytes)
```

Chunks use the UserData lane and consume the reserved init schedule positions.
The next chunk is not sent until the transport completion token for the prior
chunk is transmitted. The worker requires one active image, exact contiguous
offsets, exact static schedule positions, and chunks no larger than its fixed
buffer. It copies each chunk into stable storage before calling
`begin_init_chunk`, polls the driver, hashes the bytes only after completion,
and zeroes the temporary chunk buffer.

At `InitEnd`, the worker checks the final byte count and the streamed digest,
then calls `finish_init_image` and polls the final admission. It sends exactly
one `InitAck` per device. The master marks that device complete. After every
device acknowledgment, it sends `InitComplete`; the worker accepts that message
only when no image operation or acknowledgment is pending and every device is
complete, then sends `InitCompleteAck`. Only then do both peers enter runtime.

Init admission is intentionally not a general file or artifact path. The
caller supplies bytes, and the worker driver owns the native allocation and
copy. Any worker error in image begin, chunk begin/poll, or image finish is a
terminal driver fault.

### Loop execution

`MasterRun` exposes `submit_task`, `send_user_data`, `request_user_data`,
`progress`, `capacities`, `data`, `release_data`, `take_metric`, `begin_exit`,
and `cancel`.

#### Driver tasks

`submit_task(task)` accepts only an idle worker `ProgramTaskKind::Driver` in
`RunPhase::Loop`. It sends `Execute` on Control and marks the task active. The
worker validates the phase, role, and idle status, consumes one fixed task slot,
and calls `WorkerDriver::submit_task`.

The worker polls active native tasks round-robin. A completed task may carry one
f32 or i32 metric. If present, the metric is sent on the Metrics lane and the
worker waits for that transmission to flush before sending `TaskComplete` on
Control. The master stores the metric in a bounded metric slot and emits
`MasterRunEvent::MetricReady`; the caller retrieves it with `take_metric`. A
poll-time driver error becomes `TaskFailed` for that task, which the master
reports as `MasterRunEvent::TaskFailed` and marks failed. Submission or other
driver errors that occur while handling a command enter the terminal fault
path instead.

The remote layer does not calculate dependencies or choose schedules. Those
facts are already in the finalized program and are independently enforced by
the worker executor session and its projection. The remote layer only admits
the exact task ID, phase, role, and one-shot status that the provisioned
program allows.

#### Master-to-worker data

`send_user_data(task, bytes)` accepts only an idle cross transfer whose static
direction is `MasterToWorker`, phase matches the handle, and byte count matches
exactly. It acquires all half-duplex claims, sends `UserData` on the UserData
lane with the transfer's static schedule, then marks the task active. The
worker validates direction, schedule, byte count, phase, and status, copies the
payload into a fixed ingress buffer, and calls
`begin_receive_user_data` followed by nonblocking polls. On completion it emits
`DataAccepted`, sends one `TaskComplete`, and releases the transfer token. The
master receives the task completion and releases its matching token.

#### Worker-to-master data

`request_user_data(task)` accepts only an idle `WorkerToMaster` cross transfer.
The master permits at most one active worker-to-master request at a time,
acquires its half-duplex claims, and sends `DataRequest` on Control. The worker
calls `begin_produce_user_data` into fixed egress scratch, polls it, and sends
`UserData` with the static worker-to-master schedule. It then waits in
`WaitAck`.

The master validates the task, direction, schedule, phase, and byte count,
copies the payload into a bounded data slot, marks the task
`ReceivedAwaitAck`, and returns `MasterRunEvent::DataReady { task, slot,
bytes }`. The caller reads `data(slot)` and calls `release_data(slot)` when the
inbox storage can be reused. Slot release is independent of the protocol
acknowledgment: the next `progress` sends `DataAck` even if the caller has not
released the slot. The worker
calls `user_data_acked`, zeroes its egress scratch, completes the task, and
releases its half-duplex claims. The master then marks the task complete and
releases its claims. A full data inbox or a full lane is backpressure, not an
implicit allocation or retry.

#### Master progress events

`MasterRunEvent` has four forms:

* `TaskComplete(TaskId)` for a local driver task or either cross-transfer
  direction;
* `TaskFailed { task, fault }` for a poll-time worker task fault;
* `DataReady { task, slot, bytes }` for worker egress data; and
* `MetricReady { slot, sample }` for one bounded metric mailbox entry.

`MasterRun::begin_exit` is accepted only when every loop task is `Complete`.
Failed tasks do not satisfy this requirement. `MasterRun::cancel` consumes the
handle and enters the independent cancellation cleanup path at any point where
the master still owns the runtime. `MasterExit::cancel` exposes the same
cancellation transition after exit work has begun. Handshake and init states do
not expose cancellation because no runtime resources have been admitted yet.

### Exit and exact release

`MasterExit::progress` first sends `BeginExit` and waits for `ExitReady`. The
worker accepts it only after all loop tasks are complete and after
`WorkerDriver::begin_exit` succeeds. The master can then submit the exact
provisioned `RunPhase::Exit` driver and cross-transfer tasks through the same
methods as loop execution, but phase checks now require `Exit`.

When every exit task is complete, the master sends one `Release` per worker
device, in the fixed device order, and waits for its matching `ReleaseAck`
before requesting the next. The worker calls `release_arena` only for a device
still marked `Needed`, sends the acknowledgment after the release succeeds, and
rejects duplicate or out-of-order requests. `ExitComplete` is sent only after
all release acknowledgments have arrived. The worker accepts it only when all
exit tasks and all release states are complete, calls `finish`, and sends
`ExitAck` after the transmission flushes. `MasterExit::is_complete` becomes true
only after that acknowledgment; `into_complete` then yields `MasterComplete`.

`WorkerComplete::into_parts` returns the reusable `RemoteChannel` and driver.
Both complete handles expose `was_cancelled` and `run_id`.

### Cancellation

`MasterCancelling::progress` sends one `Cancel` with the opaque
`CancelReason::code`. While cancellation is in flight it tolerates already
queued task completions, task failures, metrics, data, data acknowledgments,
and `ExitReady`, because those frames may have been submitted before the
cancel. It does not accept a second cancel or an unexpected message. It waits
for every release acknowledgment and then `CancelAck` before yielding a
cancelled `MasterComplete`.

On `Cancel`, the worker calls `WorkerDriver::cancel`, releases every arena one
at a time, waits for each release-ack transmission to flush, calls
`WorkerDriver::finish`, and sends `CancelAck`. A driver error during this path
changes to the terminal fault path. Cancellation therefore has the normal
per-arena release protocol and differs from a fatal driver fault, which has
already quiesced and released resources locally.

## Worker driver contract

`WorkerDriver` is the only native execution surface visible to the protocol.
It is `Debug`, allocation-free in its payloads, and receives only finalized IDs,
digests, phases hidden by the current typestate, and bounded byte slices. It
has no host calculation callback, closure, compiler hook, discovery hook,
filesystem operation, vendor math API, or arbitrary resource lookup.

| Method group | Required behavior |
| --- | --- |
| `prepare(run, program)` | Bind the exact provisioned program and pre-realize local and external execution resources before init. |
| `begin_init_image` / `begin_init_chunk` / `poll_init_chunk` / `finish_init_image` / `poll_init_image` | Admit one exact streamed device image. The matching poll must report the exact byte count. |
| `submit_task` / `poll_task` | Submit and poll one provisioned native task without host-side calculation. Completion may carry one f32 or i32 metric. |
| `begin_receive_user_data` / `poll_receive_user_data` | Consume a stable master-to-worker slice and report its exact byte count. |
| `begin_produce_user_data` / `poll_produce_user_data` / `user_data_acked` | Fill stable caller-owned storage, report the exact egress byte count, and retain it until the master acknowledges it. |
| `cancel` | Stop active work for the requested run. The reason is an opaque protocol code. |
| `begin_exit` / `release_arena` / `finish` | Enter exit, release each exact device arena, then destroy or return all prepared native resources. |
| `cleanup_after_fault(primary)` | Quiesce all native work and release every local resource before the terminal `DriverFault` is sent. |

`DriverPoll` is `Pending` or `Complete { metric }`. `DriverTransferPoll` is
`Pending` or `Complete { bytes }`. The session compares each completion byte
count with the finalized contract and poisons the protocol on a mismatch.

`DriverFault` is a compact `{ code: u32, detail: u64 }` value. Recipe reserves
the `0x5245_0002` through `0x5245_0008` codes for identity, lifecycle,
projection, executor preparation/operation, and cleanup failures. Backend
native faults may use other codes. `DriverFault::cleanup_failed` combines the
primary and cleanup code into `FATAL_CLEANUP_FAILED`.

The terminal fault sequence is strict: the worker calls
`cleanup_after_fault`, combines a cleanup failure with the primary fault if
needed, queues exactly one `DriverFault`, waits until that frame is transmitted,
and then returns `RemoteError::Driver` to its caller. The protocol does not add
per-arena release acknowledgments after this point because successful cleanup
already proves local resource release. A second terminal fault is a protocol
violation.

## `ExecutorWorkerDriver`

`ExecutorWorkerDriver<B>` is the concrete adapter for a
`recipe_executor::WorkerBackend` implementation. Its constructor receives the
same finalized bundle and topology used to build the remote program, a
`WorkerAssignment`, the `ProvisionedProgram`, backend value, and executor
`Watchdog`.

Construction derives `WorkerProjection::derive` and then compares the wire
program with that immutable projection. It rejects:

* bundle identity, worker device count, device IDs, or init-image size
  mismatches;
* a program task count or sorted task identity/phase mismatch;
* a projected local task represented as anything other than `Driver`;
* a projected external ingress or egress represented as anything other than
  `CrossTransfer`;
* init admission leaking into the remote runtime task list; and
* a cross-transfer count, task identity, direction, byte count, route link,
  half/full-duplex mode, or single capacity claim mismatch.

`prepare` requires the exact expected program digest and an idle adapter. It
passes the projection to `WorkerExecutionSession::prepare`, which binds worker
resources, pre-realizes all local and external pending tokens, and records the
prepared executor state before any image is admitted. A preparation failure
returns the backend to the adapter and maps the executor error and any cleanup
error to reserved driver fault codes.

The live adapter methods are direct translations:

| Remote method | Executor call |
| --- | --- |
| Init begin/chunk/finalize/poll | `begin_init_image`, `write_init_chunk`, `submit_init_image`, `poll_init_image`. A successfully written chunk reports completion with the copied byte count. |
| Task submit/poll | `submit_task` and `poll_task` in the active Loop or Exit phase. Executor metric values become `RemoteMetricValue`. |
| Ingress/egress begin and poll | `begin_external_ingress`, `poll_external_ingress`, `begin_external_egress`, `poll_external_egress`. |
| Egress acknowledgment | `acknowledge_external_egress`. |
| Cancel, exit, release, finish | `cancel`, `begin_exit`, `release_arena`, `finish`. |
| Fatal cleanup | `fatal_cleanup`, followed by recovery of the backend only when the executor session is finished. |

The adapter rejects operations without an active run or outside Loop/Exit,
rejects overlapping init chunks, and clears its pending state when a session
finishes. Its `Drop` implementation invokes executor fatal cleanup if a live
session is abandoned. The executor's `WorkerExecutionError` variants are
reduced to stable numeric details, including task/device IDs, lifecycle states,
backend operation categories, watchdog expiration, journal failure, and
capacity overflow. The executor API currently has no reason parameter, so the
remote cancellation code is intentionally not forwarded by the adapter.

## Error and ownership rules

`RemoteError` is `#[non_exhaustive]` and covers:

* `InvalidConfiguration` for impossible local limits, IDs, schedules, or
  channel reuse;
* `ManifestMismatch` for plan, artifact, topology, or image identity failures;
* `Protocol` for wrong phase, role, status, lane, direction, schedule,
  acknowledgment, or message order;
* `RunMismatch`, `UnknownTask`, `DuplicateTask`, `UnknownDevice`, and
  `DuplicateDevice` for identity violations;
* `CapacityExhausted` and `Backpressured` for fixed bounds and full lanes;
* `Codec` for malformed remote payloads;
* `Driver` for terminal worker faults;
* `Transport` for a poisoned or failed `RuntimeChannel`; and
* `Poisoned` after a terminal protocol or transport violation.

The protocol never silently substitutes an alternate plan, route, schedule,
artifact, device, or transport. Every received frame is checked against the
active run, lane, per-lane sequence, static task/transfer contract, and current
typestate before its transport token is released. Every borrowed data slice has
a defined owner and lifetime: master send slices remain stable until transport
submission completion, worker driver slices remain stable until their matching
poll, and master data inbox slots remain occupied until the caller calls
`release_data`.

Steady-state progress reuses the fixed codec and storage boxes allocated during
session construction. There is no per-message heap-backed queue, retry loop,
or hidden socket thread. Callers own the progress loop and decide when to retry
an operation after a reported capacity condition.

## End-to-end use

The real integration boundary is an already connected channel. A caller first
creates matching `EndpointIdentity` and `SessionIdentity` values for the
transport, chooses identical nonzero `RemoteLimits`, finalizes a bundle and
topology, and derives the same `ProvisionedProgram` on both peers. The master
then drives the following shape:

```text
MasterHandshake::new(...).progress()
  -> Pending until Ready(MasterInit)
MasterInit::queue_image(device, bytes) for each worker device
MasterInit::progress()
  -> Pending until Ready(MasterRun)
MasterRun::submit_task / send_user_data / request_user_data
MasterRun::progress() -> MasterRunEvent values
MasterRun::begin_exit()
  -> MasterExit after every loop task is complete
MasterExit::progress()
  -> release acknowledgments, then is_complete()
MasterExit::into_complete() -> MasterComplete
MasterComplete::into_channel() -> reusable RemoteChannel
```

The worker drives its own `WorkerHandshake<D>::progress`, queues no user image
itself, and handles each `WorkerInit::progress` result until `WorkerRun<D>`.
`WorkerRun::progress` returns `Running`, `Exit`, or `Cancelled`; the caller
rebinds the returned handle and continues until `WorkerComplete<D>`. For native
execution, `D` is normally `ExecutorWorkerDriver<B>` with a CUDA, HSA, or
other Recipe-owned `WorkerBackend` implementation. The remote crate never
chooses that backend or opens the connection.

## Validation boundary

The relevant structural check for this crate is:

```text
cargo check -p recipe-remote
```

Formatting is checked through the workspace formatter. A successful compile or
format pass proves only that the public contracts are structurally valid. A
runtime acceptance run must still use a real connected transport, matching
measured identities, a finalized real-data bundle, a real worker backend, and
the complete handshake, init, loop, exit or cancellation, and teardown path.
