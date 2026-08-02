# Native preparation

`src/native_prepare.rs` is the root-library boundary between a validated measured
profile and the native resources that a real local training or inference run can
use. It does not schedule a graph, compile a model, or execute a loop itself. It
reopens the exact machine described by the profile, constructs owned host and
compiler inputs, and lends exact CUDA and HSA handles to one callback. The
callback is then responsible for constructing the production preparer and
running the complete `init -> loop -> exit` path while those handles are alive.

The module is deliberately fail-closed. It never chooses a device by ordinal,
product name, capacity, performance similarity, or newest cache file. Every
identity used by native execution must be present in the measured profile and
must still be observed on the current machine.

## Pipeline position

The surrounding system has two phases that are easy to confuse:

```text
recipe probe
  -> NativeProbeConfig + exhaustive discovery + bounded measurements
  -> identity-named MeasuredProfile and active-native receipt

with_current_native_preparation
  -> current host/config from the receipt (or the documented defaults)
  -> exact profile cache load
  -> exact native discovery and profile-to-inventory resolution
  -> borrowed CUDA contexts/HSA sessions
  -> owned NativeHostPlan and NativeTargetPlan
  -> callback builds the production Preparer and executes the run
```

Preparation in this module is pre-loop work. Discovery, compiler verification,
artifact compilation, native module loading, allocation, queue creation,
warmup, and capacity observation all happen before the finalized run enters
`init`. The loop cannot perform any of those operations.

The main implementations are:

| Concern | Authoritative implementation | Product used here |
| --- | --- | --- |
| Current config and profile path | `src/cli.rs::current_native_inputs` and `ActiveNativeReceipt` | `NativeProbeConfig`, `HostInventory`, `CacheIdentity` |
| Bare-metal discovery and native reopening | `native-probe/src/native.rs`, `bindings.rs`, `cuda.rs`, `hsa.rs` | `NativeGpuProbe`, `NativeExecutionBindings` |
| Profile cache and identity validation | `probe/src/cache.rs`, `codec.rs`, `resolve.rs` | `MeasuredProfile`, `ResolvedLocalInventory` |
| Target lowering inputs | `src/native_prepare.rs::build_scope` | `TargetBuildSpec`, `DeferredArtifactCompiler` |
| Deferred artifact realization | `prepare/src/production.rs` and `kernel/src/builder.rs` | `NativeArtifact`, runtime images, pinned Realize build |
| Candidate planning and stabilization | `prepare/src/lib.rs`, `prepare/src/production.rs` | `PreparedSystem`, `PreparedNativeSession` |
| Physical native execution | `native-executor` and `training/src/execute.rs` | warmed local backend and immutable bundle |

## Inputs and identities

`with_native_preparation` accepts three inputs:

* `NativeProbeConfig` contains the host RAM origin key, PCI sysfs root, ordered
  CUDA and ROCr/HSA library candidates, PTX ISA, HSA code-object version, the
  offline LLVM/linker/`ptxas` toolchain, a human-readable release label, an
  absolute scratch directory, and the bounded probe FMA-chain length. The CLI
  receipt canonicalizes the PCI root before reopening it.
* `MeasuredProfile` is the fully validated, hashed result of `recipe probe`.
  It contains machine fingerprints, retained RAM/storage/GPU origin keys,
  measured topology and discovery capabilities, benchmark evidence, and the
  profile cache identity.
* `HostInventory` is a fresh host discovery. Its RAM, storage, and machine
  fingerprint are matched to the profile; its storage domains also provide the
  benchmark roots used to derive run-scoped arena paths.

`load_cached_measured_profile` instead accepts one absolute profile path. The
path name supplies the expected `CacheIdentity` and the file supplies the
profile bytes. `load_native_preparation` combines this path-based load with a
fresh native reopen and returns the profile together with an owned target plan.

The CLI normally obtains the config from an active-native receipt written by
`recipe probe`. The receipt pins library and tool paths and their digests, the
profile path and identity, PCI root, scratch directory, PTX ISA, HSA
code-object version, release label, and FMA-chain length. If no receipt exists,
`current_native_inputs` derives the same shape from the topology contract and
the current host, computes the exact discovery-only cache identity, and selects
that identity-named profile path. It never searches for an arbitrary profile.

## Public data products

### `NativeDeviceBuildTarget`

One entry describes one measured GPU:

* the stable topology `DeviceId`;
* the retained native origin `Label` (for example the CUDA UUID plus PCI BDF or
  the HSA UUID plus PCI address);
* the exact `TargetIdentity` from measured discovery; and
* the exact measured `ToolchainIdentity`.

The entry is an owned record. It does not contain a context, session, allocator,
or other live handle.

### `NativeTargetPlan`

The plan owns the resolved `MachineId`, every local GPU entry, and one
`TargetBuildSpec` for each unique calculation target. Identical GPUs retain
separate entries in `devices` but share one compiler specification in
`target_build_specs`. The plan can be converted into a checked
`DeferredArtifactCompiler`; the compiler sorts targets by `TargetIdentity`,
rejects duplicate specifications, and validates every target/runtime-policy
pair.

### `NativeHostPlan`

The host plan owns the resolved machine, sorted RAM device IDs, and sorted
`(DeviceId, benchmark_root)` storage pairs. `backend_config` is pure plan
construction: it creates RAM bindings and deterministic disk specs named
`.recipe-run-{run}-device-{device}-arena`, then calls
`HostBackendConfig::new`. It performs no filesystem I/O. Candidate realization
later creates each arena with `create_new`, so an overlapping run ID is a real
failure rather than an overwrite.

### `LoadedNativePreparation`

This is the path-oriented product of `load_native_preparation`: the owned
`MeasuredProfile` and its reopened `NativeTargetPlan` travel together. The
profile is available for downstream planner inputs, and the target plan is
available for deferred native compilation.

### `NativePreparationScope<'cuda, 'hsa>`

The scope combines:

1. `NativeExecutionBindings<'cuda, 'hsa>`, whose CUDA bindings borrow reopened
   contexts and whose HSA bindings borrow reopened sessions and host allocators;
2. the owned `NativeHostPlan`; and
3. the owned `NativeTargetPlan`.

The callback passed to `with_native_preparation` is higher-ranked over both
lifetimes. Therefore a callback may move the scope into the production factory
and use it for the run, but it cannot return a value that borrows a CUDA context
or HSA session. `into_targets` deliberately drops the bindings before returning
an owned target plan. `into_parts` is used by training and inference to hand
the bindings, host plan, and target plan to their production setup together.

## Loading one measured profile

`load_cached_measured_profile` performs two identity checks before it returns a
profile:

1. `cache_identity_from_path` requires a UTF-8 filename of the exact form
   `measured-v<decimal-schema>-<64-lowercase-hex-digits>.recipe-profile`. The
   schema must fit `u32`, and the digest is decoded into the 32-byte
   `CacheIdentity` digest. Uppercase hex, a wrong length, a missing separator,
   an extra separator, or a different suffix is rejected as
   `InvalidCachePath`.
2. `ExplicitPathProfileCache::new` and `ProfileCache::load` enforce the cache
   boundary. The path is absolute and names a file; its parent is a canonical,
   real directory owned by the effective user with no group/other permissions;
   the target is a private regular file, not a symlink, and is opened with
   `O_NOFOLLOW`. Device/inode identity is checked after opening, the profile
   size is bounded, and the codec verifies the magic, codec schema, SHA-256
   checksum, decoded field structure, and complete `MeasuredProfile` validation.
   Finally the profile's embedded `cache_identity` must equal the identity
   decoded from the filename.

`None` from the cache means the exact file is absent, which is converted to
`ProfileNotFound`. There is no newest-file fallback and no attempt to use a
different profile whose measurements look similar.

`with_current_native_preparation` has the same exactness property but receives
the path and `CacheIdentity` from `current_native_inputs`, then calls
`cache.load(current.profile_identity)`. It does not infer an identity from an
unrelated path or select another file.

## Reopening the exact local machine

### Probe construction and discovery

`with_native_preparation` constructs a new `NativeGpuProbe` from the supplied
config. `NativeGpuProbe::new` validates the nonzero FMA-chain length and
absolute scratch directory; the HSA backend also requires a nonzero code-object
version. It then constructs both CUDA and HSA backends with `exhaustive = true`.
A backend library may be absent only when PCI discovery proves that vendor is
absent. If hardware is present, a missing library, failed load, failed
enumeration, or changed identity is an error.

`NativeGpuProbe::discover_all` asks every configured backend to discover all
devices, sorts descriptors by stable origin key, and rejects duplicate keys.
CUDA descriptors include the driver/device deployment, PCI surface, compute
capability, runtime and firmware identities, queue limits, and a toolchain
identity. HSA descriptors include the stable agent UUID and PCI surface, the
exact selected AMD ISA target, queue and memory limits, runtime identity, and
the configured code-object/toolchain identity. Both backends retain enough
identity for the later binding reopen to compare the current hardware with the
profile.

### Profile-to-inventory resolution

`with_native_preparation_from_probe` first calls `discover_all`, then
`MeasuredProfile::resolve_local_inventory`. Resolution validates the profile,
requires an exhaustive GPU inventory, matches the current machine fingerprint
to exactly one measured machine, and requires equal stable key sets for local
RAM, storage, and GPU domains. It also checks that each storage/GPU domain
references a currently present RAM key and that each GPU's current calculation
target equals the measured target. Missing keys, newly visible keys, duplicate
live keys, changed targets, or a changed machine fingerprint fail as a probe
profile mismatch. Capacity, product name, ordinal, and benchmark similarity
are not selectors.

The resolver returns borrowed `ResolvedLocalInventory` values. The root module
immediately converts them into owned plans:

* `owned_host_plan` copies and sorts RAM IDs and storage roots;
* `owned_gpu_inventory` clones descriptors and sorts `(DeviceId, descriptor)`;
* `require_complete_local_gpu_scope` rejects a profile whose calculation GPU
  set contains any device belonging to another machine or absent from the
  resolved local set. A local run cannot silently cover only part of a
  multi-machine profile; distributed execution needs a different entrypoint.

An empty local calculation-GPU set is rejected even if host resolution itself
would succeed.

### Native binding reopen

The helper then calls `with_native_execution_bindings`. That function performs
another discovery and profile resolution before creating handles, so the
identity is checked at the exact point where native resources are opened as
well as at the earlier planning point.

It partitions the resolved GPU descriptors by the measured backend and records
the enabled DRM display-connector count from the canonical PCI sysfs BDF. CUDA
reopening loads the selected Driver library, re-enumerates devices, verifies
each descriptor and deployment identity, creates one context per measured GPU,
and requires every expected origin to be seen exactly once. HSA reopening uses
the probe's retained ROCr runtime, re-enumerates agents, verifies every GPU
origin and exact ISA target, requires queue limits, selects one unambiguous
CPU host allocator by NUMA rules, and creates one HSA session per measured GPU.

The result is `NativeExecutionBindings` containing sorted, preparation-scoped
CUDA and HSA bindings. A missing backend is acceptable only when the measured
profile has no GPU for that backend. A reopened device outside the profile,
duplicate origin, missing expected origin, changed runtime surface, missing HSA
queue/allocator, or context/session construction error aborts the scope.

`NativeGpuProbe` retains the ROCr runtime in its HSA backend. The
`CURRENT_NATIVE_PROBE` thread-local stores `(NativeProbeConfig, NativeGpuProbe)`
for the current-native entrypoint. The first call installs the probe; later
calls reuse it and rebuild all per-run host/compiler/preparer resources. If a
later call reports a different config, the thread returns `IdentityMismatch`
instead of reopening a second runtime with a different identity. The cached
probe is thread-local, so it does not create a process-wide handle registry.

## Building the preparation scope

`build_scope` turns the exact binding set and measured descriptors into the
compiler inputs used by `recipe-prepare`.

1. The binding machine must equal the machine selected by profile resolution.
2. CUDA and HSA binding vectors are indexed by `DeviceId`; duplicate entries
   are rejected independently. The union of both maps must equal the measured
   local GPU set, and a device may not appear in both backends.
3. One `ArtifactBuilder` is created from the configured offline toolchain.
   Construction immediately rechecks every pinned executable path and digest:
   LLVM `opt`, LLVM `llc`, and any configured `lld` or `ptxas`.
4. Each measured device is converted into a backend-specific
   `TargetBuildSpec` and a `NativeDeviceBuildTarget` entry.
5. Specs are deduplicated by `TargetIdentity`. Identical GPUs may share a
   spec, but duplicate devices must produce equivalent target, toolchain,
   scratch, and runtime policy values. Any difference is an identity failure.
6. `DeferredArtifactCompiler::new` sorts and validates the unique specs again,
   producing the checked compiler retained by the plan.

### CUDA target specification

`cuda_spec` obtains the SM major/minor and driver version from the reopened
`DeploymentIdentity`, converts them to the checked `NvidiaTarget`, and combines
that target with the configured PTX ISA. It then requires:

* measured ABI `elf64-cubin`;
* measured architecture equal to `NvidiaTarget::architecture()` (`sm_Mm`);
* every symbol in `recipe_cuda::REQUIRED_DRIVER_SYMBOLS` to be advertised by
  the reopened Driver API; and
* a pinned NVIDIA assembler, because `cuda_toolchain_identity` records the
  `ptxas` digest.

The resulting runtime policy is `RuntimeArtifactPolicy::Cuda` with the exact
deployed driver as both the minimum and maximum driver, the required symbol
set, and a CUDA toolchain identity containing the release label, LLVM tool
digests, PTX ISA, `ptxas` digest, and `elf64-cubin` format. The legacy CUDA
`zig_version` field is explicitly marked as not used by the Rust-owned IR path,
and no CUDA Toolkit API is claimed: Recipe invokes only the pinned assembler.

### HSA target specification

`hsa_spec` requires the reopened binding's full target ID to equal the measured
architecture and its code-object version to equal the configured version. The
binding target must carry the canonical `amdgcn-amd-amdhsa--` prefix. The
prefix is removed only for the kernel crate's `AmdTarget`, whose target tail
must validate as a `gfx...` target; the measured `TargetIdentity` retains the
full discovery architecture and measured ABI. The runtime policy is
`RuntimeArtifactPolicy::Hsa`.

Both backend specs retain the measured descriptor's `ToolchainIdentity`, the
shared checked `ArtifactBuilder`, and the configured private scratch parent.

## From target plan to artifacts and native execution

The root module stops after producing an owned target plan or a callback scope.
The following steps are the authoritative consumers of that product.

### Training and inference callers

`src/training.rs::execute_current_training_native` and
`src/inference.rs::execute_current_inference_native` both call
`with_current_native_preparation`.

Inside the callback they:

1. training optionally compares a resumed native bundle's topology, discovery,
   target, and toolchain identities with the current profile/plan;
2. consume the scope into bindings, host plan, and target plan;
3. derive hardware-based runtime tuning from the measured profile and machine;
4. call `NativeHostPlan::backend_config` with a fresh `RunId` to produce
   deterministic host resources;
5. clone bindings for `StagedCrossBackend`, then construct
   `LocalCandidateFactory::production`, `NativeExecutorDriver`, and the
   `DeferredArtifactCompiler` from the target plan;
6. optionally attach an exact prebuilt native bundle for resume; and
7. construct `NativeCandidateRealizer`, `NativeArtifactProvider`, and
   `Preparer` before calling the real training or inference execution entry
   point.

The callback keeps the borrowed native handles alive through preparation and
the final handoff. No declaration stores a dynamic CUDA context or HSA
session.

### Deferred compilation and prebuilt loading

`DeferredArtifactCompiler` is intentionally retained until candidate
realization, not run while declarations are compiled. `Preparer::prepare_program`
validates the measured profile, requests exact reservation evidence, computes
an optimistic planning capacity, resolves the native artifact catalog, and
enumerates finite planner candidates.

For each candidate, `DeferredArtifactCompiler::materialize`:

* copies exact static catalog artifacts selected by the draft;
* groups deferred `ArtifactBuildRecipe` values by the one measured target to
  which each artifact is placed;
* finds the exact lowered program by source-kernel and program digest;
* lowers each stage with generated entry `recipe_stage_<artifact-id>` and the
  draft's workgroup geometry;
* if a prebuilt bundle was supplied for that target, structurally inspects its
  CUDA or HSA entries and reuses its bytes without invoking a compiler; or
* otherwise calls `ArtifactBuilder::build_cubin_bundle` or
  `build_hsaco_bundle` in `BuildPhase::Realize`, verifies the returned
  inspection count and entry names, and creates one immutable runtime image
  for the bundle.

Each resulting `NativeArtifact` carries the finalized artifact ID, byte digest,
format, target, toolchain, entry ABI, kernel template, resources, and build
provenance. CUDA runtime identity includes compute capability, exact driver
range, required Driver symbols, and the pinned compiler identity. HSA runtime
identity includes the measured architecture and code-object version. The
native artifact validator cross-checks IDs, digests, entry symbols, formats,
targets, and runtime kind before any driver sees the image.

The kernel builder itself verifies the pinned tools again, verifies LLVM IR with
`opt`, invokes `llc` as required, and packages all entries for one target in one
HSACO or cubin bundle. Its public compiler entry points accept only `Offline` or
`Realize` phases. No compiler entry point exists for Finalize, `init`, `loop`, or
`exit`.

### Planner-produced Drafts

The planner runs between target-plan construction and native artifact
realization. `Preparer::prepare_program` passes the measured topology,
discovery, reservation ledger, optimistic planning capacity, and the catalog's
artifact identities to `plan_program_candidates` in `planner/src/planner.rs`.
That entrypoint validates the static program, calculation graph, topology,
discovery, reservations, and capacity before it does any candidate work.

The planner lowers Recipe primitives to target-independent `LoweredProgram`
values using common measured hardware bounds, derives stage-scoped kernel
template identities, and enumerates every finite placement assignment over the
available GPU calculation devices. For each assignment it builds one immutable
`DraftPlan` containing:

* init images, logical values, aliases, and arena lifetimes;
* calculation, transfer, and specialized metric tasks with dependency edges;
* target-independent deferred `ArtifactBuildRecipe` values, each carrying its
  source-kernel and program digest, stage ordinal, affine argument views,
  dispatch geometry, work bounds, fault flag, and resource envelope; and
* the exact measured-profile identities, selected static artifact identities,
  resource manifest, output copies, and loop domains.

If a catalog entry exactly matches a deferred recipe's stage provenance,
resource bounds, kernel template, and measured target, the planner keeps that
entry as a static artifact. Otherwise it retains the target-independent build
recipe for `DeferredArtifactCompiler::materialize`; target, format, entry
symbol, and toolchain are intentionally absent from the recipe at Draft time.

The planner lowers multi-hop movement into dependency-chained one-link transfer
tasks, then the scheduler computes deterministic windows from measured rates,
queue limits, compute concurrency, link lanes, duplex contention, and
transfer/compute overlap. It packs arena objects against the optimistic
capacity and rejects assignments that cannot fit. Valid candidates are sorted
by measured makespan and stable candidate identity. `ProgramPlannerSearch` is a
one-shot stream: an identity is issued once, and only an issued candidate may
be rejected. A rejected candidate is never returned again.

The optimistic capacity is only a finite enumeration bound. After the native
driver has loaded and warmed the same Draft, its measured capacity snapshots
replace that bound for final arena packing. This separation is why a target
plan can be reused across candidates while the concrete native image and
resource session remain candidate-scoped.

### Candidate realization, loading, warmup, and finalization

`NativeCandidateRealizer` owns the bridge from immutable planning to physical
resources:

1. It validates that the target plan covers every measured calculation target.
2. It obtains one exact reservation entry per topology device from the native
   driver. The production local driver reports the required Recipe headroom
   evidence from host bindings and GPU display-connector state.
3. It checks candidate topology/discovery identities and reservation validity,
   materializes all artifacts, and sends one `CandidateRealizationRequest` to
   `NativeExecutorDriver`.
4. `NativeExecutorDriver` wraps `ValidatedCandidateFactory`, which validates
   topology, discovery, Draft, reservations, artifact identities, target
   placement, and runtime image contracts before delegating to
   `LocalCandidateFactory::production`.
5. The local factory prepares host resources, CUDA/HSA device resources, exact
   module/image inputs, queues, pending pools, and staged cross-backend tasks.
   CUDA and HSA prepared resources reject missing, duplicate, or unexpected
   runtime artifacts and devices.
   On CUDA, each distinct image is checked against the reopened deployment,
   inspected for the exact SM and entry symbol, loaded once as a Driver module,
   and resolved into one function per logical artifact. On HSA, each distinct
   HSACO image is inspected for the exact target, code-object version, and
   entries, loaded once as an executable, and resolved through the runtime
   symbol table with kernarg metadata checks. Both paths then pre-create the
   measured queues and completion objects, per-device staging and optional
   scratch, argument/invocation storage, four-byte metric buffers, and fixed
   exit buffers before warmup.
6. For every bounded stabilization pass, the realizer runs the candidate's
   maximum-concurrency warm trace, then records one capacity snapshot. The
   default policy is three passes with a stable two-snapshot tail.
7. A successful result is `PreparedNativeSession`: the opaque native session
   plus immutable images, and `RealizationObservation`: artifact identities,
   resource evidence, reservations, and all post-warm capacity snapshots.

`Preparer` validates the observation, repacks arenas against the observed
capacity, hashes the realization and bundle identities, and finalizes one
immutable `FinalizedBundle`. If a candidate is rejected during realization,
stabilization, or final capacity packing, its live session is destroyed before
the planner tries another finite candidate. Finalize never changes a Draft
choice.

The training and inference execution functions consume the prepared system,
move the validated session into `LocalPreparedSession::into_backend`, and reuse
the warmed host/CUDA/HSA resources. The handoff rejects a session that was not
fully warmed and observed, or whose bundle identity differs. Only after this
handoff does `PreparedRun` admit `init` images and start the immutable loop.
This is why native preparation is part of the ahead-of-run fixed-point
boundary, not a lazy loader for the loop.

## State and ownership transitions

The complete transition is:

1. **Configuration state:** `recipe probe` or `current_native_inputs` produces
   a config whose tool paths, libraries, scratch root, and scalar choices are
   pinned. A later config change on a cached thread is rejected.
2. **Profile state:** an exact identity-named cache file is decoded and fully
   validated. Missing or stale bytes stop the transition.
3. **Inventory state:** exhaustive native and host inventories are reopened and
   associated with measured topology IDs by retained keys.
4. **Owned plan state:** borrowed resolver entries become sorted owned host and
   target plans. The target plan contains no live native handle.
5. **Binding scope state:** exact CUDA contexts and HSA sessions are lent to one
   higher-ranked callback. The scope is the only root-module value that carries
   those borrowed handles.
6. **Candidate state:** the callback turns the scope into a production factory;
   deferred stages are lowered/built or exact prebuilt images are inspected,
   then host/native resources are loaded and allocated for one immutable Draft.
7. **Warmed state:** maximum-concurrency traces and capacity snapshots complete
   for every stabilization pass. Rejected candidates are destroyed here.
8. **Finalized state:** the stable candidate is packed and finalized; its warmed
   session crosses into the run lifecycle exactly once. The compiler and
   preparer are no longer needed by the execution loop.
9. **Run and teardown:** `init`, the fixed loop, `exit`, and ordered teardown
   use only the finalized bundle and the handed-off resources. The callback
   returns only after these borrowed handles are no longer needed.

The current-native thread-local keeps the probe root, not a mutable plan or a
substitute inventory. Every call still rebuilds host paths, compiler specs,
artifact catalogs, candidates, and run resources for its own profile and
`RunId`.

## Invariants

The following invariants are enforced by this module or immediately by its
authoritative consumers:

* The current machine fingerprint and complete local RAM/storage/GPU key sets
  equal the measured profile.
* Every measured calculation GPU is inside the local scope. Nonlocal GPUs are
  never silently dropped.
* The binding machine equals the resolved machine; every measured GPU has
  exactly one native binding; no binding is duplicated or assigned to both
  backends.
* A target identity has one equivalent build policy. Equal GPU targets share a
  compiler specification; different policies for one identity are rejected.
* Every compiler tool path is canonical and its bytes match the pinned digest.
  NVIDIA preparation requires a pinned `ptxas`; AMD preparation requires a
  pinned ELF linker during HSACO build.
* Measured target ABI and architecture agree with the reopened CUDA deployment
  or HSA binding, including Driver symbols and HSA code-object version.
* The callback cannot leak a borrowed context or session. No later dynamic
  placement path can reopen a different device behind the declaration.
* Host disk arena names are deterministic and run-scoped, and realization owns
  their create-new semantics.
* Deferred compilation, module loading, resource allocation, and warmup happen
  in Realize/preparation. Finalize and the run lifecycle do not compile, load,
  discover, resize, or replan.
* A successful final handoff reuses the exact warmed resources. It does not
  perform a second load or allocation pass after Finalize.

## Failure taxonomy

`NativePreparationError` is the root facade's pre-final error type:

| Variant | Meaning at this boundary |
| --- | --- |
| `InvalidCachePath` | The caller supplied a path whose identity filename shape is not accepted. |
| `ProfileNotFound` | The exact requested profile file is absent. |
| `Probe(ProbeError)` | Discovery, profile codec/validation, cache security, inventory resolution, backend loading, or native binding reopening failed. |
| `Lowering(LoweringError)` | The pinned `ArtifactBuilder` or a checked CUDA/HSA target cannot be constructed. |
| `TargetSpecification(NativePrepareError<Infallible>)` | The deduplicated target table fails deferred-compiler validation, such as a duplicate target or mismatched runtime policy. |
| `LocalConfiguration` | CLI current-input reconstruction or a caller's host/tuning setup failed and was reported through this boundary. |
| `IdentityMismatch` | Machine, device set, backend, binding, deployment, target, toolchain, or per-target policy identities do not agree. |

Downstream `NativePrepareError` adds invalid profile/catalog/candidate/artifact,
missing build target, driver, teardown, or pre-final-realization failures.
`CandidateRejected` is the bounded retry path for one finite candidate only;
the live session must be destroyed before the planner can try the next one.
Fatal driver, teardown, profile, artifact, or finalization errors abort the
preparation rather than hiding the broken transition with a fallback.

## Callers and source map

The root crate re-exports all public products from `src/lib.rs`. The production
callers are:

* `src/training.rs::execute_current_training_native`, for native training,
  optional native-kernel resume, and the controlled training lifecycle;
* `src/inference.rs::execute_current_inference_native`, for dense, KNN, Bayes,
  and GGUF-derived native inference; and
* `acceptance/src/main.rs::capture_native_profile`, which records the exact
  profile, device origins, target identities, and toolchain identities used by
  an acceptance run.

The source-of-truth files for this page are:

* [`src/native_prepare.rs`](../../src/native_prepare.rs), the public facade and
  all scope/target-plan logic;
* [`src/cli.rs`](../../src/cli.rs), current-input reconstruction and the active
  native receipt;
* [`native-probe/src/bindings.rs`](../../native-probe/src/bindings.rs), exact
  context/session reopening;
* [`native-probe/src/native.rs`](../../native-probe/src/native.rs), exhaustive
  native discovery;
* [`probe/src/cache.rs`](../../probe/src/cache.rs),
  [`probe/src/codec.rs`](../../probe/src/codec.rs), and
  [`probe/src/resolve.rs`](../../probe/src/resolve.rs), profile persistence and
  identity association;
* [`prepare/src/production.rs`](../../prepare/src/production.rs), deferred
  compilation, artifact identity, candidate realization, and stabilization;
* [`prepare/src/lib.rs`](../../prepare/src/lib.rs), finite planning, capacity
  checks, and Finalize; and
* [`kernel/src/builder.rs`](../../kernel/src/builder.rs) plus
  [`native-executor/src/local.rs`](../../native-executor/src/local.rs), the
  actual Realize compiler and warmed native resource handoff.
