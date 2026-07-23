use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, kernels, linalg};
use std::ptr;

const TOL: f64 = 1e-9;

fn max_abs_diff(a: &[f64], b: &[f64]) -> (f64, usize) {
	assert_eq!(
		a.len(),
		b.len(),
		"length mismatch: {} vs {}",
		a.len(),
		b.len()
	);
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

fn fill(len: usize, seed: u64) -> Vec<f64> {
	let mut s = seed.wrapping_add(0x9E3779B97F4A7C15);
	let mut v = Vec::with_capacity(len);
	for _ in 0..len {
		s = s.wrapping_add(0x9E3779B97F4A7C15);
		let mut z = s;
		z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
		z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
		z ^= z >> 31;
		let u = (z >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
		v.push(u * 4.0 - 2.0);
	}
	v
}

// ── CPU oracles ──────────────────────────────────────────────────────────────

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
	eprintln!(
		"{label}: OK (max abs diff {worst:e} over {} elems)",
		gpu.len()
	);
}

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
		let ag = {
			let __up = &a;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let bg = {
			let __up = &b;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let cg = GpuBuffer::alloc(m * n).unwrap();
		kernels::gpu_gemm(&ag, &bg, m, n, k, &cg).unwrap();
		let gpu = {
			let mut __dv = vec![0.0f64; cg.n_floats()];
			unsafe { cg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		};
		let cpu = cpu_gemm(&a, &b, m, n, k);
		assert_parity(&format!("gpu_gemm m={m} n={n} k={k}"), &gpu, &cpu);
	}
}

// ── gpu_gemm_at : C = A^T · B ──────────────────────────────────────────────────

#[test]
fn gemm_at_matches_cpu_oracle() {
	hip::set_device(0).unwrap();
	for &(m, n, k) in GEMM_SIZES {
		let a = fill(k * m, 0x3333 ^ ((k * 100 + m) as u64));
		let b = fill(k * n, 0x4444 ^ ((k * 100 + n) as u64));
		let ag = {
			let __up = &a;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let bg = {
			let __up = &b;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let cg = GpuBuffer::alloc(m * n).unwrap();
		kernels::gpu_gemm_at(&ag, &bg, m, n, k, &cg).unwrap();
		let gpu = {
			let mut __dv = vec![0.0f64; cg.n_floats()];
			unsafe { cg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		};
		let cpu = cpu_gemm_at(&a, &b, m, n, k);
		assert_parity(&format!("gpu_gemm_at m={m} n={n} k={k}"), &gpu, &cpu);
	}
}

// ── gpu_gemm_bt : C = A · B^T ──────────────────────────────────────────────────

#[test]
fn gemm_bt_matches_cpu_oracle() {
	hip::set_device(0).unwrap();
	for &(m, n, k) in GEMM_SIZES {
		let a = fill(m * k, 0x5555 ^ ((m * 100 + k) as u64));
		let b = fill(n * k, 0x6666 ^ ((n * 100 + k) as u64));
		let ag = {
			let __up = &a;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let bg = {
			let __up = &b;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let cg = GpuBuffer::alloc(m * n).unwrap();
		kernels::gpu_gemm_bt_into(&ag, &bg, m, n, k, &cg).unwrap();
		let gpu = {
			let mut __dv = vec![0.0f64; cg.n_floats()];
			unsafe { cg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		};
		let cpu = cpu_gemm_bt(&a, &b, m, n, k);
		assert_parity(&format!("gpu_gemm_bt m={m} n={n} k={k}"), &gpu, &cpu);
	}
}

// ── gpu_dsyrk : C = A^T · A  (symmetric rank-k) ────────────────────────────────

#[test]
fn dsyrk_lower_triangle_matches_cpu_oracle() {
	hip::set_device(0).unwrap();
	for &n in &[4usize, 7, 17, 33] {
		let k = n;
		let a = fill(k * n, 0x7777 ^ (n as u64));
		let ag = {
			let __up = &a;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let cg = GpuBuffer::alloc(n * n).unwrap();
		linalg::gpu_dsyrk(&ag, n, k, &cg).unwrap();
		let gpu = {
			let mut __dv = vec![0.0f64; cg.n_floats()];
			unsafe { cg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		};
		let gram = cpu_gram(&a, n, k);

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
