# CUDA native probe

~~~toml
module = "native-probe/src/cuda.rs"
backend_type = "CudaBackend"
public_owner = "NativeGpuProbe"
driver_surface = "CUDA Driver API via recipe-cuda"
library_vendor = "NVIDIA"
pci_vendor_id = 0x10de
kernel_entry = "recipe_probe_fma_f32"
target_backend = "nvidia-cuda-driver"
target_abi = "elf64-cubin"
~~~

This document describes the CUDA half of recipe-native-probe. The backend is
private to the crate. Callers use NativeGpuProbe::new,
NativeGpuProbe::cuda_diagnostic, GpuDiscovery::discover_all, and
GpuBenchmarkIo::benchmark_gpu; preparation uses the exact CUDA bindings
returned by with_native_execution_bindings.

The implementation deliberately uses the CUDA Driver API, not the CUDA Runtime
API. recipe-cuda is Linux-only and requires a 64-bit process. Its dynamic
loader resolves Driver symbols with dlopen and dlsym; no CUDA header, runtime
library, or vendor math library is linked into the probe.

## Stable source anchors

The names below are the parseable implementation anchors used by the flow
tables in this document.

| anchor | source | role |
| --- | --- | --- |
| CudaBackend::new | native-probe/src/cuda.rs | copy the CUDA portion of NativeProbeConfig |
| CudaBackend::open | native-probe/src/cuda.rs | PCI preflight, library selection, Driver load, exhaustive discovery |
| CudaBackend::descriptor | native-probe/src/cuda.rs | convert one recipe_cuda::DeviceInfo into one GpuDescriptor |
| CudaBackend::matching_device | native-probe/src/cuda.rs | re-identify one device for a benchmark |
| CudaBackend::benchmark_open | native-probe/src/cuda.rs | reopen Driver state and select an exact descriptor |
| CudaBackend::benchmark_device | native-probe/src/cuda.rs | allocate buffers and measure transfers plus calculation |
| CudaBackend::benchmark_calculation | native-probe/src/cuda.rs | lower, build, inspect, launch, and verify the FMA kernel |
| CudaBackend::discover | native-probe/src/cuda.rs | Backend implementation for inventory discovery |
| CudaBackend::benchmark | native-probe/src/cuda.rs | Backend implementation for one bounded measurement |
| complete_cuda | native-probe/src/cuda.rs | poll an event and preserve resource lifetimes on timeout |
| parse_pci_bus_id | native-probe/src/cuda.rs | parse Driver BDF text for sysfs lookup and identity checking |
| NativeGpuProbe::discover_all | native-probe/src/native.rs | aggregate CUDA and HSA descriptors and require unique keys |
| NativeGpuProbe::benchmark_gpu | native-probe/src/native.rs | find the unique backend owner before dispatching a benchmark |
| realize_cuda | native-probe/src/bindings.rs | reopen measured CUDA devices and create borrowed execution bindings |
| cuda_spec | src/native_prepare.rs | turn a measured CUDA deployment into a Realize build specification |

## Configuration inputs

NativeProbeConfig is constructed by the CLI in src/cli.rs or supplied by a
library caller. CudaBackend::new copies, rather than borrows, the following
values, so later calls observe one immutable backend configuration.

| value | type | use in CUDA path | validity and source |
| --- | --- | --- | --- |
| cuda.library.candidates | Vec<PathBuf> | ordered candidates for libcuda | candidates must be absolute when inspected; missing entries mean no candidate |
| cuda.ptx_isa | u16 | NvidiaTarget.ptx_isa for descriptor identity and every probe kernel | NvidiaTarget::validate accepts 32 through 90 inclusive |
| host_memory_key | Label | every descriptor's host RAM origin | already nonempty; ProbeEngine later verifies it exists in host inventory |
| pci_sysfs_root | PathBuf | PCI vendor preflight, PCI surface digest, DRM connector count | must be absolute; normally /sys/bus/pci/devices |
| kernels.toolchain.verifier | PinnedTool | LLVM verifier invocation and toolchain identity | canonical regular executable plus SHA-256 digest |
| kernels.toolchain.llvm_codegen | PinnedTool | LLVM NVPTX code generation and toolchain identity | canonical regular executable plus SHA-256 digest |
| kernels.toolchain.elf_linker | Option<PinnedTool> | not invoked by CUDA, but checked by ArtifactBuilder::new when configured | optional; its digest is not part of the NVIDIA required-tool identity |
| kernels.toolchain.ptx_assembler | Option<PinnedTool> | pinned ptxas; required for NVIDIA descriptor identity and cubin build | absence is a discovery/build error once NVIDIA hardware is present |
| kernels.release | Label | human-readable release component of ToolchainIdentity | nonempty label paired with exact tool digests |
| kernels.scratch_parent | PathBuf | private per-build workspace parent | NativeGpuProbe::new requires absolute; ArtifactBuilder additionally requires a real private directory |
| kernels.fma_chain_length | u16 | dependent FMA count in the probe template and identity | NativeGpuProbe rejects zero |

The CLI defaults are observable configuration, not CUDA code defaults. Unless
an option overrides them, native_config uses these CUDA library candidates in
order:

~~~text
/usr/lib/x86_64-linux-gnu/libcuda.so.1
/usr/lib64/libcuda.so.1
/usr/lib/libcuda.so.1
/usr/local/nvidia/lib64/libcuda.so.1
~~~

recipe probe accepts repeated --cuda-driver PATH values, which replace the
default candidate list. It accepts one each of --llvm-opt, --llvm-llc, and
--ptxas. The default PTX ISA is 74; the default FMA chain is 64.
The active-native receipt records the selected canonical library and each tool
digest. Reopening a receipt rechecks path and digest equality before a new
NativeProbeConfig is made.

NativeProbeConfig also carries HsaProbeConfig for the sibling HSA backend.
CudaBackend never reads the HSA library or code-object version; normal
NativeGpuProbe::new constructs both backends, while cuda_diagnostic constructs
only this CUDA backend.

## Ownership and call graph

~~~text
NativeGpuProbe::new(config)
  -> validate_config(config)
  -> CudaBackend::new(config)

ProbeEngine::inspect
  -> NativeGpuProbe::discover_all
  -> CudaBackend::discover
  -> CudaBackend::open
  -> CudaBackend::descriptor for every Driver device

ProbeEngine::probe
  -> NativeGpuProbe::benchmark_gpu(descriptor, bounded_plan)
  -> discover each backend again to find the exact owner
  -> CudaBackend::benchmark
  -> CudaBackend::benchmark_device

with_native_execution_bindings
  -> probe.discover_all and profile.resolve_local_inventory
  -> realize_cuda
  -> recipe_native_executor::CudaBinding

with_native_preparation
  -> bindings plus src/native_prepare.rs::cuda_spec
  -> recipe_prepare::DeferredArtifactCompiler
  -> Realize cubin bundle for production stages
~~~

CudaBackend owns configuration only. A Driver, Context, stream, event, buffer,
module, and pending submission are created inside the operation that uses it.
Contexts in bindings are borrowed through a higher-ranked callback; they cannot
escape the preparation scope.

## Discovery

### CudaBackend::open

open is the single CUDA entry point used by both discover and benchmark_open.

| order | operation | result |
| --- | --- | --- |
| 1 | pci_accelerator_present(pci_sysfs_root, 0x10de) scans each PCI directory, reads vendor and class as hexadecimal, and accepts class major 0x03 or 0x12 | Ok(None) when no NVIDIA accelerator is present; any sysfs read or parse failure is ProbeError::Discovery or ProbeError::Io |
| 2 | selected_library(&library, "CUDA Driver") validates all existing candidates, canonicalizes them, de-duplicates canonical paths, and selects the first canonical file | Ok(None) when every candidate is missing; malformed, nonabsolute, unreadable, nonregular, or noncanonical-target candidates are hard errors |
| 3 | convert the selected path to UTF-8 | non-UTF-8 paths are a discovery error because recipe-cuda::Driver::load_from_path takes &str |
| 4 | Driver::load_from_path calls dlopen(path, RTLD_NOW | RTLD_LOCAL) and loads every required Driver symbol | open failures, invalid NUL paths, and missing required symbols are wrapped as ProbeError::Discovery("load CUDA Driver: ...") |
| 5 | Driver::discover initializes the Driver, reads version and device count, then reads every device and rejects duplicate UUIDs | any Driver call or malformed Driver value becomes ProbeError::Discovery("exhaustive CUDA discovery: ...") |

The absence rule is intentional: an installed CUDA library without matching
PCI hardware is ignored, but matching hardware without a configured usable
library is an error ("NVIDIA PCI accelerator is present but no configured CUDA
Driver library exists"). A loaded library that fails to initialize or enumerate
is never downgraded to backend absence.

recipe-cuda requires these Driver symbols before discovery can proceed:

~~~text
cuInit
cuDriverGetVersion
cuDeviceGetCount
cuDeviceGet
cuDeviceGetName
cuDeviceGetUuid
cuDeviceGetPCIBusId
cuDeviceTotalMem_v2
cuDeviceGetAttribute
cuCtxCreate_v2
cuCtxDestroy_v2
cuCtxPushCurrent_v2
cuCtxPopCurrent_v2
cuModuleLoadData
cuModuleUnload
cuModuleGetFunction
cuMemAlloc_v2
cuMemFree_v2
cuMemGetInfo_v2
cuMemHostAlloc
cuMemFreeHost
cuStreamCreate
cuStreamDestroy_v2
cuStreamQuery
cuEventCreate
cuEventRecord
cuEventQuery
cuEventDestroy_v2
cuMemcpyHtoDAsync_v2
cuMemcpyDtoHAsync_v2
cuMemcpyDtoDAsync_v2
cuLaunchKernel
~~~

These optional symbols are retained in Discovery.capabilities when present:
cuDeviceGetUuid_v2, cuModuleGetLoadingMode, cuGetErrorName, and
cuGetErrorString. UUID v2 is used when it succeeds, with a fallback to UUID
v1 only for CUDA_ERROR_NOT_SUPPORTED; the other optional symbols do not gate
probe discovery.

### Driver discovery payload

For each ordinal, recipe-cuda::Driver::discover records:

~~~text
ordinal
UUID (16 bytes)
trimmed, nonempty device name
compute capability major and minor
exact Driver PCI text dddd:bb:dd.f
total memory bytes
async engine count
concurrent-kernels Boolean (only 0 or 1)
warp size
maximum threads per block
maximum shared memory per block
PCI domain, bus, and device attributes
core and memory clock rates
global memory bus width
number of multiprocessors
~~~

Driver discovery rejects a negative version or count, negative attributes,
non-NUL-terminated or non-UTF-8 names and PCI text, an empty name, a Boolean
attribute other than 0 or 1, a duplicate UUID, and a device PCI string that is
not exactly dddd:bb:dd.f. The CUDA backend uses only the fields needed to
construct a descriptor or benchmark geometry; clocks, bus width, and
multiprocessor count remain in the discovery object but are not copied into
GpuDescriptor.

### PCI parsing and surface identity

parse_pci_bus_id repeats the Driver identity split as
domain:bus:device.function.

~~~text
domain, bus, device: hexadecimal u32 components
function: decimal u32 component, restricted to 0..=7
canonical sysfs BDF: "{domain:04x}:{bus:02x}:{device:02x}.{function}"
~~~

Missing separators, invalid hexadecimal components, a nondecimal function, or a
function greater than 7 return ProbeError::Discovery. descriptor then compares
the parsed domain, bus, and device to the three Driver attributes. A mismatch
is fatal even if the textual BDF looks valid. The function is carried by the
canonical BDF and checked by the syntax rule because the Driver attribute set
has no separate function field.

pci_surface reads the canonical BDF directory and creates three SHA-256
surface labels:

| label | bytes and identity inputs | missing-file policy |
| --- | --- | --- |
| driver | canonical driver symlink target, /proc/sys/kernel/osrelease, and driver/module/version | at least one readable file is required |
| firmware | revision, subsystem_vendor, subsystem_device, vbios_version | all may be absent; the count is part of the digest |
| pcie-link | current and maximum link speed and width plus numa_node, prefixed by the BDF | all may be absent; the count is part of the digest |

Each digest is domain-separated by recipe-native-probe-surface-v1, includes
path bytes and byte lengths, and is formatted as
<domain>:sha256=<lowercase hex>. Missing or permission-denied surface files
are skipped. Other filesystem errors fail discovery.

### CudaBackend::descriptor

The descriptor transformation is deterministic and has no fallback selector:

| field | value |
| --- | --- |
| key | cuda:<DeviceUuid>@<canonical sysfs BDF> |
| name | Driver device name |
| host_memory_key | NativeProbeConfig.host_memory_key |
| target.backend | nvidia-cuda-driver |
| target.architecture | sm_<compute capability major><minor> |
| target.abi | elf64-cubin |
| capacity_hint | nonzero Driver total_memory_bytes |
| driver | cuda-kernel-driver:<driver_version.raw>:<driver surface label> |
| runtime_abi | cuda-driver-api:<driver_version.raw>:<canonical library path and SHA-256> |
| firmware | PCI firmware surface label |
| link_identity | PCIe-link surface label |
| transport_kind | Pcie |
| duplex | Full |
| host_to_device_maximum_inflight | one transfer lane |
| device_to_host_maximum_inflight | one transfer lane |
| asynchronous_submission | true |
| maximum_submission_queues | CUDA_MAXIMUM_SUBMISSION_QUEUES, currently 32; bounded Recipe policy because Driver exposes no finite stream-count attribute |
| maximum_concurrent_tasks | one |
| subgroup_lanes | Driver warp size |
| maximum_workgroup_lanes | Driver maximum threads per block |
| maximum_shared_memory_per_workgroup | Driver maximum shared memory per block, in bytes |
| transfer_overlaps_calculation | async_engine_count != 0 && concurrent_kernels |

The target is validated before any descriptor is returned. NVIDIA target
validation accepts SM major 3 through 12, SM minor 0 through 9, and PTX ISA 32
through 90. Major or minor values that do not fit u8 or an invalid label fails
descriptor construction. The shared capacity helper rejects zero capacity as a
ProbeError::Benchmark, even when the error is propagated from discovery.
Transfer lane construction uses the literal nonzero value one. A zero warp or
maximum-threads attribute is retained in the descriptor and fails later when
the calculation benchmark selects workgroup lanes.

### Inventory consumer

NativeGpuProbe::discover_all calls every configured backend, propagates the
first error, sorts all descriptors by key, rejects adjacent duplicate keys,
and returns GpuInventory { exhaustive: self.exhaustive, devices }.

NativeGpuProbe::new sets exhaustive = true and includes CUDA plus HSA.
cuda_diagnostic includes only CUDA and sets exhaustive = false; it is useful
for a backend diagnostic but ProbeEngine::inspect rejects its inventory with
IncompleteGpuEnumeration. A normal profile therefore always uses new.

## Toolchain and target identity

### Probe identity

backend_toolchain_identity in native-probe/src/identity.rs builds the core
ToolchainIdentity used by every CUDA descriptor. Its digest is SHA-256 over
length-delimited fields in this order:

~~~text
recipe-native-probe-toolchain-and-benchmark-v2
backend = "nvidia-cuda"
KernelBuildConfig.release
target configuration = "<architecture>:ptx<ptx_isa>:dependent-f32-fma-chain-<length>"
RECIPE_NATIVE_PROBE_SOURCE_DIGEST
PinnedTool(path bytes, ArtifactDigest bytes) for verifier
PinnedTool(path bytes, ArtifactDigest bytes) for LLVM codegen
PinnedTool(path bytes, ArtifactDigest bytes) for pinned PTX assembler
~~~

The verifier and LLVM codegen are always required. The NVIDIA branch requires
ptx_assembler; missing it is the discovery error
"NVIDIA probing requires an explicitly pinned PTX assembler". The returned
identity has name recipe-owned-llvm-nvidia-cuda, the configured release as
version, and the computed digest.

ArtifactBuilder::new also verifies an optional configured ELF linker before the
benchmark starts, even though the NVIDIA build path never invokes it. The
NVIDIA identity deliberately hashes only the verifier, LLVM codegen, and
ptxas, because those are the tools required by the CUDA build.

native-probe/build.rs supplies RECIPE_NATIVE_PROBE_SOURCE_DIGEST. It hashes
the Rust sources of recipe-native-probe, recipe-core, recipe-executor,
recipe-host, recipe-kernel, recipe-language, recipe-native-executor,
recipe-planner, recipe-primitives, recipe-scheduler, recipe-cuda, recipe-hsa,
and recipe-probe, plus the relevant Cargo manifests, native-probe/build.rs,
compiler identity (rustc -Vv), and selected build environment variables.
Therefore the descriptor toolchain identity changes when probe logic, lowering,
the CUDA binding, the pinned tools, or the build environment changes.

The library identity is separate from the compiler identity:
cuda-driver-library:<canonical path>:sha256=<library digest>. It appears in
runtime_abi, while the Driver version and PCI driver surface appear in the
driver field.

### Preparation target specification

src/native_prepare.rs::cuda_spec consumes a measured descriptor and the
DeploymentIdentity produced while reopening its context.

~~~text
sm_major = deployment.target.major as u8
sm_minor = deployment.target.minor as u8
ptx_isa = config.cuda.ptx_isa
target = KernelTarget::Nvidia(NvidiaTarget { sm_major, sm_minor, ptx_isa })
~~~

The target must validate and must exactly agree with the descriptor's backend,
architecture, and ABI. Every symbol in recipe_cuda::REQUIRED_DRIVER_SYMBOLS
must be present in the deployment capabilities. The current Driver version is
used as both the CUDA artifact policy minimum and maximum, so a production
cubin is pinned to this exact Driver version.

The CUDA runtime policy records:

~~~text
zig_version = "not-used-recipe-rust-owned-ir"
llvm_version = "<release>;opt-sha256=<digest>;llc-sha256=<digest>"
ptx_isa_version = decimal config.cuda.ptx_isa
ptxas_version = "sha256=<ptxas digest>"
cuda_toolkit_version = "not-claimed-pinned-ptxas-only"
cubin_format = "elf64-cubin"
minimum_driver = deployment.driver_version
maximum_driver = Some(deployment.driver_version)
required_driver_symbols = all required Driver symbols
~~~

TargetBuildSpec carries this policy, the descriptor's core toolchain identity,
the validated ArtifactBuilder, and the configured scratch parent. Identical
measured target identities share one specification only when target, toolchain
identity, scratch parent, and runtime policy are all equal.
DeferredArtifactCompiler later lowers each production deferred stage and
invokes ArtifactBuilder::build_cubin_bundle(BuildPhase::Realize, ...). This is
the production target path; probe-time measurement uses the single-kernel path
described below.

## Benchmark contract

### Plan boundary

GpuBenchmarkIo::benchmark_gpu and CudaBackend::benchmark_device require a
BoundedBenchmarkPlan whose buffer_bytes, iterations, and maximum_duration are
all nonzero. NativeGpuProbe::benchmark_gpu rejects an unbounded plan before
backend ownership lookup. time_bounded repeats a submission until iterations
is reached or the plan duration expires. It rejects zero timed iterations and
zero elapsed duration.

The standard ProbeEngine plan is derived from the seed contract as follows:

~~~text
gpu.buffer_bytes = clamp(seed.estimates.gpu_memory_capacity / 1024,
                         4 KiB, 64 MiB)
gpu.iterations = 8
gpu.maximum_duration = 2 seconds
~~~

The seed only bounds work. Returned rates and capacity are marked
PropertyProvenance::Measured and come from completed native operations.

### Exact owner lookup

Before a benchmark, NativeGpuProbe::benchmark_gpu rediscoveries every
configured backend. It compares each candidate descriptor for exact structural
equality with the requested descriptor. More than one owner is an error, and
zero owners is GPU <key> identity changed after discovery. A CUDA owner then
calls CudaBackend::benchmark.

CudaBackend::benchmark_open repeats open and matching_device. If PCI hardware
or the Driver surface disappeared it returns CUDA backend disappeared before
benchmark; if no exact descriptor remains it returns CUDA GPU <key> disappeared
or changed identity. A duplicate exact match is CUDA descriptor <key> is not
unique.

### Transfer and memory sequence

benchmark_device creates one context with
ContextFlags::new(SchedulingPolicy::Yield), then one nonblocking stream. It
uses the full plan buffer size for all transfer allocations.

~~~text
plan_bytes = usize(plan.buffer_bytes)
check plan_bytes <= discovered total memory
allocate pinned host_source and pinned host_destination
fill_input(host_source)
allocate device_source and device_destination

H2D: host_source -> device_source, timed_bounded
D2H: device_source -> host_destination, timed_bounded
compare host_source and host_destination

D2D: device_source -> device_destination, timed_bounded
verification D2H: device_destination -> host_destination, one bounded timeout
compare host_source and host_destination

calculation: benchmark_calculation(...)
return capacity and four measured rates
~~~

Each timed submission creates a completion event, enqueues the Driver
operation, and passes the Pending token to complete_cuda. All ranges are
zero-offset ranges within same-context live allocations. The destination of
the first D2H is mutable pinned memory; the D2D destination is independently
allocated. Verification copies are not included in the corresponding timed
rate.

Transfer rates use
bytes_per_iteration * completed_iterations * 1_000_000_000 / elapsed_ns,
with checked work multiplication, checked numerator multiplication, a nonzero
elapsed duration, and a nonzero BytesPerSecond result. Overflow or a zero rate
is a benchmark error.

PinnedHostBuffer allocation zeroes its bytes before fill_input runs. fill_input
uses chunks_exact_mut(4), so a non-four-byte transfer plan leaves its final one
to three bytes at zero; the round-trip comparisons still cover those bytes.
The compute region is explicitly aligned down to a multiple of four.

### Recipe-owned FMA calculation

benchmark_calculation bounds compute memory independently from transfer memory.
compute_buffer_bytes clamps the plan buffer to 4 MiB, aligns down to the
four-byte f32 width, and rejects a result that cannot hold one f32.

fma_template(bytes, fma_chain_length) creates one one-dimensional contiguous
f32 input and output with forbidden input/output aliasing. Its scalar program
contains fma_chain_length dependent Fma instructions:

~~~text
input value id = 1
multiplier constant id = 2, f32 bits for 1.0009766
addend constant id = 3, f32 bits for 0.00012207031
result ids = 4 through (3 + chain length)
template id = 0x0050_524f_4245
output = final dependent result
~~~

The work count is two FLOPs per FMA per element. fill_input writes finite
native-endian f32 values whose low ten mantissa bits vary by element. Output
verification checks equal buffer lengths, at least one f32, a finite first
output value, and that the first output bit pattern differs from the first input
bit pattern. It intentionally does not compare every output element.

The workgroup width is the greatest power of two no larger than both the
Driver's maximum threads per block and the element count. Element counts are
clamped to u32::MAX for this selection. Zero maximum threads or zero elements
is an error. A width above the LLVM lowerer's 1024-lane limit is rejected by
lower_elementwise rather than silently reduced.

The target and compiler sequence is:

~~~text
target = KernelTarget::Nvidia(NvidiaTarget {
  sm_major = context.device.compute_capability.major as u8,
  sm_minor = context.device.compute_capability.minor as u8,
  ptx_isa = configured PTX ISA,
})
lower_elementwise(template, target, {
  entry_symbol = "recipe_probe_fma_f32",
  workgroup_lanes = selected power of two,
})
ArtifactBuilder::new(cloned pinned OfflineToolchain)
builder.build(BuildPhase::Realize, lowered, target, scratch_parent)
~~~

lower_elementwise validates the template, target, entry symbol, workgroup size,
generated LLVM audit, ABI, and checked FLOP count. The NVIDIA builder verifies
tool paths and digests, writes a private kernel.ll, runs the pinned LLVM
verifier, runs pinned LLVM NVPTX codegen with -march=nvptx64,
-mcpu=sm_<major><minor>, and -mattr=+ptx<isa>, then runs pinned ptxas with
-arch=sm_<major><minor>. It returns a BuiltArtifact::Cubin and an independent
structural inspect_cubin result. Only BuildPhase::Offline or BuildPhase::Realize
is accepted by the builder; the probe passes Realize.

The builder workspace is created below the configured scratch parent with mode
0700, and is removed when the build returns. The parent must be an existing
real directory with no group or other permissions. Tool invocation clears the
environment except for deterministic LC_ALL=C and SOURCE_DATE_EPOCH=0.

The probe requires the result to be BuiltArtifact::Cubin; a non-cubin result is
CUDA benchmark builder returned a non-cubin artifact. It also requires the
inspected entry symbol to remain exactly recipe_probe_fma_f32; otherwise the
error is CUDA benchmark cubin entry identity changed.

inspect_cubin requires a little-endian ELF64 CUDA image with a recognized
CUDA OS-ABI and CUDA ELF machine, the expected encoded SM, no unresolved global
symbols, a defined global function with the requested name, and a nonempty
executable .text.<entry> section. Module::load_cubin repeats the ELF magic
boundary and rejects PTX or fat binaries. The module function lookup must find
the same entry symbol.

The launch uses elements = lowered.abi.elements,
blocks = ceil(elements / workgroup_lanes), a one-dimensional grid, a
one-dimensional block, and zero dynamic shared memory. Grid arithmetic is
checked for overflow and u32 fit. The host-side parameter array is exactly:

~~~text
pointer to input device allocation
pointer to output device allocation
i64 element count
~~~

The launch keeps both device buffers borrowed until event completion. The
output is downloaded through the same verification path, checked by
verify_compute_output, and converted to FlopsPerSecond using the lowered FLOP
count and timed iterations.

### Measurement output

On success benchmark_device returns:

| GpuMeasurement field | source |
| --- | --- |
| capacity | discovered DeviceInfo.total_memory_bytes, validated nonzero |
| calculation_rate | lowered FMA FLOP count and completed kernel iterations |
| memory_rate | timed D2D transfer |
| host_to_device_rate | timed H2D transfer |
| device_to_host_rate | timed D2H transfer |

Every property is wrapped with measured(...). ProbeEngine rejects any
unmeasured property before topology or discovery profile construction. The
four GPU rates become the GPU memory and host link measurements in the measured
topology.

## Completion and lifetime invariant

complete_cuda polls the event until Complete or the operation's remaining
deadline expires. Normal polling starts at 50 microseconds and doubles up to
2 milliseconds. A completed event is recycled and dropped before returning.

CUDA has no cancellation operation. When the deadline is reached while a
submission is pending, complete_cuda retains the Pending token and waits with
a 10 millisecond to 100 millisecond capped cleanup backoff until the GPU
reports completion. It then returns <operation> exceeded its <timeout> deadline.
It never submits replacement work or drops live buffers while the Driver may
still access them. A Driver error during initial or cleanup polling is wrapped
as ProbeError::Benchmark("CUDA native operation: ...").

The recipe-cuda runtime enforces the same invariant in its types:

~~~text
Pending is must-use and owns its completion Event
Pending::poll -> Pending or Complete
Pending::recycle_event requires Complete
copy_h2d, copy_d2h, copy_d2d, and launch require same-context resources
all ranges are checked against allocation lengths
launch keepalive borrows every referenced DeviceBuffer
~~~

Context creation pops the newly created current context and checks the context
stack. Each operation temporarily pushes its context and checks the matching
pop. Context, stream, event, module, host allocation, and device allocation
drop paths release Driver resources; explicit close failures are surfaced when
the operation calls close, while drop cannot report an error.

## Binding and execution consumers

with_native_execution_bindings first calls discover_all and
MeasuredProfile::resolve_local_inventory. It partitions exact measured GPU
keys by target.backend. realize_cuda then:

1. calls CudaBackend::open again;
2. requires the reopened Driver surface when expected CUDA keys are nonempty;
3. creates a descriptor for every reopened device and rejects any key absent
   from the measured map;
4. rejects duplicate reopened keys;
5. derives DeploymentIdentity::from_discovery, which binds Driver version, UUID,
   compute capability, and Driver capabilities to the exact discovery;
6. creates a default CUDA context for each expected device;
7. requires every expected key to have been reopened and sorts realized devices
   by DeviceId.

Each realized context is borrowed by one
recipe_native_executor::CudaBinding with the measured DeviceId, deployment
identity, queue ceiling 32, and current enabled DRM connector count. The
binding is valid only inside the callback. Missing CUDA backend with expected
CUDA origins, an unknown reopened device, duplicate device, context creation
failure, deployment identity omission, or an unreopened expected key is a
ProbeError::Discovery binding failure.

The connector count is read by NativeGpuProbe::enabled_display_connectors before
realize_cuda constructs each binding. The GPU origin must end in a canonical
12-byte BDF of the form dddd:bb:dd.f. The helper checks the BDF directory,
returns zero when its drm directory is absent, and otherwise:

~~~text
collect real drm/card<decimal> directories
sort card names
for each card, collect real connector directories named <card>-<name>
sort connector paths
read each connector's enabled file
count exactly "enabled"; accept exactly "disabled"
~~~

Invalid UTF-8 entries, connector read errors, an unknown state, or a count
overflow are discovery failures. This count is runtime display state, not part
of the GPU descriptor identity.

The same deployment is consumed by src/native_prepare.rs::cuda_spec, which
checks the measured target and required Driver symbols before allowing a
production TargetBuildSpec. recipe_native_executor::CudaBackend later uses
the binding to load inspected cubins and enforce the finalized ABI, target,
digest, queue, transfer, and completion contracts. The probe does not create
execution arenas or submit user graph tasks.

## Failure matrix

The first matching row is the operation that emits the failure. All failures
are fail-closed; there is no alternate CUDA implementation or partial CUDA
inventory.

| stage | condition | externally visible error class or text |
| --- | --- | --- |
| config | FMA chain is zero | Discovery: native FLOP benchmark requires a nonzero FMA chain |
| config | scratch parent is relative | Discovery: kernel scratch parent ... is not absolute |
| PCI preflight | root is relative, unreadable, or a vendor/class field is malformed | Discovery or Io |
| PCI preflight | no NVIDIA accelerator class 0x03 or 0x12 | CUDA backend returns None, not an error |
| library selection | hardware exists and no candidate exists | NVIDIA PCI accelerator is present but no configured CUDA Driver library exists |
| library selection | candidate is relative, nonregular, unreadable, or canonical target is not a regular file | Discovery or Io |
| Driver load | path is non-UTF-8, dlopen fails, or a required symbol is missing | Discovery: load CUDA Driver: ... |
| Driver discovery | Driver call fails, values are negative or malformed, UUIDs repeat | Discovery: exhaustive CUDA discovery: ... |
| descriptor | BDF syntax, PCI attribute, surface, label, target, or toolchain identity fails | Discovery: ... |
| descriptor | Driver reports zero total memory | Benchmark: native backend reported zero GPU capacity |
| inventory | CUDA/HSA keys collide | Discovery: native backends emitted duplicate GPU key ... |
| benchmark plan | any plan component is zero | Benchmark: native GPU benchmark received an unbounded plan or GPU benchmark plan is not bounded |
| benchmark owner | backend rediscovery itself fails | the backend's Discovery or Io error is propagated before ownership is selected |
| benchmark owner | multiple exact owners or no exact owner | Benchmark: multiple native backends claim exact GPU ... or GPU ... identity changed after discovery |
| benchmark reopen | backend disappears, device changes, or descriptor is not unique | CUDA backend benchmark errors listed above |
| allocation | size does not fit usize, exceeds discovered capacity, or Driver allocation fails | Benchmark with exact CUDA operation |
| transfer | enqueue, event creation, poll, or byte verification fails | Benchmark with CUDA operation or transfer verification text |
| compute bounds | no f32, zero FMA chain, invalid index space, or workgroup zero | Benchmark from compute_buffer_bytes, fma_template, or cuda_workgroup_lanes |
| lowering/build | target, ABI, LLVM audit, tool digest, scratch directory, compiler, or cubin inspection fails | Benchmark: Recipe-owned GPU artifact: ... |
| launch | grid overflow, dimension overflow, missing module entry, Driver launch failure | Benchmark with exact operation |
| compute verification | output length, nonfinite first value, or unchanged first value | Recipe-owned FLOP kernel did not produce a finite changed value |
| timeout | submission remains pending through cleanup | <operation> exceeded its <timeout> deadline |
| profile | returned property is not marked measured | ProbeEngine rejects the GPU measurement |

## Invariants summary

~~~toml
absence_rule = "missing CUDA library is absence only when no matching PCI hardware exists"
enumeration = "every Driver device is inspected; no partial inventory"
identity_selector = "UUID plus canonical PCI BDF and full descriptor equality"
target = "NVIDIA SM plus configured PTX ISA, validated before lowering"
compiler = "pinned opt plus llc plus ptxas; BuildPhase::Realize for probe kernel"
artifact = "ELF cubin only; inspected expected SM and recipe_probe_fma_f32 entry"
memory = "full bounded plan for transfers, at most 4 MiB aligned f32 region for FMA"
submission = "nonblocking stream, event-backed Pending, same-context allocations"
timeout = "no cancellation; retain Pending until terminal completion before unwind"
measurement = "capacity and four rates are measured properties only"
binding = "exact measured keys, borrowed contexts, no ordinal or product fallback"
~~~

The CUDA path contributes two kinds of evidence to Recipe: discovery identity
from the current Driver, PCI, library, and pinned tool surfaces, and measured
capacity/rates from completed transfers and a verified Recipe-owned kernel.
Configuration and seed estimates select bounded work and compiler inputs, but
neither is emitted as a measured hardware property.
