# `recipe_core::plan`

```yaml
document: recipe_core.plan
source: core/src/plan.rs
kind: immutable-plan-contract
authority:
  - core/src/plan.rs
  - core/src/schedule.rs
  - core/src/artifact.rs
  - planner/src/planner.rs
  - scheduler/src/static_schedule.rs
  - scheduler/src/arena.rs
  - prepare/src/lib.rs
  - executor/src/executor.rs
  - system-contract.md
```

This document describes the data boundary that turns a graph and measured
machine profile into one immutable execution bundle. It follows the values,
tasks, resources, artifacts, addresses, and validation evidence through Draft,
Realize, Finalize, and the runtime. It does not treat discovery, compilation,
allocation, loading, or transport as model work. The only model work kinds are
calculations and transfers. A metric is a specialized four-byte device
readback transfer.

Source line references below are anchors for the current tree. The record and
algorithm names are the parseable contract; line numbers identify the
implementation that currently enforces each rule.

## 1. Contract vocabulary

### 1.1 Lifecycle

`RunPhase` is the total order `Init < Loop < Exit` (`core/src/schedule.rs:14-19`).
The phase is stored on every `Task` and is never inferred by an executor.

| phase | permitted task kinds | meaning |
| --- | --- | --- |
| `Init` | external to device `Transfer`; internal device to device `Transfer` is representable | admit one packed image into each required device |
| `Loop` | `Calculation`, internal device to device `Transfer`, `Metric` | execute the immutable graph for one or more loop iterations |
| `Exit` | internal device to device or device to external `Transfer` | publish external outputs and only then release arenas |

`TaskKind::Metric` is not a third model operation. It reads exactly one
four-byte device value into a preallocated metric slot. `MetricPurpose::User`
publishes newest-value-wins telemetry; `MetricPurpose::FaultReadback` is the
mandatory fail-closed readback for a checked calculation cohort
(`core/src/schedule.rs:415-434`).

### 1.2 Loop count and activation domains

`LoopIterations` is either `Finite(NonZeroU64)` or `Unbounded`
(`core/src/schedule.rs:21-82`). Zero iterations cannot be represented. An
`IterationDomain` is a nonempty, zero-based, half-open arithmetic progression
with a nonzero stride (`core/src/schedule.rs:99-218`). A loop task is active
only when its domain contains the current iteration index.

`LoopTaskDomain { task, domain }` is the sidecar association for a loop task,
and `LoopSchedule { iterations, domains }` is the complete input to Finalize
(`core/src/schedule.rs:220-254`). The task graph is never unrolled. The bundle
retains one task and one pending token per graph task; repetition reuses those
objects when a backend advertises `supports_loop_repetition`.

### 1.3 Time and submission identity

`ScheduleWindow { start, end }` is a half-open interval. It is valid exactly
when `start < end`, and two windows overlap when
`left.start < right.end && right.start < left.end`
(`core/src/schedule.rs:256-269`).

`SubmissionSlots { queue, completion }` names the preallocated queue and
completion token for one task. A queue and completion slot must belong to the
task's submission device. Submission slots are compacted after static
scheduling and then are immutable in the Draft and FinalizedBundle.

The repository-wide lifecycle contract agrees with these types: one logical
image admission per required device in `Init`, no external dataset or result
movement in `Loop`, one arena release per device in `Exit`, and no compilation,
loading, allocation, routing, topology discovery, or replanning after
preparation (`system-contract.md:138-179`). The plan records are the typed
evidence that lets those runtime restrictions be checked before execution.

The complete representation flow is:

```text
CalculationGraph
  -> LoweredProgram + KernelTemplate
  -> UnscheduledTask + ValueSpec + ArtifactBuildRecipe
  -> StaticSchedule(Task with window and lane claims)
  -> DraftPlan + candidate ArenaLayout
  -> RealizationProfile + final ArenaLayout
  -> FinalizedBundle(ResolvedValueLocation, LoopTaskDomain)
  -> PreparedTask(BackendWork, Pending token)
  -> backend submission/poll and ExitImage
```

Each arrow either adds evidence or resolves an earlier logical reference. No
later stage is allowed to rewrite a prior task kind, dependency, artifact
identity, route, or value ownership.

## 2. Plan records

### 2.1 Reservation records

```text
EXACT_USER_RESERVATION = 1_000_000_000 bytes
ReservationMechanism = HeldAllocation | EnforcedQuota
ReservationEvidence = NonGpu | GpuDisplay { enabled_connectors: u32 }
ReservationEntry = {
  device: DeviceId,
  name: Label,
  bytes: ByteCount,
  mechanism: ReservationMechanism,
  evidence: ReservationEvidence,
}
ReservationLedger = { entries: Vec<ReservationEntry> }
```

The byte contract is evidence-derived (`core/src/plan.rs:23-50`):

```text
required_bytes(NonGpu) = EXACT_USER_RESERVATION
required_bytes(GpuDisplay { enabled_connectors: 1.. }) = EXACT_USER_RESERVATION
required_bytes(GpuDisplay { enabled_connectors: 0 }) = 0
```

`ReservationLedger::validate(topology)` (`core/src/plan.rs:66-132`) requires:

1. every entry device exists;
2. each device occurs once;
3. RAM and disk use `NonGpu`, GPU memory uses `GpuDisplay`;
4. `entry.bytes == evidence.required_bytes()` exactly; and
5. every topology device has one entry.

`mechanism` and `name` are carried as evidence. The validator does not infer a
different mechanism or silently change the requested byte count.

### 2.2 Capacity records

```text
CapacityLedgerEntry = {
  device: DeviceId,
  total: Property<ByteCount>,
  runtime_overhead: Property<ByteCount>,
  fragmentation: Property<ByteCount>,
  safety_headroom: Property<ByteCount>,
  recipe_usable: Property<ByteCount>,
}
CapacityLedger = { entries: Vec<CapacityLedgerEntry> }
```

`CapacityLedger::validate(topology, reservations)` (`core/src/plan.rs:134-230`)
requires one entry for every topology device, unique and known device IDs, and
schedulable evidence for all five properties. For each device it checks the
overflow-safe accounting inequality:

```text
reservation.bytes
+ runtime_overhead.value
+ fragmentation.value
+ safety_headroom.value
+ recipe_usable.value
<= total.value
```

An addition overflow is `CapacityOverflow`; a missing device is
`MissingRequiredObject`; an unmeasured property is `UnmeasuredProperty`.
`recipe_usable` is the only capacity consumed by arena, staging, and scratch
planning.

### 2.3 DraftPlan

`DraftPlan` is the exact, offset-free candidate emitted by the planner
(`core/src/plan.rs:232-250`). Every field is public because Draft is the
planner-to-preparation interchange record. It is immutable by convention after
construction and validated before it leaves planning.

```text
DraftPlan = {
  identity: DraftIdentity,
  candidate: CandidateIdentity,
  discovery: DiscoveryIdentity,
  topology: TopologyIdentity,
  values: Vec<ValueSpec>,
  kernels: Vec<KernelTemplate>,
  artifacts: Vec<ArtifactIdentity>,
  artifact_builds: Vec<ArtifactBuildRecipe>,
  tasks: Vec<Task>,
  resources: ResourceManifest,
  arena_objects: Vec<ArenaObject>,
  value_bindings: Vec<ValueBinding>,
  value_aliases: Vec<ValueAliasContract>,
  init_images: Vec<InitDataImage>,
  releases: Vec<ArenaRelease>,
}
```

The Draft deliberately has no physical arena offsets. `ValueBinding` names an
arena object and object-relative offset. `ArenaLayout` offsets are supplied by
the scheduler after realization capacity is known. Deferred artifacts are
represented by `artifact_builds`; a Draft never contains the same artifact ID
in both `artifacts` and `artifact_builds`.

### 2.4 Values, bindings, and addresses

The schedule records used by Draft are defined in
`core/src/schedule.rs:277-340`:

```text
ValueSpec = {
  id: ValueId,
  dtype: DType,
  bytes: ByteCount,
  device: DeviceId,
  producer: Option<TaskId>,
}
ValueBinding = {
  value: ValueId,
  object: ArenaObjectId,
  object_offset: ByteOffset,
}
ResolvedValueLocation = {
  value: ValueId,
  dtype: DType,
  device: DeviceId,
  bytes: ByteCount,
  object: ArenaObjectId,
  object_offset: ByteOffset,
  arena_offset: ByteOffset,
}
```

`ValueSpec.device` is the resident device, not a host ownership hint. The
Finalized address is computed as:

```text
arena_offset = allocation(object).offset + binding.object_offset
```

The addition, value end, device, type alignment, and arena bounds are checked
by `resolve_value_locations` (`core/src/plan.rs:2775-2866`). A missing binding
does not produce a partial location. Only validated locations are retained in
the FinalizedBundle.

`ArenaObject { id, device, bytes, alignment, lifetime }` has a logical lifetime
but no physical offset (`core/src/schedule.rs:568-576`). `ArenaRelease { device }`
is the one exit release contract per topology device.

### 2.5 Tasks and transfers

```text
CalculationTask = {
  device: DeviceId,
  kernel_template: KernelTemplateId,
  artifact: ArtifactId,
  inputs: Vec<ValueId>,
  outputs: Vec<ValueId>,
  fault_flag: Option<ValueId>,
  work: FlopCount,
  submission: SubmissionSlots,
}
TransferEndpoint = External | Device { device: DeviceId, value: ValueId }
TransferLaneClaim = Link { link: LinkId, lane: u32 }
                    | External { device: DeviceId, lane: u32 }
TransferTask = {
  source: TransferEndpoint,
  destination: TransferEndpoint,
  bytes: ByteCount,
  route: Vec<LinkId>,
  lane_claims: Vec<TransferLaneClaim>,
  submission: SubmissionSlots,
}
MetricTask = {
  purpose: MetricPurpose,
  metric: MetricId,
  value: ValueId,
  slot: MetricSlotId,
  submission: SubmissionSlots,
}
Task = {
  id: TaskId,
  phase: RunPhase,
  window: ScheduleWindow,
  dependencies: Vec<TaskId>,
  kind: TaskKind,
}
```

These records are `core/src/schedule.rs:342-450`. An executor-visible
device-to-device transfer has zero or one route link. A multi-link path is
represented by dependency-chained transfer tasks and resident intermediate
values. A same-device copy has an empty route but still has device endpoints.
An external admission or egress has an empty route and exactly one external
lane claim on its endpoint device.

### 2.6 Packed init images

```text
InitDataImageMember = {
  logical: ValueId,       # graph-level external input
  physical: ValueId,      # candidate resident copy
  dtype: DType,
  bytes: ByteCount,
  image_offset: ByteOffset,
}
InitDataImage = {
  device: DeviceId,
  image: ValueId,         # resident value spanning the complete image
  bytes: ByteCount,
  members: Vec<InitDataImageMember>,
}
```

There is exactly one image and one `Init` admission task per topology device.
The image includes external input members and any preallocated int32 fault
flags. The image is at least four bytes, even when it has no graph input. The
member offsets are relative to the packed image; Finalize resolves both the
image and each member into one arena object.

### 2.7 Resources and aliases

`ResourceManifest` fixes queues, completion slots, metric slots, pinned staging
peaks, and scratch peaks (`core/src/schedule.rs:452-566`):

```text
QueueSlot = { id: QueueSlotId, device: DeviceId }
CompletionSlot = { id: CompletionSlotId, device: DeviceId }
MetricSlot = { id: MetricSlotId, metric: MetricId }
DeviceBytes = { device: DeviceId, bytes: ByteCount }
ResourceManifest = {
  queues: Vec<QueueSlot>,
  completions: Vec<CompletionSlot>,
  metrics: Vec<MetricSlot>,
  pinned_staging: Vec<DeviceBytes>,
  scratch: Vec<DeviceBytes>,
}
```

Its validator checks unique IDs and known device ownership, but task-specific
queue/completion ownership is checked by `validate_submission` in the plan
validator.

The scheduler returns the physical offset sidecar only after packing:

```text
ArenaAllocation = { object: ArenaObjectId, offset: ByteOffset }
ArenaLayout = {
  device: DeviceId,
  size: ByteCount,
  allocations: Vec<ArenaAllocation>,
}
```

`ArenaLayout` is not part of `DraftPlan`; it is supplied to Finalize from the
realization-capacity packing pass and becomes private FinalizedBundle state.

`ValueAliasContract { input, output, permission }` records the exact storage
relationship selected for one primitive source alias pair. The three
permissions are:

```text
Forbidden       => ranges must be disjoint
MayAliasExact   => ranges may overlap only when object, offset, and byte size are exact
MustAliasExact  => object, offset, and byte size must be exact
```

Alias constraints are checked for explicit Draft contracts and for every alias
rule attached to a kernel template (`core/src/plan.rs:1827-2005`).

### 2.8 Artifact records carried by the plan

The artifact module supplies two disjoint records used by Draft and
Realization (`core/src/artifact.rs:9-112`, `214-266`):

```text
TargetIdentity = { backend: Label, architecture: Label, abi: Label }
ToolchainIdentity = { name: Label, version: Label, digest: Digest }
KernelResourceBounds = {
  private_bytes_per_lane: ByteCount,
  shared_bytes_per_workgroup: ByteCount,
  scratch_bytes_per_dispatch: ByteCount,
  maximum_workgroup_lanes: u32,
}
ArtifactBuildProvenance = {
  program_digest: Digest,
  stage_ordinal: u32,
  contract_digest: Digest,
}
ArtifactBuildView = {
  logical_extents: Vec<u64>,
  offset_elements: u64,
  strides: Vec<u64>,
  storage_bytes: ByteCount,
}
ArtifactBuildBinding = {
  value: ValueId,
  dtype: DType,
  access: Read | Write | ReadWrite | ReadWriteAtomic,
  view: ArtifactBuildView,
}
ArtifactDispatchGeometry = {
  logical_lanes: u64,
  workgroup_lanes: u32,
  workgroups: u64,
}
ArtifactWorkBounds = {
  flops: FlopCount,
  integer_operations: u64,
  atomic_operations: u64,
}
ArtifactBuildRecipe = {
  artifact: ArtifactId,
  kernel_template: KernelTemplateId,
  source_kernel: KernelTemplateId,
  provenance: ArtifactBuildProvenance,
  bindings: Vec<ArtifactBuildBinding>,
  dispatch: ArtifactDispatchGeometry,
  work: ArtifactWorkBounds,
  fault_flag: Option<ValueId>,
  resources: KernelResourceBounds,
}
ArtifactIdentity = {
  id: ArtifactId,
  digest: Digest,
  format: Label,
  target: TargetIdentity,
  toolchain: ToolchainIdentity,
  entry_symbol: Label,
  kernel_template: KernelTemplateId,
  resources: KernelResourceBounds,
  build: Option<ArtifactBuildProvenance>,
}
```

`ArtifactBuildRecipe` is target-independent and is the deferred Draft request;
the same validated recipe is retained unchanged in FinalizedBundle beside its
realized identity. It requires nonzero stage identities and provenance, positive launch
geometry, exact workgroup ceiling division, unique ordered value bindings, and
an exact int32 atomic binding when `fault_flag` is present. `ArtifactIdentity`
is the target-specific realized image identity; its digest, toolchain digest,
resource bounds, and optional provenance are validated before it can satisfy a
calculation task. A Draft calculation names one `ArtifactId`, and the Draft
validator requires that ID to resolve to exactly one of these two records.
Production native identities use the measured target ABI: CUDA artifacts carry
`elf64-cubin`, while HSA artifacts carry
`elf64-amdgpu-code-object-v<code_object_version>`. The plan stores this format
and target identity; the native executor separately verifies that the runtime
image bytes and driver/code-object metadata agree with it.

## 3. Draft validation

`DraftPlan::validate(topology, discovery)` is an accumulating validation pass,
not a first-error shortcut (`core/src/plan.rs:252-405`). It first validates the
supplied topology, scheduling properties, discovery profile, identities, and
resource manifest. It then builds ID indexes and invokes the focused validators
in this order:

```text
validate_tasks
validate_arena_objects
validate_value_bindings
validate_init_images
validate_releases
```

The call graph is intentionally one-directional and has no mutation of the
Draft:

```yaml
DraftPlan.validate:
  preflight:
    - Topology.validate
    - Topology.validate_scheduling_properties
    - DiscoveryProfile.validate(topology)
    - ResourceManifest.validate(topology)
  indexes:
    - values by ValueId
    - kernels by KernelTemplateId
    - artifacts by ArtifactId
    - artifact_builds by ArtifactId
  cross_record:
    - validate_tasks(indexes)
    - validate_arena_objects
    - validate_value_bindings(indexes)
    - validate_init_images(indexes)
    - validate_releases
  result: Validator.finish() -> ValidationResult
```

Every failure is a `ValidationError { code, path, message }`; paths are stable
field paths such as `tasks[3].kind`, `init_images[0].members[2].image_offset`,
or `values[7].producer` (`core/src/error.rs:75-131`).

### 3.1 Top-level identity and catalog checks

The first checks require:

```text
draft.identity != zero
draft.candidate != zero
draft.discovery == discovery.identity
draft.topology == topology.identity
```

Values, kernel templates, realized artifacts, and deferred builds each have
unique IDs. A build artifact ID cannot also be a realized artifact ID, and one
stage-scoped kernel template cannot appear in two deferred builds. Every build
is validated by `ArtifactBuildRecipe::validate`; every realized artifact is
validated by `ArtifactIdentity::validate` (`core/src/artifact.rs:95-212`,
`214-266`). Build bindings must name Draft values.

### 3.2 Task graph and phase checks

`validate_tasks` (`core/src/plan.rs:416-871`) performs the following for every
task:

| check | required condition | failure code |
| --- | --- | --- |
| ID | task IDs unique | `DuplicateId` |
| window | `window.start < window.end` | `InvalidLifetime` |
| dependency identity | each dependency unique and present | `DuplicateId`, `UnknownReference` |
| dependency phase | predecessor phase is not later than dependent phase | `DependencyPhaseOrder` |
| dependency time | predecessor end is no later than dependent start | `DependencyScheduleOrder` |
| graph acyclicity | Kahn traversal visits every task | `DependencyCycle` |

Phase and kind legality is exact:

```text
Calculation => phase == Loop
Metric      => phase == Loop
External -> Device transfer => phase == Init, route == []
Device -> External transfer => phase == Exit, route == []
Device -> Device transfer => any phase, route.len() <= 1 and topology route is valid
External -> External        => invalid run task
```

The planner normally emits internal copies in `Loop`, while the executor's
generic `PreparedTask` projection also accepts an internal copy in `Init` or
`Exit`. The phase-specific restriction is on external admission and egress,
not on the endpoint pair of an internal transfer.

Calculations must run on a `GpuMemory` device. Every input, output, and optional
fault flag must exist and be resident on that GPU. A fault flag is exactly one
aligned `DType::I32` value with four bytes. A calculation's artifact must
resolve to exactly one realized identity or deferred build. If realized, the
artifact's kernel template and discovered target must match. If deferred, the
build's stage identity, FLOP work, ordered read and write bindings, resident
value type and storage size, and fault-flag presence must match the task.

Transfers validate both endpoint values, exact byte counts, source and
destination device ownership, equal source and destination dtypes for internal
copies, route validity, and submission ownership. Lane claims must be strictly
sorted and unique. An internal transfer claims exactly one lane for every route
link and no external lane. An external transfer claims exactly one lane on its
endpoint device and no link lane. A transfer-to-transfer overlap on the same
lane, or opposing links sharing a half-duplex resource, is
`ResourceContention`.

Metrics require a loop phase, a known value, a matching metric slot, and a
valid submission. Fault readbacks require exactly one four-byte int32 value.

Finally, every topology device must have exactly one external admission task.
Zero admissions is `MissingUpload`; more than one is `DuplicateUpload`.

### 3.3 Fault readback closure

`validate_fault_readbacks` (`core/src/plan.rs:873-979`) groups calculation
tasks by their fault flag and metric tasks by the value they read. For each
fault cohort it requires:

```text
exactly one MetricPurpose::FaultReadback task
the readback metric slot has exactly one user
the readback directly depends on every checked calculation
every Exit task transitively depends on the readback
every User metric transitively depends on the readback
```

A readback naming a flag with no checked calculation is invalid. This makes a
nonzero device fault fail before any publication or egress can complete.

### 3.4 Resource contention and measured capabilities

`validate_resource_contention` (`core/src/plan.rs:1142-1417`) examines every
pair of overlapping task windows:

* overlapping submissions cannot share a queue or completion slot;
* overlapping transfers cannot claim the same lane;
* opposing transfers on different links cannot overlap on one half-duplex
  transport resource;
* a transfer touching a calculation device may overlap only when discovery
  says `overlaps_calculation`; and
* event sweeps over each link, each device external-transfer capability, and
  each device calculation capability cannot exceed measured concurrency.

Arithmetic is checked with saturating counters for the event sweep and
`CapabilityConcurrencyExceeded` is emitted when a measured bound is exceeded.
The validator never adds a queue, lane, or capability fallback.

### 3.5 Arena object and value binding checks

`validate_arena_objects` (`core/src/plan.rs:1444-1472`) requires unique object
IDs, known devices, nonzero power-of-two alignment, and valid lifetimes.

`validate_value_bindings` (`core/src/plan.rs:1474-1599`) requires one binding
for every value and no duplicate value binding. It checks:

```text
value.device == object.device
object.alignment >= dtype.byte_width
object.alignment is a multiple of dtype.byte_width
object_offset is dtype aligned
object_offset + value.bytes <= object.bytes
producer task exists and lies inside object lifetime
every task referencing the value lies inside object lifetime
```

The corresponding codes are `ResourceMismatch`,
`ValueBindingMisaligned`, `ValueBindingOutOfBounds`,
`ValueLifetimeMismatch`, `UnknownReference`, `DuplicateValueBinding`, and
`MissingValueBinding`.

### 3.6 Init image checks

`validate_init_images` (`core/src/plan.rs:1601-1825`) verifies one image per
device, nonzero image bytes, matching image `ValueSpec`, and exact agreement
with that device's sole external admission task. The image value is produced
by the admission task and has a binding. Each member requires:

```text
logical != zero and unique within its image
physical is globally unique across all images
physical value exists on image.device with matching dtype and bytes
physical producer is this image's admission task
physical and image share one arena object
physical.object_offset == image.object_offset + member.image_offset
member.image_offset is dtype aligned
member.image_offset + member.bytes <= image.bytes
member byte ranges do not overlap
```

Every topology device must have a manifest. Violations use
`InvalidDataImage`, `MissingUpload`, `DuplicateId`, `MissingValueBinding`,
`ValueBindingMisaligned`, `ValueBindingOutOfBounds`,
`LiveAllocationOverlap`, `ResourceMismatch`, or `MissingRequiredObject`.

### 3.7 Alias and release checks

`validate_alias_bindings` checks explicit alias contracts and kernel alias rules.
It first requires calculation input and output arity to match its kernel
template, then evaluates each alias permission against binding ranges. A
range end overflow is conservatively treated as overlap, so it cannot bypass
`AliasViolation` (`core/src/plan.rs:1827-2005`).

`validate_releases` (`core/src/plan.rs:2025-2055`) requires one known
`ArenaRelease` per topology device. Missing or duplicate releases are
`MissingRelease` and `DuplicateRelease`.

### 3.8 Draft validation failure index

```yaml
DraftPlan.validate:
  identity/catalog: [InvalidIdentity, IdentityMismatch, DuplicateId, UnknownReference,
                     WrongKind, ArtifactMismatch, ResourceMismatch]
  task_graph: [DuplicateId, InvalidLifetime, UnknownReference, DependencyPhaseOrder,
               DependencyScheduleOrder, DependencyCycle, InvalidPhase]
  calculation: [InvalidCalculationPlacement, ArtifactMismatch, ResourceMismatch,
                ScalarArity]
  transfer: [InvalidExternalTransfer, InvalidRoute, InvalidLaneClaim,
             ResourceMismatch, ResourceContention]
  metric_fault: [InvalidFaultReadback]
  measured_contention: [ResourceContention, CapabilityConcurrencyExceeded]
  arena: [AllocationMisaligned, InvalidLifetime, DuplicateValueBinding,
          MissingValueBinding, ValueBindingOutOfBounds, ValueBindingMisaligned,
          ValueLifetimeMismatch, LiveAllocationOverlap]
  init_image: [InvalidDataImage, MissingUpload, DuplicateUpload, MissingValueBinding,
               ValueBindingOutOfBounds, ValueBindingMisaligned, LiveAllocationOverlap]
  alias_release: [DuplicateAliasRule, AliasViolation, MissingRelease, DuplicateRelease]
```

### 3.9 Validation-code emission map

The following map is exhaustive for `ValidationCode` values emitted by
`core/src/plan.rs`. A code may occur in more than one validator because the
same cross-record invariant is checked at both the Draft and Finalize boundary.

```yaml
reservation_ledger_validate: [UnknownReference, DuplicateReservation, WrongKind,
                              WrongReservationSize, MissingReservation]
capacity_ledger_validate: [UnknownReference, DuplicateId, UnmeasuredProperty,
                            CapacityOverflow, MissingRequiredObject]
draft_catalog_indexing: [InvalidIdentity, IdentityMismatch, DuplicateId,
                         UnknownReference, WrongKind]
task_graph_and_phase: [DuplicateId, InvalidLifetime, UnknownReference,
                       DependencyPhaseOrder, DependencyScheduleOrder,
                       InvalidPhase, DependencyCycle]
calculation_contract: [InvalidCalculationPlacement, UnknownReference,
                       ArtifactMismatch, ResourceMismatch]
transfer_endpoint_and_route: [UnknownReference, ResourceMismatch,
                               InvalidExternalTransfer, InvalidRoute,
                               InvalidLaneClaim]
metric_and_fault_contract: [InvalidPhase, UnknownReference, InvalidFaultReadback,
                            ResourceMismatch]
resource_contention: [ResourceContention, CapabilityConcurrencyExceeded]
arena_object_contract: [DuplicateId, UnknownReference, AllocationMisaligned,
                         InvalidLifetime]
value_binding_contract: [DuplicateValueBinding, UnknownReference,
                         ResourceMismatch, ValueBindingMisaligned,
                         ValueBindingOutOfBounds, ValueLifetimeMismatch,
                         MissingValueBinding]
init_image_contract: [UnknownReference, DuplicateId, InvalidDataImage,
                      ResourceMismatch, MissingUpload, MissingValueBinding,
                      ValueBindingMisaligned, ValueBindingOutOfBounds,
                      LiveAllocationOverlap, MissingRequiredObject]
alias_contract: [DuplicateAliasRule, UnknownReference, ScalarArity,
                 AliasViolation]
release_contract: [UnknownReference, MissingRelease, DuplicateRelease]
realization_artifacts: [InvalidIdentity, IdentityMismatch, DuplicateId,
                        ArtifactMismatch, ResourceMismatch]
loop_domains: [DuplicateId, UnknownReference, InvalidPhase,
               InvalidIterationDomain]
finalized_layouts: [UnknownReference, DuplicateId, DuplicateAllocation,
                    ResourceMismatch, AllocationMisaligned,
                    AllocationOutOfBounds, InsufficientCapacity,
                    LiveAllocationOverlap, MissingRequiredObject,
                    MissingAllocation]
resolved_locations: [AddressOverflow, ResourceMismatch,
                     ValueBindingOutOfBounds, ValueBindingMisaligned]
```

The corresponding implementation spans are, respectively,
`ReservationLedger::validate` (`core/src/plan.rs:66-132`),
`CapacityLedger::validate` (`134-230`), `DraftPlan::validate` and its task
indexing (`252-405`), task and resource validation (`416-2055`), realization
validation (`2059-2228`), loop-domain validation (`2230-2373`), and Finalize
layout/address validation (`2632-2866`).

The public core-plan API has these contracts:

```yaml
ReservationLedger.validate(topology): ValidationResult
CapacityLedger.validate(topology, reservations): ValidationResult
DraftPlan.validate(topology, discovery): ValidationResult
RealizationProfile.validate(topology, discovery, draft): ValidationResult
FinalizedBundle.finalize(identity, topology, discovery, draft, realization,
                        arena_layouts): ValidationResult<FinalizedBundle>
FinalizedBundle.finalize_with_loop_iterations(identity, topology, discovery,
                                              draft, realization, arena_layouts,
                                              loop_iterations): ValidationResult<FinalizedBundle>
FinalizedBundle.finalize_with_loop_schedule(identity, topology, discovery,
                                            draft, realization, arena_layouts,
                                            loop_schedule): ValidationResult<FinalizedBundle>
FinalizedBundle.transfer_endpoints(task): Option<ResolvedTransferEndpoints>
FinalizedBundle.value_location(value): Option<&ResolvedValueLocation>
```

The first four validators accumulate all observed failures. Finalize appends
Draft, realization, loop-domain, layout, and address failures into one result;
only an error-free pass may move the records into the private bundle.

### 3.10 Validator helper semantics

The private helpers do not create alternate state; they define how the public
validators derive their evidence:

```yaml
validate_transfer_endpoint:
  input: TransferEndpoint, exact bytes, ValueSpec index
  checks: [known value, endpoint device ownership, exact byte count]
  output: referenced ValueSpec or None for External
validate_transfer_lane_claims:
  checks: [strict ordering, uniqueness, known links/devices, lane bounds,
           exact route-to-claim correspondence]
validate_concurrency_events:
  algorithm: sort (time, delta), subtract releases before starts at equal time
  failure: CapabilityConcurrencyExceeded when active > measured maximum
task_depends_on:
  algorithm: transitive DFS over Task.dependencies with visited TaskId set
  use: fault-readback publication barriers
validate_compute_transfer_overlap:
  checks: endpoint/route touches calculation device and discovery overlap flag
  failure: ResourceContention when overlap is unsupported
task_references_value:
  includes: calculation inputs/outputs/fault flag, transfer endpoints, metric value
window_contains:
  predicate: container.start <= contained.start && contained.end <= container.end
binding_ranges_overlap:
  predicate: same object and intersecting checked byte ranges
validate_alias_pair:
  decision: permission-specific Forbidden/MayAliasExact/MustAliasExact check
validate_realized_artifacts:
  checks: exact prebuilt set, exact deferred set, provenance, target, resources
validate_loop_domains:
  checks: one domain per loop task, none for non-loop, domain bounds and coupling
```

`submission_slots` and `transfer_links` are normalization helpers used by the
contention pass. They expose no new contract: the former extracts the one
`SubmissionSlots` value from any `TaskKind`, while the latter deduplicates route
links before half-duplex checks (`core/src/plan.rs:1332-1417`).

## 4. Planner trace: graph to Draft

The planner is the only producer of a normal Draft. The public entry points are
`plan_candidates` and `plan_program_candidates`
(`planner/src/planner.rs:187-350`). The latter performs this fixed sequence:

```text
1. validate StaticCalculationProgram and its graph
2. validate topology, scheduling properties, discovery, reservations, capacity
3. derive graph topological order, tensor index, and lowered primitive programs
4. validate the artifact catalog
5. enumerate every measured GPU placement option for every source kernel
6. hash graph/domain/program identity
7. lower each finite placement assignment
8. retain valid candidates, rank by makespan then CandidateIdentity
```

There is no speculative placement on a non-GPU or undiscovered device. If no
measured GPU calculation capability exists, planning stops with
`NoCalculationDevice`. If every assignment fails, the first candidate failure
is reported as `NoViableCandidate` with its classified cause.

### 4.1 Planner-owned candidate envelopes

The planner keeps the Draft together with the evidence needed by preparation
and later output mapping. These records are not alternative plan formats:
`PlannedCandidate.draft` is the sole task/resource contract, while the other
fields are immutable evidence about how that Draft was produced
(`planner/src/planner.rs:28-71`).

```text
KernelPlacement = {
  kernel: KernelTemplateId,
  device: DeviceId,
}
StagePlacement = {
  source_kernel: KernelTemplateId,
  stage_ordinal: u32,
  kernel_template: KernelTemplateId,
  device: DeviceId,
  artifact: ArtifactId,
}
LogicalValueCopy = {
  logical: ValueId,
  device: DeviceId,
  physical: ValueId,
}
PlannedExternalOutput = {
  task: TaskId,
  logical: ValueId,
  device: DeviceId,
  physical: ValueId,
}
PlannedCandidate = {
  draft: DraftPlan,
  arena_layouts: Vec<ArenaLayout>,
  makespan: Nanoseconds,
  placements: Vec<KernelPlacement>,
  stage_placements: Vec<StagePlacement>,
  lowered_programs: Vec<LoweredProgram>,
  value_copies: Vec<LogicalValueCopy>,
  external_outputs: Vec<PlannedExternalOutput>,
}
PlannedProgramCandidate = {
  planned: PlannedCandidate,
  loop_iterations: LoopIterations,
  loop_domains: Vec<LoopTaskDomain>,
}
```

`PlannedExternalOutput` is deliberately retained rather than reverse-matched
from a finalized task. It proves which physical value the planner selected for
each logical output. Preparation carries it into `PreparedSystem`; training and
inference verify it against `FinalizedBundle.transfer_endpoints` before
accepting an exit image.

`PlannerSearch` and `ProgramPlannerSearch` are ranked, one-shot streams
(`planner/src/planner.rs:73-169`). `next_candidate` advances its cursor and
marks the returned candidate identity issued. An issued candidate is never
returned again, even if the caller does not call `reject`. `reject` accepts only
an issued identity and rejects a second rejection with `AlreadyRejected`; an
identity that was never issued is `UnknownCandidate`. Preparation therefore
cannot mutate and retry one Draft: it either accepts the candidate or records a
candidate-local rejection and advances to the next ranked identity.

The planner's identity layers are deterministic. `graph_digest` includes
tensor dtype, storage bytes, shape and layout, external-input/output flags,
topological kernel order, source arities and work, lowered-program digests and
stage template IDs, loop count and domains, and metric declarations
(`planner/src/planner.rs:574-650`). `candidate_identity` adds topology and
discovery identities plus the ordered kernel-to-device assignment
(`planner/src/planner.rs:746-763`). Candidate ranking is measured makespan
first, then candidate identity, so the stream order is stable for equal timing.

### 4.2 Lowered stage identities and artifacts

`lower_programs` lowers every primitive program against common measured
hardware limits and gives each lowered stage a collision-checked
`KernelTemplateId` (`planner/src/planner.rs:359-433`). A stage identity includes
the lowered program digest, source kernel, and stage ordinal. A scalar-map stage
is inserted into the template catalog; an identity collision is fatal.

For each stage invocation, `lower_program_invocation`
(`planner/src/planner.rs:1299-1542`) creates one loop `CalculationTask`, one
submission pair, and one target-independent `ArtifactBuildRecipe`. The build
recipe carries ordered bindings, launch geometry, operation bounds, optional
fault binding, and resource bounds. `select_or_defer_artifact`
(`planner/src/planner.rs:1681-1717`) does exactly one of:

```text
catalog has matching ArtifactIdentity => retain it in Draft.artifacts
catalog has no entry                  => retain exact ArtifactBuildRecipe
catalog has mismatched entry          => InvalidArtifact
```

Matching requires stage identity, deferred provenance, resource envelope, and
the discovered calculation target to agree. The planner does not compile a
deferred stage.

### 4.3 Init image construction

`initialize_data_images` (`planner/src/planner.rs:2276-2466`) scans every
program for external inputs and program fault buffers, grouping them by the
selected device. It then visits all topology devices in sorted ID order and:

```text
allocate one image ValueSpec and one Init task
append external input members in deterministic logical-value order
append one int32 fault flag for each device/domain fault cohort
set image.bytes = max(packed_bytes, 4)
emit one External -> Device TransferTask with empty route
```

The image task is dependency-free and is the sole producer of its image and
physical members. A pass-through external input/output tensor is assigned to
the first sorted topology device when no selected kernel already admits it.

### 4.4 Copies and route decomposition

`ensure_copy` (`planner/src/planner.rs:2468-2616`) maintains one
`RuntimeCopy` per logical value and destination device. An existing direct or
same-domain copy is reused. A transferred copy for a different iteration
domain is not reused, because that would make activation freshness ambiguous.

For a missing copy, every directed simple route is trial-scheduled. The best
candidate is selected by completion time, trial makespan, source device, source
value, and lexicographic route. `build_transfer_chain`
(`planner/src/planner.rs:2126-2274`) expands a route into one loop
`TransferTask` per link, allocates intermediate `ValueSpec`s, chains each hop
dependency, and assigns the consumer's iteration domain to every hop. A
same-device copy is one empty-route task. No multi-link route reaches the
executor as one task.

The planner preserves the allocator head for the final destination value and
places intermediate values after it. Stable task and value IDs are checked
again while committing the chosen chain; a changed allocation order is an
`InvalidDraft` planner failure.

### 4.5 Calculation dependencies, aliases, and metrics

For each lowered stage, read bindings depend on the latest producer and write
bindings become the latest producer. Stage barriers preserve the primitive
program's order. `MustAliasExact` rules add dependencies from all readers of
the old input before the overwriting task; a reader in `Exit` is a dependency
conflict (`planner/src/planner.rs:1924-1971`).

`add_fault_readbacks` creates one loop metric task for each `(device,
IterationDomain)` fault cohort, with direct dependencies on all checked
calculations (`planner/src/planner.rs:1544-1575`). `add_user_metrics` binds a
user metric to the producer-resident copy and rejects a transferred copy
(`planner/src/planner.rs:1216-1297`).

`add_external_outputs` chooses one non-overwritten, producer-resident copy per
external output. It trial-schedules an `Exit` device-to-external transfer,
chooses the fastest deterministic source, commits the task, and retains
`PlannedExternalOutput { task, logical, device, physical }`
(`planner/src/planner.rs:2643-2749`).

`add_phase_barriers` adds every Init task as a dependency of every Loop task,
and every Init and Loop task as a dependency of every Exit task
(`planner/src/planner.rs:1973-1994`). These explicit dependencies are retained
in the Draft; the scheduler also adds global phase barriers to its private
graph.

### 4.6 Scheduling, resources, arenas, and identity

`lower_candidate` (`planner/src/planner.rs:979-1213`) submits the unscheduled
tasks to `recipe_scheduler::schedule`, compacts queue/completion slots,
computes pinned-staging and scratch peaks, builds arena objects and bindings,
packs arenas, and checks total arena plus auxiliary usage against
`CapacityLedger.recipe_usable`.

The resulting records are canonicalized by stable IDs. Every topology device
gets an `ArenaRelease`. Loop domains are emitted for every scheduled loop task;
absence of a domain is an `InvalidDraft` planner error. `hash_draft`
(`planner/src/planner.rs:3594-3835`) includes identities, loop count and domains,
values, kernels, tasks, artifacts/builds, resources, arena objects, bindings,
aliases, images, and releases. The resulting `DraftIdentity` changes whenever
any execution-relevant field changes.

The constructed Draft is validated immediately. A planner candidate is not
returned when its own emitted data fails `DraftPlan::validate`.

### 4.7 Planner helper contract

The remaining planner helpers are deterministic reductions, not additional
planning surfaces:

```yaml
graph_and_choices:
  tensor_index: graph tensors -> ValueId index
  legal_choices: every source kernel -> sorted measured GPU options
  enumerate_assignments: depth-first Cartesian product over sorted options
  common_lowering_hardware: max subgroup, min workgroup/shared limits; reject no common width
  stage_template_identity: digest(program, source kernel, stage ordinal) -> nonzero KernelTemplateId
  validate_artifact_catalog: unique nonzero IDs and ArtifactIdentity.validate
lowered_bindings:
  materialize_program_buffer: tensor input -> existing copy; output/scratch -> new resident ValueSpec; fault -> init flag
  lower_build_binding: primitive BufferBinding -> ordered ArtifactBuildBinding
  program_tensor_buffer: logical tensor -> lowered ProgramBuffer or InvalidDraft
  program_tensor_value: ProgramBuffer -> materialized ValueId
  source_buffer: source input/output ordinal -> exact tensor buffer
  first_output_writer: first stage writing a source output -> TaskId
  set_value_producer: assign one ValueSpec producer TaskId
  stable_argument_position: usize ordinal + 1 with overflow check
  validate_source_must_alias: exact alias requires equal typed static views
  value_alias_contracts: invocation alias ordinals -> sorted ValueAliasContract records
transfer_search:
  trial_timing: schedule one trial task and return completion/makespan
  trial_chain_timing: schedule existing tasks plus one candidate transfer chain
  directed_routes: enumerate simple directed paths, including empty same-device path
  enumerate_directed_routes: DFS path enumerator with visited-device set
  stable_offset: allocator base + hop offset with checked arithmetic
  stable_argument_position: stable one-based source argument conversion
  build_transfer_chain: each hop, or one same-device empty route, -> one Task; intermediate ValueSpec per non-final hop
alias_and_barriers:
  add_alias_dependencies: MustAliasExact readers -> overwrite dependencies
  must_alias_inputs: collect values that cannot be overwritten before Exit
  task_kind_references: all value references in one TaskKind
  task_reads_value: input/fault/source/metric read references only
  value_crosses_phase_boundary: detect producer/readers in different phases
  value_crosses_iteration_boundary: detect stale loop copies across domains
  domain_has_nonzero_activation: whether an IterationDomain reaches index > 0
  domain_covers: producer domain covers every consumer activation
  extend_lifetime: min start and max end accumulation
resources_and_memory:
  compact_submission_resources: measured per-device queue-limit wrapper
  compact_submission_resources_with_limits: interval coloring by owner class
  task_submission_device: derive queue owner from task endpoints/value device
  finalize_auxiliary_resources: staging/scratch usage event collection
  push_usage: half-open resource usage start/end events
  peak_usage: sorted event sweep -> per-device peak bytes
  build_arena_contract: alias union-find, fixed image objects, lifetime/object bindings
  require_total_capacity: arena + auxiliary peak <= recipe_usable
hashing:
  hash_iteration_domain: first/end/stride/unbounded fields
  hash_loop_iterations: finite count or unbounded marker
  hash_kernel_template: index space, typed access, scalar program, aliases
  hash_static_access: view offset, strides, storage bytes
  hash_artifact: artifact identity and optional build provenance
  hash_artifact_build: complete deferred stage contract
  hash_device_bytes: auxiliary per-device byte peaks
  hash_endpoint: External marker or device/value endpoint
  hash_build_contract: deferred binding/dispatch/work/resource digest
```

## 5. Scheduler trace: tasks to static windows

`recipe_scheduler::schedule` consumes `UnscheduledTask` and returns
`StaticSchedule { tasks: Vec<Task>, makespan }`
(`scheduler/src/static_schedule.rs:14-160`). It validates the topology and
discovery again, then:

```text
prepare_tasks
dependency_graph
critical_path_lengths
critical-path list schedule with measured resources
persist selected transfer lane claims
```

The scheduler boundary is intentionally narrow:

```text
UnscheduledTask = {
  id: TaskId,
  phase: RunPhase,
  dependencies: Vec<TaskId>,
  kind: TaskKind,
}
StaticSchedule = {
  tasks: Vec<Task>,
  makespan: Nanoseconds,
}
Route = {
  links: Vec<LinkId>,
  duration: Nanoseconds,
}
```

`UnscheduledTask` has no window and a transfer has no lane claims at this
boundary. The scheduler supplies both. `Route` is a planner search result,
not an executor task: a route with multiple links must be expanded into
dependency-chained tasks before `schedule` is called
(`scheduler/src/route.rs:10-30`).

### 5.1 Task preparation and measured timing

`CalculationTask` duration is `ceil(flops / measured_rate)`, clamped to at least
one nanosecond, and reserves every measured compute lane plus queue,
completion, and optionally `NoComputeTransferOverlap`
(`scheduler/src/static_schedule.rs:193-243`).

An internal transfer with an omitted route may be filled only by a direct
shortest route. A route longer than one link is rejected. A one-link transfer
is timed from measured link bandwidth, reserves every link lane and a
half-duplex direction resource when required, and reserves endpoint
non-overlap resources. External transfers use the endpoint device transfer
rate and external lanes (`scheduler/src/static_schedule.rs:245-376`). Metrics
have one nanosecond duration and their submission resources.

The scheduler's `shortest_route` is a measured Dijkstra search over directed
links. Each hop costs `ceil(bytes / link.bandwidth)`; equal-duration candidates
are ordered lexicographically by link IDs. Source equals destination yields an
empty route with one nanosecond duration. Unknown endpoints or an unreachable
destination are `InvalidTransfer` or `NoRoute`
(`scheduler/src/route.rs:20-110`). The planner's earlier `directed_routes`
enumeration is broader: it visits every simple directed path so `ensure_copy`
can trial-schedule each candidate and choose by the complete schedule, not just
the isolated route duration.

### 5.2 Dependency graph and phase barriers

`dependency_graph` rejects duplicate or unknown dependencies and dependencies
from a later phase. It then adds every edge from every lower phase task to
every higher phase task, ensuring that no Loop task starts while any Init task
is unscheduled and no Exit task starts before Loop (`scheduler/src/static_schedule.rs:458-510`).

Ready tasks are ordered by descending critical-path length and then ascending
`TaskId`. Each task starts no earlier than its dependency completion and phase
floor. `reserve_earliest` searches the first interval with no fixed-resource
conflict and at least one available lane in each compute/transfer group
(`scheduler/src/static_schedule.rs:512-669`). Selected resources are retained
in `Task.window` and transfer lane claims are persisted into the returned
`TaskKind::Transfer`.

### 5.3 Arena packing

`pack_arenas` consumes logical `ArenaObject`s and final capacity
(`scheduler/src/arena.rs:9-153`). For each device, objects are sorted by start
time and ID. The lowest aligned offset that does not overlap a live object is
selected. Non-overlapping lifetimes may reuse bytes. The layout size is the
maximum allocation end and must fit `recipe_usable`. Layouts are emitted for
all topology devices, including devices with no object allocations.

The scheduler's private stages preserve this representation order:

```yaml
prepare_tasks:
  input: UnscheduledTask
  output: PreparedTask(duration, Resource list)
  failures: [DuplicateTask, InvalidCalculationPlacement, InvalidTransfer,
             UnavailableCapability, ArithmeticOverflow, NoRoute]
dependency_graph:
  input: PreparedTask IDs and dependencies
  output: sorted successors and indegrees plus global phase edges
  failures: [UnknownDependency, InvalidLifecycleDependency]
critical_path_lengths:
  input: dependency graph and measured durations
  output: u64 downstream critical-path length per TaskId
  failures: [DependencyCycle, ArithmeticOverflow]
reserve_earliest:
  input: earliest start, duration, fixed resources, lane groups, prior windows
  output: first conflict-free ScheduleWindow and selected resources
  failures: [ArithmeticOverflow, UnavailableCapability]
persist_transfer_lane_claims:
  input: selected scheduler resources
  output: canonical TransferLaneClaim list in TaskKind::Transfer
  failure: UnavailableCapability when a complete claim set cannot be persisted
pack_arenas:
  input: ArenaObject lifetimes and final CapacityLedger
  output: one deterministic ArenaLayout per topology device
  failures: [InvalidTopology, InvalidTransfer, ArithmeticOverflow,
             InsufficientCapacity]
```

## 6. Preparation and realization fixed point

`recipe_prepare` owns the only public boundary where a measured profile becomes
a FinalizedBundle (`prepare/src/lib.rs:4-10`). `Preparer::prepare_program`
(`prepare/src/lib.rs:310-338`) validates the policy and measured profile, then
calls `prepare_program_validated`.

The preparation records preserve ownership across that boundary:

```text
PreparationPolicy = {
  stabilization_passes: NonZeroU32,
  stable_tail: NonZeroU32,
}
RealizationObservation = {
  artifacts: Vec<ArtifactIdentity>,
  resources: ResourceManifest,
  reservations: ReservationLedger,
  capacity_snapshots: Vec<CapacityLedger>,
}
RealizedCandidate<S> = {
  session: S,
  observation: RealizationObservation,
}
CandidateRejection = {
  candidate: CandidateIdentity,
  stage: Realize | Stabilize | FinalCapacity,
  detail: String,
}
PreparedSystem<C, S> = {
  bundle: FinalizedBundle,
  realization: RealizationProfile,
  catalog: C,
  session: S,
  external_outputs: Vec<PlannedExternalOutput>,
  attempted_candidates: usize,
  rejections: Vec<CandidateRejection>,
}
```

`PreparationPolicy::validate` requires `stable_tail <= stabilization_passes`;
both values are nonzero by type. The default is three complete warm passes and
a two-snapshot stable tail (`prepare/src/lib.rs:37-66`).

`PreparedSystem` keeps the successful opaque realization session alive beside
the immutable bundle. `into_parts` moves both together, while read-only
accessors expose the bundle, realization, catalog, session, output identities,
attempt count, and rejection evidence (`prepare/src/lib.rs:224-272`). A caller
cannot substitute a newly finalized bundle for the session that warmed the
candidate.

### 6.1 Candidate preparation algorithm

```text
input: StaticCalculationProgram, measured Topology + DiscoveryProfile
1. realizer.reservation_plan(topology, discovery)
2. ReservationLedger.validate
3. build optimistic planning CapacityLedger
4. ArtifactProvider.resolve(graph, topology, discovery)
5. call `plan_program_candidates` with the program, topology, discovery,
   artifact identities, reservations, and `optimistic_capacity`
6. for each ranked candidate:
   a. realizer.realize(candidate, catalog, reservations, policy)
   b. validate_observation and stabilization tail
   c. pack_arenas against final capacity
   d. construct RealizationProfile and hash it
   e. hash bundle identity
   f. FinalizedBundle::finalize_with_loop_schedule
   g. return PreparedSystem(bundle, realization, catalog, session)
7. if no candidate survives, CandidateExhaustion with every rejection
```

This is implemented at `prepare/src/lib.rs:340-511`. A candidate may be
rejected at Realize, Stabilize, or FinalCapacity. A retry is a next ranked
candidate, not a mutation of the rejected Draft. Rejected sessions are
destroyed before the next attempt. Fatal realization or teardown errors stop
the pipeline.

### 6.2 Optimistic versus observed capacity

`optimistic_planning_capacity` (`prepare/src/lib.rs:554-606`) uses measured
total capacity minus only the exact reservation, with zero overhead,
fragmentation, and headroom. It is explicitly an upper bound for candidate
enumeration, not a runtime snapshot. The final candidate is repacked against
post-warm measured capacity.

`RealizationObservation` carries artifact identities, the exact resource
manifest, reservations, and one `CapacityLedger` per bounded stabilization
pass (`prepare/src/lib.rs:98-107`). `validate_observation`
(`prepare/src/lib.rs:608-670`) requires exactly the policy pass count, validates
each snapshot, and requires all snapshots in the final stable tail to equal the
last snapshot. It then validates a provisional `RealizationProfile` against the
unchanged Draft. A changing capacity tail is a `Stabilize` candidate rejection
with field-level byte deltas.

The provisional profile used by `validate_observation` carries a deliberately
nonzero placeholder realization identity only to exercise
`RealizationProfile::validate`; the canonical realization hash is computed
after the observation has passed all checks (`prepare/src/lib.rs:656-670`,
`757-780`). It is never exported or handed to an executor.

### 6.3 Native artifact realization

`NativeCandidateRealizer` (`prepare/src/production.rs:935-1074`) compiles every
deferred `ArtifactBuildRecipe` during `BuildPhase::Realize`, or validates a
prebuilt bundle against the exact lowered stage ABI. It hands immutable
`NativeArtifact { identity, runtime }` pairs to one candidate-scoped driver.
The driver loads modules, reserves memory, realizes queues/tokens, warms the
maximum-concurrency schedule, and emits capacity snapshots. The compiler and
driver do not cross the prepared session boundary; the retained session owns
only resources that the final runtime can reuse.

`NativeArtifactProvider` accepts only a validated catalog whose targets occur
in measured discovery (`prepare/src/production.rs:78-175`).
`DeferredArtifactCompiler::materialize` first copies exact prebuilt identities,
then groups deferred builds by the one target selected by their calculation
tasks. It lowers each stage with its reserved artifact ID and launch width,
builds or inspects one CUDA cubin or HSA code-object bundle, and constructs one
`NativeArtifact` per stage with the shared image digest and exact build
provenance (`prepare/src/production.rs:254-575`). Duplicate, missing, or
cross-target artifact identities are fatal realization errors.

`ArtifactIdentity` binds ID, image digest, format, target, toolchain, entry
symbol, stage template, resource bounds, and optional deferred provenance
(`core/src/artifact.rs:214-266`). A missing catalog entry for a deferred build
is normal before Realize. A mismatched catalog entry is not a fallback path.

### 6.4 Preparation helper contract

```yaml
next_owned_program_candidate:
  operation: clone the next one-shot ProgramPlannerSearch candidate
reject_candidate:
  checks: ProgramPlannerSearch.reject(candidate)
  output: append CandidateRejection only after bookkeeping succeeds
destroy_rejected:
  operation: CandidateRealizer.destroy(session)
  failure: PrepareError.Teardown
optimistic_planning_capacity:
  operation: measured total - exact reservation, zero transient overhead fields
validate_observation:
  checks: exact pass count, every snapshot valid, stable tail equality,
          provisional RealizationProfile against unchanged Draft
capacity_stability_deltas:
  output: per-pass/device/field byte deltas for a changing stable tail
hash_realization:
  output: RealizationIdentity from Draft, observation, and policy evidence
hash_bundle_with_loop_domains:
  output: BundleIdentity from topology, discovery, Draft, realization,
          loop schedule, resources, capacity, and arena layouts
```

The helper ordering makes session ownership explicit: a rejected candidate is
destroyed before `reject_candidate` advances the search, while a successful
candidate is hashed and finalized without changing its Draft records.

## 7. RealizationProfile and FinalizedBundle

### 7.1 RealizationProfile

```text
RealizationProfile = {
  identity: RealizationIdentity,
  draft: DraftIdentity,
  candidate: CandidateIdentity,
  discovery: DiscoveryIdentity,
  topology: TopologyIdentity,
  artifacts: Vec<ArtifactIdentity>,
  resources: ResourceManifest,
  reservations: ReservationLedger,
  capacity: CapacityLedger,
}
```

`RealizationProfile::validate` (`core/src/plan.rs:2057-2119`) requires a
nonzero identity and exact equality of Draft, candidate, discovery, and
topology identities. It requires resources to equal the unchanged Draft
manifest. The realized artifact set must contain every prebuilt Draft artifact
unchanged and exactly one realized identity for every deferred build. Every
realized deferred artifact must validate, retain the build provenance and
resource envelope, and target the discovered device capability used by its
calculation tasks (`core/src/plan.rs:2121-2228`). Reservations and capacity are
validated again.

### 7.2 FinalizedBundle construction

`FinalizedBundle` has private fields and no public constructor. Only the three
Finalize entry points can create it (`core/src/plan.rs:2375-2517`):

```text
finalize(identity, topology, discovery, draft, realization, arena_layouts)
  => finalize_with_loop_iterations(identity, topology, discovery, draft,
     realization, arena_layouts, LoopIterations::ONE)

finalize_with_loop_iterations(identity, topology, discovery, draft, realization,
                              arena_layouts, loop_iterations)
  => assign IterationDomain::every(loop_iterations) to every loop task
  => finalize_with_loop_schedule(identity, topology, discovery, draft,
     realization, arena_layouts, LoopSchedule::new(loop_iterations, domains))

finalize_with_loop_schedule(identity, topology, discovery, draft, realization,
                            arena_layouts, LoopSchedule)
  => sort domains by task ID
  => validate bundle identity, Draft, RealizationProfile, domains, layouts
  => resolve every value to an arena address
  => move validated records into private immutable fields
```

The final record is:

```text
FinalizedBundle = {
  identity: BundleIdentity,
  loop_iterations: LoopIterations,
  loop_domains: Vec<LoopTaskDomain>,
  topology: TopologyIdentity,
  discovery: DiscoveryIdentity,
  draft: DraftIdentity,
  realization: RealizationIdentity,
  candidate: CandidateIdentity,
  artifacts: Vec<ArtifactIdentity>,
  artifact_builds: Vec<ArtifactBuildRecipe>,
  kernels: Vec<KernelTemplate>,
  tasks: Vec<Task>,
  resources: ResourceManifest,
  reservations: ReservationLedger,
  arena_layouts: Vec<ArenaLayout>,
  value_locations: Vec<ResolvedValueLocation>,
  value_aliases: Vec<ValueAliasContract>,
  init_images: Vec<InitDataImage>,
}
```

`releases` is validated as a Draft lifecycle contract but is not duplicated in
the FinalizedBundle. The runtime derives one release operation from each
finalized arena layout during ordered teardown; retaining a second release list
would duplicate the same authoritative device set.

Preparation computes the identity before calling Finalize. The realization
digest includes the unchanged Draft identity and candidate/profile identities,
build recipes, aliases, tasks, stabilization policy, observed artifacts,
resources, reservations, and every capacity snapshot
(`prepare/src/lib.rs:757-780`). The bundle digest additionally includes the
loop count and sorted task domains, realized artifacts and deferred builds,
resources, reservations, final capacity, and sorted per-device arena layouts
(`prepare/src/lib.rs:782-834`). Thus changing an address layout, a measured
capacity field, a loop activation domain, or an artifact image changes the
identity that consumers use for handoff and remote manifests.

No partially validated bundle is returned. The Finalize validator checks that
every loop task has exactly one domain, every non-loop task has none, each
domain is within the bundle loop count, fault readbacks share the checked
calculation domain, and internal-transfer consumers share the transfer domain
(`core/src/plan.rs:2230-2373`).

`validate_layouts` (`core/src/plan.rs:2632-2773`) requires one layout per
topology device, one allocation per object, correct device ownership, aligned
and in-bounds offsets, exact `layout.size == maximum allocation end`, capacity
fit, and no overlapping live objects. `resolve_value_locations` then checks
the final offset and end arithmetic and returns values sorted by `ValueId`.

### 7.3 Finalized read API

The private fields are exposed only through read-only accessors
(`core/src/plan.rs:2519-2629`):

```text
identity, loop_iterations, loop_domains, iteration_domain(task)
topology, discovery, draft, realization, candidate
artifacts, artifact_builds, artifact_build(artifact)
kernels, kernel(template), tasks, resources, reservations
arena_layouts, value_locations, value_aliases, init_images, init_image(device)
value_location(value), transfer_endpoints(task)
```

`transfer_endpoints(task)` resolves each device endpoint through the finalized
value location and rejects a device/value mismatch by returning `None`.
External endpoints remain `ResolvedTransferEndpoint::External`.

## 8. Runtime executor trace

The backend-neutral executor consumes only a `FinalizedBundle`; its public
typestate is `PreparedRun -> InitializedRun -> RunningRun -> ExitedLoop ->
ExitedRun` (`executor/src/lib.rs:4-10`). The bundle, not the backend, owns task
phase, schedule windows, dependencies, value addresses, resources, and
artifact identities.

### 8.1 Pre-init preparation

`PreparedRun::prepare_recoverable` (`executor/src/executor.rs:773-978`) first
derives fixed journal capacity from the exact task graph and loop domains. It
rejects repeated or unbounded loops unless `Backend::supports_loop_repetition`
is true. It then:

```text
PreparedPhases::new(bundle)
CompletionLedger::new(bundle.tasks())
backend.bind_resources(bundle)
realize_phase(init)
realize_phase(loop)
realize_phase(exit)
MetricMailbox::new(bundle.resources().metrics, bundle.tasks())
record LogicalEvent::Prepared
```

`realize_phase` creates one `PendingRequest` and one backend pending token for
every task before init (`executor/src/executor.rs:2085-2122`). No compiler,
loader, allocator, dynamic task, or lazy token is reachable from `submit` or
`poll` (`executor/src/backend.rs:318-429`).

`JournalCapacity::for_bundle` derives fixed storage from the bundle rather than
from a watchdog estimate (`executor/src/executor.rs:232-348`). It counts
non-loop tasks once, active loop-domain submissions for the retained detail,
metric emissions, external exit images, arena operations, and one terminal poll
record per task. Pending polls are not appended without bound: `RunJournal`
keeps one sorted `PendingPollCount` per finalized task and retains only the
first marker plus the exact count for repeated pending polls
(`executor/src/executor.rs:351-477`). Repeated loop detail is compacted after
the first iteration unless the caller explicitly asks to retain all detail.
Physical-call batches are fixed-size and capped by the backend ABI, so a
journal-capacity failure is preparation failure, not a runtime allocation path.

### 8.2 Bundle task projection

`PreparedTask::new` (`executor/src/executor.rs:1620-1809`) resolves Draft value
IDs and transfer endpoints into `ResolvedValueLocation`s and produces one of
four prepared work records:

```text
InitAdmission { device, destination, bytes, submission }
Calculation   { device, kernel_template, artifact, inputs, outputs, fault_flag, submission }
Transfer     { class: InternalTransfer | ExitTransfer, source, destination,
               bytes, route, lane_claims, submission }
Metric       { purpose, metric, slot, value, submission }
```

It assigns a loop domain to loop work and rejects every illegal phase/kind
combination. Init admission must be External to Device; loop transfers must be
internal; exit cannot admit external data. Missing finalized value locations
are backend protocol errors, not runtime allocation opportunities.

### 8.3 Init and admission images

`PreparedRun::initialize` (`executor/src/executor.rs:980-1051`) validates the
caller supplied `DeviceImage` set against every `InitDataImage`: exactly one per
device, exact image `ValueId`, exact byte length, no missing or unexpected
devices. It zeroes the finalized fault-flag ranges inside the image before the
first admission. It allocates each finalized arena layout, records
`ArenaAllocated`, runs the complete Init phase, and records `Initialized`.

The image validation requires the fault flag and packed image to share one
object and image span. A malformed image returns `DuplicateAdmission`,
`MissingAdmission`, `UnexpectedAdmission`, `AdmissionImageMismatch`,
`AdmissionSizeMismatch`, or a backend protocol error.

### 8.4 Loop execution and domains

`InitializedRun::start_loop` requires iteration zero and records `LoopStarted`
and `LoopIterationStarted`. `RunningRun::poll_with_progress_or_stop`
(`executor/src/executor.rs:1131-1249`) repeatedly:

```text
mark inactive ready tasks complete when their dependencies are complete
submit remaining tasks active on current iteration and schedule window
poll each pending token
complete metrics, fault readbacks, egress collection, and task ledger entries
```

Task activation is `IterationDomain::contains(iteration.index())`; an inactive
loop task is marked complete without a backend submission, so sparse domains do
not deadlock dependents. Dependencies and overlapping schedule windows are
checked before submission. A dependency on an incomplete predecessor is
accepted only when the backend explicitly supports same-queue pipelining and
the predecessor and successor use that same queue; otherwise completion is
required first (`executor/src/backend.rs:372-380`,
`executor/src/executor.rs:2319-2348`). A failed or stalled phase records `LoopFailed` and
preserves the backend in `RunFailure` for ordered teardown.

At terminal completion, the executor either accepts a graceful stop or obtains
the next iteration from `LoopIterations`. It resets only Loop completion bits,
rearms phase slots, and keeps the same arenas and pending tokens. It records
`LoopIterationCompleted`, optional `LoopStopAccepted`, and `LoopCompleted`.

### 8.5 Calculation, transfer, and metric backend work

The closed backend ABI (`executor/src/backend.rs:16-107`) receives only:

```text
InitAdmissionWork { destination, bytes, image, submission }
CalculationWork { run, iteration, device, kernel_template, artifact,
                  resolved inputs/outputs, optional resolved fault_flag }
TransferWork { resolved endpoints, bytes, route, lane_claims, submission }
MetricWork { iteration, purpose, metric, slot, resolved value, submission }
```

`BackendWork` has no compilation, loading, discovery, topology mutation, or
allocation variant. The backend must append bounded physical-call records and
return a nonblocking pending or complete result.

On a user metric completion, `MetricMailbox` publishes the value and the
executor records `MetricPublished`. On a fault readback, int32 zero records
`FaultChecked`; any nonzero int32 returns `DeviceFault`; a non-int32 result is
`BackendProtocol` (`executor/src/executor.rs:2496-2594`).

### 8.6 Exit, egress, and teardown

`ExitedLoop::exit_recoverable` runs the prepared Exit phase only after the loop
is terminal. For each device-to-external transfer, the backend collects an
`ExitImage { task, source: ResolvedValueLocation, bytes }`. After all Exit
tasks complete, the executor releases every arena in layout order, destroys
backend resources, records `ArenaReleased` and `Exited`, and returns
`ExitedRun` (`executor/src/executor.rs:1321-1424`).

Failure teardown attempts every remaining arena and resource release. The first
teardown error is retained as the primary cleanup error after the run failure;
later cleanup errors remain observable in the failure record.

### 8.7 Executor helper contract

```yaml
PreparedTask:
  active_on: IterationDomain.contains(current.index)
  backend_work: resolved PreparedWork -> BackendWork borrowed from fixed records
  external_exit: device-source ExitTransfer -> (ResolvedValueLocation, bytes)
  fault_reset: calculation fault location -> one pre-init reset descriptor
PreparedPhase.new:
  operation: filter bundle.tasks by RunPhase, resolve and sort by (window.start, TaskId)
PreparedPhases.fault_resets:
  operation: collect loop fault locations, sort by ValueId, deduplicate
realize_phase:
  operation: prepare_pending once per task into fixed TaskSlot list
validate_images:
  checks: exact per-device InitDataImage key and bytes, then zero fault ranges
poll_phase_once:
  operation: mark inactive ready tasks, submit active ready tasks, poll pending tokens
submit_slot:
  operation: Backend.submit or Backend.submit_loop_iteration with fixed ArenaSet
complete_slot:
  operation: metric publication/fault check/exit collection, then CompletionLedger.mark
collect_exit_image:
  operation: Backend.collect_exit into precomputed ExitImage capacity
run_phase_blocking:
  operation: bounded poll loop with progress reset and exponential backoff
teardown_resources:
  operation: release every arena, consume backend resource, destroy resources in order
```

`resolve_values`, `resolve_value`, and `resolve_transfer_endpoints` are
fail-closed adapters over FinalizedBundle accessors. They cannot allocate a
missing value location or infer an endpoint device
(`executor/src/executor.rs:2667-2691`).

## 9. Artifact consumers

### 9.1 Native executor execution plan

CUDA and HSA resources build `native_executor::ExecutionPlan` from the
FinalizedBundle (`native-executor/src/plan.rs:122-225`). It indexes runtime
images by `ArtifactId`, requires exactly the bundle's selected artifact set,
and validates each runtime image against its `ArtifactIdentity`:

```text
runtime.id == identity.id
SHA-256(runtime.bytes) == identity.digest
runtime ABI entry == identity.entry_symbol
identity.format == identity.target.abi
ABI workgroup width <= identity.resources.maximum_workgroup_lanes
runtime backend, architecture, ABI, and driver/code-object identity match target
```

For every calculation task it checks the kernel template, deferred build
provenance or kernel template fallback, launch element count, ordered buffer
operands, dtype, access mode, backing byte size, alignment, optional int32
fault-flag argument, canonical run-id and loop-iteration suffixes, and final
element-count argument (`native-executor/src/plan.rs:227-640`). A missing or
unexpected runtime image is an artifact contract failure, never a recompile in
the run lifecycle.

`plan_submissions` derives one native owner device and validates queue and
completion slots for every finalized task (`native-executor/src/plan.rs:642-714`).
CUDA and HSA `bind_partition` consume this plan exactly once; a warmed resource
must prove the same bundle, partition, tasks, artifacts, and layouts at handoff
(`native-executor/src/cuda.rs:295-321`, `native-executor/src/hsa.rs:329-356`).

### 9.2 Remote manifest and worker projection

`remote::Manifest::from_bundle` serializes bundle, Draft, and realization
identities plus sorted artifact IDs and digests (`remote/src/model.rs:261-365`).
`ProvisionedProgram::from_bundle` projects the same FinalizedBundle into worker
devices, projected task IDs, one image byte count per worker device, and
cross-machine one-hop transfers (`remote/src/model.rs:387-568`).

Remote classification uses finalized resolved endpoints. A worker owns local
calculations and metrics; a cross-machine device transfer becomes a scheduled
`CrossTransfer`; Init external admissions are represented by the one image, not
by a remote task. Any route with more than one link, endpoint mismatch, lane
claim mismatch, duplicate task, or identity mismatch is rejected.

`WorkerProjection::derive` validates topology identity, worker ownership,
arena/layout/reservation/init-image presence, task resource ownership, and
projected dependency closure (`executor/src/worker.rs:372-990`). The worker
session pre-realizes all local and external pending tokens before accepting
image chunks and enforces the same Init, Loop, Exit lifecycle.

### 9.3 Training and inference consumers

The production training and inference entry points create a
`NativeCandidateRealizer`, `Preparer`, and `PreparedRun` in one path
(`training/src/execute.rs:1203-1309`, `2087-2238`; `src/inference.rs:602-658`).
They hand the retained prepared native session to the local backend, then pass
the exact FinalizedBundle through Init, Loop, and Exit.

Inference packs declared inputs from `bundle.init_images()` and maps the
planner's `PreparedSystem::external_outputs()` to finalized Exit transfer tasks
(`training/src/execute.rs:1184-1196`, `2514-2634`). The mapping independently
checks logical value, physical device/value, Exit phase, external destination,
dtype, byte size, and non-overlap. Training performs the analogous checkpoint
mapping (`training/src/execute.rs:2405-2512`). The physical output image is
accepted only when it is the image collected for the exact finalized task.

Realized native kernels retained for reports are the immutable runtime images
that were loaded and warmed by the successful realization, grouped by format,
target, toolchain, and digest. The run does not create a replacement artifact
or alter the FinalizedBundle.

### 9.4 Local partition, host backend, and bridge consumers

`native-executor::LocalBackend::bind_resources` classifies every finalized
device and task into Host, CUDA, HSA, or Bridge ownership, then binds each
partition against the same bundle (`native-executor/src/local.rs:1647-1701`,
`3481-3579`). Calculations must belong to CUDA or HSA; metrics are owned by
the backend of their resolved value location; an internal transfer crossing
backend ownership is assigned to the bridge. A bridge task must have exactly
one finalized link. The partition validator rejects a multi-link task or a
calculation assigned to Host or Bridge.

`partition_candidate_artifacts` splits the successful `NativeArtifact` runtime
images by the GPU backend that owns their calculation tasks. It rejects an
artifact missing from either partition, an unexpected artifact, or one ID
assigned to both CUDA and HSA (`native-executor/src/local.rs:2396-2453`).
Before handoff, `validate_prepared_identity` compares topology, discovery,
Draft and candidate identities, tasks, templates, deferred builds, resources,
init images, reservations, and the complete artifact identity map
(`native-executor/src/local.rs:3430-3479`).

The Host backend consumes no kernel artifact. It rejects GPU calculation work,
but accepts finalized metrics and transfers after resolving value locations and
transfer endpoints. It requires an enforced-quota reservation for every bound
device and binds only devices that have finalized arena layouts
(`host/src/backend.rs:1121-1176`, `1313-1353`, `1596-1665`). The bridge likewise
pre-realizes one pending token per cross-backend transfer and validates every
phase, endpoint, byte count, route, lane claim, submission slot, and backend
class against the finalized task before it can be handed to the executor
(`native-executor/src/bridge.rs:437-490`, `1066-1100`, `1293-1323`).

## 10. End-to-end invariants

The following invariants are the shortest complete trace from source graph to
native execution:

```yaml
identity:
  Draft: nonzero and bound to one candidate, topology, discovery
  Realization: nonzero and bound to unchanged Draft, topology, discovery
  Bundle: nonzero and includes Draft, Realization, candidate, loop schedule
immutability:
  - FinalizedBundle fields are private
  - no runtime API can add tasks, values, artifacts, queues, lanes, or arenas
  - compiler and loader are unreachable after preparation handoff
phase:
  - every Init task is an external admission
  - every Loop calculation/metric has one domain
  - every Exit task waits for Init and Loop completion
values:
  - every value has one Draft binding and one Finalized location
  - locations are typed, device-owned, aligned, in bounds, and lifetime-safe
  - transfer endpoints resolve only through those locations
artifacts:
  - every calculation artifact resolves exactly once
  - deferred build provenance survives into the realized identity
  - runtime bytes and ABI match identity before bind
resources:
  - queues, completion slots, metric slots, lanes, and arenas are preallocated
  - overlap and measured concurrency are validated before Finalize
faults:
  - one int32 fault flag per checked calculation cohort
  - one exclusive readback after every checked cohort
  - all publication and Exit tasks depend on that readback
outputs:
  - one selected physical source per logical external output
  - only finalized Exit egress tasks produce external images
  - logical output mapping independently verifies the finalized source
```

## 11. Failure boundaries and recovery policy

```text
planner error
  -> candidate-local NoRoute / DependencyConflict / CandidateInfeasible /
     InvalidCapacity rejection, or fatal invalid graph/topology/catalog error

realizer error
  -> CandidateRejected only when the session is destroyed and the next ranked
     candidate may be attempted
  -> Fatal for profile, artifact, backend, or teardown failures

observation error
  -> Stabilize rejection after session destruction

pack/layout error
  -> FinalCapacity rejection after session destruction

Finalize error
  -> fatal PrepareError::Finalization; unchanged candidate and session are torn
     down, because a supposedly realized candidate violated its own contract

executor error
  -> RunFailure preserves bundle identity, backend ownership, journal, and
     cleanup error while attempting ordered arena/resource teardown
```

No boundary invents a fallback task, route, artifact, capacity, address, or
state value. The next safe action is selected from the evidence produced by the
current boundary: a measured route, a ranked candidate, a stabilized snapshot,
or a validated finalized address.

## 12. Trace index

```yaml
source_to_draft:
  graph_validation: planner/src/planner.rs:220-250
  candidate_envelopes_and_search: planner/src/planner.rs:28-169
  primitive_lowering: planner/src/planner.rs:359-433
  placement_enumeration: planner/src/planner.rs:519-571
  candidate_lowering: planner/src/planner.rs:979-1213
  init_images: planner/src/planner.rs:2276-2466
  transfer_chains: planner/src/planner.rs:2126-2274
  external_outputs: planner/src/planner.rs:2643-2749
  draft_validation: core/src/plan.rs:252-405
draft_to_schedule:
  static_scheduler: scheduler/src/static_schedule.rs:61-160
  lane_claims: scheduler/src/static_schedule.rs:245-449
  arena_packing: scheduler/src/arena.rs:13-153
schedule_to_realization:
  preparation: prepare/src/lib.rs:340-511
  observation_stability: prepare/src/lib.rs:608-670
  native_artifacts: prepare/src/production.rs:310-575
realization_to_bundle:
  profile_validation: core/src/plan.rs:2071-2228
  finalization: core/src/plan.rs:2398-2517
  address_resolution: core/src/plan.rs:2632-2866
bundle_to_execution:
  prepared_work: executor/src/executor.rs:1620-1934
  pending_tokens: executor/src/executor.rs:2085-2122
  init: executor/src/executor.rs:980-1035
  loop: executor/src/executor.rs:1072-1318
  exit: executor/src/executor.rs:1338-1371
artifact_consumers:
  native_execution_plan: native-executor/src/plan.rs:122-714
  cuda_bind: native-executor/src/cuda.rs:295-421
  hsa_bind: native-executor/src/hsa.rs:329-356
  remote_manifest: remote/src/model.rs:261-568
  worker_projection: executor/src/worker.rs:372-990
  training_outputs: training/src/execute.rs:2405-2512
  inference_outputs: training/src/execute.rs:2514-2634
  local_partition_and_handoff: native-executor/src/local.rs:1647-1701,2396-2453,3430-3579
  host_partition: host/src/backend.rs:1121-1176,1313-1353,1596-1665
  cross_backend_bridge: native-executor/src/bridge.rs:437-490,1066-1100,1293-1323
```
