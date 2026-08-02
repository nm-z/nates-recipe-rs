# `recipe_remote::model`: the immutable remote-run contract

[`remote/src/model.rs`](../../src/model.rs) is the static contract layer for
the `recipe-remote` master/worker runtime. It turns one already finalized
`recipe_core::FinalizedBundle`, one measured and schedulable `Topology`, and a
chosen set of worker devices into a bounded `ProvisionedProgram`. The result
is the same on both peers before a session starts. The model records the
identities, task ownership, image sizes, cross-machine transfer contracts,
duplex reservations, limits, and digests that the session can later admit.

The module does not open a connection, discover hardware, read an artifact,
compile a kernel, allocate a native arena, or perform a transfer. Those are
outside the remote boundary. The caller supplies an already connected
`recipe_transport::RuntimeChannel`, a `RemoteIdentity`, fixed `RemoteLimits`,
the same provisioned program, and a worker-side `WorkerDriver`. The model is
therefore a proof-carrying input to the lifecycle in
[`remote/src/session.rs`](../../src/session.rs), not a second planner or an
execution engine.

## Source relationships

The model has five consumers with different responsibilities:

| Consumer | What it takes from the model |
| --- | --- |
| [`codec.rs`](../../src/codec.rs) | `Capabilities`, endpoint identities, fixed limits, manifest identities and artifact records for the hello and manifest proof; the wire header carries the `RunId`. |
| [`session.rs`](../../src/session.rs) | Every immutable task, device, transfer, phase, schedule, image byte count, half-duplex resource, digest, and limit needed to validate and advance the typestate protocol. |
| [`driver.rs`](../../src/driver.rs) | The `WorkerDriver::prepare` boundary receives the complete `ProvisionedProgram`; the remaining driver methods consume its finalized ids and byte contracts through session validation. |
| [`executor_driver.rs`](../../src/executor_driver.rs) | `ProvisionedProgram` plus private task/device/transfer views to compare the wire contract with the executor's worker projection before native preparation. |
| `lib.rs` | Reexports the public model surface: capabilities, identity, limits, manifests, transfers, provisioned programs, and reported runtime capacities. |

`ProgramTaskKind`, `ProgramTask`, and `ProgramDevice` remain crate-private.
They are deliberately not a second wire schema. The worker receives task and
device identifiers in messages, while both peers retain the complete static
program and validate those identifiers against it.

## Capabilities, identity, and bounds

### `Capabilities`

`Capabilities(u64)` is a bit set used during the symmetric hello exchange.
The six defined bits are:

| Constant | Bit | Meaning required by the session |
| --- | --- | --- |
| `CHUNKED_INIT` | `1 << 0` | Device arena images are admitted as a begin, ordered chunks, and an end. |
| `BIDIRECTIONAL_DATA` | `1 << 1` | The run may carry master-to-worker and worker-to-master external transfers. |
| `PREPARED_EXECUTION` | `1 << 2` | The worker prepares its native executor before init admission. |
| `METRICS` | `1 << 3` | A worker task may publish a typed scalar metric on the metrics lane. |
| `CANCELLATION` | `1 << 4` | The normal cancel path is available. |
| `EXACT_RELEASE_ACK` | `1 << 5` | Every worker arena release has a matching acknowledgment before exit completion. |

`REQUIRED` is the OR of all six bits. `from_bits` and `bits` are lossless
constructors/accessors, `contains` tests whether all requested bits are set,
and `BitOr` combines sets. Unknown future bits are retained by `from_bits`;
the session requires `contains(REQUIRED)` rather than requiring exact bit
equality. `codec::encode_hello` always advertises `REQUIRED`. Decoding only
turns the integer into a set; `SessionCore::validate_hello` rejects a peer
whose set lacks any required bit.

### `RemoteIdentity`

`RemoteIdentity` owns two private `EndpointIdentity` values:

```text
local  : EndpointIdentity { machine: Digest, profile: Digest }
remote : EndpointIdentity { machine: Digest, profile: Digest }
```

`new` rejects equal machine digests with
`InvalidConfiguration("remote endpoints must identify distinct machines")`.
The endpoint type has already rejected zero machine or profile digests, so the
model does not duplicate those checks. `local`, `remote`, and `reversed` are
constant accessors. `reversed` swaps the two endpoints without changing their
contents, which lets the worker construct the view opposite the master's view.

`SessionCore::new` compares both endpoints, including profile digests, with the
already connected transport identity. A mismatch is an invalid configuration,
not a negotiated alternative. The hello frame projects the four endpoint
digests into `WireHello`; the receiver checks that the peer's local endpoint is
this side's expected remote endpoint and vice versa.

### Configuration limits

`ManifestLimits` groups the four static capacities `max_artifacts`,
`max_tasks`, `max_devices`, and `max_transfers`. Its fields are private so a
caller can only create it through `new`. Every capacity must be nonzero or the
constructor returns
`InvalidConfiguration("every remote manifest capacity must be nonzero")`.

`RuntimeSlots` similarly groups the fixed `task_slots`, `data_slots`, and
`metric_slots`. Each must be nonzero, otherwise `new` returns
`InvalidConfiguration("every remote runtime slot capacity must be nonzero")`.

`RemoteLimits` flattens both groups and adds the wire bound
`max_message_bytes`:

```text
max_message_bytes, max_artifacts, max_tasks, max_devices, max_transfers,
task_slots, data_slots, metric_slots
```

`new` requires at least 256 message bytes and rejects a size above `u32::MAX`,
because codec lengths are encoded as 32-bit fields. The errors are respectively
`InvalidConfiguration("remote messages require at least 256 bytes")` and
`InvalidConfiguration("remote message size does not fit the wire ABI")`.
The constructor copies the already validated manifest and runtime capacities;
there are no code-side fallback capacities. Public accessors expose the
message size and the three runtime slot counts. `wire_tuple` exposes all eight
values as `u64` for exact hello comparison. `SessionCore::validate_hello`
rejects any peer tuple that is not byte-for-byte equal to its local tuple.

The model uses two codec overhead constants when sizing these limits:

* `INIT_CHUNK_OVERHEAD` is 48 bytes: the 28-byte remote message header plus a
  device id, offset, and chunk length. It bounds each `InitChunk` payload.
* `codec::USER_DATA_OVERHEAD` is 40 bytes: the same header plus a task id and
  data length. It bounds each `UserData` payload.

`Manifest::from_bundle` also asks `codec::manifest_encoded_bytes` to calculate
the complete manifest payload before a session exists. This makes an oversized
manifest fail during provisioning, rather than after a hello has started.

## Cross-machine transfer model

### Direction and capacity claims

`DataDirection` has two ordered variants, `MasterToWorker` and
`WorkerToMaster`. Ordering is intentional: cross-transfer specifications sort
first by direction, then by finalized schedule start, then by task id.

`DuplexClaim` is a public pair of a nonzero `DuplexResourceId` and its
`DuplexMode` (`Half` or `Full`). It derives total ordering so claims can be
canonicalized. Claims are not sent as individual frames. They are included in
the program digest and checked against the worker executor projection; session
storage turns half-duplex claims into one-token reservations. Full-duplex
claims never consume a shared token.

### `CrossTransfer`

Each `CrossTransfer` is the immutable external boundary for one non-init
transfer task:

| Field | Meaning and downstream use |
| --- | --- |
| `task: TaskId` | The finalized task identity. It indexes session task state and every `UserData`, `DataRequest`, `DataAck`, completion, and failure message. |
| `direction: DataDirection` | Determines which peer owns the byte source and which message form is legal. |
| `schedule: u64` | A finite per-direction static user-data position. It is translated to the connection-global transport position by `SessionCore`. `u64::MAX` is forbidden because it is the transport's unscheduled sentinel. |
| `bytes: ByteCount` | Exact payload length. Master and worker reject any user-data frame or driver completion with a different length. |
| `claims: Box<[DuplexClaim]>` | Canonical capacity resources. Half-duplex resources are acquired before submission and released only after the corresponding completion or acknowledgment. |

`CrossTransfer::new` is private and is reached only after topology-derived
validation. It rejects a zero task id, the sentinel schedule, a zero byte
count, a zero claim resource, or two claims naming the same resource. Claims
are sorted before duplicate detection and storage, so equivalent inputs have
one digest representation. It returns `InvalidConfiguration` for each of
those violations. The current topology derivation emits exactly one claim for
each remote transfer, but the type is a slice so the canonical contract can
represent a validated set without changing session storage.

## Manifest identity

`ManifestArtifact` is the public `(ArtifactId, Digest)` pair sent in a
manifest. `Manifest` contains:

```text
protocol    : u16
bundle      : Digest       // finalized bundle identity
draft       : Digest       // static draft identity
realization : Digest       // realized schedule and resource identity
artifacts   : Box<[ManifestArtifact]>
digest      : Digest       // hash of all fields above, including artifacts
```

`Manifest::from_bundle` performs the first canonicalization step. It checks
the artifact count against `RemoteLimits.max_artifacts`, checks the complete
encoded size against `max_message_bytes`, copies each artifact id and digest,
sorts by id, rejects duplicate ids, fills the three plan identities and the
current `PROTOCOL_VERSION`, then computes `digest`. Capacity failures are
`CapacityExhausted("artifact manifest")` for a count or arithmetic overflow
and `CapacityExhausted("artifact manifest codec")` for a message-size bound.
Duplicate ids are `InvalidConfiguration("artifact ids must be unique")`.

The digest domain is the literal `recipe-remote-manifest-v1`, followed by the
little-endian protocol, bundle, draft, and realization bytes, the artifact
count, and each sorted artifact id and digest. `Manifest::validate`, called by
`SessionCore::new` on both peers, requires the current protocol version,
nonzero plan identities, strictly increasing artifact ids, nonzero artifact
digests, and a recomputed digest equal to `self.digest`. Violations become
`ManifestMismatch` with a distinct reason for the protocol, zero plan identity,
noncanonical order, zero artifact digest, or digest mismatch.

The wire manifest does not transmit the private `Manifest` object directly.
`codec::encode_manifest` writes its three plan identities, manifest digest,
the complete artifact list, and the enclosing `ProvisionedProgram` digest.
`WireManifest::matches_proof` compares those values with the worker's local
program and recomputes the same manifest domain hash from the borrowed artifact
bytes. It additionally requires each wire artifact id to be nonzero and
strictly greater than its predecessor and each digest to be nonzero. Thus the
worker can authenticate a manifest without allocating a second manifest.

## `ProvisionedProgram`: construction and canonical state

`ProvisionedProgram` owns six private fields:

| Field | Contents |
| --- | --- |
| `manifest` | The full-bundle artifact and plan identity proof described above. |
| `digest` | The program digest, computed after every derived array is canonical. |
| `tasks` | Sorted `ProgramTask` entries for worker-owned non-init work. |
| `devices` | Sorted `ProgramDevice` entries for the selected worker devices and their finalized init image sizes. |
| `transfers` | Sorted, direction-scheduled `CrossTransfer` entries for boundary tasks. |
| `half_duplex_resources` | Sorted unique resources extracted from half-duplex claims for fixed session tokens. |

The private records inside those arrays are deliberately small:

```text
ProgramTaskKind = Driver | CrossTransfer
ProgramTask     { id: TaskId, phase: RunPhase, kind: ProgramTaskKind }
ProgramDevice   { id: DeviceId, image_bytes: ByteCount }
CrossTransferSpec
                { task: TaskId, direction: DataDirection, start: u64,
                  bytes: ByteCount, claims: Vec<DuplexClaim> }
```

`ProgramTaskKind::Driver` covers both worker calculations and worker-located
metric tasks. `ProgramTaskKind::CrossTransfer` covers only a boundary transfer
that the remote session must carry over its data lane. `CrossTransferSpec` is a
temporary planner-derived record; it becomes a canonical `CrossTransfer` only
after direction-specific schedule assignment and all size checks. The private
`TaskOwnership` decision has the variants `Master`, `WorkerDriver`,
`Cross(DataDirection, ByteCount)`, and `InitImage`; those variants are the
complete classification outcomes and are not stored after construction.

`from_bundle(bundle, topology, worker_devices, limits)` is the only
constructor. Its steps are deliberately ordered so every later digest and
session allocation depends on validated state:

1. Build and size the manifest.
2. Require `bundle.topology() == topology.identity`, then run both
   `topology.validate()` and `topology.validate_scheduling_properties()`.
   Any validation failure maps to
   `InvalidConfiguration("remote provisioning requires a valid schedulable topology")`.
   Estimated topology properties cannot silently drive this production
   schedule; only measured or explicit override properties pass the core
   validation.
3. Canonicalize `worker_devices` with `canonical_devices`.
4. Look up exactly one finalized init image for every canonical worker device,
   recording only its device id and `image.bytes` in a `ProgramDevice`.
5. Classify every finalized bundle task, reject invalid init ownership, derive
   one-hop remote transfers, and create the corresponding `ProgramTask`.
6. Schedule the derived cross transfers, sort tasks by id, enforce capacities
   and uniqueness, derive the half-duplex resource set, and compute the
   program digest.

### Device canonicalization

`canonical_devices` requires a nonempty list no larger than
`limits.max_devices`, sorts it, rejects zero ids, and rejects duplicate ids.
The first case is
`InvalidConfiguration("worker device count is outside the configured bounds")`,
zero ids are
`InvalidConfiguration("worker device ids must be nonzero")`, and duplicates
are `DuplicateDevice(device)`. Sorting makes worker selection deterministic and
lets both peers compare the same device sequence. A selected id without a
bundle init image returns `UnknownDevice(device)`.

### Task ownership classification

`classify_task` is the sole ownership decision. It never invents a task or
rewrites the finalized graph:

| Finalized task | Worker selection and result |
| --- | --- |
| `TaskKind::Calculation` | A calculation on a selected worker device becomes `WorkerDriver`; otherwise it is master-owned and omitted from the remote task list. |
| `TaskKind::Metric` | The metric value location is resolved from the bundle. A value on a selected worker device becomes `WorkerDriver`; a master value is omitted. Missing value location is `UnknownTask`. |
| `TaskKind::Transfer` with external source, worker destination, and `RunPhase::Init` | `InitImage`. The one device image represents this admission, so no `ProgramTask` is emitted. |
| `TaskKind::Transfer` with both endpoints on selected worker devices | `WorkerDriver`, including local worker executor transfers. |
| `TaskKind::Transfer` with both endpoints outside the worker set | Master-owned and omitted. |
| `TaskKind::Transfer` from nonworker to worker | `Cross(MasterToWorker, bytes)`. |
| `TaskKind::Transfer` from worker to nonworker | `Cross(WorkerToMaster, bytes)`. |

Every finalized task id must be nonzero. A worker-owned init calculation or
metric is invalid because init work must be represented by one device image,
and an init cross transfer is invalid for the same reason. The exact errors
are `InvalidConfiguration("init-phase worker work must be represented by the
one device image")` and
`InvalidConfiguration("init-phase cross transfers must be represented by the
one device image")`.

### One-hop transfer derivation

`derive_cross_transfer` proves that a classified boundary task still matches
the measured plan:

* Its `TaskKind` must be `Transfer`.
* Its byte count must equal the classified transfer byte count and its route
  must contain exactly one link. This is the planner-expanded one-hop
  requirement.
* The link id must resolve in the supplied topology.
* Finalized source and destination endpoints must both resolve to devices.
* The link's `from` and `to` must equal the resolved source and destination,
  preserving direction.
* The transfer's lane claims must contain exactly one `Link` claim for that
  link. External claims are ignored while collecting link claims, but an extra
  link or a different link fails the check.

Failure is an `InvalidConfiguration` explaining a non-transfer task, non-one-
hop task, unknown measured link, non-device endpoint, direction mismatch, or
lane-claim mismatch; a missing finalized endpoint record is
`UnknownTask(task)`. A successful spec stores the task id, direction, the
finalized schedule-window start, exact bytes, and one `DuplexClaim` copied from
the measured link's `capacity_resource` and `duplex` mode.

### Cross-transfer schedule

`schedule_cross_transfers` sorts specs by `(direction, start, task)` before it
assigns wire positions. It reserves the first `init_chunk_count(devices,
limits)` positions of the master-to-worker direction for init image chunks.
Master-to-worker transfers then start at that count; worker-to-master transfers
start at zero. Each direction increments with checked arithmetic, so schedule
overflow is an invalid configuration. `CrossTransfer::new` rejects the
transport sentinel as a second guard.

`validate_transfer_order` enforces `max_transfers` and strictly increasing
schedules within each direction. Finally, every transfer byte count must fit
the host `usize` and be no larger than
`max_message_bytes - USER_DATA_OVERHEAD`. A subtraction failure is
`InvalidConfiguration("remote message capacity cannot hold user data")`; an
oversized payload is `CapacityExhausted("cross-transfer codec payload")`.

`init_chunk_count` uses `max_message_bytes - INIT_CHUNK_OVERHEAD` as the
chunk capacity and computes a ceiling division for each `ProgramDevice` image.
It checks host conversion, additions, the `u64` schedule range, and the same
message-capacity subtraction. It returns `CapacityExhausted` or
`InvalidConfiguration` with the specific host, codec, count, or schedule
reason rather than truncating a size.

### Program and digest canonicalization

After transfer scheduling, tasks are sorted by id. More than
`limits.max_tasks` returns `CapacityExhausted("remote task manifest")`; a
duplicate id returns `DuplicateTask(task)`. The half-duplex list is the sorted
`BTreeSet` of every claim whose mode is `Half`.

The program digest domain is `recipe-remote-program-v1`, followed by:

1. `manifest.digest`.
2. Device count, then every sorted device id and image byte count.
3. Task count, then every sorted task id, phase (`Init = 1`, `Loop = 2`,
   `Exit = 3`), and kind (`Driver = 1`, `CrossTransfer = 2`).
4. Transfer count, then each transfer task id, direction (`MasterToWorker = 1`,
   `WorkerToMaster = 2`), schedule, byte count, claim count, and every sorted
   claim resource and mode (`Half = 1`, `Full = 2`).

`half_duplex_resources` is not hashed separately because it is a deterministic
projection of the hashed transfer claims. The digest is set only after all
arrays are canonical. There is no public mutable constructor or post-build
validation path that could create a second representation.

Public accessors expose the manifest, digest, an exact-size iterator of worker
device ids, and the transfer slice. Session and executor code use the crate-
private task, device, and half-duplex slices. The model intentionally exposes
no device image bytes, only their required lengths; `MasterInit::queue_image`
receives the actual bytes from the caller and hashes them at admission time.

## Runtime capacity report and run identity

`RuntimeCapacities` is a public observation type, not a provisioning input:

```text
task_slots          // active task capacity
data_slots          // retained user-data inbox capacity
metric_slots        // retained metric capacity
scratch_bytes       // local codec and transfer scratch bytes
half_duplex_tokens  // distinct half-duplex resources
```

`MasterStorage::capacities` reports one task slot per provisioned task, the
configured data and metric slot counts, the session codec scratch size, and
the number of program half-duplex resources. The worker reports its configured
active task slots, one fixed data slot in its public capacity report, separate
incoming and outgoing scratch buffers, configured metric slots, their summed
scratch size with the codec buffer, and the same half-duplex token count. These
values describe fixed allocation and backpressure; they are not transmitted
or negotiated separately.

`validate_run_id` rejects `RunId::new(0)` with
`InvalidConfiguration("remote run id must be nonzero")`. `SessionCore::new`
also requires a strictly increasing run id when a `RemoteChannel` is reused.
The nonzero id is written into every codec message header and makes a frame
from another run a `RunMismatch`, which poisons the session.

## Pairing with the wire protocol

The model types are projected into the manual, allocation-free codec rather
than serialized with a generic format. Every encoded message begins with the
remote magic, protocol version, tag, reserved byte, per-lane sequence, and
run id. The codec validates length bounds and trailing bytes; the session then
validates lane, sequence, run, and typestate.

### Model-to-wire projections

| Model or session value | Wire representation and proof |
| --- | --- |
| `Capabilities` | `WireHello.capabilities` as a `u64`; `validate_hello` requires all `REQUIRED` bits. |
| `RemoteIdentity` | Four `Digest` fields in `WireHello`: local machine/profile and remote machine/profile. |
| `RemoteLimits` | `WireHello.limits`, the exact eight-entry `wire_tuple`; any difference is a protocol violation. |
| `Manifest` | `WireManifest` carries bundle, draft, realization, manifest digest, artifact count, and borrowed `(id,digest)` bytes. The outer message also carries `ProvisionedProgram.digest`. |
| `ManifestArtifact` | One 8-byte artifact id followed by one 32-byte digest. The wire side enforces sorted nonzero ids and nonzero digests while recomputing the manifest hash. |
| `ProgramDevice` | Not sent as a record. `InitBegin`, `InitChunk`, and `InitEnd` carry the selected id, expected size, content digest, offsets, and bytes; both sessions check them against local `ProgramDevice.image_bytes`. |
| `ProgramTask` and `ProgramTaskKind` | Not sent as a manifest list. `Execute`, completion, failure, metric, data request, and data frames carry task ids; each role checks the id, phase, kind, direction, and status against its local list. |
| `CrossTransfer` and `DuplexClaim` | Not sent as claims. Direction, schedule, bytes, and half-duplex ownership are checked against the local transfer slice. The complete contract is authenticated through the program digest and the executor projection check. |
| `RuntimeCapacities` | Local report only. It sizes caller-facing slot access and documents backpressure, not wire negotiation. |
| `RunId` | Header field checked by `validate_run_id` and `SessionCore::received`. |

The codec-side paired records are also explicit, borrowed views rather than
owned alternatives:

```text
PeerRole = Master | Worker
WireHello {
    role: PeerRole,
    capabilities: Capabilities,
    local_machine: Digest,
    local_profile: Digest,
    remote_machine: Digest,
    remote_profile: Digest,
    limits: [u64; 8],
}
WireManifest<'a> {
    bundle: Digest,
    draft: Digest,
    realization: Digest,
    manifest_digest: Digest,
    program_digest: Digest,
    artifact_count: usize,
    artifact_bytes: &'a [u8],
}
Decoded<'a> { sequence: u64, run: RunId, message: Message<'a> }
```

`PeerRole` is encoded as 1 for master and 2 for worker; any other value is a
codec error. `WireManifest::artifact_bytes` is exactly
`artifact_count * 40` bytes and is borrowed from the received frame. The
decoded header sequence and run are checked before any session state consumes
the borrowed message.

The complete `Message<'a>` payload enum is the paired protocol vocabulary:

```text
Hello(WireHello)
Manifest(WireManifest<'a>)
ManifestAck { manifest: Digest, program: Digest }
Prepare | PrepareAck
InitBegin { device: DeviceId, bytes: ByteCount, digest: Digest }
InitChunk { device: DeviceId, offset: u64, bytes: &'a [u8] }
InitEnd { device: DeviceId, bytes: ByteCount }
InitAck { device: DeviceId }
InitComplete | InitCompleteAck
Execute { task: TaskId }
TaskComplete { task: TaskId }
TaskFailed { task: TaskId, fault: DriverFault }
DriverFault { fault: DriverFault }
DataRequest { task: TaskId }
UserData { task: TaskId, bytes: &'a [u8] }
DataAck { task: TaskId }
Metric { task: TaskId, value: RemoteMetricValue }
Cancel { reason: CancelReason }
CancelAck
BeginExit | ExitReady
Release { device: DeviceId }
ReleaseAck { device: DeviceId }
ExitComplete | ExitAck
```

The tag assignment is stable and follows that order, with `DriverFault` using
tag 27 after the original 26 variants. `encode` writes every field in
little-endian form, preserving the model's exact ids, sizes, digests, and
scalar bits. `decode` rejects unknown tags and scalar kinds, truncated or
oversized slices, a nonzero reserved header byte, a protocol-version mismatch,
and trailing bytes before returning `Decoded`.

### Message phases and lanes

The 27 stable message tags in [`codec.rs`](../../src/codec.rs) are grouped as
follows. Control messages use the control lane, `InitChunk` and `UserData` use
the user-data lane, and `Metric` uses the metrics lane.

| Phase | Master/worker exchange | Model contract checked |
| --- | --- | --- |
| Handshake | `Hello`, `Manifest`, `ManifestAck`, `Prepare`, `PrepareAck` | Endpoint/profile identities, required capabilities, exact limits, all manifest identities and artifacts, and the program digest. |
| Init | `InitBegin`, repeated `InitChunk`, `InitEnd`, `InitAck`, `InitComplete`, `InitCompleteAck` | `ProgramDevice` id and image size, SHA-256 content digest, contiguous offsets, reserved init schedules, and one admission per device. |
| Driver execution | `Execute`, `TaskComplete`, `TaskFailed`, `DriverFault`, `Metric` | A `ProgramTaskKind::Driver` id in the active phase, optional typed metric, and terminal fault cleanup. |
| External data | `DataRequest`, `UserData`, `DataAck` | A `CrossTransfer` id, direction, exact byte count, static schedule, and half-duplex token. |
| Cancellation | `Cancel`, `CancelAck` | Driver cancellation followed by exact per-device arena release and acknowledgment. |
| Exit | `BeginExit`, `ExitReady`, `Release`, `ReleaseAck`, `ExitComplete`, `ExitAck` | Exit-phase completion, one release request and acknowledgment per `ProgramDevice`, and terminal flush ordering. |

`Message::lane` is a closed mapping. A probe frame, a message on the wrong
lane, a missing user-data schedule, a replayed per-lane sequence, a wrong run,
or a transport schedule that is not strictly increasing poisons the session.
The model's `CrossTransfer.schedule` is translated by
`SessionCore::try_send` from a per-run position to the connection-global
position returned by `RuntimeChannel::next_*_schedule_position`.

## Session construction and lifecycle consumers

### `SessionCore` and handshake

`MasterHandshake::new` and `WorkerHandshake::new` both call
`SessionCore::new`. It validates the nonzero run id, local manifest, init
schedule reservation, transport identity, strict reused-channel run ordering,
and obtains connection-global schedule bases. It allocates one codec scratch
buffer of `max_message_bytes` and initializes three transmit and receive
sequence counters.

The master creates `MasterStorage` from the program. It copies every private
`ProgramTask` into a sorted `MasterTaskState { task, phase, kind, status: Idle }`,
clones transfers, allocates configured data and metric slots, makes one empty
`HalfDuplexToken` per model half resource, and makes one `ReleaseState` per
program device. The worker creates the analogous `WorkerStorage`, with
configured active task slots, one incoming and one outgoing scratch buffer,
the same transfer and token projections, and the same per-device release
states.

The session status records are projections of model entries, not new task
identities. `TaskStatus` has the complete transition vocabulary `Idle -> Active
-> Complete` or `Idle -> Active -> Failed`; a byte transfer may pass through
`ReceivedAwaitAck` between `Active` and `Complete`. `DeviceImageStatus` is
`Needed -> Active -> Complete`. `ReleaseStatus` is `Needed -> Requested ->
Complete`. `HalfDuplexToken` stores a model resource id and an optional owning
task id. These small states are what make duplicate, out-of-phase, overlapping,
and premature release messages observable protocol errors.

Hello is symmetric. The master sends its role, the worker sends its role, and
each receiver checks the expected opposite role and all identity and limit
fields. The master then sends the encoded manifest and requires a matching
`ManifestAck`. The worker checks `WireManifest::matches_proof` against its
local manifest and program digest, acknowledges those two digests, and waits
for `Prepare`. Only after `WorkerDriver::prepare(run, &program)` succeeds does
the worker send `PrepareAck`; the master then enters `MasterInit` and the worker
enters `WorkerInit`. A driver fault uses the terminal cleanup path before its
fault frame is flushed.

### Init state and `ProgramDevice`

`MasterInit` creates one `DeviceImageState { device, bytes: image_bytes,
status: Needed }` per program device. `queue_image` admits only one active
logical image at a time, requires a known device in `Needed` state, and requires
the caller's byte length to equal `ProgramDevice.image_bytes`. It computes the
content digest and progresses through `Begin`, `Chunks`, `End`, and `WaitAck`:

1. `InitBegin` carries the device, exact byte count, and SHA-256 digest.
2. Each `InitChunk` carries a contiguous offset and at most
   `max_message_bytes - INIT_CHUNK_OVERHEAD` bytes. Its user-data schedule is
   the next reserved init position. The chunk remains borrowed until the
   transport completion token is transmitted.
3. `InitEnd` repeats the exact total byte count. The master waits for
   `InitAck`, then marks that device complete and admits the next image.
4. When every `ProgramDevice` is complete, `InitComplete` is sent and the
   master waits for `InitCompleteAck`.

`WorkerInit` mirrors the same device list and permits one active image and one
pending chunk or final admission. It checks the device and size against the
`ProgramDevice`, requires each chunk schedule to equal its next local
position, requires a contiguous offset and in-range end, copies into a fixed
chunk buffer, and asks the driver to admit it. A completed chunk must report
the exact submitted byte length. On `InitEnd`, the worker requires the full
image to have arrived, verifies the running SHA-256 digest, calls
`finish_init_image`, polls the exact final length, marks the image complete,
and sends one `InitAck`. `InitComplete` is accepted only with no active image,
no pending final admission, no pending acknowledgment, and every device
complete. The next successful `InitCompleteAck` transition constructs
`WorkerRun`.

The model's image byte counts and `INIT_CHUNK_OVERHEAD` therefore control both
allocation and protocol order, while actual image bytes remain caller-owned
until the master queues them.

### Run state and `ProgramTask` or `CrossTransfer`

`MasterRuntime::submit_task` admits only an idle `ProgramTaskKind::Driver` in
the requested `RunPhase`, sends `Execute`, and marks it active. It never lets
the master submit a worker cross transfer through the driver path.

For `MasterToWorker`, `send_user_data` looks up the `CrossTransfer`, checks
direction, exact byte count, phase, kind, and idle status, acquires every
half-duplex token, sends `UserData` at the static schedule, and marks the task
active. If the lane is full it releases the token and returns
`Backpressured("user-data lane")`. For `WorkerToMaster`,
`request_user_data` enforces one active egress request, checks the same static
contract, acquires tokens, sends `DataRequest`, and returns control-lane
backpressure if submission is unavailable.

On the master, `TaskComplete` and `TaskFailed` require an active task in the
current phase. They complete or fail it and release cross-transfer tokens.
Worker-to-master `UserData` must carry the expected task, direction, static
schedule, and exact byte count. It is copied into the first free fixed data
slot and remains `ReceivedAwaitAck`; the public `DataReady` event identifies
that slot. The next progress call sends `DataAck`, marks the task complete,
releases its tokens, and emits `TaskComplete`. A `Metric` must belong to an
active driver task in the current phase; it is stored in a fixed metric slot
and returned as `MetricReady` with `RemoteMetricValue::F32` or `I32`.

On the worker, `Execute` checks phase, driver kind, idle status, and a free
active slot before calling `WorkerDriver::submit_task`. Poll completion may
carry one optional metric. A metric is flushed on the metrics lane before the
worker sends `TaskComplete`; a driver error becomes `TaskFailed`. The task
slot returns to `Free` only after the corresponding result is submitted.

For an inbound master-to-worker `UserData`, the worker rejects overlap,
checks the static transfer and schedule, acquires half-duplex claims, copies
into fixed incoming scratch, and calls `begin_receive_user_data`. A poll must
report the exact byte count before the scratch is cleared and the task becomes
`ReceivedAwaitAck`; the worker then sends `TaskComplete` and releases claims.

For an outbound worker-to-master `DataRequest`, the worker checks direction,
phase, status, exact configured length, and scratch capacity, acquires claims,
and calls `begin_produce_user_data`. The state progresses from `Producing` to
`NeedSend` after an exact-length poll, then sends `UserData` at the static
schedule and waits in `WaitAck`. `DataAck` calls
`user_data_acked`, clears scratch, completes the task, and releases claims.

`MasterStorage` and `WorkerStorage` both check half-duplex claims twice: all
required tokens must be free before any are taken, and release is valid only
when each token is owned by that task. A missing token, disappearing token, or
wrong owner is a `Protocol` error. Full-duplex transfers proceed independently
because they do not appear in the token set.

### Exit, cancellation, and faults

`MasterRun::begin_exit` requires every loop-phase `ProgramTask` and
`CrossTransfer` to be complete. `MasterExit` sends `BeginExit`, waits for
`ExitReady`, runs the static exit phase, then sends `Release` one device at a
time. Every `ReleaseAck` must correspond to a requested `ProgramDevice`; only
after all are complete does it send `ExitComplete` and wait for `ExitAck`.

The worker accepts `BeginExit` only after its loop phase is complete and its
driver has entered exit. It accepts `Release` only after exit tasks are
complete and only once per model device, calls `release_arena`, and sends the
matching `ReleaseAck` after the call succeeds. `ExitComplete` is rejected until
every model release is complete. The driver is finished before `ExitAck` is
flushed. This is the exact release acknowledgment guarantee advertised by
`Capabilities::EXACT_RELEASE_ACK`.

Cancellation takes a separate normal cleanup path. The worker calls
`driver.cancel(reason.code())`, releases every model device with the same exact
acknowledgment states, calls `finish`, and flushes `CancelAck`; the master waits
for all release acknowledgments before accepting `CancelAck`. A terminal
`DriverFault` does not pretend that arena acknowledgments happened. Before a
fault frame is sent, `WorkerDriver::cleanup_after_fault` must quiesce and
release local native resources. If cleanup fails, `DriverFault::cleanup_failed`
combines the primary and cleanup codes. The session then reports
`RemoteError::Driver` and poisons the boundary.

## Concrete executor projection

`ExecutorWorkerDriver::new` derives a `WorkerProjection` from the same
finalized bundle, topology, and worker assignment, then calls its private
`validate_program` before retaining a backend. That comparison is the second
worker-side use of model internals after handshake proof:

* `program.manifest().bundle` must equal the projection bundle digest.
* Device count, every device id, and every `ProgramDevice.image_bytes` must
  equal the projected device and init image contracts.
* Every non-init projected task must have the same id and phase. A projected
  `Local` task maps to `ProgramTaskKind::Driver`; `ExternalIngress` and
  `ExternalEgress` map to `CrossTransfer`; an `InitAdmission` role is forbidden
  in the runtime task list.
* Cross-transfer count, task identity, direction, byte count, exactly one
  claim, claim resource, and claim duplex mode must equal the projection's
  external transfer and measured link.

Any mismatch is `ExecutorDriverBuildError::ProgramMismatch`, before
`WorkerDriver::prepare`. After that proof, each driver method receives only
the model's finalized run, phase, device, task, digest, and bounded byte
contracts. The adapter forwards them to `WorkerExecutionSession`, translates
native metric values to `RemoteMetricValue`, and maps executor failures into
reserved `DriverFault` codes. It cannot bind an unlisted device or execute an
unlisted task.

## Error and invariant map

The model's errors are fail-closed and fall into a few deliberate classes:

| Error | Model invariant represented |
| --- | --- |
| `InvalidConfiguration` | Caller-supplied identity, capacity, run, topology, route, schedule, byte arithmetic, or ownership configuration is impossible or noncanonical. |
| `ManifestMismatch` | A finalized plan identity, artifact list, manifest protocol, or digest differs from the expected immutable proof. |
| `CapacityExhausted` | A configured artifact, task, transfer, image chunk, or message buffer cannot hold the requested bounded object. |
| `UnknownTask` / `DuplicateTask` | A transfer endpoint lookup, session message, or canonical task list cannot resolve exactly one task. |
| `UnknownDevice` / `DuplicateDevice` | A selected worker device, image, release, or session message cannot resolve exactly one device. |
| `Protocol` | A later session consumer observed a state transition inconsistent with this static model, such as wrong phase, direction, schedule, owner, or release status. |
| `Backpressured` | A valid operation cannot currently fit a fixed runtime lane, data inbox, metric inbox, task slot, or half-duplex token. |
| `Codec`, `Transport`, `Driver`, `Poisoned` | The paired codec, connected transport, native worker, or already-failed session boundary rejected the operation. |

Important invariants are established once and then consumed repeatedly:

* Both peers use the same protocol version, endpoint/profile identities,
  eight-element limits tuple, manifest digest, artifact list, and program
  digest before `prepare`.
* Every selected worker device and every remote task id is nonzero, unique,
  sorted, and tied to a finalized bundle object.
* Init work is represented exactly once by each `ProgramDevice` image. It cannot
  leak into the runtime task list or cross-transfer schedule.
* Every cross transfer is a planner-expanded one-hop device route with exact
  direction, bytes, finite per-direction schedule, and canonical capacity
  claims.
* User-data schedule positions reserve all init chunks and cannot collide with
  the transport sentinel. The session translates them to a strictly monotonic
  connection-global schedule.
* Runtime messages cannot create new tasks, devices, transfers, capacities, or
  state transitions. They only advance objects already present in the model.
* A half-duplex resource has at most one owner at a time and is released only
  by the task that acquired it. A device arena has one release request and one
  matching acknowledgment before terminal completion.

## End-to-end role

The complete supported path is:

```text
FinalizedBundle + measured Topology + worker DeviceIds + RemoteLimits
    -> ProvisionedProgram::from_bundle
    -> same manifest/program proof on master and worker
    -> MasterHandshake / WorkerHandshake
    -> WorkerDriver::prepare
    -> MasterInit / WorkerInit, one chunked image per ProgramDevice
    -> MasterRun / WorkerRun, only ProgramTask and CrossTransfer contracts
    -> MasterExit / WorkerExit, or cancellation
    -> exact release acknowledgments and terminal completion
```

At no point does `recipe-remote` infer a topology, choose a new schedule,
accept a user-typed rate, or substitute a fallback task. The finalized core
bundle and measured topology are authoritative, `ProvisionedProgram` freezes
the worker-facing projection, the codec authenticates the identities, and the
session and driver enforce the resulting state transitions through the real
connected transport and native executor boundary.
