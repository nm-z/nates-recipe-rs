# recipe_planner::planner

    document: recipe_planner.planner
    source: planner/src/planner.rs
    kind: measured-aot-candidate-planner
    authority:
      - planner/src/planner.rs
      - planner/src/lib.rs
      - planner/src/error.rs
      - planner/src/hash.rs
      - core/src/plan.rs
      - core/src/schedule.rs
      - core/src/topology.rs
      - core/src/discovery.rs
      - core/src/artifact.rs
      - language/src/graph.rs
      - program/src/lib.rs
      - primitives/src/model.rs
      - primitives/src/lower.rs
      - scheduler/src/static_schedule.rs
      - scheduler/src/route.rs
      - scheduler/src/arena.rs
      - prepare/src/lib.rs
      - native-executor/src/candidate.rs
      - system-contract.md

This document follows the current planner implementation, not a proposed design.
It records what the code accepts, constructs, schedules, rejects, and returns.
The planner is the Draft boundary in the fixed preparation pipeline:

    CalculationGraph + StaticCalculationProgram
          + Topology + DiscoveryProfile
          + ReservationLedger + CapacityLedger + artifact identities
                               |
                               v
               plan_program_candidates / plan_candidates
                               |
                               v
               ranked PlannedProgramCandidate values
               (validated offset-free DraftPlan plus evidence)
                               |
                               v
               prepare: realize, warm, observe, repack, Finalize
                               |
                               v
               immutable FinalizedBundle -> init -> loop -> exit

The planner performs no native allocation, module load, compiler invocation, arena
offset assignment, queue creation, or runtime submission. It lowers logical graph
work to concrete calculation and transfer tasks, constructs measured schedule
windows and resource claims, and emits the exact Draft that preparation must
realize without changing. TaskKind::Metric is only a four-byte device readback,
including user telemetry and fault readback, not a third model operation.

## Public surface and result ownership

planner/src/lib.rs forbids unsafe code, denies missing Debug implementations, and
reexports the error types, placement records, candidate records, and the two
planning entrypoints. planner.rs contains the implementation and hash.rs contains
the domain-separated StableDigest helper.

### Placement and copy records

The planner exposes four records (planner/src/planner.rs:29-59):

    KernelPlacement {
      kernel: KernelTemplateId,
      device: DeviceId,
    }

    StagePlacement {
      source_kernel: KernelTemplateId,
      stage_ordinal: u32,
      kernel_template: KernelTemplateId,
      device: DeviceId,
      artifact: ArtifactId,
    }

    LogicalValueCopy {
      logical: ValueId,
      device: DeviceId,
      physical: ValueId,
    }

    PlannedExternalOutput {
      task: TaskId,
      logical: ValueId,
      device: DeviceId,
      physical: ValueId,
    }

LogicalValueCopy is the complete logical-to-physical resident-copy inventory.
PlannedExternalOutput is narrower: it names the physical copy selected by the
finalized Exit egress task, not every copy that exists.

### Planned candidates

PlannedCandidate (planner/src/planner.rs:62-73) contains:

    draft: DraftPlan
    arena_layouts: Vec<ArenaLayout>
    makespan: Nanoseconds
    placements: Vec<KernelPlacement>
    stage_placements: Vec<StagePlacement>
    lowered_programs: Vec<LoweredProgram>
    value_copies: Vec<LogicalValueCopy>
    external_outputs: Vec<PlannedExternalOutput>

DraftPlan is the complete logical plan. It has values, kernel templates, static
and deferred artifact identities, scheduled tasks, resource manifests, logical
arena objects and bindings, alias contracts, packed init-image manifests, and one
release record per topology device. arena_layouts is attached as feasibility and
ranking evidence. The Draft remains offset-free: ValueBinding.object_offset is
relative to an arena object and physical arena offsets are selected by
scheduler::pack_arenas after realization capacity is available.

PlannedProgramCandidate (planner/src/planner.rs:121-126) wraps a PlannedCandidate
with the declared LoopIterations and one LoopTaskDomain { task, domain } for every
scheduled loop task. This is the value consumed by prepare::PrepareSystem for
finite and unbounded programs.

### One-shot search streams

PlannerSearch and ProgramPlannerSearch are ranked, one-shot streams. Each holds a
sorted candidate vector, a cursor, an issued set, and a rejected set
(planner/src/planner.rs:76-176). ranked_candidates returns the complete ranked
slice, including candidates later marked rejected. next_candidate advances the
cursor, skips identities in the rejected set, records the identity in issued, and
returns at most one candidate. A candidate is never issued again after the cursor
passes it, even if the caller did not call reject.

reject is bookkeeping, not replanning. It succeeds only for an identity previously
returned by next_candidate, and only once. An unknown or not-yet-issued identity is
UnknownCandidate; a second rejection is AlreadyRejected. Preparation clones each
returned candidate before native realization and marks it rejected when realization,
stabilization, or final capacity fails (prepare/src/lib.rs:379-510, 528-551).

## Entry points

### plan_candidates

plan_candidates(graph, topology, discovery, artifacts, reservations, capacity) is
the compatibility entrypoint for a graph with no explicit loop metadata
(planner/src/planner.rs:190-218). It creates
StaticCalculationProgram::every_iteration(graph.clone(), LoopIterations::ONE),
maps a program error to InvalidGraph, delegates entirely to plan_program_candidates,
and strips the loop sidecar from each candidate. There is no second algorithm.

### plan_program_candidates

plan_program_candidates(program, topology, discovery, artifacts, reservations,
capacity) is the complete implementation (planner/src/planner.rs:222-350). It
accepts a finite or unbounded static program without unrolling its graph or
duplicating stage artifacts. The implementation performs these stages:

1. Validate program and graph.
2. Validate topology structure and scheduling properties.
3. Validate DiscoveryProfile against that topology.
4. Validate reservations and capacity against the same topology.
5. Obtain the deterministic graph topological order and indexes.
6. Lower every source primitive to a validated LoweredProgram and derive
   stage-scoped identities and scalar kernel templates.
7. Validate the artifact catalog.
8. Build the finite GPU placement product.
9. Hash graph, program, domain, and metric identity.
10. Lower every placement assignment, retaining only candidate-local infeasibility.
11. If none survive, return NoViableCandidate. Otherwise sort by measured
    makespan and stable candidate identity and initialize the one-shot search.

Malformed global input, an unexpected identity error, or an unexpected schedule
error aborts the call. Candidate-local NoRoute, DependencyConflict,
CandidateInfeasible, and InvalidCapacity are collected while enumeration
continues. No surviving assignment produces NoViableCandidate with assignment count
and the first candidate failure.

## Inputs and preconditions

The function explicitly calls these validators before deriving planning state:

| Input | Validation and effect |
| --- | --- |
| StaticCalculationProgram | program.validate checks the graph, one domain per kernel, domain bounds, dependency initialization, and metric declarations. Failure is InvalidGraph. |
| CalculationGraph | graph.validate checks tensors, primitive kernels, unique producers, external boundaries, and acyclic dependencies. topological_order repeats validation and returns stable KernelTemplateId order. |
| Topology | validate checks identity, devices, node ownership, directed links, transports, and duplex resources. validate_scheduling_properties rejects unmeasured capacity, transfer rate, calculation rate, link bandwidth, and link concurrency. Failure is InvalidTopology. |
| DiscoveryProfile | validate(topology) requires every topology object to be discovered and available, asynchronous schedulable capabilities, GPU calculation capability, and identity agreement. Failure is InvalidDiscovery. |
| ReservationLedger | validate(topology) requires exactly one evidence-matching reservation per device. Failure is InvalidReservation. |
| CapacityLedger | validate(topology, reservations) requires one schedulable entry per device and overflow-safe reservation, overhead, fragmentation, headroom, and Recipe-usable accounting. Failure is InvalidCapacity. |
| Artifact slice | validate_artifact_catalog rejects zero or duplicate IDs and calls ArtifactIdentity.validate on every entry. Failure is InvalidArtifact. |

The planner never converts an estimated topology property into a schedule. Only
measured properties or explicit overrides that pass core schedulability validators
reach the scheduler (core/src/topology.rs:421-464,
system-contract.md:196-205, 294-307).

StaticCalculationProgram supplies iterations, one source domain per kernel, and
MetricEmission { metric, value, domain } records. Its validator guarantees every
domain is inside the loop horizon and every metric domain is covered by its value
producer. The planner copies those values into hashing and assigns the same
domains to generated loop tasks.

## Program lowering and stable identities

### Common lowering hardware

lower_programs (planner/src/planner.rs:359-434) lowers every graph node with
recipe_primitives::lower. It first derives one LoweringHardware from all
available discovered calculation devices (436-472):

* subgroup width is the maximum discovered subgroup width;
* maximum workgroup width is the minimum discovered workgroup limit; and
* shared memory limit is the minimum discovered per-workgroup limit.

No available calculation device, or a common subgroup width greater than the
common workgroup width, is InvalidDiscovery. The common intersection makes one
backend-neutral lowering valid for every legal placement. The planner does not
lower a different primitive program for each assignment.

Every lowered program is revalidated. Primitive lowering or LoweredProgram.validate
failure is InvalidGraph. A lowered program retains stage geometry, typed static
views, operation bounds, synchronization, atomic and fault contracts, resource
bounds, and its canonical digest. Zero-element lowering is preserved as a program
with no fabricated dispatch; the invocation path emits no calculation task or
artifact for a stage that does not exist.

### Stage identities and templates

stage_template_identity hashes the domain-separated program digest, source kernel ID,
and stage ordinal (474-498). The first eight digest bytes become a nonzero
KernelTemplateId. A reserved zero result or collision between distinct source-stage
pairs is IdentityCollision. A scalar-map stage clones its source KernelTemplate,
replaces the template ID with this stage identity, validates it, and inserts it in
the template map. Non-scalar stages retain stage identity and build contract but do
not fabricate a KernelTemplate record.

The resulting map is keyed by source kernel for LoweredKernelPlan { program,
stage_templates }, and by stage identity for scalar templates. Every stage artifact
and calculation task uses the derived identity.

### Graph and Draft identities

graph_digest uses recipe-planner-graph-v7 (574-650). It includes loop horizon,
every tensor ID, dtype, storage bytes, external flags, offset, shape extents, and
strides; every topological kernel ID, iteration domain, ordered inputs and outputs,
calculated work, lowered program digest, and ordered stage identities; and every
metric ID, value ID, and metric domain. StableDigest uses length-delimited bytes
and fixed little-endian integer encoding (planner/src/hash.rs). A missing graph
item during hashing is InvalidGraph.

candidate_identity uses graph, topology identity, discovery identity, and ordered
(kernel, device) placement under recipe-planner-candidate-v3 (746-763). Distinct
assignments must produce distinct identities; the enumerator detects a collision
before lowering.

hash_draft uses recipe-planner-draft-v10 (3594-3768). It serializes candidate and
profile identities, loop schedule, values and producers, scalar templates, every
task window/dependency/payload/route/lane claim/submission slot, static and
deferred artifact contracts, resource manifest, arena objects and lifetimes,
value bindings, alias contracts, init images, and releases. Any change to these
observable Draft fields changes DraftIdentity.

## Finite placement enumeration

legal_choices returns one sorted option list per graph kernel (519-555). An option
is legal only when the topology device is DeviceKind::GpuMemory and the validated
discovery entry exposes calculation capability. Non-GPU storage is never a
calculation target. With n graph kernels and g legal GPUs, enumeration visits the
finite g^n product recursively in sorted device-ID order (557-572). There is no
heuristic placement pruning before lowering.

The same GPU can be selected for every kernel. Cross-device transfers are inserted
only where a consumer's selected device differs from its resident source copy.
Candidates rank by measured schedule makespan, then CandidateIdentity (338-343).

## Candidate lowering pipeline

lower_candidate (979-1214) builds one isolated LoweringState per assignment. IDs
start at one and are monotonically allocated by StableIdAllocator; overflow is
ArithmeticOverflow. A task allocation also creates temporary queue and completion
IDs equal to that task ID; interval compaction later replaces these temporary slots.

    initialize_data_images
      -> one Init transfer per topology device
    lower_program_invocation for each topological kernel
      -> stage calculations, buffers, copies, aliases, artifacts
    add_fault_readbacks
    add_user_metrics
    add_alias_dependencies
    add_external_outputs
    add_alias_dependencies again, including Exit tasks
    add_phase_barriers
    schedule
    compact_submission_resources
    finalize_auxiliary_resources
    build_arena_contract
    pack_arenas
    require_total_capacity
    canonical sorting and Draft hash
    DraftPlan.validate
    return PlannedProgramCandidate

Every helper returns PlannerResult. No helper silently substitutes a task, route,
artifact, or value after a failure.

### Lowering state

LoweringState (792-961) owns candidate-local task and value allocators, all
ValueSpec records, a logical-to-device RuntimeCopy inventory, logical copy and
external-output sidecars, init-image regions, unscheduled tasks, the resource
manifest, alias invocations, fault flag/cohort/readback maps, init-task map, and
loop-domain map. RuntimeCopy records resident device, physical value, producer
task, and optional transfer domain. Direct/init copies have no transfer domain;
loop-refreshed copies carry the consumer IterationDomain.

push_copy permits at most one copy of a logical value per device. A duplicate
device copy is InvalidDraft, not an overwrite. assign_loop_domain permits one
domain per task; assigning a different domain is InvalidDraft.

### Init data images

initialize_data_images (2276-2466) performs all init image and fault flag
materialization before loop tasks exist:

1. For each selected kernel, record every external-input tensor used by that
   kernel on its selected device.
2. Assign an external input/output tensor not consumed by any kernel to the first
   topology device so a pass-through output has a resident copy.
3. For every topology device, including RAM and disk devices, allocate one init
   task, one image value, and one external-to-device TransferTask in Init.
4. Pack external tensor members in deterministic BTreeSet order. Allocate each
   physical ValueSpec, logical copy, InitDataImageMember, and image offset. The
   image is at least four bytes even without tensor members.
5. Group each ProgramFaultFlag by (device, IterationDomain). Allocate one
   four-byte I32 flag per domain cohort, map it to every checked stage in that
   cohort, and include it in the image. The image task is its producer.
6. Make the image value span the complete packed image and be the destination of
   the single admission task. Byte-size overflow is ArithmeticOverflow.

state.init_tasks[device] is the readiness predecessor for init-resident inputs,
outputs with no nonempty dispatch writer, and fault flags. Core Draft validation
requires exactly one init upload per topology device
(core/src/plan.rs:651-684, 813-834).

### One source-kernel invocation

lower_program_invocation (1299-1542) lowers one graph node on one selected GPU
and one source iteration domain:

* Allocate one submission/task ID per lowered stage before materializing buffers.
* materialize_program_buffer (1577-1648) maps an input tensor through ensure_copy,
  creates new resident values for output and scratch buffers, and resolves a fault
  buffer to the preallocated image flag. An unrelated tensor buffer is
  InvalidDraft.
* Stage dependencies become dependencies on prior stage tasks. Every read binding
  depends on the latest ready writer. Dependencies are sorted and deduplicated,
  and self-dependencies are removed.
* Convert bindings to ArtifactBuildBinding records, preserving dtype, access mode,
  extents, offset, strides, and storage bytes. Read bindings other than the fault
  flag become calculation inputs; writes other than the fault flag become outputs.
* Build and validate an ArtifactBuildRecipe containing stage identity, source kernel,
  lowered program digest, stage ordinal, typed views, dispatch geometry,
  FLOP/integer/atomic bounds, fault flag, and kernel resource bounds.
* select_or_defer_artifact either chooses an exactly matching catalog artifact or
  retains the build recipe. A catalog entry must match stage template, build
  provenance, resource bounds, and selected discovery target
  (1681-1717). A mismatch is InvalidArtifact.
* Emit one TaskKind::Calculation in Loop with stage inputs, outputs, fault flag,
  work, and submission. Assign the source domain. Checked stages join the
  (device, domain) fault cohort, and all cohort stages must share one flag.
* Writes update ready/writer maps and ValueSpec.producer, except the fault flag,
  whose producer remains the init image task.
* Resolve source outputs to physical values. A nonempty output must have a
  dispatch writer; a zero-byte output may use init-task readiness. Register each
  source output as a direct resident copy with no transfer domain.
* Convert source alias positions to one-based core IDs, validate exact typed static
  views for MustAliasExact, and retain an AliasInvocation with writer task,
  physical inputs/outputs, and permissions.

Access helpers recognize Read, Write, ReadWrite, and ReadWriteAtomic. A stage
reading before a ready writer, a missing buffer, or an out-of-range alias position
returns the specific InvalidDraft or InvalidGraph at the call site.

### Fault readbacks and user metrics

add_fault_readbacks (1544-1575) emits one loop MetricTask per (device, domain)
fault cohort. It depends directly on every checked calculation, reads the shared
four-byte flag, owns one metric slot, and uses MetricPurpose::FaultReadback.

add_user_metrics (1216-1297) resolves each MetricEmission to its source-kernel
producer and selected producer device. It requires a resident producer-device copy
with no transfer domain, so a user metric cannot publish a refreshed remote copy.
The user metric depends on its value producer and every fault readback, uses a
dedicated metric slot, is a Loop task, and receives the emission domain. Missing
producer, resident copy, or a transferred copy is InvalidGraph or InvalidDraft.

Core Draft validation additionally requires one readback per fault flag, direct
dependencies on every checked calculation, and transitive dependencies from user
metrics and Exit tasks (core/src/plan.rs:873-958).

## Transfer lowering and route selection

### Directed route enumeration

directed_routes (2060-2109) enumerates every simple directed path from source to
destination. Source-equal-destination returns one empty route. Outgoing links are
sorted by stable LinkId and a visited-device set avoids cycles. No route is
invented for a disconnected graph.

### Physical hop chains

build_transfer_chain (2126-2274) validates a complete route, expands it into one
executor-visible TransferTask per link, and allocates one resident intermediate
value per internal endpoint. The destination value uses the allocator head;
intermediate values follow in route order. Each hop:

* is a Loop task on the consumer domain;
* depends on the source producer for the first hop or preceding hop;
* names device source and device destination values;
* contains one route link, or an empty route for an explicit same-device copy;
* receives a queue/completion pair derived from its task ID; and
* submits on the hop source device.

The chain retains final destination value/task, all ValueSpecs, all unscheduled hop
tasks, source-device submission ownership, and loop sidecars. Route endpoint
mismatch, absent link, wrong hop order, wrong destination, or no hop is
InvalidDraft.

This decomposition is required by scheduler and core validation: an
executor-visible internal transfer has at most one directed link, while a
multi-link path is dependency-chained through resident intermediate values
(scheduler/src/static_schedule.rs:14-23, 245-376;
core/src/schedule.rs:385-398).

### Choosing a source and route

ensure_copy (2468-2616) reuses an existing destination copy when it is
init/directly produced or transferred for the same consumer domain. If a
different-domain transfer already occupies that device, it returns
CandidateInfeasible rather than reusing a stale domain copy.

Otherwise it tries every compatible resident source and every directed route.
A source is compatible when it is direct/init resident or was transferred for the
same consumer domain. Each chain is trial scheduled through trial_chain_timing,
which invokes the complete scheduler with phase barriers. A route is retained
only
when its full chain is schedulable. The comparison key is:

    (final_hop_end, trial_makespan, source_device, source_value, route_link_ids)

Final readiness is primary, then total measured makespan, then stable source and
route identity. route_count == 0 maps to NoRoute; routes found but none
schedulable map to CandidateInfeasible. The selected chain is materialized for
real, and every allocator result is checked against the trial allocation head.
Any change in stable task/value allocation is InvalidDraft.

Only the final destination copy enters the logical copy inventory. Intermediate
values remain physical values and participate in arena lifetime analysis.

trial_chain_timing treats arithmetic overflow, insufficient capacity, and no route
as an unschedulable trial (Ok(None)). Dependency cycles and invalid lifecycle
dependencies are DependencyConflict; any other scheduler error is Schedule
(1996-2058).

## External output lowering

add_external_outputs (2643-2749) handles every external_output tensor in sorted
tensor-ID order. It first computes all must-alias input physical values. For each
output it considers only a resident copy with no transfer domain and not a
must-alias input that a later calculation may overwrite.

Each source is trial scheduled as an Exit device-to-external transfer. The choice
key is (trial_makespan, final_end, source_device, source_value). A source with no
safe copy is DependencyConflict; safe copies with no schedulable egress are
CandidateInfeasible. The chosen task receives a submission on its source device
and is recorded as PlannedExternalOutput.

External admission and egress always have an empty internal route. Core Draft
validation requires admission in Init, egress in Exit, one upload per topology
device, and source/destination values resident on named devices
(core/src/plan.rs:634-712, 813-834).

## Alias contracts and dependencies

value_alias_contracts converts each invocation's one-based argument positions to
physical ValueId pairs and sorts by input/output identity (1849-1874). Only
MustAliasExact rules affect scheduling and arena coalescing; Forbidden and
MayAliasExact remain contracts without forcing storage reuse.

add_alias_dependencies (1924-1971) protects a must-alias overwrite. For a
must-alias input, every other task referencing the old physical input is added as
a dependency of the writer. An Exit consumer that still needs the old value is an
immediate DependencyConflict. The helper runs before output insertion and again
after external-output tasks, so the final dependencies include all consumers.

build_arena_contract unions must-alias input/output values with a stable disjoint
set (3139-3391). Union members must have the same resident device and exact byte
size. Image-fixed members must refer to one fixed image object and offset; two
different image locations are invalid. Otherwise one 16-byte-aligned object with
object-relative offset zero is made. Non-alias values each get an object. Every
value gets a ValueBinding and every object gets a derived lifetime.

## Lifecycle barriers and scheduling

add_phase_barriers (1973-1994) adds every Init task as a dependency of every Loop
task, and every Init and Loop task as a dependency of every Exit task. Existing
dependencies are sorted and deduplicated. The scheduler independently adds global
phase edges (scheduler/src/static_schedule.rs:458-510). No loop task can begin
before all images are admitted, and no exit task can begin before all loop work
and readbacks finish.

schedule(topology, discovery, tasks) (1059-1071) validates and schedules the
complete unscheduled set. It computes durations from measured FLOP rates and
transfer bandwidth, claims measured compute, transfer, and external lanes,
queue/completion slots, half-duplex direction resources, and overlap resources,
then performs deterministic critical-path list scheduling
(scheduler/src/static_schedule.rs:52-160). It rejects duplicate or unknown
dependencies, cycles, later-phase dependencies, unavailable capabilities,
invalid calculation placement, invalid routes, arithmetic overflow, and
insufficient capability. Planner mapping is:

| Scheduler error | Planner error |
| --- | --- |
| DependencyCycle, InvalidLifecycleDependency | DependencyConflict |
| ArithmeticOverflow, InsufficientCapacity | CandidateInfeasible |
| NoRoute | NoRoute |
| any other scheduler error | Schedule |

StaticSchedule supplies concrete half-open task windows and measured makespan.
Those windows become immutable Task.window values in Draft and drive all
resource-lifetime calculations.

## Queue and completion compaction

next_submission initially creates one queue and completion pair per task. The
initial manifest is not final. compact_submission_resources (2784-2911) computes
interval ownership from scheduled task windows and measured queue limits.

task_submission_device classifies tasks as follows:

* calculation, metric, same-device, external admission, and external egress are
  DeviceLocal on their owning device;
* a cross-device transfer is owned by its source device and exact
  (source,destination) CrossDevice class.

Intervals are sorted by start, end, and task ID. Greedy interval coloring runs
independently within each owner class. A color's first task supplies stable
queue/completion IDs and later nonoverlapping tasks reuse those slots. Device
colors from all owner classes are counted against
DiscoveryProfile.maximum_submission_queues; exceeding the measured limit is
CandidateInfeasible. Task payloads are rewritten with compact slots and the final
queue/completion vectors are sorted by slot ID.

The owner split prevents one logical cross-device submission slot from being
materialized in each backend owner while allowing nonoverlapping local work to
reuse slots. It does not alter dependencies, windows, routes, or lane claims.

## Auxiliary resource accounting and capacity

finalize_auxiliary_resources (2963-3031) derives staging and scratch peaks from
scheduled windows:

* external-to-device and device-to-external transfers contribute byte counts to
  endpoint-device pinned-staging events;
* internal device-to-device hops do not consume external staging;
* each calculation resolves its artifact exactly once from static identity or
  deferred build recipe and contributes its scratch bound on its calculation
  device; and
* metric tasks add no staging or scratch bytes.

peak_usage (3034-3094) uses half-open start/end events, checks addition overflow
and release underflow, and returns only nonzero per-device peaks. The resource
manifest receives separate pinned_staging and scratch vectors; a combined peak
is retained for capacity admission.

build_arena_contract computes lifetimes from each value producer and every task
reference. A value crossing Init, Loop, or Exit keeps the whole schedule window.
For multi-iteration or unbounded loops, a value not refreshed before every active
consumer domain keeps the whole loop window. value_crosses_iteration_boundary
uses domain_covers for first index, stride divisibility, and finite/unbounded end
coverage (3393-3484). Tasks with zero active finite iterations do not force a
loop-wide lifetime.

pack_arenas(topology, arena_objects, capacity) chooses lowest aligned legal
offsets for nonoverlapping lifetimes and checks each arena against recipe_usable
(scheduler/src/arena.rs:9-153). require_total_capacity (3537-3571) adds each
device's combined staging and scratch peak to arena size. Overflow is
CandidateInfeasible; missing capacity is InvalidCapacity; required total above
recipe_usable is InvalidCapacity.

ArenaLayout values are attached to PlannedCandidate. They are not written into
DraftPlan and cannot cause a later runtime replan.

## Artifact selection and realization contract

Every lowered stage reserves an ArtifactId equal to its stage template ID. If the
catalog contains that ID, the planner accepts it only when all of these match:

* kernel template identity;
* ArtifactBuildProvenance, including lowered program digest, stage ordinal, and
  build contract digest;
* kernel resource bounds; and
* selected device's discovered target identity.

The exact identity is copied into DraftPlan.artifacts. If the catalog has no entry,
the exact ArtifactBuildRecipe is copied into DraftPlan.artifact_builds. A stage
cannot be both realized and deferred, and every calculation task must resolve
exactly once. Native preparation later materializes deferred recipes or loads
selected identities, then validates runtime images against immutable stage
placement and target (native-executor/src/candidate.rs:475-613;
prepare/src/production.rs:310-370).

The planner does not compile or inspect native bytes. It records typed views,
geometry, work, fault, and resource evidence for the realization boundary to
authenticate the artifact without changing the Draft.

## Final Draft assembly

After scheduling and resource finalization, lower_candidate canonicalizes
artifacts, build recipes, kernel placements, stage placements, logical copies,
external outputs, and values by stable identity. It emits:

* all scalar KernelTemplate values from the stage template map;
* one ValueSpec per resident tensor, scratch, image, fault flag, and transfer
  intermediate;
* Task records returned by the scheduler, including windows and canonical
  transfer lane claims;
* compacted ResourceManifest records;
* alias contracts and image members;
* arena objects and object-relative value bindings;
* one ArenaRelease for every topology device; and
* the loop sidecar for every scheduled loop task.

Draft discovery and topology identities are copied from validated inputs.
hash_draft computes DraftIdentity, then DraftPlan.validate is called against the
same topology and discovery. Any constructed invariant failure is InvalidDraft;
no invalid or partial Draft is returned.

## Caller and callee trace

### Preparation caller

PrepareSystem::prepare_program_validated obtains a reservation plan, validates it,
computes optimistic planning capacity, resolves the artifact catalog, and calls
plan_program_candidates (prepare/src/lib.rs:340-377). Planning capacity is only
an upper-bound enumeration ledger. For each one-shot candidate, preparation calls
the native CandidateRealizer, warms maximum concurrency, validates post-warm
capacity snapshots, repacks arenas, and calls
FinalizedBundle::finalize_with_loop_schedule. A candidate rejected by realization
or stabilization is destroyed and marked in search; capacity or finalization
failure destroys the session before the next candidate. The first unchanged
candidate that passes all evidence becomes the prepared system. If all finite
candidates are rejected, preparation returns CandidateExhaustion.

### Primitive and program callees

The planner calls:

* StaticCalculationProgram::validate, graph, domains, iterations, and metrics;
* CalculationGraph::validate and topological_order;
* recipe_primitives::lower and LoweredProgram::validate;
* ArtifactIdentity::validate;
* topology, discovery, reservation, and capacity validators;
* recipe_scheduler::schedule and pack_arenas; and
* DraftPlan::validate.

It does not call a native backend, executor, compiler, allocator, transport, or
probe. Native execution consumes PlannedCandidate only after preparation. The
native candidate validator checks topology/discovery and Draft identities,
artifact set exactness, deferred build provenance, immutable stage placement, GPU
capability, and target equality before realization
(native-executor/src/candidate.rs:280-412).

### Runtime boundary

The planner output is consumed by native execution as an immutable task and
resource description. Finalization resolves logical values to arena offsets and
transfer endpoints to resolved addresses; runtime submission uses the selected
queue/completion slots, routes, lane claims, artifacts, and phase windows. The
loop cannot add a route, compile, allocate, discover, resize, or replan. This
preserves one Init admission, repeated immutable loop-domain activation, and one
Exit egress/release per required device.

## Error behavior

PlannerErrorKind (planner/src/error.rs:4-24) is explicit:

    InvalidGraph          program, graph, primitive, buffer, alias, or stage contract
    InvalidTopology       topology structure or scheduling properties
    InvalidDiscovery      missing, unavailable, incompatible, or non-common hardware
    InvalidReservation    reservation ledger mismatch
    InvalidCapacity       capacity ledger or total planned bytes do not fit
    InvalidArtifact       catalog, build, or realization identity mismatch
    InvalidDraft          constructed Draft reference or invariant failure
    MissingArtifact       reserved artifact absent (public error surface)
    NoCalculationDevice   no measured GPU calculation choice
    NoRoute               no directed route exists for a required transfer
    DependencyConflict    alias, lifecycle, dependency conflict, or scheduler cycle
    CandidateInfeasible   candidate route, queue, arithmetic, staging, scratch, or arena infeasible
    NoViableCandidate     every finite assignment failed
    Schedule              other scheduler failure
    ArithmeticOverflow    stable ID, byte, index, or hash-input overflow
    IdentityCollision     candidate, stage, or template identity collision
    UnknownCandidate      reject called for an unissued identity
    AlreadyRejected       reject called twice for one identity

MissingArtifact is part of the public non-exhaustive error enum, but the current
planner path treats an absent catalog entry as a deferred build recipe rather
than an error. A missing native artifact is normal realization input when the
stage build contract is retained.

Candidate-local failures are skipped only at the plan_program_candidates
enumeration boundary, and only for NoRoute, DependencyConflict,
CandidateInfeasible, and InvalidCapacity. A malformed global input, identity
collision, invalid graph, invalid artifact catalog, or unexpected schedule error
aborts the call. There are no hidden retries or alternate implementations; the
next candidate is the next finite placement assignment already enumerated.

## Invariants enforced by the current implementation

The returned candidate is valid only when all of these hold:

* Every graph kernel is assigned to one measured GPU calculation device.
* Every graph kernel has one validated lowered program and one source domain.
* Every nonempty lowered stage has one collision-checked stage identity and one
  exact static artifact or deferred build contract.
* Every resident value has one device, typed byte size, and producer or init owner;
  a logical value has at most one copy per device.
* Every loop task, including transfer hops and metrics, has exactly one domain.
* Every topology device has exactly one packed init image and upload.
* Every cross-device path is split into one-link transfer tasks with resident
  intermediates and predecessor dependencies.
* Every scheduled transfer has the complete canonical measured lane claim set,
  and every calculation, metric, or transfer submission names compacted queue and
  completion slots owned by its resource class.
* Every checked calculation cohort has one four-byte fault flag and one direct
  readback; user metrics and external outputs depend on those readbacks.
* Must-alias values have equal device and exact bytes, and their overwrite task
  cannot race a consumer or invalidate an exit value.
* External outputs use a safe direct or init copy, never a domain-specific transfer
  copy or a value that must be overwritten in place.
* Arena objects have nonempty lifetimes, 16-byte alignment in planner-created
  objects, deterministic bindings, and capacity-backed layouts.
* Arena plus pinned-staging plus scratch peaks fit each validated device's
  Recipe-usable capacity.
* Draft identity covers every emitted logical choice, and Draft validation succeeds
  against the exact topology and discovery identities supplied.

These checks are planning evidence only. Native realization must still prove that
selected artifacts, reservations, warmed resources, and post-warm capacity realize
this unchanged Draft before Finalize.
