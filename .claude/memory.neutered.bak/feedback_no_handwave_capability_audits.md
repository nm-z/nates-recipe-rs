---
name: ""
metadata: 
  node_type: memory
  originSessionId: 37002afc-9794-4d76-9e65-256d036b31fe
---

On capability questions ("do we have BPE / RoPE / symmetric quant / safetensors?") and invariant audits ("are the launchers free of hipMalloc?"), the banned move is a binary implemented-yes/no derived from reading function names, comments, enum variants, struct fields, or test-inventory string literals. He calls this "hand waving" and it's the same root as [[feedback_trust_user_over_priors]] / [[feedback_no_cya_on_results]].

**Why:** a name/comment/test-string/stub is NOT implementation; a narrow `rg` keyword list misses variants, separator/line-continuation/macro obfuscation, and the declaration-vs-call distinction. A yes/no verdict that conflates these is worse than useless — it hides the truth he's probing for. Several of his items are deliberate traps (exotic fabricated terms like "entropy-gated discrete diffusion"; `"moe_top_k"` is a test alias for a generic topk kernel, not MoE routing).

**How to apply:**
- Enumerate the ground truth, don't keyword-match: e.g. `grep -rhoE 'hip[A-Za-z_]+' *.hip | sort | uniq -c` lists EVERY api the kernels touch — nothing can hide behind it.
- Close obfuscation vectors explicitly: separator forms (`hip[^A-Za-z0-9]{1,4}Malloc`), backslash line-continuation splices (splice with `perl -0777 -pe 's/\\\n//g'` then grep), `##` token-paste, and api variants.
- Distinguish DECLARATION vs CALL-site (`grep -v 'fn '`), PRIMITIVE (real logic, callable, but not wired into the training/inference/data path) vs WIRED (reachable via the user API/pipeline) vs NAME_ONLY vs ABSENT.
- Cite file:line for every claim from code actually read. If a grep label could be a false positive (e.g. matched a `pub fn` decl), retract it immediately and precisely — owning it is the non-hand-wave move.
- Verdict taxonomy beats yes/no. For this repo as of 2026-06: BPE absent (byte-level only), safetensors absent (OGDL only, no dep), RoPE primitive-only (`gpu_rope` real+tested, NN uses additive `sinusoidal_pe`), MoE absent, symmetric quant primitive-only (`quantizedx.hip`, raw FFI, zp=0, no quantized compute), discrete diffusion absent. Sync `hipMalloc(`/`hipFree(` declared in `hip.rs` FFI but never called; device alloc is `hipMallocAsync`/`hipFreeAsync` only; `build.rs` banned-pattern scan walks only the root crate `src/`, NOT gpu-core.
