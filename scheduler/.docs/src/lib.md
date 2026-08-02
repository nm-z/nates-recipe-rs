# `recipe-scheduler` crate facade

`scheduler/Cargo.toml` defines the `recipe-scheduler` library package at
version `0.1.0`, edition 2024, with MIT licensing. Its only dependency is the
backend-neutral `recipe-core` crate. There is no feature table, binary target,
build script, global state, driver dependency, or runtime thread. The crate
root is `scheduler/src/lib.rs`.

The crate-level contract is deliberately small:

```rust
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
```

The implementation is synchronous and deterministic. It consumes a validated
topology, measured or explicitly overridden scheduling properties, and a
finite list of already lowered tasks. It returns either a complete static
schedule or a structured `ScheduleError`. It never probes hardware, compiles
an artifact, allocates a native resource, submits work, performs a data copy,
or changes the topology. Arena packing is also a pure calculation: it chooses
logical byte offsets for a caller-owned list of objects and does not allocate
the resulting memory.

## Role in the static pipeline

Recipe separates planning from execution. The scheduler is the point where
measured rates and concurrency limits become deterministic task windows,
transfer lane claims, and arena offsets:

1. `recipe-probe` and the profile validators establish a `Topology` and a
   `DiscoveryProfile`. Estimated topology values are rejected by
   `Topology::validate_scheduling_properties`, and unavailable or
   unmeasured discovery records are rejected by `DiscoveryProfile::validate`.
2. `recipe-planner` lowers a calculation graph into `UnscheduledTask` records.
   It supplies phase, dependency, task-kind, and submission-slot declarations,
   while leaving transfer lane claims and schedule windows empty.
3. `recipe-scheduler::schedule` validates that input, computes each duration,
   selects one lane from every constrained resource group, and returns
   `recipe_core::Task` values with nonempty half-open windows. A scheduled
   transfer carries the complete canonical `TransferLaneClaim` list selected
   by this step.
4. The planner derives arena-object lifetimes from the resulting windows and
   calls `pack_arenas` with a capacity ledger. The returned `ArenaLayout` values
   are provisional planning evidence retained beside the planned candidate;
   the logical objects and lifetimes are checked by `DraftPlan`, and the final
   layouts are checked by `FinalizedBundle`.
5. `recipe-prepare` repeats `pack_arenas` after native realization and capacity
   stabilization. It then passes the final layouts to
   `FinalizedBundle::finalize_with_loop_schedule`, which resolves every value
   to an immutable arena location.
6. `recipe-executor`, `recipe-host`, `recipe-native-executor`, and
   `recipe-remote` consume the finalized tasks, windows, routes, and lane
   claims. They do not replan or reinterpret scheduler output. The executor
   sorts each phase by `(window.start, task.id)`, and the host, native, worker,
   and remote projections check that submitted work exactly matches the
   finalized route and lane contract.

The scheduler therefore owns timing, route-cost calculation, logical resource
contention, and static arena placement. It does not own graph lowering,
artifact selection, realization, loop activation domains, or physical memory
allocation.

## Crate root and module map

`lib.rs` declares four private implementation modules in this order:

```text
arena, error, route, static_schedule
```

The modules are private so callers use one flat, stable import surface. The
root reexports exactly these names:

```rust
pub use arena::pack_arenas;
pub use error::{ScheduleError, ScheduleErrorKind};
pub use route::{Route, shortest_route};
pub use static_schedule::{StaticSchedule, UnscheduledTask, schedule};
```

| Module | Owns | Public root items |
| --- | --- | --- |
| `arena.rs` | Per-device first-fit placement of bounded objects into logical arenas | `pack_arenas` |
| `error.rs` | Scheduler-specific machine-readable and display diagnostics | `ScheduleErrorKind`, `ScheduleError` |
| `route.rs` | Measured directed-link shortest path and store-and-forward cost | `Route`, `shortest_route` |
| `static_schedule.rs` | Task preparation, dependency graph, critical-path priority, resource reservation, and schedule output | `UnscheduledTask`, `StaticSchedule`, `schedule` |

There are no public submodule paths. `recipe/src/facade.rs` exposes the crate
under `recipe::engine::scheduler`; direct workspace callers use
`recipe_scheduler`.

## Public data and function surface

### `UnscheduledTask`

```rust
pub struct UnscheduledTask {
    pub id: TaskId,
    pub phase: RunPhase,
    pub dependencies: Vec<TaskId>,
    pub kind: TaskKind,
}
```

It is `Clone + Debug + PartialEq + Eq`. This is the scheduler input record,
not a partial runtime task. The caller must provide a stable task identity,
the lifecycle phase (`Init`, `Loop`, or `Exit`), and all explicit dependency
IDs. A transfer must have an empty `lane_claims` list. An internal transfer
may have an empty route, in which case `schedule` can fill it only when the
shortest measured route is one direct link. A caller that needs a longer path
must lower it to dependency-chained one-hop tasks first.

`TaskKind` and all of its records come from `recipe-core`:

- `Calculation(CalculationTask)` names one GPU calculation, its work count,
  resident input and output values, optional checked-operation fault flag,
  kernel/artifact identities, and `SubmissionSlots`.
- `Transfer(TransferTask)` names external or device endpoints, byte count,
  an internal route, initially empty lane claims, and submission slots.
- `Metric(MetricTask)` is a four-byte device readback declaration with a
  metric purpose, value, metric slot, and submission slots.

The scheduler clones every input record into an internal `PreparedTask`. It
can therefore add a direct route and later persist selected lane claims
without mutating the caller's slice.

### `StaticSchedule`

```rust
pub struct StaticSchedule {
    pub tasks: Vec<Task>,
    pub makespan: Nanoseconds,
}
```

It is `Clone + Debug + PartialEq + Eq`. `tasks` contains every input task once,
with the same IDs, phases, dependencies, and task kinds except for scheduler
materialization of a direct route and transfer lane claims. Every returned
`Task.window` is a valid half-open interval (`start < end`). The vector is
ordered by ascending `TaskId` because it is collected from a `BTreeMap`; it is
not an execution-order trace. `makespan` is the maximum returned end time, or
zero when the input task list is empty.

### `Route` and `shortest_route`

```rust
pub struct Route {
    pub links: Vec<LinkId>,
    pub duration: Nanoseconds,
}

pub fn shortest_route(
    topology: &Topology,
    source: DeviceId,
    destination: DeviceId,
    bytes: ByteCount,
) -> Result<Route, ScheduleError>;
```

`Route` is `Clone + Debug + PartialEq + Eq`. Its links are ordered directed
link IDs. `duration` is the conservative store-and-forward cost for the
payload, not a reservation or an executor task. A path containing several
links is a planner candidate only; the scheduler's executor-visible transfer
contract accepts at most one link per task.

`shortest_route` is public because a planner or another static caller may need
the same measured path policy. It takes no `DiscoveryProfile`: it uses the
validated topology link bandwidths. Discovery availability and lane counts
are checked by `schedule` when a task is prepared.

### `pack_arenas`

```rust
pub fn pack_arenas(
    topology: &Topology,
    objects: &[ArenaObject],
    capacity: &CapacityLedger,
) -> Result<Vec<ArenaLayout>, ScheduleError>;
```

`ArenaObject`, `ArenaLayout`, `ArenaAllocation`, byte units, lifetimes, and the
capacity ledger are `recipe-core` records. The function returns one layout for
every topology device, including a zero-size layout with no allocations when
that device has no objects. Layouts and allocations are deterministic and do
not represent an allocation in a driver.

## Error model

`ScheduleErrorKind` is `Clone + Copy + Debug + PartialEq + Eq` and marked
`#[non_exhaustive]`:

| Kind | Actual source in this crate |
| --- | --- |
| `InvalidTopology` | `Topology::validate` or `Topology::validate_scheduling_properties` failed at a schedule or route entry point |
| `InvalidDiscovery` | `DiscoveryProfile::validate` failed at `schedule` |
| `DuplicateTask` | Two input records have the same `TaskId` |
| `UnknownDependency` | A dependency ID is absent, or one task repeats a dependency ID |
| `DependencyCycle` | The explicit plus lifecycle-barrier graph cannot be topologically ordered |
| `InvalidLifecycleDependency` | A task depends on a predecessor in a later `RunPhase` |
| `UnavailableCapability` | Required discovery capability is absent, a lane group is empty, or selected transfer claims do not match the endpoint/route contract |
| `InvalidCalculationPlacement` | A calculation names an unknown device or a non-GPU storage device |
| `InvalidTransfer` | Endpoint class, route, alignment/lifetime arena input, preselected lane claims, or external/internal transfer shape is invalid |
| `NoRoute` | `shortest_route` exhausted all directed paths |
| `ArithmeticOverflow` | Checked unit, route, critical-path, schedule-window, or arena-offset arithmetic overflowed |
| `InsufficientCapacity` | A device has no capacity entry, or packed arena size exceeds `recipe_usable` capacity |

```rust
pub struct ScheduleError {
    pub kind: ScheduleErrorKind,
    pub task: Option<TaskId>,
    pub device: Option<DeviceId>,
    pub message: String,
}
```

`ScheduleError` is `Clone + Debug + PartialEq + Eq` and implements
`std::error::Error`. `ScheduleError::new(kind, message)` starts with no task or
device context. `for_task` and `for_device` attach context by value and return
the modified error, so call sites can write a single error expression. Its
`Display` output is exactly:

```text
<Debug-formatted kind> [for task N] [on device D]: <message>
```

The optional fragments appear only when their fields are `Some`. Entry-point
validation errors carry no task context because they precede task preparation;
task-level failures normally carry the offending task, and capability or
capacity failures normally also carry a device.

## Core contracts consumed by the scheduler

The scheduler relies on invariants established by `recipe-core`, but still
calls the relevant validators at its public boundaries.

### Topology and scheduling properties

`Topology` contains storage devices and directed links. A `Device` is a
calculation target exactly when its `DeviceKind` is `GpuMemory`. Each
`DirectedLink` has a stable `LinkId`, endpoints, transport and duplex identity,
bandwidth, and a directional `maximum_inflight_transfers` property. A half
duplex reverse pair shares one `capacity_resource`; a full-duplex pair has one
resource per direction. `Topology::validate` proves ownership, link reversal,
transport pairing, and duplex coherence. `validate_scheduling_properties`
rejects every `PropertyProvenance::Estimated` capacity, transfer rate, FLOP
rate, bandwidth, or link lane count. The scheduler maps either validator's
aggregate diagnostics to `InvalidTopology`.

The `Property<T>` wrapper retains both value and provenance. Once validation
passes, route and task timing use the `.value` fields directly. The scheduler
does not substitute a default rate or turn an estimate into a schedule.

### Discovery profile

`DiscoveryProfile::validate(topology)` proves identity equality, complete and
unique device/link coverage, availability, asynchronous submission, measured
or overridden transfer rates and lane counts, and GPU calculation capability.
It also checks that discovered link bandwidth and lane counts match the
corresponding topology direction. `schedule` performs this validation once at
its entry point. During preparation it uses:

- `CalculationCapability.rate` and `maximum_concurrent_tasks` for calculations;
- per-device `TransferCapability.rate`, `maximum_inflight_transfers`, and
  `overlaps_calculation` for external transfers and compute/transfer overlap;
- the topology link's measured bandwidth, duplex resource, and lane count for
  an internal hop. Discovery validation has already proved those link values
  agree with the topology.

### Time and window units

`ByteCount`, `ByteOffset`, `FlopCount`, `Nanoseconds`, and nonzero rate types
are checked `u64` wrappers. `transfer_time_ceil(bytes, rate)` and
`calculation_time_ceil(work, rate)` compute
`ceil(work * 1_000_000_000 / rate)` with checked `u128` intermediate
arithmetic, returning `UnitError::Overflow` when the result does not fit in
`u64`. The scheduler maps that error to `ArithmeticOverflow` and clamps every
computed duration to at least one nanosecond. This clamp also makes zero-byte
transfers and zero-FLOP calculations occupy a valid, nonempty window.

`ScheduleWindow` is half-open. Its `overlaps` predicate is
`left.start < right.end && right.start < left.end`; tasks ending exactly when
another starts do not contend.

## Route selection in detail

`shortest_route` is implemented in `route.rs` as a deterministic Dijkstra
search over directed topology links.

1. It validates the topology and scheduling properties. A failure becomes
   `InvalidTopology` with the validator's display text.
2. It checks both endpoint IDs. An unknown source or destination is
   `InvalidTransfer` with `device` set to the unknown ID.
3. A same-device request returns `Route { links: vec![], duration: 1 ns }`.
   The one-nanosecond value keeps the route useful as a nonempty schedule
   candidate even though no physical hop is needed.
4. It builds a `BTreeMap<DeviceId, Vec<&DirectedLink>>` of outgoing links and
   sorts each adjacency list by stable `LinkId`.
5. The ready heap stores `Reverse((elapsed, path, current_device))`, and the
   best map stores `(elapsed, path)` for each device. A stale heap entry is
   skipped unless it exactly matches the best map entry.
6. Each edge cost is
   `max(1, transfer_time_ceil(bytes, link.bandwidth.value).get())`. Checked
   addition of the current elapsed time maps overflow to
   `ArithmeticOverflow` with message `route duration overflowed`.
7. A candidate replaces the known destination record when the tuple
   `(elapsed, link-id path)` compares lower. Consequently equal-duration paths
   are ordered lexicographically by their complete stable link-ID vectors.
8. Reaching the destination returns the path and `Nanoseconds::new(elapsed.max(1))`.
   If the heap empties, the result is `NoRoute` with a directed source and
   destination message.

This route search does not inspect link availability flags, discovery lane
counts, or duplex contention. Those are scheduling concerns. A caller that
uses a returned multi-link route must decompose it before creating an executor
task, as described by `UnscheduledTask` and `TransferTask`.

## Arena placement in detail

`pack_arenas` implements deterministic per-device first-fit placement for
half-open object lifetimes. It is intentionally independent of task ordering
or runtime memory APIs.

### Input checks

Each object is first grouped by `object.device` in a `BTreeMap`. The function
checks that the device exists in the topology, that `alignment` is nonzero and
a power of two, and that `lifetime.is_valid()` is true. Unknown devices map to
`InvalidTopology`; invalid alignment and empty lifetime map to
`InvalidTransfer`, each with the object's device context. Duplicate object IDs
are not diagnosed here, because `DraftPlan::validate` owns the full Draft
identity/reference check and the planner supplies unique IDs.

The function does not call `Topology::validate`, does not validate the complete
capacity ledger, and does not reject estimated capacity provenance itself. It
looks up one `CapacityLedgerEntry` per topology device at the end of each
device's placement pass. The caller is responsible for the ledger's prior
validation and provenance.

### Placement algorithm

1. Device IDs are copied from `topology.devices` and sorted. Thus the output
   layout order is independent of input object order.
2. Objects for a device are sorted by `(lifetime.start, object.id)`. The
   placed list records each object and its selected offset.
3. For each object, candidates begin with byte offset zero. For every already
   placed object whose lifetime overlaps, the checked end of that object's
   allocation is added as a candidate. Candidate offsets are sorted and
   deduplicated.
4. Candidates are aligned upward with `ByteCount::checked_align_up`. Alignment
   or allocation-end overflow maps to `ArithmeticOverflow` with the device
   context. A candidate is accepted when its byte interval does not overlap a
   live prior allocation. The first legal candidate is selected. Memory
   overlap is tested as two half-open byte intervals, and it is ignored for
   non-overlapping lifetimes, allowing byte reuse.
5. After all objects are placed, allocations are sorted by `ArenaObjectId`.
   `size` is the checked maximum allocation end, not a rounded capacity.
6. Missing capacity entry returns `InsufficientCapacity`. If `size` exceeds
   `entry.recipe_usable.value`, the same kind reports the planned and usable
   byte counts. Otherwise an `ArenaLayout { device, size, allocations }` is
   emitted.

The result always includes an empty layout for a topology device with no
objects, and therefore still requires a capacity entry for that device. The
planner later combines this logical arena size with staging and scratch peaks;
`pack_arenas` itself considers only the `ArenaObject` list.

## Static schedule entrypoint

```rust
pub fn schedule(
    topology: &Topology,
    discovery: &DiscoveryProfile,
    tasks: &[UnscheduledTask],
) -> Result<StaticSchedule, ScheduleError>;
```

The complete control flow is:

```text
validate topology
validate schedulable topology properties
validate discovery against topology
prepare task durations and resource alternatives
build dependency graph plus lifecycle barriers
compute reverse critical-path lengths
critical-path list scheduling with first-gap reservations
persist selected transfer lane claims
return tasks ordered by TaskId and global makespan
```

### Entry validation

`Topology::validate` and `Topology::validate_scheduling_properties` are called
in that order. Their errors become `InvalidTopology`. Then
`DiscoveryProfile::validate` runs and maps to `InvalidDiscovery`. No task is
prepared when one of these aggregate validations fails.

### Task preparation and duration

`prepare_tasks` inserts one `PreparedTask` per ID into a `BTreeMap`. A repeated
ID fails immediately as `DuplicateTask`. The source record is cloned before
matching its kind, and the resulting resource vector is sorted and deduped.

#### Calculation tasks

`prepare_calculation` requires an existing `GpuMemory` device. An unknown or
non-GPU placement is `InvalidCalculationPlacement`. The corresponding
discovered device must have a calculation capability, or the result is
`UnavailableCapability`.

Duration is `calculation_time_ceil(calculation.work, capability.rate.value)`,
clamped to at least one nanosecond. Arithmetic failure is
`ArithmeticOverflow`. The task claims its `QueueSlot` and `CompletionSlot`,
then presents one `ComputeLane(device, lane)` alternative for every lane in
`0..maximum_concurrent_tasks`. The first-gap allocator chooses one available
lane from that group, so the measured maximum becomes concurrency rather than
a requirement that every lane be held simultaneously. If the discovered
transfer capability says transfers cannot overlap calculation, the task also
claims the fixed `NoComputeTransferOverlap(device)` resource.

#### Internal device transfers

`prepare_transfer` first rejects any nonempty incoming `lane_claims` with
`InvalidTransfer`; claims are scheduler output, never input.

For two device endpoints:

* An empty route between distinct devices invokes `shortest_route`. A route
  with anything other than one link is rejected as
  `InvalidTransfer` because a multi-hop path must already be decomposed.
  The direct link is written into the cloned transfer.
* An explicitly supplied route longer than one link is rejected with the same
  kind. A zero-link route is legal only for a same-device copy.
* `Topology::validate_route` checks link existence, directed endpoint
  continuity, nonempty route for distinct devices, and the final destination.
  Its diagnostics become `InvalidTransfer`.
* A one-link hop costs
  `max(1, transfer_time_ceil(bytes, link.bandwidth.value).get())`. A missing
  link after route validation is `InvalidTransfer`; timing overflow is
  `ArithmeticOverflow`.
* Every lane index in the link's topology
  `maximum_inflight_transfers.value` becomes a `TransferLane(link, lane)`
  alternative. A half-duplex link also adds
  `HalfDuplexDirection(link.capacity_resource, link.id)`, which conflicts
  with the reverse direction on the same capacity resource.
* Both endpoint devices must be present in discovery. Missing endpoint
  capability is `UnavailableCapability`. For each endpoint whose discovered
  transfer capability cannot overlap calculation, the corresponding fixed
  `NoComputeTransferOverlap(device)` resource is added.

The same-device case has no link lane or duplex resource, but still has the
submission slots, endpoint capability checks, and any no-overlap resources.
It occupies one nanosecond.

#### External transfers

For `External -> Device` admission or `Device -> External` egress, an internal
route is invalid. The endpoint device must be discovered. The task gets one
`ExternalTransferLane(device, lane)` alternative per discovered transfer lane,
an optional no-overlap resource, and duration
`max(1, transfer_time_ceil(bytes, capability.transfer.rate.value).get())`.
External-to-external transfers always fail as `InvalidTransfer`.

External admission and egress phase restrictions are enforced later by
`DraftPlan::validate`: admission is `Init`, egress is `Exit`. The scheduler
itself accepts an endpoint-shaped task in any phase as long as the resource
model is valid; phase and endpoint contract validation remains a core Draft
invariant.

#### Metric tasks

Metrics are specialized four-byte device readbacks, not a third resource
capacity. Preparation assigns a one-nanosecond duration and only the declared
queue and completion resources. Value type, metric slot, fault-readback
shape, and loop phase are checked by `DraftPlan::validate` downstream.

### Persisting transfer lane claims

`persist_transfer_lane_claims` runs after a transfer window has been selected.
It extracts selected `TransferLane` and `ExternalTransferLane` resources,
sorts and deduplicates them, and checks that the complete claim set matches the
endpoint contract:

* device-to-device tasks must claim only link lanes, and the set of claimed
  links must equal the task's route links. Same-device copies therefore claim
  no lanes;
* external-to-device and device-to-external tasks must claim exactly one
  external lane for the endpoint device;
* external-to-external is invalid.

Any mismatch is `UnavailableCapability`. A valid set is written into the
cloned `TransferTask`, and the resulting `TaskKind::Transfer` is stored in the
returned `StaticSchedule`. Claims are canonical because they are sorted by
`TransferLaneClaim`'s derived ordering.

## Dependency and lifecycle graph

`dependency_graph` builds a `BTreeSet<(predecessor, successor)>` so edges are
stable and cannot be inserted twice. For each explicit dependency it checks:

- the dependency ID exists, otherwise `UnknownDependency`;
- the same dependency is not repeated in one task, also
  `UnknownDependency`;
- the predecessor's phase is not later than the dependent task's phase,
  otherwise `InvalidLifecycleDependency`.

After explicit edges, the scheduler adds every edge from every lower phase to
every higher phase. These global barriers are intentional: all `Init` work
must be ordered before all `Loop` work, and all `Init` and `Loop` work must be
ordered before any `Exit` work, even when the caller omitted a direct
dependency. Successor lists are sorted and indegrees are initialized for all
task IDs.

`critical_path_lengths` topologically orders this graph with a min-ID
tie-breaker, then walks it in reverse. A task length is its duration plus the
maximum successor length, with checked addition. A cycle returns
`DependencyCycle`; overflow returns `ArithmeticOverflow`. The critical path
length is a priority estimate only. It does not alter the task's own duration.

## Deterministic list scheduling

The scheduler maintains:

```text
task_end:     TaskId -> Nanoseconds
scheduled:    TaskId -> Task
reservations: Resource -> Vec<ScheduleWindow>
phase_end:    [Init end, Loop end, Exit end]
```

Ready tasks are held in a max heap keyed by
`(critical_path_length, Reverse(TaskId))`. Thus a larger remaining critical
path wins, and equal priorities select the lower stable task ID. For each task:

1. `dependency_end` is the maximum end time of its scheduled dependencies.
2. `phase_floor` is zero for `Init`, the current global init end for `Loop`, or
   the maximum init/loop end for `Exit`.
3. The earliest candidate start is the maximum of those two values.
4. `reserve_earliest` finds the first legal half-open window at or after that
   time. Every selected resource receives the window, the task kind receives
   persisted transfer claims, and the phase end and task end maps are updated.
5. Successor indegrees are decremented. A successor whose count reaches zero
   enters the ready heap with its critical-path priority.

If the heap empties before every prepared task is scheduled, the graph is
reported as `DependencyCycle`. Otherwise the final makespan is the maximum
window end. The task map is consumed into ascending `TaskId` order.

### Resource classes

The private `Resource` enum has these meanings:

| Resource | Contention rule |
| --- | --- |
| `Queue(QueueSlotId)` | One task at a time for the exact submission queue slot |
| `Completion(CompletionSlotId)` | One task at a time for the exact completion slot |
| `ComputeLane(DeviceId, u32)` | One calculation per selected measured concurrency lane |
| `TransferLane(LinkId, u32)` | One transfer per selected directed-link lane |
| `ExternalTransferLane(DeviceId, u32)` | One external admission/egress per selected device lane |
| `HalfDuplexDirection(DuplexResourceId, LinkId)` | Conflicts with the opposite direction sharing the same capacity resource |
| `NoComputeTransferOverlap(DeviceId)` | Conflicts between calculation and transfer touching a device when discovery reports no overlap |

`Queue`, `Completion`, `HalfDuplexDirection`, and
`NoComputeTransferOverlap` are fixed resources. Compute and transfer lanes are
resource groups: a task presents all measured lane alternatives, and the
allocator selects one free member of each group. Resource vectors are sorted
before scheduling, so a lower lane ID wins whenever several are available.

### First-gap reservation

`reserve_earliest` splits a prepared resource list into fixed resources and a
sorted `BTreeSet` of groups (`Compute(device)`, `Transfer(link)`, and
`ExternalTransfer(device)`). `first_gap` then repeats until a window works:

1. It checked-adds the duration to the candidate start. Overflow is
   `ArithmeticOverflow`.
2. If a fixed resource has an overlapping reservation, the start advances to
   the latest conflicting end and the candidate is retried.
3. For each group, each alternative lane is tested. The first lane without an
   overlapping reservation is selected. If every lane conflicts, the start
   advances to the earliest release among that group's lanes. An empty group
   returns `UnavailableCapability`.
4. When all groups have a selected lane, fixed resources plus selected lanes
   are sorted and returned with the candidate window.

Reservations use `ScheduleWindow::overlaps`, so adjacent windows can reuse a
queue, completion slot, lane, duplex capacity, or no-overlap resource. A
half-duplex conflict is found by scanning reservations for the same
`DuplexResourceId` with a different direction link. Same-direction contention
is still governed by the directional `TransferLane` group.

The algorithm never sleeps, polls, or asks a backend whether a resource is
free. It reasons solely from the measured limits and windows already reserved
in this static schedule.

## Output invariants and downstream validation

The scheduler establishes the invariants that later core validation and
execution depend on:

- every task ID is unique and every returned window is nonempty;
- dependencies refer to known tasks, are unique per task, respect phase order,
  and complete no later than the dependent task's start;
- all lifecycle phases are globally ordered;
- calculations are placed on GPU devices and use one measured compute lane;
- internal executor-visible transfers have zero or one directed link, and
  external transfers have no internal route;
- every transfer has a canonical lane claim set selected by the measured lane
  capacity;
- queue and completion slots, fixed no-overlap resources, transfer lanes, and
  half-duplex directions do not overlap in conflicting windows;
- `makespan` covers every returned task end.

`DraftPlan::validate` repeats and extends these checks. It verifies transfer
endpoint values and bytes, exact lane claim cardinality and bounds, route
endpoint continuity, phase-specific external admission/egress, resource
contention, link and external-transfer concurrency events, measured compute
concurrency, and compute/transfer overlap capability. It also requires one
init data-image admission per topology device, one fault readback per checked
fault cohort, and a complete acyclic task graph. A scheduler-produced schedule
can therefore still be rejected by core if a caller supplied inconsistent
value, artifact, metric, or Draft metadata; scheduler success is not a bypass
of Draft validation.

`pack_arenas` establishes the logical offset invariants later checked by
`FinalizedBundle`:

- one layout per topology device;
- every object allocated exactly once on its own device;
- offset alignment and checked object ends;
- layout size exactly equal to the maximum allocation end;
- no overlapping byte ranges for objects whose lifetimes overlap;
- arena size no greater than the device's Recipe-usable capacity.

Finalize combines those layouts with `ValueBinding::object_offset` and checks
resolved value locations, type alignment, lifetime containment, and capacity
again before exposing an immutable bundle.

## Call graph and real callers

### Route and schedule calls from the planner

`planner/src/planner.rs` imports
`{ScheduleErrorKind, UnscheduledTask, pack_arenas, schedule}` from the crate.
Its `LoweringState::next_submission` allocates a stable task ID and queue and
completion slot IDs with that same numeric value, and records those slots in
the Draft `ResourceManifest`.

During `lower_candidate` the planner:

1. creates one `Init` external-to-device transfer per required topology device
   for the packed data image;
2. lowers each primitive stage to a `Loop` calculation task, assigning a loop
   domain and dependencies on stage barriers and resident value producers;
3. builds loop transfer chains. The planner's `directed_routes` enumerates all
   simple directed paths, and `build_transfer_chain` turns a selected path
   into one dependency-chained `TransferTask` per link with intermediate
   resident values. Same-device copies use an empty route;
4. adds fault readbacks, user metrics, alias dependencies, and `Exit` external
   output trials;
5. adds explicit phase-barrier dependencies and calls `schedule` on the full
   unscheduled list;
6. maps scheduler errors. Dependency cycles and invalid lifecycle dependencies
   become `PlannerErrorKind::DependencyConflict`; arithmetic overflow and
   insufficient capacity become `CandidateInfeasible`; `NoRoute` becomes
   `NoRoute`; other kinds become `Schedule`;
7. compacts the scheduler's submission slots to the measured per-device queue
   limit, builds logical arena objects from task windows, calls `pack_arenas`
   with planning capacity, and validates total arena/staging/scratch capacity;
8. stores the scheduled tasks, logical values, bindings, init images, releases,
   and loop-domain sidecar in a `DraftPlan`, then runs `DraftPlan::validate`.

The same scheduler entrypoint is used by `trial_chain_timing` while choosing a
route or an external egress. A trial appends one candidate chain to the current
tasks, schedules the complete real graph, and reports the trial task end and
global makespan. Arithmetic overflow, insufficient capacity, and no route are
treated as an unschedulable trial; dependency conflicts become a planner
error; all other scheduler errors are surfaced as a schedule failure. This
means route selection is based on the resulting graph's measured static
contention, not only on isolated link cost.

The planner does not call `shortest_route` directly when it is comparing
multi-hop paths. It enumerates and expands those paths itself so every hop and
intermediate value is visible to `schedule`. `shortest_route` remains the
automatic direct-route fill policy for a caller that submits an empty-route
distinct-device task directly to `schedule`.

### Final capacity call from preparation

`prepare/src/lib.rs` imports only `pack_arenas` from this crate. In
`Preparer::prepare_program_validated`, planning first uses an optimistic
capacity ledger to obtain ranked candidates. Each candidate is realized and
its post-warm capacity snapshots are validated. The stabilized ledger is then
passed to `pack_arenas` with the unchanged Draft arena objects. A packing or
capacity failure rejects that candidate, destroys its native session, and lets
the one-shot candidate search continue. A successful layout is handed to
`FinalizedBundle::finalize_with_loop_schedule`, along with the planner's exact
loop domains. Thus the scheduler's pure arena result is the final fixed-point
bridge from opaque measured capacity to immutable addresses.

### Runtime consumers

The scheduler has no runtime caller that invokes it during `init`, `loop`, or
`exit`. The finalized task records it produced are consumed later:

- `executor/src/executor.rs` constructs `PreparedPhase` values from
  `FinalizedBundle::tasks()`, sorts each phase by `(window.start, task.id)`,
  and uses the resolved endpoints, dependencies, and submission slots for
  execution and completion accounting;
- `host/src/backend.rs` and `executor/src/worker.rs` classify transfers and
  require the finalized one-hop route and exact lane claims;
- `native-executor` CUDA, HSA, and local paths bind calculations and transfers
  to the finalized task contract and reject mismatched routes or claims;
- `remote/src/model.rs` and `remote/src/session.rs` project cross-boundary
  tasks, using window starts, one-hop routes, duplex resources, and link claims
  to construct worker schedules.

These consumers are downstream evidence of the scheduler contract, not
alternate scheduling implementations. None calls back into `schedule` to
repair a task or invent a route.

## Determinism, scope, and limits

Determinism comes from stable IDs and ordered collections throughout the crate:

- topology adjacency and arena objects are sorted by stable IDs;
- route ties compare complete link-ID vectors;
- dependency edges and successor lists use ordered sets/maps;
- ready tasks prefer larger critical paths, then lower task IDs;
- lane alternatives and selected resource lists are sorted;
- output tasks and arena allocations are sorted by stable IDs.

The scheduler is intentionally bounded by the caller's finite task and object
lists. It does not unroll `LoopIterations`, assign loop activation domains, or
simulate repeated runtime iterations. The one static loop graph is scheduled
once; `FinalizedBundle` carries the separate `LoopSchedule` sidecar for runtime
activation. It also does not account for staging or scratch bytes when packing
arenas; the planner computes those auxiliary peaks and checks their sum against
the same capacity ledger.

There are no scheduler-local tests, examples, or additional source files in
the current crate manifest. Structural compilation is exercised through the
workspace build and check commands. Logical correctness is established by the
planner and preparation end-to-end paths described above, which use real
topology, discovery, capacity, and finalization boundaries rather than mocked
resource state.

## Source index

The implementation details documented here are concentrated in:

- [`scheduler/src/lib.rs`](../../src/lib.rs): attributes, module declarations,
  root documentation, and reexports;
- [`scheduler/src/error.rs`](../../src/error.rs): error kinds, context
  builders, and display formatting;
- [`scheduler/src/route.rs`](../../src/route.rs): topology validation,
  deterministic measured shortest route, and path timing;
- [`scheduler/src/arena.rs`](../../src/arena.rs): object validation,
  lifetime-aware first-fit placement, layout sizing, and capacity checks;
- [`scheduler/src/static_schedule.rs`](../../src/static_schedule.rs): public
  task/result records, task preparation, route filling, lane claims,
  dependencies, critical paths, resource groups, and first-gap reservation;
- [`planner/src/planner.rs`](../../planner/src/planner.rs): the real task and
  route lowering caller, scheduler trial timing, arena-object lifetime
  construction, and Draft assembly;
- [`prepare/src/lib.rs`](../../prepare/src/lib.rs): stabilized-capacity arena
  repacking before Finalize;
- [`core/src/schedule.rs`](../../core/src/schedule.rs),
  [`core/src/topology.rs`](../../core/src/topology.rs),
  [`core/src/discovery.rs`](../../core/src/discovery.rs), and
  [`core/src/plan.rs`](../../core/src/plan.rs): the typed records and
  downstream validators that define the scheduler's input and output
  contracts.
