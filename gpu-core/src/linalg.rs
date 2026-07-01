use crate::hip::{HipError, check};
use crate::kernels::{hipblas_handle, hipsolver_handle, safe_i32};
use crate::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

// ── hipBLAS operation / fill / side / diag enum constants ─────────────────
const OP_NONE: u32 = 111;
const OP_TRANS: u32 = 112;
const FILL_LOWER: u32 = 121;

// ── hipSOLVER enum constants ──────────────────────────────────────────────
// op_n=111, fill: upper=121 / lower=122, eig jobz vectors=202, gesvd job 'A'=65 / 'N'=78
// 121: the factor lives in the same triangle the hipBLAS dtrsm solves read.
const SOLVER_FILL_LOWER: u32 = 122;
// col-major UPPER. gpu_cholesky factors with this (so the factor reads row-major as
// the lower L); gpu_potrs must consume it with the SAME fill mode to solve correctly.
const SOLVER_FILL_UPPER: u32 = 121;
const SOLVER_EIG_VECTOR: u32 = 202;
const SOLVER_JOB_ALL: i8 = 65; // 'A' — all singular vectors

// ── hipFFT enum constants ─────────────────────────────────────────────────
const HIPFFT_Z2Z: i32 = 0x69;
const HIPFFT_D2Z: i32 = 0x6a;
const HIPFFT_FORWARD: i32 = -1;
const HIPFFT_BACKWARD: i32 = 1;

// ── extern declarations ───────────────────────────────────────────────────
// Every prototype transcribed slot-by-slot from the header greps above.
// Enums are u32, hipblas_stride is i64, size_t is usize.
unsafe extern "C" {
	// ── hipBLAS L1 ─────────────────────────────────────────────────────
	// hipblasDdot(handle, n, x, incx, y, incy, result) -> i32
	fn hipblasDdot(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		y: *const f64,
		incy: i32,
		result: *mut f64,
	) -> i32;

	// hipblasDnrm2(handle, n, x, incx, result) -> i32
	fn hipblasDnrm2(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut f64,
	) -> i32;

	// hipblasDasum(handle, n, x, incx, result) -> i32
	fn hipblasDasum(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut f64,
	) -> i32;

	// hipblasIdamax(handle, n, x, incx, result) -> i32  [result is 1-based BLAS index]
	fn hipblasIdamax(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut i32,
	) -> i32;

	// ── hipBLAS L2 ─────────────────────────────────────────────────────
	// hipblasDgemv(handle, trans, m, n, alpha, A, lda, x, incx, beta, y, incy) -> i32
	fn hipblasDgemv(
		handle: *mut c_void,
		trans: u32,
		m: i32,
		n: i32,
		alpha: *const f64,
		A: *const f64,
		lda: i32,
		x: *const f64,
		incx: i32,
		beta: *const f64,
		y: *mut f64,
		incy: i32,
	) -> i32;

	// hipblasDger(handle, m, n, alpha, x, incx, y, incy, A, lda) -> i32
	fn hipblasDger(
		handle: *mut c_void,
		m: i32,
		n: i32,
		alpha: *const f64,
		x: *const f64,
		incx: i32,
		y: *const f64,
		incy: i32,
		A: *mut f64,
		lda: i32,
	) -> i32;

	// ── hipBLAS L3 ─────────────────────────────────────────────────────
	// hipblasDsyrk(handle, uplo, transA, n, k, alpha, A, lda, beta, C, ldc) -> i32
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

	// hipblasDgemmStridedBatched(handle, transA, transB, m, n, k,
	//   alpha, A, lda, stride_a, B, ldb, stride_b, beta, C, ldc, stride_c, batch_count) -> i32
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

	// ── hipSOLVER ──────────────────────────────────────────────────────
	// hipSOLVER requires an explicit device workspace (work/lwork) on every
	// compute call; each op has a paired _bufferSize query to size it.
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

	// ── hipFFT ─────────────────────────────────────────────────────────
	// Plans auto-allocate their own work area; no setup/teardown or exec-info.
	fn hipfftPlan1d(plan: *mut *mut c_void, nx: i32, fft_type: i32, batch: i32) -> i32;
	fn hipfftExecZ2Z(
		plan: *mut c_void,
		idata: *mut c_void,
		odata: *mut c_void,
		direction: i32,
	) -> i32;
	fn hipfftExecD2Z(plan: *mut c_void, idata: *mut c_void, odata: *mut c_void) -> i32;
}

// helper: copy n f64 elements out of a GpuBuffer into a fresh buffer
fn gpu_copy_n(src: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	crate::kernels::gpu_copy(src, n)
}

// ── L1 scalar routines ────────────────────────────────────────────────────

// Dot product: a · b  (n elements, stride 1).
// The shared handle stays in host-pointer mode; result is written to a stack
// variable via host-pointer path — no GpuBuffer download needed.
pub fn gpu_ddot(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	let mut result = 0.0f64;
	let status = unsafe {
		hipblasDdot(
			hipblas_handle(),
			safe_i32(n),
			a.ptr_raw() as *const f64,
			1,
			b.ptr_raw() as *const f64,
			1,
			&mut result,
		)
	};
	check(status)?;
	Ok(result)
}

// Euclidean norm: ||x||_2  (n elements, stride 1).
pub fn gpu_dnrm2(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	let mut result = 0.0f64;
	let status = unsafe {
		hipblasDnrm2(
			hipblas_handle(),
			safe_i32(n),
			x.ptr_raw() as *const f64,
			1,
			&mut result,
		)
	};
	check(status)?;
	Ok(result)
}

// Sum of absolute values: sum |x_i|  (n elements, stride 1).
pub fn gpu_dasum(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
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
	Ok(result)
}

// Index of element with largest absolute value (n elements, stride 1).
// hipBLAS returns a 1-based BLAS index; subtract 1 for 0-based usize.
pub fn gpu_idamax(x: &GpuBuffer, n: usize) -> Result<usize, HipError> {
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
	Ok((result - 1).max(0) as usize)
}

// ── L2 routines ───────────────────────────────────────────────────────────

// Matrix-vector multiply: y = A @ x  (if !trans)  or  y = A^T @ x  (if trans).
// A is row-major (m x n).  Output length: m when !trans, n when trans.
//
// hipBLAS is column-major.  For row-major A(m×n) stored as contiguous f64:
//   A_rm treated as A_cm^T(n×m).
// To compute y = A_rm @ x  (A col, x len n, y len m):
//   hipblasDgemv(TRANS, n, m, 1, A, n, x, 1, 0, y, 1)
//   — the library transposes A_cm → which is A_rm, times x(n) → y(m).
// To compute y = A_rm^T @ x  (A col, x len m, y len n):
//   hipblasDgemv(NONE, n, m, 1, A, n, x, 1, 0, y, 1)
pub fn gpu_dgemv(
	a: &GpuBuffer,
	x: &GpuBuffer,
	m: usize,
	n: usize,
	trans: bool,
) -> Result<GpuBuffer, HipError> {
	let out_len = if trans { n } else { m };
	let out = GpuBuffer::alloc(out_len)?;
	let alpha = 1.0f64;
	let beta = 0.0f64;
	let (rb_trans, rb_m, rb_n) = if trans {
		(OP_NONE, n as i32, m as i32)
	} else {
		(OP_TRANS, n as i32, m as i32)
	};
	let status = unsafe {
		hipblasDgemv(
			hipblas_handle(),
			rb_trans,
			rb_m,
			rb_n,
			&alpha,
			a.ptr_raw() as *const f64,
			rb_m,
			x.ptr_raw() as *const f64,
			1,
			&beta,
			out.ptr_raw() as *mut f64,
			1,
		)
	};
	check(status)?;
	Ok(out)
}

// Rank-1 update: A = x ⊗ y^T  (outer product), A is (m x n) row-major output.
// Allocates zeroed output first (hipblasDger accumulates into A).
//
// Column-major dger computes A_cm += alpha * x_cm * y_cm^T.
// For row-major result: we want A_rm = x(m) ⊗ y(n)^T.
// A_rm(i,j) = x[i]*y[j].  Stored as A_cm^T(n,m).
// Pass y as the "x" operand (length n) and x as the "y" operand (length m),
// with m_rb=n, n_rb=m, lda=n — hipBLAS writes A_cm(j,i)=y[j]*x[i]=A_rm(i,j). ✓
pub fn gpu_dger(x: &GpuBuffer, y: &GpuBuffer, m: usize, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(m * n)?;
	unsafe {
		crate::memory::memset_sync(out.ptr_raw(), 0, m * n * std::mem::size_of::<f64>())?;
	}
	let alpha = 1.0f64;
	let status = unsafe {
		hipblasDger(
			hipblas_handle(),
			n as i32,
			m as i32, // m_rb=n, n_rb=m (column-major transposed layout)
			&alpha,
			y.ptr_raw() as *const f64,
			1, // "x" operand in hipBLAS
			x.ptr_raw() as *const f64,
			1, // "y" operand in hipBLAS
			out.ptr_raw() as *mut f64,
			n as i32, // lda=n (stride between col-major cols = n)
		)
	};
	check(status)?;
	Ok(out)
}

// ── Symmetric rank-k update ───────────────────────────────────────────────

// C = A^T @ A  where A is (k x n) row-major; result C is (n x n) symmetric.
// Only the lower triangle of C is written (upper is garbage — matches gpu_cholesky convention).
//
// hipBLAS dsyrk(uplo, transA, n, k, alpha, A, lda, beta, C, ldc).
// We want C = A_rm^T @ A_rm.
// A_rm(k×n) stored as A_cm^T(n×k).  To get C = A_rm^T@A_rm = A_cm @ A_cm^T,
// call dsyrk with transA=NONE (C += A_cm * A_cm^T), lda=k (stride between cm cols).
// Wait — A_rm stored row-major means lda_rm=n; as cm that is n cols of length k → lda_cm=k.
// dsyrk NONE: C_cm(n,n) = A_cm(k,n)^{colwise} × A_cm^T → but NONE means C+=A*A^T?
// hipBLAS dsyrk: if transA=NONE, C += alpha*A*A^T (A is n×k cm, so n×k@k×n=n×n ✓).
// With lda=k that interprets A as cm n×k — but A_rm(k×n) as cm is (n×k), so lda_cm=k. ✓
// Result: C(n×n) cm lower triangle = A_rm^T @ A_rm in rm. ✓
pub fn gpu_dsyrk(a: &GpuBuffer, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
	let c = GpuBuffer::alloc(n * n)?;
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
			k as i32, // lda=k: A_rm(k×n) as cm n×k
			&beta,
			c.ptr_raw() as *mut f64,
			n as i32,
		)
	};
	check(status)?;
	Ok(c)
}

// ── Strided batched GEMM ──────────────────────────────────────────────────

// C[i] = A[i] @ B[i]  for i in 0..batch.  A is (m×k), B is (k×n), C is (m×n), all row-major.
// Strides: stride_a = m*k, stride_b = k*n, stride_c = m*n.
//
// Mirror gpu_gemm's column-major identity: C_rm = (B_cm @ A_cm)^T.
// hipblasDgemmStridedBatched(N, N, n, m, k, 1, B, n, stride_b, A, k, stride_a, 0, C, n, stride_c, batch).
// hipBLAS "A" = our B (lda=n, stride=k*n), "B" = our A (ldb=k, stride=m*k). ✓
pub fn gpu_dgemm_strided_batched(
	a: &GpuBuffer,
	b: &GpuBuffer,
	batch: usize,
	m: usize,
	n: usize,
	k: usize,
) -> Result<GpuBuffer, HipError> {
	let c = GpuBuffer::alloc(batch * m * n)?;
	let alpha = 1.0f64;
	let beta = 0.0f64;
	let stride_a = (m * k) as i64;
	let stride_b = (k * n) as i64;
	let stride_c = (m * n) as i64;
	let status = unsafe {
		hipblasDgemmStridedBatched(
			hipblas_handle(),
			OP_NONE,
			OP_NONE,
			n as i32,
			m as i32,
			k as i32,
			&alpha,
			b.ptr_raw() as *const f64,
			n as i32,
			stride_b, // hipBLAS "A" = our B
			a.ptr_raw() as *const f64,
			k as i32,
			stride_a, // hipBLAS "B" = our A
			&beta,
			c.ptr_raw() as *mut f64,
			n as i32,
			stride_c,
			batch as i32,
		)
	};
	check(status)?;
	Ok(c)
}

/// Alloc-free batched GEMM into a preallocated `c`, per batch i (all row-major):
///   C_i(m×n, ld=ldc) = opA(A_i) @ opB(B_i),  opX = transpose iff trans_x.
/// A_i begins at element `a_off + i*stride_a` (row stride `lda`), B_i at
/// `b_off + i*stride_b` (ld `ldb`), C_i at `c_off + i*stride_c` (ld `ldc`).
/// `k` is the contraction dim. Leading dims allow per-head sub-matrix views
/// (e.g. one head's `hd` columns of a packed [S, heads*hd] block: lda = full d).
///
/// hipBLAS is column-major; a row-major (r×c, ld) matrix is its col-major transpose.
/// So C_rm = opA(A)@opB(B) is computed as C_cm = (Cᵀ): hipBLAS(transA=opB, transB=opA,
/// m=n, n=m, k=k, A_roc=B, B_roc=A) — derived op-by-op, matches gpu_dgemm_strided_batched
/// for the no-transpose case.
#[allow(clippy::too_many_arguments)]
pub fn gpu_bmm_into(
	c: &GpuBuffer,
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
	trans_a: bool,
	trans_b: bool,
) {
	let alpha = 1.0f64;
	let beta = 0.0f64;
	let op_a = if trans_a { OP_TRANS } else { OP_NONE };
	let op_b = if trans_b { OP_TRANS } else { OP_NONE };
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
			c.as_ptr_offset(c_off) as *mut f64,
			ldc as i32,
			stride_c as i64,
			batch as i32,
		)
	};
	check(status).expect("gpu_bmm_into: hipblas dgemm_strided_batched");
}

// ── rocSOLVER: LU factorization and solve ─────────────────────────────────

// LU factorization: PA = LU  (n×n).  Returns (lu_buf, ipiv_buf) on GPU.
// lu_buf is a copy of A overwritten by the factorization (A is preserved).
// ipiv_buf holds n i32 pivot indices (1-based LAPACK convention).
// Callers pass lu_buf to gpu_lu_solve; do not read its content directly.
pub fn gpu_lu_factor(a: &GpuBuffer, n: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
	let lu = gpu_copy_n(a, n * n)?;
	let ipiv = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgetrf_bufferSize(
			hipsolver_handle(),
			n as i32,
			n as i32,
			lu.ptr_raw() as *mut f64,
			n as i32,
			&mut lwork,
		);
	}
	let work = GpuBuffer::alloc_bytes((lwork.max(1) as usize) * 8)?;
	let status = unsafe {
		hipsolverDgetrf(
			hipsolver_handle(),
			n as i32,
			n as i32,
			lu.ptr_raw() as *mut f64,
			n as i32,
			work.ptr_raw() as *mut f64,
			lwork,
			ipiv.ptr_raw() as *mut i32,
			info.ptr_raw() as *mut i32,
		)
	};
	check(status)?;
	Ok((lu, ipiv))
}

// Solve A*X = B using a pre-factored LU (from gpu_lu_factor).
// A is n×n, B is n×nrhs (both row-major).  Returns solution X on GPU.
// dgetrs operates on the factorization in-place and overwrites B.
// B is copied; the lu factor is read-only (ipiv is const in the header).
pub fn gpu_lu_solve(
	lu: &GpuBuffer,
	ipiv: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
) -> Result<GpuBuffer, HipError> {
	let b_copy = gpu_copy_n(b, n * nrhs)?;
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
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
			b_copy.ptr_raw() as *mut f64,
			n as i32,
			&mut lwork,
		);
	}
	let work = GpuBuffer::alloc_bytes((lwork.max(1) as usize) * 8)?;
	let status = unsafe {
		hipsolverDgetrs(
			hipsolver_handle(),
			OP_NONE,
			n as i32,
			nrhs as i32,
			lu.ptr_raw() as *mut f64,
			n as i32,
			ipiv.ptr_raw() as *mut i32,
			b_copy.ptr_raw() as *mut f64,
			n as i32,
			work.ptr_raw() as *mut f64,
			lwork,
			info.ptr_raw() as *mut i32,
		)
	};
	check(status)?;
	Ok(b_copy)
}

// ── rocSOLVER: Cholesky solve given pre-factored L ────────────────────────

// Cholesky solve: L*L^T * X = B  (A is n×n SPD already factored by gpu_cholesky).
// n×n factor L (lower triangle), B is n×nrhs.  Returns X on GPU.
// dpotrs reads the factor but does NOT overwrite it; only B is mutated.
// uplo = 121 (lower) matches gpu_cholesky's convention.
pub fn gpu_potrs(
	l: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	nrhs: usize,
) -> Result<GpuBuffer, HipError> {
	let b_copy = gpu_copy_n(b, n * nrhs)?;
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDpotrs_bufferSize(
			hipsolver_handle(),
			SOLVER_FILL_UPPER, // match gpu_cholesky's potrf fill mode
			n as i32,
			nrhs as i32,
			l.ptr_raw() as *mut f64,
			n as i32,
			b_copy.ptr_raw() as *mut f64,
			n as i32,
			&mut lwork,
		);
	}
	let work = GpuBuffer::alloc_bytes((lwork.max(1) as usize) * 8)?;
	let status = unsafe {
		hipsolverDpotrs(
			hipsolver_handle(),
			SOLVER_FILL_UPPER, // match gpu_cholesky's potrf fill mode
			n as i32,
			nrhs as i32,
			l.ptr_raw() as *mut f64,
			n as i32,
			b_copy.ptr_raw() as *mut f64,
			n as i32,
			work.ptr_raw() as *mut f64,
			lwork,
			info.ptr_raw() as *mut i32,
		)
	};
	check(status)?;
	Ok(b_copy)
}

// ── rocSOLVER: QR factorization ───────────────────────────────────────────

// QR decomposition of A (m×n, row-major, m >= n).  Returns (q, r).
// q is (m×n) col-major buffer (Q columns are orthonormal basis vectors).
// r is (n×n) col-major upper-triangular; R[i,j] = r[j*n+i] for i<=j.
// Caller reconstructs A as: A[i,j] = sum_k Q[i,k] * R[k,j]
//   where Q[i,k] = q[k*m+i] and R[k,j] = r[j*n+k] (k<=j, else 0).
//
// rocSOLVER is column-major.  To factor the row-major A correctly:
//   1. Transpose A_rm (m×n) → factor (m×n col-major, lda=m via gpu_transpose).
//      Now factor[j*m+i] = A[i,j], which is A in the column-major form dgeqrf expects.
//   2. dgeqrf(m, n, factor, lda=m) overwrites factor with Householder vectors + R.
//   3. Extract R: R is stored in the upper triangle of factor with lda=m.
//      Copy the full m*n factor into a compact n×n buffer using a strided extraction:
//      R[i,j] lives at factor[j*m+i]; copy to r[j*n+i] for 0<=i<=j<n.
//   4. dorgqr(m, n, k, factor, lda=m) expands Householder vectors into explicit Q (m×n col-major).
pub fn gpu_qr(a: &GpuBuffer, m: usize, n: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
	let k = m.min(n);

	// Step 1: transpose A_rm to col-major layout (factor[j*m+i] = A[i,j]).
	let factor = crate::kernels::gpu_transpose(a, m, n)?;
	let tau = GpuBuffer::alloc(k)?;

	// Step 2: QR factorize in-place; lda=m is correct for the col-major factor.
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDgeqrf_bufferSize(
			hipsolver_handle(),
			m as i32,
			n as i32,
			factor.ptr_raw() as *mut f64,
			m as i32,
			&mut lwork,
		);
	}
	let work = GpuBuffer::alloc_bytes((lwork.max(1) as usize) * 8)?;
	let status = unsafe {
		hipsolverDgeqrf(
			hipsolver_handle(),
			m as i32,
			n as i32,
			factor.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *mut f64,
			work.ptr_raw() as *mut f64,
			lwork,
			info.ptr_raw() as *mut i32,
		)
	};
	check(status)?;

	// Step 3: Extract R (n×n col-major) from the upper triangle of factor (lda=m),
	// entirely on the GPU — r[j*n+i] = factor[j*m+i] for i<=j, else 0. Must run
	// before dorgqr below overwrites factor with Q. (No GPU→CPU→GPU round trip.)
	let r = GpuBuffer::alloc(n * n)?;
	crate::kernels::gpu_pack_upper_tri(&factor, &r, m, n);

	// Step 4: expand Householder reflectors → explicit Q (m×n col-major, lda=m).
	let mut lwork_q: i32 = 0;
	unsafe {
		hipsolverDorgqr_bufferSize(
			hipsolver_handle(),
			m as i32,
			n as i32,
			k as i32,
			factor.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *mut f64,
			&mut lwork_q,
		);
	}
	let work_q = GpuBuffer::alloc_bytes((lwork_q.max(1) as usize) * 8)?;
	let info_q = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;
	let status = unsafe {
		hipsolverDorgqr(
			hipsolver_handle(),
			m as i32,
			n as i32,
			k as i32,
			factor.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *mut f64,
			work_q.ptr_raw() as *mut f64,
			lwork_q,
			info_q.ptr_raw() as *mut i32,
		)
	};
	check(status)?;

	Ok((factor, r))
}

// ── rocSOLVER: symmetric eigendecomposition ───────────────────────────────

// Symmetric eigen-decomposition of A (n×n, symmetric, row-major).
// Returns (evals, evecs): evals is length-n ascending eigenvalues;
// evecs is (n×n) where COLUMNS (in column-major storage = rows in row-major) are eigenvectors.
// A_rm == A_cm for symmetric matrices, so no layout adjustment needed.
// dsyevd overwrites A with eigenvectors; we copy A first.
pub fn gpu_eigh_sym(a: &GpuBuffer, n: usize) -> Result<(GpuBuffer, GpuBuffer), HipError> {
	let evecs = gpu_copy_n(a, n * n)?;
	let evals = GpuBuffer::alloc(n)?;
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;

	let mut lwork: i32 = 0;
	unsafe {
		hipsolverDsyevd_bufferSize(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			n as i32,
			evecs.ptr_raw() as *mut f64,
			n as i32,
			evals.ptr_raw() as *mut f64,
			&mut lwork,
		);
	}
	let work = GpuBuffer::alloc_bytes((lwork.max(1) as usize) * 8)?;
	let status = unsafe {
		hipsolverDsyevd(
			hipsolver_handle(),
			SOLVER_EIG_VECTOR,
			SOLVER_FILL_LOWER,
			n as i32,
			evecs.ptr_raw() as *mut f64,
			n as i32,
			evals.ptr_raw() as *mut f64,
			work.ptr_raw() as *mut f64,
			lwork,
			info.ptr_raw() as *mut i32,
		)
	};
	check(status)?;
	Ok((evals, evecs))
}

// ── rocSOLVER: SVD ────────────────────────────────────────────────────────

// SVD: A = U * diag(S) * Vt  where A is (m×n) row-major.
// Returns (u, s, vt): u is (m×m) col-major, s is length min(m,n),
// vt is (n×n) col-major.  Reconstruction: A[i,j] = sum_k u[k*m+i]*s[k]*vt[j*n+k].
//
// rocSOLVER is column-major.  Transpose A_rm (m×n row-major) to a_cm (m×n col-major)
// so dgesvd sees A correctly.  gpu_transpose produces a_cm where a_cm[j*m+i]=A[i,j].
// dgesvd returns:
//   U (m×m col-major, ldu=m): left singular vectors as columns.
//   V (n×n col-major, ldv=n): right singular vectors as columns (NOT V^T).
// Transpose V → Vt so callers use a uniform U·diag(S)·Vt contract.
// E (superdiagonal workspace) has length max(1, min(m,n)-1).
pub fn gpu_svd(
	a: &GpuBuffer,
	m: usize,
	n: usize,
) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
	let k = m.min(n);
	let a_cm = crate::kernels::gpu_transpose(a, m, n)?;
	let s = GpuBuffer::alloc(k)?;
	let u = GpuBuffer::alloc(m * m)?;
	let v = GpuBuffer::alloc(n * n)?;
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;

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
	let work = GpuBuffer::alloc_bytes((lwork.max(1) as usize) * 8)?;
	let status = unsafe {
		hipsolverDgesvd(
			hipsolver_handle(),
			SOLVER_JOB_ALL,
			SOLVER_JOB_ALL,
			m as i32,
			n as i32,
			a_cm.ptr_raw() as *mut f64,
			m as i32,
			s.ptr_raw() as *mut f64,
			u.ptr_raw() as *mut f64,
			m as i32,
			v.ptr_raw() as *mut f64,
			n as i32,
			work.ptr_raw() as *mut f64,
			lwork,
			std::ptr::null_mut(),
			info.ptr_raw() as *mut i32,
		)
	};
	check(status)?;
	let vt = crate::kernels::gpu_transpose(&v, n, n)?;
	Ok((u, s, vt))
}

// ── rocFFT: complex 1-D FFT ───────────────────────────────────────────────

// 1-D complex-to-complex FFT.  Input: 2*n f64 (interleaved re/im, n complex numbers).
// Output: 2*n f64 (same format).  forward=true → DFT, forward=false → IDFT.
// rocFFT plans are expensive to create/destroy; cache them for the process lifetime
// keyed by (transform_type, n). Each cached entry owns its work buffer + execution
// info, so repeated transforms of the same shape reuse the plan with zero setup.
// Single-GPU-stream use only — the shared work buffer is not re-entrant.
struct CachedFftPlan {
	plan: usize,
}
// SAFETY: hipFFT plan handles live for the process and are only read out under
// the cache mutex on the single GPU thread.
unsafe impl Send for CachedFftPlan {}

static FFT_CACHE: OnceLock<Mutex<HashMap<(i32, usize), CachedFftPlan>>> = OnceLock::new();

// Return a cached plan for (fft_type, n), creating it on first use.
// Plans are never destroyed — that's the point of the cache. The plan
// auto-allocates its own work area, so no separate work buffer is tracked.
fn fft_plan(fft_type: i32, n: usize) -> *mut c_void {
	let mut cache = FFT_CACHE
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.expect("fft cache poisoned");
	let entry = cache.entry((fft_type, n)).or_insert_with(|| {
		let mut plan: *mut c_void = std::ptr::null_mut();
		let status = unsafe { hipfftPlan1d(&mut plan, n as i32, fft_type, 1) };
		assert_eq!(status, 0, "hipfftPlan1d failed: {}", status);
		CachedFftPlan {
			plan: plan as usize,
		}
	});
	entry.plan as *mut c_void
}

pub fn gpu_fft_c2c_1d(input: &GpuBuffer, n: usize, forward: bool) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(2 * n)?; // 2 f64 per complex element
	let plan = fft_plan(HIPFFT_Z2Z, n);
	let direction = if forward {
		HIPFFT_FORWARD
	} else {
		HIPFFT_BACKWARD
	};
	let status = unsafe { hipfftExecZ2Z(plan, input.ptr_raw(), out.ptr_raw(), direction) };
	assert_eq!(status, 0, "hipfftExecZ2Z failed: {}", status);
	Ok(out)
}

// 1-D real-to-complex FFT.  Input: n f64 (real).
// Output: 2*(n/2+1) f64 (interleaved complex, hermitian-symmetric half-spectrum).
pub fn gpu_rfft_1d(input_real: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out_complex = n / 2 + 1;
	let out = GpuBuffer::alloc(2 * out_complex)?; // 2 f64 per complex
	let plan = fft_plan(HIPFFT_D2Z, n);
	let status = unsafe { hipfftExecD2Z(plan, input_real.ptr_raw(), out.ptr_raw()) };
	assert_eq!(status, 0, "hipfftExecD2Z failed: {}", status);
	Ok(out)
}
