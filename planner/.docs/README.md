# `recipe-planner`

`recipe-planner` is the measured-topology boundary that turns one validated
Recipe calculation program into a deterministic, finite stream of complete
`DraftPlan` candidates. It owns placement enumeration, primitive-program
lowering, target-independent artifact requests, physical value-copy and
transfer lowering, static scheduling, resource accounting, arena lifetime
construction, and candidate identity generation. It does not load a driver,
compile native code, allocate a physical arena, or execute a task.

The implementation is intentionally concentrated in
[`src/planner.rs`](../src/planner.rs). The crate facade and public types are in
[`src/lib.rs`](../src/lib.rs), planner error values are in
[`src/error.rs`](../src/error.rs), and the private length-prefixed SHA-256
helper is in [`src/hash.rs`](../src/hash.rs). The crate has no unsafe code and
denies missing `Debug` implementations.

## Boundary and callers

The planner consumes these already materialized inputs:

| Input | Meaning at this boundary |
| --- | --- |
| `StaticCalculationProgram` | An acyclic `CalculationGraph`, one explicit `IterationDomain` per source kernel, a finite or unbounded `LoopIterations` contract, and optional scalar metric emissions. |
| `Topology` | Stable machines, nodes, storage devices, and directed physical links. GPU-memory devices are the only legal calculation placements. |
| `DiscoveryProfile` | The measured or explicitly overridden rates, concurrency limits, asynchronous capabilities, native target identities, and hardware limits used by lowering and scheduling. |
| `ArtifactIdentity` slice | Exact prebuilt artifacts that may satisfy a stage. A stage absent from this catalog remains a target-independent `ArtifactBuildRecipe` in the Draft. |
| `ReservationLedger` | The exact per-device user reservation required by the prepare boundary. |
| `CapacityLedger` | The optimistic or observed Recipe-usable capacity against which arena packing and auxiliary-resource accounting are checked. |

The direct production caller is `recipe-prepare`. Its
`Preparer::prepare_program_validated` first obtains reservations, derives an
optimistic planning capacity from measured total capacity minus the mandatory
reservation, resolves an artifact catalog, and calls
`plan_program_candidates`. It owns the subsequent fixed-point loop: it takes a
candidate from the planner search, realizes and warms that exact Draft through
the native boundary, observes post-warm capacity, repacks arenas, and either
finalizes the unchanged candidate or rejects it through
`ProgramPlannerSearch::reject` before trying the next one. See
[`prepare/src/lib.rs`](../../prepare/src/lib.rs), especially
`prepare_program_validated` and its `next_owned_program_candidate` helper.

The native executor receives the resulting `PlannedCandidate` through
`CandidateRealizationRequest`. Its validation checks the Draft, exact static
and deferred artifact sets, stage placements, target identities, and runtime
images before native resources are created. The local and bridge backends then
consume the Draft tasks, resources, images, value bindings, and artifact
contracts. Finalize later combines the planner's arena objects with measured
post-realization layouts. The planner's `lowered_programs` are retained so
`recipe-prepare` can lower deferred stages during `BuildPhase::Realize`.
The public facade reexports this crate as `recipe::planner`; no other crate
enters private planner helpers.

The crate depends on `recipe-core`, `recipe-language`, `recipe-primitives`,
`recipe-program`, `recipe-scheduler`, and `sha2`. The scheduler is the only
component that turns the planner's unscheduled tasks into windows and lane
claims. The planner supplies the scheduler with one executor-visible transfer
task per physical hop, never a composite multi-link task.

## Public surface

`src/lib.rs` reexports the following values:

* `plan_candidates` is the graph compatibility entry point. It wraps a graph
  in `StaticCalculationProgram::every_iteration` for exactly one iteration,
  delegates all work to `plan_program_candidates`, and removes the loop-sidecar
  fields from each returned value while preserving the same Draft.
* `plan_program_candidates` validates the complete input boundary, enumerates
  every legal kernel-to-GPU assignment, lowers every feasible assignment, and
  returns a `ProgramPlannerSearch` ranked by measured makespan and stable
  candidate identity.
* `PlannedCandidate` contains the complete `DraftPlan`, deterministic arena
  layouts used as planning evidence, its scheduled makespan, source-kernel
  placements, stage placements, lowered primitive programs, every logical value
  copy, and the exact external-output copy selected by each exit transfer.
* `PlannedProgramCandidate` wraps a `PlannedCandidate` with the program's
  `LoopIterations` and the exact `Vec<LoopTaskDomain>` sidecar required by
  Finalize. The graph and stage artifacts remain single copies even for an
  unbounded or repeated loop.
* `PlannerSearch` and `ProgramPlannerSearch` are ranked, one-shot streams.
  `ranked_candidates` exposes the immutable ranked slice. `next_candidate`
  advances a cursor, skips identities previously rejected, records every
  issued identity, and returns a borrowed candidate. `reject` succeeds only
  for an identity already issued and only once. Unknown or repeated rejection
  is an error rather than a fallback or a retry.
* `KernelPlacement` maps a source `KernelTemplateId` to its selected
  calculation `DeviceId`.
* `StagePlacement` maps a source kernel and stage ordinal to its collision-safe
  stage template, selected device, and reserved `ArtifactId`.
* `LogicalValueCopy` records one logical graph value, its resident device, and
  the candidate-specific physical `ValueId` for that device.
* `PlannedExternalOutput` records the task, logical value, device, and physical
  value used by one finalized exit egress. It is deliberately narrower than
  the complete copy inventory.
* `PlannerError`, `PlannerErrorKind`, and `PlannerResult` are the only public
  error API. `PlannerError` stores a typed kind and a human-readable message,
  displays as `Kind: message`, and implements `std::error::Error`.

The public structs are data contracts, not mutable planner state. Internal
enumeration and lowering state is private so callers cannot bypass validation,
stable ID allocation, scheduling, or Draft construction.

## End-to-end planning flow

`plan_program_candidates` follows one fixed sequence. Any input validation
failure returns immediately. Candidate-local infeasibility is collected while
the finite assignment product is explored, but structural failures abort the
whole call.

1. Validate the static program, its graph, the topology, topology scheduling
   properties, discovery, reservations, and capacity. Program and graph
   failures become `InvalidGraph`; topology failures become
   `InvalidTopology`; discovery, reservation, and capacity failures retain
   their corresponding kinds. Topological order is recomputed after
   validation.
2. Index graph tensors by `ValueId` and graph nodes by source kernel ID.
3. Lower every source primitive kernel once with
   `recipe_primitives::lower`. The planner first computes one common
   `LoweringHardware` from all available discovered calculation devices:
   subgroup width is the maximum, while maximum workgroup width and shared
   memory are the minima. No available calculation device is an error, and a
   common subgroup-compatible width is required. Every lowered program is
   independently validated before use.
4. Give every lowered stage a collision-checked `KernelTemplateId` derived
   from the lowered program digest, source kernel ID, and stage ordinal. Scalar
   stages contribute their `KernelTemplate` to the template catalog. A zero
   digest prefix, a duplicate stage identity, or an invalid template is a
   planner identity/graph error.
5. Validate the supplied artifact catalog. IDs must be nonzero and unique, and
   every identity must satisfy the core artifact contract.
6. Build legal placement choices. For each source kernel in topological order,
   the choices are every sorted topology device of kind `GpuMemory` for which
   discovery reports calculation capability. An empty graph produces an empty
   choice list and is then handled as one empty assignment by the same lowering
   path. If a nonempty graph has no such device, return `NoCalculationDevice`.
7. Hash the graph identity from tensor storage and layout, kernel input/output
   IDs and work, source iteration domains, loop count, metric declarations,
   lowered program digests, and stage-template identities. The topology and
   discovery identities are intentionally added later when a placement
   candidate identity is formed.
8. Recursively enumerate the complete Cartesian product of per-kernel device
   choices in stable order. For each assignment, derive a candidate identity
   from the graph digest, topology identity, discovery identity, and ordered
   `(kernel, device)` pairs. A collision between two distinct assignments is a
   fatal `IdentityCollision`.
9. Lower each assignment into a full `PlannedProgramCandidate`. `lower_candidate`
   creates all init, loop, and exit tasks, schedules and validates them, builds
   arena contracts, and computes the Draft identity. `NoRoute`,
   `DependencyConflict`, `CandidateInfeasible`, and `InvalidCapacity` are
   candidate-local failures and do not stop enumeration. Other errors, such as
   an invalid artifact catalog entry for a selected stage, are returned.
10. If no assignment survived, return `NoViableCandidate` with the number of
    finite assignments and the first recorded candidate failure. Otherwise
    sort by `(makespan, draft.candidate)` and initialize a fresh one-shot
    `ProgramPlannerSearch`.

The rank is therefore a static measured estimate, not a promise that native
realization will fit after warming. `recipe-prepare` performs that later
observation and may reject a ranked candidate without changing its Draft.

## Lowering one candidate

`lower_candidate` uses a `PlanningContext` containing the ordered graph,
indexed nodes and tensors, lowered programs and stage templates, topology,
discovery, artifact catalog, planning capacity, loop contract, source domains,
and metrics. An `Assignment` is converted to a kernel-to-placement map. A new
`LoweringState` starts all stable task and value allocators at one and owns the
following mutable construction state:

* `values` is the eventual `ValueSpec` inventory.
* `copies` maps each logical value to resident `RuntimeCopy` records per
  device. A copy includes its producing task and, for a transfer-created copy,
  the loop domain that refreshes it.
* `logical_copies` is the public copy inventory. `external_outputs` records
  only selected egress copies.
* `images` describes one packed init image per topology device. `tasks` holds
  `UnscheduledTask` values until the scheduler returns windows.
* `resources` starts with empty queues, completions, metric slots, staging,
  and scratch manifests. `aliases` records source alias invocations.
* `fault_flags`, `fault_cohorts`, and `fault_readbacks` connect checked scalar
  stages to one preallocated four-byte device readback per `(device, domain)`.
* `init_tasks` and `loop_domains` preserve admission dependencies and exact
  activation domains.

### Init images

`initialize_data_images` walks every selected kernel and lowered program. An
external-input tensor is admitted to each selected consumer device. A tensor
that is both an external input and output is also placed on the first sorted
topology device if no consumer selected it, so a pass-through output has a
resident copy. Fault buffers are grouped by selected device and exact source
iteration domain.

Every topology device receives exactly one init task, even if its image has no
graph members. The task transfers `External` to one device image value in
`RunPhase::Init`; its image size is the packed member bytes, with a minimum of
four bytes. Image members receive deterministic offsets and resident values,
and each logical input copy points at its physical member. A fault flag is an
int32, four-byte member of the same image and may be shared by all checked
stages on that device and domain. Core Draft validation later enforces one
init admission per required device.

### Source-kernel invocation and stages

`lower_program_invocation` allocates a submission task ID and provisional
queue/completion slot for each lowered stage, then materializes every
`ProgramBuffer`:

* input tensor buffers call `ensure_copy`,
* output tensor buffers receive a new device-resident `ValueSpec`,
* scratch buffers receive a new device-resident `ValueSpec`, and
* a fault buffer resolves to the image's shared int32 value.

For each stage, dependencies come from the lowered stage barriers and from the
producer recorded for every read binding. Bindings become ordered
`ArtifactBuildBinding` values with their typed affine views and access modes.
Inputs and outputs in the `CalculationTask` exclude the fault binding. A stage
creates a target-independent `ArtifactBuildRecipe` with:

* the stage-scoped template ID as both the calculation template and artifact
  identity (`ArtifactId` uses the same numeric value),
* source kernel ID and lowered-program digest plus stage ordinal provenance,
* a contract digest over ordered bindings, dispatch geometry, work, fault
  binding, and resource limits,
* the exact one-dimensional launch geometry and operation bounds, and
* the stage resource envelope, including the fixed workgroup size.

The recipe is validated before it reaches the Draft. `select_or_defer_artifact`
looks up that reserved artifact ID. A catalog hit must match the stage template,
deferred provenance, resource envelope, and selected device's discovered native
target exactly, or planning returns `InvalidArtifact`. A miss is not an error:
the recipe is appended to `DraftPlan::artifact_builds` for Realize.

The resulting stage task is a loop `CalculationTask` with the selected device,
artifact/template IDs, resident input/output values, optional fault flag,
measured work, and provisional submission slots. Its source domain is recorded
in `loop_domains`. Writes update readiness and producer maps. Every source
output then becomes a no-transfer `RuntimeCopy` on the selected device, with a
producer and no transfer domain. Stage placements retain the source kernel,
stage ordinal, template, device, and artifact ID.

The source alias list is converted from primitive zero-based positions to the
core's one-based `AliasRule` representation. Required exact aliases are
checked before task construction: input and output static views must have the
same dtype and access contract. The invocation is retained for later value
alias contracts and dependency insertion.

### Fault and user metrics

`add_fault_readbacks` creates one loop `MetricTask` with purpose
`FaultReadback` for every `(device, iteration domain)` fault cohort. It depends
on all checked calculation stages in that cohort and reads the shared int32
fault flag into a preallocated metric slot. The metric ID and slot are derived
from the stable task ID.

`add_user_metrics` locates the source kernel for each declared metric, selects
that kernel's device, and requires the producer-resident copy. A transferred
copy is rejected because telemetry must read the value produced on the source
device. The user metric depends on its value producer and all fault readbacks,
is a loop task, receives the emission domain, and gets a distinct metric slot.

### Alias ordering and external outputs

`add_alias_dependencies` handles `MustAliasExact` rules. Every task that still
references the old input value is made a dependency of the aliasing invocation
so the overwrite cannot happen early. If an exit task still needs that old
value, the candidate is rejected with `DependencyConflict`. The dependency pass
is run before and after output lowering because egress tasks are created after
source invocations.

`add_external_outputs` considers every graph tensor marked `external_output`.
It excludes transfer-created copies and any physical value that a must-alias
invocation will overwrite. Each surviving resident copy is trial-scheduled as
an exit transfer to `External`; the key is `(makespan, end, device, value)`.
The best safe and schedulable copy becomes one exit `TransferTask`, and its
exact identity is recorded in `PlannedExternalOutput`. No composite internal
route is used for logical egress.

Finally, `add_phase_barriers` adds all init tasks to every loop dependency and
all init plus loop tasks to every exit dependency. Init admission is therefore
always before the loop, and external egress always observes the complete loop
phase.

## Transfer lowering and route choice

`ensure_copy` is the only path that creates an internal logical copy. An
existing resident copy is reused when it has no transfer domain or was created
for the same consumer domain. A copy created for another domain cannot be
reused, and the candidate returns `CandidateInfeasible` rather than silently
sharing stale state.

When a new copy is required, every resident source copy whose domain is
compatible is paired with every simple directed route returned by
`directed_routes`. Routes are enumerated from sorted outgoing link IDs, avoid
revisiting a device, and include the empty route for an explicit same-device
copy. For each source and route, `build_transfer_chain`:

* validates the route against the topology;
* makes one transfer hop per directed link, or one explicit same-device copy;
* creates an intermediate resident `ValueSpec` for each internal endpoint;
* chains each hop to the preceding producer task;
* assigns the final destination value at the allocator head, followed by
  intermediate values in route order; and
* assigns a queue and completion slot owned by each hop's source device.

The complete chain is trial-scheduled together with existing tasks and phase
barriers. A route participates in measured bandwidth, inflight lanes,
half-duplex conflicts, endpoint transfer/compute overlap, queue contention,
source readiness, and makespan. The selected key is
`(final-hop end, chain makespan, source device, source value, route)`. The
planner then replays the selected chain and asserts that stable task/value
allocation has not changed during trial search. Only the final value is added
to the public logical-copy inventory; intermediate values remain arena objects
whose lifetimes are derived later.

If no directed route exists, `NoRoute` is candidate-local. If routes exist but
none can be scheduled, the result is `CandidateInfeasible`. The scheduler
rejects any executor-visible transfer with more than one directed link, so a
validated Draft never exposes a multi-hop task. This is the same contract
described in [`TRANSFER_LOWERING.md`](../TRANSFER_LOWERING.md).

## Scheduling and resources

The planner sends the complete unscheduled task list to
`recipe_scheduler::schedule`. The scheduler revalidates topology and
discovery, computes measured calculation and transfer durations, resolves
direct routes, performs deterministic critical-path list scheduling, assigns
queue/completion resources, reserves compute and transfer lanes, honors
half-duplex resources, and persists selected transfer lane claims. Scheduler
errors map as follows:

| Scheduler condition | Planner result |
| --- | --- |
| Dependency cycle or invalid lifecycle dependency | `DependencyConflict` |
| Arithmetic overflow or insufficient scheduling capacity | `CandidateInfeasible` |
| No route | `NoRoute` |
| Other scheduler failure | `Schedule` |

The returned schedule supplies concrete `Task` windows and a measured
candidate makespan. `compact_submission_resources` then replaces provisional
per-task slots with the smallest deterministic queue/completion manifest that
fits each device's measured `maximum_submission_queues`.

Compaction classifies each task by the device that owns its submission and by
an owner class. Local calculations, metrics, admissions, egresses, and
same-device copies share a device-local class. A cross-device hop is keyed by
its exact `(source, destination)` pair, preventing one logical slot from being
materialized by two backend owners. Intervals are sorted by start, end, and
task ID and greedily colored; the first task on each color supplies its stable
physical slot ID. Exceeding the measured queue limit is
`CandidateInfeasible`. The resulting queues and completions are sorted and
replace the provisional manifest references in every calculation, transfer,
and metric task.

`finalize_auxiliary_resources` derives peak pinned staging from external
admission/egress windows and peak scratch from calculation windows. It resolves
each calculation artifact exactly once from either `draft.artifacts` or
`draft.artifact_builds`, and uses that artifact's scratch envelope. Combined
staging plus scratch peaks are returned for total-capacity checking. Arithmetic
overflow becomes candidate infeasibility at this stage.

## Arena contract and lifetimes

`build_arena_contract` turns value specs and scheduled tasks into offset-free
arena objects plus `ValueBinding` records. It first unions values connected by
`MustAliasExact` rules with a deterministic smallest-root disjoint-set. Every
member of an alias group must have the same device and exact byte size.

Init image members are fixed into their image object at the recorded offsets.
Other alias groups get one new object, aligned to 16 bytes, unless exactly one
member already has a fixed image location. More than one fixed location is an
invalid Draft. Every value receives a binding to its object and relative
offset; no physical arena offset is selected here.

Object lifetimes include the producer task and every task that references each
member. A value read across init, loop, or exit extends its object over the
whole scheduled window. For a repeated or unbounded loop, a loop value also
extends over the loop window when any reader is not refreshed before every read
by a loop producer whose domain covers the reader domain. The domain helpers
check first iteration, end bounds, and stride divisibility. Every object must
have both a start and end, or lowering returns `InvalidDraft`.

The objects are packed with `recipe_scheduler::pack_arenas`. Packing is per
device, stable by lifetime start and object ID, reuses nonoverlapping
half-open lifetimes, aligns to the lowest legal offset, and checks the supplied
Recipe-usable capacity. The planner then adds the auxiliary staging and
scratch peak to each layout size. Missing capacity entries and arithmetic
overflow are invalid capacity or candidate infeasibility as appropriate.

The resulting `arena_layouts` are planning evidence only. They are not inserted
into the Draft and are recomputed after native stabilization by `prepare`.

## Draft assembly and identity

Before constructing `DraftPlan`, the planner sorts all order-independent
collections: realized artifacts, deferred builds, source and stage placements,
logical copies, external outputs, and values. Releases include every topology
device and are sorted by device. The core scalar-stage templates are copied
from the lowered template catalog. Value alias contracts are normalized from
alias invocations, and init images are converted from the internal image
regions.

`hash_draft` computes `DraftIdentity` with the domain
`recipe-planner-draft-v10`. It includes the candidate, topology and discovery
identities, loop count and every task domain, all value specs and producers,
kernel templates, task phases/windows/dependencies and task-kind payloads,
artifact identities and build recipes, queue/completion/metric manifests,
staging and scratch peaks, arena objects and bindings, alias contracts, init
images, and releases. The hash therefore changes when any execution-relevant
Draft field changes, including compaction slots and scheduled lane claims.

The completed Draft is validated through `DraftPlan::validate` against the same
topology and discovery. That core validator enforces nonzero identities,
identity ownership, unique values/kernels/artifacts/tasks, exactly one realized
or deferred artifact per calculation, resident value devices, legal lifecycle
phases, dependency ordering and windows, one init upload per topology device,
typed transfer endpoints, lane claims, metric slots, fault readbacks, arena
bindings, and release coverage. A constructed candidate that fails this final
check returns `InvalidDraft` rather than being exposed to preparation.

## Lifecycle and loop semantics

The graph is never unrolled. A source kernel's stages and artifacts are emitted
once, and each loop task carries its `IterationDomain` in the
`PlannedProgramCandidate::loop_domains` sidecar. `LoopIterations::Finite(n)`
means exactly `n` nonzero iterations; `Unbounded` has no fabricated terminal
iteration. Domains are nonempty arithmetic progressions and must fit the
program loop. The static program validator rejects a consumer whose first
activation precedes its producer and rejects a metric domain not covered by its
producer.

The planner keeps lifecycle phases explicit:

* Init contains exactly one external admission task per topology device.
* Loop contains calculations, internal transfers, fault readbacks, and user
  metrics, each with its declared domain.
* Exit contains one selected external egress task per external output.

Phase barriers and task dependencies provide the immutable ordering. Native
execution later repeats loop tasks according to the sidecar without allocating
per-iteration task or artifact copies.

## Error behavior

`PlannerErrorKind` is non-exhaustive and currently includes:

* input and catalog failures: `InvalidGraph`, `InvalidTopology`,
  `InvalidDiscovery`, `InvalidReservation`, `InvalidCapacity`, and
  `InvalidArtifact`;
* construction failures: `InvalidDraft`, `MissingArtifact`,
  `NoCalculationDevice`, `NoRoute`, `DependencyConflict`,
  `CandidateInfeasible`, `Schedule`, and `ArithmeticOverflow`;
* search/result failures: `NoViableCandidate`, `IdentityCollision`,
  `UnknownCandidate`, and `AlreadyRejected`.

Validation failures are returned before assignment enumeration. During lowering,
`NoRoute`, `DependencyConflict`, `CandidateInfeasible`, and
`InvalidCapacity` are the only errors deliberately treated as candidate-local
and collected for the final `NoViableCandidate` report. Missing artifact IDs,
invalid build contracts, absent values or producers, alias index errors,
stable ID allocation changes, impossible image or arena relationships, and
other `InvalidDraft` conditions abort planning. There are no alternate route
implementations, retries, substitute values, or defensive recovery branches.

The search bookkeeping has its own strict errors. Calling `reject` before a
candidate has been issued returns `UnknownCandidate`; calling it twice for the
same issued identity returns `AlreadyRejected`. Rejected identities remain
skipped when the cursor advances, while an unissued candidate cannot be
silently removed.

## Determinism and actual limits

Stable ordering is part of the planner contract. Graph topological order uses
kernel IDs, legal device choices and directed routes use sorted IDs, recursive
assignment order is fixed, route and egress trial keys have explicit
tie-breakers, IDs allocate from one, resource colors use first-task IDs, and
all serialized collections are sorted before hashing or validation. Candidate
ranking is therefore repeatable for the same graph, measured identities,
catalog, reservation, and planning capacity.

The placement search is exhaustive over `GPU-memory devices ^ source kernels`.
There is no heuristic pruning before a candidate is lowered. Every route trial
enumerates all simple directed paths, and every trial invokes the real static
scheduler. This is finite for a finite topology and graph, but can be
expensive. The planner's output is still bounded because an unbounded loop
changes activation domains, not graph or artifact cardinality.

The planner uses common lowering hardware across all available calculation
devices, but placement and artifact target checks remain per selected device.
An exact prebuilt artifact may satisfy a stage only when its stage provenance,
resource envelope, and discovered target match. Otherwise a deferred build is
the intended path. Zero-element lowered programs remain honest no-dispatch
candidates: the planner does not invent a calculation task, artifact, or fake
stage merely to make the candidate nonempty.

## Source map

| Area | Source of truth |
| --- | --- |
| Public facade and data contracts | [`src/lib.rs`](../src/lib.rs) |
| Error kinds and formatting | [`src/error.rs`](../src/error.rs) |
| Stable digest framing | [`src/hash.rs`](../src/hash.rs) |
| Validation, enumeration, lowering, scheduling integration, identities | [`src/planner.rs`](../src/planner.rs) |
| Primitive stage contract | [`../primitives/src/model.rs`](../../primitives/src/model.rs), [`../primitives/src/lower.rs`](../../primitives/src/lower.rs) |
| Static loop and metric contract | [`../program/src/lib.rs`](../../program/src/lib.rs) |
| Draft, value, task, artifact, topology, and discovery invariants | [`../core/src/plan.rs`](../../core/src/plan.rs), [`../core/src/schedule.rs`](../../core/src/schedule.rs), [`../core/src/artifact.rs`](../../core/src/artifact.rs), [`../core/src/topology.rs`](../../core/src/topology.rs), [`../core/src/discovery.rs`](../../core/src/discovery.rs) |
| Measured static scheduling and arena packing | [`../scheduler/src/static_schedule.rs`](../../scheduler/src/static_schedule.rs), [`../scheduler/src/route.rs`](../../scheduler/src/route.rs), [`../scheduler/src/arena.rs`](../../scheduler/src/arena.rs) |
| Prepare fixed-point caller | [`../../prepare/src/lib.rs`](../../prepare/src/lib.rs) |
| Native candidate validation and realization consumer | [`../../native-executor/src/candidate.rs`](../../native-executor/src/candidate.rs) |
| Primitive and transfer summary contracts | [`../PRIMITIVE_INTEGRATION.md`](../PRIMITIVE_INTEGRATION.md), [`../TRANSFER_LOWERING.md`](../TRANSFER_LOWERING.md) |
