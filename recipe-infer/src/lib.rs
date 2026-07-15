#![allow(unsafe_code, reason = "FFI to HIP runtime")]
#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]

pub mod bridge;
pub mod dequant;
pub mod enums;
pub mod forward;
pub mod gguf;
pub mod llm;
pub mod params;
pub mod safetensors;
pub mod scratch;
pub mod tokenizer;
pub mod work;

pub use bridge::*;
pub use enums::*;
pub use forward::*;
pub use gpu_core::hip::device_synchronize;
pub use gpu_core::log;
pub use gpu_core::memory::{
	ExitD2H, GpuBuffer, Stage, adopt_run_backing_with_image, claim_device_arena_bytes,
	claim_device_arena_bytes_with_image, claim_device_arena_with_image, claimable_bytes,
	device_arena_active, exit_d2h_enqueue, park_run_backing, release_device_arena,
};
pub use gpu_core::tiered;
pub use params::*;
pub use scratch::*;
pub use work::{GEMM_GFLOPS, VRAM_GBS, Work, layer_bwd, layer_fwd};

pub fn init() -> Result<(), gpu_core::hip::HipError> {
	gpu_core::hip::set_device(0)?;
	gpu_core::hip::retain_mempool(0)
}

pub fn shutdown() {
	scratch::free_pinned_pair();
	gpu_core::kernels::gpu_shutdown();
}

struct ByteUnit {
	floor: f64,
	div: f64,
	prec: usize,
	label: &'static str,
}

pub fn human_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	let units = [
		ByteUnit {
			floor: K * K * K,
			div: K * K * K,
			prec: 2,
			label: "GB",
		},
		ByteUnit {
			floor: K * K,
			div: K * K,
			prec: 1,
			label: "MB",
		},
		ByteUnit {
			floor: 0.0,
			div: K,
			prec: 1,
			label: "KB",
		},
	];
	let pick = units.iter().find(|u| f >= u.floor).unwrap_or(&units[2]);
	format!("{:.prec$} {}", f / pick.div, pick.label, prec = pick.prec)
}
