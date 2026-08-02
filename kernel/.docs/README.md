# `recipe-kernel`

`recipe-kernel` is Recipe's compiler boundary. It turns the backend-neutral
scalar and primitive-stage contracts into direct target LLVM IR, then turns
already-lowered IR into inspected AMDGPU HSACO or NVIDIA cubin bytes through a
pinned offline toolchain. It also inspects those bytes before they cross into a
native runtime.

The crate owns lowering and artifact identity checks. It does not own graph
construction, primitive semantics, placement, scheduling, allocation, driver
contexts, queues, execution lifecycle, model serialization, or public model
artifact policy. A kernel is compiled during `Realize`, before the immutable
`init -> loop -> exit` runtime. No compiler entry point exists for Finalize,
Init, Loop, or Exit.

The normative stage contract is kept beside this overview in
[`STAGE_LOWERING.md`](../STAGE_LOWERING.md). That document records the exact
stage validation boundary, native ABI, owned algorithms, and Philox V1
contract. The source links below point to the implementation that owns each
piece of the contract.

## Position in the pipeline

The complete artifact path is:

```text
recipe-language, recipe-ops, recipe-training
        |
        | typed scalar programs and placement-free primitive kernels
        v
recipe-primitives::lower
        |
        | LoweredProgram: buffers, ordered ProgramStage values,
        | dispatch geometry, bindings, synchronization, atomics,
        | fault contracts, resource bounds, canonical digest
        v
recipe-planner
        |
        | DraftPlan::artifact_builds contains target-independent
        | ArtifactBuildRecipe values for deferred stages
        v
recipe-prepare::DeferredArtifactCompiler (Realize only)
        |
        +--> recipe-kernel::lower_stage
        |       |
        |       +--> target LLVM IR and KernelAbi
        |       +--> IR declaration audit
        |       +--> immutable contract revalidation
        |
        +--> ArtifactBuilder::build_*_bundle
                |
                +--> pinned verifier/compiler/linker/assembler
                +--> HSACO or cubin bytes
                +--> ELF and kernel-metadata inspection
                v
        NativeArtifact / RuntimeArtifact
                |
                v
        recipe-native-executor loads once and launches by immutable ABI
```

There are two deliberate entry paths:

* The production path lowers each deferred `ArtifactBuildRecipe` and groups
  stages by one measured target. It builds one multi-entry HSACO or cubin per
  target and retains one logical artifact identity per entry point.
* The benchmark and standalone example paths call `lower_elementwise` for a
  validated `KernelTemplate`, then call `ArtifactBuilder::build` for one
  entry. They are useful for probing and diagnostics, but they do not replace
  production Draft, Realize, or native-executor validation.

A prebuilt production bundle follows the same lowering and ABI recovery path,
but skips compiler invocation after `inspect_hsaco_bundle` or `inspect_cubin`
has proved the supplied bytes. It is not a second lowering implementation.

## Manifest and module graph

`kernel/Cargo.toml` declares package `recipe-kernel` version `0.1.0`, Rust
edition 2024, MIT licensing, and these dependencies:

| Dependency | Role at this boundary |
| --- | --- |
| `recipe-core` | `DType`, scalar SSA, kernel templates, IDs, units, `ArtifactBuildRecipe`, digests, and resource contracts. |
| `recipe-language` | Index policies, reduction and sort descriptors, scatter conflicts, and other primitive-stage semantic enums. |
| `recipe-primitives` | `LoweredProgram`, `ProgramStage`, `StageKind`, stage contracts, resource bounds, fault contracts, and Philox/sort/tree records. |
| `sha2` | SHA-256 for pinned tool identity, IR/image identity, and the independently recomputed artifact-build contract digest. |

The crate root forbids unsafe code and denies missing `Debug` implementations.
All implementation modules are private; `kernel/src/lib.rs` is the only public
facade:

```text
lib.rs
  |
  +-- error.rs       LoweringError and machine-readable failure kinds
  +-- target.rs      AmdTarget, NvidiaTarget, KernelTarget validation
  +-- audit.rs       closed-module LLVM declaration audit
  +-- llvm.rs        scalar KernelTemplate lowering and scalar ABI
  +-- stage.rs       artifact-contract verification and owned stage lowering
  +-- builder.rs     pinned tool execution and HSACO/cubin construction
  +-- artifact.rs    ELF/AMDGPU metadata/cubin inspection and SHA-256 digest
```

The implementation dependencies are one-way:

```text
recipe-core + recipe-language + recipe-primitives
       |             |                 |
       +-------------+-----------------+--> stage.rs
recipe-core -------------------------------> llvm.rs
target.rs + error.rs ----------------------> llvm.rs, stage.rs, builder.rs,
                                             artifact.rs, audit.rs
llvm.rs -------------------------------> stage.rs and builder.rs
artifact.rs ----------------------------> builder.rs
audit.rs --------------------------------> llvm.rs and stage.rs
```

| Module | Public responsibility | Explicit non-responsibility |
| --- | --- | --- |
| [`error.rs`](../src/error.rs) | Classifies every lowering, target, contract, tool, I/O, format, and ABI failure in `LoweringErrorKind`; optionally attaches a `ScalarValueId`. | It does not recover from a failed transition or invent a fallback. |
| [`target.rs`](../src/target.rs) | Validates AMD target IDs and code-object versions, NVIDIA SM/PTX identities, and the backend enum. | It does not discover hardware or select a target from a profile. |
| [`audit.rs`](../src/audit.rs) | Finds external LLVM declarations, allowing only `llvm.*` intrinsics. | It does not duplicate the repository and final-ELF policy owned by `recipe-audit`. |
| [`llvm.rs`](../src/llvm.rs) | Lowers one validated `KernelTemplate` to direct AMDGPU or NVPTX LLVM, constructs `KernelAbi`, and counts work. | It does not lower reductions, scans, gathers, atomics, or other `ProgramStage` kinds. |
| [`stage.rs`](../src/stage.rs) | Recomputes and validates the planner contract, emits every owned `StageKind`, creates stage ABI, and audits the resulting module. | It does not choose a stage, change geometry, repair a stale recipe, or schedule a kernel. |
| [`builder.rs`](../src/builder.rs) | Verifies pinned tools, runs deterministic offline compilation, returns bytes plus provenance, and performs post-build inspection. | It does not lower source graphs or load a driver module. |
| [`artifact.rs`](../src/artifact.rs) | Parses only the required little-endian ELF structures, HSA MessagePack metadata, and CUDA symbol/section data; matches exact target and ABI. | It does not execute bytes or accept unresolved native symbols. |
| [`lib.rs`](../src/lib.rs) | Re-exports the stable surface and forwards `lower_stage` and the contract digest to the private stage module. | It does not hold mutable compiler or runtime state. |

## Public surface and intent vocabulary

The public names have one purpose each. Keeping these meanings distinct is
important when reading callers:

| Name | Intent | Result or proof |
| --- | --- | --- |
| `lower_elementwise` | Lower a complete core `KernelTemplate` with its scalar program and affine input/output views. | `LoweredKernel` containing target LLVM text, `KernelAbi`, total `FlopCount`, and the exact `KernelTarget`. |
| `lower_stage` | Realize one planner-owned `ProgramStage` described by a complete `LoweredProgram` and `ArtifactBuildRecipe`. | A target-specific `LoweredKernel`; any mismatch is `InvalidStageContract`. |
| `artifact_build_contract_digest` | Recompute the planner's domain-separated digest independently at the realization boundary. | A `recipe_core::Digest` over every build field that affects ABI, work, geometry, bindings, faults, or resources. |
| `ArtifactBuilder::build` | Compile and inspect one already-lowered module. | `BuiltArtifact::Hsaco` or `BuiltArtifact::Cubin`, with image digest, inspection, tool identities, and invocation record. |
| `build_hsaco_bundle` / `build_cubin_bundle` | Compile deterministic, separately lowered entries into one target image. | A nonempty bundle with one inspection per input, preserving input order. |
| `inspect_hsaco` / `inspect_hsaco_bundle` | Prove ELF kind, code-object version, target ID, defined symbols, metadata, and exact explicit ABI. | `InspectedHsaco` values keyed by image digest and logical entry. |
| `inspect_cubin` | Prove CUDA ELF kind, encoded SM, defined entry function, executable `.text.<entry>` bytes, and no unresolved global symbols. | `InspectedCubin` keyed by image digest and entry. |
| `audit_llvm_ir` | Reject an unresolved external declaration before a toolchain sees the module. | A list of `AuditFinding`; an empty list is required for generated modules. |
| `PinnedTool` | Bind an absolute canonical executable path to its SHA-256 bytes. | Reverification failure is `ArtifactMismatch` or `Io`, never a silent tool substitution. |

## Target model

[`target.rs`](../src/target.rs) is intentionally small and explicit.

### AMD

`AmdTarget` contains the complete HSA target ID, including feature modifiers,
and a nonzero code-object version. Validation requires a nonempty token made of
ASCII letters, digits, `_`, `-`, `+`, `:`, or `.`, and requires the target ID to
begin with `gfx`. Build code splits the target at `:` into one processor and
optional `+feature` or `-feature` components. Feature names contain only ASCII
letters, digits, or `_`.

LLVM uses triple `amdgcn-amd-amdhsa` and calling convention
`amdgpu_kernel`. AMD global buffers and fault flags use address space 1,
shared workgroup storage uses address space 3, and private per-lane storage
uses address space 5. The builder passes `-march=amdgcn`, the processor, the
code-object version, and the validated feature list to LLVM and the linker.

### NVIDIA

`NvidiaTarget` contains `sm_major`, `sm_minor`, and a PTX ISA encoded as
`major * 10 + minor`. Validation accepts SM major 3 through 12, SM minor 0
through 9, and PTX ISA 32 through 90. `architecture()` returns `sm_MM`, and
`llvm_ptx_feature()` returns `+ptxNN`.

LLVM uses triple `nvptx64-nvidia-cuda` and calling convention `ptx_kernel`.
Global buffers and fault flags use address space 1. Generic address space 0 is
used for private `alloca` storage because NVPTX lowering rewrites generic
allocas to the target local space; emitting address space 5 directly can leave
invalid local pointers after optimization. The builder passes `-march=nvptx64`,
the SM architecture, and the PTX feature to LLVM, then passes the same
architecture to `ptxas`.

`KernelTarget::validate` dispatches to exactly one of these validators. No
other backend is accepted by this crate.

## Scalar `KernelTemplate` lowering

[`llvm.rs`](../src/llvm.rs) is the direct scalar consumer. `recipe-language`
and `recipe-ops` construct scalar programs, `recipe-primitives` embeds them in
`StageKind::ScalarMap`, and `lower_elementwise` emits one lane-wise target
kernel. The function does not call a host compiler, rewrite compiler text, or
delegate an operation to a vendor math library.

### Validation and emission order

`lower_elementwise` performs these steps in order:

1. Validate the complete `KernelTemplate`, including scalar SSA definitions,
   input/output arity and types, index space, affine access spans, and alias
   rules.
2. Validate `KernelTarget` and `LoweringOptions`.
3. Emit target work-item and workgroup IDs, calculate a 64-bit global lane,
   reject lanes outside `%element_count`, and enter the body only for an
   in-bounds lane.
4. Convert each static input view into an affine element index, load one value
   from global memory, and bind it to the matching `ScalarValueId`.
5. Materialize scalar constants without a host f32 round trip. `F32Bits` is
   bitcast from its preserved integer bits.
6. Lower each ordered `ScalarInstruction`, accumulating checked per-element
   FLOPs from `ScalarOpcode::flops()`.
7. Publish one combined scalar fault flag when checked instructions rejected an
   input, then calculate each output affine index and store its typed value.
8. Assemble a complete LLVM module, audit declarations, derive the ordered
   ABI, and multiply per-element work by the index-space element count.

The generated entry arguments are ordered input buffers, output buffers, an
optional `FaultFlag`, and `ElementCount`. Every pointer or by-value slot is
eight bytes in the ABI record, with eight-byte argument alignment. Input and
output pointer alignment metadata is four bytes because the payload types are
only `f32` and `i32`.

### Scalar opcode mapping

The scalar lowerer accepts the opcode and result-type combinations that core
validation defines. The implementation still checks the combination at the
lowering boundary so an invalid or newer program cannot become arbitrary LLVM:

| Scalar family | Lowering | Fault or representation rule |
| --- | --- | --- |
| f32 add, subtract, multiply, remainder | `llvm.experimental.constrained.*.f32` with round-to-nearest and ignored FP exceptions | Result is canonicalized to Recipe's f32 NaN. |
| f32 divide | LLVM `fdiv`, followed by NaN canonicalization | Kept as the scalar contract's division operation. |
| f32 FMA | constrained fma intrinsic | Counts as two FLOPs. |
| f32 minimum, maximum | `llvm.minimum.f32` or `llvm.maximum.f32` | Result is canonicalized to the module's canonical NaN. |
| f32 negate, absolute | bitcast to i32, XOR or AND sign bits, bitcast back | Does not call a runtime helper. |
| f32 comparisons | ordered or unordered `fcmp` predicate selected by opcode, then zero-extend to i32 | Comparison results are always i32. |
| f32 square root, floor, ceiling, round-nearest-even | corresponding LLVM intrinsic | Intrinsics are declared only when used. |
| i32 add, subtract, multiply | direct LLVM integer operation | Core validation owns operand/result typing. |
| i32 divide and remainder | checked `sdiv` or `srem` | Zero divisor and `MIN / -1` overflow are rejected and return zero. |
| i32 negate and absolute | checked min-value path | `i32::MIN` is rejected and returns zero. |
| i32 minimum, maximum, comparisons | signed `icmp` and `select` | Comparison results are zero-extended i32. |
| i32 bit and/or/xor/not and shifts | direct bit operation; shift count masked to 31 | Bit and shift operations on f32 are rejected. |
| i32 `Require` | accepted when nonzero, return the accepted bit, record rejection otherwise | Rejection sets the fault channel. |
| i32 `IsFinite` and `IsNan` | f32 bit classification, result zero-extended to i32 | The operand is an f32 even though the result type is i32. |
| f32 to i32 and i32 to f32 | saturating `fptosi` or signed `sitofp` | A mismatched result type is `UnsupportedOperation`. |
| bitcasts | exact `f32`/`i32` LLVM bitcast | No numeric conversion occurs. |
| any other opcode/type pair | no fallback | `UnsupportedOperation` identifies the scalar result when possible. |

All constrained f32 arithmetic is emitted in a `strictfp` entry with IEEE
denormals and `no-trapping-math` set to false. Every raw f32 result that can
carry NaN passes through the canonical NaN value. The only external LLVM
declarations are target intrinsics and the selected LLVM arithmetic intrinsics;
`audit_llvm_ir` rejects every other declaration.

### Scalar fault behavior

The standalone scalar emitter combines all rejection conditions into one branch
and emits `atomicrmw or` of one into `%fault_flag` with monotonic ordering. A
planner-owned scalar stage has a stronger contract: `stage.rs` requires its
fault binding and replaces that generic publication with
`atomicrmw xchg` of the exact planned fault code and release ordering. This
rewrite is checked: if the generic publication is absent, realization fails
instead of silently accepting a scalar module with the wrong fault ABI.

## Planner stage realization

[`stage.rs`](../src/stage.rs) is the contract gate for every deferred primitive
stage. Its input is the complete `LoweredProgram`, one
`ArtifactBuildRecipe`, a validated target, and lowering options. The complete
program is required even when one stage is being lowered because the stage
ordinal, source-kernel identity, program digest, scan-axis context, and stage
template identity are not safe to infer from a fragment.

### Independent contract checks

Before emitting any stage IR, `validate_contract` requires:

* canonical validation of `LoweredProgram` and `ArtifactBuildRecipe`;
* a valid target and an ASCII entry symbol with a workgroup size from 1
  through 1024;
* `build.provenance.program_digest` equal to the canonical lowered-program
  digest;
* `build.source_kernel` equal to `program.source_kernel`;
* `build.provenance.stage_ordinal` naming exactly one stage;
* `build.kernel_template` equal to the independently derived
  `recipe-planner-stage-template-v1` identity over program digest, source
  kernel, and stage ordinal;
* `build.artifact` equal to that reserved stage-scoped identity;
* `build.provenance.contract_digest` equal to the independently recomputed
  `recipe-planner-artifact-build-v1` digest;
* logical lane count, workgroup lane count, and workgroup count equal to the
  stage geometry and exact ceiling division;
* `ArtifactWorkBounds` equal to the stage's f32, integer, and atomic bounds;
* private, shared, scratch, and maximum-workgroup resource values equal to the
  stage resource envelope, with scratch required to be zero at this boundary;
* binding count, dtype, access mode, logical extents, offset, strides, and
  storage bytes equal to the stage bindings in order; and
* exact fault binding presence, value ID, int32 type, read-write-atomic access,
  and four-byte storage when the stage declares a `FaultContract`.

Every failed check returns `LoweringErrorKind::InvalidStageContract`. A
nearby-looking stage, target, binding, or digest is never substituted.

The contract digest covers the artifact ID, stage template and source IDs,
program digest, stage ordinal, every binding value/type/access/view field,
dispatch geometry, all three work counters, optional fault value, and every
resource bound. Lengths are encoded before variable-length byte sequences, and
integer fields use little-endian bytes. The domain string is
`recipe-planner-artifact-build-v1`.

### Stage ABI construction

`StageSignature::new` derives both LLVM parameters and `KernelArgument` values
from the ordered build bindings. The fixed explicit order is:

1. readable data pointers in stage-binding order;
2. writable data pointers in stage-binding order;
3. the optional fault-flag pointer;
4. the dynamic `RunId`, only for Philox stages;
5. the dynamic `LoopIteration`, only for Philox stages and index maps with a
   nonzero iteration step; and
6. `ElementCount`.

A read-write binding therefore receives two pointer slots, one in each pointer
phase. The binding object keeps separate read and write names so an emitter
cannot accidentally load from a write-only slot or store through a read-only
slot. Every explicit slot is eight bytes, and `KernelAbi::argument_bytes` is
the checked slot count multiplied by eight.

`RunId` and `LoopIteration` are launch-time values. They are never folded into
an AOT image, and a stage that does not declare the corresponding dynamic
contract cannot receive those arguments. HSA inspection requires the dynamic
values to be eight-byte `by_value` metadata arguments.

### Stage-kind dispatch

`lower_owned_stage` has one direct emitter for every `recipe_primitives::StageKind`:

| `StageKind` | Emitter and purpose |
| --- | --- |
| `ScalarMap { template }` | Routed through `lower_elementwise`; optional checked scalar fault publication is rewritten to the stage code. |
| `Fill { value }` | One in-bounds lane computes the affine output address and stores the typed literal. |
| `Copy` | One lane loads the source affine view and stores the destination affine view. |
| `FixedTreeReduce` | Shared-memory fixed tree, operator-identity padding, optional value/index outputs, and lowest-logical-index tie break. |
| `FixedTreeScanLocal` | Planner's fixed Blelloch upsweep and downsweep, including stage synchronization ordinals. |
| `ScanUniformCombine` | Finds the unique same-level local scan axis and applies the preceding hierarchy level. Ambiguous or missing context fails the contract. |
| `TiledContraction` | Traverses canonical contracted coordinates, uses the fixed tile and barriers, and selects the NVIDIA TF32 path only when its planned strategy requires it. |
| `Gather` | Applies reject, clamp, or wrap index policy before payload address calculation. |
| `Scatter` | Applies the same index policies and emits the planned direct or atomic conflict update. |
| `HistogramClear` | Zeroes the exact histogram binding. |
| `HistogramAccumulate` | Maps each input to a bin and applies the planned weighted/unweighted atomic operation and ordering. |
| `StableSortInitialize` | Builds IEEE total-order keys and original-axis-index tie-break state for the padded network. |
| `StableSortCompareExchange` | Emits one planned bitonic compare-exchange pair. |
| `StableSortFinalize` | Writes sorted values and, when requested, stable original indices. |
| `IndexMap` | Computes checked affine index arithmetic, optional Euclidean modulus, and code 4 arithmetic-domain fault publication. |
| `Philox4x32_10` | Emits Recipe-owned Philox V1, distribution mapping, and dynamic run/iteration inputs. |

If a `ScalarMap` reaches the owned-stage emitter, the function returns
`InvalidStageContract`. If a future stage kind is not represented by the
match, compilation fails at the source boundary rather than taking an
unrelated implementation.

### Shared IR and memory rules

The stage `Ir` accumulator keeps deterministic ordered declarations, globals,
and helper definitions. It emits target work-item and workgroup IDs, computes a
64-bit `%global_id`, and uses a common in-bounds prologue and return epilogue
for lane-wise stages. Affine indexing converts one linear logical lane into
coordinates using checked extents, then combines offsets and strides in
elements. Loads and stores are typed `f32` or `i32` global-memory operations
with four-byte alignment.

Collective stages use target barriers (`llvm.amdgcn.s.barrier` or
`llvm.nvvm.barrier0`) and shared address space 3. The emitter validates the
fixed tree or tile contract before using a synchronization point. Partial
workgroups still execute every required barrier, including inactive lanes.

Rejecting gather, scatter, histogram, index-map, and checked arithmetic paths
publish the exact planned fault code with `atomicrmw xchg` and release ordering.
The bounds or overflow branch always precedes the payload address calculation.
Integer atomics are direct LLVM atomics. Floating-point add, minimum, and
maximum atomics use Recipe-owned compare/exchange loops with constrained
arithmetic, total-order comparison, and canonical NaN output.

Generated modules declare only LLVM intrinsics. No module may call HIP, the
CUDA Runtime API, ROCr/HSA, rocBLAS, rocSOLVER, rocFFT, MIOpen, RCCL, cuBLAS,
cuSOLVER, cuFFT, cuDNN, NCCL, or another vendor operation library. Native
driver calls belong to `recipe-cuda`, `recipe-hsa`, and
`recipe-native-executor` after this crate has returned bytes.

### Algorithm contracts

The stage emitters consume already-fixed primitive semantics. They do not infer
an alternate algorithm from a shape or target:

* Reduction trees use the planner's strides and operator identity for inactive
  lanes. Indexed reductions preserve the lowest logical input index on ties.
* Local scans use the exact tree and synchronization list from the program.
  Uniform-combine stages recover their axis only from the unique matching local
  stage in the complete program.
* Contractions perform both barriers around every fixed reduction tile, even
  when some lanes are inactive. NVIDIA TF32 fragments are explicit bit-level
  conversions and matrix operations, not a vendor library call.
* Gather and scatter implement only the declared reject, clamp, or wrap policy.
  Reject branches write the exact fault code before any invalid pointer is
  formed.
* Histogram stages use the declared bin mapping, count, weighting, and atomic
  ordering. Histogram accumulation requires its checked fault contract.
* Stable sort uses the padded bitonic network, IEEE total-order keys, and the
  original axis index as its stable tie-break. Network shape and compare
  distance come from `SortNetwork` and `SortCompareStage`.
* Philox stages use the complete `Philox10Contract`, dynamic run identity, and
  the versioned mappings below. Recompiling for AMDGPU and NVPTX must preserve
  the same stream and mapping.

#### Philox4x32-10 V1

The four counter words are:

```text
[element_low, element_high, low(run_id XOR stream), high(run_id XOR stream)]
```

The key folds the 128-bit seed and source-kernel identity:

```text
key_0 = low(seed_low)  XOR high(seed_high) XOR low(source_kernel)
key_1 = high(seed_low) XOR low(seed_high)  XOR high(source_kernel)
```

The helper is unrolled to exactly ten rounds with multipliers
`0xd2511f53` and `0xcd9e8d57`. The first nine key advances use Weyl constants
`0x9e3779b9` and `0xbb67ae85`. `UniformF32` uses the upper 23 random bits for
`[0, 1)`. `BernoulliI32` compares a word with `floor(p * 2^32)` and treats
`p = 1` exactly. `UniformI32` uses unbiased multiply-high mapping, retries a
low-product rejection with counter word three XORed by the retry ordinal, and
then adds the lower bound. `NormalF32` uses two 24-bit uniforms in `(0, 1]`
and Recipe's Box-Muller V1 mapping. The owned logarithm uses the odd atanh
series through `y^15`; owned cosine reduces the angle and evaluates a Horner
polynomial through `x^10`. Arithmetic remains constrained, IEEE f32, and
canonical-NaN preserving.

Changing the counter layout, key fold, retry rule, distribution mapping, or
normal approximation is a new distribution version, not a harmless compiler
optimization.

## LLVM and interface audits

Both scalar and owned-stage lowering assemble their final module and call
[`audit_llvm_ir`](../src/audit.rs) before returning. The audit scans declaration
lines and reports every declared symbol that does not begin with `llvm.`. The
first finding becomes `LoweringErrorKind::ProhibitedInterface`, including the
line and token. LLVM target intrinsics are therefore allowed, while an
unresolved host/runtime or vendor-library call is rejected at the source
boundary.

This check is intentionally narrower than the repository-wide `recipe-audit`
gate. `recipe-audit` owns the final policy for repositories, dependencies,
linker inputs, dynamic loading, and final ELF contents. Keeping that policy in
one gate avoids divergent prohibited-name lists in the compiler crate.

## Pinned native artifact build

[`builder.rs`](../src/builder.rs) owns the compiler process boundary. It is
usable only with an `OfflineToolchain` containing:

* a pinned verifier, normally `llvm-opt`;
* a pinned LLVM code generator, normally `llc`;
* an optional pinned ELF linker, required for AMD HSACO; and
* an optional pinned PTX assembler, required for NVIDIA cubin.

`PinnedTool::inspect` requires an absolute regular-file path, canonicalizes it,
reads its bytes, and stores the SHA-256 digest. `ArtifactBuilder::new`
revalidates every configured tool. Every later invocation verifies the path and
digest again, so replacing a tool after configuration is an
`ArtifactMismatch`.

### Build phases and workspace

`BuildPhase` has only `Offline` and `Realize`. It records provenance and does
not authorize a third lifecycle phase. Current production callers pass
`BuildPhase::Realize`; a build performed in another phase is still visible in
the returned provenance and is rejected by the production prepare checks when
the phase is not `Realize`.

Every build receives an explicit scratch parent. The parent must be an existing
real directory with no group or other permissions. The builder creates a
unique `recipe-build-<pid>-<nonce>` child with mode 0700, creates input files
with mode 0600 and `create_new`, and removes the whole child on drop. Outputs
must be regular non-symlink files. Tool environments are cleared and set only
to `LC_ALL=C` and `SOURCE_DATE_EPOCH=0`. Compiler stderr is bounded to 16 KiB
in a `ToolchainFailed` error. Scratch paths in provenance are normalized to
`@scratch`, so the record is deterministic and does not leak host paths.

The verifier is always the first compiler operation. A failed command returns
the real tool status and bounded stderr. There is no retry, alternate tool,
fallback artifact, or compiler-diagnostic rewriting path.

### Single-entry build

`ArtifactBuilder::build` first validates the requested target and requires the
`LoweredKernel::target` to be identical. It writes `kernel.ll`, verifies it,
then selects one backend:

| Target | Tool sequence | Final proof |
| --- | --- | --- |
| AMD | `llvm-opt -passes=verify`, `llc -filetype=obj -march=amdgcn -mcpu=<processor> --amdhsa-code-object-version=<v>`, `lld -shared --no-undefined` | `inspect_hsaco(bytes, target ID, code-object version, KernelAbi)`. |
| NVIDIA | `llvm-opt -passes=verify`, `llc -march=nvptx64 -mcpu=sm_MM -mattr=+ptxNN`, `ptxas -arch=sm_MM` | `inspect_cubin(bytes, SM, entry symbol)`. |

The returned `BuiltArtifact` retains native bytes, the inspected ABI/entry,
and `BuildProvenance`. The single NVIDIA variant also retains generated PTX.
`BuildProvenance` records build phase, SHA-256 of the input LLVM, each distinct
pinned tool digest, and normalized invocation arguments.

### Multi-entry bundles

Production groups all lowered stages for one measured target in deterministic
input order. Empty bundles are `InvalidKernel`, mismatched targets are
`ArtifactMismatch`, and repeated entry symbols are `InvalidEntrySymbol`.

For AMD, each module is verified and serialized to bitcode, then one full-LTO
`lld` shared link emits `bundle.hsaco` with `--no-undefined`, `--lto-O0`, the
processor, feature list, and code-object version. Full LTO is required because
ordinary AMDGPU relocatable linking does not merge the per-object HSA metadata
notes on the supported LLVM toolchains. `inspect_hsaco_bundle` decodes the ELF
and metadata once, then checks every requested entry in the original order.

For NVIDIA, each module is verified and lowered to its own PTX translation
unit. One pinned `ptxas -arch=sm_MM` invocation receives all PTX units and
emits `bundle.cubin`. Each entry is inspected against its expected SM and
symbol. The returned `CubinBundleProvenance` retains one LLVM digest per input,
the tool identities, and the exact normalized invocation sequence.

## Native artifact inspection

[`artifact.rs`](../src/artifact.rs) contains a deliberately narrow ELF parser,
not a general object-file dependency. `ArtifactDigest` is SHA-256 over the
complete byte image. ELF parsing requires a 64-bit little-endian file, a
well-bounded section table, valid NUL-terminated UTF-8 string tables, valid
symbol table entry sizes, and every file-backed section range in bounds.
Undefined global or weak symbols are always rejected.

### HSACO checks

`inspect_hsaco_bundle` proves all of the following before returning an
`InspectedHsaco` per requested ABI:

1. ELF OS ABI is AMDGPU HSA and machine is `EM_AMDGPU`.
2. ELF ABI version plus two equals the expected code-object version.
3. The symbol table has no unresolved global/weak symbols.
4. AMDGPU MessagePack metadata exists in an `AMDGPU` note, decodes completely,
   has no duplicate map keys, and remains within the decoder's 128-level
   nesting limit.
5. Metadata contains a target ID equivalent to the expected processor and
   feature set. Full `amdgcn-amd-amdhsa--` prefixes are normalized only for
   comparison, not silently rewritten in the artifact.
6. Metadata contains a unique kernel list, and every requested entry has both
   a defined global function symbol and its `<entry>.kd` descriptor object.
7. The descriptor name, eight-byte kernarg alignment, kernarg size, maximum
   workgroup size, explicit argument offsets and sizes, value kinds, address
   spaces, and optional argument names match `KernelAbi`.
8. Only hidden trailing metadata arguments are allowed after the explicit
   ABI. A non-hidden trailing argument is an `ArtifactMismatch`.

Metadata names are optional where AMDHSA permits omission, but when a name is
present it must use the expected `input_`, `output_`, `fault_flag`, `run_id`,
`loop_iteration`, or `element_count` contract.

### Cubin checks

`inspect_cubin` proves ELF CUDA OS ABI (one of the explicitly supported CUDA
tags), `EM_CUDA`, the encoded SM layout for that ABI version, and the absence
of unresolved global/weak symbols. It then requires a defined global/weak
function with the requested entry name and a nonempty executable
`.text.<entry>` section. CUDA toolkit 13.3's ABI 8 layout is decoded from the
next ELF flags byte; older accepted layouts use the low byte. Other
architecture-specific OS ABI tags are not accepted by guessing.

## Direct consumers and ownership boundaries

The following callers are present in the workspace and use the public kernel
surface for distinct reasons:

| Consumer | Calls into `recipe-kernel` | What remains owned by the consumer |
| --- | --- | --- |
| `recipe-primitives` | Supplies `LoweredProgram` and `StageKind` values to the later realization boundary; it does not import target LLVM or compile bytes. | Primitive semantics, measured hardware limits, stage decomposition, fixed trees, atomics, faults, and resource formulas. |
| `recipe-prepare::DeferredArtifactCompiler` | Finds the exact lowered program, calls `lower_stage`, groups entries by target, calls `build_hsaco_bundle` or `build_cubin_bundle`, or validates a prebuilt bundle. | Candidate identity, measured target/toolchain mapping, runtime artifact identity, candidate stabilization, warm-up, capacity, and lifecycle handoff. |
| `native-probe` CUDA/HSA benchmarkers | Build a measured f32 FMA `KernelTemplate` with `lower_elementwise`, call single-entry `ArtifactBuilder::build`, inspect the artifact, then load and time it through the real backend. | GPU allocation, driver API, launch parameters, completion, and measured rate. |
| `src/native_prepare.rs` | Creates one pinned `ArtifactBuilder` per validated native configuration and builds `TargetBuildSpec` values for deferred realization. | Deployment identity, backend binding, exact measured target and toolchain policy, and scratch configuration. |
| `recipe-native-executor` | Uses `KernelAbi`, `KernelArgument`, `ArtifactDigest`, `inspect_cubin`, and `inspect_hsaco_bundle` before loading shared images and resolving logical entries. | Driver contexts, executable/module ownership, launch argument storage, queues, completion, cleanup, and runtime errors. |
| root facade and CLI | Re-export the crate and use `PinnedTool`/`OfflineToolchain` for configuration and receipt identity. | User declarations, probing, model persistence, and command orchestration. |

`recipe-kernel` never retains an allocator, driver handle, queue, module,
session, candidate, or finalized bundle. `recipe-native-executor` can group
multiple logical artifacts that share one image digest and load that image
once, while retaining each logical `KernelAbi`; this is why a bundle inspector
returns one inspection per requested entry rather than collapsing identities.

## End-to-end Realize flow

The production route is deliberately linear and fail closed:

```text
1. Draft stores ArtifactBuildRecipe with no target, ABI, entry, or tool.
2. Prepare maps the selected artifact to exactly one measured target.
3. Prepare finds the exact LoweredProgram by source-kernel ID and program digest.
4. Kernel validates the complete program, build recipe, target, and options.
5. Kernel emits one target-specific LoweredKernel and validates its ABI/work.
6. Prepare groups lowered entries by target and chooses one bundle build path.
7. Builder verifies each pinned tool, runs the deterministic tool sequence,
   reads the regular output, and records provenance.
8. Kernel artifact inspection proves the target, symbols, metadata, ABI, and
   absence of unresolved globals.
9. Prepare creates NativeArtifact and RuntimeArtifact identities from the
   image digest and logical ABI.
10. Native prepare/executor loads, reserves, warms, and observes the exact
    candidate. Only then can Finalize produce the runtime bundle.
```

When a prebuilt bundle is configured, steps 5 and 8 still occur. Only step 7's
compiler process is omitted. A missing or mismatched native image is therefore
a normal fail-closed artifact error, not a reason to compile a different stage
or export a substitute file.

## Failure vocabulary and diagnostics

[`LoweringErrorKind`](../src/error.rs) is the machine-readable failure surface:

| Kind | Meaning at this boundary |
| --- | --- |
| `InvalidKernel` | The complete scalar template or core kernel shape/alias contract is invalid. |
| `InvalidStageContract` | A planner `LoweredProgram`/`ArtifactBuildRecipe` mismatch, impossible stage context, or realized ABI/work disagreement. |
| `InvalidEntrySymbol` | Entry name is not an ASCII LLVM identifier or is duplicated in a bundle. |
| `InvalidTarget` | AMD target token, code-object version, NVIDIA SM/PTX, required linker, required assembler, or AMD feature form is invalid. |
| `InvalidWorkgroupSize` | A scalar or stage lowering option is outside 1 through 1024 lanes. |
| `UnknownScalarValue` | An instruction or output refers to a value not available in the ordered scalar environment. |
| `UnsupportedOperation` | A scalar opcode/type pair is not implemented by this lowerer. |
| `ArithmeticOverflow` | Checked element, work, ABI, index, target, or digest arithmetic overflowed. |
| `ProhibitedInterface` | Generated LLVM declared a non-LLVM external symbol. |
| `ArtifactFormat` | ELF, section, symbol, metadata, MessagePack, or byte-range encoding is malformed. |
| `ArtifactMismatch` | Target, image, symbol, ABI, tool digest/path, or provenance differs from the requested contract. |
| `ToolchainFailed` | A pinned compiler command exited unsuccessfully; the status and bounded stderr are preserved. |
| `Io` | Tool path, scratch directory, temporary file, or output file violated the filesystem contract. |

`LoweringError` carries a message and, when the failure is attached to one
scalar instruction, its `ScalarValueId`. `Display` prints the kind, scalar
identity when present, and the direct message. Callers convert this error into
their own domain error without hiding the original classification.

There are no retries, alternate toolchains, compatibility shims, internal
mock artifacts, driver bypasses, or status-only success paths. A failed
verification, compiler invocation, contract digest, or binary inspection stops
the transition that observed it.

## Invariants to preserve

The following are architectural invariants, not suggestions for future code:

* LLVM emitted by this crate is direct target kernel IR. It contains no host
  runtime call and no vendor operation-library call.
* Only `recipe-primitives` and the planner choose primitive stage semantics,
  geometry, trees, algorithms, and resource bounds. Kernel lowering consumes
  and rechecks those choices.
* Target, ABI, entry symbol, source program digest, stage ordinal, contract
  digest, bindings, work, fault channel, and resources are one authenticated
  chain. A mismatch cannot be repaired by selecting a similar stage.
* Dynamic `RunId` and `LoopIteration` are explicit launch arguments when the
  stage contract requires them. They are never embedded in native bytes.
* Native artifact construction happens only in Offline or Realize. The
  finalized runtime receives bytes and immutable ABI, not a compiler object.
* Every emitted module is audited before compilation, every tool is reverified
  before invocation, and every output is inspected after compilation.
* Every bundle is nonempty, target-homogeneous, entry-symbol-unique, and
  inspected in the same deterministic order as its lowered inputs.
* HSA explicit arguments are eight-byte slots at ordered offsets; CUDA entry
  symbols and executable text are present and nonempty. Hidden HSA metadata
  arguments may trail the explicit ABI, but visible extras are rejected.
* Scratch workspaces are private, temporary, deterministic in recorded paths,
  and removed after the build. Public model artifacts are not written here.
* Any arithmetic or byte-range overflow is reported. LLVM integer instructions
  are never used to disguise a host-side overflow in contract construction.

## Examples and structural checks

The examples are intentionally small direct probes of the public scalar path:

* [`examples/lower_add.rs`](../examples/lower_add.rs) accepts `gfxNNNN` or
  `sm_MM` and writes the generated LLVM for one f32 add template.
* [`examples/inspect_add.rs`](../examples/inspect_add.rs) rebuilds the same
  ABI description, reads an existing HSACO or cubin, and runs the matching
  structural inspector.

For structural validation of this documentation and the crate boundary, use
the real workspace commands:

```bash
cargo check -p recipe-kernel --all-targets
cargo run -p recipe-kernel --example lower_add -- gfx1101 target/recipe-add.ll
cargo run -p recipe-kernel --example lower_add -- sm_86 target/recipe-add.ptx.ll
git diff --check -- kernel/.docs/README.md
```

The examples prove parsing, scalar lowering, and inspection mechanics only.
Runtime correctness, measured performance, candidate stabilization, and the
complete native lifecycle require the production `recipe prepare` path and the
hardware acceptance workflow. A successful Rust build or an empty string audit
does not prove that a device loaded or executed the image.

## Deliberate exclusions

This crate intentionally does not:

* expose a public graph builder or infer primitive stages;
* choose a device, route, workgroup size, algorithm, or schedule;
* read a model, data set, topology profile, or user artifact declaration;
* own CUDA Driver API, ROCr/HSA, HIP, allocator, queue, signal, or completion
  state;
* write `.ogdl`, `.cubin`, `.hsaco`, journals, plans, caches, or checkpoints as
  user-facing files;
* turn a native image into semantic model state;
* catch a failed stage and continue through another implementation; or
* claim runtime behavior from a compile status, emitted text match, or internal
  provenance record alone.

The smallest coherent interpretation is therefore: `recipe-kernel` is the
closed, target-aware realization and inspection boundary between immutable
Recipe stage contracts and the native executor. Everything before it must
provide a complete contract; everything after it must consume the exact bytes
and ABI it proves.
