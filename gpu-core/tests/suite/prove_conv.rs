use crate::common;

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;
use std::fs;
use std::ptr;

// ── FFI: convx_ launchers (signatures must match convx.hip EXACTLY) ───────────
unsafe extern "C" {
	fn launch_convx_conv1d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		l: i32,
		cout: i32,
		k: i32,
		lout: i32,
		stride: i32,
		s: *mut c_void,
	);
	fn launch_convx_conv2d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		h: i32,
		w_: i32,
		cout: i32,
		kh: i32,
		kw: i32,
		hout: i32,
		wout: i32,
		s: *mut c_void,
	);
	fn launch_convx_conv3d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		d: i32,
		h: i32,
		w_: i32,
		cout: i32,
		kd: i32,
		kh: i32,
		kw: i32,
		dout: i32,
		hout: i32,
		wout: i32,
		s: *mut c_void,
	);
	fn launch_convx_dwconv1d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		l: i32,
		m: i32,
		k: i32,
		lout: i32,
		s: *mut c_void,
	);
	fn launch_convx_dwconv2d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		h: i32,
		w_: i32,
		m: i32,
		kh: i32,
		kw: i32,
		hout: i32,
		wout: i32,
		s: *mut c_void,
	);
	fn launch_convx_gconv2d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		h: i32,
		w_: i32,
		cout: i32,
		g: i32,
		kh: i32,
		kw: i32,
		hout: i32,
		wout: i32,
		s: *mut c_void,
	);
	fn launch_convx_convtranspose2d(
		x: *const c_void,
		w: *const c_void,
		bias: *const c_void,
		y: *mut c_void,
		n: i32,
		cin: i32,
		h: i32,
		w_: i32,
		cout: i32,
		kh: i32,
		kw: i32,
		hout: i32,
		wout: i32,
		s: *mut c_void,
	);
	fn launch_convx_dilation2d(
		x: *const c_void,
		w: *const c_void,
		y: *mut c_void,
		n: i32,
		c: i32,
		h: i32,
		w_: i32,
		kh: i32,
		kw: i32,
		hout: i32,
		wout: i32,
		erode: i32,
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

fn fill(n: usize, seed: u64) -> Vec<f64> {
	let mut s = seed.wrapping_add(0x9E3779B97F4A7C15);
	(0..n).map(|_| {
		s ^= s << 13;
		s ^= s >> 7;
		s ^= s << 17;
		((s >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
	})
	.collect()
}

fn run_gpu<F: FnOnce(*mut c_void)>(out_len: usize, launch: F) -> Vec<f64> {
	let o = GpuBuffer::alloc(out_len).unwrap();
	launch(o.ptr_raw());
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; out_len];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

// ── Per-op GPU-vs-oracle proofs. Each returns true iff GPU == CPU oracle. ─────

fn prove_conv1d() -> bool {
	let (n, cin, l, cout, k) = (2, 3, 9, 4, 3);
	let lout = l - k + 1;
	let x = fill(n * cin * l, 1);
	let w = fill(cout * cin * k, 2);
	let b = fill(cout, 3);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * cout * lout, |y| unsafe {
		launch_convx_conv1d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			y,
			n as i32,
			cin as i32,
			l as i32,
			cout as i32,
			k as i32,
			lout as i32,
			1, // stride
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * cout * lout];
	for nn in 0..n {
		for co in 0..cout {
			for t in 0..lout {
				let mut acc = b[co];
				for ci in 0..cin {
					for kk in 0..k {
						acc += x[(nn * cin + ci) * l + t + kk]
							* w[(co * cin + ci) * k + kk];
					}
				}
				want[(nn * cout + co) * lout + t] = acc;
			}
		}
	}
	close(&got, &want)
}

fn prove_conv2d() -> bool {
	let (n, cin, h, w_, cout, kh, kw) = (2, 3, 7, 6, 4, 3, 2);
	let (hout, wout) = (h - kh + 1, w_ - kw + 1);
	let x = fill(n * cin * h * w_, 11);
	let w = fill(cout * cin * kh * kw, 12);
	let b = fill(cout, 13);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * cout * hout * wout, |y| unsafe {
		launch_convx_conv2d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			y,
			n as i32,
			cin as i32,
			h as i32,
			w_ as i32,
			cout as i32,
			kh as i32,
			kw as i32,
			hout as i32,
			wout as i32,
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * cout * hout * wout];
	for nn in 0..n {
		for co in 0..cout {
			for oh in 0..hout {
				for ow in 0..wout {
					let mut acc = b[co];
					for ci in 0..cin {
						for ph in 0..kh {
							for pw in 0..kw {
								acc += x[((nn * cin + ci) * h + oh + ph) * w_
									+ ow + pw] * w
									[((co * cin + ci) * kh + ph) * kw + pw];
							}
						}
					}
					want[((nn * cout + co) * hout + oh) * wout + ow] = acc;
				}
			}
		}
	}
	close(&got, &want)
}

fn prove_conv3d() -> bool {
	let (n, cin, d, h, w_, cout, kd, kh, kw) = (2, 2, 5, 4, 4, 3, 2, 2, 3);
	let (dout, hout, wout) = (d - kd + 1, h - kh + 1, w_ - kw + 1);
	let x = fill(n * cin * d * h * w_, 21);
	let w = fill(cout * cin * kd * kh * kw, 22);
	let b = fill(cout, 23);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * cout * dout * hout * wout, |y| unsafe {
		launch_convx_conv3d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			y,
			n as i32,
			cin as i32,
			d as i32,
			h as i32,
			w_ as i32,
			cout as i32,
			kd as i32,
			kh as i32,
			kw as i32,
			dout as i32,
			hout as i32,
			wout as i32,
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * cout * dout * hout * wout];
	for nn in 0..n {
		for co in 0..cout {
			for od in 0..dout {
				for oh in 0..hout {
					for ow in 0..wout {
						let mut acc = b[co];
						for ci in 0..cin {
							for pd in 0..kd {
								for ph in 0..kh {
									for pw in 0..kw {
										acc +=
											x[(((nn * cin + ci) * d
												+ od + pd) * h + oh + ph) * w_
												+ ow + pw] * w[(((co * cin
												+ ci) * kd
												+ pd) * kh + ph)
												* kw + pw];
									}
								}
							}
						}
						want[(((nn * cout + co) * dout + od) * hout + oh) * wout
							+ ow] = acc;
					}
				}
			}
		}
	}
	close(&got, &want)
}

fn prove_depthwise_conv1d() -> bool {
	let (n, cin, l, m, k) = (2, 4, 9, 2, 3);
	let cout = cin * m;
	let lout = l - k + 1;
	let x = fill(n * cin * l, 31);
	let w = fill(cin * m * k, 32);
	let b = fill(cout, 33);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * cout * lout, |y| unsafe {
		launch_convx_dwconv1d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			y,
			n as i32,
			cin as i32,
			l as i32,
			m as i32,
			k as i32,
			lout as i32,
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * cout * lout];
	for nn in 0..n {
		for co in 0..cout {
			for t in 0..lout {
				let ci = co / m;
				let mm = co % m;
				let mut acc = b[co];
				for kk in 0..k {
					acc += x[(nn * cin + ci) * l + t + kk] * w[(ci * m + mm) * k + kk];
				}
				want[(nn * cout + co) * lout + t] = acc;
			}
		}
	}
	close(&got, &want)
}

fn prove_depthwise_conv2d() -> bool {
	let (n, cin, h, w_, m, kh, kw) = (2, 4, 6, 5, 2, 2, 3);
	let cout = cin * m;
	let (hout, wout) = (h - kh + 1, w_ - kw + 1);
	let x = fill(n * cin * h * w_, 41);
	let w = fill(cin * m * kh * kw, 42);
	let b = fill(cout, 43);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * cout * hout * wout, |y| unsafe {
		launch_convx_dwconv2d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			y,
			n as i32,
			cin as i32,
			h as i32,
			w_ as i32,
			m as i32,
			kh as i32,
			kw as i32,
			hout as i32,
			wout as i32,
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * cout * hout * wout];
	for nn in 0..n {
		for co in 0..cout {
			for oh in 0..hout {
				for ow in 0..wout {
					let ci = co / m;
					let mm = co % m;
					let mut acc = b[co];
					for ph in 0..kh {
						for pw in 0..kw {
							acc += x[((nn * cin + ci) * h + oh + ph) * w_ + ow + pw]
								* w[((ci * m + mm) * kh + ph) * kw + pw];
						}
					}
					want[((nn * cout + co) * hout + oh) * wout + ow] = acc;
				}
			}
		}
	}
	close(&got, &want)
}

fn prove_grouped_conv2d() -> bool {
	let (n, cin, h, w_, cout, g, kh, kw) = (2, 6, 5, 5, 4, 2, 2, 2);
	let (hout, wout) = (h - kh + 1, w_ - kw + 1);
	let cin_g = cin / g;
	let cout_g = cout / g;
	let x = fill(n * cin * h * w_, 51);
	let w = fill(cout * cin_g * kh * kw, 52);
	let b = fill(cout, 53);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * cout * hout * wout, |y| unsafe {
		launch_convx_gconv2d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			y,
			n as i32,
			cin as i32,
			h as i32,
			w_ as i32,
			cout as i32,
			g as i32,
			kh as i32,
			kw as i32,
			hout as i32,
			wout as i32,
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * cout * hout * wout];
	for nn in 0..n {
		for co in 0..cout {
			for oh in 0..hout {
				for ow in 0..wout {
					let grp = co / cout_g;
					let ci_base = grp * cin_g;
					let mut acc = b[co];
					for ciw in 0..cin_g {
						let ci = ci_base + ciw;
						for ph in 0..kh {
							for pw in 0..kw {
								acc += x[((nn * cin + ci) * h + oh + ph) * w_
									+ ow + pw] * w
									[((co * cin_g + ciw) * kh + ph) * kw + pw];
							}
						}
					}
					want[((nn * cout + co) * hout + oh) * wout + ow] = acc;
				}
			}
		}
	}
	close(&got, &want)
}

fn prove_convtranspose2d() -> bool {
	let (n, cin, h, w_, cout, kh, kw) = (2, 3, 5, 4, 2, 3, 2);
	let (hout, wout) = (h + kh - 1, w_ + kw - 1);
	let x = fill(n * cin * h * w_, 61);
	let w = fill(cin * cout * kh * kw, 62);
	let b = fill(cout, 63);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = &b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * cout * hout * wout, |y| unsafe {
		launch_convx_convtranspose2d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			y,
			n as i32,
			cin as i32,
			h as i32,
			w_ as i32,
			cout as i32,
			kh as i32,
			kw as i32,
			hout as i32,
			wout as i32,
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * cout * hout * wout];
	for co in 0..cout {
		for nn in 0..n {
			for oh in 0..hout {
				for ow in 0..wout {
					want[((nn * cout + co) * hout + oh) * wout + ow] = b[co];
				}
			}
		}
	}
	for nn in 0..n {
		for ci in 0..cin {
			for ih in 0..h {
				for iw in 0..w_ {
					let xv = x[((nn * cin + ci) * h + ih) * w_ + iw];
					for co in 0..cout {
						for ph in 0..kh {
							for pw in 0..kw {
								let oh = ih + ph;
								let ow = iw + pw;
								want[((nn * cout + co) * hout + oh) * wout + ow] +=
									xv * w[((ci * cout + co) * kh + ph) * kw + pw];
							}
						}
					}
				}
			}
		}
	}
	close(&got, &want)
}

fn prove_dilation2d(erode: bool) -> bool {
	let (n, c, h, w_, kh, kw) = (2, 3, 6, 5, 2, 3);
	let (hout, wout) = (h - kh + 1, w_ - kw + 1);
	let x = fill(n * c * h * w_, 71);
	let w = fill(c * kh * kw, 72);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bw = {
		let __up = &w;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let got = run_gpu(n * c * hout * wout, |y| unsafe {
		launch_convx_dilation2d(
			bx.ptr_raw() as *const c_void,
			bw.ptr_raw() as *const c_void,
			y,
			n as i32,
			c as i32,
			h as i32,
			w_ as i32,
			kh as i32,
			kw as i32,
			hout as i32,
			wout as i32,
			if erode { 1 } else { 0 },
			ptr::null_mut(),
		);
	});
	let mut want = vec![0.0; n * c * hout * wout];
	for nn in 0..n {
		for cc in 0..c {
			for oh in 0..hout {
				for ow in 0..wout {
					let mut best = if erode {
						f64::INFINITY
					} else {
						f64::NEG_INFINITY
					};
					for ph in 0..kh {
						for pw in 0..kw {
							let xv = x[((nn * c + cc) * h + oh + ph) * w_ + ow + pw];
							let wv = w[(cc * kh + ph) * kw + pw];
							let v = if erode { xv - wv } else { xv + wv };
							best = if erode { best.min(v) } else { best.max(v) };
						}
					}
					want[((nn * c + cc) * hout + oh) * wout + ow] = best;
				}
			}
		}
	}
	close(&got, &want)
}

// ── Existing gpu-core ops: im2col (1d+2d) and col2im (1d+2d, scatter-add/fold). ─
fn prove_im2col_2d() -> bool {
	use gpu_core::kernels::gpu_im2col_2d;
	let (n, c, h, w) = (2, 2, 5, 4);
	let (kh, kw) = (2, 3);
	let (oh, ow) = (h - kh + 1, w - kw + 1);
	let x = fill(n * c * h * w, 81);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let mut got = vec![0.0; n * oh * ow * c * kh * kw];
	let bout = GpuBuffer::alloc(n * oh * ow * c * kh * kw).unwrap();
	gpu_im2col_2d(&bx, n, c, h, w, kh, kw, &bout).unwrap();
	unsafe { bout.download_async(&mut got, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let ps = c * kh * kw;
	let mut want = vec![0.0; n * oh * ow * ps];
	for nn in 0..n {
		for ohh in 0..oh {
			for oww in 0..ow {
				let pidx = (nn * oh + ohh) * ow + oww;
				for cc in 0..c {
					for ph in 0..kh {
						for pw in 0..kw {
							let within = (cc * kh + ph) * kw + pw;
							want[pidx * ps + within] =
								x[((nn * c + cc) * h + ohh + ph) * w + oww + pw];
						}
					}
				}
			}
		}
	}
	close(&got, &want)
}

fn prove_im2col_1d() -> bool {
	use gpu_core::kernels::gpu_im2col_1d;
	let (n, p, ks) = (3, 8, 3);
	let out_len = p - ks + 1;
	let x = fill(n * p, 91);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let mut got = vec![0.0; n * out_len * ks];
	let bout = GpuBuffer::alloc(n * out_len * ks).unwrap();
	gpu_im2col_1d(&bx, n, p, ks, &bout).unwrap();
	unsafe { bout.download_async(&mut got, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut want = vec![0.0; n * out_len * ks];
	for i in 0..n {
		for t in 0..out_len {
			for k in 0..ks {
				want[(i * out_len + t) * ks + k] = x[i * p + t + k];
			}
		}
	}
	close(&got, &want)
}

fn prove_col2im_2d() -> bool {
	use gpu_core::kernels::{gpu_col2im_2d, gpu_im2col_2d};
	let (n, c, h, w) = (1, 2, 5, 4);
	let (kh, kw) = (2, 3);
	let x = fill(n * c * h * w, 101);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let (poh, pow) = (h - kh + 1, w - kw + 1);
	let patches = GpuBuffer::alloc(n * poh * pow * c * kh * kw).unwrap();
	gpu_im2col_2d(&bx, n, c, h, w, kh, kw, &patches).unwrap();
	let mut got = vec![0.0; n * c * h * w];
	let bout = GpuBuffer::alloc(n * c * h * w).unwrap();
	gpu_col2im_2d(&patches, n, c, h, w, kh, kw, &bout).unwrap();
	unsafe { bout.download_async(&mut got, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let (oh, ow) = (h - kh + 1, w - kw + 1);
	let mut want = vec![0.0; n * c * h * w];
	for nn in 0..n {
		for cc in 0..c {
			for ohh in 0..oh {
				for oww in 0..ow {
					for ph in 0..kh {
						for pw in 0..kw {
							let ih = ohh + ph;
							let iw = oww + pw;
							want[((nn * c + cc) * h + ih) * w + iw] +=
								x[((nn * c + cc) * h + ih) * w + iw];
						}
					}
				}
			}
		}
	}
	close(&got, &want)
}

fn prove_col2im_1d() -> bool {
	use gpu_core::kernels::{gpu_col2im_1d, gpu_im2col_1d};
	let (n, p, ks) = (2, 8, 3);
	let out_len = p - ks + 1;
	let x = fill(n * p, 111);
	let bx = {
		let __up = &x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let patches = GpuBuffer::alloc(n * out_len * ks).unwrap();
	gpu_im2col_1d(&bx, n, p, ks, &patches).unwrap();
	let mut got = vec![0.0; n * p];
	let bout = GpuBuffer::alloc(n * p).unwrap();
	gpu_col2im_1d(&patches, n, p, ks, &bout).unwrap();
	unsafe { bout.download_async(&mut got, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut want = vec![0.0; n * p];
	for i in 0..n {
		for t in 0..out_len {
			for k in 0..ks {
				want[i * p + t + k] += x[i * p + t + k];
			}
		}
	}
	close(&got, &want)
}

// ── Canonicalization: conv JSON name → registry key (forward-conv buckets only). ─
fn canon(name: &str) -> Option<&'static str> {
	let mut base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	while let Some(s) = base.strip_prefix('_') {
		base = s.to_string();
	}

	let excl_sub = [
		"backprop",
		"bwd",
		"backward",
		"grad",
		"dgrad",
		"wgrad",
		"fused",
		"biasactivation",
		"relufusion",
		"convbnact",
		"quantized",
		"int8",
		"fp8",
		"requantize",
		"algo_",
		"findconvolution",
		"fusionplan",
		"forwardalgorithm",
		"convlstm",
		"locallyconnected",
	];
	for s in excl_sub {
		if base.contains(s) {
			return None;
		}
	}
	let excl_exact = [
		"spacetobatch",
		"spacetobatchnd",
		"batchtospace",
		"batchtospacend",
		"spacetodepth",
		"depthtospace",
		"causal_conv1d_fwd",
		"causal_conv1d_update",
		"causal_conv1d_bwd",
		"causal_conv1d_n_step_update",
		"iconvolutionlayer",
		"ideconvolutionlayer",
		"vxnnconvolutionlayer",
		"bottleneck",
		"forwardimmediate",
		"getconvolutionforwardalgorithm_v7",
		"convolve",     // OpenCV true-flip convolution (distinct)
		"dynamic_conv", // weights are data-dependent (distinct)
		"separable_conv2d",
		"separableconv2d",
		"separableconv1d", // two-stage
		"conv_general_dilated_quantized",
	];
	if excl_exact.contains(&base.as_str()) {
		return None;
	}

	if base.contains("transpose") || base.contains("deconv") {
		return Some("convtranspose2d");
	}

	if base == "depthwiseconv1d" {
		return Some("depthwise_conv1d");
	}
	if base.contains("depthwise") || base == "conv_depthwise2d" || base == "conv_depthwise3d" {
		return Some("depthwise_conv2d");
	}

	if base == "group_conv" || base.starts_with("grouped") || base == "devicegroupedconvfwd" {
		return Some("grouped_conv2d");
	}

	if base == "dilation2d"
		|| base == "dilation2d_native"
		|| base == "erosion2d"
		|| base == "erosion2d_native"
	{
		return Some("dilation2d");
	}

	if base == "im2col" || base == "unfold" || base == "conv_general_dilated_patches" {
		return Some("im2col");
	}
	if base == "col2im" || base == "fold" {
		return Some("col2im");
	}

	if base == "conv1d"
		|| base == "conv_general_dilated"
		|| base == "conv_with_general_padding"
		|| base == "conv_general_dilated_local"
		|| base == "conv_general_dilated_relu"
	{
		return Some("conv1d");
	}

	if base == "conv3d"
		|| base == "conv_3d"
		|| base == "convolution3d"
		|| base == "slow_conv3d_forward"
		|| base == "deviceconv3dfwdxdl"
		|| base == "conv3dfprop"
	{
		return Some("conv3d");
	}

	if base.contains("forward") || base.contains("convfwd") || base == "convforward" {
		return Some("conv2d");
	}
	let conv2d_keys = [
		"conv2d",
		"conv_2d",
		"conv",
		"convolution",
		"convolution2d",
		"slow_conv2d_forward",
		"implicitgemmconvolution",
		"hexagon_fast_conv",
		"edgetpu_custom_conv",
		"stablehlo_convolution",
		"deviceconv2dfwdxdl",
		"conv2dfprop",
		"atrous_conv2d",
		"cudnn_convolution",
		"miopen_convolution",
		"mkldnn_convolution",
		"mps_convolution",
		"deviceconvfwd_bias_activation",
	];
	if conv2d_keys.contains(&base.as_str()) {
		return Some("conv2d");
	}

	None
}

fn load_conv() -> Vec<String> {
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
					if cat != "conv" {
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
fn prove_conv() {
	let items = load_conv();
	assert!(!items.is_empty(), "no conv items in inventory");

	let mut op_ok: HashMap<&'static str, bool> = HashMap::new();
	op_ok.insert("conv1d", prove_conv1d());
	op_ok.insert("conv2d", prove_conv2d());
	op_ok.insert("conv3d", prove_conv3d());
	op_ok.insert("depthwise_conv1d", prove_depthwise_conv1d());
	op_ok.insert("depthwise_conv2d", prove_depthwise_conv2d());
	op_ok.insert("grouped_conv2d", prove_grouped_conv2d());
	op_ok.insert("convtranspose2d", prove_convtranspose2d());
	op_ok.insert(
		"dilation2d",
		prove_dilation2d(false) && prove_dilation2d(true),
	);
	op_ok.insert("im2col", prove_im2col_1d() && prove_im2col_2d());
	op_ok.insert("col2im", prove_col2im_1d() && prove_col2im_2d());

	let failures: Vec<&str> = op_ok
		.iter()
		.filter(|(_, ok)| !**ok)
		.map(|(k, _)| *k)
		.collect();

	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<&str> = Default::default();
	for name in &items {
		if let Some(key) = canon(name)
			&& *op_ok.get(key).unwrap_or(&false)
		{
			proven += 1;
			proven_keys.insert(key);
		}
	}

	let mut impls: Vec<&str> = op_ok.keys().copied().collect();
	impls.sort();
	eprintln!("\n=== PROVE conv ===");
	eprintln!("PROVE conv: {} / {}", proven, total);
	eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().copied().collect::<Vec<_>>().join(", ")
	);

	assert!(
		failures.is_empty(),
		"registered conv op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero conv items proven");
}
