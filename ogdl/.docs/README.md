# `recipe-ogdl`

`recipe-ogdl` is Recipe's dependency-free ordered graph syntax. It is the
lowest textual layer used by Recipe's `.ogdl` documents. The crate parses a
UTF-8 string into an ordered rooted forest, lets callers build the same forest
through an append-only arena API, and emits one deterministic canonical text
form. It does not know what a field, schema, model, tensor, program, or GGUF
record means. The packages that own those meanings layer their own strict
decoders over this graph.

The deliberate boundary is:

```text
UTF-8 text
    |
    v
recipe-ogdl parser  ->  Graph (ordered forest, arena-owned NodeId values)
    ^                              |
    |                              v
    +---- canonical serializer <---+

recipe-language, recipe-program, recipe-ingest, and recipe-training
interpret the graph and own their document schemas and semantic validation.
```

The syntax has only two structural controls: tabs and line endings. A tab at
the beginning of a line selects that line's parent depth. Every later tab on
the same line starts
another child in one inline chain. A line ending finishes the chain and starts
the next one. Ordinary spaces, including leading, trailing, and repeated
spaces, are literal node text.

For example, the following text is one root followed by a two-node child chain
on each following line. The code block writes each ASCII tab as the visible
two-character marker `\t`; actual input contains tab bytes.

```text
system\tmaster\tdevice
\tworkers\tworker 0
\t\tworker 1
```

The first line creates `system -> master -> device`. The second line selects
`system` as its parent and creates `workers -> worker 0`; the third selects
`workers` and creates `worker 1`. The graph is not a general graph: it has no
shared references, anchors, links, cycles, comments, escapes, schema syntax,
or binary syntax. A consumer may put a schema label in a node's text, but the
OGDL crate treats that label as ordinary text.

## Package and module ownership

[`Cargo.toml`](../Cargo.toml) declares package `recipe-ogdl`, version `0.1.0`,
Rust edition 2024, MIT license, and the description "Recipe's dependency-free
OGDL-derived ordered graph syntax". There are no dependencies, feature flags,
build scripts, binaries, examples, or optional integrations. The manifest
forbids unsafe Rust. The crate root additionally denies missing `Debug`
implementations. `cargo tree -p recipe-ogdl` therefore contains only this
package.

All implementation modules are private. [`src/lib.rs`](../src/lib.rs) is the
only facade:

```text
src/lib.rs
├── error.rs       source locations and graph/parser failures
├── graph.rs       NodeId, Node, Graph, arena mutation, equality, Display
├── parser.rs      text -> Graph state machine (crate-private entrypoint)
└── serializer.rs  Graph -> canonical text (crate-private entrypoint)
```

The implementation dependency direction is intentionally small:

```text
error.rs      -> NodeId
parser.rs     -> Graph, ParseError, ParseErrorKind, SourceLocation
serializer.rs -> Graph, NodeId
graph.rs      -> GraphError, NodeTextErrorKind, ParseError,
                parser::parse, serializer::serialize
lib.rs        -> declares all four modules and re-exports their public types
```

`parser::parse`, `serializer::serialize`, `Graph::push_node`, and
`graph::validate_text` are not public. This keeps parser construction and
serialization coupled to the graph's private representation. Downstream
crates use the root re-exports rather than module paths:

```rust
use recipe_ogdl::{Graph, GraphError, Node, NodeId, ParseError, ParseErrorKind, SourceLocation};
```

The root re-exports are:

| Export | Owner | Purpose |
| --- | --- | --- |
| `NodeId` | `graph.rs` | Opaque, graph-local arena identity. |
| `Node` | `graph.rs` | Immutable text plus parent and ordered-child views. |
| `Graph` | `graph.rs` | Ordered rooted forest and append-only builder. |
| `SourceLocation` | `error.rs` | One-based line/column and zero-based byte offset. |
| `NodeTextErrorKind`, `GraphError` | `error.rs` | Failures from direct graph construction. |
| `ParseErrorKind`, `ParseError` | `error.rs` | Failures from textual parsing. |

No IO is performed here. Callers read or write files, convert bytes to UTF-8,
and apply their own size, schema, and semantic limits before or after calling
this crate.

## Syntax contract

The parser can be described by this grammar plus one state constraint:

```text
document       ::= empty | line (newline line)* [newline]
newline        ::= LF | CRLF
line           ::= TAB* segment (TAB segment)*
segment        ::= one or more UTF-8 characters other than TAB, CR, or LF
```

`line` is never empty. The optional final `newline` is accepted, but an empty
line between two newlines is rejected. The state constraint is applied before
the line's segments are appended:

```text
depth = number of leading TAB bytes on the current line
depth <= number of nodes in the previous line's current ancestor path
parent = none                         when depth = 0
       = previous_path[depth - 1]     when depth > 0
```

After selecting `parent`, the parser truncates the previous path to `depth`
entries and appends every `segment` as a child of the preceding segment. The
new path is the selected prefix followed by the complete inline chain. A depth
of zero therefore starts a new root and makes all nodes from the preceding
line unavailable as parents. A depth greater than the available path would
skip an ancestor and is rejected.

There is no quoting or escaping layer. A tab always has structural meaning,
so a tab cannot be part of node text. Carriage return and line feed are also
reserved. A Unicode character that merely looks like whitespace is ordinary
text unless it is the ASCII tab, carriage return, or line feed byte/code point.
NUL, punctuation, and all ordinary spaces remain valid node text.

### Accepted and rejected documents

The exact parser behavior is easiest to see from representative inputs:

| Input | Result |
| --- | --- |
| `""` | Empty graph. |
| `"a"` | One root named `a`. |
| `"a\n"` or `"a\r\nb"` | Final LF is ignored; CRLF separates roots and is normalized by serialization. |
| `"a\n\n"` or `"\n"` | `ParseErrorKind::EmptyNode`. |
| `"a\t"` or `"a\t\tb"` | `EmptyNode` at the trailing or consecutive tab. |
| `"\tchild"` when no previous path exists | `IndentationJump { found: 1, maximum: 0 }`. |
| A depth larger than the previous path | `IndentationJump { found, maximum }`. |
| A bare `\r` | `BareCarriageReturn`; only LF or CRLF is a line ending. |

The parser reports the first failure encountered while scanning. It checks a
bare carriage return before it parses the line containing it. On a newline it
parses the preceding line immediately, so an empty line fails before later
input is considered. A final LF or CRLF does not create a second empty line.

## In-memory representation

`Graph` is an ordered rooted forest backed by two private vectors:

```text
Graph
├── nodes: Vec<Node>       append order is the arena order
└── roots: Vec<NodeId>     root order is insertion order

Node
├── text: String            immutable through the public API
├── parent: Option<NodeId>
└── children: Vec<NodeId>   ordered child identities
```

`NodeId` wraps a `usize` and exposes only `index()`. There is intentionally no
public constructor. Callers obtain IDs from `append_root`, `append_child`,
`roots`, `children`, or `nodes`; identities are local to the graph that issued
them. Passing an ID from another graph violates that ownership contract. It
returns `GraphError::UnknownParent` when its numeric index is absent in the
target, but `NodeId` carries no graph token, so a coincident in-range index
cannot be detected and addresses the target's node. A graph's append operation
never renumbers existing IDs.

`Node` exposes `text()`, `parent()`, and `children()`. The text is borrowed and
the child list is an immutable slice, so callers cannot mutate an edge or node
text without going through `Graph`. `Graph::node(id)` returns `Option<&Node>`
and is the safe lookup boundary for an ID. `Graph::roots()` returns the ordered
root slice. `Graph::nodes()` yields an `ExactSizeIterator` of `(NodeId,
&Node)` in arena order. `len()` counts all nodes and `is_empty()` is equivalent
to `len() == 0`.

The arena is append-only. `append_root` validates text and pushes a root;
`append_child` first verifies that the parent belongs to this graph, validates
text, and then pushes the child and records the edge in the parent's child
list. There is no remove, reparent, mutable node, or arbitrary-ID insertion
API. The private `push_node` is shared by these methods and the parser, so all
public and parsed graphs use the same edge construction.

## Public API inventory

### `Graph`

| API | Contract |
| --- | --- |
| `Graph::new()` / `Default` | Construct an empty forest. `new` is `const`. |
| `Graph::parse(&str)` | Parse the syntax contract into a forest or return `ParseError`. |
| `FromStr for Graph` | Delegates to `Graph::parse`; the error type is `ParseError`. |
| `len()` / `is_empty()` | Query arena cardinality. |
| `roots()` | Borrow root IDs in insertion order. |
| `node(NodeId)` | Borrow one node when its ID belongs to this graph. |
| `nodes()` | Iterate every arena entry in append order with its local ID. |
| `append_root(impl Into<String>)` | Validate text and add a root, returning its ID or `GraphError`. |
| `append_child(NodeId, impl Into<String>)` | Validate parent ownership and text, then add an ordered child. |
| `to_canonical_string()` | Serialize the complete forest to canonical LF text. It never fails for a valid public graph. |

`Graph` implements `Display` by writing `to_canonical_string()`. It also
implements `Clone`, `Debug`, `Default`, `PartialEq`, and `Eq`. The custom graph
equality compares root count, node count, node text, and ordered child shape by
walking corresponding roots. It intentionally compares forest content rather
than graph-local numeric identities. The private parent and child invariants
make the parent pointers implied by that walk; callers must not treat a
`NodeId` from one equal graph as usable in the other graph.

### `NodeId` and `Node`

`NodeId` is `Copy`, `Clone`, ordered, hashable, and `Debug`-printable. Its
`index()` method exposes the underlying arena index for diagnostics and for
consumer-owned maps. It is not a globally stable identifier and cannot be
constructed from an arbitrary index through the public API.

`Node::text()` returns the exact node text, including spaces. `Node::parent()`
returns `None` for roots and the local parent ID otherwise. `Node::children()`
returns children in the exact order established by parsing or appending.

### Error and location values

`SourceLocation` stores private fields and has `const fn new(line, column,
offset)`, plus `line()`, `column()`, and `offset()` accessors. Parser-produced
line and column values are one-based. `offset` is a zero-based UTF-8 byte
offset into the original input. The public constructor is a value constructor,
not a validator; callers should treat parser-created locations as the trusted
source positions.

`GraphError` and `ParseError` both implement `Display` and
`std::error::Error`, and both are `Clone + Debug + PartialEq + Eq`. Their
enums are `#[non_exhaustive]`, so external matches must include a wildcard or
otherwise remain forward-compatible.

## Parser ownership and failures

[`src/parser.rs`](../src/parser.rs) owns the only text-to-graph operation. It
uses byte scanning for LF/CR detection, then `char_indices` for tab-delimited
segments. This combination is safe for UTF-8: ASCII control bytes cannot occur
inside a UTF-8 continuation sequence, and all slices are made at the line or
tab boundaries returned by the string APIs.

The parser's state is local to one call:

```text
input &str
  -> line number, line-start byte, cursor
  -> Graph
  -> ancestors: Vec<NodeId> for the latest line's inline path
```

It never retains a source buffer or global parser state. Its three failure
kinds are:

| `ParseErrorKind` | Trigger | Location details |
| --- | --- | --- |
| `EmptyNode` | Empty line, all-tab line, consecutive tab, or trailing tab. | The one-based character/column where the missing segment begins. |
| `IndentationJump { found, maximum }` | Leading-tab depth is greater than the previous line's available ancestor path. | Column 1 and the line's starting byte offset. |
| `BareCarriageReturn` | `\r` is not immediately followed by `\n`. | The CR's one-based character column and byte offset. |

`ParseError::kind()` borrows the enum and `ParseError::location()` returns the
copyable location. `Display` includes the human message and line/column, while
callers that need a machine-stable byte position use `location().offset()`.
No parser error is silently converted to an empty graph or a partial graph.
The graph accumulated before a failure is local and is discarded with the
failed `Result`.

## Graph-building validation and failures

[`src/graph.rs`](../src/graph.rs) owns `validate_text`. It rejects the first
forbidden character in Unicode character order and reports a one-based
character index, not a byte offset:

| `NodeTextErrorKind` | Rejected text |
| --- | --- |
| `Empty` | The empty string; the reported character is `1`. |
| `Tab` | ASCII `\t`, which would be structural syntax. |
| `CarriageReturn` | ASCII `\r`, which would be a line-ending control. |
| `LineFeed` | ASCII `\n`, which would split lines. |

`GraphError::InvalidNodeText { kind, character }` carries that kind and index.
`GraphError::UnknownParent(NodeId)` is returned before text conversion when a
parent index is not present in the target graph. An out-of-range ID obtained
from another graph reaches this path; an in-range foreign index is not
distinguishable from a local ID because `NodeId` is only a `usize`. A failed
append does not mutate the graph.

`GraphError` has no source location because direct construction does not come
from a document. Conversely, `ParseError` has no graph error variant because
the parser establishes the node-text constraints while it splits lines. The
two error families therefore identify their ownership boundary rather than
forming a catch-all error.

## Canonical serializer

[`src/serializer.rs`](../src/serializer.rs) owns `serialize`, which is exposed
only through `Graph::to_canonical_string()` and `Display`. It performs an
iterative depth-first walk with an explicit pending stack, so serialization
does not depend on the Rust call stack for graph depth. The output rules are:

1. The first root starts at byte zero. Later roots start on a new LF line with
   zero indentation.
2. The first child of a node is written inline after one tab. Later children
   start on LF lines indented to that node's child depth.
3. The same rule is applied recursively, so a single-child path is one inline
   tab chain and sibling branches are separate lines.
4. Output always uses LF, never CRLF, and never appends a final newline.
5. Node text is copied byte-for-byte. The graph API guarantees that it cannot
   contain a structural tab, CR, or LF.

For a graph built as `r` with children `c1` and `c2`, where `c1` has child
`gc`, canonical output is below. As above, each visible `\t` marker represents
one tab byte:

```text
r\tc1\tgc
\tc2
```

The empty forest serializes to the empty string. Parsing canonical output
reconstructs the same ordered forest, and parsing CRLF input followed by
serialization normalizes line endings to this LF form. The serializer uses an
internal `expect` when resolving a child ID. That is an assertion of the
private arena invariant, not a recoverable user input path: all public graph
mutation checks the parent before it records an edge, and parsed edges come
from the parser's own IDs.

Canonicalization also removes equivalent line-layout spellings. For example,
`a\n\tb` and `a\tb` parse to the same root with one child, and both serialize
as `a\tb` (with `\t` denoting one tab byte). Consumers that authenticate bytes,
such as a canonical model decoder, must compare or hash the canonical output
when that distinction matters.

## Consumer boundaries

The workspace has five direct Cargo consumers of `recipe-ogdl` in addition to
the crate itself. The root package also exposes it publicly:

| Package | Direct use | Schema or responsibility owned outside `recipe-ogdl` |
| --- | --- | --- |
| `recipe` | `src/facade.rs` re-exports `recipe_ogdl` as `recipe::engine::ogdl`; no root implementation code calls the raw API. | Advanced callers choose their own schema. |
| `recipe-language` | `language/src/ogdl.rs` builds and parses `Graph`; `CalculationGraph::{to_ogdl,to_ogdl_graph,from_ogdl,from_ogdl_graph}` are the public codec. | `RecipeIR` / `CalculationGraph` version `1`, required fields, enum spellings, numbers, and semantic graph validation. |
| `recipe-program` | `program/src/lib.rs` builds a `RecipeProgram` root and copies a `RecipeIR` subtree as a second root; it parses with `Graph::parse`. | `StaticCalculationProgram` version `2` (and legacy version `1`), iteration domains, metrics, root order, and program validation. |
| `recipe-ingest` | `ingest/src/gguf_ogdl.rs` parses the non-stream structural converter form and walks roots, nodes, and children. | GGUF v3 structure, typed metadata, tensor descriptors, payload fields, byte limits, and binary reconstruction. |
| `recipe-training` | `checkpoint.rs`, `bayes_checkpoint.rs`, and `knn_checkpoint.rs` parse or build model artifacts; the generic dense encoder writes canonical tab lines directly. | Checkpoint/model versions, field sets, payload encodings, limits, semantic invariants, and atomic file persistence. |

The package graph confirms the same direct set: `recipe`,
`recipe-ingest`, `recipe-language`, `recipe-program`, and `recipe-training`
depend on the local `recipe-ogdl` package. Other crates reach it transitively
through these owners. Consumers do not mutate an existing `Graph` after
parsing; `recipe-ingest` additionally has a separate streaming line reader
for its bounded CLI path. Each consumer owns the semantic checks that follow
the syntax boundary.

### `recipe-language`: typed calculation IR

`language/src/ogdl.rs` validates a `CalculationGraph` before encoding. It
constructs a one-root graph named `RecipeIR`, with scalar fields `schema
CalculationGraph` and `version 1`, then ordered `tensors` and `nodes` records.
`to_ogdl()` calls `to_ogdl_graph()` and the OGDL canonical serializer. Graph
construction failures become `OgdlCodecError::Build`; invalid source syntax
becomes `OgdlCodecError::Syntax(ParseError)`.

`from_ogdl()` first calls `Graph::parse`, then requires exactly one `RecipeIR`
root, exact schema/version text, required and unique fields, exact collection
item names, leaf values, canonical booleans/numbers, and known enum variants.
Only after those document checks does it call `CalculationGraph::validate`.
Missing, duplicate, unknown, or structurally unexpected fields are language
document errors, not parser errors. Node IDs are borrowed from the immutable
parsed graph and helpers use `expect` only after those IDs came from the same
graph's child slices.

Training and inference compilers use this codec as a real boundary. Their
`finish` paths construct a graph, validate it, serialize canonical OGDL, parse
it back, construct a static program, serialize that program, and parse it back.
This proves that the production graph and program artifacts survive the same
public text path used for persisted `.ogdl` data. The training checkpoint code
also hashes the canonical static-program text for its program digest, so a
serializer change is an artifact-identity change even when the semantic graph
is unchanged.

### `recipe-program`: two-root lifecycle envelope

`program/src/lib.rs` uses the forest property explicitly. The canonical
program document has two roots in this exact order:

```text
RecipeProgram  (schema StaticCalculationProgram, version 2)
RecipeIR       (the language codec's calculation graph subtree)
```

Version `1` omits the metrics field; version `2` includes it. The encoder
validates the static program, builds the first root with `Graph::append_*`,
serializes the calculation graph with `CalculationGraph::to_ogdl_graph`, and
copies that root subtree into the same output forest. The decoder calls
`Graph::parse`, requires exactly two roots and their names, enforces exact
program fields and iteration-domain values, copies the `RecipeIR` subtree into
a fresh `Graph`, and delegates semantic graph decoding to
`CalculationGraph::from_ogdl_graph`.

`ParseError` is wrapped as `ProgramError::Syntax`, graph-building failures as
`ProgramError::Build`, and language codec failures as `ProgramError::Graph`.
The program layer does not reinterpret tabs or line endings; those remain
`recipe-ogdl` syntax concerns.

### `recipe-ingest`: structural GGUF conversion

`ingest/src/gguf_ogdl.rs` has both in-memory and streaming converter paths.
`gguf_to_structural_ogdl` and its streaming counterpart emit fixed-depth tab
records such as `gguf`, `schema recipe-gguf-structural-v1`, typed metadata,
and tensor payload declarations. The non-stream reverse function,
`structural_ogdl_to_gguf`, calls `Graph::parse`, requires exactly one `gguf`
root and seven ordered fields, then walks `Node::children()` to decode metadata
and tensor payload fields before re-parsing the rebuilt bytes through the
bounded GGUF validator.

The production CLI deliberately uses the streaming functions instead. Their
`CanonicalLines` reader is a separate bounded line reader that requires LF,
rejects blank lines, tracks depth and line numbers, and makes multiple passes
over a seekable input. It does not call `Graph::parse`. Consequently the OGDL
crate's accepted CRLF/trailing-newline syntax describes the in-memory graph
API, while the CLI's structural converter applies its own stricter stream
contract.

### `recipe-training`: semantic model artifacts

The generic dense checkpoint decoder converts bytes to UTF-8, calls
`OgdlGraph::parse`, checks the resulting node count against its decode limit,
and walks `OgdlNodeId` values through a path-aware decoder. It then applies
checkpoint root, field, scalar, payload, version, and semantic-invariant
checks. Syntax failures are wrapped as the training decoder's
`InvalidSyntax`; an absent node identity is a decoder failure rather than an
OGDL panic.

The generic dense checkpoint encoder writes the same tab/newline shape directly
to an `io::Write` instead of constructing a second in-memory `Graph`. The
Bayesian and KNN model encoders do use `Graph::new`, `append_root`,
`append_child`, and `to_canonical_string`; their decoders use `Graph::parse`,
root/child traversal, and independent model limits. Bayesian decoding also
re-encodes and compares the canonical image, so a valid but noncanonical final
newline or CRLF spelling is rejected. KNN decoding validates canonical numeric
and payload values but does not compare a re-encoded document, so the OGDL
parser's accepted line-ending variants can reach the KNN semantic decoder.
These model crates wrap `GraphError` or `ParseError` in their own
manifest/decode errors because they, not `recipe-ogdl`, own model versions and
semantic consistency.

## Error propagation in the workspace

The layering is intentionally visible at every consumer boundary:

```text
recipe-ogdl::ParseError
    -> language::OgdlCodecError::Syntax
    -> program::ProgramError::Graph (when decoding the language subtree)
    -> training/checkpoint decoder InvalidSyntax or model manifest error
    -> ingest::GgufOgdlErrorKind::InvalidSyntax

recipe-ogdl::GraphError
    -> language::OgdlCodecError::Build
    -> program::ProgramError::Build
    -> training model manifest errors
```

Semantic errors do not get folded into `ParseError`: an input can be valid
OGDL and still have an unknown Recipe field, wrong version, invalid number,
duplicate field, out-of-range payload, or inconsistent model. Conversely,
`recipe-ogdl` never reports a domain-specific error for a field it cannot
interpret. This separation lets callers distinguish a malformed text stream
from a well-formed document that violates the caller's schema.

The wrappers preserve different amounts of parser detail. The language and
program errors retain `ParseError` as a nested source, while ingest and the
training checkpoint decoders currently put `ParseError::to_string()` into
their own path/manifest detail. The underlying parser still computes the exact
line, column, and byte offset before that formatting step.

## Invariants and non-goals

The following invariants are enforced by the current public surface:

* every node has nonempty text without tab, carriage return, or line feed;
* every child and parent ID stored in a graph refers to the same graph's arena;
* child order and root order are stable append order;
* parser-created IDs are inserted before they are used as later-line parents;
* a parsed line cannot skip an ancestor depth;
* canonical output is deterministic and parseable with LF line endings;
* graph equality compares ordered forest content, not cross-graph ID identity;
* all parser and graph failures are returned as typed `Result` values.

This crate intentionally does not provide:

* shared node references, anchors, links, DAGs, or cycles;
* comments, quoting, escapes, or a way to put a structural tab in text;
* schema dispatch, type checking, numeric range checking, or field uniqueness;
* streaming readers/writers, byte-level encoders, file persistence, or limits;
* mutation after append, deletion, reparenting, or graph merging.

Consumers that need any of those properties must define them at their own
boundary. In particular, a successful `Graph::parse` proves syntax and forest
shape only; it does not prove that a `.ogdl` model, calculation graph, program,
or structural GGUF document is valid.

## Focused validation

The crate currently has no external dependencies, crate-local test files, or
custom test target. The structural package check used while documenting this
boundary is:

```text
cargo check -p recipe-ogdl
```

It completes successfully in the active checkout. Runtime callers should
validate their own end-to-end artifact paths, because only those callers know
which root names, field order, limits, versions, and semantic invariants are
required after the common syntax parser succeeds.
