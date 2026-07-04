---
name: Don't filter command output
description: User wants to see raw output from commands, not filtered/grepped versions
type: feedback
originSessionId: 36551121-db6d-49ea-896e-8ae13f19d2f0
---
Don't pipe command output through grep/tail/head to "clean it up". Let the user see the full output, or give them the command to run themselves.

**Why:** User explicitly asked to stop shielding output and to just give the command to run.

**How to apply:** When running cargo/build commands, show full output. When the user wants to run something, give them the command string instead of running it filtered.

**Visibility exception (his later, explicit correction):** the ban is on HIDING meaningful output (tail/grep/head used to conceal a failure or cherry-pick). Using `tail`/`tee` to make an otherwise-silent long-running command VISIBLE is the opposite and is REQUIRED — `tail -n 15` of a live/saved log beats showing nothing at all. Never redirect into `/dev/null` or the void; redirect to a file and tail it so output is saved AND live. See [[feedback_test_output_visible]].
