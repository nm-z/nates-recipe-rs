# `audit/src/artifact.rs`

## Intent

`artifact.rs` is the policy-evaluation boundary for native linker evidence and
already-extracted ELF evidence. It converts prohibited library names and native
symbols into deterministic, blocking [`Finding`](../../src/model.rs) values.
The module does not discover files, parse ELF bytes, inspect Cargo metadata, or
decide whether a finding is grandfathered. Those responsibilities belong to
`native.rs`, `dependency.rs`, `model.rs`, and the aggregate `audit` function in
`lib.rs`.

The policy source of truth is `policy.rs`. Both operations treat only
`InterfaceClassification::Prohibited(_)` as a violation. `Allowed(_)` and
`Unknown` are ignored.

## Structure

| Item | Input | Output | Role |
| --- | --- | --- | --- |
| `audit_linker_inputs` (lines 8-26) | `&[LinkerInput]` | `Vec<Finding>` | Splits each linker argument into candidate tokens, classifies each candidate as a library and as a native symbol, and emits `BuildLinkInput` findings for prohibited candidates. |
| `audit_elf_facts` (lines 30-65) | `&ElfFacts` | `Vec<Finding>` | Classifies `DT_NEEDED` library names and undefined ELF symbols, emitting `DynamicNeeded` and `UndefinedSymbol` findings respectively. |
| `linker_candidates` (lines 68-81) | `&str` | sorted, deduplicated `Vec<&str>` | Produces the complete argument and delimiter-separated candidates used by `audit_linker_inputs`. |

The two public functions are re-exported by `audit/src/lib.rs`, so callers may
use them directly. The normal production caller is `audit(input)` in `lib.rs`.

## Model dependencies

- `LinkerInput` supplies `path`, `line`, and the raw `argument`. Its constructor
  normalizes backslashes in `path`, while aggregate `AuditInput::validate`
  additionally requires a nonempty, already-normalized path and argument.
- `ElfFacts` supplies `path`, `needed`, and `undefined_symbols`; each
  `ArtifactSymbol` contributes its `name`. `ElfFacts::new` normalizes the
  display path, but does not itself reject empty or duplicate fact strings.
- `Finding::blocking` supplies the normalized path and `Blocking` disposition.
  The category and line are selected by this module, and the symbol is copied
  from the candidate or fact without policy normalization.
- `Finding`'s derived ordering is what makes the final `sort` and `dedup`
  deterministic. The aggregate audit uses the same key when reconciling
  findings with exact legacy grants.

## `audit_linker_inputs`

For every `LinkerInput`, the function calls `linker_candidates` on
`input.argument`. For each candidate it performs this policy check:

```text
classify_library(candidate).is_prohibited()
    OR classify_interface_symbol(candidate).is_prohibited()
```

When the check is true, it constructs:

```text
Finding {
    category: BuildLinkInput,
    path: input.path,
    line: input.line,
    symbol: candidate,
    disposition: Blocking,
}
```

`Finding::blocking` slash-normalizes the path and leaves the candidate spelling
as the finding symbol. The function sorts all findings using `Finding`'s total
ordering and removes exact duplicates before returning them. Therefore a
prohibited candidate repeated in one argument or across identical path/line
inputs appears once, while the same candidate at different locations remains
distinct.

Library classification is evaluated first and uses a short-circuiting `OR`.
That ordering does not change the emitted finding, but it means a candidate
that is already a prohibited library does not need a second symbol-policy
classification. The input line is copied verbatim, with no one-based or
nonzero check in this module.

### Candidate extraction

`linker_candidates` always starts with the whole argument after outer
whitespace trimming. It then splits the original argument on ASCII whitespace
and these delimiters:

```text
, = : ( ) [ ] ;
```

Each split piece is trimmed of all leading and trailing `"`, `'`, `{`, and `}`
characters, and empty pieces are discarded. The complete argument is not
trimmed of braces by this helper, but its split pieces are. Candidates are
sorted with `sort_unstable` and deduplicated. No shell parsing, response-file
expansion, glob expansion, or linker execution occurs.

This intentionally lets policy normalization inspect common forms such as
`-lcudart`, `/path/to/libcudart.so`, quoted library names, and
`-Wl,-l...` components. The helper itself does not decide whether a token is a
library or symbol; that remains the policy module's job.

The whole trimmed argument remains a candidate even when it contains no
delimiters. A whitespace-only argument therefore contributes an empty
candidate and normally produces no finding; aggregate validation rejects only
an exactly empty argument, not whitespace-only text. Candidate normalization is
not a shell or linker grammar.

## `audit_elf_facts`

The function evaluates one already-populated `ElfFacts` value in two passes.

1. Every string in `elf.needed` is passed to `classify_library`. A prohibited
   library produces a `DynamicNeeded` finding with `elf.path`, line `0`, and
   the original library string as `symbol`.
2. Every `ArtifactSymbol` in `elf.undefined_symbols` is passed to
   `classify_interface_symbol`. Any `Prohibited(_)` classification produces an
   `UndefinedSymbol` finding with `elf.path`, line `0`, and the original symbol
   name.

The function does not classify `needed` values as symbols, does not classify
undefined symbols as libraries, and does not report `Allowed` or `Unknown`
values. It sorts and exact-deduplicates the combined result before returning.
Identical text in both evidence vectors can therefore yield two findings when
their categories differ. Line zero is the model's binary-evidence convention,
not a source line.

## Policy dependencies

`classify_library` normalizes a library token through the policy module before
classification. It recognizes approved CUDA Driver (`cuda`, `nvcuda`) and
ROCr/HSA runtime (`hsa-runtime64`, `hsa-runtime`) names, while prohibited
families include direct KFD, HIP, CUDA Runtime, and the listed ROCm/NVIDIA
operation libraries. It can recognize basename paths, `-l` forms, common
library suffixes, `/DEFAULTLIB:` forms, and static markers.

`classify_interface_symbol` removes the policy module's symbol decoration,
accepts the exact reviewed CUDA Driver allowlist and `hsa_` calls, and rejects
unallowlisted `cuXxx`, HIP, CUDA Runtime, direct-KFD, and operation-family
symbols. The artifact module does not duplicate or broaden these lists. In
particular, it does not call `classify_dependency`; dependency evidence is
audited separately by `DependencyGraph::audit`, and source/build strings are
handled by `source.rs`.

The resulting finding records the evidence kind and spelling, not the
`NativeInterface` family returned by policy. A candidate prohibited by either
classifier still produces one `BuildLinkInput` finding; the family is not a
separate output field.

Policy normalization is deliberately asymmetric across evidence kinds. A
`DT_NEEDED` value is treated as a library name only, and an undefined symbol is
treated as a symbol only. For example, an undefined `libcudart.so` name does
not become `UndefinedSymbol`, while a needed `cuNotReviewed` name does not
become `DynamicNeeded` merely because the symbol classifier would reject it.

## Callers and data flow

The normal CLI path is:

```text
recipe-audit CLI
  -> collect_native_scope(scope, elf_paths)
       -> read_elf_facts(...) -> ElfFacts
  -> LinkerInput::new("<command-line>", 0, each --link-input)
  -> AuditInput
  -> recipe_audit::audit(input)
       -> AuditInput::validate()
       -> audit_source(...) for sources
       -> DependencyGraph::audit() when supplied
       -> audit_linker_inputs(&linker_inputs)
       -> audit_elf_facts(&elf) for each ELF fact
       -> global sort/dedup
       -> apply exact legacy grants in legacy mode
       -> AuditReport::new(...)
  -> pretty JSON and exit code
```

`collect_native_scope` and `read_elf_facts` perform all filesystem and ELF
parsing work before this module is called. They sort and deduplicate the
extracted `DT_NEEDED` and undefined-symbol facts. `AuditInput::validate` also
checks nonempty linker arguments and ELF fact strings, normalized paths, and
unique source/ELF paths when the aggregate path is used. Direct calls to the
two re-exported functions do not perform that validation.

The collector requires an absolute, real directory scope, skips nested `.git`
and `target` directories while collecting source/build files, and accepts
explicitly named ELF files only when they are real regular files that
canonicalize beneath that scope. This is why a binary under `target` can still
reach this module when it is supplied through `--elf`, even though that nested
directory is not source-walked.

## Outputs and disposition

The functions return only in-memory `Vec<Finding>` values and have no filesystem,
process, network, logging, or global-state effects. Every finding created here
starts as `FindingDisposition::Blocking`. The aggregate `audit` function later
sorts and deduplicates findings from all audit stages and, only in `legacy` mode,
changes an exact path/line/category/symbol match to `Grandfathered` through
`apply_legacy_grants`. The artifact functions themselves never apply grants.

At the CLI boundary, a report containing any remaining blocking finding is
printed as JSON and exits with status `1`; a fully passed report exits `0`.
Any `AuditError` raised before or after artifact evaluation prints an error and
usage and exits `2`.

## Invariants

- A result contains only `BuildLinkInput`, `DynamicNeeded`, or
  `UndefinedSymbol` categories, as selected by the evidence kind.
- A result contains no `Allowed` or `Unknown` policy classification.
- Every emitted finding is blocking until the aggregate legacy-grant pass.
- Finding paths are normalized by `Finding::blocking`; symbols retain their
  original candidate, needed-library, or undefined-symbol spelling.
- Results are sorted and exact-deduplicated at each public operation and again
  by the aggregate `audit` function.
- ELF findings always use line `0`; linker findings preserve the input line,
  including CLI line `0` for command-line arguments.
- No candidate is executed, loaded, opened, or resolved by this module.

## Limits and failure boundaries

`artifact.rs` declares no byte, count, or time limits and returns no
`Result`. Its work is bounded only by the supplied slices and strings. The
surrounding CLI and collector impose these separate input bounds:

| Boundary | Limit | Enforced by |
| --- | ---: | --- |
| Artifact audit functions | none declared | caller-supplied slices and strings |
| Collected source text | 16 MiB per file | `collect_native_scope` |
| Collected source files | 100,000 files | `collect_native_scope` |
| ELF bytes | 1 GiB per file | `read_elf_facts` |
| Metadata or legacy-grant JSON | 64 MiB per file | CLI `read_bounded_text` |

The native limits are collection limits, not artifact-audit limits. The JSON
limit applies before `AuditInput` is built and does not bound linker or ELF
vectors supplied directly to the library.

Malformed direct inputs can therefore be accepted as ordinary values: an empty
linker argument yields no prohibited candidate, and empty or unrecognized ELF
facts yield no finding. Through `audit(input)`, validation rejects empty linker
arguments, empty ELF library/symbol strings, duplicate paths, and unnormalized
paths before this module runs. Collection can additionally fail with
configuration, I/O, or invalid-ELF errors. Source or dependency audit errors
occur before artifact evaluation in the aggregate order; stale or duplicate
legacy grants can fail after findings have been collected. None of those errors
are generated by `artifact.rs` itself.
