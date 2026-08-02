<!--
Intent: describe the complete build-script boundary for recipe-ops. The script
turns the preserved operation-surface.txt inventory into one compile-time Rust
source file. It does not implement operation semantics, lower kernels, or
perform runtime discovery.
-->

# `recipe-ops/build.rs`

[`ops/build.rs`](../build.rs) is the Cargo package build script for
`recipe-ops`, declared by [`ops/Cargo.toml`](../Cargo.toml). Its only generated
program input is the preserved operation inventory at
[`operation-surface.txt`](../../operation-surface.txt). The script parses that
file, preserves source order and duplicate symbols, and writes a Rust source
fragment named `operation_surface.rs` in Cargo's `OUT_DIR`.

[`ops/src/registry.rs`](../src/registry.rs) includes that fragment at compile
time. The resulting constants form the immutable legacy prefix of
`OperationRegistry`; registry code then classifies each raw entry against
Recipe-owned scalar, primitive, composition, workspace, and non-calculation
definitions. The build script does not inspect those definitions and does not
claim that every inventory row has a lowering.

## Intent and parseable contract

The current behavior is represented by this parseable map. The map describes
the implementation; it is not a second configuration file.

```yaml
recipe_ops_build:
  package:
    name: recipe-ops
    manifest: ops/Cargo.toml
    script: ops/build.rs
    cargo_key: build = "build.rs"
  working_directory:
    required: ops
    reason: "build.rs reads ../operation-surface.txt as a relative path"
  inputs:
    operation_surface:
      path_from_working_directory: ../operation-surface.txt
      read: std::fs::read_to_string
      encoding: UTF-8
      lines: std::str::lines
      line_number: one_based_physical_line_number
      right_trim: std::primitive::str::trim_end
      skip_when: "line.is_empty() || line.starts_with('#')"
      fields: exactly_two_tab_separated_fields
      field_0: symbol
      field_1: source
      field_validation: both_nonempty
      duplicate_accounting: BTreeMap<String, u16>
  generated:
    directory: env("OUT_DIR")
    file: operation_surface.rs
    constants:
      - RAW_OPERATION_SURFACE: "&[RawSurfaceEntry]"
      - RAW_OPERATION_COUNT: usize
    source_order: preserved
    ordinal: zero_based_parsed_entry_index
    occurrence: one_based_symbol_occurrence_in_source_order
    occurrences: total_symbol_occurrences
  cargo:
    rerun_if_changed:
      - ../operation-surface.txt
    rerun_if_env_changed: []
    rustc_env: []
    link_flags: []
```

## Package boundary and execution

`recipe-ops` is an edition-2024 library with the package-level
`build = "build.rs"` declaration. Its normal dependencies are `recipe-core`,
`recipe-language`, `recipe-math`, and `recipe-primitives`. The build script
uses only the Rust standard library (`BTreeMap`, `env`, `fs`, and `PathBuf`), so
it has no separate build-dependency graph. Cargo metadata exposes the script as
a `custom-build` binary target (`build-script-build`) in addition to the
`recipe_ops` library target. The package lint settings (`unsafe_code =
"forbid"` and `clippy::all = "deny"`) are applied while compiling that build
target as well as the library.

Cargo runs the compiled build script with the package directory (`ops`) as its
working directory. The literal `../operation-surface.txt` therefore resolves
to the repository-root inventory. This is a Cargo build boundary, not a
supported standalone command: invoking the script from another directory can
make the input path fail even when the repository file exists.

The script performs no runtime work. It does not read hardware, load a driver,
compile native code, allocate a tensor, execute an operation, or write a user
artifact. The generated file is an internal Cargo target artifact and is not a
public `.ogdl`, `.cubin`, or `.hsaco` export.

## Input collection and exact parser rules

`main` first emits the Cargo rerun directive, then reads the complete input:

```rust
println!("cargo:rerun-if-changed=../operation-surface.txt");
let source = fs::read_to_string("../operation-surface.txt")
    .expect("operation-surface.txt must be readable");
```

The parser then applies these rules to `source.lines().enumerate()`:

1. `line_number` is the one-based `str::lines` segment number (`index + 1`). A
   final newline does not produce an additional empty record because it is
   handled by `str::lines`.
2. `raw_line.trim_end()` removes all trailing Rust whitespace before any other
   decision. This removes a trailing `\r` from CRLF input, trailing spaces, and
   trailing tabs. Leading whitespace is retained.
3. A line is skipped only when the right-trimmed value is empty or begins with
   `#`. Comment detection does not call `trim_start`; a line beginning with
   spaces followed by `#` is not a comment and will continue through field
   validation.
4. The remaining line is split on the tab character with `split('\t')`. The
   first field is `symbol`; the second field is `source`.
5. A missing source field (no tab) panics with
   `operation-surface.txt:<line>: missing source field`.
6. A third field panics with
   `operation-surface.txt:<line>: expected exactly two tab-separated fields`.
   Since right trimming happens first, a trailing tab is removed and does not
   by itself create a third field.
7. Empty `symbol` or `source` panics with
   `operation-surface.txt:<line>: fields must be nonempty`.
8. Accepted values are copied into owned `String`s as
   `(line_number, symbol, source)`. The parser does not normalize case,
   whitespace inside a field, path spelling, or symbol spelling.

The first `BTreeMap<String, u16>` (`totals`) counts each accepted symbol. Both
the total and every later occurrence counter use checked `u16` addition. The
second `BTreeMap<String, u16>` (`seen`) assigns one-based occurrence numbers in
the original accepted-entry order. The maps are used for lookup only; no map
iteration changes output order.

`read_to_string` requires valid UTF-8. A non-UTF-8 inventory, a missing file,
or an input read error reaches the `expect` and fails the build. The current
inventory is ASCII and has no blank lines. Its measured snapshot is:

| Inventory observation | Current value | Meaning |
| --- | ---: | --- |
| Physical lines | 431 | Includes comments and accepted rows. |
| Comment lines | 10 | Lines beginning with `#`. |
| Accepted rows | 421 | The generated `RAW_OPERATION_COUNT`. |
| Distinct symbols | 415 | Count after parsing accepted rows. |
| `predict` rows | 3 | Three source-qualified entries. |
| `train` rows | 3 | Three source-qualified entries. |
| `predict_proba` rows | 2 | Two source-qualified entries. |
| `train_multiclass` rows | 2 | Two source-qualified entries. |

These counts describe the checkout observed while documenting the script, not
fixed limits. Any accepted inventory edit can change them.

The current header comments identify this file as the C13 legacy public
operation baseline. They state that a symbol remains until its behavior is
mapped to an owned f32/int32 scalar program or kernel template, while
infrastructure-only allocation, probing, and teardown entries remain tracked
by the migration map. Those comments are documentation to humans only; the
parser skips them and does not preserve their text in the generated source.

## Generated Rust source

After parsing, `main` constructs one `String` in this exact shape:

```rust
pub(crate) const RAW_OPERATION_SURFACE: &[RawSurfaceEntry] = &[
    RawSurfaceEntry {
        ordinal: <zero_based_index>,
        line: <one_based_input_line>,
        symbol: <Rust_debug_string_literal>,
        source: <Rust_debug_string_literal>,
        occurrence: <one_based_symbol_occurrence>,
        occurrences: <total_symbol_count>,
    },
    // one compact line with the same fields is emitted for every accepted row
];
pub(crate) const RAW_OPERATION_COUNT: usize = <accepted_row_count>;
```

The actual emitter uses a compact one-line `RawSurfaceEntry` and Rust's
`Debug` formatting (`{symbol:?}` and `{source:?}`) to produce valid escaped
string literals. It emits entries in `parsed.iter()` order, which is the order
of accepted rows in `operation-surface.txt`; it does not sort by symbol or
source. `ordinal` is contiguous from zero. `line` retains the original input
line number, including skipped comment lines before it.

The output path is:

```text
<Cargo OUT_DIR>/operation_surface.rs
```

For a normal target directory this has the form
`target/<profile>/build/recipe-ops-<Cargo fingerprint>/out/operation_surface.rs`.
The fingerprint and profile directory are Cargo-owned and can differ between
debug, release, feature, target, and toolchain builds. The file is not checked
into the repository and must not be edited by hand. Cargo may retain several
profile or fingerprint copies at once.

The current debug artifact observed in `target/debug/build/recipe-ops-*/out`
contains 424 lines and 62,377 bytes: one header, 421 entry lines, the closing
delimiter, and the count constant. All observed copies have the same content
for this inventory. These measurements are diagnostic snapshots, not API
contracts.

Cargo's generated `recipe_ops-*.d` dependency files list both
`ops/src/registry.rs` and the concrete `OUT_DIR/operation_surface.rs` path, and
record `OUT_DIR` as an environment dependency. This is the compiler-side edge
that makes the generated source part of the `recipe-ops` library build; it is
separate from the build-script rerun directive and does not make the generated
file a checked-in source file.

## Cargo rerun policy

The only directive emitted by this build script is:

```text
cargo:rerun-if-changed=../operation-surface.txt
```

Cargo records that path in the package build-script fingerprint. Editing,
replacing, or removing the existing inventory therefore schedules another
script run. The script reads the new bytes and rewrites the current
`OUT_DIR/operation_surface.rs` before `registry.rs` is compiled.

The script emits no `cargo:rerun-if-env-changed` directives and no
`cargo:rustc-env` or link directives. `OUT_DIR` is consumed as a Cargo-provided
output location, not as a semantic input. The script has no configurable
environment behavior beyond selecting that output location.

Cargo also applies its normal package build-script triggers, including changes
to `build.rs` and package manifest metadata. Those implicit Cargo triggers are
distinct from the explicit path printed by `main`.

Because an explicit `rerun-if-changed` directive is present, this script does
not ask Cargo to watch `ops/src`, other workspace sources, documentation,
examples, or directories below the package. A change to `ops/src/registry.rs`
or another Rust source still recompiles the library through Cargo's normal
crate dependency tracking, but it does not require regenerating the inventory.
Conversely, adding a new file under an unlisted directory cannot add an
inventory row because the script never scans directories. The input itself is
one explicitly watched file, not a directory glob.

The directive is printed before input parsing. If the input is then missing or
invalid, Cargo retains the rerun metadata but the build still fails at the
script's panic; a rerun directive never converts a build failure into success.

## Registry integration and source consumers

### Compile-time include and descriptor construction

`registry.rs` declares the private `RawSurfaceEntry` shape, then includes the
generated source:

```rust
include!(concat!(env!("OUT_DIR"), "/operation_surface.rs"));
```

The generated constants therefore compile in the `recipe_ops::registry` module
and have access to that module's `RawSurfaceEntry` type. No other crate reads
the generated file directly.

`OperationRegistry` keeps the generated rows as a prefix and appends two
manually declared Recipe-owned entries (`recipe_max_pool_1d` and
`recipe_max_pool_1d_backward`). The generated rows have a positive source line;
the two owned entries use `line: 0`, which is how `OperationId::is_recipe_owned`
distinguishes them. Registry cardinality is therefore:

```text
registry.len() = RAW_OPERATION_COUNT + RECIPE_OWNED_OPERATIONS.len()
             = 421 + 2 for the current inventory
             = 423 for the current checkout
```

For each raw entry, `describe` computes the remaining descriptor fields in a
fixed order:

```text
raw symbol/source
  -> scalar recipe lookup
  -> primitive recipe lookup
  -> workspace formula lookup
  -> non-calculation entry lookup
  -> structured composition lookup
  -> legacy dtype exclusion / dynamic conversion / host behavior / pending
  -> dtype, family, alias, determinism, and legacy-dtype contracts
  -> OperationDescriptor
```

The build script supplies only `ordinal`, source line, symbol, source, and
duplicate occurrence metadata. All lowering and contract decisions are owned
by the normal `recipe-ops` source modules and are evaluated when descriptors
are constructed. The generated `source` value is semantically relevant after
the build boundary: `NonCalculationRecipe::for_entry` and
`CompositionRecipe::for_entry` receive both symbol and source, so duplicate
symbols can map to different source-specific recipes. Scalar, primitive, and
workspace lookup starts from the symbol, while exact registry resolution
always retains the source string.

### Registry API consumers

| Consumer | Boundary | Use of generated registry data |
| --- | --- | --- |
| `ops/src/registry.rs` | Internal canonical registry | `iter`, `surface_iter`, `owned_iter`, `named`, `resolve_unique`, and `resolve_exact` traverse or resolve the generated prefix. |
| `ops/src/materialize.rs` | Structured operation preparation | `remaining_composition_manifest` iterates all descriptors; materialization and errors carry the resulting `OperationId`. |
| `ops/src/lib.rs` | Crate export surface | Reexports `OperationDescriptor`, `OperationId`, `OperationRegistry`, `operation_registry`, and descriptor/lowering APIs. |
| Root `Cargo.toml` and `training/Cargo.toml` | Cargo dependency boundary | The root `recipe` package and `recipe-training` package depend on `recipe-ops`; both receive the compiled registry, never the build script executable. |
| `src/facade.rs` | Public `recipe::operations` facade | Exposes `registry`, `all`, `resolve`, and `resolve_exact`, then forwards lowering, composition, and workspace calls to `recipe-ops`. |
| `training/src/compile.rs` | Training graph compiler | Resolves owned scalar symbols before lowering and resolves source-qualified composition symbols before materialization. |
| `training/src/inference.rs` | Inference graph compiler | Uses the same unique scalar and composition resolution paths for inference graphs. |
| `training/src/error.rs` and `training/src/inference.rs` | Error propagation | Converts `OperationError` into the training or inference compile error without replacing its failure with a fallback. |
| `ops/src/materialize/*.rs` | Family-specific materializers | Consume descriptors and IDs passed by the registry/materialization dispatcher; they do not include the generated file themselves. |

An exhaustive source search found one generated-file include (`registry.rs`),
one registry iterator use in composition-manifest construction
(`materialize.rs`), the root facade wrappers, and the four direct training
compiler calls listed above in each compiler (two scalar resolutions and two
composition resolutions, eight calls total). No other crate or module reads
`RAW_OPERATION_SURFACE` or `RAW_OPERATION_COUNT` directly.

The direct operation path is therefore:

```text
operation-surface.txt
  -> recipe-ops/build.rs
  -> OUT_DIR/operation_surface.rs
  -> ops/src/registry.rs include!
  -> OperationRegistry / OperationDescriptor
  -> root operations facade or training compile/inference
  -> owned lowering or fail-closed OperationError
```

Compilation of `recipe-ops` is enough to type-check the generated source. A
runtime caller must still resolve and lower a descriptor before it has a
calculation graph; presence in `operation-surface.txt` alone is not proof that
an operation is implemented. `training/src/forward.rs` is intentionally not a
raw inventory consumer: its canonical activation helpers use explicit owned
scalar programs and only pass operation symbols through compiler paths that
resolve them against this registry.

## Invariants

The following properties are required by the current implementation:

1. Every accepted non-comment inventory row produces exactly one raw entry.
2. Accepted-entry order is preserved. Reordering rows changes ordinals and can
   change operation identities even when the set of symbols is unchanged.
3. `RAW_OPERATION_COUNT` equals the number of generated entries.
4. Every generated ordinal is contiguous, zero-based, and unique within the
   generated prefix.
5. `line` identifies the original one-based file line, not the compacted row
   index.
6. For a symbol with `N` accepted rows, its entries carry occurrences `1..=N`
   in source order and all carry `occurrences: N`.
7. Duplicate symbols remain distinct source-qualified descriptors. The build
   script does not deduplicate them or reject duplicate `(symbol, source)`
   pairs.
8. `resolve_unique` must reject a duplicate symbol with
   `OperationErrorKind::AmbiguousSymbol`; callers needing a legacy source must
   use `resolve_exact`.
9. The generated prefix remains before the two Recipe-owned registry entries;
   the owned entries are not part of `RAW_OPERATION_SURFACE`.
10. The output is deterministic for the same input bytes and build-script
    source: map lookup affects counts, while generation uses parsed source
    order and no filesystem enumeration.
11. The generated source is internal build output. Public artifact contracts do
    not include it, and runtime execution must not depend on target output
    remaining after compilation.
12. Unclassified rows remain visible as `LoweringAvailability::Unsupported`
    and fail closed at the lowering boundary. The build script must not invent a
    fallback operation implementation.

## Failure behavior

| Failure or edge case | Observed behavior | Boundary |
| --- | --- | --- |
| `build.rs` fails to compile or violates package lints | Cargo cannot run `main`; the current invocation produces no successful generated result and the package build stops. | Custom-build target compilation. |
| `operation-surface.txt` missing, unreadable, or non-UTF-8 | `read_to_string(...).expect(...)` panics with `operation-surface.txt must be readable`. | Build script aborts before library compilation. |
| No tab on an otherwise non-comment line | `unwrap_or_else` panics with the line-specific missing-source message. | Input grammar failure. |
| More than two tab-separated fields after right trim | `assert!` panics with the line-specific exactly-two-fields message. | Input grammar failure. |
| Empty symbol or source | `assert!` panics with the line-specific nonempty-fields message. | Input grammar failure. |
| Leading whitespace before `#` | Not treated as a comment; it is parsed as a field and normally fails for missing source. | Exact parser rule, not recovery. |
| More than 65,535 occurrences of one symbol | Checked `u16` increment panics while building totals or occurrences. | Inventory cardinality limit. |
| `OUT_DIR` absent | `env::var_os("OUT_DIR").expect("Cargo provides OUT_DIR")` panics. | Unsupported non-Cargo invocation or broken Cargo environment. |
| Output directory or file is not writable | `fs::write(...).expect("generated operation inventory must be writable")` panics. | Build output failure. |
| `RawSurfaceEntry` or generated field types change without matching output | `registry.rs` fails to compile at the `include!` site. | Generated-source/schema mismatch. |
| A symbol is absent at runtime | `resolve_unique` or `resolve_exact` returns `UnknownOperation`. | Registry API, after a successful build. |
| A symbol has multiple source-qualified rows | `resolve_unique` returns `AmbiguousSymbol`; `resolve_exact` selects one source if the pair is unique. | Registry API, after a successful build. |
| A row has no owned lowering or concrete materializer | Descriptor remains `Unsupported`, or the relevant lowering/materialization API returns its typed `OperationError`. | Runtime semantic boundary, not build-script success. |
| A new file is added below an unlisted directory | No build-script rerun is caused by that file; it cannot affect this generated inventory. | Explicit Cargo watch set. |

No retry, alternate input, compatibility shim, or generated fallback exists.
Each failure remains visible at the boundary that owns it.

## Validation and maintenance

The real package path was checked with:

```text
cargo check -p recipe-ops -vv
```

The check completed successfully. A fresh Cargo target run showed the build
script invocation, the single output line
`cargo:rerun-if-changed=../operation-surface.txt`, and the subsequent
`recipe-ops` rustc command with `OUT_DIR` pointing at the generated source.
Cargo's debug fingerprint records the same path as its explicit rerun input,
and the observed `target/debug/build/recipe-ops-*/out/operation_surface.rs`
contains `RAW_OPERATION_COUNT: usize = 421`. `git diff --check
-- ops/.docs/build.md` was also run after documentation changes.

When changing [`operation-surface.txt`](../../operation-surface.txt), preserve
the two-field tab-separated grammar, run `cargo check -p recipe-ops`, and
inspect the resulting `OUT_DIR` source if the inventory shape changed. Do not
check generated target files into Git. When changing `build.rs` or the raw
entry schema in `registry.rs`, rerun the same package check because the Cargo
script must regenerate and the include must type-check together.
