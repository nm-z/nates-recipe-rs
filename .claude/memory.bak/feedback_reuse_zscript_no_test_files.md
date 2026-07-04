---
name: reuse-the-same-zscript-don-t-write-new-test-rs-files
description: Each new cargo +nightly -Zscript file triggers a full ~15-min rebuild; reuse /home/nate/Desktop/train.rs in place
metadata: 
  node_type: memory
  type: feedback
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

Do NOT create new `.rs` cargo-script files (e.g. /tmp/verify_*.rs) to test things in this project. Each distinct `#!/usr/bin/env -S cargo +nightly -Zscript` file is its own crate and triggers a FULL rebuild of the nates-recipe dependency (~15 minutes).

**Why:** cargo-script caches the compiled binary per script-content hash; a brand-new script can't reuse the incremental cache, so it recompiles the whole dep tree from scratch.

**How to apply:** To try a different config (smaller split, different layers, fewer epochs), EDIT the existing `/home/nate/Desktop/train.rs` in place and re-run it — same script → incremental build (seconds). Restore it after. Never spin up a second script to "verify" something. If you must verify a primitive in isolation, prefer a `cargo test` in the existing gpu-core test crate (already compiled) over a new script. [[feedback_test_output_visible]]
