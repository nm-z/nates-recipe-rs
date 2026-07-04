---
name: kwinmcp is the host
description: Never say kwin-mcp session is different from the host — writable=true means it IS the host filesystem
type: feedback
originSessionId: deec1417-f3f8-4935-911d-468a64b0bb91
---
kwin-mcp with writable=true shares the host filesystem. The release binary at /home/nate/Desktop/nates-recipe-rs/target/release/nates_recipe IS the same binary. Do not say "build on the host and launch in kwin" — they are the same thing. Do not distinguish between "host" and "kwin session" when writable=true.

**Why:** User got angry when I suggested building on "the host" separately — the kwin session with writable=true IS the host.

**How to apply:** When using kwin-mcp with writable=true, treat it as the user's actual desktop. Files, binaries, paths are all the same. Never suggest copying files between "host" and "session".
