# `recipe-scheduler`

`recipe-scheduler` is Recipe's deterministic AOT boundary for two related
contracts: it assigns schedule windows and transfer lanes to a fully lowered
task set, and it packs the task lifetimes into per-device byte arenas. It also
exports the measured-time route helper used while lowering a transfer. The
crate consumes only `recipe-core` contracts. It does not probe hardware,
compile or load artifacts, allocate native memory, submit work, or run a
runtime scheduler.

The results are owned values. The crate does not keep a runtime service or a
mutable global plan. Once `recipe-planner` has validated the returned
`StaticSchedule` and arena layouts into a `DraftPlan`, those values become the
fixed schedule and offset-free-to-offset-resolved inputs of the immutable
`FinalizedBundle` lifecycle.

## Boundary and end-to-end role

The scheduler sits between measured discovery and an executable draft:

```text
recipe-probe / recipe-cluster
  measured Topology + DiscoveryProfile
                |
                v
recipe-planner
  placement choice, primitive lowering, values, dependencies,
  one-hop transfer candidates, arena lifetimes
                |
                +--> recipe_scheduler::schedule
                |      Task windows + canonical transfer lane claims
                |
                +--> recipe_scheduler::pack_arenas
                       per-device ArenaLayout offsets and sizes
                |
                v
  DraftPlan (tasks, resources, arena objects, value bindings)
                |
recipe-prepare
  realizes the exact Draft, observes stable capacity, repacks arenas
                |
                v
  FinalizedBundle -> recipe-executor / native-executor / remote workers
```

`recipe-planner` is the direct runtime caller of `schedule`. Its candidate
lowering creates `UnscheduledTask` values, adds alias and lifecycle
dependencies, calls `schedule`, and uses the result to construct arena
lifetimes and a `DraftPlan`. The same scheduler entry point is used by planner
trial schedules while choosing a transfer route or an external output copy.
`recipe-prepare` does not schedule again. After native realization and bounded
capacity stabilization it calls `pack_arenas` with the unchanged Draft's arena
objects and the observed `CapacityLedger`, then passes those layouts to
`FinalizedBundle::finalize_with_loop_schedule`.

The root facade exposes the crate for advanced callers as
`recipe::engine::scheduler`; the package itself is `recipe-scheduler`.
`native-probe/build.rs` also includes `scheduler/src` and `scheduler/Cargo.toml`
in its source digest. That build-time inclusion is an audit/rebuild input, not
an additional scheduling caller.

## Manifest and module graph

[`scheduler/Cargo.toml`](../Cargo.toml) declares package `recipe-scheduler`,
version `0.1.0`, Rust edition 2024, MIT licensing, and one path dependency,
`recipe-core`. There are no driver, compiler, allocator, filesystem, network,
async-runtime, or logging dependencies. The crate root forbids unsafe code and
denies missing `Debug` implementations.

The facade in [`src/lib.rs`](../src/lib.rs) declares four private modules and
re-exports their public contracts:

```text
recipe-scheduler
├── arena.rs          pack_arenas and deterministic arena placement
├── error.rs          ScheduleErrorKind and ScheduleError
├── route.rs          Route and shortest_route
└── static_schedule.rs
                     UnscheduledTask, StaticSchedule, schedule,
                     dependency/resource reservation implementation
```

The implementation direction is one way toward `recipe-core`:

| Module | Core contracts consumed | Responsibility |
| --- | --- | --- |
| `arena.rs` | `ArenaObject`, `ArenaAllocation`, `ArenaLayout`, `ByteCount`, `ByteOffset`, `CapacityLedger`, `Topology` | Place lifetime-bounded objects at aligned offsets and reject a layout that exceeds the per-device Recipe-usable capacity. |
| `error.rs` | `DeviceId`, `TaskId` | Carry one machine-readable scheduler error kind, optional task/device context, and a human-readable message. |
| `route.rs` | `ByteCount`, `DeviceId`, `LinkId`, `Nanoseconds`, `Topology`, `transfer_time_ceil` | Find a minimum measured-time directed path with stable link-ID tie-breaking. |
| `static_schedule.rs` | task kinds, phases, submissions, links, discovery capabilities, windows, checked timing functions | Prepare durations and resource sets, build dependency barriers, list-schedule ready tasks, and persist selected transfer lanes. |

All helper types used by `static_schedule.rs` (`Resource`,
`PreparedTask`, `ResourceGroup`, `Successors`, and `Indegrees`) are private.
Callers cannot bypass preparation, dependency validation, lane selection, or
the deterministic ordering by constructing those internal representations.

## Public API

The re-exported API is intentionally small:

| Item | Contract |
| --- | --- |
| `UnscheduledTask` | A lowered task with `id`, `phase`, explicit `dependencies`, and a `TaskKind`. Its calculation placement, values, and submission slots are already selected; an internal route is explicit or intentionally empty for direct-route selection. Its schedule window and transfer lane claims are not. |
| `StaticSchedule` | The owned `Vec<Task>` with windows and finalized transfer lane claims, plus the schedule `makespan`. Tasks are returned in stable `TaskId` order, not in reservation order. |
| `schedule` | Validates topology/discovery, prepares all tasks, constructs lifecycle barriers, performs deterministic critical-path list scheduling, and returns a `StaticSchedule` or `ScheduleError`. |
| `Route` | A candidate `Vec<LinkId>` and its conservative store-and-forward `Nanoseconds` duration. A multi-link Route is planner evidence, not one executor-visible transfer task. |
| `shortest_route` | Validates the topology and production scheduling properties, then returns the minimum measured-time directed route for a byte count. |
| `pack_arenas` | Packs all supplied `ArenaObject` lifetimes into one `ArenaLayout` for every topology device, subject to alignment, checked arithmetic, and the `recipe_usable` capacity entry. |
| `ScheduleErrorKind` | The non-exhaustive, machine-readable failure vocabulary. |
| `ScheduleError` | A cloneable, debug-printable, equality-comparable error with public `kind`, optional `task`, optional `device`, and `message` fields. It implements `Display` and `std::error::Error`. |

The public structs derive `Clone`, `Debug`, `PartialEq`, and `Eq` where their
source declarations permit it. Their fields are public because they are
`recipe-core` data contracts. Rust ownership makes each result independent of
the scheduler's local maps; the downstream Draft and Finalize validators are
the authority that turns the result into an immutable execution contract.

### `UnscheduledTask` and `StaticSchedule`

`UnscheduledTask` is deliberately not a second task ontology. Its `kind` is
one of the three `recipe-core::TaskKind` variants:

* `Calculation` names a GPU device, kernel/artifact identity, value inputs and
  outputs, optional four-byte fault flag, FLOP work, and submission slots.
* `Transfer` names external or device endpoints, a byte count, zero or one
  executor-visible directed link, empty pre-schedule lane claims, and
  submission slots.
* `Metric` names a four-byte device readback, metric purpose and slot, and
  submission slots. The scheduler treats it as a one-nanosecond submission
  operation; core Draft validation supplies the metric's phase and value
  invariants later.

`StaticSchedule.tasks` contains `recipe-core::Task` values. Each task keeps its
ID, original phase and dependencies, receives a nonempty half-open
`ScheduleWindow`, and carries a cloned task kind. For a transfer, the cloned
kind has the route filled when the scheduler selected a direct route and has a
strictly sorted, unique `lane_claims` list selected by the reservation pass.
`makespan` is the maximum task-window end, or zero when the input task set is
empty.

## Inputs and evidence policy

`schedule` accepts `&Topology`, `&DiscoveryProfile`, and a slice of
`UnscheduledTask`. It immediately runs, in order:

1. `Topology::validate`, which checks identities, device ownership, paired
   reverse links, transport and duplex consistency, and reference integrity.
2. `Topology::validate_scheduling_properties`, which rejects `Estimated`
   capacity, transfer rates, calculation rates, link bandwidths, and link lane
   counts. Only `Measured` or explicit `Override` properties can drive this
   production schedule.
3. `DiscoveryProfile::validate`, which requires the exact topology identity,
   complete available device/link coverage, schedulable capability properties,
   asynchronous submission, nonzero capability limits, and discovery values
   equal to the topology's directional link measurements.

An error from the first two checks becomes `ScheduleErrorKind::InvalidTopology`;
an error from the third becomes `InvalidDiscovery`. The checks are repeated by
`shortest_route` because it is an independently exported boundary. They are
not repeated by `pack_arenas`: that function only checks the topology device
references needed for packing and relies on its caller (`recipe-planner` or
`recipe-prepare`) to have validated topology, discovery, and capacity.

The rate and concurrency values have distinct owners:

| Work | Duration source | Lane/concurrency source |
| --- | --- | --- |
| GPU calculation | `calculation_time_ceil(work, discovered.calculation.rate)` | `maximum_concurrent_tasks` from the device's discovered calculation capability |
| Device-to-device hop | `transfer_time_ceil(bytes, topology link.bandwidth)` | link `maximum_inflight_transfers` from topology; discovery validation requires the same value |
| External admission or egress | `transfer_time_ceil(bytes, discovered.device.transfer.rate)` | discovered device transfer `maximum_inflight_transfers` |
| Same-device internal copy | one nanosecond | no link lane |
| Metric readback | one nanosecond | its submission queue and completion slot only |

Every calculated duration is forced to at least one nanosecond. The core unit
helpers use checked arithmetic; scheduler overflow is an explicit error, never
wrapping. A zero-byte transfer therefore still occupies a nonempty schedule
window, which preserves dependency and resource ordering.

## Route construction

### `Route` and `shortest_route`

`shortest_route(topology, source, destination, bytes)` is a deterministic
positive-weight directed-path search:

1. Validate topology and scheduling properties.
2. Reject an unknown source or destination as `InvalidTransfer`, carrying the
   offending device in the error.
3. Return an empty-link route with duration one for `source == destination`.
4. Build outgoing adjacency lists and sort each list by stable `LinkId`.
5. Run a `BinaryHeap<Reverse<(elapsed, link_path, device)>>` over a map of the
   best `(elapsed, path)` per device. Each edge costs
   `ceil(bytes / link.bandwidth)` with a minimum of one nanosecond.
6. Replace a known destination state only when the `(elapsed, path)` tuple is
   smaller. Therefore equal-duration paths compare lexicographically by their
   link IDs, independently of input vector order.
7. Return the first destination popped from the ordered heap. If no directed
   path exists, return `NoRoute`.

The route duration is a conservative store-and-forward sum of hop durations,
not a pipelined transfer estimate. A route with several links is intentionally
not executable as one `TransferTask`; the planner must insert one
dependency-chained task and one resident intermediate value per hop.

### How `schedule` uses the route helper

During transfer preparation, an internal device-to-device transfer with an
empty route and distinct endpoints calls `shortest_route`. The scheduler accepts
the result only when it has exactly one link and writes that link into the
transfer. A selected multi-link route produces `InvalidTransfer` with the
message that the planner must lower one task per directed link. An explicit
route longer than one link is rejected for the same reason. An empty route is
valid for an explicit same-device copy and receives a one-nanosecond duration.

The route is then checked with `Topology::validate_route`: link endpoints must
chain from source to destination, and only a same-device copy may have no link.
An absent validated link is still reported as `InvalidTransfer` if the lookup
cannot find it. External transfers must have an empty route.

## Static schedule construction

`static_schedule.rs` implements deterministic critical-path list scheduling.
The construction has no wall-clock state and does not poll or submit work.

### 1. Prepare task durations and resources

`prepare_tasks` clones each input task into a private `PreparedTask`, rejects a
duplicate ID, prepares the task kind, sorts its resource keys, removes duplicate
keys, and stores it in a `BTreeMap<TaskId, PreparedTask>`.

#### Calculation preparation

`prepare_calculation` requires the placement device to exist and to be a
`GpuMemory` calculation target. It requires discovered calculation capability,
charges checked FLOP time, and reserves:

* the task's submission queue and completion slot;
* every `ComputeLane(device, lane)` for lane indices below the discovered
  `maximum_concurrent_tasks`; one lane will be selected for the task;
* `NoComputeTransferOverlap(device)` when the discovered transfer capability
  says transfers cannot overlap calculation on that device.

An unknown device or a RAM/disk placement is
`InvalidCalculationPlacement`. Missing calculation capability is
`UnavailableCapability`. Checked timing overflow is `ArithmeticOverflow`.

#### Transfer preparation

Before touching endpoints, an unscheduled transfer must have an empty
`lane_claims` vector. A caller that passes preselected claims receives
`InvalidTransfer`; lane assignment belongs solely to this scheduler pass.

For an internal device-to-device transfer, preparation validates or fills the
single-link route, charges the topology link bandwidth, reserves every
directional `TransferLane(link, lane)` admitted by the link, and adds
`HalfDuplexDirection(capacity_resource, link)` for a half-duplex link. It then
checks transfer capability on both endpoint devices. For each endpoint whose
transfer capability does not overlap calculation, it adds the matching
`NoComputeTransferOverlap(device)` resource. Thus a hop touching either
non-overlapping endpoint cannot overlap that endpoint's calculation work.

For an external-to-device admission or device-to-external egress, preparation
rejects a nonempty route, charges the discovered endpoint transfer rate,
reserves every `ExternalTransferLane(device, lane)`, and adds the endpoint's
`NoComputeTransferOverlap` resource when required. External-to-external is
never schedulable and is `InvalidTransfer`.

The endpoint discovery lookup is required even for a same-device internal copy.
Missing endpoint capability is `UnavailableCapability`. Timing overflow is
`ArithmeticOverflow`. The scheduler does not infer a route, rate, or lane count
from an estimate after validation has rejected it.

#### Metrics

`TaskKind::Metric` receives a one-nanosecond duration and only its submission
queue/completion resources. It does not get a compute, transfer, or
no-overlap resource. Core Draft validation later enforces that metrics are loop
tasks, reference known values and metric slots, and use a four-byte value for a
fault readback.

### 2. Build the dependency graph

`dependency_graph` first creates explicit edges from every listed dependency:

* an absent predecessor is `UnknownDependency`;
* a repeated dependency on one task is also `UnknownDependency`;
* a predecessor in a later `RunPhase` is `InvalidLifecycleDependency`;
* duplicate edges are collapsed in a `BTreeSet` so every predecessor contributes
  exactly one indegree.

It then adds global lifecycle edges for every pair of tasks with an earlier
phase and a later phase. Therefore all Init tasks complete before any Loop task
can become ready, and all Init and Loop tasks complete before any Exit task can
become ready. These explicit edges prevent a lower-ID or higher-criticality
Loop/Exit task from bypassing an unscheduled lower phase. The planner also adds
the same phase dependencies to its source tasks before calling the scheduler;
the scheduler's graph construction is the final authoritative barrier.

The phase ordering is `Init < Loop < Exit`. Dependencies within one phase are
allowed if acyclic. Any reverse-phase explicit dependency fails before the
global barrier edges are considered.

### 3. Compute critical paths and choose ready tasks

`critical_path_lengths` performs a deterministic topological pass over the same
graph. For each task, its length is its own prepared duration plus the maximum
successor length, using checked `u64` addition. A graph that cannot be
topologically ordered returns `DependencyCycle`.

The main pass stores ready tasks in a max-heap keyed by
`(critical_path_length, Reverse<TaskId>)`. The largest remaining critical path
is selected first; when paths tie, the lowest task ID wins. The heap only admits
a task after every explicit and lifecycle predecessor has been scheduled.

### 4. Apply phase floors and reserve the first legal gap

For a ready task, the earliest start is the maximum of its scheduled dependency
ends and its phase floor:

* Init floor: zero;
* Loop floor: the latest end of Init work;
* Exit floor: the latest end of Init or Loop work.

`reserve_earliest` and `first_gap` then search half-open windows beginning at
that time. A `Resource` is either fixed or belongs to a capacity group:

| Resource | Reservation behavior |
| --- | --- |
| `Queue(queue_id)` | Exclusive exact queue slot. |
| `Completion(completion_id)` | Exclusive exact completion slot. |
| `ComputeLane(device, lane)` | One selected lane from the device's compute group. |
| `TransferLane(link, lane)` | One selected lane from the directed-link group. |
| `ExternalTransferLane(device, lane)` | One selected lane from the endpoint's external-transfer group. |
| `HalfDuplexDirection(resource, link)` | Fixed direction marker that conflicts with reservations for the opposite link sharing the same half-duplex capacity resource. |
| `NoComputeTransferOverlap(device)` | Fixed device marker shared by calculation and transfer tasks when overlap is unsupported. |

Queue, completion, half-duplex, and no-overlap resources are checked as fixed
keys. For each compute or transfer group, the pass scans its candidate lanes in
the resource's stable order and selects the first lane whose prior windows do
not overlap the candidate. If every lane is occupied, it advances to the
earliest release among that group's lanes. If a group has no lane at all,
`UnavailableCapability` reports an empty lane group.

The candidate window is checked against every fixed resource before lane
selection. If any fixed conflict exists, the start advances to the latest end
of those conflicts. If one or more groups are full, the start advances to the
earliest release across the full groups and retries. It returns only when every
fixed resource is clear and one lane from every required capacity group has
been selected. Checked `start + duration` overflow is `ArithmeticOverflow`.

Half-duplex contention is exact: a reservation for one
`HalfDuplexDirection(capacity_resource, link)` searches all reservations for a
different link with the same capacity resource. Full-duplex reverse links have
distinct resources and therefore do not conflict through this rule. Same-link
tasks still contend through their directional transfer-lane group.

When a window is accepted, it is inserted into the reservation list for every
selected resource. The task kind is cloned, and `persist_transfer_lane_claims`
converts selected lanes into canonical `TransferLaneClaim` values:

* an internal route claims exactly one `Link { link, lane }` for every route
  link, or no lane for a same-device empty-route copy;
* an external transfer claims exactly one `External { device, lane }` on its
  device;
* non-lane resources never appear in the claim list.

Claims are sorted and deduplicated. If the selected claims do not exactly match
the transfer's endpoint and route shape, the scheduler returns
`UnavailableCapability` rather than emitting a partial contract. Core's Draft
validator independently requires the same strict ordering, lane bounds, and
exact route/device coverage.

### 5. Emit the owned schedule

The accepted task receives its phase, window, dependencies, and prepared kind in
a `recipe-core::Task`. `phase_end` records the latest end per lifecycle phase;
the dependency barriers already enforce cross-phase ordering, while the phase
floor keeps the local reservation calculation explicit. Successor indegrees are
decremented, newly ready tasks enter the critical-path heap, and the loop
continues.

If the number of scheduled tasks differs from the prepared map, the graph had a
cycle and `DependencyCycle` is returned. Otherwise the map is consumed in
ascending `TaskId` order into `StaticSchedule.tasks`. This stable output order
is separate from execution order, which is represented by each task's window
and dependencies. The makespan is the maximum end time and is zero only for an
empty task set.

## Arena construction

### Input contract

`pack_arenas(topology, objects, capacity)` consumes `ArenaObject` values. Each
object names an `ArenaObjectId`, a topology `DeviceId`, a byte size, a byte
alignment, and a nonempty half-open lifetime `ScheduleWindow`. The lifetime is
normally derived by planner from all scheduled tasks that can reference the
object. The scheduler itself does not inspect tasks while packing.

Before placement, each object is checked as follows:

* an unknown device is `InvalidTopology` and carries the device;
* an alignment of zero or a non-power-of-two alignment is `InvalidTransfer`;
* an empty or otherwise invalid lifetime (`start >= end`) is
  `InvalidTransfer`.

`pack_arenas` does not call `Topology::validate`, does not inspect property
provenance, and does not reject duplicate object IDs or duplicate capacity
entries by itself. The planner and core Draft/Finalize validators establish
those wider collection invariants before accepting a layout. The packing
function only performs the exact lookups and arithmetic needed for its own
operation.

### Deterministic first-fit placement

Objects are grouped by device in a `BTreeMap`. Topology device IDs are sorted,
and each device's objects are sorted by `(lifetime.start, object.id)`. For each
object, the candidate offsets begin at zero and include the checked end of each
already placed object whose half-open lifetime overlaps. Candidates are sorted
and deduplicated.

Each candidate is aligned upward with checked `ByteCount::checked_align_up`.
The first aligned offset whose byte interval does not overlap any previously
placed interval with an overlapping lifetime is selected. Memory intervals are
half-open, so two objects can share an offset when their lifetimes do not
overlap, and two live objects conflict exactly when both their lifetimes and
their byte ranges overlap. Every end calculation and alignment operation is
checked. Overflow, including the case where no representable offset remains,
is `ArithmeticOverflow`.

After all objects on a device are placed, allocations are sorted by object ID.
The layout size is the maximum checked allocation end, or zero for a device with
no objects. The function then requires a `CapacityLedger` entry for that
device. A missing entry or a layout whose size exceeds
`entry.recipe_usable.value` is `InsufficientCapacity`. A layout is emitted for
every topology device, including devices with an empty allocation vector, and
the layouts are returned in ascending device-ID order.

The result contains offsets and sizes only. It does not contain object
lifetimes, value bindings, or native allocations. Core Finalize later checks
that every Draft object has exactly one allocation, every device has exactly one
layout, offsets satisfy alignment and bounds, the size is the exact maximum end,
and no live objects overlap in memory.

### Planning versus final packing

`recipe-planner::lower_candidate` calls `pack_arenas` against its planning
capacity after it has scheduled tasks and built arena objects. The resulting
layouts are retained as `PlannedCandidate::arena_layouts` and as evidence for
candidate feasibility. The planner then validates and hashes the complete
Draft.

`recipe-prepare::prepare_program_validated` calls `pack_arenas` a second time
only after a candidate has been realized, warmed, and measured. It supplies the
stabilized observed `CapacityLedger`, not the optimistic planning bound. A
capacity failure rejects that unchanged candidate at the `FinalCapacity` stage;
the preparer destroys the candidate session and asks the one-shot planner search
for the next candidate. Successful layouts are handed to
`FinalizedBundle::finalize_with_loop_schedule`, which resolves each value's
physical arena address without changing the scheduled task graph.

## Planner callers and route lowering

The scheduler has no dependency on planner implementation state, but the
planner is the layer that gives its generic contracts their execution meaning.

### Candidate lowering

`lower_candidate` in
[`planner/src/planner.rs`](../../planner/src/planner.rs#L979-L1213) performs the
following sequence before and after the scheduler call:

1. Allocate stable task/value IDs and one queue/completion pair per planned
   submission.
2. Build one Init external admission per device data image.
3. Lower each primitive stage into a Loop `CalculationTask`, with dependencies
   for prior stages and produced/consumed values.
4. Add mandatory fault-readback `MetricTask` values for checked calculations
   and optional user metric readbacks.
5. Insert device-to-device transfer chains for resident copies and external
   output trial tasks.
6. Add dependencies required by exact alias contracts, then add global phase
   dependencies before calling `schedule`.
7. Map `ScheduleErrorKind` into planner errors. Dependency cycles and reverse
   lifecycle edges become `DependencyConflict`; arithmetic overflow and
   capacity become `CandidateInfeasible`; `NoRoute` stays `NoRoute`; all other
   scheduler errors become `Schedule`.
8. Compact submission resources, account auxiliary resources, build arena
   lifetimes, call `pack_arenas`, and require total capacity.
9. Sort the resulting Draft collections, construct loop-domain sidecar data,
   hash the exact scheduled Draft, and run `DraftPlan::validate` before the
   candidate is ranked.

### Transfer route selection and hop chains

Planner `ensure_copy` does not blindly ask the scheduler for the shortest route.
It enumerates every simple directed route with `directed_routes`, constructs a
prospective chain with `build_transfer_chain`, and calls `trial_chain_timing`.
That helper appends the trial tasks, adds phase barriers, and invokes
`schedule`; an infeasible timing is discarded while dependency conflicts remain
planner errors. The planner ranks a schedulable route by trial end, trial
makespan, source device, source value, and link path, then materializes the
winning chain with stable IDs.

`build_transfer_chain` validates the complete route and creates one
`UnscheduledTask` per directed link. Each hop has one source and destination
resident value, depends on the previous hop (or the original producer), and
shares the caller's loop domain. Intermediate values are real `ValueSpec`
objects on intermediate devices. This is why the scheduler's executor-visible
route limit of one link is compatible with arbitrarily long physical paths.

External output selection follows the same trial boundary. For each safe
resident copy, planner creates a prospective Exit device-to-external transfer,
asks `schedule` for its end/makespan, and commits the best candidate with a
stable submission pair. The final scheduled task has an empty route and, after
the scheduler pass, one external transfer lane claim.

### Trial schedules are real scheduler calls

`trial_timing` and `trial_chain_timing` invoke the same public `schedule` entry
point, with the same topology and discovery, rather than reproducing timing
formulas in planner. Trial errors are classified narrowly: arithmetic overflow,
capacity, or no route mean that candidate is not schedulable; dependency-cycle
and reverse-phase errors are structural planner conflicts; any other scheduler
error is surfaced as a planner schedule failure.

## Prepare and downstream consumers

The direct calls are visible in these source locations:

| Caller | Scheduler entry | What crosses the boundary |
| --- | --- | --- |
| `planner::lower_candidate` | `schedule`, then `pack_arenas` | Fully lowered Draft tasks, measured topology/discovery, arena lifetimes, and planning capacity. |
| `planner::trial_chain_timing` | `schedule` | Existing task set plus one route or output trial, used only to derive an end and makespan before committing IDs. |
| `prepare::Preparer::prepare_program_validated` | `pack_arenas` | Unchanged candidate arena objects plus stabilized post-realization capacity. |
| `recipe::engine::scheduler` | re-export | Advanced callers can name the same contracts, but no facade wrapper changes their behavior. |
| `native-probe/build.rs` | source/path digest only | Scheduler Rust and manifest files are hashed for rebuild and audit invalidation. |

No executor, CUDA, HSA, host, transport, or remote module calls
`recipe_scheduler::schedule` or `shortest_route` at runtime. They consume the
finalized task windows, lane claims, arena layouts, and resolved values that
the planner, scheduler, prepare, and core Finalize stages have already fixed.
The scheduler never sees a driver handle or a backend queue object.

## Error contract

`ScheduleErrorKind` is `#[non_exhaustive]`; callers must match a known kind
without assuming the enum will never grow. `ScheduleError::new` starts with no
context. `for_task` and `for_device` attach one stable ID and return the error
for chaining. `Display` renders the debug form of the kind, optional task and
device context, and the message:

```text
Kind for task <id> on device <id>: message
```

The actual kinds and their construction sites are:

| Kind | Current causes |
| --- | --- |
| `InvalidTopology` | Topology validation failure in `schedule` or `shortest_route`; an arena object names an unknown device. |
| `InvalidDiscovery` | Discovery validation failure at either scheduling entry. |
| `DuplicateTask` | Two `UnscheduledTask` inputs use one `TaskId`. |
| `UnknownDependency` | A dependency is absent or repeated on the same task. |
| `DependencyCycle` | Critical-path graph or final ready pass cannot schedule every task. |
| `InvalidLifecycleDependency` | An explicit dependency points from a later phase to an earlier phase. |
| `UnavailableCapability` | Missing calculation/transfer endpoint discovery, an empty compute/transfer lane group, or a persisted lane set that does not cover the transfer. |
| `InvalidCalculationPlacement` | Calculation names an unknown device or a non-GPU storage device. |
| `InvalidTransfer` | Preselected lane claims, malformed route, multi-hop executor-visible transfer, route on external transfer, external-to-external transfer, invalid arena alignment/lifetime, or related transfer shape failure. |
| `NoRoute` | `shortest_route` cannot reach the destination, or a planner-selected copy has no directed route. |
| `ArithmeticOverflow` | Checked transfer/calculation timing, route elapsed time, critical-path sum, schedule-window end, arena alignment/end, or representable arena offset overflows. |
| `InsufficientCapacity` | A required device has no capacity entry, or its planned arena exceeds `recipe_usable`. |

`ScheduleError` does not aggregate multiple failures. The first failing stage
returns one error with the narrowest context available. Core Draft validation
later aggregates cross-object contract failures, including queue/completion
ownership, task phase, transfer endpoint shape, lane bounds, concurrency, and
live allocation overlap.

## Invariants preserved across the boundary

The scheduler's implementation and the downstream core validators jointly
preserve these facts:

* all production timing inputs are measured or explicitly overridden;
* each returned task has one nonempty half-open window and an explicit phase;
* dependencies complete before successors, and lifecycle phases form global
  `Init -> Loop -> Exit` barriers;
* a calculation uses one selected compute lane from measured concurrency;
* each external or distinct-device transfer uses one selected lane from measured
  link/device concurrency, and half-duplex opposite directions never overlap;
* transfers and calculations overlap only when endpoint discovery says that
  overlap is supported;
* an internal executor-visible transfer contains zero links only for a
  same-device copy, or exactly one directed link for a distinct-device hop;
* multi-hop physical paths are represented by dependency-chained tasks and
  resident intermediate values before scheduling;
* scheduled transfer lane claims are complete, sorted, unique, and canonical;
* arena offsets satisfy power-of-two alignment and live objects never overlap
  in bytes;
* every topology device receives a layout and a capacity check, even when its
  object list is empty;
* makespan and layout order are stable functions of IDs and measured values;
* no native resource is allocated and no task is submitted while the scheduler
  is running.

The scheduler deliberately does not own facts that are checked elsewhere. It
does not validate a task's queue/completion slot belongs to its device, verify
that a metric is loop-only, verify value bindings, enforce unique arena IDs, or
validate capacity provenance. Those are `DraftPlan` and `FinalizedBundle`
contract checks. Keeping those responsibilities in `recipe-core` prevents a
second, divergent validation ontology in the scheduler.

## Source map

The implementation and the adjacent contracts can be read directly here:

* [`src/lib.rs`](../src/lib.rs): crate attributes, module declarations, and
  public re-exports.
* [`src/error.rs`](../src/error.rs): error kinds, context builders, and
  formatting.
* [`src/route.rs`](../src/route.rs): `Route` and measured shortest-path search.
* [`src/static_schedule.rs`](../src/static_schedule.rs): task preparation,
  graph/barrier construction, critical paths, resource reservation, and lane
  persistence.
* [`src/arena.rs`](../src/arena.rs): per-device lifetime-aware first-fit arena
  packing.
* [`../../core/src/schedule.rs`](../../core/src/schedule.rs): the task,
  phase, transfer, window, and arena data contracts consumed and returned.
* [`../../core/src/topology.rs`](../../core/src/topology.rs): measured property
  provenance, topology validation, route validation, and duplex resources.
* [`../../core/src/discovery.rs`](../../core/src/discovery.rs): required
  capability and identity validation.
* [`../../core/src/plan.rs`](../../core/src/plan.rs): Draft validation,
  contention checks, arena-layout validation, and Finalize.
* [`../../planner/src/planner.rs`](../../planner/src/planner.rs): the production
  lowering caller, trial scheduling, route-hop decomposition, and Draft
  construction.
* [`../../prepare/src/lib.rs`](../../prepare/src/lib.rs): post-realization
  repacking and finalization handoff.

The normative lifecycle remains `init -> loop -> exit`. This crate computes the
static contract that orders that lifecycle; it never executes it.
