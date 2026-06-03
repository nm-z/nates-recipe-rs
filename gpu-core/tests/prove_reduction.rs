mod common;
// Live-GPU proof harness for the "reduction" inventory category.
//
// For every reduction-category item in kernel_inventory/*.json, canonicalize its
// name; if that canonical op is registered here, run the gpu-core op on the LIVE
// gfx1101 GPU and assert it matches an AUTHORITATIVE oracle (std f64 / textbook
// definition). tol 1e-7. A proven op counts ALL its inventory variants (collapsed
// by canon). The test FAILS on any registered-op mismatch (a real bug).
//
// Generic / windowed / distributed / order-statistic / segment / metric / scan /
// warp-intrinsic items stay backlog (reported, not faked green) — see canon()'s
// note list. Mapping any of those to a scalar op would be a false identity.

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

// ── New reductionx_ launchers (scalar/2-slot out) ─────────────────────────────
unsafe extern "C" {
      fn launch_reductionx_prod(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_nansum(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_mean_abs(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_sumsq(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_count_nonzero(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_any(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_all(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_ptp(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_logsumexp(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_sumsqdev(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_argmax(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
      fn launch_reductionx_argmin(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
}

type Launch = unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void);

// Run a launcher that writes `slots` doubles into out, return them.
fn run_slots(f: Launch, x: &[f64], slots: usize) -> Vec<f64> {
      let b = GpuBuffer::upload(x).unwrap();
      let o = GpuBuffer::alloc(slots).unwrap();
      unsafe { f(b.ptr_raw() as *const c_void, o.ptr_raw(), x.len() as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut out = vec![0.0; slots];
      o.download(&mut out).unwrap();
      out
}
fn scalar(f: Launch, x: &[f64]) -> f64 { run_slots(f, x, 1)[0] }

// ── GPU op wrappers producing a single f64 result on the LIVE GPU ─────────────
// Each closure uploads x, runs the device op, returns the finished scalar.
fn g_sum(x: &[f64]) -> f64 { gpu_core::reductions::gpu_sum_all(&GpuBuffer::upload(x).unwrap(), x.len()).unwrap() }
fn g_mean(x: &[f64]) -> f64 { gpu_core::reductions::gpu_mean_all(&GpuBuffer::upload(x).unwrap(), x.len()).unwrap() }
fn g_max(x: &[f64]) -> f64 { gpu_core::reductions::gpu_max_all(&GpuBuffer::upload(x).unwrap(), x.len()).unwrap() }
fn g_min(x: &[f64]) -> f64 { gpu_core::reductions::gpu_min_all(&GpuBuffer::upload(x).unwrap(), x.len()).unwrap() }
fn g_l2(x: &[f64]) -> f64 { gpu_core::reductions::gpu_l2_norm(&GpuBuffer::upload(x).unwrap(), x.len()).unwrap() }
fn g_asum(x: &[f64]) -> f64 { gpu_core::linalg::gpu_dasum(&GpuBuffer::upload(x).unwrap(), x.len()).unwrap() }
fn g_dot(x: &[f64]) -> f64 {
      let b = GpuBuffer::upload(x).unwrap();
      gpu_core::reductions::gpu_dot(&b, &b, x.len()).unwrap()      // dot(x,x) = Σx²
}
fn g_prod(x: &[f64]) -> f64 { scalar(launch_reductionx_prod, x) }
fn g_nansum(x: &[f64]) -> f64 { scalar(launch_reductionx_nansum, x) }
fn g_mean_abs(x: &[f64]) -> f64 { scalar(launch_reductionx_mean_abs, x) / x.len() as f64 }
fn g_sumsq(x: &[f64]) -> f64 { scalar(launch_reductionx_sumsq, x) }
fn g_count_nonzero(x: &[f64]) -> f64 { scalar(launch_reductionx_count_nonzero, x) }
fn g_any(x: &[f64]) -> f64 { scalar(launch_reductionx_any, x) }
fn g_all(x: &[f64]) -> f64 { scalar(launch_reductionx_all, x) }
fn g_ptp(x: &[f64]) -> f64 { let s = run_slots(launch_reductionx_ptp, x, 2); s[0] - s[1] }
fn g_logsumexp(x: &[f64]) -> f64 { let s = run_slots(launch_reductionx_logsumexp, x, 2); s[0] + s[1].ln() }
fn g_var_pop(x: &[f64]) -> f64 { scalar(launch_reductionx_sumsqdev, x) / x.len() as f64 }
fn g_var_samp(x: &[f64]) -> f64 { scalar(launch_reductionx_sumsqdev, x) / (x.len() as f64 - 1.0) }
fn g_std_pop(x: &[f64]) -> f64 { g_var_pop(x).sqrt() }
fn g_std_samp(x: &[f64]) -> f64 { g_var_samp(x).sqrt() }
fn g_argmax(x: &[f64]) -> f64 { scalar(launch_reductionx_argmax, x) }
fn g_argmin(x: &[f64]) -> f64 { scalar(launch_reductionx_argmin, x) }
// iamax: index of element with largest |x| (BLAS Idamax/Isamax family). 0-based.
fn g_iamax(x: &[f64]) -> f64 { gpu_core::linalg::gpu_idamax(&GpuBuffer::upload(x).unwrap(), x.len()).unwrap() as f64 }

// ── Op registry: canonical name -> (gpu closure, cpu oracle over the probe) ───
struct Op { gpu: Box<dyn Fn(&[f64]) -> f64>, oracle: Box<dyn Fn(&[f64]) -> f64> }

fn registry() -> HashMap<&'static str, Op> {
      let mut m: HashMap<&'static str, Op> = HashMap::new();
      macro_rules! op { ($k:literal, $g:expr, $o:expr) => {
            m.insert($k, Op { gpu: Box::new($g), oracle: Box::new($o) });
      }; }

      // existing ops (oracle = std f64)
      op!("sum",  g_sum,  |x: &[f64]| x.iter().sum());
      op!("mean", g_mean, |x: &[f64]| x.iter().sum::<f64>() / x.len() as f64);
      op!("max",  g_max,  |x: &[f64]| x.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
      op!("min",  g_min,  |x: &[f64]| x.iter().cloned().fold(f64::INFINITY, f64::min));
      op!("l2",   g_l2,   |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>().sqrt());
      op!("asum", g_asum, |x: &[f64]| x.iter().map(|v| v.abs()).sum());
      op!("dot",  g_dot,  |x: &[f64]| x.iter().map(|v| v * v).sum());      // dot(x,x)

      // new reductionx ops
      op!("prod",          g_prod,          |x: &[f64]| x.iter().product());
      op!("nansum",        g_nansum,        |x: &[f64]| x.iter().filter(|v| !v.is_nan()).sum());
      op!("sumsq",         g_sumsq,         |x: &[f64]| x.iter().map(|v| v * v).sum());
      op!("mean_abs",      g_mean_abs,      |x: &[f64]| x.iter().map(|v| v.abs()).sum::<f64>() / x.len() as f64);
      op!("count_nonzero", g_count_nonzero, |x: &[f64]| x.iter().filter(|v| **v != 0.0).count() as f64);
      op!("any",           g_any,           |x: &[f64]| if x.iter().any(|v| *v != 0.0) { 1.0 } else { 0.0 });
      op!("all",           g_all,           |x: &[f64]| if x.iter().all(|v| *v != 0.0) { 1.0 } else { 0.0 });
      op!("ptp",           g_ptp,           |x: &[f64]| {
            let mx = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mn = x.iter().cloned().fold(f64::INFINITY, f64::min);
            mx - mn
      });
      op!("logsumexp",     g_logsumexp,     |x: &[f64]| {
            let m = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            m + x.iter().map(|v| (v - m).exp()).sum::<f64>().ln()
      });
      op!("var_pop",  g_var_pop,  |x: &[f64]| { let mu = x.iter().sum::<f64>() / x.len() as f64; x.iter().map(|v| (v - mu).powi(2)).sum::<f64>() / x.len() as f64 });
      op!("var_samp", g_var_samp, |x: &[f64]| { let mu = x.iter().sum::<f64>() / x.len() as f64; x.iter().map(|v| (v - mu).powi(2)).sum::<f64>() / (x.len() as f64 - 1.0) });
      op!("std_pop",  g_std_pop,  |x: &[f64]| { let mu = x.iter().sum::<f64>() / x.len() as f64; (x.iter().map(|v| (v - mu).powi(2)).sum::<f64>() / x.len() as f64).sqrt() });
      op!("std_samp", g_std_samp, |x: &[f64]| { let mu = x.iter().sum::<f64>() / x.len() as f64; (x.iter().map(|v| (v - mu).powi(2)).sum::<f64>() / (x.len() as f64 - 1.0)).sqrt() });
      // argmax/argmin: first-occurrence index of the global extremum, as f64
      op!("argmax", g_argmax, |x: &[f64]| {
            let mut bi = 0; for i in 1..x.len() { if x[i] > x[bi] { bi = i; } } bi as f64
      });
      op!("argmin", g_argmin, |x: &[f64]| {
            let mut bi = 0; for i in 1..x.len() { if x[i] < x[bi] { bi = i; } } bi as f64
      });
      // iamax = index of max |x| (BLAS i*amax). Distinct from plain argmax.
      op!("iamax", g_iamax, |x: &[f64]| {
            let mut bi = 0; for i in 1..x.len() { if x[i].abs() > x[bi].abs() { bi = i; } } bi as f64
      });
      m
}

// ── Canonicalize a reduction-category JSON name to a registry key ─────────────
// Strips library/vendor prefixes + a BLAS dtype letter, lowercases the last
// dotted/colon/$ segment, then maps TRUE synonyms only. Anything generic,
// windowed, distributed, order-statistic, segmented, metric, scan, or a warp
// intrinsic is left unmapped (backlog) — never aliased to a scalar op.
fn canon(name: &str) -> String {
      let mut base = name.rsplit(['.', ':', '$']).next().unwrap_or(name).to_lowercase();
      base = base.trim_start_matches('_').to_string();
      // vendor BLAS prefixes (longest first)
      for p in ["rocblas_", "hipblaslt_", "hipblas_", "cublaslt_", "cublas_", "rocblas", "hipblas", "cublas"] {
            if let Some(s) = base.strip_prefix(p) { base = s.to_string(); break; }
      }
      // suffixed batched/strided/ex variants are a different SHAPE -> leave unmapped (backlog)
      if base.ends_with("_batched") || base.ends_with("_strided_batched") || base.ends_with("_ex")
            || base.ends_with("stridedbatched") {
            return base;
      }
      // BLAS reduction ops carry a dtype letter (s/d/c/z/h). Strip it for an exact match.
      // conjugate/complex dot (cdotc/zdotu/...) and i-prefixed iamax are handled BELOW (kept separate).
      let blas_ops = ["dot", "nrm2", "asum"];
      if base.len() >= 4 {
            let (f, rest) = base.split_at(1);
            if matches!(f, "s" | "d" | "c" | "z" | "h") && blas_ops.contains(&rest) {
                  return match rest { "nrm2" => "l2", "asum" => "asum", _ => "dot" }.to_string();
            }
      }
      // bare BLAS op names
      match base.as_str() { "dot" => return "dot".into(), "nrm2" => return "l2".into(), "asum" => return "asum".into(), _ => {} }

      let alias: &[(&str, &str)] = &[
            // ── sum family ──
            ("sum", "sum"), ("reduce_sum", "sum"), ("reducesum", "sum"), ("sumforward", "sum"),
            ("miopensumforward", "sum"), ("cudnnreduce_add", "sum"), ("vector_reduce_sum", "sum"),
            // named npp sums
            ("nppisum_8u_c1r", "sum"), ("nppisum_32f_c1r", "sum"),
            // ── mean / average family ──
            ("mean", "mean"), ("reduce_mean", "mean"), ("reducemean", "mean"), ("meanforward", "mean"),
            ("miopenmeanforward", "mean"), ("cudnnreduce_avg", "mean"), ("average", "mean"),
            ("nppimean_8u_c1r", "mean"),
            // ── max family ──
            ("max", "max"), ("amax", "max"), ("reduce_max", "max"), ("reducemax", "max"),
            ("maxforward", "max"), ("miopenmaxforward", "max"), ("cudnnreduce_max", "max"),
            ("nppimax_8u_c1r", "max"), ("vector_reduce_max", "max"),
            // ── min family ──
            ("min", "min"), ("amin", "min"), ("reduce_min", "min"), ("reducemin", "min"),
            ("minforward", "min"), ("miopenminforward", "min"), ("cudnnreduce_min", "min"),
            ("nppimin_8u_c1r", "min"), ("vector_reduce_min", "min"),
            // ── prod family ──
            ("prod", "prod"), ("product", "prod"), ("reduce_prod", "prod"), ("reduceprod", "prod"),
            ("reduceproduct", "prod"), ("prodforward", "prod"), ("miopenprodforward", "prod"),
            ("cudnnreduce_mul", "prod"), ("vector_reduce_prod", "prod"),
            // ── l2 / euclidean norm family ──
            ("norm", "l2"), ("l2_norm", "l2"), ("euclideannorm", "l2"), ("frobenius_norm", "l2"),
            ("multi_tensor_l2norm", "l2"), ("cudnnreduce_norm2", "l2"),
            ("nppinorm_l2_8u_c1r", "l2"),
            // ── asum / L1 norm family ──
            ("abssum", "asum"), ("cudnnreduce_norm1", "asum"), ("nppinorm_l1_8u_c1r", "asum"),
            // ── sum-of-squares ──
            ("sum_of_squares", "sumsq"), ("sqrsum", "sumsq"),
            // ── dot / inner product ──
            ("inner", "dot"), ("inner_product", "dot"), ("vdot", "dot"),
            ("bfdot", "dot"), ("hdot", "dot"),
            // ── argmax family ──
            ("argmax", "argmax"), ("arg_max", "argmax"), ("argmax_dim", "argmax"),
            ("reduceargmax", "argmax"), ("reduce_argmax", "argmax"), ("idxmax", "argmax"),
            ("vector_reduce_argmax", "argmax"), ("maximumindices", "argmax"),
            // ── argmin family ──
            ("argmin", "argmin"), ("arg_min", "argmin"), ("argmin_dim", "argmin"),
            ("reduceargmin", "argmin"), ("reduce_argmin", "argmin"), ("idxmin", "argmin"),
            ("vector_reduce_argmin", "argmin"),
            // ── any (logical OR over truthiness) ──
            ("any", "any"), ("reduce_any", "any"), ("any_of", "any"), ("reduce_or", "any"),
            // ── all (logical AND over truthiness) ──
            ("all", "all"), ("reduce_all", "all"), ("all_of", "all"), ("reduce_and", "all"),
            // ── var family (population: numpy/jax/tf default ddof=0) ──
            ("variance", "var_pop"), ("reduce_variance", "var_pop"),
            // ── var family (sample: torch/cudf/pandas default ddof=1) ──
            // torch.var / cudf::reduce::var collapse to "var" -> sample default
            ("var", "var_samp"),
            // ── std family (population) ──
            ("std_dev", "std_pop"),
            // ── std family (sample: torch/cudf default) ──
            ("std", "std_samp"),
            // ── logsumexp ──
            ("logsumexp", "logsumexp"), ("fast_logsumexp", "logsumexp"),
            // ── count_nonzero ──
            ("count_nonzero", "count_nonzero"), ("countnonzero", "count_nonzero"),
            // ── nansum ──
            ("nansum", "nansum"),
            // ── ptp ──
            ("ptp", "ptp"),
            // ── iamax: index of max|x| (real dtypes only: d=f64, s=f32). complex i*amax/i*amin and
            //    i*amin (no idamin fn) stay backlog. ──
            ("idamax", "iamax"), ("isamax", "iamax"),
      ];
      for (a, c) in alias { if base == *a { return c.to_string(); } }
      base
}

fn load_reduction() -> Vec<String> {
      let dir = common::inventory_dir();
      let mut items = Vec::new();
      for e in std::fs::read_dir(&dir).expect("no kernel_inventory").flatten() {
            let p = e.path();
            if p.extension().map_or(false, |x| x == "json") {
                  let Ok(txt) = std::fs::read_to_string(&p) else { continue; };
                  let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else { continue; };
                  if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
                        for k in ks {
                              if k.get("category").and_then(|c| c.as_str()) != Some("reduction") { continue; }
                              if let Some(n) = k.get("name").and_then(|n| n.as_str()) {
                                    if !n.is_empty() { items.push(n.to_string()); }
                              }
                        }
                  }
            }
      }
      items.sort();
      items.dedup();
      items
}

const TOL: f64 = 1e-7;
fn close(a: f64, b: f64) -> bool { (a - b).abs() <= TOL * (1.0 + b.abs()) }

// Per-op probe data. Several ops need shaped inputs (NaN for nansum, zeros for
// any/all/count, unique extremum for argmax/argmin). General default otherwise.
fn probe_for(key: &str) -> Vec<f64> {
      match key {
            "nansum" => vec![1.0, f64::NAN, 3.0, f64::NAN, 5.5, -2.0],
            "count_nonzero" | "any" => vec![0.0, 0.0, 3.0, 0.0, -1.5, 0.0],
            "all" => vec![1.0, 2.0, 3.0, 4.0, -1.5, 7.0],         // all nonzero -> 1
            "prod" => vec![1.5, -2.0, 0.5, 3.0, -1.25],            // bounded magnitude
            "argmax" => vec![-1.0, 4.0, 2.0, 9.5, 3.0, -7.0, 8.0], // unique max at idx 3
            "argmin" => vec![5.0, 4.0, 2.0, -9.5, 3.0, 7.0, 8.0],  // unique min at idx 3
            "iamax" => vec![1.0, -4.0, 2.0, -9.5, 3.0, 7.0, 8.0],  // unique max|x| at idx 3 (|-9.5|)
            "logsumexp" => vec![-2.0, 0.5, 1.0, 3.0, -1.5, 2.25],
            _ => vec![3.0, -1.5, 2.0, 4.25, -0.5, 1.75, 6.0, -3.5],
      }
}

#[test]
fn prove_reduction() {
      let items = load_reduction();
      assert!(!items.is_empty(), "no reduction items in inventory");
      let reg = registry();

      // Prove each registered op against its oracle on the LIVE GPU.
      let mut op_ok: HashMap<&str, bool> = HashMap::new();
      let mut failures: Vec<String> = Vec::new();
      for (k, op) in reg.iter() {
            let x = probe_for(k);
            let got = (op.gpu)(&x);
            let want = (op.oracle)(&x);
            let ok = close(got, want);
            if !ok { failures.push(format!("{}: gpu={} oracle={}", k, got, want)); }
            op_ok.insert(*k, ok);
      }

      // Extra defining-edge checks: any/all must exercise BOTH outcomes.
      {
            let all_zero = g_any(&[0.0, 0.0, 0.0]);
            if all_zero != 0.0 { failures.push(format!("any(all-zero)={} != 0", all_zero)); }
            let has_zero = g_all(&[1.0, 0.0, 2.0]);
            if has_zero != 0.0 { failures.push(format!("all(has-zero)={} != 0", has_zero)); }
      }

      // Walk inventory: item proven iff its canon maps to a passing registered op.
      let total = items.len();
      let mut proven = 0usize;
      let mut proven_keys: BTreeSet<String> = Default::default();
      let mut unmapped: Vec<String> = Vec::new();
      for name in &items {
            let key = canon(name);
            match op_ok.get(key.as_str()) {
                  Some(&true) => { proven += 1; proven_keys.insert(key); }
                  Some(&false) => {}
                  None => unmapped.push(format!("{} -> {}", name, key)),
            }
      }

      eprintln!("\n=== PROVE reduction ===");
      eprintln!("PROVE reduction: {} / {}", proven, total);
      let mut impls: Vec<&str> = reg.keys().copied().collect();
      impls.sort();
      eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
      eprintln!("proven canonical ops ({}): {}", proven_keys.len(), proven_keys.iter().cloned().collect::<Vec<_>>().join(", "));
      eprintln!("backlog/unmapped ({}):", unmapped.len());
      for u in &unmapped { eprintln!("    {}", u); }

      assert!(failures.is_empty(), "registered reduction op(s) FAILED oracle: {:?}", failures);
      assert!(proven > 0, "zero reduction items proven");
}
