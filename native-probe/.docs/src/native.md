# Unified native probe orchestration

The <code>recipe-native-probe</code> crate is the native half of Recipe's
bare-metal probe. It owns one exact CUDA Driver route and one exact ROCr/HSA
route, turns their current hardware observations into <code>GpuDescriptor</code>
values, runs bounded transfer and Recipe-owned calculation submissions, and
returns measured <code>GpuMeasurement</code> values. It also reopens the same
origins for one preparation callback and lends the callback borrowed native
execution bindings.

The native probe is deliberately fail-closed. A library may be absent only
when PCI discovery proves that its vendor has no accelerator. If hardware is
present, a missing library, a failed load, a failed exhaustive enumeration, a
changed identity, an ambiguous host allocator, or an invalid native artifact
is an error. No ordinal, product-name, capacity, rate, or newest-file fallback
selects a device.

This document describes the crate as one pipeline. The module-specific
documents contain the same source facts organized by implementation file:
[configuration](config.md), [identity](identity.md),
[bounded benchmark helpers](benchmark.md), [CUDA](cuda.md), [HSA](hsa.md),
[scoped bindings](bindings.md), and the [public crate surface](lib.md).

## Source map and machine-readable contract

The authoritative implementation is:

| Area | Source | Primary responsibility |
| --- | --- | --- |
| Orchestration | <code>native-probe/src/native.rs:13-299</code> | Backend trait, backend sum type, constructors, display connector accounting, exhaustive discovery, exact benchmark ownership |
| Configuration | <code>native-probe/src/config.rs:6-47</code> | Explicit library candidates, target settings, pinned kernel toolchain, scratch directory, and host/Pci roots |
| Library and hardware identity | <code>native-probe/src/identity.rs:14-308</code> | Ordered library selection and hashing, toolchain digest, PCI preflight, PCI surface digests |
| Shared benchmark work | <code>native-probe/src/benchmark.rs:11-204</code> | Bounded timing, checked rates, F32 FMA template, deterministic input, output verification, measured provenance |
| CUDA route | <code>native-probe/src/cuda.rs:31-590</code> | Driver loading, descriptor construction, Driver API transfer tests, CUBIN build and launch |
| HSA route | <code>native-probe/src/hsa.rs:32-875</code> | ROCr lifetime, agent/ISA descriptor construction, HSA copies, HSACO build and dispatch |
| Preparation bindings | <code>native-probe/src/bindings.rs:22-482</code> | Exact profile resolution, CUDA contexts, HSA sessions, CPU host allocators, and higher-ranked callback scope |
| Source invalidation | <code>native-probe/build.rs:7-176</code> | Digest all native-probe and dependent Rust/Cargo inputs plus toolchain environment and rustc identity |
| Public exports | <code>native-probe/src/lib.rs:1-20</code> | Re-export the config types, probe, bindings, and host-config helper |

The pipeline can be consumed as a structured record:

~~~yaml
native_probe:
  input:
    config: NativeProbeConfig
    host_memory_key: Label
    pci_sysfs_root: absolute_path
    cuda_library_candidates: ordered_absolute_paths
    hsa_library_candidates: ordered_absolute_paths
    ptx_isa: major_times_10_plus_minor
    hsa_code_object_version: nonzero_u8
    offline_toolchain: pinned_tools_and_artifact_digests
    release: Label
    scratch_parent: absolute_path
    fma_chain_length: nonzero_u16
  discovery:
    backend_order: [cuda, hsa]
    result: GpuInventory
    exhaustive_for_profile: true
    key: backend_native_identity_at_canonical_pci_bdf
  measurement:
    plan: BoundedBenchmarkPlan
    transfer_tests: [host_to_device, device_to_host, device_to_device]
    calculation_test: Recipe_owned_f32_dependent_fma_chain
    result: GpuMeasurement
    provenance: measured_for_every_property
  preparation:
    profile_resolution: exact_machine_and_complete_ram_storage_gpu_key_sets
    native_bindings: borrowed_cuda_contexts_and_hsa_sessions
    callback_scope: higher_ranked_over_cuda_and_hsa_lifetimes
  failure_policy:
    vendor_absence: allowed_only_after_pci_preflight
    changed_identity: error
    incomplete_gpu_inventory: error
    fallback_selection: forbidden
    timeout: await_live_submission_then_return_benchmark_error
~~~

## Pipeline position

The CLI creates the configuration and installs the native probe into
<code>ProbeEngine</code>. The engine first performs discovery-only inspection to
derive an exact cache identity, then either loads a matching profile or runs
bounded host, storage, GPU, and peer measurements. The native route supplies
both <code>GpuDiscovery</code> and <code>GpuBenchmarkIo</code>.

~~~text
recipe probe
  -> host discovery and first RAM origin
  -> NativeProbeConfig from explicit options or fixed local candidates
  -> NativeGpuProbe::new
  -> ProbeEngine::current_cache_identity
       -> NativeGpuProbe::discover_all
       -> exhaustive GpuInventory and stable GPU keys
  -> cache hit, or ProbeEngine::probe
       -> bounded native CUDA/HSA transfer and FMA measurements
       -> measured topology and DiscoveryProfile
       -> identity-named MeasuredProfile cache file
       -> active-native receipt containing exact paths, digests, and settings

training or inference
  -> current_native_inputs reads the receipt, or derives the documented default
  -> exact profile cache load
  -> NativeGpuProbe::new or the thread-local retained probe
  -> discover_all and MeasuredProfile::resolve_local_inventory
  -> with_native_execution_bindings
       -> exact CUDA contexts and HSA sessions
       -> exact HSA CPU host allocators
  -> NativePreparationScope
       -> production host backend, native executor, deferred compiler, realizer
  -> init -> loop -> exit
  -> borrowed native resources are destroyed before the callback returns
~~~

Discovery, compilation, allocation, module loading, queue creation, and
binding construction are preparation work. They are not model work in the
finalized loop. The native probe has no dynamic placement path and never
stores a driver handle in a declaration.

## Configuration and construction

### Configuration fields

<code>NativeProbeConfig</code> is a cloneable, equality-comparable value with
one host origin, one explicit PCI root, two backend records, and one kernel
build record.

| Type and field | Meaning | Validation point |
| --- | --- | --- |
| <code>BackendLibrary::candidates</code> | Ordered CUDA or ROCr/HSA shared-library paths | <code>selected_library</code> requires every candidate to be absolute and a file or symlink when it exists |
| <code>CudaProbeConfig::ptx_isa</code> | PTX ISA encoded as major * 10 + minor | The NVIDIA artifact target validates it when a CUDA descriptor or benchmark is built |
| <code>HsaProbeConfig::code_object_version</code> | AMD code-object ABI version | <code>HsaBackend::new</code> rejects zero |
| <code>KernelBuildConfig::toolchain</code> | Pinned LLVM verifier/code generator plus backend linker or PTX assembler | <code>backend_toolchain_identity</code> requires verifier and LLVM codegen, then <code>lld</code> for AMD or <code>ptxas</code> for NVIDIA |
| <code>KernelBuildConfig::release</code> | Human-readable release label included in the digest | Label validation is performed by <code>label</code> when identities are built |
| <code>KernelBuildConfig::scratch_parent</code> | Parent for temporary native artifact realization | <code>validate_config</code> requires an absolute path; CLI receipt reopening also requires a private canonical directory |
| <code>KernelBuildConfig::fma_chain_length</code> | Dependent F32 FMA operations per element in the live calculation benchmark | <code>validate_config</code> and <code>fma_template</code> reject zero |
| <code>host_memory_key</code> | RAM origin supplied by host discovery | Descriptors retain it and profile resolution requires it to remain present |
| <code>pci_sysfs_root</code> | Explicit PCI sysfs root, normally <code>/sys/bus/pci/devices</code> | Absolute-root checks happen in PCI helpers; the CLI canonicalizes it before a receipt is reopened |

### Backend candidates are pinned, not guessed

<code>selected_library</code> walks candidates in their configured order.
Relative candidates fail immediately. Missing candidates are skipped. An
existing candidate must be a regular file or symlink, must canonicalize to a
regular file, and is read completely to obtain its SHA-256 digest. Canonical
paths are deduplicated, and the first existing canonical candidate is the
selected <code>PinnedLibrary</code>. A later candidate is still inspected while
the list is traversed, so an invalid configured path cannot be hidden behind an
earlier usable path.

If no candidate exists, the backend returns <code>None</code> from its open
operation. That result is meaningful only after the vendor PCI preflight has
proved that the vendor has no accelerator. Once matching hardware exists,
missing candidates are a discovery error.

### Constructors and diagnostic probes

<code>NativeGpuProbe::new(config)</code> first calls <code>validate_config</code>,
then constructs both <code>CudaBackend</code> and <code>HsaBackend</code> and
sets <code>exhaustive</code> to true. The resulting inventory is the only
inventory admissible to profile creation.

<code>NativeGpuProbe::cuda_diagnostic(config)</code> and
<code>NativeGpuProbe::hsa_diagnostic(config)</code> construct only one backend
and set <code>exhaustive</code> to false. They are useful for one-backend
diagnostics on a mixed machine, but <code>ProbeEngine::inspect</code> returns
<code>ProbeError::IncompleteGpuEnumeration</code> for either inventory. They
cannot create an accepted measured profile.

<code>NativeGpuProbe</code> retains only backend configuration and the PCI root.
The CUDA backend opens a fresh Driver and discovery snapshot for each discovery
or benchmark call. The HSA backend retains one initialized ROCr runtime in a
<code>RefCell</code> so a preparation thread can reuse that runtime while
recreating per-call sessions and resources.

## Stable identity before measurement

### Source digest and toolchain identity

The build script recursively hashes Rust files from native-probe and the
dependent core, executor, host, kernel, language, native-executor, planner,
primitives, scheduler, CUDA, HSA, and probe crates. It also hashes the relevant
Cargo manifests and lockfile, target/build environment variables, rustflags,
and <code>rustc -Vv</code>. The resulting lowercase SHA-256 string is exposed
as <code>RECIPE_NATIVE_PROBE_SOURCE_DIGEST</code>.

For a backend, <code>backend_toolchain_identity</code> hashes a version marker,
the backend name, the release label, the target configuration, that source
digest, and each required pinned tool's path and artifact digest. The returned
identity is named <code>recipe-owned-llvm-&lt;backend&gt;</code>, carries the
release as its version, and carries the digest as the exact toolchain identity.
AMD target configuration includes the target tail, code-object version, and FMA
chain length. NVIDIA configuration includes architecture, PTX ISA, and chain
length.

Consequently a source change, tool path or digest change, target change,
release change, or benchmark-chain change changes the measured descriptor's
toolchain identity and therefore the profile/cache identity.

### PCI vendor preflight and surface

<code>pci_accelerator_present(root, vendor)</code> requires an absolute root,
enumerates its entries, parses each <code>vendor</code> and <code>class</code>
file as hexadecimal, and returns true for the requested vendor when the PCI
class major is 0x03 (display controller) or 0x12 (processing accelerator).
Malformed or unreadable PCI identity files return <code>ProbeError::Io</code>
or <code>ProbeError::Discovery</code>; they are not treated as absence.

<code>pci_surface(root, bdf)</code> requires a real device directory and a
resolvable <code>driver</code> link. It hashes three labelled identity
surfaces:

* <code>driver</code>: the canonical driver target path, kernel
  <code>osrelease</code>, and the driver's module version. At least one file
  must be readable.
* <code>firmware</code>: revision, subsystem vendor/device, and VBIOS version.
* <code>pcie-link</code>: current and maximum link speed/width and NUMA node,
  prefixed by the canonical BDF.

Missing or permission-denied optional files are omitted from the digest, while
other I/O failures are errors. Every digest includes a domain marker, field
paths, byte lengths, and bytes, so the descriptor retains a stable identity
without retaining mutable sysfs text.

### Common descriptor identity

Both backends emit a <code>GpuDescriptor</code> with:

* a stable <code>key</code> containing the native UUID and canonical PCI BDF;
* a name and the configured host RAM key;
* backend, architecture, and ABI in <code>TargetIdentity</code>;
* nonzero capacity hint, driver, runtime ABI, firmware, and PCIe-link labels;
* PCIe transport with full duplex and one allowed transfer lane in each
  direction;
* asynchronous submission, a queue limit, a concurrency limit, subgroup and
  workgroup limits, shared-memory limit, and transfer/calculation overlap;
* a <code>ToolchainIdentity</code> that binds the descriptor to the exact
  Recipe-owned toolchain inputs and benchmark configuration.

Descriptor properties are observations or exact runtime limits. The seed
contract supplies only benchmark bounds through <code>BoundedBenchmarkPlan</code>;
it does not replace descriptor or measurement values.

## NativeGpuProbe orchestration

### Backend dispatch

The private <code>Backend</code> trait has exactly two operations:

<code>discover() -&gt; ProbeResult&lt;Vec&lt;GpuDescriptor&gt;&gt;</code> and
<code>benchmark(device, plan) -&gt; ProbeResult&lt;GpuMeasurement&gt;</code>.

<code>NativeBackend</code> stores either a CUDA or HSA backend and its
<code>backend()</code> method returns the trait object. The enum prevents a
caller from selecting an implementation by an arbitrary string or by device
ordinal.

### Exhaustive discovery

<code>GpuDiscovery::discover_all</code> loops over the configured backend vector
in CUDA-then-HSA order, extends one descriptor vector, sorts it by
<code>GpuDescriptor::key</code>, and rejects adjacent duplicate keys. It returns
the sorted descriptors and the constructor's <code>exhaustive</code> flag.

The implementation does not silently remove a backend that fails. A backend
can contribute zero descriptors only when its PCI vendor is absent. If it
has hardware, its library load, runtime initialization, exhaustive enumeration,
descriptor construction, or identity checks must succeed.

### Exact benchmark ownership

<code>GpuBenchmarkIo::benchmark_gpu</code> first requires
<code>BoundedBenchmarkPlan::is_bounded()</code>. It then asks every backend to
rediscover its current descriptors and compares each descriptor for full
equality with the requested descriptor. Exactly one backend must claim the
device:

* two claims return a benchmark error for multiple native owners;
* zero claims return a benchmark error that the identity changed after
  discovery;
* one claim receives the original bounded plan.

The selected backend opens or reopens its native runtime again and performs
its own exact descriptor match before allocation. Therefore discovery and
measurement are separate identity checkpoints, not one cached enumeration.

### Display connector accounting

Preparation needs the current number of enabled display connectors for each GPU
to size and schedule native resources. <code>enabled_display_connectors</code>
extracts the text after the final <code>@</code> in a retained origin key and
requires a canonical twelve-byte PCI BDF:
<code>dddd:bb:ss.f</code>, hexadecimal domain, bus, and slot, and function
0 through 7. Any other origin format is a discovery error.

The PCI device directory must exist and be a directory. A missing
<code>drm</code> directory means zero enabled connectors. Otherwise the code:

1. keeps only directory entries named <code>card</code> followed by decimal
   digits;
2. sorts card names;
3. under each card, keeps only directory entries prefixed by that card and a
   hyphen;
4. sorts connector paths;
5. reads each connector's <code>enabled</code> file;
6. counts exactly <code>enabled</code>, accepts exactly <code>disabled</code>,
   and rejects any other state.

The count uses checked <code>u32</code> addition. Enumeration, UTF-8, metadata,
and state-read failures are returned rather than guessed.

## CUDA route

### Open and exhaustive Driver discovery

<code>CudaBackend::open</code> performs the vendor preflight for NVIDIA vendor
0x10de. No NVIDIA accelerator returns <code>Ok(None)</code> without touching a
configured library. With hardware, <code>selected_library</code> must return a
pinned candidate. The canonical path must be UTF-8 for the Driver loader.
<code>Driver::load_from_path</code> and <code>Driver::discover</code> then load
and exhaustively enumerate the Driver snapshot. Loading or enumeration errors
are <code>ProbeError::Discovery</code>.

### Descriptor construction

For each <code>DeviceInfo</code>, <code>CudaBackend::descriptor</code>:

1. parses the Driver's <code>domain:bus:device.function</code> PCI string as
   hexadecimal domain, bus, and device plus decimal function;
2. rejects malformed components and functions above 7;
3. compares domain, bus, and device with the Driver attribute fields;
4. formats one lowercase sysfs BDF for PCI lookup and surface hashing;
5. constructs and validates <code>NvidiaTarget</code> from the device SM
   major/minor and configured PTX ISA;
6. computes the NVIDIA toolchain and Driver library identities;
7. emits key <code>cuda:&lt;uuid&gt;@&lt;sysfs_bdf&gt;</code>.

The target ABI is <code>elf64-cubin</code> and the target backend is
<code>nvidia-cuda-driver</code>. Capacity is Driver-reported total memory and
must be nonzero. Driver identity combines the Driver version and PCI driver
surface. Runtime identity combines the Driver version and the selected
library identity. Warp size, maximum threads per block, maximum shared memory
per block, asynchronous-engine count, and concurrent-kernel capability become
the scheduling fields. The native executor ceiling
<code>CUDA_MAXIMUM_SUBMISSION_QUEUES</code> is 32 because the Driver API does
not expose a finite stream-count attribute. The descriptor reports one
maximum concurrent task.

<code>matching_device</code> is an exact descriptor comparison over the same
discovery snapshot. A second match is an ambiguity error, and no match is an
identity change error.

### Transfer measurement

<code>benchmark_device</code> opens the Driver again, selects the exact device,
creates a context with Driver scheduling policy <code>Yield</code>, and creates
a nonblocking stream. The plan's transfer bytes must fit <code>usize</code> and
must not exceed the device's discovered total memory. It allocates two pinned
host buffers and two device buffers, then fills the source bytes with the
shared deterministic F32 pattern.

Three independent <code>time_bounded</code> loops run the same iteration count
and deadline:

1. host to device: <code>copy_h2d(device_source, host_source)</code>;
2. device to host: <code>copy_d2h(host_destination, device_source)</code>;
3. device to device: <code>copy_d2d(device_destination, device_source)</code>.

Each submission owns a completion event and is synchronously driven to terminal
completion. The host-to-device and device-to-host buffers are compared after
the first two loops. The device-to-device result is downloaded and compared
again. Any byte mismatch is a benchmark error.

The resulting memory rate is the checked byte rate of the device-to-device
loop. The two directional rates use their corresponding loop. No rate is
copied from a theoretical seed.

### Recipe-owned calculation measurement

The calculation path intentionally uses Recipe's scalar IR and artifact
builder, not the CUDA Runtime API or vendor math libraries.

1. <code>compute_buffer_bytes</code> caps the plan at 4 MiB and aligns it down
   to a whole F32. Zero aligned bytes fail.
2. <code>fma_template</code> builds one contiguous F32 input and one
   non-aliasing F32 output. It chains the configured number of dependent FMA
   instructions using fixed finite constants, so the output cannot be
   optimized into an unchanged copy.
3. A <code>NvidiaTarget</code> is built from the context SM and configured PTX
   ISA. The workgroup width is the largest power of two no greater than the
   smaller of the device maximum and element count.
4. <code>lower_elementwise</code> lowers the template with entry symbol
   <code>recipe_probe_fma_f32</code>. <code>ArtifactBuilder</code> realizes a
   CUBIN in the configured scratch parent using the pinned offline toolchain.
5. The result must be <code>BuiltArtifact::Cubin</code>, and inspected metadata
   must retain the exact entry symbol. The module is loaded through
   <code>Module::load_cubin</code>, the function is looked up, and a checked
   grid is computed from elements and workgroup lanes.
6. Each timed launch passes the input pointer, output pointer, and element count
   parameter storage, retains both allocations through completion, and waits
   on an event.
7. The output is downloaded and <code>verify_compute_output</code> requires
   equal buffer lengths, at least one F32, a finite first output, and a changed
   first output bit pattern.

The measured calculation rate is the lowered FMA work multiplied by completed
iterations and divided by elapsed nanoseconds. The CUBIN, module, stream,
buffers, and context are all temporary benchmark resources.

### CUDA completion and timeout behavior

<code>complete_cuda</code> polls a pending Driver token immediately and then
backs off from 50 microseconds to a 2 millisecond cap until the operation
completes or the plan's remaining deadline expires. CUDA has no cancellation
primitive. On deadline, the code continues polling the same token with a
10 millisecond to 100 millisecond cleanup backoff. It never submits replacement
work and never releases buffers while the device can still use them. Once the
token completes, its event is recycled and a benchmark error reports that the
operation exceeded its deadline. Poll, event, allocation, launch, module, and
artifact errors are benchmark errors.

## HSA route

### ROCr lifetime and open policy

<code>HsaBackend</code> stores the configured ROCr candidates, host key, PCI
root, code-object version, kernel settings, and an optional
<code>HsaRuntimeState</code> containing a pinned library and opened
<code>Runtime</code>.

<code>with_runtime</code> first checks AMD vendor 0x1002 PCI presence:

* no AMD accelerator and no retained runtime returns <code>Ok(None)</code>;
* no accelerator after a runtime was initialized is a disappearance error;
* AMD hardware with no selected library is an error;
* an existing runtime whose selected library path or digest changed is an
  identity error;
* otherwise the runtime is opened once and the operation runs against the
  retained state.

The operation result is wrapped in <code>Some</code>. This single retained
runtime is why preparation must pass the same probe instance through its
higher-ranked callback.

### Descriptor construction

Non-GPU ROCr agents are ignored. A GPU agent must support kernel dispatch, carry
a stable UUID value, carry an exact PCI address, expose AMD capability
properties, and expose physical queue limits. Its ISAs must pass
<code>exact_target</code> and the target must carry the canonical AMDGPU triple
prefix.

<code>exact_target</code> collects every ISA's AMD target. A single
non-generic target record wins. If all target records are generic but have one
distinct identity, that identity wins. Multiple non-generic target records,
ambiguous generic targets, an empty target set, or a non-AMDGPU ISA are
discovery errors.

The key is <code>hsa:&lt;uuid&gt;@&lt;pci_address&gt;</code>. The target backend is
<code>amd-rocr-hsa</code>, the architecture is the full ISA target string, and
the ABI is <code>elf64-amdgpu-code-object-v&lt;version&gt;</code>. Capacity is
the largest allocatable GPU-local global coarse-grained pool; if no such pool
exists, the nonzero AMD available-memory property is used. Driver identity
contains the KFD node and PCI driver surface. Runtime identity contains HSA
and AMD extension versions plus the selected ROCr library identity.

The descriptor retains the first ISA wavefront width, the minimum workgroup
limit across all ISAs, KFD LDS capacity, physical maximum queue count, one
maximum concurrent task, and SDMA presence as the transfer/calculation
overlap signal. LDS capacity is read from
<code>/sys/class/kfd/kfd/topology/nodes/&lt;node&gt;/properties</code> and requires
a nonzero integer <code>lds_size_in_kb</code>.

### Transfer measurement

<code>benchmark_with_runtime</code> rediscoveries the complete agent set and
selects exactly one agent whose descriptor equals the requested descriptor. It
then selects one CPU memory agent, preferring the GPU's NUMA node and
falling back to the first CPU agent when no same-NUMA CPU exists. Missing or
duplicate GPU UUIDs and a missing CPU agent are benchmark errors.

The plan's transfer bytes must fit <code>usize</code> and not exceed the
selected GPU capacity. The benchmark allocates CPU fine-grained source and
destination memory and GPU coarse-grained source and destination memory.
Access is granted in both directions before any submission. A deterministic
F32 input is copied into the CPU source allocation.

Three bounded loops submit asynchronous ROCr copies:

1. CPU fine source to GPU coarse source;
2. GPU coarse source to CPU fine destination;
3. GPU coarse source to GPU coarse destination.

The first two are checked by copying the CPU destination to host bytes. The
third is checked by downloading the destination. Mismatches are benchmark
errors. Rates are calculated exactly like the CUDA route and marked measured.

### Recipe-owned HSACO calculation

The HSA calculation route uses the same scalar template and constants as CUDA,
but lowers to an <code>AmdTarget</code> built from the selected target tail and
configured code-object version.

1. The compute buffer is capped at 4 MiB and aligned to F32.
2. Workgroup lanes are the largest power of two no greater than the smaller
   of element count and the minimum ISA workgroup limit.
3. The lowered entry symbol is <code>recipe_probe_fma_f32</code>.
4. The pinned offline builder must return <code>BuiltArtifact::Hsaco</code>.
5. The executable is loaded with <code>Session::load_hsaco</code>, and the
   inspected kernel symbol is looked up.
6. The live ROCr kernarg segment size must equal inspected HSACO metadata.
   CPU kernarg storage is allocated, GPU access is granted, and the explicit
   three-slot Recipe ABI writes input pointer at offset 0, output pointer at
   offset 8, and element count at offset 16.
7. A single-producer queue is created with the physical minimum packet count.
   One-dimensional geometry dispatches the kernel for each timed iteration.
8. The output is copied to the CPU host allocation and checked with the shared
   finite-and-changed F32 verifier.
9. Queue, executable, kernel reference, and kernarg resources are closed or
   dropped before the calculation rate is returned.

Kernarg size, pointer conversion, offset arithmetic, queue capability, dispatch,
artifact type, symbol, and metadata mismatches are benchmark errors.

### HSA completion and timeout behavior

<code>complete_hsa</code> polls <code>Pending</code> tokens with the same
50-microsecond to 2-millisecond benchmark backoff used by CUDA. On deadline it
keeps the token and its retained signal, allocations, and executable alive,
polling at a 10-millisecond to 100-millisecond cleanup cap. It returns only
after terminal completion, then reports the original operation's deadline
failure. It does not release a live token or spin a rapid display-driver query
loop.

## Lowering and native API contracts

The probe's submission code relies on stricter contracts in the CUDA, HSA, and
kernel crates. These are part of the native route even though their source is
outside <code>native-probe</code>.

### Recipe-owned lowering and artifact realization

<code>recipe-kernel::lower_elementwise</code> validates the kernel template,
backend target, and lowering options before it emits direct AMDGPU or NVPTX
LLVM IR. The generated entry point has one global pointer argument for each
input and output, followed by a 64-bit element count. Each lane computes one
linear element at most, with target-specific index intrinsics. The lowering
result retains:

* the LLVM module text;
* an ordered <code>KernelAbi</code> with argument kinds, byte size, alignment,
  element count, and workgroup lanes;
* total FMA work as a typed <code>FlopCount</code>; and
* the exact <code>KernelTarget</code> used for emission.

The lowerer audits the generated module and rejects an external declaration
unless it is an LLVM target intrinsic. The probe therefore cannot accidentally
benchmark a host runtime call or a vendor math library. Invalid entry symbols,
workgroup sizes outside 1 through 1024, unknown scalar values, unsupported
operations, and arithmetic overflow are lowering errors before any tool is
invoked.

<code>ArtifactBuilder::new</code> canonicalizes and hashes each configured
tool. It verifies the same path and digest again at build time. The probe calls
<code>build(BuildPhase::Realize, ...)</code>; the builder has no Finalize, init,
loop, or exit compiler phase. It validates target equality, creates a private
mode-700 workspace below the configured scratch parent, writes mode-600 inputs
with create-new semantics, and invokes the verifier with output disabled.

The backend realization is explicit:

| Target | Pinned tool sequence | Structural check |
| --- | --- | --- |
| AMD | LLVM codegen to an object, then the pinned ELF linker to HSACO | ELF machine and HSA ABI, code-object version, target ID, function and descriptor symbols, MessagePack kernel metadata, argument offsets and kinds, kernarg size/alignment, and workgroup limit |
| NVIDIA | LLVM codegen to PTX, then the pinned <code>ptxas</code> to CUBIN | CUDA ELF ABI and machine, exact SM encoding, function symbol, executable nonempty text, and entry symbol |

Tool invocations clear the inherited environment and set
<code>LC_ALL=C</code> and <code>SOURCE_DATE_EPOCH=0</code>. Build outputs are
read only from regular non-symlink files and the temporary workspace is removed
when its owner drops. A failed invocation reports bounded stderr as a
<code>LoweringError</code>, never as a partially usable artifact.

The probe checks the returned artifact variant and entry identity again because
the live submission ABI is part of measurement. CUDA receives the inspected
entry symbol from the CUBIN inspection. HSA compares the live ROCr kernel's
kernarg segment size with inspected metadata before writing the explicit
three-slot ABI.

### CUDA resource and completion contract

The CUDA crate is a Linux-only, 64-bit Driver API loader. It uses
<code>dlopen</code> with a pinned path, resolves the required Driver symbols,
and exposes capabilities for optional symbols. <code>Driver::discover</code>
calls <code>cuInit</code>, reads a nonnegative Driver version and device count,
enumerates every ordinal, and rejects duplicate UUIDs. It reads names, UUIDs,
PCI identity, total memory, compute capability, and all scheduling attributes
used by the descriptor.

<code>Context::create</code> validates the R470-safe flag set, creates a context,
and immediately pops it while checking that the popped handle is the one just
created. Context guards push and pop the same handle, and every resource keeps
the context lifetime. Device buffers and pinned host buffers reject zero sizes
and check every offset-plus-length range. Streams, modules, events, contexts,
and allocations have explicit close methods plus drop cleanup.

The asynchronous Driver wrappers enforce a same-context rule for every source,
destination, function, event, and keepalive buffer. A copy or launch records a
completion event and returns a <code>Pending</code> token whose borrow keeps the
referenced Rust resources alive. <code>Pending::poll</code> maps
<code>CUDA_ERROR_NOT_READY</code> to Pending and any other non-success status to
an error. <code>Pending::recycle_event</code> succeeds only after a terminal
poll. These contracts are why <code>complete_cuda</code> can safely await a
timed-out operation before releasing its resources.

### ROCr and HSA resource contract

The HSA crate is a 64-bit large-model ABI loader with no process-global runtime.
It opens the configured library with immediate symbol resolution, requires the
full discovery, allocation, queue, executable, signal, and asynchronous-copy
symbol surface, and calls <code>hsa_init</code> while constructing a
<code>Runtime</code>. Discovery decodes agent, ISA, queue, memory-pool, UUID,
PCI, and system-version attributes into typed descriptions. Unknown memory-pool
flag bits are retained rather than rewritten.

An HSA <code>Session</code> owns one GPU agent and its dependent resources.
Fine-grained host allocations, coarse GPU allocations, access grants, queues,
executables, kernels, kernarg allocations, completion signals, and asynchronous
copy or dispatch tokens are all lifetime-linked. A queue submission returns a
pending token. Polling reports Complete or Pending and preserves the signal and
resources until terminal completion. Runtime, queue, allocation, executable,
invalid dispatch, asynchronous signal, session-poisoned, and deferred-retirement
conditions remain typed HSA errors; the native route prefixes them with the
operation and returns a benchmark error.

The HSA dispatch packet is a 64-byte large-model structure. The probe does not
construct an untyped packet itself: it passes the inspected kernel, explicit
kernarg allocation, and <code>DispatchGeometry</code> to the queue API. The
queue API enforces its advertised minimum and maximum packet counts and
single-producer kind. This preserves the same resource and completion contract
as the CUDA stream path while using ROCr's signal protocol.

## Shared benchmark and result contract

<code>BoundedBenchmarkPlan</code> is bounded exactly when buffer bytes,
iteration count, and maximum duration are all nonzero. The engine derives plans
from seed estimates and clamps each buffer between 4 KiB and 64 MiB. The native
route additionally caps only the calculation buffer at 4 MiB. Every timed loop
stops when its iteration limit or deadline is reached.

<code>time_bounded</code> rejects an unbounded plan, invokes a submit-and-complete
closure with the remaining duration, and rejects zero completed work or a zero
elapsed duration. Transfer and FMA work counters use checked <code>u128</code>
multiplication. Nanoseconds must be nonzero, and resulting rates must fit the
typed <code>u64</code> rate wrappers.

The shared input pattern writes finite native-endian F32 values whose mantissa
is derived from the element index. The shared output check is intentionally
small but real: buffers must have equal length and at least one F32, and the
first output must be finite and bitwise different from the first input.

Each backend returns:

| Measurement field | CUDA source | HSA source |
| --- | --- | --- |
| <code>capacity</code> | Driver total memory | largest GPU-local coarse pool, or AMD available memory |
| <code>calculation_rate</code> | completed lowered CUBIN FMA work | completed lowered HSACO FMA work |
| <code>memory_rate</code> | device-to-device copy | GPU coarse-to-coarse copy |
| <code>host_to_device_rate</code> | pinned host to device | CPU fine to GPU coarse |
| <code>device_to_host_rate</code> | device to pinned host | GPU coarse to CPU fine |

All five properties are wrapped by <code>measured</code>, which sets
<code>PropertyProvenance::Measured</code>. <code>ProbeEngine</code> rejects any
other provenance and stores the measurements in the validated
<code>MeasuredProfile</code>. Its discovery profile maps GPU descriptors and
measurements to a <code>DiscoveredDevice</code> with a transfer capability and a
calculation capability. The profile also retains the stable
<code>MeasuredGpuOrigin</code> key, topology identity, discovery identity,
benchmark metadata, and cache identity.

### ProbeEngine acceptance boundary

<code>ProbeEngine::inspect</code> discovers the host and native inventory before
it constructs a benchmark plan. It normalizes and validates the host, requires
<code>GpuInventory::exhaustive</code>, rejects an empty GPU list, sorts native
descriptors by key, and checks that every GPU points at a known host RAM key,
advertises asynchronous submission, and reports nonzero queue and concurrent
task limits. This is where a diagnostic inventory becomes an explicit
<code>IncompleteGpuEnumeration</code> failure instead of a partial profile.

The discovery-only inspection also builds the exact cache identity. The cache
digest includes the seed contract, machine fingerprint, every GPU key/name,
capacity hint, host key, backend/architecture/ABI, driver/runtime/firmware/link
identities, transport and duplex, toolchain name/version/digest, queue and
concurrency limits, subgroup/workgroup/shared-memory limits, transfer overlap,
and directional inflight limits. Native tool, library, PCI, target, or source
changes therefore select a different identity before expensive measurement.

When the cache does not contain that identity, <code>ProbeEngine::probe</code>
uses seed estimates only to derive bounded plans. It benchmarks every host RAM
and storage domain, every GPU through <code>benchmark_gpu</code>, and every peer
session. For each GPU it requires all five properties to carry measured
provenance. It then builds measured topology and discovery records, validates
topology scheduling properties and discovery against topology, validates the
codec representation, and stores only the complete profile. A cache hit still
passes through the cache reader and its profile validator; it does not bypass
current discovery identity checks.

The resulting local GPU discovery record contains the measured capacity and
memory rate as transfer capability, measured host-to-device and device-to-host
rates as directional links, and the measured calculation rate plus the
descriptor's target and calculation limits as <code>CalculationCapability</code>.
The stable origin key remains in <code>MeasuredOrigins::gpu</code>, while the
numeric topology <code>DeviceId</code> is assigned by the probe engine. Later
preparation resolves that key back to the current descriptor and refuses a
changed target or key set.

## Preparation-scoped bindings

### Host backend configuration

<code>host_backend_config_from_inventory</code> maps every resolved RAM origin
to <code>HostDeviceBinding::Ram</code>. For each resolved storage origin it
creates a deterministic <code>DiskFileSpec</code> below that domain's
benchmark root:

~~~text
.recipe-run-{run}-device-{device}-arena
~~~

Construction performs no filesystem I/O. The host backend later creates the
arena with <code>create_new</code>, so an overlapping <code>RunId</code> fails
without overwriting another run. The root-library
<code>NativeHostPlan::backend_config</code> emits the same run-scoped naming
rule after it owns sorted RAM and storage origins.

### Exact profile reopening

<code>with_native_execution_bindings</code> performs these steps in order:

1. call <code>probe.discover_all()</code>;
2. call <code>profile.resolve_local_inventory(host, &amp;inventory)</code>;
3. partition resolved GPU origins by the exact backend labels
   <code>nvidia-cuda-driver</code> and <code>amd-rocr-hsa</code>;
4. compute current enabled-display-connector counts from each retained key;
5. reopen every expected CUDA and HSA device;
6. construct borrowed <code>CudaBinding</code> and <code>HsaBinding</code>
   values;
7. invoke the callback exactly once while all borrowed contexts, sessions, and
   allocators remain alive.

Profile resolution first validates the measured profile, requires
<code>GpuInventory::exhaustive</code>, matches the current host fingerprint to
one measured machine, and requires equal stable key sets for local RAM,
storage, and GPU origins. GPU target identity must still match the profile and
each GPU and storage origin must reference one currently resolved RAM key.
Capacity, name, ordinal, rate, and performance similarity are not selectors.

The callback is higher-ranked over both native lifetimes:

~~~text
FnOnce for<'cuda, 'hsa>
  (NativeExecutionBindings<'cuda, 'hsa>) -> ProbeResult<T>
~~~

It may return an owned result, but cannot return a value that borrows a CUDA
context, HSA session, or HSA CPU allocator. The HSA callback runs inside the
retained ROCr runtime closure. If no HSA device is expected, a probe with no
HSA runtime surface may invoke the callback with an empty HSA vector. If HSA
devices are expected, an absent backend or absent reopened runtime surface is
an error.

### CUDA realization

<code>realize_cuda</code> calls <code>CudaBackend::open</code>. No current NVIDIA
surface is acceptable only when the expected CUDA map is empty. Otherwise it
is an error. Every current Driver device is converted to a descriptor and must
occur in the measured expected map. A current extra device, duplicate key,
missing deployment identity, failed context creation, or missing expected key
fails the scope. Contexts use default Driver context flags for execution
bindings. The realized vector is sorted by topology <code>DeviceId</code> and
each binding receives the current queue limit and connector count.

### HSA realization and CPU allocator selection

<code>realize_hsa</code> partitions the owned ROCr agents into CPU and GPU
agents. CPU host allocators are retained only when they expose both:

* a runtime-allocatable global pool with kernarg-initialization capability; and
* a runtime-allocatable global pool with fine-grained or extended-scope
  fine-grained capability.

Retained allocators are sorted by NUMA node, UUID, name, and vendor. For every
current GPU descriptor, the function requires an expected measured key,
an exact ISA target, a physical queue, a unique key, and a session. The GPU's
NUMA node chooses one allocator:

| Matching allocator set | Result |
| --- | --- |
| exactly one same-NUMA allocator | use it |
| no same-NUMA allocator and exactly one allocator total | use the sole fallback |
| no allocator | missing-host-allocator error |
| several same-NUMA allocators | ambiguous-same-NUMA error |
| no same-NUMA allocator and several fallbacks | ambiguous-fallback error |

The realized HSA vector is sorted by <code>DeviceId</code>. Each binding carries
the session, selected CPU allocator, full target ID, code-object version,
minimum queue packets, maximum submission queues, and connector count.

### Binding set invariants

<code>require_all_reopened</code> compares the measured expected map with the
keys actually reopened. The root-library preparation layer then builds maps by
<code>DeviceId</code>, rejects duplicate binding entries, requires the union of
CUDA and HSA IDs to equal the measured local GPU set, and rejects any ID
appearing in both backend maps. A callback never receives a partial set.

## Root-library and CLI consumers

### CLI probe

<code>src/cli.rs::run_probe</code> requires bare metal, reads the topology seed,
creates a private state and scratch directory, discovers the host, and uses
the first RAM domain's key for <code>NativeProbeConfig</code>. With no explicit
paths, the CLI uses fixed candidate lists for:

* CUDA Driver libraries under the system and NVIDIA library directories;
* ROCr/HSA libraries under ROCm and system library directories;
* LLVM <code>opt</code> and <code>llc</code> for required verifier/codegen;
* LLVM <code>ld.lld</code> and NVIDIA <code>ptxas</code> as backend-dependent
  optional tools.

The local defaults set PCI root to <code>/sys/bus/pci/devices</code>, PTX ISA
74, HSA code-object version 6, release
<code>auto-pinned-local-tools-and-benchmark-v3</code>, and FMA chain length 64.
Explicit tool paths are inspected and pinned; missing required LLVM tools fail
configuration. Optional tools may be absent while constructing the config, but
the identity step fails if a measured backend needs its missing linker or
assembler.

The CLI installs one <code>NativeGpuProbe</code> as both GPU discovery and GPU
benchmark I/O in <code>ProbeEngine</code>. A fresh profile is stored only after
all topology, discovery, and measured-provenance validation succeeds. The CLI
prints the profile path and identities, then captures an active-native receipt.

### Active-native receipt and current inputs

The canonical v1 receipt records the profile path and cache identity, host RAM
key, PCI root, scratch parent, selected CUDA/HSA library pins, LLVM and
backend-tool pins, PTX ISA, HSA code-object version, release, and FMA chain.
It uses fixed field order, tab separators, byte-hex paths and labels, lowercase
SHA-256 digests, and decimal scalar values. Unknown, missing, noncanonical, or
trailing fields are rejected.

<code>current_native_inputs</code> revalidates the receipt's host RAM key and
all pinned paths and digests before rebuilding <code>NativeProbeConfig</code>.
If no receipt exists, it derives the documented local defaults and computes the
discovery-only cache identity to name one exact profile path. It never scans
for a newest or merely similar profile.

### Native preparation

<code>src/native_prepare.rs::with_native_preparation</code> constructs a full
exhaustive <code>NativeGpuProbe</code>, resolves the current inventory against
the profile, obtains the scoped bindings above, and builds a
<code>NativePreparationScope</code>. Scope construction checks the machine ID,
binding set, target ABI, target architecture, deployment capabilities,
toolchain identity, runtime policy, and duplicate target specifications. CUDA
target specs require all <code>REQUIRED_DRIVER_SYMBOLS</code>; HSA specs
require the binding's full target prefix and code-object version.

Identical measured target identities share one deferred compiler specification,
but every physical GPU remains a separate device target entry. A mismatch
between equivalent target specs is an identity error. The callback receives
the scope only after <code>DeferredArtifactCompiler</code> accepts all target
specifications.

<code>with_current_native_preparation</code> loads the exact profile from the
receipt identity and stores one <code>(NativeProbeConfig, NativeGpuProbe)</code>
in thread-local state. A second call on the same thread must use an equal
configuration. This preserves the one initialized ROCr runtime while each run
still receives fresh run-scoped host resources, contexts, sessions, compiler
inputs, and executor resources.

### Training, inference, and acceptance

The training and inference entry points call
<code>with_current_native_preparation</code>. Their callbacks consume the
profile, binding vectors, host plan, and target plan to construct the production
host backend, staged CUDA/HSA bridge, <code>LocalCandidateFactory</code>,
<code>NativeExecutorDriver</code>, deferred compiler, and
<code>NativeCandidateRealizer</code>. They then execute the complete
<code>init -&gt; loop -&gt; exit</code> lifecycle while native bindings remain in
scope.

Inference uses the scope to derive measured runtime tuning, build host staging,
prepare one local execution, and return an execution report after teardown.
Training performs the same setup and, when a supplied native kernel is being
resumed, checks topology identity, discovery identity, target identity, and
toolchain identity before accepting the prebuilt bundle. Acceptance records
the measured profile, device origins, target identities, and toolchain
identities through the same callback boundary.

No consumer is permitted to retain a dynamic CUDA or HSA handle in a model
declaration or to realize an artifact inside the finalized loop.

## Invariants and failure matrix

### Invariants that must hold

| Invariant | Enforcement |
| --- | --- |
| Both vendor routes are considered for an accepted profile | <code>NativeGpuProbe::new</code> stores CUDA and HSA backends and marks the inventory exhaustive |
| A one-backend diagnostic cannot masquerade as complete hardware | Diagnostic constructors set <code>exhaustive = false</code>; <code>ProbeEngine</code> rejects it |
| Vendor absence is the only absent-backend success | PCI preflight gates missing-library handling; post-initialization disappearance is fatal |
| Every GPU has one canonical native key | CUDA and HSA keys include UUID and PCI address; the unified inventory sorts and rejects duplicates |
| Descriptor identity is current at every boundary | Discovery, benchmark ownership, backend reopen, profile resolution, and binding realization all compare exact identities |
| Measurements are real completed submissions | Transfer loops and Recipe-owned FMA launches complete native tokens and verify bytes or changed finite output |
| All emitted GPU rates and capacity are measured | Backend wraps every property with measured provenance; ProbeEngine validates it |
| Native artifacts use Recipe-owned semantics | The FMA template lowers through <code>ArtifactBuilder</code>; no CUDA Runtime API, HIP API, or vendor math library is used |
| Live resources are not released while a token is pending | CUDA and HSA timeout cleanup keep polling the original token to terminal completion |
| HSA CPU allocation is deterministic | Only allocatable fine-grained/kernarg CPU agents are considered, with explicit NUMA ambiguity errors |
| Bindings cannot escape preparation | The callback is higher-ranked over CUDA and HSA lifetimes |
| The production device set is complete | Profile resolution, <code>require_all_reopened</code>, unique binding maps, and root scope checks require exact set equality |
| Runtime configuration cannot drift silently | Library, tool, source, target, release, and benchmark settings participate in digests and receipt validation |
| Run-scoped disk paths cannot overwrite another run | Host config construction is pure; realization uses <code>create_new</code> |

### Failure classes

| Stage | Representative failures | Result |
| --- | --- | --- |
| Construction | zero FMA chain, relative scratch parent, zero HSA code-object version | <code>ProbeError::Discovery</code> |
| Library selection | relative candidate, non-file target, canonicalization/read failure, changed digest | discovery error or I/O error |
| PCI preflight | non-absolute root, malformed vendor/class, missing required device directory or driver link | discovery or I/O error |
| Runtime open | hardware present with no configured library, Driver/ROCr load failure, exhaustive enumeration failure | discovery error |
| Descriptor | malformed CUDA PCI string, attribute disagreement, unstable HSA UUID, missing PCI/ISA/queue/capability, ambiguous HSA target, zero capacity/LDS | discovery error |
| Unified discovery | duplicate descriptor key, zero devices at ProbeEngine inspection, non-exhaustive diagnostic inventory | discovery error or incomplete-enumeration error |
| Benchmark admission | unbounded plan, identity changed, multiple backend owners | benchmark error |
| Transfer benchmark | allocation/access grant/copy/poll failure, bytes exceed capacity, byte mismatch, no timed work | benchmark error |
| Calculation benchmark | invalid FMA template, lowering/build failure, wrong CUBIN/HSACO, wrong entry or metadata, invalid grid/ABI, changed or nonfinite output | benchmark error |
| Completion | deadline reached while a native token is live | token is awaited to terminal completion, then a deadline benchmark error is returned |
| Profile reopen | changed machine fingerprint, missing/new/duplicate RAM/storage/GPU key, changed target, nonexhaustive inventory | invalid-profile error |
| Binding realization | extra or missing current GPU, duplicate key, missing context/session, ambiguous or missing HSA CPU allocator, missing display state | discovery error |
| Root scope | binding machine or device set differs, target ABI/architecture/toolchain/policy mismatch, missing CUDA Driver symbol, duplicate target policy | native identity or target-specification error |
| CLI handoff | stale or insecure profile/receipt, changed tool/library digest, invalid canonical field order, missing profile | preparation or local-configuration error |

Errors are returned through <code>ProbeResult</code> and then wrapped by
<code>NativePreparationError</code> or the training/inference error type at the
root-library boundary. There is no rejection branch that converts an invalid
or changed native state into an absent device.

### Native error wording anchors

The following message stems are emitted by the native implementation. They are
useful when locating a failing boundary, but the enum variant remains the
stable programmatic contract.

| Function or stage | Condition | Message stem or exact text |
| --- | --- | --- |
| <code>validate_config</code> | FMA chain is zero | <code>native FLOP benchmark requires a nonzero FMA chain</code> |
| <code>validate_config</code> | Scratch parent is relative | <code>kernel scratch parent ... is not absolute</code> |
| <code>HsaBackend::new</code> | Code-object version is zero | <code>HSA code-object version must be nonzero</code> |
| <code>enabled_display_connectors</code> | Origin has no canonical BDF suffix | <code>GPU origin ... has no canonical PCI BDF suffix</code> |
| connector scan | Invalid connector state | <code>GPU DRM connector ... reported invalid enabled state ...</code> |
| <code>discover_all</code> | Duplicate descriptor key | <code>native backends emitted duplicate GPU key ...</code> |
| <code>benchmark_gpu</code> | Plan is unbounded | <code>native GPU benchmark received an unbounded plan</code> |
| <code>benchmark_gpu</code> | Two backend claims | <code>multiple native backends claim exact GPU ...</code> |
| <code>benchmark_gpu</code> | No backend claim | <code>GPU ... identity changed after discovery</code> |
| CUDA open | NVIDIA hardware but no library | <code>NVIDIA PCI accelerator is present but no configured CUDA Driver library exists</code> |
| CUDA open | Driver load or enumeration | <code>load CUDA Driver: ...</code> or <code>exhaustive CUDA discovery: ...</code> |
| CUDA descriptor | Driver PCI disagreement | <code>CUDA PCI identity ... disagrees with Driver attributes ...</code> |
| CUDA benchmark | Device disappeared | <code>CUDA backend disappeared before benchmark</code> or <code>CUDA GPU ... disappeared or changed identity</code> |
| CUDA completion | Deadline after cleanup | <code>CUDA ... exceeded its ... deadline</code> |
| HSA runtime | AMD hardware but no library | <code>AMD PCI accelerator is present but no configured ROCr/HSA runtime library exists</code> |
| HSA runtime | Runtime/library disappears | <code>every AMD PCI accelerator disappeared after HSA initialization</code>, <code>ROCr/HSA library disappeared after initialization</code>, or <code>ROCr/HSA library identity changed after initialization</code> |
| HSA benchmark | Device disappears | <code>ROCr/HSA backend disappeared before benchmark</code> or <code>HSA GPU ... disappeared, changed, or is ambiguous</code> |
| HSA target | Multiple target records | <code>HSA GPU ... exposed ... distinct non-generic artifact targets</code> or <code>ambiguous generic artifact targets</code> |
| HSA completion | Deadline after cleanup | <code>HSA ... exceeded its ... deadline</code> |
| profile partition | Unsupported backend label | <code>GPU origin ... uses unsupported native backend ...</code> |
| CUDA reopen | Extra current device | <code>reopened CUDA device ... was not present in the measured profile</code> |
| HSA reopen | Extra current device | <code>reopened HSA device ... was not present in the measured profile</code> |
| HSA allocator | No exact CPU allocator | <code>measured HSA device ... has no CPU agent with allocatable fine-grained and kernarg pools</code> |
| HSA allocator | Ambiguous same NUMA | <code>... same-NUMA CPU host allocators; exact binding is ambiguous</code> |
| HSA allocator | Ambiguous fallback | <code>... no same-NUMA CPU host allocator and ... fallback allocators; exact binding is ambiguous</code> |
| reopening | Expected key missing | <code>CUDA/HSA did not reopen measured GPU origins ...</code> |

All lower-level CUDA, HSA, and kernel errors are preserved after the operation
stem is added. A diagnostic or cache caller can therefore distinguish discovery
failure, benchmark failure, invalid-profile failure, and local preparation
failure without interpreting a success status as hardware evidence.

## Verification checklist for changes to this pipeline

When changing native-probe behavior, preserve this order of evidence:

1. inspect the exact source path and configuration that changed;
2. run the real <code>recipe probe</code> entry point with actual host, PCI,
   CUDA, HSA, and pinned-tool inputs;
3. verify the produced or loaded profile contains exhaustive descriptors and
   measured GPU properties;
4. run the real training or inference entry point through
   <code>with_current_native_preparation</code>;
5. independently inspect the resulting native execution end state and teardown
   evidence, rather than treating a text status or exit code as proof;
6. exercise one reachable identity, timeout, or allocator edge case through the
   same public boundary and repair the observed cause before widening scope.

Compilation, formatting, and linting can establish structural Rust validity but
do not prove that a native submission completed or that a current machine still
matches its measured profile. Hardware and the corresponding offline toolchain
are prerequisites for a successful acceptance run.
