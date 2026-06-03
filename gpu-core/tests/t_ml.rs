use gpu_core::memory::GpuBuffer;
use gpu_core::rl::{
      gpu_discounted_returns, gpu_gae, gpu_td_targets,
      gpu_categorical_logprob, gpu_gaussian_logprob,
};
use gpu_core::bayes::{
      gpu_nb_count_table, gpu_nb_feature_log_prob,
      gpu_multinomial_nb_logprob, gpu_bernoulli_nb_logprob,
};
use gpu_core::forest::{
      gpu_bootstrap_sample, gpu_feature_subset,
      gpu_random_threshold_split, gpu_oob_mask,
};
use gpu_core::catboost::{
      gpu_iota, gpu_random_permutation, gpu_ordered_target_stats,
};
use gpu_core::svm::{gpu_kernel_matrix, gpu_smo_train};

const EPS: f64 = 1e-9;

// ── RL ────────────────────────────────────────────────────────────────────────

#[test]
fn test_rl_discounted_returns() {
      let rewards = GpuBuffer::upload(&[1.0_f64, 1.0, 1.0]).unwrap();
      let gamma = 0.5_f64;
      let out = gpu_discounted_returns(&rewards, gamma, 3).unwrap();
      let mut result = [0.0_f64; 3];
      out.download(&mut result).unwrap();

      // G[2] = 1.0
      // G[1] = 1.0 + 0.5 * 1.0 = 1.5
      // G[0] = 1.0 + 0.5 * 1.5 = 1.75
      assert!((result[0] - 1.75).abs() < EPS, "G[0]={} expected 1.75", result[0]);
      assert!((result[1] - 1.50).abs() < EPS, "G[1]={} expected 1.5",  result[1]);
      assert!((result[2] - 1.00).abs() < EPS, "G[2]={} expected 1.0",  result[2]);
      eprintln!("discounted_returns OK: {:?}", result);
}

#[test]
fn test_rl_gae() {
      // Simple 3-step case: r=[1,1,1], v=[1,1,1], gamma=0.9, lam=0.95
      let rewards = GpuBuffer::upload(&[1.0_f64, 1.0, 1.0]).unwrap();
      let values  = GpuBuffer::upload(&[1.0_f64, 1.0, 1.0]).unwrap();
      let gamma = 0.9_f64;
      let lam   = 0.95_f64;
      let out = gpu_gae(&rewards, &values, gamma, lam, 3).unwrap();
      let mut result = [0.0_f64; 3];
      out.download(&mut result).unwrap();

      // Hand compute:
      // delta[t] = r[t] + gamma * v[t+1] - v[t]   (v[3]=0)
      // delta[2] = 1 + 0.9*0 - 1 = 0
      // delta[1] = 1 + 0.9*1 - 1 = 0.9
      // delta[0] = 1 + 0.9*1 - 1 = 0.9
      // A[t] = delta[t] + gamma*lam*A[t+1]
      // A[2] = 0
      // A[1] = 0.9 + 0.9*0.95*0 = 0.9
      // A[0] = 0.9 + 0.9*0.95*0.9 = 0.9 + 0.7695 = 1.6695
      let exp_a2 = 0.0_f64;
      let exp_a1 = 0.9_f64;
      let exp_a0 = 0.9 + gamma * lam * exp_a1;
      assert!((result[2] - exp_a2).abs() < 1e-9, "GAE A[2]={}", result[2]);
      assert!((result[1] - exp_a1).abs() < 1e-9, "GAE A[1]={}", result[1]);
      assert!((result[0] - exp_a0).abs() < 1e-9, "GAE A[0]={} expected {}", result[0], exp_a0);
      eprintln!("gae OK: {:?}", result);
}

#[test]
fn test_rl_td_targets() {
      // r=[1,2,3], v_next=[4,5,6], done=[0,1,0], gamma=0.99
      let rewards     = GpuBuffer::upload(&[1.0_f64, 2.0, 3.0]).unwrap();
      let values_next = GpuBuffer::upload(&[4.0_f64, 5.0, 6.0]).unwrap();
      let done        = GpuBuffer::upload(&[0.0_f64, 1.0, 0.0]).unwrap();
      let gamma = 0.99_f64;
      let out = gpu_td_targets(&rewards, &values_next, gamma, &done, 3).unwrap();
      let mut result = [0.0_f64; 3];
      out.download(&mut result).unwrap();

      // t[0] = 1 + 0.99*4*(1-0) = 1 + 3.96 = 4.96
      // t[1] = 2 + 0.99*5*(1-1) = 2
      // t[2] = 3 + 0.99*6*(1-0) = 3 + 5.94 = 8.94
      let exp = [4.96_f64, 2.0, 8.94];
      for i in 0..3 {
            assert!((result[i] - exp[i]).abs() < 1e-9, "td_targets[{}]={} expected {}", i, result[i], exp[i]);
      }
      eprintln!("td_targets OK: {:?}", result);
}

#[test]
fn test_rl_categorical_logprob() {
      // n=1, n_actions=3, logits=[0,1,2], action=2
      // softmax: exp([0,1,2])/sum = [1, e, e^2] / (1+e+e^2)
      // log_softmax[2] = 2 - log(1 + e + e^2)
      let logits  = GpuBuffer::upload(&[0.0_f64, 1.0, 2.0]).unwrap();
      let actions = GpuBuffer::upload_i32(&[2_i32]).unwrap();
      let out = gpu_categorical_logprob(&logits, &actions, 1, 3).unwrap();
      let mut result = [0.0_f64; 1];
      out.download(&mut result).unwrap();

      let log_z = (1.0_f64.exp() + 2.0_f64.exp() + 1.0_f64).ln();
      let expected = 2.0 - log_z;
      assert!(result[0].is_finite(), "categorical_logprob is not finite: {}", result[0]);
      assert!((result[0] - expected).abs() < 1e-9,
            "categorical_logprob={} expected={}", result[0], expected);
      eprintln!("categorical_logprob OK: {} (expected {})", result[0], expected);
}

#[test]
fn test_rl_gaussian_logprob() {
      // n=1, dim=1, mu=0, log_std=0 (sigma=1), action=1.0
      // logp = -0.5*(1)^2 - 0 - 0.5*log(2*pi)
      let mu      = GpuBuffer::upload(&[0.0_f64]).unwrap();
      let log_std = GpuBuffer::upload(&[0.0_f64]).unwrap();
      let actions = GpuBuffer::upload(&[1.0_f64]).unwrap();
      let out = gpu_gaussian_logprob(&mu, &log_std, &actions, 1, 1).unwrap();
      let mut result = [0.0_f64; 1];
      out.download(&mut result).unwrap();

      let half_log2pi = 0.5 * (2.0 * std::f64::consts::PI).ln();
      let expected = -0.5 - 0.0 - half_log2pi;
      assert!(result[0].is_finite(), "gaussian_logprob not finite: {}", result[0]);
      assert!((result[0] - expected).abs() < 1e-9,
            "gaussian_logprob={} expected={}", result[0], expected);
      eprintln!("gaussian_logprob OK: {} (expected {})", result[0], expected);
}

// ── Bayes ─────────────────────────────────────────────────────────────────────

#[test]
fn test_bayes_nb_feature_log_prob() {
      // 2 classes, 3 features. counts:
      // class 0: [1, 2, 3]  sum=6, alpha=1 -> smoothed: [2,3,4]/9
      // class 1: [4, 5, 6]  sum=15, alpha=1 -> smoothed: [5,6,7]/18
      let counts = GpuBuffer::upload(&[
            1.0_f64, 2.0, 3.0,   // class 0
            4.0,     5.0, 6.0,   // class 1
      ]).unwrap();
      let alpha = 1.0_f64;
      let out = gpu_nb_feature_log_prob(&counts, 2, 3, alpha).unwrap();
      let mut result = [0.0_f64; 6];
      out.download(&mut result).unwrap();

      // class 0: log([2,3,4]) - log(9)
      let log9 = 9.0_f64.ln();
      let exp_c0 = [2.0_f64.ln() - log9, 3.0_f64.ln() - log9, 4.0_f64.ln() - log9];
      // class 1: log([5,6,7]) - log(18)
      let log18 = 18.0_f64.ln();
      let exp_c1 = [5.0_f64.ln() - log18, 6.0_f64.ln() - log18, 7.0_f64.ln() - log18];

      for f in 0..3 {
            assert!(result[f].is_finite(), "class0 feat{} not finite", f);
            assert!((result[f] - exp_c0[f]).abs() < 1e-9,
                  "class0 feat{}: got={} expected={}", f, result[f], exp_c0[f]);
      }
      for f in 0..3 {
            assert!(result[3+f].is_finite(), "class1 feat{} not finite", f);
            assert!((result[3+f] - exp_c1[f]).abs() < 1e-9,
                  "class1 feat{}: got={} expected={}", f, result[3+f], exp_c1[f]);
      }
      eprintln!("nb_feature_log_prob OK: {:?}", result);
}

#[test]
fn test_bayes_nb_count_table() {
      // n=3, n_features=2, n_classes=2
      // x_counts: [[1,0],[0,1],[1,1]], y=[0,1,0]
      // expected count_table: class0=[2,1], class1=[0,1]
      let x = GpuBuffer::upload(&[
            1.0_f64, 0.0,
            0.0,     1.0,
            1.0,     1.0,
      ]).unwrap();
      let y = GpuBuffer::upload_i32(&[0_i32, 1, 0]).unwrap();
      let out = gpu_nb_count_table(&x, &y, 3, 2, 2).unwrap();
      let mut result = [0.0_f64; 4];
      out.download(&mut result).unwrap();

      // row-major: class0=[result[0],result[1]], class1=[result[2],result[3]]
      assert!((result[0] - 2.0).abs() < 1e-9, "c0f0={}", result[0]);
      assert!((result[1] - 1.0).abs() < 1e-9, "c0f1={}", result[1]);
      assert!((result[2] - 0.0).abs() < 1e-9, "c1f0={}", result[2]);
      assert!((result[3] - 1.0).abs() < 1e-9, "c1f1={}", result[3]);
      eprintln!("nb_count_table OK: {:?}", result);
}

#[test]
fn test_bayes_multinomial_nb_logprob() {
      // 1 sample, 2 features, 2 classes
      // log_prior=[log(0.5),log(0.5)]
      // feature_log_prob: class0=[log(0.3),log(0.7)], class1=[log(0.6),log(0.4)]
      // x=[1,1]
      // out[0] = log(0.5) + 1*log(0.3) + 1*log(0.7)
      // out[1] = log(0.5) + 1*log(0.6) + 1*log(0.4)
      let log_prior = GpuBuffer::upload(&[0.5_f64.ln(), 0.5_f64.ln()]).unwrap();
      let flp = GpuBuffer::upload(&[
            0.3_f64.ln(), 0.7_f64.ln(),   // class 0
            0.6_f64.ln(), 0.4_f64.ln(),   // class 1
      ]).unwrap();
      let x = GpuBuffer::upload(&[1.0_f64, 1.0]).unwrap();
      let out = gpu_multinomial_nb_logprob(&log_prior, &flp, &x, 1, 2, 2).unwrap();
      let mut result = [0.0_f64; 2];
      out.download(&mut result).unwrap();

      let exp0 = 0.5_f64.ln() + 0.3_f64.ln() + 0.7_f64.ln();
      let exp1 = 0.5_f64.ln() + 0.6_f64.ln() + 0.4_f64.ln();
      assert!(result[0].is_finite(), "multinomial logprob[0] not finite");
      assert!(result[1].is_finite(), "multinomial logprob[1] not finite");
      assert!((result[0] - exp0).abs() < 1e-9, "logprob[0]={} expected={}", result[0], exp0);
      assert!((result[1] - exp1).abs() < 1e-9, "logprob[1]={} expected={}", result[1], exp1);
      eprintln!("multinomial_nb_logprob OK: {:?}", result);
}

#[test]
fn test_bayes_bernoulli_nb_logprob() {
      // 1 sample, 2 features, 2 classes
      // log_p: class0=[log(0.3),log(0.7)], class1=[log(0.6),log(0.4)]
      // log_neg: class0=[log(0.7),log(0.3)], class1=[log(0.4),log(0.6)]
      // x=[1,0] (binary)
      // out[0] = log_prior[0] + 1*log(0.3) + 0*log(0.7) + 0*log(0.7) + 1*log(0.3)
      //        = log_prior[0] + log(0.3) + log(0.3)
      // out[1] = log_prior[1] + 1*log(0.6) + 0*log(0.4) + 0*log(0.4) + 1*log(0.6)
      //        = log_prior[1] + log(0.6) + log(0.6)
      let log_prior = GpuBuffer::upload(&[0.5_f64.ln(), 0.5_f64.ln()]).unwrap();
      let log_p   = GpuBuffer::upload(&[
            0.3_f64.ln(), 0.7_f64.ln(),
            0.6_f64.ln(), 0.4_f64.ln(),
      ]).unwrap();
      let log_neg = GpuBuffer::upload(&[
            0.7_f64.ln(), 0.3_f64.ln(),
            0.4_f64.ln(), 0.6_f64.ln(),
      ]).unwrap();
      let x = GpuBuffer::upload(&[1.0_f64, 0.0]).unwrap();
      let out = gpu_bernoulli_nb_logprob(&log_prior, &log_p, &log_neg, &x, 1, 2, 2).unwrap();
      let mut result = [0.0_f64; 2];
      out.download(&mut result).unwrap();

      // x=[1,0]: feat0 present (use log_p[c,0]), feat1 absent (use log_neg[c,1])
      let exp0 = 0.5_f64.ln() + 0.3_f64.ln() + 0.3_f64.ln();
      let exp1 = 0.5_f64.ln() + 0.6_f64.ln() + 0.6_f64.ln();
      assert!(result[0].is_finite(), "bernoulli logprob[0] not finite");
      assert!(result[1].is_finite(), "bernoulli logprob[1] not finite");
      assert!((result[0] - exp0).abs() < 1e-9, "bernoulli logprob[0]={} expected={}", result[0], exp0);
      assert!((result[1] - exp1).abs() < 1e-9, "bernoulli logprob[1]={} expected={}", result[1], exp1);
      eprintln!("bernoulli_nb_logprob OK: {:?}", result);
}

// ── Forest ────────────────────────────────────────────────────────────────────

#[test]
fn test_forest_bootstrap_sample() {
      let n = 10_usize;
      let n_samples = 20_usize;
      let buf = gpu_bootstrap_sample(n, n_samples, 42).unwrap();
      // buf.len() is bytes = n_samples * 4
      assert_eq!(buf.len(), n_samples * 4, "wrong byte length");
      let mut idx = vec![0_i32; n_samples];
      buf.download_i32(&mut idx).unwrap();
      for &v in &idx {
            assert!(v >= 0 && v < n as i32, "index {} out of [0,{})", v, n);
      }
      eprintln!("bootstrap_sample OK: {:?}", idx);
}

#[test]
fn test_forest_feature_subset() {
      let n_features = 10_usize;
      let k = 4_usize;
      let buf = gpu_feature_subset(n_features, k, 7).unwrap();
      // buf is alloc_bytes(n_features * 4) — always returns n_features indices sorted by key
      // the first k are the selected subset
      let mut all_idx = vec![0_i32; n_features];
      buf.download_i32(&mut all_idx).unwrap();
      // First k indices must be in [0, n_features)
      for i in 0..k {
            assert!(all_idx[i] >= 0 && all_idx[i] < n_features as i32,
                  "feature_subset[{}]={} out of range", i, all_idx[i]);
      }
      // Check first k are distinct
      let subset: std::collections::HashSet<i32> = all_idx[..k].iter().cloned().collect();
      assert_eq!(subset.len(), k, "feature_subset has duplicates in first {}: {:?}", k, &all_idx[..k]);
      eprintln!("feature_subset OK (first {}): {:?}", k, &all_idx[..k]);
}

#[test]
fn test_forest_random_threshold_split() {
      let col_data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
      let col = GpuBuffer::upload(&col_data).unwrap();
      let threshold = gpu_random_threshold_split(&col, col_data.len(), 99).unwrap();
      assert!(threshold.is_finite(), "threshold not finite: {}", threshold);
      let col_min = 1.0_f64;
      let col_max = 5.0_f64;
      assert!(threshold >= col_min && threshold <= col_max,
            "threshold {} not in [{}, {}]", threshold, col_min, col_max);
      eprintln!("random_threshold_split OK: {}", threshold);
}

#[test]
fn test_forest_oob_mask() {
      // n=6, bootstrap picks indices [0,1,2,3] (4 samples)
      // OOB should be indices 4 and 5 (never sampled)
      let bootstrap = [0_i32, 1, 2, 3];
      let n = 6_usize;
      let bs_buf = GpuBuffer::upload_i32(&bootstrap).unwrap();
      let mask_buf = gpu_oob_mask(&bs_buf, n).unwrap();
      // mask_buf is zeros_bytes(n) -> u8 array of length n bytes
      let mut mask = vec![0_u8; n];
      mask_buf.download_u8(&mut mask).unwrap();

      // Indices 0..3 are in bootstrap -> oob=0; indices 4,5 not in bootstrap -> oob=1
      for i in 0..4 {
            assert_eq!(mask[i], 0, "oob[{}] should be 0 (was sampled)", i);
      }
      for i in 4..6 {
            assert_eq!(mask[i], 1, "oob[{}] should be 1 (not sampled)", i);
      }
      eprintln!("oob_mask OK: {:?}", mask);
}

// ── CatBoost ──────────────────────────────────────────────────────────────────

#[test]
fn test_catboost_iota() {
      let n = 8_usize;
      let buf = gpu_iota(n).unwrap();
      let mut out = vec![0_i32; n];
      buf.download_i32(&mut out).unwrap();
      for (i, &v) in out.iter().enumerate() {
            assert_eq!(v, i as i32, "iota[{}]={} expected {}", i, v, i);
      }
      eprintln!("iota OK: {:?}", out);
}

#[test]
fn test_catboost_random_permutation() {
      let n = 16_usize;
      let buf = gpu_random_permutation(n, 42).unwrap();
      let mut perm = vec![0_i32; n];
      buf.download_i32(&mut perm).unwrap();

      // Valid permutation: sort and check 0..n
      let mut sorted = perm.clone();
      sorted.sort();
      for (i, &v) in sorted.iter().enumerate() {
            assert_eq!(v, i as i32, "permutation is not a valid permutation at position {}", i);
      }
      eprintln!("random_permutation OK: {:?}", perm);
}

#[test]
fn test_catboost_ordered_target_stats() {
      // Simple 4-row case, 2 categories (0,1), prior=0, smoothing=1
      // cat_col:  [0, 1, 0, 1]
      // target:   [10, 20, 30, 40]
      // perm:     [0, 1, 2, 3]   (identity permutation)
      //
      // Walk:
      //   p=0, row=0, cat=0: sum=0,cnt=0 -> TS=(0+0*1)/(0+1)=0.  after: sum[0]=10,cnt[0]=1
      //   p=1, row=1, cat=1: sum=0,cnt=0 -> TS=(0+0*1)/(0+1)=0.  after: sum[1]=20,cnt[1]=1
      //   p=2, row=2, cat=0: sum=10,cnt=1-> TS=(10+0)/(1+1)=5.   after: sum[0]=40,cnt[0]=2
      //   p=3, row=3, cat=1: sum=20,cnt=1-> TS=(20+0)/(1+1)=10.  after: sum[1]=60,cnt[1]=2
      // encoded_out = [0, 0, 5, 10] (indexed by original row)
      let cat_col = GpuBuffer::upload_i32(&[0_i32, 1, 0, 1]).unwrap();
      let target  = GpuBuffer::upload(&[10.0_f64, 20.0, 30.0, 40.0]).unwrap();
      let perm    = GpuBuffer::upload_i32(&[0_i32, 1, 2, 3]).unwrap();
      let prior = 0.0_f64;
      let smoothing = 1.0_f64;
      let out = gpu_ordered_target_stats(&cat_col, &target, &perm, 4, 2, prior, smoothing).unwrap();
      let mut result = [0.0_f64; 4];
      out.download(&mut result).unwrap();

      let expected = [0.0_f64, 0.0, 5.0, 10.0];
      for i in 0..4 {
            assert!(result[i].is_finite(), "ordered_target_stats[{}] not finite", i);
            assert!((result[i] - expected[i]).abs() < 1e-9,
                  "ordered_target_stats[{}]={} expected {}", i, result[i], expected[i]);
      }
      eprintln!("ordered_target_stats OK: {:?}", result);
}

// ── SVM ───────────────────────────────────────────────────────────────────────

#[test]
fn test_svm_kernel_matrix_linear() {
      // 3 points in 2D: x=[[1,0],[0,1],[1,1]]
      // Linear kernel: K[i,j] = x_i . x_j
      // K = [[1,0,1],[0,1,1],[1,1,2]]
      let x = GpuBuffer::upload(&[
            1.0_f64, 0.0,
            0.0,     1.0,
            1.0,     1.0,
      ]).unwrap();
      let k = gpu_kernel_matrix(&x, 3, 2, 0, 1.0, 0.0, 2.0).unwrap();
      let mut km = [0.0_f64; 9];
      k.download(&mut km).unwrap();

      let expected = [
            1.0, 0.0, 1.0,
            0.0, 1.0, 1.0,
            1.0, 1.0, 2.0,
      ];
      for (i, (&got, &exp)) in km.iter().zip(expected.iter()).enumerate() {
            assert!(got.is_finite(), "K[{}] not finite", i);
            assert!((got - exp).abs() < 1e-9, "K_linear[{}]={} expected {}", i, got, exp);
      }
      eprintln!("kernel_matrix linear OK: {:?}", km);
}

#[test]
fn test_svm_kernel_matrix_rbf() {
      // 2 points in 1D: x=[[0],[1]], gamma=1
      // RBF: K[0,0]=exp(0)=1, K[0,1]=exp(-1), K[1,0]=exp(-1), K[1,1]=1
      let x = GpuBuffer::upload(&[0.0_f64, 1.0]).unwrap();
      let k = gpu_kernel_matrix(&x, 2, 1, 1, 1.0, 0.0, 2.0).unwrap();
      let mut km = [0.0_f64; 4];
      k.download(&mut km).unwrap();

      let exp01 = (-1.0_f64).exp();
      assert!((km[0] - 1.0).abs() < 1e-9, "K[0,0]={}", km[0]);
      assert!((km[1] - exp01).abs() < 1e-9, "K[0,1]={} expected {}", km[1], exp01);
      assert!((km[2] - exp01).abs() < 1e-9, "K[1,0]={} expected {}", km[2], exp01);
      assert!((km[3] - 1.0).abs() < 1e-9, "K[1,1]={}", km[3]);
      eprintln!("kernel_matrix rbf OK: {:?}", km);
}

#[test]
fn test_svm_smo_train_no_hang_box_valid() {
      // Verifies smo_train does not hang and returns box-feasible alphas.
      //
      // KNOWN DEFECT (host-side, separate from svm.hip sign bug):
      // svm.rs:204 breaks on delta<1e-12 at iteration 0. With all-zero alpha init,
      // the dual gradient G is uniformly -1, so G[i]-G[j]=0 for any working-set pair,
      // giving zero step -> immediate delta-break. smo_train cannot converge from alpha=0
      // on any binary SVM problem until this second bug is also fixed.
      //
      // The svm.hip:159 sign fix is correct and necessary: without it the KKT gap
      // falsely reads 0 (gap=score_i-score_j=1-1=0 with wrong sign) and the convergence
      // check fires even before the delta-break. With the fix, gap=2>0 correctly, but
      // the delta-break fires instead. Both bugs compound.
      use std::time::{Duration, Instant};

      let x = GpuBuffer::upload(&[
             2.0_f64,  2.0,
             2.0,     -2.0,
            -2.0,      2.0,
            -2.0,     -2.0,
      ]).unwrap();
      let y_pm1 = [1.0_f64, 1.0, -1.0, -1.0];
      let k = gpu_kernel_matrix(&x, 4, 2, 0, 1.0, 0.0, 1.0).unwrap();

      let start = Instant::now();
      let (alphas, b) = gpu_smo_train(&k, &y_pm1, 1.0, 1e-3, 1000, 4)
            .expect("gpu_smo_train returned Err");
      let elapsed = start.elapsed();

      assert!(elapsed < Duration::from_secs(30),
            "FINDING: gpu_smo_train HUNG (>30s)");

      for (i, &a) in alphas.iter().enumerate() {
            assert!(a.is_finite(), "alpha[{}] not finite: {}", i, a);
            assert!(a >= -1e-9, "alpha[{}]={} < 0", i, a);
            assert!(a <= 1.0 + 1e-9, "alpha[{}]={} > C=1.0", i, a);
      }
      assert!(b.is_finite(), "bias b not finite: {}", b);

      eprintln!("smo_train no-hang + box-valid OK: alphas={:?}, b={} ({:?})", alphas, b, elapsed);
      eprintln!("NOTE: alphas remain 0 (second bug: svm.rs:204 delta-break on uniform G at init)");
}
