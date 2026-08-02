# Recipe hardware acceptance

This package is an executable gate, not a Rust test harness. It requires the
real dataset/model bytes, an existing measured Recipe profile, a real CUDA GPU,
and the exact pinned llama.cpp oracle. Missing prerequisites fail the command.

Build the pinned CUDA oracle from an exact llama.cpp checkout:

```bash
git clone https://github.com/ggml-org/llama.cpp.git /path/to/llama.cpp
git -C /path/to/llama.cpp checkout aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3
cmake -S acceptance/oracle -B target/recipe-llama-oracle \
  -DLLAMA_CPP_ROOT=/path/to/llama.cpp -DCMAKE_BUILD_TYPE=Release
cmake --build target/recipe-llama-oracle --target recipe-llama-oracle -j
```

Then run the complete CUDA gate explicitly:

```bash
cargo run --release -p recipe-acceptance -- cuda \
  --dataset-root examples/datasets \
  --digest-manifest acceptance/inputs.sha256 \
  --recipe-cli target/release/recipe \
  --llama-oracle target/recipe-llama-oracle/recipe-llama-oracle \
  --samples 7
```

The command performs one real UCI Airfoil training workflow and checks its
full-partition optimizer submissions, GPU-only calculation placement, and
completed native lifecycle. A real GRU train-save-infer workflow separately
proves one bundled image per recurrent training and saved-model inference run,
full-partition recurrent updates, and teardown. The command then runs nine literal `recipe run` save/resume
and artifact-count cases in fresh directories, including a real epoch-bound
SIGINT, complete teardown, and loadable stopped-model resume. It then runs the exact
dense-F32 GGUF/token workload through Recipe and pinned llama.cpp, rejects
logit NMSE at or above `1e-3`, reports all seven throughput samples, and fails
when Recipe's median tokens/second is lower. The current runner is intentionally
CUDA-only because that is the hardware available for this acceptance record;
it makes no AMD claim.

The throughput interval is each implementation's checked native decode loop:
Recipe measures the completed immutable loop after admission and before output
publication/teardown, while the pinned oracle measures `llama_decode` and
logit collection. Dataset/model parsing, compilation, realization, warm-up,
and teardown are outside both throughput samples; their lifecycles remain
covered by the surrounding acceptance evidence.

Every invocation writes a tab-delimited record beneath `target/acceptance/`,
including failed invocations. The record captures the commit and dirty state,
release/debug profile, Rust/CUDA/LLVM/linker identities, exact measured-profile
and device identities, input and oracle revisions, native resource counters,
correctness result, all timing samples, medians, and the final failure reason.
Files in this directory are acceptance observations, never model artifacts.
