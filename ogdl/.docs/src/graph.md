# `ogdl/src/graph.rs`

## Role and boundary

`graph.rs` is the public data and mutation facade for the `recipe-ogdl` crate.
It exposes [`NodeId`], [`Node`], and [`Graph`], delegates text parsing to
[`parser.rs`](../../src/parser.rs), and delegates canonical text generation to
[`serializer.rs`](../../src/serializer.rs). The crate is a dependency-free,
OGDL-derived syntax profile. Its graph is an ordered rooted forest, not a
general graph:

* one node has one parent or is a root;
* each parent has an ordered list of children;
* node identities are arena indexes local to one graph; and
* there are no public operations for links, shared references, anchors,
  cycles, deletion, or reparenting. The syntax has no comments, quoting,
  escaping, or schema directives at this layer.

The graph layer knows only node text and tree relationships. It does not know
whether a node is a field, a scalar, a schema record, a tensor, or a model.
Those meanings belong to consumers such as the language codec and checkpoint
decoders. A successful [`Graph::parse`] therefore proves syntax only. It does
not prove a document root, schema, field set, scalar shape, version, or domain
semantics.

The public facade re-exports the crate as `recipe::engine::ogdl` in
[`src/facade.rs`](../../../src/facade.rs). The implementation modules are private;
callers reach parsing and serialization through `Graph` methods and trait
implementations.

## Representation

### `NodeId`

`NodeId` is a `usize` arena index wrapped in a private tuple field. It derives
`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash`.
The only constructor-like operation is the ID returned by a graph operation;
callers cannot manufacture an arbitrary ID because the tuple field is private.
`NodeId::index()` exposes the zero-based numeric index for diagnostics and
other external identity formats.

An ID is stable for the lifetime of its graph. Appending nodes never changes an
existing index, although Rust borrow rules still limit how long a reference to
a `Node` may be held while the graph is mutably borrowed. IDs have no
cross-graph provenance. An ID copied from one graph may be numerically valid in
another graph, so callers must keep the ID paired with the graph that produced
it. `Graph::node` checks only the numeric arena range.

### `Node`

Each node stores three private fields:

| Field | Meaning and observable API |
| --- | --- |
| `text: String` | Immutable node text. [`Node::text`] returns `&str`; spaces and every character other than tab, carriage return, and line feed are literal. |
| `parent: Option<NodeId>` | `None` for a root, otherwise the parent ID. [`Node::parent`] returns the copied option. |
| `children: Vec<NodeId>` | Child IDs in append and source order. [`Node::children`] returns the borrowed slice. |

`Node` derives `Clone`, `Debug`, `PartialEq`, and `Eq`. The fields are private,
so callers cannot mutate text or edges and cannot construct a node detached
from a graph. A root is identified by both membership in `Graph::roots()` and a
`None` parent. A non-root should be reachable from exactly one parent's child
slice.

### `Graph`

`Graph` owns the arena and the root list:

```text
Graph
├── nodes: Vec<Node>       // private, arena order is NodeId order
└── roots: Vec<NodeId>     // private, root insertion/source order
```

It derives `Clone`, `Debug`, and `Default`; `Graph::new()` is the `const`
empty constructor. The private fields are the integrity boundary. Every public
graph operation preserves the following invariants:

1. If `nodes.len() == n`, the valid IDs are exactly `NodeId(0)` through
   `NodeId(n - 1)`.
2. Every root ID resolves in the arena, appears once in `roots`, and has
   `parent() == None`.
3. Every non-root ID resolves, has a parent that resolves, and occurs in that
   parent's child list.
4. Child slices preserve insertion order. No public operation removes,
   reorders, duplicates, or reparents an edge.
5. Every node is reachable from one root by following child edges. The
   append-only construction therefore produces a forest with no cycle.
6. Every node text is nonempty and contains no tab, carriage return, or line
   feed. Parser-created nodes satisfy the same rule through the parser's
   segment checks.

`Graph` has no general mutation method beyond appending a root or appending a
child. There is no public way to invalidate these invariants. Internal
`push_node` is shared by the parser and the append methods and assumes that a
supplied parent is already valid.

## Query operations

All query methods borrow the graph and expose the existing arena; none clones,
normalizes, or computes semantic values.

| Operation | Result and ordering |
| --- | --- |
| `Graph::len()` | Number of arena nodes. It is also the number of items yielded by `nodes()`. |
| `Graph::is_empty()` | True exactly when the arena has no nodes. An empty graph has no roots and serializes to the empty string. |
| `Graph::roots()` | `&[NodeId]` in root insertion order for programmatic graphs, or source order for parsed documents. A forest may have zero, one, or many roots. |
| `Graph::node(id)` | `Some(&Node)` when `id.index() < len()`, otherwise `None`. The method does not verify that the ID came from this graph beyond the numeric range. |
| `Graph::nodes()` | An `ExactSizeIterator` of `(NodeId, &Node)` in arena order, equivalent to IDs `0..len()`. The iterator is the direct way to inspect all nodes and their assigned IDs. |
| `NodeId::index()` | The zero-based `usize` representation. It is not a portable identity outside its graph. |
| `Node::text()` | Borrowed, exact text with no whitespace trimming. |
| `Node::parent()` | `None` for a root, otherwise the parent ID. |
| `Node::children()` | Borrowed child IDs in source or append order. The slice is read-only. |

Consumers commonly use `roots()` to select a document envelope, then use
`node()` and `children()` while walking records. A missing ID returns `None`
instead of panicking. The syntax serializer uses an internal `expect` only after
obtaining IDs from the same graph, where the private-field invariants make a
missing node impossible. Domain consumers generally convert `None` into their
own invalid-document error.

## Appending nodes

### `Graph::append_root`

`append_root(text)` converts its argument to `String`, validates the complete
text, appends a node with `parent == None`, records its ID in `roots`, and
returns the new ID. The ID is always the previous `len()`, so roots and
non-roots share one monotonically increasing arena namespace. A text validation
failure leaves the graph unchanged.

### `Graph::append_child`

`append_child(parent, text)` first checks that `parent` resolves in this graph.
An unknown parent returns `GraphError::UnknownParent(parent)` without appending
anything. Once the parent is known, the method converts and validates the text.
On success it appends a node whose `parent` is the supplied ID and pushes the
new ID onto that parent's child slice. A text validation failure also leaves
the graph unchanged. The new ID is returned after both the arena node and the
parent edge have been written.

Both methods use the same text validator:

| Input text | Error |
| --- | --- |
| `""` | `GraphError::InvalidNodeText { kind: Empty, character: 1 }` |
| Any tab (`\t`) | `InvalidNodeText { kind: Tab, character: position }` |
| Any carriage return (`\r`) | `InvalidNodeText { kind: CarriageReturn, character: position }` |
| Any line feed (`\n`) | `InvalidNodeText { kind: LineFeed, character: position }` |

The reported `character` is one-based and counts Rust `char` values, not UTF-8
bytes. Other Unicode characters, ordinary spaces, and other control
characters are not rejected by this validator. Tabs and line endings are
reserved for the textual structure, which is why callers must represent them
through graph edges and lines rather than embedding them in node text.

`GraphError` is non-exhaustive and implements `std::error::Error`. Its display
form is `invalid node text at character N: KIND` for text failures, where
`KIND` is the debug spelling of `NodeTextErrorKind`, and `unknown parent node N`
for an invalid parent ID. It carries no source line because append operations
receive already separated node text rather than a document buffer.

The parser calls the crate-private `push_node` directly after it has split a
line into nonempty, tab-free segments. It does not route parser input through
`GraphError`; parser failures are reported as `ParseError` with source
locations instead.

## Text parsing

`Graph::parse(input)` and `input.parse::<Graph>()` both call the same private
parser and return `Result<Graph, ParseError>`. Parsing an empty string returns
`Graph::new()`. For nonempty input, the parser scans bytes to recognize LF and
CRLF line endings, then parses each line while retaining an ancestor stack.

### Line grammar

The narrow profile can be described as this grammar plus the indentation rule
below:

```text
document       := "" | line (line_ending line)* line_ending?
line           := leading_tabs text ("\t" text)*
leading_tabs   := "\t"*
line_ending    := "\n" | "\r\n"
text           := one or more Unicode scalar values other than "\t", "\r", "\n"
```

Operationally, `leading_tabs` is the maximal initial run of tab characters;
the remaining tabs are delimiters in the final production. This removes the
otherwise harmless ambiguity between indentation and an inline delimiter.

This grammar is stateful because a line's leading-tab count must not skip a
currently available ancestor. It intentionally gives spaces no structural
meaning. For example, `"  name"` is one node whose text begins with two spaces,
while `"name \tvalue"` creates a node named `"name "` and a child named
`"value"`.

The parser treats every tab after the leading indentation as a delimiter, not
as additional indentation. Adjacent delimiters or a trailing delimiter create
an empty segment and fail. A line made entirely of leading tabs also fails.

### How a line becomes edges

The parser maintains `ancestors`, one ID for each depth in the most recently
parsed chain. For a line with `d` leading tabs:

1. `d > ancestors.len()` is an indentation jump and fails.
2. With `d == 0`, the first segment becomes a new root. With `d > 0`, the
   first segment's parent is `ancestors[d - 1]`.
3. The parser truncates the ancestor stack to `d`.
4. It creates one node for each tab-delimited segment. Each later segment is a
   child of the segment immediately before it, and each new ID is pushed onto
   `ancestors`.

Thus leading tabs select the parent of the first segment, while later tabs on
the same line extend a child chain. Consider:

```text
root	first	value
	second	leaf
other	child
```

The first line creates `root -> first -> value`. The second line has one
leading tab, so `second` is another child of `root`, and `leaf` is its child.
The final line starts a second root, `other -> child`. The resulting arena is
equivalent to:

```text
root
	first
		value
	second
		leaf
other
	child
```

A line with two leading tabs after the first line would select `ancestors[1]`
as the parent of its first segment. To continue below the last inline segment,
the line needs one more leading tab for that segment's depth. This distinction
is what makes inline tabs and line indentation composable rather than
interchangeable.

### Line endings and trailing lines

LF and CRLF are accepted. CRLF's carriage return is removed from the line
slice before segmentation, and canonical output always uses LF. A bare `\r`
anywhere that is not immediately followed by `\n` is rejected. A final LF is
allowed because it terminates the preceding nonempty line; an additional LF
would introduce an empty line and fail. The empty input is the only input that
produces an empty graph.

### Parse failures and locations

The parser returns one of the following public `ParseErrorKind` values:

| Kind | Trigger | Location used by the parser |
| --- | --- | --- |
| `EmptyNode` | Empty input line, all-tab line, adjacent inline tabs, or a trailing inline tab | The first empty segment. For an all-tab line this is column `indentation + 1` at the line end; for an inline segment it is the delimiter or line end. The offset is the corresponding byte position. |
| `IndentationJump { found, maximum }` | Leading-tab depth is greater than the current ancestor-stack length | Column 1 and the line-start byte offset. `found` is the leading-tab count; `maximum` is the available ancestor count. |
| `BareCarriageReturn` | A carriage return is not followed by LF | The carriage-return byte's line, character column, and byte offset. |

`ParseError::location()` returns a `SourceLocation` whose line and column are
one-based. The `offset` is a zero-based UTF-8 byte offset into the original
input. Columns count Unicode `char` values in the line prefix, while offsets
count bytes. `ParseError::kind()` preserves the structured failure, and its
`Display` implementation reports the kind-specific message plus line and
column. Parsing stops at the first failure and returns no partial graph.

Representative first-failure locations are:

| Input | Kind and location |
| --- | --- |
| `"\n"` | `EmptyNode`, line 1, column 1, offset 0. |
| `"\troot"` | `IndentationJump { found: 1, maximum: 0 }`, line 1, column 1, offset 0. |
| `"root\t"` | `EmptyNode`, line 1, column 6, offset 5, at the line end after the delimiter. |
| `"root\r"` | `BareCarriageReturn`, line 1, column 5, offset 4. |

The trailing-tab row is intentionally spelled out because the error points
after the tab, not at it: `root` has four characters, the tab is column 5, and
the empty segment begins at column 6, byte offset 5. The parser's location
fields are source coordinates, while `ParseError`'s display omits the byte
offset and prints only line and column.

Syntax acceptance is deliberately weaker than document acceptance. For
example, an empty input and a multi-root forest are valid `Graph` values, but a
Recipe IR, static program, or semantic checkpoint consumer may reject them
because its own contract requires one or more named roots.

The graph parser has no built-in source-byte, node-count, depth, or text-size
limit. Callers that accept untrusted or persisted bytes must impose those
limits before or around parsing, as the checkpoint decoders do. A syntax error
is not a resource-limit result, and a resource limit is not a `ParseError`.
Both parser and serializer walk the structure iteratively with vectors rather
than recursive Rust calls; depth is represented by the ancestor or pending
stack and remains bounded only by the input or graph size.

## Canonical serialization

`Graph::to_canonical_string()` calls the private serializer and returns a
`String` without a graph error result. `Display for Graph` writes exactly the
same bytes and can only propagate the formatter's own `fmt::Error`. Node text
has already been validated, and the serializer's internal `expect` is guarded
by graph-owned IDs and private invariants.

The canonical traversal is deterministic, depth-first, and pre-order:

* the first root is written without a prefix;
* each later root starts with LF at indentation depth zero;
* the first child of a node is written inline after a tab;
* every later child starts a new line with tabs equal to that child's depth;
  and
* each node's children are traversed in their stored order.

The serializer uses an explicit pending stack but emits the same order as a
recursive pre-order walk. It reserves capacity from `Graph::len()` and does
not sort, deduplicate, or inspect semantic text. An empty graph serializes to
`""`. There is no quoting or escaping because node text cannot contain the
three structural characters. Spaces and all other permitted characters are
written byte-for-byte.

For example, the forest represented by the previous parser example has the
canonical form:

```text
root	first	value
	second	leaf
other	child
```

`Graph::parse(graph.to_canonical_string())` reconstructs the same ordered
forest for every graph reachable through the public constructors. The custom
`Graph` equality implementation compares that forest structure and text, so
the round trip can be checked with `==`. Noncanonical but valid source text is
normalized to this placement on serialization. The serializer always emits LF
even when the input was CRLF.

## Equality, cloning, and formatting

`Graph` implements `PartialEq` and `Eq` with a structural comparison rather
than comparing arena indexes directly. It first requires equal root counts and
equal total node counts, then walks corresponding roots and child lists in
order. It compares each node's text and child count, and pairs children by
position. Consequently:

* root order, child order, text, and tree shape are significant;
* equivalent forests may use different internal arena numbering and still be
  equal if their reachable structure and text match;
* node parent fields and child IDs are not compared as raw values by the graph
  equality algorithm; and
* the private invariants make all corresponding IDs resolve, so malformed
  internal graphs would violate the assumptions behind this comparison.

`Node` equality is different: its derived implementation compares its text,
parent ID, and child-ID vector directly. A `Node` borrowed from one graph is
therefore not a portable structural value by itself. `Graph::clone()` preserves
the entire arena and all IDs, while `NodeId` is a cheap copied handle.

`FromStr` uses `Graph::parse`, so parse errors are the `Err` type for string
parsing. `Display` uses canonical serialization and emits no extra trailing
newline.

## Ownership and cost model

The arena owns every node text and every edge. `NodeId`, `Node::parent()`, and
the entries returned by `Node::children()` are copied handles into that arena;
they do not allocate or retain a second ownership graph. The borrowed query
methods keep the graph as the single source of truth.

For a graph with `n` nodes and total text length `b` bytes, the relevant costs
are:

| Operation | Time | Additional storage |
| --- | --- | --- |
| `new`, `len`, `is_empty`, `roots`, `node` | O(1) | None beyond returned borrows. |
| `nodes()` creation | O(1) | Iterator state only; iteration visits `n` arena entries. |
| `append_root` / `append_child` | O(text length) for validation, then amortized O(1) arena and edge append | Vec growth owned by the graph. |
| `parse` | O(input bytes plus Unicode character scans) | Arena nodes, child slices, and the current ancestor stack. |
| `to_canonical_string` | O(`n + b`) | Output `String` plus a pending traversal stack bounded by the node count. |
| `Graph` equality | O(`n + b`) for valid graphs | A pending pair stack bounded by the number of visited nodes. |

Text validation is performed before a public append mutates either vector, so
failed text checks do not leave a partially appended node. Parsing may have
already built earlier lines internally when a later line fails, but that
temporary graph is returned only on complete success and is dropped on error.
No partial graph is exposed to the caller.

## Consumers and failure boundaries

The graph API is intentionally small enough that higher-level codecs build
their own domain checks. Current in-tree consumers of `recipe_ogdl::Graph`
follow this pattern:

| Consumer | Graph operations | Domain checks after syntax |
| --- | --- | --- |
| `ogdl/src/parser.rs` | Creates `Graph`, maintains ancestor IDs, and calls private `push_node`. | Enforces line grammar, indentation continuity, and source locations before returning a graph. |
| `ogdl/src/serializer.rs` | Reads `roots()`, `len()`, `node()`, and `Node::children()` to emit canonical text. | Relies on graph-owned IDs; an unexpected missing node is an internal invariant failure. |
| `language/src/ogdl.rs` | Builds Recipe IR with `new`, `append_root`, and `append_child`; decodes with `parse`, `roots`, `node`, `text`, and `children`. | Requires exactly one `RecipeIR` root, exact fields, exact scalar/value cardinality, exact enum/version spellings, and `CalculationGraph::validate()`. `GraphError` becomes `OgdlCodecError::Build`; `ParseError` becomes `OgdlCodecError::Syntax`. |
| `program/src/lib.rs` | Builds a `RecipeProgram` root and copies the language graph subtree with append methods; reads two roots and walks fields on decode. | Requires exactly `RecipeProgram` and `RecipeIR` roots, checks program fields and versions, then delegates the second root to the language codec. Unknown IDs become `ProgramError::Contract { kind: InvalidDocument, ... }`; graph and parse errors are wrapped in `ProgramError`. |
| `ingest/src/gguf_ogdl.rs` | Parses UTF-8 text, reads roots and child slices, and accesses nodes through checked helpers. | Applies GGUF root, metadata, tensor, scalar, size, and value validation. A missing ID is an ingest structure error; parser failure is mapped to the ingest invalid-syntax boundary. |
| `training/src/checkpoint.rs` | Parses bounded UTF-8 checkpoint bytes as `OgdlGraph`, checks `len()`, and walks node/child IDs. | Enforces source and node limits, root/format/version contracts, fields, payload accounting, and model semantics. Syntax and missing-node failures become checkpoint decode errors. |
| `training/src/knn_checkpoint.rs` | Encodes with `new`, `append_root`, `append_child`, and `to_canonical_string`; decodes with `parse`, `roots`, `node`, and `children`. | Enforces byte/node limits, exactly one named root, required and unique fields, scalar leaf shape, and KNN value constraints. |
| `training/src/bayes_checkpoint.rs` | Encodes the Bayesian model with append methods and canonical output; decodes through `Graph::parse` and node/child accessors. | Enforces byte/node limits, one named root, version-specific field sets, scalar leaf shape, and Bayesian semantic constraints. |

All of these consumers keep the graph immutable while decoding. They retain
`NodeId` handles only as references into the parsed graph, and they convert
`Graph::node(id) == None` into a domain error instead of attempting to repair
the graph. The syntax layer does not supply defaults, reorder fields, or infer
schema meaning for them. Other workspace types also use the name `NodeId` for
hardware or topology identities; those types are distinct and are not
`recipe_ogdl::NodeId` consumers.

None of these graph operations perform file I/O or byte decoding. Callers read
files or receive bytes, apply their own UTF-8 and size policy, then pass a
`&str` to `Graph::parse`; the graph crate returns only its syntax result.

## Practical contract

Use the graph layer when a document needs ordered node text and tree edges:

1. Start with `Graph::new()` for programmatic construction, or
   `Graph::parse()` for text.
2. Retain returned `NodeId` values with their owning graph.
3. Add roots and children only through `append_root` and `append_child` so text
   and edge invariants are checked at the mutation boundary.
4. Traverse through `roots`, `node`, `Node::children`, and `Node::text`; do not
   infer structure from spaces.
5. Treat `to_canonical_string()` or `Display` as the stable textual form. It
   emits LF and normalizes sibling placement.
6. Do not expect the graph layer to reject duplicate labels, unknown fields,
   or multiple roots; impose those document rules in the consuming codec.
7. Perform schema, version, field, scalar, size, and semantic checks in the
   consuming codec. A parsed graph is a syntax tree, not a validated model.

The resulting boundary is deliberately narrow: `recipe-ogdl` owns text to
ordered-forest conversion and the integrity of its arena, while each domain
crate owns the meaning and acceptance policy of the nodes it consumes.
