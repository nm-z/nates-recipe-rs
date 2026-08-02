<!--
Intent: describe primitives/src/hash.rs together with the ProgramDigest state
that it computes in primitives/src/model.rs. The byte stream below is the
implemented contract, not a proposed serialization format.
-->

# `primitives/src/hash.rs`

## Intent and boundary

This private module computes the content identity of one fully lowered
`recipe_primitives::LoweredProgram`. It is not a hash of the source
`PrimitiveKernel`, a tensor payload, measured hardware, a target, a toolchain,
an artifact image, or the existing `program.digest` field. It hashes the
immutable backend-neutral program contract after lowering has selected its
buffers, stages, geometry, synchronization, atomics, checked-fault behavior,
stage algorithms, and resource bounds.

The boundary is deliberately small:

~~~
validated PrimitiveKernel + tensor index + LoweringHardware
        |
        | recipe_primitives::lower
        v
LoweredProgram { ...all lowered contract fields..., digest = ZERO }
        |
        | LoweredProgram::canonical_digest
        v
CanonicalWriter(HASH_DOMAIN, fields in fixed order)
        |
        | private sha256
        v
ProgramDigest([u8; 32])
        |
        | assign, validate, and carry as provenance
        v
planner -> deferred artifact build -> kernel realization -> preparation
~~~

`program_digest` is `pub(crate)` and `hash` is a private module. External
callers can only request the same calculation through the public
`LoweredProgram::canonical_digest` method. `CanonicalWriter` and `sha256` are
private implementation details. The module has no fallible return type, no
runtime state, no filesystem or hardware access, and no alternate hash path.

The digest is a proof of the lowered contract. It is not a stage identity by
itself and it does not authenticate target realization. Planner and realization
add their own domain-separated identities and contract digests around it.

## Producer, initializer, and validator

`recipe_primitives::lower` validates the language kernel and hardware, adds
external tensor buffers in input/output encounter order, emits the ordered
`ProgramStage` chain for the selected primitive family, aggregates exact
resource bounds, and calls `ProgramBuilder::finish`.

`ProgramBuilder::finish` performs the identity transition in this order:

1. It computes `ProgramResourceBounds` with checked arithmetic.
2. It constructs `LoweredProgram` with schema `LOWERED_PROGRAM_SCHEMA_VERSION`
   (currently `2`), the source kernel and arities, source alias records,
   buffers, stages, aggregate resources, and `ProgramDigest::ZERO`.
3. It replaces the zero sentinel with `program.canonical_digest()`.
4. It calls `program.validate()`. Any structural error, resource mismatch, or
   digest mismatch becomes `LoweringErrorKind::InvalidLoweredProgram`; the
   program is not returned.

`LoweredProgram::canonical_digest` calls this module directly. The method is
useful both for initial construction and for independent recomputation after
the value has crossed a boundary. `validate` runs all structural checks and
then requires `program.digest == program.canonical_digest()`. Because the
stored digest is not an input to its own hash, mutating any hashed field makes
the recomputation differ, while mutating only the stored digest leaves the
recomputed value unchanged and is caught by that equality check.

`ProgramDigest` in `model.rs` is a typed 32-byte value. It is `Copy`, ordered,
hashable, and equality comparable; `ZERO` is the all-zero construction
sentinel, `new` and `bytes` are infallible, and `Debug` prints only the first
six bytes as `ProgramDigest(<12 hex digits>...)`. The primitive validator does
not reject an all-zero computed digest, although downstream core artifact
validation rejects a zero `ArtifactBuildProvenance::program_digest`. There is
no digest serialization, hex conversion, or algorithm choice in
`ProgramDigest` itself.

## Canonical byte primitives

`CanonicalWriter` owns one `Vec<u8>`. Every method appends to that vector; it
does not insert separators other than the explicit length prefixes and enum
tags below. The stream is deterministic for the same field values and vector
orders.

| Writer method | Bytes appended |
| --- | --- |
| `bytes(value)` | `length(value.len())`, then the raw bytes. |
| `bool(value)` | `u8(0)` for `false`, `u8(1)` for `true`. |
| `u8(value)` and `tag(value)` | One byte. `tag` is only a naming alias for `u8`. |
| `u32(value)` | `value.to_le_bytes()`, four bytes. |
| `i32(value)` | `value.to_le_bytes()`, four bytes, preserving two's-complement bits. |
| `u64(value)` | `value.to_le_bytes()`, eight bytes. |
| `usize(value)` | `length(value)`. It is not an eight-byte integer field. |
| `length(value)` | A 16-byte little-endian, zero-extended representation made by `usize_u128_le_bytes`. |
| `sequence(values, encode)` | `length(values.len())`, followed by each element encoded in slice order. |
| `finish()` | Moves out the accumulated vector. |

`usize_u128_le_bytes` copies the target's native `usize::to_le_bytes()` into
the low bytes of a 16-byte zero-filled array. Thus vector lengths and every
field passed to `usize` have a fixed 128-bit-width representation, with the
same bytes for equal values on the supported 32-bit and 64-bit targets. It is
not a checked conversion and cannot return an error.

The domain is written through `bytes`, so it is length-delimited. The current
domain is the literal bytes of
`recipe-lowered-primitive-program-v2\0` (the trailing NUL is part of the
domain), preceded by its 16-byte length. The domain and the program schema
field are separate version controls. Changing either changes every digest.

## Top-level `LoweredProgram` order

`program_digest` appends exactly this sequence, then hashes the resulting
bytes:

~~~
bytes(HASH_DOMAIN)
u32(program.schema_version)
u64(program.source_kernel.get())
usize(program.source_input_count)
usize(program.source_output_count)
sequence(program.source_aliases, encode_source_alias)
sequence(program.buffers, encode_buffer)
sequence(program.stages, encode_stage)
program.resources through encode_program_resources
~~~

The `digest` field is intentionally absent. There is no trailing checksum,
salt, timestamp, pointer, allocation address, target choice, or runtime run
identifier. `source_kernel.get()` is encoded as a `u64`, while source arities,
alias indexes, axes, and other `usize` values use the 16-byte length encoding.

### Source alias matrix

Each `SourceAliasContract` is appended as:

~~~
usize(alias.input)
usize(alias.output)
alias_permission(alias.permission)
~~~

The sequence length comes first. `AliasPermission` has fixed tags:

| Value | Tag |
| --- | ---: |
| `Forbidden` | 0 |
| `MayAliasExact` | 1 |
| `MustAliasExact` | 2 |

The language validator and primitive validator require a complete matrix with
no duplicate `(input, output)` pair and in-range indexes. They do not sort the
matrix. The hash therefore preserves the exact `source_aliases` vector order
provided by the source kernel. A valid permutation of the same complete set
has a different digest.

### Program buffers

The buffer sequence is encoded in `ProgramBuffer` vector order. Each record is:

~~~
u32(buffer.id.get())
buffer.origin
buffer.lifetime tag
dtype(buffer.dtype)
sequence(buffer.shape, u64)
static_access(buffer.access)
~~~

`BufferOrigin` is encoded as follows:

| Origin | Bytes after the origin tag |
| --- | --- |
| `Tensor(value)` | tag `0`, then `u64(value.get())` |
| `Scratch { ordinal, purpose }` | tag `1`, then `u32(ordinal)`, then `scratch_purpose(purpose)` |
| `FaultFlag` | tag `2` |

`BufferLifetime` tags are `ExternalValue = 0`, `ProgramScratch = 1`, and
`ProgramFaultFlag = 2`. `DType` tags are `F32 = 0` and `I32 = 1`.
`ScratchPurpose` tags are:

| Purpose | Tag |
| --- | ---: |
| `ReductionValues` | 0 |
| `ReductionIndices` | 1 |
| `ScanValues` | 2 |
| `ScanBlockTotals` | 3 |
| `SortValues` | 4 |
| `SortIndices` | 5 |

`static_access` contains the complete lowered view, in this order:

~~~
sequence(access.logical_extents, u64)
u64(access.offset_elements)
sequence(access.strides, u64)
u64(access.storage_bytes.get())
~~~

The buffer validator requires dense ordered `BufferId` values, unique origins,
an explicit shape, shape equal to `access.logical_extents`, valid address
arithmetic, storage coverage, and lifetime/type/shape combinations. These
checks do not change the byte order, but they ensure that the hashed record is
the authoritative static buffer contract. `ProgramBuilder` normally adds
external tensor buffers by the first occurrence in `kernel.inputs` followed by
`kernel.outputs`, reuses a buffer for a repeated tensor ID, and appends scratch
and fault buffers as the lowering algorithm requests them.

### Program stages and common records

The stage sequence is encoded in vector order. A `ProgramStage` record is:

~~~
u32(stage.id.get())
sequence(stage.dependencies, u32 stage IDs)
u64(stage.geometry.logical_lanes)
u32(stage.geometry.workgroup_lanes)
u64(stage.geometry.workgroups)
sequence(stage.bindings, encode_binding)
sequence(stage.synchronization, encode_synchronization)
sequence(stage.atomics, encode_atomic)
optional fault: bool(false), or bool(true) followed by encode_fault
encode_stage_resources(stage.resources)
encode_stage_kind(stage.kind)
~~~

`BufferBinding` is encoded as:

~~~
u32(binding.buffer.get())
dtype(binding.dtype)
access mode tag
static_access(binding.view)
~~~

`AccessMode` tags are `Read = 0`, `Write = 1`, `ReadWrite = 2`, and
`ReadWriteAtomic = 3`.

`SynchronizationPoint` is encoded as `u32(after_step)`, a scope tag, and a
memory-semantics tag. `SynchronizationScope` uses `Workgroup = 0` and
`DispatchBoundary = 1`. `MemorySemantics` uses
`SharedAcquireRelease = 0` and `GlobalAcquireRelease = 1`.

`AtomicContract` is encoded as:

~~~
u32(atomic.buffer.get())
dtype(atomic.dtype)
atomic operation tag
owned atomic ordering tag
atomic address-domain tag
~~~

The operation tags are `Exchange = 0`, `Add = 1`, `Minimum = 2`, and
`Maximum = 3`. `OwnedAtomicOrdering` uses
`Relaxed = 0`, `Acquire = 1`, `Release = 2`,
`AcquireRelease = 3`, and `SequentiallyConsistent = 4`. The address-domain
tags are `SingleFaultFlag = 0`, `TensorElements = 1`, and `HistogramBins = 2`.

`FaultContract` is encoded as:

~~~
u32(fault.flag.get())
fault reason tag
i32(fault.code)
bool(fault.guard_before_address)
encode_atomic(fault.publish)
~~~

`FaultReason` tags are `ArithmeticDomain = 0`, `IndexOutOfBounds = 1`, and
`HistogramBinOutOfBounds = 2`. The validator requires the guard flag, the
published atomic to target the same flag, an int32 release exchange in the
single-fault-flag domain, and that publication to appear in the stage atomic
list. The hash records all of those facts, including the failure code and the
guard bit.

Stage resources are always encoded after the optional fault and before the
stage kind:

~~~
u64(resources.flops.get())
u64(resources.integer_operations)
u64(resources.atomic_operations)
u64(resources.shared_bytes_per_workgroup.get())
u64(resources.private_bytes_per_lane.get())
u32(resources.maximum_workgroup_lanes)
~~~

The program aggregate at the end of the top-level stream uses the analogous
fixed order:

~~~
u64(resources.total_flops.get())
u64(resources.total_integer_operations)
u64(resources.total_atomic_operations)
u64(resources.persistent_scratch_bytes.get())
u64(resources.fault_bytes.get())
u64(resources.peak_shared_bytes_per_workgroup.get())
u64(resources.peak_private_bytes_per_lane.get())
u32(resources.maximum_workgroup_lanes)
~~~

The validator requires dense ordered `StageId` values and exactly one
dependency on the immediately preceding stage, or no dependency for stage
zero. It also checks launch geometry, binding references and views, atomics,
fault ABI, kind-specific invariants, exact resource formulas, synchronization
counts, and aggregate resource maxima/sums. Stage and binding vectors are
encoded exactly as stored. The validators enforce the algorithmic order where
the contract requires it, but the hash writer itself does not sort a vector.

## Stage-kind encodings

`stage_kind` starts every variant with an explicit stable tag. The tags and
payloads are:

| Tag | Variant | Payload after the tag |
| ---: | --- | --- |
| 0 | `ScalarMap { template }` | `kernel_template(template)` |
| 1 | `Fill { value }` | `literal(value)` |
| 2 | `Copy { reason }` | `CopyReason` tag, currently `ScatterBase = 0` |
| 3 | `FixedTreeReduce(stage)` | Reduction fields below |
| 4 | `FixedTreeScanLocal(stage)` | Local-scan fields below |
| 5 | `ScanUniformCombine(stage)` | Uniform-combine fields below |
| 6 | `TiledContraction(stage)` | Contraction fields below |
| 7 | `Gather { axis, bounds }` | `usize(axis)`, `IndexBounds` tag |
| 8 | `Scatter { axis, bounds, conflict }` | `usize(axis)`, `IndexBounds` tag, `ScatterConflict` |
| 9 | `HistogramClear` | No payload |
| 10 | `HistogramAccumulate { ... }` | Histogram fields below |
| 11 | `StableSortInitialize { network }` | `sort_network(network)` |
| 12 | `StableSortCompareExchange(stage)` | `sort_network`, merge width, compare distance |
| 13 | `StableSortFinalize { network, emit_indices }` | `sort_network`, `bool(emit_indices)` |
| 14 | `Philox4x32_10(contract)` | Philox fields below |
| 15 | `IndexMap(spec)` | Index-map fields below |

### Scalar templates and literals

`ScalarLiteral::F32Bits(bits)` is tag `0` followed by `u32(bits)`, preserving
the exact binary32 representation, and `ScalarLiteral::I32(value)` is tag `1`
followed by `i32(value)`.

`kernel_template` encodes the complete `recipe_core::KernelTemplate`, not just
its ID:

~~~
u64(template.id.get())
sequence(template.index_space.dimensions(), u64 extent.get())
sequence(template.inputs,
    u64(input.id.get()), dtype(input.dtype), core_static_access(input.access))
sequence(template.outputs,
    u64(output.id.get()), dtype(output.dtype), core_static_access(output.access))
sequence(template.program.inputs,
    u64(input.id.get()), dtype(input.dtype))
sequence(template.program.constants,
    u64(constant.id.get()), literal(constant.value))
sequence(template.program.instructions,
    u64(instruction.result.get()), dtype(instruction.dtype),
    scalar_opcode(instruction.opcode),
    sequence(instruction.operands, u64 operand.get()))
sequence(template.program.outputs, u64 value IDs)
sequence(template.alias_rules,
    u64(rule.input.get()), u64(rule.output.get()),
    alias_permission(rule.permission))
~~~

`core_static_access` differs from the lowered `static_access` by design. The
core access is paired with the kernel index-space dimensions, so it contains
only `u64(offset_elements)`, a length-prefixed `u64` stride sequence, and
`u64(storage_bytes.get())`; it has no logical-extents field.

`ScalarOpcode` is `#[non_exhaustive]`. Rather than assign a numeric fallback
that could collide with a future opcode, `scalar_opcode` formats the opcode
with `format!("{value:?}")` and writes that UTF-8 variant spelling through
`bytes`, including its 16-byte length prefix. The derived spelling is therefore
part of the version-2 canonical schema. Renaming a variant, changing its
`Debug` spelling, or adding a variant with a different spelling changes the
digest; a deliberate schema/domain version change is required if compatibility
with old digests is not intended.

### Trees, reductions, scans, and contraction

`fixed_tree` writes `u32(tree.lanes)`, then a length-prefixed step sequence.
Each `TreeStep` is a phase tag followed by `u32(stride)`. `TreePhase` tags are
`Reduction = 0`, `ScanUpsweep = 1`, and `ScanDownsweep = 2`.

For `FixedTreeReduce`, after tag `3`, the writer appends:

~~~
u32(stage.pass)
reduce_operator(stage.operator)
reduce_result(stage.result)
dtype(stage.dtype)
u64(stage.sequences)
u64(stage.input_width)
u64(stage.output_width)
sequence(stage.reduced_axes, usize)
fixed_tree(stage.tree)
ReductionPadding::OperatorIdentity tag (0)
ReductionTieBreak::LowestLogicalIndex tag (0)
~~~

`ReduceOperator` tags are `Sum = 0`, `Product = 1`, `Minimum = 2`,
`Maximum = 3`, `Any = 4`, and `All = 5`. `ReduceResult` tags are
`Value = 0`, `Index = 1`, and `ValueAndIndex = 2`.

For `FixedTreeScanLocal`, after tag `4`, the order is:

~~~
u32(stage.level)
reduce_operator(stage.operator)
dtype(stage.dtype)
u64(stage.sequences)
u64(stage.input_width)
u64(stage.output_width)
usize(stage.axis)
bool(stage.reverse)
scan_mode(stage.mode)
fixed_tree(stage.tree)
~~~

`ScanStageMode` is `UserInclusive` tag `0`, `UserExclusive { identity }`
tag `1` followed by `literal(identity)`, or
`HierarchyExclusiveIdentity` tag `2`.

For `ScanUniformCombine`, after tag `5`, the order is
`u32(level)`, `reduce_operator`, `dtype`, `u64(sequences)`, `u64(width)`,
`u32(block_lanes)`, and `bool(reverse)`.

For `TiledContraction`, `contraction` writes:

~~~
sequence(stage.spec.batch_axes, each pair as usize(first), usize(second))
sequence(stage.spec.contract_axes, each pair as usize(first), usize(second))
dtype(stage.dtype)
u64(stage.output_elements)
u64(stage.contracted_elements)
ContractionStrategy::Direct tag (0), or Staged tag (1)
u32(stage.tile.output_x)
u32(stage.tile.output_y)
u32(stage.tile.reduction)
bool(stage.canonical_contracted_order)
~~~

`canonical_contracted_order` is hashed even though validation requires it to be
true. The tile dimensions and strategy are also hashed because they select the
physical fixed-order implementation, not merely a scheduling hint.

### Gather, scatter, histogram, and index map

`IndexBounds` tags are `Reject = 0`, `Clamp = 1`, and `Wrap = 2`.
`Gather` writes its axis as `usize` followed by the bounds tag. `Scatter`
writes the same axis and bounds, then `scatter_conflict`:

~~~
UniqueIndices -> tag 0
Atomic { operation, ordering } -> tag 1, atomic_operation, language_atomic_ordering
~~~

Both language-level operation and ordering use the tags
`Exchange = 0`, `Add = 1`, `Minimum = 2`, `Maximum = 3`, and
`Relaxed = 0`, `Acquire = 1`, `Release = 2`, `AcquireRelease = 3`,
`SequentiallyConsistent = 4` respectively.

`HistogramClear` has only tag `9`. `HistogramAccumulate` (tag `10`) appends
`u32(bins)`, `bool(weighted)`, a bin-mapping tag, and an owned-ordering tag.
`HistogramBinMapping` uses `I32Direct = 0` and
`F32TruncateTowardZero = 1`.

`IndexMap` (tag `15`) appends `i32(start)`, `i32(element_step)`,
`i32(iteration_step)`, then a presence boolean and `i32(modulus)` only when
the optional modulus is present. The language and primitive validators require
a positive modulus when present and the checked arithmetic fault contract.

### Stable sort

`sort_network` is shared by all three sort variants. Its order is:

~~~
usize(network.axis)
u64(network.axis_length)
u64(network.padded_axis_length)
u64(network.slices)
sort_direction(network.direction)
bool(network.requested_stable)
SortTieBreak::OriginalAxisIndexAscending tag (0)
FloatSortOrder::Ieee754TotalOrder tag (0)
SortPadding::AfterAllValidElements tag (0)
~~~

`SortDirection` uses `Ascending = 0` and `Descending = 1`.

`StableSortCompareExchange` (tag `12`) appends that network, then
`u64(merge_width)` and `u64(compare_distance)`. The validator requires a
least-power-of-two padded axis, the total-order float policy, original-index
tie breaking, and valid bitonic phase widths. `StableSortFinalize` (tag `13`)
also hashes whether indices are emitted.

### Philox random contract

`Philox4x32_10` (tag `14`) appends the complete Recipe-owned random contract:

~~~
u64(contract.key.seed_low)
u64(contract.key.seed_high)
u64(contract.key.stream)
random_distribution(contract.distribution)
u8(contract.rounds)
u32(contract.multiplier_0)
u32(contract.multiplier_1)
u32(contract.weyl_0)
u32(contract.weyl_1)
for the four fixed counter words in array order: one counter-word tag each
bool(contract.fold_kernel_id_into_key)
bool(contract.fold_run_id_into_key)
UniformI32Mapping tag
NormalF32Mapping tag
~~~

The fixed counter-word tags are `ElementLow = 0`, `ElementHigh = 1`,
`IterationXorStreamLow = 2`, and `IterationXorStreamHigh = 3`.
`UniformI32Mapping::UnbiasedMultiplyHighWithCounterRejection` and
`NormalF32Mapping::OwnedBoxMullerV1` are currently the sole variants and both
use tag `0`.

`RandomDistribution` encodes `UniformF32` as `0`, `NormalF32` as `1`,
`BernoulliI32 { probability_bits }` as `2` followed by `u32(probability_bits)`,
or `UniformI32 { low, high_exclusive }` as `3` followed by the two `i32`
values. The source key, constants, distribution mapping, counter layout, and
run/kernel folding switches are all identity-relevant. Runtime `RunId` and
loop iteration values are not included because they are dynamic inputs to the
same immutable random stage, not part of its static lowered program.

## SHA-256 implementation

`sha256` is a self-contained standard SHA-256 implementation rather than a
crate call. It uses the standard eight initial words and 64 round constants,
builds the message schedule with the SHA-256 small-sigma functions, executes
the 64 choice/majority rounds with wrapping `u32` arithmetic, and emits eight
big-endian words for 32 output bytes.

Padding follows the SHA-256 bit-level contract:

1. Convert `input.len()` to the zero-extended 128-bit helper representation,
   multiply by eight with wrapping `u128` arithmetic, and keep the low eight
   bytes of its big-endian form as the SHA-256 64-bit bit length.
2. Copy the input into a new vector, append `0x80`, and append enough zero bytes
   for the length field to begin at byte offset `56` of a 64-byte block.
3. Append the eight-byte big-endian bit length and process every exact 64-byte
   block.

The message schedule reads each initial word as big-endian, while all fields
fed into the message were written by the canonical writer in their specified
little-endian or raw-byte forms. `sha256` has no error return. Allocation
failure is a process-level failure, not a `LoweringError` path. Inputs larger
than the SHA-256 64-bit length domain would have a wrapped length field, but a
`LoweredProgram` already has to exist in memory and the implementation follows
the standard low-64-bit SHA-256 length representation.

## What is and is not canonical

The implementation is canonical for the materialized program record and its
stored vector order. It does not perform normalization or sorting:

* Source alias records, buffers, stages, bindings, synchronization points,
  atomics, tree steps, axis lists, instruction operands, and all other slices
  are emitted in their existing order.
* `ProgramBuilder` and the lowering algorithms establish the normal order:
  dense buffers, dense serial stage IDs, immediately previous-stage
  dependencies, fixed collective trees, fixed sort phases, and fixed Philox
  counter arrays.
* Validators reject malformed structure and require exact algorithmic values,
  but they do not generally make an otherwise valid vector permutation equal
  to the original. A permutation that survives validation is a distinct
  program identity.
* Numeric enum values are never taken from Rust discriminants. Every closed
  enum used by the writer has an explicit tag mapping. Adding a `StageKind`
  variant or changing a match mapping requires source changes before the code
  can compile. The non-exhaustive `ScalarOpcode` path intentionally uses its
  stable debug spelling instead of a numeric fallback.
* `LoweringHardware` is not serialized directly. Its effect is represented by
  selected workgroup and collective geometry, tile strategy, synchronization,
  and exact resource bounds in the resulting program. Two hardware inputs that
  produce identical lowered fields produce the same program digest.
* Raw source primitive fields are not serialized as a second copy. Their
  lowered meaning appears in `StageKind`, buffers, bindings, faults, and
  resources. A source detail that has no effect on the resulting lowered
  contract does not affect this digest.
* Tensor data, addresses, pointer values, target/backend labels, compiler
  versions, runtime IDs, loop iterations, and realized binary bytes are outside
  this module. Their identities are owned by later or separate contracts.

Changing the domain, schema, field order, width, enum tag, opcode spelling, or
any materialized field is an identity-format change. The current domain suffix
`v2` and `LOWERED_PROGRAM_SCHEMA_VERSION = 2` make such changes explicit
rather than silently colliding with older program digests.

## Callers and consumers

The hash function has one constructor caller and one validation caller inside
`recipe-primitives`:

| Path | Use of this module |
| --- | --- |
| `lower.rs::ProgramBuilder::finish` | Computes the initial digest after all buffers, stages, and aggregate resources are fixed, then validates the completed program. |
| `validate.rs::validate` | Recomputes the digest and rejects a stale or mutated `LoweredProgram` at `digest`. |
| `model.rs::LoweredProgram::canonical_digest` | Public recomputation method that delegates to `program_digest`. |

The returned program then crosses these concrete boundaries:

1. `ops/src/primitive.rs::lower_primitive` and the public facade call
   `recipe_primitives::lower` and return the self-validating program. They do
   not recompute or replace its digest.
2. `planner/src/planner.rs::lower_programs` lowers every graph node and calls
   `program.validate()` again. `stage_template_identity` hashes the program
   digest with `recipe-planner-stage-template-v1`, the source kernel ID, and
   the stage ordinal, then uses the first eight digest bytes as a nonzero
   `KernelTemplateId`. The planner collision-checks these identities.
3. The planner graph identity (`recipe-planner-graph-v7`) includes every
   lowered program digest and the ordered stage-template IDs. Therefore a
   changed primitive contract changes the graph identity and all candidate
   identities derived from that graph.
4. Each deferred `ArtifactBuildRecipe` stores the 32 digest bytes as
   `ArtifactBuildProvenance::program_digest`, alongside the source kernel and
   stage ordinal. The planner's artifact-build contract digest includes this
   provenance, so it cannot be removed without changing the contract digest.
5. `kernel/src/stage.rs::validate_contract` requires the complete program to
   validate and compares `build.provenance.program_digest` with
   `Digest::new(program.digest.bytes())`. It also derives the same
   stage-scoped identity from the program digest before emitting target LLVM.
   A stale digest is an invalid stage contract, not a reason to lower a nearby
   stage.
6. `prepare/src/production.rs::lower_deferred_stage` selects the exact
   `LoweredProgram` whose `source_kernel` and digest equal the build
   provenance. If there is no exact match, preparation returns
   `NativePrepareError::InvalidCandidate` and does not compile a substitute.
7. Core artifact validation rejects zero program or contract digests in a
   deferred build, and later draft, realization, bundle, and remote manifest
   identities carry the build provenance transitively. These later hashes are
   distinct domains and are not alternate implementations of this module.

The end-to-end meaning is therefore:

~~~
one validated primitive source
  -> one ordered backend-neutral lowering
  -> one content digest
  -> one stage-scoped identity and provenance
  -> one exact deferred build contract
  -> one verified target realization
~~~

Any mismatch is observable at the boundary that owns it. The primitive hash
does not retry, search for a compatible program, repair vector order, or hide a
failed transition.

## Failure boundaries and invariants

`program_digest` itself is infallible and does not call `validate`. That is
intentional: validation needs to recompute a digest for a possibly malformed or
mutated value. The surrounding failure boundaries are:

* Invalid source language, missing tensors, invalid hardware, static-address
  overflow, scratch/resource overflow, or an invalid stage construction stop in
  `recipe_primitives::lower` before a program is returned.
* A program that reaches `ProgramBuilder::finish` with bad structure is hashed
  once, rejected by `LoweredProgram::validate`, and returned as
  `InvalidLoweredProgram` with joined validation details.
* A later mutation of a hashed field causes `validate` to report a digest
  mismatch in addition to any structural errors caused by that mutation.
* A later mutation of only `digest` causes only the digest equality check to
  fail. Recomputing `canonical_digest` never trusts the stale field.
* Planner, kernel, prepare, and core artifact checks preserve the same
  fail-closed behavior with their own error kinds. They do not invoke a second
  primitive hash algorithm or select a fallback implementation.

The invariant that makes the identity useful is:

~~~
validated program A == validated program B in every encoded field and vector
order
    iff
canonical bytes(A) == canonical bytes(B)
    -> expected equal ProgramDigest values
~~~

The converse is the usual cryptographic collision assumption, not a property
proved by this module. Domain separation, explicit widths, length framing,
fixed enum tags, and complete contract coverage make accidental byte-stream
ambiguity and cross-domain reuse visible without pretending that SHA-256 is a
semantic validator.
