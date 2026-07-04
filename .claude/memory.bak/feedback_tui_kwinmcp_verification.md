---
name: TUI must be verified in kwinmcp before done
description: Do not end turn until TUI has 0 bugs verified via kwin-mcp screenshots — no claiming success without visual proof
type: feedback
originSessionId: deec1417-f3f8-4935-911d-468a64b0bb91
---
Do not claim TUI work is done until it's verified working in kwin-mcp with screenshots. Launch the binary, take screenshots, confirm every panel updates, trials progress, scores are real. If anything is broken (stuck, -inf, not updating), fix it before ending the turn.

**Why:** Multiple times claimed "compiles clean" while the TUI was completely broken at runtime.

**How to apply:** After any TUI or optimizer change, launch in kwin-mcp, screenshot, verify visually. Don't end turn until it works.
