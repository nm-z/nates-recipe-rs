---
name: feedback_rust_can_do_anything
description: Never claim a task needs an external runtime/tool or is impossible for lack of one — build it from scratch in Rust
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

Rust is a general-purpose systems language; it can implement anything — a full LLM
inference engine, a particle-accelerator sim, a from-scratch tokenizer, GGUF/Q4
dequant, transformer forward passes. So **never** answer a build task with "no
runtime supports this," "no external tool for it," or "it's impossible on this
hardware" — those are excuses, not blockers. If a capability is missing, WRITE IT
in Rust and compile+run your own binary (that is the runtime; it is not an
"external binary").

**Why:** the user builds novel systems from primitives and expects the same of me.
Declaring impossibility for lack of an off-the-shelf tool is exactly the laziness
he rejects. The only real constraints are correctness and physics (FLOPs, bytes),
and even those are usually surmountable with the right algorithm (quantization,
disk/RAM/VRAM tiering — see [[project_crate_architecture]] and the tiered buffer).

**How to apply:** when I catch myself about to say "can't be done / needs X tool /
unsupported arch," stop — scope the from-scratch Rust implementation instead and
start building it incrementally. Concrete case: running a 26B diffusion-gemma MoE
GGUF with no external binaries → write the GGUF loader, Q4_K dequant, gemma
tokenizer, and MoE forward in Rust. Do not hold; build. "I'd have to
implement it myself" is never a reason to decline a build task — scope it and
build it. (Destructive edits to working code and fabricated results remain
off-limits, as always.)
