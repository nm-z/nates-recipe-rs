# `recipe-native-executor`: native execution below the executor boundary

`native-executor/src/lib.rs` is the root facade for the `recipe-native-executor`
crate. It is the Recipe-owned boundary between the backend-neutral
`recipe-executor::Backend` lifecycle and concrete host, CUDA Driver, and
ROCr/HSA resources. The root itself declares the module graph and reexports
the types that make that boundary usable. The implementation is in the linked
modules; this document records the exact intent, ownership model, state
transitions, public surface, and repository consumers of that facade.

The crate accepts an immutable `recipe_core::FinalizedBundle`, exact native
runtime images, and bindings that borrow already reopened CUDA contexts or HSA
sessions. It validates the artifact and submission contracts, realizes all
native resources before the run loop, owns the exact arena and pending-token
state used by the loop, and translates the closed
`recipe_executor::BackendWork` enum into asynchronous native operations. It
does not compile kernels, discover hardware, choose a candidate, allocate
resources lazily, load modules from the running API, or mutate topology.

The intended lifecycle is:

```text
measured profile and bindings
    -> candidate request validation
    -> pre-Finalize native realization
    -> maximum-concurrency warm pass(es)
    -> post-warm capacity observation
    -> exact FinalizedBundle handoff
    -> Backend::bind_resources
    -> PreparedRun::initialize
    -> loop submissions and nonblocking polls
    -> exit readback
    -> ordered arena release and resource destruction
```

The same native objects cross the warm-to-final handoff. A successful
pre-final session is not a recipe for rebuilding resources later, and the
finalized backend has no operation that can realize another copy.

## Package and crate-level rules

`native-executor/Cargo.toml` names package `recipe-native-executor`, version
`0.1.0`, Rust edition `2024`, MIT license, and the description
"Native CUDA Driver and ROCr/HSA execution resources for Recipe". Its direct
dependencies are:

| Dependency | Boundary used by this crate |
| --- | --- |
| `recipe-core` | IDs, topology, discovery, draft/final bundles, arenas, tasks, phases, transfers, reservations, and ABI locations. |
| `recipe-cuda` | CUDA Driver contexts, modules, device and pinned buffers, streams, events, and pending tokens. |
| `recipe-executor` | The sealed `Backend` trait, closed `BackendWork` values, pending requests, arena views, polls, and physical accounting records. |
| `recipe-hsa` | ROCr sessions, queues, executable/kernel handles, allocations, prepared pending tokens, and asynchronous copies. |
| `recipe-host` | Host RAM/disk arenas, worker runtime, host pending tokens, and host backend operations. |
| `recipe-kernel` | Kernel ABI and artifact digest inspection for cubin and HSACO images. |
| `recipe-planner` | `PlannedCandidate`, which is the exact pre-final candidate supplied by preparation. |

The root attributes are exact:

```rust
#![deny(missing_debug_implementations)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]
```

Every public type has a `Debug` implementation. Unsafe is confined to the
reviewed Driver, ROCr, host-worker pointer, and lifetime-erasure boundaries in
the implementation modules. The crate does not use the CUDA Runtime API, HIP,
or vendor math libraries.

## Root module graph

The declarations in `lib.rs` are the complete module graph. `accounting`,
`bridge`, `cuda_ffi`, `error`, `evidence`, `local`, and `plan` are private
implementation modules. `candidate`, `cuda`, and `hsa` are public modules,
and their root reexports provide the stable names normally used by consumers.

| Module | Owns | Visibility and role |
| --- | --- | --- |
| [`accounting`](../../src/accounting.rs) | Conversion of a `BackendWork` or completion status into one `PhysicalCall`, plus fixed-batch insertion. | Private. Every adapter and the local composite use it to keep physical accounting at the `recipe-executor` boundary. |
| [`bridge`](../../src/bridge.rs) | Pre-realized one-hop Host/CUDA/HSA staging resources, worker threads, bridge contracts, and bridge pending state. | Private implementation, with selected error, resource, pending, and bridge types reexported at the root. |
| [`candidate`](../../src/candidate.rs) | Candidate artifact pairs, request validation, the dependency-neutral pre-final session factory trait, validation wrapper, and unavailable fail-closed implementation. | Public module and root reexports. This is the preparation boundary used by `recipe-prepare`. |
| [`cuda_ffi`](../../src/cuda_ffi.rs) | The narrow CUDA launch-parameter block and its one unsafe enqueue operation. | Private. It is used only by `cuda.rs`. |
| [`error`](../../src/error.rs) | The non-exhaustive native error domain and submission-queue-capacity check. | Private module, with `Error` and `Result` reexported at the root. |
| [`evidence`](../../src/evidence.rs) | Bounded per-device native realization counts and completed-run teardown evidence. | Private module, with all evidence value types reexported at the root. |
| [`local`](../../src/local.rs) | Host/CUDA/HSA device and task partitioning, candidate stabilization, arena projection, the local composite backend, and the prepared bridge handoff. | Private module, with the local public types and traits reexported at the root. |
| [`plan`](../../src/plan.rs) | Runtime image and artifact values, immutable artifact/submission contracts, finalized artifact ABI checks, and device/task submission planning. | Private module, with runtime artifact values reexported at the root. `ExecutionPlan` and `PlannedSubmission` are crate-private. |
| [`cuda`](../../src/cuda.rs) | CUDA binding, arena, pre-final resource realization, warmed/finalized state handoff, pending state, and `Backend` implementation. | Public module and root reexports. |
| [`hsa`](../../src/hsa.rs) | HSA binding, arena, pre-final resource realization, warmed/finalized state handoff, pending state, and `Backend` implementation. | Public module and root reexports. |

The dependency direction is intentionally one-way at the facade:

```text
recipe-core / recipe-planner / recipe-kernel
                  |
recipe-cuda  recipe-hsa  recipe-host  recipe-executor
       \\          |          |          /
        plan, candidate, cuda, hsa, bridge, local
                         |
                       lib.rs
```

`cuda.rs` and `hsa.rs` implement the sealed `recipe_executor::Backend` trait.
`local.rs` implements the same trait by routing each finalized task to one
host partition, one native partition, or one pre-realized bridge. The backend
trait is sealed by `recipe-executor`; application crates cannot add a fourth
native adapter through this crate.

## Exact root reexports

The following names are the deliberate public root inventory in
[`lib.rs`](../../src/lib.rs). A declaration that is public only inside a
private module is not part of the root API unless it appears here.

### Bridge

```text
StagedBridgeError
StagedBridgePending
StagedBridgeResources
StagedCrossBackend
```

`StagedCrossBackend::new` constructs the production bridge from cloned vectors
of `CudaBinding` and `HsaBinding`. The resource and pending values are opaque
outside the crate except for their `Debug` implementations and their use as
associated types of `CrossBackendTransfer`.

### Candidate preparation

```text
CandidateArtifact
CandidateFailure
CandidateRealizationRequest
CandidateRequestError
CandidateSessionFactory
UnavailableCandidateError
UnavailableCandidateFactory
ValidatedCandidateFactory
ValidatedCandidateSession
```

These types are also available as `recipe_native_executor::candidate::*`.
They let `recipe-prepare` drive a candidate-scoped realization without
depending on CUDA, HSA, or host implementation details.

### CUDA

```text
CUDA_MAXIMUM_SUBMISSION_QUEUES
CudaBackend
CudaBinding
```

`CudaBackend` and `CudaBinding` are also available as
`recipe_native_executor::cuda::*`.

### HSA

```text
HsaBackend
HsaBinding
```

They are also available under `recipe_native_executor::hsa::*`.

### Errors, evidence, and runtime artifact values

```text
Error
Result<T>
NativeBackendKind
NativeDeviceExecutionEvidence
NativeExecutionEvidence
RuntimeArtifact
RuntimeArtifactKind
RuntimeImage
```

The root additionally has these crate-private imports, which are not external
API:

```text
ExecutionPlan
PlannedSubmission
```

## Shared lifecycle contract

The crate owns physical execution, not model semantics. The only executable
work values are the closed `recipe_executor::BackendWork` variants:

| Work variant | Native meaning | Valid phase/endpoint shape |
| --- | --- | --- |
| `InitAdmission` | Copy a caller-provided packed image into one finalized device arena. | `Init`, external source to device destination. |
| `Calculation` | Launch one checked kernel with finalized operand locations and dynamic run/iteration values. | `Loop`, GPU-owned task. |
| `InternalTransfer` | Copy between two resolved device locations, either inside one native context or through the bridge. | `Init` or `Loop`, device to device. |
| `Metric` | Copy one resolved four-byte device value into a pre-realized host-visible buffer and return `f32` or `i32` after completion. | `Loop`, metric value on its submission device. |
| `ExitTransfer` | Copy one resolved device value to a pre-realized host output, or to another HSA-owned device through the HSA child. | `Exit`, device source. |

There is no compiler, loader, allocator, discovery, topology, external-input,
or external-output operation in the running backend API. Those concerns are
represented by preparation, finalized manifests, `InitAdmission`, and
`ExitTransfer`.

Every native adapter records a single `PhysicalCall` for each public backend
operation. `MAX_NON_POLL_PHYSICAL_CALLS` is `1` for CUDA, HSA, and the local
composite. Polling records exactly one `PhysicalCall::Poll` with status
`Pending`, `Complete`, or `Failed`. The helper in `accounting.rs` maps work
variants to admission, calculation, internal-transfer, metric, and exit
submission records, and maps a poll result to its physical poll record. A
fixed `PhysicalCallBatch` overflow becomes `Error::PhysicalAccountingOverflow`
or `LocalError::PhysicalAccountingOverflow`.

The native execution invariants are:

1. Device bindings, runtime artifacts, queue slots, completion slots, arena
   layouts, and task ownership must match the finalized bundle exactly.
2. All resources reachable from a pending token are created before the token
   is handed to the executor. Submission and polling do not grow a native
   resource table or create a module, queue, event, executable, or allocator.
3. A pending token is prepared once per task, becomes active once, reaches a
   terminal state once, and is either recycled for a later loop iteration or
   consumed by exit/teardown.
4. A loop token may be reused only after terminal completion. CUDA resets its
   logical token while retaining the underlying event or stream; HSA resets its
   prepared AQL token; the staged bridge resets both native legs and its worker.
5. Arena ownership is immutable. A local arena is routed only to the backend
   class that owns its finalized device, and every resolved range is checked
   before a native pointer or copy is formed.
6. Teardown destroys bridge resources first, then HSA, then CUDA, then host
   resources in the local composite. The first error is retained while later
   destruction is still attempted.

The current integration note in
[`INTEGRATION_REQUIRED.md`](../../INTEGRATION_REQUIRED.md) records one known
unfinished edge: poisoned native resources currently reject destruction while
unhealthy. Ordered cancellation, terminal observation, and release after a
device or transport failure still require production integration and real
hardware acceptance. This is a current limitation, not a fallback path.

## Runtime images and immutable plan (`plan.rs`)

### Runtime artifact values

`RuntimeArtifactKind` is the native target identity carried with one image:

```text
RuntimeArtifactKind::Cuda {
    identity: recipe_cuda::ArtifactIdentity,
}
RuntimeArtifactKind::Hsa {
    target_id: String,
    code_object_version: u8,
}
```

`RuntimeImage` owns an `Arc<[u8]>` and computes its `recipe_kernel::ArtifactDigest`
once in `RuntimeImage::new(bytes)`. Its public methods are:

```text
RuntimeImage::new(bytes: Arc<[u8]>) -> RuntimeImage
RuntimeImage::bytes(&self) -> &Arc<[u8]>
RuntimeImage::digest(&self) -> ArtifactDigest
```

`RuntimeArtifact` pairs an `ArtifactId`, the shared bytes and digest, a
`KernelAbi`, and a `RuntimeArtifactKind`. Its constructors and readers are:

```text
RuntimeArtifact::new(id, bytes, abi, kind) -> RuntimeArtifact
RuntimeArtifact::from_image(id, image, abi, kind) -> RuntimeArtifact
id(&self) -> ArtifactId
bytes(&self) -> &Arc<[u8]>
digest(&self) -> ArtifactDigest
abi(&self) -> &KernelAbi
kind(&self) -> &RuntimeArtifactKind
```

The fields are `pub(crate)`, so external callers use the readers and cannot
mutate identity after construction. `recipe-prepare` creates these values
after lowering or validating a prebuilt image; native backends then inspect and
load the exact bytes.

### `ExecutionPlan` and submission contracts

`ExecutionPlan` is crate-private and contains three immutable maps:

```text
artifacts: BTreeMap<ArtifactId, ArtifactContract>
submissions: BTreeMap<TaskId, PlannedSubmission>
devices: BTreeSet<DeviceId>
```

`ArtifactContract` retains the finalized `ArtifactIdentity` beside its
runtime image. `PlannedSubmission` records one task, one native device, and
its finalized `SubmissionSlots`. `InitImageContract` records one device, image
value, and exact byte count from an `InitDataImage`.

`ExecutionPlan::validate(bundle, runtime_artifacts)` validates the complete
bundle. `validate_partition(bundle, runtime_artifacts, devices, tasks)` scopes
artifact requirements and task checks to a local backend partition. Both paths
perform these checks:

* Runtime artifact IDs are unique.
* Every finalized calculation artifact has exactly one supplied runtime image,
  and no unselected or unknown runtime image is accepted.
* The image digest, ABI entry symbol, target format, workgroup width, and
  backend-specific target identity match the finalized artifact identity.
* CUDA images must use backend `nvidia-cuda-driver` and ABI `elf64-cubin`; the
  CUDA deployment target string and SHA-256 must match the image identity.
* HSA images must use backend `amd-rocr-hsa`, ABI
  `elf64-amdgpu-code-object-v<version>`, and the exact target ID.
* Every calculation ABI has the finalized element count and buffer argument
  count. Buffer dtype, read/write access, backing byte count, power-of-two
  alignment, and arena-offset alignment are checked in canonical input/output
  order.
* A fault-flag argument, when finalized, is one aligned four-byte device I32
  value on the calculation device and appears exactly at the ABI fault-flag
  position.
* Dynamic `RunId` and `LoopIteration` arguments occur at most once and, when
  present, occur after buffers and fault flag. `ElementCount` is the final ABI
  argument.
* Every task receives one immutable native device and queue/completion slot.
  A transfer's device is its source device, or its destination device for an
  external-to-device admission. External-to-external transfers are rejected.
  Queue and completion slots must exist in the same finalized device resource
  manifest as the task.

The readers used by adapters are `runtime_artifacts()`, `artifact_contract`,
`submission`, and `devices()`. No adapter recreates this plan from mutable
runtime observations.

## Candidate realization (`candidate.rs`)

### Candidate artifact and request

`CandidateArtifact` is the pair supplied to a candidate driver:

```text
CandidateArtifact {
    identity: ArtifactIdentity,
    runtime: RuntimeArtifact,
}
```

Its exact methods are `new`, `identity`, `runtime`, and `into_parts`. The
identity is the static value serialized in the candidate or finalized bundle;
the runtime value is the exact loaded image and ABI.

`CandidateRealizationRequest<'a>` borrows:

```text
topology: &'a Topology
discovery: &'a DiscoveryProfile
candidate: &'a PlannedCandidate
artifacts: &'a [CandidateArtifact]
reservations: &'a ReservationLedger
```

`validate()` validates topology, discovery against topology, the candidate's
Draft against both, the reservation ledger against topology, and the complete
artifact set. Artifact validation rejects overlapping static and deferred-build
IDs, duplicate or unexpected images, a static identity mismatch, missing
runtime images, a runtime identity mismatch, deferred build provenance or
resource mismatch, missing stage placement, missing GPU capability, and target
mismatch. A runtime image is passed through the same `validate_runtime_artifact`
checks used by `ExecutionPlan`.

`CandidateRequestError` is the typed rejection domain. Its validation variants
retain `ValidationErrors` where the core validator provides them and otherwise
identify the offending artifact or identity relation. It implements
`Display` and `std::error::Error`; topology, discovery, Draft, reservation,
and runtime artifact errors expose their source where one exists.

### `CandidateSessionFactory`

`CandidateSessionFactory` is dependency-neutral and is the only native factory
interface visible to `recipe-prepare` through the adapter. It has:

```text
type Session: Debug
type Error: Error + Send + Sync + 'static

reservation_evidence(&self, device: DeviceId)
    -> Result<ReservationEvidence, Self::Error>
realize_candidate(&mut self, request: CandidateRealizationRequest<'_>)
    -> Result<Self::Session, CandidateFailure<Self::Error>>
warm_maximum_concurrency(
    &mut self,
    session: &mut Self::Session,
    candidate: &PlannedCandidate,
    pass: u32,
) -> Result<(), CandidateFailure<Self::Error>>
capacity_snapshot(
    &mut self,
    session: &mut Self::Session,
    topology: &Topology,
    discovery: &DiscoveryProfile,
) -> Result<CapacityLedger, CandidateFailure<Self::Error>>
destroy_candidate(&mut self, session: Self::Session) -> Result<(), Self::Error>
```

The implementation must load and inspect every exact artifact, allocate every
reservation and candidate resource-manifest object, and perform no deferred
initialization after `realize_candidate` returns. A warm pass must execute the
candidate's maximum-concurrency trace. Capacity is observed only after a
complete pass. Destruction is deterministic for both partial and complete
realization.

`CandidateFailure<E>` distinguishes:

```text
CandidateRejected { detail: String }
Fatal(E)
PreFinalRealizationUnavailable { detail: &'static str }
```

The first is candidate-local and may permit trying another finite candidate;
the second aborts preparation; the third says the implementation cannot uphold
the fixed-point boundary.

### Validation wrapper and unavailable factory

`ValidatedCandidateFactory<F>` owns an inner factory and exposes `new`,
`inner`, `inner_mut`, and `into_inner`. Its `CandidateSessionFactory`
implementation validates a request before delegation, stores the exact
candidate/topology/discovery/reservation identities in
`ValidatedCandidateSession<S>`, requires warm pass numbers to start at one and
to name the same candidate, requires capacity snapshots to use the same
topology and discovery identity, and validates every returned `CapacityLedger`
against the stored topology and reservations. The session exposes `inner`,
`inner_mut`, and `into_inner` only; its identity fields are immutable and
private.

`UnavailableCandidateFactory` is the explicit fail-closed default. Its
`reservation_evidence` returns `UnavailableCandidateError`; realization and
warm/capacity methods return `PreFinalRealizationUnavailable`. It never
pretends that a post-Finalize binding path satisfies candidate realization.

## Local composite (`local.rs`)

### Device, arena, and task views

`LocalDeviceClass` is the closed owner class `Host`, `Cuda`, or `Hsa`.
`LocalArena<'cuda, 'hsa>` owns one concrete arena, with variants `Host`,
`Cuda`, and `Hsa`. Its readers are `class()` and `device()`; the native arena
types expose `bytes()` publicly and keep release operations crate-private.

`LocalArenaRef` is the read-only counterpart used by bridge submissions. Its
readers are `class()` and `device()`. `LocalArenaSet` borrows the executor's
immutable `ArenaSet<LocalArena>` and exposes allocation-free `get(device)` and
`iter()` views. It cannot insert, replace, or release an arena.

`LocalPending` is the composite pending-token sum:

```text
Host(HostPending)
Cuda(CudaPending)
Hsa(HsaPending)
Bridge { task: TaskId, pending: BridgePending }
```

Its task identity is internal and is used to route polls and enforce that the
pending token owner matches the finalized task partition.

### Cross-backend traits

`CrossBackendTransfer<'cuda, 'hsa>` describes one pre-realized one-hop bridge.
Its associated `Resource`, `Pending`, and `Error` types must implement the
required debug and error bounds. The methods are:

```text
bind(&mut self, bundle, tasks, devices) -> Result<Resource, Error>
prepare_pending(&mut self, resource, request) -> Result<Pending, Error>
submit(&mut self, resource, arenas, pending, class, work) -> Result<(), Error>
poll(&mut self, resource, pending) -> Result<BackendPoll, Error>
supports_loop_repetition(&self) -> bool
rearm_loop_pending(&mut self, resource, pending) -> Result<(), Error>
destroy(&mut self, resource) -> Result<(), Error>
```

Only `bind` and `prepare_pending` may allocate, register memory, create queues,
or grow storage. `submit` and `poll` are nonblocking and allocation-free. The
composite passes only planner-expanded one-hop device-to-device transfers to
this interface. The default repetition support is false, and the default
rearm method does nothing.

`CandidateCrossBackendTransfer` extends the trait with `Clone` and adds the
pre-final methods:

```text
realize_candidate(&mut self, candidate, tasks, devices) -> Result<Resource, Error>
validate_handoff(&mut self, resource, bundle, tasks, devices) -> Result<(), Error>
recycle_candidate_pending(&mut self, resource, pending) -> Result<(), Error>
```

The implementation must create every staging allocation, registration, queue,
completion object, and worker before Finalize. `validate_handoff` may inspect
finalized addresses but may not allocate or replace the resource.

`RejectCrossBackend` is the homogeneous deployment policy. It succeeds only
when the selected cross-backend task set is empty and otherwise returns
`CrossBackendUnavailable { task }`; it advertises loop repetition only because
there is no real token. `CrossBackendUnavailable::task()` identifies the
rejected task. This policy is useful for a deployment that proves all transfers
are same-owner and must fail closed if that proof stops holding.

`PreparedBridge<Bridge, Resource>` wraps a candidate-realized bridge. It stores
the exact task/device partition and an `Available` or `Consumed` resource
state. `bind` requires the same partition, a prior successful
`validate_handoff`, and one available resource, then consumes it exactly once.
All later bridge operations delegate to the inner bridge and wrap failures in
`PreparedBridgeError::{State, Bridge}`.

### Local partitioning and ownership

`LocalCandidateFactory` is the production `CandidateSessionFactory` over host,
CUDA, HSA, and a candidate bridge. Constructors are:

```text
LocalCandidateFactory::new(host, cuda_bindings, hsa_bindings, bridge, stabilizer)
LocalCandidateFactory::fail_closed(host, cuda_bindings, hsa_bindings, bridge)
LocalCandidateFactory::production(host, cuda_bindings, hsa_bindings, bridge)
```

`fail_closed` installs `UnavailableLocalStabilizer`; `production` installs
`NativeLocalStabilizer`, which executes the real warm trace and observes the
real capacities. The factory captures one initial free-capacity snapshot for
one topology/discovery identity and reuses that snapshot for subsequent
candidates in the same preparation scope. It rejects a changed identity.

`reservation_evidence(device)` selects exactly one declared host, CUDA, or HSA
owner. Host owners produce `ReservationEvidence::NonGpu`; GPU bindings produce
`ReservationEvidence::GpuDisplay` with the binding's enabled connector count.
Duplicate or missing owners are errors.

Candidate and finalized partitioning use the same ownership rules:

| Task or endpoint | Owner |
| --- | --- |
| Calculation on CUDA device | CUDA child backend. |
| Calculation on HSA device | HSA child backend. |
| Calculation on Host or an undeclared device | Rejected as unsupported. |
| Metric | Owner of the metric value's resolved device. |
| External to device or device to external | Owner of that one device. |
| Host to Host transfer | Host child backend. |
| HSA to HSA transfer | HSA child backend. |
| Same-device CUDA to CUDA transfer | CUDA child backend. |
| Different CUDA devices or any Host/CUDA/HSA ownership crossing | Bridge. |
| External to external | Rejected, because no local owner exists. |

Executor-visible transfer routes must have zero or one link. A bridge transfer
must have exactly one link. Queue and completion slots may not be shared by
different local owners, because each child owns its slot table.

Candidate artifact partitioning requires every calculation artifact to belong to
exactly one native GPU class. A runtime artifact used by both CUDA and HSA is
rejected; every selected artifact must occur in the request, and every request
artifact must be selected.

### `LocalPreparedSession` state machine

`LocalPreparedSession<'cuda, 'hsa, Bridge, BridgeResource>` owns:

```text
topology, discovery, PlannedCandidate
loaded ArtifactIdentity values
ReservationLedger
InitialCapacitySnapshot
optional anchored CapacityLedger
Partitions
candidate bridge value
LocalPreparedPhysical
StabilizationState
```

`LocalPreparedPhysical` is one of:

```text
Candidate { host, cuda, hsa, bridge }
Warm { bundle, resources, arenas, tasks, images, exits }
Transition
Destroyed
```

`StabilizationState` is `Realized`, `Warmed { pass }`, or
`Observed { pass }`. The factory permits only this sequence:

```text
realize_candidate -> Realized
warm_maximum_concurrency(pass 1) -> Warmed { 1 }
capacity_snapshot -> Observed { 1 }
warm(pass 2) -> Warmed { 2 }
capacity_snapshot -> Observed { 2 }
...
```

Warm passes must be strictly ordered and must name the same candidate. A
capacity snapshot requires the immediately preceding pass and the same
topology/discovery identity. `NativeLocalStabilizer` calls the session's real
`execute_warm_pass` and `observe_capacity`; `UnavailableLocalStabilizer`
returns a pre-final-unavailable failure.

`execute_warm_pass` first builds a provisional finalized bundle from the exact
candidate Draft, loaded artifact identities, reservations, and discovery
capacity. It transitions the physical state from `Candidate` to `Warm` by
binding each child to that provisional bundle exactly once, creates resolved
warm task records and zero-filled init/exit buffers, and allocates one warm
arena for each candidate arena layout. It then executes all phases through the
same child backends and bridge used by the final run. A failed child bind or
bridge bind attempts cleanup in bridge, HSA, CUDA, host order and marks the
physical state destroyed.

The warm scheduler runs `Init`, `Loop`, and `Exit` in order. A task is runnable
only when its phase matches, all dependencies are complete, and its schedule
window does not overlap a currently pending task. Runnable tasks are prepared
and submitted through the owning child. Every pending task is polled without
waiting in the backend. A terminal task collects a warm exit image when needed,
recycles its token, marks itself complete, and unlocks dependents. If no task is
runnable while no task is pending, the trace is stalled. If pending work makes
no progress, the scheduler backs off from 50 microseconds to 2 milliseconds
and fails closed after its bounded idle-poll limit.

After a complete trace, `observe_capacity` releases all warm arenas and reads
host, CUDA, or HSA free bytes. It computes measured runtime overhead as the
initial free capacity minus capped live free capacity, and recipe-usable bytes
as capped live capacity minus the exact reservation. Fragmentation and safety
headroom are zero in this measured ledger. The first successful snapshot is
anchored; later allocator or display-counter drift cannot rewrite the
finalized scheduler contract.

`into_backend(&FinalizedBundle)` is the only public session handoff. It first
requires `Observed`, exact candidate/bundle identity, exact tasks/kernels/builds
and init images, exact reservations, and exact artifact identities. It requires
warm arenas to have been released and each child and bridge to accept the same
handoff. It consumes the session and returns a `LocalBackend` whose child
states are `Warmed` and whose bridge is a `PreparedBridge` with one available
resource. If validation fails, it destroys the candidate resources before
returning the validation error; a teardown error replaces the validation error
when teardown itself fails.

### `LocalBackend` and the sealed `Backend` implementation

`LocalBackend<'cuda, 'hsa, Bridge>` has these private fields:

```text
host: Option<HostBackend>
cuda: CudaBackend<'cuda>
hsa: HsaBackend<'hsa>
bridge: Bridge
declared_devices: Vec<(DeviceId, LocalDeviceClass)>
native_evidence: NativeExecutionEvidence
```

`LocalBackend::new(host, cuda_bindings, cuda_artifacts, hsa_bindings,
hsa_artifacts, bridge)` constructs a direct post-final `Ready` backend path.
Production preparation uses `LocalPreparedSession::into_backend`, which keeps
the warmed resources instead of realizing a second copy. `native_evidence()`
borrows the completed evidence value.

The associated backend types are:

```text
Arena    = LocalArena<'cuda, 'hsa>
Pending  = LocalPending<'cuda, 'hsa, Bridge::Pending>
Resource = LocalResources<'cuda, 'hsa, Bridge::Resource>
Error    = LocalError<Bridge::Error>
```

`MAX_NON_POLL_PHYSICAL_CALLS` is `1`. The operations route as follows:

| Backend method | Local action |
| --- | --- |
| `bind_resources` | Classify finalized devices/tasks, bind host/CUDA/HSA partitions, and consume the prepared bridge resource. |
| `prepare_pending` | Look up the immutable `TaskOwner` and call the corresponding child or bridge preparation method. |
| `allocate_arena` | Allocate through the owner child and wrap the result in `LocalArena`. |
| `supports_loop_repetition` | Delegates to the bridge. CUDA, HSA, and host child repetition is handled by their pending tokens; a bridge that cannot rearm makes the composite one-shot. |
| `supports_same_queue_pipelining` | Returns true only for CUDA-owned tasks. |
| `submit` | Checks pending ownership, projects the immutable arena map into the child lookup, and dispatches the closed work value to host, CUDA, HSA, or bridge. |
| `submit_loop_iteration` | Rearms or prepares the child/bridge token, then calls the same submission path. |
| `poll` | Polls the owner child or bridge and records one physical poll status. |
| `collect_exit` | Collects only host, CUDA, or HSA external egress. A bridge pending token with an external endpoint is rejected. |
| `release_arena` | Checks device/class ownership, requires healthy native children, and releases through the owner. |
| `destroy_resources` | Captures native evidence, destroys bridge, HSA, CUDA, and host resources, and marks evidence complete only when all succeed. |

`LocalError<BridgeError>` preserves duplicate/missing/unexpected devices,
unsupported calculations or routes, task and arena owner mismatches,
capacity mismatches, backend state, physical accounting overflow, and wrapped
host/native/bridge errors. `LocalResources` stores the immutable device and
task owner maps plus child resources. `ProjectedArenas` implements the host,
CUDA, and HSA arena lookup traits by returning only the matching local variant.

## Staged cross-backend bridge (`bridge.rs`)

### Construction and contracts

`StagedCrossBackend<'cuda, 'hsa>` owns cloned binding vectors. Its public
constructor is:

```text
StagedCrossBackend::new(
    cuda_bindings: Vec<CudaBinding<'cuda>>,
    hsa_bindings: Vec<HsaBinding<'hsa>>,
) -> StagedCrossBackend<'cuda, 'hsa>
```

`StagedBridgeError` is non-exhaustive and covers duplicate/missing bindings,
missing tasks, immutable contract mismatches, invalid state, wrapped host,
CUDA, or HSA errors, worker creation/panic/disconnect/state failures, and
integer conversion failures for transfer bytes and arena offsets.

For each selected bridge task, `TransferContract::from_task` requires a
device-to-device transfer with exactly one finalized route link and different
local ownership, except that two different CUDA devices also cross the bridge.
It records task ID, phase, derived work class, source/destination device and
value IDs/classes, byte count, route, lane claims, and submission slots. The
contract is checked once against the finalized bundle and again against every
submitted `TransferWork`.

`realize_candidate` creates all resources before Finalize:

* Host endpoints use the host arena directly and need no separate native leg.
* CUDA endpoints allocate one pinned host staging buffer, one nonblocking
  stream, and one completion event.
* HSA endpoints allocate one host fine-grained staging allocation and one
  prepared HSA pending token.
* Every selected task gets one bounded host worker with a one-message sync
  channel.

`validate_handoff` checks binding uniqueness, the exact selected task set,
every transfer contract, and every resolved finalized endpoint. It does not
allocate. `recycle_candidate_pending` accepts only a terminal token, resets
both legs and the worker, and returns the prepared token to the task resource.

### Bridge pending state and operation sequence

`StagedBridgeResources` owns a `BTreeMap<TaskId, StagedTaskResources>`. Each
task resource contains its immutable contract, source and destination leg
resources, an available-or-consumed pair of prepared native tokens, and its
worker. `StagedBridgePending` is opaque and records:

```text
task
state: Ready | Source | Middle | Destination | Complete
source active leg
destination active leg
middle host job state: Uninitialized | Ready | Submitted
destination target pointer/host marker
```

Submission validates the token and contract, checks both arena owners and
resolved ranges, creates a host middle job, and then:

```text
Host source     -> submit host read/write/copy worker -> Middle
CUDA/HSA source -> native async source copy            -> Source
Source complete -> submit host worker                  -> Middle
Middle complete to Host                                -> Complete
Middle complete to CUDA/HSA -> native destination copy -> Destination
Destination complete                                    -> Complete
```

Polling is nonblocking. A worker status is an atomic `Idle`, `Pending`,
`Complete`, or `Failed` value; a worker failure is consumed once from its
mutex-protected error slot. Loop repetition is supported. A terminal bridge
token resets the worker, CUDA events or pending tokens, HSA prepared tokens,
middle job, target, and state back to `Ready`. Teardown closes workers and
frees destination then source native staging legs for every task, retaining the
first error while attempting all resources.

The one reviewed lifetime erasure is `erase_cuda_pending_lifetime`: the CUDA
pending value stores a phantom operation borrow, while bridge resources retain
the arena, pinned staging, event, and stream until terminal polling. An
abandoned active CUDA bridge token is therefore not dropped while its native
references could still be in flight.

## CUDA Driver adapter (`cuda.rs`)

### Binding and arena values

`CUDA_MAXIMUM_SUBMISSION_QUEUES` is the explicit native-executor ceiling `32`.
The CUDA Driver API does not expose a finite stream-count attribute, so this is
a Recipe ceiling, not a claim about hardware capacity.

`CudaBinding<'context>` borrows one exact `recipe_cuda::Context` and stores:

```text
device: DeviceId
context: &'context Context
deployment: DeploymentIdentity
maximum_submission_queues: u32
enabled_display_connectors: u32
```

Its public methods are `new`, `device`, `deployment`,
`maximum_submission_queues`, `enabled_display_connectors`, and
`available_bytes`. `available_bytes` reads the Driver free-memory counter and
converts it to Recipe `ByteCount`. The context accessor is crate-private.

`CudaArena<'context>` owns one `DeviceBuffer` and its device ID. `bytes()` is
public. The buffer accessor and consuming `release()` method are crate-private
so only the backend and local composite can release it.

### Realized resource shape

`CudaResources` retains an `ExecutionPlan`, per-task `TaskContract` values,
the set of prepared task IDs, one `DeviceResources` map, and a poisoned bit.
Each device resource retains:

```text
the borrowed CUDA context
queue-slot -> nonblocking Stream
completion-slot -> Available Event or Active task
artifact ID -> loaded Function plus KernelAbi
artifact digest -> stable boxed Module
completion slot -> reusable ParameterBlock
metric task -> four-byte pinned host buffer
one pinned staging buffer sized by the finalized manifest
the exact init-image contract
exit task -> preallocated host output bytes
optional scratch DeviceBuffer
```

`CudaBackend` is a typestate wrapper with states `Ready`, `Prepared`,
`Warmed`, and `Bound`. Its public constructor is `new(bindings, artifacts)`.
`from_prepared` and `from_warmed` are crate-private handoff constructors.
`bind_partition` consumes one state exactly once, validates a partition plan,
and either realizes a direct backend or validates a prepared/warmed handoff.

### Pre-final realization and artifact loading

`CudaPreparedResources::realize` validates duplicate bindings, selected
calculation devices and runtime artifact set, reservations, and each binding's
context/deployment identity. It realizes queues, completion events, pinned
staging, optional scratch, metric buffers, egress buffers, and all selected
modules/functions before returning. Artifacts sharing one digest are grouped so
one cubin module is loaded per distinct image while each logical artifact keeps
its own entry function and ABI.

Before loading, each cubin is checked against the deployment identity and
`recipe_kernel::inspect_cubin`. The inspected entry symbol must equal the
immutable ABI entry. A digest collision with different bytes is rejected.
Invocation `ParameterBlock` capacity is the maximum ABI argument count for a
completion slot. Its values and pointer array are preallocated; it retains
arena buffers as keepalives until the Driver launch returns.

`CudaPreparedResources::validate_handoff` changes its candidate handoff to a
finalized handoff containing bundle identity, task set, immutable plan, and
contracts. `bind_candidate` performs the corresponding warm provisional bind,
and `bind` accepts only the previously validated finalized identity and task
set. A candidate resource cannot be rebound as a warm candidate after it has
been finalized.

### Native work operations

`CudaResources::prepare_pending` checks task phase/class/slots against the
contract, checks realized queue and completion slots, and inserts the task into
the prepared set. `CudaPending` then starts in `Ready`.

Submission validates task, class, slots, phase, init image, route, lane claims,
and resolved ranges. The concrete operations are:

| Operation | Driver action |
| --- | --- |
| Admission | Copy the packed image into pinned staging, then enqueue H2D to the resolved arena range with the task's completion event. |
| Calculation | Fill a preallocated `ParameterBlock`, compute the checked grid, and enqueue the kernel on the finalized stream. |
| Same-context internal transfer | Bounds-check both resolved ranges and enqueue D2D. The device must equal the submission device. Cross-context routes are bridge work. |
| Metric | Require one four-byte value, enqueue D2H into the pre-realized pinned metric buffer, and decode the completed bytes as `F32` or `I32`. |
| Exit transfer | Require device-to-external, enqueue D2H into pinned staging with a completion event, and copy the completed staging bytes into the preallocated per-task egress vector. |

CUDA Driver errors poison the resource state on submission or poll failure.
`poll` checks event ownership or stream presence, returns `Pending` until
terminal, returns completion events to their slot, performs metric/egress
postprocessing, and marks the pending token terminal. `rearm_pending` is legal
only for a terminal loop token and resets the logical native state. An active
token cannot be submitted again. `recycle_pending` requires terminal state and
removes the task from the prepared set.

`take_egress(task)` removes one completed per-task egress vector. It is used by
the executor's exit image path and is not a second transfer operation.

### CUDA teardown and evidence

`destroy` first requires a healthy resource. `destroy_devices` verifies every
stream is idle and every completion event is available, destroys events and
streams, drops loaded functions before unloading stable boxed modules, frees
metric buffers, pinned staging, and optional scratch. `execution_evidence`
reports one `NativeDeviceExecutionEvidence` per device with module-load count,
logical entry lookup count, queue count, completion count, and persistent
allocation count.

## ROCr/HSA adapter (`hsa.rs`)

### Binding and arena values

`HsaBinding<'scope>` borrows one HSA `Session` and one CPU
`DiscoveredAgent` used for host fine-grained and kernarg allocations. It stores:

```text
device: DeviceId
session: &'scope Session<'scope>
host_allocator: &'scope DiscoveredAgent<'scope>
target_id: String
code_object_version: u8
queue_packets: u32
maximum_submission_queues: u32
enabled_display_connectors: u32
```

Its public methods are `new`, `device`, `target_id`, `code_object_version`,
`queue_packets`, `maximum_submission_queues`,
`enabled_display_connectors`, `available_bytes`, and
`allocate_host_fine`. The last method allocates from the exact CPU agent and
grants the bound GPU access. Session and allocator accessors are crate-private.

`HsaArena<'scope>` owns one coarse HSA allocation and its device ID. `bytes()`
is public; allocation access and consuming release are crate-private.

### Realized resource shape

`HsaResources` retains an immutable plan, HSA task contracts, prepared-task
IDs, a pre-final pending-token pool, device resources, and a poisoned bit. Each
device resource retains:

```text
the borrowed HSA session and host allocator
queue-slot -> single-producer Queue
completion-slot -> Available or Active task
artifact ID -> loaded Kernel plus ABI and resource envelope
artifact digest -> one shared Executable per HSACO image
completion slot -> host-visible kernarg allocation and byte buffer
metric task -> four-byte fine-grained allocation
one fine-grained staging allocation
the exact init-image contract
exit task -> preallocated host output bytes
optional coarse scratch allocation
```

`HsaBackend` has the same `Ready`, `Prepared`, `Warmed`, and `Bound` typestate
as CUDA. `new`, `bind_partition`, and the crate-private prepared/warmed
constructors enforce one bind. `HsaResources::realize` also prepares one HSA
pending token for every selected task before the executor lifecycle starts.

### HSA artifact and dispatch preparation

Every HSA binding must have a CPU allocator, a global allocatable kernarg pool,
a global fine-grained pool, and an exact AMD target advertised by its session.
Reservations must use `ReservationMechanism::EnforcedQuota`.

Each selected HSACO is checked for exact target and code-object version. Images
sharing one digest use one loaded executable; `inspect_hsaco_bundle` maps each
logical ABI entry to the runtime symbol. The loaded kernel metadata must provide
at least the finalized kernarg size and alignment. A runtime entry to symbol
lookup failure is reported as `Error::HsaSymbolLookup` with both logical and
runtime names.

The finalized artifact resource envelope records `KernelResourceBounds` and,
when the loaded kernel uses a dynamic call stack, the nonzero private-byte
bound converted to the AQL field width. `bind_finalized_artifact_resources`
compares this envelope against the finalized identity and never permits a
different dynamic launch budget after handoff.

Kernarg capacity is the maximum kernel metadata segment size for each
completion slot. `fill_kernarg` writes canonical eight-byte values for buffers,
fault flag, run ID, loop iteration, and element count into the preallocated
host byte buffer, copies it to the host-visible kernarg allocation, and checks
that no operand or fault flag is missing or duplicated.

### Native work operations

`HsaResources::prepare_pending` validates the immutable task contract, removes
the task's prepared token from the pool, and inserts the task into the prepared
set. `HsaPending` starts in `Ready`, stores the task/device/queue/completion and
work class, and owns the prepared native token.

Submission uses the same closed work classes as CUDA:

| Operation | ROCr/HSA action |
| --- | --- |
| Admission | Copy the caller image into fine-grained staging, then submit prepared H2D with the task's queue and completion slot. |
| Internal transfer | Bounds-check both device endpoints and submit a prepared device copy. |
| Calculation | Fill the prepared kernarg, verify queue capacity is ready, compute padded one-dimensional grid and workgroup geometry, include dynamic private bytes, and dispatch the loaded kernel. |
| Metric | Require one four-byte value, submit a prepared device-to-fine-grained copy, and decode terminal bytes as `F32` or `I32`. |
| Exit transfer | Submit device-to-fine-grained staging for external egress, or device-to-device for an HSA-owned exit route. |

Completion ownership is claimed before submission and released on terminal
completion or submission failure. HSA session-poisoning, poisoned deferred
retirement, negative asynchronous signals, and backend-poisoned errors mark the
resource poisoned. Polling a poisoned resource returns `BackendPoisoned`.

`rearm_pending` resets a terminal loop token's prepared native signal and
logical action. `prepare_loop_pending` accepts a ready token or rearms a
terminal token and rejects active reuse. `recycle_pending` requires terminal
state, resets the native token, and returns it to the pending pool. `take_egress`
and `collect_exit` expose only completed preallocated output bytes.

### HSA teardown and evidence

`destroy` drops the pre-final pending pool before closing executables, because a
prepared token may retain the executable used by its last dispatch. Teardown
requires every completion slot to be available, drains session retirements,
closes queues, drops kernel handles before shared executables, closes kernarg
and metric allocations, closes staging, and frees optional scratch.
`execution_evidence` reports executable image loads, logical entry lookups,
queues, completions, and persistent kernarg/metric/scratch/staging allocations.

## Error and evidence surfaces

### `Error` and `Result`

`Error` is `#[non_exhaustive]`, implements `Debug`, `Display`, and
`std::error::Error`, and is reexported with `Result<T> = Result<T, Error>`.
Its variants are the complete native rejection categories:

```text
DuplicateDevice, DuplicateArtifact, MissingArtifact, UnexpectedArtifact,
ArtifactMismatch, MissingDevice, UnexpectedDevice,
MissingQueue, MissingCompletion, ResourceContention, CompletionBusy,
ArenaMismatch, ValueMismatch, UnsupportedTransfer, UnsupportedLoopContract,
BackendState, BackendPoisoned, SubmissionQueueLimitExceeded,
IntegerOverflow, PhysicalAccountingOverflow,
Cuda, CudaContract, HsaSymbolLookup, Hsa, Kernel, Protocol
```

Identity and finalized-plan errors use IDs and detail strings. Resource
contention identifies the task. Completion contention identifies backend, task,
completion slot, and prior owner. Driver, ROCr, kernel-inspection, and HSA
symbol failures retain their typed source errors. The helper
`ensure_submission_queue_capacity` is crate-private and rejects requested
queues above the binding's exact maximum.

### Native execution evidence

`NativeBackendKind` is `Cuda` or `Hsa`.

`NativeDeviceExecutionEvidence` has public fields:

```text
device: DeviceId
backend: NativeBackendKind
image_loads: usize
entry_lookups: usize
queues: usize
completion_objects: usize
persistent_allocations: usize
```

`NativeExecutionEvidence` is cloneable, comparable, debug-printable, and
defaultable. Its public readers are:

```text
devices(&self) -> &[NativeDeviceExecutionEvidence]
loop_realization_calls(&self) -> u64
teardown_completed(&self) -> bool
live_resources_after_teardown(&self) -> usize
```

Only the local backend can construct completed evidence. A completed value has
zero loop realization calls, `teardown_completed == true`, and zero live
resources after teardown. The local backend captures per-device CUDA/HSA counts
before destruction and publishes completed evidence only after bridge, native,
and host teardown succeeds. Acceptance uses these fields to prove one real
image load, nonzero pre-realized resources, no loop-time realization, and zero
post-teardown live resources.

## End-to-end ownership timeline

The following table maps one production run to the concrete owner at each
boundary.

| Stage | Owner and operation | What may still allocate or realize |
| --- | --- | --- |
| Binding scope | `native-probe::bindings` reopens exact contexts/sessions and constructs `CudaBinding`/`HsaBinding`. | Probe/runtime reopening only. Bindings borrow the scope and cannot escape it. |
| Artifact realization | `recipe-prepare::DeferredArtifactCompiler` lowers deferred stages and builds or inspects cubin/HSACO; it constructs `RuntimeArtifact`. | Compiler and artifact builder. No runtime backend yet. |
| Candidate admission | `NativeExecutorDriver` adapts `CandidateSessionFactory`; `ValidatedCandidateFactory` validates request. | Candidate factory realizes exact modules, queues, signals, staging, scratch, and workers. |
| Warm stabilization | `LocalCandidateFactory` binds provisional children, allocates warm arenas, runs init/loop/exit, recycles tokens, and observes capacities. | One warm arena cycle and backend operations required by the exact candidate. |
| Final handoff | `LocalPreparedSession::into_backend` validates identity and moves the same child resources into `LocalBackend`. | No second module, queue, signal, staging, or candidate resource realization. |
| Executor preparation | `PreparedRun` calls `LocalBackend::bind_resources`, prepares images, pending tokens, and finalized arenas. | Finalized arena allocation and pending-token preparation, all before `LoopStarted`. |
| Init | `InitAdmission` submits one image per finalized device. | No resource creation. |
| Loop | `Calculation`, `InternalTransfer`, and `Metric` are submitted and polled. | No resource creation, loading, discovery, or allocator operation. |
| Exit | `ExitTransfer` completes and the backend copies preallocated egress vectors into executor exit images. | No new native object. |
| Teardown | Executor releases arenas, then asks `destroy_resources`; local order is bridge, HSA, CUDA, host. | Destruction and idle/retirement checks only. |

## Repository consumers

The crate is consumed through exact, observable paths rather than an alternate
runtime. The current workspace references are:

| Consumer | Native-executor surface | Purpose |
| --- | --- | --- |
| [`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs) | `CudaBinding`, `HsaBinding`, their constructors and accessors. | Reopen exact measured GPU contexts/sessions and lend borrowed bindings to one preparation callback. |
| [`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs) | `CUDA_MAXIMUM_SUBMISSION_QUEUES`. | Use the explicit Recipe stream ceiling while probing CUDA resources. |
| [`src/native_prepare.rs`](../../../src/native_prepare.rs) | Binding accessors and `HsaBinding` target metadata. | Build exact per-device target specifications and expose the scoped native preparation callback. |
| [`prepare/src/production.rs`](../../../prepare/src/production.rs) | `CandidateArtifact`, `CandidateRealizationRequest`, `CandidateSessionFactory`, `RuntimeArtifact`, `RuntimeImage`, `RuntimeArtifactKind`, `ValidatedCandidateFactory`, `ValidatedCandidateSession`. | Adapt native candidate realization to `recipe-prepare::CandidateDriver`; retain only the warmed opaque session and immutable images after Finalize. |
| [`training/src/execute.rs`](../../../training/src/execute.rs) | `CandidateCrossBackendTransfer`, `CrossBackendTransfer`, `LocalError`, `LocalPreparedSession`, `NativeExecutionEvidence`, `RuntimeArtifactKind`, `ValidatedCandidateSession`. | Type-check the production `PreparedNativeSession<ValidatedCandidateSession<LocalPreparedSession<...>>>` path, consume `into_parts`, call `into_backend`, and expose native evidence in completed training/inference results. |
| [`src/training.rs`](../../../src/training.rs) | `LocalCandidateFactory`, `StagedCrossBackend`, `NativeExecutorDriver`. | Build the production factory and bridge inside the current native binding scope, construct the `NativeCandidateRealizer`, and run controlled native training. |
| [`src/inference.rs`](../../../src/inference.rs) | `LocalCandidateFactory`, `StagedCrossBackend`, `NativeExecutorDriver`. | Build the same production factory and bridge for dense, KNN, Bayes, and GGUF native inference. |
| [`src/lib.rs`](../../../src/lib.rs) | `NativeBackendKind`, `NativeDeviceExecutionEvidence`, `NativeExecutionEvidence`. | Reexport evidence through the root Recipe library. |
| [`src/facade.rs`](../../../src/facade.rs) | `recipe_native_executor as native_executor`. | Expose the complete crate to advanced callers as `recipe::engine::native_executor`. |
| [`training/src/checkpoint.rs`](../../../training/src/checkpoint.rs) | `NativeExecutionEvidence` through completed execution. | Forward native evidence from checkpoint/report values. |
| [`acceptance/src/main.rs`](../../../acceptance/src/main.rs) | `NativeBackendKind`, `NativeExecutionEvidence` readers and per-device fields. | Verify real CUDA image-load counts, queues, completion objects, persistent allocations, zero loop realization calls, complete teardown, and full lifecycle observations. |

The production constructors in `src/training.rs` and `src/inference.rs` are
representative:

```text
bindings.into_parts()
    -> StagedCrossBackend::new(cuda.clone(), hsa.clone())
    -> LocalCandidateFactory::production(Some(host), cuda, hsa, bridge)
    -> NativeExecutorDriver::new(factory)
    -> NativeCandidateRealizer::new(profile, deferred_compiler, driver)
    -> Preparer::new(provider, realizer)
    -> prepare_and_execute_local_* (...)
```

The training and inference execution functions then consume the prepared
bundle/session, call `ValidatedCandidateSession::into_inner`, call
`LocalPreparedSession::into_backend`, and enter the real `PreparedRun` executor
lifecycle. Completed values clone `LocalBackend::native_evidence()` only after
the executor has reached terminal exit and performed ordered destruction.

## Operational limits and deliberate rejections

The native executor is intentionally closed. The following are current
contractual rejections, not fallback opportunities:

* A runtime image whose digest, target, ABI, entry symbol, workgroup, argument
  shape, or resource identity differs from the finalized identity.
* A binding set with duplicate, missing, or unexpected devices, or with a CUDA
  context/HSA session that does not match its measured deployment.
* A requested queue count above the binding's maximum, or a queue/completion
  slot assigned to another device or local owner.
* A host calculation, an external-to-external transfer, a multi-hop executor
  transfer, or a bridge transfer without exactly one finalized link.
* A cross-backend bridge submission with an external endpoint. External ingress
  and egress are handled by the child backend on the one device endpoint.
* A second backend bind, second pending preparation, active-token resubmission,
  nonterminal recycle, duplicate bridge token, or mismatched finalized handoff.
* CUDA internal work that crosses contexts, HSA work whose target or dynamic
  resource envelope differs, or either backend operation after poisoning.
* A metric that is not one four-byte `f32` or `i32` resolved value, an exit
  collection with the wrong task/size/endpoint, or an arena range outside its
  finalized allocation.

These checks preserve one authoritative state transition from finalized plan to
native operation. They do not synthesize substitute state or hide a broken
transition.
