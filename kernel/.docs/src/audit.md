# kernel/src/audit.rs

kernel/src/audit.rs is the narrow source-level interface gate for generated
LLVM modules. It receives the complete LLVM IR string assembled by a kernel
lowerer and reports declarations whose symbol is not an LLVM intrinsic. The
purpose is to keep a Recipe kernel a closed module: lowering may use target
intrinsics that LLVM understands, but it may not introduce an unresolved host
runtime, operation-library, or other external call.

This module is deliberately not the repository's complete native-boundary
policy. The recipe-audit crate owns source, dependency, linker, dynamic-load,
and final-ELF policy. The kernel artifact inspector has a separate final-ELF
check for unresolved global symbols. Keeping those checks separate prevents
the compiler from carrying a second, divergent list of prohibited native
interface names.

## Ownership and public surface

kernel/src/lib.rs declares the module privately and re-exports its three
public items:

~~~text
mod audit;
pub use audit::{AuditFinding, AuditKind, audit_llvm_ir};
~~~

The crate has #![forbid(unsafe_code)] and denies missing Debug
implementations. The audit module has no filesystem access, subprocesses,
configuration loading, mutable global state, target probing, or artifact
parsing. It only scans the borrowed &str supplied by its caller.

| Item | Shape | Meaning |
| --- | --- | --- |
| AuditKind | #[non_exhaustive] enum | Currently one reason, ExternalIrDeclaration. |
| AuditFinding | Clone + Debug + PartialEq + Eq struct | One declaration that violates the closed-module rule. |
| audit_llvm_ir | #[must_use] fn(&str) -> Vec<AuditFinding> | Deterministically scans one complete LLVM IR string. |

AuditKind is non-exhaustive so consumers must not assume that external
declarations are the only future audit reason. It currently has no ordering or
serialization implementation. AuditFinding contains:

~~~text
AuditFinding {
    kind: AuditKind,
    line: usize,
    token: String,
}
~~~

line is one-based. token is the ASCII identifier-like text extracted after
the first @ on the declaration line. The token is owned, so the returned
vector remains valid after the input string is dropped. Findings are emitted in
source order and are not sorted or deduplicated by this module.

## Exact scanning algorithm

The implementation is intentionally lexical rather than an LLVM parser. Its
behavior is equivalent to this pseudocode:

~~~text
findings = []
for (line_index, line) in ir.lines().enumerate():
    trimmed = line.trim_start()
    if not trimmed.starts_with("declare "):
        continue

    at = trimmed.find('@')
    if no at:
        continue

    candidate = trimmed[at + 1 ..]
    token = candidate
        .split(characters_not_in_ascii_alphanumeric_dot_underscore)
        .next()
        .unwrap_or_default()

    if not token.starts_with("llvm."):
        findings.push(AuditFinding {
            kind: ExternalIrDeclaration,
            line: line_index + 1,
            token: token.to_owned(),
        })

return findings
~~~

The exact consequences are part of the contract:

* Leading whitespace is ignored. The declaration marker must be the exact
  lowercase text declare followed by an ASCII space. Additional spaces after
  that prefix are accepted. A tab, a different case, a comment prefix, or
  declare without that space is not recognized.
* Only the first @ is considered. Text before it is not parsed and text
  after the extracted token is irrelevant.
* The token consists only of ASCII letters, ASCII digits, ., and _. Punctuation,
  $, -, quotes, Unicode characters, and whitespace terminate the token. A
  delimiter immediately after @ therefore produces an empty token, which is
  still a finding because the empty string does not start with llvm..
* The allow rule is a case-sensitive lexical prefix. Every token beginning
  with llvm. is accepted, including intrinsic spellings the current emitter
  does not yet know about. llvmfoo, LLVM.foo, and every non-llvm. token are
  rejected.
* Only declaration lines are examined. Calls, invokes, callbr instructions,
  definitions, globals, comments, and metadata are not independently parsed.
  A non-declaration line containing @host_call produces no finding here.
* The function scans all lines and returns all findings. It does not stop after
  the first violation, return a Result, or attempt recovery, normalization,
  symbol version resolution, or duplicate suppression.

Examples:

| LLVM text | Result |
| --- | --- |
| declare i32 @llvm.sqrt.f32(float) | No finding. |
| leading-space declaration of void @external() | ExternalIrDeclaration, line 1, token external. |
| declare i32 @foo.bar(i32) | Token foo.bar, one finding. |
| declare i32 @foo@VERSION(i32) | Token foo, one finding. |
| declare i32 @ | Empty token, one finding. |
| declare i32 @"external"(i32) | Empty token, one finding. |
| call void @external() | No finding from this helper. |
| declare<TAB>i32 @external() | No finding because the marker is not declare followed by an ASCII space. |
| declare i32 @LLVM.sqrt.f32(float) | Finding because the prefix is case-sensitive. |

Malformed LLVM syntax is therefore not necessarily rejected by this helper.
The compiler and artifact toolchain remain responsible for parsing and lowering
the module; this check only enforces the declaration boundary that it owns.

## Declarations emitted by the kernel lowerers

The expected declarations are all target intrinsics and therefore satisfy the
llvm. prefix rule.

### Elementwise lowering

kernel/src/llvm.rs::lower_elementwise assembles a complete module and audits it
before returning LoweredKernel:

~~~text
validate template, target, and options
    -> emit elementwise body
    -> assemble_module
    -> audit_llvm_ir
    -> build KernelAbi and LoweredKernel only when the audit is empty
~~~

assemble_module emits one of these target headers and intrinsic families:

| Target or operation | Declarations |
| --- | --- |
| AMD global indexing | llvm.amdgcn.workitem.id.x, llvm.amdgcn.workgroup.id.x |
| NVIDIA global indexing | llvm.nvvm.read.ptx.sreg.tid.x, llvm.nvvm.read.ptx.sreg.ctaid.x |
| Constrained f32 arithmetic | llvm.experimental.constrained.fadd.f32, fsub.f32, fmul.f32, frem.f32 |
| Constrained f32 FMA | llvm.experimental.constrained.fma.f32 |
| f32 min and max | llvm.minimum.f32, llvm.maximum.f32 |
| Checked f32 to i32 conversion | llvm.fptosi.sat.i32.f32 |
| f32 elementary operations | llvm.sqrt.f32, llvm.floor.f32, llvm.ceil.f32, llvm.roundeven.f32 |

The module uses amdgpu_kernel or ptx_kernel entry conventions and emits
Recipe-owned definitions for the kernel body. Definitions and ordinary LLVM
instructions do not become AuditFinding values. Only a declaration whose
extracted token does not start with llvm. can fail this gate.

### Owned stage lowering

kernel/src/stage.rs::lower_stage validates the complete immutable stage
contract first. It then has two paths:

~~~text
ScalarMap stage
    -> crate::lower_elementwise (audit in llvm.rs)
    -> rewrite_scalar_fault
    -> validate_realized_kernel

Owned stage
    -> emit stage body and declarations
    -> assemble_stage_module
    -> audit_llvm_ir
    -> validate_realized_kernel
~~~

The owned-stage emitter registers declarations in a BTreeMap and emits them
once in assemble_stage_module. The intrinsic names currently used by that path
include:

* AMD indexing and barrier intrinsics:
  llvm.amdgcn.workitem.id.x, llvm.amdgcn.workgroup.id.x, and
  llvm.amdgcn.s.barrier.
* NVIDIA indexing and barrier intrinsics:
  llvm.nvvm.read.ptx.sreg.tid.x, llvm.nvvm.read.ptx.sreg.ctaid.x, and
  llvm.nvvm.barrier0.
* Checked index arithmetic:
  llvm.smul.with.overflow.i64 and llvm.sadd.with.overflow.i64.
* Floating-point and conversion intrinsics:
  llvm.experimental.constrained.fadd.f32,
  llvm.experimental.constrained.fmul.f32,
  llvm.experimental.constrained.fsub.f32,
  llvm.experimental.constrained.fma.f32, llvm.sqrt.f32, and
  llvm.fptosi.sat.i32.f32.

Stage helpers such as Philox and sort routines are emitted as definitions with
Recipe-generated names. They are not external declarations and are not an
exception to the closed-module rule.

For a scalar-map stage, the audit runs before rewrite_scalar_fault. That rewrite
changes an already-emitted atomicrmw instruction from the generic scalar fault
publication to the stage's checked fault operation. It does not add a
declaration, so the earlier declaration audit remains the relevant gate.
Owned stages are audited after their complete module text is assembled and
before ABI and work validation completes.

## Callers and failure propagation

The helper has only two in-repository call sites:

| Caller | Audit point | Failure conversion |
| --- | --- | --- |
| kernel/src/llvm.rs::lower_elementwise | After assemble_module, before LoweredKernel is returned | First finding becomes LoweringErrorKind::ProhibitedInterface. |
| kernel/src/stage.rs::lower_owned_stage | After assemble_stage_module, before validate_realized_kernel | First finding becomes LoweringErrorKind::ProhibitedInterface. |

Both callers format the same evidence into the error message, with a path- or
stage-specific prefix:

~~~text
generated LLVM IR failed audit at line {line}: {kind:?} token {token}
generated stage LLVM IR failed audit at line {line}: {kind:?} token {token}
~~~

The callers inspect findings.first(). Later findings remain available only to a
direct caller of audit_llvm_ir; they are not included in the LoweringError
message. No artifact compiler, linker, runtime loader, or alternate lowering
path is invoked after this error.

The direct helper itself cannot fail with an error: an empty vector means that
the supplied text contains no matching non-LLVM declaration, and a nonempty
vector is the complete lexical evidence. Errors arise only when a lowering
caller turns the first finding into its typed failure. Other lowerer failures
remain owned by their existing checks, including InvalidKernel,
InvalidStageContract, InvalidTarget, InvalidEntrySymbol, InvalidWorkgroupSize,
UnknownScalarValue, UnsupportedOperation, ArithmeticOverflow, and toolchain
or I/O errors; this module does not reinterpret them.

The public lowering paths that carry this error are:

* recipe_kernel::lower_elementwise, used by the kernel examples and the CUDA
  and HSA native-probe benchmark paths;
* recipe_kernel::lower_stage, used by deferred native preparation in
  prepare/src/production.rs;
* callers of those functions, which map LoweringError into their own typed
  preparation or benchmark error without replacing the failed lowering.

The helper itself is public through the crate facade, so a library caller may
audit arbitrary IR directly. A direct call receives the raw finding vector and
does not automatically construct LoweringError or run the broad recipe-audit
gate.

## Final artifact boundary

The IR declaration check is necessary but not sufficient proof that a native
artifact is closed. After the pinned assembler/linker produces an HSACO or
cubin, kernel/src/artifact.rs parses the ELF and calls its private
audit_symbols helper from inspect_hsaco_bundle and inspect_cubin.

audit_symbols collects every symbol with SHN_UNDEF and global or weak binding
into a sorted BTreeSet. Any nonempty set returns
LoweringErrorKind::ArtifactMismatch with the message:

~~~text
artifact has unresolved global symbols: {comma-separated sorted names}
~~~

This downstream check sees the final binary's symbol tables, not the original
LLVM text. It rejects unresolved global and weak symbols, but it does not
classify library families or inspect source spelling. It runs before HSACO ABI
metadata matching and before cubin entry-section checks. ArtifactBuilder
invokes the inspectors for newly built artifacts, while native preparation and
the CUDA/HSA executors invoke them again for prebuilt or loaded images.

The two kernel checks therefore cover different representations:

~~~text
generated LLVM text
    -> audit_llvm_ir: no non-LLVM external declaration
compiled ELF image
    -> audit_symbols: no unresolved global/weak symbol
~~~

Neither check replaces the recipe-audit crate's native-interface policy. An
artifact can be symbolically closed and still be rejected by the broader gate
if its source, dependency graph, linker inputs, dynamic DT_NEEDED names, or
undefined symbols expose a prohibited interface family.

## Broader native-interface audit boundary

The separate audit workspace crate is the policy and collection boundary for
native interfaces. It is not called by recipe-kernel and does not call
audit_llvm_ir. Its central operation is:

~~~text
recipe_audit::audit(AuditInput) -> Result<AuditReport, AuditError>
~~~

### Inputs and collection

AuditInput contains exactly these facts:

| Field | Evidence |
| --- | --- |
| mode | AuditMode::Next (no exceptions) or AuditMode::Legacy (exact grants). |
| sources | SourceUnit { path, kind, contents } for Rust, Zig, C, C++, LLVM IR, or build metadata. |
| dependencies | Optional complete DependencyGraph with packages, edges, and exact root IDs. |
| linker_inputs | LinkerInput { path, line, argument } values from a build or command line. |
| elf_facts | ElfFacts { path, needed, undefined_symbols } from a native artifact. |
| legacy_grants | Exact (category, path, line, symbol) records used only in legacy mode. |

The CLI recipe-audit constructs those facts only after explicit input is
provided. collect_native_scope requires an absolute real directory, rejects
symlinks, skips .git and target during recursive source collection, admits
only supported source/build extensions, reads UTF-8 files, and reports paths
relative to the canonical scope. Explicit ELF paths must be absolute real
regular files, canonicalize inside that scope, and be unique. The collector
enforces a 16 MiB per text file bound, a 1 GiB per ELF bound, and a 100,000
supported-source-file bound. read_elf_facts is the one-file public reader; it
requires an absolute path but has no scope-containment guarantee.

ELF collection copies and sorts/deduplicates DT_NEEDED names and combines
undefined names from dynamic and regular symbol tables. It does not classify
them. Source collection only assigns SourceKind; lexical policy runs later.
The collector returns an error instead of partial evidence when a required
filesystem read, UTF-8 conversion, canonicalization, or ELF parse fails.

AuditInput::validate then rejects duplicate source or ELF paths, empty or
backslash-unnormalized display paths, empty linker arguments, and empty ELF
library or symbol names. It does not manufacture omitted evidence.

The broad collector and evaluator preserve failure provenance through these
AuditError variants:

| Variant | Owning boundary |
| --- | --- |
| Configuration | Absolute-path, scope, size, duplicate, mode, grant, or input-shape violation. |
| InvalidDependencyGraph | Missing, duplicate, or inconsistent injected graph records. |
| InvalidCargoMetadata | Malformed Cargo format-version 1 JSON or incomplete resolve data. |
| Lexical | Unterminated source comment or literal, with path and one-based line. |
| InvalidElf | Non-ELF, oversized, or malformed artifact fact input. |
| Io | Filesystem metadata, directory, text, JSON, or ELF read failure. |

An error aborts the current aggregate operation; it is never converted into a
clean report or a partial finding vector.

### Policy classification

The policy in audit/src/policy.rs classifies complete symbols and normalized
library basenames. It returns Allowed(NativeInterface),
Prohibited(NativeInterface), or Unknown.

Symbol classification strips one leading U+0001 decoration and everything after
the first @ version suffix, then applies this ordered policy:

1. hsaKmt... and kfd_... are prohibited direct-KFD symbols.
2. hsa_... is allowed ROCr/HSA.
3. A symbol in the exact reviewed CUDA_DRIVER_API_ALLOWLIST is allowed.
   Versioned spellings such as _v2 are separate entries, not a prefix rule.
4. A Driver-shaped cuXxx symbol outside that exact list is prohibited.
5. HIP families and CUDA Runtime symbols are prohibited.
6. Operation-library families are prohibited; all other values are unknown.

The current 89-entry allowlist is deliberately explicit:

~~~text
cuCtxCreate, cuCtxCreate_v2, cuCtxDestroy, cuCtxDestroy_v2,
cuCtxGetCurrent, cuCtxGetDevice, cuCtxPopCurrent, cuCtxPopCurrent_v2,
cuCtxPushCurrent, cuCtxPushCurrent_v2, cuCtxSetCurrent,
cuDeviceCanAccessPeer, cuDeviceComputeCapability, cuDeviceGet,
cuDeviceGetAttribute, cuDeviceGetCount, cuDeviceGetName, cuDeviceGetPCIBusId,
cuDeviceGetUuid, cuDeviceGetUuid_v2, cuDevicePrimaryCtxRelease,
cuDevicePrimaryCtxRelease_v2, cuDevicePrimaryCtxRetain, cuDeviceTotalMem,
cuDeviceTotalMem_v2, cuDriverGetVersion,
cuEventCreate, cuEventDestroy, cuEventDestroy_v2, cuEventElapsedTime,
cuEventQuery, cuEventRecord, cuEventSynchronize,
cuFuncGetAttribute, cuFuncSetAttribute, cuGetErrorName, cuGetErrorString,
cuGraphAddDependencies, cuGraphAddEmptyNode, cuGraphAddEventRecordNode,
cuGraphAddEventWaitNode, cuGraphAddKernelNode, cuGraphAddMemcpyNode,
cuGraphDestroy, cuGraphExecDestroy, cuGraphInstantiate,
cuGraphInstantiate_v2, cuGraphLaunch, cuInit, cuLaunchKernel,
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
~~~

The prohibited HIP families are hip, hipblas, hipcub, hipfft, hiprand, hiprtc,
hipsolver, and hipsparse. CUDA Runtime forms include cudart, cuda_runtime,
cuda_runtime_api, and cuda followed by an uppercase letter. Operation families
include rocblaslt, rocblas, rocsolver, rocfft, miopen, rccl, cublas, cusolver,
cufft, cudnn, and nccl.

Library classification strips whitespace and surrounding quoting/brackets,
keeps the basename, removes -l or /DEFAULTLIB:, removes a leading lib,
lowercases, stops at .so, .dylib, .dll, .a, or .lib, and trims -static.
cuda and nvcuda library stems plus hsa-runtime64 and hsa-runtime are allowed.
hsakmt, kfd, HIP families, amdhip64, CUDA Runtime families, and the operation
families are prohibited. Family suffixes are explicit (_static, -static, lt,
lt_static, or coded numeric forms), not arbitrary substring matches. Cargo
dependency classification uses its own lowercase, underscore-to-hyphen
normalization and package suffix rules.

### Evidence evaluation and outputs

After validation, recipe_audit::audit evaluates facts in this order:

~~~text
audit_source for each SourceUnit
    -> DependencyGraph::audit when present
    -> audit_linker_inputs
    -> audit_elf_facts for each ElfFacts
    -> global sort and deduplication
    -> mode-specific legacy handling
    -> AuditReport
~~~

Source findings cover source APIs, build-link inputs, runtime loads, and
same-line LLVM declarations or calls. Linker findings split whole arguments
and punctuation-delimited candidates. ELF findings classify DT_NEEDED as
dynamic-needed and undefined symbols as undefined-symbol; binary facts use line
0. Dependency findings cover only the root-reachable graph closure and also use
line 0.

Every finding starts as blocking. In next mode any supplied legacy grant is a
configuration error and any finding makes passed false. In legacy mode every
grant must be an exact non-wildcard match, duplicate grants are rejected,
matching findings become grandfathered, and unused or stale grants are an
error. AuditReport::passed is true only when every finding is grandfathered,
which includes the empty-finding case.

The CLI serializes a successful report as pretty JSON with mode, passed, and
findings fields. Exit status 0 means no blocking findings, 1 means a valid
report still has blocking findings, and 2 means a parse, configuration,
collection, lexical, metadata, graph, grant, ELF, or serialization error. The
library returns typed errors and never prints. These outputs are distinct from
the LoweringError returned by the kernel IR gate.

## Invariants and non-goals

The combined boundary has these invariants:

* Generated Recipe LLVM may declare only LLVM-prefixed intrinsics. A host
  runtime or operation-library declaration is a lowering failure.
* A successful lowering does not imply a successful artifact. Final HSACO or
  cubin inspection must also find no unresolved global or weak symbols and must
  match the requested target and ABI.
* The kernel checks do not own the list of ROCr, CUDA Driver, HIP, CUDA Runtime,
  KFD, or vendor operation-library names. That list is centralized in
  audit/src/policy.rs and consumed by recipe-audit evidence adapters.
* Missing evidence is not treated as proof. The broad gate audits only facts
  supplied through its explicit input and reports collection or validation
  failures instead of silently substituting another source.
* Audit errors are visible and terminal at their owning boundary. There is no
  retry, alternate parser, fallback lowering, grant wildcard, or automatic
  policy exception.
* All successful scans are deterministic for the supplied bytes and facts.
  audit_llvm_ir preserves declaration order; broad audit producers and the
  aggregate report sort and deduplicate their finding records.

The narrow helper intentionally does not parse LLVM grammar, inspect call
instructions, resolve aliases or symbol versions, inspect comments, classify
native library names, prove final binary closure, or invoke recipe-audit.
Those are separate responsibilities with separate evidence and failure types.
