# `training/src/execute.rs`: prepared native execution

This document describes the implementation in [`training/src/execute.rs`](../../src/execute.rs). The file is the execution boundary for the `training` crate. It does not parse public declarations, prepare data, compile a dense graph, discover hardware, or write user artifacts. Those operations produce a `CompiledTraining`, `CompiledInference`, or `CompiledKnnInference`, a measured [`MeasuredProfile`](../../../probe/src/lib.rs), and a production-configured [`Preparer`](../../../prepare/src/lib.rs). This module joins those immutable inputs to one warmed native session, drives the executor's `init -> loop -> exit` state machine, and returns evidence only after native teardown.

The module also contains the target-free inference wrappers. Training and inference share preparation, native handoff, fixed-capacity polling, and teardown, but their boundaries differ:

- training may have a finite or unbounded loop, admits data only in `init`, drains user metrics, and accepts a host stop only at a completed loop iteration;
- dense and KNN inference require exactly one loop iteration, reject user metrics and loop external transfers, and validate typed prediction images after `exit`.

All public names from this private module are re-exported by [`training/src/lib.rs`](../../src/lib.rs:51-60). The root facade uses the training entrypoint through [`src/training.rs`](../../../src/training.rs), while direct crate users can call the public functions documented below with their own `Preparer` and measured profile.

## Position in the production call path

The normal dense training route is:

```text
public recipe declarations
  -> src/training.rs::compile_training_package
       -> CompiledTraining + CheckpointManifest + optional native resume
  -> install SigintGuard
  -> src/training.rs::execute_current_training
  -> execute_current_training_native
       -> current measured native scope and NativeRuntimeTuning
       -> production LocalCandidateFactory and NativeCandidateRealizer
       -> Preparer::prepare_program(training.program(), profile)
       -> prepare_and_execute_local_training_controlled
            -> FinalizedBundle + warmed PreparedNativeSession
            -> LocalPreparedSession::into_backend(&bundle)
            -> PreparedRun::prepare(..., Watchdog)
            -> initialize(DeviceImage...)
            -> start_loop()
            -> poll_with_progress_or_stop(...)
            -> into_exited_loop()
            -> exit()
            -> CompletedTrainingExecution
  -> TrainingReport::dense
  -> optional `.ogdl` and/or native-kernel save
```

`Train::try_run_with` is the caller-side branch point. Bayesian declarations call semantic preparation and never reach this module. A standalone KNN declaration builds a reference model and also never reaches this module. Every ordinary dense model reaches the controlled native entrypoint after compilation and, when requested, resume validation. The execution module therefore receives a complete static program, not a mutable builder or a partially prepared graph.

Inference follows the same preparation and native handoff, but calls `prepare_and_execute_local_inference` or `prepare_and_execute_local_knn_inference`. Those functions use recoverable executor transitions so an `ExecutorError` can be returned with run, bundle, journal, and cleanup evidence.

## Public execution surface

### Limits and graceful-stop control

`TrainingExecutionLimits` and `InferenceExecutionLimits` each hold the caller-selected executor `Watchdog`. `new` is a transparent constructor. The watchdog is passed to `PreparedRun`; it is not a host timeout around the wrapper. Journal capacity is derived from the finalized bundle after preparation, and pending polls use one fixed counter per task rather than an allocation proportional to a loop bound.

`TrainingExecutionControl<'a>` is a read-only wrapper around an optional `&AtomicBool`:

- `uninterrupted()` has no stop source and is suitable for finite training;
- `graceful_stop(flag)` gives the executor an Acquire read of the caller's flag;
- `validate_training_execution_control` rejects an unbounded `LoopIterations` value when no stop source exists.

The wrapper cannot mutate graph values, task state, or native resources. The flag is read by `RunningRun::poll_with_progress_or_stop` only after the active loop iteration is terminal. A request that arrives during a calculation, transfer, fault readback, or metric readback therefore cannot interrupt that work.

### Live metric observation

`bounded_training_metric_channel(capacity, selected, cadence)` creates a bounded synchronous channel and a `TrainingMetricObserver`. `selected` is converted to a deterministic `BTreeMap<MetricId, u64>` of independent counters. `cadence` is nonzero. The consumer receives cloned `TrainingMetricSample` values; it receives no capability over the running executor.

`TrainingMetricObserver::try_observe` ignores metrics not in `selected`, increments that metric's counter and global `observed` count, and offers only every `cadence`th sample. `try_send` is deliberately nonblocking. A full or disconnected channel increments `dropped`; it never waits, retries, or affects executor polling. `TrainingMetricObserverStats` reports `observed`, `cadence_eligible`, `delivered`, and `dropped`.

`TrainingMetricSample` converts an executor `MetricSample` into the user boundary. `sequence` remains the mailbox sequence, `zero_based_iteration` remains the immutable schedule coordinate, and `epoch` is a checked one-based `NonZeroU64` value. It also retains task, metric slot, metric identity, and the executor `MetricValue` (`F32` or `I32`). The executor's coordinate is never presented as a one-based epoch without this conversion.

`FinalTrainingMetric` is one statically planned user metric slot and its newest retained sample. `sample` is `None` only when that planned task never activated. The live channel is optional; final retention is independent of whether a consumer receives a notification.

### Native image identity and completed training evidence

`NativeKernelFormat` is `Cubin` or `Hsaco`; `extension()` returns the corresponding public artifact extension. `RealizedNativeKernel` stores the exact runtime bytes and the target, toolchain, digest, and entry identities from successful preparation. `RealizedNativeKernelSet` adds the realization, topology, and discovery identities and contains the deduplicated images handed to the executor.

`retain_realized_native_kernels` converts `NativeArtifact` runtime kind to `Cubin` or `Hsaco`. Images are merged only when format, target, toolchain, digest, and bytes all match; matching artifact identities are appended to one `entries` list. Thus a returned image is the exact realized image, not a recompiled or reconstructed artifact.

`CompletedTrainingExecution` is created only after `ExitedLoop::exit` has released arenas and destroyed native resources. It retains:

| Field | Meaning |
| --- | --- |
| `run` | caller-provided `RunId`; it is not regenerated after handoff |
| `bundle` | identity of the finalized bundle actually executed |
| `external_outputs` | sorted post-exit `ExitImage` values |
| `external_output_values` | logical `ValueId` for each image, index-aligned with `external_outputs` |
| `metrics` | one `FinalTrainingMetric` per planned user slot |
| `native_kernels` | exact realized native images and measured identities |
| `native_evidence` | native image, queue, completion, allocation, and teardown evidence |
| `training_evidence` | compiled bounds joined with actual journal observations |
| `journal` | bounded logical-event and physical-call record |

Accessors expose each field without allowing a live native handle. `into_parts` consumes the completed value and returns the same evidence components, preserving output/value index alignment.

`TrainingExecutionEvidence` is derived from two independent sources. `PlannedTrainingExecutionEvidence::new` reads the compiled `TrainingBounds`, optimizer progress, parameter update kernels, and finalized bundle tasks. `complete` then counts actual `RunJournal` records. It exposes:

- `bounds` and `logical_updates_per_epoch`;
- optimizer parameter kernel, task, and physical submission counts;
- loop iterations started and completed;
- finalized loop calculation task count;
- non-GPU calculation task and submission counts;
- compacted logical-event and physical-call totals.

The optimizer and non-GPU submission counts are matched by task identity against `PhysicalCall::SubmitCalculation`, not inferred from a success status or a text log. Iteration counts come from `LogicalEvent::LoopIterationStarted` and `LoopIterationCompleted`. The journal summary records how much repeated detail was compacted while preserving the exact bounded storage contract.

## Prepared system and native session

The controlled entrypoint receives a mutable `Preparer<A, R>` whose type parameters are constrained to the production handoff:

```text
Preparer<ArtifactProvider, CandidateRealizer>
  -> PreparedNativeSession<
       ValidatedCandidateSession<
         LocalPreparedSession<'cuda, 'hsa, Bridge, Bridge::Resource>
       >
     >
```

`Preparer::prepare_program` validates its policy and measured profile, asks the realizer for reservations, resolves the artifact catalog, plans finite program candidates, and realizes candidates in order. Candidate realization compiles deferred artifacts, allocates and loads the candidate's native resources, executes the bounded maximum-concurrency stabilization passes, observes capacity, packs arena layouts, hashes the realization and bundle identities, and calls `FinalizedBundle::finalize_with_loop_schedule`. A rejected candidate is torn down before the next candidate is considered. A successful `PreparedSystem` contains one immutable finalized bundle, one realization profile, the artifact catalog, the same warmed native session, and logical exit mappings.

`PreparedNativeSession::into_parts` consumes only the wrapper and returns the exact driver session plus immutable native artifacts. It does not destroy or recreate the warmed session. The controlled entrypoint then calls `ValidatedCandidateSession::into_inner` and `LocalPreparedSession::into_backend(&bundle)`. That handoff validates candidate, topology, discovery, reservations, artifacts, task ownership, bridge, host, CUDA, and HSA identities. It consumes the prepared resource exactly once. The resulting `LocalBackend` contains only pre-realized resources and fixed task partitions; its running API has no compiler, loader, allocator, queue creator, or topology mutation.

`NativeExecutionEvidence` is cloned from the backend immediately after `ExitedRun::into_parts` returns it. Successful evidence records the participating device backends and their image loads, entry lookups, queues, completion objects, and persistent allocations, plus `loop_realization_calls == 0`, `teardown_completed == true`, and `live_resources_after_teardown == 0`. A failed run does not fabricate completed native evidence.

## Training image admission

`build_training_device_images(training, bundle)` is the public packing boundary. It calls `pack_device_images(training.external_inputs(), bundle.init_images())` and returns one `DeviceImage` per finalized init manifest.

The packer performs these operations in order:

1. Index all `OwnedExternalInput` values by logical `ValueId`; a duplicate is `DuplicateExternalInput`.
2. Require each `InitDataImage` to name a unique device; a repeated device is `DuplicateInitDevice`.
3. Convert the manifest byte count to host `usize`; an unrepresentable size is `ImageSizeUnsupported`.
4. Allocate a zero-filled image. Sort members by `(image_offset, logical)` so validation and copying are deterministic. Gaps and reserved fault-buffer bytes remain zero.
5. Reject duplicate logical members, missing external inputs, dtype mismatches, and byte-size mismatches.
6. Checked-add each member offset and size, require the range to lie within the manifest image, and require conversion to host indices. Failures are `ImageMemberOutOfBounds`.
7. Compare each sorted member with the preceding range; an overlap is `ImageMembersOverlap`.
8. Copy the input bytes into the validated range and emit `DeviceImage::new(manifest.device, manifest.image, bytes)`.
9. Sort the resulting images by device.

A logical input may occur in several device manifests and is copied into each image, but it is admitted once per device by the executor's init phase. An input that is not used by any selected manifest is not uploaded and is not an error. The packer performs no host calculation or transformation.

## The controlled training lifecycle

`prepare_and_execute_local_training_controlled` is the single implementation for the three training entrypoints. It takes a compiled program, measured profile, production preparer, run ID, watchdog limits, an optional metric observer, and a `TrainingExecutionControl`.

### Entry wrappers

`prepare_and_execute_local_training` calls the controlled function with no observer and `TrainingExecutionControl::uninterrupted()`. It is the plain finite-run API, but the controlled function still rejects an unbounded program because no stop source was supplied.

`prepare_and_execute_local_training_with_observer` supplies `Some(observer)` and otherwise uses uninterrupted control. It does not duplicate lifecycle logic.

`prepare_and_execute_local_training_controlled` is the path used by the root dense runner. The root runner supplies the process-wide SIGINT flag through `TrainingExecutionControl::graceful_stop`, so unbounded training has an explicit safe-stop capability.

### Preparation and boundary checks

The controlled function first validates the iteration/control pair. It then calls `preparer.prepare_program(training.program(), profile)`. Once a bundle exists, `reject_loop_external_transfers` scans only `RunPhase::Loop` tasks and returns `LoopExternalTransfer` if either endpoint is `External`. External admission belongs to init and external output egress belongs to exit; no loop task may read or write host data.

Before handing off the native session, it derives `PlannedTrainingExecutionEvidence`, packs init images, and maps planned exit tasks with `map_external_output_tasks`. It discovers user metric slots with `user_metric_slots`, sorts and deduplicates `(MetricSlotId, MetricId)` pairs, and initializes each `FinalTrainingMetric` with no sample.

`map_external_output_tasks` checks the complete output boundary against both compiled graph tensors and the finalized bundle. Every planned output must be an external graph tensor, each task and logical tensor must be unique, each task must be a finalized `RunPhase::Exit` transfer to `External`, and its resolved source device/value must match the planner's `(device, physical)` identity. The set of planned tasks must equal all finalized external exit tasks, and the seen logical values must equal all graph external outputs. Any mismatch is `ExternalOutputMapping`.

### Native handoff and init

The prepared system is consumed with `into_parts`. The realized artifacts are retained in `RealizedNativeKernelSet`; the warmed local session is consumed into `LocalBackend`. A handoff failure is wrapped as `TrainingExecutionError::NativeHandoff` and no executor run is started.

`PreparedRun::prepare(run, bundle, backend, limits.watchdog)` computes fixed journal capacities, verifies loop-repetition support for the finalized loop count, realizes the immutable init, loop, and exit task slots, creates the metric mailbox, and records `LogicalEvent::Prepared`. `initialize(images)` validates the exact image set, allocates one arena per finalized layout, executes init admissions, records arena allocations and `LogicalEvent::Initialized`, and returns `InitializedRun`. This is the only training data ingress point.

### Loop, metrics, stop, and bounded waiting

`start_loop()` chooses iteration zero, marks every loop slot `Remaining`, resets the phase counters, and records `LoopStarted` and `LoopIterationStarted`. It returns `RunningRun`, whose task slots, dependencies, schedule windows, backend pending tokens, metric slots, and journal storage were all fixed before init.

Each call to `poll_with_progress_or_stop` performs one bounded scheduler/backend pass. It may submit work or complete pending work, but it cannot allocate a new task slot or native resource. The executor reports `(LoopStatus, made_progress)`:

1. If the active iteration is incomplete, the wrapper drains the newest value from each user metric slot with `running.try_take_metric`, offers it to the optional observer, retains it when its sequence is newer, and either polls again immediately or waits with bounded exponential backoff. Backoff starts at 50 microseconds, doubles after a non-progressing pass, and saturates at 2 milliseconds; any progress resets it.
2. When all active tasks are complete, the executor records `LoopIterationCompleted` and evaluates the stop closure. A true request records `LoopStopAccepted`, suppresses the next iteration, records `LoopCompleted`, and returns `Complete`. A false request starts the next configured iteration after resetting loop task completion. At a finite endpoint no next iteration exists, so the loop completes normally; a simultaneous true request is still recorded as accepted after that completed iteration.
3. Training drains metrics again after `into_exited_loop` succeeds. The final mailbox drain is needed because the last metric readback can become visible at the terminal poll boundary.

`RunningRun::into_exited_loop` succeeds only when the loop phase is complete and no failure is recorded. Calling it early returns the still-running state, which the training wrapper converts to `LoopDidNotReachTerminalState`.

The loop has no file access, host data admission, output egress, or native realization path. All calculation, transfer, optimizer, recurrent-state, and metric work is represented by the finalized task graph and executor state.

### Exit and returned state

`ExitedLoop::exit()` runs finalized exit transfers, collects `ExitImage` values, releases every arena, destroys the native backend resources, and records `LogicalEvent::Exited`. It returns `ExitedRun` only after all of those operations succeed. The wrapper takes backend, metric mailbox, external images, and journal from `ExitedRun::into_parts`, clones native evidence, sorts images by task, maps each image to its logical output value, drains the mailbox one final time, and completes `TrainingExecutionEvidence` from the journal.

The result is `CompletedTrainingExecution`. The completed value contains no live native session. `external_outputs` and `external_output_values` are index-aligned, metrics retain the newest executor sequence per planned slot, and all resource and lifecycle evidence describes the exact run that produced those values.

## Metric ownership and retention details

The executor creates a `MetricMailbox` with one capacity-one slot for each finalized user metric. A publish validates the slot and metric identity, assigns a globally increasing sequence, and replaces only the unread sample in that slot. This is newest-value-wins state, not an unbounded stream.

`drain_user_metrics` iterates the statically planned `FinalTrainingMetric` slice. It takes at most one sample per slot from the current executor state, converts the sample to a one-based epoch, optionally calls `TrainingMetricObserver::try_observe`, and updates final retention only when the incoming sequence is newer. It is called after every training poll, after loop completion, and after the final exited mailbox is recovered. Observer delivery can be dropped without changing final retention.

Metric tasks are loop-phase four-byte readbacks represented by `TaskKind::Metric`; they are not a third model work category. Fault readbacks remain executor-owned fault handling and do not enter the user metric slice. The training wrapper only exposes `MetricPurpose::User` slots.

## Inference execution paths in this module

### Closed input packing

`build_inference_device_images` and `build_knn_inference_device_images` validate their compiled boundary before calling `pack_inference_device_images`. Inference input packing is stricter than training admission: every declared input must appear in at least one manifest, and every manifest member must name a declared input. It rejects duplicate values or devices, undeclared members, unbound declared inputs, dtype and size mismatches, unsupported image sizes, out-of-bounds members, and overlapping ranges. It then returns sorted device images.

`validate_compiled_inference_boundary` and its KNN counterpart require exactly `LoopIterations::ONE`, no program metrics, a valid graph, one occurrence of each allowed inference input role, exact declared-to-graph external input equality, canonical contiguous row-major boundary tensors, and F32 prediction outputs. Dense inference also checks the task-specific prediction kind and matrix width. KNN validates its complete output contracts. These checks happen before preparation and again at the finalized output mapping boundary.

### Dense and KNN lifecycle

`prepare_and_execute_local_inference` and `prepare_and_execute_local_knn_inference` both:

1. validate the compiled boundary and prepare the program;
2. require the finalized bundle to retain one loop iteration;
3. reject loop external transfers and finalized user metric tasks;
4. pack admission images and map finalized external exit tasks to typed output contracts;
5. consume the warmed local session and hand it to `LocalBackend`;
6. use `PreparedRun::prepare_recoverable`, `initialize_recoverable`, and `start_loop_recoverable`;
7. poll with bounded backoff until `LoopStatus::Complete`;
8. require `into_exited_loop`, call `exit_recoverable`, and obtain post-teardown images and journal;
9. validate images against the finalized physical source, dtype, byte count, task set, and overlap rules;
10. return a completed prediction record or a typed failure retaining run and cleanup evidence.

Dense inference reports one `InferencePrediction`; KNN reports predictions in declared output order. Inference measures `elapsed` around only the checked loop polling interval. Admission, preparation, compilation, resource realization, warm execution, output validation, and teardown are outside that interval.

Prediction collection never reconstructs a value from partial device state. It copies bytes from finalized external exit images only after `ExitedRun` has completed. Missing, duplicate, unexpected, overlapping, mismatched, or physically displaced images return a post-exit validation error.

## Error surface and propagation

### Training errors

`TrainingExecutionError` is `#[non_exhaustive]` and has these current variants:

| Variant | Condition |
| --- | --- |
| `Preparation(PrepareError)` | profile, planning, candidate realization, finalization, or prepared-session failure |
| `NativeHandoff(Box<dyn Error + Send + Sync>)` | warmed local session cannot become the finalized backend |
| `Executor(ExecutorError)` | `PreparedRun`, init, polling, exit, journal, backend, or teardown failure |
| `DuplicateExternalInput` | the compiled training input list repeats a logical value |
| `DuplicateInitDevice` | finalized init manifests repeat a device |
| `DuplicateImageMember` | one manifest repeats a logical input |
| `MissingExternalInput` | a manifest member has no supplied training input |
| `ImageMemberDTypeMismatch` | input dtype differs from its manifest member |
| `ImageMemberSizeMismatch` | input bytes differ from the finalized member size |
| `ImageSizeUnsupported` | a complete image byte count does not fit host `usize` |
| `ImageMemberOutOfBounds` | checked member range exceeds the image or host slice |
| `ImageMembersOverlap` | sorted member ranges overlap |
| `LoopExternalTransfer` | a loop transfer names `External` as source or destination |
| `ExternalOutputMapping` | planner output identities do not equal the finalized exit boundary |
| `InvalidTrainingBounds` | the compiled training bound is invalid for evidence derivation |
| `UnboundedTrainingRequiresStopControl` | unbounded training has no explicit stop source |
| `LoopDidNotReachTerminalState` | the wrapper could not legally enter `ExitedLoop` |

`Preparation`, `NativeHandoff`, and `Executor` preserve nested error sources. Structural boundary variants have no source because the failure is already the precise diagnostic. `From<PrepareError>` and `From<ExecutorError>` preserve the category rather than flattening it. The root `execute_current_training_native` wraps the result as a `TrainingError::Runtime` stage only at the outer facade boundary.

The training path uses the ordinary executor methods. On any executor failure, the executor drops phase state, releases every arena it can, destroys resources, records the primary error, and retains a first cleanup error separately. No retry or alternate backend is selected. A failed run does not return `CompletedTrainingExecution`.

### Inference errors

`InferenceExecutionError` has equivalent preparation and native-handoff categories plus recoverable executor and post-exit wrappers:

- `Executor` carries the primary `ExecutorError` and an `InferenceRunFailure` with run, bundle, optional journal, and optional cleanup error;
- `PostExitValidation` carries a completed journal and cleanup-complete failure when output images fail validation after teardown;
- `InvalidLoopIterations`, `InvalidInferenceBoundary`, and input-image variants reject a closed boundary before or during admission;
- prediction variants reject missing, duplicate, unexpected, overlapping, dtype-mismatched, size-mismatched, or physically mismapped outputs;
- `LoopDidNotReachTerminalState` is the lifecycle invariant for a bounded wait that cannot enter `ExitedLoop`.

`run_failure()` exposes the retained failure only for `Executor` and `PostExitValidation`. `inference_executor_failure` consumes `RunFailure`, moves its evidence into `InferenceRunFailure`, drops the recovered backend, and leaves the original executor error as the source. There is no successful completed record on any failure path.

## Invariants enforced at this boundary

The implementation and its callees enforce the following concrete invariants:

1. Preparation is complete before `PreparedRun` is constructed. Native compilation, image loading, queue/completion creation, arena planning, and warm passes are pre-loop work.
2. The finalized bundle and measured profile are immutable for the run. Handoff validates topology, discovery, target, toolchain, artifact, reservation, task, and bridge identities.
3. One validated `DeviceImage` is admitted per finalized init manifest and device. Training permits a logical input to be replicated across devices, but not duplicate members in one image.
4. External data enters only in `init` and leaves only through finalized `exit` transfers. Loop external transfers are rejected before backend handoff.
5. Every loop task has a fixed slot, dependency set, schedule window, iteration domain, and pre-realized pending token. Polling cannot allocate, realize, or mutate the graph.
6. A stop request is observed only after all work in the active iteration is terminal. The completed epoch's calculation and optimizer state are preserved.
7. User metric publication is bounded and newest-value-wins. Live observation cannot backpressure polling, and dropped live notifications do not erase final samples.
8. Output images are mapped by finalized task and physical source identity, then paired with logical values by an explicit map. The host does not reverse-match arbitrary resident copies.
9. `CompletedTrainingExecution`, `CompletedInferenceExecution`, and `CompletedKnnInferenceExecution` are published only after exit transfers, arena release, resource destruction, and the `Exited` journal event.
10. Journal allocation is bounded from the finalized task graph. Repeated loop detail may be compacted into counters, but stop, lifecycle, and failure evidence remain ordered and inspectable.

## End-to-end training role and callers

The complete dense role is intentionally split across layers:

| Layer | Responsibility | Handoff into this module |
| --- | --- | --- |
| `src/api.rs` | record and validate fluent declarations | immutable `Data`, `Model`, `Train` |
| `src/data_prepare.rs` | bounded ingest, typed rows, split, schemas | prepared data for compilation |
| `src/training.rs` compile path | map model and policy, build graph/program, load semantic resume | `CompiledTraining` and optional native resume |
| `src/training.rs` native scope | measured bindings, host tuning, driver, bridge, artifact provider, `Preparer` | production `Preparer` and `MeasuredProfile` |
| `training/src/execute.rs` | prepare/finalize, handoff, init, loop, metrics, exit, evidence | `CompletedTrainingExecution` |
| `src/training.rs` report | construct checkpoint wrapper, derive graceful-stop state | `TrainingReport::Dense` |
| `training/src/checkpoint.rs` | serialize semantic model and exact native image | optional `.ogdl`, `.cubin`, or `.hsaco` |

The root caller installs the SIGINT guard only after compilation and resume preparation have succeeded. It starts the optional live-metric presenter, passes its observer and stop flag to this module, joins the presenter after execution, and then reports static device metrics. Artifact writes happen after this module returns successfully. Therefore a missing `.save(...)` produces no user artifact, while a declared save is independent of whether a semantic resume file existed.

The execution boundary is not a second training implementation. It does not know public layer types, data rows, optimizer declarations, artifact paths, or presentation policy. It consumes the compiled graph and finalized identities, executes the one real native lifecycle, and publishes authoritative post-exit state for the caller to wrap or save.

## Source map

The paired implementation regions are:

```text
training/src/execute.rs:75-129
  TrainingExecutionLimits, TrainingExecutionControl, unbounded-stop validation

training/src/execute.rs:142-255
  bounded observer, metric samples, final metric slots

training/src/execute.rs:257-368
  NativeKernelFormat, realized native image retention

training/src/execute.rs:370-501
  CompletedTrainingExecution and TrainingExecutionEvidence

training/src/execute.rs:687-867
  TrainingExecutionError and conversions

training/src/execute.rs:1184-1427
  public inference image builders and recoverable dense/KNN lifecycles

training/src/execute.rs:1444-2076
  closed inference boundary validation and strict image packing

training/src/execute.rs:2078-2294
  public training image builder and controlled lifecycle

training/src/execute.rs:2296-2512
  planned evidence and finalized training output mapping

training/src/execute.rs:2938-3192
  metric draining, transfer rejection, user-slot discovery, training image packing
```

The direct callees that define the state transition are `prepare::Preparer`,
`prepare::PreparedNativeSession`, `native_executor::LocalPreparedSession`,
`executor::PreparedRun`, `executor::RunningRun`, `executor::ExitedLoop`, and
`executor::RunJournal`. Their implementations remain the source of truth for
resource ownership, scheduling, backend submission, polling, cleanup, and
bounded journal behavior.
