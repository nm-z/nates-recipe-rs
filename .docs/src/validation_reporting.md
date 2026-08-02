# `src/validation_reporting.rs`

## Contract record

| field | value |
| --- | --- |
| module | `src/validation_reporting.rs` |
| entry point | `unavailable_validation_line(status: ValidationMetricStatus) -> Option<String>` |
| visibility | `pub(crate)` function in a private root module (`src/lib.rs:7`) |
| purpose | Turn an authoritative typed validation-unavailability status into one deterministic human-readable stdout row |
| authority | `recipe_training::ValidationMetricStatus`, produced by `training/src/compile.rs` and carried in `TrainingOutputs` |
| direct caller | `src/training.rs::report_unavailable_validation` |
| side effects in this module | none; formatting only |
| output transport | the caller writes the returned row to locked `stdout`, then flushes it |

The module is deliberately a reporting adapter, not a validator. It does not
inspect a dataset, infer a task family, count target values, or decide whether a
metric is mathematically defined. Those decisions are made while compiling the
dense training graph. The helper preserves that decision in a stable textual
row; the `ValidationMetricStatus` value remains the structured source of truth.

## Exact function behavior

The function first pattern-matches the input. `NotRequested` and `Available`
return `None`. Only `Unavailable { family, reason, split_rows }` returns
`Some(String)`.

For an unavailable value, the current source maps enum variants as follows:

| typed value | text inserted in the row |
| --- | --- |
| `ValidationMetricFamily::Binary` | `binary` |
| `ValidationMetricFamily::Multiclass` | `multiclass` |
| `ValidationMetricFamily::Regression` | `regression` |
| `ValidationUnavailableReason::NoKnownTargets` | reason `no-known-targets`, `known_rows = 0` |
| `ValidationUnavailableReason::SingleKnownClass { known_rows }` | reason `single-known-class`, `known_rows = known_rows.get()` |

The complete returned string has this exact spelling and spacing:

```text
validation unavailable  family {family}  reason {reason}  known_rows {known_rows}  split_rows {split_rows}
```

There are two literal ASCII spaces between each adjacent field segment. The
returned `String` does not contain a newline. `report_unavailable_validation`
passes it to `write_live_metric_row`; that function adds the newline with
`writeln!` and flushes the output stream.

### Parse grammar

The current row is parseable as the following restricted grammar. `uint` is a
base-10, unsigned `u64` rendering with no sign or separators.

```text
row      = "validation unavailable" "  "
           "family " family "  "
           "reason " reason "  "
           "known_rows " uint "  "
           "split_rows " uint ;
family   = "binary" | "multiclass" | "regression" ;
reason   = "no-known-targets" | "single-known-class" ;
uint     = digit , { digit } ;
digit    = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
```

This is a human-readable line protocol, not JSON, TOML, or a public parser
API. The two-space separators and field names are part of the observed output
contract. There is no version marker, configurable label, ANSI color, metric
name, or trailing whitespace in the string produced by this module.

Examples:

```text
ValidationMetricStatus::Unavailable {
    family: ValidationMetricFamily::Binary,
    reason: ValidationUnavailableReason::NoKnownTargets,
    split_rows: 2,
}
=> validation unavailable  family binary  reason no-known-targets  known_rows 0  split_rows 2

ValidationMetricStatus::Unavailable {
    family: ValidationMetricFamily::Binary,
    reason: ValidationUnavailableReason::SingleKnownClass { known_rows: 7 },
    split_rows: 9,
}
=> validation unavailable  family binary  reason single-known-class  known_rows 7  split_rows 9
```

The helper does not validate that a `reason` and `family` pairing was produced
by the compiler, or that `known_rows <= split_rows`. Such consistency is an
invariant of the authoritative producer, not of this formatting boundary.

## Typed status and authoritative producers

`ValidationMetricStatus` is declared in `training/src/model.rs` and re-exported
from both the `recipe_training` crate and the root `recipe` crate. It is
`Clone + Copy + Debug + PartialEq + Eq` and has this shape:

```text
NotRequested
Available {
    family: ValidationMetricFamily,
    known_rows: NonZeroU64,
}
Unavailable {
    family: ValidationMetricFamily,
    reason: ValidationUnavailableReason,
    split_rows: u64,
}
```

The public `TrainingOutputs` structure stores the status beside the optional
binary, multiclass, and regression validation output structures. The status is
computed once during dense graph compilation, before native preparation or
execution. It is copied, not recomputed, into a dense `TrainingReport`; callers
can inspect it through `TrainingReport::validation_status()` after a successful
run.

### Data and target preparation

The root training compiler calls `prepare_data` before it invokes the
`recipe_training` compiler. Public preparation requires at least one target and
an explicit `.split(...)`. The split is an exact rational partition of retained
rows: training rows occupy the first range, and the validation partition is the
remaining range in source order. An empty validation split is rejected during
validation-config checking, not represented as `NoKnownTargets`.

`LoweredDenseDataset::from_prepared` then:

1. lowers the train and validation target vectors or ordered target matrix;
2. records `validation_split_rows` from the original prepared validation
   partition before target filtering; and
3. applies `DensePartition::known_only()` to validation. Rows marked
   `TargetObservation::Missing` or `TargetObservation::Unseen` are removed
   from the validation features and targets. A validation partition with no
   remaining known rows becomes `None`; a nonempty known subset is compacted in
   its original order.

For ordered multi-target objectives, a row is known only when every declared
target in that row is known. One missing or unseen cell therefore removes the
whole row from validation, rather than creating per-column row populations.
Categorical dictionary codes at the reserved unseen route become
`TargetObservation::Unseen`; empty numeric or categorical cells become
`Missing`.

### Status construction in `training/src/compile.rs`

`validate_validation_config` is the authoritative status producer. It first
rejects mutually exclusive family requests, incompatible loss/task pairs, and
an empty prepared validation partition. It then calls
`validation_metric_status` for the selected family:

| producer condition | typed status |
| --- | --- |
| no validation rows remain known after `known_only()` | `Unavailable { reason: NoKnownTargets, split_rows }`; this formatter renders `known_rows` as `0` |
| one or more known validation rows and the family is multiclass or regression | `Available { family, known_rows }` |
| one or more known validation rows and every binary target column contains exact `0` and `1` | `Available { family: Binary, known_rows }` |
| one or more known validation rows but at least one binary target column lacks either exact class | `Unavailable { family: Binary, reason: SingleKnownClass { known_rows }, split_rows }` |

`NoKnownTargets` is selected when the lowered validation option is `None`, not
when the original split itself is empty. The compiler converts the original
split row count to `u64` and the known row count to `NonZeroU64`; conversion
failure is a `TrainingCompileError`, so no unavailable row is produced.

The binary special case checks each column in the lowered row-major target
matrix. Integer targets are checked for exact `0` and `1`; binary32 targets are
decoded from their bits and checked for exact `0.0` and `1.0`. Multi-target
binary validation requires both classes in every target column. The status keeps
only the total known row count, not the failing column or per-class counts.

This check protects the complete binary metric bundle. The bundle includes
class-dependent ranking and recall operations (in addition to mean BCE,
accuracy, Brier score, and calibration error), so a single-class population is
not compiled as a partially populated metric bundle. The system contract states
the same rule for ordered multi-target binary metrics: both classes must occur
in every target column's known validation rows.

Multiclass and regression status construction checks only for a nonzero known
row population. It does not perform a class-diversity check for multiclass or a
target-variance check for R2. Those numerical conditions are outside this
reporting helper's contract.

## What an unavailable status changes in the compiled graph

After status construction, `compile_dense_training_impl` derives three gates:

```text
validation_available = status is Available
retain_validation_inputs = status is not Unavailable
calibration_iterations = configured binary calibration iterations only when Available
```

For either unavailable reason, validation features and targets are not retained
as external inputs. The corresponding binary, multiclass, or regression
validation output is `None`; the `(Some(config), None)` branch is explicitly
accepted when the status is `Unavailable`, rather than converted to a compile
error. Consequently `training_metric_bindings` contains no validation metric
bindings for that run. Training loss, optimizer state, and the ordinary dense
training loop still compile and execute. A requested binary temperature-scaling
phase is also omitted (`calibration_iterations = 0`).

This is why the row says `validation unavailable` while the training run can
continue: it reports the absence of the optional validation metric population,
not an invalid training objective or a native execution failure.

## Reporting boundary and call graph

The only current call site is the dense training execution path:

```text
Train::run()
  or source-frontend Train::__recipe_run_with(...)
    -> Train::try_run_with(...)
      -> compile_training_package
        -> compile_training_graph
          -> recipe_training::compile_dense_training_impl
            -> TrainingOutputs.validation_status
      -> execute_current_training
        -> report_unavailable_validation
          -> unavailable_validation_line
          -> write_live_metric_row(stdout)
        -> native execution and optional live metric presenter
      -> TrainingReport::dense(..., validation_status)
```

`report_unavailable_validation` runs synchronously at the first line of
`execute_current_training`, before native preparation, before the native
executor is entered, and before the live metric presenter thread is spawned.
When the helper returns `None`, the caller performs no write and proceeds
normally. When it returns `Some(line)`, the caller locks process stdout and
calls `write_live_metric_row`:

```text
writeln!(stdout_lock, line)?;
stdout_lock.flush()
```

The line is therefore emitted once per dense run with an unavailable status,
not once per epoch. If live metric rows are enabled, this row is emitted before
the presenter can write training or plot rows. It is written to stdout, not to
the repository debug log and not to stderr.

When the `recipe run FILE.rs` CLI is used, the compiled child process's stdout
is read by a dedicated forwarder and written, flushed, and retained as a bounded
tail by `src/cli.rs`. The row therefore crosses the user-facing CLI boundary on
the stdout path without being parsed or rewritten. A nonzero child exit still
returns the CLI's run-failure result; the captured stdout tail may contain this
row even though the overall training command failed later.

The public `compile_training(...)` boundary and the lower-level
`recipe_training` compile entry points expose the typed status through
`CompiledTraining::outputs()` but do not call this formatter. A compile-only
caller therefore receives no stdout row. KNN and Bayesian `Train::run` branches
do not enter dense execution; their reports set `validation_status` to
`NotRequested`, so this helper is never called for them.

## Failure behavior

The formatter itself returns no recoverable error. It either returns `None` or
allocates the deterministic `String`. All semantic validation failures occur
upstream and are returned as typed declaration, data-preparation, or compile
errors before this boundary.

At the reporting boundary, a write or flush failure from stdout is mapped to:

```text
TrainingError::Runtime {
    stage: "report validation availability",
    detail: io_error.to_string(),
}
```

The `?` in `execute_current_training` propagates that error immediately. Native
preparation and execution, live metric presentation, static device reporting,
and artifact saving are not attempted after this failure. There is no retry,
alternate stream, or fallback text. Depending on the writer, a partial row may
already have been written before the error is returned.

If the row is written successfully, an unavailable status does not fail the
training run. Native execution proceeds without validation inputs and the
completed dense report still exposes the same typed unavailable status. The
status is in-memory compile/report metadata; no current checkpoint or exported
model/native-kernel artifact serializes it, so a later run recomputes it from
the current prepared data and task.

## Invariants

1. `None` means this helper did not report an unavailable status. It covers both
   `NotRequested` and `Available`; it does not distinguish those states in the
   text channel.
2. `Some` always starts with the exact prefix `validation unavailable` and has
   exactly four named fields in the fixed order `family`, `reason`,
   `known_rows`, `split_rows`.
3. Current family and reason matches are exhaustive. Adding a family or reason
   variant in `recipe_training` requires a source update before this module
   compiles. A new `ValidationMetricStatus` variant would instead fall through
   the let-else and silently produce `None` until the reporting contract is
   updated.
4. `known_rows` is decimal `0` only for `NoKnownTargets`; the compiler's
   `SingleKnownClass` payload is nonzero by type construction.
5. `split_rows` is the original prepared validation population, while
   `known_rows` is the post-filter known row population. They may differ.
6. The formatter does not mutate or consume authoritative training state in
   practice: `ValidationMetricStatus` is `Copy`, and the caller passes a copy
   from `TrainingOutputs` while retaining the same value for `TrainingReport`.
7. The function has no policy, metric-selection, dataset, or hardware inputs;
   it cannot make validation available, recover missing targets, or alter the
   compiled graph.

## Limitations and non-goals

- The line reports only availability. It does not contain metric values,
  metric names, target identities, the failing target column, class counts,
  missing versus unseen counts, or a remediation suggestion.
- `known_rows` is a row count, not a cell count. For a multi-target row it is
  counted only when all declared target columns are known.
- The helper accepts any `Unavailable` enum value that Rust code constructs,
  including pairings the current compiler never emits, and does not check row
  count relationships. Producer invariants provide those semantics.
- A future `ValidationMetricStatus` variant is treated like every current
  non-`Unavailable` variant by the let-else and yields `None`; adding such a
  status therefore requires a deliberate reporting-contract review rather than
  relying on an exhaustive-match compiler error.
- A validation request that has no prepared validation partition is a compile
  error. It is not rendered as `NoKnownTargets`; the row is reserved for a
  nonempty original split whose known subset is empty.
- A run that never requests validation metrics remains silent even if its
  validation rows are unknown. The status is `NotRequested`, and this helper
  intentionally does not report general data quality.
- The text row has no schema/version negotiation and no public parser function.
  Consumers that need forward-compatible structured data should use the typed
  `TrainingReport::validation_status()` or `CompiledTraining::outputs()` value.
- The formatter does not make numerical validation safe by itself. Multiclass
  and regression availability currently require only known rows; numerical
  metric-domain faults outside those status predicates are handled, if at all,
  by the compiled metric operations and execution layers.

## Source traceability

- Formatter implementation: `src/validation_reporting.rs:1-24`.
- Private module declaration and public status re-exports: `src/lib.rs:1-31`.
- Dense run boundary, status copy into `TrainingReport`, and caller:
  `src/training.rs:223-255`, `src/training.rs:848-967`.
- User-facing `recipe run` stdout forwarding and bounded failure-tail capture:
  `src/cli.rs:272-362`, `src/cli.rs:433-565`.
- Training policy metric-family selection:
  `src/training.rs:1826-1908`.
- Status type and `TrainingOutputs` field:
  `training/src/model.rs:2486-2527`.
- Validation partition lowering and known-row compaction:
  `training/src/model.rs:1250-1423`.
- Validation status producer and graph gating:
  `training/src/compile.rs:723-844`, `training/src/compile.rs:1261-1352`,
  `training/src/compile.rs:2276-2474`.
- Public data split boundary and exact partition construction:
  `src/data_prepare.rs:140-171`, `ingest/src/prepare.rs:14-133`,
  `ingest/src/prepare.rs:1073-1117`.
- Normative binary multi-target availability rule:
  `system-contract.md:586-603`.
