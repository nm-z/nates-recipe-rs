---
name: nates-gpu-ruby is outside the workspace
description: nates-gpu-ruby has its own Cargo.toml and target/ — must build with --manifest-path or touch gpu-core sources
type: feedback
originSessionId: f44be0ad-41ac-4569-b9ac-5e1062477260
---
nates-gpu-ruby is excluded from the root workspace (`exclude = ["nates-gpu-lua", "catboost-rs"]`, and nates-gpu-ruby is not a member). Running `cargo build --release` from the repo root builds `nates_recipe`, NOT the Ruby gem.

**Why:** Spent an entire session editing gpu-core/src/memory.rs and rebuilding from the repo root. Cargo silently skipped gpu-core recompilation every time — the .so never changed. The bug was never fixed because the fix was never compiled.

**How to apply:** Always build with `cargo build --release --manifest-path /home/nate/Desktop/nates-recipe-rs/nates-gpu-ruby/Cargo.toml`. If editing gpu-core sources, `touch` the changed files first — cargo's mtime detection can miss Write tool edits. Verify with `Compiling gpu-core` in the output.
