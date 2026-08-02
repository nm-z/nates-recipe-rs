# Kernel target contract

This page documents the target value that crosses the `recipe-kernel` lowering
and native-artifact boundaries. The implementation source is
[`kernel/src/target.rs`](../../src/target.rs). A target is not a device ordinal,
driver handle, or a user-facing model target name. It is the exact AMDGPU or
NVIDIA code-generation identity used to lower LLVM IR, invoke the pinned
toolchain, and inspect the resulting native image.

The target flow is deliberately one-way:

```text
hardware/runtime discovery
  -> GpuDescriptor.target + GpuDescriptor.toolchain
  -> measured profile and exact native binding
  -> TargetBuildSpec { target: KernelTarget, toolchain_identity }
  -> LoweredKernel.target
  -> HSACO or cubin build and inspection
  -> RuntimeArtifactKind and native-executor target checks
```

No stage infers a replacement target when one of these identities disagrees.
The first boundary that can prove the disagreement returns an error.

## Public target types

`recipe-kernel` re-exports `AmdTarget`, `NvidiaTarget`, and `KernelTarget` from
[`kernel/src/lib.rs`](../../src/lib.rs). All three derive equality and ordering,
so they can be compared and used as map keys. `KernelTarget::validate` only
delegates to the selected backend descriptor.

| Type | Fields | Meaning | Derived spelling |
| --- | --- | --- | --- |
| `AmdTarget` | `target_id: String` | AMDGPU processor plus optional feature modifiers, without the `amdgcn-amd-amdhsa--` triple prefix | The target ID passed to LLVM as `-mcpu` and `-mattr` components |
| `AmdTarget` | `code_object_version: u8` | Required AMD HSA code-object version | Passed to AMD code-generation/linker options and HSACO inspection |
| `NvidiaTarget` | `sm_major: u8`, `sm_minor: u8` | NVIDIA compute capability | `architecture()` returns `sm_{major}{minor}` |
| `NvidiaTarget` | `ptx_isa: u16` | PTX ISA encoded as major times ten plus minor | `llvm_ptx_feature()` returns `+ptx{ptx_isa}` |
| `KernelTarget` | `Amd(AmdTarget)` or `Nvidia(NvidiaTarget)` | Backend choice and its complete code-generation descriptor | Variant determines LLVM intrinsics, address spaces, ABI, and artifact format |

Examples of complete values are:

```text
AmdTarget {
    target_id: "gfx1101:xnack-:sramecc+",
    code_object_version: 6,
}

NvidiaTarget {
    sm_major: 8,
    sm_minor: 6,
    ptx_isa: 75,
}
```

`AmdTarget.target_id` is the complete target tail, so feature modifiers are
part of equality and of the build identity. `NvidiaTarget.ptx_isa` is also part
of equality even though the profile's `TargetIdentity.architecture` carries
only `sm_86`; PTX is retained in the toolchain identity and in the LLVM
invocation.

## Validation in `target.rs`

The implementation performs the following checks, in this order.

### AMD

1. `validate_token("AMD target ID", target_id)` rejects an empty string and
   every byte other than ASCII alphanumeric, `_`, `-`, `+`, `:`, or `.`. This is
   a character check, not a complete AMD feature grammar.
2. The value must begin with the literal `gfx`.
3. `code_object_version` must be nonzero.

Failures use `LoweringErrorKind::InvalidTarget` with these messages:

```text
AMD target ID contains unsupported characters
AMD target ID must begin with `gfx`
AMDGPU code-object version must be nonzero
```

The source accepts a feature-bearing tail such as `gfx1101:xnack-`. It does
not reject a malformed colon component at this layer. The artifact builder's
`amd_processor_and_features` parser is the later, stricter check: every
component after the processor must have a nonempty ASCII alphanumeric or `_`
name followed by exactly `+` or `-`. It emits `InvalidTarget` for an empty
processor, a missing polarity suffix, or invalid feature characters. Thus a
value can pass `AmdTarget::validate` and still fail before tool invocation.

### NVIDIA

`NvidiaTarget::validate` requires:

```text
3 <= sm_major <= 12
0 <= sm_minor <= 9
32 <= ptx_isa <= 90
```

The checks are independent. For example, the type does not encode a list of
currently shipped GPU combinations, and it does not require a particular PTX
version for a particular SM. Failures are `InvalidTarget` with either
`unsupported NVIDIA target sm_{major}{minor}` or `unsupported PTX ISA {ptx}`.

`architecture()` and `llvm_ptx_feature()` are pure formatting helpers and do
not validate. Callers that construct a target from wider hardware integers
must first perform their checked conversions, then call `validate`.

### Common boundary behavior

`KernelTarget::validate` dispatches to the selected variant. The following
public entry points invoke it before using target data:

- `lower_elementwise` in [`kernel/src/llvm.rs`](../../src/llvm.rs);
- `lower_stage` in [`kernel/src/stage.rs`](../../src/stage.rs);
- `ArtifactBuilder::build`, `build_hsaco_bundle`, and
  `build_cubin_bundle` in [`kernel/src/builder.rs`](../../src/builder.rs).

`target.rs` does not probe hardware, check a driver, canonicalize a target
triple, inspect a native image, or verify that LLVM accepts a particular
feature. Those are separate fail-closed boundaries described below.

## Descriptor and identity mapping

The kernel target is derived from an observed backend descriptor. The opaque
core identity is `recipe_core::TargetIdentity`:

```text
TargetIdentity {
    backend: Label,
    architecture: Label,
    abi: Label,
}
```

It is carried by `GpuDescriptor`, `CalculationCapability`, discovery profiles,
artifact identities, and finalized plans. The core type does not reinterpret
the strings. Native probing and preparation are responsible for constructing
an exactly matching `KernelTarget`.

### CUDA mapping

[`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs) reads the CUDA
Driver's `ComputeCapability { major, minor }`, converts each component to
`u8`, and constructs:

```text
NvidiaTarget {
    sm_major: compute_capability.major as u8,
    sm_minor: compute_capability.minor as u8,
    ptx_isa: NativeProbeConfig.cuda.ptx_isa,
}
```

The target is validated immediately. The descriptor then uses:

```text
TargetIdentity.backend       = "nvidia-cuda-driver"
TargetIdentity.architecture = target.architecture()   # for example sm_86
TargetIdentity.abi          = "elf64-cubin"
```

The descriptor's architecture is therefore the same `sm_XX` string used by
LLVM and `ptxas`. PTX ISA is not folded into that architecture string; it is
part of the target value, the probe toolchain configuration, and the core
`ToolchainIdentity` digest.

The benchmark re-derives the same target from the live CUDA context before
lowering its Recipe-owned FMA kernel. A changed compute capability or a value
outside the `u8` or target validation ranges is a benchmark/discovery error,
not a fallback to another SM.

### HSA mapping

ROCr exposes one or more parsed `hsa::IsaTarget` values. Each value has a full
string such as:

```text
amdgcn-amd-amdhsa--gfx1101:sramecc+:xnack-
```

[`native-probe/src/hsa.rs`](../../../native-probe/src/hsa.rs) uses
`exact_target` to choose one target:

- all ISA entries must be AMDGPU targets;
- multiple non-`-generic` entries, including duplicate entries, are rejected;
- if every entry is generic, all entries must have the same full identity;
- no target, multiple specific targets, or ambiguous generic targets is an
  error.

`hsa_target_tail` then removes the exact
`amdgcn-amd-amdhsa--` prefix. The resulting tail is the `AmdTarget.target_id`:

```text
AmdTarget.target_id           = "gfx1101:sramecc+:xnack-"
AmdTarget.code_object_version = NativeProbeConfig.hsa.code_object_version
```

The measured descriptor retains the full identity and code-object ABI:

```text
TargetIdentity.backend       = "amd-rocr-hsa"
TargetIdentity.architecture = "amdgcn-amd-amdhsa--gfx1101:sramecc+:xnack-"
TargetIdentity.abi           = "elf64-amdgpu-code-object-v6"
```

The exact HSA binding also retains the full target string. This intentional
split is important:

| Boundary | AMD target spelling |
| --- | --- |
| `AmdTarget` and LLVM `-mcpu`/`-mattr` | tail beginning with `gfx` |
| `TargetIdentity.architecture`, HSA binding, and runtime artifact kind | full `amdgcn-amd-amdhsa--` string |
| HSACO inspector expected value | `AmdTarget.target_id`; actual metadata may be full or tail and is compared after removing the triple prefix |

The HSA `IsaTarget` parser is stricter than `AmdTarget::validate`: it requires
the canonical prefix, lowercase architecture/feature names, `+` or `-`
feature suffixes, and no duplicate feature names. A target produced by the
normal HSA path has already passed that parser. Direct callers of
`recipe_kernel` still rely on the builder's feature parser and the artifact
inspector for their later checks.

## Toolchain and feature mapping

`NativeProbeConfig` in [`native-probe/src/config.rs`](../../../native-probe/src/config.rs)
contains the target-affecting settings:

| Configuration | Used by | Identity impact |
| --- | --- | --- |
| `cuda.ptx_isa` | `NvidiaTarget.ptx_isa`, `+ptxN`, descriptor toolchain configuration | Changes LLVM code generation, probe toolchain digest, and CUDA artifact policy |
| `hsa.code_object_version` | `AmdTarget.code_object_version`, HSACO ABI, descriptor ABI | Changes linker flags, metadata expectation, runtime ABI, and toolchain configuration |
| `kernels.toolchain` | verifier, LLVM code generator, AMD linker, NVIDIA `ptxas` | Every path is pinned by canonical absolute path and binary digest |
| `kernels.release` | core toolchain version label | Included in the toolchain identity digest |
| `kernels.fma_chain_length` | bounded probe kernel and target configuration text | Makes the measured toolchain/profile identity sensitive to the probe workload |
| `kernels.scratch_parent` | temporary compiler workspace | Must be absolute and private; it is retained in each `TargetBuildSpec` |

`NativeGpuProbe::new` validates the nonzero FMA chain and absolute scratch
parent. `HsaBackend::new` separately rejects code-object version zero. A CUDA
PTX value is checked when a CUDA target is constructed. Current CLI-generated
defaults are PTX 7.4, HSA code-object version 6, and a 64-FMA probe chain; the
values are configuration defaults, not constants in `target.rs`.

### Pinned tool set

[`kernel/src/builder.rs`](../../src/builder.rs) defines `OfflineToolchain` as:

- mandatory verifier (`opt`) and LLVM code generator (`llc`);
- optional pinned ELF linker (`lld`), required by every AMD build;
- optional pinned PTX assembler (`ptxas`), required by every NVIDIA build.

`ArtifactBuilder::new` canonicalizes and digests every supplied tool. Each
invocation verifies its path and digest again. A missing backend-specific tool
is reported as `InvalidTarget`; changed path or bytes is
`ArtifactMismatch`; execution failure is `ToolchainFailed`; filesystem and
workspace failures are `Io`.

`native-probe/src/identity.rs` computes the core `ToolchainIdentity` digest
from a domain tag, backend name, release label, target configuration string,
the native-probe source digest, and the path plus digest of each required tool.
The target configuration strings are:

```text
nvidia-cuda: {architecture}:ptx{ptx_isa}:dependent-f32-fma-chain-{length}
amd-hsa:    {target_tail}:code-object-v{code_object_version}:dependent-f32-fma-chain-{length}
```

The resulting identity is named `recipe-owned-llvm-{backend}` and is stored
on the measured GPU descriptor. It prevents a target or probe/tool change from
being mistaken for the same measured build environment.

The CUDA runtime policy has a second, legacy-shaped identity in
[`src/native_prepare.rs`](../../../src/native_prepare.rs): it records the
PTX ISA, `ptxas` digest, LLVM release and tool digests, cubin format, driver
range, and required Driver API symbols. It explicitly records that Recipe uses
Rust-owned LLVM IR and pinned `ptxas`, not a CUDA toolkit API. This policy is
distinct from the core `ToolchainIdentity`, but both must agree before an
artifact enters a finalized plan.

## Production target construction

The root preparation path reopens the exact measured profile and native
bindings in [`src/native_prepare.rs`](../../../src/native_prepare.rs).
`build_scope` partitions devices by the descriptor backend, creates one
`TargetBuildSpec` per unique `TargetIdentity`, and requires all devices sharing
that identity to produce equivalent target, toolchain, scratch, and runtime
policies.

### CUDA `TargetBuildSpec`

`cuda_spec` converts the live `DeploymentIdentity.target` to `u8` SM fields and
uses the configured PTX ISA. It then requires all of the following:

```text
descriptor.target.backend      == "nvidia-cuda-driver"
descriptor.target.architecture == target.architecture()
descriptor.target.abi          == "elf64-cubin"
all REQUIRED_DRIVER_SYMBOLS are present in the deployment
```

The spec stores `KernelTarget::Nvidia(target)`, the descriptor's core
toolchain identity, the pinned `ArtifactBuilder`, and a CUDA runtime policy.

### HSA `TargetBuildSpec`

`hsa_spec` requires the descriptor architecture to equal the binding's full
target string and the binding code-object version to equal the configured
version. It then requires the binding string to have the canonical
`amdgcn-amd-amdhsa--` prefix, strips that prefix, and constructs
`KernelTarget::Amd` from the tail and binding version. The resulting spec must
have:

```text
descriptor.target.backend == "amd-rocr-hsa"
descriptor.target.abi     == "elf64-amdgpu-code-object-v{version}"
descriptor.target.architecture == tail or full target, as accepted by TargetBuildSpec::validate
```

The normal root path has the full architecture in the descriptor and binding;
the acceptance of a tail in `TargetBuildSpec::validate` exists for already
constructed specifications and does not authorize a binding mismatch.

`DeferredArtifactCompiler` sorts specs by `TargetIdentity`, rejects duplicate
identities, and later maps each candidate artifact to exactly one measured
target. An artifact assigned to no target or more than one target is rejected.

## Consumers of `KernelTarget`

### LLVM lowering

`lower_elementwise` and `lower_stage` validate the target and copy it into
`LoweredKernel.target`. The target variant changes generated IR in these ways:

| Concern | AMD | NVIDIA |
| --- | --- | --- |
| Global index intrinsics | `llvm.amdgcn.workitem.id.x`, `llvm.amdgcn.workgroup.id.x` | `llvm.nvvm.read.ptx.sreg.tid.x`, `llvm.nvvm.read.ptx.sreg.ctaid.x` |
| Barrier intrinsic | `llvm.amdgcn.s.barrier` | `llvm.nvvm.barrier0` |
| LLVM triple | `amdgcn-amd-amdhsa` | `nvptx64-nvidia-cuda` |
| Kernel calling convention | `amdgpu_kernel` | `ptx_kernel` |
| Private address space for owned local slots | address space `5` | generic address space `0`, allowing NVPTX lowering to select local memory |
| Target-specific algorithm choice | no SM gate | TF32 matrix contraction is eligible when `sm_major >= 8` |

The SM and PTX values do not alter the variant-specific LLVM intrinsic set,
but they remain attached to the lowered module and control the later codegen
flags. `ArtifactBuilder` rejects a lowered module whose exact target differs
from the requested build target, even when both modules use the same backend
variant.

### Native artifact builder

`ArtifactBuilder::build` first verifies the target and exact
`LoweredKernel.target`, then runs the pinned verifier. It dispatches by
`KernelTarget`:

- AMD single-image builds run `llc -march=amdgcn -mcpu={processor}` with
  `--amdhsa-code-object-version={N}` and optional `-mattr={features}`, then
  link with pinned `lld` to `kernel.hsaco`. The resulting ELF metadata is
  inspected for the expected target ID, code-object version, symbols, and
  kernel ABI.
- NVIDIA single-image builds run `llc -march=nvptx64 -mcpu={sm}` with
  `-mattr=+ptx{P}`, then pinned `ptxas -arch={sm}` to `kernel.cubin`. The
  resulting ELF is inspected for the expected SM, entry symbol, executable
  text, and ABI.

The bundle methods apply the same mapping to every lowered entry, reject an
empty bundle, require exact target equality for every entry, and reject a
duplicate entry symbol. AMD bundles use one full-LTO `lld` invocation so HSA
metadata is merged; NVIDIA bundles lower each module to PTX and pass all PTX
units to one `ptxas` invocation. Input order is deterministic and retained in
the build provenance.

`nvidia_sm` uses checked multiply/add when converting the two SM bytes to the
single-byte inspection identity. Although `NvidiaTarget::validate` bounds the
normal range, this remains a separate checked failure boundary.

### Artifact inspection

[`kernel/src/artifact.rs`](../../src/artifact.rs) performs structural checks,
not string-only success checks:

- HSACO requires AMDGPU-HSA ELF, computes code-object version from the ELF ABI
  byte, reads `amdhsa.target` metadata, canonicalizes both expected and actual
  target IDs by removing an optional triple prefix, and compares the processor
  plus feature map. Duplicate or malformed feature modifiers are rejected.
- Cubin requires an accepted CUDA ELF OS-ABI and CUDA machine, decodes the SM
  from the known ELF flag layout, and requires the expected SM and exact entry
  symbol with nonempty executable text.

Inspection failures are `ArtifactFormat` for malformed ELF/metadata and
`ArtifactMismatch` for a valid artifact whose target, code-object version, SM,
symbol, or ABI differs from the request.

### Preparation and native execution

[`prepare/src/production.rs`](../../../prepare/src/production.rs) groups
deferred stage builds by `TargetBuildSpec.target`. It lowers each stage with
that exact target, optionally inspects a supplied prebuilt bundle, or invokes
the corresponding AMD/NVIDIA bundle builder. Runtime metadata is then formed
as follows:

```text
KernelTarget::Nvidia -> RuntimeArtifactKind::Cuda {
    ComputeCapability(sm_major, sm_minor),
    CUDA artifact policy,
}

KernelTarget::Amd -> RuntimeArtifactKind::Hsa {
    target_id: TargetIdentity.architecture (full HSA string),
    code_object_version,
}
```

The finalized artifact identity must use the matching backend, architecture,
ABI, format, digest, entry symbol, and toolchain identity. The native executor
rechecks this relationship before loading a module. CUDA compares the cubin
target with the live deployment identity and validates driver compatibility.
HSA compares the runtime target and code-object version with the live binding,
then re-inspects every entry in the loaded HSACO.

The measured profile codec stores the three `TargetIdentity` labels. Live
resolution requires the current descriptor target to equal the cached target;
target drift is a profile mismatch, not a reason to reuse an old artifact.

## Failure matrix

| Boundary | Observable failure | Classification |
| --- | --- | --- |
| `AmdTarget::validate` or `NvidiaTarget::validate` | malformed token, non-`gfx` AMD ID, zero code-object version, unsupported SM, or unsupported PTX | `LoweringErrorKind::InvalidTarget` |
| AMD builder feature parsing | missing `+`/`-`, empty processor, invalid feature name | `InvalidTarget` |
| Lowering contract | target invalid or requested target differs from the lowered module | `InvalidTarget` or `ArtifactMismatch` |
| `ArtifactBuilder` setup | pinned path is not absolute/canonical, tool bytes changed | `Io` or `ArtifactMismatch` |
| Backend build | AMD linker or NVIDIA `ptxas` absent | `InvalidTarget` |
| Tool invocation | verifier, `llc`, linker, or assembler exits unsuccessfully | `ToolchainFailed` with bounded stderr |
| Native output | malformed ELF, wrong machine/ABI, target metadata mismatch, wrong SM, missing symbol, or bad kernel metadata | `ArtifactFormat` or `ArtifactMismatch` |
| CUDA discovery/preparation | compute capability conversion/validation or descriptor versus deployment mismatch | probe discovery error or `IdentityMismatch` |
| HSA discovery/preparation | no exact ISA, ambiguous ISA, prefix mismatch, target/version drift, or descriptor versus binding mismatch | probe discovery error or `IdentityMismatch` |
| Deferred preparation | missing/duplicate target spec, artifact assigned to multiple targets, prebuilt bundle mismatch | `NativePrepareError` target/configuration/candidate/artifact failure |
| Finalized native executor | runtime kind, target, ABI, digest, or live binding mismatch | `ArtifactMismatch` |

No branch silently substitutes a generic target, a nearby SM, a different PTX
ISA, a different HSA feature set, or a newly discovered toolchain. A failed
profile reopen or artifact check must remain visible to the caller.

## Direct examples

The two kernel examples are intentionally small direct producers:

- [`kernel/examples/lower_add.rs`](../../examples/lower_add.rs) accepts a
  `gfx...` string or two-digit `sm_MM`, uses code-object version 6 or PTX 75, and relies on
  `lower_elementwise` for the authoritative validation.
- [`kernel/examples/inspect_add.rs`](../../examples/inspect_add.rs) constructs
  the same two target shapes and passes the AMD tail plus code-object version
  to `inspect_hsaco`, or the checked SM identity to `inspect_cubin`.

These examples are not target discovery and do not define production defaults.
Production values come from `NativeProbeConfig`, the measured descriptor, and
the exact native binding described above.

## Source map

| Source | Responsibility |
| --- | --- |
| [`kernel/src/target.rs`](../../src/target.rs) | Public target descriptors, validation, architecture and PTX feature formatting |
| [`kernel/src/llvm.rs`](../../src/llvm.rs) | Backend-specific elementwise LLVM IR and `LoweredKernel.target` |
| [`kernel/src/stage.rs`](../../src/stage.rs) | Target-aware owned stage lowering and contract validation |
| [`kernel/src/builder.rs`](../../src/builder.rs) | Pinned tools, target flags, HSACO/cubin builds, and provenance |
| [`kernel/src/artifact.rs`](../../src/artifact.rs) | Structural native-image parsing and target/ABI inspection |
| [`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs) | CUDA descriptor, target construction, and live probe kernel |
| [`native-probe/src/hsa.rs`](../../../native-probe/src/hsa.rs) | HSA ISA selection, descriptor, target construction, and live probe kernel |
| [`native-probe/src/identity.rs`](../../../native-probe/src/identity.rs) | Target-sensitive core toolchain digest |
| [`native-probe/src/config.rs`](../../../native-probe/src/config.rs) | PTX, code-object, toolchain, release, and scratch configuration |
| [`src/native_prepare.rs`](../../../src/native_prepare.rs) | Exact measured binding to `TargetBuildSpec` mapping |
| [`prepare/src/production.rs`](../../../prepare/src/production.rs) | Deferred target grouping, bundle realization, and runtime artifact metadata |
| [`native-executor/src/plan.rs`](../../../native-executor/src/plan.rs) | Final runtime artifact target and ABI checks |
| [`probe/src/model.rs`](../../../probe/src/model.rs) and [`probe/src/resolve.rs`](../../../probe/src/resolve.rs) | Descriptor/profile storage and cached-target drift rejection |
