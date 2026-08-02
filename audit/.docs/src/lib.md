# `audit/src/lib.rs`

Source: [`audit/src/lib.rs`](../../src/lib.rs)

```toml
[module]
path = "audit/src/lib.rs"
kind = "crate-facade-and-audit-orchestrator"
intent = "Evaluate explicit native-boundary evidence with one deterministic policy pass."
purpose = "Validate an AuditInput, collect findings from each evidence family, apply mode-specific legacy policy, and return an AuditReport."
structure = "Eight private modules behind one root facade, with policy and evidence helpers re-exported explicitly."
private_modules = ["artifact", "dependency", "error", "lexer", "model", "native", "policy", "source"]
state = "per-call"
orchestrator_side_effects = "none"
crate_collection_side_effects = "filesystem reads occur only through explicit native or CLI JSON helpers, not through audit()"

[boundary]
input = "AuditInput"
output = "Result<AuditReport, AuditError>"
accepted_evidence = ["source units", "optional Cargo dependency graph", "linker inputs", "ELF facts"]
policy_exception = "exact LegacyGrant values only in legacy mode"
self_host_rule = "audit_source skips only the exact display path audit/src/policy.rs"
host_discovery = "never implicit"

[boundaries]
collection = "native helpers and CLI only; audit() consumes injected facts"
policy = "policy module classifiers only; audit() combines findings but does not redefine families"
presentation = "CLI JSON and exit status only; audit() returns an in-memory report"
failure = "malformed aggregate evidence returns AuditError; prohibited valid evidence returns a blocking finding"

[flow]
steps = ["AuditInput::validate", "audit_source for every source", "DependencyGraph::audit when present", "audit_linker_inputs", "audit_elf_facts for every ELF", "sort and deduplicate findings", "reject or apply legacy grants", "AuditReport::new"]
ordering = "source, dependency, linker, ELF collection before one global sort/dedup"
determinism = "findings are ordered and deduplicated before the report is built"

[failure_policy]
malformed_input = "Err(AuditError), never a passing report"
blocking_evidence = "Ok(AuditReport { passed = false, ... })"
fully_granted_legacy_evidence = "Ok(AuditReport { passed = true, ... })"
error_variants = ["Configuration", "InvalidDependencyGraph", "InvalidCargoMetadata", "Lexical", "InvalidElf", "Io"]

[exports]
functions = ["audit", "audit_source", "audit_linker_inputs", "audit_elf_facts", "collect_native_scope", "read_elf_facts", "classify_interface_symbol", "classify_library"]
types = ["ArtifactSymbol", "AuditError", "AuditInput", "AuditMode", "AuditReport", "DependencyEdge", "DependencyGraph", "DependencyPackage", "ElfFacts", "Finding", "FindingCategory", "FindingDisposition", "InterfaceClassification", "LegacyGrant", "LinkerInput", "NativeCollection", "NativeInterface", "SourceKind", "SourceUnit"]
constants = ["CUDA_DRIVER_API_ALLOWLIST"]

[ownership]
input = "audit consumes AuditInput by value"
evidence = "delegated stages borrow injected facts and return new findings"
grant_mutation = "only local Finding.disposition values may change"
report = "AuditReport owns the final sorted finding vector"

[model]
finding_categories = ["dependency", "source-api", "build-link-input", "dynamic-needed", "llvm-declaration", "llvm-call", "undefined-symbol", "runtime-load", "disallowed-native-interface"]
finding_dispositions = ["blocking", "grandfathered"]
binary_or_graph_line = 0
```

## Intent and boundary

The library is the deterministic gate at Recipe's native-interface boundary. Its
central function, `audit(AuditInput)`, does not discover files, invoke Cargo,
read ELF files, run a linker, or consult the host. It consumes facts that a
caller has already selected or that the opt-in native collector has extracted.
This separation lets build systems and tests inject the same evidence without
changing policy evaluation.

The function owns the cross-source transaction: a malformed input or malformed
evidence source returns an error, while valid prohibited evidence becomes a
finding. It never turns missing evidence into proof of compliance. The
filesystem-owning functions `collect_native_scope` and `read_elf_facts` are
separate public helpers; the command-line binary decides when to call them.

## Module structure

All modules are private (`mod`, not `pub mod`). The crate's public surface is
the explicit re-export list below, plus `audit` itself. Each module owns one
evidence or policy concern:

| module | public items re-exported by the facade | responsibility and boundary |
| --- | --- | --- |
| `artifact` | `audit_linker_inputs`, `audit_elf_facts` | Convert linker arguments and injected ELF facts into findings. It classifies complete candidates and returns a vector, never an error. |
| `dependency` | `DependencyEdge`, `DependencyGraph`, `DependencyPackage` | Represent a caller-selected Cargo graph, parse Cargo metadata JSON, validate graph integrity, and audit the reachable dependency closure. Graph parsing and graph integrity failures are errors. |
| `error` | `AuditError` | Own every collection, configuration, lexical, metadata, graph, and ELF failure type. |
| `lexer` | none | Tokenize a `SourceUnit` while removing comments and tracking one-based lines. Unterminated comments or literals become an internal lexical error consumed by `source`. |
| `model` | `ArtifactSymbol`, `AuditInput`, `AuditMode`, `AuditReport`, `ElfFacts`, `Finding`, `FindingCategory`, `FindingDisposition`, `LegacyGrant`, `LinkerInput`, `SourceKind`, `SourceUnit` | Define the injected facts, stable finding identity, mode, grants, and report. It owns input validation and report pass computation. |
| `native` | `NativeCollection`, `collect_native_scope`, `read_elf_facts` | Opt-in filesystem collection. It chooses supported source kinds, reads UTF-8 text, and extracts ELF `DT_NEEDED` and undefined symbols. It does not evaluate policy. |
| `policy` | `CUDA_DRIVER_API_ALLOWLIST`, `InterfaceClassification`, `NativeInterface`, `classify_interface_symbol`, `classify_library` | Define the exact symbol, library, and dependency classification policy. `classify_dependency` remains crate-private for source and graph auditing. |
| `source` | `audit_source` | Lexically inspect one source or build unit, recognize include, link, and LLVM declaration/call context, and emit category-specific findings. |

The facade imports only `BTreeMap` and `BTreeSet` for legacy-grant indexing and
usage tracking. No global state, cache, thread, or mutable policy registry is
used.

Source anchors for this trace are stable within the current module layout:

| lines in `audit/src/lib.rs` | content |
| --- | --- |
| 8-15 | private module declarations |
| 19-30 | public re-exports |
| 32-41 | `audit` contract and error documentation |
| 42-80 | aggregate audit pipeline |
| 82-118 | exact legacy-grant application |

Delegated source anchors used by the flow are:

| source | relevant ranges |
| --- | --- |
| `audit/src/model.rs` | 263-315 for `AuditInput` and validation, 343-367 for `AuditReport` |
| `audit/src/source.rs` | 16-63 for the public source pass and LLVM seed findings, 66-107 for LLVM context, 110-168 for strings and classifier-to-category mapping |
| `audit/src/dependency.rs` | 61-146 for Cargo JSON parsing, 148-193 for reachable-closure audit, 195-247 for graph validation |
| `audit/src/artifact.rs` | 8-26 for linker candidates, 30-66 for ELF facts |
| `audit/src/native.rs` | 33-104 for scope collection, 106-166 for ELF extraction, 168-247 for path and source-kind rules |
| `audit/src/policy.rs` | 164-225 for symbol/library classification, 227-379 for exact family matchers and normalization |
| `audit/src/error.rs` | 5-22 for variants, 33-60 for display and error-source ownership |
| `audit/src/main.rs` | 20-97 for CLI-to-facade orchestration, 99-124 for bounded JSON input, 126-219 for argument grammar and usage |

## Crate build surface

`audit/Cargo.toml` exposes the library as `recipe_audit` and the binary as
`recipe-audit`, both from this crate's `src` tree. The library uses `goblin` for
ELF parsing and `serde`/`serde_json` for model and metadata/grant boundaries.
The crate forbids unsafe Rust and denies its configured Clippy `all` and
`pedantic` lint groups. None of those dependencies perform native policy
evaluation; they support fact collection or serialization at the edges
described below. It is a standalone workspace member with no dependency on
Recipe's runtime crates, so the audit gate can inspect those crates' source,
metadata, link inputs, and artifacts without importing their implementation.

## Public facade

The following items are reachable from `recipe_audit` without exposing module
paths:

| item | input and result | owner of behavior |
| --- | --- | --- |
| `audit` | `AuditInput -> Result<AuditReport, AuditError>` | This file; the complete orchestration described below. |
| `audit_source` | `&SourceUnit -> Result<Vec<Finding>, AuditError>` | `source`; lexical failures abort, policy hits are returned as findings. |
| `audit_linker_inputs` | `&[LinkerInput] -> Vec<Finding>` | `artifact`; scans exact linker candidates. |
| `audit_elf_facts` | `&ElfFacts -> Vec<Finding>` | `artifact`; scans needed libraries and undefined symbols. |
| `collect_native_scope` | `(&Path, &[PathBuf]) -> Result<NativeCollection, AuditError>` | `native`; explicit absolute scope and explicit ELF paths only. |
| `read_elf_facts` | `&Path -> Result<ElfFacts, AuditError>` | `native`; reads one absolute regular ELF file. |
| `classify_interface_symbol` | `&str -> InterfaceClassification` | `policy`; classifies one complete symbol after decoration stripping. |
| `classify_library` | `&str -> InterfaceClassification` | `policy`; classifies one normalized library basename. |
| `CUDA_DRIVER_API_ALLOWLIST` | `&[&str]` | `policy`; reviewed exact CUDA Driver symbol spellings, including versioned spellings. |
| dependency types and constructors | `DependencyPackage`, `DependencyEdge`, `DependencyGraph`, including `from_cargo_metadata_json` | `dependency`; callers choose exact roots and supply the complete graph closure. |
| model types and public constructors | `AuditMode`, `Finding`, `LegacyGrant`, `SourceUnit`, `LinkerInput`, `ArtifactSymbol`, `ElfFacts`, `AuditInput`, `AuditReport` | `model`; public constructors normalize display paths where documented, `AuditInput::empty` creates an empty per-mode input, and `AuditReport` has no public constructor. |
| `AuditError` | error enum implementing `Display` and `Error` | `error`; errors are terminal for the current audit call. |

The helper functions are intentionally usable independently, but only
`audit` combines their outputs with input validation, sorting, deduplication,
and grant semantics. The pure classifier functions return `Unknown` for
unrecognized complete names rather than guessing from arbitrary substrings.

The public constructors and queries preserve that same boundary:

| public method | contract |
| --- | --- |
| `AuditMode::as_str`, `FindingCategory::as_str`, `Display`, `FromStr` | Convert stable mode/category spellings (`Display` is implemented for mode and category; `FromStr` is implemented for mode). Mode parsing accepts exactly `next` and `legacy`; unknown text is `AuditError::Configuration`. |
| `InterfaceClassification::is_prohibited` | A pure predicate over one classification. `Allowed` and `Unknown` are non-prohibited. |
| `Finding::blocking` | Build one blocking finding and slash-normalize its display path. It does not validate that the symbol or path is nonempty. |
| `LegacyGrant::new` | Build one exact grant and slash-normalize its path. Full grant validation remains private and occurs only when legacy policy is applied. |
| `SourceUnit::new`, `LinkerInput::new`, `ArtifactSymbol::new`, `ElfFacts::new` | Build injected evidence records. The path-bearing constructors normalize paths; symbol and argument content is retained verbatim for later validation/classification. |
| `AuditInput::empty` | Create an empty input for a selected mode. It does not imply that a native scope or dependency graph was collected. |
| `AuditReport::blocking_count` | Count current `Blocking` findings without recomputing or changing `passed`. |
| `DependencyPackage::new`, `DependencyEdge::new`, `DependencyGraph::new` | Assemble an injected graph. Graph integrity is checked when metadata is parsed or when `audit` reaches the optional graph stage. |
| `DependencyGraph::from_cargo_metadata_json` | Parse Cargo metadata format version 1 and require exact caller-supplied root IDs and a complete `resolve.nodes` graph. |

## `audit` orchestration

The implementation in `audit/src/lib.rs:42-80` is a single ordered pipeline:

1. `input.validate()?` checks the structural shape of the complete input before
   any evidence is evaluated. It rejects duplicate source or ELF paths,
   empty or non-normalized display paths, empty linker arguments, and empty ELF
   library or symbol facts. It intentionally leaves dependency-graph and grant
   validation to the stage that owns each concern.
2. The input is destructured and consumed. The mode, vectors, optional graph,
   and grants become local values. No caller-owned object is mutated.
3. Every `SourceUnit` is passed to `audit_source`. Findings are appended in
   input order. A lexical error stops the pipeline immediately; prohibited
   tokens do not stop collection.
4. When `dependencies` is `Some`, `DependencyGraph::audit` validates the graph,
   traverses from the exact root package IDs, and appends one finding for each
   reachable prohibited package. An invalid graph stops the pipeline.
5. `audit_linker_inputs` scans all declarative or command-line linker inputs
   and appends `BuildLinkInput` findings.
6. Each injected `ElfFacts` value is passed to `audit_elf_facts`, which appends
   `DynamicNeeded` and `UndefinedSymbol` findings.
7. The complete finding vector is sorted using the derived stable ordering and
   deduplicated. The derived `Finding` order compares category, path, line,
   symbol, and disposition in struct-field order. At this point every finding
   is `Blocking`, so deduplication removes repeated evidence with the same
   category, path, line, and symbol. Thus source, graph, linker, and ELF input
   order cannot change a successful report's finding order or duplicate count.
8. Mode policy is applied. `next` rejects any nonempty grant vector. `legacy`
   validates and indexes every grant, changes exact matching findings to
   `Grandfathered`, and rejects every unused or stale grant.
9. `AuditReport::new` records the selected mode, the final findings, and
   `passed = true` only when every finding is `Grandfathered`. With no findings,
   `all(...)` is true, so an empty valid input passes in either mode; in next
   mode any finding remains blocking and therefore fails.

The `?` operators make errors fail closed. There is no partial report, retry,
alternate collector, or fallback policy path.

The same pipeline in implementation-shaped pseudocode is:

```text
validate(input) or return Err
move mode, sources, graph, linker_inputs, elf_facts, grants out of input
findings = []
for source in sources: findings += audit_source(source) or return Err
if graph exists: findings += graph.audit() or return Err
findings += audit_linker_inputs(linker_inputs)
for elf in elf_facts: findings += audit_elf_facts(elf)
sort(findings); deduplicate(findings)
if mode is next and grants are nonempty: return configuration Err
if mode is legacy: apply_exact_grants(findings, grants) or return configuration Err
return AuditReport::new(mode, findings)
```

The pseudocode is intentionally a data-flow description, not a second
implementation. Every arrow either transfers ownership into a local stage or
returns the stage's error; no stage can inject a substitute value after a
failure.

### Determinism proof by stage

For successful inputs, every source of ordering is bounded:

1. The native collector sorts filesystem paths and ELF display paths. A
   programmatic caller may supply a different order, but each source, linker,
   and ELF helper sorts its own findings.
2. Dependency traversal uses a `BTreeMap` adjacency, sorted dependency lists,
   a `BTreeSet` visited set, and a final finding sort. Cycles and root-order
   changes therefore cannot change the emitted order.
3. The facade performs one final `Finding::sort` and `dedup` across all evidence
   kinds, using the derived field order.
4. Valid legacy grants are indexed and stale-key diagnostics are emitted
   through `BTreeMap`/`BTreeSet`, so exact grant input order cannot change the
   report or stale-key error ordering. Malformed grants still fail at their
   first validation point, as covered by the error-precedence rule.

The only order-sensitive result is which malformed condition is reported first
when a caller supplies multiple invalid facts. That precedence is the explicit
pipeline order above, not an accidental dependence of a passing report.

## Security and trust boundary

The evaluator treats path, graph, linker, and ELF values as explicit evidence,
not as instructions. `audit()` never follows a path, opens a file, executes a
candidate, expands a glob, invokes Cargo, or loads a native library. The native
collector narrows its own trust boundary with absolute paths, canonical scope
containment, regular-file checks, symlink rejection, and bounded reads before
facts are handed to the evaluator. The CLI's JSON reader is separately bounded
and explicit, but does not inherit native scope containment.

Malformed aggregate facts fail closed; a prohibited complete name is a blocking
observation, while an unknown complete name is simply outside the policy
dictionary. There is no ambient allowlist,
environment override, path blanket, retry, or substitute evidence source.
Legacy grants are the one intentional exception and remain exact, typed, and
usage-checked.

## Input and model flow

`AuditInput` is the only aggregate consumed by `audit`:

| field | meaning at this boundary | validation or downstream owner |
| --- | --- | --- |
| `mode` | `AuditMode::Next` means no exceptions; `AuditMode::Legacy` enables exact grants. | The final mode branch in this file. `AuditMode::FromStr` accepts only `next` and `legacy`. |
| `sources` | Explicit UTF-8 `SourceUnit` values with a `SourceKind` of Rust, Zig, C, C++, LLVM IR, or build metadata. | `AuditInput::validate`, then `audit_source`; source paths must be unique and slash-normalized, but the library does not require them to be relative or canonical. |
| `dependencies` | Optional `DependencyGraph` with explicit root package IDs and directed package edges. | `DependencyGraph::audit`; absence means no dependency evidence is evaluated. |
| `linker_inputs` | Explicit `LinkerInput` records containing a display path, line, and full argument. | `AuditInput::validate`, then `audit_linker_inputs`; a nonempty argument is required. |
| `elf_facts` | Explicit `ElfFacts` records containing needed library names and undefined `ArtifactSymbol` values. | `AuditInput::validate`, then `audit_elf_facts`; paths are unique and all fact names are nonempty. |
| `legacy_grants` | Exact `(category, path, line, symbol)` exceptions. | Next mode rejects any vector; legacy mode validates, deduplicates, applies, and requires every grant to match a finding. |

Path constructors (`DependencyPackage::new`, `SourceUnit::new`,
`LinkerInput::new`, `ElfFacts::new`, and `LegacyGrant::new`) normalize
backslashes to `/`. Public fields remain writable,
so `AuditInput::validate` checks the resulting strings again. `Finding::blocking`
also normalizes its path. A finding line is one-based for source evidence and
zero for graph or binary evidence. Dependency graph validation requires a
nonempty manifest path but does not require it to be normalized, absolute, or
present on disk; the later `Finding::blocking` call supplies slash normalization
for the emitted dependency finding.

Ownership of the model across one call is linear:

| phase | owner and mutability |
| --- | --- |
| call entry | caller owns an `AuditInput`; `audit` receives it by value |
| validation | `validate` borrows the input immutably and returns only an error or permission to continue |
| post-validation | destructuring moves every field into local variables; the original aggregate no longer exists |
| evidence stages | source units, graph, linker records, and ELF records are borrowed immutably; each stage returns new finding values |
| grant stage | grants are borrowed immutably; only the local finding slice receives disposition mutations |
| report | `AuditReport` owns the final finding vector and mode; the caller receives it on `Ok` |

This ownership boundary prevents a helper from changing source text, graph
edges, linker arguments, ELF facts, or grant declarations while an audit is in
progress.

## Policy and finding flow

All interface and library policy decisions use the classifiers in `policy`;
their allowlists do not search arbitrary substrings. Source context detection
does intentionally look for explicit link/include markers on the current line.
ROCr/HSA and only the exact reviewed CUDA Driver
API allowlist are allowed. HIP, the CUDA Runtime API, direct KFD ownership, and
the listed AMD/NVIDIA operation libraries are prohibited. A Driver-shaped
`cuXxx` symbol outside the allowlist is classified as
`CudaDriverOutsideAllowlist` and remains prohibited.

Classification is deliberately boundary-aware:

| classifier | normalization and exact rules |
| --- | --- |
| `classify_interface_symbol` | Removes one leading symbol-decoration byte and any `@version` suffix. `hsa_...` is allowed ROCr/HSA; `hsaKmt...` and `kfd_...` are prohibited direct KFD. Exact entries in `CUDA_DRIVER_API_ALLOWLIST` are allowed CUDA Driver. A `cu` plus uppercase-letter shape outside that list is prohibited, as are identifier-boundary HIP, CUDA Runtime, and operation-family names. |
| `classify_library` | Trims whitespace and quote/bracket wrappers, keeps the basename after `/` or `\\`, removes `-l`, `/DEFAULTLIB:` when that token is still present, a `lib` prefix, known ABI suffixes (`.so`, `.dylib`, `.dll`, `.a`, `.lib`), and `-static`. The linker consumer separately splits `:` so common `/DEFAULTLIB:lib...` input still reaches a normal library candidate. It then matches exact CUDA Driver/HSA names, KFD names, HIP/runtime families, or named operation-library variants. A named family accepts an empty suffix, `_static`, `-static`, `lt`, `lt_static`, `64_` followed by digits, or `_` followed by digits. |
| crate-private `classify_dependency` | Lowercases and changes `_` to `-`, accepts only `cuda-driver`, `cuda-driver-sys`, `hsa`, `hsa-sys`, `rocr`, or `rocr-sys` as allowed package names, and matches explicit `-sys`, `-src`, `-bindings`, `-runtime`, `-static`, or `-wrapper` family suffixes for prohibited interfaces. |

Identifier-family checks require an empty suffix or a boundary marker such as
an uppercase letter, underscore, or digit for operation symbols, or (for
dependency names) an approved package suffix. A name that merely contains a
policy word inside a larger unrelated identifier remains `Unknown` and does
not create a finding.

`NativeInterface` records the reason for a non-unknown classification. The
allowed variants are `RocrHsa` and `CudaDriver`. The prohibited variants are
`Hip`, `CudaRuntime`, `DirectKfd`, `RocBlas`, `RocSolver`, `RocFft`, `MiOpen`,
`Rccl`, `CuBlas`, `CuSolver`, `CuFft`, `CuDnn`, `Nccl`, and
`CudaDriverOutsideAllowlist`. The reason is not serialized into a finding; the
finding category is the stable evidence location described in the next table.

The evidence family determines the finding category:

| evidence path | classifier/context | category and line |
| --- | --- | --- |
| source identifier | `classify_interface_symbol`; build metadata falls back to `classify_dependency` | `SourceApi` for source languages, `BuildLinkInput` for build metadata, or `DisallowedNativeInterface` for a prohibited outside-allowlist Driver symbol or direct KFD symbol |
| source string | library, symbol, or build-dependency classification; include and link markers are recognized from the same source line | `BuildLinkInput` in link context, `SourceApi` in include context, otherwise `RuntimeLoad` |
| LLVM `@symbol` declaration | same symbol classifier, with same-line `declare` recognition | `LlvmDeclaration` |
| LLVM `@symbol` call | same symbol classifier, with same-line `call`, `invoke`, or `callbr` recognition | `LlvmCall` |
| reachable dependency package | `classify_dependency(package.name)` | `Dependency`, line `0`, package manifest path |
| linker argument | `classify_library` and `classify_interface_symbol` on complete extracted candidates | `BuildLinkInput` |
| ELF `DT_NEEDED` library | `classify_library` | `DynamicNeeded`, line `0` |
| ELF undefined symbol | `classify_interface_symbol` | `UndefinedSymbol`, line `0` |

The category is evidence-location data, not an interface-family label. For
example, a prohibited HIP library in a linker argument is `BuildLinkInput`, the
same library in `DT_NEEDED` is `DynamicNeeded`, and a HIP identifier in a Rust
source unit is `SourceApi`. `NativeInterface` explains why the classifier
matched; `FindingCategory` explains where the evidence was observed. This
separation lets one exact legacy grant target one observation without granting
the same interface everywhere.

`DisallowedNativeInterface` is the deliberate precedence exception in
`push_interface_finding`: a prohibited `CudaDriverOutsideAllowlist` or
`DirectKfd` identifier takes that category even when its normal source or
build context would otherwise be `SourceApi`, `BuildLinkInput`, or an LLVM
category. String evidence uses its context category because `audit_string`
does not call this identifier-category mapping.

`audit_source` has one intentional self-hosting boundary: the exact source path
`audit/src/policy.rs` returns no source findings because that file contains the
policy dictionary itself, not a runtime use of the prohibited interface. Its
dependency, linker, and ELF evidence is still audited through the other stages.
The comparison uses the caller-supplied display path, so a native collection
root that makes the file display as `src/policy.rs` does not trigger this
exception.

## Legacy grant transaction

`apply_legacy_grants` is private to this facade and is the only code that can
change a finding's disposition. It builds a `BTreeMap` keyed by
`(FindingCategory, normalized path, line, symbol)`:

1. Each grant must have a nonempty exact path and symbol, no `*` or `?` in its
   path, no `*` in its symbol, and a path already normalized to slash form.
2. Inserting a second identical key returns a configuration error.
3. Each finding whose key is present is changed from `Blocking` to
   `Grandfathered`; matching keys are recorded as used.
4. Any indexed key not used by a finding produces one deterministic
   `unused or stale legacy grants` configuration error.

The match excludes the finding disposition, so a grant can only excuse a
finding with the exact category, path, line, and symbol supplied by the caller.
There is no wildcard, path-prefix, blanket, or unused-grant forgiveness. The
CLI additionally requires a grants JSON file whenever it selects legacy mode,
but the library itself accepts an empty grant vector and reports any resulting
findings as blocking.

Grant JSON is deserialized into `LegacyGrant` with kebab-case category names
and `deny_unknown_fields`. Deserialization therefore rejects misspelled or
extra fields before `apply_legacy_grants`; structural grant validation then
rejects empty values, wildcards, and unnormalized paths. This two-step boundary
keeps parsing errors and policy-use errors in the configuration domain.

## Failure ownership

Errors are owned by the narrowest boundary that can establish the failure:

| boundary | failure evidence | result |
| --- | --- | --- |
| `AuditInput::validate` | duplicate source/ELF path, empty or non-normalized path, empty linker argument, empty ELF library or symbol | `AuditError::Configuration` from `audit` before any policy stage |
| `audit_source` and `lexer` | unterminated block comment, string, character, or Rust raw string | `AuditError::Lexical { path, line, reason }` from `audit` |
| `DependencyGraph::from_cargo_metadata_json` | malformed JSON, wrong metadata version, missing arrays/fields, duplicate resolve nodes, or missing resolve graph | `AuditError::InvalidCargoMetadata` before a graph can be stored |
| `DependencyGraph::audit` | empty graph roots, duplicate package IDs, missing edge endpoints, or duplicate edges | `AuditError::InvalidDependencyGraph` while the optional graph stage runs |
| `collect_native_scope` | relative paths, symlink scope/source/artifact paths, scope escapes, unsupported filesystem objects, source-count or size limits, I/O, invalid UTF-8, or malformed ELF | `AuditError::Configuration`, `AuditError::Io`, or `AuditError::InvalidElf` before facts reach `audit` |
| `read_elf_facts` | relative path, missing/non-regular/oversized/unreadable file, or malformed ELF | `AuditError::Configuration`, `AuditError::Io`, or `AuditError::InvalidElf`; this direct helper does not have a scope argument and therefore does not enforce the collector's scope containment or symlink rejection. |
| mode branch and `apply_legacy_grants` | grants in next mode, malformed or duplicate grants, or unused/stale grants | `AuditError::Configuration` and no report |
| valid prohibited evidence | policy match with complete facts | `Ok(AuditReport)` containing `Blocking` findings, not an error |

The public pure helpers do not perform aggregate validation. A caller that
invokes `audit_linker_inputs`, `audit_elf_facts`, or a classifier directly gets
their vector/classification result; aggregate path and fact invariants are
enforced when the same values enter `audit`.

`AuditError` keeps error rendering human-readable at the outer boundary:
configuration errors begin with `invalid audit configuration`, graph and Cargo
metadata errors identify their respective domains, lexical errors include
`path:line`, invalid ELF errors include the path and parser reason, and I/O
errors preserve the operating-system source error. Only `Io` exposes an
underlying error through `Error::source`; the other variants are policy or
input descriptions.

For a single `audit` call, the first failing stage owns the result. The
precedence is structural input validation, source lexical audit in source-vector
order, optional dependency graph validation and traversal, linker and ELF fact
collection, then mode/grant policy. Linker and ELF helpers themselves do not
return errors, so once aggregate validation succeeds they cannot supersede a
later mode error. The CLI adds an earlier parse, scope, metadata-file, and grant
JSON boundary before it constructs `AuditInput`.

## Evidence normalization owned by the delegated stages

The facade does not normalize evidence itself after `AuditInput::validate`.
Each delegated stage has a narrow, observable normalization contract:

- `lexer::lex` removes line comments, nested block comments, LLVM semicolon
  comments, and build-metadata hash comments. It emits identifiers, strings,
  and `@` markers with one-based source lines. Rust lifetimes, ordinary
  character literals, and Rust raw strings are distinguished before policy
  inspection. Any unterminated literal or comment is lexical evidence failure,
  not an empty token stream. Apostrophes in `BuildMetadata` are punctuation,
  not character delimiters, so identifiers between them remain visible to the
  build scanner.
- `source::audit_source` inspects complete identifiers and strings. LLVM
  `@name` tokens are classified only when same-line declaration or call words
  (`declare`, `call`, `invoke`, or `callbr`) establish the category. Strings
  use same-line link markers (`rustc-link-lib`, `rustc-link-arg`, `#[link`,
  `target_link_libraries`, `linkSystemLibrary`, or `-l`) and include markers
  (`#include`, `@import`, or `@cImport`) to distinguish build, source, and
  runtime evidence. The LLVM `@` pairing accepts the next lexical token even
  across a line break, while the declaration/call words are searched only on
  the `@` token's line; this is a lexical implementation boundary, not a
  syntax parser guarantee. String policy trims surrounding whitespace and
  trailing `\\00` or `\\0` escape text before classifying the value.
- `artifact::audit_linker_inputs` examines both the complete argument and
  candidates split on ASCII whitespace and `,`, `=`, `:`, `(`, `)`, `[`, `]`,
  or `;`. It trims quote and brace wrappers, sorts candidates, and deduplicates
  them before classification.
  A single argument can therefore produce more than one finding, while a
  repeated candidate produces one finding.
- `dependency::audit` builds sorted, duplicate-free adjacency lists and walks
  from exactly the supplied root IDs. It records only reachable prohibited
  package names, using the package manifest path as the finding path and line
  zero as the no-text-location marker.
- `native::read_elf_facts` sorts and deduplicates `DT_NEEDED` names and combines
  undefined dynamic and static symbols into one sorted set. `artifact` then
  classifies those names without reopening the file.

These transformations happen before the facade's final global sort and
deduplication. They do not change the caller's vectors or mutate the source,
graph, linker, or ELF records.

## Report and serialization contract

`AuditReport` exposes `mode`, `passed`, and `findings`; it derives `Clone`,
`Debug`, `Eq`, `PartialEq`, and `Serialize` but not `Deserialize`. `AuditMode`,
`FindingCategory`, and `FindingDisposition` serialize in kebab-case, so the
JSON emitted by the CLI uses values such as `next`, `source-api`,
`undefined-symbol`, `blocking`, and `grandfathered`. A `Finding` has the stable
fields `category`, `path`, `line`, `symbol`, and `disposition` in that source
order. The report does not include raw source text, dependency edges, linker
arguments, ELF bytes, policy classifications, or grant usage details.

`Finding` derives `Clone`, `Debug`, `Eq`, `Ord`, `PartialEq`, `PartialOrd`, and
`Serialize`, but not `Deserialize`; `FindingDisposition` is therefore report
state rather than a caller-deserializable input field. `FindingCategory` and
`LegacyGrant` are the shared category vocabulary used for grant JSON.

The input aggregate and evidence records are not a general JSON schema:
`AuditInput`, `SourceUnit`, `LinkerInput`, `ElfFacts`, `ArtifactSymbol`, and the
dependency graph types are programmatic values, while `LegacyGrant` is the
only input record with a derive-based JSON deserialization contract. Report
serialization is one-way through `AuditReport` and its finding types; callers
should not infer that a serialized report can be fed back as an `AuditInput`.

`AuditReport::blocking_count` counts only findings whose disposition is
`Blocking`; it is a derived query and does not alter `passed`. The report is
computed only inside the model module's private `new` path. Its fields are
public for result inspection and serialization, so a caller could write a
struct literal, but that literal does not participate in the audit gate or gain
any validation. Reports returned by `audit` are the authoritative evaluated
results.

## Native and CLI boundary

The binary in `audit/src/main.rs` is the opt-in collector and presentation
layer around this facade:

1. `Cli::parse` accepts explicit `--mode`, absolute `--scope`, optional exact
   `--metadata` plus one or more `--package-id`, `--link-input`, `--elf`, and
   legacy `--legacy-grants` paths. It rejects duplicate singleton options and
   empty package IDs or linker inputs. Each option and its value are separate
   arguments; `--link-input=value` and analogous equals forms are unknown
   arguments because parsing matches the option token exactly.
2. `run` requires mode and scope, calls `collect_native_scope`, optionally
   parses bounded Cargo metadata and exact roots, and loads bounded legacy JSON
   grants according to the selected mode.
3. Each command-line linker argument becomes `LinkerInput::new("<command-line>",
   0, argument)`. Collected sources and ELF facts are placed into one
   `AuditInput`, then passed to `recipe_audit::audit`.
4. The report is pretty-printed as JSON. Exit code `0` means `report.passed`,
   exit code `1` means a valid report with blocking findings, and exit code `2`
   means an `AuditError` or CLI/configuration failure.

The metadata and grant JSON reader requires an absolute regular file no larger
than 64 MiB and reads UTF-8 text. It is separate from the native collector's
16 MiB source-file and 1 GiB ELF bounds.

`collect_native_scope` requires an absolute real directory, rejects symlinks,
skips `.git` and `target` while walking supported source/build extensions, and
requires explicitly named ELF paths to be real files under the canonical
scope. It bounds each collected text file at 16 MiB, the complete source set at
100,000 files, and each ELF at 1 GiB. `read_elf_facts` shares the ELF size and
format checks but, as a direct helper, follows the filesystem metadata rules
for its one absolute path rather than applying a caller scope. The direct
helper keeps a normalized absolute display path in `ElfFacts`; the collector
uses a normalized path relative to its canonical scope. This collection policy
is outside `audit`; callers using injected facts can choose a different fact
source while preserving the same evaluation.

The skip rule applies to child directory entries. Selecting a directory whose
own basename is `target` as the explicit scope still traverses that scope, so
an ELF beneath it can be supplied through `--elf` and audited.

The collector sorts discovered filesystem paths before reading them and sorts
the resulting ELF facts by their slash-normalized display path. The evaluator
still performs its own final finding sort, so injected callers do not need to
use the collector's order to obtain a deterministic report.

Collection recognizes `.rs`, `.zig`, `.c`/`.h`, C++ header and source
extensions, `.ll`, and build metadata extensions (`.toml`, `.json`, `.yaml`,
`.yml`, `.cmake`, `.mk`, `.ninja`, `.bazel`, `.bzl`) plus named files such as
`Cargo.lock`, `Makefile`, `CMakeLists.txt`, `BUILD`, and `WORKSPACE`. Other
regular files are ignored, but a symlink anywhere in the walked scope is an
error even when its target would have an unsupported extension.

The collector has no aggregate byte bound, ELF-count bound, recursion-depth
bound, file hashing, locking, retry, or atomic filesystem snapshot. It reads
accepted text and ELF bytes completely before policy evaluation. Goblin parsing
accepts any supported ELF class and byte order; the collector does not impose a
machine, architecture, executable-bit, ABI, or operating-system filter. Those
facts are outside the policy boundary and are not inferred by `audit`.

## Observable edge cases

These cases follow directly from the branch ordering and are part of the
facade's contract:

| supplied state | result |
| --- | --- |
| no sources, graph, linker inputs, ELF facts, or grants | an empty valid input returns a passing report in either mode |
| whitespace-only linker argument | accepted by aggregate validation, then produces no linker finding |
| duplicate source or ELF display path | configuration error before source or artifact policy runs |
| identical linker records or repeated ELF names | matching helper findings are globally sorted and deduplicated; there is no duplicate-input configuration error for these fields |
| unknown symbol or library classification | no finding; absence of a policy match is not itself an error |
| dependency graph cycle | valid graph; the visited set terminates traversal and each reachable package is considered once |
| duplicate edge supplied through Cargo JSON | parser's `BTreeSet` removes the repeated edge before graph validation |
| duplicate edge supplied directly to `DependencyGraph::new` | graph validation reports `InvalidDependencyGraph` |
| a lexical error and a malformed later graph or grant | lexical error wins because sources are audited before optional graph and mode stages |
| prohibited finding with no matching legacy grant | valid legacy report with a blocking finding and `passed = false` |
| valid legacy grant with no matching finding | configuration error for an unused or stale grant, not a passing report |
| malformed legacy grant in next mode | mode configuration error because any nonempty grant vector is rejected before grant validation |
| direct helper called with malformed aggregate paths | helper behavior is returned without aggregate validation; `audit` is the boundary that enforces `AuditInput` invariants |

The table distinguishes errors that describe unusable evidence from reports that
describe usable evidence which violates policy. This distinction is maintained
even when the caller supplies no evidence at all.

Dependency metadata has its own exactness rules: numeric JSON version `1` is
required (`1.0` or string `"1"` is not accepted), package and node IDs are
case-sensitive nonempty strings, unknown top-level JSON fields are ignored,
duplicate node IDs are metadata errors, and duplicate package IDs or missing
edge endpoints are graph errors. Root IDs are caller-supplied and may repeat;
the traversal's visited set collapses them. Disconnected packages, even when
prohibited, never enter the reachable closure and therefore never produce a
finding.

The caller can choose one of two complete boundary patterns without changing
the evaluator:

| pattern | preparation | evaluation |
| --- | --- | --- |
| injected facts | construct `AuditInput` and its records directly, optionally parse a `DependencyGraph` from a caller-provided JSON string, and supply any ELF facts obtained elsewhere | call `recipe_audit::audit`; no host access occurs in this path |
| CLI collection | parse explicit arguments, call `collect_native_scope` for source and selected ELF paths, optionally load bounded Cargo metadata and grants, and wrap link arguments as `LinkerInput` | call the same `audit`; JSON and exit-code presentation happen after the report |

The two patterns share the model and policy stages. Collection differences do
not create a second policy implementation, and the CLI does not inspect the
report to infer additional findings.

The current call graph is therefore one converging path:

```text
recipe-audit::main::run
  -> collect_native_scope / read_bounded_text / LinkerInput::new
  -> AuditInput
  -> recipe_audit::audit
       -> AuditInput::validate
       -> source::audit_source -> lexer + policy
       -> dependency::DependencyGraph::audit -> policy
       -> artifact::audit_linker_inputs -> policy
       -> artifact::audit_elf_facts -> policy
       -> apply_legacy_grants (legacy only)
       -> model::AuditReport::new
  -> serde_json report + exit code
```

The library can be entered below the CLI at any public helper, but only the
aggregate path above creates a complete report. No helper calls back
into the CLI or into native collection.

For delegated implementation detail, follow [`artifact.rs`](artifact.md),
[`dependency.rs`](dependency.md), [`error.rs`](error.md),
[`lexer.rs`](lexer.md), [`model.rs`](model.md), [`native.rs`](native.md),
[`policy.rs`](policy.md), [`source.rs`](source.md), and the CLI trace in
[`main.rs`](main.md). This file remains the ownership and convergence map for
those modules.

## Non-responsibilities and invariants

- `audit` does not choose a current directory, infer a scope, run
  `cargo metadata`, or inspect unlisted artifacts.
- Policy dictionaries are not legacy exceptions. The sole source-level
  self-hosting skip for the exact `audit/src/policy.rs` display path is a
  collection boundary; only an exact legacy grant can change an emitted
  finding's disposition, and only after the finding exists.
- Discovery, lexical collection, ELF parsing, dependency parsing, and policy
  evaluation remain separate stages. An error in one stage is not converted
  into a warning or a guessed fact for another stage.
- Findings are stable records of category, slash-normalized path, line, symbol,
  and disposition. The global sort and dedup make equivalent injected evidence
  produce the same report independent of input ordering.
- A report is a result of valid evidence. `passed = false` is a policy result;
  malformed or incomplete evidence is represented by `Err(AuditError)` instead.
