---
name: project_userapi_forge_gap
description: "Hardened backend→user-API gap audit — 202 backend capabilities have no builder API (~45 user-API decisions); only 6 ever surfaced, 0 agreed; 3 are absent (build-from-scratch)."
metadata: 
  node_type: memory
  type: project
  originSessionId: 37002afc-9794-4d76-9e65-256d036b31fe
---

41-agent verified audit (surface-map → 20 domain auditors → 20 adversarial verifiers, each grepping the builder to refute verdicts; 19 false positives corrected). Result superseded the old unverified `/tmp/backend-vs-userapi-gap.md` "~36" guess.

**202** distinct backend capabilities (gpu-core/pantry/recipe-infer) are callable but have NO path from the Data/Model/Train builder — every one file:line-verified. **CORRECTION: 202 is INFLATED — 18 f32 dead-mirrors + 9 deferred-by-design 2D/3D (no overlap) = 27 items that should be `internal`, not forge → real forge ≈ 175** (inflation concentrated in attention-rope 14→~2 and conv-pool 15→6). See [[feedback_no_inflate_audit_with_deadcode]] — never count f32/2D-3D-deferred as work. The de-inflated set collapses to **~40 distinct user-API decisions**. **32** wired today, **159** genuine internal/by-design (gemm/reductions, pooling=`.stride()`, 2D/3D-conv deferred, f16 mirrors — NOT forge candidates).

Big unwired buckets: optimizers (16: adam/adamw/momentum/rmsprop/adagrad/lamb/lion/nadam + nesterov/adadelta/radam/lars partial + grad-clip), activations (26: mish/softplus/swiglu/geglu/glu/gelu_exact/…), normalization (LayerNorm f64 fwd+bwd ALREADY complete, +rmsnorm/batchnorm/groupnorm/dropout), losses (11), forest/boost (RF+GBDT+oblivious-catboost+lightgbm-leafwise+GOSS), classical ML (svm-SMO, kmeans/dbscan/hdbscan, naive-bayes ×3, GCN), sequence (crf/hmm/viterbi/dtw/lstm/gru), rl, vae+diffusion, encoding/quant/distance (22), bpe, + linalg gems hiding in "internal": LU/Chol/QR→`Train::closed_form()`, eig/SVD→`Data::pca()`, FFT→`Data::fourier()`, SSM scan→`ssm(dim)` layer.

**3 ABSENT (build-from-scratch, not a wrapper):** PagedAttention/block-table KV (closest = wired FlashAttention KV-cache `forward.rs attn_forward_cached`), VAE-ELBO objective (reparam+KL exist, fused ELBO loss doesn't), full apriori driver (support-count+candidate-gen kernels exist, level-wise driver + rule extraction doesn't).

The 6 ever surfaced to Nate: RoPE, MoE, entropy-gated diffusion, BPE, symmetric-quant, safetensors(wired). **0 of the 202 approved for forging** — do not start forging without his per-item yes. Ties to [[feedback_no_handwave_capability_audits]] (this is the hardened replacement for skim-and-assert), [[project_pooling_kernel_only]], [[project_no_2d_3d_conv]].
