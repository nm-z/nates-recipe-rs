# Planner errors

`planner/src/error.rs` is the complete public failure vocabulary for the
finite, deterministic planner.  The planner turns a validated graph, measured
topology and discovery profile, artifact identities, reservations, and a
capacity ledger into immutable `DraftPlan` candidates.  It does not execute a
candidate and it does not repair an invalid input.  An error therefore means
that an input contract, a generated draft invariant, a candidate resource
constraint, or the one-shot candidate-stream protocol was not satisfied.

The implementation is in `planner/src/planner.rs`.  The line references below
name the actual construction sites in that file.  The source file is the
authority for the exact wording of each message; this document records what
each site means, where the value goes next, and what state is consequently
discarded or retained.

## Error value shape

`PlannerErrorKind` is `Copy`, `Debug`, `PartialEq`, and `Eq`, and is marked
`#[non_exhaustive]`.  Downstream matches must retain a wildcard.  The current
members are:

```text
InvalidGraph, InvalidTopology, InvalidDiscovery, InvalidReservation,
InvalidCapacity, InvalidArtifact, InvalidDraft, MissingArtifact,
NoCalculationDevice, NoRoute, DependencyConflict, CandidateInfeasible,
NoViableCandidate, Schedule, ArithmeticOverflow, IdentityCollision,
UnknownCandidate, AlreadyRejected
```

`PlannerError` has two public fields:

* `kind: PlannerErrorKind` is the stable classification.
* `message: String` is the observed detail, normally the `Display` text of a
  lower-level validation or scheduling error, or a planner invariant
  statement.

`PlannerError::new` accepts any `Into<String>` and stores the value without
additional wrapping.  `Display` renders `"{:?}: {}"`, so a user sees the
variant name followed by the detail.  The type implements
`std::error::Error`; there is no source-error field or `From` implementation.
`PlannerResult<T>` is exactly `Result<T, PlannerError>`.

The planner does not accumulate multiple independent `PlannerError` values.
Validation collections from `recipe-core` are converted to one message, and a
helper's first returned error propagates through `?`.  The one exception is
candidate enumeration: four explicitly candidate-local kinds are retained as
strings so another placement can be tried.  That distinction is described
below.

### Error-producing call graph

The `?` boundaries are direct and intentionally shallow.  The following is
the complete error-producing call graph, with the caller that owns the next
propagation decision:

| Caller | Error-producing callees | Next boundary |
| --- | --- | --- |
| `plan_candidates` | `StaticCalculationProgram::every_iteration`, `plan_program_candidates` | Initial graph construction is `InvalidGraph`; delegated planner errors pass through. |
| `plan_program_candidates` | `StaticCalculationProgram::validate`, `CalculationGraph::validate`, `Topology::validate`, `Topology::validate_scheduling_properties`, `DiscoveryProfile::validate`, `ReservationLedger::validate`, `CapacityLedger::validate`, `topological_order` | Admission and indexing errors return immediately. |
| `plan_program_candidates` setup | `lower_programs`, `validate_artifact_catalog`, `legal_choices`, `graph_digest` | Any error is fatal before enumeration. |
| `enumerate_assignments` callback | `candidate_identity` followed by the identity-set check, `lower_candidate` | Identity collisions are fatal; only the four candidate-local kinds are retained. |
| `lower_candidate` | `initialize_data_images`, `lower_program_invocation`, `add_fault_readbacks`, `add_user_metrics`, `add_alias_dependencies`, `add_external_outputs`, `compact_submission_resources`, `finalize_auxiliary_resources`, `build_arena_contract`, `value_alias_contracts`, `pack_arenas`, `require_total_capacity`, `DraftPlan::validate` | The local `LoweringState` is dropped on error; the caller applies the candidate-local match. |
| `lower_program_invocation` | `LoweringState::next_submission`, `materialize_program_buffer`, `lower_build_binding`, `select_or_defer_artifact`, `set_value_producer`, `program_tensor_value`, `first_output_writer`, `validate_source_must_alias` | Errors return to `lower_candidate`; no partial invocation is retained. |
| `materialize_program_buffer` | `ensure_copy`, `LoweringState::next_value` | Tensor and transfer errors return through invocation lowering. |
| `ensure_copy` | `build_transfer_chain`, `trial_chain_timing`, `LoweringState` ID/submission allocators, `assign_loop_domain`, `push_copy` | Trial schedule misses become `None`; chosen-chain invariant failures return to `lower_candidate`. |
| `add_external_outputs` | `must_alias_inputs`, `trial_timing`, `trial_chain_timing`, `LoweringState::next_submission` | Safe-copy absence becomes a candidate-local dependency/feasibility error; bookkeeping errors are fatal. |
| `compact_submission_resources` | `compact_submission_resources_with_limits`, `task_submission_device` | Queue-limit infeasibility is candidate-local; malformed task/value indexing is fatal. |
| `finalize_auxiliary_resources` | `peak_usage` | Arithmetic overflow is remapped to `CandidateInfeasible`; underflow remains `InvalidDraft`. |
| `build_arena_contract` | `StableIdAllocator::take`, alias index conversion, `value_crosses_phase_boundary`, `value_crosses_iteration_boundary` | Arena invariant failures return directly to `lower_candidate`. |
| `lower_candidate` finalization | `value_alias_contracts`, `DraftPlan::validate` | The completed draft is either returned or discarded as one unit. |
| `PlannerSearch::reject`, `ProgramPlannerSearch::reject` | No planner helper | Protocol errors are `UnknownCandidate` or `AlreadyRejected`; no candidate state is changed. |

The scheduler and arena packer are the only foreign error boundaries with a
separate planner-kind mapping table.  Their typed errors are converted
explicitly, as detailed in the scheduler section; no lower-level scheduler or
packer error is allowed to leak through `PlannerResult`.  Core, language,
primitive, and program validation errors are rendered directly into the
planner message at their call sites.

## Public call graph and propagation boundary

The public functions and the only in-workspace caller form this path:

```text
plan_candidates(graph, ...)
  -> StaticCalculationProgram::every_iteration(..., LoopIterations::ONE)
  -> plan_program_candidates(program, ...)
  -> PlannerSearch (loop metadata is removed from each ranked item)

plan_program_candidates(program, ...)
  -> validate program and graph
  -> validate topology and scheduling properties
  -> validate discovery, reservations, and capacity
  -> topological_order and graph indexing
  -> lower_programs and stage-template identities
  -> validate artifact catalog and enumerate legal device choices
  -> graph_digest
  -> enumerate_assignments
       -> lower_candidate for each finite assignment
  -> rank successful candidates by (makespan, CandidateIdentity)
  -> ProgramPlannerSearch
```

`plan_candidates` converts only the initial
`StaticCalculationProgram::every_iteration` failure at
`planner/src/planner.rs:198-199`; all errors from its delegated
`plan_program_candidates` call pass through unchanged.  The program form is
the full planner boundary.

`plan_program_candidates` performs all checks through line 277 before the
first assignment is lowered.  An error from this admission/lowering setup is
returned immediately.  During `enumerate_assignments` (lines 293-323), a
candidate identity is inserted before lowering.  `lower_candidate` errors of
`NoRoute`, `DependencyConflict`, `CandidateInfeasible`, or `InvalidCapacity`
are appended to `candidate_failures` and enumeration continues.  Every other
error, including `InvalidDraft`, `InvalidArtifact`, `Schedule`,
`ArithmeticOverflow`, and `IdentityCollision`, aborts the entire planner at
the first occurrence.  If no candidate reaches `ranked`, the planner returns
`NoViableCandidate` with the number of assignments and the first captured
failure (or `"no assignment could be lowered"`).  A successful candidate is
never returned together with an error.

The workspace caller is `recipe-prepare::Preparer::prepare_program_validated`
(`prepare/src/lib.rs:369-377`).  It maps every planner error, including
`NoViableCandidate`, to `PrepareErrorKind::Planning` and retains only
`PlannerError::to_string()` in the message.  The prepare layer does not branch
on an individual `PlannerErrorKind`.  A planner error therefore stops the
fixed-point preparation before native realization.  The direct planner API
still exposes the typed kind to callers that use `recipe-planner` itself.

## Candidate state and error disposition

`lower_candidate` owns one fresh `LoweringState` (values, tasks, copies,
images, aliases, resource slots, and loop domains).  Every helper below is
called through this local state.  On any error the state is dropped, so no
partially allocated value, task, resource slot, arena object, or artifact
selection can escape.  For a candidate-local failure, only its formatted
message is retained in `candidate_failures`; the next assignment starts with
a fresh state and stable ID allocators.  For a fatal error, assignment
enumeration and the whole search return immediately.

The exact candidate-local set is the match at
`planner/src/planner.rs:309-320`:

| Planner kind | Candidate handling | Result when all assignments fail this way |
| --- | --- | --- |
| `NoRoute` | Retain detail and try the next assignment. | `NoViableCandidate`. |
| `DependencyConflict` | Retain detail and try the next assignment. | `NoViableCandidate`. |
| `CandidateInfeasible` | Retain detail and try the next assignment. | `NoViableCandidate`. |
| `InvalidCapacity` | Retain detail and try the next assignment. | `NoViableCandidate`. |

`InvalidCapacity` is intentionally in this set even though the admission
ledger was validated.  It covers a candidate's arena packing or final total
capacity check, and therefore can differ by placement.  The broad mapping at
`planner/src/planner.rs:1099-1100` converts any `pack_arenas` scheduler error
to this kind, including an arena/topology or arithmetic error reported by
that lower-level function.

The four kinds that are transient during route or egress trial scheduling are
handled one level earlier.  `trial_chain_timing` treats scheduler arithmetic
overflow, insufficient capacity, and no route as `Ok(None)` (lines
2016-2027), allowing the caller to test another route or source copy.  That is
not a planner error construction.  If all choices disappear, `ensure_copy`
or `add_external_outputs` constructs the candidate-local kind described in the
variant index.

## Variant reference and construction index

The following sections enumerate every current variant and every planner
construction site.  A line range identifies the caller and the immediate
invariant that failed.

### Input and graph/profile contracts

#### `InvalidGraph`

This kind means that the calculation graph, its static program, or a
lowered-program lookup is internally inconsistent.  It is always fatal to
the whole search because no placement can make the graph valid.

* `plan_candidates` maps a failure constructing the one-iteration static
  program at `198-199`.
* `plan_program_candidates` maps `StaticCalculationProgram::validate` at
  `230-232`, `CalculationGraph::validate` at `233-235`, and
  `topological_order` at `252-254`.
* `lower_programs` maps primitive lowering failure at `372-380`, an invalid
  lowered program at `381-393`, and a scalar stage template validation failure
  at `416-420`.
* `graph_digest` requires every ordered kernel, source iteration domain, and
  lowered program to be present.  Missing node, missing domain, a kernel
  `work` calculation failure, and missing lowered program are constructed at
  `604-609`, `611-616`, `626-630`, and `631-636` respectively.
* `lower_candidate` repeats the authoritative lookups while constructing one
  assignment.  Missing node, lowered program, and source domain are at
  `1007-1012`, `1014-1019`, and `1025-1030`.
* `add_user_metrics` rejects a metric whose value has no source-kernel
  producer at `1221-1239`.
* `validate_source_must_alias` rejects a declared exact alias whose typed
  static views differ at `1809-1823`.  `source_buffer` rejects an alias index
  beyond the declared input/output arity at `1829-1847`.
* `initialize_data_images` requires every ordered kernel, lowered program,
  fault-buffer domain, and image tensor to remain indexed.  The missing-node,
  missing-program, missing-domain, and missing-tensor sites are
  `2288-2300`, `2301-2308`, and `2361-2367`.
* `ensure_copy` rejects a transfer request for a tensor absent from the graph
  tensor index at `2494-2499`.

Each of these errors returns through `?` to `plan_program_candidates`; the
candidate-local match does not catch `InvalidGraph`.  The candidate state is
dropped if the failure occurred after lowering began.

#### `InvalidTopology`

This kind is used for the topology admission contract, not for a route that
became inconsistent while a candidate was being assembled.

* `plan_program_candidates` maps both `Topology::validate` and
  `Topology::validate_scheduling_properties` at `236-241`.
* `initialize_data_images` needs a deterministic device for an external
  pass-through tensor.  An empty topology at `2344-2351` constructs this kind
  at `2346`.

The preflight forms are fatal.  The pass-through case is reached only while
lowering an assignment, but it is not classified as candidate-local, so it
also aborts the search rather than trying another device.

#### `InvalidDiscovery`

This kind means that measured discovery data cannot support the planner's
required common lowering or submission-resource contract.

* The complete discovery profile is mapped at `242-244`.
* `common_lowering_hardware` requires at least one available calculation
  capability at `436-447` and a common full-subgroup workgroup width across
  all available calculation devices at `453-470`.
* `compact_submission_resources_with_limits` requires a measured queue limit
  for every task-owning device at `2856-2862`.

The first site is admission-fatal.  The two lowering sites are also fatal, not
candidate-local, because lowering has no coherent hardware contract to use.

#### `InvalidReservation`

`plan_program_candidates` maps `ReservationLedger::validate(topology)` at
`245-247`.  The planner accepts no unvalidated reservation ledger.  The
failure returns before graph lowering and no candidate state exists.

#### `InvalidCapacity`

The capacity ledger is mapped during admission at `248-250`.  Candidate
capacity failures have three additional sources:

* `pack_arenas` is mapped wholesale at `1099-1100`, as described in the
  candidate-local table.  The scheduler's original kind and detail survive
  only in the formatted message.
* `require_total_capacity` rejects a missing capacity entry at `3553-3558`.
* The same helper rejects arena plus pinned-staging/scratch bytes greater than
  the usable ledger entry at `3559-3568`.

The arithmetic overflow while adding those two totals is deliberately
`CandidateInfeasible` at `3547-3552`, not `InvalidCapacity`.

#### `InvalidArtifact`

This kind means that an artifact identity or deferred build does not exactly
match the stage contract.

* `validate_artifact_catalog` rejects a zero or duplicate artifact ID at
  `500-507`, then maps `ArtifactIdentity::validate` at `509-514`.
* `select_or_defer_artifact` rejects a catalog artifact whose template,
  provenance, resources, or discovered target does not exactly match the
  generated build at `1688-1710`.
* `finalize_auxiliary_resources` requires each calculation task artifact to
  resolve exactly once, either in the chosen catalog or in deferred builds.
  The neither/both case is `3000-3011`.

All three sites are fatal.  A missing catalog entry alone is not an error:
`select_or_defer_artifact` appends the generated build to `artifact_builds` at
`1712-1715`.

#### `MissingArtifact`

This enum member is declaration-only in the current workspace.  There is no
`PlannerErrorKind::MissingArtifact` construction site.  In particular, an
artifact absent from the input catalog follows the deferred-build path above;
the planner does not report a missing artifact for that normal case.  The
`#[non_exhaustive]` enum leaves this category available for a future contract,
but no current caller can observe it.

### Placement, route, dependency, and schedule outcomes

#### `NoCalculationDevice`

`legal_choices` filters topology devices to `DeviceKind::GpuMemory` entries
whose discovery entry has calculation capability.  If the resulting set is
empty, `519-543` constructs `NoCalculationDevice` at `538-542`.  This occurs
after graph/profile admission but before assignment enumeration, so it is a
fatal planner error and there are no candidate failures to aggregate.

#### `NoRoute`

There are two construction paths:

* The main `schedule` call maps `ScheduleErrorKind::NoRoute` directly at
  `1059-1070` (the `1067` arm).
* `ensure_copy` counts every directed route from an existing resident copy to
  the requested device.  When `route_count == 0`, the `best` lookup at
  `2556-2568` constructs `NoRoute` at `2558` and includes both route and
  schedulable counts in the message.

The main schedule failure is candidate-local through the match at
`309-320`.  Trial route scheduling suppresses lower-level no-route errors as
`Ok(None)` and reaches this construction only after all source/route options
are exhausted.  A no-route candidate is discarded; a search with only such
assignments returns `NoViableCandidate`.

#### `DependencyConflict`

This kind says that dependencies cannot be ordered without violating an
alias or lifecycle invariant.

* The main scheduler maps `DependencyCycle` and
  `InvalidLifecycleDependency` to this kind at `1061-1063`.
* `add_alias_dependencies` rejects a must-alias invocation that would
  overwrite a value still required by an exit task at `1924-1954`, with the
  construction at `1948-1954`.
* `trial_chain_timing` maps the same two scheduler dependency errors at
  `2028-2038`, constructing the planner error at `2034-2037`.
* `add_external_outputs` counts resident copies that are not overwritten by a
  must-alias input.  If `safe_count == 0`, the egress `best` lookup at
  `2705-2718` constructs this kind at `2707`.

The main and trial forms are candidate-local when they return through
`lower_candidate`.  A rejected egress has no task or resource entry retained;
the candidate state is dropped before another assignment is tried.

#### `CandidateInfeasible`

This kind means that a structurally valid candidate cannot satisfy measured
route, domain, queue, time, or memory constraints.

* Main scheduling arithmetic overflow and insufficient capacity are mapped at
  `1064-1066`.
* `finalize_auxiliary_resources` changes only its own
  `ArithmeticOverflow` result to `CandidateInfeasible` at `1084-1089`,
  preserving the original message while declaring the candidate unusable.
* `ensure_copy` rejects reuse of a transferred copy whose iteration domain is
  different from the consumer's at `2477-2492` (construction `2486-2492`).
* When directed routes exist but none can be trial-scheduled,
  `ensure_copy` chooses this kind at `2556-2567`, construction `2560`.
* If external-output copies survive must-alias overwrite checks but no egress
  trial can be scheduled, `add_external_outputs` chooses this kind at
  `2705-2717`, construction `2709`.
* Submission interval coloring rejects a device requiring more simultaneous
  queues than measured at `2856-2876`, construction `2870-2876`.
* `require_total_capacity` classifies overflow of arena plus auxiliary bytes
  as this kind at `3547-3552`.

All of these are in the candidate-local set.  They do not imply that another
placement is valid, only that the current finite assignment is not.

#### `Schedule`

`lower_candidate` maps every scheduler error not explicitly recognized as a
dependency conflict, candidate infeasibility, or no route to `Schedule` at
`1059-1070`, the wildcard arm at `1068`.  This includes scheduler duplicate,
unknown-dependency, unavailable-capability, invalid-calculation-placement,
invalid-transfer, and unexpected topology/discovery kinds that somehow pass
the planner's earlier admission.  Because `Schedule` is not in the
candidate-local match, it aborts the whole search.

`trial_chain_timing` maps an unrecognized trial scheduler error to `Schedule`
at `2039-2044`.  It also constructs `Schedule` when the scheduler succeeds
but omits the requested completion task from its static result, at
`2046-2056` (construction `2052-2055`).  These are treated as fatal planner
invariant failures, not as a reason to silently skip a route.

### Generated-draft invariant failures

`InvalidDraft` is the largest category.  It means the planner generated, or
is about to generate, an object that cannot satisfy the immutable
`DraftPlan` contract.  It is never candidate-local: a malformed generated
draft indicates a planner or input invariant failure, not a placement that
can safely be retried.

The construction sites are grouped by the state they protect.

#### Identity, state indexing, and loop domains

* `stage_template_identity` reports a changed digest width as `InvalidDraft`
  at `474-485` (construction `481-484`).  A zero resulting identity is a
  separate `IdentityCollision`.
* `LoweringState::push_copy` rejects two resident copies of one logical value
  on the same device at `927-947` (construction `934-940`).
* `LoweringState::assign_loop_domain` rejects assigning one loop task two
  different domains at `950-960` (construction `951-957`).
* After scheduling, `lower_candidate` requires every loop task to have a
  domain at `1131-1152` (construction `1145-1150`).
* The final constructed `DraftPlan` is validated at `1188-1194`; any core
  validation collection is wrapped as `InvalidDraft` at `1189-1193`.

#### Metrics, invocation bindings, and stage ordering

* `add_user_metrics` requires a selected device for the metric's producer at
  `1240-1248` (construction `1243-1248`).  It then requires a producer-device
  resident copy at `1249-1262` (construction `1254-1262`) and rejects a
  transferred copy where a producer-resident value is required at
  `1263-1271` (construction `1264-1270`).
* `lower_program_invocation` requires every declared prior stage barrier to
  exist at `1331-1350` (construction `1338-1347`).
* A read binding must have an earlier producer at `1351-1365` (construction
  `1353-1363`).
* The generated `ArtifactBuildRecipe` must validate at `1414-1425`
  (construction `1416-1424`).
* All calculations sharing a device and iteration domain must use one fault
  flag.  A second flag is rejected at `1443-1454` (construction
  `1448-1453`).
* A nonempty kernel output must have a dispatch writer at `1493-1509`
  (construction `1498-1505`).
* A lowered tensor buffer must be an input or output of its source kernel at
  `1577-1618` (construction `1610-1617`).
* A fault buffer must have a corresponding init-image value at `1630-1646`
  (construction `1635-1643`).
* Every stage binding must resolve to a materialized buffer value at
  `1650-1678` (construction `1654-1661`).
* `set_value_producer` rejects assignment to a value absent from `state.values`
  at `1719-1732` (construction `1724-1729`).
* `program_tensor_buffer` rejects a kernel tensor with no lowered program
  buffer at `1734-1748` (construction `1739-1747`);
  `program_tensor_value` rejects a buffer that was not materialized at
  `1750-1765` (construction `1755-1764`).

#### Alias and transfer lowering

* `add_alias_dependencies` rejects a must-alias input index absent from its
  invocation at `1937-1942` (construction `1938-1942`).
* `build_transfer_chain` maps route validation failure at `2136-2146`, rejects
  an absent link at `2153-2159`, a hop with the wrong source at `2160-2167`,
  a route ending at the wrong device at `2172-2177`, and a route that produced
  no hop task at `2180-2190` (construction `2185-2189`).
* `initialize_data_images` rejects a kernel/fault-buffer key inserted twice
  at `2418-2429` (construction `2423-2428`).
* `ensure_copy` verifies that stable value allocation is unchanged after
  route trials at `2579-2587`, rejects a chain containing a non-transfer task
  at `2588-2595`, and verifies stable task/submission allocation at
  `2595-2601` (construction `2597-2600`).
* `must_alias_inputs` rejects an invocation without the referenced input at
  `2625-2637` (construction `2631-2636`).
* `add_external_outputs` requires every declared external output to have a
  resident copy at `2657-2663` (construction `2658-2663`) and verifies stable
  egress task allocation after trial scheduling at `2719-2725`
  (construction `2721-2724`).

#### Resource ownership and auxiliary planning

* Submission compaction rejects an index that no longer names a scheduled
  task at `2893-2899` (construction `2894-2898`).
* `task_submission_device` rejects an external-to-external transfer at
  `2917-2943` (construction `2938-2942`) and rejects a metric whose value is
  absent from the value-device index at `2945-2959` (construction
  `2950-2957`).
* `finalize_auxiliary_resources` rejects an external-to-external transfer
  reaching staging planning at `2981-2992` (construction `2986-2991`).
* `peak_usage` rejects resource subtraction below zero at `3068-3083`
  (construction `3077-3082`).

#### Arena aliasing and lifetime contracts

* `build_arena_contract` rejects a must-alias input or output index absent from
  an invocation at `3169-3184` (constructions `3170-3173` and `3180-3183`).
* Every must-alias group must have equal device and exact byte size at
  `3214-3223` (construction `3219-3222`).
* A fixed data-image location must remain indexed at `3241-3248`
  (construction `3242-3247`), and must-alias values may not occupy two fixed
  image locations at `3249-3254` (construction `3250-3253`).
* Every selected arena object must still exist when members are attached at
  `3256-3270` (construction `3265-3269`).
* The complete schedule must have either both lifetime boundaries or neither
  at `3283-3297` (construction `3291-3296`).  The same invariant for a
  multi-iteration loop schedule is enforced at `3305-3319` (construction
  `3311-3316`).
* A value producer must name a scheduled task at `3327-3339` (construction
  `3332-3337`).
* Every arena object must have a live task and a lifetime end at
  `3321-3380` (constructions `3367-3373` and `3374-3379`).
* `value_crosses_phase_boundary` rejects a read value with no scheduled
  producer at `3399-3413` (construction `3403-3408`).
* `value_crosses_iteration_boundary` requires every loop reader to have a
  domain at `3423-3435` (construction `3430-3434`).

All of these sites return through the current helper's `PlannerResult`, then
through `lower_candidate`, `enumerate_assignments`, and
`plan_program_candidates`.  None is converted into a candidate rejection.

### Numeric and identity safety

#### `ArithmeticOverflow`

This kind means that a stable ID, offset, index, byte count, or resource
accumulator could not be represented.  The direct construction sites are:

* `StableIdAllocator::take` detects the next task, value, or arena-object ID
  overflow at `853-862` (construction `855-860`).
* Buffer ID to host-index conversion in `lower_program_invocation` is mapped
  at `1461-1466` (construction `1462-1465`).
* `stable_argument_position` checks `usize` to one-based `u64` conversion at
  `1797-1807` (construction `1801-1806`).
* `value_alias_contracts` checks input and output alias-index conversion at
  `1849-1864` (constructions `1853-1858` and `1859-1864`).
* `add_alias_dependencies` checks the must-alias input index at
  `1931-1936` (construction `1931-1935`).
* `stable_offset` checks `usize` to `u64` conversion and base-plus-offset
  overflow at `2111-2123` (constructions `2112-2117` and `2118-2123`).
* `initialize_data_images` checks external-tensor image bytes and four-byte
  fault-flag extension at `2391-2396` and `2412-2417`.
* `must_alias_inputs` checks the input index conversion at `2625-2630`.
* Submission queue-limit conversion is checked at `2863-2868`.
* `peak_usage` checks active-byte addition at `3068-3075`.
* `build_arena_contract` checks must-alias input and output index conversion
  at `3157-3162` and `3163-3168`.

These errors normally abort planning immediately.  There are three explicit
nonfatal boundaries: trial scheduling treats scheduler arithmetic overflow as
an unschedulable trial (`2016-2027`), the main scheduler maps its arithmetic
overflow to `CandidateInfeasible` (`1064-1066`), and auxiliary-resource
arithmetic overflow is remapped to `CandidateInfeasible` (`1084-1089`).
`StableIdAllocator` overflow or any other direct planner overflow remains
`ArithmeticOverflow` and is not captured by the candidate-local match.

#### `IdentityCollision`

Planner identities are deterministic and are not deduplicated or replaced.
The four collision guards are:

* Distinct placement assignments producing the same `CandidateIdentity` are
  rejected during enumeration at `300-305`.
* `lower_programs` records each stage identity's source kernel and ordinal;
  a different pair reusing one identity is rejected at `397-408`.
* Inserting the same scalar stage template identity twice is rejected at
  `416-425`.
* A stage digest that produces the reserved zero `KernelTemplateId` is
  rejected at `486-495`.

Every collision is fatal.  The identity is not salted again, and the
assignment is not silently merged with the prior one.

### Candidate-stream protocol

`PlannerSearch` and `ProgramPlannerSearch` each hold a ranked vector, a cursor,
an `issued` set, and a `rejected` set.  Their `next_candidate` methods return
`None` at the end and skip identities already in `rejected`; they do not return
`PlannerError`.  A candidate is inserted into `issued` immediately before the
borrowed candidate is returned (`87-98` and `140-151`).

#### `UnknownCandidate`

Both `reject` methods require the supplied identity to be in `issued`.  A
future candidate, an identity from another search, or an arbitrary identity
constructs `UnknownCandidate` at `101-107` for `PlannerSearch` and
`154-160` for `ProgramPlannerSearch`.  The rejection set is unchanged.

#### `AlreadyRejected`

If the identity is already in the corresponding `rejected` set, the same
methods construct `AlreadyRejected` at `108-114` and `161-167`.  The set stays
unchanged.  A successful first rejection inserts the identity and returns
`Ok(())`; subsequent `next_candidate` calls skip it.

`recipe-prepare` calls `reject` only after a candidate-local realization,
stabilization, or final-capacity rejection.  If this bookkeeping itself
returns `UnknownCandidate` or `AlreadyRejected`, `prepare/src/lib.rs:532-545`
maps it to `PrepareErrorKind::Planning` instead of retrying or inventing a
different identity.

## Scheduler translation details

The planner depends on `recipe-scheduler`, whose `ScheduleErrorKind` is a
different non-exhaustive enum.  The main schedule call has this exact mapping
(`planner/src/planner.rs:1059-1071`):

| Scheduler kind | Planner kind | Main-candidate consequence |
| --- | --- | --- |
| `DependencyCycle`, `InvalidLifecycleDependency` | `DependencyConflict` | Candidate-local rejection. |
| `ArithmeticOverflow`, `InsufficientCapacity` | `CandidateInfeasible` | Candidate-local rejection. |
| `NoRoute` | `NoRoute` | Candidate-local rejection. |
| Any other scheduler kind | `Schedule` | Fatal planner error. |

Trial schedule calls use a deliberately different boundary.  At
`trial_chain_timing` (`1996-2058`), `ArithmeticOverflow`,
`InsufficientCapacity`, and `NoRoute` return `Ok(None)` so the caller can
consider another route or resident copy.  Dependency-cycle and lifecycle
errors become `DependencyConflict`; all other errors become `Schedule`; and a
successful schedule without the requested completion task is `Schedule`.

`pack_arenas` has no special mapping.  Its entire `ScheduleError` is rendered
to text and classified as `InvalidCapacity` at `1099-1100`, which is then
candidate-local.  No scheduler error object survives in `PlannerError`.

## End-to-end failure behavior

The observable path for a direct planner caller is:

1. A graph/profile/catalog/reservation/capacity admission error returns one
   `PlannerError` before any candidate is exposed.
2. A fatal lowering, identity, schedule, draft, artifact, or arithmetic error
   returns one `PlannerError`; every local task/value/resource allocation is
   dropped with its candidate state.
3. A candidate-local route, dependency, feasibility, or candidate-capacity
   error is recorded internally and the next finite assignment is lowered.
4. If at least one candidate succeeds, the search is ranked and returned.  The
   recorded failed-candidate details are not attached to the successful
   `PlannerSearch` or `ProgramPlannerSearch`.
5. If all assignments fail in the candidate-local set, the caller receives
   `NoViableCandidate` containing the finite assignment count and the first
   failure detail.  No partially lowered candidate is returned.
6. If the caller later rejects a ranked candidate, the one-shot search either
   advances to the next identity or reports `UnknownCandidate`/
   `AlreadyRejected` for protocol misuse.

For the fixed-point public preparation path, any step 1, 2, or 5 error is
rendered by `PlannerError` and wrapped as `PrepareErrorKind::Planning` at
`prepare/src/lib.rs:369-377`.  Candidate-local failures that were exhausted
inside the planner do not become per-candidate `CandidateRejection` records,
because preparation never receives a search when `NoViableCandidate` is
returned.  A search that does return candidates can later produce preparation
rejection records for native realization, stabilization, or final-capacity
failures; those are outside the planner error vocabulary.

The planner has no retry loop beyond finite assignment enumeration, no
fallback identity, no substitute capacity, and no recovery branch for a fatal
variant.  The typed kind and its message are the complete evidence of the
failed boundary.
