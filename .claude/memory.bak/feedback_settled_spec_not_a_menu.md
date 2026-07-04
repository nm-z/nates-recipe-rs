---
name: settled-spec-is-not-a-menu-and-fix-swallowed-errors-without-asking
description: When the spec or a stated rule already determines the answer, implement it — don't surface it as a decision/option. Two recurring forms — (1) never offer spec-violating "alternatives" as a choice; (2) always fix swallowed/unchecked errors without asking.
metadata:
  node_type: memory
  type: feedback
  originSessionId: 299a50bb-b629-47ca-8f4c-fad35efc3ade
---

When the spec, a constraint, or a stated rule already determines the answer, **implement it — do not present it as a choice.** Offering options the user must adjudicate, when the spec settled it, is friction and reads as not having internalized the spec.

**Why:** Two concrete blunders Nate corrected in one session:
1. After the spec said "the training loop allocates nothing; all buffers pre-allocated in Scratch," I offered three ways to fix an arena leak — including "cache a workspace in the kernel layer" and "pooled hipMallocAsync." Both are *allocation hiding behind a kernel* — exactly what the spec eliminates. "This was never a choice." The only spec-conformant answer was: pre-allocate the reduce workspace in Scratch alongside acts/da/dz/dw/db, sized once from model dims, reused every epoch.
2. I asked permission to add `check().expect()` on a swallowed rocBLAS status. "Don't ask me about swallowed errors, fix them." Swallowed errors / ignored return statuses violate the standing let-it-fail rule; fixing them is never a question.

**How to apply:**
- Before presenting options, ask: does the spec/stated design already decide this? If yes, there are no options — build the spec-conformant version. Only the genuinely-open forks (ones the spec leaves unspecified) go to the user, and only via [[feedback_implementation_priority]]'s one-question-at-a-time, never a menu.
- Any spec-violating "alternative" is not an option — discard it, don't offer it. A workaround that reintroduces the exact thing the spec bans (e.g. hidden allocation) is wrong by construction.
- Swallowed errors, ignored return codes, unchecked statuses: fix on sight, no permission. Generalize [[feedback_no_cya_optimizer]] / [[feedback_sampler_owns_failures]]. Related: [[feedback_direct_execution]], [[feedback_no_form_when_frustrated]].
