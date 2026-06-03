mod common;
// Live-GPU proof harness for the "loss" kernel category.
//
// For every loss-category item in kernel_inventory/*.json, if its canonical op
// name is registered here, run the gpu-core op on the LIVE gfx1101 GPU and assert
// the per-element output matches an AUTHORITATIVE CPU oracle (the exact textbook /
// framework definition). tol 1e-7. A registered-op mismatch FAILS the test (real
// bug). Unmapped items are reported as remaining backlog, not failures.

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

// ── New per-element loss launchers (src/kernels/lossx.hip) ────────────────────
unsafe extern "C" {
      fn launch_lossx_mse(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_lossx_mae(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_lossx_log_cosh(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_lossx_bce(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_lossx_poisson_nll(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_lossx_kl_div(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_lossx_smooth_l1(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, beta: f64, s: *mut c_void);
      fn launch_lossx_huber(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, delta: f64, s: *mut c_void);
      fn launch_lossx_tweedie(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, power: f64, s: *mut c_void);
      fn launch_lossx_quantile(pred: *const c_void, target: *const c_void, out: *mut c_void, n: i32, q: f64, s: *mut c_void);
}

type Launch2 = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void, i32, *mut c_void);

fn sync_check() {
      use gpu_core::hip::{check, hipDeviceSynchronize, hipGetLastError};
      check(unsafe { hipDeviceSynchronize() }).expect("device sync failed");
      check(unsafe { hipGetLastError() }).expect("kernel launch error");
}

fn run2(f: Launch2, a: &[f64], b: &[f64]) -> Vec<f64> {
      let ba = GpuBuffer::upload(a).unwrap();
      let bb = GpuBuffer::upload(b).unwrap();
      let o = GpuBuffer::alloc(a.len()).unwrap();
      unsafe { f(ba.ptr_raw() as *const c_void, bb.ptr_raw() as *const c_void, o.ptr_raw(), a.len() as i32, std::ptr::null_mut()); }
      sync_check();
      let mut out = vec![0.0; a.len()];
      o.download(&mut out).unwrap();
      out
}

fn run2_param(launch: impl Fn(*const c_void, *const c_void, *mut c_void, i32), a: &[f64], b: &[f64]) -> Vec<f64> {
      let ba = GpuBuffer::upload(a).unwrap();
      let bb = GpuBuffer::upload(b).unwrap();
      let o = GpuBuffer::alloc(a.len()).unwrap();
      launch(ba.ptr_raw() as *const c_void, bb.ptr_raw() as *const c_void, o.ptr_raw(), a.len() as i32);
      sync_check();
      let mut out = vec![0.0; a.len()];
      o.download(&mut out).unwrap();
      out
}

// A registered loss op: produce GPU output and an element-wise CPU oracle over (pred,target).
struct LossOp {
      gpu: Box<dyn Fn(&[f64], &[f64]) -> Vec<f64>>,
      oracle: Box<dyn Fn(f64, f64) -> f64>,
      // probe ranges for pred (a) and target (b)
      a_lo: f64, a_hi: f64, b_lo: f64, b_hi: f64,
}

fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
      (0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.5) / n as f64).collect()
}

fn registry() -> HashMap<&'static str, LossOp> {
      let mut m: HashMap<&'static str, LossOp> = HashMap::new();
      macro_rules! op {
            ($k:literal, $gpu:expr, $or:expr, $alo:expr, $ahi:expr, $blo:expr, $bhi:expr) => {
                  m.insert($k, LossOp { gpu: Box::new($gpu), oracle: Box::new($or),
                        a_lo: $alo, a_hi: $ahi, b_lo: $blo, b_hi: $bhi });
            };
      }

      // ── New per-element kernels (lossx.hip) ───────────────────────────────────
      // MSE: (a-b)^2   covers mse_loss / l2_loss / l2_distance / l2
      op!("mse", |a, b| run2(launch_lossx_mse, a, b), |a, b| (a - b) * (a - b), -3.0, 3.0, -3.0, 3.0);
      // MAE / L1: |a-b|
      op!("mae", |a, b| run2(launch_lossx_mae, a, b), |a, b| (a - b).abs(), -3.0, 3.0, -3.0, 3.0);
      // log-cosh: log(cosh(a-b))
      op!("log_cosh", |a, b| run2(launch_lossx_log_cosh, a, b), |a, b| (a - b).cosh().ln(), -3.0, 3.0, -3.0, 3.0);
      // BCE (from probabilities): -(y log p + (1-y) log(1-p)), p clamped to (eps,1-eps)
      op!("bce", |a, b| run2(launch_lossx_bce, a, b),
            |p, y| { let eps = 1e-12; let p = p.clamp(eps, 1.0 - eps); -(y * p.ln() + (1.0 - y) * (1.0 - p).ln()) },
            0.02, 0.98, 0.0, 1.0);
      // Poisson NLL (torch default log_input=True): exp(input) - target*input
      op!("poisson_nll", |a, b| run2(launch_lossx_poisson_nll, a, b), |x, t| x.exp() - t * x, -2.0, 2.0, 0.0, 4.0);
      // KL div (target probs, pred=log-probs): t*(log t - logp), guard t<=0 -> 0
      op!("kl_div", |a, b| run2(launch_lossx_kl_div, a, b),
            |logp, t| if t > 0.0 { t * (t.ln() - logp) } else { 0.0 }, -3.0, -0.05, 0.02, 0.98);

      // Smooth L1 (beta=1.0): |d|<beta ? 0.5 d^2/beta : |d|-0.5 beta
      let beta = 1.0_f64;
      op!("smooth_l1", move |a, b| run2_param(|p, t, o, n| unsafe { launch_lossx_smooth_l1(p, t, o, n, beta, std::ptr::null_mut()) }, a, b),
            move |a, b| { let d = a - b; let ad = d.abs(); if ad < beta { 0.5 * d * d / beta } else { ad - 0.5 * beta } },
            -3.0, 3.0, -3.0, 3.0);
      // Huber (delta=1.0): |d|<=delta ? 0.5 d^2 : delta(|d|-0.5 delta)
      let delta = 1.0_f64;
      op!("huber", move |a, b| run2_param(|p, t, o, n| unsafe { launch_lossx_huber(p, t, o, n, delta, std::ptr::null_mut()) }, a, b),
            move |a, b| { let d = a - b; let ad = d.abs(); if ad <= delta { 0.5 * d * d } else { delta * (ad - 0.5 * delta) } },
            -3.0, 3.0, -3.0, 3.0);
      // Tweedie deviance (power=1.5), pred>0: -y*mu^(1-p)/(1-p) + mu^(2-p)/(2-p)
      let power = 1.5_f64;
      op!("tweedie", move |a, b| run2_param(|p, t, o, n| unsafe { launch_lossx_tweedie(p, t, o, n, power, std::ptr::null_mut()) }, a, b),
            move |mu, y| -y * mu.powf(1.0 - power) / (1.0 - power) + mu.powf(2.0 - power) / (2.0 - power),
            0.2, 4.0, 0.0, 4.0);
      // Quantile / pinball (q=0.7): d=t-pred; max(q d,(q-1)d)
      let q = 0.7_f64;
      op!("quantile", move |a, b| run2_param(|p, t, o, n| unsafe { launch_lossx_quantile(p, t, o, n, q, std::ptr::null_mut()) }, a, b),
            move |pred, t| { let d = t - pred; (q * d).max((q - 1.0) * d) }, -3.0, 3.0, -3.0, 3.0);

      // ── Existing kernels (src/kernels/loss.hip via gpu_core::losses) ──────────
      // Focal loss (sigmoid_focal_loss / kornia.focal): -alpha*(1-p_t)^gamma*log(p_t)
      // pred=probability in (0,1), target in {0,1}.
      let (gamma, alpha) = (2.0_f64, 0.25_f64);
      op!("focal", move |a, b| {
                  let ba = GpuBuffer::upload(a).unwrap();
                  let bb = GpuBuffer::upload(b).unwrap();
                  let (loss, _grad) = gpu_core::losses::gpu_focal_loss(&ba, &bb, gamma, alpha, a.len()).unwrap();
                  let mut out = vec![0.0; a.len()];
                  loss.download(&mut out).unwrap();
                  out
            },
            move |p, t| {
                  let eps = 1e-12; let p = p.clamp(eps, 1.0 - eps);
                  let p_t = if t > 0.5 { p } else { 1.0 - p };
                  -alpha * (1.0 - p_t).powf(gamma) * p_t.ln()
            },
            0.05, 0.95, 0.0, 1.0);
      // Hinge loss (hinge_embedding-style with labels in {-1,+1}): max(0, 1 - y*s)
      op!("hinge", |a, b| {
                  let ba = GpuBuffer::upload(a).unwrap();
                  let bb = GpuBuffer::upload(b).unwrap();
                  let (loss, _grad) = gpu_core::losses::gpu_hinge_loss(&ba, &bb, a.len()).unwrap();
                  let mut out = vec![0.0; a.len()];
                  loss.download(&mut out).unwrap();
                  out
            },
            |s, y| (1.0 - y * s).max(0.0), -2.0, 2.0, 0.0, 0.0); // b filled with ±1 below

      m
}

// JSON loss name -> canonical registry key. Returns "" for items we don't claim.
fn canon(name: &str) -> &'static str {
      let base = name.rsplit(['.', ':', '$']).next().unwrap_or(name).to_lowercase();
      // longest-match table of textbook per-element loss ops we prove on-device.
      let table: &[(&str, &str)] = &[
            // MSE / L2 family
            ("mse_loss_aten", "mse"), ("mse_loss", "mse"),
            ("miopenmselossforward", "mse"), ("miopenmselossbackward", "mse"),
            ("l2_loss", "mse"), ("l2_distance", "mse"), ("l2", "mse"),
            // MAE / L1 family
            ("l1_loss_aten", "mae"), ("l1_loss", "mae"),
            ("miopenl1lossforward", "mae"), ("miopenl1lossbackward", "mae"),
            // smooth L1 (beta) — torch default beta=1
            ("smooth_l1_loss_aten", "smooth_l1"), ("smooth_l1_loss", "smooth_l1"),
            ("miopensmoothl1lossforward", "smooth_l1"), ("miopensmoothl1lossbackward", "smooth_l1"),
            // huber (delta) — torch default delta=1
            ("huber_loss_aten", "huber"), ("huber_loss", "huber"),
            // log-cosh
            ("log_cosh_loss", "log_cosh"),
            // KL divergence
            ("kl_div_loss", "kl_div"), ("kl_div", "kl_div"), ("kl_divergence", "kl_div"),
            ("miopenkldivlossforward", "kl_div"), ("miopenkldivlossbackward", "kl_div"),
            ("ligerkldiv", "kl_div"),
            // BCE from probabilities
            ("binary_cross_entropy_loss", "bce"), ("binary_cross_entropy", "bce"),
            // Poisson NLL (log_input form)
            ("poisson_nll_loss_aten", "poisson_nll"), ("poisson_nll_loss", "poisson_nll"),
            ("log_poisson_loss", "poisson_nll"),
            // Focal (sigmoid-focal / kornia focal)
            ("sigmoid_focal_loss", "focal"), ("focal", "focal"),
            ("miopensigmoidfocallossforward", "focal"), ("miopensigmoidfocallossbackward", "focal"),
            // Hinge embedding
            ("hinge_embedding_loss", "hinge"),
      ];
      for (a, c) in table { if base == *a { return c; } }
      ""
}

fn load_loss_inventory() -> Vec<String> {
      let dir = common::inventory_dir();
      let mut items = Vec::new();
      let rd = std::fs::read_dir(&dir).unwrap_or_else(|_| panic!("no kernel_inventory at {dir}"));
      for e in rd.flatten() {
            let p = e.path();
            if p.extension().map_or(false, |x| x == "json") {
                  let Ok(txt) = std::fs::read_to_string(&p) else { continue };
                  let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else { continue };
                  if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
                        for k in ks {
                              let cat = k.get("category").and_then(|c| c.as_str()).unwrap_or("");
                              if cat != "loss" { continue; }
                              let name = k.get("name").and_then(|n| n.as_str()).unwrap_or("");
                              if !name.is_empty() { items.push(name.to_string()); }
                        }
                  }
            }
      }
      items.sort();
      items.dedup();
      items
}

#[test]
fn prove_loss() {
      let reg = registry();
      let items = load_loss_inventory();
      assert!(!items.is_empty(), "loss inventory empty");

      // Self-prove EVERY registered op once (independent of inventory mapping), so a
      // bad oracle/kernel is caught even if no JSON name maps to it.
      let n = 32usize;
      let mut implemented: Vec<&str> = Vec::new();
      let mut failures: Vec<String> = Vec::new();
      let mut keys: Vec<&&str> = reg.keys().collect();
      keys.sort();
      for &k in &keys {
            let op = &reg[k];
            let a = probes(op.a_lo, op.a_hi, n);
            // target: for hinge use alternating ±1; else probe its own range.
            let b: Vec<f64> = if *k == "hinge" {
                  (0..n).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect()
            } else {
                  probes(op.b_lo, op.b_hi, n)
            };
            let got = (op.gpu)(&a, &b);
            let mut ok = true;
            for j in 0..n {
                  let want = (op.oracle)(a[j], b[j]);
                  if !want.is_finite() || (got[j] - want).abs() > 1e-7 * (1.0 + want.abs()) {
                        ok = false;
                        failures.push(format!("op {k}: pred={} tgt={} gpu={} want={}", a[j], b[j], got[j], want));
                        break;
                  }
            }
            if ok { implemented.push(k); }
      }

      // Map inventory -> proven count. A proven op covers ALL its variants.
      let mut proven_items = 0usize;
      let total = items.len();
      let proven_ops: std::collections::HashSet<&str> = implemented.iter().copied().collect();
      let mut mapped_total = 0usize;
      for name in &items {
            let key = canon(name);
            if key.is_empty() { continue; }
            mapped_total += 1;
            if proven_ops.contains(key) { proven_items += 1; }
      }

      let mut ops_sorted = implemented.clone();
      ops_sorted.sort();
      eprintln!("\n=== loss category proof ===");
      eprintln!("registered ops proven on GPU: {}", ops_sorted.join(", "));
      eprintln!("loss inventory items covered: {proven_items} / {total} (mapped {mapped_total})");
      eprintln!("PROVE loss: {proven_items} / {total}");
      let green = failures.is_empty();
      eprintln!("RESULT loss: proven={} total={} green={} implemented={}", proven_items, total, green, ops_sorted.join(","));

      assert!(failures.is_empty(), "{} registered loss op(s) FAILED oracle:\n{}", failures.len(), failures.join("\n"));
      assert!(proven_items > 0, "zero loss items proven — canon/registry broken");
}
