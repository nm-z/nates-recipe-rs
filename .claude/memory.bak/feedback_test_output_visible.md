---
name: tests-must-be-visible-and-must-not-hog-his-gpu
description: No surprise long GPU tests while he works hands-on; never run a blocking command that shows no output; stream/tail+tee it; tail -n 15 beats /dev/null; a hang must surface fast
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

Two problems Nate hit repeatedly: (a) Claude writes test modules / hand-rolled spot-tests he never asked for that take 10–20 min and seize the GPU while he is testing hands-on, and (b) Claude runs them as blocking commands that print NOTHING — redirected away or silently blocking — so he cannot tell whether Claude failed, the command failed, or it is fine and Claude just buried a blocking call in it. Often the test is literally proving something hangs, but it never exits with a message, so the hang is invisible and would take "two weeks" to notice.

**How to apply:**
- Don't write or run tests he isn't aware of. The GPU is the single resource he is actively using hands-on — never launch a long (10–20 min) GPU-hogging test that blocks his work. Run only ONE GPU process at a time.
- NEVER run a blocking command that shows no output. He must always be able to see whether it is progressing, failing, hanging, or fine. Output visibility is mandatory, not optional.
- `tail -n 15` is BETTER than `/dev/null` or redirecting output into the void. Best pattern: redirect to a file AND tail it (`cmd > /tmp/x.txt & tail -f /tmp/x.txt`, or `tee`) so full output is saved AND he sees it live. This is the visibility use of tail — it REVEALS, it does not conceal (different from tailing to hide the meaningful output).
- A hang must surface fast: stream progress and let it exit with a clear message. Never background a silent 10–20 min command behind a 5-min wait where a hang is undetectable. If it can hang, make the hang observable immediately.
Related: [[feedback_no_stderr_redirect]], [[feedback_no_output_filtering]], [[feedback_no_cya_on_results]], [[feedback_never_touch_processes]].
