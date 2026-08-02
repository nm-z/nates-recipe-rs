# `recipe-audit` command flow

This document traces the `recipe-audit` binary in `audit/src/main.rs` from
process entry through argument parsing, filesystem collection, policy
evaluation, report serialization, and the final process code. The binary is a
small adapter around the `recipe_audit` library. It does not discover a scope,
run Cargo itself, load a policy file, or infer missing facts. Every source,
dependency, linker, and artifact input is either collected from an explicit
absolute path or supplied as an explicit command-line value.

The intended product boundary is a deterministic prohibition gate. `next`
audits replacement code with no exceptions. `legacy` audits older code while
requiring each exception to be represented by one exact, typed grant. The
policy accepts ROCr/HSA and the reviewed CUDA Driver symbols, and rejects HIP,
the CUDA Runtime API, direct KFD ownership, and the listed operation libraries.
The same policy is applied to source text, build metadata, Cargo dependency
closure, linker arguments, LLVM declarations and calls, ELF `DT_NEEDED` names,
and undefined artifact symbols.

## Module boundaries

The binary imports only the public library surface:

| Module | Responsibility reached from `main.rs` |
| --- | --- |
| `audit/src/main.rs` | Manual CLI parser, bounded JSON reads, input assembly, JSON output, and process exit codes. |
| `audit/src/lib.rs` | Validates the assembled `AuditInput`, runs each evidence source in a fixed order, applies mode and grants, sorts and deduplicates findings, and creates `AuditReport`. |
| `audit/src/model.rs` | `AuditMode`, finding categories and dispositions, source/linker/ELF input records, grants, input validation, and report shape. |
| `audit/src/native.rs` | Explicit-scope traversal, source-kind selection, UTF-8 source loading, explicit ELF validation, and ELF fact extraction. |
| `audit/src/dependency.rs` | Cargo metadata v1 parsing, graph validation, root-reachable closure, and prohibited-package findings. |
| `audit/src/source.rs` | Lexical source/build/LLVM inspection and context-sensitive finding categories. |
| `audit/src/lexer.rs` | Comment and literal removal plus complete identifier, string, and LLVM `@` tokens. |
| `audit/src/artifact.rs` | Linker argument tokenization and inspection of supplied ELF facts. |
| `audit/src/policy.rs` | Compile-time exact symbol, library, and package classifiers. There is no runtime policy file to load. |
| `audit/src/error.rs` | User-facing error variants and their `Display` text. |

The crate manifest (`audit/Cargo.toml`) names the binary `recipe-audit` and the
library `recipe_audit`. `goblin` parses ELF files, while `serde` and
`serde_json` serialize reports and read Cargo metadata and grants. The parser is
deliberately handwritten; there is no `clap` or environment-based defaulting.
The binary imports only `AuditError`, `AuditInput`, `AuditMode`,
`DependencyGraph`, `LegacyGrant`, `LinkerInput`, and `collect_native_scope`,
then calls the library's `audit` function by its fully qualified name. All
source, dependency, linker, and ELF policy work is therefore reached through
that single library dispatch rather than duplicated in the CLI.

## End-to-end control flow

The real process path is:

```text
std::env::args_os()
        |
        v
Cli::parse
        |
        +-- parse error ------------------------------> stderr error + usage, exit 2
        |
        +-- help -------------------------------------> usage on stdout, exit 0
        |
        v
require --mode and --scope
        |
        +-- missing required value -------------------> stderr error + usage, exit 2
        |
        v
collect_native_scope(scope, --elf paths)
        |
        +-- collection/ELF error ---------------------> stderr error + usage, exit 2
        |
        v
optional bounded Cargo metadata + exact root IDs
        |
        +-- read/parse/graph error -------------------> stderr error + usage, exit 2
        |
        v
mode-dependent optional/required legacy grants
        |
        +-- grant read/parse/mode error --------------> stderr error + usage, exit 2
        |
        v
convert repeated --link-input values to LinkerInput
        |
        v
recipe_audit::audit(AuditInput)
        |
        +-- invalid input/policy/grant error ---------> stderr error + usage, exit 2
        |
        v
serde_json::to_string_pretty(AuditReport)
        |
        +-- serialization error ----------------------> stderr error + usage, exit 2
        |
        v
JSON report on stdout
        |
        +-- report.passed == true --------------------> exit 0
        +-- report.passed == false -------------------> exit 1
```

`main` (lines 9-18) owns the last error boundary. `run` propagates every
`AuditError` with `?`; it never catches an error to turn it into a report. The
only non-error failure result is a valid report whose `passed` field is false.

## CLI grammar and parser behavior

`usage()` (lines 214-219) is the sole help text:

```text
recipe-audit --mode next|legacy --scope ABSOLUTE_DIR
    [--metadata ABSOLUTE_CARGO_METADATA_JSON --package-id EXACT_ID ...]
    [--link-input EXACT_LINKER_ARGUMENT ...] [--elf ABSOLUTE_ELF ...]
    [--legacy-grants ABSOLUTE_JSON]
legacy mode requires exact grants; next mode rejects them
```

`Cli` (lines 126-136) stores:

| Option | Stored value | Cardinality and semantic requirement |
| --- | --- | --- |
| `-h`, `--help` | `bool` | Repeatable. If true after parsing, the binary prints usage and exits successfully before requiring mode or scope. |
| `--mode VALUE` | `Option<AuditMode>` | At most once. `VALUE` is exactly `next` or `legacy`, case-sensitive. Required unless help was requested. |
| `--scope PATH` | `Option<PathBuf>` | At most once. Required unless help was requested, and later required to be an absolute real directory. |
| `--metadata PATH` | `Option<PathBuf>` | At most once. If present, at least one `--package-id` is required. The file must be absolute, regular, and at most 64 MiB. |
| `--package-id ID` | `Vec<String>` | Repeatable. Each value must be nonempty outside help mode. IDs are passed exactly as caller supplied. |
| `--link-input ARG` | `Vec<String>` | Repeatable. Each value must be nonempty outside help mode. Values are not interpreted by the parser and may begin with `-`. |
| `--elf PATH` | `Vec<PathBuf>` | Repeatable. Each path is checked later as an absolute real file under the canonical scope. |
| `--legacy-grants PATH` | `Option<PathBuf>` | At most once. Forbidden in `next`; required in `legacy`; the JSON file is bounded-read and parsed later. |

The parser consumes the first `OsString` as the program name and processes the
rest one token at a time. Every option name and every option value must be
valid UTF-8. A non-UTF-8 argument produces a configuration error. Only the
spelled forms above are recognized: `--mode=next` and other `--option=value`
forms are unknown arguments. There is no `--` end-of-options mode, although a
value consumed by `--link-input` can itself be `--`.

Singleton options call `reject_duplicate` before consuming their value, so a
second `--mode`, `--scope`, `--metadata`, or `--legacy-grants` fails with a
configuration error. Repeated package IDs, linker inputs, ELF paths, and help
flags are accepted by parsing; later stages validate the relevant facts. A
missing value is reported as `OPTION requires a value`, and a non-UTF-8 value
has the option-specific `OPTION value must be valid UTF-8` message. Unknown
tokens fail immediately.

After all tokens are consumed, non-help parsing rejects empty package IDs and
empty linker inputs. Help short-circuits this final check, but it does not
short-circuit token parsing: `--help --unknown` still fails, while
`--help --mode legacy` prints usage without requiring a grant file. Help also
does not invoke filesystem collection, metadata reads, policy evaluation, or
JSON report serialization.

`AuditMode::from_str` returns a configuration error for every value other than
`next` and `legacy`. The mode is serialized later as the kebab-case strings
`"next"` and `"legacy"`.

## `run` preparation order

Once parsing has succeeded without help, `run` performs these operations in
this exact order. Earlier failures mask later inputs because each step returns
immediately on error.

1. It extracts `parsed.mode`, otherwise returning `--mode is required`.
2. It extracts `parsed.scope`, otherwise returning `--scope is required`.
3. It calls `collect_native_scope(&scope, &parsed.elf_paths)`. This always
   happens before metadata or grant handling, even when no source files or ELF
   paths were requested.
4. If `--metadata` was supplied, it first requires at least one package ID,
   reads the bounded absolute text file, and calls
   `DependencyGraph::from_cargo_metadata_json`. Without metadata, any package
   ID is an error and the dependency field is `None`.
5. It resolves grants from `(mode, parsed.legacy_grants)`: `next` plus no path
   becomes an empty vector, `next` plus a path is rejected, `legacy` without a
   path is rejected, and `legacy` with a path reads and deserializes a
   `Vec<LegacyGrant>`.
6. Every linker argument becomes `LinkerInput::new("<command-line>", 0,
   argument)`. The CLI has no source line for an argument, so all such findings
   use the synthetic path `<command-line>` and line zero.
7. It constructs `AuditInput` from the collected sources and ELF facts,
   optional dependency graph, adapted linker inputs, selected mode, and grant
   vector, then calls `recipe_audit::audit`.
8. It pretty-serializes the resulting `AuditReport`, prints the complete JSON
   document to stdout, and chooses exit 0 or 1 from `report.passed`.

There is no implicit current directory, Cargo invocation, metadata discovery,
artifact discovery, or policy file lookup. The CLI README's example therefore
requires callers to produce Cargo metadata and pass the exact root package IDs
themselves.

### Bounded JSON reads

`read_bounded_text` (lines 99-124) is used for both Cargo metadata and legacy
grants. It requires an absolute `PathBuf`, calls `fs::metadata`, requires a
regular file, and rejects files larger than `MAX_JSON_BYTES`, which is exactly
64 MiB (`64 * 1024 * 1024`). It then uses `fs::read_to_string`, so invalid UTF-8
is reported as an I/O error. The function does not require the JSON file to be
inside the source scope and does not use `symlink_metadata`; a symlink that
resolves to a regular file is accepted here. A missing file, permissions error,
directory, oversize file, or invalid UTF-8 is an error and produces exit 2.

Metadata parse errors are `InvalidCargoMetadata`. Grant deserialization errors
are wrapped as a configuration message containing the grant path. The later
library validation can still reject structurally valid JSON grants for
wildcards, duplicate keys, non-normalized paths, or stale entries.

## Native scope collection

`collect_native_scope` in `native.rs` receives the required scope and all
explicit `--elf` paths. Its limits are 16 MiB per collected text file, 1 GiB
per ELF file, and 100,000 collected source files.

### Scope and source traversal

The scope must be absolute. `symlink_metadata` rejects a symlink scope and any
scope that is not a directory. The real directory is canonicalized once. A
recursive `read_dir` traversal sorts directory entries by filename before
descending. Every symlink encountered during this traversal is rejected,
including symlinks whose contents would otherwise be unsupported. Directories
named `.git` and `target` are skipped exactly, at every depth, so their
contents are not traversed or inspected. Other directories are visited. The
source count limit is checked during recursion and again after collection.
The selected scope root itself is not subject to this child-name skip, so
selecting a directory whose own name is `target` still permits explicit ELF
inspection and traverses its children for any supported source suffixes.

Only these files become `SourceUnit`s, with extension matching case-sensitively:

| File suffix or exact name | `SourceKind` |
| --- | --- |
| `.rs` | `Rust` |
| `.zig` | `Zig` |
| `.c`, `.h` | `C` |
| `.cc`, `.cpp`, `.cxx`, `.hh`, `.hpp`, `.hxx` | `Cpp` |
| `.ll` | `LlvmIr` |
| `.toml`, `.json`, `.yaml`, `.yml`, `.cmake`, `.mk`, `.ninja`, `.bazel`, `.bzl` | `BuildMetadata` |
| `Cargo.lock`, `Makefile`, `CMakeLists.txt`, `BUILD`, `BUILD.bazel`, `WORKSPACE`, `WORKSPACE.bazel` | `BuildMetadata` |

Files with other suffixes are ignored. A non-UTF-8 filename cannot yield a
source kind and is ignored; a supported file with invalid UTF-8 contents fails
with an I/O error. Each supported file must be no larger than 16 MiB and is
read completely into a `String`.

The collector reports source paths relative to the canonical scope, replacing
backslashes with `/`. The resulting `SourceUnit`s are sorted by their
canonical path before being returned. The relative path is the stable path
used in source findings and legacy-grant keys. It is not an absolute host path.

### Explicit ELF paths

The `--elf` list is independent of source discovery. A caller may explicitly
name an artifact under a skipped `target` directory, but every path must be
absolute, must be a real regular file according to `symlink_metadata`, and
must not itself be a symlink. The path is canonicalized and must remain under
the canonical scope. Canonical duplicate paths are rejected. This containment
check happens before parsing, so an artifact outside the scope is a
configuration error even if it is a valid ELF file.

Each accepted artifact is parsed by `read_elf_facts_with_display` using
`goblin::elf::Elf`. Files larger than 1 GiB, nonregular files, unreadable files,
and malformed ELF data produce `InvalidElf` or `Io` errors. `DT_NEEDED` library
names are copied, sorted, and deduplicated. Both the dynamic symbol table and
the regular symbol table are scanned for `SHN_UNDEF` entries with nonzero names;
valid names are combined in a `BTreeSet`, so an undefined symbol appears once
even when both tables contain it. ELF facts use the scope-relative display path
and line zero for findings. The final ELF fact list is sorted by display path.

The public `read_elf_facts` function follows the same parser and size rules for
library callers, but uses the normalized path string supplied by the caller as
the display path. The CLI always uses the scope-relative form above.

## Cargo dependency evidence

When metadata is present, `DependencyGraph::from_cargo_metadata_json` expects
Cargo `--format-version 1` JSON. It requires:

- a nonempty caller-provided root ID list;
- a top-level JSON object with numeric `version` exactly `1`;
- a `packages` array whose entries are objects with nonempty string `id`,
  `name`, and `manifest_path` fields;
- a `resolve` object with a `nodes` array;
- one unique `resolve.nodes[].id` string per node; and
- a `dependencies` array of string package IDs on every resolve node.

The parser converts each node dependency into a directed package-to-dependency
edge. A `BTreeSet` removes duplicate edges while reading JSON. It preserves the
caller-selected root IDs exactly, rather than deriving roots from metadata.
The graph is then validated. Every root must be a package, every edge endpoint
must be a package, package IDs must be unique and nonempty, and package names
and manifest paths must be nonempty. Programmatically built graphs reject
duplicate edges; duplicate root IDs are not separately rejected and simply do
not change breadth-first traversal.

`DependencyGraph::audit` builds sorted adjacency lists, walks the complete
root-reachable closure with a queue and a visited set, and classifies only
packages in that closure. A package whose name belongs to a prohibited policy
family yields one blocking `dependency` finding with its manifest path, line
zero, and exact package name. Unreachable prohibited packages do not appear in
the report. Findings are sorted and deduplicated before being returned.

The dependency classifier lowercases names and changes `_` to `-`. Exact
`cuda-driver`, `cuda-driver-sys`, `hsa`, `hsa-sys`, `rocr`, and `rocr-sys` are
allowed. HIP families, CUDA Runtime and `cuda-runtime` forms, direct KFD
families, and the operation-library families are prohibited when their package
suffix is one of the recognized exact forms such as `-sys`, `-bindings`,
`-runtime`, `-static`, or `-wrapper`. Unknown package names are ignored.

## Source, build, and LLVM evidence

Each collected `SourceUnit` is passed to `audit_source` in the order returned by
the collector. The source auditor first checks for the exact path
`audit/src/policy.rs` and returns no source findings for that one path. This is
the self-hosting exception for the policy dictionary: the prohibited spellings
in that file define the classifier rather than represent runtime use. The
compiled auditor is still covered by dependency, linker, and ELF evidence.

The path comparison is literal. Because native collection reports paths
relative to the selected scope, a repository-root scope yields
`audit/src/policy.rs` and activates the exception, while a scope rooted at
`audit` yields `src/policy.rs` and does not. A library caller can also construct
the exact path directly with `SourceUnit::new`.

### Lexer

`lexer::lex` scans UTF-8 bytes and emits only complete identifiers, strings,
and `@` markers, all with one-based start lines. It removes `//` comments,
nested `/* ... */` comments, LLVM `;` comments, and build-metadata `#` comments.
It recognizes normal double-quoted strings for every source kind, character
literals for all non-build kinds, Rust lifetimes, and Rust raw strings with any
number of `#` delimiters. Identifier starts are ASCII letters, `_`, `$`, or
`.`. Continuations add ASCII digits, `_`, `$`, `.`, or `-`.

An unterminated nested block comment, normal string, character literal, or Rust
raw string is a lexical error. `audit_source` wraps it as
`AuditError::Lexical { path, line, reason }`; no partial findings are returned.

### Identifier and string rules

For non-LLVM units, every identifier is sent to `push_interface_finding` and
every string to `audit_string`. LLVM units receive a declaration/call pass
first, then the generic pass. In the generic LLVM pass, the identifier or string
immediately following an `@` on the same line is skipped because the LLVM
pre-pass already classifies that symbol. Other tokens are still inspected.

`push_interface_finding` first calls `classify_interface_symbol`. Build metadata
uses `classify_dependency` as a fallback when the symbol classifier returns
unknown. Prohibited Driver-style symbols outside the reviewed allowlist and
direct KFD symbols become `disallowed-native-interface`; every other prohibited
symbol is `build-link-input` for build metadata and `source-api` for source
languages. Allowed and unknown symbols produce no finding.

`audit_string` trims whitespace and removes trailing LLVM-style `\\00` and
`\\0` escapes. A string is in link context if its source kind is
`BuildMetadata`, or its source line contains one of
`rustc-link-lib`, `rustc-link-arg`, `#[link`, `target_link_libraries`,
`linkSystemLibrary`, or `-l`. Link-context strings are classified as build
metadata. If a prohibited token is found, the category is `build-link-input`.
Otherwise, include markers `#include`, `@import`, and `@cImport` select
`source-api`; all other prohibited strings select `runtime-load`.

`string_has_prohibited_token` first classifies the complete string as a library
name and, for build metadata, as a package name. It then splits on slash,
backslash, `=`, comma, colon, parentheses, brackets, semicolon, and ASCII
whitespace. Each nonempty component is classified as a symbol, with additional
library and dependency classification for build metadata. This catches paths,
linker declarations, and compound load strings without using arbitrary
substring search.

### LLVM declaration and call categories

For each `@` token in LLVM IR, `audit_llvm` examines the next identifier or
string and scans preceding tokens on the same line. If it sees `declare`, the
symbol is categorized as `llvm-declaration`. Otherwise, `call`, `invoke`, or
`callbr` selects `llvm-call`. An `@` without one of these same-line markers is
not categorized by this pass. The symbol is then sent through the same policy
classifier, so allowed or unknown names remain absent and prohibited names are
reported with the selected LLVM category, except for the special
`disallowed-native-interface` category.

## Compile-time native policy

`policy.rs` contains the only policy dictionary. It is compiled into the
library; there is no TOML, JSON, environment variable, or path-based policy
loading in `main.rs`.

`NativeInterface` names the accepted and prohibited families. The exact
`CUDA_DRIVER_API_ALLOWLIST` contains reviewed CUDA Driver ABI spellings,
including versioned `_v2` entries. `classify_interface_symbol` strips an
optional leading U+0001 decoration and everything after the first `@` version
suffix before classification. Its ordered rules are:

1. `hsaKmt...` and `kfd_...` are prohibited direct KFD symbols.
2. `hsa_...` is allowed ROCr/HSA.
3. A symbol in the exact CUDA Driver allowlist is allowed.
4. A `cu` symbol with an uppercase third byte, excluding names beginning with
   `cuda`, is prohibited as a Driver-shaped symbol outside the allowlist.
5. HIP-shaped symbols are prohibited.
6. CUDA Runtime-shaped symbols are prohibited.
7. Operation-library symbol families are prohibited; unknown symbols remain
   unknown.

The operation families are `rocblaslt`, `rocblas`, `rocsolver`, `rocfft`,
`miopen`, `rccl`, `cublas`, `cusolver`, `cufft`, `cudnn`, and `nccl`. HIP
families include `hip`, `hipblas`, `hipcub`, `hipfft`, `hiprand`, `hiprtc`,
`hipsolver`, and `hipsparse`.

`classify_library` first normalizes a complete library token: trim whitespace
and surrounding quotes/brackets, keep the basename after `/` or `\\`, remove
`-l` or `/DEFAULTLIB:`, remove a leading `lib`, lowercase, stop at the first
`.so`, `.dylib`, `.dll`, `.a`, or `.lib`, and trim a `-static` suffix. Exact
`cuda`/`nvcuda` and `hsa-runtime64`/`hsa-runtime` are allowed. `hsakmt`/`kfd`,
`amdhip64` and HIP families, `cudart`, and recognized operation-library stems
are prohibited. Unrecognized stems are unknown. Family matching admits only
the explicitly coded static, `lt`, or numeric suffix forms, not arbitrary
prefix matches.

The dependency classifier is separate because Cargo names have different
boundaries. It lowercases and normalizes underscores, allows the exact driver
and ROCr package names, and recognizes only its explicit package suffix forms.

## Linker and ELF artifact audits

`audit_linker_inputs` examines every adapted command-line or injected linker
input. `linker_candidates` includes the whole trimmed argument and all
nonempty components split on whitespace, comma, `=`, colon, parentheses,
brackets, and semicolon, with surrounding quotes and braces removed. Candidates
are sorted and deduplicated. A candidate is blocking when either the library or
symbol classifier marks it prohibited. The finding category is
`build-link-input`, and the input's path and line are preserved.

`audit_elf_facts` classifies each `needed` library as a normalized library name.
Prohibited names become `dynamic-needed` findings at the ELF path and line
zero. It then classifies each deduplicated undefined symbol. Any prohibited
symbol becomes an `undefined-symbol` finding, also at line zero. Allowed and
unknown names do not appear. Both audit functions sort and deduplicate their
own results; the library performs another global sort and deduplication after
all evidence sources have run.

## Library audit order, validation, and grants

`recipe_audit::audit` begins with `AuditInput::validate`, before any source or
dependency processing. It rejects duplicate source paths and duplicate ELF
paths, empty or non-slash-normalized source/linker/ELF paths, empty linker
arguments, and empty ELF library or symbol names. It does not re-read files or
validate source text because the CLI collector has already produced `String`s.

After validation, the fixed evaluation sequence is:

1. `audit_source` for each source;
2. `DependencyGraph::audit` if a graph is present;
3. `audit_linker_inputs` for all linker inputs; and
4. `audit_elf_facts` for each ELF fact.

The combined `Finding` vector is sorted by its derived ordering
(category, path, line, symbol, disposition) and deduplicated. In `next`, a
nonempty grant vector is rejected even though the CLI already rejects the
corresponding option. In `legacy`, `apply_legacy_grants` validates every grant,
indexes exact `(category, path, line, symbol)` keys, rejects duplicate keys,
marks matching findings `grandfathered`, and rejects every indexed grant that
was not used. A grant path must be nonempty, slash-normalized, and free of `*`
and `?`; its symbol must be nonempty and free of `*`. The implementation does
not reject `?` in a grant symbol, because only wildcard `*` is checked there.

Unused grants are reported as one configuration error listing each exact stale
key. A grant can only grandfather a finding with the same category, normalized
path, line, and symbol. There is no blanket path exception, wildcard matching,
or grant that changes policy classification.

`AuditReport::new` sets `passed` to true only when every finding has
`grandfathered` disposition. Therefore an empty finding vector passes in either
mode, a next-mode finding always fails, and a legacy report passes only when all
findings were matched by valid grants. The report retains all findings,
including grandfathered ones, so the JSON result records what was exempted.

## Output and exit contract

On a valid audit, `run` serializes `AuditReport` with
`serde_json::to_string_pretty` and prints it once to stdout. The JSON fields are:

```json
{
  "mode": "next" | "legacy",
  "passed": true | false,
  "findings": [
    {
      "category": "dependency" | "source-api" | "build-link-input" |
        "dynamic-needed" | "llvm-declaration" | "llvm-call" |
        "undefined-symbol" | "runtime-load" |
        "disallowed-native-interface",
      "path": "slash/normalized/path",
      "line": 0,
      "symbol": "exact triggering token",
      "disposition": "blocking" | "grandfathered"
    }
  ]
}
```

Text findings use one-based lines. Dependency, linker-command, and ELF facts
use line zero. `path` is scope-relative for collected source and ELF facts,
`<command-line>` for CLI linker arguments, and caller-supplied normalized text
for injected library inputs. The report ordering follows the Rust derived
ordering, whose category precedence is the declaration order
`dependency`, `source-api`, `build-link-input`, `dynamic-needed`,
`llvm-declaration`, `llvm-call`, `undefined-symbol`, `runtime-load`, then
`disallowed-native-interface`; path, line, symbol, and disposition break ties.

The process codes are intentionally distinct:

| Condition | Stdout | Stderr | Exit |
| --- | --- | --- | --- |
| Help after successful parsing | Usage | Empty | 0 |
| Valid audit with no blocking findings | Pretty report | Empty | 0 |
| Valid audit with one or more blocking findings | Pretty report | Empty | 1 |
| Any parse, path, I/O, lexical, metadata, graph, grant, input-validation, or serialization error | None | `recipe-audit: ERROR` followed by usage | 2 |

The error prefix and usage are emitted by `main`, not by the library. The
library itself returns typed `AuditError` values and never prints. Report
serialization is the last fallible step, although the report's fields are all
serializable by construction.

The exit-2 table covers every `AuditError` returned through `run`. The
`println!` and `eprintln!` calls themselves are not fallible return paths: a
broken stdout or stderr stream can make the standard formatting macros panic
instead of producing one of the documented `ExitCode` values. Normal terminal
and pipe operation follows the table above.

## Direct observations of the production entrypoint

The following checks were run against the built `target/debug/recipe-audit`
binary, using the same command-line boundary a caller uses:

- `--help` printed the usage text and returned 0 without a scope.
- `--mode next` returned 2, printed `--scope is required`, and printed usage.
- `--mode next --scope /home/nate/Desktop/nates-recipe-rs/audit/.docs`
  returned a report with `mode: "next"`, `passed: true`, no findings, and exit
  0. The scope contains no collected source kinds.
- Adding `--link-input -lcudart` to that empty source scope returned exit 1 and
  exactly one blocking `build-link-input` finding with path
  `<command-line>`, line 0, and symbol `-lcudart`.
- `--metadata relative --package-id root` returned exit 2 with the absolute
  JSON path requirement. `--metadata` without a package ID returned exit 2
  before any JSON read.
- `--mode legacy` without `--legacy-grants` returned exit 2 with the explicit
  grant requirement. `--mode next --legacy-grants ...` returned exit 2 because
  next mode rejects grants.
- Using `/etc/hosts` as an absolute metadata file returned the typed `invalid
  Cargo metadata` error and exit 2. Using the same non-JSON file as a legacy
  grant file returned the path-qualified `invalid legacy grants` configuration
  error and exit 2. A directory passed as JSON input was rejected by the
  regular-file check before reading.
- Scoping the built `target/debug` directory itself and explicitly naming the
  `recipe-audit` ELF produced a clean report, confirming that an artifact below
  a directory normally skipped during recursive source collection is still
  parsed when named with `--elf`.
- `--help --unknown` returned exit 2, demonstrating that help short-circuits
  semantic execution only after the complete token parser has accepted every
  option.

These observations confirm that the documented output and failure branches are
the actual production entrypoint behavior, not an internal helper or a test
double.

### Small-input edge cases preserved by the implementation

Several values are checked for exact emptiness rather than whitespace or
semantic usefulness. A linker argument containing only spaces is accepted by
`Cli::parse`; it becomes a nonempty `LinkerInput`, then trims to no candidates
and contributes no finding. A package ID containing spaces is passed to Cargo
metadata graph lookup unchanged. A repeated package ID is also passed twice as
a root; breadth-first traversal visits it once. These are consequences of the
explicit parser and graph contracts, not hidden normalization in `main`.

`--link-input` consumes its next token even when that token starts with a dash,
so `--link-input -lcudart` is the supported spelling. An argument written as
`--link-input=-lcudart` is an unknown option because the parser does not split
equals signs. There is no separate option terminator. The same rule means a
literal `--` can be consumed as a linker value and then produces no prohibited
candidate.

The parser validates singleton duplication before it consumes a duplicate's
value. Thus a duplicate `--mode` fails with the duplicate-option message even
if the following token would itself have been invalid. Conversely, a missing
value fails with the option-specific missing-value message before any later
semantic requirement is considered. A malformed scope can therefore mask
metadata, grants, and linker facts because scope collection is earlier in
`run`.

The public collector and public `read_elf_facts` function have slightly
different symlink boundaries. The CLI's explicit ELF path is rejected when
the path itself is a symlink. `read_elf_facts` only requires an absolute path
and a regular file after `fs::metadata`, so a symlink resolving to a regular
ELF is accepted by that library entry point. The report still records the
normalized path string passed by the library caller.

The LLVM pre-pass looks at the next token after `@` without requiring it to be
on the same line, while the generic-pass `follows_at` suppression requires the
same line. Well-formed LLVM keeps the symbol on the declaration or call line.
Malformed cross-line input can therefore be classified once by the pre-pass
and again by the generic identifier pass, with the normal global sort and
deduplication retaining both categories when they differ. An `@` that has no
same-line `declare`, `call`, `invoke`, or `callbr` marker is ignored by the
pre-pass, and its immediate same-line symbol is suppressed by the generic pass.

Line accounting is byte-scanner accounting. Newline bytes increment the line,
block comments and raw strings count internal newlines, and a backslash skips
the next byte in a normal string or character literal without incrementing the
line when that skipped byte is a newline. Consequently an escaped newline can
make later token lines differ from a physical line count, and link/include
context is still looked up against the source text line selected by that token
number.

## Operational implications

The caller must choose an explicit absolute scope that produces the desired
stable path namespace. The source-policy self-hosting exception is keyed to the
relative path `audit/src/policy.rs`, so a scope rooted at the repository is the
natural shape when the auditor is auditing this repository. A scope rooted at a
subdirectory changes every finding path and can change whether that exact
exception applies.

The caller must also pass every evidence class it wants checked. Source files
are collected automatically only for the supported extensions and only below
the scope, with `.git` and `target` omitted. Cargo dependency closure is never
inferred from the scope, and binaries are never discovered automatically. A
binary under `target` is audited only when supplied through `--elf`; it still
must be a real file and remain under the canonical scope. Likewise, a linker
argument is audited only when repeated `--link-input` values are supplied.

Finally, a clean JSON report means that the supplied and collected facts passed
the selected mode. It does not mean that omitted metadata, omitted linker
inputs, omitted ELF artifacts, unsupported file suffixes, or unreachable
dependency packages were proven clean. The explicit-input design keeps absent
evidence distinct from an observed allowed result.
