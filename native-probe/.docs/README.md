# `recipe-native-probe`

This file is the source-oriented contract for the `recipe-native-probe` crate.
It describes the code currently in this checkout, not a desired future probe.
Paths and line numbers below are relative to the repository root and are kept
close to the implementation they describe.

```toml
[crate]
name = "recipe-native-probe"
version = "0.1.0"
edition = "2024"
description = "Bare-metal NVIDIA CUDA Driver and AMD ROCr probes for Recipe"
library = "native-probe/src/lib.rs"
build_script = "native-probe/build.rs"
workspace_member = true

[public]
types = [
  "BackendLibrary",
  "CudaProbeConfig",
  "HsaProbeConfig",
  "KernelBuildConfig",
  "NativeProbeConfig",
  "NativeGpuProbe",
  "NativeExecutionBindings",
]
functions = [
  "host_backend_config_from_inventory",
  "with_native_execution_bindings",
]

[error]
type = "recipe_probe::ProbeError"
discovery_and_binding_are_fatal = true
benchmark_failures_are_fatal = true
```

## Role and boundaries

`recipe-native-probe` is the GPU half of `recipe-probe`. It performs exhaustive
bare-metal NVIDIA Driver and AMD ROCr/HSA discovery, builds exact
`GpuDescriptor` values, and runs bounded transfer and Recipe-owned calculation
benchmarks that return `GpuMeasurement` values with `Measured` provenance. It
does not choose a placement, schedule a graph, or run a model. The generic
`ProbeEngine` owns the host probe, benchmark-plan derivation, profile assembly,
cache identity, and profile validation (`probe/src/engine.rs:32-160`).

The native backends intentionally use the reviewed interfaces in
`recipe-cuda` and `recipe-hsa`. NVIDIA uses the CUDA Driver API only. AMD uses
ROCr/HSA and its AQL queue path. There is no CUDA Runtime API, HIP path, raw KFD
queue, or vendor operation library in this crate. Generated benchmark kernels
go through `recipe-kernel::ArtifactBuilder` and pinned offline tools, not a
toolkit runtime API.

Discovery, compilation, module loading, allocation, and runtime-handle
creation belong before the finalized execution loop. The binding callback below
keeps those handles inside preparation and lends them to the native executor;
the declarations and dynamic placement code never receive a leaked runtime
handle.

## Manifest and build graph

### Cargo manifest

`native-probe/Cargo.toml` is a library package with no feature flags, examples,
tests, or binaries of its own. It has these direct dependencies:

| dependency | purpose in this crate |
| --- | --- |
| `recipe-core` | labels, IDs, target identities, units, toolchain identity, scalar and kernel template types |
| `recipe-cuda` | reviewed dynamic CUDA Driver loader, discovery, contexts, buffers, streams, events, modules, and pending completions |
| `recipe-hsa` | reviewed dynamic ROCr loader, agent/ISA/memory-pool discovery, sessions, allocations, queues, dispatch, and pending completions |
| `recipe-host` | `HostBackendConfig`, RAM bindings, and deterministic disk arena specifications |
| `recipe-kernel` | `OfflineToolchain`, target lowering, artifact realization, cubin/HSACO inspection, and kernel ABI metadata |
| `recipe-native-executor` | `CudaBinding`, `HsaBinding`, and the CUDA submission-queue ceiling |
| `recipe-probe` | discovery and benchmark traits, descriptors, measurements, profile resolution, and `ProbeError` |
| `sha2` | SHA-256 for shared-library, PCI-surface, and toolchain identities |

`sha2` is also a build dependency because `build.rs` computes the source digest.
The crate denies `unsafe_op_in_unsafe_fn` and undocumented unsafe blocks at its
own lint boundary (`native-probe/Cargo.toml:1-24`). Unsafe implementation work
is delegated to the reviewed native crates.

The workspace root includes `native-probe` as member and depends on it as
`recipe-native-probe` (`Cargo.toml:27-56`, `Cargo.toml:61-84`). The lockfile
records the same direct dependency set (`Cargo.lock:959-971`).

### Build-script inputs and output

`native-probe/build.rs` is a content-addressed invalidation boundary:

1. It reads `CARGO_MANIFEST_DIR` and recursively collects every `.rs` file,
   sorted by a stable `domain/relative-path` name, from these domains:
   `recipe-native-probe/src`, `recipe-core/src`, `recipe-executor/src`,
   `recipe-host/src`, `recipe-kernel/src`, `recipe-language/src`,
   `recipe-native-executor/src`, `recipe-planner/src`, `recipe-primitives/src`,
   `recipe-scheduler/src`, `recipe-cuda/src`, `recipe-hsa/src`, and
   `recipe-probe/src` (`native-probe/build.rs:10-52`, `native-probe/build.rs:163-181`).
2. It adds `native-probe/build.rs`, the workspace `Cargo.lock` and root
   `Cargo.toml`, and the selected manifests for `native-probe`, `core`,
   `executor`, `host`, `kernel`, `language`, `native-executor`, `planner`,
   `primitives`, `scheduler`, `cuda`, `hsa`, and `probe`
   (`native-probe/build.rs:53-95`).
3. It prints `cargo:rerun-if-changed` for every file and
   `cargo:rerun-if-env-changed` for `HOST`, `TARGET`, `OPT_LEVEL`, `DEBUG`,
   `PROFILE`, `CARGO_CFG_TARGET_ARCH`, `CARGO_CFG_TARGET_OS`,
   `CARGO_CFG_TARGET_ENV`, `CARGO_CFG_TARGET_FEATURE`,
   `CARGO_ENCODED_RUSTFLAGS`, `RUSTFLAGS`, and `RUSTC`
   (`native-probe/build.rs:87-129`, `native-probe/build.rs:134-161`).
4. It hashes each field with an explicit little-endian length prefix, starting
   with the domain marker `recipe-native-probe-build-v3`. It runs the selected
   `RUSTC -Vv` and includes the executable path and stdout/stderr in the hash.
   The resulting lowercase SHA-256 is exported as
   `RECIPE_NATIVE_PROBE_SOURCE_DIGEST` (`native-probe/build.rs:102-132`).

Missing `CARGO_MANIFEST_DIR` or `RUSTC`, a failed `rustc -Vv`, an unreadable
source or manifest, or a directory traversal error fails the build script. The
digest is consumed by `identity::backend_toolchain_identity`, so changing the
probed source graph changes every native backend toolchain identity.

## Module graph

`native-probe/src/lib.rs` declares all implementation modules privately and
re-exports only the public preparation surface (`native-probe/src/lib.rs:1-20`).

```text
lib
├── config       public configuration records
├── native       NativeGpuProbe, backend dispatch, PCI DRM connector count
│   ├── cuda     CUDA Driver discovery and benchmark backend
│   └── hsa      ROCr/HSA discovery and benchmark backend
├── identity     library pins, SHA-256 identities, PCI surfaces, labels
├── benchmark    shared bounded timing, rates, FMA template, input and checks
└── bindings     exact profile reopening, CUDA/HSA realization, host config
```

The private edges are:

| module | imports from this crate | main external contracts |
| --- | --- | --- |
| `native` | `config`, `cuda`, `hsa` | `GpuDiscovery`, `GpuBenchmarkIo`, `GpuInventory`, `BoundedBenchmarkPlan` |
| `cuda` | `benchmark`, `config`, `identity`, `native::Backend` | `recipe-cuda`, `recipe-kernel`, `recipe-probe` |
| `hsa` | `benchmark`, `config`, `identity`, `native::Backend` | `recipe-hsa`, `recipe-kernel`, `recipe-probe` |
| `identity` | `config` | `recipe-core`, `recipe-kernel`, `recipe-probe`, `sha2` |
| `benchmark` | none | `recipe-core`, `recipe-probe` |
| `bindings` | `native`, `cuda::CudaBackend`, `hsa::HsaBackend` | `recipe-core`, `recipe-cuda`, `recipe-hsa`, `recipe-host`, `recipe-native-executor`, `recipe-probe` |

`benchmark` and `identity` have no public items. The root exports the two
binding functions, `NativeExecutionBindings`, all five configuration records,
and `NativeGpuProbe`; backend structs and helper types remain crate-private.

## Configuration

The records in `src/config.rs` are plain typed values. They do not parse TOML
themselves. The CLI constructs them from explicit command-line paths and fixed
candidate lists, while embedders can construct them directly
(`native-probe/src/config.rs:1-47`).

```text
NativeProbeConfig
├── host_memory_key: Label
├── pci_sysfs_root: PathBuf
├── cuda: CudaProbeConfig
│   ├── library: BackendLibrary { candidates: Vec<PathBuf> }
│   └── ptx_isa: u16              # major * 10 + minor
├── hsa: HsaProbeConfig
│   ├── library: BackendLibrary
│   └── code_object_version: u8   # must be nonzero
└── kernels: KernelBuildConfig
    ├── toolchain: OfflineToolchain
    ├── release: Label
    ├── scratch_parent: PathBuf   # must be absolute
    └── fma_chain_length: u16     # must be nonzero
```

`BackendLibrary` candidates are ordered. Every candidate must be absolute. A
missing candidate is skipped, the first existing canonical regular file is
selected, duplicate canonical targets are ignored, and the selected file is
hashed. Existing candidates after the first are still inspected, so a malformed
later candidate is an error rather than a silently ignored fallback
(`native-probe/src/identity.rs:22-73`).

### CLI defaults

The production `recipe probe` command uses these defaults in
`src/cli.rs:1747-1856`:

| setting | default |
| --- | --- |
| PCI root | `/sys/bus/pci/devices` |
| CUDA Driver candidates | `/usr/lib/x86_64-linux-gnu/libcuda.so.1`, `/usr/lib64/libcuda.so.1`, `/usr/lib/libcuda.so.1`, `/usr/local/nvidia/lib64/libcuda.so.1` |
| ROCr/HSA candidates | `/opt/rocm/lib/libhsa-runtime64.so.1`, `/usr/lib/x86_64-linux-gnu/libhsa-runtime64.so.1`, `/usr/lib64/libhsa-runtime64.so.1`, `/usr/lib/libhsa-runtime64.so.1` |
| required LLVM verifier | first existing `/usr/bin/opt`, `/usr/local/bin/opt`, `/usr/lib/llvm-22/bin/opt`, `/usr/lib/llvm-21/bin/opt`, `/usr/lib/llvm-20/bin/opt`, `/usr/lib/llvm-19/bin/opt`, `/opt/llvm/bin/opt` |
| required LLVM code generator | same fixed roots with `llc` |
| optional ELF linker | `ld.lld` in `/usr/bin`, `/usr/local/bin`, LLVM 22, 21, 20, or `/opt/llvm/bin` |
| optional PTX assembler | `/opt/cuda-11.8/bin/ptxas`, 11.7, 11.6, 11.5, 11.4, `/usr/local/cuda-11.8/bin/ptxas`, `/usr/local/cuda-11.4/bin/ptxas`, `/opt/cuda/bin/ptxas`, `/usr/local/cuda/bin/ptxas`, `/usr/bin/ptxas` |
| PTX ISA | `74` (PTX 7.4) |
| HSA code-object version | `6` |
| release label | `auto-pinned-local-tools-and-benchmark-v3` |
| FMA chain | `64` dependent f32 FMAs per element |

An explicit `--cuda-driver`, `--hsa-runtime`, or tool path replaces the
corresponding candidate list. `--llvm-opt`, `--llvm-llc` are required. `--lld`
and `--ptxas` are optional at config construction, but the backend identity
requires `ld.lld` for AMD and `ptxas` for NVIDIA once that backend exposes a
GPU (`identity.rs:115-134`). Thus an unused vendor may have no assembler, but a
present vendor cannot be discovered without its pinned backend linker/assembler.

## Production lifecycle

The exact zero-argument CLI path is:

```text
recipe probe
  -> reject container/PID-namespace execution
  -> parse topology/contract.toml or --contract
  -> create private state_root/scratch
  -> LocalSystemDiscovery::discover_host()
  -> choose first discovered RAM key as host_memory_key
  -> native_config(...)
  -> NativeGpuProbe::new(config)
  -> ProbeEngine::current_cache_identity(seed, no peers)
  -> ExplicitPathProfileCache::load(identity)
       hit: validate and reuse exact measured profile
       miss: ProbeEngine::probe_and_store(...)
  -> capture and atomically write active-native-v1 receipt
  -> print profile, source, cache/topology/discovery identities and counts
```

The CLI implementation is `src/cli.rs:876-947`. The state root is
`$XDG_CACHE_HOME/recipe-next` when `XDG_CACHE_HOME` is absolute, otherwise
`$HOME/.cache/recipe-next`; directories are canonical, owned by the effective
user, and not group/other writable (`src/cli.rs:1898-1966`). `recipe probe`
rejects `/.dockerenv`, `/run/.containerenv`, container cgroup markers, and a
nested PID namespace (`src/cli.rs:1969-1994`).

The `ProbeEngine` derives all four plans from the seed: suggested bytes are RAM
capacity divided by 1024, disk capacity divided by 16384, GPU memory divided by
1024, and Ethernet rate divided by 8. Each is clamped to 4 KiB through 64 MiB,
uses eight iterations, and has a two-second maximum duration
(`probe/src/engine.rs:241-276`). It validates exhaustive host/GPU discovery,
requires every measured property, builds measured topology and discovery
profiles, validates them, and only then stores the profile
(`probe/src/engine.rs:63-160`). The theoretical values in
`topology/contract.toml` size the first pass only; they are never emitted as
native measured rates.

## `NativeGpuProbe` orchestration

`NativeGpuProbe::new` validates the FMA chain and absolute scratch path, creates
both backend objects, and marks the inventory exhaustive. The two diagnostic
constructors intentionally create one backend and mark the inventory
non-exhaustive. They are useful for backend diagnosis but `ProbeEngine::inspect`
rejects them with `IncompleteGpuEnumeration`; they cannot produce an accepted
measured profile (`native-probe/src/native.rs:33-85`, `probe/src/engine.rs:163-182`).

`GpuDiscovery::discover_all` calls every configured backend, concatenates and
sorts descriptors by key, rejects duplicate keys, and returns the `exhaustive`
flag (`native-probe/src/native.rs:245-265`). A missing vendor accelerator is a
valid empty result. A present accelerator with a missing or broken runtime is a
fatal discovery error.

`GpuBenchmarkIo::benchmark_gpu` first rejects an unbounded plan. It rediscovers
each backend and finds the one backend whose descriptor is exactly equal to the
requested descriptor. More than one owner is an error; no owner means the GPU
identity changed or disappeared. Only then does it submit the backend benchmark
(`native-probe/src/native.rs:267-298`).

The origin string used by preparation must end in a canonical PCI BDF of the
form `dddd:bb:dd.f`, with hexadecimal domain/bus/device and function `0..=7`.
The probe uses that suffix to count enabled DRM connectors for display-headroom
accounting (`native-probe/src/native.rs:106-218`). Missing `drm` means zero
connectors; invalid names, unreadable connector directories, or a connector
state other than `enabled` or `disabled` fail discovery.

## Identity surfaces

`identity.rs` supplies the identities retained in `GpuDescriptor` and in the
profile cache key:

* `selected_library` canonicalizes and SHA-256 hashes the first existing
  candidate, while enforcing absolute paths and regular-file/symlink shape.
* `backend_toolchain_identity` hashes a version marker, backend name, release,
  target configuration, the build-script source digest, and the exact path plus
  artifact digest of `opt` and `llc`, plus `ld.lld` for AMD or `ptxas` for
  NVIDIA. It returns `recipe-owned-llvm-<backend>` with the supplied release and
  digest (`native-probe/src/identity.rs:88-142`).
* `pci_accelerator_present` reads every PCI entry's hexadecimal `vendor` and
  `class`; class major `0x03` or `0x12` counts as an accelerator. A vendor match
  is only a preflight. Runtime loading and exhaustive enumeration remain fatal
  (`native-probe/src/identity.rs:169-206`).
* `pci_surface` requires an absolute root and real device directory, resolves
  the `driver` symlink, and hashes the kernel release plus driver module version
  as the required driver surface. It optionally hashes revision, subsystem and
  VBIOS values as firmware, and PCIe link speed/width plus NUMA node as the link
  surface. Missing or permission-denied optional files are omitted and included
  in the digest through a count; a driver surface with no readable file fails
  (`native-probe/src/identity.rs:208-300`).
* `library_identity` embeds the canonical path and lowercase SHA-256 in the
  runtime identity (`native-probe/src/identity.rs:302-308`).

The profile cache identity in `recipe-probe` also includes each descriptor's
key, target, capacity hint, driver/runtime/firmware/link identities, toolchain
identity, queue/concurrency/geometry limits, overlap flag, duplex, and transfer
lane counts (`probe/src/engine.rs:575-668`). A machine, device, driver,
runtime-ABI, firmware, link, or artifact-toolchain change therefore forces a
new profile.

## CUDA backend

### Opening and describing a device

`CudaBackend::open` checks for a vendor `0x10de` accelerator under the configured
PCI root. With no such function it returns `None`. With one, it requires an
existing configured candidate, requires a UTF-8 canonical path, loads
`recipe_cuda::Driver`, and performs exhaustive `cuInit`, version, device-count,
UUID, PCI, memory, and attribute discovery (`native-probe/src/cuda.rs:66-107`).

For each device, `descriptor` parses the Driver's
`domain:bus:device.function`, checks its numeric domain/bus/device against
Driver attributes, canonicalizes it to lowercase sysfs spelling, and hashes the
PCI surfaces. It validates an `NvidiaTarget` made from Driver compute
capability plus configured PTX ISA. The resulting fields are:

| field | value |
| --- | --- |
| key | `cuda:<Driver UUID>@<lowercase sysfs BDF>` |
| target backend and ABI | `nvidia-cuda-driver`, `elf64-cubin` |
| target architecture | `sm_<major><minor>` from compute capability |
| capacity hint | Driver total memory bytes, nonzero |
| driver identity | `cuda-kernel-driver:<raw driver version>:<PCI driver surface>` |
| runtime identity | `cuda-driver-api:<raw driver version>:<hashed library>` |
| transport and duplex | PCIe, full duplex |
| transfer lanes | one host-to-device and one device-to-host lane |
| asynchronous and concurrency | asynchronous, one concurrent task |
| submission queues | `CUDA_MAXIMUM_SUBMISSION_QUEUES`, currently 32 |
| geometry | Driver warp size, max threads per block, max shared memory per block |
| transfer overlap | true only when async-engine count is nonzero and concurrent kernels is true |

(`native-probe/src/cuda.rs:109-208`; the queue ceiling is
`native-executor/src/cuda.rs:30-33`). A non-UTF-8 path, malformed PCI string,
attribute disagreement, target validation failure, missing required tool,
zero memory, or missing PCI identity surface is a discovery failure.

### CUDA benchmark

`benchmark_device` reopens the Driver and matches the requested descriptor
exactly once. It creates a yield-scheduling context and nonblocking stream,
allocates two pinned host buffers and two device buffers for the complete plan
size, and rejects a buffer larger than Driver-reported total memory
(`native-probe/src/cuda.rs:210-268`). It then measures and verifies, in order:

1. timed host-to-device copies;
2. timed device-to-host copies and byte-for-byte host verification;
3. timed device-to-device copies and a verification download;
4. a Recipe-owned dependent-f32-FMA kernel, compiled during `BuildPhase::Realize`
   for the exact SM/PTX target, loaded from an inspected cubin, launched with
   the inspected one-input/one-output/i64-elements ABI, and downloaded for
   finite-changed-output verification.

The calculation path lowers `fma_template` with a power-of-two workgroup no
larger than the smaller of the device limit and element count. It requires a
`BuiltArtifact::Cubin`, the exact entry symbol `recipe_probe_fma_f32`, and a
grid count that fits `u32` (`native-probe/src/cuda.rs:346-446`). Rates and
capacity are wrapped in `PropertyProvenance::Measured`.

CUDA completion polling starts at 50 microseconds and doubles up to 2 ms. If a
plan deadline is reached, CUDA cannot cancel the submission, so the code keeps
polling at a 10 ms to 100 ms capped backoff until the token completes, then
returns a deadline error. It never returns a live token while its borrowed
buffers can unwind (`native-probe/src/cuda.rs:466-518`). Poll, allocation,
module, launch, ABI, verification, overflow, and artifact errors are all
`ProbeError::Benchmark` failures.

## HSA backend

### Runtime and descriptor

`HsaBackend::new` rejects code-object version zero and retains the configured
library, host-memory key, PCI root, kernel config, and an initially empty
`RefCell<Option<HsaRuntimeState>>` (`native-probe/src/hsa.rs:67-110`).

`with_runtime` performs the same vendor preflight for AMD `0x1002`. No AMD
accelerator and no initialized runtime returns `Ok(None)`. A matching accelerator
requires a configured ROCr library, loads and initializes exactly one retained
`recipe_hsa::Runtime`, and rejects a library identity change or disappearance
after initialization (`native-probe/src/hsa.rs:112-149`).

The descriptor path accepts only GPU agents that support kernel dispatch, expose
a value UUID, exact PCI address, AMD properties, a queue capability, a stable
wavefront width, ISA limits, and a KFD node with readable nonzero LDS capacity.
`exact_target` requires every ISA to expose an AMDGPU target. One specific
non-generic target is preferred; one shared generic target is accepted only when
all targets agree; multiple specific or ambiguous generic targets fail
(`native-probe/src/hsa.rs:153-237`, `native-probe/src/hsa.rs:633-683`).

The resulting descriptor has:

| field | value |
| --- | --- |
| key | `hsa:<ROCr UUID>@<PCI address>` |
| target backend and ABI | `amd-rocr-hsa`, `elf64-amdgpu-code-object-v<configured version>` |
| architecture | exact `amdgcn-amd-amdhsa--...` ISA string |
| capacity hint | largest allocatable coarse-grained GPU global pool, otherwise nonzero AMD available memory |
| driver identity | `amdgpu-kfd-node-<node>:<PCI driver surface>` |
| runtime identity | HSA and AMD extension versions plus hashed ROCr library |
| transport and duplex | PCIe, full duplex |
| transfer lanes | one host-to-device and one device-to-host lane |
| asynchronous and concurrency | asynchronous, one concurrent task |
| submission queues | ROCr-reported maximum queues |
| geometry | first ISA wavefront, minimum per-ISA workgroup limit, KFD `lds_size_in_kb * 1024` |
| transfer overlap | true when `sdma_engine_count != 0` |

(`native-probe/src/hsa.rs:238-293`). `capacity` rejects zero. A local coarse
pool is selected only when it is global, GPU-located, runtime-allocatable, and
coarse-grained; otherwise the AMD available-memory value must be nonzero
(`native-probe/src/hsa.rs:685-725`).

### HSA benchmark

The benchmark rediscoveries the exact descriptor and UUID, selects the GPU's
NUMA node and a CPU agent, opens a GPU session, and checks the complete plan
buffer against the selected capacity. It allocates fine-grained CPU source and
destination plus coarse-grained GPU source and destination, grants access in
both directions, fills deterministic input, and measures/verifies H2D, D2H, and
D2D copies (`native-probe/src/hsa.rs:307-453`).

The calculation path lowers the same FMA template to an inspected HSACO for the
exact AMD target and code-object version, loads it, checks ROCr kernarg size
against inspected metadata, and writes the explicit ABI fields at offsets 0,
8, and 16: input pointer, output pointer, and element count. It creates a
single-producer queue using the discovered minimum packet count, dispatches a
one-dimensional geometry, downloads and verifies output, and closes the queue,
kernel, executable, and kernarg before returning the measured FLOP rate
(`native-probe/src/hsa.rs:455-568`).

HSA completion uses the same 50 microsecond to 2 ms benchmark backoff and 10 ms
to 100 ms timed-out cleanup backoff. A live token retains signals, allocations,
and executable resources until it completes, so timeout cleanup is deliberately
awaited rather than abandoned (`native-probe/src/hsa.rs:790-851`). A missing
CPU memory agent, missing queue/ISA, duplicate UUID, capacity/allocation/access
failure, kernarg-size mismatch, dispatch failure, invalid pointer/offset, close
failure, verification failure, or artifact failure is a benchmark error.

For benchmark-only agent selection, the first CPU agent on the GPU NUMA node is
preferred, otherwise the first CPU fallback is used. Binding realization is
stricter and rejects ambiguity; see the next section
(`native-probe/src/hsa.rs:738-768`).

## Shared benchmark contract and limits

`benchmark.rs` is the only common timing and kernel-work implementation:

* Every plan must be bounded: nonzero buffer, iteration count, and duration.
  `time_bounded` submits at most `plan.iterations`, passing the remaining
  duration to each operation, and rejects zero completed work.
* Transfer rates count `buffer_bytes * iterations`; calculation rates count
  `FlopCount * iterations`. Work counters, the nanosecond conversion, and the
  final `u64` rate are checked for overflow (`native-probe/src/benchmark.rs:20-82`).
* Transfers use the full `plan.buffer_bytes`, converted to `usize`. Calculation
  buffers use at most `MAXIMUM_COMPUTE_BYTES = 4 * 1024 * 1024`, are aligned down
  to four-byte f32 elements, and must retain at least one f32
  (`native-probe/src/benchmark.rs:11-12`, `native-probe/src/benchmark.rs:84-93`).
* `fma_template` creates a one-dimensional contiguous f32 input and output,
  forbids input/output aliasing, and emits the configured nonzero number of
  dependent FMA instructions. Constants are the f32 bit patterns for
  `1.0009766` and `0.00012207031`; the fixed template ID is
  `0x0050_524f_4245` (`native-probe/src/benchmark.rs:95-160`).
* `fill_input` writes finite deterministic binary32 values. Verification only
  proves equal buffer lengths, at least four bytes, a finite first output, and
  a changed first output bit pattern. It is a smoke-level kernel check, not a
  full numerical oracle (`native-probe/src/benchmark.rs:163-185`).
* `capacity(0)` is rejected. Successful rates and capacities are explicitly
  marked `PropertyProvenance::Measured` (`native-probe/src/benchmark.rs:187-203`).

The native benchmark therefore measures actual Driver/ROCr submissions and
copies. Seed estimates bound the first workload only; they do not overwrite the
returned measurements.

## Preparation-scoped native bindings

`NativeExecutionBindings<'cuda, 'hsa>` contains a machine ID plus vectors of
borrowed `CudaBinding` and `HsaBinding` values. Its getters expose slices or
consume into `(Vec<CudaBinding>, Vec<HsaBinding>)`. The lifetimes enforce that a
CUDA context and HSA session cannot outlive the callback that owns the runtime
scope (`native-probe/src/bindings.rs:22-47`).

### Host backend configuration

`host_backend_config_from_inventory` performs no I/O. It maps every resolved
RAM origin to `HostDeviceBinding::Ram`, then maps every storage origin to a
`DiskFileSpec` under its discovered `benchmark_root` with the deterministic
run-scoped name:

```text
.recipe-run-<RunId>-device-<DeviceId>-arena
```

It returns `HostBackendConfig::new(worker_threads, staging_bytes_per_worker,
bindings)`. The host backend later creates disk arenas with `create_new`, so an
overlapping run ID fails rather than overwriting a file
(`native-probe/src/bindings.rs:87-118`).

### Exact reopening algorithm

`with_native_execution_bindings` is the sole public native-handle reopening
boundary (`native-probe/src/bindings.rs:120-234`):

1. Discover all current GPU descriptors through the supplied probe and resolve
   the current host/GPU inventory against the supplied `MeasuredProfile`.
   Resolution uses exact machine and origin keys, never ordinal, name,
   capacity, or performance fallback.
2. Partition expected GPUs by the exact target backend strings
   `nvidia-cuda-driver` and `amd-rocr-hsa`, and recount current DRM connectors
   from each origin's PCI BDF.
3. Reopen CUDA through the existing backend. Every current CUDA descriptor must
   be present in the measured key map exactly once. A context is created with
   default `ContextFlags`; the binding retains deployment identity, measured
   queue limit, and connector count.
4. Reopen HSA through the backend's retained runtime, rediscover all agents, and
   realize every measured GPU exactly once. CPU agents are retained only when
   they expose a runtime-allocatable global pool with kernarg initialization and
   fine-grained or extended-fine-grained flags. A GPU selects exactly one CPU
   allocator: one same-NUMA match, or one total fallback. Zero, multiple
   same-NUMA, or multiple fallback allocators fail. The binding retains exact
   target string, code-object version, queue packet size, queue limit, and
   connector count.
5. Invoke the higher-ranked callback with the borrowed vectors. If no measured
   HSA GPU exists, an absent HSA runtime is allowed and the callback receives an
   empty HSA vector. If measured HSA origins exist, absence or disappearance of
   the HSA backend is fatal.

The callback result is returned only after bindings are dropped. A changed
library identity, missing expected device, duplicate device, machine mismatch,
unsupported target backend, missing HSA allocator, or incomplete reopening is a
`ProbeError::Discovery` failure. There is no ordinal, product-name, capacity, or
performance fallback (`native-probe/src/bindings.rs:236-482`).

## Consumers and handoff

The root `recipe` crate imports the config records and `NativeGpuProbe` in
`src/cli.rs:20-28`. `run_probe` constructs `ProbeEngine` with native discovery
and native GPU benchmarks (`src/cli.rs:889-923`). The root also exposes the
preparation API from `src/native_prepare.rs` through `src/lib.rs:17-22`.

`src/native_prepare.rs` is the main consumer of the binding bridge. It:

* loads an identity-named measured profile and rejects malformed, insecure,
  stale, absent, or invalid profiles;
* calls `NativeGpuProbe::new`, exact discovery, profile resolution, and
  `with_native_execution_bindings`;
* verifies the complete local GPU scope and exact binding set;
* builds one `TargetBuildSpec` per equivalent target, preserving one device entry
  per measured GPU; and
* lends `NativePreparationScope` to preparation callbacks without storing
  dynamic handles in a declaration (`src/native_prepare.rs:248-365`,
  `src/native_prepare.rs:543-619`).

`with_current_native_preparation` obtains the active-native-v1 receipt, validates
the current host RAM origin, caches one `(NativeProbeConfig, NativeGpuProbe)` per
thread, reloads the exact profile, and rejects a changed config after runtime
initialization (`src/native_prepare.rs:368-411`). The receipt is written by the
CLI as a canonical tab-delimited `recipe-active-native-v1` file containing the
profile identity, PCI root, scratch path, optional backend library pins, LLVM
and linker/assembler pins, PTX/code-object settings, release label, and FMA
chain (`src/cli.rs:1123-1246`). Every pinned native file is re-inspected by
canonical path and SHA-256 before reuse (`src/cli.rs:1249-1279`,
`src/cli.rs:1335-1376`).

Training and inference consume `with_current_native_preparation` and convert
the scope into a host backend, cross-backend bridge, `LocalCandidateFactory`,
`NativeExecutorDriver`, deferred compiler, and realizer. Inference does this at
`src/inference.rs:602-658`; training does it at `src/training.rs:1278-1325`.
The acceptance runner records the measured profile, device origins, target
identities, and toolchain identities through the same callback
(`acceptance/src/main.rs:181-209`).

`recipe-native-executor` receives the resulting `CudaBinding` and `HsaBinding`.
Those bindings retain only borrowed contexts/sessions, exact deployment/target
identity, queue limits, display connector counts, and live available-memory
queries. The executor owns arenas, modules, queues, and completion tokens after
preparation; it does not rediscover hardware in the loop
(`native-executor/src/cuda.rs:30-92`, `native-executor/src/hsa.rs:31-99`).

## Failure and limit matrix

| stage | observed condition | result |
| --- | --- | --- |
| build | missing manifest/RUSTC, failed `rustc -Vv`, unreadable included file | build-script error |
| config | zero FMA chain, relative scratch parent, zero HSA code-object version | `ProbeError::Discovery` from constructor/backend validation |
| library selection | relative path, malformed existing candidate, canonicalization/read failure | discovery error |
| vendor preflight | no matching PCI accelerator | backend returns no devices; missing library is not an error |
| vendor preflight | matching PCI accelerator but no configured runtime | discovery error |
| CUDA open | non-UTF-8 path, Driver load/init/discovery failure | discovery error |
| HSA open | ROCr load/init/discovery failure or post-init library/hardware identity change | discovery error |
| PCI identity | malformed or contradictory BDF, missing device/driver surface, invalid DRM connector state | discovery error |
| descriptor | missing stable UUID, PCI address, AMD properties, queue, ISA, KFD LDS, or valid target | discovery error |
| inventory | duplicate key or non-exhaustive diagnostic probe | discovery error or `IncompleteGpuEnumeration` in `ProbeEngine` |
| benchmark plan | zero bytes/iterations/duration, non-fitting `usize`, no completed work | benchmark error |
| CUDA benchmark | allocation exceeds total memory, copy/launch/poll/module/ABI/verification failure | benchmark error; timed-out work is drained then reported |
| HSA benchmark | allocation/access/queue/kernarg/dispatch/close/verification failure | benchmark error; timed-out work is drained then reported |
| profile | unmeasured property, invalid topology/discovery, stale cache identity | profile or missing-measurement error |
| reopening | machine/key set changed, expected GPU missing, duplicate backend owner, changed library/tool | discovery/identity failure, rerun `recipe probe` |
| HSA host allocator | no allocator, multiple same-NUMA allocators, or multiple fallback allocators | binding discovery failure |
| preparation | measured local scope includes nonlocal calculation GPU, binding set differs, target/toolchain policy differs | `NativePreparationError::IdentityMismatch` |

The implementation deliberately does not catch these failures and downgrade
them to backend absence. Only a vendor with no matching PCI accelerator is
optional. Once hardware is present, every required runtime, identity surface,
toolchain, descriptor field, benchmark, and exact binding must succeed.

## Source map

| concern | implementation |
| --- | --- |
| public exports and crate description | `native-probe/src/lib.rs:1-20` |
| typed configuration | `native-probe/src/config.rs:1-47` |
| source digest and rebuild invalidation | `native-probe/build.rs:1-181` |
| shared timing, FMA workload, verification | `native-probe/src/benchmark.rs:1-204` |
| library, toolchain, PCI, and surface identity | `native-probe/src/identity.rs:1-308` |
| backend aggregation and DRM connectors | `native-probe/src/native.rs:1-299` |
| CUDA Driver descriptor and benchmark | `native-probe/src/cuda.rs:1-590` |
| ROCr/HSA descriptor and benchmark | `native-probe/src/hsa.rs:1-875` |
| scoped bindings and host config | `native-probe/src/bindings.rs:1-482` |
| CLI config, probe command, and receipt | `src/cli.rs:20-28`, `src/cli.rs:841-947`, `src/cli.rs:1011-1568`, `src/cli.rs:1747-1896` |
| profile planning and validation | `probe/src/model.rs:81-173`, `probe/src/engine.rs:32-276` |
| exact current-origin resolution | `probe/src/resolve.rs:52-114` |
| root preparation and consumers | `src/native_prepare.rs:248-411`, `src/inference.rs:602-658`, `src/training.rs:1278-1325` |

## Structural checks

The crate compiled in this checkout with:

```text
cargo check -p recipe-native-probe
Finished `dev` profile [optimized + debuginfo]
```

The production command exposes the probe entry point through the root binary:

```text
cargo run --bin recipe -- probe --help
Usage:
        recipe run FILE.rs   [ARGS...]
        recipe probe          [OPTIONS]
        recipe convert INPUT OUTPUT
```

These are structural and CLI-surface checks. They do not replace the required
real bare-metal run with a matching CUDA or HSA runtime, pinned offline
toolchain, and real device. The acceptance package explicitly treats missing
hardware or tools as unsuccessful rather than skipped.
