---
name: Test TUI via tmux or kwinmcp
description: Never claim TUI can't be tested — use tmux or kwin-mcp to verify TUI works
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
Don't say "can't test TUI from here." Use tmux to create a PTY or kwin-mcp to launch a terminal and screenshot. Both are available.

**Why:** User has tmux and kwin-mcp available. Claiming inability to test is lazy.
**How to apply:** When testing TUI code, launch via tmux or kwin-mcp, take screenshot to verify.
