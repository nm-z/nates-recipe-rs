use crate::HipError;
use crate::kernels::{check_launch, ci};
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

unsafe extern "C" {
	fn launch_mae_grad(
		pred: *const c_void,
		target: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_huber_grad(
		pred: *const c_void,
		target: *const c_void,
		out: *mut c_void,
		n: i32,
		delta: *const c_void,
		stream: *mut c_void,
	);
	fn launch_bce_logits(
		z: *const c_void,
		y: *const c_void,
		loss_out: *mut c_void,
		grad_out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_focal_loss(
		prob: *const c_void,
		target: *const c_void,
		loss_out: *mut c_void,
		grad_out: *mut c_void,
		n: i32,
		gamma: *const c_void,
		alpha: *const c_void,
		stream: *mut c_void,
	);
	fn launch_focal_grad(
		prob: *const c_void,
		target: *const c_void,
		grad_out: *mut c_void,
		n: i32,
		gamma: *const c_void,
		alpha: *const c_void,
		inv_n: *const c_void,
		stream: *mut c_void,
	);
	fn launch_kl_div_loss(
		log_p: *const c_void,
		target: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_hinge(
		scores: *const c_void,
		labels: *const c_void,
		loss_out: *mut c_void,
		grad_out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_cosine_emb(
		a: *const c_void,
		b: *const c_void,
		label: *const c_void,
		out: *mut c_void,
		n: i32,
		dim: i32,
		margin: *const c_void,
		stream: *mut c_void,
	);
	fn launch_triplet(
		anchor: *const c_void,
		pos: *const c_void,
		neg: *const c_void,
		out: *mut c_void,
		n: i32,
		dim: i32,
		margin: *const c_void,
		stream: *mut c_void,
	);
	fn launch_contrastive(
		a: *const c_void,
		b: *const c_void,
		label: *const c_void,
		out: *mut c_void,
		n: i32,
		dim: i32,
		margin: *const c_void,
		stream: *mut c_void,
	);
}

#[inline]
pub fn gpu_mae_grad(
	pred: &GpuBuffer,
	target: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: buffers outlive the synchronous launch and n fits i32.
	unsafe {
		launch_mae_grad(
			pred.ptr_raw().cast_const(),
			target.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_huber_grad(
	pred: &GpuBuffer,
	target: &GpuBuffer,
	delta: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: buffers outlive the synchronous launch and n fits i32.
	unsafe {
		launch_huber_grad(
			pred.ptr_raw().cast_const(),
			target.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			delta.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_bce_with_logits(
	logits: &GpuBuffer,
	target: &GpuBuffer,
	n: usize,
	loss_out: &GpuBuffer,
	grad_out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: buffers outlive the synchronous launch and n fits i32.
	unsafe {
		launch_bce_logits(
			logits.ptr_raw().cast_const(),
			target.ptr_raw().cast_const(),
			loss_out.ptr_raw(),
			grad_out.ptr_raw(),
			ni,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_focal_into(
	prob: &GpuBuffer,
	target: &GpuBuffer,
	gamma: &GpuBuffer,
	alpha: &GpuBuffer,
	n: usize,
	loss_out: &GpuBuffer,
	grad_out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: pointers reference live `GpuBuffer` allocations valid for the launch.
	unsafe {
		launch_focal_loss(
			prob.ptr_raw().cast_const(),
			target.ptr_raw().cast_const(),
			loss_out.ptr_raw(),
			grad_out.ptr_raw(),
			ni,
			gamma.ptr_raw().cast_const(),
			alpha.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_focal_grad_into(
	prob: &GpuBuffer,
	target: &GpuBuffer,
	gamma: &GpuBuffer,
	alpha: &GpuBuffer,
	inv_n: &GpuBuffer,
	n: usize,
	grad_out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: pointers reference live `GpuBuffer` allocations valid for the launch.
	unsafe {
		launch_focal_grad(
			prob.ptr_raw().cast_const(),
			target.ptr_raw().cast_const(),
			grad_out.ptr_raw(),
			ni,
			gamma.ptr_raw().cast_const(),
			alpha.ptr_raw().cast_const(),
			inv_n.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_kl_div_loss(
	log_p: &GpuBuffer,
	target: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: pointers reference live `GpuBuffer` allocations valid for the launch.
	unsafe {
		launch_kl_div_loss(
			log_p.ptr_raw().cast_const(),
			target.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(n)?,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_hinge_loss(
	scores: &GpuBuffer,
	labels: &GpuBuffer,
	n: usize,
	loss_out: &GpuBuffer,
	grad_out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: pointers reference live GpuBuffers valid for the launch; sizes are range-checked above.
	unsafe {
		launch_hinge(
			scores.ptr_raw().cast_const(),
			labels.ptr_raw().cast_const(),
			loss_out.ptr_raw(),
			grad_out.ptr_raw(),
			ni,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_cosine_embedding_loss(
	a: &GpuBuffer,
	b: &GpuBuffer,
	label: &GpuBuffer,
	margin: &GpuBuffer,
	n: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	let di = ci(dim)?;
	// SAFETY: pointers reference live GpuBuffers valid for the launch; sizes are range-checked above.
	unsafe {
		launch_cosine_emb(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			label.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			di,
			margin.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_triplet_loss(
	anchor: &GpuBuffer,
	pos: &GpuBuffer,
	neg: &GpuBuffer,
	margin: &GpuBuffer,
	n: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	let di = ci(dim)?;
	// SAFETY: buffers outlive the synchronous launch and n and dim fit i32.
	unsafe {
		launch_triplet(
			anchor.ptr_raw().cast_const(),
			pos.ptr_raw().cast_const(),
			neg.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			di,
			margin.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_contrastive_loss(
	a: &GpuBuffer,
	b: &GpuBuffer,
	label: &GpuBuffer,
	margin: &GpuBuffer,
	n: usize,
	dim: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	let di = ci(dim)?;
	// SAFETY: buffers outlive the synchronous launch and n and dim fit i32.
	unsafe {
		launch_contrastive(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			label.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			di,
			margin.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
