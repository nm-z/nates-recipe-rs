# `recipe-prepare`

`recipe-prepare` is Recipe's public, fixed-point boundary between a validated
measured machine profile and one immutable execution bundle whose native
resources are already alive. It owns the first four lifecycle stages:

```text
measured DiscoveryProfile -> draft -> realize -> finalize
```

The crate does not execute a user loop. It makes the execution choice, native
images, reservations, warmed resources, capacity evidence, arena offsets, and
identities agree before `recipe-executor` can enter `init -> loop -> exit`.
After a successful call, the returned `PreparedSystem` keeps the
`FinalizedBundle` beside the exact session that realized it. A caller cannot
finalize an optimistic draft and then repair it around whatever a driver later
allocated.

The normative lifecycle and preparation contract are in
[`system-contract.md`](../../system-contract.md#c9-ahead-of-run-boundary). The
crate's complete public facade is [`src/lib.rs`](../src/lib.rs); its production
compiler and driver adapter is [`src/production.rs`](../src/production.rs).

## Position in the workspace

```text
recipe-probe::MeasuredProfile
          + recipe-program::StaticCalculationProgram
          + recipe-language::CalculationGraph
                            │
                            ▼
                     recipe-prepare
                 (draft -> realize -> finalize)
                   │        │          │
                   │        │          └─ recipe-core::FinalizedBundle
                   │        └─ recipe-kernel + recipe-native-executor
                   └─ recipe-planner + recipe-scheduler
                            │
                            ▼
               recipe-training / root inference
                            │
                            ▼
              recipe-executor::PreparedRun
                 init -> loop -> exit
```

The root facade re-exports this crate as `recipe::engine::prepare` in
[`src/facade.rs`](../../src/facade.rs#L17-L41). The direct runtime callers are
the local training and inference paths. The root native-preparation boundary
opens the exact measured CUDA/HSA bindings and builds target specifications;
`recipe-prepare` then consumes those specifications through its generic
traits. The crate itself has no probe cache reader, native probing loop,
socket listener, executor loop, or model declaration parser.

## Manifest and module boundary

[`prepare/Cargo.toml`](../Cargo.toml) declares the crate as Rust 2024,
`recipe-prepare`, with `unsafe_code = "forbid"`. Its direct dependencies and
their roles are:

| Dependency | Role at this boundary |
| --- | --- |
| `recipe-core` | typed topology, measured discovery, reservations, capacity, drafts, identities, finalization, and loop schedule |
| `recipe-language` | calculation graph accepted by the graph-level convenience API |
| `recipe-program` | validated static program and loop domains |
| `recipe-planner` | finite candidate enumeration, lowering, placement, routes, tasks, and external-output identities |
| `recipe-scheduler` | deterministic arena packing against realized capacity |
| `recipe-probe` | `MeasuredProfile` and measured-profile validation |
| `recipe-kernel` | deferred stage lowering, CUDA cubin building/inspection, and AMD HSACO building/inspection |
| `recipe-cuda` | CUDA toolchain, driver-version, and symbol policy values |
| `recipe-native-executor` | validated candidate-scoped loading, allocation, warmup, capacity observation, and destruction |
| `sha2` | domain-separated canonical realization and bundle hashes |

Only two Rust modules exist. `lib.rs` is the public fixed-point algorithm and
trait facade. `production.rs` is private and is re-exported selectively by
`lib.rs`; it contains native artifact validation/materialization and the
`CandidateDriver` adapter. The crate denies missing `Debug` implementations and
forbids unsafe code at the crate root.

## Public API surface

The public names fall into four groups.

### Generic preparation

- `DEFAULT_STABILIZATION_PASSES` is three and `DEFAULT_STABLE_TAIL` is two.
- `PreparationPolicy` carries the bounded pass count and the number of equal
  final snapshots required for stabilization. `with_policy` rejects a stable
  tail longer than the pass count.
- `ArtifactProvider` resolves one catalog from the graph, topology, and
  discovery and exposes only the catalog's final `ArtifactIdentity` values to
  planning. A provider may compile in an allowed pre-final phase, but no
  compiler entry point is available after `resolve` returns.
- `CandidateRealizer<C>` supplies a reservation ledger, realizes one
  `PlannedCandidate`, and explicitly destroys rejected sessions.
- `RealizeFailure` separates retryable `CandidateRejected` from fatal errors.
- `RealizationObservation` reports artifacts, the draft resource manifest, the
  reservation ledger, and one post-warm `CapacityLedger` per stabilization pass.
- `RealizedCandidate<S>` pairs that observation with the live session that
  produced it.
- `Preparer<A, R>` owns one provider, one realizer, and one policy. `new`,
  `with_policy`, `artifact_provider`, `realizer`, and `realizer_mut` are the
  construction/access methods. `prepare` accepts a graph and creates a
  one-iteration static program. `prepare_program` accepts an existing static
  program and a measured profile.
- `PreparedSystem<C, S>` exposes references to the finalized bundle, realization
  profile, catalog, and live session; exact external-output mappings;
  attempted-candidate count; rejection evidence; and `into_parts` for the
  one-way handoff to execution.

### Errors and rejection evidence

`PrepareErrorKind` is non-exhaustive and classifies invalid policy, invalid
measured profile, artifact catalog, reservation, planning, realization,
teardown, finalization, candidate exhaustion, and arithmetic overflow.
`PrepareError` carries a human-readable message plus all
`CandidateRejection { candidate, stage, detail }` values accumulated before the
failure. Rejection stages are `Realize`, `Stabilize`, and `FinalCapacity`.

### Native artifact and driver types

The production re-exports are:

- `NativeArtifact`, one validated `ArtifactIdentity` plus one native executor
  `RuntimeArtifact` sharing ID, digest, entry symbol, target, and format;
- `NativeArtifactCatalog`, sorted unique prebuilt artifacts with parallel
  identity access;
- `NativeArtifactProvider`, the `ArtifactProvider` for a fixed catalog;
- `CudaArtifactPolicy` and `RuntimeArtifactPolicy`, the target-specific CUDA or
  HSA ABI/toolchain policy;
- `TargetBuildSpec`, the exact measured target to compiler target, toolchain,
  scratch directory, and runtime-policy mapping;
- `DeferredArtifactCompiler`, the Realize-only compiler set and optional
  prebuilt bundle map;
- `CandidateRealizationRequest`, the topology, discovery, candidate, native
  artifacts, and reservations passed to a driver;
- `CandidateDriver` and `CandidateDriverFailure`, the dependency-neutral native
  realization lifecycle and its retryable/fatal/pre-Finalize-unavailable
  classifications;
- `NativeExecutorDriver<F>`, the adapter over
  `recipe_native_executor::ValidatedCandidateFactory<F>`;
- `PreparedNativeSession<S>`, the opaque live driver session plus immutable
  native images;
- `NativeCandidateRealizer<D>`, the production `CandidateRealizer` that
  materializes artifacts, invokes the driver, runs stabilization, and retains
  only the successful session and images; and
- `NativePrepareError<E>`, the production configuration, profile, catalog,
  candidate, artifact, reservation, lowering, driver, teardown, and
  pre-Finalize availability error surface.

These names are all re-exported from the single facade in
[`lib.rs`](../src/lib.rs#L29-L35). There are no public modules below that
facade.

## State model and ownership

The important state is typed and moves in one direction:

| State | Owner | Mutable? | Meaning |
| --- | --- | --- | --- |
| `MeasuredProfile` | caller/probe layer | no during preparation | exact topology and discovery identities plus measured capabilities and capacities |
| `StaticCalculationProgram` | caller/program layer | no during preparation | validated graph, loop count/domains, and metric declarations |
| planning capacity | `Preparer` local | no after planning call | optimistic upper bound: measured total minus mandatory reservation, with zero runtime/fragmentation/headroom estimates |
| `ProgramPlannerSearch` | `Preparer` local | cursor and rejection sets | one-shot ranked candidate stream; an issued candidate is never yielded again |
| `DraftPlan` / `PlannedCandidate` | planner | no | exact values, stage templates, artifacts/build recipes, tasks, routes, resources, arena objects, and logical copies, without final physical offsets |
| `NativeArtifactCatalog` | provider/realizer | no after resolve/materialize | validated immutable runtime images and identities |
| candidate session | `CandidateRealizer`/driver | yes while warming | every reservation, context, module, queue, signal/event, staging allocation, scratch allocation, and warmed executable required by that candidate |
| `RealizationObservation` | realizer | no after return | measured post-warm evidence for the unchanged draft |
| `RealizationProfile` | `Preparer` | no | hashed identity tying draft, candidate, topology, discovery, artifacts, resources, reservations, and final stable capacity together |
| arena layouts | scheduler | no after `pack_arenas` | deterministic offsets and per-device sizes computed from final observed capacity |
| `FinalizedBundle` | `PreparedSystem` | no | immutable execution product validated by `recipe-core` |
| `PreparedNativeSession` | `PreparedSystem` | opaque ownership | the exact warmed physical session that must be handed to the finalized local backend |

`PreparedSystem` deliberately stores the bundle and session together. Its
`bundle`, `realization`, `catalog`, and `session` fields are private; callers
cannot replace one independently. `into_parts` consumes the wrapper and moves
the same values onward. `PreparedNativeSession::into_parts` similarly moves the
same driver session and image vector without allocating or loading a second
copy.

## The fixed-point pipeline

`Preparer::prepare_program` is the complete public path. The implementation is
[`prepare_program_validated`](../src/lib.rs#L310-L526).

### 1. Validate the policy and measured profile

`prepare_program` checks `PreparationPolicy`, then calls
`recipe_probe::validate_profile`. A profile with an invalid topology,
discovery, identity, or property never reaches planning. The graph-level
`prepare` convenience method first constructs
`StaticCalculationProgram::every_iteration(graph.clone(), LoopIterations::ONE)`;
failure is reported as `PrepareErrorKind::Planning`.

The program-level API preserves the program's loop count and exact loop task
domains. It does not unroll the graph. The graph-level convenience API is the
only path that forces one iteration.

### 2. Derive and validate the exact reservation ledger

`CandidateRealizer::reservation_plan` is called before artifact resolution or
candidate enumeration. The returned ledger must validate against every
topology device. Production `NativeCandidateRealizer` calls
`exact_reservation_plan`, which asks its `CandidateDriver` for a reservation
mechanism and reservation evidence for each measured device. The production
`NativeExecutorDriver` reports `EnforcedQuota` and obtains evidence from the
validated native-executor factory.

Core reservation validation requires one entry per topology device, matching
device kind and evidence, with exactly the evidence-required bytes. The
constant is `1_000_000_000` bytes for RAM and disk and for a GPU with one or
more live display connectors; a zero-connector GPU has the explicit zero-byte
display exemption. The reservation is accounting headroom, not a dummy
allocation. See [`core/src/plan.rs`](../../core/src/plan.rs#L23-L126) and the
normative [C5 contract](../../system-contract.md#c5-exact-user-reservation).

`optimistic_planning_capacity` then subtracts only the reservation bytes from
each discovered total. Runtime overhead, fragmentation, and safety headroom are
zero-valued with the discovered provenance. This ledger is explicitly an
optimistic planning bound, not a driver observation. It is validated before
being passed to `recipe-planner`; final packing never trusts it.

### 3. Resolve the artifact catalog and enumerate drafts

`ArtifactProvider::resolve` receives the program graph, topology, and
discovery. The native provider validates the graph and discovery again and
rejects any prebuilt artifact whose target is absent from measured discovery.
The provider returns a catalog and a slice of final artifact identities.

`recipe_planner::plan_program_candidates` validates the program, graph,
topology, scheduling properties, discovery, reservations, and planning
capacity. It lowers every primitive program, computes legal GPU placements,
routes, stage templates, tasks, resource manifests, logical copies, init/exit
transfers, and deferred artifact build recipes. It exhaustively visits the
finite placement product, retains feasible `PlannedProgramCandidate` values,
and ranks them by measured makespan and then candidate identity. The planner's
one-shot stream and rejection contract are visible in
[`planner/src/planner.rs`](../../planner/src/planner.rs#L62-L169) and its entry
point in [`planner/src/planner.rs`](../../planner/src/planner.rs#L220-L350).

At this point a `DraftPlan` has no final arena offsets. It is an exact
offset-free choice, not permission to allocate later under a different
placement or artifact set.

### 4. Realize one candidate

`Preparer` takes the next owned planner candidate and increments the attempt
count with checked arithmetic. It calls `CandidateRealizer::realize` with the
candidate, the resolved catalog, the exact reservations, and the policy.

- `RealizeFailure::CandidateRejected` means the implementation has already
  destroyed all partial native objects and returns a detail suitable for the
  planner rejection record. `Preparer` records stage `Realize`, marks the
  identity rejected in `ProgramPlannerSearch`, and continues.
- `RealizeFailure::Fatal` aborts preparation as `PrepareErrorKind::Realization`
  and retains earlier rejection evidence.

The production realizer first checks that the candidate's topology and
discovery identities equal the profile captured at construction, validates the
reservation ledger, and asks `DeferredArtifactCompiler::materialize` for every
runtime image required by the draft. It then passes one
`CandidateRealizationRequest` to one driver. A driver may not return a live
session through a post-Finalize binding path; that condition is fatal.

### 5. Warm and stabilize at maximum concurrency

The production realizer runs exactly `policy.stabilization_passes` passes. Each
pass calls `CandidateDriver::warm_maximum_concurrency` and only then
`capacity_snapshot`. A snapshot is therefore post-trace evidence, not a
preallocation estimate. `RealizationObservation` stores the snapshots in pass
order.

`validate_observation` requires:

1. exactly the configured number of snapshots;
2. every snapshot to validate against topology and the observation's
   reservations;
3. the final `stable_tail` snapshots to be byte-for-byte equal; and
4. the unchanged draft, resources, candidate, identities, artifacts,
   reservations, and final snapshot to pass `RealizationProfile::validate`.

Failure destroys the live session, records a `Stabilize` rejection, and tries
the next finite candidate. If destruction fails, the result is
`PrepareErrorKind::Teardown` and no replacement session is attempted. The
capacity ledger itself requires measured or explicitly overridden properties
and checks reservation plus runtime overhead, fragmentation, safety headroom,
and Recipe-usable bytes do not exceed total capacity.

The default is three complete warm/snapshot passes with the last two equal.
`stable_tail = 0` is representable by the type but policy validation only
constrains it to be no greater than the pass count; the observation code still
requires a nonempty final snapshot because a successful realization needs a
last capacity ledger.

### 6. Pack final arenas against observed capacity

After stabilization, `Preparer` calls `recipe_scheduler::pack_arenas` with the
unchanged draft arena objects and the final capacity ledger. The scheduler
chooses deterministic aligned offsets, reusing non-overlapping lifetimes. A
layout failure destroys the session, records a `FinalCapacity` rejection, and
continues. The optimistic planning ledger is never reused for this step.

### 7. Hash and finalize one immutable bundle

`hash_realization` uses the domain `recipe-realization-v3` and includes the
draft/candidate/profile identities, artifact builds, aliases, tasks, policy,
realized artifacts, resources, reservations, and every capacity snapshot.
`hash_bundle_with_loop_domains` uses `recipe-finalized-bundle-v5` and adds the
loop count/domains, final capacity, and sorted arena layouts. Canonical hashing
sorts identity-bearing collections and encodes byte strings with a length
prefix and integers as little-endian `u64` values.

The resulting `RealizationProfile` and `BundleIdentity` are passed to
`FinalizedBundle::finalize_with_loop_schedule`. Core validation checks the
unchanged draft, realization identity links, exact loop-domain coverage,
capacity-backed layouts, and resolved value locations before constructing the
immutable bundle. A finalization error destroys the session and returns
`PrepareErrorKind::Finalization` immediately, because the candidate that just
passed realization and packing was not valid for the final core contract.

On success, `Preparer` returns a `PreparedSystem` containing the finalized
bundle, realization profile, catalog, live session, exact external-output
mappings, attempt count, and prior rejections. If the one-shot planner stream
ends, the result is `CandidateExhaustion` with every recorded candidate
rejection.

## Production artifact path

The private [`production.rs`](../src/production.rs) module is the only in-tree
implementation of the generic production boundaries.

### Artifact identity and catalog

`NativeArtifact::new` validates the Recipe identity against the native runtime
image before either can be used. It checks the artifact's own core identity,
runtime ID, byte digest, entry symbol, format/ABI, and CUDA or HSA target
identity. `NativeArtifactCatalog::new` sorts by artifact ID, rejects duplicate
IDs, revalidates each pair, and stores a parallel identity slice. The catalog
contains no builder, loader, context, allocator, or driver handle.

`NativeArtifactProvider` is intentionally a fixed-catalog provider. It checks
the graph and discovery and requires every catalog target to exist in the
measured calculation capabilities. It clones the validated catalog for the
preparation call; it does not discover a near-match target or silently compile
after resolution.

### Deferred compilation and prebuilt bundles

`TargetBuildSpec` ties one exact measured `TargetIdentity` to a
`KernelTarget`, nonzero toolchain digest, `ArtifactBuilder`, scratch parent,
and `RuntimeArtifactPolicy`. CUDA specs require the NVIDIA backend and cubin
ABI, matching architecture and cubin format, and a coherent driver range.
HSA specs require the AMD backend, the expected code-object ABI, and either the
short or canonical full architecture spelling. A CUDA target with an HSA
policy, or vice versa, is rejected.

`DeferredArtifactCompiler::new` sorts specs and rejects duplicate target
identities. `with_prebuilt_bundle` accepts one nonempty exact bundle only for a
known target. During `materialize` it:

1. copies every prebuilt draft artifact from the catalog, requiring exact
   identity equality;
2. maps every deferred `ArtifactBuildRecipe` to exactly one target spec from
   the candidate's measured calculation placements;
3. finds the exact lowered program by source-kernel ID and program digest;
4. lowers the stage with an entry symbol `recipe_stage_<artifact-id>` and the
   draft workgroup width in `BuildPhase::Realize`;
5. either inspects a supplied cubin/HSACO bundle or builds one in Realize;
6. verifies inspection count and every entry symbol; and
7. derives the `ArtifactIdentity` digest from the immutable runtime image and
   constructs the matching CUDA or HSA `RuntimeArtifact`.

An artifact assigned to no target, more than one target, or more than once is
invalid. A prebuilt image is an optimization of the same identity path, not a
second realization implementation. Compilation and inspection happen only in
this pre-Finalize materialization path. The compiler is not stored in
`PreparedNativeSession`.

### Candidate driver and native executor bridge

`CandidateDriver` requires the following sequence for one candidate:

```text
reservation_mechanism/evidence (per topology device)
  -> realize_candidate(request)
  -> [warm_maximum_concurrency -> capacity_snapshot] × pass count
  -> destroy_candidate(session) on rejection
```

The session owns the exact reservation, contexts, loaded modules, queues,
completion objects, staging, scratch, warmed executable, and other candidate
resources. `warm_maximum_concurrency` must execute the maximum-concurrency
schedule without changing the draft. A successful session is reused by the
finalized runtime; silently loading or allocating a replacement after
`Finalize` is prohibited.

`NativeExecutorDriver` wraps
`recipe_native_executor::ValidatedCandidateFactory`. It converts each
`NativeArtifact` to the executor's `CandidateArtifact`, delegates candidate
validation and realization, maps executor failure classes, and delegates warm,
snapshot, and destruction calls. The executor validation layer rejects any
topology, discovery, draft, artifact, reservation, warm-pass, or capacity
mismatch before evidence crosses this crate boundary.

`NativeCandidateRealizer::new` validates the complete measured profile and
requires a compiler spec for every measured calculation target. Its
`CandidateRealizer` implementation captures the profile identities, builds an
exact per-device reservation plan, materializes artifacts, realizes one driver
session, runs the bounded trace, and returns:

```text
PreparedNativeSession {
    driver_session: validated native session,
    artifacts: immutable native images,
}
```

If warmup or snapshot fails after a session exists, `reject_after_session`
destroys it before mapping a candidate rejection or fatal driver error. A
destruction failure becomes `NativePrepareError::Teardown` and is never hidden
by a retry. `destroy` later delegates to the driver's deterministic destroy
operation for candidates rejected by observation or arena packing.

## Callers and the end-to-end handoff

### Root native preparation

[`src/native_prepare.rs`](../../src/native_prepare.rs#L248-L411) loads the exact
identity-named measured profile, reopens the current machine's GPU and host
bindings, and lends them to one higher-ranked callback. It constructs one
`TargetBuildSpec` per unique measured target, checks equivalent specs for GPUs
that share a target, and exposes `NativeTargetPlan::deferred_compiler`.
The callback cannot let CUDA contexts or HSA sessions escape into a declaration.

### Inference

[`src/inference.rs`](../../src/inference.rs#L602-L659) enters
`with_current_native_preparation`, derives host/runtime limits, constructs a
production `LocalCandidateFactory`, wraps it in `NativeExecutorDriver`, creates
`DeferredArtifactCompiler`, `NativeCandidateRealizer`, an empty
`NativeArtifactProvider`, and `Preparer`, then calls one of the training-crate
inference execution functions. Dense, Bayes, GGUF Llama, and KNN all use this
same prepare boundary.

[`training/src/execute.rs`](../../training/src/execute.rs#L1200-L1310) prepares
ordinary inference, checks the one-iteration contract and the no-loop-external
transfer/user-metric rules, builds finalized init images and output mappings,
consumes `PreparedSystem`, hands the exact warmed local session to
`PreparedRun::prepare_recoverable`, then runs `init -> loop -> exit`.
The KNN path follows the same handoff at
[`training/src/execute.rs`](../../training/src/execute.rs#L1321-L1369).

### Training

[`src/training.rs`](../../src/training.rs#L1278-L1338) uses the same callback and
factory construction for training. If a native resume bundle exists, it first
checks its topology, discovery, target, and toolchain identities, then supplies
the exact bytes through `DeferredArtifactCompiler::with_prebuilt_bundle`.
Absent a resume bundle, deferred stages are compiled normally in Realize.

[`training/src/execute.rs`](../../training/src/execute.rs#L2176-L2245) calls
`prepare_program(training.program(), profile)`, rejects loop external
transfers, builds one init image per finalized device, maps exact external
output tasks, and retains user metric slots. It consumes the same session,
converts it to a local backend with `into_backend(&bundle)`, initializes a
`PreparedRun`, and polls the bounded loop. The observer and graceful-stop
variants wrap this same controlled function; they do not create another
preparation path.

The executor sees only the finalized bundle and the already-realized backend.
Its `init` stage admits fixed images, its loop executes the preplanned graph,
and its `exit` stage performs planned egress and releases. There is no public
API after preparation for replanning, topology mutation, late compilation,
late loading, external loop ingress, or a hidden fallback device.

## Error and retry semantics

The generic and production error layers are intentionally separate:

| Layer | Retryable | Fatal examples |
| --- | --- | --- |
| `CandidateRealizer` | `RealizeFailure::CandidateRejected` for one candidate | provider/driver errors returned as `Fatal` |
| `CandidateDriver` | `CandidateDriverFailure::CandidateRejected` | `Fatal` or `PreFinalRealizationUnavailable` |
| `Preparer` observation | stable-tail mismatch, invalid observed capacity, arena pack failure | teardown failure, finalization failure, attempt-count overflow |
| production materialization | none after an invalid exact artifact/candidate/configuration | missing build target, lowering/inspection error, identity mismatch |

`PrepareError` reports the first non-retryable stage and retains rejection
records. `CandidateExhaustion` is the only normal terminal result after all
finite planner candidates have been rejected. Rejection bookkeeping itself is
validated by `ProgramPlannerSearch::reject`; an unknown or already rejected
identity is a planning error, not a reason to guess another candidate.

`NativePrepareError` distinguishes invalid configuration/profile/catalog/
candidate/artifact/reservation, profile mismatch, missing build target,
lowering failure, driver failure, teardown failure, and a backend that has no
pre-Finalize realization path. `NativePrepareError<Infallible>` is used for
configuration and image-only operations; `erase_driver` changes only the
infallible error parameter and cannot discard a real driver error.

## Invariants that must hold

- Preparation accepts a validated measured profile. The theoretical topology
  seed is not a runtime profile and cannot enter `Preparer::prepare_program`.
- Topology and discovery identities remain equal from profile validation through
  draft, realization, finalization, and executor handoff.
- Every topology device has one valid reservation entry. Required bytes follow
  reservation evidence exactly; extra free capacity does not rewrite headroom.
- The planner is finite and deterministic. Candidate identities are unique,
  ranked by measured makespan plus stable ID, issued once, and rejected once.
- A `RealizationProfile` describes the unchanged draft. It cannot add a task,
  alter resources, substitute artifacts, or change candidate placement.
- Every deferred artifact is materialized exactly once with its measured target,
  build provenance, ABI, entry symbol, resource bounds, digest, and runtime
  image aligned.
- All compilation, driver loading, allocation, warmup, and opaque allocation
  stabilization happen before `Finalize`. Finalize only validates and freezes.
- Arena offsets are packed from the final stable capacity snapshot, not from the
  optimistic planning bound or a previous candidate.
- A candidate that fails after allocating resources is explicitly destroyed
  before another candidate is attempted. Destruction failures remain visible.
- The successful session is the same physical session handed to the finalized
  local backend. No second load, allocation, or warmup is permitted after the
  handoff.
- The finalized loop contains no external dataset/model/file ingress or result
  egress. Admission is an `init` transfer and user output is an `exit` transfer.
- `TaskKind::Metric` remains a specialized loop readback transfer. Metrics do
  not become a separate preparation or model-work ontology.

## Source map and validation

| Concern | Source |
| --- | --- |
| Public policy, traits, errors, `Preparer`, pipeline, hashes | [`prepare/src/lib.rs`](../src/lib.rs) |
| Native artifact/catalog/provider/compiler and driver adapter | [`prepare/src/production.rs`](../src/production.rs) |
| Draft, reservation, capacity, realization, and immutable finalization | [`core/src/plan.rs`](../../core/src/plan.rs) |
| Ranked candidate enumeration and rejection bookkeeping | [`planner/src/planner.rs`](../../planner/src/planner.rs#L62-L350) |
| Final arena packing | [`scheduler/src/arena.rs`](../../scheduler/src/arena.rs#L9-L120) |
| Validated native candidate lifecycle | [`native-executor/src/candidate.rs`](../../native-executor/src/candidate.rs#L243-L411) |
| Local host/CUDA/HSA realization, warmup, snapshots, and destruction | [`native-executor/src/local.rs`](../../native-executor/src/local.rs#L997-L1608) |
| Profile-scoped target and binding construction | [`src/native_prepare.rs`](../../src/native_prepare.rs#L248-L792) |
| Training and inference execution handoff | [`training/src/execute.rs`](../../training/src/execute.rs#L1200-L1369) and [`training/src/execute.rs`](../../training/src/execute.rs#L2176-L2245) |

Structural validation for this crate is:

```bash
cargo check -p recipe-prepare
```

That command checks the Rust API and dependency wiring. It is not runtime
evidence. Runtime proof requires the public training or inference declaration,
an identity-matching measured profile, the corresponding offline toolchain and
drivers, real CUDA or HSA hardware, and the acceptance gates described in
[`acceptance/README.md`](../../acceptance/README.md).
