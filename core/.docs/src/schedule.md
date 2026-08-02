<!--
  This document is a contract map for recipe_core::schedule.  The Rust types
  remain authoritative.  Every rule below names the producer, validator, or
  executor that enforces it so the document is mechanically checkable against
  the repository.
-->

# Static schedule contract

## 1. Parseable intent

```toml
[intent]
module = "recipe_core::schedule"
source = "core/src/schedule.rs"
role = "dependency-free vocabulary for immutable ahead-of-run execution"
side_effects = "none"
owns_runtime = false
owns_driver_calls = false
owns_validation = "ResourceManifest.validate only; Draft and Finalize validation is in core/src/plan.rs"
execution_model = "one immutable init -> loop -> exit task graph"
fundamental_work = ["calculation", "transfer"]
metric_model = "four-byte device readback transfer with a purpose tag"
schedule_time = "integer nanoseconds, half-open windows"
loop_coordinate = "zero-based u64 iteration index"
memory_model = "logical arena objects in Draft, physical offsets only after Finalize"
route_model = "one executor-visible directed hop; longer routes are dependency chains"
resource_model = ["queue slot", "completion slot", "compute lane", "transfer lane", "external transfer lane", "duplex resource"]
mutation_after_finalize = false
```

The module does not discover hardware, choose a device, calculate a route,
submit work, allocate memory, or poll a completion.  It supplies the values
that those stages exchange.  `scheduler::schedule` fills windows and transfer
lane claims.  `planner` builds values, tasks, domains, routes, and arena
objects.  `core::plan::DraftPlan::validate` and
`FinalizedBundle::finalize_with_loop_schedule` reject inconsistent products.
`executor` resolves the products into closed backend work and enforces the
same phase, dependency, and completion rules at runtime.

The schedule is intentionally a sidecar to the long-standing `Task` layout:
`Task` contains the static graph and time window, while `LoopSchedule` maps
each loop task to its activation domain.  The graph is never unrolled for
repeated loop iterations.

## 2. Source anchors and data flow

| Stage | Entry point | Schedule data consumed or produced | Observable guarantee |
| --- | --- | --- | --- |
| Program declaration | `program::StaticCalculationProgram::{new_with_metrics,every_iteration,from_ogdl}` | `LoopIterations`, `IterationDomain`, kernel domains, metric domains | Every source kernel has one valid domain. A metric is a four-byte scalar whose producer covers its domain. |
| Candidate lowering | `planner::lower_candidate` | `ValueSpec`, `TaskKind`, `LoopTaskDomain`, `InitDataImage`, aliases, resources | One init admission per device, loop calculations and internal copies, exit egresses, and fault/user metric tasks are materialized. |
| Route lowering | `planner::ensure_copy`, `build_transfer_chain` | `TransferTask`, one-link `route`, intermediate `ValueSpec`s, matching `LoopTaskDomain`s | A multi-link path becomes one dependency-chained task per link and one resident value per intermediate device. |
| Static scheduling | `scheduler::schedule` | `UnscheduledTask`, topology, measured `DiscoveryProfile` | Deterministic windows, queue/completion reservations, compute/transfer capacity reservations, half-duplex exclusion, and canonical transfer lane claims. |
| Resource compaction | `planner::compact_submission_resources` | scheduled `Task`s and windows | Queue and completion slots are interval-colored after scheduling, with separate owner classes for local and cross-device backend owners. |
| Arena contract | `planner::build_arena_contract` and `scheduler::pack_arenas` | `ArenaObject`, `ValueBinding`, `ArenaLayout` | Logical lifetimes are derived from task use and domain boundaries; physical offsets are deterministic and capacity-bounded. |
| Draft validation | `core::plan::DraftPlan::validate` | all Draft fields | Cross-field identities, phases, dependencies, values, routes, lanes, fault readbacks, contention, images, aliases, lifetimes, and one-upload/one-release rules are checked. |
| Realization | `prepare::Preparer::prepare_program_validated` | Draft, measured realization, stabilized capacity | The unchanged Draft is warmed and observed. A candidate is destroyed on realization, stabilization, or final-capacity rejection. |
| Finalization | `core::plan::FinalizedBundle::finalize_with_loop_schedule` | `LoopSchedule`, finalized layouts | Domains are sorted and validated, arena locations are resolved, and an immutable bundle identity is returned. |
| Local execution | `executor::PreparedRun` and `RunningRun` | finalized tasks/domains/locations/resources | Every pending token and arena is realized before init. Submission and polling use only preallocated work and exact dependencies. |
| Native execution | `native-executor::{cuda,hsa,local,bridge,plan}` | `BackendWork`, resolved endpoints, queue/completion slots, routes, lane claims | Driver contracts are rechecked against the immutable plan. Runtime submission cannot rewrite routes or allocate missing resources. |
| Remote execution | `executor::WorkerProjection`, `remote::{model,session}` | projected tasks, cross-transfer schedule stamps, duplex claims | A worker receives only its immutable projection. Cross-machine data is one-hop, direction-checked, statically stamped, and half-duplex serialized. |

Normative system rules that bound this vocabulary are in
`system-contract.md`, especially C6 through C9, C15 through C20, and the
failure and validator tables.

## 3. Loop coordinate model

### 3.1 Lifecycle phases

`RunPhase` is an ordered enum:

```text
Init < Loop < Exit
```

The ordering is used by `scheduler::dependency_graph`, by Draft dependency
validation, by executor phase selection, and by remote worker lifecycle.  The
legal semantic matrix is:

| Task kind | Init | Loop | Exit |
| --- | --- | --- | --- |
| `Calculation` | invalid | legal, GPU only | invalid |
| `Metric` | invalid | legal, four-byte readback | invalid |
| `Transfer(External -> Device)` | one logical admission per device | invalid | invalid |
| `Transfer(Device -> Device)` | legal when a candidate needs an init copy; planner normally places these in the loop | legal | legal as an internal exit copy |
| `Transfer(Device -> External)` | invalid | invalid | one logical egress per selected output |
| `Transfer(External -> External)` | invalid | invalid | invalid |

The scheduler adds explicit graph edges between every lower phase and every
higher phase, even when callers did not put those edges in the input.  It also
uses `phase_end` as a floor.  Planner lowering adds the same phase barriers to
its unscheduled graph.  The executor still executes phase vectors separately,
so a runtime backend cannot start loop work before init completion or exit work
before loop completion.

### 3.2 LoopIterations

`LoopIterations` is either `Finite(NonZeroU64)` or `Unbounded`.

| Operation | Finite behavior | Unbounded behavior |
| --- | --- | --- |
| `new(u64)` | `Some(Finite(n))` when `n != 0` | not applicable |
| `finite()` | returns `Some(n)` | returns `None` |
| `is_unbounded()` | `false` | `true` |
| `iteration(index)` | returns `None` when `index >= n` | returns `Some` for every `u64` index |
| `Display` | decimal nonzero bound | `unbounded` |

`ONE` is `Finite(1)`. `Default` is `ONE`.  No zero-length finite loop can be
represented.  `LoopIteration` can only be obtained through
`LoopIterations::iteration`, and exposes a zero-based `index` plus the exact
`total` bound.

The executor increments an active iteration with checked `u64` arithmetic.  A
finite run stops when `total.iteration(next_index)` returns `None`.  An
unbounded run has no implicit terminal index and requires an explicit graceful
stop control at a completed iteration boundary for training.  A backend that
does not return `supports_loop_repetition() == true` is rejected before init
when the bound is greater than one or unbounded.

### 3.3 IterationDomain

`IterationDomain` is a nonempty arithmetic progression:

```text
finite:   { first + k * stride | k >= 0, first + k * stride < end_exclusive }
unbounded:{ first + k * stride | k >= 0 }
stride > 0
```

The constructors and predicates have exact contracts:

| Function | Contract |
| --- | --- |
| `new(first,end,stride)` | returns `None` for zero stride or `first >= end`; otherwise stores a finite half-open end |
| `unbounded(first,stride)` | returns `None` only for zero stride |
| `every(iterations)` | `[0,iterations)` with stride one, or `[0,unbounded)` with stride one |
| `first()` | exactly `[0,1)` with stride one |
| `periodic(offset,period,iterations)` | finite uses `new(offset,total,period)`, unbounded uses `unbounded(offset,period)` |
| `contains(index)` | lower bound, optional exclusive upper bound, and modular stride all must match |
| `is_within(iterations)` | finite end must be `<=` finite loop bound; finite domains are allowed in an unbounded loop; an unbounded domain is valid only in an unbounded loop |

`is_within` validates the domain's declared end, not the last activated point.
The end therefore remains the canonical half-open bound even when the stride
does not land exactly on `end_exclusive`.

`LoopTaskDomain { task, domain }` assigns one exact domain to one finalized
loop task.  `LoopSchedule` carries one loop bound and the complete assignment
vector.  `LoopSchedule::new` itself is a plain value constructor.  Finalize
sorts assignments by task ID and performs all duplicate, task-kind, and
coverage checks.

The program layer performs an earlier domain check: every graph kernel has one
domain; a consumer cannot first run before its producer's first run; each user
metric domain is within the loop and is covered by its producer domain.  The
planner copies those domains to every calculation, internal transfer hop,
fault readback, and user metric task that it creates.

Finalized loop-domain invariants:

```text
for every task T:
    (T.phase == Loop) == (exactly one LoopTaskDomain(T) exists)
domain(T).is_within(bundle.loop_iterations())
for every fault readback R and checked calculation C sharing R.value:
    domain(R) == domain(C)
for every internal transfer X writing destination value V and loop consumer C of V:
    domain(X) == domain(C)
```

The second equality prevents one fault flag from being read back at a different
iteration cadence than the calculations that write it.  The third equality
prevents a stale or overproduced cross-device copy from being consumed under a
different cadence.

## 4. Time and submission vocabulary

`ScheduleWindow { start, end }` is a half-open interval in integer
nanoseconds.  `is_valid` requires `start < end`; `overlaps` is true exactly
when `start < other.end && other.start < end`.  Adjacent windows do not
overlap and may reuse an arena range, queue color, transfer lane, or completion
slot.  Scheduler operation durations are always at least one nanosecond,
including same-device copies and metric readbacks, so every emitted task window
is nonempty.

`SubmissionSlots { queue, completion }` names the logical queue and completion
token reserved for one task.  The slot IDs are typed stable IDs.  Planner
allocation starts IDs at one, but the schedule types do not themselves reject a
manually constructed zero ID.  `ResourceManifest`, Draft validation, native
planning, and backend binding reject unknown or wrongly owned slots.

Submission-device rules are:

| Work | Submission device |
| --- | --- |
| Calculation | `CalculationTask.device` |
| Metric | resolved device of `MetricTask.value` |
| External admission | destination device |
| Internal device copy | source device, including a same-device copy |
| External egress | source device |
| Cross-device transfer | source device; planner compaction additionally keeps `(source,destination)` owner classes separate |

`DraftPlan::validate` requires both the queue and completion slot to exist in
the manifest and to belong to this device.  Overlapping tasks may not share
either slot.  Non-overlapping tasks may share a compacted slot.  A native
backend may still decline to pipeline a reused queue, in which case the
executor waits for the prior completion before submitting the next task.

## 5. Values, aliases, and resolved addresses

### 5.1 ValueSpec and ValueBinding

```text
ValueSpec {
    id: ValueId,
    dtype: DType,             # only F32 or I32 in the scalar payload domain
    bytes: ByteCount,
    device: DeviceId,
    producer: Option<TaskId>,
}
```

Draft validation requires a known device and `bytes % dtype.byte_width() == 0`.
`producer` may be temporarily absent while lowering an output, but every value
must have a `ValueBinding`, and any present producer must name a task whose
window lies inside the binding object's lifetime.

`ValueBinding` stores `(value, object, object_offset)`.  The offset is relative
to an `ArenaObject`; it is not a physical arena address.  Validation requires:

```text
value.device == object.device
object.alignment >= dtype_width
object.alignment % dtype_width == 0
object_offset % dtype_width == 0
object_offset + value.bytes <= object.bytes
every producer and every task reference is within object.lifetime
```

`ResolvedValueLocation` is the Finalize product.  Its `arena_offset` is
`ArenaAllocation.offset + ValueBinding.object_offset`, checked for overflow,
device equality, arena bounds, and typed alignment.  Native backends receive
only this resolved form.

### 5.2 Alias permissions

`ValueAliasContract` records one actual input/output value pair and one
`AliasPermission`:

| Permission | Allowed byte-range relation |
| --- | --- |
| `Forbidden` | ranges must not overlap |
| `MayAliasExact` | disjoint ranges or exact same object, offset, and byte size |
| `MustAliasExact` | exact same object, offset, and byte size |

Ranges are half-open and only overlap inside the same arena object.  Overflow
while computing a range is treated as overlap.  The planner unions
`MustAliasExact` values into one object and adds dependencies from the
overwriting invocation to every old-value consumer.  An old-value exit reader
is a dependency conflict, because the output would be overwritten before it
could be exported.

Draft validation checks both explicit `value_aliases` and every alias rule in
the referenced `KernelTemplate`.  It also checks kernel input/output arity and
the ordered deferred-artifact bindings, so a task cannot change the alias or
operand interpretation at execution time.

### 5.3 Init images

`InitDataImage` is one packed logical admission for one device:

```text
InitDataImage {
    device: DeviceId,
    image: ValueId,       # resident value spanning the whole image
    bytes: ByteCount,     # exact transfer size, nonzero
    members: [InitDataImageMember],
}
```

Each member maps a stable graph-level `logical` value to a candidate-specific
resident `physical` value and an `image_offset`.  The member dtype, byte size,
typed alignment, and half-open image range are checked.  Members in one image
are nonoverlapping and logical IDs are unique.  Physical member IDs are unique
across all manifests.  The physical binding uses the image's object and
`image_binding.object_offset + member.image_offset` exactly.

The Draft task graph must contain exactly one `External -> Device` transfer in
`Init` for every topology device.  That task's destination value and byte count
must equal the image manifest, and the image value and every listed physical
member must name the upload task as producer.  Every topology device must have
exactly one image manifest.  The planner also reserves fault-flag bytes in the
image object; those implicit fault members are zeroed in the supplied image
before init, even though only graph-level external data members are serialized
in `InitDataImage.members`.

`ResolvedTransferEndpoint` replaces a device endpoint with its resolved value
location after Finalize.  `FinalizedBundle::transfer_endpoints(task)` returns
`None` when a task is not a transfer, a value has no location, or a resolved
location's device disagrees with its logical endpoint.  This is the last
logical-to-physical check before backend submission.

## 6. Task kinds and their invariants

### 6.1 CalculationTask

```text
CalculationTask {
    device: DeviceId,
    kernel_template: KernelTemplateId,
    artifact: ArtifactId,
    inputs: [ValueId],
    outputs: [ValueId],
    fault_flag: Option<ValueId>,
    work: FlopCount,
    submission: SubmissionSlots,
}
```

The task must be in `Loop` on a `GpuMemory` device.  Its artifact is exactly
one realized `ArtifactIdentity` or one deferred `ArtifactBuildRecipe`, never
both or neither.  The artifact stage identity, target, work, ordered inputs,
ordered outputs, and fault-flag requirement must match the kernel/build
contract.  Every input and output value is resident on the calculation device.
When present, the fault flag is exactly one aligned `I32` value with four bytes
on that device.  A checked scalar program therefore has a preallocated device
fault channel rather than a host-side exception path.

`work` is a cost input, not a runtime loop count.  The scheduler computes
`ceil(work / measured_flops_per_second)` nanoseconds and clamps a zero result
to one nanosecond.  Native CUDA and HSA adapters later validate the artifact
ABI, argument order, resolved locations, and dynamic `RunId` and
`LoopIteration` suffix arguments.  A kernel receives the zero-based
`LoopIteration.index()`, not a schedule-window timestamp.

### 6.2 TransferEndpoint and TransferTask

`TransferEndpoint::External` means the host, file, or remote protocol boundary.
`Device { device, value }` means one exact resident value on one storage
device.  For device endpoints, Draft validation requires:

```text
known value
value.device == endpoint.device
transfer.bytes == value.bytes
```

For a device-to-device task, source and destination dtypes must match.  A
same-device copy has an empty route.  A distinct-device task has one directed
link in its executor-visible route.  Longer paths are represented as a chain
of tasks and intermediate resident values, so each executor submission remains
one physical hop.

The phase and route contracts are:

```text
Init:  External -> Device, route = []
Loop:  Device -> Device, route = [] or [one link]
Exit:  Device -> Device or Device -> External, route = [] or [one link]
never External -> External
never an internal route on an external admission or egress
```

Core validation allows an internal device copy in any phase; planner lowering
normally uses `Init` only for the singular external admission and uses `Loop`
for recurrent copies.  The generic executor and native adapters implement the
same matrix, with remote workers intentionally restricting `Init` to their
image admission.

### 6.3 TransferLaneClaim

`TransferLaneClaim` is the static zero-based lane selected by the scheduler:

```text
Link { link: LinkId, lane: u32 }
External { device: DeviceId, lane: u32 }
```

An unscheduled transfer must have an empty claim list.  The scheduler reserves
all lanes in a group as alternatives, picks one free lane in the earliest legal
window, and persists only the selected lane.  A scheduled internal transfer
claims exactly one `Link` lane for every distinct route link, and no external
lane.  A scheduled external transfer claims exactly one `External` lane on its
endpoint device, and no link lane.  A same-device internal copy has no claims.

Draft validation requires claims to be strictly sorted and unique, link/device
references to exist, and lane indices to be below measured capacity.  It then
checks the endpoint-specific exact claim set.  Overlapping transfers cannot
claim the same exact lane.  Opposing links on a half-duplex transport cannot
overlap even when their lane indices differ.  Full-duplex directions use
distinct capacity resources and may overlap.

### 6.4 MetricTask

`MetricTask` is a specialized asynchronous four-byte readback.  It is not a
third model-work ontology.  Its `value` is copied to a preallocated
`MetricSlot` using the task's queue and completion slot.

| Purpose | Producer | Completion behavior |
| --- | --- | --- |
| `User` | planner selects the producer-resident scalar | `MetricMailbox` stores the newest sample per slot; replacing an unconsumed sample is success and never backpressures calculation |
| `FaultReadback` | planner groups all checked calculations that share one flag and domain | exactly one exclusive slot, direct dependency on every checked calculation, zero means `FaultChecked`, nonzero `I32` becomes `DeviceFault` |

Draft validation requires loop phase, a known value and matching submission
device, and a manifest metric slot mapped to the same metric ID.  Fault
readbacks require exactly one `I32` value of four bytes.  Every exit task and
every user metric must depend transitively on each applicable fault readback,
so an output cannot be published after an unchecked device fault.

### 6.5 Task and ResourceManifest

`Task { id, phase, window, dependencies, kind }` is the complete immutable
logical operation.  Dependencies are unique, known, phase-forward, and must
finish (`dependency.window.end <= task.window.start`) before the dependent task
starts in a validated Draft.  The explicit graph is acyclic.  The scheduler
also inserts the global phase edges described in section 3.1.

`ResourceManifest` fixes all non-arena resources:

| Field | Meaning and validation |
| --- | --- |
| `queues` | unique `QueueSlotId -> DeviceId` entries |
| `completions` | unique `CompletionSlotId -> DeviceId` entries |
| `metrics` | unique `MetricSlotId -> MetricId` entries |
| `pinned_staging` | at most one `DeviceBytes` entry per known device |
| `scratch` | at most one `DeviceBytes` entry per known device |

The manifest validator intentionally checks references and uniqueness, not
whether every device has a queue or staging entry.  Task-specific validation,
planner compaction, and native binding require each referenced resource and
the measured submission-queue limit.  `DeviceBytes` is the peak byte count
for that device's preallocated staging or scratch class.

## 7. Static scheduler algorithm

### 7.1 Admission and preparation

`scheduler::schedule(topology, discovery, tasks)` first validates topology
identity/ownership/route structure, rejects estimated scheduling properties,
and validates every measured discovery entry.  A theoretical topology seed
cannot reach this function.

Each `UnscheduledTask` has fixed ID, phase, dependencies, and kind.  The
scheduler rejects duplicate task IDs.  It then prepares duration and resource
alternatives:

```text
Calculation:
    duration = max(1, ceil(work / measured calculation rate))
    fixed = queue slot + completion slot
    grouped = every ComputeLane(device, 0..maximum_concurrent_tasks)
    add NoComputeTransferOverlap(device) when measured transfer/calculation overlap is false

Internal Device -> Device:
    if source != destination and route == []:
        shortest_route may fill route only when exactly one directed link exists
    reject route length > 1
    duration = 1 for same-device empty route,
               max(1, ceil(bytes / measured link bandwidth)) otherwise
    fixed = queue + completion + half-duplex direction when applicable
    grouped = every TransferLane(link, 0..maximum_inflight_transfers)
    add no-overlap resources for both endpoint devices when required

External admission or egress:
    route must be empty
    duration = max(1, ceil(bytes / discovered endpoint transfer rate))
    fixed = queue + completion + endpoint no-overlap resource when required
    grouped = every ExternalTransferLane(endpoint device, 0..maximum_inflight_transfers)

Metric:
    duration = 1
    fixed = queue + completion
```

Resource vectors are sorted and deduplicated before reservation.  A task that
already carries a lane claim is invalid at this unscheduled boundary.  The
link and endpoint rates, lane counts, and compute rates are measured or
explicit overrides from the validated profile; all duration arithmetic is
checked for overflow.

### 7.2 Route search and lowering

`shortest_route` uses directed adjacency sorted by `LinkId`.  Each hop costs
`max(1, ceil(bytes / measured link bandwidth))`; the path is charged
store-and-forward.  Dijkstra's queue orders `(elapsed, lexicographic link
path, device)`, so equal-duration paths choose the lexicographically smallest
link sequence.  Source and destination must be known; source equal to
destination returns an empty route with one nanosecond duration; no reachable
destination returns `NoRoute`.

The static scheduler refuses a shortest path containing more than one link.
`planner::ensure_copy` enumerates simple directed paths, appends a trial
dependency chain for every candidate path, and calls the scheduler to measure
the resulting task end and makespan.  It chooses `(end, makespan, source
device, source value, route)`.  It then allocates exactly the chosen chain:

```text
source copy -> hop task 0 -> intermediate value 0
             -> hop task 1 -> intermediate value 1
             -> ...
             -> final destination value
```

Every hop receives the consumer's `IterationDomain`.  The first hop depends on
the source producer; later hops depend on the previous hop.  A destination
already holding a copy for another transfer domain is a candidate-infeasible
condition rather than a hidden duplicate copy.

### 7.3 Dependency graph and critical path

The scheduler rejects unknown or repeated dependencies and a dependency from a
later phase.  It adds every pair `(predecessor, successor)` where the
predecessor phase is lower, then canonicalizes successor lists.  A topological
pass detects cycles.  Reverse topological dynamic programming computes:

```text
critical_path(task) = duration(task) + max(critical_path(successor))
```

with zero for a leaf.  Overflow is an `ArithmeticOverflow` schedule error.

Ready tasks are kept in a max heap keyed by `(critical_path, Reverse<TaskId>)`:
the longest remaining path wins, and a smaller task ID wins ties.  For each
ready task:

```text
dependency_end = maximum completed dependency end, or 0
phase_floor = 0 for Init,
              end of Init for Loop,
              max(end of Init, end of Loop) for Exit
earliest = max(dependency_end, phase_floor)
```

`reserve_earliest` scans forward to the first half-open gap.  Fixed resources
must be free for the entire candidate window.  Each grouped capacity selects
the first free lane; if every lane is occupied, the scan jumps to the earliest
release among that group.  An empty group is `UnavailableCapability`.

The selected window is inserted into every reserved resource.  For transfers,
the selected link or external lane is persisted as the canonical
`TransferLaneClaim` list.  `phase_end`, task end, and successor indegrees are
then updated.  The final `StaticSchedule.tasks` is sorted by task ID and
`makespan` is the maximum window end, or zero for an empty input.

### 7.4 Resource contention after scheduling

`DraftPlan::validate` independently rechecks the scheduled result.  In every
pair of overlapping tasks it rejects shared queue/completion slots, shared
exact transfer claims, opposing half-duplex transports, and unsupported
calculation/transfer overlap on a touched device.  It also counts half-open
start/end events for each directed link, each endpoint's external-transfer
capability, and each device's concurrent calculations against the measured
limits.  The validator treats an end at the same timestamp as preceding a
start, matching `ScheduleWindow::overlaps`.

This second pass is intentional.  Scheduler reservations prove the policy used
to create a candidate; Draft validation proves that the serialized candidate
still contains that policy after lane persistence, queue compaction, artifact
selection, and hashing.

## 8. Arena objects and physical packing

### 8.1 Logical arena contract

`ArenaObject { id, device, bytes, alignment, lifetime }` has no physical offset
in Draft.  `ArenaRelease { device }` is the logical one-free-per-device
contract.  `ArenaAllocation { object, offset }` and
`ArenaLayout { device, size, allocations }` are Finalize products.

The planner groups values into objects after applying exact alias unions.  It
derives each object's lifetime from:

1. every value producer task;
2. every calculation input/output/fault-flag use;
3. every transfer endpoint and metric read;
4. the full schedule window when a value crosses a lifecycle phase; and
5. the full loop window when a value is not refreshed before every activation
   across a repeated or unbounded loop.

`value_crosses_iteration_boundary` proves that a producer is a loop task whose
domain covers every reader domain and whose window ends before the reader
starts.  If that proof fails, the object remains live for the complete loop
window.  This is the mechanism that makes one physical arena safe for repeated
execution without per-iteration allocation.

### 8.2 `pack_arenas`

`scheduler::pack_arenas` groups objects by device, rejects unknown devices,
empty lifetimes, and non-power-of-two alignments, and sorts each device's
objects by `(lifetime.start, object.id)`.  For each object it considers zero and
the checked ends of already placed objects whose lifetimes overlap.  Each
candidate is aligned upward and accepted at the lowest offset with no live
byte collision.  It then sorts allocations by object ID and sets:

```text
layout.size = exact maximum(offset + object.bytes)
```

The exact size must fit `CapacityLedgerEntry.recipe_usable`.  No speculative
padding or fallback allocation is introduced.  A byte range may reuse an
offset only when the two object lifetimes do not overlap.

Finalize's `validate_layouts` repeats these checks, requires one layout for
every topology device and one allocation for every object, rejects duplicate
allocations, wrong-device allocations, bounds/alignment errors, capacity
overflow, and live overlapping objects, and requires the declared layout size
to equal the exact maximum allocation end.

## 9. Draft, realization, and FinalizedBundle

`DraftPlan` is offset-free but otherwise complete.  Its identity covers the
candidate, measured topology/discovery identities, loop bound/domains, values,
tasks, artifacts/builds, resources, arena objects/bindings, aliases, init
images, and releases.  Draft validation checks all of those cross-references.

The validation order is observable through error paths:

```text
topology, topology.scheduling, discovery
identity/candidate/discovery/topology identity matches
resources
values, kernels, realized artifacts, deferred artifact builds
tasks and task contention
arena objects
value bindings and alias rules
init image manifests and sole admissions
one release per topology device
```

`RealizationProfile` must retain the exact Draft, candidate, measured
topology/discovery identities, resources, reservations, and realized artifact
set.  Its capacity entries must be measured or explicitly overridden and must
account for reservation, runtime overhead, fragmentation, safety headroom, and
Recipe-usable bytes without overflow.

`Preparer` observes the candidate after native realization and maximum-
concurrency warm traces.  It requires exactly the configured number of
capacity snapshots and equality across the configured stable tail.  The final
stable snapshot is used to repack arenas.  A realization rejection destroys all
partially created native objects before trying another finite candidate.  A
successful realization cannot mutate the Draft.

`FinalizedBundle::finalize_with_loop_schedule`:

1. sorts the supplied `LoopTaskDomain` vector by task ID;
2. validates the nonzero bundle identity, Draft, realization, loop domains,
   layouts, and resolved value locations;
3. stores the exact tasks, resources, reservations, arena layouts, aliases,
   init images, artifacts, and loop sidecar behind private fields; and
4. exposes only read-only accessors and identity-keyed lookup methods.

The bundle identity includes topology/discovery, Draft and realization
identities, tasks, loop bound/domains, artifacts/builds, aliases, resources,
reservations, capacity, layouts, and allocation offsets.  Any schedule window,
route, lane claim, domain, slot, or physical offset change therefore produces a
different immutable identity.

## 10. Generic executor trace

### 10.1 Preparation before init

`executor::PreparedRun::prepare` derives bounded `JournalCapacity` from the
finalized task graph, rejects repeated/unbounded loops when the backend does
not support repetition, and builds three sorted `PreparedPhase` vectors.
`PreparedTask::new` resolves all value locations and transfer endpoints before
the run starts.  It rejects illegal kind/phase combinations and records the
loop domain for every loop task.

The backend then receives:

```text
bind_resources(bundle)
prepare_pending(PendingRequest { task, phase, class, submission }) for every task
```

All queues, completion tokens, native images, argument storage, staging, and
other resources reachable from those pending tokens must already exist before
`init`.  `submit`, `poll`, and error formatting are required to be
allocation-free and must not lazily load or discover anything.

### 10.2 Init admission

`initialize` validates exactly one `DeviceImage` per finalized image/device,
exact `image` value ID, and exact byte length.  Extra, duplicate, missing, or
misidentified images fail before native admission.  It allocates each finalized
arena once, zeroes every fault-flag range inside its packed image, and runs the
init phase to terminal completion.

The backend sees `BackendWork::InitAdmission` with the resolved destination,
exact byte count, submission slots, and image bytes.  Physical chunking or DMA
calls are backend accounting detail; the logical schedule still contains one
admission task per device.

### 10.3 Submission and polling

`poll_phase_once` is a bounded scheduler pass, not a second time scheduler.  A
loop task is active only when `IterationDomain::contains(current.index())` is
true.  An inactive task with all dependencies complete is marked complete
without a backend submission.  A remaining active task is runnable only when:

```text
all projected dependencies are complete
or a pending predecessor is on the same queue and the backend explicitly
   supports same-queue pipelining
no pending task with a non-overlapping static window blocks it,
unless that same queue pipeline capability applies
```

Runnable tasks are submitted in prepared window-start/task-ID order.  Loop
submissions use `Backend::submit_loop_iteration` with the exact
`LoopIteration`; init and exit use `Backend::submit`.  Every pending token is
polled once per pass.  Completion marks the task in the fixed
`CompletionLedger`, publishes a metric or fault result when applicable, and
collects an exit image only after its egress is terminal.

If no task remains and no token is pending, the phase is complete.  If tasks
remain, no token is pending, and no submission or completion made progress,
the executor returns `SchedulerStalled`.  Otherwise nonprogress polls advance
the finite watchdog and return `WatchdogExpired` at the configured bound.

### 10.4 Repeated and sparse iterations

At `start_loop`, iteration zero must exist.  A completed iteration is logged;
the stop callback is read only at that completed boundary.  Without a stop, the
next `LoopIterations::iteration(index + 1)` is selected.  The loop completion
ledger and every loop slot return to `Remaining`, while arenas, task graph,
queue slots, completion slots, metric slots, and native objects remain singular.

Before a repeated submission, a repeatable backend re-arms the terminal pending
token in place.  A sparse task whose domain starts later is skipped until its
first active index.  Its dependency completion still participates in the
current iteration, so a consumer cannot run before a skipped producer's
declared dependency is satisfied.

`RunJournal` retains fixed ordered detail for the first loop iteration and
compacts repeated loop events.  It keeps one fixed pending-poll counter per
finalized task, preserving exact counts without allocating in proportion to an
unbounded or large loop bound.

### 10.5 Exit and teardown

`into_exited_loop` is legal only after the loop phase is terminal and no loop
failure exists.  `ExitedLoop::exit` runs all exit tasks, collects each
`Device -> External` result into precomputed exit slots, releases every arena
exactly once, destroys backend resources, and records `Exited`.  Teardown
attempts every remaining arena release and resource destruction even after the
first failure; the primary error and first cleanup error are retained in
`RunFailure`.

`MetricMailbox` has one capacity-one slot per planned user metric.  Publishing
replaces an unconsumed older sample for that slot and never blocks calculation
polling.  Fault readbacks do not enter the user mailbox: zero is a successful
check, nonzero is a fail-closed `DeviceFault`, and an incompatible metric type
is a backend protocol error.

## 11. Native and host backend enforcement

### 11.1 Closed backend work

`executor::BackendWork` has only these variants:

```text
InitAdmission(InitAdmissionWork)
Calculation(CalculationWork)
InternalTransfer(TransferWork)
Metric(MetricWork)
ExitTransfer(TransferWork)
```

There is no compiler, loader, allocator, discovery, or topology-mutation
variant.  `TransferWork` carries resolved endpoints, byte count, route, lane
claims, and submission slots.  `CalculationWork` carries resolved input,
output, and optional fault-flag locations plus run and iteration identity.

### 11.2 Native plan and queue ownership

`native-executor::ExecutionPlan` validates runtime artifact IDs, digests, ABI
symbols, target backend/architecture, workgroup geometry, kernel argument
contract, and every task's immutable submission device/slots.  CUDA and HSA
resource binding creates the exact queue and completion objects named by the
manifest, loads each distinct native image once, and preallocates metric,
staging, scratch, argument, and exit storage.

Each native pending token captures task ID, phase, class, device, queue, and
completion ownership.  `prepare_pending` rejects any mismatch or second
preparation.  Submission rechecks the task contract and exact route/lane
claims.  CUDA and HSA completion objects transition `Available -> Active(task)
-> Available`; a second owner is `CompletionBusy`.  Polling cannot complete a
token that is not active, and a loop token can be rearmed only after terminal
completion.

CUDA internal transfers are same-context device copies.  HSA internal
transfers are same-session copies.  Cross-backend or cross-device ownership is
sent to the staged bridge, not silently converted to a different route.  Host
backend task subsets reject calculations and implement only admissions,
internal/exit copies, and four-byte metrics with exact arena bounds.

### 11.3 Staged cross-backend transfers

`StagedCrossBackend` realizes one source leg, one destination leg, staging
buffers, native completion tokens, and a bounded host staging worker for every
selected one-hop transfer before init.  Its immutable `TransferContract`
captures phase, endpoint devices/values, bytes, route, lane claims, and slots.
At submit and at Finalize handoff it compares all fields to the finalized task.

The physical bridge state machine is:

```text
Ready
  -> Source       (native source copy, when source is non-host)
  -> Middle       (host staging read/copy/write)
  -> Destination  (native destination copy, when destination is non-host)
  -> Complete
```

Host endpoints can enter `Middle` directly.  Polling advances one state at a
time and never reports completion before the destination leg is terminal.
Loop repetition resets both native legs, the host worker, and the middle job
only from `Complete` back to `Ready`.

## 12. Worker and remote projection

`WorkerProjection::derive` first validates topology identity, machine/node
ownership, worker role, nonempty local device set, one finalized arena,
reservation, and init image per local device.  It classifies each finalized
task as local calculation/metric/internal copy, local init admission, or an
external cross-boundary transfer.  Local values and artifacts are resolved;
all projected resources and queue/completion ownership are checked.

For each projected task the projection stores both original dependencies and
`projected_dependencies` containing only dependencies owned by this worker.
The master protocol is responsible for satisfying omitted cross-owner
dependencies by completing the corresponding transfer task.  A worker init
projection contains only its exact device admissions, never a hidden internal
init task.

An external worker transfer must be a planner-expanded one-hop task.  The
measured link direction must point into the local destination for ingress or
out of the local source for egress, and exactly one matching `Link` lane claim
must be present.  A worker session pre-realizes local and external pending
tokens, receives one digest-checked image per device in ordered chunks, and
then enters the loop.

`WorkerExecutionSession` checks phase, role, task state, projected
dependencies, exact byte counts, and static-window conflicts before every
dispatch.  External egress remains `AwaitingAck` until the master acknowledges
receipt.  Exit requires every projected loop task complete, then every exit
task complete before arena release and resource destruction.  Cancellation
quiesces native work before releasing arenas.

`remote::ProvisionedProgram` creates a bounded manifest from the Finalized
Bundle.  Init image admissions are not represented as cross transfers.  Every
cross-machine task is one hop with a matching route claim.  Cross transfers
are assigned strictly increasing per-direction schedule stamps, ordered by
static window start and task ID; master-to-worker stamps begin after all
reserved init chunks.  `MasterStorage` and worker storage own fixed task,
data, metric, and half-duplex token slots.  A half-duplex claim is acquired
before data submission and released only at terminal completion or data
acknowledgment.  Full-duplex directions do not share that token.  Wrong phase,
wrong stamp, duplicate status, unexpected data size, out-of-order ack, or
capacity exhaustion poisons the remote session.

## 13. User-facing training and inference boundaries

`training::prepare_and_execute_local_training_controlled` is the production
training path.  It validates that an unbounded loop has a stop source, runs
the complete Preparer fixed point, rejects any loop task with an external
endpoint, packs the exact finalized device images, hands the warmed native
session to `PreparedRun`, and executes `init -> loop -> exit`.  Stop requests
are observed only after a completed loop iteration.  External outputs are
mapped by the planned `(task, logical value, device, physical value)` evidence
and rechecked against finalized resolved endpoints.

Inference and KNN inference deliberately require `LoopIterations::ONE`, reject
user metric tasks, reject loop external transfers, and expose only finalized
exit output images.  A compiled inference bundle with any other loop bound or
user metric is a boundary error, not a request to reinterpret the schedule.

## 14. Failure map

### 14.1 Static scheduling failures

| Error | Direct cause | Planner handling |
| --- | --- | --- |
| `InvalidTopology` | topology structure or unschedulable property provenance | planning error |
| `InvalidDiscovery` | missing, unavailable, estimated, or mismatched capability | planning error |
| `DuplicateTask` | repeated task ID | invalid candidate |
| `UnknownDependency` | absent or repeated dependency | invalid candidate |
| `DependencyCycle` | explicit or phase-barrier graph cycle | `DependencyConflict` |
| `InvalidLifecycleDependency` | dependency points from a later phase | `DependencyConflict` |
| `UnavailableCapability` | no discovered endpoint capability, empty lane group, or incomplete selected claim set | schedule error or trial infeasible |
| `InvalidCalculationPlacement` | unknown or non-GPU calculation device | invalid candidate |
| `InvalidTransfer` | preselected unscheduled claims, external route, invalid endpoint route, or multi-hop executor task | invalid candidate |
| `NoRoute` | no directed path for a required copy | `NoRoute`, or trial infeasible |
| `ArithmeticOverflow` | duration, route, critical path, or window arithmetic overflow | `CandidateInfeasible` for candidate trials |
| `InsufficientCapacity` | arena or total arena/auxiliary bytes exceed Recipe-usable capacity | `CandidateInfeasible` |

The scheduler returns a typed `ScheduleError` with optional task and device
context.  It does not invent a fallback route, serialize a forbidden overlap,
or silently move a calculation to host storage.

### 14.2 Core validation failures

The machine-readable `ValidationCode` values most directly associated with
this module are:

```text
DuplicateId, UnknownReference, InvalidPhase,
DependencyPhaseOrder, DependencyScheduleOrder, DependencyCycle,
InvalidCalculationPlacement, InvalidIterationDomain,
InvalidExternalTransfer, InvalidRoute, InvalidLaneClaim,
InvalidFaultReadback, ResourceMismatch, ResourceContention,
CapabilityConcurrencyExceeded, InvalidLifetime,
MissingUpload, DuplicateUpload, MissingRelease, DuplicateRelease,
InvalidDataImage, MissingValueBinding, DuplicateValueBinding,
ValueBindingOutOfBounds, ValueBindingMisaligned, ValueLifetimeMismatch,
AliasViolation, DuplicateAliasRule,
DuplicateAllocation, MissingAllocation, AllocationOutOfBounds,
AllocationMisaligned, LiveAllocationOverlap, AddressOverflow,
InsufficientCapacity, CapacityOverflow
```

Validation accumulates all observed errors in one pass.  A failure is not a
signal to choose a near-match artifact, remove a device, add a retry, or add a
defensive watcher for an impossible event.

### 14.3 Runtime and backend failures

The generic executor fails closed on duplicate/missing/unexpected admissions,
invalid phase tasks, unresolved finalized values, backend protocol mismatches,
device fault codes, scheduler stalls, watchdog expiry, unsupported loop
repetition, metric sequence or pending-poll overflow, journal capacity
exhaustion, and exit image size/allocation failures.  It retains the primary
failure plus the first ordered cleanup failure and still attempts all teardown
steps.

Native adapters additionally reject missing or unexpected devices/artifacts,
ABI or target mismatches, missing queue/completion slots, completion ownership
conflicts, arena/value mismatches, unsupported routes, queue-limit overflow,
negative HSA signals, poisoned sessions, and any submitted work that differs
from its pre-realized contract.  The staged bridge reports missing bindings,
invalid state transitions, worker failure/disconnect, integer conversion
overflow, and contract differences.

Remote execution rejects manifest identity/configuration mismatches, unknown or
duplicate tasks/devices, wrong run IDs, wrong phase or schedule stamps,
backpressure/capacity exhaustion, codec or transport failures, driver faults,
and poisoned sessions.  None of these paths changes the immutable bundle.

## 15. Invariant checklist

The following checklist is intended to be copied into audits or code reviews.
Each item is a boolean property of the current implementation, not a proposed
future feature.

```text
S01  finite loop bound is nonzero
S02  every LoopIteration index is zero-based and bound-consistent
S03  every IterationDomain has nonzero stride and nonempty progression
S04  every loop task has exactly one domain and no non-loop task has one
S05  every domain is within the finalized loop bound
S06  fault readback and checked calculations share a domain
S07  internal transfer and every loop consumer of its destination share a domain
S08  every Task window is nonempty and half-open
S09  every dependency is unique, known, acyclic, phase-forward, and schedule-ordered
S10  scheduler inserts Init -> Loop -> Exit global barriers
S11  every calculation is Loop phase and GPU-resident
S12  every calculation input/output is resident on its calculation device
S13  checked calculations have one aligned four-byte I32 fault flag
S14  every transfer byte count equals each device endpoint value size
S15  every external admission is Init and route-empty
S16  every external egress is Exit and route-empty
S17  no External -> External task exists
S18  every executor-visible distinct-device route has one directed link
S19  every internal route link has exactly one selected Link lane claim
S20  every external transfer has exactly one selected External lane claim
S21  same-device internal copies have no link lane claim
S22  overlapping transfers share no exact lane claim
S23  opposing half-duplex links never overlap
S24  measured transfer/calc overlap capability gates mixed windows
S25  overlapping tasks share neither queue nor completion slot
S26  every topology device has exactly one init image/admission
S27  every topology device has exactly one arena layout and one release
S28  every value has one binding and every object has one finalized allocation
S29  live overlapping arena objects never overlap in bytes
S30  resolved value addresses are typed, aligned, bounded, and device-correct
S31  alias permissions match actual resolved byte ranges
S32  every fault cohort has one exclusive direct readback
S33  every exit/user metric publication waits for required fault readbacks
S34  all pending tokens and native resources are realized before init
S35  loop repetition only rearms terminal preallocated tokens
S36  loop progress never performs compile, load, discovery, allocation, or replan
S37  user metric publication is nonblocking and newest-value-wins
S38  exit collection occurs only after terminal egress completion
S39  arenas release before backend resource destruction
S40  any failed transition remains visible as a typed failure
```

These invariants reduce the schedule to the repository's two model work kinds,
calculation and transfer.  Dependencies, routes, queues, completion objects,
metrics, arena lifetimes, lane claims, phase barriers, and remote messages
order or realize those two kinds.  They do not become additional model
semantics.
