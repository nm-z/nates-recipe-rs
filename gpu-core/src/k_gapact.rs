use crate::hip::{HipError, check};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

pub const SELU_ALPHA: f64 = 1.6732632423543772848170429916717;
pub const SELU_LAMBDA: f64 = 1.0507009873554804934193349852946;

unsafe extern "C" {
	fn launch_gapact_elu(x: *const c_void, out: *mut c_void, n: i32, alpha: f64, s: *mut c_void);
	fn launch_gapact_elu_backward(
		g: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: f64,
		s: *mut c_void,
	);
	fn launch_gapact_selu(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: f64,
		lambda: f64,
		s: *mut c_void,
	);
	fn launch_gapact_selu_backward(
		g: *const c_void,
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		alpha: f64,
		lambda: f64,
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
	fn launch_gapact_softplus(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
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
	check(unsafe { crate::hip::hipGetLastError() })
}

pub fn gpu_elu(x: &GpuBuffer, n: usize, alpha: f64) -> Result<GpuBuffer, HipError> {
	let o = GpuBuffer::alloc(n)?;
	unsafe {
		launch_gapact_elu(
			x.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			alpha,
			std::ptr::null_mut(),
		);
	}
	e()?;
	Ok(o)
}
pub fn gpu_elu_backward(
	g: &GpuBuffer,
	x: &GpuBuffer,
	n: usize,
	alpha: f64,
) -> Result<GpuBuffer, HipError> {
	let o = GpuBuffer::alloc(n)?;
	unsafe {
		launch_gapact_elu_backward(
			g.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			alpha,
			std::ptr::null_mut(),
		);
	}
	e()?;
	Ok(o)
}
pub fn gpu_selu(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let o = GpuBuffer::alloc(n)?;
	unsafe {
		launch_gapact_selu(
			x.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			SELU_ALPHA,
			SELU_LAMBDA,
			std::ptr::null_mut(),
		);
	}
	e()?;
	Ok(o)
}
pub fn gpu_selu_backward(g: &GpuBuffer, x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let o = GpuBuffer::alloc(n)?;
	unsafe {
		launch_gapact_selu_backward(
			g.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			SELU_ALPHA,
			SELU_LAMBDA,
			std::ptr::null_mut(),
		);
	}
	e()?;
	Ok(o)
}

// Alloc-free ELU/SELU (backward takes the PRE-activation x, not the output).
pub fn gpu_elu_into(x: &GpuBuffer, out: &GpuBuffer, n: usize, alpha: f64) {
	unsafe {
		launch_gapact_elu(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			alpha,
			std::ptr::null_mut(),
		);
	}
}
pub fn gpu_elu_backward_into(g: &GpuBuffer, x: &GpuBuffer, out: &GpuBuffer, n: usize, alpha: f64) {
	unsafe {
		launch_gapact_elu_backward(
			g.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			alpha,
			std::ptr::null_mut(),
		);
	}
}
pub fn gpu_selu_into(x: &GpuBuffer, out: &GpuBuffer, n: usize) {
	unsafe {
		launch_gapact_selu(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			SELU_ALPHA,
			SELU_LAMBDA,
			std::ptr::null_mut(),
		);
	}
}
pub fn gpu_selu_backward_into(g: &GpuBuffer, x: &GpuBuffer, out: &GpuBuffer, n: usize) {
	unsafe {
		launch_gapact_selu_backward(
			g.ptr_raw() as *const c_void,
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			SELU_ALPHA,
			SELU_LAMBDA,
			std::ptr::null_mut(),
		);
	}
}

macro_rules! u {
	($name:ident, $launch:ident) => {
		pub fn $name(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
			let o = GpuBuffer::alloc(n)?;
			unsafe {
				$launch(
					x.ptr_raw() as *const c_void,
					o.ptr_raw(),
					n as i32,
					std::ptr::null_mut(),
				);
			}
			e()?;
			Ok(o)
		}
	};
}
macro_rules! ub {
	($name:ident, $launch:ident) => {
		pub fn $name(g: &GpuBuffer, x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
			let o = GpuBuffer::alloc(n)?;
			unsafe {
				$launch(
					g.ptr_raw() as *const c_void,
					x.ptr_raw() as *const c_void,
					o.ptr_raw(),
					n as i32,
					std::ptr::null_mut(),
				);
			}
			e()?;
			Ok(o)
		}
	};
}
macro_rules! gate {
	($name:ident, $launch:ident) => {
		pub fn $name(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
			let o = GpuBuffer::alloc(n)?;
			unsafe {
				$launch(
					a.ptr_raw() as *const c_void,
					b.ptr_raw() as *const c_void,
					o.ptr_raw(),
					n as i32,
					std::ptr::null_mut(),
				);
			}
			e()?;
			Ok(o)
		}
	};
}

u!(gpu_mish, launch_gapact_mish);
ub!(gpu_mish_backward, launch_gapact_mish_backward);
u!(gpu_softplus, launch_gapact_softplus);
ub!(gpu_softplus_backward, launch_gapact_softplus_backward);
u!(gpu_hardswish, launch_gapact_hardswish);
ub!(gpu_hardswish_backward, launch_gapact_hardswish_backward);
gate!(gpu_swiglu, launch_gapact_swiglu);
gate!(gpu_geglu, launch_gapact_geglu);
