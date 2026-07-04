---
name: No unnecessary feature branches
description: Work directly on master for direct fixes, don't create branches without reason
type: feedback
originSessionId: 65e31ea2-00c3-4e51-a5bf-62ffe7e839d2
---
Don't create feature branches for straightforward fixes and improvements. Work directly on master unless there's a real reason for isolation (risky experiment, parallel competing approaches, etc.).

**Why:** User called it out — "why are we on 2 branches?" There was no review gate or reason to isolate.

**How to apply:** Default to committing on current branch. Only branch when the work is genuinely experimental or needs isolation.
