# Native candidate sessions

`native-executor/src/candidate.rs` is the dependency-neutral, pre-Finalize
boundary for one exact `PlannedCandidate`. It is earlier than
`LocalBackend` binding and later than planner lowering. A successful session
owns the loaded runtime images, reservations, native queues, completion
objects, staging, scratch, pending pools, and warmed driver state, but it does
not own finalized arena offsets. The offsets are packed only after the measured
post-warm capacity is accepted. This separation is the reason a candidate can
be rejected and destroyed without mutating a `DraftPlan`.

The authoritative implementation and its callers are:

| Concern | Source of truth |
| --- | --- |
| Candidate request, errors, factory trait, validation wrapper | [`candidate.rs`](../../src/candidate.rs), especially `CandidateRealizationRequest`, `CandidateSessionFactory`, and `ValidatedCandidateFactory` |
| Candidate construction and one-shot planner stream | [`planner.rs`](../../../planner/src/planner.rs), `PlannedCandidate`, `lower_candidate`, and `ProgramPlannerSearch` |
| Fixed-point loop, pass count, observation validation, rejection, and Finalize | [`prepare/src/lib.rs`](../../../prepare/src/lib.rs), `Preparer::prepare_program_validated` |
| Artifact materialization and driver adapter | [`prepare/src/production.rs`](../../../prepare/src/production.rs), `NativeCandidateRealizer` and `NativeExecutorDriver` |
| Concrete host, CUDA, HSA, and bridge realization | [`local.rs`](../../src/local.rs), [`cuda.rs`](../../src/cuda.rs), [`hsa.rs`](../../src/hsa.rs), [`bridge.rs`](../../src/bridge.rs) |
| Reservation and capacity accounting | [`core/src/plan.rs`](../../../core/src/plan.rs), `ReservationLedger` and `CapacityLedger` |
| Finalized immutable bundle and resolved addresses | [`core/src/plan.rs`](../../../core/src/plan.rs), `FinalizedBundle::finalize_with_loop_schedule` |

## Lifecycle in one view

The production path is a single ownership chain. There is no lazy loader,
second compiler, or alternate post-Finalize realization path.

```text
public training or inference callback
  -> LocalCandidateFactory::production
  -> NativeExecutorDriver<F>
  -> ValidatedCandidateFactory<F>
  -> NativeCandidateRealizer::realize(candidate, catalog, reservations, policy)
  -> CandidateSessionFactory::realize_candidate(request)
  -> warm pass 1 -> capacity snapshot 1
  -> warm pass 2 -> capacity snapshot 2
  -> ... one pair per policy pass ...
  -> Preparer validates observation and stable tail
  -> final capacity arena packing and FinalizedBundle::finalize
  -> PreparedNativeSession::into_parts
  -> ValidatedCandidateSession::into_inner
  -> LocalPreparedSession::into_backend(finalized_bundle)
  -> LocalBackend bind and the immutable init -> loop -> exit run
```

`src/training.rs` constructs this chain for native training
(`execute_current_training_native`, lines 1285-1337), while `src/inference.rs`
constructs it for native inference (lines 606-655). The execution boundary
then consumes the successful product, moves the validated session through
`into_backend`, and only then admits input images. The inference handoff is at
`training/src/execute.rs:1228-1249`; the training handoff is at
`training/src/execute.rs:2205-2231`.

The state machine below is the concrete state of a
`LocalPreparedSession` (`local.rs:660-896`):

| State | Physical value | Legal next operation |
| --- | --- | --- |
| `Realized` | `LocalPreparedPhysical::Candidate` containing prepared host, CUDA, HSA, and bridge resources | warm pass `1` only, or deterministic destroy |
| `Warmed { pass }` | `LocalPreparedPhysical::Warm` containing bound warm resources, warm arenas, translated tasks, images, and exit buffers | one capacity snapshot for that `pass`, or destroy |
| `Observed { pass }` | same warm resources, with warm arenas released and the capacity snapshot anchored | next pass `pass + 1`, or final handoff, or destroy |
| `Transition` | ownership is being moved or torn down | no public lifecycle operation; a re-entry is a backend-state error |
| `Destroyed` | no candidate physical objects remain | destroy is idempotent; any other use is rejected |

The outer `NativeCandidateRealizer` repeats the `Warmed -> Observed` pair for
every `PreparationPolicy::stabilization_passes` value. The default is three
passes and a stable tail of two snapshots (`prepare/src/lib.rs:37-55`).

## Candidate data model

### Planner output

`PlannedCandidate` (`planner/src/planner.rs:61-71`) is an owned immutable
record containing:

* `draft: DraftPlan`, the exact profile identities, values, kernels, tasks,
  static artifacts, deferred `ArtifactBuildRecipe` values, resource manifest,
  arena objects, value bindings, aliases, init images, and releases;
* `arena_layouts`, the planner's optimistic offsets used only to describe the
  candidate before the post-warm repack;
* measured `makespan` and selected kernel `placements`;
* `stage_placements`, which map each deferred artifact to one measured device;
* `lowered_programs`, retained so deferred stages can be lowered again during
  Realize; and
* logical copies and exact external-output identities.

`plan_program_candidates` validates the graph, topology, discovery,
reservations, and optimistic capacity, enumerates every finite GPU placement,
lowers each assignment, schedules its calculation/transfer/metric tasks,
compacts submission resources, computes auxiliary staging and scratch peaks,
packs optimistic arenas, and validates the resulting `DraftPlan`
(`planner/src/planner.rs:222-356`, `979-1213`). Candidate identity is derived
from the graph digest, topology/discovery identities, and ordered kernel-to-
device assignment (`planner/src/planner.rs:746-764`). Valid candidates are
ranked by makespan, then candidate identity. `ProgramPlannerSearch` issues each
identity once and accepts rejection only for an issued identity
(`planner/src/planner.rs:73-169`).

The planner keeps a static artifact only when its identity exactly matches the
deferred stage's kernel template, build provenance, resource envelope, and
measured target. Otherwise the Draft contains a target-independent deferred
build (`planner/src/planner.rs:1330-1400`, `select_or_defer_artifact`). A Draft
therefore has two disjoint artifact domains:

```text
draft.artifacts       = already built, immutable identities
draft.artifact_builds = deferred stage contracts, no runtime image yet
```

`DraftPlan::validate` rejects duplicate IDs and an artifact that appears in
both domains (`core/src/plan.rs:232-421`). It also requires every calculation
to be on a GPU storage device, requires loop phase for calculations and
metrics, validates transfer phase and route shape, validates submission slots,
and checks that deferred bindings, work, fault flags, and resident values
agree (`core/src/plan.rs:420-760`). These checks run before a native driver is
asked to allocate anything.

### Runtime artifact pair

`CandidateArtifact` (`candidate.rs:23-41`) is the exact pair sent to the
executor boundary:

```text
CandidateArtifact {
    identity: ArtifactIdentity, // serialized semantic identity
    runtime: RuntimeArtifact,    // bytes, digest, ABI, and runtime kind
}
```

`CandidateArtifact::new` is a `const` constructor and intentionally does not
validate the pair. Validation is centralized in
`CandidateRealizationRequest::validate`, so every factory implementation can
be wrapped by `ValidatedCandidateFactory`.

`NativeCandidateRealizer` first materializes all pairs in
`DeferredArtifactCompiler::materialize` (`prepare/src/production.rs:310-377`):

1. Every selected static Draft identity is looked up in the exact catalog and
   compared byte-for-byte as an identity.
2. Every deferred build is assigned to the single measured target found from
   its calculation placements. It is lowered from the exact retained program
   digest with entry `recipe_stage_<artifact-id>`.
3. A prebuilt cubin or HSACO is structurally inspected, or the pinned builder
   emits the bundle in `BuildPhase::Realize`.
4. The resulting image digest, ABI, target, toolchain, resource envelope,
   kernel template, and build provenance form one `NativeArtifact`
   (`prepare/src/production.rs:549-575`).
5. The sorted result has no duplicate artifact IDs. The driver adapter copies
   each identity/runtime pair into an executor `CandidateArtifact`
   (`prepare/src/production.rs:845-863`).

The executor's runtime validator (`native-executor/src/plan.rs:261-352`)
checks all of the following for each pair: runtime ID equals identity ID,
image digest equals the semantic digest, ABI entry equals the identity entry,
format equals target ABI, workgroup lanes are nonzero and within the finalized
maximum, and the runtime kind agrees with the exact CUDA or HSA backend,
architecture, ABI, target digest, and code-object version. This is separate
from the native loader's structural inspection.

## Request validation

`CandidateRealizationRequest<'a>` (`candidate.rs:43-68`) borrows the topology,
discovery profile, planned candidate, exact runtime artifact pairs, and
reservation ledger. `validate()` is ordered and fail-fast:

```text
topology.validate()
discovery.validate(topology)
candidate.draft.validate(topology, discovery)
reservations.validate(topology)
validate_artifacts(request)
```

The first four failures retain their `ValidationErrors` as
`InvalidTopology`, `InvalidDiscovery`, `InvalidDraft`, or
`InvalidReservations`. `validate_artifacts` then enforces the exact artifact
set and the static/deferred contracts.

### Exact artifact-set algorithm

The implementation at `candidate.rs:475-552` is equivalent to this parseable
algorithm:

```text
static  := map draft.artifacts      by artifact.id
builds  := map draft.artifact_builds by build.artifact
expected := keys(static) union keys(builds)
reject if expected.size != static.size + builds.size

observed := {}
for pair in request.artifacts:
    identity := pair.identity
    reject InvalidArtifact if identity.validate() fails
    reject DuplicateArtifact if identity.id is already in observed

    if identity.id is in static and not in builds:
        reject StaticArtifactMismatch unless identity == static[id]
    else if identity.id is in builds and not in static:
        validate_built_identity(request, builds[id], identity)
    else:
        reject UnexpectedArtifact

    reject RuntimeArtifact if validate_runtime_artifact(identity, pair.runtime) fails
    insert identity.id in observed

if observed != expected:
    reject MissingRuntimeArtifact for the first missing expected ID,
    otherwise reject RuntimeArtifactSetMismatch
```

`DraftPlan::validate` normally makes the overlap and duplicate-map branches
unreachable for a planner-produced Draft, but `CandidateRealizationRequest`
rechecks them at the native boundary. No artifact is silently ignored, and a
runtime image is never accepted merely because its ID is present.

### Deferred identity algorithm

`validate_built_identity` (`candidate.rs:554-612`) applies checks that static
artifacts do not need:

1. `kernel_template`, `resources`, and `build` must equal the
   `ArtifactBuildRecipe`'s template, envelope, and provenance. Otherwise the
   error is `DeferredArtifactMismatch { artifact, build }`.
2. The candidate must contain exactly one immutable `stage_placement` for that
   artifact. Absence is `MissingStagePlacement`.
3. The placement device must occur in discovery and have a calculation
   capability. Absence of either is `MissingGpuCapability`.
4. The realized identity target must equal the measured calculation target on
   that placement. Otherwise the error is `TargetMismatch`.

This is the final check that a newly compiled stage was built for the measured
device selected by this candidate, not merely for some available target.

### Request error table

| Error | Exact condition |
| --- | --- |
| `InvalidTopology` | `Topology::validate` fails. |
| `InvalidDiscovery` | `DiscoveryProfile::validate(topology)` fails. |
| `InvalidDraft` | The candidate Draft is not valid for the supplied topology and discovery identities. |
| `InvalidReservations` | The ledger has missing, duplicate, wrong-kind, or wrong-size entries. |
| `ArtifactSetOverlap` | A Draft ID occurs in both static artifacts and deferred builds. |
| `InvalidArtifact` | A semantic artifact identity fails `ArtifactIdentity::validate`. |
| `DuplicateArtifact` | The request supplies the same runtime artifact ID twice. |
| `StaticArtifactMismatch` | A supplied static identity differs from the immutable Draft identity. |
| `UnexpectedArtifact` | A supplied ID is absent from the Draft, or is in both Draft domains. |
| `RuntimeArtifact` | The runtime bytes, digest, ABI, target, or runtime kind fail executor validation. |
| `MissingRuntimeArtifact` | An expected static or deferred ID was not supplied. |
| `RuntimeArtifactSetMismatch` | The observed set differs without a single first missing ID being available. |
| `DeferredArtifactMismatch` | A deferred identity does not prove template, resources, and build provenance. |
| `MissingStagePlacement` | A deferred artifact has no candidate stage placement. |
| `MissingGpuCapability` | The placement device is absent or has no measured calculation capability. |
| `TargetMismatch` | Deferred identity target differs from the placement's measured target. |

`CandidateRequestError` implements `Display` with these stable meanings and
exposes validation errors or the wrapped runtime `crate::Error` through
`std::error::Error::source` (`candidate.rs:114-208`).

## Factory contract and failure classes

`CandidateSessionFactory` (`candidate.rs:243-278`) has five operations:

| Method | Contract |
| --- | --- |
| `reservation_evidence(device)` | Return live evidence for the exact storage device before candidate resources are realized. |
| `realize_candidate(request)` | Load and inspect every exact artifact, allocate every exact reservation and resource-manifest object, and return a session with no deferred initialization. |
| `warm_maximum_concurrency(session, candidate, pass)` | Execute one complete maximum-concurrency trace for this exact candidate. The pass is complete only when all pending work reaches terminal state. |
| `capacity_snapshot(session, topology, discovery)` | Observe capacity only after a complete warm pass. The snapshot must describe the same immutable profile and reservations. |
| `destroy_candidate(session)` | Deterministically release every partially or fully realized object. The operation consumes the session. |

The trait bounds require a debuggable session and an error that is `Send`,
`Sync`, and a standard error. The trait deliberately has no `bind` method:
finalized binding is owned by the session implementation after the outer
preparation fixed point succeeds.

`CandidateFailure<E>` (`candidate.rs:210-240`) has three meanings:

| Variant | Preparation behavior |
| --- | --- |
| `CandidateRejected { detail }` | The current finite candidate is invalid or does not fit. The realizer must have no live session when this is returned, or must destroy it before returning. `Preparer` records a retryable rejection and asks the one-shot planner for the next candidate. |
| `Fatal(E)` | A driver, arithmetic, profile, or teardown failure makes the complete preparation invalid. `Preparer` aborts instead of guessing or falling back. |
| `PreFinalRealizationUnavailable { detail }` | No implementation can satisfy the ahead-of-run contract. The production adapter maps this to a fatal native preparation error. |

`UnavailableCandidateFactory` is the explicit fail-closed implementation
(`candidate.rs:414-472`). Its reservation query returns
`UnavailableCandidateError`; realization, warm, and capacity always return
`PreFinalRealizationUnavailable` with `no native candidate factory was
configured`; destroying its unit session succeeds. It never pretends that a
post-Finalize backend is equivalent to a pre-final session.

## Validation wrapper

`ValidatedCandidateFactory<F>` (`candidate.rs:280-412`) is the adapter used by
`NativeExecutorDriver` (`prepare/src/production.rs:797-904`). It stores no
alternate resources. Its `realize_candidate` performs request validation,
clones the candidate, topology/discovery identities, and reservations into a
`ValidatedCandidateSession<S>`, then delegates exactly once to `F`.

The wrapper adds checks that a physical factory cannot safely infer from its
own opaque session:

* warm pass `0` is rejected; valid pass numbers start at `1`;
* the warm call's `PlannedCandidate` must compare equal to the candidate stored
  in the session; and
* the capacity call's topology and discovery identities must equal the stored
  identities, and the returned `CapacityLedger` must validate against the
  stored reservations.

The wrapper forwards reservation evidence and destruction without changing
the evidence. `ValidatedCandidateSession::inner`, `inner_mut`, and
`into_inner` are ownership accessors, not duplicate lifecycle operations.

`NativeExecutorDriver` converts `prepare`'s `NativeArtifact` pairs into
executor `CandidateArtifact` pairs, maps executor `CandidateFailure` to
`CandidateDriverFailure`, and reports `ReservationMechanism::EnforcedQuota`
for every device (`prepare/src/production.rs:821-904`).

## Native preparation caller and rejection flow

`NativeCandidateRealizer::realize` (`prepare/src/production.rs:979-1073`) is
the only production implementation of `CandidateRealizer`:

1. It requires the candidate Draft's topology and discovery identities to equal
   the profile captured by the realizer.
2. It validates the reservation ledger and materializes all static and deferred
   artifact pairs.
3. It sends one request to `CandidateDriver::realize_candidate`.
4. For `pass` in `1..=policy.stabilization_passes`, it calls warm and then
   capacity snapshot. A failure after a session exists goes through
   `reject_after_session`, which destroys the session before mapping the
   failure. A destroy failure becomes `NativePrepareError::Teardown` and is
   fatal (`prepare/src/production.rs:1132-1146`).
5. On success it returns `PreparedNativeSession { driver_session, artifacts }`
   plus `RealizationObservation { artifacts, resources, reservations,
   capacity_snapshots }`.

`Preparer::prepare_program_validated` (`prepare/src/lib.rs:340-525`) surrounds
that call with the fixed point:

```text
reservations := realizer.reservation_plan(topology, discovery)
planning_capacity := measured total - exact reservation
catalog := artifact_provider.resolve(graph, topology, discovery)
search := plan_program_candidates(..., planning_capacity)

for candidate in one_shot(search):
    realized := realizer.realize(candidate, catalog, reservations, policy)
    if CandidateRejected:
        reject candidate and continue
    if Fatal:
        abort preparation

    capacity := validate_observation(realized.observation, policy)
    if invalid:
        destroy(realized.session)
        reject candidate and continue

    layouts := pack_arenas(topology, candidate.draft.arena_objects, capacity)
    if packing fails:
        destroy(realized.session)
        reject candidate and continue

    realization := hash and validate the unchanged Draft plus observation
    bundle := FinalizedBundle::finalize_with_loop_schedule(...)
    return PreparedSystem(bundle, realization, catalog, realized.session)
```

The preparation crate expects exactly one snapshot per pass, validates every
snapshot against topology and reservations, and requires every snapshot in the
configured stable tail to equal the final snapshot (`prepare/src/lib.rs:608-670`).
An unstable tail is a `Stabilize` rejection. Final arena packing against the
post-warm capacity is a `FinalCapacity` rejection. Finalize errors are fatal
for the unchanged candidate and do not cause an implicit Draft mutation.

## Concrete local factory

### Construction and reservation evidence

`LocalCandidateFactory` (`local.rs:997-1071`) stores the optional
`HostBackendConfig`, exact borrowed CUDA and HSA bindings, one bridge value,
the selected stabilizer, and one optional `InitialCapacitySnapshot`.

* `LocalCandidateFactory::fail_closed` selects
  `UnavailableLocalStabilizer`.
* `LocalCandidateFactory::production` selects `NativeLocalStabilizer`, which
  calls the real warm trace and capacity observer.
* `LocalCandidateFactory::new` does not allocate or query capacity.

`reservation_evidence_for_device` (`local.rs:2145-2185`) resolves one owner
for each device. Host RAM/disk bindings report `ReservationEvidence::NonGpu`.
CUDA and HSA bindings report `GpuDisplay { enabled_connectors }`. Duplicate
owners are `LocalError::DuplicateDevice`, and an absent owner is
`LocalError::MissingDevice`.

`NativeCandidateRealizer::reservation_plan` turns this evidence into a
`ReservationEntry` named `recipe-user-<device>` and uses the adapter's
`EnforcedQuota` mechanism (`prepare/src/production.rs:1076-1115`). Core
reservation validation requires every topology device exactly once. The byte
policy is exact: RAM/disk and display-connected GPUs retain
`1_000_000_000` bytes; a GPU with zero enabled display connectors has an
explicit zero-byte exemption (`core/src/plan.rs:23-126`).

At the first realization for a factory/profile,
`capture_initial_capacity` queries each owner exactly once. Host capacity comes
from `MemAvailable` or `statvfs`, CUDA from the reopened driver's free-memory
counter, and HSA from the session's available-memory counter
(`local.rs:2187-2244`, `host/src/backend.rs:95-107`, `cuda.rs:65-85`,
`hsa.rs:71-89`). A later candidate for another topology or discovery identity
is rejected as `LocalError::BackendState`, not silently recaptured. The
reservation must fit that immutable initial snapshot
(`local.rs:2246-2265`).

### Candidate classification

`classify_candidate` (`local.rs:2291-2394`) maps every candidate device and
task to one owner. The candidate's arena-layout device set must equal the
declared host/CUDA/HSA binding set. Duplicate, missing, or extra devices are
rejected. Task ownership is:

| Draft task | Owner rule |
| --- | --- |
| Calculation | CUDA or HSA according to the calculation device. Host and missing owners are `UnsupportedCalculation`. |
| Metric | The backend owning the metric value's resident device. |
| External admission or egress | The backend owning its one device endpoint. |
| Same-device internal transfer | Host, CUDA, or HSA according to both endpoint classes. |
| Cross-device internal transfer | `Bridge`, including peer CUDA transfers between distinct CUDA devices. |

Executor-visible routes may contain zero or one link. A bridge task must be a
transfer with exactly one link. Multi-hop routes, calculations or metrics sent
to the bridge, and external-to-external transfers are rejected. Queue and
completion slots may not be shared by different local owners
(`local.rs:2512-2558`). This prevents one physical submission object from
being realized in two backend partitions.

`partition_candidate_artifacts` (`local.rs:2396-2453`) derives the artifact
IDs used by CUDA and HSA calculation tasks. One runtime image cannot be
assigned to both native backends. Every supplied runtime artifact must be
consumed by exactly one native backend, and every required ID must be present.

### Realization order and partial cleanup

`LocalCandidateFactory::realize_candidate` (`local.rs:1422-1547`) performs this
ordered sequence:

1. Validate the complete request again. A request failure is a retryable
   `CandidateRejected` with no session returned.
2. Build declared-device partitions and capture or reuse the immutable initial
   capacity for the exact profile.
3. Validate initial reservation headroom and partition runtime artifact IDs.
4. If configured, call `HostBackend::prepare_candidate`, which validates host
   task membership and bindings, requires scheduler-enforced quota, creates the
   host runtime, and prepares one pending copy/staging token per selected task
   (`host/src/backend.rs:452-500`, `1390-1480`). Host partitions contain only
   transfers and metrics, never payload calculations.
5. Call `CudaPreparedResources::realize` with CUDA artifacts, reservations,
   cloned bindings, and CUDA task IDs. It validates binding identity, queue
   limits, quota mechanism, init-image staging, artifact kind/target/ABI,
   loads each distinct cubin image once, resolves one function per artifact,
   and creates invocation storage, metric buffers, egress buffers, staging,
   optional scratch, queues, and completion objects
   (`cuda.rs:1001-1086`, `1640-1905`).
6. Call `HsaPreparedResources::realize` with HSA artifacts, reservations,
   cloned bindings, and HSA task IDs. It validates the CPU kernarg/fine pools,
   exact target and code-object version, quota mechanism, queue limits, and
   init-image staging; loads each distinct HSACO image once, resolves symbols,
   creates kernarg slots, metric buffers, egress buffers, fine staging,
   optional scratch, queues, completion states, and a reusable pending pool
   (`hsa.rs:1159-1248`, `1900-2055`).
7. Clone the bridge and call `CandidateCrossBackendTransfer::realize_candidate`
   for every bridge task. `StagedCrossBackend` creates exact endpoint staging,
   CUDA stream/event or HSA pending resources, and one host staging worker per
   task (`bridge.rs:253-333`).
8. Return `LocalPreparedSession` in `Candidate` physical state with
   `StabilizationState::Realized`.

If CUDA preparation fails, host prepared resources are destroyed. If HSA
preparation fails, HSA's partial result is returned as an error and CUDA then
host are destroyed. If bridge preparation fails, bridge cleanup is attempted
after HSA, CUDA, and host cleanup. Cleanup helpers retain the first failure and
drop later cleanup errors (`local.rs:3368-3428`). No partially allocated native
object is hidden behind a successful candidate rejection.

### Resource manifests

The candidate resource manifest is planner-owned and immutable. The local
factory realizes the physical objects named by that manifest:

* Host resources own a bounded worker runtime, per-task copy slots, optional
  RAM staging for init/metric/egress operations, host bindings, and a pending
  pool. Disk arenas are opened later with the caller's exact run-scoped path.
* CUDA `DeviceResources` own the reopened context, measured submission streams,
  completion events, one loaded module per distinct cubin digest, one logical
  function per artifact entry, per-completion parameter blocks, pinned metric
  buffers, pinned staging, init admission contract, fixed egress vectors, and
  optional device scratch (`cuda.rs:160-218`).
* HSA `DeviceResources` own the reopened session and exact CPU allocator,
  submission queues, completion state, one executable per distinct HSACO
  digest, logical kernels, kernarg allocations and host bytes, fine-grained
  metric/staging allocations, init admission, fixed egress vectors, and
  optional coarse scratch (`hsa.rs:150-260`). HSA finalized binding additionally
  computes and stores each loaded kernel's resource envelope.
* `StagedBridgeResources` owns a transfer contract, source and destination
  legs, prepared terminal tokens, and one `HostStageWorker` for each selected
  cross-backend task (`bridge.rs:643-655`). CUDA legs use pinned host buffers,
  nonblocking streams, and completion events. HSA legs use fine-grained host
  allocations and pre-created ROCr pending tokens. Host legs use no separate
  staging leg.

Every object above is created before Finalize. The runtime loop receives only
the moved resources and finalized arena views.

## Warm maximum-concurrency trace

### Pass ordering

`LocalCandidateFactory::warm_maximum_concurrency` accepts only the expected
next pass and exact candidate (`local.rs:1549-1573`):

```text
Realized                -> expected pass 1
Observed { pass: n }    -> expected pass n + 1
Warmed { .. }           -> no warm call allowed before its snapshot
```

Pass `0`, a skipped pass, a repeated pass, or a different candidate returns
`CandidateRejected`. `ValidatedCandidateFactory` enforces the same pass and
candidate identity checks before delegating (`candidate.rs:356-380`).

`NativeLocalStabilizer` calls `LocalPreparedSession::execute_warm_pass`.
`UnavailableLocalStabilizer` instead returns
`PreFinalRealizationUnavailable`, so the factory cannot accidentally claim
stability without a real trace (`local.rs:919-995`).

### Activating candidate resources

The first warm call changes `Candidate` to `Warm` in
`activate_warm_resources` (`local.rs:1134-1221`):

1. `provisional_warm_bundle` creates a temporary address-resolved bundle from
   the exact Draft, supplied artifact identities, reservations, and measured
   discovery totals minus reservations (`local.rs:2598-2653`). It is only a
   warm contract. It does not alter the Draft or create finalized offsets.
2. Host, CUDA, and HSA prepared resources are consumed by `bind_candidate`.
   They validate task sets, device bindings, init-image admission contracts,
   execution plans, and runtime artifact sets against the provisional bundle.
3. The already realized bridge resource is retained. No second bridge
   allocation is performed.
4. The session builds `LocalResources`, translates every Draft task to
   `WarmTask`, allocates zeroed init-image buffers and fixed external-exit
   buffers, and starts with an empty warm-arena map.

The child prepared-resource implementations reject a finalized handoff being
used as a warm candidate. This keeps resource ownership monotonic: prepared
candidate objects become warm resources once, then warmed resources become
final backend states once.

### Warm task translation

`prepare_warm_tasks` (`local.rs:2655-2752`) converts the provisional bundle:

| Draft shape | `WarmWork` |
| --- | --- |
| Init `External -> Device` transfer | `Init`, carrying device destination, bytes, and submission slots. |
| Loop calculation | `Calculation`, carrying device, kernel template, artifact, submission slots, resolved input/output locations, and optional fault-flag location. |
| Loop metric | `Metric`, carrying metric purpose, ID, slot, resolved value location, and submission slots. |
| Init/loop internal transfer | `Transfer { class: InternalTransfer }`, with resolved endpoints, bytes, route, lane claims, and submission slots. |
| Exit transfer | `Transfer { class: ExitTransfer }`, with the same resolved contract. |

Calculations and metrics in init or exit are illegal. A missing resolved value
or transfer endpoint is `TaskNotAssigned`; an init or exit image that does not
fit host address space is a capacity or backend-state failure.

### Scheduler behavior

`run_warm_trace` (`local.rs:2844-3036`) runs one `RunId` equal to the pass and
one loop iteration. For each lifecycle phase in `Init`, `Loop`, `Exit`, it
repeatedly:

1. Marks a task runnable only when it has the current phase, all dependencies
   are complete, and its schedule window does not conflict with a still-pending
   task.
2. Creates one pending token from the owning host, CUDA, HSA, or bridge pool.
3. Converts `WarmWork` into a closed `BackendWork` value and submits it against
   the exact warm arena map.
4. Polls every pending task. On terminal completion, collects external exit
   bytes into the preallocated exit image when applicable, recycles the token,
   and marks the task complete.
5. Fails if no runnable or pending task can progress, or after
   `10_000_000` idle polls. Idle polling backs off from 50 microseconds to a
   two-millisecond cap. These are scheduler polling controls, not retry paths.

The warm trace therefore exercises real maximum overlap, queue and completion
ownership, bridge worker staging, native module entry points, metric readback,
init admission, and exit egress. It does not replace any layer with a mock or
synthetic task.

For each task owner, `prepare_warm_pending`, `submit_warm_work`,
`poll_warm_pending`, `collect_warm_exit`, and `recycle_warm_pending` enforce
the owner map and pending-token identity (`local.rs:3020-3230`). A mismatched
owner, task, route, class, submission slot, or endpoint is a real
`LocalError`, not a fallback to another backend.

After `execute_warm_pass` returns, the factory records
`StabilizationState::Warmed { pass }`. It cannot accept a capacity snapshot
until the complete trace has returned.

## Capacity observation and stabilization

`LocalCandidateFactory::capacity_snapshot` checks profile identities and state
before delegating (`local.rs:1575-1605`). It accepts only
`StabilizationState::Warmed { pass }`; calling it in `Realized` or `Observed`
returns `CandidateRejected` with `capacity snapshot requires one new complete
warm pass`.

`LocalPreparedSession::observe_capacity` first removes every warm arena and
releases it through its owning host, CUDA, or HSA allocator
(`local.rs:1223-1259`). This ordering is essential: the snapshot includes
persistent runtime overhead and auxiliary objects, not a transient candidate
arena allocation that will not survive Finalize.

`anchor_capacity_snapshot` stores the first successful observation and returns a
clone on later passes (`local.rs:3262-3277`). Later operating-system, display,
or allocator-counter drift therefore cannot rewrite the scheduler contract.

`observe_capacity_ledger` checks, for every topology device:

1. the observation profile identities equal the immutable initial snapshot;
2. discovery has a capacity entry;
3. the initial available byte count exists;
4. an exact reservation entry exists;
5. the local owner has realized resources; and
6. the backend's live available counter can be read.

It then computes:

```text
capped_live = min(live_available, initial_available)
runtime_overhead = initial_available - capped_live
recipe_usable = capped_live - reservation.bytes
```

Underflow is `CapacityMismatch`. Each result is measured, with zero
fragmentation and zero additional safety-headroom fields
(`local.rs:3279-3366`). The wrapper validates the resulting ledger against the
same topology and reservations (`candidate.rs:396-406`). Core capacity
validation additionally requires measured or explicitly schedulable fields,
all required devices exactly once, and accounted bytes no greater than total
(`core/src/plan.rs:134-187`).

The outer preparer receives one ledger per pass. It rejects a wrong count,
invalid entry, or unstable configured tail. Only the final stable ledger is
used by `pack_arenas`; the capacity used during planner enumeration is never
treated as runtime evidence.

## Final handoff and bind

### Local session validation

`LocalPreparedSession::into_backend` (`local.rs:1262-1323`) first calls
`validate_handoff`. Handoff requires:

* stabilization state `Observed`, not merely `Realized` or `Warmed`;
* finalized topology, discovery, Draft, and candidate identities equal to the
  session's exact values;
* finalized tasks, kernels, artifact builds, resource manifest, init images,
  reservations, and artifact identities equal to the candidate and loaded
  images (`validate_prepared_identity`, `local.rs:3430-3479`);
* all warm arenas already released;
* bridge `validate_handoff` accepts the exact bridge task set, device classes,
  finalized endpoint contracts, route, lane claims, and submission slots; and
* host, CUDA, and HSA child resources validate their pending pools, admission
  manifests, execution plans, artifact ABIs, and backend contracts.

If any validation fails, `into_backend` destroys the session and returns the
validation error unless teardown itself fails, in which case the teardown
error is returned. This prevents a mismatched FinalizedBundle from leaving
live candidate resources behind.

On success, `into_backend` moves the same `LocalResources` into:

```text
LocalBackend {
    host: HostBackend::from_warmed(host),
    cuda: CudaBackend::from_warmed(cuda),
    hsa: HsaBackend::from_warmed(hsa),
    bridge: PreparedBridge {
        resource: Available(existing_bridge_resource),
        handoff_validated: true,
        exact task/device partitions,
    },
}
```

`PreparedBridge::bind` checks the exact task and device partitions, requires
`handoff_validated`, replaces `Available` with `Consumed`, and returns the
existing bridge resource. A second bind is `prepared bridge resource was
already consumed` (`local.rs:504-633`).

### Composite finalized bind

`LocalBackend::bind_resources` (`local.rs:1647-1701`) records one composite
`PhysicalCall::BindResources`, classifies the FinalizedBundle again, and calls
host, CUDA, HSA, and bridge `bind_partition`/`bind` exactly once. Each child
backend is already in `Warmed` state, so it validates the finalized contracts
and returns the same physical resources. It does not load a second module,
create a second queue, allocate a second pending pool, or recreate a bridge.

The host, CUDA, and HSA state machines all replace their state with `Bound`
before dispatch. A repeat bind therefore fails closed. Their warm handoff
checks include:

* host pending pool keys equal the final host task set, every warm admission
  contract equals the finalized init image, and all warm tokens were recycled
  (`host/src/backend.rs:1064-1108`);
* CUDA pending tokens are all recycled, every device's admission manifest is
  unchanged, and the final partition execution plan/contracts validate
  (`cuda.rs:960-980`); and
* HSA pending tokens are all recycled, admission manifests are unchanged,
  finalized artifact resource envelopes are bound to the already loaded
  kernels, and the final partition plan/contracts validate
  (`hsa.rs:1114-1138`).

Only after this handoff does the executor allocate finalized packed arenas.
`LocalBackend::allocate_arena` routes each final `ArenaLayout` to the matching
host, CUDA, or HSA resource. Candidate warm arenas were intentionally released
before the handoff, so these are the one final arena allocations described by
the immutable bundle, not a duplicate candidate copy.

## Destruction and cleanup

`LocalPreparedSession::destroy` consumes the session after replacing its
physical state with `Transition` (`local.rs:1376-1419`):

* In `Candidate`, it destroys bridge, HSA, CUDA, then host prepared resources.
* In `Warm`, it first releases every warm arena, then destroys bridge, HSA,
  CUDA, then host warmed resources.
* In `Transition`, it reports a backend-state error.
* In `Destroyed`, it succeeds.

The first error is retained and later cleanup errors are dropped. The same
bridge/HSA/CUDA/host order is used by `destroy_warm_resources`
(`local.rs:3230-3260`). Native child teardown refuses to destroy an active
completion or stream, so an incomplete warm trace remains visible as the real
failure rather than being hidden by a broad reset.

The outer rejection paths are:

| Failure point | Required cleanup and result |
| --- | --- |
| Factory request validation or realization before a session is returned | Factory cleans every partial object and returns `CandidateRejected` or `Fatal`. |
| Warm or capacity failure after a session exists | `NativeCandidateRealizer::reject_after_session` calls `destroy_candidate`; a clean destroy preserves the retryable classification, while a destroy failure is fatal teardown. |
| Observation count, invalid snapshot, or unstable stable-tail | `Preparer` destroys the successful session, records a `Stabilize` rejection, and tries the next finite candidate. |
| Final arena packing failure | `Preparer` destroys the session, records `FinalCapacity`, and tries the next candidate. |
| Finalize failure for an unchanged candidate | `Preparer` destroys the session and returns `PrepareErrorKind::Finalization`; no Draft mutation or fallback is attempted. |
| Handoff identity or child contract mismatch | `into_backend` destroys the session and returns the validation or teardown error. |

`PreparedNativeSession::into_parts` consumes only the wrapper and preserves the
same opaque driver session plus immutable artifact images
(`prepare/src/production.rs:906-932`). `PreparedSystem::into_parts` likewise
keeps the finalized bundle, realization profile, catalog, and session together
until execution consumes them. There is no safe API that extracts a warm
resource and then constructs a different bundle around it.

## Failure inventory

The local implementation's `LocalError` (`local.rs:329-430`) is the physical
failure vocabulary beneath `CandidateFailure<LocalError<_>>`:

| Error | Meaning at the candidate boundary |
| --- | --- |
| `DuplicateDevice` | More than one backend binding owns a device, or duplicate arena ownership was observed. |
| `MissingDevice` | A required candidate device or task endpoint has no declared owner. |
| `UnexpectedDevice` | A declared binding has no candidate/finalized arena. |
| `UnsupportedCalculation` | A calculation is assigned to host, bridge, or a missing owner. |
| `UnsupportedRoute` | A route is multi-hop at executor visibility, bridge shape is invalid, or an external endpoint is used where it is forbidden. |
| `TaskNotAssigned` | A task, value, or submission contract cannot be mapped to one owner. |
| `ArenaOwnerMismatch` | A warm or final arena's device/class differs from its immutable owner. |
| `PendingOwnerMismatch` | A pending token is submitted, collected, or recycled for another task/owner. |
| `CapacityMismatch` | Initial/live capacity, reservation, staging, or address-space evidence is absent or underflowed. |
| `BackendState` | A lifecycle phase was skipped, repeated, or entered after transition/destroy. |
| `PhysicalAccountingOverflow` | The composite physical-call batch cannot record the required operation. |
| `Host` | Host runtime, copy, worker, staging, disk, or host arena failure. |
| `Native` | CUDA Driver or ROCr/HSA resource, ABI, queue, completion, or native memory failure. |
| `Bridge` | Cross-backend staging worker, endpoint, token, or native copy failure. |

These errors are not alternative routing signals. The owner map is derived
from the active candidate or finalized bundle, so an invalid event has no valid
watcher and is rejected at the point where it would otherwise cross the
boundary.

## Invariants to preserve

The following statements are the contract of this module and its concrete
factory. A change is incomplete if any statement becomes false:

1. One request names one validated topology, discovery profile, Draft,
   reservation ledger, and exact runtime artifact set.
2. Static Draft artifact identities are immutable. Deferred identities prove
   their exact build provenance, stage placement, measured target, runtime
   digest, ABI, and backend kind.
3. A successful `realize_candidate` has loaded every required artifact and
   created every queue, completion object, staging allocation, scratch object,
   pending pool, host worker, and resource-manifest object needed by the
   candidate. No deferred initialization is allowed after that call.
4. Every warm pass uses the exact candidate, starts at pass one, exercises all
   lifecycle phases and maximum overlap, recycles every terminal token, and
   releases its temporary arenas before capacity is observed.
5. Capacity snapshots are measured from the same initial profile and exact
   reservations. The first complete post-warm snapshot is anchored, and only
   a stable configured tail can enter final arena packing.
6. Finalization never changes Draft choices. It resolves offsets and identities
   against the measured post-warm ledger only after candidate observation.
7. Final bind consumes the same warmed resources exactly once. It validates
   bundle, task, device, artifact, admission, pending-pool, and bridge
   identities before moving ownership, and it never realizes a second copy.
8. Every rejection destroys live resources before another finite candidate is
   issued. Teardown failures remain fatal and visible.
9. The run lifecycle after bind can perform only finalized init admission,
   scheduled loop work, finalized exit egress, arena release, and ordered
   resource destruction. Discovery, compilation, loading, placement, routing,
   resizing, and replanning are pre-final operations.

## Source map for maintenance

When changing this boundary, inspect these ranges together:

* `candidate.rs:23-68`, request and pair shape;
* `candidate.rs:71-240`, request and failure errors;
* `candidate.rs:243-412`, factory trait and validation wrapper;
* `candidate.rs:475-612`, exact artifact and deferred-target validation;
* `prepare/src/production.rs:310-377`, artifact materialization;
* `prepare/src/production.rs:724-904`, driver boundary and failure mapping;
* `prepare/src/production.rs:979-1146`, pass loop and teardown after live failure;
* `prepare/src/lib.rs:340-525`, candidate search, observation, repack, and
  Finalize;
* `local.rs:1422-1608`, concrete factory realization, warm order, and capacity
  order;
* `local.rs:1073-1419`, warm activation, observation, handoff, and destruction;
* `local.rs:2291-2558`, owner and artifact partitioning;
* `local.rs:2598-3366`, provisional warm bundle, warm scheduler, release, and
  capacity accounting;
* `local.rs:3430-3479`, finalized identity validation;
* `cuda.rs:1001-1190`, CUDA prepared-resource lifecycle;
* `hsa.rs:1159-1378`, HSA prepared-resource lifecycle; and
* `bridge.rs:253-333` plus `bridge.rs:1293-1355`, staged bridge realization and
  handoff.
