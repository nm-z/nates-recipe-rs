// Runtime parity for the hipSOLVER/hipFFT-backed ops (gpu_solve, gpu_cholesky_solve,
// gpu_eigh_sym, gpu_fft_c2c_1d). Layout-free checks: linear-solve residuals,
// eigenvalue-sum == trace, FFT round-trip. Same test runs on AMD (rocSOLVER/rocFFT)
// and NVIDIA (cuSOLVER/cuFFT) → cross-backend parity. f64, tol generous for solvers.
use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, kernels, linalg};

// deterministic SPD matrix A = MᵀM + n·I (row-major == col-major since symmetric)
fn spd(n: usize, seed: u64) -> Vec<f64> {
	let mut s = seed;
	let mut rnd = || {
		s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
		((s >> 33) as f64 / (1u64 << 31) as f64) - 1.0
	};
	let m: Vec<f64> = (0..n * n).map(|_| rnd()).collect();
	let mut a = vec![0.0; n * n];
	for i in 0..n {
		for j in 0..n {
			let mut acc = 0.0;
			for k in 0..n {
				acc += m[i * n + k] * m[j * n + k];
			}
			a[i * n + j] = acc + if i == j { n as f64 } else { 0.0 };
		}
	}
	a
}

fn matvec(a: &[f64], x: &[f64], n: usize) -> Vec<f64> {
	(0..n).map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum()).collect()
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
	a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max)
}

#[test]
fn cholesky_solve_residual() {
	hip::set_device(0).unwrap();
	for &n in &[8usize, 16, 31] {
		let a = spd(n, 11);
		let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.37).sin()).collect();
		let b = matvec(&a, &x, n);
		let da = GpuBuffer::upload(&a).unwrap();
		let db = GpuBuffer::upload(&b).unwrap();
		let work = GpuBuffer::alloc_bytes(kernels::gpu_cholesky_solve_workspace_bytes(n)).unwrap();
		let info = GpuBuffer::alloc(1).unwrap();
		let a_copy = GpuBuffer::alloc(n * n).unwrap();
		let out = GpuBuffer::alloc(n).unwrap();
		kernels::gpu_cholesky_solve(&da, &db, n, &work, &info, &a_copy, &out).unwrap();
		let xg = out.download_vec().unwrap();
		let d = max_abs_diff(&xg, &x);
		assert!(d < 1e-6, "cholesky_solve n={n}: max diff {d:e}");
	}
}

#[test]
fn lu_solve_residual() {
	hip::set_device(0).unwrap();
	for &n in &[8usize, 16, 31] {
		let a = spd(n, 23); // symmetric → layout-unambiguous
		let x: Vec<f64> = (0..n).map(|i| 1.0 + (i as f64 * 0.21).cos()).collect();
		let b = matvec(&a, &x, n);
		let da = GpuBuffer::upload(&a).unwrap();
		let db = GpuBuffer::upload(&b).unwrap();
		let work = GpuBuffer::alloc_bytes(kernels::gpu_solve_getrf_workspace_bytes(n)).unwrap();
		let work_s = GpuBuffer::alloc_bytes(kernels::gpu_solve_getrs_workspace_bytes(n, 1)).unwrap();
		let ipiv = GpuBuffer::alloc(n).unwrap();
		let info = GpuBuffer::alloc(1).unwrap();
		let a_copy = GpuBuffer::alloc(n * n).unwrap();
		let out = GpuBuffer::alloc(n).unwrap();
		kernels::gpu_solve(&da, &db, n, 1, &work, &work_s, &ipiv, &info, &a_copy, &out).unwrap();
		let xg = out.download_vec().unwrap();
		let d = max_abs_diff(&xg, &x);
		assert!(d < 1e-6, "gpu_solve n={n}: max diff {d:e}");
	}
}

#[test]
fn eigh_eigenvalue_sum_equals_trace() {
	hip::set_device(0).unwrap();
	for &n in &[8usize, 17] {
		let a = spd(n, 31);
		let trace: f64 = (0..n).map(|i| a[i * n + i]).sum();
		let da = GpuBuffer::upload(&a).unwrap();
		let work = GpuBuffer::alloc_bytes(linalg::gpu_eigh_sym_workspace_bytes(n)).unwrap();
		let info = GpuBuffer::alloc(1).unwrap();
		let evals = GpuBuffer::alloc(n).unwrap();
		let evecs = GpuBuffer::alloc(n * n).unwrap();
		linalg::gpu_eigh_sym(&da, n, &work, &info, &evals, &evecs).unwrap();
		let ev = evals.download_vec().unwrap();
		let sum: f64 = ev.iter().sum();
		// SPD → all eigenvalues strictly positive; sum == trace (invariant)
		assert!((sum - trace).abs() < 1e-6, "eigh n={n}: sum {sum} vs trace {trace}");
		assert!(ev.iter().all(|&l| l > 0.0), "eigh n={n}: non-positive eigenvalue");
	}
}

#[test]
fn fft_roundtrip() {
	hip::set_device(0).unwrap();
	for &n in &[8usize, 16, 64] {
		// interleaved re/im, n complex elements
		let inp: Vec<f64> = (0..2 * n).map(|i| (i as f64 * 0.13).sin()).collect();
		let din = GpuBuffer::upload(&inp).unwrap();
		let fwd = GpuBuffer::alloc(2 * n).unwrap();
		linalg::gpu_fft_c2c_1d(&din, n, 1, &fwd).unwrap();
		let inv_buf = GpuBuffer::alloc(2 * n).unwrap();
		linalg::gpu_fft_c2c_1d(&fwd, n, 0, &inv_buf).unwrap();
		let inv = inv_buf.download_vec().unwrap();
		// hipFFT is unnormalized: inverse(forward(x)) = n·x
		let recovered: Vec<f64> = inv.iter().map(|v| v / n as f64).collect();
		let d = max_abs_diff(&recovered, &inp);
		assert!(d < 1e-9, "fft roundtrip n={n}: max diff {d:e}");
	}
}

// gpu_cholesky_inv must return the true inverse: A·A⁻¹ = I. (A⁻¹ is symmetric for
// SPD A, so the row/col-major layout is unambiguous.) Guards the potrf/dtrsm fill
// mode agreeing inside that function.
#[test]
fn cholesky_inv_times_a_is_identity() {
	hip::set_device(0).unwrap();
	for &n in &[8usize, 16, 31] {
		let a = spd(n, 41);
		let da = GpuBuffer::upload(&a).unwrap();
		let work = GpuBuffer::alloc_bytes(kernels::gpu_cholesky_inv_workspace_bytes(n)).unwrap();
		let info = GpuBuffer::alloc(1).unwrap();
		let a_copy = GpuBuffer::alloc(n * n).unwrap();
		let out = GpuBuffer::alloc(n * n).unwrap();
		kernels::gpu_eye(n, &out).unwrap();
		kernels::gpu_cholesky_inv(&da, n, &work, &info, &a_copy, &out).unwrap();
		let inv = out.download_vec().unwrap();
		let mut maxoff = 0.0f64;
		for i in 0..n {
			for j in 0..n {
				let mut acc = 0.0;
				for k in 0..n {
					acc += a[i * n + k] * inv[k * n + j];
				}
				let want = if i == j { 1.0 } else { 0.0 };
				maxoff = maxoff.max((acc - want).abs());
			}
		}
		assert!(maxoff < 1e-6, "cholesky_inv n={n}: max |A·A⁻¹ − I| = {maxoff:e}");
	}
}

// gpu_cholesky (factor) feeding gpu_tri_solve must reconstruct the linear solve:
// A=L·Lᵀ stored col-major as U=Lᵀ, so A·x=b is forward Uᵀz=b (trans=true) then
// backward Ux=z (trans=false). Guards gpu_tri_solve's fill mode matching gpu_cholesky.
#[test]
fn cholesky_factor_then_tri_solve() {
	hip::set_device(0).unwrap();
	for &n in &[8usize, 16, 31] {
		let a = spd(n, 53);
		let x_true: Vec<f64> = (0..n).map(|i| (i as f64 * 0.41).cos() + 0.5).collect();
		let b = matvec(&a, &x_true, n);
		let da = GpuBuffer::upload(&a).unwrap();
		let db = GpuBuffer::upload(&b).unwrap();
		let work = GpuBuffer::alloc_bytes(kernels::gpu_cholesky_workspace_bytes(n)).unwrap();
		let info = GpuBuffer::alloc(1).unwrap();
		let l = GpuBuffer::alloc(n * n).unwrap();
		kernels::gpu_cholesky(&da, n, &work, &info, &l).unwrap();
		let z = GpuBuffer::alloc(n).unwrap();
		kernels::gpu_tri_solve(&l, &db, n, 1, 1, &z).unwrap();
		let x_buf = GpuBuffer::alloc(n).unwrap();
		kernels::gpu_tri_solve(&l, &z, n, 1, 0, &x_buf).unwrap();
		let x = x_buf.download_vec().unwrap();
		let d = max_abs_diff(&x, &x_true);
		assert!(d < 1e-6, "cholesky+tri_solve n={n}: max diff {d:e}");
	}
}
