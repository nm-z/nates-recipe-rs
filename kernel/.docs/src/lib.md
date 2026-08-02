# `recipe-kernel` facade

Source: [`kernel/src/lib.rs`](../../src/lib.rs)

```toml
[module]
path = "kernel/src/lib.rs"
kind = "crate-facade-and-native-kernel-realizer"
intent = "Lower Recipe-owned scalar and primitive stages to target LLVM IR, build inspected native images, and expose the exact launch ABI."
purpose = "Provide one explicit root surface for target validation, LLVM lowering, deferred-stage contract verification, native artifact construction, artifact inspection, and lowering diagnostics."
structure = "Seven private implementation modules behind explicit root re-exports, with two public wrappers around the stage module's contract verifier and realization entry point."
private_modules = ["artifact", "audit", "builder", "error", "llvm", "stage", "target"]
crate_attributes = ["forbid(unsafe_code)", "deny(missing_debug_implementations)"]
state = "lowering and inspection are per-call; ArtifactBuilder retains only pinned tool identities; workspace nonce allocation is process-local"
filesystem_ownership = "only ArtifactBuilder and PinnedTool inspect or create files; lowering and inspection of supplied bytes do not access the filesystem"
driver_ownership = "none; native-probe, prepare, and native-executor own discovery, loading, allocation, submission, and completion"

[boundary]
inputs = ["recipe_core::KernelTemplate", "recipe_primitives::LoweredProgram", "recipe_core::ArtifactBuildRecipe", "KernelTarget", "LoweringOptions", "pinned offline tools", "native ELF bytes"]
outputs = ["LoweredKernel", "BuiltArtifact", "BuiltHsacoBundle", "BuiltCubinBundle", "InspectedHsaco", "InspectedCubin", "ArtifactDigest", "LoweringError"]
semantic_owner = "recipe-core owns typed scalar and artifact contracts; recipe-primitives owns stage declarations and resource bounds; recipe-kernel owns target-specific LLVM emission and artifact proof"
runtime_owner = "recipe-prepare realizes candidates, recipe-native-executor loads and launches images, and recipe-cuda or recipe-hsa own driver operations"
rejection = "invalid source contracts, unsupported operations, prohibited interfaces, target mismatches, malformed ELF, unresolved symbols, tool failures, and I/O failures return LoweringError"

[flow]
steps = ["validate target and lowering options", "validate a KernelTemplate for lower_elementwise or validate the complete LoweredProgram and ArtifactBuildRecipe for lower_stage", "emit direct target LLVM IR", "audit declarations so only LLVM intrinsics remain", "construct KernelAbi and LoweredKernel", "optionally invoke pinned offline tools in a private scratch workspace", "inspect the resulting ELF and compare its target, symbols, metadata, and ABI", "hand the immutable image and ABI to preparation or native execution"]
scalar_path = "lower_elementwise validates one backend-neutral KernelTemplate and emits one lane-per-element entry point"
stage_path = "lower_stage independently verifies the planner-owned stage recipe, dispatch geometry, resources, bindings, fault contract, and canonical digests before emitting one stage"
artifact_path = "ArtifactBuilder verifies every pinned tool before each invocation, builds HSACO or cubin, inspects the complete result, records provenance, and removes its scratch directory on drop"
run_boundary = "compilation, allocation, image loading, and inspection finish before the immutable init -> loop -> exit run boundary"

[abi]
pointer_order = "readable buffers in source binding order, then writable buffers in source binding order"
control_order = "optional fault flag, optional RunId for Philox stages, optional LoopIteration for iteration-dependent stages, then ElementCount"
slot_size = "8 bytes per explicit argument"
alignment = "8-byte argument alignment; f32 and int32 payload pointers are aligned to 4 bytes"
dynamic_values = ["RunId is a 64-bit by-value launch argument and is never embedded in an AOT image", "LoopIteration is a 64-bit by-value launch argument only when the stage depends on iteration", "ElementCount is always a 64-bit by-value launch argument"]

[exports]
structs = ["AmdTarget", "ArtifactBuilder", "ArtifactDigest", "AuditFinding", "BuildProvenance", "BuiltCubinBundle", "BuiltHsacoBundle", "CubinBundleProvenance", "HsaKernelArgument", "HsaKernelMetadata", "HsacoBundleProvenance", "InspectedCubin", "InspectedHsaco", "KernelAbi", "LoweredKernel", "LoweringError", "LoweringOptions", "NvidiaTarget", "OfflineToolchain", "PinnedTool", "ToolInvocation"]
enums = ["AuditKind", "BufferAccess", "BuildPhase", "BuiltArtifact", "KernelArgument", "KernelTarget", "LoweringErrorKind"]
functions = ["artifact_build_contract_digest", "audit_llvm_ir", "inspect_cubin", "inspect_hsaco", "inspect_hsaco_bundle", "lower_elementwise", "lower_stage"]
root_only = ["artifact_build_contract_digest", "lower_stage"]
module_visibility = "there are no public modules; callers use the root names above"

[consumers]
facade = "src/facade.rs re-exports the crate as recipe::engine::kernel for advanced callers"
cli = "src/cli.rs captures and reopens PinnedTool values and derives ArtifactDigest values for the active native receipt"
native_probe = "native-probe lowers and builds one Recipe-owned FMA kernel per measured CUDA or HSA device, then loads it for benchmark evidence"
prepare = "prepare/src/production.rs lowers planner-owned deferred stages, groups them into target bundles, invokes BuildPhase::Realize, and validates every returned entry"
native_executor = "native-executor validates RuntimeArtifact ABI and bytes with inspect_cubin or inspect_hsaco_bundle, then binds KernelArgument values to driver launches"
examples = "kernel/examples/lower_add.rs demonstrates LLVM emission and kernel/examples/inspect_add.rs demonstrates native inspection"
```

## Intent and crate boundary

`recipe-kernel` is the target-specific realization boundary in Recipe's static
pipeline. It consumes declarations and measured target choices that already
belong to other crates. It does not discover a GPU, choose a placement or
schedule, allocate a device buffer, load a driver image, submit work, or
observe a run. Its job is to turn an owned scalar template or primitive stage
into direct LLVM IR and, when requested by preparation or probing, turn that IR
into one inspected HSACO or cubin.

The source-level attributes at the top of `kernel/src/lib.rs` are
`#![forbid(unsafe_code)]` and `#![deny(missing_debug_implementations)]`. The
crate depends on `recipe-core`, `recipe-language`, `recipe-primitives`, and
`sha2`; it has no CUDA Runtime API, HIP, vendor math, driver, or unsafe FFI
dependency. LLVM intrinsics are the only declarations permitted in generated
IR. The native driver crates consume the resulting bytes after this crate has
finished.

The public surface is intentionally flat. Every implementation module is
private, each public value has one owning module, and `lib.rs` only re-exports
those values or forwards to a private implementation. There is no second
lowering path hidden behind a public module.

## What `lib.rs` exposes

The facade declares these private modules, in source order:

```text
artifact, audit, builder, error, llvm, stage, target
```

It then re-exports the following names explicitly:

```rust
pub use artifact::{
    ArtifactDigest, HsaKernelArgument, HsaKernelMetadata, InspectedCubin, InspectedHsaco,
    inspect_cubin, inspect_hsaco, inspect_hsaco_bundle,
};
pub use audit::{AuditFinding, AuditKind, audit_llvm_ir};
pub use builder::{
    ArtifactBuilder, BuildPhase, BuildProvenance, BuiltArtifact, BuiltCubinBundle,
    BuiltHsacoBundle, CubinBundleProvenance, HsacoBundleProvenance, OfflineToolchain,
    PinnedTool, ToolInvocation,
};
pub use error::{LoweringError, LoweringErrorKind};
pub use llvm::{
    BufferAccess, KernelAbi, KernelArgument, LoweredKernel, LoweringOptions, lower_elementwise,
};
pub use target::{AmdTarget, KernelTarget, NvidiaTarget};
```

The two functions below are the only public paths into `stage.rs`:

```rust
pub fn artifact_build_contract_digest(
    build: &recipe_core::ArtifactBuildRecipe,
) -> recipe_core::Digest;

pub fn lower_stage(
    program: &recipe_primitives::LoweredProgram,
    build: &recipe_core::ArtifactBuildRecipe,
    target: &KernelTarget,
    options: &LoweringOptions,
) -> Result<LoweredKernel, LoweringError>;
```

`artifact_build_contract_digest` forwards to the canonical
`recipe-planner-artifact-build-v1` digest implementation. `lower_stage`
forwards to the stage verifier and emitter. Neither wrapper adds validation or
an alternate implementation, so the source module remains the owner of the
stage contract.

## Exact public API inventory

The following inventory is the current rustdoc surface. Struct fields listed
below are public unless stated otherwise; private implementation state is not
part of the API.

### Targets and lowering records

| Item | Public shape and role |
| --- | --- |
| `AmdTarget` | `{ target_id: String, code_object_version: u8 }`; validates a `gfx...` target and nonzero AMDGPU code-object version. |
| `NvidiaTarget` | `{ sm_major: u8, sm_minor: u8, ptx_isa: u16 }`; validates the SM and PTX ranges, formats `sm_Mm`, and formats the LLVM `+ptxN` feature. |
| `KernelTarget` | `Amd(AmdTarget)` or `Nvidia(NvidiaTarget)`; delegates target validation to the selected backend. |
| `LoweringOptions` | `{ entry_symbol: String, workgroup_lanes: u32 }`; names the emitted entry and fixes its workgroup size. |
| `BufferAccess` | `Read` or `Write`; describes one explicit buffer slot in `KernelArgument`. |
| `KernelArgument` | `Buffer { access, dtype, alignment }`, `FaultFlag`, `RunId`, `LoopIteration`, or `ElementCount`; describes the exact native launch ABI in order. |
| `KernelAbi` | `{ entry_symbol, arguments, argument_bytes, argument_alignment, elements, workgroup_lanes }`; the launcher contract recovered from lowering and checked against native metadata. |
| `LoweredKernel` | `{ llvm_ir, abi, work, target }`; owns target-specific LLVM text, its ABI, the calculated `FlopCount`, and the exact target used to emit it. |

`AmdTarget::validate`, `NvidiaTarget::validate`,
`NvidiaTarget::architecture`, `NvidiaTarget::llvm_ptx_feature`, and
`KernelTarget::validate` are the target methods. AMD target IDs permit only
ASCII alphanumeric characters and `_`, `-`, `+`, `:`, or `.`, and must begin
with `gfx`. NVIDIA validation accepts SM major values 3 through 12, SM minor
values through 9, and PTX ISA values from 32 through 90. The target enum is
cloned into `LoweredKernel`, so a builder cannot silently compile a module for
another target.

### Artifact identities, metadata, and inspection

| Item | Public shape and role |
| --- | --- |
| `ArtifactDigest` | Opaque 32-byte SHA-256 digest. `of(&[u8])` hashes bytes, `bytes()` returns the fixed array, and `to_hex()` returns lower-case hexadecimal. |
| `HsaKernelArgument` | `{ name: Option<String>, offset: u64, size: u64, value_kind: String, address_space: Option<String> }`; `is_hidden()` identifies AMDHSA hidden arguments. |
| `HsaKernelMetadata` | `{ name, symbol, arguments, kernarg_segment_size, kernarg_segment_alignment, group_segment_fixed_size, private_segment_fixed_size, maximum_workgroup_size, wavefront_size }`; decoded AMDGPU metadata for one entry. |
| `InspectedHsaco` | `{ digest, elf_abi_version, code_object_version, elf_flags, target_id, kernel }`; proof facts for one requested HSACO entry. |
| `InspectedCubin` | `{ digest, elf_abi_version, elf_flags, sm, entry_symbol, text_bytes }`; proof facts for one requested cubin entry. |

`inspect_hsaco` accepts bytes, an expected target ID, an expected code-object
version, and one `KernelAbi`. `inspect_hsaco_bundle` accepts the same target
identity plus an ordered iterator of ABIs, decodes the ELF symbol table and
AMDGPU MessagePack note once, and returns inspections in the requested order.
Both paths require an AMDGPU-HSA ELF, matching target components and code
object version, defined function and `.kd` descriptor symbols, no unresolved
global or weak symbols, and metadata that satisfies every explicit argument's
offset, size, kind, address space, entry identity, kernarg alignment, and
workgroup limit. Hidden trailing arguments are allowed; unexpected non-hidden
arguments are rejected.

`inspect_cubin` accepts bytes, an expected decimal SM value, and an expected
entry symbol. It requires one of the supported CUDA ELF OS-ABI layouts, the
CUDA ELF machine, the matching SM encoding, no unresolved global or weak
symbols, a defined global entry function, and a nonempty executable
`.text.<entry>` section. All inspection failures are represented as
`LoweringErrorKind::ArtifactFormat` or `ArtifactMismatch`, never as a partial
inspection.

### Toolchain and build records

| Item | Public shape and role |
| --- | --- |
| `BuildPhase` | `Offline` or `Realize`; the caller-selected lifecycle label recorded in provenance. Current production callers use `Realize`. |
| `PinnedTool` | `{ path: PathBuf, digest: ArtifactDigest }`; `inspect(path)` canonicalizes an absolute regular file and captures its bytes digest. Every builder invocation rechecks both path identity and digest. |
| `OfflineToolchain` | `{ verifier, llvm_codegen, elf_linker: Option<PinnedTool>, ptx_assembler: Option<PinnedTool> }`; the pinned `opt`, `llc`, linker, and optional `ptxas` set. |
| `ToolInvocation` | `{ program: PathBuf, arguments: Vec<OsString> }`; one normalized invocation retained in build provenance. |
| `BuildProvenance` | `{ phase, llvm_ir, tools, invocations }` for one image. |
| `HsacoBundleProvenance` | `{ phase, llvm_ir: Vec<ArtifactDigest>, tools, invocations }` in the same order as bundled LLVM modules and inspections. |
| `CubinBundleProvenance` | `{ phase, llvm_ir: Vec<ArtifactDigest>, tools, invocations }` in the same order as bundled LLVM modules and inspections. |
| `BuiltArtifact` | `Hsaco { bytes, inspection, provenance }` or `Cubin { ptx, bytes, inspection, provenance }`; the single-entry build result. |
| `BuiltHsacoBundle` | `{ bytes, inspections, provenance }`; one multi-entry AMD code object. |
| `BuiltCubinBundle` | `{ bytes, inspections, provenance }`; one multi-entry NVIDIA cubin. |
| `ArtifactBuilder` | Owns a private `OfflineToolchain`; `new`, `build`, `build_hsaco_bundle`, and `build_cubin_bundle` are the only public methods. |

`ArtifactBuilder::new` verifies every configured tool before retaining the
toolchain. `build` first verifies the requested target and exact
`LoweredKernel::target`, writes LLVM IR into a newly created private workspace,
runs the verifier, invokes the target compiler path, reads a regular output,
inspects it against the lowered ABI, and returns bytes plus provenance. The
single-entry AMD path uses `llc` to produce an object and a pinned ELF linker
to produce HSACO. The single-entry NVIDIA path uses `llc` for PTX and pinned
`ptxas` for cubin.

The bundle methods reject empty input and repeated entry symbols, require that
every lowered module has the requested target, verify each module separately,
then package all entries in one native image. AMD uses full-LTO linking with
the requested processor, features, and code-object version. NVIDIA passes all
PTX units to one `ptxas` invocation for the requested `sm_Mm`. Every entry is
inspected before the result is returned, and the input order is retained in
the inspection and provenance vectors.

Tool invocations clear the inherited environment and set deterministic
`LC_ALL=C` and `SOURCE_DATE_EPOCH=0` values. Tool stderr is bounded for an
error message. Scratch parents must be real private directories, Recipe-created
source files use create-new mode `0600`, workspaces are mode `0700`, and the
workspace removes itself when the builder operation ends. This is build-time
filesystem and process ownership, not run-time driver ownership.

### Audit and errors

`audit_llvm_ir(&str)` is a deterministic lexical gate. It examines lines whose
trimmed text begins with `declare`, extracts the symbol after `@`, and returns
an `AuditFinding` for every declaration that does not begin with `llvm.`. The
public `AuditKind` is currently non-exhaustive with the single
`ExternalIrDeclaration` variant. `AuditFinding` contains `kind`, one-based
`line`, and the offending `token`. The separate repository audit gate owns
linker, dependency, dynamic-load, and final-ELF policy; this crate only checks
generated LLVM declarations.

`LoweringErrorKind` is non-exhaustive and currently contains:

```text
InvalidKernel, InvalidStageContract, InvalidEntrySymbol, InvalidTarget,
InvalidWorkgroupSize, UnknownScalarValue, UnsupportedOperation,
ArithmeticOverflow, ProhibitedInterface, ArtifactFormat, ArtifactMismatch,
ToolchainFailed, Io
```

`LoweringError` owns `{ kind, scalar: Option<ScalarValueId>, message }`.
`LoweringError::new` creates a diagnostic and `for_scalar` attaches the
offending scalar value when an operation or operand is the source of failure.
Its `Display` output includes the kind, optional scalar, and message, and it
implements `std::error::Error`. No public API turns an error into a substitute
kernel or a fallback artifact.

## Module ownership map

| Private module | Root exports | Owned responsibility and boundary |
| --- | --- | --- |
| `artifact` | `ArtifactDigest`, HSA metadata and inspection records, `inspect_hsaco`, `inspect_hsaco_bundle`, `inspect_cubin` | Parse bounded little-endian ELF64 and AMDGPU MessagePack metadata from supplied bytes, check symbols, targets, entry ABI, and executable sections, and return immutable proof facts. It does not load an image or invoke a tool. |
| `audit` | `AuditFinding`, `AuditKind`, `audit_llvm_ir` | Enforce the closed generated-IR declaration rule, allowing LLVM intrinsics only. It does not scan source repositories or final binaries. |
| `builder` | Tool pins, phase/provenance records, built image records, `ArtifactBuilder` | Verify pinned offline tools, invoke LLVM and target assemblers in private scratch, inspect outputs, and retain deterministic provenance. It owns build filesystem and process effects. |
| `error` | `LoweringError`, `LoweringErrorKind` | One diagnostic type shared by lowering, contract verification, artifact parsing, and tool execution. |
| `llvm` | `BufferAccess`, `KernelArgument`, `KernelAbi`, `LoweredKernel`, `LoweringOptions`, `lower_elementwise` | Emit direct elementwise AMDGPU or NVPTX LLVM from a validated `KernelTemplate`, derive the scalar ABI and FLOP work, and audit the resulting module. |
| `stage` | no direct module exports; reached through the two root wrappers | Recompute the canonical artifact-build digest, validate the complete planner stage contract, dispatch every owned `StageKind`, build stage ABI and IR, rewrite checked scalar fault publication to the planned code, and validate the realized kernel. |
| `target` | `AmdTarget`, `KernelTarget`, `NvidiaTarget` | Validate backend identity and format target strings; it does not inspect hardware or infer a target from bytes. |

The module boundaries are ownership boundaries, not alternate APIs. For
example, `stage` calls the public-in-crate `llvm::lower_elementwise` function
for scalar-map stages, then applies the stage fault contract and final stage
checks. It does not duplicate scalar emission.

## Lowering paths and generated ABI

### `lower_elementwise`

`lower_elementwise` accepts a validated `recipe_core::KernelTemplate`, a
`KernelTarget`, and `LoweringOptions`. It validates the template, target,
ASCII entry symbol, and workgroup size of 1 through 1024 lanes. The emitter
computes a target-specific global lane index from AMDGPU or NVPTX intrinsics,
guards lanes beyond `element_count`, computes each input and output address
from its `StaticBufferAccess` affine view, and loads or stores only f32 or
int32 payloads.

The scalar instruction emitter owns the direct mapping of the validated
`ScalarOpcode` domain to LLVM operations. Floating arithmetic uses constrained
round-to-nearest-even operations and canonicalizes NaNs. Checked int32 divide,
remainder, negate, absolute, and `Require` operations produce safe values and
publish rejection through a preallocated fault flag. Comparisons produce an
int32 condition, bitcasts preserve bits, shifts mask their count, and
unsupported opcode or dtype combinations return `UnsupportedOperation` with a
scalar ID when available. Generated IR declares target intrinsics and scalar
LLVM intrinsics only, then passes `audit_llvm_ir` before returning.

The elementwise ABI is ordered as all template input pointers, all template
output pointers, an optional fault-flag pointer when a checked scalar operation
was emitted, and `i64 element_count`. Each explicit slot is eight bytes and
the ABI alignment is eight. `elements` is the template index-space element
count, `workgroup_lanes` comes from options, and `work` is the checked product
of per-instruction FLOPs and element count. The returned target clone binds
the IR to the selected AMD or NVIDIA backend.

### `lower_stage`

`lower_stage` receives the complete `LoweredProgram` and complete
`ArtifactBuildRecipe`, not fragments selected by the caller. Before emitting
anything it requires canonical validation of both records, matching program
digest and source-kernel identity, an existing stage ordinal, the canonical
stage-scoped kernel-template identity, the reserved artifact identity, and the
recomputed `recipe-planner-artifact-build-v1` contract digest. It then checks
dispatch geometry, lowering workgroup size, FLOP/integer/atomic work bounds,
resource bounds, binding count and exact dtype/access/view fields, and the
optional fault binding. Any contradiction is `InvalidStageContract`.

Scalar-map stages reuse `lower_elementwise`. If the stage has a fault contract,
the stage path replaces the generic scalar emitter's atomic OR with one
release `atomicrmw xchg` carrying the planned fault code. All other stages use
the owned stage emitters in one match over these `StageKind` variants:

```text
Fill, Copy, FixedTreeReduce, FixedTreeScanLocal, ScanUniformCombine,
TiledContraction, Gather, Scatter, HistogramClear, HistogramAccumulate,
StableSortInitialize, StableSortCompareExchange, StableSortFinalize,
IndexMap, Philox4x32_10, ScalarMap
```

Those emitters implement Recipe-owned affine fill/copy, fixed-tree reduction
and scan, tiled contraction, checked gather and scatter policies, histogram
clear and accumulation, stable sort network stages, checked index mapping, and
Philox4x32-10 random stages. The stage path uses barriers and target address
spaces directly in LLVM, never a vendor math or runtime library. Philox stages
carry `RunId`; stages with a nonzero index-map iteration step carry
`LoopIteration`.

The stage ABI is generated from the immutable build bindings. Readable
bindings become input pointers in binding order, writable bindings become
output pointers in binding order, the optional fault flag follows, dynamic
`RunId` and `LoopIteration` values follow when required, and `element_count`
is last. Read-write bindings therefore have distinct input and output slots.
The stage checks the resulting ABI element count, workgroup size, work bound,
and fault-argument count against the stage contract before returning
`LoweredKernel`.

`artifact_build_contract_digest` is the independent digest used in that
verification. Its domain includes the artifact, kernel-template, and source
kernel IDs; program digest and stage ordinal; every binding value, dtype,
access, extent, offset, stride, and storage size; dispatch geometry; work
bounds; optional fault value; and the private/shared/scratch/workgroup resource
envelope. Recomputing it is a proof operation, not an invitation to mutate a
recipe.

## Native artifact proof boundary

The builder and inspector form a fail-closed sequence:

```text
LoweredKernel
  -> verify target and pinned tools
  -> write one or more LLVM modules in a private scratch workspace
  -> run LLVM verifier and target-specific offline compiler
  -> read the regular HSACO or cubin output
  -> parse ELF headers, sections, symbols, and target metadata
  -> match every requested entry ABI
  -> return bytes, inspections, and provenance
```

For AMD, `inspect_hsaco_bundle` matches the AMDGPU HSA OS ABI and machine,
derives the code-object version, compares the complete target ID including
feature modifiers, rejects unresolved symbols, requires each function and
`.kd` descriptor, and decodes the AMDGPU metadata note. It validates explicit
argument offsets and sizes, global-buffer versus by-value kinds, optional
argument names, kernarg alignment and size, and maximum workgroup size.

For NVIDIA, `inspect_cubin` accepts the observed CUDA ELF OS-ABI layouts,
decodes the SM from the layout-specific flag location, rejects unresolved
symbols, and requires the expected executable entry section and nonzero code
size. The digest on every inspection is the SHA-256 of the complete native
image, not a digest of a filename or a generic checkpoint.

The native image is still only an artifact at this boundary. The builder never
loads it and never creates a driver function. Preparation wraps bytes and ABI
facts into target-specific runtime identities. Native execution repeats the
inspection against those immutable identities, groups entries that share one
multi-entry image, loads each image once per backend, and resolves individual
functions only after the image proof succeeds.

## End-to-end ownership and lifecycle

The complete path from declaration to execution is:

1. `recipe-language`, `recipe-ops`, and `recipe-primitives` construct a typed
   `KernelTemplate` or a validated `LoweredProgram` containing stage kinds,
   bindings, geometry, work bounds, resources, and the program digest.
2. `recipe-planner` creates an offset-free `ArtifactBuildRecipe` for deferred
   stages. The recipe carries the exact binding views, target-independent
   dispatch and resource contract, stage provenance, and canonical contract
   digest. The planner owns placement and scheduling; the kernel crate does not
   choose them.
3. Measured discovery supplies a `KernelTarget`, toolchain pins, and private
   scratch parent. `prepare` groups deferred recipes by measured target and
   calls `lower_stage` for every current stage with an entry symbol derived from
   its reserved artifact ID.
4. `lower_stage` validates each complete pair and emits target LLVM. `ArtifactBuilder`
   packages the lowered entries in `BuildPhase::Realize`, inspects the complete
   HSACO or cubin, and returns immutable bytes and provenance. If preparation
   was given a prebuilt bundle, it still lowers every current stage and calls
   the inspectors, but it does not invoke the compiler.
5. `recipe-prepare` joins the inspected image, ABI, target identity, toolchain
   identity, and measured runtime policy into a candidate realization. The
   compiler and builder are not retained in the prepared run resources.
6. `recipe-native-executor` validates runtime artifacts against the finalized
   bundle and backend binding. CUDA uses `inspect_cubin`; HSA groups logical
   entries by image and uses `inspect_hsaco_bundle`. The executor then loads the
   proved image, resolves entry functions, packs `KernelArgument` values, and
   submits only the already-finalized calculation tasks.
7. The immutable `init -> loop -> exit` run executes with dynamic `RunId`,
   `LoopIteration`, and `ElementCount` values supplied by the executor. No
   compilation, discovery, allocation, image inspection, or artifact mutation
   occurs in the finalized loop.

This separation keeps semantic ownership with the upstream graph and planner,
target lowering and artifact proof here, candidate stabilization in prepare,
and driver lifetime and submission in native-executor. A failure remains at
the boundary that observed it. There is no fallback kernel, alternate compiler,
generic checkpoint, or duplicate ABI implementation.

## Workspace consumers

| Consumer | Evidence in source | Kernel surface used |
| --- | --- | --- |
| Root advanced facade | [`src/facade.rs`](../../../src/facade.rs) | Re-exports the complete crate as `recipe::engine::kernel`; it does not wrap or duplicate any implementation. |
| CLI native receipt | [`src/cli.rs`](../../../src/cli.rs) | `PinnedTool::inspect`, `OfflineToolchain`, and `ArtifactDigest` capture exact tool paths and digests for the active native preparation receipt. |
| Native preparation configuration | [`src/native_prepare.rs`](../../../src/native_prepare.rs) | `AmdTarget`, `NvidiaTarget`, `KernelTarget`, `ArtifactBuilder`, and `LoweringError` assemble measured target build specifications. |
| Production candidate realization | [`prepare/src/production.rs`](../../../prepare/src/production.rs) | `lower_stage`, `LoweringOptions`, `ArtifactBuilder::build_*_bundle`, `BuildPhase::Realize`, and both bundle inspectors materialize deferred stages and validate each entry. |
| CUDA probe | [`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs) | Builds a direct FMA benchmark with `lower_elementwise`, `ArtifactBuilder::build`, and `inspect_cubin` before driver loading and timed verification. |
| HSA probe | [`native-probe/src/hsa.rs`](../../../native-probe/src/hsa.rs) | Builds the corresponding FMA benchmark with `lower_elementwise`, `ArtifactBuilder::build`, and `inspect_hsaco` before ROCr loading and timed verification. |
| Probe identity/config | [`native-probe/src/identity.rs`](../../../native-probe/src/identity.rs), [`native-probe/src/config.rs`](../../../native-probe/src/config.rs) | Carries `ArtifactDigest`, `OfflineToolchain`, `PinnedTool`, and explicit kernel build settings into measured profile identity. |
| CUDA executor | [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs) | Uses `KernelAbi`, `KernelArgument`, `ArtifactDigest`, and `inspect_cubin` to validate images and bind driver launch parameters. |
| HSA executor | [`native-executor/src/hsa.rs`](../../../native-executor/src/hsa.rs) | Uses `KernelAbi`, `KernelArgument`, `ArtifactDigest`, and `inspect_hsaco_bundle` to validate shared images, resolve symbols, and bind kernarg metadata. |
| Executor plan/error boundary | [`native-executor/src/plan.rs`](../../../native-executor/src/plan.rs), [`native-executor/src/error.rs`](../../../native-executor/src/error.rs) | Reuses `BufferAccess`, `KernelAbi`, `KernelArgument`, `ArtifactDigest`, and `LoweringError` while validating finalized runtime plans. |
| Public examples | [`kernel/examples/lower_add.rs`](../../examples/lower_add.rs), [`kernel/examples/inspect_add.rs`](../../examples/inspect_add.rs) | Show direct elementwise lowering and target-specific HSACO or cubin inspection. |

The examples are demonstration entry points only. Production preparation and
native execution use the same root APIs and the same artifact proof boundary.
