# `recipe_remote::RemoteError`

This page is the error contract for [`remote/src/error.rs`](../../src/error.rs).
The public crate re-exports `RemoteError` and
`RemoteResult<T> = core::result::Result<T, RemoteError>` from
[`remote/src/lib.rs`](../../src/lib.rs#L18-L20). The page also records the
other public error-shaped values that form the remote worker boundary:
`DriverFault`, `DriverFaultCode`, and `ExecutorDriverBuildError`.

The remote crate is a bounded protocol over an already connected
`recipe_transport::RuntimeChannel`. It does not establish a connection, load
an artifact, discover hardware, or recover a failed session. An error either
rejects a static contract before a session exists, reports a live protocol or
driver failure, or marks the connection-global session core poisoned. The
caller must then stop using that session and, when the worker owns a native
executor, complete the worker fault or cleanup path described below.

## Shape and rendering

`RemoteError` is `Debug`, `#[non_exhaustive]`, implements `Display` and
`std::error::Error`, and has no `source()` override. Consequently a contained
`DriverFault` or `recipe_transport::TransportError` is rendered in the
message, but is not exposed through an error-source chain. The source does not
derive `Clone`, `Copy`, `PartialEq`, or `Eq` for this enum.

`RemoteResult<T>` is only an alias. It adds no conversion or state behavior.
The only explicit conversion is
`From<recipe_transport::TransportError> for RemoteError`, which constructs
`RemoteError::Transport` unchanged (`error.rs:64-66`). Rust's `?` operator uses
that conversion at every remote call into `RuntimeChannel`.

The exact `Display` forms are:

| Variant | Display text |
| --- | --- |
| `InvalidConfiguration(detail)` | `invalid remote configuration: {detail}` |
| `ManifestMismatch(detail)` | `remote manifest mismatch: {detail}` |
| `Protocol { detail }` | `remote protocol violation: {detail}` |
| `RunMismatch { expected, actual }` | `remote run {actual} differs from expected run {expected}` |
| `UnknownTask(task)` | `remote task {task} is not provisioned` |
| `DuplicateTask(task)` | `remote task {task} appears more than once` |
| `UnknownDevice(device)` | `remote device {device} is not provisioned` |
| `DuplicateDevice(device)` | `remote device {device} appears more than once` |
| `CapacityExhausted(resource)` | `remote {resource} capacity is exhausted` |
| `Backpressured(resource)` | `remote {resource} is backpressured` |
| `Codec(detail)` | `remote codec rejected a message: {detail}` |
| `Driver(fault)` | `worker driver fault {fault.code} with detail {fault.detail}` |
| `Transport(error)` | `remote transport failed: {error}` |
| `Poisoned` | `remote session is poisoned` |

The `Debug` representation is the derived Rust enum representation. The
static string payloads are the current operation details, not stable machine
codes. `DriverFault.code` is the machine-facing code; its `detail` field is an
allocation-free numeric context described in the driver section.

## Session error state

`SessionCore` owns a `poisoned: bool` bit (`session.rs:407-421`).
`ensure_healthy` returns `Poisoned` once that bit is set (`session.rs:471-477`),
and `poison` stores the bit and returns the supplied error unchanged
(`session.rs:479-482`). Poisoning is terminal for the current remote
typestate. It is not a replacement error: the first error is returned to the
caller, and later operations return `Poisoned`.

The bit is set in these paths:

1. `SessionCore::progress_transport` and all tracked send paths convert a
   non-capacity transport error into `Transport` and call `poison`
   (`session.rs:484-496`, `535-547`, `560-572`, `584-596`). A transport lane
   capacity result is intentionally treated as pending, not as failure.
2. `SessionCore::received` sets the bit for codec rejection, wrong run, wrong
   message lane, and wrong per-lane sequence (`session.rs:599-631`). A frame
   that arrived on a probe lane, a schedule underflow, or a sequence-counter
   arithmetic overflow is returned directly by that function and does not set
   the bit at that statement (`session.rs:604`, `632-647`).
3. `release_received` poisons when the transport cannot release the receive
   token (`session.rs:655-660`).
4. Session phase machines explicitly call `core.poison` for a message that is
   valid on the wire but invalid for the current typestate, for a remote
   terminal `DriverFault`, and for a manifest or image digest mismatch. The
   phase-specific inventory below names each such check.

Many errors are deliberately returned directly and leave `poisoned` unchanged:
static configuration failures, caller-side unknown or duplicate resources,
backpressure, codec errors while encoding an outbound message, and local
state checks written as `return Err(...)` or `?` rather than
`core.poison(...)`. A direct error does not imply that a session can continue;
it only means this implementation did not set the bit at that boundary.

## Variant inventory

The enum has fourteen current variants. It is non-exhaustive, so downstream
matches require a wildcard arm. The configuration, manifest, protocol, run,
task, and device variants identify the violated contract or resource.
`CapacityExhausted` and `Backpressured`
describe distinct fixed-capacity outcomes. `Codec`, `Driver`, and `Transport`
are boundary wrappers, while `Poisoned` is the post-failure state guard.

| Variant | Primary producer | State consequence |
| --- | --- | --- |
| `InvalidConfiguration` | `model.rs` constructors and static schedule checks, plus local session argument checks | Returned before or during the requested operation; never constructed through `core.poison` in the current source. |
| `ManifestMismatch` | Manifest validation, handshake proof checks, and init-image hash verification | Construction-time values are direct. Handshake and init mismatches call `core.poison`. |
| `Protocol` | Wire/session state, lane and schedule invariants, task/transfer state, and typestate transitions | Some caller checks return directly. Remote-message and terminal-state violations usually call `core.poison`; exact sites are listed below. |
| `RunMismatch` | `SessionCore::received` after codec decoding | Sets `poisoned` before returning. |
| `UnknownTask` | Provisioning lookup, task/transfer storage lookup, and malformed remote task references | Usually returned directly. Worker message handling routes it through `core.poison`; master progress lookup returns it directly. |
| `DuplicateTask` | Canonical task validation in `ProvisionedProgram::from_bundle` | Static construction fails before a session exists. |
| `UnknownDevice` | Provisioning lookup, image queueing, image/release acknowledgment lookup | Usually returned directly. The exact phase machines do not replace it with `Poisoned`. |
| `DuplicateDevice` | Canonical worker-device validation and repeated `MasterInit::queue_image` | Static or caller operation fails; no session poison. |
| `CapacityExhausted` | Manifest/task/transfer limits, codec buffers, data slots, and worker scratch/task slots | Static and caller capacity failures are direct. The worker init-chunk-buffer check explicitly poisons; active worker message capacity errors are subsequently poisoned by the worker progress boundary. |
| `Backpressured` | Fixed half-duplex tokens, logical image queue, inboxes, and full transport lanes | A recoverable admission refusal. The caller can progress and retry; no poison bit is set. |
| `Codec` | Manual wire encoder/decoder and bounded reader/writer | Outbound encoding errors return directly. Inbound decoding errors set `poisoned`. |
| `Driver` | `WorkerDriver` calls, worker terminal-fault reporting, and received `DriverFault` frames | A local worker fault enters cleanup/reporting. A master-received terminal fault poisons. A task-level `TaskFailed` frame is an event, not this variant. |
| `Transport` | `From<TransportError>` and `?` across the runtime channel | Transport failures from progress, send, receive release, or schedule conversion normally poison when they pass through the corresponding core helper. Construction-time channel errors are direct. |
| `Poisoned` | `SessionCore::ensure_healthy` after the bit is set | No transport or driver operation is attempted. |

## Static model and configuration construction

All constructors in this section return `RemoteResult` before a
`SessionCore` exists. They therefore return their original error directly and
cannot leave a session poisoned.

### `RemoteIdentity`, limits, and transfer contracts

`RemoteIdentity::new` rejects equal endpoint machine identities with
`InvalidConfiguration("remote endpoints must identify distinct machines")`
(`model.rs:55-62`). `ManifestLimits::new` and `RuntimeSlots::new` reject any
zero capacity with, respectively,
`"every remote manifest capacity must be nonzero"` and
`"every remote runtime slot capacity must be nonzero"`
(`model.rs:88-127`). `RemoteLimits::new` requires at least 256 message bytes
and a size representable by the wire `u32`, producing
`"remote messages require at least 256 bytes"` or
`"remote message size does not fit the wire ABI"`
(`model.rs:143-164`).

`CrossTransfer::new` is the canonical transfer contract constructor
(`model.rs:214-258`). It rejects:

- zero task IDs: `"cross-transfer task id must be nonzero"`;
- `u64::MAX` schedules, which collide with the transport unscheduled
  sentinel: `"cross-transfer schedule collides with the transport sentinel"`;
- zero byte counts: `"cross-transfer byte count must be nonzero"`;
- duplicate capacity resources: `"cross-transfer capacity resources must be unique"`;
- zero capacity-resource IDs: `"cross-transfer capacity resource must be nonzero"`.

Claims are sorted before duplicate detection and storage. A failed
constructor does not retain a partially normalized transfer.

### Manifest creation and validation

`Manifest::from_bundle` returns `CapacityExhausted("artifact manifest")` when
the artifact count exceeds `RemoteLimits.max_artifacts`, when checked encoded
size arithmetic overflows, or `CapacityExhausted("artifact manifest codec")`
when the encoded manifest exceeds the message limit (`model.rs:277-285`). It
sorts artifacts by ID and returns
`InvalidConfiguration("artifact ids must be unique")` for a duplicate ID
(`model.rs:287-302`).

`Manifest::validate` constructs `ManifestMismatch` for each failed canonical
manifest invariant (`model.rs:330-364`):

- `"manifest protocol version differs"`;
- `"manifest contains a zero plan identity"`;
- `"artifact manifest is not canonical"` (IDs are not strictly increasing);
- `"artifact manifest contains a zero digest"`;
- `"manifest digest does not match its contents"`.

`ProvisionedProgram::from_bundle` first calls `Manifest::from_bundle`, then
rejects a topology identity mismatch with
`ManifestMismatch("provisioning topology differs from the finalized bundle")`
(`model.rs:397-409`). Invalid topology validation is normalized to
`InvalidConfiguration("remote provisioning requires a valid schedulable topology")`
(`model.rs:410-415`). A worker device absent from the finalized init images
produces `UnknownDevice(device)` (`model.rs:418-426`).

Task ownership construction rejects these additional configuration errors
(`model.rs:429-483`):

- `"task ids must be nonzero"`;
- `"init-phase worker work must be represented by the one device image"`;
- `"init-phase cross transfers must be represented by the one device image"`;
- `CapacityExhausted("remote task manifest")` when worker task count exceeds
  the configured limit;
- `DuplicateTask(task)` when sorted worker tasks repeat;
- `InvalidConfiguration("duplicate task lookup failed")` only if the
  immediately preceding duplicate search cannot recover the duplicate ID.

### Cross-transfer derivation and scheduling

`derive_cross_transfer` validates the planner-expanded transfer and its measured
one-hop link (`model.rs:579-638`). It returns `InvalidConfiguration` with one of:

- `"cross-boundary work is not a finalized transfer"`;
- `"remote transfers must be planner-expanded one-hop tasks"`;
- `"remote transfer route names an unknown measured link"`;
- `"cross-boundary transfer endpoints must both be devices"`;
- `"remote transfer direction differs from its measured topology link"`;
- `"remote transfer lane claim differs from its one-hop route"`.

Missing finalized transfer endpoints are `UnknownTask(task)`. The returned
spec contains one capacity claim for the measured link, preserving the
direction and byte count that the session later checks on every data message.

`schedule_cross_transfers` assigns independent increasing schedules to the two
directions, beginning master-to-worker positions after all init chunks
(`model.rs:640-679`). Overflow gives
`InvalidConfiguration("master-to-worker schedule range overflowed")` or
`"worker-to-master schedule range overflowed"`. A transfer constructor failure
is propagated unchanged. A message limit too small for user data yields
`InvalidConfiguration("remote message capacity cannot hold user data")`, and
an individual transfer that exceeds the codec payload yields
`CapacityExhausted("cross-transfer codec payload")`
(`model.rs:680-694`). More transfers than the manifest limit yield
`CapacityExhausted("cross-transfer manifest")`; non-increasing per-direction
schedules yield
`InvalidConfiguration("cross-transfer schedules must increase strictly in each direction")`
(`model.rs:810-828`).

`init_chunk_count` uses the configured payload capacity minus the fixed
`INIT_CHUNK_OVERHEAD`. It returns direct `InvalidConfiguration` values for
`"remote message capacity cannot hold an init chunk"`,
`"init image byte count does not fit this host"`,
`"init chunk count overflowed"`,
`"init chunk count does not fit the wire ABI"`, and
`"init schedule range overflowed"` (`model.rs:697-721`).
Canonical worker-device validation returns
`InvalidConfiguration("worker device count is outside the configured bounds")`
for empty or over-limit input, `"worker device ids must be nonzero"` for a zero
ID, and `DuplicateDevice(device)` for a repeated ID (`model.rs:791-807`).
`validate_run_id` returns
`InvalidConfiguration("remote run id must be nonzero")` for run zero
(`model.rs:840-848`).

## Codec errors

The manual, allocation-free codec in `remote/src/codec.rs` uses
`RemoteResult` throughout. `Message::lane` is a static mapping: init chunks and
user data use the user-data lane, metrics use the metrics lane, and all other
messages use control (`codec.rs:183-213`). A lane mismatch is therefore a
protocol error in `SessionCore::received`, not a codec error.

### Outbound construction

`PeerRole::decode` returns `Codec("unknown peer role")` for any value other
than the two role tags (`codec.rs:27-37`). `encode` returns these exact codec
details when a field cannot fit its wire representation:

- `"artifact count does not fit the wire ABI"` for `Manifest`;
- `"init chunk does not fit the wire ABI"` for `InitChunk`;
- `"user data does not fit the wire ABI"` for `UserData`.

`encode_manifest` additionally returns `Codec("manifest payload length overflowed")`
for checked-size overflow, `CapacityExhausted("manifest codec buffer")` when
the caller's scratch buffer is too small, and the same artifact-count codec
detail for a `u32` conversion (`codec.rs:354-384`).

`Writer::bytes` returns `Codec("message length overflowed")` on checked cursor
overflow and `CapacityExhausted("message codec buffer")` when the destination
slice is too small (`codec.rs:616-631`). All writer helpers propagate those
values unchanged. These outbound errors are returned from
`SessionCore::try_send_tracked` without setting `poisoned`; a caller that
already acquired a transfer releases its local transfer token on the failed
submission path (`session.rs:1535-1553`, `1584-1599`).

### Inbound decoding

`decode` first rejects a payload above `RemoteLimits.max_message_bytes()` with
`Codec("message exceeds the configured bound")`. The fixed header checks then
return, in order:

- `"message magic differs"`;
- `"message protocol version differs"`;
- `"message reserved byte is nonzero"`.

Manifest decoding returns `"artifact count does not fit this host"`,
`"artifact count exceeds configured bound"`, or
`"artifact byte count overflowed"` as appropriate. Init and user-data lengths
return `"init chunk length does not fit this host"` or
`"user-data length does not fit this host"`. Metric decoding accepts only the
two scalar tags and returns `Codec("unknown metric scalar type")` otherwise.
An unknown message tag returns `Codec("unknown message tag")`; any unread
payload returns `Codec("message has trailing bytes")` (`codec.rs:387-570`).

`Reader::take` and fixed-width helpers return
`"message range overflowed"`, `"message is truncated"`, or
`"fixed-width field is truncated"` (`codec.rs:646-681`). Inbound codec errors
are consumed by `SessionCore::received`, which sets `poisoned = true` before
returning the original `Codec` value (`session.rs:604-610`). No later frame is
accepted on that session.

## Session core and transport boundary

`SessionCore::new` validates the run ID, manifest, init schedule, connected
transport identity, and strictly increasing run epoch before allocating the
session scratch buffer (`session.rs:423-461`). It returns direct errors for:

- the validation errors listed above;
- `InvalidConfiguration("remote identity differs from the connected transport")`;
- `InvalidConfiguration("run ids must increase strictly on a reused remote channel")`;
- `Transport` from `next_outbound_schedule_position` or
  `next_inbound_schedule_position`.

The two schedule bases are connection-global. `try_send_tracked` requires a
static schedule for user-data messages, returning
`Protocol("user-data message has no static schedule stamp")` when absent and
`Protocol("connection-global user-data schedule exhausted")` when adding the
run base overflows (`session.rs:502-533`). A transmit sequence overflow returns
`Protocol("remote transmit sequence exhausted")`. Transport lane capacity is
converted to `Ok(None)`, allowing the phase machine to remain pending; every
other transport error is converted with `From` and passed to `core.poison`
(`session.rs:535-547`). Hello and manifest sends apply the same sequence and
transport rules (`session.rs:550-596`).

On receive, the transport frame kind is mapped to a runtime lane. Probe frames
produce direct `Protocol("probe frame reached the remote runtime")`
(`session.rs:706-716`). After decoding, the run ID, message lane, and expected
per-lane sequence are checked. A wrong run returns
`RunMismatch { expected, actual }`; a wrong lane returns
`Protocol("message type arrived on the wrong runtime lane")`; and a replayed
or out-of-order sequence returns
`Protocol("remote per-lane sequence is replayed or out of order")`. Each of
those three values poisons. A receive sequence counter overflow returns
`Protocol("remote receive sequence exhausted")` directly. A schedule stamp
below the run's inbound base returns
`Protocol("user-data schedule predates the active run")` directly
(`session.rs:599-652`).

`validate_hello` does not poison itself. It returns direct protocol errors for
wrong role, missing required capability, mismatched endpoint/profile identity,
or differing fixed limits:

- `"peer advertised the wrong remote role"`;
- `"peer lacks a required remote capability"`;
- `"peer hello profile identities differ from the prevalidated endpoints"`;
- `"peer fixed remote limits differ"` (`session.rs:662-687`).

The handshake typestates decide whether to poison after that direct helper
returns. `release_received` maps a transport release failure to `Transport`
and poisons (`session.rs:655-660`).

## Protocol and state invariants

`Protocol { detail }` is the session's invariant failure value. The following
inventory includes every current detail string in `remote/src/session.rs` and
identifies whether the call site poisons the core (`poison`) or returns the
value directly. A direct result is still a failed operation, and the caller
must not infer that the protocol state was repaired.

### Core storage and scheduling

Direct checks in the fixed master and worker storage are:

- `"half-duplex resource is absent from fixed storage"`,
  `"half-duplex resource disappeared"`, and
  `"half-duplex capacity token belongs to another task"` in master transfer
  acquisition/release (`session.rs:302-367`);
- `"worker half-duplex token is absent"`,
  `"worker observed overlapping half-duplex transfers"`,
  `"worker half-duplex token disappeared"`, and
  `"worker half-duplex token belongs to another task"` in worker transfer
  ownership (`session.rs:1923-1990`);
- `"received frame disappeared"` whenever a prior `channel.received()` hint
  is not followed by a decodable frame (`session.rs:766-773`, `867-871`,
  `1023-1029`, `1332-1336`);
- `"release state has no next device"` when the master release table has no
  `Needed` entry even though not all entries are complete
  (`session.rs:2611-2619`).

Master task and data API checks return directly:

- `"driver task is not idle in the active remote phase"`;
- `"master-to-worker data differs from the provisioned transfer"`;
- `"cross transfer is not idle in the active remote phase"`;
- `"data request names the wrong transfer direction"`;
- `"data request is not idle in the active remote phase"`;
- `"unknown data inbox slot"` is `InvalidConfiguration`, not `Protocol`;
- `"data inbox slot is already free"`;
- `"cannot exit before all remote loop tasks complete"`.

These are the checks in `MasterRuntime::submit_task`, `send_user_data`,
`request_user_data`, `release_data`, and `MasterRun::begin_exit`
(`session.rs:1495-1600`, `1724-1737`, `1749-1788`). Failed sends release a
half-duplex token when the lane is full, then return
`Backpressured("control lane")` or `Backpressured("user-data lane")`.

### Handshake

Both handshake state machines return direct `Protocol("received frame disappeared")`
for an inconsistent receive hint. Once a frame exists, a duplicate or
out-of-order handshake message poisons with either:

- `"master received a duplicate or out-of-order handshake message"`;
- `"worker received a duplicate or out-of-order handshake message"`
  (`session.rs:766-812`, `849-920`).

The master poisons for a `ManifestAck` whose manifest or program digest differs
from its own, with `ManifestMismatch("worker acknowledged different manifest identities")`.
The worker poisons for a manifest proof mismatch, with
`ManifestMismatch("master manifest differs from worker provisioning")`.
The master poisons for a received terminal `DriverFault`; the worker's own
prepare failure enters the worker fault-report path rather than returning a
terminal error immediately.

### Init admission

`MasterInit::queue_image` is a caller-side admission API. It returns
`Backpressured("logical init image")` if another image is active,
`UnknownDevice(device)` for an unprovisioned device,
`DuplicateDevice(device)` for a completed or active image, and
`InvalidConfiguration("init image size differs from the finalized arena")`
for a byte-count mismatch (`session.rs:990-1018`). None poisons.

Master receive checks include direct `"received frame disappeared"` and
`UnknownDevice(device)`. An `InitAck` without an active image is direct
`Protocol("init acknowledgment has no active image")`; an acknowledgment that
names the wrong image poisons with `"init acknowledgment names the wrong image"`.
An `InitCompleteAck` is accepted only after `InitComplete` was sent. Any other
message poisons with `"master received an out-of-order init message"`; a
remote `DriverFault` also poisons (`session.rs:1021-1062`).

Master image encoding can return direct
`InvalidConfiguration("init image does not fit the wire ABI")`,
`CapacityExhausted("init chunk codec")`,
`InvalidConfiguration("init offset does not fit the wire ABI")`, and
`Protocol("init schedule sequence exhausted")` (`session.rs:1076-1150`).

Worker init polling turns driver errors into the fault-report path. A driver
completion with a wrong byte count poisons with
`"worker driver completed the wrong init chunk length"` or
`"worker driver completed the wrong init image length"`. Missing or mismatched
active image state uses, respectively, direct
`"completed init chunk has no active image"` and poisoning
`"completed init chunk differs from active image state"`; the image equivalent
is poisoning `"completed init image is not active"`. Schedule counter overflow
is direct `"init schedule sequence exhausted"`; an unknown device lookup is
direct `UnknownDevice(device)` (`session.rs:1253-1330`).

Worker received init messages have these checks:

- overlapping begin messages poison with `"worker received overlapping init images"`;
- unknown devices return `UnknownDevice(device)`;
- a wrong image state or byte identity poisons with
  `"worker init image identity or size differs"`;
- overlapping chunks poison with `"worker received overlapping init chunks"`;
- a schedule replay or gap poisons with `"init chunk schedule is replayed or out of order"`;
- no active image is direct `"init chunk has no active image"`;
- wrong device or offset poisons with `"init chunk device or offset is out of order"`;
- a chunk length conversion failure is direct
  `"init chunk length does not fit the wire ABI"`;
- checked offset overflow is direct `"init chunk range overflowed"`;
- an end beyond the finalized image poisons with
  `"init chunk exceeds the finalized image"`;
- a chunk larger than the preallocated buffer poisons with
  `CapacityExhausted("worker init chunk buffer")`;
- overlapping final admissions poison with `"worker received overlapping final init admissions"`;
- an `InitEnd` without an active image is direct `"init end has no active image"`;
- a partial, wrong-size, or wrong-device image poisons with
  `"init end differs from the complete logical image"`;
- a SHA-256 mismatch poisons with
  `ManifestMismatch("init image content digest differs")`;
- conversion of final admission bytes can return direct
  `CapacityExhausted("worker final init admission")`;
- `InitComplete` before all images and acknowledgments poisons with
  `"init completed before every device image"`;
- any other init message poisons with `"worker received an out-of-order init message"`
  (`session.rs:1332-1486`).

### Active execution

Master-side completion and data checks in `MasterRuntime::progress` poison for:

- `"task completion is duplicate or out of phase"`;
- `"task failure is duplicate or out of phase"`;
- `"worker-to-master data differs from the static transfer"`;
- `"worker-to-master data is duplicate or out of phase"`;
- `"more than one data acknowledgment is pending"`;
- `"metric is duplicate or belongs to an inactive task"`;
- `"master received a message invalid for active execution"`.

The missing schedule stamp in a worker-to-master message is a direct
`Protocol("worker-to-master data has no schedule stamp")`; the transfer lookup
can return `UnknownTask`. A valid `TaskFailed` frame marks the task `Failed`,
releases its cross-transfer token, and returns `MasterRunEvent::TaskFailed`.
It does not poison and does not construct `RemoteError::Driver`.
(`session.rs:1603-1701`). A valid task completion similarly marks the task
complete and releases cross-transfer capacity.

`MasterStorage::store_data` returns `Backpressured("master data inbox")` when
no data slot is free and `CapacityExhausted("master data inbox payload")` when
the payload is larger than a slot (`session.rs:369-383`).
`store_metric` returns `Backpressured("master metric inbox")` when all metric
slots are occupied (`session.rs:385-394`). Those values are direct from
`MasterRuntime::progress`; they do not poison.

On the worker, active polling checks driver byte counts. A wrong inbound or
outbound completion length poisons with
`"worker driver completed the wrong inbound data length"` or
`"worker driver completed the wrong outbound data length"`
(`session.rs:2129-2188`). A completed native task with a missing metric scalar
poisons with `"worker metric completion lost its scalar"`.

`handle_execution_message` is the worker's remote-command validator. Its
direct checks include:

- `"worker execute command is duplicate or out of phase"`;
- `"worker received overlapping inbound data"`;
- `"master-to-worker data has no schedule stamp"`;
- `"master-to-worker data differs from the static transfer"`;
- `"master-to-worker transfer is duplicate or out of phase"`;
- `"worker received overlapping data requests"`;
- `"worker data request has the wrong direction"`;
- `"worker data request is duplicate or out of phase"`;
- `"worker data acknowledgment has no outgoing transfer"`;
- `"worker data acknowledgment is duplicate or out of order"`;
- `"worker received a message invalid for active execution"`.

The same function returns `UnknownTask` for absent task or transfer entries,
`CapacityExhausted("worker task slots")`, `CapacityExhausted("worker incoming data")`,
or `CapacityExhausted("worker outgoing data")` for fixed storage limits. Driver
errors are mapped to `RemoteError::Driver` (`session.rs:2191-2316`).
`WorkerRun::progress` and `WorkerExit::progress` route a non-driver error from
this function through `core.poison`; a driver error instead starts
`cleanup_after_fault` and keeps the typestate pending while the terminal fault
frame is flushed (`session.rs:2331-2431`, `2957-3077`).

### Exit and cancellation

The master cannot enter exit until every loop task is complete. In
`MasterExit::progress`, the expected messages and consequences are:

- a driver fault in `WaitReady`, `WaitComplete`, or release processing poisons
  with `RemoteError::Driver`;
- an unexpected message poisons with
  `"master expected the worker exit-ready acknowledgment"`,
  `"master expected the terminal exit acknowledgment"`, or
  `"master expected an arena release acknowledgment"`;
- an acknowledgment for an unknown device returns direct `UnknownDevice(device)`;
- an acknowledgment not in `Requested` state poisons with
  `"arena release acknowledgment is duplicate or out of order"`;
- if the release table has no next device, direct
  `"release state has no next device"` is returned.

The worker checks `BeginExit` only after the loop phase is complete. A failure
from `begin_exit` enters the driver fault path. Otherwise `WorkerExit` sends
`ExitReady`, accepts exact releases, waits for `ExitComplete`, calls
`finish`, and flushes `ExitAck` only after every release acknowledgment is
complete (`session.rs:2382-2403`, `2896-2955`).

Worker exit command checks are:

- `"worker received exit before all loop work completed"` poisons;
- `"worker received arena release before exit tasks completed"` poisons;
- `UnknownDevice(device)` is direct for a release not in the fixed table;
- `"worker arena release is duplicate or out of order"` poisons;
- `"worker received exit-complete before exact arena release"` poisons;
- any other active-execution message follows the execution checks above
  (`session.rs:2974-3077`).

Cancellation uses a separate normal release protocol. The master sends one
`Cancel` and accepts task, metric, data, and exit-ready messages while waiting
for all `ReleaseAck` frames and `CancelAck`. A duplicate release acknowledgment
poisons with `"cancel release acknowledgment is duplicate"`; a driver fault
poisons; any other invalid message poisons with
`"master received an invalid message while cancelling"`
(`session.rs:2683-2746`). Unknown release devices are direct `UnknownDevice`.

The worker's `cancel` driver call can enter terminal fault cleanup. During
`WorkerCancelled`, an acknowledgment with the wrong release state poisons with
`"cancel release acknowledgment has invalid state"`; unknown devices are
direct. The worker releases each arena, calls `finish`, and sends `CancelAck`
only after the exact release sequence has flushed. A worker-side terminal
fault returns `RemoteError::Driver` only after its `DriverFault` frame has been
transmitted (`session.rs:2766-2885`).

`MasterExit::ensure_active` returns direct
`Protocol("remote exit work is not yet active")` for methods called before
`ExitReady`. `into_complete` returns direct
`Protocol("remote exit is not complete")` until the terminal `ExitAck` has
flushed (`session.rs:2631-2653`).

## Identity and resource variants

`UnknownTask` is constructed by `MasterStorage` and `WorkerStorage` binary or
linear lookups (`session.rs:287-300`, `1908-1921`), transfer acquisition and
release (`session.rs:302-367`, `1923-1990`), and static model lookup of
finalized transfer endpoints or value locations (`model.rs:579-605`,
`732-780`). A caller-supplied task ID therefore fails before a command is sent.
When a malformed worker command reaches `handle_execution_message`, the
worker progress boundary poisons the core with that same `UnknownTask` value;
master progress returns the lookup error directly.

`DuplicateTask` is only constructed while sorting the worker task manifest in
`ProvisionedProgram::from_bundle` (`model.rs:470-483`). It prevents a program
with ambiguous task ownership from being handed to either peer.

`UnknownDevice` is constructed while mapping worker devices to finalized init
images (`model.rs:418-426`), queueing a master image, and matching image or
release acknowledgments (`session.rs:990-1005`, `1040-1045`, `1314-1319`,
`1350-1353`, `2571-2574`, `2690-2697`, `2808-2813`, `3003-3009`). It is a
direct lookup failure in all of those current sites. The device table itself
is immutable after session construction.

`DuplicateDevice` is constructed by `canonical_devices` for repeated worker
device IDs and by `MasterInit::queue_image` for an image already active or
complete (`model.rs:791-807`, `session.rs:995-1002`). No image is replaced and
the previously queued image remains the state owner.

## Fixed-capacity outcomes

`CapacityExhausted` means the requested object cannot fit a fixed bound. The
current resource labels are:

- `"artifact manifest"`, `"artifact manifest codec"`,
  `"remote task manifest"`, `"cross-transfer manifest"`, and
  `"cross-transfer codec payload"` in static model construction;
- `"manifest codec buffer"` and `"message codec buffer"` in codec writes;
- `"master data inbox payload"`, `"init chunk codec"`,
  `"worker init chunk buffer"`, and `"worker final init admission"` in init
  or data storage;
- `"worker task slots"`, `"worker incoming data"`, and
  `"worker outgoing data"` in worker runtime admission.

The static and master-side values are direct. The worker init chunk buffer is
passed to `core.poison`; worker command capacity values are returned from
`handle_execution_message` and then poisoned by `WorkerRun::progress` or
`WorkerExit::progress` because they are malformed or unserviceable remote
commands.

`Backpressured` is different: it means a fixed slot or half-duplex token is
temporarily occupied and the protocol can make progress. Current labels are
`"half-duplex capacity token"`, `"master data inbox"`,
`"master metric inbox"`, `"logical init image"`, `"control lane"`,
`"user-data lane"`, and `"worker-to-master transfer request"`
(`session.rs:321`, `375`, `391`, `993`, `1506`, `1546`, `1563`, `1593`).
Transport lane capacity is first represented as `Ok(None)` by
`SessionCore::try_send_tracked`; public master admission methods translate
that pending result to `Backpressured`, while progress methods simply retain
their state and retry. No backpressure path poisons.

## Driver faults

`DriverFault` is a `Copy` payload with only `code: u32` and `detail: u64`
(`driver.rs:5-10`). `DriverFault::new` is the general constructor. The
payload is deliberately allocation-free so a worker can report a native
failure in the live loop. `DriverFault::cleanup_failed(primary, cleanup)`
replaces both codes with `FATAL_CLEANUP_FAILED` and packs the primary code in
the high 32 bits and cleanup code in the low 32 bits of `detail`
(`driver.rs:12-22`). It does not preserve either original detail value.

### Fault code namespace

`DriverFaultCode` is a `Copy`, `Debug`, `Eq`, and `PartialEq` wrapper around a
`u32`. The current Recipe-reserved values are (`driver.rs:25-43`):

| Code | Hex value | Current construction |
| --- | --- | --- |
| `PROGRAM_IDENTITY_MISMATCH` | `0x5245_0002` | `ExecutorWorkerDriver::prepare` when the supplied program digest differs from the driver construction digest; `detail` is the first eight digest bytes interpreted little-endian. |
| `INVALID_LIFECYCLE` | `0x5245_0003` | `invalid_lifecycle()` for missing session/run, duplicate pending init chunk, wrong device poll, or other adapter-local lifecycle precondition; detail is `0`. |
| `FATAL_CLEANUP_FAILED` | `0x5245_0004` | `DriverFault::cleanup_failed`; detail packs two fault codes. |
| `PROGRAM_PROJECTION_MISMATCH` | `0x5245_0005` | Defined in the public namespace but not constructed anywhere in the current source. Program/projection construction mismatches use `ExecutorDriverBuildError::ProgramMismatch` instead. |
| `EXECUTOR_PREPARE_FAILED` | `0x5245_0006` | `executor_fault` for `WorkerExecutionSession::prepare` failure. |
| `EXECUTOR_OPERATION_FAILED` | `0x5245_0007` | `executor_fault` for init, task, external-transfer, cancellation, exit, release, and finish operations. |
| `EXECUTOR_CLEANUP_FAILED` | `0x5245_0008` | `executor_fault` for a native cleanup error returned by `cleanup_after_fault`. |

Backend-native drivers may use codes outside this reserved namespace. The
remote wire carries the code and detail without interpreting them
(`codec.rs:297-305`, `500-507`, `559-565`).

### Worker-driver propagation boundary

`WorkerDriver` methods return `Result<_, DriverFault>` for prepare, one-image
admission, chunk and image polling, task submission/polling, both external data
directions, cancellation, exit, per-device release, finish, and
`cleanup_after_fault` (`driver.rs:62-120`). The session calls these methods in
the following way:

1. A driver error during worker handshake prepare, init polling, active task
   polling, data polling, command handling, cancellation, exit, release, or
   finish enters `begin_driver_fault` when the phase machine can still run the
   fault protocol. `begin_driver_fault` invokes `cleanup_after_fault` before
   publishing any terminal fault (`session.rs:83-102`).
2. A cleanup error is combined with the primary fault using
   `FATAL_CLEANUP_FAILED`. If cleanup succeeds, the original fault is retained.
3. `progress_driver_fault` queues `Message::DriverFault`, waits for its
   completion token to flush, then returns `Some(fault)` to the worker public
   progress method (`session.rs:104-130`). The worker returns
   `RemoteError::Driver(fault)` only after this send is complete. A transport
   error while sending follows the core transport poison path.
4. If a second local terminal fault is attempted while a report is active,
   `begin_driver_fault` returns direct
   `Protocol("worker attempted to report more than one terminal driver fault")`.

The master receives `Message::DriverFault` in handshake, init, active run,
exit, and cancellation. Those handlers call `core.poison(RemoteError::Driver)`
and stop the current typestate. A `Message::TaskFailed { task, fault }` is
different: it is a completed task result, returned as
`MasterRunEvent::TaskFailed`, and leaves other tasks and the session's
typestate usable (`session.rs:1632-1644`).

## Executor-backed driver errors

`ExecutorDriverBuildError` is a separate public error returned by
`ExecutorWorkerDriver::new`, before a `WorkerDriver` can be passed to
`WorkerHandshake::new` (`executor_driver.rs:15-47`, `75-95`). It is
`Clone`, `Copy`, `Debug`, `Eq`, `PartialEq`, and `#[non_exhaustive]`, implements
`Display` and `std::error::Error`, and has two variants:

| Variant | Construction and propagation |
| --- | --- |
| `Projection(WorkerProjectionError)` | `WorkerProjection::derive` failure is converted by `From<WorkerProjectionError>` with no loss. `source()` returns the contained projection error, and `Display` forwards its text. |
| `ProgramMismatch(&'static str)` | `validate_program` rejects a wire `ProvisionedProgram` that differs from the projected executor. `source()` is `None`; display is `remote program differs from worker projection: {detail}`. |

`validate_program` currently uses these exact mismatch details
(`executor_driver.rs:358-448`): `"bundle identity differs"`,
`"device count differs"`, `"device image contract differs"`,
`"task count differs"`, `"init admission leaked into runtime task manifest"`,
`"task identity, phase, or role differs"`, `"cross-transfer count differs"`,
`"cross-transfer identity is absent"`, `"cross-transfer route is unavailable"`,
and `"cross-transfer contract differs"`. This error never becomes
`RemoteError` automatically. A caller must handle it before constructing a
worker handshake.

The nested `WorkerProjectionError` source is the executor projection's
static contract error. Its current variants are
`InvalidTopology`, `TopologyMismatch { bundle, actual }`,
`UnknownMachine`, `UnknownNode`, `NodeIsNotWorker`,
`NodeMachineMismatch { node, expected, actual }`, `EmptyWorker`,
`MissingArena`, `MissingReservation`, `MissingInitImage`,
`MissingInitAdmission`, `DuplicateInitAdmission`,
`InvalidTask { task, detail }`, `InvalidResource { task, detail }`, and
`CapacityOverflow` (`executor/src/worker.rs:97-129`). Every one can be
returned by `WorkerProjection::derive`; the remote adapter wraps it as
`ExecutorDriverBuildError::Projection` and does not turn it into a wire fault.

### Mapping executor failures into `DriverFault`

After construction, `ExecutorWorkerDriver` maps every
`WorkerExecutionError` from the executor session to a numeric
`DriverFault`. Preparation uses `EXECUTOR_PREPARE_FAILED`; all live executor
operations use `EXECUTOR_OPERATION_FAILED`; cleanup uses
`EXECUTOR_CLEANUP_FAILED` (`executor_driver.rs:146-355`). The adapter retains
the backend in the prepare-failure path, combines a prepare and cleanup error
when both occur, and recovers the backend only after a successful finish or
fatal cleanup. `Drop` invokes native fatal cleanup and discards its result
(`executor_driver.rs:121-143`).

`executor_fault` derives `detail` as follows (`executor_driver.rs:460-492`):

| `WorkerExecutionError` variants | `DriverFault.detail` |
| --- | --- |
| `Projection(_)`, `BundleMismatch` | `1` |
| `InvalidRun(run)` | `run.get()` |
| `RunMismatch { actual, .. }` | `actual.get()` |
| `UnknownTask`, `DuplicateDispatch`, `TaskNotActive`, `MetricContract`, `WatchdogExpired` | task ID |
| `UnknownDevice`, `InitDigestMismatch`, `ArenaAlreadyReleased` | device ID |
| `WrongRole`, `DependencyIncomplete`, `ScheduleConflict`, `DeviceFault { readback, .. }` | task ID or readback task ID |
| `WrongPhase { task, .. }`, `ByteCountMismatch { task, .. }` | optional task ID, or `0` when absent |
| `PhaseIncomplete { task, .. }` | task ID |
| `InvalidLifecycle { state, .. }` | lifecycle code: `Prepared=1`, `Init=2`, `Loop=3`, `Exit=4`, `Cancelling=5`, `Finished=6`, `Failed=7` |
| `InitOffsetMismatch { device, .. }` | device ID |
| `Backend { operation, .. }` | operation code: `BindProjection=1`, `PrepareLocal=2`, `PrepareExternal=3`, `AllocateArena=4`, `SubmitLocal=5`, `PollLocal=6`, `SubmitIngress=7`, `SubmitEgress=8`, `PollExternal=9`, `AcknowledgeEgress=10`, `Quiesce=11`, `ReleaseArena=12`, `DestroyResources=13` |
| `Journal(_)` | `u64::MAX - 1` |
| `CapacityOverflow` | `u64::MAX` |
| Any future or otherwise unclassified non-exhaustive variant | `u64::MAX - 2` |

The mapping intentionally discards textual detail and backend error sources;
the remote wire contract is the fixed code/detail pair. Local adapter
lifecycle checks such as polling a different pending init device use
`INVALID_LIFECYCLE` directly instead of the executor mapper.

## Transport wrapper and end-to-end failure behavior

`RemoteError::Transport` is the only remote variant that contains another
error object. `From<recipe_transport::TransportError>` preserves the exact
transport variant. The transport type can report invalid configuration or
identity, unsupported version, invalid or oversized frame, small receive
buffer, sequence or identity mismatch, integrity failure, cancellation,
deadline, closed connection, I/O kind, protocol state, capacity exhaustion,
unknown completion, benchmark failure, or its own `Poisoned` state. Remote
codec and session code does not rewrite those payloads.

There are three observable transport boundaries:

1. `SessionCore::new` returns `Transport` directly if the existing channel
   cannot provide the next global schedule base.
2. `progress_transport`, `try_send_tracked`, hello/manifest sends, and
   `release_received` convert non-capacity transport errors to `Transport` and
   poison the core. A transport lane capacity error is represented as pending,
   not as `RemoteError::Transport` or `Backpressured` until a public admission
   method chooses to report it.
3. `ScheduleStamp::new` errors while adding a user-data frame are converted
   directly with `From` inside `try_send_tracked`; the resulting `Transport`
   value follows the send caller's propagation. It is not silently retried.

The complete failure paths are therefore:

```text
static model or adapter construction
    -> RemoteError or ExecutorDriverBuildError returned directly
    -> no session or native worker state is published

wire decode / wrong run / wrong lane / wrong sequence
    -> RemoteError returned
    -> SessionCore.poisoned = true
    -> later calls return RemoteError::Poisoned

master command admission while a fixed slot is full
    -> RemoteError::Backpressured (or progress remains Pending)
    -> local task/token state is retained or rolled back
    -> caller progresses and retries

worker driver failure
    -> cleanup_after_fault(primary)
    -> optional FATAL_CLEANUP_FAILED combination
    -> DriverFault frame is flushed
    -> worker progress returns RemoteError::Driver
    -> master receives the frame and poisons its session

worker task completion with TaskFailed
    -> MasterRunEvent::TaskFailed(task, fault)
    -> task becomes Failed and transfer capacity is released
    -> session remains in its current run phase

normal exit or cancellation
    -> exact arena release acknowledgments
    -> finish and terminal acknowledgment
    -> MasterComplete/WorkerComplete, with no error
```

The typestate methods return the next state only after the corresponding
terminal acknowledgment has flushed. `RemoteError` does not perform retries,
substitute state, or bypass a failed transition. The authoritative state is
the session core's poison bit plus the fixed task, transfer, image, release,
and fault-report tables described at the construction sites above.
