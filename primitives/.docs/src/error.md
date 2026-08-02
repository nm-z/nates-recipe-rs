# Primitive lowering errors

`recipe-primitives` has two error surfaces. `LoweringError` is the fail-fast
result of constructing a backend-neutral program. `ProgramValidationError` is
the structured result of checking an already constructed `LoweredProgram`.
Both types are public from `primitives/src/lib.rs`.

The crate is a pure, backend-neutral lowering boundary. A failed lowering does
not allocate a device resource, emit a stage, publish a fault flag, or return a
partial program. The mutable `ProgramBuilder` is dropped on an error. A
successful `lower` call returns a digest-bearing `LoweredProgram` that has
already passed the same structural validator exposed by
`LoweredProgram::validate`.

## Public surface

### `LoweringErrorKind`

`LoweringErrorKind` is `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, and
`#[non_exhaustive]`. Its variants are:

| Kind | Meaning in this crate | Primary construction sites | State consequence |
| --- | --- | --- | --- |
| `InvalidLanguage` | The source `PrimitiveKernel` failed `recipe-language` validation, or an impossible empty minimum/maximum reduction reached the lowerer. | `From<LanguageError> for LoweringError` in `error.rs`; `lower_empty_reduction` in `lower.rs` | No `ProgramBuilder` is created for a source validation failure. The explicit empty minimum/maximum guard also returns before emitting a stage. |
| `MissingTensor` | A `ValueId` required by lowering is absent from the tensor index or has no corresponding program buffer. | `tensor`; `ProgramBuilder::tensor_buffer` | The current lowering aborts before the stage or binding that needs the value can be emitted. |
| `ArithmeticOverflow` | A checked conversion, multiplication, addition, index-space calculation, resource bound, or identifier calculation cannot be represented in the fixed integer type used by the lowered contract. | `ProgramBuilder` conversion helpers, `overflow`, `aggregate_resources`, `checked_mul`, and the primitive-family lowerers | The exact static contract cannot be represented, so lowering returns no program. No saturating substitute is used for these failures. |
| `InvalidStaticAccess` | A statically derived tensor view cannot be formed under the required rank relationship. | `broadcast_access` | The elementwise stage is not emitted because its input address contract is undefined. |
| `InvalidLoweredProgram` | Measured hardware limits are malformed, a builder lookup finds an absent buffer, an internal lowering invariant is broken, or the finished program fails canonical validation. | `validate_hardware`; `ProgramBuilder::buffer`; `lower_reduce`; `ProgramBuilder::finish` | The candidate program is rejected before it crosses the planner or realization boundary. |

The enum is intentionally non-exhaustive. Callers must preserve a catch-all
when matching it, and they should use `Display` when they need the complete
detail and optional source identity.

### `LoweringError`

`LoweringError` is `Clone`, `Debug`, `PartialEq`, and `Eq`. It stores four
public fields:

```text
kind: LoweringErrorKind
detail: String
kernel: Option<KernelTemplateId>
value: Option<ValueId>
```

`LoweringError::new(kind, detail)` initializes the two identity fields to
`None`. `for_kernel` and `for_value` are consuming builders, so a construction
site can attach the source kernel and value without changing the error kind or
detail. `From<LanguageError>` copies the language error's detail, kernel, and
value and changes only its kind to `InvalidLanguage`.

`LoweringError` implements `std::error::Error`. Its `Display` output is the
debug spelling of the kind, a colon, and the detail. A present kernel is then
rendered as ` [kernel <id>]`, and a present value as ` [value <id>]`, in that
order. Absent identities add nothing. `LoweringResult<T>` is exactly
`Result<T, LoweringError>`.

### `ProgramValidationError`

`ProgramValidationError` is `Clone`, `Debug`, `PartialEq`, and `Eq`. It is a
pair of public strings:

```text
path: String
detail: String
```

`ProgramValidationError::new(path, detail)` is the only constructor. It stores
both arguments after conversion to `String`. `Display` renders
`<path>: <detail>`. It is not wrapped in `LoweringError` by the validator
itself, and it does not carry kernel or value identity. A caller receives all
failures found in one validation pass as `Vec<ProgramValidationError>`.

## Lowering entry and propagation

The public `lower` function in `primitives/src/lower.rs` accepts a
`PrimitiveKernel`, a `BTreeMap<ValueId, &Tensor>`, and measured
`LoweringHardware`. It performs these operations in order:

1. `validate_hardware` checks the measured limits.
2. `kernel.validate(tensors)` checks language-level tensor, arity, shape,
   alias, and primitive invariants. Its `LanguageError` is converted with
   `LoweringError::from`.
3. `ProgramBuilder::new` records the source kernel and hardware.
4. Every input and output is looked up with `tensor` and added as an external
   program buffer.
5. The `PrimitiveKind` dispatch calls exactly one family lowerer.
6. `ProgramBuilder::finish` aggregates resources, computes the canonical
   digest, validates the complete program, and returns it.

The `?` operators in this chain preserve a `LoweringError` unchanged. Family
lowerers use the same result type, so an error from a tensor lookup, binding,
scratch allocation, stage push, checked arithmetic helper, or family-specific
operation reaches `lower` without a second wrapper. The only conversion in
the entry path is `LanguageError -> LoweringError`.

### Entry checks

`validate_hardware` returns `InvalidLoweredProgram` with `kernel` set when any
of these conditions is false:

* `subgroup_lanes` is nonzero;
* `subgroup_lanes` is a power of two;
* `maximum_workgroup_lanes` is at least `subgroup_lanes`; and
* `maximum_shared_memory_per_workgroup` is nonzero.

The detail includes the complete `LoweringHardware` value. This check rejects
an unusable measured profile before any program buffer is added.

`tensor` returns `MissingTensor` with both kernel and value set when the input
or output `ValueId` is absent from the supplied tensor index. In the normal
public path, `PrimitiveKernel::validate` reports the same absence first as a
language `UnknownTensor`; the lookup remains the direct invariant check used
by the builder.

## Construction sites by kind

The following list covers every `LoweringErrorKind` construction in the
primitive lowerer and the helper that propagates it.

### `InvalidLanguage`

* `From<LanguageError>` in `error.rs` consumes the language error and copies its
  `detail`, `kernel`, and `value`. `lower` uses this conversion immediately
  after `kernel.validate(tensors)`.
* `lower_empty_reduction` returns this kind, with the source kernel attached,
  when a zero-width reduction requests `Minimum` or `Maximum`. The language
  layer is required to reject that combination, so this branch records the
  violated source-language invariant rather than choosing an identity.

### `MissingTensor`

* `tensor` reports `tensor <value> is absent from the lowering index`, with
  kernel and value context.
* `ProgramBuilder::tensor_buffer` reports `tensor <value> has no program
  buffer`, with the builder's kernel and the value context. It is reached by
  `binding` and by family lowerers when they refer to an input or output after
  the initial buffer-registration pass.

Both sites stop the current family lowerer before it can construct a binding
that names an unowned buffer.

### `ArithmeticOverflow`

All arithmetic failures use checked operations or checked conversions and
return this kind with the source kernel attached. The details identify the
static quantity whose representation failed.

`ProgramBuilder` sites:

* `scratch` uses `elements.checked_mul(dtype.byte_width())` for scratch byte
  size and `scratch_ordinal.checked_add(1)` for its ordinal. Either failure is
  produced by `ProgramBuilder::overflow` as `<subject> overflowed its static
  representation`.
* `buffer` converts a `BufferId`'s `u32` to `usize`; conversion failure is
  reported as `buffer index conversion failed: <conversion error>`.
* `push_stage` converts the current stage vector length to `u32`; failure is
  `stage identifier conversion failed: <conversion error>`.
* `next_buffer_id` converts the buffer vector length to `u32`; failure is
  `buffer identifier conversion failed: <conversion error>`.
* `overflow` is the common builder helper for checked products and sums. It is
  used by scratch allocation, FLOP and resource bounds, reduction and scan
  widths, contraction geometry, sort dimensions, random work, index-map work,
  and `checked_mul`.

`aggregate_resources` sites:

* stage FLOP totals, integer-operation totals, and atomic-operation totals use
  checked additions;
* persistent scratch bytes and fault bytes use checked additions;
* peak shared/private values and the maximum workgroup width use maxima and do
  not overflow.

An overflow in either resource loop is returned by the free `overflow` helper
with the subject `total FLOP bound`, `total integer-operation bound`, `total
atomic-operation bound`, `persistent scratch bound`, or `fault bound`.

Family-specific sites:

* `lower_elementwise` converts each output extent to `ElementCount`, creates an
  `IndexSpace`, converts one-based `KernelInputId` and `KernelOutputId` values,
  and converts scalar-slot counts to `u64`. The details are respectively
  `elementwise index extent is invalid: ...`, `elementwise index space is
  invalid: ...`, `kernel input identifier conversion failed: ...`, `kernel
  output identifier conversion failed: ...`, and `elementwise scalar-slot
  conversion failed: ...`.
* `lower_elementwise` also uses the common overflow helper for per-lane FLOPs,
  total FLOPs, and the private-byte bound.
* `lower_reduce` uses the common helper for reduction width, scratch element
  counts, group counts, logical lanes, combines, FLOPs, and shared bytes. It
  converts each pass index to `u32`, producing `reduction pass conversion
  failed: ...` on failure.
* `lower_scan` uses the common helper for level output, block totals,
  workgroups, logical lanes, tree combines, active elements, inclusive
  combines, FLOPs, and shared bytes. It converts each hierarchy level to
  `u32`, producing `scan hierarchy-level conversion failed: ...`.
* `lower_contraction` uses the common helper for the contracted extent, output
  products, FLOPs, synchronization count, free-axis extent, workgroup width,
  and the checked multiplication used by its physical plan. It converts the
  synchronization count to `u32`, producing `contraction synchronization
  conversion failed: ...`.
* `lower_sort` uses `checked_next_power_of_two` for the padded axis and the
  common helper for scratch elements. A failure in the former is reported as
  `sort padded axis overflowed its static representation`.
* `lower_random` uses the common helper for Philox round work and scheduled
  work, and the builder overflow helper for the integer-operation bound.
* `lower_index_map` uses the common helper for its integer-operation bound.

All other family lowerers can propagate these errors through `scratch`,
`binding`, `tensor_buffer`, `push_stage`, or `checked_mul`; they do not create
another arithmetic variant.

### `InvalidStaticAccess`

`broadcast_access` aligns an input tensor's rank to an elementwise output rank.
Its checked subtraction of the input rank from the output rank returns
`InvalidStaticAccess` with the input value attached when the input rank is
larger. The detail is `broadcast input rank exceeds output rank`. No broadcast
stride view is returned in that case. The normal language validator rejects
the same shape relationship before this helper, so the error marks a violated
static-address invariant at the lowering boundary.

### `InvalidLoweredProgram`

* `validate_hardware` uses this kind for malformed measured limits, as described
  above.
* `ProgramBuilder::buffer` uses it when a representable `BufferId` indexes no
  current buffer. The detail is `buffer <id> is absent`, with the source kernel
  attached.
* `lower_reduce` uses it when a non-final reduction pass has no value scratch
  buffer. The detail is `non-final reduction omitted its value scratch`, with
  the source kernel attached. The builder cannot advance `current_value` to
  the next pass without that buffer.
* `ProgramBuilder::finish` calls `program.validate()` after computing the
  canonical digest. If validation returns one or more
  `ProgramValidationError`s, it joins their `Display` forms with `; ` and
  returns one `InvalidLoweredProgram` whose detail is that complete list and
  whose kernel is `program.source_kernel`.

This last path is the normal bridge from structured invariant failures to the
fail-fast `LoweringResult` used by the builder. It preserves every validator
path and detail in one string, but the `LoweringError` does not retain the
individual vector as a separate field.

## Structured program validation

`LoweredProgram::validate` delegates to `validate::validate`. It never mutates
the program. It appends failures to one vector in this order, then returns
`Ok(())` only when the vector is empty:

1. schema version;
2. source alias matrix;
3. buffers and static accesses;
4. stages, bindings, atomics, fault contracts, stage kinds, and resources;
5. program-level resource totals; and
6. canonical digest.

`require(condition, path, detail, errors)` is the common constructor. When the
condition is false it appends `ProgramValidationError::new(path, detail)` and
otherwise appends nothing. The direct `ProgramValidationError::new` calls
listed below cover the branches where conversion or arithmetic prevents a
boolean invariant from being evaluated.

### Schema and source aliases

* `schema_version` must equal `LOWERED_PROGRAM_SCHEMA_VERSION` (`2`). A
  mismatch reports `schema <actual> is unsupported; expected 2`.
* Each `source_aliases[index]` must have an input below
  `source_input_count` and an output below `source_output_count`; otherwise it
  reports `source alias pair exceeds the declared input/output arity`.
* Each input/output pair must be unique; duplicates report `source input/output
  alias pair appears more than once`.
* The alias vector length must equal the full input-count times output-count
  matrix; otherwise `source_aliases` reports `source alias matrix is
  incomplete`.

These checks ensure that planner alias handling sees exactly the source ABI
declared by the primitive kernel.

### Buffers and static accesses

For every `buffers[index]`:

* `id` must convert to the current vector index, enforcing dense ordered IDs.
* `shape` must be nonempty, so every buffer has an explicit rank.
* `access.logical_extents` must equal `shape`, making the canonical buffer
  view describe the complete shape.
* The origin key must be unique. Tensor origins are keyed by value ID, scratch
  origins by ordinal and purpose, and the fault flag has one fixed key.
* A tensor origin must have `ExternalValue` lifetime. A scratch origin must
  have `ProgramScratch` lifetime and a one-dimensional shape. A fault origin
  must have `ProgramFaultFlag` lifetime, `I32` type, and shape `[1]`. Any other
  combination reports `buffer origin, lifetime, type, and shape are
  inconsistent` at `.lifetime`.
* A valid fault flag must have exactly four storage bytes at
  `.access.storage_bytes`.

`validate_access` is used for both canonical buffer views and stage binding
views. It reports:

* `.strides`: extent and stride ranks differ;
* `.access`: the checked static address calculation overflows `u64`;
* `.storage_bytes`: the required byte span exceeds the declared storage;
* `.strides`: a non-atomic writable view is overlapping or broadcast.

The required byte span is computed from offset, each `(extent - 1) * stride`,
and the element width. Empty extents use required size zero. The writable
injectivity check applies only to `Write` and `ReadWrite` bindings. Atomic
bindings are intentionally excluded because their contract is checked by
`validate_atomics`.

### Stages and dispatch geometry

For every `stages[index]`:

* `id` must be the dense vector index, reporting `stage identifiers must be
  dense and ordered` at `.id` otherwise.
* Dependencies must be exactly the immediately preceding stage ID, or empty
  for stage zero. A mismatch reports `canonical lowering serializes stages
  with the immediately previous dispatch dependency`.
* `geometry.logical_lanes` and `geometry.workgroup_lanes` must both be
  nonzero.
* `geometry.workgroups` must equal the ceiling of logical lanes divided by
  workgroup lanes.

These errors prevent a planner or kernel emitter from accepting a stage whose
launch or serialization order differs from the immutable lowering contract.

### Bindings

For every `stages[index].bindings[binding_index]`:

* A buffer ID that cannot convert to a table index reports
  `buffer identifier cannot index the buffer table` at `.buffer`.
* A table index with no buffer reports `binding references an absent buffer` at
  `.buffer`.
* A present buffer must occupy the referenced dense slot, otherwise `binding
  references the wrong dense buffer slot`.
* Binding and buffer dtypes must match, otherwise `binding type differs from
  its buffer type`.
* Binding storage cannot exceed buffer storage, otherwise `binding view exceeds
  its buffer storage`.
* The binding view is then checked by `validate_access` with writable set only
  for `Write` and `ReadWrite` modes.

### Atomic and checked-fault contracts

`validate_atomics` requires each atomic tuple of buffer, domain, operation, and
ordering to be unique. A duplicate reports `atomic contract appears more than
once`. Every atomic must have a matching `ReadWriteAtomic` binding and matching
dtype, or it reports `atomic contract requires a ReadWriteAtomic binding` or
`atomic contract type differs from its binding`.

When a stage has a `FaultContract`, `validate_fault` requires:

* `guard_before_address` is true, preserving the guard-before-address safety
  rule;
* the declared flag equals the publication atomic's buffer;
* publication uses the `SingleFaultFlag` domain, `Exchange` operation, and
  `I32` dtype; and
* the publication atomic appears in the stage atomic list.

Failures report the corresponding `.fault.guard_before_address`,
`.fault.publish.buffer`, `.fault.publish`, or `.atomics` path. A checked fault
is therefore part of the stage ABI, not an exception thrown by this crate at
runtime. The successful program describes how a backend publishes a data
dependent fault flag.

### Stage-kind contracts

`validate_kind` applies the following exact checks:

* `ScalarMap`: the embedded `KernelTemplate::validate` errors are copied to
  `<stage>.kind.template`; its index-space element count must equal launch
  logical lanes; binding count must equal template inputs plus outputs plus an
  optional fault binding; and the scalar program's fault requirement must
  equal stage fault presence.
* `FixedTreeReduce`: tree lanes must be a power of two in `1..=1024`; steps
  must be the canonical descending-stride reduction tree; output width must
  equal input width divided by tree fan-in with ceiling; and tie-break must be
  `LowestLogicalIndex`.
* `FixedTreeScanLocal`: tree lanes and canonical Blelloch upsweep/downsweep
  steps are required; output width must equal input width divided by tree fan-in
  with ceiling.
* `TiledContraction`: contracted coordinates must have the canonical
  backend-independent order; tile dimensions must all be nonzero; and
  `output_x * output_y` must equal realized workgroup lanes.
* `Gather` and `Scatter`: `IndexBounds::Reject` must be equivalent to the
  presence of a checked fault contract. Other bounds must not carry that fault
  path.
* `HistogramAccumulate`: a fault contract with reason
  `HistogramBinOutOfBounds` is mandatory.
* Stable sort initialization and finalization validate the shared sort network.
  A compare-exchange stage validates it too, and additionally requires
  power-of-two merge width and compare distance, merge width no larger than
  padded axis length, and compare distance below merge width.
* `Philox4x32_10`: rounds and all four Recipe-owned constants must be exact
  (`0xd251_1f53`, `0xcd9e_8d57`, `0x9e37_79b9`, and `0xbb67_ae85`). The counter
  word order must be element low, element high, iteration XOR stream low,
  iteration XOR stream high, and both kernel-ID and run-ID key folding flags
  must be true.
* `IndexMap`: exactly two bindings are required, an optional modulus must be
  strictly positive, and an `ArithmeticDomain` fault contract is mandatory.
* `Fill`, `Copy`, `ScanUniformCombine`, and `HistogramClear` require exactly
  one, two, two, and one binding respectively.

### Tree and sort helpers

`validate_reduction_tree` and `validate_scan_tree` use the same lane range
`1..=1024` and report whether lane count or exact steps are wrong. Reduction
steps must descend through `TreePhase::Reduction`; scan steps must be the
canonical upsweep followed by reverse-order downsweep.

`validate_sort_network` requires a nonzero axis length, a power-of-two padded
length at least as large as the axis, and exactly `axis_length.next_power_of_two()`
padding. It also requires original-axis-index tie breaking, IEEE-754 total
ordering, and padding after all valid elements. The resulting detail states
that sort must remain stable and deterministic for equal values, NaNs,
signed-zero values, and padded keys.

### Stage resources and synchronization

`expected_stage_resources` recomputes FLOPs, integer operations, atomics,
shared bytes, private bytes, and maximum workgroup lanes from the stage kind
and geometry using checked arithmetic. If any checked expression cannot be
represented, it appends a direct error at `<stage>.resources` with detail
`stage resource arithmetic overflowed`. Otherwise a mismatch between the
stored and recomputed bounds reports `stage resource bounds differ from the
exact kind/geometry bounds: expected <computed>`.

The expected synchronization count is the fixed tree step count for reduction
and local scan, twice the staged contraction tile count for a staged
contraction, and zero for all other stage kinds. A contraction count that
cannot convert to `usize` appends
`synchronization count conversion failed: <conversion error>` at
`<stage>.synchronization`; an unrepresentable expected count appends
`synchronization count cannot be represented`. Any ordinary length mismatch
reports `synchronization count differs from the fixed algorithm`.

`validate_resources` then recomputes program-wide totals. Overflow in stage
resource totals reports `resources: program resource aggregation overflowed`.
Overflow in scratch or fault storage totals reports `resources: program storage
aggregation overflowed`. If the stored `ProgramResourceBounds` differs from
the exact aggregate, it reports
`resources: program resource bounds differ from exact aggregate <computed>`.

### Digest

The final check recomputes `program.canonical_digest()` and requires it to equal
the stored digest. A mismatch reports `digest: program digest does not match
its canonical contents`. This is the last validator step, so a program with a
stale digest can also report earlier structural failures in the same pass.

## Caller and callee boundaries

### Planner boundary

`planner::lower_programs` obtains common measured hardware from the discovery
profile, then calls `recipe_primitives::lower` once for each calculation graph
node. Any `LoweringError` is converted to `PlannerErrorKind::InvalidGraph`
with detail `primitive lowering failed for kernel <id>: <displayed error>`.
The planner then calls `program.validate()` again. A nonempty validation vector
is converted to the same planner kind with all displayed validation errors
joined by `; ` and prefixed by `primitive lowering for kernel <id> produced an
invalid program`.

The planner therefore stores a `LoweredProgram` only after both the lowering
result and an independent validation pass succeed. A primitive error aborts
the current graph's program-lowering phase; no placement, stage task, buffer
image, or artifact build recipe is produced for that failed kernel.

### Operation boundary

`ops::lower_primitive` selects a direct primitive recipe and then calls the
private `lower_recipe`, which calls `recipe_primitives::lower` only after the
recipe shape, dtype, arity, and kind match. A primitives `LoweringError` is
converted to `OperationErrorKind::PrimitiveLoweringFailed` using
`error.to_string()`. The operation-level wrapper therefore preserves the
formatted kind, detail, and optional identities as text, but does not retain a
typed `LoweringError` field. `lower_primitive` attaches the descriptor's
operation ID to the resulting operation error; the separate public
`lower_index_map` helper does not have a descriptor and therefore does not add
one.

The top-level `src/facade.rs` exposes `operations::lower_primitive` by direct
delegation to `recipe_ops::lower_primitive`. It does not catch or alter the
operation error.

### Realization boundary

`kernel::lower_stage` receives a complete `LoweredProgram` and calls
`program.validate()` in `kernel::stage::validate_contract`. It tests only
`.is_ok()`. Any primitives validation vector is therefore collapsed at this
boundary into the kernel crate's `LoweringErrorKind::InvalidStageContract`
with detail `lowered program failed canonical validation`. The kernel emitter
does not realize a stage until this check, the artifact-build validation,
target validation, options validation, program digest check, source-kernel
check, and stage-identity checks all pass.

This is a distinct error type from `recipe_primitives::LoweringError`; the
primitive error document covers the source validator and records the exact
point where realization intentionally reduces it to a generic stage-contract
failure.

## End-to-end failure role

The normal successful path is:

```text
public operation or planner request
  -> recipe_primitives::lower
  -> immutable LoweredProgram::validate
  -> planner stage/task and buffer contracts
  -> kernel::lower_stage revalidation
  -> target-specific realization
```

Each primitives error stops this path before the next state transition:

* `InvalidLanguage` stops source interpretation before builder state exists;
* `MissingTensor` stops a missing value from becoming a buffer binding;
* `ArithmeticOverflow` stops an unrepresentable static dimension, ID, resource
  bound, or dispatch contract from being truncated or saturated;
* `InvalidStaticAccess` stops an invalid broadcast view from becoming an
  address calculation; and
* `InvalidLoweredProgram` stops malformed hardware input or a violated
  builder/validator invariant from reaching planning or realization.

`ProgramValidationError` has the same end-to-end role for an already formed
program, but it is aggregating and non-mutating. The caller can report every
invalid path in one pass, while the program remains rejected. It does not
represent a device-side data fault. Device-side index, arithmetic, and
histogram faults are successful stage contracts containing a four-byte fault
flag and an atomic publication rule; they are handled later by planner and
executor fault readback, not by constructing a `LoweringError` here.
