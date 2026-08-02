# `recipe-prepare`

`recipe-prepare` is Recipe's ahead-of-run fixed-point boundary. It takes one
validated measured profile and one static calculation program, enumerates the
finite planner candidates, realizes each candidate against native resources,
waits for opaque capacity to stabilize, and finalizes the first candidate that
still fits. The result is an immutable `FinalizedBundle` paired with the exact
physical session that produced its realization evidence.

The normative meaning of this boundary is system contract C9 in
[`system-contract.md`](../../../system-contract.md): discovery, drafting,
placement, routes, geometry, resource bounds, native loading, warmup,
stabilization, capacity recording, and final arena packing all happen before
the run's `init -> loop -> exit` lifecycle. `init` may bind the already
realized session and admit fixed images, but it cannot replan. C10 permits
artifact compilation in the Realize portion only; `Finalize`, `init`, `loop`,
and `exit` cannot compile or JIT.

## Crate boundary and module layout

The crate root is [`prepare/src/lib.rs`](../../src/lib.rs). It forbids unsafe
code and requires `Debug` implementations for public types. The root owns the
generic fixed-point protocol, error aggregation, capacity validation, identity
hashes, and `Preparer`. The only private module is
[`prepare/src/production.rs`](../../src/production.rs), which supplies the
native artifact and candidate-driver implementation used by the production
local path.

The root imports the domain types from `recipe-core`, graph and program types
from `recipe-language` and `recipe-program`, finite candidate planning from
`recipe-planner`, measured profiles from `recipe-probe`, arena packing from
`recipe-scheduler`, and SHA-256 for canonical realization and bundle
identities. It re-exports the production module's public types, so users of
`recipe_prepare` do not need to reach into `production`.

The public production re-exports are:

- artifact inputs and policies: `NativeArtifact`, `NativeArtifactCatalog`,
  `NativeArtifactProvider`, `CudaArtifactPolicy`, `RuntimeArtifactPolicy`,
  `TargetBuildSpec`, and `DeferredArtifactCompiler`;
- the dependency-neutral driver boundary: `CandidateRealizationRequest`,
  `CandidateDriver`, `CandidateDriverFailure`, `NativeExecutorDriver`;
- the retained native session and generic adapter:
  `PreparedNativeSession`, `NativeCandidateRealizer`, and
  `NativePrepareError`.

There are no alternate preparation modules or post-finalization realization
paths in this crate.

## Generic preparation API

### Policy and observations

`DEFAULT_STABILIZATION_PASSES` is three and `DEFAULT_STABLE_TAIL` is two.
`PreparationPolicy` stores both values as `NonZeroU32`. `Default` installs
those constants. `with_policy` rejects a `stable_tail` larger than the total
pass count with `PrepareErrorKind::InvalidPolicy`; conversions to `usize` are
checked later when a backend allocates the snapshot vector.

`RealizationObservation` is the evidence returned by a successful realizer:

- the realized `ArtifactIdentity` values;
- the candidate's `ResourceManifest`;
- the `ReservationLedger` used for the candidate; and
- exactly one post-warm `CapacityLedger` per configured stabilization pass.

`RealizedCandidate<S>` keeps that observation beside the opaque session `S`.
The session is expected to own every reservation, context, loaded module,
queue, completion object, staging allocation, warmed executable, and other
native object needed by the candidate. `CandidateRealizer::destroy` is the
explicit deterministic teardown operation for a rejected session.

### Provider and realizer traits

`ArtifactProvider` is the Recipe-owned catalog boundary. `resolve` receives the
calculation graph, topology, and measured discovery profile and returns a
catalog. `identities` exposes only the final artifact identities to the
planner. A provider can read exact prebuilt inputs or compile in an allowed
preparation phase, but no compiler entry point may remain reachable once the
catalog is handed back.

`RealizeFailure<E>` has two meanings. `CandidateRejected` is retryable and
guarantees that all partially created native objects have already been
destroyed. `Fatal(E)` aborts the complete preparation.

`CandidateRealizer<C>` has three operations:

1. `reservation_plan` selects the implementation mechanism for the mandatory
   exact reservation on every topology storage device.
2. `realize` receives one `PlannedCandidate`, the provider catalog, the
   validated reservation ledger, and the policy. It must load and warm that
   exact candidate and return a live session plus observations.
3. `destroy` releases a session that the root rejected before finalization.

The root does not know how a driver allocates, loads, or warms resources. It
only accepts the evidence and session through this trait.

### Result and error values

`PreparedSystem<C, S>` is the successful fixed-point product. Its private
fields contain the `FinalizedBundle`, the `RealizationProfile`, the provider
catalog, the live realization session, the planner's exact logical-to-
physical external-output mapping, the number of attempted candidates, and the
ordered candidate rejection list. Accessors expose each part without allowing
mutation. `external_outputs()` yields `(task, logical, device, physical)`
tuples from the planner's selected egresses, rather than reverse-matching
resident copies. `into_parts` consumes the wrapper and transfers the bundle,
realization, catalog, and session to the execution caller.

`CandidateRejectionStage` records whether a candidate failed during
`Realize`, post-warm `Stabilize`, or final arena `FinalCapacity` packing.
`CandidateRejection` retains the candidate identity, stage, and detail.

`PrepareErrorKind` classifies root failures as `InvalidPolicy`,
`InvalidMeasuredProfile`, `ArtifactCatalog`, `InvalidReservation`,
`Planning`, `Realization`, `Teardown`, `Finalization`,
`CandidateExhaustion`, or `ArithmeticOverflow`. `PrepareError` carries the
kind, a human-readable message, and all rejections accumulated before the
fatal condition. Its `Display` output includes every rejection in order.

## `Preparer` pipeline

`Preparer<A, R>` owns one artifact provider, one candidate realizer, and one
policy. `new` installs the default policy. `with_policy` validates and replaces
it. The provider and realizer have read-only accessors, and `realizer_mut`
allows the caller to maintain backend state between attempts without creating
a second realization path.

### Entry points

`prepare(graph, profile)` is the one-graph convenience entry point. It wraps
the graph in `StaticCalculationProgram::every_iteration` with
`LoopIterations::ONE`, maps construction errors to `Planning`, then delegates
to `prepare_program`.

`prepare_program(program, profile)` first validates the policy and calls
`recipe_probe::validate_profile`. A theoretical topology seed cannot enter
this API. It then delegates to the private `prepare_program_validated` using
the profile's exact topology and discovery values.

### Fixed-point sequence

`prepare_program_validated` performs these operations in order:

1. It asks the realizer for a reservation plan and validates that ledger
   against the topology. Failure is `InvalidReservation`.
2. It builds an optimistic planning capacity from measured total capacity
   minus only each exact reservation. Runtime overhead, fragmentation, and
   safety headroom are zero-valued properties carrying the measured
   provenance. This ledger is only an enumeration upper bound, never a
   realization snapshot.
3. It resolves the artifact catalog and maps provider errors to
   `ArtifactCatalog`.
4. It calls `plan_program_candidates`. The planner revalidates the program,
   graph, topology, discovery, reservations, and optimistic capacity; lowers
   the graph, enumerates legal placements and artifacts, creates schedules and
   deferred build recipes, and returns a deterministic ranked
   `ProgramPlannerSearch`. Each item contains a `PlannedCandidate` plus the
   exact loop iteration count and per-task loop domains.
5. It consumes candidates one time each with `next_owned_program_candidate`.
   The helper clones the planner item so the realizer cannot borrow the
   planner's internal vector. The attempt counter is checked for `usize`
   overflow.
6. It calls `CandidateRealizer::realize` for the exact candidate. A
   `CandidateRejected` result records a `Realize` rejection, tells the planner
   to reject the issued identity, and continues. A `Fatal` result stops with
   `Realization` and the rejections collected so far.
7. It validates the returned observation with `validate_observation` (below).
   Observation failure destroys the live session first, records a `Stabilize`
   rejection, rejects the identity in the planner, and continues. A teardown
   error is fatal and is reported as `Teardown`.
8. It calls `pack_arenas` with the unchanged draft objects and the stabilized
   capacity. An insufficient or otherwise invalid layout destroys the session,
   records a `FinalCapacity` rejection, rejects the identity, and tries the
   next finite candidate.
9. It computes a canonical `RealizationIdentity` from the draft identity and
   candidate identity, profile identities, artifact builds, aliases, tasks,
   policy, realized artifacts, resources, reservations, and every capacity
   snapshot. It then builds the `RealizationProfile` using the final snapshot.
10. It computes a canonical `BundleIdentity` from the topology, discovery,
    unchanged draft, realization, artifacts, build recipes, aliases, tasks,
    resources, reservations, final capacity, sorted arena layouts, loop
    iteration count, and loop domains. The hash domains are
    `recipe-realization-v3` and `recipe-finalized-bundle-v5`.
11. It calls `FinalizedBundle::finalize_with_loop_schedule`. Finalize validates
    the draft and realization identities, loop domains, layouts, and resolved
    value locations. If it fails, the unchanged candidate session is destroyed
    and preparation returns `Finalization`; this is not silently converted into
    another realization.
12. It returns `PreparedSystem` with the finalized bundle, the same realization
    session, catalog, selected external outputs, attempt count, and rejection
    evidence. No candidate choice is mutable after this point.

If the finite planner stream ends, the result is `CandidateExhaustion` with
the number of attempts and every recorded rejection. Rejection bookkeeping
itself is checked: an unknown or already rejected planner identity becomes a
`Planning` error.

### Observation and stability rules

`validate_observation` enforces the fixed-point evidence independently of the
driver:

- the snapshot vector length must equal `stabilization_passes` exactly;
- each snapshot must validate against the topology and the returned
  reservations;
- the final `stable_tail` snapshots must be byte-for-byte equal to the final
  snapshot, including total capacity, runtime overhead, fragmentation, safety
  headroom, recipe-usable capacity, and their metadata;
- a missing snapshot or a pass-count conversion failure is an observation
  error; and
- a temporary `RealizationProfile` is built against the unchanged draft and
  validated, which proves that resources, identities, artifacts, reservations,
  and capacity still describe that draft.

When stability fails, the error includes per-pass, per-device byte deltas (or
an ordering/metadata difference when bytes are equal). `byte_delta` reports
whether the final value increased, decreased, or had no byte delta.

## Production native module

### Artifact identities and deferred compilation

`NativeArtifact` pairs an immutable Recipe `ArtifactIdentity` with the native
executor's `RuntimeArtifact`. `new` and the private validator cross-check the
Recipe identity, runtime ID, runtime byte digest, entry symbol, format/ABI,
backend, architecture, and CUDA or HSA runtime identity. A mismatch is an
`InvalidArtifact` error before the pair can enter a catalog or driver request.

`NativeArtifactCatalog::new` sorts artifacts by ID, validates every pair, and
rejects duplicate IDs. It stores both the full pairs and a parallel identity
slice. The catalog contains no builder, loader, context, allocator, or driver
handle. `NativeArtifactProvider` owns one catalog, validates the graph and
discovery profile on `resolve`, verifies that every catalog target appears in
the measured calculation devices, and returns a clone. The production root
currently passes an empty catalog because deferred builds are materialized by
the compiler below.

`CudaArtifactPolicy` records the CUDA toolchain, minimum and optional maximum
driver versions, and required Driver API symbols. `RuntimeArtifactPolicy` is
either that CUDA policy or HSA. `TargetBuildSpec` binds one measured
`TargetIdentity` to a `KernelTarget`, toolchain identity, `ArtifactBuilder`,
scratch path, and runtime policy. Its validation rejects zero toolchain
digests, mismatched backend/ABI/architecture, invalid CUDA driver ranges, and
crossed CUDA/HSA policy pairs.

`DeferredArtifactCompiler` sorts and validates unique target specifications.
`with_prebuilt_bundle` accepts one non-empty exact bundle for a known target;
the bundle is still inspected after every stage is lowered. During
`materialize`, static catalog artifacts are copied only when their identity
matches the candidate exactly. Deferred build recipes are grouped by their
measured target, sorted by artifact ID, and lowered by locating the exact
source-kernel and program digest in `PlannedCandidate::lowered_programs`.
Each lowered stage receives the entry symbol `recipe_stage_<artifact-id>` and
the build's workgroup width.

For NVIDIA, materialization either inspects a supplied cubin for the expected
SM and every entry symbol or builds a cubin bundle with
`BuildPhase::Realize`. For AMD, it similarly inspects or builds an HSACO bundle
for the target ID and code-object version. The build phase, inspection count,
and every returned entry name are checked. The image digest becomes the
artifact digest, and the resulting identity records the target ABI, pinned
toolchain, kernel template, resource bounds, and build provenance. A deferred
artifact must be used by exactly one measured native target, and all resolved
artifact IDs must be unique.

### Candidate driver and native adapter

`CandidateRealizationRequest` carries borrowed topology, discovery, planned
candidate, native artifacts, and reservation ledger. `CandidateDriver` is the
dependency-neutral physical boundary. It selects a reservation mechanism and
evidence for each discovered storage device, realizes one exact candidate,
warms its maximum-concurrency trace for a numbered pass, reports a validated
capacity snapshot, and destroys the session.

`CandidateDriverFailure` distinguishes a retryable candidate rejection, a
fatal driver error, and `PreFinalRealizationUnavailable`. The last case is
fatal for this crate: a backend that can bind only after Finalize cannot satisfy
the fixed-point contract.

`NativeExecutorDriver` adapts the native executor's
`ValidatedCandidateFactory`. It always selects `EnforcedQuota`, obtains
reservation evidence from the factory, converts `NativeArtifact` pairs into
executor candidate artifacts, and delegates realization, warmup, capacity,
and destruction. Executor failures are mapped one-for-one to
`CandidateDriverFailure` without a fallback implementation.

`PreparedNativeSession<S>` retains only the opaque driver session and the
immutable images that produced it. `into_parts` transfers that same warmed
session and image list to the finalized local executor. It does not reload,
reallocate, or recreate native resources.

### `NativeCandidateRealizer`

`NativeCandidateRealizer::new` validates the measured profile and requires a
compiler specification for every discovered calculation device. It retains
cloned topology and discovery identities, the deferred compiler, and one
driver. `reservation_plan` first requires those exact profile values, then
creates one `ReservationEntry` per topology device. Names are
`recipe-user-<device-id>`, bytes come from driver reservation evidence, and
the mechanism and evidence are validated as a complete ledger.

`realize` rejects a candidate whose draft topology or discovery identity differs
from the constructor profile, validates reservations, materializes every
artifact, and passes one exact request to the driver. It then runs passes
`1..=policy.stabilization_passes`, calling `warm_maximum_concurrency` followed
by `capacity_snapshot` for each pass. Any driver rejection or fatal error after
a session exists invokes `destroy_candidate` before the failure is returned.
Successful observations copy the candidate's draft resources and the exact
reservation ledger, pair the artifact identities with all snapshots, and wrap
the live session in `PreparedNativeSession`. `destroy` unwraps that wrapper and
delegates deterministic physical teardown.

`NativePrepareError<E>` reports invalid configuration, profile mismatch,
catalog, candidate, artifact, reservation, missing build target, lowering,
driver, teardown, or pre-Finalize availability failures. Its `Display` text
preserves the operation context, and `source` exposes lowering, driver, and
teardown causes. The `Infallible` specialization erases the impossible driver
variant when compiler-only validation has completed.

## Callers and end-to-end handoff

The root crate's `native_prepare` module reopens the identity-named measured
profile, resolves the current machine and exact CUDA/HSA bindings, builds one
`NativeTargetPlan`, and constructs a `DeferredArtifactCompiler`. The
`with_current_native_preparation` callback lends those scoped bindings and
owned target specifications without allowing driver handles to escape user
declarations.

`src/training.rs` and `src/inference.rs` use that callback to construct a
`LocalCandidateFactory`, wrap it in `NativeExecutorDriver`, create
`NativeCandidateRealizer` and an empty `NativeArtifactProvider`, and pass a
`Preparer` into the training crate. The public training entry points in
[`training/src/execute.rs`](../../../training/src/execute.rs) call
`prepare_program` themselves. After it returns they validate workload-specific
loop and transfer rules, build the finalized `init` images and external-output
mapping, consume `PreparedSystem::into_parts`, and hand the retained native
session to `ValidatedCandidateSession::into_inner` and the local backend.

The local executor then runs `PreparedRun::prepare`, `initialize`, the bounded
or stop-controlled loop, and `exit`. Inference follows the same sequence and
uses `PreparedSystem::external_outputs` to map physical exit values back to
the declared logical tensors. Training additionally retains realized native
kernels and post-exit metrics. Both paths use the exact session warmed before
Finalize, so the run lifecycle cannot perform a second native realization.

The downstream `recipe-native-executor` validation wrapper checks the request's
topology, discovery, draft, reservations, and artifact set before creating its
session. It records candidate/profile identities in that session, rejects a
different candidate or pass zero during warmup, validates snapshot identity and
capacity, and deterministically destroys the inner session. This is the
callee-side proof that the evidence accepted by `recipe-prepare` belongs to the
same immutable candidate later handed to the executor.

## Invariants and deliberate limits

- Preparation requires a complete, hashed measured profile. The optimistic
  planning ledger cannot be mistaken for post-warm capacity.
- Every topology device receives the exact reservation ledger entry before
  planning. Reservation, resource, artifact, draft, topology, discovery, and
  candidate identities must remain unchanged through realization and
  Finalize.
- Candidate enumeration is finite and one-shot. Only an issued identity may
  be rejected, and rejection never mutates the draft or planner ranking.
- Capacity is accepted only after the configured stable tail agrees exactly.
  Final arena offsets are packed once against that final ledger.
- A rejected candidate's live session is destroyed before the next candidate.
  A successful session survives beside the immutable bundle and is handed off
  exactly once.
- Deferred compilation occurs only in Realize and all resulting image and
  toolchain identities are recorded. No compiler, loader, allocator, mutable
  driver, or post-Finalize fallback is retained in the prepared session.
- Native artifacts are backend-specific. CUDA identities require the cubin ABI
  and matching SM, toolchain, driver range, and symbols. HSA identities require
  the measured target ID and matching code-object ABI.
- The root returns `CandidateExhaustion` rather than silently substituting an
  unvalidated plan. Fatal realization, teardown, planning, or Finalize errors
  remain visible with the accumulated rejection evidence.

This crate prepares the fixed point; it does not own dataset packing,
training/inference semantics, run polling, metric collection, or model export.
Those responsibilities stay in the callers and executor layers described
above.
