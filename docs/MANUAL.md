# CLAUDE.md — nates-recipe-rs Manual

## 1. What This Is

GPU-native neural-network training framework in Rust: builder API for models, data loading (CSV/ARFF/zip/image dirs), training on AMD GPUs via HIP/ROCm, evaluation. All compute runs on GPU through `gpu-core`; no CPU ML crates. 5-member workspace, strict one-way dependency DAG:

```
gpu-core                       HIP kernels, links ROCm, depends on nothing
recipe-infer → gpu-core        forward pass + ogdl/safetensors load; pure tensor fns
                               (weights + input matrix → output matrix); owns GPU
                               device lifecycle (init/shutdown); knows nothing of Datasets
pantry       → recipe-infer    ALL parsing (csv/arff/zip/dir), encoding + NaN policy,
                               trained char-level column-type detector + `detect` binary
recipe       → all three       Model/Train builder, backward, fit (in-VRAM + out-of-core),
                               save/resume, TUI, eval; delegates loading to pantry
vramspy                        LD_PRELOAD cdylib interposing HSA alloc entry points;
                               counts library-side VRAM beneath the ledger choke points
```

`catboost-rs` / `lightgbm-rs` / `xgboost-rs` are workspace-excluded, standalone, untouched. The `recipe-infer` boundary is a pure tensor function — it must never reference Dataset/Model/Train (grep-enforced); the Dataset→Mat conversion is the seam between "understanding data" (pantry/recipe) and "running math" (recipe-infer).

## 2. Build, Run & Test

### Commands

```bash
cargo build --release                    # thin LTO, links ROCm
cargo test all                           # THE suite (SUITE SPEC v3): every test in every crate, one
                                         #   OS process per test, 60s SIGKILL deadline each = that
                                         #   test's FAIL (suite continues), device-health probe after
                                         #   any crash/kill, one log: suite.log; verdict:
                                         #   rg '\[FAIL\]' suite.log
cargo test -p recipe-infer --release     # forward/KV-cache/ogdl behavioral tests (GPU)
cargo test -p recipe model::metric_gpu_tests::   # GPU metric/gradient tests
cargo test -p gpu-core --release --test suite    # kernel proof suite (tests/suite/: inventory_proof,
                                         #   parity_blas_*, prove_*; needs kernel_inventory.db)
cargo run --release -- train.csv --target Price       # recipe CLI
cargo run --release -- detect <path>                  # column-type detection
./target/release/detect <path>           # standalone GPU-only detector (pantry bin)
cargo run --release --example cookbook   # the e2e: NN/CNN/MLP/LLM scenarios
cargo run --release --example train_detector          # retrain → pantry/detector.ogdl
cargo run --release --example gemma4 -- "prompt"      # gemma-26B f64 inference
```

### Environment

ROCm default `/opt/rocm`; overrides: `ROCM_PATH`, `ROCM_EXTRA_LIB`, `ROCM_EXTRA_INCLUDE`, `GPU_ARCH` (default `gfx1101`), `HIPCC`. Datasets live in `datasets/` (gitignored). Edit examples and `/home/nate/Desktop/train.rs` (the one -Zscript file) in place — every new crate target or new script file is its own crate and triggers a full ~15-min rebuild; same-file edits rebuild incrementally in seconds.

### Build system

- `gpu-core/build.rs` compiles every `.hip` under `src/kernels/` with hipcc/amdclang++ into `libhipkernels.a`, then links `amdhip64` (+ hipblas/hipsolver/hipfft — under-migration exceptions, see §11). It must rebuild the archive from scratch each run: `ar r` adds/replaces members but never removes stale ones.
- Root `build.rs` scans `src/*.rs` and rejects `hipMalloc(` / `hipFree(` (sync variants) at compile time. gpu-core's build.rs rejects `rocblas`/`cublas` tokens in `src/*.rs`. Note: the root scan covers only the root crate's `src/`, not gpu-core.
- An NVIDIA backend path routes rocPRIM/hipCUB kernels through plain nvcc using `src/nvidia_compat/` header shims (rocprim→CUB, maskless `__shfl_*` → `*_sync`); inert under hipcc.

## 3. Codebase Map

```
recipe-infer (forward engine — tensors in, tensors out; deps gpu-core + ndarray only)
├── src/lib.rs         — re-exports; init()/shutdown() (GPU device lifecycle); human_bytes
├── src/enums.rs       — Activation, LayerSpec, LayerKind, Loss, Metric + user consts (mse/ce/R2/…)
├── src/params.rs      — Saved, LayerParams, Scaler, build_layer_params, sinusoidal_pe,
│                        concat_layer, pinned_vocab
├── src/scratch.rs     — Scratch (ping-pong activation/grad arena), vram_bytes estimate
├── src/forward.rs     — forward_into (dense/embed/attn/conv + KV-cache flash-attn inference
│                        path), attn_forward(_cached), metric_gpu(_into), upload/zscore/download
├── src/ogdl.rs        — OGDL checkpoint codec: load_ogdl(_str), dump_ogdl, saved_score
└── src/safetensors.rs — safetensors header parser (gemma weight loading)

pantry (all parsing + encoding + detection; deps recipe-infer only)
├── src/lib.rs         — Mat/Vec1 aliases, Kind enum, Attr struct, available_ram_bytes
├── src/data.rs        — loaders: read_raw_csv, parse_arff, load_groups/zip/dir, image dirs,
│                        group_and_hash (join keys), RAM guards
├── src/encode.rs      — encoding + THE one NaN adapter: nan_clean (Drop/ImputeMean/Error),
│                        clean_dataset; assemble (hash join); select (lazy materialization)
├── src/bpe.rs         — tokenizer
├── src/detect.rs      — detector inference: tokenize_column, predict_kinds (runs the embedded
│                        ogdl through recipe-infer's forward), KIND_* consts, CONTEXT=256, VOCAB=257
├── src/main.rs        — standalone `detect` binary (init → load_groups → predict_kinds → print)
└── detector.ogdl      — trained detector weights, include_str!'d (compile-time — rebuild to ship)

recipe (root crate — training framework)
├── src/lib.rs         — type aliases, re-exports (incl. `pub use pantry::data`, recipe_infer enums)
├── src/main.rs        — CLI: recipe <train.csv> [--target <col>] | recipe detect <path>
└── src/utils/
    ├── dataset.rs     — Data builder (.load().set().exclude().test().split().target()); Dataset;
    │                    LAZY — .target() only describes, Train::run materializes+frees per run;
    │                    collapse_onehot (embed-on-categorical seam)
    ├── model.rs       — Model (layer stack + chained activations), Train (run/save/resume), TUI,
    │                    preflight, post-fit scoring, RunData/Prepared, thread-local registry
    ├── train.rs       — backward_step, fit loop, use_ooc decision, checkpoints, ^C handling
    └── ooc.rs         — out-of-core fit: per-window VRAM→RAM→DISK homes, spill file,
                         read-ahead + write-behind through a fixed host pool (POOL_BUFS)

gpu-core (HIP/ROCm)
├── src/hip.rs         — HIP FFI, set_device, streams/events, pinned, mem_info/pool_slack/trim,
│                        sysfs_vram_free, device_synchronize
├── src/memory.rs      — GpuBuffer + THE choke points: ONE call site each for alloc/free/xfer/
│                        memset; byte ledger (live+peak per tag, cumulative H2D/D2H/D2D);
│                        pool growth gate; pinned H2D bounce; par_copy/par_touch; device arena
│                        (one-claim mode); claimable_bytes(); ledger_report()
├── src/kernels.rs     — pub fns: gemm, activations, losses, metrics, optimizers, reductions
├── src/kernels/*.hip  — ~57 HIP kernel source files → libhipkernels.a
├── src/waterfall.rs   — one-claim VRAM→RAM→DISK blob store for immutable weights (gemma)
├── src/tiered.rs      — VMM tiered buffer (hipMemAddressReserve+hipMemMap)
├── src/infer_ops.rs   — f64 inference ops (rmsnorm, gemm_bt, rope, gqa attn, gelu_mul, widen bf16)
├── src/callspy.rs     — HIP API call counting (the `hip` metric)
├── src/nvidia_compat/ — nvcc-only shims
├── tests/suite/       — kernel proofs (inventory_proof, parity_blas_*, prove_*, oversize_oom)
└── src/{attention,bayes,catboost,cluster,diffusion,encoding,forest,graph,k_actx,k_gapact,
     k_mathx,linalg,losses,math_ops,moe,nn_f32,optimizers,reductions,rl,rope,sequence,svm}.rs
     — domain modules, one per file, each with its own extern "C" FFI block

vramspy (cdylib)
└── src/lib.rs         — LD_PRELOAD interposer for hsa_amd_memory_pool_{allocate,free} +
                         hsa_memory_{allocate,free}; classifies by pool-owning agent (ownership,
                         not segment flags, is the reliable discriminator); memory.rs reads the
                         counters via RTLD_DEFAULT when run under LD_PRELOAD=libvramspy.so
```

Notes: `svm`/`cluster` neighbor/perm fns are standalone prototypes with no production callers (signatures freely changeable). `nn_f32` and other f32 mirrors are dead code under the f64 rule — never count them as pending work or wire them.

## 4. User API

Three import styles, one crate, the same methods — only the door differs. Everything
after the constructor is dot chaining, identical whichever door you came in by.

```rust
// style 1: static (dot syntax)          use recipe::*;
recipe.data("train.csv").split(0.8).target("Price");
recipe.model().layer(64).leak().layer(1).loss(mse).lr(0.001);
recipe.train().epochs(100).run(data, model);

// style 2: struct (associated function)  use recipe::{Data, Model, Train};
let data  = Data::load("train.csv").split(0.8).target("Price");
let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.001);
Train::new().epochs(100).log([Loss, R2, Lr]).plot([Loss, R2]).run(data, model).save(());
model.eval(data);

// style 3: crate path (free function)
let data  = recipe::data("train.csv").split(0.8).target("Price");
let model = recipe::model().layer(64).leak().layer(1).loss(recipe::mse);
recipe::train().epochs(100).run(data, model);
recipe::eval(model, data);
```

`recipe` is a unit `static` in the value namespace, so `recipe.data(…)` (style 1) and
`recipe::data(…)` (style 3, the crate path) coexist without ambiguity.

Every constructor returns `&'static mut Self` (`Box::leak` of the heap-pinned inner),
so every builder method can take `&mut self` and return `&mut Self`: the chain and the
`let` binding it produces name the same config, at an address that never moves.

- **`Data`** — `Data::load(path)` accepts CSV, ARFF, `.safetensors`, zip, or a directory; `.set(path)` adds a further source. `.exclude(pattern)` (exact / `group:*` / group / bare header). `.test(path)`, `.split(frac)`, `.target("col")` / `.target(["a","b"])`, `.datasets()`. **Lazy**: `.target()` only records config; `Train::run`/`Model::eval` materializes via `Prepared{Owned,Borrowed}` and frees the Owned dataset when the run returns — only one scenario resident at a time (this is what lets the 4-scenario cookbook run without OOM).
- **`Model`** — `.layer(n)` + chained activation: `.relu() .leak() .sigmoid() .tanh() .selu() .gelu() .silu()` (also prelu/elu in enums). `.layer(embed(dim))`, `.layer(attn(heads))`, `conv(filters, kernel, stride)` — stride is the downsampling mechanism (no pool API). Embed behavior: text cols → embed token ids; no text but categoricals → embed categorical indices (one-hot groups collapsed to integer indices via `collapse_onehot`); no embed → categoricals stay one-hot. `.loss()`, `.lr()`, `.eval(data)`. `Model::load(weights, proto, d)` builds forward-only from shipped OGDL text. `ModelInner` is crate-private — `eval` lives on `Model`.
- **`Train`** — `.epochs() .log_every(n) .log([..]) .plot([..]) .resume(p) .net([..])`. `.run(data, model)` takes both explicitly (`data` is any `&dyn RunData`: a `Data`, a `Dataset`, an `Option<Dataset>` holdout) and returns `&mut Train`, so `.save(p)` chains straight off it. Training vs inference is decided by the data (no target / 0 epochs → forward only).
- **Forward-only arena** — a fit carves the arena to the last byte (`ooc` budgets its windows from `arena_remaining()`), so a forward pass cannot carve even one input buffer out of what the run parked. `Model::begin_forward` re-arms that parked backing as the pass's arena (rewind + one memset, **no free, no realloc** — the freeAsync VA-reuse race and the driver's post-free counter depression both stay impossible), rebuilds the weights from the host mirror, and `end_forward` parks it again for the next run.
- **save/resume** — `.save(p)` / `.resume(p)` take `impl SavePath`; `()` means `"model.ogdl"` (Rust can't overload arity, so it's `.save(())`, not `.save()`). `save` writes ALL params with a best-only guard (only overwrites if the new score beats the saved `saved_score`). Resume shape mismatch prints OGDL-vs-model dims and exits(1) — never silently reinitializes. NaN cells in a loaded OGDL are individually re-randomized (He-scaled) with a report, not rejected.
- **Preflight** checks VRAM (`Scratch::vram_bytes`), embed/text, and loss/output shape before any GPU work; over-ceiling scenarios are skipped gracefully (the RAM guard bail is caught via catch_unwind in `run()`), not crashed.
- **Consts** — losses `mse mae huber ce bce focal`; metrics `Loss Accuracy R2 Lr Epoch Time hip` (hip = HIP call report).

## 5. Data Pipeline (pantry)

### 5.1 Column-type detection

A trained char-level transformer, not heuristics: `embed(32).vocab(257) → attn(4) → 64.leak() → 6 → ce`. Each column's raw cells → byte stream (`id = byte+1`, PAD=0, CONTEXT=256) → one row → argmax over 6 Kinds: **Numeric, Temporal, Categorical, Ordinal, Text, Image**. Missing markers (NA, NaN, N/A, NULL, None, ?, ., -) are filtered before detection. Inference runs the embedded `pantry/detector.ogdl` through recipe-infer directly (no Model/Train — that would cycle the DAG).

Detection logic contains **zero picked numeric thresholds** — every such constant is a guess some real dataset lands on the wrong side of. Allowed distinctions are binary structural tests only: all-values-parse-as-f64 (Numeric), structural format ID (Image), `distinct < total` vs `distinct == total` (Categorical vs Text). Don't reintroduce range/ratio/count constants.

Trainer lives in `examples/train_detector.rs` (corpus + labels don't ship in the library). Retraining gotchas: (1) on a corpus change, delete `pantry/detector.ogdl` first — the best-only save guard blocks incomparable scores; (2) `include_str!` bakes weights at compile time — rebuild after training. Current weights: train 0.957 / held-out 0.934 on the expanded corpus (wide numeric files are column-sampled so one file can't flood the Numeric class; it overfits past ~10k epochs — don't chase the last points with more epochs).

### 5.2 Directory loading & the hash join

`load_dir_groups` parses a directory into GROUPS by file type (`feature_group` = part after `__` in the filename, or the extension). CSVs within a group STACK rows (rows = samples; no collapse, no aggregation, ever). A single file = one un-grouped Table. Raw string cells are kept; typing + encoding happen later once roles are known.

`assemble`: the group owning `.target()` defines the samples at full resolution. Every other group hash-joins onto it: 1-row-per-hash BROADCASTS; equal-rows-per-hash ALIGNS by within-hash position; mismatched counts are reported as unjoined and left out — never averaged to force alignment (the user `.exclude()`s or fixes the join). Columns are namespaced `group:col` (same header across groups is NOT the same feature — different instrument/context).

**Images**: the whole dir is ONE `DirGroup::Image`, keyed by file stems. The join to a CSV works by finding the CSV column whose cell stems match the image filename keys (that `Kind::Image` filename column is the join key, contributes 0 feature width); each row gathers its image (32×32×3 → 3072 flattened features). Materialization is lazy (`Assembled` + gather indices; `select(feats)` builds only kept columns) so a dropped/excluded image group costs nothing.

Target resolution: explicit `.target` wins; else if a test file exists and exactly one table column is train-only, that's the target; else None.

### 5.3 Encoding

- Numeric → passthrough (blank/unparseable → NaN).
- Categorical FEATURE → one-hot `col=cat` (categories inferred from the train set; test reuses them BY NAME, not position).
- Categorical TARGET → ONE class-index column (0..N-1). In fit, `expand_ce = classify && n_targets==1 && out_dim>1` expands it host-side to a one-hot of the model's `out_dim` — so `.layer(36)+ce` works even when train shows only 35 classes, and `.layer(1)+bce` works for binary.
- Text / high-cardinality → feature hashing: tokenize → hash mod D → fixed-width D-vector of counts. One-hot is the special case where the vocab fits D. High-cardinality columns are never excluded, never collapsed to a frequency column, never cardinality-capped — the fix for a too-big matrix is a clean failure naming the size and culprit columns.
- Temporal → days.

### 5.4 NaN policy — one adapter

All NaN VALUE-policy lives in `pantry::encode::nan_clean(v, strategy, name)` (`enum Nan { Drop, ImputeMean, Error }`), invoked once per column-vector by `clean_dataset` at `Data::prepare`: features → ImputeMean, targets → Drop (a missing label can't be invented). After it the matrix has no NaN; nothing downstream handles NaN. The missing→NaN *producers* (parse/join/image) are data representation, not policy — they stay. GPU `has_nan`/`isfinite_all` are test-only diagnostics.

### 5.5 RAM guard

`pantry::available_ram_bytes`-based guard panics if the projected parse exceeds 90% of available memory (`libc::malloc_trim(0)` runs first so glibc-retained pages don't skew it). Corpus measurement lesson: encoding bloat is usually concentrated in one pathological file, not systemic — measure (per-file f64-matrix projection) before generalizing any fix.

## 6. Training Engine (recipe)

### 6.1 The loop and its five constraints

Full-batch gradient descent on GPU. Per epoch: forward all layers → loss gradient → backward all layers → SGD update. Constraints: (1) load data once RAM→VRAM; (2) N kernels in a loop; (3) zero allocations inside the loop (`AllocGuard` panics on `GpuBuffer::alloc_bytes` inside); (4) no VRAM↔RAM roundtrips inside — the only D2H is the 8-byte metric scalar, one-way, via the async copy-stream (`read_metric_scalar`; zero per-epoch blocking copies); (5) the API surface should make violations impossible (partially met — the guard is runtime). `fit_loop_memory_flat` is the regression test.

### 6.2 Scratch (recipe-infer/scratch.rs)

Ping-pong gradient buffers instead of per-layer: `da_a`/`da_b` alternate across backward layers (a `flip` bool), single max-sized `dz`/`dw`/`db` reused per layer, `acts` per-layer (backward needs all activations), `dw_partials` sized for the widest layer (split-K). Roughly halves the working set on deep stacks. `Scratch::drop` calls `device_synchronize()` FIRST — see §7.5.

### 6.3 Attention

Training attention is flash on every path — no L×L score matrix ever materializes. Forward emits per-row logsumexp `a_lse` [n·h·S]; backward is 3 deterministic passes (dsum → dQ query-major → dK+dV key-major), recomputing P per tile as exp(q·k·scale − lse), no atomics (CPU-oracle match <1e-12). Scratch carries `a_lse`/`a_dsum` (n·h·S), not scores (n·h·S²). Shared-mem ceiling: head_dim ≲ 24–32 on 64KB LDS. RoPE (per-head rotary, theta 10000) is applied to a_q/a_k after projection, un-rotated on gradients (orthogonal rotation) — wired into `attn_forward`, `attn_forward_cached`, and `attn_backward`. Inference uses the KV-cache path (`attn_forward_cached`, len-1 backward buffers); equivalence with full attention is tested to ~1e-13.

### 6.4 GEMM performance model

All shapes here are tall-skinny (M = full sample count). Forward `Y=X·W` is bandwidth-bound (short K); backward `dW=Xᵀ·grad` has a tiny output (out×in) with a huge contraction — a naive GEMM starves the 54 CUs (a few workgroups). Fix is **split-K**: two-pass custom kernel (`splitk_dw_pass1/2` in `kernels/math.hip`, partials buffer in Scratch, deterministic, validated to ~1e-14; 7.8× over the library kernel on the churn shape). Rule of thumb: small-output/huge-K GEMM = occupancy starvation → split-K, not retiling. Never claim %-of-peak without measured GFLOP/s ÷ hardware peak using actual dispatch dims (`rocpd_kernel_dispatch`: workgroups = grid/workgroup).

### 6.5 Checkpointing

When `.save` is configured, every epoch computes train R² and, on any increase vs the previous epoch, writes the **pre-update** weights (the ones that produced that R²) — crash resilience; clean finish/SIGINT/`q` still write final weights. Saving after backprop would persist weights one SGD step off from the logged R² — keep pre-update timing.

### 6.6 Out-of-core fit (src/utils/ooc.rs)

When training scratch exceeds claimable VRAM, fit streams: every big buffer is homed per-window (~168MB granularity) strictly VRAM→RAM→DISK (one unlinked spill file); windows stream through fixed device staging buffers with read-ahead (depth ≥2) and write-behind worker threads drawing from a preallocated, page-touched host pool (`POOL_BUFS` — structural max, loud panic on exhaustion). Math stays full-batch: every sample flows through every op each epoch. All transient host bytes come from the pool so RAM sits flat at its fill line while disk churns. At ooc-fit end: `gpu_core::memory::pool_trim()` (device sync → `hipMemPoolTrimTo(0)` → reset the verified high-water) — without it, post-fit allocation storms hit fragmented pool slack and die in the driver's uncatchable VmHeap assert. Post-fit scoring reuses fit's own stashed score whenever the forward-only Scratch wouldn't fit — never re-allocate the buffer set that forced ooc.

## 7. GPU Memory System (gpu-core)

### 7.1 Ledger — one call site per API

Exactly ONE call site each for `hipMallocAsync`, `hipFreeAsync`, `hipMemcpyAsync`, memset — the choke points in `memory.rs`. Every byte is tagged and ledgered: live+peak per purpose-tag, cumulative H2D/D2H/D2D bytes+calls; `ledger_report()` dumps it and the OOM autopsy includes it. Never add a raw `hip::` memory call anywhere else — a second alloc site makes the tag accounting a lie. New subsystems register their physical bytes under a tag. "Library-internal, can't measure" is not accepted reasoning: vramspy interposes the HSA layer and `/proc/self/fdinfo` drm-memory-vram cross-checks kernel truth. Total accounting = choke-point ledger (our bytes) + vramspy (library bytes) + fdinfo (kernel).

### 7.2 Waterfall placement + the 1GB reserve

VRAM fills completely before one byte pools in RAM; RAM before disk — water never pools in two layers at once. `gpu_core::waterfall::Waterfall` is THE placer for weight blobs: fill it LAST (arena/stage/workspace pre-allocated), ONE slab = min(hip_free, sysfs_vram_free) − pool_slack, memset once (commits pages). Exactly **1 GB per tier** belongs to the user (`ooc::USER_GB`) — the only headroom constant anywhere; percentage/ratio floors were removed and stay out. "Full" is measured by committed/touched pages (the driver's own numbers), never pool reservation. All will-it-fit decisions go through `memory::claimable_bytes()`, never raw `hipMemGetInfo`.

### 7.3 One-claim lifecycle + blocking-op budget

Steady-state target per run: init → ONE async pool alloc of all free VRAM registered as the device arena (everything carves from it: norms, arena, stage, blobs, workspace), exit → that one async free. Budget: async transfers/allocs/frees unlimited; **max 2 blocking allocations + 2 blocking frees per run**; blocking transfers unlimited. Blocking alloc/free are pipeline drains and pool-growth wedge lottery tickets.

### 7.4 Driver diseases and their fixes (gfx1101 / ROCm 7.2.x)

- **Pool-growth wedge**: hipMallocAsync can spin forever inside HSA during pool growth (stochastic per growth event). Fix: one slab, no growth events; a load watchdog thread aborts loudly after 20s without a heartbeat.
- **Stale-zero pool pages**: fresh hipMallocAsync pages can read back as zeros until flushed → silent wrong results / run-to-run nondeterminism. Fix: memset the slab once at claim; a staleness canary (readback head/tail vs source bytes) bails on mismatch.
- **SDMA↔gfx-L2 incoherency** on reused pool pages (silent wrong GEMM) → `disable_sdma_once`. Pageable-host blit faults → pinned 64MB bounce at the one transfer choke.
- **hipFreeAsync racing an in-flight GEMM on another stream** → intermittent "Memory access fault … Page not present" at phase boundaries. Fix: `Scratch::drop` runs `device_synchronize()` before any frees (plus syncs at fit-end and score-pass). Drain at phase boundaries, never per-free. This fix silently regresses whenever Drop is edited — if a teardown fault reappears, check the sync is still first.
- **Never `hipDeviceReset()` in an atexit handler** — atexit runs LIFO, ours runs before HIP's own `__hip_module_dtor`, which then double-frees (heap corruption with varying symptoms). The OS reclaims VRAM at process death; `gpu_shutdown()` may destroy handles, nothing more.
- **VMM ordering**: a `hipMemAddressReserve`+`hipMemMap` reservation (tiered.rs) corrupts LATER `hipMallocAsync` pointers (first-touch faults). Allocate all pool/arena memory up front, then create VMM buffers; staging windows for tiered consumers come from the pre-allocated arena.
- **Oversize asks** die in the driver's uncatchable VmHeap assert, not a returnable error — the pool-growth gate at the alloc choke refuses growth past `min(hip_free, sysfs_vram_free) − pool_slack − 1GB` with a clean `HipError(2)`. Regression test: `tests/suite/oversize_oom.rs` (only acceptable end is `try_alloc_bytes → None`; signal 6 = the bug).
- **hipMallocAsync/hipFreeAsync only** — hipFree does an implicit full-device sync per drop.

## 8. Inference Engines

### 8.1 Runtime strategy (applies to every downloaded model)

First-principles layering: low-level ops compose into kernels → kernels dispatch through clean functions → those functions use the exact optimal memory strategy for the hardware. Two strict layers:

1. **General GPU lib = gpu-core (f64), user-API-composable.** Everything reusable: GEMM (`gpu_gemm_bt` matches the `w[o*in+i]` weight layout, no transpose), RMSNorm, RoPE (per-head/GQA, partial rotary), GQA attention+softmax, GELU/SiLU, embedding gather, MoE routing, diffusion sampler primitives, tiered streaming.
2. **Per-model custom module = only that model's weird wiring** (for gemma4: self-conditioning MLP, exact norm placement, per-phase output scales, denoise schedule, mask-token handling). Generalize a component into gpu-core only after 2 model modules share it — never pre-generalize from one model.

Weights: load safetensors bf16 and widen every value to f64 on GPU — no quantized path on the GPU side (low-precision source values fill in during f64 accumulation). Loader: `recipe-infer/src/safetensors.rs`.

### 8.2 gemma4 (diffusion-gemma-26B) — current engine

State: full 30-layer forward + self-conditioned diffusion sampler on GPU in f64 from 11 bf16 safetensors; forward is 100% our .hip kernels (custom `gemm_bt_f64` beats the vendor GEMM on 5/6 shapes); one-claim VRAM lifecycle (ledger: allocs 1, frees 1, blocking 0); waterfall weight homes VRAM(~10GB slab)→RAM(~18GB)→DISK; bit-deterministic across runs; ~0.98 tok/s. Per-run budget: disk-tier expert reads ~27.5s (RAM maxed, no page cache), per-expert sync roundtrips ~12.5s (batch expert outputs on GPU + one download/layer is the next lever), routing ~2s.

Architecture facts (source-verified): FULL-attn layers (every 6th) have head_dim 512, 2 kv-heads, partial rotary 128/512, NO v_proj (v = k_proj then scale-less rms_norm, no rope); sliding layers 256/8. kq_scale = 1.0 (not 1/√d). Norms are plain `x̂·w` (the +1 is folded into stored weights). Residual: `h_next = (post_ffw_norm(mlp+moe) + attn_out) * out_scale` — scaling the sum, not just the FFN branch. Final logits: output_norm → tied token_embd → softcap 30·tanh(x/30). Diffusion: encoder phase = prompt, causal; decoder phase = 256-token canvas, bidirectional, input = scale-less rms_norm(scaled_embed(mask) + self_cond_mlp(soft)) where soft = Σ top-8 prob·emb·√n_embd (distribution-weighted; hard-token self-cond collapses). Temperature top-50 sampling (1.0→0.3) with seeded per-(step,position) PRNG breaks the all-mask symmetry; the mask-signal token id 242122 is excluded from candidates. CPU exploration engine at `~/Desktop/gemma4/rustgemma/` (f32/gguf — exploration only; don't follow its precision or quant path, and don't run external reference binaries like llama-cli).

## 9. Kernel Development Guide (gpu-core)

- **One module per domain**: each `src/<domain>.rs` owns its `extern "C"` FFI block + `gpu_*` wrappers; each `src/kernels/<domain>.hip` owns its `launch_*` fns. New modules depend only on public `gpu_*` + `GpuBuffer` — never on each other. Shared-file edits (build.rs, lib.rs, Cargo.toml) are quarantined to setup/integration phases, never done during parallel agent fan-out.
- **FFI parity is the silent killer**: C linkage matches symbol names only. A dropped/transposed parameter links green and corrupts at runtime. Check per-slot type parity between every `.hip` launcher and its Rust decl. The three failure classes compile+link+parity cannot catch: column-major BLAS fed row-major data (transpose to col-major first, swap outputs), shared-memory sync races, and algorithm logic — only runtime tests catch these.
- **After adding a NEW .hip file**, `touch` an existing one so build.rs re-discovers the set. Duplicate-symbol link errors have two causes: stale old-named `.o` in OUT_DIR (delete them + libhipkernels.a, touch a .hip; `cargo clean -p gpu-core` also works but recompiles everything) or a genuine duplicate `extern "C"` definition across two .hip sources (delete the redundant source copy).
- **rocPRIM in .hip files**: `#include <rocprim/rocprim.hpp>` AND `#include <cstring>` (without cstring, hipcc fails in texture_cache_iterator). Device temp is caller-owned: expose `*_workspace_bytes(...)` (query with nullptr temp) + a launcher taking `(tmp, tmp_bytes)`; the Rust wrapper allocates the GpuBuffer. `reduce.hip` is the working reference.
- **Library handles pin to the null stream** (kernels.rs) — a handle on its own stream races our kernels.
- **Proof suite**: `tests/suite/` — `inventory_proof.rs` loads `kernel_inventory.db` (repo root, committed, ~3.7MB SQLite, ~13k named kernels across 6 vendor sources) and proves each mapped `gpu_*` op against a CPU/library oracle; `prove_<cat>.rs` per category. The db is live test data — ~20 tests fail without it (SUITE SPEC R10: a test reads only committed or self-created files; env-var paths banned; any #[ignore] reds the suite). To extend coverage: register an op + oracle + canon alias, or implement the gap kernel and register it. Test invocation needs `-p gpu-core`; a .hip change relinks many big test binaries (~10 min, not a hang).

## 10. Hardware & System Reference

- **CPU**: Ryzen 5 7600X, 6c/12t (`nproc`=12). **GPU**: RX 7700 XT, **gfx1101** (RDNA3, Navi 32), 54 CU, 12GB GDDR6, wavefront 32. Roofline: **f64 peak 1.099 TFLOP/s** (1/16 rate — a hard floor; speed levers are overlap/algorithms, not precision), **memory BW 432 GB/s**, ridge ≈2.54 FLOP/byte; f64 reductions (~0.125 FLOP/byte) are bandwidth-bound. Sibling 7800 XT is also gfx1101 (60 CU, 624 GB/s) — confirm via cu_count before using numbers.
- **OS**: Arch Linux. ROCm 7.2.x via pacman; `/opt/rocm` symlinks to `~/.rocm-install/rocm` (root partition is tight; home has 800GB — build targets run 16GB+). hipcc = `/opt/rocm/bin/amdclang++ -x hip --rocm-path=/opt/rocm`. GPU busy: `/sys/class/drm/card1/device/gpu_busy_percent`. In rocminfo the CPU is Agent 1 and the GPU Agent 2 — don't head-truncate the output and miss it.
- **`dev` wrapper** (Nate's bashrc): `dev <cmd>` interleaves a 1Hz `CPU/RAM/GPU/VRAM` line with the command's output. From a non-tty shell: `script -qec 'bash -ic "dev <cmd>"' /dev/null`. Use it to verify every perf-relevant change: tiers at top−1GB, no alternating-util sawtooth, no low-CPU phase while RAM is moving (a RAM climb at 8% CPU = a single-threaded memory path to fan out).
- **rocprofv3** hangs at teardown — always `timeout 8 rocprofv3 --hip-trace --kernel-trace --memory-copy-trace -f rocpd -d <dir> -- <bin>`. The SQLite DB is written incrementally and survives the kill (`engi/<pid>_results.db`; query the unsuffixed views; 8-byte pinned D2H copies don't appear in memory-copy-trace — count hipMemcpy vs hipMemcpyAsync in `rocpd_region`).
- **Crash forensics** (read-only, always fine): `coredumpctl list` / `coredumpctl info <PID>` for the userspace backtrace (the aborting thread tops out in libhsa-runtime64; another thread shows the real chain). `journalctl -k` is THE decisive source for GPU faults: `amdgpu … page fault` + `sq_intr: error type 2` = a shader wrote an unmapped page, surfaced at the NEXT HIP call — NOT out-of-memory; the abort frame is where the async fault surfaced, not where it originated. Don't rebuild over a binary before inspecting its core (invalidates symbols). `MALLOC_CHECK_=3` forces abort at first heap corruption; `gdb -batch -ex run -ex bt` (run as gdb's child — yama blocks attach). Other useful ctls: `oomctl`, `zramctl`, `smartctl`, `systemctl`.

## 11. Design Invariants (settled; not open questions)

- **f64 only.** No fp32/fp16/bf16/TF32/mixed precision anywhere, in any layer, ever — not even as a listed option. All numeric compute is `Array2<f64>`; the GPU path is f64. The ~32× consumer-silicon fp64 penalty is accepted; any perf estimate assuming fp32-class throughput is wrong by that factor.
- **Full-batch only.** No batch_size, mini-batch SGD, gradient accumulation, or chunked eval. M = full sample count is intrinsic; memory sizing holds full-batch activations by design.
- **No caps, no magic thresholds.** Never shrink classes/iterations/cardinality/seq-len/ranges to fix size or speed — accelerate the code or fail clean (print size + culprit columns and exit; a budget check that prints-and-exits is fine, a silent encoding change is not). The optimizer/search sees the full problem.
- **Vendor math libraries are out.** No rocBLAS/hipBLAS/rocSOLVER/rocFFT/cuBLAS math calls; all GPU math is our own `.hip` kernels (they allocate dark, run opaque stream semantics, and break total accounting). Sole sanctioned exception: exactly 2 cu shim call sites for hip→cu memory alloc (NVIDIA backend) — `rg` for cu matches exactly 2 at end-state. Remaining `hipblas*` calls in kernels.rs are under migration (forward ✓ → training GEMMs → solver/fft → unlink) — never add one.
- **No fallback defaults.** A failed runtime query (device props, capability probe, env) panics with the cause and remedy; never substitute a plausible value (a silent CU-count=1 fallback sizes every launch wrong invisibly). Same family: no CPU fallbacks, no cfg feature gates for GPU (no ROCm → doesn't compile, and that's fine), no clamping hyperparams, no swallowing statuses. Failed model fits return their garbage metrics for the optimizer to learn from.
- **Sampler owns failures** (optimizer context): NaN/div-zero/out-of-range from a sampled config are information — return them, let the pruner kill it. Never crash the study, never clamp, never constrain "might-be-bad" (constraints only for the truly impossible).
- **No 2D/3D conv layers** — grid arrangement is a feature transform (reshape/tile), not a new operation; only 1D conv exists (their kernels exist in convx.hip, unwired by design). **No `.pool()` API** — pooling is a kernel-level concern; downsampling is the conv `stride` param.
- **Resume mismatches crash** with a diagnostic; trained weights are never silently discarded.
- **Saturation laws**: something is always happening at the bandwidth ceiling of whatever front a phase touches; CPU or GPU (or both) pinned during any compute phase; a moving RAM level implies all 12 threads busy; alternating util graphs = serialization to fix with read-ahead/write-behind/stream overlap. The 1 GB/tier user reserve is the only headroom constant.

## 12. Working Style

- **Build the stated spec verbatim.** Novel designs deviate from convention by design; deviation is not evidence of a flaw. Don't substitute the standard implementation, don't normalize, don't pre-fix. If a flaw seems real: build the spec first so it can be tested, then note the concern in one line. A clean failure of the actual spec is more valuable than a success of a substitute — the failure is the experiment. Building something different, making it pass by testing the easy case, is the worst outcome; a no-op is second-worst (zero information).
- **When the spec already decides something, implement it** — don't present options. A workaround that reintroduces exactly what the spec eliminates (e.g. hidden allocation behind a kernel) is wrong by construction, not an alternative. Only genuinely open forks go to the user, one high-level question at a time, never a menu. Don't offer abandon-the-design framings.
- **Report results flat.** Lead with the measured number or exact error, fewest words; state failures as plainly as successes. Explain problems at the highest level (what + why) in one clean sentence, then stop — no jargon dumps, no code refs he didn't ask for, no lists of failure modes, no false either-or choices. Iterating through many failures is the normal state of this work; stay in the loop rather than proposing to reconsider the approach.
- **No unproven claims.** A bug is "open" only with a reproduction on the current artifact (a one-off crash on a stale binary is not a live bug); a fix is "verified" only with the measurement in hand. Capability questions ("do we have X?") are answered by enumerating ground truth with file:line and a verdict taxonomy (wired / primitive-only / name-only / absent) — never yes/no from names, comments, enum variants, or test strings; close obfuscation vectors (separator forms, line-continuation splices) and distinguish declaration from call site. Retract a false-positive grep label immediately and precisely.
- **Fix any bug you hit, regardless of origin.** Zero effort proving it pre-existing; all effort on the fix. Swallowed errors and unchecked statuses get fixed as part of the task — settled policy, not a question. Diagnosing a reported error: write your own repro with your own chosen dataset rather than combing the user's script/data for an input to blame; data values never steer model/feature/threshold decisions (leakage).
- **No scope creep.** Before adding anything, confirm it was requested; a feature with no user-message origin is your own invention — grep the conversation before treating it as a requirement, and delete it rather than defend it. Never implement-then-disable (silent no-ops, gated features); if you build it, it runs. Complete changes everywhere the same data appears (per-step lines + summary + holdout, all call sites).
- **No shortcuts under pressure**: no scope reduction presented as done; no synthetic or hand-rolled test data — real, externally-sourced datasets only (randn/zeros are for weight init, not data); a bug-proving test must FAIL before the fix (a test with no assertion is a printout, not a test); no normalizing targets inside models; never delete/comment/rewrite a failing example — the example is the spec, a failing cookbook entry means the framework is wrong, so fix the framework and don't reason case-by-case from what the CSV happens to hold.
- **Missing capability → build it in Rust.** "No runtime/tool supports this" is not a blocker — a full inference engine, tokenizer, loader, or sim can be written from scratch; scope it and start. (Destructive edits to working code and fabricated results remain off-limits, as always.)
- **Terse, direct execution.** Act on clear directives; no performative agreement or contrition theater (agreement is shown by the changed behavior); no process ceremony; no clarifying questions when context answers them; no question-forms in response to frustration or a broad directive — acknowledge once, act on the most-grounded interpretation. Generalize every correction to its root class so the same lesson never needs re-teaching in a new costume.
- **Crates**: use crate APIs instead of hand-rolling (non-GPU-math only — §11 covers GPU math); check docs.rs before writing a workaround. Read a SQLite schema (`PRAGMA table_info` on every table, views, constraints) before querying any .db. Keep `nates-gpu-ruby/sig/nates_gpu.rbs` in sync with lib.rs (the RBS is the human-readable function reference).

## 13. Verification Discipline

- Real verification = build (`cargo build --release [--example X]`), then RUN the real binary in the **foreground** and read actual output (loss falls, predictions sane, no panic). `cargo test` green is supplementary, never the e2e claim. TUI work: launch via tmux or kwin-mcp, screenshot, confirm every panel updates before calling it done (kwin-mcp with writable=true IS the host filesystem — same binaries, same paths).
- Never run a blocking command that shows no output. Redirect to a file AND tail it (`cmd > /tmp/x.txt & tail -f`, or `tee`) so output is saved and live. In this project don't merge or hide streams — no `2>&1`, no `/dev/null`, no filter-pipes that conceal (training logs go to stderr, results to stdout; merging reads as cherry-picking). Using tail/tee to make silent output VISIBLE is the required opposite. Wrap anything that can hang in a short `timeout` so hangs surface immediately instead of sitting silent.
- Tests ≤60s each. Iterate with op-level probes, load-only checks, or single-step runs — not full multi-GB weight reloads per edit. No multi-hour training runs (throughput ~1.6 epochs/s on the detector corpus: 20k epochs ≈ 3.5h+ = operationally a hang); iterate in short checkpointed runs with `.resume`. If the user manually backgrounds one of my commands, the run is hung or under-utilizing — stop launching long blockers.
- One GPU process at a time (concurrent runs OOM at weight init; the GPU is also the resource the user is using hands-on — don't launch surprise long GPU tests). Verify perf changes under `dev` (§10).

## 14. Processes, Git, Agents

- **Processes**: leave anything you didn't spawn alone — no kill/pkill/pgrep/rocm-smi remediation, no diagnosing external VRAM/RAM contention (an OOM is almost always this code's own allocation: print the size and exit). Exception, standing instruction: a runaway process you spawned yourself pegging the GPU — kill it immediately yourself. coredumpctl/journalctl forensics on dead processes are read-only and always fine.
- **Git**: work directly on master; branch only for genuinely experimental work. At the end of any turn with edits, commit the whole working tree and push (standing instruction — including files you didn't create). If a revert is ever needed, stash first. `datasets/` and `kernel_inventory.db` are gitignored.
- **Subagents never run git** — worktree isolation can silently fail and land the agent in the shared checkout (one such agent committed 30M dataset lines and switched the branch). Every file-editing agent brief includes: no git commands at all; edit files, leave everything uncommitted; the main session does all git and verifies repo state itself with `git status`.
- **Subagent-first**: dispatch implementation/research/testing to background agents and keep the main thread free for conversation. Use Opus 4.8 for subagents in this project (weaker models flail on the gpu-core surface); reviews/design audits/adversarial verification always run on Opus with max thinking. Feed agents the accumulated context (kernel signatures, file:line, the plan) so they don't re-derive it. For any large scattered build: fan out parallel background read-agents to map the whole space FIRST (conclusions + file:line, not file dumps), then write the engine once with complete information — the anti-pattern is guess → inline attempt → stall at the first wall. A first approach failing is data, not a wall; have approach #2 ready.

## 15. Side Projects & External References

- **nates-gpu-ruby** (workspace-excluded): build with `cargo build --release --manifest-path nates-gpu-ruby/Cargo.toml`; `touch` edited gpu-core sources first (mtime detection can miss tool edits) and confirm `Compiling gpu-core` in output. Ruby loads via the named symlink (`require ".../target/release/nates_gpu.so"` — the filename determines the `Init_*` entry symbol). `define_module_function` registration means both `NatesGpu.fn` and bare calls after top-level `include NatesGpu` work (watch the short names sum/mean/max shadowing). API shape: `upload/download`, `linear(_backward)`, `gemm(a,b,"T","N")`, `sgd_update`, reductions, `solve/cholesky`, Philox RNG. Design: "Fisher Price ML" — if Ruby goes >2 tabs deep, a fused primitive is missing; write it in Rust.
- **nates-gpu-lua**: Lua model functions are pure math recipes — all inputs explicit (data + weights + hyperparams), predictions out; no init/seed/state inside; explicit primitives over convenience wrappers.
- **Kaggle S6E4** (irrigation, balanced accuracy): `kaggle_s6e4/solve_v2.rb`, 5-fold OOF, GPU-GBM+ResNet+XGB+LGBM+CatBoost per fold; best LB 0.97102.
- **arc3/** crate: ARC-AGI-3 interactive-RL agent (own workspace, path-deps gpu-core). Gateway REST harness (`/api/cmd/RESET|ACTION1..7`, 64×64 layered frames, values 0–15). Settled design: LSTM = the learner/policy (fixed update budget); MANN = the meta-learner outputting the LSTM's update controls (lr, clip, exploration temp, update gates), trained on post-adaptation performance; train LSTM → freeze contract → train MANN. Built in this Rust/GPU stack, not the official Python agents.
- **Python original**: `/tmp/nates_recipe-V2/` (Optuna TPE-era AutoML; the Rust optimizer port reached feature parity).
- **sentry host**: separate benchmark box (2×Tesla M60 sm_52 + AMD V340; CUDA 11.7 + g++-11 for sm_52; patched CatBoost/XGBoost GPU builds under `~/Desktop/bench/`).

## 16. Conventions

Edition 2024, stable toolchain. 6-space indentation. `anyhow::Result` for fallible fns; `#![deny(clippy::unwrap_used)]`. Lowercase const aliases (`mse`, `bce`, `w`, `b`, `Loss`, `R2`). Activations as chained methods. Progress/diagnostics to stderr, never stdout. Comments minimal: no prose/explanation blocks, no doc paragraphs, no arrows or em-dashes inside `//` comments (`logit to prob`, not `logit → prob`); one-line box-drawn section dividers (`// ── section ──…`) are good. The code is the spec.

## 17. Active State & Roadmap (2026-07)

- **gemma4**: ~0.98 tok/s, fully custom kernels, deterministic. Next levers, in order: llama.cpp-style `build_norm/attn/ffn/moe_ffn` adapters generalized into recipe-infer (quirks become config values); batch MoE expert outputs on GPU + one download per layer (kills ~9s of per-expert roundtrips); disk-tier expert read overlap (~28s). Residual: cross-process test-suite churn faults ~25% at teardown (driver-level, loud never silent).
- **Backend→user-API gap**: ~40 distinct builder decisions would cover the unwired backend capabilities (optimizers, activations, norms, losses, forest/boost, classical ML, sequence, VAE/diffusion, linalg like `Train::closed_form()`/`Data::pca()`/`Data::fourier()`). **None are pre-approved** — wiring/forging any needs a per-item yes. Three are absent entirely (PagedAttention block-table KV, fused VAE-ELBO, apriori driver). f32 mirrors and 2D/3D-conv items are by-design exclusions, not pending work.
- **Vendor-lib migration**: forward ✓ → training GEMMs (split-K precedent) → solver/fft users → unlink hipblas/hipsolver/hipfft from gpu-core/build.rs. End-state acceptance: `rg` for cu = exactly 2 (the alloc shims).
