# kernel/examples/lower_add.rs

## Intent

This example is a direct, deterministic smoke path for the public
recipe_kernel::lower_elementwise API. It constructs one complete
recipe_core::KernelTemplate for elementwise f32 addition, selects one target
family, lowers the template to target-specific textual LLVM IR, and writes that
IR to the caller-selected path.

The executable deliberately stops at LoweredKernel. It does not invoke an
offline compiler, produce an HSACO or cubin, inspect an ELF artifact, allocate
device memory, launch a kernel, or run a calculation. Those are separate public
boundaries in recipe_kernel and are described below so the stopping point is
explicit.

The source contract is represented by these parseable fields:

~~~text
entrypoint: main() -> Result<(), Box<dyn std::error::Error>>
arguments: <gfx...|sm_..> <output.ll>
accepted_target_families: KernelTarget::Amd, KernelTarget::Nvidia
logical_shape: [1_048_576]
payload: f32 + f32 -> f32
entry_symbol: recipe_add_f32
workgroup_lanes: 256
output_kind: textual target LLVM IR
native_artifacts: none
runtime_execution: none
~~~

The usage string names gfx1101 and sm_86, but the parser and target validators
are the actual authority. The AMD branch accepts any nonempty gfx... token that
passes AmdTarget::validate. The NVIDIA branch parses two ASCII digits after
sm_ and then applies NvidiaTarget::validate. The output path has no extension
check. fs::write receives the path unchanged and is called only after lowering
succeeds.

## Structure

The source has one import block and one main function. There are no helper
functions, tests, hidden defaults, or alternate lowering paths.

| Lines | Source operation | Resulting state |
| --- | --- | --- |
| 1 | Import env, fs, and PathBuf | OS argument access and final file write |
| 3-6 | Import scalar and kernel-template records from recipe_core | Typed IDs, dtypes, index space, accesses, program, and alias rules |
| 8 | Import target, option, and lowering types from recipe_kernel | Public lowering boundary used by the example |
| 10-22 | Parse exactly two positional arguments | Target token and output path, or a usage error |
| 23-43 | Convert the target token to KernelTarget | AMD target with code-object version 6, or NVIDIA target with PTX ISA 75 |
| 45-47 | Allocate scalar value IDs | left = 1, right = 2, result = 3 |
| 48 | Build a one-dimensional IndexSpace | 1,048,576 logical elements |
| 49 | Build one contiguous F32 access mapping | Offset 0, stride 1, 4,194,304 bytes of backing storage |
| 50-102 | Assemble KernelTemplate | Two F32 inputs, one F32 output, one F32 Add, and a complete alias matrix |
| 103-106 | Call lower_elementwise | A checked LoweredKernel containing LLVM IR, ABI, work, and target |
| 107 | Write lowered.llvm_ir | The caller-selected output file, or an I/O error |
| 108 | Return success | Process status 0 |

The call graph for a successful invocation is:

~~~text
OS arguments
  -> target parser and target value
  -> IndexSpace::new
  -> StaticBufferAccess::contiguous
  -> KernelTemplate value
  -> lower_elementwise
       -> KernelTemplate::validate
       -> KernelTarget::validate
       -> LoweringOptions validation
       -> target-specific index and buffer emission
       -> ScalarOpcode::Add lowering
       -> LLVM module assembly
       -> audit_llvm_ir
       -> LoweredKernel ABI and work accounting
  -> fs::write(output, lowered.llvm_ir)
~~~

main uses ? at every fallible construction and at the lowering and file write
boundaries. A failed step prevents all later steps. In particular, a failed
target parse or lowering never reaches fs::write.

## Purpose and invocation

The example is registered through Cargo's automatic examples/ discovery in the
recipe-kernel package. From the workspace root, invoke it as:

~~~text
cargo run -p recipe-kernel --example lower_add -- gfx1101 /tmp/lower_add.ll
cargo run -p recipe-kernel --example lower_add -- sm_86 /tmp/lower_add.ll
~~~

The first command selects KernelTarget::Amd(AmdTarget { target_id: "gfx1101",
code_object_version: 6 }). The second selects
KernelTarget::Nvidia(NvidiaTarget { sm_major: 8, sm_minor: 6, ptx_isa: 75 }).
Cargo's build and run lines are wrapper output. The example itself prints
nothing on success. The output file is ordinary text LLVM IR, not a native
code object.

## Argument contract and target construction

### Argument cardinality

env::args_os().skip(1) is used so the executable name is discarded without
requiring UTF-8. The next argument is the target token and the following one is
converted to PathBuf. A missing target, missing output path, or any third
argument returns the same boxed string:

~~~text
usage: lower_add <gfx1101|sm_86> <output.ll>
~~~

The output path is parsed before the target spelling is checked. It is not
opened or changed during parsing. A non-UTF-8 target is converted with
to_string_lossy; replacement characters then fail target parsing or target
validation instead of being passed to a compiler.

### AMD spelling

If the token has the gfx prefix, the suffix is inspected first. An empty suffix
returns:

~~~text
AMD target must include its numeric target ID
~~~

Otherwise the code creates:

~~~text
KernelTarget::Amd(AmdTarget {
    target_id: format!("gfx{architecture}"),
    code_object_version: 6,
})
~~~

The example does not parse the suffix as an integer. AmdTarget::validate later
requires a nonempty token containing only ASCII alphanumeric characters or
_, -, +, :, and ., requires the gfx prefix, and rejects a zero code-object
version. Thus gfx1101, gfx0, and gfxabc pass this example's current target
validation, while gfx11!01 reaches lowering and returns
LoweringErrorKind::InvalidTarget for unsupported characters.

The code-object version is carried in LoweredKernel.target for downstream
artifact identity. It does not change the LLVM target triple emitted by this
example.

### NVIDIA spelling

If the token has the sm_ prefix, the suffix must contain exactly two ASCII
digits. Any other length or byte returns:

~~~text
NVIDIA target must have the form sm_86
~~~

The two bytes are converted by subtracting b'0' and used to construct:

~~~text
KernelTarget::Nvidia(NvidiaTarget {
    sm_major: digits[0] - b'0',
    sm_minor: digits[1] - b'0',
    ptx_isa: 75,
})
~~~

NvidiaTarget::validate then accepts major values 3 through 12, minor values 0
through 9, and PTX ISA values 32 through 90. The fixed PTX ISA 75 is valid.
Consequently sm_86, sm_99, and other two-digit targets in those ranges are
accepted, while a two-digit target outside the range returns
LoweringErrorKind::InvalidTarget. The architecture() and llvm_ptx_feature()
helpers are not called by lower_add; the artifact builder uses them later when
translating LLVM IR to PTX and invoking ptxas.

### Unknown prefixes

Tokens that are neither gfx... nor sm_.. return:

~~~text
target must have the form gfx1101 or sm_86
~~~

This is a boxed string from main, not a LoweringError from the kernel crate.

## Kernel template construction

### Logical index space

The example first evaluates:

~~~rust
IndexSpace::new(vec![ElementCount::new(1_048_576)?])?
~~~

ElementCount::new rejects zero and stores a nonzero u64. IndexSpace::new
requires at least one dimension and checks the product for overflow. The
result has one dimension with extent 1,048,576 and a total element count of
1,048,576. It is a linear logical domain, not a device allocation and not a
launch grid.

### Contiguous accesses

StaticBufferAccess::contiguous(&index_space, DType::F32)? computes row-major
strides. With one dimension, the stride vector is [1], the element offset is
zero, and the backing byte bound is:

~~~text
1_048_576 elements * 4 bytes per F32 = 4_194_304 bytes
~~~

The access is cloned for both inputs and moved into the output. Input mappings
are read-only for template validation. The output mapping is writable and is
injective because its stride is one, so every logical element has a distinct
destination. The same access object means all three logical buffers have the
same shape and storage extent; it does not mean the three runtime pointers are
the same allocation.

### Ordered inputs and output

The KernelTemplate uses KernelTemplateId::new(1). Its two ordered inputs are:

| Position | Kernel ID | Scalar ID | Type | Access |
| --- | --- | --- | --- | --- |
| 0 | KernelInputId::new(1) | ScalarValueId::new(1) | DType::F32 | contiguous read |
| 1 | KernelInputId::new(2) | ScalarValueId::new(2) | DType::F32 | contiguous read |

Its one ordered output is:

| Position | Kernel ID | Scalar value | Type | Access |
| --- | --- | --- | --- | --- |
| 0 | KernelOutputId::new(1) | ScalarValueId::new(3) | DType::F32 | contiguous write |

Kernel input IDs, kernel output IDs, and scalar value IDs are distinct typed
namespaces. Reusing the numeric value 1 for an input ID and an output ID is
therefore intentional and valid.

### Scalar program

The embedded ScalarProgram is an ordered, typed SSA program:

~~~text
inputs:
  value 1: F32
  value 2: F32
constants: none
instructions:
  value 3: F32 = Add(value 1, value 2)
outputs:
  value 3
~~~

ScalarOpcode::Add has arity two and preserves one common operand type. The
declared F32 result and F32 operands satisfy ScalarProgram::validate. There
are no constants, no side-effecting operations, no checked integer operations,
and no Require, so requires_fault_flag() is false for this program.

### Alias matrix

The template declares exactly one rule for each input/output pair:

| Input | Output | Permission | Meaning |
| --- | --- | --- | --- |
| input 1 | output 1 | MayAliasExact | Runtime storage may be disjoint or exactly the same object, offset, and range |
| input 2 | output 1 | Forbidden | Runtime storage ranges must not overlap |

KernelTemplate::validate checks duplicate IDs, access mappings, scalar input and
output counts and dtypes, unknown references, duplicate alias rules, and the
complete input/output matrix. Alias permissions are a contract for the planner
and executor. They do not add arguments or runtime pointer checks to the
generated LLVM body. The generated body simply loads both input pointers and
stores the output pointer in the declared order.

## Public lowering boundary

The only lowering call in this example is:

~~~rust
let lowered = lower_elementwise(&template, &target, &LoweringOptions {
    entry_symbol: "recipe_add_f32".to_owned(),
    workgroup_lanes: 256,
})?;
~~~

The public re-export is defined by kernel/src/lib.rs and implemented in
kernel/src/llvm.rs. lower_elementwise performs these checks in order:

1. template.validate() must succeed. All accumulated core validation failures
   are converted to LoweringErrorKind::InvalidKernel with the core validation
   text as the message.
2. target.validate() must succeed. The target value is retained in the returned
   LoweredKernel.
3. The options must contain a nonempty ASCII LLVM identifier. The first byte
   must be an ASCII letter or _, every remaining byte must be ASCII
   alphanumeric or _, and the workgroup width must be in the inclusive range 1
   through 1024.

The hardcoded options pass those checks. recipe_add_f32 is the entry symbol and
256 is the workgroup width used in both index generation and the returned ABI.
The example has no way to supply an invalid option from the command line, but
the public API returns InvalidEntrySymbol or InvalidWorkgroupSize when another
caller does.

## Lowering sequence and generated calculation

### Global index and bounds

The emitter first chooses the target's lane and workgroup intrinsics:

| Target | Lane intrinsic | Workgroup intrinsic | Module triple | Calling convention |
| --- | --- | --- | --- | --- |
| AMD | llvm.amdgcn.workitem.id.x | llvm.amdgcn.workgroup.id.x | amdgcn-amd-amdhsa | amdgpu_kernel |
| NVIDIA | llvm.nvvm.read.ptx.sreg.tid.x | llvm.nvvm.read.ptx.sreg.ctaid.x | nvptx64-nvidia-cuda | ptx_kernel |

Each 32-bit intrinsic result is zero-extended to i64. The body computes:

~~~text
group_base = group_id * 256
global_id  = group_base + local_id
~~~

It compares global_id unsigned against the explicit i64 element_count
argument. An in-range lane branches to body; an out-of-range lane branches
directly to exit. Therefore every lane processes at most one linear element,
and a launch covering the full logical domain may round its number of lanes up
to a multiple of 256 without writing past element 1,048,576.

### Input loads

The one-dimensional contiguous mapping makes each input element index exactly
global_id. For input position 0 and 1, the emitter respectively creates a
getelementptr inbounds float in global address space 1, then a four-byte
aligned F32 load. The loaded values are bound to scalar IDs 1 and 2 in the
emitter's value map. The generated function parameters are named input_0 and
input_1 in this order.

### F32 addition and NaN canonicalization

The one instruction is lowered as a constrained F32 add:

~~~text
call float @llvm.experimental.constrained.fadd.f32(
    float %loaded_1,
    float %loaded_3,
    metadata !"round.tonearest",
    metadata !"fpexcept.ignore"
)
~~~

The emitter then checks the raw result with fcmp uno and selects the bit-pattern
0x7fc00000 (decimal 2143289344) whenever the result is NaN. Finite results pass
through unchanged. This canonicalization is part of the Recipe-owned scalar
semantics, so a NaN from either input or from the add does not preserve an
arbitrary payload in the output.

The constrained add is the only arithmetic intrinsic declaration in this
program. There is no fault condition, so no fault flag argument, atomic fault
publication, or fault branch is emitted.

### Output store and exit

The scalar output value 3 is checked against the F32 output dtype, mapped to
the contiguous output index global_id, and stored with four-byte alignment
through output_0. The body then branches to exit, which returns void. The
module receives strict floating-point attributes:

~~~text
nounwind strictfp "denormal-fp-math-f32"="ieee" "no-trapping-math"="false"
~~~

The LLVM audit rejects any declaration whose symbol is not an llvm. intrinsic.
The module assembled by this example therefore remains closed to host runtimes
and operation libraries. A finding would return
LoweringErrorKind::ProhibitedInterface with the offending line and token.

## Returned LoweredKernel

For this exact template, lower_elementwise returns these values after the
module audit:

| Field | Value |
| --- | --- |
| llvm_ir | Target-specific 33-line LLVM module described above |
| abi.entry_symbol | recipe_add_f32 |
| abi.arguments | Read F32 buffer, read F32 buffer, write F32 buffer, element count |
| abi.argument_bytes | 32 bytes, four eight-byte argument slots |
| abi.argument_alignment | 8 bytes |
| abi.elements | ElementCount(1_048_576) |
| abi.workgroup_lanes | 256 |
| work | FlopCount(1_048_576), one Add FLOP per element |
| target | The exact AMD or NVIDIA target value parsed by main |

The ABI contains no FaultFlag, RunId, or LoopIteration. The three pointer
arguments correspond to the ordered template inputs and output, followed by the
explicit element count. Address formation, loads, stores, NaN checking, and
fault-free control flow do not increase work; scalar opcode pricing counts the
single F32 Add as one FLOP per logical element.

The argument_bytes calculation is based on the number of pointer arguments plus
the element count and uses checked arithmetic. The total work calculation also
uses checked multiplication. These overflow paths are not reachable with the
fixed one-million-element template, but another caller can receive
LoweringErrorKind::ArithmeticOverflow from the same public API.

## Observed LLVM outputs

The real Cargo entrypoint was run for both documented targets. The resulting
files had no stdout from the example and contained the expected target-specific
module differences:

| Invocation | File size | Target-specific declarations and definition |
| --- | ---: | --- |
| cargo run -q -p recipe-kernel --example lower_add -- gfx1101 /tmp/lower_add_amd_098.ll | 1,702 bytes, 33 lines | AMDGPU triple, AMD lane/workgroup intrinsics, amdgpu_kernel |
| cargo run -q -p recipe-kernel --example lower_add -- sm_86 /tmp/lower_add_nv_098b.ll | 1,719 bytes, 33 lines | NVPTX triple, NVVM lane/workgroup intrinsics, ptx_kernel |

Both outputs contain the same parameter order, * 256 index equation,
element_count bounds check, constrained F32 add, NaN canonicalization, one
output store, and strictfp attributes. The byte difference is the target
triple and intrinsic names, not a different scalar calculation.

The structural compile check also passes:

~~~text
cargo check -q -p recipe-kernel --example lower_add
status: 0
~~~

These commands prove source compilation and textual output generation. They do
not prove native compiler acceptance or device execution because this example
does not cross those boundaries.

## Stage lowering relationship

recipe_kernel::lower_stage is a separate public entrypoint exported by
kernel/src/lib.rs:

~~~text
lower_stage(
    &recipe_primitives::LoweredProgram,
    &recipe_core::ArtifactBuildRecipe,
    &KernelTarget,
    &LoweringOptions,
) -> Result<LoweredKernel, LoweringError>
~~~

lower_add does not construct a LoweredProgram or an ArtifactBuildRecipe, so it
never calls this function and never exercises stage-contract validation. The
stage path is used by production native preparation for planner-owned stages.
It independently checks canonical program and build validation, target and
option validity, program digest, source kernel, stage ordinal, stage-scoped
template identity, artifact ID, contract digest, dispatch geometry, work and
resource bounds, binding views, and fault binding. A mismatch is
LoweringErrorKind::InvalidStageContract.

When the selected stage is StageKind::ScalarMap, lower_stage delegates to the
same lower_elementwise implementation used here, then rewrites the generic
scalar fault publication to the stage's exact fault code and validates the
realized ABI against the immutable stage contract. Other stage kinds use the
owned stage emitter in kernel/src/stage.rs. This distinction matters: the
standalone example demonstrates template lowering only, while production stage
lowering also binds planner state and artifact identity.

## Native artifact builder relationship

The public ArtifactBuilder is downstream of LoweredKernel, not part of the
example's output path. Its public surface is:

| API | Input and result | Role |
| --- | --- | --- |
| PinnedTool::inspect | Executable path -> pinned path and SHA-256 digest | Captures a tool identity |
| ArtifactBuilder::new | OfflineToolchain -> builder | Verifies every required and optional pinned tool |
| ArtifactBuilder::build | One LoweredKernel, target, BuildPhase, private scratch parent -> BuiltArtifact | Verifies LLVM, emits one HSACO or cubin, then inspects it |
| build_hsaco_bundle | Ordered AMD lowered kernels -> BuiltHsacoBundle | Full-LTO multi-entry HSACO |
| build_cubin_bundle | Ordered NVIDIA lowered kernels -> BuiltCubinBundle | PTX translation and one ptxas invocation |

BuildPhase has only Offline and Realize. The builder has no compiler entrypoint
for Finalize, init, loop, or exit. It requires an existing, non-symlink scratch
parent that is inaccessible to group and other users, then creates a private
mode-700 workspace and removes it on drop.

For a single AMD build, the builder verifies the IR, invokes pinned LLVM code
generation for amdgcn, invokes the pinned ELF linker with the AMD processor and
code-object version, reads the HSACO, and calls inspect_hsaco against the
returned ABI. For a single NVIDIA build, it invokes LLVM for nvptx64, uses the
target's sm_86 and +ptx75 settings to produce PTX, invokes pinned ptxas, reads
the cubin, and calls inspect_cubin. Bundle methods perform the same checks for
every module, reject an empty list or duplicate entry symbol, and inspect each
requested entry in deterministic input order.

None of those tool invocations happen in lower_add. Writing an output named
kernel.hsaco or kernel.cubin would still write LLVM text because this example
writes lowered.llvm_ir without checking the suffix. A native artifact must come
from ArtifactBuilder after this lowering boundary.

## Artifact inspection relationship

The artifact inspection functions are also public re-exports, but they consume
native bytes rather than LLVM text:

~~~text
inspect_hsaco(bytes, expected_target_id, expected_code_object_version, expected_abi)
inspect_hsaco_bundle(bytes, expected_target_id, expected_code_object_version, expected_abis)
inspect_cubin(bytes, expected_sm, expected_entry_symbol)
~~~

ArtifactDigest is a SHA-256 digest of the complete byte slice and exposes raw
bytes and hexadecimal text. InspectedHsaco records the digest, ELF and code
object versions, flags, target ID, and parsed HSA kernel metadata. It requires
an AMDGPU-HSA ELF, matching code-object version and target feature set, defined
entry and descriptor symbols, no unresolved global symbols, an AMDGPU metadata
note, and metadata that matches the expected ABI order, offsets, sizes, global
buffer arguments, by-value element count, kernarg alignment and workgroup
bound. inspect_hsaco is the one-entry wrapper around the bundle parser.

InspectedCubin records the digest, ELF ABI and flags, decoded SM, entry symbol,
and executable text size. inspect_cubin requires an accepted CUDA ELF OS-ABI,
CUDA machine, matching decoded SM, no unresolved global symbols, a defined
global or weak function symbol with the requested name, a nonempty executable
.text.<entry> section, and a nonzero symbol size.

Malformed or truncated bytes return LoweringErrorKind::ArtifactFormat.
Correctly formed but wrong-target, wrong-ABI, missing-symbol, unresolved, or
otherwise contradictory bytes return LoweringErrorKind::ArtifactMismatch. The
.ll files produced by this example are intentionally not ELF files. The real
inspect_add entrypoint confirms the boundary:

~~~text
cargo run -q -p recipe-kernel --example inspect_add -- gfx1101 /tmp/lower_add_amd_098.ll
Error: LoweringError { kind: ArtifactFormat, scalar: None, message: "artifact is not an ELF file" }
status: 1
~~~

The NVIDIA .ll output fails with the same ArtifactFormat result. No inspection
is attempted by lower_add itself.

## Failure and output matrix

The observed CLI failures use the exact production Cargo example boundary.
status: 1 means the process returned an error; the custom string errors are
shown as quoted values by Rust's binary termination path.

| Invocation shape | Result |
| --- | --- |
| no arguments | "usage: lower_add <gfx1101|sm_86> <output.ll>" |
| gfx1101 /tmp/out.ll trailing | same usage error |
| gfx /tmp/out.ll | "AMD target must include its numeric target ID" |
| sm_8 /tmp/out.ll | "NVIDIA target must have the form sm_86" |
| sm_8a /tmp/out.ll | same NVIDIA shape error |
| foo /tmp/out.ll | "target must have the form gfx1101 or sm_86" |
| gfx11!01 /tmp/out.ll | LoweringError { kind: InvalidTarget, scalar: None, message: "AMD target ID contains unsupported characters" } |
| gfx0 /tmp/out.ll | success, because the current validator checks the gfx prefix and token characters, not numeric value |
| sm_99 /tmp/out.ll | success, because SM 99 is within the current major/minor validator bounds and PTX ISA is fixed at 75 |
| gfx1101 /tmp/no-such-parent/out.ll | raw std::io::Error, OS code 2, parent does not exist |
| gfx1101 /tmp | raw std::io::Error, OS code 21, output path is a directory |
| inspect_add gfx1101 /tmp/lower_add_amd_098.ll | LoweringErrorKind::ArtifactFormat, bytes are LLVM text rather than ELF |

Failures reachable only by constructing a different public value are also
defined by the lowering implementation:

| Boundary | Failure kind | Cause |
| --- | --- | --- |
| KernelTemplate::validate | InvalidKernel | Core scalar, access, count, dtype, ID, or alias-matrix validation fails |
| KernelTarget::validate | InvalidTarget | Target token, AMD code-object version, NVIDIA SM, or PTX ISA is invalid |
| LoweringOptions validation | InvalidEntrySymbol, InvalidWorkgroupSize | Entry name or workgroup width is outside the LLVM contract |
| Scalar value lookup | UnknownScalarValue | An operand or output is absent from the emitter's value map |
| Instruction dispatch | UnsupportedOperation | Opcode/type combination is not implemented by the LLVM emitter |
| Checked arithmetic | ArithmeticOverflow | FLOP, argument-count, or ABI-size arithmetic overflows |
| Module audit | ProhibitedInterface | Generated IR declares a non-llvm. external symbol |
| Stage contract | InvalidStageContract | Planner stage/build recipe differs from canonical state |
| Artifact parser | ArtifactFormat, ArtifactMismatch | Native bytes are malformed or contradict expected identity/ABI |
| Tool invocation | ToolchainFailed, Io | Pinned verifier, code generator, linker, assembler, workspace, or output fails |

LoweringError stores a machine-readable kind, an optional scalar ID, and a
message. Its Display form is Kind, optionally for scalar <id>, followed by the
message. main returns it through Box<dyn Error>, so the executable's top-level
diagnostic may use the full debug representation shown above.

## Non-goals and boundary invariants

- The example is not a benchmark. It does not time the generated kernel or
  claim a measured FLOP rate.
- It is not a native build test. LLVM verifier, LLVM code generation, an AMD
  linker, NVIDIA ptxas, and artifact inspection are all downstream APIs.
- It is not a runtime test. No device, allocation, queue, stream, launch, or
  init -> loop -> exit lifecycle is entered.
- The alias matrix is declared and validated as template metadata, but runtime
  pointer aliasing is owned by the planner and executor.
- The output extension is documentary only. The bytes are always the exact
  LoweredKernel.llvm_ir string produced by the selected target branch.
- The logical element count, workgroup width, entry symbol, target fields, and
  scalar operation are all fixed in source. There is no configuration file,
  environment fallback, retry, alternate target, or hidden artifact output.

The example's purpose is therefore narrow and complete: prove that a small
Recipe-owned scalar template can be validated and lowered into closed,
target-specific LLVM IR with an explicit ABI, while leaving native artifact
realization and execution to the public APIs designed for those later phases.
