# `recipe-audit`

`recipe-audit` is Recipe's deterministic native-boundary gate. It evaluates
explicit source, build, dependency, linker, and ELF facts against one exact
policy and returns stable findings. A finding is evidence that a prohibited
native interface or vendor operation library is present. The gate is intended
for the replacement runtime (`next` mode) and for the separately governed
legacy boundary (`legacy` mode).

The crate is a fact auditor, not a build system. It does not invoke Cargo,
compile source, run a linker, inspect the current directory implicitly, load a
library, rewrite source, or execute a workload. Collection and policy are
separate: the library accepts injected facts, while the command-line binary
performs the opt-in filesystem and ELF collection needed for a production
invocation.

## Package and entrypoints

The package is the unpublished Rust 2024 workspace member `recipe-audit`
(`audit/Cargo.toml`). It exposes:

- the `recipe_audit` library target at `audit/src/lib.rs`;
- the `recipe-audit` binary target at `audit/src/main.rs`;
- `goblin` 0.10 with only ELF32, ELF64, and file-descriptor endianness
  features for ELF inspection;
- `serde` with derive support and `serde_json` for metadata, grants, and the
  report format.

The manifest forbids unsafe Rust and denies all Clippy and pedantic lints. It
has no feature flags, build script, Recipe-crate dependency, or implicit host
integration. The workspace root lists the package as a member, but the root
`recipe` package does not depend on it. A caller must run the binary or call
the library explicitly. The separate `kernel/src/audit.rs` check rejects
external LLVM declarations in generated kernels; this crate owns the broader
source, dependency, linker, dynamic-library, and final-ELF policy.

All implementation modules are private `mod` declarations; callers reach them
through the deliberate root re-exports only. The public library surface
re-exports the input model, report model, policy classifiers, source auditor,
dependency graph, native collectors, linker and ELF auditors, and `AuditError`.
The central entrypoint is:

```text
audit(AuditInput) -> Result<AuditReport, AuditError>
```

The public lower-level entrypoints are `audit_source`,
`audit_linker_inputs`, `audit_elf_facts`,
`DependencyGraph::from_cargo_metadata_json`, `collect_native_scope`, and
`read_elf_facts`. They allow a build or test harness to collect facts through
its own boundary and still use the same policy evaluator.

The remaining public constructors and queries are intentionally data-oriented:
`AuditInput::empty`, `DependencyPackage::new`, `DependencyEdge::new`,
`DependencyGraph::new`, `SourceUnit::new`, `LinkerInput::new`,
`ArtifactSymbol::new`, `ElfFacts::new`, `Finding::blocking`,
`LegacyGrant::new`, `AuditMode::as_str`, `FindingCategory::as_str`,
`InterfaceClassification::is_prohibited`, and
`AuditReport::blocking_count`. They do not perform host discovery or mutate
global policy. Structural graph, input, and grant validation remains at the
owning audit boundary described below.

## End-to-end data flow

The binary follows this order. Each step either produces an explicit fact or
returns an error; there is no fallback source of evidence.

```text
CLI arguments
    |
    +--> collect_native_scope(absolute scope, explicit ELF paths)
    |        +--> SourceUnit values for supported source/build files
    |        +--> ElfFacts values for named ELF files
    |
    +--> optional bounded Cargo metadata + exact package roots
    |        +--> DependencyGraph
    |
    +--> optional exact legacy grant JSON --> LegacyGrant values
    |
    +--> repeated linker arguments --> LinkerInput values
             |
             v
        AuditInput
             |
             v
        input validation
             |
             +--> source lexing and policy findings
             +--> reachable dependency closure findings
             +--> linker candidate findings
             +--> ELF DT_NEEDED and undefined-symbol findings
             |
             v
        sort and deduplicate findings
             |
             +--> next: reject any grant list
             +--> legacy: validate, apply, and account for exact grants
             |
             v
        AuditReport -> pretty JSON (CLI stdout) and exit status
```

The library performs the same evaluation after the caller has built an
`AuditInput`. `AuditInput::validate` checks unique source and ELF display
paths, normalized paths, nonempty linker arguments, and nonempty ELF library
and symbol names. It intentionally does not manufacture missing facts. An
optional dependency graph is audited only when supplied.

Every emitted finding starts as `blocking`. Findings are sorted using the
derived ordering and deduplicated before mode handling. In legacy mode an
exact matching grant changes only the disposition to `grandfathered`; the
category, normalized path, line, and symbol remain unchanged. A report passes
when every finding is grandfathered, which also means an empty finding set
passes. Any malformed input, graph, lexical unit, grant, ELF, or filesystem
operation returns an error instead of a report.

Consequently, after validation and deduplication, `next` passes exactly when
the finding set is empty. `legacy` passes exactly when the grant-key set equals
the finding-key set: every finding must be granted and every grant must be
used. A grant cannot make a report pass while another finding remains
unmatched.

### State ownership

The crate has no persistent global or process state. The CLI owns parsed
arguments and collected facts only for one `run` call; `AuditInput` then owns
those vectors and is consumed by `audit`. Policy dictionaries are compile-time
constants. Scanners allocate local finding vectors, the legacy pass is the only
stage that mutates a finding disposition, and `AuditReport` is the final
per-call value. Reusing the library means constructing a new input and report,
not resetting a cache or daemon.

## Command-line contract

The binary requires an explicit absolute scope and mode:

```text
recipe-audit --mode next|legacy --scope ABSOLUTE_DIR
    [--metadata ABSOLUTE_CARGO_METADATA_JSON --package-id EXACT_ID ...]
    [--link-input EXACT_LINKER_ARGUMENT ...] [--elf ABSOLUTE_ELF ...]
    [--legacy-grants ABSOLUTE_JSON]
```

Option values are separate arguments. The parser does not implement
`--option=value`; for example, use `--link-input '-Wl,-Bstatic,-lowned'`, not
`--link-input=-Wl,-Bstatic,-lowned`. Singleton options (`--mode`, `--scope`,
`--metadata`, and `--legacy-grants`) may occur once. `--package-id`,
`--link-input`, and `--elf` may repeat. Unknown options, missing values,
non-UTF-8 arguments, duplicate singleton options, and empty package IDs or
linker arguments are configuration errors. `-h` and `--help` print the usage
text when parsing succeeds. Help is not an early token-parser escape:
`--help --unknown` still errors, while `--help --mode legacy` prints usage
without requiring a scope or grant file.

There is no `--` end-of-options mode. A value consumed by `--link-input` may
begin with `-`, so `--link-input -lcudart` is valid and the next token is
consumed as its value.

The binary never runs `cargo metadata` itself. `--metadata` requires at least
one exact `--package-id`; a package ID without metadata is rejected. The
metadata file and legacy-grant file must each be absolute regular files no
larger than 64 MiB. `legacy` mode requires `--legacy-grants`, while `next`
mode rejects that option. Linker arguments become `LinkerInput` values with
path `<command-line>` and line `0`. ELF arguments are collected only when
explicitly named. Scope collection runs before metadata and grant loading, so
when several inputs are invalid the first collection failure is the one
returned. The bounded JSON reader uses ordinary metadata, so an absolute
symlink resolving to a regular file is accepted for metadata or grants; the
stricter symlink rejection applies to the native scope and explicit ELF
collector.

On success the binary serializes `AuditReport` as pretty JSON on stdout. The
JSON fields are `mode`, `passed`, and `findings`; each finding contains
`category`, `path`, `line`, `symbol`, and `disposition`, with kebab-case enum
values. Exit status `0` means the report passed, `1` means the report contains
one or more blocking findings, and `2` means an input, collection, policy, or
serialization error. Errors are printed to stderr followed by the usage text.

The real binary path was checked with an empty supported-file scope:

```text
cargo run -p recipe-audit -- \
  --mode next --scope /home/nate/Desktop/nates-recipe-rs/audit/.docs
```

It produced `{"mode":"next","passed":true,"findings":[]}` (pretty-printed)
and exited `0`. Adding `--link-input -lcudart` produced one blocking
`build-link-input` finding for `<command-line>`, line `0`, symbol `-lcudart`,
and exited `1`. These are production CLI observations, not calls to an
internal helper or a test double. Supplying the built auditor explicitly from
the otherwise source-excluded `target/debug` scope with `--elf` produced a
passing report with no findings, confirming the explicit ELF path reaches the
`goblin` fact reader without making `target` part of source traversal.

## Domain model

### Modes and findings

`AuditMode::Next` represents replacement code and permits no exceptions.
`AuditMode::Legacy` represents code that may carry individually reviewed
exceptions. Text parsing accepts exactly `next` and `legacy`; serialization
uses the same kebab-case strings.

`Finding` is the stable result record:

| Field | Meaning |
| --- | --- |
| `category` | One of the policy categories below. |
| `path` | Slash-normalized display path supplied by the input or collector. |
| `line` | One-based text line, or `0` for dependency-graph, command-line, and binary facts. |
| `symbol` | Exact triggering identifier, library, package name, or artifact symbol. |
| `disposition` | `blocking` until an exact legacy grant changes it to `grandfathered`. |

The categories are `dependency`, `source-api`, `build-link-input`,
`dynamic-needed`, `llvm-declaration`, `llvm-call`, `undefined-symbol`,
`runtime-load`, and `disallowed-native-interface`.

`LegacyGrant` has the same four identifying fields as a finding. Its JSON
decoder denies unknown fields. Grants require nonempty path and symbol, a
slash-normalized path, no `*` or `?` in the path, and no `*` in the symbol.
They are exact records, not path, line, or symbol patterns. Duplicate grants
are an error. Every supplied grant must match a finding, otherwise the whole
legacy audit fails with an unused-or-stale-grant error.

### Injected fact types

- `SourceUnit` contains a normalized display path, a `SourceKind`, and UTF-8
  contents. `SourceKind` is `Rust`, `Zig`, `C`, `Cpp`, `LlvmIr`, or
  `BuildMetadata`.
- `LinkerInput` contains a normalized path, a line number, and one complete
  linker argument or declarative linker value.
- `DependencyPackage` contains a Cargo package ID, package name, and
  normalized manifest path. `DependencyEdge` is a directed
  `package -> dependency` pair. `DependencyGraph` carries package records,
  edges, and exact root IDs.
- `ArtifactSymbol` stores one extracted symbol name. `ElfFacts` stores an
  artifact display path, `DT_NEEDED` library names, and undefined symbols.
- `AuditInput` groups the mode, source units, optional dependency graph,
  linker inputs, ELF facts, and legacy grants. `AuditInput::empty(mode)` is a
  valid empty starting point for library callers.
- `AuditReport` stores the selected mode, the boolean `passed` value, and the
  stable sorted findings. `blocking_count()` counts findings that still block
  the selected mode.

Constructors normalize backslashes to `/` only, but manually assembled public
structs are still validated by `audit`; `.` segments, duplicate separators,
and filesystem canonicalization are not removed by this display transform.
Normalization is display-only: the collector separately canonicalizes
filesystem paths before checking scope containment.

## Native-interface policy

The policy is deliberately exact. Source and artifact symbols are classified
after removing one leading symbol-decoration byte (`U+0001`) and truncating at
the first `@` version suffix. Library names are normalized to a basename and
stem before classification. Symbol classification runs in this order:
direct-KFD prefixes, allowed ROCr/HSA prefixes, the exact CUDA Driver list,
outside-allowlist `cuXxx` shape, HIP, CUDA Runtime, then vendor operation
families. Unknown values remain `Unknown` and are not reported.

### Allowed interfaces

- ROCr/HSA symbols beginning with `hsa_` are allowed. Library stems
  `hsa-runtime64` and `hsa-runtime` are allowed.
- CUDA Driver libraries with stems `cuda` or `nvcuda` are allowed.
- The CUDA Driver callable surface is an exact 89-entry allowlist exported as
  `CUDA_DRIVER_API_ALLOWLIST`. Versioned spellings such as `_v2` are separate
  entries. A new Driver API call is therefore a visible policy edit, not a
  prefix match. The current list covers context, device, driver-version,
  event, function, graph, launch, memory, copy, module, pointer, and stream
  operations present in `audit/src/policy.rs`:

  ```text
  cuCtxCreate, cuCtxCreate_v2, cuCtxDestroy, cuCtxDestroy_v2,
  cuCtxGetCurrent, cuCtxGetDevice, cuCtxPopCurrent, cuCtxPopCurrent_v2,
  cuCtxPushCurrent, cuCtxPushCurrent_v2, cuCtxSetCurrent,
  cuDeviceCanAccessPeer, cuDeviceComputeCapability, cuDeviceGet,
  cuDeviceGetAttribute, cuDeviceGetCount, cuDeviceGetName,
  cuDeviceGetPCIBusId, cuDeviceGetUuid, cuDeviceGetUuid_v2,
  cuDevicePrimaryCtxRelease, cuDevicePrimaryCtxRelease_v2,
  cuDevicePrimaryCtxRetain, cuDeviceTotalMem, cuDeviceTotalMem_v2,
  cuDriverGetVersion,
  cuEventCreate, cuEventDestroy, cuEventDestroy_v2, cuEventElapsedTime,
  cuEventQuery, cuEventRecord, cuEventSynchronize,
  cuFuncGetAttribute, cuFuncSetAttribute, cuGetErrorName, cuGetErrorString,
  cuGraphAddDependencies, cuGraphAddEmptyNode, cuGraphAddEventRecordNode,
  cuGraphAddEventWaitNode, cuGraphAddKernelNode, cuGraphAddMemcpyNode,
  cuGraphDestroy, cuGraphExecDestroy, cuGraphInstantiate,
  cuGraphInstantiate_v2, cuGraphLaunch,
  cuInit, cuLaunchKernel,
  cuMemAlloc, cuMemAllocHost, cuMemAllocHost_v2, cuMemAlloc_v2,
  cuMemFree, cuMemFreeHost, cuMemFree_v2, cuMemGetInfo, cuMemGetInfo_v2,
  cuMemHostAlloc, cuMemHostGetDevicePointer, cuMemHostGetDevicePointer_v2,
  cuMemHostRegister, cuMemHostRegister_v2, cuMemHostUnregister,
  cuMemcpyDtoDAsync, cuMemcpyDtoDAsync_v2, cuMemcpyDtoHAsync,
  cuMemcpyDtoHAsync_v2, cuMemcpyHtoDAsync, cuMemcpyHtoDAsync_v2,
  cuMemcpyPeerAsync,
  cuModuleGetFunction, cuModuleGetGlobal, cuModuleGetGlobal_v2,
  cuModuleGetLoadingMode, cuModuleLoad, cuModuleLoadData,
  cuModuleLoadDataEx, cuModuleUnload, cuPointerGetAttribute,
  cuStreamCreate, cuStreamCreateWithPriority, cuStreamDestroy,
  cuStreamDestroy_v2, cuStreamGetPriority, cuStreamQuery,
  cuStreamSynchronize, cuStreamWaitEvent
  ```

### Prohibited interfaces and libraries

- Direct KFD ownership is prohibited: symbols beginning with `hsaKmt` or
  `kfd_`, and library stems `hsakmt` or `kfd`.
- HIP is prohibited. Symbol families include `hip`, `hipblas`, `hipcub`,
  `hipfft`, `hiprand`, `hiprtc`, `hipsolver`, and `hipsparse`, plus the exact
  names `hip`, `hipcc`, `hipconfig`, `hiprtc`, and `amdhip64`. Library names
  from those families and `amdhip64` are prohibited.
- The CUDA Runtime API is prohibited. Symbols with the `cuda` followed by an
  uppercase letter shape, plus `cudart`, `cuda_runtime`, and
  `cuda_runtime_api`, are prohibited. Library stems in the `cudart` family
  are prohibited.
- Driver-style symbols shaped like `cuXxx` that are not in the reviewed
  allowlist are classified as `CudaDriverOutsideAllowlist` and prohibited.
- Vendor operation libraries are prohibited in both AMD and NVIDIA forms:
  `rocblaslt`, `rocblas`, `rocsolver`, `rocfft`, `miopen`, `rccl`, `cublas`,
  `cusolver`, `cufft`, `cudnn`, and `nccl`. The corresponding symbol families
  are matched case-insensitively when the suffix is empty, uppercase,
  underscore-prefixed, or digit-prefixed.

`NativeInterface` records the reason without changing the public finding shape:
allowed classifications are `RocrHsa` and `CudaDriver`; prohibited
classifications are `Hip`, `CudaRuntime`, `DirectKfd`, `RocBlas`, `RocSolver`,
`RocFft`, `MiOpen`, `Rccl`, `CuBlas`, `CuSolver`, `CuFft`, `CuDnn`, `Nccl`,
and `CudaDriverOutsideAllowlist`. `InterfaceClassification` wraps one of those
as `Allowed(...)` or `Prohibited(...)`, or returns `Unknown`; consumers use
`is_prohibited()` rather than duplicating the dictionary.

Library normalization recognizes `-lfoo`, `libfoo`, path basenames, and
`.so`, `.dylib`, `.dll`, `.a`, or `.lib` suffixes. Linker and source candidate
splitting also exposes the library token in `/DEFAULTLIB:foo` values. The
direct library classifier strips `/DEFAULTLIB:` only when that prefix is still
present after basename extraction, so a leading slash is normally removed
before this branch; the downstream split is the reliable `/DEFAULTLIB:` path.
It strips a `-static` ending and recognizes exact, `_static`, `-static`, `lt`,
`lt_static`, numeric `64_...`, and numeric `_...` family forms. This is still a
named family check, not an arbitrary substring search.

Cargo package classification lowercases the name and changes `_` to `-`.
`cuda-driver`, `cuda-driver-sys`, `hsa`, `hsa-sys`, `rocr`, and `rocr-sys`
are allowed package names. Prohibited package families accept only the
explicit suffixes `-sys`, `-src`, `-bindings`, `-runtime`, `-runtime-sys`,
`-static`, and `-wrapper`, in addition to the bare family. Dependency
classification is based on package name, not an arbitrary manifest or source
substring.

The same policy therefore routes equivalent evidence by its boundary rather
than by the spelling alone:

| Evidence | Result |
| --- | --- |
| Rust identifier `hipMalloc` | `source-api` |
| Build-metadata identifier or string `hip` | `build-link-input` |
| Source identifier `hsaKmtOpenKFD` or non-allowlisted `cuNewApi` | `disallowed-native-interface` |
| Rust string `libcudart.so` used as a load value | `runtime-load` |
| `#[link(name = "cudart")]` string | `build-link-input` |
| LLVM `declare ... @hipFoo` | `llvm-declaration` |
| LLVM `call ... @hipFoo` | `llvm-call` |
| Linker candidate `-lcudart` | `build-link-input` |
| ELF `DT_NEEDED` value `libcudart.so` | `dynamic-needed` |
| ELF undefined symbol `cudaMalloc` | `undefined-symbol` |
| Reachable Cargo package `hip-sys` | `dependency` |

The two native-interface cases in the table override their ordinary source or
LLVM category because the policy treats direct KFD ownership and an
unreviewed Driver symbol as boundary violations, not merely API uses.

## Source and lexical audit

`audit_source` lexes one `SourceUnit` and never scans arbitrary substrings.
Comments and literals are removed or represented as tokens before policy
classification. Unterminated comments or literals are errors because
incomplete lexical evidence cannot pass the gate.

The lexer emits complete `Identifier`, `String`, and `At` tokens with source
line numbers. It supports:

- `//` line comments and nested `/* ... */` block comments for all kinds;
- `;` comments for LLVM IR and `#` comments for build metadata;
- normal double-quoted strings and non-build single-quoted literals, with
  escaped bytes skipped;
- Rust lifetimes and Rust raw strings with any number of `#` delimiters;
- ASCII identifiers beginning with a letter, `_`, `$`, or `.`, continuing
  with letters, digits, `_`, `$`, `.`, or `-`.

Raw-string closing checks require at least the opening hash count, so a longer
run of closing `#` characters can terminate the token at the first required
count. The lexer does not implement Rust byte-string raw forms or validate
language grammar beyond lexical termination.

Line numbers count LF bytes only. CRLF advances at the LF, standalone CR and
other Unicode separators do not advance the counter, and a backslash escape in
a normal literal skips its following byte without counting an escaped LF.

Build metadata does not treat apostrophes as literal delimiters, so apostrophe
content remains ordinary punctuation and identifiers for that lexical mode.

For non-LLVM source, prohibited complete identifiers become `source-api`
findings, except that build metadata identifiers become `build-link-input`.
Allowed and unknown identifiers produce no finding. Build metadata also tries
Cargo package classification when symbol classification is unknown.

Strings are trimmed, including a trailing `\\00` or `\\0`, and then checked as
whole library values and as components split on path, assignment, punctuation,
and whitespace delimiters. A prohibited string is categorized as follows:

- `build-link-input` when the unit is build metadata or the source line has a
  linker marker (`rustc-link-lib`, `rustc-link-arg`, `#[link`,
  `target_link_libraries`, `linkSystemLibrary`, or `-l`);
- `source-api` when the line has `#include`, `@import`, or `@cImport` context;
- `runtime-load` otherwise, covering string-based library and dynamic-load
  values.

Context markers are raw substring checks on the original source line, not
token-aware directive parsing. A marker in unrelated text or a comment can
reclassify a string, and link context wins before include context.

LLVM IR receives two additional categories. For each `@symbol` token, the
same-line prefix is inspected for `declare`, `call`, `invoke`, or `callbr`.
`declare` takes priority over a call marker, producing `llvm-declaration`;
otherwise a call marker produces `llvm-call`. LLVM symbols after `@` are not
then double-counted by the generic identifier/string pass. An `@symbol` with
neither marker is not reported by this LLVM-specific pass. As with all
identifier findings, a prohibited direct-KFD or outside-allowlist Driver
symbol uses `disallowed-native-interface` instead of the normal LLVM category.
The pre-pass pairs `@` with the immediate next token even across a newline,
while generic-pass suppression requires the same line. Malformed cross-line IR
can therefore produce both an LLVM-category finding and a generic finding.

The exact normalized path `audit/src/policy.rs` is exempted from source
findings because it is the self-hosted policy dictionary, not runtime use of
the prohibited names. Dependency, linker, and ELF evidence for the compiled
auditor is still audited. This exception is path-sensitive: collecting the
`audit` directory as the scope yields `src/policy.rs`, so it does not match
the repository-root spelling and will be scanned normally. The matching return
happens before lexing and ignores the unit's `SourceKind` and contents.

`audit_source` itself does not validate path normalization, uniqueness, scope
membership, or file size. Those invariants belong to `AuditInput::validate` and
`collect_native_scope`; a direct library caller can intentionally audit one
in-memory unit with a different display path.

## Dependency graph audit

`DependencyGraph::from_cargo_metadata_json` consumes Cargo metadata format
version `1` and requires at least one exact root package ID. The top-level
value must be an object with a numeric `version: 1`, a `packages` array, and a
`resolve` object containing a `nodes` array. Every package must provide
nonempty string `id`, `name`, and `manifest_path` fields. Every resolve node
must provide a unique nonempty `id` and a `dependencies` array whose members
are nonempty package-ID strings. A missing resolve graph is rejected as the
equivalent of `--no-deps`, because a partial graph cannot establish a safe
closure. The parser uses the caller's root list rather than Cargo's optional
`resolve.root` field, and ignores unrelated metadata fields.
It does not add a duplicate-object-key check beyond `serde_json`'s normal
object parsing semantics.

The required `nodes` value may technically be an empty array, and the parser
does not require one node for every package record. Such metadata can validate
with a root package and produce a root-only closure, so callers remain
responsible for supplying Cargo's complete resolve graph when they want
dependency evidence rather than a syntactically valid partial graph.

Parsed edges are inserted into a `BTreeSet`, so duplicate dependency entries
in Cargo JSON collapse during parsing. A graph constructed directly through
`DependencyGraph::new` is validated strictly: roots and edge endpoints must
exist, package IDs must be unique and nonempty, and duplicate edges are an
`InvalidDependencyGraph` error. Manifest paths are normalized by the package
constructor but are not otherwise required to be absolute. Repeated root IDs
are accepted and collapse during the visited-set traversal. Parsed JSON that
passes field-shape checks but has a missing root or dangling edge can likewise
return `InvalidDependencyGraph` from the shared validator.

Auditing builds a sorted adjacency map and breadth-first traverses the full
reachable closure from the selected roots. Only reached package names are
classified. Each prohibited package produces one `dependency` finding at its
manifest path with line `0` and the package name as the symbol. Unreachable
packages do not affect the report.

## Linker and ELF artifact audit

`audit_linker_inputs` accepts declarative or command-line strings. For every
input it checks the complete trimmed argument and deterministic candidates
split on whitespace and `,`, `=`, `:`, `(`, `)`, `[`, `]`, and `;`, with quote
and brace characters trimmed from each candidate. A prohibited library or
symbol produces a `build-link-input` finding at the input's supplied path and
line. This fact-level auditor deliberately reports all linker hits under that
category, including direct-KFD and outside-allowlist hits.

Structural validation rejects an exactly empty linker argument, but a
whitespace-only argument is nonempty, trims to an empty whole candidate, and
normally yields no finding. The artifact helper itself has no separate size or
content limit; the caller-supplied slice and strings are its complete input.

`audit_elf_facts` checks two independent fact sets:

- prohibited `DT_NEEDED` library names become `dynamic-needed` findings;
- prohibited undefined dynamic or static symbols become `undefined-symbol`
  findings.

Both artifact categories use line `0`, preserve the ELF display path, and are
sorted and deduplicated. Unknown and allowed libraries or symbols are ignored.
The function consumes `ElfFacts`; it does not parse an artifact itself.

## Native filesystem and ELF collection

`collect_native_scope` is the binary's explicit collector. The scope must be
an absolute, real directory. It is canonicalized first. Recursive traversal
is name-sorted, rejects every symlink, skips `.git` and `target` directories,
and collects only these file forms:

Its output is `NativeCollection { sources: Vec<SourceUnit>, elf_facts:
Vec<ElfFacts> }`. It contains collected facts only, with no policy
classifications or report status; `main` places both vectors into `AuditInput`.

| File form | `SourceKind` |
| --- | --- |
| `.rs` | `Rust` |
| `.zig` | `Zig` |
| `.c`, `.h` | `C` |
| `.cc`, `.cpp`, `.cxx`, `.hh`, `.hpp`, `.hxx` | `Cpp` |
| `.ll` | `LlvmIr` |
| `.toml`, `.json`, `.yaml`, `.yml`, `.cmake`, `.mk`, `.ninja`, `.bazel`, `.bzl` | `BuildMetadata` |
| `Cargo.lock`, `Makefile`, `CMakeLists.txt`, `BUILD`, `BUILD.bazel`, `WORKSPACE`, `WORKSPACE.bazel` | `BuildMetadata` |

Unsupported files are ignored. At most 100,000 source files are collected,
and each collected text file must be no larger than 16 MiB and valid UTF-8.
Collected source paths are relative to the canonical scope and slash
normalized. The limits are per text file and supported-file count; there is no
aggregate scope-byte limit, ELF-count limit, or explicit recursion-depth limit.
The resulting source list is deterministic for the filesystem state observed
by the traversal.

ELF paths are separate, explicit arguments. Each must be an absolute real
regular file, must canonicalize under the canonical scope, and must be unique
after canonicalization. This allows a caller to audit an artifact in
`target` even though `target` is excluded from source traversal. ELF files are
bounded to 1 GiB. `read_elf_facts` uses `goblin::elf::Elf::parse`, collects and
sorts unique `DT_NEEDED` names, and collects unique nonempty undefined names
from both dynamic and static symbol tables (`SHN_UNDEF`).
The parser does not impose an ELF machine, ABI, executable-bit, or operating
system check beyond Goblin accepting the file as ELF.

`read_elf_facts` itself still requires an absolute path, but its returned
display path is the normalized path supplied by the caller. It uses ordinary
metadata and therefore can follow a final symlink to a regular ELF, unlike the
scoped collector's explicit symlink rejection. The collector instead returns a
path relative to the scope. Both paths are evidence labels, not authority for
filesystem access after collection.

## Errors and failure boundaries

`AuditError` is the only library error type. All variants fail the gate:

| Variant | Boundary |
| --- | --- |
| `Configuration(String)` | Missing/relative paths, invalid mode coupling, empty facts, duplicate display paths, grant syntax, limits, or CLI configuration. |
| `InvalidDependencyGraph(String)` | Directly constructed graph has missing roots or endpoints, duplicate package IDs, or duplicate edges. |
| `InvalidCargoMetadata(String)` | JSON parse/shape/version/field errors, duplicate resolve nodes, absent resolve closure, or no roots. |
| `Lexical { path, line, reason }` | Unterminated comments or literals in a source unit; `line` is the opening line of the construct. |
| `InvalidElf { path, reason }` | Non-regular, oversized, or malformed ELF input. |
| `Io { path, source }` | Filesystem metadata, read, canonicalization, directory traversal, or UTF-8 read failures represented by an underlying `std::io::Error`. |

`Display` prefixes errors with the boundary (`invalid audit configuration`,
`invalid dependency graph`, `invalid Cargo metadata`, lexical location, or
invalid ELF path). Only `Io` exposes an underlying error through
`std::error::Error::source`; policy findings are report data, not errors.

Failure precedence follows the call graph. The CLI parses first, then collects
the native scope and explicit ELF paths, loads optional metadata, resolves the
mode-dependent grant file, assembles linker facts, and finally calls the
library. Inside `audit`, aggregate input validation precedes source lexing,
dependency-graph validation, linker scanning, ELF scanning, sorting, and grant
application. The first error at that boundary aborts the call; earlier local
findings are never returned as a partial report.

## Determinism and security invariants

- Paths are slash-normalized for display, and injected source and ELF paths
  must be unique and already normalized.
- Directory entries, dependency adjacency, dependency closure findings, ELF
  libraries, and ELF symbols use ordered collections or explicit sorting.
- Findings are sorted and deduplicated before grants are applied. Grant keys
  are exact `(category, path, line, symbol)` tuples.
- The collector requires absolute scope and artifact paths, rejects symlinks,
  enforces canonical scope containment, and applies 16 MiB, 1 GiB, 100,000
  source-file, and 64 MiB JSON limits.
- No classifier is a raw substring allowlist. Symbol families require an
  explicit prefix or identifier boundary, library families require a
  recognized named stem, and dependencies require an explicit package-family
  suffix.
- Missing optional facts remain unaudited facts. They are not replaced with a
  guessed package graph, current directory, linker invocation, or host scan.

The collectors are batch-oriented: accepted source text and ELF bytes are
materialized before policy evaluation, with no aggregate source-byte cap beyond
the per-file and count limits above. Ordinary lexing is linear in source bytes,
but LLVM declaration/call detection rescans same-line token prefixes for each
`@` and line-context checks rescan `contents.lines()` for each string. The
dependency walk visits each reachable package and edge once, while each local
finding vector is sorted before deduplication. These are current implementation
properties, not hidden background workers or asynchronous collection.

The current crate has no checked-in unit, integration, or doc tests: `cargo
test -p recipe-audit` builds the library and binary test targets and reports
zero tests. Structural checks therefore establish compilation only; behavioral
evidence for this crate comes from invoking the public CLI path or from a
caller exercising the public `recipe_audit::audit` boundary with real facts.

The private helper topology is also intentionally narrow: `lib.rs` has only
`apply_legacy_grants`; `artifact.rs` has `linker_candidates`;
`dependency.rs` has `required_string` plus graph validation/audit methods;
`native.rs` has `collect_paths`, `source_kind`, `relative_display`,
`require_absolute`, and `read_elf_facts_with_display`; `source.rs` has the LLVM,
string, context, category, and finding helpers; `lexer.rs` has the comment,
literal, raw-string, lifetime, and identifier helpers; `policy.rs` contains
the decoration, family, library-stem, and package-family matchers; and
`model.rs` contains display-path and aggregate-validation helpers. `main.rs`
contains the one-shot `run`, bounded-text reader, `Cli::parse`, UTF-8 option,
duplicate-option, and usage helpers. None is an alternate public execution
path.

## Source map

| Module | Responsibility and primary boundary |
| --- | --- |
| [`lib.rs`](../src/lib.rs) | Public re-exports, orchestration, global ordering/deduplication, and legacy-grant application. |
| [`main.rs`](../src/main.rs) | Strict argument parser, bounded JSON loading, native collection, report serialization, and exit codes. |
| [`model.rs`](../src/model.rs) | Modes, finding/report records, injected fact types, path normalization, and input validation. |
| [`policy.rs`](../src/policy.rs) | Exact native-interface, library, dependency, and CUDA Driver allowlist policy. |
| [`lexer.rs`](../src/lexer.rs) | Language-sensitive tokenization and lexical failure detection. |
| [`source.rs`](../src/source.rs) | Source, build metadata, string-context, and LLVM declaration/call auditing. |
| [`dependency.rs`](../src/dependency.rs) | Cargo metadata parsing, graph validation, reachable closure, and package findings. |
| [`artifact.rs`](../src/artifact.rs) | Linker candidate extraction and `ElfFacts` policy evaluation. |
| [`native.rs`](../src/native.rs) | Bounded filesystem collection and `goblin` ELF fact extraction. |
| [`error.rs`](../src/error.rs) | Fatal input and collection error taxonomy and formatting. |

The per-module files under `audit/.docs/src/` provide focused module traces;
this README is the crate-level contract and cross-module data-flow summary.

Focused traces are available for [`lib`](src/lib.md), [`main`](src/main.md),
[`model`](src/model.md), [`policy`](src/policy.md), [`lexer`](src/lexer.md),
[`source`](src/source.md), [`dependency`](src/dependency.md),
[`artifact`](src/artifact.md), [`native`](src/native.md), and
[`error`](src/error.md).
