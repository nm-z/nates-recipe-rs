use gpu_core::memory::GpuBuffer;
use gpu_core::svm::gpu_smo_train;

#[test]
fn smo_trains_separable() {
      let x = [[2.0, 2.0], [2.0, -2.0], [-2.0, 2.0], [-2.0, -2.0]];
      let y = [1.0_f64, 1.0, -1.0, -1.0];
      let n = 4;

      let mut k = vec![0.0_f64; n * n];
      for i in 0..n {
            for j in 0..n {
                  k[i * n + j] = x[i][0] * x[j][0] + x[i][1] * x[j][1];
            }
      }
      let k_gpu = GpuBuffer::upload(&k).unwrap();

      let (alpha, b) = gpu_smo_train(&k_gpu, &y, 1.0, 1e-3, 1000, n).unwrap();
      eprintln!("alpha = {:?}, b = {}", alpha, b);

      assert!(alpha.iter().all(|a| a.is_finite()), "non-finite alpha");
      assert!(alpha.iter().all(|&a| a >= -1e-9 && a <= 1.0 + 1e-9), "alpha out of [0,C]");
      assert!(alpha.iter().any(|&a| a > 1e-6), "SMO did not train: all alphas ~0");

      for i in 0..n {
            let f: f64 = (0..n).map(|t| alpha[t] * y[t] * k[t * n + i]).sum::<f64>() + b;
            eprintln!("point {}: f={:.4} y={}", i, f, y[i]);
            assert!(f * y[i] > 0.0, "point {} misclassified: f={} y={}", i, f, y[i]);
      }
}
