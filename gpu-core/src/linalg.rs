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

// ── L1 scalar routines ────────────────────────────────────────────────────
// The shared handle stays in host-pointer mode (per the vendor rule this file
// only converts signatures; adding hipblasSetPointerMode is a forbidden new
// vendor call). The scalar the vendor writes to a host stack slot is then
// stored into the caller's 1-elem device `out` via the H2D choke point.

// Sum of absolute values: sum |x_i|  (n elements, stride 1). Result -> out.
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

// Index of element with largest absolute value (n elements, stride 1).
// hipBLAS returns a 1-based BLAS index; the 0-based index is stored as f64 in
// out (1-elem). The trivial -1/max(0) stays host-side (the vendor already
// returned the value to host under host-pointer mode).
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
// trans is a plan-time flag encoded as usize (0 = A@x, nonzero = A^T@x).
// out is caller-provided: length m when trans==0, else n.

// Rank-1 update: A = x ⊗ y^T  (outer product), A is (m x n) row-major output.
// Allocates zeroed output first (hipblasDger accumulates into A).
//
// Column-major dger computes A_cm += alpha * x_cm * y_cm^T.
// For row-major result: we want A_rm = x(m) ⊗ y(n)^T.
// A_rm(i,j) = x[i]*y[j].  Stored as A_cm^T(n,m).
// Pass y as the "x" operand (length n) and x as the "y" operand (length m),
// with m_rb=n, n_rb=m, lda=n — hipBLAS writes A_cm(j,i)=y[j]*x[i]=A_rm(i,j). ✓
// hipblasDger ACCUMULATES into out, so the caller must pre-zero out (m*n)
// before the call; the op does no internal memset (conforming ops never init).

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
// out is (n×n); only the lower triangle is written (upper is garbage).
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
			k as i32, // lda=k: A_rm(k×n) as cm n×k
			&beta,
			out.ptr_raw() as *mut f64,
			n as i32,
		)
	};
	check(status)
}

// ── Strided batched GEMM ──────────────────────────────────────────────────

/// Alloc-free batched GEMM into a preallocated `c`, per batch i (all row-major):
///   C_i(m×n, ld=ldc) = opA(A_i) @ opB(B_i),  opX = transpose iff trans_x.
/// A_i begins at element `a_off + i*stride_a` (row stride `lda`), B_i at
/// `b_off + i*stride_b` (ld `ldb`), C_i at `c_off + i*stride_c` (ld `ldc`).
/// `k` is the contraction dim. Leading dims allow per-head sub-matrix views
/// (e.g. one head's `hd` columns of a packed [S, heads*hd] block: lda = full d).
///
/// hipBLAS is column-major; a row-major (r×c, ld) matrix is its col-major transpose.
/// So C_rm = opA(A)@opB(B) is computed as C_cm = (Cᵀ): hipBLAS(transA=opB, transB=opA,
/// m=n, n=m, k=k, A_roc=B, B_roc=A) — derived op-by-op, matches the legacy no-transpose batched GEMM
/// for the no-transpose case.
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
	let op_a = if trans_a != 0 { OP_TRANS } else { OP_NONE };
	let op_b = if trans_b != 0 { OP_TRANS } else { OP_NONE };
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

// ── rocSOLVER: LU factorization and solve ─────────────────────────────────

// LU factorization: PA = LU  (n×n).  Returns (lu_buf, ipiv_buf) on GPU.
// lu_buf is a copy of A overwritten by the factorization (A is preserved).
// ipiv_buf holds n i32 pivot indices (1-based LAPACK convention).
// Callers pass lu_buf to gpu_lu_solve; do not read its content directly.
// Plan-time workspace size (bytes) for gpu_lu_factor's rocSOLVER work buffer.
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

// Caller provides: work (>= gpu_lu_factor_workspace_bytes), lu_out (n*n f64,
// receives A then its LU factors), ipiv_out (n i32), info_out (1 i32).
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

// Solve A*X = B using a pre-factored LU (from gpu_lu_factor).
// A is n×n, B is n×nrhs (both row-major).  Returns solution X on GPU.
// dgetrs operates on the factorization in-place and overwrites B.
// B is copied; the lu factor is read-only (ipiv is const in the header).
// Plan-time workspace size (bytes) for gpu_lu_solve.
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

// dgetrs overwrites B in place; the op copies b into caller-provided x_out
// (n*nrhs) then solves there. work >= gpu_lu_solve_workspace_bytes, info_out 1 i32.
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

// ── rocSOLVER: Cholesky solve given pre-factored L ────────────────────────

// Cholesky solve: L*L^T * X = B  (A is n×n SPD already factored by gpu_cholesky).
// n×n factor L (lower triangle), B is n×nrhs.  Returns X on GPU.
// dpotrs reads the factor but does NOT overwrite it; only B is mutated.
// uplo = 121 (lower) matches gpu_cholesky's convention.
// Plan-time workspace size (bytes) for gpu_potrs.
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

// dpotrs overwrites B in place; the op copies b into caller-provided x_out
// (n*nrhs) then solves. uplo=UPPER matches gpu_cholesky's potrf fill mode.
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
			SOLVER_FILL_UPPER, // match gpu_cholesky's potrf fill mode
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
			SOLVER_FILL_UPPER, // match gpu_cholesky's potrf fill mode
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
// Plan-time workspace size (bytes): max of the geqrf and orgqr lwork queries,
// since gpu_qr reuses one caller work buffer for both stages.
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

// Caller provides: work (>= gpu_qr_workspace_bytes), tau (k=min(m,n) f64),
// info_out (1 i32), q_out (m*n f64 — holds the col-major factor then explicit Q),
// r_out (n*n f64, col-major upper-triangular). One work buffer serves both stages.
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

	// Step 1: transpose A_rm to col-major layout (q_out[j*m+i] = A[i,j]).
	crate::kernels::gpu_transpose(a, m, n, q_out)?;

	// Step 2: QR factorize in-place; lda=m is correct for the col-major factor.
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

	// Step 3: Extract R (n×n col-major) from the upper triangle of the factor
	// (lda=m) — r[j*n+i] = factor[j*m+i] for i<=j, else 0. Must run before dorgqr
	// below overwrites q_out with Q. (No GPU→CPU→GPU round trip.)
	crate::kernels::gpu_pack_upper_tri(q_out, m, n, r_out)?;

	// Step 4: expand Householder reflectors → explicit Q (m×n col-major, lda=m).
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

// ── rocSOLVER: symmetric eigendecomposition ───────────────────────────────

// Symmetric eigen-decomposition of A (n×n, symmetric, row-major).
// Returns (evals, evecs): evals is length-n ascending eigenvalues;
// evecs is (n×n) where COLUMNS (in column-major storage = rows in row-major) are eigenvectors.
// A_rm == A_cm for symmetric matrices, so no layout adjustment needed.
// dsyevd overwrites A with eigenvectors; we copy A first.
// Plan-time workspace size (bytes) for gpu_eigh_sym.
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

// dsyevd overwrites A with eigenvectors; the op copies a into evecs_out (n*n)
// then computes. evals_out is length-n ascending. work >= workspace_bytes.
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
// Plan-time workspace size (bytes): rocSOLVER gesvd lwork PLUS two device
// scratch regions carved from the same buffer — a_cm (m*n, the col-major input
// gesvd destroys) and v (n*n, right vectors before transpose into vt_out).
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

// Caller provides work (>= gpu_svd_workspace_bytes), info_out (1 i32),
// u_out (m*m f64), s_out (min(m,n) f64), vt_out (n*n f64). The a_cm and v
// scratch are carved from the front of work; the solver work area follows them.
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

// not-an-op: plan-time helper — return a cached hipFFT plan for (fft_type, n),
// creating it on first use. Plans are never destroyed (that's the cache's point)
// and auto-allocate their own work area, so no separate work buffer is tracked.
fn fft_plan(fft_type: i32, n: usize) -> Result<*mut c_void, HipError> {
	let mut cache = FFT_CACHE
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.expect("fft cache poisoned");
	if let Some(entry) = cache.get(&(fft_type, n)) {
		return Ok(entry.plan as *mut c_void);
	}
	let mut plan: *mut c_void = std::ptr::null_mut();
	let status = unsafe { hipfftPlan1d(&mut plan, n as i32, fft_type, 1) };
	check(status)?;
	cache.insert((fft_type, n), CachedFftPlan { plan: plan as usize });
	Ok(plan)
}

// forward is a plan-time flag as usize (nonzero = DFT, 0 = IDFT).
// out is caller-provided, length 2*n f64 (interleaved re/im).
pub fn gpu_fft_c2c_1d(
	input: &GpuBuffer,
	n: usize,
	forward: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let plan = fft_plan(HIPFFT_Z2Z, n)?;
	let direction = if forward != 0 {
		HIPFFT_FORWARD
	} else {
		HIPFFT_BACKWARD
	};
	let status = unsafe { hipfftExecZ2Z(plan, input.ptr_raw(), out.ptr_raw(), direction) };
	check(status)
}

// 1-D real-to-complex FFT.  Input: n f64 (real).
// out is caller-provided, length 2*(n/2+1) f64 (hermitian-symmetric half-spectrum).
pub fn gpu_rfft_1d(input_real: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let plan = fft_plan(HIPFFT_D2Z, n)?;
	let status = unsafe { hipfftExecD2Z(plan, input_real.ptr_raw(), out.ptr_raw()) };
	check(status)
}
