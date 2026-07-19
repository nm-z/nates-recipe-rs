use crate::HipError;
use crate::callspy;
use crate::hip::{check, hipGetLastError};
use crate::kernels::ci;
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

pub const SELU_ALPHA: f64 = 1.673_263_242_354_377_284_817_042_991_671_7;
pub const SELU_LAMBDA: f64 = 1.050_700_987_355_480_493_419_334_985_294_6;

unsafe extern "C" {
	fn launch_gapact_elu(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: *const c_void,
		s: *mut c_void,
		dtype: i32,
	);
	fn launch_gapact_elu_backward(
		g: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: *const c_void,
		s: *mut c_void,
	);
	fn launch_gapact_selu(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: *const c_void,
		lambda: *const c_void,
		s: *mut c_void,
		dtype: i32,
	);
	fn launch_gapact_selu_backward(
		g: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: *const c_void,
		lambda: *const c_void,
		s: *mut c_void,
	);
	fn launch_gapact_mish(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_gapact_mish_backward(
		g: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_gapact_softplus(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void, dtype: i32);
	fn launch_gapact_softplus_backward(
		g: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_gapact_hardswish(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_gapact_hardswish_backward(
		g: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_gapact_swiglu(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_gapact_geglu(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
}

fn e() -> Result<(), HipError> {
	callspy::tick(&callspy::LAUNCH);
	callspy::tick(&callspy::GET_LAST_ERROR);
	// SAFETY: FFI query of the last HIP error; reads global error state with no caller preconditions.
	return check(unsafe { hipGetLastError() });
}

#[inline]
pub fn gpu_elu(
	x: &GpuBuffer,
	alpha: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: buffer pointers come from live GpuBuffers valid for n elements; the launch reads and writes only within them.
	unsafe {
		launch_gapact_elu(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			alpha.ptr_raw().cast_const(),
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return e();
}
#[inline]
pub fn gpu_elu_backward(
	g: &GpuBuffer,
	x: &GpuBuffer,
	alpha: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: buffer pointers come from live GpuBuffers valid for n elements; the launch reads and writes only within them.
	unsafe {
		launch_gapact_elu_backward(
			g.ptr_raw().cast_const(),
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			alpha.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return e();
}
#[inline]
pub fn gpu_selu(
	x: &GpuBuffer,
	alpha: &GpuBuffer,
	lambda: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: buffer pointers come from live GpuBuffers valid for n elements; the launch reads and writes only within them.
	unsafe {
		launch_gapact_selu(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			alpha.ptr_raw().cast_const(),
			lambda.ptr_raw().cast_const(),
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return e();
}
#[inline]
pub fn gpu_selu_backward(
	g: &GpuBuffer,
	x: &GpuBuffer,
	alpha: &GpuBuffer,
	lambda: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: g/x/alpha/lambda/out reference live GpuBuffers valid for n elements; the launcher only touches that range.
	unsafe {
		launch_gapact_selu_backward(
			g.ptr_raw().cast_const(),
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			alpha.ptr_raw().cast_const(),
			lambda.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	return e();
}

macro_rules! u {
	($name:ident, $launch:ident) => {
		#[inline]
		pub fn $name(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
			let ni = ci(n)?;
			// SAFETY: x/out reference live GpuBuffers valid for n elements; the launcher only touches that range.
			unsafe {
				$launch(x.ptr_raw().cast_const(), out.ptr_raw(), ni, ptr::null_mut());
			}
			return e();
		}
	};
}
macro_rules! ub {
	($name:ident, $launch:ident) => {
		#[inline]
		pub fn $name(
			g: &GpuBuffer,
			x: &GpuBuffer,
			n: usize,
			out: &GpuBuffer,
		) -> Result<(), HipError> {
			let ni = ci(n)?;
			// SAFETY: g/x/out reference live GpuBuffers valid for n elements; the launcher only touches that range.
			unsafe {
				$launch(
					g.ptr_raw().cast_const(),
					x.ptr_raw().cast_const(),
					out.ptr_raw(),
					ni,
					ptr::null_mut(),
				);
			}
			return e();
		}
	};
}
macro_rules! gate {
	($name:ident, $launch:ident) => {
		#[inline]
		pub fn $name(
			a: &GpuBuffer,
			b: &GpuBuffer,
			n: usize,
			out: &GpuBuffer,
		) -> Result<(), HipError> {
			let ni = ci(n)?;
			// SAFETY: the buffers outlive the launch and `n` is the checked length.
			unsafe {
				$launch(
					a.ptr_raw().cast_const(),
					b.ptr_raw().cast_const(),
					out.ptr_raw(),
					ni,
					ptr::null_mut(),
				);
			}
			return e();
		}
	};
}

u!(gpu_mish, launch_gapact_mish);
ub!(gpu_mish_backward, launch_gapact_mish_backward);
#[inline]
pub fn gpu_softplus(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let ni = ci(n)?;
	// SAFETY: x/out reference live GpuBuffers valid for n elements; the launcher only touches that range.
	unsafe {
		launch_gapact_softplus(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			ni,
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return e();
}
ub!(gpu_softplus_backward, launch_gapact_softplus_backward);
u!(gpu_hardswish, launch_gapact_hardswish);
ub!(gpu_hardswish_backward, launch_gapact_hardswish_backward);
gate!(gpu_swiglu, launch_gapact_swiglu);
gate!(gpu_geglu, launch_gapact_geglu);
