use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_rsqrt(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_reciprocal(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_emax(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_emin(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_sin(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_cos(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_tan(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_atan2(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_log1p(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_expm1(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_floor(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_ceil(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_round(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_trunc(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_fmod(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_sub_scalar(x: *const c_void, out: *mut c_void, n: i32, s: f64, stream: *mut c_void);
	fn launch_div_scalar(x: *const c_void, out: *mut c_void, n: i32, s: f64, stream: *mut c_void);
	fn launch_rsub_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: f64,
		stream: *mut c_void,
	);
	fn launch_rdiv_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: f64,
		stream: *mut c_void,
	);
	fn launch_has_nan(x: *const c_void, flag: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_isfinite_all(x: *const c_void, flag: *mut c_void, n: i32, stream: *mut c_void);
}

pub fn gpu_rsqrt(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_rsqrt(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_reciprocal(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_reciprocal(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_max(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_emax(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_min(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_emin(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_sin(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_sin(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_cos(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_cos(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_tan(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_tan(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_atan2(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_atan2(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_log1p(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_log1p(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_expm1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_expm1(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_floor(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_floor(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_ceil(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_ceil(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_round(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_round(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_trunc(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_trunc(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_fmod(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_fmod(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_sub_scalar(x: &GpuBuffer, s: f64, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_sub_scalar(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			s,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_div_scalar(x: &GpuBuffer, s: f64, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_div_scalar(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			s,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_rsub_scalar(x: &GpuBuffer, s: f64, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_rsub_scalar(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			s,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_rdiv_scalar(x: &GpuBuffer, s: f64, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_rdiv_scalar(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			s,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_has_nan(x: &GpuBuffer, n: usize) -> Result<bool, HipError> {
	let flag = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	unsafe {
		crate::hip::check(crate::hip::hipMemset(
			flag.ptr_raw(),
			0,
			std::mem::size_of::<i32>(),
		))?;
		launch_has_nan(
			x.ptr_raw() as *const c_void,
			flag.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut v = [0i32; 1];
	flag.download_i32(&mut v)?;
	Ok(v[0] != 0)
}

pub fn gpu_isfinite_all(x: &GpuBuffer, n: usize) -> Result<bool, HipError> {
	let flag = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	unsafe {
		crate::hip::check(crate::hip::hipMemset(
			flag.ptr_raw(),
			0,
			std::mem::size_of::<i32>(),
		))?;
		launch_isfinite_all(
			x.ptr_raw() as *const c_void,
			flag.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut v = [0i32; 1];
	flag.download_i32(&mut v)?;
	Ok(v[0] == 0)
}
