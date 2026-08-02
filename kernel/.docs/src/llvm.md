# `kernel/src/llvm.rs`: elementwise LLVM lowering

This page documents the direct scalar-map lowering boundary implemented in
`kernel/src/llvm.rs`. The module converts one validated
`recipe_core::KernelTemplate` into target-specific LLVM IR plus the ABI and
work metadata needed by the offline artifact builder and native executors. It
does not invoke an LLVM executable, write an object file, or create a cubin or
HSACO. Those operations belong to `kernel/src/builder.rs` and consume the
`LoweredKernel` returned here.

The repository has a second, larger stage emitter in `kernel/src/stage.rs`.
That emitter owns fill, copy, reductions, scans, contractions, gather,
scatter, histogram, sort, index-map, and Philox stages. A scalar-map stage is
the one stage that delegates its scalar program to `lower_elementwise`; the
stage path then rewrites the generic scalar fault publication to the exact
stage fault code and release ordering. The distinction matters when tracing a
kernel: direct elementwise IR is emitted by this file, while owned-stage IR is
emitted by `stage.rs` and only shares the ABI and artifact builder.

## Source map and public data

The relevant definitions are:

| Source | Responsibility |
| --- | --- |
| `kernel/src/llvm.rs:10-63` | `BufferAccess`, `KernelArgument`, `KernelAbi`, `LoweringOptions`, and `LoweredKernel`. |
| `kernel/src/llvm.rs:71-170` | The private `Emitter`, scalar-value map, temporary naming, NaN canonicalization, and fault aggregation. |
| `kernel/src/llvm.rs:176-375` | `lower_elementwise`, the complete lowering pipeline, ABI construction, and FLOP accounting. |
| `kernel/src/llvm.rs:377-443` | Affine logical-index to physical-buffer-index emission. |
| `kernel/src/llvm.rs:445-498` | Entry-symbol and workgroup validation, and target-specific global-index intrinsics. |
| `kernel/src/llvm.rs:500-608` | Scalar opcode dispatch and operation-type failures. |
| `kernel/src/llvm.rs:610-929` | Individual floating-point, integer, comparison, conversion, classification, and shift emitters. |
| `kernel/src/llvm.rs:931-1029` | Module header, declarations, kernel signature, body, and function attributes. |
| `kernel/src/llvm.rs:1031-1038` | `DType` to LLVM type and opcode-to-FLOP adapters. |
| `kernel/src/target.rs` | AMD and NVIDIA target identity validation. |
| `kernel/src/audit.rs` | Closed-module audit that rejects non-LLVM external declarations. |
| `kernel/src/builder.rs` | Pinned verifier/code generator/linker/assembler invocation and artifact inspection. |
| `kernel/src/artifact.rs` | ELF, cubin, HSACO metadata, and ABI inspection. |

The implementation is `#![forbid(unsafe_code)]`. All IR is assembled into
`String` values with `fmt::Write`; there is no LLVM C API, vendor runtime, or
dynamic library call in this lowering layer.

### `KernelArgument`

The enum is the canonical argument description shared by lowering, artifact
inspection, launch planning, and CUDA/HSA argument packing:

| Variant | Meaning in a direct elementwise kernel |
| --- | --- |
| `Buffer { access, dtype, alignment }` | One global address-space pointer. Inputs are read pointers, outputs are write pointers, both with four-byte alignment. |
| `FaultFlag` | Optional global `i32` pointer. It is present when a checked scalar operation can reject an element. |
| `RunId` | Dynamic 64-bit run identity. Direct elementwise lowering never emits this; Philox stage lowering does. |
| `LoopIteration` | Dynamic 64-bit static-loop index. Direct elementwise lowering never emits this; stage lowering emits it only for iteration-dependent stages. |
| `ElementCount` | Final 64-bit by-value logical element count. |

`KernelAbi` records the entry symbol, ordered arguments, total explicit argument
bytes, eight-byte argument alignment, logical `ElementCount`, and workgroup
width. Every explicit ABI slot is eight bytes, including a pointer, a by-value
run value, or the element count. `LoweredKernel` carries the textual LLVM IR,
this ABI, total `FlopCount`, and the exact `KernelTarget` used to emit the
module. The target is retained because address spaces, intrinsics, calling
conventions, and floating-point attributes are target-specific.

`LoweringOptions` is intentionally small: an LLVM entry symbol and a
workgroup width. The options are not a substitute for a planner contract. The
stage path independently checks that the width equals the immutable stage
geometry before calling this function.

## Inputs and validation boundary

`lower_elementwise(template, target, options)` takes only in-memory values. It
does not read a file or inspect a device. Validation occurs before any emitter
state is created:

1. `template.validate()` checks the scalar program, input and output IDs,
   dtype agreement, rank and storage bounds of every `StaticBufferAccess`,
   writable-view injectivity, and the complete input/output alias matrix. Any
   validation aggregate is converted to `LoweringErrorKind::InvalidKernel`.
2. `target.validate()` checks the exact AMD or NVIDIA identity (see
   [Targets](#targets)).
3. `validate_options` checks the entry symbol and workgroup width. The symbol
   must be nonempty, begin with an ASCII letter or `_`, and contain only ASCII
   letters, digits, and `_`. The width must be in the inclusive range `1..=1024`.

The scalar validator is the reason the emitter can use direct indexing such as
`operands[0]` and `operands[1]`: it has already checked opcode arity, operand
definition order, and result dtype. A malformed or newer scalar opcode therefore
returns a typed error before an invalid IR instruction is accepted. Impossible
post-validation branches use `unreachable!` and are not fallback behavior.

The source `recipe_core::ScalarOpcode` domain is only `F32` and `I32`, and each
payload is four bytes. `IndexSpace::new` requires at least one nonzero
dimension and checked multiplication of dimensions. `StaticBufferAccess` stores
an element offset, one stride per logical axis, and complete backing storage
bytes. Template validation proves that the largest mapped element is within
that storage and that a writable mapping is injective. Lowering can therefore
use `getelementptr inbounds` and four-byte loads/stores without re-deriving
those contracts.

## Exact lowering sequence

The following sequence is the control/data flow in `lower_elementwise`:

```text
KernelTemplate + KernelTarget + LoweringOptions
        |
        +-- template.validate, target.validate, validate_options
        |
        +-- create Emitter
        +-- emit target global index and `%in_bounds` branch
        +-- emit `%canonical_nan` constant
        +-- map and load every scalar input
        +-- materialize every scalar constant
        +-- lower instructions in program order
        +-- publish one aggregated fault flag when required
        +-- map and store every scalar output
        +-- emit exit and `ret void`
        +-- assemble target module
        +-- audit all external declarations
        +-- compute ABI and checked FLOP work
        +-- return LoweredKernel
```

The function never launches the kernel. A lane processes at most one linear
logical element. The body is guarded by
`%in_bounds = icmp ult i64 %global_id, %element_count`; an out-of-range lane
branches directly to `exit` and executes no buffer access. The common body
defines `%canonical_nan` by bitcasting integer bits `2143289344` (`0x7fc00000`)
to `float`.

### Global index

`emit_global_index` selects the target intrinsic pair:

| Target | Local ID intrinsic | Workgroup ID intrinsic |
| --- | --- | --- |
| AMD | `@llvm.amdgcn.workitem.id.x()` | `@llvm.amdgcn.workgroup.id.x()` |
| NVIDIA | `@llvm.nvvm.read.ptx.sreg.tid.x()` | `@llvm.nvvm.read.ptx.sreg.ctaid.x()` |

Both values are zero-extended from `i32` to `i64`. The workgroup ID is multiplied
by `options.workgroup_lanes`, then the local ID is added:

```llvm
%group_base = mul i64 %group_id, <workgroup_lanes>
%global_id  = add i64 %group_base, %local_id
```

The builder later supplies the real machine width to LLVM. The IR itself uses
the exact width that is recorded in `KernelAbi`.

### Buffer index emission

For each input and output, `emit_buffer_index` converts the linear logical ID
to an affine physical element offset. Let `E` be the total logical element
count, `extent[a]` the axis extent, and `stride[a]` the static physical stride.
The emitter starts with `logical_stride = E`, divides by each extent in axis
order, and for each nontrivial axis:

1. computes `quotient = udiv %global_id, logical_stride`, unless that stride is
   one, in which case `%global_id` is used directly;
2. uses the quotient directly for a leading full-span axis, otherwise emits
   `urem` by the axis extent to obtain the coordinate;
3. multiplies by the physical stride unless it is one;
4. adds the contribution to the running offset, which starts at
   `offset_elements` when nonzero.

Extent-one axes and zero-stride broadcast axes emit no arithmetic. If no
contribution exists, the physical index is the literal `0`. The resulting IR
uses an `inbounds` GEP in `ptr addrspace(1)` followed by an aligned four-byte
load or store. Input bindings are paired with `program.inputs` by vector order,
and output bindings are paired with `program.outputs` by vector order. The
function does not inspect alias rules beyond the template validation already
performed.

### Constants and scalar values

The emitter keeps a `BTreeMap<ScalarValueId, ValueRef>`. A `ValueRef` retains
the validated `DType` and the textual LLVM operand. Inputs are inserted after
their loads. `ScalarLiteral::I32` becomes a textual integer operand. A
`ScalarLiteral::F32Bits` becomes a temporary produced by
`bitcast i32 <bits> to float`. Each instruction result is inserted only after
its operands have been looked up and lowered. Missing IDs return
`UnknownScalarValue` and carry the missing scalar ID in `LoweringError.scalar`.

Temporary names are monotonically numbered (`%<purpose>_<id>`), so names are
unique within one function and deterministic for a deterministic template.

## Scalar opcode lowering

`lower_instruction` dispatches on `(ScalarOpcode, instruction.dtype)`. The
validated operand `ValueRef`s carry the operand dtype, while the instruction
dtype is the result dtype. Every supported operation emits direct LLVM
instructions or an LLVM intrinsic. The following table is the complete current
dispatch:

| Opcode and result | Emission and contract |
| --- | --- |
| `Add`, `Subtract`, `Multiply`, `Remainder` with `F32` | `llvm.experimental.constrained.fadd/fsub/fmul/frem.f32` with `round.tonearest` and `fpexcept.ignore`, then NaN canonicalization. The corresponding declaration is recorded. |
| `Divide` with `F32` | Plain `fdiv float`, then NaN canonicalization. This intentionally has no constrained intrinsic declaration. |
| `Negate` with `F32` | Bitcast to `i32`, XOR with `0x80000000`, bitcast back. |
| `Absolute` with `F32` | Bitcast to `i32`, AND with `0x7fffffff`, bitcast back. |
| `Minimum` or `Maximum` with `F32` | `llvm.minimum.f32` or `llvm.maximum.f32`, then NaN canonicalization. |
| `Fma` with `F32` | `llvm.experimental.constrained.fma.f32` with round-to-nearest and ignored exceptions, then NaN canonicalization. It counts as two FLOPs. |
| `Add`, `Subtract`, `Multiply` with `I32` | Direct `add`, `sub`, or `mul i32`. LLVM integer wrap semantics are the scalar contract. |
| `Divide` or `Remainder` with `I32` | Checked truncating operation, described below. |
| `Negate` or `Absolute` with `I32` | Checked `INT_MIN` handling, described below. |
| `Minimum` or `Maximum` with `I32` | Signed `icmp slt` or `icmp sgt` followed by `select i1`. |
| Comparisons | Integer comparisons use `icmp`; float comparisons use `fcmp`. The boolean is zero-extended to `i32`. Predicates are ordered for all float comparisons except `NotEqual`, which uses `une` so NaN is not equal to itself. |
| `Select` | The first `i32` operand is nonzero-tested, then LLVM `select` chooses operand two or three with the validated result dtype. |
| `BitAnd`, `BitOr`, `BitXor`, `BitNot` with `I32` | Direct integer bit operations. `BitNot` XORs with `-1`. |
| `BitcastF32ToI32`, `BitcastI32ToF32` | LLVM bitcasts preserve the exact four-byte representation. |
| `ShiftLeft`, `ShiftRightLogical`, `ShiftRightArithmetic` with `I32` | The shift count is masked with `and i32 <count>, 31`, then `shl`, `lshr`, or `ashr` is emitted. |
| `Require` with `I32` | Tests the operand against zero, returns normalized `0` or `1`, and records rejection when the operand is zero. |
| `IsFinite` or `IsNan` with result `I32` | Bitcasts the float, masks exponent bits with `2139095040` (`0x7f800000`), and for NaN also tests a nonzero mantissa using `8388607` (`0x007fffff`). The boolean is zero-extended to `i32`. |
| `SquareRoot` with `F32` | `llvm.sqrt.f32`, then NaN canonicalization. |
| `Floor`, `Ceiling`, `RoundNearestEven` with `F32` | `llvm.floor.f32`, `llvm.ceil.f32`, or `llvm.roundeven.f32`, then NaN canonicalization. |
| `ConvertF32ToI32` | Saturating `llvm.fptosi.sat.i32.f32`. |
| `ConvertI32ToF32` | Signed `sitofp i32` to `float`. |

The float arithmetic helpers canonicalize NaN by emitting
`fcmp uno float <raw>, <raw>` and selecting `%canonical_nan` when unordered.
This canonicalization applies to constrained binary arithmetic, plain float
division, FMA, minimum, maximum, square root, floor, ceiling, and round-even.
The bit-level sign and absolute operations intentionally preserve the operand's
bit payload except for the sign bit.

### Checked integer operations and fault conditions

`ScalarProgram::requires_fault_flag` identifies exactly the operations that can
reject: `Require`, integer divide, integer remainder, integer negate, and
integer absolute. Their result remains deterministic, but the lane publishes a
fault after replacing an invalid operation with a safe operand:

- `sdiv` and `srem` test divisor zero and the `INT_MIN / -1` overflow pair. A
  rejected divisor is replaced with one, the operation is performed, and the
  result is selected as zero on rejection.
- Integer negate and absolute test `INT_MIN`. A rejected operand is replaced
  with zero before subtraction; absolute then selects the nonnegative operand
  or its negation.
- `Require` returns zero for a rejected condition and one otherwise.

Each rejection predicate is appended to `Emitter::fault_conditions`. After all
scalar instructions, `emit_fault_flag` ORs the predicates into one `i1`. If the
aggregate is true, it branches to a `fault_rejected_*` block and executes

```llvm
atomicrmw or ptr addrspace(1) %fault_flag, i32 1 monotonic, align 4
```

before joining the normal continuation. No fault argument or fault block is
emitted when the vector is empty. The ABI therefore has one `FaultFlag` slot
iff at least one checked operation occurs.

The scalar-map stage path in `stage.rs` requires a stage fault contract. Its
`rewrite_scalar_fault` function verifies that this exact `atomicrmw or`
instruction exists and replaces the first occurrence with

```llvm
atomicrmw xchg ptr addrspace(1) %fault_flag, i32 <stage_code> release, align 4
```

If the generic publication is absent, realization fails with
`InvalidStageContract`; it does not silently accept a scalar module with the
wrong fault channel.

### FLOP accounting

For every instruction, `instruction_flops` delegates to
`ScalarOpcode::flops()`. The per-element count is accumulated with checked
`u64` addition. FMA contributes two; arithmetic, comparisons, and selected
math functions contribute one; bit operations, conversions, classification,
select, and validation predicates contribute zero. The per-element value is
multiplied by `template.index_space.elements()` using checked arithmetic. An
overflow returns `ArithmeticOverflow` and no `LoweredKernel` is produced.

## Module assembly

`assemble_module` produces one closed textual module:

1. AMD starts with `target triple = "amdgcn-amd-amdhsa"`; NVIDIA starts with
   `target triple = "nvptx64-nvidia-cuda"`.
2. The target index intrinsic declarations are emitted for the selected backend.
3. Declarations are added only for intrinsic families actually used by the
   emitter. Constrained binary declarations are selected by searching the body
   for the corresponding `constrained.<operation>.f32` text. No non-LLVM
   declaration is generated by this file.
4. The entry function uses `amdgpu_kernel` for AMD or `ptx_kernel` for NVIDIA.
   Parameters are ordered as all input pointers, all output pointers, the
   optional fault pointer, and `i64 %element_count`.
5. The body is wrapped in `entry:` and ends with the explicit `exit:` block and
   `ret void`.
6. Function attribute group `#0` is
   `nounwind strictfp "denormal-fp-math-f32"="ieee" "no-trapping-math"="false"`.

Global pointers are always `ptr addrspace(1)`. The textual declaration is
target-neutral at the pointer level, while the target triple, intrinsic set,
calling convention, and downstream codegen select AMDGPU or NVPTX semantics.
The module does not contain `RunId` or `LoopIteration` parameters. Those are
added by the owned-stage emitter only when the stage contract requires them.

## Closed-module audit

Immediately after assembly, `lower_elementwise` calls `audit_llvm_ir`. The
audit scans every line whose trimmed text starts with `declare`, extracts the
symbol after `@`, and accepts it only when the symbol starts with `llvm.`. Any
other external declaration returns `LoweringErrorKind::ProhibitedInterface`
with the declaration line, audit kind, and token in the message. The audit does
not maintain a second vendor-library deny list; repository, dependency,
linker, dynamic-load, and final-ELF policy belongs to the separate
`recipe-audit` gate. This separation keeps the generated module closed without
duplicating policy in the compiler.

## ABI construction

After the audit succeeds, the ABI is derived from the same template vectors
used to emit the function:

```text
read Buffer for each template input, in input order
write Buffer for each template output, in output order
FaultFlag, only when fault_conditions is nonempty
ElementCount
```

`pointer_arguments` is the input count plus output count plus one optional fault
pointer. `argument_bytes` is `(pointer_arguments + 1) * 8`, where the final one
is `ElementCount`; each addition and multiplication is checked. The returned
ABI has `argument_alignment = 8`, `elements = template.index_space.elements()`,
and `workgroup_lanes = options.workgroup_lanes`. Buffer descriptors use
`alignment = 4` because both supported dtypes are four-byte payloads.

There are two related ABI builders in the repository:

- `llvm.rs` emits every template input pointer, even though an input access
  view may be broadcast or have a zero stride. The pointer still exists because
  the scalar program has one input per template input.
- `stage.rs::StageSignature` filters bindings by the stage's read/write access,
  excludes the reserved fault binding from ordinary buffers, and appends fault,
  run, and loop arguments according to the stage kind. It validates its ABI
  against the immutable `ArtifactBuildRecipe` before returning a
  `LoweredKernel`.

The native executor treats this ordering as protocol, not metadata. CUDA's
`fill_invocation` and HSA's `fill_kernarg` consume buffer locations in input
then output order, bind the optional fault pointer once, bind run and iteration
values by enum position, and require `ElementCount` to be last. A mismatch is a
runtime protocol error. The HSA artifact inspector additionally requires every
explicit slot to be an eight-byte argument at offset `index * 8`, and verifies
that pointer, by-value, name, and address-space metadata match `KernelArgument`.

## Targets

`KernelTarget` has two variants:

| Variant | Validation in `target.rs` | LLVM use in `llvm.rs` | Codegen use in `builder.rs` |
| --- | --- | --- | --- |
| `Amd(AmdTarget)` | `target_id` is a token, begins with `gfx`, and `code_object_version` is nonzero. Feature modifiers after `:` are allowed by token validation. | AMDGPU intrinsics, `amdgcn-amd-amdhsa` triple, `amdgpu_kernel` convention. | Splits target ID into processor and `+/-` features, then passes `-march=amdgcn`, `-mcpu=<processor>`, code-object version, and optional `-mattr`. The ELF linker receives the same processor, features, and code-object version. |
| `Nvidia(NvidiaTarget)` | `sm_major` is `3..=12`, `sm_minor <= 9`, and `ptx_isa` is `32..=90`. | NVVM special-register intrinsics, `nvptx64-nvidia-cuda` triple, `ptx_kernel` convention. | Uses `sm_<major><minor>`, `-march=nvptx64`, `-mcpu=sm_<...>`, and `-mattr=+ptx<ptx_isa>`, then invokes pinned `ptxas -arch=sm_<...>`. |

The target identity is copied into `LoweredKernel.target`. `ArtifactBuilder::build`
rejects a requested target that differs from this exact value, so IR cannot be
compiled for a nearby architecture accidentally. The target-specific metadata
is not encoded in a direct elementwise artifact until the builder's codegen and
artifact inspector run.

## Tool invocation and artifact path

The direct caller normally hands the returned value to `ArtifactBuilder`:

```text
lower_elementwise(template, target, options)
        |
        v
ArtifactBuilder::build(phase, lowered, target, scratch_parent)
        |
        +-- verify pinned verifier and LLVM codegen tools
        +-- write kernel.ll in a private build workspace
        +-- verifier: -passes=verify -disable-output kernel.ll
        +-- AMD: llvm codegen -> kernel.o, ELF linker -> kernel.hsaco
        +-- NVIDIA: llvm codegen -> kernel.ptx, ptxas -> kernel.cubin
        +-- inspect exact target, entry symbol, and ABI
        +-- return BuiltArtifact with bytes and provenance
```

`OfflineToolchain` contains a pinned verifier, pinned LLVM code generator, and
an optional pinned ELF linker or PTX assembler. `PinnedTool::verify` requires
an absolute canonical regular-file path and recomputes the executable digest;
path changes or digest changes return `ArtifactMismatch`. `ArtifactBuilder::new`
verifies every configured tool. Every later invocation verifies the selected
tool again immediately before execution.

`BuildWorkspace::create` requires an existing non-symlink scratch parent with
no group or other permissions, creates a unique `recipe-build-<pid>-<nonce>`
directory, and sets it to mode `0700`. Inputs are written with `create_new` and
mode `0600`, then `sync_all` is called. Outputs must be non-symlink regular
files. The workspace removes itself on drop, including after an error.

`invoke` clears the child environment, sets `LC_ALL=C` and
`SOURCE_DATE_EPOCH=0`, records normalized arguments in `ToolInvocation`, and
returns `ToolchainFailed` with at most 16 KiB of stderr when a tool exits
unsuccessfully. The normalized `@scratch` paths make provenance deterministic
without exposing the per-run workspace name.

For one AMD kernel, LLVM codegen receives `-filetype=obj`, `-march=amdgcn`,
`-mcpu`, code-object version, optional features, `kernel.ll`, and `-o
kernel.o`; the linker receives `-shared --no-undefined kernel.o -o kernel.hsaco`.
For one NVIDIA kernel, LLVM codegen receives `-march=nvptx64`, `-mcpu`, the
PTX feature, `kernel.ll`, and `-o kernel.ptx`; `ptxas` receives
`-arch=<architecture> kernel.ptx -o kernel.cubin`.

Bundle builds verify every module, preserve caller-provided deterministic input
order, and produce one image. AMD bundles first serialize each module to
bitcode with the verifier, then invoke the pinned ELF linker once with
`--lto-O0`, `--no-undefined`, target processor/features, and code-object
version. NVIDIA bundles verify and lower each module to PTX, then invoke
`ptxas` once with all PTX files. Duplicate entry symbols, empty bundles, target
mismatches, or missing linker/assembler configuration fail before a tool is
run.

The builder records SHA-256 digests of every LLVM input and each invoked pinned
tool in `BuildProvenance`, `HsacoBundleProvenance`, or
`CubinBundleProvenance`. `BuiltArtifact` contains bytes, structural inspection,
and provenance. The single-kernel NVIDIA variant also returns the generated PTX
bytes in its `ptx` field; bundle results return only the shared native image.
These are realization artifacts, not model semantics, and no intermediate
`.ll`, `.bc`, `.o`, or `.ptx` file escapes the private workspace.

## Artifact inspection boundary

`inspect_hsaco` and `inspect_cubin` are called after code generation, so a
successful LLVM string is not treated as a successful native artifact.

For HSACO, the inspector verifies AMDGPU-HSA ELF identity, code-object version,
target ID, defined symbols, AMDGPU MessagePack metadata, and every requested
entry's `.kd` descriptor. It checks kernarg alignment and size, maximum
workgroup width, explicit argument order and offsets, eight-byte sizes, global
buffer versus by-value kinds, optional names, and absence of unexpected
non-hidden trailing arguments. The bundle inspector decodes the ELF symbol
table and metadata once, then preserves requested ABI order.

For cubin, the inspector accepts only the known CUDA ELF OS-ABI tags and CUDA
machine ID, decodes the observed SM flag layout, checks the exact expected SM,
requires a defined global function symbol, and requires a nonempty executable
`.text.<entry>` section. Unresolved global symbols are rejected by both
artifact paths. Artifact digests are computed from final bytes.

## Callers and complete data flow

The public re-exports in `kernel/src/lib.rs` expose `lower_elementwise`, the
ABI types, targets, builder, and inspectors. Current production and example
callers are:

| Caller | Entry path | Purpose |
| --- | --- | --- |
| `kernel/examples/lower_add.rs` | Builds a `KernelTemplate`, calls `lower_elementwise`, writes the returned `.llvm_ir` to the user-selected path. | Human-readable direct IR example. Accepted target syntax is `gfx<id>` or `sm_<two digits>`. |
| `kernel/examples/inspect_add.rs` | Lowers the same add template, reads an existing artifact, then calls `inspect_hsaco` or `inspect_cubin` with `lowered.abi`. | ABI and target inspection example. |
| `native-probe/src/cuda.rs` | Builds a probe template, lowers for a measured NVIDIA target, then calls `ArtifactBuilder::build(BuildPhase::Realize, ...)`. | Probe artifact used for measured CUDA timings. |
| `native-probe/src/hsa.rs` | Builds a probe template, lowers for a measured AMD target, then calls `ArtifactBuilder::build(BuildPhase::Realize, ...)`. | Probe artifact used for measured HSA timings. |
| `prepare/src/production.rs` | `lower_deferred_stage` calls `recipe_kernel::lower_stage`, which delegates scalar maps here and emits owned stages in `stage.rs`; grouped lowered kernels go to `build_cubin_bundle` or `build_hsaco_bundle` with `BuildPhase::Realize`. | Realize-time native artifact preparation from immutable planner recipes. |
| `src/native_prepare.rs` | Constructs exact target/toolchain specifications and an `ArtifactBuilder`; its deferred compiler resolves one lowered program and stage contract per artifact. | Native preparation setup and target identity validation. |

The production path is therefore:

```text
planner LoweredProgram + ArtifactBuildRecipe
        -> prepare::lower_deferred_stage
        -> recipe_kernel::lower_stage
             -> validate complete stage contract
             -> scalar map: lower_elementwise + fault rewrite
             -> owned stage: stage.rs emitter
        -> ArtifactBuilder bundle by exact target
        -> artifact inspector
        -> NativeArtifact / RuntimeImage
        -> native-executor plan and launch
```

CUDA and HSA launchers consume the returned ABI rather than reconstructing it.
`fill_invocation` and `fill_kernarg` bind pointers from resolved arena locations,
the optional fault flag, dynamic run and loop values, and the final element
count. They retain arena ownership for pointer arguments and reject missing,
extra, repeated, or misordered operands. This preserves the single source of
truth established by lowering and artifact inspection.

## Failure and invariant matrix

The error kind is part of the fail-closed boundary. The direct elementwise
function can return the first group below; the builder, stage verifier, and
artifact inspector add the downstream kinds.

| Error kind | Observed cause | Boundary |
| --- | --- | --- |
| `InvalidKernel` | Template or scalar program validation failed, or a validated output value changed dtype during lowering. | `lower_elementwise` and template contract. |
| `InvalidTarget` | AMD or NVIDIA target identity is outside the allowed token, architecture, code-object, or PTX ranges; AMD codegen feature syntax is malformed. | `target.rs` and builder target argument construction. |
| `InvalidEntrySymbol` | Empty or punctuated entry symbol; duplicate bundle entry symbol. | `validate_options`, bundle checks. |
| `InvalidWorkgroupSize` | Width is zero or greater than 1024. | `validate_options`. |
| `UnknownScalarValue` | Instruction or output references a value absent from the emitter map. | `Emitter::value` and `lower_instruction`. |
| `UnsupportedOperation` | FMA requested for `I32`, bitwise/shift requested for `F32`, conversion result type does not match its opcode, or an opcode is newer than this dispatch. | `lower_instruction`. |
| `ArithmeticOverflow` | Per-element FLOP sum, total FLOP multiplication, argument count/size, or checked target arithmetic overflowed. | `lower_elementwise`, builder, and artifact decoders. |
| `ProhibitedInterface` | Assembled IR declares an external symbol that is not an `llvm.` intrinsic. | `audit_llvm_ir` immediately after assembly. |
| `InvalidStageContract` | Planner program, recipe digest, stage identity, geometry, ABI, work, or scalar fault publication does not match the immutable realization contract. | `stage.rs` before/after this module. |
| `ArtifactMismatch` | Lowered target differs from requested build target, pinned tool path/digest changed, or generated ELF target/entry/ABI differs from the expected value. | `ArtifactBuilder` and artifact inspectors. |
| `ToolchainFailed` | A verifier, LLVM code generator, linker, or PTX assembler exits unsuccessfully. | `builder.rs::invoke`. |
| `ArtifactFormat` | Output is not a bounded little-endian ELF64, lacks required sections/metadata, or contains malformed MessagePack/ELF fields. | `artifact.rs`. |
| `Io` | Scratch parent, tool, input, or output violates path, regular-file, permission, or filesystem requirements. | `builder.rs`. |

The following invariants are intentional and should remain visible in any
change to this module:

- Only `F32` and `I32` payloads are lowered, each with four-byte memory access.
- Every lane derives one 64-bit global logical index and checks it before
  touching a buffer.
- Every input and output pointer is in global address space and appears in the
  fixed ABI order. `ElementCount` is always the final argument.
- Checked scalar rejection is aggregated per lane. A direct scalar module uses
  `atomicrmw or` with monotonic ordering; a stage contract rewrites it to its
  exact release `atomicrmw xchg` code.
- Floating-point operations use strict function attributes, deterministic
  round-to-nearest metadata where constrained intrinsics are used, IEEE
  denormals, and canonical NaN outputs for arithmetic/math paths.
- Generated modules declare LLVM intrinsics only. Host runtimes, vendor math,
  and unresolved helper symbols cannot enter through this lowering boundary.
- The target copied into `LoweredKernel` is immutable evidence of the IR's
  intended backend. The builder and final artifact inspector must match it
  before an image is accepted.
- Discovery, compilation, allocation, and native-image loading are preparation
  work. They are not model calculations executed in the finalized loop.

There are no retries, alternate code generators, compatibility aliases, or
silent target fallbacks. A failed validation, audit, tool invocation, or
artifact inspection remains an observable failure and must be repaired at its
own boundary.

## Reproducing the direct example

`kernel/examples/lower_add.rs` constructs a one-dimensional `1_048_576`-element
F32 add template, lowers it with entry symbol `recipe_add_f32` and width `256`,
and writes only the textual IR requested by the caller. Its source-level usage
is:

```text
cargo run -p recipe-kernel --example lower_add -- gfx1101 /tmp/add.ll
cargo run -p recipe-kernel --example lower_add -- sm_86 /tmp/add.ll
```

The example itself maps `gfx<id>` to AMD code-object version `6`, and `sm_86` to
SM 86 with PTX ISA `75`; production preparation obtains these values from the
measured target and pinned toolchain instead of the example defaults. A valid
`.ll` file proves only that this module emitted and audited a string. Native
correctness requires the pinned offline build and the corresponding HSACO or
cubin structural inspection described above.
