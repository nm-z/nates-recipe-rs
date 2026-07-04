---
name: feedback_foreground_e2e_testing
description: "Verify by building then running the real binary/example in the FOREGROUND — never background e2e runs, never treat `cargo test` as proof (it's performative)."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: c71a10a1-a999-4982-b472-5c12ca8ab0be
---

Real verification = build the binary, then RUN it in the foreground and read the actual output. `cargo test` passing is NOT proof the feature works e2e — the user calls relying on it "performative testing." Tests must be run INDIVIDUAL to building (build step, then run step), in the foreground (never `run_in_background`, never a backgrounded e2e).

**Why:** I claimed the three GPU fixes "verified" off `cargo test` green + a profiling harness, and worked AROUND the categorical-target-multi-class issue instead of running the real end-to-end training and confirming it. The user wants the actual program run and observed, not a green test summary.

**How to apply:** to verify a change, `cargo build --release [--example X]` then `./target/release/...` in the foreground with visible output; confirm the real behavior (loss falls, predictions sane, no panic). Use `cargo test` only as a supplement, never as the e2e claim. Never background the run. Ties to [[feedback_no_cya_on_results]] and [[feedback_test_output_visible]].
