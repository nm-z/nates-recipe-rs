---
name: project_model_runtime_strategy
description: How model inference runtimes are built — general f64 GPU primitives in gpu-core (user-API-composable) + a thin per-model custom module; generalize only after 2 models share a component
metadata: 
  node_type: memory
  type: project
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

**The philosophy (Nate's words, 2026-07-01):** first-principles build, brute-forcing generalization at each abstraction level. Layering: low-level ops compose into commonly-used kernels → kernels dispatched through elegant functions → those functions allocate the EXACT optimal memory strategy for the hardware (exact amount, or paged, or windowed, or batched — whichever the HW makes optimal). f64-only and VRAM→RAM→disk are the fixed invariants that keep every layer honest. Existing libs are rejected because their "abstractions are only abstract and never — on any layer — turn intuitive": every layer here must stay intuitive and hardware-honest. Extends [[feedback_primitive_design]].

**The architecture strategy for running downloaded models (gemma4, and others coming next):**

1. **f64 everywhere, NO QUANT.** Non-negotiable ([[project_f64_required]]). Requirement was "F64 ONLY" from the start. **Load the 11 safetensors (bf16), widen every value to f64 — do NOT use the gguf quant, do NOT write K-quant dequant kernels.** "NO QUANT IF <16 sig figs, PAD THAT BITCH, IT WILL GET FILLED ON FIRST CALC" — bf16→f64 widening is correct; low-precision source values fill in during f64 accumulation. Input = `/home/nate/Desktop/gemma4/diffusiongemma-26B-A4B-it/model-000{01..11}-of-00011.safetensors`. Loader ALREADY EXISTS: `recipe-infer/src/safetensors.rs` decodes every tensor's bytes to f64 on the host. The gfN/gfO.rs at ~/Desktop/gemma4/rustgemma are f32 CPU exploration — the GPU port is f64 and reads safetensors, NOT gguf. Do NOT follow gfO's f32 or its gguf/quant path.

2. **Two layers, strict split:**
   - **General GPU lib = gpu-core (f64), exposed through the user-facing API.** EVERYTHING reusable lives here: dequant (Q4_K/Q6_K/Q8_0/Q5_0 → f64 on GPU), GEMM (`gpu_gemm_bt` matches the `w[o*inn+i]` weight layout with no transpose), RMSNorm, RoPE (per-head/GQA), GQA attention+softmax, GELU/SiLU, embedding gather, MoE top-k routing, tiered VRAM→RAM→disk expert streaming ([[project_gemma_engine_state]] tiered.rs), AND the diffusion sampler primitives (temperature/inverse-CDF sampling, canvas denoise step). Goal: a general user can COMPOSE a diffusion model from the API. The user cannot read rustgemma — the arch must live in composable API calls, not a golfed file.
   - **Per-model custom module = only the model's weird/specific wiring.** For gemma4: self-conditioning gated MLP, its exact norm placement, per-phase `layer_output_scale`/`enc_layer_output_scale`, the (comb+attn_out)*out_scale residual, the 6-step denoise schedule, mask token id=4, suppress-242122. Nothing reusable goes here.

3. **Generalize only after 2 concrete examples.** Write a custom module per model we encounter. When 2 model modules exist, diff them, lift the SHARED tiny components down into gpu-core. Never pre-generalize arch from one model. (build-2-then-extract; matches [[feedback_primitive_design]].)

4. **gemma4 is model #1.** More tensors (non-gemma) are being downloaded to test against next — so the primitive set in gpu-core must be model-agnostic; the gemma4 module is the only gemma-specific code.

**Capability gaps to add to gpu-core in f64** (verified via capability map): f64 RMSNorm (attention.rs:267 `gpu_rmsnorm` outputs f32 — need f64 gemma-form), GPU K-quant→f64 dequant (experts are 16GB quantized; f64 = 208GB won't fit → must dequant-on-stream per active expert into a VRAM window), GQA-aware attention (existing f64 flash_attn is MHA), MoE top-8 router+gate (moe.rs is dense-all-expert), tanh logit softcap, temperature/inverse-CDF sampler. `gpu_gemm_bt`, `gpu_add/mul/scale`, `gpu_gelu/silu`, `gpu_softmax_rows`, `gpu_gather_rows`, `gpu_rope_qk_heads_inplace` already exist in f64.
