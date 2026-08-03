# Recipe

Recipe is a capability-driven, ahead-of-time scheduler and execution system for
AI, deep-learning, and machine-learning workloads on AMD and NVIDIA GPUs.
`main` contains the replacement implementation. The retired HIP implementation
is preserved on the `legacy` branch and is not part of this workspace or its
dependency graph.

The replacement owns its calculation primitives and talks only to the native
ROCr/HSA and CUDA Driver interfaces. It does not use HIP, the CUDA Runtime API,
or vendor operation libraries such as rocBLAS, MIOpen, RCCL, cuBLAS, cuDNN, or
NCCL.

## Current entry points

The root library exports immutable, validated declarations for data, models,
training, and inference. It also exposes the lower-level discovery, language,
primitive, planning, scheduling, preparation, and execution crates.
Declaration construction has no hidden I/O or execution side effects.

The installed command currently exposes the bare-metal probe:

```bash
cargo run --release -- probe
```

`recipe probe` discovers the current host, storage, RAM, links, and native GPU
devices, benchmarks the discovered resources, and writes an identity-keyed
measured profile beneath the user's private state directory. The estimates in
`topology/contract.toml` bound the initial benchmark work; they are not accepted
as production scheduling values, and users do not manually enumerate hardware
or enter rates.

Use `recipe probe --help` for exact library and toolchain path overrides. The
probe requires LLVM `opt` and `llc`. AMD probing additionally needs ROCr/HSA and
an ELF linker; NVIDIA probing needs the CUDA Driver library and can use a pinned
`ptxas`.

Distributed runs use the standalone Ethernet worker:

```bash
cargo run --release --bin rpc -- rpc.toml
```

The required file names the worker address and byte bound explicitly:

```toml
[rpc]
listen_address = "0.0.0.0:7331"
max_payload_bytes = 67108864
```

Hugging Face tokenization is a standalone preprocessing step. It reads text
from a file or stdin and writes raw little-endian int32 token IDs:

```bash
cargo run --release --features tokenize --bin tokenize -- tokenizer.json input.txt tokens.i32
```

The public dense-binary training facade loads and semantically prepares the
dataset, compiles Recipe-owned primitives, schedules and prepares the measured
native GPU, executes the immutable training lifecycle, streams nonblocking
metrics, tears native resources down, and saves the final weights as OGDL. The
complete runnable path is:

```bash
cargo run --release --example train
```

It writes `model.ogdl` only after a successful `init -> loop -> exit`
lifecycle. Inference builders remain checked declarations; the README does not
advertise the retired `train`, `serve`, or `detect` CLI commands.

## Build and acceptance

```bash
cargo build --workspace
cargo check --workspace --all-targets
cargo build --examples --release
cargo run --bin recipe -- probe --help
```

The real-data, real-hardware public workflow is the cookbook:

```bash
cargo run --release --example cookbook
```

Missing prerequisites fail rather than skip.

Compilation is build hygiene, not runtime evidence. Acceptance executes public
training and inference declarations on real datasets and real CUDA or HSA
hardware. A missing device, driver, dataset, compiler, or comparison oracle
makes that acceptance run unsuccessful rather than skipped. Standalone probing
is useful diagnostics but does not establish workload correctness.

The main pipeline is split into focused crates:

- `recipe-probe` and `recipe-native-probe`: bare-metal discovery and bounded
  measurement
- `recipe-language`, `recipe-primitives`, and `recipe-ops`: backend-neutral
  f32/int32 calculations and the owned operation inventory
- `recipe-planner`, `recipe-scheduler`, and `recipe-prepare`: finite candidate
  planning, AOT scheduling, realization, and immutable finalization
- `recipe-cuda` and `recipe-hsa`: reviewed native driver/runtime boundaries
- `recipe-executor` and `recipe-native-executor`: typestate lifecycle and native
  execution
- `recipe-host` and `recipe-ingest`: preallocated host storage plus bounded
  pre-init data preparation

The root `rpc` and `tokenize` binaries own distributed Ethernet transport and
text preprocessing without adding library crate boundaries.

The normative contracts are in `system-contract.md`,
`topology/contract.toml`, and `operation-surface.txt`.
