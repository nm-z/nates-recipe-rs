# CLAUDE.md

## What This Is

GPU-native neural-network training framework in Rust: builder API for models, data loading (CSV/ARFF/zip/image dirs), training on AMD GPUs via HIP/ROCm, evaluation. All compute runs on GPU through `gpu-core`; no CPU ML crates. 10-member workspace (every crate in the repo); core DAG:

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

`catboost-rs` / `lightgbm-rs` / `xgboost-rs-broken` are workspace members; every crate in the repo is.

## Build & Run

```bash
cargo build --release                    # thin LTO, links ROCm
cargo test all                           # THE suite: every test in every crate, one OS process per
                                         #   test, 60s SIGKILL deadline each (kill = that test's FAIL,
                                         #   suite continues), single log at suite.log, verdict:
                                         #   rg '^FAIL' suite.log   (SUITE SPEC v3; tests/all.rs)
cargo test -p recipe-infer --release     # forward/KV-cache/ogdl behavioral tests (GPU)
cargo test -p gpu-core --release --test suite   # kernel proof suite alone (target renamed from `all`;
                                         #   no test id may contain substring "all" — cargo filter)
cargo run --release -- train.csv --target Price
cargo run --release -- detect <path>     # or ./target/release/detect <path> (standalone bin)
cargo run --release --example cookbook   # the e2e: NN/CNN/MLP/LLM scenarios
cargo run --release --example train_detector   # retrains → pantry/detector.ogdl
cargo run --release --example gemma4 -- "prompt"  # gemma-26B f64 inference
```

ROCm default `/opt/rocm` (symlink to ~/.rocm-install/rocm); overrides: `ROCM_PATH`, `GPU_ARCH` (default gfx1101), `HIPCC`. Datasets live in `datasets/` — tracked via **Git LFS** (`.gitattributes`: `datasets/** filter=lfs`), not gitignored. A fresh `git clone` smudges ~3.3 GiB of dataset blobs; a compute/serve node that only needs source can skip them with `GIT_LFS_SKIP_SMUDGE=1 git clone/pull` (leaves LFS pointer files, ~110 MB tree). Edit examples and `/home/nate/Desktop/train.rs` in place — every new crate target or new -Zscript file triggers a full ~15-min rebuild.

## User API

Three import styles, one crate, the same methods — only the door differs. Everything
after the constructor is dot chaining. Constructors return an OWNED value and every
builder method consumes and returns it, so the chain's result is what the `let` binds
and drops. **No `Box::leak` anywhere** — a loop building 1000 models holds one at a time
(pinned by `builders_are_owned_not_leaked`, an RSS test over 50k build cycles).

```rust
use recipe::*;                                              // style 1: static
recipe.data("train.csv").split(0.8).target("Price");
recipe.model().layer(64).leak().layer(1).loss(mse).lr(0.001);
recipe.train().epochs(100).run(&data, &model);
recipe.eval(&model, &data);

let data  = Data::load("train.csv").split(0.8).target("Price");   // style 2: struct
let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.001);
Train::new().epochs(100).log([Loss, R2, Lr]).run(&data, &model).save(());
model.eval(&data);                            // SavePath: () = "model.ogdl"

let data  = recipe::data("train.csv").split(0.8).target("Price"); // style 3: free fn
recipe::train().epochs(100).run(&data, &recipe::model().layer(1));
recipe::eval(&model, &data);
```

`recipe` is a unit static (value namespace), so `recipe.data(…)` and `recipe::data(…)` coexist.

- `Data`: `Data::load(path)` then `.set() .test() .split() .exclude() .target() .datasets()` — lazy; `.target()` only describes, `Train::run` materializes+frees per run.
- `Model`: `.layer(n)` + chained activation (`.relu() .leak() .gelu() …`), `.layer(embed(dim))`, `.layer(attn(heads))`, `conv(filters, kernel, stride)`, `.eval(&data)`. `ModelInner` is crate-private; `eval` lives on `Model`, in `model.rs` beside it.
- `Train`: `.epochs() .log_every() .log() .plot() .net() .resume(path)`; `.run(&data, &model)` takes both explicitly (`data` is any `&dyn RunData`), takes `&self` and returns `&Train` so `.save(path)` chains off it. Resume shape mismatch crashes with a diagnostic, never silently reinitializes. Preflight checks VRAM/embed/loss before GPU work; over-ceiling scenarios skip gracefully.
- A forward-only pass is a run: it **adopts** the previous run's parked arena (rewind + memset, no free), rebuilds weights from the host mirror, and parks it again. A fit leaves `arena_remaining() == 0`, so it cannot carve otherwise.
- Consts: losses `mse mae huber ce bce focal`; metrics `Loss Accuracy R2 Lr Epoch Time hip`.

## Hard Design Invariants (settled; not open questions)

- **f64 only.** No fp32/fp16/bf16/TF32/mixed precision anywhere, ever — not even as a suggestion. The fp64 GEMM cost is the fixed compute floor; speed comes from overlap, stall removal, and algorithms.
- **Init lifecycle: 2 blocks per device per run.** One arena claim + one staging upload at init, one free at exit; everything between is 100% async tier-to-tier transfers (VRAM/RAM/DISK/NET). No allocations and no frees after load — the freeAsync VA-reuse crash class is structurally impossible when nothing is freed mid-run.
- **No keepalive.** If the card is idle 5 seconds during training, something is fundamentally broken — a heartbeat doesn't fix whatever caused the work to stop, it masks it. Defensive code against a failure mode the design should prevent is banned.
- **No warmup.** The 1GB alloc/zero/free existed to force early driver page-mapping; the arena claim is larger and does the same thing itself. Delete, don't pre-touch.
- **Normalize on device, in place.** No fresh allocations after load. mean/std stay on device — the GPU computed them; eval uses the same GPU path. Stats cross to RAM exactly once, in the exit/save block.
- **Features + targets are ONE matrix.** A CSV row contains both, so they ride one upload; the GPU treats the target columns as a subregion of the same buffer. Separate uploads double the transfer calls for no benefit. (Parsing logic for the combined matrix is currently incomplete — finish it, don't split the upload.)
- **Full-batch only.** No batch_size, no mini-batch SGD, no gradient accumulation, no chunked eval. Tall-skinny GEMM shapes are intrinsic.
- **No caps or magic thresholds.** Never shrink classes/iterations/cardinality/seq-len to "fix" size or speed; accelerate the code or fail clean (print the size + culprit columns and exit). Type detection uses only binary structural tests (parses-or-not, distinct==total) — no picked constants.
- **Vendor math libraries are out.** No rocBLAS/hipBLAS/rocSOLVER/rocFFT/cuBLAS calls; all GPU math is our own `.hip` kernels. Sole exception: exactly 2 cu shim call sites for hip→cu memory alloc (NVIDIA backend). The ~57 remaining `hipblas*` calls in kernels.rs are violations under migration — never add one. Precedent: custom split-K dW beat rocBLAS 7.8×.
- **Memory ledger.** Exactly ONE call site per HIP memory API (alloc/free/xfer/memset choke points in `gpu-core/memory.rs`); every byte tagged and ledgered (live+peak per tag, cumulative H2D/D2H/D2D). Never add a raw `hip::` memory call elsewhere. "Library-internal, can't measure" is not accepted reasoning — vramspy + fdinfo close the gap.
- **Waterfall placement.** VRAM fills completely before one byte pools in RAM; RAM before disk. Exactly 1 GB per tier is the user's reserve (`USER_GB`) — the only headroom constant; ratio floors are out. All fit decisions go through `gpu_core::memory::claimable_bytes()`, never raw `hipMemGetInfo`.
- **Blocking-op budget:** async transfers/allocs/frees unlimited; max 2 blocking allocations + 2 blocking frees per run; blocking transfers unlimited.
- **`hipMallocAsync`/`hipFreeAsync` only** (sync variants rejected at compile time by the root build.rs scanner; hipFree does an implicit device sync). Never `hipDeviceReset()` at exit (double-frees with HIP's own atexit dtor). `Scratch::drop` must `device_synchronize()` first (frees racing an in-flight GEMM page-fault the next phase). Allocate pool/arena memory BEFORE any VMM reservation (tiered.rs) or the pool pointer faults.
- **Saturation:** during any compute phase, CPU or GPU (or both) should be pinned; a moving RAM level implies all 12 threads busy; alternating util graphs (GPU 100%/disk 0% then the reverse) indicate serialization to fix with read-ahead/write-behind/overlap.
- **No 2D/3D conv layers** (grid arrangement is a feature transform; only 1D conv exists). **No `.pool()` API** — pooling is kernel-level; downsampling is the conv `stride` param.
- **Text/high-cardinality columns are feature-hashed** (tokenize → hash mod D → fixed-width counts). Never `.exclude()` them, never collapse to frequency, never cap cardinality.
- **NaN policy lives in ONE adapter** (`pantry::encode::nan_clean` via `clean_dataset` at prepare): features mean-impute, only NaN-target rows drop. No scattered NaN handling.
- **No fallback defaults.** A failed runtime query (device props, capability probe) panics with the cause; never substitute a plausible value. Failed model fits return garbage metrics for the optimizer to learn from — never clamp, never hide.
- **Training loop:** load once, N kernels in a loop, zero allocations inside (AllocGuard), no VRAM↔RAM roundtrips (single 8-byte metric scalar download only). Out-of-core fit (`src/utils/ooc.rs`) streams windows VRAM→RAM→DISK with read-ahead/write-behind from a preallocated page-touched host pool; math stays full-batch.

## Working Style

- **Build the stated spec verbatim.** Novel designs deviate from convention by design; deviation is not evidence of a flaw. Don't substitute the standard/conventional implementation, don't normalize, don't pre-fix. If a flaw seems real: build the spec first so it can be tested, then note the concern in one line. A clean failure of the actual spec is more valuable than a success of a substitute — the failure is the experiment.
- **When the spec already decides something, implement it** — don't present it as options. Only genuinely open forks go to the user, one high-level question at a time, never a menu. Don't offer spec-violating "alternatives" or abandon-the-design framings.
- **Report results flat.** Lead with the measured number or exact error, fewest words; state failures as plainly as successes. Explain problems at the highest level (what + why), then stop — no jargon dumps, no theatrical framing, no lists of failure modes, no false either-or choices. Iterating through many failures is the normal state of this work; stay in the loop.
- **No unproven claims.** A bug is "open" only with a reproduction on the current artifact; a fix is "verified" only with the measurement in hand. Capability questions ("do we have X?") are answered by enumerating ground truth with file:line and distinguishing declared / primitive / wired — never from names, comments, or test strings. Never assert %-of-peak without GFLOP/s ÷ hardware-peak arithmetic from actual dispatch dims.
- **Fix any bug you hit, regardless of origin.** Zero effort on proving it pre-existing; all effort on the fix. Swallowed errors and unchecked statuses get fixed as part of the task (settled policy). Diagnosing a reported error: write your own repro with your own dataset; the user's data values never steer design decisions.
- **No scope creep.** Before adding anything, confirm it was requested; a feature with no user-message origin is your invention — delete it, don't defend it. Never implement-then-disable (cfg gates, silent no-ops); if you build it, it runs. Complete changes everywhere the same data appears (per-step + summary + holdout).
- **No shortcuts under pressure:** no scope reduction presented as done, no synthetic/hand-rolled test data (real datasets only), no tests that can't fail (a bug-proving test fails before the fix), no normalizing targets inside models, no removing a failing example — a failing cookbook example means the framework is wrong; fix the framework.
- **Missing capability → build it in Rust.** "No runtime/tool supports this" is not a blocker; scope the from-scratch implementation and start. (Destructive edits to working code and fabricated results remain off-limits, as always.)
- **Effort ceiling is 4x, not 1x.** When resolving a request costs more than specified, keep going — up to 4x the specified lengths — before ever coming back with a question. 2x is EXPECTED overrun: unexpected details, unforeseen blockers, refactoring of existing untargeted logic. 4x is the infrastructure not supporting the request: things must be moved or partitioned to allow it — still just do it. Ask ONLY when the resolution is unequivocally past 4x: the architecture contradicts the request, or a fundamental unspecified design is required (e.g. making `ogdl()` work without the bang would mean writing an interpreter, ~18x — that's an ask). Never stop at 1x to present sub-1x options.
- **Terse, direct execution.** Act on clear directives; no performative agreement, no process ceremony, no clarifying questions when context answers them, no question-forms in response to frustration or broad directives. Generalize every correction to its root class so it never needs re-teaching.
- **Crates:** use crate APIs instead of hand-rolling (non-GPU-math only — the vendor rule covers GPU math). Read a SQLite schema (`PRAGMA table_info` on every table) before querying. Keep `nates-gpu-ruby/sig/nates_gpu.rbs` in sync with lib.rs.

## Verification & Running Things

- Real verification = build, then run the real binary in the **foreground** with visible output; `cargo test` green is supplementary, never the e2e claim. TUI work is verified with kwin-mcp screenshots before claiming done (kwin-mcp with writable=true IS the host filesystem).
- Never run a blocking command that shows no output; redirect to a file AND tail it. Don't merge or hide streams in this project (no `2>&1`, no `/dev/null`, no output-filtering pipes that conceal; using `tail`/`tee` to make silent output visible is required). Wrap anything that can hang in a short `timeout` so the hang surfaces immediately.
- Tests ≤60s each — enforced by the `cargo test all` orchestrator (per-test SIGKILL at 60s = that test's FAIL; the suite continues; never wall-clock-kill the suite itself). Iterate with op-level probes or single-step runs, not full multi-GB reloads. No multi-hour training runs — iterate in short checkpointed runs; a run with no bounded ETA is a hang. If the user backgrounds one of my commands, that's a signal to stop launching long blockers.
- One GPU process at a time (concurrent runs OOM at weight init). Wrap heavy commands in Nate's `dev` function for live 1Hz CPU/RAM/GPU/VRAM metrics: `script -qec 'bash -ic "dev <cmd>"' /dev/null`. After perf changes, verify under `dev`: tiers at top−1GB, no util sawtooth, no low-CPU phase while RAM moves.
- rocprofv3 hangs at teardown — always `timeout 8 rocprofv3 --hip-trace --kernel-trace -d <dir> -- <bin>`; the SQLite DB survives the kill (query `rocpd_kernel_dispatch`; workgroups = grid/workgroup dims).
- GPU crash forensics (read-only, always fine): `coredumpctl list/info <PID>` for the backtrace, `journalctl -k` for amdgpu page-fault vs OOM truth. Don't rebuild over a binary before inspecting its core.

## Processes, Git, Agents

- Leave processes you did not spawn alone — no kill/pkill/pgrep/rocm-smi remediation, no diagnosing external VRAM/RAM contention. On OOM: print the size and exit; the cause is almost always this code's own allocation. Exception (standing instruction): a zombie you spawned yourself pegging the GPU — kill it immediately yourself.
- Work directly on master; branch only for genuinely experimental work. At the end of a turn with edits, commit the whole working tree and push (standing instruction). If a revert is ever needed, stash first.
- Dispatched subagents don't run git, ever (worktree isolation can silently fail; an agent once committed 30M dataset lines and switched the checkout's branch). Agents edit files and leave them uncommitted; the main session does all git and verifies with `git status` itself.
- Subagent-first: dispatch implementation/research to background agents and keep the main thread free. **Opus 4.8 ONLY for every subagent and every workflow agent** — pin `model: "opus"` explicitly on every Agent spawn and every Workflow `agent()` call; never let a worker inherit the session model (Fable 5 included — Fable is the orchestrator, never the worker). Reviews/verification always Opus, max thinking. This is not a per-session preference; do not re-decide it. For large scattered builds, fan out parallel background read-agents to map the space (conclusions + file:line) BEFORE writing code; a first approach failing is data, not a wall — have approach #2 ready.

## Hardware & Environment

- Ryzen 5 7600X (6c/12t), **RX 7700 XT gfx1101** (RDNA3, 54 CU, 12GB): f64 peak **1.099 TFLOP/s** (1/16 rate — hard floor), memory BW **432 GB/s**, ridge ≈2.54 FLOP/byte. Arch Linux; ROCm 7.2.x via pacman; hipcc → `/opt/rocm/bin/amdclang++ -x hip --rocm-path=/opt/rocm`. GPU busy: `/sys/class/drm/card1/device/gpu_busy_percent`. Root partition is tight; home has space (build targets are 16GB+).
- Remote boxes reachable via ssh-config aliases (no IPs in this file): `ssh archy` — Precision Tower 5810, Xeon E5-2620 v3 (12t), 24GB RAM, 4× Tesla M60 + Radeon 540, Arch. `ssh sentry` — Precision Tower 7810, Xeon E5-2643 v4 (24t), 32GB RAM, Radeon Pro V340 (MI25×2), Arch. Quick check: `ssh <host> 'neofetch'`.
- Original Python AutoML reference: `/tmp/nates_recipe-V2/`. CPU gemma exploration engine: `~/Desktop/gemma4/rustgemma/` (f32/gguf, exploration only — the GPU path is safetensors→f64; don't run external reference binaries like llama-cli).
- `nates-gpu-ruby` is workspace-excluded: build with `--manifest-path nates-gpu-ruby/Cargo.toml`, `touch` edited gpu-core sources first, confirm `Compiling gpu-core` in output. Load from Ruby via the `nates_gpu.so` named symlink (entry symbol lookup requires that name).

## Gotchas

- `kernel_inventory.db` (repo root, committed, ~3.7MB SQLite) is live test data for the gpu-core proof suite — ~20 tests fail without it (SUITE SPEC R10: a test reads only committed or self-created files; env-var paths banned; any #[ignore] reds the suite).
- gpu-core kernels compile through the `cc` crate (parallel feature, jobserver-shared): build.rs sweeps every `.o` + `libhip*.a` from OUT_DIR then recompiles the full set fresh, so a duplicate-symbol link error is always a genuine duplicate `extern "C"` definition across two .hip sources (delete the redundant source copy). `cargo:rerun-if-changed=src/kernels` watches the directory — new .hip files are discovered without touching anything.
- rocPRIM in .hip files needs `#include <rocprim/rocprim.hpp>` AND `#include <cstring>`; device temp is caller-owned (`*_workspace_bytes` query + launcher taking tmp) — see reduce.hip.
- FFI parity is the silent killer: C linkage matches names only; check per-slot type parity between every `.hip` launcher and its Rust decl.
- Detector retrain: delete `pantry/detector.ogdl` first on a corpus change (best-only save guard blocks incomparable scores), and rebuild after training (`include_str!` bakes weights at compile time).
- hipMallocAsync pool growth can wedge (HSA spin) and fresh pool pages can read back as stale zeros: one slab + one memset + canary + loud watchdog is the pattern (`waterfall.rs`).

## Conventions

Edition 2024, stable. 6-space indentation. `anyhow::Result`; `#![deny(clippy::unwrap_used)]`. Lowercase const aliases (`mse`, `w`, `b`). Activations as chained methods. Progress/diagnostics to stderr, never stdout. **Comment ban** (2026-07-18 law, supersedes budgeted comments): the ONLY comments allowed are `// SAFETY:` on unsafe blocks (required there, max 1) and `// ─` separator lines. Everything else is banned: `///` and `//!` doc comments, `/* */`, every other `//`. Attributes (`#![...]`, `#[...]`) are not comments and stay. Enforced by `h03_comment_ban` in `tests/hygiene/cases.rs` (lexer-based). Naming and structure carry ALL the meaning. **Commit messages are banned too** (2026-07-18): every commit is made with `--allow-empty-message -m ''`; the diff is the record.

## Active State (2026-07)

- gemma4 engine: forward is 100% our .hip kernels (custom `gemm_bt_f64` beats hipBLAS on 5/6 shapes), one-claim VRAM lifecycle, waterfall weight homes, deterministic; ~0.98 tok/s. Next levers: llama.cpp-style `build_*` adapters generalized into recipe-infer, batching MoE outputs on GPU (per-expert roundtrips ~9s), disk-tier expert reads (~28s).
- Backend→user-API gap: ~40 distinct builder decisions cover the unwired backend capabilities; **none are pre-approved** — wiring/forging any needs a per-item yes. Three are absent entirely (PagedAttention block-table KV, fused VAE-ELBO, apriori driver).
- Vendor-lib migration order: forward ✓ → training GEMMs → solver/fft → unlink libs from gpu-core/build.rs.
