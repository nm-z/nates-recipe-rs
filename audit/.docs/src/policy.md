# Native interface policy

`audit/src/policy.rs` is the single classification dictionary for the
`recipe-audit` native-interface gate. It does not collect facts, parse source,
walk a dependency graph, inspect an ELF, or decide whether a report passes.
Those operations belong to the consumers described below. This module answers
one question for one complete symbol, library name, or package name: is it an
explicitly allowed native interface, an explicitly prohibited interface, or
not a name known to this policy?

The policy is deliberately lexical. Symbol rules operate on complete lexical
tokens, and library rules operate on a normalized basename. No rule searches an
arbitrary substring. The source implementation is the authority for the
spelling and boundary details in this document.

## Authority and configuration boundary

All policy data is compiled into `policy.rs`:

- `CUDA_DRIVER_API_ALLOWLIST` is a public `const` slice containing the exact
  reviewed CUDA Driver host-call spellings.
- `OPERATION_FAMILIES` is a private `const` slice mapping prohibited operation
  family roots used by symbol, library, and dependency matching to
  `NativeInterface` variants.
- `HIP_INTERFACE_FAMILIES` is a private `const` slice containing prohibited HIP
  interface families.
- The matching helpers in this file define the accepted decoration, case, and
  suffix boundaries.

There is no TOML, JSON, environment, command-line, filesystem, or hardware
loading in this module. There is no `Default` policy and no mutable policy
state. The only inputs are strings supplied by callers. A policy change is a
source change, followed by a rebuild of the auditor. In particular, adding a
CUDA Driver call requires adding its exact spelling to the allowlist; a broad
`cu` prefix is never an implicit default.

The public library facade re-exports `NativeInterface`,
`InterfaceClassification`, `CUDA_DRIVER_API_ALLOWLIST`,
`classify_interface_symbol`, and `classify_library`. `classify_dependency` is
crate-private because Cargo package names are an internal input to the
dependency and build-metadata auditors. No other workspace crate currently
calls these exports.

`Unknown` is a real result, not an error. `InterfaceClassification::is_prohibited`
returns true only for `Prohibited(_)`; both `Allowed(_)` and `Unknown` return
false. Consumers therefore report only names recognized as prohibited. The
policy has no catch-all rejection for an unrecognized name.

## Classification data model

`NativeInterface` identifies the family that caused a classification:

| Variant | Meaning |
| --- | --- |
| `RocrHsa` | ROCr/HSA, the allowed AMD native boundary. |
| `CudaDriver` | CUDA Driver API, allowed only for the reviewed symbols or library/dependency names. |
| `Hip` | HIP and its listed interface families, prohibited. |
| `CudaRuntime` | CUDA Runtime API, prohibited. |
| `DirectKfd` | Direct KFD ownership or access, prohibited. |
| `RocBlas` | `rocblaslt` or `rocblas`, prohibited. |
| `RocSolver` | `rocsolver`, prohibited. |
| `RocFft` | `rocfft`, prohibited. |
| `MiOpen` | `miopen`, prohibited. |
| `Rccl` | `rccl`, prohibited. |
| `CuBlas` | `cublas`, prohibited. |
| `CuSolver` | `cusolver`, prohibited. |
| `CuFft` | `cufft`, prohibited. |
| `CuDnn` | `cudnn`, prohibited. |
| `Nccl` | `nccl`, prohibited. |
| `CudaDriverOutsideAllowlist` | A Driver-shaped `cuXxx` symbol that is not in the exact reviewed list. It is prohibited even though it resembles the allowed Driver API. |

`InterfaceClassification` is one of:

- `Allowed(interface)`, which a consumer must permit;
- `Prohibited(interface)`, which a consumer must turn into a finding when the
  relevant evidence path audits it; or
- `Unknown`, for a name outside the dictionaries.

`NativeInterface` derives ordering and equality traits for deterministic
comparison. `InterfaceClassification` derives equality traits, and the
classification itself is `Copy` with no allocation.

## Allowed names

### Symbols

The symbol classifier allows every symbol beginning with the case-sensitive
prefix `hsa_`. It allows no other HSA spelling through this rule. Direct-KFD
prefixes are checked first, so `hsaKmt...` remains prohibited and is not
mistaken for ROCr/HSA.

CUDA Driver symbols are allowed only when the complete decorated-and-stripped
symbol is one of the 89 entries in `CUDA_DRIVER_API_ALLOWLIST`. The list is
case-sensitive and keeps ABI versions as separate entries. The current entries
are grouped here by operation for readability; the source slice remains the
authoritative spelling:

| Group | Exact entries |
| --- | --- |
| Context | `cuCtxCreate`, `cuCtxCreate_v2`, `cuCtxDestroy`, `cuCtxDestroy_v2`, `cuCtxGetCurrent`, `cuCtxGetDevice`, `cuCtxPopCurrent`, `cuCtxPopCurrent_v2`, `cuCtxPushCurrent`, `cuCtxPushCurrent_v2`, `cuCtxSetCurrent` |
| Device and driver | `cuDeviceCanAccessPeer`, `cuDeviceComputeCapability`, `cuDeviceGet`, `cuDeviceGetAttribute`, `cuDeviceGetCount`, `cuDeviceGetName`, `cuDeviceGetPCIBusId`, `cuDeviceGetUuid`, `cuDeviceGetUuid_v2`, `cuDevicePrimaryCtxRelease`, `cuDevicePrimaryCtxRelease_v2`, `cuDevicePrimaryCtxRetain`, `cuDeviceTotalMem`, `cuDeviceTotalMem_v2`, `cuDriverGetVersion` |
| Events | `cuEventCreate`, `cuEventDestroy`, `cuEventDestroy_v2`, `cuEventElapsedTime`, `cuEventQuery`, `cuEventRecord`, `cuEventSynchronize` |
| Functions and errors | `cuFuncGetAttribute`, `cuFuncSetAttribute`, `cuGetErrorName`, `cuGetErrorString` |
| Graphs | `cuGraphAddDependencies`, `cuGraphAddEmptyNode`, `cuGraphAddEventRecordNode`, `cuGraphAddEventWaitNode`, `cuGraphAddKernelNode`, `cuGraphAddMemcpyNode`, `cuGraphDestroy`, `cuGraphExecDestroy`, `cuGraphInstantiate`, `cuGraphInstantiate_v2`, `cuGraphLaunch` |
| Initialization and launch | `cuInit`, `cuLaunchKernel` |
| Memory and copies | `cuMemAlloc`, `cuMemAllocHost`, `cuMemAllocHost_v2`, `cuMemAlloc_v2`, `cuMemFree`, `cuMemFreeHost`, `cuMemFree_v2`, `cuMemGetInfo`, `cuMemGetInfo_v2`, `cuMemHostAlloc`, `cuMemHostGetDevicePointer`, `cuMemHostGetDevicePointer_v2`, `cuMemHostRegister`, `cuMemHostRegister_v2`, `cuMemHostUnregister`, `cuMemcpyDtoDAsync`, `cuMemcpyDtoDAsync_v2`, `cuMemcpyDtoHAsync`, `cuMemcpyDtoHAsync_v2`, `cuMemcpyHtoDAsync`, `cuMemcpyHtoDAsync_v2`, `cuMemcpyPeerAsync` |
| Modules | `cuModuleGetFunction`, `cuModuleGetGlobal`, `cuModuleGetGlobal_v2`, `cuModuleGetLoadingMode`, `cuModuleLoad`, `cuModuleLoadData`, `cuModuleLoadDataEx`, `cuModuleUnload` |
| Pointers and streams | `cuPointerGetAttribute`, `cuStreamCreate`, `cuStreamCreateWithPriority`, `cuStreamDestroy`, `cuStreamDestroy_v2`, `cuStreamGetPriority`, `cuStreamQuery`, `cuStreamSynchronize`, `cuStreamWaitEvent` |

The exact list includes `cuModuleGetLoadingMode`; capability gating elsewhere
may select a smaller runtime subset, but this policy does not infer capability
or add a second default list.

### Libraries

After library-name normalization, the following complete stems are allowed:

- `cuda` and `nvcuda`, classified as `CudaDriver`;
- `hsa-runtime64` and `hsa-runtime`, classified as `RocrHsa`.

### Dependencies

Dependency names are allowed only for these exact normalized package names:

- `cuda-driver` and `cuda-driver-sys`, classified as `CudaDriver`;
- `hsa`, `hsa-sys`, `rocr`, and `rocr-sys`, classified as `RocrHsa`.

An allowed dependency name does not make any similarly named library or symbol
allowed. Each evidence kind enters its own classifier.

## Prohibited dictionaries

`OPERATION_FAMILIES` maps these exact family roots to prohibited variants:

| Family roots | Variant |
| --- | --- |
| `rocblaslt`, `rocblas` | `RocBlas` |
| `rocsolver` | `RocSolver` |
| `rocfft` | `RocFft` |
| `miopen` | `MiOpen` |
| `rccl` | `Rccl` |
| `cublas` | `CuBlas` |
| `cusolver` | `CuSolver` |
| `cufft` | `CuFft` |
| `cudnn` | `CuDnn` |
| `nccl` | `Nccl` |

`HIP_INTERFACE_FAMILIES` contains `hip`, `hipblas`, `hipcub`, `hipfft`,
`hiprand`, `hiprtc`, `hipsolver`, and `hipsparse`. `amdhip64` is also
recognized as a prohibited HIP library and symbol spelling, but it is not in
the dependency-family slice.

Direct KFD is recognized separately:

- symbols beginning with `hsaKmt` or `kfd_`;
- library stems exactly named `hsakmt` or `kfd` after normalization; and
- package families `hsakmt` and `kfd` with an accepted package suffix.

The CUDA Runtime classifier recognizes the exact symbols `cudart`,
`cuda_runtime`, and `cuda_runtime_api`, plus `cuda` followed by an ASCII
uppercase character, such as `cudaMalloc`. The corresponding library family is
`cudart`, and the dependency families are `cudart` and `cuda-runtime`.

## Symbol matching

`classify_interface_symbol` applies these checks in order:

1. `strip_symbol_decoration` removes one leading U+0001 object-file marker and
   then keeps only the text before the first `@`. A version suffix therefore
   does not change a classification, for example `cuInit@VERSION` is treated
   as `cuInit`.
2. A `hsaKmt` or `kfd_` prefix returns
   `Prohibited(DirectKfd)`.
3. An `hsa_` prefix returns `Allowed(RocrHsa)`.
4. An exact entry in `CUDA_DRIVER_API_ALLOWLIST` returns
   `Allowed(CudaDriver)`.
5. `is_cuda_driver_shape` marks a name longer than two bytes that starts with
   `cu`, has an ASCII uppercase third byte, and does not start with `cuda`, as
   `Prohibited(CudaDriverOutsideAllowlist)`. This is deliberately checked
   before the other prohibited families. An unlisted `cuFoo` therefore cannot
   pass merely because it looks like a Driver call.
6. `is_hip_symbol` checks the explicit names `hip`, `hipcc`, `hipconfig`,
   `hiprtc`, and `amdhip64`, then removes one leading `__` and applies the
   case-sensitive HIP family boundary described below. A matching name returns
   `Prohibited(Hip)`.
7. `is_cuda_runtime_symbol` removes one leading `__`, checks its three exact
   names, then checks the uppercase `cudaXxx` shape. A match returns
   `Prohibited(CudaRuntime)`.
8. `classify_operation_symbol` checks operation families case-insensitively.
   A matching name returns the mapped prohibited operation variant; otherwise
   the result is `Unknown`.

The family boundary is significant. HIP symbol matching accepts an empty
suffix or a suffix whose first byte is ASCII uppercase or `_`. Operation symbol
matching is case-insensitive for the family prefix and accepts an empty suffix
or a suffix beginning with ASCII uppercase, `_`, or a digit. Thus a family is
not an arbitrary substring: lower-case continuation text without a boundary is
not matched. The operation search is ordered with `rocblaslt` before `rocblas`,
although both map to `RocBlas`.

The checks are case-sensitive except where explicitly stated. Symbols with
unrecognized decoration, lower-case variants, or an unsupported suffix become
`Unknown`, not a policy error.

## Library-name matching

`classify_library` first calls `normalized_library_stem`. Normalization is
deterministic and occurs before any family check:

1. Trim whitespace, then trim any leading or trailing quote, apostrophe,
   parenthesis, or square bracket characters.
2. Keep the basename after the last `/` or `\\`.
3. Remove a leading `-l` or a leading `/DEFAULTLIB:`. These spellings are
   case-sensitive at this stage. Because basename extraction happens first, a
   literal token beginning with `/DEFAULTLIB:` has already lost its leading
   slash and does not match this second alternative when passed directly to
   `classify_library`. The linker-input consumer also splits on `:`, so its
   separate `lib...` candidate still reaches the library classifier.
4. Remove one leading `lib`, also case-sensitive at this stage.
5. Lowercase using ASCII rules.
6. Cut at the earliest occurrence of `.so`, `.dylib`, `.dll`, `.a`, or `.lib`.
   This handles versioned names such as `libcudart.so.12` by retaining the
   stem before `.so`.
7. Remove a trailing `-static`. An empty result is `None`, which produces
   `Unknown`.

The normalized stem is then classified in this order:

1. `cuda` or `nvcuda` is `Allowed(CudaDriver)`.
2. `hsa-runtime64` or `hsa-runtime` is `Allowed(RocrHsa)`.
3. `hsakmt` or `kfd` is `Prohibited(DirectKfd)`.
4. `amdhip64` or a HIP family is `Prohibited(Hip)`.
5. The `cudart` family is `Prohibited(CudaRuntime)`.
6. An operation family is `Prohibited` with its mapped variant.
7. No match is `Unknown`.

The library-family helper accepts only these suffix forms after an exact
family root: no suffix, `_static`, `-static`, `lt`, `lt_static`, `64_` followed
by one or more ASCII digits, or `_` followed only by ASCII digits. This covers
the reviewed static and versioned linker names without accepting arbitrary
prefix extensions. The same boundary is used for HIP, CUDA Runtime, and
operation libraries. The operation-family list is searched in source order.

## Dependency-name matching

`classify_dependency` lowercases the package name and replaces every `_` with
`-`. It does not trim whitespace, parse a Cargo package specification, or
accept arbitrary version suffixes. It first checks the six exact allowed names
listed above. It then applies `has_package_family` to prohibited families.

For package families, the suffix must be empty or exactly one of `-sys`,
`-src`, `-bindings`, `-runtime`, `-runtime-sys`, `-static`, or `-wrapper`.
The helper is used for HIP, `cudart`, `cuda-runtime`, `hsakmt`, `kfd`, and the
operation families. A package such as `hip-1` is therefore `Unknown`, while
`hip-sys` is `Prohibited(Hip)`.

No dependency classification is exported from the crate facade. The graph
auditor uses it for package names, and the source auditor uses it only as a
fallback for unknown identifiers in `BuildMetadata` units.

## Consumer trace

The policy is pure, but its result has different evidence categories depending
on the caller. The production path is:

```text
recipe-audit CLI
  -> collect_native_scope / optional Cargo metadata / explicit linker and ELF facts
  -> recipe_audit::audit(AuditInput)
       -> audit_source -> symbol, library, and (for build metadata) dependency classifiers
       -> DependencyGraph::audit -> dependency classifier
       -> audit_linker_inputs -> library and symbol classifiers
       -> audit_elf_facts -> library and symbol classifiers
       -> sort and deduplicate findings
       -> mode-specific legacy-grant handling
       -> AuditReport::new
```

### Source and build metadata

`audit_source` lexes a `SourceUnit` with the language-aware lexer, so comments
and literals are handled before policy matching. It has one path-exact early
return for `audit/src/policy.rs`, intended as the self-hosting exception for the
policy dictionary itself. The exception is coordinate-sensitive: native
collection reports paths relative to the selected scope. A repository-root
scope reports this file as `audit/src/policy.rs` and reaches the exemption, but
an `audit`-directory scope reports it as `src/policy.rs` and does not. The
latter is an observed current behavior, so the policy source is then audited
like any other source unit. The early return happens before lexing, so an exact
`audit/src/policy.rs` unit also bypasses lexical-error reporting for that file.
Dependency, linker, and ELF facts for the compiled auditor are audited in either
case.

For ordinary Rust, Zig, C, and C++ units, each identifier is passed to
`classify_interface_symbol`. A prohibited result becomes a `SourceApi` finding.
For `BuildMetadata`, the same result is categorized as `BuildLinkInput`, and an
`Unknown` symbol is then passed to `classify_dependency` so package names and
link metadata are covered.

LLVM IR receives one additional pass. An `@` followed on the same line by an
identifier or string is classified as an `LlvmDeclaration` when the preceding
same-line tokens contain `declare`, or as an `LlvmCall` when they contain
`call`, `invoke`, or `callbr`. If both markers occur, `declare` wins because
that category is selected first. The general token loop skips that immediate
`@` target, preventing duplicate findings. A prohibited direct-KFD or
unlisted-Driver-shaped symbol overrides those categories with
`DisallowedNativeInterface`.

String literals are checked as possible libraries or runtime loads. The string
is trimmed and terminal `\00` or `\0` escapes are removed. A build-metadata
string is always link context. Other strings are link context when their line
contains `rustc-link-lib`, `rustc-link-arg`, `#[link`,
`target_link_libraries`, `linkSystemLibrary`, or `-l`. Link context produces a
`BuildLinkInput` finding. Otherwise, an include marker (`#include`, `@import`,
or `@cImport`) produces `SourceApi`; all other prohibited strings produce
`RuntimeLoad`.

The string checker first tries the complete value as a library, then, for build
metadata, as a dependency. It also splits on path, assignment, punctuation,
and ASCII-whitespace separators and checks each component as a symbol, library,
or build dependency. It still reports only a `Prohibited` classification.

### Linker and ELF artifacts

`audit_linker_inputs` splits each explicit linker argument into candidates,
including the complete trimmed argument, whitespace and punctuation-separated
components, and quote-trimmed components. A candidate prohibited as either a
library or a symbol becomes a `BuildLinkInput` finding at the supplied path and
line. Candidates are sorted and deduplicated before returning.

`audit_elf_facts` applies `classify_library` only to `DT_NEEDED` names, creating
`DynamicNeeded` findings for prohibited libraries. It applies
`classify_interface_symbol` to every undefined dynamic or static symbol and
creates `UndefinedSymbol` findings for any `Prohibited(_)`, including
`CudaDriverOutsideAllowlist` and `DirectKfd`. Allowed HSA or Driver symbols do
not produce findings. Binary evidence uses line zero.

### Cargo dependency closure

`DependencyGraph::audit` validates package IDs, names, manifest paths, roots,
and edges before traversing from the exact caller-selected roots. The traversal
is breadth-first over a sorted, deduplicated adjacency map. Only reachable
packages are classified. A prohibited package name creates a `Dependency`
finding with the package manifest path, line zero, and package name. An
unreachable prohibited package is not evidence for the selected roots.

`from_cargo_metadata_json` requires Cargo metadata format version 1, a complete
`packages` array, a `resolve.nodes` array, exact nonempty root IDs, and a
resolve graph. Missing or partial metadata is an error, not an empty graph.

### Report assembly and CLI

`recipe_audit::audit` validates the complete `AuditInput` before calling any
consumer. It combines all findings, sorts them using the derived ordering, and
deduplicates exact equal findings. In `next` mode, any legacy grant is a
configuration error. In `legacy` mode, every grant is validated and indexed by
the exact tuple `(category, normalized path, line, symbol)`. A matching finding
becomes `Grandfathered`; duplicate, wildcard, unused, or stale grants are
configuration errors. No grant can match a family or substring.

`AuditReport::new` sets `passed` only when every finding is `Grandfathered`.
An empty finding list therefore passes. A prohibited classification is blocking
in `next` mode and remains
blocking in `legacy` mode unless one exact grant consumes it. The CLI prints
the JSON report and exits 0 for `passed`, 1 for a report with blocking findings,
and 2 for parsing, collection, validation, lexical, metadata, graph, or other
`AuditError` failures.

The CLI requires an explicit `--mode` and absolute `--scope`; it does not select
the current directory or run Cargo metadata implicitly. Optional dependency
metadata, package roots, linker arguments, ELF paths, and legacy grants are
all explicit inputs. Native collection excludes `.git` and `target` from source
walking, while explicitly named ELF files under the scope, including `target`,
are still inspected.

## Validation and failure boundaries

The policy functions themselves are total over `&str`: empty or malformed
candidate strings normally produce `Unknown` rather than panicking or returning
an `AuditError`. Validation and failure behavior is owned by the surrounding
layers:

- `AuditInput::validate` rejects duplicate source or ELF paths, empty or
  unnormalized display paths, empty linker arguments, and empty ELF library or
  symbol facts.
- `audit_source` returns a lexical error for an unterminated comment or literal.
- `DependencyGraph` rejects missing roots, duplicate IDs or edges, absent edge
  endpoints, and malformed Cargo metadata.
- Native collection rejects relative paths, symlinks, paths escaping the
  explicit scope, oversized or invalid UTF-8 inputs, and malformed ELF files.
- Legacy grant validation rejects empty fields, wildcard syntax, unnormalized
  paths, duplicates, and grants that match no finding.

These failures do not become `Unknown` or a clean report. They abort the audit
with `AuditError`, preserving the distinction between absent evidence and a
known clean result.

## Invariants for policy changes

The current implementation relies on the following invariants:

1. ROCr/HSA and the reviewed CUDA Driver surface are the only allowed native
   interfaces. Allowed libraries and package names are separate explicit
   entries, not inferred from an allowed symbol.
2. HIP, CUDA Runtime, direct KFD, and all listed operation-library families are
   prohibited across every evidence kind that can represent them.
3. A Driver-shaped symbol outside the exact allowlist is prohibited. Extending
   the Driver surface requires an exact, visible allowlist edit.
4. Matching is boundary-aware and deterministic. Prefix-like names with an
   unsupported suffix remain `Unknown`; arbitrary substring matching is not
   permitted.
5. Normalization is limited to the input kind: symbol decoration stripping,
   library basename normalization, or dependency underscore replacement. It
   does not merge the three dictionaries or create cross-kind defaults.
6. `Unknown` never grants an exception. It simply produces no finding in the
   consumers that ask only whether a result is prohibited; malformed evidence
   is rejected before classification by the owning input layer.
7. The self-hosted policy source exemption is path-exact and applies only to
   source scanning. Its path is relative to the caller's `SourceUnit`, so it is
   not scope-independent. It does not exempt the built auditor's dependencies,
   linker inputs, or ELF symbols.

When the policy changes, the affected classifier and every evidence consumer
must retain these ordering, boundary, and category invariants. There are no
runtime defaults or external policy files to update.
