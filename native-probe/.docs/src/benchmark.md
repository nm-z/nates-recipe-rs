---
file: native-probe/src/benchmark.rs
crate: recipe-native-probe
package_version: 0.1.0
role: native_gpu_benchmark_helpers
intent: >
  Provide one bounded timing and accounting path for native CUDA and ROCr/HSA
  GPU transfer measurements, plus the Recipe-owned dependent f32 FMA workload
  used for calculation-rate measurements.
authority:
  - native-probe/src/benchmark.rs
  - native-probe/src/native.rs
  - native-probe/src/cuda.rs
  - native-probe/src/hsa.rs
  - probe/src/engine.rs
  - probe/src/model.rs
  - probe/src/codec.rs
---

# `native-probe/src/benchmark.rs`

## Scope and boundary

`benchmark.rs` is a private module of `recipe-native-probe`. It does not open a
driver, allocate a native object, discover a device, build a profile, or print
output. It supplies the common helpers used by both native backends:

* [`time_bounded`](../../src/benchmark.rs#L20) runs a counted,
  deadline-aware sequence of already-submitted-and-completed operations.
* [`transfer_rate`](../../src/benchmark.rs#L52) and
  [`calculation_rate`](../../src/benchmark.rs#L61) convert one
  `TimedWork` aggregate into a nonzero typed rate.
* [`plan_bytes`](../../src/benchmark.rs#L191),
  [`compute_buffer_bytes`](../../src/benchmark.rs#L84), and
  [`capacity`](../../src/benchmark.rs#L196) validate native sizes.
* [`fma_template`](../../src/benchmark.rs#L95) constructs the
  backend-neutral calculation kernel, while [`fill_input`](../../src/benchmark.rs#L163)
  and [`verify_compute_output`](../../src/benchmark.rs#L171)
  provide its deterministic input and postcondition.
* [`measured`](../../src/benchmark.rs#L187) marks every value
  returned by a native benchmark as `PropertyProvenance::Measured`.

The module therefore owns the common measurement semantics, not the native
submission mechanics. CUDA and HSA each call these helpers from their own
allocation, submission, completion, artifact, and verification paths. Host
RAM, disk, and peer benchmarks use separate implementations in `recipe-probe`.

The production call path is:

```text
recipe probe
  -> src/cli.rs::run_probe
  -> ProbeEngine::load_or_probe_and_store
  -> ProbeEngine::probe
  -> GpuBenchmarkIo::benchmark_gpu
  -> NativeGpuProbe::benchmark_gpu
  -> exact CUDA or HSA backend owner
  -> backend transfer and FMA operations
  -> benchmark.rs accounting helpers
  -> GpuMeasurement
  -> MeasuredProfile, topology, discovery, and cache/receipt
```

`run_probe` supplies the same `NativeGpuProbe` as both the `GpuDiscovery` and
`GpuBenchmarkIo` implementation. It passes an empty peer list, so the command
does not execute network sessions. A benchmark failure propagates as
`ProbeError::Benchmark` (or a lower-level discovery or I/O error) and aborts
profile construction; no partial GPU measurement is accepted.

## Plan model and source of bounds

`recipe-probe::BoundedBenchmarkPlan` has exactly three fields:

| field | type | meaning |
| --- | --- | --- |
| `buffer_bytes` | `ByteCount` | Byte span used by transfer allocations and submissions. |
| `iterations` | `u32` | Maximum number of completed submissions for one measured direction or calculation. |
| `maximum_duration` | `Duration` | Deadline supplied to each timed operation. |

`BoundedBenchmarkPlan::is_bounded` is a structural predicate. It returns true
only when all three values are nonzero: `buffer_bytes.get() != 0`,
`iterations != 0`, and `maximum_duration` is not zero. It does not enforce a
global byte ceiling, a standard iteration count, or a two-second duration.
Those policy values are applied by `ProbeEngine` when it derives plans from the
seed.

### Plan derivation in `ProbeEngine`

`ProbeEngine::BenchmarkPlans::from_seed` derives one plan for each benchmark
class from theoretical seed estimates. The estimates are divisors for the
first bounded pass, not measured values:

| plan | suggested bytes |
| --- | --- |
| RAM | `seed.estimates.ram_capacity / 1024` |
| storage | `seed.estimates.disk_capacity / 16_384` |
| GPU | `seed.estimates.gpu_memory_capacity / 1024` |
| network | `seed.estimates.ethernet_rate / 8` |

`bounded_plan` then clamps each suggestion inclusively to 4 KiB through 64 MiB,
sets `iterations` to 8, and sets `maximum_duration` to 2 seconds. The policy is
in [`probe/src/engine.rs`](../../../probe/src/engine.rs#L26-L29) and
[`probe/src/engine.rs`](../../../probe/src/engine.rs#L249-L275). With the
checked-in `topology/contract.toml`, integer division yields 46,875,000 bytes
for RAM, 61,035,156 bytes for storage, 11,718,750 bytes for GPU, and
15,625,000 bytes for network. All four values are already inside the 4 KiB to
64 MiB interval, so this seed does not trigger either clamp. The code still
derives each value from the supplied seed, and another valid seed can exercise
either clamp.

The GPU helper does not reapply the 64 MiB plan clamp. A caller that constructs
a plan directly can pass any plan whose three fields are nonzero and whose
`ByteCount` fits the native allocation and `usize`; `NativeGpuProbe` checks
only `is_bounded`, and each backend checks capacity and conversion limits. The
standard CLI path always uses the engine's 4 KiB to 64 MiB plans.

The public `NativeGpuProbe::benchmark_gpu` guard runs before backend ownership
rediscovery and therefore rejects an unbounded plan without opening a driver or
ROCr runtime. The private backend methods reach `time_bounded` only after their
context/session and transfer allocations are prepared, so an internal direct
call with an invalid plan can perform setup before the helper reports the
invalid plan. Production callers use the public guard.

### Profile metadata

After all measurements complete, `BenchmarkPlans::metadata` records the seed
schema and all four plans in `recipe_probe::BenchmarkMetadata`. The profile
codec encodes each plan as three scalar values, in this order:

```text
benchmarks.seed_schema
benchmarks.ram.{buffer_bytes, iterations, maximum_duration_ns}
benchmarks.storage.{buffer_bytes, iterations, maximum_duration_ns}
benchmarks.gpu.{buffer_bytes, iterations, maximum_duration_ns}
benchmarks.network.{buffer_bytes, iterations, maximum_duration_ns}
```

Decode reconstructs `Duration` from nanoseconds. Profile validation requires
the seed schema to equal the seed contract schema, every plan to satisfy
`is_bounded`, and each duration to fit the canonical `u64` nanosecond form. It
does not reconstruct the seed divisors or require the engine's 8-iteration and
2-second values, because the metadata is evidence of the plan actually used.

## `TimedWork` and rate accounting

`TimedWork` is the only statistics record produced by this module:

```text
TimedWork {
    iterations: u32,
    elapsed: Duration,
}
```

It contains a completed-operation count and one wall-clock duration. It does
not retain per-sample durations, minimum/maximum values, variance, queue
occupancy, or a distribution. Consequently, native GPU rates are aggregate
throughput values, not a statistical summary of multiple independent samples.
`TimedWork` itself is not serialized in `MeasuredProfile`; only the plan
metadata and the resulting typed rates survive profile publication.

The profile model does have a separate `DirectionalBenchmarkEvidence` record
for explicitly established peer sessions. That record carries sample count,
minimum, maximum, mean, and variance nanoseconds for outbound and inbound
network directions. `recipe probe` passes `peers = []`, and CUDA/HSA never
construct that evidence. It must not be confused with the GPU `TimedWork`
record or treated as an additional native GPU output.

### `time_bounded`

`time_bounded(plan, submit_and_complete)` performs this exact sequence:

1. Reject the plan with `ProbeError::Benchmark("GPU benchmark plan is not bounded")`
   unless `plan.is_bounded()` is true.
2. Capture one `Instant` before any submission and initialize an iteration
   count of zero.
3. While the count is below `plan.iterations`, compute
   `remaining = plan.maximum_duration.saturating_sub(start.elapsed())`.
4. Stop the loop when `remaining` is zero. Otherwise call the callback with
   that remaining duration. The callback must submit one native operation and
   drive it to terminal completion before returning success.
5. Increment the count only after the callback returns `Ok(())`.
6. Capture `start.elapsed()` once more. If no callback completed or the elapsed
   duration is zero, return
   `ProbeError::Benchmark("bounded GPU benchmark completed no timed work")`.
   Otherwise return `TimedWork`.

The helper uses `saturating_sub`, so the remaining timeout never underflows.
The elapsed duration includes each callback's submission and completion polling
and the helper's backoff sleeps. Native allocation, artifact realization,
module/session setup, and the post-timing verification copy occur outside the
timed region.

Each transfer direction and the calculation receives its own independent
`time_bounded` invocation. The two-second engine value is therefore a per-metric
deadline, not a single wall-clock budget for the complete GPU benchmark. The
three timed copies, the timed FMA dispatches, and their separate verification
copies can each consume their own completion timeout.
The callback's own completion implementation is responsible for treating the
remaining duration as a deadline. If a callback returns an error, that error is
returned immediately and no `TimedWork` is produced. A successful operation
that completes just after the nominal deadline can be counted if the callback
observes completion before its own timeout path reports an error; the helper
itself performs no post-callback deadline comparison.

### `rate_per_second`

Both public accounting helpers use the same private integer calculation:

```text
nanos = elapsed.as_nanos()
rate  = (work * 1_000_000_000) / nanos
```

The multiplication and division are `u128` arithmetic, and division truncates
toward zero. A zero nanosecond duration is rejected before the division. The
scaled numerator must fit `u128`; the resulting value must fit `u64`; and the
typed constructor must accept the value as nonzero.

`transfer_rate(bytes_per_iteration, timed)` first computes
`bytes_per_iteration * timed.iterations` with checked `u128` multiplication,
then calls `rate_per_second` and constructs `BytesPerSecond`. Its failure
messages distinguish work-counter overflow, zero time or timer failure, rate
numerator overflow, `u64` conversion failure, and an invalid zero rate.

`calculation_rate(flops_per_iteration, timed)` is identical except that the
work counter is `flops_per_iteration.get() * timed.iterations` and the result is
`FlopsPerSecond`. `FlopCount` itself is an exact `u64` count. The kernel lowerer
counts each FMA as two FLOPs, so the FMA workload's work is
`2 * chain_length * element_count` when that product is representable.

## Size, capacity, and workload helpers

### Transfer bytes: `plan_bytes`

`plan_bytes` converts `plan.buffer_bytes.get()` to `usize`. It does not clamp,
align, or check device capacity. A conversion failure returns
`ProbeError::Benchmark("GPU transfer buffer does not fit usize")`. CUDA and HSA
use this value for every host and device transfer allocation and submission.

### Calculation bytes: `compute_buffer_bytes`

Calculation uses a deliberately smaller and f32-aligned span:

```text
bytes   = min(plan.buffer_bytes.get(), MAXIMUM_COMPUTE_BYTES)
aligned = bytes - (bytes % DType::F32.byte_width())
```

`MAXIMUM_COMPUTE_BYTES` is 4 MiB. `DType::F32.byte_width()` is 4, so the
calculation span is floored to a multiple of four. A zero result returns
`ProbeError::Benchmark("GPU benchmark buffer cannot hold one f32")`; otherwise
the aligned value must fit `usize`, or the helper returns
`"GPU benchmark buffer does not fit usize"`.

This produces an intentional distinction whenever a plan is larger than 4 MiB:
transfers use the complete plan span, while the generated FMA kernel calculates
over at most 4 MiB per iteration. The two rates therefore have different work
numerators and are not interchangeable. The standard checked-in seed produces
an 11,718,750-byte GPU plan, so that path is capped to 4 MiB for calculation.
A caller supplying a smaller direct plan can avoid the cap; the helper's cap
remains an independent upper bound.

### Capacity: `capacity`

`capacity(bytes)` rejects zero with
`ProbeError::Benchmark("native backend reported zero GPU capacity")` and wraps
any positive `u64` as `ByteCount`. It does not prove that the plan fits. The
CUDA and HSA callers perform the plan-versus-capacity comparison before
allocation, then use this helper for the measured capacity field.

### Deterministic input: `fill_input`

`fill_input` walks `bytes.chunks_exact_mut(4)`. For element index `i`, it uses

```text
mantissa = (i & 0x3ff) as u32
bits     = 0x3f00_0000 | mantissa
value    = f32::from_bits(bits)
```

The native-endian bytes of that f32 are copied into the four-byte chunk. The
masked mantissa keeps the input finite and bounded while varying the payload;
the low-level helper intentionally ignores a trailing one-to-three-byte
remainder. Standard engine plans are not required by `fill_input` to be f32
aligned, although the calculation path aligns its own span before constructing
the kernel.

### Calculation postcondition: `verify_compute_output`

Verification first requires equal input and output lengths and at least four
bytes. It decodes only the first f32 in each buffer. The first output must be
finite and its bit pattern must differ from the first input bit pattern. Any
failure returns
`ProbeError::Benchmark("FLOP benchmark verification buffers have invalid lengths")`
or
`ProbeError::Benchmark("Recipe-owned FLOP kernel did not produce a finite changed value")`.

The check does not scan every element, compare every output byte, or independently
recompute the FMA chain. Transfer verification remains a separate full-slice
equality check in each native backend.

## Recipe-owned FMA template

`fma_template(bytes, chain_length)` builds the exact `KernelTemplate` consumed
by both lowerers:

| template part | value |
| --- | --- |
| index space | one dimension, `bytes / 4` `F32` elements |
| input | kernel input ID 1, f32, contiguous access |
| output | kernel output ID 1, f32, the same contiguous span |
| scalar input | value ID 1, f32 |
| constants | value ID 2 = `1.000_976_6_f32` bits, value ID 3 = `0.000_122_070_31_f32` bits |
| instruction chain | `chain_length` dependent `Fma` instructions, IDs beginning at 4 |
| scalar output | the final chain value |
| alias rule | input 1 and output 1 are `AliasPermission::Forbidden` |
| template ID | `0x0050_524f_4245` |

Each instruction is f32 `Fma(previous, multiplier, addend)`. The first uses
the loaded input, and each subsequent instruction consumes the prior result, so
the chain cannot be reduced to independent operations. `chain_length == 0`
returns
`ProbeError::Benchmark("FLOP benchmark FMA chain cannot be empty")`.

The index-space and contiguous-access constructors can fail for an invalid
element count, an address calculation overflow, or an invalid access; those
errors are wrapped as `ProbeError::Benchmark` with the corresponding Recipe
validation detail. `lower_elementwise` then performs the complete
`KernelTemplate::validate` check, including scalar arity and types, contiguous
access bounds, writable output injectivity, and the complete alias matrix.

The lowerer computes work by summing `ScalarOpcode::flops()` and multiplying by
the index-space element count. FMA contributes two FLOPs per element. The
resulting `FlopCount` is what `calculation_rate` uses, so the reported rate is
the work of the exact emitted kernel, not a seed estimate.

### Native configuration carried into the workload

The CLI's `native_config` supplies PTX ISA 74, HSA code-object version 6, the
release label `auto-pinned-local-tools-and-benchmark-v3`, an absolute private
scratch parent, and a dependent FMA chain length of 64. The public config types
keep these values explicit: `CudaProbeConfig::ptx_isa`,
`HsaProbeConfig::code_object_version`, and
`KernelBuildConfig::fma_chain_length`. `NativeGpuProbe::new` rejects a zero
chain and a nonabsolute scratch parent; `HsaBackend::new` independently rejects
a zero code-object version.

The descriptor's toolchain identity includes the backend name, release label,
target configuration, FMA-chain length, the build-time native-probe source
digest, and the exact pinned verifier/code-generation/linker/assembler paths
and digests. The build script's source digest covers the native-probe and all
linked Recipe kernel, CUDA, HSA, probe, executor, and scheduling Rust sources,
their manifests, the lockfile, relevant target environment, and the rustc
identity. A change to the benchmark implementation therefore changes the
identity used to validate later preparation, rather than silently reusing an
old rate for a different workload.

### Probe artifact realization is temporary and inspected

`ArtifactBuilder::new` verifies the pinned LLVM verifier and code generator,
plus any configured linker and PTX assembler. `build(BuildPhase::Realize, ...)`
requires the lowered target to equal the requested target, creates a unique
0700 workspace below the private scratch parent, writes `kernel.ll` as a
create-new mode-0600 file, and runs the pinned verifier with a cleared
environment. CUDA then runs LLVM NVPTX code generation and the pinned `ptxas`
for the exact SM/PTX target. HSA runs LLVM AMDGPU code generation and the pinned
ELF linker for the exact processor, features, and code-object version.

The resulting bytes are read from regular non-symlink files and structurally
inspected before being returned in memory. CUDA inspection checks a recognized
CUDA ELF ABI, the expected SM encoding, a defined global function with the
requested entry, and nonempty executable code. HSA inspection checks AMDGPU-HSA
ELF identity, code-object version, target metadata, kernel symbols and
descriptors, kernarg alignment and size, workgroup capacity, and the explicit
three-argument ABI. The per-build workspace is removed on return, whether
build succeeds or fails. Native-probe does not export the cubin or HSACO used
for a measurement, and a builder variant mismatch is a benchmark failure.

## CUDA caller

The CUDA implementation is in
[`native-probe/src/cuda.rs`](../../src/cuda.rs). `CudaBackend`
implements the private `Backend` trait, and `NativeGpuProbe` selects it only
after an exact descriptor rediscovery.

### Reopen and allocation

`benchmark_device` calls `benchmark_open`, which requires the NVIDIA PCI
accelerator to still exist, selects the pinned CUDA Driver library, loads it,
performs exhaustive discovery, and finds exactly one `DeviceInfo` whose full
descriptor equals the expected measured descriptor. A disappeared backend,
changed identity, or duplicate match is a benchmark error. The method then:

1. Creates a CUDA context with `SchedulingPolicy::Yield`.
2. Creates one nonblocking stream.
3. Converts the plan buffer to `usize` with `plan_bytes` and checks it against
   `info.total_memory_bytes` before allocation.
4. Allocates two pinned host buffers and two device buffers of the full transfer
   size.
5. Fills the host source with `fill_input`.

The capacity comparison is per device-buffer span. Two device allocations and
the pinned host allocations still have to succeed together, so a direct plan
that fits one `total_memory_bytes` comparison can fail later at aggregate native
allocation. The allocator error is retained as a benchmark failure.

Every CUDA operation is mapped through `cuda_benchmark_error`, which prefixes
the native error with `CUDA native operation:`. Kernel lowering and artifact
construction are mapped through `kernel_benchmark_error`, which prefixes the
Recipe artifact error with `Recipe-owned GPU artifact:`.

### Transfer measurements

The three timed directions use the same full `bytes` value and the same plan:

| direction | native operation | completion and verification |
| --- | --- | --- |
| host to device | one `stream.copy_h2d` from pinned source to `device_source` | `complete_cuda` drives the event to completion |
| device to host | one `stream.copy_d2h` from `device_source` to pinned destination | `complete_cuda`, then full host source/destination equality |
| device to device | one `stream.copy_d2d` from `device_source` to `device_destination` | `complete_cuda`, a verification download, then full equality |

Each operation is submitted once per `time_bounded` iteration. The returned
`TimedWork` is converted with `transfer_rate(bytes, timed)`. The resulting
`GpuMeasurement` uses the three typed rates as `host_to_device_rate`,
`device_to_host_rate`, and `memory_rate` respectively.

The verification download after the device-to-device timing is not added to
the measured D2D `TimedWork`; it is a separate correctness operation bounded by
`plan.maximum_duration`. Likewise, the D2H transfer used to verify FMA output
is not included in the FMA timed work.

### CUDA calculation measurement

`benchmark_calculation` uses `compute_buffer_bytes`, builds the FMA template,
and computes `elements = bytes / size_of::<f32>()`. It chooses a workgroup size
with `cuda_workgroup_lanes`: the maximum is the lesser of the discovered CUDA
maximum threads per block and the element count, converted to `u32`; the
selected size is the greatest power of two no larger than that maximum. A zero
limit or zero element count fails. The element count must also fit the grid
calculation and the resulting grid dimension must fit `u32`.

The template is lowered for the exact discovered SM major/minor and configured
PTX ISA, with entry symbol `recipe_probe_fma_f32` and the selected workgroup
size. `ArtifactBuilder` realizes a `BuiltArtifact::Cubin` in the configured
scratch parent. The result must be a cubin and its inspected entry symbol must
remain `recipe_probe_fma_f32`; otherwise the benchmark fails. CUDA then loads
the cubin, resolves the same function, and launches:

```text
grid.x  = ceil(elements / workgroup_lanes)
grid.y  = 1
grid.z  = 1
block.x = workgroup_lanes
block.y = 1
block.z = 1
```

The shared lowerer accepts workgroup sizes from 1 through 1024 lanes. The
CUDA selector does not silently lower a discovered block limit above 1024; a
larger selected power of two reaches `lower_elementwise` and fails its
`workgroup size must be between 1 and 1024 lanes` validation.

`NvidiaTarget::validate` also bounds SM major to 3 through 12, SM minor to 0
through 9, and PTX ISA to 32 through 90. These are target-validation failures
before an artifact can be loaded; the benchmark does not substitute a nearby
SM, PTX version, or generic target.

The parameter storage is exactly two device pointers followed by an i64
element count. The input and output allocations are retained through terminal
completion. Each timed launch is completed by `complete_cuda`; after timing,
the output is downloaded for `verify_compute_output`, and only then is
`calculation_rate(lowered.work, timed)` called.

### CUDA completion and timeout cleanup

`complete_cuda` polls the event with a `CompletionPollBackoff` beginning at 50
microseconds and doubling up to 2 milliseconds. It stops successfully on
`CompletionStatus::Complete`, recycles the terminal event, and returns `Ok(())`.
Polling errors are benchmark errors.

If the normal deadline is reached while the event is still pending, CUDA has no
cancellation primitive. The helper therefore keeps the pending token and polls
until completion with a separate cleanup backoff beginning at 10 milliseconds
and capped at 100 milliseconds. Once the event finally completes it returns a
deadline error naming the operation and original timeout. It never releases the
borrowed buffers while the driver could still use them, submits replacement
work, or rapidly hammers the driver. A pending operation that never reaches a
terminal state can therefore keep the benchmark call waiting; that is an
intentional lifetime-safety consequence, not a retry policy.

## ROCr/HSA caller

The HSA implementation is in
[`native-probe/src/hsa.rs`](../../src/hsa.rs). It uses one retained
ROCr runtime per `HsaBackend` and the same benchmark helpers as CUDA.

### Reopen, agents, and allocations

`benchmark_device` calls `with_runtime`. Before initialization, no AMD PCI
accelerator returns `None`, which is converted to
`"ROCr/HSA backend disappeared before benchmark"`. After initialization,
disappearance of the PCI surface or a changed selected library is a discovery
error. The runtime is rediscovered and the expected descriptor must match
exactly one stable GPU UUID. Zero, multiple, changed, or vanished matches fail
with a benchmark error.

The adapter selects the exact HSA target, its GPU capacity, the GPU's NUMA node,
and a CPU agent. It prefers a CPU agent on the same NUMA node and otherwise
uses the first fallback CPU agent. No CPU agent is a hard error. GPU capacity
comes from the largest coarse-grained GPU-local global memory pool with runtime
allocation support, or from the reported AMD available-memory value when no
such pool exists. A zero capacity is rejected by `capacity`.

`exact_target` requires every reported ISA to expose an AMDGPU target. It accepts
one distinct non-generic target, or, when no specific target exists, one sole
generic target identity. Multiple distinct specific targets, ambiguous generic
targets, or no target are discovery errors. The selected target tail and
code-object version are used for both artifact identity and HSACO inspection.

The full transfer span from `plan_bytes` must fit that capacity. HSA then:

1. Allocates fine-grained CPU source and destination allocations.
2. Allocates coarse-grained GPU source and destination allocations.
3. Grants GPU access to both host allocations and CPU access to both GPU
   allocations.
4. Fills a host `Vec<u8>` with `fill_input` and copies it into the coherent
   host source allocation.

As with CUDA, the capacity comparison is for one requested span while two GPU
coarse allocations and two CPU allocations are live. ROCr allocation and
access checks remain authoritative for aggregate usage and can fail even after
the single-span comparison succeeds.

Allocation, access, session, and runtime failures are wrapped with the
operation-specific `hsa_benchmark_error` prefix and returned as benchmark
errors.

### HSA transfer measurements

HSA times the same three logical directions:

| direction | native operation | completion and verification |
| --- | --- | --- |
| host to device | `Session::copy_async(device_source, host_source, bytes)` | `complete_hsa` |
| device to host | `Session::copy_async(host_destination, device_source, bytes)` | `complete_hsa`, then read the coherent host allocation and compare all bytes |
| device to device | `Session::copy_async(device_destination, device_source, bytes)` | `complete_hsa`, then a separate verification download and full comparison |

The rates are computed with `transfer_rate` and stored as measured
`host_to_device_rate`, `device_to_host_rate`, and `memory_rate`. Verification
copies use `plan.maximum_duration` but are excluded from the measured timed
work.

### HSA calculation measurement

HSA caps and aligns the calculation span exactly as CUDA does. It converts the
element count to `u32`, selects the least maximum workgroup limit across all
reported ISAs and the element count, then chooses the greatest power of two no
larger than that limit. A missing ISA limit, zero limit, or `u16` conversion
failure is a benchmark error.

The template is lowered for the exact AMD target tail and configured HSA
code-object version with entry symbol `recipe_probe_fma_f32`. The offline
builder must return `BuiltArtifact::Hsaco`. ROCr loads it and resolves the
inspected kernel symbol. The kernel's runtime-reported kernarg segment size
must equal the inspected HSACO metadata. HSA allocates a kernarg block from a
CPU kernarg-capable pool, grants GPU access, and writes the explicit ABI:

```text
offset 0   input GPU pointer, u64
offset 8   output GPU pointer, u64
offset 16  element count, u64
```

The queue is single-producer with the discovered minimum packet count. Its
one-dimensional geometry uses the calculation element count and selected
workgroup lanes. Timed dispatches are completed by `complete_hsa`; a separate
download and `verify_compute_output` check the first output element. The queue,
executable, and kernarg are then closed before `calculation_rate` returns.

The shared lowerer applies the same 1 through 1024 lane validation to the HSA
workgroup choice. ROCr then validates the one-dimensional geometry against all
of the selected GPU's ISA limits; a geometry rejection is a benchmark error,
not a smaller fallback dispatch.

### HSA completion and timeout cleanup

`complete_hsa` polls the HSA signal with the same normal backoff as CUDA: 50
microseconds initially, doubling to 2 milliseconds. A `PollStatus::Complete`
returns success. A signal or session fault is returned immediately as a native
benchmark error.

HSA also has no safe cancellation path for a submitted operation. On a normal
deadline, the pending token remains live and is polled with the 10 millisecond
to 100 millisecond cleanup backoff until completion. It then returns an error
stating that the operation exceeded the original timeout. Keeping the token
alive retains its signal, allocations, queue, and executable until the already
published operation is terminal, so error unwinding cannot release live HSA
resources. A timed-out operation is not retried or replaced.

## Native dispatch and identity gate

`NativeGpuProbe` is the only `GpuBenchmarkIo` implementation used by the
production CLI. Its dispatch contract is in
[`native-probe/src/native.rs`](../../src/native.rs#L267):

1. Reject an unbounded plan before touching a backend.
2. Rediscover every configured backend and compare each candidate descriptor to
   the requested descriptor by full equality, not by ordinal, name, capacity,
   or rate similarity.
3. Require exactly one backend owner. Multiple exact owners fail with
   `multiple native backends claim exact GPU ...`; no owner fails with
   `GPU ... identity changed after discovery`.
4. Call that backend's `benchmark` method with the original plan.

`NativeGpuProbe::new` validates a nonzero configured FMA chain and an absolute
kernel scratch parent before constructing both CUDA and HSA adapters. The
CUDA-only and HSA-only diagnostic constructors deliberately set
`GpuInventory::exhaustive` to false and therefore cannot produce an accepted
measured profile. Normal probing uses `new`, discovers both backends, sorts
descriptors by stable key, and rejects duplicate keys.

Backend absence is only optional when the vendor PCI preflight finds no
accelerator. If hardware is present and its configured runtime is missing,
cannot load, or cannot enumerate, the error remains visible. A backend is never
silently treated as absent after an identity has been established.

## Measurement and profile consumers

`ProbeEngine::probe` measures every discovered GPU with the one GPU plan and
requires all five `GpuMeasurement` properties to have measured provenance:

| `GpuMeasurement` field | profile/topology consumer |
| --- | --- |
| `capacity` | GPU-memory `Device.total_capacity` and `DiscoveredDevice.total_capacity` |
| `calculation_rate` | GPU `CalculationCapability.rate` |
| `memory_rate` | GPU transfer rate and the symmetric device-memory topology capability |
| `host_to_device_rate` | RAM to GPU directed-link bandwidth |
| `device_to_host_rate` | GPU to RAM directed-link bandwidth |

The engine's GPU validation rejects an estimated, overridden, or otherwise
unmeasured property with `ProbeError::MissingMeasurement`. It then computes
topology and discovery digests over the measured values, records stable origins
for each GPU key, validates scheduling properties, and constructs one
`MeasuredProfile`. The benchmark values are not replaced with seed estimates at
any later stage.

`build_topology` uses `memory_rate` for the GPU-memory device's generic transfer
rate and the directed host links use the two directional PCIe measurements.
`build_discovery` uses the same `memory_rate` and `calculation_rate`, while the
descriptor supplies submission queues, concurrency, subgroup/workgroup limits,
asynchronous submission, and transfer-overlap properties. The benchmark module
does not infer those capability fields.

In particular, the CUDA and HSA descriptors advertise one maximum inflight lane
for each host direction. The GPU benchmark submits one operation at a time on
one stream (CUDA) or one session operation at a time (HSA); it does not run a
concurrency sweep and does not derive a larger lane count from a high measured
rate. CUDA's `maximum_submission_queues` is the fixed
`CUDA_MAXIMUM_SUBMISSION_QUEUES` executor contract, currently 32, while HSA
uses the runtime's reported maximum queue count. Those are discovery
properties, not benchmark statistics.

`BenchmarkMetadata` and every measured property are serialized inside the
identity-keyed profile. The active-native receipt then pins the profile path,
profile digest, selected backend libraries, toolchain identities, PTX/HSA
settings, and FMA chain configuration. Preparation later reloads that exact
profile and reopens exact GPU origins; it does not rerun the benchmark or pick
a similar device. Codec validation additionally requires measured provenance on
every persisted topology and discovery capacity, rate, bandwidth, and inflight
property, not only the five fields in the transient `GpuMeasurement`.

## Outputs and observable process behavior

The native helper functions return typed values only. They write no log file,
temporary result file, standard output, or standard error directly. Scratch
artifacts used by the offline CUDA/HSACO builder belong to the configured native
scratch parent and are not exported as benchmark results.

On the `recipe probe` path, a successful fresh measurement is written to the
identity-derived profile path (or the explicitly supplied profile path), then
the active-native receipt is atomically installed. The CLI prints only these
summary lines:

```text
profile=ABSOLUTE_PROFILE_PATH
source=validated-cache|fresh-measurement
cache_identity=LOWERCASE_DIGEST
topology_identity=LOWERCASE_DIGEST
discovery_identity=LOWERCASE_DIGEST
machines=COUNT
devices=COUNT
directed_links=COUNT
```

The CLI does not print GPU rates, byte spans, iteration counts, elapsed times,
native library paths, or raw device properties. Those remain in the measured
profile and authenticated receipt. A validated cache hit returns the existing
profile without invoking `benchmark_gpu`; a fresh probe executes the complete
transfer and calculation sequence for every GPU before publication. The
`source` line is computed from `profile_path.exists()` before the cache call,
not from the cache result. In the normal successful cases, an existing path is
a validated cache hit and an absent path is a fresh measurement. An existing
but invalid or stale profile is returned as a cache error by
`load_or_probe_and_store`, so the command fails before printing these summary
lines rather than silently replacing that file.

The profile cache is a separate bounded output boundary. The codec rejects
encoded or decoded profiles over 256 MiB, truncated payloads, a wrong magic or
codec schema, a SHA-256 checksum mismatch, unconsumed payload bytes, invalid
benchmark metadata, or any profile that fails topology, discovery, origin, or
measured-provenance validation. The explicit cache path must be absolute, have
a canonical private parent, and refer to a regular non-symlink file owned by
the effective user. Fresh publication writes a mode-0600 sibling temporary,
synchronizes it, installs it without replacing a different existing profile,
and synchronizes the parent directory. These cache limits are independent of
the 64 MiB benchmark-plan ceiling and the 4 MiB calculation ceiling.

## Failure matrix

The following are the direct helper and caller failures that can prevent a
measurement. Every row is fail-closed: no rate or partial `GpuMeasurement` is
returned.

| boundary | observed condition | result |
| --- | --- | --- |
| plan admission | zero buffer, zero iterations, or zero duration | `GPU benchmark plan is not bounded` from `time_bounded`, or `native GPU benchmark received an unbounded plan` from `NativeGpuProbe` |
| timed work | no callback completed, zero elapsed time, callback/native poll error | benchmark error; no `TimedWork` |
| work accounting | checked work product or scaled numerator overflows | benchmark error naming the counter or numerator |
| rate conversion | elapsed nanoseconds is zero, result does not fit `u64`, or typed rate would be zero | benchmark error |
| transfer size | plan bytes do not fit `usize`, or backend capacity comparison fails | benchmark error before native allocation |
| compute size | aligned span cannot hold one f32 or does not fit `usize` | benchmark error |
| FMA template | zero chain, invalid element/index/access construction, or lowerer validation failure | benchmark error with Recipe validation detail |
| CUDA identity | PCI/runtime absent after hardware was found, descriptor missing or nonunique | discovery or benchmark error |
| HSA identity | runtime disappearance, changed library, zero or multiple exact UUID matches | discovery or benchmark error |
| native allocation | context/session, pinned/fine/coarse allocation, access grant, queue, or event failure | operation-specific benchmark error |
| transfer correctness | host/device or device/device full-slice comparison differs | `CUDA host/device transfer verification failed`, `CUDA device-memory transfer verification failed`, or the corresponding HSA messages |
| artifact realization | lowering/build fails, wrong cubin/HSACO variant, entry identity changes, or HSA kernarg metadata disagrees | Recipe artifact or benchmark error |
| workgroup/launch | zero limits, integer conversion overflow, CUDA grid overflow, missing HSA user queue, invalid geometry | benchmark error |
| calculation correctness | verification buffers have unequal/short lengths, or first output is nonfinite/bit-identical | `FLOP benchmark verification buffers have invalid lengths` or `Recipe-owned FLOP kernel did not produce a finite changed value` |
| completion timeout | operation remains pending at normal deadline | token is retained and cleanup-polled; after terminal completion, benchmark error names operation and timeout |
| profile assembly | any GPU property is not measured, topology/discovery validation fails, or profile metadata is not bounded/canonical | `MissingMeasurement` or `InvalidProfile`; profile is not stored |

Native errors retain their original operation context. There is no alternate
backend, synthetic measurement, retry, cancellation shim, or estimate fallback
after a valid hardware identity has been selected.

## Source map

| source | benchmark responsibility |
| --- | --- |
| [`native-probe/src/benchmark.rs`](../../src/benchmark.rs) | timing, integer rate accounting, byte/capacity checks, FMA template, deterministic input, output postcondition, measured provenance |
| [`native-probe/src/native.rs`](../../src/native.rs) | exhaustive GPU inventory, exact descriptor ownership, bounded-plan gate, backend dispatch |
| [`native-probe/src/cuda.rs`](../../src/cuda.rs) | CUDA reopen, allocations, H2D/D2H/D2D copies, cubin realization and launch, CUDA completion cleanup |
| [`native-probe/src/hsa.rs`](../../src/hsa.rs) | ROCr reopen, agent and memory selection, HSA copies, HSACO realization and dispatch, HSA completion cleanup |
| [`probe/src/engine.rs`](../../../probe/src/engine.rs) | seed-derived plans, measurement loops, measured-provenance validation, profile/topology/discovery assembly |
| [`probe/src/model.rs`](../../../probe/src/model.rs) | `BoundedBenchmarkPlan`, `GpuMeasurement`, `BenchmarkMetadata`, and measured-profile data model |
| [`probe/src/codec.rs`](../../../probe/src/codec.rs) | canonical plan encoding/decoding and profile benchmark/evidence validation |
| [`src/cli.rs`](../../../src/cli.rs) | `recipe probe` orchestration, cache/receipt publication, and summary stdout |
