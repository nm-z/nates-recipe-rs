# TODO

User-facing features referenced in `examples/cookbook.rs` that don't exist yet.

## Data loading
- [ ] `.images("dir/")` — load image directories as samples (pixel matrix + label from CSV or subdir name)
- [ ] `.window(n)` — sliding window for time series (n prior steps as features, next step as target)

## Layer types
- [ ] `conv(filters, kernel_size)` — 1D/2D convolutional layer
- [ ] `pool(size)` — max/avg pooling layer
- [ ] `gru(hidden)` — GRU recurrent layer
- [ ] `lstm(hidden)` — LSTM recurrent layer

## Model types
- [ ] `.trees(n)` — gradient boosted trees (gpu-core has `forest.rs`, `catboost.rs`)
- [ ] `.depth(d)` — tree max depth
- [ ] `.kmeans(k)` — unsupervised k-means clustering (gpu-core has `cluster.rs`)
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

## Training
- [ ] `.submit("submission.csv")` — predict on test set and write Kaggle submission CSV

## Models from gpu-core not yet wired
- [ ] SVM (gpu-core has `svm.rs`: kernel matrix, SMO solver)
- [ ] Naive Bayes (gpu-core has `bayes.rs`: multinomial, Bernoulli)
- [ ] Graph neural networks (gpu-core has `graph.rs`: CSR SpMV/SpMM, neighbor aggregate, GCN norm)
- [ ] Sequence models (gpu-core has `sequence.rs`: forward-backward, Viterbi)
