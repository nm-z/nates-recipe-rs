use crate::hip::{HipError, check};
use crate::kernels::{gpu_copy_into, safe_i32};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_diffusionx_entropy_gated_step(
		logits: *const c_void,
		canvas: *const c_void,
		accepted: *mut c_void,
		renoise: *mut c_void,
		bound: *const c_void,
		n_positions: i32,
		vocab: i32,
		stream: *mut c_void,
	);
	fn launch_diffusionx_commit(
		canvas: *mut c_void,
		accepted: *const c_void,
		renoise: *const c_void,
		committed: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
}

fn e() -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::LAUNCH);
	crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
	check(unsafe { crate::hip::hipGetLastError() })
}

pub fn gpu_entropy_gated_step(
	logits: &GpuBuffer,
	canvas: &GpuBuffer,
	entropy_bound: &GpuBuffer,
	n_positions: usize,
	vocab: usize,
	accepted: &GpuBuffer,
	renoise: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_diffusionx_entropy_gated_step(
			logits.ptr_raw() as *const c_void,
			canvas.ptr_raw() as *const c_void,
			accepted.ptr_raw(),
			renoise.ptr_raw(),
			entropy_bound.ptr_raw() as *const c_void,
			safe_i32(n_positions),
			safe_i32(vocab),
			std::ptr::null_mut(),
		);
	}
	e()
}

pub fn gpu_diffusion_commit(
	accepted: &GpuBuffer,
	renoise: &GpuBuffer,
	n: usize,
	canvas: &GpuBuffer,
	committed: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_diffusionx_commit(
			canvas.ptr_raw(),
			accepted.ptr_raw() as *const c_void,
			renoise.ptr_raw() as *const c_void,
			committed.ptr_raw(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	e()
}

pub struct DiffusionSample {
	pub canvas: GpuBuffer,
	pub steps: usize,
}

pub fn gpu_diffusion_sample(
	mut logits_fn: impl FnMut(&GpuBuffer) -> Result<GpuBuffer, HipError>,
	initial_canvas: &GpuBuffer,
	entropy_bound: f64,
	max_steps: usize,
	n_positions: usize,
	vocab: usize,
) -> Result<DiffusionSample, HipError> {
	let canvas = GpuBuffer::alloc(n_positions)?;
	gpu_copy_into(initial_canvas, n_positions, &canvas)?;
	let committed = GpuBuffer::alloc_bytes(n_positions * std::mem::size_of::<f64>())?;
	committed.memset_zero(n_positions * std::mem::size_of::<f64>())?;
	let bound = GpuBuffer::alloc(1)?;
	bound.load(&[entropy_bound])?;
	let accepted = GpuBuffer::alloc(n_positions)?;
	let renoise = GpuBuffer::alloc(n_positions)?;
	let mut host = vec![0.0f64; n_positions];
	let mut steps = 0usize;
	for s in 0..max_steps {
		steps = s + 1;
		let logits = logits_fn(&canvas)?;
		gpu_entropy_gated_step(
			&logits,
			&canvas,
			&bound,
			n_positions,
			vocab,
			&accepted,
			&renoise,
		)?;
		gpu_diffusion_commit(&accepted, &renoise, n_positions, &canvas, &committed)?;
		unsafe { committed.download_async(&mut host, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()?;
		match host.iter().find(|&&c| c == 0.0) {
			None => break,
			Some(_pending) => continue,
		}
	}
	Ok(DiffusionSample { canvas, steps })
}
