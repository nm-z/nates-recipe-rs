# `src/cli.rs`: production command-line boundary

[`src/cli.rs`](../../src/cli.rs) is the complete implementation of the
installed `recipe` command. The binary at [`src/main.rs`](../../src/main.rs)
does no parsing or work of its own: its `main` function calls
`recipe::cli::main()`, and [`src/facade.rs`](../../src/facade.rs) exposes that
module by including this file. The CLI is therefore the process boundary for three
operations:

```text
recipe run FILE.rs [ARGS...]
recipe probe [OPTIONS]
recipe convert INPUT OUTPUT
```

Cargo declares both the `recipe` library and the `recipe` binary; the binary's
only source is `src/main.rs`, while `src/lib.rs` includes the facade that exposes
`cli`. The binary has no separate test or parser entrypoint, so this module is
the production boundary for every installed command invocation.

The module is deliberately small in its public surface, but it also owns the
private state handoff used by native preparation. It converts operating-system
arguments into one of the three real workflows, turns every failure into a
single human-readable `String`, and lets the crate root's process entry point
map success or failure to the process exit code.

## Process entry and dispatch

`main` collects `env::args_os().skip(1)` into `Vec<OsString>` and passes that
vector to `run`. It does not use a third-party argument parser. A successful
`run` returns `ExitCode::SUCCESS`. Any `Err(String)` is printed as
`recipe: {error}` on standard error and returns `ExitCode::FAILURE`. No panic is
intended as a user-facing command result; panics that happen in a worker thread
are converted to errors where the CLI joins that thread. The module does not
call `process::exit`; the returned `ExitCode` is the process termination value
consumed by `src/main.rs`.

`run` examines only the first argument and keeps the original `OsString` until
the command name must be interpreted as UTF-8:

| input | behavior |
| --- | --- |
| no argument | error `missing command` followed by the usage block |
| `-h` or `--help` | print the usage block and succeed |
| `run` | require a source argument, then call `run_source`; remaining arguments are passed unchanged to the compiled child |
| `probe` | accept `-h` or `--help` only as its sole option, parse path options, then call `run_probe` |
| `convert` | accept `-h` or `--help` only as its sole argument, then call `run_model_conversion` |
| any other valid UTF-8 command | error naming the command and the three accepted names, followed by usage |
| a non-UTF-8 command | error `commands must be valid UTF-8` |

For `run`, a missing second argument is a distinct error stating that a Rust
source file is required. `run -h` or `run --help` prints usage and returns before
any later arguments are considered. `run FILE.rs -h` treats `-h` as a child
argument, not as CLI help, because only the source-position argument is checked
for help. For `probe` and `convert`, help is recognized only when
`arguments.len() == 2`; `probe --help EXTRA` and `convert --help EXTRA` follow
the normal option or extension/arity path instead of printing help.

The usage text is intentionally the one authoritative synopsis in this file:

```text
Usage:
	recipe run FILE.rs	[ARGS...]
	recipe probe		[OPTIONS]
	recipe convert INPUT OUTPUT
```

Help is the only dispatch path that writes a fixed block and returns without
touching private state. It writes `USAGE` to standard output and returns
`ExitCode::SUCCESS`. Successful `run` has no parent-generated summary: the
compiler and child streams are the output. Successful `probe` and `convert`
are the only paths with CLI-owned success lines. Every failure, including a
downstream child or compiler failure, is prefixed once by `main` as
`recipe: ...` on standard error.

The boundary cases can be read without inferring parser state:

| argument shape | stdout | stderr/result |
| --- | --- | --- |
| no arguments | none | `missing command` plus usage, failure |
| `--help` or `-h` | usage | success |
| `run` | none | missing-source message plus usage, failure |
| `run --help [anything...]` | usage | success, later arguments ignored |
| `run SOURCE [ARGS...]` | compiler/child streams | child result after cleanup |
| `probe --help` or `probe -h` | usage | success |
| `probe [OPTION VALUE ...]` | probe summary only after receipt commit | option/probe failure or success |
| `convert --help` or `convert -h` | usage | success |
| `convert INPUT OUTPUT` | conversion summary on success | arity/extension/stream failure or success |
| any other first argument | none | unknown-command message plus usage, failure |

## Production function map

The following inventory is the complete call surface in `src/cli.rs`; helpers
below the three command branches do not create alternate workflows.

| function or type | role and authoritative effect |
| --- | --- |
| `main` | Converts `run`'s `Result` into `ExitCode`, the only process termination value. |
| `run` | Decodes the first command and delegates exactly once to `run_source`, `run_probe`, or `run_model_conversion`. |
| `ProbeOptions` | Holds parsed option paths until `run_probe`; it has no defaults beyond empty/`None` values. |
| `CurrentNativeInputs` | Owns the config, fresh host inventory, exact profile identity/path, and native probe returned to preparation. |
| `PinnedNativeFile` | Couples one canonical native path with its 32-byte artifact digest. |
| `ActiveNativeReceipt` | Owns the complete serialized v1 runtime handoff and is the sole receipt authority. |
| `run_model_conversion` | Validates two paths, selects the extension direction, invokes one ingest stream, and prints the conversion summary. |
| `conversion_limits` | Derives one finite `GgufLimits` from observed source and declared output lengths. |
| `write_new_conversion_output` | Owns create-new output admission, stream callback, synchronization, and partial-file removal. |
| `run_source` | Owns source admission, library/state lookup, frontend lowering, one compilation, child execution, binary removal, and final child-status interpretation. |
| `LiveRunOutput` / `CapturedOutputTail` | Carry the final child status and bounded stdout/stderr tail evidence from the live boundary to `run_status_message`. |
| `OutputTailBuffer::{new,extend,finish}` | Retains bounded per-stream tails and truncation state; it never changes live forwarding. |
| `RunOutputDestination::{Stdout,Stderr}` / `name` | Selects the parent stream and supplies the only destination-specific text used by stream errors and thread names. |
| `run_compiled_source_live` | Installs SIGINT handling, spawns the child and two forwarders, polls/waits, joins, and returns `LiveRunOutput`. |
| `spawn_run_output_forwarder` | Names one stdout or stderr thread and connects it to `forward_run_output_stream`. |
| `forward_run_output_stream` | Reads 8 KiB chunks, writes and flushes each chunk, and builds a tail. |
| `join_run_output_forwarder` / `thread_panic_detail` | Converts a forwarder panic or `Result` error into the parent `String` boundary. |
| `run_status_message` / `allocation_bytes` / `bytes_to_gb` | Derives the bounded OOM, signal, exit-code, and truncation wording from final status and tails. |
| `compile_run_source` / `emit_compiler_output` | Performs one `rustc` process invocation and emits its stdout plus normalized diagnostics. |
| `TransformedRunSource::{create,path}` and `Drop` | Publishes and removes one sibling rewrite file when the frontend produced edits. |
| `locate_recipe_library` / `newest_recipe_library` / `newest_cargo_build_output_library` | Resolve the one `librecipe.rlib` and its library root using the fixed search order. |
| `cargo_build_output_roots` | Enumerates bounded Cargo `out` roots used as `rustc -L` inputs. |
| `parse_probe_options` / `set_once` | Parse strict option/value pairs and enforce single-valued options. |
| `run_probe` | Performs bare-metal admission, discovery, identity/cache, measurement, receipt publication, and probe summary output. |
| `current_native_inputs` | Reopens the active receipt or derives the default exact identity path for native preparation. |
| `ActiveNativeReceipt::{capture,reopen_config,encode,decode}` | Captures, validates, serializes, and reconstructs the immutable native handoff. |
| `PinnedNativeFile::{from_tool,reopen_tool}` | Captures and revalidates canonical tool paths and SHA-256 digests. |
| `profile_uses_backend` / `required_selected_library` / `reopen_library` | Select backend pins only when the measured profile uses that backend, then reopen exact files. |
| `inspect_pinned_regular_file` / `inspect_private_regular_path` / `inspect_canonical_directory` / `inspect_private_directory` / `require_effective_user` | Enforce path, file-type, canonicality, ownership, and permission invariants. |
| `write_active_native_receipt` / `load_active_native_receipt` | Atomically publish or race-check/read the receipt file. |
| `PendingReceipt::{create,Drop}` | Allocate at most 64 private same-directory temporary names and remove uncommitted files. |
| `write_receipt_field`, `encode_optional_pin`, `encode_pin`, `decode_optional_pin`, `decode_required_pin`, `decode_pin` | Encode and decode the fixed receipt field values and optional pins. |
| `encode_os`, `encode_hex`, `decode_path`, `decode_label`, `decode_digest`, `decode_hex`, `receipt_hex_nibble`, `parse_decimal` | Implement canonical byte-hex, label, digest, path, and decimal scalar codecs. |
| `native_config` / `configured_or_default` / `required_tool` / `optional_tool` | Build one explicit native configuration from options and fixed candidate lists. |
| `private_state_root` / `ensure_private_directory` | Select and enforce the private `recipe-next` state tree. |
| `require_bare_metal` | Reject observed container or nested-PID markers; it is not a general command parser. |
| `hex` | Render profile/cache digests as lowercase hexadecimal for names and output. |

The module-level constants are the CLI-owned boundary values: the receipt filename,
magic and 64 KiB maximum; 64 KiB child tails and 8 KiB forwarding buffers;
private mode `0600`, group/other mask `0o077`, no-follow/close-on-exec flags;
the two backend identity strings; the sixteen receipt field names; and the
fixed usage block. Native benchmark sizes, rates, and durations remain owned by
the probe engine and seed contract rather than being duplicated here.
Small local-operation literals such as the 4-rank conversion bound, the
4,096 Cargo output-root cap, the 10 ms child poll interval, and decimal
gigabyte divisor are intrinsic to those operations and are not user settings.

For source-oriented navigation, the current line ranges are: dispatch and
conversion `src/cli.rs:115-270`, source compilation and live execution
`:272-839`, probe option parsing and measured-profile publication `:841-947`,
runtime handoff and receipt codec `:949-1746`, and native configuration, private
state, bare-metal checks, and digest formatting `:1747-2002`.

## `recipe convert`: bounded structural model conversion

`run_model_conversion` requires exactly two `OsString` values after the command.
The values are treated as paths without first canonicalizing them. Extensions
are compared case-sensitively as the exact `OsStr` values `gguf` and `ogdl`;
the input and output extensions select one of exactly two directions:

* `.gguf` input to `.ogdl` output opens the source, obtains its byte length,
  builds conversion limits, wraps the source in `BufReader`, creates the output,
  and streams `gguf_to_structural_ogdl_stream` into a `BufWriter`.
* `.ogdl` input to `.gguf` output opens the source, obtains its byte length,
  asks `structural_ogdl_declared_gguf_bytes` for the declared binary length,
  builds conversion limits from both lengths, and streams
  `structural_ogdl_to_gguf_stream` into the new output file.

Any other extension pair fails before opening the source with an error that
states the required `.gguf .ogdl` or `.ogdl .gguf` forms and shows the supplied
paths. A missing extension is part of that same rejection. Arity errors happen
before extension inspection.

`conversion_limits` turns the observed source and output lengths into one
`GgufLimits` value. `source_bytes.max(1)` bounds the file, metadata-pair,
tensor-count, aggregate-string, aggregate-array-element, and array-depth
budgets; `output_bytes.max(1)` is the file-byte budget for the declared output;
and the rank bound is the intrinsic value `4`. This gives the streaming ingest
layer finite bounds derived from the files rather than an unbounded allocation
policy. Failure to construct those limits is reported as
`construct conversion bounds: ...`.

`write_new_conversion_output` is the output transaction used by both directions:

1. Open the destination with `create_new(true)`, read/write access, and mode
   `0600`. Existing files are never overwritten.
2. Invoke the supplied streaming writer.
3. On success, call `sync_all` before returning.
4. On any writer or sync failure, close the file and remove the partial path.
   If removal also fails, return both the original error and the cleanup error.

The successful command prints `Converted INPUT -> OUTPUT` and returns `Ok(())`.
It does not write the Recipe private state directory, a profile, a receipt, or
any intermediate artifact. The only user-owned file created by this command is
the requested destination, and a failed conversion leaves no partial output
when cleanup succeeds.

The ingest streams are seekable, bounded transformations rather than an
in-memory image copy. GGUF-to-OGDL seeks to the source end, validates the full
GGUF v3 layout, rewinds, and emits canonical structural metadata and tensor
records while reading payload chunks. OGDL-to-GGUF first reads and rewinds the
declared `file_bytes` preamble, performs a structural validation pass, rewinds
again, and writes a GGUF v3 header, metadata, tensor descriptors, and payload
through bounded passes. The declared output length is therefore part of the
input structure and the actual destination must begin empty.

GGUF validation checks the `GGUF` magic, version 3 and endianness, bounded
metadata/tensor counts, unique JSON metadata keys and tensor names, nonzero
aligned layout, rank at most four, tensor spans and zero padding, and each
typed tensor block before it is emitted. Structural OGDL validation checks the
canonical schema/preamble, alignment and declared byte length, metadata types
and array depths/elements, unique keys, tensor descriptors and non-overlapping
aligned offsets. The reverse stream checks the preamble and descriptors on each
pass, rejects input mutation between passes, writes payloads at declared
offsets, flushes and reparses the resulting GGUF before returning.

## `recipe run`: source compilation and live child execution

`run_source` is the source-runner path used by examples, generated training
programs, and the hardware acceptance runner. It compiles the requested Rust
file once, links it against the current Recipe library, runs the resulting
binary in the source directory, forwards output live, and removes the temporary
binary before deciding whether the training program succeeded.

### Source validation and temporary state

The requested `OsStr` is first canonicalized. Canonicalization and metadata
failures are reported as `read training source PATH: ...`, even though the
first operation is path resolution. The canonical target must be a regular
file. Its canonical parent is retained as the child working directory. The
source is read as UTF-8 text; invalid UTF-8 is a source-read failure.

The runner then locates the Recipe `rlib` and the private runtime state:

* `locate_recipe_library` checks the executable directory, its profile parent
  when the executable is under `deps`, the current directory's
  `target/debug` and `target/release`, and `/usr/lib/recipe`, in that order.
  For each root it prefers the newest `librecipe-*.rlib` in `deps` whose
  followed metadata is a regular file; then the newest matching file under
  `build/recipe/*/out`; then a direct `librecipe.rlib` whose followed metadata
  is a file. If none is found, the command fails
  with `librecipe.rlib is unavailable; build Recipe first or install its
  source-runner libraries under /usr/lib/recipe`.
* `private_state_root` resolves the state base from an absolute
  `XDG_CACHE_HOME`, or from the canonicalized `HOME/.cache` when that variable
  is absent or relative. It creates the base when necessary, then uses the
  private `recipe-next` directory. `run_source` creates a private `run`
  subdirectory below it.

The runner allocates a process-local monotonic source sequence and names the
compiled executable `recipe-next/run/recipe-run-PID-SEQUENCE`. The sequence is
not persisted and is used only to avoid collisions between concurrent or
repeated invocations in one process. The `--extern` value is assembled as
`recipe=PATH_TO_SELECTED_RLIB`.

### Deterministic Recipe source lowering

Before invoking `rustc`, `run_source` calls
[`source_frontend::lower_recipe_source`](source_frontend.md) exactly once.
That pass is source-driven. It parses tokens and syntax, classifies the
explicit `recipe` facade and local `Data`, `Model`, `Train`, and `Infer`
bindings, and makes one deterministic set of edits. It never waits for a Rust
diagnostic to decide whether to rewrite or retry.

The currently supported CLI-visible rewrites are:

* a zero-argument facade `.data` call receives `()` so the public builder can
  express the source form;
* a model `.residual` call with two or more arguments receives bracket syntax;
* a model `.grad(clip: EXPR)` named field is converted to the public
  `clip(EXPR)` form. Duplicate, unknown, missing, or malformed named fields
  produce a source-located error before `rustc` starts;
* two-argument train calls are renamed to the hidden API methods
  `__recipe_save_pair` and `__recipe_resume_pair` (the frontend checks the
  receiver and arity, not whether the arguments are literals);
* a two-argument Train `.run(...)` call is renamed to `__recipe_run_with`.

The hidden pair methods are ordinary `Train` methods after lowering: they
validate a semantic `.ogdl` model path first and a `.cubin` or `.hsaco` native
path second. The hidden two-argument run method calls the same training
implementation with the supplied `Model` and `Data` references; unlike the
ordinary `.run()`, it does not consume the facade's thread-local declaration
sequence. These methods therefore adapt syntax without introducing a second
execution path.

Receiver classification is intentionally source-level. The frontend recognizes
an unbound path named `recipe`, the four facade methods `data`, `model`, `train`,
and `infer`, and simple local identifier bindings whose explicit type or
initializer resolves to `Data`, `Model`, `Train`, or `Infer`. It follows groups,
parentheses, references, and Recipe method chains, but does not perform Rust
type checking or lexical-scope analysis. A call that cannot be classified is
left for `rustc` unchanged. Named-gradient candidates are first probed by
replacing their argument list with `::recipe::clip(1.0)` so `syn` can parse the
classification source; final edits are then applied to the original byte
boundaries. Overlapping or non-UTF-8-boundary edits return a frontend error.

If the source cannot be tokenized or parsed as a Rust file, lowering returns
`None` and the original source is compiled. If edits exist,
`TransformedRunSource::create` writes the generated source next to the original
as a hidden `.<original-file-name>.recipe-PID-SEQUENCE.rs` file, using
`create_new`, mode
`0600`, `O_CLOEXEC`, and `O_NOFOLLOW`. Its `Drop` implementation removes the
temporary source whether compilation succeeds or fails. The compiler receives
`--remap-path-prefix GENERATED=ORIGINAL`, so diagnostics still name and point
at the user's source.

### The exact `rustc` invocation

`compile_run_source` selects `RUSTC` from the environment, defaulting to the
literal `rustc`. It executes one compiler process with:

```text
SOURCE
--crate-name recipe_run
--crate-type bin
--edition 2024
-Dunused_must_use
--error-format=json
-L LIBRARY_ROOT
[ -L LIBRARY_ROOT/deps ]
[ -L each LIBRARY_ROOT/build/*/*/out directory ]
[ --remap-path-prefix GENERATED=ORIGINAL ]
--extern recipe=RECIPE_RLIB
-o PRIVATE_BINARY
```

`cargo_build_output_roots` walks only the direct package and variant output
directories below `LIBRARY_ROOT/build`, collects `out` directories, sorts them,
and rejects more than 4096 roots. Directory-read and metadata failures are
returned rather than silently ignored, except for a missing build directory,
which simply contributes no additional `-L` values. Failure to start `rustc` is
reported as `start rustc for SOURCE: ...`.

The `Output` returned by `rustc` contains complete compiler stdout and stderr
before the CLI renders them; unlike the child path, compiler output is not
tail-bounded by `OutputTailBuffer`.
`DiagnosticStream::parse` treats JSON lines whose `$message_type` is
`diagnostic` as structured diagnostics and keeps every other line as raw text,
using UTF-8 loss replacement for raw bytes that are not valid UTF-8. Without a
rewrite, a structured entry's `rendered` field is preserved when present. With
a rewrite, every structured entry is remapped and rendered by the CLI's
normalizer, while raw entries only undergo generated-to-original path
replacement. The CLI writes compiler stdout to its stdout and the rendered
diagnostics to stderr. A failure to write either stream is itself a CLI error.
A failed compiler still has any successfully emitted output written before the
runner removes the attempted binary (ignoring that removal's result) and
returns `rustc failed for SOURCE with STATUS`. If writing compiler output itself
fails, `run_source` returns that write error before reaching the status branch,
so this particular path does not attempt the binary removal. There is no second
compiler invocation and no diagnostic-driven source rewrite retry.

### Live child process and Ctrl+C

After successful compilation, `run_compiled_source_live` installs the process
SIGINT guard from [`src/signal.rs`](../../src/signal.rs), starts the binary with
the user's remaining arguments, and sets its current directory to the canonical
source parent. Child
stdout and stderr are piped. The two pipes are consumed by separate named
threads (`recipe-run-stdout` and `recipe-run-stderr`) so the child cannot block
on a full pipe while the parent polls its status. The child inherits the
parent's environment and stdin; only stdout and stderr are replaced by pipes.

Each forwarder reads in 8 KiB chunks, writes the same bytes immediately to the
corresponding CLI stream, flushes after each chunk, and retains only the last
64 KiB in an `OutputTailBuffer`. If a chunk or cumulative output exceeds that
bound, the oldest bytes are discarded and a `truncated` flag is retained. A
forwarder read, write, flush, thread-spawn, or join failure is a real CLI error.
If spawning the second forwarder fails, the child is killed and waited for and
the first forwarder is joined before returning.

The parent polls `try_wait` every 10 milliseconds. The first observed Ctrl+C
sets a process-scoped atomic flag through `SigintGuard`; the parent forwards one
SIGINT to the child and never forwards a second one. An already-exited child
(`ESRCH`) is accepted. Any other forwarding failure kills and waits for the
child and becomes `forward SIGINT to training child PID: ...`. A `try_wait`
failure likewise kills and waits, then reports `wait for SOURCE: ...`.

After the child exits, both forwarder threads are joined. The guard restores the
previous SIGINT action when this function returns. If either pipe cannot be
taken from the spawned child, the corresponding capture error is returned
immediately (the child is then dropped); failures while starting the first or
second forwarder perform the explicit kill/wait cleanup described above.
`run_source` removes the compiled binary immediately after
`run_compiled_source_live` returns, even when that function returned an error.
A removal failure is `remove compiled training binary PATH: ...`; when the live
run itself returned a successful `LiveRunOutput`, this cleanup error is checked
before child-status interpretation. If the live run returned an error, that
live-run error is returned and the cleanup result is not reintroduced.

### Child status and failure text

If the child exits successfully, `run_source` returns `Ok(())`; all child output
has already reached the user's terminal. A non-success status is summarized by
`run_status_message`, which inspects only the bounded stderr tail and the OS
status:

1. If the retained stderr tail is valid UTF-8 and its first
   `memory allocation of N` marker is followed by decimal digits, the reason
   is `out of memory while requesting N bytes (X.XXX GB)`. The conversion uses
   decimal gigabytes, dividing by 1,000,000,000. A missing digit sequence,
   overflow, or invalid UTF-8 falls through to status inspection.
2. Otherwise a signal status is `terminated by signal N`.
3. Otherwise an ordinary exit is `exited with code N`.
4. Any status without a signal or numeric code is rendered as `exited with
   STATUS`.

The complete error is `training source SOURCE failed: REASON`, with an
additional marker when either retained tail was truncated: the suffix names
`captured stdout tail was truncated`, `captured stderr tail was truncated`, or
both. The actual tail bytes are not duplicated into this final error because
they were already streamed live. A child may therefore fail after producing
useful diagnostics without causing the CLI to buffer an unbounded transcript.

### Downstream child boundary

`run_source` does not interpret a declaration, build a graph, or choose a
device. The compiled child links the same `recipe` facade and owns all
declaration and execution state. `recipe.data(...)` and `recipe.model()` update
the child thread's `RecipeSequence`; `recipe.train()` and `recipe.infer()` only
construct policies. Ordinary `Train::run()` or `Infer::evaluate()` consumes the
preceding pair, validates it, and returns typed training or inference errors
that the child prints or propagates through its own `main` (see the declaration
details in [`api.md`](api.md)).

The frontend's hidden `__recipe_run_with(model, data)` calls the same internal
training implementation with explicit references instead of consuming that
sequence. Native training and inference preparation then call
`with_current_native_preparation`, which reaches this file's
`current_native_inputs` handoff. The CLI observes only the resulting child
streams and `ExitStatus`; it never treats a printed report, artifact path, or
success-looking line as completion evidence. A successful child status is the
authoritative `recipe run` result.

Within [`Train::run`](training.md), Bayesian declarations and a standalone KNN block produce
reference-model reports without an optimizer loop; their semantic model saves
are still child-owned and a native-kernel save is rejected. Other models compile
the dense training package, optionally authenticate a semantic resume and
prebuilt native kernel, prepare the measured native scope, run the immutable
`init -> loop -> exit` lifecycle, and save the declared semantic/native
artifacts after a completed report. Any declaration, preparation, execution, or
artifact-write error becomes a non-success child status observed by the CLI.

Artifact declarations remain independent of the CLI's temporary binary. A
missing `.resume` model starts a fresh training run rather than rejecting an
independent `.save`; a supplied native resume kernel is used only when its
semantic model and measured identities authenticate it. Omitting `.save` writes
no user-owned model or native kernel, while one-path and literal two-path saves
select the `.ogdl`, `.cubin`, or `.hsaco` outputs described by the declaration.

[`Infer::evaluate()`](inference.md) requires target-free data and a model loaded from `.ogdl` or
`.gguf`, distills and prepares the declared rows, executes the selected dense,
KNN, Bayesian, or GGUF model through the same native handoff, then writes its
tab-separated report and prediction rows to the child's stdout. Unsupported
targets, splits, redeclared normalization, missing model sources, model decode
errors, native preparation errors, execution errors, or report-write errors all
surface as a failed child status; the parent forwards any rows already written.

## `recipe probe`: discovery, measurement, and the active native receipt

### Option grammar

`parse_probe_options` consumes the remaining arguments strictly as option/value
pairs. Every option therefore requires a following path, and a dangling option
reports `option OPTION requires a path argument`. Option names must be UTF-8;
path values remain `OsString` data in a `PathBuf` and may be non-UTF-8.
The accepted options are:

| option | stored value | repetition |
| --- | --- | --- |
| `--contract PATH` | alternate seed contract | once |
| `--profile PATH` | explicit measured-profile destination | once |
| `--cuda-driver PATH` | ordered CUDA Driver library candidates | repeatable |
| `--hsa-runtime PATH` | ordered ROCr/HSA library candidates | repeatable |
| `--llvm-opt PATH` | LLVM `opt` | once |
| `--llvm-llc PATH` | LLVM `llc` | once |
| `--lld PATH` | LLVM `ld.lld` | once |
| `--ptxas PATH` | NVIDIA `ptxas` | once |

Unknown options with a value fail with the option name and the usage block;
because arity is checked first, an unknown option with no following value gets
`option OPTION requires a path argument` instead. `set_once` rejects a second
occurrence of each single-valued option. Library options are intentionally
repeatable because their order is the backend candidate order.

### Probe execution order

`run_probe` performs these operations in order:

1. `require_bare_metal` rejects known container environments before any native
   probing. It checks `/.dockerenv` and `/run/.containerenv`, scans PID 1's
   cgroup for `docker`, `containerd`, `kubepods`, `libpod`, or `lxc`, and rejects
   a process whose `NSpid` line shows a nested PID namespace. A positive match
   returns an explicit `recipe probe requires bare metal` error.
2. The seed is read from `--contract PATH`, or parsed from the checked-in
   `topology/contract.toml` with `include_str!`. The seed is only the bounded
   benchmark contract. `SeedContract` also checks its schema, required
   discovery/measurement gates, and the mandated `recipe probe` plus
   `bare-metal` identity.
3. The private state root is resolved. `scratch` is created and verified as a
   private directory. The host discovery object is
   `LocalSystemDiscovery::with_benchmark_roots(vec![state_root.clone()])`, so
   storage discovery can choose a writable location on a mounted device while
   keeping benchmark state below the private root when possible.
4. `discover_host` reads the current Linux procfs, sysfs, and `/etc` identity,
   discovers RAM, storage, and network domains, and returns a `HostInventory`.
   The first discovered RAM domain supplies `host_memory_key`; no RAM domain is
   an immediate error.
5. `native_config` selects the exact native libraries and offline tools, and
   `NativeGpuProbe::new` validates the configuration while constructing both
   CUDA Driver and ROCr/HSA backend adapters. The adapters open and enumerate
   their runtimes when the engine performs discovery. A missing backend library
   is allowed only when that vendor has no discovered hardware; a present but
   unloadable or incomplete backend is a hard error.
6. `ProbeEngine::new` combines host discovery, native GPU discovery, local host
   benchmarks, and native GPU benchmarks. The CLI supplies an empty peer list,
   so this invocation measures one local host and no remote sessions. The
   engine's `current_cache_identity` performs discovery-only inspection and
   produces the exact identity used for the profile filename.
7. Unless `--profile` supplied a path, the destination is
   `state_root/profiles/measured-vSCHEMA-DIGEST.recipe-profile`; the profiles
   directory is created and verified private. `ExplicitPathProfileCache` then
   validates the absolute path and its existing canonical private parent (an
   explicitly supplied relative path or missing parent fails). `was_cached` is
   captured before loading, so the final `source=` line describes whether the
   requested path existed at that check, not a later race with another writer.
8. `load_or_probe_and_store` recomputes the identity, attempts an exact cache
   load, and otherwise performs bounded RAM, storage, GPU, and peer benchmarks,
   validates the resulting topology and discovery profile, and atomically stores
   the fully measured profile. A cache is accepted only when its identity,
   ownership, mode, file type, inode, size bound, and encoded profile all match.
9. `ActiveNativeReceipt::capture` validates the profile and every native input
   needed to reproduce it, pins paths and SHA-256 digests, and
   `write_active_native_receipt` atomically installs the receipt in the private
   state root. Training and inference use that handoff rather than selecting a
   merely newest profile or library.
10. The command prints the profile path, whether the profile was a validated
    cache or a fresh measurement, the profile cache identity, topology and
    discovery identities, and counts of machines, devices, and directed links.

The probe does not print rates, tool paths, or raw hardware inventory. Those
remain encoded in the measured profile and authenticated receipt for the native
preparation boundary.

The seed contract is therefore input policy, not runtime truth. Measured rates,
topology, discovery properties, origins, and cache identity come from the
validated `MeasuredProfile`; canonical paths, file digests, scalar ABI settings,
and the selected profile path come from `active-native-v1`. Native preparation
must satisfy both records before it can construct a live execution scope.

The profile codec is part of that cache boundary. It rejects files over its
256 MiB limit or below the binary header/checksum minimum, verifies the profile
magic, codec schema, SHA-256 payload checksum, and complete payload consumption,
then revalidates profile schema, benchmark bounds, topology/discovery
consistency, canonical ordering, stable origins, and measured provenance. A
profile that only decodes structurally but fails any of those checks is not a
cache hit and is returned as a profile-cache error.

On a fresh measurement, `ProbeEngine` derives each RAM, storage, GPU, and
network plan from the seed estimate, clamps the buffer to 4 KiB through 64 MiB,
uses eight iterations, and caps each benchmark at two seconds. It measures all
discovered RAM and storage domains and all GPUs, validates every measurement,
builds topology/discovery identities and origins, and persists only the fully
validated result. The CLI passes an empty peer slice, so the network-plan loop
has no peer session to execute in this command.

For each GPU, `NativeGpuProbe` reopens the selected backend and rechecks the
descriptor identity before allocating bounded host/device buffers. It measures
host-to-device, device-to-host, and device-to-device transfers, then lowers and
executes a Recipe-owned dependent f32 FMA kernel through the pinned offline
toolchain, verifies the returned values, and records measured capacity,
throughput, and calculation rate. Backend disappearance, identity drift,
allocation limits, toolchain failures, timeout, or verification mismatch is a
probe error, not a partial profile.

### Native configuration selection

`native_config` makes all probe inputs explicit in one `NativeProbeConfig`:

* LLVM `opt` and `llc` are required. An explicit option is inspected directly;
  otherwise fixed absolute candidates are checked in this order: `/usr/bin`,
  `/usr/local/bin`, `/usr/lib/llvm-22/bin`, `/usr/lib/llvm-21/bin`,
  `/usr/lib/llvm-20/bin`, `/usr/lib/llvm-19/bin`, and `/opt/llvm/bin`.
* LLVM `ld.lld` and NVIDIA `ptxas` are optional. If explicitly supplied they
  must inspect successfully. Without an explicit path, the fixed candidate
  lists are tried and absence yields `None`. `ld.lld` checks `/usr/bin`,
  `/usr/local/bin`, LLVM 22, 21, and 20 directories, then `/opt/llvm/bin`;
  `ptxas` checks CUDA 11.8, 11.7, 11.6, 11.5, and 11.4 under `/opt`, CUDA
  11.8 and 11.4 under `/usr/local`, then `/opt/cuda/bin`,
  `/usr/local/cuda/bin`, and `/usr/bin`.
* CUDA candidates default, in order, to
  `/usr/lib/x86_64-linux-gnu/libcuda.so.1`, `/usr/lib64/libcuda.so.1`,
  `/usr/lib/libcuda.so.1`, and `/usr/local/nvidia/lib64/libcuda.so.1`. HSA
  candidates default, in order, to `/opt/rocm/lib/libhsa-runtime64.so.1`,
  `/usr/lib/x86_64-linux-gnu/libhsa-runtime64.so.1`,
  `/usr/lib64/libhsa-runtime64.so.1`, and `/usr/lib/libhsa-runtime64.so.1`.
  Supplying one or more library options replaces, rather than extends, the
  defaults and preserves the supplied order.
  Native backend discovery requires every candidate path to be absolute; a
  relative `--cuda-driver` or `--hsa-runtime` value reaches discovery as an
  explicit configuration error.
* PCI discovery is rooted at `/sys/bus/pci/devices`.
* PTX ISA is `74`, HSA code-object version is `6`, release identity is
  `auto-pinned-local-tools-and-benchmark-v3`, and the dependent FMA chain is
  `64`.

`required_tool` is `optional_tool` plus a missing-tool error that tells the
user to provide an exact absolute path. `optional_tool` stops at the first
candidate whose directory entry exists, then lets `PinnedTool::inspect` reject
wrong type, noncanonical, unreadable, or otherwise unpinnable paths. A missing
default candidate is skipped; other inspection errors stop the probe. The
globally optional `ld.lld` and `ptxas` become required by native identity
construction when an AMD or NVIDIA GPU is actually discovered, respectively.

## `current_native_inputs`: the runtime handoff used by preparation

`current_native_inputs` is `pub(crate)` rather than a command dispatch case.
`src/native_prepare.rs::with_current_native_preparation` calls it before every
current-machine training or inference preparation. It deliberately reuses the
same private state and identity rules as `recipe probe`.

The function first enforces bare metal, resolves the state root, and discovers
the current host. It then attempts to load `active-native-v1`:

* When a receipt exists, the current host must still contain the receipt's
  exact `host_memory_key`. The receipt's profile path, PCI root, scratch parent,
  backend libraries, and toolchain are reopened and checked. A missing RAM
  origin, changed canonical path, or changed digest tells the user to rerun
  `recipe probe`.
* When no receipt exists, the function reconstructs default seed/configuration
  inputs, discovers the current host's first RAM domain, computes the exact
  current cache identity through discovery-only inspection (no benchmark
  measurement or cache store), and derives the corresponding
  `state_root/profiles/measured-vSCHEMA-DIGEST.recipe-profile` path. This is a
  preparation fallback for locating the exact profile produced by a prior
  probe, not a replacement measurement path.

In either case it constructs a fresh `NativeGpuProbe` and returns
`CurrentNativeInputs { config, host, profile_identity, profile_path, native }`.
`with_current_native_preparation` maps a CLI error into
`NativePreparationError::LocalConfiguration`, then loads the profile through
`ExplicitPathProfileCache::load` using the returned identity. If no exact
profile exists, preparation reports `NativePreparationError::ProfileNotFound`;
cache decode and identity failures remain probe errors. The preparation layer
keeps the probe object in thread-local state after the first successful use and
rejects a later configuration change; the CLI function itself does not own that
cache. After the cache load, native preparation asks the probe for an exhaustive
current GPU inventory, resolves that inventory against the profile and host by
stable origins, rejects a missing or extra local calculation device, opens exact
CUDA/HSA bindings, and builds one target specification per measured target.
Those downstream identity, binding, toolchain, and target failures are typed by
`native_prepare` (see [`native_prepare.md`](native_prepare.md)); they do not
cause `current_native_inputs` to select another profile or silently omit a
device.

The binding reopen itself is exact: it rediscovers the host/GPU inventory,
partitions measured origins by the two backend identities, checks PCI display
surfaces, creates one CUDA context or HSA session per measured GPU, and for HSA
selects one unambiguous allocatable CPU agent by NUMA identity. Missing,
unexpected, duplicate, or ambiguous devices and resources are discovery errors;
there is no ordinal or single-device fallback.

## Active native receipt format and integrity checks

The receipt is a fixed v1 handoff named `active-native-v1` beneath
`recipe-next`. Its first line is the magic marker
`recipe-active-native-v1`. The next sixteen lines are tab-separated records in
the exact order listed by `RECEIPT_FIELDS`:

```text
profile
profile_schema
profile_digest
host_memory_key
pci_sysfs_root
scratch_parent
cuda_library
hsa_library
llvm_opt
llvm_llc
lld
ptxas
ptx_isa
hsa_code_object_version
release
fma_chain_length
```

Paths and labels are encoded as lowercase byte-hex. Digests are exactly 32
bytes, also lowercase hexadecimal. Numeric settings are decimal. A native file
pin is `HEX_PATH:64_HEX_DIGEST`; optional backend/linker/assembler pins use the
literal `none` when absent. `ActiveNativeReceipt::decode` requires the marker,
all sixteen fields in order, the canonical final newline, no trailing field,
valid lowercase encodings, and round-trip equality with `encode`, which rejects
noncanonical representations.

`capture` is stricter than serialization. It requires an absolute, canonical,
private, effective-user-owned measured profile; a real canonical PCI directory;
and a private scratch directory. Every backend in the profile must be either
`nvidia-cuda-driver` or `amd-rocr-hsa`. If the profile uses CUDA or HSA, the
first configured candidate that exists is canonicalized, verified as a regular
file, read, and pinned by `ArtifactDigest`. LLVM `opt` and `llc` are always
repinned; `lld` and `ptxas` are repinned only when configured. A tool that
changes path or digest during capture fails rather than producing a stale
receipt.

`write_active_native_receipt` uses `PendingReceipt` to create a same-directory
temporary file with a process id and atomic sequence suffix. It tries at most
64 names, uses `O_NOFOLLOW | O_CLOEXEC`, sets mode `0600`, writes and syncs the
bytes, renames over the target, syncs the directory, and rechecks that the final
file is a mode-0600 regular file owned by the effective user. `Drop` removes a
temporary file unless the rename committed.

`load_active_native_receipt` verifies the state directory, then checks that the
receipt is a non-symlink regular file, mode `0600`, owned by the effective user,
and no larger than 64 KiB. It opens with no-follow and close-on-exec flags,
compares device and inode with the earlier metadata to detect replacement while
opening, reads at most one byte beyond the limit, and decodes the canonical
format. Any discrepancy is a local-configuration error, not a silent reset to a
different profile.

The private codec helpers keep this format unambiguous: `encode_os` and
`encode_hex` emit every byte as two lowercase hexadecimal digits; `decode_hex`
rejects odd-length or non-lowercase input; `decode_path` rejects an empty byte
sequence; `decode_label` requires UTF-8 accepted by `Label`; digest fields must
decode to exactly 32 bytes; and `parse_decimal` accepts only the target integer
type's normal decimal parser. Required pins reject `none`, while optional pins
map `none` to `None`. Codec decoding only reconstructs values; absolute,
canonical, private, ownership, file-type, and digest checks happen in receipt
capture/reopen rather than being inferred from encoded bytes.

The supporting path helpers enforce these invariants throughout the handoff:

* private regular files must be absolute, non-symlink, canonical, mode-private,
  and owned by the effective user;
* canonical directories must be absolute, real directories with no symlink
  path components; private directories additionally require mode `0700`-style
  group/other exclusion and effective-user ownership;
* pinned native libraries and tools must remain absolute, regular,
  non-symlink files whose current digest equals the receipt digest;
* a changed tool or library rejects the handoff; digest changes explicitly
  report that the user must rerun `recipe probe`, while path/type/permission
  failures retain their specific inspection error.

## Private state root and bare-metal boundary

`private_state_root` is shared by probing, source compilation, and native
preparation. An absolute `XDG_CACHE_HOME` is used as-is. A missing or relative
value causes the CLI to canonicalize `HOME` and append `.cache`. The base is
created recursively with mode `0700`, canonicalized, and extended with
`recipe-next`. `ensure_private_directory` is called on every operational
subdirectory and rejects nonabsolute paths, symlinks, non-directories, any group
or other permission bits, ownership by another uid, and paths that canonicalize
to a different spelling. These checks prevent a cache, transformed source,
compiled binary, or receipt from being redirected through a shared or mutable
directory.

The environment selection is deterministic:

| environment | selected base | failure |
| --- | --- | --- |
| absolute `XDG_CACHE_HOME` | that path, canonicalized after creation if absent | cache-base creation or canonicalization error |
| relative `XDG_CACHE_HOME` | canonicalized `HOME` plus `.cache` | `canonicalize user home` or cache-base error |
| missing `XDG_CACHE_HOME` with `HOME` | canonicalized `HOME` plus `.cache` | cache-base error if creation/canonicalization fails |
| missing/relative `XDG_CACHE_HOME` and missing `HOME` | none | `neither an absolute XDG_CACHE_HOME nor HOME is available` |

The selected base is extended with `recipe-next`; `run` adds `run`, probing
adds `scratch` and (for the default profile path) `profiles`, and receipt
publication stays at the `recipe-next` root. Existing directories are checked
with `symlink_metadata`, ownership from `/proc/self`, and canonical path
equality before they are used.

`require_bare_metal` is called by `run_probe` and by `current_native_inputs`,
not by `recipe run` or `recipe convert`. Marker and namespace files that cannot
be read are not independently treated as positive matches; only an observed
marker, cgroup token, or nested `NSpid` list rejects the call. Running a source
program in a container is therefore possible as long as that program does not
request current native preparation; a probe or a native training/inference run
fails at the native boundary with the explicit container or PID-namespace
reason.

The positive matches and their exact returned forms are:

| observation | returned text |
| --- | --- |
| `/.dockerenv` or `/run/.containerenv` exists | `` `recipe probe` requires bare metal; container marker PATH exists `` |
| PID 1 cgroup contains `docker`, `containerd`, `kubepods`, `libpod`, or `lxc` (case-insensitive) | `` `recipe probe` requires bare metal; PID 1 cgroup reports MARKER `` |
| `NSpid:` has more than one numeric field after the label | `` `recipe probe` requires bare metal; the process is inside a PID namespace `` |

The cgroup and status reads are conditional: an unreadable file contributes no
positive match, while an observed marker stops the workflow before seed or
hardware discovery.

## Failure model at the CLI boundary

All command paths return `Result<(), String>`. The error strings preserve the
operation and path that failed, and `main` adds only the `recipe:` prefix plus a
failure exit code. The important failure classes are:

* dispatch errors: missing command, missing run source, invalid UTF-8 command,
  unknown command, probe option arity/repetition, conversion arity, or invalid
  extension pair;
* source-runner errors: source canonicalization/read/type checks, missing Recipe
  library, private-state creation, deterministic frontend rewrite, transformed
  source creation, Cargo output-root inspection, rustc startup/output, compiler
  diagnostics emission, child startup, pipe forwarding, SIGINT forwarding,
  child waiting, forwarder panic/join, temporary binary removal, and a nonzero
  child status;
* conversion errors: input open/metadata, bound construction, GGUF or OGDL
  stream conversion, output creation without overwrite, output flush, sync, or
  removal of a partial output;
* probe errors: container detection, seed-contract parsing, private state or
  scratch validation, host/RAM/storage/network discovery, tool and library
  pinning, native backend construction, discovery-only identity, cache read or
  atomic store, measured benchmark validation, receipt capture, receipt write,
  or final receipt integrity verification;
* runtime-handoff errors: a missing exact profile, stale profile identity,
  missing current RAM origin, changed canonical path, changed native digest,
  unsupported backend identity, malformed receipt, or insecure receipt file.

The first observable error at each boundary is deliberately specific:

| boundary | representative returned text | cleanup or state rule |
| --- | --- | --- |
| dispatch | `missing command`, a missing-run-source message, or an unknown-command/option/extension message, each with `USAGE` where the source includes it | no workflow state is touched |
| source admission | `read training source PATH: ...` or `training source PATH is not a regular file` | no compiler or child is started |
| source compilation | `start rustc for PATH: ...`, an output-write error, or `rustc failed for PATH with STATUS` | transformed source drops; after successful diagnostic emission a failed compiler's binary removal is best effort |
| live child | `run PATH: ...`, capture/forward/wait/join text, or `training source PATH failed: ...` | forwarder failures perform their local kill/wait cleanup; the compiled binary is removed by `run_source` |
| conversion | `open conversion input ...`, `create conversion output ...`, stream/flush/sync text, or a combined partial-output cleanup error | destination uses `create_new`; failed writes remove the partial path when possible |
| probe | the underlying seed, discovery, cache, native, receipt, or integrity detail | a measured profile can remain if a later receipt step fails; no success lines are printed |
| current native handoff | `local native configuration failed: ...`, profile-not-found, probe, or identity-mismatch errors after wrapping | no alternate profile, library, or tool is selected |

There are no alternate profile or receipt paths, newest-profile guesses, retry
compilations, or silent substitutions in these paths. (The source runner's
deliberate newest-`librecipe-*.rlib` selection is library discovery, not profile
state.) A valid command either reaches its real downstream operation and
reports its resulting state, or returns the first specific failure that
prevents the next safe operation.

## End-to-end flow summary

At the process boundary, each workflow has one authoritative completion signal:

| workflow | authoritative result | durable mutation | success output |
| --- | --- | --- | --- |
| `run` | the compiled child's `ExitStatus` after both output forwarders finish | one private compiled binary is created, then removed; a transformed source is sibling-temporary and removed on drop; the child may independently write declared artifacts | the child and compiler streams, with no parent summary on success |
| `probe` | the validated `MeasuredProfile` plus a successfully installed `active-native-v1` receipt | identity-named profile cache (possibly reused) and atomically replaced receipt | eight `key=value` lines describing the profile and identities |
| `convert` | the synchronized destination file created with `create_new` | exactly the requested output path on success | `Converted INPUT -> OUTPUT` |

An error before a workflow reaches its authoritative result is returned as a
`String`; `cli::main` writes one `recipe: ...` line to standard error and
returns `ExitCode::FAILURE`. A child may have emitted arbitrary useful output
before its non-success status, and a probe may have stored a profile before
receipt capture fails, but neither partial outcome is reported as command
success.

The three production paths can be read as one state diagram:

```text
argv -> main -> run
  ├─ run FILE.rs
  │    -> canonical/read source
  │    -> locate rlib and private run state
  │    -> deterministic Recipe lowering
  │    -> one rustc invocation
  │    -> mapped diagnostics
  │    -> live child stdout/stderr and one-shot SIGINT forwarding
  │    -> remove binary -> child-status result
  ├─ probe [OPTIONS]
  │    -> parse pair options
  │    -> bare-metal check and seed
  │    -> host/native discovery
  │    -> exact identity and profile cache load or bounded measurement
  │    -> pin native inputs and atomically write active-native-v1
  │    -> profile and identity summary
  └─ convert INPUT OUTPUT
       -> exact arity and extension direction
       -> bounded streaming GGUF/OGDL conversion
       -> create-new mode-0600 output and sync
       -> remove partial output on failure
       -> conversion summary
```

The source runner is the user-visible execution terminal. The probe is the
identity and measurement terminal. The converter is a private, overwrite-safe
file transformation terminal. `current_native_inputs` joins the probe and
runtime paths without adding another command or another source of hardware
truth.

There are no test-only entrypoints in `src/cli.rs`. Any acceptance of these
contracts must invoke the installed `recipe` binary, traverse `main -> run`,
and observe the resulting files, streams, child status, profile/receipt state,
or converted destination through the same production boundary.
