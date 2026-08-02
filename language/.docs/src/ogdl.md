# RecipeIR OGDL graph codec

This document describes the versioned text bridge implemented by
[`language/src/ogdl.rs`](../../src/ogdl.rs). It is the codec contract for a
typed [`CalculationGraph`](../../src/graph.rs), not a general OGDL
implementation and not a semantic model-file decoder. The language crate owns
tensor metadata, placement-free primitive kernels, and their calculation
dependencies. It does not serialize devices, transfers, queues, schedules,
lifecycle phases, or native binaries.

The codec has no file I/O. It exposes four graph-boundary methods:

| Method | Boundary | Behavior |
| --- | --- | --- |
| `CalculationGraph::to_ogdl` | typed graph to `String` | Validates the complete graph, encodes it into a `recipe_ogdl::Graph`, and returns that graph's canonical string. |
| `CalculationGraph::to_ogdl_graph` | typed graph to `recipe_ogdl::Graph` | Performs the same validation and encoding without serializing the arena. |
| `CalculationGraph::from_ogdl` | `&str` to typed graph | Parses the narrow OGDL syntax, then applies the strict `RecipeIR` decoder and graph validation. |
| `CalculationGraph::from_ogdl_graph` | parsed `Graph` to typed graph | Applies the strict decoder to an already parsed ordered forest. |

Encoding is validation-first. Decoding is parse-first, document-strict, then
semantic: syntax is accepted by `Graph::parse`, every required record and value
is checked by the codec, and the resulting `CalculationGraph` is passed to
`CalculationGraph::validate`. No missing field receives a default, and no
unknown field, enum spelling, schema version, or scalar opcode is silently
discarded.

## Format identity and boundary

The language codec has one root and one schema version:

| Constant | Required text |
| --- | --- |
| root | `RecipeIR` |
| schema field | `CalculationGraph` |
| version field | `1` |

The standalone document therefore starts with `RecipeIR`, followed by exactly
the fields `schema`, `version`, `tensors`, and `nodes`. A string beginning with
`recipe`, `recipe-knn-model`, or `recipe-bayes-model` is a different semantic
artifact family. Those roots are decoded by the training inference/checkpoint
code and are not accepted by `CalculationGraph::from_ogdl`.

`recipe-program` adds its own outer `RecipeProgram` root and embeds a copied
`RecipeIR` subtree as the second root. `StaticCalculationProgram::from_ogdl`
validates its program envelope first, then calls
`CalculationGraph::from_ogdl_graph` on that second root. The language codec
does not know about loop iterations or metric emissions.

## The underlying OGDL-derived syntax

`recipe-ogdl` stores an ordered rooted forest in an append-only arena. Its
syntax gives spaces no structural meaning:

* ordinary spaces remain literal node text;
* leading tabs select the parent depth for a line;
* every later tab on the same line separates another node in a parent-child
  chain; and
* a newline starts another chain at that line's indentation depth.

For example, a line with one leading tab followed by `schema`, a tab, and
`CalculationGraph` creates a `schema` child of the root with one value child.
A later line with one leading tab and `tensor` starts a different root child. The parser permits LF and CRLF line endings, but
rejects a bare carriage return. A line may not jump beyond the currently
available ancestor depth. Empty lines and empty tab-delimited segments are
empty nodes and are rejected. Node text may not contain a tab, carriage return,
or line feed, so there is no quoting, escaping, comment, anchor, link, shared
reference, or cycle syntax.

The canonical serializer writes LF, places the first child inline after its
parent, and writes later siblings on lines indented to their depth. Canonical
output is deterministic for one arena. The decoder checks exact field names and
values but does not require source fields to be in canonical order; re-encoding
such a document normalizes the order emitted by `encode_graph`.

The parser itself can represent multiple roots. `CalculationGraph` decoding
requires exactly one root, so an empty input, a multi-root forest, or a root
with any name other than `RecipeIR` is a document error.

## Document tree

The top-level tree is:

The following blocks are structural notation, not literal documents: angle
brackets stand for concrete decimal or enum text, and `#` introduces an
explanatory comment outside the format. The final example is a concrete
parseable document.

```text
RecipeIR
	schema	CalculationGraph
	version	1
	tensors
		tensor ...
	nodes
		node ...
```

`tensors` and `nodes` are explicit collections. Their child records must be
named `tensor` and `node`, respectively, and may be empty. Collection order is
retained. There are no count fields: the child count is the collection.

### Tensor records

Each `tensor` has exactly these fields:

```text
tensor
	id	<ValueId as decimal u64>
	dtype	F32 | I32
	shape
		extent	<u64>     # one line per extent
	layout
		offset_elements	<u64>
		strides
			stride	<u64>  # one line per stride
	storage_bytes	<u64>
	external_input	true | false
	external_output	true | false
```

`shape` and `layout` are records, while `shape.extent` and
`layout.strides.stride` are collections. The decoder constructs `Shape` from
the extent list and `TensorLayout` from the offset and stride list. Shape rank
must be nonzero, and the product of extents must fit `u64`; an extent of zero
is allowed and describes an empty payload. A `ValueId` is an opaque `u64`
identity, so zero is representable by the ID wrapper even though normal
builders generally start at one.

Tensor validation then requires:

* stride count equal to shape rank;
* no zero stride on a non-singleton axis;
* non-overlapping logical elements for the declared layout;
* checked layout span arithmetic; and
* the byte span implied by dtype and layout no larger than `storage_bytes`.

`storage_bytes` may be larger than the logical span. Both external flags are
serialized independently. `external_input` participates in graph producer
validation; `external_output` marks the run boundary but does not remove the
requirement that an internally produced tensor have a producer.

### Calculation node and kernel records

Each `node` has exactly one `kernel` record. A kernel has exactly these fields:

```text
node
	kernel
		id	<KernelTemplateId as decimal u64>
		inputs
			value	<ValueId>  # one line per input, order is retained
		outputs
			value	<ValueId>  # one line per output, order is retained
		alias_rules
			alias_rule
				input	<usize>
				output	<usize>
				permission	Forbidden | MayAliasExact | MustAliasExact
		kind
		<exactly one primitive variant>
```

The `inputs` and `outputs` children are value collections. The `input` and
`output` fields inside an `alias_rule` are positional indexes into those two
collections, not tensor IDs. Every input/output pair must have exactly one
alias rule, and no pair may be repeated. An index outside the respective
collection is invalid. The permission text is an exact spelling of the
`recipe_core::AliasPermission` variant.

`kind` has exactly one child. The child name selects one of the ten primitive
variants below. Variant records use the fields shown here, with no additional
fields.

### Primitive variants

#### `Elementwise`

```text
kind
	Elementwise
		program
			inputs
				input
					id	<ScalarValueId>
					dtype	F32 | I32
				constants
					constant
						id	<ScalarValueId>
						literal
						<F32Bits or I32 variant>
				instructions
					instruction
						result	<ScalarValueId>
						dtype	F32 | I32
						opcode	<exact scalar opcode>
						operands
							value	<ScalarValueId>
				outputs
					value	<ScalarValueId>
```

`inputs`, `constants`, `instructions`, and `outputs` are all explicit
collections. Scalar input/constant/instruction IDs share one value namespace.
Instruction order is semantic: an operand must refer to an earlier input,
constant, or instruction result. A literal is an exact one-child variant:

* `F32Bits` followed by a decimal `u32`, preserving the raw binary32 bits;
* `I32` followed by a signed decimal `i32`.

The current version-1 scalar opcode spellings are:

```text
Add Subtract Multiply Divide Remainder Negate Absolute Minimum Maximum Fma
Equal NotEqual LessThan LessThanOrEqual GreaterThan GreaterThanOrEqual Select
BitAnd BitOr BitXor BitNot BitcastF32ToI32 BitcastI32ToF32 ShiftLeft
ShiftRightLogical ShiftRightArithmetic Require IsFinite IsNan SquareRoot Floor
Ceiling RoundNearestEven ConvertF32ToI32 ConvertI32ToF32
```

The encoder maps a future `ScalarOpcode` that has no version-1 spelling to
`OgdlDocumentErrorKind::UnsupportedValue`. The decoder rejects any other
spelling as `UnknownVariant`.

#### `Reduce`

```text
kind
	Reduce
		operator	Sum | Product | Minimum | Maximum | Any | All
		axes
			axis	<usize>       # one line per reduced axis
		keep_dimensions	true | false
		result	Value | Index | ValueAndIndex
		tree_lanes	<u32>
```

The axis list is nonempty, sorted, and duplicate-free after `AxisSet::new`,
and every axis must be within the input rank. `tree_lanes` must be a power of
two in `1..=1024`. `Any` and `All` require an I32 input. `Index` and
`ValueAndIndex` require `Minimum` or `Maximum`; index outputs are I32, value
outputs retain the input dtype, and all result shapes are derived from the
reduced axes and `keep_dimensions`. A minimum or maximum over an empty reduced
axis is rejected because no implicit identity is defined. Reducing every input
axis with `keep_dimensions` false produces the explicit scalar payload shape
`[1]`; keeping dimensions instead retains one extent per reduced axis.

#### `Scan`

```text
kind
	Scan
		operator	Sum | Product | Minimum | Maximum | Any | All
		axis	<usize>
		mode
			Inclusive
		# or
			Exclusive
				identity
					<F32Bits or I32 variant>
		reverse	true | false
		tree_lanes	<u32>
```

`mode` has exactly one variant child. An exclusive identity's literal dtype
must equal the input dtype. The axis must be in rank, `tree_lanes` has the same
power-of-two bound as reduction, `Any` and `All` require I32, and the one output
must have the input dtype and shape.

#### `Contraction`

```text
kind
	Contraction
		batch_axes
			pair
				left	<usize>
				right	<usize>
		contract_axes
			pair
				left	<usize>
				right	<usize>
```

There must be at least one contracted pair. Every pair is in bounds, each
operand axis is used at most once across both lists, paired extents match, and
the two inputs and one output have the same dtype. The output shape is the
ordered batch extents followed by unused left axes and unused right axes, with
`[1]` used when no output axes remain.

#### `Gather`

```text
kind
	Gather
		axis	<usize>
		bounds	Reject | Clamp | Wrap
```

The operation has two inputs and one output. The index input is I32. The output
dtype equals the source input dtype, and the output shape replaces the source
axis with the complete index shape. The axis must be valid for the source rank.

#### `Scatter`

```text
kind
	Scatter
		axis	<usize>
		bounds	Reject | Clamp | Wrap
		conflict
			UniqueIndices
		# or
			Atomic
				operation	Exchange | Add | Minimum | Maximum
				ordering	Relaxed | Acquire | Release | AcquireRelease | SequentiallyConsistent
```

Scatter has three inputs, one output, and the same source/output dtype. Its
index input is I32, and its axis must be valid for the source rank. The output
shape equals the source shape; the update shape must equal the source shape with
the selected axis replaced by the index shape.
`UniqueIndices` and `Atomic` are exact conflict variants. Atomic operation and
ordering are preserved as semantic fields for lowering.

#### `Histogram`

```text
kind
	Histogram
		bins	<u32>
		weighted	true | false
		ordering	Relaxed | Acquire | Release | AcquireRelease | SequentiallyConsistent
```

An unweighted histogram has one input, an I32 output, and one output dimension
of `bins`. A weighted histogram has two inputs, requires F32 weights with the
same shape as the sample input, and has an F32 output. `bins` is in
`1..=i32::MAX`.

#### `Sort`

```text
kind
	Sort
		axis	<usize>
		direction	Ascending | Descending
		stable	true | false
		emit_indices	true | false
```

Sort has one input. It has one output unless `emit_indices` is true, in which
case the second output is an I32 index tensor. Value and index output shapes
equal the input shape. The axis must be in rank and its extent must fit an
I32 index.

#### `IndexMap`

```text
kind
	IndexMap
		start	<i32>
		element_step	<i32>
		iteration_step	<i32>
		modulus
			None
		# or
			Some	<i32>
```

IndexMap has no inputs and one I32 output. A present modulus must be strictly
positive. The runtime evaluates the affine source using checked intermediates;
the optional modulus uses Euclidean remainder before the I32 result is stored.

#### `Random`

```text
kind
	Random
		distribution
			UniformF32
		# or NormalF32, BernoulliI32, or UniformI32
		key
			seed_low	<u64>
			seed_high	<u64>
			stream	<u64>
		philox_rounds	<u8>
```

The distribution variants with fields are:

```text
distribution
	BernoulliI32
		probability_bits	<u32>

distribution
	UniformI32
		low	<i32>
		high_exclusive	<i32>
```

Random has no inputs and one output. `UniformF32` and `NormalF32` produce F32.
`BernoulliI32` preserves the probability as raw F32 bits and requires a finite
probability in `[0, 1]`; `UniformI32` requires `low < high_exclusive` and
produces I32. `philox_rounds` must equal exactly `10`, making the Recipe-owned
Philox4x32-10 choice explicit rather than a backend default.

## Strict decoding rules

After parsing, `decode_graph` applies these checks in order:

1. There must be exactly one root named `RecipeIR`.
2. The root must contain exactly the required field set
   `schema`, `version`, `tensors`, and `nodes`. Field order is not significant,
   but every unknown or duplicate child is rejected and every required child
   must be present.
3. `schema` must contain one leaf value exactly equal to
   `CalculationGraph`; `version` must contain one leaf value exactly equal to
   `1`.
4. Every collection child must have the exact item name expected by its
   decoder. Each scalar field must have one leaf child. Each enum/variant
   wrapper must have exactly one child, and leaf variants must have no children.
5. Numeric text is parsed directly with the target Rust integer type. It is
   decimal, in range, and has no accepted surrounding whitespace. Booleans are
   exactly lowercase `true` or `false`. Enum and variant spellings are exact
   and case-sensitive.
6. The typed graph is constructed and passed to `CalculationGraph::validate`.

Required-field checking is shared by every record decoder. Consequently an
extra child such as `tensor.debug`, a repeated `dtype`, or an omitted
`layout.strides` produces a document error before semantic validation. Empty
collections are valid where the typed model permits them, because their field
node simply has no item children. A collection containing a wrong item name is
not treated as an unknown extension; it is an `UnknownField` document error.

The decoder preserves source ordering for tensors, nodes, kernel references,
alias rules, axis pairs, scalar definitions, instructions, and outputs. It
does not sort IDs or deduplicate collections. Any ordering requirements that
are semantic, such as scalar definition order or alias position, are checked
by the corresponding validator.

## Semantic validation after decoding

`CalculationGraph::validate` is the final admission gate for both decoded and
newly constructed graphs. It builds a tensor index, validates every tensor,
then validates each kernel and its primitive. It also checks graph-wide
producer and dependency invariants:

* tensor IDs are unique;
* kernel IDs are unique;
* every kernel input and output refers to a declared tensor;
* every output tensor has at most one producer;
* an external input cannot also be produced;
* every non-external tensor has a producer; and
* the producer dependency graph is acyclic, including rejection of a kernel
  that consumes its own output.

Kernel validation checks the complete alias matrix before dispatching to the
primitive-specific validator. Primitive checks enforce arity, dtype, shape,
axis, scalar-program, and parameter contracts listed above. Failures are
reported as `LanguageError` with a `LanguageErrorKind` and optional kernel and
value identities. The codec wraps that error as `OgdlCodecError::InvalidGraph`;
it does not turn an invalid graph into a partially decoded value.

The scalar and elementwise checks are layered rather than implied by parsing:

* scalar input, constant, and instruction-result IDs share one namespace and
  must be unique;
* every scalar instruction operand must already be defined, its operand count
  must match the opcode arity, and its declared dtype must equal the opcode's
  result dtype;
* a scalar program must expose at least one output, and every output ID must be
  defined;
* an elementwise kernel's tensor input and output counts must match the scalar
  program, tensor input dtypes must match scalar input dtypes, and tensor
  inputs must broadcast to every scalar output shape and dtype; and
* an elementwise kernel cannot have zero tensor inputs, even if its scalar
  program contains only constants.

The remaining primitive checks add the contracts that cannot be represented by
field spelling alone. Reduction and scan validate fixed tree lanes, axis
rank, truth-value dtype, and result/identity dtype. Contraction validates
pair bounds, pair uniqueness across both operands, matching paired extents,
and its derived output shape. Gather and scatter validate I32 indices and
derived index/update shapes. Histogram validates weighted arity, weight shape
and F32 dtype, bin range, and output dtype/shape. Sort validates output arity,
axis rank, int32 index extent, and optional index output dtype/shape. IndexMap
validates zero inputs, one I32 output, and positive optional modulus. Random
validates zero inputs, one output, exact Philox round count, distribution
output dtype, finite Bernoulli probability, and an increasing integer range.

The graph-level validation categories reachable from this codec include empty
or overflowing shapes, invalid axes, duplicate axes, byte-size or layout
overflow, invalid layouts, duplicate or unknown tensors, duplicate kernels,
duplicate producers, missing producers, cycles, arity and dtype mismatches,
shape mismatches, invalid scalar programs, invalid primitive parameters, and
work-count overflow when a caller asks a kernel for its priced work. The last
category is not introduced by decoding itself, but the same validated graph is
used by work accounting and planning.

## Error surface and failure boundaries

`OgdlCodecError` is deliberately split by the stage that failed:

| Variant | Source and display form |
| --- | --- |
| `Syntax(ParseError)` | `Graph::parse` rejected empty node text, an indentation jump, or a bare carriage return. The source error carries one-based line and column plus a zero-based byte offset. Display starts with `invalid OGDL syntax:`. |
| `Document { kind, path, detail }` | The parsed forest is not a `RecipeIR` document. Display is `invalid Recipe IR at <path>: <kind>: <detail>`. |
| `InvalidGraph(LanguageError)` | Records decoded, but tensor, kernel, primitive, scalar, producer, or cycle validation failed. Display starts with `invalid calculation graph:`. |
| `Build(GraphError)` | Encoding could not append a graph node, for example because a node text was empty or contained a structural character, or because an arena parent was unknown. Display starts with `cannot encode Recipe IR graph:`. The current encoder uses fixed valid names and graph-owned parents, so this boundary protects the arena API rather than providing a fallback. |

`OgdlDocumentErrorKind` has these exact categories:

```text
InvalidRoot MissingField DuplicateField UnknownField UnknownVariant
InvalidNumber InvalidBoolean UnexpectedChildren UnsupportedValue
```

Document paths identify the failing record and collection position, for
example `RecipeIR.tensors.tensor[0].shape.extent[0]`,
`RecipeIR.nodes.node[0].kernel.kind.Reduce.axes.axis[1]`, or
`RecipeIR.nodes.node[0].kernel.kind.Elementwise.program.instructions.instruction[2].opcode`.
`required_fields` rejects an unknown child immediately, reports duplicate
fields at the second occurrence, and reports missing fields after scanning all
children. `only_child` reports a missing child or unexpected child count for
variant and scalar-value wrappers. Integer conversion failures are
`InvalidNumber`; only the exact strings `true` and `false` are accepted for
booleans.

The encoder has one additional version boundary. `scalar_opcode_name` returns
`None` for an opcode not assigned a version-1 spelling, producing
`UnsupportedValue` instead of emitting text that a version-1 decoder could not
interpret. This is an explicit failure, not a fallback opcode or a silently
changed operation.

## Callers and round-trip use

The direct callers use the codec as a representation boundary, not as a model
loader:

| Caller | Use of the language codec | Failure propagation |
| --- | --- | --- |
| `program/src/lib.rs` | `StaticCalculationProgram::to_ogdl_graph` copies the encoded `RecipeIR` subtree under the outer `RecipeProgram` root. Its decoder copies that second root into a new `Graph` and calls `CalculationGraph::from_ogdl_graph`. | `OgdlCodecError` becomes `ProgramError::Graph`; parser and graph-build errors retain their stage. |
| `training/src/compile.rs::finish` | Validates the assembled graph, serializes it, decodes it again, then builds and round-trips the static program. | `LanguageError`, `OgdlCodecError`, and `ProgramError` become their corresponding `TrainingCompileErrorKind` values. |
| `training/src/inference.rs` dense and KNN finish paths | Performs the same graph canonicalization and decode round trip before accepting the compiled inference program. | The inference compile result preserves the language, OGDL, and program error boundaries. |
| `training/src/checkpoint.rs::compiled_training_program_digest` | Hashes the canonical static-program OGDL, whose nested `RecipeIR` is emitted by this codec. | A serialization failure is reported as a checkpoint manifest error. |

The compiler round trips are intentional: they prove that the graph emitted by
operation and model lowering is accepted by the same strict decoder that will
consume the representation, and they establish the canonical text used by
program identity and digest calculations. They do not add a second graph
implementation or a test-only serializer.

## Minimal parseable graph

The following is a complete, semantically valid version-1 document. Every
indentation character in the block is a tab. It contains one I32 output
produced by an input-free `IndexMap` kernel; empty `inputs` and `alias_rules`
collections are represented by their field nodes with no children.

```text
RecipeIR
	schema	CalculationGraph
	version	1
	tensors
		tensor
			id	1
			dtype	I32
			shape	extent	1
			layout	offset_elements	0
				strides	stride	1
			storage_bytes	4
			external_input	false
			external_output	true
	nodes
		node
			kernel
				id	1
				inputs
				outputs	value	1
				alias_rules
				kind	IndexMap
					start	0
					element_step	1
					iteration_step	0
					modulus	None
```

The same tree can be supplied through `from_ogdl_graph` after parsing. A
version change that sets the version field to `2` reaches the parser
successfully but is rejected by the document decoder with `UnknownVariant` at
`RecipeIR.version`. A syntactically malformed indentation, empty node, or
bare carriage return fails earlier as `Syntax` and never reaches schema or
graph validation.
