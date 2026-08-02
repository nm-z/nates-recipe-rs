# `native-probe` identity contract

This document is the source-traced contract for
[`native-probe/src/identity.rs`](../../src/identity.rs).  It describes the
identity values that the native GPU probe emits, the exact bytes used to hash
them, and the consumers that reject a changed identity.  The implementation
uses `recipe_probe::ProbeError`: filesystem failures are `ProbeError::Io`, and
identity or format failures are `ProbeError::Discovery` unless a caller wraps
the failure in its own benchmark or preparation error.

The identity boundary is deliberately fail-closed.  A missing optional
backend is represented by `None` only when the corresponding vendor has no
PCI accelerator.  Once hardware or a runtime has been observed, a missing,
unreadable, changed, or ambiguous identity is an error.  No product-name,
ordinal, capacity, or newest-file fallback is part of this contract.

## Identity data flow

The production path is the following closed flow.  The arrows are ownership or
validation boundaries, not alternate implementations.

```text
NativeProbeConfig
  ├─ BackendLibrary.candidates ──> selected_library
  │                                  └─> PinnedLibrary { canonical path, SHA-256(bytes) }
  ├─ KernelBuildConfig.toolchain ──> backend_toolchain_identity
  │                                  └─> ToolchainIdentity { name, release, SHA-256(manifest) }
  └─ pci_sysfs_root ──> pci_accelerator_present / pci_surface
                         └─> driver, firmware, and PCIe-link identity strings

CUDA Driver discovery + PinnedLibrary + PCI surface + toolchain
  └─> GpuDescriptor { key, TargetIdentity, driver, runtime_abi,
                      firmware, link_identity, ToolchainIdentity, capabilities }

ROCr/HSA discovery + PinnedLibrary + PCI surface + toolchain
  └─> GpuDescriptor { key, TargetIdentity, driver, runtime_abi,
                      firmware, link_identity, ToolchainIdentity, capabilities }

GpuDescriptor values + host and peer descriptors + SeedContract
  └─> recipe-probe cache identity
       └─> measured profile, topology identity, discovery identity
            └─> exact local re-resolution and native preparation
```

`native-probe` itself does not create a `MachineFingerprint`.  Linux host
identity is produced by `probe/src/local.rs` and is included by
`probe/src/engine.rs` in the same cache identity.  That adjacent producer and
the cache consumer are documented below because a native GPU identity is not
valid in isolation from the machine and profile that retain it.

## Types and canonical encodings

### `PinnedLibrary`

`PinnedLibrary` (`identity.rs:16-20`) is the only retained identity for a
selected backend shared object:

| Field | Contract |
| --- | --- |
| `path: PathBuf` | The result of `fs::canonicalize(candidate)`, therefore an absolute, resolved path.  The original spelling and symlink are not retained. |
| `digest: [u8; 32]` | Raw SHA-256 digest of the bytes read from the canonical target. |

The struct derives `Eq`.  HSA runtime state compares the complete pair, so a
path move, symlink retarget, or byte change is an identity change even when the
new library exports the same API.

### `PciSurface`

`PciSurface` (`identity.rs:162-167`) contains three already-formatted labels:

```text
driver   = driver:sha256=<lowercase hex>
firmware = firmware:sha256=<lowercase hex>
link     = pcie-link:sha256=<lowercase hex>
```

The exact digest inputs are specified in [PCI surface hashing](#pci-surface-hashing).
The fields are not raw sysfs text and must not be reconstructed by
callers.

### `Label` and `hex`

`label(value, field)` (`identity.rs:75-77`) delegates to
`recipe_core::Label::new`.  A value whose `trim()` is empty is rejected with a
`Discovery` error containing the supplied field name.  Nonempty values are
retained as supplied; the helper does not trim or otherwise normalize them.

`hex(bytes)` (`identity.rs:79-86`) emits exactly two lowercase hexadecimal
characters per byte, in input order.  It is used for library and surface
digest text.  It writes only to an in-memory `String`; the `expect` is
infallible for that operation.

All length prefixes in identity hashing are unsigned 64-bit little-endian
values.  Raw byte strings use `OsStr::as_encoded_bytes()` for paths and UTF-8
bytes for Rust `str` values.  A textual digest is not interchangeable with its
raw 32 bytes: the toolchain hash decodes an `ArtifactDigest`'s hexadecimal
representation back to raw bytes before hashing it.

## Build source identity

`identity.rs:14` obtains `PROBE_SOURCE_DIGEST` with `env!`, so the native-probe
crate cannot compile when its build script did not provide the variable.  The
producer is `native-probe/build.rs`.

### `RECIPE_NATIVE_PROBE_SOURCE_DIGEST`

`build.rs` computes one lowercase 64-character SHA-256 digest as follows
(`build.rs:10-136`):

1. Start with the unlength-prefixed marker
   `recipe-native-probe-build-v3`.
2. Recursively collect every `.rs` file under these source roots, using the
   domain names shown.  Directory entries are sorted by file name during
   recursion, then all `(domain/relative-path, absolute-path)` records are
   sorted lexicographically by the record name:

   ```text
   recipe-native-probe/src
   recipe-core/src
   recipe-executor/src
   recipe-host/src
   recipe-kernel/src
   recipe-language/src
   recipe-native-executor/src
   recipe-planner/src
   recipe-primitives/src
   recipe-scheduler/src
   recipe-cuda/src
   recipe-hsa/src
   recipe-probe/src
   ```

3. Append `native-probe/build.rs` itself and the workspace/package manifests
   listed in `build.rs:41-95`: `Cargo.lock`, the workspace `Cargo.toml`, and
   the manifests for `native-probe`, `core`, `executor`, `host`, `kernel`,
   `language`, `native-executor`, `planner`, `primitives`, `scheduler`,
   `cuda`, `hsa`, and `probe`.
4. For each sorted file record append `hash_field(name)` followed by
   `hash_field(file_bytes)`, where `hash_field(x)` is
   `u64_le(x.len()) || x`.
5. Append each environment name and its current value with `hash_field`.  The
   watched names and order are:

   ```text
   HOST
   TARGET
   OPT_LEVEL
   DEBUG
   PROFILE
   CARGO_CFG_TARGET_ARCH
   CARGO_CFG_TARGET_OS
   CARGO_CFG_TARGET_ENV
   CARGO_CFG_TARGET_FEATURE
   CARGO_ENCODED_RUSTFLAGS
   RUSTFLAGS
   ```

   An absent variable hashes as an empty field.  The build script emits a
   `cargo:rerun-if-env-changed` directive for every one.
6. Require `RUSTC`, invoke `<RUSTC> -Vv`, and require a successful exit.  The
   literal field `RUSTC`, the encoded compiler path, stdout, and stderr are
   each appended with `hash_field` (`build.rs:144-160`).  A missing `RUSTC`,
   spawn failure, or nonzero status has the direct build-script result, not a
   probe result.  Stderr from a successful invocation is hashed verbatim.
7. Publish the final lowercase digest as
   `RECIPE_NATIVE_PROBE_SOURCE_DIGEST` and register all files and environment
   values with Cargo's rerun mechanism.

Because this digest covers the identity implementation, all native backends,
the profile/core/kernel contracts, manifests, build flags, and the Rust
compiler identity, a change in any of those inputs changes every subsequent
`ToolchainIdentity.digest` emitted by this crate.

## Backend library selection

### `selected_library`

`selected_library(library, backend)` (`identity.rs:22-73`) consumes the ordered
`BackendLibrary.candidates` list.  It returns `Ok(None)` only when no candidate
exists.  The first existing candidate after canonical-target de-duplication is
selected; later existing, nonduplicate candidates are still validated and
hashed, but do not replace the first selection.

The exact per-candidate procedure is:

| Step | Operation | Result |
| --- | --- | --- |
| 1 | Require `candidate.is_absolute()` | Relative paths fail with `Discovery("{backend} library candidate ... is not absolute")`. |
| 2 | Read `fs::symlink_metadata(candidate)` | `NotFound` means this candidate is absent and is skipped.  Any other error is `Io("inspect native backend library", candidate, ...)`. |
| 3 | Check the candidate file type | Only a regular file or a symlink is accepted.  A directory, device, socket, or other type fails with `Discovery`. |
| 4 | `fs::canonicalize(candidate)` | Failure is `Io("canonicalize native backend library", candidate, ...)`.  A dangling symlink therefore fails rather than acting absent. |
| 5 | De-duplicate the canonical target in a `BTreeSet` | A second spelling of the same target is ignored after canonicalization. |
| 6 | Inspect canonical metadata | The target must be a regular file.  Metadata failure is `Io("inspect native backend library target", canonical, ...)`; a non-file target is `Discovery`. |
| 7 | Read canonical bytes | Failure is `Io("read native backend library", canonical, ...)`. |
| 8 | Hash and optionally select | `Sha256::digest(bytes)` is converted to `[u8; 32]`.  The first such `PinnedLibrary` becomes the return value. |

The function deliberately does not stop after selecting the first library.  It
continues through all configured candidates, so a later existing but
unreadable, malformed, or nonregular candidate is still a hard error.  This
prevents a broken configured path from being hidden behind an earlier valid
path.  Conversely, a missing candidate is harmless because candidate lists
are ordered search inputs.

### Runtime identity string

`library_identity(domain, pinned)` (`identity.rs:302-308`) formats:

```text
{domain}:{pinned.path.display()}:sha256={hex(pinned.digest)}
```

The native callers use these domains:

| Caller | Domain | Example shape |
| --- | --- | --- |
| CUDA descriptor | `cuda-driver-library` | `cuda-driver-library:/canonical/libcuda.so.1:sha256=...` |
| HSA descriptor | `rocr-library` | `rocr-library:/canonical/libhsa-runtime64.so.1:sha256=...` |

This is a stable label, not a new digest.  It is embedded in the CUDA or HSA
`runtime_abi` label and therefore in the probe cache digest.  HSA also retains
the original `PinnedLibrary` and compares its path and bytes on every later
`with_runtime` call.  CUDA reopens and reselects on every discovery, then
requires the complete descriptor to match when it benchmarks an existing
device.

## Toolchain and benchmark identity

### Required tool order

`required_tools(toolchain, backend)` (`identity.rs:115-135`) always begins with
these references in this order:

```text
toolchain.verifier       (LLVM verifier, normally opt)
toolchain.llvm_codegen   (LLVM code generator, normally llc)
```

It then requires exactly one backend-specific pinned tool:

| `backend` argument | Third tool | Missing-tool error |
| --- | --- | --- |
| `amd-hsa` | `toolchain.elf_linker` | `AMD probing requires an explicitly pinned ELF linker` |
| `nvidia-cuda` | `toolchain.ptx_assembler` | `NVIDIA probing requires an explicitly pinned PTX assembler` |
| any other value | none | `unknown native toolchain backend {backend}` |

The function never hashes optional tools that are not required by the selected
backend.  The returned references borrow the supplied `OfflineToolchain`; no
tool is opened or reread here.  Tool path and byte verification is performed by
`recipe_kernel::PinnedTool` when configuration is created or an artifact
builder is opened.

### Exact digest formula

`backend_toolchain_identity(toolchain, release, backend,
target_configuration)` (`identity.rs:88-113`) computes:

```text
H = SHA-256(
    "recipe-native-probe-toolchain-and-benchmark-v2"
  || u64_le(len(backend))              || backend UTF-8 bytes
  || u64_le(len(release.as_str()))     || release UTF-8 bytes
  || u64_le(len(target_configuration)) || target_configuration UTF-8 bytes
  || u64_le(len(PROBE_SOURCE_DIGEST))  || PROBE_SOURCE_DIGEST ASCII bytes
  || hash_tool(verifier)
  || hash_tool(llvm_codegen)
  || hash_tool(backend-specific tool)
)
```

`hash_tool` (`identity.rs:137-142`) appends, for each tool in the required
order:

```text
u64_le(len(tool.path.as_os_str().as_encoded_bytes()))
|| tool.path.as_os_str().as_encoded_bytes()
|| artifact_digest_bytes(tool.digest)
```

The resulting raw 32 bytes become `recipe_core::Digest::new(bytes)`.  The
returned `ToolchainIdentity` is:

```text
name    = Label("recipe-owned-llvm-{backend}")
version = release.clone()
digest  = H
```

The label constructor can only fail if the backend string somehow produces an
empty or whitespace-only name.  In normal calls `required_tools` has already
accepted one of the two backend names.

`target_configuration` is not a free-form description at the call sites.  It
binds the descriptor to the exact artifact and benchmark dimensions:

| Backend | `target_configuration` passed by the descriptor |
| --- | --- |
| CUDA | `{architecture}:ptx{ptx_isa}:dependent-f32-fma-chain-{fma_chain_length}`, where `architecture` is `sm_{major}{minor}`. |
| HSA | `{target_tail}:code-object-v{code_object_version}:dependent-f32-fma-chain-{fma_chain_length}`, where `target_tail` is the exact AMDGPU target after `amdgcn-amd-amdhsa--`. |

Changing the release label, compiler path, compiler bytes, source digest,
backend, target architecture, PTX or code-object version, or FMA chain length
changes this identity.  This is why a measured profile cannot be reused for a
different native benchmark or compiler even if the physical GPU is unchanged.

Configuration validation rejects a zero FMA chain before either backend is
constructed (`native.rs:221-233`), and HSA construction rejects a zero
code-object version (`hsa.rs:95-109`).  The CUDA PTX ISA is validated when its
descriptor target is built; the HSA target and code-object version are
validated when the exact HSA target is selected.  A relative scratch parent is
also rejected before discovery.  These checks prevent a malformed benchmark
configuration from acquiring a plausible identity.

### Artifact digest decoding

`artifact_digest_bytes` (`identity.rs:144-151`) calls
`ArtifactDigest::to_hex()`, expects the crate's 64 hexadecimal characters, and
decodes every two characters into one raw byte.  `hex_nibble` accepts `0-9`,
`a-f`, and `A-F`; any other byte is an `unreachable!` because
`ArtifactDigest::to_hex` is the producer.  The function does not hash the
64-character text.  The 32 decoded bytes are included directly in the
toolchain manifest.

## PCI presence and device surfaces

### `pci_accelerator_present`

`pci_accelerator_present(root, vendor)` (`identity.rs:169-193`) is the
preflight that distinguishes an absent optional backend from a broken runtime:

1. `root` must be absolute.  A relative root is `Discovery`.
2. `fs::read_dir(root)` must succeed.  Enumeration errors are `Io("enumerate
   PCI devices", root, ...)`; per-entry enumeration errors are
   `Io("enumerate PCI device", root, ...)`.
3. For every directory entry, read and parse `vendor` and `class` using
   `read_pci_hex`.  There is no filtering before these reads, so a malformed
   or unreadable entry is fatal even if another entry would match.
4. A match requires `actual_vendor == u32::from(vendor)` and
   `is_accelerator_class(class)`.  The class helper (`identity.rs:206`) accepts
   only class-code high words `0x03` (display controller) or `0x12` (processing
   accelerator).
5. Return `Ok(true)` on the first match and `Ok(false)` after all entries have
   been examined.

`read_pci_hex` trims the file, removes a lowercase `0x` prefix if present, and
parses the remainder as base-16 `u32`.  An unreadable file is
`Io("read PCI identity", path, ...)`; a nonhex value is `Discovery` with the
field and path.  Uppercase `0X` is not a recognized prefix and therefore is
parsed as invalid hexadecimal.

The CUDA caller supplies vendor `0x10de`; the HSA caller supplies `0x1002`.
If no matching accelerator exists, CUDA discovery returns no devices and HSA
returns `None` without requiring a runtime library.  If matching hardware
exists, a missing configured library is an error, not backend absence.

### `pci_surface`

`pci_surface(root, bdf)` (`identity.rs:208-257`) binds one exact PCI function
to three digests.  `root` must be absolute.  `root.join(bdf)` must be an
existing directory.  The driver symlink must canonicalize successfully; the
canonical target path is part of the driver digest prefix.

The file lists and prefixes are fixed and ordered:

| Surface | Prefix passed to `surface_digest` | Candidate files, in order | `require_one_file` |
| --- | --- | --- | --- |
| `driver` | canonical `driver` target path bytes | `/proc/sys/kernel/osrelease`; `<device>/driver/module/version` | `true` |
| `firmware` | none | `<device>/revision`; `<device>/subsystem_vendor`; `<device>/subsystem_device`; `<device>/vbios_version` | `false` |
| `pcie-link` | BDF string bytes | `<device>/current_link_speed`; `<device>/current_link_width`; `<device>/max_link_speed`; `<device>/max_link_width`; `<device>/numa_node` | `false` |

The returned `PciSurface` fields are the three `surface_digest` results.  A
missing or unreadable required driver file can therefore invalidate discovery;
the optional firmware and link surfaces may legitimately hash zero readable
files and still produce a digest.

### Surface hashing

`surface_digest(domain, prefix, paths, require_one_file)` (`identity.rs:259-300`)
uses this exact byte sequence:

```text
SHA-256(
    "recipe-native-probe-surface-v1"
  || domain bytes
  || [u64_le(len(prefix)) || prefix bytes]       (only when prefix is Some)
  || for each readable path in the supplied order:
       u64_le(len(path.as_os_str().as_encoded_bytes()))
    || path.as_os_str().as_encoded_bytes()
    || u64_le(len(file_bytes))
    || file_bytes
  || u64_le(found)
)
```

`NotFound` and `PermissionDenied` from `fs::read(path)` are skipped and do not
increment `found`.  Any other read error is `Io("read PCI identity surface",
path, ...)`.  If `require_one_file` is true and `found == 0`, the function
returns `Discovery("{domain} identity surface had no readable files")`.  The
digest is formatted as `{domain}:sha256={hex(finalize())}` with lowercase hex.

The path bytes are included so a surface cannot silently move to a different
sysfs location while retaining the same file contents.  The driver prefix adds
the resolved module target, and the link prefix adds the canonical BDF.  The
kernel release is therefore represented in both the host machine fingerprint
and the GPU driver surface, while firmware, lane, NUMA, and subsystem changes
alter their corresponding digest.

## GPU descriptor producers

`GpuDescriptor` is defined by `probe/src/model.rs:88-110`.  The native backend
descriptors are the only producers for CUDA and HSA devices.  Both constructors
retain the identity strings from `identity.rs` and attach measured-capability
fields used by scheduling.  Every string is passed through `label` before it
enters the descriptor.

### CUDA

`CudaBackend::descriptor` (`native-probe/src/cuda.rs:109-208`) derives one
descriptor from a CUDA Driver `DeviceInfo`, the complete `Discovery`, and the
selected `PinnedLibrary`:

1. Parse `device.pci_bus_id` as `domain:bus:device.function`.  Domain, bus,
   and device are hexadecimal; function is decimal and must be in `0..=7`.
   The parsed numbers must equal the Driver attribute fields.  The canonical
   sysfs BDF is then fixed-width lowercase `{:04x}:{:02x}:{:02x}.{}` so
   uppercase Driver rendering cannot create a second identity.
2. Read the PCI surfaces for that BDF.
3. Convert compute capability major and minor to `u8`, construct
   `NvidiaTarget { sm_major, sm_minor, ptx_isa }`, and validate its supported
   SM/PTX ranges.  The architecture is `sm_{major}{minor}`.
4. Compute `ToolchainIdentity` with backend `nvidia-cuda`, the architecture,
   configured PTX ISA, and configured FMA chain length.
5. Compute `runtime = library_identity("cuda-driver-library", library)`.
6. Build the stable key `cuda:{device.uuid}@{sysfs_bdf}`.

The resulting identity-bearing fields are:

| Field | Exact value |
| --- | --- |
| `key` | `cuda:{Driver UUID}@{canonical BDF}` |
| `target.backend` | `nvidia-cuda-driver` |
| `target.architecture` | `sm_{major}{minor}` |
| `target.abi` | `elf64-cubin` |
| `driver` | `cuda-kernel-driver:{driver_version.raw()}:{surface.driver}` |
| `runtime_abi` | `cuda-driver-api:{driver_version.raw()}:{runtime}` |
| `firmware` | `surface.firmware` |
| `link_identity` | `surface.link` |
| `toolchain` | `backend_toolchain_identity(..., "nvidia-cuda", ...)` |

Other descriptor fields are fixed or directly discovered: PCIe transport and
full duplex, one host-to-device and one device-to-host lane, asynchronous
submission, the CUDA queue limit, one concurrent task, warp size, maximum
threads/shared memory, and transfer overlap from async engines plus concurrent
kernels.  These fields are also included in the cache hash, so capability
changes invalidate the measured profile even when the key remains stable.

`parse_pci_bus_id` rejects missing components, invalid numeric components, and
function values above seven.  A numeric mismatch against Driver attributes is
a discovery error.  `NvidiaTarget::validate` failures are reported as
`Discovery("CUDA artifact target: ...")`; a non-UTF-8 canonical library path
fails before Driver loading.  Driver loading and exhaustive enumeration errors
are wrapped by `CudaBackend::open`.

### HSA

`HsaBackend::descriptor` (`native-probe/src/hsa.rs:153-294`) derives one
descriptor from `SystemDescription`, an HSA `AgentDescription`, and the
selected `PinnedLibrary`:

1. Ignore non-GPU agents by returning `Ok(None)`.
2. Require kernel-dispatch support.
3. Require a value-bearing stable UUID, not an absent or synthetic UUID.
4. Require an exact PCI address and use its canonical `Display` form for the
   sysfs BDF and key.
5. Hash the PCI surfaces.
6. Select one exact AMDGPU ISA with `exact_target`.  Every reported ISA must
   carry an AMD target.  One non-generic target wins; if all targets are the
   same generic target, that one is accepted.  Multiple specific targets or
   ambiguous generic targets fail.  `hsa_target_tail` then requires the
   `amdgcn-amd-amdhsa--` prefix and returns the remainder.
7. Require AMD capability properties and a physical queue.  Compute the
   toolchain identity with backend `amd-hsa`, the target tail, code-object
   version, and FMA chain length.
8. Compute `runtime = library_identity("rocr-library", library)` and choose
   the AMD product name, falling back to the HSA agent name only when the
   product name trims empty.

The identity-bearing fields are:

| Field | Exact value |
| --- | --- |
| `key` | `hsa:{agent UUID}@{HSA PCI address}` |
| `target.backend` | `amd-rocr-hsa` |
| `target.architecture` | Complete `IsaTarget::as_str()` |
| `target.abi` | `elf64-amdgpu-code-object-v{code_object_version}` |
| `driver` | `amdgpu-kfd-node-{driver_node_id}:{surface.driver}` |
| `runtime_abi` | `hsa-{hsa_major}.{hsa_minor}-amdext-{ext_major}.{ext_minor}:{runtime}` |
| `firmware` | `surface.firmware` |
| `link_identity` | `surface.link` |
| `toolchain` | `backend_toolchain_identity(..., "amd-hsa", ...)` |

The remaining fields retain the HSA queue, wavefront, ISA workgroup, LDS,
capacity, SDMA overlap, PCIe/full-duplex, and asynchronous-submission facts.
The descriptor rejects missing kernel capability, unstable UUID, PCI address,
AMD properties, queue, wavefront, ISA limits, KFD node, LDS properties, or
capacity.  All such failures are `Discovery` errors with the UUID in the
message where available.

### Backend discovery and revalidation

`CudaBackend::discover` calls `open`, then maps every CUDA device through the
descriptor constructor.  `HsaBackend::discover` opens or reuses one ROCr
runtime, exhaustively discovers all agents, and retains only descriptor-bearing
GPU agents.  `NativeGpuProbe::discover_all` (`native.rs:245-264`) merges both
lists, sorts by `GpuDescriptor.key`, and rejects duplicate keys.  The returned
`GpuInventory` is marked exhaustive only for `NativeGpuProbe::new`; the
CUDA-only and HSA-only diagnostic constructors deliberately mark it
non-exhaustive.

The same canonical key is also the input to
`NativeGpuProbe::enabled_display_connectors` (`native.rs:106-116`).  It splits
the final `@` component, requires the fixed 12-byte hexadecimal BDF shape with
a decimal function in `0..=7`, and reads DRM connector state below that exact
PCI directory.  A key without a canonical BDF suffix is a discovery error
rather than an ordinal fallback.  The connector count is a separate live
capability used by native bindings; it does not replace the PCIe-link surface
digest.

Every benchmark re-discovers the device and requires an exact descriptor
equality match.  CUDA reports a disappeared or changed descriptor as a
benchmark error.  HSA selects exactly one UUID whose descriptor equals the
expected descriptor and rejects zero or multiple matches.  A changed key,
target, driver/runtime surface, firmware, link, toolchain, or capability is
therefore visible before native benchmark work is submitted.

The HSA runtime has an additional lifetime check in
`HsaBackend::with_runtime` (`hsa.rs:112-149`):

* before initialization, no AMD PCI accelerator plus no current runtime returns
  `Ok(None)`;
* after initialization, disappearance of every AMD accelerator is a discovery
  error;
* a missing library is tolerated only before initialization and only when no
  AMD accelerator exists;
* once initialized, a different `PinnedLibrary` pair is
  `Discovery("ROCr/HSA library identity changed after initialization")`;
* a runtime load or operation failure is propagated as a discovery error.

## System and profile identity around the native descriptors

### Host machine producer

`probe/src/local.rs:42-78` creates `MachineFingerprint` before native GPU
discovery.  It reads and labels:

| Field | Source and fallback |
| --- | --- |
| `hostname` | `/proc/sys/kernel/hostname`, trimmed and required. |
| `stable_id` | First trimmed value available from `/etc/machine-id`, then `/sys/class/dmi/id/product_uuid`; no value is a discovery error. |
| `runtime_abi` | `/proc/sys/kernel/osrelease`, trimmed and required. |
| `firmware` | Joined trimmed values from DMI BIOS vendor, version, and date with `|`; if all are unavailable, literal `firmware-unreported`. |

`read_trimmed` reports read failures and empty files.  The stable-ID helper is
the only deliberate first-available fallback in this identity path.  Host RAM,
storage, and network domains then receive stable labels from Linux keys such as
NUMA node names, physical block major/minor, and network ifindex plus MAC.
`ProbeEngine::normalize_host` sorts these domains by key and rejects empty or
duplicate identities before hashing.

The host-domain producers used by the cache are precise, not heuristic:

| Domain | Stable key and identity fields |
| --- | --- |
| RAM | Enumerated `nodeN` directories are sorted and keyed by `nodeN`; capacity comes from that node's `MemTotal` in `meminfo`, and the link label is `memory-link:{name}`.  If no NUMA node is readable, `/proc/meminfo` produces one `memory0` domain with `memory-link:memory0`. |
| Storage | Mounted block devices are canonicalized to their physical parent.  A domain is keyed by `block:{major_minor}`, carries the physical path as `driver`, joins available model/revision/firmware text with `|`, and uses `storage-link:{physical_path}`.  Paths containing `/nvme` or `/sas` select NVMe or SAS full duplex; all other physical paths select SATA half duplex. |
| Network | Every non-`lo` interface is keyed by `net:{ifindex}:{MAC}` and linked as `network-link:{ifindex}:{MAC}`.  Its driver is the canonical driver path or `network-driver-unreported`; firmware joins `firmware_version` and `uevent` or uses `network-firmware-unreported`.  A `wireless` directory selects WLAN and half duplex; otherwise Ethernet and full duplex. |

Storage and network discovery still fail on required reads or inconsistent
physical capacity/transport observations.  Their fallback strings only cover
optional firmware or driver files, not missing stable keys.

### Cache identity hash

`ProbeEngine::build_cache_identity` (`probe/src/engine.rs:575-668`) is the
consumer that turns system and device identity into a cache key.  It uses
`CanonicalDigest::new("recipe-probe-cache-v7", PROFILE_SCHEMA)`, where the
canonical digest starts with a length-prefixed domain string followed by the
schema as little-endian `u64` (`probe/src/hash.rs:9-17`).  Its primitives are:

```text
string(s) = u64_le(len(s.as_bytes())) || s.as_bytes()
bytes(b)  = u64_le(len(b)) || b
u64(n)    = n.to_le_bytes()
bool(x)   = [u8::from(x)]
digest(d) = d.bytes()                     (raw 32 bytes, no length)
```

The exact field order is:

1. `hash_seed`: seed schema; the eleven estimate values in
   `SeedEstimates` order; storage reservation; every sorted invalidation facet;
   every sorted transport name and full/half duplex string.
2. `hash_machine`: hostname, stable ID, runtime ABI, firmware.
3. For each sorted RAM domain: tag `ram`, key, capacity hint, link identity,
   maximum in-flight count.
4. For each sorted storage domain: tag `storage`, key, name, capacity hint,
   benchmark root as lossy string, host-memory key, driver, firmware, link,
   transport kind, asynchronous flag, full-duplex flag, read lanes, write
   lanes.
5. For each sorted network interface: tag `network`, key, name, address,
   driver, firmware, link, transport kind, asynchronous flag, full-duplex
   flag.
6. For each sorted GPU descriptor: tag `gpu`, key, name, capacity hint,
   host-memory key, target backend/architecture/ABI, driver, runtime ABI,
   firmware, link, transport kind, toolchain name/version/raw digest,
   asynchronous flag, queue limit, concurrent-task limit, subgroup lanes,
   workgroup lanes, shared-memory limit, transfer-overlap flag, full-duplex
   flag, host-to-device lanes, device-to-host lanes.
7. For each peer sorted by session ID: tag `peer`, session ID, peer machine
   fingerprint, remote and local memory/interface keys, remote interface,
   driver, firmware, link, transport kind, asynchronous flag, full-duplex
   flag, outbound and inbound lanes.

The resulting raw digest and schema form `CacheIdentity`.  The seed contract
requires all machine, device, driver, runtime, firmware, link, and
artifact-toolchain invalidation facets (`topology/contract.toml:34-43`), so
the identity strings generated by this module are all cache-invalidating
inputs, not display-only metadata.

`ProbeEngine::inspect` builds this key after exhaustive host/GPU/peer discovery
and validation.  `current_cache_identity` computes it without benchmarks;
`load_or_probe_and_store` asks the cache for exactly that identity.  Host and
GPU order is normalized before hashing, so reordering discovery results does
not change the key.

### Profile and topology identities

After real bounded measurements, `build_profile_digest` starts a separate
canonical digest for `recipe-topology-v6` and `recipe-discovery-v6`, appends the
cache digest, then appends measured capacities/rates in sorted domain order.
RAM, storage, and GPU records carry their tag, stable key, and measured values.
Each peer record carries its session ID, remote-memory capacity/rate, optional
outbound and inbound rates, and all authenticated benchmark evidence: protocol
schema, both endpoint machine/profile digests, simultaneous-versus-serialized
execution, and for each direction total bytes, elapsed time, sample count,
minimum/maximum/mean sample times, and the 128-bit variance encoded as raw
little-endian bytes (`probe/src/engine.rs:727-787`).  The resulting raw digests
become `TopologyIdentity` and `DiscoveryIdentity`.
These identities are carried by the measured profile and are not substitutes
for the cache identity.  A profile is valid only when its schema is
`PROFILE_SCHEMA` and its retained `CacheIdentity` equals the freshly computed
one (`probe/src/model.rs:484-488`).

The profile retains origin records mapping each topology machine/device ID back
to the exact `MachineFingerprint` or domain/GPU key.  Local preparation resolves
the current exhaustive inventory only by those keys and by exact target
identity; missing, additional, or changed identities fail profile resolution.

## Identity consumers and transition checks

The following consumers make the identity operational rather than advisory:

| Consumer | Identity used | Required equality or action |
| --- | --- | --- |
| `NativeGpuProbe::discover_all` | GPU keys | Sort and reject duplicate keys; mark inventory exhaustive only for the full constructor. |
| `CudaBackend::benchmark_open` / `matching_device` | Complete `GpuDescriptor` | Reopen Driver and PCI surfaces, then require exactly one descriptor equal to the measured descriptor. |
| `HsaBackend::with_runtime` | `PinnedLibrary` | Retain one runtime and reject a changed path or bytes after initialization. |
| `HsaBackend::benchmark_with_runtime` | Complete descriptor plus UUID | Require one matching descriptor in the fresh HSA discovery. |
| `ProbeEngine::build_cache_identity` | Host, GPU, peer, seed fields | Hash all identity facets and capability inputs into `CacheIdentity`. |
| `ExplicitPathProfileCache::load_existing` | Cache schema and digest | Decode the profile and reject a stale file identity before returning it. |
| `MeasuredProfile::resolve_local_inventory` | Machine fingerprint, RAM/storage/GPU keys, GPU target | Require exhaustive current inventory with exactly the same sets; no ordinal/name/capacity/performance fallback. |
| `with_native_execution_bindings` | Resolved device IDs and descriptors | Reopen exact backends and require the same GPU set and target identities before lending bindings. |
| `src/native_prepare.rs` target planning | `TargetIdentity`, `ToolchainIdentity`, runtime policy | Reject missing exact bindings, changed target/toolchain, unsupported backend, or non-equivalent build specs. |
| active native receipt (`src/cli.rs`) | Canonical paths and raw file digests | Reinspect every pinned library/tool; changed path or bytes requires rerunning `recipe probe`. |
| `prepare/src/production.rs` | Target and toolchain in `TargetBuildSpec` and `ArtifactIdentity` | Reject zero toolchain digest or target/runtime-policy disagreement before artifact realization. |
| `planner/src/planner.rs` | Artifact target/toolchain fields | Include labels and raw toolchain digest in candidate identity hashing. |
| `training/src/execute.rs` and checkpoint | Realized native target/toolchain/image digest | Coalesce only exact images and reject zero, duplicate, or delimiter-bearing identities. |
| `src/training.rs` resume path | Topology, discovery, target, and toolchain identities | Reject a supplied native bundle made for another measured system or compiler. |
| `native-executor/src/candidate.rs` | Candidate topology/discovery identities | Reject a capacity snapshot from a profile different from the pre-final session. |

The native preparation boundary also reconstructs CUDA/HSA compiler targets from
the descriptor and compares them to measured target labels.  It passes the
descriptor's `ToolchainIdentity` into `TargetBuildSpec`; downstream artifact
identity and candidate planning therefore cannot silently substitute a new
compiler or target.

The root preparation path accepts only an identity-named profile file.  Its
basename must be `measured-v<decimal-schema>-<64 lowercase hex>.recipe-profile`
(`src/native_prepare.rs:413-465`), and the decoded schema/digest must equal the
profile's embedded `CacheIdentity`.  `with_current_native_preparation` then
retains the first opened `(NativeProbeConfig, NativeGpuProbe)` in thread-local
state and rejects a later call whose configuration differs.  This preserves
one initialized runtime without permitting a later call to bind a different
library, tool, PCI root, or benchmark configuration to the same thread.

The CLI persists the same handoff as a private `active-native-v1` receipt.  Its
fixed 16-field order is `profile`, `profile_schema`, `profile_digest`,
`host_memory_key`, `pci_sysfs_root`, `scratch_parent`, `cuda_library`,
`hsa_library`, `llvm_opt`, `llvm_llc`, `lld`, `ptxas`, `ptx_isa`,
`hsa_code_object_version`, `release`, and `fma_chain_length` (`src/cli.rs:41-58`).
Paths and raw digests are lowercase hex, labels are UTF-8 hex, optional pins
are `none`, and scalar settings are decimal.  Capture and reopen re-inspect
canonical files and compare both path and bytes; the receipt is rejected if it
is noncanonical, changed while opened, non-private, or owned by another user.

The same identity remains authenticated after preparation:

* `prepare/src/production.rs:192-250` rejects a zero toolchain digest, checks
  that the target backend, architecture, ABI, and runtime policy agree, and
  copies the exact target and `ToolchainIdentity` into every `ArtifactIdentity`.
* `planner/src/planner.rs:765-785` includes artifact ID, image digest, format,
  target labels, toolchain name/version/raw digest, entry symbol, resources,
  and build provenance in the candidate artifact hash.  A changed toolchain
  therefore changes candidate identity even when the graph is unchanged.
* `training/src/execute.rs:274-365` retains target, toolchain, and image digest
  for each realized native kernel.  Native images are coalesced only when
  format, target, toolchain, digest, and bytes all match.
* `src/training.rs:1285-1301` rejects a resumed native bundle unless its
  topology and discovery identities and at least one current target build spec
  match both target and toolchain exactly.
* `training/src/checkpoint.rs:4917-4988` requires nonzero measured-system,
  image, and toolchain digests, forbids OGDL delimiter characters in identity
  labels, and rejects duplicate `(format, target, toolchain, image digest)`
  tuples.  The semantic model stores this identity metadata, never a substitute
  compiler or a reconstructed native image.
* `native-executor/src/candidate.rs:323-407` captures topology and discovery
  identities in each validated candidate session and rejects a capacity
  snapshot whose profile identities differ.  The local factory retains the
  same identities with the pre-final native resources, so stabilization cannot
  silently move a candidate to another measured system.

## Failure matrix

The table lists every identity-specific failure in `identity.rs`.  Operations
that merely propagate a native Driver or HSA error retain their caller's
context and are not converted into absence.

| Function | Condition | Failure |
| --- | --- | --- |
| `selected_library` | Candidate is relative | `Discovery`, candidate is not absolute. |
| `selected_library` | Candidate metadata fails other than `NotFound` | `Io`, inspect native backend library. |
| `selected_library` | Candidate is not a regular file or symlink | `Discovery`. |
| `selected_library` | Canonicalization fails, including dangling symlink | `Io`, canonicalize native backend library. |
| `selected_library` | Canonical target metadata fails or target is not a regular file | `Io` or `Discovery`. |
| `selected_library` | Canonical bytes cannot be read | `Io`, read native backend library. |
| `label` | Value is empty or whitespace | `Discovery` naming the field. |
| `backend_toolchain_identity` | Required backend tool is absent | `Discovery`, explicit linker or assembler required. |
| `backend_toolchain_identity` | Backend is not `amd-hsa` or `nvidia-cuda` | `Discovery`, unknown native toolchain backend. |
| `pci_accelerator_present` | Sysfs root is relative | `Discovery`. |
| `pci_accelerator_present` | Directory or entry enumeration fails | `Io`. |
| `pci_accelerator_present` | Any entry lacks readable/hex vendor or class | `Io` or `Discovery`; malformed unrelated entries are not skipped. |
| `pci_surface` | Sysfs root is relative or BDF path is not a directory | `Discovery` or `Io`. |
| `pci_surface` | Driver symlink cannot canonicalize | `Io`. |
| `surface_digest` | Non-NotFound, non-PermissionDenied read error | `Io`, read PCI identity surface. |
| `surface_digest` | Required driver surface has no readable file | `Discovery`, surface had no readable files. |
| CUDA descriptor | PCI syntax, PCI numeric mismatch, target conversion/validation, invalid library UTF-8 | `Discovery`. |
| HSA descriptor | Unstable UUID, missing PCI/queue/ISA/capability/KFD data, ambiguous target | `Discovery`. |

There is no retry loop, alternate hash, substitute path, downgrade, or
catch-all branch in these transitions.  A caller may report the error with a
benchmark or preparation prefix, but it must not reinterpret the identity as
unchanged.

## Invariants and change impact

* A `PinnedLibrary` always identifies canonical path plus the bytes actually
  read.  The original candidate spelling is never sufficient for equality.
* Missing backend libraries are optional only when PCI preflight proves no
  matching accelerator.  Hardware presence makes runtime absence fatal.
* `PciSurface` hashes ordered file paths and bytes, records readable-file
  count, and requires at least one driver file.  Firmware and link surfaces may
  be empty but still produce a deterministic digest.
* CUDA BDF components are checked numerically against Driver attributes and
  then rendered in one lowercase sysfs spelling.  HSA requires a stable UUID
  and exact PCI address.  Device keys are therefore UUID plus canonical PCI,
  not an ordinal or product name.
* `ToolchainIdentity.digest` binds backend, release, target and benchmark
  configuration, build-source digest, and the ordered paths and bytes of all
  required tools.  It is not enough to compare only the human-readable release
  label.
* Every identity string entering a descriptor is a validated `Label`; an empty
  or whitespace-only identity cannot reach the profile or cache.
* `GpuDescriptor` equality is the revalidation boundary for rediscovery and
  benchmark.  A physical device that changed any retained identity or required
  capability is a different device for this run.
* The cache hash includes all native identity facets and is computed after
  deterministic sorting.  A profile whose cache identity does not match the
  current machine is stale, even if its measured numbers look plausible.
* Native preparation uses retained stable keys and exact target/toolchain
  equality.  It cannot bind by capacity, name, ordinal, or benchmark similarity.

The smallest safe change to an identity input is therefore a deliberate schema
or domain-version change plus corresponding cache/profile invalidation.  A
caller must not alter one field, omit it from a hash, or add a fallback without
changing the contract that proves exact current hardware.

## Source map

The implementation and direct consumers covered by this document are:

| File | Relevant symbols |
| --- | --- |
| `native-probe/src/identity.rs` | `PinnedLibrary`, `selected_library`, `label`, `hex`, `backend_toolchain_identity`, `required_tools`, `hash_tool`, `artifact_digest_bytes`, `PciSurface`, `pci_accelerator_present`, `read_pci_hex`, `pci_surface`, `surface_digest`, `library_identity`. |
| `native-probe/build.rs` | `main`, `hash_field`, `hash_rustc_identity`, `collect_rust_files`. |
| `native-probe/src/cuda.rs` | `CudaBackend::open`, `descriptor`, `matching_device`, `parse_pci_bus_id`, `discover`. |
| `native-probe/src/hsa.rs` | `HsaBackend::with_runtime`, `descriptor`, `exact_target`, `hsa_target_tail`, `discover`. |
| `native-probe/src/native.rs` | `NativeGpuProbe::new`, diagnostics, `discover_all`, `benchmark_gpu`. |
| `probe/src/local.rs` | `LocalSystemDiscovery::discover_host`, `read_first_available`, `join_available`, host labels. |
| `probe/src/engine.rs` | `ProbeEngine::inspect`, `build_cache_identity`, `hash_seed`, `hash_machine`, `build_profile_digest`. |
| `probe/src/hash.rs` | `CanonicalDigest` byte encoding. |
| `probe/src/cache.rs` | exact cache load/store identity checks. |
| `probe/src/resolve.rs` | exact current-inventory resolution by retained keys and target identity. |
| `src/native_prepare.rs` | current profile/reopen checks and target/toolchain planning. |
| `src/cli.rs` | active native receipt capture/reopen and pinned-file verification. |

No runtime behavior is inferred from a printed identity or a successful hash
operation alone.  The proof is the complete path from current filesystem,
driver, HSA, and tool observations through descriptor equality, cache identity,
profile validation, and exact native preparation.
