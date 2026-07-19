#![allow(unsafe_code)]
#![allow(
	clippy::too_many_arguments,
	clippy::unnecessary_cast,
	clippy::missing_safety_doc,
	clippy::type_complexity,
	reason = "GPU kernel launchers carry many FFI scalar args, cast between HIP integer widths at ABI boundaries, document safety at the module level, and encode tier/dim state in nested types"
)]

pub mod asm;
pub mod attention;
pub mod bayes;
pub mod bridge;
pub mod callspy;
pub mod catboost;
pub mod cluster;
pub mod diffusion;
pub mod encoding;
pub mod forest;
pub mod gate;
pub mod graph;
pub mod hip;
pub mod hw;
pub mod infer_ops;
pub mod k_actx;
pub mod k_gapact;
pub mod k_mathx;
pub mod kernels;
pub mod linalg;
pub use log;
pub mod losses;
pub mod math_ops;
pub mod memory;
pub mod moe;
pub mod nn_f32;
pub mod optimizers;
pub mod reductions;
pub mod rl;
pub mod rope;
pub mod sequence;
pub mod svm;
pub mod sys;
pub mod tiered;
pub mod waterfall;

#[derive(Debug)]
pub struct HipError(pub i32);
