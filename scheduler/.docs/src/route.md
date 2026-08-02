<!--
This document describes the routing boundary used by the scheduler.  The
source of truth is scheduler/src/route.rs and the transfer half of
scheduler/src/static_schedule.rs.  The planner and core/executor references
below explain the contracts that make that boundary usable end to end.
-->

# Routing

## Purpose and boundary

Routing answers two related but deliberately different questions:

1. `recipe_scheduler::shortest_route` (`scheduler/src/route.rs`) computes a
   deterministic path candidate and its conservative store-and-forward time.
2. `schedule` (`scheduler/src/static_schedule.rs`) turns an unscheduled
   transfer into one schedulable physical hop.  If an internal transfer has no
   route, it may call `shortest_route` as a convenience, but it accepts only a
   direct result.  It never allocates an intermediate value or creates another
   task.

The planner owns the operation that a multi-link path requires.  It enumerates
simple directed paths, tests each path through the real scheduler, and lowers a
selected path into dependency-chained transfer tasks with resident values
between hops.  This is why the scheduler rejects a route with more than one
link instead of silently splitting it.  A split without planner-owned values
would describe an executor task that has no legal source or destination
allocation.

The public scheduler module re-exports `Route` and `shortest_route` from
`scheduler/src/lib.rs`.  Its schedule entry point also re-exports
`UnscheduledTask`, `StaticSchedule`, and `schedule`.  The scheduler has no
unsafe routing code and consumes only topology properties that are measured or
explicitly overridden.

## Data model

`Route` is the public result:

```text
Route {
    links: Vec<LinkId>,
    duration: Nanoseconds,
}
```

`links` is an ordered sequence of directed link identities.  The first link
must start at the source device, every later link must start at the previous
link's destination, and the last link must end at the destination.  `duration`
is the sum of the per-hop transfer times used while searching.  It is a path
candidate cost, not an executor operation and not a reservation.  The static
scheduler recomputes each executor-visible hop's duration while preparing the
task and does not copy `Route::duration` into a task.

`TransferTask` in `core/src/schedule.rs` carries the route that reaches the
executor:

```text
TransferTask {
    source: TransferEndpoint,
    destination: TransferEndpoint,
    bytes: ByteCount,
    route: Vec<LinkId>,
    lane_claims: Vec<TransferLaneClaim>,
    submission: SubmissionSlots,
}
```

An internal transfer is either a same-device copy with an empty route or one
directed physical hop with one link.  An external admission or egress also has
an empty route, because the host endpoint is not a topology link.  A scheduled
task has canonical lane claims; an unscheduled task must have no lane claims.
The route and lane-claim invariants are repeated by `core::DraftPlan` and
`core::FinalizedBundle` validation before native execution.

## Call graph

The relevant calls are:

```text
recipe_planner::lower_candidate
  -> recipe_scheduler::schedule
       -> topology.validate
       -> topology.validate_scheduling_properties
       -> discovery.validate(topology)
       -> prepare_tasks
            -> prepare_transfer
                 -> shortest_route (only for an empty distinct-device route)
       -> dependency_graph / critical_path_lengths
       -> reserve_earliest
       -> persist_transfer_lane_claims

recipe_planner::ensure_copy
  -> directed_routes
  -> build_transfer_chain (one UnscheduledTask per link)
  -> trial_chain_timing
       -> add_phase_barriers
       -> schedule
```

`shortest_route` has no caller outside the scheduler crate in the current
workspace.  The direct caller is `prepare_transfer`; the planner normally
passes explicit one-link routes after its own route search.  The same
`shortest_route` function remains useful for callers that construct an
unscheduled direct transfer and want the scheduler to select a direct link.

After scheduling, `core/src/plan.rs` validates the route, lane claims, task
windows, dependencies, and contention.  `native-executor/src/local.rs` and the
backend-specific executors accept only zero- or one-link executor-visible
transfers.  Cross-backend bridge and remote worker paths also require exactly
one link.  Thus the one-hop restriction is an end-to-end contract, not merely
an optimization in this crate.

## Preconditions and measured inputs

`shortest_route(topology, source, destination, bytes)` first performs:

1. `Topology::validate()`.
2. `Topology::validate_scheduling_properties()`.
3. Existence checks for `source` and `destination` in `topology.devices`.

The first two checks happen before either device check.  Therefore an invalid
or theoretical property anywhere in the topology can prevent route selection,
even if that property is not on the requested path.  `validate_scheduling_properties`
requires every device capacity and transfer rate, every GPU calculation rate,
and every directed-link bandwidth and lane count to be `Measured` or
`Override`.  `Estimated` values may seed probing but cannot drive a schedule.

`Topology::validate()` establishes the structural facts used by the search:

- device IDs and directed-link IDs are unique and every link endpoint exists;
- a directed link connects distinct devices;
- each transport has exactly two reverse directed edges;
- both directions have the same transport kind and duplex mode;
- a half-duplex pair shares one nonzero `capacity_resource`;
- a full-duplex pair has different nonzero capacity resources;
- capacity resources are not shared by different transports.

`DirectedLink.bandwidth` is a `Property<BytesPerSecond>`, and
`maximum_inflight_transfers` is a nonzero `Property<TransferLaneCount>`.  The
search uses the topology direction's bandwidth value.  `DiscoveryProfile` is
not an argument to `shortest_route`, but `schedule` validates it before any
task is prepared.  Discovery must cover every topology device and link, mark
them available and asynchronous where required, and match each link's
bandwidth and lane count exactly.  This prevents the scheduler from selecting
against a profile that disagrees with the topology cost or capacity.

`bytes` is an exact `ByteCount`.  `transfer_time_ceil(bytes, bandwidth)`
computes a nanosecond ceiling using checked arithmetic.  A zero-byte result is
raised to one nanosecond so that every hop and every same-device transfer has a
nonempty scheduling window.

## `shortest_route` algorithm

### Initialization

After validation and endpoint checks:

- If `source == destination`, the function returns
  `Route { links: Vec::new(), duration: Nanoseconds::new(1) }`.  No link is
  required for an explicit same-device copy.
- Otherwise, it builds an adjacency `BTreeMap<DeviceId, Vec<&DirectedLink>>`
  by `link.from` and sorts every outgoing list by stable `LinkId`.
- `best` stores at most one known pair `(elapsed_nanoseconds, path_link_ids)`
  per device.  The source starts at `(0, [])`.
- `ready` is a min-priority heap implemented as
  `BinaryHeap<Reverse<(u64, Vec<LinkId>, DeviceId)>>`.

The tuple ordering is intentional.  Lower elapsed time wins; equal elapsed
time is ordered lexicographically by the complete link-ID path; a remaining
device-ID comparison makes heap behavior deterministic when the first two
fields are equal.  The path order does not use topology declaration order.

### Relaxation and completion

For each popped state `(elapsed, path, current)`:

1. A stale heap item is discarded unless `best[current]` is exactly the same
   `(elapsed, path)`.  This keeps an older, slower path from relaxing edges.
2. If `current == destination`, the path is returned immediately as
   `Route { links: path, duration: Nanoseconds::new(elapsed.max(1)) }`.
3. Every outgoing link is charged
   `hop = transfer_time_ceil(bytes, link.bandwidth.value).get().max(1)`.
4. `elapsed.checked_add(hop)` forms the candidate total.  Overflow returns
   `ScheduleErrorKind::ArithmeticOverflow`.
5. The link ID is appended to a cloned path.  The candidate replaces
   `best[link.to]` only when the pair `(candidate_elapsed, candidate_path)` is
   strictly smaller than the known pair.  A replacement is pushed into the
   heap.

All hop costs are positive after the `max(1)` operation.  Consequently a path
that revisits a device cannot improve that device's existing pair, so the
single-best-per-device Dijkstra state remains finite without a separate
visited set.  The resulting cost is store-and-forward: hop times are summed,
not overlapped or pipelined.

The function returns `ScheduleErrorKind::NoRoute` only after the heap is
exhausted.  Edges are directed, so a reverse physical edge is a separate
candidate with its own ID, bandwidth, lane count, and direction.

### Determinism and tie behavior

There is one canonical result for a fixed validated topology and byte count:

- adjacency lists are sorted by `LinkId`;
- heap items compare duration, path IDs, and device ID;
- `best` compares duration first and full path IDs second;
- `Route` preserves the selected link order.

Equal-duration routes therefore do not depend on the order of `topology.links`
or on hash-map iteration.  A lexicographically smaller path wins even when it
has a different number of hops.  The search is cost-only.  It does not account
for current queue reservations, other tasks, duplex contention, or endpoint
compute overlap.  Those constraints are applied later by `schedule`.

## The paired scheduler half: `prepare_transfer`

`prepare_tasks` clones each `UnscheduledTask`, then calls
`prepare_transfer` for a transfer.  The clone is important: automatic direct
route selection mutates the prepared copy, while the caller's unscheduled task
still has no lane claims or scheduler-owned state.

The first invariant is that `transfer.lane_claims` must be empty.  A nonempty
list is an `InvalidTransfer` because lane selection belongs to static
scheduling, not to an unscheduled caller.

### Device to device

For `(Device { device: source, .. }, Device { device: destination, .. })`:

1. If `route` is empty and `source != destination`, call `shortest_route`.
   Only a one-link result is accepted.  A multi-link result returns
   `InvalidTransfer` with the message that the path must be lowered into one
   dependency-chained task per directed link.
2. If the supplied or selected route has more than one link, return
   `InvalidTransfer`.  The scheduler is not allowed to infer intermediate
   values.
3. Call `Topology::validate_route(source, destination, route)`.  It checks
   link existence, link direction at every hop, nonempty routes between
   distinct devices, and the final endpoint.  A same-device empty route is the
   one permitted empty internal route.
4. With an empty same-device route, use duration one and add no link lane or
   duplex resource.
5. With one link, look it up, charge
   `transfer_time_ceil(transfer.bytes, link.bandwidth.value).get().max(1)`,
   and add every directional lane resource
   `Resource::TransferLane(link.id, lane)` for
   `lane in 0..link.maximum_inflight_transfers.value.get()`.
6. If the link is half duplex, add
   `Resource::HalfDuplexDirection(link.capacity_resource, link.id)`.  The
   opposite directed edge with the same capacity resource will conflict with
   this resource while windows overlap.  Full-duplex directions have distinct
   capacity resources and do not receive this opposing-direction resource.
7. For both endpoint devices, look up the discovered device.  Missing
   discovery is `UnavailableCapability` with the task and endpoint device.
   When `device.transfer.overlaps_calculation` is false, add
   `Resource::NoComputeTransferOverlap(endpoint)` for that endpoint.

The base resources always include the transfer's `SubmissionSlots`, namely a
queue and completion resource.  Thus a one-hop task consumes a queue slot, a
completion slot, one link lane, and possibly one half-duplex and/or two
no-overlap resources for its full window.

### External admission and egress

For external-to-device or device-to-external transfers:

- `route` must be empty.  An internal link on an external operation is an
  `InvalidTransfer`.
- The endpoint device must be present in discovery.
- The duration uses the discovered endpoint's external transfer rate, not a
  topology link bandwidth.
- Every lane in
  `0..capability.transfer.maximum_inflight_transfers.value.get()` is offered
  as `Resource::ExternalTransferLane(device, lane)`.
- `NoComputeTransferOverlap(device)` is added when the capability disallows
  overlap.

External-to-external is rejected as `InvalidTransfer`; it has no device owner,
rate, or schedulable queue boundary.

### Resource selection and persisted state

`prepare_transfer` returns a duration and a resource set.  `schedule` sorts and
deduplicates those resources, then `reserve_earliest` chooses the first
available resource from each compute, link-transfer, or external-transfer lane
group while requiring every fixed resource to be free.  Queue and completion
slots are fixed resources and are therefore exclusive.  The selected window
is recorded in the reservation ledger.

After the window is selected, `persist_transfer_lane_claims` projects only
lane resources into `TransferLaneClaim` values, sorts and deduplicates them,
and writes them into the cloned scheduled `TransferTask`:

- internal transfers must contain only `Link` claims, and the set of claimed
  link IDs must equal the route's link-ID set;
- same-device internal copies have an empty route and therefore no link claim;
- external transfers must contain exactly one `External` claim on their
  endpoint device;
- an empty or incomplete selected lane group is an
  `UnavailableCapability` failure.

The scheduled task's route and claims are then immutable input to plan
finalization and native execution.  The core validator independently checks
that each claimed lane is within the topology or discovery lane count, that
claims are strictly sorted and unique, and that overlapping tasks do not share
a lane, queue, or completion slot.

## Planner-owned multi-hop lowering

The scheduler's shortest-path fallback must not be confused with the planner's
route search.  `planner/src/planner.rs` contains `directed_routes`,
`enumerate_directed_routes`, `build_transfer_chain`, and
`ensure_copy`:

1. `directed_routes` builds outgoing `(LinkId, DeviceId)` pairs, sorts them,
   and recursively enumerates every simple directed path.  A `BTreeSet` of
   visited devices prevents cycles.  A same-device request is represented by
   one empty route.
2. `ensure_copy` considers each route from each resident source copy that is
   valid for the consumer's iteration domain.  It calls `build_transfer_chain`
   and then `trial_chain_timing`, which appends the trial tasks to the current
   tasks, adds lifecycle barriers, and calls the real scheduler.
3. `build_transfer_chain` first calls `Topology::validate_route`.  For a
   nonempty route it creates one endpoint pair and one `TransferTask` per link.
   For an empty route it creates a same-device task only after validation has
   established that source and destination are equal.
4. Every hop receives a stable task ID offset from the allocator, a stable
   intermediate value except for the final destination value, and a unique
   queue/completion pair.  The first hop depends on the source producer; every
   later hop depends on the preceding hop.  The source value is replaced by
   the previous hop's resident value as the chain advances.
5. The final logical destination value is kept at the allocator head.  The
   intermediate values follow route order even though their producers execute
   first.  This gives arena construction and dependency analysis an explicit
   physical location for every hop.
6. Candidate routes are selected by `(completion_end, schedule_makespan,
   source_device, source_value, route)`, not by the `shortest_route` duration
   alone.  The source copy is carried in the candidate record after those
   fields are chosen, but is not an additional tie-break comparison.  A longer
   physical path can win when its resources and dependencies produce a better
   real schedule.
7. The selected chain is rebuilt, stable IDs are consumed, and its unscheduled
   tasks are added to the candidate before the final `schedule` call.

If no directed path exists, `ensure_copy` reports `PlannerErrorKind::NoRoute`.
If paths exist but every chain is rejected by arithmetic overflow, capacity, or
other scheduling constraints, it reports candidate infeasibility.  During a
trial, `ArithmeticOverflow`, `InsufficientCapacity`, and `NoRoute` mean that
candidate is not schedulable and produce `Ok(None)`; dependency-cycle and
invalid-lifecycle errors are planner dependency conflicts; other scheduler
errors are surfaced as planner schedule errors.  The final lowering maps
`ScheduleErrorKind::NoRoute` to the same planner no-route kind.

This split keeps all model semantics in the planner.  The scheduler receives
only physical hop tasks and never invents a value, task ID, queue, completion,
or dependency for an intermediate device.

## Scheduling role of a route

The complete production path is:

1. The cluster/probe path supplies a `Topology` whose link bandwidth and lane
   counts are measured or explicitly overridden, and a matching
   `DiscoveryProfile` with availability and asynchronous capabilities.
2. Planner lowering creates init, loop, and exit transfer/calculation tasks.
   Internal copies use explicit one-link tasks after planner route selection;
   data-image admission and external output egress use empty external routes.
3. `schedule` validates topology and discovery, prepares every task, and
   computes route-aware durations.  A missing direct internal route can be
   selected by `shortest_route`; a resulting multi-hop path is a hard signal
   that planner lowering was skipped.
4. Dependencies plus global `init -> loop -> exit` phase edges form a DAG.
   Critical-path lengths prioritize ready work.  Route duration contributes to
   the hop task's critical path and earliest resource reservation.
5. `reserve_earliest` finds a nonoverlapping window across queue, completion,
   link or external lanes, half-duplex direction, and endpoint overlap
   resources.  The selected lane is persisted in the scheduled task.
6. `StaticSchedule.tasks` is returned in stable `TaskId` order with a
   `makespan`.  Planner resource compaction and arena packing consume the
   schedule, then core plan validation checks the independent end state.
7. Finalization resolves values to arena locations.  Native local, CUDA, HSA,
   bridge, and remote execution consume the same route and lane claims.  They
   reject an executor-visible route longer than one link, so a multi-hop path
   can execute only as the planner's dependency chain.

No part of this path overlaps links speculatively.  A route is a sequence of
directed physical links, a transfer task is one hop, and an intermediate value
is the explicit state that connects hops.

## Failure and diagnostic contract

`ScheduleError` carries a `ScheduleErrorKind`, optional `TaskId`, optional
`DeviceId`, and a human-readable message.  Routing-related outcomes are:

| Condition | Result | Where it originates |
| --- | --- | --- |
| Topology identity/structure is invalid | `InvalidTopology` | `Topology::validate` in `shortest_route` or `schedule` |
| Any topology scheduling property is estimated | `InvalidTopology` | `validate_scheduling_properties` |
| Source or destination is absent during automatic selection | `InvalidTransfer` plus the endpoint device | `shortest_route` |
| No directed path is reachable | `NoRoute` | exhausted `shortest_route` heap |
| Rate multiplication, duration addition, critical path, or window overflows | `ArithmeticOverflow` | unit conversion, route search, or scheduler reservation |
| Caller supplies a route with more than one link | `InvalidTransfer` for the task | `prepare_transfer` |
| Automatic selection finds a multi-link path | `InvalidTransfer` for the task | `prepare_transfer` |
| Route has an unknown link, wrong direction, missing endpoint, or empty distinct-device path | `InvalidTransfer` for the task | `Topology::validate_route` mapping |
| An unscheduled transfer already has lane claims | `InvalidTransfer` for the task | `prepare_transfer` |
| External transfer names an internal route | `InvalidTransfer` for the task | `prepare_transfer` |
| External-to-external transfer | `InvalidTransfer` for the task | `prepare_transfer` |
| Required endpoint discovery is absent | `UnavailableCapability` for task and device | `prepare_transfer` |
| No lane exists in a lane group | `UnavailableCapability` | `first_gap` |
| Scheduler-selected claims do not exactly match route or external endpoint | `UnavailableCapability` for task | `persist_transfer_lane_claims` |
| Discovery identity, availability, rate, lane, or asynchronous capability disagrees | `InvalidDiscovery` | `schedule` before preparation |

The planner may translate `NoRoute`, arithmetic overflow, or capacity failure
into candidate search outcomes, but it does not turn an invalid multi-hop
executor task into a hidden fallback.  Native and remote layers preserve the
one-hop failure if an invalid plan reaches them.

## Invariants to preserve

- Routing uses directed `LinkId` identities.  Never substitute an undirected
  edge or infer a reverse link without looking it up.
- Every route is validated against the exact source and destination before a
  transfer is scheduled.
- A scheduler-visible internal transfer has zero or one link.  Multi-link
  movement is a planner chain of one-link tasks and resident intermediate
  values.
- Route cost is `sum(ceil(bytes / measured_or_overridden_link_bandwidth))`,
  with each hop and same-device operation at least one nanosecond.
- Tie-breaking is stable and lexicographic by link IDs after elapsed time.
- Queue and completion slots are exclusive fixed resources.  A transfer lane
  is selected from the measured directional lane group, exactly one claim per
  routed link.  External transfers claim one endpoint lane.
- Half-duplex opposing directions share a capacity resource and cannot overlap;
  full-duplex directions use distinct resources.
- Endpoint transfer capabilities control calculation overlap.  Discovery link
  rates and lane counts must match topology values before scheduling.
- Lane claims are scheduler output, not caller input, and are sorted, unique,
  and complete before plan finalization.
- `Route::duration` is a search cost only.  It is not permission to submit a
  multi-link executor task and it does not reserve resources by itself.
