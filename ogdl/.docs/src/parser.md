# Parser

`ogdl/src/parser.rs` implements the complete textual parser for the
`recipe-ogdl` crate. The implementation is deliberately small and strict: it
turns a tab-indented document into an ordered rooted forest, and it reports the
first syntax error with a source location. The parser has no mode switches,
schema, comment syntax, or configurable limits.

The implementation is split into three functions in
[`ogdl/src/parser.rs`](../../src/parser.rs):

- `parse` (lines 3-55, the crate-private entry point) scans the complete UTF-8
  string, recognizes LF and CRLF line endings, and owns the graph and ancestor
  stack.
- `parse_line` (lines 57-111) parses one line after its line ending has been
  removed. It determines the parent depth, splits node text at structural tabs,
  and appends the resulting chain to the graph.
- `empty_node_error` (lines 113-119) builds an `EmptyNode` error at a byte
  position in one line.

The public wrapper is [`Graph::parse`](../../src/graph.rs), and
`Graph` also implements `FromStr` with `ParseError` as its error type. The
crate-private `parser::parse` function is not directly callable by downstream
crates.

## Accepted document

The parser treats a document as an ordered sequence of lines. The following
grammar describes the lexical shape. The indentation rule below the grammar is
semantic and depends on the previously parsed line.

```text
document      = "" | line { line_ending line } [ line_ending ] ;
line_ending   = LF | CR LF ;
line          = indentation node { TAB node } ;
indentation   = { TAB } ;
node          = character { character } ;
character     = any Unicode scalar value except TAB, CR, and LF ;
```

This grammar has these concrete consequences:

- The empty string is the one valid empty document. It produces an empty
  `Graph`.
- A nonempty document must contain at least one nonempty line. An LF or CRLF
  in the middle of a document therefore separates two lines, and the line on
  each side must contain a node. A final LF or CRLF terminates the last line;
  it does not create an additional empty line.
- `\n` is LF. `\r\n` is CRLF and is accepted as one line ending. A `\r` that is
  not immediately followed by `\n` is never node text and is rejected as a
  `BareCarriageReturn`.
- Leading tabs are indentation. Every tab after the leading run is a node
  delimiter, so a node cannot contain a tab. Tabs are not escaped, quoted, or
  otherwise representable inside node text.
- Spaces have no structural meaning. Leading spaces are part of the first
  node's text, and spaces elsewhere are preserved exactly. Punctuation,
  Unicode text, and other characters that are not TAB, CR, or LF are also
  literal node text.
- There is no quoting, escaping, comment, anchor, link, reference, or cycle
  syntax. Such characters are ordinary text unless they are one of the three
  structural line characters.

The grammar alone does not permit an indentation jump. If a line begins with
`d` tabs, then `d` must be no greater than the number of available ancestors
from the preceding line. This is the rule that gives the flat text a rooted
tree interpretation.

### Indentation and same-line chains

The leading tab count is a parent depth, not the depth of the first node that
will be created. On a line with indentation `d` and segments
`s0`, `s1`, ..., `sk`, the parser performs these operations:

1. With `d == 0`, `s0` is a new root. With `d > 0`, `s0` is a child of the
   ancestor at stack index `d - 1`.
2. `s1` is a child of `s0`, `s2` is a child of `s1`, and so on. Thus every
   internal tab creates an edge; it does not create a sibling.
3. The ancestor stack is truncated to the first `d` entries and then extended
   with all nodes created on this line. The last segment is consequently the
   deepest available ancestor for the next line.

For example, the tabs in this document are structural tabs, shown literally in
the code block:

```text
system	master	device
	workers	worker 0
		worker 1
```

The resulting edges are:

```text
system
├── master
│   └── device
└── workers
    ├── worker 0
    └── worker 1
```

The first line creates the chain `system -> master -> device`. The second line
has indentation one, so `workers` is attached to `system`; its second segment,
`worker 0`, is attached to `workers`. The third line has indentation two, so
`worker 1` is attached to the `workers` node from the previous line.

## `parse` scan and line framing

`parse` keeps these local values while it scans the input:

| State | Meaning |
| --- | --- |
| `bytes` | The original UTF-8 string viewed as bytes. This makes LF and CRLF detection exact and gives locations their original byte offsets. |
| `graph` | A new `Graph` that receives nodes in input order. |
| `ancestors` | A `Vec<NodeId>` containing the current path, including the deepest node created by the most recently parsed line. It starts empty. |
| `line` | One-based current line number. It starts at `1` and advances after every LF. |
| `line_start` | Zero-based byte offset of the current line in the original input. It starts at `0` and becomes one byte after each LF. |
| `cursor` | Zero-based byte index being scanned. It advances one byte per loop iteration. |

The scan is byte-based, but it never slices in the middle of a UTF-8 code
point. LF and CR are one-byte ASCII values, and `line_start` is always set
after an LF. The scanner's behavior is:

1. If the input is exactly empty, return `Graph::new()` immediately. This is
   the only path that returns an empty graph.
2. On a CR whose next byte is not LF, return `BareCarriageReturn` at the CR
   immediately. The current line has not been passed to `parse_line` yet.
3. On LF, set `line_end` to the byte before a preceding CR for CRLF, otherwise
   to the LF byte itself. Pass `input[line_start..line_end]` to `parse_line`.
   Then increment `line`, set `line_start` to the byte after LF, and continue.
4. After the byte loop, call `parse_line` for a final unterminated line only
   when `line_start < input.len()`. If the input ended immediately after LF,
   this condition is false, which is why a trailing line ending is accepted
   without producing a blank line.
5. Return the graph after every line succeeds.

The scanner therefore normalizes no data in the graph itself. A CR in CRLF is
only removed from the line slice used for parsing; node text never contains
that CR. The canonical serializer later emits LF line endings.

### Line-ending examples

These inputs have the following outcomes:

| Input | Result |
| --- | --- |
| `" "` | One root whose text is one literal space. Spaces are not blank-line syntax. |
| `""` | Empty graph. |
| `"a"` | One root node, with no final line ending. |
| `"a\n"` | The same one-node graph. The final LF terminates `a`. |
| `"a\r\n"` | The same one-node graph. CRLF is accepted and the CR is excluded from node text. |
| `"\n"` | `EmptyNode` on line 1. A nonempty input line is required. |
| `"a\n\n"` | `EmptyNode` on line 2. The second LF separates an empty line; only the final LF may be a terminator. |
| `"a\rb"` | `BareCarriageReturn` at the CR. |
| `"a\r"` | `BareCarriageReturn` at the final CR. |
| `"a\r\nb"` | Two nodes in separate root lines. |

Because the scanner checks bare CR bytes before invoking `parse_line`, a line
that contains both a bare CR and another line-level problem reports the bare CR
first. A syntax error on an earlier line is reported when its LF is reached,
before any later bytes are scanned.

## `parse_line` algorithm

`parse_line(source, line, line_offset, graph, ancestors)` receives one line
without LF or the CR of CRLF. It does not allocate node text until the whole
line has passed its structural checks.

### 1. Count indentation

The leading run is counted with `source.bytes().take_while(...)`, so only ASCII
TAB bytes count as indentation. Let this count be `indentation`.

- If `indentation == source.len()`, the line contains no node text. This covers
  an empty line and a line made entirely of tabs. Return `EmptyNode` at the byte
  immediately after the leading tabs, with column `indentation + 1`.
- Otherwise, if `indentation > ancestors.len()`, return
  `IndentationJump { found: indentation, maximum: ancestors.len() }` at the
  start of the line (column 1). This check occurs before tokenization.

The all-tabs check has precedence over the indentation-jump check. For example,
the first line `"\t\t"` is `EmptyNode`, not an indentation jump, because the
line has no node text at all. The first line `"\tchild"` is an indentation jump
because it has content and there are no available ancestors.

### 2. Split segments at later tabs

The parser starts `segment_start` immediately after the leading indentation and
walks the remainder with `char_indices()`. Every TAB encountered is a
delimiter:

1. If the delimiter is exactly `segment_start`, two delimiters are adjacent,
   so the segment between them is empty. Return `EmptyNode` at that delimiter.
2. Otherwise, borrow the nonempty slice before the delimiter as one segment and
   set `segment_start` to the byte after the delimiter.
3. After the walk, if `segment_start == source.len()`, the line ended in a
   delimiter. Return `EmptyNode` at that end position. Otherwise, borrow the
   final nonempty segment.

The segments are borrowed slices while this phase runs. A line such as
`"a\t\tb"` fails at the second tab, and `"a\t"` fails immediately after the
trailing tab. A line beginning with tabs has already had all leading tabs
removed from this phase, so the first character examined here cannot be a TAB
unless it is the empty/consecutive-delimiter case.

No segment can contain CR or LF: LF ends the line before this function runs,
CRLF removes CR before the call, and a bare CR has already caused an immediate
scanner error. No segment can contain TAB because every TAB is consumed as a
delimiter. Segments can contain spaces, Unicode, and other nonstructural
characters exactly as supplied.

### 3. Select the parent and update the ancestor path

After tokenization succeeds, the parent is selected before truncating the
ancestor vector:

```text
parent = None                         when indentation == 0
parent = ancestors[indentation - 1]   when indentation > 0
ancestors.truncate(indentation)
```

The earlier bound check guarantees that the indexed ancestor exists. Each
segment is then appended from left to right:

```text
node = graph.push_node(parent, segment.to_owned())
ancestors.push(node)
parent = Some(node)
```

`Graph::push_node` assigns the next sequential `NodeId`, adds a root when the
parent is `None`, or appends the new ID to the parent's ordered child list.
Because all segments were checked to be nonempty and delimiter-free, this
private append path preserves the same node-text invariants enforced by the
public `Graph::append_root` and `Graph::append_child` methods.

At the end of a successful line, `ancestors` has this invariant:

```text
ancestors = (the first `indentation` IDs of the previous path)
            followed by (every node created by this line, left to right)
```

Its length is therefore `indentation + number_of_segments`. On the next line,
that length is the exact maximum legal indentation. A line with lower
indentation truncates the path and starts under an earlier ancestor; a line
with equal indentation continues under the deepest retained ancestor; a line
with greater indentation is rejected.

## Graph construction and identity

Parsing is append-only. Node IDs are assigned in the order segments are
encountered, including all segments on a line before any later line. The IDs
are local to the returned graph and are not source offsets. The graph remains
an ordered rooted forest:

- Every root is the first segment of a line with zero leading tabs.
- Every non-root is either a later segment on the same line or the first
  segment of a line attached to an existing ancestor.
- Parent IDs always refer to nodes already in the arena. A jump cannot produce
  an unknown parent because it is rejected before indexing `ancestors`.
- Child order is append order, so source order is preserved.
- Each node has exactly one parent or is a root. The parser never creates
  shared references or cycles.
- A successful parsed node has nonempty text with no TAB, CR, or LF. Spaces
  and all other allowed characters remain part of that text.

For the example document above, the arena state is:

| ID | Text | Parent | Children |
| ---: | --- | --- | --- |
| 0 | `system` | root | 1, 3 |
| 1 | `master` | 0 | 2 |
| 2 | `device` | 1 | none |
| 3 | `workers` | 0 | 4, 5 |
| 4 | `worker 0` | 3 | none |
| 5 | `worker 1` | 3 | none |

The `Graph` type keeps its arena and root list private. Callers inspect the
result through `len`, `is_empty`, `roots`, `node`, and `nodes`; each `Node`
exposes `text`, `parent`, and `children`. See [`Graph`](../../src/graph.rs)
for those accessors and [`NodeId::index`](../../src/graph.rs) for the numeric
index of an ID.

## Public entry points and round trips

Use the parser through one of these equivalent public forms:

```rust
use recipe_ogdl::Graph;

let from_method = Graph::parse("root\tchild\nnext")?;
let from_trait: Graph = "root\tchild\nnext".parse()?;
# Ok::<(), recipe_ogdl::ParseError>(())
```

Both forms call the same crate-private `parse` function and return
`Result<Graph, ParseError>`. There is no partial graph on an error. Although
nodes from earlier lines may already have been appended to the parser's local
arena when a later line fails, that local graph is discarded with the `Err`.

For a successful graph, `to_canonical_string` and `Display` use the serializer,
not the parser. Parsing canonical output reconstructs an equivalent forest:

```text
input text -> Graph::parse -> Graph -> to_canonical_string -> Graph::parse
```

Canonical output uses LF, emits no final line ending, writes the first child
inline with its parent, and writes later siblings on indented lines. Therefore
the graph and node text are preserved, but input bytes need not be preserved:
CRLF becomes LF, and a noncanonical line layout may be rewritten. Spaces inside
node text remain unchanged.

## Errors and failure precedence

The parser returns only the variants of `ParseErrorKind` currently defined by
the crate. `ParseError` stores both the kind and a `SourceLocation`; callers
can inspect them with `kind()` and `location()`. `ParseErrorKind` is
`#[non_exhaustive]`, so downstream matches must include a wildcard for future
variants.

| Kind | Trigger | Location |
| --- | --- | --- |
| `EmptyNode` | Empty input line, a line containing only indentation tabs, adjacent post-indentation tabs, or a trailing delimiter tab. | For an empty line, the line start. For all-leading-tabs, the byte after the tabs. For a delimiter error, the offending delimiter or its end position. |
| `IndentationJump { found, maximum }` | A nonempty line has more leading tabs than `ancestors.len()`. | The line start, column 1. `found` is the leading-tab count and `maximum` is the available ancestor count at that line. |
| `BareCarriageReturn` | A CR byte is not immediately followed by LF. | The CR byte itself, before line parsing. |

The parser reports one error and stops. The order is observable:

1. The outer scan detects a bare CR as soon as it reaches it. A bare CR on the
   current line wins over that line's indentation or token errors.
2. At each LF, `parse_line` checks all-leading-tabs before indentation jumps,
   then checks consecutive or trailing delimiters from left to right.
3. A line is appended only after all of its tokenization checks succeed. An
   earlier line error stops processing before later lines are considered.

### Source locations

`SourceLocation` uses one-based line and column numbers and a zero-based UTF-8
byte offset into the original input:

- `line` starts at 1 and increases once per LF. CRLF increases it once, not
  twice.
- `column` is computed with `chars().count() + 1` over the bytes before the
  reported position on that line. It counts Unicode scalar values, not UTF-8
  bytes or grapheme clusters. The indentation-jump location is always column
  1, and an all-tabs line reports column `indentation + 1`.
- `offset` is the original byte index. It can differ from the column when the
  line contains multibyte characters. The offset of an empty-line error is the
  line start; the offset of a bare CR is the CR byte.

For a CRLF line, `parse_line` receives a slice that ends before the CR. An
`EmptyNode` position calculated at that slice end therefore has an offset that
points at the stripped CR in the original input. For a final unterminated line,
an end-position error such as a trailing delimiter can use `input.len()` as its
offset, one byte past the last input byte.

`Display` renders the human-readable kind and the one-based line and column,
but not the byte offset. The current messages are:

```text
empty node at line <line>, column <column>
indentation depth <found> skips an ancestor; maximum depth is <maximum> at line <line>, column <column>
bare carriage return; use LF or CRLF at line <line>, column <column>
```

The exact machine-readable kind and all three location fields remain available
through the accessors, so callers that need byte-accurate diagnostics should
not parse the display string.

## Cost model

The normal successful path scans each input byte for line framing, scans each
line's characters for indentation and delimiters, and copies each segment once
into its owned `String`. It is linear in the input length plus the number of
nodes, with amortized vector growth for the graph's node and child arrays. A
source location on an error may count characters in the line prefix once more
to compute its Unicode column, but it does not change the successful-path
complexity. The parser is iterative; tree depth consumes the `ancestors` vector
rather than call-stack recursion.

## Limits and non-goals

There are no parser configuration values or explicit limits for input bytes,
line length, number of lines, number of segments, node count, or indentation
depth. The implementation uses `usize` byte indices and standard `Vec` and
`String` allocations. In normal operation, memory grows with the input's
owned node text and graph relationships, plus the current ancestor path and
the borrowed segment list for one line. Resource exhaustion or an allocation
failure is outside the `ParseError` result; the result type reports syntax
errors only.

The parser intentionally does not provide:

- space-based indentation or mixed indentation rules;
- a way to include a literal tab, CR, or LF in one node;
- blank-line, comment, quoting, escaping, or continuation support;
- shared nodes, anchors, links, references, or cycles;
- recovery, multiple-error collection, or a partial graph on failure; or
- a separate parser configuration or compatibility mode.

These restrictions are part of the current `recipe-ogdl` contract. Consumers
that need a richer document format must add an explicit format rather than
assuming that this parser silently accepts or preserves those features.
