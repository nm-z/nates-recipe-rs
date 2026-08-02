<!--
This document describes kernel/src/stage.rs. Source line references are kept
close to the implementation so the contract can be checked against the code.
-->

# Stage lowering

`kernel/src/stage.rs` is the closed realization boundary for one deferred
primitive stage. It accepts the complete, canonical `LoweredProgram`, the
planner's target-independent `ArtifactBuildRecipe`, one validated
`KernelTarget`, and the target-specific `LoweringOptions`. It returns one
target LLVM module and its explicit launch ABI as `LoweredKernel`.

The module has one purpose:

```text
immutable LoweredProgram + immutable ArtifactBuildRecipe
        -> contract validation
        -> one exact ProgramStage selection
        -> direct AMDGPU or NVPTX LLVM emission
        -> LLVM declaration audit
        -> LoweredKernel { llvm_ir, abi, work, target }
```

It does not choose a stage, change geometry, select an algorithm, repair a
stale build recipe, schedule work, allocate memory, load a driver module, or
execute a dispatch. A mismatch is an observable error and never a reason to
select a nearby stage or implementation.

## Position in the pipeline

The production transition is:

```text
recipe-language / recipe-ops
        |
        | typed primitive kernel and tensor views
        v
recipe-primitives::lower
        |
        | LoweredProgram:
        |   ordered ProgramStage values, buffers, geometry, bindings,
        |   synchronization, atomics, faults, resource bounds, digest
        v
recipe-planner::lower_program_invocation
        |
        | DraftPlan::artifact_builds:
        |   ArtifactBuildRecipe with stage-scoped identity and contract digest
        v
recipe-prepare::DeferredArtifactCompiler::materialize
        |
        | finds the exact program by source_kernel and program_digest
        | supplies measured target and entry symbol recipe_stage_<artifact>
        v
recipe-kernel::lower_stage
        |
        | validates the complete contract and emits one LoweredKernel
        v
ArtifactBuilder::{build_hsaco_bundle,build_cubin_bundle}
        |
        | pinned verifier, code generator, linker or ptxas, binary inspection
        v
NativeArtifact / RuntimeArtifact
        |
        v
recipe-native-executor loads once and launches by the recovered KernelAbi
```

`lower_stage` is called by `prepare/src/production.rs:523-546`. The caller
groups the resulting modules by one discovered target, builds one multi-entry
HSACO or cubin, and retains one logical artifact identity per stage entry. The
single-template `lower_elementwise` path in `kernel/src/llvm.rs` is a separate
public probe and scalar implementation; this module calls it only for
`StageKind::ScalarMap`.

## Inputs, output, and ownership

### Inputs

| Input | Owner before this module | Meaning at this boundary |
| --- | --- | --- |
| `program: &LoweredProgram` | `recipe-primitives` and planner | Complete immutable program. It is required even for one stage because it authenticates the source digest, stage ordinal, stage-scoped identity, and uniform-scan axis context. |
| `build: &ArtifactBuildRecipe` | planner `DraftPlan` | Candidate-local values, affine views, dispatch, operation bounds, fault value, resource envelope, and provenance for exactly one stage artifact. |
| `target: &KernelTarget` | measured discovery and `prepare` target table | Complete AMD target ID and code-object version, or NVIDIA SM and PTX identity. |
| `options: &LoweringOptions` | `prepare` | Entry symbol and the immutable workgroup width copied from the stage geometry. |

The stage lowerer does not read runtime state. `RunId` and `LoopIteration`
remain explicit launch arguments when a stage contract requires them.

### Output

`LoweredKernel` contains:

* `llvm_ir`: complete target LLVM text, including target triple, entry point,
  declarations, private helpers, shared globals, and strict floating-point
  attributes;
* `abi`: `KernelAbi` with the exact entry symbol, ordered `KernelArgument`
  values, checked byte size and alignment, logical element count, and
  workgroup width;
* `work`: the stage `FlopCount` copied from the immutable build contract; and
* `target`: an owned clone of the exact `KernelTarget` used for emission.

The module returns no native bytes. `builder.rs` consumes this result during
`Realize`, and the native executor consumes the inspected image and ABI after
finalization.

## Immutable representation consumed by `stage.rs`

`ProgramStage` is the primitive lowering record. Its fields are ordered and
authenticated by `recipe-primitives/src/hash.rs`:

| Field | Use in stage lowering |
| --- | --- |
| `id` | Selected by `build.provenance.stage_ordinal`; it also participates in the stage-scoped kernel identity. |
| `dependencies` | Already represented in the planner task graph. The LLVM module does not create or alter task dependencies. |
| `geometry` | Supplies logical lanes, workgroup lanes, and workgroups. All launch values must match the build recipe. |
| `bindings` | Ordered `BufferBinding` records. Dtype, access mode, affine extents, offset, strides, and storage bytes define the stage pointer map. |
| `synchronization` | Fixed tree or tile synchronization points. Their ordinals and memory semantics are checked before barriers are emitted. |
| `atomics` | Declared payload, histogram, and fault atomic contracts. Emitters require the matching domain, operation, ordering, and dtype. |
| `fault` | Optional checked-path flag, reason, code, guard-before-address rule, and publication atomic. |
| `resources` | Exact FLOP, integer, atomic, shared, private, and maximum-workgroup bounds. |
| `kind` | One of the sixteen `StageKind` variants dispatched by `lower_owned_stage`. |

`LoweredProgram` additionally carries schema version, source kernel identity,
source input/output arity, a complete source alias matrix, dense buffers,
program-level resources, and its canonical digest. `program.validate()` checks
dense buffer and stage IDs, serial stage dependencies, view safety, atomic and
fault contracts, kind-specific arity, fixed algorithm shapes, synchronization,
resource formulas, and the digest. `stage.rs` requires this validation to pass
again at realization.

`ArtifactBuildRecipe` carries the candidate-local `ValueId` for each ordered
binding, `ArtifactBuildAccess`, the copied `ArtifactBuildView`, exact dispatch
geometry, all three work counters, optional fault value, resource envelope,
stage-scoped identity, source identity, and provenance. Its own validator
rejects zero identities, zero geometry, incorrect ceiling workgroup counts,
duplicate values, malformed views, and an invalid fault binding.

## Realization transition

`lower_stage` (lines 86-101) is deliberately short. Its transition is:

1. `validate_contract` authenticates the complete program, recipe, target,
   options, identity, geometry, work, resources, bindings, and fault mapping.
2. `StageKind::ScalarMap` is delegated to `crate::lower_elementwise` and its
   generic scalar fault publication is rewritten to the planned fault code.
3. Every other kind enters `lower_owned_stage`, which constructs a stage
   signature, emits direct LLVM, audits declarations, derives the ABI, and
   validates the realized element count, work, workgroup width, and fault ABI.

No step mutates the input program or build recipe. The only mutation is the
local scalar LLVM string during the checked fault rewrite.

## Contract validation

### `validate_contract`

`validate_contract` (lines 122-208) returns the exact `ProgramStage` reference
that the recipe names. It performs these checks in order:

1. `program.validate()` must succeed.
2. `build.validate()` must succeed.
3. `target.validate()` and `validate_options(options)` must succeed.
4. `build.provenance.program_digest` must equal the canonical program digest.
5. `build.source_kernel` must equal `program.source_kernel`.
6. `build.provenance.stage_ordinal` must name a stage in `program.stages`.
7. `build.kernel_template` must equal `stage_template_identity(program,stage)`.
8. `build.artifact.get()` must equal the reserved stage-scoped identity.
9. `build.provenance.contract_digest` must equal the independent digest
   returned by `artifact_build_contract_digest(build)`.
10. All three dispatch fields must equal `stage.geometry`.
11. `options.workgroup_lanes` must equal the immutable stage width.
12. `build.work` must equal the stage's FLOP, integer-operation, and
    atomic-operation bounds.
13. `build.resources` must equal the stage private/shared bounds, zero
    scratch-per-dispatch, and maximum workgroup width.
14. Binding count and every ordered binding dtype, access, logical extents,
    offset, strides, and storage byte count must equal the stage binding.
15. `validate_fault_binding` must prove the optional fault value is absent for
    unchecked stages or names the build value at the exact ordered fault
    binding position for checked stages.

All failed stage-contract checks use `LoweringErrorKind::InvalidStageContract`.
Target validation and option validation retain their own machine-readable
`InvalidTarget`, `InvalidEntrySymbol`, and `InvalidWorkgroupSize` kinds.

The build binding's candidate `ValueId` is intentionally not compared with a
primitive `BufferId`. The ordered position, dtype, access, and view are the
contract bridge. The fault binding is additionally tied to its stage buffer
position, which prevents a caller from moving the flag to another ABI slot.

### Stage-scoped identity

`stage_template_identity` (lines 255-271) hashes the domain
`recipe-planner-stage-template-v1`, the canonical program digest, the source
kernel ID, and the stage ID. It takes the first eight digest bytes as a little
endian `KernelTemplateId`; zero is rejected as reserved. `planner.rs` uses an
independent `StableDigest` implementation over the same fields. The identity
is collision-checked during planning and is reused as both `kernel_template`
and `artifact` in the deferred recipe.

### Contract digest

`artifact_build_contract_digest` (lines 30-79) is an independently implemented
SHA-256 contract digest. `ContractDigest` writes the domain and every variable
length byte sequence as decimal byte length, `:`, then bytes. `Digest` values
are appended as raw 32 bytes. `u8` values are one byte and `u64` values are
little endian. The encoded field order is fixed:

```text
domain = recipe-planner-artifact-build-v1
artifact.get
kernel_template.get
source_kernel.get
provenance.program_digest
provenance.stage_ordinal
bindings.len
for binding in bindings:
    value.get
    dtype tag: F32=0, I32=1
    access tag: Read=0, Write=1, ReadWrite=2, ReadWriteAtomic=3
    view.logical_extents.len, each extent
    view.offset_elements
    view.strides.len, each stride
    view.storage_bytes
dispatch.logical_lanes
dispatch.workgroup_lanes
dispatch.workgroups
work.flops
work.integer_operations
work.atomic_operations
fault_flag presence, then fault_flag.get when present
resources.private_bytes_per_lane
resources.shared_bytes_per_workgroup
resources.scratch_bytes_per_dispatch
resources.maximum_workgroup_lanes
```

The digest authenticates every field that can change the pointer ABI, launch
geometry, operation bounds, fault storage, or resource reservation. A stale
planner hash cannot bless a changed recipe at realization.

## Stage signature and ABI

### `StageSignature`

`StageSignature::new` (lines 343-440) zips build bindings with stage bindings
and creates one `BoundPointer` per ordered binding. A pointer keeps its dtype,
copied affine view, optional read parameter, optional write parameter, and a
fault marker. The fault marker is true only for the stage binding whose
`BufferId` equals `stage.fault.flag`.

The explicit argument order is fixed and is shared by LLVM text, HSA metadata,
CUDA launch filling, and native-executor validation:

```text
1. readable non-fault bindings, in stage order: KernelArgument::Buffer(Read)
2. writable non-fault bindings, in stage order: KernelArgument::Buffer(Write)
3. one KernelArgument::FaultFlag when stage.fault is Some
4. one KernelArgument::RunId for Philox4x32_10
5. one KernelArgument::LoopIteration for Philox4x32_10, or IndexMap with a
   nonzero iteration_step
6. one KernelArgument::ElementCount, always last
```

A `ReadWrite` or `ReadWriteAtomic` binding therefore receives two pointer
slots, one in each pointer phase. It is one `BoundPointer` with separate read
and write names, so an emitter cannot load from a write-only binding or store
through a read-only binding. Fault storage is not exposed as a normal data
pointer and is never counted among the input or output value pointers.

Every data and fault pointer uses global address space 1 and four-byte payload
alignment. Dynamic and element-count values are 64-bit by-value parameters.
`abi()` (lines 448-477) checks the argument count fits `u32`, computes
`argument_bytes = count * 8` with checked arithmetic, constructs a nonzero
`ElementCount` from logical lanes, and sets ABI alignment to eight bytes.

The CUDA and HSA native launchers consume this same ordered list. They bind
input and output arena addresses, the resolved fault flag, the run ID, the
zero-based loop iteration, and `abi.elements`. Native validation rejects a
runtime artifact whose explicit ABI order, count, dtype, access, backing bytes,
alignment, or final element-count argument differs.

## LLVM accumulator and common memory rules

`Ir` (lines 480-705) is the owned text accumulator:

* `body` is emitted in operation order.
* `next` gives temporaries and labels unique numeric suffixes.
* `declarations` is a `BTreeMap`, so intrinsic declarations are deterministic
  and de-duplicated.
* `globals` preserves insertion order for shared workgroup arrays.
* `helpers` is a `BTreeMap`, so generated atomic and Philox helpers are
  deterministic and de-duplicated.

`emit_global_index` uses AMDGPU workgroup/workitem intrinsics or NVPTX
`ctaid.x`/`tid.x`, zero-extends both IDs to i64, multiplies group ID by the
immutable workgroup width, and adds local ID to form `%global_id`.

`barrier` emits `llvm.amdgcn.s.barrier` for AMD or `llvm.nvvm.barrier0` for
NVIDIA. `coordinates` decomposes a row-major linear value through nonzero
logical extents with `udiv` and `urem`. `index_from_coordinates` adds the
view offset and each nonzero stride contribution in elements. `load`, `store`,
and `write_address` form typed global address-space-1 pointers with inbounds
GEP and four-byte alignment; missing read or write capability is an
`InvalidStageContract` error.

`llvm_type` maps `DType::F32` to `float` and `DType::I32` to `i32`.
`private_address_space` is AMD address space 5 and NVIDIA address space 0.
NVIDIA generic address space 0 is intentional: NVPTX lowering rewrites
generic allocas to local storage, while directly emitting address space 5 can
leave an invalid pointer after optimization.

`assemble_stage_module` (lines 853-895) emits the target triple, sorted
declarations, ordered globals, sorted helpers, and one kernel entry using
`amdgpu_kernel` or `ptx_kernel`. The entry has `nounwind`, `strictfp`, IEEE
f32 denormal handling, and `no-trapping-math=false`. `begin_lane` guards
`%global_id < %element_count`; `finish_lane` branches to `exit` and returns.

## Owned stage dispatch

`lower_owned_stage` (lines 725-827) derives dynamic-argument presence from the
kind, creates the signature and IR accumulator, emits the target global index,
then dispatches exactly one emitter. It rejects `ScalarMap` if one reaches this
branch, because scalar maps must use the complete scalar emitter. After the
kind emitter returns it assembles and audits the module, derives the ABI, uses
the build's FLOP bound as `LoweredKernel.work`, and calls
`validate_realized_kernel`.

| `StageKind` | Direct emitter | Contract and result |
| --- | --- | --- |
| `ScalarMap { template }` | `crate::lower_elementwise` in `lower_stage` | Complete scalar SSA template, optional planned fault rewrite. |
| `Fill { value }` | `emit_fill` | One lane computes the output affine address and stores a typed literal. |
| `Copy` | `emit_copy` | One lane loads input affine view and stores output affine view; dtypes must match. |
| `FixedTreeReduce` | `emit_reduction` | Fixed shared-memory reduction, optional indexed result, deterministic tie break. |
| `FixedTreeScanLocal` | `emit_scan_local` | Fixed Blelloch upsweep and downsweep over one local block. |
| `ScanUniformCombine` | `emit_scan_uniform` | Adds the unique matching hierarchy offset to each post-first-block element. |
| `TiledContraction` | `emit_contraction` | Canonical contracted coordinate order, private accumulator, optional staged tile barriers. |
| `Gather` | `emit_gather` | Affine gather with reject, clamp, or wrap bounds policy. |
| `Scatter` | `emit_scatter` | Affine scatter with unique store or declared tensor-element atomic. |
| `HistogramClear` | `emit_zero` | Clears each histogram output element. |
| `HistogramAccumulate` | `emit_histogram` | Checked bin mapping and declared weighted or unit atomic add. |
| `StableSortInitialize` | `emit_sort_initialize` | Loads valid values and pads scratch entries with index `-1`. |
| `StableSortCompareExchange` | `emit_sort_compare` | One fixed bitonic compare-exchange pair with stable ordering. |
| `StableSortFinalize` | `emit_sort_finalize` | Copies sorted values and optional original indices to the output view. |
| `IndexMap` | `emit_index_map` | Checked i64 affine arithmetic, optional Euclidean modulus, code 4 fault. |
| `Philox4x32_10` | `emit_philox` | Recipe-owned counter-based random generation with dynamic run and iteration. |

## Per-kind lowering contracts

The following descriptions state what the emitter consumes and emits. They do
not describe speculative algorithms. Primitive lowering fixes these fields
before this module is entered.

### Fill, zero, and copy

`emit_fill` requires pointer 0 to have the literal's dtype. It emits an
in-bounds lane, computes `affine_index(output,%global_id)`, bitcasts f32
literal bits without a host conversion, and stores the value. `emit_zero`
uses the typed zero literal and the same affine path. `emit_copy` requires
readable pointer 0 and writable pointer 1 with equal dtypes, loads one typed
value, and stores it through the destination view.

### Checked index map

`emit_index_map` requires an int32 output, an arithmetic-domain fault with code
4, no `RunId`, and a `LoopIteration` argument exactly when
`iteration_step != 0`. A present modulus must be strictly positive.

Each lane computes:

```text
element = checked_i64(%global_id * element_step)
iteration = checked_i64(%loop_iteration * iteration_step)
affine = checked_i64(start + element + iteration)
```

`checked_unsigned_scale` rejects an unsigned input above `i64::MAX` and uses
`llvm.smul.with.overflow.i64`; `checked_signed_add` uses
`llvm.sadd.with.overflow.i64`. With a modulus, `srem` is corrected to a
nonnegative Euclidean remainder and narrowed to i32. Without a modulus, the
affine value is additionally checked against `i32::MIN..=i32::MAX`. Rejected
lanes publish the exact code with release `atomicrmw xchg` and exit before the
output address is formed. Valid lanes store through the output view.

### Fixed-tree reductions

`emit_reduction` requires the canonical fixed tree, operator-identity padding,
and lowest-logical-index tie breaking. A non-value reduction also requires an
int32 index binding. The first pass maps a sequence and reduced coordinate
back into the original tensor axes. Later passes flatten `sequence * input_width
+ reduced` across one-dimensional scratch values.

The emitter allocates a shared `[lanes x i32]` value array and, for indexed
results, a matching index array. Every lane selects an identity for inactive
input, stores its payload, initializes an index from the source or local
position (inactive uses `i32::MAX`), then barriers. For each tree step,
participating lanes combine a left and right shared value, optionally carrying
the selected index, store the result, and barrier again. The synchronization
entry at each step must carry its zero-based tree ordinal, workgroup scope, and
shared acquire/release semantics. Local lane zero writes the final value and/or
index to the output affine view.

Identities are sum 0, product 1, minimum positive infinity or `i32::MAX`,
maximum negative infinity or `i32::MIN`, any 0, and all 1. `Any` and `All`
require int32. Floating min/max use a sign-aware total-order key and
canonicalize NaNs; equal values with index payload select the lower index.
Floating sum/product use constrained fadd/fmul, while integer arithmetic uses
LLVM `add`/`mul`.

### Local and uniform scans

`emit_scan_local` requires the fixed scan tree, typed input/output and optional
totals binding, and the synchronization list for that tree. It maps each
workgroup to one scan block, reverses positions when requested, pads inactive
lanes with the operator identity, and performs shared-memory Blelloch
upsweep/downsweep. The first downsweep step resets the root, stores a block
total when a totals binding exists, and writes either the user exclusive
identity or the operator identity. A scan tree containing a reduction-phase
step is rejected. Every emitted step checks its synchronization ordinal.

Active lanes write user-inclusive values by combining the exclusive prefix
with the original value. User-exclusive and hierarchy-exclusive modes write
the exclusive prefix directly. Rank-one bindings flatten as
`sequence * input_width + position`; higher-rank bindings require the declared
axis extent to equal `input_width`, then decompose the free axes.

`scan_axis` is used by `ScanUniformCombine`. It scans all other program stages
for exactly one `FixedTreeScanLocal` with the requested level. Zero matches or
more than one distinct axis returns `InvalidStageContract`, rather than
guessing. `emit_scan_uniform` requires a nonempty post-first-block domain,
loads the target and hierarchy offset, combines them with the scan operator,
and stores the target. The target view must expose the recovered axis and
declared width.

### Tiled contractions and NVIDIA TF32

`emit_contraction` requires equal operand/output dtype, canonical contracted
axis order, unique in-range axis pairs, matching paired extents, and an output
rank equal to batch plus free left and free right axes (with scalar rank one).
It maps output coordinates into each operand and visits contracted coordinates
in the fixed row-major order. Each lane keeps a private typed accumulator and
iterates fixed reduction tiles. F32 uses constrained FMA; I32 uses multiply
then add. Inactive lanes use output zero for safe indexing, execute all tile
control flow and any required barriers, and skip the final store.

The `Staged` strategy checkpoints and restores each lane's accumulator through
an address-space-3 array with two barriers per tile. The measured primitive
planner currently selects `Direct`, but the emitter preserves the staged
contract if a valid stage carries it. Private accumulator and loop slots use
AMD address space 5 and NVIDIA generic address space 0.

For NVIDIA SM major at least 8, `emit_nvidia_tf32_matrix_contraction` handles
only dense rank-two `A * B^T` F32 with contract axis `(1,1)`, zero offsets,
contiguous row-major strides, complete 16x8x4 tiles, and complete warps. One
warp owns one output tile, loads explicit fragments, emits inline PTX
`cvt.rna.tf32.f32` and `mma.sync.aligned.m16n8k4.row.col.f32.tf32.tf32.f32`,
then stores four accumulators per lane. Any shape outside that exact predicate
falls back to the canonical scalar contraction path.

### Gather and scatter

`emit_gather_mapping` derives the index rank from result rank and payload rank,
requires the index view rank to match, loads one int32 index, and constructs
payload coordinates around the declared axis. `normalize_index` requires a
nonzero extent no larger than `i64::MAX`. Reject returns a signed index plus a
valid predicate, clamp maps to `[0,extent-1]`, and wrap uses signed remainder
with negative correction.

`emit_gather` requires int32 indices and matching input/output dtypes. Reject
must have exactly one valid predicate and checked fault contract. Invalid lanes
publish before forming a payload address and return. Clamp and wrap must have
neither predicate nor fault. Valid lanes load the mapped input and store the
result.

`emit_scatter` maps indices against the output view and loads the update before
the bounds branch. Unique conflicts store directly. Atomic conflicts require a
matching `TensorElements` atomic contract for operation, ordering, and dtype;
the payload address is then updated with `emit_atomic_update`.

### Atomics and checked faults

`emit_fault_publish` accepts only `guard_before_address=true`, int32
`SingleFaultFlag`, `Exchange`, and `Release`. It emits
`atomicrmw xchg ptr addrspace(1) %fault_flag, i32 <code> release`.

I32 payload atomics map exchange/add/minimum/maximum to LLVM `atomicrmw`
`xchg`/`add`/`min`/`max`. F32 exchange uses `atomicrmw xchg`; F32 add, minimum,
and maximum use one Recipe-owned compare/exchange helper per operation and
ordering. Helpers load an i32 bit pattern, compute a constrained desired f32
value, canonicalize NaN, compare/exchange with the requested success and
legal failure orderings, and retry until success. Floating min/max compare the
same total-order keys used by reductions and sorting.

`atomic_ordering` maps Relaxed, Acquire, Release, AcquireRelease, and
SequentiallyConsistent to `monotonic`, `acquire`, `release`, `acq_rel`, and
`seq_cst`. `atomic_failure_ordering` maps Release to monotonic and
AcquireRelease to acquire, as required for a valid cmpxchg failure order.

### Histogram

`emit_histogram` requires a checked histogram fault, nonzero bin count, output
extent `[bins]`, output dtype I32 for unweighted or F32 for weighted, and F32
weight input when weighted. It also requires the stage's `HistogramBins`
atomic to be Add with the declared ordering and output dtype.

For `I32Direct`, the input is sign-extended and accepted only in
`0..bins`. For `F32TruncateTowardZero`, `llvm.fptosi.sat.i32.f32` performs the
conversion, an ordered comparison rejects NaN, and the converted value must be
nonnegative and below `bins`. Invalid lanes publish the histogram fault and
return. Valid lanes use the loaded weight or integer one as the atomic add
operand and update the bin through the declared affine view.

### Stable sort

`validate_sort_network` requires a nonzero axis length, exact next-power-of-two
padding, original-axis-index ascending tie break, IEEE-754 total order, and
padding after every valid element.

`emit_sort_initialize` maps each lane to a slice and padded position. Valid
positions load the input and store their original axis index. Padding stores a
typed zero and index `-1`. `emit_sort_compare` maps a lane to a pair in one
bitonic phase, loads both scratch values and indices, and computes stable
ordering through `sort_before`: padding is always after valid values, values
use signed I32 order or F32 total-order keys, and equal bit patterns select the
lower original index. The merge-width mask chooses ascending or descending
phase and both selected values and indices are written back.

`emit_sort_finalize` maps output coordinates to a free-axis slice, reads the
corresponding padded scratch entry, writes the value, and optionally writes the
original index. `sort_tensor_index` validates the network axis and reconstructs
non-axis coordinates. `emit_linear_from_coordinates` folds free coordinates
row-major for scratch slice addressing.

### Philox4x32-10 V1

`emit_philox` requires the exact Recipe contract: ten rounds, multipliers
`0xd2511f53` and `0xcd9e8d57`, Weyl constants `0x9e3779b9` and `0xbb67ae85`,
counter words `[ElementLow, ElementHigh, IterationXorStreamLow,
IterationXorStreamHigh]`, both key-fold flags true, unbiased multiply-high
int32 mapping, and owned Box-Muller V1 normal mapping. The output dtype must
match the distribution.

The lane ID is split into low and high 32-bit words. `loop_iteration XOR
contract.key.stream` supplies the stream counter words. The key base folds
seed low/high words with the source kernel ID, then XORs the two run ID words:

```text
counter = [element_low, element_high,
           low(loop_iteration XOR stream), high(loop_iteration XOR stream)]
key_0 = low(seed_low)  XOR high(seed_high) XOR low(source_kernel)
key_1 = high(seed_low) XOR low(seed_high)  XOR high(source_kernel)
key = [low(run_id) XOR key_0, high(run_id) XOR key_1]
```

`ensure_philox_helper` emits one internal tuple-returning helper and unrolls
exactly ten rounds. Each round multiplies the first and third counters in i64,
extracts high and low words, xors cross-products with the other counters and
keys, then advances keys for rounds zero through eight. The helper is inserted
into the sorted helper map once.

Distribution mappings are direct and versioned by the contract:

| Distribution | Emission |
| --- | --- |
| `UniformF32` | Shift a word by nine, OR with the `1.0` exponent bits, bitcast, then constrained subtract one to obtain `[0,1)`. |
| `BernoulliI32` | Convert the validated finite probability to an integer threshold in `[0,2^32]`; `p=1` returns one, otherwise compare unsigned word below threshold and zero-extend. |
| `UniformI32` | Compute `range = high_exclusive - low` in i64, reject conversion overflow, retry Philox with counter word three XORed by attempt until low product is above `(-range) mod range`, then use product high plus `low`. |
| `NormalF32` | Convert two words to positive 24-bit uniforms in `(0,1]`, evaluate owned `sqrt(-2*log(u1))*cos(2*pi*u2)`, and preserve constrained f32 behavior. |

`bernoulli_threshold` rejects nonfinite or out-of-range probabilities and
performs the threshold conversion from f32 bits without a host float-to-int
rounding path. `owned_log_v1` decomposes exponent and mantissa and sums the
odd atanh terms through divisor 15. `owned_cos_v1` reduces by tau, reflects
past half-pi, and evaluates the fixed Horner polynomial through x10. The
normal path's private slots and uniform-int retry state use the target-specific
private address-space rule above.

## Module assembly and audit

After an owned emitter completes, `assemble_stage_module` emits one complete
LLVM module. `audit_llvm_ir` scans declaration lines and rejects the first
declared symbol that does not start with `llvm.` as
`LoweringErrorKind::ProhibitedInterface`. This permits target intrinsics,
constrained arithmetic, square root, saturating conversion, and checked
overflow intrinsics, while rejecting unresolved host runtimes and vendor math
libraries.

`validate_realized_kernel` (lines 829-851) proves that the returned ABI element
count and workgroup width equal the stage geometry, the returned FLOP work
equals the build work, and the number of `FaultFlag` arguments equals the stage
fault presence. Native artifact inspection and native-executor plan validation
perform the later target-specific metadata, argument type, backing storage,
alignment, and launch checks.

## Caller transitions and tool inputs/outputs

### Planner to prepare

`planner::lower_program_invocation` materializes every primitive buffer to a
candidate `ValueId`, copies each stage binding into an `ArtifactBuildBinding`,
filters the fault value out of calculation input and output lists, and hashes
the build contract. It creates one loop calculation task per stage, preserving
stage order and serial dependencies. A checked stage joins a device and
iteration-domain fault cohort; the planner emits one four-byte metric readback
after the cohort and makes user metrics and exit transfers depend on it.

### Prepare to kernel

`lower_deferred_stage` locates the one `LoweredProgram` whose `source_kernel`
and digest match the build provenance. It derives `entry_symbol` as
`recipe_stage_<artifact.get()>` and passes the build workgroup width in
`LoweringOptions`. Missing exact program, target ambiguity, stage mismatch,
or any lowering error stops materialization.

### Kernel to builder

The returned `LoweredKernel` is grouped with other entries for the same target.
AMD invokes the pinned verifier once per LLVM module, links all bitcode modules
with the pinned ELF linker, and inspects every entry ABI in one HSACO. NVIDIA
verifies and code-generates each module to PTX, invokes ptxas once for the
cubin bundle, and inspects every entry. The builder receives no source
primitive or build recipe and does not lower again.

### Builder to native executor

Prepare creates a `NativeArtifact` with the image digest, target, toolchain,
entry symbol, stage identity, resources, and build provenance. The native
executor loads one shared image per digest, resolves each logical entry, and
retains the `KernelAbi`. Before launch, `native-executor/src/plan.rs` checks
the ABI against the finalized build bindings or scalar template. CUDA fills
device pointers and by-value arguments into a preallocated launch block. HSA
writes the same values at eight-byte kernarg offsets and checks metadata size,
alignment, and visible argument order.

## Failure vocabulary

The module returns `Result<LoweredKernel, LoweringError>` and never returns a
partially trusted module. The relevant failures are:

| Condition | Kind and source |
| --- | --- |
| Program or build recipe fails canonical validation | `InvalidStageContract`, `validate_contract` |
| Program digest, source ID, stage ordinal, stage identity, artifact identity, or contract digest differs | `InvalidStageContract` |
| Geometry, work, resources, binding order/view, or fault value differs | `InvalidStageContract` |
| Entry symbol is empty, begins with a digit, or contains punctuation | `InvalidEntrySymbol`, `validate_options` |
| Workgroup width is outside `1..=1024` | `InvalidWorkgroupSize`, `validate_options` |
| Target identity is invalid | `InvalidTarget`, `KernelTarget::validate` |
| A stage pointer index is absent, or read/write capability is missing | `InvalidStageContract`, `StageSignature` or `Ir` |
| Stage ABI argument count, byte size, or logical element count overflows | `ArithmeticOverflow` or `InvalidStageContract` |
| Scalar template lowering fails | `InvalidKernel`, `UnknownScalarValue`, `UnsupportedOperation`, or `ArithmeticOverflow` from `llvm.rs` |
| Scalar checked stage lacks the generic OR publication to rewrite | `InvalidStageContract`, `rewrite_scalar_fault` |
| Owned-stage LLVM declares a non-LLVM external symbol | `ProhibitedInterface`, `audit_llvm_ir` |
| Tree, scan, contraction, index, histogram, sort, or Philox shape/contract is inconsistent | `InvalidStageContract` |
| Checked operation has no exact fault guard or fault publication ABI | `InvalidStageContract` |
| Bernoulli probability is nonfinite, outside `[0,1]`, or has an invalid exponent | `InvalidStageContract` |
| Bernoulli or uniform-int host arithmetic cannot form the required threshold/range | `InvalidStageContract` |

The module does not retry, switch target, substitute a helper, accept stale
metadata, or mask a failed transition with a status-only success.

## Invariants to preserve

* `LoweredProgram` and `ArtifactBuildRecipe` are immutable authenticated
  inputs. The complete program is required to recover context that a fragment
  cannot prove.
* The planner and primitive lowerer choose semantics, geometry, trees, tile
  strategy, atomics, faults, and resource bounds. `stage.rs` consumes and
  independently rechecks those choices.
* Stage identity is scoped by program digest, source kernel, and stage ordinal;
  it is not the source primitive ID and is reused as the reserved artifact ID.
* ABI order is readable pointers, writable pointers, optional fault flag,
  dynamic run ID, dynamic loop iteration, and final element count. A read-write
  binding appears in both pointer phases.
* Every global payload access uses the exact affine view and typed f32 or i32
  load/store. Higher-rank addressing remains in the binding, while launch is
  always one-dimensional.
* Fault branches publish the planned code with release exchange before return
  and before forming an invalid payload address.
* Collective stages execute all required barriers, including inactive lanes in
  a partial workgroup. Barrier ordinals and memory semantics come from the
  stage contract.
* F32 arithmetic is constrained, strict, and NaN-canonicalized where the
  operation contract requires it. No vendor operation library is called.
* Dynamic run and iteration values are explicit launch arguments and are never
  embedded into AOT image bytes.
* LLVM declaration audit happens before any toolchain invocation. Native image
  inspection and executor ABI checks remain required after this module returns.
* No compiler, allocator, driver, queue, scheduler, artifact file, or runtime
  lifecycle state is retained by this module.

## Source map

| Lines | Responsibility |
| --- | --- |
| 24-79 | Independent artifact-build contract digest |
| 81-120 | Public stage transition and scalar fault rewrite |
| 122-296 | Contract checks, identity, access conversion, error helpers |
| 298-332 | SHA-256 contract digest writer |
| 334-478 | Ordered binding pointers and `KernelAbi` construction |
| 480-705 | Deterministic LLVM accumulator, target IDs, affine views, memory helpers |
| 706-724 | Dtype and private address-space mappings |
| 725-851 | Owned-stage dispatch, module assembly, declaration audit, realized ABI checks |
| 853-1184 | Lane prologue, fill/copy/index-map, checked arithmetic, scan-axis lookup |
| 1194-2035 | Fixed reductions, identities, comparisons, constrained f32 helpers |
| 2037-2488 | Local scans, uniform scan combination, coordinate mapping |
| 2489-3010 | Tiled contractions and NVIDIA TF32 matrix path |
| 3010-3260 | Gather, scatter, index normalization, fault publication |
| 3278-3559 | Integer and floating atomics, histogram accumulation |
| 3560-3993 | Stable sort network initialization, compare-exchange, finalization |
| 3994-4638 | Philox V1 helper, distributions, owned log and cosine approximations |

The smallest coherent interpretation is therefore: `stage.rs` is the
fail-closed, target-aware compiler boundary between immutable Recipe primitive
stage contracts and native artifact construction. Everything before it must
provide a complete contract. Everything after it must consume the exact LLVM
module, ABI, work, and target it proves.
