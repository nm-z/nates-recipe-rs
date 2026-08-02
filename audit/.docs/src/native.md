# `audit::native`

`audit/src/native.rs` is the filesystem and ELF evidence boundary for
`recipe-audit`. It turns one caller-selected directory plus zero or more
explicit artifact paths into typed `SourceUnit` and `ElfFacts` values. It does
not decide whether a symbol or library is allowed. That decision is made later
by `source.rs`, `artifact.rs`, and `policy.rs`.

The module has two separate entry points:

| Entry point | Input boundary | Result path identity | Main use |
| --- | --- | --- | --- |
| `collect_native_scope(scope, elf_paths)` | An absolute directory and explicit absolute ELF paths | Source and ELF paths are slash-normalized paths relative to the canonical scope | The CLI's complete native collection before `audit` runs |
| `read_elf_facts(path)` | One absolute filesystem path, with no scope argument | The supplied absolute path, slash-normalized but not canonicalized | A public library helper for one artifact or an artifact producer |

Both entry points return `Result<_, AuditError>`. A failure means that the
requested evidence was not trustworthy and is propagated as an error. The
module never returns a partial `NativeCollection` or partial `ElfFacts`.

## Public structure

`NativeCollection` (`audit/src/native.rs:15-20`) is the aggregate returned by
the scoped collector:

```text
NativeCollection {
    sources: Vec<SourceUnit>,
    elf_facts: Vec<ElfFacts>,
}
```

`SourceUnit` and `SourceKind` come from `model.rs`. Each source has normalized
display path, a language or build-metadata kind, and the complete UTF-8 file
contents. The kind is consumed by the language-aware lexer and source audit:
Rust, Zig, C, C++, LLVM IR, and build metadata use different lexical and
policy contexts (`audit/src/source.rs:1-29`, `205-229`).

`ElfFacts` also comes from `model.rs`:

```text
ElfFacts {
    path: String,
    needed: Vec<String>,
    undefined_symbols: Vec<ArtifactSymbol>,
}
```

`needed` is the sorted, duplicate-free set of ELF dynamic `DT_NEEDED`
library names. `undefined_symbols` is the sorted, duplicate-free set of names
whose symbol-table section index is `SHN_UNDEF`, collected from both the
dynamic and regular symbol tables. `ArtifactSymbol` only wraps the extracted
name. Neither fact type carries a policy classification or a source line.
Binary findings consequently use line `0` (`audit/src/artifact.rs:24-59`,
`audit/src/model.rs:90-102`).

The module is re-exported by the library facade (`audit/src/lib.rs:26`), so
library callers can use both public functions and the public collection type.
The only private helper shared by the two functions is
`read_elf_facts_with_display` (`audit/src/native.rs:111-166`).

## Fixed limits and dependencies

The implementation has three private compile-time limits:

| Constant | Value | Applied to |
| --- | ---: | --- |
| `MAX_TEXT_FILE_BYTES` | `16 * 1024 * 1024` bytes | Each supported source/build file in a scope |
| `MAX_ELF_BYTES` | `1024 * 1024 * 1024` bytes | Each ELF read by the shared parser |
| `MAX_SOURCE_FILES` | `100_000` files | The number of supported regular files discovered under the scope |

These are source-code constants, not CLI or TOML settings. There is no
aggregate byte limit for a scope, no ELF-count limit, and no streaming path:
accepted text is read into a `String`, while accepted ELF bytes are read into a
single `Vec<u8>` before parsing (`audit/src/native.rs:11-13`, `54-65`,
`119-131`).

The parser is `goblin::elf::Elf::parse`. `audit/Cargo.toml` enables Goblin's
`elf32`, `elf64`, and `endian_fd` features with default features disabled. The
module therefore depends on Goblin's supported ELF classes and byte orders, but
does not impose an architecture, machine, ABI, executable-bit, or operating
system check. It accepts a valid ELF shared object, executable, relocatable
object, or other ELF form that Goblin can parse; non-ELF formats such as PE and
Mach-O are outside this module.

## Scoped collection contract

`collect_native_scope` (`audit/src/native.rs:33-98`) follows this exact order.
The order matters because the CLI calls it before loading optional Cargo
metadata, grants, or linker inputs (`audit/src/main.rs:27-34`).

1. **Require an explicit absolute scope.** `require_absolute(scope, "scope")`
   checks only `Path::is_absolute`. A relative path fails immediately with
   `AuditError::Configuration` and no filesystem access.
2. **Check the scope entry without following its final symlink.**
   `fs::symlink_metadata(scope)` is mapped to `AuditError::Io`. A final symlink
   or a non-directory is rejected as
   `scope must be a real directory, not a symlink: ...` using
   `AuditError::Configuration`.
3. **Canonicalize the scope.** `fs::canonicalize` resolves the existing scope
   and is also an `Io` boundary. The canonical path is the root used for
   recursion and for the containment check on explicit ELF paths.
4. **Discover supported files.** `collect_paths` recursively walks the
   canonical scope. It rejects any directory entry that is a symlink, skips
   directories named exactly `.git` or `target`, and records only regular files
   for which `source_kind` returns a kind. Other non-symlink non-regular entries
   and unsupported file names are ignored.
5. **Apply the source count bound.** More than `MAX_SOURCE_FILES` collected
   files produces `AuditError::Configuration`. The count is over supported
   regular files, not every directory entry and not files in skipped
   directories.
6. **Sort discovered paths.** The `(PathBuf, SourceKind)` list is sorted by
   `PathBuf` after recursion, making source order independent of directory
   enumeration order.
7. **Read and materialize each source.** Each path is checked with
   `fs::metadata`, rejected if its byte length is greater than
   `MAX_TEXT_FILE_BYTES`, and read with `fs::read_to_string`. Metadata and read
   failures are `AuditError::Io`; invalid UTF-8 from `read_to_string` is
   reported through that same I/O error boundary.
8. **Make the source display path.** `relative_display` strips the canonical
   scope prefix and passes the result to `normalize_display_path`, which only
   replaces backslashes with forward slashes. `SourceUnit::new` applies the
   same normalization. The contents and discovered kind are retained exactly.
9. **Validate and parse each explicit ELF path.** The next section describes
   the per-path checks. Parsed facts use a path relative to the canonical scope.
10. **Sort ELF facts by display path.** The final `elf_facts` vector is sorted
    by `ElfFacts.path`, then returned with the source vector in its already
    sorted order.

The collection is therefore deterministic for a stable filesystem snapshot,
but it is not an atomic snapshot. Files can be changed between metadata checks,
canonicalization, and reads. There is no file descriptor pinning, hashing,
locking, retry, or substitution when a race occurs.

### Recursive source traversal

`collect_paths(scope, directory, output)` (`audit/src/native.rs:168-203`)
reads a directory into a vector, maps directory-entry read failures to
`AuditError::Io`, and sorts entries by `DirEntry::file_name` before processing
them. For each entry it performs `symlink_metadata` first. A symlink is always
an immediate configuration error, even if its name would otherwise be `.git` or
`target`; the skip check is reached only for a real directory.

Real directories are traversed depth-first. Names `.git` and `target` are
skipped at every depth, so source discovery intentionally excludes repository
metadata and build output. This does not prevent an explicitly supplied ELF
under `target` from being audited, because ELF paths are handled separately.
There is no explicit recursion-depth limit. A very deeply nested tree can
therefore consume call stack while being traversed.

Real regular files are passed to `source_kind`:

| Path spelling | `SourceKind` |
| --- | --- |
| `.rs` | `Rust` |
| `.zig` | `Zig` |
| `.c`, `.h` | `C` |
| `.cc`, `.cpp`, `.cxx`, `.hh`, `.hpp`, `.hxx` | `Cpp` |
| `.ll` | `LlvmIr` |
| `.toml`, `.json`, `.yaml`, `.yml`, `.cmake`, `.mk`, `.ninja`, `.bazel`, `.bzl` | `BuildMetadata` |
| `Cargo.lock`, `Makefile`, `CMakeLists.txt`, `BUILD`, `BUILD.bazel`, `WORKSPACE`, `WORKSPACE.bazel` | `BuildMetadata` |

Matching is case-sensitive and uses the final extension or exact basename.
There is no globbing, content sniffing, or language fallback. `source_kind`
requires a UTF-8 basename; extension-based matching additionally requires a
UTF-8 extension. A non-UTF-8 basename therefore returns `None` and is silently
ignored, even if its byte suffix would look like a supported extension. A
UTF-8 file with invalid contents is different: it is collected and then fails
during `read_to_string`.

The count check after each recursive directory and the final check in
`collect_native_scope` enforce the same upper bound. A single directory can be
enumerated and accumulated before its recursive call returns, so the limit is
an admission guard rather than a streaming memory cap.

### Explicit ELF admission

For every `PathBuf` in `elf_paths`, `collect_native_scope` (`:68-94`) does the
following:

* It requires an absolute path. A relative path is a configuration error before
  any metadata call.
* It calls `fs::symlink_metadata` and requires a real regular file. A final
  symlink, directory, device, socket, or other non-regular path is rejected as
  `ELF path must be a real regular file: ...` with
  `AuditError::Configuration`.
* It canonicalizes the path. Canonicalization failures are `AuditError::Io`.
  Parent path components that are symlinks may be resolved by canonicalization;
  the explicit final entry was already rejected as a symlink.
* It checks `canonical.starts_with(&canonical_scope)`. This is a component-aware
  `Path` prefix check, so the canonical artifact must remain under the selected
  scope. An artifact outside the scope is a configuration error with
  `ELF escapes explicit scope: ...`.
* It inserts the canonical path into a `BTreeSet`. A second spelling that
  canonicalizes to the same path, including `.` or `..` aliases, fails with
  `duplicate ELF path: ...`. Distinct hard-link names are not detected as the
  same file because they have distinct canonical path names.
* It computes a slash-normalized path relative to the canonical scope and calls
  `read_elf_facts_with_display` with the canonical path and that display value.

The scope boundary is enforced only by `collect_native_scope`. The public
single-file function below intentionally has no scope parameter and therefore
cannot make this containment guarantee.

## ELF fact extraction

`read_elf_facts` (`audit/src/native.rs:100-109`) is the public one-file reader.
It requires an absolute path and calls the shared helper with
`normalize_display_path(&path.display().to_string())`. It does not
canonicalize, check containment, reject symlinks, or make the display path
relative. Consequently, a symlink whose target is a regular file is accepted by
this entry point because the helper uses `fs::metadata`, which follows the
symlink. This differs deliberately from scoped collection, whose admission
checks use `symlink_metadata` and reject a final symlink.

`read_elf_facts_with_display` (`:111-166`) then performs the common parse:

1. `fs::metadata(path)` must report a regular file. A directory or other
   non-file produces `AuditError::InvalidElf` with reason `not a regular file`.
   Metadata failure itself is `AuditError::Io`.
2. The reported byte length must be at most `MAX_ELF_BYTES`. A larger file
   produces `InvalidElf` with reason `file exceeds 1073741824 bytes` (the
   formatted value of the constant).
3. `fs::read(path)` reads the complete file. Read failure is `AuditError::Io`.
4. `Elf::parse(&bytes)` validates and decodes the ELF. Any Goblin parse error
   becomes `AuditError::InvalidElf` with the parser's text in `reason`.
5. `elf.libraries` is copied into `needed`, then sorted and deduplicated. These
   are the dynamic dependency names represented by ELF `DT_NEEDED` entries.
6. A `BTreeSet<String>` is populated from `elf.dynsyms`. A symbol is included
   only when `st_shndx == SHN_UNDEF`, `st_name != 0`, its dynamic string-table
   index resolves through `dynstrtab.get_at`, and the resolved name is not
   empty.
7. The same conditions are applied to `elf.syms` and `strtab.get_at`, adding
   names to the same set. This combines undefined dynamic and regular/static
   symbol evidence and removes duplicates across the two tables.
8. `ElfFacts::new(display, needed, ...)` turns the ordered set into
   `ArtifactSymbol` values. The resulting undefined-symbol order is the
   `BTreeSet` order, so it is deterministic.

The reader does not extract `DT_SONAME`, `DT_RPATH`, `DT_RUNPATH`, relocations,
defined symbols, section contents, symbol binding/visibility, architecture,
interpreter, or loader behavior. It also does not normalize library or symbol
spellings beyond the display path. `artifact.rs` later applies policy's exact
library and symbol normalization to the copied strings.

The facts are limited by what Goblin exposes through these fields. Goblin's
dynamic parser logs and skips a `DT_NEEDED` entry whose string-table offset is
invalid, so native collection can return a valid `ElfFacts` without that
library name. Dynamic symbol-table sizing is derived from the ELF hash tables
and relocation references; a format that has no usable hash or relocation
count can expose an empty `dynsyms` iterator even when bytes resembling a
dynamic symbol table are present. The regular symbol table is likewise the
single `SHT_SYMTAB` table selected by Goblin. Native code treats an empty or
unresolvable table as no symbol evidence and does not surface those omissions
as a separate audit error.

### What the extracted facts mean downstream

The CLI's `run` function passes `NativeCollection.elf_facts` into
`AuditInput` (`audit/src/main.rs:76-87`). `recipe_audit::audit` validates those
facts and invokes `audit_elf_facts` for each (`audit/src/lib.rs:42-64`). The
artifact audit then:

* classifies each `needed` library with `classify_library`; prohibited names
  become `FindingCategory::DynamicNeeded`, with the ELF display path, line `0`,
  and the original library string (`audit/src/artifact.rs:24-39`); and
* classifies each undefined symbol with `classify_interface_symbol`; prohibited
  symbols become `FindingCategory::UndefinedSymbol`, with line `0` and the
  original symbol string (`audit/src/artifact.rs:41-58`).

Allowed and unknown libraries or symbols produce no finding. The native module
does not know which policy family is allowed, does not classify ROCr/HSA or
CUDA Driver symbols, and does not produce a pass/fail value. `audit` sorts and
deduplicates findings across source, dependency, linker, and ELF evidence
(`audit/src/lib.rs:49-67`), and the report computes `passed` only after that
aggregation.

The collector's source kinds have the same downstream role. `SourceUnit`
contents are passed to `audit_source`, which lexes according to the kind and
emits source, runtime-load, LLVM, or build-link findings. In particular, a
`.ll` file is marked `LlvmIr`, while build files and lock/configuration files
are marked `BuildMetadata`; the collector itself never interprets their text.

## Path and determinism rules

The module uses two path forms deliberately:

* **Filesystem paths** are `Path` or `PathBuf` values used for metadata,
  canonicalization, directory traversal, and reads. Errors retain these paths
  in `AuditError::Io` or `AuditError::InvalidElf`.
* **Display paths** are `String` values placed in `SourceUnit.path` and
  `ElfFacts.path`. `normalize_display_path` only replaces `\\` with `/`; it
  does not resolve `.` or `..`, collapse separators, case-fold, or verify that
  a path is canonical.

For scoped collection, source and ELF display paths are relative to the
canonical scope. For direct `read_elf_facts`, the display path is based on the
caller-supplied absolute spelling. `Finding::blocking` and the model validators
expect normalized display strings later (`audit/src/model.rs:105-126`,
`331-339`). A direct caller that supplies a non-canonical display path or a
caller whose filenames collide after backslash replacement can therefore reach
the later `AuditInput::validate` error rather than being repaired here.

The following ordering and deduplication decisions make successful collection
stable for an unchanged filesystem:

* directory entries are sorted by basename before recursion;
* collected source paths are sorted by `PathBuf` after recursion;
* `DT_NEEDED` names are sorted and deduplicated;
* undefined dynamic and regular symbols share a `BTreeSet` and are emitted in
  sorted order; and
* scoped ELF facts are sorted by normalized display path, while canonical-path
  duplicates are rejected before parsing.

The implementation does not snapshot directory contents. An external mutation
can still make a successful result depend on timing, or turn an otherwise
valid path into an `Io`, configuration, or parse error. No retry or alternate
path is attempted.

## Failure contract

The native module constructs or propagates these `AuditError` variants:

| Condition | Variant | Boundary |
| --- | --- | --- |
| Relative scope or ELF path | `Configuration` | `require_absolute` |
| Missing/inaccessible scope, directory, entry, source, ELF, canonicalization, or byte read | `Io` | `symlink_metadata`, `canonicalize`, `read_dir`, `metadata`, `read_to_string`, `read` |
| Final scope symlink, wrong scope type, symlink in recursive scope | `Configuration` | Scoped admission and `collect_paths` |
| More than 100,000 supported source files | `Configuration` | Count checks in `collect_native_scope` and `collect_paths` |
| Supported text file over 16 MiB | `Configuration` | Per-source metadata check |
| Explicit ELF final symlink or non-regular path | `Configuration` | Scoped ELF admission |
| Explicit ELF canonical path outside scope | `Configuration` | Scoped containment check |
| Repeated explicit ELF canonical path | `Configuration` | Scoped `BTreeSet` |
| Direct reader path not a regular file | `InvalidElf` | Shared parser helper |
| ELF over 1 GiB | `InvalidElf` | Shared parser helper |
| Goblin rejects bytes as ELF | `InvalidElf` | `Elf::parse` mapping |
| Path cannot be made relative to canonical scope | `Configuration` | `relative_display` |

The scoped collector fails on the first error encountered in its deterministic
walk or explicit-ELF argument order. It does not retain earlier sources or
ELFs as a partial return. Once a `NativeCollection` is returned, later
`AuditInput::validate` can still reject duplicate normalized display paths or
empty manually constructed facts, but facts produced by the normal collector
already satisfy the collector's own path and content checks.

At the CLI boundary, any native error is printed as `recipe-audit: {error}`
with usage text and exits with code `2`; a valid report containing blocking ELF
findings is instead JSON with exit code `1`, and a valid report with no blocking
findings exits `0` (`audit/src/main.rs:9-17`, `88-96`). The distinction keeps an
unreadable or malformed artifact from being mistaken for a clean audit.

## Purpose and non-goals

The module's purpose is narrow and concrete: establish bounded, explicit,
deterministically ordered filesystem and ELF facts for the policy audit. Its
non-goals are equally important:

* it does not walk Cargo dependency metadata, parse linker arguments, or inspect
  runtime behavior;
* it does not audit source text, classify names, or apply legacy grants;
* it does not infer dependencies or prohibited interfaces from arbitrary bytes,
  strings, relocations, or defined symbols;
* it does not select the current directory, call `cargo metadata`, search for
  ELF files implicitly, or include `target` artifacts without an explicit
  `--elf` path at the CLI; and
* it does not offer policy, path, size, symlink, or error fallbacks.

The separation is intentional. Collection provides only evidence that reached
the explicit boundary. `artifact.rs` and the rest of the audit pipeline decide
what that evidence means, and any missing, malformed, or ambiguous input is
reported as an error instead of silently substituted.
