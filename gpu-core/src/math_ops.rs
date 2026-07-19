use crate::HipError;
use crate::hip::cu_count;
use crate::kernels::{check_launch, ci};
use crate::memory::{GpuBuffer, memset_dev};
use core::ffi::c_void;
use core::mem;
use core::ptr;

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
	fn launch_sub_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *const c_void,
		stream: *mut c_void,
	);
	fn launch_div_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *const c_void,
		stream: *mut c_void,
	);
	fn launch_rsub_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *const c_void,
		stream: *mut c_void,
	);
	fn launch_rdiv_scalar(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *const c_void,
		stream: *mut c_void,
	);
	fn launch_has_nan(x: *const c_void, flag: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_isfinite_all(x: *const c_void, flag: *mut c_void, n: i32, stream: *mut c_void);
	pub fn launch_tall_skinny_dgemm(
		x: *const c_void,
		w: *const c_void,
		c: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		stream: *mut c_void,
		dtype: i32,
	);
	pub fn launch_splitk_dw(
		input: *const c_void,
		grad: *const c_void,
		partials: *mut c_void,
		grad_w: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		p: i32,
		stream: *mut c_void,
	);
}

const SK_BM: usize = 64;
const SK_BN: usize = 64;
const SK_WAVES: usize = 8;
const SK_MIN_SLICE: usize = 256;

#[must_use]
#[inline]
pub fn splitk_dw_p(m: usize, k: usize, n: usize) -> usize {
	let target_blocks = cu_count() * SK_WAVES;
	let out_tiles = k.div_ceil(SK_BM) * n.div_ceil(SK_BN);
	let target = target_blocks.div_euclid(out_tiles.max(1)).max(1);
	let max_by_rows = m.div_euclid(SK_MIN_SLICE).max(1);
	return target.min(max_by_rows).min(m.max(1));
}

#[must_use]
#[inline]
pub fn splitk_dw_partials_elems(m: usize, k: usize, n: usize) -> usize {
	return splitk_dw_p(m, k, n) * k * n;
}

#[inline]
pub fn gpu_rsqrt(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_rsqrt(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_reciprocal(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_reciprocal(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_max(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `a`, `b`, and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_emax(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_min(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `a`, `b`, and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_emin(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_sin(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_sin(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_cos(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_cos(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_tan(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_tan(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_atan2(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `a`, `b`, and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_atan2(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_log1p(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_log1p(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_expm1(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_expm1(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_floor(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_floor(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_ceil(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_ceil(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_round(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_round(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_trunc(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x` and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_trunc(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_fmod(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `a`, `b`, and `out` are valid device buffers of at least `n` elements.
	unsafe {
		launch_fmod(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_sub_scalar(
	x: &GpuBuffer,
	s: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x`, `s`, and `out` are valid device buffers; `s` holds one scalar.
	unsafe {
		launch_sub_scalar(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			s.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_div_scalar(
	x: &GpuBuffer,
	s: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x`, `s`, and `out` are valid device buffers; `s` holds one scalar.
	unsafe {
		launch_div_scalar(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			s.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_rsub_scalar(
	x: &GpuBuffer,
	s: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x`, `s`, and `out` are valid device buffers; `s` holds one scalar.
	unsafe {
		launch_rsub_scalar(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			s.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_rdiv_scalar(
	x: &GpuBuffer,
	s: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `x`, `s`, and `out` are valid device buffers; `s` holds one scalar.
	unsafe {
		launch_rdiv_scalar(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i32,
			s.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_has_nan(x: &GpuBuffer, n: usize, flag: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `flag` is a valid device buffer of at least one `i32`.
	unsafe {
		memset_dev(flag.ptr_raw(), 0, mem::size_of::<i32>(), ptr::null_mut())?;
	}
	// SAFETY: `x` and `flag` are valid device buffers of `n` and one element.
	unsafe {
		launch_has_nan(
			x.ptr_raw().cast_const(),
			flag.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_isfinite_all(x: &GpuBuffer, n: usize, flag: &GpuBuffer) -> Result<(), HipError> {
	let n_i32 = ci(n)?;
	// SAFETY: `flag` is a valid device buffer of at least one `i32`.
	unsafe {
		memset_dev(flag.ptr_raw(), 0, mem::size_of::<i32>(), ptr::null_mut())?;
	}
	// SAFETY: `x` and `flag` are valid device buffers of `n` and one element.
	unsafe {
		launch_isfinite_all(
			x.ptr_raw().cast_const(),
			flag.ptr_raw(),
			n_i32,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}
