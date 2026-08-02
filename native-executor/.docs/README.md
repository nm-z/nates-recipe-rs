# `recipe-native-executor`

`recipe-native-executor` is the native side of Recipe's closed
`recipe_executor::Backend` boundary. It consumes an immutable
`recipe_core::FinalizedBundle`, validated runtime images, and native bindings
that were reopened for the measured machine. It realizes CUDA Driver or
ROCr/HSA queues, completion objects, module/executable images, staging,
scratch, metric buffers, and pending tokens before the executor admits data.
It then translates the executor's closed `BackendWork` values into
nonblocking native submissions and polls.

The crate does not discover hardware, compile a kernel, choose a placement,
mutate topology, load a resource lazily, or allocate from `submit` or `poll`.
Discovery and binding are supplied by `recipe-native-probe`; artifact
materialization and candidate selection are supplied by `recipe-prepare`.
The backend receives the resulting immutable plan and owns the native
resources until ordered teardown.

The source of truth for this page is the current implementation under
[`src/`](../src/). The preparation-to-runtime boundary is also described in
[`INTEGRATION_REQUIRED.md`](../INTEGRATION_REQUIRED.md), including the one
known poisoned-teardown integration gap.

## Manifest and dependency graph

[`Cargo.toml`](../Cargo.toml) declares:

| Field | Value |
| --- | --- |
| Package | `recipe-native-executor` |
| Version | `0.1.0` |
| Edition | Rust 2024 |
| License | MIT |
| Description | Native CUDA Driver and ROCr/HSA execution resources for Recipe |
| Rust lint | `unsafe_op_in_unsafe_fn = "deny"` |
| Clippy lint | `undocumented_unsafe_blocks = "deny"` |

The seven path dependencies are deliberately the already-owned Recipe
boundaries:

| Dependency | Native-executor uses it for |
| --- | --- |
| `recipe-core` | Finalized graphs, identities, task phases, resolved values and endpoints, resources, reservations, capacity, and topology contracts. |
| `recipe-cuda` | CUDA Driver context, buffers, streams, events, modules, functions, pinned host buffers, and driver pending operations. |
| `recipe-executor` | The sealed backend ABI, closed `BackendWork`, arenas, pending requests, nonblocking polls, physical-call accounting, and lifecycle ownership. |
| `recipe-hsa` | ROCr/HSA sessions, queues, allocations, executables, kernels, prepared pending signals, and completion polling. |
| `recipe-host` | Host RAM and disk arenas, host backend partitions, exact range I/O, and host staging used by the composite backend and bridge. |
| `recipe-kernel` | Artifact digests, CUDA/HSACO inspection, kernel ABI and arguments, and resource metadata. |
| `recipe-planner` | `PlannedCandidate`, the pre-final draft, stage placements, and arena layouts used during realization. |

The manifest adds no standalone compiler, discovery, network, filesystem,
async-runtime, or vendor-math package. Host disk behavior is reached only
through the `recipe-host` path API. The CUDA and HSA wrappers are the only
code that touches a driver API, and the native executor does not use the CUDA
Runtime API or HIP.

## Module graph and ownership

The crate root [`src/lib.rs`](../src/lib.rs) denies missing `Debug`
implementations and declares the following modules. `accounting`, `bridge`,
`cuda_ffi`, `error`, `evidence`, `local`, and `plan` are private. `candidate`,
`cuda`, and `hsa` have public module surfaces. The root re-exports the public
types listed below.

```text
recipe_native_executor
├── plan.rs       RuntimeArtifact, RuntimeImage, ExecutionPlan, ABI/slot checks
├── candidate.rs  pre-final request validation and CandidateSessionFactory
├── local.rs      Host/CUDA/HSA partitioning, warm stabilization, LocalBackend
├── bridge.rs     one-hop staged cross-backend transfers and host workers
├── cuda.rs       CUDA Driver realization, submission, polling, teardown
├── hsa.rs        ROCr/HSA realization, submission, polling, teardown
├── cuda_ffi.rs   fixed CUDA launch parameter block and one narrow FFI call
├── accounting.rs PhysicalCall mapping and fixed-batch overflow conversion
├── evidence.rs   bounded native realization and teardown evidence
└── error.rs      native, protocol, resource, and lifecycle failure vocabulary

recipe-core ─────┬─> plan, candidate, local, cuda, hsa, bridge
recipe-kernel ───┘       │
recipe-cuda ─────────────┼─> cuda, bridge
recipe-hsa ──────────────┼─> hsa, bridge
recipe-host ─────────────┼─> local, bridge
recipe-executor ─────────┴─> local, cuda, hsa
recipe-planner ─────────────> candidate, local
```

Root exports are grouped by boundary:

* `RuntimeArtifact`, `RuntimeArtifactKind`, and `RuntimeImage` identify an
  immutable native image and its checked launch ABI.
* `CudaBinding`, `CudaBackend`, and
  `CUDA_MAXIMUM_SUBMISSION_QUEUES` expose the CUDA adapter.
* `HsaBinding` and `HsaBackend` expose the HSA adapter.
* `LocalBackend`, `LocalCandidateFactory`, `LocalPreparedSession`, arena
  views, ownership traits, pending variants, and `PreparedBridge` expose the
  heterogeneous local composition.
* `StagedCrossBackend`, `StagedBridgeResources`, `StagedBridgePending`, and
  `StagedBridgeError` expose the production cross-owner transfer bridge.
* `CandidateSessionFactory`, `ValidatedCandidateFactory`, candidate artifact
  and request types, and candidate failure types expose the dependency-neutral
  pre-final lifecycle.
* `NativeExecutionEvidence` and its device records expose post-teardown
  evidence. `Error` and `Result` expose the native error boundary.

## Boundary and lifecycle

The complete path has one direction. A measured probe lends bindings, the
preparer materializes and validates images, the candidate factory realizes
physical resources, and the executor consumes a finalized backend.

```text
measured profile and exact bindings
        │
        ▼
CandidateRealizationRequest::validate
        │ topology, discovery, Draft, reservations, artifact identities/ABIs
        ▼
LocalCandidateFactory::realize_candidate
        │ host runtime, CUDA/HSA resources, bridge resources, pending pools
        ▼
warm_maximum_concurrency(pass = 1..N)
        │ complete Init -> Loop -> Exit trace, including arena cycle
        ▼
capacity_snapshot
        │ release warm arenas, anchor measured quota
        ▼
LocalPreparedSession::into_backend(finalized bundle)
        │ exact identity and resource handoff, no second realization
        ▼
recipe_executor::PreparedRun
        │ bind -> prepare pending -> allocate arenas -> init -> loop -> exit
        ▼
ordered release and destroy
        │ native evidence is retained only after successful destruction
        ▼
Completed training or inference report
```

The running API is intentionally closed. All queue creation, module loading,
executable loading, host staging allocation, completion creation, pending
token preparation, and scratch/metric/egress allocation happen in candidate
realization or backend bind and pending preparation. `submit`, `poll`, loop
rearm, exit collection, arena release, and destruction operate on those exact
objects.

`recipe_executor` owns task ordering, dependencies, phase transitions,
iteration domains, metric publication, exit-image ownership, and the logical
journal. Native-executor owns native resource contracts, native asynchronous
operations, completion-slot ownership, range and ABI checks, and physical
accounting. A task remains one of Recipe's calculation, transfer, or metric
kinds. A metric is a four-byte readback, not a third native workload kind.

Ownership at each boundary is deliberately non-overlapping:

| Owner | State retained |
| --- | --- |
| Planner and finalized bundle | Immutable task identities, phases, routes, lane claims, values, queue/completion slots, arena layouts, init manifests, and artifact identities. |
| Candidate session | Loaded images, reservations, queues, completion objects, native staging, scratch, metric/egress storage, bridge workers, and warm pending pools before final arena offsets exist. |
| `LocalPreparedSession` | The exact candidate identity, measured capacity anchor, child resources, bridge resource, warm arenas, and stabilization state until final handoff or deterministic destruction. |
| `CudaResources` / `HsaResources` | Driver/session objects, task contracts, completion ownership, prepared tokens, per-device native allocations, and the poisoned state after bind. |
| `StagedBridgeResources` | One immutable transfer contract, two endpoint legs, prepared native tokens, host worker, and per-task staged pending state. |
| `recipe_executor` run | Finalized arenas, executor pending handles, dependency and phase state, metrics, exit images, and logical/physical journals. |
| Native evidence | Counts captured from child resources before destruction, then a completed flag only after ordered destruction succeeds. |

## Runtime artifacts and execution plans

### Runtime image types

`RuntimeImage` stores `Arc<[u8]>` plus an `ArtifactDigest` computed from those
bytes. `RuntimeArtifact` adds an `ArtifactId`, `KernelAbi`, and one
`RuntimeArtifactKind`:

| Kind | Identity checked at realization |
| --- | --- |
| `Cuda { identity }` | CUDA target, driver artifact identity, and image digest. |
| `Hsa { target_id, code_object_version }` | AMD target ID and `elf64-amdgpu-code-object-vN` code-object ABI. |

The constructor does not infer target or ABI information. The finalized
artifact identity and the native image must agree at `ExecutionPlan` and
candidate validation.

### `ExecutionPlan`

`plan.rs` builds one internal `ExecutionPlan` from a `FinalizedBundle` and the
runtime images. `validate` covers all bundle tasks and arena devices;
`validate_partition` covers the selected task and device partition used by a
child backend. Both reject duplicate, missing, and unexpected runtime image
IDs.

Artifact checks include:

1. runtime ID, SHA-256 digest, ABI entry symbol, format, target backend, and
   target architecture must match the finalized `ArtifactIdentity`;
2. ABI workgroup width must be nonzero and within finalized resource bounds;
3. finalized build or kernel-template element count and operand count must
   match the ABI;
4. buffer dtypes, read/write access, backing bytes, and power-of-two alignment
   must match each resolved value location;
5. a checked calculation's fault flag must be an aligned four-byte device
   `I32`, and the ABI must contain exactly one canonical `FaultFlag` argument;
6. dynamic `RunId` and `LoopIteration` arguments may occur at most once and
   must follow the buffer and fault arguments; the final argument is always
   `ElementCount`;
7. CUDA images must match the CUDA backend/ABI and driver target and have a
   driver identity digest equal to the image digest;
8. HSA images must match the HSA backend, target architecture, and code-object
   version.

`plan_submissions` derives each task's native device and checks that the
finalized queue and completion slots exist on that same device. A transfer
uses its source device, or its destination device for external-to-device
admission. External-to-external transfers have no native submission device
and are rejected. The plan stores immutable task-to-device/slot assignments;
adapters do not recalculate them during a run.

## Generic candidate boundary

[`src/candidate.rs`](../src/candidate.rs) separates pre-final resource
realization from final arena-address binding.

### Request and artifact validation

`CandidateRealizationRequest` contains references to the topology, measured
discovery profile, `PlannedCandidate`, exact `CandidateArtifact` pairs, and
`ReservationLedger`. `validate` checks topology, discovery, Draft, and
reservations, then validates the artifact set:

* static Draft identities and deferred build identities must be disjoint;
* each supplied artifact identity is valid, unique, and expected exactly once;
* static identities must match the Draft bit-for-bit;
* a deferred identity must match its build recipe, immutable stage placement,
  measured GPU capability, and target;
* each runtime image must pass `validate_runtime_artifact`;
* no required runtime image may be missing or unexpected.

Failures are typed as `CandidateRequestError`, including invalid profile,
artifact overlap, identity mismatch, missing stage placement or capability,
target mismatch, and runtime artifact validation errors.

### `CandidateSessionFactory`

The trait is the dependency-neutral preparation contract:

| Method | Required operation |
| --- | --- |
| `reservation_evidence(device)` | Report the exact mechanism/evidence used for the device reservation. |
| `realize_candidate(request)` | Load every image, create every candidate resource, and perform no deferred initialization. |
| `warm_maximum_concurrency(session, candidate, pass)` | Execute the candidate's complete maximum-concurrency trace. Pass numbers begin at one. |
| `capacity_snapshot(session, topology, discovery)` | Observe capacity only after a complete warm pass. |
| `destroy_candidate(session)` | Deterministically release partial or complete resources. |

`CandidateFailure` distinguishes a candidate rejection, a fatal physical
error, and an explicitly unavailable pre-final implementation.
`ValidatedCandidateFactory` validates the request before delegating, stores
the candidate, topology, discovery, and reservation identities inside
`ValidatedCandidateSession`, enforces pass ordering and candidate identity,
and validates every returned capacity ledger against the same topology and
reservations. `UnavailableCandidateFactory` fails closed rather than
creating a fallback implementation.

## CUDA adapter

[`src/cuda.rs`](../src/cuda.rs) implements the sealed executor backend over
the CUDA Driver API. `CudaBinding` borrows an already-created `Context` and
stores its `DeploymentIdentity`, queue ceiling, and display-connector count.
`available_bytes` reads the driver's current free-memory counter. The crate
constant `CUDA_MAXIMUM_SUBMISSION_QUEUES` is `32`, the bounded native ceiling
used when probe data cannot expose a finite stream-count attribute.

### CUDA resource ownership

`CudaResources` owns one `ExecutionPlan`, task contracts, prepared-task IDs,
per-device resources, and a poisoned bit. Each `DeviceResources` owns:

* nonblocking `Stream` values for exactly the queue slots used by the selected
  tasks;
* one available or active `Event` per used completion slot;
* loaded logical functions and one stable `Module` per distinct cubin digest;
* one fixed `ParameterBlock` per completion slot, sized to the largest ABI;
* one four-byte pinned metric buffer per metric task;
* one pinned staging buffer sized by the finalized per-device staging entry;
* the exact admission image contract;
* one pre-sized egress `Vec<u8>` per device-to-external exit task; and
* optional device scratch.

`realize` rejects duplicate, missing, and unexpected bindings, checks the
queue ceiling, requires an `EnforcedQuota` reservation, and validates each
context's device UUID and compute capability against its deployment identity.
For each artifact it checks CUDA compatibility and cubin inspection for the
exact compute capability and entry symbol. Images with the same digest are
loaded once, while each logical artifact retains its own ABI/function lookup.
The direct finalized path does not allocate a finalized arena until the
executor's `allocate_arena` call. Candidate stabilization uses separate
temporary arenas, which are released before final handoff.

The backend state is one-shot:

```text
Ready(bindings, artifacts)
       │ bind_resources or bind_partition
       ▼
Bound (resource returned)

Prepared(CudaPreparedResources) -- validated handoff --> Bound
Warmed(CudaResources) ---------- validate_handoff ----> Bound
```

The implementation replaces the prior state with `Bound` before attempting
the handoff. A second bind, a mismatched finalized partition, an unvalidated
candidate, or a reused finalized resource returns `Error::BackendState`.
The production local path uses `Warmed`; the direct `Prepared` path remains
for the corresponding one-shot handoff contract.

### CUDA pending and work

`CudaPending` records task, phase, work class, device, queue, completion,
native state, completion action, and a terminal flag. Its native state is
`Ready`, `Active(Event)` or `Active(Stream)`, and `Terminal`. A loop token may
be submitted when it is ready, or rearmed after terminal completion. An active
token cannot be submitted twice.

Before a token is returned, `prepare_pending` checks the immutable task
contract, planned submission slots, and realized queue/completion objects. A
submission then validates the same contract again and performs exactly one of
these operations:

| Work | Native operation |
| --- | --- |
| `InitAdmission` | Copy the packed image into pinned staging and enqueue H2D to the resolved destination, guarded by a completion event. |
| `Calculation` | Fill the fixed `ParameterBlock` with checked arena pointers, optional fault flag, run ID, loop iteration, and element count, then enqueue a Driver launch. |
| `InternalTransfer` | Enqueue same-context device-to-device copy. Cross-owner or cross-context routes belong to the staged bridge. |
| `Metric` | Enqueue a four-byte D2H copy into the task's pinned metric buffer. |
| `ExitTransfer` | Enqueue device-to-external D2H into pinned staging; terminal completion copies staging into the preallocated egress vector. |

All arena ranges, offsets, bytes, ABI arguments, queue IDs, completion IDs,
route links, and lane claims are checked before the native call. Native calls
are the only unsafe boundary. The launch parameter helper in
[`src/cuda_ffi.rs`](../src/cuda_ffi.rs) owns a boxed value array, pointer array,
and buffer keepalive list; one reviewed lifetime erasure widens the driver's
phantom operation lifetime while the resource owner prevents teardown before
terminal completion.

`poll` is nonblocking. Event or stream status maps to
`BackendPoll::Pending` or terminal completion. Terminal completion releases
the event slot and decodes an `F32` or `I32` metric, or copies an egress
buffer. An asynchronous driver error poisons the resource. `CudaPending::Drop`
leaks an abandoned active native token instead of allowing the referenced
operation to be destroyed prematurely.

The CUDA `ExitTransfer` submission branch accepts only device-to-external
egress. A finalized device-to-device task in the Exit phase therefore fails
with `UnsupportedTransfer` when it reaches this adapter; same-context
device-to-device movement is implemented only by the `InternalTransfer`
branch.

The CUDA adapter reports `supports_loop_repetition = true` and
`supports_same_queue_pipelining = true`. Rearm only resets a terminal loop
token; it does not create a stream, event, staging buffer, or argument block.

### CUDA teardown and evidence

Arena release first requires a healthy resource and exact device identity.
Resource destruction requires every stream to report idle and every completion
event to be available. It destroys events and streams, drops logical function
records, unloads modules, frees metric buffers and pinned staging, and frees
optional scratch. `CudaResources::destroy` currently refuses a poisoned
resource, which is the poisoned-teardown issue recorded in
[`INTEGRATION_REQUIRED.md`](../INTEGRATION_REQUIRED.md).

The adapter reports per-device evidence as:

```text
backend = Cuda
image_loads = distinct cubin modules
entry_lookups = logical artifact function records
queues = realized CUDA streams
completion_objects = realized CUDA events
persistent_allocations = metric buffers + optional scratch + staging
```

## HSA adapter

[`src/hsa.rs`](../src/hsa.rs) implements the sealed executor backend over
ROCr/HSA. `HsaBinding` borrows an HSA `Session` and an exact CPU
`DiscoveredAgent` used for kernarg and fine-grained host allocations. It also
stores target ID, code-object version, queue packet count, queue ceiling, and
display-connector count. `validate_binding` requires a CPU allocator with an
allocatable kernarg pool, an allocatable fine-grained pool, and an exact target
advertised by the session.

### HSA resource ownership

`HsaResources` owns the execution plan, task contracts, prepared-task IDs, a
pre-final pending-token pool, per-device resources, and a poisoned bit. Each
HSA device owns:

* a `Queue<..., ...>` for each used queue slot, configured as a single
  producer with the discovered packet count;
* available or active completion-slot records;
* one loaded logical `Kernel` record per artifact and one `Executable` per
  distinct HSACO digest;
* a host-accessible kernarg allocation and host byte buffer per completion
  slot;
* one four-byte fine-grained metric allocation per metric task;
* exact fine-grained staging and the admission contract;
* a pre-sized egress vector per device-to-external exit; and
* optional coarse scratch.

HSA realization validates duplicate/missing/unexpected bindings, queue
capacity, enforced reservations, target and memory-pool contracts, and each
artifact's target/code-object identity. It inspects every logical entry in a
shared HSACO bundle, maps the inspected runtime symbol to a kernel, and checks
kernarg segment size/alignment. Finalized HSA resource bounds become an
immutable resource envelope, including dynamic private bytes when the loaded
kernel uses a dynamic call stack. The candidate path also prepares one HSA
pending signal per selected task before finalization.

The backend state is analogous to CUDA: `Ready`, `Prepared`, `Warmed`, and
`Bound`, with a one-shot bind and exact bundle/task identity checks. The
production local path moves warmed resources into the final backend and binds
the finalized artifact resource envelopes without loading another executable.

### HSA pending and work

`HsaPending` records task, phase, class, device, queue, completion, a prepared
ROCr pending token, an action, and `Ready`, `Active`, or `Terminal` state. The
candidate pool is consumed once by `prepare_pending`; terminal warm tokens are
reset and returned to that pool by `recycle_pending`.

Submission checks the exact contract and dispatches:

| Work | Native operation |
| --- | --- |
| `InitAdmission` | Copy the packed image into fine-grained staging, then issue a prepared asynchronous host-to-device copy. |
| `InternalTransfer` | Issue a prepared asynchronous device transfer between checked resolved ranges. |
| `Calculation` | Fill the fixed kernarg bytes with checked pointers and dynamic arguments, verify queue capacity, then dispatch the prepared kernel with finalized geometry and private-byte bounds. |
| `Metric` | Issue a four-byte asynchronous device-to-host copy into a pre-realized fine-grained buffer. |
| `ExitTransfer` | Copy device data to fine-grained staging for external egress, or issue a device transfer for a device destination. |

Completion releases the exact completion slot, decodes a four-byte metric
after system-scope completion, and copies fine-grained egress into the caller
buffer when required. ROCr errors that indicate a poisoned session or fatal
asynchronous signal poison the resource. Other contract errors remain visible
without inventing a retry path. HSA loop tokens support repetition by calling
the prepared token's reset operation; same-queue pipelining is not enabled by
the HSA adapter.

### HSA teardown and evidence

Arena release requires a healthy resource and exact device identity. Destroy
first drops the pre-final pending pool, then checks every completion slot is
available and drains retirements for ten milliseconds. It closes queues,
drops logical kernel records, closes executables, closes kernarg and metric
allocations, closes staging, and closes optional scratch. As with CUDA, the
current `destroy` path refuses a poisoned resource; see
[`INTEGRATION_REQUIRED.md`](../INTEGRATION_REQUIRED.md).

HSA evidence uses the same shape as CUDA:

```text
backend = Hsa
image_loads = distinct HSACO executables
entry_lookups = logical artifact kernel records
queues = realized AQL queues
completion_objects = completion slots
persistent_allocations = kernarg slots + metric buffers + optional scratch + staging
```

## Staged cross-backend bridge

[`src/bridge.rs`](../src/bridge.rs) implements `StagedCrossBackend`, the
production `CrossBackendTransfer` and `CandidateCrossBackendTransfer`
implementation. It handles one finalized device-to-device hop whose source
and destination have different local ownership classes, including CUDA to a
different CUDA device. It does not handle external endpoints, calculations,
metrics, or multi-link routes.

The bridge contract requires:

* both endpoints are devices;
* the finalized route contains exactly one link;
* source and destination classes differ, or they are different CUDA devices;
* phase maps to `InternalTransfer` for Init/Loop and `ExitTransfer` for Exit;
* source/destination values, bytes, route, lane claims, submission slots, and
  local device classes remain identical from candidate through final handoff.

For each selected task, realization creates one source leg, one destination
leg, one host staging worker, and one prepared completion token per native leg.
The leg resources are:

| Endpoint class | Pre-realized resources |
| --- | --- |
| Host | No separate native staging. The host arena is read or written by the worker. |
| CUDA | Nonblocking stream, pinned host staging buffer, and completion event. |
| HSA | Fine-grained host allocation and prepared HSA pending signal. |

`StagedBridgePending` has the state sequence
`Ready -> Source -> Middle -> Destination -> Complete`. A Host source submits
the middle job immediately. A CUDA or HSA source first copies into staging;
after that leg completes, the worker runs. If the destination is native, the
worker completion starts the destination copy from staging. `submit` and
`poll` only enqueue or inspect these pre-realized operations and do not wait or
allocate.

The host worker uses a bounded synchronous channel and a detached thread named
`recipe-bridge-{task}`. Its atomic states are `Idle`, `Pending`, `Complete`,
and `Failed`. `Read` and `Write` call the host arena's exact bridge range I/O;
`Copy` copies between the two retained native staging allocations. Polling is
nonblocking, reset is allowed only after `Complete`, and shutdown closes the
sender then waits for the worker completion notification.

Candidate handoff validates the exact selected task set, final bundle,
resolved endpoints, device classes, and transfer fields. Candidate recycling
returns terminal leg tokens and resets the worker. Loop rearm resets terminal
native legs, the worker, middle-job state, and destination target in place.
Bridge teardown drops prepared tokens, closes each worker, and destroys the
destination and source leg resources while retaining the first error.

`RejectCrossBackend` is the homogeneous policy. It reports loop repetition as
supported but rejects any nonempty cross-backend task set at bind or candidate
realization. This is a closed policy, not a fallback bridge implementation.

## Local composition and ownership

[`src/local.rs`](../src/local.rs) composes Host, CUDA, HSA, and a bridge into
one sealed executor backend. `LocalArena` and `LocalArenaRef` preserve the
class and device of every arena. `LocalArenaSet` is a read-only view over the
executor-owned arena map; it performs no allocation and projects only the
matching arena class to each child adapter.

### Partitioning rules

`classify_candidate` uses Draft values and `classify` uses finalized resolved
locations. Both produce immutable device owners and task owners:

| Finalized task shape | Owner |
| --- | --- |
| Calculation on CUDA | CUDA child |
| Calculation on HSA | HSA child |
| Calculation on Host or absent device | Reject with `UnsupportedCalculation` |
| Metric | Owner of its resolved value device |
| External -> Device admission or Device -> External exit | Owner of the device endpoint |
| Device -> Device on Host | Host child |
| Device -> Device on HSA class | HSA child |
| Same-device CUDA transfer | CUDA child |
| Different CUDA devices or mixed classes | Bridge |
| External -> External | Reject, no local owner |

Executor-visible transfers must be planner-expanded to at most one link. A
bridge transfer must have exactly one link. Queue and completion slots may not
be shared by different child owners. Runtime artifacts are partitioned by the
GPU calculation owner, and one artifact ID may not be assigned to both CUDA
and HSA.

### Candidate factory and stabilization

`LocalCandidateFactory` owns optional `HostBackendConfig`, borrowed CUDA/HSA
bindings, a cloneable bridge, and a `LocalCandidateStabilizer`. `fail_closed`
uses `UnavailableLocalStabilizer`, so warm or capacity operations return
`PreFinalRealizationUnavailable`. `production` uses `NativeLocalStabilizer`.

`realize_candidate` performs this order:

1. validate the generic candidate request;
2. classify every device and task and partition runtime artifacts;
3. capture each device's current available bytes once and require the exact
   reservation to fit as user headroom;
4. pre-realize host resources and pending slots;
5. pre-realize CUDA resources and pending contracts;
6. pre-realize HSA resources, pending signals, and pending pool; and
7. pre-realize bridge legs and workers.

Later failures use explicit stage-local cleanup and retain the first error.
The fully realized candidate/session destroy paths release bridge, HSA, CUDA,
and host objects in that order. The returned `LocalPreparedSession` contains
no finalized arena offsets and starts in `Realized` stabilization state.

Each warm pass builds one provisional bundle, binds the prepared child
resources, allocates the candidate arena layouts once, and runs a dependency
and schedule-window trace through Init, one Loop iteration, and Exit. It
prepares, submits, polls, collects external warm exits, and recycles every
pending token. A stalled trace or a nonterminal pending token is a fatal local
state error. Passes must be sequential and name the same candidate.

After each complete warm pass, `capacity_snapshot` releases the warm arenas
and observes each local owner's current available bytes. The first complete
observation is anchored. Runtime overhead is the initial available counter
minus the capped live counter, and recipe-usable bytes are the capped live
counter minus the exact reservation headroom. Later observations return the
anchored ledger, so display or allocator drift cannot rewrite the scheduler
contract.

### Final handoff

`LocalPreparedSession::into_backend` requires `Observed` stabilization, exact
topology/discovery/Draft/candidate identity, exact tasks, kernels, build
recipes, resources, init images, reservations, and artifact identities, and no
warm arenas remaining. It asks the bridge and each child resource to validate
the same finalized handoff, then moves those warmed objects into:

```text
LocalBackend {
    host: Option<HostBackend>,
    cuda: CudaBackend,
    hsa: HsaBackend,
    bridge: PreparedBridge,
    declared_devices,
    native_evidence: default,
}
```

No second module/executable load, queue creation, allocator call, or native
realization is performed by this transition. A failed validation destroys the
session instead of returning a partially bound backend.

### `LocalBackend` executor implementation

`LocalBackend` implements the sealed `recipe_executor::Backend` trait with
`LocalArena` and `LocalPending`. Its methods delegate directly according to
the immutable owner map:

| Backend method | Local operation |
| --- | --- |
| `bind_resources` | Classify the finalized bundle, bind Host/CUDA/HSA child partitions, and bind the bridge. |
| `prepare_pending` | Return a `Host`, `Cuda`, `Hsa`, or `Bridge` token for the task owner. |
| `allocate_arena` | Allocate the exact layout from its owner and wrap it as `LocalArena`. |
| `submit` | Validate pending/task owner and delegate closed work to the owning child. |
| `submit_loop_iteration` | Rearm the owning loop token, then call the same submit path. |
| `poll` | Delegate a nonblocking poll and append exactly one physical poll status. |
| `collect_exit` | Delegate Host/CUDA/HSA external collection; reject bridge external exits. |
| `release_arena` | Validate device and class ownership, then release through the child. |
| `destroy_resources` | Gather CUDA/HSA evidence, then destroy bridge, HSA, CUDA, and host in order. |

The composite reports loop repetition if its bridge does, and permits same
queue pipelining only for CUDA-owned tasks. Physical accounting is recorded at
the composite boundary, not duplicated in child logical state. Successful
destruction replaces the default evidence with `NativeExecutionEvidence::completed`.

### Constructor paths

The constructors make the preparation boundary visible:

| Constructor | Resulting path |
| --- | --- |
| `CudaBackend::new` or `HsaBackend::new` | A `Ready` one-shot child. Its first executor bind validates the complete finalized partition and realizes child resources directly. |
| `LocalBackend::new` | A direct `Ready` composite around Host, CUDA, HSA, and the supplied bridge. It is useful only when the caller already owns the exact runtime images and bindings. |
| `LocalCandidateFactory::fail_closed` | A candidate factory whose stabilizer rejects warmup and capacity observation as unavailable. |
| `LocalCandidateFactory::production` | A candidate factory using `NativeLocalStabilizer` for the complete warm and measured path. |
| `LocalPreparedSession::into_backend` | The production transition from warmed candidate resources to a `LocalBackend` whose children are already in the `Warmed` handoff state. |
| `RejectCrossBackend` | A zero-resource bridge policy for deployments where any cross-owner transfer must be rejected. |

The private `from_prepared` and finalized handoff variants remain available to
the child implementations for the same one-shot contract, but the current
production local flow validates warmed resources and moves them directly.

## Physical accounting

[`src/accounting.rs`](../src/accounting.rs) is intentionally small. It pushes
one `PhysicalCall` into the executor's fixed `PhysicalCallBatch` and maps a
batch overflow to `Error::PhysicalAccountingOverflow`.

The submission mapping is exact:

| `BackendWork` | Physical call |
| --- | --- |
| `InitAdmission` | `AdmissionChunk { task, destination.device, bytes, chunk_index: 0 }` |
| `Calculation` | `SubmitCalculation { task }` |
| `InternalTransfer` | `SubmitInternalTransfer { task }` |
| `Metric` | `SubmitMetric { task, slot }` |
| `ExitTransfer` | `SubmitExitTransfer { task }` |

Every adapter records one `PhysicalCall::Poll` for each `poll`, with
`Pending`, `Complete`, or `Failed` status. Bind, pending preparation, arena
allocation, exit collection, arena release, and resource destruction are
recorded at the backend boundary. The batch never grows and accounting does
not become a second scheduler.

## Evidence

[`src/evidence.rs`](../src/evidence.rs) defines `NativeBackendKind::{Cuda,
Hsa}`, `NativeDeviceExecutionEvidence`, and `NativeExecutionEvidence`.
Device records count the resources retained immediately before teardown:

| Field | Meaning |
| --- | --- |
| `device` | Recipe device identity. |
| `backend` | Driver family that owns it. |
| `image_loads` | Distinct CUDA modules or HSA executables. |
| `entry_lookups` | Logical artifact entry records resolved from those images. |
| `queues` | Native queue or stream count. |
| `completion_objects` | Native completion slots/events. |
| `persistent_allocations` | Backend staging plus metric/kernarg and optional scratch allocations counted by the adapter. |

`NativeExecutionEvidence::completed` sets `loop_realization_calls` to zero,
`teardown_completed` to true, and `live_resources_after_teardown` to zero.
The running API exposes no realization operation, so the report is the
bounded post-run contract consumed by acceptance and completed execution
reports. Acceptance checks these values independently, including one real
CUDA image load where the workload requires it.

## Consumers

The direct package consumers are `recipe-native-probe`, `recipe-prepare`,
`recipe-training`, and the root `recipe` crate. The dependency graph can be
seen with `cargo metadata --no-deps`; there is no reverse dependency from
native-executor into those consumers.

### `recipe-native-probe`

`native-probe/src/bindings.rs` reopens the exact measured CUDA devices and HSA
agents, creates contexts/sessions, selects an exact HSA CPU allocator, and
constructs borrowed `CudaBinding` and `HsaBinding` values. It rejects missing,
duplicate, ambiguous, or changed measured origins. Binding lifetimes are
limited to one preparation callback and cannot leak a runtime handle into a
later dynamic placement path.

### `recipe-prepare`

`NativeExecutorDriver` wraps a `ValidatedCandidateFactory` and implements the
preparer's `CandidateDriver`. It reports `EnforcedQuota`, forwards exact
reservation evidence, converts `NativeArtifact` pairs into
`CandidateArtifact`, maps candidate failures, and forwards warm and capacity
operations. `NativeCandidateRealizer` compiles deferred stages before calling
the driver, runs the configured stabilization passes, stores immutable image
identities and capacity snapshots, and returns `PreparedNativeSession`.
`PreparedNativeSession::into_parts` transfers the warmed session and images
without retaining a compiler, loader, allocator, or mutable driver.

### `recipe-training` and root inference

The root training and inference paths construct:

```text
StagedCrossBackend::new(cuda_bindings, hsa_bindings)
LocalCandidateFactory::production(host_config, cuda_bindings, hsa_bindings, bridge)
NativeExecutorDriver::new(factory)
NativeCandidateRealizer::new(profile, compiler, driver)
Preparer::prepare_program(...)
PreparedNativeSession -> LocalPreparedSession::into_backend
PreparedRun -> initialize -> start_loop -> poll -> exit
```

Training can observe bounded user metrics and request a graceful stop, which
the executor accepts only after a complete loop iteration. Inference rejects
loop external transfers and user metrics, requires exactly one loop iteration,
and collects its output only in Exit. Both paths retain native kernels and
`NativeExecutionEvidence` in their completed report. Checkpoints are created
only after Exit and ordered native teardown have succeeded.

### Acceptance

The acceptance runner checks native evidence, not a log string or an exit
status. It requires zero forbidden loop realization calls, completed teardown,
zero live resources after teardown, the expected driver family, and nonzero
image, entry, queue, completion, and persistent-allocation counts. The actual
CUDA/HSA driver, ISA, and complete public workload are required for acceptance;
compilation alone does not prove this crate's behavior.

## Failure vocabulary and failure boundaries

[`src/error.rs`](../src/error.rs) is non-exhaustive and keeps failures visible:

| Category | Representative errors |
| --- | --- |
| Identity and plan | `DuplicateArtifact`, `MissingArtifact`, `UnexpectedArtifact`, `ArtifactMismatch`, `DuplicateDevice`, `MissingDevice`, `UnexpectedDevice`. |
| Native slots and ownership | `MissingQueue`, `MissingCompletion`, `CompletionBusy`, `ResourceContention`, `ArenaMismatch`, `ValueMismatch`. |
| Work protocol | `UnsupportedTransfer`, `UnsupportedLoopContract`, `Protocol`, `BackendState`, `BackendPoisoned`. |
| Capacity and accounting | `SubmissionQueueLimitExceeded`, `IntegerOverflow`, `PhysicalAccountingOverflow`. |
| Driver and image | `Cuda`, `CudaContract`, `Hsa`, `HsaSymbolLookup`, `Kernel`. |

CUDA and HSA contract checks reject malformed work before native submission.
Driver/session failures that can invalidate asynchronous ownership poison the
child resource; subsequent operations fail with `BackendPoisoned`. The local
boundary wraps child failures as `LocalError::{Host, Native, Bridge}` and
retains the first teardown error while still attempting later children.
Bridge failures identify the task and distinguish immutable contract mismatch,
state mismatch, worker disconnect/panic, native staging errors, and integer
conversion failures.

The current known limitation is explicit rather than hidden: poisoned CUDA or
HSA resources currently refuse arena release and destruction because those
methods require a healthy resource. [`INTEGRATION_REQUIRED.md`](../INTEGRATION_REQUIRED.md)
tracks the remaining work to make cancellation, terminal observation, release,
and total ordered teardown possible after device or transport failure. No
fallback destruction path is implemented in this crate today.

Failure boundaries by lifecycle phase are:

| Phase | Failure is observed as |
| --- | --- |
| Request validation | `CandidateFailure::CandidateRejected` with topology, Draft, reservation, or artifact detail. No native resource has been created. |
| Candidate realization | `CandidateFailure::Fatal` or `PreFinalRealizationUnavailable`; already-created host, native, and bridge objects are explicitly cleaned up. |
| Warm trace | Candidate mismatch, illegal phase/endpoint, stalled dependency/window trace, pending-token mismatch, native submission failure, or nonterminal completion. The session remains rejectable and is destroyed on failure. |
| Capacity observation | Topology/discovery identity drift, missing reservation/capacity owner, insufficient headroom, or live capacity below the reservation. |
| Final handoff | `LocalError::BackendState`, child `Error::BackendState`, or bridge contract failure for any identity, task set, image, manifest, slot, or recycled-token mismatch. |
| Executor bind and pending preparation | Duplicate/missing/unexpected owner, artifact, queue, completion, or task, or a second one-shot bind/preparation. |
| Submission | `Protocol`, `ValueMismatch`, `ArenaMismatch`, `UnsupportedTransfer`, ABI/resource contention, bridge state, or driver/session errors. CUDA poisons on any submission error; HSA poisons only errors classified as session/fatal asynchronous failures. |
| Poll and completion | Nonblocking pending/complete status, or a driver/session/worker error. Native asynchronous errors poison the affected child; a completion token is not silently reused. |
| Arena release | Device/class mismatch, active resource use, poisoned child, or underlying host/native close error. |
| Destroy | Active stream/queue, checked-out completion, worker shutdown, retirement, unload, close, or poisoned-resource error. Destruction still attempts later children and retains the first error at composite boundaries. |

## Validation commands

These commands validate structure and the documented module boundary. They do
not replace hardware acceptance:

```bash
cargo check -p recipe-native-executor
cargo check -p recipe-native-executor --all-targets
cargo +nightly fmt --all -- --check
```

Runtime evidence must come from the public training or inference entrypoint on
the real measured CUDA or HSA system, followed by the complete Init, Loop,
Exit, release, and destroy lifecycle.
