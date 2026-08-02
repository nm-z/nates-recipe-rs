---
file: native-probe/src/hsa.rs
crate: recipe-native-probe
package_version: 0.1.0
module: native-probe/src/hsa.rs
backend_id: amd-rocr-hsa
pci_vendor: 0x1002
runtime_boundary: recipe-hsa
scope: discovery, bounded_gpu_benchmark, preparation_binding
role: rocr_hsa_gpu_discovery_and_benchmark_backend
intent: >
  Convert the live ROCr/HSA topology into exact GPU descriptors, measure
  transfers and a Recipe-owned HSACO calculation, and lend exact sessions to
  the preparation callback without fallback selection.
authority:
  - native-probe/src/hsa.rs
  - native-probe/src/native.rs
  - native-probe/src/bindings.rs
  - native-probe/src/identity.rs
  - native-probe/src/benchmark.rs
  - hsa/src/runtime.rs
  - hsa/src/discovery.rs
  - hsa/src/session.rs
  - hsa/src/execution.rs
---

# ROCr/HSA native probe backend

This document describes the AMD backend in `native-probe/src/hsa.rs`. It is an
implementation map, not a hardware acceptance result. The backend discovers
AMD GPUs through the reviewed `recipe-hsa` ROCr boundary, measures the exact
device with real asynchronous copies and a Recipe-owned HSACO kernel, and lends
scoped sessions to native preparation. It never substitutes a CPU path, an
ordinal device, an ambiguous generic target, or a different runtime after an
identity mismatch. A generic target is accepted only when it is the sole
target identity exposed by the GPU.

## Source index

The source ranges below are the authoritative implementation anchors.

| Area | Source |
| --- | --- |
| Constants and completion backoff | `native-probe/src/hsa.rs:L32-L65` |
| Backend state and construction | `native-probe/src/hsa.rs:L67-L151` |
| Descriptor construction | `native-probe/src/hsa.rs:L153-L294` |
| Device benchmark entry and rediscovery | `native-probe/src/hsa.rs:L296-L453` |
| Recipe-owned calculation benchmark | `native-probe/src/hsa.rs:L455-L569` |
| KFD LDS capacity and `Backend` implementation | `native-probe/src/hsa.rs:L571-L631` |
| Exact target selection | `native-probe/src/hsa.rs:L633-L683` |
| Capacity, agent, and workgroup helpers | `native-probe/src/hsa.rs:L685-L788` |
| Copy, completion, ABI, and error helpers | `native-probe/src/hsa.rs:L790-L875` |
| Generic probe ownership and exhaustive inventory | `native-probe/src/native.rs:L21-L31`, `L239-L291` |
| Preparation-scoped ROCr bindings | `native-probe/src/bindings.rs:L128-L230`, `L319-L416` |
| HSA target build specification | `src/native_prepare.rs:L543-L619`, `L705-L745` |
| CLI defaults and path overrides | `src/cli.rs:L841-L865`, `L1747-L1857` |

## Role and ownership

`HsaBackend` is an internal `recipe-native-probe` backend. It implements the
crate-private `Backend` trait with `discover` and `benchmark`; the public
`NativeGpuProbe` owns one HSA backend in its `NativeBackend::Hsa` variant.
`NativeGpuProbe::new` enables both CUDA and HSA and marks the inventory
exhaustive. `NativeGpuProbe::hsa_diagnostic` enables only HSA and deliberately
marks it non-exhaustive. The generic `ProbeEngine` rejects a non-exhaustive
GPU inventory before it benchmarks anything, so the diagnostic constructor is
not a profile-construction path (`native-probe/src/native.rs:L40-L84`,
`probe/src/engine.rs:L163-L182`).

The backend stores configuration and one lazily opened runtime:

| Field | Meaning |
| --- | --- |
| `library` | Ordered `BackendLibrary` candidates for ROCr/HSA. |
| `host_memory_key` | Host RAM origin copied into each `GpuDescriptor`. |
| `pci_sysfs_root` | Absolute PCI sysfs root used for vendor preflight and PCI identity surfaces. |
| `code_object_version` | Nonzero AMDGPU code-object version used in target, ABI, lowering, and HSACO checks. |
| `kernels` | Pinned LLVM toolchain, release label, private scratch directory, and FMA-chain length. |
| `runtime` | `RefCell<Option<HsaRuntimeState>>`; the retained `Runtime` and the `PinnedLibrary` it was opened from. |

The HSA runtime is not process-global. `recipe-hsa::Runtime` keeps the dynamic
library and its `hsa_init` reference alive while discovery records, sessions,
queues, allocations, executables, and completion tokens borrow it. Rust
lifetimes prevent shutdown while those children are still live
(`hsa/src/runtime.rs:L10-L18`, `L34-L89`).

## Configuration and identity inputs

`NativeProbeConfig` supplies `HsaProbeConfig { library, code_object_version }`
and shared `KernelBuildConfig` (`native-probe/src/config.rs:L22-L47`). The
normal CLI configuration currently uses:

* ROCr candidates, in order, `/opt/rocm/lib/libhsa-runtime64.so.1`,
  `/usr/lib/x86_64-linux-gnu/libhsa-runtime64.so.1`,
  `/usr/lib64/libhsa-runtime64.so.1`, and `/usr/lib/libhsa-runtime64.so.1`;
* code-object version `6`;
* pinned `opt` and `llc`, with `ld.lld` required when an HSA target is used;
* release label `auto-pinned-local-tools-and-benchmark-v3`;
* a private scratch directory and a dependent FMA chain of `64` operations.

These are defaults in `src/cli.rs:L1747-L1857`, not hidden values in the HSA
backend. `--hsa-runtime`, `--llvm-opt`, `--llvm-llc`, and `--lld` replace the
candidate lists or tool paths. Every tool is captured as a canonical path plus
SHA-256 digest by `PinnedTool` before it enters the configuration.

`NativeGpuProbe::new` first rejects a zero FMA chain or a non-absolute scratch
parent (`native-probe/src/native.rs:L221-L234`), then `HsaBackend::new` rejects
a zero code-object version (`native-probe/src/hsa.rs:L95-L110`). A valid
constructor has not opened ROCr and has not touched hardware yet.

### Library selection

`identity::selected_library` is shared by CUDA and HSA
(`native-probe/src/identity.rs:L22-L73`):

1. Every candidate must be absolute. A missing candidate is skipped.
2. Existing candidates must be a regular file or symlink. The symlink target
   is canonicalized and must be a regular file.
3. Canonical targets are de-duplicated. The first unique existing target is
   selected, and its bytes are hashed into `PinnedLibrary`.
4. Read, canonicalization, metadata, and invalid-file failures are hard
   discovery errors. The function does not silently fall through to a later
   candidate after an existing candidate fails inspection.

The selected canonical path and digest form `rocr-library:<path>:sha256=<hex>`
through `library_identity` (`native-probe/src/identity.rs:L302-L307`). The
runtime identity is therefore part of the descriptor and of every profile
that contains an HSA device.

### Toolchain identity

`backend_toolchain_identity` hashes a domain tag, backend name `amd-hsa`, the
release label, an HSA target configuration string, the generated
`RECIPE_NATIVE_PROBE_SOURCE_DIGEST`, and the canonical path plus pinned digest
of `opt`, `llc`, and `ld.lld` (`native-probe/src/identity.rs:L88-L142`). For HSA,
the target configuration is:

```text
<target tail>:code-object-v<version>:dependent-f32-fma-chain-<length>
```

The source digest is generated by `native-probe/build.rs` from the source and
Cargo inputs for `recipe-native-probe`, `recipe-hsa`, `recipe-kernel`, and the
other native crates (`native-probe/build.rs:L10-L136`). Changing the backend
source, compiler binary, target, code-object version, or FMA benchmark shape
therefore changes the descriptor's `ToolchainIdentity`; a cached measured
profile cannot silently retain the old identity.

## Runtime opening and retention

All HSA backend operations pass through `HsaBackend::with_runtime`
(`native-probe/src/hsa.rs:L112-L149`). Its state machine is:

```text
check PCI for an AMD accelerator (vendor 0x1002)
  no matching accelerator + runtime unopened -> Ok(None)
  no matching accelerator + runtime already open -> discovery error
check and pin the configured ROCr library
  no library + runtime unopened -> discovery error when AMD hardware exists
  no library + runtime already open -> discovery error
  different library identity after opening -> discovery error
open Runtime once when state is empty
invoke the operation with (&PinnedLibrary, &Runtime)
```

`pci_accelerator_present` requires an absolute root, reads every PCI entry's
`vendor` and `class`, and recognizes class-code high bytes `0x03` or `0x12`
(`native-probe/src/identity.rs:L169-L206`). Enumeration, file, or hexadecimal
parse errors are not treated as absence. This makes an installed ROCr library
irrelevant on a machine with no AMD accelerator, while making a missing or
broken runtime fatal once matching PCI hardware exists.

`recipe-hsa::Runtime::open` uses `dlopen(..., RTLD_NOW | RTLD_LOCAL)`, resolves
the complete reviewed ROCr symbol set, then calls `hsa_init`
(`hsa/src/loader.rs:L22-L89`, `L132-L270`; `hsa/src/runtime.rs:L34-L50`). A
library-open error, missing symbol, invalid path, or failed `hsa_init` is
wrapped as a `ProbeError::Discovery` by `with_runtime`. The runtime remains in
`HsaRuntimeState` until the backend is dropped. The callback cannot outlive the
borrowed runtime, and `with_runtime` never creates a second runtime for the
same backend.

## Discovery flow

`HsaBackend::discover` calls `with_runtime`, invokes
`Runtime::discover`, and visits every `DiscoveredAgent`. Non-GPU agents return
`Ok(None)` and are ignored. Every GPU is converted by `descriptor`; the first
descriptor error aborts the exhaustive discovery rather than dropping one
agent (`native-probe/src/hsa.rs:L608-L626`). `recipe-hsa` discovery itself
reads system versions, all agents, all GPU properties, all ISA records, and
all memory pools. Callback panics, invalid raw attributes, invalid UTF-8,
inconsistent queue limits, allocation failures while collecting records, and
ROCr status failures become typed errors (`hsa/src/discovery.rs:L154-L188`,
`L191-L457`, `L759-L884`).

### GPU eligibility gates

`descriptor` applies these gates in order (`native-probe/src/hsa.rs:L153-L237`):

1. `device_type` must be `Gpu`; other agent types are a normal skip.
2. The feature bits must advertise kernel dispatch.
3. The agent UUID must contain a real `Value`, not `XX`/`Unavailable`.
4. ROCr must provide an exact PCI address.
5. The PCI address must have a readable driver, firmware, and PCIe-link
   identity surface through `pci_surface`.
6. `exact_target` must choose one unambiguous AMD ISA target.
7. AMD GPU capability properties and physical queue capabilities must exist.
8. The configured HSA toolchain identity must be constructible, which requires
   a pinned ELF linker.
9. The first ISA wavefront width must be present.
10. At least one ISA record must exist so a workgroup limit can be derived, and
    the KFD node must provide a nonzero LDS capacity. A zero workgroup limit is
    rejected when the benchmark chooses dispatch geometry.

No product-name, capacity, target, or UUID fallback is used to pass one of
these gates.

### Exact target selection

`exact_target` first requires every ISA record to carry an AMD target. A
non-AMDGPU ISA on a GPU is an error. It then records all target strings in a
set and counts the target entries whose architecture does not end in
`-generic` (`native-probe/src/hsa.rs:L633-L683`). The decision is:

| ISA records | Result |
| --- | --- |
| Exactly one non-generic entry | Select that entry. |
| No non-generic entries and exactly one distinct target string | Select the sole target, including a generic target. |
| No target strings | Discovery error. |
| More than one non-generic entry | Discovery error for multiple non-generic artifact targets. The count is over entries, so repeated entries are not silently collapsed. |
| Multiple generic target strings and no specific target | Discovery error for ambiguous generic artifact targets. |

`hsa_target_tail` then requires the complete target to start with
`amdgcn-amd-amdhsa--` and returns the suffix such as `gfx1101` or
`gfx1101:feature+` (`native-probe/src/hsa.rs:L727-L736`). The complete target,
including ordered feature modifiers, remains the descriptor architecture and
the runtime binding target. Only the suffix is passed to `recipe_kernel::AmdTarget`.

### Capacity and physical limits

`hsa_capacity` prefers the largest discovered pool satisfying all of:

```text
segment == Global
location == Some(Gpu)
runtime_allocation.is_some()
global_flags contains COARSE_GRAINED
```

If no such local pool exists, it uses ROCr's AMD available-memory counter. A
pool size that cannot fit `u64`, a zero fallback counter, or no AMD capability
record is an error (`native-probe/src/hsa.rs:L685-L725`). The value becomes
both the descriptor capacity hint and the benchmark's capacity measurement.

`maximum_workgroup_lanes` is the minimum, over all ISAs, of the ISA's
`maximum_workgroup_size` and its first-dimensional maximum
(`native-probe/src/hsa.rs:L217-L230`). The backend reads shared LDS capacity
from `/sys/class/kfd/kfd/topology/nodes/<driver_node_id>/properties`, finds a
line exactly shaped as `lds_size_in_kb <integer>`, multiplies by 1024 with
overflow checking, and rejects missing, malformed, zero, or overflowing data
(`native-probe/src/hsa.rs:L571-L606`). This read supplies a measured scheduling
surface; it does not create a competing KFD runtime or queue path.

### Descriptor fields

For a valid GPU, `descriptor` emits one `GpuDescriptor`
(`native-probe/src/hsa.rs:L238-L293`):

| Field | HSA value |
| --- | --- |
| `key` | `hsa:<ROCr UUID>@<canonical PCI BDF>`. |
| `name` | AMD product name, or ROCr agent name when the product name is blank. |
| `host_memory_key` | The configured host RAM origin. |
| `target.backend` | `amd-rocr-hsa`. |
| `target.architecture` | Complete selected `IsaTarget::as_str()`. |
| `target.abi` | `elf64-amdgpu-code-object-v<code_object_version>`. |
| `capacity_hint` | `hsa_capacity(description)`. |
| `driver` | `amdgpu-kfd-node-<ROCr driver node>:<PCI driver surface digest>`. |
| `runtime_abi` | `hsa-<HSA major>.<minor>-amdext-<major>.<minor>:<ROCr library identity>`. |
| `firmware` | PCI firmware surface digest. |
| `link_identity` | PCIe link and NUMA surface digest. |
| `transport_kind` and `duplex` | `Pcie` and `Full`. |
| transfer lane limits | One host-to-device lane and one device-to-host lane. |
| submission properties | Asynchronous submission, ROCr maximum queue count, one maximum concurrent task. |
| parallelism properties | First ISA wavefront width, minimum workgroup lanes, KFD LDS capacity. |
| `transfer_overlaps_calculation` | `true` exactly when `sdma_engine_count != 0`. |
| `toolchain` | The identity described in the toolchain section above. |

## Bounded measurement flow

`NativeGpuProbe::benchmark_gpu` first rejects an unbounded plan. It asks every
enabled backend to rediscover and compare descriptors for an exact owner. Zero
owners means the identity changed; more than one owner is an ambiguity. Only
the sole exact owner receives the benchmark (`native-probe/src/native.rs:L267-L291`).
For HSA, `benchmark_device` additionally requires `with_runtime` to return a
measurement. A backend that disappears between ownership lookup and the HSA
benchmark returns `ROCr/HSA backend disappeared before benchmark`
(`native-probe/src/hsa.rs:L296-L305`).

### Rediscovery and exact agent pairing

`benchmark_with_runtime` performs a fresh `Runtime::discover`, rebuilds every
GPU descriptor against the fresh `SystemDescription`, and collects UUIDs only
when the descriptor is exactly equal to the expected measured descriptor. It
requires exactly one matching UUID. A missing, changed, or ambiguous GPU is a
benchmark error (`native-probe/src/hsa.rs:L307-L337`).

The owned agent list is then searched for that UUID and its NUMA node. The
`select_agents` helper enforces a unique GPU UUID and chooses one CPU agent:

1. Prefer the first CPU agent on the GPU's NUMA node.
2. If none is local, use the first CPU agent anywhere.
3. Duplicate GPU UUIDs, a missing GPU, or no CPU agent are errors.

The helper does not choose by product name or capacity and does not inspect
allocator flags. Allocator suitability is proven by the allocation operations
that follow (`native-probe/src/hsa.rs:L738-L768`).

The selected GPU is converted into a `recipe-hsa::Session`. That operation
rechecks GPU type, kernel-dispatch support, exact AMD ISA identities, and
runtime liveness (`hsa/src/session.rs:L136-L201`). The requested transfer size
is converted to `usize` and `u64`, then compared with `hsa_capacity`; a bounded
buffer larger than the discovered capacity fails before allocation
(`native-probe/src/hsa.rs:L338-L357`).

### Copy benchmark and verification

The benchmark allocates four buffers:

| Allocation | Owner and pool |
| --- | --- |
| `host_source`, `host_destination` | Selected CPU agent, fine-grained global pool. |
| `device_source`, `device_destination` | Selected GPU session, coarse-grained global pool. |

It grants GPU access to both host allocations and CPU access to both device
allocations. ROCr treats an access grant as replacement of the direct-access
set, so these grants are required before submission. The `recipe-hsa`
allocation API rejects zero sizes, non-allocatable pools, over-large pools,
cross-runtime grants, and out-of-range copies (`hsa/src/execution.rs:L525-L621`,
`L623-L708`).

The host source is filled with deterministic f32 bit patterns. The unsafe host
write is valid because `allocate_fine` selected a host-accessible coherent
pool and no device operation has been published yet
(`native-probe/src/hsa.rs:L360-L391`). Three bounded measurements are made:

| Measurement | Submitted operation | Verification |
| --- | --- | --- |
| Host to device | `device_source <- host_source` | Included in the later device-to-host comparison. |
| Device to host | `host_destination <- device_source` | Read after terminal completion and compare all bytes with the source. |
| Device to device | `device_destination <- device_source` | Download `device_destination` through `copy_hsa`, read host bytes, and compare all bytes. |

Each submission is asynchronous and completes through `complete_hsa`. A
failed byte comparison is a benchmark error, not a degraded measurement.
`copy_hsa_to_host` is only used after system-scope completion and only on a
fine-grained host allocation (`native-probe/src/hsa.rs:L393-L431`, `L790-L812`).

`time_bounded` executes at most `plan.iterations` submissions and stops at the
plan deadline. It rejects a plan that is not bounded, zero timed iterations,
zero elapsed time, arithmetic overflow in work counters, and rates that do not
fit the core rate types (`native-probe/src/benchmark.rs:L16-L94`). The returned
rates count bytes per completed iteration divided by elapsed wall-clock time.

### Recipe-owned FLOP kernel

The calculation measurement uses a separate capped and f32-aligned view of the
plan buffer. `compute_buffer_bytes` caps it at 4 MiB, rounds down to a four-byte
f32 boundary, and rejects an empty result (`native-probe/src/benchmark.rs:L96-L111`).
The element count must fit `u32`.

`hsa_workgroup_lanes` computes the minimum valid one-dimensional workgroup
limit across all ISAs, caps it at the element count, requires it to be nonzero,
and selects the greatest power of two no larger than that limit
(`native-probe/src/hsa.rs:L770-L788`). This gives one valid workgroup size for
every discovered ISA. `DispatchGeometry::one_dimensional(elements, lanes)`
then asks ROCr to validate the resulting grid and workgroup against every ISA
limit (`hsa/src/execution.rs:L1062-L1083`, `L1909-L1960`).

The kernel path is:

1. Build an `AmdTarget` with the selected target tail and configured
   code-object version.
2. Build `fma_template(bytes, fma_chain_length)`, a single f32 input and
   output with a forbidden alias, two f32 constants, and a dependent chain of
   `Fma` instructions (`native-probe/src/benchmark.rs:L113-L187`).
3. Lower with entry symbol `recipe_probe_fma_f32` and the selected workgroup
   lanes. `lower_elementwise` emits target-specific LLVM IR, a pointer-plus-
   u64-element-count ABI, and a measured FLOP count (`kernel/src/llvm.rs:L172-L185`,
   `L304-L374`).
4. Construct `ArtifactBuilder` from the pinned toolchain and call
   `build(BuildPhase::Realize, ...)`. The builder verifies tool digests, runs
   LLVM verification and AMD code generation, links with the pinned ELF linker,
   and structurally inspects the resulting HSACO
   (`kernel/src/builder.rs:L154-L215`, `L454-L518`). The AMD invocation uses
   `-march=amdgcn`, `-mcpu=<processor>`,
   `--amdhsa-code-object-version=<version>`, and any target feature modifiers
   (`kernel/src/builder.rs:L676-L710`).
5. Require `BuiltArtifact::Hsaco`. A cubin or any other artifact is an
   internal builder mismatch and fails the benchmark.
6. Load the in-memory HSACO into the exact session. Resolve the inspected
   kernel symbol and require ROCr's `kernarg_segment_size` to equal inspected
   metadata. `recipe-hsa` also checks that the symbol is a kernel, has a valid
   nonzero object, and reports a power-of-two kernarg alignment
   (`hsa/src/execution.rs:L831-L944`).
7. Allocate a CPU kernarg pool, grant the GPU access, and write three native-
   endian 64-bit values at offsets `0`, `8`, and `16`: input pointer, output
   pointer, and the lowered element count. `write_u64` checks offset overflow
   and the explicit ABI size (`native-probe/src/hsa.rs:L512-L529`, `L853-L867`).
8. Require a discovered queue, create a single-producer queue at its minimum
   packet size, dispatch one-dimensional geometry, and time completion.
9. Download the output, then require the first f32 output to be finite and to
   have bits different from the first f32 input. This is the independent
   end-state check used by the probe's calculation benchmark
   (`native-probe/src/benchmark.rs:L166-L184`).
10. Close the queue, release the kernel handle, close the executable, and close
    the kernarg allocation. Teardown errors remain benchmark errors.

The measured calculation rate uses `lowered.work`, so the configured chain and
element count are part of the work numerator rather than an inferred device
throughput value (`native-probe/src/hsa.rs:L456-L568`).

## Completion and timeout safety

`complete_hsa` polls a `recipe-hsa::Pending` token with an exponential backoff:

```text
normal poll: 50 microseconds, doubling up to 2 milliseconds
timeout cleanup: 10 milliseconds, doubling up to 100 milliseconds
```

It polls until `Complete` or the supplied deadline. If the deadline expires
while the token is still pending, it does not release live resources or return
immediately. It moves to a capped cleanup loop, keeps polling until the
already-submitted operation reaches a terminal signal, and then returns a
benchmark deadline error. A poll error, negative signal, or queue poison is
returned as a benchmark error (`native-probe/src/hsa.rs:L32-L65`, `L814-L851`).

This apparent unbounded cleanup is intentional. `recipe-hsa::Pending` retains
the completion signal, allocations, executable, and queue until terminal
completion; dropping an incomplete token moves those resources into a
session-owned deferred-retirement set. Releasing them while ROCr can still
reference them would be unsafe. The HSA boundary reports unresolved terminal
leaks instead of destroying a live signal (`hsa/src/execution.rs:L1165-L1279`,
`hsa/src/error.rs:L112-L124`).

## Preparation binding flow

Discovery descriptors are not execution handles. Preparation reopens exact
handles through `native-probe/src/bindings.rs`:

```text
with_native_execution_bindings
  -> discover_all and resolve the measured local inventory
  -> partition expected origins by target.backend
  -> HsaBackend::with_runtime
       -> Runtime::discover
       -> realize_hsa
       -> lend HsaBinding values to one higher-ranked callback
  -> destroy the callback scope and all borrowed sessions
```

`realize_hsa` partitions the fresh ROCr agents into CPU and GPU sets, retains
only CPU agents with both an allocatable kernarg pool and an allocatable fine
or extended-fine pool, and sorts them deterministically by NUMA node, UUID,
name, and vendor. Every GPU is converted with the same `descriptor` function
used during measurement. An unexpected key, duplicate key, missing queue, or
missing measured GPU is a hard binding error
(`native-probe/src/bindings.rs:L319-L416`).

For each measured GPU, host allocator selection is exact:

| Candidate set | Result |
| --- | --- |
| One same-NUMA CPU allocator | Select it. |
| More than one same-NUMA allocator | Ambiguous, fail closed. |
| No same-NUMA allocator and exactly one total allocator | Select the fallback. |
| No allocator or multiple fallback allocators | Missing or ambiguous, fail closed. |

The resulting `HsaBinding` contains the measured `DeviceId`, the borrowed GPU
`Session`, borrowed CPU `DiscoveredAgent`, complete target ID, code-object
version, queue minimum packet count, maximum queue count, and enabled display
connector count (`native-executor/src/hsa.rs:L31-L100`). Its lifetime is tied to
the callback; no ROCr handle is stored in a declaration or later dynamic
placement path.

`src/native_prepare.rs::build_scope` verifies that the callback's CUDA and HSA
device set equals the resolved measured GPU set and that no device has both
backend bindings. For every HSA device, `hsa_spec` requires:

```text
descriptor.target.architecture == binding.target_id
binding.code_object_version == config.hsa.code_object_version
binding.target_id starts with amdgcn-amd-amdhsa--
```

It strips the canonical prefix, validates `AmdTarget { target_id: tail,
code_object_version }`, and creates a `TargetBuildSpec` with the descriptor's
target and toolchain identities, shared `ArtifactBuilder`, private scratch
directory, and `RuntimeArtifactPolicy::Hsa` (`src/native_prepare.rs:L543-L619`,
`L705-L745`). Identical target identities must also have equivalent build
specifications before the deferred artifact compiler accepts them.

The native executor consumes the binding during pre-loop realization. It
requires a CPU host allocator with kernarg and fine-grained pools, an exact
target advertised by the session, single-producer queues using the retained
queue packet count, and runtime HSA artifacts whose target ID and code-object
version equal the binding (`native-executor/src/hsa.rs:L1692-L1738`,
`L1808-L1921`). Runtime artifacts are structurally inspected before loading,
and each executable resolves the immutable ABI symbol and kernarg metadata.
The HSA backend therefore participates in the immutable preparation-to-loop
lifecycle without exposing compilation, discovery, or allocation in the loop.

### CLI receipt and reopen consumer

The root CLI is the first production consumer of the public probe API. `run_probe`
builds `NativeProbeConfig`, constructs `NativeGpuProbe`, and gives the same
probe to `ProbeEngine` as both `GpuDiscovery` and `GpuBenchmarkIo`. After the
profile is validated and stored, `ActiveNativeReceipt::capture` records the
identity-named profile, host RAM key, canonical PCI root, private scratch
directory, selected ROCr library path and digest, HSA code-object version, FMA
chain length, and pinned LLVM/lld tool identities (`src/cli.rs:L876-L946`,
`L1011-L1073`). It records an HSA library only when the measured profile uses
the HSA backend, but it never invents a library when the profile does not.

Later preparation loads that receipt through `current_native_inputs`, checks
that the recorded host RAM origin is still in current host discovery, reopens
the exact profile and all pinned files, and constructs a new
`NativeProbeConfig` with the recorded HSA candidates and version
(`src/cli.rs:L949-L1008`, `L1076-L1120`). The thread-local
`CURRENT_NATIVE_PROBE` retains the initialized `NativeGpuProbe` so repeated
preparation calls can reuse its one ROCr initialization reference. A changed
configuration after that runtime is opened is an identity error, not a reason
to open a second runtime (`src/native_prepare.rs:L368-L410`).

## Descriptor and artifact identity chain

The following values must remain equal across every stage:

```text
ROCr IsaTarget::as_str()
  == GpuDescriptor.target.architecture
  == HsaBinding.target_id
  == RuntimeArtifactKind::Hsa.target_id
  == inspected HSACO target ID

config.hsa.code_object_version
  == GpuDescriptor.target.abi suffix
  == HsaBinding.code_object_version
  == RuntimeArtifactKind::Hsa.code_object_version
  == inspected HSACO code-object version
```

`recipe_kernel::inspect_hsaco` checks AMDGPU-HSA ELF identity, code-object
version, target, defined function and `.kd` symbols, HSA metadata, argument
offsets and sizes, kernarg alignment, and maximum workgroup size
(`kernel/src/artifact.rs:L297-L411`, `L511-L550`).
`recipe-native-executor` repeats the runtime-target and ABI checks when binding
an artifact (`native-executor/src/plan.rs:L308-L352`). A near-match artifact is
rejected; it is never loaded for a different GPU.

## Failure matrix

The backend fails closed. The following are the reachable failure classes and
their source of truth.

| Stage | Failure evidence | Result |
| --- | --- | --- |
| Construction | Zero HSA code-object version, zero FMA chain, non-absolute scratch parent | `ProbeError::Discovery`. |
| PCI preflight | Non-absolute root, unreadable entry, malformed vendor/class, or PCI enumeration failure | Discovery error. No absence fallback. |
| Runtime selection | Existing candidate is not a file or symlink, canonical target is invalid, bytes cannot be read, or no candidate exists with AMD hardware | Discovery error. |
| Runtime open | `dlopen`, required symbol resolution, or `hsa_init` fails | Discovery error with ROCr detail. |
| Runtime continuity | AMD PCI accelerator or library disappears, or the selected library path/digest changes after initialization | Discovery error. |
| ROCr discovery | System, agent, ISA, queue, memory-pool, callback, UTF-8, identity, or status failure | Discovery error; no partial inventory. |
| GPU eligibility | No kernel dispatch, unavailable UUID, missing PCI, AMD properties, queue, wavefront, or KFD node | Discovery error for that backend operation. |
| Target | Non-AMDGPU ISA, missing target, malformed prefix, repeated non-generic entries, or ambiguous generic targets | Discovery error. |
| Capacity and limits | No usable coarse GPU pool or nonzero available memory, integer conversion failure, missing LDS property, invalid workgroup limit | Discovery or benchmark error. |
| Exact ownership | Zero or multiple descriptor owners, changed descriptor, duplicate UUID, missing GPU, or no CPU memory agent | Benchmark error. |
| Allocation and access | Size exceeds capacity, no matching fine/coarse/kernarg pool, allocation failure, or failed reciprocal access grant | Benchmark error from ROCr/HSA. |
| Copy benchmark | Submit/poll failure, queue poison, negative signal, timeout cleanup failure, or byte mismatch | Benchmark error. |
| Kernel lowering/build | Invalid template, element or FLOP overflow, invalid target, missing or changed tool, LLVM audit failure, linker/codegen failure, or non-HSACO artifact | Benchmark error. |
| Kernel realization | Empty or incompatible HSACO, symbol lookup/type failure, metadata mismatch, kernarg allocation/access failure, queue or dispatch rejection | Benchmark error. |
| Verification/teardown | Nonfinite or unchanged f32 output, verification copy mismatch, or queue/executable/kernarg close error | Benchmark error. |
| Preparation binding | Missing/unexpected/duplicate measured key, queue disappearance, ambiguous host allocator, machine mismatch, target/version mismatch | `ProbeError::Discovery` or native preparation identity error. |

The generic probe engine records only completed measurements as
`PropertyProvenance::Measured`; seed estimates bound the work but cannot replace
these results (`native-probe/src/benchmark.rs:L187-L203`, `probe/src/engine.rs:L63-L160`).

## Operational call graph

```text
recipe probe
  -> native_config (CLI paths, version 6, pinned LLVM/lld, FMA chain)
  -> NativeGpuProbe::new
  -> ProbeEngine::load_or_probe_and_store
       -> NativeGpuProbe::discover_all
            -> HsaBackend::discover
                 -> with_runtime
                 -> recipe_hsa::Runtime::discover
                 -> descriptor for every GPU agent
       -> NativeGpuProbe::benchmark_gpu for every descriptor
            -> exact backend ownership rediscovery
            -> HsaBackend::benchmark
                 -> HSA copy rates and Recipe HSACO FLOP rate
       -> validated measured profile and active native receipt

preparation or training
  -> with_native_execution_bindings
       -> exact profile/local-inventory resolution
       -> HSA rediscovery and realize_hsa
       -> scoped HsaBinding values
  -> build_scope / hsa_spec
       -> AmdTarget and DeferredArtifactCompiler specification
  -> recipe-native-executor
       -> pre-loop queues, arenas, kernargs, executables, and immutable loop work
```

The HSA-only diagnostic follows the same backend path but emits
`GpuInventory { exhaustive: false }`; it is useful for diagnosing ROCr on a
mixed machine and is intentionally rejected by normal profile construction.
