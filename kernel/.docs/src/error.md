# `recipe_kernel::LoweringError`

`LoweringError` is the single error type for the `recipe-kernel` crate. It is
returned by target validation, scalar and owned-stage LLVM lowering, native
artifact construction, and HSACO/cubin inspection. The type is re-exported by
[`kernel/src/lib.rs`](../../src/lib.rs#L27), so callers do not need to name the
private `error` module.

This page follows the current implementation in
[`kernel/src/error.rs`](../../src/error.rs). The source locations below are
part of the trace: they identify the producer, not a hypothetical future
error path.

## Shape and rendering

The public type is:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweringError {
	pub kind: LoweringErrorKind,
	pub scalar: Option<recipe_core::ScalarValueId>,
	pub message: String,
}
```

`LoweringErrorKind` is `Copy`, `Clone`, `Debug`, `Eq`, and `PartialEq`, and is
marked `#[non_exhaustive]`. `LoweringError` has the same comparison and debug
traits, but is not non-exhaustive. The two public fields are deliberately
inspectable. `kind` is the machine-selectable category, `message` is the
human-readable detail, and `scalar` optionally identifies the scalar result
that made a scalar lowering failure observable.

`LoweringError::new(kind, message)` accepts any `Into<String>`, stores the
converted message, and initializes `scalar` to `None`. It is the only general
constructor. `for_scalar(id)` consumes an existing error, replaces its scalar
context with `Some(id)`, and returns it. Both methods are `#[must_use]`.

`Display` is exact and intentionally compact:

```text
{kind:?}: {message}
{kind:?} for scalar {scalar}: {message}
```

The second form is selected only when `scalar` is `Some`. The kind uses its
Rust `Debug` spelling, not a separate wire label. `ScalarValueId` uses its own
`Display` implementation. `LoweringError` implements `std::error::Error` with
no `source()` override, so an error does not expose a nested cause. Filesystem,
tool, compiler, and parser details are already flattened into `message`.

## Complete kind inventory

The enum currently has thirteen variants. It is non-exhaustive, so downstream
matches must retain a wildcard arm.

| Kind | Meaning and current producers | Native consequence |
| --- | --- | --- |
| `InvalidKernel` | `KernelTemplate::validate` failure, an output type changing during elementwise lowering, or an empty AMD/NVIDIA bundle. See [`llvm.rs`](../../src/llvm.rs#L181) and [`builder.rs`](../../src/builder.rs#L232). | Rejected before backend compilation. An empty-bundle error is also before any verifier invocation. |
| `InvalidStageContract` | Canonical lowered-program/build-recipe mismatch, an impossible owned-stage binding or geometry, an invalid primitive contract, or an internal stage indexing invariant. All such sites use `stage_error` or `ensure_stage` in [`stage.rs`](../../src/stage.rs#L122). | `lower_stage` stops before returning LLVM IR. No native compiler is invoked. |
| `InvalidEntrySymbol` | Entry symbol is not an ASCII LLVM identifier, or a bundle repeats an entry symbol. See [`llvm.rs`](../../src/llvm.rs#L445), [`stage.rs`](../../src/stage.rs#L231), and [`builder.rs`](../../src/builder.rs#L250). | Rejected before the corresponding LLVM module is compiled. |
| `InvalidTarget` | AMD/NVIDIA target fields are invalid, AMD target feature text cannot be converted to compiler options, or the required pinned linker/assembler is absent. See [`target.rs`](../../src/target.rs#L8) and [`builder.rs`](../../src/builder.rs#L263). | Target and tool availability failures stop compilation. The error can also be raised while selecting a backend compiler in a bundle build. |
| `InvalidWorkgroupSize` | `LoweringOptions.workgroup_lanes` is outside `1..=1024`. The scalar and stage paths validate it independently. | Rejected before IR is emitted or compiled. |
| `UnknownScalarValue` | An instruction operand or output references a `ScalarValueId` absent from the emitter value map. [`Emitter::value`](../../src/llvm.rs#L117) attaches that ID with `for_scalar`. | Scalar lowering stops before module audit and native compilation. |
| `UnsupportedOperation` | A scalar opcode/dtype pair is not implemented, including integer `Fma`, float bitwise/shift operations, invalid conversion result types, and a newer opcode. [`lower_instruction`](../../src/llvm.rs#L500) attaches the result ID. | No module is returned and no native compiler is invoked. |
| `ArithmeticOverflow` | Checked FLOP, argument-count, ABI-size, stage-ABI, and NVIDIA SM calculations overflow. Producers are in [`llvm.rs`](../../src/llvm.rs#L240), [`stage.rs`](../../src/stage.rs#L448), and [`builder.rs`](../../src/builder.rs#L617). | Lowering-time overflows stop before compilation. `nvidia_sm` is called after cubin output is read, so that checked conversion can report after a backend invocation. |
| `ProhibitedInterface` | The generated LLVM module contains an external declaration whose symbol does not begin with `llvm.`. The first finding from `audit_llvm_ir` is reported by scalar and owned-stage lowering. | Module audit fails before native compilation. Target intrinsics are allowed; unresolved host/runtime interfaces are not. |
| `ArtifactFormat` | ELF64, section-table, symbol-table, AMDGPU MessagePack, byte-range, integer, UTF-8, and metadata decoding failures. Every constructor goes through `artifact_format` in [`artifact.rs`](../../src/artifact.rs#L770). | Usually raised after a compiler or prebuilt image supplied bytes, during structural inspection. The image is never accepted as a runtime artifact. |
| `ArtifactMismatch` | A structurally readable tool, target, symbol, ABI, metadata record, code-object version, SM, or undefined-symbol set disagrees with the requested identity. Constructors are in [`builder.rs`](../../src/builder.rs#L49) and the `artifact_mismatch` helper in [`artifact.rs`](../../src/artifact.rs#L774). | The produced or supplied image is rejected. A backend process may already have run before the mismatch is discovered. |
| `ToolchainFailed` | A pinned verifier, LLVM code generator, linker, or `ptxas` exits unsuccessfully. [`invoke`](../../src/builder.rs#L629) includes the status and at most 16 KiB of stderr. | The current compile pipeline stops. Earlier successful invocations are not retried or replaced. |
| `Io` | Path, filesystem, workspace, regular-file, permission, read/write, and process-exec failures in the pinned-tool builder. See [`builder.rs`](../../src/builder.rs#L713). | The current operation stops and its private build workspace is dropped. No fallback compiler or path is attempted. |

The `#[non_exhaustive]` marker applies to `LoweringErrorKind`, not to the
string format. A caller should branch on `kind`, retain `message` for logs, and
avoid parsing the `Display` text as a protocol.

## Construction trace by subsystem

### Target and option validation

[`AmdTarget::validate`](../../src/target.rs#L8) first calls `validate_token`.
An empty target ID or any byte other than ASCII alphanumeric, `_`, `-`, `+`,
`:`, or `.` yields `InvalidTarget` with
`"AMD target ID contains unsupported characters"`. The target must then begin
with `gfx`, and `code_object_version` must be nonzero. The two failures are
`"AMD target ID must begin with \`gfx\`"` and
`"AMDGPU code-object version must be nonzero"`.

[`NvidiaTarget::validate`](../../src/target.rs#L38) accepts SM major values
3 through 12, SM minor values through 9, and PTX ISA values from 32 through
90. It reports `"unsupported NVIDIA target sm_{major}{minor}"` or
`"unsupported PTX ISA {ptx_isa}"` as `InvalidTarget`. `KernelTarget::validate`
dispatches directly to the selected target variant.

`LoweringOptions` validation is duplicated intentionally at the two real
lowering boundaries. [`llvm::validate_options`](../../src/llvm.rs#L445) and
[`stage::validate_options`](../../src/stage.rs#L231) require a nonempty symbol
whose first byte is ASCII alphabetic or `_`, whose remaining bytes are ASCII
alphanumeric or `_`, and a workgroup size in `1..=1024`. They return
`InvalidEntrySymbol` with `"entry symbol must be an ASCII LLVM identifier
without punctuation"` or `InvalidWorkgroupSize` with
`"workgroup size must be between 1 and 1024 lanes"`.

The builder adds target-specific validation. `amd_processor_and_features`
([`builder.rs`](../../src/builder.rs#L676)) rejects an empty processor,
features without a trailing `+` or `-`, and feature names containing anything
other than ASCII alphanumeric or `_`, all as `InvalidTarget`. AMD builds need a
pinned ELF linker and NVIDIA builds need a pinned `ptxas`; missing entries are
reported as `InvalidTarget` before invoking a backend compiler.

### Scalar elementwise lowering

`lower_elementwise` follows this order ([`llvm.rs`](../../src/llvm.rs#L176)):

1. `KernelTemplate::validate` errors are converted to `InvalidKernel` with the
   validation report as the message.
2. The target and options are validated.
3. Inputs and constants populate the emitter's `BTreeMap<ScalarValueId, ValueRef>`.
4. Every instruction is passed to `lower_instruction`.
5. FLOP counts, the generated LLVM module, the closed-module audit, the ABI
   argument count and byte size, and total work are checked in order.

`Emitter::value` is the sole unknown-value producer. A missing map entry yields
`UnknownScalarValue` with `"scalar value {id} is unavailable during lowering"`
and `scalar = Some(id)`.

`lower_instruction` resolves operands through that method, then matches the
opcode and dtype. The four direct `UnsupportedOperation` constructions are:

* integer `Fma`: `"Fma is defined only for f32"`;
* float bitwise or shift operation: `"bitwise and shift operations require
  int32"`;
* a conversion whose result dtype does not match its opcode:
  `"conversion result type does not match the opcode"`;
* every unmatched pair: `"scalar opcode is newer than this lowering
  implementation"`.

Each is tagged with `instruction.result` through `for_scalar`. A bad validated
output type is an `InvalidKernel` with
`"validated output type changed during lowering"` and the output scalar ID.

The checked scalar arithmetic sites use `ArithmeticOverflow` for
`"per-element FLOP count overflowed"`, `"kernel argument count overflowed"`,
`"kernel fault argument count overflowed"`, `"kernel ABI size overflowed"`,
and `"kernel FLOP count overflowed"`. A prohibited external declaration is
reported as `ProhibitedInterface` with
`"generated LLVM IR failed audit at line {line}: {kind:?} \`{token}\`"`.
The audit only rejects declarations whose symbol does not begin with `llvm.`.

No scalar error is wrapped in another crate error inside `recipe-kernel`.
Every `?` in this path returns the same `LoweringError` unchanged.

### Owned primitive-stage lowering

The public [`lower_stage`](../../src/lib.rs#L38) calls
[`stage::lower_stage`](../../src/stage.rs#L86). It first validates the complete
`LoweredProgram` and `ArtifactBuildRecipe`, rather than trusting a caller
selected stage fragment. The scalar-map variant delegates to
`lower_elementwise`, rewrites its checked fault publication, and validates the
realized ABI. Every other `StageKind` goes through `lower_owned_stage`.

#### Admission and immutable-contract checks

`validate_contract` uses `stage_error` and `ensure_stage`, both of which create
`LoweringErrorKind::InvalidStageContract` through `LoweringError::new`
([`stage.rs`](../../src/stage.rs#L283)). The complete admission messages are:

* `lowered program failed canonical validation`;
* `artifact build recipe failed canonical validation`;
* `artifact build program digest differs from the canonical lowered program`;
* `artifact build source kernel differs from the lowered program`;
* `artifact build stage ordinal is absent from the lowered program`;
* `artifact build stage-scoped kernel identity is not canonical`;
* `artifact ID differs from the reserved stage-scoped identity`;
* `artifact build contract digest does not match its contents`;
* `artifact dispatch geometry differs from the primitive stage`;
* `lowering options workgroup size differs from the immutable stage geometry`;
* `artifact work bounds differ from the primitive stage`;
* `artifact resource envelope differs from the primitive stage`;
* `artifact binding count differs from the primitive stage`;
* `artifact binding differs from the primitive stage`;
* `artifact supplies a fault value for an unchecked stage`;
* `stage fault flag has no ordered binding`;
* `artifact fault value differs from the stage fault binding`; and
* `canonical stage-scoped kernel identity is the reserved zero value`.

The target and options calls inside this function retain their own
`InvalidTarget`, `InvalidEntrySymbol`, and `InvalidWorkgroupSize` kinds. They
are not normalized to `InvalidStageContract`.

`StageSignature::pointer` adds the dynamic message
`"stage binding {index} is absent"`. Its `abi` method adds
`ArithmeticOverflow` for an argument count that cannot fit `u32`, or
`"stage kernel ABI size overflowed"`, and maps an invalid `ElementCount` to
`InvalidStageContract` with
`"stage logical lane count cannot form a kernel ABI: {error}"`.

The `Ir` load, store, and write-address helpers add dynamic contract errors:
`"{purpose} binding is not readable"` and
`"{purpose} binding is not writable"`. They are reached by all owned emitters
through `?`, so the original `InvalidStageContract` is preserved.

#### Owned-stage dispatch and final checks

`lower_owned_stage` constructs signatures for `Fill`, `Copy`, fixed-tree
reduction, local and uniform scans, tiled contraction, gather, scatter,
histogram clear/accumulate, the three stable-sort phases, index map, and
Philox4x32-10 ([`stage.rs`](../../src/stage.rs#L725)). A scalar-map reaching
this owned emitter is an invariant failure with
`"scalar-map stage reached the owned-stage emitter"`.

After emission, `audit_llvm_ir` produces `ProhibitedInterface` with the same
line/kind/token format as scalar lowering. `validate_realized_kernel` then
checks the logical element count, workgroup size, work, and exact count of
fault-flag arguments. Mismatches are `InvalidStageContract` with
`"realized kernel ABI or work differs from the immutable build recipe"` or
`"realized kernel fault ABI differs from the immutable stage"`.

#### Primitive emitter contract messages

The following list is an exhaustive index of the remaining
`InvalidStageContract` constructors in [`stage.rs`](../../src/stage.rs). It is
organized by emitter, while the linked source ranges retain the exact
conditions and call order.

| Emitter or helper | Messages |
| --- | --- |
| `rewrite_scalar_fault`, fill, copy, and index map ([`stage.rs`](../../src/stage.rs#L104)) | `scalar stage requires a fault publication that the scalar emitter did not produce`; `fill literal type differs from its output`; `copy input and output types differ`; `index-map stage omitted its checked fault contract`; `index-map type, dynamic ABI, or arithmetic-fault contract is invalid`; `index-map modulus is not strictly positive`. |
| Scan-axis discovery ([`stage.rs`](../../src/stage.rs#L1149)) | `uniform scan stage has no same-level local stage carrying its axis`; `uniform scan stage has ambiguous same-level scan axes`. |
| Reduction ([`stage.rs`](../../src/stage.rs#L1194)) | `reduction index domain exceeds the int32 result contract`; `reduction padding or tie-break contract is not Recipe canonical`; `reduction stage has no readable value binding`; `reduction value input type differs from its stage contract`; `later reduction pass omitted its readable index binding`; `value reduction omitted its writable result`; `index reduction omitted its writable result`; `indexed reduction omitted its writable value`; `indexed reduction omitted its writable index`; `reduction index binding is not int32`; `reduction synchronization ordinal differs from its tree step`; `reduction index combine contract is incomplete`; `reduction index output contract is incomplete`; `fixed tree differs from launch geometry or synchronization count`; `fixed tree synchronization semantics are not canonical`; `reduction axis exceeds its first-pass input rank`; `reduction coordinate underflow`; `free reduction coordinate underflow`; `Any and All fixed trees require int32`; `reduction combine has only one index operand`; `Any and All combine require int32`; `ordered comparison has only one index operand`; and the conversion detail from the reduction tree step ordinal when its `usize` cannot fit `u32`. |
| Scans ([`stage.rs`](../../src/stage.rs#L2037)) | `scan binding type differs from its contract`; `scan tree contains a reduction step`; `scan synchronization ordinal differs from its tree step`; `scan axis or width differs from its tensor binding`; `scan coordinate underflow`; `uniform scan binding type differs from its contract`; `uniform scan stage has no post-first-block elements`; `uniform scan axis or width differs from its target binding`; `uniform scan coordinate underflow`; and the conversion detail from the scan tree step ordinal when its `usize` cannot fit `u32`. |
| Contraction ([`stage.rs`](../../src/stage.rs#L2489)) | `contraction binding type differs from its contract`; `contraction does not declare canonical contracted order`; `contraction axis pairs do not match operand bindings`; `contraction output rank differs from its canonical axis mapping`; `contraction workgroup width overflowed`; `NVIDIA TF32 contraction tile count overflowed`; `NVIDIA TF32 contraction workgroup width overflowed`; and `NVIDIA TF32 contraction requires complete warps`. |
| Gather and scatter ([`stage.rs`](../../src/stage.rs#L3010)) | `gather binding types are invalid`; `rejecting gather lacks its exact fault guard`; `gather bounds policy and fault contract disagree`; `scatter binding types are invalid`; `rejecting scatter lacks its exact fault guard`; `scatter bounds policy and fault contract disagree`; `atomic scatter omitted its payload atomic contract`; `scatter payload atomic differs from its stage contract`; `indexed operation axis or result rank is invalid`; `indexed operation index rank differs from the result/payload relation`; `indexed extent is outside the signed index domain`; and `fault publication contract is not the Recipe checked-path ABI`. |
| Histogram ([`stage.rs`](../../src/stage.rs#L3441)) | `histogram stage omitted its checked fault contract`; `histogram binding shape or type differs from its contract`; `histogram omitted its bin atomic contract`; `histogram bin atomic differs from its stage contract`; and `histogram bin mapping differs from its input dtype`. |
| Stable sort ([`stage.rs`](../../src/stage.rs#L3560)) | `sort network is not the Recipe stable total-order contract`; `sort initialization bindings differ from its scratch network`; `sort compare bindings or network phase are invalid`; `sort final binding types are invalid`; `sort final output axis differs from its network`; `sort input axis differs from its network`; and `sort input coordinate underflow`. |
| Philox and random distributions ([`stage.rs`](../../src/stage.rs#L3994)) | `Philox stage is not the Recipe Philox4x32-10 v1 contract`; `Philox output dtype differs from its distribution`; `Bernoulli probability is outside the closed unit interval`; `Bernoulli probability exponent is invalid`; and the conversion detail from `u32::try_from(high_exclusive - low)` for an invalid uniform-i32 range. |

All of these are construction sites for the same kind. They are not runtime
device faults. Checked arithmetic in index-map, gather, scatter, and random
code is emitted into the kernel's fault flag contract; it does not construct a
host `ArithmeticOverflow` or `InvalidStageContract` while the generated kernel
is running.

## Artifact inspection errors

`inspect_hsaco`, `inspect_hsaco_bundle`, and `inspect_cubin` are public
structural boundaries. They parse bytes without a vendor runtime, then compare
the result with the requested target and [`KernelAbi`](../../src/llvm.rs#L37).

### Format failures

`artifact_format` at [`artifact.rs`](../../src/artifact.rs#L770) is the sole
constructor for `ArtifactFormat`. Its callers cover the complete parser:

* `ElfFile::parse` rejects a missing ELF magic, non-ELF64 or non-little-endian
  bytes, an unexpected section-header size, an out-of-bounds section table,
  a section-name table with no file data, invalid section ranges, and malformed
  symbol tables;
* `raw_section`, `subslice`, `read_u16`, `read_u32`, `read_u64`,
  `read_string`, `usize_from_u32`, `usize_from_u64`, and `align_four` report
  checked offset, range, conversion, NUL-termination, UTF-8, and alignment
  failures;
* HSACO inspection reports code-object-version arithmetic overflow, missing
  target ID or kernel list, non-map or missing-name kernel metadata, and
  malformed or incomplete AMDGPU notes;
* cubin inspection reports an SM flag outside `u8` (the observed CUDA layouts
  are decoded by `cubin_sm`);
* HSA ABI parsing reports missing `.args`, malformed argument maps, missing
  `.offset`, `.size`, or `.value_kind`, and missing required kernel metadata
  strings or unsigned fields; and
* the bounded MessagePack decoder reports nesting at 128 levels, unsupported
  markers, non-string or duplicate map keys, invalid UTF-8, truncated values
  or integers, and cursor/range overflow.

The exact source-to-helper path is visible in [`artifact.rs`](../../src/artifact.rs#L132-L295),
[`artifact.rs`](../../src/artifact.rs#L676-L830), and
[`artifact.rs`](../../src/artifact.rs#L891-L1028). Every `?` preserves
`ArtifactFormat` unchanged as it moves toward the public inspection function.

### Mismatch failures

`artifact_mismatch` at [`artifact.rs`](../../src/artifact.rs#L774) constructs
`ArtifactMismatch` for all semantic identity checks:

* HSACO OS ABI/machine, code-object version, target ID, missing entry or
  descriptor symbols, missing metadata kernel names, and HSA descriptor and
  argument ABI checks;
* cubin OS ABI/machine, decoded SM, missing entry or `.text.<entry>` section,
  empty or non-executable entry code, and unresolved global symbols; and
* the shared HSA/CUDA symbol audit and all target, name, offset, size, value-kind,
  address-space, alignment, workgroup, and trailing non-hidden-argument checks.

`require_indexed_symbol`, `require_symbol`, `audit_symbols`, and
`validate_hsa_abi` are the complete helper set. A parseable artifact with the
wrong identity is therefore `ArtifactMismatch`, not `ArtifactFormat`.

## Pinned native-tool builder errors

`PinnedTool::inspect` and `PinnedTool::verify` canonicalize absolute regular
files, read their bytes, and compare the SHA-256 digest. `canonical_tool`
reports `Io` for a relative path or a non-regular file and wraps canonicalize,
metadata, and read failures with `io_error`. A changed canonical path or digest
is `ArtifactMismatch` with the old and actual identity.

`ArtifactBuilder::new` verifies every configured verifier and LLVM codegen tool,
then any optional linker and `ptxas` ([`builder.rs`](../../src/builder.rs#L159)).
`build` validates the requested target, requires the lowered module's target to
match it, creates a private workspace, writes `kernel.ll`, invokes the pinned
LLVM verifier, and dispatches to the AMD or NVIDIA backend. The bundle methods
do the same checks for every input, reject an empty list as `InvalidKernel`,
reject a target mismatch as `ArtifactMismatch`, and reject duplicate entry
symbols as `InvalidEntrySymbol` before writing module files.

`BuildWorkspace::create` reports `Io` when the scratch parent cannot be
inspected, is not a real directory, is group/other accessible, cannot be made
mode `0700`, cannot create a unique directory after 1024 attempts, or any
filesystem operation fails. `write_new` and `read_regular` enforce create-new,
regular-file, non-symlink, mode `0600` input behavior and return `Io` details.
The `Drop` implementation removes the private workspace and deliberately
ignores cleanup errors, so cleanup cannot replace the original result.

`invoke` re-verifies the pinned tool, clears the environment except for fixed
locale and reproducibility settings, and executes it. Process-start failure is
`Io`. A non-success status is `ToolchainFailed` with the executable path,
numeric exit code or `signal`, and bounded lossy stderr. There is no retry.

The AMD path parses the processor and `+feature`/`-feature` suffixes, runs LLVM
object generation and one pinned ELF link, reads the regular HSACO, then calls
`inspect_hsaco`. The NVIDIA path runs LLVM NVPTX generation and one pinned
`ptxas`, reads PTX and cubin, computes the checked SM identity, then calls
`inspect_cubin`. Bundle paths verify every module and inspect every requested
entry in the one shared image. Thus `ArtifactFormat` and `ArtifactMismatch`
can be returned after successful native compiler invocations.

## Propagation and rendering outside `recipe-kernel`

The crate itself uses `?` without changing the error type. The following are
the complete cross-crate propagation boundaries found in the workspace:

| Boundary | Conversion and rendered prefix |
| --- | --- |
| Root native preparation, [`src/native_prepare.rs`](../../../src/native_prepare.rs#L34) | `ArtifactBuilder::new` and CUDA/HSA target validation use `map_err(NativePreparationError::Lowering)`. `NativePreparationError::Display` renders `native artifact builder is invalid: {error}` and its `Error::source` returns the `LoweringError`. |
| Deferred candidate realization, [`prepare/src/production.rs`](../../../prepare/src/production.rs#L398) | `lower_stage`, prebuilt `inspect_cubin`/`inspect_hsaco_bundle`, and `build_cubin_bundle`/`build_hsaco_bundle` use `map_err(NativePrepareError::Lowering)`. `NativePrepareError::Display` renders `artifact realization failed: {error}` and exposes the kernel error as `source()`. |
| Public training, [`src/training.rs`](../../../src/training.rs#L79) | `NativePreparationError` is converted to `TrainingError::Native`. The public display is `prepare current native system: {native preparation error}` and the source chain continues through both wrappers. |
| Public inference, [`src/inference.rs`](../../../src/inference.rs#L34) | `NativePreparationError` is converted to `InferenceError::Native`, rendered with the same `prepare current native system: ...` prefix, and exposed through `source()`. |
| Native executor runtime load, [`native-executor/src/error.rs`](../../../native-executor/src/error.rs#L90) | `inspect_cubin` and `inspect_hsaco_bundle` use `?`; `From<recipe_kernel::LoweringError>` creates `Error::Kernel`. Its display is `kernel artifact validation failed: {error}`. `native-executor::Error` does not override `source()`, so this wrapper terminates the standard error chain. |
| CUDA/HSA probe benchmark, [`native-probe/src/cuda.rs`](../../../native-probe/src/cuda.rs#L371) and [`native-probe/src/hsa.rs`](../../../native-probe/src/hsa.rs#L477) | Elementwise lowering, `ArtifactBuilder::new`, and `build` map to `ProbeError::Benchmark(format!("Recipe-owned GPU artifact: {error}"))`. A lowering or artifact failure aborts that measured benchmark and therefore the probe result. |

At the root command boundary, `TrainingError`, `InferenceError`, and
`NativePreparationError` are eventually formatted into the CLI's
`recipe: {error}` line. No wrapper changes the `LoweringErrorKind`, scalar
context, or message before formatting.

## Native compile consequence map

The real native path is:

```text
validated graph/stage
  -> recipe_kernel::lower_stage or lower_elementwise
  -> LoweredKernel { llvm_ir, abi, target }
  -> ArtifactBuilder verifier
  -> LLVM code generation
  -> lld (AMD) or ptxas (NVIDIA)
  -> ELF/metadata inspection
  -> runtime artifact loading and re-inspection
```

`InvalidKernel`, `InvalidStageContract`, `InvalidEntrySymbol`,
`InvalidTarget` from target/options, `InvalidWorkgroupSize`,
`UnknownScalarValue`, `UnsupportedOperation`, `ProhibitedInterface`, and the
lowering-time `ArithmeticOverflow` variants all fail before `LoweredKernel` is
returned. They cannot result in a native image.

`InvalidTarget` for a missing linker or assembler, `Io` from a build input or
workspace, and `ToolchainFailed` can occur after the verifier has run. They
stop the current pipeline and leave no accepted artifact. `ArtifactFormat` and
`ArtifactMismatch` are intentionally late checks: they validate compiler output
or a supplied prebuilt image against the target and exact ABI. A late error is
still a failed build, not a warning or a request to try another compiler.

The current workspace invokes the builder in `BuildPhase::Realize` from
deferred production realization and both native probe benchmarks. The
`BuildPhase::Offline` enum value is recorded in provenance when a caller uses
it, but the builder's error taxonomy and compile sequence do not branch into a
second implementation.

## Caller guidance

* Match `LoweringErrorKind`, not a `Display` prefix. The enum is
  non-exhaustive.
* Preserve `scalar` when presenting scalar failures. It is present only for
  emitter lookup, output-type, and unsupported-op failures that identify a
  scalar result.
* Treat `ArtifactFormat` as malformed bytes or metadata and
  `ArtifactMismatch` as a validly decoded artifact with the wrong identity.
* Treat `ToolchainFailed` and `Io` as concrete native-tool or filesystem
  failures. The message may contain bounded tool stderr and a path, so it is
  for diagnostics rather than a stable machine protocol.
* Do not retry with another target, tool, compiler, or artifact. The callers
  propagate the original error so the measured target, pinned toolchain, and
  immutable ABI remain authoritative.
