use crate::hip::{HipError, check};
use crate::kernels::{rocblas_handle, safe_i32};
use crate::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

// ── rocBLAS operation / fill / side / diag enum constants ─────────────────
const OP_NONE: u32 = 111;
const OP_TRANS: u32 = 112;
const FILL_LOWER: u32 = 121;

// ── rocsolver svect / evect / workmode enum constants ─────────────────────
const SVECT_ALL: u32 = 191;
const EVECT_ORIGINAL: u32 = 211;
const OUTOFPLACE: u32 = 201;

// ── rocFFT enum constants ─────────────────────────────────────────────────
// rocfft_transform_type: complex_forward=0, complex_inverse=1, real_forward=2, real_inverse=3
// rocfft_precision:      single=0, double=1
// rocfft_result_placement: inplace=0, notinplace=1
const ROCFFT_COMPLEX_FORWARD: u32 = 0;
const ROCFFT_COMPLEX_INVERSE: u32 = 1;
const ROCFFT_REAL_FORWARD: u32 = 2;
const ROCFFT_PRECISION_DOUBLE: u32 = 1;
const ROCFFT_NOTINPLACE: u32 = 1;

// ── extern declarations ───────────────────────────────────────────────────
// Every prototype transcribed slot-by-slot from the header greps above.
// Enums are u32, rocblas_stride is i64, size_t is usize.
unsafe extern "C" {
	// ── rocBLAS L1 ─────────────────────────────────────────────────────
	// rocblas_ddot(handle, n, x, incx, y, incy, result) -> i32
	fn rocblas_ddot(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		y: *const f64,
		incy: i32,
		result: *mut f64,
	) -> i32;

	// rocblas_dnrm2(handle, n, x, incx, result) -> i32
	fn rocblas_dnrm2(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut f64,
	) -> i32;

	// rocblas_dasum(handle, n, x, incx, result) -> i32
	fn rocblas_dasum(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut f64,
	) -> i32;

	// rocblas_idamax(handle, n, x, incx, result) -> i32  [result is 1-based BLAS index]
	fn rocblas_idamax(
		handle: *mut c_void,
		n: i32,
		x: *const f64,
		incx: i32,
		result: *mut i32,
	) -> i32;

	// ── rocBLAS L2 ─────────────────────────────────────────────────────
	// rocblas_dgemv(handle, trans, m, n, alpha, A, lda, x, incx, beta, y, incy) -> i32
	fn rocblas_dgemv(
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

	// rocblas_dger(handle, m, n, alpha, x, incx, y, incy, A, lda) -> i32
	fn rocblas_dger(
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

	// ── rocBLAS L3 ─────────────────────────────────────────────────────
	// rocblas_dsyrk(handle, uplo, transA, n, k, alpha, A, lda, beta, C, ldc) -> i32
	fn rocblas_dsyrk(
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

	// rocblas_dgemm_strided_batched(handle, transA, transB, m, n, k,
	//   alpha, A, lda, stride_a, B, ldb, stride_b, beta, C, ldc, stride_c, batch_count) -> i32
	fn rocblas_dgemm_strided_batched(
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

	// ── rocSOLVER ──────────────────────────────────────────────────────
	// rocsolver_dgetrf(handle, m, n, A, lda, ipiv, info) -> i32
	fn rocsolver_dgetrf(
		handle: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *mut i32,
		info: *mut i32,
	) -> i32;

	// rocsolver_dgetrs(handle, trans, n, nrhs, A, lda, ipiv, B, ldb) -> i32
	fn rocsolver_dgetrs(
		handle: *mut c_void,
		trans: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *const i32,
		B: *mut f64,
		ldb: i32,
	) -> i32;

	// rocsolver_dpotrs(handle, uplo, n, nrhs, A, lda, B, ldb) -> i32
	fn rocsolver_dpotrs(
		handle: *mut c_void,
		uplo: u32,
		n: i32,
		nrhs: i32,
		A: *mut f64,
		lda: i32,
		B: *mut f64,
		ldb: i32,
	) -> i32;

	// rocsolver_dgeqrf(handle, m, n, A, lda, ipiv) -> i32   [ipiv = householder scalars tau]
	fn rocsolver_dgeqrf(
		handle: *mut c_void,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *mut f64,
	) -> i32;

	// rocsolver_dorgqr(handle, m, n, k, A, lda, ipiv) -> i32
	fn rocsolver_dorgqr(
		handle: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		A: *mut f64,
		lda: i32,
		ipiv: *const f64,
	) -> i32;

	// rocsolver_dsyevd(handle, evect, uplo, n, A, lda, D, E, info) -> i32
	fn rocsolver_dsyevd(
		handle: *mut c_void,
		evect: u32,
		uplo: u32,
		n: i32,
		A: *mut f64,
		lda: i32,
		D: *mut f64,
		E: *mut f64,
		info: *mut i32,
	) -> i32;

	// rocsolver_dgesvd(handle, left_svect, right_svect, m, n, A, lda,
	//   S, U, ldu, V, ldv, E, fast_alg, info) -> i32
	fn rocsolver_dgesvd(
		handle: *mut c_void,
		left_svect: u32,
		right_svect: u32,
		m: i32,
		n: i32,
		A: *mut f64,
		lda: i32,
		S: *mut f64,
		U: *mut f64,
		ldu: i32,
		V: *mut f64,
		ldv: i32,
		E: *mut f64,
		fast_alg: u32,
		info: *mut i32,
	) -> i32;

	// ── rocFFT ─────────────────────────────────────────────────────────
	fn rocfft_setup() -> i32;

	fn rocfft_plan_create(
		plan: *mut *mut c_void,
		placement: u32,
		transform_type: u32,
		precision: u32,
		dimensions: usize,
		lengths: *const usize,
		number_of_transforms: usize,
		description: *const c_void,
	) -> i32;

	fn rocfft_execute(
		plan: *const c_void,
		in_buffer: *mut *mut c_void,
		out_buffer: *mut *mut c_void,
		info: *mut c_void,
	) -> i32;

	fn rocfft_plan_destroy(plan: *mut c_void) -> i32;

	fn rocfft_plan_get_work_buffer_size(plan: *const c_void, size_in_bytes: *mut usize) -> i32;

	fn rocfft_execution_info_create(info: *mut *mut c_void) -> i32;
	fn rocfft_execution_info_destroy(info: *mut c_void) -> i32;
	fn rocfft_execution_info_set_work_buffer(
		info: *mut c_void,
		work_buffer: *mut c_void,
		size_in_bytes: usize,
	) -> i32;
}

// ── One-time rocFFT init ───────────────────────────────────────────────────
static ROCFFT_INIT: OnceLock<()> = OnceLock::new();

fn rocfft_init() {
	ROCFFT_INIT.get_or_init(|| {
		let s = unsafe { rocfft_setup() };
		assert_eq!(s, 0, "rocfft_setup failed with status {}", s);
	});
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
		rocblas_ddot(
			rocblas_handle(),
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
		rocblas_dnrm2(
			rocblas_handle(),
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
		rocblas_dasum(
			rocblas_handle(),
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
// rocBLAS returns a 1-based BLAS index; subtract 1 for 0-based usize.
pub fn gpu_idamax(x: &GpuBuffer, n: usize) -> Result<usize, HipError> {
	let mut result: i32 = 0;
	let status = unsafe {
		rocblas_idamax(
			rocblas_handle(),
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
// rocBLAS is column-major.  For row-major A(m×n) stored as contiguous f64:
//   A_rm treated as A_cm^T(n×m).
// To compute y = A_rm @ x  (A col, x len n, y len m):
//   rocblas_dgemv(TRANS, n, m, 1, A, n, x, 1, 0, y, 1)
//   — the library transposes A_cm → which is A_rm, times x(n) → y(m).
// To compute y = A_rm^T @ x  (A col, x len m, y len n):
//   rocblas_dgemv(NONE, n, m, 1, A, n, x, 1, 0, y, 1)
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
		rocblas_dgemv(
			rocblas_handle(),
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
// Allocates zeroed output first (rocblas_dger accumulates into A).
//
// Column-major dger computes A_cm += alpha * x_cm * y_cm^T.
// For row-major result: we want A_rm = x(m) ⊗ y(n)^T.
// A_rm(i,j) = x[i]*y[j].  Stored as A_cm^T(n,m).
// Pass y as the "x" operand (length n) and x as the "y" operand (length m),
// with m_rb=n, n_rb=m, lda=n — rocBLAS writes A_cm(j,i)=y[j]*x[i]=A_rm(i,j). ✓
pub fn gpu_dger(x: &GpuBuffer, y: &GpuBuffer, m: usize, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(m * n)?;
	unsafe {
		crate::hip::hipMemset(out.ptr_raw(), 0, m * n * std::mem::size_of::<f64>());
	}
	let alpha = 1.0f64;
	let status = unsafe {
		rocblas_dger(
			rocblas_handle(),
			n as i32,
			m as i32, // m_rb=n, n_rb=m (column-major transposed layout)
			&alpha,
			y.ptr_raw() as *const f64,
			1, // "x" operand in rocBLAS
			x.ptr_raw() as *const f64,
			1, // "y" operand in rocBLAS
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
// rocBLAS dsyrk(uplo, transA, n, k, alpha, A, lda, beta, C, ldc).
// We want C = A_rm^T @ A_rm.
// A_rm(k×n) stored as A_cm^T(n×k).  To get C = A_rm^T@A_rm = A_cm @ A_cm^T,
// call dsyrk with transA=NONE (C += A_cm * A_cm^T), lda=k (stride between cm cols).
// Wait — A_rm stored row-major means lda_rm=n; as cm that is n cols of length k → lda_cm=k.
// dsyrk NONE: C_cm(n,n) = A_cm(k,n)^{colwise} × A_cm^T → but NONE means C+=A*A^T?
// rocBLAS dsyrk: if transA=NONE, C += alpha*A*A^T (A is n×k cm, so n×k@k×n=n×n ✓).
// With lda=k that interprets A as cm n×k — but A_rm(k×n) as cm is (n×k), so lda_cm=k. ✓
// Result: C(n×n) cm lower triangle = A_rm^T @ A_rm in rm. ✓
pub fn gpu_dsyrk(a: &GpuBuffer, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
	let c = GpuBuffer::alloc(n * n)?;
	let alpha = 1.0f64;
	let beta = 0.0f64;
	let status = unsafe {
		rocblas_dsyrk(
			rocblas_handle(),
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
// rocblas_dgemm_strided_batched(N, N, n, m, k, 1, B, n, stride_b, A, k, stride_a, 0, C, n, stride_c, batch).
// rocBLAS "A" = our B (lda=n, stride=k*n), "B" = our A (ldb=k, stride=m*k). ✓
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
		rocblas_dgemm_strided_batched(
			rocblas_handle(),
			OP_NONE,
			OP_NONE,
			n as i32,
			m as i32,
			k as i32,
			&alpha,
			b.ptr_raw() as *const f64,
			n as i32,
			stride_b, // rocBLAS "A" = our B
			a.ptr_raw() as *const f64,
			k as i32,
			stride_a, // rocBLAS "B" = our A
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
/// rocBLAS is column-major; a row-major (r×c, ld) matrix is its col-major transpose.
/// So C_rm = opA(A)@opB(B) is computed as C_cm = (Cᵀ): rocBLAS(transA=opB, transB=opA,
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
		rocblas_dgemm_strided_batched(
			rocblas_handle(),
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
	check(status).expect("gpu_bmm_into: rocblas dgemm_strided_batched");
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
	let status = unsafe {
		rocsolver_dgetrf(
			rocblas_handle(),
			n as i32,
			n as i32,
			lu.ptr_raw() as *mut f64,
			n as i32,
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
	let status = unsafe {
		rocsolver_dgetrs(
			rocblas_handle(),
			OP_NONE,
			n as i32,
			nrhs as i32,
			lu.ptr_raw() as *mut f64,
			n as i32,
			ipiv.ptr_raw() as *const i32,
			b_copy.ptr_raw() as *mut f64,
			n as i32,
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
	let status = unsafe {
		rocsolver_dpotrs(
			rocblas_handle(),
			FILL_LOWER,
			n as i32,
			nrhs as i32,
			l.ptr_raw() as *mut f64,
			n as i32,
			b_copy.ptr_raw() as *mut f64,
			n as i32,
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
	let status = unsafe {
		rocsolver_dgeqrf(
			rocblas_handle(),
			m as i32,
			n as i32,
			factor.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *mut f64,
		)
	};
	check(status)?;

	// Step 3: Extract R (n×n col-major) from the upper triangle of factor (lda=m),
	// entirely on the GPU — r[j*n+i] = factor[j*m+i] for i<=j, else 0. Must run
	// before dorgqr below overwrites factor with Q. (No GPU→CPU→GPU round trip.)
	let r = GpuBuffer::alloc(n * n)?;
	crate::kernels::gpu_pack_upper_tri(&factor, &r, m, n);

	// Step 4: expand Householder reflectors → explicit Q (m×n col-major, lda=m).
	let status = unsafe {
		rocsolver_dorgqr(
			rocblas_handle(),
			m as i32,
			n as i32,
			k as i32,
			factor.ptr_raw() as *mut f64,
			m as i32,
			tau.ptr_raw() as *const f64,
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
	let e_work = GpuBuffer::alloc(n)?;
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;

	let status = unsafe {
		rocsolver_dsyevd(
			rocblas_handle(),
			EVECT_ORIGINAL,
			FILL_LOWER,
			n as i32,
			evecs.ptr_raw() as *mut f64,
			n as i32,
			evals.ptr_raw() as *mut f64,
			e_work.ptr_raw() as *mut f64,
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
	let e_work = GpuBuffer::alloc(k.max(1))?;
	let info = GpuBuffer::alloc_bytes(std::mem::size_of::<i32>())?;

	let status = unsafe {
		rocsolver_dgesvd(
			rocblas_handle(),
			SVECT_ALL,
			SVECT_ALL,
			m as i32,
			n as i32,
			a_cm.ptr_raw() as *mut f64,
			m as i32,
			s.ptr_raw() as *mut f64,
			u.ptr_raw() as *mut f64,
			m as i32,
			v.ptr_raw() as *mut f64,
			n as i32,
			e_work.ptr_raw() as *mut f64,
			OUTOFPLACE,
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
	einfo: usize,
	_work: Option<GpuBuffer>,
}
// SAFETY: rocFFT plan/einfo handles live for the process and are only read out under
// the cache mutex on the single GPU thread; GpuBuffer is already Send.
unsafe impl Send for CachedFftPlan {}

static FFT_CACHE: OnceLock<Mutex<HashMap<(u32, usize), CachedFftPlan>>> = OnceLock::new();

// Return a cached (plan, exec_info) for (transform_type, n), creating it on first use.
// Plans are never destroyed — that's the point of the cache.
fn fft_plan(transform_type: u32, n: usize) -> (*const c_void, *mut c_void) {
	rocfft_init();
	let mut cache = FFT_CACHE
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.expect("fft cache poisoned");
	let entry = cache.entry((transform_type, n)).or_insert_with(|| {
		let lengths = [n];
		let mut plan: *mut c_void = std::ptr::null_mut();
		let status = unsafe {
			rocfft_plan_create(
				&mut plan,
				ROCFFT_NOTINPLACE,
				transform_type,
				ROCFFT_PRECISION_DOUBLE,
				1,
				lengths.as_ptr(),
				1,
				std::ptr::null(),
			)
		};
		assert_eq!(status, 0, "rocfft_plan_create failed: {}", status);
		let mut work_size: usize = 0;
		unsafe {
			rocfft_plan_get_work_buffer_size(plan as *const c_void, &mut work_size);
		}
		let (work, einfo) = if work_size > 0 {
			let wb = GpuBuffer::alloc_bytes(work_size).expect("fft work buffer");
			let mut einfo: *mut c_void = std::ptr::null_mut();
			unsafe {
				rocfft_execution_info_create(&mut einfo);
			}
			unsafe {
				rocfft_execution_info_set_work_buffer(einfo, wb.ptr_raw(), work_size);
			}
			(Some(wb), einfo)
		} else {
			(None, std::ptr::null_mut())
		};
		CachedFftPlan {
			plan: plan as usize,
			einfo: einfo as usize,
			_work: work,
		}
	});
	(entry.plan as *const c_void, entry.einfo as *mut c_void)
}

pub fn gpu_fft_c2c_1d(input: &GpuBuffer, n: usize, forward: bool) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(2 * n)?; // 2 f64 per complex element
	let transform_type = if forward {
		ROCFFT_COMPLEX_FORWARD
	} else {
		ROCFFT_COMPLEX_INVERSE
	};
	let (plan, einfo) = fft_plan(transform_type, n);
	let mut in_ptr = input.ptr_raw();
	let mut out_ptr = out.ptr_raw();
	let status = unsafe { rocfft_execute(plan, &mut in_ptr, &mut out_ptr, einfo) };
	assert_eq!(status, 0, "rocfft_execute failed: {}", status);
	Ok(out)
}

// 1-D real-to-complex FFT.  Input: n f64 (real).
// Output: 2*(n/2+1) f64 (interleaved complex, hermitian-symmetric half-spectrum).
pub fn gpu_rfft_1d(input_real: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out_complex = n / 2 + 1;
	let out = GpuBuffer::alloc(2 * out_complex)?; // 2 f64 per complex
	let (plan, einfo) = fft_plan(ROCFFT_REAL_FORWARD, n);
	let mut in_ptr = input_real.ptr_raw();
	let mut out_ptr = out.ptr_raw();
	let status = unsafe { rocfft_execute(plan, &mut in_ptr, &mut out_ptr, einfo) };
	assert_eq!(status, 0, "rocfft_execute failed: {}", status);
	Ok(out)
}
