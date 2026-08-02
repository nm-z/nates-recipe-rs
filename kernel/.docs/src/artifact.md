# `recipe_kernel::artifact`

`kernel/src/artifact.rs` is the byte-level acceptance boundary for Recipe
native kernel images. It does not compile LLVM, choose a target, allocate a
runtime resource, or write a public model file. It parses the serialized ELF
image returned by the builder, checks the image identity and entry ABI supplied
by the caller, and returns typed inspection data plus a SHA-256 image digest.

The public surface is re-exported by `kernel/src/lib.rs`:

| Item | Role |
| --- | --- |
| `ArtifactDigest` | 32-byte SHA-256 value for bytes, with raw-byte and hexadecimal views. |
| `HsaKernelArgument` | One AMDHSA metadata argument. |
| `HsaKernelMetadata` | One AMDHSA metadata kernel record. |
| `InspectedHsaco` | HSACO image and one validated entry-point inspection. |
| `InspectedCubin` | Cubin image and one validated entry-point inspection. |
| `inspect_hsaco` | Inspect one AMD code object entry. |
| `inspect_hsaco_bundle` | Inspect every requested entry in one shared AMD code object. |
| `inspect_cubin` | Inspect one NVIDIA cubin entry. |

The module has no encoder. `ArtifactBuilder` in `kernel/src/builder.rs`
serializes LLVM modules through pinned LLVM, an ELF linker, or `ptxas`; this
module parses and authenticates the resulting bytes. The core
`ArtifactIdentity` and `ArtifactBuildRecipe` are in-memory contracts. They are
hashed by planning and preparation, and the remote manifest transmits only
artifact IDs and image digests, not the full identity or image.

## Digest and inspection data

### `ArtifactDigest`

```rust
pub struct ArtifactDigest([u8; 32]);
```

`ArtifactDigest::of(bytes)` computes SHA-256 over exactly the supplied byte
slice. `bytes()` returns the array without conversion. `to_hex()` returns 64
lowercase hexadecimal characters, two per digest byte. The wrapper derives
ordering, equality, hashing, copying, and debugging, so it is suitable as a
stable key for loaded-image maps. It is distinct from
`recipe_core::Digest`, although Realize copies the same 32 bytes into that
core type when constructing `ArtifactIdentity`.

`inspect_hsaco_bundle` computes the digest once from the complete image and
copies it into every per-entry inspection. `inspect_cubin` is a one-entry
operation and computes the same digest for that call; the cubin bundle builder
invokes it once per requested entry. A multi-entry image therefore has one
content identity while each entry retains its own ABI and symbol identity.

### AMD inspection records

`HsaKernelArgument` contains:

| Field | Meaning |
| --- | --- |
| `name: Option<String>` | Optional AMDHSA metadata name. Names are contradiction checks, not the ABI ordering source. |
| `offset: u64` | Byte offset in the kernarg segment. |
| `size: u64` | Serialized argument size. |
| `value_kind: String` | AMDHSA value-kind string, for example `global_buffer` or `by_value`. |
| `address_space: Option<String>` | Optional AMDHSA address-space string. |

`is_hidden()` is true when `value_kind` starts with `hidden_`. Hidden trailing
arguments are allowed by the ABI validator, while a hidden argument in the
explicit prefix is rejected.

`HsaKernelMetadata` contains the required kernel `.name`, descriptor `.symbol`,
ordered `.args`, and the numeric AMDHSA resource fields
`kernarg_segment_size`, `kernarg_segment_alignment`,
`group_segment_fixed_size`, `private_segment_fixed_size`,
`maximum_workgroup_size`, and `wavefront_size`. The latter fields are returned
for runtime resource setup. `validate_hsa_abi` directly compares descriptor,
kernarg alignment and size, maximum workgroup size, and argument metadata to
the expected `KernelAbi`.

`InspectedHsaco` reports:

```rust
pub struct InspectedHsaco {
    pub digest: ArtifactDigest,
    pub elf_abi_version: u8,
    pub code_object_version: u8,
    pub elf_flags: u32,
    pub target_id: String,
    pub kernel: HsaKernelMetadata,
}
```

The code-object version is decoded as `ELF e_ident[EI_ABIVERSION] + 2`, with
checked addition. The target ID is the metadata value, retained in the exact
spelling found in the image.

### NVIDIA inspection records

```rust
pub struct InspectedCubin {
    pub digest: ArtifactDigest,
    pub elf_abi_version: u8,
    pub elf_flags: u32,
    pub sm: u8,
    pub entry_symbol: String,
    pub text_bytes: u64,
}
```

`sm` is the decoded CUDA SM number, such as `86` for `sm_86`.
`text_bytes` is the size of `.text.<entry_symbol>`. The parser requires that
section to be nonempty and executable and requires a nonzero function-symbol
size.

## Shared ELF parser

`ElfFile::parse` is deliberately small and fail-closed. All malformed byte
ranges become `LoweringErrorKind::ArtifactFormat`; semantic identity
contradictions become `LoweringErrorKind::ArtifactMismatch` in the caller.

The parser performs these operations in order:

1. Require at least 64 bytes and the ELF magic `0x7fELF`.
2. Require ELF class 64 and little-endian data encoding (`EI_CLASS == 2`,
   `EI_DATA == 1`).
3. Read the section-table offset at byte 40, section-header size at byte 58,
   section count at byte 60, and section-name string-table index at byte 62.
4. Require a 64-byte section header and checked section-table multiplication and
   addition. The table must fit in the image and the string-table index must be
   less than the section count.
5. Read the section-name string table. It may not be `SHT_NOBITS`; its byte
   range must fit. Every section name is a NUL-terminated UTF-8 string.
6. Decode each fixed-layout section header into `ElfSection`:
   `name`, `kind`, `flags`, `offset`, `size`, `link`, and `entry_size`.
   Every non-`SHT_NOBITS` section's file range is checked immediately. NOBITS
   sections are retained for lookup but have no file data.

Section-header fields are read as little-endian unsigned values. `raw_section`
uses checked index arithmetic and the common `subslice` helper, so a wrapping
offset, a host-size conversion failure, or an out-of-bounds range is a format
error rather than a panic.

`ElfFile::section(name)` returns the first section with that name. It is used
for cubin code sections. `section_data` refuses NOBITS sections and then checks
the stored range again before returning a borrowed slice.

### Symbols

`ElfFile::symbols()` scans both `SHT_SYMTAB` (2) and `SHT_DYNSYM` (11).
Each symbol table must have a 24-byte ELF64 entry size and a valid linked
string-table section. Entries are decoded as follows:

| ELF symbol bytes | `ElfSymbol` field |
| --- | --- |
| `st_name` at offset 0 | UTF-8 NUL-terminated name from the linked string table |
| `st_info` high nibble | binding |
| `st_info` low nibble | kind |
| `st_shndx` at offset 6 | section index |
| `st_size` at offset 16 | symbol size |

The unnamed symbol is ignored. Symbols are sorted by name, section, binding,
kind, and size, then exact duplicates are removed. `require_symbol` and
`require_indexed_symbol` only accept a defined (`section != 0`) global or weak
symbol of the requested kind. `audit_symbols` rejects every unresolved global
or weak symbol, reporting all distinct names in sorted order. This audit runs
for both HSA and CUDA images before entry-specific checks.

The parser intentionally does not infer an entry from a symbol that merely has
the right text section. The symbol's name, kind, binding, defined section, and
the target-specific code section must all agree.

## HSACO inspection

`inspect_hsaco(bytes, expected_target_id, expected_code_object_version,
expected_abi)` is a one-entry convenience wrapper. It calls
`inspect_hsaco_bundle` with an iterator containing exactly one ABI and returns
the sole result. Its internal `expect` is reached only after that one-element
iterator has produced one inspection.

`inspect_hsaco_bundle` decodes shared ELF state and metadata once, then maps
the requested ABIs in caller order. Its validation pipeline is:

1. Parse ELF and require OS ABI `64` (`ELFOSABI_AMDGPU_HSA`) and machine `224`
   (`EM_AMDGPU`). A different machine or OS ABI is an artifact mismatch.
2. Compute `code_object_version = elf.abi_version + 2` with checked addition and
   compare it exactly to the expected version.
3. Decode and audit all symbols. Build a set of defined global or weak
   `(name, kind)` pairs for fast per-entry lookup.
4. Locate an AMDGPU metadata note, decode its MessagePack root map, and require
   a target ID under either `amdhsa.target` or `.amdhsa.target`.
5. Compare image and expected target IDs with `target_ids_match`.
6. Require a kernel array under either `amdhsa.kernels` or `.amdhsa.kernels`.
   Every element must be a map with a string `.name`, and duplicate names are a
   format error. The maps are indexed by name in a `BTreeMap`.
7. Compute one image digest. For each expected ABI, in order:
   * require a defined global or weak function symbol named exactly the entry;
   * require a defined global or weak object symbol named `<entry>.kd`;
   * require a metadata kernel named exactly the entry;
   * parse that kernel map into `HsaKernelMetadata`; and
   * validate its complete explicit argument ABI against `KernelAbi`.

The function returns one `InspectedHsaco` per requested ABI. An empty expected
ABI iterator is legal and returns an empty vector after validating the shared
image and metadata. The builder never passes an empty bundle, and the runtime
passes the exact entries assigned to one device.

### AMD target comparison

`target_ids_match` accepts both the short AMD target (`gfx1101:xnack-`) and a
full triple with a target suffix (`amdgcn-amd-amdhsa--gfx1101:xnack-`). It
strips everything through the final `--`, then parses:

```text
processor[:feature+|- ...]
```

The processor must be nonempty. Each feature must end in one `+` or `-`, have a
nonempty name, and occur at most once. Feature order is ignored because the
features are compared as a `BTreeMap<name, enabled>`. Processor spelling and
feature polarity are otherwise exact. A malformed target ID yields no match,
which is an artifact mismatch at the public inspection boundary.

### AMDGPU note and MessagePack metadata

`hsa_metadata` scans every `SHT_NOTE` section (type 7). Each note has three
little-endian `u32` values, name size, description size, and note type. Name and
description ranges are checked and advanced to four-byte boundaries. The first
note whose type is 32 and whose name, up to its first NUL, is `AMDGPU` is
decoded. Exactly one MessagePack value must consume the description, and that
value must be a map. If no matching note exists, the image is an
`ArtifactFormat` error.

`parse_hsa_kernel` requires these map keys:

| Key | Expected decoded value |
| --- | --- |
| `.name` | UTF-8 string |
| `.symbol` | UTF-8 string, later required to equal `<entry>.kd` |
| `.args` | Array of maps |
| `.kernarg_segment_size` | Nonnegative integer |
| `.kernarg_segment_align` | Nonnegative integer |
| `.group_segment_fixed_size` | Nonnegative integer |
| `.private_segment_fixed_size` | Nonnegative integer |
| `.max_flat_workgroup_size` | Nonnegative integer |
| `.wavefront_size` | Nonnegative integer |

Each `.args` map requires `.offset`, `.size`, and `.value_kind` and may omit
`.name` and `.address_space`. `MsgMapExt::unsigned` accepts an unsigned
MessagePack integer or a signed integer that converts to a nonnegative `u64`;
negative values and all other value kinds are treated as missing required
metadata.

### AMD HSA ABI checks

`validate_hsa_abi` treats `KernelAbi.arguments` as the authoritative ordered
explicit prefix. It requires:

* metadata `.symbol == <entry_symbol>.kd`;
* metadata kernarg alignment exactly equals `KernelAbi.argument_alignment`;
* metadata maximum workgroup size is at least the expected workgroup width;
* metadata kernarg segment size is at least `KernelAbi.argument_bytes`; and
* metadata contains at least as many arguments as the expected explicit list.

For explicit argument index `i`, metadata offset must be `i * 8`, size must be
8, and the argument may not be hidden. The multiplication is checked. The
remaining kind checks are:

| Expected `KernelArgument` | Required metadata |
| --- | --- |
| `Buffer { access: Read, .. }` | `value_kind == global_buffer`, address space `global`; optional name starts with `input_`. |
| `Buffer { access: Write, .. }` | `value_kind == global_buffer`, address space `global`; optional name starts with `output_`. |
| `FaultFlag` | Optional name is exactly `fault_flag`, `global_buffer`, address space `global`. |
| `ElementCount` | Optional name is exactly `element_count`, `by_value`. |
| `RunId` | Optional name is exactly `run_id`, `by_value`. |
| `LoopIteration` | Optional name is exactly `loop_iteration`, `by_value`. |

Names are optional because AMDHSA metadata and full LTO may omit them. When a
name exists, it is a contradiction check. Ordering, offset, size, value kind,
and address space remain mandatory. Any metadata after the explicit prefix must
be hidden. A non-hidden trailing argument is rejected, so the image cannot
silently add a dynamic ABI value.

The expected argument list is produced by stage lowering, not by parsing
metadata. `StageSignature` emits readable bindings first, writable bindings
second, one fault pointer when checked arithmetic requires it, optional
`run_id` and `loop_iteration`, and `element_count` last. A `ReadWrite` binding
therefore contributes separate read and write pointer slots. The inspector
checks that serialized HSA metadata preserves this exact ABI.

## Cubin inspection

`inspect_cubin(bytes, expected_sm, expected_entry_symbol)` uses the same ELF
parser and unresolved-symbol audit, then applies this sequence:

1. Accept only CUDA OS ABI tags 51 (`ELFOSABI_CUDA`), 41
   (`ELFOSABI_CUDA_V2`), or 65 (the observed toolkit 13.3 ABI), and machine
   190 (`EM_CUDA`). Other architecture-specific OS ABI tags are not accepted.
2. Decode the SM from `e_flags` using the ABI layout selected by OS ABI and
   ABI version. For tags 51 and 41, the low byte is the SM. For tag 65 with
   `elf.abi_version >= 8`, bits 8 through 15 carry the decimal SM and the low
   byte carries ABI flags. No unrelated byte is scanned for a coincidental
   match.
3. Require the decoded SM to equal `expected_sm`.
4. Require a defined global or weak function symbol named exactly
   `expected_entry_symbol`.
5. Require section `.text.<expected_entry_symbol>`, a nonzero section size, the
   executable flag (`SHF_EXECINSTR`, bit `0x4`), and a nonzero symbol size.
6. Return the image digest, ELF ABI version and flags, decoded SM, exact entry
   symbol, and text size.

The cubin path has no MessagePack metadata dependency. The symbol and text
section are the structural proof that the requested entry has code. Missing
sections, unresolved globals, target mismatch, or an empty/non-executable
entry are `ArtifactMismatch`; malformed ELF ranges and an unrepresentable text
size are `ArtifactFormat`.

## MessagePack decoder contract

The private `MsgDecoder` is a bounded, allocation-owning decoder for the
AMDGPU note. It supports exactly these marker families:

| Marker family | `MsgValue` |
| --- | --- |
| positive fixint, `uint8`, `uint16`, `uint32`, `uint64` | `Unsigned(u64)` |
| negative fixint, `int8`, `int16`, `int32`, `int64` | `Signed(i64)` |
| fixstr, `str8`, `str16`, `str32` | UTF-8 `String(String)` |
| fixarray, `array16`, `array32` | `Array(Vec<MsgValue>)` |
| fixmap, `map16`, `map32` | `Map(BTreeMap<String, MsgValue>)` |
| `nil` | `Nil` |
| `false`, `true` | `Boolean(bool)` |

Binary blobs, floating-point markers, extension markers, reserved markers, and
all other MessagePack forms are rejected. MessagePack integer widths and
lengths are big-endian, unlike the ELF fields. Strings must be valid UTF-8.
Map keys must be strings and duplicate keys are rejected. Recursion depth is
limited to 128 nested values. Every byte and length operation uses checked
cursor arithmetic, and the metadata caller requires the decoder to finish at
the exact end of the note description.

## Native image production and serialization

The artifact bytes inspected here are produced by `ArtifactBuilder`:

| Entry point | Inputs and serialized intermediates | Final image |
| --- | --- | --- |
| `build` with `KernelTarget::Amd` | LLVM IR, pinned verifier, LLVM object emission, pinned ELF linker | One HSACO, inspected with `inspect_hsaco`. |
| `build` with `KernelTarget::Nvidia` | LLVM IR, pinned verifier, LLVM PTX emission, pinned `ptxas` | One cubin, inspected with `inspect_cubin`. |
| `build_hsaco_bundle` | Each LLVM module is verified and serialized to bitcode, then one full-LTO ELF link | One multi-entry HSACO, inspected once for every input ABI. |
| `build_cubin_bundle` | Each LLVM module is verified, lowered to one PTX unit, then all PTX units are passed to one `ptxas` call | One multi-entry cubin, inspected once for every input ABI. |

`BuildPhase` has only `Offline` and `Realize`. No compiler entry point exists
for Finalize, init, loop, or exit. The builder validates the requested
`KernelTarget`, requires the lowered module's target to equal it, verifies each
pinned tool before use, and rejects an empty bundle or repeated entry symbol.
AMD bundles require a pinned ELF linker. NVIDIA bundles require a pinned
`ptxas`.

`PinnedTool::inspect` canonicalizes an absolute path, requires a regular file,
and records the file's `ArtifactDigest`. `ArtifactBuilder::new` verifies the
verifier, LLVM code generator, and any configured linker or assembler before
retaining the toolchain. Every later invocation re-canonicalizes the path,
requires it to equal the pinned path, rereads the file, and compares its digest.
A replaced executable therefore fails as `ArtifactMismatch`; a missing,
non-absolute, non-regular, or unreadable tool is `Io`.

Bundle input order is significant and must already be deterministic. The
returned `inspections` and the LLVM digest vector in
`HsacoBundleProvenance` or `CubinBundleProvenance` preserve that order. The
image itself can be shared by several logical `ArtifactIdentity` values, one
per entry point, while `ArtifactDigest` remains the digest of the shared bytes.

Single-image `BuildProvenance` records the phase, SHA-256 of the source LLVM
IR, every invoked pinned tool path and digest, and normalized tool arguments.
The bundle provenance records vectors of LLVM digests with the same input
order. Scratch paths in recorded arguments are normalized to `@scratch` so
ephemeral directory names do not enter provenance.

Build serialization is constrained to private scratch state. LLVM input files
are created with `create_new`, mode `0600`, and `sync_all`. The scratch parent
must be an existing non-symlink directory with no group or other permissions.
Each unique `recipe-build-<pid>-<nonce>` child is mode `0700` and
is removed on `BuildWorkspace` drop. Outputs must be regular non-symlink files.
Tool invocations clear the environment and set `LC_ALL=C` and
`SOURCE_DATE_EPOCH=0`. Compiler failure is returned as `ToolchainFailed` with
stderr bounded to 16 KiB; it is not replaced by another compiler or artifact.

`BuiltArtifact::Cubin` retains both PTX bytes and cubin bytes for the caller,
while HSACO retains only HSACO bytes. Bundle return values retain the one image
and all per-entry inspections. None of these types serializes a
`recipe_core::ArtifactIdentity`; preparation constructs that identity after
inspection.

## Target and stage relationship

`KernelTarget` is validated before lowering or building:

| Target | Validation and derived values |
| --- | --- |
| `AmdTarget { target_id, code_object_version }` | Target ID is a nonempty token containing only ASCII alphanumeric, `_`, `-`, `+`, `:`, or `.`; it starts with `gfx`; code-object version is nonzero. The target ID may include feature modifiers. |
| `NvidiaTarget { sm_major, sm_minor, ptx_isa }` | Major is 3 through 12, minor is at most 9, and PTX ISA is 32 through 90. `architecture()` returns `sm_<major><minor>`, and `llvm_ptx_feature()` returns `+ptx<isa>`. |

The planner lowers each `LoweredProgram` stage and computes a collision-checked
stage identity in domain `recipe-planner-stage-template-v1` over the complete
program digest, source kernel ID, and stage ordinal. The first eight digest
bytes, interpreted as a little-endian `u64`, become `KernelTemplateId`; zero is
reserved. For each stage, planner construction reserves
`ArtifactId::new(stage_template.get())`, stores `source_kernel` separately,
copies ordered bindings, dispatch geometry, operation bounds, fault storage,
and resource bounds into `ArtifactBuildRecipe`, then computes
`provenance.contract_digest` in domain
`recipe-planner-artifact-build-v1`.

The public `recipe_kernel::artifact_build_contract_digest` is an independent
recomputation of that planner digest. It hashes, in order, artifact ID, stage
template ID, source kernel ID, program digest, stage ordinal, binding count and
each ordered binding's value, dtype tag, access tag, extents, offset, strides,
and storage bytes, then dispatch geometry, FLOP/integer/atomic bounds, optional
fault value, and all resource bounds. Byte strings are decimal-length-prefixed
with `length:`; integers are little-endian; access tags are `Read=0`,
`Write=1`, `ReadWrite=2`, and `ReadWriteAtomic=3`. The contract digest field
itself is not included in its own input.

`lower_stage` is the stage-to-LLVM verifier immediately before compilation. It
requires the complete lowered program and recipe to validate, target and
lowering options to validate, matching program digest and source kernel,
present stage ordinal, canonical stage template identity, and
`artifact.get() == kernel_template.get()`. It independently recomputes the
contract digest. It then compares dispatch geometry, work bounds, resource
envelope, binding count and every ordered binding field to the primitive
stage, and checks the exact fault binding. Any mismatch is
`InvalidStageContract`; it does not choose a nearby stage or repair stale data.

`LoweringOptions.entry_symbol` must be an ASCII LLVM identifier. Preparation
uses `recipe_stage_<artifact id>` for deferred stages. The workgroup width in
the options must equal the immutable stage geometry. The emitted ABI preserves
the binding access projection, optional fault pointer, dynamic run and loop
arguments for stages that need them, and `element_count` as the final argument.

## Realize, identity construction, and finalized ownership

`prepare::DeferredArtifactCompiler` groups deferred recipes by the one measured
`TargetIdentity` used by their calculation tasks. A deferred recipe that is
unused, assigned to more than one target, or lacks a matching
`TargetBuildSpec` fails. For each group it locates the exact lowered program by
`source_kernel` and `provenance.program_digest`, calls `lower_stage`, then
either inspects a configured prebuilt bundle or invokes the appropriate
`ArtifactBuilder` method with `BuildPhase::Realize`.

`native_artifact_from_image` constructs one `ArtifactIdentity` per lowered
entry:

| Identity field | Source |
| --- | --- |
| `id` | `build.artifact` |
| `digest` | SHA-256 of the shared runtime image bytes |
| `format` | `TargetBuildSpec.target_identity.abi` |
| `target` | Exact measured target identity from the build specification |
| `toolchain` | Pinned toolchain identity from the build specification |
| `entry_symbol` | Lowered `KernelAbi.entry_symbol` |
| `kernel_template` | `build.kernel_template` |
| `resources` | `build.resources` |
| `build` | `Some(build.provenance)` |

The paired runtime object retains the same bytes and digest, the lowered
`KernelAbi`, and a backend kind. Native preparation rejects any mismatch in
ID, digest, entry symbol, format versus target ABI, backend, architecture,
code-object ABI, or target ID before the pair enters the catalog or candidate
session. Prebuilt identities are accepted only when the current catalog
identity equals the immutable Draft identity exactly.

Draft keeps `artifacts` and `artifact_builds` disjoint. Finalize requires every
prebuilt identity unchanged and exactly one realized identity for every
deferred recipe, with matching stage identity, provenance, resources, and
measured target. The finalized bundle stores identities and deferred recipes,
not native bytes. Runtime bytes stay in the prepared native session and are
checked again by the executor.

## Runtime consumers

`native-executor` uses `RuntimeArtifact` as the byte-side object:

```rust
pub struct RuntimeArtifact {
    id: ArtifactId,
    bytes: Arc<[u8]>,
    digest: ArtifactDigest,
    abi: KernelAbi,
    kind: RuntimeArtifactKind,
}
```

`ExecutionPlan::validate_scoped` builds a map by `ArtifactId`, requires every
finalized identity exactly once, rejects duplicate and unexpected runtime
images, compares runtime and identity digests and entry symbols, requires
`format == target.abi`, and checks nonzero ABI workgroup width within the
identity resource maximum. It also validates backend-specific target metadata.
Every calculation task must name the same stage-scoped `kernel_template` as its
artifact identity.

Before loading, the CUDA backend groups runtime entries by
`ArtifactDigest`, validates each cubin for the selected SM and entry symbol,
loads one driver module per distinct image, then resolves one function per
logical artifact entry. HSA does the corresponding digest grouping,
`inspect_hsaco_bundle` validation for all entries, one executable per distinct
image, and descriptor-symbol lookup for each logical entry. The shared-image
optimization does not merge logical artifact IDs or their ABIs.

At submission, the executor looks up the artifact ID, fills parameters from the
validated `KernelAbi`, derives the grid from `elements` and workgroup width,
and launches the exact resolved entry. No runtime path accepts a byte image by
filename alone or infers a substitute entry.

The remote manifest is a separate wire serialization boundary. It sorts
`FinalizedBundle::artifacts()` by ID and transmits, for each entry, an
8-byte little-endian artifact ID followed by its 32-byte core image digest.
The fixed manifest payload uses 40 bytes per artifact, a `u32` artifact count,
the bundle, Draft, realization, manifest, and program digests, and the
`recipe-remote-manifest-v1` SHA-256 proof. Decoding rejects count overflow,
configured-limit overflow, zero IDs or digests, non-increasing IDs, and a
manifest digest that does not recompute. It deliberately does not transmit
target labels, toolchain identity, ABI, or native bytes; each worker must have
the corresponding validated runtime artifact locally.

Public training `.ogdl`, `.cubin`, and `.hsaco` files are another layer and are
not emitted by `artifact.rs`. Native save retains the exact image bytes that
passed this inspector. A semantic model or a remote manifest is not a
replacement native image.

## Error taxonomy

The private constructors centralize the two artifact-specific kernel errors:

* `artifact_format(message)` returns `LoweringErrorKind::ArtifactFormat`.
  It covers malformed ELF, section or symbol ranges, missing NUL-terminated
  UTF-8 strings, integer conversion and alignment overflow, missing or
  malformed AMDGPU notes, unsupported/truncated MessagePack, duplicate map or
  kernel metadata keys, required metadata of the wrong type, and a text size
  that cannot fit `u64`.
* `artifact_mismatch(message)` returns `LoweringErrorKind::ArtifactMismatch`.
  It covers the wrong backend ELF, code-object or SM target, target-feature
  contradiction, unresolved globals, missing defined entry/descriptor symbols,
  absent code sections, empty or non-executable code, and every HSA ABI or
  metadata contradiction.

Other artifact-path errors come from the surrounding builder and target
layers:

| Kind | Typical source |
| --- | --- |
| `InvalidTarget` | Target range/token failure, missing pinned linker or `ptxas`, malformed AMD feature list. |
| `InvalidEntrySymbol` | Non-ASCII LLVM identifier or duplicate bundle entry symbol. |
| `InvalidWorkgroupSize` | Lowering option outside 1 through 1024 lanes. |
| `InvalidStageContract` | Recipe differs from the canonical lowered stage or independent contract digest. |
| `ToolchainFailed` | Pinned compiler exits unsuccessfully; stderr is bounded and returned. |
| `Io` | Tool, scratch, input, or output path is not the required private regular file/directory. |
| `ArithmeticOverflow` | Target SM identity, stage ABI, or builder size arithmetic overflows. |

No parser error retries with another format, compiler, target layout, or
entry. The caller must repair the underlying image or contract.

## Invariant checklist

For an accepted native entry, all of these are true:

1. The bytes are a little-endian ELF64 image with a target-specific machine and
   explicitly supported OS ABI.
2. Every non-hidden global or weak symbol is resolved, and the requested entry
   is a defined function with target-specific code storage.
3. The image target and ABI version match the measured target supplied by the
   caller.
4. The image digest is SHA-256 over the exact bytes that will be retained and
   loaded.
5. An HSACO's AMDGPU metadata is one complete MessagePack map with unique
   kernel names and a serialized explicit argument ABI equal to `KernelAbi`.
6. A cubin's requested entry has a nonempty executable `.text.<entry>` section
   and a nonzero function symbol.
7. The entry was lowered from the exact stage-scoped identity and immutable
   build contract, not a compatible-looking neighboring stage.
8. Preparation paired the inspected bytes, digest, ABI, target, format,
   toolchain, resources, and provenance in one validated runtime artifact.
9. Candidate and finalized validation require the complete artifact set exactly
   once, and native execution repeats identity and ABI checks before load.
10. Shared multi-entry images may be loaded once, but every logical artifact
    entry retains its own ID, symbol, ABI, and stage relationship.
