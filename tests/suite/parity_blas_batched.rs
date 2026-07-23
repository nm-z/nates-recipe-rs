use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, kernels, linalg};
use std::ptr;

// ── CPU oracles (plain Rust, row-major contiguous) ────────────────────────

fn cpu_bmm(a: &[f64], b: &[f64], batch: usize, m: usize, n: usize, k: usize, ta: bool, tb: bool) -> Vec<f64> {
	let mut c = vec![0.0f64; batch * m * n];
	for bi in 0..batch {
		let ao = bi * m * k;
		let bo = bi * k * n;
		let co = bi * m * n;
		for i in 0..m {
			for j in 0..n {
				let mut s = 0.0;
				for p in 0..k {
					let av = if ta {
						a[ao + p * m + i]
					} else {
						a[ao + i * k + p]
					};
					let bv = if tb {
						b[bo + j * k + p]
					} else {
						b[bo + p * n + j]
					};
					s += av * bv;
				}
				c[co + i * n + j] = s;
			}
		}
	}
	c
}

fn cpu_gemm(a: &[f64], b: &[f64], m: usize, n: usize, k: usize) -> Vec<f64> {
	cpu_bmm(a, b, 1, m, n, k, false, false)
}

fn max_abs_diff(want: &[f64], got: &[f64]) -> f64 {
	want.iter()
		.zip(got)
		.map(|(x, y)| (x - y).abs())
		.fold(0.0, f64::max)
}

fn run_bmm_case(batch: usize, m: usize, n: usize, k: usize, ta: bool, tb: bool) {
	hip::set_device(0).unwrap();

	let a_rows = if ta { k } else { m };
	let a_cols = if ta { m } else { k };
	let b_rows = if tb { n } else { k };
	let b_cols = if tb { k } else { n };

	let a: Vec<f64> = (0..batch * a_rows * a_cols)
		.map(|i| (i as f64 * 0.37).sin() + 0.1 * i as f64)
		.collect();
	let b: Vec<f64> = (0..batch * b_rows * b_cols)
		.map(|i| (i as f64 * 0.53).cos() - 0.05 * i as f64)
		.collect();

	let want = cpu_bmm(&a, &b, batch, m, n, k, ta, tb);

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
	let cg = GpuBuffer::alloc(batch * m * n).unwrap();

	linalg::gpu_bmm_into(
		&ag,
		&bg,
		batch,
		m,
		n,
		k,
		a_cols,          // lda = row length of stored A
		b_cols,          // ldb = row length of stored B
		n,               // ldc
		a_rows * a_cols, // stride_a
		b_rows * b_cols, // stride_b
		m * n,           // stride_c
		0,
		0,
		0,
		ta as usize,
		tb as usize,
		&cg,
	)
	.unwrap();

	let got = {
		let mut __dv = vec![0.0f64; cg.n_floats()];
		unsafe { cg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		__dv
	};
	let d = max_abs_diff(&want, &got);
	assert!(
		d < 1e-9,
		"bmm parity failed batch={batch} m={m} n={n} k={k} ta={ta} tb={tb} maxdiff={d:.3e}"
	);
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[test]
fn bmm_parity_square_aligned() {
	run_bmm_case(4, 32, 32, 32, false, false);
}

#[test]
fn bmm_parity_nonsquare_ragged() {
	run_bmm_case(3, 17, 23, 11, false, false);
	run_bmm_case(2, 13, 5, 19, false, false);
}

#[test]
fn bmm_parity_transpose_modes() {
	run_bmm_case(3, 7, 13, 5, false, false);
	run_bmm_case(3, 7, 13, 5, false, true);
	run_bmm_case(3, 7, 13, 5, true, false);
	run_bmm_case(3, 7, 13, 5, true, true);
}

#[test]
fn bmm_parity_single_batch() {
	run_bmm_case(1, 9, 6, 14, false, false);
}

#[test]
fn bmm_parity_two_batch_explicit() {
	hip::set_device(0).unwrap();
	let (batch, m, n, k) = (2usize, 3usize, 2usize, 4usize);
	let a: Vec<f64> = (0..batch * m * k).map(|i| i as f64 + 1.0).collect();
	let b: Vec<f64> = (0..batch * k * n).map(|i| (i as f64 + 1.0) * 0.5).collect();
	let want = cpu_bmm(&a, &b, batch, m, n, k, false, false);

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
	let cg = GpuBuffer::alloc(batch * m * n).unwrap();
	linalg::gpu_bmm_into(
		&ag,
		&bg,
		batch,
		m,
		n,
		k,
		k,
		n,
		n,
		m * k,
		k * n,
		m * n,
		0,
		0,
		0,
		0,
		0,
		&cg,
	)
	.unwrap();
	let got = {
		let mut __dv = vec![0.0f64; cg.n_floats()];
		unsafe { cg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		__dv
	};
	let d = max_abs_diff(&want, &got);
	assert!(d < 1e-9, "explicit 2-batch bmm maxdiff={d:.3e}");
}

#[test]
fn gemm_pipeline_compose_parity() {
	hip::set_device(0).unwrap();
	let (rows, f0, f1, f2) = (10usize, 6usize, 13usize, 4usize); // ragged, non-aligned
	let x: Vec<f64> = (0..rows * f0).map(|i| (i as f64 * 0.11).sin()).collect();
	let w1: Vec<f64> = (0..f0 * f1).map(|i| (i as f64 * 0.07).cos()).collect();
	let w2: Vec<f64> = (0..f1 * f2).map(|i| (i as f64 * 0.13).sin()).collect();

	let h_cpu = cpu_gemm(&x, &w1, rows, f1, f0);
	let y_cpu = cpu_gemm(&h_cpu, &w2, rows, f2, f1);

	let xg = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let w1g = {
		let __up = &w1;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let w2g = {
		let __up = &w2;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let hg = GpuBuffer::alloc(rows * f1).unwrap();
	kernels::gpu_gemm(&xg, &w1g, rows, f1, f0, &hg).unwrap();
	let yg = GpuBuffer::alloc(rows * f2).unwrap();
	kernels::gpu_gemm(&hg, &w2g, rows, f2, f1, &yg).unwrap();

	let h_gpu = {
		let mut __dv = vec![0.0f64; hg.n_floats()];
		unsafe { hg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		__dv
	};
	let dh = max_abs_diff(&h_cpu, &h_gpu);
	assert!(dh < 1e-9, "pipeline H maxdiff={dh:.3e}");

	let y_gpu = {
		let mut __dv = vec![0.0f64; yg.n_floats()];
		unsafe { yg.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		__dv
	};
	let dy = max_abs_diff(&y_cpu, &y_gpu);
	assert!(dy < 1e-9, "pipeline Y maxdiff={dy:.3e}");
}
