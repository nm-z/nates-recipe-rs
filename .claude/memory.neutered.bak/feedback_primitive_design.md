---
name: Primitive design philosophy
description: GPU library design rules — composable primitives, 2-tab max, no monoliths, no boilerplate, readable ML
type: feedback
originSessionId: a9e9df7a-26ee-4c90-adc8-620eeb6bf430
---
The GPU library is "Fisher Price ML" — not notebook trash, not rigid wrappers around prebuilt models.

Rules:
- If Ruby code goes >2 tabs deep, a primitive is missing. Stop and write it.
- No monolithic training functions in Rust. Primitives compose in Ruby.
- Functions should read like the algorithm: `grad`, `tree`, `softmax`, `report` — not plumbing.
- No comments in spec files. The function signature IS the documentation.
- Don't duplicate docs in markdown tables — .rbs stubs and Rust source are the single source of truth.
- Deleted GPU_FUNCTIONS.md for this reason.

**Why:** User is building a reusable GPU ML toolkit, not a one-off Kaggle script. Code readability is a design constraint, not a nice-to-have.
**How to apply:** Before writing training code, check if the inner loop reads like the algorithm. If not, identify what primitives would make it read cleanly, write those first.
