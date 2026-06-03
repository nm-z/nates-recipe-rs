// Lead smoke test: trivially-verifiable functions across domains, executed on the GPU.
use gpu_core::memory::GpuBuffer;
use gpu_core::reductions::{gpu_sum_all, gpu_sort, gpu_cumsum_rows};
use gpu_core::linalg::gpu_ddot;
use gpu_core::math_ops::{gpu_rsqrt, gpu_reciprocal, gpu_max};
use gpu_core::encoding::gpu_one_hot;
use gpu_core::rl::gpu_discounted_returns;

fn approx(a: &[f64], b: &[f64]) {
      assert_eq!(a.len(), b.len(), "len mismatch: {:?} vs {:?}", a, b);
      for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert!(x.is_finite(), "non-finite at {}: {:?}", i, a);
            assert!((x - y).abs() < 1e-6 * (1.0 + y.abs()), "idx {}: got {} want {} (full {:?})", i, x, y, a);
      }
}

#[test]
fn sum_all() {
      let x = GpuBuffer::upload(&[1.0, 2.0, 3.0, 4.0]).unwrap();
      let s = gpu_sum_all(&x, 4).unwrap();
      eprintln!("sum_all = {}", s);
      assert!((s - 10.0).abs() < 1e-9, "sum_all got {}", s);
}

#[test]
fn ddot() {
      let a = GpuBuffer::upload(&[1.0, 2.0, 3.0]).unwrap();
      let b = GpuBuffer::upload(&[1.0, 1.0, 1.0]).unwrap();
      let d = gpu_ddot(&a, &b, 3).unwrap();
      eprintln!("ddot = {}", d);
      assert!((d - 6.0).abs() < 1e-9, "ddot got {}", d);
}

#[test]
fn rsqrt_recip_max() {
      let x = GpuBuffer::upload(&[4.0, 16.0]).unwrap();
      let r = gpu_rsqrt(&x, 2).unwrap();
      let mut out = [0.0; 2]; r.download(&mut out).unwrap();
      eprintln!("rsqrt = {:?}", out); approx(&out, &[0.5, 0.25]);

      let y = GpuBuffer::upload(&[2.0, 4.0]).unwrap();
      let rc = gpu_reciprocal(&y, 2).unwrap();
      rc.download(&mut out).unwrap();
      eprintln!("recip = {:?}", out); approx(&out, &[0.5, 0.25]);

      let a = GpuBuffer::upload(&[1.0, 5.0]).unwrap();
      let b = GpuBuffer::upload(&[3.0, 2.0]).unwrap();
      let m = gpu_max(&a, &b, 2).unwrap();
      m.download(&mut out).unwrap();
      eprintln!("max = {:?}", out); approx(&out, &[3.0, 5.0]);
}

#[test]
fn sort_pow2() {
      let x = GpuBuffer::upload(&[3.0, 1.0, 2.0, 4.0]).unwrap();
      let s = gpu_sort(&x, 4).unwrap();
      let mut out = [0.0; 4]; s.download(&mut out).unwrap();
      eprintln!("sort(n=4) = {:?}", out); approx(&out, &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn sort_non_pow2() {
      let x = GpuBuffer::upload(&[3.0, 1.0, 2.0]).unwrap();
      let s = gpu_sort(&x, 3).unwrap();
      let mut out = [0.0; 3]; s.download(&mut out).unwrap();
      eprintln!("sort(n=3) = {:?}", out); approx(&out, &[1.0, 2.0, 3.0]);
}

#[test]
fn cumsum_rows() {
      let x = GpuBuffer::upload(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
      let c = gpu_cumsum_rows(&x, 2, 3).unwrap();
      let mut out = [0.0; 6]; c.download(&mut out).unwrap();
      eprintln!("cumsum_rows = {:?}", out); approx(&out, &[1.0, 3.0, 6.0, 4.0, 9.0, 15.0]);
}

#[test]
fn one_hot() {
      let labels = GpuBuffer::upload_i32(&[0, 2]).unwrap();
      let oh = gpu_one_hot(&labels, 2, 3).unwrap();
      let mut out = [0.0; 6]; oh.download(&mut out).unwrap();
      eprintln!("one_hot = {:?}", out); approx(&out, &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn discounted_returns() {
      let r = GpuBuffer::upload(&[1.0, 1.0, 1.0]).unwrap();
      let g = gpu_discounted_returns(&r, 0.5, 3).unwrap();
      let mut out = [0.0; 3]; g.download(&mut out).unwrap();
      eprintln!("discounted_returns = {:?}", out); approx(&out, &[1.75, 1.5, 1.0]);
}
