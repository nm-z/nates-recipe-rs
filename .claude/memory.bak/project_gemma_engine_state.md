---
name: project_gemma_engine_state
description: Hand-written Rust diffusion-gemma-26B inference engine — what works, the remaining forward bug, and the exact reference-derived architecture
metadata:
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

**WATERFALL + DETERMINISM SHIPPED (1105a2f):** gpu_core::waterfall places every weight
blob VRAM(one pre-checked slab, ~10GB)→RAM(~18GB)→DISK per [[waterfall-memory-law]];
gemma4's 3 hand pools deleted. DETERMINISM PROVEN: 2 runs bit-identical (all step-0
layer hashes + canvas) at different slab sizes — old variance was stale-zero uploads
([[gpu-wedge-and-staleness]]). 52s/run ≈ 9s/step ≈ 0.92 tok/s. NOT h2d-bound (h2d 7s
@9.5GB/s through bounce): budget/run = DISK-tier expert reads 27.5s (3800 fetches ×
11.9MB, RAM maxed so no page cache) + per-expert sync roundtrips 12.5s (xg.load +
dv.download per expert drains the queue ~1390×/step; batch on GPU + one dl/layer =
next lever) + route 2s. Cross-process suite churn: FIXED structurally — 37 test bins
→ one all.rs harness, 211 tests, 4/4 clean 0.65s. Wedge flavor of the driver race
gets a loud 20s watchdog abort. User directive: generalize adapters llama.cpp-style
(build_norm/attn/ffn/moe_ffn... into recipe-infer, quirks = config values) — NEXT.

**ZERO-ALLOC ARENA SHIPPED (aa3b3f3, follows ledger-law fa28a35):** the forward is
allocation-free in steady state — Arena of 33 named buffers once at setup, `_into` op
variants (gemm_bt/rmsnorm/gqa_attn/gelu_mul/glu_gelu/add), ha/hb ping-pong, ledger proof
"steady-state allocs: 0" printed per run. This made the intermittent hipMallocAsync
churn page-fault UNREACHABLE (was killing 1-in-2 runs mid-run): 3/3 clean, 15s/step
(was 17), deterministic. Preexisting driver races root-caused & mitigated in fa28a35:
SDMA↔gfx-L2 incoherency on reused pool pages (silent wrong gemm!) → disable_sdma_once;
pageable-host blit faults → pinned 64MB bounce in the ONE xfer choke; cross-process
teardown → trim_mempool in gpu_shutdown. Residual: full-test-suite process churn still
faults ~25% (driver-level, ROCm 7.2.4 = repo latest; loud, never silent). LEDGER SAYS
the next perf lever: H2D 120.79GB/run (26848 calls) ≈ 20GB/step ≈ 1.4GB/s effective —
far under PCIe ~25GB/s because the pinned bounce is synchronous per 64MB chunk; async
double-buffered bounce + copy-stream overlap is the >1 tok/s path (now 48tok/90s = 0.53).

**GPU f64 MILESTONE 1 SHIPPED (commit a0a726d, 2026-07-01):** `cargo run --release --example
gemma4 -- "The capital of France is"` in nates-recipe-rs runs the FULL 30-layer forward +
self-cond sampler on GPU in f64 from the 11 bf16 SAFETENSORS (no quant, no gguf) → " Paris."
lead token. 0.5s load, 17s/step, 6 steps 102s = 0.47 tok/s. Design: bf16 shard bytes
`read_exact_at` on demand (OS page cache = free RAM tier; Tiered deferred to milestone 2),
GPU bf16→f64 widen into ONE reusable 184MB VRAM window per hipBLAS DGEMM. New f64 primitives
in gpu-core/infer_ops.rs + kernels/f64_infer.hip (widen, GQA masked attn w/ mixed
causal/bidir prefix mask + kq_scale=1.0, NeoX partial rotary w/ theta+rotary_dim, gelu_mul,
normx_rmsnorm binding); recipe-infer parse_safetensors_header. HF ARCH DISCOVERIES (from
config.json, cost 3 page-faults): FULL-attn layers (every 6th) have head_dim **512**,
**2** kv-heads, partial rotary 128/512 (`partial_rotary_factor`), NO v_proj (v = k_proj);
sliding layers 256/8. rope_type "proportional" on full layers (approximated — suspect for
the canvas-tail degeneration "Paris is Paris is…"); this checkpoint has ONE layer_scalar
(not per-phase enc/dec). Norm probe mean=4.48 → folded x̂·w confirmed on safetensors too.
FRAMEWORK-WIDE fixes: hipblas handle pinned to null stream (kernels.rs:1485 — was racing
widen kernels); gated `memory::set_alloc_sync` (off by default; streaming inference needs
it — SDMA writes fault into lazy-committed hipMallocAsync pages); `init()` retains mempool.
MILESTONE 2 = quality (proportional rope exact, layer_scalar) + perf (kill alloc-sync
serialization via copy-stream+events, pin hot set VRAM+RAM via Tiered instead of page-cache
luck; 52GB re-streamed per forward is the floor: RAM 23GB < 52GB → NVMe every step).

CPU exploration engine: `/home/nate/Desktop/gemma4/rustgemma/` (OUTSIDE the repo). Build:
`rustc -O --edition 2021 gfN.rs -o gfN`. Runs against
`gguf/diffusiongemma-26B-A4B-it-Q4_K_M.gguf` (gguf/quant = CPU exploration ONLY; the GPU
path is safetensors/f64 per [[project_model_runtime_strategy]]). Reference source (read-only,
NEVER run the binary): `/home/nate/Desktop/gemma4/llama.cpp/src/models/{gemma4,diffusion-gemma}.cpp`
and `examples/diffusion-gemma/diffusion-gemma-cli.cpp`. See [[feedback_rust_can_do_anything]].

**WORKS (verified):** GGUF loader; mixed-precision dequant dispatched per-tensor by the
dir's stored ggml type (Q4_K/Q6_K/Q8_0/Q5_0) — NOT hardcoded (the big bug: Q4_K_M mixes
types across layers, e.g. some `ffn_down_exps` are Q8_0, some not → hardcoding gave `inf`);
gemma tokenizer (greedy longest-match on ▁-prefixed vocab); NaN-free 30-layer forward;
12-thread `mv` via `std::thread::scope` (1.9× → ~14–18s/forward for ~16 tokens); per-LAYER
expert dequant cache (per-forward cache OOMs — 13MB/expert × thousands).

**Expert dims (were wrong):** shared FFN intermediate = `ffn_gate.len()/n_embd` (2112);
MoE expert intermediate `n_ff_exp` = 704 (metadata `expert_feed_forward_length`). gate_up_exps
[2816,1408,128], down_exps [704,2816,128], per-expert byte offset = e*tbytes(type, per).

**Reference forward (gemma4 block, all confirmed in source):** attn_norm → Wq(+q_norm+rope)/
Wk(+k_norm+rope)/ V=(wv?Wv·:Wk·, then SCALE-LESS rms_norm, NO rope, NO weight) → build_attn
(kq_scale=1.0, NOT 1/√d — tested, 1/√d is worse) → Wo → attn_post_norm → +residual = attn_out.
FFN: shared[ffn_norm→gelu(gate)*up→down→post_ffw_norm_1] + MoE[pre_ffw_norm_2→top8→post_ffw_norm_2],
router logits=(rms(attn_out)·(1/√n_embd)·ffn_gate_inp.scale)@ffn_gate_inp, softmax→top8→renorm;
combine cur_mlp+cur_moe → post_ffw_norm → +attn_out → ×out_scale. Final: output_norm → tied
`token_embd` logits → softcap 30·tanh(x/30). Only `.scale` tensors: ffn_down_exps.scale (per-expert,
~1.0), ffn_gate_inp.scale (router). Attn q/k/v/o and shared ffn have NO scale companions.

**Diffusion (two-phase, from diffusion-gemma.cpp):** ENCODER = prompt, CAUSAL, no self-cond,
uses `enc_layer_output_scale`, input = scaled_embed (×√n_embd). DECODER = 256 canvas, BIDIRECTIONAL,
uses `layer_output_scale`, input = SCALE-LESS `ggml_rms_norm(scaled_embed + sc)` where sc =
gated-MLP(sc_pre_norm(soft))·, soft = (top-k prev probs @ token_embd)·√n_embd (soft=0 at step 1,
so sc=0 but the rms_norm STILL applies). Sampler: 48 steps, temp 0.8→0.4, ENTROPY_BOUND 0.1,
CONFIDENCE_THRESHOLD 0.005, mask id=4, commit stable+confident tokens, roll canvas K/V each step.

**SOLVED — forward now works (gfG.rs).** Two killer bugs, both fixed:
(1) OUT_SCALE PLACEMENT: reference is `cur = post_ffw_norm(mlp+moe); cur = cur + attn_out;
cur = cur * out_scale` → i.e. `h_next = (comb + attn_out) * out_scale`, NOT `attn_out + comb*out_scale`.
Scaling only the FFN branch let the residual grow unbounded over 30 layers (logits ~700).
(2) NORM CONVENTION: plain `x̂·w` (the gguf HAS +1 folded), NOT `x̂·(1+w)`. My earlier "(1+w) is better"
was an artifact of the out_scale bug masking it. With BOTH fixed: logits sane (~14), predictions
context-driven — "Paris is the capital of"→"Paris"/"countries", "capital of Japan is"→"Japan",
"opposite of hot is"→"hot", "two plus two equals"→"two". Single masked step ECHOES salient context
words (correct for a diffusion model pre-full-loop; it needs the iterative denoise to complete).
Config: plain-w rn, attn scale 1.0, decoder-input scale-less rms_norm, per-phase out-scale,
rope_freqs on full layers, (comb+attn_out)*out_scale.

**COHERENT OUTPUT ACHIEVED (gfL.rs).** "The capital of France is" → " Paris." — hand-written Rust,
26B diffusion-gemma, NO external binaries. The sampler that works:
- Canvas = prompt(encoder,causal) + N mask tokens(decoder,bidirectional).
- Self-conditioning (REQUIRED): each canvas position's decoder input = scale-less rms_norm(scaled_embed(mask)
  + sc_mlp(soft)), where soft = Σ top8 prob·emb(id)·√n_embd (SOFT, distribution-weighted — hard-token
  self-cond collapses). sc_mlp = self_cond_down(gelu(self_cond_gate·scn)*self_cond_up·scn), scn=rn(soft,self_cond_pre_norm).
  Step 0: soft=0 (no self-cond) → all positions predict identical mask-signal → temperature sampling breaks symmetry.
- Temperature top-50 sampling (temp 1.0→0.3 over steps) with a seeded xorshift PRNG per (step,position) — REQUIRED
  to break the all-mask symmetry; greedy argmax collapses to one token.
- SUPPRESS the mask-signal token "掩" (id 242122): the model emits it (and <mask>) when a position is surrounded
  by masks ("still masked"). Excluding it from candidates forces real tokens; self-cond then refines to the answer.
  (Reference does this implicitly via entropy-bound confidence gating.)
- Convergence: step0 noise ("a the マスク mask...") → step1 "Paris." → step2 "Paris." (stable).

**REMAINING: speed only — and it's RAM-bound (MEASURED).** Profiled one 19s forward (gfN.rs, 16 tokens):
expert dequant 9.0s (47%) | expert matmul 3.6s | rest (proj/attn/shared-FFN/logits) 6.4s. Batching q/k/v
projections into one GEMM (mm() over all positions) barely helped → bottleneck is the MoE dequant, not projections.
Cross-step expert caching would remove the 9s from steps 1+, BUT 128 experts/layer × 30 layers dequanted ≈ 46GB
even in f16 > 31GB RAM (with 16GB model resident) → cannot cache; per-step dequant is unavoidable on this box.
CORRECTION (measured, not assumed): the 9s dequant was SINGLE-THREADED — threading deq() over blocks
(deqblock + thread::scope) dropped it 9s→2s. Then batched all matmuls into mm() (all positions per weight,
one threaded region, weights read once): q/k/v/o projections, shared FFN, logits (emb read once not 10×),
and MoE per-expert (gather positions sharing an expert → one GEMM, reuse the dequant). Result: forward 19s→9s
(2.1×), correctness intact (gfN.rs still → "Paris."). Remaining ~5s is diffuse single-threaded work (attention
score/softmax/v loops, per-head q/k norm+rope, ~30k small thread-spawns) — batchable for ~another 2× but a
deeper rewrite. At 9s/forward + fast convergence (France by step 1) that's ~0.5 tok/s; large canvas amortizes
the fixed 2s dequant toward ~1. Still short of a clean >1 on CPU — GPU is the decisive path.
FINAL PROOF (measured): forward = 2.1s fixed + 0.37s/position (fit from t=16→8s, t=54→22s). Threading OVER
experts (vs inside each mm) gave ZERO change (23s vs 22s) → the forward is MEMORY-BANDWIDTH-BOUND: at large
canvas all 128 experts/layer are read = ~90GB dequanted expert weights/forward; CPU RAM BW (~15-30GB/s) is the
floor, no threading beats it. tok/s math: needs exactly K=2 steps × canvas≥33 to exceed 1, but 2 steps = NOISY
output (clean coherence needs 3-4 steps, e.g. hot→cold took 4); K≥3 has per-position cost too high to ever cross 1.
So clean-coherent-AND->1-tok/s are mutually exclusive on this CPU — provably. Engine DOES generate coherent
multi-token sequences (canvas=48: "four six seven eight nine ten" continuing "one two three"). GPU (VRAM 400+GB/s,
tiered streaming since 12GB<16GB model) is the ONLY path to clean >1 tok/s. This is empirical, not assumed.
So forward floors ~13-16s on CPU; a 6-step canvas ≈ 0.1-0.3 tok/s. >1 tok/s REQUIRES GPU weight-streaming
(VRAM 12GB < 16GB model → stream experts on demand via the tiered.rs VRAM→RAM→disk buffer + ROCm f32 GEMM/dequant
kernels). That's the next build: HIP FFI in the engine (or build it inside the nates-recipe workspace to link gpu-core),
per-expert dequant+GEMM kernels, tiered streaming of the active expert set. Correctness is DONE; this is pure perf.

**RULED OUT earlier (all verified, none was the bug):** head-count matches (nqh=16=qd/hd,
GQA 16:8, hd=256, ne=2816, nl=30); mv orientation matches ggml mul_mat (w[o*in+i]=weight[in=i,out=o]);
Q4_K (type12) and Q6_K (type14=token_embd) dequants verified BIT-FOR-BIT vs ggml block structs
(get_scale_min_k4, sc[is+0/2/4/6], qh shifts) — correct; rope_freqs.weight is [256] type0 (applied to
full layers, no change); cosine (direction-only) ranking is ALSO junk → it's a genuine hidden-DIRECTION
error, not an embedding-norm artifact. Forward DOES vary by prompt (France/Japan/math differ) so context
flows, but always lands Korean-biased (어/아닌/불구하고). Every source-derivable fix applied; bug persists.
Conclusion: not isolable by source inspection — needs op-by-op numeric diff vs a reference RUN.

**OPEN BUG:** single mask "The capital of France is <mask>" → predicts foreign junk (어/아니라/
ยิน) not "Paris", under BOTH norm conventions (x̂·w and x̂·(1+w)) and with decoder-input-norm +
per-phase out-scale applied. So a residual numerical bug remains OR the single flat mixed-phase
forward is inadequate (needs true separate encoder-prefill + decoder pass with a KV cache).
Isolating it needs op-by-op numeric diff vs a reference RUN — blocked by the user's
"DO NOT RUN EXTERNAL BINARIES" (llama-cli is the only oracle). Next session: either get the
oracle unblocked, or build the real two-phase KV-cached sampler and judge coherence over the
full 48-step loop rather than a single step. Speed to >1 tok/s needs GEMM-batching (all positions
per weight, weights read once) — current per-position `mv` has thread-spawn overhead.
