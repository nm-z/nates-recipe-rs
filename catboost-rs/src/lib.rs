use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use rand_distr::Exp1;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub enum Error {
      InvalidInput(String),
}
impl fmt::Display for Error {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self { Error::InvalidInput(s) => write!(f, "{s}") }
      }
}
impl std::error::Error for Error {}

pub struct Params {
      pub iterations: usize,
      pub depth: usize,
      pub learning_rate: f64,
      pub l2_reg: f64,
      pub border_count: usize,
      pub cat_features: Vec<usize>,
      pub n_permutations: usize,
      pub ts_prior: f64,
      pub seed: u64,
}

impl Default for Params {
      fn default() -> Self {
            Self {
                  iterations: 1000, depth: 6, learning_rate: 0.03, l2_reg: 3.0,
                  border_count: 254, cat_features: vec![], n_permutations: 4,
                  ts_prior: 1.0, seed: 42,
            }
      }
}

pub struct ObliviousTree {
      split_features: Vec<usize>,
      split_bins: Vec<usize>,
      leaf_values: Vec<Vec<f64>>,
      ts_borders: Vec<Vec<f64>>,
}

pub struct TsInfo {
      cat_feature: usize,
      greedy_ts: HashMap<u32, f64>,
      default_ts: f64,
}

pub struct Model {
      pub trees: Vec<ObliviousTree>,
      pub learning_rate: f64,
      pub n_classes: usize,
      pub n_orig_features: usize,
      borders: Vec<Vec<f64>>,
      cat_features: Vec<usize>,
      cat_mappings: Vec<HashMap<i64, u32>>,
      ts_infos: Vec<TsInfo>,
}

fn validate_train(x: &[f64], y: &[usize], n: usize, p: usize, nc: usize, params: &Params) -> Result<(), Error> {
      if n == 0 { return Err(Error::InvalidInput("n must be > 0".into())); }
      if p == 0 { return Err(Error::InvalidInput("p must be > 0".into())); }
      if nc < 2 { return Err(Error::InvalidInput("n_classes must be >= 2".into())); }
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }
      if y.len() != n { return Err(Error::InvalidInput(format!("y.len()={} != n={}", y.len(), n))); }
      if params.n_permutations == 0 { return Err(Error::InvalidInput("n_permutations must be > 0".into())); }
      for &c in &params.cat_features {
            if c >= p { return Err(Error::InvalidInput(format!("cat_feature {c} >= p={p}"))); }
      }
      if y.iter().any(|&v| v >= nc) { return Err(Error::InvalidInput("y contains value >= n_classes".into())); }
      for (i, &v) in x.iter().enumerate() {
            if v.is_nan() { return Err(Error::InvalidInput(format!("x[{i}] is NaN"))); }
      }
      let cat_set: std::collections::HashSet<usize> = params.cat_features.iter().cloned().collect();
      for &cat_j in &params.cat_features {
            for i in 0..n {
                  let v = x[i * p + cat_j];
                  if v != (v as i64) as f64 {
                        return Err(Error::InvalidInput(format!("categorical x[{},{}]={v} is not an exact integer", i, cat_j)));
                  }
            }
      }
      Ok(())
}

fn quantize(
      x: &[f64], n: usize, p: usize, border_count: usize, cat_features: &[usize],
) -> (Vec<Vec<u32>>, Vec<Vec<f64>>, Vec<HashMap<i64, u32>>) {
      let cat_set: std::collections::HashSet<usize> = cat_features.iter().cloned().collect();
      let mut bins = vec![vec![0u32; n]; p];
      let mut borders = vec![vec![]; p];
      let mut cat_mappings: Vec<HashMap<i64, u32>> = vec![HashMap::new(); p];
      for j in 0..p {
            if cat_set.contains(&j) {
                  let mut next_id = 0u32;
                  for i in 0..n {
                        let v = x[i * p + j] as i64;
                        let id = *cat_mappings[j].entry(v).or_insert_with(|| { let id = next_id; next_id += 1; id });
                        bins[j][i] = id;
                  }
            } else {
                  let mut col: Vec<f64> = (0..n).map(|i| x[i * p + j]).collect();
                  col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                  let step = (n as f64 / border_count as f64).max(1.0);
                  let mut b: Vec<f64> = vec![];
                  let mut idx = step;
                  while (idx as usize) < n {
                        let v = col[idx as usize];
                        if b.is_empty() || (v - b[b.len() - 1]).abs() > 1e-12_f64 { b.push(v); }
                        idx += step;
                  }
                  for i in 0..n { bins[j][i] = b.partition_point(|&t| t <= x[i * p + j]) as u32; }
                  borders[j] = b;
            }
      }
      (bins, borders, cat_mappings)
}

fn quantize_predict(
      x: &[f64], n: usize, p: usize, borders: &[Vec<f64>],
      cat_features: &[usize], cat_mappings: &[HashMap<i64, u32>],
) -> Vec<Vec<u32>> {
      let cat_set: std::collections::HashSet<usize> = cat_features.iter().cloned().collect();
      (0..p).map(|j| {
            if cat_set.contains(&j) {
                  let map = &cat_mappings[j];
                  let fallback = map.len() as u32;
                  (0..n).map(|i| *map.get(&(x[i * p + j] as i64)).unwrap_or(&fallback)).collect()
            } else {
                  let b = &borders[j];
                  (0..n).map(|i| b.partition_point(|&t| t <= x[i * p + j]) as u32).collect()
            }
      }).collect()
}

fn ordered_target_stats(
      y: &[usize], cat_bins: &[u32], perm: &[usize], n_cats: usize,
      class_k: usize, prior: f64, class_prior: f64,
) -> Vec<f64> {
      let n = y.len();
      let mut count = vec![0.0f64; n_cats + 1];
      let mut sum = vec![0.0f64; n_cats + 1];
      let mut result = vec![0.0f64; n];
      for &idx in perm {
            let c = cat_bins[idx] as usize;
            result[idx] = (sum[c] + prior * class_prior) / (count[c] + prior);
            count[c] += 1.0;
            sum[c] += if y[idx] == class_k { 1.0 } else { 0.0 };
      }
      result
}

fn greedy_target_stats(
      y: &[usize], cat_bins: &[u32], n_cats: usize,
      class_k: usize, prior: f64, class_prior: f64,
) -> HashMap<u32, f64> {
      let mut count = vec![0.0f64; n_cats + 1];
      let mut sum = vec![0.0f64; n_cats + 1];
      for (i, &c) in cat_bins.iter().enumerate() {
            count[c as usize] += 1.0;
            sum[c as usize] += if y[i] == class_k { 1.0 } else { 0.0 };
      }
      (0..=n_cats as u32)
            .filter(|&c| count[c as usize] > 0.0)
            .map(|c| (c, (sum[c as usize] + prior * class_prior) / (count[c as usize] + prior)))
            .collect()
}

fn quantize_float_col(values: &[f64], n: usize, border_count: usize) -> (Vec<u32>, Vec<f64>) {
      let mut sorted = values.to_vec();
      sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
      let step = (n as f64 / border_count as f64).max(1.0);
      let mut b: Vec<f64> = vec![];
      let mut idx = step;
      while (idx as usize) < n {
            let v = sorted[idx as usize];
            if b.is_empty() || (v - b[b.len() - 1]).abs() > 1e-12_f64 { b.push(v); }
            idx += step;
      }
      let binned = values.iter().map(|&v| b.partition_point(|&t| t <= v) as u32).collect();
      (binned, b)
}

#[derive(Clone)]
struct HistBucket { grad_sum: f64, hess_sum: f64 }

fn leaf_score(g: f64, h: f64, l2: f64) -> f64 { (g * g) / (h + l2) }
fn leaf_weight(g: f64, h: f64, l2: f64) -> f64 { -g / (h + l2) }

fn softmax_cpu(logits: &[f64], n: usize, nc: usize) -> Vec<f64> {
      let mut probs = vec![0.0f64; n * nc];
      for i in 0..n {
            let row = &logits[i * nc..(i + 1) * nc];
            let mx = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let s: f64 = row.iter().map(|&v| (v - mx).exp()).sum();
            for k in 0..nc { probs[i * nc + k] = (row[k] - mx).exp() / s; }
      }
      probs
}

fn build_oblivious_tree(
      bins: &[&[u32]], grads: &[f64], hesses: &[f64],
      n: usize, n_features: usize, n_classes: usize, depth: usize, l2: f64,
) -> (ObliviousTree, Vec<u32>) {
      let n_leaves = 1usize << depth;
      let mut leaf_indices = vec![0u32; n];
      let mut split_features = Vec::with_capacity(depth);
      let mut split_bins_vec = Vec::with_capacity(depth);

      for d in 0..depth {
            let n_nodes = 1u32 << d;
            let mut best_feat = 0;
            let mut best_bin = 0;
            let mut best_gain = f64::NEG_INFINITY;

            for feat in 0..n_features {
                  let fb = bins[feat];
                  let n_bins = fb.iter().max().map(|&v| v as usize + 1).unwrap_or(1);
                  let mut gain_per_bin = vec![0.0f64; n_bins];
                  let mut has_valid = vec![false; n_bins];

                  for k in 0..n_classes {
                        let mut leaf_hists = vec![vec![HistBucket { grad_sum: 0.0, hess_sum: 0.0 }; n_bins]; n_nodes as usize];
                        for i in 0..n {
                              let leaf = leaf_indices[i] as usize;
                              let b = fb[i] as usize;
                              leaf_hists[leaf][b].grad_sum += grads[i * n_classes + k];
                              leaf_hists[leaf][b].hess_sum += hesses[i * n_classes + k];
                        }
                        for node in 0..n_nodes as usize {
                              let hist = &leaf_hists[node];
                              let tg: f64 = hist.iter().map(|h| h.grad_sum).sum();
                              let th: f64 = hist.iter().map(|h| h.hess_sum).sum();
                              if th < 1e-10 { continue; }
                              let ps = leaf_score(tg, th, l2);
                              let mut lg = 0.0;
                              let mut lh = 0.0;
                              for (bin, bucket) in hist.iter().enumerate() {
                                    lg += bucket.grad_sum;
                                    lh += bucket.hess_sum;
                                    let rg = tg - lg;
                                    let rh = th - lh;
                                    if rh < 1e-10 || lh < 1e-10 { continue; }
                                    gain_per_bin[bin] += leaf_score(lg, lh, l2)
                                          + leaf_score(rg, rh, l2) - ps;
                                    has_valid[bin] = true;
                              }
                        }
                  }
                  for (bin, &gain) in gain_per_bin.iter().enumerate() {
                        if has_valid[bin] && gain > best_gain {
                              best_gain = gain; best_feat = feat; best_bin = bin;
                        }
                  }
            }
            split_features.push(best_feat);
            split_bins_vec.push(best_bin);
            for i in 0..n {
                  leaf_indices[i] = (leaf_indices[i] << 1) | ((bins[best_feat][i] as usize > best_bin) as u32);
            }
      }

      let mut leaf_values: Vec<Vec<f64>> = Vec::with_capacity(n_classes);
      for k in 0..n_classes {
            let mut lg = vec![0.0f64; n_leaves];
            let mut lh = vec![0.0f64; n_leaves];
            for i in 0..n {
                  lg[leaf_indices[i] as usize] += grads[i * n_classes + k];
                  lh[leaf_indices[i] as usize] += hesses[i * n_classes + k];
            }
            leaf_values.push((0..n_leaves).map(|j| leaf_weight(lg[j], lh[j], l2)).collect());
      }
      (ObliviousTree { split_features, split_bins: split_bins_vec, leaf_values, ts_borders: vec![] }, leaf_indices)
}

fn compute_ts_floats(
      y: &[usize], bins: &[Vec<u32>], perm: &[usize], cat_features: &[usize],
      n_classes: usize, n: usize, prior: f64, class_priors: &[f64],
) -> Vec<Vec<f64>> {
      let mut ts_floats = Vec::new();
      for &cat_j in cat_features {
            let n_cats = *bins[cat_j].iter().max().unwrap_or(&0) as usize + 1;
            for k in 0..n_classes {
                  ts_floats.push(ordered_target_stats(y, &bins[cat_j], perm, n_cats, k, prior, class_priors[k]));
            }
      }
      ts_floats
}

fn bin_ts_floats(ts_floats: &[Vec<f64>], borders: &[Vec<f64>]) -> Vec<Vec<u32>> {
      ts_floats.iter().zip(borders).map(|(vals, bord)| {
            vals.iter().map(|&v| bord.partition_point(|&t| t <= v) as u32).collect()
      }).collect()
}

fn compute_ts_bins_fresh(
      y: &[usize], bins: &[Vec<u32>], perm: &[usize], cat_features: &[usize],
      n_classes: usize, n: usize, prior: f64, class_priors: &[f64], border_count: usize,
) -> (Vec<Vec<u32>>, Vec<Vec<f64>>) {
      let ts_floats = compute_ts_floats(y, bins, perm, cat_features, n_classes, n, prior, class_priors);
      let mut ts_columns = Vec::new();
      let mut ts_borders = Vec::new();
      for vals in &ts_floats {
            let (binned, bord) = quantize_float_col(vals, n, border_count);
            ts_columns.push(binned);
            ts_borders.push(bord);
      }
      (ts_columns, ts_borders)
}

fn route_through_tree(
      tree: &ObliviousTree, orig_bins: &[Vec<u32>], ts_columns: &[Vec<u32>],
      p: usize, n: usize,
) -> Vec<u32> {
      let mut leaves = vec![0u32; n];
      for i in 0..n {
            let mut leaf = 0u32;
            for (&feat, &bin) in tree.split_features.iter().zip(&tree.split_bins) {
                  let sample_bin = if feat < p { orig_bins[feat][i] as usize } else { ts_columns[feat - p][i] as usize };
                  leaf = (leaf << 1) | ((sample_bin > bin) as u32);
            }
            leaves[i] = leaf;
      }
      leaves
}

fn gen_permutations(n: usize, count: usize, rng: &mut ChaCha8Rng) -> Vec<Vec<usize>> {
      (0..count).map(|_| { let mut p: Vec<usize> = (0..n).collect(); p.shuffle(rng); p }).collect()
}

pub fn train(
      x: &[f64], y: &[usize], n: usize, p: usize, n_classes: usize, params: &Params,
) -> Result<Model, Error> {
      validate_train(x, y, n, p, n_classes, params)?;
      let mut rng = ChaCha8Rng::seed_from_u64(params.seed);
      let (bins, borders, cat_mappings) = quantize(x, n, p, params.border_count, &params.cat_features);
      let perms = gen_permutations(n, params.n_permutations, &mut rng);
      let class_priors: Vec<f64> = (0..n_classes).map(|k| y.iter().filter(|&&c| c == k).count() as f64 / n as f64).collect();

      let s = params.n_permutations;
      let mut support_logits: Vec<Vec<f64>> = (0..s).map(|_| vec![0.0f64; n * n_classes]).collect();
      let mut avg_logits = vec![0.0f64; n * n_classes];
      let mut trees = Vec::with_capacity(params.iterations);

      for t in 0..params.iterations {
            let perm_idx = t % s;
            let perm = &perms[perm_idx];

            let (ts_columns, ts_bord) = compute_ts_bins_fresh(
                  y, &bins, perm, &params.cat_features,
                  n_classes, n, params.ts_prior, &class_priors, params.border_count,
            );
            let n_total = p + ts_columns.len();
            let mut all_bin_refs: Vec<&[u32]> = bins.iter().map(|v| v.as_slice()).collect();
            for col in &ts_columns { all_bin_refs.push(col.as_slice()); }

            let bb: Vec<f64> = {
                  let mut w: Vec<f64> = (0..n).map(|_| rng.sample::<f64, _>(Exp1)).collect();
                  let sum: f64 = w.iter().sum();
                  let scale = n as f64 / sum;
                  w.iter_mut().for_each(|v| *v *= scale);
                  w
            };

            let probs = softmax_cpu(&support_logits[perm_idx], n, n_classes);
            let mut grads = vec![0.0f64; n * n_classes];
            let mut hesses = vec![0.0f64; n * n_classes];
            for i in 0..n {
                  for k in 0..n_classes {
                        let pk = probs[i * n_classes + k];
                        grads[i * n_classes + k] = (pk - if y[i] == k { 1.0 } else { 0.0 }) * bb[i];
                        hesses[i * n_classes + k] = (pk * (1.0 - pk)).max(1e-6) * bb[i];
                  }
            }

            let (mut tree, leaf_indices) = build_oblivious_tree(
                  &all_bin_refs, &grads, &hesses, n, n_total, n_classes, params.depth, params.l2_reg,
            );
            tree.ts_borders = ts_bord;

            let n_leaves = 1usize << params.depth;
            {
                  let avg_probs = softmax_cpu(&avg_logits, n, n_classes);
                  let mut new_lv: Vec<Vec<f64>> = Vec::with_capacity(n_classes);
                  for k in 0..n_classes {
                        let mut lg = vec![0.0f64; n_leaves];
                        let mut lh = vec![0.0f64; n_leaves];
                        for i in 0..n {
                              let pk = avg_probs[i * n_classes + k];
                              lg[leaf_indices[i] as usize] += (pk - if y[i] == k { 1.0 } else { 0.0 }) * bb[i];
                              lh[leaf_indices[i] as usize] += (pk * (1.0 - pk)).max(1e-6) * bb[i];
                        }
                        new_lv.push((0..n_leaves).map(|j| leaf_weight(lg[j], lh[j], params.l2_reg)).collect());
                  }
                  tree.leaf_values = new_lv;
            }
            for i in 0..n {
                  let leaf = leaf_indices[i] as usize;
                  for k in 0..n_classes {
                        avg_logits[i * n_classes + k] += params.learning_rate * tree.leaf_values[k][leaf];
                  }
            }

            for r in 0..s {
                  let rperm = &perms[r];
                  let r_ts_floats = compute_ts_floats(y, &bins, rperm, &params.cat_features, n_classes, n, params.ts_prior, &class_priors);
                  let r_ts_cols = bin_ts_floats(&r_ts_floats, &tree.ts_borders);
                  let r_leaf_indices = route_through_tree(&tree, &bins, &r_ts_cols, p, n);

                  let r_probs = softmax_cpu(&support_logits[r], n, n_classes);
                  let mut r_grads = vec![0.0f64; n * n_classes];
                  let mut r_hesses = vec![0.0f64; n * n_classes];
                  for i in 0..n {
                        for k in 0..n_classes {
                              let pk = r_probs[i * n_classes + k];
                              r_grads[i * n_classes + k] = (pk - if y[i] == k { 1.0 } else { 0.0 }) * bb[i];
                              r_hesses[i * n_classes + k] = (pk * (1.0 - pk)).max(1e-6) * bb[i];
                        }
                  }

                  let mut prefix_lg = vec![vec![0.0f64; n_leaves]; n_classes];
                  let mut prefix_lh = vec![vec![0.0f64; n_leaves]; n_classes];
                  for &idx in rperm {
                        let leaf = r_leaf_indices[idx] as usize;
                        for k in 0..n_classes {
                              let w = if prefix_lh[k][leaf] > 1e-10 {
                                    leaf_weight(prefix_lg[k][leaf], prefix_lh[k][leaf], params.l2_reg)
                              } else { 0.0 };
                              support_logits[r][idx * n_classes + k] += params.learning_rate * w;
                        }
                        for k in 0..n_classes {
                              prefix_lg[k][leaf] += r_grads[idx * n_classes + k];
                              prefix_lh[k][leaf] += r_hesses[idx * n_classes + k];
                        }
                  }
            }
            trees.push(tree);
            if (t + 1) % 1 == 0 {
                  eprintln!("      cb iter={}/{}", t + 1, params.iterations);
            }
      }

      let mut ts_infos = Vec::new();
      for &cat_j in &params.cat_features {
            let n_cats = *bins[cat_j].iter().max().unwrap_or(&0) as usize + 1;
            for k in 0..n_classes {
                  let greedy = greedy_target_stats(y, &bins[cat_j], n_cats, k, params.ts_prior, class_priors[k]);
                  ts_infos.push(TsInfo { cat_feature: cat_j, greedy_ts: greedy, default_ts: class_priors[k] });
            }
      }

      Ok(Model {
            trees, learning_rate: params.learning_rate, n_classes, n_orig_features: p,
            borders, cat_features: params.cat_features.clone(), cat_mappings, ts_infos,
      })
}

pub fn predict(model: &Model, x: &[f64], n: usize) -> Result<Vec<f64>, Error> {
      let p = model.n_orig_features;
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }

      let orig_bins = quantize_predict(x, n, p, &model.borders, &model.cat_features, &model.cat_mappings);

      let ts_float_vals: Vec<Vec<f64>> = model.ts_infos.iter().map(|info| {
            let cat_bins = &orig_bins[info.cat_feature];
            cat_bins.iter().map(|&c| *info.greedy_ts.get(&c).unwrap_or(&info.default_ts)).collect()
      }).collect();

      let nc = model.n_classes;
      let mut logits = vec![0.0f64; n * nc];
      for tree in &model.trees {
            let ts_columns: Vec<Vec<u32>> = if tree.ts_borders.is_empty() {
                  vec![]
            } else {
                  ts_float_vals.iter().enumerate().map(|(ti, vals)| {
                        let bord = &tree.ts_borders[ti];
                        vals.iter().map(|&v| bord.partition_point(|&t| t <= v) as u32).collect()
                  }).collect()
            };
            let leaves = route_through_tree(tree, &orig_bins, &ts_columns, p, n);
            for i in 0..n {
                  let leaf = leaves[i] as usize;
                  for k in 0..nc { logits[i * nc + k] += model.learning_rate * tree.leaf_values[k][leaf]; }
            }
      }
      Ok(softmax_cpu(&logits, n, nc))
}

#[cfg(test)]
mod tests {
      use super::*;

      #[test]
      fn test_sanity_depth1() {
            let x = vec![0.1, 0.2, 0.8, 0.9];
            let y = vec![0, 0, 1, 1];
            let params = Params { iterations: 20, depth: 1, learning_rate: 0.5, border_count: 4, ..Default::default() };
            let model = train(&x, &y, 4, 1, 2, &params).unwrap();
            let probs = predict(&model, &x, 4).unwrap();
            for i in 0..4 {
                  let pred = if probs[i * 2] > probs[i * 2 + 1] { 0 } else { 1 };
                  assert_eq!(pred, y[i], "sample {i} wrong");
            }
      }

      #[test]
      fn test_overfit_binary() {
            let x = vec![
                  0.1, 0.2, 0.2, 0.3, 0.15, 0.25, 0.3, 0.1,
                  0.8, 0.9, 0.9, 0.7, 0.85, 0.8, 0.7, 0.95,
                  0.25, 0.15, 0.35, 0.2, 0.75, 0.85, 0.65, 0.9,
            ];
            let y = vec![0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1];
            let params = Params { iterations: 100, depth: 3, learning_rate: 0.3, border_count: 8, ..Default::default() };
            let model = train(&x, &y, 12, 2, 2, &params).unwrap();
            let probs = predict(&model, &x, 12).unwrap();
            let correct = (0..12).filter(|&i| (if probs[i*2] > probs[i*2+1] { 0 } else { 1 }) == y[i]).count();
            assert!(correct >= 10, "only {correct}/12 correct");
      }

      #[test]
      fn test_multiclass_predict() {
            let mut x = vec![]; let mut y = vec![];
            for i in 0..120 { x.push(i as f64); x.push((i * 2) as f64); y.push(i % 3); }
            let params = Params { iterations: 200, depth: 3, learning_rate: 0.3, l2_reg: 0.5, n_permutations: 2, ..Default::default() };
            let model = train(&x, &y, 120, 2, 3, &params).unwrap();
            let probs = predict(&model, &x, 120).unwrap();
            let correct = (0..120).filter(|&i| (0..3).max_by(|&a, &b| probs[i*3+a].partial_cmp(&probs[i*3+b]).unwrap()).unwrap() == y[i]).count();
            assert!(correct >= 50, "only {correct}/120 correct");
      }

      #[test]
      fn test_categorical_roundtrip() {
            let x = vec![10.0, 0.1, 10.0, 0.2, 20.0, 0.3, 20.0, 0.4, 30.0, 0.5, 30.0, 0.6, 10.0, 0.7, 20.0, 0.8];
            let y = vec![0, 0, 1, 1, 0, 0, 0, 1];
            let params = Params { iterations: 50, depth: 2, learning_rate: 0.3, border_count: 8, cat_features: vec![0], ..Default::default() };
            let model = train(&x, &y, 8, 2, 2, &params).unwrap();
            let probs = predict(&model, &x, 8).unwrap();
            let correct = (0..8).filter(|&i| (if probs[i*2] > probs[i*2+1] { 0 } else { 1 }) == y[i]).count();
            assert!(correct >= 6, "cat roundtrip: only {correct}/8 correct");
            let probs_unseen = predict(&model, &[99.0, 0.5], 1).unwrap();
            assert!((probs_unseen[0] + probs_unseen[1] - 1.0).abs() < 1e-6);
      }

      #[test]
      fn test_ts_feature_split_roundtrip() {
            let mut rng = ChaCha8Rng::seed_from_u64(999);
            let n = 40;
            let mut x = vec![]; let mut y = vec![];
            for i in 0..n {
                  x.push(if i < n / 2 { 0.0 } else { 1.0 });
                  x.push(rng.random::<f64>());
                  y.push(if i < n / 2 { 0 } else { 1 });
            }
            let params = Params { iterations: 50, depth: 2, learning_rate: 0.3, border_count: 8, cat_features: vec![0], n_permutations: 2, ..Default::default() };
            let model = train(&x, &y, n, 2, 2, &params).unwrap();
            assert!(model.trees.iter().any(|t| t.split_features.iter().any(|&f| f >= 2)), "no TS split");
            let probs = predict(&model, &x, n).unwrap();
            let correct = (0..n).filter(|&i| (if probs[i*2] > probs[i*2+1] { 0 } else { 1 }) == y[i]).count();
            assert!(correct >= 35, "TS roundtrip: only {correct}/{n} correct");
      }

      #[test]
      fn test_ordered_boosting_divergent_models() {
            let mut x = vec![]; let mut y = vec![];
            for i in 0..20 { x.push(i as f64); x.push((i * 3) as f64); y.push(if i < 10 { 0 } else { 1 }); }
            let params = Params { iterations: 20, depth: 2, learning_rate: 0.3, n_permutations: 2, border_count: 8, ..Default::default() };
            let model = train(&x, &y, 20, 2, 2, &params).unwrap();
            let probs = predict(&model, &x, 20).unwrap();
            let correct = (0..20).filter(|&i| (if probs[i*2] > probs[i*2+1] { 0 } else { 1 }) == y[i]).count();
            assert!(correct >= 16, "ordered boosting: only {correct}/20 correct");
      }

      #[test]
      fn test_invalid_inputs() {
            let x = vec![1.0, 2.0];
            let y = vec![0];
            assert!(train(&x, &y, 1, 2, 2, &Params { n_permutations: 0, ..Default::default() }).is_err());
            assert!(train(&x, &y, 1, 2, 2, &Params { cat_features: vec![5], ..Default::default() }).is_err());
            assert!(train(&[f64::NAN], &[0], 1, 1, 2, &Params::default()).is_err());
            assert!(train(&[1.5], &[0], 1, 1, 2, &Params { cat_features: vec![0], ..Default::default() }).is_err());
            let model = train(&x, &y, 1, 2, 2, &Params { iterations: 1, depth: 1, ..Default::default() }).unwrap();
            assert!(predict(&model, &[1.0], 1).is_err());
      }
}
