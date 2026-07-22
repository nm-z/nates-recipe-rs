
use gpu_core::memory::GpuBuffer;
use gpu_core::reductions::{
	gpu_dot, gpu_dot_workspace_bytes, gpu_l2_norm, gpu_l2_norm_workspace_bytes,
};
use gpu_core::{hip, linalg};
use std::ptr;

const TOL: f64 = 1e-9;

const SIZES: &[usize] = &[1, 7, 31, 32, 33, 64, 100, 127, 256, 1000];

fn make_seq(n: usize, seed: u64) -> Vec<f64> {
	let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
	let mut v = Vec::with_capacity(n);
	for _ in 0..n {
		state = state
			.wrapping_mul(6364136223846793005)
			.wrapping_add(1442695040888963407);
		let u = ((state >> 11) as f64) / ((1u64 << 53) as f64); // [0,1)
		v.push(2.0 * u - 1.0);
	}
	v
}

fn cpu_ddot(a: &[f64], b: &[f64]) -> f64 {
	a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn cpu_dnrm2(x: &[f64]) -> f64 {
	x.iter().map(|v| v * v).sum::<f64>().sqrt()
}

fn cpu_dasum(x: &[f64]) -> f64 {
	x.iter().map(|v| v.abs()).sum()
}

fn cpu_idamax(x: &[f64]) -> usize {
	let mut best = 0usize;
	let mut best_abs = x[0].abs();
	for (i, v) in x.iter().enumerate() {
		if v.abs() > best_abs {
			best_abs = v.abs();
			best = i;
		}
	}
	best
}

#[test]
fn ddot_parity() {
	hip::set_device(0).unwrap();
	let mut max_diff = 0.0f64;
	for &n in SIZES {
		let a = make_seq(n, 1);
		let b = make_seq(n, 2);
		let ga = {
			let __up = &a;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gb = {
			let __up = &b;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let ws = GpuBuffer::alloc_bytes(gpu_dot_workspace_bytes(n)).unwrap();
		let prod = GpuBuffer::alloc(n).unwrap();
		let out = GpuBuffer::alloc(1).unwrap();
		gpu_dot(&ga, &gb, &ws, &prod, n, &out).unwrap();
		let got = {
			let mut __dv = vec![0.0f64; out.n_floats()];
			unsafe { out.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		}[0];
		let want = cpu_ddot(&a, &b);
		let diff = (got - want).abs();
		if diff > max_diff {
			max_diff = diff;
		}
		assert!(
			diff < TOL,
			"ddot mismatch n={n}: gpu={got} cpu={want} diff={diff}"
		);
	}
	eprintln!("ddot_parity max abs diff = {max_diff:e}");
}

#[test]
fn dnrm2_parity() {
	hip::set_device(0).unwrap();
	let mut max_diff = 0.0f64;
	for &n in SIZES {
		let x = make_seq(n, 3);
		let gx = {
			let __up = &x;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let ws = GpuBuffer::alloc_bytes(gpu_l2_norm_workspace_bytes(n)).unwrap();
		let sq = GpuBuffer::alloc(n).unwrap();
		let out = GpuBuffer::alloc(1).unwrap();
		gpu_l2_norm(&gx, &ws, &sq, n, &out).unwrap();
		let got = {
			let mut __dv = vec![0.0f64; out.n_floats()];
			unsafe { out.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		}[0];
		let want = cpu_dnrm2(&x);
		let diff = (got - want).abs();
		if diff > max_diff {
			max_diff = diff;
		}
		assert!(
			diff < TOL,
			"dnrm2 mismatch n={n}: gpu={got} cpu={want} diff={diff}"
		);
	}
	eprintln!("dnrm2_parity max abs diff = {max_diff:e}");
}

#[test]
fn dasum_parity() {
	hip::set_device(0).unwrap();
	let mut max_diff = 0.0f64;
	for &n in SIZES {
		let x = make_seq(n, 4);
		let gx = {
			let __up = &x;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let out = GpuBuffer::alloc(1).unwrap();
		linalg::gpu_dasum(&gx, n, &out).unwrap();
		let got = {
			let mut __dv = vec![0.0f64; out.n_floats()];
			unsafe { out.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		}[0];
		let want = cpu_dasum(&x);
		let diff = (got - want).abs();
		if diff > max_diff {
			max_diff = diff;
		}
		assert!(
			diff < TOL,
			"dasum mismatch n={n}: gpu={got} cpu={want} diff={diff}"
		);
	}
	eprintln!("dasum_parity max abs diff = {max_diff:e}");
}

#[test]
fn idamax_parity() {
	hip::set_device(0).unwrap();
	for &n in SIZES {
		let x = make_seq(n, 5);
		let gx = {
			let __up = &x;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let out = GpuBuffer::alloc(1).unwrap();
		linalg::gpu_idamax(&gx, n, &out).unwrap();
		let got = {
			let mut __dv = vec![0.0f64; out.n_floats()];
			unsafe { out.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			__dv
		}[0]
		.round() as usize;
		let want = cpu_idamax(&x);
		assert_eq!(
			got, want,
			"idamax mismatch n={n}: gpu={got} cpu={want} (0-based index)"
		);
	}

	let mut x = make_seq(50, 6);
	x[37] = -99.0;
	let gx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(1).unwrap();
	linalg::gpu_idamax(&gx, x.len(), &out).unwrap();
	let got = {
		let mut __dv = vec![0.0f64; out.n_floats()];
		unsafe { out.download_async(&mut __dv, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		__dv
	}[0]
	.round() as usize;
	assert_eq!(got, 37, "idamax negative-spike: gpu={got} want=37");
}
