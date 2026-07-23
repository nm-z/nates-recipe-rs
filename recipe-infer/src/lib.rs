#![allow(unsafe_code)]
pub mod chat;
pub mod dequant;
pub mod gguf;
pub mod llm;
pub mod safetensors;
pub mod tokenizer;

pub use gpu_core::hip::device_synchronize;
pub use gpu_core::memory::{
	ExitD2H, GpuBuffer, Stage, adopt_run_backing_with_image, claim_device_arena_bytes,
	claim_device_arena_bytes_with_image, claim_device_arena_with_image, claimable_bytes, device_arena_active,
	exit_d2h_enqueue, exit_d2h_enqueue_buf, park_run_backing, release_device_arena,
};
pub use gpu_core::tiered;

pub fn init() -> Result<(), gpu_core::HipError> {
	gpu_core::hip::set_device(0)?;
	gpu_core::hip::retain_mempool(0)
}

pub fn shutdown() {
	gpu_core::memory::free_pinned_slots();
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
