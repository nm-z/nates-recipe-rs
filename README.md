```
.
├── build.rs
├── gpu-core
│   ├── build.rs
│   ├── src
│   │   ├── attention.rs
│   │   ├── bayes.rs
│   │   ├── catboost.rs
│   │   ├── cluster.rs
│   │   ├── encoding.rs
│   │   ├── forest.rs
│   │   ├── graph.rs
│   │   ├── hip.rs
│   │   ├── k_actx.rs
│   │   ├── kernels
│   │   │   ├── activationx.hip
│   │   │   ├── actx.hip
│   │   │   ├── apriori.hip
│   │   │   ├── argsort.hip
│   │   │   ├── attention.hip
│   │   │   ├── bayes.hip
│   │   │   ├── catboost.hip
│   │   │   ├── cluster.hip
│   │   │   ├── convx.hip
│   │   │   ├── creationx.hip
│   │   │   ├── distance.hip
│   │   │   ├── distancex.hip
│   │   │   ├── dtw.hip
│   │   │   ├── elementwise_binaryx.hip
│   │   │   ├── elementwise.hip
│   │   │   ├── elementwise_unaryx.hip
│   │   │   ├── embeddingx.hip
│   │   │   ├── encode.hip
│   │   │   ├── foreachx.hip
│   │   │   ├── forest.hip
│   │   │   ├── gapact.hip
│   │   │   ├── graph.hip
│   │   │   ├── histogramx.hip
│   │   │   ├── indexingx.hip
│   │   │   ├── lightgbm.hip
│   │   │   ├── loss.hip
│   │   │   ├── lossx.hip
│   │   │   ├── math.hip
│   │   │   ├── mathx.hip
│   │   │   ├── metrics_fused.hip
│   │   │   ├── metrics.hip
│   │   │   ├── neural_f32.hip
│   │   │   ├── normx.hip
│   │   │   ├── optim.hip
│   │   │   ├── optimizerx.hip
│   │   │   ├── paddingx.hip
│   │   │   ├── poolx.hip
│   │   │   ├── quantizedx.hip
│   │   │   ├── reduce.hip
│   │   │   ├── reductionx.hip
│   │   │   ├── rl.hip
│   │   │   ├── scan.hip
│   │   │   ├── scanx.hip
│   │   │   ├── searchx.hip
│   │   │   ├── sequence.hip
│   │   │   ├── setx.hip
│   │   │   ├── shapex.hip
│   │   │   ├── sortx.hip
│   │   │   ├── specialx.hip
│   │   │   ├── svm.hip
│   │   │   └── tree.hip
│   │   ├── kernels.rs
│   │   ├── k_gapact.rs
│   │   ├── k_mathx.rs
│   │   ├── lib.rs
│   │   ├── linalg.rs
│   │   ├── losses.rs
│   │   ├── math_ops.rs
│   │   ├── memory.rs
│   │   ├── nn_f32.rs
│   │   ├── optimizers.rs
│   │   ├── reductions.rs
│   │   ├── rl.rs
│   │   ├── sequence.rs
│   │   └── svm.rs
│   └── tests
│       ├── common
│       │   └── mod.rs
│       ├── _gpu_live.rs
│       ├── inventory_proof.rs
│       ├── prove_activation.rs
│       ├── prove_conv.rs
│       ├── prove_creation.rs
│       ├── prove_distance.rs
│       ├── prove_elementwise_binary.rs
│       ├── prove_elementwise_unary.rs
│       ├── prove_embedding.rs
│       ├── prove_foreach.rs
│       ├── prove_histogram.rs
│       ├── prove_indexing.rs
│       ├── prove_loss.rs
│       ├── prove_norm.rs
│       ├── prove_optimizer.rs
│       ├── prove_padding.rs
│       ├── prove_pool.rs
│       ├── prove_quantized.rs
│       ├── prove_reduction.rs
│       ├── prove_scan.rs
│       ├── prove_search.rs
│       ├── prove_set.rs
│       ├── prove_shape.rs
│       ├── prove_sort.rs
│       ├── prove_special.rs
│       ├── t_algos.rs
│       ├── t_lead_smoke.rs
│       ├── t_linalg_reduce.rs
│       ├── t_math_enc_loss.rs
│       ├── t_ml.rs
│       ├── t_nn.rs
│       └── t_smo_verify.rs
├── src
│   ├── lib.rs
│   ├── main.rs
│   └── utils
│       ├── data.rs
│       ├── dataset.rs
│       ├── model.rs
│       └── tests.rs
└── train.rs
```
