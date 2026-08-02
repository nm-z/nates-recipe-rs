# `inspect_add`

## Intent

`kernel/examples/inspect_add.rs` is a small structural inspection client for
one known elementwise f32-add kernel. It rebuilds the kernel declaration in
Recipe's public core types, lowers that declaration for the requested backend,
uses the resulting ABI as the independent expectation, and then inspects an
already-built HSACO or cubin. It does not compile the artifact, launch it, or
check numerical output. A successful run proves that the bytes have the target
format and the expected entry-point contract.

The example is deliberately paired with `lower_add.rs`. `lower_add` writes the
same declaration as LLVM IR. A toolchain, or `ArtifactBuilder`, can turn that
IR into a native image; `inspect_add` checks the resulting image at the final
artifact boundary.

## Invocation

```text
cargo run -p recipe-kernel --example inspect_add -- gfx1101 path/to/kernel.hsaco
cargo run -p recipe-kernel --example inspect_add -- sm_86 path/to/kernel.cubin
```

The program consumes exactly two positional arguments after the executable:

1. `gfx1101` selects `KernelTarget::Amd` with target ID `gfx1101` and AMDGPU
   code-object version `6`.
2. `sm_86` selects `KernelTarget::Nvidia` with SM major `8`, minor `6`, and
   PTX ISA `75`.
3. The second argument is read as raw bytes. Its extension is not examined;
   the first argument selects the inspector.

Missing arguments or an extra argument return the exact usage error
`usage: inspect_add <gfx1101|sm_86> <artifact>`. Any target other than the two
literal forms returns `example accepts gfx1101 or sm_86`.

## Kernel declaration

The declaration is assembled inline before the artifact is read. Every
constructor is fallible where the core contract requires it, and `main` uses
`?`, so an invalid core object stops the run before inspection.

| Field | Value | Meaning |
| --- | --- | --- |
| `KernelTemplateId` | `1` | Stable template identity for this example |
| index space | one dimension of `1_048_576` elements | One logical lane per f32 result |
| input 1 | ID `1`, `DType::F32`, contiguous | Read buffer, four bytes per element |
| input 2 | ID `2`, `DType::F32`, contiguous | Read buffer, four bytes per element |
| output | ID `1`, `DType::F32`, contiguous | Writable result buffer |
| scalar inputs | value IDs `1` and `2`, both f32 | Values loaded from the two inputs |
| scalar instruction | result ID `3`, `ScalarOpcode::Add`, f32 | `result = left + right` |
| scalar outputs | value ID `3` | The value stored to the output |
| constants | empty | No embedded scalar constants |
| alias `(input 1, output)` | `MayAliasExact` | The output may be the exact first input buffer |
| alias `(input 2, output)` | `Forbidden` | The output may not overlap the second input |

`ElementCount::new(1_048_576)` and `IndexSpace::new` establish a nonzero,
overflow-checked one-dimensional space. `StaticBufferAccess::contiguous`
creates rank-one stride `[1]`, offset `0`, and `4_194_304` bytes of backing
storage for each f32 buffer. `KernelTemplate::validate` then checks the scalar
program, access mappings, input/output counts and types, and the complete
two-entry alias matrix. The `Add` signature is valid because both operands and
the result are f32.

## Target and lowering path

The target is parsed before lowering:

```text
gfx1101 -> KernelTarget::Amd(AmdTarget { target_id: "gfx1101", code_object_version: 6 })
sm_86   -> KernelTarget::Nvidia(NvidiaTarget { sm_major: 8, sm_minor: 6, ptx_isa: 75 })
```

`AmdTarget::validate` requires a nonempty token beginning with `gfx` and a
nonzero code-object version. `NvidiaTarget::validate` accepts SM majors 3
through 12, minor values through 9, and PTX ISAs from 32 through 90. The two
literal targets satisfy those checks.

The call is:

```rust
let lowered = lower_elementwise(&template, &target, &LoweringOptions {
    entry_symbol: "recipe_add_f32".to_owned(),
    workgroup_lanes: 256,
})?;
```

`lower_elementwise` performs, in order, template validation, target validation,
and lowering-option validation. The entry symbol must be an ASCII LLVM
identifier, and the workgroup must contain 1 through 1024 lanes. It emits a
target-specific entry point with one global pointer for each input and output,
followed by a 64-bit element count. Each lane computes its linear global ID,
branches around out-of-bounds lanes, loads both inputs, performs a constrained
round-to-nearest f32 add, canonicalizes a NaN result, and stores the result.

For AMD the generated module uses the AMDGPU work-item and workgroup intrinsics
and the `amdgpu_kernel` calling convention. For NVIDIA it uses the NVPTX thread
and CTA intrinsics and the `ptx_kernel` calling convention. The module has the
matching AMDGPU or NVPTX target triple. Only LLVM declarations are emitted for
this program: the backend index intrinsics and the constrained fadd intrinsic.

The resulting `LoweredKernel` contains the LLVM text, target, FLOP work and
`KernelAbi`. This program has one f32 instruction per element, so
`work == 1_048_576` FLOPs. Its ABI is deterministic:

```text
arguments = [
    Buffer { access: Read,  dtype: F32, alignment: 4 }, // input_0
    Buffer { access: Read,  dtype: F32, alignment: 4 }, // input_1
    Buffer { access: Write, dtype: F32, alignment: 4 }, // output_0
    ElementCount,
]
argument_bytes     = 32
argument_alignment = 8
elements           = 1_048_576
workgroup_lanes    = 256
entry_symbol       = "recipe_add_f32"
```

No `Require`, checked integer operation, gather, scatter or other faulting
operation exists in this program, so there is no `FaultFlag` argument.

Before returning, `lower_elementwise` calls the public `audit_llvm_ir` logic on
the assembled module. That audit scans `declare` lines and reports every
external declaration whose symbol does not begin with `llvm.`. A finding would
become `LoweringErrorKind::ProhibitedInterface`; this add module has no
findings. The audit is an IR-interface check, not a native artifact or runtime
correctness check.

## Artifact read and dispatch

Only after successful lowering does the example execute `fs::read` on the
artifact path. The bytes are then dispatched by the already-parsed target:

```text
KernelTarget::Amd(target)    -> inspect_hsaco(&bytes, &target.target_id,
                                               target.code_object_version,
                                               &lowered.abi)
KernelTarget::Nvidia(target) -> inspect_cubin(&bytes, 86,
                                               &lowered.abi.entry_symbol)
```

`inspect_hsaco` is the one-entry wrapper around
`inspect_hsaco_bundle`. It parses the ELF and AMDGPU metadata once and returns
one `InspectedHsaco`. `inspect_cubin` parses the CUDA ELF and returns one
`InspectedCubin`. Both functions are pure over the supplied byte slice and the
expected values. They do not load or execute the image.

## What HSACO inspection proves

The AMD path rejects malformed or unrelated bytes before returning. The
artifact parser requires a bounded little-endian ELF64 file with a valid
section table and string tables. The inspector then requires all of the
following:

- AMDGPU-HSA ELF OS ABI and AMDGPU machine ID;
- code-object version equal to `6` (the ELF ABI version plus two);
- no unresolved global or weak symbols;
- an AMDGPU metadata note containing a target ID and a kernel list;
- a target ID whose processor and feature components match `gfx1101` (the
  canonical matcher accepts a full triple ending in `--gfx1101`);
- one unique metadata kernel named `recipe_add_f32`;
- a defined global or weak function symbol `recipe_add_f32` and a matching
  defined object symbol `recipe_add_f32.kd`;
- a kernel descriptor whose symbol is `recipe_add_f32.kd`, kernarg alignment is
  exactly the ABI's required alignment of `8`, maximum workgroup size is at
  least `256`, and kernarg segment size is at least `32` bytes;
- the first four explicit metadata arguments in the ABI order and at eight-byte
  offsets: two global buffers, one global output buffer, and one by-value
  `element_count`; each is eight bytes and none is hidden;
- any remaining metadata arguments are hidden ABI arguments only.

When optional metadata names are present, input buffer names must start with
`input_`, the output name must start with `output_`, and the element count name
must be `element_count`. The returned `InspectedHsaco` carries the SHA-256
`ArtifactDigest`, ELF ABI version, code-object version, ELF flags, actual target
ID and the complete decoded `HsaKernelMetadata`.

## What cubin inspection proves

The NVIDIA path requires a bounded little-endian ELF64 file with one of the
accepted CUDA OS ABI tags and the CUDA machine ID. It decodes the SM identity
from the architecture-specific ELF flags and requires exactly `sm_86`. It also
requires no unresolved global or weak symbols, a defined global or weak
function symbol named `recipe_add_f32`, and a nonempty executable section named
`.text.recipe_add_f32` whose function symbol has nonzero size. The returned
`InspectedCubin` carries the SHA-256 digest, ELF ABI version, ELF flags, SM
number, entry symbol and executable text byte count.

Unlike the AMD metadata path, cubin inspection has no argument metadata to
decode. The expected ABI still controls the entry-symbol value supplied by the
example, but `inspect_cubin` cannot prove per-argument metadata from the cubin
alone.

## Observable output

On success the example prints exactly one pretty `Debug` value to stdout:

```text
InspectedHsaco { ... }
```

or

```text
InspectedCubin { ... }
```

`{inspected:#?}` expands all public inspection fields. `ArtifactDigest` is
printed as its tuple of 32 raw bytes, not as hexadecimal. There is no separate
success marker, file output, log file, device launch or result buffer. A
successful command exits zero after printing the inspection.

For the locally generated reference artifacts, the AMD result reports target
ID `amdgcn-amd-amdhsa--gfx1101`, code-object version `6`, and the four explicit
arguments followed by hidden AMD dispatch arguments. The NVIDIA result reports
`sm: 86`, entry symbol `recipe_add_f32`, and a nonzero executable text size.

## Failure behavior

`main` returns `Result<(), Box<dyn std::error::Error>>`, so every `?` failure
ends the process with a nonzero exit status and the Rust error display on
stderr. The relevant boundaries are:

- CLI shape: the usage errors above;
- target selection: `example accepts gfx1101 or sm_86`;
- core construction: zero or overflowing element counts, invalid index spaces,
  invalid buffer mappings, scalar type/arity errors, incomplete aliases and
  other `KernelTemplate::validate` failures;
- lowering: `InvalidTarget`, `InvalidEntrySymbol`, `InvalidWorkgroupSize`,
  `InvalidKernel`, unsupported scalar operations, unknown values, arithmetic
  overflow, or `ProhibitedInterface` from the generated-IR audit;
- file access: the underlying `fs::read` I/O error, before either artifact
  inspector runs;
- artifact format: non-ELF, non-64-bit, non-little-endian or structurally
  truncated images, malformed section/symbol tables, malformed AMDGPU notes or
  malformed MessagePack metadata;
- artifact mismatch: the wrong backend image, target or code-object version,
  unresolved symbols, missing entry or descriptor symbols, wrong AMD metadata
  ABI, unexpected non-hidden trailing arguments, missing executable cubin code,
  or an entry symbol/section mismatch.

For example, passing the valid cubin to the AMD form reports an AMDGPU-HSA ELF
OS-ABI or machine mismatch. Passing the valid HSACO to the NVIDIA form reports
the corresponding CUDA ELF mismatch. Passing an empty file reports
`ArtifactFormat: artifact is not an ELF file`.

## Relationship to builder and downstream execution

The example does not instantiate `ArtifactBuilder`, select `BuildPhase`, pin a
tool, create a scratch workspace, or record build provenance. Those are the
compilation APIs used by production preparation. `ArtifactBuilder::build`
verifies the pinned tools, verifies the lowered LLVM module, compiles for the
requested target, reads the generated HSACO or cubin, and calls the same
`inspect_hsaco` or `inspect_cubin` function before returning `BuiltArtifact`.
The bundle methods perform the analogous inspection for every entry in one
multi-entry image. The example is therefore a manual post-build inspection
client, not an alternative compiler path.

The expected ABI is regenerated from the source template instead of inferred
from the artifact. That makes the inspector a fail-closed boundary for target,
entry identity and (for HSACO) explicit argument metadata. It does not prove
that the native instructions add the intended values, that a launch succeeds on
hardware, that buffers contain valid data, or that the artifact has the exact
LLVM text used to derive the ABI. Those properties belong to compilation,
runtime execution and independent acceptance measurements.
