use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::fmt;

#[derive(Debug)]
pub enum Error {
      InvalidInput(String),
}
impl fmt::Display for Error {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self { Error::InvalidInput(s) => write!(f, "xgboost-rs: {s}") }
      }
}
impl std::error::Error for Error {}

pub struct Params {
      pub n_estimators: usize,
      pub max_depth: usize,
      pub learning_rate: f64,
      pub l2_reg: f64,
      pub min_child_weight: f64,
      pub subsample: f64,
      pub colsample_bytree: f64,
      pub n_bins: usize,
      pub seed: u64,
}

impl Default for Params {
      fn default() -> Self {
            Self {
                  n_estimators: 100, max_depth: 6, learning_rate: 0.1,
                  l2_reg: 1.0, min_child_weight: 1.0, subsample: 1.0,
                  colsample_bytree: 1.0, n_bins: 256, seed: 42,
            }
      }
}

#[derive(Clone)]
struct LevelTree {
      split_feat: Vec<i32>,
      split_bin:  Vec<i32>,
      leaf_val:   Vec<f64>,
      max_depth:  usize,
      col_map:    Vec<usize>,
}

pub struct Model {
      trees:         Vec<Vec<LevelTree>>,
      learning_rate: f64,
      n_classes:     usize,
      borders:       Vec<Vec<f64>>,
      n_features:    usize,
}

fn quantize_col(x: &[f64], n: usize, p: usize, j: usize, n_bins: usize) -> (Vec<f64>, Vec<u8>) {
      let mut col: Vec<f64> = (0..n).map(|i| x[i * p + j]).collect();
      col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
      let step = (n as f64 / (n_bins - 1) as f64).max(1.0);
      let mut borders: Vec<f64> = vec![];
      let mut idx = step;
      while (idx as usize) < n {
            let v = col[idx as usize];
            if borders.is_empty() || (v - borders[borders.len() - 1]).abs() > 1e-12 {
                  borders.push(v);
            }
            idx += step;
      }
      let bins: Vec<u8> = (0..n).map(|i| {
            let v = x[i * p + j];
            let b = borders.partition_point(|&t| t <= v);
            b.min(255) as u8
      }).collect();
      (borders, bins)
}

fn quantize(x: &[f64], n: usize, p: usize, n_bins: usize) -> (Vec<Vec<f64>>, Vec<u8>) {
      let mut all_borders = vec![vec![]; p];
      let mut flat = vec![0u8; n * p];
      for j in 0..p {
            let (borders, bins) = quantize_col(x, n, p, j, n_bins);
            all_borders[j] = borders;
            for i in 0..n { flat[i * p + j] = bins[i]; }
      }
      (all_borders, flat)
}

fn quantize_with_borders(x: &[f64], n: usize, p: usize, borders: &[Vec<f64>]) -> Vec<u8> {
      let mut flat = vec![0u8; n * p];
      for j in 0..p {
            let b = &borders[j];
            for i in 0..n {
                  let v = x[i * p + j];
                  flat[i * p + j] = b.partition_point(|&t| t <= v).min(255) as u8;
            }
      }
      flat
}

fn softmax_inplace(logits: &mut [f64], n: usize, nc: usize) {
      for i in 0..n {
            let row = &mut logits[i * nc..(i + 1) * nc];
            let mx = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let s: f64 = row.iter().map(|&v| (v - mx).exp()).sum();
            for v in row.iter_mut() { *v = (*v - mx).exp() / s; }
      }
}

fn build_cpu_tree(
      bins_full: &[u8], n: usize, p: usize,
      grad: &[f64], hess: &[f64],
      col_map: &[usize],
      max_depth: usize, lambda: f64,
) -> (LevelTree, Vec<u32>) {
      let max_nodes = (1usize << (max_depth + 1)) - 1;
      let mut node_assign = vec![0u32; n];
      let mut sf = vec![-1i32; max_nodes];
      let mut sb = vec![-1i32; max_nodes];

      for d in 0..max_depth {
            let level_base = (1usize << d) - 1;
            let n_level = 1usize << d;
            let mut best_f = vec![-1i32; n_level];
            let mut best_b = vec![-1i32; n_level];

            for node_off in 0..n_level {
                  let node_id = level_base + node_off;
                  let mut best_gain = 0.0f64;

                  for (jc, &j) in col_map.iter().enumerate() {
                        let n_hist = 256usize;
                        let mut gh = vec![0.0f64; n_hist];
                        let mut hh = vec![0.0f64; n_hist];
                        for i in 0..n {
                              if node_assign[i] as usize != node_id { continue; }
                              let b = bins_full[i * p + j] as usize;
                              gh[b] += grad[i];
                              hh[b] += hess[i];
                        }
                        let total_g: f64 = gh.iter().sum();
                        let total_h: f64 = hh.iter().sum();
                        let parent = total_g * total_g / (total_h + lambda);
                        let mut lg = 0.0f64;
                        let mut lh = 0.0f64;
                        for b in 0..n_hist - 1 {
                              lg += gh[b]; lh += hh[b];
                              let rg = total_g - lg;
                              let rh = total_h - lh;
                              if lh < 1e-4 || rh < 1e-4 { continue; }
                              let gain = lg * lg / (lh + lambda) + rg * rg / (rh + lambda) - parent;
                              if gain > best_gain {
                                    best_gain = gain;
                                    best_f[node_off] = jc as i32;
                                    best_b[node_off] = b as i32;
                              }
                        }
                  }
                  sf[node_id] = if best_f[node_off] >= 0 { best_f[node_off] } else { -1 };
                  sb[node_id] = if best_b[node_off] >= 0 { best_b[node_off] } else { -1 };
            }

            for i in 0..n {
                  let node = node_assign[i] as usize;
                  if sf[node] < 0 { continue; }
                  let f = col_map[sf[node] as usize];
                  let b = sb[node];
                  let bin_val = bins_full[i * p + f] as i32;
                  node_assign[i] = if bin_val <= b { (2 * node + 1) as u32 } else { (2 * node + 2) as u32 };
            }
      }

      let mut sum_g = vec![0.0f64; max_nodes];
      let mut sum_h = vec![0.0f64; max_nodes];
      for i in 0..n {
            let node = node_assign[i] as usize;
            sum_g[node] += grad[i];
            sum_h[node] += hess[i];
      }
      let leaf_val: Vec<f64> = (0..max_nodes).map(|node| -sum_g[node] / (sum_h[node] + lambda)).collect();

      (LevelTree { split_feat: sf, split_bin: sb, leaf_val, max_depth, col_map: col_map.to_vec() }, node_assign)
}

fn route_tree(tree: &LevelTree, bins_full: &[u8], n: usize, p: usize) -> Vec<u32> {
      (0..n).map(|i| {
            let mut node = 0usize;
            for _ in 0..tree.max_depth {
                  if tree.split_feat[node] < 0 { break; }
                  let jc = tree.split_feat[node] as usize;
                  let j = tree.col_map[jc];
                  let bin_val = bins_full[i * p + j] as i32;
                  node = if bin_val <= tree.split_bin[node] { 2 * node + 1 } else { 2 * node + 2 };
            }
            node as u32
      }).collect()
}

fn sample_mask(n: usize, rate: f64, rng: &mut ChaCha8Rng) -> Vec<bool> {
      (0..n).map(|_| rng.random::<f64>() < rate).collect()
}

fn sample_cols(p: usize, rate: f64, rng: &mut ChaCha8Rng) -> Vec<usize> {
      let mut cols: Vec<usize> = (0..p).collect();
      cols.shuffle(rng);
      let k = ((p as f64 * rate).ceil() as usize).max(1).min(p);
      cols.truncate(k);
      cols.sort_unstable();
      cols
}

fn validate(x: &[f64], n: usize, p: usize) -> Result<(), Error> {
      if n == 0 { return Err(Error::InvalidInput("n must be > 0".into())); }
      if p == 0 { return Err(Error::InvalidInput("p must be > 0".into())); }
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }
      for (i, &v) in x.iter().enumerate() {
            if v.is_nan() { return Err(Error::InvalidInput(format!("x[{i}] is NaN"))); }
      }
      Ok(())
}

pub fn train(x: &[f64], y: &[f64], n: usize, p: usize, params: &Params) -> Result<Model, Error> {
      validate(x, n, p)?;
      if y.len() != n { return Err(Error::InvalidInput(format!("y.len()={} != n={}", y.len(), n))); }

      let mut rng = ChaCha8Rng::seed_from_u64(params.seed);
      let (borders, bins) = quantize(x, n, p, params.n_bins);
      let mut pred = vec![0.0f64; n];
      let mut trees: Vec<Vec<LevelTree>> = Vec::with_capacity(params.n_estimators);

      for t in 0..params.n_estimators {
            let row_mask = sample_mask(n, params.subsample, &mut rng);
            let col_map = sample_cols(p, params.colsample_bytree, &mut (ChaCha8Rng::seed_from_u64(params.seed ^ (t as u64 * 6364136223846793005 + 1))));

            let mut grad = vec![0.0f64; n];
            let mut hess = vec![0.0f64; n];
            for i in 0..n {
                  if !row_mask[i] { continue; }
                  grad[i] = pred[i] - y[i];
                  hess[i] = 1.0;
            }

            // GPU path not integrated: gpu_tree_build_into emits only per-sample tr_pred/te_pred, never the split_feat/split_bin/leaf_val tree the Model stores for predict() — true GPU integration requires gpu-core to return that tree structure.
            let (cpu_tree, node_assign) = build_cpu_tree(&bins, n, p, &grad, &hess, &col_map, params.max_depth, params.l2_reg);

            for i in 0..n {
                  if !row_mask[i] { continue; }
                  let node = node_assign[i] as usize;
                  pred[i] += params.learning_rate * cpu_tree.leaf_val[node];
            }

            trees.push(vec![cpu_tree]);

            if (t + 1) % 100 == 0 {
                  let mse: f64 = (0..n).map(|i| (pred[i] - y[i]).powi(2)).sum::<f64>() / n as f64;
                  eprintln!("      xgb reg iter={}/{} mse={:.4}", t + 1, params.n_estimators, mse);
            }
      }

      Ok(Model { trees, learning_rate: params.learning_rate, n_classes: 1, borders, n_features: p })
}

pub fn predict(model: &Model, x: &[f64], n: usize) -> Result<Vec<f64>, Error> {
      let p = model.n_features;
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }
      let bins = quantize_with_borders(x, n, p, &model.borders);
      let mut pred = vec![0.0f64; n];
      for round_trees in &model.trees {
            let tree = &round_trees[0];
            let leaves = route_tree(tree, &bins, n, p);
            for i in 0..n { pred[i] += model.learning_rate * tree.leaf_val[leaves[i] as usize]; }
      }
      Ok(pred)
}

pub fn train_multiclass(x: &[f64], y: &[usize], n: usize, p: usize, n_classes: usize, params: &Params) -> Result<Model, Error> {
      validate(x, n, p)?;
      if y.len() != n { return Err(Error::InvalidInput(format!("y.len()={} != n={}", y.len(), n))); }
      if n_classes < 2 { return Err(Error::InvalidInput("n_classes must be >= 2".into())); }
      if y.iter().any(|&v| v >= n_classes) { return Err(Error::InvalidInput("y contains value >= n_classes".into())); }

      let mut rng = ChaCha8Rng::seed_from_u64(params.seed);
      let (borders, bins) = quantize(x, n, p, params.n_bins);
      let mut logits = vec![0.0f64; n * n_classes];
      let mut trees: Vec<Vec<LevelTree>> = Vec::with_capacity(params.n_estimators);

      for t in 0..params.n_estimators {
            let row_mask = sample_mask(n, params.subsample, &mut rng);
            let col_map = sample_cols(p, params.colsample_bytree, &mut (ChaCha8Rng::seed_from_u64(params.seed ^ (t as u64 * 6364136223846793005 + 1))));

            let mut probs = logits.clone();
            softmax_inplace(&mut probs, n, n_classes);

            let mut round_trees = Vec::with_capacity(n_classes);

            for k in 0..n_classes {
                  let mut grad = vec![0.0f64; n];
                  let mut hess = vec![0.0f64; n];
                  for i in 0..n {
                        if !row_mask[i] { continue; }
                        let pk = probs[i * n_classes + k];
                        grad[i] = pk - if y[i] == k { 1.0 } else { 0.0 };
                        hess[i] = (pk * (1.0 - pk)).max(1e-6);
                  }

                  // GPU path not integrated: gpu_tree_build_into emits only per-sample tr_pred/te_pred, never the split_feat/split_bin/leaf_val tree the Model stores for predict() — true GPU integration requires gpu-core to return that tree structure.
                  let (cpu_tree, node_assign) = build_cpu_tree(&bins, n, p, &grad, &hess, &col_map, params.max_depth, params.l2_reg);

                  for i in 0..n {
                        if !row_mask[i] { continue; }
                        let node = node_assign[i] as usize;
                        logits[i * n_classes + k] += params.learning_rate * cpu_tree.leaf_val[node];
                  }
                  round_trees.push(cpu_tree);
            }
            trees.push(round_trees);

            if (t + 1) % 100 == 0 {
                  let mut probs_log = logits.clone();
                  softmax_inplace(&mut probs_log, n, n_classes);
                  let acc = (0..n).filter(|&i| {
                        (0..n_classes).max_by(|&a, &b| probs_log[i*n_classes+a].partial_cmp(&probs_log[i*n_classes+b]).unwrap()).unwrap() == y[i]
                  }).count();
                  eprintln!("      xgb mc iter={}/{} acc={:.3}", t + 1, params.n_estimators, acc as f64 / n as f64);
            }
      }

      Ok(Model { trees, learning_rate: params.learning_rate, n_classes, borders, n_features: p })
}

pub fn predict_proba(model: &Model, x: &[f64], n: usize) -> Result<Vec<f64>, Error> {
      let p = model.n_features;
      let nc = model.n_classes;
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }
      let bins = quantize_with_borders(x, n, p, &model.borders);
      let mut logits = vec![0.0f64; n * nc];
      for round_trees in &model.trees {
            for (k, tree) in round_trees.iter().enumerate() {
                  let leaves = route_tree(tree, &bins, n, p);
                  for i in 0..n { logits[i * nc + k] += model.learning_rate * tree.leaf_val[leaves[i] as usize]; }
            }
      }
      softmax_inplace(&mut logits, n, nc);
      Ok(logits)
}

#[cfg(test)]
mod tests {
      use super::*;

      #[test]
      fn test_regression_sanity() {
            let n = 100;
            let x: Vec<f64> = (0..n).map(|i| i as f64 / n as f64).collect();
            let y: Vec<f64> = x.iter().map(|&v| 2.0 * v + 1.0).collect();
            let params = Params {
                  n_estimators: 200, max_depth: 4, learning_rate: 0.1,
                  l2_reg: 0.1, min_child_weight: 1.0, subsample: 1.0,
                  colsample_bytree: 1.0, n_bins: 64, seed: 42,
            };
            let model = train(&x, &y, n, 1, &params).unwrap();
            let preds = predict(&model, &x, n).unwrap();
            let mae: f64 = (0..n).map(|i| (preds[i] - y[i]).abs()).sum::<f64>() / n as f64;
            assert!(mae < 0.05, "MAE={mae:.4} >= 0.05");
      }

      #[test]
      fn test_binary_classification() {
            let n = 200;
            let x: Vec<f64> = (0..n).map(|i| {
                  let v = i as f64 / n as f64;
                  if i < n / 2 { v * 0.4 } else { 0.6 + v * 0.4 }
            }).collect();
            let y: Vec<usize> = (0..n).map(|i| if i < n / 2 { 0 } else { 1 }).collect();
            let params = Params {
                  n_estimators: 100, max_depth: 3, learning_rate: 0.2,
                  l2_reg: 1.0, min_child_weight: 1.0, subsample: 1.0,
                  colsample_bytree: 1.0, n_bins: 32, seed: 0,
            };
            let model = train_multiclass(&x, &y, n, 1, 2, &params).unwrap();
            let probs = predict_proba(&model, &x, n).unwrap();
            let correct = (0..n).filter(|&i| {
                  let pred = if probs[i*2] > probs[i*2+1] { 0 } else { 1 };
                  pred == y[i]
            }).count();
            let acc = correct as f64 / n as f64;
            assert!(acc >= 0.95, "accuracy={acc:.3} < 0.95 ({correct}/{n})");
      }

      #[test]
      fn test_multiclass_3class() {
            let mut x = vec![];
            let mut y = vec![];
            for i in 0..40usize {
                  x.push(i as f64 * 0.1);
                  x.push(i as f64 * 0.05);
                  y.push(0usize);
            }
            for i in 0..40usize {
                  x.push(50.0 + i as f64 * 0.1);
                  x.push(50.0 + i as f64 * 0.05);
                  y.push(1usize);
            }
            for i in 0..40usize {
                  x.push(100.0 + i as f64 * 0.1);
                  x.push(100.0 + i as f64 * 0.05);
                  y.push(2usize);
            }
            let params = Params {
                  n_estimators: 200, max_depth: 3, learning_rate: 0.2,
                  l2_reg: 1.0, min_child_weight: 1.0, subsample: 1.0,
                  colsample_bytree: 1.0, n_bins: 64, seed: 7,
            };
            let model = train_multiclass(&x, &y, 120, 2, 3, &params).unwrap();
            let probs = predict_proba(&model, &x, 120).unwrap();
            let correct = (0..120).filter(|&i| {
                  (0..3).max_by(|&a, &b| probs[i*3+a].partial_cmp(&probs[i*3+b]).unwrap()).unwrap() == y[i]
            }).count();
            let acc = correct as f64 / 120.0;
            assert!(acc >= 0.7, "accuracy={acc:.3} < 0.7 ({correct}/120)");
      }

      #[test]
      fn test_invalid_inputs() {
            let x = vec![1.0, 2.0];
            let y = vec![1.0];
            assert!(train(&x, &y, 0, 2, &Params::default()).is_err());
            assert!(train(&x, &y, 1, 0, &Params::default()).is_err());
            assert!(train(&[1.0], &y, 1, 2, &Params::default()).is_err());
            assert!(train(&[f64::NAN], &[0.0], 1, 1, &Params::default()).is_err());
            let model = train(&x, &y, 1, 2, &Params { n_estimators: 1, ..Default::default() }).unwrap();
            assert!(predict(&model, &[1.0], 1).is_err());
            let yc = vec![0usize];
            let model2 = train_multiclass(&x, &yc, 1, 2, 2, &Params { n_estimators: 1, ..Default::default() }).unwrap();
            assert!(predict_proba(&model2, &[1.0], 1).is_err());
      }
}
