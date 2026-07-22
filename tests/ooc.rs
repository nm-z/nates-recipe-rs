#![allow(unsafe_code)]
use core::ptr;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use recipe_runtime::memory::{Window, chunks, view};

fn cpu_bce_grad(p: &[f64], y: &[f64], n_total: usize) -> Vec<f64> {
	let eps = 1e-7;
	p.iter()
		.zip(y)
		.map(|(&p, &y)| {
			let p = p.clamp(eps, 1.0 - eps);
			(p - y) / (p * (1.0 - p)) / n_total as f64
		})
		.collect()
}

fn cpu_focal_grad(p: &[f64], y: &[f64], gamma: f64, alpha: f64, n_total: usize) -> Vec<f64> {
	let eps = 1e-12;
	p.iter()
		.zip(y)
		.map(|(&p, &t)| {
			let p = p.clamp(eps, 1.0 - eps);
			let p_t = if t > 0.5 { p } else { 1.0 - p };
			let wt = 1.0 - p_t;
			let sign_pt = if t > 0.5 { 1.0 } else { -1.0 };
			let g = -alpha
				* (gamma * wt.powf(gamma - 1.0) * (-sign_pt))
					.mul_add(p_t.ln(), wt.powf(gamma) * sign_pt / p_t);
			g / n_total as f64
		})
		.collect()
}

#[test]
fn ragged_window_bce_focal_grad_matches_full_batch() {
	gpu_core::hip::set_device(0).expect("set_device");
	let n = 7usize;
	let chunk = 3usize;
	let p = [0.9, 0.2, 0.7, 0.4, 0.6, 0.15, 0.85];
	let y = [1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
	let bp = {
		let __up = &p;
		let __ub = GpuBuffer::alloc(__up.len()).expect("p");
		__ub.load(__up).expect("p");
		__ub
	};
	let by = {
		let __up = &y;
		let __ub = GpuBuffer::alloc(__up.len()).expect("y");
		__ub.load(__up).expect("y");
		__ub
	};
	let inv_n = {
		let __up = &[1.0 / n as f64];
		let __ub = GpuBuffer::alloc(__up.len()).expect("inv_n");
		__ub.load(__up).expect("inv_n");
		__ub
	};
	let (gamma, alpha) = (2.0, 0.25);
	let bgamma = {
		let __up = &[gamma];
		let __ub = GpuBuffer::alloc(__up.len()).expect("gamma");
		__ub.load(__up).expect("gamma");
		__ub
	};
	let balpha = {
		let __up = &[alpha];
		let __ub = GpuBuffer::alloc(__up.len()).expect("alpha");
		__ub.load(__up).expect("alpha");
		__ub
	};

	let da_full = GpuBuffer::alloc(n).expect("da");
	kernels::gpu_bce_grad_into(&bp, &by, &inv_n, n, &da_full).expect("bce full");
	let mut got = vec![0.0; n];
	unsafe { da_full.download_async(&mut got, ptr::null_mut()) }.expect("dl");
	gpu_core::hip::device_synchronize().expect("dl sync");
	let want = cpu_bce_grad(&p, &y, n);
	for i in 0..n {
		assert!(
			(got[i] - want[i]).abs() < 1e-12,
			"bce full[{i}] {} vs {}",
			got[i],
			want[i]
		);
	}

	let da_win = GpuBuffer::alloc(n).expect("da_win");
	for Window { s0, cnt } in chunks(n, chunk) {
		let pw = view(&bp, s0 * 8, cnt * 8);
		let yw = view(&by, s0 * 8, cnt * 8);
		let dw = view(&da_win, s0 * 8, cnt * 8);
		kernels::gpu_bce_grad_into(&pw, &yw, &inv_n, cnt, &dw).expect("bce win");
	}
	unsafe { da_win.download_async(&mut got, ptr::null_mut()) }.expect("dl");
	gpu_core::hip::device_synchronize().expect("dl sync");
	for i in 0..n {
		assert!(
			(got[i] - want[i]).abs() < 1e-12,
			"bce win[{i}] {} vs {}",
			got[i],
			want[i]
		);
	}

	let da_raw = GpuBuffer::alloc(n).expect("da_raw");
	for Window { s0, cnt } in chunks(n, chunk) {
		let inv_cnt = {
			let __up = &[1.0 / cnt as f64];
			let __ub = GpuBuffer::alloc(__up.len()).expect("inv_cnt");
			__ub.load(__up).expect("inv_cnt");
			__ub
		};
		let pw = view(&bp, s0 * 8, cnt * 8);
		let yw = view(&by, s0 * 8, cnt * 8);
		let dw = view(&da_raw, s0 * 8, cnt * 8);
		kernels::gpu_bce_grad_into(&pw, &yw, &inv_cnt, cnt, &dw).expect("bce raw win");
	}
	unsafe { da_raw.download_async(&mut got, ptr::null_mut()) }.expect("dl");
	gpu_core::hip::device_synchronize().expect("dl sync");
	let max_dev = got
		.iter()
		.zip(&want)
		.map(|(g, w)| (g - w).abs())
		.fold(0.0, f64::max);
	assert!(
		max_dev > 1e-3,
		"per-window 1/cnt unexpectedly matched full batch (max dev {max_dev})"
	);

	let want_f = cpu_focal_grad(&p, &y, gamma, alpha, n);
	let da_f = GpuBuffer::alloc(n).expect("da_f");
	gpu_core::losses::gpu_focal_grad_into(&bp, &by, &bgamma, &balpha, &inv_n, n, &da_f)
		.expect("focal full");
	unsafe { da_f.download_async(&mut got, ptr::null_mut()) }.expect("dl");
	gpu_core::hip::device_synchronize().expect("dl sync");
	for i in 0..n {
		assert!(
			(got[i] - want_f[i]).abs() < 1e-12,
			"focal full[{i}] {} vs {}",
			got[i],
			want_f[i]
		);
	}
	let da_fw = GpuBuffer::alloc(n).expect("da_fw");
	for Window { s0, cnt } in chunks(n, chunk) {
		let pw = view(&bp, s0 * 8, cnt * 8);
		let yw = view(&by, s0 * 8, cnt * 8);
		let dw = view(&da_fw, s0 * 8, cnt * 8);
		gpu_core::losses::gpu_focal_grad_into(&pw, &yw, &bgamma, &balpha, &inv_n, cnt, &dw)
			.expect("focal win");
	}
	unsafe { da_fw.download_async(&mut got, ptr::null_mut()) }.expect("dl");
	gpu_core::hip::device_synchronize().expect("dl sync");
	for i in 0..n {
		assert!(
			(got[i] - want_f[i]).abs() < 1e-12,
			"focal win[{i}] {} vs {}",
			got[i],
			want_f[i]
		);
	}
}
