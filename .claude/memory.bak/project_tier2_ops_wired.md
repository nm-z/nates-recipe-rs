---
name: project_tier2_ops_wired
description: "RoPE/MoE/diffusion were dead \"internal interfaces\" (commit daf0c08) — now made correct 2026-06-29 — RoPE wired into attention fwd+bwd, MoE got a backward, diffusion got a block-AR loop. All proven by foreground finite-diff tests."
metadata: 
  node_type: memory
  type: project
  originSessionId: c71a10a1-a999-4982-b472-5c12ca8ab0be
---

The "internal interfaces" added in daf0c08 (BPE, safetensors, RoPE, MoE, entropy-gated diffusion) were defined but mathematically incomplete/unwired. Fixed 2026-06-29:

**RoPE (commit ee7749d).** Attention had NO positions (zero bias) → order-blind. Added per-head rotary embedding: new `ropex_qk_heads` kernel (gpu-core/kernels/ropex.hip) rotates WITHIN each head block [h*hd, h*hd+hd) by angle = (row%seq)/theta^(2j/hd), `sgn` arg = forward(+1)/inverse(-1). Wired into `attn_forward` AND `attn_forward_cached` (rotate a_q,a_k in-place after projection, before QK), and `attn_backward` un-rotates a_dq,a_dk (sgn=-1) before the Wq/Wk projection backward (a rotation is orthogonal: dQ = R(-angle)·dQ_rot). Wrapper `gpu_rope_qk_heads_inplace` in rope.rs, ROPE_THETA=10000. Proof: `rope_heads_backward_is_inverse_rotation` finite-diff 2.1e-10; kv_cache_matches_full_attention still 2.5e-13 (both paths rotate). The pre-existing `gpu_rope_qk` (full-dim, returns new bufs) was wrong for multi-head — left as-is, unused.

**MoE (commit 8ed25ec).** `gpu_moe_route` was forward-only. Added `gpu_moe_backward(hidden, gate_w, expert_w, d_out, n, d, E) -> (d_hidden, d_gate_w, d_expert_w)` covering every op: new `moex` backward kernels (d_ye = gate[:,e]·d_out; d_gate[:,e] = Σ_j Ye·d_out), per-expert FFN backward (gpu_gemm_bt/gpu_gemm_at), router softmax backward (gpu_softmax_backward_into), router GEMM backward. Self-contained (recomputes gate + per-expert Ye). Proof: `moe_backward_matches_finite_diff` 4.66e-11 over all three param groups. Still NOT wired as a user LayerKind::Moe — it's a correct primitive op.

**Diffusion (commit 4e64aba).** `gpu_entropy_gated_step` was a single shot, no loop. Added `gpu_diffusion_sample(logits_fn, initial_canvas, bound, max_steps, n, vocab) -> (canvas, steps)`: block-autoregressive loop — iterate the gated step, commit confident positions, FREEZE committed ones (new `diffusionx_commit` kernel), converge when all committed. Takes a logits_fn(canvas) closure (model-agnostic). Proof: `diffusion_block_ar_progressive_commit` — predecessor-gated logits force left-to-right decode, converges in exactly n steps.

Verification discipline this session: every proof is a foreground finite-difference/equivalence test showing real numbers, per [[feedback_foreground_e2e_testing]]. `cargo test -p gpu-core` rebuilds+runs the WHOLE gpu-core test suite (many big test binaries relink after any .hip change) — slow (~10 min), not a hang; the lib unittest (where these live) runs first under "Running unittests src/lib.rs". See [[project_gpu_core_expansion]], [[feedback_commit_before_ending_turn]].
