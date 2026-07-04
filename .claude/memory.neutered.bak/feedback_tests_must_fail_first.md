---
name: tests-must-fail-first
description: "A test that proves a bug exists must FAIL before the fix, not pass — passing proves nothing is wrong"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

When writing a test to prove a bug exists, the test must FAIL in the current state. A test that always passes is a printout experiment, not a test. Write the assertion that captures the contract being violated — it should fail now, then pass after the fix.

**Why:** CC wrote a "diagnostic test" with no real assertions (only tested that hipMalloc returned success), then called it proof that hipMallocAsync had overhead. The user called it out: no pass/fail threshold = not a test. Similarly, a VRAM budget test that passes doesn't prove the model doesn't fit.

**How to apply:** Before writing the fix, write the test that fails. The assertion should state the contract ("model fits with 1500 MB headroom"), and the failure message should show the measured violation. Only then implement the fix that makes it pass. [[feedback_no_cya_on_results]]
