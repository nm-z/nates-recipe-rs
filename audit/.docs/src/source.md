# `audit/src/source.rs`

## Intent

`source.rs` is the source-text policy adapter for the `recipe-audit` gate.  It
accepts one already collected [`SourceUnit`](../../src/model.rs), lexes the
UTF-8 contents with the language mode carried by that unit, classifies complete
identifiers and strings through `policy.rs`, and returns deterministic blocking
`Finding` values.  It is a lexical gate, not a Rust, C, C++, Zig, LLVM, TOML,
JSON, YAML, CMake, Make, Ninja, or Bazel parser.

The module has one public operation:

```text
audit_source(source: &SourceUnit) -> Result<Vec<Finding>, AuditError>
```

### Contract summary

```text
module: audit/src/source.rs
purpose: lex one SourceUnit and classify prohibited interface/library evidence
input: SourceUnit { path, SourceKind, UTF-8 contents }
output: sorted, exact-deduplicated Vec<Finding>, all disposition=blocking
errors: AuditError::Lexical for an unterminated comment or literal
filesystem-side-effects: none
policy-side-effects: none
report-status: not decided here
```

The output/error grammar is:

```text
SourceAuditResult = Ok(Finding*) | Err(AuditError::Lexical)
Finding = {
    category: SOURCE_API | BUILD_LINK_INPUT | LLVM_DECLARATION | LLVM_CALL
              | RUNTIME_LOAD | DISALLOWED_NATIVE_INTERFACE,
    path: slash_normalized_display_path,
    line: ONE_BASED_LEXER_LINE,
    symbol: exact_identifier_or_trimmed_string,
    disposition: BLOCKING,
}
```

`SourceAuditResult` is descriptive. The Rust enum and struct definitions remain
the implementation authority, and `Finding` is serialized only by the enclosing
report path.

The operation does not read paths, discover files, inspect dependencies, parse
ELF, apply legacy grants, or decide the final report status.  Those concerns
belong to `native.rs`, `dependency.rs`, `artifact.rs`, and `lib.rs`.  A direct
caller can therefore use `audit_source` to inspect only one in-memory source
unit.  The normal CLI reaches it through the complete collection and report
pipeline described below.

| invocation boundary | what is guaranteed before `audit_source` |
| --- | --- |
| direct public `audit_source` call | only the Rust type `SourceUnit`; path normalization, scope membership, size, uniqueness, and file identity are caller concerns |
| `lib::audit(AuditInput)` | aggregate source path uniqueness/normalization, linker/ELF fact shape, then source scanning in caller-supplied order |
| CLI `recipe-audit` | explicit absolute scope collection, source/ELF limits and UTF-8 reads, optional metadata/grant preparation, then `lib::audit` |

## Position in the production flow

The only production caller of `audit_source` is `audit::audit` in
`audit/src/lib.rs:42-80`.  `lib.rs` re-exports the operation at line 30.  The
CLI path is:

```text
recipe-audit::main::run
    -> native::collect_native_scope
        -> Vec<SourceUnit> (one per supported file)
    -> AuditInput { sources, dependencies, linker_inputs, elf_facts, grants }
    -> lib::audit
        -> AuditInput::validate
        -> source::audit_source for each source
        -> dependency graph audit, linker-input audit, ELF-fact audit
        -> global sort and deduplication
        -> legacy-grant application or next-mode rejection
    -> AuditReport and JSON/exit code
```

`lib.rs` declares `mod source;` without a feature gate. The module is private
as a Rust module, while `audit_source` is made public through the facade; the
lexer and all helper functions remain crate-private implementation details.

`main::run` requires explicit `--mode` and `--scope`; it never selects the
current directory or runs Cargo metadata implicitly.  It also converts each
`--link-input` argument into a separate `LinkerInput` with path `<command-line>`
and line `0`, reads optional metadata and grants as bounded absolute files, and
then invokes `lib::audit`.  The standalone kernel module `kernel/src/audit.rs`
has a different LLVM helper and does not call this module.

On a successful aggregate audit, `main::run` serializes the `AuditReport` with
pretty JSON. It returns process code `0` when `report.passed` is true and `1`
when the report contains blocking findings. `audit_source` itself neither
serializes nor chooses either process code.

`lib::audit` invokes `audit_source` in the order supplied by its `sources`
vector.  `collect_native_scope` supplies that vector in sorted canonical-path
order, while `AuditInput::validate` rejects duplicate display paths before any
source is audited.  `lib::audit` sorts and deduplicates the aggregate again, so
source-local ordering is deterministic even when a programmatic caller supplies
an arbitrary order.

| caller or owner | source lines | handoff to this module |
| --- | --- | --- |
| `main::run` | `audit/src/main.rs:20-97` | Parses CLI options, calls native collection, builds `AuditInput`, and calls `lib::audit`. |
| `native::collect_native_scope` | `audit/src/native.rs:33-98` | Reads supported files into `SourceUnit` values; it never calls `audit_source` directly. |
| `lib::audit` | `audit/src/lib.rs:42-80` | Validates the aggregate input and extends its finding vector with `audit_source(source)?` for every source. |
| public re-export | `audit/src/lib.rs:30` | Exposes `audit_source` to programmatic callers without exposing lexer internals. |

## Input contract

`SourceUnit` is defined in `audit/src/model.rs:193-210`:

| field | meaning used by this module |
| --- | --- |
| `path: String` | Display path copied into every finding and into lexical errors. `Finding::blocking` slash-normalizes the path on findings; the lexical error stores the path string as supplied. `audit_source` itself does not validate that it is nonempty, relative, normalized, unique, or inside a filesystem scope. `AuditInput::validate` checks nonempty, normalized, and duplicate display paths; the native collector, not this model validator, establishes the relative in-scope relationship. |
| `kind: SourceKind` | Selects lexer comment/literal rules and source/build/LLVM classification behavior. Variants are `Rust`, `Zig`, `C`, `Cpp`, `LlvmIr`, and `BuildMetadata`. |
| `contents: String` | In-memory UTF-8 text. The function borrows it and never mutates it. The collector obtains it with `fs::read_to_string`; invalid UTF-8 therefore fails during collection, before this module. |

`SourceUnit::new` only normalizes backslashes in the display path. It does not
perform the validation that `AuditInput::validate` performs. The exact policy
self-exemption below consequently compares the supplied display string, not a
canonical filesystem path.

## Internal structure

| function | lines | role |
| --- | --- | --- |
| `audit_source` | 16-64 | Self-policy bypass, lexing, LLVM pre-pass dispatch, general token walk, sort, and deduplication. |
| `audit_llvm` | 66-108 | Detects LLVM `@symbol` positions associated with `declare`, `call`, `invoke`, or `callbr`. |
| `audit_string` | 110-139 | Normalizes one string token, determines context, checks policy, and appends one finding. |
| `string_has_prohibited_token` | 141-160 | Checks whole string and separator-delimited components against library, dependency, and interface policy. |
| `push_interface_finding` | 162-192 | Maps policy classifications to finding categories and appends only prohibited identifiers. |
| `identifier_category` | 194-200 | Chooses `build-link-input` for BuildMetadata or `source-api` otherwise. |
| `follows_at` | 202-204 | Identifies an immediate same-line LLVM symbol token. |
| `line_has_link_context` | 206-219 | Performs raw same-line link-marker detection. |
| `line_has_include_context` | 221-227 | Performs raw same-line include/import-marker detection. |
| `line_text` | 229-234 | Converts a one-based lexer line to the corresponding source line. |

### Cross-module contracts consumed

| module | value or operation consumed | consequence for `source.rs` |
| --- | --- | --- |
| `lexer.rs` | `lex(contents, kind) -> Result<Vec<Lexeme>, LexError>` | Supplies comment-free complete tokens and one-based start lines, or the only source-local error path. |
| `model.rs` | `SourceUnit`, `SourceKind`, `Finding`, `FindingCategory` | Defines the input shape and stable output fields. `Finding::blocking` normalizes display paths and fixes disposition to blocking. |
| `policy.rs` | `classify_interface_symbol`, `classify_library`, private `classify_dependency`, `InterfaceClassification`, `NativeInterface` | Supplies exact policy decisions; this module only maps them to categories and never changes the policy tables. |
| `native.rs` | `NativeCollection` and `collect_native_scope` | Establishes filesystem scope, source kind, display path, and UTF-8 contents before this module runs. |
| `lib.rs` | `audit(AuditInput)` | Owns aggregate validation, fact combination, legacy grants, report pass status, and final ordering. |

The neighboring module documents provide the corresponding full traces for
[`lexer.rs`](lexer.md), [`model.rs`](model.md), [`native.rs`](native.md),
[`policy.rs`](policy.md), [`lib.rs`](lib.md), and [`main.rs`](main.md).

The private helper signatures are all local data-flow operations:

```text
audit_llvm(&SourceUnit, &[Lexeme]) -> Vec<Finding>
audit_string(&SourceUnit, &Lexeme, &mut Vec<Finding>)
string_has_prohibited_token(&str, SourceKind) -> bool
push_interface_finding(&mut Vec<Finding>, &SourceUnit, u64, &str, FindingCategory)
identifier_category(SourceKind) -> FindingCategory
follows_at(&[Lexeme], usize) -> bool
line_has_link_context(&SourceUnit, u64) -> bool
line_has_include_context(&SourceUnit, u64) -> bool
line_text(&SourceUnit, u64) -> Option<&str>
```

None of these helpers owns a filesystem path, opens a handle, or returns a
policy classification to its caller. They either append to the caller-owned
local finding vector or answer a pure predicate/query.

## Top-level algorithm

`audit_source` is implemented at `source.rs:16-64` and has these ordered
stages:

1. If `source.path` is exactly `audit/src/policy.rs` (regardless of its
   supplied `SourceKind` or contents), return an empty success.
   This is the self-hosted policy dictionary exception. The dictionary's
   prohibited spellings are definitions, not runtime calls. The exception is
   source-only: dependency, linker, and ELF facts for the built auditor still
   pass through the other modules.
2. Call `lexer::lex(&source.contents, source.kind)` once. Convert a `LexError`
   into `AuditError::Lexical` with this source path, the one-based lexer line,
   and the static reason. No partial token stream is accepted after a lexical
   error.
3. For LLVM IR, run `audit_llvm` first to attach `llvm-declaration` and
   `llvm-call` categories to external symbols. Other source kinds start with an
   empty finding vector.
4. Walk every emitted token. Identifiers go through `push_interface_finding`;
   strings go through `audit_string`; `At` tokens are structural markers only.
   For LLVM, an identifier or string immediately following an `At` on the same
   source line is skipped by this general pass because `audit_llvm` owns that
   symbol position.
5. Sort the local vector using `Finding`'s derived order and remove adjacent
   exact duplicates. Return `Ok(findings)`.

Every source finding created here is `FindingDisposition::Blocking`. This module
never creates a grandfathered finding. `lib::audit` may later change an exact
finding's disposition when legacy grants are applied. A prohibited token is not
an early return: the token walk continues, and a successful source result lets
the aggregate audit inspect later sources and all other fact classes.

### Control-flow table

| stage | predicate | action |
| --- | --- | --- |
| policy exemption | `source.path == "audit/src/policy.rs"` | Return `Ok(Vec::new())`; do not lex. |
| lexing | `lex` returns `Err` | Return `AuditError::Lexical`; do not inspect any partial tokens. |
| LLVM initialization | `source.kind == LlvmIr` | Seed findings with `audit_llvm`; otherwise seed with an empty vector. |
| identifier token | LLVM and `follows_at(tokens, index)` | Skip this token. |
| identifier token | any other identifier | Classify through `push_interface_finding`. |
| string token | LLVM and `follows_at(tokens, index)` | Skip this token. |
| string token | any other string | Normalize and classify through `audit_string`. |
| `At` token | always | Ignore in the general pass. |
| completion | after token walk | Sort and exact-deduplicate, then return success. |

Equivalent implementation-level pseudocode is:

```text
if path == "audit/src/policy.rs": return []
tokens = lex(contents, kind) or return Lexical(path, line, reason)
findings = (kind == LlvmIr) ? audit_llvm(source, tokens) : []
for (index, token) in tokens:
    if token is Identifier:
        if kind == LlvmIr and follows_at(tokens, index): continue
        push_interface_finding(token.text, token.line, identifier_category(kind))
    if token is String:
        if kind == LlvmIr and follows_at(tokens, index): continue
        audit_string(source, token)
    if token is At: continue
sort(findings)
deduplicate_exactly(findings)
return findings
```

The two independent LLVM decisions are intentional: `audit_llvm` determines a
declaration/call category for an `@` position, while the general walk classifies
all other complete identifiers and strings. They share `push_interface_finding`
so policy classification and the direct-KFD/Driver-shaped category mapping stay
identical.

## Lexer boundary and token flow

`source.rs` consumes the private `Lexeme` and `LexemeKind` types from
`audit/src/lexer.rs`. The lexer scans bytes while retaining one-based starting
line numbers. It emits only:

```text
LexemeKind::Identifier  { text, line }
LexemeKind::String      { text without delimiters, line }
LexemeKind::At          { text "@", line }
```

An `At` token is emitted only when the scanner is outside a comment or quoted
literal. An `@` character inside a string is part of that string's text and
cannot trigger LLVM symbol handling by itself.

Comments are discarded before this module sees them. The lexer recognizes `//`
and nested `/* ... */` comments for every kind, LLVM `;` line comments, and
BuildMetadata `#` line comments. It recognizes normal double-quoted strings,
non-BuildMetadata character literals, Rust lifetimes, and Rust raw strings.
Rust raw-string delimiters can contain any number of `#` characters. Identifiers
are ASCII-oriented: starts are alphabetic, `_`, `$`, or `.`, and continuations
also allow ASCII digits and `-`.

The lexer is intentionally permissive about language syntax. Punctuation other
than `@`, quote delimiters, and the comment introducers is skipped. It does not
build an AST or validate declarations. Unicode identifier characters are not
lexed as identifiers. Escapes are skipped as byte pairs while searching for a
normal string terminator. Newlines in raw strings and block comments advance the
line counter; a backslash escape in a normal string is skipped as one escape and
does not increment that counter.

The lexer returns an error for an unterminated nested block comment, normal
string, character literal, or Rust raw string. `audit_source` maps that error to
`AuditError::Lexical { path, line, reason }` (`error.rs:9-13`). An invalid source
syntax that is still lexically traversable does not itself fail the audit; only
tokens recognized by the policy can produce findings.

For an unterminated construct, `line` is the one-based opening line recorded by
the lexer, not the line where end-of-input was reached.

The lexical failure reasons are stable string literals from `lexer.rs`:

| unterminated construct | `AuditError::Lexical.reason` |
| --- | --- |
| nested block comment | `unterminated block comment` |
| double-quoted string | `unterminated string literal` |
| non-BuildMetadata character literal | `unterminated character literal` |
| Rust raw string | `unterminated raw string literal` |

The top-level scanner branch order is relevant to what reaches the source audit:

| byte or prefix | active kinds | token/effect visible to `source.rs` |
| --- | --- | --- |
| LF `\n` | all | increments the lexer line and emits nothing |
| `//` | all | skips to LF or EOF and emits nothing |
| `/*` | all | consumes nested block comments, counts LF, or returns lexical error |
| `;` | LLVM IR | skips the physical line as an LLVM comment |
| `#` | BuildMetadata | skips the physical line as a build-metadata comment |
| `@` | all | emits `LexemeKind::At` with text `@` |
| `"` | all | emits one String lexeme after a matching double quote, or errors |
| `'` | non-BuildMetadata | consumes a Rust lifetime or character literal, emitting no token, or errors |
| `r` followed by hashes and `"` | Rust | emits one raw String lexeme, or errors if it never closes |
| ASCII identifier start | all | emits one complete Identifier lexeme |
| any other byte | all | advances one byte and emits nothing |

Because this order is in `lexer.rs`, `source.rs` never sees comment contents,
character-literal contents, or an `@` that occurs inside a string. It does see
identifiers that occur between apostrophes in BuildMetadata, because that kind
does not enter the apostrophe literal branch.

The `SourceKind` switches that matter to this module can be summarized as:

| kind | lexer-specific behavior | ordinary identifier category | string policy kind before context override |
| --- | --- | --- | --- |
| `Rust` | Rust lifetimes and raw strings are recognized; character literals are skipped. | `source-api` | `Rust`, unless link context changes it to `BuildMetadata`. |
| `Zig` | Character literals are skipped; no Rust raw-string handling. | `source-api` | `Zig`, unless link context changes it to `BuildMetadata`. |
| `C` or `Cpp` | Character literals are skipped; C/C++ punctuation is otherwise not parsed. | `source-api` | `C` or `Cpp`, unless link context changes it to `BuildMetadata`. |
| `LlvmIr` | `;` starts a line comment; `@` is emitted as a structural token. | `source-api` for non-`@` identifiers | `LlvmIr`, with `@symbol` positions owned by the LLVM pre-pass. |
| `BuildMetadata` | `#` starts a line comment; apostrophes are punctuation, so single-quoted contents are still scanned as ordinary tokens. | `build-link-input`, with dependency fallback | always treated as `BuildMetadata` link context |

## LLVM IR pre-pass

`audit_llvm` (`source.rs:66-108`) runs only for `SourceKind::LlvmIr`.

For every `At` token, it examines the immediately following token. Only an
identifier or string can be the candidate symbol. It scans the reverse prefix of
tokens on the same line, stopping at the first token from another line. Any
identifier named `declare` sets the declaration flag; `call`, `invoke`, or
`callbr` sets the call flag. Punctuation and strings in that prefix are ignored.

The category selection is intentionally ordered:

```text
declaration flag true -> FindingCategory::LlvmDeclaration
otherwise call flag true -> FindingCategory::LlvmCall
otherwise -> no LLVM pre-pass finding
```

When a category is selected, the candidate's own line and text are passed to
`push_interface_finding`. Thus policy still decides whether the symbol is
prohibited, allowed, or unknown. The pre-pass does not report every LLVM global
name. It reports only `@name` positions whose same-line prefix contains an LLVM
declaration or call opcode. Even when the candidate lexeme is a quoted LLVM
name, this path is an interface-symbol classification, not `audit_string`;
library basename and runtime-load handling applies only when the string survives
to the general string pass.

The general token pass suppresses an identifier or string immediately preceded by
an `At` token on the same line (`follows_at`, lines 202-204). This prevents the
normal `source-api` or `runtime-load` path from duplicating an LLVM declaration or
call. If `@` and its candidate are separated by a newline, `audit_llvm` can still
inspect the candidate because it uses `tokens.get(index + 1)`, but `follows_at`
does not suppress the general pass. A finding can then be emitted once by the
LLVM pre-pass and once by the general pass with different line/category data, so
the final exact deduplication is not guaranteed to collapse that malformed or
unusual layout. This is a lexical line heuristic, not an LLVM grammar parser.

Because comments produce no tokens, a same-line comment between `@` and a name is
still an immediate token adjacency for `follows_at`. A block comment containing a
newline changes the candidate's recorded line and can therefore change that
suppression decision.

The pre-pass gives declaration precedence if a line contains both `declare` and
a call-like word before the same `@` token. It also scans every same-line
identifier before the `@`, without stopping at a statement boundary. A comment
cannot contribute tokens because the lexer removed it, but arbitrary identifiers
in the same remaining line can satisfy the textual opcode test.

Its core decision can be written as:

```text
for each At at index:
    candidate = tokens[index + 1] if present and Identifier or String
    prefix = reverse tokens before At while line == At.line
    declaration = any(prefix identifier == "declare")
    call = any(prefix identifier in {"call", "invoke", "callbr"})
    category = LlvmDeclaration if declaration
               else LlvmCall if call
               else none
    if category != none: push_interface_finding(candidate, category)
```

## General identifier pass

For each non-skipped `LexemeKind::Identifier`, `audit_source` calls
`push_interface_finding` with the token's one-based line and
`identifier_category(source.kind)`:

| source kind | normal category passed to helper |
| --- | --- |
| `BuildMetadata` | `FindingCategory::BuildLinkInput` |
| `Rust`, `Zig`, `C`, `Cpp`, `LlvmIr` | `FindingCategory::SourceApi` |

`push_interface_finding` (`source.rs:162-192`) first calls
`policy::classify_interface_symbol`. For BuildMetadata only, an `Unknown`
classification is then passed to `policy::classify_dependency`. The helper
maps the result as follows:

| policy result | emitted result |
| --- | --- |
| `Prohibited(CudaDriverOutsideAllowlist)` or `Prohibited(DirectKfd)` | `DisallowedNativeInterface`, regardless of source kind |
| any other `Prohibited(_)` in BuildMetadata | `BuildLinkInput` |
| any other `Prohibited(_)` in a source or LLVM unit | the `normal_category` supplied by the caller |
| `Allowed(_)` or `Unknown` | no finding |

The symbol stored in the finding is the original complete token. Policy may
strip decoration while classifying it, but `push_interface_finding` does not
rewrite the stored text. The path is copied from the `SourceUnit`, the line is
the lexer line, and `Finding::blocking` supplies the blocking disposition.

The helper's exact dispatch is:

```text
classification = classify_interface_symbol(symbol)
if classification == Unknown and kind == BuildMetadata:
    classification = classify_dependency(symbol)
match classification:
    Prohibited(CudaDriverOutsideAllowlist | DirectKfd): DisallowedNativeInterface
    Prohibited(_): BuildLinkInput if BuildMetadata else normal_category
    Allowed(_) | Unknown: return without a finding
append blocking(category, source.path, line, symbol)
```

The policy classifier itself is in `audit/src/policy.rs`; the full dictionary
trace is in [`policy.md`](policy.md). It recognizes exact
interface symbols, the reviewed CUDA Driver allowlist, direct KFD spellings,
HIP and CUDA Runtime families, and operation-library families. Allowed ROCr/HSA
symbols and exact reviewed CUDA Driver symbols are deliberately silent. A
CUDA-Driver-shaped `cuXxx` symbol outside the allowlist and direct-KFD symbols
are surfaced as `disallowed-native-interface`, rather than being conflated with
the ordinary source API category. This special remapping is used by
`push_interface_finding`; a prohibited component found inside `audit_string`
keeps the string-context category (`runtime-load` or `build-link-input`) because
that helper emits the context category directly.

## String pass

`audit_string` (`source.rs:110-139`) handles every non-skipped string token.
The lexer has already removed the surrounding delimiter. The helper then:

1. Trims leading and trailing whitespace.
2. Removes trailing token-text `\00` sequences (the Rust source pattern is
   `"\\00"`), then trailing token-text `\0` sequences (source pattern
   `"\\0"`). These are textual escape suffixes in the token, not NUL bytes.
3. Determines link context from the source kind or from raw text on the token's
   source line.
4. Selects a classification kind, checks the entire trimmed value and its
   components, and returns if no prohibited token is present.
5. Emits one blocking finding whose `symbol` is the entire trimmed string, not
   the individual component that caused the match.

### Link and include context

`line_has_link_context` (`source.rs:206-219`) returns true for all
BuildMetadata units. For other kinds it checks the original, unlexed source line
for any of these literal markers:

```text
rustc-link-lib
rustc-link-arg
#[link
target_link_libraries
linkSystemLibrary
-l
```

If link context is true, the string is classified as `SourceKind::BuildMetadata`
and a prohibited value is categorized as `FindingCategory::BuildLinkInput`.
This enables dependency-name checks even when a Rust, C, or Zig source contains
a build-link string.

If link context is false, `line_has_include_context` (`source.rs:221-227`)
checks the original line for `#include`, `@import`, or `@cImport`. A prohibited
string on such a line is `FindingCategory::SourceApi`. A prohibited string with
neither context is `FindingCategory::RuntimeLoad`.

The category precedence is therefore:

```text
BuildMetadata kind or link marker -> BuildLinkInput
else include/import marker       -> SourceApi
else                             -> RuntimeLoad
```

Context tests are raw substring tests over `source.contents`, not token-aware
tests. A marker in a comment, an unrelated string, or other text on the same
line can change the category even though comments do not produce tokens. Link
context wins over include context. Marker matching is case-sensitive and does
not trim or normalize the source line. `line_text` converts the one-based lexer
line to a zero-based `usize` with `checked_sub(1)` and returns `None` for line
zero or an out-of-range line; normal lexer output starts at line one.

### Prohibited-token check

`string_has_prohibited_token` (`source.rs:141-160`) first classifies the whole
trimmed value with `classify_library`. For BuildMetadata it also classifies the
whole value with `classify_dependency`. If either whole-value check is
prohibited, the helper returns immediately.

It then splits the value at `/`, `\\`, `=`, `,`, `:`, `(`, `)`, `[`, `]`, `;`,
and ASCII whitespace, drops empty components, and checks each component with
`classify_interface_symbol`. For BuildMetadata components it additionally
checks `classify_library` and `classify_dependency`. Consequently:

```text
component_separator(character) =
    character in {'/', '\\', '=', ',', ':', '(', ')', '[', ']', ';'}
    or character.is_ascii_whitespace()
```

Hyphens, periods, braces, quotes that remain inside a token, and other
punctuation are not separators in this helper. Hyphenated BuildMetadata package
names therefore remain one identifier for the dependency classifier, while a
slash or assignment expression is split before component policy checks.

| value location | whole-value checks | component checks |
| --- | --- | --- |
| ordinary source/LLVM string | `classify_library` | `classify_interface_symbol` |
| link-context source string | `classify_library`, then BuildMetadata `classify_dependency` | `classify_interface_symbol`, `classify_library`, and `classify_dependency` |
| BuildMetadata string | `classify_library`, then `classify_dependency` | `classify_interface_symbol`, `classify_library`, and `classify_dependency` |

Identifier policy calls differ from string policy: an identifier first uses
`classify_interface_symbol`, then uses `classify_dependency` only when the kind
is `BuildMetadata` and the symbol was `Unknown`. It never calls
`classify_library`. LLVM declaration/call candidates follow this identifier
path even when their lexeme kind is `String`.

* a quoted include such as `hip/hip_runtime.h` is caught through the `hip`
  component and is reported as a source API finding;
* a build string such as
  `cargo:rustc-link-lib=static=rocblas` is caught through its `rocblas`
  component and is reported as a build-link finding;
* a runtime string naming `libcublas.so.12` is caught by whole-value library
  normalization and is reported as a runtime-load finding;
* ordinary runtime strings are not split into library checks for every
  component. Component-level library and dependency checks are enabled only
  when the effective kind is BuildMetadata.

The whole string, after trimming and escape-suffix removal, is preserved as the
finding symbol. The string scanner does not identify which component matched and
does not emit one finding per component.

The string path is equivalently:

```text
value = trim(token.text)
value = trim_end(value, "\\00")
value = trim_end(value, "\\0")
link = kind == BuildMetadata or line_has_link_context(source, token.line)
effective_kind = BuildMetadata if link else kind
if not string_has_prohibited_token(value, effective_kind): return
category = BuildLinkInput if link
           else SourceApi if line_has_include_context(source, token.line)
           else RuntimeLoad
append blocking(category, source.path, token.line, value)
```

### Representative outcomes

The following rows describe the current classifier, assuming the shown token is
on the shown line and the `SourceUnit.path` is already valid for the aggregate
pipeline:

| source text and kind | result |
| --- | --- |
| Rust identifier `cudaMalloc` | one blocking `source-api` finding with symbol `cudaMalloc` |
| Rust identifier `hsa_queue_create` | no finding, because the ROCr/HSA family is allowed |
| Rust identifier `cuNotAllowlisted` | one `disallowed-native-interface` finding, because it has a Driver-shaped `cuXxx` spelling outside the exact allowlist |
| Rust string `"libcublas.so.12"` without a context marker | one `runtime-load` finding whose symbol is the whole library string |
| C string on `#include "hip/hip_runtime.h"` | one `source-api` finding whose symbol is `hip/hip_runtime.h` |
| Rust build-link string `"cargo:rustc-link-lib=static=rocblas"` | one `build-link-input` finding whose symbol is the whole string |
| BuildMetadata identifier `hip-sys` | one `build-link-input` finding through dependency-name fallback |
| BuildMetadata text `'hip'` | one `build-link-input` finding for identifier `hip`; apostrophes are skipped punctuation in this kind |
| LLVM `declare i32 @rocblas_sgemm(...)` | one `llvm-declaration` finding for `rocblas_sgemm` |
| LLVM `call i32 @cublasCreate_v2(...)` | one `llvm-call` finding for `cublasCreate_v2` |
| LLVM `@global` with no same-line declaration or call word | no LLVM pre-pass finding, and the general pass skips the same-line symbol |
| `// hipMalloc` comment | no finding, because comments are removed before classification |
| Unterminated string or block comment | `Err(AuditError::Lexical { ... })`, with no findings returned |

The same cases as a token-to-category trace are:

```text
Rust: `cudaMalloc`
  lex -> Identifier("cudaMalloc", line 1)
  walk -> push_interface_finding(SourceApi)
  result -> Blocking(source-api, "cudaMalloc")

C: `#include "hip/hip_runtime.h"`
  lex -> Identifier("include"), String("hip/hip_runtime.h")
  walk -> identifier is unknown; string has include context
  result -> Blocking(source-api, "hip/hip_runtime.h")

Rust: `println!("cargo:rustc-link-lib=static=rocblas")`
  lex -> identifiers plus String("cargo:rustc-link-lib=static=rocblas")
  walk -> raw line marker selects BuildMetadata string policy
  result -> Blocking(build-link-input, full string)

LLVM: `declare i32 @rocblas_sgemm(ptr)`
  lex -> Identifier("declare"), Identifier("i32"), At, Identifier("rocblas_sgemm")
  pre-pass -> declaration flag, then push_interface_finding(LlvmDeclaration)
  general pass -> symbol follows At on same line, so skip
  result -> Blocking(llvm-declaration, "rocblas_sgemm")
```

## Finding categories produced here

`source.rs` can produce these `FindingCategory` values:

| category | creation path |
| --- | --- |
| `source-api` | Prohibited interface identifier in Rust, Zig, C, C++, or an ordinary LLVM token; or a prohibited include/import string. |
| `build-link-input` | Prohibited BuildMetadata identifier or string; or a prohibited link-context string in another source kind. |
| `llvm-declaration` | Prohibited symbol after `@` on a same-line LLVM declaration. |
| `llvm-call` | Prohibited symbol after `@` on a same-line LLVM `call`, `invoke`, or `callbr`. |
| `runtime-load` | Prohibited string with no link or include context. |
| `disallowed-native-interface` | A prohibited direct-KFD symbol or non-allowlisted CUDA-Driver-shaped symbol in the identifier path. |

The producer matrix is:

| token | Rust/Zig/C/C++ | LLVM IR | BuildMetadata |
| --- | --- | --- | --- |
| ordinary identifier | `push_interface_finding(SourceApi)` | `push_interface_finding(SourceApi)`, unless it follows same-line `@` | `push_interface_finding(BuildLinkInput)` with dependency fallback |
| `@` followed by identifier/string | not a special path | `audit_llvm`, then same-line general-pass skip | ordinary token behavior, because LLVM gating is kind-specific |
| ordinary string | `audit_string`, `RuntimeLoad` unless link/include context | `audit_string`, `RuntimeLoad` unless link/include context; `@` target skip applies | `audit_string`, always `BuildLinkInput` if prohibited |
| comment or character literal contents | no token | no token | comments removed; apostrophe interiors remain identifiers |

The matrix names the normal category. A prohibited direct-KFD or non-allowlisted
CUDA-Driver-shaped identifier overrides that normal category with
`DisallowedNativeInterface`, as described in the helper mapping above.

The module does not emit `dependency`, `dynamic-needed`, or
`undefined-symbol`; those categories are produced by the dependency and artifact
modules. A BuildMetadata dependency name discovered in this lexical path is
reported as `build-link-input`, while a complete Cargo dependency graph is
reported as `dependency` by `dependency.rs`.

### Result shape

The returned `Finding` fields serialize as `category`, `path`, `line`, `symbol`,
and `disposition`. Category and disposition enum values use kebab-case in JSON.
Source findings have one-based text lines from the lexer, a slash-normalized
display path, the exact complete identifier or trimmed string that was passed to
policy, and `blocking` disposition. The `line == 0` convention described in the
README is reserved for dependency-graph and binary-artifact facts; this module
does not create line-zero findings. `audit_source` returns only the vector, not
the enclosing `AuditReport` fields `mode` and `passed`.

## Determinism and duplicate behavior

Each helper appends findings without relying on hash-map iteration. At the end
of `audit_source`, `findings.sort()` uses the derived `Ord` implementation for
`Finding` (`category`, `path`, `line`, `symbol`, then disposition), and
`findings.dedup()` removes equal adjacent values. The public `lib::audit` repeats
sort and dedup after adding graph, linker, and ELF findings. A finding is
therefore stable for a fixed `SourceUnit` and policy, but two semantically
similar findings with different categories, paths, lines, or symbols are not
merged. For example, a `rocblas` source-string `BuildLinkInput` finding, a
reachable Cargo package `Dependency` finding, and an ELF `DynamicNeeded`
finding remain three evidence records even when their displayed symbol text is
similar.

All source findings are blocking at this stage. In legacy mode, `lib.rs` later
matches a grant by exact `(category, path, line, symbol)` and may change that
finding to `Grandfathered`. Grants cannot alter the lexical scan or suppress a
lexical error.

`AuditReport::new` marks the aggregate as passed only when every finding is
`Grandfathered`. Therefore an ordinary source finding makes next mode fail and
also makes legacy mode fail unless a matching exact grant is supplied and all
other facts are clean. An empty source vector contributes no finding and does
not by itself prevent a report from passing.

## Complexity and memory behavior

For a `SourceUnit` with `B` content bytes, `T` emitted tokens, `S` string
tokens, and `F` findings, the lexer, ordinary token walk, and policy checks are
linear in the input and policy text, with `O(B + T + total_string_bytes)` work
before line-context rescans and the final sort. The local sort is `O(F log F)`
and the exact deduplication pass is linear in `F`.

LLVM classification scans the same-line token prefix for every `At`, so a
single unusually long line with many `@` positions can approach `O(T²)` work.
`line_text` uses `source.contents.lines().nth(...)` for each context check and
does not cache line offsets; many strings at late line numbers can likewise
rescan earlier lines. Findings and lexemes are retained in memory until the
function returns.

The source collector is also batch-oriented. It retains every selected file's
contents in the `NativeCollection` before `lib::audit` starts, with a 16 MiB
per-file limit and a 100,000-file count limit but no aggregate source-byte cap.
These bounds and the implementation's retained vectors are collection
properties, not tunables in `source.rs`.

`audit_source` has no mutable global state, filesystem handle, clock, random
source, or shared cache. Its only mutation is to local token and finding
vectors, so independent calls are reentrant and depend only on the supplied
`SourceUnit` and the compiled policy tables.

Lexeme text and finding path/symbol fields are owned allocations. The returned
`Vec<Finding>` therefore remains valid after the borrowed `SourceUnit` is
dropped; no finding retains a reference into source contents.

The module has no `unwrap`, `expect`, panic-based validation, unsafe block, or
fallback parser. Candidate lookahead uses `tokens.get`, line conversion uses
`checked_sub` and `try_from`, and malformed lexical constructs return the
typed `AuditError::Lexical` path supplied by the lexer.

## Filesystem boundary and source discovery

No filesystem operation occurs in `source.rs`. `native.rs` owns collection and
is the boundary that turns paths into the `SourceUnit` values consumed here.
The relevant sequence is:

1. `collect_native_scope` requires an absolute scope, rejects a symlink or
   non-directory scope, canonicalizes it, and recursively walks that canonical
   directory.
2. `collect_paths` sorts directory entries by filename, rejects every symlink
   encountered, skips `.git` and `target` directories, and keeps only regular
   files whose extension or exact basename maps to a `SourceKind`.
3. Supported mappings are `.rs` to Rust, `.zig` to Zig, `.c` and `.h` to C,
   `.cc`, `.cpp`, `.cxx`, `.hh`, `.hpp`, and `.hxx` to C++, `.ll` to LLVM IR,
   and `.toml`, `.json`, `.yaml`, `.yml`, `.cmake`, `.mk`, `.ninja`, `.bazel`,
   and `.bzl` to BuildMetadata. Exact build basenames include `Cargo.lock`,
   `Makefile`, `CMakeLists.txt`, `BUILD`, `BUILD.bazel`, `WORKSPACE`, and
   `WORKSPACE.bazel`. Matching is case-sensitive. Unsupported extensions,
   non-UTF-8 file names that cannot be converted to a `str`, and non-regular
   non-directory entries are ignored after symlink checks.
4. The collector enforces at most 100,000 source files and at most 16 MiB per
   text file, reads each file with `fs::read_to_string`, derives a slash-
   normalized path relative to the canonical scope, and constructs a
   `SourceUnit`.
5. Paths are sorted before reading, so the normal `sources` vector is
   deterministic. A failed directory read, metadata query, oversized file, or
   UTF-8 decode is returned as `AuditError::Io` or `AuditError::Configuration`
   before `audit_source` is called.

The explicit ELF vector is sorted by its normalized display path after each
artifact is parsed. Consequently both source and binary evidence have stable
collection order before the library's aggregate finding sort; source auditing
itself does not inspect or reorder the ELF vector.

The display root determines the self-policy spelling:

| explicit scope | collected display path for the policy file | source exemption |
| --- | --- | --- |
| repository root `/.../nates-recipe-rs` | `audit/src/policy.rs` | applies |
| audit crate root `/.../nates-recipe-rs/audit` | `src/policy.rs` | does not apply |
| any other scope or injected spelling | caller-supplied relative/display path | applies only if it is exactly `audit/src/policy.rs` |

Explicit ELF paths are collected separately by `native.rs`; they must be
absolute, real regular files, canonicalize under the source scope, and be unique
by canonical path. Their facts are parsed by `goblin` and audited by
`artifact.rs`, not by this module. Optional Cargo metadata and legacy-grant JSON
files are read by `main.rs` with a separate 64 MiB absolute-regular-file bound;
they are not source units. `collect_native_scope` completes both source reading
and explicit ELF collection before returning, so an invalid ELF prevents any of
the collected sources from reaching `audit_source` in that CLI call. Skipping
the `target` directory affects source discovery only; an artifact under `target`
can still be audited when passed explicitly through `--elf`.

`AuditInput::validate` (`model.rs:285-314`) adds programmatic invariants before
the `lib::audit` loop: source and ELF display paths must be nonempty and slash-
normalized, source and ELF paths must be unique, linker paths must be normalized,
linker arguments must be nonempty, and ELF facts cannot contain empty library or
symbol names. A direct call to `audit_source` bypasses these aggregate checks.

The collection-to-source failure mapping is:

| boundary condition | owning error |
| --- | --- |
| relative scope or ELF path | `AuditError::Configuration` |
| missing/unreadable scope entry, directory entry, source metadata, or source read | `AuditError::Io` |
| symlink scope, symlink inside scope, non-directory scope, non-regular explicit ELF, ELF outside scope, duplicate canonical ELF | `AuditError::Configuration` |
| more than 100,000 supported files or a text file over 16 MiB | `AuditError::Configuration` |
| invalid UTF-8 source text | `AuditError::Io` from `read_to_string` |
| malformed or oversized explicit ELF | `AuditError::InvalidElf` |
| canonical display path cannot be stripped back to the scope | `AuditError::Configuration` |

Only after these boundaries succeed does `SourceUnit` reach `audit_source`; a
lexical error from this module is therefore distinct from an I/O or collection
failure.

## Error and failure boundary

The source-specific failure surface is deliberately small:

| condition | result |
| --- | --- |
| exact self-policy path | `Ok(empty)` without lexing |
| empty contents or contents containing only ignored text | `Ok(empty)` |
| valid lexing and no prohibited policy match | `Ok(empty)` |
| one or more prohibited matches | `Ok(sorted, deduplicated, blocking findings)` |
| unterminated comment or literal recognized by `lexer::lex` | `Err(AuditError::Lexical { path, line, reason })` |

`AuditError` formats that last case as
`{path}:{line}: lexical audit failed: {reason}`. The source function does not
log, print, or recover from the error; ownership of presentation stays with the
CLI or the calling application. Findings that were accumulated from tokens
before the failed construct are discarded with the `Err`; no partial `Vec` is
returned.

Policy classification never returns an error. The function does not report a
malformed language construct unless the construct prevents lexical termination.
Source collection failures, invalid paths, duplicate facts, malformed Cargo
metadata, invalid dependency graphs, and invalid ELF are raised by their owning
modules and prevent the report from being produced. `main` prints such errors
with exit code `2`; a produced report with blocking findings prints JSON and
returns exit code `1`.

The standalone function has no mode or grant handling. In the library pipeline,
source scanning occurs before the final mode branch in `lib::audit`, so a
programmatic `AuditInput` can encounter a source lexical error before a later
next-mode legacy-grant configuration error. The CLI rejects its mode/grant
combination while assembling arguments, before it constructs the `AuditInput`.

The CLI's metadata and grant parsing also occurs before `lib::audit`; malformed
metadata, missing required grants, or invalid grant JSON can therefore prevent
source scanning even though the native collector has already read the source
files.

`lib::audit` uses `findings.extend(audit_source(source)?)`, so the first lexical
error aborts the aggregate call immediately. Later source units, the dependency
graph, linker inputs, ELF facts, and report/grant processing are not evaluated in
that call. A successful source result, including an empty one, allows the loop
to continue. Since the library does not sort before this loop, a programmatic
caller controls which source error is observed first; native collection already
sorts its own source paths.

## Intentional limitations and review points

* The policy is token and basename based. It avoids arbitrary substring
  matching, but it does not resolve macros, aliases, generated code, conditional
  compilation, shell expansion, compiler semantics, or dynamic loading behavior.
* Context is line-local and marker based. A marker in a comment or unrelated
  text can reclassify a string, and a link/include directive spanning lines is
  not joined by this module.
* A multiline string is one `Lexeme` whose line is its opening line. Findings
  and context markers for prohibited text later in that string use that opening
  line rather than the physical line containing the text.
* Include and link syntax is not parsed. Angle-bracket headers, macro-expanded
  paths, and unquoted linker forms are reduced to the lexer tokens that happen
  to be present. A header or `-l` spelling is caught only when a complete emitted
  component satisfies policy; explicit linker arguments are handled separately
  by `artifact.rs`.
* `BuildMetadata` covers several formats with different grammars. The shared
  lexer applies BuildMetadata `#` comments and double-quoted strings to all of
  them, so JSON, TOML, YAML, CMake, Make, Ninja, and Bazel syntax is not
  individually validated.
* LLVM recognition is a same-line opcode heuristic. It does not parse LLVM IR,
  and an `@` symbol not preceded by the recognized words is intentionally not
  reported by the LLVM pre-pass. The same-line general-pass skip means even a
  prohibited-looking `@cudaMalloc` global is ignored when no recognized opcode
  owns that position. Newline placement can affect duplicate or category
  behavior as described above.
* Identifier recognition is ASCII-oriented and punctuation is mostly discarded.
  A prohibited spelling split across punctuation is seen only if the resulting
  complete components individually match policy.
* Identifier tokens are classified as symbols, not library names. Library
  basename normalization such as removing a leading `lib` or `-l` is applied to
  strings and artifact/linker evidence, not to an identifier like `libcublas`.
* Whole-value library checks expect a standalone or path-like library token.
  A composite runtime string containing prose around a library name can evade
  basename normalization, and ordinary source strings do not run library
  classification on each split component.
* Rust lifetime-looking text and non-BuildMetadata character literals are
  consumed by the lexer, so policy spellings inside those lexical regions are
  not inspected as identifiers or strings.
* Line tracking is byte-level and counts `\n`. An escaped newline in a normal
  string is skipped as an escape, and other Unicode line-separator characters do
  not advance the lexer line counter.
* Strings retain escape text. Only the two specified trailing textual NUL forms
  are removed; other escape encodings are classified as written.
* The policy-file exemption is an exact display-path string
  (`audit/src/policy.rs`). A scope rooted at the `audit` directory produces
  `src/policy.rs`, and a programmatic caller can supply another spelling, so the
  exemption does not follow canonical paths or file identity. A direct caller
  can also claim the exempt path for arbitrary contents because `source.rs` does
  not verify file identity. The normal native collector supplies that path only
  for the corresponding file in the explicit scope. The exception also happens
  before lexical validation for that one path. `SourceUnit::new` changes a
  backslash spelling into the slash spelling before this comparison, while a
  direct struct literal can preserve the backslashes and miss the exemption.
* Source collection rejects symlinks and constrains explicit ELF paths, but
  `source.rs` itself has no snapshot, hash, or race protection after a
  `SourceUnit` is constructed. It audits the bytes it receives.
* The source pass emits only blocking findings. Whether those findings are
  grandfathered, whether all facts pass, and whether the final report is
  successful are decisions made after this module returns.

These limits are part of the current boundary. Extending them would require a
policy or collection change, not a new fallback inside `audit_source`.
