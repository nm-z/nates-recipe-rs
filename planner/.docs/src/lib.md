# `recipe-planner` crate facade

The package is `recipe-planner` version `0.1.0` (library target
`recipe_planner`, Rust edition 2024, MIT license). Its dependencies are the
backend-neutral Recipe contracts (`recipe-core`, `recipe-language`,
`recipe-primitives`, `recipe-program`, and `recipe-scheduler`) plus `sha2` for
stable identities. The package has no feature flags, binary target, build
script, or optional dependency.

`planner/src/lib.rs` is the public boundary between a validated calculation
program and immutable, offset-free execution Draft candidates. The crate
enumerates every legal kernel-to-GPU placement, lowers each assignment into
the complete `init -> loop -> exit` task graph, asks the measured scheduler for
the exact static timing, packs logical arena objects for feasibility evidence,
and returns candidates ordered by measured makespan. It does not probe
hardware, compile or load a native artifact, allocate a runtime resource,
perform a data transfer, run a task, or finalize physical arena addresses.

The crate-level attributes are `forbid(unsafe_code)` and
`deny(missing_debug_implementations)`. All implementation modules are private,
there is no global state, and planning is synchronous. Inputs are borrowed and
never mutated. A successful call returns all candidate state in owned values;
there is no hidden background work or later planner callback.

## Public surface

The root re-exports the following names:

| Item | Role | Main source |
| --- | --- | --- |
| `plan_candidates` | Graph convenience entrypoint that creates a one-iteration static program and delegates to `plan_program_candidates` | `planner.rs` |
| `plan_program_candidates` | Validate, enumerate, lower, schedule, rank, and return program candidates | `planner.rs` |
| `PlannerSearch` | One-shot ranked stream for the graph convenience entrypoint | `planner.rs` |
| `ProgramPlannerSearch` | One-shot ranked stream that retains loop iteration metadata | `planner.rs` |
| `PlannedCandidate` | Immutable candidate Draft plus provisional layouts, timing, placements, programs, and copy/egress evidence | `planner.rs` |
| `PlannedProgramCandidate` | A `PlannedCandidate` paired with the program's loop count and every loop task domain | `planner.rs` |
| `KernelPlacement` | Source-kernel to selected calculation device mapping | `planner.rs` |
| `StagePlacement` | Lowered stage to kernel template, device, and artifact mapping | `planner.rs` |
| `LogicalValueCopy` | Logical tensor to one physical resident copy on one device | `planner.rs` |
| `PlannedExternalOutput` | Exact physical value and exit task selected for one external output | `planner.rs` |
| `PlannerError` and `PlannerResult<T>` | Structured fail-closed error and `Result` alias | `error.rs` |
| `PlannerErrorKind` | Non-exhaustive machine-readable failure vocabulary | `error.rs` |

The public records expose fields deliberately. Their values are the evidence
consumed by `recipe-prepare` and the native executors, while the mutable
construction state, route trials, alias invocations, and hashing helpers stay
private. There is no public `planner` module path to bypass the root facade.

### Placement and copy records

`KernelPlacement` is `Clone + Copy + Debug + PartialEq + Eq + PartialOrd + Ord`
and contains:

```text
kernel: KernelTemplateId
device: DeviceId
```

It records one source kernel from the topological graph and the GPU selected
for every invocation of that kernel. A placement always names a topology
device that appears in measured discovery with a calculation capability.

`StagePlacement` has the same derives and identifies one lowered stage:

```text
source_kernel: KernelTemplateId
stage_ordinal: u32
kernel_template: KernelTemplateId
device: DeviceId
artifact: ArtifactId
```

The `kernel_template` and `artifact` IDs are derived from the lowered program
digest and stage ordinal. They are not arbitrary indexes and cannot be reused
for a different source stage without an identity collision.

`LogicalValueCopy` records the resident copy inventory:

```text
logical: ValueId
device: DeviceId
physical: ValueId
```

There is at most one entry for a logical value on a given device. A direct
producer or init image has no transfer domain; a copy produced by a loop
transfer is tied to the iteration domain that refreshes it. This distinction
prevents a transferred copy for one domain from being silently reused by a
consumer with a different domain.

`PlannedExternalOutput` is also `Clone + Copy + Debug + PartialEq + Eq +
PartialOrd + Ord`:

```text
task: TaskId
logical: ValueId
device: DeviceId
physical: ValueId
```

It is the exact declaration-to-egress association chosen during lowering. The
planner records the physical value selected by the exit task rather than
asking a later consumer to reverse-match an external tensor against all
resident copies. This is especially important when aliasing leaves more than
one physically valid value.

### `PlannedCandidate`

`PlannedCandidate` is `Clone + Debug + PartialEq + Eq` and contains:

```text
draft: DraftPlan
arena_layouts: Vec<ArenaLayout>
makespan: Nanoseconds
placements: Vec<KernelPlacement>
stage_placements: Vec<StagePlacement>
lowered_programs: Vec<LoweredProgram>
value_copies: Vec<LogicalValueCopy>
external_outputs: Vec<PlannedExternalOutput>
```

`draft` is the authoritative offset-free contract. It contains validated
values, scalar kernel templates, chosen realized artifacts or target-
independent deferred build recipes, scheduled tasks, resource manifests,
logical arena objects and bindings, alias contracts, init images, and one
release record per topology device. The planner hashes and validates this
Draft before returning it.

`arena_layouts` is the deterministic result of the planner's optimistic
`pack_arenas` call. It is feasibility evidence for the supplied capacity
ledger, not a permission to allocate or a replacement for post-realization
final packing. `recipe-prepare` repacks the unchanged Draft against the
capacity observed after native realization and warm stabilization.

`makespan` is the scheduler's static completion time for the candidate. It
includes init admission, every loop task that is statically present, and exit
egress according to phase barriers. It is the primary ranking key.

`placements`, `stage_placements`, and `lowered_programs` preserve the lowering
evidence needed by artifact realization and backend checks. `value_copies` is
the complete logical-to-physical inventory, while `external_outputs` is only
the final egress selection.

### Program and graph search

`PlannedProgramCandidate` is `Clone + Debug + PartialEq + Eq`:

```text
planned: PlannedCandidate
loop_iterations: LoopIterations
loop_domains: Vec<LoopTaskDomain>
```

The loop fields retain the program's finite or unbounded lifecycle and the
exact domain assigned to every scheduled loop task. The planner does not
unroll an unbounded program, duplicate stage artifacts for iterations, or
invent a terminal iteration.

`PlannerSearch` and `ProgramPlannerSearch` are `Clone + Debug`. They own a
ranked vector, a cursor, an issued identity set, and a rejected identity set.
Their public methods are:

```text
ranked_candidates(&self) -> &[PlannedCandidate]
next_candidate(&mut self) -> Option<&PlannedCandidate>
reject(&mut self, identity: CandidateIdentity) -> PlannerResult<()>

ranked_candidates(&self) -> &[PlannedProgramCandidate]
next_candidate(&mut self) -> Option<&PlannedProgramCandidate>
reject(&mut self, identity: CandidateIdentity) -> PlannerResult<()>
```

`ranked_candidates` is a read-only view of the complete original ranking,
including candidates that have subsequently been rejected. `next_candidate`
advances the cursor before returning, records the candidate identity as
issued, skips identities already in the rejected set, and never returns an
identity twice. The returned reference is borrowed from the search; callers
that need to retain a candidate while advancing the stream must clone it.
`reject` succeeds only for an identity previously returned by
`next_candidate`, and only once. It does not rewind the cursor or alter the
order. Calling it for an unissued identity returns `UnknownCandidate`; calling
it twice returns `AlreadyRejected`.

`plan_candidates` converts a `CalculationGraph` into
`StaticCalculationProgram::every_iteration(graph.clone(), LoopIterations::ONE)`.
It delegates all validation and lowering to `plan_program_candidates`, then
projects away loop metadata and returns a fresh `PlannerSearch`. Use this
entrypoint only for the legacy one-iteration graph shape. Programs with
explicit domains, finite horizons, unbounded horizons, or user metrics must
use `plan_program_candidates`.

`plan_program_candidates` accepts:

```text
plan_program_candidates(
    program: &StaticCalculationProgram,
    topology: &Topology,
    discovery: &DiscoveryProfile,
    artifacts: &[ArtifactIdentity],
    reservations: &ReservationLedger,
    capacity: &CapacityLedger,
) -> PlannerResult<ProgramPlannerSearch>
```

The artifact slice is an exact catalog of already realized identities known to
the caller. An artifact absent from that slice is represented in the Draft as
an `ArtifactBuildRecipe`; the planner does not compile the missing image.
`reservations` and `capacity` are validated inputs used for optimistic arena
feasibility. They are not mutated and do not become runtime allocations.

### Errors

`PlannerError` is `Clone + Debug + PartialEq + Eq` with two public fields:

```text
kind: PlannerErrorKind
message: String
```

`PlannerError::new(kind, message)` is the only constructor. `Display` formats
the kind followed by the message, and the type implements `std::error::Error`.
`PlannerResult<T>` is `Result<T, PlannerError>`.

`PlannerErrorKind` is `Copy + Debug + PartialEq + Eq`, marked
`#[non_exhaustive]`, and currently declares:

```text
InvalidGraph          InvalidTopology       InvalidDiscovery
InvalidReservation    InvalidCapacity       InvalidArtifact
InvalidDraft          MissingArtifact       NoCalculationDevice
NoRoute               DependencyConflict    CandidateInfeasible
NoViableCandidate     Schedule              ArithmeticOverflow
IdentityCollision     UnknownCandidate      AlreadyRejected
```

Downstream matches must include a wildcard because the vocabulary can grow.
`MissingArtifact` is declared in the public vocabulary but is not constructed
by the current implementation path; a missing catalog entry normally becomes
a deferred build recipe. The other variants have direct construction sites
described below.

## Module map

`lib.rs` declares all modules privately and re-exports the root surface:

```text
mod error;
mod hash;
mod planner;
```

* `error.rs` owns `PlannerErrorKind`, `PlannerError`, and
  `PlannerResult<T>`. It has no planner state or side effects.
* `hash.rs` owns the crate-private `StableDigest`, a SHA-256 stream with a
  domain prefix, length-delimited byte fields, typed integer encodings, and a
  final `recipe_core::Digest`. No hash helper is public.
* `planner.rs` owns every public result/search type and the complete
  validation, assignment, lowering, routing, scheduling, resource, arena,
  and identity pipeline. Its private state is single-candidate state and is
  discarded after that candidate is built.

The principal `planner.rs` regions are:

| Source region | Responsibility |
| --- | --- |
| `1-185` | Imports, public records, search state, private assignment records |
| `187-350` | Public entrypoints, input validation, assignment enumeration, ranking |
| `352-790` | Primitive lowering, common hardware, stage/catalog validation, graph and candidate identities |
| `792-978` | Runtime copy, transfer, image, alias, artifact, stable-ID, and lowering-state records |
| `979-1214` | One candidate's complete lowering, scheduling, packing, Draft hashing and validation |
| `1216-1778` | User metrics, stage invocations, fault readbacks, buffer materialization, artifact selection |
| `1779-2059` | Alias contracts/dependencies, phase barriers, trial scheduling |
| `2060-2752` | Directed routes, physical transfer chains, init images, copy selection, external egress |
| `2753-3097` | Submission compaction, queue ownership, staging and scratch peak accounting |
| `3097-3572` | Alias groups, arena lifetimes, phase and iteration boundaries, total capacity |
| `3574-3835` | Canonical Draft and build-contract hashing |

## Planner boundary and ownership

The planner starts with a complete static declaration and measured evidence.
It requires the graph or program to be valid, the topology and scheduling
properties to be valid, discovery to cover that exact topology, reservations
to cover every topology device, capacity to agree with those reservations,
and each supplied artifact identity to be unique and valid. It never fills in
missing measurements with theoretical defaults.

The resulting Draft is the handoff between static planning and preparation:

```text
CalculationGraph / StaticCalculationProgram
        + Topology + DiscoveryProfile
        + ArtifactIdentity[] + ReservationLedger + CapacityLedger
                              |
                              v
                recipe-planner candidate search
                              |
                              v
      PlannedProgramCandidate { DraftPlan, timing, evidence }
                              |
                              v
               recipe-prepare realization loop
       (realize, warm, observe capacity, reject or Finalize)
                              |
                              v
        immutable FinalizedBundle -> native executors
```

`recipe-scheduler` is the planner's timing and arena callee. The planner gives
it one `UnscheduledTask` per physical calculation stage, metric, init/exit
transfer, and internal transfer hop. The scheduler validates the same
topology and discovery, selects measured start times and transfer lanes, and
returns `Task` windows plus a makespan. The planner then calls
`pack_arenas` over logical `ArenaObject` lifetimes. Neither scheduler result
executes work nor allocates a backend object.

`recipe-primitives::lower` is the calculation lowering callee. It turns each
source kernel into a validated `LoweredProgram` containing typed buffers,
ordered stages, dependencies, static views, dispatch geometry, resource
bounds, fault flags, and source alias rules. The planner owns the conversion
from that program to core `CalculationTask`, `ArtifactBuildRecipe`, and
`KernelTemplate` records.

`recipe-language` supplies the acyclic `CalculationGraph` and tensor metadata.
`recipe-program` supplies the explicit loop horizon, per-kernel
`IterationDomain`, and `MetricEmission` declarations. `recipe-core` supplies
all identities, typed IDs, task records, validation, Draft, arena, capacity,
artifact, topology, discovery, and reservation contracts.

## Construction flow

The implementation is a straight, fail-closed pipeline. Every stage below
must complete before the next stage can publish a result.

### 1. Validate declarations and index the graph

`plan_program_candidates` first calls `program.validate`, then validates the
embedded graph, topology, topology scheduling properties, discovery,
reservations, and capacity. All validation failures are wrapped with the
corresponding `PlannerErrorKind` and stop the call before candidate state is
created.

The graph is topologically ordered once. Tensors are indexed by `ValueId` and
calculation nodes by source `KernelTemplateId`. The program's domains are
indexed by kernel and its metrics are retained in their canonical order. A
missing source kernel, tensor, or domain later becomes `InvalidGraph` rather
than an implicit default.

### 2. Lower source kernels and derive stage identities

`lower_programs` computes one common `LoweringHardware` from available
calculation capabilities. Subgroup width is the maximum reported width,
while maximum workgroup lanes and shared memory are the minimum supported
across available calculation devices. If no available calculation capability
exists, or the common width cannot form a full subgroup workgroup, planning
returns `InvalidDiscovery`.

Every graph node goes through `recipe_primitives::lower` and its returned
program is validated. Each nonempty lowered stage receives a
`KernelTemplateId` from a domain-separated digest of the lowered program,
source kernel, and stage ordinal. Scalar-map stages also carry a cloned core
`KernelTemplate` with that identity. Repeated identities for different source
stages are `IdentityCollision`; an invalid lowered program is `InvalidGraph`.

This stage does not duplicate artifacts per loop iteration. One lowered stage
and one artifact/build identity serve every activation of that stage.

### 3. Validate the catalog and enumerate legal assignments

`validate_artifact_catalog` rejects zero or duplicate artifact IDs and any
artifact failing its core validator. `legal_choices` scans topology devices in
stable order and keeps only `DeviceKind::GpuMemory` devices whose discovery
entry has a calculation capability. Every graph kernel receives the same
sorted GPU option list. An empty graph yields no assignment dimensions; a
nonempty graph with no such device returns `NoCalculationDevice`.

`enumerate_assignments` recursively visits the Cartesian product, in sorted
device order, without pruning a placement merely because it looks slower.
The graph identity is a domain-separated digest of tensor IDs, dtypes,
storage, external flags, shapes and layouts, topological kernels and I/O,
work, iteration domains and horizon, lowered program and stage identities,
and metric declarations. A candidate identity adds topology and discovery
identities plus every kernel-to-device pair. A collision between two distinct
assignments is fatal `IdentityCollision`.

### 4. Build one candidate's init and loop state

`lower_candidate` creates a fresh `LoweringState`. Its task and value
allocators start at one and advance with checked arithmetic. Each call to
`next_submission` gives the task a queue and completion ID equal to the task
ID, records the owning device in the provisional resource manifest, and
returns the submission slots used by the task.

`initialize_data_images` then creates one init transfer per topology device.
The image contains all external input tensors needed on that device, and a
pass-through external input/output is admitted on the first stable device if
no kernel consumes it. Every program fault buffer for a device and iteration
domain gets one four-byte i32 member in the same image. The image value and
its members become init-produced `ValueSpec` records and direct resident
copies. No host data bytes are copied by the planner; the init task only
describes the eventual external admission.

For each source kernel in topological order, `lower_program_invocation`:

1. allocates one loop calculation task and submission record per lowered
   stage;
2. materializes input tensors through `ensure_copy`, output tensors as fresh
   device values, scratch buffers as fresh values, and fault buffers from the
   device's init image;
3. adds prior-stage barriers and producer dependencies for every read binding;
4. creates and validates one `ArtifactBuildRecipe` per stage, selecting an
   exactly matching catalog artifact when present or retaining the deferred
   recipe otherwise;
5. emits a core `CalculationTask` with typed input/output values, work bounds,
   fault flag, stage template, artifact ID, device, and loop domain;
6. records writers and resident output copies for each source output; and
7. converts source alias rules to core argument positions and records the
   invocation for later alias dependency and arena construction.

A catalog artifact is accepted only when its template, build provenance,
resource envelope, and measured target exactly match the deferred stage. A
colliding but mismatched catalog entry is `InvalidArtifact`, not a fallback to
another image. A nonempty output without a dispatch writer, an unrelated
tensor buffer, a missing fault image member, or a stage that reads before a
producer is `InvalidDraft`.

### 5. Resolve copies and physical transfer routes

`ensure_copy` returns an existing direct or same-domain resident copy when one
is valid. Otherwise it enumerates every simple directed route from every
compatible source copy to the destination device. The empty route represents
a required same-device copy. A nonempty route is decomposed by
`build_transfer_chain` into one `TransferTask` per directed link, one
intermediate `ValueSpec` per internal endpoint, a dependency from each hop to
the preceding hop, and a submission owned by the hop's source device. The
final destination value is allocated at the allocator head; intermediates
follow it in route order.

Each candidate chain is trial-scheduled with the complete existing task list.
Measured bandwidth, lane limits, duplex conflicts, source readiness, phase
ordering, overlap properties, and makespan therefore participate in route
selection. The deterministic key is earliest completion of the final hop,
then trial makespan, source device, source physical value, and route order. A
route that cannot be scheduled is discarded for that assignment. When no
directed route exists, the result is `NoRoute`; when routes exist but none are
schedulable, the result is `CandidateInfeasible`.

The final Draft never contains a composite multi-link transfer. Every
internal transfer task has one directed link or an explicitly empty
same-device route, matching the scheduler and executor contract. A transferred
copy is tagged with its consumer domain. Reusing it for a different domain is
`CandidateInfeasible`, which forces distinct resident values instead of
silently changing loop semantics.

### 6. Add metrics, fault readbacks, aliases, and egress

After all kernel invocations, `add_fault_readbacks` creates one four-byte
`MetricTask` with `MetricPurpose::FaultReadback` for each `(device, domain)`
fault cohort. It depends on every calculation sharing that cohort's flag and
is assigned the same loop domain. User metrics are emitted by
`add_user_metrics` only from the producer-resident copy on the producer
device. They depend on that value's producer and all known fault readbacks,
use a preallocated metric slot, and retain the declared metric domain. A
transferred copy cannot satisfy a user metric.

`add_alias_dependencies` turns `MustAliasExact` rules into overwrite barriers.
Consumers of the old input must finish before the aliasing invocation can
overwrite it; an exit consumer that would still need the old value is a
`DependencyConflict`. The same rules are converted to core
`ValueAliasContract` records and unioned into arena objects. The planner
validates exact dtype and static view equality for source must-alias rules.

`add_external_outputs` creates one exit transfer per external-output tensor.
It considers only direct, non-transferred resident copies that are not inputs
overwritten by a must-alias rule. Each possible egress is trial-scheduled and
ranked by makespan, completion time, source device, and physical value. A
safe copy with no schedulable exit is `CandidateInfeasible`; no safe copy is a
`DependencyConflict`. The selected task and physical value are retained in
`PlannedExternalOutput`.

Finally, `add_phase_barriers` adds every init task as a dependency of every
loop task, and every init and loop task as a dependency of every exit task.
The scheduler consequently sees explicit lifecycle ordering rather than
inferring it from task insertion order.

### 7. Schedule and compact resources

The complete `UnscheduledTask` list is passed to `recipe_scheduler::schedule`.
Scheduler dependency cycles and invalid lifecycle dependencies become
`DependencyConflict`; arithmetic or insufficient scheduling capacity becomes
`CandidateInfeasible`; a missing route becomes `NoRoute`; other scheduler
failures become `Schedule`. The returned static windows and makespan are the
only timing evidence used for ranking.

`compact_submission_resources` replaces provisional one-task queue and
completion IDs with a deterministic interval coloring. Local device work,
metrics, admissions, egress, and same-device copies share a device-local
owner class. Cross-device hops are colored separately by exact source and
destination pair so one physical queue is not materialized in two backend
owners. The measured `maximum_submission_queues` limit is enforced per
device. Exceeding it is `CandidateInfeasible`; a missing limit is
`InvalidDiscovery`.

`finalize_auxiliary_resources` computes peak pinned staging for external
admission/egress and peak scratch for calculation tasks from the scheduled
windows. Artifact identities and deferred build recipes must resolve exactly
once for each calculation task. Arithmetic overflow while computing these
peaks is treated as candidate infeasibility.

### 8. Derive arena lifetimes and capacity evidence

`build_arena_contract` unions values connected by exact must-alias contracts,
fixes init-image members at their image offsets, and creates one
16-byte-aligned `ArenaObject` per remaining disjoint value group. It binds each
logical value to its object and offset, then derives lifetime from the
producer, every task reference, phase crossings, and loop-domain crossings.

An object that crosses `init`, `loop`, or `exit` is kept live for the complete
static schedule. A loop-only object that is not refreshed before every reader
activation is kept live across the loop window. Domain coverage is checked
explicitly, including first iteration, stride, finite end, and unbounded end.
Missing producers, missing loop domains, conflicting fixed image locations,
different device or byte sizes inside an alias group, and empty lifetimes are
`InvalidDraft`.

The resulting objects are passed to `pack_arenas`. Packing reuses bytes only
for non-overlapping lifetimes and returns one deterministic layout per device.
The planner then adds the auxiliary staging and scratch peaks to each layout
size and compares the total to `CapacityLedgerEntry::recipe_usable`. A
missing capacity entry is `InvalidCapacity`; overflow or insufficient total
space rejects the candidate as `CandidateInfeasible` or `InvalidCapacity` at
the corresponding checked boundary.

### 9. Canonicalize, hash, validate, and publish

Before constructing the public result, artifact lists, placements, stage
placements, copies, outputs, values, releases, and loop-domain records are
sorted by stable IDs. The Draft identity is a domain-separated SHA-256 digest
of the candidate, topology and discovery identities, loop horizon/domains,
values and producers, kernel templates, scheduled task windows/dependencies
and task payloads, artifact identities/build contracts, resource manifest,
arena objects and bindings, alias contracts, init images, and releases. This
means two semantically different static plans cannot share a Draft identity
unless the identity implementation itself is broken, which is surfaced as an
identity or validation failure.

`DraftPlan::validate` is called on the constructed value with the same topology
and discovery. Failure is `InvalidDraft`; the planner never returns a partially
validated candidate. A successful result contains the Draft, provisional arena
layouts, scheduler makespan, all lowering evidence, and the program loop
metadata.

## Candidate ranking and rejection behavior

Every assignment receives an identity before lowering. Assignment failures in
the set `{NoRoute, DependencyConflict, CandidateInfeasible,
InvalidCapacity}` are collected as candidate-local failures and enumeration
continues. Structural input, identity, artifact, arithmetic, Draft, and
unexpected scheduler errors abort the complete planning call. If no assignment
produces a valid candidate, the call returns `NoViableCandidate` with the
number of attempted identities and the first candidate-local failure.

Otherwise candidates are sorted by `(planned.makespan,
planned.draft.candidate)`. Makespan is therefore the measured primary score;
the stable candidate identity is the deterministic tie-breaker. The planner
does not use a heuristic placement preference or a backend result to mutate a
candidate after ranking.

`ProgramPlannerSearch` is consumed by `recipe-prepare` as follows:

```text
plan_program_candidates(...)
    -> next_candidate().cloned()
    -> realizer.realize(candidate)
       -> success: warm/stabilize, repack, Finalize
       -> CandidateRejected: reject(identity), continue
       -> Fatal: abort preparation
```

The search itself knows nothing about native realization or capacity snapshots
after warmup. Preparation owns those observations and calls `reject` only
after a candidate has been issued and destroyed. This keeps planner identity,
candidate-local failure, and runtime teardown separate.

## End-to-end callers and callees

The direct in-workspace caller of `plan_program_candidates` is
`recipe-prepare::prepare_program_validated` (`prepare/src/lib.rs`). It obtains
a reservation plan, computes optimistic planning capacity, resolves the
artifact catalog, invokes the planner, and then walks the one-shot program
search. A candidate that cannot be realized, stabilized, or repacked is
explicitly rejected and the next planner candidate is attempted. A successful
candidate remains unchanged while preparation creates native artifacts and
resources, warms the maximum-concurrency trace, validates post-warm capacity,
recomputes layouts, and calls core Finalize.

`plan_candidates` currently has no separate in-workspace call site. It remains
the public graph-shaped compatibility API and has exactly the same validation,
lowering, scheduling, and ranking semantics through the one-iteration program
adapter.

The principal downstream consumers of `PlannedCandidate` are:

* `recipe-prepare` realization adapters, which inspect `draft.artifacts`,
  `draft.artifact_builds`, `stage_placements`, `placements`, and the resource
  manifest before native loading;
* `recipe-native-executor::candidate`, which validates the exact Draft and
  artifact set before final handoff;
* `recipe-native-executor::local` and `::bridge`, which bind calculation,
  metric, init, exit, and one-hop transfer tasks to native backend resources;
* core `FinalizedBundle`, which consumes the unchanged Draft after
  post-realization capacity and layout evidence.

The planner's principal callees are:

* `recipe-language::CalculationGraph` for graph validation and topological
  order;
* `recipe-primitives::lower` and `LoweredProgram::validate` for typed stage
  lowering;
* `recipe-core` validators for topology, discovery, reservations, capacity,
  artifact, and Draft contracts;
* `recipe-scheduler::schedule` for measured windows, lane claims, and
  makespan;
* `recipe-scheduler::pack_arenas` for deterministic logical-object packing.

No callee in this path opens a device, compiles code, allocates backend state,
or executes a task. Those effects begin only in the preparation and native
executor stages after a candidate has crossed this boundary.

## Invariants a caller may rely on

The following properties are established before a candidate is returned:

1. The graph/program, topology, discovery, reservations, capacity, and every
   supplied artifact have passed their owning validators.
2. Every source kernel has one legal GPU placement, every lowered stage has a
   stable template and artifact/build identity, and every returned placement
   is sorted deterministically.
3. The Draft contains only core `Calculation`, `Transfer`, and `Metric` task
   kinds. Init admission and exit egress are transfers. User metrics and
   fault readbacks are four-byte metric transfers. No planner-only state is
   required to interpret a task.
4. Internal routes are represented as one-hop transfers with explicit
   intermediate values and dependencies. A final transfer route can never
   hide multiple directed links from the scheduler or executor.
5. Init precedes loop, loop precedes exit, alias overwrites wait for old-value
   consumers, and every loop task has exactly one `LoopTaskDomain`.
6. A logical value has no duplicate resident copy on one device. A transferred
   copy cannot satisfy a different consumer domain, and user metrics read the
   producer-resident copy.
7. External outputs identify the exact physical value used by their finalized
   exit task. Must-alias inputs that would overwrite the only safe copy cannot
   be exported.
8. Arena objects, value bindings, alias groups, image offsets, lifetimes,
   queue/completion slots, metric slots, staging, and scratch account for the
   complete static schedule. The optimistic layout plus auxiliary peaks fits
   the supplied Recipe-usable capacity.
9. The Draft identity covers all schedule, resource, artifact, loop, and
   layout-contract evidence, and `DraftPlan::validate` succeeds against the
   exact topology and discovery identities supplied to planning.
10. Candidate ranking and search delivery are deterministic. A caller can
    reject a returned identity and continue without changing any other
    candidate's contents or order.

These are planning contracts, not runtime success claims. Native realization
can still reject a candidate when a target artifact, reservation, warm trace,
or post-warm capacity observation does not satisfy the same immutable Draft.
