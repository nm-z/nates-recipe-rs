use crate::HipError;
use crate::callspy::{GET_LAST_ERROR, LAUNCH, tick};
use crate::hip::{check, device_synchronize, hipGetLastError};
use crate::kernels::{gpu_copy_into, safe_i32};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::mem;
use core::ptr;

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
	tick(&LAUNCH);
	tick(&GET_LAST_ERROR);
	// SAFETY: hipGetLastError only reads and clears the calling thread's last HIP error and is always sound to call.
	return check(unsafe { hipGetLastError() });
}

#[inline]
pub fn gpu_entropy_gated_step(
	logits: &GpuBuffer,
	canvas: &GpuBuffer,
	entropy_bound: &GpuBuffer,
	n_positions: usize,
	vocab: usize,
	accepted: &GpuBuffer,
	renoise: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every buffer is a valid device allocation sized for n_positions and vocab, matching the launcher's bounds.
	unsafe {
		launch_diffusionx_entropy_gated_step(
			logits.ptr_raw().cast_const(),
			canvas.ptr_raw().cast_const(),
			accepted.ptr_raw(),
			renoise.ptr_raw(),
			entropy_bound.ptr_raw().cast_const(),
			safe_i32(n_positions),
			safe_i32(vocab),
			ptr::null_mut(),
		);
	}
	return e();
}

#[inline]
pub fn gpu_diffusion_commit(
	accepted: &GpuBuffer,
	renoise: &GpuBuffer,
	n: usize,
	canvas: &GpuBuffer,
	committed: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: every buffer is a valid device allocation of at least n elements, matching the launcher's bounds.
	unsafe {
		launch_diffusionx_commit(
			canvas.ptr_raw(),
			accepted.ptr_raw().cast_const(),
			renoise.ptr_raw().cast_const(),
			committed.ptr_raw(),
			safe_i32(n),
			ptr::null_mut(),
		);
	}
	return e();
}

pub struct Sample {
	pub canvas: GpuBuffer,
	pub steps: usize,
}

#[inline]
pub fn gpu_diffusion_sample(
	mut logits_fn: impl FnMut(&GpuBuffer) -> Result<GpuBuffer, HipError>,
	initial_canvas: &GpuBuffer,
	entropy_bound: f64,
	max_steps: usize,
	n_positions: usize,
	vocab: usize,
) -> Result<Sample, HipError> {
	let canvas = GpuBuffer::alloc(n_positions)?;
	gpu_copy_into(initial_canvas, n_positions, &canvas)?;
	let committed = GpuBuffer::alloc_bytes(n_positions * mem::size_of::<f64>())?;
	committed.memset_zero(n_positions * mem::size_of::<f64>())?;
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
		// SAFETY: host holds n_positions f64 slots matching committed's element count for the async copy.
		unsafe { committed.download_async(&mut host, ptr::null_mut()) }?;
		device_synchronize()?;
		if !host.iter().any(|&c| return c == 0.0f64) {
			break;
		}
	}
	return Ok(Sample { canvas, steps });
}
