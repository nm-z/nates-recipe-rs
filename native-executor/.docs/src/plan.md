---
file: native-executor/src/plan.rs
crate: recipe-native-executor
package_version: 0.1.0
edition: "2024"
role: immutable-native-plan-validation-and-projection
intent: >
  Authenticate runtime images against one immutable FinalizedBundle and project
  every finalized task onto one native device, queue, and completion pair.
authority:
  - native-executor/src/plan.rs
  - core/src/plan.rs
  - core/src/schedule.rs
  - executor/src/backend.rs
  - executor/src/executor.rs
  - native-executor/src/cuda.rs
  - native-executor/src/hsa.rs
---

# `native-executor/src/plan.rs`

## Boundary

`plan.rs` is the native adapter's immutable admission gate. It accepts a
validated [`recipe_core::FinalizedBundle`](../../../core/src/plan.rs) and runtime
images that were produced or inspected for the current native target. It does
not allocate an arena, create a queue, load a module, select a device, or
construct an executor work record. Its product is an `ExecutionPlan` containing
only the contracts that CUDA and HSA need after Finalize:

```yaml
execution_plan:
  artifacts:
    type: BTreeMap<ArtifactId, ArtifactContract>
    meaning: finalized identity paired with one authenticated runtime image
  submissions:
    type: BTreeMap<TaskId, PlannedSubmission>
    meaning: immutable native device and SubmissionSlots for every finalized task
  devices:
    type: BTreeSet<DeviceId>
    meaning: devices whose arenas are present in the finalized bundle, or the explicit partition
runtime_image:
  bytes: Arc<[u8]>
  digest: ArtifactDigest
  digest_source: SHA-256 of bytes at RuntimeImage::new
```

The distinction between this plan and the other finalized projections is
important:

| Product | Owner | Retained data | Native allocation or submission? |
| --- | --- | --- | --- |
| `FinalizedBundle` | `recipe-core` | Tasks, artifacts, build recipes, resolved values, arena offsets, init-image manifests, and resource manifest | No |
| `ExecutionPlan` | `plan.rs` | Authenticated runtime artifacts, task submission assignments, and device set | No |
| `PreparedTask` | `recipe-executor` | Resolved value locations, transfer endpoints, phase, dependencies, iteration domain, and work payload | No |
| `BackendWork` | `recipe-executor` | One closed submission record for the current operation, including run and iteration values | No, it is an input to `Backend::submit` |
| `TaskContract` | `cuda.rs` or `hsa.rs` | Phase, `WorkClass`, slots, admission manifest, route, and lane claims for one backend partition | No |
| `DeviceResources` | `cuda.rs` or `hsa.rs` | Real queues, completion objects, loaded artifacts, staging, metrics, egress, scratch, and native state | Yes, before `init` |

`ExecutionPlan` is therefore narrower than a complete backend work projection.
The executor resolves values and endpoints from the same immutable bundle, while
the native backend uses the plan to reject a work record whose task or slots do
not match the finalized contract.

## Source structure

The module has one public runtime-image vocabulary and one crate-private plan
vocabulary:

| Source item | Visibility | Contract |
| --- | --- | --- |
| `RuntimeArtifactKind` | public | A CUDA driver identity or an HSA target ID and code-object version. |
| `RuntimeImage` | public | Shared image bytes and their content digest. |
| `RuntimeArtifact` | public type, crate-private fields | Artifact ID, image bytes, digest, inspected `KernelAbi`, and backend kind. |
| `ArtifactContract` | crate-private | One `ArtifactIdentity` from Finalize paired with one `RuntimeArtifact`. |
| `PlannedSubmission` | crate-private | Task ID, native device, and immutable queue/completion slots. |
| `InitImageContract` | crate-private | Device, packed init-image value, and byte count, copied from `InitDataImage`. |
| `ExecutionPlan` | crate-private | BTree maps and set consumed by CUDA and HSA resources. |

The module exports only `RuntimeArtifact`, `RuntimeArtifactKind`, and
`RuntimeImage` from `native-executor/src/lib.rs`. `ExecutionPlan`,
`PlannedSubmission`, `ArtifactContract`, and `InitImageContract` remain
internal to the native adapter.

## Runtime image records

### `RuntimeImage`

`RuntimeImage::new(bytes)` stores the supplied `Arc<[u8]>` and computes one
`ArtifactDigest` from those bytes. `bytes()` returns the same shared allocation
and `digest()` returns the precomputed digest. The constructor does not inspect
ELF structure or infer a target. ELF and ABI inspection belongs to the artifact
builder and CUDA/HSA realization paths.

### `RuntimeArtifact`

`RuntimeArtifact::new(id, bytes, abi, kind)` delegates to `from_image` after
constructing a `RuntimeImage`. `from_image` moves the shared bytes and digest
into the record and retains the supplied `KernelAbi` and
`RuntimeArtifactKind`. Accessors expose the ID, bytes, digest, ABI, and kind;
the fields themselves remain crate-private so the native crate controls how a
runtime image enters validation.

`RuntimeArtifactKind` has exactly two forms:

```yaml
runtime_artifact_kind:
  Cuda:
    identity: recipe_cuda::ArtifactIdentity
    checked_against:
      backend: nvidia-cuda-driver
      abi: elf64-cubin
      architecture: identity.target.to_string()
      digest: identity.sha256
  Hsa:
    target_id: String
    code_object_version: u8
    checked_against:
      backend: amd-rocr-hsa
      abi: elf64-amdgpu-code-object-v<code_object_version>
      architecture: target_id
```

The CUDA form carries the driver's deployment identity, including the digest
that the driver inspected for the image. The HSA form carries the exact target
and code-object version selected by the HSA inspection path. A runtime image
cannot be accepted as an untyped or backend-neutral artifact.

## Construction algorithm

### Full bundle validation

`ExecutionPlan::validate(bundle, runtime_artifacts)` derives the device set
from every `ArenaLayout.device` in `bundle.arena_layouts()`. It then delegates
to `validate_scoped` with `tasks = None`. Full validation consequently requires
one runtime artifact for every `bundle.artifacts()` identity and validates every
task submission and every arena device.

### Partition validation

`ExecutionPlan::validate_partition(bundle, runtime_artifacts, devices, tasks)`
passes an explicit device set and task set to `validate_scoped`. The partition
is used only for artifact selection and task-device admission:

1. `selected_artifacts` is the set of artifact IDs named by selected
   calculation tasks. Transfers and metrics do not select runtime images.
2. Only identities in that selected set are consumed from the supplied runtime
   artifact map and checked with `validate_artifact_contract`.
3. `plan_submissions` still walks every task in the bundle, not only selected
   tasks, and constructs a submission map for the complete bundle.
4. Every selected task must have a planned submission and its planned device
   must be present in the explicit `devices` set.

This means a partition plan may retain a complete task-to-submission map while
retaining only the runtime artifacts needed by its selected calculations. A
selected artifact's ABI is checked against every calculation in the bundle that
names that artifact, because `validate_artifact_contract` iterates all bundle
tasks after the identity is selected.

### `validate_scoped` steps

The exact order is observable in the failure that is returned first:

```text
runtime_artifacts
  -> runtime_by_id (reject duplicate IDs)
  -> selected_artifacts (partition only)
  -> artifact identity loop
       remove matching runtime ID
       reject missing runtime image
       validate identity, target, ABI, and calculation contract
  -> reject first remaining runtime ID as unexpected
  -> plan_submissions(bundle)
  -> selected task existence and device membership checks
  -> ExecutionPlan { artifacts, submissions, devices }
```

`BTreeMap` and `BTreeSet` make duplicate, missing, and unexpected selection
deterministic. `reject_unexpected_artifact` reports the lowest remaining
`ArtifactId` because the input map is ordered.

### Plan accessors

`runtime_artifacts()` iterates the runtime side of the artifact contracts in
artifact-ID order. `artifact_contract(id)` returns the complete identity and
runtime pair for one artifact. `submission(task)` returns a copied
`PlannedSubmission`; callers cannot mutate the map through this accessor.
`devices()` iterates the ordered device set. No accessor exposes a finalized
value location, route, or arena offset because those remain owned by the
`FinalizedBundle` and the executor's prepared work.

## Artifact contract validation

### Identity and image checks

`validate_artifact_contract` first calls `validate_runtime_artifact`, then scans
all bundle tasks. For each calculation whose `calculation.artifact` equals the
identity being checked, it requires the task's `kernel_template` to equal
`identity.kernel_template` and validates the calculation ABI. Non-calculation
tasks are ignored by this function.

The runtime identity checks are fail-closed and all return
`Error::ArtifactMismatch { artifact: identity.id, detail }`:

| Check | Required equality or range | Failure detail category |
| --- | --- | --- |
| Runtime ID | `runtime.id == identity.id` | Runtime ID differs from manifest ID. |
| Image digest | `runtime.digest.bytes() == identity.digest.bytes()` | Image SHA-256 differs from finalized identity. |
| Entry symbol | `runtime.abi.entry_symbol == identity.entry_symbol` | ABI entry differs from finalized entry. |
| Manifest format | `identity.format == identity.target.abi` | Artifact format differs from target ABI. |
| Workgroup width | Nonzero and no greater than `identity.resources.maximum_workgroup_lanes` | ABI workgroup size is zero or exceeds finalized maximum. |
| Backend target | Kind-specific checks in `validate_target` | Runtime image is paired with a nonmatching finalized target. |

The format check is an internal finalized-identity consistency check. It does
not inspect an ELF header. The ABI's `argument_bytes` and `argument_alignment`
fields are also not inspected by `plan.rs`; backend-specific artifact
inspection and submission paths consume those fields later.

### CUDA target checks

For `RuntimeArtifactKind::Cuda { identity: driver }`, all of the following are
required:

```yaml
cuda_target:
  identity.target.backend: nvidia-cuda-driver
  identity.target.abi: elf64-cubin
  identity.target.architecture: driver.target.to_string()
  driver.sha256: runtime.digest.bytes()
```

The driver architecture and image digest therefore authenticate both the
current deployment and the exact bytes loaded by the CUDA realization path.

### HSA target checks

For `RuntimeArtifactKind::Hsa { target_id, code_object_version }`, the expected
ABI is formed as `elf64-amdgpu-code-object-v<code_object_version>`. The
finalized target must use backend `amd-rocr-hsa`, that exact ABI label, and the
same architecture string as `target_id`. The HSA branch has no separate driver
digest field; the outer runtime-versus-manifest digest check remains mandatory.

## Calculation ABI validation

`validate_calculation_abi` compares one `KernelAbi` with one finalized
calculation task and its immutable stage contract. It accepts only a
`TaskKind::Calculation`; passing another task kind returns an
`ArtifactMismatch` for that artifact.

### Expected stage contract

When `bundle.artifact_build(calculation.artifact)` exists, the build recipe is
authoritative:

* `build.kernel_template` must equal `calculation.kernel_template`.
* `abi.workgroup_lanes` must equal `build.dispatch.workgroup_lanes`.
* Expected element count is `build.dispatch.logical_lanes`.
* Expected buffer operands are all non-fault bindings whose access reads,
  followed by all non-fault bindings whose access writes. Each expected tuple
  is `(dtype, view.storage_bytes, BufferAccess::{Read,Write})`.
* A `ReadWrite` or `ReadWriteAtomic` binding contributes once to the read
  sequence and once to the write sequence, because the two filters are
  independent. The fault binding is excluded from both sequences.

If no build recipe exists, the function falls back to the finalized kernel
template named by `calculation.kernel_template`:

* Expected element count is `template.index_space.elements()`.
* Expected operands are template inputs as reads, followed by template outputs
  as writes, using each input or output's storage byte count.
* A missing template is an `ArtifactMismatch` with detail that the task has
  neither a finalized build recipe nor a kernel template.

The fallback is a read of finalized metadata, not a second compilation path.

### ABI shape and argument order

The canonical argument grammar is:

```text
abi.arguments = buffer(inputs, outputs)
               + optional FaultFlag
               + optional RunId
               + optional LoopIteration
               + ElementCount
```

The exact checks are:

1. `abi.elements` equals the expected stage element count.
2. `buffer_arguments = inputs.len() + outputs.len()` must not overflow.
3. The ABI contains at most one `RunId` and at most one `LoopIteration`.
4. The expected total argument count is
   `buffer_arguments + fault_flag_present + run_id_count + loop_iteration_count
   + 1`, with checked additions. `abi.arguments.len()` must equal it.
5. Every calculation input and output is resolved through
   `bundle.value_location`. A missing location is an `ArtifactMismatch`.
6. The ordered resolved locations, expected operand tuples, and ABI arguments
   are zipped. Each ABI entry at a buffer position must be
   `KernelArgument::Buffer`.
7. Buffer dtype must equal both the resolved location dtype and expected stage
   dtype. Access must equal the expected read or write access.
8. Buffer alignment must be nonzero, a power of two, and divide the resolved
   `arena_offset`. Resolved backing bytes must equal expected storage bytes.
9. When `calculation.fault_flag` exists, its location must be on the
   calculation device, have `DType::I32`, exactly four bytes, and an offset
   divisible by four. The argument immediately after the buffer arguments must
   be `KernelArgument::FaultFlag`.
10. If one `RunId` exists, it must be at index
    `buffer_arguments + fault_flag_present`. If one `LoopIteration` exists, it
    must follow the run ID when present. The final argument must always be
    `KernelArgument::ElementCount`.

Repeated dynamic arguments, misplaced suffix arguments, a non-buffer in a
buffer position, a missing fault argument, a noncanonical final argument, and
all dtype, access, alignment, size, element, or count mismatches return
`Error::ArtifactMismatch`.

The function validates the ABI's ordered shape against finalized locations. It
does not compare the `ValueId` of each stage binding with the task input/output
vectors directly; the ordered location count, dtype, access, alignment, and
storage-byte checks are the boundary enforced here. The CUDA and HSA argument
fillers perform a second exact ordered walk before launching.

## Submission projection

`plan_submissions` creates one `PlannedSubmission` for every `bundle.tasks()`
entry. The task-kind mapping is:

| Finalized kind | Native device selected by `plan.rs` | Slots | Additional check |
| --- | --- | --- | --- |
| `Calculation` | `calculation.device` | `calculation.submission` | Queue and completion must belong to that device. |
| `Transfer` | `transfer_submission_device` | `transfer.submission` | Source device wins; for external ingress the destination device wins; external to external is invalid. |
| `Metric` | `bundle.value_location(metric.value).device` | `metric.submission` | Metric value must have a finalized location. |

`PlannedSubmission.task` repeats the map key so a copied record remains
self-identifying. The map is populated with `result.insert`; upstream
`FinalizedBundle` validation requires unique task IDs, and the backend
task-contract builders repeat that identity check at their own boundary.

### Transfer device rule

`transfer_submission_device(task, source, destination)` is deliberately small:

```yaml
transfer_submission_device:
  Device(source_device, source_value): source_device
  External -> Device(destination_device, destination_value): destination_device
  External -> External: error Protocol("external-to-external transfer has no native device")
```

For a device-to-device transfer, the source device supplies the native
submission device. `plan.rs` does not require the destination device to equal
the source device and does not inspect route links or lane claims. Native
backend task contracts and the local or worker partition then decide whether
that transfer is a same-device copy, a cross-backend bridge, or a remote
external leg.

### Queue and completion ownership

`validate_slots(bundle, task, device, slots)` performs two independent lookups:

* `bundle.resources().queue(slots.queue)` must exist and its `device` must equal
  the projected device, otherwise `Error::MissingQueue` is returned.
* `bundle.resources().completion(slots.completion)` must exist and its `device`
  must equal the projected device, otherwise `Error::MissingCompletion` is
  returned.

The function does not reserve or claim the slots. CUDA and HSA create native
objects for the IDs and later mark completion slots active while a pending
operation owns them. A slot may therefore pass this immutable check and still
produce `CompletionBusy` during a concurrent or pipelined submission.

## Call graph and consumers

The native plan is built through these real entrypoints:

```text
executor::PreparedRun::prepare
  -> backend.bind_resources(bundle)
     -> LocalBackend::bind_resources
        -> classify(bundle)
        -> CudaBackend::bind_partition(bundle, cuda_tasks)
           -> ExecutionPlan::validate_partition(...)
        -> HsaBackend::bind_partition(bundle, hsa_tasks)
           -> ExecutionPlan::validate_partition(...)
        -> bridge.bind(...)

direct CudaBackend::bind_resources
  -> ExecutionPlan::validate(bundle, runtime_artifacts)
  -> CudaResources::realize

direct HsaBackend::bind_resources
  -> ExecutionPlan::validate(bundle, runtime_artifacts)
  -> HsaResources::realize

candidate validation
  -> CandidateRequest::validate
     -> validate_runtime_artifact(identity, runtime)
```

`native-executor/src/candidate.rs` calls the exported-in-crate
`validate_runtime_artifact` before finalization. This catches target, digest,
entry, and workgroup contradictions while a candidate is still being warmed.
`ExecutionPlan::validate` repeats the runtime checks when resources bind to a
specific FinalizedBundle, then adds task ABI and submission checks.

CUDA and HSA each call `validate_partition` in four situations:

| Caller | Inputs | Purpose |
| --- | --- | --- |
| `bind_partition` from `Ready` | runtime artifacts, binding devices, selected tasks | Build a plan for an unprepared partition. |
| `PreparedResources::bind_candidate` | candidate runtime artifacts, realized device keys, selected tasks | Convert a warmed candidate to a finalized resource set. |
| `PreparedResources::validate_handoff` | candidate artifacts, realized device keys, selected tasks | Authenticate finalized handoff before consumption. |
| `Resources::validate_handoff` | runtime artifacts retained in the warm plan, realized device keys, selected tasks | Revalidate a warmed session against a finalized bundle. |

The direct `Backend::bind_resources` implementation validates the whole bundle
when the backend owns all runtime resources itself. A second bind is rejected by
the backend state machine before a new plan can be installed.

## Finalized bundle to backend work

`ExecutionPlan` does not itself create `BackendWork`. The executor performs the
full projection in `PreparedTask::new` and `PreparedTask::backend_work`:

| Finalized task | Prepared work | Backend work |
| --- | --- | --- |
| Loop `Calculation` | Device, kernel template, artifact, resolved input/output locations, optional resolved fault flag, and slots | `BackendWork::Calculation(CalculationWork)` with `RunId` and active `LoopIteration` |
| Loop `Metric` | Purpose, metric ID, metric slot, resolved value, and slots | `BackendWork::Metric(MetricWork)` with active `LoopIteration` |
| Init `External -> Device` transfer | Destination value location, destination device, bytes, and slots | `BackendWork::InitAdmission(InitAdmissionWork)` with packed host image bytes |
| Init or loop `Device -> Device` transfer | Resolved endpoints, bytes, route, lane claims, and slots | `BackendWork::InternalTransfer(TransferWork)` |
| Exit `Device -> Device` or `Device -> External` transfer | Resolved endpoints, bytes, route, lane claims, and slots | `BackendWork::ExitTransfer(TransferWork)` |

All loop calculations and metrics require an active iteration. A missing loop
iteration is an executor lifecycle error before the native adapter is called.
The executor sorts each phase by `(window.start, task.id)`, prepares one pending
token per task before `init`, and calls the backend through the closed
`Backend` trait. `BackendWork` has no compiler, loader, allocator, discovery,
or topology mutation variant.

The executor's phase checks reject calculations or metrics outside the loop,
loop transfers that are not internal, init transfers that are neither admission
nor internal movement, and exit transfers that admit external data. Native
`task_contracts` repeat the phase and work-class checks, which keeps a malformed
direct backend call from bypassing the executor's projection.

## Backend task contracts

CUDA and HSA derive a second map, `TaskContract` or `HsaTaskContract`, from the
same finalized tasks. The map contains:

```yaml
task_contract:
  phase: RunPhase
  class: WorkClass
  submission: SubmissionSlots
  admission: optional InitImageContract
  route: LinkId[]
  lane_claims: TransferLaneClaim[]
```

For calculations and metrics, the class is `Calculation` or `Metric`, slots are
copied, and route metadata is empty. For transfers,
`transfer_work_class` accepts only these combinations:

```yaml
valid_transfer_classes:
  - phase: Init
    source: External
    destination: Device
    class: InitAdmission
  - phase: Init or Loop
    source: Device
    destination: Device
    class: InternalTransfer
  - phase: Exit
    source: Device
    destination: Device or External
    class: ExitTransfer
```

Every other phase and endpoint combination is `Error::Protocol`. Init admission
also resolves both endpoints through `bundle.transfer_endpoints`, requires a
device destination, requires that device's `InitDataImage`, and requires image
value and bytes to equal the transfer. Internal and exit contracts copy the
finalized route and lane claims. At submit time both backends require the
`BackendWork` class and slots to equal this contract. Admission work must equal
the `InitImageContract`; transfer work must equal route and lane claims.

HSA additionally requires calculations and metrics to have `RunPhase::Loop` in
its task-contract builder. CUDA relies on the common executor phase projection
for that constraint, then applies the same class and slot checks.

## Queue and resource realization

### Device set and bindings

`CudaResources::realize` and `HsaResources::realize` consume an already
validated plan. They build `binding_by_device` from native bindings and require:

1. no duplicate binding device (`Error::DuplicateDevice`);
2. every plan device has a binding (`Error::MissingDevice`); and
3. no binding device exists outside `plan.devices()` (`Error::UnexpectedDevice`).

The plan's device set is therefore the exact join key between FinalizedBundle
arena layouts and native contexts or HSA sessions. Arena allocation occurs later
through `Backend::allocate_arena(layout)`, and the layout's device must already
be in this set.

### Scoped tasks and slot sets

Each realization builds `scoped_tasks` from all bundle tasks when no partition is
provided, or from the selected task set otherwise. It derives unique queue and
completion IDs from task submissions, then filters the finalized
`ResourceManifest` by both device and those IDs:

```text
queue_ids       = unique task.submission.queue for scoped tasks
completion_ids  = unique task.submission.completion for scoped tasks
device.queues   = resources.queues where slot.device == device and id in queue_ids
device.completions = resources.completions where slot.device == device and id in completion_ids
```

Before creating native queues, both backends count the distinct manifest queue
slots requested by the scoped tasks and compare that count with the discovered
maximum. Exceeding the limit returns `SubmissionQueueLimitExceeded`. A queue or
completion that passed `validate_slots` but was not realized for a scoped
partition fails later as a backend `Protocol` error when a pending token is
prepared.

### CUDA device resources

`native-executor/src/cuda.rs::realize_device` maps the finalized manifest to:

| Finalized source | Device resource |
| --- | --- |
| Used `QueueSlot` IDs on this device | Nonblocking CUDA Driver streams |
| Used `CompletionSlot` IDs on this device | Reusable completion events |
| `pinned_staging[device]` | One pinned host staging buffer, large enough for the init image |
| `init_images[device]` | `InitImageContract` retained as the admission identity |
| Nonzero `scratch[device]` | One device scratch buffer; zero or absent means no scratch allocation |
| Calculation artifact IDs on this device | One loaded `LoadedArtifact` per logical artifact and one CUDA module per distinct image digest |
| Calculation completion slots | Preallocated parameter blocks sized for each artifact ABI |
| Loop metric tasks whose value is on this device | Four-byte pinned metric buffers keyed by task ID |
| Exit device-to-external transfers on this device | Host egress byte vectors keyed by task ID |

Every CUDA runtime artifact is checked again against the deployment, inspected
cubin entry, and digest before loading. Distinct artifact IDs may share one
cubin image digest; the module is loaded once and each logical ABI entry is
looked up from it. A digest collision with different bytes is rejected.

### HSA device resources

`native-executor/src/hsa.rs::realize_device` maps the same finalized choices to:

| Finalized source | Device resource |
| --- | --- |
| Used `QueueSlot` IDs on this device | Single-producer HSA queues |
| Used `CompletionSlot` IDs on this device | Completion state entries |
| `pinned_staging[device]` | One fine-grained host staging allocation, large enough for the init image |
| Nonzero `scratch[device]` | One coarse HSA scratch allocation; zero or absent means no scratch allocation |
| Calculation artifact IDs on this device | `LoadedArtifact` entries and one executable per distinct HSACO image digest |
| Calculation completion slots | Preallocated host kernarg slots sized from the ABI |
| Loop metric tasks whose value is on this device | Four-byte fine-grained metric allocations keyed by task ID |
| Exit device-to-external transfers on this device | Host egress byte vectors keyed by task ID |

HSA groups runtime artifacts by digest, inspects every requested ABI in one
HSACO bundle, loads one executable per distinct image, and resolves each logical
entry symbol. Finalized resource bounds are attached to each loaded artifact
before calculation submission.

### Native submission uses the plan

On `prepare_pending`, CUDA and HSA look up the backend task contract and the
plan's `PlannedSubmission`. They require request phase, class, and optional
slots to equal the contract, then require the planned queue and completion IDs
to exist in the realized device maps. On `submit`, they repeat the plan lookup,
validate the pending token's task and slots, validate the backend task contract,
and dispatch by the closed `BackendWork` variant.

The native operation then uses finalized locations and the planned slots:

* Admission checks the planned device, init image identity, byte count, and
  staging capacity before copying host bytes to the resolved arena offset.
* Calculation looks up the planned artifact and completion-owned argument slot,
  fills buffer pointers from each `ResolvedValueLocation`, appends run ID,
  loop iteration, and element count in the validated ABI order, and submits on
  the planned queue.
* Internal transfer requires two resolved device endpoints. CUDA requires the
  source device, destination device, and planned device to be one context. HSA
  requires the source device and planned device to match, then validates both
  resolved arenas through the source session; its direct function does not add
  a separate destination-device equality check. Cross-backend or cross-machine
  movement is owned by the bridge or worker projection, not by this plan's
  direct native transfer operation.
* Metric requires one resolved four-byte value on the planned device and copies
  it to the preallocated metric slot.
* Exit transfer requires a resolved device source, a matching planned device,
  and either a device destination or external destination. External egress uses
  the preallocated staging and egress buffer, and collection happens only after
  terminal completion.

`checked_arena` in each adapter uses the resolved device and arena offset,
checks the requested range against the allocated arena, and rejects a missing
device, wrong arena identity, offset overflow, or out-of-bounds value. The
immutable plan's slot check is therefore followed by a runtime range check at
the final native boundary.

## Invariants

These are the invariants established by `plan.rs` or required immediately by
its consumers:

1. Runtime artifact IDs, bytes, image digests, entry symbols, target labels,
   backend labels, ABI labels, and workgroup bounds agree with the finalized
   artifact identity.
2. Every runtime artifact supplied to a full plan is consumed exactly once;
   partition validation consumes exactly the artifacts named by selected
   calculation tasks. Duplicate, missing, and extra images fail closed.
3. Every calculation using a selected artifact has an ABI whose element count,
   buffer count, ordered access and dtype, alignment, storage bytes, optional
   fault flag, optional dynamic arguments, and final element-count argument
   agree with finalized metadata.
4. Every finalized task has one deterministic `PlannedSubmission`. Its queue
   and completion IDs exist in the manifest and belong to the projected device.
5. A transfer has a native submission device. External-to-external transfers
   are impossible at this boundary and fail as protocol errors.
6. The explicit partition device set contains every selected task's planned
   device. Native bindings and later arena allocations must cover the same set.
7. The plan never owns mutable domain state. It cannot alter finalized arena
   offsets, value locations, routes, lane claims, resources, or task kinds.
8. All native queues, completion objects, images, argument blocks, staging,
   metrics, scratch, and egress storage are realized before `init`; loop submit
   and poll may only use those retained objects.
9. Completion slots are reusable only after terminal completion. A valid plan
   does not prevent a later `CompletionBusy`, queue backpressure, or poisoned
   backend error when a native resource is already active.
10. No compiler, loader, allocator, discovery, fallback backend, retry, or
    alternate artifact path is reachable through `ExecutionPlan`.

## Failure taxonomy

The plan's direct failures are `recipe_native_executor::Error` values. The
following table maps each class of broken input to the first boundary that
reports it:

| Variant | Boundary and cause |
| --- | --- |
| `DuplicateArtifact` | Two supplied runtime records have one `ArtifactId`. |
| `MissingArtifact` | A required finalized identity has no supplied runtime record. |
| `UnexpectedArtifact` | A supplied runtime record remains after expected identities are consumed. |
| `ArtifactMismatch` | Runtime identity, target, digest, ABI shape, calculation stage contract, resolved location, dtype, access, alignment, size, fault flag, dynamic suffix, or element count contradicts Finalize. |
| `MissingDevice` | A selected task's planned device is outside a partition, a binding is absent, or an arena/resource lookup has no device. |
| `UnexpectedDevice` | A native binding exists for a device not represented by the plan's finalized arena set. |
| `MissingQueue` | A task's queue ID is absent or belongs to another device. |
| `MissingCompletion` | A task's completion ID is absent or belongs to another device. |
| `ValueMismatch` | A metric has no finalized location, or a downstream native range or four-byte metric contract is invalid. |
| `Protocol` | External-to-external transfer, unknown selected task, invalid task phase or endpoint class, inconsistent admission, route, lane claims, work class, slots, or pending-token state. |
| `SubmissionQueueLimitExceeded` | Distinct queues requested by the scoped tasks exceed the discovered native maximum. |
| `CompletionBusy` | A valid completion slot is currently owned by another pending task. |
| `ResourceContention` | A preallocated argument, queue, or native resource cannot satisfy the exact operation. |
| `ArenaMismatch` | A finalized init image, reservation, staging size, or arena identity differs from the warmed native resource. |
| `UnsupportedTransfer` | A direct CUDA/HSA operation receives endpoints outside its native copy contract. |
| `BackendState` or `BackendPoisoned` | Resources are rebound, a warm handoff is incomplete, or a prior native failure poisoned the adapter. |

The plan itself does not emit `Cuda`, `Hsa`, `Kernel`, or symbol-lookup errors.
Those arise after plan validation during native image inspection, loading,
argument preparation, queue submission, polling, or teardown.

## Non-responsibilities and common misreadings

* `ExecutionPlan` is not an arena layout. Physical arena offsets remain in
  `ResolvedValueLocation` records owned by `FinalizedBundle`.
* `PlannedSubmission` is not a completion token. It names the immutable queue
  and completion slots; `prepare_pending` creates or retrieves the native token.
* The plan does not classify `TaskKind::Transfer` into admission, internal, or
  exit work. `transfer_work_class` in each backend and `PreparedTask::new` own
  phase and endpoint classification.
* The plan does not validate route topology or lane capacity. The finalized
  scheduler owns route claims; backend and worker projections compare submitted
  claims with their task contracts and measured links.
* Runtime artifact validation is not compilation. `RuntimeArtifact` already
  contains bytes and an inspected ABI. Compilation and image inspection happen
  before or alongside resource realization, never in a running loop.
* A partition plan is not a reduced bundle. Its submissions map is built from
  all bundle tasks, while its artifact map is narrowed to selected calculation
  artifacts and its device set is supplied by the partition caller.

## Source map

| Concern | Source boundary |
| --- | --- |
| Runtime image and artifact records | [`native-executor/src/plan.rs`](../../src/plan.rs) lines 19-90 |
| Artifact and submission contract storage | [`native-executor/src/plan.rs`](../../src/plan.rs) lines 92-225 |
| Full and partition plan construction | [`native-executor/src/plan.rs`](../../src/plan.rs) lines 129-225 |
| Runtime target and image checks | [`native-executor/src/plan.rs`](../../src/plan.rs) lines 227-352 |
| Calculation ABI contract | [`native-executor/src/plan.rs`](../../src/plan.rs) lines 354-640 |
| Task-to-device and slot projection | [`native-executor/src/plan.rs`](../../src/plan.rs) lines 642-728 |
| Finalized bundle fields and resolved accessors | [`core/src/plan.rs`](../../../core/src/plan.rs) lines 2375-2630 |
| Submission slots, init images, values, tasks, and resources | [`core/src/schedule.rs`](../../../core/src/schedule.rs) lines 271-566 |
| Backend work records and closed backend ABI | [`executor/src/backend.rs`](../../../executor/src/backend.rs) lines 16-121 and 318-449 |
| Finalized task to `PreparedWork` and `BackendWork` | [`executor/src/executor.rs`](../../../executor/src/executor.rs) lines 1620-1934 |
| Backend bind, pending, and plan consumers | [`native-executor/src/cuda.rs`](../../src/cuda.rs) lines 295-552 and [`native-executor/src/hsa.rs`](../../src/hsa.rs) lines 329-697 |
| Native queue, artifact, staging, metric, scratch, and egress realization | [`native-executor/src/cuda.rs`](../../src/cuda.rs) lines 1665-1904 and [`native-executor/src/hsa.rs`](../../src/hsa.rs) lines 1808-2050 |
| Local partition ownership and slot-owner checks | [`native-executor/src/local.rs`](../../src/local.rs) lines 2291-2585 |
| Candidate-time runtime artifact validation | [`native-executor/src/candidate.rs`](../../src/candidate.rs) lines 475-552 |
| Parallel remote worker projection, if used | [`executor/src/worker.rs`](../../../executor/src/worker.rs) lines 372-962 |

The line references above describe the current checkout. The authoritative
behavior remains the linked source, especially when a future refactor moves a
helper without changing its contract.
