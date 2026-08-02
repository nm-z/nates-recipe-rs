# Recipe examples

`cookbook.rs` contains the complete set of runnable public-API recipes in one cookbook. Each named workflow retains
its real data, training, save/resume, or inference declaration without multiplying example binaries.

Native training and inference need a current measured profile:

```text
cargo run --bin recipe -- probe
cargo run --example cookbook
```

The cookbook currently covers the executable data, dense layer, perceptron, ordered multi-target objectives,
convolution/pooling, K-means, all-output KNN, fixed-token embedding and causal multi-head attention, dense-F32 llama
GGUF token-logit inference, scalar-sequence vanilla RNN, reset-before GRU, and zero-cell LSTM, singular and repeated
observed categorical Bayesian target conditionals, supervised tree/forest, residual, activation (including distinct
signed `.log()` and natural `.ln()` semantics), normalization, loss, optimizer, schedule, observability, save/resume,
and semantic-OGDL inference families. Examples for the remaining declared model cases belong here only after their
public execution paths exist; a declaration-only example is not evidence that an API works.

## Dated hardware matrix

Only completed public workloads are marked as observed. The current machine has NVIDIA hardware only, so no AMD cell
is inferred from compilation or historical runs.

| Date | Public workload | NVIDIA CUDA | AMD HSA |
|---|---|---|---|
| 2026-08-02 | UCI Airfoil full-partition dense training | observed on RTX 2050 | not run |
| 2026-08-02 | GRU full-partition train-save-infer lifecycle gate | observed on RTX 2050 | not run |
| 2026-08-02 | Literal save/resume/SIGINT artifact matrix | observed on RTX 2050 | not run |
| 2026-08-02 | Dense-F32 GGUF Llama inference | correctness/lifecycle/speed observed on RTX 2050 | not run |
| 2026-08-02 | Unified cookbook: all 21 public workflows | observed on RTX 2050; tree/forest Compute Sanitizer clean; GGUF speed proven by acceptance runner | not run |

The pre-consolidation NVIDIA cookbook record is
`target/acceptance/cookbooks-cuda-1785670794-454449/results.tsv`: all 21 workflows exited successfully as separate
release binaries, and the record retains each source/binary digest, elapsed time, dataset digest, GPU identity, and
full per-workload log.
