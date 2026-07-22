#![allow(unsafe_code)]
pub mod bridge;
pub mod chat;
pub mod dequant;
pub mod enums;
pub mod forward;
pub mod gguf;
pub mod llm;
pub mod params;
pub mod safetensors;
pub mod scratch;
pub mod tokenizer;

pub use bridge::*;
pub use enums::*;
pub use forward::*;
pub use gpu_core::hip::device_synchronize;
pub use ogdl::log;
pub use gpu_core::memory::{
	ExitD2H, GpuBuffer, Stage, adopt_run_backing_with_image, claim_device_arena_bytes,
	claim_device_arena_bytes_with_image, claim_device_arena_with_image, claimable_bytes,
	device_arena_active, exit_d2h_enqueue, exit_d2h_enqueue_buf, park_run_backing,
	release_device_arena,
};
pub use gpu_core::tiered;
pub use params::*;
pub use scratch::*;

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

#[ctor::ctor]
fn probe_child_answer() {
	if std::env::var_os("VRAM_PROBE").is_some() || std::env::var_os("RAM_PROBE").is_some() {
		if let Some(code) = llm::vram_probe_ask() {
			std::process::exit(code);
		}
		if let Some(code) = gpu_core::memory::ram_probe_ask() {
			std::process::exit(code);
		}
	}
}
