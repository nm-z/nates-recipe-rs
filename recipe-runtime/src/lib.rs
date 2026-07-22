#![allow(unsafe_code)]
pub mod execute;
pub mod graph;
pub mod machine;
pub mod memory;
pub mod plan;
pub mod resolve;
pub mod transport;

pub use execute::{attn_forward, attn_forward_cached, forward_into};
pub use memory::{
	ELU_ALPHA, FOCAL_ALPHA, FOCAL_GAMMA, LEAKY_ALPHA, LayerEvents, LayerMs, LayerParams,
	LayerPlan, PRELU_INIT, PlanMode, SCRATCH_CONSTS, Saved, Scaler, Scratch, concat_layer,
	dump_ogdl, layer_ms_from, load_ogdl, load_ogdl_str, ogdl_text, plan_layer_params,
	saved_score, sinusoidal_pe, vram_estimate, write_ogdl,
};
