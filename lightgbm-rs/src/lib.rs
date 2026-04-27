use gpu_core::memory::GpuBuffer;
use gpu_core::hip::HipError;
use gpu_core::kernels::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::fmt;

#[derive(Debug)]
pub enum Error {
      InvalidInput(String),
      Gpu(HipError),
}
impl fmt::Display for Error {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                  Error::InvalidInput(s) => write!(f, "{s}"),
                  Error::Gpu(e) => write!(f, "GPU error: {e}"),
            }
      }
}
impl std::error::Error for Error {}
impl From<HipError> for Error {
      fn from(e: HipError) -> Self { Error::Gpu(e) }
}

pub struct Params {
      pub n_estimators: usize,
      pub num_leaves: usize,
      pub max_depth: usize,
      pub learning_rate: f64,
      pub l2_reg: f64,
      pub min_child_weight: f64,
      pub min_gain_to_split: f64,
      pub n_bins: usize,
      pub goss_a: f64,
      pub goss_b: f64,
      pub use_efb: bool,
      pub efb_max_conflict: f64,
      pub seed: u64,
}

impl Default for Params {
      fn default() -> Self {
            Self {
                  n_estimators: 100, num_leaves: 31, max_depth: 0,
                  learning_rate: 0.1, l2_reg: 1.0, min_child_weight: 1.0,
                  min_gain_to_split: 0.0, n_bins: 255,
                  goss_a: 0.0, goss_b: 0.3,
                  use_efb: false, efb_max_conflict: 0.0,
                  seed: 42,
            }
      }
}

#[derive(Clone)]
pub struct SplitNode {
      pub feature: usize,
      pub bin: u8,
      pub left_child: usize,
      pub right_child: usize,
      pub leaf_value: f32,
      pub is_leaf: bool,
}

pub struct Tree {
      pub nodes: Vec<SplitNode>,
}

pub struct Model {
      pub trees: Vec<Vec<Tree>>,
      pub learning_rate: f64,
      pub n_classes: usize,
      pub n_orig_features: usize,
      pub borders: Vec<Vec<f32>>,
      pub bundle_map: Vec<(usize, u8)>,
      pub n_eff_features: usize,
}

fn validate_regression(x: &[f64], y: &[f64], n: usize, p: usize, params: &Params) -> Result<(), Error> {
      if n == 0 { return Err(Error::InvalidInput("n must be > 0".into())); }
      if p == 0 { return Err(Error::InvalidInput("p must be > 0".into())); }
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }
      if y.len() != n { return Err(Error::InvalidInput(format!("y.len()={} != n={}", y.len(), n))); }
      if params.num_leaves < 2 { return Err(Error::InvalidInput("num_leaves must be >= 2".into())); }
      if params.num_leaves > 255 { return Err(Error::InvalidInput("num_leaves must be <= 255".into())); }
      if params.n_bins < 2 || params.n_bins > 255 { return Err(Error::InvalidInput("n_bins must be in [2,255]".into())); }
      if params.goss_a < 0.0 || params.goss_a >= 1.0 { return Err(Error::InvalidInput("goss_a must be in [0,1)".into())); }
      if params.goss_b <= 0.0 || params.goss_b > 1.0 { return Err(Error::InvalidInput("goss_b must be in (0,1]".into())); }
      if params.goss_a > 0.0 && params.goss_a + params.goss_b > 1.0 {
            return Err(Error::InvalidInput("goss_a + goss_b must be <= 1.0".into()));
      }
      Ok(())
}

fn validate_classification(x: &[f64], y: &[usize], n: usize, p: usize, n_classes: usize, params: &Params) -> Result<(), Error> {
      if n == 0 { return Err(Error::InvalidInput("n must be > 0".into())); }
      if p == 0 { return Err(Error::InvalidInput("p must be > 0".into())); }
      if n_classes < 2 { return Err(Error::InvalidInput("n_classes must be >= 2".into())); }
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }
      if y.len() != n { return Err(Error::InvalidInput(format!("y.len()={} != n={}", y.len(), n))); }
      if y.iter().any(|&v| v >= n_classes) { return Err(Error::InvalidInput("y contains value >= n_classes".into())); }
      if params.num_leaves < 2 { return Err(Error::InvalidInput("num_leaves must be >= 2".into())); }
      if params.num_leaves > 255 { return Err(Error::InvalidInput("num_leaves must be <= 255".into())); }
      if params.n_bins < 2 || params.n_bins > 255 { return Err(Error::InvalidInput("n_bins must be in [2,255]".into())); }
      if params.goss_a < 0.0 || params.goss_a >= 1.0 { return Err(Error::InvalidInput("goss_a must be in [0,1)".into())); }
      if params.goss_b <= 0.0 || params.goss_b > 1.0 { return Err(Error::InvalidInput("goss_b must be in (0,1]".into())); }
      if params.goss_a > 0.0 && params.goss_a + params.goss_b > 1.0 {
            return Err(Error::InvalidInput("goss_a + goss_b must be <= 1.0".into()));
      }
      Ok(())
}

fn compute_borders(x: &[f64], n: usize, p: usize, n_bins: usize) -> Vec<Vec<f32>> {
      (0..p).map(|j| {
            let mut col: Vec<f32> = (0..n).map(|i| x[i * p + j] as f32).collect();
            col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            col.dedup();
            if col.len() <= 1 { return vec![col.last().copied().unwrap_or(0.0)]; }
            let step = (col.len() as f64 / n_bins as f64).max(1.0);
            let mut borders = Vec::with_capacity(n_bins);
            let mut idx = step;
            while (idx as usize) < col.len() {
                  borders.push(col[idx as usize]);
                  idx += step;
            }
            if borders.is_empty() || borders.last() != col.last() {
                  if let Some(&last) = col.last() { borders.push(last); }
            }
            borders
      }).collect()
}

fn quantize(x: &[f64], n: usize, p: usize, borders: &[Vec<f32>]) -> Vec<Vec<u8>> {
      (0..p).map(|j| {
            let b = &borders[j];
            (0..n).map(|i| b.partition_point(|&t| t < x[i * p + j] as f32).min(b.len()).min(255) as u8).collect()
      }).collect()
}

fn efb_bundle(bins: &[Vec<u8>], n: usize, p: usize, max_conflict: f64) -> (Vec<(usize, u8)>, Vec<Vec<u8>>, usize) {
      let nonzero_counts: Vec<usize> = (0..p).map(|j| bins[j].iter().filter(|&&b| b > 0).count()).collect();
      let mut order: Vec<usize> = (0..p).collect();
      order.sort_by(|&a, &b| nonzero_counts[b].cmp(&nonzero_counts[a]));

      let max_conflict_count = (n as f64 * max_conflict) as usize;
      let mut bundle_map: Vec<(usize, u8)> = vec![(0, 0); p];
      let mut bundles: Vec<Vec<usize>> = Vec::new();
      let mut bundle_max_bin: Vec<u8> = Vec::new();

      for &j in &order {
            let max_bin_j = bins[j].iter().copied().max().unwrap_or(0);
            let mut placed = false;
            for bid in 0..bundles.len() {
                  let conflicts: usize = bundles[bid].iter().map(|&other| {
                        bins[j].iter().zip(bins[other].iter()).filter(|&(&bj, &bo)| bj > 0 && bo > 0).count()
                  }).sum();
                  let offset = bundle_max_bin[bid];
                  let needed = offset as usize + max_bin_j as usize + 1;
                  if conflicts <= max_conflict_count && needed <= 255 {
                        bundle_map[j] = (bid, offset + 1);
                        bundle_max_bin[bid] = (offset + 1 + max_bin_j).min(255);
                        bundles[bid].push(j);
                        placed = true;
                        break;
                  }
            }
            if !placed {
                  bundle_map[j] = (bundles.len(), 0);
                  bundle_max_bin.push(max_bin_j);
                  bundles.push(vec![j]);
            }
      }

      let n_bundled = bundles.len();
      let mut bundled_bins: Vec<Vec<u8>> = vec![vec![0u8; n]; n_bundled];
      for j in 0..p {
            let (bid, offset) = bundle_map[j];
            for i in 0..n {
                  if bins[j][i] > 0 {
                        bundled_bins[bid][i] = (bins[j][i] as u16 + offset as u16).min(255) as u8;
                  }
            }
      }
      (bundle_map, bundled_bins, n_bundled)
}

fn apply_bundle_map_predict(bins_raw: &[Vec<u8>], n: usize, p: usize, bundle_map: &[(usize, u8)], n_bundled: usize) -> Vec<Vec<u8>> {
      let mut bundled = vec![vec![0u8; n]; n_bundled];
      for j in 0..p {
            let (bid, offset) = bundle_map[j];
            for i in 0..n {
                  if bins_raw[j][i] > 0 {
                        bundled[bid][i] = (bins_raw[j][i] as u16 + offset as u16).min(255) as u8;
                  }
            }
      }
      bundled
}

fn mse_grad_hess(pred: &[f32], y: &[f32], n: usize) -> (Vec<f32>, Vec<f32>) {
      let mut grad = vec![0.0f32; n];
      let hess = vec![1.0f32; n];
      for i in 0..n { grad[i] = pred[i] - y[i]; }
      (grad, hess)
}

fn mc_grad_hess_row(logits: &[f32], y_class: usize, _n_classes: usize) -> (f32, f32) {
      let mx = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
      let s: f32 = logits.iter().map(|&v| (v - mx).exp()).sum();
      let prob_k = (logits[y_class] - mx).exp() / s;
      let g = prob_k - 1.0;
      let h = (prob_k * (1.0 - prob_k)).max(1e-6f32);
      (g, h)
}

fn apply_goss(grad: &mut [f32], hess: &mut [f32], n: usize, goss_a: f64, goss_b: f64, rng: &mut ChaCha8Rng) -> Result<(), HipError> {
      let top_k = (n as f64 * goss_a).ceil() as usize;
      let keep_weight = ((1.0 - goss_a) / goss_b) as f32;

      let abs_grad: Vec<f32> = grad.iter().map(|&g| g.abs()).collect();
      let mut sorted_idx: Vec<i32> = (0..n as i32).collect();
      sorted_idx.sort_by(|&a, &b| abs_grad[b as usize].partial_cmp(&abs_grad[a as usize]).unwrap_or(std::cmp::Ordering::Equal));
      let rand_vals: Vec<f32> = (0..n).map(|_| rng.random::<f32>()).collect();

      let sorted_gpu = GpuBuffer::upload_i32(&sorted_idx)?;
      let rand_gpu = GpuBuffer::upload_f32(&rand_vals)?;
      let weights_gpu = GpuBuffer::zeros_f32(n)?;

      gpu_goss_sample(&sorted_gpu, &weights_gpu, &rand_gpu, n, top_k, keep_weight);

      let mut weights = vec![0.0f32; n];
      weights_gpu.download_f32(&mut weights)?;
      for i in 0..n {
            grad[i] *= weights[i];
            hess[i] *= weights[i];
      }
      Ok(())
}

fn build_leaf_wise_tree(
      bins_eff: &[Vec<u8>],
      grad_h: &[f32],
      hess_h: &[f32],
      n: usize,
      n_eff: usize,
      n_bins: usize,
      num_leaves: usize,
      max_depth: usize,
      lambda: f32,
      min_cw: f32,
      min_gain: f32,
) -> Result<Tree, HipError> {
      let mut bins_flat = vec![0u8; n_eff * n];
      for (j, col) in bins_eff.iter().enumerate() {
            bins_flat[j * n..(j + 1) * n].copy_from_slice(col);
      }

      let bins_fm = GpuBuffer::upload_u8(&bins_flat)?;
      let node_idx = GpuBuffer::zeros_bytes(n)?;
      let grad_gpu = GpuBuffer::upload_f32(grad_h)?;
      let hess_gpu = GpuBuffer::upload_f32(hess_h)?;

      let hist_size = num_leaves * n_eff * n_bins;
      let grad_hist = GpuBuffer::zeros_f32(hist_size)?;
      let hess_hist = GpuBuffer::zeros_f32(hist_size)?;
      let gain_out  = GpuBuffer::zeros_f32(hist_size)?;

      let max_nodes = 2 * num_leaves - 1;
      let mut nodes: Vec<SplitNode> = (0..max_nodes).map(|_| SplitNode {
            feature: 0, bin: 0, left_child: 0, right_child: 0, leaf_value: 0.0, is_leaf: true,
      }).collect();

      let mut slot_tree: Vec<i32> = vec![-1; num_leaves];
      slot_tree[0] = 0;
      let mut depth_map: Vec<usize> = vec![0; max_nodes];
      let mut next_tree_node = 1usize;

      for _iter in 0..num_leaves - 1 {
            let active_i32: Vec<i32> = slot_tree.iter().map(|&t| if t >= 0 { 1 } else { 0 }).collect();
            if active_i32.iter().sum::<i32>() == 0 { break; }
            let leaf_active_gpu = GpuBuffer::upload_i32(&active_i32)?;

            grad_hist.memset_zero(hist_size * 4)?;
            hess_hist.memset_zero(hist_size * 4)?;

            gpu_oblivious_histogram(&bins_fm, &node_idx, &grad_gpu, &hess_gpu, &grad_hist, &hess_hist, n, n_eff, n_bins, num_leaves);
            gpu_leaf_wise_best_split(&grad_hist, &hess_hist, &leaf_active_gpu, &gain_out, lambda, min_cw, num_leaves, n_eff, n_bins);

            let mut gain_host = vec![0.0f32; hist_size];
            gain_out.download_f32(&mut gain_host)?;

            let (best_idx, best_gain) = gain_host.iter().enumerate()
                  .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                  .map(|(i, &g)| (i, g))
                  .unwrap_or((0, f32::NEG_INFINITY));

            if best_gain <= min_gain { break; }

            let best_slot = best_idx / (n_eff * n_bins);
            let best_feat = (best_idx % (n_eff * n_bins)) / n_bins;
            let best_bin  = (best_idx % n_bins) as u8;

            let parent_tree = slot_tree[best_slot];
            if parent_tree < 0 { break; }
            if max_depth > 0 && depth_map[parent_tree as usize] >= max_depth { break; }

            let right_slot = match slot_tree.iter().position(|&t| t < 0) {
                  Some(s) => s,
                  None => break,
            };
            if next_tree_node + 2 > max_nodes { break; }

            let left_tree  = next_tree_node;
            let right_tree = next_tree_node + 1;
            next_tree_node += 2;

            let parent_depth = depth_map[parent_tree as usize];
            depth_map[left_tree]  = parent_depth + 1;
            depth_map[right_tree] = parent_depth + 1;

            nodes[parent_tree as usize].feature = best_feat;
            nodes[parent_tree as usize].bin = best_bin;
            nodes[parent_tree as usize].left_child = left_tree;
            nodes[parent_tree as usize].right_child = right_tree;
            nodes[parent_tree as usize].is_leaf = false;

            slot_tree[best_slot]  = left_tree as i32;
            slot_tree[right_slot] = right_tree as i32;

            gpu_leaf_split_apply(&bins_fm, &node_idx, best_slot, best_slot, right_slot, best_feat, best_bin, n, n_eff);
      }

      let leaf_grad = GpuBuffer::zeros_f32(num_leaves)?;
      let leaf_hess = GpuBuffer::zeros_f32(num_leaves)?;
      let leaf_val  = GpuBuffer::zeros_f32(num_leaves)?;

      gpu_leaf_reduce(&node_idx, &grad_gpu, &hess_gpu, &leaf_grad, &leaf_hess, n);
      gpu_leaf_finalize(&leaf_grad, &leaf_hess, &leaf_val, lambda, num_leaves);

      let mut leaf_values = vec![0.0f32; num_leaves];
      leaf_val.download_f32(&mut leaf_values)?;

      for slot in 0..num_leaves {
            let t = slot_tree[slot];
            if t >= 0 { nodes[t as usize].leaf_value = leaf_values[slot]; }
      }

      Ok(Tree { nodes })
}

fn predict_tree_cpu(tree: &Tree, bins_flat: &[u8], n: usize, _n_eff: usize) -> Vec<f32> {
      let mut out = vec![0.0f32; n];
      for i in 0..n {
            let mut node = 0usize;
            loop {
                  let sn = &tree.nodes[node];
                  if sn.is_leaf {
                        out[i] = sn.leaf_value;
                        break;
                  }
                  let bin = bins_flat[sn.feature * n + i];
                  node = if bin <= sn.bin { sn.left_child } else { sn.right_child };
            }
      }
      out
}

pub fn train(x: &[f64], y: &[f64], n: usize, p: usize, params: &Params) -> Result<Model, Error> {
      validate_regression(x, y, n, p, params)?;
      let y_f32: Vec<f32> = y.iter().map(|&v| v as f32).collect();
      let mut rng = ChaCha8Rng::seed_from_u64(params.seed);

      let borders = compute_borders(x, n, p, params.n_bins);
      let bins_raw = quantize(x, n, p, &borders);

      let (bins_eff, bundle_map, n_eff) = if params.use_efb {
            let (bm, bundled, nf) = efb_bundle(&bins_raw, n, p, params.efb_max_conflict);
            (bundled, bm, nf)
      } else {
            let bm: Vec<(usize, u8)> = (0..p).map(|j| (j, 0u8)).collect();
            (bins_raw, bm, p)
      };

      let use_goss = params.goss_a > 0.0;
      let lambda = params.l2_reg as f32;
      let min_cw = params.min_child_weight as f32;
      let min_gain = params.min_gain_to_split as f32;
      let lr = params.learning_rate as f32;

      let mut pred = vec![0.0f32; n];
      let mut trees: Vec<Tree> = Vec::with_capacity(params.n_estimators);

      let mut bins_flat = vec![0u8; n_eff * n];
      for (j, col) in bins_eff.iter().enumerate() {
            bins_flat[j * n..(j + 1) * n].copy_from_slice(col);
      }

      for t in 0..params.n_estimators {
            let (mut grad, mut hess) = mse_grad_hess(&pred, &y_f32, n);

            if use_goss {
                  apply_goss(&mut grad, &mut hess, n, params.goss_a, params.goss_b, &mut rng)?;
            }

            let tree = build_leaf_wise_tree(&bins_eff, &grad, &hess, n, n_eff, params.n_bins, params.num_leaves, params.max_depth, lambda, min_cw, min_gain)?;

            let leaf_preds = predict_tree_cpu(&tree, &bins_flat, n, n_eff);
            for i in 0..n { pred[i] += lr * leaf_preds[i]; }
            eprintln!("      lgbm iter={}/{}", t + 1, params.n_estimators);
            trees.push(tree);
      }

      Ok(Model {
            trees: vec![trees],
            learning_rate: params.learning_rate,
            n_classes: 1,
            n_orig_features: p,
            borders,
            bundle_map,
            n_eff_features: n_eff,
      })
}

pub fn predict(model: &Model, x: &[f64], n: usize) -> Result<Vec<f64>, Error> {
      let p = model.n_orig_features;
      let n_eff = model.n_eff_features;
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }

      let bins_raw = quantize(x, n, p, &model.borders);
      let bins_eff = if n_eff != p {
            apply_bundle_map_predict(&bins_raw, n, p, &model.bundle_map, n_eff)
      } else {
            bins_raw
      };

      let mut bins_flat = vec![0u8; n_eff * n];
      for (j, col) in bins_eff.iter().enumerate() {
            bins_flat[j * n..(j + 1) * n].copy_from_slice(col);
      }

      let mut pred = vec![0.0f32; n];
      for tree in &model.trees[0] {
            let leaf_preds = predict_tree_cpu(tree, &bins_flat, n, n_eff);
            for i in 0..n { pred[i] += model.learning_rate as f32 * leaf_preds[i]; }
      }
      Ok(pred.iter().map(|&v| v as f64).collect())
}

pub fn train_multiclass(x: &[f64], y: &[usize], n: usize, p: usize, n_classes: usize, params: &Params) -> Result<Model, Error> {
      validate_classification(x, y, n, p, n_classes, params)?;
      let mut rng = ChaCha8Rng::seed_from_u64(params.seed);

      let borders = compute_borders(x, n, p, params.n_bins);
      let bins_raw = quantize(x, n, p, &borders);

      let (bins_eff, bundle_map, n_eff) = if params.use_efb {
            let (bm, bundled, nf) = efb_bundle(&bins_raw, n, p, params.efb_max_conflict);
            (bundled, bm, nf)
      } else {
            let bm: Vec<(usize, u8)> = (0..p).map(|j| (j, 0u8)).collect();
            (bins_raw, bm, p)
      };

      let use_goss = params.goss_a > 0.0;
      let lambda = params.l2_reg as f32;
      let min_cw = params.min_child_weight as f32;
      let min_gain = params.min_gain_to_split as f32;
      let lr = params.learning_rate as f32;

      let mut pred_logits = vec![0.0f32; n * n_classes];
      let mut all_trees: Vec<Vec<Tree>> = (0..n_classes).map(|_| Vec::with_capacity(params.n_estimators)).collect();

      let mut bins_flat = vec![0u8; n_eff * n];
      for (j, col) in bins_eff.iter().enumerate() {
            bins_flat[j * n..(j + 1) * n].copy_from_slice(col);
      }

      for t in 0..params.n_estimators {
            for k in 0..n_classes {
                  let (mut grad, mut hess): (Vec<f32>, Vec<f32>) = (0..n).map(|i| {
                        let row = &pred_logits[i * n_classes..(i + 1) * n_classes];
                        mc_grad_hess_row(row, y[i], n_classes)
                  }).unzip();

                  if use_goss {
                        apply_goss(&mut grad, &mut hess, n, params.goss_a, params.goss_b, &mut rng)?;
                  }

                  let tree = build_leaf_wise_tree(&bins_eff, &grad, &hess, n, n_eff, params.n_bins, params.num_leaves, params.max_depth, lambda, min_cw, min_gain)?;

                  let leaf_preds = predict_tree_cpu(&tree, &bins_flat, n, n_eff);
                  for i in 0..n { pred_logits[i * n_classes + k] += lr * leaf_preds[i]; }
                  all_trees[k].push(tree);
            }
            eprintln!("      lgbm iter={}/{}", t + 1, params.n_estimators);
      }

      Ok(Model {
            trees: all_trees,
            learning_rate: params.learning_rate,
            n_classes,
            n_orig_features: p,
            borders,
            bundle_map,
            n_eff_features: n_eff,
      })
}

pub fn predict_proba(model: &Model, x: &[f64], n: usize) -> Result<Vec<f64>, Error> {
      let p = model.n_orig_features;
      let nc = model.n_classes;
      let n_eff = model.n_eff_features;
      if x.len() != n * p { return Err(Error::InvalidInput(format!("x.len()={} != n*p={}", x.len(), n * p))); }

      let bins_raw = quantize(x, n, p, &model.borders);
      let bins_eff = if n_eff != p {
            apply_bundle_map_predict(&bins_raw, n, p, &model.bundle_map, n_eff)
      } else {
            bins_raw
      };

      let mut bins_flat = vec![0u8; n_eff * n];
      for (j, col) in bins_eff.iter().enumerate() {
            bins_flat[j * n..(j + 1) * n].copy_from_slice(col);
      }

      let mut logits = vec![0.0f32; n * nc];
      for k in 0..nc {
            for tree in &model.trees[k] {
                  let leaf_preds = predict_tree_cpu(tree, &bins_flat, n, n_eff);
                  for i in 0..n { logits[i * nc + k] += model.learning_rate as f32 * leaf_preds[i]; }
            }
      }

      let mut probs = vec![0.0f64; n * nc];
      for i in 0..n {
            let row = &logits[i * nc..(i + 1) * nc];
            let mx = row.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let s: f32 = row.iter().map(|&v| (v - mx).exp()).sum();
            for k in 0..nc { probs[i * nc + k] = ((logits[i * nc + k] - mx).exp() / s) as f64; }
      }
      Ok(probs)
}

#[cfg(test)]
mod tests {
      use super::*;

      fn make_regression_data(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let x: Vec<f64> = (0..n).map(|_| rng.random::<f64>()).collect();
            let y: Vec<f64> = x.iter().map(|&v| 2.0 * v + 1.0).collect();
            (x, y)
      }

      #[test]
      fn test_regression_sanity() {
            let (x, y) = make_regression_data(300, 0);
            let params = Params { n_estimators: 80, num_leaves: 8, learning_rate: 0.3, l2_reg: 1.0, n_bins: 32, ..Default::default() };
            let model = train(&x, &y, 300, 1, &params).unwrap();
            let pred = predict(&model, &x, 300).unwrap();
            let mae: f64 = pred.iter().zip(y.iter()).map(|(&p, &t)| (p - t).abs()).sum::<f64>() / 300.0;
            assert!(mae < 0.05, "MAE={mae:.4} >= 0.05");
      }

      #[test]
      fn test_binary_classification() {
            let mut rng = ChaCha8Rng::seed_from_u64(1);
            let n = 300;
            let x: Vec<f64> = (0..n).map(|_| rng.random::<f64>()).collect();
            let y: Vec<usize> = x.iter().map(|&v| if v > 0.5 { 1 } else { 0 }).collect();
            let params = Params { n_estimators: 80, num_leaves: 8, learning_rate: 0.2, l2_reg: 1.0, n_bins: 32, ..Default::default() };
            let model = train_multiclass(&x, &y, n, 1, 2, &params).unwrap();
            let probs = predict_proba(&model, &x, n).unwrap();
            let correct = (0..n).filter(|&i| (if probs[i * 2] > probs[i * 2 + 1] { 0 } else { 1 }) == y[i]).count();
            let acc = correct as f64 / n as f64;
            assert!(acc >= 0.95, "accuracy={acc:.3} < 0.95");
      }

      #[test]
      fn test_multiclass_3class() {
            let n = 300;
            let x: Vec<f64> = (0..n).map(|i| (i as f64) / n as f64).collect();
            let x2: Vec<f64> = x.iter().map(|&v| v * v).collect();
            let xdata: Vec<f64> = x.iter().zip(x2.iter()).flat_map(|(&a, &b)| [a, b]).collect();
            let y: Vec<usize> = (0..n).map(|i| i % 3).collect();
            let params = Params { n_estimators: 100, num_leaves: 16, learning_rate: 0.15, l2_reg: 1.0, n_bins: 32, ..Default::default() };
            let model = train_multiclass(&xdata, &y, n, 2, 3, &params).unwrap();
            let probs = predict_proba(&model, &xdata, n).unwrap();
            let correct = (0..n).filter(|&i| (0..3).max_by(|&a, &b| probs[i * 3 + a].partial_cmp(&probs[i * 3 + b]).unwrap()).unwrap() == y[i]).count();
            let acc = correct as f64 / n as f64;
            assert!(acc >= 0.7, "accuracy={acc:.3} < 0.7");
      }

      #[test]
      fn test_goss_sampling() {
            let mut rng = ChaCha8Rng::seed_from_u64(2);
            let n = 300;
            let x: Vec<f64> = (0..n).map(|_| rng.random::<f64>()).collect();
            let y: Vec<usize> = x.iter().map(|&v| if v > 0.5 { 1 } else { 0 }).collect();

            let params_base = Params { n_estimators: 50, num_leaves: 8, learning_rate: 0.2, l2_reg: 1.0, n_bins: 32, goss_a: 0.0, goss_b: 0.3, seed: 42, ..Default::default() };
            let model_base = train_multiclass(&x, &y, n, 1, 2, &params_base).unwrap();
            let probs_base = predict_proba(&model_base, &x, n).unwrap();
            let acc_base = (0..n).filter(|&i| (if probs_base[i * 2] > probs_base[i * 2 + 1] { 0 } else { 1 }) == y[i]).count() as f64 / n as f64;

            let params_goss = Params { goss_a: 0.2, goss_b: 0.3, ..params_base };
            let model_goss = train_multiclass(&x, &y, n, 1, 2, &params_goss).unwrap();
            let probs_goss = predict_proba(&model_goss, &x, n).unwrap();
            let acc_goss = (0..n).filter(|&i| (if probs_goss[i * 2] > probs_goss[i * 2 + 1] { 0 } else { 1 }) == y[i]).count() as f64 / n as f64;

            assert!(acc_goss >= acc_base - 0.15, "GOSS acc={acc_goss:.3} much worse than base acc={acc_base:.3}");
      }

      #[test]
      fn test_efb_bundling() {
            let n = 200;
            let p = 10;
            let mut rng = ChaCha8Rng::seed_from_u64(3);
            let mut x = vec![0.0f64; n * p];
            let y: Vec<usize> = (0..n).map(|_| (rng.random::<f64>() > 0.5) as usize).collect();
            for i in 0..n {
                  let active = (rng.random::<f64>() * p as f64) as usize % p;
                  x[i * p + active] = rng.random::<f64>();
            }

            let params_no_efb = Params { n_estimators: 30, num_leaves: 8, learning_rate: 0.2, l2_reg: 1.0, n_bins: 32, use_efb: false, ..Default::default() };
            let model_no = train_multiclass(&x, &y, n, p, 2, &params_no_efb).unwrap();
            let probs_no = predict_proba(&model_no, &x, n).unwrap();
            let acc_no = (0..n).filter(|&i| (if probs_no[i*2] > probs_no[i*2+1] { 0 } else { 1 }) == y[i]).count() as f64 / n as f64;

            let params_efb = Params { use_efb: true, efb_max_conflict: 0.1, ..params_no_efb };
            let model_efb = train_multiclass(&x, &y, n, p, 2, &params_efb).unwrap();
            let probs_efb = predict_proba(&model_efb, &x, n).unwrap();
            let acc_efb = (0..n).filter(|&i| (if probs_efb[i*2] > probs_efb[i*2+1] { 0 } else { 1 }) == y[i]).count() as f64 / n as f64;

            assert!((acc_efb - acc_no).abs() <= 0.15, "EFB acc={acc_efb:.3} vs no-EFB acc={acc_no:.3}, diff > 0.15");
      }

      #[test]
      fn test_invalid_inputs() {
            let x = vec![1.0f64, 2.0];
            let y = vec![0.0f64, 1.0];
            let yu = vec![0usize, 1];
            let params = Params::default();

            assert!(train(&x, &y, 2, 1, &Params { num_leaves: 1, ..Default::default() }).is_err());
            assert!(train(&x, &y, 2, 1, &Params { num_leaves: 256, ..Default::default() }).is_err());
            assert!(train(&x, &y, 2, 1, &Params { n_bins: 1, ..Default::default() }).is_err());
            assert!(train(&x, &y, 3, 1, &params).is_err());
            assert!(train_multiclass(&x, &yu, 2, 1, 1, &params).is_err());
            assert!(train_multiclass(&x, &[0, 5], 2, 1, 2, &params).is_err());
      }
}
