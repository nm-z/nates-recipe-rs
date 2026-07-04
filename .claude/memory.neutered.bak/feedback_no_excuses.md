---
name: No excuses — build the thing
description: User wants action not explanations of why something can't be done
type: feedback
originSessionId: 36551121-db6d-49ea-896e-8ae13f19d2f0
---
When the user asks for something buildable, build it. Don't lead with why the data doesn't fit or why the architecture can't handle it — if something needs to be built or adapted to make it work, build it.

**Why:** User called out laziness when told "the pipeline can't handle protein sequences" — correct response was to build feature extraction, not explain the limitation. Same with GPU saturation — the answer was to fix the code until it actually saturated, not explain why small datasets don't fill GPU compute units.

**How to apply:** If a task seems impossible with current code, change the code. If data doesn't fit the pipeline, add a preprocessing step. If the GPU isn't being used, keep fixing until it is. The user sets the goal, you figure out how.
