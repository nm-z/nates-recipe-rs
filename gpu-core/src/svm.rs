use crate::HipError;
use crate::hip::{HIP_MEMCPY_D2H, HIP_MEMCPY_H2D, device_synchronize};
use crate::kernels::{check_launch, ci, cu};
use crate::memory::{GpuBuffer, xfer};
use core::cmp::Ordering;
use core::ffi::c_void;
use core::mem;
use core::ops::Div;
use core::ptr;

unsafe extern "C" {
	fn launch_kernel_matrix(
		x: *const c_void,
		k_out: *mut c_void,
		n: i32,
		dim: i32,
		kind: i32,
		gamma: *const c_void,
		coef0: *const c_void,
		degree: *const c_void,
		stream: *mut c_void,
	);

	fn launch_smo_kkt_score(
		grad: *const c_void,
		alpha: *const c_void,
		y: *const c_void,
		score_i: *mut c_void,
		score_j: *mut c_void,
		n: i32,
		c: *const c_void,
		stream: *mut c_void,
	);

	fn launch_smo_kernel_row(
		x: *const c_void,
		krow: *mut c_void,
		n: i32,
		dim: i32,
		row: i32,
		kind: i32,
		gamma: *const c_void,
		coef0: *const c_void,
		degree: *const c_void,
		stream: *mut c_void,
	);

	fn launch_smo_argmax(s: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);

	fn launch_smo_update_gradient_rows(
		grad: *mut c_void,
		ki: *const c_void,
		kj: *const c_void,
		n: i32,
		di: *const c_void,
		dj: *const c_void,
		stream: *mut c_void,
	);
}

const fn fadd(a: f64, b: f64) -> f64 {
	return a.mul_add(1.0, b);
}

const fn fsub(a: f64, b: f64) -> f64 {
	return b.mul_add(-1.0, a);
}

const fn fmul(a: f64, b: f64) -> f64 {
	return a.mul_add(b, 0.0);
}

fn fdiv(a: f64, b: f64) -> f64 {
	return Div::div(a, b);
}

const fn fneg(a: f64) -> f64 {
	return a.mul_add(-1.0, 0.0);
}

#[inline]
pub fn gpu_kernel_matrix(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	coef0: &GpuBuffer,
	degree: &GpuBuffer,
	n: usize,
	dim: usize,
	kind: usize,
	k_out: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	let dim_i = ci(dim)?;
	let kind_i = ci(kind)?;
	// SAFETY: buffer pointers are valid for the kernel launch and the dimensions were range-checked above.
	unsafe {
		launch_kernel_matrix(
			x.ptr_raw().cast_const(),
			k_out.ptr_raw(),
			n_i,
			dim_i,
			kind_i,
			gamma.ptr_raw().cast_const(),
			coef0.ptr_raw().cast_const(),
			degree.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_smo_kkt_score(
	grad: &GpuBuffer,
	alpha: &GpuBuffer,
	y: &GpuBuffer,
	c: &GpuBuffer,
	n: usize,
	score_i: &GpuBuffer,
	score_j: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	// SAFETY: buffer pointers are valid for the kernel launch and n was range-checked above.
	unsafe {
		launch_smo_kkt_score(
			grad.ptr_raw().cast_const(),
			alpha.ptr_raw().cast_const(),
			y.ptr_raw().cast_const(),
			score_i.ptr_raw(),
			score_j.ptr_raw(),
			n_i,
			c.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_smo_kernel_row(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	coef0: &GpuBuffer,
	degree: &GpuBuffer,
	n: usize,
	dim: usize,
	kind: usize,
	row: usize,
	krow: &GpuBuffer,
) -> Result<(), HipError> {
	let n_i = ci(n)?;
	let dim_i = ci(dim)?;
	let row_i = ci(row)?;
	let kind_i = ci(kind)?;
	// SAFETY: device pointers come from live GpuBuffer allocations sized for these dims.
	unsafe {
		launch_smo_kernel_row(
			x.ptr_raw().cast_const(),
			krow.ptr_raw(),
			n_i,
			dim_i,
			row_i,
			kind_i,
			gamma.ptr_raw().cast_const(),
			coef0.ptr_raw().cast_const(),
			degree.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_smo_argmax(s: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n_i = ci(n)?;
	// SAFETY: device pointers come from live GpuBuffer allocations sized for n.
	unsafe {
		launch_smo_argmax(
			s.ptr_raw().cast_const(),
			out.ptr_raw(),
			n_i,
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

#[inline]
pub fn gpu_smo_update_gradient_rows(
	ki: &GpuBuffer,
	kj: &GpuBuffer,
	di: &GpuBuffer,
	dj: &GpuBuffer,
	n: usize,
	grad: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: device pointers come from live GpuBuffer allocations sized for n.
	unsafe {
		launch_smo_update_gradient_rows(
			grad.ptr_raw(),
			ki.ptr_raw().cast_const(),
			kj.ptr_raw().cast_const(),
			ci(n)?,
			di.ptr_raw().cast_const(),
			dj.ptr_raw().cast_const(),
			ptr::null_mut(),
		);
	}
	check_launch();
	return Ok(());
}

fn read_at(buf: &GpuBuffer, idx: usize) -> Result<f64, HipError> {
	let mut v = [0.0f64];
	// SAFETY: idx * size_of::<f64>() keeps the byte offset inside buf's allocation.
	let src =
		unsafe { buf.ptr_raw().cast::<u8>().add(idx * mem::size_of::<f64>()) }.cast::<c_void>();
	// SAFETY: src (device) and v (host) span exactly one f64 for this D2H copy.
	unsafe {
		xfer(
			v.as_mut_ptr().cast::<c_void>(),
			src,
			mem::size_of::<f64>(),
			HIP_MEMCPY_D2H,
			ptr::null_mut(),
		)?;
	}
	device_synchronize()?;
	return Ok(v[0]);
}

fn write1(buf: &GpuBuffer, val: f64) -> Result<(), HipError> {
	let v = [val];
	// SAFETY: v (host) holds one f64 copied H2D into buf (device).
	unsafe {
		xfer(
			buf.ptr_raw(),
			v.as_ptr().cast::<c_void>(),
			mem::size_of::<f64>(),
			HIP_MEMCPY_H2D,
			ptr::null_mut(),
		)?;
	}
	return Ok(());
}

fn argmax_pick(score: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(f64, usize), HipError> {
	let mut am = [0.0f64; 2];
	gpu_smo_argmax(score, n, out)?;
	// SAFETY: am is a 2-element host array matching out's two device f64s.
	unsafe { out.download_async(&mut am, ptr::null_mut()) }?;
	device_synchronize()?;
	// SAFETY: the argmax kernel writes a finite non-negative index below n, exactly representable in f64 and within usize range.
	let idx = unsafe { am[1].to_int_unchecked::<usize>() };
	return Ok((am[0], idx));
}

fn bias_contrib(
	grad_buf: &GpuBuffer,
	idx: usize,
	y: f64,
	a: f64,
	c: f64,
) -> Result<Option<f64>, HipError> {
	if a > 0.0f64 && a < c {
		return Ok(Some(fdiv(fneg(read_at(grad_buf, idx)?), y)));
	}
	return Ok(None);
}

pub struct SmoModel {
	pub alpha: Vec<f64>,
	pub b: f64,
}

struct Bounds {
	lo: f64,
	hi: f64,
}

struct SmoCtx<'a> {
	x_buf: &'a GpuBuffer,
	y_buf: GpuBuffer,
	alpha_buf: GpuBuffer,
	grad_buf: GpuBuffer,
	up_score_buf: GpuBuffer,
	low_score_buf: GpuBuffer,
	krow_i: GpuBuffer,
	krow_j: GpuBuffer,
	argmax_out: GpuBuffer,
	gamma_buf: GpuBuffer,
	coef0_buf: GpuBuffer,
	degree_buf: GpuBuffer,
	c_buf: GpuBuffer,
	up_delta_buf: GpuBuffer,
	low_delta_buf: GpuBuffer,
	n: usize,
	dim: usize,
	kind_u: usize,
	tol: f64,
	c: f64,
}

fn smo_kernel_row(ctx: &SmoCtx<'_>, row: usize, krow: &GpuBuffer) -> Result<(), HipError> {
	return gpu_smo_kernel_row(
		ctx.x_buf,
		&ctx.gamma_buf,
		&ctx.coef0_buf,
		&ctx.degree_buf,
		ctx.n,
		ctx.dim,
		ctx.kind_u,
		row,
		krow,
	);
}

#[inline]
pub fn gpu_smo_train(
	x_buf: &GpuBuffer,
	y_pm1: &[f64],
	n: usize,
	dim: usize,
	kind: i32,
	gamma: f64,
	coef0: f64,
	degree: f64,
	c: f64,
	tol: f64,
	max_iter: i32,
) -> Result<SmoModel, HipError> {
	let y_buf = GpuBuffer::alloc(n)?;
	y_buf.load(y_pm1)?;
	let alpha_buf = GpuBuffer::alloc(n)?;
	alpha_buf.memset_zero(n * mem::size_of::<f64>())?;
	let grad_buf = GpuBuffer::alloc(n)?;
	grad_buf.load(&vec![-1.0f64; n])?;
	let gamma_buf = GpuBuffer::alloc(1)?;
	gamma_buf.load(&[gamma])?;
	let coef0_buf = GpuBuffer::alloc(1)?;
	coef0_buf.load(&[coef0])?;
	let degree_buf = GpuBuffer::alloc(1)?;
	degree_buf.load(&[degree])?;
	let c_buf = GpuBuffer::alloc(1)?;
	c_buf.load(&[c])?;
	let kind_u = usize::try_from(kind).map_err(|_e| return HipError(1))?;

	let ctx = SmoCtx {
		x_buf,
		y_buf,
		alpha_buf,
		grad_buf,
		up_score_buf: GpuBuffer::alloc(n)?,
		low_score_buf: GpuBuffer::alloc(n)?,
		krow_i: GpuBuffer::alloc(n)?,
		krow_j: GpuBuffer::alloc(n)?,
		argmax_out: GpuBuffer::alloc(2)?,
		gamma_buf,
		coef0_buf,
		degree_buf,
		c_buf,
		up_delta_buf: GpuBuffer::alloc(1)?,
		low_delta_buf: GpuBuffer::alloc(1)?,
		n,
		dim,
		kind_u,
		tol,
		c,
	};

	let mut alpha_host = vec![0.0f64; n];
	let mut bias = 0.0f64;
	let mut b_count = 0usize;
	for _iter in 0i32..max_iter {
		gpu_smo_kkt_score(
			&ctx.grad_buf,
			&ctx.alpha_buf,
			&ctx.y_buf,
			&ctx.c_buf,
			ctx.n,
			&ctx.up_score_buf,
			&ctx.low_score_buf,
		)?;

		let (val_i, up_idx) = argmax_pick(&ctx.up_score_buf, ctx.n, &ctx.argmax_out)?;
		let (val_j, low_idx) = argmax_pick(&ctx.low_score_buf, ctx.n, &ctx.argmax_out)?;
		if fsub(val_i, val_j).partial_cmp(&ctx.tol) == Some(Ordering::Less) {
			break;
		}

		smo_kernel_row(&ctx, up_idx, &ctx.krow_i)?;
		smo_kernel_row(&ctx, low_idx, &ctx.krow_j)?;
		let kii = read_at(&ctx.krow_i, up_idx)?;
		let kij = read_at(&ctx.krow_i, low_idx)?;
		let kjj = read_at(&ctx.krow_j, low_idx)?;

		let yi = y_pm1[up_idx];
		let yj = y_pm1[low_idx];
		let eta = 2.0f64.mul_add(fneg(kij), fadd(kii, kjj));
		let old_a_up = alpha_host[up_idx];
		let old_a_low = alpha_host[low_idx];

		let grad_diff = fneg(fsub(val_i, val_j));
		let new_a_low_raw = match eta.abs().partial_cmp(&1e-12f64) {
			Some(Ordering::Greater) => fadd(old_a_low, fdiv(fmul(yj, grad_diff), eta)),
			Some(Ordering::Less | Ordering::Equal) | None => old_a_low,
		};
		let bounds = match fsub(yi, yj).abs().partial_cmp(&1e-9f64) {
			Some(Ordering::Less) => {
				let alpha_sum = fadd(old_a_up, old_a_low);
				Bounds {
					lo: f64::max(0.0, fsub(alpha_sum, ctx.c)),
					hi: f64::min(ctx.c, alpha_sum),
				}
			}
			Some(Ordering::Equal | Ordering::Greater) | None => {
				let alpha_sum = fsub(old_a_up, old_a_low);
				Bounds {
					lo: f64::max(0.0, fneg(alpha_sum)),
					hi: f64::min(ctx.c, fsub(ctx.c, alpha_sum)),
				}
			}
		};
		let new_a_low = new_a_low_raw.clamp(bounds.lo, bounds.hi);
		let new_a_up = fmul(yi, yj)
			.mul_add(fsub(old_a_low, new_a_low), old_a_up)
			.clamp(0.0, ctx.c);

		let delta_a_up = fsub(new_a_up, old_a_up);
		let delta_a_low = fsub(new_a_low, old_a_low);
		let both_small = (delta_a_up.abs() < 1e-12f64)
			.then_some(())
			.filter(|_u| return delta_a_low.abs() < 1e-12f64);
		if both_small.is_some() {
			break;
		}

		write1(&ctx.up_delta_buf, fmul(yi, delta_a_up))?;
		write1(&ctx.low_delta_buf, fmul(yj, delta_a_low))?;
		gpu_smo_update_gradient_rows(
			&ctx.krow_i,
			&ctx.krow_j,
			&ctx.up_delta_buf,
			&ctx.low_delta_buf,
			ctx.n,
			&ctx.grad_buf,
		)?;

		alpha_host[up_idx] = new_a_up;
		alpha_host[low_idx] = new_a_low;

		if let Some(bc) = bias_contrib(&ctx.grad_buf, up_idx, yi, new_a_up, ctx.c)? {
			bias = fadd(bias, bc);
			b_count += 1;
		}
		if let Some(bc) = bias_contrib(&ctx.grad_buf, low_idx, yj, new_a_low, ctx.c)? {
			bias = fadd(bias, bc);
			b_count += 1;
		}
	}

	let b_final = match b_count.cmp(&0) {
		Ordering::Greater => fdiv(bias, f64::from(cu(b_count)?)),
		Ordering::Equal | Ordering::Less => 0.0f64,
	};
	return Ok(SmoModel {
		alpha: alpha_host,
		b: b_final,
	});
}
