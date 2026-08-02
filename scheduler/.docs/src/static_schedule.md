<!--
This document describes the deterministic static scheduler in
scheduler/src/static_schedule.rs.  The implementation is the source of truth.
The planner, core plan validator, and executor references below explain how
the returned tasks are consumed through the real execution path.
-->

# Static scheduling

## Purpose and boundary

`recipe_scheduler::schedule` turns a fully lowered task list into one
deterministic static schedule.  It chooses a half-open `ScheduleWindow` for
each task, chooses one physical lane from every measured compute or transfer
lane group, and computes the candidate `makespan`.  For transfers it also
persists the selected lanes as canonical `TransferLaneClaim` values in the
returned task.

The scheduler is deliberately below graph lowering and above execution.  It
does not choose a kernel placement, allocate a value, create an intermediate
value for a route, probe hardware, compact the final queue manifest, or execute
work.  The planner must supply those logical choices as `UnscheduledTask`
values.  In particular, an executor-visible device-to-device transfer is one
physical hop.  A multi-link path is a planner-owned dependency chain of one-hop
tasks with resident intermediate values.  The scheduler's automatic route
fallback is allowed to fill only a direct route.

The implementation is [`scheduler/src/static_schedule.rs`](../../src/static_schedule.rs).
The public entry point and its result types are re-exported by
[`scheduler/src/lib.rs`](../../src/lib.rs).

## Public representations

### Input: `UnscheduledTask`

```text
UnscheduledTask {
    id: TaskId,
    phase: RunPhase,                 // Init, Loop, or Exit
    dependencies: Vec<TaskId>,
    kind: TaskKind,
}
```

`UnscheduledTask` fixes task identity, lifecycle phase, logical dependencies,
and operation payload.  It has no schedule window.  A transfer must also have
an empty `lane_claims` list: lane selection is owned by this scheduler, not by
the caller.  `schedule` clones every input task before it mutates an omitted
direct route or writes lane claims, so the caller's unscheduled list is not
modified.

`TaskKind` is the concrete calculation/transfer reduction used by the rest of
Recipe:

| Kind | Important input fields | Scheduler treatment |
| --- | --- | --- |
| `Calculation(CalculationTask)` | GPU `device`, kernel and artifact identities, resident input/output values, optional int32 `fault_flag`, exact FLOP `work`, `SubmissionSlots` | Duration is the measured FLOP rate rounded up to nanoseconds. The task claims all measured compute lanes on its device as one selectable group, plus queue/completion and, when required, a device transfer/compute exclusion resource. |
| `Transfer(TransferTask)` | `source` and `destination` endpoints, exact `bytes`, zero or one-link `route` for an executor-visible internal operation, empty `lane_claims`, `SubmissionSlots` | Duration is the measured link or external-device transfer rate rounded up to nanoseconds. The task claims one link lane or one external lane, the optional half-duplex direction resource, endpoint overlap resources, and queue/completion. The selected lane claims are written into the returned task. |
| `Metric(MetricTask)` | `MetricPurpose`, metric and value IDs, preallocated metric slot, `SubmissionSlots` | A specialized four-byte device readback with a fixed one-nanosecond scheduling duration. It claims only queue and completion resources. Its value/device and phase contracts are validated later by the core plan validator. |

The transfer endpoint forms are `External` and
`Device { device: DeviceId, value: ValueId }`.  An empty internal route is a
same-device copy when both device endpoints are equal.  An external admission
or egress also has an empty route because the host is not a topology link.

### Output: `StaticSchedule`

```text
StaticSchedule {
    tasks: Vec<Task>,          // each input task plus window and claims
    makespan: Nanoseconds,     // maximum task.window.end, or zero for no tasks
}
```

The returned `Task` adds a nonempty half-open window and carries the original
task dependencies and phase.  Lifecycle barrier edges that the scheduler adds
internally are not appended to `Task.dependencies`; they only control the
topological scheduling order.  The `BTreeMap` used during construction means
`tasks` are returned in ascending `TaskId` order, not in submission or
critical-path order.  `makespan` is the largest scheduled end time and is
planner timing evidence, not a runtime timeout.

### Internal representation

The scheduler prepares each input as:

```text
PreparedTask {
    task: UnscheduledTask,      // cloned, possibly with a direct route filled
    duration: Nanoseconds,
    resources: Vec<Resource>,
}
```

`Resource` is an internal exact identity or a selectable lane group:

| Resource | Meaning | Grouped by `resource_group`? |
| --- | --- | --- |
| `Queue(QueueSlotId)` | One submission queue slot is exclusive for the window | No |
| `Completion(CompletionSlotId)` | One completion slot is exclusive for the window | No |
| `ComputeLane(DeviceId, lane)` | One of the device's concurrent calculation lanes | `Compute(device)` |
| `TransferLane(LinkId, lane)` | One directional lane on an internal link | `Transfer(link)` |
| `ExternalTransferLane(DeviceId, lane)` | One host transfer lane on an endpoint device | `ExternalTransfer(device)` |
| `HalfDuplexDirection(DuplexResourceId, LinkId)` | The direction of a half-duplex physical transport | No; conflicts with the opposite direction sharing the resource |
| `NoComputeTransferOverlap(DeviceId)` | A device where transfers and calculations cannot overlap | No |

The resource list is sorted and deduplicated after preparation.  A task must
have one free resource from every grouped family and every fixed resource must
be free for the whole window.

## Call graph and ownership

The production call graph is:

```text
plan_program_candidates / plan_candidates
  -> lower_candidate
       -> initialize_data_images
       -> lower_program_invocation              (Loop calculations)
       -> add_fault_readbacks / add_user_metrics (Loop metrics)
       -> add_alias_dependencies
       -> add_external_outputs                  (Exit egress)
       -> add_phase_barriers
       -> recipe_scheduler::schedule
            -> topology.validate
            -> topology.validate_scheduling_properties
            -> discovery.validate(topology)
            -> prepare_tasks
                 -> prepare_calculation
                 -> prepare_transfer
                      -> shortest_route (only empty distinct-device route)
            -> dependency_graph
            -> critical_path_lengths
            -> reserve_earliest / first_gap
            -> persist_transfer_lane_claims
       -> compact_submission_resources
       -> finalize_auxiliary_resources
       -> build_arena_contract / pack_arenas
       -> DraftPlan::validate
```

`ensure_copy` is the planner's route-sensitive caller.  It enumerates simple
directed paths, creates a trial `TransferChain` with one task per hop, and
calls `trial_chain_timing`, which appends phase barriers and invokes this same
real scheduler.  `trial_timing` uses the same path for each candidate external
egress.  There are no other `schedule` call sites in the current workspace.

The scheduler owns only timing and physical resource claims.  The planner owns
task/value IDs, resident copies, intermediate values, phase-specific task
construction, and final queue/completion compaction.  The executor consumes
the immutable result after core validation and finalization.

## Validated inputs and measured constraints

The first three operations in `schedule` are mandatory, in this order:

```text
topology.validate()
topology.validate_scheduling_properties()
discovery.validate(topology)
```

Any validation failure is converted to `ScheduleErrorKind::InvalidTopology` or
`InvalidDiscovery` before a task is prepared.

### Topology

`Topology::validate` establishes the structural facts used by scheduling:

- topology identity is nonzero;
- machine, node, device, and directed-link IDs are unique and all references
  resolve;
- every configured device is owned by one node and every machine has storage;
- GPU-memory devices have a calculation rate, while RAM and disk do not;
- each directed link connects distinct known devices;
- each transport has exactly two reverse directed edges with matching kind and
  duplex mode;
- half-duplex reverse edges share one nonzero `capacity_resource`;
- full-duplex reverse edges have distinct capacity resources; and
- a capacity resource is not reused by another transport.

`validate_scheduling_properties` rejects `PropertyProvenance::Estimated` for
every device capacity and transfer rate, every present GPU calculation rate,
and every link bandwidth and inflight-transfer count.  Only measured values or
explicit overrides can reach this entry point.  The scheduler does not use
device capacity, topology transfer rate, reservations, or a capacity ledger to
place windows; those values are still validated here because the topology is a
single measured scheduling contract and later arena/capacity steps consume it.

### Discovery profile

`DiscoveryProfile::validate` requires a nonzero discovery identity matching the
topology identity.  It requires exactly one available, asynchronous discovery
record for every topology device and link.  Device records must provide a
nonzero measured or overridden external transfer rate and inflight lane count,
at least one submission queue, and a calculation capability for every GPU.
GPU calculation capabilities must be asynchronous, measured or overridden,
have a nonzero `maximum_concurrent_tasks`, and satisfy the backend workgroup
constraints.  Non-GPU devices must not expose calculation capability.  Link
records must provide measured or overridden bandwidth and lane count, be
asynchronous, and exactly match the corresponding topology direction's
bandwidth and inflight count.

The fields that directly drive this scheduler are:

| Measured discovery/topology field | Used by |
| --- | --- |
| `DiscoveredDevice.calculation.rate` | Calculation duration |
| `DiscoveredDevice.calculation.maximum_concurrent_tasks` | Compute lane group size |
| `DiscoveredDevice.transfer.rate` | External admission and egress duration |
| `DiscoveredDevice.transfer.maximum_inflight_transfers` | External transfer lane group size |
| `DiscoveredDevice.transfer.overlaps_calculation` | Endpoint exclusion resource |
| `DirectedLink.bandwidth` | Internal one-hop duration |
| `DirectedLink.maximum_inflight_transfers` | Internal link lane group size |
| `DirectedLink.duplex` and `capacity_resource` | Opposing half-duplex conflict |

The link discovery values are not read a second time while preparing an
internal transfer.  `DiscoveryProfile::validate` has already proved they equal
the topology direction, so `prepare_transfer` uses the topology link's
bandwidth, lane count, and duplex identity.

`SubmissionSlots` are caller-selected inputs.  The scheduler treats their
queue and completion IDs as globally exclusive resources but does not check
that a slot exists in a `ResourceManifest` or belongs to the operation's
device.  The planner's `compact_submission_resources` and core `DraftPlan`
validation perform that ownership check later.

## Construction pipeline

### 1. Prepare each task

`prepare_tasks` inserts cloned tasks into a `BTreeMap<TaskId, PreparedTask>`.
Duplicate IDs fail immediately with `DuplicateTask` and the task ID.  Each
kind is prepared as follows.

#### Calculations

`prepare_calculation`:

1. Looks up the calculation device in the topology.  An unknown device or a
   non-GPU storage device is `InvalidCalculationPlacement` with both task and
   device context.
2. Requires a discovered calculation capability.  A missing capability is
   `UnavailableCapability`.
3. Computes
   `ceil(work_flops * 1_000_000_000 / measured_flops_per_second)` using
   `calculation_time_ceil`.  Checked arithmetic failures are
   `ArithmeticOverflow`; the resulting duration is raised to one nanosecond.
4. Adds `Queue(submission.queue)` and `Completion(submission.completion)`.
5. Adds `ComputeLane(device, lane)` for every lane in
   `0..maximum_concurrent_tasks`.  `reserve_earliest` later selects one of
   those lanes, so at most the measured number of calculations can overlap.
6. Adds `NoComputeTransferOverlap(device)` when the discovered transfer
   capability disallows transfer/calculation overlap.

The scheduler does not inspect kernel artifacts, value residency, dtypes,
fault-flag shape, or calculation phase.  Those are semantic and realization
contracts checked by `DraftPlan::validate` and `PreparedTask::new` in the
executor.

#### Transfers

`prepare_transfer` first rejects any nonempty input `lane_claims` as
`InvalidTransfer`.  It always starts with the transfer's queue and completion
resources, then handles endpoint pairs:

| Endpoints | Route and duration | Additional resources and failures |
| --- | --- | --- |
| `Device(source) -> Device(destination)` | If route is empty and devices differ, call `shortest_route`. Only a one-link result is accepted. A same-device empty route is a one-nanosecond copy. A supplied route longer than one link is rejected. A one-link route is checked by `Topology::validate_route` and charged `transfer_time_ceil(bytes, link.bandwidth).max(1)`. | Every measured directional lane on the link is offered as `TransferLane`. A half-duplex link also adds `HalfDuplexDirection(capacity_resource, link)`. Both endpoint devices add `NoComputeTransferOverlap` when their transfer capability disallows overlap. Unknown endpoints, links, wrong direction, or a route that does not end at the destination are `InvalidTransfer`; missing endpoint discovery is `UnavailableCapability`. |
| `External -> Device` or `Device -> External` | Route must be empty. Duration is `transfer_time_ceil(bytes, discovered_device.transfer.rate).max(1)`. | Every external transfer lane on the discovered endpoint is offered as `ExternalTransferLane`. `NoComputeTransferOverlap(device)` is added when required. A nonempty route or missing endpoint discovery is an `InvalidTransfer` or `UnavailableCapability`, respectively. |
| `External -> External` | No device-owned rate or resource exists. | Always `InvalidTransfer`; the operation is not schedulable. |

For an empty distinct-device route, `shortest_route` validates the topology,
uses measured link bandwidth, and returns the least store-and-forward path with
stable link-ID tie breaking.  If it returns `NoRoute`, that error propagates.
If it returns more than one link, `prepare_transfer` returns `InvalidTransfer`
instead of inventing intermediate values.  The route implementation and the
planner's path lowering are described in
[`scheduler/.docs/src/route.md`](route.md).

The internal transfer preparation uses the topology link's lane count because
discovery validation has already required an exact match.  A half-duplex
resource is not itself a lane group: same-direction transfers may use
different lanes and overlap up to the measured lane count, while the conflict
function rejects an overlapping reservation on the opposite directed link
sharing that capacity resource.

#### Metrics

`Metric` preparation returns `Nanoseconds::new(1)` and only its queue and
completion resources.  No transfer, compute, or overlap resource is added.
This is intentional: a metric is a four-byte device readback represented by a
specialized `TaskKind`, and its value residency and metric slot are checked at
the plan boundary rather than inferred from the scheduler's input.

After a kind is prepared, resources are sorted and deduplicated.  This matters
for a same-device transfer, where the source and destination may add the same
`NoComputeTransferOverlap` resource twice.

### 2. Build the dependency graph

`dependency_graph` builds a `BTreeSet<(predecessor, successor)>` so the graph
has stable edge order and no duplicate edge from repeated barrier generation.
For every input dependency it checks:

- the referenced task exists, otherwise `UnknownDependency`;
- the dependency is not repeated within the task, otherwise
  `UnknownDependency`; and
- the predecessor phase is not later than the dependent phase, otherwise
  `InvalidLifecycleDependency`.

It then adds global lifecycle edges for every pair whose predecessor phase is
less than the successor phase: all Init tasks precede all Loop tasks, and all
Init and Loop tasks precede all Exit tasks.  These edges are internal graph
edges.  The returned task keeps the caller's dependency vector unchanged.
The planner currently adds the same phase barriers explicitly, which the
`BTreeSet` deduplicates; direct scheduler callers get the barrier behavior even
when they omit those dependencies.

The resulting `Successors` lists are sorted by task ID, and every task gets an
indegree entry, including tasks with no outgoing edge.

### 3. Compute critical-path priority

`critical_path_lengths` runs a Kahn topological traversal over the graph.  A
cycle is reported as `DependencyCycle` before any reservation occurs.  In
reverse topological order it computes:

```text
critical_path(task) = duration(task)
                    + max(critical_path(successor))
```

The addition is checked and can return `ArithmeticOverflow`.  For each initial
zero-indegree task, `schedule` pushes `(critical_path, Reverse(TaskId))` into a
max `BinaryHeap`.  The largest remaining critical path is selected first; a
tie is resolved by the smallest stable task ID.  This is list scheduling, not
an attempt to solve a global resource-constrained optimum.

### 4. Reserve windows and materialize tasks

The mutable scheduling state is:

```text
remaining_dependencies: BTreeMap<TaskId, usize>
ready:                 BinaryHeap<(u64, Reverse<TaskId>)>
task_end:              BTreeMap<TaskId, Nanoseconds>
scheduled:             BTreeMap<TaskId, Task>
reservations:          BTreeMap<Resource, Vec<ScheduleWindow>>
phase_end:             [Nanoseconds; 3]       // Init, Loop, Exit
```

For every ready task, in heap order:

1. `dependency_end` is the maximum end of its declared dependencies, or zero
   when none are declared.  Because the graph's phase barrier edges also gate
   readiness, every lower-phase task has already been scheduled even though
   those internal edges are not copied to the output dependency list.
2. `phase_floor` is zero for Init, the end of all Init work for Loop, and the
   maximum of Init and Loop ends for Exit.  `earliest` is the maximum of the
   dependency end and phase floor.
3. `reserve_earliest(earliest, duration, resources, reservations)` finds the
   first legal half-open interval.  The selected resources are inserted into
   the reservation map for that interval.
4. The cloned kind is passed to `persist_transfer_lane_claims`.  For a
   calculation or metric this is a no-op.  For a transfer it projects only
   selected lane resources into sorted, deduplicated `TransferLaneClaim`
   values and verifies that the claim set is complete for the endpoint form.
5. The scheduler updates the phase end and task end, inserts a `Task` with the
   original dependency vector, and decrements every successor indegree.  A
   successor whose count reaches zero is pushed with its critical-path key.

If all prepared tasks are scheduled, `makespan` is the maximum returned task
end.  A mismatch in scheduled and prepared counts is reported as
`DependencyCycle`; in normal operation `critical_path_lengths` has already
made that case unreachable.  An empty, otherwise valid task list returns an
empty `tasks` vector and a zero makespan.

## Earliest-gap resource algorithm

`reserve_earliest` partitions a prepared resource list into:

- `fixed`: queue, completion, half-duplex direction, and no-overlap resources;
- `groups`: distinct `Compute(device)`, `Transfer(link)`, and
  `ExternalTransfer(device)` families.

`first_gap` starts at `earliest` and repeats until it can reserve a complete
window:

1. Add `duration` with checked arithmetic. Overflow is `ArithmeticOverflow`.
2. If any fixed resource has an overlapping reservation, advance `start` to
   the latest conflicting end and retry.
3. For each lane group, inspect its lanes in sorted resource order. Select the
   first lane with no overlapping reservation for the candidate window. If all
   lanes conflict, record the earliest release among those lanes.
4. If one or more groups are unavailable, advance to the earliest recorded
   release and retry. An empty lane group is `UnavailableCapability`.
5. When every group has a selected lane and every fixed resource is free,
   sort the selected resource list and return the candidate window.

`resource_conflict_end` treats windows as half-open via
`ScheduleWindow::overlaps`: touching at an end/start boundary is legal.  Exact
resource conflicts look up one resource key.  A
`HalfDuplexDirection(capacity, direction)` conflict scans reservations for the
same capacity resource with a different direction, so opposing links sharing
one half-duplex transport cannot overlap.  Full-duplex reverse links have
different capacity resources and never add this opposing-direction resource.

This algorithm both honors measured lane capacities and leaves independent
groups overlap-capable.  It also serializes every task that claims the same
fixed `NoComputeTransferOverlap(device)` resource.  In the current code that
includes transfers touching the device as well as calculations, so when the
capability disallows overlap, transfer-to-transfer overlap on that endpoint is
also prevented by the exact resource key.

## Transfer lane claims and queue state

`persist_transfer_lane_claims` is the only place where scheduled lane claims
are written.  It projects selected resources and then checks by endpoint pair:

| Endpoint pair | Required scheduled claims |
| --- | --- |
| Device to device | Only `TransferLaneClaim::Link`; the set of claimed links equals the route's link-ID set. A same-device empty route therefore has no claims. |
| External to device or device to external | Exactly one `TransferLaneClaim::External` naming the endpoint device; no link claims. |
| External to external | Invalid and never validly claimable. |

The scheduler chooses one lane, not every lane offered by preparation.  It
therefore persists exactly one claim for a one-link internal route and one
claim for an external operation.  The resulting claims are strictly sorted and
unique because resources and claims are canonicalized before persistence.
Core validation independently checks lane ranges, route equality, queue and
completion ownership, and pairwise contention.  This second check is useful
because `schedule` intentionally accepts caller-provided slot IDs without
looking at a later `ResourceManifest`.

The planner initially gives each task a stable queue/completion pair through
`LoweringState::next_submission`.  After scheduling, `compact_submission_resources`
recolors the non-overlapping task intervals into the smallest deterministic
per-device queue/completion manifest allowed by measured
`maximum_submission_queues`, then rewrites every calculation, transfer, and
metric submission field.  The static schedule itself never performs that
compaction and never uses the measured queue limit.

## Planner task construction and routes

The task phases supplied by the current planner are:

| Phase | Planner producer | Produced unscheduled tasks and dependencies |
| --- | --- | --- |
| Init | `initialize_data_images` | One `External -> Device` transfer per topology device, containing that device's packed data image and fault flags. It has no explicit dependency and an empty route. |
| Loop | `lower_program_invocation` | One `Calculation` per lowered program stage. Stage barriers and ready-buffer producers become dependencies. Each task receives a loop iteration domain. |
| Loop | `ensure_copy` / `build_transfer_chain` | One internal transfer per directed route link, or one explicit same-device copy with an empty route. The first hop depends on the source producer; later hops depend on the preceding hop. Intermediate values are allocated by the planner. Every hop receives a loop domain. |
| Loop | `add_fault_readbacks` | One four-byte `MetricPurpose::FaultReadback` per `(device, iteration domain)` fault cohort, directly depending on every checked calculation. |
| Loop | `add_user_metrics` | One `MetricPurpose::User` metric from the producer-resident copy. It depends on the value producer and all known fault readbacks. |
| Exit | `add_external_outputs` | One `Device -> External` transfer for each selected safe direct resident output copy. It depends on that copy's producer and has an empty route. |

`add_phase_barriers` in the planner adds every Init task to every Loop task,
and every Init and Loop task to every Exit task.  `dependency_graph` adds the
same lifecycle ordering internally, so direct scheduler callers cannot start a
later phase early.  The scheduler still validates explicit dependencies for
phase order and cycles.

For a cross-device copy, the planner does not use `Route::duration` as the
schedule result.  It enumerates every simple directed route from every
domain-compatible resident source, builds one-hop chains, and trial-schedules
each chain with existing work.  A candidate is ranked by final-hop end,
chain makespan, source device, source value, and route IDs.  The chosen chain is
rebuilt with stable IDs and then passed to the final scheduler call.  If no
route exists the planner records `NoRoute`; if routes exist but none can be
scheduled, the candidate is infeasible.  A route with multiple links reaching
`prepare_transfer` is a lowering error, not permission to create a hidden
intermediate task.

## Failure and diagnostic contract

`ScheduleError` contains a `ScheduleErrorKind`, optional task and device IDs,
and a message.  The static schedule can produce these outcomes:

| Condition | Result and stage |
| --- | --- |
| Topology identity, structure, or any estimated scheduling property is invalid | `InvalidTopology` from the initial topology validation |
| Discovery identity, availability, measured property, link match, or asynchronous capability is invalid | `InvalidDiscovery` from initial discovery validation |
| A task ID appears twice | `DuplicateTask` from `prepare_tasks` |
| Calculation names an unknown or non-GPU device | `InvalidCalculationPlacement` from `prepare_calculation` |
| Calculation capability or transfer endpoint discovery is absent | `UnavailableCapability` with task and usually device context |
| An unscheduled transfer has lane claims, an invalid endpoint pair, an external route, a wrong or multi-link internal route, or a route that fails topology validation | `InvalidTransfer` from `prepare_transfer` |
| Automatic direct-route selection has no directed path | `NoRoute` from `shortest_route` |
| A dependency is unknown or repeated | `UnknownDependency` from `dependency_graph` |
| A dependency points from a later phase | `InvalidLifecycleDependency` from `dependency_graph` |
| Explicit or lifecycle edges form a cycle | `DependencyCycle` from `critical_path_lengths` or the final count check |
| Unit conversion, critical-path addition, or schedule-window addition overflows | `ArithmeticOverflow` |
| A lane group has no lanes, or selected transfer claims are incomplete | `UnavailableCapability` from `first_gap` or `persist_transfer_lane_claims` |

`ScheduleErrorKind::InsufficientCapacity` is part of the scheduler crate's
shared error enum for arena packing, but `schedule` itself does not inspect
arena capacity and does not emit that kind.  `pack_arenas` and the planner's
capacity checks handle it after scheduling.

The scheduler intentionally does not report malformed semantic tasks that are
outside its timing inputs.  Unknown value IDs, value/device or byte mismatches,
wrong task-kind phases, missing queue/completion manifest entries, invalid
metric slots, missing fault readbacks, dependency window order, lane ranges,
and pairwise contention are independently checked by
`core/src/plan.rs` before finalization.  `executor::PreparedTask::new` then
rejects phase/task combinations again while creating backend work.  These are
not fallback paths: a scheduler result that fails those checks is an invalid
candidate.

Planner error mapping is candidate-aware:

| Scheduler error | `lower_candidate` mapping | `trial_chain_timing` behavior |
| --- | --- | --- |
| `DependencyCycle`, `InvalidLifecycleDependency` | `PlannerErrorKind::DependencyConflict` | Same |
| `ArithmeticOverflow`, `InsufficientCapacity` | `PlannerErrorKind::CandidateInfeasible` | Candidate trial becomes `Ok(None)` |
| `NoRoute` | `PlannerErrorKind::NoRoute` | Candidate trial becomes `Ok(None)` |
| Other kinds | `PlannerErrorKind::Schedule` | Surfaced as planner schedule failure |

## End-to-end execution role

The static schedule is timing and resource evidence in the complete path:

1. Planner lowering creates the task/value graph and calls `schedule`.  The
   returned `makespan` ranks otherwise valid placement candidates.
2. The planner compacts provisional queue and completion IDs, derives staging
   and scratch peaks from the scheduled windows, derives arena object
   lifetimes, packs arenas, checks total measured capacity, and constructs a
   `DraftPlan` containing the scheduled `Task` values.
3. `DraftPlan::validate` independently proves nonempty windows, dependency
   schedule order, lifecycle order, route shape, lane claims, queue and
   completion ownership, measured link/device concurrency, half-duplex
   exclusion, endpoint transfer/compute overlap, and fault-readback barriers.
4. `FinalizedBundle` resolves every logical value in each task endpoint to an
   immutable arena location.  It preserves each task's phase, window,
   dependencies, route, lane claims, and submission slots.
5. `executor::PreparedPhases::new` partitions the finalized tasks into Init,
   Loop, and Exit, sorts each phase by `(window.start, task.id)`, resolves
   endpoints and values, and prepares backend work.  The executor does not use
   static durations to sleep; windows govern runnable overlap and dependency
   ordering.
6. Init submits the one packed external admission per device.  Once all Init
   tasks complete, the run enters Loop.  For each iteration, tasks whose
   `IterationDomain` does not contain that iteration are marked complete without
   backend submission.  Active tasks are submitted only after dependencies are
   complete, with the explicit same-queue pipelining exception.  A pending task
   with an overlapping window blocks another submission unless the backend
   explicitly permits that same-queue pipeline.  The windows and claims made by
   this scheduler therefore prevent queue, completion, lane, half-duplex, and
   forbidden endpoint overlap at runtime.
7. Loop metrics and fault readbacks publish their fixed slots.  Exit tasks then
   move each selected output to `External`; exit image collection and
   acknowledgement use the same finalized task identity.
8. Native CUDA/HSA, local executor, bridge, and remote worker paths all consume
   the same resolved route, lane claims, windows, dependencies, and submission
   slots.  Worker projections retain the full task window and filter only the
   dependencies that are local to that worker.  They still reject an
   executor-visible route longer than one link.

The result is an immutable `init -> loop -> exit` execution contract.  The
static scheduler chooses when independent physical operations may coexist; it
does not add model semantics, unroll loop iterations, or provide a runtime
fallback for an invalid transition.

## Invariants and non-goals

- Scheduling is deterministic for fixed validated topology, discovery, tasks,
  and stable IDs.  B-tree collections, sorted successors/resources, critical
  path priority, and smallest-ID ties remove input iteration ambiguity.
- Every returned window is nonempty, half-open, and starts no earlier than all
  declared dependencies and the global phase floor.
- Every executor-visible internal transfer has zero or one directed link.  A
  multi-link operation is represented only as planner-created hop tasks joined
  by dependencies and resident values.
- Every scheduled transfer has complete canonical lane claims: one link claim
  per routed link or one external claim on its endpoint, with no claims for a
  same-device empty-route copy.
- Queue and completion slots, half-duplex opposite directions, and forbidden
  endpoint overlap resources never overlap.  Compute, link, and external lane
  groups permit overlap up to their measured counts.
- Lifecycle ordering is global: all Init work is before Loop work, and all Init
  and Loop work is before Exit work.  Explicit task dependencies cannot point
  from a later phase.
- The scheduler consumes measured or explicitly overridden timing and lane
  properties.  It does not turn estimated seed values, theoretical capacity,
  queue limits, reservations, or an arena layout into schedule evidence.
- Loop repetition is not represented by duplicate scheduled tasks.  The
  finalized executor reuses these windows and task identities with the
  planner-provided iteration domains.
- Core validation and executor preparation remain required.  A mathematically
  schedulable task list is not a valid Recipe plan until values, artifacts,
  phases, resource manifests, arenas, and backend contracts all validate.
