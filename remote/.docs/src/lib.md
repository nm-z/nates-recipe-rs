<!--
This document describes the implementation in remote/src. Keep statements
about callers and behavior tied to the current source. The public contract is
the exported Rust API plus the protocol checks performed by the session state
machines.
-->

# `recipe-remote`

`recipe-remote` is Recipe's bounded master/worker runtime on top of an already
connected `recipe_transport::RuntimeChannel`. It carries a finalized,
profile-derived program from one machine to the other, admits the worker's
device images, dispatches provisioned worker tasks, exchanges cross-machine
data and metrics, and closes the run with exact arena-release acknowledgments.
It does not open a socket, choose a peer, discover hardware, compile a kernel,
load an artifact, or perform file I/O. Those operations belong to the caller's
transport, probe, planning, preparation, and native-executor layers.

The crate is intentionally a small protocol boundary. The master owns the
control decisions and user-facing data buffers. The worker owns one local
`WorkerDriver` and never receives a host-calculation callback or an arbitrary
closure. Both sides must be constructed from the same `ProvisionedProgram`,
the same fixed `RemoteLimits`, mirrored endpoint identities, and one nonzero
`RunId`.

## Crate root and integration surface

`remote/src/lib.rs` forbids unsafe code and denies missing `Debug`
implementations. It declares six private implementation modules and re-exports
their public types:

| Module | Root exports | Responsibility |
| --- | --- | --- |
| `codec` | none | Manual bounded wire encoding and decoding. It is private so callers cannot bypass session state checks. |
| `driver` | `DriverFault`, `DriverFaultCode`, `DriverPoll`, `DriverTransferPoll`, `WorkerDriver` | The allocation-free worker-side native-operation boundary. |
| `error` | `RemoteError`, `RemoteResult` | Protocol, configuration, capacity, transport, and driver failures. |
| `executor_driver` | `ExecutorDriverBuildError`, `ExecutorWorkerDriver` | Adapter from `recipe-executor::WorkerExecutionSession` to `WorkerDriver`. |
| `model` | `Capabilities`, `CrossTransfer`, `DataDirection`, `DuplexClaim`, `Manifest`, `ManifestArtifact`, `ManifestLimits`, `ProvisionedProgram`, `RemoteIdentity`, `RemoteLimits`, `RuntimeCapacities`, `RuntimeSlots` | Canonical program proof, identities, fixed limits, and transfer contracts. |
| `session` | `Advance`, `CancelReason`, all master and worker typestates, run events, `MetricSample`, `RemoteChannel`, and metric value types | The nonblocking handshake, init, execution, cancellation, and exit state machines. |

`PROTOCOL_VERSION` is the crate's wire version, currently `2`. The workspace
root depends on `recipe-remote`, and `src/facade.rs` exposes it only as
`recipe::engine::remote`. There are no other current Rust call sites in the
repository: an application or acceptance runner is expected to compose the
exported API with `recipe-transport` and the finalized planning/executor
outputs.

The direct crate dependencies are `recipe-core` for IDs, phases, digests, and
finalized plans, `recipe-transport` for the connected fixed-capacity channel,
`recipe-executor` for the concrete worker adapter, and `sha2` for manifest,
program, and init-image digests.

## Construction boundary and data model

### Identities and capabilities

`RemoteIdentity::new(local, remote)` requires distinct machine identities. It
retains the exact `recipe_transport::EndpointIdentity` values, exposes
`local()` and `remote()`, and provides `reversed()` for constructing the peer
side. `SessionCore::new` compares this value with the identity already bound
to `RuntimeChannel`; a mismatch is an `InvalidConfiguration` error.

`Capabilities` is a bit set. The handshake advertises and requires all six
current capabilities:

* `CHUNKED_INIT`: device images are transferred in bounded chunks.
* `BIDIRECTIONAL_DATA`: master-to-worker and worker-to-master transfers are
  both valid.
* `PREPARED_EXECUTION`: the worker prepares its finalized executor before init
  admission.
* `METRICS`: worker task completion may carry an `f32` or `i32` metric.
* `CANCELLATION`: a run can enter the cancellation release path.
* `EXACT_RELEASE_ACK`: each worker arena has one matching release acknowledgment.

`Capabilities::REQUIRED` is the OR of these bits. Unknown extra bits are
preserved by `from_bits`, while `contains(REQUIRED)` is the handshake test.

### Fixed capacities

`ManifestLimits::new` requires nonzero maxima for artifacts, tasks, devices,
and cross transfers. `RuntimeSlots::new` requires nonzero task, data, and
metric slot counts. `RemoteLimits::new` combines both sets and requires a
message capacity of at least 256 bytes that fits the wire `u32` length field.
The public accessors expose the message size and runtime slot counts; the
manifest maxima remain part of the exact wire tuple exchanged by `Hello`.

`RuntimeCapacities` reports the fixed runtime shape as public fields:
`task_slots`, `data_slots`, `metric_slots`, `scratch_bytes`, and
`half_duplex_tokens`. The master allocates one data slot per configured data
slot. The worker has one inbound/outbound data scratch path, so its reported
`data_slots` is one and its scratch count includes the codec, outgoing, and
incoming buffers.

### Manifest and provisioned program

`Manifest::from_bundle` creates the proof sent during the handshake. It checks
the artifact bound and encoded size, copies artifact IDs and digests into
ascending ID order, rejects duplicate IDs, and records the protocol, bundle,
draft, and realization digests from `FinalizedBundle`. Its digest is SHA-256
over the domain string `recipe-remote-manifest-v1`, those identities, and the
canonical artifact list. `Manifest::validate` repeats the protocol, nonzero
identity, canonical ordering, nonzero artifact digest, and digest-consistency
checks before a session can start.

`ProvisionedProgram::from_bundle(bundle, topology, worker_devices, limits)`
turns the finalized bundle into the exact worker view:

1. It builds and validates the manifest, requires `bundle.topology()` to equal
   the supplied topology identity, and requires the topology to pass both
   topology validation checks.
2. It canonicalizes worker device IDs, requiring a nonempty, bounded,
   nonzero, duplicate-free set. Every selected device must have a finalized
   init image; its ID and image byte count become a device contract.
3. It classifies each finalized task. Calculations and metrics located on a
   worker device become `Driver` tasks. A one-hop transfer crossing the
   master/worker boundary becomes a `CrossTransfer`. Master-local work is not
   sent. An init external-to-worker transfer is represented by the one device
   image, not by an init task.
4. It rejects init worker work and init cross transfers, transfers that are not
   planner-expanded one-hop tasks, endpoints that are not devices, route/link
   mismatches, and lane claims that do not match the measured topology link.
5. It assigns separate strictly increasing schedules to master-to-worker and
   worker-to-master transfers. Master-to-worker positions begin after all
   reserved init-chunk positions. Each transfer records its byte count,
   direction, and one or more sorted unique `DuplexClaim` values. Half-duplex
   resources are collected into fixed tokens; full-duplex resources do not
   require a shared token.
6. It sorts task IDs, enforces task and transfer limits, and computes the
   program digest over the manifest, devices, task phase/kind, transfer
   schedules/contracts, and claim modes using the domain string
   `recipe-remote-program-v1`.

The public accessors expose the manifest, program digest, worker device IDs,
and cross-transfer contracts. Internal task, device, and half-duplex lists stay
private so callers cannot alter the proof after construction.

`CrossTransfer` itself enforces a nonzero task ID, finite schedule (the
transport's `u64::MAX` sentinel is reserved), nonzero bytes, nonzero unique
capacity resources, and canonical claim ordering. `DataDirection` distinguishes
`MasterToWorker` from `WorkerToMaster`; `DuplexClaim` records the finalized
resource and `DuplexMode` (`Half` or `Full`).

## Wire codec and transport calls

`remote/src/codec.rs` is a private, manual codec. It writes into the one
preallocated session scratch buffer and decodes by borrowing the transport's
received frame. The fixed remote header is:

```text
RCPREM01 | version:u16 | tag:u8 | reserved:u8 |
sequence:u64 | run:u64 | message payload
```

The reserved byte must be zero. Every payload is checked against
`RemoteLimits::max_message_bytes()`, and decoding rejects unknown tags,
truncated fields, unknown scalar kinds, out-of-bound artifact counts, and any
trailing bytes. The codec is allocation-free after construction.

Message tags are private, but their categories are the observable protocol:

| Tags | Messages | Lane |
| --- | --- | --- |
| 1-5 | `Hello`, `Manifest`, `ManifestAck`, `Prepare`, `PrepareAck` | Control |
| 6-11 | `InitBegin`, `InitChunk`, `InitEnd`, `InitAck`, `InitComplete`, `InitCompleteAck` | `InitChunk` uses UserData; the rest use Control |
| 12-18 | `Execute`, `TaskComplete`, `TaskFailed`, `DataRequest`, `UserData`, `DataAck`, `Metric` | `UserData` uses UserData, `Metric` uses Metrics, the rest use Control |
| 19-20 | `Cancel`, `CancelAck` | Control |
| 21-27 | `BeginExit`, `ExitReady`, `Release`, `ReleaseAck`, `ExitComplete`, `ExitAck`, `DriverFault` | Control |

`SessionCore` drives message encoding and decoding. The model layer also uses
the codec's private manifest-size helper. SessionCore calls the channel's
`submit_control`, `submit_metrics`, or `submit_user_data` methods and retains
the returned `CompletionToken`. User-data messages require their static
schedule. The channel's connection-global schedule base is captured at session
construction, so a reused channel can run again without reusing old positions.

There are three independent remote sequence counters, one per runtime lane.
On receive, `SessionCore` checks the transport frame kind, decodes the remote
header, requires the exact current `RunId`, requires that the message's lane
matches the frame lane, and requires the next per-lane sequence. It translates
the transport schedule back to the run-relative schedule. Any malformed,
wrong-run, wrong-lane, replayed, out-of-order, or probe frame poisons the
session. A poisoned session returns `RemoteError::Poisoned` on subsequent
operations. Received and transmitted transport storage is released explicitly
through the matching completion token.

## Typestate lifecycle

The public states make the legal lifecycle explicit:

```text
MasterHandshake  -> MasterInit -> MasterRun -> MasterExit -> MasterComplete
                                         \-> MasterCancelling -> MasterComplete

WorkerHandshake  -> WorkerInit -> WorkerRun -> WorkerExit -> WorkerComplete
                                         \-> WorkerCancelled -> WorkerComplete
```

The progress APIs are nonblocking. Handshake, init, worker run, worker exit,
and worker cancellation consume the current state and return
`Advance::Pending(state)` or `Advance::Ready(next_state)`, or the corresponding
worker progress enum. Master run and master exit retain `&mut self` so a caller
can submit work and inspect events between progress calls.

### Handshake

`MasterHandshake::new` and `WorkerHandshake::new` call `SessionCore::new`,
which validates the nonzero run, manifest, init schedule reservation,
transport identity, and strict increase over the prior run on a reused
`RemoteChannel`. Each side allocates fixed task, data, metric, half-duplex,
and release storage.

Each `progress` call advances transport completions, consumes at most one frame,
and drives the next send:

1. Both sides send `Hello` with their role, required capability bits, mirrored
   machine/profile digests, and the complete fixed limit tuple. The peer role,
   identities, capabilities, and limits must match exactly.
2. The master sends the canonical manifest and program digest. The worker
   recomputes the manifest proof from its own provisioned program and sends a
   matching `ManifestAck`.
3. The master sends `Prepare`. The worker invokes `WorkerDriver::prepare` on
   its finalized program, then sends `PrepareAck` only after preparation
   succeeds.

A worker driver failure enters the terminal driver-fault path. The worker first
   calls `cleanup_after_fault`, combines a cleanup failure with the primary
   fault when necessary, sends one `DriverFault` frame, and returns the fault
   only after that frame is transmitted. The master turns the received fault
   into `RemoteError::Driver` and poisons the session.

### Init image admission

`MasterInit::queue_image` accepts one worker device image at a time. The device
must be provisioned and still `Needed`, and the boxed byte length must exactly
match the finalized arena size. The master hashes the image with SHA-256.

`MasterInit::progress` sends `InitBegin`, then chunks of at most
`max_message_bytes - 48` bytes. Each chunk uses the next reserved user-data
schedule and the master waits for its transmit completion before advancing the
offset. It sends `InitEnd`, waits for the matching `InitAck`, and marks the
device complete. After every device is complete it sends `InitComplete` and
waits for `InitCompleteAck` before returning `MasterRun`.

`WorkerInit::progress` permits exactly one active image and one pending chunk or
final admission. It checks the device ID, image size, sequential offset, and
reserved schedule. It copies each chunk into a bounded scratch buffer before
calling `begin_init_chunk`, polls the driver, hashes only the completed bytes,
and clears the scratch bytes. `InitEnd` requires the exact byte count and digest;
the worker then calls `finish_init_image` and polls `poll_init_image`. A single
`InitAck` is sent for each completed device. `InitComplete` is accepted only
when every image is complete and no admission is pending, then the worker sends
`InitCompleteAck` and becomes `WorkerRun`.

### Master run and worker run

`MasterRun` exposes:

* `submit_task(task)`: sends `Execute` only for an idle provisioned worker
  driver task in the loop phase.
* `send_user_data(task, bytes)`: sends an exact-size master-to-worker transfer
  at its finalized schedule, acquiring any half-duplex token first.
* `request_user_data(task)`: requests one idle worker-to-master transfer at a
  time, also respecting half-duplex tokens.
* `progress()`: advances transport and returns one `MasterRunEvent`, or `None`.
* `data`, `release_data`, and `take_metric`: inspect and release fixed inbox
  slots. A received worker-to-master payload occupies its slot until
  `release_data`; `progress` separately flushes the `DataAck` and marks the
  transfer complete.
* `capacities()`: reports the fixed runtime shape.

`MasterRunEvent` is `TaskComplete`, `TaskFailed { task, fault }`,
`DataReady { task, slot, bytes }`, or `MetricReady { slot, sample }`. Driver
tasks release their transfer token on completion or failure. Cross transfers
release half-duplex tokens when their completion acknowledgment is complete.
Metrics remain in fixed metric slots until `take_metric` removes them.

`WorkerRun::progress` returns `WorkerRunProgress::Running { session, event }`,
`Exit(WorkerExit)`, or `Cancelled(WorkerCancelled)`. Its worker events are:
`TaskAccepted`, `TaskReported`, `DataAccepted`, and `DataAcknowledged`.
`WorkerRuntime` polls active driver tasks round-robin through fixed task slots.
It sends a metric before `TaskComplete` when the driver supplied one, reports a
`TaskFailed` frame for a task fault, and never performs host-side calculation.

For a master-to-worker transfer, the worker validates the static direction,
schedule, phase, task, and byte count, copies the payload into bounded scratch,
calls `begin_receive_user_data`, polls it, and then reports the task complete.
For a worker-to-master transfer, `DataRequest` allocates the one outgoing
scratch contract, calls `begin_produce_user_data`, polls it, sends the payload
at the finalized schedule, waits for `DataAck`, calls `user_data_acked`, clears
the scratch bytes, and reports `DataAcknowledged`.

`MasterRun::begin_exit` is legal only after every loop-phase task is complete.
`MasterRun::cancel` enters the independent cancellation path at any point where
the caller still owns the run state.

### Normal exit

`MasterExit` starts in `NeedBegin` and sends `BeginExit`. The worker checks that
all loop work is complete, calls `WorkerDriver::begin_exit`, and sends
`ExitReady`. Only then does the master enter the active exit phase. The same
task and data methods are available for `RunPhase::Exit`, but only while the
exit state is active.

After all exit tasks and pending data acknowledgments complete, the master sends
one `Release { device }` at a time. It waits for the exact `ReleaseAck` before
requesting the next device. Once every release is acknowledged it sends
`ExitComplete` and waits for `ExitAck`; `is_complete()` becomes true and
`into_complete()` yields `MasterComplete`.

`WorkerExit` sends `ExitReady`, continues polling exit work, and handles each
`Release` only after all exit tasks are complete. It calls
`release_arena(device)`, sends the matching `ReleaseAck`, and records the
release as complete. `ExitComplete` is accepted only after every arena release
and exit task is complete. The worker then calls `finish`, sends `ExitAck`, and
returns `WorkerExitProgress::Complete(WorkerComplete)` after the acknowledgment
is flushed.

`MasterComplete` exposes `was_cancelled()`, `run_id()`, and `into_channel()`.
The returned `RemoteChannel` retains the run epoch and transport schedule
positions, so it can be supplied to a later handshake with a strictly larger
run ID.

### Cancellation

`MasterCancelling::progress` sends `Cancel { reason }`, accepts in-flight task,
metric, and data frames while cancellation is being established, then waits
for one `ReleaseAck` per worker arena followed by `CancelAck`. It returns
`MasterComplete { cancelled: true }` only after all releases and the terminal
acknowledgment are complete.

On `Cancel`, the worker calls `WorkerDriver::cancel` and enters
`WorkerCancelled`. That state releases each arena, waits for the corresponding
release transmission to flush, calls `finish`, sends `CancelAck`, and returns
`WorkerComplete { cancelled: true }`. A driver failure at any point takes the
same cleanup-and-single-terminal-fault path as normal execution.

## `WorkerDriver` and executor integration

`WorkerDriver` is the only worker-side execution capability required by the
session state machine. Its methods are deliberately narrow and operate only on
finalized IDs, digests, and bounded byte slices:

| Group | Methods | Contract used by the session |
| --- | --- | --- |
| Preparation and init | `prepare`, `begin_init_image`, `begin_init_chunk`, `poll_init_chunk`, `finish_init_image`, `poll_init_image` | Bind the exact program, admit one checksummed image, and report nonblocking transfer completion. |
| Local tasks | `submit_task`, `poll_task` | Submit a provisioned GPU-native task and optionally return one `f32` or `i32` metric. |
| Cross-machine data | `begin_receive_user_data`, `poll_receive_user_data`, `begin_produce_user_data`, `poll_produce_user_data`, `user_data_acked` | Keep source/destination storage stable until the matching poll or acknowledgment. |
| Lifecycle | `cancel`, `begin_exit`, `release_arena`, `finish` | Follow the worker's typestate and exact per-device release protocol. |
| Terminal fault | `cleanup_after_fault` | Quiesce native work and release all local resources before one `DriverFault` is sent. |

`DriverPoll` has `Pending` or `Complete { metric }`; `DriverTransferPoll` has
`Pending` or `Complete { bytes }`. The session verifies every reported byte
count against the finalized contract. `cleanup_after_fault` has no protocol
per-arena acknowledgment, so an ordinary terminal driver fault proves cleanup
has already returned successfully. Implementations must be deterministic and
safe to call after resources have already been released.

`DriverFault` is a copyable code/detail pair. `DriverFault::cleanup_failed`
uses the reserved `FATAL_CLEANUP_FAILED` code and packs the primary and cleanup
codes into the detail field. `DriverFaultCode` reserves codes for program
identity mismatch, invalid lifecycle, fatal cleanup, projection mismatch,
executor preparation/operation failure, and executor cleanup failure; backend
native codes may use the remaining namespace.

`ExecutorWorkerDriver<B>` is the concrete adapter for a
`recipe_executor::WorkerBackend`. Its constructor:

1. derives a `WorkerProjection` from the finalized bundle, topology, and
   `WorkerAssignment`;
2. compares the projection with the `ProvisionedProgram` device image sizes,
   non-init task IDs/phases/roles, and external transfer direction, bytes,
   route link, duplex mode, and capacity resource; and
3. retains the backend, a `Watchdog`, and the expected program digest.

`ExecutorDriverBuildError` distinguishes projection derivation failures from
program mismatches. After construction, `projection()`, `expected_program()`,
and `active_run()` provide read-only identity/lifecycle observations.

The adapter maps each `WorkerDriver` method to the corresponding
`WorkerExecutionSession` operation, translates executor polls and metric
scalars, checks run and phase through the executor, and maps
`WorkerExecutionError` to a compact `DriverFault` code/detail. `prepare` takes
ownership of the backend only after the program digest and lifecycle are
valid. `finish` recovers the backend from a completed session. `Drop` invokes
fatal cleanup if a session is still active. No HIP, CUDA Runtime API, vendor
math library, host calculation callback, or file path is introduced by this
adapter.

## Errors and failure semantics

`RemoteResult<T>` is `Result<T, RemoteError>`. The variants and their boundaries
are:

* `InvalidConfiguration`: invalid identities, bounds, run epochs, image sizes,
  or other construction-time values.
* `ManifestMismatch`: a manifest, program proof, artifact list, or init-image
  digest differs from the finalized contract.
* `Protocol`: a validly encoded message is illegal for the current typestate,
  phase, task status, direction, schedule, or release state.
* `RunMismatch`, `UnknownTask`, `DuplicateTask`, `UnknownDevice`, and
  `DuplicateDevice`: identity lookup and run-epoch failures.
* `CapacityExhausted` and `Backpressured`: fixed codec, lane, inbox, task-slot,
  or half-duplex capacity cannot accept the operation yet.
* `Codec`: malformed or out-of-bound remote bytes.
* `Driver(DriverFault)`: a worker driver operation or terminal cleanup failed.
* `Transport(recipe_transport::TransportError)`: the lower channel failed;
  `From<TransportError>` performs this conversion.
* `Poisoned`: the session has already observed a fatal protocol, codec, or
  transport transition and cannot be reused.

Transport capacity exhaustion is surfaced as `Backpressured` or a `false`
internal send result without changing task state. The public progress methods
do not hide it with a second implementation or an internal fallback. Protocol,
codec, run, lane, and integrity violations poison the session immediately.
Task-local worker faults are reported as `MasterRunEvent::TaskFailed`; faults
that prevent a safe terminal transition become a single `RemoteError::Driver`
after worker cleanup.

## End-to-end role in Recipe

The expected production path is:

1. Upstream probe, cluster, planning, scheduling, and preparation produce a
   `FinalizedBundle`, its matching measured `Topology`, the worker's device
   assignment, finalized init images, and native executor resources. The
   caller establishes a transport `RuntimeChannel` and exact endpoint
   identities, then chooses `RemoteLimits` and channel capacities that agree.
2. Both peers construct `ProvisionedProgram::from_bundle` from the same bundle
   and topology, selecting the worker device IDs on the worker side. The
   master starts `MasterHandshake`; the worker starts `WorkerHandshake` with a
   driver, mirrored identity, and the same run ID.
3. Both callers poll their handshake until `Ready`, queue each worker image on
   the master, and poll init until both sides enter their run typestate.
4. The master submits only task IDs and transfer payloads represented in the
   immutable program. It consumes `MasterRunEvent` values, releases data slots,
   takes metrics, and keeps progressing while the worker polls its native
   driver and emits worker events.
5. The master either enters `MasterExit` after loop completion or
   `MasterCancelling`. The worker performs the corresponding exit or release
   sequence, calls `finish`, and yields its completed driver and channel.

This boundary preserves Recipe's ownership model: the immutable prepared plan
defines all tasks, bytes, schedules, identities, and capacity resources;
`recipe-remote` only transports and realizes those declarations. The GUI or
other caller must not infer domain state from transport signals, mutate the
program, or bypass the state-machine methods.
