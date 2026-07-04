---
name: Keep RBS type hints updated
description: When adding new functions to nates-gpu-ruby lib.rs, always add matching RBS signatures
type: feedback
originSessionId: 6e438ce9-7c93-4dea-9503-beaa7166951b
---
Keep nates-gpu-ruby/sig/nates_gpu.rbs in sync with lib.rs. User reads the RBS as the function reference since it's easier to scan than the Rust init block.

**Why:** The RBS is more readable than grepping through 200+ lines of `define_module_function` calls in Rust. 18 functions drifted out of sync without anyone noticing.

**How to apply:** When adding or changing any `define_module_function` in lib.rs, add/update the corresponding `def self.*` in sig/nates_gpu.rbs in the same edit.
