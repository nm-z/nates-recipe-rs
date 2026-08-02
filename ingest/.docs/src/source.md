<!--
Intent: document the bounded filesystem boundary in ingest/src/source.rs and
the real preparation paths that consume its snapshots. The source module owns
one-file admission and content identity. Format detection and payload framing
remain downstream concerns.
-->

# `ingest/src/source.rs`

## Intent

`source.rs` is Recipe's single-file preparation snapshot boundary. It opens one
path during preparation, proves that the opened object is a regular file,
admits no more than a caller-fixed nonzero byte bound, closes the file before
returning, and computes a SHA-256 digest over the exact retained bytes. The
result is an immutable `SourceSnapshot`: later parsing and runtime code receive
bytes, a display path, and content identity, not an open handle or a callback
that can read the filesystem again.

The module deliberately does not select a file format, parse a table, decode a
model, traverse a directory, inspect a ZIP member, or perform numerical
preprocessing. Those operations consume the snapshot or already-admitted byte
slices in the caller. A source snapshot is therefore a preparation fact, not a
model calculation and not runtime work.

### Contract summary

```text
module: recipe_ingest::source
public facade: ingest/src/lib.rs:67
primary operation: read_source_snapshot(path, SourceLimit) -> SourceResult<SourceSnapshot>
input: one filesystem path and one nonzero u64 byte limit
filesystem effect: open, metadata, and one bounded read during preparation
output: path copy, exact bytes, Digest(SHA-256(bytes)); file is closed
read bound: at most limit + 1 bytes, with the extra byte used only to detect overflow
success condition: opened object is regular, metadata length <= limit, bytes read <= limit
failure: SourceError with a stable kind, optional path, and human detail
runtime contract: no handle, callback, or path re-read is retained
format selection: downstream, never in source.rs
payload calculation: downstream, never in source.rs
```

`SourceLimit` is the constructor-enforced nonzero boundary. `SourceSnapshot`
owns the bytes and exposes only borrowed views or an explicit consuming move.
`SourceErrorKind` is non-exhaustive so external callers must allow future error
categories. `SourceError` keeps the original path when an operation has one,
but the source module does not canonicalize, normalize, or resolve that path.

## Position in the crate and preparation lifecycle

`ingest/src/lib.rs` keeps the module private (`mod source;`) and re-exports
`SourceError`, `SourceErrorKind`, `SourceLimit`, `SourceResult`,
`SourceSnapshot`, and `read_source_snapshot` at line 67. The crate-level
documentation states the intended lifecycle: external files are copied into
bounded, content-addressed preparation snapshots and closed before runtime.
The snapshot boundary is shared by dataset ingestion, tokenizer loading,
semantic-model loading, GGUF model loading, and native-kernel resume
authentication.

The common lifecycle is:

```text
public declaration or preparation request
    -> caller derives a domain-specific byte limit
    -> SourceLimit::new(nonzero u64)
    -> read_source_snapshot(path, limit)
    -> caller parses or authenticates snapshot.bytes()
    -> caller stores owned bytes or decoded preparation state
    -> runtime receives no source handle and performs no filesystem read
```

`source.rs` has no feature gate and no unsafe code. The digest type comes from
`recipe_core::Digest`; the hashing implementation is `sha2::Sha256`. The
source module does not define a digest domain, schema number, or path-binding
rule. Its digest is exactly the SHA-256 of the retained bytes, so equal bytes
from different paths have equal `Digest` values and a path change alone does
not change the digest.

## Public surface and data grammar

The public types and functions are all in `ingest/src/source.rs`:

| symbol | representation | purpose |
| --- | --- | --- |
| `SourceLimit` | tuple struct containing `NonZeroU64` | Caller-fixed maximum bytes for one external file. |
| `SourceLimit::new` | `fn(u64) -> SourceResult<SourceLimit>` | Rejects zero and retains the exact nonzero value. |
| `SourceLimit::bytes` | `const fn(self) -> NonZeroU64` | Returns the bound without exposing a mutable representation. |
| `SourceSnapshot` | private `PathBuf`, `Vec<u8>`, `Digest` fields | Closed, owned, content-addressed source image. |
| `SourceSnapshot::path` | `&Path` | Borrows the path copy supplied to the read call. |
| `SourceSnapshot::bytes` | `&[u8]` | Borrows all retained source bytes. |
| `SourceSnapshot::digest` | `Digest` | Copies the SHA-256 content identity. |
| `SourceSnapshot::into_bytes` | `Vec<u8>` | Consumes the snapshot and moves the retained bytes. |
| `SourceErrorKind` | non-exhaustive enum | Stable source-boundary failure categories. |
| `SourceError` | public `kind`, `path`, `detail` | Error value with optional path context. |
| `SourceResult<T>` | `Result<T, SourceError>` | Alias used by the constructor and reader. |
| `read_source_snapshot` | `fn(&Path, SourceLimit) -> SourceResult<SourceSnapshot>` | Performs the complete one-file admission operation. |

The implementation uses private `SourceError::new` and `SourceError::for_path`
helpers. Callers can inspect the public fields or use `Display`; they cannot
construct an arbitrary source error through a public constructor.

### Internal structure map

The file is intentionally small. The source of truth for each operation is:

| lines | item | role and ownership |
| --- | --- | --- |
| 1-9 | imports | `File`, `Read`, `Path`/`PathBuf`, `recipe_core::Digest`, and `sha2::Sha256`; no parser or runtime dependency is introduced. |
| 11-32 | `SourceLimit` and its impl | Nonzero one-file byte policy, with no filesystem access. |
| 34-57 | `SourceSnapshot` and its impl | Private owned path/bytes/digest state and borrow-or-move projections. |
| 59-67 | `SourceErrorKind` | Non-exhaustive source admission categories. |
| 69-101 | `SourceError`, private builders, `Display`, `Error` | Error context and formatting; no recovery or retry behavior. |
| 103 | `SourceResult<T>` | Public result alias shared by the constructor and reader. |
| 105-115 | reader documentation | States the one-read, closed-handle, bounded-snapshot contract. |
| 116-177 | `read_source_snapshot` | The only filesystem operation: open, inspect, bound, read, measure, hash, and return. |

There are no private parser helpers, format enums, global caches, retries, or
background tasks in this module. A change that adds one of those concerns
belongs at the caller boundary that actually needs it, not in the snapshot
admission operation.

The logical output grammar is:

```text
SourceLimit = NonZeroU64(bytes)

SourceSnapshot = {
    path: PathBuf,       // exact caller path copy, not canonicalized
    bytes: Vec<u8>,      // 0..=SourceLimit.bytes().get() bytes
    digest: Digest,      // SHA-256(bytes), wrapped in recipe_core::Digest
}

SourceResult<T> = Ok(T) | Err(SourceError)

SourceError = {
    kind: InvalidLimit | Io | NotRegularFile | LimitExceeded
           | ArithmeticOverflow,
    path: None | PathBuf,
    detail: String,
}
```

The `bytes` inequality is a postcondition of a successful snapshot. The
reader may transiently retain one extra byte while proving a concurrent growth
violation, but that path returns an error and never exposes those bytes in a
snapshot.

## `SourceLimit`

`SourceLimit` is defined at lines 11-32. It is `Clone`, `Copy`, `Debug`,
`PartialEq`, and `Eq`, so passing it to a preparation helper does not transfer
ownership of configuration. Its only field is a `NonZeroU64`, which prevents a
successful value from representing an empty admission window.

`SourceLimit::new(bytes)` calls `NonZeroU64::new(bytes)`:

```text
bytes == 0       -> Err(SourceError {
                         kind: InvalidLimit,
                         path: None,
                         detail: "source byte limit must be nonzero",
                     })
bytes != 0       -> Ok(SourceLimit(bytes))
```

The constructor does not reject `u64::MAX`. That value is nonzero and is
therefore a valid `SourceLimit` value, but the reader cannot form `MAX + 1`
for its growth-detection read cap. `read_source_snapshot` consequently returns
`ArithmeticOverflow` for that value before reading. This is a deliberate,
checked failure rather than a wrapped bound.

The bound is one-file input to `source.rs`. Aggregate data limits are owned by
`IngestLimits` in `table.rs` and by the model/tokenizer limit types in their
respective crates. Callers translate their configured limit into `SourceLimit`
before entering this module.

## `SourceSnapshot`

`SourceSnapshot` is defined at lines 34-57. Its fields are private, and the
type is `Clone`, `Debug`, `PartialEq`, and `Eq`:

| field | construction | observable contract |
| --- | --- | --- |
| `path: PathBuf` | `path.to_path_buf()` at return | Exact path spelling supplied to the read call. It can be relative, contain symlink components, or be non-canonical. |
| `bytes: Vec<u8>` | `read_to_end` from the bounded file reader | Complete admitted byte sequence, including arbitrary binary bytes and empty input. |
| `digest: Digest` | `Digest::new(Sha256::digest(&bytes).into())` | SHA-256 of `bytes`, with no path, limit, timestamp, or metadata mixed into the digest. |

The open `File` is local to `read_source_snapshot`. It is consumed by
`take(read_limit)` and dropped when the function returns, whether the result is
success or error. A successful snapshot can be cloned or borrowed, but no API
can use it to reopen the path. `bytes()` borrows the vector, while
`into_bytes()` consumes the snapshot and moves it out. These choices make the
closed-byte image explicit at every downstream boundary.

The type accepts empty regular files. Their bytes are an empty slice and their
digest is SHA-256 of the empty input. Dataset distillation may later reject a
source collection that yields no samples, but that policy is not part of the
single-file snapshot contract.

`SourceSnapshot::digest` returns the opaque `recipe_core::Digest` by value.
`Digest` stores exactly 32 bytes, exposes those bytes only through its own
`bytes()` method, and does not hash or validate them. The source module is the
producer that chooses SHA-256; consumers compare the opaque value where their
own authentication contract requires it.

## `SourceErrorKind` and `SourceError`

`SourceErrorKind` is defined at lines 59-67 and marked `#[non_exhaustive]`.
Current meanings are:

| kind | produced by | detail shape | path |
| --- | --- | --- | --- |
| `InvalidLimit` | `SourceLimit::new(0)` | `source byte limit must be nonzero` | none, because no path is an input to the constructor |
| `Io` | open, metadata, or bounded read failure | `open source: ...`, `inspect source: ...`, or `read source: ...` | supplied path |
| `NotRegularFile` | opened object has `metadata.is_file() == false` | `external source is not a regular file` | supplied path |
| `LimitExceeded` | metadata is too large or the read returns more than the limit | `source metadata reports ...` or `source grew to ...` | supplied path |
| `ArithmeticOverflow` | `limit + 1` or `bytes.len() -> u64` cannot be represented | `source read bound exceeds u64` or `read source length cannot be represented as u64: ...` | supplied path |

`SourceError` is `Clone`, `Debug`, `PartialEq`, and `Eq`; its public fields are
the exact category, optional path, and owned detail. `Display` writes:

```text
{kind:?}: {detail}
{kind:?}: {detail} [{path.display()}]       // when path is Some
```

It implements `std::error::Error`, with no separate source error chain. A
downstream layer can therefore preserve the source error as one typed cause or
map the category into its own domain error while retaining `path` and `detail`.

## `read_source_snapshot` algorithm

The public function is lines 105-177. Its sequence is intentionally linear and
fail-closed. It never returns a partially filled snapshot.

### Ordered stages

1. **Open the path.** `File::open(path)` creates the handle. Any failure is
   `SourceErrorKind::Io` with detail `open source: {error}` and the supplied
   path.
2. **Inspect the opened handle.** `file.metadata()` is used, rather than a
   second path lookup. Failure is `Io` with detail `inspect source: {error}`.
3. **Require a regular file.** If `metadata.is_file()` is false, return
   `NotRegularFile` with detail `external source is not a regular file`.
4. **Apply the metadata early refusal.** If `metadata.len() > limit.bytes().get()`,
   return `LimitExceeded` with the reported byte count and configured limit.
   The file is not read in this case.
5. **Compute the growth-detection cap.** `limit.bytes().get().checked_add(1)`
   produces `read_limit`. Overflow returns `ArithmeticOverflow` with detail
   `source read bound exceeds u64`.
6. **Choose an allocation hint.** The metadata length is converted to `usize`,
   with `usize::MAX` as the conversion fallback, then capped at `1_048_576`
   bytes. This is only the initial vector capacity, not the admission bound.
7. **Read through a hard cap.** `file.take(read_limit)` wraps the handle and
   `read_to_end` copies at most `limit + 1` bytes into a new `Vec<u8>`. A read
   failure is `Io` with detail `read source: {error}`.
8. **Check the actual length.** `bytes.len()` is converted to `u64`. A failed
   conversion returns `ArithmeticOverflow` with detail
   `read source length cannot be represented as u64: {error}`. If the count is
   greater than the configured limit, return `LimitExceeded` with detail
   `source grew to {count} bytes while reading, limit is {limit}`.
9. **Hash only admitted bytes.** Compute `Sha256::digest(&bytes)`, copy its
   32-byte result into `Digest::new`, and construct the snapshot with the path
   copy, bytes, and digest.
10. **Close before handoff.** The local reader and file are dropped as the
    function returns. The caller receives only owned preparation data.

Equivalent implementation-level pseudocode is:

```text
file = File::open(path) or Err(Io("open source: ...", path))
metadata = file.metadata() or Err(Io("inspect source: ...", path))
if !metadata.is_file():
    return Err(NotRegularFile("external source is not a regular file", path))
if metadata.len() > limit.bytes():
    return Err(LimitExceeded("source metadata reports ...", path))
read_limit = checked_add(limit.bytes(), 1)
    or return Err(ArithmeticOverflow("source read bound exceeds u64", path))
capacity = min(try_usize(metadata.len()) or usize::MAX, 1_048_576)
bytes = read_to_end(file.take(read_limit), capacity)
    or return Err(Io("read source: ...", path))
count = try_u64(bytes.len())
    or return Err(ArithmeticOverflow("read source length ...", path))
if count > limit.bytes():
    return Err(LimitExceeded("source grew to ...", path))
digest = Digest::new(SHA256(bytes))
return Ok(SourceSnapshot { path: path.copy(), bytes, digest })
```

The operation's only successful filesystem observation is the opened file's
metadata and byte stream. There is no post-read path metadata check, no
canonicalization, and no second read.

## Bounded-read invariants and race behavior

The metadata check and the capped read serve different purposes:

| condition | result | reason |
| --- | --- | --- |
| file already reports more than the limit | immediate `LimitExceeded` | Avoid allocation and parsing for an obviously oversized source. |
| file reports at most the limit and stays at most the limit | successful snapshot | All bytes were admitted and hashed. |
| file shrinks after metadata | successful snapshot containing the shorter stream | The actual stream is the authoritative retained image; shrinking does not violate the upper bound. |
| file grows after metadata but the open handle reads at most the old size | successful snapshot of the bytes read | A growth not visible through this opened handle cannot add bytes to the image. |
| file grows while the open handle is read and at least `limit + 1` bytes are available | `LimitExceeded` after reading one extra byte | `take(limit + 1)` prevents a concurrent growth from bypassing the configured bound. |
| read returns an I/O error after partial progress | `Io`, partial vector dropped | No partial source is exposed to callers. |
| path is replaced after `File::open` | the already-open file object remains the source | Metadata and bytes are tied to the handle, while `SourceSnapshot::path` remains the caller's display path. |

`read_limit = limit + 1` is the key invariant. Reading exactly `limit` bytes
would make a source that grows during the operation indistinguishable from an
exactly-at-limit source. Reading one extra byte gives a deterministic refusal,
while `take` keeps the memory/read exposure bounded. A successful snapshot
always satisfies `bytes.len() <= limit`.

The `1_048_576` capacity cap is not a second policy bound. It prevents a large
metadata value from forcing an equally large initial allocation. `read_to_end`
may grow the vector as needed, but only through the `limit + 1` reader cap.

The constructor's nonzero invariant is the only validation of the limit itself.
The reader trusts the `NonZeroU64` representation and checks only operations
that can overflow while deriving its cap or measuring the result.

## Path and object invariants

`source.rs` checks the object reached by `File::open`, not the path spelling:

* A regular path is accepted when its opened metadata reports a regular file.
* A directory or other non-regular object returns `NotRegularFile` if the
  platform permits it to be opened far enough to inspect metadata. A platform
  `open` failure is reported as `Io` instead.
* A direct caller may pass a symlink. `File::open` follows it, and the target
  is accepted if the opened handle reports a regular file. Symlink refusal is a
  higher-level dataset policy, described below, not a `source.rs` policy.
* The path is copied exactly into the snapshot and into errors. No absolute
  conversion, slash normalization, symlink resolution, or existence check is
  performed outside the open and metadata operation.
* Empty regular files are valid snapshots. Whether an empty image is a valid
  logical dataset is decided by the downstream format or dataset accumulator.

Opening before metadata also means the metadata used for the early size check
belongs to the same open object that supplies the bytes. The implementation
does not claim a global filesystem transaction: another process can still
modify the open file while it is being read. The `limit + 1` cap and final
length check are the guarantees that matter for admission.

## Downstream call graph

The source API is reused at several preparation boundaries. All current
callers are listed below; `rg` finds no other `SourceSnapshot` or
`read_source_snapshot` consumers in the workspace.

| caller | source location | limit source | bytes/digest use | outer error boundary |
| --- | --- | --- | --- | --- |
| dataset regular-file helper | `ingest/src/dataset.rs:658-663` | `IngestLimits.source_bytes()` | Moves `snapshot.into_bytes()`. Path and digest are intentionally discarded because dataset metadata computes its own content hash while appending logical tables. | `DatasetSourceError` through `dataset_error_from_source`. |
| tokenizer file loader | `text/src/lib.rs:452-469` | `TextLimits.model_bytes` converted to `u64` | Borrows `snapshot.bytes()` and passes it to `Tokenizer::from_json`; path and digest are not retained. | `TextErrorKind::Source` for source failures; tokenizer parsing then has its own errors. |
| dense checkpoint loader | `training/src/inference.rs:683-694` | `CheckpointDecodeLimits.source_bytes` | Borrows bytes for strict `decode_checkpoint`. | `InferencePreparationError::CheckpointSource` via `From<SourceError>`. |
| KNN model loader | `training/src/inference.rs:696-709` | `KnnModelDecodeLimits.source_bytes` | Borrows bytes for strict `decode_knn_model`. | `InferencePreparationError::CheckpointSource`. |
| categorical Bayes model loader | `training/src/inference.rs:711-723` | `BayesModelDecodeLimits.source_bytes` | Borrows bytes for strict `decode_bayes_model`. | `InferencePreparationError::CheckpointSource`. |
| semantic model dispatcher | `training/src/inference.rs:725-780` | maximum of checkpoint, KNN, and default Bayes source limits | Borrows bytes first for root probe, then passes the same snapshot bytes to exactly one strict decoder. | `InferencePreparationError::CheckpointSource` for source failures; decoder errors for root/format failures. |
| GGUF llama loader | `training/src/gguf_llama.rs:478-483` | `GgufLimits.file_bytes()` | Borrows bytes for `decode_gguf_llama`. | `InferencePreparationError::CheckpointSource` for source failures; `GgufLlamaError` for model failures. |
| native training-resume loader | `src/training.rs:761-845` | default checkpoint source-byte bound | Compares `snapshot.digest()` with the authenticated native-kernel digest, then moves bytes into `Arc<[u8]>`. | `TrainingError::NativeKernelSource` for source failures; `CheckpointError::IncompatibleResume` for digest mismatch. |

The model loaders all run during preparation. Their decoded artifacts, not the
snapshot or a path, cross into the later graph and native stages. The native
resume path is the only current consumer of the snapshot digest itself.

### Dataset boundary in detail

`ingest/src/dataset.rs` imports the source API at lines 15-22. Its
`read_regular_file` helper does exactly this:

```text
limit = SourceLimit::new(limits.source_bytes().get())
bytes = read_source_snapshot(path, limit)?.into_bytes()
```

`SourceError` is translated by `dataset_error_from_source`:

| `SourceErrorKind` | `DatasetSourceErrorKind` |
| --- | --- |
| `InvalidLimit` | `LimitExceeded` |
| `LimitExceeded` | `LimitExceeded` |
| `Io` | `Io` |
| `NotRegularFile` | `InvalidPath` |
| `ArithmeticOverflow` | `ArithmeticOverflow` |

The source path and detail are copied into the dataset error. The invalid-limit
case is grouped with limit failures because `IngestLimits` should already have
rejected zero bounds; if a zero reaches this helper, it is an admission-limit
failure at the dataset boundary.

`distill_datasets` in `dataset.rs:517-540` first collects the declared paths,
rejects an empty collection, decides whether source metadata columns are
needed, and visits each source in declaration order. `distill_one_source`
performs `fs::symlink_metadata` before calling the source reader. This is why
dataset ingestion rejects a source symlink even though a direct
`read_source_snapshot` caller may follow one:

```text
distill_one_source(path)
    -> symlink_metadata(path)
    -> Symlink error if the path itself is a symlink
    -> directory traversal or read_regular_file(path)
```

Directory entries are checked the same way before reading. Directory traversal
is sorted by file name, and archive members are sorted by enclosed path, so the
same source tree or ZIP produces deterministic row order. Dataset traversal
uses the source snapshot only for each regular external file. Bytes inside a
ZIP are already in memory and are bounded separately by the archive visitor.

After bytes are admitted, `visit_bytes` selects the format. The source module
does not participate in this selection. `visit_bytes` checks XLSX and PPTX by
extension first, then generic ZIP by extension or magic, and only then calls
`admit_leaf` and `parse_leaf` for an ordinary leaf. The outer archive is not
counted as a leaf; each expanded member is read through its own `limit + 1`
cap and admitted against aggregate leaf-byte limits.

`Accumulator` additionally enforces aggregate limits for leaf bytes, samples,
vectors, and individual field values. It appends logical tables into one
rectangular `RawTable`, and `finish` refuses to return a dataset when no regular
file or no sample was produced. Consequently, a successful source snapshot is
necessary for a dataset file but is not sufficient for a successful distilled
dataset.

### Format routing after a snapshot

`SourceFormat` is defined in `dataset.rs:31-53`. Its `as_str` values are stable
lowercase labels used in source-context metadata. The `parse_leaf` routing table
is:

| routing condition, evaluated in order | parser/shape | `SourceFormat` and semantic intent |
| --- | --- | --- |
| extension `csv` or `tsv` | `parse_delimited`, header present; CSV auto-sniffs, TSV uses tab | `Delimited`, inferred columns |
| extension `all-data`, `dat`, `data`, `data-numeric`, `tra`, or `trn` | `parse_delimited`, header absent | `Delimited`, inferred columns |
| extension `json` | `parse_json`, with arrays/objects/scalars converted to one or more logical tables | `Json`, inferred or key-classified columns |
| extension `gguf` | `parse_gguf_tables`, bounded GGUF metadata/tensor inspection | `Gguf`, metadata/tensor tables or one binary payload |
| extension `safetensors` | `parse_safetensor_tables`, bounded safetensor metadata/tensor inspection | `SafeTensors`, metadata/tensor tables or one binary payload |
| extension `png` and PNG signature | one `image` payload | `Image`, exact image/bytes encoding |
| extension `inp`, `out`, `patch`, or `sh` | UTF-8 `parse_text` | `Text`, exact UTF-8 encoding |
| extension `txt` with structural table shape | `parse_delimited`, header absent | `Delimited`, inferred columns |
| extension `txt` otherwise | UTF-8 `parse_text` | `Text`, exact UTF-8 encoding |
| extension `bin`, `logits`, or `model` | one `binary` payload | `Binary`, exact binary/bytes encoding |
| any extension with recognized image signature | one `image` payload | `Image`, exact image/bytes encoding |
| UTF-8, printable/control-safe bytes, and structural table shape | `parse_delimited`, header absent | `Delimited`, inferred columns |
| UTF-8, printable/control-safe bytes | `parse_text` | `Text`, exact UTF-8 encoding |
| all remaining bytes | one `binary` payload | `Binary`, exact binary/bytes encoding |

`extension` lowercases only the final path extension. `parse_delimited` chooses
tab for `tsv`, auto for `csv`, and otherwise uses the structural delimiter
sniffer or `Delimiter::Auto`. The structural sniffer considers up to eight
nonempty lines and recognizes tab, semicolon, comma, or numeric ASCII
whitespace with consistent widths. The table parser performs framing and its
own per-record/per-field checks; dataset wraps those failures as
`DatasetSourceErrorKind::Ingest`.

The specialized routes preserve encoded payload bytes rather than decoding
calculation payloads on the CPU. GGUF and safetensors tables expose metadata,
shape, rank, encoded byte counts, and raw encoded tensors. A format parser can
produce multiple `LogicalTable` values, such as one table per JSON member or
XLSX worksheet; `Accumulator::append` merges them into the global rectangular
table.

### Containers and nested bounded reads

There are two container paths after the outer source snapshot:

* Generic ZIP is detected by `.zip` extension or the `PK` local/empty/spanned
  signatures. `visit_archive` refuses depth `>= 32`, rejects malformed ZIPs,
  rejects member names that escape the archive root, ignores directory entries,
  sorts remaining members by enclosed path, and rejects an empty archive.
* Each ZIP member's declared uncompressed size must be at most the source byte
  limit. The member stream is read with `take(limit + 1)`, an initial capacity
  capped at `1_048_576`, and an actual decoded-length check. The resulting bytes
  are recursively routed through `visit_bytes`, so a nested member can itself
  be another ZIP, a spreadsheet, or any ordinary leaf. Aggregate leaf-byte,
  record, vector, and depth limits still apply.

XLSX and PPTX are handled before generic ZIP. Their preflight pass sums every
ZIP member's declared expansion and rejects an expansion above the source byte
limit or a `u64` sum overflow before calamine or XML extraction. XLSX exposes
nonempty worksheets as spreadsheet tables. PPTX reads slide XML, sorts slides
by number, and emits numeric `slide` plus UTF-8 `text` rows. These preflight
checks protect decompression expansion, while the outer regular file was
already bounded by `read_source_snapshot`.

### Source context and the discarded snapshot digest

When a directory member, archive member, or one of multiple declared sources
is appended, `Accumulator::append` creates `SourceMetadata`. It stores source
index, logical path, parent, folder, file name/stem, extension, format, member,
hex SHA-256, byte count, zero-based sample index/count, and nesting depth as
ordinary vectors. The digest is recomputed directly with `Sha256::digest(bytes)` and
encoded as 64 lowercase hex bytes. This is equivalent in value to
`SourceSnapshot::digest()` for the same bytes, but `read_regular_file` has
already consumed the snapshot with `into_bytes`, so the `Digest` object itself
does not flow into the accumulator.

For one direct regular-file declaration, source context is disabled and only
the logical file's vectors are exposed. A directory, archive, or multi-source
declaration enables context so rows retain their source identity. Data columns
whose names collide with context names receive a `data:` prefix. Duplicate
local names receive `#2`, `#3`, and so on. These are dataset semantics, not
source-snapshot semantics.

## Direct model and tokenizer consumers

### Tokenizer model

`text::Tokenizer::from_file` converts the caller's `TextLimits.model_bytes`
from `usize` to `u64`, then constructs `SourceLimit`. Conversion failure is a
`TextErrorKind::ArithmeticOverflow`; a zero model bound becomes
`TextErrorKind::Source` containing the source error. A source read failure is
also wrapped as `TextErrorKind::Source` with the source error's display text.
On success, `snapshot.bytes()` enters `Tokenizer::from_json`, which checks the
same model-byte limit again before handing bytes to the tokenizer parser. The
second check protects the in-memory API when callers use `from_json` directly;
it is not a second filesystem read.

### Semantic model loaders

`training/src/inference.rs` has three direct typed loaders. Each converts its
decode-limit `source_bytes` to `u64`, constructs `SourceLimit`, reads one
snapshot, and passes only the retained bytes to its strict decoder. `SourceError`
converts through `From<SourceError>` into
`InferencePreparationError::CheckpointSource`. The decoder then owns version,
canonicality, schema, and model-family checks.

`load_semantic_model_file` must choose a decoder from the first root segment,
but it must not read the path twice. It therefore computes the maximum source
bound among the dense checkpoint, KNN, and default Bayes limits, reads one
snapshot, and probes bytes through the first newline/tab. The root must be
UTF-8 and exactly one of:

```text
"recipe"           -> decode_checkpoint(...)
"recipe-knn-model" -> decode_knn_model(...)
"recipe-bayes-model" -> decode_bayes_model(...)
other              -> checkpoint decode error: unknown semantic-model root
```

The root probe is only dispatch. The selected decoder remains responsible for
complete syntax and validation, and all three branches consume the same
snapshot bytes.

`src/inference.rs` selects this loader for `.ogdl` model paths and the GGUF
loader for `.gguf` paths during `compile_inference_package`. After model load,
`distill_data` uses the separate dataset snapshot/format path for inference
data. Source admission therefore happens independently for the model and data
files, each under its own typed bound.

### GGUF llama loader

`training/src/gguf_llama.rs::load_gguf_llama_model_file` uses
`limits.file_bytes().get()` as the `SourceLimit`, reads one snapshot, and
passes `snapshot.bytes()` to the strict GGUF llama decoder. The source digest
is not used here because GGUF validation is structural and tensor metadata is
the decoder's responsibility. The public training crate re-exports this loader
through `training/src/lib.rs:62-74`.

## Native resume authentication

`src/training.rs::load_resume_native_bundle` is the only current direct
consumer of `SourceSnapshot::digest()`. The surrounding code has already:

1. validated that the optional native resume path exists,
2. selected `.cubin` or `.hsaco` from the path extension,
3. loaded a semantic checkpoint and authenticated native realization metadata,
4. matched the requested format to exactly one recorded kernel, and
5. checked the recorded program digest against the current compiled training
   program.

It then derives a `u64` bound from the default checkpoint decode limits,
constructs `SourceLimit`, and reads the kernel through `read_source_snapshot`.
The comparison is:

```text
snapshot.digest() == authenticated_kernel.digest()
```

Mismatch returns `CheckpointError::IncompatibleResume` with detail that the
supplied native kernel bytes do not match the digest authenticated by the
semantic model. On a match, the bytes are moved into `Arc<[u8]>` and the
snapshot path and digest are no longer needed. A source read failure is kept as
`TrainingError::NativeKernelSource`, so missing, non-regular, oversized, or
I/O-failed kernel inputs do not become an unauthenticated fallback.

This path demonstrates why the digest is content-only and computed before the
file handle is closed: the semantic model records a digest of the artifact,
and the resume boundary can authenticate the exact bytes without retaining a
filesystem capability. The source module does not decide whether a kernel is
compatible; it only supplies bounded bytes and their digest. Program, target,
toolchain, topology, and format checks remain in the training caller.

## Failure propagation by boundary

The source error categories remain precise at the source API and are translated
only where a caller has a broader domain vocabulary:

```text
SourceLimit::new(0)
    -> SourceError(InvalidLimit, path=None)

read_source_snapshot(path, limit)
    -> SourceError(Io | NotRegularFile | LimitExceeded | ArithmeticOverflow,
                   path=Some(path))

dataset::read_regular_file
    -> DatasetSourceError( LimitExceeded | Io | InvalidPath | ArithmeticOverflow )
       with source path and detail retained

text::Tokenizer::from_file
    -> TextError(Source) for source construction/read errors

training model loaders
    -> InferencePreparationError::CheckpointSource

training native resume
    -> TrainingError::NativeKernelSource for source admission,
       CheckpointError::IncompatibleResume for digest mismatch
```

No caller treats `LimitExceeded` as a truncation success. No caller receives a
partial byte vector on error. Format decoders are reached only after source
admission succeeds, and decoder failures remain distinct from source failures.

## Invariants owned by this module

The following are implementation invariants, not caller assumptions:

1. A successful `SourceLimit` contains a nonzero `u64`.
2. A successful snapshot came from one opened object whose metadata reported a
   regular file.
3. A successful snapshot contains no more than the configured limit.
4. The read path is capped at `limit + 1`, and checked arithmetic refuses a cap
   that cannot be represented.
5. Metadata is an early refusal and capacity hint only. The actual read length
   is checked independently.
6. The digest is SHA-256 over exactly the retained bytes, with no other input.
7. The path copy is descriptive context, not part of digest identity.
8. The file handle is not retained beyond the function call.
9. Errors drop any partial bytes and expose no partial snapshot.
10. The module performs no format routing, payload decoding, or numerical
    transformation.

The following are deliberately *not* invariants of `source.rs`:

* a path must be absolute or canonical;
* a path must not be a symlink;
* a source must be nonempty or parseable;
* one source's bytes must fit an aggregate dataset budget;
* a source must be valid JSON, table data, GGUF, safetensors, text, image, or
  another format;
* a digest proves a model's schema, target, program, or deployment
  compatibility;
* runtime can reopen a path or obtain fresh bytes.

Those rules belong to the dataset walker, format-specific decoders, model
loaders, or training authentication code described above.

## Boundary distinctions that prevent misuse

### `source.rs` versus `table.rs`

`ingest/src/table.rs::read_table` has a separate `read_bounded` helper. It also
checks metadata, caps a read at `IngestLimits.source_bytes() + 1`, and rejects
growth after the read, but it returns an un-hashed `Vec<u8>` directly to
`parse_table`. `dataset.rs` uses `parse_table` on bytes that came through
`SourceSnapshot`; direct callers of `read_table` do not obtain a
`SourceSnapshot` or a content digest. The two APIs therefore have distinct
contracts:

| API | output | regular-file check | digest | format |
| --- | --- | --- | --- | --- |
| `read_source_snapshot` | closed path/bytes/digest snapshot | yes, on opened metadata | SHA-256 | none |
| `read_table` | framed `RawTable` | no dedicated `is_file` check in `read_bounded` | none | delimited/whitespace table |
| `parse_table` | framed `RawTable` from caller bytes | not applicable | none | delimited/whitespace table |

Use `read_source_snapshot` when a preparation boundary needs a closed source
image or content authentication. Use `parse_table` when bytes are already
admitted and the caller needs table framing. Do not infer format behavior from
the snapshot type.

### Snapshot identity versus dataset metadata hash

Dataset source context stores a lowercase hexadecimal SHA-256 string, while
`SourceSnapshot::digest()` stores a typed 32-byte `Digest`. They represent the
same hash function over the same bytes when computed for the same leaf, but
they are different representations and are consumed by different contracts.
The dataset context hash is a model-visible vector; the snapshot digest is an
opaque preparation identity that native resume can compare without converting
to text.

### Source admission versus format validity

An accepted snapshot proves only bounded regular-file bytes. For example,
accepted bytes may still be malformed ZIP, invalid UTF-8 for a text route,
inconsistent table rows, an unsupported GGUF architecture, or an unknown
semantic-model root. Those failures occur after the source boundary and must
not be collapsed into `SourceErrorKind`.

## Review checklist

When changing this module, preserve these observable facts unless the public
contract is intentionally versioned:

```text
[ ] zero limits fail at SourceLimit::new, before any filesystem operation
[ ] regular-file validation uses metadata from the opened handle
[ ] metadata oversize is refused before reading
[ ] actual read uses checked limit + 1 and rejects growth
[ ] successful bytes length is <= the configured limit
[ ] partial reads never escape as snapshots
[ ] SHA-256 covers exactly the retained bytes
[ ] SourceSnapshot owns bytes and cannot retain a file capability
[ ] path spelling is preserved for diagnostics, not mixed into digest
[ ] source errors retain their precise kind and path at the source boundary
[ ] downstream format routing remains outside source.rs
[ ] native resume continues comparing the typed digest, not a path or text hash
```

The normal structural checks are the workspace Rust build and formatter. They
do not prove filesystem race behavior or model authentication. Runtime
acceptance for callers must invoke their public preparation entry point with a
real source file and independently inspect the resulting admitted bytes,
decoded artifact, or authenticated kernel state. A compile success or a
matching error string alone is not evidence that the complete caller boundary
worked.
