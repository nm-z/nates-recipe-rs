use gpu_core::memory::GpuBuffer;
use gpu_core::math_ops::*;
use gpu_core::encoding::*;
use gpu_core::losses::*;

fn approx_eq(a: f64, b: f64) -> bool {
      (a - b).abs() < 1e-6 * (1.0 + b.abs())
}

fn upload(data: &[f64]) -> GpuBuffer {
      GpuBuffer::upload(data).expect("upload")
}

fn download(buf: &GpuBuffer, n: usize) -> Vec<f64> {
      let mut v = vec![0.0f64; n];
      buf.download(&mut v).expect("download");
      v
}

// ─── math_ops ────────────────────────────────────────────────────────────────

#[test]
fn test_rsqrt() {
      let x = upload(&[4.0, 16.0, 1.0]);
      let out = gpu_rsqrt(&x, 3).unwrap();
      let r = download(&out, 3);
      assert!(r.iter().all(|v| v.is_finite()), "rsqrt: got NaN/inf: {:?}", r);
      assert!(approx_eq(r[0], 0.5),    "rsqrt(4)  expected 0.5, got {}", r[0]);
      assert!(approx_eq(r[1], 0.25),   "rsqrt(16) expected 0.25, got {}", r[1]);
      assert!(approx_eq(r[2], 1.0),    "rsqrt(1)  expected 1.0, got {}", r[2]);
}

#[test]
fn test_reciprocal() {
      let x = upload(&[2.0, 4.0, 0.5]);
      let out = gpu_reciprocal(&x, 3).unwrap();
      let r = download(&out, 3);
      assert!(r.iter().all(|v| v.is_finite()), "reciprocal: NaN/inf: {:?}", r);
      assert!(approx_eq(r[0], 0.5),  "1/2 expected 0.5, got {}", r[0]);
      assert!(approx_eq(r[1], 0.25), "1/4 expected 0.25, got {}", r[1]);
      assert!(approx_eq(r[2], 2.0),  "1/0.5 expected 2.0, got {}", r[2]);
}

#[test]
fn test_max() {
      let a = upload(&[1.0, 5.0, 3.0]);
      let b = upload(&[3.0, 2.0, 3.0]);
      let out = gpu_max(&a, &b, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], 3.0), "max(1,3) expected 3, got {}", r[0]);
      assert!(approx_eq(r[1], 5.0), "max(5,2) expected 5, got {}", r[1]);
      assert!(approx_eq(r[2], 3.0), "max(3,3) expected 3, got {}", r[2]);
}

#[test]
fn test_min() {
      let a = upload(&[1.0, 5.0, 3.0]);
      let b = upload(&[3.0, 2.0, 3.0]);
      let out = gpu_min(&a, &b, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], 1.0), "min(1,3) expected 1, got {}", r[0]);
      assert!(approx_eq(r[1], 2.0), "min(5,2) expected 2, got {}", r[1]);
      assert!(approx_eq(r[2], 3.0), "min(3,3) expected 3, got {}", r[2]);
}

#[test]
fn test_sin_cos_tan() {
      let angles = [0.0f64, std::f64::consts::PI / 6.0, std::f64::consts::PI / 4.0];
      let x = upload(&angles);

      let sin_out = gpu_sin(&x, 3).unwrap();
      let s = download(&sin_out, 3);
      for (i, &a) in angles.iter().enumerate() {
            assert!(approx_eq(s[i], a.sin()), "sin[{}]: expected {}, got {}", i, a.sin(), s[i]);
      }

      let cos_out = gpu_cos(&x, 3).unwrap();
      let c = download(&cos_out, 3);
      for (i, &a) in angles.iter().enumerate() {
            assert!(approx_eq(c[i], a.cos()), "cos[{}]: expected {}, got {}", i, a.cos(), c[i]);
      }

      let tan_out = gpu_tan(&x, 3).unwrap();
      let t = download(&tan_out, 3);
      for (i, &a) in angles.iter().enumerate() {
            assert!(approx_eq(t[i], a.tan()), "tan[{}]: expected {}, got {}", i, a.tan(), t[i]);
      }
}

#[test]
fn test_atan2() {
      // atan2(y=1, x=1) = pi/4; atan2(y=1, x=0) = pi/2
      let y = upload(&[1.0, 1.0, 0.0]);
      let x = upload(&[1.0, 0.0, 1.0]);
      let out = gpu_atan2(&y, &x, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], std::f64::consts::PI / 4.0), "atan2(1,1) expected pi/4, got {}", r[0]);
      assert!(approx_eq(r[1], std::f64::consts::PI / 2.0), "atan2(1,0) expected pi/2, got {}", r[1]);
      assert!(approx_eq(r[2], 0.0), "atan2(0,1) expected 0, got {}", r[2]);
}

#[test]
fn test_log1p_expm1() {
      let vals = [0.0f64, 1.0, -0.5, 2.0];
      let x = upload(&vals);

      let log_out = gpu_log1p(&x, 4).unwrap();
      let l = download(&log_out, 4);
      for (i, &v) in vals.iter().enumerate() {
            let exp = v.ln_1p();
            if exp.is_finite() {
                  assert!(approx_eq(l[i], exp), "log1p[{}]: expected {}, got {}", i, exp, l[i]);
            }
      }

      let exp_out = gpu_expm1(&x, 4).unwrap();
      let e = download(&exp_out, 4);
      for (i, &v) in vals.iter().enumerate() {
            assert!(approx_eq(e[i], v.exp_m1()), "expm1[{}]: expected {}, got {}", i, v.exp_m1(), e[i]);
      }
}

#[test]
fn test_floor_ceil_round_trunc() {
      let vals = [1.2f64, -1.2, 1.8, -1.8, 0.5, -0.5];
      let x = upload(&vals);
      let n = vals.len();

      let fl = download(&gpu_floor(&x, n).unwrap(), n);
      let ce = download(&gpu_ceil(&x, n).unwrap(), n);
      let ro = download(&gpu_round(&x, n).unwrap(), n);
      let tr = download(&gpu_trunc(&x, n).unwrap(), n);

      for i in 0..n {
            assert!(approx_eq(fl[i], vals[i].floor()), "floor[{}]: expected {}, got {}", i, vals[i].floor(), fl[i]);
            assert!(approx_eq(ce[i], vals[i].ceil()),  "ceil[{}]: expected {}, got {}", i, vals[i].ceil(), ce[i]);
            assert!(approx_eq(tr[i], vals[i].trunc()), "trunc[{}]: expected {}, got {}", i, vals[i].trunc(), tr[i]);
      }
      // round: C round() rounds halfway away from zero (1.5→2, -1.5→-2)
      // vals[4]=0.5 → 1.0, vals[5]=-0.5 → -1.0
      assert!(approx_eq(ro[4], 1.0),  "round(0.5) expected 1.0, got {}", ro[4]);
      assert!(approx_eq(ro[5], -1.0), "round(-0.5) expected -1.0, got {}", ro[5]);
}

#[test]
fn test_fmod() {
      let a = upload(&[5.0, -5.0, 7.0]);
      let b = upload(&[3.0,  3.0, 2.0]);
      let out = gpu_fmod(&a, &b, 3).unwrap();
      let r = download(&out, 3);
      // C fmod: 5%3=2, -5%3=-2, 7%2=1
      assert!(approx_eq(r[0],  2.0), "fmod(5,3) expected 2, got {}", r[0]);
      assert!(approx_eq(r[1], -2.0), "fmod(-5,3) expected -2, got {}", r[1]);
      assert!(approx_eq(r[2],  1.0), "fmod(7,2) expected 1, got {}", r[2]);
}

#[test]
fn test_sub_scalar() {
      let x = upload(&[10.0, 20.0, 30.0]);
      let out = gpu_sub_scalar(&x, 5.0, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], 5.0),  "sub_scalar: 10-5=5, got {}", r[0]);
      assert!(approx_eq(r[1], 15.0), "sub_scalar: 20-5=15, got {}", r[1]);
      assert!(approx_eq(r[2], 25.0), "sub_scalar: 30-5=25, got {}", r[2]);
}

#[test]
fn test_div_scalar() {
      let x = upload(&[10.0, 20.0, 30.0]);
      let out = gpu_div_scalar(&x, 5.0, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], 2.0), "div_scalar: 10/5=2, got {}", r[0]);
      assert!(approx_eq(r[1], 4.0), "div_scalar: 20/5=4, got {}", r[1]);
      assert!(approx_eq(r[2], 6.0), "div_scalar: 30/5=6, got {}", r[2]);
}

#[test]
fn test_rsub_scalar() {
      // rsub: out[i] = s - x[i]
      let x = upload(&[1.0, 2.0, 3.0]);
      let out = gpu_rsub_scalar(&x, 10.0, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], 9.0), "rsub_scalar: 10-1=9, got {}", r[0]);
      assert!(approx_eq(r[1], 8.0), "rsub_scalar: 10-2=8, got {}", r[1]);
      assert!(approx_eq(r[2], 7.0), "rsub_scalar: 10-3=7, got {}", r[2]);
}

#[test]
fn test_rdiv_scalar() {
      // rdiv: out[i] = s / x[i]
      let x = upload(&[2.0, 4.0, 5.0]);
      let out = gpu_rdiv_scalar(&x, 20.0, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], 10.0), "rdiv_scalar: 20/2=10, got {}", r[0]);
      assert!(approx_eq(r[1], 5.0),  "rdiv_scalar: 20/4=5, got {}", r[1]);
      assert!(approx_eq(r[2], 4.0),  "rdiv_scalar: 20/5=4, got {}", r[2]);
}

#[test]
fn test_has_nan_true() {
      let x = upload(&[1.0, f64::NAN, 3.0]);
      let result = gpu_has_nan(&x, 3).unwrap();
      assert!(result, "has_nan with NaN should return true");
}

#[test]
fn test_has_nan_false() {
      let x = upload(&[1.0, 2.0, 3.0]);
      let result = gpu_has_nan(&x, 3).unwrap();
      assert!(!result, "has_nan with no NaN should return false");
}

#[test]
fn test_isfinite_all_true() {
      let x = upload(&[1.0, 2.0, 3.0]);
      let result = gpu_isfinite_all(&x, 3).unwrap();
      assert!(result, "isfinite_all with all finite should return true");
}

#[test]
fn test_isfinite_all_false_inf() {
      let x = upload(&[1.0, f64::INFINITY, 3.0]);
      let result = gpu_isfinite_all(&x, 3).unwrap();
      assert!(!result, "isfinite_all with Inf should return false");
}

#[test]
fn test_isfinite_all_false_nan() {
      let x = upload(&[1.0, f64::NAN, 3.0]);
      let result = gpu_isfinite_all(&x, 3).unwrap();
      assert!(!result, "isfinite_all with NaN should return false");
}

// ─── encoding ────────────────────────────────────────────────────────────────

#[test]
fn test_one_hot_layout() {
      // labels [0, 2] → n=2, n_classes=3
      // row-major output: row 0 = [1,0,0], row 1 = [0,0,1]
      let labels_i32: Vec<i32> = vec![0, 2];
      let labels_buf = GpuBuffer::upload_i32(&labels_i32).unwrap();
      let out = gpu_one_hot(&labels_buf, 2, 3).unwrap();
      let r = download(&out, 6);
      // row 0
      assert!(approx_eq(r[0], 1.0), "one_hot[0][0]=1, got {}", r[0]);
      assert!(approx_eq(r[1], 0.0), "one_hot[0][1]=0, got {}", r[1]);
      assert!(approx_eq(r[2], 0.0), "one_hot[0][2]=0, got {}", r[2]);
      // row 1
      assert!(approx_eq(r[3], 0.0), "one_hot[1][0]=0, got {}", r[3]);
      assert!(approx_eq(r[4], 0.0), "one_hot[1][1]=0, got {}", r[4]);
      assert!(approx_eq(r[5], 1.0), "one_hot[1][2]=1, got {}", r[5]);
}

#[test]
fn test_one_hot_single() {
      let labels_i32: Vec<i32> = vec![1];
      let labels_buf = GpuBuffer::upload_i32(&labels_i32).unwrap();
      let out = gpu_one_hot(&labels_buf, 1, 3).unwrap();
      let r = download(&out, 3);
      assert!(approx_eq(r[0], 0.0), "one_hot[0]=0, got {}", r[0]);
      assert!(approx_eq(r[1], 1.0), "one_hot[1]=1, got {}", r[1]);
      assert!(approx_eq(r[2], 0.0), "one_hot[2]=0, got {}", r[2]);
}

#[test]
fn test_bin_edges_uniform() {
      // 4 rows, 1 col, values [0, 1, 2, 3] → min=0, max=3
      // n_bins=3: edges = [0, 1, 2, 3]
      let x = upload(&[0.0, 1.0, 2.0, 3.0]);
      let out = gpu_bin_edges_uniform(&x, 4, 1, 3).unwrap();
      let r = download(&out, 4); // cols*(n_bins+1) = 1*4 = 4
      assert!(approx_eq(r[0], 0.0), "edge[0]=0, got {}", r[0]);
      assert!(approx_eq(r[1], 1.0), "edge[1]=1, got {}", r[1]);
      assert!(approx_eq(r[2], 2.0), "edge[2]=2, got {}", r[2]);
      assert!(approx_eq(r[3], 3.0), "edge[3]=3, got {}", r[3]);
}

#[test]
fn test_bin_edges_uniform_two_cols() {
      // 2 rows, 2 cols: col0=[0,2], col1=[10,20]
      // row-major layout: [0, 10, 2, 20]
      // n_bins=2: col0 edges=[0,1,2], col1 edges=[10,15,20]
      // edges layout: col-major: col0*(n_bins+1) then col1*(n_bins+1)
      // i.e. edges[0..3]=[0,1,2], edges[3..6]=[10,15,20]
      let x = upload(&[0.0, 10.0, 2.0, 20.0]);
      let out = gpu_bin_edges_uniform(&x, 2, 2, 2).unwrap();
      let r = download(&out, 6);
      assert!(approx_eq(r[0], 0.0),  "col0 edge[0]=0, got {}", r[0]);
      assert!(approx_eq(r[1], 1.0),  "col0 edge[1]=1, got {}", r[1]);
      assert!(approx_eq(r[2], 2.0),  "col0 edge[2]=2, got {}", r[2]);
      assert!(approx_eq(r[3], 10.0), "col1 edge[0]=10, got {}", r[3]);
      assert!(approx_eq(r[4], 15.0), "col1 edge[1]=15, got {}", r[4]);
      assert!(approx_eq(r[5], 20.0), "col1 edge[2]=20, got {}", r[5]);
}

#[test]
fn test_quantize_features_basic() {
      // 4 rows, 1 col, values [0,1,2,3], n_bins=3, edges=[0,1,2,3]
      // quantize: binary search for largest b such that ep[b] <= v
      // v=0: ep[0]=0 ≤ 0, ep[1]=1 > 0 → bin=0
      // v=1: ep[0]=0, ep[1]=1 ≤ 1, ep[2]=2 > 1 → bin=1
      // v=2: ep[2]=2 ≤ 2, ep[3]=3 > 2 → bin=2 (clamped to n_bins-1=2)
      // v=3: ep[3]=3 ≤ 3 → lo=3, clamped to n_bins-1=2
      let x = upload(&[0.0, 1.0, 2.0, 3.0]);
      let edges = upload(&[0.0, 1.0, 2.0, 3.0]);
      let out = gpu_quantize_features(&x, &edges, 4, 1, 3).unwrap();
      let mut r = vec![0u8; 4];
      out.download_u8(&mut r).unwrap();
      assert_eq!(r[0], 0, "quantize v=0 expected bin 0, got {}", r[0]);
      assert_eq!(r[1], 1, "quantize v=1 expected bin 1, got {}", r[1]);
      assert_eq!(r[2], 2, "quantize v=2 expected bin 2, got {}", r[2]);
      assert_eq!(r[3], 2, "quantize v=3 expected bin 2 (clamped), got {}", r[3]);
}

#[test]
fn test_quantize_boundary_below_first_edge() {
      // v=-1 is below min edge 0; binary search gives lo=0 (ep[0]=0 > -1 immediately → lo stays 0)
      // so bin 0 is expected
      let x = upload(&[-1.0]);
      let edges = upload(&[0.0, 1.0, 2.0, 3.0]);
      let out = gpu_quantize_features(&x, &edges, 1, 1, 3).unwrap();
      let mut r = vec![0u8; 1];
      out.download_u8(&mut r).unwrap();
      assert_eq!(r[0], 0, "below-min expected bin 0, got {}", r[0]);
}

#[test]
fn test_bin_edges_quantile_sorted() {
      // 4 rows, 1 col, sorted values [0,1,2,3] → n_bins=4
      // edges: [0, 0.75, 1.5, 2.25, 3] (quantile positions at 0, 1/4, 2/4, 3/4, 1)
      // Actually quantile formula: b * (rows-1) / n_bins
      // b=1: 1*3/4=0.75 → lo=0, hi=1, frac=0.75 → 0*0.25 + 1*0.75 = 0.75
      // b=2: 2*3/4=1.5 → 0.5*(1+2) = 1.5
      // b=3: 3*3/4=2.25 → 0.75*(2+3) ... lo=2, frac=0.25 → 2+0.25 = 2.25
      let x = upload(&[0.0, 1.0, 2.0, 3.0]);
      let out = gpu_bin_edges_quantile(&x, 4, 1, 4).unwrap();
      let r = download(&out, 5);
      assert!(approx_eq(r[0], 0.0),  "q_edge[0]=0, got {}", r[0]);
      assert!(approx_eq(r[1], 0.75), "q_edge[1]=0.75, got {}", r[1]);
      assert!(approx_eq(r[2], 1.5),  "q_edge[2]=1.5, got {}", r[2]);
      assert!(approx_eq(r[3], 2.25), "q_edge[3]=2.25, got {}", r[3]);
      assert!(approx_eq(r[4], 3.0),  "q_edge[4]=3.0, got {}", r[4]);
}

#[test]
fn test_count_distinct() {
      // sorted: [1, 1, 2, 3, 3, 3] → 3 distinct
      let data: Vec<i32> = vec![1, 1, 2, 3, 3, 3];
      let buf = GpuBuffer::upload_i32(&data).unwrap();
      let count = gpu_count_distinct(&buf, 6).unwrap();
      assert_eq!(count, 3, "count_distinct expected 3, got {}", count);
}

#[test]
fn test_count_distinct_all_same() {
      let data: Vec<i32> = vec![5, 5, 5];
      let buf = GpuBuffer::upload_i32(&data).unwrap();
      let count = gpu_count_distinct(&buf, 3).unwrap();
      assert_eq!(count, 1, "count_distinct all-same expected 1, got {}", count);
}

#[test]
fn test_run_length() {
      // sorted: [1, 1, 2, 3, 3, 3]
      // expected: values=[1,2,3], counts=[2,1,3], n_runs=3
      let data: Vec<i32> = vec![1, 1, 2, 3, 3, 3];
      let buf = GpuBuffer::upload_i32(&data).unwrap();
      let (vals_buf, counts_buf, n_runs) = gpu_run_length(&buf, 6).unwrap();
      assert_eq!(n_runs, 3, "run_length n_runs expected 3, got {}", n_runs);

      let mut vals = vec![0i32; n_runs];
      let mut counts = vec![0i32; n_runs];
      vals_buf.download_i32(&mut vals).unwrap();
      counts_buf.download_i32(&mut counts).unwrap();

      assert_eq!(vals[0], 1, "run_val[0]=1, got {}", vals[0]);
      assert_eq!(vals[1], 2, "run_val[1]=2, got {}", vals[1]);
      assert_eq!(vals[2], 3, "run_val[2]=3, got {}", vals[2]);
      assert_eq!(counts[0], 2, "run_count[0]=2, got {}", counts[0]);
      assert_eq!(counts[1], 1, "run_count[1]=1, got {}", counts[1]);
      assert_eq!(counts[2], 3, "run_count[2]=3, got {}", counts[2]);
}

#[test]
fn test_pairwise_cosine_identical() {
      // Two identical vectors [1,0] — cosine similarity = 1
      let q = upload(&[1.0, 0.0]);
      let t = upload(&[1.0, 0.0]);
      let out = gpu_pairwise_cosine(&q, &t, 1, 1, 2).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "pairwise_cosine identical: NaN/inf");
      assert!(approx_eq(r[0], 1.0), "cosine([1,0],[1,0])=1, got {}", r[0]);
}

#[test]
fn test_pairwise_cosine_orthogonal() {
      // [1,0] vs [0,1] → cosine = 0
      let q = upload(&[1.0, 0.0]);
      let t = upload(&[0.0, 1.0]);
      let out = gpu_pairwise_cosine(&q, &t, 1, 1, 2).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "pairwise_cosine orthogonal: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "cosine([1,0],[0,1])=0, got {}", r[0]);
}

#[test]
fn test_pairwise_cosine_2x2() {
      // 2 queries: [1,0],[0,1]; 2 trains: [1,0],[0,1]; dim=2
      // out[nq*nt]: out[0]=cos([1,0],[1,0])=1, out[1]=cos([1,0],[0,1])=0
      //             out[2]=cos([0,1],[1,0])=0, out[3]=cos([0,1],[0,1])=1
      let q = upload(&[1.0, 0.0, 0.0, 1.0]);
      let t = upload(&[1.0, 0.0, 0.0, 1.0]);
      let out = gpu_pairwise_cosine(&q, &t, 2, 2, 2).unwrap();
      let r = download(&out, 4);
      assert!(r.iter().all(|v| v.is_finite()), "pairwise_cosine 2x2: NaN/inf: {:?}", r);
      assert!(approx_eq(r[0], 1.0), "cosine[0,0]=1, got {}", r[0]);
      assert!(approx_eq(r[1], 0.0), "cosine[0,1]=0, got {}", r[1]);
      assert!(approx_eq(r[2], 0.0), "cosine[1,0]=0, got {}", r[2]);
      assert!(approx_eq(r[3], 1.0), "cosine[1,1]=1, got {}", r[3]);
}

#[test]
fn test_pairwise_l1_basic() {
      // q=[0,0], t=[3,4] → L1 = |0-3| + |0-4| = 7
      let q = upload(&[0.0, 0.0]);
      let t = upload(&[3.0, 4.0]);
      let out = gpu_pairwise_l1(&q, &t, 1, 1, 2).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "pairwise_l1: NaN/inf");
      assert!(approx_eq(r[0], 7.0), "L1([0,0],[3,4])=7, got {}", r[0]);
}

#[test]
fn test_pairwise_l1_2x2() {
      // q: [[0,0],[1,1]], t: [[0,0],[2,3]]; dim=2
      // out[0]=|0-0|+|0-0|=0, out[1]=|0-2|+|0-3|=5
      // out[2]=|1-0|+|1-0|=2, out[3]=|1-2|+|1-3|=3
      let q = upload(&[0.0, 0.0, 1.0, 1.0]);
      let t = upload(&[0.0, 0.0, 2.0, 3.0]);
      let out = gpu_pairwise_l1(&q, &t, 2, 2, 2).unwrap();
      let r = download(&out, 4);
      assert!(r.iter().all(|v| v.is_finite()), "pairwise_l1 2x2: NaN/inf: {:?}", r);
      assert!(approx_eq(r[0], 0.0), "L1[0,0]=0, got {}", r[0]);
      assert!(approx_eq(r[1], 5.0), "L1[0,1]=5, got {}", r[1]);
      assert!(approx_eq(r[2], 2.0), "L1[1,0]=2, got {}", r[2]);
      assert!(approx_eq(r[3], 3.0), "L1[1,1]=3, got {}", r[3]);
}

#[test]
fn test_pairwise_hamming_identical() {
      // [1,2] vs [1,2] → 0/2 = 0.0
      let q_u8 = GpuBuffer::upload_u8(&[1u8, 2u8]).unwrap();
      let t_u8 = GpuBuffer::upload_u8(&[1u8, 2u8]).unwrap();
      let out = gpu_pairwise_hamming(&q_u8, &t_u8, 1, 1, 2).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "hamming identical: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "hamming identical expected 0, got {}", r[0]);
}

#[test]
fn test_pairwise_hamming_half_mismatch() {
      // [1,0] vs [0,0] → 1/2 = 0.5
      let q_u8 = GpuBuffer::upload_u8(&[1u8, 0u8]).unwrap();
      let t_u8 = GpuBuffer::upload_u8(&[0u8, 0u8]).unwrap();
      let out = gpu_pairwise_hamming(&q_u8, &t_u8, 1, 1, 2).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "hamming half-mismatch: NaN/inf");
      assert!(approx_eq(r[0], 0.5), "hamming 1/2 mismatch expected 0.5, got {}", r[0]);
}

#[test]
fn test_pairwise_hamming_2x2() {
      // q: [[0,1],[1,0]], t: [[0,1],[1,1]]; dim=2
      // out[0]=hamming([0,1],[0,1])=0/2=0
      // out[1]=hamming([0,1],[1,1])=1/2=0.5
      // out[2]=hamming([1,0],[0,1])=2/2=1.0
      // out[3]=hamming([1,0],[1,1])=1/2=0.5
      let q_u8 = GpuBuffer::upload_u8(&[0u8, 1, 1, 0]).unwrap();
      let t_u8 = GpuBuffer::upload_u8(&[0u8, 1, 1, 1]).unwrap();
      let out = gpu_pairwise_hamming(&q_u8, &t_u8, 2, 2, 2).unwrap();
      let r = download(&out, 4);
      assert!(r.iter().all(|v| v.is_finite()), "hamming 2x2: NaN/inf: {:?}", r);
      assert!(approx_eq(r[0], 0.0), "hamming[0,0]=0, got {}", r[0]);
      assert!(approx_eq(r[1], 0.5), "hamming[0,1]=0.5, got {}", r[1]);
      assert!(approx_eq(r[2], 1.0), "hamming[1,0]=1.0, got {}", r[2]);
      assert!(approx_eq(r[3], 0.5), "hamming[1,1]=0.5, got {}", r[3]);
}

// ─── losses ──────────────────────────────────────────────────────────────────

#[test]
fn test_mae_grad_basic() {
      // pred=[2,1,3], target=[1,3,3] → d=[1,-2,0]
      // grad = sign(d)/n → [1/3, -1/3, 0/3]
      let pred   = upload(&[2.0, 1.0, 3.0]);
      let target = upload(&[1.0, 3.0, 3.0]);
      let out = gpu_mae_grad(&pred, &target, 3).unwrap();
      let r = download(&out, 3);
      assert!(r.iter().all(|v| v.is_finite()), "mae_grad: NaN/inf: {:?}", r);
      let inv3 = 1.0 / 3.0;
      assert!(approx_eq(r[0],  inv3), "mae_grad[0]=1/3, got {}", r[0]);
      assert!(approx_eq(r[1], -inv3), "mae_grad[1]=-1/3, got {}", r[1]);
      assert!(approx_eq(r[2],  0.0),  "mae_grad[2]=0, got {}", r[2]);
}

#[test]
fn test_huber_grad_small_residual() {
      // |d| <= delta=1.0: grad = d/n
      // pred=[1.5], target=[1.0], d=0.5 <= 1.0 → grad = 0.5/1
      let pred   = upload(&[1.5]);
      let target = upload(&[1.0]);
      let out = gpu_huber_grad(&pred, &target, 1.0, 1).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "huber_grad small: NaN/inf");
      assert!(approx_eq(r[0], 0.5), "huber_grad |d|<delta: expected 0.5, got {}", r[0]);
}

#[test]
fn test_huber_grad_large_residual() {
      // |d| > delta=1.0: grad = delta * sign(d) / n
      // pred=[3.0], target=[0.0], d=3.0 > 1.0 → grad = 1.0 * 1 / 1 = 1.0
      let pred   = upload(&[3.0]);
      let target = upload(&[0.0]);
      let out = gpu_huber_grad(&pred, &target, 1.0, 1).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "huber_grad large pos: NaN/inf");
      assert!(approx_eq(r[0], 1.0), "huber_grad large pos: expected 1.0, got {}", r[0]);
}

#[test]
fn test_huber_grad_large_negative_residual() {
      // d=-3.0 < -1.0 → grad = -delta/n = -1.0
      let pred   = upload(&[0.0]);
      let target = upload(&[3.0]);
      let out = gpu_huber_grad(&pred, &target, 1.0, 1).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "huber_grad large neg: NaN/inf");
      assert!(approx_eq(r[0], -1.0), "huber_grad large neg: expected -1.0, got {}", r[0]);
}

#[test]
fn test_bce_with_logits_loss_and_grad() {
      // stable BCE: loss = max(z,0) - z*y + log1p(exp(-|z|))
      // grad = sigmoid(z) - y
      // Test z=0, y=1: loss = 0 - 0 + log1p(1) = ln(2) ≈ 0.6931; grad = 0.5 - 1 = -0.5
      let z = upload(&[0.0]);
      let y = upload(&[1.0]);
      let (loss_buf, grad_buf) = gpu_bce_with_logits(&z, &y, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      let expected_loss = (2.0f64).ln();
      assert!(loss[0].is_finite(), "bce loss: NaN/inf");
      assert!(grad[0].is_finite(), "bce grad: NaN/inf");
      assert!(approx_eq(loss[0], expected_loss), "bce loss(z=0,y=1): expected ln(2)={}, got {}", expected_loss, loss[0]);
      assert!(approx_eq(grad[0], -0.5), "bce grad(z=0,y=1): expected -0.5, got {}", grad[0]);
}

#[test]
fn test_bce_with_logits_positive_logit() {
      // z=2.0, y=1.0
      // loss = 2 - 2*1 + log1p(exp(-2)) = log1p(exp(-2)) ≈ 0.1269
      // grad = sigmoid(2) - 1 = 1/(1+exp(-2)) - 1 ≈ 0.8808 - 1 = -0.1192
      let z = upload(&[2.0]);
      let y = upload(&[1.0]);
      let (loss_buf, grad_buf) = gpu_bce_with_logits(&z, &y, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      let sig2 = 1.0 / (1.0 + (-2.0f64).exp());
      let expected_loss = ((-2.0f64).exp()).ln_1p();
      let expected_grad = sig2 - 1.0;
      assert!(loss[0].is_finite(), "bce loss z=2: NaN/inf");
      assert!(approx_eq(loss[0], expected_loss), "bce loss(z=2,y=1): expected {}, got {}", expected_loss, loss[0]);
      assert!(approx_eq(grad[0], expected_grad), "bce grad(z=2,y=1): expected {}, got {}", expected_grad, grad[0]);
}

#[test]
fn test_bce_with_logits_negative_logit() {
      // z=-2.0, y=0.0
      // loss = max(-2,0) - (-2)*0 + log1p(exp(-2)) = log1p(exp(-2))
      // but wait: loss = max(z,0) - z*y + log1p(exp(-|z|))
      // = max(-2,0) - (-2)*0 + log1p(exp(-|-2|))
      // = 0 - 0 + log1p(exp(-2)) = log1p(exp(-2))
      // grad = sigmoid(-2) - 0 = 1/(1+exp(2)) ≈ 0.1192
      let z = upload(&[-2.0]);
      let y = upload(&[0.0]);
      let (loss_buf, grad_buf) = gpu_bce_with_logits(&z, &y, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      let sig_neg2 = 1.0 / (1.0 + (2.0f64).exp());
      let expected_loss = ((-2.0f64).exp()).ln_1p();
      assert!(loss[0].is_finite(), "bce loss z=-2: NaN/inf");
      assert!(approx_eq(loss[0], expected_loss), "bce loss(z=-2,y=0): expected {}, got {}", expected_loss, loss[0]);
      assert!(approx_eq(grad[0], sig_neg2), "bce grad(z=-2,y=0): expected {}, got {}", sig_neg2, grad[0]);
}

#[test]
fn test_focal_loss_target1() {
      // prob=0.9, target=1, gamma=2, alpha=1
      // p_t = 0.9, log_pt = ln(0.9), wt = 0.1
      // focal_wt = 1 * 0.1^2 = 0.01
      // loss = -0.01 * ln(0.9)
      let prob   = upload(&[0.9]);
      let target = upload(&[1.0]);
      let (loss_buf, grad_buf) = gpu_focal_loss(&prob, &target, 2.0, 1.0, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      let p_t = 0.9f64;
      let wt = 1.0 - p_t;
      let expected_loss = -(1.0 * wt.powi(2) * p_t.ln());
      assert!(loss[0].is_finite(), "focal_loss t=1: NaN/inf in loss");
      assert!(grad[0].is_finite(), "focal_loss t=1: NaN/inf in grad");
      assert!(approx_eq(loss[0], expected_loss), "focal_loss(p=0.9,t=1): expected {}, got {}", expected_loss, loss[0]);
}

#[test]
fn test_focal_loss_target0() {
      // prob=0.1, target=0, gamma=2, alpha=1
      // p_t = 1-0.1 = 0.9, loss = -(1*(1-0.9)^2 * ln(0.9)) = -0.01*ln(0.9)
      let prob   = upload(&[0.1]);
      let target = upload(&[0.0]);
      let (loss_buf, grad_buf) = gpu_focal_loss(&prob, &target, 2.0, 1.0, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      let p_t = 0.9f64;
      let wt = 1.0 - p_t;
      let expected_loss = -(1.0 * wt.powi(2) * p_t.ln());
      assert!(loss[0].is_finite(), "focal_loss t=0: NaN/inf in loss");
      assert!(grad[0].is_finite(), "focal_loss t=0: NaN/inf in grad");
      assert!(approx_eq(loss[0], expected_loss), "focal_loss(p=0.1,t=0): expected {}, got {}", expected_loss, loss[0]);
}

#[test]
fn test_kl_div_loss() {
      // KL: out[i] = target[i] * (log(target[i]) - log_p[i])
      // target=[0.5, 0.5], log_p=[ln(0.5), ln(0.5)] → out=[0,0] (uniform matches uniform)
      let log_p  = upload(&[(0.5f64).ln(), (0.5f64).ln()]);
      let target = upload(&[0.5, 0.5]);
      let out = gpu_kl_div_loss(&log_p, &target, 2).unwrap();
      let r = download(&out, 2);
      assert!(r.iter().all(|v| v.is_finite()), "kl_div: NaN/inf: {:?}", r);
      assert!(approx_eq(r[0], 0.0), "kl_div uniform[0]=0, got {}", r[0]);
      assert!(approx_eq(r[1], 0.0), "kl_div uniform[1]=0, got {}", r[1]);
}

#[test]
fn test_kl_div_loss_skewed() {
      // target=[1.0, 0.0], log_p=[ln(0.8), ln(0.2)]
      // out[0] = 1.0 * (ln(1.0) - ln(0.8)) = -ln(0.8)
      // out[1] = 0 → 0
      let log_p  = upload(&[(0.8f64).ln(), (0.2f64).ln()]);
      let target = upload(&[1.0, 0.0]);
      let out = gpu_kl_div_loss(&log_p, &target, 2).unwrap();
      let r = download(&out, 2);
      let expected0 = 1.0 * (1.0f64.ln() - (0.8f64).ln());
      assert!(r[0].is_finite(), "kl_div skewed[0]: NaN/inf");
      assert!(approx_eq(r[0], expected0), "kl_div[0]=-ln(0.8), expected {}, got {}", expected0, r[0]);
      assert!(approx_eq(r[1], 0.0), "kl_div[1]=0, got {}", r[1]);
}

#[test]
fn test_hinge_loss_margin_positive() {
      // score=0.5, label=+1 → margin = 1 - 0.5 = 0.5 > 0 → loss=0.5, grad=-1
      let scores = upload(&[0.5]);
      let labels = upload(&[1.0]);
      let (loss_buf, grad_buf) = gpu_hinge_loss(&scores, &labels, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      assert!(approx_eq(loss[0], 0.5), "hinge loss: expected 0.5, got {}", loss[0]);
      assert!(approx_eq(grad[0], -1.0), "hinge grad: expected -1.0, got {}", grad[0]);
}

#[test]
fn test_hinge_loss_no_margin() {
      // score=2.0, label=+1 → margin = 1 - 2 = -1 < 0 → loss=0, grad=0
      let scores = upload(&[2.0]);
      let labels = upload(&[1.0]);
      let (loss_buf, grad_buf) = gpu_hinge_loss(&scores, &labels, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      assert!(approx_eq(loss[0], 0.0), "hinge no-margin loss: expected 0, got {}", loss[0]);
      assert!(approx_eq(grad[0], 0.0), "hinge no-margin grad: expected 0, got {}", grad[0]);
}

#[test]
fn test_hinge_loss_negative_label() {
      // score=-0.5, label=-1 → margin = 1 - (-1)(-0.5) = 1 - 0.5 = 0.5 > 0 → loss=0.5, grad=+1
      let scores = upload(&[-0.5]);
      let labels = upload(&[-1.0]);
      let (loss_buf, grad_buf) = gpu_hinge_loss(&scores, &labels, 1).unwrap();
      let loss = download(&loss_buf, 1);
      let grad = download(&grad_buf, 1);
      assert!(approx_eq(loss[0], 0.5), "hinge neg label loss: expected 0.5, got {}", loss[0]);
      assert!(approx_eq(grad[0], 1.0), "hinge neg label grad: expected +1, got {}", grad[0]);
}

#[test]
fn test_cosine_embedding_loss_similar() {
      // a=[1,0], b=[1,0], label=+1, margin=0.5
      // cos_sim=1.0 → loss = 1 - 1 = 0
      let a = upload(&[1.0, 0.0]);
      let b = upload(&[1.0, 0.0]);
      let label = upload(&[1.0]);
      let out = gpu_cosine_embedding_loss(&a, &b, &label, 1, 2, 0.5).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "cosine_emb similar: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "cosine_emb similar: expected 0, got {}", r[0]);
}

#[test]
fn test_cosine_embedding_loss_dissimilar_no_violation() {
      // a=[1,0], b=[-1,0], label=-1, margin=0.0
      // cos_sim=-1.0 → v = cos_sim - margin = -1 - 0 = -1 < 0 → loss = 0
      let a = upload(&[1.0, 0.0]);
      let b = upload(&[-1.0, 0.0]);
      let label = upload(&[-1.0]);
      let out = gpu_cosine_embedding_loss(&a, &b, &label, 1, 2, 0.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "cosine_emb dissim no-viol: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "cosine_emb dissim no-viol: expected 0, got {}", r[0]);
}

#[test]
fn test_cosine_embedding_loss_dissimilar_violation() {
      // a=[1,0], b=[1,0], label=-1, margin=0.5
      // cos_sim=1.0 → v = 1.0 - 0.5 = 0.5 > 0 → loss = 0.5
      let a = upload(&[1.0, 0.0]);
      let b = upload(&[1.0, 0.0]);
      let label = upload(&[-1.0]);
      let out = gpu_cosine_embedding_loss(&a, &b, &label, 1, 2, 0.5).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "cosine_emb dissim violation: NaN/inf");
      assert!(approx_eq(r[0], 0.5), "cosine_emb dissim violation: expected 0.5, got {}", r[0]);
}

#[test]
fn test_triplet_loss_no_violation() {
      // anchor=[0,0], pos=[1,0], neg=[0,0] (same as anchor)
      // d_ap = ||anchor-pos||^2 = 1, d_an = ||anchor-neg||^2 = 0
      // v = 1 - 0 + margin=0.0 = 1.0 > 0 → loss=1.0
      // (test with neg far away so no violation)
      // anchor=[0,0], pos=[0,0], neg=[2,0] → d_ap=0, d_an=4, v=0-4+0=-4 → loss=0
      let anchor = upload(&[0.0, 0.0]);
      let pos    = upload(&[0.0, 0.0]);
      let neg    = upload(&[2.0, 0.0]);
      let out = gpu_triplet_loss(&anchor, &pos, &neg, 1, 2, 0.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "triplet no-viol: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "triplet no-viol: expected 0, got {}", r[0]);
}

#[test]
fn test_triplet_loss_with_violation() {
      // anchor=[0,0], pos=[1,0], neg=[0.5,0], margin=0
      // d_ap = 1, d_an = 0.25 → v = 1 - 0.25 + 0 = 0.75 → loss=0.75
      let anchor = upload(&[0.0, 0.0]);
      let pos    = upload(&[1.0, 0.0]);
      let neg    = upload(&[0.5, 0.0]);
      let out = gpu_triplet_loss(&anchor, &pos, &neg, 1, 2, 0.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "triplet violation: NaN/inf");
      assert!(approx_eq(r[0], 0.75), "triplet violation: expected 0.75, got {}", r[0]);
}

#[test]
fn test_triplet_loss_margin() {
      // anchor=[0,0], pos=[1,0], neg=[2,0], margin=1.0
      // d_ap=1, d_an=4 → v=1-4+1=-2 → loss=0
      let anchor = upload(&[0.0, 0.0]);
      let pos    = upload(&[1.0, 0.0]);
      let neg    = upload(&[2.0, 0.0]);
      let out = gpu_triplet_loss(&anchor, &pos, &neg, 1, 2, 1.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "triplet margin: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "triplet margin satisfied: expected 0, got {}", r[0]);
}

#[test]
fn test_contrastive_loss_similar() {
      // a=[0,0], b=[0,0], label=1 (similar), margin=1.0
      // dist2=0 → loss = 1*0 + 0 = 0
      let a = upload(&[0.0, 0.0]);
      let b = upload(&[0.0, 0.0]);
      let label = upload(&[1.0]);
      let out = gpu_contrastive_loss(&a, &b, &label, 1, 2, 1.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "contrastive similar: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "contrastive similar: expected 0, got {}", r[0]);
}

#[test]
fn test_contrastive_loss_similar_nonzero() {
      // a=[1,0], b=[3,0], label=1 (similar)
      // dist2 = (1-3)^2 = 4 → loss = 1*4 = 4
      let a = upload(&[1.0, 0.0]);
      let b = upload(&[3.0, 0.0]);
      let label = upload(&[1.0]);
      let out = gpu_contrastive_loss(&a, &b, &label, 1, 2, 1.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "contrastive similar nonzero: NaN/inf");
      assert!(approx_eq(r[0], 4.0), "contrastive similar: expected 4, got {}", r[0]);
}

#[test]
fn test_contrastive_loss_dissimilar_no_violation() {
      // a=[0,0], b=[2,0], label=0 (dissimilar), margin=1.0
      // dist = 2, margin-dist = 1-2 = -1 < 0 → neg_term=0 → loss=0
      let a = upload(&[0.0, 0.0]);
      let b = upload(&[2.0, 0.0]);
      let label = upload(&[0.0]);
      let out = gpu_contrastive_loss(&a, &b, &label, 1, 2, 1.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "contrastive dissim no-viol: NaN/inf");
      assert!(approx_eq(r[0], 0.0), "contrastive dissim no-viol: expected 0, got {}", r[0]);
}

#[test]
fn test_contrastive_loss_dissimilar_violation() {
      // a=[0,0], b=[0.5,0], label=0 (dissimilar), margin=1.0
      // dist=0.5, dist2=0.25 → neg_margin=1-0.5=0.5 → neg_term=0.25 → loss=0*(0.25)+(1-0)*0.25=0.25
      let a = upload(&[0.0, 0.0]);
      let b = upload(&[0.5, 0.0]);
      let label = upload(&[0.0]);
      let out = gpu_contrastive_loss(&a, &b, &label, 1, 2, 1.0).unwrap();
      let r = download(&out, 1);
      assert!(r[0].is_finite(), "contrastive dissim violation: NaN/inf");
      assert!(approx_eq(r[0], 0.25), "contrastive dissim violation: expected 0.25, got {}", r[0]);
}
