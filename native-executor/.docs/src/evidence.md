# `recipe_native_executor::evidence`

```yaml
document: recipe_native_executor.evidence
source: native-executor/src/evidence.rs
kind: bounded-native-lifecycle-evidence
authority:
  - native-executor/src/evidence.rs
  - native-executor/src/local.rs
  - native-executor/src/cuda.rs
  - native-executor/src/hsa.rs
  - native-executor/src/error.rs
  - executor/src/backend.rs
  - executor/src/executor.rs
  - training/src/execute.rs
  - training/src/checkpoint.rs
  - src/training.rs
  - src/inference.rs
  - acceptance/src/main.rs
```

This page specifies the bounded native realization and teardown record exposed
by `recipe-native-executor`. It documents the current implementation contract,
not a second scheduler or a new runtime counter. The finalized bundle remains
the authority for devices, tasks, artifacts, queue slots, completion slots,
and arena layouts. The evidence record reports the resources that the CUDA and
ROCr/HSA child backends retained for that finalized partition immediately before
they are destroyed.

The record is deliberately small. It does not retain a journal of every Driver
or ROCr call, a list of arena allocations, host resources, bridge resources, or
loop iterations. The executor journal and the training execution evidence own
those other facts. Native evidence answers a narrower question: which native
GPU resources were realized for this run, and did the final native teardown
complete under the strict pre-realization contract?

Line references below identify the current checkout. Names and field formulas
are the parseable contract; line numbers are source anchors rather than a
versioned wire format.

## 1. Public data model

### 1.1 Backend family

`NativeBackendKind` is the closed native driver-family tag:

```text
NativeBackendKind = Cuda | Hsa
```

It derives `Clone`, `Copy`, `Debug`, `PartialEq`, and `Eq`
(`native-executor/src/evidence.rs:3-8`). `Cuda` means the record was produced
from `CudaResources`; `Hsa` means it was produced from `HsaResources`. The tag
does not mean that a device was discovered by ordinal or selected by this
record. Device identity and backend assignment are already fixed by the
finalized local partition.

### 1.2 Per-device record

`NativeDeviceExecutionEvidence` is a public, cloneable, equality-comparable
record (`native-executor/src/evidence.rs:10-21`):

```text
NativeDeviceExecutionEvidence {
    device: DeviceId,
    backend: NativeBackendKind,
    image_loads: usize,
    entry_lookups: usize,
    queues: usize,
    completion_objects: usize,
    persistent_allocations: usize,
}
```

The fields have exactly these meanings in the current producers:

| field | type | producer expression | meaning and limits |
| --- | --- | --- | --- |
| `device` | `DeviceId` | child resource map key | The finalized local device represented by this child resource set. |
| `backend` | `NativeBackendKind` | `Cuda` or `Hsa` literal | Which child adapter supplied the counts. |
| `image_loads` | `usize` | CUDA: `resources.modules.len()`; HSA: `resources.executables.len()` | Number of distinct native code images loaded for this device. Logical entries sharing one content digest count as one image. |
| `entry_lookups` | `usize` | `resources.artifacts.len()` in both adapters | Number of logical finalized runtime artifacts whose entry point was resolved in a loaded image. This can exceed `image_loads` when several entries share one image. |
| `queues` | `usize` | `resources.queues.len()` in both adapters | Number of finalized queue slots materialized for the selected task partition on this device. |
| `completion_objects` | `usize` | `resources.completions.len()` in both adapters | Number of finalized completion slots represented in the child resource map. CUDA stores `Event` values in these slots. HSA stores slot ownership state while native prepared tokens carry ROCr completion state. The evidence uses the map cardinality exactly. |
| `persistent_allocations` | `usize` | CUDA: `metric_buffers.len() + usize::from(scratch.is_some()) + 1`; HSA: `kernargs.len() + metric_buffers.len() + usize::from(scratch.is_some()) + 1` | Count of the child adapter's pre-realized persistent backing allocations represented by the formula. The constant `1` is the device's pre-realized staging allocation. Executor-owned arenas, host `Vec` egress images, bridge staging, queues, completion slots, modules, and executables are not included in this field. |

All count fields are map lengths or additions of `usize` values. The formulas
do not query the driver at evidence-read time, and they do not count loop
submissions. A zero count is therefore a valid structural value for an empty
selected partition, but a production run whose finalized partition requires a
resource must fail during realization rather than silently report a missing
resource.

### 1.3 Run record

`NativeExecutionEvidence` is the completed-run aggregate
(`native-executor/src/evidence.rs:23-35`):

```text
NativeExecutionEvidence {
    devices: Vec<NativeDeviceExecutionEvidence>,
    loop_realization_calls: u64,
    teardown_completed: bool,
    live_resources_after_teardown: usize,
}
```

The fields are private. Callers use these accessors
(`native-executor/src/evidence.rs:47-58`):

```text
devices() -> &[NativeDeviceExecutionEvidence]
loop_realization_calls() -> u64
teardown_completed() -> bool
live_resources_after_teardown() -> usize
```

`devices()` borrows the retained vector. It does not sort, deduplicate, or
re-query anything. The vector is formed by concatenating the CUDA child map's
ascending `BTreeMap` iteration with the HSA child map's ascending `BTreeMap`
iteration (`local.rs:2070-2071`, `cuda.rs:586-603`, `hsa.rs:608-626`). Each local
device has one owner class, so a valid `LocalBackend` cannot contribute the
same `DeviceId` from both children. The combined vector is grouped by backend,
not promised to be globally sorted.

## 2. Constructors and representation states

### 2.1 Default is not completion evidence

`NativeExecutionEvidence` derives `Default` (`evidence.rs:29`). The derived
value is equivalent to:

```text
devices = []
loop_realization_calls = 0
teardown_completed = false
live_resources_after_teardown = 0
```

`LocalBackend::new` and `LocalPreparedSession::into_backend` initialize their
`native_evidence` field with this default (`local.rs:1630-1638`,
`local.rs:1301-1314`). The zero live-resource value in a default record is not
proof that a run was torn down. It only means that no completed evidence has
been installed. Consumers must require `teardown_completed() == true` before
interpreting any count, and a successful public report always carries the
completed value rather than this placeholder.

### 2.2 The only completed constructor

`NativeExecutionEvidence::completed(devices)` is crate-private
(`evidence.rs:37-45`). It copies the supplied per-device vector and sets:

```text
loop_realization_calls = 0
teardown_completed = true
live_resources_after_teardown = 0
```

There is no setter, incrementer, deserializer, or alternate public constructor.
The repository-wide search has no other write to any of these fields. Thus the
current `loop_realization_calls` value is a contract assertion, not an
instrumented Driver-call counter: a successful record can only say zero because
the running API admits no compiler, loader, queue creator, completion creator,
or allocator operation after handoff. Likewise, `live_resources_after_teardown`
is the explicit postcondition used by the constructor, not an independent live
handle enumeration.

This distinction is important for interpretation:

```text
completed record
  => native child destroy operations returned Ok
  => teardown_completed == true
  => live_resources_after_teardown == 0
  => no admitted loop realization path (asserted as 0)

default record
  => no completed report was produced
  => teardown_completed == false
  => other zero values are placeholders, not observations
```

## 3. Production lifecycle and publication point

The evidence publication point is the final executor teardown boundary. The
full path is:

```text
measured candidate
  -> LocalCandidateFactory::realize_candidate
  -> CUDA/HSA candidate resources (modules, entries, queues, completion state,
     staging, persistent buffers, pending tokens)
  -> NativeLocalStabilizer warm pass and capacity observation
  -> LocalPreparedSession::into_backend(finalized bundle)
  -> LocalBackend with native_evidence = default
  -> PreparedRun::prepare -> initialize -> start_loop -> exit
  -> executor releases every arena
  -> LocalBackend::destroy_resources
       snapshot CUDA/HSA per-device counts
       destroy bridge
       destroy HSA resources
       destroy CUDA resources
       destroy host resources
       install NativeExecutionEvidence::completed(snapshot)
  -> ExitedRun backend
  -> training or inference clones backend.native_evidence()
```

### 3.1 Pre-loop realization

`LocalCandidateFactory::realize_candidate` validates the candidate, partitions
tasks and devices, and realizes the child resources before Finalize
(`local.rs:1439-1547`). `CudaPreparedResources::realize` and
`HsaPreparedResources::realize` load code images, resolve logical entries,
create queue/completion storage, allocate staging and persistent buffers, and
prepare native pending tokens. The production factory uses
`NativeLocalStabilizer`, which executes a maximum-concurrency warm pass and
then observes capacity (`local.rs:970-994`, `local.rs:1549-1604`).

Warm execution is still preparation. `execute_warm_pass` allocates temporary
candidate arenas and drives a provisional warm trace; capacity observation
releases those arenas before the finalized handoff (`local.rs:1076-1132`,
`local.rs:1223-1260`). None of these warm resources are included in the final
`NativeExecutionEvidence`; the final record is created only for the resources
that survive into `LocalBackend`.

`LocalPreparedSession::into_backend` requires an observed warm pass, unchanged
candidate and finalized identities, empty warm arenas, and successful child
handoff validation (`local.rs:1262-1374`). It moves the warmed child objects
into `LocalBackend` and explicitly starts the evidence field at `default`
(`local.rs:1289-1314`). This is the point after which the executor owns the
immutable `init -> loop -> exit` run, but before any successful evidence exists.

### 3.2 Executor teardown ordering

`ExitedLoop::exit_recoverable` first drives every finalized Exit task to
completion. It then drops the exit phase, calls `teardown_resources`, and only
returns `ExitedRun` if teardown and the final `Exited` journal event succeed
(`executor/src/executor.rs:1341-1371`). `teardown_resources` takes every
executor arena and invokes `Backend::release_arena` once per device before it
consumes the backend resource and invokes `Backend::destroy_resources`
(`executor/src/executor.rs:1489-1541`). Therefore the evidence snapshot does
not include executor-owned arenas. Arenas are released before child-resource
evidence is captured.

`LocalBackend::destroy_resources` records the physical destroy call, obtains
CUDA and HSA evidence while both child resource maps are still intact, then
destructures the composite resource (`local.rs:2064-2078`). It attempts child
destruction in this exact order:

```text
bridge.destroy(bridge_resource)
hsa.destroy()
cuda.destroy()
host.destroy_partition(host_resource)   # only when a host partition exists
```

`retain_first` keeps the first error while still attempting each later child
(`local.rs:2079-2102`, `local.rs:3649-3657`). The evidence constructor is called
only when all four applicable operations return `Ok(())`
(`local.rs:2103-2108`). A bridge or host error therefore prevents publication
even though CUDA and HSA counts were already snapshotted and their destroy
attempts were made.

### 3.3 Successful report boundary

On success, `teardown_resources` returns no error, the executor records
`LogicalEvent::Exited`, and `ExitedRun::into_parts` returns the backend,
mailbox, exit images, and journal (`executor/src/executor.rs:1352-1370`,
`executor/src/executor.rs:1421-1423`). The backend still owns the
`NativeExecutionEvidence` value, but no native resource remains live. Training
and inference clone the value immediately after taking the backend, then put it
into their completed execution records:

```text
training/src/execute.rs:2261-2293
  backend.native_evidence().clone()
  -> CompletedTrainingExecution.native_evidence

training/src/execute.rs:1284-1309
  backend.native_evidence().clone()
  -> CompletedInferenceExecution.native_evidence

training/src/execute.rs:1402-1426
  backend.native_evidence().clone()
  -> CompletedKnnInferenceExecution.native_evidence
```

The comment on each completed execution type states the same invariant:
native resources have been destroyed before the value is returned
(`training/src/execute.rs:370-383`, `:525-536`, `:605-614`).

## 4. Native producers

### 4.1 CUDA producer

`CudaResources::execution_evidence` iterates `self.devices` and emits one
record per map entry (`native-executor/src/cuda.rs:586-603`):

```text
NativeDeviceExecutionEvidence {
    device,
    backend: NativeBackendKind::Cuda,
    image_loads: resources.modules.len(),
    entry_lookups: resources.artifacts.len(),
    queues: resources.queues.len(),
    completion_objects: resources.completions.len(),
    persistent_allocations:
        resources.metric_buffers.len()
        + usize::from(resources.scratch.is_some())
        + 1,
}
```

The map contents are established by `realize_device`
(`cuda.rs:1665-1903`):

1. The binding must name a finalized device, use an exact matching CUDA
   context and deployment, and hold an enforced scheduler quota
   (`cuda.rs:1633-1663`).
2. Queue IDs and completion IDs are collected from the selected finalized
   tasks. Each selected queue slot creates one nonblocking CUDA `Stream`; each
   selected completion slot creates one completion `Event`
   (`cuda.rs:1677-1702`). The requested queue count is checked against the
   binding maximum before creation (`cuda.rs:392-399`, `:1620-1631`).
3. One pinned staging buffer is allocated from the finalized per-device
   staging byte count. The device's init-image contract must exist and fit that
   buffer (`cuda.rs:1703-1729`). This is the `+1` persistent allocation.
4. A nonzero finalized scratch entry creates one device buffer; absent or zero
   scratch creates none (`cuda.rs:1730-1743`). This contributes the optional
   `usize::from(scratch.is_some())` term.
5. Calculation artifact IDs are deduplicated by task. Runtime artifacts are
   checked against the finalized identity, deployment, cubin inspection, ABI,
   and content digest (`cuda.rs:1744-1815`).
6. Distinct cubin content digests each call `Module::load_cubin`, so one loaded
   module is retained per distinct image. Every logical artifact entry in the
   digest group then calls `module.function`, producing one `LoadedArtifact`
   per logical entry (`cuda.rs:1817-1848`). Consequently, shared-image entries
   make `entry_lookups > image_loads` while preserving one actual module load.
7. Invocation parameter blocks are allocated per required completion slot,
   metric readback buffers are allocated as one pinned four-byte buffer per
   selected metric task on the device, and exit transfers receive preallocated
   host `Vec<u8>` images (`cuda.rs:1850-1889`). Invocation blocks and egress
   vectors are intentionally not part of the persistent-allocation formula.

CUDA evidence is therefore a realization cardinality, not a count of every
physical Driver call. The actual loop only uses the retained streams, events,
functions, modules, invocation storage, arenas, and buffers. `submit` dispatches
among `InitAdmission`, `Calculation`, `InternalTransfer`, `Metric`, and
`ExitTransfer` without any code-loading or resource-creation branch
(`cuda.rs:480-510`).

### 4.2 HSA producer

`HsaResources::execution_evidence` has the same record shape with the ROCr/HSA
tag (`native-executor/src/hsa.rs:608-626`):

```text
NativeDeviceExecutionEvidence {
    device,
    backend: NativeBackendKind::Hsa,
    image_loads: resources.executables.len(),
    entry_lookups: resources.artifacts.len(),
    queues: resources.queues.len(),
    completion_objects: resources.completions.len(),
    persistent_allocations:
        resources.kernargs.len()
        + resources.metric_buffers.len()
        + usize::from(resources.scratch.is_some())
        + 1,
}
```

`realize_device` builds the retained map (`hsa.rs:1808-2049`):

1. The binding must have an enforced reservation, a CPU kernarg allocator,
   and an allocatable kernarg pool. Selected queue slots create ROCr queues;
   selected completion slots populate `CompletionState::Available`
   (`hsa.rs:1692-1755`, `:1817-1848`). Queue capacity is checked before queue
   creation (`hsa.rs:428-435`).
2. One fine-grained host staging allocation is created from the selected
   device's host allocator. The finalized init-image contract must exist and
   fit it (`hsa.rs:1849-1872`). This is the `+1` persistent allocation.
3. A nonzero finalized scratch entry creates one coarse allocation; absent or
   zero scratch creates none (`hsa.rs:1873-1885`).
4. Calculation artifacts are grouped by content digest after target and code
   object validation (`hsa.rs:1886-1935`). One `Executable` is loaded per
   distinct image. Every logical artifact in that image is inspected and
   resolved with `executable.kernel`, producing one `LoadedArtifact` per entry
   (`hsa.rs:1937-1987`). Thus shared HSACO entries likewise make
   `entry_lookups > image_loads` possible.
5. Kernarg slots are allocated for the maximum ABI size required by each
   completion slot. Metric tasks receive one four-byte fine-grained host
   allocation each, and exit transfers receive preallocated host vectors
   (`hsa.rs:1989-2034`). Kernarg slots and metric buffers contribute to the
   persistent-allocation formula; host vectors do not.

The HSA pending pool is prepared once per candidate task before handoff
(`hsa.rs:1757-1775`). Each pool token owns the native asynchronous state needed
by that task. `completion_objects` still uses the finalized completion-state map
length, not a fresh signal count, so callers must not reinterpret it as the
number of AQL packets or all internal signal allocations.

HSA `submit` dispatches only over the closed backend work variants and reuses
the prepared pending token (`hsa.rs:541-580`). It never loads an executable,
resolves a symbol, creates a queue, or allocates a buffer after handoff.

### 4.3 What is not a producer

The host child backend and the cross-backend bridge do not emit
`NativeDeviceExecutionEvidence`. `LocalBackend` snapshots only the vectors
returned by `cuda.execution_evidence()` and `hsa.execution_evidence()`
(`local.rs:2064-2072`). Bridge staging, host worker threads, host arenas, and
host allocations are destroyed and validated separately. Executor arena
allocation and release are also outside these records.

## 5. Validation that makes the record meaningful

Evidence has no standalone `validate` method. Its validity is established by
the resource and lifecycle checks that must succeed before
`NativeExecutionEvidence::completed` can be called.

### 5.1 Immutable plan and artifact checks

`ExecutionPlan::validate` and `validate_partition` derive the device set,
selected artifact set, and one immutable `PlannedSubmission` per finalized
task (`native-executor/src/plan.rs:122-225`). They reject duplicate, missing,
unexpected, or incompatible runtime artifacts, and require every queue and
completion slot to belong to the planned task device
(`plan.rs:227-352`, `:642-714`). This is the source of truth for which maps the
child realization may create.

Both adapters then build per-task contracts from the finalized bundle. The
contract records phase, work class, submission slots, init-image identity,
transfer route, and lane claims (`cuda.rs:1485-1575`, `hsa.rs:2118-2214`). A
calculation or metric outside `Loop`, an invalid transfer endpoint/phase, a
missing finalized admission image, or a repeated task identity is rejected
before submission.

### 5.2 Handoff checks

The warmed handoff must be exact. `LocalPreparedSession::validate_handoff`
requires an observed final warm pass, unchanged topology/discovery/draft/
candidate/task/kernel/resource/init-image/reservation identities, empty warm
arenas, and successful bridge, host, CUDA, and HSA validation
(`local.rs:1325-1374`, `:3430-3479`). The CUDA and HSA child handoff checks
also require that the warm admission manifest equals the finalized init image,
that all warm pending tokens were recycled, and that finalized artifact plans
match the warmed resources (`cuda.rs:960-980`, `hsa.rs:1114-1139`).

This is why a later module load or queue creation would be a protocol violation:
the final resource maps and their task contracts were already validated and
retained before the executor accepted the bundle.

### 5.3 Pending, submission, and poll checks

For every task, `prepare_pending` checks the request against its immutable
contract, verifies the planned device, queue, and completion slot, and rejects
duplicate preparation (`cuda.rs:438-478`, `hsa.rs:487-539`). The local composite
selects exactly one owner for each task and rejects owner mismatch
(`local.rs:1703-1750`, `:3626-3640`).

Submission validates the pending token and work contract. CUDA marks the child
poisoned on any submission error (`cuda.rs:480-510`); HSA poisons only errors
identified as session, deferred-retirement, or failed asynchronous-signal
conditions (`hsa.rs:541-580`, `:2486-2493`). Poll validates active completion
ownership and marks native failures as poisoned (`cuda.rs:554-577`,
`hsa.rs:582-600`). Loop repetition can only rearm a terminal loop token, never
an active token (`cuda.rs:898-943`, `hsa.rs:1053-1104`).

These checks enforce the absence of loop realization operations. A loop task
can only use its pre-realized queue, completion state, artifact entry, staging,
metric buffer, kernarg, and arena handles. The `loop_realization_calls == 0`
field records this closed surface after teardown.

### 5.4 Teardown checks

CUDA teardown polls every retained stream idle, rejects an active completion
event, destroys available events and streams, drops logical function holders
before unloading modules, and frees metric buffers, staging, and scratch
(`cuda.rs:2203-2238`). `CudaResources::destroy` first requires the child to be
healthy (`cuda.rs:644-647`).

HSA teardown requires every completion state to be `Available`, drains session
retirements, closes queues, drops logical kernel holders before executables,
then closes kernarg slots, metric buffers, staging, and scratch
(`hsa.rs:699-711`, `hsa.rs:2571-2606`). `HsaResources::destroy` also rejects a
poisoned child before destruction (`hsa.rs:699-705`).

Only when all applicable child teardown operations return `Ok(())` does the
composite install `completed(...)`. Therefore `teardown_completed == true` is
equivalent to the successful native teardown boundary, while a nonzero count
or a child cleanup error never produces a partially successful public evidence
record.

## 6. Failure behavior and observability

### 6.1 Before final backend handoff

Candidate realization, warm execution, capacity observation, and finalized
handoff can fail with `LocalError` or child `Error` before a `LocalBackend` is
returned. Cleanup attempts the bridge, HSA, CUDA, and host resources in the
same ordered style, but there is no `NativeExecutionEvidence` value to return.
`LocalPreparedSession::into_backend` destroys the prepared session when handoff
validation fails and returns the validation or teardown error
(`local.rs:1262-1321`).

Relevant native failure classes are enumerated by
`native-executor::Error` (`native-executor/src/error.rs:7-97`): duplicate,
missing, unexpected, or mismatched artifacts/devices; missing queues or
completion slots; resource contention and busy completions; arena/value
mismatches; unsupported transfers or loop contracts; invalid backend state;
poisoned backend; queue-limit overflow; integer and physical-accounting
overflow; Driver/ROCr/kernel failures; and task protocol violations. These are
fail-closed validation failures, not evidence values.

### 6.2 Runtime or exit failure after handoff

The executor's recoverable failure path stores the backend and optional journal
in `RunFailure`, attempts every arena release and resource destroy in lifecycle
order, and records the first teardown error separately as `cleanup_error`
(`executor/src/executor.rs:693-758`, `:1489-1553`). If a native child is
poisoned, its `destroy` method may return `BackendPoisoned`; the composite still
attempts later children, but `completed(...)` is not installed.

Inference maps a recoverable `RunFailure` to `InferenceRunFailure`, retaining
run/bundle identity, optional journal, and cleanup status while dropping the
recovered backend (`training/src/execute.rs:2962-3002`). Training returns the
executor error through `TrainingExecutionError::Executor` and likewise does not
publish native evidence on a failed execution. `InferenceRunFailure::cleanup_completed`
only reports whether the executor recorded a cleanup error; it is not a
replacement for `NativeExecutionEvidence::teardown_completed`.

If exit output validation fails after native teardown, inference retains the
journal and a cleanup-complete failure wrapper, but still returns an error
instead of a completed execution record (`training/src/execute.rs:1284-1309`,
`:1402-1426`, `:2987-3002`). The native evidence was cloned before that
post-exit validation, so it is not exposed through the failed error value.

### 6.3 Partial cleanup and default-value traps

`LocalBackend::destroy_resources` deliberately attempts bridge, HSA, CUDA, and
host destruction even after an earlier child fails. `retain_first` discards
later errors after preserving the first. A run that reaches this partial-cleanup
path returns `RunFailure` with an error or cleanup error, not a completed native
record. In particular:

```text
teardown_completed() == false
  means no successful completed evidence was installed

live_resources_after_teardown() == 0
  by itself means nothing, because the derived default also contains zero
```

Do not infer that a failed run leaked no resources from the default zero. Use
the executor's cleanup error and journal for failure diagnosis; the evidence
record is intentionally available only on successful completed reports.

## 7. Public consumers

### 7.1 Crate and root re-exports

`native-executor/src/lib.rs:22,36` keeps the implementation module private
while re-exporting `NativeBackendKind`, `NativeDeviceExecutionEvidence`, and
`NativeExecutionEvidence`. The root crate re-exports the same three names
(`src/lib.rs:22`), so users do not import the private module path.

### 7.2 Training reports

`CompletedTrainingExecution` stores a required `NativeExecutionEvidence` and
exposes `native_evidence()` (`training/src/execute.rs:370-383`, `:441-474`).
`CompletedTrainingCheckpoint` forwards the same reference
(`training/src/checkpoint.rs:8700-8706`). `TrainingReport::native_evidence()`
returns `Some` only for a completed dense native run and `None` for KNN or
Bayesian reference preparation (`src/training.rs:320-328`). The report is
constructed only after the executor's successful exit and teardown
(`src/training.rs:888-916`).

### 7.3 Inference reports

Dense, Bayesian, GGUF, and KNN completed execution types each retain the
native record. `InferenceReport::native_evidence()` forwards it for all four
payload variants (`src/inference.rs:277-285`). Dense and KNN local execution
both clone it from the backend only after `ExitedRun::into_parts` succeeds
(`training/src/execute.rs:1284-1309`, `:1402-1426`). The accessor is therefore
not evidence that the KNN path used an optimizer loop; it only reports the
native lifecycle of the completed KNN inference execution.

### 7.4 Acceptance validation

The CUDA acceptance runner is the current explicit validator
(`acceptance/src/main.rs:1019-1062`). It requires:

```text
loop_realization_calls == 0
teardown_completed == true
live_resources_after_teardown == 0
device record count == 1
devices[0].backend == NativeBackendKind::Cuda
devices[0].image_loads == 1
entry_lookups > 0
queues > 0
completion_objects > 0
persistent_allocations > 0
```

The same checks are applied to dense training, recurrent train/save/infer, and
the inference path (`acceptance/src/main.rs:430-447`, `:580-615`). The safe-stop
source also asserts one device, one image, zero loop realization calls, and
zero live resources after teardown (`acceptance/src/main.rs:842-858`). The
acceptance fields written to its single record are direct projections of these
accessors (`acceptance/src/main.rs:507-528`).

## 8. Machine-readable contract summary

The following restricted shape is a compact parser-oriented description of the
current API. It intentionally records formulas and state predicates rather than
inventing a serialization format:

```yaml
types:
  NativeBackendKind:
    variants: [Cuda, Hsa]
  NativeDeviceExecutionEvidence:
    fields:
      device: DeviceId
      backend: NativeBackendKind
      image_loads: usize
      entry_lookups: usize
      queues: usize
      completion_objects: usize
      persistent_allocations: usize
  NativeExecutionEvidence:
    fields:
      devices: "Vec<NativeDeviceExecutionEvidence>"
      loop_realization_calls: u64
      teardown_completed: bool
      live_resources_after_teardown: usize
constructors:
  default:
    devices: []
    loop_realization_calls: 0
    teardown_completed: false
    live_resources_after_teardown: 0
  completed:
    visibility: crate
    precondition: "bridge/HSA/CUDA/host destruction all return Ok"
    loop_realization_calls: 0
    teardown_completed: true
    live_resources_after_teardown: 0
producers:
  cuda:
    image_loads: "modules.len"
    entry_lookups: "artifacts.len"
    queues: "queues.len"
    completion_objects: "completions.len"
    persistent_allocations: "metric_buffers.len + scratch.is_some as usize + 1"
  hsa:
    image_loads: "executables.len"
    entry_lookups: "artifacts.len"
    queues: "queues.len"
    completion_objects: "completions.len"
    persistent_allocations: "kernargs.len + metric_buffers.len + scratch.is_some as usize + 1"
publication:
  owner: LocalBackend::destroy_resources
  order: [bridge, hsa, cuda, host]
  success: "install completed(snapshot)"
  failure: "return first error; leave evidence at default"
scope:
  included: [CUDA, HSA]
  excluded: [host, bridge, executor_arenas, loop_call_history]
```

The YAML block is descriptive only. `NativeExecutionEvidence` is not serialized
by `native-executor`, and no consumer may reconstruct it from a journal or from
the retained native kernel image.
