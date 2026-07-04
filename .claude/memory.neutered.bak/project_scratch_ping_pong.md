---
name: scratch-ping-pong
description: Scratch uses shared da_a/da_b ping-pong + single dz/dw/db instead of per-layer — saves ~6 GB on 200/100/200/1 arch
metadata: 
  node_type: memory
  type: project
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

**Location (post crate-split, [[project_crate_architecture.md]]):** `Scratch` now lives in `recipe-infer/src/scratch.rs` (was `model.rs`). `forward_into` is in `recipe-infer/src/forward.rs`; `backward_step` stayed in `nates-recipe/src/utils/train.rs` and reads the activations forward retained. `fit_loop_allocations_flat` is in nates-recipe; forward/KV-cache equivalence tests in recipe-infer.

Scratch struct uses shared gradient buffers instead of per-layer:
- `da_a`/`da_b`: two max-sized buffers that alternate (ping-pong) across backward layers
- `dz`: single max-sized buffer reused each layer
- `dw`/`db`: single max-sized buffers reused each layer
- `acts`: still per-layer (backward needs all activations)

backward_step uses a `flip` bool to alternate which da buffer is current vs below.

46→200→100→200→1 at n=594194: 11,583 MB → 5,218 MB working set. Accepted after passing forward_into==forward equivalence and fit_loop_allocations_flat (R² rises, zero allocs/epoch).

**How to apply:** alloc_freeze constraint #4 is runtime (panic!), not compile-time. Typestate or linker version script is the final answer for constraint #5.
