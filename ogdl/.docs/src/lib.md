# `recipe-ogdl`

`recipe-ogdl` is Recipe's dependency-free, deliberately narrow textual graph
layer. The crate owns the syntax and the smallest in-memory representation
needed by Recipe's higher-level codecs. It does not own a model schema, a
calculation vocabulary, file I/O, byte encoding, or execution.

The implementation is rooted at [`ogdl/src/lib.rs`](../../src/lib.rs). That
file keeps the four implementation modules private, documents the syntax
profile, and re-exports the graph and error types that callers use. The public
contract is therefore one small surface:

```text
text input -> Graph::parse / str::parse -> ordered forest
ordered forest -> Graph::to_canonical_string / Display -> canonical text
```

The representation is an ordered rooted forest stored in an append-only arena.
It is not a general graph. A `NodeId` is local to one `Graph`, every node has at
most one parent, and the crate has no shared references, anchors, links, or
cycles.

## Source and module surface

`lib.rs` has the following structure:

| Source item | Visibility | Responsibility |
| --- | --- | --- |
| `mod error` | Private | `SourceLocation`, parse failures, graph-building failures, and their display implementations. |
| `mod graph` | Private module, public items re-exported | `NodeId`, `Node`, `Graph`, graph construction, lookup, equality, `FromStr`, and `Display`. |
| `mod parser` | Private | Converts the tab/newline syntax into a `Graph`; reached through `Graph::parse`. |
| `mod serializer` | Private | Emits canonical text; reached through `Graph::to_canonical_string` and `Graph`'s `Display` implementation. |
| `pub use error::{...}` | Public crate root | `GraphError`, `NodeTextErrorKind`, `ParseError`, `ParseErrorKind`, and `SourceLocation`. |
| `pub use graph::{...}` | Public crate root | `Graph`, `Node`, and `NodeId`. |

There is no public `parser` or `serializer` module. Callers must use the
methods and trait implementations on `Graph`. The crate also has
`#![forbid(unsafe_code)]` and `#![deny(missing_debug_implementations)]`.
`recipe-ogdl` declares no Cargo dependencies.

The root package depends on this crate as `recipe-ogdl` and exposes it to
advanced callers through [`src/facade.rs`](../../../src/facade.rs)'s
`recipe::engine::ogdl` alias. The root package does not copy or wrap these
types into its declaration facade.

## The ordered-forest model

### `NodeId`

`NodeId` is a `Copy` value containing an arena index. Its field is private, so a
caller can obtain an identity only from a graph operation or an existing node
relationship. `NodeId::index(self) -> usize` exposes the index for diagnostics
and integration code. It derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`,
`PartialOrd`, `Ord`, and `Hash`.

An identity is meaningful only for the `Graph` from which it came. Passing an
identity from another graph to `node` or `append_child` does not make it valid:
`node` returns `None`, and `append_child` returns
`GraphError::UnknownParent`.

### `Node`

`Node` has private fields and immutable accessors:

| Method | Result | Meaning |
| --- | --- | --- |
| `text(&self)` | `&str` | The exact node text. Spaces and all non-structural Unicode are literal. |
| `parent(&self)` | `Option<NodeId>` | The parent identity, or `None` for a root. |
| `children(&self)` | `&[NodeId]` | Child identities in declaration/append order. |

`Node` derives `Clone`, `Debug`, `PartialEq`, and `Eq`. A caller cannot mutate a
node's text or relationship directly. The graph is the only owner of the arena
and keeps the parent and child links consistent.

### `Graph`

`Graph` contains a private node arena and a private ordered root list. It
derives `Clone`, `Debug`, and `Default`. Its public methods are:

| Method | Result | Contract |
| --- | --- | --- |
| `Graph::new()` | `Graph` | Const constructor for an empty forest. |
| `Graph::parse(input)` | `Result<Graph, ParseError>` | Parse the crate's text profile. Empty input produces an empty graph. |
| `len()` | `usize` | Number of nodes in the arena. |
| `is_empty()` | `bool` | Whether the arena contains no nodes. |
| `roots()` | `&[NodeId]` | Root identities in source or append order. |
| `node(id)` | `Option<&Node>` | Lookup by an identity from this graph. |
| `nodes()` | `impl ExactSizeIterator<Item = (NodeId, &Node)>` | Iterate all nodes in arena append order. |
| `append_root(text)` | `Result<NodeId, GraphError>` | Validate text and append a new root. |
| `append_child(parent, text)` | `Result<NodeId, GraphError>` | Require an existing parent, validate text, and append one child. |
| `to_canonical_string()` | `String` | Serialize the forest using canonical LF and tab placement. |

`append_root` and `append_child` accept any `Into<String>`. Node text must be
nonempty and must not contain a tab, carriage return, or line feed. The
character reported by `GraphError::InvalidNodeText` is one-based in the
Unicode scalar sequence used by `str::chars()`. `append_child` checks the
parent before validating text, so an unknown parent is reported as
`UnknownParent` even if the supplied text is also invalid.

`Graph` implements structural `PartialEq` and `Eq`. Equality compares root
order, node text, and ordered child structure. It does not require the two
graphs to have the same arena allocation history or the same numerical
`NodeId` values. It also implements `FromStr<Err = ParseError>`, so
`input.parse::<Graph>()` is the same parsing boundary as `Graph::parse`, and
`Display`, which writes `to_canonical_string()`.

The arena is append-only through the public construction methods. Each new
node receives the next local index. A root is added to `roots`; a child is
added to its parent's `children`. There is no public removal, reparenting, or
cycle-forming operation.

## Text profile and parser behavior

The syntax is OGDL-derived, but it intentionally gives spaces no structural
meaning:

- Leading tab characters select the parent depth for a line.
- Every later tab on that line separates another node in one parent-child
  chain.
- A line feed starts another chain. CRLF is accepted and the carriage return
  is removed from the line. A bare carriage return is rejected.
- Node text is every non-tab character in its segment, including spaces. It
  must be nonempty and cannot contain tab, carriage-return, or line-feed
  characters.

For example, the following source has one root, two children of that root, and
one grandchild:

```text
system	master	device
	workers	worker 0
		worker 1
```

The first line is a three-node chain. The second line starts at depth one, so
`workers` is a child of `system`, followed inline by `worker 0`. The third line
starts at depth two, so `worker 1` is a child of `workers`. Spaces in `worker 0`
are part of the text.

The parser keeps an ancestor stack for the most recently parsed chain. For a
line with `n` leading tabs, `n` must not exceed the current stack length. The
line then truncates the stack to depth `n` and appends each tab-separated
segment in order. Consequently, a line cannot skip an ancestor, but it can
continue a prior chain or start another root at depth zero.

The parser reports these syntax failures as `ParseError`:

| `ParseErrorKind` | When it is returned |
| --- | --- |
| `EmptyNode` | A blank line, a line made only of leading tabs, or two adjacent tabs that would create an empty segment. |
| `IndentationJump { found, maximum }` | Leading indentation is deeper than the available ancestor stack. `found` is the requested depth and `maximum` is the current stack length. |
| `BareCarriageReturn` | A carriage return is not immediately followed by line feed. Use LF or CRLF. |

An empty input string is the one empty-document case and returns
`Graph::new()`. One trailing LF (or CRLF) is allowed because it terminates the
final line; an interior blank line is not. There is no whitespace trimming, comment
syntax, quote syntax, escape syntax, or implicit empty value.

`ParseError` stores both the `ParseErrorKind` and a `SourceLocation`. Its
accessors are `kind() -> &ParseErrorKind` and `location() -> SourceLocation`.
`SourceLocation::new` is a const constructor; `line()` and `column()` are
one-based, while `offset()` is a zero-based UTF-8 byte offset into the original
input. Columns are counted in Unicode scalar values, not bytes. `ParseError`
implements `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, and
`std::error::Error`. Its display text includes the kind and line/column; use
the accessors when the byte offset is needed.

## Canonical serialization

The serializer is private and is exposed only through
`Graph::to_canonical_string()` and `Display`. It walks roots and children in
order. The first child of a node is written inline after one tab; later
siblings start on new lines at their depth. Later roots start on new lines at
depth zero. Canonical output always uses LF, never emits a trailing LF, and
returns the empty string for an empty graph.

The construction equivalent of the example above is:

```rust
let mut graph = recipe_ogdl::Graph::new();
let system = graph.append_root("system")?;
let master = graph.append_child(system, "master")?;
graph.append_child(master, "device")?;
let workers = graph.append_child(system, "workers")?;
graph.append_child(workers, "worker 0")?;
graph.append_child(workers, "worker 1")?;
```

Its canonical text is a normalized representation of the same ordered forest:

```text
system	master	device
	workers	worker 0
		worker 1
```

Parsing and serializing is therefore a graph-preserving normalization boundary,
not a promise to reproduce the caller's original line layout. For example,
`system\n\tmaster` and `system\tmaster` describe the same forest and serialize
to the latter form. The serializer never invents node text or schema values;
it only emits relationships already held by the graph.

## Error surface

All public error types are re-exported at the crate root. They are intentionally
small and syntax-focused; higher-level crates wrap them in errors that add a
document schema, path, or artifact context.

### `GraphError`

`GraphError` is `#[non_exhaustive]` and has:

- `InvalidNodeText { kind: NodeTextErrorKind, character: usize }`, for an
  empty node or forbidden tab, carriage return, or line feed in text supplied
  to an append method;
- `UnknownParent(NodeId)`, when `append_child` is given an identity absent from
  this graph.

`NodeTextErrorKind` is also `#[non_exhaustive]` and currently contains
`Empty`, `Tab`, `CarriageReturn`, and `LineFeed`. `GraphError` implements
`Clone`, `Debug`, `PartialEq`, `Eq`, `Display`, and `std::error::Error`.

### `ParseError`

`ParseErrorKind` is `#[non_exhaustive]` and currently contains `EmptyNode`,
`IndentationJump { found, maximum }`, and `BareCarriageReturn`. `ParseError`
is a location-bearing value with `new`, `kind`, and `location` accessors. It
implements `Clone`, `Debug`, `PartialEq`, `Eq`, `Display`, and
`std::error::Error`.

Neither error type performs recovery. A parse or append operation returns the
first observed failure and does not return a partial public graph.

## Workspace consumers

The direct Cargo consumers are the root package, `recipe-language`,
`recipe-program`, `recipe-ingest`, and `recipe-training`. A repository search
shows no other crate importing `recipe-ogdl` directly. Each consumer owns its
own schema and semantic checks; `recipe-ogdl` supplies only syntax and tree
relationships.

### `recipe-language`

[`language/src/ogdl.rs`](../../../language/src/ogdl.rs) imports `Graph`,
`GraphError`, `NodeId`, and `ParseError` to implement the typed
`CalculationGraph` codec:

- `CalculationGraph::to_ogdl_graph` validates the typed graph, creates a
  `Graph`, and appends a `RecipeIR` root with `schema`, `version`, `tensors`,
  and `nodes` fields. It uses `append_root` and `append_child` for every
  record, list item, scalar, and primitive variant.
- `CalculationGraph::to_ogdl` obtains that graph and calls
  `to_canonical_string`.
- `from_ogdl` calls `Graph::parse`, then `from_ogdl_graph` walks `roots`,
  `node`, `Node::text`, and `Node::children`. It requires exactly one
  `RecipeIR` root, exact schema/version values, known fields, one value child
  for scalar fields, and valid typed calculation content before calling the
  calculation graph validator.
- Syntax failures become `OgdlCodecError::Syntax`; construction failures
  become `OgdlCodecError::Build`; typed graph failures become
  `OgdlCodecError::InvalidGraph`.

This codec is the semantic Recipe IR boundary. The OGDL crate does not know
what `RecipeIR`, a tensor, a primitive, or a scalar instruction means.

### `recipe-program`

[`program/src/lib.rs`](../../../program/src/lib.rs) imports the same graph
types for `StaticCalculationProgram` serialization:

- `to_ogdl_graph` validates the static program, writes a `RecipeProgram` root,
  appends loop, domain, and metric fields, then copies the separately encoded
  `RecipeIR` subtree into the same graph.
- `to_ogdl` serializes the resulting two-root graph canonically.
- `from_ogdl` parses text through `Graph::parse`. `from_ogdl_graph` requires
  exactly two roots in order: `RecipeProgram` followed by `RecipeIR`. It
  validates the supported program versions, exact fields, scalar leaves, and
  iteration domains, then copies the calculation subtree into a new `Graph`
  for `CalculationGraph::from_ogdl_graph`.
- `GraphError` and `ParseError` are converted into the program's own
  `ProgramError`; all program-level schema and lifecycle checks remain in
  `recipe-program`.

The two-root document is a consumer convention. A plain `Graph` permits any
number of roots, including zero; it does not require `RecipeProgram` or
`RecipeIR` labels.

### `recipe-ingest`

[`ingest/src/gguf_ogdl.rs`](../../../ingest/src/gguf_ogdl.rs) imports only
`Graph` and `NodeId`. Its `structural_ogdl_to_gguf` path parses structural GGUF
text with `Graph::parse`, then uses `roots`, `node`, `Node::text`, and
`Node::children` while enforcing the GGUF-specific document:

- exactly one root named `gguf`;
- seven ordered root fields, including the exact structural schema and version;
- nested metadata, tensor, payload, and block shapes;
- no unexpected children for scalar nodes and no duplicate metadata keys.

`ParseError` is wrapped as `GgufOgdlErrorKind::InvalidSyntax`; all GGUF
structure, value, size, and arithmetic failures are produced by ingest. The
opposite conversion functions write their structural tab-indented text
directly, and stream variants avoid building a `Graph`; the graph crate is not
a GGUF reader, writer, or binary validator.

### `recipe-training` checkpoint codecs

The training crate has three direct OGDL integrations, each with a distinct
artifact schema.

#### Dense checkpoints

[`training/src/checkpoint.rs`](../../../training/src/checkpoint.rs) aliases
`Graph` to `OgdlGraph` and `NodeId` to `OgdlNodeId` in `decode_checkpoint`.
After bounded UTF-8 admission, it parses the text and checks the graph node
count against decode limits. Its `Decoder` then uses `roots`, `node`,
`Node::text`, and `Node::children` to enforce the single `recipe` root, exact
fields, scalar and tagged values, payload chunks, and all versioned model
semantics. Unknown identities, missing fields, duplicate fields, unexpected
children, and invalid values are reported as checkpoint errors with a typed
path.

The dense checkpoint encoder is deliberately separate: `encode_artifact`
writes the tab-indented canonical form directly to an `io::Write` with
`writeln!`; it does not construct a `recipe-ogdl::Graph` or call the graph
serializer. The parser still supplies the common syntax boundary for decoding.

#### Bayesian semantic models

[`training/src/bayes_checkpoint.rs`](../../../training/src/bayes_checkpoint.rs)
uses `Graph::new`, `append_root`, `append_child`, `Graph::parse`, `roots`,
`node`, `Node::text`, and `Node::children`. `BayesModelArtifact::encode`
builds a canonical graph rooted at the Bayesian artifact label and
`decode_bayes_model` parses bounded UTF-8 text, requires one exact root, and
performs strict field and scalar validation. `save` persists the encoded
semantic model only at a `.ogdl` path. The graph crate supplies no Bayesian
meaning, smoothing rule, parent/child schema, or payload interpretation.

#### KNN semantic models

[`training/src/knn_checkpoint.rs`](../../../training/src/knn_checkpoint.rs)
follows the same pattern. `KnnModelArtifact::encode` builds its ordered graph
with append operations and serializes it with `to_canonical_string`.
`decode_knn_model` parses bounded UTF-8 text, requires one KNN root, and walks
strict fields, scalar values, tags, vectors, spans, masks, and outputs through
the graph accessors. `save` writes the canonical semantic model to `.ogdl`.
KNN validation, feature lowering, limits, and resume compatibility remain in
the training crate.

### Root facade exposure

[`src/facade.rs`](../../../src/facade.rs) re-exports the crate as
`recipe::engine::ogdl` alongside the other implementation crates. This is an
organizational alias, not a second implementation or a bypass around the
higher-level codecs. The root declaration API does not expose `Graph` as a
builder for user model declarations; callers needing the low-level ordered
forest use the explicit engine namespace or depend on `recipe-ogdl` directly.

## Boundaries and non-goals

The following behavior is intentionally outside this crate:

- OGDL shared references, anchors, links, cycles, and general graph
  reachability;
- comments, quoting, escapes, delimiters other than tab/newline, or a way to
  represent an empty node text;
- typed numbers, booleans, binary payloads, schemas, version negotiation, and
  unknown-field policy;
- reading or writing files, streams, sockets, or byte buffers;
- data ingestion, model/checkpoint semantics, calculation lowering, scheduling,
  native compilation, and execution;
- resource limits beyond normal allocation behavior. Consumers that process
  untrusted or bounded artifacts must impose byte, node, payload, and schema
  limits before or after parsing, as the training and ingest codecs do.

Higher-level formats may encode values as text such as hexadecimal or JSON,
but that encoding belongs to the owning consumer. A node's text remains opaque
to `recipe-ogdl`; only the three structural characters and emptiness are
interpreted.

When a consumer needs a stricter document, it must validate the parsed forest
itself: check root count and labels, field order or uniqueness, leaf shape,
variants, numeric spelling, and semantic invariants. `Graph::parse` succeeding
means only that the text can be represented by this ordered forest.
