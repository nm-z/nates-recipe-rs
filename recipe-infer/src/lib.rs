#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]
//! Inference half of the framework, lifted into its own crate: layer execution
//! enums, the OGDL checkpoint codec, layer-parameter construction, the forward
//! path (dense/embed/attn/conv + the KV-cache flash-attention inference path),
//! fused GPU metric reductions, feature scaling, and the reusable GPU scratch
//! arena. Tensors in, tensors out — it knows nothing of datasets, columns, or
//! where data came from. Depends only on `gpu_core` and `ndarray`.

pub mod enums;
pub mod forward;
pub mod ogdl;
pub mod params;
pub mod safetensors;
pub mod scratch;

pub use enums::*;
pub use forward::*;
pub use ogdl::*;
pub use params::*;
pub use scratch::*;

/// Select GPU device 0 — call once before any inference. recipe-infer owns the
/// device lifecycle so callers (pantry, binaries) reach the GPU only through it.
pub fn init() -> Result<(), gpu_core::hip::HipError> {
	gpu_core::hip::set_device(0)
}

/// Release GPU resources at process exit.
pub fn shutdown() {
	gpu_core::kernels::gpu_shutdown();
}

/// Human-readable byte size for OOM/VRAM diagnostics: `1.5 GB`, `12.0 MB`, `4.0 KB`.
pub fn human_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	if f >= K * K * K {
		format!("{:.2} GB", f / (K * K * K))
	} else if f >= K * K {
		format!("{:.1} MB", f / (K * K))
	} else {
		format!("{:.1} KB", f / K)
	}
}
