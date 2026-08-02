# `audit/src/model.rs`

## Role and boundaries

`model.rs` is the data boundary for the `recipe-audit` gate. It defines the
mode, finding vocabulary, evidence records, aggregate input, and final report
that are shared by the collector, lexical scanners, dependency scanner,
artifact scanner, CLI, and public library entry point. The module does not
read files, parse Cargo metadata, lex source, classify native interfaces, or
write reports. Those operations live in `native.rs`, `dependency.rs`,
`lexer.rs`, `policy.rs`, `source.rs`, `artifact.rs`, and `main.rs`.

The private `model` module's public types are re-exported by `lib.rs`, so the
types in this file are the crate's injection and reporting API. The intended
path is:

```text
native collector / CLI / caller-owned facts
    -> SourceUnit, LinkerInput, ElfFacts, DependencyGraph, LegacyGrant
    -> AuditInput::validate
    -> lib::audit
    -> source, dependency, linker, and ELF scanners
    -> blocking Finding values
    -> deterministic sort and deduplication
    -> mode-specific grant application
    -> AuditReport
    -> CLI JSON and exit status
```

### Trust boundary

Model values are evidence and policy results, not executable instructions. The
model never follows a path, opens a file, invokes Cargo, runs a linker, loads a
library, or expands a grant pattern. Native collection and the CLI enforce
their own absolute-path, scope, symlink, size, and bounded-JSON rules before
assembling records. The model's path helper only changes display separators;
it does not provide filesystem canonicalization or scope containment. Exact
grant validation is the sole deliberate exception path, and it cannot broaden
matching beyond one current finding identity.

Public convenience constructors are side-effect free and marked `#[must_use]`.
The private report constructor is also side-effect free but is not a public
construction boundary. Path-bearing constructors normalize display paths but do
not make records self-validating. Structural validation is deliberately
performed at the `AuditInput` boundary. Grant validation is performed only in
legacy mode. Public fields mean callers can also construct
literals directly; the invariants below describe the normal `AuditInput` ->
`audit` path.

Constructor and producer ownership is intentionally narrow:

| Constructor or operation | Current producer or consumer |
| --- | --- |
| `SourceUnit::new` | `native::collect_native_scope`; public for injected source text |
| `LinkerInput::new` | CLI `--link-input` assembly; public for declarative linker facts |
| `ArtifactSymbol::new`, `ElfFacts::new` | `native::read_elf_facts`; public for injected artifact facts |
| `LegacyGrant::new` | Public programmatic grant construction; CLI JSON uses `Deserialize` instead |
| `Finding::blocking` | Source, dependency, linker, and ELF scanners |
| `AuditInput::empty` | Public convenience for assembling an injected request |
| `AuditReport::new` | Private, called only after `lib::audit` finalizes findings |

Cross-module contract index:

| Module | Model values produced or consumed | Boundary responsibility |
| --- | --- | --- |
| `lib.rs` | `AuditInput`, `Finding`, `LegacyGrant`, `FindingDisposition`, `AuditReport`, `AuditMode` | Aggregate validation, scanner orchestration, sorting/deduplication, grant state transition, and report pass state. |
| `main.rs` | `AuditMode`, `LegacyGrant`, `LinkerInput`, `AuditInput`, `AuditReport` | CLI parsing, bounded JSON acquisition, command-line provenance, JSON serialization, and exit code. |
| `native.rs` | `SourceKind`, `SourceUnit`, `ArtifactSymbol`, `ElfFacts`, `NativeCollection` | Explicit filesystem scope, UTF-8 collection, ELF extraction, path display, and native-source classification. |
| `source.rs` and `lexer.rs` | `SourceUnit`, `SourceKind`, `Finding`, `FindingCategory` | Lexical source policy, line provenance, and source-category mapping. |
| `dependency.rs` | `DependencyPackage`, `DependencyEdge`, `DependencyGraph`, `Finding`, `FindingCategory` | Cargo graph parsing/validation, reachable closure, and dependency findings. |
| `artifact.rs` | `LinkerInput`, `ElfFacts`, `Finding`, `FindingCategory` | Linker-candidate and ELF needed/undefined-symbol policy. |
| `policy.rs` | `FindingCategory` indirectly | Native-interface/library classification; no model record stores classifier state. |

Function index:

| Function | Role |
| --- | --- |
| `AuditMode::as_str`, `AuditMode::from_str`, `AuditMode::fmt` | Stable mode spelling for parsing, display, and report serialization. |
| `FindingCategory::as_str`, `FindingCategory::fmt` | Stable category spelling for Serde and grant diagnostics. |
| `Finding::blocking`, `Finding::key` | Emit a blocking observation and expose its crate-private grant identity. |
| `LegacyGrant::new`, `LegacyGrant::validate`, `LegacyGrant::key` | Construct, validate, and index one exact legacy exception. |
| `SourceUnit::new`, `LinkerInput::new`, `ArtifactSymbol::new`, `ElfFacts::new` | Build owned evidence records, normalizing only path-bearing values. |
| `AuditInput::empty`, `AuditInput::validate` | Assemble an explicit request and enforce aggregate structure. |
| `validate_unique_paths`, `validate_display_path`, `normalize_display_path` | Enforce and produce stable display-path invariants. |
| `AuditReport::new`, `AuditReport::blocking_count` | Derive pass state and count remaining blockers. |

## Policy vocabulary

### `AuditMode`

`AuditMode` (lines 7-45) selects the product boundary being audited:

| Variant | Meaning in `lib::audit` |
| --- | --- |
| `Next` | Replacement code. Any finding remains `Blocking`; a nonempty grant list is a configuration error. |
| `Legacy` | Retained legacy code. Every supplied grant must match one current finding exactly, and every finding that should pass must be grandfathered. |

The enum derives `Clone`, `Copy`, `Debug`, `Eq`, `PartialEq`, and `Serialize`.
Serde's `kebab-case` representation is `"next"` or `"legacy"`. It is not
`Deserialize`; the CLI parses `--mode` through `FromStr`, which accepts only
those two lower-case strings without trimming or case folding. Any other value
returns `AuditError::Configuration` with the original value. `as_str` and
`Display` return the same stable lower-case spelling. The selected value is
stored in both `AuditInput.mode` and `AuditReport.mode`.

`main.rs` also enforces the command-line policy before constructing the input:
next mode rejects `--legacy-grants`, while legacy mode requires an explicit
grant-file path. The library-level `audit` function repeats the next-mode
rejection and applies grants in legacy mode, so callers cannot bypass the
mode boundary by using the library API. In next mode the library checks only
that the grant vector is empty; it does not validate grant contents because no
grant is legal in that mode.

### `FindingCategory`

`FindingCategory` (lines 47-81) is the stable, ordered policy vocabulary shared
by all finding producers and by `LegacyGrant`. It derives `Deserialize` and
`Serialize` with exact `kebab-case` labels, plus `Ord` and `PartialOrd` for
deterministic finding and grant-key ordering. `as_str` and `Display` expose
the same labels in duplicate and stale-grant errors.

| Variant and serialized label | Producer and evidence represented | Line value |
| --- | --- | --- |
| `Dependency` (`dependency`) | Reachable prohibited package name from `DependencyGraph::audit` | `0`, graph fact |
| `SourceApi` (`source-api`) | Prohibited source identifier, or a prohibited string in include/import context, from `source.rs` | One-based source line |
| `BuildLinkInput` (`build-link-input`) | Build-metadata identifier/string, source line with link context, or a prohibited `LinkerInput` candidate | Source line, or supplied linker line, often `0` for CLI input |
| `DynamicNeeded` (`dynamic-needed`) | Prohibited library in `ElfFacts.needed` | `0`, binary fact |
| `LlvmDeclaration` (`llvm-declaration`) | Prohibited LLVM `@symbol` on a line containing `declare` | LLVM source line |
| `LlvmCall` (`llvm-call`) | Prohibited LLVM `@symbol` on a line containing `call`, `invoke`, or `callbr` when it is not a declaration | LLVM source line |
| `UndefinedSymbol` (`undefined-symbol`) | Prohibited undefined symbol in an ELF fact | `0`, binary fact |
| `RuntimeLoad` (`runtime-load`) | Prohibited library or symbol string that is not link or include context | One-based source line |
| `DisallowedNativeInterface` (`disallowed-native-interface`) | A prohibited direct-KFD or unallowlisted CUDA Driver-shaped complete source identifier found by `source.rs` | One-based source line |

The category is part of finding identity. A grant for one category never
matches a finding with the same path, line, and symbol in another category.
The special `DisallowedNativeInterface` mapping is owned by `source.rs`: it
overrides the normal source or LLVM category only for the two native-interface
families above. Linker and ELF scanners retain `BuildLinkInput` or
`UndefinedSymbol`/`DynamicNeeded` even when their classifier reports one of
those families.

`policy.rs` supplies the classifier that precedes these category choices. It
returns `Allowed`, `Prohibited`, or `Unknown` for complete symbols, library
basenames, and dependency names. Allowed ROCr/HSA and exact reviewed CUDA
Driver interfaces are ignored; prohibited HIP, CUDA Runtime, direct KFD,
unallowlisted Driver-shaped symbols, and operation-library families become
findings; unknown values are valid evidence with no finding. The model records
only the resulting category and evidence spelling, never the classifier's
`NativeInterface` value.

### `FindingDisposition`

`FindingDisposition` (lines 83-89) is workflow state, not a policy category.
It has the serialized labels `"blocking"` and `"grandfathered"`, derives
ordering and equality, and is serializable but not deserializable. Every
finding constructor sets `Blocking`. Only `lib.rs::apply_legacy_grants` can
change a generated finding to `Grandfathered` after an exact grant match.

## Findings and legacy exceptions

### `Finding`

`Finding` (lines 91-129) is one exact audit result. Its fields are:

| Field | Contract |
| --- | --- |
| `category` | Stable `FindingCategory` describing the evidence source and policy class. |
| `path` | Display path with backslashes replaced by `/`. For source facts this is the explicit scope path; for graph and ELF facts it is the manifest or artifact display path. |
| `line` | One-based text line, or `0` for graph, binary, and command-line facts. |
| `symbol` | The exact token, package name, library candidate, needed library, or artifact symbol retained by the producer. Classifier normalization does not rewrite this field. |
| `disposition` | Initially `Blocking`; legacy matching may change it to `Grandfathered`. |

The type derives `Clone`, `Debug`, `Eq`, `Ord`, `PartialEq`, `PartialOrd`, and
`Serialize`, but not `Deserialize`. The derived ordering follows declaration
order, so category, path, line, symbol, and disposition provide a stable sort
order. All scanners return blocking findings, sort and deduplicate their local
results, and `lib::audit` performs a final global sort and deduplication.

`Finding::blocking` is the single constructor used by the source, dependency,
linker, and ELF scanners. It converts both generic string arguments to owned
values, normalizes the path with `normalize_display_path`, preserves the
supplied line and symbol verbatim, and sets `disposition` to `Blocking`. It
does not reject an empty path or symbol; the aggregate input validator is the
normal structural gate for those producer records.

`Finding::key` returns an owned tuple
`(FindingCategory, String, u64, String)`. The key intentionally excludes
`disposition`. `apply_legacy_grants` uses it to index grants and to mutate a
matching finding without changing the finding's identity. The key also makes
the exact category, normalized path, line, and emitted symbol the complete
grant identity. There is no path globbing, category family matching, or symbol
prefix matching.

Producer-specific symbol retention matters for grants. `source.rs` may trim a
string token and remove one trailing `\\00` or `\\0`; `artifact.rs` stores the
candidate produced by linker splitting; dependency scanning stores the package
name; and ELF scanning stores the original undefined-symbol spelling even when
the policy classifier strips decoration for classification. A grant must use
the value that appears in the resulting `Finding`, not an independently
normalized classifier value.

### `LegacyGrant`

`LegacyGrant` (lines 131-180) has the four identity fields of a finding but no
disposition. It derives `Clone`, `Debug`, `Deserialize`, `Eq`, `Ord`,
`PartialEq`, `PartialOrd`, and `Serialize`. `#[serde(deny_unknown_fields)]`
means a grant JSON object may contain only `category`, `path`, `line`, and
`symbol`; `FindingCategory` supplies the exact kebab-case category strings.
This is the only model record deserialized by the CLI. The CLI reads a bounded
absolute JSON file into `Vec<LegacyGrant>`; report JSON is output-only.

`LegacyGrant::new` normalizes its path and otherwise stores its arguments
without validation. Deserialized values do not pass through this constructor,
so `validate` is mandatory before use. `validate` rejects, in order:

1. an empty path or symbol;
2. `*` or `?` anywhere in the path, or `*` anywhere in the symbol; and
3. a path whose slash-normalized form differs from the supplied path.

It does not impose a nonzero line, absolute/relative path rule, or additional
symbol character rule. The observed wildcard checks are intentionally exact;
in particular, `?` is rejected in paths but not specially rejected in
symbols. `validate` returns `AuditError::Configuration` for every failure.

`apply_legacy_grants` validates every grant, inserts its `key` into a
`BTreeMap`, and rejects duplicate identity keys. It then walks the sorted,
deduplicated findings, marks exact key matches as `Grandfathered`, and records
used keys. Any indexed key not used by a current finding produces a deterministic
"unused or stale legacy grants" configuration error. Consequently a successful
legacy report has no stale grants, while an ungranted finding remains blocking.

## Evidence input records

### `SourceKind`

`SourceKind` (lines 182-191) is the lexical policy selector for a
`SourceUnit`. It is `Copy`, ordered, and comparable, but intentionally has no
Serde representation because source units are programmatic evidence records.
`native.rs::source_kind` assigns values by supported extension or well-known
build filename:

| Kind | Native collector mapping | Lexer and scanner consequence |
| --- | --- | --- |
| `Rust` | `.rs` | Rust lifetimes and raw strings are recognized; prohibited identifiers are normally `SourceApi`. |
| `Zig` | `.zig` | Generic source lexing; prohibited identifiers are normally `SourceApi`. |
| `C` | `.c`, `.h` | Generic C-like lexing; prohibited identifiers are normally `SourceApi`. |
| `Cpp` | `.cc`, `.cpp`, `.cxx`, `.hh`, `.hpp`, `.hxx` | Generic C++-like lexing; prohibited identifiers are normally `SourceApi`. |
| `LlvmIr` | `.ll` | Semicolon comments, `@` tokens, and declaration/call context produce LLVM-specific categories. |
| `BuildMetadata` | Metadata extensions such as `.toml`, `.json`, `.yaml`, `.cmake`, `.mk`, `.ninja`, `.bazel`, `.bzl`, plus the exact names `Cargo.lock`, `Makefile`, `CMakeLists.txt`, `BUILD`, `BUILD.bazel`, `WORKSPACE`, and `WORKSPACE.bazel` | `#` comments, dependency-name classification, and `BuildLinkInput` categorization. |

The collector skips unsupported files, `.git`, and `target` directories, while
rejecting symlinks. A caller may inject any kind for any contents; the model
does not infer or verify an extension. Native extension and filename matching
is case-sensitive and does not recognize additional aliases.

The source consumer applies these category defaults after policy classification:

| `SourceKind` group | Prohibited identifier | Prohibited string without line context |
| --- | --- | --- |
| `Rust`, `Zig`, `C`, `Cpp` | `SourceApi` | `RuntimeLoad` |
| `LlvmIr` | LLVM declaration/call category when an `@` context is recognized; ordinary identifiers use `SourceApi` | `RuntimeLoad` |
| `BuildMetadata` | `BuildLinkInput` | `BuildLinkInput` because every metadata string is treated as link context |

For any kind, a source line containing an explicit link marker changes a
prohibited string to `BuildLinkInput`; on code kinds, include/import markers
change a non-link string to `SourceApi`. Direct-KFD and unallowlisted Driver
shapes override these defaults to `DisallowedNativeInterface` for identifiers.
An `@`-following LLVM token with no declaration or call context is skipped by
the LLVM-specific path rather than emitted as a generic finding.

### `SourceUnit`

`SourceUnit` (lines 193-210) owns one UTF-8 source or build file in the
explicit scope:

- `path` is the display key used in findings and must be nonempty and slash
  normalized when an `AuditInput` is validated;
- `kind` selects lexical behavior and category defaults; and
- `contents` is the complete UTF-8 text. It may be empty, and the model does
  not parse or size-check it.

`SourceUnit::new` normalizes only the path and moves the kind and contents. The
native collector creates records after enforcing its absolute-scope, file-size,
UTF-8, and symlink rules, using a path relative to the canonical scope. The
public `audit_source` consumer lexes the record, emits findings, and returns a
lexical `AuditError` for unterminated comments or literals. It also contains a
deliberate exact-path escape for `audit/src/policy.rs`, whose policy dictionary
is definitions rather than audited runtime use.

The aggregate validator rejects duplicate source paths and rejects empty or
backslash-containing paths. It does not require paths to be relative, does not
canonicalize `.` or `..`, and does not validate contents or `SourceKind` beyond
the closed enum. Source findings use one-based lexer lines, and the original
display path is carried through `Finding::blocking`.

### `LinkerInput`

`LinkerInput` (lines 212-229) records one declarative or command-line linker
argument. `path` identifies the source of the argument, `line` identifies its
location, and `argument` preserves the complete input string. The constructor
normalizes only `path`.

The CLI maps each repeated `--link-input` value to
`LinkerInput::new("<command-line>", 0, argument)`, so command-line findings
carry line `0`. Build metadata findings instead use the source unit's line.
Programmatic callers can supply a file path and nonzero line. `AuditInput`
validation rejects an empty path, an unnormalized path, or an argument whose
length is zero. Whitespace-only arguments are not rejected by this validator;
the linker scanner trims and splits them and normally produces no candidate.
There is no uniqueness rule for linker inputs, so several arguments may share
one path and line.

`artifact.rs::audit_linker_inputs` splits each argument on whitespace and the
linker punctuation `, = : ( ) [ ] ;`, trims quote and brace delimiters, adds the
trimmed whole argument, sorts and deduplicates candidates, and classifies each
candidate as a library or complete interface symbol. Every prohibited
candidate becomes a `BuildLinkInput` finding with the input path, line, and
candidate text.

### `ArtifactSymbol`

`ArtifactSymbol` (lines 231-240) is a named undefined symbol extracted from a
native artifact. It owns only `name`, derives total ordering for deterministic
handling, and has no path normalization or validation in `new`. The aggregate
ELF validator rejects an empty name. `native.rs` gathers names from both dynamic
and regular ELF symbol tables, retains only undefined entries with nonempty
string-table names, deduplicates them in a `BTreeSet`, and constructs sorted
`ArtifactSymbol` values.

`artifact.rs::audit_elf_facts` classifies the complete name with
`classify_interface_symbol`. A prohibited name becomes an `UndefinedSymbol`
finding at line `0` with the original `name`; allowed and unknown names are
ignored. Policy decoration stripping therefore affects only the decision, not
the exact grant/report symbol.

### `ElfFacts`

`ElfFacts` (lines 242-259) is the complete model-side summary needed from one
ELF artifact:

- `path` is the normalized display path of the artifact;
- `needed` contains dynamic `DT_NEEDED` library names; and
- `undefined_symbols` contains `ArtifactSymbol` records from dynamic and
  regular symbol tables.

`ElfFacts::new` normalizes only `path` and moves both vectors. The native
collector supplies a relative path when an ELF is explicitly collected under
the scope. The standalone `read_elf_facts` API supplies a normalized full path.
Native extraction sorts and deduplicates `needed` and undefined names; direct
injection is allowed to preserve any order and duplicates, because the artifact
scanner and final audit still sort and deduplicate findings.

`AuditInput::validate` rejects duplicate ELF paths, empty paths, empty library
names, and empty artifact-symbol names. It does not check that a direct fact
actually names a file, that the path is in a scope, or that vector entries are
unique. `audit_elf_facts` emits `DynamicNeeded` findings for prohibited
libraries and `UndefinedSymbol` findings for prohibited symbols, each at line
`0`, using `elf.path` as the finding path.

## Aggregate input and validation

### `AuditInput`

`AuditInput` (lines 261-315) owns one complete, injected audit request:

| Field | Owner and use |
| --- | --- |
| `mode` | `AuditMode` that selects grant policy and is copied into the report. |
| `sources` | Explicit `SourceUnit` records scanned in source order, then globally sorted in the report. |
| `dependencies` | Optional `crate::DependencyGraph`. `None` omits dependency evidence; `Some` is semantically validated by `DependencyGraph::audit`. |
| `linker_inputs` | Declarative or command-line linker facts consumed by `audit_linker_inputs`. |
| `elf_facts` | Explicit artifact summaries consumed one at a time by `audit_elf_facts`. |
| `legacy_grants` | Candidate exact exceptions. They are rejected in next mode and validated/applied only in legacy mode. |

`AuditInput::empty(mode)` is a `const` constructor that keeps the chosen mode
and initializes empty source, linker, ELF, and grant vectors with no dependency
graph. It is useful for a caller that wants to add facts programmatically. The
CLI uses a struct literal so it can combine the native collection, optional
Cargo graph, linker arguments, and grants in one request.

`AuditInput::validate` is the first operation in `lib::audit` and must succeed
before any policy scanner runs. It performs these checks in order:

1. `validate_unique_paths` rejects duplicate source paths and duplicate ELF
   paths independently, using exact string keys in `BTreeSet`s. A source and
   an ELF may share a display path because they are different evidence sets.
   A duplicate returns `AuditError::Configuration` with the domain label and
   repeated path.
2. Every source path passes `validate_display_path`, which requires nonempty
   slash-normalized text.
3. Every linker input path passes the same check, and its argument must be
   nonempty.
4. Every ELF path passes the same check, and every needed library and artifact
   symbol name must be nonempty.

The validator does not classify values, reject prohibited values, inspect
source contents, validate `SourceKind`, validate linker line numbers, or
validate grants and dependency graphs. Those checks belong to their consumers:
dependency validation occurs in `DependencyGraph::audit`, lexical failures
occur in `audit_source`, and grant checks occur only in legacy-mode application.
This separation allows prohibited evidence to be represented as a valid input
and reported as a finding, while malformed evidence fails as `AuditError`
instead of being mistaken for a clean audit.

### Path helpers

`normalize_display_path` (line 370) is crate-visible and shared by model,
native, and dependency code. It replaces every backslash character with `/`;
it does not trim, canonicalize, collapse repeated separators, resolve `.` or
`..`, or enforce scope membership. It is idempotent and is applied by public
constructors for `Finding`, `LegacyGrant`, `SourceUnit`, `LinkerInput`, and
`ElfFacts`, as well as by native display-path creation and dependency package
construction.

`validate_display_path` accepts only a nonempty string already equal to its
slash-normalized form. It therefore rejects backslashes but permits absolute or
relative paths and otherwise arbitrary characters. The explicit validator is
used for source, linker, and ELF records. Grants use their stricter
`LegacyGrant::validate` check, and finding paths are normalized at emission.

### Model-owned error surfaces

| Check | Error and consequence |
| --- | --- |
| Unknown `AuditMode` text | `AuditError::Configuration`; no `AuditInput` is assembled by the CLI. |
| Duplicate source or ELF path | `AuditError::Configuration` naming the domain and path; no scanner runs. |
| Empty or unnormalized source, linker, or ELF path | `AuditError::Configuration`; the request is rejected before policy. |
| Empty linker argument, needed library, or artifact symbol | `AuditError::Configuration`; the corresponding evidence is not silently dropped. |
| Invalid legacy grant fields | `AuditError::Configuration` from `LegacyGrant::validate`; no report. |
| Duplicate or unused legacy grant key | `AuditError::Configuration` from `apply_legacy_grants`; no report. |
| Nonempty grants in next mode | `AuditError::Configuration`; no exception is applied. |

Lexical, dependency-graph, metadata, filesystem, and ELF parser failures use
the other `AuditError` variants in their owning modules and are propagated by
`lib::audit` without being converted into findings or a clean report.

## Report construction and serialization

### Trait and visibility matrix

The model intentionally exposes records but keeps orchestration helpers
crate-private:

| Type | Public surface and derived traits | Serde boundary |
| --- | --- | --- |
| `AuditMode` | Public enum, `Copy`, equality, `Display`, `FromStr` | Serialize only, `next`/`legacy` |
| `FindingCategory` | Public ordered enum, `Copy`, `Display` | Deserialize and Serialize, kebab-case labels |
| `FindingDisposition` | Public ordered enum, `Copy` | Serialize only, `blocking`/`grandfathered` |
| `Finding` | Public fields, public `blocking`, crate-private `key` | Serialize only |
| `LegacyGrant` | Public fields and `new`, crate-private `validate` and `key` | Deserialize and Serialize, unknown fields denied |
| `SourceKind` | Public ordered enum, `Copy` | No Serde implementation |
| `SourceUnit`, `LinkerInput`, `ArtifactSymbol`, `ElfFacts` | Public fields and side-effect-free `new` constructors | No Serde implementation |
| `AuditInput` | Public fields, public `empty`, crate-private `validate` | No Serde implementation |
| `AuditReport` | Public fields, public `blocking_count`, crate-private `new` | Serialize only |

The module itself remains private. `lib.rs` re-exports the listed public
types, while `normalize_display_path`, `validate_unique_paths`, and
`validate_display_path` remain unavailable to external callers.

### `AuditReport`

`AuditReport` (lines 341-368) is the deterministic public result. It derives
`Clone`, `Debug`, `Eq`, `PartialEq`, and `Serialize`; it is not deserializable.
Its fields are:

- `mode`, the selected `AuditMode`;
- `passed`, computed by the private `AuditReport::new`; and
- `findings`, the globally sorted and deduplicated findings after any legacy
  disposition changes. Sorting occurs before grants are applied; grant
  mutation changes only disposition and does not reorder the unique identities.

`AuditReport::new` sets `passed` to the `all` predicate that every finding has
`FindingDisposition::Grandfathered`. The predicate is vacuously true for an
empty finding set. Therefore a next-mode report passes exactly when no finding
exists, while a legacy report passes when every finding matched a valid exact
grant. A legacy report with no findings also passes with an empty grant list at
the library boundary; the CLI independently requires a grant-file option in
legacy mode.

`blocking_count` counts only findings whose disposition is `Blocking`. On
reports produced by `lib::audit`, `passed` is equivalent to a zero blocking
count because the disposition enum has only the two generated states. The
method does not cache, sort, mutate, or revalidate the vector.

Report ordering follows the derived `Finding` order: category declaration
order (`dependency`, `source-api`, `build-link-input`, `dynamic-needed`,
`llvm-declaration`, `llvm-call`, `undefined-symbol`, `runtime-load`, then
`disallowed-native-interface`), followed by path, line, symbol, and disposition.
Grant mutation occurs after this ordering, and unique identities are not
reordered.

The CLI serializes the report with `serde_json::to_string_pretty`. Enum values
therefore use the stable kebab-case strings, and each finding exposes
`category`, `path`, `line`, `symbol`, and `disposition`. Input evidence records
are not serialized by this model, and grants are input-only. The CLI exits `0`
for `passed`, `1` for a report containing blockers, and `2` for an `AuditError`
before a report exists.

The derived JSON shape is therefore structurally:

```json
{
  "mode": "legacy",
  "passed": true,
  "findings": [
    {
      "category": "source-api",
      "path": "src/legacy.rs",
      "line": 12,
      "symbol": "hipMalloc",
      "disposition": "grandfathered"
    }
  ]
}
```

The example shows the current field and enum names; the vector can be empty,
and a next-mode finding would normally retain `"blocking"` disposition.

## End-to-end model traces

### Source evidence

`collect_native_scope` maps a supported file to a `SourceKind`, reads UTF-8
contents, computes a relative slash-normalized display path, and calls
`SourceUnit::new`. `lib::audit` validates the path and uniqueness, then calls
`audit_source`. The lexer produces line-numbered tokens. Source identifiers,
strings, and LLVM `@` contexts map policy results to `FindingCategory` values,
and every emitted result enters through `Finding::blocking`. The source module
sorts and deduplicates its vector; the library repeats that operation across
all evidence domains.

### Dependency evidence

The CLI reads bounded Cargo metadata only when exact package IDs were supplied
and builds an optional `DependencyGraph`. The graph validates package IDs,
edge endpoints, duplicate records, and roots during its audit. It traverses the
reachable closure deterministically, classifies package names, and emits
`Dependency` findings with manifest paths and line `0`. `AuditInput` carries the
graph without copying or translating it. Manifest paths are dependency
metadata display values and are not required to lie under the native source
scope; the model only applies slash normalization.

### Linker evidence

The CLI turns each `--link-input` into a `LinkerInput` with the sentinel path
`<command-line>` and line `0`; source scanners independently create
`BuildLinkInput` findings for declarative link context. The artifact scanner
expands each argument into exact candidates, classifies libraries and complete
symbols, and emits findings with the candidate text. Input validation happens
before scanning, so an empty argument is an error rather than absent evidence.

### ELF evidence

The native collector or `read_elf_facts` parses an explicit artifact and builds
`ElfFacts` from sorted needed libraries and deduplicated undefined
`ArtifactSymbol`s. Aggregate validation rejects duplicate artifact paths and
empty names. The artifact scanner emits separate `DynamicNeeded` and
`UndefinedSymbol` findings, both line `0`, retaining the artifact display path
and original evidence spelling.

### Mode, grants, and report

After all producer vectors are combined, `lib::audit` sorts and deduplicates
the findings by their derived total order. Next mode requires no grants and
leaves every result blocking. Legacy mode validates exact grants, rejects
duplicates, marks key matches grandfathered, and rejects stale keys. Only then
does the private report constructor derive `passed`. This ordering ensures a
malformed fact, malformed grant, duplicate, or stale exception cannot appear as
a clean serialized report.

### Observable boundary cases

These cases follow directly from the constructors and validators:

| Input state | Result through `lib::audit` |
| --- | --- |
| Empty `AuditInput::empty(Next)` or `empty(Legacy)` with no grants | Empty report, `passed=true`, `blocking_count=0` (the CLI still requires its mode/scope options and requires a grant-file option for legacy mode). |
| One prohibited fact in next mode | One blocking finding and `passed=false`. |
| One prohibited fact plus one exact legacy grant | The finding becomes grandfathered, `passed=true`, and the grant is considered used. |
| Legacy grant with a wildcard, empty identity field, noncanonical path, duplicate key, or no matching finding | `AuditError::Configuration`; no report. |
| Duplicate source or ELF display path | `AuditError::Configuration` before policy scanning. |
| Empty linker argument, ELF library, or artifact symbol name | `AuditError::Configuration` before the corresponding scanner. |
| Whitespace-only linker or evidence strings | They are not empty by this model's checks; policy classification can yield no finding, so callers must provide meaningful evidence. |
| Unterminated source comment or literal | `AuditError::Lexical`; no report. |
| Malformed dependency graph or ELF collection | The graph/native error type is returned; no report. |

## Invariants and non-goals

- Finding identity is exactly `(category, normalized path, line, symbol)`;
  disposition is mutable policy state and is not part of a grant key.
- Generated source lines are one-based; graph, binary, and command-line facts
  use zero. Grants must use the same line convention.
- Generated display paths use `/`. The model does not canonicalize paths or
  prove that an injected path belongs to a filesystem scope.
- Prohibited values are valid evidence and produce findings. Empty structural
  values are invalid input and produce `AuditError` instead.
- Allowed and unknown policy classifications produce no finding. The model
  stores no classifier result or `NativeInterface` value.
- Legacy exceptions are exact and one-to-one with current findings. There are
  no wildcard, blanket path, category-family, or prefix grants.
- Ordered enums, derived finding order, and `BTreeMap`/`BTreeSet` grant and
  graph traversal keep report and stale-key ordering deterministic; no hash-map
  order or runtime timestamp enters the model.
- Collection, dependency parsing, lexing, policy classification, ELF parsing,
  report formatting, and exit-code selection remain outside this module.

Source anchors: [`audit/src/model.rs`](../../src/model.rs),
[`audit/src/lib.rs`](../../src/lib.rs), [`audit/src/source.rs`](../../src/source.rs),
[`audit/src/artifact.rs`](../../src/artifact.rs),
[`audit/src/dependency.rs`](../../src/dependency.rs),
[`audit/src/native.rs`](../../src/native.rs), and
[`audit/src/main.rs`](../../src/main.rs), with shared error variants in
[`audit/src/error.rs`](../../src/error.rs).
