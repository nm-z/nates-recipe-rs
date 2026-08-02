<!--
Intent: describe the complete build-script boundary for recipe-native-probe.
The script does not build a native backend or discover hardware. It computes a
deterministic identity for the repository inputs and compiler environment that
must be present when native probe identities are compiled.
-->

# `native-probe/build.rs`

[`native-probe/build.rs`](../build.rs) is the package build script for
`recipe-native-probe`. Its only successful product is a Cargo compile-time
environment variable, `RECIPE_NATIVE_PROBE_SOURCE_DIGEST`. The package uses
that value from [`src/identity.rs`](../src/identity.rs) when it creates the
toolchain identity carried by every CUDA or HSA GPU descriptor.

The script is an identity and invalidation boundary, not a runtime probe. It
does not load CUDA or ROCr, inspect PCI or sysfs, read `NativeProbeConfig`, run
LLVM, assemble a kernel, or write a generated source file. Hardware discovery,
native library hashing, and native compiler-tool validation happen in the
runtime crate after this build step.

## Intent and parseable contract

The current behavior can be represented as follows:

```text
native_probe_build:
  manifest:
    source: env_os("CARGO_MANIFEST_DIR")
    missing: fail("CARGO_MANIFEST_DIR is unavailable")

  file_inputs:
    rust_roots:
      - domain: recipe-native-probe
        path: manifest/src
      - domain: recipe-core
        path: manifest/../core/src
      - domain: recipe-executor
        path: manifest/../executor/src
      - domain: recipe-host
        path: manifest/../host/src
      - domain: recipe-kernel
        path: manifest/../kernel/src
      - domain: recipe-language
        path: manifest/../language/src
      - domain: recipe-native-executor
        path: manifest/../native-executor/src
      - domain: recipe-planner
        path: manifest/../planner/src
      - domain: recipe-primitives
        path: manifest/../primitives/src
      - domain: recipe-scheduler
        path: manifest/../scheduler/src
      - domain: recipe-cuda
        path: manifest/../cuda/src
      - domain: recipe-hsa
        path: manifest/../hsa/src
      - domain: recipe-probe
        path: manifest/../probe/src
      rule: recursively retain non-directory entries whose final extension is
        exactly "rs"; each retained logical name is "<domain>/<path relative
        to root>"
    explicit_files:
      - name: Cargo.lock
        path: manifest/../Cargo.lock
      - name: Cargo.toml
        path: manifest/../Cargo.toml
      - name: native-probe/Cargo.toml
        path: manifest/Cargo.toml
      - name: core/Cargo.toml
        path: manifest/../core/Cargo.toml
      - name: executor/Cargo.toml
        path: manifest/../executor/Cargo.toml
      - name: host/Cargo.toml
        path: manifest/../host/Cargo.toml
      - name: kernel/Cargo.toml
        path: manifest/../kernel/Cargo.toml
      - name: language/Cargo.toml
        path: manifest/../language/Cargo.toml
      - name: native-executor/Cargo.toml
        path: manifest/../native-executor/Cargo.toml
      - name: planner/Cargo.toml
        path: manifest/../planner/Cargo.toml
      - name: primitives/Cargo.toml
        path: manifest/../primitives/Cargo.toml
      - name: scheduler/Cargo.toml
        path: manifest/../scheduler/Cargo.toml
      - name: cuda/Cargo.toml
        path: manifest/../cuda/Cargo.toml
      - name: hsa/Cargo.toml
        path: manifest/../hsa/Cargo.toml
      - name: probe/Cargo.toml
        path: manifest/../probe/Cargo.toml
      - name: native-probe/build.rs
        path: manifest/build.rs
    ordering: sort every (logical_name, path) pair lexicographically by
      logical_name before any directive or hash is emitted

  environment_inputs:
    - HOST
    - TARGET
    - OPT_LEVEL
    - DEBUG
    - PROFILE
    - CARGO_CFG_TARGET_ARCH
    - CARGO_CFG_TARGET_OS
    - CARGO_CFG_TARGET_ENV
    - CARGO_CFG_TARGET_FEATURE
    - CARGO_ENCODED_RUSTFLAGS
    - RUSTFLAGS
    missing_value: hash_field(empty_bytes)
    value_encoding: OsStr::as_encoded_bytes()

  rustc_input:
    rerun_variable: RUSTC
    executable: env_os("RUSTC")
    command: ["RUSTC", "-Vv"]
    success_required: true
    hashed_fields: ["RUSTC", rustc_path_bytes, stdout, stderr]

  digest:
    initial_bytes: "recipe-native-probe-build-v3"
    field_encoding: little_endian_u64_byte_length + raw_bytes
    fields: ordered_file_name_and_file_bytes, ordered_environment_name_and_value,
      rustc_identity_fields
    output: lowercase hexadecimal SHA-256, exactly 64 ASCII characters

  cargo_output:
    - cargo:rerun-if-changed=<each ordered file path>
    - cargo:rerun-if-env-changed=<each environment name>
    - cargo:rustc-env=RECIPE_NATIVE_PROBE_SOURCE_DIGEST=<digest>
```

The `native_probe_build` map describes implementation behavior, not a second
configuration format. The arrays in the Rust source are authoritative.

## Package boundary and build dependencies

`native-probe/Cargo.toml` declares the package as `recipe-native-probe` and
uses edition 2024. The build script is compiled with the `sha2 = "0.10"`
build dependency. The same crate is also a normal runtime dependency because
`src/identity.rs` computes identities for discovered backends. The package's
runtime path dependencies are `recipe-core`, `recipe-cuda`, `recipe-hsa`,
`recipe-host`, `recipe-kernel`, `recipe-native-executor`, and `recipe-probe`.

The build script additionally tracks source and manifests for
`recipe-executor`, `recipe-language`, `recipe-planner`,
`recipe-primitives`, and `recipe-scheduler`. Those crates are not all direct
entries in the package's runtime dependency table, but their source is part of
the native-probe identity boundary. A change to any tracked source or manifest
therefore changes the generated digest after the script reruns.

The script does not parse or hash every workspace file. In particular, it does
not directly include `math`, `ingest`, `training`, `transport`, `remote`,
`cluster`, `prepare`, `ops`, `program`, `audit`, or their manifests. It also
does not directly list `.cargo/config.toml`, `rust-toolchain.toml`,
`rustfmt.toml`, `clippy.toml`, `topology/contract.toml`, documentation,
examples, target output, non-Rust files below a source root, or native backend
libraries and compiler tools. Some of those values can affect Cargo-provided
environment variables or the `RUSTC -Vv` result, but they are not independent
file inputs to this script.

## Input collection

### Manifest root

`main` reads `CARGO_MANIFEST_DIR` with `std::env::var_os` and constructs a
`PathBuf`. Cargo normally supplies the absolute package directory. The value is
used as the base for every path above, but its spelling is not itself hashed.
The digest uses logical names and file bytes, not the absolute path printed in
Cargo directives.

### Recursive Rust collection

`collect_rust_files(domain, root, directory, output)` performs a depth-first
walk with these exact rules:

1. `fs::read_dir(directory)` is collected into a vector. Directory entries are
   sorted by their `file_name` before processing, so filesystem enumeration
   order cannot reorder the collected names.
2. An entry for which `path.is_dir()` is true is recursed into. This check
   follows directory symlinks; there is no symlink rejection or cycle guard.
3. Otherwise, an entry is retained only when
   `path.extension().is_some_and(|extension| extension == "rs")` is true. The
   extension comparison is case-sensitive, so `x.rs` is included while
   `x.RS` is not.
4. The retained path is made relative to the root with
   `strip_prefix(root)`. Its logical name is
   `format!("{domain}/{}", relative.display())`; the absolute path remains
   the path later passed to `fs::read` and Cargo's rerun directive.

The walk does not hash directory names, directory metadata, file metadata,
permissions, timestamps, symlink metadata, or non-`.rs` files. A retained path
is read as a complete byte sequence, so a file symlink is hashed through its
target and a directory symlink is traversed. A newly added Rust file or directory is
not a listed Cargo path until the build script runs again, so the explicit
rerun list is a snapshot of the files found during that invocation. Existing
listed files are monitored individually, not by their containing directory.

### Explicit files and ordering

After walking all roots, `main` appends `native-probe/build.rs` and the 15
manifest/lock entries shown in the contract map. It then sorts the complete
vector by the logical name only. The logical name, rather than the absolute
filesystem path, is the hash key. This makes the digest independent of a
workspace checkout's root path when the logical file set, file bytes, and
environment are otherwise equal.

At the current checkout there are 99 `.rs` files below the 13 roots. Together
with `native-probe/build.rs` and the 15 explicit manifest/lock entries, the
script emits 115 `cargo:rerun-if-changed` lines. The count is not a contract:
it changes when tracked Rust files are added or removed.

## Digest construction

### Field encoding

`hash_field` writes one value as the eight-byte little-endian representation of
`value.len()` followed by the value bytes. It is used for logical names,
source/config bytes, environment names and values, the `RUSTC` path, and both
`rustc -Vv` output streams. Length prefixes prevent adjacent fields from being
ambiguous. The initial domain marker is the one exception: the literal
`recipe-native-probe-build-v3` is written directly without a length prefix.

The complete hash stream is:

```text
SHA256(
  raw("recipe-native-probe-build-v3")
  || for (name, path) in files sorted by name:
       field(UTF8(name)) || field(fs::read(path))
  || for name in [HOST, TARGET, OPT_LEVEL, DEBUG, PROFILE,
                  CARGO_CFG_TARGET_ARCH, CARGO_CFG_TARGET_OS,
                  CARGO_CFG_TARGET_ENV, CARGO_CFG_TARGET_FEATURE,
                  CARGO_ENCODED_RUSTFLAGS, RUSTFLAGS]:
       field(UTF8(name)) || field(value_os(name) or empty_bytes)
  || field(UTF8("RUSTC"))
  || field(raw_os_bytes(value_os("RUSTC")))
  || field(rustc_stdout)
  || field(rustc_stderr)
)
```

`OsStr::as_encoded_bytes()` is used for environment and executable-path
values. The bytes are the platform's encoded `OsStr` representation, not a
lossy Unicode conversion. Environment names and the logical file names are
ordinary UTF-8 Rust string bytes.

The `RUSTC` name is intentionally hashed twice: once in the general
environment loop, where its current value is included as an environment field,
and once in `hash_rustc_identity`, where the executable path and `-Vv` output
are added. `hash_rustc_identity` also emits a second
`cargo:rerun-if-env-changed=RUSTC` line. The duplicate directive is harmless;
the duplicate hash field is part of the current digest algorithm.

After all fields are written, `Sha256::finalize` returns 32 bytes. Each byte is
formatted with two lowercase hexadecimal digits and concatenated, producing a
64-character ASCII string. No digest file, plan, source file, or runtime
environment mutation is created.

### Environment values

The general environment list contains Cargo's host, target, profile, target
configuration, and Rust-flags values:

| Name | Build-time meaning in this boundary |
| --- | --- |
| `HOST` | Host triple supplied to the build script. |
| `TARGET` | Compilation target triple. |
| `OPT_LEVEL` | Cargo optimization level. |
| `DEBUG` | Cargo debug-information setting. |
| `PROFILE` | Cargo profile name. |
| `CARGO_CFG_TARGET_ARCH` | Target architecture cfg value. |
| `CARGO_CFG_TARGET_OS` | Target operating-system cfg value. |
| `CARGO_CFG_TARGET_ENV` | Target environment cfg value. |
| `CARGO_CFG_TARGET_FEATURE` | Target feature list. |
| `CARGO_ENCODED_RUSTFLAGS` | Cargo's encoded compiler flags. |
| `RUSTFLAGS` | User/compiler Rust flags when supplied separately. |

Each name is announced with `cargo:rerun-if-env-changed`. An unset value is
not an error and hashes as an empty field. A present value, including an empty
present value, is read with `var_os` and hashed as its encoded bytes.

`hash_rustc_identity` handles the compiler executable separately. It requires
`RUSTC`, invokes that exact executable with `-Vv`, and hashes the executable
path plus both captured output streams. The output is not printed by the build
script. A successful invocation can include compiler version, host, commit,
and other rustc identity lines in the digest without putting those lines in
the Cargo build log.

## Cargo rerun policy and output

For every sorted file pair, `main` prints:

```text
cargo:rerun-if-changed=<path.display()>
```

The path is the collected absolute-or-relative path, as supplied by
`CARGO_MANIFEST_DIR`, not the logical hash name. For every general environment
name and again for `RUSTC`, it prints:

```text
cargo:rerun-if-env-changed=<name>
```

These directives make Cargo rerun the script when one of the listed existing
files or one of the listed environment values changes. Because the script
emits explicit `rerun-if-changed` directives, unlisted files and directories
are not part of this script's explicit watch set. In particular, adding a new
`.rs` file under a tracked root does not itself produce a new directive until
some already watched input causes another invocation. Removing or editing an
already listed file is watched, while changing an unlisted file such as a
documentation page is not.

The final line is:

```text
cargo:rustc-env=RECIPE_NATIVE_PROBE_SOURCE_DIGEST=<64 lowercase hex digits>
```

Cargo passes that value to the `recipe-native-probe` package's rustc
invocation. It is consumed as a compile-time value by `env!` in the package,
not a generated artifact available to users.

## `rustc` identity and failure contract

The build function returns `Result<(), Box<dyn Error>>`; every `?` stops the
script and reports the underlying error to Cargo. The failure boundaries are:

| Operation | Failure and consequence |
| --- | --- |
| Read `CARGO_MANIFEST_DIR` | Missing value returns `CARGO_MANIFEST_DIR is unavailable`; the package does not proceed. |
| Enumerate a source root or nested directory | `fs::read_dir` or an entry error is propagated; no partial digest is emitted. |
| Recurse and strip a collected path | `strip_prefix` failure is propagated. |
| Read any listed source, manifest, lockfile, or `build.rs` | `fs::read` failure is propagated; unreadable or missing tracked input aborts the build. |
| Read `RUSTC` | Missing value returns `RUSTC is unavailable`. |
| Spawn `RUSTC -Vv` | An executable lookup, permission, or process-launch error is propagated. |
| `RUSTC -Vv` exits unsuccessfully | The script returns `"<rustc path> -Vv failed with <status>"`; captured output is not hashed on this path. |
| Hash and format successful inputs | `Sha256` finalization and hexadecimal formatting complete in memory; there is no recoverable output-file failure path. |

The script does not substitute an empty digest, use a stale digest, skip an
unreadable file, or fall back to another compiler. A failed invocation leaves
the real build failure visible. If the script were changed so that it did not
emit the variable, `env!("RECIPE_NATIVE_PROBE_SOURCE_DIGEST")` in
`src/identity.rs` would fail package compilation instead of silently choosing a
default.

## Downstream use of the generated value

`src/identity.rs` binds the Cargo value at compile time:

```rust
const PROBE_SOURCE_DIGEST: &str = env!("RECIPE_NATIVE_PROBE_SOURCE_DIGEST");
```

`backend_toolchain_identity` then hashes this string into a second SHA-256
domain, `recipe-native-probe-toolchain-and-benchmark-v2`, after the backend,
release, and target-configuration fields and before the required pinned native
tools. It returns `recipe_core::ToolchainIdentity` with:

```text
ToolchainIdentity:
  name: "recipe-owned-llvm-<backend>"
  version: configured release label
  digest: SHA256(backend, release, target_configuration,
                 PROBE_SOURCE_DIGEST, required tool paths and digests)
```

The function is called from both native descriptor paths:

* `native-probe/src/cuda.rs` uses backend `nvidia-cuda` and includes the CUDA
  architecture, PTX ISA, and configured dependent FMA-chain length.
* `native-probe/src/hsa.rs` uses backend `amd-hsa` and includes the HSA target,
  code-object version, and configured dependent FMA-chain length.

The resulting value is stored in `probe::GpuDescriptor.toolchain`. The probe
cache identity in `probe/src/engine.rs` hashes the toolchain name, release, and
digest for every GPU, so a changed build input produces a different measured
profile identity rather than reusing a profile made by another native-probe
build. The same `ToolchainIdentity` is carried into target plans, artifact
identity hashing, native preparation, and resume checks. A previously realized
kernel whose target or toolchain no longer equals the current measured plan is
rejected as an identity mismatch; the source digest is therefore part of the
compatibility boundary for native images, not merely diagnostic text.

The generated digest does not by itself prove that a compiler tool, CUDA
library, ROCr library, or GPU is usable. Those inputs are pinned, hashed,
loaded, or benchmarked by the runtime probing and preparation paths. The build
script only makes the source/configuration/compiler identity available to those
paths and ensures that changing it invalidates the identities they publish.

## Observed Cargo evidence

`cargo check -p recipe-native-probe` succeeds with the current checkout. The
recorded build-script output under `target/debug/build/recipe-native-probe-*/output`
contains 115 `cargo:rerun-if-changed` lines, 12
`cargo:rerun-if-env-changed` lines, and one `cargo:rustc-env` line. The emitted
digest is build-context-specific, so the hexadecimal value in that generated
file is evidence for that invocation only and is not a repository constant.
