---
name: dev-wrapper-and-60s-tests
description: "Wrap GPU/heavy commands in Nate's `dev` function for inline 1Hz CPU/RAM/GPU/VRAM metrics; hard 60s timeout per test — never iterate with full multi-GB-load runs"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 57ccd39e-5251-42a7-a8fa-ad277f372b44
---

Nate has a `dev` shell function (defined in his bashrc): `dev <cmd>` runs the command with a 1Hz bold-yellow `CPU: x% RAM: xGB GPU: x% VRAM: xGB` line printed to /dev/tty, interleaved live with the command's output. It auto-picks the largest GPU via sysfs and reaps its sampler on exit/INT/TERM. `DEV_INTERVAL` overrides the period.

**Why:** He watches memory behavior (waterfall levels, leaks, tier fills) AS a command runs — correlating a separate monitor against timestamps is the friction. Also: iteration loops that reload tens of GB of weights per test run are too slow — a test that needs >60s is the wrong test.

**How to apply:** (1) From Claude's non-tty Bash shell, `dev` needs a pty + interactive bash: `script -qec 'bash -ic "dev <cmd>"' /dev/null` — metrics then appear in captured output. (2) Timeout 60s per test invocation. For gemma4-class engines (20s load + 50s+ generate) don't verify with full runs each edit: use op-level probes (det_probe), load-only checks, or single-step runs that fit in 60s. Related: [[waterfall-memory-law]].
