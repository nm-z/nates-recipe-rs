# Canonical serialization of `recipe-ogdl::Graph`

[`ogdl/src/serializer.rs`](../../src/serializer.rs#L1-L51) is the private text writer
for the `recipe-ogdl` ordered forest. It does not implement a general OGDL
writer, assign schema meaning, or perform file I/O. Its only job is to turn the
already validated arena owned by [`Graph`](../../src/graph.rs) into the one
deterministic spelling used by [`Graph::to_canonical_string`](../../src/graph.rs#L88-L90)
and [`Display`](../../src/graph.rs#L143-L147).

The serializer is deliberately narrower than the name OGDL might suggest. A
node has nonempty text with no tab, carriage return, or line-feed character;
tabs are syntax and line endings are structure. There is no quoting,
backslash escape, comment, anchor, link, shared-reference, or cycle syntax.
The graph is an ordered rooted forest, so the canonical result describes
ordered trees and not arbitrary graph identity.

## Public boundary and ownership

The `serializer` module and `serialize` function are `pub(crate)`. Callers use
the following public boundaries instead:

| API | Result and relationship to this module |
| --- | --- |
| `Graph::to_canonical_string(&self) -> String` | Calls `serializer::serialize` and returns the complete canonical text. It has no fallible result because the public graph builders maintain the serializer's input invariants. |
| `impl Display for Graph` | Writes exactly `to_canonical_string()` to the formatter. It does not add a final line ending. |
| `Graph::parse(&str) -> Result<Graph, ParseError>` | The inverse syntax boundary. It accepts the narrow tab/newline profile described by [`parser.rs`](../../src/parser.rs), then rebuilds an arena. |
| `impl FromStr for Graph` | Delegates to `Graph::parse`; it does not use a second decoder. |

The serializer receives only a shared `&Graph`. Node text, parent links,
child vectors, and root vectors are private in `Graph`, and `append_root` and
`append_child` validate all text before a node enters the arena. Higher-level
code such as the Recipe IR codec validates its own semantic graph first, then
uses this syntax-only boundary.

## Canonical output contract

For a nonempty graph, the output has these properties:

* the first root starts at byte zero, with no leading tabs or newline;
* each later root starts after one LF and has zero leading tabs;
* the first child of every node is appended to its parent's text with one tab;
* each later child starts after one LF followed by one tab per depth level;
* a tab between two node texts is always a parent-child edge, never text data;
* node text is copied byte-for-byte as UTF-8, including ordinary spaces and
  other characters allowed by the graph builder;
* line endings emitted by the serializer are LF (`\n`) only; and
* the result has no trailing LF.

The only bytes introduced by the writer are tab (`0x09`) child delimiters,
LF (`0x0a`) line separators, and the tab indentation that follows those line
separators. It does not add a byte-order mark, normalize Unicode, trim spaces,
or insert a final newline. Any NUL or other non-structural character already
present in node text remains part of the returned `String`.

An empty graph produces the empty string. There is no special empty-document
marker, and an empty line is not a representation of an empty graph when the
text is parsed.

The layout can be described by this notation. It is a grammar summary, not a
literal document, so `TAB^d` means `d` literal tab bytes and `NODE` means one
nonempty node text:

```text
document       := empty | root-line (LF root-line)*
root-line      := NODE (TAB NODE)*
child-line(d)  := TAB^d NODE (TAB NODE)*
```

The parser accepts a broader set of line placements than the canonical writer
uses. In particular, a first child may be put on its own indented line in an
input document. Re-serializing that graph moves the first child inline and
therefore normalizes the spelling.

## Implementation walk

`serialize` is an iterative depth-first pre-order traversal. It uses a local
`String` and an explicit `Vec<(NodeId, usize, Placement)>`, reserving
`graph.len()` stack entries up front. `Placement` is private and has exactly
three meanings:

| Placement | Prefix emitted before the node text |
| --- | --- |
| `First` | Nothing. This is used only for root index zero. |
| `Inline` | One tab. This is used only for child index zero. |
| `Line(indentation)` | LF, then `indentation` tabs. Later roots use `Line(0)`; later children use their own depth. |

The stack is populated in reverse order so its LIFO behavior emits source
order. The actual steps are:

1. Iterate `graph.roots()` in reverse. Push every root with depth `0`; root
   index zero receives `First`, and every other root receives `Line(0)`.
2. Pop one pending tuple. Emit its placement prefix, resolve the `NodeId` with
   `graph.node(id)`, and append `node.text()` unchanged.
3. Call `push_children` with `depth + 1`. It walks the child slice in reverse,
   pushes the first child as `Inline`, and pushes every later child as
   `Line(depth + 1)` while recording that child depth.
4. Continue until the pending stack is empty, then return the accumulated
   string.

The `depth` carried in each tuple is the logical tree depth, with roots at
zero. For an inline child, adding one tab increases the depth by one relative
to the parent's position on the line. For a later child, `Line(depth)` writes
that same logical depth as leading tabs before the node. This invariant keeps
indentation correct even when several first-child edges share one physical
line.

Because children are pushed in reverse and the first child is popped first,
the output is a pre-order traversal with roots and child vectors in their
stored order. A node's first-child chain stays on one physical line until a
branch or a later root requires a line break.

For example, the following canonical text represents one root, whose first
child has two children, and whose root has a second child:

```text
root	first	first-child
		second-child
	second
```

The first line contains `root`, `first`, and `first-child` because each is the
first child of the preceding node. `second-child` is the later child of
`first`, so it starts at depth two. `second` is the later child of `root`, so
it starts at depth one. Two roots are separated similarly, but root lines
always start at depth zero:

```text
root-a	child-a
root-b	child-b
```

No recursive Rust call is used. If `T` is the total UTF-8 byte length of node
text, `N` is the node count, and `D` is the sum of indentation tabs emitted for
line-start nodes, the output length is `T + O(N) + D`; traversal work is linear
in that output length. The returned `String` owns the output, while the
pending stack is released when the call returns.

For a nonempty graph, the byte accounting is exact if `R` is the root count,
`B` is the number of non-first child edges, `F` is the number of parents with a
first child, and `S` is the sum of logical depths for non-first children:

```text
LF bytes   = (R - 1) + B
TAB bytes  = F + S
text bytes = sum(node.text().len())
```

The returned length is the sum of those three rows. This is a description of
the writer's output, not a separate size limit; the API has no configured
maximum graph or text size.

## Text and escaping

The writer performs `output.push_str(node.text())`. It has no escaping layer.
That is safe only because the arena API rejects the structural characters
before insertion:

| Input text supplied to `append_root` or `append_child` | Result |
| --- | --- |
| Empty string | `GraphError::InvalidNodeText { kind: Empty, character: 1 }` |
| Contains a tab | `GraphError::InvalidNodeText { kind: Tab, character: n }` |
| Contains `\r` | `GraphError::InvalidNodeText { kind: CarriageReturn, character: n }` |
| Contains `\n` | `GraphError::InvalidNodeText { kind: LineFeed, character: n }` |
| Anything else, including spaces, quotes, backslashes, `#`, and Unicode | Copied literally into the canonical result |

The reported `character` is one-based in Unicode scalar values. Spaces have no
structural meaning: leading spaces after indentation, repeated spaces, and
trailing spaces remain part of `Node::text()`. For example, the following is
one root with a child whose text begins with a space, not an escape sequence:

```text
root	 child with spaces
```

To represent a literal tab, carriage return, or line feed in application data,
the application must encode that value at a higher semantic layer. This
module cannot make such a value round-trip by adding quotes or backslashes.

## Ordering and node identity

Serialization observes the ordered relationships, not arena insertion order:

* roots are emitted in `Graph::roots()` order;
* children are emitted in each `Node::children()` order;
* each subtree is emitted completely before the next sibling; and
* `NodeId` values, parent IDs, and the order returned by `Graph::nodes()` are
  not written to the text.

This distinction matters because the public builder permits a child to be
appended after another root already exists. For example, constructing root A,
then root B, then appending a child to A gives arena IDs A=`0`, B=`1`,
child=`2`, but canonical text is:

```text
A	child-of-A
B
```

Parsing that text necessarily allocates A, then its child, then B, so the new
arena IDs are A=`0`, child=`1`, B=`2`. The forest shape and text are unchanged,
but IDs are local to an arena and are intentionally not part of the format.
`Graph` equality follows the same structural rule: it compares root count,
node count, node text, and ordered child counts while traversing matching
roots; it does not require equal numeric `NodeId` values.

## Parser round trips and canonicalization

For every graph `graph` that can be built through the public API, the parser
round trip preserves the ordered forest:

```text
Graph::parse(&graph.to_canonical_string())? == graph
```

The equality here is `Graph`'s structural `PartialEq`, not identity equality.
The serializer's layout always satisfies the parser's indentation rule, and
every emitted node text is nonempty and contains no delimiter character, so
this parse cannot fail for a valid graph.

The parser's `ancestors` stack explains why the layout is invertible. A
tab-delimited run on one line is inserted as a parent-child chain. A later
child line starts at exactly that child's depth, so the parser truncates the
previous ancestor chain to the parent depth and attaches the new node beside
the earlier child. A root line starts at depth zero and therefore truncates
the active chain before creating a new root. Induction over the serializer's
pre-order traversal then recovers every root, child vector, and text value in
order.

For every input accepted by `Graph::parse`, serializing the resulting graph
produces a canonical spelling. The accepted input need not already be
canonical. This input has the first child on its own line:

```text
root
	first-child
	second-child
```

It parses as one root with two children, then canonicalizes to:

```text
root	first-child
	second-child
```

LF and CRLF are both accepted by the parser. A final line ending is also
accepted after a nonempty final line, but the serializer removes it. Thus an
input equivalent to `root\r\n\tchild\r\n` canonicalizes to the two-node string
`root\tchild`, containing one tab and no line ending. A blank line is not
ignored: it is an empty-node parse error.

Canonicalization is idempotent:

```text
canonical = Graph::parse(input)?.to_canonical_string()
Graph::parse(&canonical)?.to_canonical_string() == canonical
```

The empty case follows the same rule. `Graph::new().to_canonical_string()` is
`""`, and `Graph::parse("")` returns `Graph::new()`.

## Production callers

This crate owns syntax only. The main semantic codecs construct a validated
`Graph`, then reach this serializer through the same public method:

| Caller | Sequence |
| --- | --- |
| `language::CalculationGraph::to_ogdl` | Validates the typed calculation graph, encodes it into a `recipe-ogdl::Graph`, and calls `to_canonical_string` ([`language/src/ogdl.rs`](../../../language/src/ogdl.rs#L93-L103)). |
| `program::StaticCalculationProgram::to_ogdl` | Validates the static program, builds its `RecipeProgram` and `RecipeIR` roots, and serializes the resulting forest ([`program/src/lib.rs`](../../../program/src/lib.rs#L308-L392)). |
| Training program digest | Calls `StaticCalculationProgram::to_ogdl`, then hashes the returned canonical UTF-8 bytes; the digest boundary does not invent a second text format ([`training/src/checkpoint.rs`](../../../training/src/checkpoint.rs#L7739-L7746)). |
| KNN and Bayesian model encoders | Validate their semantic artifact, build a graph, call `to_canonical_string`, and persist those bytes as a model image ([`training/src/knn_checkpoint.rs`](../../../training/src/knn_checkpoint.rs#L115-L121), [`training/src/bayes_checkpoint.rs`](../../../training/src/bayes_checkpoint.rs#L127-L133)). |

Their decoders call `Graph::parse` first and then apply a schema-specific
decoder. A successful syntax round trip therefore proves only the ordered
forest and node-text contract; each semantic caller remains responsible for
root names, field order, scalar spelling, limits, and domain validation.

## Invariants and failure behavior

The serializer itself returns `String`, not `Result<String, _>`. Its input
invariants are supplied by the private `Graph` representation and these
public construction paths:

* `append_root` validates text before adding a root;
* `append_child` validates the parent ID, then validates text;
* the parser splits only nonempty tab-delimited segments and inserts them with
  the crate-private `push_node`; and
* `push_node` appends the new ID to exactly one parent's child vector or to
  the root vector.

While resolving each pending item, `serialize` calls
`graph.node(id).expect("graph-owned node identity must resolve")`. A panic at
that point indicates an internal graph invariant violation, not malformed
user text. With the fields private and the safe APIs above, callers cannot
construct such a graph through this crate. Allocation failure is likewise a
process-level failure, not a recoverable serializer error.

Malformed text fails before serialization, at `Graph::parse`, with a
`ParseError` carrying a one-based line and column plus a zero-based UTF-8 byte
offset. The parser's relevant failure kinds are:

| Parse error | Trigger | Example |
| --- | --- | --- |
| `EmptyNode` | Blank line, a line made only of tabs, adjacent tabs, or a trailing tab that leaves an empty segment | `root\t` |
| `IndentationJump { found, maximum }` | A line starts deeper than the active ancestor stack permits | `root\n\t\tchild` |
| `BareCarriageReturn` | `\r` is not immediately followed by `\n` | `root\rchild` |

The parser rejects these forms rather than asking the serializer to escape or
repair them. Conversely, `GraphError::UnknownParent` and the
`InvalidNodeText` variants above are returned while constructing an arena and
therefore prevent invalid input from reaching `serialize` at all.

## Use the canonical boundary

Code that owns a `Graph` should call `to_canonical_string` or use `Display`
instead of reproducing tab and newline placement. Code that receives text
should call `Graph::parse` or `str::parse::<Graph>()`, then use the graph's
public node and root queries. A caller that needs a semantic schema, numeric
canonical forms, or a reversible representation of arbitrary graph identity
must define those rules above this syntax crate; this serializer intentionally
does not infer them.
