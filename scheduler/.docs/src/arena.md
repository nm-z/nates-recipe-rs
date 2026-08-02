# `recipe-scheduler::arena`

```yaml
document: recipe_scheduler.arena
source: scheduler/src/arena.rs
kind: deterministic_arena_packing
authority:
  - scheduler/src/arena.rs
  - planner/src/planner.rs
  - core/src/schedule.rs
  - core/src/plan.rs
  - prepare/src/lib.rs
  - executor/src/executor.rs
  - executor/src/backend.rs
  - native-executor/src/local.rs
  - system-contract.md
```

This document records the implemented arena contract, not a proposed allocator.
`recipe-scheduler::arena::pack_arenas` is the small physical-layout step between
the planner's logical memory contract and the immutable execution bundle. The
planner decides which values share a logical object and computes each object's
live interval. The scheduler chooses one byte offset for every object on every
topology device. Preparation invokes the same operation again after native
realization has produced the final capacity ledger. Finalize validates those
offsets, resolves value addresses, and then the executor allocates exactly the
resulting per-device arena buffers.

The important boundary is therefore:

```text
graph lowering and static schedule
    -> ArenaObject { device, bytes, alignment, lifetime }
    -> pack_arenas(topology, objects, capacity)
    -> ArenaLayout { device, exact size, object offsets }
    -> FinalizedBundle { resolved value locations }
    -> init allocation, loop use, exit release
```

There is no allocation, resizing, or replanning in the loop. An arena layout is
an immutable preparation result and is not a runtime estimate.

## 1. Data model and ownership

### 1.1 Logical objects

`ArenaObject` is defined in `core/src/schedule.rs:568-576`:

```text
ArenaObject = {
    id: ArenaObjectId,
    device: DeviceId,
    bytes: ByteCount,
    alignment: ByteCount,
    lifetime: ScheduleWindow,
}
```

It is a logical allocation request. It deliberately has no physical arena
offset. The `device` selects the independent arena in which it must reside.
`bytes` is the object's complete byte extent, including any values grouped into
the object. `alignment` is a power-of-two byte requirement. `lifetime` is the
half-open schedule interval during which the object may be accessed.

`ScheduleWindow` (`core/src/schedule.rs:256-269`) is valid exactly when
`start < end`; two windows overlap exactly when
`left.start < right.end && right.start < left.end`. Equality at one endpoint is
therefore non-overlap. The packer uses this same predicate for memory reuse.

The object is not the same thing as a graph value. A value is represented by a
`ValueSpec` (`core/src/schedule.rs:277-284`) and attached to an object by a
`ValueBinding` (`core/src/schedule.rs:286-296`):

```text
ValueBinding = {
    value: ValueId,
    object: ArenaObjectId,
    object_offset: ByteOffset,
}
```

`object_offset` is relative to the object. It is not selected by
`pack_arenas`; Finalize adds it to the object allocation to obtain the absolute
arena address. This keeps the Draft offset-free while still allowing aliases
and packed init images to use exact subranges.

### 1.2 Physical layout records

The scheduler emits the core records in `core/src/schedule.rs:583-594`:

```text
ArenaAllocation = {
    object: ArenaObjectId,
    offset: ByteOffset,
}

ArenaLayout = {
    device: DeviceId,
    size: ByteCount,
    allocations: Vec<ArenaAllocation>,
}
```

There is one `ArenaLayout` for every topology device, including a device with
no objects. Its `size` is the exact maximum allocation end, not a capacity hint
and not a rounded reserve. `allocations` is sorted by stable object ID in the
returned layout. The layout is the only source used later to request a native
arena buffer.

`ByteCount` and `ByteOffset` are checked u64 wrappers
(`core/src/units.rs:3-55`). `ByteCount::checked_align_up` accepts only a
nonzero power-of-two alignment and reports `UnitError::InvalidAlignment` or
`UnitError::Overflow`. `ByteOffset::checked_end` reports an overflow as
`None`. `pack_arenas` converts every failed checked operation into a
`ScheduleError` with device context.

### 1.3 Capacity input

`CapacityLedger` and `CapacityLedgerEntry` are in
`core/src/plan.rs:134-224`:

```text
CapacityLedgerEntry = {
    device: DeviceId,
    total: Property<ByteCount>,
    runtime_overhead: Property<ByteCount>,
    fragmentation: Property<ByteCount>,
    safety_headroom: Property<ByteCount>,
    recipe_usable: Property<ByteCount>,
}
```

`recipe_usable` is the field consumed by `pack_arenas`. It is the capacity left
for Recipe's arena, staging, and scratch demand after the ledger's reservation,
runtime, fragmentation, and safety accounting. The packer does not recompute
that accounting. Callers validate the ledger first, and `pack_arenas` performs
the final per-layout comparison `layout.size <= recipe_usable.value`.

`CapacityLedger::validate` requires one known, unique, schedulable entry for
every topology device and checks overflow-safe accounting against the device's
total (`core/src/plan.rs:150-224`). A ledger entry absent for a topology device
is still an error at the arena boundary, even when that device has no objects,
because an empty layout is still emitted for the device.

## 2. The planner-side arena contract

The scheduler does not discover values or infer lifetimes. The paired logical
planning implementation is `build_arena_contract` in
`planner/src/planner.rs:3139-3391`. It consumes the values, already scheduled
tasks, init-image regions, alias invocations, loop count, and loop domains. It
returns `(Vec<ArenaObject>, Vec<ValueBinding>)`, which is exactly the input
contract for `pack_arenas` plus the bindings later used by Finalize.

### 2.1 Stable alias groups

`DisjointValues` (`planner/src/planner.rs:3096-3127`) is a deterministic union-
find over all `ValueId`s. For every primitive alias rule whose permission is
`AliasPermission::MustAliasExact`, `build_arena_contract` resolves the one-based
kernel input and output indexes, checks that both values exist in the invocation,
and unions the two values. The smaller root ID is retained, so group formation
does not depend on hash-map order. Index conversion or missing invocation
entries are `PlannerErrorKind::ArithmeticOverflow` or `InvalidDraft`.

The values are then grouped by union-find root. Every member of one exact-alias
group must have the same device and exact byte size. A mismatch is an
`InvalidDraft`, before any physical layout is attempted. `ValueAliasContract`
records the source-level permission separately; the union only applies to the
strict `MustAliasExact` case.

### 2.2 Fixed init-image objects

Before ordinary value groups, each `ImageRegion` gets an object ID from the
planner's `StableIdAllocator` (which starts at one and checks ID-space
overflow). The region becomes an `ObjectBlueprint` with:

```text
device   = image.device
bytes    = image.bytes
alignment = 16 B
members  = image.members
```

The region's `members` map each image value to an offset relative to the image.
`fixed` records `(ArenaObjectId, ByteOffset)` for those values, so an external
input or preallocated fault flag already in the packed image is not moved by
arena packing. The image value itself is a member at offset zero.

`initialize_data_images` (`planner/src/planner.rs:2276-2465`) creates one image
per topology device. It adds external input copies used by the selected kernels,
four-byte int32 fault flags grouped by loop domain, and the image value. The
image byte count is the packed member sum, with a minimum of four bytes. The
init task is an external-to-device transfer in `RunPhase::Init`. Its physical
values have the init task as producer, so their lifetimes begin at that task and
are extended by later readers. A device with no external input still receives a
minimum four-byte image and an init task.

The public `InitDataImage` keeps only the logical external members and their
`image_offset`; fault flags remain internal values. Core validation later checks
that every image member resolves to the same object and that
`image_binding.object_offset + image_offset` equals the physical member's
binding (`core/src/plan.rs:1601-1825`).

### 2.3 Ordinary groups and bindings

For each alias group, the planner gathers any fixed image locations:

* With no fixed location, it allocates a new 16-byte-aligned blueprint sized to
  the group's value size and gives all members object offset zero.
* With exactly one fixed location, every member uses that existing image object
  and that object's fixed offset.
* With more than one fixed location, the group would require two distinct image
  placements and lowering fails with `PlannerErrorKind::InvalidDraft`.

Each member is appended once to the selected blueprint and receives a
`ValueBinding { object, object_offset }`. Bindings are collected from a
`BTreeMap`, so the returned binding vector is sorted by `ValueId`.

This is the logical sharing decision. `pack_arenas` may reuse bytes between
different objects when their lifetimes do not overlap, but it never changes a
group's object identity or a value's object-relative offset.

### 2.4 Lifetime derivation

After object membership is known, `build_arena_contract` computes one lifetime
for each blueprint:

1. It indexes every scheduled task by `TaskId` and computes the complete
   schedule window as the minimum task start and maximum task end.
2. It computes a loop-only window when execution is unbounded or has more than
   one finite iteration. A loop with one iteration does not receive this
   extension.
3. For every member value, it extends the object's start and end over the
   producer task, if any, and every task that references the value. References
   include calculation inputs, outputs, fault flags, transfer endpoints, and
   metric reads (`task_references` at `planner/src/planner.rs:3491-3535`).
4. If any reader is in a different phase from its producer,
   `value_crosses_phase_boundary` extends the object to the entire schedule
   window. A read value without a scheduled producer is an `InvalidDraft`.
5. If the value stays within the loop but can cross an iteration boundary,
   `value_crosses_iteration_boundary` extends it over the entire loop window.
   A loop reader must have a declared iteration domain. A producer refreshes a
   value before every reader only when it is a loop producer ending no later than
   the reader, has a declared domain that covers the reader domain, and the
   domain arithmetic proves the refresh reaches every read. Otherwise the value
   is live across iterations. Finite domains with no nonzero iteration
   activation are ignored.

The resulting `ScheduleWindow { start, end }` is required to have both bounds.
An object with no live producer or reference, a producer task missing from the
task index, a missing loop domain, or only one lifetime boundary fails lowering
with `PlannerErrorKind::InvalidDraft`. Objects are finally sorted by
`ArenaObjectId`.

The lifetime is therefore conservative with respect to the complete static
schedule. It expresses when a byte range may be read or written, not when a
native allocation happens. The allocator can safely share an offset only when
the half-open intervals prove that the two objects are never simultaneously
live.

## 3. `pack_arenas` algorithm

The complete implementation is `scheduler/src/arena.rs:13-152`. It performs no
topology probing, task scheduling, value binding, or native allocation.

### 3.1 Input validation and partitioning

The first pass walks the caller-provided objects and partitions references by
`DeviceId` in a `BTreeMap`:

1. If `topology.device(object.device)` is absent, it returns
   `ScheduleErrorKind::InvalidTopology`, with the object ID in the message and
   the object device in the error context.
2. An alignment of zero or a non-power-of-two alignment returns
   `ScheduleErrorKind::InvalidTransfer`, with device context.
3. An empty lifetime (`start >= end`) returns `InvalidTransfer`, also with
   device context.
4. Valid objects are stored by reference. The function does not mutate the
   objects, topology, or capacity ledger.

The packer itself does not call `Topology::validate`,
`Topology::validate_scheduling_properties`, or `CapacityLedger::validate`. The
planner, preparation, and finalization boundaries perform those broader checks.
It also does not reject duplicate `ArenaObjectId`s or zero-byte objects.
Duplicate IDs are caught by Draft/finalization validation. A zero-byte object is
accepted by this path and is not separately rejected by the current validators.

Next, it copies all topology device IDs, sorts them, and processes them in
ascending stable order. Every topology device gets one output layout. An
unknown object device was rejected already, so no unconsumed partition can be
validly left behind.

### 3.2 Stable placement order

For each device, the partition is sorted by `(lifetime.start, object.id)`.
Objects starting at the same time are therefore considered by ascending stable
ID. The allocator maintains:

```text
placed: Vec<(&ArenaObject, ByteOffset)>
```

The list is local to one device. Objects on different devices can never share
an offset because they are allocated in different physical arenas.

### 3.3 Candidate offsets and first-fit choice

For the next object, the candidate byte positions begin with zero. For every
previously placed object whose lifetime overlaps the next object's lifetime,
the allocator computes that object's checked end
`other_offset + other.bytes` and adds the end to the candidate list. Candidate
positions are sorted and deduplicated.

Each candidate is processed in order:

1. `ByteCount::checked_align_up(candidate, object.alignment)` chooses the
   lowest aligned address at or after the candidate. An alignment error is
   reported as `ScheduleErrorKind::ArithmeticOverflow` with the unit error text.
2. `ByteOffset::checked_end(offset, object.bytes)` checks the new object's end.
   Overflow is `ArithmeticOverflow` with `arena allocation end overflowed`.
3. The proposed half-open byte range is compared with every already placed
   object whose lifetime overlaps. A collision exists exactly when
   `new_start < old_end && old_start < new_end`. If any collision exists, the
   candidate is rejected.
4. The first candidate with no collision is selected.

If no candidate is selected, the function returns `ArithmeticOverflow` with
`no representable arena offset remains`. In ordinary u64 arithmetic an earlier
checked operation normally reports the more specific overflow first; the final
branch remains the implementation's explicit failure for an exhausted candidate
set.

The result is a deterministic aligned first-fit placement. The candidate list
contains zero and the ends of all currently overlapping ranges, so a free gap
created by a non-overlapping lifetime can be reused. The collision check is
still authoritative: alignment may move a candidate into a live range, in
which case the allocator advances to the next candidate.

When placement succeeds, the object and offset are appended to `placed`. The
algorithm never moves a previously placed object, and it never searches a
different device.

### 3.4 Size, output order, and capacity

After all objects for a device are placed, `placed` is sorted by object ID. The
allocator then folds each checked object end into `size = max(size, end)` and
emits `ArenaAllocation { object, offset }` in that ID order. A final checked-end
overflow is `ArithmeticOverflow`.

The function looks up `capacity.entry(device)`. A missing entry returns
`ScheduleErrorKind::InsufficientCapacity` with the message
`device has no realized capacity ledger entry`, even when `size` is zero. If
`size > usable.recipe_usable.value`, it returns `InsufficientCapacity` with the
planned and usable byte counts. Otherwise it appends:

```text
ArenaLayout {
    device,
    size,
    allocations,
}
```

The layouts are already in ascending device order, and the function returns
`Ok(layouts)` after all topology devices have passed the capacity check.

## 4. Determinism and memory-safety invariants

The scheduler and core validators jointly enforce these properties:

| Invariant | Enforced by | Consequence |
| --- | --- | --- |
| Every object names a topology device | `pack_arenas`, Draft validation | No allocation targets an unknown storage domain. |
| Every object lifetime is nonempty | `pack_arenas`, Draft validation | Reuse decisions have a real half-open interval. |
| Every alignment is a nonzero power of two | `pack_arenas`, Draft validation | `checked_align_up` and finalized offsets are well-defined. |
| A live pair has disjoint byte ranges | placement collision check and `validate_layouts` | Distinct simultaneously live values never alias. |
| A dead pair may share bytes | lifetime overlap predicate | Arena size reflects peak live demand, not the sum of all objects. |
| Every object is allocated once on its own device | `FinalizedBundle::validate_layouts` | Bindings resolve to one physical arena. |
| Layout size equals the maximum allocation end | `validate_layouts` | The backend receives the exact required range. |
| Layout size fits `recipe_usable` | `pack_arenas` and `validate_layouts` | Realized Recipe capacity is not exceeded. |
| Devices and allocations are stable ordered records | BTree maps and explicit sorts | Candidate and bundle identities are reproducible. |
| Arithmetic uses checked additions | units, packer, finalization | Overflow fails closed instead of wrapping an address. |

`FinalizedBundle::validate_layouts` (`core/src/plan.rs:2632-2773`) repeats the
critical checks on the supplied layouts. It rejects unknown or duplicate layout
devices, duplicate or unknown object allocations, wrong-device objects,
misaligned offsets, out-of-bounds ends, a `size` larger than
`recipe_usable`, and any pair whose byte ranges and lifetimes overlap. It also
requires one layout for every topology device and one allocation for every
Draft object. This second check is intentional: the scheduler creates the
layout, while Finalize is the immutable-bundle trust boundary.

`resolve_value_locations` (`core/src/plan.rs:2775-2866`) then computes each
absolute value address as:

```text
arena_offset = allocation(object).offset + binding.object_offset
```

It checks the addition, value end, device identity, arena bounds, and dtype
alignment before retaining a `ResolvedValueLocation`. A value cannot reach an
executor with an unchecked or partial location.

## 5. Call graph and phase boundaries

### 5.1 Planner candidate construction

`plan_program_candidates` (`planner/src/planner.rs:222-344`) validates the
program, graph, topology, discovery, reservations, and planning capacity. Its
`lower_candidate` path:

1. creates one `LoweringState` and initializes one logical data image per
   topology device;
2. lowers calculation and transfer tasks;
3. adds aliases, fault readbacks, external outputs, and phase barriers;
4. calls `recipe_scheduler::schedule`, which assigns the final task windows;
5. compacts submission resources and computes peak staging/scratch demand;
6. calls `build_arena_contract` to produce logical objects and bindings;
7. calls `pack_arenas(context.topology, &arena_objects, context.capacity)`;
8. calls `require_total_capacity` with `arena_layout.size + auxiliary_peak`;
9. validates the resulting `DraftPlan`; and
10. returns `PlannedCandidate { draft, arena_layouts, ... }`.

The candidate's `arena_layouts` are therefore an optimistic planning result.
They prove that the Draft fits the planning ledger, but they do not authorize a
native allocation and are not retained if realization changes usable capacity.
`require_total_capacity` (`planner/src/planner.rs:3537-3571`) is a separate
planner check because `pack_arenas` knows only arena objects, not staging or
scratch peaks.

`plan_candidates` is a convenience wrapper. It creates a one-iteration static
program and delegates to `plan_program_candidates`; it does not implement a
second arena path.

### 5.2 Preparation and the final repack

`Preparer::prepare_program_validated` (`prepare/src/lib.rs:340-511`) performs
the fixed-point boundary:

1. The realizer supplies and validates reservations.
2. `optimistic_planning_capacity` derives a planning ledger from measured total
   capacity minus the exact reservation, with runtime, fragmentation, and
   safety fields initially zero (`prepare/src/lib.rs:558-605`).
3. The planner emits ranked candidates, each with a Draft and provisional
   layouts.
4. The realizer creates and warms exactly one candidate and returns capacity
   snapshots.
5. `validate_observation` requires the configured number of snapshots, validates
   every ledger, and requires the stable tail to equal the final snapshot.
6. The unchanged Draft is passed to `pack_arenas` again, now with the final
   post-warm `CapacityLedger` (`prepare/src/lib.rs:418-450`).
7. A packing or final-capacity error destroys the candidate session, records a
   `CandidateRejectionStage::FinalCapacity` rejection, and advances to the next
   planner candidate.
8. The successful layouts, realization profile, Draft, and loop schedule are
   passed to `FinalizedBundle::finalize_with_loop_schedule`.

This is the only normal reason the same logical object contract is packed twice:
the first call enumerates candidates under an upper bound, while the second
call uses measured capacity after opaque native realization and warmup. The
Draft's values, tasks, lifetimes, aliases, and object IDs are not changed
between calls. If finalization fails after a successful repack, preparation
destroys the retained session and returns `PrepareErrorKind::Finalization`; it
does not silently alter the layout or retry through another allocator.

### 5.3 Finalization and address resolution

`FinalizedBundle::finalize_with_loop_schedule`
(`core/src/plan.rs:2455-2517`) validates the Draft and realization identities,
loop domains, layouts, and resolved value locations. On success it stores the
layouts and resolved locations in the immutable bundle. The public accessors
`arena_layouts`, `value_locations`, `value_location`, and `init_image` expose
read-only views. `transfer_endpoints` resolves device endpoints through those
same locations and verifies that the resolved device still matches the endpoint.

The bundle's `ArenaLayout` is thus the handoff from planning to execution. No
executor function is allowed to infer a new offset from a value, task, or
capacity field.

### 5.4 Runtime allocation and lifecycle

The executor consumes finalized layouts through the typed lifecycle in
`executor/src/executor.rs`:

* `PreparedRun` has realized backend resources and phase plans, but its arena
  map is empty (`executor/src/executor.rs:773-781`, `959-977`).
* `PreparedRun::initialize` validates the supplied images, iterates the final
  bundle's layouts, and calls `Backend::allocate_arena` once per device before
  running the init phase (`executor/src/executor.rs:980-1035`). It records the
  exact layout size in the logical journal. The init transfer admits one packed
  image per device into the already allocated arena.
* `InitializedRun` and the loop states use resolved value locations and retain
  the same arena map. The loop cannot allocate, resize, or replan.
* `ExitedLoop::exit` runs the exit phase, then `teardown_resources` drains the
  arena map and calls `Backend::release_arena` once per device before destroying
  other resources (`executor/src/executor.rs:1321-1371`, `1489-1521`).

The backend boundary (`executor/src/backend.rs:357-443`) receives the exact
`ArenaLayout` for allocation and the owned arena handle for release. The local
backend dispatches by device class (`native-executor/src/local.rs:1753-1785`);
CUDA and HSA allocate buffers from `layout.size`
(`native-executor/src/cuda.rs:423-435`, `native-executor/src/hsa.rs:464-480`).
The scheduler never chooses a CUDA, HSA, host, or disk representation.

The lifecycle matches system contract C6 and C9
(`system-contract.md:138-175`): one logical image admission during `init`, no
external ingress or egress in the loop, and one complete arena release during
`exit`. Physical DMA calls, staging, queue tokens, and backend cleanup are
separate runtime evidence and do not create additional arena layouts.

## 6. Errors and their translation

`ScheduleError` (`scheduler/src/error.rs:5-66`) carries a
`ScheduleErrorKind`, optional task/device context, and a message. Arena packing
uses these variants:

| Condition | Kind | Detail |
| --- | --- | --- |
| Object device is absent from topology | `InvalidTopology` | `arena object {id} references an unknown device` |
| Zero or non-power-of-two object alignment | `InvalidTransfer` | `arena object {id} has invalid alignment` |
| Empty object lifetime | `InvalidTransfer` | `arena object {id} has an empty lifetime` |
| Checked end, alignment, or final size overflows | `ArithmeticOverflow` | Unit error or `arena allocation end overflowed` |
| No candidate offset can be selected | `ArithmeticOverflow` | `no representable arena offset remains` |
| Capacity entry is missing | `InsufficientCapacity` | `device has no realized capacity ledger entry` |
| Planned size exceeds `recipe_usable` | `InsufficientCapacity` | Planned and usable byte counts are reported |

The packer attaches the affected device to every object/layout error. It does
not attach a task because no task is being scheduled at this stage.

Planner candidate lowering maps `ArithmeticOverflow` and
`InsufficientCapacity` from the scheduler to `PlannerErrorKind::CandidateInfeasible`
when the error comes through `schedule`, and maps a direct `pack_arenas` error
to `PlannerErrorKind::InvalidCapacity` (`planner/src/planner.rs:1059-1071,
1091-1101`). The separate auxiliary-capacity check uses
`CandidateInfeasible` for checked addition overflow and `InvalidCapacity` for a
failed usable-capacity comparison.

Preparation treats a post-realization packing failure as a retryable
`FinalCapacity` rejection after deterministic teardown. Finalize's independent
layout validator can still report `ValidationCode` errors for malformed or
inconsistent records. Those errors are fatal finalization failures, not a
request to invent a different offset policy.

## 7. End-to-end role

For one accepted candidate, the complete arena path is:

```text
selected placements and lowered values
  -> scheduled Task windows
  -> build_arena_contract
       exact alias groups, fixed image members, bindings, conservative lifetimes
  -> pack_arenas (planning capacity)
       deterministic per-device offsets and exact layout sizes
  -> native realization and stable capacity snapshots
  -> pack_arenas (final capacity)
       same objects and lifetimes, rechecked against measured Recipe capacity
  -> FinalizedBundle::validate_layouts
       one layout and allocation per topology device, no live range overlap
  -> resolve_value_locations
       object offset + binding offset, checked absolute addresses
  -> PreparedRun::initialize
       allocate one backend arena per layout, admit one init image per device
  -> loop
       calculations and transfers use only finalized locations and arenas
  -> exit
       external egress completes, then each arena is released exactly once
```

The scheduler's arena module is intentionally narrow. It provides deterministic
physical placement from already measured topology, already scheduled lifetimes,
and an already accounted capacity ledger. Planner aliasing and lifetime
derivation, preparation's fixed-point capacity observation, core finalization,
and executor lifecycle enforcement surround it. Together those boundaries turn
logical values into safe, stable physical storage without making allocation a
model operation or allowing runtime replanning.
