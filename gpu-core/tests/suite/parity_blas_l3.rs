// Parity test for hipBLAS-backed L3 (level-3) BLAS ops.
// GPU result (cuBLAS via shim on NVIDIA, rocBLAS on AMD) must match a plain-Rust
// CPU oracle within 1e-9 absolute. Matching the SAME oracle on both backends == parity.
//
// Ops covered (semantics read from gpu-core/src/{kernels.rs,linalg.rs}):
//   kernels::gpu_gemm(a,b,m,n,k)    -> C(m×n) = A(m×k) · B(k×n)        (all row-major)
//   kernels::gpu_gemm_at(a,b,m,n,k) -> C(m×n) = A(k×m)^T · B(k×n)
//   kernels::gpu_gemm_bt(a,b,m,n,k) -> C(m×n) = A(m×k) · B(n×k)^T
//   linalg::gpu_dsyrk(a,n,k)        -> C(n×n) = A(k×n)^T · A(k×n)  (k==n; row-major
//                                      lower triangle i>=j is the populated triangle)

use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, kernels, linalg};

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

// Deterministic, reproducible "random-ish" fill — spread of magnitudes and signs,
// no dependence on any RNG crate so the oracle is byte-stable across runs.
fn fill(len: usize, seed: u64) -> Vec<f64> {
	let mut s = seed.wrapping_add(0x9E3779B97F4A7C15);
	let mut v = Vec::with_capacity(len);
	for _ in 0..len {
		// splitmix64
		s = s.wrapping_add(0x9E3779B97F4A7C15);
		let mut z = s;
		z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
		z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
		z ^= z >> 31;
		// map to roughly [-2.0, 2.0)
		let u = (z >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
		v.push(u * 4.0 - 2.0);
	}
	v
}

// ── CPU oracles ──────────────────────────────────────────────────────────────

// C(m×n) = A(m×k) · B(k×n), all row-major.
fn cpu_gemm(a: &[f64], b: &[f64], m: usize, n: usize, k: usize) -> Vec<f64> {
	let mut c = vec![0.0; m * n];
	for i in 0..m {
		for j in 0..n {
			let mut acc = 0.0;
			for l in 0..k {
				acc += a[i * k + l] * b[l * n + j];
			}
			c[i * n + j] = acc;
		}
	}
	c
}

// C(m×n) = A(k×m)^T · B(k×n).  A row-major (k×m): A[l*m+i].  B row-major (k×n): B[l*n+j].
fn cpu_gemm_at(a: &[f64], b: &[f64], m: usize, n: usize, k: usize) -> Vec<f64> {
	let mut c = vec![0.0; m * n];
	for i in 0..m {
		for j in 0..n {
			let mut acc = 0.0;
			for l in 0..k {
				acc += a[l * m + i] * b[l * n + j];
			}
			c[i * n + j] = acc;
		}
	}
	c
}

// C(m×n) = A(m×k) · B(n×k)^T.  A row-major (m×k): A[i*k+l].  B row-major (n×k): B[j*k+l].
fn cpu_gemm_bt(a: &[f64], b: &[f64], m: usize, n: usize, k: usize) -> Vec<f64> {
	let mut c = vec![0.0; m * n];
	for i in 0..m {
		for j in 0..n {
			let mut acc = 0.0;
			for l in 0..k {
				acc += a[i * k + l] * b[j * k + l];
			}
			c[i * n + j] = acc;
		}
	}
	c
}

// Full symmetric Gram C(n×n) = A(k×n)^T · A(k×n).  A row-major (k×n): A[l*n+i].
fn cpu_gram(a: &[f64], n: usize, k: usize) -> Vec<f64> {
	let mut c = vec![0.0; n * n];
	for i in 0..n {
		for j in 0..n {
			let mut acc = 0.0;
			for l in 0..k {
				acc += a[l * n + i] * a[l * n + j];
			}
			c[i * n + j] = acc;
		}
	}
	c
}

fn assert_parity(label: &str, gpu: &[f64], cpu: &[f64]) {
	let (worst, idx) = max_abs_diff(gpu, cpu);
	assert!(
		worst < TOL,
		"{label}: max abs diff {worst:e} at index {idx} (gpu={}, cpu={}) exceeds tol {TOL:e}",
		gpu[idx],
		cpu[idx],
	);
	eprintln!("{label}: OK (max abs diff {worst:e} over {} elems)", gpu.len());
}

// Sizes: square, two non-square, and several NOT multiples of 32 (7,11,13,33,17,40)
// to stress the warp/tiling boundaries.
const GEMM_SIZES: &[(usize, usize, usize)] = &[
	(4, 4, 4),    // square baseline
	(3, 5, 4),    // non-square, small
	(7, 11, 13),  // none a multiple of 32
	(33, 17, 40), // straddles the 32 boundary, non-square
	(1, 9, 6),    // degenerate single-row
];

// ── gpu_gemm : C = A · B ──────────────────────────────────────────────────────

#[test]
fn gemm_matches_cpu_oracle() {
	hip::set_device(0).unwrap();
	for &(m, n, k) in GEMM_SIZES {
		let a = fill(m * k, 0x1111 ^ ((m * 100 + k) as u64));
		let b = fill(k * n, 0x2222 ^ ((k * 100 + n) as u64));
		let ag = GpuBuffer::upload(&a).unwrap();
		let bg = GpuBuffer::upload(&b).unwrap();
		let cg = kernels::gpu_gemm(&ag, &bg, m, n, k).unwrap();
		let gpu = cg.download_vec().unwrap();
		let cpu = cpu_gemm(&a, &b, m, n, k);
		assert_parity(&format!("gpu_gemm m={m} n={n} k={k}"), &gpu, &cpu);
	}
}

// ── gpu_gemm_at : C = A^T · B ──────────────────────────────────────────────────

#[test]
fn gemm_at_matches_cpu_oracle() {
	hip::set_device(0).unwrap();
	for &(m, n, k) in GEMM_SIZES {
		// A is (k×m) row-major, B is (k×n) row-major.
		let a = fill(k * m, 0x3333 ^ ((k * 100 + m) as u64));
		let b = fill(k * n, 0x4444 ^ ((k * 100 + n) as u64));
		let ag = GpuBuffer::upload(&a).unwrap();
		let bg = GpuBuffer::upload(&b).unwrap();
		let cg = kernels::gpu_gemm_at(&ag, &bg, m, n, k).unwrap();
		let gpu = cg.download_vec().unwrap();
		let cpu = cpu_gemm_at(&a, &b, m, n, k);
		assert_parity(&format!("gpu_gemm_at m={m} n={n} k={k}"), &gpu, &cpu);
	}
}

// ── gpu_gemm_bt : C = A · B^T ──────────────────────────────────────────────────

#[test]
fn gemm_bt_matches_cpu_oracle() {
	hip::set_device(0).unwrap();
	for &(m, n, k) in GEMM_SIZES {
		// A is (m×k) row-major, B is (n×k) row-major.
		let a = fill(m * k, 0x5555 ^ ((m * 100 + k) as u64));
		let b = fill(n * k, 0x6666 ^ ((n * 100 + k) as u64));
		let ag = GpuBuffer::upload(&a).unwrap();
		let bg = GpuBuffer::upload(&b).unwrap();
		let cg = kernels::gpu_gemm_bt(&ag, &bg, m, n, k).unwrap();
		let gpu = cg.download_vec().unwrap();
		let cpu = cpu_gemm_bt(&a, &b, m, n, k);
		assert_parity(&format!("gpu_gemm_bt m={m} n={n} k={k}"), &gpu, &cpu);
	}
}

// ── gpu_dsyrk : C = A^T · A  (symmetric rank-k) ────────────────────────────────
//
// gpu_dsyrk(a, n, k) passes lda=k to hipblasDsyrk, so the op is well-defined only
// for square A (k == n); production callers and the existing committed test use
// k == n. hipBLAS writes one triangle: the populated (valid) triangle in the
// downloaded row-major buffer is the LOWER triangle (i >= j) — confirmed against
// the existing t_linalg_reduce::test_dsyrk. We verify exactly that triangle; the
// Gram matrix is symmetric so its values equal the full oracle there.

#[test]
fn dsyrk_lower_triangle_matches_cpu_oracle() {
	hip::set_device(0).unwrap();
	// n == k; include non-multiples of 32 (7, 33, 17) plus a small baseline.
	for &n in &[4usize, 7, 17, 33] {
		let k = n;
		let a = fill(k * n, 0x7777 ^ (n as u64));
		let ag = GpuBuffer::upload(&a).unwrap();
		let cg = linalg::gpu_dsyrk(&ag, n, k).unwrap();
		let gpu = cg.download_vec().unwrap();
		let gram = cpu_gram(&a, n, k);

		// Compare only the populated lower triangle (i >= j) of the row-major output.
		let mut worst = 0.0;
		let mut at = (0usize, 0usize);
		for i in 0..n {
			for j in 0..=i {
				let d = (gpu[i * n + j] - gram[i * n + j]).abs();
				if d > worst {
					worst = d;
					at = (i, j);
				}
			}
		}
		assert!(
			worst < TOL,
			"gpu_dsyrk n={n}: lower-triangle max abs diff {worst:e} at ({},{}) (gpu={}, cpu={}) exceeds tol {TOL:e}",
			at.0,
			at.1,
			gpu[at.0 * n + at.1],
			gram[at.0 * n + at.1],
		);
		eprintln!("gpu_dsyrk n={n}: OK (lower-triangle max abs diff {worst:e})");
	}
}
