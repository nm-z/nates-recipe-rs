<!--
This document describes kernel/src/builder.rs.  Source line references below
refer to the current implementation and are intentionally kept close to the
contract they explain.
-->

# Artifact builder

`kernel/src/builder.rs` is Recipe's offline native-artifact boundary.  It does
not invent kernel semantics, schedule work, allocate device memory, load a
module, or execute a dispatch.  It accepts a completely lowered, target-
specific `LoweredKernel`, invokes only explicitly pinned compiler tools in a
private temporary workspace, structurally inspects the emitted ELF, and
returns bytes plus an exact build record.  The same boundary also packages a
caller-ordered set of lowered stages into one multi-entry HSACO or cubin; the
production caller supplies deterministic artifact-ID order and the builder
preserves it in compiler arguments, inspections, and provenance.

The implementation has one important separation:

* `recipe-primitives` fixes backend-neutral buffers, stage order, dispatch
  geometry, synchronization, atomics, faults, and resource bounds in a
  `LoweredProgram`.
* `kernel/src/stage.rs` realizes one immutable stage contract as target LLVM
  IR and returns a `LoweredKernel` with its launch ABI.
* `builder.rs` realizes that already-lowered module into a native image and
  checks that the image's ELF, entry point, target, and ABI agree with the
  requested module.
* `prepare/src/production.rs` owns candidate-scoped realization and hands the
  resulting immutable bytes to the native executor.  The builder is not kept
  in the finalized run lifecycle.

There are no compiler entry points for `Finalize`, `init`, `loop`, or `exit`.
Compilation is an offline or pre-final realization action, represented by
`BuildPhase` (lines 20-25), and not a model task.

This is the concrete kernel-side implementation of the system contract's
`draft -> realize -> finalize` boundary: compilation and native-image loading
are ahead-of-run work, every resulting identity is tied to its toolchain, and
the finalized lifecycle receives inspected AMDGPU or NVIDIA images rather than
recompiling or selecting a near-match artifact.

## End-to-end data flow

```text
recipe_language::PrimitiveKernel + tensors + measured lowering limits
                 |
                 v
recipe_primitives::lower
  - validates the primitive and hardware
  - creates ProgramBuffer and ProgramStage values
  - records access, dependencies, faults, atomics, geometry, resources
  - computes LoweredProgram::digest and validates the complete program
                 |
                 v
planner::lower_program_invocation
  - selects a stage-scoped identity
  - creates ArtifactBuildRecipe for every stage
  - copies ordered bindings, affine views, dispatch, work, faults, resources
  - hashes the immutable build contract
                 |
                 v
prepare::lower_deferred_stage
  - finds the exact LoweredProgram by source kernel and program digest
  - calls recipe_kernel::lower_stage with target and exact workgroup size
                 |
                 v
kernel::stage::lower_stage
  - revalidates program, recipe, digest, stage ordinal, identities, bindings,
    geometry, work, resource envelope, and fault binding
  - lowers ScalarMap through lower_elementwise, or emits an owned stage
  - audits LLVM IR and returns LoweredKernel { llvm_ir, abi, work, target }
                 |
                 v
ArtifactBuilder::{build,build_hsaco_bundle,build_cubin_bundle}
  - verifies pinned tools again
  - writes LLVM input in a private scratch workspace
  - runs verifier, code generation, linker or ptxas
  - reads only regular output files
  - inspects the native ELF against every requested KernelAbi
                 |
                 v
immutable native bytes + Inspected{Hsaco,Cubin} + provenance
                 |
                 v
prepare::NativeArtifact -> native executor load/reserve/warm/observe
```

The builder never receives a `PrimitiveKernel`, `ScalarProgram`, or
`ArtifactBuildRecipe`.  Those objects are consumed upstream.  The builder's
direct semantic input is `LoweredKernel`; the stage path that produced it is
documented below because it explains the ABI and target invariants that the
builder checks.

## Public data types

### Build phase

`BuildPhase` has exactly two values, `Offline` and `Realize` (20-25).  The
phase is copied into every single-image or bundle provenance record.  All
current repository callers use `BuildPhase::Realize`; there is no current
`BuildPhase::Offline` call site.  `Offline` remains the explicit value for an
approved offline build path, rather than an implicit fallback.  The builder
does not branch on the phase and cannot be used to compile a lifecycle phase.
Callers, notably `prepare::DeferredArtifactCompiler`, check that a production
bundle reports `Realize` before accepting it.

### Pinned tools and toolchain

`PinnedTool` stores a canonical absolute tool-file path and an
`ArtifactDigest` (28-31).  `PinnedTool::inspect` (33-44) does the initial
identity capture:

1. `canonical_tool` rejects a relative path, canonicalizes the path, requires
   a regular file, and maps filesystem failures to `LoweringErrorKind::Io`.
2. The entire canonical file is read and SHA-256 hashed with
   `ArtifactDigest::of`.

`PinnedTool::verify` (46-72) repeats the canonicalization and read at every
   use.  A path that now resolves differently, or bytes whose digest differs
   from the pinned digest, produces `LoweringErrorKind::ArtifactMismatch`.
   This makes a successful build conditional on both executable identity and
   executable contents remaining unchanged.  `ArtifactBuilder::new` verifies
   the required verifier and LLVM code generator and verifies either optional
   linker or assembler when present (159-170).  Each build path verifies the
   backend-specific tool again, and `invoke` verifies the selected tool before
   every process launch.

`OfflineToolchain` (75-81) contains:

* `verifier`: the LLVM verifier used with `-passes=verify`;
* `llvm_codegen`: the LLVM code generator used for AMD object or NVIDIA PTX;
* `elf_linker`: optional at configuration time, required for an AMD build;
* `ptx_assembler`: optional at configuration time, required for an NVIDIA
  build.

The optional fields allow one configuration to describe either backend.  They
do not provide a fallback implementation.  Calling the wrong backend without
its tool returns `InvalidTarget`.

### Invocation and provenance records

`ToolInvocation` (83-87) stores the canonical program path and the exact
argument vector passed to one tool.  Scratch paths in recorded arguments are
rewritten to the stable `@scratch` prefix by `normalize_scratch_argument`
(668-674), so the record does not expose a process-specific temporary path.

`BuildProvenance` (89-95) is the record for one module.  It contains the
phase, SHA-256 digest of the input LLVM IR, unique invoked tool paths paired
with their pinned digests, and invocations in execution order.

`HsacoBundleProvenance` (97-108) and `CubinBundleProvenance` (118-129) use a
vector of LLVM-IR digests.  The vector order is the exact input order and is
the same order as `BuiltHsacoBundle::inspections` or
`BuiltCubinBundle::inspections`.  Each bundle also records unique tool
identities and all invocations in order.

`invoked_tools` (589-614) walks invocations in first-use order, de-duplicates
paths, and looks up each path in the four pinned tool slots.  The
`expect("every recorded invocation uses a pinned tool")` is an internal
invariant: `invoke` can only be called with one of those slots.

The `tools` vector describes tools that actually appear in the invocation
list, not every verified pin.  In particular, AMD bundle construction verifies
`llvm_codegen` but lets the full-LTO linker consume the bitcode, so that pin is
not an invoked-tool entry for that bundle.  The configured toolchain identity
outside this module still includes every backend-required pin.

The builder provenance is an in-memory compiler record.  Preparation separately
stores the planner's target-independent `ArtifactBuildProvenance` in the core
`ArtifactIdentity`, while the resulting `RuntimeImage` supplies the digest of
the actual native bytes.  The two records answer different questions: which
compiler inputs and invocations produced this image, and which immutable Draft
stage recipe the image realizes.

### Returned artifacts

`BuiltArtifact` (139-152) is the single-module result:

* `Hsaco` contains native bytes, one `InspectedHsaco`, and
  `BuildProvenance`.
* `Cubin` contains the generated PTX bytes as well as native cubin bytes, one
  `InspectedCubin`, and `BuildProvenance`.

`BuiltHsacoBundle` (110-116) and `BuiltCubinBundle` (131-137) contain one
  shared native image, an inspection for every requested entry point, and the
  corresponding bundle provenance.  A bundle is one code object, not a list of
  independently linked images.

## `ArtifactBuilder` lifecycle

`ArtifactBuilder` stores only the `OfflineToolchain` (154-157).  It is cheap to
clone and is intentionally a compiler capability, not a runtime session.  The
constructor performs an early identity check, but each public build method
rechecks tools after validating its inputs.  This closes the time-of-check to
time-of-use window between configuration and actual invocation.

All public builders receive an explicit `scratch_parent`.  It must already be
an owner-only, real directory.  No default temporary directory, environment
lookup, or alternate output location is chosen by this module.

## Single-module build

`ArtifactBuilder::build` (172-215) compiles one `LoweredKernel`.

### Preconditions

1. `target.validate()` checks the target-specific identity.
2. `lowered.target` must equal the requested `KernelTarget`, otherwise the
   build fails with `ArtifactMismatch`.  LLVM address spaces, intrinsics,
   calling convention, and floating-point attributes are target-specific, so
   a module cannot be retargeted by changing the request.
3. The verifier and LLVM code generator are reverified.
4. `BuildWorkspace::create` allocates a private workspace below the explicit
   parent.

The module is written to `kernel.ll` with `create_new`, mode `0600`, and a
successful `sync_all` (195-198, 731-742).  The verifier is invoked first with:

```text
verifier -passes=verify -disable-output @scratch/kernel.ll
```

The verifier invocation is recorded before target-specific compilation.

### AMD path

`build_hsaco` (454-519) requires and verifies the pinned ELF linker.  It parses
`AmdTarget::target_id` with `amd_processor_and_features` (676-711): the first
colon-separated component is a nonempty processor such as `gfx1101`; every
remaining component must end in `+` or `-`, have a nonempty ASCII-alphanumeric
or underscore name, and is normalized into a comma-separated feature list.

The exact invocations are (the `@scratch/...` notation below abbreviates the
absolute workspace path; the real process receives that absolute path):

```text
llvm_codegen -filetype=obj -march=amdgcn -mcpu=<processor>
             --amdhsa-code-object-version=<version>
             [-mattr=<features>] @scratch/kernel.ll -o @scratch/kernel.o
elf_linker -shared --no-undefined @scratch/kernel.o -o @scratch/kernel.hsaco
```

The output must be a non-symlink regular file.  `inspect_hsaco` then parses the
ELF and AMDGPU metadata and matches the expected target ID, code-object
version, entry symbol, descriptor, explicit argument order and sizes,
alignment, global-buffer/fault/by-value argument kinds, at-least-required
workgroup limit, and
absence of unresolved global symbols.  A successful image becomes
`BuiltArtifact::Hsaco`; the image digest is the digest returned by the
inspection and the provenance IR digest is the original `kernel.ll` bytes.

### NVIDIA path

`build_cubin` (521-573) requires and verifies the pinned `ptx_assembler`.  The
target's `architecture()` is `sm_<major><minor>` and
`llvm_ptx_feature()` is `+ptx<isa>`.  It runs (the `@scratch/...` notation
abbreviates the absolute workspace path in these examples):

```text
llvm_codegen -march=nvptx64 -mcpu=<sm> -mattr=+ptx<isa>
             @scratch/kernel.ll -o @scratch/kernel.ptx
ptx_assembler -arch=<sm> @scratch/kernel.ptx -o @scratch/kernel.cubin
```

Both PTX and cubin are read as regular non-symlink files.  `nvidia_sm` computes
`sm_major * 10 + sm_minor` with checked arithmetic (617-626), and
`inspect_cubin` requires an accepted CUDA ELF OS-ABI, CUDA machine, exact SM,
the requested defined global function, and a nonempty executable
`.text.<entry>` section.  The returned `BuiltArtifact::Cubin` keeps the PTX
bytes because PTX is the direct code-generation product and is useful for the
build record, while the cubin bytes are the loadable artifact.

## AMD multi-entry HSACO bundle

`build_hsaco_bundle` (217-331) is the multi-stage AMD path.  It is used when
several deferred stages target one measured AMD target.

### Input checks

* `AmdTarget::validate` runs first.
* The slice must be nonempty, otherwise `InvalidKernel`.
* Every `LoweredKernel::target` must equal `KernelTarget::Amd(target)`, or the
  operation returns `ArtifactMismatch`.
* A `BTreeSet` rejects a repeated `abi.entry_symbol` with
  `InvalidEntrySymbol`.  The caller must already provide deterministic input
  order; the builder preserves that order and does not sort it.
* The verifier, LLVM code generator, and pinned ELF linker are verified.  A
  missing linker is `InvalidTarget`.

### Compilation and inspection

For each input at index `i`, the builder creates `kernel-i.ll` and
`kernel-i.bc`.  It invokes the verifier with `-passes=verify kernel-i.ll -o
kernel-i.bc`; this both verifies and serializes the module to bitcode.  The
single linker invocation is (the `@scratch/...` notation below abbreviates the
absolute workspace path):

```text
elf_linker -shared --no-undefined --lto-O0
            --plugin-opt=-mcpu=<processor>
            --plugin-opt=-amdhsa-code-object-version=<version>
            [--plugin-opt=-mattr=<features>]
            @scratch/kernel-0.bc ... @scratch/kernel-n.bc
            -o @scratch/bundle.hsaco
```

Full LTO is deliberate.  Ordinary AMDGPU relocatable linking on the supported
LLVM toolchains does not merge each object's HSA metadata notes, while one
full-LTO link produces one metadata-bearing code object.

The output is read as a regular file and passed to `inspect_hsaco_bundle` with
the expected target, code-object version, and every input ABI.  Inspection is
performed once over the shared ELF and metadata, then returned in requested
input order.  The bundle's LLVM digest vector is computed from the original
`.llvm_ir` strings, not from compiler-produced bitcode.

## NVIDIA multi-entry cubin bundle

`build_cubin_bundle` (333-452) follows the same deterministic shape.

### Input checks

It validates the `NvidiaTarget`, rejects an empty slice with `InvalidKernel`,
requires exact target equality for every lowered module, and rejects duplicate
entry symbols with `InvalidEntrySymbol`.  The verifier, LLVM code generator,
and pinned `ptxas` are verified; a missing assembler is `InvalidTarget`.

### Compilation and inspection

For each module `i`, it writes `kernel-i.ll`, verifies it to `kernel-i.bc`, and
code-generates a PTX translation unit:

```text
llvm_codegen -march=nvptx64 -mcpu=<sm> -mattr=+ptx<isa>
             @scratch/kernel-i.bc -o @scratch/kernel-i.ptx
```

One assembler invocation packages all PTX units in order (`@scratch/...` is
the absolute workspace path in the real argument vector):

```text
ptx_assembler -arch=<sm> @scratch/kernel-0.ptx ... @scratch/kernel-n.ptx
                -o @scratch/bundle.cubin
```

The output is read once.  For every lowered ABI, `inspect_cubin` checks the
same shared cubin against the expected SM and entry symbol.  The inspection
vector and LLVM digest vector retain the same input order, while tool
invocations retain the actual verifier/codegen/assembler order.

## Stage and scalar origin of the builder input

The builder is intentionally downstream of the semantic and primitive layers,
but its ABI checks are only meaningful when the origin of a `LoweredKernel` is
understood.

### Scalar program and elementwise stage

`recipe_core::ScalarProgram` is a typed, acyclic program of `ScalarInput`,
`ScalarConstant`, `ScalarInstruction`, and output `ScalarValueId` values.  Its
validation checks duplicate definitions, use-before-definition, opcode arity,
operand types, result types, and the presence of at least one output.  The
domain is exactly f32 and int32.  Arithmetic opcodes report deterministic FLOP
counts, while representation changes, predicates, bit operations, and
validation predicates report zero FLOPs.  Checked int32 divide/remainder,
negate, absolute, and `Require` mark that a preallocated fault flag is needed.

`recipe_primitives::lower` receives a validated `PrimitiveKernel`, tensor
index, and measured `LoweringHardware` (primitives/src/lower.rs:51-95).  It
validates hardware limits and the primitive, interns external tensors as
`ProgramBuffer` values, dispatches by `PrimitiveKind`, and appends one or more
`ProgramStage` values.  A `ProgramStage` carries:

* `StageKind`, including `ScalarMap { template }`, fill/copy, reductions,
  scans, contraction, gather/scatter, histogram, sort, index-map, and Philox;
* ordered buffer bindings and `AccessMode` values;
* dispatch geometry and stage dependencies;
* synchronization and atomic contracts;
* an optional checked-fault contract; and
* per-stage resource bounds.

Primitive validation fixes the algorithm contracts before any target lowering:
tree lanes and steps are canonical powers-of-two, contraction tiles equal the
chosen workgroup width and use a canonical accumulation order, rejecting gather
and scatter require a fault contract, histogram accumulation requires a checked
bin fault, sort networks use the least power-of-two padding and stable total
ordering, and Philox stages retain the exact ten-round constants, counter words,
and key/run-ID folding.  It also derives exact resource and synchronization
bounds for each kind.  These are not builder policy choices; a failure here is
an invalid lowered program before native compilation is possible.

For elementwise lowering (primitives/src/lower.rs:489-635), the primitive
scalar program is placed into a `KernelTemplate` with an `IndexSpace`, static
input/output affine views, dtypes, and alias rules.  The stage's bindings,
fault buffer, operation count, private scalar-slot bound, and workgroup size
are computed from that template.  `ProgramBuilder::finish` aggregates resource
bounds, creates `LoweredProgram`, computes its canonical digest, and revalidates
the complete structure.

### Planner build recipe

The planner walks every lowered stage (planner/src/planner.rs:1327-1479).  It
creates a stage-scoped `KernelTemplateId` and `ArtifactId`, copies the stage's
ordered binding views into `ArtifactBuildBinding`, copies exact geometry,
floating/integer/atomic work, fault value, and the kernel resource envelope,
then computes `provenance.contract_digest` over all those fields.  The recipe is
target-independent by design: it does not contain target, format, entry symbol,
or toolchain identity.

Draft validation requires every artifact ID to resolve to exactly one realized
identity or one deferred build, rejects duplicate stage identities, validates
all binding value references, and validates the dispatch ceiling, fault binding,
and resource envelope (core/src/plan.rs:349-405).  A deferred build is therefore
an immutable request for later Realize, not permission to compile arbitrary IR.
If a catalog already contains the reserved artifact ID, planning accepts it
only when its stage identity, planner build provenance, resource envelope, and
discovered target match exactly; otherwise planning fails instead of silently
replacing it.  With no catalog entry, the recipe is carried into deferred
realization.

### `lower_stage` re-establishes the contract

`kernel/src/stage.rs::lower_stage` (81-102) is the immediate caller-side
boundary before `ArtifactBuilder`:

1. `LoweredProgram::validate` and `ArtifactBuildRecipe::validate` must pass.
2. The recipe's program digest and source kernel must match the full program.
3. The stage ordinal must identify an actual stage.
4. The stage-scoped template identity and reserved artifact ID must be exactly
   the canonical identity.
5. The independently recomputed contract digest must equal the planner digest.
6. Dispatch geometry, requested workgroup size, work bounds, resource envelope,
   ordered binding dtypes/access/views, and fault value must match the stage.

For `StageKind::ScalarMap`, `lower_stage` calls `lower_elementwise`, rewrites
the scalar emitter's monotonic fault publication into the stage's checked
`atomicrmw xchg` with the prescribed fault code, and validates the realized
ABI.  For every other `StageKind`, `StageSignature` converts ordered build
bindings into read/write/fault pointer arguments (a read-write binding gets its
ordered read and write pointer positions) and appends dynamic `run_id` or
`loop_iteration` only for Philox and iteration-dependent index maps.  The
element-count value is always the final explicit argument.  The owned-stage
emitter then creates direct target LLVM IR, audits it, constructs a `KernelAbi`,
and rechecks elements, workgroup size, work, and fault-argument count.  The
returned `LoweredKernel` is the exact target-specific input the builder accepts.

The resulting identity chain is therefore `source_kernel -> stage ordinal ->
stage-scoped kernel template/artifact ID -> recipe_stage_<artifact> entry
symbol -> KernelAbi -> inspected ELF entry`.  A bundle may share bytes across
these entries, but it never shares or infers their ABI contracts.

## Callers

### Production deferred realization

`prepare/src/native_prepare.rs:573-609` constructs one `ArtifactBuilder` from
the configured `OfflineToolchain`, validates each measured device's target and
runtime policy, and places clones in `TargetBuildSpec`.  The builder is grouped
by measured target identity, not by source primitive identity.

`prepare/src/production.rs::DeferredArtifactCompiler::materialize`
(254-377) resolves already-realized catalog entries, groups deferred
`ArtifactBuildRecipe` values by target, sorts each group by `ArtifactId`, and
calls exactly one bundle builder for that target.  `lower_deferred_stage`
(523-547) recovers each exact `LoweredProgram` by source kernel plus digest and
derives the entry symbol `recipe_stage_<artifact id>`.  It then calls
`recipe_kernel::lower_stage` with the target and immutable workgroup size.

If a prebuilt bundle was supplied, realization still lowers every current stage
to recover each ABI and runs `inspect_cubin` or `inspect_hsaco_bundle`; it skips
compiler invocation but does not skip validation.  If the builder compiles a
bundle, the caller requires `provenance.phase == BuildPhase::Realize`, requires
one inspection per lowered stage, and checks each inspection's entry symbol
against the corresponding ABI (398-520).  One `RuntimeImage` is then shared by
the per-stage `NativeArtifact` values.  The native artifact identity stores the
image digest, target ABI, toolchain identity, entry symbol, stage resources, and
the planner build provenance (549-575).

The resulting artifacts are validated and handed to the native executor's
candidate-scoped loader.  The compiler, loader, allocator, and mutable driver
are not retained after preparation.

`NativeArtifact::new` then checks the image digest, artifact ID, entry symbol,
format/target ABI, and backend-specific runtime identity.  The catalog sorts
these immutable pairs by artifact ID and rejects duplicates before any driver
can see them.  A shared bundle therefore remains one byte image, while each
stage receives its own inspected `KernelAbi` and stage-scoped identity.

### Native probe benchmarks

`native-probe/src/cuda.rs:345-400` and `native-probe/src/hsa.rs:455-503` create a
real `KernelTemplate` containing the benchmark's FMA chain, lower it with the
measured target and workgroup size, construct an `ArtifactBuilder`, and call
`build(BuildPhase::Realize, ...)`.  CUDA requires the returned variant to be
`BuiltArtifact::Cubin`, checks the entry symbol, then loads and launches the
cubin.  HSA requires `BuiltArtifact::Hsaco`, loads it through ROCr, and checks
the inspected kernarg metadata against the runtime kernel metadata before
launch.  These are direct end-to-end users of the single-module API, not test
or mock paths.

### Tool configuration and identity

The CLI creates required `opt` and `llc` pins and optional `lld` and `ptxas`
pins, all through `PinnedTool::inspect` (src/cli.rs:1867-1895).  Active native
receipts reopen each path and reject any path or digest change (src/cli.rs:
1249-1279).  `native-probe/src/identity.rs:88-142` includes the required
backend tool paths and digests in the measured `ToolchainIdentity`.  Thus the
bytes produced by the builder and the toolchain recorded in the candidate
identity are tied to the same executable files.

## Determinism, isolation, and I/O behavior

### Tool process environment

`invoke` (629-666) clears the inherited environment and sets only:

```text
LC_ALL=C
SOURCE_DATE_EPOCH=0
```

It runs the canonical executable in the workspace current directory.  The
complete argument vector is captured.  A nonzero exit returns
`LoweringErrorKind::ToolchainFailed` with the numeric exit code or `signal`
and at most 16 KiB of lossy UTF-8 stderr (`ERROR_OUTPUT_LIMIT`, 17-18,
762-765).  The builder does not retry, switch tools, or conceal a compiler
failure.

### Scratch workspace

`BuildWorkspace::create` (767-808) checks the parent with
`symlink_metadata`: it must be a real directory, not a symlink, with no group
or other permission bits (`mode & 0o077 == 0`).  It tries at most 1024 names of
the form `recipe-build-<process id>-<global nonce>`, uses atomic `create_dir`,
and sets each workspace to mode `0700`.  Existing names are retried; every
other creation error is returned as `Io`, and exhausting the attempts returns
`Io`.

`BuildWorkspace` removes its directory recursively in `Drop` (811-813), with
cleanup errors intentionally ignored.  Build inputs and outputs are therefore
ephemeral.  `write_new` uses `create_new`, mode `0600`, writes all bytes, and
syncs the file.  `read_regular` rejects a symlink even if it resolves to a
regular file, then reads the bytes.  These checks prevent an output path from
being silently replaced by an unexpected file or link.

## Invariants and failure classification

The builder returns `Result<_, LoweringError>` and never produces a partially
trusted artifact.  The following failures are deliberate and observable:

| Condition | Result | Where enforced |
| --- | --- | --- |
| Relative, missing, non-file, or changed pinned tool | `Io` or `ArtifactMismatch` | `canonical_tool`, `PinnedTool::{inspect,verify}` |
| Invalid AMD or NVIDIA target | `InvalidTarget` | target validation and `amd_processor_and_features` |
| Lowered target differs from requested target | `ArtifactMismatch` | `ArtifactBuilder::build` and both bundle entry checks |
| Empty bundle | `InvalidKernel` | `build_hsaco_bundle`, `build_cubin_bundle` |
| Repeated bundle entry symbol | `InvalidEntrySymbol` | `BTreeSet` preflight in both bundle methods |
| Missing backend linker or assembler | `InvalidTarget` | `build_hsaco{,_bundle}`, `build_cubin{,_bundle}` |
| Verifier, code generator, linker, or assembler exits nonzero | `ToolchainFailed` | `invoke` |
| Compiler input/output cannot be safely created, read, or synced | `Io` | `BuildWorkspace`, `write_new`, `read_regular`, `io_error` |
| Output is malformed ELF or violates required format structure | `ArtifactFormat` | artifact parser and metadata decoder |
| Output target, symbol, ABI, metadata, or code differs from request | `ArtifactMismatch` | `inspect_hsaco`, `inspect_hsaco_bundle`, `inspect_cubin` |
| `sm_major * 10 + sm_minor` overflows | `ArithmeticOverflow` | `nvidia_sm` |

Failures from upstream lowering remain distinct.  Unknown scalar values,
unsupported scalar operations, invalid stage contracts, prohibited LLVM
interfaces, and invalid workgroup sizes are raised before `ArtifactBuilder`
receives a module.  The builder does not add a fallback for those conditions.

Two ordering rules are part of the API contract:

* Single-module invocation order is verifier, then backend code generation,
  then linker or assembler.
* Bundle invocation order is, for each input, verifier and (for NVIDIA)
  code generation, followed by exactly one final linker or assembler
  invocation.  Input order is caller-owned and preserved in inspections and
  LLVM digest vectors.  `prepare` supplies deterministic artifact-ID order.

## What the builder deliberately does not do

* It does not lower scalar instructions or primitive stages.  Those operations
  belong to `llvm.rs` and `stage.rs` after upstream validation.
* It does not choose a target, workgroup size, memory layout, or toolchain.  It
  verifies the target and uses the target and ABI already carried by the
  lowered module.
* It does not invoke driver APIs, load native images, allocate memory, submit
  kernels, or measure performance.  Native probe and prepare own those steps.
* It does not retain scratch files or export build intermediates.  A single
  public result is a native HSACO or cubin (plus the in-memory PTX field for a
  single cubin build), and a bundle is one shared native image.
* It does not retry a failed tool, substitute a missing backend tool, accept a
  mismatched target, or trust a binary merely because a compiler returned
  success.  Successful return requires structural inspection.

## Source map

| Lines | Responsibility |
| --- | --- |
| 20-170 | phases, pinned tools, toolchain, invocation/provenance/result types, constructor |
| 172-215 | single-module dispatch and verifier preflight |
| 217-331 | AMD multi-entry HSACO bundle |
| 333-452 | NVIDIA multi-entry cubin bundle |
| 454-587 | single AMD and NVIDIA compiler pipelines and provenance |
| 589-615 | unique tool identity capture |
| 617-626 | checked NVIDIA SM encoding |
| 629-674 | deterministic process invocation and scratch-argument normalization |
| 676-711 | AMD processor/feature parsing |
| 713-765 | tool path, file, I/O, and bounded diagnostic helpers |
| 767-813 | private workspace allocation and cleanup |

The builder's correctness boundary is therefore: pinned inputs, exact target
and ABI, hermetic compiler process, regular-file I/O, structural ELF
inspection, and complete provenance.  Everything before it defines what the
kernel means; everything after it is responsible for loading and executing the
immutable bytes.
