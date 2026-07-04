---
name: Never cap or limit optimizer choices
description: Never artificially reduce classes, iterations, ranges, or search space — the optimizer decides, not us
type: feedback
originSessionId: 448d34bf-56b4-4094-93a5-1203c04fe3f1
---
Never cap the number of classes, iterations, hyperparameter ranges, or search space dimensions. If the data has 664 unique values and a classifier treats each as a class, that's what the optimizer gets. If it's slow, accelerate the computation — don't shrink the problem.

**Why:** Capping is reward hacking. The optimizer sees the full problem and decides what's worth exploring. If 664-class logistic regression is slow, make it faster via GPU — don't reduce to 10 classes and pretend the problem is simpler.
**How to apply:** When something is slow, accelerate the code path. Never reduce dimensionality, class count, iteration count, or parameter ranges to "fix" performance.

**Concrete repeat (encoding cardinality):** To "fix" a one-hot OOM (a text column with ~n distinct values → n one-hot columns → n×n matrix → 26 GiB), CC added `ONEHOT_MAX_CARD=256`, collapsing high-cardinality columns to a single frequency column past the threshold. That's a CAP on feature representation — same forbidden class. The user: "what do you mean cap? cap what?" The right fix for a too-big allocation is NOT to shrink it silently — it's to fail clean: compute the size, name the columns that blow it up, exit and let the user `.exclude()` them. A RAM/VRAM budget check that prints size + culprit + exits is allowed (it's "print size and exit", not a cap); silently changing how data encodes to make it fit is not. [[feedback_never_touch_processes]]
