#![allow(unsafe_code)]
use gpu_core::tiered::{Budgets, Full, P, Tiered, human};
use std::ffi::c_void;
use std::path::Path;
use std::ptr;
use std::slice;

#[test]
fn budgets_are_the_sum() {
	let b = Budgets::measure(0, 0, Path::new("/tmp/tiered_budgets.spill"));
	eprintln!(
		"vram_data={} ram_data={} disk_data={} cap={} n_v={} n_r={}",
		human(b.vram_data),
		human(b.ram_data),
		human(b.disk_data),
		human(b.cap),
		b.n_v,
		b.n_r
	);
	assert_eq!(b.cap, b.vram_data + b.ram_data + b.disk_data);
	assert_eq!(b.n_v, b.vram_data / P);
	assert_eq!(b.n_r, b.ram_data / P);
}

#[test]
fn admit_rejects_over_cap() {
	let spill = Path::new("/tmp/tiered_reject.spill");
	let cap = Budgets::measure(0, 0, spill).cap;
	let over = cap * 2;
	match Tiered::alloc(over, 0, 0, spill) {
		Err(Full { need, cap: c }) => {
			assert_eq!(need, over);
			assert!(c < need, "rejected with cap {c} not below need {need}");
		}
		Ok(_) => panic!("admitted a buffer over the ceiling"),
	}
}

#[test]
fn tiled_full_batch_runs_and_matches_whole() {
	use gpu_core::kernels;
	use gpu_core::memory::GpuBuffer;
	gpu_core::hip::set_device(0).expect("dev");
	let (n, d, o) = (60000usize, 16usize, 4usize);
	let epochs = 20;
	let lr = 0.05;
	let mut xh = vec![0f64; n * d];
	let mut yh = vec![0f64; n * o];
	for i in 0..n {
		for j in 0..d {
			xh[i * d + j] = (((i * 7 + j * 13) % 97) as f64) / 97.0 - 0.5;
		}
	}
	for i in 0..n {
		for k in 0..o {
			let mut s = 0.0;
			for j in 0..d {
				s += xh[i * d + j] * ((((j * 3 + k * 5) % 11) as f64) - 5.0) * 0.1;
			}
			yh[i * o + k] = s;
		}
	}
	let dl = |buf: &GpuBuffer, m: usize| -> Vec<f64> {
		let mut h = vec![0f64; m];
		unsafe {
			gpu_core::memory::xfer(
				h.as_mut_ptr() as *mut c_void,
				buf.ptr_raw(),
				m * 8,
				gpu_core::hip::HIP_MEMCPY_D2H,
				ptr::null_mut(),
			)
			.expect("D2H");
		}
		gpu_core::hip::device_synchronize().expect("sync");
		h
	};
	let x_dev = GpuBuffer::alloc(xh.len()).expect("x");
	x_dev.load(&xh).expect("x load");
	let y_dev = GpuBuffer::alloc(yh.len()).expect("y");
	y_dev.load(&yh).expect("y load");
	let make = |sz: usize| GpuBuffer::alloc(sz).expect("buf");
	let scale = lr / n as f64;
	let zero = GpuBuffer::alloc(1).expect("zero");
	zero.load(&[0.0f64]).expect("zero load");
	let neg_scale = GpuBuffer::alloc(1).expect("neg_scale");
	neg_scale.load(&[-scale]).expect("neg_scale load");
	let (w_ref, b_ref) = (make(d * o), make(o));
	let (w_t, b_t) = (make(d * o), make(o));
	kernels::gpu_scale_inplace(&zero, d * o, &w_ref).expect("enq");
	kernels::gpu_scale_inplace(&zero, o, &b_ref).expect("enq");
	kernels::gpu_scale_inplace(&zero, d * o, &w_t).expect("enq");
	kernels::gpu_scale_inplace(&zero, o, &b_t).expect("enq");
	let yhat = make(n * o);
	let (dw, db) = (make(d * o), make(o));
	let (dw_acc, db_acc) = (make(d * o), make(o));
	let rws_bytes = kernels::gpu_reduce_sum_cols_workspace_bytes(n, o)
		.max(kernels::gpu_reduce_sum_cols_workspace_bytes(n * o, 1))
		.max(kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1));
	let reduce_ws = GpuBuffer::alloc_bytes(rws_bytes).expect("rws");
	let dw_partials = make(kernels::gpu_splitk_dw_partials_elems(n, d, o));
	let rows_per_block = 4096usize;
	let window = make(rows_per_block * d);
	kernels::gpu_linear_into(&x_dev, &w_ref, &b_ref, 1, o, d, &yhat).expect("enq");
	gpu_core::hip::device_synchronize().expect("warmup");

	for _ in 0..epochs {
		kernels::gpu_linear_into(&x_dev, &w_ref, &b_ref, n, o, d, &yhat).expect("enq");
		kernels::gpu_sub_inplace(&y_dev, n * o, &yhat).expect("enq");
		kernels::gpu_linear_backward_weights_only_into(
			&yhat,
			&x_dev,
			&reduce_ws,
			&dw_partials,
			n,
			o,
			d,
			&dw,
			&db,
		)
		.expect("enq");
		kernels::gpu_sgd_update(&dw, &neg_scale, d * o, &w_ref).expect("enq");
		kernels::gpu_sgd_update(&db, &neg_scale, o, &b_ref).expect("enq");
	}
	let w_ref_h = dl(&w_ref, d * o);

	let bytes = n * d * 8;
	let mut t = Tiered::alloc_capped(bytes, 1, 1, Path::new("/tmp/tiled_train.spill"));
	let xbytes = unsafe { slice::from_raw_parts(xh.as_ptr() as *const u8, bytes) };
	t.fill(xbytes);
	t.sync().expect("fill");
	assert!(!t.is_contiguous_vram(), "buffer must span >1 tier");
	for _ in 0..epochs {
		kernels::gpu_scale_inplace(&zero, d * o, &dw_acc).expect("enq");
		kernels::gpu_scale_inplace(&zero, o, &db_acc).expect("enq");
		let mut r0 = 0;
		while r0 < n {
			let r = rows_per_block.min(n - r0);
			t.stage_bytes(r0 * d * 8, r * d * 8, window.ptr_raw());
			kernels::gpu_linear_into(&window, &w_t, &b_t, r, o, d, &yhat).expect("enq");
			let yblk = GpuBuffer::borrow(
				unsafe { (y_dev.ptr_raw() as *mut f64).add(r0 * o) as *mut c_void },
				r * o * 8,
			);
			kernels::gpu_sub_inplace(&yblk, r * o, &yhat).expect("enq");
			kernels::gpu_linear_backward_weights_only_into(
				&yhat,
				&window,
				&reduce_ws,
				&dw_partials,
				r,
				o,
				d,
				&dw,
				&db,
			)
			.expect("enq");
			kernels::gpu_add_inplace(&dw, d * o, &dw_acc).expect("enq");
			kernels::gpu_add_inplace(&db, o, &db_acc).expect("enq");
			r0 += r;
		}
		kernels::gpu_sgd_update(&dw_acc, &neg_scale, d * o, &w_t).expect("enq");
		kernels::gpu_sgd_update(&db_acc, &neg_scale, o, &b_t).expect("enq");
	}
	let w_t_h = dl(&w_t, d * o);
	let maxdiff = w_ref_h
		.iter()
		.zip(&w_t_h)
		.map(|(a, b)| (a - b).abs())
		.fold(0.0f64, f64::max);
	eprintln!(
		"[tiled] pages={} spilled=true maxdiff(tiled vs whole)={maxdiff:e}",
		t.pages()
	);
	assert!(
		maxdiff < 1e-9,
		"tiled full-batch must match whole-batch: maxdiff={maxdiff}"
	);
}

#[test]
fn stage_across_three_tiers() {
	gpu_core::hip::set_device(0).expect("set device");
	let spill = Path::new("/tmp/tiered_3tier.spill");
	let pages = 6usize;
	let bytes = pages * P;
	let window_buf = gpu_core::memory::GpuBuffer::alloc_bytes(bytes).expect("window");
	let window = window_buf.ptr_raw();
	let mut t = Tiered::alloc_capped(bytes, 2, 2, spill);
	assert_eq!(t.pages(), pages);
	assert!(!t.is_contiguous_vram(), "capped buffer must span >1 tier");
	let mut src = vec![0u8; bytes];
	for p in 0..pages {
		for i in 0..P {
			src[p * P + i] = (p as u8).wrapping_add(1);
		}
	}
	t.fill(&src);
	t.sync().expect("sync");
	t.stage_into(0, pages, window);
	gpu_core::hip::device_synchronize().expect("sync");
	let mut back = vec![0u8; bytes];
	unsafe {
		gpu_core::memory::xfer(
			back.as_mut_ptr() as *mut c_void,
			window,
			bytes,
			gpu_core::hip::HIP_MEMCPY_D2H,
			ptr::null_mut(),
		)
		.expect("D2H");
	}
	gpu_core::hip::device_synchronize().expect("sync");
	for p in 0..pages {
		let m = (p as u8).wrapping_add(1);
		assert_eq!(back[p * P], m, "page {p} head");
		assert_eq!(back[p * P + P - 1], m, "page {p} tail");
	}
}

#[test]
fn vram_fits_roundtrips() {
	gpu_core::hip::set_device(0).expect("set device");
	let spill = Path::new("/tmp/tiered_fit.spill");
	let bytes = 4 * P;
	let mut t = Tiered::alloc(bytes, 0, 0, spill).expect("alloc");
	assert!(
		t.is_contiguous_vram(),
		"small buffer must be contiguous VRAM"
	);
	assert_eq!(t.pages(), 4);
	let mut src = vec![0u8; bytes];
	for p in 0..4 {
		for i in 0..P {
			src[p * P + i] = (p as u8).wrapping_add(1);
		}
	}
	t.fill(&src);
	t.sync().expect("sync");
	let mut back = vec![0u8; bytes];
	unsafe {
		gpu_core::memory::xfer(
			back.as_mut_ptr() as *mut c_void,
			t.device_ptr(),
			bytes,
			gpu_core::hip::HIP_MEMCPY_D2H,
			ptr::null_mut(),
		)
		.expect("D2H");
	}
	gpu_core::hip::device_synchronize().expect("sync");
	for p in 0..4 {
		let m = (p as u8).wrapping_add(1);
		assert_eq!(back[p * P], m, "page {p} head");
		assert_eq!(back[p * P + P - 1], m, "page {p} tail");
	}
}
