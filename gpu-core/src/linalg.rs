use crate::HipError;
use crate::hip::check;
use crate::kernels::{
	ci, gpu_copy_into, gpu_pack_upper_tri, gpu_transpose, hipblas_handle, hipsolver_handle, safe_i32,
};
use crate::memory::GpuBuffer;
use core::cmp::Ordering;
use core::ffi::c_void;
use core::ptr;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

const OP_NONE: u32 = 111;
const OP_TRANS: u32 = 112;
const FILL_LOWER: u32 = 121;

const SOLVER_FILL_LOWER: u32 = 122;
const SOLVER_FILL_UPPER: u32 = 121;
const SOLVER_EIG_VECTOR: u32 = 202;
const SOLVER_JOB_ALL: i8 = 65;

const HIPFFT_Z2Z: i32 = 0x69;
const HIPFFT_D2Z: i32 = 0x6a;
const HIPFFT_FORWARD: i32 = -1;
const HIPFFT_BACKWARD: i32 = 1;

unsafe extern "C" {
	fn hipblasDasum(handle: *mut c_void, n: i32, x: *const f64, incx: i32, result: *mut f64) -> i32;

	fn hipblasIdamax(handle: *mut c_void, n: i32, x: *const f64, incx: i32, result: *mut i32) -> i32;

	fn hipblasDsyrk(
		handle: *mut c_void,
		uplo: u32,
		transA: u32,
		n: i32,
		k: i32,
		alpha: *const f64,
		A: *const f64,
		lda: i32,
		beta: *const f64,
		C: *mut f64,
		ldc: i32,
	) -> i32;

	fn hipblasDgemmStridedBatched(
		handle: *mut c_void,
		transA: u32,
		transB: u32,
		m: i32,
		n: i32,
		k: i32,
		alpha: *const f64,
		A: *const f64,
		lda: i32,
		stride_a: i64,
		B: *const f64,
		ldb: i32,
		stride_b: i64,
		beta: *const f64,
		C: *mut f64,
		ldc: i32,
		stride_c: i64,
		batch_count: i32,
	) -> i32;

	fn hipsolverDgetrf_bufferSize(h: *mut c_void, m: i32, n: i32, A: *mut f64, lda: i32, lwork: *mut i32) -> i32;
	fn hipsolverDgetrf(
		h: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		work: *mut f64,
		lwork: i32,
		ipiv: *mut i32,
		info: *mut i32,
	) -> i32;

	fn hipsolverDgetrs_bufferSize(
		h: *mut c_void,
		trans: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *mut i32,
		B: *mut f64,
		ldb: i32,
		lwork: *mut i32,
	) -> i32;
	fn hipsolverDgetrs(
		h: *mut c_void,
		trans: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *mut i32,
		B: *mut f64,
		ldb: i32,
		work: *mut f64,
		lwork: i32,
		info: *mut i32,
	) -> i32;

	fn hipsolverDpotrs_bufferSize(
		h: *mut c_void,
		uplo: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		B: *mut f64,
		ldb: i32,
		lwork: *mut i32,
	) -> i32;
	fn hipsolverDpotrs(
		h: *mut c_void,
		uplo: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		B: *mut f64,
		ldb: i32,
		work: *mut f64,
		lwork: i32,
		info: *mut i32,
	) -> i32;

	fn hipsolverDgeqrf_bufferSize(h: *mut c_void, m: i32, n: i32, A: *mut f64, lda: i32, lwork: *mut i32) -> i32;
	fn hipsolverDgeqrf(
		h: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		tau: *mut f64,
		work: *mut f64,
		lwork: i32,
		info: *mut i32,
	) -> i32;

	fn hipsolverDorgqr_bufferSize(
		h: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		A: *mut f64,
		lda: i32,
		tau: *mut f64,
		lwork: *mut i32,
	) -> i32;
	fn hipsolverDorgqr(
		h: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		A: *mut f64,
		lda: i32,
		tau: *mut f64,
		work: *mut f64,
		lwork: i32,
		info: *mut i32,
	) -> i32;

	fn hipsolverDsyevd_bufferSize(
		h: *mut c_void,
		jobz: u32,
		uplo: u32,
		n: i32,
		A: *mut f64,
		lda: i32,
		D: *mut f64,
		lwork: *mut i32,
	) -> i32;
	fn hipsolverDsyevd(
		h: *mut c_void,
		jobz: u32,
		uplo: u32,
		n: i32,
		A: *mut f64,
		lda: i32,
		D: *mut f64,
		work: *mut f64,
		lwork: i32,
		info: *mut i32,
	) -> i32;

	fn hipsolverDgesvd_bufferSize(h: *mut c_void, jobu: i8, jobv: i8, m: i32, n: i32, lwork: *mut i32) -> i32;
	fn hipsolverDgesvd(
		h: *mut c_void,
		jobu: i8,
		jobv: i8,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		S: *mut f64,
		U: *mut f64,
		ldu: i32,
		V: *mut f64,
		ldv: i32,
		work: *mut f64,
		lwork: i32,
		rwork: *mut f64,
		info: *mut i32,
	) -> i32;

	fn hipfftPlan1d(plan: *mut *mut c_void, nx: i32, fft_type: i32, batch: i32) -> i32;
	fn hipfftExecZ2Z(plan: *mut c_void, idata: *mut c_void, odata: *mut c_void, direction: i32) -> i32;
	fn hipfftExecD2Z(plan: *mut c_void, idata: *mut c_void, odata: *mut c_void) -> i32;
}

#[inline]
pub fn gpu_dasum(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let mut result = 0.0f64;
	// SAFETY: the hipBLAS handle is initialized and x points to n valid device f64s; result is a live host scalar.
	let status = unsafe {
		hipblasDasum(
			hipblas_handle(),
			safe_i32(n),
			x.ptr_raw().cast::<f64>(),
			1,
			&raw mut result,
		)
	};
	check(status)?;
	return out.load(&[result]);
}

#[inline]
pub fn gpu_idamax(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let mut result: i32 = 0;
	// SAFETY: the hipBLAS handle is initialized and x points to n valid device f64s; result is a live host i32.
	let status = unsafe {
		hipblasIdamax(
			hipblas_handle(),
			safe_i32(n),
			x.ptr_raw().cast::<f64>(),
			1,
			&raw mut result,
		)
	};
	check(status)?;
	return out.load(&[f64::from((result - 1).max(0))]);
}

#[inline]
pub fn gpu_dsyrk(a: &GpuBuffer, n: usize, k: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	let ni = ci(n)?;
	let ki = ci(k)?;
	// SAFETY: the hipBLAS handle is initialized and a, out are valid device buffers for the n-by-k update.
	let status = unsafe {
		hipblasDsyrk(
			hipblas_handle(),
			FILL_LOWER,
			OP_NONE,
			ni,
			ki,
			&raw const alpha,
			a.ptr_raw().cast::<f64>(),
			ki,
			&raw const beta,
			out.ptr_raw().cast::<f64>(),
			ni,
		)
	};
	return check(status);
}

#[expect(
	clippy::too_many_arguments,
	reason = "strided-batched GEMM exposes every leading dimension, stride, offset, and transpose flag as a distinct parameter"
)]
#[inline]
pub fn gpu_bmm_into(
	a_mat: &GpuBuffer,
	b_mat: &GpuBuffer,
	batch: usize,
	m: usize,
	n: usize,
	k: usize,
	lda: usize,
	ldb: usize,
	ldc: usize,
	stride_a: usize,
	stride_b: usize,
	stride_c: usize,
	a_off: usize,
	b_off: usize,
	c_off: usize,
	trans_a: usize,
	trans_b: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	let op_a = match trans_a.cmp(&0) {
		Ordering::Equal => OP_NONE,
		Ordering::Less | Ordering::Greater => OP_TRANS,
	};
	let op_b = match trans_b.cmp(&0) {
		Ordering::Equal => OP_NONE,
		Ordering::Less | Ordering::Greater => OP_TRANS,
	};
	let ni = ci(n)?;
	let mi = ci(m)?;
	let ki = ci(k)?;
	let batchi = ci(batch)?;
	// SAFETY: every buffer pointer is a live device allocation sized by the caller, and the dimension, stride, and offset arguments are range-checked into their FFI integer types as they are passed.
	let status = unsafe {
		hipblasDgemmStridedBatched(
			hipblas_handle(),
			op_b,
			op_a,
			ni,
			mi,
			ki,
			&raw const alpha,
			b_mat.as_ptr_offset(b_off).cast::<f64>(),
			ci(ldb)?,
			i64::try_from(stride_b).map_err(|_err| return HipError(1))?,
			a_mat.as_ptr_offset(a_off).cast::<f64>(),
			ci(lda)?,
			i64::try_from(stride_a).map_err(|_err| return HipError(1))?,
			&raw const beta,
			out.as_ptr_offset(c_off).cast::<f64>(),
			ci(ldc)?,
			i64::try_from(stride_c).map_err(|_err| return HipError(1))?,
			batchi,
		)
	};
	return check(status);
}

#[must_use]
#[inline]
pub fn gpu_lu_factor_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: hipsolverDgetrf_bufferSize reads only the scalar dimension arguments and writes lwork through a valid stack pointer; the matrix pointer is null as permitted for a size-only query.
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			safe_i32(n),
			safe_i32(n),
			ptr::null_mut(),
			safe_i32(n),
			&raw mut lwork,
		);
	}
	return usize::try_from(lwork.max(1)).unwrap_or(1) * size_of::<f64>();
}

#[inline]
pub fn gpu_lu_factor(
	a: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	lu_out: &GpuBuffer,
	ipiv_out: &GpuBuffer,
	info_out: &GpuBuffer,
) -> Result<(), HipError> {
	gpu_copy_into(a, n * n, lu_out)?;
	let mut lwork: i32 = 0;
	let ni = ci(n)?;
	// SAFETY: hipSOLVER getrf buffer-size query; device pointers are valid and n is range-checked to i32.
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			ni,
			ni,
			lu_out.ptr_raw().cast::<f64>(),
			ni,
			&raw mut lwork,
		);
	}
	// SAFETY: hipSOLVER getrf factorization; device pointers are valid and n/lwork are checked.
	let status = unsafe {
		hipsolverDgetrf(
			hipsolver_handle(),
			ni,
			ni,
			lu_out.ptr_raw().cast::<f64>(),
			ni,
			work.ptr_raw().cast::<f64>(),
			lwork,
			ipiv_out.ptr_raw().cast::<i32>(),
			info_out.ptr_raw().cast::<i32>(),
		)
	};
	return check(status);
}

#[must_use]
#[inline]
pub fn gpu_lu_solve_workspace_bytes(n: usize, nrhs: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: hipsolverDgetrs_bufferSize reads only the scalar dimension arguments and writes lwork through a valid stack pointer; every matrix pointer is null as permitted for a size-only query.
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			OP_NONE,
			safe_i32(n),
			safe_i32(nrhs),
			ptr::null_mut(),
			safe_i32(n),
			ptr::null_mut(),
			ptr::null_mut(),
			safe_i32(n),
			&raw mut lwork,
		);
	}
	return usize::try_from(lwork.max(1)).unwrap_or(1) * size_of::<f64>();
}

#[inline]
pub fn gpu_lu_solve(
	lu: &GpuBuffer,
	ipiv: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
	work: &GpuBuffer,
	info_out: &GpuBuffer,
	x_out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::kernels::{ci, gpu_copy_into};
	gpu_copy_into(b, n * nrhs, x_out)?;
	let mut lwork: i32 = 0;
	let ni = ci(n)?;
	let nrhsi = ci(nrhs)?;
	// SAFETY: hipsolverDgetrs_bufferSize reads the scalar dimensions and the live lu/ipiv/x_out device allocations and writes lwork through a valid stack pointer; n and nrhs were range-checked into i32 above.
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			OP_NONE,
			ni,
			nrhsi,
			lu.ptr_raw().cast::<f64>(),
			ni,
			ipiv.ptr_raw().cast::<i32>(),
			x_out.ptr_raw().cast::<f64>(),
			ni,
			&raw mut lwork,
		);
	}
	// SAFETY: lu/ipiv/x_out/work/info_out are live device buffers sized for the n-by-nrhs solve and lwork was queried above.
	let status = unsafe {
		hipsolverDgetrs(
			hipsolver_handle(),
			OP_NONE,
			ni,
			nrhsi,
			lu.ptr_raw().cast::<f64>(),
			ni,
			ipiv.ptr_raw().cast::<i32>(),
			x_out.ptr_raw().cast::<f64>(),
			ni,
			work.ptr_raw().cast::<f64>(),
			lwork,
			info_out.ptr_raw().cast::<i32>(),
		)
	};
	return check(status);
}

#[inline]
#[must_use]
pub fn gpu_potrs_workspace_bytes(n: usize, nrhs: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: buffer-size query with null data pointers; only the local lwork is written.
	unsafe {
		hipsolverDpotrs_bufferSize(
			hipsolver_handle(),
			SOLVER_FILL_UPPER,
			safe_i32(n),
			safe_i32(nrhs),
			ptr::null_mut(),
			safe_i32(n),
			ptr::null_mut(),
			safe_i32(n),
			&raw mut lwork,
		);
	}
	return usize::try_from(lwork.max(1)).unwrap_or_default() * size_of::<f64>();
}

#[inline]
pub fn gpu_potrs(
	l: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
	work: &GpuBuffer,
	info_out: &GpuBuffer,
	x_out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::kernels::gpu_copy_into;
	gpu_copy_into(b, n * nrhs, x_out)?;
	let mut lwork: i32 = 0;
	// SAFETY: l and x_out are valid device buffers and lwork is a valid local; the query writes only lwork.
	unsafe {
		hipsolverDpotrs_bufferSize(
			hipsolver_handle(),
			SOLVER_FILL_UPPER,
			safe_i32(n),
			safe_i32(nrhs),
			l.ptr_raw().cast::<f64>(),
			safe_i32(n),
			x_out.ptr_raw().cast::<f64>(),
			ci(n)?,
			&raw mut lwork,
		);
	}
	// SAFETY: hipSOLVER FFI; handle and device buffers are valid for the call and dims are range-checked.
	let status = unsafe {
		hipsolverDpotrs(
			hipsolver_handle(),
			SOLVER_FILL_UPPER,
			ci(n)?,
			ci(nrhs)?,
			l.ptr_raw().cast::<f64>(),
			ci(n)?,
			x_out.ptr_raw().cast::<f64>(),
			ci(n)?,
			work.ptr_raw().cast::<f64>(),
			lwork,
			info_out.ptr_raw().cast::<i32>(),
		)
	};
	return check(status);
}

#[must_use]
#[inline]
pub fn gpu_qr_workspace_bytes(m: usize, n: usize) -> usize {
	let k = m.min(n);
	let mut lwork: i32 = 0;
	let mut lwork_q: i32 = 0;
	// SAFETY: hipSOLVER geqrf size query; null data pointer is valid for a size-only query and dims are range-checked.
	unsafe {
		hipsolverDgeqrf_bufferSize(
			hipsolver_handle(),
			safe_i32(m),
			safe_i32(n),
			ptr::null_mut(),
			safe_i32(m),
			&raw mut lwork,
		);
	}
	// SAFETY: hipSOLVER orgqr size query; null data pointer is valid for a size-only query and dims are range-checked.
	unsafe {
		hipsolverDorgqr_bufferSize(
			hipsolver_handle(),
			safe_i32(m),
			safe_i32(n),
			safe_i32(k),
			ptr::null_mut(),
			safe_i32(m),
			ptr::null_mut(),
			&raw mut lwork_q,
		);
	}
	return usize::try_from(lwork.max(lwork_q).max(1)).unwrap_or(1) * size_of::<f64>();
}

#[inline]
pub fn gpu_qr(
	a: &GpuBuffer,
	m: usize,
	n: usize,
	work: &GpuBuffer,
	tau: &GpuBuffer,
	info_out: &GpuBuffer,
	q_out: &GpuBuffer,
	r_out: &GpuBuffer,
) -> Result<(), HipError> {
	let k = m.min(n);

	gpu_transpose(a, m, n, q_out)?;

	let mut lwork: i32 = 0;
	// SAFETY: hipSOLVER buffer-size query receives the live handle, the q_out device buffer, and a stack out-pointer; dims are range-checked into i32.
	unsafe {
		hipsolverDgeqrf_bufferSize(
			hipsolver_handle(),
			ci(m)?,
			ci(n)?,
			q_out.ptr_raw().cast::<f64>(),
			ci(m)?,
			&raw mut lwork,
		);
	}
	// SAFETY: hipSOLVER geqrf receives the live handle and device buffers for q_out/tau/work/info; dims are range-checked into i32.
	let status = unsafe {
		hipsolverDgeqrf(
			hipsolver_handle(),
			ci(m)?,
			ci(n)?,
			q_out.ptr_raw().cast::<f64>(),
			ci(m)?,
			tau.ptr_raw().cast::<f64>(),
			work.ptr_raw().cast::<f64>(),
			lwork,
			info_out.ptr_raw().cast::<i32>(),
		)
	};
	check(status)?;

	gpu_pack_upper_tri(q_out, m, n, r_out)?;

	let mut lwork_q: i32 = 0;
	// SAFETY: hipsolver handle is initialized and q_out, tau are valid device buffers; lwork_q is a valid host out-param.
	unsafe {
		hipsolverDorgqr_bufferSize(
			hipsolver_handle(),
			ci(m)?,
			ci(n)?,
			ci(k)?,
			q_out.ptr_raw().cast::<f64>(),
			ci(m)?,
			tau.ptr_raw().cast::<f64>(),
			&raw mut lwork_q,
		);
	}
	// SAFETY: hipsolver handle is initialized and q_out, tau, work, info_out are valid device buffers for the m-by-n factorization.
	let status_q = unsafe {
		hipsolverDorgqr(
			hipsolver_handle(),
			ci(m)?,
			ci(n)?,
			ci(k)?,
			q_out.ptr_raw().cast::<f64>(),
			safe_i32(m),
			tau.ptr_raw().cast::<f64>(),
			work.ptr_raw().cast::<f64>(),
			lwork_q,
			info_out.ptr_raw().cast::<i32>(),
		)
	};
	return check(status_q);
}

#[inline]
#[must_use]
pub fn gpu_eigh_sym_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: hipsolverDsyevd_bufferSize only probes workspace size; the null data pointers are its documented probe inputs
	unsafe {
		hipsolverDsyevd_bufferSize(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			safe_i32(n),
			ptr::null_mut(),
			safe_i32(n),
			ptr::null_mut(),
			&raw mut lwork,
		);
	}
	return usize::try_from(lwork.max(1)).unwrap_or(1) * size_of::<f64>();
}

#[inline]
pub fn gpu_eigh_sym(
	a: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	info_out: &GpuBuffer,
	evals_out: &GpuBuffer,
	evecs_out: &GpuBuffer,
) -> Result<(), HipError> {
	use crate::kernels;
	kernels::gpu_copy_into(a, n * n, evecs_out)?;
	let mut lwork: i32 = 0;
	// SAFETY: every pointer references a live device buffer for the eigendecomposition; hipsolver writes lwork only
	unsafe {
		hipsolverDsyevd_bufferSize(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			safe_i32(n),
			evecs_out.ptr_raw().cast::<f64>(),
			safe_i32(n),
			evals_out.ptr_raw().cast::<f64>(),
			&raw mut lwork,
		);
	}
	// SAFETY: handle and device buffers are valid and dims are range-checked for the symmetric eigensolver
	let status = unsafe {
		hipsolverDsyevd(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			ci(n)?,
			evecs_out.ptr_raw().cast::<f64>(),
			ci(n)?,
			evals_out.ptr_raw().cast::<f64>(),
			work.ptr_raw().cast::<f64>(),
			lwork,
			info_out.ptr_raw().cast::<i32>(),
		)
	};
	return check(status);
}

#[inline]
#[must_use]
pub fn gpu_svd_workspace_bytes(m: usize, n: usize) -> usize {
	let mut lwork: i32 = 0;
	// SAFETY: handle is valid; the workspace-size query only writes lwork
	unsafe {
		hipsolverDgesvd_bufferSize(
			hipsolver_handle(),
			SOLVER_JOB_ALL,
			SOLVER_JOB_ALL,
			safe_i32(m),
			safe_i32(n),
			&raw mut lwork,
		);
	}
	return (usize::try_from(lwork.max(1)).unwrap_or(1) + m * n + n * n) * size_of::<f64>();
}

#[inline]
pub fn gpu_svd(
	a: &GpuBuffer,
	m: usize,
	n: usize,
	work: &GpuBuffer,
	info_out: &GpuBuffer,
	u_out: &GpuBuffer,
	s_out: &GpuBuffer,
	vt_out: &GpuBuffer,
) -> Result<(), HipError> {
	let mn = m * n;
	let nn = n * n;
	let a_cm = work.view(0, mn);
	let v = work.view(mn, nn);
	let solver_off = mn + nn;

	gpu_transpose(a, m, n, &a_cm)?;

	let mut lwork: i32 = 0;
	// SAFETY: handle is valid; the workspace-size query only writes lwork
	unsafe {
		hipsolverDgesvd_bufferSize(
			hipsolver_handle(),
			SOLVER_JOB_ALL,
			SOLVER_JOB_ALL,
			ci(m)?,
			ci(n)?,
			&raw mut lwork,
		);
	}
	let mi = ci(m)?;
	let ni = ci(n)?;
	// SAFETY: mi and ni are the validated i32 matrix dimensions and every pointer argument addresses a device buffer sized for the gesvd call.
	let status = unsafe {
		hipsolverDgesvd(
			hipsolver_handle(),
			SOLVER_JOB_ALL,
			SOLVER_JOB_ALL,
			mi,
			ni,
			a_cm.ptr_raw().cast::<f64>(),
			mi,
			s_out.ptr_raw().cast::<f64>(),
			u_out.ptr_raw().cast::<f64>(),
			mi,
			v.ptr_raw().cast::<f64>(),
			ni,
			work.as_ptr_offset(solver_off).cast::<f64>(),
			lwork,
			ptr::null_mut(),
			info_out.ptr_raw().cast::<i32>(),
		)
	};
	check(status)?;
	return gpu_transpose(&v, n, n, vt_out);
}

struct CachedFftPlan {
	plan: usize,
}
// SAFETY: the wrapped plan is an opaque driver handle only ever touched under the cache mutex, so moving it across threads is sound.
unsafe impl Send for CachedFftPlan {}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct FftKey {
	fft_type: i32,
	n: usize,
}

static FFT_CACHE: OnceLock<Mutex<HashMap<FftKey, CachedFftPlan>>> = OnceLock::new();

fn fft_plan(fft_type: i32, n: usize) -> Result<*mut c_void, HipError> {
	let mut cache = FFT_CACHE
		.get_or_init(|| return Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(PoisonError::into_inner);
	let key = FftKey { fft_type, n };
	let cached = cache.get(&key).map(|entry| return entry.plan);
	if let Some(plan) = cached {
		drop(cache);
		return Ok(ptr::with_exposed_provenance_mut(plan));
	}
	let mut plan: *mut c_void = ptr::null_mut();
	let nx = ci(n)?;
	// SAFETY: nx is the validated transform length and plan is a valid out-pointer the driver fills in.
	let status = unsafe { hipfftPlan1d(&raw mut plan, nx, fft_type, 1) };
	check(status)?;
	cache.insert(
		key,
		CachedFftPlan {
			plan: plan.expose_provenance(),
		},
	);
	drop(cache);
	return Ok(plan);
}

#[inline]
pub fn gpu_fft_c2c_1d(input: &GpuBuffer, n: usize, forward: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let plan = fft_plan(HIPFFT_Z2Z, n)?;
	let direction = match forward.cmp(&0) {
		Ordering::Equal => HIPFFT_BACKWARD,
		Ordering::Less | Ordering::Greater => HIPFFT_FORWARD,
	};
	// SAFETY: plan is a valid cached hipFFT handle and the buffers outlive the call.
	let status = unsafe { hipfftExecZ2Z(plan, input.ptr_raw(), out.ptr_raw(), direction) };
	return check(status);
}

#[inline]
pub fn gpu_rfft_1d(input_real: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let plan = fft_plan(HIPFFT_D2Z, n)?;
	// SAFETY: plan is a valid cached hipFFT handle and the buffers outlive the call.
	let status = unsafe { hipfftExecD2Z(plan, input_real.ptr_raw(), out.ptr_raw()) };
	return check(status);
}
