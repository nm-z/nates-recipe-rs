# Scheduler errors

## Boundary and representation

`recipe-scheduler` is the deterministic, measured-property boundary that turns
an already lowered task list into either a complete static schedule or an
explicit failure. The crate exports `ScheduleError`, `ScheduleErrorKind`,
`shortest_route`, `pack_arenas`, and `schedule` from `scheduler/src/lib.rs`.
The three public operations have no shared mutable state and return
`Result<..., ScheduleError>`:

* `shortest_route` selects a measured, directed-link path and computes its
  conservative store-and-forward duration.
* `pack_arenas` assigns deterministic physical offsets to bounded objects on
  each device and checks the realized Recipe-usable capacity.
* `schedule` validates topology and discovery, prepares calculation, transfer,
  and metric tasks, builds dependency and lifecycle edges, reserves resources,
  and returns `StaticSchedule { tasks, makespan }`.

The scheduler does not retry, substitute a property, or return a partial
result. A caller receives either the complete value or the first error reached
by the deterministic traversal. Route search and arena packing work on local
maps and vectors. Static scheduling clones each `UnscheduledTask` into its
private preparation map, so route and lane-claim assignment cannot mutate the
caller's task slice. Reservations, phase ends, and scheduled tasks are also
local; if any later check fails, no schedule escapes.

`ScheduleErrorKind` is `#[non_exhaustive]`, so downstream matches must retain a
wildcard arm. Its complete current set is:

```text
InvalidTopology, InvalidDiscovery, DuplicateTask, UnknownDependency,
DependencyCycle, InvalidLifecycleDependency, UnavailableCapability,
InvalidCalculationPlacement, InvalidTransfer, NoRoute, ArithmeticOverflow,
InsufficientCapacity
```

`ScheduleError` (`scheduler/src/error.rs:22-28`) carries the machine-readable
`kind`, optional `task: TaskId`, optional `device: DeviceId`, and an owned
`message`. `new` clears both context fields. `for_task` and `for_device` add
context and return the same error (`error.rs:30-52`), so callers must use their
`#[must_use]` return values. `Display` prints the Debug spelling of the kind,
then any task and device, then the message (`error.rs:54-64`), for example
`InvalidTransfer for task 7: ...`. The type implements `std::error::Error`
without a `source` (`error.rs:67`): topology validation errors and unit
arithmetic errors are deliberately flattened with `to_string()` at the
construction site. There is no `From` conversion in this crate, and no typed
cause survives a scheduler boundary.

## Complete construction ledger

The following inventory covers every current `ScheduleErrorKind` construction.
The line ranges are source anchors, not an inferred list of possible errors.

| Kind | Construction sites and exact meaning | Context and immediate state consequence |
| --- | --- | --- |
| `InvalidTopology` | `route.rs:31-36` and `static_schedule.rs:66-71` wrap `Topology::validate` or `Topology::validate_scheduling_properties`; `arena.rs:20-25` rejects an arena object whose device is absent. | The route and schedule validators fail before search or task preparation. Arena packing stops before grouping the bad object and attaches its device. No topology, route, or layout is returned. |
| `InvalidDiscovery` | `static_schedule.rs:72-74` wraps `DiscoveryProfile::validate(topology)`. | Scheduling stops before preparing any task. Discovery identity, required devices, measured rates, queues, and asynchronous capabilities are therefore a precondition, not a fallback. |
| `DuplicateTask` | `static_schedule.rs:169-175`, when two input tasks have the same `TaskId`. | Preparation stops at the duplicate and identifies that task. No dependency graph or resource reservation is built. |
| `UnknownDependency` | `static_schedule.rs:463-468` for a dependency ID absent from the prepared map; `:470-475` for a repeated dependency in one task. | The task graph is rejected before lifecycle edges and topological ordering. A repeated dependency is reported under this kind even though it names an existing task, because the scheduler's graph input requires a unique predecessor list. |
| `DependencyCycle` | Main list scheduling detects `scheduled.len() != prepared.len()` at `static_schedule.rs:145-149`; critical-path topological ordering detects the same at `:535-539`. | The cycle is a graph failure, not an attempt to break the cycle. The local ready heap and reservations are discarded. The two checks protect both the critical-path pass and the final scheduling pass. |
| `InvalidLifecycleDependency` | `static_schedule.rs:478-483` rejects a task that depends on a predecessor in a later `RunPhase`. | No cross-phase dependency is accepted. Global Init -> Loop -> Exit barrier edges are added only after this check, so a later phase cannot be used to satisfy an earlier one. |
| `UnavailableCapability` | Missing calculation capability at `static_schedule.rs:218-225`; missing discovery for an internal endpoint at `:320-327`; missing external endpoint capability at `:343-350`; an incomplete selected transfer lane set at `:440-445`; an empty compute or transfer lane group at `:650-655`. | Calculation and transfer failures include task and, where an endpoint is known, device. Lane-set and empty-group failures prove that the measured capability did not produce the resources required by the task. Reservations remain local and are discarded on error. |
| `InvalidCalculationPlacement` | `static_schedule.rs:199-205` for an unknown device and `:207-213` for a non-GPU storage device. | The task is rejected before reading calculation capability or allocating compute resources. Both task and device identify the invalid placement. |
| `InvalidTransfer` | Unknown route endpoints from `shortest_route` are made at `route.rs:39-50` with device context. Static preparation rejects preselected lane claims (`static_schedule.rs:251-256`), an automatically found multi-hop route (`:269-275`), any executor-visible route longer than one link (`:278-283`), failed `Topology::validate_route` (`:285-289`), an absent validated link (`:293-299`), an external route (`:336-341`), and external-to-external transfer (`:367-372`). Arena input uses this kind for zero or non-power-of-two alignment and an empty lifetime (`arena.rs:27-39`). | These are malformed transfer or physical-placement contracts, not unavailable resources. Task context is attached by static scheduling; route endpoint and arena failures attach the known device. The offending transfer or object is never admitted to the graph or layout. |
| `NoRoute` | `route.rs:106-109` after the directed shortest-path heap is exhausted. | There is no directed path from source to destination. The error has no task or device context because the public route function has only the two endpoint values in its message. When called by static transfer preparation it propagates unchanged. |
| `ArithmeticOverflow` | Route hop duration conversion and elapsed accumulation (`route.rs:82-91`); calculation duration (`static_schedule.rs:226-229`); internal and external transfer duration (`:300-304`, `:358-362`); critical-path addition (`:554-559`); schedule-window end (`:612-617`); arena checked ends, alignment, and final offsets (`arena.rs:60-65`, `:76-85`, `:103-108`, `:117-123`). | Task context is attached where the operation is task-specific; arena arithmetic attaches the device. Route search, critical-path reduction, and `first_gap` have no task available and therefore carry only kind and message. Checked arithmetic leaves no wrapped duration, offset, or window. |
| `InsufficientCapacity` | `arena.rs:129-135` when a device has no realized capacity-ledger entry, and `:136-144` when the packed arena exceeds `recipe_usable`. | Packing stops for the device. No layout for that call is returned. `schedule` itself does not construct this kind; the enum is shared by route/schedule/arena APIs, while capacity is enforced by `pack_arenas`. |

The table is exhaustive for `ScheduleErrorKind` constructors in the scheduler
crate (`rg 'ScheduleErrorKind::' scheduler/src`). No constructor silently
changes one kind into another. The only flattening is conversion of a lower
level validator or unit error into the message string.

## Route errors and propagation

`shortest_route` first calls both topology validators (`route.rs:30-36`).
`Topology::validate_scheduling_properties` is important: theoretical seed
properties cannot enter a schedule. It then checks both endpoint devices. An
unknown source or destination is `InvalidTransfer` with the corresponding
device context, not `InvalidTopology`, because the topology itself may be
valid while the requested transfer is not. A same-device request is a valid
empty route with one nanosecond duration (`route.rs:52-57`).

For distinct devices, adjacency is grouped by source and sorted by stable link
ID (`route.rs:59-65`). The heap key is elapsed duration, lexicographic link
path, then device ID. Each hop uses
`transfer_time_ceil(bytes, measured_bandwidth)`, rounded up and clamped to at
least one nanosecond. Unit conversion overflow is `ArithmeticOverflow` at
`route.rs:82-85`; checked path accumulation is the same kind at `:87-91`.
When the destination is popped, the path and `elapsed.max(1)` are returned.
Exhausting the heap produces `NoRoute` (`:106-109`).

Static transfer preparation calls `shortest_route` only for a distinct-device
transfer with an empty route (`static_schedule.rs:267-276`). A multi-link
result is deliberately rejected as `InvalidTransfer`: an executor-visible
transfer is exactly one directed hop. The planner must split a longer path
into dependency-chained hop tasks with intermediate resident values before it
calls `schedule`. A `NoRoute` or route arithmetic error from the public route
call propagates through `prepare_transfer` with `?`; because the route function
has no task parameter, that propagated error has no task context. A direct
public caller sees the same `ScheduleError` shape.

If a route was supplied, or after an automatic direct route was inserted,
`Topology::validate_route` checks link existence, contiguous endpoints, and the
destination (`static_schedule.rs:285-289`). A route longer than one link,
absent link, or external route is rejected before a lane is selected. Internal
link duration overflow is task-scoped (`:300-304`). The link contributes one
resource for every measured inflight lane and, for half duplex, a directional
capacity resource (`:307-315`). Both endpoints also contribute a
`NoComputeTransferOverlap` resource when discovered transfer/calculation
overlap is false (`:319-330`).

The route object is local to search. Neither `shortest_route` nor transfer
preparation mutates `Topology`; the only transfer mutation is on the cloned
task that becomes part of a successful `StaticSchedule`. If route selection
fails, no candidate task, intermediate value, or reservation is published.

## Arena errors and propagation

`pack_arenas` takes a topology, draft `ArenaObject` slice, and a realized
`CapacityLedger`. It does not replace the caller's topology or objects and it
does not validate the complete topology itself. For every object it checks the
device exists (`arena.rs:20-25`), alignment is a nonzero power of two
(`:27-32`), and the half-open lifetime is nonempty (`:34-39`). Invalid
alignment and lifetime use `InvalidTransfer` because they violate the
physical-placement contract shared with transfer inputs; they are not
capacity shortages.

Objects are grouped by device and processed in sorted device order. Within a
device they are sorted by lifetime start and stable object ID. The allocator
tries zero and ends of overlapping objects, aligns each candidate upward, and
accepts the lowest non-colliding offset (`arena.rs:44-110`). Half-open
non-overlapping lifetimes can therefore reuse bytes. Checked offset ends and
alignment failures are `ArithmeticOverflow` with device context. The explicit
`no representable arena offset remains` branch is also overflow, not a hidden
allocation fallback. Final allocations are sorted by object ID and the arena
size is the maximum checked end (`:113-128`).

The capacity ledger must contain the device and its `recipe_usable` value must
cover the packed size. Missing entry and an oversized arena are
`InsufficientCapacity` with device context (`:129-144`). Because the function
returns immediately on every error, even layouts already built for earlier
devices are not observable by the caller.

There are two materially different arena call paths:

1. `planner::lower_candidate` packs against optimistic planning capacity at
   `planner/src/planner.rs:1091-1101`. Every arena `ScheduleError`, including
   malformed alignment or overflow, is converted to
   `PlannerErrorKind::InvalidCapacity` with `error.to_string()`. The planner
   has no native session at this point, so there is no teardown side effect.
2. `prepare::Preparer::prepare_program_validated` packs the realized candidate
   at `prepare/src/lib.rs:439-451`. Any error destroys the just-realized
   session, records a `CandidateRejection` at `FinalCapacity`, and continues
   to the next ranked candidate. A destroy failure changes the result to
   `PrepareErrorKind::Teardown` (`prepare/src/lib.rs:513-524`). If every finite
   candidate is rejected, the final result is `CandidateExhaustion` with all
   recorded arena details (`prepare/src/lib.rs:506-510`).

## Static scheduling errors and state transitions

`schedule` validates topology, schedulable measured properties, and discovery
before touching tasks (`static_schedule.rs:61-76`). `prepare_tasks` then clones
each task, rejects duplicate IDs, and computes duration plus a sorted,
deduplicated resource set. Calculation preparation checks GPU placement,
discovered calculation capability, and checked work-to-time conversion
(`static_schedule.rs:193-243`). It reserves the task's queue and completion
slots, one resource for every measured compute lane, and the discovered
compute/transfer overlap constraint. A missing calculation capability is
`UnavailableCapability`; an invalid placement is
`InvalidCalculationPlacement`; conversion overflow is task-scoped
`ArithmeticOverflow`.

Transfer preparation enforces the unscheduled representation: incoming
`lane_claims` must be empty (`:251-256`). It distinguishes three endpoint
forms:

* Device to device: an empty route is either same-device or automatically
  filled with a direct route. A multi-hop route must already have been lowered
  into one task per hop. Route validation, link lookup, measured duration, and
  endpoint discovery produce the errors described above.
* External to device or device to external: an internal route is invalid, the
  device's discovered transfer capability is required, and every measured
  external transfer lane is added as a resource. Duration overflow is
  task-scoped `ArithmeticOverflow` (`:334-365`).
* External to external: no device capability can define its cost or lane, so
  it is always `InvalidTransfer` (`:367-372`).

After reservation, `persist_transfer_lane_claims` extracts selected link or
external lanes and checks that the canonical claim set exactly matches the
transfer endpoints and route (`static_schedule.rs:378-448`). A wrong, mixed,
or incomplete set is `UnavailableCapability`, even though the transfer had a
window candidate. Only a successful schedule stores the canonical claims in
the returned `TaskKind::Transfer`.

`dependency_graph` rejects missing or repeated predecessors and later-phase
predecessors, then adds explicit global edges from every lower phase to every
higher phase (`:458-509`). This keeps an unscheduled Loop task from becoming
ready while any Init task remains. `critical_path_lengths` performs a second
topological pass, rejects a cycle, and checks each duration plus successor
length for overflow (`:512-563`).

The final list scheduler uses the critical-path length and stable task ID as a
deterministic ready-heap key. It computes the dependency end and phase floor,
then asks `reserve_earliest` for the first legal half-open window
(`static_schedule.rs:80-125`). Queue and completion resources are exclusive;
compute, link, and external-transfer resource groups select one free measured
lane; half-duplex opposing directions contend through a shared capacity
resource. `first_gap` checks `start + duration` and returns
`ArithmeticOverflow` on a nonrepresentable window (`:602-617`). If a group has
no lanes, it returns `UnavailableCapability` (`:629-655`). A failed reservation
does not leak its tentative candidate because the reservation is inserted only
after `reserve_earliest` succeeds.

When all ready tasks are scheduled, the scheduler persists selected transfer
claims, updates local phase ends and task ends, and inserts a complete `Task`
with its dependencies and window. If the ready heap drains before all tasks,
the final `DependencyCycle` check at `:145-149` fires. On success, tasks are
returned in `TaskId` order and `makespan` is the maximum end, or zero for an
empty input. On any error, no `StaticSchedule` or partially reserved state is
returned.

## Planner conversion and candidate state

The planner owns candidate enumeration and is the only in-repository caller of
`schedule` and `shortest_route` indirectly. `lower_candidate` calls
`schedule` at `planner/src/planner.rs:1059-1071`. It inspects the structured
kind before formatting the error:

| Scheduler kind | Planner kind in `lower_candidate` | Candidate effect |
| --- | --- | --- |
| `DependencyCycle`, `InvalidLifecycleDependency` | `DependencyConflict` | The lowered assignment is rejected as a dependency conflict. |
| `ArithmeticOverflow`, `InsufficientCapacity` | `CandidateInfeasible` | The assignment cannot produce a bounded candidate. |
| `NoRoute` | `NoRoute` | The assignment has no usable directed path. |
| Every other current kind | `Schedule` | The error is treated as a schedule failure, not silently repaired. |

The conversion creates a new `PlannerError` from `error.to_string()` and
therefore loses the separate task and device fields, although their text
remains in the formatted message. There is no `source` link from
`PlannerError` back to `ScheduleError`.

The candidate lowerer then calls `pack_arenas` (`planner/src/planner.rs:1091-1101`)
and maps every arena failure to `PlannerErrorKind::InvalidCapacity`. It builds
the `DraftPlan` only after scheduling, arena packing, total-capacity checks,
and Draft validation succeed (`:1171-1194`). Thus a scheduler error discards
the whole candidate-local `LoweringState`; no task IDs, aliases, values, or
arena objects from that failed assignment are reused.

`plan_program_candidates` classifies only
`NoRoute`, `DependencyConflict`, `CandidateInfeasible`, and `InvalidCapacity`
as candidate-local failures (`planner/src/planner.rs:293-323`). It records their
formatted details and continues enumerating placement assignments. A
`Schedule` error, including `InvalidTopology`, `InvalidDiscovery`,
`InvalidTransfer`, `InvalidCalculationPlacement`, `UnavailableCapability`,
`DuplicateTask`, or `UnknownDependency`, aborts enumeration because those
conditions contradict the already validated planning input. If every finite
assignment fails locally, the public planner result is
`PlannerErrorKind::NoViableCandidate` with the first scheduler-derived detail
(`planner/src/planner.rs:325-336`).

Route selection in `ensure_copy` uses `trial_chain_timing`. That helper clones
the current task list, adds a trial hop chain, and calls `schedule` at
`planner/src/planner.rs:2006-2045`. `ArithmeticOverflow`,
`InsufficientCapacity`, and `NoRoute` mean this particular trial is not
schedulable and return `Ok(None)`, allowing the caller to examine another
directed route. `DependencyCycle` and `InvalidLifecycleDependency` become
`PlannerErrorKind::DependencyConflict`; every other scheduler kind becomes
`PlannerErrorKind::Schedule`. `ensure_copy` distinguishes no directed route
from routes that all fail scheduling: `route_count == 0` is `NoRoute`, while
nonzero routes with no schedulable trial are `CandidateInfeasible`
(`planner/src/planner.rs:2519-2567`). This is the planner's explicit
candidate search policy, not a scheduler fallback.

## Preparation and public end-to-end behavior

`Preparer::prepare_program` validates the measured profile and reservations,
resolves artifacts, and maps any planner result to
`PrepareErrorKind::Planning` (`prepare/src/lib.rs:329-377`). Once a candidate
has been realized and stabilized, an arena `ScheduleError` is retryable only
at the candidate boundary: the native session is destroyed, the error text is
recorded as `CandidateRejectionStage::FinalCapacity`, and the next candidate
is attempted (`prepare/src/lib.rs:439-451`). Finalization errors are separate
`PrepareErrorKind::Finalization` failures after the schedule and layouts have
already passed their own checks (`prepare/src/lib.rs:476-492`).

The native execution crate converts `PrepareError` to
`TrainingExecutionError::Preparation` and
`InferenceExecutionError::Preparation` (`training/src/execute.rs:861-862`
and `:1180-1181`). Therefore a scheduler failure that reaches execution has
already gone through planner classification, candidate rejection, or both.

The public inference path preserves this typed nesting. Each dense, KNN,
Bayes, or GGUF local path calls `preparer.prepare_program` before native
handoff (`training/src/execute.rs:1227-1238`, `:1346-1356`; training uses
`:2204-2214`). `src/inference.rs:602-659` returns the execution error through
`InferenceError::Execute`, whose `source` chain includes the preparation
error (`src/inference.rs:37-46`, `:78-89`, `:108-114`). The CLI therefore
reports the `PrepareError` kind, candidate rejection details, and the embedded
`ScheduleError` display text without pretending that native execution began.

Training follows the same preparation boundary but intentionally flattens its
outer public result: `src/training.rs:1326-1337` maps the execution error to
`TrainingError::runtime("execute native training", error.to_string())`. The
formatted scheduler kind and message remain visible, but the public
`TrainingError::Runtime` has no typed `PrepareError` source. In both paths, a
scheduler failure occurs before `PreparedRun::prepare_recoverable`, so no
`init -> loop -> exit` executor lifecycle, device allocation, or user output is
started for that failed candidate.

## Invariants to preserve when changing this surface

* **Measured inputs only.** `schedule` and `shortest_route` reject theoretical
  topology properties; discovery validation requires available devices,
  asynchronous submission, measured rates, and nonzero queue/lane
  capabilities. Do not add a code-side estimate or fallback.
* **Calculation/transfer ontology.** Scheduler errors describe validation,
  placement, dependency, capability, arithmetic, route, and capacity failures
  for the existing `TaskKind::Calculation`, `Transfer`, and `Metric` model.
  Queue, completion, lanes, duplex resources, and lifecycle phases are
  scheduling machinery, not new model task kinds.
* **One-hop executor transfers.** A route may be multi-link as a planner
  candidate, but each executor-visible `TransferTask` has at most one link.
  Longer paths are dependency-chained tasks with intermediate values before
  `schedule` is called.
* **Canonical claims and windows.** Unscheduled transfers have no lane claims;
  successful scheduling writes the complete selected claim set. Windows and
  arena lifetimes are half-open, and all conflict checks use that same rule.
* **Lifecycle ordering.** Explicit dependencies cannot point to a later phase,
  and global phase barriers force every Init task before Loop and every Loop
  task before Exit. A cycle is reported, never broken by deleting an edge.
* **Determinism.** Device, object, route, task, resource, lane, and candidate
  ordering all use stable IDs or measured durations. Error construction follows
  that same order, so changing traversal order changes observable diagnostics
  and must be deliberate.
* **No partial authority.** Route maps, arena placements, reservations, and
  prepared tasks are provisional until the enclosing operation returns `Ok`.
  Callers may retry a planner candidate or destroy a realized candidate, but
  the scheduler itself never publishes a partial schedule, layout, or state.
* **Context is optional, not a substitute for kind.** `task` and `device` are
  diagnostic coordinates. Consumers must branch on `kind`, preserve the
  formatted message, and tolerate a future `ScheduleErrorKind` because the
  enum is non-exhaustive.
