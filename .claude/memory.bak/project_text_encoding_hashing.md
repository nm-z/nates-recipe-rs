---
name: text-columns-must-be-hashed-to-fixed-width-never-excluded
description: "Free-text / high-cardinality columns get the hashing trick (tokenize → hash → fixed-D vector), never .exclude(), never a frequency cap"
metadata: 
  node_type: memory
  type: project
  originSessionId: bdbe577d-a8c6-4bae-a270-446a76d6b7aa
---

Free-text / high-cardinality columns (prompt, response_a, response_b — ~n distinct each) must be encoded by **feature hashing**, NOT excluded and NOT one-hot-exploded to n×n (which OOMs at tens of GiB).

The user's exact directive: "you should be doing Tokenization or you could do hash each text to a fixed-width vector ... if its hashed or tokenized, then we should be able to one-hot." Tokenize the text, hash each token mod D into a **fixed-width D-vector** (counts per bucket). Width is D regardless of text length or vocabulary size → no OOM, train/test align by construction (same hash, same D).

Encoding rule in `encode()` (src/utils/dataset.rs): a `Kind::Nominal` column with `cats.len() <= D` → one-hot as before (model_a, 64 cats); `cats.len() > D` → hashing trick into D buckets. one-hot is just the special case where the vocab fits the fixed width D.

**Banned alternatives the user rejected:**
- `.exclude()`-ing the text columns to dodge the OOM ("remove .exclude I did not say you could add APIs"). Excluding throws away the only predictive signal. Text must be USED, via hashing.
- Collapsing high-cardinality to a single frequency column (ONEHOT_MAX_CARD cap) — see [[feedback_never_cap]]. That loses token info.
- "Length features" / any reframe that is mini-batching in disguise — see [[project_no_minibatching]]. Feature hashing is a per-row transform then FULL-batch training; keep it that way.

D is the hash dimensionality (the user's "fixed width"), an encoder constant, not a new builder API — the user said don't add APIs. Crate-first still applies if a HashingVectorizer crate (sklears-feature-extraction, vtext) gets wired in; tokenize+hash-mod-D inline is primitive string/hash ops, not a hand-rolled ML algorithm.
