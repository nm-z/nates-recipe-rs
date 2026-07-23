use gpu_core::encoding::*;
use gpu_core::losses::*;
use gpu_core::math_ops::*;
use gpu_core::memory::GpuBuffer;
use std::f64::consts;
use std::ptr;

fn approx_eq(a: f64, b: f64) -> bool {
	(a - b).abs() < 1e-6 * (1.0 + b.abs())
}

fn upload(data: &[f64]) -> GpuBuffer {
	{
		let __up = data;
		let __ub = GpuBuffer::alloc(__up.len()).expect("upload");
		__ub.load(__up).expect("upload");
		__ub
	}
}

fn out_buf(n: usize) -> GpuBuffer {
	GpuBuffer::alloc(n).expect("alloc out")
}

fn download(buf: &GpuBuffer, n: usize) -> Vec<f64> {
	let mut v = vec![0.0f64; n];
	unsafe { buf.download_async(&mut v, ptr::null_mut()) }.expect("download");
	gpu_core::hip::device_synchronize().expect("download");
	v
}

// ─── math_ops ────────────────────────────────────────────────────────────────

#[test]
fn test_rsqrt() {
	let x = upload(&[4.0, 16.0, 1.0]);
	let out = out_buf(3);
	gpu_rsqrt(&x, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(
		r.iter().all(|v| v.is_finite()),
		"rsqrt: got NaN/inf: {:?}",
		r
	);
	assert!(approx_eq(r[0], 0.5), "rsqrt(4)  expected 0.5, got {}", r[0]);
	assert!(
		approx_eq(r[1], 0.25),
		"rsqrt(16) expected 0.25, got {}",
		r[1]
	);
	assert!(approx_eq(r[2], 1.0), "rsqrt(1)  expected 1.0, got {}", r[2]);
}

#[test]
fn test_reciprocal() {
	let x = upload(&[2.0, 4.0, 0.5]);
	let out = out_buf(3);
	gpu_reciprocal(&x, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(
		r.iter().all(|v| v.is_finite()),
		"reciprocal: NaN/inf: {:?}",
		r
	);
	assert!(approx_eq(r[0], 0.5), "1/2 expected 0.5, got {}", r[0]);
	assert!(approx_eq(r[1], 0.25), "1/4 expected 0.25, got {}", r[1]);
	assert!(approx_eq(r[2], 2.0), "1/0.5 expected 2.0, got {}", r[2]);
}

#[test]
fn test_max() {
	let a = upload(&[1.0, 5.0, 3.0]);
	let b = upload(&[3.0, 2.0, 3.0]);
	let out = out_buf(3);
	gpu_max(&a, &b, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 3.0), "max(1,3) expected 3, got {}", r[0]);
	assert!(approx_eq(r[1], 5.0), "max(5,2) expected 5, got {}", r[1]);
	assert!(approx_eq(r[2], 3.0), "max(3,3) expected 3, got {}", r[2]);
}

#[test]
fn test_min() {
	let a = upload(&[1.0, 5.0, 3.0]);
	let b = upload(&[3.0, 2.0, 3.0]);
	let out = out_buf(3);
	gpu_min(&a, &b, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 1.0), "min(1,3) expected 1, got {}", r[0]);
	assert!(approx_eq(r[1], 2.0), "min(5,2) expected 2, got {}", r[1]);
	assert!(approx_eq(r[2], 3.0), "min(3,3) expected 3, got {}", r[2]);
}

#[test]
fn test_sin_cos_tan() {
	let angles = [0.0f64, consts::PI / 6.0, consts::PI / 4.0];
	let x = upload(&angles);

	let sin_out = out_buf(3);
	gpu_sin(&x, 3, &sin_out).unwrap();
	let s = download(&sin_out, 3);
	for (i, &a) in angles.iter().enumerate() {
		assert!(
			approx_eq(s[i], a.sin()),
			"sin[{}]: expected {}, got {}",
			i,
			a.sin(),
			s[i]
		);
	}

	let cos_out = out_buf(3);
	gpu_cos(&x, 3, &cos_out).unwrap();
	let c = download(&cos_out, 3);
	for (i, &a) in angles.iter().enumerate() {
		assert!(
			approx_eq(c[i], a.cos()),
			"cos[{}]: expected {}, got {}",
			i,
			a.cos(),
			c[i]
		);
	}

	let tan_out = out_buf(3);
	gpu_tan(&x, 3, &tan_out).unwrap();
	let t = download(&tan_out, 3);
	for (i, &a) in angles.iter().enumerate() {
		assert!(
			approx_eq(t[i], a.tan()),
			"tan[{}]: expected {}, got {}",
			i,
			a.tan(),
			t[i]
		);
	}
}

#[test]
fn test_atan2() {
	let y = upload(&[1.0, 1.0, 0.0]);
	let x = upload(&[1.0, 0.0, 1.0]);
	let out = out_buf(3);
	gpu_atan2(&y, &x, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(
		approx_eq(r[0], consts::PI / 4.0),
		"atan2(1,1) expected pi/4, got {}",
		r[0]
	);
	assert!(
		approx_eq(r[1], consts::PI / 2.0),
		"atan2(1,0) expected pi/2, got {}",
		r[1]
	);
	assert!(approx_eq(r[2], 0.0), "atan2(0,1) expected 0, got {}", r[2]);
}

#[test]
fn test_log1p_expm1() {
	let vals = [0.0f64, 1.0, -0.5, 2.0];
	let x = upload(&vals);

	let log_out = out_buf(4);
	gpu_log1p(&x, 4, &log_out).unwrap();
	let l = download(&log_out, 4);
	for (i, &v) in vals.iter().enumerate() {
		let exp = v.ln_1p();
		if exp.is_finite() {
			assert!(
				approx_eq(l[i], exp),
				"log1p[{}]: expected {}, got {}",
				i,
				exp,
				l[i]
			);
		}
	}

	let exp_out = out_buf(4);
	gpu_expm1(&x, 4, &exp_out).unwrap();
	let e = download(&exp_out, 4);
	for (i, &v) in vals.iter().enumerate() {
		assert!(
			approx_eq(e[i], v.exp_m1()),
			"expm1[{}]: expected {}, got {}",
			i,
			v.exp_m1(),
			e[i]
		);
	}
}

#[test]
fn test_floor_ceil_round_trunc() {
	let vals = [1.2f64, -1.2, 1.8, -1.8, 0.5, -0.5];
	let x = upload(&vals);
	let n = vals.len();

	let fl_b = out_buf(n);
	gpu_floor(&x, n, &fl_b).unwrap();
	let fl = download(&fl_b, n);
	let ce_b = out_buf(n);
	gpu_ceil(&x, n, &ce_b).unwrap();
	let ce = download(&ce_b, n);
	let ro_b = out_buf(n);
	gpu_round(&x, n, &ro_b).unwrap();
	let ro = download(&ro_b, n);
	let tr_b = out_buf(n);
	gpu_trunc(&x, n, &tr_b).unwrap();
	let tr = download(&tr_b, n);

	for i in 0..n {
		assert!(
			approx_eq(fl[i], vals[i].floor()),
			"floor[{}]: expected {}, got {}",
			i,
			vals[i].floor(),
			fl[i]
		);
		assert!(
			approx_eq(ce[i], vals[i].ceil()),
			"ceil[{}]: expected {}, got {}",
			i,
			vals[i].ceil(),
			ce[i]
		);
		assert!(
			approx_eq(tr[i], vals[i].trunc()),
			"trunc[{}]: expected {}, got {}",
			i,
			vals[i].trunc(),
			tr[i]
		);
	}
	assert!(
		approx_eq(ro[4], 1.0),
		"round(0.5) expected 1.0, got {}",
		ro[4]
	);
	assert!(
		approx_eq(ro[5], -1.0),
		"round(-0.5) expected -1.0, got {}",
		ro[5]
	);
}

#[test]
fn test_fmod() {
	let a = upload(&[5.0, -5.0, 7.0]);
	let b = upload(&[3.0, 3.0, 2.0]);
	let out = out_buf(3);
	gpu_fmod(&a, &b, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 2.0), "fmod(5,3) expected 2, got {}", r[0]);
	assert!(
		approx_eq(r[1], -2.0),
		"fmod(-5,3) expected -2, got {}",
		r[1]
	);
	assert!(approx_eq(r[2], 1.0), "fmod(7,2) expected 1, got {}", r[2]);
}

#[test]
fn test_sub_scalar() {
	let x = upload(&[10.0, 20.0, 30.0]);
	let s = upload(&[5.0]);
	let out = out_buf(3);
	gpu_sub_scalar(&x, &s, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 5.0), "sub_scalar: 10-5=5, got {}", r[0]);
	assert!(approx_eq(r[1], 15.0), "sub_scalar: 20-5=15, got {}", r[1]);
	assert!(approx_eq(r[2], 25.0), "sub_scalar: 30-5=25, got {}", r[2]);
}

#[test]
fn test_div_scalar() {
	let x = upload(&[10.0, 20.0, 30.0]);
	let s = upload(&[5.0]);
	let out = out_buf(3);
	gpu_div_scalar(&x, &s, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 2.0), "div_scalar: 10/5=2, got {}", r[0]);
	assert!(approx_eq(r[1], 4.0), "div_scalar: 20/5=4, got {}", r[1]);
	assert!(approx_eq(r[2], 6.0), "div_scalar: 30/5=6, got {}", r[2]);
}

#[test]
fn test_rsub_scalar() {
	let x = upload(&[1.0, 2.0, 3.0]);
	let s = upload(&[10.0]);
	let out = out_buf(3);
	gpu_rsub_scalar(&x, &s, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 9.0), "rsub_scalar: 10-1=9, got {}", r[0]);
	assert!(approx_eq(r[1], 8.0), "rsub_scalar: 10-2=8, got {}", r[1]);
	assert!(approx_eq(r[2], 7.0), "rsub_scalar: 10-3=7, got {}", r[2]);
}

#[test]
fn test_rdiv_scalar() {
	let x = upload(&[2.0, 4.0, 5.0]);
	let s = upload(&[20.0]);
	let out = out_buf(3);
	gpu_rdiv_scalar(&x, &s, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 10.0), "rdiv_scalar: 20/2=10, got {}", r[0]);
	assert!(approx_eq(r[1], 5.0), "rdiv_scalar: 20/4=5, got {}", r[1]);
	assert!(approx_eq(r[2], 4.0), "rdiv_scalar: 20/5=4, got {}", r[2]);
}

#[test]
fn test_has_nan_true() {
	let x = upload(&[1.0, f64::NAN, 3.0]);
	let flag = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_has_nan(&x, 3, &flag).unwrap();
	let mut f = [0i32; 1];
	flag.download_i32(&mut f).unwrap();
	assert!(f[0] != 0, "has_nan with NaN should return true");
}

#[test]
fn test_has_nan_false() {
	let x = upload(&[1.0, 2.0, 3.0]);
	let flag = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_has_nan(&x, 3, &flag).unwrap();
	let mut f = [0i32; 1];
	flag.download_i32(&mut f).unwrap();
	assert!(f[0] == 0, "has_nan with no NaN should return false");
}

#[test]
fn test_isfinite_entire_true() {
	let x = upload(&[1.0, 2.0, 3.0]);
	let flag = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_isfinite_all(&x, 3, &flag).unwrap();
	let mut f = [0i32; 1];
	flag.download_i32(&mut f).unwrap();
	assert!(f[0] == 0, "isfinite_all with all finite should return true");
}

#[test]
fn test_isfinite_entire_false_inf() {
	let x = upload(&[1.0, f64::INFINITY, 3.0]);
	let flag = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_isfinite_all(&x, 3, &flag).unwrap();
	let mut f = [0i32; 1];
	flag.download_i32(&mut f).unwrap();
	assert!(f[0] != 0, "isfinite_all with Inf should return false");
}

#[test]
fn test_isfinite_entire_false_nan() {
	let x = upload(&[1.0, f64::NAN, 3.0]);
	let flag = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_isfinite_all(&x, 3, &flag).unwrap();
	let mut f = [0i32; 1];
	flag.download_i32(&mut f).unwrap();
	assert!(f[0] != 0, "isfinite_all with NaN should return false");
}

// ─── encoding ────────────────────────────────────────────────────────────────

#[test]
fn test_one_hot_layout() {
	let labels_i32: Vec<i32> = vec![0, 2];
	let labels_buf = GpuBuffer::upload_i32(&labels_i32).unwrap();
	let out = out_buf(6);
	gpu_one_hot(&labels_buf, 2, 3, &out).unwrap();
	let r = download(&out, 6);
	assert!(approx_eq(r[0], 1.0), "one_hot[0][0]=1, got {}", r[0]);
	assert!(approx_eq(r[1], 0.0), "one_hot[0][1]=0, got {}", r[1]);
	assert!(approx_eq(r[2], 0.0), "one_hot[0][2]=0, got {}", r[2]);
	assert!(approx_eq(r[3], 0.0), "one_hot[1][0]=0, got {}", r[3]);
	assert!(approx_eq(r[4], 0.0), "one_hot[1][1]=0, got {}", r[4]);
	assert!(approx_eq(r[5], 1.0), "one_hot[1][2]=1, got {}", r[5]);
}

#[test]
fn test_one_hot_single() {
	let labels_i32: Vec<i32> = vec![1];
	let labels_buf = GpuBuffer::upload_i32(&labels_i32).unwrap();
	let out = out_buf(3);
	gpu_one_hot(&labels_buf, 1, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(approx_eq(r[0], 0.0), "one_hot[0]=0, got {}", r[0]);
	assert!(approx_eq(r[1], 1.0), "one_hot[1]=1, got {}", r[1]);
	assert!(approx_eq(r[2], 0.0), "one_hot[2]=0, got {}", r[2]);
}

#[test]
fn test_bin_edges_uniform() {
	let x = upload(&[0.0, 1.0, 2.0, 3.0]);
	let out = out_buf(4); // cols*(n_bins+1) = 1*4 = 4
	gpu_bin_edges_uniform(&x, 4, 1, 3, &out).unwrap();
	let r = download(&out, 4);
	assert!(approx_eq(r[0], 0.0), "edge[0]=0, got {}", r[0]);
	assert!(approx_eq(r[1], 1.0), "edge[1]=1, got {}", r[1]);
	assert!(approx_eq(r[2], 2.0), "edge[2]=2, got {}", r[2]);
	assert!(approx_eq(r[3], 3.0), "edge[3]=3, got {}", r[3]);
}

#[test]
fn test_bin_edges_uniform_two_cols() {
	let x = upload(&[0.0, 10.0, 2.0, 20.0]);
	let out = out_buf(6);
	gpu_bin_edges_uniform(&x, 2, 2, 2, &out).unwrap();
	let r = download(&out, 6);
	assert!(approx_eq(r[0], 0.0), "col0 edge[0]=0, got {}", r[0]);
	assert!(approx_eq(r[1], 1.0), "col0 edge[1]=1, got {}", r[1]);
	assert!(approx_eq(r[2], 2.0), "col0 edge[2]=2, got {}", r[2]);
	assert!(approx_eq(r[3], 10.0), "col1 edge[0]=10, got {}", r[3]);
	assert!(approx_eq(r[4], 15.0), "col1 edge[1]=15, got {}", r[4]);
	assert!(approx_eq(r[5], 20.0), "col1 edge[2]=20, got {}", r[5]);
}

#[test]
fn test_quantize_features_basic() {
	let x = upload(&[0.0, 1.0, 2.0, 3.0]);
	let edges = upload(&[0.0, 1.0, 2.0, 3.0]);
	let out = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_quantize_features(&x, &edges, 4, 1, 3, &out).unwrap();
	let mut r = vec![0u8; 4];
	out.download_u8(&mut r).unwrap();
	assert_eq!(r[0], 0, "quantize v=0 expected bin 0, got {}", r[0]);
	assert_eq!(r[1], 1, "quantize v=1 expected bin 1, got {}", r[1]);
	assert_eq!(r[2], 2, "quantize v=2 expected bin 2, got {}", r[2]);
	assert_eq!(
		r[3], 2,
		"quantize v=3 expected bin 2 (clamped), got {}",
		r[3]
	);
}

#[test]
fn test_quantize_boundary_below_first_edge() {
	let x = upload(&[-1.0]);
	let edges = upload(&[0.0, 1.0, 2.0, 3.0]);
	let out = GpuBuffer::alloc_bytes(1).unwrap();
	gpu_quantize_features(&x, &edges, 1, 1, 3, &out).unwrap();
	let mut r = vec![0u8; 1];
	out.download_u8(&mut r).unwrap();
	assert_eq!(r[0], 0, "below-min expected bin 0, got {}", r[0]);
}

#[test]
fn test_bin_edges_quantile_sorted() {
	let x = upload(&[0.0, 1.0, 2.0, 3.0]);
	let tmp = out_buf(4); // cols*rows
	let out = out_buf(5); // cols*(n_bins+1)
	gpu_bin_edges_quantile(&x, 4, 1, 4, &tmp, &out).unwrap();
	let r = download(&out, 5);
	assert!(approx_eq(r[0], 0.0), "q_edge[0]=0, got {}", r[0]);
	assert!(approx_eq(r[1], 0.75), "q_edge[1]=0.75, got {}", r[1]);
	assert!(approx_eq(r[2], 1.5), "q_edge[2]=1.5, got {}", r[2]);
	assert!(approx_eq(r[3], 2.25), "q_edge[3]=2.25, got {}", r[3]);
	assert!(approx_eq(r[4], 3.0), "q_edge[4]=3.0, got {}", r[4]);
}

#[test]
fn test_count_distinct() {
	let data: Vec<i32> = vec![1, 1, 2, 3, 3, 3];
	let buf = GpuBuffer::upload_i32(&data).unwrap();
	let n = 6;
	let unique_vals = GpuBuffer::alloc_bytes(n * 4).unwrap();
	let run_counts = GpuBuffer::alloc_bytes(n * 4).unwrap();
	let temp = GpuBuffer::alloc_bytes(gpu_count_distinct_workspace_bytes(&buf, n).max(8)).unwrap();
	let count_out = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_count_distinct(&buf, n, &unique_vals, &run_counts, &temp, &count_out).unwrap();
	let mut c = [0i32; 1];
	count_out.download_i32(&mut c).unwrap();
	let count = c[0];
	assert_eq!(count, 3, "count_distinct expected 3, got {}", count);
}

#[test]
fn test_count_distinct_uniform() {
	let data: Vec<i32> = vec![5, 5, 5];
	let buf = GpuBuffer::upload_i32(&data).unwrap();
	let n = 3;
	let unique_vals = GpuBuffer::alloc_bytes(n * 4).unwrap();
	let run_counts = GpuBuffer::alloc_bytes(n * 4).unwrap();
	let temp = GpuBuffer::alloc_bytes(gpu_count_distinct_workspace_bytes(&buf, n).max(8)).unwrap();
	let count_out = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_count_distinct(&buf, n, &unique_vals, &run_counts, &temp, &count_out).unwrap();
	let mut c = [0i32; 1];
	count_out.download_i32(&mut c).unwrap();
	let count = c[0];
	assert_eq!(
		count, 1,
		"count_distinct all-same expected 1, got {}",
		count
	);
}

#[test]
fn test_run_length() {
	let data: Vec<i32> = vec![1, 1, 2, 3, 3, 3];
	let buf = GpuBuffer::upload_i32(&data).unwrap();
	let n = 6;
	let temp = GpuBuffer::alloc_bytes(gpu_run_length_workspace_bytes(&buf, n).max(8)).unwrap();
	let values_out = GpuBuffer::alloc_bytes(n * 4).unwrap();
	let counts_out = GpuBuffer::alloc_bytes(n * 4).unwrap();
	let n_runs_out = GpuBuffer::alloc_bytes(4).unwrap();
	gpu_run_length(&buf, n, &temp, &values_out, &counts_out, &n_runs_out).unwrap();
	let mut nr = [0i32; 1];
	n_runs_out.download_i32(&mut nr).unwrap();
	let n_runs = nr[0] as usize;
	assert_eq!(n_runs, 3, "run_length n_runs expected 3, got {}", n_runs);

	let mut vals = vec![0i32; n_runs];
	let mut counts = vec![0i32; n_runs];
	values_out.download_i32(&mut vals).unwrap();
	counts_out.download_i32(&mut counts).unwrap();

	assert_eq!(vals[0], 1, "run_val[0]=1, got {}", vals[0]);
	assert_eq!(vals[1], 2, "run_val[1]=2, got {}", vals[1]);
	assert_eq!(vals[2], 3, "run_val[2]=3, got {}", vals[2]);
	assert_eq!(counts[0], 2, "run_count[0]=2, got {}", counts[0]);
	assert_eq!(counts[1], 1, "run_count[1]=1, got {}", counts[1]);
	assert_eq!(counts[2], 3, "run_count[2]=3, got {}", counts[2]);
}

#[test]
fn test_pairwise_cosine_identical() {
	let q = upload(&[1.0, 0.0]);
	let t = upload(&[1.0, 0.0]);
	let out = out_buf(1);
	gpu_pairwise_cosine(&q, &t, 1, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "pairwise_cosine identical: NaN/inf");
	assert!(approx_eq(r[0], 1.0), "cosine([1,0],[1,0])=1, got {}", r[0]);
}

#[test]
fn test_pairwise_cosine_orthogonal() {
	let q = upload(&[1.0, 0.0]);
	let t = upload(&[0.0, 1.0]);
	let out = out_buf(1);
	gpu_pairwise_cosine(&q, &t, 1, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "pairwise_cosine orthogonal: NaN/inf");
	assert!(approx_eq(r[0], 0.0), "cosine([1,0],[0,1])=0, got {}", r[0]);
}

#[test]
fn test_pairwise_cosine_2x2() {
	let q = upload(&[1.0, 0.0, 0.0, 1.0]);
	let t = upload(&[1.0, 0.0, 0.0, 1.0]);
	let out = out_buf(4);
	gpu_pairwise_cosine(&q, &t, 2, 2, 2, &out).unwrap();
	let r = download(&out, 4);
	assert!(
		r.iter().all(|v| v.is_finite()),
		"pairwise_cosine 2x2: NaN/inf: {:?}",
		r
	);
	assert!(approx_eq(r[0], 1.0), "cosine[0,0]=1, got {}", r[0]);
	assert!(approx_eq(r[1], 0.0), "cosine[0,1]=0, got {}", r[1]);
	assert!(approx_eq(r[2], 0.0), "cosine[1,0]=0, got {}", r[2]);
	assert!(approx_eq(r[3], 1.0), "cosine[1,1]=1, got {}", r[3]);
}

#[test]
fn test_pairwise_l1_basic() {
	let q = upload(&[0.0, 0.0]);
	let t = upload(&[3.0, 4.0]);
	let out = out_buf(1);
	gpu_pairwise_l1(&q, &t, 1, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "pairwise_l1: NaN/inf");
	assert!(approx_eq(r[0], 7.0), "L1([0,0],[3,4])=7, got {}", r[0]);
}

#[test]
fn test_pairwise_l1_2x2() {
	let q = upload(&[0.0, 0.0, 1.0, 1.0]);
	let t = upload(&[0.0, 0.0, 2.0, 3.0]);
	let out = out_buf(4);
	gpu_pairwise_l1(&q, &t, 2, 2, 2, &out).unwrap();
	let r = download(&out, 4);
	assert!(
		r.iter().all(|v| v.is_finite()),
		"pairwise_l1 2x2: NaN/inf: {:?}",
		r
	);
	assert!(approx_eq(r[0], 0.0), "L1[0,0]=0, got {}", r[0]);
	assert!(approx_eq(r[1], 5.0), "L1[0,1]=5, got {}", r[1]);
	assert!(approx_eq(r[2], 2.0), "L1[1,0]=2, got {}", r[2]);
	assert!(approx_eq(r[3], 3.0), "L1[1,1]=3, got {}", r[3]);
}

#[test]
fn test_pairwise_hamming_identical() {
	let q_u8 = {
		let __u = &[1u8, 2u8];
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let t_u8 = {
		let __u = &[1u8, 2u8];
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let out = out_buf(1);
	gpu_pairwise_hamming(&q_u8, &t_u8, 1, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "hamming identical: NaN/inf");
	assert!(
		approx_eq(r[0], 0.0),
		"hamming identical expected 0, got {}",
		r[0]
	);
}

#[test]
fn test_pairwise_hamming_half_mismatch() {
	let q_u8 = {
		let __u = &[1u8, 0u8];
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let t_u8 = {
		let __u = &[0u8, 0u8];
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let out = out_buf(1);
	gpu_pairwise_hamming(&q_u8, &t_u8, 1, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "hamming half-mismatch: NaN/inf");
	assert!(
		approx_eq(r[0], 0.5),
		"hamming 1/2 mismatch expected 0.5, got {}",
		r[0]
	);
}

#[test]
fn test_pairwise_hamming_2x2() {
	let q_u8 = {
		let __u = &[0u8, 1, 1, 0];
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let t_u8 = {
		let __u = &[0u8, 1, 1, 1];
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let out = out_buf(4);
	gpu_pairwise_hamming(&q_u8, &t_u8, 2, 2, 2, &out).unwrap();
	let r = download(&out, 4);
	assert!(
		r.iter().all(|v| v.is_finite()),
		"hamming 2x2: NaN/inf: {:?}",
		r
	);
	assert!(approx_eq(r[0], 0.0), "hamming[0,0]=0, got {}", r[0]);
	assert!(approx_eq(r[1], 0.5), "hamming[0,1]=0.5, got {}", r[1]);
	assert!(approx_eq(r[2], 1.0), "hamming[1,0]=1.0, got {}", r[2]);
	assert!(approx_eq(r[3], 0.5), "hamming[1,1]=0.5, got {}", r[3]);
}

// ─── losses ──────────────────────────────────────────────────────────────────

#[test]
fn test_mae_grad_basic() {
	let pred = upload(&[2.0, 1.0, 3.0]);
	let target = upload(&[1.0, 3.0, 3.0]);
	let out = out_buf(3);
	gpu_mae_grad(&pred, &target, 3, &out).unwrap();
	let r = download(&out, 3);
	assert!(
		r.iter().all(|v| v.is_finite()),
		"mae_grad: NaN/inf: {:?}",
		r
	);
	let inv3 = 1.0 / 3.0;
	assert!(approx_eq(r[0], inv3), "mae_grad[0]=1/3, got {}", r[0]);
	assert!(approx_eq(r[1], -inv3), "mae_grad[1]=-1/3, got {}", r[1]);
	assert!(approx_eq(r[2], 0.0), "mae_grad[2]=0, got {}", r[2]);
}

#[test]
fn test_huber_grad_tiny_residual() {
	let pred = upload(&[1.5]);
	let target = upload(&[1.0]);
	let delta = upload(&[1.0]);
	let out = out_buf(1);
	gpu_huber_grad(&pred, &target, &delta, 1, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "huber_grad small: NaN/inf");
	assert!(
		approx_eq(r[0], 0.5),
		"huber_grad |d|<delta: expected 0.5, got {}",
		r[0]
	);
}

#[test]
fn test_huber_grad_large_residual() {
	let pred = upload(&[3.0]);
	let target = upload(&[0.0]);
	let delta = upload(&[1.0]);
	let out = out_buf(1);
	gpu_huber_grad(&pred, &target, &delta, 1, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "huber_grad large pos: NaN/inf");
	assert!(
		approx_eq(r[0], 1.0),
		"huber_grad large pos: expected 1.0, got {}",
		r[0]
	);
}

#[test]
fn test_huber_grad_large_negative_residual() {
	let pred = upload(&[0.0]);
	let target = upload(&[3.0]);
	let delta = upload(&[1.0]);
	let out = out_buf(1);
	gpu_huber_grad(&pred, &target, &delta, 1, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "huber_grad large neg: NaN/inf");
	assert!(
		approx_eq(r[0], -1.0),
		"huber_grad large neg: expected -1.0, got {}",
		r[0]
	);
}

#[test]
fn test_bce_with_logits_loss_and_grad() {
	let z = upload(&[0.0]);
	let y = upload(&[1.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_bce_with_logits(&z, &y, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	let expected_loss = (2.0f64).ln();
	assert!(loss[0].is_finite(), "bce loss: NaN/inf");
	assert!(grad[0].is_finite(), "bce grad: NaN/inf");
	assert!(
		approx_eq(loss[0], expected_loss),
		"bce loss(z=0,y=1): expected ln(2)={}, got {}",
		expected_loss,
		loss[0]
	);
	assert!(
		approx_eq(grad[0], -0.5),
		"bce grad(z=0,y=1): expected -0.5, got {}",
		grad[0]
	);
}

#[test]
fn test_bce_with_logits_positive_logit() {
	let z = upload(&[2.0]);
	let y = upload(&[1.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_bce_with_logits(&z, &y, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	let sig2 = 1.0 / (1.0 + (-2.0f64).exp());
	let expected_loss = ((-2.0f64).exp()).ln_1p();
	let expected_grad = sig2 - 1.0;
	assert!(loss[0].is_finite(), "bce loss z=2: NaN/inf");
	assert!(
		approx_eq(loss[0], expected_loss),
		"bce loss(z=2,y=1): expected {}, got {}",
		expected_loss,
		loss[0]
	);
	assert!(
		approx_eq(grad[0], expected_grad),
		"bce grad(z=2,y=1): expected {}, got {}",
		expected_grad,
		grad[0]
	);
}

#[test]
fn test_bce_with_logits_negative_logit() {
	let z = upload(&[-2.0]);
	let y = upload(&[0.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_bce_with_logits(&z, &y, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	let sig_neg2 = 1.0 / (1.0 + (2.0f64).exp());
	let expected_loss = ((-2.0f64).exp()).ln_1p();
	assert!(loss[0].is_finite(), "bce loss z=-2: NaN/inf");
	assert!(
		approx_eq(loss[0], expected_loss),
		"bce loss(z=-2,y=0): expected {}, got {}",
		expected_loss,
		loss[0]
	);
	assert!(
		approx_eq(grad[0], sig_neg2),
		"bce grad(z=-2,y=0): expected {}, got {}",
		sig_neg2,
		grad[0]
	);
}

#[test]
fn test_focal_loss_target1() {
	let prob = upload(&[0.9]);
	let target = upload(&[1.0]);
	let gamma = upload(&[2.0]);
	let alpha = upload(&[1.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_focal_into(&prob, &target, &gamma, &alpha, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	let p_t = 0.9f64;
	let wt = 1.0 - p_t;
	let expected_loss = -(1.0 * wt.powi(2) * p_t.ln());
	assert!(loss[0].is_finite(), "focal_loss t=1: NaN/inf in loss");
	assert!(grad[0].is_finite(), "focal_loss t=1: NaN/inf in grad");
	assert!(
		approx_eq(loss[0], expected_loss),
		"focal_loss(p=0.9,t=1): expected {}, got {}",
		expected_loss,
		loss[0]
	);
}

#[test]
fn test_focal_loss_target0() {
	let prob = upload(&[0.1]);
	let target = upload(&[0.0]);
	let gamma = upload(&[2.0]);
	let alpha = upload(&[1.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_focal_into(&prob, &target, &gamma, &alpha, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	let p_t = 0.9f64;
	let wt = 1.0 - p_t;
	let expected_loss = -(1.0 * wt.powi(2) * p_t.ln());
	assert!(loss[0].is_finite(), "focal_loss t=0: NaN/inf in loss");
	assert!(grad[0].is_finite(), "focal_loss t=0: NaN/inf in grad");
	assert!(
		approx_eq(loss[0], expected_loss),
		"focal_loss(p=0.1,t=0): expected {}, got {}",
		expected_loss,
		loss[0]
	);
}

#[test]
fn test_kl_div_loss() {
	let log_p = upload(&[(0.5f64).ln(), (0.5f64).ln()]);
	let target = upload(&[0.5, 0.5]);
	let out = out_buf(2);
	gpu_kl_div_loss(&log_p, &target, 2, &out).unwrap();
	let r = download(&out, 2);
	assert!(r.iter().all(|v| v.is_finite()), "kl_div: NaN/inf: {:?}", r);
	assert!(approx_eq(r[0], 0.0), "kl_div uniform[0]=0, got {}", r[0]);
	assert!(approx_eq(r[1], 0.0), "kl_div uniform[1]=0, got {}", r[1]);
}

#[test]
fn test_kl_div_loss_skewed() {
	let log_p = upload(&[(0.8f64).ln(), (0.2f64).ln()]);
	let target = upload(&[1.0, 0.0]);
	let out = out_buf(2);
	gpu_kl_div_loss(&log_p, &target, 2, &out).unwrap();
	let r = download(&out, 2);
	let expected0 = 1.0 * (1.0f64.ln() - (0.8f64).ln());
	assert!(r[0].is_finite(), "kl_div skewed[0]: NaN/inf");
	assert!(
		approx_eq(r[0], expected0),
		"kl_div[0]=-ln(0.8), expected {}, got {}",
		expected0,
		r[0]
	);
	assert!(approx_eq(r[1], 0.0), "kl_div[1]=0, got {}", r[1]);
}

#[test]
fn test_hinge_loss_margin_positive() {
	let scores = upload(&[0.5]);
	let labels = upload(&[1.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_hinge_loss(&scores, &labels, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	assert!(
		approx_eq(loss[0], 0.5),
		"hinge loss: expected 0.5, got {}",
		loss[0]
	);
	assert!(
		approx_eq(grad[0], -1.0),
		"hinge grad: expected -1.0, got {}",
		grad[0]
	);
}

#[test]
fn test_hinge_loss_no_margin() {
	let scores = upload(&[2.0]);
	let labels = upload(&[1.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_hinge_loss(&scores, &labels, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	assert!(
		approx_eq(loss[0], 0.0),
		"hinge no-margin loss: expected 0, got {}",
		loss[0]
	);
	assert!(
		approx_eq(grad[0], 0.0),
		"hinge no-margin grad: expected 0, got {}",
		grad[0]
	);
}

#[test]
fn test_hinge_loss_negative_label() {
	let scores = upload(&[-0.5]);
	let labels = upload(&[-1.0]);
	let loss_buf = out_buf(1);
	let grad_buf = out_buf(1);
	gpu_hinge_loss(&scores, &labels, 1, &loss_buf, &grad_buf).unwrap();
	let loss = download(&loss_buf, 1);
	let grad = download(&grad_buf, 1);
	assert!(
		approx_eq(loss[0], 0.5),
		"hinge neg label loss: expected 0.5, got {}",
		loss[0]
	);
	assert!(
		approx_eq(grad[0], 1.0),
		"hinge neg label grad: expected +1, got {}",
		grad[0]
	);
}

#[test]
fn test_cosine_embedding_loss_similar() {
	let a = upload(&[1.0, 0.0]);
	let b = upload(&[1.0, 0.0]);
	let label = upload(&[1.0]);
	let margin = upload(&[0.5]);
	let out = out_buf(1);
	gpu_cosine_embedding_loss(&a, &b, &label, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "cosine_emb similar: NaN/inf");
	assert!(
		approx_eq(r[0], 0.0),
		"cosine_emb similar: expected 0, got {}",
		r[0]
	);
}

#[test]
fn test_cosine_embedding_loss_dissimilar_no_violation() {
	let a = upload(&[1.0, 0.0]);
	let b = upload(&[-1.0, 0.0]);
	let label = upload(&[-1.0]);
	let margin = upload(&[0.0]);
	let out = out_buf(1);
	gpu_cosine_embedding_loss(&a, &b, &label, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "cosine_emb dissim no-viol: NaN/inf");
	assert!(
		approx_eq(r[0], 0.0),
		"cosine_emb dissim no-viol: expected 0, got {}",
		r[0]
	);
}

#[test]
fn test_cosine_embedding_loss_dissimilar_violation() {
	let a = upload(&[1.0, 0.0]);
	let b = upload(&[1.0, 0.0]);
	let label = upload(&[-1.0]);
	let margin = upload(&[0.5]);
	let out = out_buf(1);
	gpu_cosine_embedding_loss(&a, &b, &label, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "cosine_emb dissim violation: NaN/inf");
	assert!(
		approx_eq(r[0], 0.5),
		"cosine_emb dissim violation: expected 0.5, got {}",
		r[0]
	);
}

#[test]
fn test_triplet_loss_no_violation() {
	let anchor = upload(&[0.0, 0.0]);
	let pos = upload(&[0.0, 0.0]);
	let neg = upload(&[2.0, 0.0]);
	let margin = upload(&[0.0]);
	let out = out_buf(1);
	gpu_triplet_loss(&anchor, &pos, &neg, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "triplet no-viol: NaN/inf");
	assert!(
		approx_eq(r[0], 0.0),
		"triplet no-viol: expected 0, got {}",
		r[0]
	);
}

#[test]
fn test_triplet_loss_with_violation() {
	let anchor = upload(&[0.0, 0.0]);
	let pos = upload(&[1.0, 0.0]);
	let neg = upload(&[0.5, 0.0]);
	let margin = upload(&[0.0]);
	let out = out_buf(1);
	gpu_triplet_loss(&anchor, &pos, &neg, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "triplet violation: NaN/inf");
	assert!(
		approx_eq(r[0], 0.75),
		"triplet violation: expected 0.75, got {}",
		r[0]
	);
}

#[test]
fn test_triplet_loss_margin() {
	let anchor = upload(&[0.0, 0.0]);
	let pos = upload(&[1.0, 0.0]);
	let neg = upload(&[2.0, 0.0]);
	let margin = upload(&[1.0]);
	let out = out_buf(1);
	gpu_triplet_loss(&anchor, &pos, &neg, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "triplet margin: NaN/inf");
	assert!(
		approx_eq(r[0], 0.0),
		"triplet margin satisfied: expected 0, got {}",
		r[0]
	);
}

#[test]
fn test_contrastive_loss_similar() {
	let a = upload(&[0.0, 0.0]);
	let b = upload(&[0.0, 0.0]);
	let label = upload(&[1.0]);
	let margin = upload(&[1.0]);
	let out = out_buf(1);
	gpu_contrastive_loss(&a, &b, &label, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "contrastive similar: NaN/inf");
	assert!(
		approx_eq(r[0], 0.0),
		"contrastive similar: expected 0, got {}",
		r[0]
	);
}

#[test]
fn test_contrastive_loss_similar_nonzero() {
	let a = upload(&[1.0, 0.0]);
	let b = upload(&[3.0, 0.0]);
	let label = upload(&[1.0]);
	let margin = upload(&[1.0]);
	let out = out_buf(1);
	gpu_contrastive_loss(&a, &b, &label, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "contrastive similar nonzero: NaN/inf");
	assert!(
		approx_eq(r[0], 4.0),
		"contrastive similar: expected 4, got {}",
		r[0]
	);
}

#[test]
fn test_contrastive_loss_dissimilar_no_violation() {
	let a = upload(&[0.0, 0.0]);
	let b = upload(&[2.0, 0.0]);
	let label = upload(&[0.0]);
	let margin = upload(&[1.0]);
	let out = out_buf(1);
	gpu_contrastive_loss(&a, &b, &label, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "contrastive dissim no-viol: NaN/inf");
	assert!(
		approx_eq(r[0], 0.0),
		"contrastive dissim no-viol: expected 0, got {}",
		r[0]
	);
}

#[test]
fn test_contrastive_loss_dissimilar_violation() {
	let a = upload(&[0.0, 0.0]);
	let b = upload(&[0.5, 0.0]);
	let label = upload(&[0.0]);
	let margin = upload(&[1.0]);
	let out = out_buf(1);
	gpu_contrastive_loss(&a, &b, &label, &margin, 1, 2, &out).unwrap();
	let r = download(&out, 1);
	assert!(r[0].is_finite(), "contrastive dissim violation: NaN/inf");
	assert!(
		approx_eq(r[0], 0.25),
		"contrastive dissim violation: expected 0.25, got {}",
		r[0]
	);
}
