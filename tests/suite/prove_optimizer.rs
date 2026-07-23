use crate::common;

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;
use std::fs;
use std::ptr;

// ── FFI for the NEW optimizerx_ launchers ───────────────────────────────────
unsafe extern "C" {
	fn launch_optimizerx_nesterov(
		w: *mut c_void,
		buf: *mut c_void,
		g: *const c_void,
		lr: f64,
		momentum: f64,
		n: i32,
		s: *mut c_void,
	);
	fn launch_optimizerx_adadelta(
		w: *mut c_void,
		eg: *mut c_void,
		edx: *mut c_void,
		g: *const c_void,
		lr: f64,
		rho: f64,
		eps: f64,
		n: i32,
		s: *mut c_void,
	);
	fn launch_optimizerx_radam(
		w: *mut c_void,
		m: *mut c_void,
		v: *mut c_void,
		g: *const c_void,
		lr: f64,
		b1: f64,
		b2: f64,
		eps: f64,
		t: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_optimizerx_lars_phase1(
		w: *const c_void,
		g: *const c_void,
		n: i32,
		w_norm_sq: *mut c_void,
		g_norm_sq: *mut c_void,
		s: *mut c_void,
	);
	fn launch_optimizerx_lars_phase2(
		w: *mut c_void,
		buf: *mut c_void,
		g: *const c_void,
		lr: f64,
		momentum: f64,
		wd: f64,
		lambda: f64,
		w_norm_sq: f64,
		g_norm_sq: f64,
		eps: f64,
		n: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-6;

fn close(a: &[f64], b: &[f64]) -> bool {
	a.len() == b.len()
		&& a.iter()
			.zip(b)
			.all(|(x, y)| (x - y).abs() <= TOL * (1.0 + y.abs()))
}

fn wv(n: usize) -> Vec<f64> {
	(0..n).map(|i| 0.30 - 0.11 * i as f64 + 0.017 * (i * i) as f64)
		.collect()
}
fn gv(n: usize) -> Vec<f64> {
	(0..n).map(|i| -0.20 + 0.13 * i as f64 - 0.009 * (i * i) as f64)
		.collect()
}
fn sv(n: usize, base: f64, step: f64) -> Vec<f64> {
	(0..n).map(|i| base + step * i as f64).collect()
}

const N: usize = 37;

// ── EXISTING-op proofs (call public Rust fns) ───────────────────────────────

fn prove_sgd() -> bool {
	let (lr, n) = (0.05, N);
	let w = wv(n);
	let g = gv(n);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bneg_lr = {
		let __up = &[-lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::kernels::gpu_sgd_update(&bg, &bneg_lr, n, &bw).unwrap();
	let mut got = vec![0.0; n];
	unsafe { bw.download_async(&mut got, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let want: Vec<f64> = (0..n).map(|i| w[i] - lr * g[i]).collect();
	close(&got, &want)
}

fn prove_momentum() -> bool {
	let (lr, mu, n) = (0.05, 0.9, N);
	let w = wv(n);
	let g = gv(n);
	let v0 = sv(n, 0.4, -0.02);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bv = {
		let __up = &v0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let blr = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bmu = {
		let __up = &[mu];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::optimizers::gpu_momentum_update(&bg, &blr, &bmu, n, &bw, &bv).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut gvb = vec![0.0; n];
	unsafe { bv.download_async(&mut gvb, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let want_v: Vec<f64> = (0..n).map(|i| mu * v0[i] - lr * g[i]).collect();
	let want_w: Vec<f64> = (0..n).map(|i| w[i] + want_v[i]).collect();
	close(&gw, &want_w) && close(&gvb, &want_v)
}

fn prove_rmsprop() -> bool {
	let (lr, decay, eps, n) = (0.05, 0.9, 1e-8, N);
	let w = wv(n);
	let g = gv(n);
	let c0 = sv(n, 0.5, 0.01);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bc = {
		let __up = &c0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let blr = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bdecay = {
		let __up = &[decay];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let beps = {
		let __up = &[eps];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::optimizers::gpu_rmsprop_update(&bg, &blr, &bdecay, &beps, n, &bw, &bc).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut gc = vec![0.0; n];
	unsafe { bc.download_async(&mut gc, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let want_c: Vec<f64> = (0..n)
		.map(|i| decay * c0[i] + (1.0 - decay) * g[i] * g[i])
		.collect();
	let want_w: Vec<f64> = (0..n)
		.map(|i| w[i] - lr * g[i] / (want_c[i].sqrt() + eps))
		.collect();
	close(&gw, &want_w) && close(&gc, &want_c)
}

fn prove_adagrad() -> bool {
	let (lr, eps, n) = (0.05, 1e-10, N);
	let w = wv(n);
	let g = gv(n);
	let a0 = sv(n, 0.3, 0.02);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ba = {
		let __up = &a0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let blr = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let beps = {
		let __up = &[eps];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::optimizers::gpu_adagrad_update(&bg, &blr, &beps, n, &bw, &ba).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut ga = vec![0.0; n];
	unsafe { ba.download_async(&mut ga, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let want_a: Vec<f64> = (0..n).map(|i| a0[i] + g[i] * g[i]).collect();
	let want_w: Vec<f64> = (0..n)
		.map(|i| w[i] - lr * g[i] / (want_a[i].sqrt() + eps))
		.collect();
	close(&gw, &want_w) && close(&ga, &want_a)
}

fn prove_lion() -> bool {
	let (lr, b1, b2, wd, n) = (0.01, 0.9, 0.99, 0.01, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, -0.15, 0.02);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bm = {
		let __up = &m0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let blr = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb1 = {
		let __up = &[b1];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb2 = {
		let __up = &[b2];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bwd = {
		let __up = &[wd];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::optimizers::gpu_lion_update(&bg, &blr, &bb1, &bb2, &bwd, n, &bw, &bm).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut gm = vec![0.0; n];
	unsafe { bm.download_async(&mut gm, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let sign = |x: f64| {
		if x > 0.0 {
			1.0
		} else if x < 0.0 {
			-1.0
		} else {
			0.0
		}
	};
	let want_w: Vec<f64> = (0..n)
		.map(|i| {
			let u = sign(b1 * m0[i] + (1.0 - b1) * g[i]);
			w[i] - lr * (u + wd * w[i])
		})
		.collect();
	let want_m: Vec<f64> = (0..n).map(|i| b2 * m0[i] + (1.0 - b2) * g[i]).collect();
	close(&gw, &want_w) && close(&gm, &want_m)
}

fn prove_adam() -> bool {
	let (lr, b1, b2, eps, t, n) = (0.01, 0.9, 0.999, 1e-8, 5usize, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bm = {
		let __up = &m0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bv = {
		let __up = &v0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let blr = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb1 = {
		let __up = &[b1];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb2 = {
		let __up = &[b2];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let beps = {
		let __up = &[eps];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::kernels::gpu_adam_update(&bg, &blr, &bb1, &bb2, &beps, t, n, &bw, &bm, &bv).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let bc1 = 1.0 - b1.powi(t as i32);
	let bc2 = 1.0 - b2.powi(t as i32);
	let want: Vec<f64> = (0..n)
		.map(|i| {
			let m = b1 * m0[i] + (1.0 - b1) * g[i];
			let v = b2 * v0[i] + (1.0 - b2) * g[i] * g[i];
			w[i] - lr * (m / bc1) / ((v / bc2).sqrt() + eps)
		})
		.collect();
	close(&gw, &want)
}

fn prove_adamw() -> bool {
	let (lr, b1, b2, eps, wd, t, n) = (0.01, 0.9, 0.999, 1e-8, 0.05, 5usize, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bm = {
		let __up = &m0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bv = {
		let __up = &v0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let blr = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb1 = {
		let __up = &[b1];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb2 = {
		let __up = &[b2];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let beps = {
		let __up = &[eps];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bwd = {
		let __up = &[wd];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::kernels::gpu_adamw_update(&bg, &blr, &bb1, &bb2, &beps, &bwd, t, n, &bw, &bm, &bv).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let bc1 = 1.0 - b1.powi(t as i32);
	let bc2 = 1.0 - b2.powi(t as i32);
	let want: Vec<f64> = (0..n)
		.map(|i| {
			let m = b1 * m0[i] + (1.0 - b1) * g[i];
			let v = b2 * v0[i] + (1.0 - b2) * g[i] * g[i];
			w[i] * (1.0 - lr * wd) - lr * (m / bc1) / ((v / bc2).sqrt() + eps)
		})
		.collect();
	close(&gw, &want)
}

fn prove_nadam() -> bool {
	let (lr, b1, b2, eps, t, n) = (0.01, 0.9, 0.999, 1e-8, 5usize, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bm = {
		let __up = &m0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bv = {
		let __up = &v0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let blr = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb1 = {
		let __up = &[b1];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb2 = {
		let __up = &[b2];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let beps = {
		let __up = &[eps];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::optimizers::gpu_nadam_update(&bg, &blr, &bb1, &bb2, &beps, t, n, &bw, &bm, &bv).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let bc1t = 1.0 - b1.powi(t as i32);
	let bc1t1 = 1.0 - b1.powi(t as i32 + 1);
	let bc2 = 1.0 - b2.powi(t as i32);
	let want: Vec<f64> = (0..n)
		.map(|i| {
			let m = b1 * m0[i] + (1.0 - b1) * g[i];
			let v = b2 * v0[i] + (1.0 - b2) * g[i] * g[i];
			let mh = b1 * m / bc1t1 + (1.0 - b1) * g[i] / bc1t;
			w[i] - lr * mh / ((v / bc2).sqrt() + eps)
		})
		.collect();
	close(&gw, &want)
}

fn prove_lamb() -> bool {
	let (lr, b1, b2, eps, wd, t, n) = (0.01, 0.9, 0.999, 1e-6, 0.02, 5i32, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bm = {
		let __up = &m0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bv = {
		let __up = &v0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let tmp_upd = GpuBuffer::alloc(n).unwrap();
	let w_norm_sq = GpuBuffer::alloc(1).unwrap();
	let u_norm_sq = GpuBuffer::alloc(1).unwrap();
	let lr_b = {
		let __up = &[lr];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b1_b = {
		let __up = &[b1];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b2_b = {
		let __up = &[b2];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let eps_b = {
		let __up = &[eps];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let wd_b = {
		let __up = &[wd];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	gpu_core::optimizers::gpu_lamb_phase1(
		&bg, &b1_b, &b2_b, &eps_b, &wd_b, t as usize, n, &bw, &bm, &bv, &tmp_upd, &w_norm_sq, &u_norm_sq,
	)
	.unwrap();
	gpu_core::optimizers::gpu_lamb_phase2(&tmp_upd, &lr_b, &w_norm_sq, &u_norm_sq, n, &bw).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let bc1 = 1.0 - b1.powi(t);
	let bc2 = 1.0 - b2.powi(t);
	let upd: Vec<f64> = (0..n)
		.map(|i| {
			let m = b1 * m0[i] + (1.0 - b1) * g[i];
			let v = b2 * v0[i] + (1.0 - b2) * g[i] * g[i];
			(m / bc1) / ((v / bc2).sqrt() + eps) + wd * w[i]
		})
		.collect();
	let wn = w.iter().map(|x| x * x).sum::<f64>().sqrt();
	let un = upd.iter().map(|x| x * x).sum::<f64>().sqrt();
	let ratio = if wn == 0.0 || un == 0.0 { 1.0 } else { wn / un };
	let want: Vec<f64> = (0..n).map(|i| w[i] - lr * ratio * upd[i]).collect();
	close(&gw, &want)
}

// ── NEW optimizerx_ kernel proofs ───────────────────────────────────────────

fn prove_nesterov() -> bool {
	let (lr, mu, n) = (0.05, 0.9, N);
	let w = wv(n);
	let g = gv(n);
	let b0 = sv(n, 0.1, 0.01);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	unsafe {
		launch_optimizerx_nesterov(
			bw.ptr_raw(),
			bb.ptr_raw(),
			bg.ptr_raw() as *const c_void,
			lr,
			mu,
			n as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut gb = vec![0.0; n];
	unsafe { bb.download_async(&mut gb, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let want_b: Vec<f64> = (0..n).map(|i| mu * b0[i] + g[i]).collect();
	let want_w: Vec<f64> = (0..n)
		.map(|i| w[i] - lr * (g[i] + mu * want_b[i]))
		.collect();
	close(&gw, &want_w) && close(&gb, &want_b)
}

fn prove_adadelta() -> bool {
	let (lr, rho, eps, n) = (1.0, 0.95, 1e-6, N);
	let w = wv(n);
	let g = gv(n);
	let eg0 = sv(n, 0.3, 0.01);
	let edx0 = sv(n, 0.15, 0.008);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let beg = {
		let __up = &eg0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bedx = {
		let __up = &edx0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	unsafe {
		launch_optimizerx_adadelta(
			bw.ptr_raw(),
			beg.ptr_raw(),
			bedx.ptr_raw(),
			bg.ptr_raw() as *const c_void,
			lr,
			rho,
			eps,
			n as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut geg = vec![0.0; n];
	unsafe { beg.download_async(&mut geg, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut gedx = vec![0.0; n];
	unsafe { bedx.download_async(&mut gedx, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let want_eg: Vec<f64> = (0..n)
		.map(|i| rho * eg0[i] + (1.0 - rho) * g[i] * g[i])
		.collect();
	let dx: Vec<f64> = (0..n)
		.map(|i| -(edx0[i] + eps).sqrt() / (want_eg[i] + eps).sqrt() * g[i])
		.collect();
	let want_edx: Vec<f64> = (0..n)
		.map(|i| rho * edx0[i] + (1.0 - rho) * dx[i] * dx[i])
		.collect();
	let want_w: Vec<f64> = (0..n).map(|i| w[i] + lr * dx[i]).collect();
	close(&gw, &want_w) && close(&geg, &want_eg) && close(&gedx, &want_edx)
}

fn radam_step(t: usize) -> (Vec<f64>, Vec<f64>) {
	let (lr, b1, b2, eps, n) = (0.01, 0.9, 0.999, 1e-8, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bm = {
		let __up = &m0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bv = {
		let __up = &v0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	unsafe {
		launch_optimizerx_radam(
			bw.ptr_raw(),
			bm.ptr_raw(),
			bv.ptr_raw(),
			bg.ptr_raw() as *const c_void,
			lr,
			b1,
			b2,
			eps,
			t as i32,
			n as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let b1t = b1.powi(t as i32);
	let b2t = b2.powi(t as i32);
	let rho_inf = 2.0 / (1.0 - b2) - 1.0;
	let rho_t = rho_inf - 2.0 * t as f64 * b2t / (1.0 - b2t);
	let want: Vec<f64> = (0..n)
		.map(|i| {
			let m = b1 * m0[i] + (1.0 - b1) * g[i];
			let v = b2 * v0[i] + (1.0 - b2) * g[i] * g[i];
			let mh = m / (1.0 - b1t);
			if rho_t > 4.0 {
				let l = ((1.0 - b2t) / (v + eps)).sqrt();
				let r = (((rho_t - 4.0) * (rho_t - 2.0) * rho_inf)
					/ ((rho_inf - 4.0) * (rho_inf - 2.0) * rho_t))
					.sqrt();
				w[i] - lr * r * mh * l
			} else {
				w[i] - lr * mh
			}
		})
		.collect();
	(gw, want)
}
fn prove_radam() -> bool {
	let rho_inf = 2.0 / (1.0 - 0.999_f64) - 1.0;
	let rho_at = |t: usize| {
		let b2t = 0.999_f64.powi(t as i32);
		rho_inf - 2.0 * t as f64 * b2t / (1.0 - b2t)
	};
	let t_lo = 4usize; // unrectified branch
	let t_hi = 12usize; // rectified branch
	assert!(
		rho_at(t_lo) <= 4.0,
		"radam t_lo branch setup wrong: rho_t={}",
		rho_at(t_lo)
	);
	assert!(
		rho_at(t_hi) > 4.0,
		"radam t_hi branch setup wrong: rho_t={}",
		rho_at(t_hi)
	);
	let (g1, w1) = radam_step(t_lo);
	let (g2, w2) = radam_step(t_hi);
	close(&g1, &w1) && close(&g2, &w2)
}

fn prove_lars() -> bool {
	let (lr, mu, wd, lambda, eps, n) = (0.05, 0.9, 0.01, 1.0, 1e-8, N);
	let w = wv(n);
	let g = gv(n);
	let b0 = sv(n, 0.1, 0.005);
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b0;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bg = {
		let __up = &g;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let wns = GpuBuffer::alloc(1).unwrap();
	let gns = GpuBuffer::alloc(1).unwrap();
	wns.memset_zero(8).unwrap();
	gns.memset_zero(8).unwrap();
	unsafe {
		launch_optimizerx_lars_phase1(
			bw.ptr_raw() as *const c_void,
			bg.ptr_raw() as *const c_void,
			n as i32,
			wns.ptr_raw(),
			gns.ptr_raw(),
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut wn = [0.0];
	let mut gn = [0.0];
	unsafe { wns.download_async(&mut wn, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { gns.download_async(&mut gn, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe {
		launch_optimizerx_lars_phase2(
			bw.ptr_raw(),
			bb.ptr_raw(),
			bg.ptr_raw() as *const c_void,
			lr,
			mu,
			wd,
			lambda,
			wn[0],
			gn[0],
			eps,
			n as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	unsafe { bw.download_async(&mut gw, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut gb = vec![0.0; n];
	unsafe { bb.download_async(&mut gb, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let w_norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
	let g_norm = g.iter().map(|x| x * x).sum::<f64>().sqrt();
	if (wn[0] - w.iter().map(|x| x * x).sum::<f64>()).abs() > 1e-9 * (1.0 + wn[0].abs()) {
		return false;
	}
	if (gn[0] - g.iter().map(|x| x * x).sum::<f64>()).abs() > 1e-9 * (1.0 + gn[0].abs()) {
		return false;
	}
	let trust = if w_norm > 0.0 && g_norm > 0.0 {
		lambda * w_norm / (g_norm + wd * w_norm + eps)
	} else {
		1.0
	};
	let want_b: Vec<f64> = (0..n)
		.map(|i| mu * b0[i] + trust * (g[i] + wd * w[i]))
		.collect();
	let want_w: Vec<f64> = (0..n).map(|i| w[i] - lr * want_b[i]).collect();
	close(&gw, &want_w) && close(&gb, &want_b)
}

// ── registry ────────────────────────────────────────────────────────────────
fn registry() -> HashMap<&'static str, fn() -> bool> {
	let mut m: HashMap<&'static str, fn() -> bool> = HashMap::new();
	m.insert("sgd", prove_sgd);
	m.insert("momentum", prove_momentum);
	m.insert("rmsprop", prove_rmsprop);
	m.insert("adagrad", prove_adagrad);
	m.insert("lion", prove_lion);
	m.insert("adam", prove_adam);
	m.insert("adamw", prove_adamw);
	m.insert("nadam", prove_nadam);
	m.insert("lamb", prove_lamb);
	m.insert("nesterov", prove_nesterov);
	m.insert("adadelta", prove_adadelta);
	m.insert("radam", prove_radam);
	m.insert("lars", prove_lars);
	m
}

fn canon(name: &str) -> String {
	let mut s = name
		.rsplit(['.', ':'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	while let Some(r) = s.strip_prefix('_') {
		s = r.to_string();
	}
	let prefixes = [
		"resourcesparseapply",
		"resourceapply",
		"sparseapply",
		"apply",
		"multi_tensor_",
		"fused",
		"deepspeedcpu",
		"paged_",
		"scale_by_",
		"keras",
		"noisy_",
		"polyak_",
		"mixedprecision",
		"dp",
		"mb",
	];
	loop {
		let mut changed = false;
		for p in prefixes {
			if let Some(r) = s.strip_prefix(p)
				&& !r.is_empty()
			{
				s = r.to_string();
				changed = true;
				break;
			}
		}
		if !changed {
			break;
		}
	}
	while let Some(r) = s.strip_suffix('_') {
		s = r.to_string();
	}
	let suffixes = [
		"_capturable_master",
		"_capturable",
		"_distributed",
		"_cpuoffload",
		"_optimizer",
		"_update",
		"_8bit",
		"_4bit",
		"_32bit",
		"_master",
		"_stage1",
		"_stage2",
		"_v2",
	];
	loop {
		let mut changed = false;
		for suf in suffixes {
			if let Some(r) = s.strip_suffix(suf)
				&& !r.is_empty()
			{
				s = r.to_string();
				changed = true;
				break;
			}
		}
		if !changed {
			break;
		}
	}
	let alias: &[(&str, &str)] = &[
		("sgd", "sgd"),
		("gradientdescent", "sgd"),
		("sgdupdate", "sgd"),
		("momentum", "momentum"),
		("nesterov", "nesterov"),
		("rmsprop", "rmsprop"),
		("adagrad", "adagrad"),
		("adadelta", "adadelta"),
		("adam", "adam"),
		("adamoptimizer", "adam"),
		("adamw", "adamw"),
		("adamwupdate", "adamw"),
		("nadam", "nadam"),
		("radam", "radam"),
		("lamb", "lamb"),
		("lambupdate", "lamb"),
		("lars", "lars"),
		("lion", "lion"),
	];
	for (a, c) in alias {
		if s == *a {
			return (*c).to_string();
		}
	}
	s
}

fn load_optimizer() -> Vec<String> {
	let dir = common::inventory_dir();
	let mut items = Vec::new();
	let rd = fs::read_dir(&dir).expect("no recipe.db");
	for e in rd.flatten() {
		let p = e.path();
		if p.extension().is_some_and(|x| x == "json") {
			let Ok(txt) = fs::read_to_string(&p) else {
				continue;
			};
			let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else {
				continue;
			};
			if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
				for k in ks {
					let cat = k.get("category").and_then(|c| c.as_str()).unwrap_or("");
					if cat != "optimizer" {
						continue;
					}
					let name = k
						.get("name")
						.and_then(|n| n.as_str())
						.unwrap_or("")
						.to_string();
					if !name.is_empty() {
						items.push(name);
					}
				}
			}
		}
	}
	items.sort();
	items.dedup();
	items
}

#[test]
fn prove_optimizer() {
	let items = load_optimizer();
	assert!(!items.is_empty(), "no optimizer items in inventory");
	let reg = registry();

	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	let mut failures: Vec<String> = Vec::new();
	for (k, f) in reg.iter() {
		let ok = f();
		op_ok.insert(*k, ok);
		if !ok {
			failures.push((*k).to_string());
		}
	}

	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<String> = Default::default();
	for name in &items {
		let key = canon(name);
		if let Some(&ok) = op_ok.get(key.as_str())
			&& ok
		{
			proven += 1;
			proven_keys.insert(key);
		}
	}

	let mut impls: Vec<&str> = reg.keys().copied().collect();
	impls.sort();
	eprintln!("\n=== PROVE optimizer ===");
	eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);
	eprintln!("PROVE optimizer: {} / {}", proven, total);

	assert!(
		failures.is_empty(),
		"registered optimizer op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero optimizer items proven");
	let implemented = "nesterov,lars,adamw,nadam,radam,adadelta";
	eprintln!(
		"RESULT optimizer: proven={} total={} green=true implemented={}",
		proven, total, implemented
	);
}
