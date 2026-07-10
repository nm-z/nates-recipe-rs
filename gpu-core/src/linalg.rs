use crate::hip::{HipError, check};
use crate::kernels::{hipblas_handle, hipsolver_handle, safe_i32};
use crate::memory::GpuBuffer;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

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
	fn hipblasDasum(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut f64,
	) -> i32;

	fn hipblasIdamax(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut i32,
	) -> i32;

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

	fn hipsolverDgetrf_bufferSize(
		h: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		lwork: *mut i32,
	) -> i32;
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

	fn hipsolverDgeqrf_bufferSize(
		h: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		lwork: *mut i32,
	) -> i32;
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

	fn hipsolverDgesvd_bufferSize(
		h: *mut c_void,
		jobu: i8,
		jobv: i8,
		m: i32,
		n: i32,
		lwork: *mut i32,
	) -> i32;
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
	fn hipfftExecZ2Z(
		plan: *mut c_void,
		idata: *mut c_void,
		odata: *mut c_void,
		direction: i32,
	) -> i32;
	fn hipfftExecD2Z(plan: *mut c_void, idata: *mut c_void, odata: *mut c_void) -> i32;
}


pub fn gpu_dasum(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let mut result = 0.0f64;
	let status = unsafe {
		hipblasDasum(
			hipblas_handle(),
			safe_i32(n),
			x.ptr_raw() as *const f64,
			1,
			&mut result,
		)
	};
	check(status)?;
	out.load(&[result])
}

pub fn gpu_idamax(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let mut result: i32 = 0;
	let status = unsafe {
		hipblasIdamax(
			hipblas_handle(),
			safe_i32(n),
			x.ptr_raw() as *const f64,
			1,
			&mut result,
		)
	};
	check(status)?;
	out.load(&[(result - 1).max(0) as f64])
}





pub fn gpu_dsyrk(
	a: &GpuBuffer,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	let status = unsafe {
		hipblasDsyrk(
			hipblas_handle(),
			FILL_LOWER,
			OP_NONE,
			n as i32,
			k as i32,
			&alpha,
			a.ptr_raw() as *const f64,
			k as i32,
			&beta,
			out.ptr_raw() as *mut f64,
			n as i32,
		)
	};
	check(status)
}


#[allow(clippy::too_many_arguments)]
pub fn gpu_bmm_into(
	a: &GpuBuffer,
	b: &GpuBuffer,
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
	let status = unsafe {
		hipblasDgemmStridedBatched(
			hipblas_handle(),
			op_b,
			op_a,
			n as i32,
			m as i32,
			k as i32,
			&alpha,
			b.as_ptr_offset(b_off) as *const f64,
			ldb as i32,
			stride_b as i64,
			a.as_ptr_offset(a_off) as *const f64,
			lda as i32,
			stride_a as i64,
			&beta,
			out.as_ptr_offset(c_off) as *mut f64,
			ldc as i32,
			stride_c as i64,
			batch as i32,
		)
	};
	check(status)
}


pub fn gpu_lu_factor_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			n as i32,
			n as i32,
			std::ptr::null_mut(),
			n as i32,
			&mut lwork,
		);
	}
	(lwork.max(1) as usize) * 8
}

pub fn gpu_lu_factor(
	a: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	lu_out: &GpuBuffer,
	ipiv_out: &GpuBuffer,
	info_out: &GpuBuffer,
) -> Result<(), HipError> {
	crate::kernels::gpu_copy_into(a, n * n, lu_out)?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			n as i32,
			n as i32,
			lu_out.ptr_raw() as *mut f64,
			n as i32,
			&mut lwork,
		);
	}
	let status = unsafe {
		hipsolverDgetrf(
			hipsolver_handle(),
			n as i32,
			n as i32,
			lu_out.ptr_raw() as *mut f64,
			n as i32,
			work.ptr_raw() as *mut f64,
			lwork,
			ipiv_out.ptr_raw() as *mut i32,
			info_out.ptr_raw() as *mut i32,
		)
	};
	check(status)
}

pub fn gpu_lu_solve_workspace_bytes(n: usize, nrhs: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			OP_NONE,
			n as i32,
			nrhs as i32,
			std::ptr::null_mut(),
			n as i32,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			n as i32,
			&mut lwork,
		);
	}
	(lwork.max(1) as usize) * 8
}

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
	crate::kernels::gpu_copy_into(b, n * nrhs, x_out)?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgetrs_bufferSize(
			hipsolver_handle(),
			OP_NONE,
			n as i32,
			nrhs as i32,
			lu.ptr_raw() as *mut f64,
			n as i32,
			ipiv.ptr_raw() as *mut i32,
			x_out.ptr_raw() as *mut f64,
			n as i32,
			&mut lwork,
		);
	}
	let status = unsafe {
		hipsolverDgetrs(
			hipsolver_handle(),
			OP_NONE,
			n as i32,
			nrhs as i32,
			lu.ptr_raw() as *mut f64,
			n as i32,
			ipiv.ptr_raw() as *mut i32,
			x_out.ptr_raw() as *mut f64,
			n as i32,
			work.ptr_raw() as *mut f64,
			lwork,
			info_out.ptr_raw() as *mut i32,
		)
	};
	check(status)
}


pub fn gpu_potrs_workspace_bytes(n: usize, nrhs: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDpotrs_bufferSize(
			hipsolver_handle(),
			SOLVER_FILL_UPPER,
			n as i32,
			nrhs as i32,
			std::ptr::null_mut(),
			n as i32,
			std::ptr::null_mut(),
			n as i32,
			&mut lwork,
		);
	}
	(lwork.max(1) as usize) * 8
}

pub fn gpu_potrs(
	l: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
	work: &GpuBuffer,
	info_out: &GpuBuffer,
	x_out: &GpuBuffer,
) -> Result<(), HipError> {
	crate::kernels::gpu_copy_into(b, n * nrhs, x_out)?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDpotrs_bufferSize(
			hipsolver_handle(),
			SOLVER_FILL_UPPER,
			n as i32,
			nrhs as i32,
			l.ptr_raw() as *mut f64,
			n as i32,
			x_out.ptr_raw() as *mut f64,
			n as i32,
			&mut lwork,
		);
	}
	let status = unsafe {
		hipsolverDpotrs(
			hipsolver_handle(),
			SOLVER_FILL_UPPER,
			n as i32,
			nrhs as i32,
			l.ptr_raw() as *mut f64,
			n as i32,
			x_out.ptr_raw() as *mut f64,
			n as i32,
			work.ptr_raw() as *mut f64,
			lwork,
			info_out.ptr_raw() as *mut i32,
		)
	};
	check(status)
}


pub fn gpu_qr_workspace_bytes(m: usize, n: usize) -> usize {
	let k = m.min(n);
	let mut lwork: i32 = 0;
	let mut lwork_q: i32 = 0;
	unsafe {
		hipsolverDgeqrf_bufferSize(
			hipsolver_handle(),
			m as i32,
			n as i32,
			std::ptr::null_mut(),
			m as i32,
			&mut lwork,
		);
		hipsolverDorgqr_bufferSize(
			hipsolver_handle(),
			m as i32,
			n as i32,
			k as i32,
			std::ptr::null_mut(),
			m as i32,
			std::ptr::null_mut(),
			&mut lwork_q,
		);
	}
	(lwork.max(lwork_q).max(1) as usize) * 8
}

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

	crate::kernels::gpu_transpose(a, m, n, q_out)?;

	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgeqrf_bufferSize(
			hipsolver_handle(),
			m as i32,
			n as i32,
			q_out.ptr_raw() as *mut f64,
			m as i32,
			&mut lwork,
		);
	}
	let status = unsafe {
		hipsolverDgeqrf(
			hipsolver_handle(),
			m as i32,
			n as i32,
			q_out.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *mut f64,
			work.ptr_raw() as *mut f64,
			lwork,
			info_out.ptr_raw() as *mut i32,
		)
	};
	check(status)?;

	crate::kernels::gpu_pack_upper_tri(q_out, m, n, r_out)?;

	let mut lwork_q: i32 = 0;
	unsafe {
		hipsolverDorgqr_bufferSize(
			hipsolver_handle(),
			m as i32,
			n as i32,
			k as i32,
			q_out.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *mut f64,
			&mut lwork_q,
		);
	}
	let status = unsafe {
		hipsolverDorgqr(
			hipsolver_handle(),
			m as i32,
			n as i32,
			k as i32,
			q_out.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *mut f64,
			work.ptr_raw() as *mut f64,
			lwork_q,
			info_out.ptr_raw() as *mut i32,
		)
	};
	check(status)
}


pub fn gpu_eigh_sym_workspace_bytes(n: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDsyevd_bufferSize(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			n as i32,
			std::ptr::null_mut(),
			n as i32,
			std::ptr::null_mut(),
			&mut lwork,
		);
	}
	(lwork.max(1) as usize) * 8
}

pub fn gpu_eigh_sym(
	a: &GpuBuffer,
	n: usize,
	work: &GpuBuffer,
	info_out: &GpuBuffer,
	evals_out: &GpuBuffer,
	evecs_out: &GpuBuffer,
) -> Result<(), HipError> {
	crate::kernels::gpu_copy_into(a, n * n, evecs_out)?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDsyevd_bufferSize(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			n as i32,
			evecs_out.ptr_raw() as *mut f64,
			n as i32,
			evals_out.ptr_raw() as *mut f64,
			&mut lwork,
		);
	}
	let status = unsafe {
		hipsolverDsyevd(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			n as i32,
			evecs_out.ptr_raw() as *mut f64,
			n as i32,
			evals_out.ptr_raw() as *mut f64,
			work.ptr_raw() as *mut f64,
			lwork,
			info_out.ptr_raw() as *mut i32,
		)
	};
	check(status)
}


pub fn gpu_svd_workspace_bytes(m: usize, n: usize) -> usize {
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgesvd_bufferSize(
			hipsolver_handle(),
			SOLVER_JOB_ALL,
			SOLVER_JOB_ALL,
			m as i32,
			n as i32,
			&mut lwork,
		);
	}
	((lwork.max(1) as usize) + m * n + n * n) * 8
}

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
	let a_cm = work.view(0, m * n);
	let v = work.view(m * n, n * n);
	let solver_off = m * n + n * n;

	crate::kernels::gpu_transpose(a, m, n, &a_cm)?;

	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgesvd_bufferSize(
			hipsolver_handle(),
			SOLVER_JOB_ALL,
			SOLVER_JOB_ALL,
			m as i32,
			n as i32,
			&mut lwork,
		);
	}
	let status = unsafe {
		hipsolverDgesvd(
			hipsolver_handle(),
			SOLVER_JOB_ALL,
			SOLVER_JOB_ALL,
			m as i32,
			n as i32,
			a_cm.ptr_raw() as *mut f64,
			m as i32,
			s_out.ptr_raw() as *mut f64,
			u_out.ptr_raw() as *mut f64,
			m as i32,
			v.ptr_raw() as *mut f64,
			n as i32,
			work.as_ptr_offset(solver_off) as *mut f64,
			lwork,
			std::ptr::null_mut(),
			info_out.ptr_raw() as *mut i32,
		)
	};
	check(status)?;
	crate::kernels::gpu_transpose(&v, n, n, vt_out)
}


struct CachedFftPlan {
	plan: usize,
}
unsafe impl Send for CachedFftPlan {}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct FftKey {
	fft_type: i32,
	n: usize,
}

static FFT_CACHE: OnceLock<Mutex<HashMap<FftKey, CachedFftPlan>>> = OnceLock::new();

fn fft_plan(fft_type: i32, n: usize) -> Result<*mut c_void, HipError> {
	let mut cache = FFT_CACHE
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.expect("fft cache poisoned");
	let key = FftKey { fft_type, n };
	let cached = cache.get(&key).map(|entry| entry.plan);
	match cached {
		Some(plan) => Ok(plan as *mut c_void),
		None => {
			let mut plan: *mut c_void = std::ptr::null_mut();
			let status = unsafe { hipfftPlan1d(&mut plan, n as i32, fft_type, 1) };
			check(status)?;
			cache.insert(key, CachedFftPlan { plan: plan as usize });
			Ok(plan)
		}
	}
}

pub fn gpu_fft_c2c_1d(
	input: &GpuBuffer,
	n: usize,
	forward: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let plan = fft_plan(HIPFFT_Z2Z, n)?;
	let direction = match forward.cmp(&0) {
		Ordering::Equal => HIPFFT_BACKWARD,
		Ordering::Less | Ordering::Greater => HIPFFT_FORWARD,
	};
	let status = unsafe { hipfftExecZ2Z(plan, input.ptr_raw(), out.ptr_raw(), direction) };
	check(status)
}

pub fn gpu_rfft_1d(input_real: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let plan = fft_plan(HIPFFT_D2Z, n)?;
	let status = unsafe { hipfftExecD2Z(plan, input_real.ptr_raw(), out.ptr_raw()) };
	check(status)
}
