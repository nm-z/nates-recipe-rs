# Recipe API implementation status

Snapshot: 2026-08-02. [`API.ogdl`](API.ogdl) is the declaration specification;
this file is a routing inventory, not proof that a declaration works. A
declaration is hardware-proven only when a dated real-data run appears in the
hardware record below. Compilation, private graph inspection, mock execution,
and deleted proxy tests do not supply that evidence.

| `API.ogdl` surface | Implementation state | Production boundary or exact gap |
|---|---|---|
| `recipe`, `.data`, `.set`, `.target`, `.exclude`, `.norm`, `.split` | routed | Public declaration and bounded preparation are in [`src/facade.rs`](src/facade.rs), [`src/api.rs`](src/api.rs), [`src/data_prepare.rs`](src/data_prepare.rs), and [`recipe-ingest`](recipe-ingest). |
| `.model().load(path)` | partial | Exported OGDL families and the bounded dense-F32 GGUF-v3 Llama subset lower in [`src/inference.rs`](src/inference.rs) and [`recipe-training`](recipe-training). Other GGUF architectures, quantization, MoE, and GQA fail closed. |
| `.bayes(child, [parents])` | routed, bounded subset | Repeated observed categorical conditionals are implemented. Numeric distributions, latent nodes, missing training observations, and configurable priors are not declared as working behavior. |
| `.perc`, `.layer`, `.residual` | routed | Dense training, backward propagation, AdamW, checkpointing, resume, and inference are implemented in [`recipe-training`](recipe-training). |
| `.conv`, `.pool` | routed, bounded subset | Stride-one channelwise 1-D convolution and channelwise 1-D max pooling are implemented. |
| `.kmeans(...).layer(...)` | routed | Full-partition Lloyd transitions, retained empty centroids, checkpointing, resume, and inference are implemented. |
| `.knn(neighbors)` | routed | Terminal all-output KNN preparation, semantic save/load/resume, and native inference are implemented. Post-reduction activation and normalization remain invalid. |
| `.lgbm`, `.cbst`, `.xgbst`, `.forest` | routed | Terminal supervised tree families, checkpoint-v12 save/resume, and inference are implemented. |
| `.embed(...).vocab(...)`, `.attn(...)` | routed, bounded subset | Fixed-width integer-token embedding and one causal self-attention block are implemented. No tokenizer, padding policy, positional encoding, or configurable mask is implied. |
| `.rnn`, `.gru`, `.lstm` | routed, bounded subset | One leading independent scalar-sequence recurrent block is implemented for each family with complete BPTT, AdamW, save/resume, and inference. Stacking, padding, and cross-row state are rejected. |
| layer/batch `.norm(...)` | routed | Ordered forward/backward lowering is implemented. |
| all declared activations | routed | Forward/backward lowering and checkpoint identities are implemented; exact formulas and domains remain normative in [`system-contract.md`](system-contract.md). |
| all declared losses | routed | BCE, MSE, MAE, CE, Huber, and focal loss lower to dtype-checked Recipe operations. |
| `.grad(clip: maximum_norm)` | routed | Optional global-norm clipping is implemented. |
| `.train().optimizer(adamw)` | routed | Recipe-owned AdamW executes through the immutable native lifecycle. |
| `.epochs`, `.lr`, `.cos`, `.exp`, `.warmup` | routed | Finite and unbounded schedules are implemented; unbounded cosine/exponential schedules without an endpoint reject. |
| training `.log`, `.every`, `.plot` | routed where mathematically applicable | Metrics execute through the production metric path; target-derived metrics reject when the task cannot define them. |
| `.resume(model[, kernel])` | hardware-proven on CUDA | Literal one- and two-path forms implement existence-conditional semantic resume, authenticated native reuse, missing-kernel recompilation, and loadable resume after a safe stop. |
| `.save(model|kernel[, kernel])` | hardware-proven on CUDA | Literal one- and two-path forms route export by extension. The nine-case CUDA run produced exactly the declared zero, one, or two files. |
| `.run()` | routed | Measured-profile native preparation, execution, exit, and teardown are in [`src/training.rs`](src/training.rs). |
| `.infer().load(...).log(...).evaluate()` | routed for supported models | Target-free preparation, native execution, teardown, returned values, and streamed prediction rows are in [`src/inference.rs`](src/inference.rs). |

## Hardware acceptance record

| Date | Hardware | Public real workload | Result |
|---|---|---|---|
| 2026-08-02 | NVIDIA GeForce RTX 2050, CUDA Driver 610.43.03 | UCI Airfoil, 1,202-row training partition, one full epoch | Passed one logical update, all six AdamW parameter submissions, zero non-GPU calculation, one CUBIN load, lifecycle, and teardown gates. |
| 2026-08-02 | NVIDIA GeForce RTX 2050, CUDA Driver 610.43.03 | GRU train-save-infer on 12 real binary-classification rows | Passed nine-row full-partition update, one CUBIN load per training/inference run, target-free 12-row inference, exact artifact ownership, lifecycle, and teardown gates. |
| 2026-08-02 | NVIDIA GeForce RTX 2050, CUDA Driver 610.43.03 | Nine literal `recipe run` artifact/resume cases | Passed exact zero/one/two-file outputs, missing-model fresh start, native reuse/recompilation, epoch-bound SIGINT, teardown, and loadable stopped-model resume. |
| 2026-08-02 | NVIDIA GeForce RTX 2050, CUDA Driver 610.43.03 | Dense-F32 GGUF Llama, 128 real token IDs, 16,384 raw logits | Passed correctness (NMSE `8.001529e-8`), exactly-one-CUBIN-load, lifecycle, and performance gates after shared recurrent/checkpoint unification. Recipe median `64762.972580` tok/s; pinned llama.cpp median `30743.025296` tok/s. Record: `target/acceptance/cuda-1785672520-540204.tsv`. |
| 2026-07-30 | AMD Radeon RX 7700 XT (`gfx1101`) | Earlier public HSA cookbook runs | Historical record only; not rerun during the current acceptance pass because only NVIDIA hardware is available. It does not make a current AMD matrix cell green. |

The versioned CUDA runner, exact input digests, and pinned llama.cpp adapter are
in [`recipe-acceptance`](recipe-acceptance). Its performance threshold remains
strict: Recipe fails when its median checked-loop throughput is below the pinned
oracle on the same GPU.

## Export audience classification

| Surface | Audience | Rule |
|---|---|---|
| `recipe`, `Data`, `Model`, `Train`, `Infer`, metrics, losses, and declaration errors | Recipe authors | This is the short declaration facade. It exposes no batch, tile, checkpoint cadence, early-stop, watchdog, or hardware-counter controls. |
| `recipe::operations` | Operation authors and audits | Normative operation inventory and checked lowering, not a training-policy surface. |
| `recipe::engine::*` and workspace crates | Backend implementers | Typed planning, preparation, executor typestate, bounded journals, and native integration. These are not exported model artifacts. |
| `TrainingReport` and `InferenceReport` | Script results | Completed outcomes and read-only execution observations; never execution controls or additional artifact paths. |

Static build commands establish build hygiene only. Current proof requirements
and adaptive implementation scope are maintained in [`plan.md`](plan.md).
