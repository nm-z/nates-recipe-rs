use lightgbm_rs::{train_multiclass, predict_proba, Params};
use std::time::Instant;

fn load_csv(path: &str) -> (Vec<f64>, Vec<usize>, usize, usize) {
      let data = std::fs::read_to_string(path).unwrap();
      let mut x = Vec::new();
      let mut y = Vec::new();
      let mut p = 0;
      for line in data.lines() {
            if line.is_empty() { continue; }
            let vals: Vec<&str> = line.split(',').collect();
            p = vals.len() - 1;
            for v in &vals[..p] { x.push(v.parse::<f64>().unwrap()); }
            y.push(vals[p].parse::<usize>().unwrap() - 1);
      }
      let n = y.len();
      (x, y, n, p)
}

fn main() {
      let (x_tr, y_tr, n_tr, p) = load_csv("/tmp/covtype_train.csv");
      let (x_te, y_te, n_te, p_te) = load_csv("/tmp/covtype_test.csv");
      assert_eq!(p, p_te);

      let n_classes = *y_tr.iter().max().unwrap() + 1;
      eprintln!("Covtype: train={n_tr} test={n_te} features={p} classes={n_classes}");

      let params = Params {
            n_estimators: 1000,
            num_leaves: 63,
            max_depth: 0,
            learning_rate: 0.05,
            l2_reg: 1.0,
            min_child_weight: 1.0,
            min_gain_to_split: 0.0,
            n_bins: 255,
            goss_a: 0.2,
            goss_b: 0.1,
            use_efb: false,
            efb_max_conflict: 0.0,
            seed: 42,
      };
      eprintln!("Config: iters={} leaves={} lr={} goss_a={} goss_b={}", params.n_estimators, params.num_leaves, params.learning_rate, params.goss_a, params.goss_b);

      let t0 = Instant::now();
      let model = train_multiclass(&x_tr, &y_tr, n_tr, p, n_classes, &params).unwrap();
      let train_time = t0.elapsed();
      eprintln!("Training: {:.2}s ({:.2}ms/iter)", train_time.as_secs_f64(), train_time.as_secs_f64() * 1000.0 / params.n_estimators as f64);

      let t1 = Instant::now();
      let probs = predict_proba(&model, &x_te, n_te).unwrap();
      let predict_time = t1.elapsed();

      let correct = (0..n_te).filter(|&i| {
            (0..n_classes).max_by(|&a, &b| {
                  probs[i * n_classes + a].partial_cmp(&probs[i * n_classes + b]).unwrap()
            }).unwrap() == y_te[i]
      }).count();
      let accuracy = correct as f64 / n_te as f64;

      eprintln!("\n# ── lightgbm-rs results ──────────────────────────────────────");
      eprintln!("  dataset:       Covertype (train={n_tr}, test={n_te}, features={p})");
      eprintln!("  accuracy:      {:.4} ({correct}/{n_te})", accuracy);
      eprintln!("  train time:    {:.2}s", train_time.as_secs_f64());
      eprintln!("  predict time:  {:.3}s", predict_time.as_secs_f64());
      eprintln!("  ms/iter:       {:.2}", train_time.as_secs_f64() * 1000.0 / params.n_estimators as f64);
}
