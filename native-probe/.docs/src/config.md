# Native probe configuration

`recipe-native-probe` exposes the configuration records used by the bare-metal
GPU discovery and benchmark boundary. The records live in
[`native-probe/src/config.rs`](../../src/config.rs). They are plain Rust values:
there is no `serde` implementation, TOML deserializer, `Default` value, or
configuration file reader in this crate. The root `recipe` binary builds a
`NativeProbeConfig` from command-line path overrides, fixed source defaults,
and the host discovery result. A private active-native receipt can later reopen
the same values and re-pin their files.

The TOML file used by `recipe probe`, `topology/contract.toml`, is a separate
`recipe-probe::SeedContract`. It contains theoretical benchmark seed values,
not native-library paths, compiler tools, GPU ISA values, or hardware
inventory. The distinction is important: changing the seed changes the probe
plan and cache identity, while changing a native setting changes the native
identity and the runtime build policy.

## Type graph

All five records derive `Clone`, `Debug`, `PartialEq`, and `Eq`. Every field is
public. `PathBuf` values are operating-system paths. `Label` accepts any
nonempty, non-whitespace string and rejects an empty or whitespace-only value.

### `BackendLibrary`

```rust
pub struct BackendLibrary {
    pub candidates: Vec<PathBuf>,
}
```

`candidates` is an ordered list of explicit shared-library paths for one native
backend. A missing path is skipped. Existing candidates are required to be
regular files or symlinks whose canonical targets are regular files and whose
bytes can be read and hashed. The first existing canonical target in list order
is selected. Canonical duplicates are inspected once. A malformed, unreadable,
or unhashable existing candidate is an error, even when an earlier candidate
could have been selected. An empty list is valid as a value, but means that a
backend with matching hardware has no usable configured runtime.

`identity::selected_library` enforces absolute paths when a backend is opened.
The value is cloned into the CUDA or HSA backend; it is not consumed by
configuration construction.

### Backend records

| Record | Field | Meaning and consumers |
| --- | --- | --- |
| `CudaProbeConfig` | `library: BackendLibrary` | CUDA Driver library candidates. `CudaBackend::open` selects and hashes a candidate only after PCI discovery finds an NVIDIA accelerator (vendor `0x10de`). |
| `CudaProbeConfig` | `ptx_isa: u16` | PTX ISA encoded as `major * 10 + minor`, for example `74` for PTX 7.4. It is placed in `NvidiaTarget`, included in the CUDA toolchain identity, used by LLVM lowering and the pinned `ptxas` build, and checked by `NvidiaTarget::validate` (`32..=90`). |
| `HsaProbeConfig` | `library: BackendLibrary` | ROCr/HSA runtime library candidates. `HsaBackend::with_runtime` selects and hashes a candidate only after PCI discovery finds an AMD accelerator (vendor `0x1002`). |
| `HsaProbeConfig` | `code_object_version: u8` | AMDGPU code-object version passed to LLVM and the ELF linker, retained in the target ABI (`elf64-amdgpu-code-object-vN`), included in the HSA toolchain identity, and checked against reopened HSA bindings. Zero is rejected by `HsaBackend::new` and by `AmdTarget::validate`. |

### `KernelBuildConfig`

```rust
pub struct KernelBuildConfig {
    pub toolchain: OfflineToolchain,
    pub release: Label,
    pub scratch_parent: PathBuf,
    pub fma_chain_length: u16,
}
```

* `toolchain` contains two required `PinnedTool` values, `verifier` (LLVM
  `opt`) and `llvm_codegen` (LLVM `llc`), plus the backend-specific optional
  values `elf_linker` (LLVM `lld`) and `ptx_assembler` (NVIDIA `ptxas`).
  `ArtifactBuilder::new` rechecks every present path and digest. The identity
  code requires `elf_linker` for `amd-hsa` and `ptx_assembler` for
  `nvidia-cuda`, so those fields are optional only for a machine that does not
  use that backend.
* `release` is a nonempty, human-readable release label. It is paired with
  exact tool digests in each backend's `ToolchainIdentity`; it is not a package
  manager version lookup.
* `scratch_parent` is the parent directory in which `ArtifactBuilder` creates
  private, temporary build workspaces. `NativeGpuProbe` requires it to be
  absolute. The CLI and active-receipt path additionally require an existing,
  canonical, non-symlink directory owned by the effective user with no group or
  other permissions. `ArtifactBuilder` independently requires a real directory
  with no group or other permissions when it creates a workspace.
* `fma_chain_length` is the number of dependent Recipe-owned f32 FMA
  instructions generated per element by the bounded native FLOP benchmark. It
  must be nonzero. The value is included in the target configuration identity,
  so changing it invalidates a measured native target even when the hardware
  is unchanged.

### `NativeProbeConfig`

```rust
pub struct NativeProbeConfig {
    pub host_memory_key: Label,
    pub pci_sysfs_root: PathBuf,
    pub cuda: CudaProbeConfig,
    pub hsa: HsaProbeConfig,
    pub kernels: KernelBuildConfig,
}
```

* `host_memory_key` names the RAM domain to which every discovered GPU is
  attached. The CLI obtains it from the first domain returned by host
  discovery. The native probe itself only stores the label; `ProbeEngine`
  rejects a GPU descriptor whose key is absent from the current host RAM
  inventory.
* `pci_sysfs_root` is the explicit PCI sysfs directory used for vendor
  preflight, PCI surface identities, and DRM connector counts. The normal
  value is `/sys/bus/pci/devices`. Its absolute-directory requirement is checked
  when PCI is read or when the active receipt is captured; the top-level native
  config validator does not inspect the directory contents.
* `cuda`, `hsa`, and `kernels` are the nested records described above. A
  `NativeGpuProbe` clones the backend and kernel records, while moving the PCI
  root into its own state.

## How the CLI constructs the value

`src/cli.rs::parse_probe_options` consumes arguments in option/value pairs.
Every option requires a following path. `--contract`, `--profile`,
`--llvm-opt`, `--llvm-llc`, `--lld`, and `--ptxas` may occur once;
`--cuda-driver` and `--hsa-runtime` may occur repeatedly and preserve their
order. Unknown options, missing values, non-UTF-8 option names, or duplicate
single-use options fail before native configuration construction.

`native_config` first pins the tools with `PinnedTool::inspect` and then
constructs the records. An explicit tool path is canonicalized and hashed. If
no explicit path was supplied, the first existing fixed candidate is inspected;
missing candidates are skipped. Required LLVM `opt` and `llc` fail if no
candidate exists. `lld` and `ptxas` may remain `None` at this stage. An
existing but invalid candidate is an error rather than a request to try a later
candidate.

The library override rule is different from the tool rule. If no
`--cuda-driver` or `--hsa-runtime` option is supplied, the corresponding fixed
list below is used. Supplying one or more options replaces that list completely
with the supplied ordered paths.

### Current source defaults

The values below are the defaults in `native_config` at the time of this
documentation. They are source values, not TOML keys.

| Config path | Default |
| --- | --- |
| `pci_sysfs_root` | `/sys/bus/pci/devices` |
| `cuda.library.candidates` | `/usr/lib/x86_64-linux-gnu/libcuda.so.1`, `/usr/lib64/libcuda.so.1`, `/usr/lib/libcuda.so.1`, `/usr/local/nvidia/lib64/libcuda.so.1` |
| `cuda.ptx_isa` | `74` (PTX 7.4) |
| `hsa.library.candidates` | `/opt/rocm/lib/libhsa-runtime64.so.1`, `/usr/lib/x86_64-linux-gnu/libhsa-runtime64.so.1`, `/usr/lib64/libhsa-runtime64.so.1`, `/usr/lib/libhsa-runtime64.so.1` |
| `hsa.code_object_version` | `6` |
| `kernels.toolchain.verifier` candidates | `/usr/bin/opt`, `/usr/local/bin/opt`, `/usr/lib/llvm-22/bin/opt`, `/usr/lib/llvm-21/bin/opt`, `/usr/lib/llvm-20/bin/opt`, `/usr/lib/llvm-19/bin/opt`, `/opt/llvm/bin/opt` |
| `kernels.toolchain.llvm_codegen` candidates | `/usr/bin/llc`, `/usr/local/bin/llc`, `/usr/lib/llvm-22/bin/llc`, `/usr/lib/llvm-21/bin/llc`, `/usr/lib/llvm-20/bin/llc`, `/usr/lib/llvm-19/bin/llc`, `/opt/llvm/bin/llc` |
| `kernels.toolchain.elf_linker` candidates | `/usr/bin/ld.lld`, `/usr/local/bin/ld.lld`, `/usr/lib/llvm-22/bin/ld.lld`, `/usr/lib/llvm-21/bin/ld.lld`, `/usr/lib/llvm-20/bin/ld.lld`, `/opt/llvm/bin/ld.lld` |
| `kernels.toolchain.ptx_assembler` candidates | `/opt/cuda-11.8/bin/ptxas`, `/opt/cuda-11.7/bin/ptxas`, `/opt/cuda-11.6/bin/ptxas`, `/opt/cuda-11.5/bin/ptxas`, `/opt/cuda-11.4/bin/ptxas`, `/usr/local/cuda-11.8/bin/ptxas`, `/usr/local/cuda-11.4/bin/ptxas`, `/opt/cuda/bin/ptxas`, `/usr/local/cuda/bin/ptxas`, `/usr/bin/ptxas` |
| `kernels.release` | `auto-pinned-local-tools-and-benchmark-v3` |
| `kernels.scratch_parent` | `<private state root>/scratch` |
| `kernels.fma_chain_length` | `64` |

The private state root is selected from an absolute `XDG_CACHE_HOME`, or from
the canonicalized `$HOME/.cache` when that variable is absent or relative. The
root is `<cache base>/recipe-next`, and the CLI creates and verifies it as a
private, canonical, effective-user-owned directory before using `scratch`.

The source comments document why the current scalar defaults are fixed: PTX
7.4 remains accepted by the R470 deployment fixture and newer pinned
assemblers, and 64 dependent FMAs amortize launch overhead while keeping the
bounded probe responsive.

## Probe and preparation consumers

The configuration flows through the system as follows:

```text
recipe probe options
    -> native_config
    -> NativeGpuProbe::new
       -> CudaBackend / HsaBackend
       -> discovery and bounded native benchmarks
       -> measured profile and active-native-v1 receipt

training or inference
    -> current_native_inputs (receipt reopen or default reconstruction)
    -> NativeGpuProbe::new
    -> with_native_preparation
       -> exact bindings, target specs, and deferred native builders
```

### Native probe construction

`NativeGpuProbe::new`, `cuda_diagnostic`, and `hsa_diagnostic` all call the same
top-level validator before constructing backends. The normal constructor owns
both backend records and reports an exhaustive inventory. The diagnostic
constructors retain one backend and deliberately report a non-exhaustive
inventory, so their result cannot be accepted as a measured production
profile. HSA construction rejects `code_object_version == 0`; the CUDA
constructor has no additional scalar check.

### CUDA backend

`CudaBackend::new` clones `cuda.library`, `cuda.ptx_isa`, the host key, PCI root,
and the kernel record. On every open or rediscovery it:

1. reads the configured PCI root and returns `None` when no NVIDIA accelerator
   exists;
2. selects the first existing canonical CUDA Driver candidate, hashing every
   existing nonduplicate candidate along the way;
3. fails if hardware exists but no candidate remains, if a candidate is not an
   absolute regular file or symlink, if canonicalization or hashing fails, or
   if loading or exhaustive Driver discovery fails; and
4. builds a `NvidiaTarget` from the discovered SM and configured PTX ISA.

The target rejects PTX values outside `32..=90`. Descriptor identity includes
the selected library digest, PCI surface, host key, target ABI, release,
required tool digests, PTX value, and FMA-chain value. The benchmark lowers the
Recipe-owned FMA template, invokes the pinned LLVM tools and `ptxas` under the
configured scratch parent, loads the resulting cubin through the CUDA Driver,
and verifies the changed finite output.

### HSA backend

`HsaBackend::new` clones the analogous HSA and kernel fields after rejecting a
zero code-object version. `with_runtime` performs the AMD PCI preflight before
selecting a ROCr candidate. No AMD accelerator means an unused HSA backend is
absent. Once AMD hardware is present, no configured runtime, a failed runtime
load, a changed library identity, or a failed exhaustive runtime discovery is a
hard discovery error. After initialization, disappearance of all AMD PCI
accelerators or the runtime library is also an error, not backend absence.

HSA descriptors and benchmark builds use the configured code-object version,
FMA-chain value, release, toolchain, host key, PCI root, and scratch parent.
The target ABI is `elf64-amdgpu-code-object-vN`; the measured descriptor and
the reopened HSA binding must carry the same version.

### Identity helpers and source changes

`identity::backend_toolchain_identity` hashes the backend name, release label,
target configuration, the native-probe source digest, and the path plus digest
of each required tool. The HSA target configuration contains the target ID,
code-object version, and FMA-chain length. The CUDA configuration contains the
SM architecture, PTX ISA, and FMA-chain length. `native-probe/build.rs`
computes the source digest from all Rust files in `native-probe/src` and the
listed dependent crates and manifests, so editing `config.rs` itself changes
the identity of later measured native targets.

`ProbeEngine` hashes each resulting GPU descriptor, including its host key,
target, runtime/library identity, PCI surfaces, capabilities, and toolchain,
into the measured-profile cache identity. `scratch_parent` is not a hardware
profile field, but it is retained in the active receipt and in every
`TargetBuildSpec` used by preparation.

## Active receipt loading and config reconstruction

After a successful `recipe probe`, `ActiveNativeReceipt::capture` records the
configuration needed to reopen the exact native environment. It validates the
private measured-profile file, canonical PCI root, and private scratch parent;
rejects unsupported native backends in the profile; selects and pins a required
CUDA or HSA library when that backend is present; and re-inspects every required
tool plus any backend-specific optional tool. A changed path or digest aborts
capture.

The receipt is a fixed-order UTF-8 text record with the marker
`recipe-active-native-v1` and these 16 fields:

```text
profile
profile_schema
profile_digest
host_memory_key
pci_sysfs_root
scratch_parent
cuda_library
hsa_library
llvm_opt
llvm_llc
lld
ptxas
ptx_isa
hsa_code_object_version
release
fma_chain_length
```

Paths and labels are lowercase byte-hex encoded, digests are 64 lowercase hex
digits, optional pins use `none`, required tool pins may not be `none`, and
scalar values are decimal. Decode rejects an invalid marker, missing or
reordered fields, trailing fields, malformed hex, invalid labels, invalid
integers, or a noncanonical re-encoding. The file is additionally required to
be a regular non-symlink mode `0600` file owned by the effective user, no larger
than 64 KiB, and unchanged between metadata inspection and open.

`reopen_config` checks the recorded profile path, PCI root, and scratch parent
again; verifies every recorded tool digest; and turns each recorded library pin
into a one-candidate `BackendLibrary`. It restores the recorded host key,
release, PTX ISA, code-object version, FMA-chain length, and toolchain. A
missing active receipt instead reconstructs the source defaults and computes
the current identity-keyed profile path. It does not select an arbitrary newest
profile.

The current native probe is retained thread-locally with its original
`NativeProbeConfig`. Repeated training or inference calls must compare equal
configs; a changed config after a runtime has been opened returns an identity
mismatch rather than silently mixing runtimes.

## Preparation consumers and checks

`src/native_prepare.rs` accepts a `&NativeProbeConfig` for explicit profile
loading, target-plan construction, and the current training/inference path.
It creates a fresh `NativeGpuProbe`, resolves the exact measured local
inventory, and lends scoped CUDA/HSA bindings. The config then controls:

* the shared `ArtifactBuilder` toolchain;
* CUDA target PTX and the pinned-assembler runtime policy;
* HSA code-object compatibility with each binding; and
* the scratch parent on every `TargetBuildSpec`.

`load_native_preparation` first requires an identity-named profile path of the
form `measured-v<schema>-<64 lowercase hex>.recipe-profile`, then delegates to
the same config-driven target-plan path. `build_native_target_plan` uses the
same scoped flow and returns owned build specifications after its native
bindings are dropped. `with_current_native_preparation` obtains the config from
the active receipt (or source defaults when no receipt exists), loads exactly
the profile named by the current discovery identity, and reuses one thread-local
probe only when the complete `NativeProbeConfig` compares equal.

Preparation fails closed when host or GPU origins differ from the profile, a
native binding is missing or duplicated, the configured HSA version disagrees
with a binding, the measured CUDA target does not match the configured PTX
target, a required backend tool is absent, or any pinned path or digest has
changed. Public training and inference reach this path through
`with_current_native_preparation`; declarations do not carry native handles or
their own copy of this configuration.

## TOML seed contract is separate

`recipe probe` loads the seed before it builds `NativeProbeConfig`:

```text
--contract PATH  -> SeedContract::read(PATH)
no --contract   -> include_str!("../topology/contract.toml")
                   -> SeedContract::parse(text)
```

`SeedContract` uses a strict, small TOML-like parser in
`probe/src/seed.rs`, not a general TOML library. It accepts comments outside
quoted strings, table headings, assignments, and multiline arrays. Duplicate
keys, unterminated arrays, malformed assignments, unquoted strings, missing
keys, invalid unsigned integers, zero rates, invalid booleans, and unknown
fields produce `ProbeError::Contract` errors.

The required contract is schema `1`, kind `probe-seed-estimates`, the eleven
`[estimates]` values, the exact storage reservation of `1_000_000_000`, the
`recipe probe` and `bare-metal` probe identity, seven enabled discovery and
benchmark gates, all seven cache invalidation facets, and at least one
`[transport.<name>]` with `directions = "both"`, `issue = "async"`, and
`duplex = "full"` or `"half"`. Every required transport field is checked and
unknown machine, device, link, or production-rate inventory keys are rejected.

The checked-in seed values are:

| TOML key | Value |
| --- | ---: |
| `estimates.ethernet_bytes_per_second` | `125_000_000` |
| `estimates.disk_bytes` | `1_000_000_000_000` |
| `estimates.sata_bytes_per_second` | `600_000_000` |
| `estimates.gpu_vram_bytes` | `12_000_000_000` |
| `estimates.pcie_bytes_per_second` | `16_000_000_000` |
| `estimates.gpu_flops_per_second` | `380_000_000_000` |
| `estimates.gpu_transfer_bytes_per_second` | `432_000_000_000` |
| `estimates.ram_bytes` | `48_000_000_000` |
| `estimates.ddr_bytes_per_second` | `90_000_000_000` |
| `estimates.cpu_reference_flops_per_second` | `150_000_000_000` |
| `estimates.ram_transfer_bytes_per_second` | `90_000_000_000` |
| `reservation.bytes_per_storage_device` | `1_000_000_000` |

The checked-in transport tables all require bidirectional asynchronous issue:

| Transport tables | `directions` | `duplex` | `issue` |
| --- | --- | --- | --- |
| `pcie`, `nvme`, `sas`, `ethernet` | `both` | `full` | `async` |
| `sata`, `wlan` | `both` | `half` | `async` |

The checked-in `[probe]` values are also fixed by the parser contract:

```toml
command = "recipe probe"
environment = "bare-metal"
discover_machines = true
discover_devices = true
discover_links = true
benchmark_capacity = true
benchmark_calculation = true
benchmark_transfer = true
require_measured_profile_for_prepare = true
invalidate_on = [
    "machine",
    "device",
    "driver",
    "runtime-abi",
    "firmware",
    "link",
    "artifact-toolchain",
]
```

The seed estimates size the first bounded benchmark plans, not accepted
performance. `ProbeEngine` derives RAM, storage, GPU, and network buffers by
dividing the corresponding estimates by `1024`, `16_384`, `1024`, and `8`,
then clamps each buffer to 4 KiB through 64 MiB, runs at most eight iterations,
and enforces a two-second maximum duration. All estimate values, the schema,
invalidation set, reservation, transport names, and transport duplex values are
hashed into the profile cache identity. Native library/tool path selections and
the scalar native target settings are supplied by `NativeProbeConfig` and the
active receipt rather than TOML; selected paths and digests are then retained
in measured descriptor identities.

## Validation timing summary

| Value or condition | Earliest check | Failure behavior |
| --- | --- | --- |
| `kernels.fma_chain_length` | `NativeGpuProbe::{new,cuda_diagnostic,hsa_diagnostic}` | Zero returns `Discovery("native FLOP benchmark requires a nonzero FMA chain")`. The benchmark template repeats the check if called directly. |
| `kernels.scratch_parent` absolute path | Same constructors | Relative paths return `Discovery("kernel scratch parent ... is not absolute")`. |
| `kernels.scratch_parent` existence, type, ownership, permissions | CLI receipt capture and `ArtifactBuilder::build` | Missing, symlink, non-directory, noncanonical, group/other-readable, or wrong-owner paths fail; no alternate scratch path is selected. |
| `hsa.code_object_version` | `HsaBackend::new` | Zero fails `NativeGpuProbe::new` and `hsa_diagnostic`, even on a host without AMD hardware. `cuda_diagnostic` does not construct the HSA backend. |
| `cuda.ptx_isa` | CUDA descriptor target construction or preparation target construction | Values outside `32..=90` fail target validation when CUDA is actually used. |
| backend library candidate paths | `identity::selected_library` on matching PCI hardware | Relative, non-file, broken, unreadable, unhashable, or changed candidates fail. All missing candidates are skipped; hardware with no remaining candidate is an error. |
| required LLVM tools | `native_config` | No valid `opt` or `llc` candidate fails config construction. |
| backend-specific linker/assembler | Descriptor identity or artifact build | Missing `lld` fails AMD discovery/build; missing `ptxas` fails NVIDIA discovery/build. They may be absent only when that backend is unused. |
| `host_memory_key` | `ProbeEngine::validate_gpu_inventory` and current receipt reopen | A label not present in current RAM discovery fails the measured inventory or asks for a fresh `recipe probe`. |
| `pci_sysfs_root` | PCI preflight/surface or receipt inspection | Relative, missing, symlink, non-directory, unreadable, or malformed PCI/DRM files fail the operation. |
| `release` | `Label::new` in CLI construction and identity creation | Empty or whitespace labels fail; the label is never inferred from a tool name. |
| active receipt | `load_active_native_receipt` and `ActiveNativeReceipt::decode` | Any schema, ownership, permission, race, size, encoding, field-order, digest, or canonicality violation fails closed. |

No config field silently falls back after it has been selected. The only
intentional absence is a backend whose matching PCI hardware is absent, and
the only intentional optional tool values are the backend-specific linker and
assembler on machines that do not use that backend.
