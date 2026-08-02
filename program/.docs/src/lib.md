<!--
Intent: describe the complete public contract implemented by program/src/lib.rs.
The program crate owns the static lifecycle envelope around a validated
CalculationGraph. Scalar SSA remains owned by recipe-core; its construction,
validation, graph codec, lowering, and runtime consumers are traced here so
the boundary is explicit.
-->

# `recipe-program` facade

`program/src/lib.rs` is the complete implementation of the `recipe-program`
crate. It adds a static lifecycle envelope to a
[`recipe_language::CalculationGraph`](../../../language/src/graph.rs): one
finite or gracefully stopped unbounded loop, one explicit activation domain
for every source kernel, and optional four-byte scalar telemetry emissions.
It does not unroll the graph, choose a device, allocate memory, build a native
image, or execute a task.

The source-level crate contract is:

```rust
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
```

`program/Cargo.toml` depends only on `recipe-core`, `recipe-language`, and
`recipe-ogdl`. Core supplies typed IDs and the loop-domain values. Language
supplies the calculation graph and its `RecipeIR` codec. OGDL supplies the
ordered syntax tree, parser, and graph-building errors. The program crate
combines those values into a two-root `RecipeProgram` document and owns the
additional lifecycle and metric invariants.

The intended boundary is:

```text
typed scalar programs and primitive kernels
    -> recipe-language::CalculationGraph
    -> recipe-program::StaticCalculationProgram
       (loop horizon + kernel domains + user metric domains)
    -> recipe-planner::plan_program_candidates
    -> recipe-prepare::Preparer::prepare_program
    -> immutable FinalizedBundle
    -> executor/native backends: init -> loop -> exit
```

The object passed to the planner is still a description. Preparation and
execution consume the finalized core bundle, not a mutable program object.

## Root surface and ownership

The crate has no public submodules. The root imports implementation
dependencies and reexports only the schedule types needed by callers:

```rust
pub use recipe_core::{IterationDomain, LoopIterations};
```

The remaining root items are declared in `program/src/lib.rs`:

```text
KernelIterationDomain
MetricEmission
StaticCalculationProgram
ProgramErrorKind
ProgramError
ProgramResult<T>
```

`KernelTemplateId`, `MetricId`, and `ValueId` are not redefined here. They are
the core ID newtypes used in the public fields. Each ID is a transparent
typed `u64` wrapper with `new`, `get`, ordering, hashing, and display. The core
ID constructors permit zero for deterministic assembly; the program contract
reserves metric ID zero and metric value ID zero explicitly during validation.

`recipe::engine` exposes this crate under the stable facade path
`recipe::engine::program` through `src/facade.rs`. There is no second program
implementation behind that reexport.

## Lifecycle meaning

`CalculationGraph` is an acyclic collection of typed tensors and
placement-free primitive kernels. Its nodes describe calculations and value
edges, but not when a node runs. `StaticCalculationProgram` supplies the
missing schedule declaration:

```text
graph nodes (kernel IDs and value producers)
       + loop iterations (finite N or unbounded)
       + one IterationDomain per kernel
       + optional MetricEmission records
       = immutable static program contract
```

An `IterationDomain` is a nonempty, zero-based arithmetic progression. A
finite domain is half-open, `[first, end_exclusive)`, with a nonzero stride. An
unbounded domain has no invented terminal iteration. The graph remains one
copy regardless of the horizon. The planner later assigns each source kernel
to a device and projects these source domains onto loop tasks, transfers,
metrics, and exit dependencies.

Init admission and exit egress are not fields of this crate. They are derived
by planning from tensor boundary flags. Discovery, lowering, artifact
realization, allocation, and native-image loading happen before the loop;
`StaticCalculationProgram` describes only model calculations, activation
domains, and user metric declarations.

## Public records

### `KernelIterationDomain`

```rust
pub struct KernelIterationDomain {
    pub kernel: KernelTemplateId,
    pub domain: IterationDomain,
}
```

The record assigns exactly one source kernel to one activation progression. It
is `Clone + Copy + Debug + PartialEq + Eq`. The vector supplied to a
constructor may be in any order, but `new_with_metrics` sorts it by `kernel`
before storing it. The sorted representation makes `StaticCalculationProgram::domain`
an ordered binary search and gives the planner a deterministic input.

### `MetricEmission`

```rust
pub struct MetricEmission {
    pub metric: MetricId,
    pub value: ValueId,
    pub domain: IterationDomain,
}
```

This is a user-facing scalar declaration. `value` names a graph tensor that a
calculation produces. `domain` selects the iterations at which a nonblocking
four-byte device readback may publish the newest value to the metric mailbox.
The program layer does not allocate a slot or create a runtime task. The
planner creates those resources after it finds the producer and its resident
device copy. A metric value is deliberately not an external output, so metric
telemetry cannot replace exit egress.

The record is also `Clone + Copy + Debug + PartialEq + Eq`. Metric records are
sorted by ID in `new_with_metrics` and `with_metrics`; `metric` therefore uses
binary search.

### `StaticCalculationProgram`

```rust
pub struct StaticCalculationProgram {
    graph: CalculationGraph,
    iterations: LoopIterations,
    domains: Vec<KernelIterationDomain>,
    metrics: Vec<MetricEmission>,
}
```

All four fields are private. Callers cannot mutate a graph, horizon, domain
assignment, or metric list without passing through construction or
`with_metrics`, and every successful constructor returns a validated value.
The type derives `Clone + Debug + PartialEq + Eq`.

The fields mean:

| Field | Meaning | Canonical storage |
| --- | --- | --- |
| `graph` | Complete language calculation graph, including tensor boundary flags and primitive kernels | The graph's own sorted/validated contracts, not a second copy of scalar state |
| `iterations` | Total finite iterations or an unbounded lifetime | `LoopIterations::Finite(NonZeroU64)` or `LoopIterations::Unbounded` |
| `domains` | One source-kernel activation progression per graph node | Sorted by `KernelTemplateId` |
| `metrics` | Optional user scalar publications | Sorted by `MetricId` |

There is no default domain lookup, implicit all-iterations behavior, or
default metric. `every_iteration` is an explicit convenience constructor that
materializes `IterationDomain::every(iterations)` for every graph node.

## Loop values reexported from core

The program root reexports the exact core schedule types. Their constructors
and accessors are part of the program input contract.

### `LoopIterations`

`LoopIterations` is either:

```rust
Finite(NonZeroU64)
Unbounded
```

It provides `ONE`, `UNBOUNDED`, `new(u64) -> Option<Self>`,
`from_nonzero`, `finite`, `is_unbounded`, `iteration(index)`, `Display`, and a
default of `ONE`. A finite value rejects zero at `new`; an unbounded value has
no physical terminal iteration. `iteration(index)` returns `None` only when a
finite index is outside the horizon. The program serializer writes a finite
value as its canonical unsigned decimal or writes the literal `unbounded`.

### `IterationDomain`

`IterationDomain` stores a private first index, either a finite exclusive end
or an unbounded end, and a `NonZeroU64` stride. Its public constructors and
queries are:

```text
new(first, end_exclusive, stride) -> Option<IterationDomain>
unbounded(first, stride) -> Option<IterationDomain>
every(iterations) -> IterationDomain
first() -> IterationDomain
periodic(offset, period, iterations) -> Option<IterationDomain>
first_iteration() -> u64
end_exclusive() -> Option<u64>
is_unbounded() -> bool
stride() -> NonZeroU64
contains(iteration) -> bool
is_within(iterations) -> bool
```

`new` rejects zero stride and `first >= end_exclusive`. `unbounded` rejects
zero stride but accepts every first index. `every(Finite(n))` is
`[0, n)` with stride one, and `every(Unbounded)` is `[0, unbounded)` with
stride one. `first()` is `[0, 1)`. `periodic` chooses a finite or unbounded
constructor according to the supplied horizon.

`contains` checks the lower bound, finite upper bound, and modular stride.
`is_within` checks that a finite domain is nonempty and ends no later than a
finite loop, that an unbounded domain is allowed only inside an unbounded
loop, and that a finite domain is valid inside an unbounded loop. The program
validator calls `is_within` for every kernel and metric domain.

## Construction API

### `new`

```rust
pub fn new<I: Into<LoopIterations>>(
    graph: CalculationGraph,
    iterations: I,
    domains: Vec<KernelIterationDomain>,
) -> ProgramResult<Self>
```

`new` delegates to `new_with_metrics` with an empty metric vector. It does not
invent domains. A graph containing one node therefore requires one matching
`KernelIterationDomain` record, even when the caller intended the node to run
on every iteration. Use `every_iteration` when that is the intended contract.

### `new_with_metrics`

```rust
pub fn new_with_metrics<I: Into<LoopIterations>>(
    graph: CalculationGraph,
    iterations: I,
    domains: Vec<KernelIterationDomain>,
    metrics: Vec<MetricEmission>,
) -> ProgramResult<Self>
```

The generic horizon is converted once. Domains are sorted by kernel ID and
metrics by metric ID. The vectors are retained as values, then
`validate()` is called before returning. A failed validation does not return a
partially usable program.

### `every_iteration`

```rust
pub fn every_iteration<I: Into<LoopIterations>>(
    graph: CalculationGraph,
    iterations: I,
) -> ProgramResult<Self>
```

This constructor walks `graph.nodes`, creates one domain per node using
`IterationDomain::every(iterations)`, and delegates to `new`. It is used by the
legacy one-iteration planner and preparer entry points. It still validates the
graph and all domain assignments before returning.

### `with_metrics`

```rust
pub fn with_metrics(self, metrics: Vec<MetricEmission>) -> ProgramResult<Self>
```

The replacement list is sorted by metric ID, assigned to the program, and
validated. This is the only post-construction mutation surface, and it is
transactional from the caller's perspective: success returns the new
validated value, while an error returns no program value.

## Read-only accessors

All accessors are read-only and marked `#[must_use]` in the implementation:

```rust
graph(&self) -> &CalculationGraph
iterations(&self) -> LoopIterations
domains(&self) -> &[KernelIterationDomain]
domain(&self, kernel: KernelTemplateId) -> Option<IterationDomain>
metrics(&self) -> &[MetricEmission]
metric(&self, metric: MetricId) -> Option<MetricEmission>
```

`graph` and `iterations` are `const fn`. `domains` and `metrics` expose the
sorted slices without copying. `domain` and `metric` use binary search and
return `None` for an absent ID. They do not synthesize an all-iterations
domain or a metric default.

## Program validation

`StaticCalculationProgram::validate` is the semantic gate used by every
constructor and by both OGDL encoders. It accumulates graph errors through
the language error wrapper only at the graph boundary, then performs the
program-specific checks in this order.

### 1. Validate the calculation graph

`graph.validate()` checks tensor contracts, primitive kernels, scalar
programs, producer uniqueness, external boundaries, and acyclicity. A
language validation error is mapped to:

```rust
ProgramError::Graph(OgdlCodecError::InvalidGraph(error))
```

The wrapper retains the language error as the OGDL codec's source. No graph
default or alternate producer is selected.

### 2. Validate kernel domain assignments

The validator builds a `BTreeSet` of kernel IDs from `graph.nodes` and an
ordered `BTreeMap` of assignments while walking `self.domains`.

For every assignment it requires:

* `assignment.kernel` exists in the graph;
* the kernel has not already received a domain;
* `assignment.domain.is_within(self.iterations)` is true.

It then requires the assigned set to equal the graph kernel set. The first
missing kernel ID produces `MissingKernel`. This is a complete assignment,
not a sparse override table. Domains are also the source of truth used by the
planner, so a missing or duplicate assignment cannot be repaired later.

### 3. Validate first-use dependency ordering

The validator maps every produced tensor value to its source kernel. For each
consumer input that has a producer, it compares only the first iteration of
the two kernel domains. If a consumer starts earlier than its producer, it
returns `UninitializedDependency` with the value and both first indices.

This check proves that a consumer never reads a producer-owned value before
the producer's first activation. It deliberately does not replace the
language graph's dependency validation or attempt to simulate every stride
combination. The planner and finalized task validators enforce the complete
task dependency graph after placement.

### 4. Validate metric declarations

For each `MetricEmission`, the validator requires all of the following:

* `metric.get() != 0`. Zero is reserved and produces `InvalidMetricId`.
* The metric ID is unique. Repetition produces `DuplicateMetric`.
* `value.get() != 0`. Zero produces `InvalidMetricValue`.
* The value exists in the graph tensor index. Otherwise `UnknownMetricValue`.
* `tensor.shape.elements() == 1` and `tensor.storage_bytes == ByteCount::new(4)`.
  This is the exact one-element, four-byte mailbox ABI. The dtype is retained,
  and core's closed type domain means it is F32 or I32.
* `tensor.external_output` is false. Metrics remain loop-internal telemetry;
  an external output must be collected by an exit task instead.
* `emission.domain.is_within(self.iterations)` is true.
* A calculation node produces the value. A tensor with no producer produces
  `UnproducedMetricValue`.
* The producer domain covers every point selected by the emission domain.
  Failure produces `UncoveredMetricDomain`.

The producer lookup is built from the graph's output lists. Graph validation
has already rejected duplicate producers, so the map has one authoritative
kernel for each produced value.

### Domain coverage rule

The private `domain_covers(producer, emission)` helper is point-oriented. It
requires:

1. The emission first index is not before the producer first index.
2. The emission first index is congruent to the producer first index modulo
   the producer stride.
3. A finite emission's last selected point is strictly before a finite
   producer end. An unbounded emission cannot be covered by a finite producer.
4. Either the emission contains only its first point, or the emission stride is
   a multiple of the producer stride.

An unbounded producer may cover a finite or unbounded emission after the
alignment and stride checks. A finite producer may cover only a finite emission
whose selected last point stays below the producer end. This is stronger than
comparing just interval endpoints and prevents a metric from observing a
value on a producer-skipped iteration.

## Canonical OGDL representation

The program codec uses the ordered forest provided by `recipe-ogdl`. A valid
static program has exactly two roots in this exact order:

```text
RecipeProgram
RecipeIR
```

The first root is owned by `recipe-program`. The second is an unmodified
subtree produced by `CalculationGraph::to_ogdl_graph` and owned semantically
by `recipe-language`.

### `RecipeProgram` schema

Version 2 is emitted by the current encoder. Its fields are exact and each
field stores one child leaf value unless it is a collection:

```text
RecipeProgram
    schema
        StaticCalculationProgram
    version
        2
    iterations
        12                 # finite example, canonical decimal
    domains
        domain
            kernel
                1
            first
                0
            end_exclusive
                12
            stride
                1
    metrics
        metric
            id
                7
            value
                42
            first
                0
            end_exclusive
                12
            stride
                4
```

The comments above are explanatory and are not part of the canonical OGDL
text. `iterations` and every numeric field are unsigned decimal text. An
unbounded loop or domain uses the exact literal `unbounded`. The encoder
always writes the `metrics` collection for version 2, even when it is empty.
It writes domains and metrics in the sorted in-memory order.

Version 1 is a read compatibility form. Its exact program fields are
`schema`, `version`, `iterations`, and `domains`; it has no `metrics` field.
Decoding version 1 reconstructs an empty metric vector and then runs the same
constructor and validation path. The encoder never emits version 1.

### Encoding: `to_ogdl` and `to_ogdl_graph`

```rust
pub fn to_ogdl(&self) -> ProgramResult<String>
pub fn to_ogdl_graph(&self) -> ProgramResult<recipe_ogdl::Graph>
```

Both methods call `self.validate()` before building text. `to_ogdl` then calls
`to_ogdl_graph` and returns `Graph::to_canonical_string()`, so a successful
string is both semantically valid and canonical. The nested validation is
intentional: the graph-building API is independently safe when called
directly.

`to_ogdl_graph` performs these steps:

1. Create an empty OGDL graph and append the `RecipeProgram` root.
2. Append `schema`, `version`, and `iterations` fields.
3. Append `domains`, then one `domain` record for each sorted assignment with
   `kernel`, `first`, `end_exclusive`, and `stride` fields.
4. Append `metrics`, then one `metric` record for each sorted emission with
   `id`, `value`, `first`, `end_exclusive`, and `stride` fields.
5. Call `CalculationGraph::to_ogdl_graph()`, obtain its sole `RecipeIR` root,
   and recursively copy that subtree into the output as the second root.

Only OGDL graph construction can fail after semantic validation. Those
`GraphError` values become `ProgramError::Build`; no partially built forest is
returned.

The program layer does not reencode or interpret nested tensors, primitive
variants, or scalar instructions. It delegates those records to the language
codec and copies the resulting node tree. Consequently a change to the
`RecipeIR` schema is a language codec change, while a change to loop fields or
metric records is a program schema change.

### Decoding: `from_ogdl` and `from_ogdl_graph`

```rust
pub fn from_ogdl(input: &str) -> ProgramResult<Self>
pub fn from_ogdl_graph(source: &recipe_ogdl::Graph) -> ProgramResult<Self>
```

`from_ogdl` first calls `Graph::parse`. Parser failures become
`ProgramError::Syntax`. `from_ogdl_graph` then applies strict document checks
before constructing any typed program:

1. Require exactly two roots. The first must be `RecipeProgram`, and the
   second must be `RecipeIR`.
2. Require one leaf `RecipeProgram.version` field. Version `1` selects the
   four-field legacy schema; version `2` selects the five-field current
   schema. Any other text is `InvalidDocument`.
3. `exact_fields` rejects unknown fields, repeated fields, and missing fields.
   Field order is not significant inside a record, but the field set is exact.
4. Require the schema leaf to equal `StaticCalculationProgram`.
5. Parse `iterations` as a nonzero unsigned decimal or the literal
   `unbounded`.
6. Require every domain child to be named `domain` with exactly `kernel`,
   `first`, `end_exclusive`, and `stride` fields. Parse the IDs and numbers,
   then construct the finite or unbounded `IterationDomain`.
7. If version 2 includes `metrics`, require every child to be named `metric`
   with exactly `id`, `value`, `first`, `end_exclusive`, and `stride` fields.
   Version 1 cannot carry metrics because its exact field set omits them.
8. Copy the `RecipeIR` root into a fresh one-root OGDL graph and delegate to
   `CalculationGraph::from_ogdl_graph`. Language codec failures become
   `ProgramError::Graph`.
9. Call `new_with_metrics`, which sorts all records and reruns the complete
   static-program validation.

The private helpers make the structural strictness concrete:

* `node` rejects a referenced but absent `NodeId`.
* `require_text` checks a node label such as `RecipeProgram`, `domain`, or
  `metric`.
* `unique_field` requires exactly one named child.
* `exact_fields` rejects unknown, duplicate, or missing field names.
* `leaf_value` requires exactly one child and requires that child to be a leaf.
* `require_leaf_value` checks a canonical leaf such as the schema string.
* `parse_u64` rejects empty text, a leading plus, noncanonical leading zeroes,
  negative text, overflow, and all non-decimal spellings.
* `parse_loop_iterations` recognizes only `unbounded` or a valid nonzero
  unsigned count.
* `parse_iteration_domain` rejects a zero stride, an empty or reversed finite
  range, and an invalid unbounded constructor.

No parser default is used for a missing field, unknown version, omitted
version-2 metrics collection, malformed leaf, or invalid number.

## Error model

`ProgramResult<T>` is:

```rust
pub type ProgramResult<T> = Result<T, ProgramError>;
```

### `ProgramErrorKind`

The non-exhaustive contract enum currently contains:

| Kind | Produced when |
| --- | --- |
| `InvalidIterationDomain` | A kernel or metric domain is outside the loop, or OGDL domain construction is empty, reversed, or zero-stride |
| `UnknownKernel` | A domain names no graph kernel |
| `DuplicateKernel` | A kernel has more than one domain assignment |
| `MissingKernel` | A graph kernel has no domain assignment |
| `UninitializedDependency` | A consumer's first activation precedes its producer's first activation |
| `InvalidMetricId` | Metric ID zero is used |
| `DuplicateMetric` | A metric ID appears more than once |
| `UnknownMetricValue` | A metric references no graph tensor |
| `InvalidMetricValue` | Metric value zero, non-four-byte/non-singleton tensor, or external output |
| `UnproducedMetricValue` | The named tensor has no calculation producer |
| `UncoveredMetricDomain` | The producer domain does not cover every emitted point |
| `InvalidDocument` | Wrong roots, labels, schema, version, or leaf values |
| `MissingField` | A required record field is absent |
| `DuplicateField` | A named field occurs more than once |
| `UnknownField` | A record contains a field not in its versioned schema |
| `UnexpectedChildren` | A scalar field is not exactly one leaf value |
| `InvalidNumber` | A numeric leaf is not canonical unsigned decimal or is not representable |

The enum is `Clone + Copy + Debug + PartialEq + Eq` and marked
`#[non_exhaustive]`, so external matches must include a future-proof arm.

### `ProgramError`

```rust
pub enum ProgramError {
    Contract {
        kind: ProgramErrorKind,
        detail: String,
    },
    Syntax(recipe_ogdl::ParseError),
    Graph(recipe_language::OgdlCodecError),
    Build(recipe_ogdl::GraphError),
}
```

The enum is `Clone + Debug + PartialEq + Eq` and non-exhaustive. The private
`ProgramError::new` constructor creates a `Contract` value. `kind()` returns
`Some(kind)` for `Contract` and `None` for parser, language, and graph-build
wrappers. This prevents callers from pretending that every downstream error
is one of the program contract kinds.

`Display` is stable and direct:

```text
Contract: {Kind:?}: {detail}
Syntax: invalid program OGDL syntax: {parser error}
Graph: invalid calculation graph: {codec error}
Build: cannot build program OGDL: {graph error}
```

The actual implementation prints the kind debug spelling followed by the
detail for a contract. `std::error::Error::source` exposes the wrapped parser,
language codec, or graph-build error, and returns `None` for a contract error.
`From<ParseError>`, `From<OgdlCodecError>`, and `From<GraphError>` feed the
three wrapper variants used by the codec helpers.

## ScalarProgram ownership and model

The name `ScalarProgram` is a core payload type, not a type declared in
`recipe-program`. The program crate carries it transitively inside
`CalculationGraph` nodes and never creates an alternate scalar IR. The
authoritative implementation is
[`core/src/scalar.rs`](../../../core/src/scalar.rs), while the safe builder is
[`language/src/scalar_builder.rs`](../../../language/src/scalar_builder.rs).
This section records the complete scalar contract because it is the payload
that the static program ultimately schedules.

### Core scalar values

`DType` is the closed payload domain:

```rust
F32 | I32
```

Both have `byte_width() == 4`. There is no F16 payload type in the scalar IR.
`ScalarLiteral` stores either exact binary32 bits (`F32Bits(u32)`) or an
`I32(i32)` value. `ScalarLiteral::dtype()` returns the matching type; keeping
F32 as bits avoids a textual floating-point round trip during serialization.

`ScalarInput` and `ScalarConstant` are declaration records:

```text
ScalarInput    { id: ScalarValueId, dtype: DType }
ScalarConstant { id: ScalarValueId, value: ScalarLiteral }
```

`ScalarInstruction` is one ordered SSA definition:

```text
{ result: ScalarValueId,
  dtype: DType,
  opcode: ScalarOpcode,
  operands: Vec<ScalarValueId> }
```

The complete program record is public and manually constructible:

```rust
pub struct ScalarProgram {
    pub inputs: Vec<ScalarInput>,
    pub constants: Vec<ScalarConstant>,
    pub instructions: Vec<ScalarInstruction>,
    pub outputs: Vec<ScalarValueId>,
}
```

Its vectors are ordered. Inputs and constants define the initial SSA
namespace, instructions must consume values defined by earlier declarations,
and outputs name previously defined values. Core validation does not reserve
scalar ID zero; it checks identity uniqueness and definition order instead.

### Opcode arity, typing, and cost

`ScalarOpcode` is non-exhaustive. Its public methods are `arity`,
`result_dtype`, and `flops`. The current opcodes are:

| Group | Opcodes and actual typing |
| --- | --- |
| Same-type arithmetic | `Add`, `Subtract`, `Multiply`, `Divide`, `Remainder`, `Negate`, `Absolute`, `Minimum`, `Maximum` require all operands to share one dtype and return that dtype. F32 divide/remainder use IEEE behavior; I32 divide/remainder are checked truncating operations. |
| F32 arithmetic | `Fma` is ternary, requires three F32 operands, and returns F32. It counts as two FLOPs. |
| Comparisons | `Equal`, `NotEqual`, `LessThan`, `LessThanOrEqual`, `GreaterThan`, `GreaterThanOrEqual` are binary, require equal operand dtypes, and return normalized I32 one or zero. |
| Selection | `Select` is ternary. The first operand is I32, the second and third must share a dtype, and the result has that dtype. I32 zero selects the third operand; nonzero selects the second. |
| Integer logic | `BitAnd`, `BitOr`, `BitXor`, `BitNot`, `ShiftLeft`, `ShiftRightLogical`, and `ShiftRightArithmetic` require I32 operands and return I32. |
| Bitcasts and checks | `BitcastF32ToI32`, `IsFinite`, `IsNan`, and `ConvertF32ToI32` take one F32 and return I32. `BitcastI32ToF32` and `ConvertI32ToF32` take one I32 and return F32. |
| Domain and F32 unary operations | `Require` takes one I32 and returns I32; `SquareRoot`, `Floor`, `Ceiling`, and `RoundNearestEven` take one F32 and return F32. |

`arity()` returns one for unary opcodes, three for `Fma` and `Select`, and
two for the remaining binary opcodes. `result_dtype()` returns `None` for an
arity or type mismatch. `flops()` counts payload arithmetic and comparisons:
the listed arithmetic, comparisons, square root, floor, ceiling, and round
operations cost one, FMA costs two, and selection, integer logic, bitcasts,
conversions, validation predicates, and `Require` cost zero. Address
calculation and memory operations are not part of this scalar FLOP count.

### `ScalarProgram` validation and queries

`ScalarProgram::validate()` is an accumulating pass:

1. Insert every input ID and dtype into a type map, reporting
   `DuplicateScalarValue` when an ID repeats.
2. Insert every constant ID and literal dtype with the same duplicate rule.
3. Walk instructions in vector order. For each operand, require that the ID
   is already in the type map, reporting `ScalarUseBeforeDefinition` for an
   absent value. Then check exact arity and `result_dtype`, reporting
   `ScalarArity` or `ScalarTypeMismatch`. Finally insert the result ID and
   report a duplicate result if it was already defined.
4. Require at least one output (`MissingScalarOutput`) and require every output
   ID to be in the type map (`UnknownScalarValue`).

The result is a core `ValidationResult`, which can contain multiple failures.
The language builder and graph codec turn it into their own boundary errors;
the program layer receives it through `CalculationGraph::validate` and wraps
it in `ProgramError::Graph`.

`dtype_of(value)` searches inputs, constants, and instruction results in that
order and returns the first matching dtype or `None`. It is a query, not a
validation pass, so callers must validate before relying on it. `requires_fault_flag()`
returns true exactly when an instruction has one of these `(opcode, dtype)`
pairs:

```text
(Require, I32)
(Divide, I32)
(Remainder, I32)
(Negate, I32)
(Absolute, I32)
```

Those are the integer operations whose device implementation needs the
preallocated arithmetic-domain fault channel. F32 IEEE divide and remainder
do not request the flag through this predicate.

### Kernel template boundary

The scalar program becomes executable payload only when a core
`KernelTemplate` supplies tensor-facing metadata:

```text
IndexSpace
KernelInput  { id, dtype, StaticBufferAccess }
KernelOutput { id, dtype, StaticBufferAccess }
ScalarProgram
AliasRule { input, output, AliasPermission }
```

`IndexSpace::new` rejects an empty dimension list and checked-multiplies
nonzero `ElementCount` dimensions. `StaticBufferAccess` is an affine mapping
from that logical index space to one backing buffer. It stores element offset,
one stride per rank, and complete backing `storage_bytes`. `linear` and
`contiguous` construct common one-dimensional and contiguous views. Validation
checks rank, address arithmetic, backing-byte coverage, and injectivity for
writable mappings. Zero strides are permitted for read-only broadcasts but
not for overlapping writable outputs.

`KernelTemplate::validate` first appends any scalar-program validation errors.
It then requires unique input and output IDs, validates each access view,
requires kernel input count and dtypes to match scalar inputs, requires kernel
output count and dtypes to match scalar outputs, rejects unknown or duplicate
alias rules, and requires one alias rule for every input/output pair. The
alias permission values are `Forbidden`, `MayAliasExact`, and
`MustAliasExact`.

This is why a `PrimitiveKind::Elementwise` node can carry a complete scalar
program without carrying a device pointer or a schedule. The scalar namespace
and tensor views are validated together only at the lowering boundary.

## Building scalar programs safely

`recipe-language::ScalarProgramBuilder` is the safe constructor for the core
record. It owns a unique builder token and starts scalar IDs at one. Its
public API is:

```text
new() -> LanguageResult<ScalarProgramBuilder>
input(dtype) -> LanguageResult<ScalarExpression>
constant(literal) -> LanguageResult<ScalarExpression>
f32(value) -> LanguageResult<ScalarExpression>
i32(value) -> LanguageResult<ScalarExpression>
apply(opcode, operands) -> LanguageResult<ScalarExpression>
unary(opcode, operand) -> LanguageResult<ScalarExpression>
binary(opcode, left, right) -> LanguageResult<ScalarExpression>
ternary(opcode, first, second, third) -> LanguageResult<ScalarExpression>
finish(outputs) -> LanguageResult<ScalarProgram>
```

`ScalarExpression` is an opaque typed handle containing a private builder
owner, core scalar ID, and dtype. Its public queries are `id()` and
`dtype()`. `apply` rejects a handle from another builder, asks the opcode for
its result dtype, allocates the next ID, and appends one instruction in call
order. `finish` rejects foreign outputs, assembles the four core vectors, and
calls `ScalarProgram::validate` before returning. Builder identity overflow and
per-builder scalar ID overflow are explicit `InvalidScalarProgram` errors.

Math recipes, operation materializers, training forward graphs, and inference
graph compilers all use this builder or a direct operation-layer composer. No
caller creates a host evaluator or a second scalar representation.

## Scalar OGDL inside `RecipeIR`

The program crate copies the language `RecipeIR` subtree, so scalar encoding
is owned by `language/src/ogdl.rs`. An elementwise primitive contains:

```text
kind
    Elementwise
        program
            inputs
                input
                    id
                        1
                    dtype
                        F32
            constants
                constant
                    id
                        2
                    literal
                        F32Bits
                            1065353216
            instructions
                instruction
                    result
                        3
                    dtype
                        F32
                    opcode
                        Add
                    operands
                        value
                            1
                        value
                            2
            outputs
                value
                    3
```

The actual encoder writes exact `F32`, `I32`, `F32Bits`, and `I32` spellings.
Instruction order, operand order, result IDs, literal bits, and output order
are preserved. A scalar opcode that is newer than the language schema maps to
an `UnsupportedValue` codec error rather than a legacy spelling. Decoding
requires exact fields and collection item labels, parses canonical numbers,
reconstructs `ScalarProgram`, and then graph validation rechecks the scalar
SSA contract.

Thus a static program OGDL round trip checks two layers:

```text
RecipeProgram fields and domains
    + copied RecipeIR tensors, primitives, and scalar instructions
    -> CalculationGraph::from_ogdl_graph
    -> ScalarProgram::validate through KernelTemplate and graph validation
```

## Lowering consumers

`StaticCalculationProgram` is consumed by planning, while the scalar payload
inside its graph follows the primitive and kernel lowering path.

### Language graph to scalar primitive

`CalculationGraph::validate` calls `PrimitiveKernel::validate`, and an
elementwise primitive validates its embedded `ScalarProgram`, tensor input
arity and dtype, broadcast shape, output dtype and shape, and complete alias
matrix. The language layer does not assign a loop domain.

`recipe-ops` creates scalar programs in three ways:

* direct `ScalarProgramBuilder` recipes for composite operations and math;
* `ScalarProgram::try_from(MathFunction)` for the Recipe-owned math catalog;
* `Composer` or `lower_scalar` for operation registry symbols.

Training and inference use the same records. Their compiler methods append a
`PrimitiveKind::Elementwise(Elementwise { program })` node and append a
`KernelIterationDomain` for the intended initialization, training, validation,
or single-inference domain.

### Primitive lowering

`recipe-primitives::lower` validates the source primitive and dispatches
`PrimitiveKind::Elementwise` to `lower_elementwise`. That function:

1. Rejects invalid measured lowering hardware and invalid source graph data.
2. Creates an `IndexSpace` from the output shape. A zero-element tensor emits
   no dispatch stage, while a nonempty output has one logical lane per output
   element.
3. Converts each tensor input to a one-based core `KernelInput` and applies
   right-aligned broadcast access. Singleton expanded dimensions receive zero
   read strides; original offsets, non-broadcast strides, dtype, and backing
   bytes remain explicit.
4. Converts each tensor output to a writable core `KernelOutput` and maps the
   language alias matrix to core input/output IDs.
5. Clones the scalar program into a `KernelTemplate` and validates it again.
6. Binds input buffers in source order, output buffers in source order, and,
   when `requires_fault_flag()` is true, one I32 four-byte read/write-atomic
   fault buffer with `FaultReason::ArithmeticDomain` and code `2`.
7. Computes per-lane FLOPs as the checked sum of instruction opcode costs,
   multiplies by output elements for the stage bound, counts integer operations
   as logical lanes, and computes private bytes as
   `(inputs + constants + instructions) * 4` per lane.
8. Chooses a measured power-of-two workgroup width, constructs one scalar-map
   stage, aggregates resources, computes a canonical digest, and validates the
   complete `LoweredProgram`.

`LoweredProgram::validate` revalidates its embedded `KernelTemplate`, launch
geometry, binding count, fault ABI, synchronization, resource equations, and
digest. A scalar-map stage has no synchronization points. The fault binding is
present if and only if the scalar program's fault predicate is true.

### Native kernel lowering

`recipe-kernel::lower_stage` receives a validated `LoweredProgram`, an exact
`ArtifactBuildRecipe`, a measured target, and lowering options. For a scalar
map, `kernel/src/llvm.rs::lower_elementwise` validates the `KernelTemplate`,
loads affine input elements, materializes bit-preserving constants, replays
instructions in order, stores each scalar output, and emits the fault branch.
The generated LLVM IR is audited for prohibited interfaces before an offline
HSACO or cubin artifact is packaged. The scalar program is not interpreted on
the host and no vendor math library is inserted.

## Direct callers and data flow

The following are the direct workspace call sites found by tracing
`StaticCalculationProgram` references.

| Caller | How it uses the program |
| --- | --- |
| `planner/src/planner.rs::plan_candidates` | Wraps a graph with `every_iteration(..., LoopIterations::ONE)` for the legacy one-iteration API, then delegates to `plan_program_candidates`. |
| `planner/src/planner.rs::plan_program_candidates` | Validates the program, uses `graph()`, `iterations()`, `domains()`, and `metrics()`, lowers each graph node, hashes loop/domain/metric declarations into graph identity, and propagates source domains into loop task domains. |
| `prepare/src/lib.rs::Preparer::prepare` | Wraps a graph with one iteration for the legacy API and delegates to `prepare_program`. |
| `prepare/src/lib.rs::Preparer::prepare_program` | Validates the measured profile, resolves artifacts against `program.graph()`, plans candidates with the complete static program, realizes the selected candidate, and finalizes the immutable bundle. |
| `training/src/compile.rs::GraphCompiler::finish` | Marks tensor boundaries, builds and validates a graph, performs a graph OGDL round trip, builds `new_with_metrics`, serializes the program, parses it back, and stores the round-tripped value in `CompiledTraining`. |
| `training/src/inference.rs::InferenceGraphCompiler::finish` | Builds one-iteration dense inference graphs, validates and round-trips the graph, creates `StaticCalculationProgram::new`, round-trips the program, and stores it in `CompiledInference`. |
| `training/src/inference.rs` KNN compiler | Assigns `IterationDomain::first()` to every KNN kernel, validates and round-trips the graph, creates one-iteration `StaticCalculationProgram`, and stores it in `CompiledKnnInference`. |
| `training/src/model.rs::CompiledTraining` | Keeps the program private and exposes `program()` and `graph()` read-only accessors. |
| `training/src/inference.rs::CompiledInference` | Keeps the program private and exposes `program()` and `graph()` read-only accessors. |
| `training/src/inference.rs::CompiledKnnInference` | Keeps the program private and exposes `program()` and `graph()` read-only accessors. |
| `training/src/execute.rs` | Passes each compiled program to `Preparer::prepare_program`; training retains declared user metrics, while target-free inference rejects metric declarations and requires one iteration. |
| `src/facade.rs` | Reexports the crate as `recipe::engine::program`; it adds no behavior. |

No executor or native backend directly imports `StaticCalculationProgram`.
After `prepare_program` returns, the program's graph and schedule have been
projected into `DraftPlan`, realized artifacts, and `FinalizedBundle` records.

## Planner projection and runtime role

`plan_program_candidates` is the main lifecycle consumer. Its sequence is:

1. Validate the static program, its graph, topology, measured discovery,
   reservations, and capacity.
2. Obtain a topological graph order and lower every primitive kernel once for
   measured hardware. Loop iterations do not duplicate lowered stages.
3. Build `source_domains` from `program.domains()` and include
   `program.iterations()` and every metric `(id, value, domain)` in the stable
   graph digest. Changing a domain, horizon, metric ID, metric value, or metric
   domain changes candidate identity.
4. Enumerate legal placement assignments. For each candidate, initialize
   external input data images and fault buffers, materialize program buffers,
   create calculation and transfer tasks, assign each task the source domain,
   and retain the complete loop horizon.
5. Add one fault readback metric task per checked `(device, domain)` cohort.
   Add each declared user metric as a `MetricPurpose::User` four-byte readback
   from the producer-resident value. User metrics depend on their producer and
   on all already-created fault readbacks.
6. Add exit egress and resource-release tasks, validate the Draft, and rank
   candidates by measured makespan and stable identity.

The planner carries `LoopIterations` and `LoopTaskDomain` records in
`PlannedProgramCandidate`. It never unrolls the graph for a finite count and
never invents an end for an unbounded program.

`Preparer::prepare_program` preserves this contract through fixed-point native
artifact realization and post-warm capacity validation. The selected program
is finalized with the exact task domains and resource manifest. A missing
native artifact is a normal deferred build path, not a program-document error.

## End-to-end execution

The complete training path is:

```text
public training declaration
  -> training graph compiler
       ScalarProgramBuilder / ops materialization
       CalculationGraph + KernelIterationDomain + MetricEmission
  -> StaticCalculationProgram::new_with_metrics
  -> graph and program canonical OGDL round trips
  -> Preparer::prepare_program
       planner lowering, placement, transfers, metric tasks
       native artifact realization and FinalizedBundle
  -> PreparedRun::initialize
       one init admission image per finalized device
  -> PreparedRun::start_loop
       immutable task domains and nonblocking metric readbacks
  -> complete finite loop or explicit graceful stop
  -> exit egress and newest user metric samples
```

Training uses metric declarations produced from `TrainingOutputs::metric_bindings`.
The runtime drains only `MetricPurpose::User` slots for user reports; fault
readbacks are internal control checks. The program contract ensures each
declared metric is a valid one-scalar value and producer-covered domain before
the runtime can be handed a bundle.

The dense and KNN inference paths use the same graph and program types but
have a narrower execution boundary. Their compilers construct exactly one
iteration with `IterationDomain::first()` for every kernel. Execution validates
`program.iterations() == LoopIterations::ONE`, rejects any user metrics, admits
external inputs only in init, waits for loop completion, collects the declared
prediction tensors in exit, and never exposes a loop-time transfer API.

The executor sees `FinalizedBundle` task records. A scalar map is represented
as a calculation task backed by a realized native artifact. A user or fault
metric is a specialized four-byte device readback task, not a third calculation
kind. Thus the static program is the source declaration and validation gate;
the finalized bundle is the runtime authority.

## Invariants at a glance

The following are the invariants that must hold before a program can cross the
planning boundary:

```text
graph validates as an acyclic language CalculationGraph
iterations is finite and nonzero, or explicitly unbounded
every graph kernel has exactly one in-range domain
no consumer starts before its producer's first activation
metric IDs and value IDs are nonzero where the program reserves zero
metric IDs are unique
metric values are known, produced, internal, singleton, and four bytes
metric domains are in-range and producer-covered
OGDL has exactly RecipeProgram then RecipeIR roots
RecipeProgram schema/version/fields/numbers are exact
scalar programs are typed SSA with at least one known output
primitive and kernel validators agree on scalar arity, dtype, memory, and aliasing
```

There are no fallbacks, retries, sparse-domain defaults, host scalar
evaluators, compatibility shims, or alternate serialized roots. A failed
transition remains visible as its typed `ProgramError`, language error,
planner error, preparation error, or executor error.

## Source map

The implementation and the most important callees are:

| Path | Responsibility |
| --- | --- |
| [`program/src/lib.rs`](../../src/lib.rs) | Root facade, static-program records, validation, versioned `RecipeProgram` codec, and errors |
| [`core/src/schedule.rs`](../../../core/src/schedule.rs) | `LoopIterations` and `IterationDomain` constructors, queries, and bounds |
| [`core/src/scalar.rs`](../../../core/src/scalar.rs) | Scalar payload types, opcode signatures/costs, SSA validation, kernel templates, access views, and alias matrix |
| [`language/src/graph.rs`](../../../language/src/graph.rs) | Calculation graph assembly, graph validation, producer map, and topological order |
| [`language/src/scalar_builder.rs`](../../../language/src/scalar_builder.rs) | Builder-owned scalar expression construction and final scalar validation |
| [`language/src/ogdl.rs`](../../../language/src/ogdl.rs) | `RecipeIR` serialization, scalar encoding, strict decoding, and graph codec errors |
| [`primitives/src/lower.rs`](../../../primitives/src/lower.rs) | Elementwise scalar-to-template lowering, affine bindings, fault ABI, and resource bounds |
| [`planner/src/planner.rs`](../../../planner/src/planner.rs) | Static-program planning, source-domain propagation, graph identity, metric tasks, and Draft construction |
| [`prepare/src/lib.rs`](../../../prepare/src/lib.rs) | Program preparation, artifact realization, capacity fixed point, and finalization |
| [`training/src/compile.rs`](../../../training/src/compile.rs) | Dense training graph/program construction and metric declarations |
| [`training/src/inference.rs`](../../../training/src/inference.rs) | Dense and KNN one-iteration program construction |
| [`training/src/execute.rs`](../../../training/src/execute.rs) | Preparation handoff, init admission, loop polling, metric draining, and post-exit output collection |
| [`kernel/src/llvm.rs`](../../../kernel/src/llvm.rs) | Direct native lowering of a validated scalar `KernelTemplate` |

The root file is intentionally the only edited artifact for this document.
