---
name: training-loop-constraints
description: "5 non-negotiable training loop constraints — load once, kernel loop, no alloc, no roundtrip, API prevents violations"
metadata: 
  node_type: memory
  type: project
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

1. Load data once (RAM→VRAM)
2. Run N kernels in a loop
3. Never allocate inside the loop
4. Never do VRAM→RAM→VRAM roundtrips inside the loop
5. The API surface itself should make it impossible to accidentally violate these constraints

Current state: #1-#4 enforced. #3 has runtime guard (alloc_freeze panics on GpuBuffer::alloc_bytes inside the loop). #5 is partially met — alloc_freeze is runtime, not compile-time. Typestate or linker version script needed for true API-level enforcement.

The only D2H inside the loop is download_scalar (8 bytes for the metric scalar). No re-upload follows — one-way, not a roundtrip.

**How to apply:** Every change to the training loop path must be verified against all 5 constraints. fit_loop_allocations_flat is the test.
