mod common;
// Live-GPU proof harness for the "norm" inventory category.
//
// For every norm-category item in kernel_inventory/*.json, canonicalize its name;
// if that canonical op is registered here, run the gpu-core op on the LIVE
// gfx1101 GPU and assert it matches an AUTHORITATIVE CPU oracle (std f64 textbook
// standardize: (x-mean)/sqrt(var+eps), rms = x/sqrt(mean(x^2)+eps), l2 =
// x/sqrt(max(sumsq,eps)), softmax = exp(x-max)/sum). tol 1e-7.
//
// A proven op counts ALL its inventory variants (collapsed by canon). The test
// FAILS on any registered-op mismatch (a real bug). Backward/grad/quant/fp8/
// dropout/scaler/imputer/LRN/whitening items are different functions and stay as
// backlog — never claimed by a forward op (mirrors prove_special.rs convention).

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

// New normx_ launchers (see src/kernels/normx.hip).
unsafe extern "C" {
      fn launch_normx_groupnorm(x: *const c_void, out: *mut c_void, gamma: *const c_void, beta: *const c_void,
            n: i32, c: i32, l: i32, g: i32, eps: f64, s: *mut c_void);
      fn launch_normx_instancenorm(x: *const c_void, out: *mut c_void, gamma: *const c_void, beta: *const c_void,
            n: i32, c: i32, l: i32, eps: f64, s: *mut c_void);
      fn launch_normx_l2_normalize(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, eps: f64, s: *mut c_void);
      fn launch_normx_rmsnorm(x: *const c_void, out: *mut c_void, gamma: *const c_void, rows: i32, cols: i32, eps: f64, s: *mut c_void);
}

const TOL: f64 = 1e-7;

fn close(a: &[f64], b: &[f64]) -> bool {
      a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= TOL * (1.0 + y.abs()))
}

// ── deterministic test data ──────────────────────────────────────────────────
fn ramp(n: usize, scale: f64, off: f64) -> Vec<f64> {
      (0..n).map(|i| (i as f64) * scale + off).collect()
}

// ── CPU oracles (authoritative textbook standardize, biased variance) ─────────
fn cpu_layernorm(x: &[f64], rows: usize, cols: usize, gamma: &[f64], beta: &[f64], eps: f64) -> Vec<f64> {
      let mut o = vec![0.0; rows * cols];
      for r in 0..rows {
            let row = &x[r * cols..(r + 1) * cols];
            let mean = row.iter().sum::<f64>() / cols as f64;
            let var = row.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / cols as f64;
            let inv = 1.0 / (var + eps).sqrt();
            for j in 0..cols { o[r * cols + j] = (row[j] - mean) * inv * gamma[j] + beta[j]; }
      }
      o
}

// batchnorm forward: normalize each channel c over the N samples. x: (N, C).
fn cpu_batchnorm(x: &[f64], n: usize, c: usize, gamma: &[f64], beta: &[f64], eps: f64) -> Vec<f64> {
      let mut o = vec![0.0; n * c];
      for ch in 0..c {
            let col: Vec<f64> = (0..n).map(|i| x[i * c + ch]).collect();
            let mean = col.iter().sum::<f64>() / n as f64;
            let var = col.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
            let inv = 1.0 / (var + eps).sqrt();
            for i in 0..n { o[i * c + ch] = gamma[ch] * (x[i * c + ch] - mean) * inv + beta[ch]; }
      }
      o
}

// groupnorm: x (N, C, L), G groups over C. normalize over (C/G)*L per (n,g).
fn cpu_groupnorm(x: &[f64], n: usize, c: usize, l: usize, g: usize, gamma: &[f64], beta: &[f64], eps: f64) -> Vec<f64> {
      let cpg = c / g;
      let mut o = vec![0.0; n * c * l];
      for ni in 0..n {
            for gi in 0..g {
                  let base = ni * c * l + gi * cpg * l;
                  let m = cpg * l;
                  let mean = (0..m).map(|i| x[base + i]).sum::<f64>() / m as f64;
                  let var = (0..m).map(|i| (x[base + i] - mean).powi(2)).sum::<f64>() / m as f64;
                  let inv = 1.0 / (var + eps).sqrt();
                  for i in 0..m {
                        let ch = gi * cpg + i / l;
                        o[base + i] = (x[base + i] - mean) * inv * gamma[ch] + beta[ch];
                  }
            }
      }
      o
}

// instancenorm: x (N, C, L). normalize over L per (n,c). == groupnorm G=C.
fn cpu_instancenorm(x: &[f64], n: usize, c: usize, l: usize, gamma: &[f64], beta: &[f64], eps: f64) -> Vec<f64> {
      cpu_groupnorm(x, n, c, l, c, gamma, beta, eps)
}

fn cpu_l2_normalize(x: &[f64], rows: usize, cols: usize, eps: f64) -> Vec<f64> {
      let mut o = vec![0.0; rows * cols];
      for r in 0..rows {
            let row = &x[r * cols..(r + 1) * cols];
            let ss = row.iter().map(|v| v * v).sum::<f64>();
            let inv = 1.0 / ss.max(eps).sqrt();
            for j in 0..cols { o[r * cols + j] = row[j] * inv; }
      }
      o
}

fn cpu_rmsnorm(x: &[f64], rows: usize, cols: usize, gamma: &[f64], eps: f64) -> Vec<f64> {
      let mut o = vec![0.0; rows * cols];
      for r in 0..rows {
            let row = &x[r * cols..(r + 1) * cols];
            let ms = row.iter().map(|v| v * v).sum::<f64>() / cols as f64;
            let inv = 1.0 / (ms + eps).sqrt();
            for j in 0..cols { o[r * cols + j] = row[j] * inv * gamma[j]; }
      }
      o
}

fn cpu_softmax(x: &[f64], rows: usize, cols: usize) -> Vec<f64> {
      let mut o = vec![0.0; rows * cols];
      for r in 0..rows {
            let row = &x[r * cols..(r + 1) * cols];
            let mx = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let ex: Vec<f64> = row.iter().map(|v| (v - mx).exp()).collect();
            let s: f64 = ex.iter().sum();
            for j in 0..cols { o[r * cols + j] = ex[j] / s; }
      }
      o
}

fn cpu_log_softmax(x: &[f64], rows: usize, cols: usize) -> Vec<f64> {
      let mut o = vec![0.0; rows * cols];
      for r in 0..rows {
            let row = &x[r * cols..(r + 1) * cols];
            let mx = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let lse = mx + row.iter().map(|v| (v - mx).exp()).sum::<f64>().ln();
            for j in 0..cols { o[r * cols + j] = row[j] - lse; }
      }
      o
}

// ── GPU runners ──────────────────────────────────────────────────────────────
fn dl(b: &GpuBuffer, n: usize) -> Vec<f64> { let mut o = vec![0.0; n]; b.download(&mut o).unwrap(); o }

// ── prove each registered op once; cache pass/fail by canonical key ───────────
fn run_proofs() -> (HashMap<&'static str, bool>, Vec<String>) {
      let mut ok: HashMap<&'static str, bool> = HashMap::new();
      let mut fails: Vec<String> = Vec::new();
      let mut put = |k: &'static str, pass: bool, fails: &mut Vec<String>| {
            ok.insert(k, pass);
            if !pass { fails.push(k.to_string()); }
      };

      // ── layernorm (existing gpu_layernorm, eps=1e-5 fixed, affine) ──
      {
            use gpu_core::kernels::gpu_layernorm;
            let (rows, cols) = (4usize, 7usize);
            let x = ramp(rows * cols, 0.13, -1.7);
            let gamma = ramp(cols, 0.05, 0.8);
            let beta = ramp(cols, -0.03, -0.2);
            let bx = GpuBuffer::upload(&x).unwrap();
            let bg = GpuBuffer::upload(&gamma).unwrap();
            let bb = GpuBuffer::upload(&beta).unwrap();
            let g = gpu_layernorm(&bx, rows, cols, Some(&bg), Some(&bb)).unwrap();
            let got = dl(&g, rows * cols);
            let want = cpu_layernorm(&x, rows, cols, &gamma, &beta, 1e-5);
            put("layernorm", close(&got, &want), &mut fails);
            // jax.nn.standardize == layernorm without affine (gamma=1,beta=0)
            let one = vec![1.0; cols]; let zero = vec![0.0; cols];
            let g2 = gpu_layernorm(&bx, rows, cols, None, None).unwrap();
            let got2 = dl(&g2, rows * cols);
            let want2 = cpu_layernorm(&x, rows, cols, &one, &zero, 1e-5);
            put("standardize", close(&got2, &want2), &mut fails);
      }

      // ── batchnorm (existing gpu_batchnorm_forward, eps arg, affine) ──
      {
            use gpu_core::kernels::gpu_batchnorm_forward;
            let (n, c) = (6usize, 5usize);
            let eps = 1e-5;
            let x = ramp(n * c, 0.07, -0.9);
            let gamma = ramp(c, 0.04, 1.1);
            let beta = ramp(c, -0.06, 0.3);
            let bx = GpuBuffer::upload(&x).unwrap();
            let bg = GpuBuffer::upload(&gamma).unwrap();
            let bb = GpuBuffer::upload(&beta).unwrap();
            let (out, _m, _i) = gpu_batchnorm_forward(&bx, &bg, &bb, n, c, eps).unwrap();
            let got = dl(&out, n * c);
            let want = cpu_batchnorm(&x, n, c, &gamma, &beta, eps);
            put("batchnorm", close(&got, &want), &mut fails);
      }

      // ── softmax + log_softmax (existing) ──
      {
            use gpu_core::kernels::{gpu_softmax_rows, gpu_log_softmax_rows};
            let (rows, cols) = (3usize, 5usize);
            let x = ramp(rows * cols, 0.21, -1.5);
            let bx = GpuBuffer::upload(&x).unwrap();
            let g = gpu_softmax_rows(&bx, rows, cols).unwrap();
            put("softmax", close(&dl(&g, rows * cols), &cpu_softmax(&x, rows, cols)), &mut fails);
            let gl = gpu_log_softmax_rows(&bx, rows, cols).unwrap();
            put("log_softmax", close(&dl(&gl, rows * cols), &cpu_log_softmax(&x, rows, cols)), &mut fails);
      }

      // ── rmsnorm (NEW normx_rmsnorm, f64 fused, affine) ──
      {
            let (rows, cols) = (4usize, 6usize);
            let eps = 1e-6;
            let x = ramp(rows * cols, 0.11, -0.8);
            let gamma = ramp(cols, 0.03, 0.9);
            let bx = GpuBuffer::upload(&x).unwrap();
            let bg = GpuBuffer::upload(&gamma).unwrap();
            let out = GpuBuffer::alloc(rows * cols).unwrap();
            unsafe { launch_normx_rmsnorm(bx.ptr_raw() as *const c_void, out.ptr_raw(), bg.ptr_raw() as *const c_void,
                  rows as i32, cols as i32, eps, std::ptr::null_mut()); }
            gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
            put("rmsnorm", close(&dl(&out, rows * cols), &cpu_rmsnorm(&x, rows, cols, &gamma, eps)), &mut fails);
      }

      // ── groupnorm (NEW normx_groupnorm, affine) ──
      {
            let (n, c, l, g) = (2usize, 6usize, 4usize, 3usize); // cpg=2
            let eps = 1e-5;
            let x = ramp(n * c * l, 0.05, -1.2);
            let gamma = ramp(c, 0.04, 0.7);
            let beta = ramp(c, -0.02, 0.1);
            let bx = GpuBuffer::upload(&x).unwrap();
            let bg = GpuBuffer::upload(&gamma).unwrap();
            let bb = GpuBuffer::upload(&beta).unwrap();
            let out = GpuBuffer::alloc(n * c * l).unwrap();
            unsafe { launch_normx_groupnorm(bx.ptr_raw() as *const c_void, out.ptr_raw(),
                  bg.ptr_raw() as *const c_void, bb.ptr_raw() as *const c_void,
                  n as i32, c as i32, l as i32, g as i32, eps, std::ptr::null_mut()); }
            gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
            put("groupnorm", close(&dl(&out, n * c * l), &cpu_groupnorm(&x, n, c, l, g, &gamma, &beta, eps)), &mut fails);
      }

      // ── instancenorm (NEW normx_instancenorm, affine) ──
      {
            let (n, c, l) = (3usize, 4usize, 5usize);
            let eps = 1e-5;
            let x = ramp(n * c * l, 0.06, -1.0);
            let gamma = ramp(c, 0.05, 0.8);
            let beta = ramp(c, -0.03, 0.2);
            let bx = GpuBuffer::upload(&x).unwrap();
            let bg = GpuBuffer::upload(&gamma).unwrap();
            let bb = GpuBuffer::upload(&beta).unwrap();
            let out = GpuBuffer::alloc(n * c * l).unwrap();
            unsafe { launch_normx_instancenorm(bx.ptr_raw() as *const c_void, out.ptr_raw(),
                  bg.ptr_raw() as *const c_void, bb.ptr_raw() as *const c_void,
                  n as i32, c as i32, l as i32, eps, std::ptr::null_mut()); }
            gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
            put("instancenorm", close(&dl(&out, n * c * l), &cpu_instancenorm(&x, n, c, l, &gamma, &beta, eps)), &mut fails);
      }

      // ── l2_normalize (NEW normx_l2_normalize) ──
      {
            let (rows, cols) = (4usize, 5usize);
            let eps = 1e-12;
            let x = ramp(rows * cols, 0.17, -1.3);
            let bx = GpuBuffer::upload(&x).unwrap();
            let out = GpuBuffer::alloc(rows * cols).unwrap();
            unsafe { launch_normx_l2_normalize(bx.ptr_raw() as *const c_void, out.ptr_raw(),
                  rows as i32, cols as i32, eps, std::ptr::null_mut()); }
            gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
            put("l2_normalize", close(&dl(&out, rows * cols), &cpu_l2_normalize(&x, rows, cols, eps)), &mut fails);
      }

      (ok, fails)
}

// ── canonicalize a norm JSON name to a registry key (forward ops only) ────────
fn canon(name: &str) -> String {
      // last segment after . : $ , lowercase
      let mut base = name.rsplit(['.', ':', '$']).next().unwrap_or(name).to_lowercase();
      base.retain(|ch| ch != '_');
      // never claim backward/grad/bwd/quant/fp8 variants from a forward op
      if base.contains("backward") || base.contains("grad") || base.ends_with("bwd")
            || base.contains("quant") || base.contains("fp8") { return base; }

      // l2 / unit normalization
      if base.contains("l2normalization") || base == "l2normalize" || base == "unitnormalization"
            || base == "normalize" { return "l2_normalize".to_string(); }
      // rms before layer (rmsnorm contains neither "layer" nor "batch")
      if base.contains("rmsnorm") || base.contains("rmsnormalization") { return "rmsnorm".to_string(); }
      if base.contains("groupnorm") || base.contains("groupnormalization") { return "groupnorm".to_string(); }
      if base.contains("instancenorm") || base.contains("instancenormalization") { return "instancenorm".to_string(); }
      if base.contains("layernorm") || base.contains("layernormalization") { return "layernorm".to_string(); }
      if base.contains("batchnorm") || base.contains("batchnormalization") { return "batchnorm".to_string(); }
      if base == "logsoftmax" { return "log_softmax".to_string(); }
      if base == "softmax" || base == "sparsesoftmax" { return "softmax".to_string(); }
      if base == "standardize" { return "standardize".to_string(); }
      base
}

fn load_norm() -> Vec<String> {
      let dir = common::inventory_dir();
      let mut items = Vec::new();
      let rd = std::fs::read_dir(&dir).expect("no kernel_inventory");
      for e in rd.flatten() {
            let p = e.path();
            if p.extension().map_or(false, |x| x == "json") {
                  let Ok(txt) = std::fs::read_to_string(&p) else { continue; };
                  let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else { continue; };
                  if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
                        for k in ks {
                              let cat = k.get("category").and_then(|c| c.as_str()).unwrap_or("");
                              if cat != "norm" { continue; }
                              let name = k.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                              if !name.is_empty() { items.push(name); }
                        }
                  }
            }
      }
      items.sort();
      items.dedup();
      items
}

#[test]
fn prove_norm() {
      let items = load_norm();
      assert!(!items.is_empty(), "no norm items in inventory");
      let (op_ok, fails) = run_proofs();

      let total = items.len();
      let mut proven = 0usize;
      let mut proven_keys: BTreeSet<String> = Default::default();
      for name in &items {
            let key = canon(name);
            if let Some(&ok) = op_ok.get(key.as_str()) {
                  if ok { proven += 1; proven_keys.insert(key); }
            }
      }

      let mut reg: Vec<&str> = op_ok.keys().copied().collect();
      reg.sort();
      eprintln!("\n=== PROVE norm ===");
      eprintln!("registered ops ({}): {}", reg.len(), reg.join(", "));
      eprintln!("proven canonical ops ({}): {}", proven_keys.len(),
            proven_keys.iter().cloned().collect::<Vec<_>>().join(", "));
      eprintln!("PROVE norm: {} / {}", proven, total);

      assert!(fails.is_empty(), "registered norm op(s) FAILED oracle: {:?}", fails);
      assert!(proven > 0, "zero norm items proven");
}
