---
name: No comments in code files
description: Don't fill files with comments, specs, or documentation — show the code or show the function signature
type: feedback
originSessionId: a9e9df7a-26ee-4c90-adc8-620eeb6bf430
---
When asked to put functions in a file, write the functions. Not comments describing functions. Not spec blocks. Not usage examples. The code IS the spec.

**Why:** User got frustrated when prim.rb was filled with comment blocks instead of actual function definitions. Also got frustrated by paragraph-level `//` explanation blocks in dataset.rs ("I can't even read the code") — stripped all 151 comment lines from it.
**How to apply:** When creating a file for function definitions, write the actual code. If the functions don't exist yet, write what they would look like when called — not what they would do internally. Strip multi-line explanatory comment blocks and verbose `///` doc paragraphs.

**Exception — section dividers ARE acceptable/helpful:** single-line box-drawn section headers like `// ── unimplemented / not-wired API sketches ──────────────────────────────────` are fine and wanted. The ban is on prose/explanation comments, not on one-line structural dividers that organize the file.

**No arrows or em-dashes in inline comments:** the "weird shit" the user wants gone is `→` and `—` inside `//` comments. Write `logit to prob` not `logit → prob`; `not wired` not `— not wired`. Box-drawing chars (──) in section dividers are NOT weird shit — keep those. Keep comments as short as possible.
