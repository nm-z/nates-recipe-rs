# `recipe-program`

`recipe-program` is the validated, target-independent lifecycle envelope for a
Recipe calculation graph. It couples one acyclic
[`recipe_language::CalculationGraph`](../../language/src/graph.rs) with a
nonzero finite or explicitly unbounded loop and one activation
[`IterationDomain`](../../core/src/schedule.rs) for every source kernel. It
also carries the optional scalar telemetry declarations consumed by the
planner's preallocated metric slots.

The crate does not lower a primitive, choose a device, build or load an image,
allocate memory, schedule a task, or execute a loop. It records the exact
graph-level schedule contract that those later stages must preserve. Repeating
the loop never clones graph nodes or native artifacts. The immutable graph and
its artifacts stay singular, while `LoopIterations` and `IterationDomain`
select which already-finalized tasks are active on each zero-based iteration.

## Position in the pipeline

The complete data flow around this crate is:

```text
recipe-language / recipe-ops / recipe-training / recipe-inference
  typed tensors, primitive kernels, scalar programs, graph dependencies
                               |
                               v
                StaticCalculationProgram
        graph + loop count + kernel activation domains
        + optional one-scalar metric emissions
                               |
             +-----------------+------------------+
             |                                    |
             v                                    v
   recipe-planner::plan_program_candidates   recipe-prepare::Preparer
   validate, lower, place, route,             resolve, realize, stabilize,
   schedule, and draft tasks                  finalize one immutable bundle
             |                                    |
             +-----------------+------------------+
                               v
                    core::FinalizedBundle
              LoopTaskDomain for every loop task
                               |
                               v
                    recipe-executor / backends
             init -> repeated loop iterations -> exit
```

The program domain is copied onto every generated loop calculation, internal
transfer, and metric task by the planner. `recipe-core` then validates those
task domains while finalizing the bundle. The executor reads the finalized
domains and calls `IterationDomain::contains` for the current iteration. Init
admission and exit egress remain transfers in their own lifecycle phases, not
program domains.

`recipe-program` therefore sits between the placement-free language graph and
the concrete task schedule. It owns graph-level activation and telemetry
invariants, while the planner owns task-level expansion and the executor owns
runtime activation.

## Manifest and module boundary

[`program/Cargo.toml`](../Cargo.toml) declares package `recipe-program`,
version `0.1.0`, Rust edition 2024, MIT licensing, and the description
"Static lifecycle and iteration domains for Recipe calculation graphs". Its
only dependencies are:

| Dependency | Use at this boundary |
| --- | --- |
| `recipe-core` | `LoopIterations`, `IterationDomain`, typed `KernelTemplateId`, `MetricId`, `ValueId`, and scalar byte units. |
| `recipe-language` | `CalculationGraph`, graph validation, and the nested Recipe IR OGDL codec. |
| `recipe-ogdl` | Ordered rooted-forest storage, canonical serialization, parsing, node IDs, and graph-build errors. |

The crate has one implementation module, [`program/src/lib.rs`](../src/lib.rs).
It forbids unsafe code and denies missing `Debug` implementations. There are no
submodules, build scripts, public macros, runtime threads, filesystem access,
driver bindings, or tests in the crate. All public names are re-exported or
defined by that single facade.

The implementation dependency direction is deliberately narrow:

```text
recipe-core (IDs, LoopIterations, IterationDomain, ByteCount)
          |
          +--> recipe-language (CalculationGraph, OgdlCodecError)
          |          |
          |          +--> recipe-ogdl (Graph, ParseError, GraphError)
          |
          +--> recipe-program (program validation and two-root OGDL envelope)
```

`recipe-program` does not define a second tensor, primitive, scalar, or task
ontology. Its `graph` field is the language graph, its schedule values are the
core schedule types, and its document contains the canonical language graph as
the second root.

## Public data model

### Re-exported loop types

`recipe_program::LoopIterations` and `recipe_program::IterationDomain` are
re-exports of the core types, so callers can construct the complete public
contract without depending on a second wrapper. Their definitions and
semantics are in [`core/src/schedule.rs`](../../core/src/schedule.rs):

* `LoopIterations::Finite(NonZeroU64)` is an exact nonzero count.
* `LoopIterations::Unbounded` has no invented terminal iteration and can stop
  only at an explicit graceful-stop or failure boundary later in the runtime.
* `IterationDomain` is a nonempty, zero-based, half-open arithmetic progression
  with a nonzero stride. A finite domain stores `[first, end_exclusive)` and an
  unbounded domain stores `first` with no end.
* `IterationDomain::every(iterations)` selects every loop iteration, and
  `IterationDomain::first()` selects only iteration zero.
* `is_within` checks that the complete domain fits the program loop. Program
  validation rejects an unbounded domain inside a finite loop, while the core
  constructors reject empty, reversed, or zero-stride domains.

### Kernel activation and telemetry records

`KernelIterationDomain` is a public copyable record:

```text
kernel: KernelTemplateId
domain: IterationDomain
```

It assigns exactly one source graph kernel to one activation progression. The
record does not contain a placement, device, task ID, artifact, or dependency.

`MetricEmission` is a public copyable record:

```text
metric: MetricId
value: ValueId
domain: IterationDomain
```

It declares that one produced four-byte scalar tensor is read through a
preallocated, nonblocking metric slot on the declared subset of loop
iterations. The record is a telemetry declaration only. It is not an external
output and it does not add a third kind of calculation work.

### `StaticCalculationProgram`

`StaticCalculationProgram` is `Clone`, `Debug`, `PartialEq`, and `Eq`. Its four
fields are private and can be populated only by a constructor or
`from_ogdl`:

| Field | Meaning and invariant |
| --- | --- |
| `graph: CalculationGraph` | The typed, acyclic, placement-free graph of tensors and primitive kernels. |
| `iterations: LoopIterations` | The exact finite or unbounded loop horizon. |
| `domains: Vec<KernelIterationDomain>` | One entry for every graph kernel, sorted by kernel ID. |
| `metrics: Vec<MetricEmission>` | Optional telemetry declarations, sorted by metric ID. |

The constructor sorts both vectors before validation. This makes lookup and
canonical encoding deterministic even when a caller supplies records in a
different order. `with_metrics` is the only mutating-looking API: it replaces
the sorted metric vector, validates the complete program, and returns the
program on success. A failed replacement returns an error and does not expose
an invalid object.

## Public API

The complete API is intentionally small:

| Method | Actual behavior |
| --- | --- |
| `new(graph, iterations, domains)` | Builds a program with no metric emissions by delegating to `new_with_metrics`. It sorts domains and validates graph, domains, and dependencies. |
| `new_with_metrics(graph, iterations, domains, metrics)` | Converts any `Into<LoopIterations>`, sorts both record vectors, validates, and returns the immutable program. |
| `every_iteration(graph, iterations)` | Creates one `KernelIterationDomain` per graph node using `IterationDomain::every(iterations)`, then calls `new`. It does not clone or unroll the graph. |
| `with_metrics(metrics)` | Replaces the metric vector, sorts by `MetricId`, validates, and returns `Ok(self)` or a `ProgramError`. |
| `validate()` | Revalidates the graph and all graph-level lifecycle and telemetry invariants. It does not mutate or lower anything. |
| `graph()` | Returns the borrowed `CalculationGraph`. |
| `iterations()` | Returns the copyable `LoopIterations`. |
| `domains()` | Returns the sorted borrowed domain slice. |
| `domain(kernel)` | Binary-searches the sorted domains and returns the matching `IterationDomain`, or `None`. |
| `metrics()` | Returns the sorted borrowed metric slice. |
| `metric(metric)` | Binary-searches the sorted metrics and returns a copied `MetricEmission`, or `None`. |
| `to_ogdl()` | Validates, builds the two-root OGDL graph, and returns its canonical string. |
| `to_ogdl_graph()` | Validates and returns the ordered `recipe_ogdl::Graph`. |
| `from_ogdl(input)` | Parses OGDL and delegates to strict `from_ogdl_graph`. |
| `from_ogdl_graph(source)` | Strictly decodes the two-root document, reconstructs the language graph, and runs the same constructor validation. |

There are no setters for the graph, loop count, or domains. Callers must build
a new program when those values change. Equality compares the complete graph,
loop contract, sorted domains, and sorted metrics, so it is suitable for
program identity checks and checkpoint compatibility.

## Validation contract

`new`, `new_with_metrics`, `with_metrics`, and both serialization directions
are validation gates. `to_ogdl` cannot emit an invalid program, and
`from_ogdl` cannot return one. `validate` performs the following checks in
order.

### Graph validity

The language graph is validated first through
`CalculationGraph::validate`. Any language error is wrapped as
`ProgramError::Graph(OgdlCodecError::InvalidGraph(...))`. Language validation
checks tensor contracts, unique tensor and kernel IDs, primitive input and
output references, one producer per output, external boundary consistency,
primitive alias matrices, scalar program validity, and graph acyclicity.

The program crate then builds a `BTreeSet` of the graph's kernel IDs and a
`BTreeMap<ValueId, KernelTemplateId>` of output producers. The producer map is
safe after graph validation has rejected duplicate producers.

### Complete kernel domain coverage

For each supplied `KernelIterationDomain`, validation requires:

1. The kernel ID exists in the graph, otherwise `UnknownKernel`.
2. The kernel has not already received a domain, otherwise `DuplicateKernel`.
3. The domain is within the declared loop, otherwise
   `InvalidIterationDomain`.

After scanning the vector, the assigned ID set must equal the graph kernel ID
set. The first missing ID produces `MissingKernel`. Thus an empty graph is
valid with an empty domain vector, while a nonempty graph must have exactly one
domain per node. The constructor's sort makes this set check independent of
input order.

### First-use dependency ordering

For every graph consumer input that has a graph producer, the producer's first
activation must not occur after the consumer's first activation. A later first
producer produces `UninitializedDependency` with the kernel and value IDs.
This is deliberately a first-activation check. The program layer does not
expand arithmetic progressions or prove every producer and consumer stride
intersection. Planner lowering and core task validation enforce the stronger
task-level transfer and dependency contracts after placement and domain
expansion.

External inputs have no producer and are skipped by this check. Graph
topological validation still rejects cycles and missing non-external producers.

### Metric contract

Every `MetricEmission` is checked independently:

* Metric ID zero is reserved (`InvalidMetricId`). Metric IDs must be unique
  (`DuplicateMetric`).
* Value ID zero is reserved (`InvalidMetricValue`). The value must exist in the
  graph (`UnknownMetricValue`).
* The value tensor must contain exactly one element and exactly four storage
  bytes. The language payload domain is only F32 and I32, both four bytes, so
  this is one device scalar. Any other shape or byte size is
  `InvalidMetricValue`.
* The tensor must remain loop-internal. An `external_output` tensor is rejected
  as `InvalidMetricValue`; egress values are transfers, not metric mailboxes.
* The metric domain must fit the program loop, otherwise
  `InvalidIterationDomain`.
* The value must have a calculation producer (`UnproducedMetricValue`).
* The producer domain must cover every metric activation, otherwise
  `UncoveredMetricDomain`.

`domain_covers(producer, emission)` first requires the emission's first
iteration to be at or after the producer's first and congruent to the
producer's stride. For finite emissions it computes the last selected point
and requires it to be strictly before a finite producer end. An unbounded
emission therefore requires an unbounded producer. A multi-point emission is
covered when its stride is an integer multiple of the producer stride; a
single-point emission is covered after the first-point checks. The check is
about activation coverage, not metric ID uniqueness or output ownership.

## Canonical OGDL envelope

The program document is an ordered two-root forest. `to_ogdl_graph` appends the
program root first and copies the canonical language graph as the second root:

```text
RecipeProgram
\tschema\tStaticCalculationProgram
\tversion\t2
\titerations\t<positive decimal | unbounded>
\tdomains
\t\tdomain
\t\t\tkernel\t<decimal KernelTemplateId>
\t\t\tfirst\t<decimal u64>
\t\t\tend_exclusive\t<decimal u64 | unbounded>
\t\t\tstride\t<positive decimal u64>
\tmetrics
\t\tmetric
\t\t\tid\t<positive decimal MetricId>
\t\t\tvalue\t<positive decimal ValueId>
\t\t\tfirst\t<decimal u64>
\t\t\tend_exclusive\t<decimal u64 | unbounded>
\t\t\tstride\t<positive decimal u64>
RecipeIR
\tschema\tCalculationGraph
\tversion\t1
\t... canonical language graph fields ...
```

The `domains` and `metrics` collection children are emitted in their sorted
vector order. Finite loop counts and domain ends are decimal `u64` text;
unbounded values are the exact word `unbounded`. `Graph::to_canonical_string`
provides the final syntax, with the program root first and the `RecipeIR` root
on a separate root line. No graph node is unrolled for any iteration.

The calculation root is generated by
`CalculationGraph::to_ogdl_graph`, which validates the graph again. Its
primitive nodes include nested scalar programs for `Elementwise` kernels. The
program crate copies that subtree node by node rather than re-encoding or
interpreting primitive fields.

### Strict decoding and compatibility

`from_ogdl_graph` requires exactly two roots in this order: `RecipeProgram`,
then `RecipeIR`. It requires exact root text and exact schema text. The program
version is the one compatibility switch:

* Version `"1"` requires exactly `schema`, `version`, `iterations`, and
  `domains`; it has no metrics and decodes as an empty metric vector.
* Version `"2"` requires those fields plus `metrics`; this is the current
  encoder version.
* Any other version is `InvalidDocument`.

Every record uses exact field sets. Missing, repeated, or unknown fields map to
`MissingField`, `DuplicateField`, or `UnknownField`. Fields are recognized by
name, so their source ordering may differ, but each field must occur once.
Each value field must contain exactly one leaf child. A value node with
children or a field with zero or multiple children is `UnexpectedChildren`.
Collection members must have the exact text `domain` or `metric`.

Unsigned numbers are parsed by `parse_u64`: empty strings, leading `+`,
multi-digit leading zeroes, negatives, non-decimal text, and values outside
`u64` are rejected as `InvalidNumber`. Loop count zero is rejected by
`LoopIterations::new`. Domains are constructed with the core constructors and
reject empty, reversed, or zero-stride values before the full program
validation. IDs themselves are opaque `u64` wrappers, so the program-specific
zero rules above remain necessary during decoding.

The second root is copied into a fresh one-root graph and decoded by
`CalculationGraph::from_ogdl_graph`. The reconstructed graph and schedule are
then passed to `new_with_metrics`, so a decoded document receives exactly the
same graph, domain, dependency, metric, and coverage checks as a program built
in memory. A syntax failure from `Graph::parse` is `ProgramError::Syntax`; a
language OGDL/document failure is `ProgramError::Graph`; and a graph-builder
failure while assembling the envelope is `ProgramError::Build`.

## Scalar programs inside the graph

The program crate does not define scalar instructions, but scalar programs are
part of the graph payload it validates and serializes. The authoritative
representation is `recipe_core::ScalarProgram` in
[`core/src/scalar.rs`](../../core/src/scalar.rs):

```text
ScalarProgram {
    inputs: Vec<ScalarInput>,
    constants: Vec<ScalarConstant>,
    instructions: Vec<ScalarInstruction>,
    outputs: Vec<ScalarValueId>,
}
```

The only payload types are `DType::F32` and `DType::I32`, each four bytes.
`ScalarLiteral::F32Bits(u32)` preserves exact binary32 bits and
`ScalarLiteral::I32(i32)` stores signed integer literals. An instruction has
an SSA result ID, result type, `ScalarOpcode`, and ordered operand IDs.
`ScalarOpcode::arity` and `result_dtype` are the single signature authority for
all arithmetic, comparisons, selection, bit operations, casts, checked integer
operations, predicates, and f32 functions. `ScalarProgram::validate` rejects
duplicate definitions, use before definition, invalid arity or type signature,
missing outputs, and unknown outputs. `requires_fault_flag` identifies
`Require`, checked integer divide/remainder, and checked integer negate/absolute
operations that need a preallocated device fault flag during lowering.

`recipe-language::ScalarProgramBuilder` is the safe producer. It gives every
builder an owner tag, allocates scalar IDs in call order, rejects expressions
from another builder, delegates opcode signatures to core, and validates the
finished program. `f32` stores exact bits, while `i32` stores an I32 literal.
The builder never reorders instructions. Training, inference, math, and ops
construct their elementwise payloads through this builder or through
`recipe_ops::lower_scalar`; the resulting `ScalarProgram` is placed in a
language `PrimitiveKind::Elementwise` node.

Language graph validation calls `PrimitiveKernel::validate`, which calls the
scalar validator and additionally checks primitive tensor arity, dtypes,
broadcast access, output shape, and the complete alias matrix. Language OGDL
encodes and decodes the nested scalar record with exact `inputs`, `constants`,
`instructions`, and `outputs` fields. Each instruction records `result`,
`dtype`, `opcode`, and ordered `operands`; literals use `F32Bits` or `I32`.
The scalar opcode spelling is strict, and an opcode unknown to the language
schema fails before a program can be returned.

After the program has been decoded, `recipe-primitives::lower` maps every
language primitive to a backend-neutral `LoweredProgram`. An elementwise
scalar program becomes a `StageKind::ScalarMap` with a validated core
`KernelTemplate`; its typed inputs, outputs, affine accesses, aliases, fault
contract, dispatch geometry, resource bounds, and digest are fixed before
target realization. The program crate remains responsible only for preserving
which source kernel and loop activations feed that lowering.

## Direct callers and callees

All direct workspace references were traced with `rg`. They fall into the
following paths.

### Planning and preparation

* `recipe-planner::plan_candidates` is the legacy graph convenience path. It
  constructs `StaticCalculationProgram::every_iteration(graph.clone(),
  LoopIterations::ONE)` and delegates to `plan_program_candidates`.
* `recipe-planner::plan_program_candidates` validates the program, obtains
  `program.graph()`, `program.domains()`, `program.iterations()`, and
  `program.metrics()`, and passes those values into primitive lowering,
  placement enumeration, graph hashing, data-image construction, task
  lowering, and candidate ranking. The graph digest includes the loop count,
  every source domain, and every metric record, so changing activation or
  telemetry changes candidate identity.
* `recipe-planner::lower_candidate` passes each source domain into
  `lower_program_invocation`. That domain is assigned to every lowered
  calculation stage, to each internal transfer hop created for a consumer
  domain, and to each fault readback cohort. `add_user_metrics` finds the
  producer device, requires a resident producer copy, creates a
  `TaskKind::Metric` with `MetricPurpose::User`, and assigns the emission
  domain to that task. External outputs are still exit transfers.
* `recipe-prepare::Preparer::prepare` creates a one-iteration
  `every_iteration` program for callers that provide only a graph.
  `Preparer::prepare_program` accepts an existing program, validates the
  measured profile, resolves artifacts from `program.graph()`, plans the
  complete candidate product, realizes native resources, and finalizes with
  the planner's exact `loop_iterations` and `loop_domains`. The bundle identity
  hashes both values. It never changes the program's graph or domains.

### Training compilation and model state

`recipe-training::compile::GraphCompiler` accumulates tensors, primitive nodes,
and one `KernelIterationDomain` for every emitted source kernel. Direct emits,
materialized operation graphs, and every optimizer or forward/backward graph
fragment receive the caller's `IterationDomain`. `training_metric_bindings`
assigns consecutive nonzero `MetricId`s to training loss, learning rate, and
requested validation outputs. It preserves each metric's value and validation
domain.

`GraphCompiler::finish` marks external input and output tensors, constructs and
validates the language graph, canonicalizes it through language OGDL, then
constructs `StaticCalculationProgram::new_with_metrics`. It immediately
round-trips the program through `to_ogdl` and `from_ogdl`; the returned
`CompiledTraining` therefore owns a strict canonical program rather than the
pre-round-trip graph. `TrainingCompileError` maps any `ProgramError` to its
`Program` kind and carries the error text.

`CompiledTraining` stores the program privately and exposes borrowed
`program()` and `graph()` accessors. Its `TrainingOutputs` retains the semantic
metric bindings used later to interpret metric slots, while the program owns
the runtime `MetricEmission` records.

`recipe-training::inference` has two analogous target-free compilers:

* `InferenceGraphCompiler::finish` marks one prediction tensor as the external
  output, gives every source and materialized kernel `IterationDomain::first()`,
  builds a one-iteration program with no metrics, and round-trips it through
  canonical OGDL. `CompiledInference` exposes `program()` and `graph()`.
* KNN compilation performs the same one-iteration, first-domain construction
  for all output kernels and returns `CompiledKnnInference` with no metric
  emissions.

Inference compilation also uses the scalar builder and operation registry for
elementwise payloads, but it never puts user metrics in the program. The
execution boundary rejects any nonempty metric vector even if a caller manually
constructs an otherwise valid inference object.

### Checkpoint identity and resume

`recipe-training::checkpoint::compiled_training_program_digest` serializes
`CompiledTraining::program().to_ogdl()` and hashes the canonical bytes. The
digest covers graph primitives, nested scalar programs, loop iterations,
source domains, and metric emissions, but not external input image bytes.
Native kernel resume compares its authenticated realization's program digest
with this value before accepting a supplied `.cubin` or `.hsaco`. A changed
program envelope therefore cannot silently reuse a kernel built for a different
schedule.

### Execution and public facade

`recipe-training::execute` passes the compiled program to
`Preparer::prepare_program` for local training, dense inference, and KNN
inference. The inference paths require `LoopIterations::ONE` and an empty
metric list. Training validates its loop control against
`program.iterations()`, admits input images in init, drains user metric slots
while the loop runs, and collects external outputs in exit.

`recipe-training::model` stores the program inside `CompiledTraining` and the
compiled inference structs. The root [`src/facade.rs`](../../src/facade.rs)
re-exports the whole crate as `recipe::engine::program`. Advanced callers can
use the same typed constructors, accessors, and OGDL codec through that facade;
the root declaration API does not duplicate this schedule representation.

## From source domain to running task

One valid training or inference path traverses the following concrete stages:

1. A compiler builds a `CalculationGraph`. Scalar expressions are validated as
   typed SSA inside each elementwise primitive. The compiler associates every
   source kernel with a `KernelIterationDomain` and optionally associates
   four-byte internal values with `MetricEmission` records.
2. `StaticCalculationProgram::new_with_metrics` sorts the records and runs
   graph, domain, dependency, scalar, and metric validation. The resulting
   program remains placement-free and immutable.
3. `recipe-planner::plan_program_candidates` obtains measured hardware limits,
   lowers each primitive into backend-neutral stages, and enumerates legal
   placements. It keeps the graph singular and expands only the runtime task
   sidecar. Source domains are attached to calculation stages, transfers,
   fault readbacks, and user metric tasks. Init images and exit transfers are
   assigned their lifecycle phases without a loop domain.
4. `recipe-prepare` realizes a ranked candidate against the measured system,
   stabilizes post-warm capacity observations, and calls
   `FinalizedBundle::finalize_with_loop_schedule`. Core validates exactly one
   domain per loop task, no domains on init or exit tasks, producer and transfer
   domain compatibility, and the full artifact/resource/arena contract.
5. `recipe-executor::PreparedTask` retrieves each finalized task's domain. On
   every `LoopIteration`, `active_on` calls `domain.contains(iteration.index())`.
   Active calculations, internal transfers, fault readbacks, and metric reads
   execute through the backend. Inactive tasks are not submitted for that
   iteration. The same task graph and native images are reused until the finite
   loop ends or an unbounded loop stops explicitly.
6. Exit transfers copy only declared external outputs. Metric values remain
   mailbox readbacks associated with their `MetricId` and slot. No program
   method performs a host transfer or backend call itself.

This separation is why the program crate is a lifecycle envelope rather than a
runtime scheduler. Every downstream state transition consumes its validated
records and adds only the information owned by that stage.

## Error surface and failure ownership

`ProgramErrorKind` is non-exhaustive and contains the graph-level contract
classes:

| Kind | Failure owned by `recipe-program` |
| --- | --- |
| `InvalidIterationDomain` | A domain is empty or out of loop bounds, or a decoded numeric domain cannot be constructed. |
| `UnknownKernel` | A domain names no graph kernel. |
| `DuplicateKernel` | A kernel receives more than one domain. |
| `MissingKernel` | A graph kernel receives no domain. |
| `UninitializedDependency` | A producer's first activation follows a consumer's first activation. |
| `InvalidMetricId` | Metric ID zero is reserved. |
| `DuplicateMetric` | A metric ID is repeated. |
| `UnknownMetricValue` | A metric names no graph tensor. |
| `InvalidMetricValue` | A metric value is zero, non-scalar, not four bytes, or an external output. |
| `UnproducedMetricValue` | The metric value has no calculation producer. |
| `UncoveredMetricDomain` | The producer progression does not cover the metric progression. |
| `InvalidDocument` | Root, schema, version, record text, or copied node identity is wrong. |
| `MissingField` | A required document field is absent. |
| `DuplicateField` | A document field occurs more than once. |
| `UnknownField` | A document contains a field not in the selected version's exact set. |
| `UnexpectedChildren` | A leaf field does not contain exactly one leaf value. |
| `InvalidNumber` | A number is not canonical unsigned decimal or does not fit `u64`. |

`ProgramError` wraps lower-layer failures that do not have a program contract
kind:

* `Syntax(ParseError)` is an OGDL parser failure, including malformed
  indentation or node text.
* `Graph(OgdlCodecError)` is a language graph document, language validation,
  or language graph-build failure. `validate` uses this variant for graph
  validation; decoding the copied `RecipeIR` can also produce it.
* `Build(GraphError)` is an ordered-forest construction failure while encoding
  the program envelope or copying a subtree.

`ProgramError::kind()` returns `Some(ProgramErrorKind)` only for contract
errors. It returns `None` for the three wrapped lower-layer variants. `Display`
adds the operation context (`invalid program OGDL syntax`, `invalid calculation
graph`, or `cannot build program OGDL`), and `Error::source` exposes the
underlying parser, language codec, or graph-builder error. The type implements
`From` for `ParseError`, `OgdlCodecError`, and `GraphError`, so helper failures
propagate without alternate fallback paths.

Failures after this crate belong to their owning stage. Primitive lowering
reports invalid lowered programs, the planner reports no route or infeasible
capacity, preparation reports artifact and realization rejection, core reports
finalized task-domain violations, and the executor reports backend or lifecycle
failures. `recipe-program` does not catch, retry, or substitute those errors.

## State and invariant summary

The central ownership rule is:

```text
CalculationGraph + LoopIterations + source domains + metric declarations
                         |
                         v
              validated StaticCalculationProgram
                         |
       (planner adds tasks, placement, transfers, artifacts)
                         |
                         v
              validated FinalizedBundle and executor state
```

Important consequences are:

* The graph and all scalar programs remain immutable after construction. No
  per-iteration graph, stage, artifact, arena, or metric slot is allocated.
* Domain and metric vectors are sorted once, and lookups use binary search.
  Callers must not rely on the order they supplied.
* Every graph kernel is explicitly assigned. There is no implicit default
  domain except the deliberate `every_iteration` convenience constructor.
* Metrics are internal four-byte mailboxes. They cannot double as external
  output declarations, and they do not change graph producer ownership.
* Program validation checks only information available at the placement-free
  boundary. The planner and core repeat relevant checks after adding devices,
  transfers, tasks, resources, and final loop schedules.
* OGDL is a canonical interchange and identity input, not an execution format.
  Decoding always returns the typed program before any planner or executor is
  invoked.
* There is no backend fallback or compatibility shim. Version `1` is accepted
  only for its exact historical field set and is normalized to the current
  in-memory representation; encoding always emits version `2`.

The repository's structural checks are `cargo check -p recipe-program` and
`cargo fmt --all -- --check`. They prove Rust and formatting validity only.
Runtime correctness is established by the real training or inference paths
that consume this program, lower it on measured hardware, finalize the bundle,
and observe independently derived output and metric state.
