---
document: recipe_native_executor.hsa
source: native-executor/src/hsa.rs
kind: ROCr/HSA native backend realization, submission, polling, and lifecycle
authority:
  - native-executor/src/hsa.rs
  - native-executor/src/local.rs
  - native-executor/src/bridge.rs
  - native-executor/src/plan.rs
  - native-executor/src/error.rs
  - hsa/src/session.rs
  - hsa/src/execution.rs
  - hsa/src/error.rs
  - kernel/src/artifact.rs
  - kernel/src/llvm.rs
  - executor/src/backend.rs
  - native-executor/INTEGRATION_REQUIRED.md
---

# ROCr/HSA native executor

## Source anchors

The source ranges below are navigation anchors for the current checkout. The
state and type invariants in this document are the contract; the line numbers
are not an additional versioned interface.

| area | source anchors | responsibility |
| --- | --- | --- |
| Binding, arenas, and resource types | `native-executor/src/hsa.rs:L31-L281` | HSA binding, arena wrapper, loaded artifacts, kernarg slots, completion state, and device resources |
| Backend states and finalized realization | `native-executor/src/hsa.rs:L283-L711` | one-shot bind, device realization, arena allocation, pending preparation, submit, poll, evidence, collection, and destroy |
| Submission implementations | `native-executor/src/hsa.rs:L713-L1038` | admission, internal and exit copies, calculation dispatch, and metric readback |
| Terminal and loop lifecycle | `native-executor/src/hsa.rs:L1040-L1380` | completion actions, reset, recycling, warm handoff, and prepared-resource destroy |
| `Backend` boundary and pending token | `native-executor/src/hsa.rs:L1381-L1690` | physical accounting, child backend methods, pending state, and identity checks |
| Binding and per-device realization | `native-executor/src/hsa.rs:L1692-L2050` | pool and quota gates, queue creation, staging, HSACO loading, kernargs, metrics, and egress |
| Contracts and native helpers | `native-executor/src/hsa.rs:L2052-L2679` | ABI resource envelopes, task contracts, kernarg encoding, range checks, signals, and teardown |
| Local callers and warm trace | `native-executor/src/local.rs:L1507-L1528`, `L1647-L2108`, `L2844-L3235` | candidate realization, composite backend dispatch, warm submission, poll, collection, and recycle |
| Cross-backend HSA legs | `native-executor/src/bridge.rs:L172-L430`, `L1066-L1580` | HSA staging allocation, prepared copy tokens, poll, reset, and recycle |
| ROCr queue and session ownership | `hsa/src/session.rs:L135-L430` | GPU session, queue callback fault, single-producer queue creation, and close |
| ROCr execution ownership | `hsa/src/execution.rs:L34-L1068`, `L1080-L2165` | allocations, signals, executable loading, prepared pending, copies, AQL packets, and queue capacity |
| Final artifact inspection | `kernel/src/artifact.rs:L319-L560` | HSACO ELF, target, metadata, symbol, and ABI validation |


This document describes the implementation in `native-executor/src/hsa.rs` and
the HSA calls that it makes through the `recipe-hsa` crate. The HSA adapter is
one `recipe_executor::Backend` implementation. It owns already-realized ROCr
queues, completion signals, executable images, kernel arguments, staging, and
result storage. The executor supplies finalized work and finalized arena
addresses. The adapter does not discover hardware, compile kernels, or change
the schedule.

The implementation has two execution boundaries:

1. `HsaPreparedResources::realize` creates candidate resources before
   `Finalize` and retains a pending token for every selected task.
2. `HsaResources::realize` either creates the same resources while binding a
   finalized bundle directly, or receives the warmed candidate resources and
   attaches finalized plans and addresses without loading a second image.

After binding, `submit` and `poll` are nonblocking and use only the resources
prepared for the task. A submitted operation is represented by one
`recipe_hsa::PreparedPending` token and one executor completion slot. The token
is reset only after terminal completion. A queue or session fault poisons the
HSA resource set and prevents later operations.

## Ownership and state model

### `HsaBinding`

`HsaBinding<'scope>` (`hsa.rs:31-121`) is the immutable bridge from one
finalized `DeviceId` to one HSA session. It stores:

| field | meaning |
| --- | --- |
| `device` | Recipe device identity used by plans, arenas, and task contracts |
| `session` | the exact `recipe_hsa::Session` for the discovered GPU agent |
| `host_allocator` | the discovered CPU agent used for fine-grained host-visible and kernarg allocations |
| `target_id` | exact AMD ISA target string expected by every HSACO assigned to this device |
| `code_object_version` | expected AMDGPU code-object version |
| `queue_packets` | configured AQL queue ring size passed to `QueueConfig::new` |
| `maximum_submission_queues` | discovered queue-count ceiling checked before queue creation |
| `enabled_display_connectors` | reservation evidence carried into candidate selection |

`new` only stores the values. `available_bytes` asks the bound session for its
current ROCr memory counter. `allocate_host_fine` allocates through the exact
CPU agent's fine-grained pool and immediately calls `Session::grant_access` so
the GPU session can access it. The binding is valid only if `validate_binding`
(`hsa.rs:1692-1738`) later confirms all of the following:

- the allocator describes a CPU agent;
- it exposes a runtime-allocatable global kernarg pool with
  `KERNARG_INITIALIZATION`;
- it exposes a runtime-allocatable global fine-grained or extended-scope
  fine-grained pool;
- the session advertises the exact `target_id`.

The binding is not a fallback selector. A missing target, pool, or exact owner
is an `ArtifactMismatch` and realization stops.

### Backend state

`HsaBackend` (`hsa.rs:283-357`) has a one-way state machine:

```text
Ready { bindings, artifacts }
        | bind_resources or bind_partition
        v
Prepared(HsaPreparedResources) --bind--> Bound child resources
        | candidate warm handoff
        v
Warmed(HsaResources) ----------validate_handoff--> Bound child resources

any successful bind -> Bound (the backend object cannot bind again)
```

`HsaBackend::new` starts in `Ready`. `from_prepared` and `from_warmed` are
internal ownership handoff constructors used by `LocalPreparedSession`. Both
`bind_partition` and the `Backend::bind_resources` implementation replace the
state with `Bound` before attempting the bind. Therefore a second bind reports
`Error::BackendState { backend: "HSA", detail: "resources may be bound only once" }`,
including when the first bind failed.

`bind_partition` is the partitioned local path. In `Ready`, it derives the
partition device set, calls `ExecutionPlan::validate_partition`, and realizes
only the selected tasks. In `Prepared`, it calls `HsaPreparedResources::bind`.
In `Warmed`, it validates the finalized handoff and returns the warmed
resources. The non-partitioned `Backend::bind_resources` path validates the
whole bundle, or performs the equivalent prepared or warmed handoff.

### Resource containers

`HsaResources<'scope>` (`hsa.rs:253-260`) owns the finalized execution state:

| field | invariant |
| --- | --- |
| `plan` | immutable `ExecutionPlan` containing runtime artifact contracts and per-task device, queue, and completion assignments |
| `devices` | one `DeviceResources` for every finalized HSA device, and no unplanned device |
| `contracts` | one `HsaTaskContract` for every selected task, derived from the finalized task and transfer manifests |
| `prepared_tasks` | task IDs whose pending tokens have been checked out of `pending_pool` and are not yet recycled |
| `pending_pool` | one unused `PreparedPending` per selected task before preparation, and again after recycling |
| `poisoned` | permanent adapter-level poison bit set after a session-fatal asynchronous failure |

`DeviceResources` (`hsa.rs:215-251`) contains all per-device native objects:

```text
session, host_allocator
queues: QueueSlotId -> Queue<SingleProducer>
completions: CompletionSlotId -> Available | Active { task }
artifacts: ArtifactId -> LoadedArtifact { Kernel, KernelAbi, resource envelope }
executables: ArtifactDigest -> Executable
kernargs: CompletionSlotId -> KernargSlot { host allocation, host byte vector }
metric_buffers: TaskId -> four-byte fine-grained allocation
staging: fine-grained host allocation
admission: optional finalized InitImageContract
egress: exit TaskId -> host byte vector
scratch: optional coarse allocation
```

The `executables` map owns one loaded executable for each distinct HSACO image
digest. `artifacts` keeps one logical ABI entry per Recipe artifact and its
`Kernel` handle. The declaration order is intentional: artifact kernel handles
drop before their shared executable owner during unwinding.

`HsaArena` (`hsa.rs:135-159`) wraps one coarse allocation and remembers its
Recipe device. `release` closes the allocation explicitly. `HsaArenaLookup` is
implemented both for the finalized `BTreeMap<DeviceId, HsaArena>` and for the
executor's borrowed `ArenaSet`, so HSA submission can validate an arena without
copying or owning the whole arena map.

### Task contracts and pending state

`HsaTaskContract` (`hsa.rs:205-213`) freezes the fields that the adapter must
not accept from a later caller: `RunPhase`, `WorkClass`, optional
`SubmissionSlots`, optional `InitImageContract`, transfer route, and transfer
lane claims. `task_contracts` derives these values from the finalized bundle.
Calculations and metrics must be loop tasks. Init external-to-device transfers
are `InitAdmission`; device-to-device init or loop transfers are
`InternalTransfer`; exit device-to-device or device-to-external transfers are
`ExitTransfer`. Any other phase and endpoint combination is a protocol error.

The contract helpers are deliberately closed over the finalized task model:

- `work_submission` and `task_submission` extract the one `SubmissionSlots`
  value from every `BackendWork` or `TaskKind` variant. A missing plan
  submission is a protocol error at preparation or submit time.
- `transfer_work_class` accepts only init external-to-device admission, init or
  loop device-to-device internal copies, and exit device-to-device or
  device-to-external copies. External-to-external, loop external copies, and
  init or exit endpoint combinations outside that matrix fail with `Protocol`.
- `candidate_task_device` chooses a calculation's device, a metric's resolved
  value device, or the device endpoint of a transfer. An external-to-external
  candidate transfer has no HSA owner and is rejected.
- `requested_submission_queue_count` counts distinct queue slot IDs from the
  selected tasks and filters the resource manifest by device. This exact count
  is checked against `maximum_submission_queues` before queue creation.
- `validate_work_contract` requires class and submission equality. Admission
  additionally requires the finalized device, image value, and byte count;
  internal and exit transfers additionally require exact route and lane claims;
  calculations and metrics have no transfer fields to validate.

`device_mut` reports `MissingDevice` rather than constructing a device on
demand. `ensure_queue` and `validate_active` similarly require a realized queue
and an active completion owner. `release_completion` changes only the matching
`Active { task }` slot back to `Available`; it rejects a missing slot or a slot
owned by another task. `finish_submission_claim` invokes this release when a
ROCr submission fails, so a rejected packet never leaves a stale completion
claim. `require_enforced_quota` is the single reservation policy check, and
`free_optional_allocation` closes scratch only when it exists. `available_bytes`
is a read-only current session query and does not reserve or allocate memory.

`HsaPending` (`hsa.rs:1577-1689`) couples the executor identity to one native
prepared token:

```text
Ready --submit--> Active --poll complete--> Terminal
  ^                  |                         |
  |                  +--poll error------------+ poisoned/error
  +--loop reset/rearm--------------------------+
```

It stores task, phase, device, queue slot, completion slot, work class, the
`PreparedPending`, a `PendingAction`, and `HsaPendingState`. `PendingAction` is
`None`, `Metric { dtype }`, or `Egress { bytes }`; it tells terminal handling
which host-visible result to materialize. `validate_ready` requires the token
to be `Ready` and requires task, class, device, queue, and completion to match
the immutable plan. `validate_collected_exit` requires a terminal
`ExitTransfer` with the exact queue, completion, action, and destination byte
count.

## Candidate and finalized realization

### Pre-final candidate realization

`HsaPreparedResources::realize` (`hsa.rs:1159-1248`) is called by
`LocalCandidateFactory::realize_candidate` in `local.rs:1507-1528`. It receives
the candidate draft, candidate runtime artifacts, reservation ledger, HSA
bindings, and the selected HSA task IDs. It performs these checks and actions
in order:

1. Build a device-to-binding map. Duplicate binding IDs fail with
   `DuplicateDevice`.
2. Collect selected draft tasks. Every selected calculation device must have
   a binding. Missing devices fail closed.
3. Collect the exact calculation artifact IDs. Every supplied runtime artifact
   must be selected, every selected artifact must be supplied, and each ID may
   occur only once. Extra or missing artifacts fail with
   `UnexpectedArtifact`, `DuplicateArtifact`, or `MissingArtifact`.
4. Build the draft value-to-device map and validate every binding with
   `validate_binding`.
5. Call `realize_device` for each binding. This creates the queues, signals,
   host staging, optional scratch, executable images, kernel handles, kernarg
   slots, metric buffers, and egress vectors described below.
6. Call `prepare_pending_pool`. Every selected task receives one
   `Session::prepare_pending(2, 0)` token. The allocation keepalive capacity of
   two is sufficient for every direct HSA copy and the dependency capacity of
   zero is intentional because the executor establishes host DAG readiness and
   the adapter uses `dispatch_prepared`, not a dependency tree.

The result is `HsaPreparedHandoff::Candidate(runtime_artifacts)`, the per-device
resources, and the pending pool. No finalized arena is allocated here. On a
candidate failure, `LocalCandidateFactory` destroys HSA resources after any
already-created bridge, CUDA, or host resources according to its cleanup order.

### Direct finalized realization

`HsaResources::realize` (`hsa.rs:384-462`) is used by the `Ready` backend path
when no warmed candidate exists. It:

1. rejects duplicate bindings;
2. requires one binding for every device in the `ExecutionPlan`;
3. rejects a binding that has no plan device;
4. filters the bundle task list to the selected partition, if any;
5. checks the requested physical queue count against each binding's discovered
   `maximum_submission_queues` before creating queues;
6. validates every binding;
7. realizes each device;
8. attaches finalized artifact resource envelopes with
   `bind_finalized_artifact_resources`;
9. derives exact task contracts; and
10. prepares one pending token per scoped task.

`ExecutionPlan::validate` has already checked runtime artifact identity, image
digest, target ABI, entry symbol, workgroup bound, and calculation ABI. The HSA
adapter repeats the checks that are specific to the loaded kernel metadata and
the exact HSA resource envelope, so a malformed or substituted image cannot be
submitted.

### Prepared handoff and warm validation

`HsaPreparedResources::bind` (`hsa.rs:1250-1289`) accepts only the internal
`Finalized` handoff variant. It requires the prepared bundle identity and task
set to equal the final bundle and selected set, then binds artifact resource
envelopes and constructs `HsaResources` without allocating a new queue,
executable, staging buffer, or pending token.

The production local path uses `bind_candidate` (`hsa.rs:1291-1340`) while
activating warm resources. It requires the warm pending keys to equal the task
set and each warm device's init-image contract to equal the candidate's
finalized image. It validates the finalized partition, rebinds artifact
resource envelopes, and derives task contracts. The retained
`validate_handoff` method (`hsa.rs:1342-1370`) performs the same identity checks
for the alternate direct prepared path.

`HsaResources::validate_handoff` (`hsa.rs:1114-1138`) is used for an already
warmed backend. Before accepting a new bundle it requires that every checked
out token has been recycled and that `pending_pool` contains exactly the final
task set. It verifies each warm admission image, validates a partition plan,
rebinds finalized artifact resources, and replaces `plan` and `contracts`.

## Per-device resource realization

`realize_device` (`hsa.rs:1808-2050`) is the only constructor for a
`DeviceResources` value. It uses the reservation and manifest values as exact
inputs, not as defaults.

### Reservations and queues

`validate_reservation` requires a reservation entry for the device and
`ReservationMechanism::EnforcedQuota`. An HSA resource set without a scheduler
enforced quota is rejected with `ArenaMismatch`.

The task submissions determine the distinct queue and completion slot IDs. Only
resource manifest queue slots for this device and this task subset are created.
Each queue uses `QueueConfig::new(binding.queue_packets, QueueKind::SingleProducer)`.
The `recipe-hsa` session additionally checks that the size is a power of two in
the discovered queue range, that the agent advertises single-producer support,
and that the returned queue has a non-null ring, a valid size, and kernel
dispatch capability. Completion entries start as `Available`.

### Host staging, admission, scratch, and egress

The device's `pinned_staging` manifest entry supplies the exact fine-grained
staging byte count. `HsaBinding::allocate_host_fine` allocates that amount from
the exact CPU allocator and grants GPU access. A device must also have an
`InitDataImage` manifest entry. Its image byte count must fit in the staging
allocation, and the saved `admission` contract must match the final image.

If the resource manifest contains nonzero scratch bytes, the device session
allocates one coarse scratch allocation. Zero or absent scratch produces
`None`.

For every exit transfer whose source is this device and whose destination is
external, `realize_device` allocates a host `Vec<u8>` of the exact transfer
size. This vector is not a native allocation. The async copy first lands in
fine-grained staging, and terminal handling copies staging into this vector.

### HSACO grouping, inspection, and symbol lookup

Calculation artifacts assigned to the device must have
`RuntimeArtifactKind::Hsa { target_id, code_object_version }` matching the
binding exactly. Artifacts are grouped by `ArtifactDigest`. Distinct byte
images with one digest are rejected if their bytes differ. One executable is
loaded per distinct image using `Session::load_hsaco`.

For each grouped image, `inspect_hsaco_bundle` checks the AMDGPU-HSA ELF,
code-object version, target ID, global entry and descriptor symbols, AMDGPU
metadata, argument order and sizes, kernarg alignment, and workgroup bound.
The inspection order is the runtime artifact order. For each inspected entry:

1. the inspected logical kernel name must equal `runtime.abi.entry_symbol`;
2. the inspected runtime symbol is looked up through
   `Executable::kernel`; a failed lookup becomes `HsaSymbolLookup` with the
   logical ABI entry and the descriptor symbol;
3. the loaded ROCr metadata must be at least as large as the inspected
   `argument_bytes` and `argument_alignment`; and
4. `LoadedArtifact` stores the returned `Kernel`, immutable `KernelAbi`, and a
   resource envelope placeholder.

`bind_finalized_artifact_resources` later requires the loaded ABI to equal the
finalized runtime ABI. `hsa_artifact_resource_envelope` copies the finalized
`KernelResourceBounds`. For a kernel with a dynamic callstack, finalized
`private_bytes_per_lane` must be nonzero and fit the AQL `u32` field; that value
becomes `DispatchGeometry::dynamic_private_bytes`. A non-dynamic kernel gets a
zero dynamic-private allowance.

### Kernarg, metric, and executable lifetime

`kernarg_sizes` computes the maximum loaded ROCr kernarg segment size for each
completion slot used by calculations on this device. One CPU kernarg allocation
and one zeroed host byte vector are created per slot, and the HSA device session
is granted access to each allocation. Sharing is allowed only among tasks that
share that planned completion slot and use a size no larger than the maximum.

Each metric task assigned to this device gets one four-byte fine-grained metric
allocation. A metric is a specialized four-byte device readback, not another
kind of model work. No metric allocation is created for a missing value device.

The HSA `Session`, `Queue`, `Executable`, `Kernel`, `Allocation`, and
`PreparedPending` objects retain ROCr handles and their keepalive references.
`recipe-hsa` keeps allocations and the executable alive until the terminal
completion signal is observed. Closing an allocation or executable while any
dependent object remains live returns `ResourceBusy`; the native executor's
drop order avoids that condition during normal teardown.

## Arena allocation and access

`HsaResources::allocate_arena` (`hsa.rs:464-485`) finds the owner device,
converts the manifest byte count to `usize`, and allocates a coarse buffer from
that device's session. It calls `Session::grant_access_exact_set` with every
HSA device session and the owner binding's CPU allocator. This replaces the allocation's
direct-access set with the exact discovered agents required by all local HSA
copies and host accesses. A successful call returns an `HsaArena` tagged with
the requested `DeviceId`.

`checked_arena` (`hsa.rs:2391-2416`) is used by every path that consumes a
resolved value location. It requires an arena for the resolved device, checks
that the arena's tag agrees with the location, checks offset plus byte count for
integer overflow, and rejects a range beyond the allocation. `offset_to_usize`
and `bytes_to_usize` convert every native ABI `u64` count explicitly and return
`IntegerOverflow` instead of truncating.

## Submission paths

### Preparing a pending token

`HsaResources::prepare_pending` (`hsa.rs:487-539`) is called by both the
`Backend` implementation and the local warm scheduler. It first requires a
healthy resource set. It then requires:

- an exact `HsaTaskContract` for `request.task`;
- equal phase, work class, and optional submission slots;
- an immutable `ExecutionPlan` submission for the task;
- a realized queue and completion slot for that submission; and
- no earlier preparation of the same task.

It removes the task's `PreparedPending` from `pending_pool`, inserts the task in
`prepared_tasks`, and returns `HsaPending::ready`. A duplicate or absent token
is a `Protocol` error. Removing the token before a submit means one task cannot
accidentally share a native signal with another task.

### Common submit gate

`HsaResources::submit` (`hsa.rs:541-580`) is the single dispatch entry. It
requires health, retrieves the immutable planned submission, validates the
pending token, and validates the work against its `HsaTaskContract`. It then
selects exactly one of `submit_admission`, `submit_internal_transfer`,
`submit_calculation`, `submit_metric`, or `submit_exit_transfer`. On success it
records the resulting action and marks the pending token `Active`. On failure,
`submission_error_requires_poison` sets the adapter poison bit for a session
poison, a negative asynchronous signal, or a deferred-retirement error marked
poisoned. Other errors leave the adapter unpoisoned so the caller can observe
the direct contract failure.

Every native submission claims its planned completion slot immediately before
publishing work. `claim_completion` changes `Available` to
`Active { task }`; an already active slot returns `CompletionBusy`. If the
ROCr submission call fails, `finish_submission_claim` releases the claim before
returning the converted HSA error.

### Init admission

`submit_admission` (`hsa.rs:713-767`) accepts only a device destination whose
device and slots equal the immutable plan. It validates the source arena and
the exact init-image contract, checks the caller image length, and checks that
fine staging is large enough. It copies the input image into fine staging with
the host-accessible `copy_from_host` operation, then submits
`Session::copy_async_prepared` from staging offset zero into the destination
arena offset. Its action is `None`.

This is the only HSA init admission upload. The staging allocation is reused by
later exit transfers after the init operation reaches terminal completion.

### Internal device transfer

`submit_internal_transfer` (`hsa.rs:769-807`) requires two resolved device
endpoints. The source device must equal the planned device and the submission
slots must match. Both arenas and both offsets are range checked. It submits
one `copy_async_prepared` from source arena to destination arena and records
`PendingAction::None`.

External endpoints are rejected here by `device_endpoints`; they are not
silently converted to host operations.

### Exit transfer

`submit_exit_transfer` (`hsa.rs:809-883`) always requires a device source on the
planned device. For a device destination it performs the same checked native
device-to-device copy as an internal transfer with action `None`. For an
external destination it requires a preallocated egress vector and sufficiently
large staging, submits a copy from the source arena into staging offset zero,
and records `PendingAction::Egress { bytes }`.

The caller cannot read egress immediately after submission. `finish_action`
copies staging into the preallocated per-task vector only after the completion
signal has reached terminal completion. `collect_exit` then copies that vector
into the caller's destination after validating the exact terminal pending token.

### Calculation dispatch

`submit_calculation` (`hsa.rs:885-989`) requires the calculation device and
slots to match the plan. It obtains the loaded artifact and finalized resource
envelope, finds the kernarg slot for the completion slot, and calls
`fill_kernarg` before publishing an AQL packet.

`fill_kernarg` (`hsa.rs:2306-2390`) zeroes the preallocated host byte vector and
writes one little-endian 64-bit value per ABI argument:

| ABI argument | encoded value |
| --- | --- |
| `KernelArgument::Buffer` | HSA allocation base address plus resolved arena offset |
| `RunId` | current work run ID |
| `LoopIteration` | zero-based loop iteration index |
| `ElementCount` | immutable ABI element count |
| `FaultFlag` | resolved int32, four-byte arena value, encoded as its device pointer |

Buffer and fault-flag locations are range checked. A fault flag must be exactly
`DType::I32` and four bytes. Missing or extra operands, a missing fault flag,
an unexpected fault flag, pointer addition overflow, or a kernarg vector smaller
than the ABI are protocol, value, or integer errors. After filling, the complete
vector is copied into the host-visible kernarg allocation before the AQL packet
is published.

Before dispatch, the adapter calls `Queue::progress_capacity(1,
NonZeroU32::MIN)`. A backpressured queue returns `ResourceContention` without
publishing a packet. It constructs one-dimensional geometry from the immutable
ABI:

```text
workgroup_lanes = nonzero ABI workgroup width
grid = ceil(elements / workgroup_lanes) * workgroup_lanes
dimensions = 1
workgroup = [workgroup_lanes, 1, 1]
dynamic_private_bytes = finalized resource envelope value
```

The padded grid must fit `u32`, the workgroup width must fit the AQL `u16`
field, and the geometry must still cover every logical element. The adapter
then claims the completion slot and calls `Queue::dispatch_prepared` with the
loaded kernel, kernarg allocation, geometry, and the pre-realized pending
token. `recipe-hsa` validates exact session ownership, single-producer queue
publication, executable agent identity, discovered ISA geometry limits, kernarg
pool flag, access grant, size, alignment, and dynamic-callstack private bytes.

The executor has already established host DAG readiness. HSA calculation
dispatch therefore uses a dependency-free prepared packet and does not build a
new device dependency tree.

### Metric readback

`submit_metric` (`hsa.rs:991-1038`) requires the resolved value to be on the
planned device and exactly four bytes. It range checks the source arena, finds
the task's preallocated four-byte metric buffer, claims the completion slot,
and submits a four-byte `copy_async_prepared` into that buffer. The action stores
the value's `DType` as `PendingAction::Metric`.

## Polling, terminal actions, and collection

`HsaResources::poll_pending` (`hsa.rs:582-606`) requires a healthy resource set,
requires the pending token's completion slot to be active for the same task,
and requires the pending state to be `Active`. It calls
`PreparedPending::poll` exactly once per poll request:

```text
PollStatus::Pending  -> BackendPoll::Pending
PollStatus::Complete -> finish_pending
HSA error            -> poison the resource set and return the error
```

`recipe_hsa::PreparedPending::poll` observes the completion signal without
blocking. A positive signal is pending. Zero is terminal and releases the
native keepalive references. A negative signal is `AsyncSignal`. A queue error
callback can poison the session even while this operation's signal is still
positive; the token remains nonterminal so `recipe-hsa` retires its signal and
all referenced resources instead of dropping objects still visible to ROCr.

`finish_pending` releases the executor completion claim, marks the token
`Terminal`, and runs `finish_action`:

- `None` returns `BackendPoll::Complete { metric: None }`;
- `Metric { dtype }` reads four bytes from the fine-grained metric allocation
  after terminal system-scope completion and decodes little-endian `f32` or
  `i32` into `MetricValue`;
- `Egress { bytes }` verifies the preallocated output length and staging size,
  copies fine staging into the per-task egress vector, and returns completion
  without a metric.

If terminal action processing fails, `finish_pending` poisons the HSA resource
set and returns the error. `take_egress` removes a completed task's vector from
the device maps. `collect_exit` requires a healthy set, a valid exit contract,
the terminal egress action, an exact caller buffer length, and a source arena
range before copying the saved vector to the caller. A bridge pending token or
an external source is rejected because external collection belongs only to the
HSA child backend.

## Loop repetition and warm execution

The HSA backend reports `supports_loop_repetition() == true` and sets
`MAX_NON_POLL_PHYSICAL_CALLS` to `1` (`hsa.rs:1381-1575`). The local composite
uses this capability for both candidate warm passes and finalized static-loop
iterations.

`prepare_loop_pending` (`hsa.rs:1068-1084`) accepts only a loop token. A
`Ready` token is used as-is. A `Terminal` token is rearmed. An `Active` token
cannot be submitted again. `rearm_pending` requires a terminal loop token,
calls `PreparedPending::reset`, clears its action, and returns it to `Ready`.
The recipe-hsa reset operation polls once if necessary, refuses a still-positive
signal with `ResourceBusy`, and restores the same signal and keepalive vectors
only after terminal completion.

`recycle_pending` (`hsa.rs:1086-1104`) is the warm/candidate return path. It
requires a terminal token and removes the task from `prepared_tasks`, resets
the native token, and inserts it back into `pending_pool`. A second recycle or
a pool collision is a protocol error. The method never allocates a replacement
signal.

The production warm caller is `run_warm_trace` in `local.rs:2844-2990`:

1. `LocalPreparedSession::activate_warm_resources` calls
   `HsaPreparedResources::bind_candidate`, moving candidate HSA objects into
   `HsaResources`.
2. The warm scheduler walks `Init`, `Loop`, and `Exit` phases, respecting task
   dependencies and schedule-window overlap.
3. `prepare_warm_pending` calls `HsaResources::prepare_pending` for HSA-owned
   tasks. `submit_warm_work` calls `HsaResources::submit`; HSA polling calls
   `poll_pending`.
4. On terminal completion, warm exit tasks call `collect_exit`, then
   `recycle_warm_pending` calls `recycle_pending`.
5. Capacity observation releases warm arenas only after all tokens are
   recycled. `LocalPreparedSession::into_backend` transfers the warmed
   `HsaResources` into `HsaBackend::from_warmed` and requires that the candidate
   arenas are gone.

The finalized `LocalBackend` follows the same child calls through its
`Backend` implementation (`local.rs:1647-2108`). It classifies each task as
`Host`, `Cuda`, `Hsa`, or `Bridge`; HSA-owned tasks become
`LocalPending::Hsa`. `prepare_pending`, `allocate_arena`, `submit`,
`submit_loop_iteration`, `poll`, and `collect_exit` dispatch directly to the
HSA child. `ProjectedArenas` implements `HsaArenaLookup` over the immutable
executor arena map, so HSA never owns or recreates another arena map.

## HSA legs in `StagedCrossBackend`

Cross-backend transfers are not HSA `HsaResources` tasks. `StagedCrossBackend`
(`bridge.rs:172-430`) owns a separate one-hop resource for each bridge task.
For an HSA endpoint, `realize_leg` allocates fine-grained host staging through
the binding and creates one `Session::prepare_pending(2, 0)` token. The bridge
also validates exact task phase, device/value endpoints, bytes, route,
lane claims, and submission slots before and after finalization.

At submit (`bridge.rs:1066-1290`), an HSA source calls
`Session::copy_async_prepared(staging, 0, arena, offset, bytes, pending)`. An
HSA destination calls the reverse copy from staging into the projected arena.
The bridge worker performs any host middle copy between the two endpoint
staging buffers. `poll_leg` maps the HSA prepared token's `Pending` and
`Complete` states to the composite `BackendPoll`. A terminal bridge loop token
is rearmed by `PreparedPending::reset`; candidate recycle performs the same reset
before returning the token to the bridge's prepared-token slot.

The HSA leg uses the same recipe-hsa checks for nonzero copy size, bounds,
mutual allocation access, session identity, signal state, and asynchronous
failure. Errors are wrapped as `StagedBridgeError::Hsa`, not as native
`Error::Hsa`. Bridge tasks cannot have external endpoints in the finalized
local path; external input and output are owned by the corresponding native
child backend.

## `recipe-hsa` boundary invariants

The adapter delegates raw ROCr ownership to `recipe-hsa`; these invariants are
observable at the calls made by `hsa.rs`.

### Session, queues, and agents

`DiscoveredAgent::into_session` accepts only a GPU agent with kernel-dispatch
capability, at least one ISA with an exact AMD target, and a live HSA runtime.
`Session::ensure_healthy` observes the shared queue-callback fault record. The
first callback records status, source queue ID when available, and an epoch;
poisoning is permanent. `Session::available_memory_bytes` and
`Session::create_queue` both check runtime liveness and session health before
calling ROCr.

`create_queue` validates the configured packet count against the discovered
minimum and maximum and requires a power of two. It validates producer kind,
installs the callback that feeds the shared fault record, and checks the
returned queue's ring pointer, size, kind, and kernel-dispatch feature bit. The
adapter requests `QueueKind::SingleProducer` because the confined AQL writer
publishes one producer's packets. Even if ROCr reports a different read-only
kind field, `recipe-hsa` retains and enforces the requested producer discipline.
`Queue::close` requires the queue's `Rc` core to have no pending-token
keepalive; otherwise it returns `ResourceBusy`.

### Allocations and access sets

`Session::allocate_coarse`, `allocate_fine`, and `allocate_kernarg` select the
first discovered global pool with the corresponding runtime-allocation and
global flags. A zero byte request, a pool above its aggregate limit, an
unallocatable pool, a null pointer, or a mismatched pool index is an HSA error.
Allocations initially grant direct access only to their pool owner.

`grant_access` and `grant_access_exact_set` use ROCr's replacement semantics,
not additive semantics. The owner is always retained, while the supplied
sessions and agents become the exact direct-access set. `realize_device` uses
the one-agent grant for fine staging, metric buffers, and kernargs. Arena
allocation uses the exact set of every HSA session plus the owner CPU allocator.
`copy_async_prepared` rejects a pair when either endpoint agent lacks direct
access to the other allocation.

The unsafe host copies used by admission, metric completion, egress completion,
and kernarg upload are valid only for fine-grained host-accessible allocations
and only when no device operation can overlap the range. The adapter calls
`copy_to_host` only after terminal system-scope completion.

### Prepared signals and deferred retirement

`Session::prepare_pending(allocation_capacity, dependency_capacity)` acquires a
completion signal and reserves both keepalive vectors before init. The HSA
adapter always requests `(2, 0)`: two allocation keepalive entries cover a
direct copy, and no dependency vector is needed by `dispatch_prepared`.

`PreparedPending` has its own native state in addition to `HsaPending`:

```text
Ready { signal, reserved vectors }
  | begin and publish
  v
Active(Pending)
  | signal reaches zero
  v
terminal Pending -> reset -> Ready
```

`begin` consumes `Ready`, verifies vector capacities, clears prior keepalives,
and returns the same signal plus fixed storage. A failed queue or copy
publication restores `Ready`; a successful publication moves to `Active` and
retains the queue, executable, and allocation handles needed by the device. A
terminal poll marks the signal zero, releases its retirement reservation, and
drops those keepalives. A negative signal records the first fatal value and
returns `AsyncSignal`.

Dropping an incomplete `Pending` never guesses that the device stopped. It
moves the signal and all keepalives into the session's explicit deferred
retirement set. `Session::poll_retirements` performs one nonblocking scan. Each
terminal retired signal is removed, its keepalives are dropped before its signal
is recycled or destroyed, and a negative value is remembered as a fatal signal.
`drain_retirements` repeats bounded scans and bounded active waits until the set
is empty, a poison is observed, or the timeout expires. Poison and timeout leave
the unresolved set intact and return `DeferredRetirement`; this is why HSA
destruction drains before closing queues or executable objects.

`PreparedPending::reset` is the only signal rearm operation. It is a no-op for
an unused `Ready` token, refuses an active positive signal with `ResourceBusy`,
polls a terminal active operation, restores the fixed keepalive storage, and
stores signal value one. The native adapter calls reset only from a terminal
HSA loop token or a terminal candidate token.

### Executables and AQL packets

`Session::load_hsaco` creates a reader from an owned byte image, creates a ROCr
executable, loads the image for the exact agent, freezes the executable, and
then checks session health again. `Executable::kernel` resolves the requested
symbol, verifies that it is a kernel with a nonzero object, reads kernarg,
group, private, alignment, and dynamic-callstack metadata, and rejects an
invalid alignment. A `Kernel` retains an `Rc` owner for its executable. The
native artifact grouping and final resource-envelope checks surround this API,
so every logical Recipe entry resolves from the inspected shared image.

`Queue::dispatch_prepared` validates session and queue identity, exact kernel
agent, geometry limits, kernarg pool flag, access grant, size, and alignment.
It computes static plus dynamic group and private segment sizes, acquires no
new signal, writes an AQL kernel packet with an initially invalid header, and
publishes the valid header, write index, and doorbell with release ordering.
`enqueue_packets` first reads write and read indices and returns `QueueFull` if
the ring lacks the required slot. `progress_capacity(1, NonZeroU32::MIN)` is a
bounded read-only probe, so HSA calculation submission never sleeps, blocks, or
allocates.

The general `dispatch_after` API can lower a dependency list into a bounded
barrier-AND tree, but the native executor does not call it. Host executor DAG
readiness and the immutable task order are the dependency boundary for this
adapter.

## Backend accounting and evidence

Every HSA backend operation records one `PhysicalCall` before it crosses the
native boundary: bind, prepare pending, arena allocation, submission,
completion poll, exit collection, arena release, and resource destruction.
`MAX_NON_POLL_PHYSICAL_CALLS` is one, so a submission cannot hide additional
physical calls. Accounting overflow is surfaced as
`PhysicalAccountingOverflow`.

`execution_evidence` (`hsa.rs:608-626`) reports one
`NativeDeviceExecutionEvidence` per HSA device:

| evidence | source |
| --- | --- |
| `backend` | `NativeBackendKind::Hsa` |
| `image_loads` | count of distinct executable image digests |
| `entry_lookups` | count of loaded logical artifact entries |
| `queues` | realized queue slot count |
| `completion_objects` | completion slot count |
| `persistent_allocations` | kernarg slots plus metric buffers plus optional scratch plus one staging allocation |

The evidence is collected by `LocalBackend::destroy_resources` only after HSA,
CUDA, bridge, and host resources have been selected for deterministic teardown.
It describes realized persistent work, not a claim that a run succeeded.

## Teardown and failure behavior

### Normal destruction

`HsaResources::destroy` (`hsa.rs:699-711`) requires a healthy adapter, drops
the pending pool first, and then calls `destroy_devices`. Dropping prepared
tokens first releases any executable keepalive retained by a ready token before
the explicitly owned executable handles are closed.

`destroy_devices` (`hsa.rs:2571-2607`) first verifies that every completion slot
is `Available`. Active slots fail with `ResourceContention`; the adapter does
not close queues or allocations while a task may still reference them. It then
calls `Session::drain_retirements(Duration::from_millis(10))` for every device.
The recipe-hsa session drains deferred signals and their allocation, queue, and
executable keepalives, returning an asynchronous or deferred-retirement error
if they do not become terminal in the bounded interval. Once drained, the close
order is:

```text
queues
loaded Kernel handles
shared Executable handles
kernarg allocations
metric allocations
fine staging
optional coarse scratch
```

The order preserves ROCr dependencies. Allocation and executable `close` calls
must have unique owners; a live dependent object produces `ResourceBusy`.

`HsaPreparedResources::destroy` follows the same pending-drop then device
destruction path. `LocalPreparedSession::destroy` invokes bridge, HSA, CUDA,
and host cleanup in its fixed order and retains the first error.

### Poisoning

`ensure_healthy` rejects every later HSA operation with
`Error::BackendPoisoned { backend: "HSA" }` after `poisoned` is set. Poison is
set when:

- `PreparedPending::poll` reports a queue/session fault or a negative signal;
- terminal metric or egress materialization fails;
- a submission returns `SessionPoisoned`, a poisoned `DeferredRetirement`, or
  a negative `AsyncSignal`;
- native poll itself returns a recipe-hsa error and `poll_pending` routes it
  through `poison`.

Contract errors such as missing artifacts, bad routes, busy completion slots,
queue backpressure, unsupported endpoint classes, arena range mismatches, and
integer overflow are returned directly. They do not become a device fault
unless the underlying recipe-hsa error is session-fatal.

At the `Backend::poll` boundary, every HSA error variant is converted to
`PhysicalPollStatus::Failed` for accounting, while the original `Error` is
returned to the executor. No failed poll is reported as completion.

The current integration contract explicitly notes that unhealthy native
resources refuse destruction. Consequently, after a poisoned HSA run,
`destroy_resources` can report `BackendPoisoned` before it reaches the normal
queue and allocation close sequence. This is an existing teardown limitation,
not a hidden recovery path. The runtime must preserve the failure and perform
the ordered teardown work required by the integration contract.

### Error classes exposed by `hsa.rs`

The following classes are intentionally fail-closed:

| class | representative causes |
| --- | --- |
| identity and topology | duplicate, missing, or unexpected device; duplicate, missing, or unexpected artifact |
| loaded image | wrong HSA artifact kind, target, code-object version, digest, entry symbol, ABI, metadata, or resource bound |
| native slots | missing queue or completion; queue-count limit; completion already owned |
| memory and values | missing arena, wrong arena owner, out-of-range offset, bad byte count, inaccessible allocation, wrong metric dtype or size |
| work contract | phase, class, route, lane, or submission mismatch; invalid transfer endpoint; calculation or metric in the wrong phase |
| queue and dispatch | AQL backpressure, zero-lane artifact, grid or workgroup overflow, geometry outside discovered ISA limits, invalid kernarg |
| asynchronous HSA | negative completion signal, queue callback session poison, deferred retirement, runtime operation failure |
| lifecycle | duplicate bind, duplicate pending preparation or recycle, active token reuse, collection before terminal completion, active completion during destruction |

`ensure`, `bytes_to_usize`, `offset_to_usize`, `u32_from_u64`, `u16_from_u32`,
and `hsa_grid_size` are the local conversion and assertion helpers. They return
typed `Error` values rather than truncating or introducing a fallback value.

## Source map

The main implementation regions are:

| region | responsibility |
| --- | --- |
| `hsa.rs:31-121` | binding construction, accessors, availability, host fine allocation |
| `hsa.rs:135-251` | arena, loaded artifact, kernarg, completion, and device-resource types |
| `hsa.rs:253-357` | backend state and one-shot bind transitions |
| `hsa.rs:384-711` | finalized realization, arena allocation, pending preparation, submit and poll, evidence, collection, and destroy |
| `hsa.rs:713-1038` | admission, internal and exit copies, calculation dispatch, and metric submission |
| `hsa.rs:1040-1157` | terminal handling, loop reset, pending recycling, handoff validation, poison |
| `hsa.rs:1159-1380` | candidate realization, prepared handoff, warm handoff, and prepared destroy |
| `hsa.rs:1381-1575` | `recipe_executor::Backend` implementation and accounting |
| `hsa.rs:1577-1690` | pending state machine and token identity validation |
| `hsa.rs:1692-2050` | binding, reservation, pending-pool, and per-device realization |
| `hsa.rs:2052-2305` | artifact resource envelopes, task contracts, transfer classes, queue counts, and kernarg sizing |
| `hsa.rs:2306-2679` | kernarg encoding, arena and endpoint checks, completion ownership, teardown, and numeric helpers |
| `local.rs:1507-1528` | candidate caller that realizes HSA resources |
| `local.rs:2844-3235` | warm scheduler calls to prepare, submit, poll, collect, and recycle |
| `local.rs:1647-2108` | finalized composite backend dispatch to the HSA child |
| `bridge.rs:172-430,1066-1580` | separate HSA staging legs for cross-backend transfers |
| `hsa/src/execution.rs:1282-1575` | prepared signals, reset, polling, retirement, and prepared async copies |
| `hsa/src/execution.rs:1969-2117` | prepared AQL dispatch, queue capacity probing, and geometry validation |
