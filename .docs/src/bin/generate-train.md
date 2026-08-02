# `generate-train`

## Intent

```text
intent: generate one Recipe training source declaration from a seeded token sequence
entrypoint: src/bin/generate-train.rs::main
cargo_target: generate-train (auto-discovered from src/bin/generate-train.rs)
default_output: ${CARGO_MANIFEST_DIR}/train.rs
source_of_truth: API.ogdl for token presence, DATASETS for target-aware files
execution: source generation only; training is delegated to `recipe run FILE.rs`
```

`generate-train` creates a complete Rust `TrainingResult<()>` program for the
currently supported dense tabular training path. It chooses a checked-in data
source, data preprocessing, a dense or perceptron topology, a loss, an AdamW
training policy, and observability declarations. A fixed `--seed` reproduces
the choices for the same `API.ogdl` and available dataset catalog. The binary
does not parse or execute the generated program, probe hardware, prepare data,
or write model artifacts.

The generated program is intentionally narrower than the full `API.ogdl`
surface. The specification remains broad, while this generator emits only the
forms that its hard-coded token sets and the current dense training compiler
can reach.

## Source structure

| Source range | Responsibility |
| --- | --- |
| `src/bin/generate-train.rs:11-29` | Compile-time manifest/API paths and all candidate value sets. |
| `src/bin/generate-train.rs:31-112` | `Task`, `Dataset`, and the eleven target-aware dataset records. |
| `src/bin/generate-train.rs:114-129` | CLI usage text and parsed `Options`. |
| `src/bin/generate-train.rs:131-175` | `GeneratedSource` and the exact-line `ApiSurface` adapter. |
| `src/bin/generate-train.rs:177-214` | Deterministic `Random` generator and bounded selection helpers. |
| `src/bin/generate-train.rs:216-250` | Process entrypoint, orchestration, and output dispatch. |
| `src/bin/generate-train.rs:252-312` | Option parsing, seed parsing, and fallback seed construction. |
| `src/bin/generate-train.rs:314-320` | Existence filter for the static dataset catalog. |
| `src/bin/generate-train.rs:322-441` | API checks, token-by-token data/model/train generation, and source assembly. |
| `src/bin/generate-train.rs:443-454` | Task-aware metric selection without replacement. |
| `src/bin/generate-train.rs:456-474` | One-line or wrapped chain emission at 60 characters. |

The call graph is deliberately small:

```text
main
`-- run
    |-- parse_options
    |   |-- parse_seed (when an explicit seed is supplied)
    |   `-- random_seed (when no seed is supplied)
    |-- fs::read_to_string(API_PATH)
    |-- ApiSurface::parse
    |-- available_datasets
    |-- generate_source
    |   |-- ApiSurface::require / ApiSurface::choose
    |   |-- Random::new / next / index / choose / bool
    |   |-- choose_metrics
    |   `-- emit_chain (three times)
    `-- fs::write or print
```

`main` maps `Ok(())` to `ExitCode::SUCCESS`. Any `Err(String)` is printed as
`generate-train: ...` on stderr and maps to `ExitCode::FAILURE`.

## Inputs

### Command line

The usage contract embedded in `USAGE` is:

```text
cargo run --bin generate-train -- [OPTIONS]
--seed SEED       reproduce the exact token sequence
--output PATH     write somewhere other than the workspace train.rs
--stdout          print generated source without writing a file
-h, --help        show this help
```

`parse_options` accepts both separated and equals forms for `--seed` and
`--output`:

```text
--seed 42
--seed=42
--output generated.rs
--output=generated.rs
```

`SEED` is parsed as a decimal `u64`. When omitted, `random_seed` XORs the
current nanoseconds since the Unix epoch with the process ID rotated left by
23 bits. This is a convenient changing seed, not a cryptographic random
source.

The initial output is the workspace-root `train.rs`, derived from the
compile-time `MANIFEST_DIRECTORY`. `--stdout` changes output to stdout and
sets the destination to `None`. `--output` and `--stdout` conflict when an
explicit non-default output has already been selected. Repeated `--seed` or
`--output` options are accepted with the last value winning. `-h` and `--help`
short-circuit option parsing, print `USAGE`, and succeed even if other options
follow them.

No parent directories are created. File output uses `fs::write`, so an existing
path can be truncated and replaced. The write is not staged or atomic.

### API grammar input

`API_PATH` is the compile-time path
`${CARGO_MANIFEST_DIR}/API.ogdl`, not a path resolved from the caller's current
directory. `ApiSurface::parse` is a presence adapter, not an OGDL parser:

1. Split the file with `str::lines()`.
2. Trim each line.
3. Keep lines whose trimmed text starts with `.`.
4. Copy those complete lines into a `BTreeSet<String>`.
5. Reject the file only when the resulting set is empty.

Indentation, nesting, the `recipe` root, argument names, duplicate entries,
comments, and token semantics are not validated. A declaration must therefore
match the candidate string exactly, including spelling and punctuation. The
ordered layout in `API.ogdl` is not used after parsing.

`ApiSurface::require` checks one exact declaration and reports:
`API.ogdl does not declare required token ...` when absent. `ApiSurface::choose`
filters a caller-provided candidate array by exact membership, reports
`API.ogdl contains none of the generator's valid next tokens` when the filtered
array is empty, and otherwise delegates selection to `Random`.

The current `API.ogdl` declarations relevant to this binary are:

| Generator phase | Required or selected declaration | Emitted Rust form |
| --- | --- | --- |
| Data source | `.data(sources)` | `.data("examples/datasets/.../file")` |
| Target | `.target(targets)` | `.target("column")` |
| Optional exclusion | `.exclude(exclusions)` | `.exclude("column")` at most once |
| Data normalization | `.norm(z_score)`, `.norm(min_max)`, `.norm(l2_norm)` | `.norm(z_score|min_max|l2_norm)` |
| Split | `.split(train_fraction)` | `.split(0.7|0.75|0.8|0.85|0.9)` |
| Model root | `.model()` | `.model()` |
| Hidden blocks | `.layer(neurons)`, `.perc(count)` | `.layer(width)` or `.perc(width)` |
| Layer normalization | `.norm(layer_norm)`, `.norm(batch_norm)` | `.norm(layer_norm|batch_norm)` |
| Activations | `.relu()`, `.gelu()`, `.silu()`, `.cos()`, `.exp()`, `.log()`, `.ln()`, `.huber()`, `.tan()` | Same token |
| Binary objective | `.loss(bce)` | `.loss(bce)` |
| Regression objective | `.loss(mse)`, `.loss(mae)`, `.loss(huber)` | One selected loss |
| Optional clipping | `.grad(clip: maximum_norm)` | `.grad(clip(value))` |
| Train root | `.train()` | `.train()` |
| Optimizer | `.optimizer(adamw)` | `.optimizer(adamw)` |
| Epoch bound | `.epochs(count)` | `.epochs(1..=20)` |
| Learning rate | `.lr(rate)` | One of `0.0001`, `0.0002`, `0.0003`, `0.001`, `0.003` |
| Decay schedule | `.cos()`, `.exp()` under `.lr(rate)` | `.cos()` or `.exp()` |
| Optional warmup | `.warmup(count)` | `.warmup(1..epochs-1)` |
| Log form | `.log([items])` or `.log([items]).every(interval)` | `.log([...])`, optionally followed by `.every(interval)` |
| Optional plot | `.plot([items])` | `.plot([...])` |
| Terminal execution | `.run()` | `.run()?` |

The placeholder `.grad(clip: maximum_norm)` only gates availability. The
generator emits the public Rust helper form `.grad(clip(value))`; it does not
emit the named placeholder syntax. Likewise, the combined logging declaration
only gates whether a separate `.every(...)` call may be emitted.

An emitted `.exclude("column")` is passed to the facade as one column-pattern
exclusion. The preparation layer resolves patterns case-insensitively and
rejects an unmatched pattern or a pattern that also matches a declared target;
the generator only supplies catalogued exact names and does not preflight those
checks.

The generator never emits `.set(...)`, multiple data sources, row predicates,
checkpoint `.load(...)`, the other model block families, `.resume(...)`,
`.save(...)`, inference declarations, or `.run(model, data)`.

### Dataset catalog input

`DATASETS` is a compile-time, ordered array. `available_datasets` retains an
entry only when `MANIFEST_DIRECTORY.join(dataset.path).is_file()` is true. It
does not read headers, validate delimiters, inspect row counts, or check target
types. The selected `Dataset` is copied into `GeneratedSource` so `run` can
report its path after a file write. `Path::is_file()` follows a symlink; the
later ingest boundary may still reject a symlinked source.

| Path | Target | Optional exclusion candidates | Task |
| --- | --- | --- | --- |
| `examples/datasets/house-prices/train.csv` | `SalePrice` | `Id` | Regression |
| `examples/datasets/no-show-appointments/KaggleV2-May-2016.csv` | `No-show` | `AppointmentID`, `PatientId` | Binary |
| `examples/datasets/uci-sonar/sonar.all-data` | `col61` | none | Binary |
| `examples/datasets/uci-ionosphere/ionosphere.data` | `col35` | none | Binary |
| `examples/datasets/uci-wdbc/wdbc.data` | `col2` | `col1` | Binary |
| `examples/datasets/uci-spambase/spambase.data` | `col58` | none | Binary |
| `examples/datasets/uci-magic/magic04.data` | `col11` | none | Binary |
| `examples/datasets/uci-bank-semicolon/bank.csv` | `y` | none | Binary |
| `examples/datasets/uci-airfoil/airfoil_self_noise.dat` | `col6` | none | Regression |
| `examples/datasets/uci-abalone/abalone.data` | `col9` | none | Regression |
| `examples/datasets/uci-winequality-semicolon/winequality-red.csv` | `quality` | none | Regression |

All eleven paths were regular files in the checked-out workspace during this
source trace. The catalog's `colN` names rely on the ingest contract: files
with `.csv` use a present header, while `.all-data`, `.dat`, and `.data` use an
absent header and receive `col1`, `col2`, and so on. Delimiters are inferred by
the ingest layer. The generator itself does not enforce this relationship.

## Deterministic generation

`Random` stores one `u64` state. `next` adds
`0x9e37_79b9_7f4a_7c15`, then applies the two wrapping multiplications and
bit-mixing steps used by the source. `index(length)` rejects an empty set with
an assertion and uses rejection sampling before taking `value % length`.
`choose` indexes a slice; `bool` uses the low bit of `next`. Every choice is
therefore deterministic for a fixed seed and fixed candidate/catalog inputs.

`generate_source` consumes the random stream in this order:

1. Require the always-needed API declarations: `.data(sources)`,
   `.target(targets)`, `.split(train_fraction)`, `.model()`,
   `.layer(neurons)`, `.train()`, `.optimizer(adamw)`, `.epochs(count)`,
   `.lr(rate)`, and `.run()`.
2. Reject an empty available-dataset list.
3. Choose one dataset from the filtered catalog.
4. Build the data chain with the dataset path and target. If the dataset has
   exclusion candidates, draw a boolean and, when true, choose exactly one
   exclusion. Choose one available data normalization and one split fraction.
5. Choose `hidden_blocks = index(4) + 1`, so there are one through four hidden
   blocks. For each block, choose `.layer` or `.perc`, choose its width, and
   choose one activation. A boolean may add one layer normalization. A second
   boolean chooses whether normalization precedes or follows the activation.
6. Append `.layer(1)`. Binary datasets require `.loss(bce)`. Regression
   datasets choose one of `.loss(mse)`, `.loss(mae)`, and `.loss(huber)`.
   Another boolean may add gradient clipping with one of `0.25`, `0.5`, `1.0`,
   `2.0`, and `5.0`.
7. Choose `epochs = index(20) + 1`, one learning rate, and one schedule. When
   `epochs > 1`, a boolean may add `.warmup(index(epochs - 1) + 1)`, which is
   always strictly less than the epoch bound.
8. Call `choose_metrics`. Begin with `Loss`, `Epoch`, `Lr`, `Time`, and
   `Device`. Binary tasks additionally offer `AuRoc`, `AuPrc`, `Brier`, and
   `CalibrationError`. Choose one through four metrics, removing each selected
   item with `swap_remove`, so no metric repeats. Regression generation never
   selects `Accuracy` or `R2`.
9. Choose between the two logging declarations. Both emit one `.log([...])`.
   The interval form additionally emits `.every(1|2|5|10)`. A final boolean
   may emit `.plot([...])` using the same metric list.
10. Append `.run()` and render the three chains.

The candidate filter runs before each random choice. Removing a declaration
from `API.ogdl` can therefore change both success and all subsequent random
draws, even when the seed is unchanged. Removing a dataset file changes the
catalog length and can also change the selected sequence.

The hard-coded values are:

```text
block_widths: 8, 12, 16, 24, 32, 48, 64, 96, 128, 256
layer_normalizations: layer_norm, batch_norm
activations: relu, gelu, silu, cos, exp, log, ln, huber, tan
regression_losses: mse, mae, huber
gradient_clips: 0.25, 0.5, 1.0, 2.0, 5.0
split_fractions: 0.7, 0.75, 0.8, 0.85, 0.9
learning_rates: 0.0001, 0.0002, 0.0003, 0.001, 0.003
schedules: cos, exp
log_intervals: 1, 2, 5, 10
```

## Generated output

The source string always has this shape:

```rust
#!/usr/bin/env -S recipe run
// Generated token by token from API.ogdl.
// Reproduce with: cargo run --bin generate-train -- --seed SEED

use recipe::*;

fn main() -> TrainingResult<()> {
    recipe.data("...") /* data chain */;

    recipe.model() /* model chain */;

    recipe.train() /* policy chain */
        .run()?;
    Ok(())
}
```

The actual indentation is tabs. `emit_chain` concatenates method strings and
keeps a chain on one line when its character count, including the leading tab
and terminator, is at most `CHAIN_WIDTH` (60). Longer chains put the root
method on a `recipe...` line and each remaining method on its own indented
line. Only the final method receives the supplied terminator. The data and
model chains use `;`; the train chain uses `?;` so `TrainingError` propagates
through `TrainingResult<()>`.

With `--output`, `run` writes the complete source and then reports
`generated PATH with seed SEED and dataset DATASET_PATH` on stderr. With
`--stdout`, only the source is printed. The source contains no `.save(...)`,
so a successful generated training run exports no user-owned model or native
kernel artifact. The shebang is source content only; `fs::write` does not make
a newly created output executable or set execute permissions.

## Caller and callee boundary

The generated file is consumed by the root CLI, not by this binary itself:

```text
recipe run FILE.rs
`-- src/main.rs::main
    `-- src/cli.rs::run
        `-- run_source
            |-- canonicalize, read, and classify FILE.rs
            |-- source_frontend::lower_recipe_source
            |-- one rustc compilation (`--edition 2024`, `-Dunused_must_use`, JSON diagnostics, `--extern recipe`)
            `-- execute compiled child in FILE.rs's parent directory
```

The generated one-argument `.run()` is not one of the source-front-end rewrite
forms. The pre-rustc pass can rewrite special two-argument save/resume/run
forms and named gradient syntax, but this generator emits ordinary public Rust
calls: `.grad(clip(value))` and `.run()`.

The source runner does not use a compiler diagnostic to choose a second source
shape or retry compilation. A generated source that reaches this boundary is
therefore checked once by the real Rust compiler before any training child is
started.

Inside the generated program, the public facade routes the declarations as:

```text
recipe.data(path)
`-- Recipe::data -> Data::set -> thread-local recipe data sequence
recipe.model()
`-- Recipe::model -> thread-local recipe model sequence
recipe.train().run()
`-- Train::run -> take_recipe_training_sequence
    |-- Data::validate
    |-- Model::validate
    |-- Train::validate
    `-- recipe_training::compile_training / native preparation / execution
```

`Recipe::data` resets the current model sequence, and each data/model builder
remembers its latest immutable declaration. `Train::run` consumes the
preceding data and model pair. The generated order is required: calling the
terminal run without both declarations produces a typed unsupported error.

The training boundary validates the generated subset before runtime work. The
current compiler accepts `.layer` and `.perc` dense blocks, the selected
activations and layer or batch normalizations, the selected built-in losses,
AdamW, an explicit learning-rate decay, positive clipping, and the declared
metrics when their task family matches. It then prepares the real dataset and
requires the normal measured native execution prerequisites. The generator
only constructs source; all of those later errors belong to `recipe run`.

## Failure behavior

| Boundary | Condition | Result |
| --- | --- | --- |
| Option parsing | `--seed` has no value | `--seed requires an unsigned integer`; exit failure. |
| Option parsing | Seed is not a `u64` | `invalid seed "..."`; exit failure. |
| Option parsing | `--output` has no value | `--output requires a path`; exit failure. |
| Option parsing | Explicit output and `--stdout` conflict | `--stdout conflicts with --output`; exit failure. |
| Option parsing | Any other token | `unknown option "..."` plus usage; exit failure. |
| API read | `API.ogdl` cannot be read | `read PATH: ...`; exit failure. |
| API parse | No trimmed line begins with `.` | `API.ogdl contains no method declarations`; exit failure. |
| Required API gate | Required declaration is absent | `API.ogdl does not declare required token ...`; exit failure. |
| Optional API choice | Every candidate in a choice set is absent | `API.ogdl contains none of the generator's valid next tokens`; exit failure. |
| Dataset availability | No catalogued path satisfies `Path::is_file()` | `none of the generator's tabular datasets exist`; exit failure. |
| Output | Parent is missing, path is invalid, or write fails | `write generated source PATH: ...`; exit failure. |
| Internal selection | A static selection slice is empty | `Random::index` assertion panic. Static arrays are nonempty in the current source. |
| Internal emission | A generated chain has no root method | `emit_chain` expectation panic. Current construction always supplies a root. |
| Generated program | Rust syntax, data, compiler, hardware, or training failure | Not observed by this binary. `recipe run` reports the later boundary's error and exits failure. |

`run` does not catch panics from its internal `expect`, `assert`, or
`unreachable!` invariants. It also does not create a fallback source when a
token or dataset check fails.

## Validation boundary and evidence

The binary's own validation ends after API presence checks, static-file
existence checks, deterministic source construction, and optional output I/O.
It does not call `syn`, `rustc`, `recipe run`, a data parser, a training
compiler, or a native backend after generating the string. `cargo check` and
`cargo test --bin generate-train` validate the generator target itself; they do
not prove that every emitted source executes.

Current checkout observations:

```text
cargo check --quiet --bin generate-train       succeeded
cargo test --quiet --bin generate-train        0 tests, succeeded
cargo run --quiet --bin generate-train -- --help
                                               printed USAGE, succeeded
cargo run --quiet --bin generate-train -- --seed 42 --stdout
                                               emitted the deterministic abalone regression source
cargo run --quiet --bin generate-train -- --seed nope --stdout
                                               rejected the seed before reading API.ogdl
cargo run --quiet --bin recipe -- run /tmp/generate-train-doc-check.rs
                                               compiled the generated source, then failed to resolve its relative dataset path
```

The full end-to-end boundary is a subsequent `recipe run FILE.rs` invocation
against the generated file. It requires the real dataset, the current source
runner and compiler, a measured profile, the offline native toolchain, and a
supported CUDA or HSA device. A successful generator invocation alone is not
runtime or hardware evidence.

## Limitations

1. **Subset, not a grammar-driven synthesizer.** `API.ogdl` is consulted only
   for exact declaration presence. The generator does not derive method order,
   argument types, valid receiver transitions, or values from the grammar.
   All such knowledge is hard-coded in the candidate arrays and generation
   order.
2. **Static dataset metadata.** `Path::is_file()` returning true is the only
   availability check. Targets, exclusions, task labels, delimiter assumptions,
   schema width, row count, and target dtype are not inspected. A changed file
   can pass generation and fail during preparation, or can no longer match its
   recorded task family.
3. **Relative source paths.** Dataset strings are relative to the workspace
   root. `recipe run` executes a compiled source with the generated file's
   parent as its current directory. A file written outside the root with
   `--output` therefore usually cannot resolve `examples/datasets/...` unless
   the caller supplies a matching directory layout.
4. **No artifact declarations.** Generated training never resumes, saves a
   semantic model, or saves a native kernel. Add those declarations manually
   after generation when that workflow is intended.
5. **No post-generation check.** The binary prints or writes source even though
   it has not type-checked it. Syntax and type failures are deliberately left
   visible at the `recipe run` compiler boundary.
6. **Seed stability is conditional.** The same seed is stable only while the
   API declaration set, candidate availability, static dataset order, and
   generator implementation remain unchanged. The default time/process seed
   is not reproducible unless the emitted seed is recorded.
7. **Output safety is caller-owned.** Default output is the workspace
   `train.rs`, and `fs::write` can overwrite it. Use `--stdout` or an explicit
   scratch path when preserving an existing program matters.
8. **No execution diagnostics.** There is no debug flag, run journal, or
   generator-side schema report. Runtime diagnostics come from the generated
   program and the `recipe run` boundary.

## Reproduction examples

Print a deterministic source without changing the workspace:

```bash
cargo run --bin generate-train -- --seed 42 --stdout
```

Write the default `train.rs` and receive a stderr summary:

```bash
cargo run --bin generate-train -- --seed 42
```

Write another path, then execute it through the real source runner only when
its parent directory can resolve the relative dataset paths:

```bash
cargo run --bin generate-train -- --seed 42 --output train-generated.rs
cargo run --bin recipe -- run train-generated.rs
```

The seed-42 source observed in this checkout selected
`examples/datasets/uci-abalone/abalone.data`, target `col9`, `min_max`
normalization, an 0.85 split, one 16-unit GELU layer plus the output layer,
MAE loss, eight epochs, a 0.0002 learning rate, cosine decay, one warmup
epoch, and `Device`, `Epoch`, and `Loss` logging.
