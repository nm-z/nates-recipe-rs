# TODO

User-facing features referenced in `examples/cookbook.rs` that don't exist yet.

## Data loading
- [ ] `.window(n)` — sliding window for time series (n prior steps as features, next step as target)
- [x] `data.target` — resolves to whatever `.target()` was set to
- [x] `data.set` / `data.test` — public fields populated by `.target()`

## Data type detection
- [x] Numeric — continuous f64, non-integer or unique floats
- [x] Temporal — date strings (ISO/slash format) auto-encoded to days
- [x] Categorical — string values that repeat, or integers where every value appears ≥2x on average
- [ ] Ordinal — ordered categorical (integer-encode instead of one-hot)
- [x] Text — all-unique strings, tokenized for embedding
- [x] Image — file paths (.png/.jpg/etc) or base64 cell content (detected, not yet encoded)
- [x] Mixed missing markers — N/A, NULL, None, nan, ?, ., - filtered before detection
- [x] Mostly-numeric columns — ≥80% f64 treated as numeric with NaN for unparseable cells

## Layer types
- [ ] `conv(filters, kernel_size)` — 1D/2D convolutional layer
- [ ] `pool(size)` — max/avg pooling layer
- [ ] `gru(hidden)` — GRU recurrent layer
- [ ] `lstm(hidden)` — LSTM recurrent layer

## API
- [x] `.layer(64).leak()` — chained activation methods replace tuple syntax
- [x] `.log([Loss, R2])` — accepts IntoIterator, no `&[]` needed
- [x] `.run()` borrows `&self`, reusable in loops
- [x] `.save([w, b], path)` — post-run param save
- [x] `.save(["Id", data.target], path)` — post-run prediction CSV
- [x] Preflight checks — VRAM, embed/text mismatch, loss/output mismatch, interactive prompt

## Model types
- [ ] `.trees(n)` — gradient boosted trees (gpu-core has `forest.rs`, `catboost.rs`)
- [ ] `.depth(d)` — tree max depth
- [ ] `.ensemble(&[model, model])` — average/vote across heterogeneous models

## Optimizers
- [ ] `.optimizer(adam)` — optimizer selection (currently SGD only; gpu-core has momentum, rmsprop, adagrad, lamb, lion, nadam)

## Losses
- [ ] `focal` — focal loss for class imbalance (gpu-core has `gpu_focal_loss`)
- [ ] `hinge` — SVM hinge loss (gpu-core has `gpu_hinge_loss`)
- [ ] `kl` — KL divergence (gpu-core has `gpu_kl_div_loss`)
- [ ] `triplet` — triplet margin loss (gpu-core has `gpu_triplet_loss`)
- [ ] `contrastive` — contrastive loss (gpu-core has `gpu_contrastive_loss`)
- [ ] `cosine` — cosine embedding loss (gpu-core has `gpu_cosine_embedding_loss`)

## Models from gpu-core not yet wired
- [ ] SVM (gpu-core has `svm.rs`: kernel matrix, SMO solver)
- [ ] Naive Bayes (gpu-core has `bayes.rs`: multinomial, Bernoulli)
- [ ] Graph neural networks (gpu-core has `graph.rs`: CSR SpMV/SpMM, neighbor aggregate, GCN norm)
- [ ] Sequence models (gpu-core has `sequence.rs`: forward-backward, Viterbi)
