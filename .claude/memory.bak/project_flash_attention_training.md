---
name: project_flash_attention_training
description: Training attention is flash now — no L×L score buffer on ANY path; a_scores/a_dscores replaced by a_lse/a_dsum (n·h·S); LLM jawn need 1680GB→66GB
metadata: 
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

SHIPPED 23b6220. Training attention no longer materializes the n·heads·S² score
matrix (it was 96% of the cookbook LLM jawn's 1680.56 GB preflight — the number
was CORRECT, the implementation was the problem). Both paths are flash now:

- Forward (`flash_attn_f64_train_fwd_kernel`): same online-softmax stream as the
  inference kernel, additionally emits per-row logsumexp `a_lse` [n][heads][S].
- Backward (`gpu_flash_attention_backward_into`, 3 deterministic passes, no
  atomics): dsum rows D=Σ dctx∘ctx → dQ (query-major) → dK+dV (key-major,
  FA_TQ=32 query tiles). P is recomputed per tile as exp(q·k·scale − lse).
- Scratch: `a_scores`/`a_dscores` (n·h·S²) → `a_lse`/`a_dsum` (n·h·S);
  `vram_estimate`/`vram_bytes` updated. LLM jawn preflight: 1680.56 → 66.11 GB
  (rest = eight n·S·d seq buffers ~34 GB + acts; still >12 GB card, aborts
  honestly).
- Proof: `gpu-core/tests/suite/prove_flash_train.rs` — CPU full-softmax oracle,
  ctx/lse/dq/dk/dv all <1e-12 at n=2,h=2,d=8,S=100 (crosses FA_BK=64 partial
  tile + FA_TQ=32 tiles). RoPE unchanged (applied to a_q/a_k before the kernel,
  un-rotated on gradients after).
- Shared-mem ceiling: kernels stage (3·BQ+2·BK)·hd (dq) / (4·BK+2·TQ)·hd (dkv)
  doubles — hd≲24-32 max on 64KB LDS, same class as the inference kernel.

Related: [[project_scratch_ping_pong]], [[project_hip_oom_asserts]], [[project_no_minibatching]].
