# `AuditError`

`AuditError` is the error boundary for the `recipe-audit` crate. It is defined
in `audit/src/error.rs` and re-exported by the library facade at
`audit/src/lib.rs:21`. The enum represents a malformed, incomplete, unsafe, or
unreadable input to the audit gate. It is separate from a `Finding`: a finding
is a valid observation that can be reported (and, in legacy mode, possibly
grandfathered), while an `AuditError` means that the requested audit could not
produce a trustworthy report. The module-level documentation for
`audit::audit` makes that boundary explicit: malformed facts, duplicate facts,
invalid grants, and stale grants are errors rather than a clean report
(`audit/src/lib.rs:32-41`).

The enum has six variants:

| Variant | Payload | Concrete meaning |
| --- | --- | --- |
| `Configuration(String)` | Human-readable message | A caller, CLI, or injected fact violates an input or mode invariant. |
| `InvalidDependencyGraph(String)` | Human-readable message | A typed dependency graph has missing, duplicate, or empty structural data. |
| `InvalidCargoMetadata(String)` | Human-readable message | Cargo metadata JSON is malformed, incomplete, or not the required format. |
| `Lexical { path, line, reason }` | Source path, one-based line, static reason | The language-aware lexer reached an unterminated comment or literal. |
| `InvalidElf { path, reason }` | Filesystem path and human-readable reason | An ELF input is not a regular file, exceeds the ELF bound, or cannot be parsed as ELF. |
| `Io { path, source }` | Filesystem path and `std::io::Error` | A filesystem operation failed before facts could be collected. |

An inventory of the current source confirms the construction split: 37 direct
`Configuration` constructions (16 in `main`, 8 in `model`, 3 in `lib`, and 10
in `native`), 7 `InvalidDependencyGraph` constructions, 12
`InvalidCargoMetadata` constructions, 1 `Lexical` mapping, and 3
`InvalidElf` constructions. `Io` has 2 direct constructions in the CLI and 11
native call sites routed through `AuditError::io`. The `error.rs` display and
source matches are not counted as construction sites.

All variants are `Debug`; the enum is not `Serialize`, `Clone`, or `Eq`. Because
the public enum is re-exported, library callers can construct or exhaustively
match its six variants. The only named helper constructor is the crate-private
`AuditError::io` helper
(`audit/src/error.rs:24-31`), which preserves the path and the original I/O
error. There is no `From` conversion that silently changes an error category.
Callers either construct the appropriate variant or use an explicit
`map_err`, and `?` propagates it unchanged.

## Construction and purpose by variant

### `Configuration`

This is the broad input-contract error. Every construction site is a direct
rejection, not a fallback:

* `AuditMode::from_str` rejects a mode other than the exact `next` or `legacy`
  spellings with `unknown mode ...; expected next or legacy`
  (`audit/src/model.rs:27-40`). The CLI reaches this implementation through
  `--mode` parsing (`audit/src/main.rs:149-152`).
* `LegacyGrant::validate` rejects an empty path or symbol, wildcard syntax in
  a path or symbol, and a path that is not slash-normalized
  (`audit/src/model.rs:152-170`). `apply_legacy_grants` calls this validator
  for every grant before indexing it (`audit/src/lib.rs:82-95`).
* `AuditInput::validate` rejects duplicate source paths, duplicate ELF paths,
  empty or non-normalized source, linker-input, and ELF paths, empty linker
  arguments, and empty ELF library or symbol facts
  (`audit/src/model.rs:285-314`). Its duplicate-path helper and display-path
  helper construct the same variant for the duplicate and normalization cases
  (`audit/src/model.rs:317-339`). `recipe_audit::audit` invokes this validation
  before collecting any findings (`audit/src/lib.rs:42-43`).
* `audit` rejects legacy grants in `next` mode, rejects duplicate grants while
  indexing legacy grants, and rejects grants that did not match a finding
  (`audit/src/lib.rs:68-77`, `82-117`). The exact messages are respectively
  `next mode cannot accept legacy grants`, `duplicate legacy grant: ...`, and
  `unused or stale legacy grants: ...`.
* CLI orchestration rejects missing `--mode` or `--scope`, an unpaired
  `--metadata` or `--package-id`, `--legacy-grants` in `next` mode, missing
  grants in `legacy` mode, and malformed grant JSON
  (`audit/src/main.rs:27-73`). Grant JSON deserialization is deliberately
  wrapped with the grant path in the message. Once the JSON has the expected
  typed shape, semantic grant errors come later from `LegacyGrant::validate`
  and do not add the grants-file path.
* `read_bounded_text` requires an absolute path and a regular file no larger
  than `MAX_JSON_BYTES`; it returns `Configuration` for a relative path or a
  type/size violation (`audit/src/main.rs:99-117`). Actual metadata and read
  failures at the same boundary are `Io`, described below.
* CLI argument parsing maps a non-UTF-8 argument, an unknown option, an empty
  package ID or linker input, a missing option value, a non-UTF-8 option value,
  and a repeated single-use option to `Configuration`
  (`audit/src/main.rs:139-192`, `196-211`). `next_utf8` and
  `reject_duplicate` are the shared helpers for the last two cases.
* Report JSON serialization is explicitly mapped to `Configuration` in
  `run` (`audit/src/main.rs:88-90`). The path is the final output boundary,
  so any serializer error is reported using the same configuration channel.
* Native collection rejects a non-absolute scope, a scope that is not a real
  directory, more than `MAX_SOURCE_FILES`, an oversized text file, a
  non-absolute or non-regular/symlink ELF path, an ELF outside the canonical
  scope, a duplicate ELF path, a symlink encountered during recursive source
  collection, and a canonical path that escaped its scope
  (`audit/src/native.rs:33-51`, `54-65`, `70-94`, `168-202`, `231-246`).
  These checks establish the explicit-scope and no-symlink contract before
  source or ELF facts are exposed to policy evaluation.

`Configuration` therefore covers both command-line shape and in-memory input
shape. It does not represent a policy violation found in valid source, graph,
linker, or ELF facts; those are returned as `Finding` values by the audit
functions. In particular, `audit_linker_inputs` and `audit_elf_facts` in
`audit/src/artifact.rs` are infallible vector-producing policy passes, and the
classifiers in `audit/src/policy.rs` return `Allowed`, `Prohibited`, or
`Unknown`, never an `AuditError`. Aggregate validation is what rejects empty
fact fields before those pure passes run through `audit`.

The JSON boundaries use different variants by ownership: Cargo metadata JSON
is parsed as `InvalidCargoMetadata`, legacy-grant JSON is configuration owned
and therefore maps serde failures to `Configuration`, and final report JSON
serialization is also mapped to `Configuration` (`audit/src/main.rs:67-71`,
`88-90`).

### `InvalidCargoMetadata`

`DependencyGraph::from_cargo_metadata_json` is the sole construction boundary
for this variant (`audit/src/dependency.rs:61-146`), with
`required_string` providing the common nonempty-string check
(`audit/src/dependency.rs:249-253`). The parser rejects, in order as
encountered:

1. An empty caller-supplied root ID list (`audit/src/dependency.rs:68-72`).
2. JSON that `serde_json` cannot parse (`audit/src/dependency.rs:73-74`), or a
   top-level JSON value that is not an object (`audit/src/dependency.rs:75-77`).
3. A metadata `version` other than exactly numeric `1`
   (`audit/src/dependency.rs:78-81`).
4. A missing or non-array `packages` field (`audit/src/dependency.rs:84-87`), a
   package entry that is not an object (`audit/src/dependency.rs:89-92`), or a
   package `id`, `name`, or `manifest_path` that is absent, non-string, or
   empty (`audit/src/dependency.rs:93-97`, through
   `required_string`).
5. A missing or non-object `resolve` value, including metadata produced with
   `--no-deps` (`audit/src/dependency.rs:100-107`), or a missing/non-array
   `resolve.nodes` (`audit/src/dependency.rs:108-111`).
6. A resolve node that is not an object (`audit/src/dependency.rs:114-117`), a
   node without a nonempty string `id` (`audit/src/dependency.rs:118`), or a
   duplicate resolve node ID (`audit/src/dependency.rs:119-123`).
7. A missing or non-array `resolve.nodes[].dependencies` value
   (`audit/src/dependency.rs:125-130`), or a dependency entry that is not a
   nonempty string package ID (`audit/src/dependency.rs:131-135`,
   through `required_string`).

The parser then builds a typed `DependencyGraph` and calls its structural
validator (`audit/src/dependency.rs:139-145`). That validator can return
`InvalidDependencyGraph`, not `InvalidCargoMetadata`, when the JSON has passed
the field-shape checks but the resulting graph has duplicate package IDs,
missing roots, or invalid edges. Resolve dependency edges are inserted into a
`BTreeSet` while parsing (`audit/src/dependency.rs:112-136`), so duplicate dependency entries in the
JSON are deduplicated at this parser boundary rather than producing a graph
error; duplicate edges supplied directly through `DependencyGraph::new` are
handled by `InvalidDependencyGraph`.

The CLI obtains the JSON only after `read_bounded_text` has accepted an
absolute, bounded regular file (`audit/src/main.rs:35-45`). Library callers can
invoke `DependencyGraph::from_cargo_metadata_json` directly. In either case,
the parser returns the first encountered error and no graph is returned. The
metadata parser receives only `&str`, so an `InvalidCargoMetadata` display does
not retain the `--metadata` path; the CLI path is retained for file I/O errors,
but not added to parser errors.

### `InvalidDependencyGraph`

`DependencyGraph::validate` is the sole constructor for this variant
(`audit/src/dependency.rs:195-246`). It protects the graph traversal in
`DependencyGraph::audit` (`audit/src/dependency.rs:148-192`) from interpreting absent or ambiguous
package facts as proof of a clean dependency closure. The concrete checks are:

* at least one root package ID is present (`audit/src/dependency.rs:196-200`);
* every package has a nonempty ID, name, and manifest path (`audit/src/dependency.rs:202-207`);
* package IDs are unique (`audit/src/dependency.rs:208-214`);
* every selected root exists in the package map (`audit/src/dependency.rs:216-221`);
* every edge source and target names an existing package (`audit/src/dependency.rs:224-237`); and
* every directed edge is unique (`audit/src/dependency.rs:238-243`).

`DependencyGraph::from_cargo_metadata_json` calls this validator before it
returns a parsed graph (`audit/src/dependency.rs:139-145`). The runtime audit
also calls it through `DependencyGraph::audit` whenever an optional graph is
present (`audit/src/lib.rs:57-59`), so a graph built directly with
`DependencyGraph::new` receives the same validation. A graph error aborts the
audit before its dependency findings are returned.

### `Lexical`

`audit_source` maps the private lexer's `LexError` to this variant
(`audit/src/source.rs:16-29`). The lexer emits only four static reasons
(`audit/src/lexer.rs:18-60`):

* `unterminated block comment`, using the line where the block comment began;
* `unterminated string literal`, using the opening line;
* `unterminated character literal`, using the opening line; and
* `unterminated raw string literal`, using the opening line.

The lexer tracks nested block comments and language-specific comments, strings,
character literals, Rust lifetimes, and Rust raw-string hash delimiters. A
failure is mapped with the source unit's path, line, and static reason. The
special self-hosted policy dictionary path `audit/src/policy.rs` is returned as
an empty finding list before lexing (`audit/src/source.rs:17-21`), so its policy
spellings do not create a lexical error or a source finding.

There is no partial result on a lexical error: `audit_source` returns `Err`,
and the top-level `audit` propagates that error while iterating sources
(`audit/src/lib.rs:53-56`). A direct library caller of `audit_source` sees the
same variant.

### `InvalidElf`

`read_elf_facts_with_display` constructs this variant after the path has passed
the absolute-path check (`audit/src/native.rs:106-109`):

* a path whose metadata says it is not a regular file produces
  `reason = "not a regular file"` (`audit/src/native.rs:111-117`);
* an ELF file larger than `MAX_ELF_BYTES` produces a formatted size reason
  (`audit/src/native.rs:119-123`); and
* a file that `goblin::elf::Elf::parse` cannot parse produces the parser's
  string error (`audit/src/native.rs:125-131`).

Metadata lookup and byte reads that fail at this function are `Io`, not
`InvalidElf`. `collect_native_scope` performs additional symlink, regular-file,
canonical-scope, and duplicate checks before calling this helper
(`audit/src/native.rs:68-94`), so those collection-level violations are
`Configuration`; a real in-scope file that fails ELF shape or parsing reaches
`InvalidElf`. The direct public `read_elf_facts` API can therefore return
`InvalidElf` for an absolute directory, an oversized regular file, or invalid
ELF bytes.

The payload retains the filesystem `PathBuf` used for the read. In
`collect_native_scope` that path is the canonical in-scope path passed to the
private helper, while a successful collection stores a separate normalized
relative display path in `ElfFacts`; direct `read_elf_facts` retains the
caller's absolute path.
The error is returned before `DT_NEEDED` or undefined-symbol facts are built,
so no ELF findings or report are emitted for that artifact.

### `Io`

`AuditError::io` (`audit/src/error.rs:24-30`) is used by native collection and
ELF reading to retain both the operation path and the original
`std::io::Error`. The call sites are:

* `collect_native_scope`: scope `symlink_metadata` and `canonicalize`, source
  file metadata and UTF-8 reads (including a `read_to_string` invalid-data
  error), explicitly named ELF metadata and canonicalization
  (`audit/src/native.rs:35`, `42`, `56`, `63`, `72`, `79`);
* `read_elf_facts_with_display`: ELF metadata and byte reads
  (`audit/src/native.rs:112`, `125`); and
* recursive `collect_paths`: directory reads, collection of directory entries,
  and entry metadata (`audit/src/native.rs:169-177`).

The CLI's `read_bounded_text` constructs `Io` directly for JSON metadata/grant
file metadata and reads (`audit/src/main.rs:106-123`). It does this after the
absolute-path and regular-file/size checks, so the variant denotes an actual
filesystem failure rather than a rejected path shape.

There is no retry, alternate path, or substitute fact. Each `map_err` returns
the `Io` immediately and every surrounding `?` preserves it through native
collection, metadata loading, or CLI orchestration. The original error remains
available through the standard error source chain, as described below.

## Formatting and error-source behavior

`audit/src/error.rs:33-51` is the complete `Display` implementation:

```text
Configuration:         invalid audit configuration: {message}
InvalidDependencyGraph: invalid dependency graph: {message}
InvalidCargoMetadata:  invalid Cargo metadata: {message}
Lexical:                {path}:{line}: lexical audit failed: {reason}
InvalidElf:             invalid ELF {path.display()}: {reason}
Io:                    {path.display()}: {source}
```

`Lexical` uses the stored string path and one-based line directly. `InvalidElf`
and `Io` use `PathBuf::display()`, so their text reflects the path supplied to
the filesystem operation. `Io` is the only variant with a nested source:
`impl std::error::Error for AuditError` returns `Some(source)` for `Io` and
`None` for every other variant (`audit/src/error.rs:54-60`). This lets a library
caller inspect the underlying OS error without changing the stable display
prefix. Within the current workspace, the CLI is the only display consumer; it
uses `Display`, not `Debug` or the nested source chain. The library re-export is
the boundary for external callers that need variant matching or source-chain
inspection.

The prefixes and lexical reason strings are literals owned by this crate.
Metadata parser text comes from `serde_json::Error::to_string`, and ELF parse
text comes from `goblin`'s error string, so those dynamic reason suffixes are
passed through rather than normalized by `AuditError`.

## Propagation and user-visible outcome

The public library facade re-exports `AuditError`, and its main entry point
returns `Result<AuditReport, AuditError>` (`audit/src/lib.rs:19-42`). Its
propagation order is:

1. `AuditInput::validate` rejects malformed injected facts.
2. Each `SourceUnit` is passed to `audit_source`; a lexical error stops the
   source loop.
3. An optional `DependencyGraph` is validated and traversed; graph errors stop
   dependency auditing.
4. Linker and ELF facts produce ordinary findings, while native collection
   errors have already stopped the CLI before this point.
5. In `next` mode, any supplied grant is a configuration error. In `legacy`
   mode, grant validation, duplicate detection, and stale-grant detection can
   return configuration errors.

The `?` operators at `audit/src/lib.rs:43`, `55`, `58`, and `76` preserve the
variant and message. There is no recovery branch in the library. A successful
call can still return a report with blocking findings, but an `AuditError`
means no report is returned. Since each stage returns immediately, the first
failing stage wins: input validation precedes source lexing, source lexing
precedes graph validation, and the CLI's native collection precedes its
metadata and grant relationship checks. Fixing one reported condition and
rerunning can therefore expose a later independent condition; the current
call never aggregates or hides those later errors.

The public entry points expose the same distinction at different boundaries:

| Entry point | Error result |
| --- | --- |
| `recipe_audit::audit` | `Configuration`, `Lexical`, or `InvalidDependencyGraph` from aggregate evaluation and grant policy; native and metadata errors occur only if the caller has already collected them. |
| `audit_source` | `Lexical` for incomplete source text. |
| `DependencyGraph::from_cargo_metadata_json` | `InvalidCargoMetadata` for document shape, then `InvalidDependencyGraph` for graph invariants. |
| `collect_native_scope` | `Configuration`, `Io`, or `InvalidElf` while collecting a scope and explicit artifacts. |
| `read_elf_facts` | `Configuration`, `Io`, or `InvalidElf` for one absolute artifact. |
| `audit_linker_inputs`, `audit_elf_facts` | No `AuditError`; they return finding vectors after receiving fact-shaped inputs. |

Each fallible operation builds its result locally and returns `Err` before the
result crosses its boundary: collection does not return a partial
`NativeCollection`, metadata parsing does not return a partial graph, source
auditing does not return partial findings after a lexical failure, and
`audit` does not return the findings accumulated before a later error.

The binary boundary is `main` in `audit/src/main.rs:9-17`. Any `Err(error)`
from CLI parsing, native collection, metadata parsing, grant loading, the
library audit, or report serialization is rendered as:

```text
recipe-audit: {error}
{usage text}
```

and exits with `ExitCode::from(2)`. A valid audit with blocking findings is a
different outcome: `run` prints the serialized `AuditReport` and exits with
code `1`; a passing report exits with code `0` (`audit/src/main.rs:88-96`).
Thus malformed or unreadable evidence cannot be mistaken for a policy pass,
and it is not represented as a JSON finding. The only user-visible recovery is
to correct the reported option, fact, source text, metadata, graph, ELF input,
or filesystem condition and invoke the same boundary again; the implementation
does not automatically recover or continue with partial evidence.
