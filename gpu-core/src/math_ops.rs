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
	pub fn launch_tall_skinny_dgemm(
		x: *const c_void, w: *const c_void, c: *mut c_void,
		m: i32, n: i32, k: i32, stream: *mut c_void,
	);
	pub fn launch_splitk_dw(
		input: *const c_void, grad: *const c_void, partials: *mut c_void, grad_w: *mut c_void,
		m: i32, n: i32, k: i32, p: i32, stream: *mut c_void,
	);
}

// Split-K partition for the backward dW kernel: dW is [k×n], reduction over m
// batch rows. Output tiles (SK_BM×SK_BN) are few, so we split the reduction into
// P slices to fill every multiprocessor. P scales DOWN as the output grows more
// tiles, so the [P×k×n] partial scratch stays ~bounded (≈ target·BM·BN). Derived
// purely from shape + the device's REAL multiprocessor count (queried at runtime,
// never hardcoded), and computed in ONE place so the kernel launch and Scratch
// sizing always agree.
const SK_BM: usize = 64;
const SK_BN: usize = 64;
const SK_WAVES: usize = 8; // occupancy waves per multiprocessor
const SK_MIN_SLICE: usize = 256; // rows per slice floor (amortize launch/LDS)

pub fn splitk_dw_p(m: usize, k: usize, n: usize) -> usize {
	// Target enough workgroups to fill the actual hardware: multiProcessorCount
	// (hipGetDeviceProperties) × occupancy waves, not a baked-in CU count.
	let target_blocks = crate::hip::cu_count() * SK_WAVES;
	let out_tiles = k.div_ceil(SK_BM) * n.div_ceil(SK_BN);
	let target = (target_blocks / out_tiles.max(1)).max(1);
	let max_by_rows = (m / SK_MIN_SLICE).max(1);
	target.min(max_by_rows).min(m.max(1))
}

/// Element count of the `[P×k×n]` partial scratch the split-K dW kernel needs.
pub fn splitk_dw_partials_elems(m: usize, k: usize, n: usize) -> usize {
	splitk_dw_p(m, k, n) * k * n
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
