# Error model

`recipe-ogdl` exposes five error-related public types from
[`ogdl/src/lib.rs`](../../src/lib.rs#L15-L21):
`SourceLocation`, `NodeTextErrorKind`, `GraphError`, `ParseErrorKind`, and
`ParseError`. They describe two different failure boundaries:

- `GraphError` reports a failed mutation through `Graph::append_root` or
  `Graph::append_child`.
- `ParseError` reports a failed textual parse through `Graph::parse` or
  `str::parse::<Graph>()`.

The parser never returns `GraphError`, and the serializer never returns an
error. A graph that reaches serialization has already passed the relevant
invariants.

## `SourceLocation`

`SourceLocation` is the location payload carried by `ParseError`. The complete
definition and implementation are in
[`ogdl/src/error.rs:5-31`](../../src/error.rs#L5-L31):

```rust
pub struct SourceLocation {
    line: usize,
    column: usize,
    offset: usize,
}
```

The fields are private. Construct and inspect a location with:

```rust
pub const fn new(line: usize, column: usize, offset: usize) -> Self;
pub const fn line(self) -> usize;
pub const fn column(self) -> usize;
pub const fn offset(self) -> usize;
```

`SourceLocation` derives `Clone`, `Copy`, `Debug`, `PartialEq`, and `Eq`.

The coordinates have different units:

| accessor | unit and origin |
| --- | --- |
| `line()` | one-based input line number |
| `column()` | one-based Unicode scalar-value position within the line |
| `offset()` | zero-based UTF-8 byte offset in the complete input string |

The parser starts at line 1 and computes columns with `str::chars().count()`.
It computes offsets from byte indices. Therefore a non-ASCII scalar can consume
more than one byte while still advancing the displayed column by one. The
offset points into the original `&str`, not into a normalized or serialized
copy.

`SourceLocation::new` stores its arguments without checking or normalizing
them. The parser is the only constructor used by the crate's normal parse
path.

## `NodeTextErrorKind`

`NodeTextErrorKind` is a non-exhaustive, copyable classification for text that
cannot be stored as one node by the arena-building API:

The enum is defined at
[`ogdl/src/error.rs:33-41`](../../src/error.rs#L33-L41). It derives
`Clone`, `Copy`, `Debug`, `PartialEq`, and `Eq`.

| variant | meaning in `Graph::append_root` or `Graph::append_child` |
| --- | --- |
| `Empty` | The supplied `String` has length zero. |
| `Tab` | A tab scalar occurs in the supplied text. |
| `CarriageReturn` | A carriage-return scalar occurs in the supplied text. |
| `LineFeed` | A line-feed scalar occurs in the supplied text. |

The enum is marked `#[non_exhaustive]`; callers must leave a wildcard when
matching it so a future variant can be added without breaking downstream
code.

The parser gives tabs structural meaning, so a parsed node never produces
`NodeTextErrorKind::Tab`. It splits a line at every tab after the leading
indentation instead. Likewise, carriage returns and line feeds are handled as
line-ending syntax or rejected as syntax before a parsed node is created.

## `GraphError`

`GraphError` is a non-exhaustive failure to extend one in-memory graph. Its
definition and `Display` implementation are at
[`ogdl/src/error.rs:43-68`](../../src/error.rs#L43-L68):

```rust
pub enum GraphError {
    InvalidNodeText {
        kind: NodeTextErrorKind,
        character: usize,
    },
    UnknownParent(NodeId),
}
```

It derives `Clone`, `Debug`, `PartialEq`, and `Eq`. The enum is
`#[non_exhaustive]`, so external matches also require a wildcard.

### `InvalidNodeText`

`InvalidNodeText` is constructed by the private `validate_text` helper in
[`ogdl/src/graph.rs:149-171`](../../src/graph.rs#L149-L171). Validation runs before `push_node`, so an unsuccessful
append does not change the graph.

The `character` field is one-based and counts Unicode scalar values, because
the validator enumerates `text.chars()`. It is not a UTF-8 byte offset and is
not a `SourceLocation` column. The first invalid scalar wins. An empty string
has no scalar to enumerate, so it reports `character: 1` with
`NodeTextErrorKind::Empty`. For a non-empty string, the first tab, carriage
return, or line feed reports its one-based scalar position.

`Graph::append_root` converts its argument with `Into<String>`, validates the
result, and only then inserts a root. `Graph::append_child` first checks that
the `NodeId` resolves in this graph. It returns `UnknownParent` immediately for
an absent or foreign (graph-local) identity, before converting or validating
the text. If the parent exists, it converts and validates exactly as
`append_root` does.

### `UnknownParent`

`UnknownParent(NodeId)` is constructed only by `Graph::append_child` at
[`ogdl/src/graph.rs:79-85`](../../src/graph.rs#L79-L85), when
`Graph::node(parent)` returns `None`. `NodeId` identities are local to one
`Graph`; an index that is valid in another graph is still unknown here. The
graph is unchanged.

### Display and error source

`GraphError` implements `Display` with these exact forms:

```text
invalid node text at character {character}: {kind:?}
unknown parent node {parent.index()}
```

For example, an empty root is displayed as
`invalid node text at character 1: Empty`, and an invalid parent with index 7
is displayed as `unknown parent node 7`. The `kind` part uses the `Debug`
spelling of `NodeTextErrorKind`, including any fields a future variant might
add. `GraphError` implements `std::error::Error` and does not override
`source()`, so it has no nested source error.

### Graph-building propagation sites

All public graph-building callers use `?` or an explicit `map_err` around the
two append methods:

- `language/src/ogdl.rs` builds a `CalculationGraph` image. Its
  `OgdlCodecError::Build(GraphError)` variant receives append failures from
  `encode_graph`, `field_value`, and the nested primitive and scalar encoders.
  `OgdlCodecError` preserves the `GraphError` as its `source()` and displays
  it as `cannot encode Recipe IR graph: {error}`.
- `program/src/lib.rs` builds the `RecipeProgram` envelope with
  `to_ogdl_graph`, `append_field`, and `copy_subtree`. Its
  `ProgramError::Build(GraphError)` variant preserves the error as its
  `source()` and displays `cannot build program OGDL: {error}`.
- `training/src/bayes_checkpoint.rs` maps append failures in `child_root` and
  `child` to `CheckpointError::InvalidManifest` details prefixed with
  `encode Bayesian ...`.
- `training/src/knn_checkpoint.rs` maps append failures in its `encode_graph`
  and `child` helpers to `CheckpointError::InvalidManifest` details prefixed
  with `encode KNN ...`.

The generated names and values in the language and program encoders are
validated before insertion and are intended to be single node texts. A
`GraphError` from those paths still remains a real propagated failure rather
than being replaced by another graph implementation.

## `ParseErrorKind`

`ParseErrorKind` is the non-exhaustive syntax classification used by the
parser. The variants are defined at
[`ogdl/src/error.rs:70-77`](../../src/error.rs#L70-L77):

| variant | parser condition |
| --- | --- |
| `EmptyNode` | A line or tab-delimited segment has no node text. |
| `IndentationJump { found, maximum }` | Leading tab indentation is deeper than the current ancestor stack. `found` is the leading-tab count and `maximum` is the available ancestor count. |
| `BareCarriageReturn` | A `\r` byte is not immediately followed by `\n`. |

`ParseErrorKind` derives `Clone`, `Debug`, `PartialEq`, and `Eq`; it is not
`Copy`. Its `#[non_exhaustive]` marker requires a wildcard in downstream
matches.

### `EmptyNode` construction

`ParseErrorKind::EmptyNode` has three lexical forms, all constructed in
[`ogdl/src/parser.rs`](../../src/parser.rs):

1. `parse_line` rejects an empty line or a line containing only leading tabs
   when `indentation == source.len()`. The location is the first position
   after the leading tabs: line `line`, column `indentation + 1`, and byte
   offset `line_offset + indentation`.
2. While scanning a line, two adjacent tabs after the leading indentation
   create an empty segment. The `empty_node_error` helper points at the second
   tab's position.
3. A tab at the end of a line leaves the final segment empty. The same helper
   points one column after the final preceding segment, at the end byte.

The helper computes the displayed column by counting Unicode scalars before
the offending byte, but the stored offset remains a byte index.

### `IndentationJump` construction

Each line begins with a count of leading tab bytes. The parser keeps the
current chain in `ancestors`; its length is the greatest valid indentation for
the next line. If `indentation > ancestors.len()`, it returns
`IndentationJump { found: indentation, maximum: ancestors.len() }` at line
column 1 and the line's starting byte offset. The first line therefore cannot
start with a tab: its maximum is zero. A line with multiple tab-delimited
segments can increase `ancestors.len()` enough for deeper following lines.

### `BareCarriageReturn` construction

The byte scanner checks every carriage-return byte before parsing a line. A
`\r` followed by `\n` is accepted as CRLF and the carriage return is removed
from the slice passed to `parse_line`. Any other `\r`, including one at end of
input or one followed by another `\r`, returns
`BareCarriageReturn`. Its location uses the current line, the scalar count
before the byte plus one, and the byte's zero-based input offset.

## `ParseError`

`ParseError` is the syntax error value returned by parsing. Its fields and
accessors are at
[`ogdl/src/error.rs:79-95`](../../src/error.rs#L79-L95):

```rust
pub struct ParseError {
    kind: ParseErrorKind,
    location: SourceLocation,
}
```

The fields are private. The public API is:

```rust
pub const fn new(kind: ParseErrorKind, location: SourceLocation) -> Self;
pub const fn kind(&self) -> &ParseErrorKind;
pub const fn location(&self) -> SourceLocation;
```

`new` stores the supplied pair exactly. It does not verify that a location
matches a kind, and it is therefore also usable by callers that need to create
their own parse-compatible diagnostic.

`ParseError` implements `Clone`, `Debug`, `PartialEq`, `Eq`, and
`std::error::Error`. Like `GraphError`, it has no nested source error.

### Display forms

`ParseError` renders the kind-specific message followed by the one-based line
and column. The byte offset is intentionally not included in `Display`:

The exact `Display` match is
[`ogdl/src/error.rs:97-115`](../../src/error.rs#L97-L115).

```text
empty node at line {line}, column {column}
indentation depth {found} skips an ancestor; maximum depth is {maximum} at line {line}, column {column}
bare carriage return; use LF or CRLF at line {line}, column {column}
```

Callers that need the byte offset must retain the value from
`ParseError::location().offset()`. Wrappers that convert a parse error with
`error.to_string()` retain the line and column text but necessarily lose the
offset and the typed `ParseErrorKind` unless they separately inspect the error
before conversion.

## Parse entry points and propagation

`Graph::parse` delegates to the crate-private parser and has the signature
`Result<Graph, ParseError>`. `impl FromStr for Graph` sets
`type Err = ParseError` and delegates to the same method. `ogdl/src/lib.rs`
re-exports the error types, so callers do not need access to the private
`error` module.

The entry points are [`Graph::parse`](../../src/graph.rs#L43-L53) and the
`FromStr` implementation at
[`ogdl/src/graph.rs:137-141`](../../src/graph.rs#L137-L141).

The repository's direct propagation sites are:

| caller | conversion | resulting rendering and source chain |
| --- | --- | --- |
| [`language::CalculationGraph::from_ogdl`](../../../language/src/ogdl.rs#L106-L113) | `?` converts `ParseError` to `OgdlCodecError::Syntax` | `invalid OGDL syntax: {error}`; `OgdlCodecError::source()` returns the original `ParseError`. |
| [`program::StaticCalculationProgram::from_ogdl`](../../../program/src/lib.rs#L394-L397) | `?` converts `ParseError` to `ProgramError::Syntax` | `invalid program OGDL syntax: {error}`; `ProgramError::source()` returns the original `ParseError`. |
| [`training::decode_checkpoint`](../../../training/src/checkpoint.rs#L1273-L1299) | maps `error.to_string()` to `CheckpointDecodeErrorKind::InvalidSyntax` at the root checkpoint path | `CheckpointError::Decode` renders `decode checkpoint: <checkpoint>: {line/column message}`; the typed parse kind and byte offset are no longer present. |
| [`training` Bayesian decoder](../../../training/src/bayes_checkpoint.rs#L362-L375) | maps `error.to_string()` to `CheckpointError::InvalidManifest` with `invalid Bayesian OGDL: ` | the manifest detail retains the `ParseError` display text only. |
| [`training` KNN decoder](../../../training/src/knn_checkpoint.rs#L856-L869) | maps `error.to_string()` to `CheckpointDecodeErrorKind::InvalidSyntax` at the root path, prefixed `invalid KNN OGDL: ` | the decode wrapper retains the display text only. |
| [`ingest::structural_ogdl_to_gguf`](../../../ingest/src/gguf_ogdl.rs#L2357-L2368) | maps `error.to_string()` to `GgufOgdlErrorKind::InvalidSyntax`, path `<ogdl>` | `GgufOgdlError` renders `<ogdl>: {line/column message}` and has no source chain. |

The language and program wrappers preserve the typed source through
`std::error::Error::source`; their higher-level compile wrappers do not. Both
`training::TrainingCompileError` and `training::InferenceCompileError` turn
an `OgdlCodecError` or `ProgramError` into a string detail, so a later caller
sees the rendered prefix and line/column text but not a nested
`ParseError`. This is a consequence of those wrapper APIs, not a change to
`recipe-ogdl`'s error values.

## Parser and serializer consequences

### Inputs that do not produce an error

- The empty string returns `Graph::new()` with no roots and no nodes.
- LF line endings are accepted.
- CRLF line endings are accepted. The `\r` byte is discarded before the line
  is split into nodes.
- A final newline after at least one parsed line is accepted. The parser does
  not call `parse_line` for the empty slice after that final newline; an input
  consisting only of a newline is still an empty line and fails.
- Ordinary spaces, leading or trailing spaces in node text, Unicode scalars,
  and other characters except tab, carriage return, and line feed are literal
  node text.

Blank lines inside a document are not ignored. An empty line, an all-tab line,
an adjacent tab delimiter, or a trailing tab produces `EmptyNode`. Consequently
`"a\n"` parses, while `"a\n\n"` fails on line 2. A leading tab on the first
line produces `IndentationJump` because no ancestor exists yet.

Parsing appends nodes with the crate-private `Graph::push_node`, not with the
public append methods. A parse failure can therefore have built a partial
temporary graph internally, but that graph is discarded when `Result::Err` is
returned. No `GraphError` is emitted by the parser.

### Canonical output

`Graph::to_canonical_string` delegates to
[`serializer::serialize`](../../src/serializer.rs#L10-L40) and returns a
`String`, never a `Result`. `impl Display for Graph` at
[`ogdl/src/graph.rs:143-146`](../../src/graph.rs#L143-L146) writes that same string.
The serializer relies on the graph's private arena invariants and uses an
internal `expect` only for a node ID that must belong to the graph; public
construction and parsing maintain that invariant.

For a valid graph, canonical output has these rules:

1. The first root starts at byte zero. Later roots start on a new LF line with
   no indentation.
2. The first child of a node is written inline after one tab. Later children
   start on new LF lines at their depth, using one tab per depth level.
3. Node text is copied byte-for-byte. Because node text cannot contain tabs,
   carriage returns, or line feeds, the output remains unambiguous.
4. Output uses LF regardless of whether the input used LF or CRLF and never
   adds a final newline.

Thus a successful parse followed by `to_canonical_string` may normalize line
endings, remove a permitted trailing newline, and choose the canonical inline
versus line layout, but it preserves the ordered forest and every node's text.
The canonical string can be parsed again without any of the three
`ParseErrorKind` failures. If parsing or graph construction fails first, there
is no graph to serialize.
