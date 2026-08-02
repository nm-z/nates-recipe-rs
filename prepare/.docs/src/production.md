# `recipe_prepare::production`

```yaml
document: recipe_prepare.production
source: prepare/src/production.rs
kind: production-pre-final-realization-contract
authority:
  - prepare/src/production.rs
  - prepare/src/lib.rs
  - native-executor/src/candidate.rs
  - native-executor/src/local.rs
  - native-executor/src/cuda.rs
  - native-executor/src/hsa.rs
  - native-executor/src/plan.rs
  - kernel/src/builder.rs
  - kernel/src/stage.rs
  - core/src/artifact.rs
  - core/src/plan.rs
  - probe/src/codec.rs
  - src/native_prepare.rs
  - src/inference.rs
  - src/training.rs
  - training/src/execute.rs
  - system-contract.md
```

This page describes the production implementation of the pre-Finalize fixed
point. `recipe-prepare` receives a validated measured profile and a target-free
static program, enumerates immutable Draft candidates, realizes each candidate
against the actual native resources, and finalizes only the candidate whose
post-warm capacity and identity evidence match. The same native session that is
loaded, allocated, and warmed before `Finalize` is handed to the run backend
after `Finalize`. There is no second compiler, loader, allocator, or mutable
driver path after the bundle has been finalized.

The complete boundary is:

```text
MeasuredProfile + StaticCalculationProgram
        |
        v
reservation_plan -> optimistic planning capacity -> ArtifactProvider::resolve
        |
        v
ProgramPlannerSearch (finite Draft candidates)
        |
        +--> DeferredArtifactCompiler::materialize
        |       lower exact deferred stages
        |       build or inspect one target bundle
        |       validate ArtifactIdentity <-> RuntimeArtifact
        |
        +--> CandidateDriver::realize_candidate
        |       validate Draft and resources
        |       load modules/executables
        |       allocate queues, staging, scratch, and pending state
        |
        +--> warm_maximum_concurrency + capacity_snapshot (bounded passes)
        |
        +--> validate_observation -> pack_arenas -> FinalizedBundle::finalize
                                     |
                                     v
                         PreparedSystem(bundle, profile, catalog, same session)
                                     |
                                     v
                         LocalPreparedSession::into_backend(bundle)
```

The Draft is never edited to fit a native result. A retryable driver rejection
or a post-warm capacity failure destroys that candidate and asks the planner for
the next finite candidate. Malformed preparation inputs, a compiler failure, a
fatal native error, or failed teardown aborts the preparation instead of
selecting a substitute implementation.

## 1. Boundary vocabulary

### 1.1 Inputs and ownership

| input | producer | production meaning |
| --- | --- | --- |
| `StaticCalculationProgram` | Recipe source/model compilation | A validated, bounded graph plus loop count. `Preparer::prepare` wraps a graph in one `LoopIterations::ONE`; `prepare_program` accepts an already decoded program (`prepare/src/lib.rs:315-338`). |
| `MeasuredProfile` | `recipe-probe` cache or fresh probe | The exact `Topology`, `DiscoveryProfile`, measured identities, capacities, rates, and provenance. `validate_profile` rejects stale schema, unmeasured properties, identity mismatch, and noncanonical order (`probe/src/codec.rs:121-157`). |
| `TargetBuildSpec` | `src/native_prepare.rs::build_scope` | One measured calculation `TargetIdentity` paired with a validated `KernelTarget`, pinned `ToolchainIdentity`, `ArtifactBuilder`, scratch directory, and native runtime policy (`prepare/src/production.rs:192-251`). |
| `DeferredArtifactCompiler` | `NativeTargetPlan::deferred_compiler` | Sorted, unique target specifications and optional exact prebuilt images. It is retained only while a candidate is being materialized (`prepare/src/production.rs:254-308`). |
| `NativeArtifactCatalog` | `NativeArtifactProvider` or another `ArtifactProvider` | Validated runtime images for static Draft artifacts. The catalog has no compiler, loader, context, allocator, or driver handle (`prepare/src/production.rs:78-129`). |
| `PlannedCandidate` | `recipe-planner::ProgramPlannerSearch` | One immutable Draft, lowered programs, stage placements, and loop-domain side data. The planner owns finite enumeration and rejection bookkeeping. |
| `ReservationLedger` | `CandidateRealizer::reservation_plan` | Exactly one reservation entry for every topology device. Its bytes are derived from native evidence, not guessed by the planner. |
| `PreparationPolicy` | `Preparer` | Bounded stabilization pass count and stable-tail length. Defaults are three passes and a final two-snapshot tail (`prepare/src/lib.rs:37-66`). |
| `CandidateDriver` | production native adapter | The only interface allowed to create and destroy candidate-scoped physical state (`prepare/src/production.rs:724-795`). |

`NativePreparationScope` in `src/native_prepare.rs` supplies the measured native
bindings, host plan, and `TargetBuildSpec` inputs. It reopens the exact current
GPU bindings, checks the local machine and device set, derives one unique target
spec per measured target, and constructs the host plan. Training and inference
consume that scope inside a higher-ranked callback, so CUDA contexts and HSA
sessions cannot enter declarations or outlive the preparation scope
(`src/native_prepare.rs:305-366`).

### 1.2 Outputs and ownership

| output | contents | lifetime rule |
| --- | --- | --- |
| `NativeArtifact` | One immutable `ArtifactIdentity` and one matching `RuntimeArtifact` image/ABI | Constructed only after digest, entry, target, and format checks pass (`prepare/src/production.rs:53-76`, `:655-722`). |
| `NativeArtifactCatalog` | Sorted static artifacts plus a parallel identity slice | Cloned into `PreparedSystem`; it never owns native handles (`prepare/src/production.rs:78-129`). |
| `PreparedNativeSession<S>` | The opaque driver session plus the immutable images used to load it | Compiler and driver are absent. `into_parts` transfers the same physical session exactly once (`prepare/src/production.rs:906-932`). |
| `RealizationObservation` | Artifact identities, Draft resource manifest, reservations, and one capacity snapshot per pass | Evidence used by `validate_observation`, realization hashing, and Finalize (`prepare/src/lib.rs:98-114`). |
| `RealizedCandidate` | Session and observation together | The session remains live until Finalize succeeds or `destroy` is called (`prepare/src/lib.rs:109-114`). |
| `PreparedSystem` | Finalized immutable bundle, realization profile, catalog, live session, egress identities, attempt count, and rejection evidence | Bundle and session are inseparable until the runtime handoff (`prepare/src/lib.rs:224-271`). |

## 2. Native artifact materialization

### 2.1 Static catalog and provider

`NativeArtifact::new` calls `validate_native_artifact` before exposing either
the semantic identity or runtime image. `NativeArtifactCatalog::new` sorts by
`ArtifactId`, revalidates every pair, and rejects duplicate IDs. Binary search
therefore resolves one exact image without a fallback or a second catalog
implementation (`prepare/src/production.rs:65-128`).

`NativeArtifactProvider::resolve` validates the graph and measured discovery,
then checks that every catalog identity's `TargetIdentity` is present on a
calculation-capable discovered device. It returns a clone of the fixed catalog;
it does not compile or inspect an image. A target absent from the measured
profile is `InvalidCatalog` (`prepare/src/production.rs:131-175`).

### 2.2 Target and runtime policy

`TargetBuildSpec::validate` binds the target-specific compiler and runtime
identity to the measured `TargetIdentity`:

| target | required measured identity | additional policy |
| --- | --- | --- |
| `KernelTarget::Nvidia` | backend `nvidia-cuda-driver`, ABI `elf64-cubin`, architecture `sm_<major><minor>` | CUDA policy must carry the same cubin ABI, a non-inverted driver range, and the pinned CUDA toolchain identity. |
| `KernelTarget::Amd` | backend `amd-rocr-hsa`, ABI `elf64-amdgpu-code-object-v<version>`, architecture equal to the target ID or its canonical `amdgcn-amd-amdhsa--<target>` form | HSA policy is the only allowed runtime policy. |

Every target toolchain digest must be nonzero. A CUDA target with HSA policy,
or an AMD target with CUDA policy, is invalid configuration. These checks keep
measured backend and ABI names authoritative instead of allowing the builder
to reinterpret them (`prepare/src/production.rs:192-251`).

`src/native_prepare.rs::cuda_spec` and `hsa_spec` construct those specs from
the reopened native bindings. CUDA checks the deployment compute capability,
required Driver API symbols, and exact driver version, then records the pinned
LLVM/PTX assembler digests in `CudaArtifactPolicy`. HSA checks target ID and
code-object version, strips only the canonical AMDGPU prefix for
`AmdTarget`, and preserves the measured toolchain identity
(`src/native_prepare.rs:651-745`). Identical GPUs may share a spec only when
target, toolchain, scratch directory, and runtime policy are byte-for-byte
equivalent (`src/native_prepare.rs:772-785`).

### 2.3 Deferred compiler setup

`DeferredArtifactCompiler::new` sorts specs by `TargetIdentity`, validates each
spec, and rejects duplicate target identities. `with_prebuilt_bundle` accepts
one nonempty image only for an existing target spec and rejects a second image
for that target. The prebuilt image is an optimization of the build step, not a
trust bypass: all current stages are still lowered so their ABI entry points can
be checked (`prepare/src/production.rs:261-308`).

### 2.4 Materialization algorithm

`materialize(candidate, catalog, discovery)` performs these exact operations:

1. Reserve capacity for every static Draft artifact. A selected ID must exist in
   the catalog and its complete identity must equal the Draft identity. Missing
   images and identity drift are `InvalidCatalog`.
2. Group every `ArtifactBuildRecipe` by its target-spec index. The index comes
   from `candidate_target`, which scans calculation tasks for the artifact,
   follows each task's device into `DiscoveryProfile`, and requires exactly one
   target. An unused deferred artifact, missing GPU capability, or assignment to
   multiple targets is `InvalidCandidate`; a target with no spec is
   `MissingBuildTarget` (`prepare/src/production.rs:310-395`, `:609-653`).
3. Sort each target group by artifact ID. The deterministic order is the input
   order for the shared native bundle.
4. For every build, `lower_deferred_stage` finds the candidate's lowered program
   whose `source_kernel` and digest exactly match
   `ArtifactBuildProvenance::program_digest`. It invokes
   `recipe_kernel::lower_stage` with entry symbol `recipe_stage_<artifact-id>`
   and the Draft workgroup lane count. No source program is guessed when the
   digest does not match (`prepare/src/production.rs:523-547`).
5. For an AMD group, either inspect the supplied HSACO with the measured target
   ID and code-object version or call
   `ArtifactBuilder::build_hsaco_bundle(BuildPhase::Realize, ...)`. For a CUDA
   group, either inspect the supplied cubin for the expected SM or call
   `build_cubin_bundle(BuildPhase::Realize, ...)`. Build provenance must say
   `BuildPhase::Realize`, inspection count must equal lowered-stage count, and
   each inspected entry must equal the lowered ABI symbol
   (`prepare/src/production.rs:398-520`).
6. Wrap the image in one `RuntimeImage`, whose digest is computed from the
   exact bytes. For each stage, construct an `ArtifactIdentity` carrying the
   Draft artifact ID, image digest, measured format and target, pinned toolchain,
   lowered entry symbol, kernel template, resource envelope, and the original
   build provenance. Construct the matching `RuntimeArtifact` with the target
   runtime policy (`prepare/src/production.rs:549-607`).
7. Sort all static and deferred artifacts by ID and reject any duplicate ID.

`ArtifactBuilder` verifies pinned tools, verifies every LLVM module, invokes the
offline linker or assembler once per target bundle, and inspects the resulting
image. AMD uses one full-LTO HSACO containing all requested entries; NVIDIA uses
one cubin assembled from all requested PTX units (`kernel/src/builder.rs:217-330`,
`:333-445`). The production compiler therefore lowers every deferred stage but
loads no native object. Loading starts only in the candidate driver.

### 2.5 Identity pair invariant

`validate_native_artifact` is the final local gate before a pair enters a
catalog or driver. It requires:

* `ArtifactIdentity::validate` succeeds, including nonzero image and toolchain
  digests and a nonzero maximum workgroup size;
* runtime ID equals identity ID;
* runtime byte digest equals identity digest;
* runtime ABI entry equals the identity label;
* identity format equals identity target ABI; and
* runtime kind agrees with measured backend, architecture, ABI, and digest.

For CUDA, the runtime driver identity's compute capability and SHA-256 must
match the measured target and image. For HSA, runtime target ID and code-object
version must produce the exact measured ABI and architecture. The native
executor repeats the runtime contract check and additionally verifies ABI
workgroup limits and finalized calculation ABI before handoff
(`native-executor/src/plan.rs:227-352`).

## 3. Reservation and candidate realization

### 3.1 Exact reservation plan

`NativeCandidateRealizer::new` first validates the complete `MeasuredProfile`.
It then requires one compiler target spec for every calculation-capable device
in discovery. The constructor stores cloned topology and discovery values, so
later calls can reject a different profile (`prepare/src/production.rs:946-977`).

`reservation_plan` accepts only those stored values and calls
`exact_reservation_plan`. The helper visits every topology device, finds its
measured discovery entry, asks the driver for a mechanism and evidence, and
creates a deterministic label `recipe-user-<device-id>`. The bytes are exactly
`ReservationEvidence::required_bytes()`, then the complete ledger is validated
against topology (`prepare/src/production.rs:985-992`, `:1076-1116`).

The production `NativeExecutorDriver` reports `EnforcedQuota` and obtains
evidence from `ValidatedCandidateFactory`. The local factory maps host/RAM and
storage devices to `NonGpu` evidence, and GPU bindings to
`GpuDisplay { enabled_connectors }`. The core reservation contract is one
1,000,000,000-byte user reservation for non-GPU devices and display-attached
GPUs; a headless GPU has an explicit zero-byte reservation. This is a scheduler
quota, not a dummy allocation (`native-executor/src/local.rs:2145-2185`,
`core/src/plan.rs:23-132`).

### 3.2 Candidate request

`NativeCandidateRealizer::realize` first compares the candidate Draft's topology
and discovery identities with the stored measured profile and validates the
reservation ledger. It materializes the exact image set, then passes one
`CandidateRealizationRequest` containing references to:

```text
topology       -> stored measured Topology
discovery      -> stored measured DiscoveryProfile
candidate      -> exact PlannedCandidate and Draft
artifacts      -> complete static plus realized NativeArtifact slice
reservations   -> exact reservation ledger
```

The driver owns every physical object created from this request. `Session` must
retain contexts, modules, queues, completion objects, reservations, staging,
scratch, and warmed resources until explicit destruction or the one-shot
finalized handoff (`prepare/src/production.rs:724-795`).

### 3.3 Native executor validation and allocation

`NativeExecutorDriver` converts each `NativeArtifact` into the dependency-neutral
`recipe_native_executor::CandidateArtifact`, calls
`ValidatedCandidateFactory`, and maps native failure classes into the
`recipe-prepare` failure classes (`prepare/src/production.rs:797-904`). The
validation wrapper checks topology, discovery, Draft, reservations, and the
complete artifact set before invoking the physical factory. Static and deferred
IDs cannot overlap; every required ID occurs once; every runtime image is
validated; deferred identities must retain the same template, resources, and
build provenance as the Draft. Deferred stage placement must name a discovered
GPU whose target equals the realized identity (`native-executor/src/candidate.rs:43-69`,
`:475-613`).

The production `LocalCandidateFactory` then:

1. Classifies every arena device and task into Host, CUDA, HSA, or one-hop
   bridge ownership. It rejects missing or unexpected devices, CPU calculations,
   multi-hop executor-visible routes, bridge use by non-transfer tasks, and
   queue/completion slots shared by different owners
   (`native-executor/src/local.rs:2291-2558`).
2. Captures one initial free-capacity snapshot for every topology device. The
   snapshot is tied to the topology and discovery identities and is reused for
   later candidate attempts in the same factory. Initial free bytes must cover
   the exact reservation (`native-executor/src/local.rs:2187-2265`).
3. Prepares host resources and reservation paths, then calls
   `CudaPreparedResources::realize` and `HsaPreparedResources::realize` with the
   candidate's Draft, image partition, reservations, bindings, and task set.
   Cross-backend resources are realized through the configured bridge. Any
   later failure cleans up already-created host, native, and bridge resources in
   deterministic order (`native-executor/src/local.rs:1439-1547`).
4. On CUDA, creates only the queues and completion events named by the resource
   manifest, allocates pinned staging, optional scratch, four-byte metric
   buffers, egress buffers, and invocation parameter blocks. It validates the
   image against the bound deployment, inspects each cubin entry, groups equal
   image digests, loads one `Module` per distinct image, and resolves every
   logical entry function from that stable module (`native-executor/src/cuda.rs:1665-1899`).
5. On HSA, creates named queues, fine-grained staging and metric allocations,
   optional scratch and kernarg allocations, inspects each HSACO entry, loads
   one executable per distinct image, resolves each symbol, and checks loaded
   kernarg metadata against the inspected ABI (`native-executor/src/hsa.rs:1808-2049`).

No finalized arena offsets are committed in this materialization and realization
phase. The candidate session owns resource-manifest objects and native
allocations; the warm trace may allocate provisional arenas using the planner's
candidate layouts, while the final `pack_arenas` result remains authoritative.

## 4. Warm, observe, stabilize

### 4.1 Ordered passes

After `realize_candidate` returns a session, `NativeCandidateRealizer` converts
the policy pass count to `usize`, reserves snapshot capacity, and runs exactly
`1..=stabilization_passes`. Each pass must complete in this order:

```text
driver.warm_maximum_concurrency(session, candidate, pass)
driver.capacity_snapshot(session, topology, discovery)
```

The driver cannot skip a pass, reorder passes, use a different candidate, or
return a snapshot before a complete warm trace. The validated native factory
checks pass numbering and candidate identity, and its local factory tracks the
state transition `Realized -> Warmed(pass) -> Observed(pass)`
(`native-executor/src/candidate.rs:243-277`, `:356-407`; `native-executor/src/local.rs:1549-1607`).

### 4.2 What a warm pass realizes

The first warm call transitions candidate resources into a provisional warm
bundle built from the exact topology, discovery, candidate artifact identities,
and reservations. It binds host, CUDA, HSA, and bridge resources against that
bundle, prepares warm task/image/exit records, and allocates one provisional
arena per candidate arena layout. `run_warm_trace` executes the candidate's
maximum concurrent schedule. Subsequent passes reuse the bound warm resources,
but allocate fresh provisional arenas after each prior snapshot released them;
they do not load another module or allocate another native context
(`native-executor/src/local.rs:1073-1132`, `:1134-1221`).

The local factory releases warm arenas after each pass, then observes current
available bytes from each realized host, CUDA, or HSA owner. Capacity accounting
caps live availability at the immutable initial value, computes runtime
overhead as the decrease from that initial value, subtracts the exact
reservation, and emits measured `CapacityLedgerEntry` values. The first
successful snapshot is anchored for this session, so later allocator or display
counter drift cannot rewrite the scheduler contract
(`native-executor/src/local.rs:1223-1259`, `:3262-3366`).

### 4.3 Observation validation

`validate_observation` is the preparation-side fixed-point gate. It requires:

1. exactly one snapshot for each configured pass;
2. every snapshot validates against the stored topology and the exact
   reservation ledger;
3. the final `stable_tail` snapshots are byte-for-byte equal, including total,
   runtime overhead, fragmentation, safety headroom, recipe usable bytes, device
   IDs, and property provenance;
4. the final snapshot is present and becomes the candidate capacity; and
5. a temporary `RealizationProfile` built from the unchanged Draft, exact
   artifact identities, resources, reservations, and final capacity validates
   against topology, discovery, and Draft.

When the stable tail differs, the error reports per-pass and per-device byte
deltas instead of silently choosing the most favorable snapshot
(`prepare/src/lib.rs:608-755`). The observation is then included in a canonical
`recipe-realization-v3` digest. That digest includes Draft identity and
candidate data, all artifact/build identities, resources, reservations, policy,
and every snapshot (`prepare/src/lib.rs:757-780`).

## 5. Finalization and ownership handoff

After observation succeeds, `Preparer::prepare_program_validated` calls
`pack_arenas(topology, draft.arena_objects, final_capacity)`. Packing is the
first point where physical arena offsets exist. A packing failure rejects the
candidate at `FinalCapacity`, destroys its session, and asks the planner for the
next candidate.

The successful path computes a canonical
`recipe-finalized-bundle-v5` identity from topology, discovery, Draft,
realization, loop domains, reservations, capacity, and sorted arena layouts.
`FinalizedBundle::finalize_with_loop_schedule` receives the unchanged Draft and
the packed layouts. It does not compile, load, allocate, or mutate the Draft.
Only then does `Preparer` return `PreparedSystem` with the live
`PreparedNativeSession` (`prepare/src/lib.rs:439-503`, `:782-828`).

`PreparedNativeSession::into_parts` yields the same opaque driver session and
the immutable images that produced its loaded modules. Inference and training
immediately call the corresponding native executor handoff:

```text
PreparedSystem::into_parts
  -> PreparedNativeSession::into_parts
  -> ValidatedCandidateSession::into_inner
  -> LocalPreparedSession::into_backend(&finalized_bundle)
```

`LocalPreparedSession::into_backend` verifies the observed state, unchanged
artifact and reservation contracts, finalized admission images, bridge
contracts, and child backend contracts. It moves the warmed resources into the
runtime backend and rejects any leftover warm arenas. Finalized arenas are then
allocated by the runtime's normal initialization path. No module or executable
is loaded a second time (`native-executor/src/local.rs:1262-1374`).

## 6. Public callers

### 6.1 Inference

`src/inference.rs::execute_current_inference_native` obtains the current exact
profile and binding scope, derives host tuning, builds a production
`LocalCandidateFactory`, wraps it in `NativeExecutorDriver`, obtains the
deferred compiler and `NativeCandidateRealizer`, and constructs
`Preparer<NativeArtifactProvider, NativeCandidateRealizer>`
(`src/inference.rs:602-636`). The selected dense, KNN, Bayes, or GGUF program
then calls the corresponding `prepare_and_execute_local_*` function. Those
functions invoke `prepare_program`, verify the one-iteration inference
contract, map finalized external outputs, consume the prepared session, and
start the executor from the same warmed backend
(`training/src/execute.rs:1198-1255`, `:1321-1367`).

### 6.2 Training

`src/training.rs::execute_current_training_native` follows the same scope,
factory, driver, compiler, realizer, provider, and `Preparer` construction
(`src/training.rs:1278-1325`). When a native resume bundle is supplied, it first
requires matching topology and discovery identities and a matching target and
toolchain. It then calls `with_prebuilt_bundle`, which still lowers and
inspects every current stage but skips the native compiler invocation
(`src/training.rs:1285-1323`). Training calls
`prepare_and_execute_local_training_controlled`, which prepares the complete
static program, rejects loop-time external transfers, consumes the same
prepared session, and enters the immutable `init -> loop -> exit` lifecycle
(`training/src/execute.rs:2092-2133`, `:2176-2225`).

## 7. Failure classes and cleanup

### 7.1 Driver failures

`CandidateDriverFailure` has three meanings:

| failure | preparation result | cleanup rule |
| --- | --- | --- |
| `CandidateRejected { detail }` | Retry the next finite candidate | Before a session exists, the factory has destroyed all partial native objects. After a session exists, `reject_after_session` calls `destroy_candidate` first. |
| `Fatal(error)` | Abort preparation as `NativePrepareError::Driver` | A live session is destroyed on an observation or warm failure; teardown failure replaces the primary result with `NativePrepareError::Teardown`. |
| `PreFinalRealizationUnavailable { backend }` | Abort as `PreFinalRealizationUnavailable` | A backend that can bind only after Finalize cannot satisfy this boundary. |

`NativeExecutorDriver::map_executor_failure` preserves those classes. A
candidate rejection remains retryable; a native error is never converted into a
retry or alternate backend (`prepare/src/production.rs:892-904`,
`:1132-1172`).

### 7.2 `NativePrepareError`

| error | emitted when |
| --- | --- |
| `InvalidConfiguration` | Target/runtime policy, duplicate target spec, invalid reservation name, pass-count conversion, or an impossible grouped spec index. |
| `InvalidProfile` | Measured topology/discovery is missing, malformed, stale, or not validated. |
| `ProfileMismatch` | A caller supplies topology, discovery, or Draft identities different from the constructor's measured profile. |
| `InvalidCatalog` | Static image is absent, duplicated, or differs from the exact Draft identity; a catalog target is absent from discovery. |
| `InvalidCandidate` | Deferred artifact has no exact lowered program, no unique measured target, is unused, is assigned to multiple targets, or resolves twice. |
| `InvalidArtifact` | Image digest, ID, ABI entry, target format, runtime kind, build phase, inspection count, or inspected symbol disagrees. |
| `InvalidReservation` | Reservation ledger is incomplete, malformed, or does not validate against topology. |
| `MissingBuildTarget` | A measured calculation target has no pinned `TargetBuildSpec`. |
| `Lowering` | Kernel lowering, pinned-tool verification, bundle compilation, or image inspection fails. |
| `Driver` | Native allocation, loading, queue/event creation, bridge realization, warm execution, or capacity observation fails fatally. |
| `Teardown` | Destroying a rejected live session fails after the original operation. |
| `PreFinalRealizationUnavailable` | The selected backend exposes no candidate-scoped pre-Finalize path. |

The error display text is part of the public diagnostic boundary and preserves
the category, target, artifact, candidate, or teardown operation
(`prepare/src/production.rs:1174-1273`). `NativePrepareError<Infallible>` is
erased into the driver's generic error type only after materialization, so
compiler-side validation cannot fabricate a driver error.

### 7.3 Preparer-level rejection and exhaustion

`Preparer` records the candidate identity, stage (`Realize`, `Stabilize`, or
`FinalCapacity`), and detail for every retryable rejection. It calls planner
rejection bookkeeping before requesting the next candidate. A fatal realization,
teardown, or Finalize error returns immediately with prior rejection evidence.
If the finite search is exhausted, `PrepareErrorKind::CandidateExhaustion`
reports the exact attempt count and all rejections (`prepare/src/lib.rs:379-415`,
`:418-510`, `:532-552`).

## 8. Postconditions

A successful production preparation proves all of the following at one fixed
point:

* measured topology and discovery identities are unchanged from constructor to
  Finalize;
* every selected static or deferred artifact has one validated immutable image;
* every deferred image was lowered from the exact candidate program and built or
  inspected for its measured target;
* native contexts, modules/executables, queues, completion objects, staging,
  scratch, metric and egress buffers, bridge resources, and exact reservations
  were created before Finalize;
* the candidate's maximum-concurrency schedule ran for every bounded pass and
  the required capacity tail stabilized;
* final arena layouts fit the measured post-warm capacity;
* the realization and bundle identities include the exact evidence used to make
  those decisions; and
* the runtime receives the same warmed physical session, not a reconstructed
  approximation.

Compilation, discovery, allocation, loading, warming, capacity observation,
arena packing, and Finalize are therefore preparation operations. They are not
calculation or transfer work in the finalized loop, and no later runtime call
may introduce a replacement path for any of them.
