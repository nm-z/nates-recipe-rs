use gpu_core::kernels::{
	gpu_cholesky, gpu_cholesky_workspace_bytes, gpu_dgemv_into, gpu_dger_into,
};
use gpu_core::linalg::{
	gpu_bmm_into, gpu_dasum, gpu_dsyrk, gpu_eigh_sym, gpu_eigh_sym_workspace_bytes,
	gpu_fft_c2c_1d, gpu_idamax, gpu_lu_factor, gpu_lu_factor_workspace_bytes, gpu_lu_solve,
	gpu_lu_solve_workspace_bytes, gpu_potrs, gpu_potrs_workspace_bytes, gpu_qr,
	gpu_qr_workspace_bytes, gpu_rfft_1d, gpu_svd, gpu_svd_workspace_bytes,
};
use gpu_core::memory::GpuBuffer;
use gpu_core::reductions::{
	gpu_argsort, gpu_cummax, gpu_cummax_workspace_bytes, gpu_cumprod,
	gpu_cumprod_workspace_bytes, gpu_cumsum_cols, gpu_cumsum_rows, gpu_dot,
	gpu_dot_workspace_bytes, gpu_l2_norm, gpu_l2_norm_workspace_bytes, gpu_max_all,
	gpu_max_all_workspace_bytes, gpu_mean_all, gpu_mean_all_workspace_bytes, gpu_min_all,
	gpu_min_all_workspace_bytes, gpu_scan_linear_recurrence, gpu_segment_max, gpu_segment_sort,
	gpu_segment_sum, gpu_sort, gpu_sort_by_key, gpu_sum_all, gpu_sum_all_workspace_bytes,
};
use std::ptr;

fn close(a: f64, b: f64) -> bool {
	(a - b).abs() < 1e-6 * (1.0 + b.abs())
}

fn assert_close(label: &str, got: f64, expected: f64) {
	assert!(got.is_finite(), "{}: got non-finite {}", label, got);
	assert!(
		close(got, expected),
		"{}: got {} expected {}",
		label,
		got,
		expected
	);
}

// Download a 1-elem device scalar to host.
fn scalar(out: &GpuBuffer) -> f64 {
	let mut v = [0.0f64; 1];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	v[0]
}

// ── reductions: scalar all-reduces ───────────────────────────────────────────

#[test]
fn test_sum_entire() {
	let x = {
		let __up = &[1.0, 2.0, 3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_sum_all_workspace_bytes(4)).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_sum_all(&x, &ws, 4, &out).unwrap();
	assert_close("sum_all", scalar(&out), 10.0);
}

#[test]
fn test_max_entire() {
	let x = {
		let __up = &[3.0, -1.0, 7.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_max_all_workspace_bytes(4)).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_max_all(&x, &ws, 4, &out).unwrap();
	assert_close("max_all", scalar(&out), 7.0);
}

#[test]
fn test_min_entire() {
	let x = {
		let __up = &[3.0, -1.0, 7.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_min_all_workspace_bytes(4)).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_min_all(&x, &ws, 4, &out).unwrap();
	assert_close("min_all", scalar(&out), -1.0);
}

#[test]
fn test_mean_entire() {
	let x = {
		let __up = &[1.0, 2.0, 3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_mean_all_workspace_bytes(4)).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_mean_all(&x, &ws, 4, &out).unwrap();
	assert_close("mean_all", scalar(&out), 2.5);
}

#[test]
fn test_l2_norm() {
	// ||[3,4]|| = 5
	let x = {
		let __up = &[3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_l2_norm_workspace_bytes(2)).unwrap();
	let sq = GpuBuffer::alloc(2).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_l2_norm(&x, &ws, &sq, 2, &out).unwrap();
	assert_close("l2_norm", scalar(&out), 5.0);
}

#[test]
fn test_dot_reductions() {
	// [1,2,3] · [1,1,1] = 6
	let a = {
		let __up = &[1.0, 2.0, 3.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &[1.0, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_dot_workspace_bytes(3)).unwrap();
	let prod = GpuBuffer::alloc(3).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_dot(&a, &b, &ws, &prod, 3, &out).unwrap();
	assert_close("dot", scalar(&out), 6.0);
}

// ── reductions: sort ──────────────────────────────────────────────────────────

#[test]
fn test_sort_basic() {
	let x = {
		let __up = &[3.0, 1.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(3).unwrap();
	gpu_sort(&x, 3, &out).unwrap();
	let mut v = [0.0f64; 3];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_eq!(v, [1.0, 2.0, 3.0]);
}

#[test]
fn test_sort_non_power_of_two() {
	// n=5 is not a power of two — tests padding/sentinel correctness
	let x = {
		let __up = &[5.0, 3.0, 1.0, 4.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(5).unwrap();
	gpu_sort(&x, 5, &out).unwrap();
	let mut v = [0.0f64; 5];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_eq!(v, [1.0, 2.0, 3.0, 4.0, 5.0], "sort non-power-of-two");
}

#[test]
fn test_sort_size_three() {
	// n=3 — padded to 4
	let x = {
		let __up = &[9.0, -1.0, 5.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(3).unwrap();
	gpu_sort(&x, 3, &out).unwrap();
	let mut v = [0.0f64; 3];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_eq!(v, [-1.0, 5.0, 9.0], "sort size 3 (padded to 4)");
}

#[test]
fn test_argsort_basic() {
	// [30, 10, 20] → indices [1, 2, 0]
	let x = {
		let __up = &[30.0, 10.0, 20.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc_bytes(3 * 4).unwrap();
	gpu_argsort(&x, 3, &out).unwrap();
	let mut v = [0i32; 3];
	out.download_i32(&mut v).unwrap();
	assert_eq!(v, [1, 2, 0], "argsort basic: {:?}", v);
}

#[test]
fn test_argsort_non_power_of_two() {
	// n=5 → padding to 8
	let x = {
		let __up = &[5.0, 1.0, 4.0, 2.0, 3.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc_bytes(5 * 4).unwrap();
	gpu_argsort(&x, 5, &out).unwrap();
	let mut v = [0i32; 5];
	out.download_i32(&mut v).unwrap();
	// sorted order: 1,2,3,4,5 → indices 1,3,4,2,0
	assert_eq!(v, [1, 3, 4, 2, 0], "argsort non-pow2: {:?}", v);
}

#[test]
fn test_sort_by_key() {
	let keys = {
		let __up = &[3.0, 1.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let vals = {
		let __up = &[30.0, 10.0, 20.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ok = GpuBuffer::alloc(3).unwrap();
	let ov = GpuBuffer::alloc(3).unwrap();
	gpu_sort_by_key(&keys, &vals, 3, &ok, &ov).unwrap();
	let mut rk = [0.0f64; 3];
	let mut rv = [0.0f64; 3];
	unsafe { ok.download_async(&mut rk, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { ov.download_async(&mut rv, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_eq!(rk, [1.0, 2.0, 3.0], "sort_by_key keys");
	assert_eq!(rv, [10.0, 20.0, 30.0], "sort_by_key vals");
}

#[test]
fn test_segment_sort() {
	// Two segments: [3,1,2] and [5,4]
	// seg_offsets = [0, 3, 5] — n_segs+1 entries
	let data = {
		let __up = &[3.0, 1.0, 2.0, 5.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let offsets_data = [0i32, 3, 5];
	let offsets = GpuBuffer::upload_i32(&offsets_data).unwrap();
	let out = GpuBuffer::alloc(5).unwrap();
	gpu_segment_sort(&data, &offsets, 5, 2, &out).unwrap();
	let mut v = [0.0f64; 5];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_eq!(v, [1.0, 2.0, 3.0, 4.0, 5.0], "segment_sort: {:?}", v);
}

// ── reductions: cumulative scans ─────────────────────────────────────────────

#[test]
fn test_cumsum_rows() {
	// 2x3 matrix: [[1,2,3],[4,5,6]]
	// row cumsums: [[1,3,6],[4,9,15]]
	let x = {
		let __up = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(6).unwrap();
	gpu_cumsum_rows(&x, 2, 3, &out).unwrap();
	let mut v = [0.0f64; 6];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let expected = [1.0, 3.0, 6.0, 4.0, 9.0, 15.0];
	for i in 0..6 {
		assert_close(&format!("cumsum_rows[{}]", i), v[i], expected[i]);
	}
}

#[test]
fn test_cumsum_cols() {
	// 2x3 matrix (row-major): [[1,2,3],[4,5,6]]
	// col cumsums: [[1,2,3],[5,7,9]]
	let x = {
		let __up = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(6).unwrap();
	gpu_cumsum_cols(&x, 2, 3, &out).unwrap();
	let mut v = [0.0f64; 6];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let expected = [1.0, 2.0, 3.0, 5.0, 7.0, 9.0];
	for i in 0..6 {
		assert_close(&format!("cumsum_cols[{}]", i), v[i], expected[i]);
	}
}

#[test]
fn test_cumprod() {
	// [1,2,3,4] → [1,2,6,24]
	let x = {
		let __up = &[1.0, 2.0, 3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_cumprod_workspace_bytes(4)).unwrap();
	let out = GpuBuffer::alloc(4).unwrap();
	gpu_cumprod(&x, &ws, 4, &out).unwrap();
	let mut v = [0.0f64; 4];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let expected = [1.0, 2.0, 6.0, 24.0];
	for i in 0..4 {
		assert_close(&format!("cumprod[{}]", i), v[i], expected[i]);
	}
}

#[test]
fn test_cummax() {
	// [3,1,4,1,5,2] → [3,3,4,4,5,5]
	let x = {
		let __up = &[3.0, 1.0, 4.0, 1.0, 5.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_cummax_workspace_bytes(6)).unwrap();
	let out = GpuBuffer::alloc(6).unwrap();
	gpu_cummax(&x, &ws, 6, &out).unwrap();
	let mut v = [0.0f64; 6];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let expected = [3.0, 3.0, 4.0, 4.0, 5.0, 5.0];
	for i in 0..6 {
		assert_close(&format!("cummax[{}]", i), v[i], expected[i]);
	}
}

// ── reductions: segment reduce ────────────────────────────────────────────────

#[test]
fn test_segment_sum() {
	// vals = [1,2,3,4,5,6], seg_ids = [0,0,1,1,2,2], n_segs=3
	// expected out = [3.0, 7.0, 11.0]
	let vals = {
		let __up = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let seg_ids = GpuBuffer::upload_i32(&[0i32, 0, 1, 1, 2, 2]).unwrap();
	// segment_sum accumulates via atomicAdd — out must be pre-zeroed.
	let out = {
		let __zb = GpuBuffer::alloc_bytes(3 * 8).unwrap();
		__zb.memset_zero(3 * 8).unwrap();
		__zb
	};
	gpu_segment_sum(&vals, &seg_ids, 6, 3, &out).unwrap();
	let mut v = [0.0f64; 3];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_close("segment_sum[0]", v[0], 3.0);
	assert_close("segment_sum[1]", v[1], 7.0);
	assert_close("segment_sum[2]", v[2], 11.0);
}

#[test]
fn test_segment_max() {
	// vals = [1,5,3,2,4,6], seg_ids = [0,0,1,1,2,2], n_segs=3
	// expected out = [5.0, 3.0, 6.0]
	let vals = {
		let __up = &[1.0, 5.0, 3.0, 2.0, 4.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let seg_ids = GpuBuffer::upload_i32(&[0i32, 0, 1, 1, 2, 2]).unwrap();
	let out = GpuBuffer::alloc(3).unwrap();
	gpu_segment_max(&vals, &seg_ids, 6, 3, &out).unwrap();
	let mut v = [0.0f64; 3];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_close("segment_max[0]", v[0], 5.0);
	assert_close("segment_max[1]", v[1], 3.0);
	assert_close("segment_max[2]", v[2], 6.0);
}

// ── reductions: linear recurrence ────────────────────────────────────────────

#[test]
fn test_scan_linear_recurrence() {
	// h_t = a_t * h_{t-1} + b_t, h_0 = 0
	// 3 steps, dim=2
	// Channel 0: a=[0.5, 0.5, 0.5], b=[1,1,1]
	//   h1 = 0.5*0 + 1 = 1
	//   h2 = 0.5*1 + 1 = 1.5
	//   h3 = 0.5*1.5 + 1 = 1.75
	// Channel 1: a=[2,2,2], b=[1,0,0]
	//   h1 = 2*0 + 1 = 1
	//   h2 = 2*1 + 0 = 2
	//   h3 = 2*2 + 0 = 4
	// Layout: a[t*dim+d], b[t*dim+d]
	let a_data = [0.5f64, 2.0, 0.5, 2.0, 0.5, 2.0];
	let b_data = [1.0f64, 1.0, 1.0, 0.0, 1.0, 0.0];
	let a = {
		let __up = &a_data;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &b_data;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(6).unwrap();
	let st = GpuBuffer::alloc(2).unwrap();
	gpu_scan_linear_recurrence(&a, &b, 3, 2, &out, &st).unwrap();
	let mut v = [0.0f64; 6];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	// v[t*2+d]
	assert_close("recur[0,0] h1_ch0", v[0], 1.0);
	assert_close("recur[0,1] h1_ch1", v[1], 1.0);
	assert_close("recur[1,0] h2_ch0", v[2], 1.5);
	assert_close("recur[1,1] h2_ch1", v[3], 2.0);
	assert_close("recur[2,0] h3_ch0", v[4], 1.75);
	assert_close("recur[2,1] h3_ch1", v[5], 4.0);
}

// ── linalg: L1 BLAS ───────────────────────────────────────────────────────────

#[test]
fn test_ddot() {
	// [1,2,3] · [4,5,6] = 4+10+18 = 32
	let a = {
		let __up = &[1.0, 2.0, 3.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &[4.0, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_dot_workspace_bytes(3)).unwrap();
	let prod = GpuBuffer::alloc(3).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_dot(&a, &b, &ws, &prod, 3, &out).unwrap();
	assert_close("ddot", scalar(&out), 32.0);
}

#[test]
fn test_dnrm2() {
	// ||[3,4]|| = 5
	let x = {
		let __up = &[3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_l2_norm_workspace_bytes(2)).unwrap();
	let sq = GpuBuffer::alloc(2).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_l2_norm(&x, &ws, &sq, 2, &out).unwrap();
	assert_close("dnrm2", scalar(&out), 5.0);
}

#[test]
fn test_dasum() {
	// sum |[-1, 2, -3, 4]| = 1+2+3+4 = 10
	let x = {
		let __up = &[-1.0, 2.0, -3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_dasum(&x, 4, &out).unwrap();
	assert_close("dasum", scalar(&out), 10.0);
}

#[test]
fn test_idamax() {
	// [1, -5, 3, 2] → largest |v| is -5 at index 1
	let x = {
		let __up = &[1.0, -5.0, 3.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_idamax(&x, 4, &out).unwrap();
	let idx = scalar(&out) as i32;
	assert_eq!(idx, 1, "idamax should be 1 (|-5| is largest), got {}", idx);
}

// ── linalg: L2 matrix-vector ──────────────────────────────────────────────────

#[test]
fn test_dgemv_no_trans() {
	// A = [[1,2],[3,4],[5,6]] (3x2 row-major), x = [1,1]
	// y = A @ x = [3, 7, 11]
	let a = {
		let __up = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let x = {
		let __up = &[1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(3).unwrap();
	gpu_dgemv_into(&a, &x, 3, 2, 0, &out).unwrap();
	let mut v = [0.0f64; 3];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_close("dgemv[0]", v[0], 3.0);
	assert_close("dgemv[1]", v[1], 7.0);
	assert_close("dgemv[2]", v[2], 11.0);
}

#[test]
fn test_dgemv_trans() {
	// A = [[1,2],[3,4],[5,6]] (3x2 row-major), x = [1,1,1]
	// y = A^T @ x = [1+3+5, 2+4+6] = [9, 12]
	let a = {
		let __up = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let x = {
		let __up = &[1.0, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(2).unwrap();
	gpu_dgemv_into(&a, &x, 3, 2, 1, &out).unwrap();
	let mut v = [0.0f64; 2];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_close("dgemv_t[0]", v[0], 9.0);
	assert_close("dgemv_t[1]", v[1], 12.0);
}

#[test]
fn test_dger() {
	// x = [1,2], y = [3,4,5] → A = x ⊗ y^T (2x3)
	// A = [[3,4,5],[6,8,10]]
	let x = {
		let __up = &[1.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let y = {
		let __up = &[3.0, 4.0, 5.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	// dger accumulates into out — pre-zero (2*3).
	let out = {
		let __zb = GpuBuffer::alloc_bytes(6 * 8).unwrap();
		__zb.memset_zero(6 * 8).unwrap();
		__zb
	};
	gpu_dger_into(&x, &y, 2, 3, &out).unwrap();
	let mut v = [0.0f64; 6];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let expected = [3.0, 4.0, 5.0, 6.0, 8.0, 10.0];
	for i in 0..6 {
		assert_close(&format!("dger[{}]", i), v[i], expected[i]);
	}
}

// ── linalg: symmetric rank-k ─────────────────────────────────────────────────

#[test]
fn test_dsyrk() {
	// A = [[1,2],[3,4]] (2x2 k=2, n=2, but we want C = A^T@A)
	// A (k=2 x n=2): k rows, n cols
	// gpu_dsyrk(a, n, k) → C = A_rm^T @ A_rm where A is (k×n)
	// A = [[1,2],[3,4]], C = A^T@A = [[1,3],[2,4]] @ [[1,2],[3,4]] = [[10,14],[14,20]]
	// Only lower triangle is written, but let's check diagonal and lower
	let a = {
		let __up = &[1.0, 2.0, 3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let c = GpuBuffer::alloc(4).unwrap();
	gpu_dsyrk(&a, 2, 2, &c).unwrap();
	let mut v = [0.0f64; 4];
	unsafe { c.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	// row-major: v[0]=C(0,0), v[1]=C(0,1), v[2]=C(1,0), v[3]=C(1,1)
	// lower triangle: C(0,0)=10, C(1,0)=14, C(1,1)=20
	assert_close("dsyrk[0,0]", v[0], 10.0);
	assert_close("dsyrk[1,0]", v[2], 14.0);
	assert_close("dsyrk[1,1]", v[3], 20.0);
}

// ── linalg: strided batched GEMM ─────────────────────────────────────────────

#[test]
fn test_dgemm_strided_batched() {
	// batch=2, m=2, n=2, k=2
	// A[0] = [[1,2],[3,4]], B[0] = [[1,0],[0,1]] → C[0] = [[1,2],[3,4]]
	// A[1] = [[2,0],[0,2]], B[1] = [[3,1],[1,3]] → C[1] = [[6,2],[2,6]]
	let a = {
		let __up = &[
			1.0, 2.0, 3.0, 4.0, // A[0]
			2.0, 0.0, 0.0, 2.0, // A[1]
		];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &[
			1.0, 0.0, 0.0, 1.0, // B[0]
			3.0, 1.0, 1.0, 3.0, // B[1]
		];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let c = GpuBuffer::alloc(8).unwrap();
	// batch=2, m=n=k=2, contiguous no-transpose: lda=k, ldb=n, ldc=n, strides = m*k / k*n / m*n.
	gpu_bmm_into(&a, &b, 2, 2, 2, 2, 2, 2, 2, 4, 4, 4, 0, 0, 0, 0, 0, &c).unwrap();
	let mut v = [0.0f64; 8];
	unsafe { c.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let expected = [1.0, 2.0, 3.0, 4.0, 6.0, 2.0, 2.0, 6.0];
	for i in 0..8 {
		assert_close(&format!("batched_gemm[{}]", i), v[i], expected[i]);
	}
}

// ── linalg: LU factor+solve ───────────────────────────────────────────────────

#[test]
fn test_lu_solve() {
	// A = [[2,1],[1,3]], b = [5, 10]
	// Solution: x = A^{-1} b
	// det=5, A^{-1} = (1/5)*[[3,-1],[-1,2]]
	// x = (1/5)*[3*5-1*10, -1*5+2*10] = (1/5)*[5,15] = [1,3]
	let a = {
		let __up = &[2.0, 1.0, 1.0, 3.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &[5.0, 10.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let fac_work = GpuBuffer::alloc_bytes(gpu_lu_factor_workspace_bytes(2)).unwrap();
	let lu = GpuBuffer::alloc(2 * 2).unwrap();
	let ipiv = GpuBuffer::alloc_bytes(2 * 4).unwrap();
	let info = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_lu_factor(&a, 2, &fac_work, &lu, &ipiv, &info).unwrap();

	let solve_work = GpuBuffer::alloc_bytes(gpu_lu_solve_workspace_bytes(2, 1)).unwrap();
	let x = GpuBuffer::alloc(2).unwrap();
	gpu_lu_solve(&lu, &ipiv, &b, 2, 1, &solve_work, &info, &x).unwrap();

	let mut v = [0.0f64; 2];
	unsafe { x.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_close("lu_solve x[0]", v[0], 1.0);
	assert_close("lu_solve x[1]", v[1], 3.0);
}

// ── linalg: Cholesky potrs ────────────────────────────────────────────────────

#[test]
fn test_potrs() {
	// A = [[4,2],[2,3]] is SPD.
	// Cholesky: L = [[2,0],[1, sqrt(2)]]
	// Solve A*x = b = [8, 7]
	// x = A^{-1}*b. A = [[4,2],[2,3]], det=8
	// x = (1/8)*[[3,-2],[-2,4]]*[8,7] = (1/8)*[24-14, -16+28] = (1/8)*[10,12] = [1.25, 1.5]
	let a = {
		let __up = &[4.0, 2.0, 2.0, 3.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &[8.0, 7.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let chol_work = GpuBuffer::alloc_bytes(gpu_cholesky_workspace_bytes(2)).unwrap();
	let info = GpuBuffer::alloc_bytes(4).unwrap();
	let l = GpuBuffer::alloc(2 * 2).unwrap();
	gpu_cholesky(&a, 2, &chol_work, &info, &l).unwrap();

	let potrs_work = GpuBuffer::alloc_bytes(gpu_potrs_workspace_bytes(2, 1)).unwrap();
	let x = GpuBuffer::alloc(2).unwrap();
	gpu_potrs(&l, &b, 2, 1, &potrs_work, &info, &x).unwrap();

	let mut v = [0.0f64; 2];
	unsafe { x.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	assert_close("potrs x[0]", v[0], 1.25);
	assert_close("potrs x[1]", v[1], 1.5);
}

// ── linalg: QR decomposition ─────────────────────────────────────────────────

#[test]
fn test_qr_square_reconstruction() {
	// A = [[3,1],[4,2]] (2x2 row-major). Verify Q*R == A and Q orthonormal.
	let a_data = [3.0f64, 1.0, 4.0, 2.0];
	let a = {
		let __up = &a_data;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let (m, n) = (2usize, 2usize);
	let work = GpuBuffer::alloc_bytes(gpu_qr_workspace_bytes(m, n)).unwrap();
	let tau = GpuBuffer::alloc(m.min(n)).unwrap();
	let info = GpuBuffer::alloc_bytes(4).unwrap();
	let q = GpuBuffer::alloc(m * n).unwrap();
	let r = GpuBuffer::alloc(n * n).unwrap();
	gpu_qr(&a, m, n, &work, &tau, &info, &q, &r).unwrap();

	let mut q_v = [0.0f64; 4];
	let mut r_v = [0.0f64; 4];
	unsafe { q.download_async(&mut q_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { r.download_async(&mut r_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	for (i, &x) in q_v.iter().enumerate() {
		assert!(x.is_finite(), "Q[{}] non-finite: {}", i, x);
	}
	for (i, &x) in r_v.iter().enumerate() {
		assert!(x.is_finite(), "R[{}] non-finite: {}", i, x);
	}

	// Q col-major lda=m=2: Q[i,k] = q_v[k*2+i]
	// R col-major lda=n=2: R[k,j] = r_v[j*2+k] for k<=j
	// Verify Q*R == A_rm[i,j] = a_data[i*2+j]
	for i in 0..m {
		for j in 0..n {
			let mut qr_ij = 0.0f64;
			for k in 0..n {
				let q_ik = q_v[k * m + i];
				let r_kj = if k <= j { r_v[j * n + k] } else { 0.0 };
				qr_ij += q_ik * r_kj;
			}
			assert_close(&format!("QR_sq[{},{}]", i, j), qr_ij, a_data[i * n + j]);
		}
	}

	// Q^T Q = I (2x2)
	for i in 0..2usize {
		for j in 0..2usize {
			let mut dot = 0.0f64;
			for k in 0..2usize {
				dot += q_v[i * 2 + k] * q_v[j * 2 + k];
			}
			assert_close(
				&format!("sq_Q^TQ[{},{}]", i, j),
				dot,
				if i == j { 1.0 } else { 0.0 },
			);
		}
	}
}

#[test]
fn test_qr_thin_reconstruction() {
	// A = [[1,2],[3,4],[5,6]] (3x2 row-major). Verify Q*R == A and Q orthonormal.
	let a_data = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
	let a = {
		let __up = &a_data;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let (m, n) = (3usize, 2usize);
	let work = GpuBuffer::alloc_bytes(gpu_qr_workspace_bytes(m, n)).unwrap();
	let tau = GpuBuffer::alloc(m.min(n)).unwrap();
	let info = GpuBuffer::alloc_bytes(4).unwrap();
	let q = GpuBuffer::alloc(m * n).unwrap();
	let r = GpuBuffer::alloc(n * n).unwrap();
	gpu_qr(&a, m, n, &work, &tau, &info, &q, &r).unwrap();

	let mut q_v = [0.0f64; 6];
	let mut r_v = [0.0f64; 4];
	unsafe { q.download_async(&mut q_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { r.download_async(&mut r_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	for (i, &x) in q_v.iter().enumerate() {
		assert!(x.is_finite(), "Q[{}] non-finite: {}", i, x);
	}
	for (i, &x) in r_v.iter().enumerate() {
		assert!(x.is_finite(), "R[{}] non-finite: {}", i, x);
	}

	// Q col-major lda=m=3: Q[i,k] = q_v[k*3+i]
	// R col-major lda=n=2: R[k,j] = r_v[j*2+k] for k<=j
	// Verify Q*R == A_rm[i,j] = a_data[i*2+j]
	for i in 0..m {
		for j in 0..n {
			let mut qr_ij = 0.0f64;
			for k in 0..n {
				let q_ik = q_v[k * m + i];
				let r_kj = if k <= j { r_v[j * n + k] } else { 0.0 };
				qr_ij += q_ik * r_kj;
			}
			assert_close(&format!("QR_tall[{},{}]", i, j), qr_ij, a_data[i * n + j]);
		}
	}

	// Q^T Q = I (2x2)
	for i in 0..2usize {
		for j in 0..2usize {
			let mut dot = 0.0f64;
			for k in 0..3usize {
				dot += q_v[i * 3 + k] * q_v[j * 3 + k];
			}
			assert_close(
				&format!("tall_Q^TQ[{},{}]", i, j),
				dot,
				if i == j { 1.0 } else { 0.0 },
			);
		}
	}
}

// ── linalg: symmetric eigendecomposition ────────────────────────────────────

#[test]
fn test_eigh_sym() {
	// A = [[2,1],[1,2]], eigenvalues = [1, 3]
	// eigenvectors: v1 = [1/√2, -1/√2], v2 = [1/√2, 1/√2]
	// dsyevd overwrites A with eigenvectors (columns in cm = rows in result)
	// A is symmetric: A_rm == A_cm
	let a = {
		let __up = &[2.0f64, 1.0, 1.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let n = 2usize;
	let work = GpuBuffer::alloc_bytes(gpu_eigh_sym_workspace_bytes(n)).unwrap();
	let info = GpuBuffer::alloc_bytes(4).unwrap();
	let evals = GpuBuffer::alloc(n).unwrap();
	let evecs = GpuBuffer::alloc(n * n).unwrap();
	gpu_eigh_sym(&a, n, &work, &info, &evals, &evecs).unwrap();

	let mut ev = [0.0f64; 2];
	unsafe { evals.download_async(&mut ev, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let mut vc = [0.0f64; 4];
	unsafe { evecs.download_async(&mut vc, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	assert_close("eigh eval[0]", ev[0], 1.0);
	assert_close("eigh eval[1]", ev[1], 3.0);

	// Verify A @ v = λ @ v for each eigenvector
	// evecs is column-major: vc[col*n+row] = V[row, col]
	let a_data = [2.0f64, 1.0, 1.0, 2.0];
	for j in 0..2usize {
		let lam = ev[j];
		for i in 0..2usize {
			let mut av_i = 0.0f64;
			for k in 0..2usize {
				av_i += a_data[i * 2 + k] * vc[j * 2 + k];
			}
			let lv_i = lam * vc[j * 2 + i];
			assert_close(&format!("eigh Av=lv [{},{}]", i, j), av_i, lv_i);
		}
	}
}

// ── linalg: SVD ──────────────────────────────────────────────────────────────

#[test]
fn test_svd_reconstruction() {
	// A = [[1,2],[3,4],[5,6]] (3x2 row-major).
	// Verify A == U * diag(S) * Vt within 1e-10.
	// U is m×m col-major, S length k=min(m,n), Vt is n×n col-major.
	// Reconstruction: A[i,j] = sum_k U[i,k]*S[k]*Vt[k,j]
	//   where U[i,k] = u_v[k*m+i] and Vt[k,j] = vt_v[j*n+k].
	let a_data = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
	let a = {
		let __up = &a_data;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let (m, n) = (3usize, 2usize);
	let k = m.min(n);
	let work = GpuBuffer::alloc_bytes(gpu_svd_workspace_bytes(m, n)).unwrap();
	let info = GpuBuffer::alloc_bytes(4).unwrap();
	let u = GpuBuffer::alloc(m * m).unwrap();
	let s = GpuBuffer::alloc(k).unwrap();
	let vt = GpuBuffer::alloc(n * n).unwrap();
	gpu_svd(&a, m, n, &work, &info, &u, &s, &vt).unwrap();

	let mut u_v = [0.0f64; 9];
	let mut s_v = [0.0f64; 2];
	let mut vt_v = [0.0f64; 4];

	unsafe { u.download_async(&mut u_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { s.download_async(&mut s_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	unsafe { vt.download_async(&mut vt_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	for (i, &x) in u_v.iter().enumerate() {
		assert!(x.is_finite(), "U[{}] non-finite: {}", i, x);
	}
	for (i, &x) in s_v.iter().enumerate() {
		assert!(
			x.is_finite() && x >= 0.0,
			"S[{}] non-finite or negative: {}",
			i,
			x
		);
	}
	for (i, &x) in vt_v.iter().enumerate() {
		assert!(x.is_finite(), "Vt[{}] non-finite: {}", i, x);
	}

	for i in 0..m {
		for j in 0..n {
			let mut aij = 0.0f64;
			for kk in 0..k {
				let u_ik = u_v[kk * m + i];
				let vt_kj = vt_v[j * n + kk];
				aij += u_ik * s_v[kk] * vt_kj;
			}
			assert_close(&format!("svd A_recon[{},{}]", i, j), aij, a_data[i * n + j]);
		}
	}
}

// ── linalg: FFT ──────────────────────────────────────────────────────────────

#[test]
fn test_fft_c2c_1d_and_inverse() {
	// Input: [1+0i, 0+0i, 0+0i, 0+0i] (4 complex numbers = 8 f64)
	// Forward FFT of [1,0,0,0]: all outputs = [1,0, 1,0, 1,0, 1,0] (all ones)
	let n = 4usize;
	let mut input = [0.0f64; 8];
	input[0] = 1.0; // re part of first element
	let x = {
		let __up = &input;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};

	let fwd = GpuBuffer::alloc(2 * n).unwrap();
	gpu_fft_c2c_1d(&x, n, 1, &fwd).unwrap();
	let mut fwd_v = [0.0f64; 8];
	unsafe { fwd.download_async(&mut fwd_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	// FFT([1,0,0,0]) = [1,1,1,1] (each with imag=0)
	for i in 0..4 {
		assert_close(&format!("fft_fwd_re[{}]", i), fwd_v[2 * i], 1.0);
		assert_close(&format!("fft_fwd_im[{}]", i), fwd_v[2 * i + 1], 0.0);
	}

	// Inverse: IFFT(FFT(x)) = n * x (rocFFT is unnormalized)
	let inv = GpuBuffer::alloc(2 * n).unwrap();
	gpu_fft_c2c_1d(&fwd, n, 0, &inv).unwrap();
	let mut inv_v = [0.0f64; 8];
	unsafe { inv.download_async(&mut inv_v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	// rocFFT inverse is unnormalized → multiply by n
	let expected_re = n as f64; // first element: n*1 = 4
	assert_close("ifft_re[0]", inv_v[0], expected_re);
	assert_close("ifft_im[0]", inv_v[1], 0.0);
	// rest should be ~0
	for i in 1..4 {
		assert_close(&format!("ifft_re[{}]", i), inv_v[2 * i], 0.0);
		assert_close(&format!("ifft_im[{}]", i), inv_v[2 * i + 1], 0.0);
	}
}

#[test]
fn test_rfft_1d() {
	// Real input: [1,0,0,0] (n=4)
	// rfft output: n/2+1=3 complex values
	// RFFT([1,0,0,0]) = [1+0i, 1+0i, 1+0i]
	let n = 4usize;
	let x = {
		let __up = &[1.0f64, 0.0, 0.0, 0.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out_complex = n / 2 + 1; // = 3
	let out = GpuBuffer::alloc(2 * out_complex).unwrap();
	gpu_rfft_1d(&x, n, &out).unwrap();
	let mut v = vec![0.0f64; 2 * out_complex];
	unsafe { out.download_async(&mut v, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	for i in 0..out_complex {
		assert_close(&format!("rfft_re[{}]", i), v[2 * i], 1.0);
		assert_close(&format!("rfft_im[{}]", i), v[2 * i + 1], 0.0);
	}
}
