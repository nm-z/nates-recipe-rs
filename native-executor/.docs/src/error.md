# Native executor error contract

## Scope

`recipe_native_executor::Error` is the single error type used by the native
CUDA Driver and ROCr/HSA adapters. It is defined in
`native-executor/src/error.rs`, is re-exported by `native-executor/src/lib.rs`,
and is the associated `Backend::Error` for `CudaBackend` and `HsaBackend`.
`Result<T>` is an alias for `core::result::Result<T, Error>`.

The enum is `Debug`, `PartialEq`, and `Eq`, and is `#[non_exhaustive]`. Code
outside this crate must therefore include a wildcard when matching it. The
error type does not own recovery policy. A returned error stops the operation
at the current backend boundary. The executor records the failed backend
operation and decides whether the run can be cleaned up.

This page documents the current source, including variants that are only
matched for exhaustiveness. It does not describe `LocalError`,
`StagedBridgeError`, or driver error enums as if they were native errors. Those
types are separate wrappers and their relationship to this enum is described
in [Propagation](#propagation-and-execution-consequences).

## Display and source behavior

`Error` implements `fmt::Display` with the exact messages in the table below.
`impl std::error::Error for Error {}` uses the default `source` method, so a
native error does not expose a nested source through the standard error chain,
even for `Cuda`, `Hsa`, `Kernel`, and `HsaSymbolLookup`. The nested value is
still rendered by `Display`.

| Variant | Payload | Exact display prefix or message |
| --- | --- | --- |
| `DuplicateDevice` | `device: DeviceId` | `native device {device} appears more than once` |
| `DuplicateArtifact` | `artifact: ArtifactId` | `runtime artifact {artifact} appears more than once` |
| `MissingArtifact` | `artifact: ArtifactId` | `runtime artifact {artifact} is missing` |
| `UnexpectedArtifact` | `artifact: ArtifactId` | `runtime artifact {artifact} is not present in the finalized bundle` |
| `ArtifactMismatch` | `artifact: ArtifactId`, `detail: String` | `runtime artifact {artifact} is incompatible: {detail}` |
| `MissingDevice` | `device: DeviceId` | `native device {device} is missing` |
| `UnexpectedDevice` | `device: DeviceId` | `native device {device} has no finalized arena` |
| `MissingQueue` | `task: TaskId`, `queue: QueueSlotId` | `task {task} references unavailable queue slot {queue}` |
| `MissingCompletion` | `task: TaskId`, `completion: CompletionSlotId` | `task {task} references unavailable completion slot {completion}` |
| `ResourceContention` | `task: TaskId`, `detail: &'static str` | `task {task} has native resource contention: {detail}` |
| `CompletionBusy` | `backend: &'static str`, `task: TaskId`, `completion: CompletionSlotId`, `owner: TaskId` | `{backend} task {task} cannot claim completion slot {completion}; task {owner} owns it` |
| `ArenaMismatch` | `device: DeviceId`, `detail: &'static str` | `arena for device {device} is incompatible: {detail}` |
| `ValueMismatch` | `value: ValueId`, `detail: &'static str` | `resolved value {value} is incompatible: {detail}` |
| `UnsupportedTransfer` | `task: TaskId`, `detail: &'static str` | `transfer task {task} is unsupported: {detail}` |
| `UnsupportedLoopContract` | `backend: &'static str`, `detail: &'static str` | `{backend} cannot uphold the strict loop contract: {detail}` |
| `BackendState` | `backend: &'static str`, `detail: &'static str` | `{backend} backend state is invalid: {detail}` |
| `BackendPoisoned` | `backend: &'static str` | `{backend} backend is poisoned` |
| `SubmissionQueueLimitExceeded` | `backend: &'static str`, `device: DeviceId`, `requested: usize`, `maximum: u32` | `{backend} device {device} requires {requested} physical submission queues, exceeding its discovered maximum of {maximum}` |
| `IntegerOverflow` | `field: &'static str` | `{field} does not fit the native ABI` |
| `PhysicalAccountingOverflow` | none | `native backend exceeded the fixed physical-call accounting capacity` |
| `Cuda` | `recipe_cuda::CudaError` | `CUDA Driver operation failed: {detail}` |
| `CudaContract` | `&'static str` | `CUDA Driver resource contract failed: {detail}` |
| `HsaSymbolLookup` | `artifact: ArtifactId`, `abi_entry: String`, `runtime_symbol: String`, `source: recipe_hsa::Error` | `HSACO artifact {artifact} logical ABI entry {abi_entry:?} failed ROCr lookup for descriptor symbol {runtime_symbol:?}: {source}` |
| `Hsa` | `recipe_hsa::Error` | `ROCr/HSA operation failed: {detail}` |
| `Kernel` | `recipe_kernel::LoweringError` | `kernel artifact validation failed: {detail}` |
| `Protocol` | `task: TaskId`, `detail: &'static str` | `task {task} violates the native protocol: {detail}` |

The implementation is in `native-executor/src/error.rs:99-227`. The three
`From` implementations at lines 231-241 are the only conversions into the
driver and kernel wrapper variants. `ArtifactMismatch` and
`HsaSymbolLookup` carry owned `String` fields because pre-final artifact
inspection can include computed diagnostics. No `Display` implementation adds
backend names or operation names beyond the strings shown above.

## Construction index

The following index names every source function that constructs a native
`Error` variant. A `?` at one of these sites returns the value to its caller;
there is no alternate native error path or fallback.

### Plan validation: `plan.rs`

`ExecutionPlan::validate_scoped` (`plan.rs:130-207`) constructs
`DuplicateArtifact` for repeated runtime IDs, `MissingArtifact` when a
finalized artifact has no matching runtime image, `UnexpectedArtifact` through
`reject_unexpected_artifact`, `Protocol` when a selected partition task lacks
an immutable submission, and `MissingDevice` when a selected submission is
not in the partition device set. The function calls
`validate_artifact_contract` and `plan_submissions`, so their errors abort plan
construction before native resources are realized.

`validate_artifact_contract`, `validate_runtime_artifact`, `validate_target`,
and `validate_calculation_abi` (`plan.rs:227-640`) construct
`ArtifactMismatch`. These checks cover runtime ID, image digest, ABI entry,
format and target ABI, workgroup width, CUDA target and digest, HSA target,
kernel template identity, element count, argument count and ordering, buffer
dtype, access, alignment and byte size, resolved locations, and the optional
aligned device `FaultFlag`. The calculation ABI checks turn argument-count
and argument-index arithmetic overflow into `ArtifactMismatch` with a detail
string, not `IntegerOverflow`.

`plan_submissions` (`plan.rs:642-672`) constructs `ValueMismatch` when a
metric value has no finalized location. `transfer_submission_device`
(`plan.rs:678-689`) constructs `Protocol` for an external-to-external
transfer, because such a transfer has no native device. `validate_slots`
(`plan.rs:695-714`) constructs `MissingQueue` or `MissingCompletion` when a
slot is absent or belongs to another device.

### CUDA adapter: `cuda.rs`

The CUDA adapter returns the crate `Result` from all resource, pending-token,
submission, poll, exit, arena-release, and destruction methods. The direct
construction sites are:

* `CudaBinding::available_bytes` and `CudaResources::available_bytes`
  (`cuda.rs:80-89`, `945-958`) construct `ArenaMismatch` if a driver free-memory
  counter cannot be represented as Recipe byte units. The driver query itself
  converts to `Cuda` through `From`.
* `CudaBackend::bind_partition`, `CudaResources::validate_handoff`,
  `CudaPreparedResources::bind`, `bind_candidate`, and `validate_handoff`, and
  the `Backend::bind_resources` implementation (`cuda.rs:295-322`,
  `960-979`, `1088-1187`, `1201-1232`) construct `BackendState` for reuse,
  candidate/finalized handoff mismatches, and a bind attempted after the state
  has become `Bound`. They construct `ArenaMismatch` when the warm or
  finalized init-image contract differs.
* `CudaResources::realize` and `CudaPreparedResources::realize`
  (`cuda.rs:348-421`, `1001-1086`) construct `DuplicateDevice` and
  `MissingDevice`. Finalized `CudaResources::realize` also rejects an
  `UnexpectedDevice` and calls the shared capacity helper, which can return
  `SubmissionQueueLimitExceeded`. The pre-final
  `CudaPreparedResources::realize` additionally constructs
  `UnexpectedArtifact`, `DuplicateArtifact`, and `MissingArtifact` while it
  reconciles candidate runtime images. Device creation and artifact loading
  then call `validate_binding`, `validate_reservation`, and `realize_device`,
  which add the variants listed below.
* `CudaResources::allocate_arena` and `prepare_pending`
  (`cuda.rs:423-478`) construct `MissingDevice` and `Protocol` for an absent
  task contract, request/contract mismatch, absent immutable submission, an
  unrealized queue or completion slot, or duplicate pending preparation.
* `CudaResources::submit` (`cuda.rs:480-511`) propagates all preflight and
  operation errors from the five work-class submitters. Any error returned by
  one of those submitters sets `poisoned = true` before the error is returned.
* `validate_work_contract` (`cuda.rs:513-552`) constructs `Protocol` for a
  missing task contract, class or slot mismatch, an admission that differs from
  the finalized init image, or transfer route and lane-claim mismatch.
* `CudaResources::poll` (`cuda.rs:554-578`) constructs `Protocol` for a
  non-active pending token and propagates `MissingQueue`, `MissingCompletion`,
  and completion ownership errors from `validate_active`. A driver poll error
  is converted to `Cuda` and passed through `poison`, which sets the poisoned
  flag.
* `collect_exit` (`cuda.rs:605-642`) constructs `UnsupportedTransfer` when the
  source is not a device and `Protocol` when the pending token, endpoints,
  staging, or caller output do not match the completed exit contract.
* `submit_admission` (`cuda.rs:649-694`) constructs `Protocol` for a device or
  init-image mismatch, and propagates `MissingQueue`, `MissingCompletion`,
  `ArenaMismatch`, `ValueMismatch`, and `IntegerOverflow` from its arena and
  slot helpers. `submit_calculation` (`cuda.rs:696-738`) propagates
  `Protocol`, `MissingArtifact`, `MissingCompletion`, `MissingQueue`,
  `ResourceContention`, `ArenaMismatch`, `ValueMismatch`, and
  `IntegerOverflow`. `submit_internal_transfer` (`cuda.rs:740-770`) constructs
  `UnsupportedTransfer` for a non-device route or a route that is not a
  same-context D2D copy. `submit_metric` (`cuda.rs:772-809`) constructs
  `ValueMismatch` for a non-four-byte value or wrong device and `Protocol` for
  an absent metric buffer. `submit_exit_transfer` (`cuda.rs:811-858`)
  constructs `UnsupportedTransfer` for a non-device source, a non-external
  destination, or a wrong device, and `Protocol` for absent egress staging.
* `finish_pending`, `rearm_pending`, and `prepare_loop_pending`
  (`cuda.rs:860-930`) construct `Protocol` for an absent completion token, a
  completion slot that was not checked out by the pending task, a non-terminal
  or non-loop token, or an active token submitted again. `finish_pending`
  propagates `MissingCompletion`, `Protocol`, `Cuda`, and the metric or egress
  checks from `finish_action`. `recycle_pending` constructs `Protocol` when a
  token is not the one terminal token prepared for its task.
* `CudaPending::validate_ready` (`cuda.rs:1409-1432`) constructs `Protocol`
  when work does not match its pre-realized pending token.
* `task_contracts` and `transfer_work_class` (`cuda.rs:1485-1601`) construct
  `Protocol` for missing admission endpoints or manifests, admission identity
  or size differences, transfer phase or endpoint-class violations, and a
  repeated finalized task identity.
* `validate_binding` (`cuda.rs:1633-1646`) constructs `ArtifactMismatch` for
  a context device or compute capability that differs from its deployment.
  `require_enforced_quota` (`cuda.rs:1655-1662`) constructs `ArenaMismatch`
  when the reservation mechanism is not an enforced quota.
* `realize_device` (`cuda.rs:1665-1904`) constructs `MissingDevice` for absent
  staging or init-image manifests, `ArenaMismatch` for an init image larger
  than pre-realized staging, `MissingArtifact` for a required image,
  `ArtifactMismatch` for a non-CUDA image, failed CUDA compatibility or cubin
  entry checks, a duplicate digest with different bytes, and
  `IntegerOverflow` for compute-capability conversion. `inspect_cubin` and
  driver calls use the `Kernel` and `Cuda` `From` conversions. The function
  also propagates `UnexpectedDevice`, `SubmissionQueueLimitExceeded`, and
  allocation errors from its callers.
* `invocation_sizes` (`cuda.rs:1906-1934`) constructs `MissingArtifact`.
  `fill_invocation` (`cuda.rs:1936-2006`) constructs `ResourceContention` for
  a too-small preallocated argument block, `Protocol` for operand or fault-flag
  ABI mismatch, and `IntegerOverflow` for device-pointer addition or argument
  indexing. `checked_arena` (`cuda.rs:2008-2041`) constructs `MissingDevice`,
  `ArenaMismatch`, `IntegerOverflow`, and `ValueMismatch` for a missing or
  differently owned arena, range arithmetic failure, an unrepresentable arena
  size, or a range outside the arena.
* `device_endpoints` (`cuda.rs:2043-2057`) constructs `UnsupportedTransfer` for
  any internal transfer without two resolved device endpoints. `take_event`
  (`cuda.rs:2059-2078`) constructs `MissingCompletion` and `CompletionBusy`.
  It replaces the slot with the new task before checking the previous owner,
  so the error is returned after that attempted claim and the enclosing submit
  path poisons CUDA.
* `validate_active` and `queue` (`cuda.rs:2080-2120`) construct
  `MissingQueue`, `MissingCompletion`, and `Protocol` for an absent slot or a
  completion not owned by the pending task. `finish_action`
  (`cuda.rs:2122-2160`) constructs `Protocol` for absent or wrongly sized
  metric and egress buffers.
* `grid_size`, `bytes_to_usize`, `u8_from_u32`, and `offset_to_usize`
  (`cuda.rs:2162-2186`, `2247-2251`) construct `IntegerOverflow` for grid
  arithmetic, host-size conversion, and ABI-width conversion.
* `destroy_devices` (`cuda.rs:2203-2245`) constructs `CudaContract` when a
  stream is still active during teardown and `ResourceContention` when a
  completion event is still checked out. Driver destruction and freeing errors
  become `Cuda`.

### HSA adapter: `hsa.rs`

The HSA adapter has the same `Result` boundary but keeps a separate poison
policy. Its direct construction sites are:

* `HsaBackend::bind_partition`, `HsaResources::validate_handoff`,
  `HsaPreparedResources::bind`, `bind_candidate`, `validate_handoff`, and
  `Backend::bind_resources` (`hsa.rs:329-356`, `1114-1157`, `1250-1378`,
  `1381-1420`) construct `BackendState` for one-shot bind and handoff state,
  and `ArenaMismatch` for differing init-image contracts. `ensure_healthy`
  constructs `BackendPoisoned` when a prior HSA failure marked the resource.
* `HsaResources::realize` and `HsaPreparedResources::realize`
  (`hsa.rs:384-462`, `1159-1248`) construct `DuplicateDevice` and
  `MissingDevice`. Finalized `HsaResources::realize` also rejects an
  `UnexpectedDevice` and calls the shared capacity helper, which can return
  `SubmissionQueueLimitExceeded`. The pre-final
  `HsaPreparedResources::realize` additionally constructs
  `UnexpectedArtifact`, `DuplicateArtifact`, and `MissingArtifact` while it
  reconciles candidate runtime images. The device realization and artifact
  binding paths add the variants below.
* `allocate_arena` and `prepare_pending` (`hsa.rs:464-539`) construct
  `MissingDevice` and `Protocol` for absent task contracts, slot mismatches,
  duplicate preparation, or an absent pre-final pending token.
* `submit`, `poll_pending`, and `collect_exit` (`hsa.rs:541-655`) propagate
  validation and operation errors. `collect_exit` constructs
  `UnsupportedTransfer` for a non-device source and `Protocol` for an absent
  egress buffer or a size mismatch.
* `validate_work_contract` (`hsa.rs:658-697`) constructs `Protocol` for task,
  class, slot, admission, route, or lane-claim mismatches.
* `submit_admission` and `submit_internal_transfer` (`hsa.rs:713-807`)
  construct `Protocol` for immutable submission or staging mismatches and
  propagate `MissingQueue`, `MissingCompletion`, `ArenaMismatch`,
  `ValueMismatch`, `UnsupportedTransfer`, and `IntegerOverflow` from helpers.
  `submit_exit_transfer` (`hsa.rs:809-883`) constructs
  `UnsupportedTransfer` for a missing device source or wrong source/slot
  contract, and `Protocol` for absent egress staging.
* `submit_calculation` (`hsa.rs:885-989`) constructs `Protocol` for device or
  slot mismatch and dispatch geometry mismatch, `MissingArtifact`,
  `ArtifactMismatch` for a missing resource envelope or zero workgroup width,
  `MissingCompletion`, `MissingQueue`, and `ResourceContention` for AQL queue
  backpressure. `submit_metric` (`hsa.rs:991-1038`) constructs
  `ValueMismatch` for a non-four-byte value or wrong device, and `Protocol` for
  an absent or wrongly sized metric buffer.
* `finish_pending`, `rearm_pending`, `prepare_loop_pending`, and
  `recycle_pending` (`hsa.rs:1040-1104`) construct `Protocol` for completion
  ownership, token phase/state, repeated submission, and duplicate recycling;
  they propagate HSA driver errors through `Hsa`.
  `HsaPending::validate_ready` (`hsa.rs:1637-1649`) also constructs
  `Protocol` when submitted work differs from its pre-realized token.
* `validate_binding` (`hsa.rs:1692-1737`) constructs `ArtifactMismatch` for a
  non-CPU kernarg allocator, missing kernarg or fine-grained pools, or a
  session without the exact target. `require_enforced_quota`
  (`hsa.rs:1747-1754`) constructs `ArenaMismatch` for a reservation that is
  not scheduler-enforced.
* `candidate_task_device` (`hsa.rs:1777-1806`) constructs `ValueMismatch` for
  a metric without a device and `Protocol` for a transfer with no device
  endpoint. `prepare_pending_pool` (`hsa.rs:1757-1775`) constructs
  `MissingDevice` and `Protocol` for an absent device or repeated candidate
  task.
* `realize_device` (`hsa.rs:1808-2050`) constructs `MissingDevice`,
  `ArenaMismatch`, `MissingArtifact`, and `ArtifactMismatch` for staging,
  init-image, target, digest, inspected entry, metadata, or image consistency
  failures. A failed `executable.kernel` lookup constructs
  `HsaSymbolLookup`, preserving the logical ABI entry, the runtime descriptor
  symbol, and the original `recipe_hsa::Error`. `inspect_hsaco_bundle` and
  HSA driver calls become `Kernel` and `Hsa` through the `From` conversions.
* `bind_finalized_artifact_resources` and
  `hsa_artifact_resource_envelope` (`hsa.rs:2052-2116`) construct
  `MissingArtifact` and `ArtifactMismatch` when loaded ABI or resource
  envelopes differ from finalized identity, or when a dynamic-callstack
  private-byte bound is zero or cannot fit the AQL field.
* `task_contracts` and `transfer_work_class` (`hsa.rs:2118-2245`) construct
  `Protocol` for invalid calculation or metric phases, missing admission
  endpoints or manifests, admission identity or size differences, repeated
  task IDs, and invalid transfer phase/endpoint combinations.
* `kernarg_sizes` (`hsa.rs:2277-2304`) constructs `MissingArtifact` and
  `IntegerOverflow`. `fill_kernarg` (`hsa.rs:2306-2389`) constructs
  `ResourceContention`, `Protocol`, `ValueMismatch`, and `IntegerOverflow`
  for kernarg capacity, operand or fault-flag ABI mismatch, fault-flag dtype or
  size, pointer arithmetic, and kernarg offset/range conversion.
* `checked_arena` (`hsa.rs:2391-2416`) constructs `MissingDevice`,
  `ArenaMismatch`, `IntegerOverflow`, and `ValueMismatch` for arena ownership,
  range arithmetic, host-size conversion, and out-of-bounds resolved values.
  `device_endpoints` (`hsa.rs:2418-2432`) constructs `UnsupportedTransfer`
  for non-device internal endpoints.
* `claim_completion`, `release_completion`, `validate_active`, and
  `ensure_queue` (`hsa.rs:2434-2515`) construct `MissingCompletion`,
  `CompletionBusy`, `Protocol`, and `MissingQueue`. `claim_completion` also
  replaces a slot with the new task before returning `CompletionBusy`; unlike
  CUDA, this ordinary validation error is not in the HSA poison set.
* `finish_action` (`hsa.rs:2517-2562`) constructs `Protocol` for absent or
  wrongly sized metric and egress buffers. `destroy_devices`
  (`hsa.rs:2571-2613`) constructs `ResourceContention` when any completion is
  still active, then propagates HSA close and retirement errors as `Hsa`.
* `bytes_to_usize`, `u32_from_u64`, `hsa_grid_size`, `u16_from_u32`,
  `kernarg_size_error`, and `pointer_size_error` (`hsa.rs:2615-2665`)
  construct `IntegerOverflow` for host-size, AQL-width, grid, kernarg, pointer,
  and arena-range conversions.

### Shared and local propagation sites

`accounting::record` (`accounting.rs:5-8`) maps a failed
`PhysicalCallBatch::try_push` to `PhysicalAccountingOverflow`.
`cuda_ffi::ParameterBlock::set_value` (`cuda_ffi.rs:31-39`) constructs
`IntegerOverflow` when a launch argument index is outside the preallocated
parameter block.

`LocalBackend` maps every CUDA and HSA result with `map_err(LocalError::Native)`
(`local.rs:1684-2060`, `2827-3212`, and the cleanup paths at
`1182-1516`, `2080-2108`, `3249-3416`). `LocalError::Native` displays
`native partition failed: {error}` and exposes the native error as its
standard `source`. Candidate artifact partitioning constructs
`LocalError::Native(Error::UnexpectedArtifact)` and
`LocalError::Native(Error::MissingArtifact)` directly (`local.rs:2396-2452`).
The local physical-call helper maps the executor's
`PhysicalCallBatchOverflow` to `LocalError::PhysicalAccountingOverflow`
(`local.rs:3642-3647`), which is a different enum variant from the native
`Error` variant.

`CandidateRequestError::RuntimeArtifact` stores a native `Error` when
`validate_runtime_artifact` fails (`candidate.rs:493-549`) and exposes it as
its source. A fatal candidate realization carries the `LocalError` through
`CandidateFailure::Fatal`; `recipe-prepare` maps that failure to its driver
failure and native preparation error wrappers. No constructor in those outer
types changes the native variant.

### Direct constructor line index

The lists below are the direct `Error::Variant` occurrences in the current
source. Enum declarations, the `Display` match, `From` match arms, and HSA's
poll classifier are excluded. The two `local.rs` entries are native errors
explicitly wrapped in `LocalError::Native`; all other local errors are a
different enum.

#### `plan.rs`

```text
DuplicateArtifact: 159
MissingArtifact: 184
Protocol: 197, 685
MissingDevice: 201
ArtifactMismatch: 234, 263, 310, 360
ValueMismatch: 657
MissingQueue: 700
MissingCompletion: 709
UnexpectedArtifact: 725
```

#### `cuda.rs`

```text
ArenaMismatch: 83, 952, 968, 1140, 1173, 1348, 1658, 1722
BackendState: 316, 962, 1100, 1108, 1132, 1165, 1226
DuplicateDevice: 360, 1014
MissingDevice: 366, 427, 460, 949, 985, 1029, 1651, 1707, 1728, 2013
UnexpectedArtifact: 1048
DuplicateArtifact: 1052
MissingArtifact: 710, 1058, 1759, 1920
BackendPoisoned: 990
UnsupportedTransfer: 613, 749, 818, 825, 2051
ValueMismatch: 780, 2036
MissingCompletion: 716, 882, 2066, 2096
ArtifactMismatch: 1637, 1763, 1770, 1796, 1805
IntegerOverflow: 1781, 1786, 1963, 1986, 2024, 2031, 2166, 2172, 2182, 2250
ResourceContention: 1944, 2215
Protocol: 440, 448, 453, 468, 473, 488, 515, 521, 535, 544, 564, 626, 632,
  636, 655, 670, 702, 790, 835, 864, 887, 903, 916, 924, 938, 1427,
  1519, 1524, 1531, 1538, 1547, 1570, 1595, 1955, 1973, 1977, 2000,
  2100, 2126, 2133, 2145, 2151
MissingQueue: 2087, 2119
CompletionBusy: 2070
CudaContract: 2208
UnexpectedDevice: 2263
```

#### `hsa.rs`

```text
BackendState: 350, 1118, 1266, 1274, 1304, 1312, 1346, 1414
DuplicateDevice: 396, 1172
MissingDevice: 402, 468, 509, 1110, 1143, 1187, 1743, 1767, 1853, 1871, 2396
UnsupportedTransfer: 639, 817, 824, 2426
MissingArtifact: 904, 963, 1216, 1901, 2060, 2291
ArtifactMismatch: 910, 939, 1696, 1708, 1720, 1731, 1909, 1916, 1925,
  1958, 1975, 2065, 2077, 2099, 2104
MissingCompletion: 918, 969, 2441, 2462, 2507
MissingQueue: 926, 976, 2497
ResourceContention: 933, 2314, 2576
ValueMismatch: 1000, 1784, 2347, 2411
ArenaMismatch: 1125, 1319, 1354, 1555, 1750, 1865, 2399
BackendPoisoned: 1148
UnexpectedArtifact: 1206
DuplicateArtifact: 1210
UnexpectedDevice: 2676
HsaSymbolLookup: 1965
Protocol: 489, 497, 502, 515, 522, 530, 534, 549, 587, 646, 650, 660, 666,
  680, 689, 722, 738, 779, 837, 894, 952, 1011, 1015, 1057, 1070, 1078,
  1090, 1099, 1645, 1667, 1769, 1796, 2131, 2144, 2160, 2165, 2172,
  2179, 2188, 2213, 2239, 2325, 2341, 2378, 2463, 2511, 2521, 2525,
  2542, 2548
IntegerOverflow: 2332, 2355, 2362, 2365, 2371, 2407, 2618, 2627, 2635,
  2640, 2649, 2655, 2662
CompletionBusy: 2445
```

`hsa.rs:1490-1515` is intentionally absent from this index. It is the
exhaustive `PhysicalPollStatus::Failed` classifier, not a construction site;
the same applies to the `submission_error_requires_poison` match at
`hsa.rs:2488-2493`. The native helper and local wrapper sites are:

```text
error.rs:252       SubmissionQueueLimitExceeded
accounting.rs:7    PhysicalAccountingOverflow
cuda_ffi.rs:34     IntegerOverflow
local.rs:2436      LocalError::Native(Error::UnexpectedArtifact)
local.rs:2450      LocalError::Native(Error::MissingArtifact)
```

## Variant consequences

The following entries summarize the operational result of each variant after
the construction index has returned it.

### `DuplicateDevice`

The binding list contains the same `DeviceId` more than once. CUDA and HSA
reject it while realizing candidate or finalized resources, before the
device-resource map is complete. The operation returns immediately and no
duplicate owner is selected.

### `DuplicateArtifact`

The runtime artifact vector contains the same `ArtifactId` more than once.
Plan validation and pre-final candidate realization stop before loading or
binding the artifact set. It is not a driver duplicate-module error.

### `MissingArtifact`

A finalized calculation names an artifact for which no exact runtime image is
available, or a later lookup cannot find an artifact that was expected to be
loaded. In plan and candidate validation this prevents resource realization. In
CUDA and HSA submission or kernarg sizing it prevents the calculation from
being submitted. The local candidate wrapper preserves the same native error
when partitioned artifacts are incomplete.

### `UnexpectedArtifact`

An input runtime image is not required by the selected finalized bundle. Plan,
CUDA, and HSA candidate validation reject the complete set rather than silently
dropping the extra image. A local candidate partition reports it through
`LocalError::Native`.

### `ArtifactMismatch`

The runtime image exists, but its identity, digest, target, ABI, resource
envelope, or finalized calculation contract differs. The detail is owned so
computed values can be included. The error aborts validation or realization,
before a mismatched image is dispatched. A few HSA checks can run during
submission, but they still return the error without a substitute kernel.

### `MissingDevice`

A finalized device, value location, arena lookup, reservation, staging
manifest, or backend resource map has no corresponding native device. The
operation cannot identify an owner or address range and returns before the
dependent native call.

### `UnexpectedDevice`

A binding contains a device for which the finalized plan has no arena. CUDA and
HSA reject it while reconciling bindings with the plan. The extra binding is
not retained as an unplanned resource.

### `MissingQueue`

The finalized queue slot is absent, attached to another device, or no longer
present in the backend resource maps. It can occur during plan validation,
pending preparation, submission, and polling. The native operation is not
valid without its queue, so the error is returned to the executor.

### `MissingCompletion`

The finalized completion slot is absent, attached to another device, or no
longer present in the backend resource maps. It can occur during plan
validation, pending preparation, submission, polling, and token checks near
teardown. The native operation is not valid without its completion owner, so
the error is returned to the executor.

### `ResourceContention`

The required pre-realized resource cannot be used safely: a CUDA parameter
block is too small, an HSA AQL queue is backpressured, a kernarg block is too
small, or teardown finds an active completion. Submission or destruction stops
at that point. This is a bounded resource failure, not a request to allocate a
larger resource in the loop.

### `CompletionBusy`

The completion slot is already claimed by another task. Both CUDA and HSA
include the backend name, claimant, slot, and prior owner in the message. The
claim helper has already replaced the slot state with the attempted task when
it reports this condition. CUDA marks any submit error as poisoned; HSA does
not mark this ordinary validation error as poisoned.

### `ArenaMismatch`

The native arena or its admission contract does not match the finalized device,
staging, reservation, or value identity. This includes unrepresentable driver
memory counters and reservations without enforced quotas. The adapter refuses
to use the range or resource and returns before the copy or kernel launch.

### `ValueMismatch`

A resolved value has no valid native shape for the operation. Current sites
cover absent finalized metric locations, non-four-byte metrics, wrong metric
devices, an out-of-bounds arena range, and a fault flag that is not one aligned
device `I32` value. No value is clamped, resized, or relocated.

### `UnsupportedTransfer`

The transfer endpoints or route do not belong to the operation being executed.
CUDA and HSA use it for internal transfers without two device endpoints, for
unsupported same-context or exit routes, and for exit collection without a
device source. The transfer is rejected at the adapter boundary.

### `UnsupportedLoopContract`

There is no construction site for this variant in the current native-executor
source. HSA's `Backend::poll` match (`hsa.rs:1485-1516`) includes it so every
known native variant maps to `PhysicalPollStatus::Failed`. If a future native
path constructs it, its `Display` text states that the strict loop contract
cannot be upheld; the current code has no such path.

### `BackendState`

The state machine is being used in the wrong phase or more than once. Examples
include binding an already bound backend, using a candidate handoff without
finalized validation, rebinding finalized resources as a warm candidate, and
failing to recycle all warm pending tokens. The operation returns without a
fallback transition. The backend bind entrypoints replace their state with
`Bound` before matching the previous state, so a failed bind attempt must not
be retried through a second implementation.

### `BackendPoisoned`

An earlier operation marked CUDA or HSA resources unhealthy. `ensure_healthy`
is called by preparation, submission, polling, exit collection, and destroy
paths, so this error prevents further use of the poisoned resource. Current
teardown calls `ensure_healthy` first, therefore a poisoned backend can also
return this error during destruction; the remaining ordered teardown work is
tracked in `INTEGRATION_REQUIRED.md`.

### `SubmissionQueueLimitExceeded`

`ensure_submission_queue_capacity` (`error.rs:243-258`) compares the exact
requested queue count with the discovered `u32` maximum. CUDA calls it while
realizing resources with the binding's maximum, and HSA does the same with its
binding maximum. An over-limit plan is rejected before queue creation; the
helper does not truncate the count or create a second queue implementation.

### `IntegerOverflow`

The named native ABI field cannot be represented in the required host, pointer,
grid, AQL, or parameter-block width. Checked arithmetic and fallible integer
conversions construct this variant before an unsafe driver call. The `field`
string identifies the exact conversion, such as `CUDA grid dimension`, `HSA
kernarg byte range`, or `CUDA launch argument index`.

### `PhysicalAccountingOverflow`

The fixed `PhysicalCallBatch` cannot accept another physical record. Native
CUDA/HSA accounting returns this error from `accounting::record`; the local
composite maps the same underlying `PhysicalCallBatchOverflow` to its own
`LocalError` variant. The logical operation is not allowed to continue while
its physical accounting is incomplete.

### `Cuda`

`From<recipe_cuda::CudaError>` wraps a CUDA Driver operation failure. The
conversion is implicit at `?` sites for context queries, allocation and free,
stream and event operations, module and function operations, launch and copy
submission, polling, waiting, and teardown. A native driver polling failure
passes through `CudaResources::poison`, which marks CUDA unhealthy. Other
driver errors follow the enclosing method's poison policy.

### `CudaContract`

This variant has one constructor: `destroy_devices` returns it when a CUDA
stream still reports active during teardown (`cuda.rs:2203-2209`). It means the
driver resource contract was violated, rather than that a Driver API call
failed. Destruction stops before proceeding past that check.

### `HsaSymbolLookup`

During HSA device realization, HSACO inspection yields a logical ABI entry and
a runtime descriptor symbol. If `Executable::kernel` cannot resolve the
descriptor symbol, the adapter records both names and the original ROCr error.
The artifact is not inserted into the loaded-artifact map and realization
fails. This is distinct from `Hsa`, which wraps an ordinary ROCr/HSA call.

### `Hsa`

`From<recipe_hsa::Error>` wraps ROCr/HSA calls. It is produced implicitly by
`?` for session, allocation, queue, copy, dispatch, poll, and close operations;
`finish_submission_claim` and `poll_pending` also call `Error::from` explicitly.
HSA submission errors are classified by `submission_error_requires_poison`:
session-poisoned, poisoned deferred retirement, negative asynchronous signals,
and an existing `BackendPoisoned` error mark the backend poisoned. Ordinary
HSA validation or capacity errors do not.

### `Kernel`

`From<recipe_kernel::LoweringError>` wraps kernel artifact inspection failures.
The current native construction paths are `inspect_cubin` in CUDA realization
and `inspect_hsaco_bundle` in HSA realization. Both occur before the loop and
prevent loading or dispatching an artifact that could not be validated.

### `Protocol`

`Protocol` is the task-scoped invariant failure. It covers missing immutable
contracts, wrong work class or phase, pending-token state transitions,
admission and transfer endpoint mismatches, route or lane-claim differences,
operand and fault-flag ABI differences, completion ownership, metric and
egress storage shape, invalid transfer endpoint classes, repeated task IDs,
and other finalized-plan inconsistencies listed at each source site above.
The detail is static and identifies the violated invariant.

The error aborts the current backend operation. CUDA marks every submitter
error as poisoned, while HSA marks only the driver/session poison cases listed
above. During a poll, both native backends report `PhysicalPollStatus::Failed`
to their physical accounting path before returning the protocol error. The
executor then turns the error into a run failure for the corresponding backend
operation.

## Propagation and execution consequences

### Native backend boundary

`CudaBackend` and `HsaBackend` implement `recipe_executor::Backend` with
`type Error = crate::Error`. The trait methods are the real production
boundary:

1. `bind_resources`, `prepare_pending`, and `allocate_arena` return errors
   during preparation or init setup.
2. `submit` and `submit_loop_iteration` return validation, resource, driver,
   and accounting errors. They do not allocate a replacement resource.
3. `poll` returns `BackendPoll::Pending` or `Complete` on success. On any
   native error, CUDA records a failed physical poll marker; HSA maps every
   current `Error` variant, including `UnsupportedLoopContract`, to
   `PhysicalPollStatus::Failed` and records that marker.
4. `collect_exit`, `release_arena`, and `destroy_resources` return their native
   errors during exit and ordered teardown.

The backend accounting call is made before or after the domain call according
to the trait method. A failed `accounting::record` can therefore return
`PhysicalAccountingOverflow` before the native operation, or while recording
the completion status after it. The operation result is never hidden by a
retry or alternate implementation.

### Local composite boundary

`LocalBackend` owns host, CUDA, HSA, and staged-bridge partitions. Every CUDA
and HSA result is mapped to `LocalError::Native`, preserving the native error as
the source. The local `poll` method maps `LocalError::Native(_)` to
`PhysicalPollStatus::Failed` and returns the wrapper. Cleanup uses
`retain_first`, so it attempts bridge, HSA, CUDA, and host teardown in its
defined order while retaining the first error and dropping later errors.

### Candidate and preparation boundary

Candidate validation wraps native artifact validation failures in
`CandidateRequestError::RuntimeArtifact`, whose source is the native error.
Fatal realization errors travel as `CandidateFailure::Fatal` and are mapped by
`recipe-prepare` to its driver and native-preparation error types. These paths
run before final execution and therefore reject the candidate rather than
entering the loop with partial resources.

### Executor rendering boundary

`recipe_executor::executor::backend_value` receives each backend
`Result<T, Error>`. It first records the supplied `PhysicalCallBatch`, then
formats the native error with `write!(&mut message, "{error}")` into a fixed
`BackendMessage` of 96 bytes. Formatting is character-safe; if the message is
longer than the capacity, `BackendMessage::Display` appends `...` after the
retained prefix. The executor returns
`ExecutorError::Backend { operation, message }`, where `operation` is one of
`BindResources`, `PreparePending`, `AllocateArena`, `Submit`, `Poll`,
`CollectExit`, `ReleaseArena`, or `DestroyResources`.

`RunFailure` retains the primary executor error and may retain a second
`cleanup_error` if release or destruction also fails. The native `Error` value
is rendered into the fixed message, not exposed as a nested `source` by
`ExecutorError`. Training and inference execution wrappers add their own
context around the executor message; they do not change the native variant.

## Completeness notes

* All 26 variants declared in `error.rs:9-97` are covered above.
* `UnsupportedLoopContract` has no constructor in the current native-executor
  source. It appears only in HSA's exhaustive poll-status match.
* `Cuda`, `Hsa`, and `Kernel` have no direct `Error::Variant` constructor at
  call sites beyond their `From` implementations. Their construction is the
  `?` conversion of the corresponding dependency errors described above.
* `PhysicalAccountingOverflow` has one native constructor in
  `accounting.rs` and one separate local mapping in `local.rs`; the two enum
  types must not be conflated.
