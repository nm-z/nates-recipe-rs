# `src/source_frontend.rs`

## Boundary and purpose

`source_frontend` is the crate-private, pre-rustc syntax adapter used by the
`recipe run FILE.rs` command. It adapts the small set of declaration forms in
`API.ogdl` that cannot be passed to the Rust facade as written before the source
is handed to `rustc`. Some forms are syntactically invalid Rust, such as the
named-gradient field; others are valid method-call syntax with an arity or
argument shape that the facade does not expose directly. It also translates
compiler diagnostics produced for a transformed temporary source back to the
user source.

The module does not parse `API.ogdl`, build a Recipe graph, read data, probe
hardware, compile a kernel, or execute a run. `API.ogdl` remains the normative
surface description. The Rust facade and this module are the current executable
implementation of selected forms from that description. The only code in the
repository that reads `API.ogdl` as text is the `generate-train` binary; this
module has no file or specification reader.

The complete source-runner path is:

```text
recipe run FILE.rs
  -> cli::run_source
  -> lower_recipe_source(original path, source text)
  -> optional SourceRewrite and sibling temporary .rs file
  -> one rustc invocation with --error-format=json
  -> DiagnosticStream::parse
  -> mapped_rendering or original_rendering
  -> execute the compiled binary
  -> the binary calls Recipe's facade, training, preparation, and executor code
```

`lower_recipe_source` is called once before rustc by `src/cli.rs:311`. A
compiler diagnostic never requests another parse, rewrite, or compilation.
This is the deliberate replacement for the historical E0061-driven retry
path.

## Source inventory

The module has four layers of state. The first two identify Recipe calls and
construct edits. The third retains a source-to-generated mapping. The last
parses and renders rustc's JSON diagnostic stream.

| Item | Shape | Responsibility |
| --- | --- | --- |
| `RecipeReceiver` | `Facade`, `Data`, `Model`, `Train`, `Infer` | Coarse receiver class used to decide whether a call is a Recipe declaration. |
| `MethodCallLocation` | method name, three byte locations, arity, optional receiver | Edit anchors collected from `syn::ExprMethodCall`. |
| `MethodCallVisitor` | `Visit` implementation over a `syn::File` | Records every method call and classifies its receiver. |
| `LocalBinding` and `LocalBindingVisitor` | local name, optional type class, optional initializer | Collects simple `let` bindings used by receiver classification. |
| `RewriteRange` | original and generated start/end offsets | Describes one replacement in a generated source. |
| `TextEdit` | original start/end and replacement text | Intermediate edit requested by the lowering pass. |
| `SourceRewrite` | original text, generated text, ranges, original line starts | Owns the generated source and maps generated compiler offsets back to original offsets. |
| `NamedGradShape` | exact field spans or an error string | Records whether a named gradient call is exactly `clip: EXPR`. |
| `NamedGradCandidate` | method and argument spans plus shape | Candidate found by the token-level `.grad(...)` scan. |
| `NamedGradField` and `NamedGradArguments` | `syn::Parse` implementations | Parse the restricted named-gradient field grammar. |
| `DiagnosticEntry` | `Json(Value)` or `Raw(String)` | Preserves diagnostic JSON while retaining non-diagnostic and malformed lines. |
| `DiagnosticStream` | ordered `Vec<DiagnosticEntry>` | Parses rustc stderr and renders it in original or mapped coordinates. |

The module imports `proc_macro2` for token scanning and byte spans,
`syn` for a complete Rust file AST and visitors, and `serde_json` for rustc's
JSON diagnostic protocol. `proc_macro2` is built with `span-locations`, which
is required for the byte ranges used by the edit and mapping code.

### Function-level call map

The following is the complete intra-module call direction. It is useful when
reading the file because no function below is registered as a callback in the
runtime or graph compiler.

```text
lower_recipe_source
  -> collect_named_grad_candidates
       -> classify_named_grad_arguments
  -> build_rewrite                 (classification probe)
  -> syn::parse_file
  -> collect_recipe_bindings
       -> LocalBindingVisitor::visit_file
       -> classify_recipe_expression
  -> MethodCallVisitor::visit_file
       -> MethodCallVisitor::visit_expr_method_call
            -> classify_recipe_expression
  -> SourceRewrite::generated_to_original
  -> render_named_grad_error       (only for selected malformed candidate)
  -> build_rewrite                 (final edits)

DiagnosticStream::parse
  -> serde_json::from_slice
DiagnosticStream::original_rendering
  -> render_diagnostic or raw append
DiagnosticStream::mapped_rendering
  -> mapped_rendering_chain
       -> remap_diagnostic
            -> remap_span
       -> render_diagnostic
            -> preferred_span
            -> render_span
```

`MethodCallVisitor` and `LocalBindingVisitor` both call their `syn::visit`
default visitor after recording the current node. Nested method calls and
initializer expressions are therefore visited by the normal AST traversal,
not by a separate source scan.

## Receiver classification

### `RecipeReceiver`

`RecipeReceiver` is intentionally not Rust type checking. It is a source-level
classification with five values:

```text
recipe root -> Facade
recipe.data(...) -> Data
recipe.model() -> Model
recipe.train() -> Train
recipe.infer() -> Infer
```

The type is private because it is an implementation detail of source lowering.
It is enough to distinguish the declarations whose syntax must be adapted.

### Local collection and fixed point

`LocalBindingVisitor` (`src/source_frontend.rs:62-82`) visits every local in the
parsed file. It records a binding only when the pattern is a simple identifier.
For a typed binding, `classify_recipe_type` (`:84-95`) looks only at the last
path segment and recognizes `Data`, `Model`, `Train`, and `Infer`. For an
untyped binding it retains the initializer expression for later inference.

`collect_recipe_bindings` (`:97-120`) repeatedly walks the collected locals
until no binding changes. An explicit recognized type wins over an initializer.
Otherwise the initializer is passed to `classify_recipe_expression` with the
bindings discovered so far. This lets declaration chains resolve in either
source order when a later fixed-point pass supplies an earlier name. The
resolved map is keyed by the text of the local name.

### Expression classification

`classify_recipe_expression` (`:122-152`) handles only these expression forms:

| Expression | Classification rule |
| --- | --- |
| `Expr::Path` | Use the last path segment as a binding name. An unbound segment named exactly `recipe` is `Facade`. |
| `Expr::MethodCall` | Classify the receiver first. A `Facade` changes to `Data`, `Model`, `Train`, or `Infer` only for `data`, `model`, `train`, or `infer`; every other method yields no class. Once a non-facade Recipe class is known, any chained method preserves that class. |
| `Expr::Group`, `Expr::Paren`, `Expr::Reference` | Recurse through the wrapper. |
| All other expressions | Return `None`. |

The last rule for a non-facade receiver is deliberately broad. For example,
`recipe.model().layer(8).loss(bce)` remains `Model`, and
`recipe.train().optimizer(adamw).save("model.ogdl")` remains `Train`. The
method name itself is checked later, so ordinary model methods do not become
source edits merely because they are on a Recipe chain.

The classifier recognizes a local alias initialized from `recipe`, and a
reference such as `(&model)`, but it does not perform lexical scope analysis.
Bindings with the same text in different scopes share one map entry. It also
does not resolve function returns, fields, indexing, macros, trait-based
conversions, type aliases, or arbitrary expressions. A call that cannot be
classified is left for rustc unchanged.

## Source rewrite representation

### `TextEdit` and `build_rewrite`

`build_rewrite` (`src/source_frontend.rs:624-667`) is the only constructor for
`SourceRewrite`. It sorts edits by `(start, end)`, computes a saturating capacity
hint, and copies original slices and replacement strings into a new generated
source. Each edit records a `RewriteRange` in generated coordinates. The
constructor rejects:

- an edit whose start precedes the end of the previous edit,
- an end before its start,
- an end beyond the source length, or
- a start or end that is not a UTF-8 character boundary.

The generated suffix is copied after the final edit. `line_starts` contains
the original offset zero and every byte after an original newline. It is not
recomputed for generated text because diagnostics are ultimately rendered from
the original text.

`SourceRewrite::generated` (`:177-180`) is the only accessor needed by the CLI
when it writes the temporary source. `original` and the mapping fields remain
private to the module.

### Generated-to-original mapping

`SourceRewrite::generated_to_original` (`:182-212`) walks the ordered rewrite
ranges. Before a range it subtracts the generated cursor and caps the result at
the original range start. Inside a range with equal original and replacement
lengths it maps proportionally. Inside a length-changing range, interior bytes
map to the original range start and the generated end maps to the original
range end. After the last range it applies the remaining cursor delta and caps
at the original source length.

This policy gives a stable source location for compiler spans that cover text
introduced by a lowering edit. It does not claim that every generated byte has
a unique original character. Multiple zero-width edits at one source offset
are accepted by `build_rewrite`; diagnostics touching such a boundary follow
the first matching range's boundary rule.

`line_column` (`:214-223`) clamps an original byte offset to the source length,
finds the one-based line from `line_starts`, and counts Unicode scalar values
for the one-based column. `line` (`:225-239`) returns an original line without
its newline, accepting both LF and CRLF input. Out-of-range line requests return
an empty string.

## Named gradient lowering

`API.ogdl:58` presents a named field form, `.grad(clip: maximum_norm)`, while
the Rust facade accepts `Model::grad(Grad)`. `recipe::clip(f64)` constructs the
`Grad` value consumed by that method (`src/api.rs:894-912, 1406-1420`). The
frontend converts the specification form to an ordinary Rust expression:

```text
.grad(clip: EXPR)
        -> .grad(::recipe::clip(EXPR))
```

The `::recipe` path relies on the CLI's `--extern recipe=...` alias. A source
that is transformed outside the Recipe source runner under a different crate
name would not have that path unless it provides the same alias.

### Candidate scan

`collect_named_grad_candidates` (`src/source_frontend.rs:519-559`) scans the
`proc_macro2::TokenStream` recursively. It looks for a three-token window:

```text
Punct('.')  Ident('grad')  Group(parenthesized arguments)
```

The first two tokens inside the argument group must be an identifier and an
alone `:` punctuation. This is only a lexical prefilter, not a claim that the
field is named `clip`. The candidate stores the method identifier span, the
inner argument span, and the result of the restricted parser.

The recursion visits every nested token group, so candidates in blocks,
closures, macro token groups, and nested calls are considered if they are
visible to `proc_macro2`. The final receiver check in `lower_recipe_source`
prevents a candidate from being rewritten unless the parsed call is a `Model`
call.

### Restricted parser and errors

`NamedGradField` parses `Ident : Expr`; `NamedGradArguments` parses a
comma-terminated `Punctuated` list. `classify_named_grad_arguments`
(`:561-596`) then enforces all of the named-field policy:

1. A parse failure becomes `NamedGradShape::Malformed` with the parser error and
   the expected `clip: EXPR` shape.
2. More than one `clip` field reports a duplicate-field error.
3. Any field other than `clip` reports an unknown-field error naming that field.
4. The list must contain exactly one field.
5. An exact field stores the byte spans for the identifier and colon.

When an exact candidate is selected, the final edit set replaces the field
identifier with `::recipe::clip`, replaces the colon with `(`, and inserts `)`
at the argument end. This keeps the expression text unchanged and lets the
normal `Model::grad` validation enforce finite, positive, representable f32
clipping values.

`render_named_grad_error` (`:598-622`) reports malformed recognized Recipe
syntax without invoking rustc. It uses the original source path, original line
and column, the complete candidate span on the first source line, and a caret
underline. The `expect` at `:457-459` relies on the invariant that a matching
named-gradient candidate always caused a classification rewrite.

A named `.grad(...)` on a receiver that is not classified as `Model` is not
rewritten and is not rejected by this module. Rustc then owns that call's
syntax and type errors.

## Deterministic lowering pass

`lower_recipe_source` (`src/source_frontend.rs:373-517`) performs one complete
pass. Its stages are:

1. Parse the complete input into a `proc_macro2::TokenStream`. A tokenization
   failure returns `Ok(None)`, leaving the original source for rustc.
2. Collect named-gradient candidates. If any exist, build a classification
   source by replacing each candidate's argument list with
   `::recipe::clip(1.0)`. This replacement is only a temporary probe that
   makes the otherwise non-Rust named-field syntax parseable.
3. Parse the classification source with `syn::parse_file`. A parse failure
   returns `Ok(None)` and does not trigger a retry.
4. Resolve local bindings and visit all method calls in the classification
   AST. Every recorded span is mapped back through the probe rewrite before it
   is compared with an original candidate or used for a final edit.
5. Select named-gradient candidates whose method span is exactly a visited
   `Model::grad` span. Exact candidates receive the three edits described
   above. A malformed selected candidate returns the formatted source error.
6. Scan all visited calls for the other deliberate syntax adaptations listed
   below.
7. Return `Ok(None)` when no final edits exist. Otherwise call `build_rewrite`
   on the original source. Invalid or overlapping final edits return a
   descriptive `Err`.

The classification probe is never passed to rustc. Only the final rewrite, if
one exists, is written to the temporary file. Compiler diagnostics are
therefore an output of this pass, never an input to it.

### API.ogdl forms and exact edits

The following table is the complete current edit set. A receiver and argument
count must match the row exactly; all other calls are left unchanged.

| Specification spelling | Recognized receiver and arity | Generated Rust spelling | Why the edit exists |
| --- | --- | --- | --- |
| `recipe.data()` | `Facade`, zero arguments | `recipe.data(())` | `Recipe::data` takes one `IntoDataSources` value. `()` is implemented as an empty source list so `.set(...)` can build the declaration afterward. |
| `.residual(layer(...), relu(), ...)` | `Model`, two or more arguments | `.residual([layer(...), relu(), ...])` | `Model::residual` takes one `IntoResidualBranch`; arrays implement that trait and preserve declaration order. A one-argument array or operation is already legal Rust and is not changed. |
| `.grad(clip: EXPR)` | `Model`, named candidate | `.grad(::recipe::clip(EXPR))` | Rust has no named method arguments, while the facade takes a `Grad`. |
| `.save("model.ogdl", "kernel.cubin")` or `.save("model.ogdl", "kernel.hsaco")` | `Train`, exactly two arguments | `.__recipe_save_pair("model.ogdl", "kernel...")` | The public one-path `Train::save` is retained; the hidden pair method preserves the literal two-path contract. |
| `.resume("model.ogdl", "kernel.cubin")` or `.resume("model.ogdl", "kernel.hsaco")` | `Train`, exactly two arguments | `.__recipe_resume_pair("model.ogdl", "kernel...")` | The public one-path `Train::resume` is retained; the hidden pair method enforces semantic model first and native kernel second. |
| `.run(model, data)` | `Train`, exactly two arguments | `.__recipe_run_with(model, data)` | `Train::run()` uses the preceding thread-local declarations; the hidden method allows the literal explicit pair in the current `API.ogdl`. |

The pair rows replace only the method identifier, so arguments and evaluation
order are preserved. The explicit run row is present in the current working
tree together with `Train::__recipe_run_with` (`src/training.rs:865-867`) and
`API.ogdl:89`.

No other API.ogdl method has a source-level rewrite. In particular, normal
one-path save and resume, `.run()`, `.data(sources)`, residual calls with one
argument, all model and data builder methods, and all inference methods are
sent to rustc as written.

## Diagnostic stream

### Parsing

`DiagnosticStream::parse` (`src/source_frontend.rs:301-316`) consumes rustc
stderr one newline-inclusive line at a time. A line is retained as JSON only
when it parses as a `serde_json::Value` and has `"$message_type":
"diagnostic"`. Every other line, including valid non-diagnostic JSON, invalid
JSON, and non-UTF-8 bytes, is retained as `DiagnosticEntry::Raw` using
`String::from_utf8_lossy`. Empty input produces an empty stream.

This preserves linker messages and other raw compiler output instead of
discarding lines that are not rustc diagnostic records.

### Original and mapped rendering

`original_rendering` (`:318-333`) preserves rustc's own `rendered` field when a
JSON diagnostic provides one. If that field is absent, it uses the module's
structured renderer. Raw lines are copied verbatim.

`mapped_rendering` (`:335-342`) is the one-rewrite convenience wrapper around
`mapped_rendering_chain` (`:344-370`). The chain applies each rewrite in order
to every JSON diagnostic and to every raw path string. JSON diagnostics are
cloned before mutation, so the stream remains reusable. Raw entries are not
parsed or otherwise reformatted.

### Mapping JSON spans

`remap_diagnostic` (`:669-687`) replaces compiler and original path strings in
the diagnostic message, remaps every span, and recursively visits child
diagnostics. `remap_span` (`:689-746`) only acts when the span file name equals
one of the configured compiler or original paths and both byte offsets are
unsigned integers that fit `usize`.

For a selected span it:

1. maps generated byte start and end to original byte offsets,
2. makes end at least start,
3. recomputes one-based line and Unicode scalar-value column positions,
4. replaces the file name and byte and line fields with original values, and
5. synthesizes a `text` array with source lines and caret highlight bounds.

Existing span fields are copied before these fields are overwritten. Spans for
other files, missing offsets, or offsets that cannot fit `usize` are left
untouched. The custom mapped renderer does not use rustc's original `rendered`
string, so mapped output is normalized through `render_diagnostic`.

### Structured rendering

`render_diagnostic` (`:748-777`) chooses a level defaulting to `error`, a
message defaulting to `compiler diagnostic`, and an optional nested diagnostic
code. It prints `level[code]: message` when a code exists, prints only the
message for `failure-note`, and otherwise prints `level: message`. It renders a
preferred primary span, falls back to the first span, recursively renders
children, and adds a blank line.

`preferred_span` (`:780-785`) implements the primary-then-first policy.
`render_span` (`:787-823`) prints the file and one-based line and column,
source lines when present, carets with a minimum width of one, and an optional
span label. Missing fields receive conservative defaults rather than causing
the diagnostic path to fail.

## CLI integration and temporary files

`src/cli.rs::run_source` (`src/cli.rs:272-360`) performs the surrounding source
workflow:

1. Canonicalize the requested path, require a regular file, read UTF-8 source,
   and locate the built `librecipe.rlib`.
2. Call `lower_recipe_source` once.
3. If a rewrite exists, write `SourceRewrite::generated()` to a sibling hidden
   file named like `.<original>.recipe-<pid>-<sequence>.rs` using private mode,
   close-on-exec, no-follow, and create-new flags. `TransformedRunSource` removes
   that file on drop.
4. Invoke rustc once with edition 2024, `-Dunused_must_use`, JSON diagnostics,
   Recipe's external crate alias, library search paths, and an output binary.
   A transformed source also receives `--remap-path-prefix transformed=original`.
5. Parse stderr with `DiagnosticStream`. Use mapped rendering only when both a
   rewrite and transformed file exist, otherwise use original rendering. Emit
   compiler stdout and the rendered diagnostics.
6. On a compile failure, remove the output binary and return a rustc failure
   error. On success, execute the binary with live output, remove it, and
   propagate a non-success run status.

Errors creating the temporary source, starting rustc, writing output, or
removing the binary are CLI `String` errors. They occur outside the lowering
module but determine whether its generated source reaches rustc.

## Facade and declaration state after lowering

The transformed source calls the same public builders as direct Rust source.
The frontend does not introduce a separate declaration model.

### Data chain

`recipe.data(sources)` calls `Recipe::data` in `src/facade.rs:256-264`, which
creates `Data::empty`, converts the `IntoDataSources` value, calls `Data::set`
for each source, and starts the thread-local declaration sequence. The empty
form produced by the frontend passes `()`; `IntoDataSources for ()` returns no
sources, so subsequent `.set(...)` calls add them.

`Data::set`, `target`, `exclude`, `split`, and `norm` retain immutable builder
semantics and call `remember_recipe_data` after each update. Invalid values are
recorded as the first deferred `DeclarationError`. `Data::validate` later
rejects the deferred error or a declaration with no source (`src/api.rs:470-485`).

`begin_recipe_data` also clears any previously remembered model. This prevents
a new data declaration from accidentally pairing with an earlier model. The
sequence is thread-local, so a normal `.run()` consumes declarations from the
same source-runner thread.

### Model chain and graph declaration

`recipe.model()` calls `Recipe::model` (`src/facade.rs:266-270`) and starts an
empty `Model` in the sequence. Model builder calls append typed `LayerSpec`
values, Bayesian dependencies, objectives, operations, or weight-source
metadata. Each builder remembers the latest model. The frontend's residual edit
therefore reaches `Model::residual` with one array argument, whose elements are
`ResidualOperation::Layer` and `ResidualOperation::Activation` values created by
`layer(width)` and `relu()`.

`Model::grad` receives the `Grad` produced by `recipe::clip`; it stores a
representable f32 bit pattern or defers a declaration error. `Model::validate`
checks deferred errors, checkpoint-versus-inline conflicts, an empty model,
standalone KNN placement, grouped-to-dense adjacency, layer validity, Bayesian
acyclicity, and referenced objectives (`src/api.rs:1462-1535`). None of those
checks is performed by `source_frontend`.

### Train chain and artifact declarations

`recipe.train()` returns `Train::new` and does not consume the remembered data
or model until execution. Policy calls record optimizer, horizon, learning-rate,
schedule, metrics, plotting, resume, and save declarations. The hidden pair
methods produced by the frontend are real `Train` methods, not macros:

- `__recipe_save_pair` validates one `.ogdl` model path followed by one `.cubin`
  or `.hsaco` native path.
- `__recipe_resume_pair` applies the same ordering to resume paths. A kernel
  alone is rejected by the method's first-argument check.
- `.save(path)` and `.resume(path)` remain the one-path public forms and route by
  extension.

`Train::validate` rejects the first deferred policy error, warmup at or beyond
the finite epoch bound, and invalid log or plot items. These methods are the
source of the artifact and declaration failures that later appear as typed
`TrainingError::Declaration` values.

## Training and preparation call graph

### Normal `.run()`

The direct generated call is `Train::run` (`src/training.rs:858-863`):

```text
Train::run
  -> take_recipe_training_sequence
       -> take Data and Model from thread-local RecipeSequence
  -> try_run_with(data, model)
```

Missing data or model is converted to `TrainingError::Unsupported` with a
specific declaration message. The sequence entries are taken, so a second
`.run()` cannot reuse them unless the source declares them again.

### Explicit `.run(model, data)`

The lowered `__recipe_run_with(model, data)` method (`src/training.rs:865-867`)
calls the same `try_run_with(data, model)` without reading the thread-local
sequence. It is the executable target of `API.ogdl:89`; the source frontend
only renames the method, and the method itself preserves the semantic argument
order by passing data first to the internal operation.

### Dispatch by model declaration

`Train::try_run_with` (`src/training.rs:869-904`) chooses one of three
downstream preparation paths:

```text
model has Bayesian dependencies
  -> compile_bayes_model
model contains a KNN layer
  -> compile_knn_model
otherwise
  -> compile_training_package
       -> compile_training_graph
       -> load_resume_native_bundle
       -> CheckpointManifest::from_compiled
       -> execute_current_training
       -> TrainingReport and optional artifact writes
```

The source frontend has no branch for these cases. They are selected from the
typed `Model` state produced by the facade.

### Validation and graph construction

The dense `compile_training_graph` path (`src/training.rs:592-758`) validates
the policy, data, and model before preparing anything. It then:

1. resolves a supported built-in loss and policy,
2. calls `prepare_data(data)` for file parsing, schema, targets, normalization,
   and the prepared training partition,
3. maps each `LayerSpec` through `map_dense_block` into the backend-neutral
   `DenseBlock` representation,
4. builds the finite training horizon, optimizer configuration, learning-rate
   decay, data normalization, gradient clip value, and validation configurations,
5. calls the appropriate Recipe training compiler for plain dense or structured
   blocks, and
6. conditionally loads and applies an existing semantic resume checkpoint.

`compile_training_package` then authenticates any supplied native resume bundle
and creates the checkpoint manifest. `execute_current_training` performs the
native preparation and execution lifecycle, reports requested metrics, handles
the stop request, and returns a completed execution. `try_run_with` writes a
semantic `.ogdl` model and/or a realized `.cubin` or `.hsaco` kernel only when
the corresponding declaration exists.

The calculation graph is therefore downstream of source lowering. The frontend
turns syntax into calls; `Model` state and `recipe_training` compiler functions
turn that state into dense blocks, validation-specific programs, transfers, and
native execution. No `CalculationGraph`, `LayerSpec`, or native handle is
constructed in `source_frontend.rs`.

### Data preparation boundary in detail

The `prepare_data(data)` call in `compile_training_graph` is the first point at
which a source path is opened. `src/data_prepare.rs:79-95` constructs bounded
default ingest limits and delegates to `prepare_data_with_limits`. That
function (`:140-172`):

1. revalidates the `Data` declaration,
2. requires at least one target and an explicit split fraction,
3. converts the split to a checked `TrainFraction`,
4. converts column and row exclusions to ingest predicates,
5. distills all declared sources in source order,
6. infers typed vectors with the categorical encoding model, and
7. prepares the selected table and training and validation partitions.

`DataPreparationError` distinguishes declaration, missing-target, missing-split,
f32 predicate, ingest, source-distillation, semantic-inference, and table
preparation failures. The source frontend can only make the empty data call
legal; it cannot make an empty source declaration valid. `Data::validate` and
then `prepare_data_with_limits` still reject `recipe.data().target(...).split(...)`
unless a later `.set(...)` supplied a source.

The same `Data` object has a separate target-free boundary for inference:
`distill_data` and `select_target_free_data` retain source and exclusion
semantics without applying training targets, splits, or data normalization.

### Typed graph construction after preparation

`recipe_training::compile_dense_training_impl` (`training/src/compile.rs:723-1388`)
is the concrete graph builder reached after the root crate maps `LayerSpec` to
`DenseBlock`. Its observable stages are:

1. Resolve the typed task from prepared target vectors and the selected loss.
2. Validate model block configuration, lower the prepared dataset and feature
   schema, and validate feature and target dtypes and shapes.
3. Derive the effective blocks and any required output adapter from the declared
   blocks, task, and feature width.
4. Allocate graph external inputs for training features, training targets,
   optional target supervision, validation inputs, and normalization masks.
5. Convert integer payloads only at the declared calculation boundary, then
   emit identity, z-score, min-max, or L2 normalization calculations.
6. Emit the declared block forward calculation, loss and loss gradient,
   masking and reductions, backward block calculations, optional global
   gradient clipping, and AdamW parameter updates.
7. Emit validation calculations for binary, multiclass, or regression metrics
   when the policy requested the matching family.
8. Finish a `CompiledTraining` containing a `CalculationGraph`, external inputs,
   training bounds, outputs, schema, config, blocks, layers, and optional output
   adapter.

The compiler's contract is one logical complete training partition per epoch
and one optimizer update per epoch. Physical tiling occurs below this boundary.
The frontend therefore has no reason to inspect batch sizes, matrix shapes, or
device capabilities while rewriting source.

Target resolution is also downstream and dtype-aware. One target chooses a
binary, scalar-regression, or categorical task according to its semantic type,
encoding, metadata, and loss. Multiple numeric targets form one ordered target
matrix and select the corresponding multi-target task. Incompatible target
meaning, encodings, widths, or partition dtypes become
`TrainingCompileError::InvalidTargetMatrix` or a related typed compile error,
not a source-front-end diagnostic.

### Native preparation and execution boundary

After graph compilation, `execute_current_training_native`
(`src/training.rs:1278-1338`) crosses the measured-system boundary:

```text
with_current_native_preparation
  -> load the exact current measured profile and native inputs
  -> discover and resolve local GPU inventory
  -> require complete machine and device identity
  -> build scoped CUDA/HSA bindings and target build specifications
  -> derive runtime tuning from the graph and measured profile
  -> build host arenas, cross-backend bridge, executor driver, and artifact realizer
  -> optionally attach an authenticated prebuilt resume kernel
  -> Preparer::new(provider, realizer)
  -> prepare_and_execute_local_training_controlled
```

`src/native_prepare.rs:317-366` rejects missing or mismatched profiles,
machines, GPU sets, bindings, target identities, toolchains, and native
backends before the callback receives a scope. The scope lends native handles
only for the higher-ranked callback and returns owned target specifications after
the handles are released. `with_current_native_preparation` caches the exact
probe on the current thread, but still reopens and validates the current profile
and creates per-run resources.

The native callback validates a supplied resume kernel against topology,
discovery, target, and toolchain identities, then gives the prebuilt bytes to
the deferred artifact compiler. It derives watchdog and staging settings from
the graph and measured hardware, prepares native artifacts before execution,
and passes the host stop flag into the controlled lifecycle. Native compilation,
loading, allocation, and image realization are preparation work, not additional
model declarations introduced by the frontend.

## Inference path that shares receiver classification

`RecipeReceiver::Infer` is currently used for classification only. No branch in
`lower_recipe_source` rewrites an inference method. A direct chain such as
`recipe.data(...); recipe.model().load("model.ogdl"); recipe.infer().evaluate()`
therefore compiles as ordinary Rust and follows the facade's thread-local
sequence.

`Infer::resolve_declaration` (`src/api.rs:2221-2234`) consumes the remembered
data and model, validates policy, data, and model, and produces an
`InferenceDeclaration`. `evaluate_inference_declaration`
(`src/inference.rs:432-480`) then calls `compile_inference_declaration` and
executes the resulting native program.

`compile_inference_package` (`src/inference.rs:500-543`) validates the target-free
policy, requires `Model::load` with a `.ogdl` or `.gguf` extension, loads the
semantic or GGUF model, distills and exclusion-selects target-free rows, and
dispatches by artifact family to dense, KNN, Bayesian, or GGUF Llama
preparation and compilation. Target declarations, split fractions, and data
normalization are rejected on this path because the saved model owns target and
normalization interpretation (`src/inference.rs:545-582`). Native inference
then uses the same measured `with_current_native_preparation` boundary and
artifact realizer as training (`src/inference.rs:602-659`).

This shared receiver classification is important: classifying an `Infer` chain
does not imply that source_frontend owns inference semantics. It merely prevents
the `Train` and `Model` rewrite rules from matching an unrelated method call.

### Specialized preparation failures

The specialized paths validate policy, data, and model before their own
preparation:

- `compile_bayes_model` requires Bayesian dependencies, rejects inline layers,
  loaded weights, generic objective or gradient policy, numeric normalization,
  optimizer and epoch controls, iterative metrics, and native kernel artifacts,
  then prepares categorical reference sets.
- `compile_knn_model` requires one standalone KNN layer, rejects Bayesian or
  loaded models, objective and gradient policy, optimizer, learning-rate,
  warmup and epoch controls, iterative metrics, and native training kernels,
  then prepares the immutable reference set.

These are typed downstream errors, not source parsing errors.

## Failure boundaries

### Lowering-time outcomes

`lower_recipe_source` has three classes of result:

```text
Ok(Some(rewrite)) -> write and compile generated source
Ok(None)          -> compile original source unchanged
Err(message)      -> stop before rustc and report a source-front-end error
```

`Ok(None)` is intentional for tokenization or `syn::parse_file` failure. It
does not mean the source is valid; rustc remains responsible for ordinary Rust
syntax and type diagnostics. `Err` is reserved for a detected named-gradient
shape error, a failed classification rewrite, or an invalid or overlapping
final edit set.

### Diagnostic-time outcomes

Diagnostic parsing is loss-preserving at the line level. JSON diagnostics with
missing fields are rendered with defaults. Spans that cannot be mapped remain
unchanged. A failure to parse one line does not discard subsequent lines.

`mapped_rendering_chain` can apply several rewrite/path pairs, but the CLI
currently supplies one final rewrite. The chain exists so a caller with staged
rewrites can remap a diagnostic in sequence without adding another diagnostic
format.

### Downstream outcomes

After compilation, declaration, preparation, graph, checkpoint, native, and
runtime failures are returned by `TrainingResult` and formatted by the CLI.
The frontend does not turn those errors into syntax diagnostics and does not
retry with a different declaration. A source that spells a legal Rust method
but requests an unsupported Recipe feature compiles and then fails at the
typed training boundary where that feature is inspected.

The root training error boundary is `src/training.rs:34-116`:

| Variant | Boundary reached after source lowering |
| --- | --- |
| `Declaration` | `Train`, `Data`, or `Model` validation rejected a deferred or cross-field declaration error. |
| `Data` | Bounded source distillation, semantic vector inference, selection, or partition preparation failed. |
| `Compile` | `recipe_training` graph, shape, dtype, task, block, metric, or horizon compilation failed. |
| `Checkpoint` | Semantic model or native kernel serialization failed. |
| `Resume` | Existing semantic model resume could not be decoded or applied. |
| `NativeKernelSource` | A supplied native resume source could not be read or authenticated. |
| `Native` | Measured profile, target, binding, toolchain, artifact, or local runtime preparation failed. |
| `Unsupported` | The declaration is well formed but the selected training path has no implementation for it. |
| `Runtime` | Native execution, metric presentation, signal handling, or another runtime stage failed. |

Inference has a parallel typed boundary in `src/inference.rs:34-116` with
`Declaration`, `Data`, `Model`, `Compile`, `Native`, `Execute`, `Runtime`, and
`Unsupported` variants. Neither error enum has a source-front-end variant,
which keeps syntax adaptation separate from domain validation and execution.

## API.ogdl coverage and current gaps

### Covered source-level adaptations

The current pass directly covers the non-Rust forms represented by
`API.ogdl:21,58,80-89`: variadic residual branches, named gradient clipping,
literal two-path save and resume, and explicit two-argument run. It also adds
the empty data convenience form and the `()` bridge required by the facade.

All other ordinary one-argument or zero-argument Rust calls in the
specification are left as normal method calls. The source frontend does not
claim that every API.ogdl declaration is executable. Backend support is
determined later by the facade validators and training compilers.

### Observed implementation limits

- Receiver inference is lexical and name based. It has no Rust type checking,
  scope model, or alias hygiene, so shadowed names and unusual wrappers can be
  classified incorrectly or not classified at all.
- The fixed-point binding loop assumes one stable class per binding name. Two
  shadowed or duplicate locals with the same name but different explicit or
  inferred classes can make the loop overwrite the entry on every pass and
  never converge before rustc gets a chance to reject the source.
- The named-gradient scanner recognizes only a parenthesized `.grad` group
  whose first two tokens look like `IDENT :`. It intentionally does not rewrite
  an unrecognized or non-Model call.
- A named-gradient classification probe replaces all candidate arguments with
  one valid `::recipe::clip(1.0)` expression. If the complete probe file is not
  parseable, the function returns `Ok(None)` and lets rustc report the original
  source. There is no second attempt that probes a subset.
- Because the probe replaces every candidate before AST visitation, a nested
  named `.grad` inside another candidate's argument can disappear from the
  classification AST and remain unrevised in the final source.
- The final edit builder accepts edits at the same zero-width boundary. The
  mapping policy is deterministic but cannot represent a unique original
  character for every inserted byte.
- Mapped diagnostics use a normalized renderer rather than rustc's `rendered`
  field, and raw path replacement is a textual replacement. This is sufficient
  for the temporary sibling source but is not a general source-map protocol.
- Diagnostic byte offsets are assumed to identify UTF-8 boundaries in the
  original source. `line_column` slices by byte range without a boundary check,
  so a malformed or foreign span with an interior UTF-8 offset is outside the
  remapper's safe input contract.
- `SourceRewrite` retains both complete source strings. It is simple and
  precise for a source-run invocation, but source size directly affects its
  memory footprint.
- The module has no public API. Its only production caller is `cli::run_source`,
  and its only diagnostic consumer is the same caller. There are no direct
  module-level runtime tests in `src/source_frontend.rs`; end-to-end coverage
  must enter through `recipe run`.
- The hidden pair and explicit-run methods are part of the source contract, but
  direct Rust callers can still name them because they are public with
  `#[doc(hidden)]`. The frontend does not enforce that callers use the literal
  forms from `API.ogdl`.

### Historical boundary

The original implementation (introduced in commit `02ea27a`) inspected rustc
diagnostics and used E0061 to decide whether to insert arity adapters. The
`b7769cb` implementation added a named-gradient probe that also depended on
compiler diagnostics. Commit `958664f` replaced both with the explicit
receiver-aware `lower_recipe_source` pass and removed the retry helpers. The
current source retains the deterministic pass and the pair-artifact
adaptations, with the empty-data and explicit-run adaptations present in the
current working tree. This history explains why diagnostics remain in the
module while no diagnostic controls parsing or lowering anymore.

## Evidence map

| Question | Primary evidence |
| --- | --- |
| Where is lowering invoked? | `src/cli.rs:311-328` |
| Where are transformed diagnostics mapped? | `src/cli.rs:329-333`, `src/source_frontend.rs:335-370` |
| What source forms are rewritten? | `src/source_frontend.rs:419-516` |
| How are named fields validated? | `src/source_frontend.rs:519-622` |
| How are edits and offsets represented? | `src/source_frontend.rs:154-239, 624-667` |
| How does ordinary `.data()` reach the facade? | `src/facade.rs:193-264` |
| How are declaration values remembered? | `src/facade.rs:142-190`, `src/api.rs` builder methods |
| What validates declarations? | `src/api.rs:470-485, 1462-1535, 2081-2094` |
| How does normal and explicit run dispatch? | `src/training.rs:858-904` |
| Where are preparation and graph compilation selected? | `src/training.rs:581-758, 1780-2110` |
| What is the normative syntax inventory? | `API.ogdl:1-94` |
