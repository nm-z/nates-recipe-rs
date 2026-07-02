// Parity test for hipBLAS-backed L2 (level-2) BLAS ops.
// GPU result (cuBLAS via shim on NVIDIA, rocBLAS on AMD) must match a plain-Rust
// CPU oracle within 1e-9 absolute. Matching on both backends == parity.

use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, linalg};

const TOL: f64 = 1e-9;

fn max_abs_diff(a: &[f64], b: &[f64]) -> (f64, usize) {
	assert_eq!(a.len(), b.len(), "length mismatch: {} vs {}", a.len(), b.len());
	let mut worst = 0.0;
	let mut idx = 0;
	for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
		let d = (x - y).abs();
		if d > worst {
			worst = d;
			idx = i;
		}
	}
	(worst, idx)
}

// CPU oracle: y = A @ x, A row-major m×n, x len n, y len m.
fn cpu_gemv_notrans(a: &[f64], x: &[f64], m: usize, n: usize) -> Vec<f64> {
	let mut y = vec![0.0; m];
	for i in 0..m {
		let mut acc = 0.0;
		for j in 0..n {
			acc += a[i * n + j] * x[j];
		}
		y[i] = acc;
	}
	y
}

// CPU oracle: y = A^T @ x, A row-major m×n, x len m, y len n.
fn cpu_gemv_trans(a: &[f64], x: &[f64], m: usize, n: usize) -> Vec<f64> {
	let mut y = vec![0.0; n];
	for j in 0..n {
		let mut acc = 0.0;
		for i in 0..m {
			acc += a[i * n + j] * x[i];
		}
		y[j] = acc;
	}
	y
}

// CPU oracle: A[i*n+j] = x[i]*y[j], A row-major m×n.
fn cpu_ger(x: &[f64], y: &[f64], m: usize, n: usize) -> Vec<f64> {
	let mut a = vec![0.0; m * n];
	for i in 0..m {
		for j in 0..n {
			a[i * n + j] = x[i] * y[j];
		}
	}
	a
}

// Deterministic pseudo-random fill in [-1, 1).
fn fill(len: usize, seed: u64) -> Vec<f64> {
	let mut s = seed.wrapping_add(0x9E3779B97F4A7C15);
	let mut v = Vec::with_capacity(len);
	for _ in 0..len {
		s ^= s << 13;
		s ^= s >> 7;
		s ^= s << 17;
		let u = (s >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
		v.push(u * 2.0 - 1.0);
	}
	v
}

// Sizes: square, non-square, and one not a multiple of 32 (warp-path stress).
const SIZES: &[(usize, usize)] = &[(32, 32), (64, 48), (37, 53), (1, 17), (29, 1)];

#[test]
fn dgemv_notrans_parity() {
	hip::set_device(0).unwrap();
	for &(m, n) in SIZES {
		let a = fill(m * n, 0x1111 ^ ((m as u64) << 16) ^ n as u64);
		let x = fill(n, 0x2222 ^ ((m as u64) << 8) ^ n as u64);

		let ga = GpuBuffer::upload(&a).unwrap();
		let gx = GpuBuffer::upload(&x).unwrap();
		let gy = linalg::gpu_dgemv(&ga, &gx, m, n, false).unwrap();
		let got = gy.download_vec().unwrap();

		let expect = cpu_gemv_notrans(&a, &x, m, n);
		let (d, idx) = max_abs_diff(&got, &expect);
		assert!(
			d < TOL,
			"dgemv notrans m={m} n={n}: max abs diff {d:e} at idx {idx} (got {}, expect {})",
			got[idx], expect[idx]
		);
	}
}

#[test]
fn dgemv_trans_parity() {
	hip::set_device(0).unwrap();
	for &(m, n) in SIZES {
		let a = fill(m * n, 0x3333 ^ ((m as u64) << 16) ^ n as u64);
		let x = fill(m, 0x4444 ^ ((m as u64) << 8) ^ n as u64);

		let ga = GpuBuffer::upload(&a).unwrap();
		let gx = GpuBuffer::upload(&x).unwrap();
		let gy = linalg::gpu_dgemv(&ga, &gx, m, n, true).unwrap();
		let got = gy.download_vec().unwrap();

		let expect = cpu_gemv_trans(&a, &x, m, n);
		let (d, idx) = max_abs_diff(&got, &expect);
		assert!(
			d < TOL,
			"dgemv trans m={m} n={n}: max abs diff {d:e} at idx {idx} (got {}, expect {})",
			got[idx], expect[idx]
		);
	}
}

#[test]
fn dger_parity() {
	hip::set_device(0).unwrap();
	for &(m, n) in SIZES {
		let x = fill(m, 0x5555 ^ ((m as u64) << 16) ^ n as u64);
		let y = fill(n, 0x6666 ^ ((m as u64) << 8) ^ n as u64);

		let gx = GpuBuffer::upload(&x).unwrap();
		let gy = GpuBuffer::upload(&y).unwrap();
		let ga = linalg::gpu_dger(&gx, &gy, m, n).unwrap();
		let got = ga.download_vec().unwrap();

		let expect = cpu_ger(&x, &y, m, n);
		let (d, idx) = max_abs_diff(&got, &expect);
		assert!(
			d < TOL,
			"dger m={m} n={n}: max abs diff {d:e} at idx {idx} (got {}, expect {})",
			got[idx], expect[idx]
		);
	}
}
