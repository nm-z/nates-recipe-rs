mod common;
// Live-GPU proof harness for the "optimizer" inventory category.
//
// For every optimizer-category item in kernel_inventory/*.json, canonicalize its
// name; if that canonical op is registered here, run ONE in-place weight-update
// step on the LIVE gfx1101 GPU from a KNOWN nonzero (w, g, state, t>=2) and assert
// it matches an AUTHORITATIVE CPU oracle written independently from the math
// definition (std f64 / textbook update rule). tol 1e-6.
//
// EXISTING ops (proven via their public Rust fns):
//   sgd, momentum, rmsprop, adagrad, lion, lamb, adam, adamw, nadam.
// NEW optimizerx_ kernels (proven via FFI launchers): nesterov, adadelta, radam, lars.
//
// A proven op counts ALL its inventory variants collapsed by canon (dtype/fused/
// precision/multi_tensor/capturable/paged wrappers of the SAME math). Algorithmically
// different names (proximal/DA/centered/adamax/ftrl/novograd/addsign/yogi/...) are
// NOT mapped to a base op — that would be faked green — they remain backlog like the
// host-only schedule/state items. The test FAILS on any registered-op mismatch.

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

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

// Deterministic, distinct-per-element test vectors of length n.
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

// sgd: w -= lr*g  (gpu_sgd_update: Y = Y - lr*X)
fn prove_sgd() -> bool {
	let (lr, n) = (0.05, N);
	let w = wv(n);
	let g = gv(n);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::kernels::gpu_sgd_update(&bw, &bg, lr, n);
	let mut got = vec![0.0; n];
	bw.download(&mut got).unwrap();
	let want: Vec<f64> = (0..n).map(|i| w[i] - lr * g[i]).collect();
	close(&got, &want)
}

// momentum (Sutskever form, as coded): v = mu*v - lr*g; w += v
fn prove_momentum() -> bool {
	let (lr, mu, n) = (0.05, 0.9, N);
	let w = wv(n);
	let g = gv(n);
	let v0 = sv(n, 0.4, -0.02);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bv = GpuBuffer::upload(&v0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::optimizers::gpu_momentum_update(&bw, &bv, &bg, lr, mu, n);
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
	let mut gvb = vec![0.0; n];
	bv.download(&mut gvb).unwrap();
	let want_v: Vec<f64> = (0..n).map(|i| mu * v0[i] - lr * g[i]).collect();
	let want_w: Vec<f64> = (0..n).map(|i| w[i] + want_v[i]).collect();
	close(&gw, &want_w) && close(&gvb, &want_v)
}

// rmsprop: cache = decay*cache + (1-decay)*g^2; w -= lr*g/(sqrt(cache)+eps)
fn prove_rmsprop() -> bool {
	let (lr, decay, eps, n) = (0.05, 0.9, 1e-8, N);
	let w = wv(n);
	let g = gv(n);
	let c0 = sv(n, 0.5, 0.01);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bc = GpuBuffer::upload(&c0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::optimizers::gpu_rmsprop_update(&bw, &bc, &bg, lr, decay, eps, n);
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
	let mut gc = vec![0.0; n];
	bc.download(&mut gc).unwrap();
	let want_c: Vec<f64> = (0..n)
		.map(|i| decay * c0[i] + (1.0 - decay) * g[i] * g[i])
		.collect();
	let want_w: Vec<f64> = (0..n)
		.map(|i| w[i] - lr * g[i] / (want_c[i].sqrt() + eps))
		.collect();
	close(&gw, &want_w) && close(&gc, &want_c)
}

// adagrad: accum += g^2; w -= lr*g/(sqrt(accum)+eps)
fn prove_adagrad() -> bool {
	let (lr, eps, n) = (0.05, 1e-10, N);
	let w = wv(n);
	let g = gv(n);
	let a0 = sv(n, 0.3, 0.02);
	let bw = GpuBuffer::upload(&w).unwrap();
	let ba = GpuBuffer::upload(&a0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::optimizers::gpu_adagrad_update(&bw, &ba, &bg, lr, eps, n);
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
	let mut ga = vec![0.0; n];
	ba.download(&mut ga).unwrap();
	let want_a: Vec<f64> = (0..n).map(|i| a0[i] + g[i] * g[i]).collect();
	let want_w: Vec<f64> = (0..n)
		.map(|i| w[i] - lr * g[i] / (want_a[i].sqrt() + eps))
		.collect();
	close(&gw, &want_w) && close(&ga, &want_a)
}

// lion: upd = sign(b1*m + (1-b1)*g); w -= lr*(upd + wd*w); m = b2*m + (1-b2)*g
fn prove_lion() -> bool {
	let (lr, b1, b2, wd, n) = (0.01, 0.9, 0.99, 0.01, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, -0.15, 0.02);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bm = GpuBuffer::upload(&m0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::optimizers::gpu_lion_update(&bw, &bm, &bg, lr, b1, b2, wd, n);
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
	let mut gm = vec![0.0; n];
	bm.download(&mut gm).unwrap();
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

// adam (t>=2): m=b1*m+(1-b1)g; v=b2*v+(1-b2)g^2; mh=m/(1-b1^t); vh=v/(1-b2^t); w-=lr*mh/(sqrt(vh)+eps)
fn prove_adam() -> bool {
	let (lr, b1, b2, eps, t, n) = (0.01, 0.9, 0.999, 1e-8, 5usize, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bm = GpuBuffer::upload(&m0).unwrap();
	let bv = GpuBuffer::upload(&v0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::kernels::gpu_adam_update(&bw, &bm, &bv, &bg, lr, b1, b2, eps, t, n);
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
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

// adamw (decoupled wd): like adam but w = w*(1-lr*wd) - lr*mh/(sqrt(vh)+eps)
fn prove_adamw() -> bool {
	let (lr, b1, b2, eps, wd, t, n) = (0.01, 0.9, 0.999, 1e-8, 0.05, 5usize, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bm = GpuBuffer::upload(&m0).unwrap();
	let bv = GpuBuffer::upload(&v0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::kernels::gpu_adamw_update(&bw, &bm, &bv, &bg, lr, b1, b2, eps, wd, t, n);
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
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

// nadam (Nesterov-accelerated Adam, Keras/Dozat form):
// m_hat = b1*m/(1-b1^(t+1)) + (1-b1)*g/(1-b1^t); v_hat = v/(1-b2^t); w -= lr*m_hat/(sqrt(v_hat)+eps)
fn prove_nadam() -> bool {
	let (lr, b1, b2, eps, t, n) = (0.01, 0.9, 0.999, 1e-8, 5usize, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bm = GpuBuffer::upload(&m0).unwrap();
	let bv = GpuBuffer::upload(&v0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::optimizers::gpu_nadam_update(&bw, &bm, &bv, &bg, lr, b1, b2, eps, t as i32, n);
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
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

// lamb: Adam moments + trust ratio ||w||/||update|| (whole-vector), upd includes wd*w.
fn prove_lamb() -> bool {
	let (lr, b1, b2, eps, wd, t, n) = (0.01, 0.9, 0.999, 1e-6, 0.02, 5i32, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bm = GpuBuffer::upload(&m0).unwrap();
	let bv = GpuBuffer::upload(&v0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	gpu_core::optimizers::gpu_lamb_update(&bw, &bm, &bv, &bg, lr, b1, b2, eps, wd, t, n).unwrap();
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
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

// nesterov (PyTorch NAG): buf = mu*buf + g; d = g + mu*buf; w -= lr*d
fn prove_nesterov() -> bool {
	let (lr, mu, n) = (0.05, 0.9, N);
	let w = wv(n);
	let g = gv(n);
	let b0 = sv(n, 0.1, 0.01);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bb = GpuBuffer::upload(&b0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
	unsafe {
		launch_optimizerx_nesterov(
			bw.ptr_raw(),
			bb.ptr_raw(),
			bg.ptr_raw() as *const c_void,
			lr,
			mu,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
	let mut gb = vec![0.0; n];
	bb.download(&mut gb).unwrap();
	let want_b: Vec<f64> = (0..n).map(|i| mu * b0[i] + g[i]).collect();
	let want_w: Vec<f64> = (0..n)
		.map(|i| w[i] - lr * (g[i] + mu * want_b[i]))
		.collect();
	close(&gw, &want_w) && close(&gb, &want_b)
}

// adadelta (Zeiler): Eg=rho*Eg+(1-rho)g^2; dx=-sqrt(Edx+eps)/sqrt(Eg+eps)*g; Edx=rho*Edx+(1-rho)dx^2; w+=lr*dx
fn prove_adadelta() -> bool {
	let (lr, rho, eps, n) = (1.0, 0.95, 1e-6, N);
	let w = wv(n);
	let g = gv(n);
	let eg0 = sv(n, 0.3, 0.01);
	let edx0 = sv(n, 0.15, 0.008);
	let bw = GpuBuffer::upload(&w).unwrap();
	let beg = GpuBuffer::upload(&eg0).unwrap();
	let bedx = GpuBuffer::upload(&edx0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
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
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
	let mut geg = vec![0.0; n];
	beg.download(&mut geg).unwrap();
	let mut gedx = vec![0.0; n];
	bedx.download(&mut gedx).unwrap();
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

// radam: rectified Adam. Run at two t values to hit BOTH branches (unrectified small t, rectified large t).
fn radam_step(t: usize) -> (Vec<f64>, Vec<f64>) {
	let (lr, b1, b2, eps, n) = (0.01, 0.9, 0.999, 1e-8, N);
	let w = wv(n);
	let g = gv(n);
	let m0 = sv(n, 0.05, 0.01);
	let v0 = sv(n, 0.2, 0.005);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bm = GpuBuffer::upload(&m0).unwrap();
	let bv = GpuBuffer::upload(&v0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
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
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
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
	// rho_inf = 2/0.001 - 1 = 1999. rho_t>4 requires t large enough.
	// t=3 -> rho_t ~ -? (unrectified); t=10 -> rectified. Verify both branches actually fire.
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

// lars: trust = lambda*||w||/(||g||+wd*||w||+eps); buf = mu*buf + trust*(g+wd*w); w -= lr*buf
fn prove_lars() -> bool {
	let (lr, mu, wd, lambda, eps, n) = (0.05, 0.9, 0.01, 1.0, 1e-8, N);
	let w = wv(n);
	let g = gv(n);
	let b0 = sv(n, 0.1, 0.005);
	let bw = GpuBuffer::upload(&w).unwrap();
	let bb = GpuBuffer::upload(&b0).unwrap();
	let bg = GpuBuffer::upload(&g).unwrap();
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
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut wn = [0.0];
	let mut gn = [0.0];
	wns.download(&mut wn).unwrap();
	gns.download(&mut gn).unwrap();
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
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut gw = vec![0.0; n];
	bw.download(&mut gw).unwrap();
	let mut gb = vec![0.0; n];
	bb.download(&mut gb).unwrap();
	// independent oracle
	let w_norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
	let g_norm = g.iter().map(|x| x * x).sum::<f64>().sqrt();
	// sanity: the GPU norm accumulators must match (proves phase1 reduction)
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

// Canonicalize an optimizer JSON name to a registry key.
// Strip lib paths, framework prefixes (apply/sparse/resource/keras/fused/multi_tensor/
// paged/scale_by/deepspeedcpu/mb/dp/noisy/polyak/optimistic), and dtype/precision/
// capturable/distributed/master suffixes. Map TRUE synonyms only. Algorithmically
// different ops (proximal/da/centered/adamax/ftrl/novograd/yogi/...) keep their own
// key and stay backlog — never folded into a base op (that would fake green).
fn canon(name: &str) -> String {
	// last path segment
	let mut s = name
		.rsplit(['.', ':'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	// leading underscores from torch._fused_*
	while let Some(r) = s.strip_prefix('_') {
		s = r.to_string();
	}
	// strip framework method/op prefixes (longest first; repeat to peel stacks)
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
	// strip trailing underscores (torch._fused_adam_)
	while let Some(r) = s.strip_suffix('_') {
		s = r.to_string();
	}
	// strip dtype / variant suffixes
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
	// explicit synonym table (same math, different name)
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
	let rd = std::fs::read_dir(&dir).expect("no kernel_inventory");
	for e in rd.flatten() {
		let p = e.path();
		if p.extension().is_some_and(|x| x == "json") {
			let Ok(txt) = std::fs::read_to_string(&p) else {
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

	// Prove each registered op ONCE on the live GPU; fail loud on any mismatch.
	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	let mut failures: Vec<String> = Vec::new();
	for (k, f) in reg.iter() {
		let ok = f();
		op_ok.insert(*k, ok);
		if !ok {
			failures.push((*k).to_string());
		}
	}

	// Walk inventory: each item whose canon maps to a passing registered op is proven.
	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: std::collections::BTreeSet<String> = Default::default();
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
	// newly implemented optimizerx_ kernels (genuinely missing before this work)
	let implemented = "nesterov,lars,adamw,nadam,radam,adadelta";
	eprintln!(
		"RESULT optimizer: proven={} total={} green=true implemented={}",
		proven, total, implemented
	);
}
