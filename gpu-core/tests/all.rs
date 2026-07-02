mod common;
#[path = "suite/_gpu_live.rs"]
mod gpu_live;
#[path = "suite/oversize_oom.rs"]
mod oversize_oom;
#[path = "suite/prove_flash_train.rs"]
mod prove_flash_train;
#[path = "suite/inventory_proof.rs"]
mod inventory_proof;
#[path = "suite/parity_blas_batched.rs"]
mod parity_blas_batched;
#[path = "suite/parity_blas_l1.rs"]
mod parity_blas_l1;
#[path = "suite/parity_blas_l2.rs"]
mod parity_blas_l2;
#[path = "suite/parity_blas_l3.rs"]
mod parity_blas_l3;
#[path = "suite/parity_solver_fft.rs"]
mod parity_solver_fft;
#[path = "suite/prove_activation.rs"]
mod prove_activation;
#[path = "suite/prove_conv.rs"]
mod prove_conv;
#[path = "suite/prove_creation.rs"]
mod prove_creation;
#[path = "suite/prove_distance.rs"]
mod prove_distance;
#[path = "suite/prove_elementwise_binary.rs"]
mod prove_elementwise_binary;
#[path = "suite/prove_elementwise_unary.rs"]
mod prove_elementwise_unary;
#[path = "suite/prove_embedding.rs"]
mod prove_embedding;
#[path = "suite/prove_foreach.rs"]
mod prove_foreach;
#[path = "suite/prove_histogram.rs"]
mod prove_histogram;
#[path = "suite/prove_indexing.rs"]
mod prove_indexing;
#[path = "suite/prove_loss.rs"]
mod prove_loss;
#[path = "suite/prove_norm.rs"]
mod prove_norm;
#[path = "suite/prove_optimizer.rs"]
mod prove_optimizer;
#[path = "suite/prove_padding.rs"]
mod prove_padding;
#[path = "suite/prove_pool.rs"]
mod prove_pool;
#[path = "suite/prove_quantized.rs"]
mod prove_quantized;
#[path = "suite/prove_reduction.rs"]
mod prove_reduction;
#[path = "suite/prove_scan.rs"]
mod prove_scan;
#[path = "suite/prove_search.rs"]
mod prove_search;
#[path = "suite/prove_set.rs"]
mod prove_set;
#[path = "suite/prove_shape.rs"]
mod prove_shape;
#[path = "suite/prove_sort.rs"]
mod prove_sort;
#[path = "suite/prove_special.rs"]
mod prove_special;
#[path = "suite/t_algos.rs"]
mod t_algos;
#[path = "suite/t_bmm.rs"]
mod t_bmm;
#[path = "suite/t_lead_smoke.rs"]
mod t_lead_smoke;
#[path = "suite/t_linalg_reduce.rs"]
mod t_linalg_reduce;
#[path = "suite/t_math_enc_loss.rs"]
mod t_math_enc_loss;
#[path = "suite/t_ml.rs"]
mod t_ml;
#[path = "suite/t_nn.rs"]
mod t_nn;
