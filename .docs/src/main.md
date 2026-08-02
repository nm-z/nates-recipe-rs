# `src/main.rs`: the `recipe` executable

## Scope and purpose

The root package declares one binary named `recipe` in `Cargo.toml`, with
`src/main.rs` as its binary source. The file is intentionally a one-line
process adapter:

```rust
fn main() -> std::process::ExitCode { recipe::cli::main() }
```

The executable does not parse declarations, discover hardware, schedule a
graph, or execute a kernel itself. It enters the library crate's public
`recipe::cli` module. `src/facade.rs` defines that module with
`include!("cli.rs")`, so the real command implementation is `src/cli.rs` and
the binary and library use the same compiled code. The CLI is the boundary for
three process workflows:

```text
OS argv
  -> recipe::cli::main
  -> cli::run
       -> run_source       (`recipe run`)
       -> run_probe        (`recipe probe`)
       -> run_model_conversion (`recipe convert`)
```

Each branch returns `Result<(), String>`. The only process-level translation is
performed by `cli::main`:

- `Ok(())` returns `ExitCode::SUCCESS`.
- `Err(error)` writes `recipe: {error}` to standard error and returns
  `ExitCode::FAILURE`.

There is no direct `process::exit` call. Child-process status, compiler status,
filesystem failures, probe failures, conversion failures, and downstream
Recipe errors therefore all terminate through the same error line and failure
exit code when they reach this boundary.

## Usage and dispatch

The usage string is one fixed value:

```text
Usage:
	recipe run FILE.rs	[ARGS...]
	recipe probe		[OPTIONS]
	recipe convert INPUT OUTPUT
```

`main` collects `env::args_os().skip(1)` into `Vec<OsString>`. Arguments are
kept as operating-system strings until a branch needs a UTF-8 command or option
name. Source paths and child arguments can therefore be non-UTF-8; the command
name itself must be UTF-8 because dispatch uses `OsString::to_str()`.

`run` applies this decision table in order:

| argv shape | action |
| --- | --- |
| no command | error `missing command`, followed by usage |
| first argument `-h` or `--help` | print usage to stdout, succeed |
| UTF-8 command `run` with no source | error that a Rust source file is required, followed by usage |
| `run -h` or `run --help` as the source argument | print usage to stdout, succeed; additional arguments are ignored |
| `run FILE.rs [ARGS...]` | compile and execute the source with the remaining arguments |
| UTF-8 command `probe` with exactly one help argument | print usage to stdout, succeed |
| `probe [OPTIONS]` | parse options as option/value pairs, then probe |
| UTF-8 command `convert` with exactly one help argument | print usage to stdout, succeed |
| `convert INPUT OUTPUT` | convert in the direction selected by the two extensions |
| any other UTF-8 command | error naming the command and the three valid commands, followed by usage |
| non-UTF-8 command | error `commands must be valid UTF-8` |

Help is not a general option parser. For example, `convert --help extra` is
treated as two conversion paths and fails extension validation, while
`run --help extra` succeeds because the second argument is recognized as the
source-position help sentinel before the extra argument is considered.

### Probe option parsing

`parse_probe_options` walks the option slice two `OsString`s at a time. Every
option must have a following path value. The option spelling must be UTF-8;
the value is retained as a `PathBuf` and need not be UTF-8. Recognized options
are:

| option | destination | multiplicity |
| --- | --- | --- |
| `--contract PATH` | seed contract file | once |
| `--profile PATH` | measured profile output/cache path | once |
| `--cuda-driver PATH` | CUDA Driver candidate list | repeatable |
| `--hsa-runtime PATH` | ROCr/HSA runtime candidate list | repeatable |
| `--llvm-opt PATH` | required LLVM `opt` tool | once |
| `--llvm-llc PATH` | required LLVM `llc` tool | once |
| `--lld PATH` | optional LLVM linker | once |
| `--ptxas PATH` | optional NVIDIA assembler | once |

An unknown option fails after its value has been consumed. A duplicate
single-valued option fails with `{option} may be supplied only once`. A missing
value fails with `option {option} requires a path argument`. No probe option
has a short spelling or an implicit boolean form.

## `recipe run`: compile and run a Rust declaration

`run_source` is a source runner, not a second Recipe execution engine. It
creates a short-lived native executable from the supplied source, links the
already-built root `recipe` library, and lets that child process exercise the
public declaration API.

### Source admission

1. The requested `OsStr` is converted to a `Path` and canonicalized. Failure
   is reported as `read training source REQUESTED: ...`.
2. The canonical path must be a regular file. A directory or other file type
   fails with `training source REQUESTED is not a regular file`.
3. The canonical file must have a parent directory, which becomes the child
   process's current directory.
4. The source is read as UTF-8 text. Invalid UTF-8 and other read failures are
   reported against the canonical path.

No source contents are executed before these checks succeed.

### Library and temporary-path selection

`locate_recipe_library` searches for a root-library `librecipe*.rlib` in this
order:

1. the directory containing the current executable;
2. its profile directory when that directory is named `deps`;
3. `target/debug` and `target/release` below the current working directory;
4. `/usr/lib/recipe`.

Within a root, the newest regular `librecipe-*.rlib` in `deps` is preferred.
The search then falls back to the newest matching library under Cargo's
`build/recipe/*/out` directories and finally to `librecipe.rlib` directly in
the root. Cargo build-output discovery is bounded at 4096 `out` directories.
If no library is found, the source run fails with the fixed message that
`librecipe.rlib` is unavailable.

The runner obtains the private Recipe state root, ensures its private `run`
directory, and allocates a process-and-sequence name:

```text
<state-root>/run/recipe-run-<parent-pid>-<sequence>
```

The sequence is process-local and monotonically allocated. The child binary is
removed after every successful compile and run path, and also removed on a
compiler failure when possible.

### Source frontend lowering

Before invoking `rustc`, `run_source` calls
`source_frontend::lower_recipe_source` once. This pass is syntax
classification, not compiler-driven retry logic. If tokenization or full-file
parsing fails, it returns `Ok(None)` and the original source is compiled. If
valid Recipe syntax is recognized, the generated source is written beside the
original as a private mode-0600 file opened with `O_CLOEXEC | O_NOFOLLOW` and
`create_new`. Its `Drop` implementation removes that file.

Receiver classification follows explicit `recipe` chains and local bindings
whose types are `Data`, `Model`, `Train`, or `Infer`. The recognized edits are:

- `recipe.data()` becomes `recipe.data(())`, selecting the unit
  `IntoDataSources` implementation.
- A model `.residual(ARG1, ARG2, ...)` call becomes one array argument.
- A model `.grad(clip: EXPR)` call becomes
  `.grad(::recipe::clip(EXPR))`.
- A train `.save(MODEL, KERNEL)` call becomes
  `.__recipe_save_pair(MODEL, KERNEL)`.
- A train `.resume(MODEL, KERNEL)` call becomes
  `.__recipe_resume_pair(MODEL, KERNEL)`.
- A train `.run(MODEL, DATA)` call becomes
  `.__recipe_run_with(MODEL, DATA)`.

Named gradient fields must contain exactly one `clip` field. Duplicate or
unknown fields, malformed fields, overlapping edits, and invalid source
boundaries return a rendered source diagnostic rather than falling through to a
second compiler attempt. If no edit is needed, the original source path is
compiled.

### `rustc` invocation and diagnostics

`compile_run_source` selects `RUSTC` from the environment, falling back to
`rustc`, and invokes it with:

```text
SOURCE
--crate-name recipe_run
--crate-type bin
--edition 2024
-Dunused_must_use
--error-format=json
-L LIBRARY_ROOT
[ -L LIBRARY_ROOT/deps ]
[ -L each Cargo build output root ]
[ --remap-path-prefix GENERATED=ORIGINAL ]
--extern recipe=LIBRARY_PATH
-o BINARY_PATH
```

`Command::output` captures compiler stdout and stderr. Stdout is written to
the runner's stdout unchanged. Stderr is parsed as newline-delimited JSON;
Rust compiler diagnostic records are rendered into concise level/code/message
lines with source spans, while non-diagnostic lines are retained as raw text.
When a generated source was used, diagnostic paths and byte spans are mapped
back to the original source. A non-successful compiler status removes the
binary (best effort) and returns `rustc failed for SOURCE with STATUS`.

### Child execution, output, and interruption

On a successful compile, `run_compiled_source_live`:

1. installs the process-scoped `SigintGuard`, which records SIGINT in one
   atomic flag and restores the previous handler on drop;
2. starts the compiled binary with the user-provided raw arguments and the
   source directory as current directory;
3. pipes child stdout and stderr;
4. starts one named forwarder thread per stream;
5. forwards each read chunk immediately to the matching parent stream and
   flushes it;
6. retains only the last 64 KiB of each stream for failure diagnosis, marking
   a tail as truncated whenever older bytes are discarded; and
7. polls `try_wait` every 10 ms until the child exits.

If the parent receives SIGINT while the child is alive, it forwards exactly
one SIGINT to the child. An already-exited child (`ESRCH`) is ignored; other
forwarding or wait failures kill and reap the child before returning an error.
Failure to create either forwarder also kills and reaps the child, and the
other forwarder is joined before the error is returned. A panicking forwarder
is reported with its string payload when available.

After the child status is known, both forwarders are joined. The compiled
binary is removed before the child result is interpreted. A cleanup failure is
itself fatal, even if the child returned success. A non-successful child status
is translated by `run_status_message`:

- a UTF-8 stderr tail containing `memory allocation of DIGITS` is reported as
  `out of memory while requesting DIGITS bytes (DECIMAL GB)`;
- otherwise a terminating signal is reported as `terminated by signal N`;
- otherwise an exit code is reported as `exited with code N`;
- otherwise the complete status display is used.

The message is prefixed with `training source SOURCE failed:` and notes which
captured tails were truncated. The parent never substitutes a success status
for a failed child and never hides the child's streamed output.

### What the child owns downstream

The child links the same `recipe` library that contains the public facade. The
facade builders are immutable declarations and do not perform I/O:

- `recipe.data(...)` records source paths and preprocessing policy in a
  thread-local `RecipeSequence`, replacing the previous data and clearing the
  previous model.
- `recipe.model()` records a model declaration in that sequence.
- `recipe.train()` and `recipe.infer()` create static policies.
- `Train::run` consumes the immediately preceding data and model. Missing
  declarations, invalid data/model/policy, preparation failures, native
  identity mismatches, execution faults, and artifact-write failures become
  typed `TrainingError`s.
- `Infer::evaluate` consumes the same declaration sequence, requires target-free
  data and a loaded `.ogdl` or `.gguf` model, executes native inference, and
  writes tab-separated report and prediction rows to stdout after native
  teardown.

Dense training prepares a measured native system, executes the immutable
`init -> loop -> exit` lifecycle, and writes declared `.ogdl`, `.cubin`, or
`.hsaco` artifacts only after successful completion. KNN and Bayesian training
are reference-model preparation paths and do not claim a native optimizer loop.
The source runner treats the child's process status and streamed output as its
authoritative observation; it does not inspect or recreate the child's model,
graph, or executor state.

## `recipe probe`: discover, measure, and publish the current native state

`run_probe` is the producer of the measured profile and the active native
receipt consumed by later training and inference preparation. It first calls
`require_bare_metal` and fails if any of the following is observed:

- `/.dockerenv` or `/run/.containerenv` exists;
- PID 1's cgroup text names `docker`, `containerd`, `kubepods`, `libpod`, or
  `lxc`; or
- `/proc/self/status` shows more than one PID in the `NSpid:` line.

The check is deliberately fail-closed. A probe is not downgraded to a partial
or simulated inventory.

### Seed, private state, and native configuration

The seed is read from `--contract PATH` when supplied, otherwise parsed from
the embedded `topology/contract.toml`. The seed parser requires schema 1,
kind `probe-seed-estimates`, all discovery and benchmark gates enabled,
complete cache invalidation facets, and at least one bidirectional asynchronous
transport. Its estimates bound the first benchmark buffers only; they are not
accepted as measured scheduling values.

`private_state_root` selects an absolute `XDG_CACHE_HOME`, or the canonical
`HOME/.cache` when that variable is missing or relative, then uses
`<cache>/recipe-next`. Every created or reused directory must be absolute,
canonical, a real directory, mode 0700-compatible (no group or other bits),
and owned by the effective user. The probe creates private `scratch` and
`profiles` directories below this root.

Host discovery runs with the state root as its benchmark root. The first
discovered RAM domain supplies `host_memory_key`; an empty RAM inventory is a
hard error. `native_config` then pins:

- required LLVM `opt` and `llc` tools from explicit paths or a fixed candidate
  list;
- optional `ld.lld` and `ptxas` tools from explicit paths or fixed candidates;
- configured CUDA Driver and ROCr/HSA library candidates, or fixed defaults;
- `/sys/bus/pci/devices` as the PCI sysfs root;
- PTX ISA 74 and HSA code-object version 6;
- release label `auto-pinned-local-tools-and-benchmark-v3`;
- the private scratch parent; and
- a dependent FMA benchmark chain length of 64.

Explicit tools are inspected and hashed. An explicit path that cannot be
inspected is an error. A required tool missing from all candidates is an error;
optional tools and backend libraries may be absent until a discovered device
requires them.

### Discovery, cache identity, and measured profile

`NativeGpuProbe::new` validates the configuration and opens both native backend
adapters. `ProbeEngine` combines that GPU discovery with local host benchmarks,
with no peer sessions (`peers = []`). It first computes the exact current cache
identity from the seed, host inventory, exhaustive GPU inventory, and stable
identities. The default profile path is:

```text
<state-root>/profiles/measured-v<schema>-<lowercase-digest>.recipe-profile
```

`ExplicitPathProfileCache` accepts only an absolute file path in a canonical,
private, effective-user-owned parent. An existing file must be a regular
non-symlink mode-0600 file, remain the same inode while opened, fit the bounded
profile size, decode successfully, and contain exactly the requested identity.
`load_or_probe_and_store` returns that profile as `validated-cache`; otherwise
it performs fresh bounded measurements and atomically installs a new profile as
`fresh-measurement`. It never selects an arbitrary newest profile and never
overwrites a different profile at an existing path.

The engine measures every discovered RAM and storage domain and every GPU,
validates the results, builds topology and discovery identities, validates both
objects, and returns one `MeasuredProfile`. A profile with no exhaustive GPU
enumeration, no GPU calculation device, missing measured properties, invalid
links, or invalid scheduling properties fails closed.

### Active native receipt

After obtaining the profile, `ActiveNativeReceipt::capture` validates the
profile path, PCI root, and scratch directory. It rejects any measured backend
other than `nvidia-cuda-driver` or `amd-rocr-hsa`. For every backend actually
used by a measured device it selects and pins one regular target library. It
also pins required LLVM tools and any configured optional linker or assembler,
recording each canonical path and SHA-256 digest.

The receipt is a canonical UTF-8 text file at:

```text
<state-root>/active-native-v1
```

It begins with `recipe-active-native-v1`, followed by exactly these tab-separated
fields in this order:

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

Paths and labels are byte-hex encoded, digests are 64 lowercase hexadecimal
digits, optional pins use `none`, and scalar settings use decimal integers.
Receipt publication writes a unique mode-0600 temporary with `create_new`,
`O_NOFOLLOW`, and `O_CLOEXEC`, syncs it, renames it into place, syncs the state
directory, and verifies regular-file type, mode, and ownership. Failed writes
remove an uncommitted temporary.

The receipt is authoritative runtime handoff state. `current_native_inputs`,
called by `with_current_native_preparation` during training and inference,
reopens and validates it before creating a `NativeGpuProbe`:

1. It rechecks bare-metal status and current host discovery.
2. If the receipt exists, it requires its recorded RAM key in the current host,
   reopens the exact profile path, and re-inspects every pinned path and digest.
3. If no receipt exists, it derives the default configuration and identity-named
   profile path from current discovery. This is only configuration derivation;
   it does not invent a profile. The subsequent exact profile-cache load remains
   authoritative and fails if the profile is absent or stale.
4. It returns `NativeProbeConfig`, `HostInventory`, `CacheIdentity`, profile
   path, and the native probe. `with_current_native_preparation` then loads the
   exact measured profile and resolves all live RAM, storage, and GPU origins by
   stable keys. Missing, extra, or changed devices and changed tool/library
   identities fail closed and require another `recipe probe`.

### Probe stdout

After the receipt is installed, the command prints one line for each field of
the measured result:

```text
profile=ABSOLUTE_PROFILE_PATH
source=validated-cache|fresh-measurement
cache_identity=LOWERCASE_DIGEST
topology_identity=LOWERCASE_DIGEST
discovery_identity=LOWERCASE_DIGEST
machines=COUNT
devices=COUNT
directed_links=COUNT
```

These lines describe the profile that was loaded or freshly measured. They are
not a substitute for the profile file or active receipt used by preparation.

## `recipe convert`: bounded GGUF and structural OGDL conversion

`run_model_conversion` requires exactly two paths. Extensions are compared as
the lowercase `OsStr` values `gguf` and `ogdl`:

- `.gguf -> .ogdl` opens the binary source, bounds conversion by the source
  length, and streams `gguf_to_structural_ogdl_stream` into the output.
- `.ogdl -> .gguf` opens the structural source, reads its declared GGUF byte
  length, constructs corresponding bounds, and streams
  `structural_ogdl_to_gguf_stream` into the output.

Any other extension pair fails before an output is opened. Input open or
metadata failures name the input path. Structural parse, GGUF parse, and stream
errors are wrapped with the direction and input path.

Conversion limits are derived from source and declared-output lengths, with a
minimum bound of one byte and fixed bounded metadata/array/depth parameters.
The converter retains bounded descriptors and chunks rather than loading an
unbounded complete image.

The output helper opens the destination with `read`, `write`, `create_new`, and
mode 0600. Existing files are never overwritten. The writer is flushed and the
file is synchronized before success. If conversion, flushing, or synchronization
fails, the partial output is removed; failure to remove it is appended to the
reported error. Success prints exactly:

```text
Converted INPUT -> OUTPUT
```

The conversion command has no connection to the active native receipt and does
not probe hardware.

## State, outputs, and termination summary

The executable's durable state has three distinct owners:

| state | producer | consumer | authority |
| --- | --- | --- | --- |
| compiled child binary | `recipe run` | the one child process | child exit status and streamed output |
| measured profile cache | `recipe probe` | native preparation | identity-keyed validated profile |
| `active-native-v1` receipt | `recipe probe` | `current_native_inputs` | canonical pinned paths, identities, and scalar settings |
| conversion destination | `recipe convert` | the caller | newly created synchronized file |

The parent does not retain the compiled child binary, does not choose a
different profile when the active receipt is invalid, does not overwrite a
conversion destination, and does not convert a failed child or failed stream
into success. Standard output from help, probe, conversion, compiler stdout,
child stdout, training metrics, inference rows, and child stderr remains on its
respective stream. Parent-generated failures are the final `recipe: ...` line
and a failure exit code.

The complete process-level lifecycle is therefore:

```text
parse OS argv
  -> validate command shape and option/path forms
  -> perform exactly one selected workflow
  -> publish only that workflow's authoritative result
  -> clean temporary resources
  -> return success or print one prefixed error and fail
```
