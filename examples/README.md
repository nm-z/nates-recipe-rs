# Recipe examples

Each `cookbook_*.rs` file is one copyable public-API program. No example relies on a cookbook-only helper, and no
single example runs a catalog of unrelated models.

Native training and inference need a current measured profile:

```text
cargo run --bin recipe -- probe
cargo run --example cookbook_convolution_pooling
```

The cookbook currently covers the executable data, dense layer, perceptron, ordered multi-target objectives,
convolution/pooling, K-means, all-output KNN, fixed-token embedding and causal multi-head attention, dense-F32 llama
GGUF token-logit inference, scalar-sequence vanilla RNN, reset-before GRU, and zero-cell LSTM, singular and repeated
observed categorical Bayesian target conditionals, supervised tree/forest, residual, activation (including
distinct signed `.log()` and natural `.ln()` semantics), normalization, loss, optimizer, schedule, observability, save/resume, and
semantic-OGDL inference families. Examples for the remaining declared model cases belong here only after their public
execution paths exist; a declaration-only example is not evidence that an API works.
