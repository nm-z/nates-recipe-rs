mod common;
// Live-GPU proof harness for the "activation" inventory category.
//
// For every activation-category item in kernel_inventory/*.json, canonicalize
// its name; if that canonical op is registered here, run the gpu-core op on the
// LIVE gfx1101 GPU and assert it matches an AUTHORITATIVE oracle (textbook
// formula / std f64 / libm). tol 1e-6.
//
// New ops live in activationx.hip (this task). Existing public activation ops
// (kernels.rs, k_actx, k_gapact) are ALSO registered so canon collapses the big
// inventory clusters (relu/sigmoid/gelu/softmax/... across every framework) onto
// their proven on-device implementation. Stochastic items (dropout family),
// derivatives (*_backward/*Grad/*_bwd) and structural items (keras.layers.*,
// stax.*, cutlass epilogues, generic Activation) stay backlog — not failures.

use gpu_core::memory::GpuBuffer;
use gpu_core::hip::HipError;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

// ── new activationx_ launchers ──────────────────────────────────────────────
unsafe extern "C" {
      fn launch_activationx_relu_squared(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
      fn launch_activationx_shifted_softplus(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
      fn launch_activationx_squareplus(x: *const c_void, o: *mut c_void, n: i32, b: f64, s: *mut c_void);
      fn launch_activationx_star_relu(x: *const c_void, o: *mut c_void, n: i32, sc: f64, b: f64, s: *mut c_void);
      fn launch_activationx_prelu(x: *const c_void, a: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
      fn launch_activationx_glu(x: *const c_void, o: *mut c_void, half: i32, s: *mut c_void);
      fn launch_activationx_reglu(x: *const c_void, o: *mut c_void, half: i32, s: *mut c_void);
      fn launch_activationx_crelu(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
      fn launch_activationx_softmin(x: *const c_void, o: *mut c_void, rows: i32, cols: i32, s: *mut c_void);
}

const TOL: f64 = 1e-6;

fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
      (0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.5) / n as f64).collect()
}
fn approx(g: f64, want: f64) -> bool {
      want.is_finite() && (g - want).abs() <= TOL * (1.0 + want.abs())
}

// ── runners ─────────────────────────────────────────────────────────────────
type LaunchU = unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void);
fn run_u(f: LaunchU, x: &[f64]) -> Vec<f64> {
      let b = GpuBuffer::upload(x).unwrap();
      let o = GpuBuffer::alloc(x.len()).unwrap();
      unsafe { f(b.ptr_raw() as *const c_void, o.ptr_raw(), x.len() as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut out = vec![0.0; x.len()];
      o.download(&mut out).unwrap();
      out
}

// existing k_* unary ops: (x, n) -> GpuBuffer
type Km = fn(&GpuBuffer, usize) -> Result<GpuBuffer, HipError>;
fn run_km(f: Km, x: &[f64]) -> Vec<f64> {
      let b = GpuBuffer::upload(x).unwrap();
      let o = f(&b, x.len()).unwrap();
      let mut out = vec![0.0; x.len()];
      o.download(&mut out).unwrap();
      out
}

struct UnaryOp { run: Box<dyn Fn(&[f64]) -> Vec<f64>>, oracle: Box<dyn Fn(f64) -> f64>, lo: f64, hi: f64 }

fn unary_registry() -> HashMap<&'static str, UnaryOp> {
      let mut m: HashMap<&'static str, UnaryOp> = HashMap::new();
      macro_rules! ax { ($k:literal, $launch:expr, $o:expr, $lo:expr, $hi:expr) => {
            m.insert($k, UnaryOp { run: Box::new(|x| run_u($launch, x)), oracle: Box::new($o), lo: $lo, hi: $hi });
      }; }
      macro_rules! km { ($k:literal, $g:expr, $o:expr, $lo:expr, $hi:expr) => {
            m.insert($k, UnaryOp { run: Box::new(|x| run_km($g, x)), oracle: Box::new($o), lo: $lo, hi: $hi });
      }; }

      // ── existing public activation ops (oracles copied verbatim from inventory_proof.rs) ──
      use gpu_core::kernels::{gpu_relu, gpu_sigmoid, gpu_tanh, gpu_silu};
      km!("relu",    gpu_relu,    |x| x.max(0.0), -3.0, 3.0);
      km!("sigmoid", gpu_sigmoid, |x| 1.0 / (1.0 + (-x).exp()), -6.0, 6.0);
      km!("tanh",    gpu_tanh,    |x| x.tanh(), -3.0, 3.0);
      km!("silu",    gpu_silu,    |x| x / (1.0 + (-x).exp()), -6.0, 6.0);

      use gpu_core::k_gapact::{gpu_selu, gpu_mish, gpu_softplus, gpu_hardswish};
      fn elu1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> { gpu_core::k_gapact::gpu_elu(x, n, 1.0) }
      km!("elu",       elu1,         |x| if x > 0.0 { x } else { x.exp() - 1.0 }, -3.0, 3.0);
      km!("selu",      gpu_selu,     |x| { let (a, l) = (1.6732632423543772, 1.0507009873554805); l * if x > 0.0 { x } else { a * (x.exp() - 1.0) } }, -3.0, 3.0);
      km!("mish",      gpu_mish,     |x| x * ((x.max(0.0) + (-x.abs()).exp().ln_1p()).tanh()), -3.0, 3.0);
      km!("softplus",  gpu_softplus, |x| x.max(0.0) + (-x.abs()).exp().ln_1p(), -3.0, 3.0);
      km!("hardswish", gpu_hardswish,|x| x * ((x + 3.0).clamp(0.0, 6.0)) / 6.0, -4.0, 4.0);

      use gpu_core::k_actx::{gpu_relu6, gpu_hardsigmoid, gpu_hardtanh, gpu_softsign, gpu_tanhshrink, gpu_logsigmoid, gpu_gelu_exact, gpu_softshrink};
      fn celu1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> { gpu_core::k_actx::gpu_celu(x, n, 1.0) }
      fn hardshrink05(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> { gpu_core::k_actx::gpu_hardshrink(x, n, 0.5) }
      fn thresh1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> { gpu_core::k_actx::gpu_thresholdedrelu(x, n, 1.0) }
      km!("relu6",          gpu_relu6,       |x| x.clamp(0.0, 6.0), -4.0, 8.0);
      km!("hardsigmoid",    gpu_hardsigmoid, |x| (x / 6.0 + 0.5).clamp(0.0, 1.0), -5.0, 5.0);
      km!("hardtanh",       gpu_hardtanh,    |x| x.clamp(-1.0, 1.0), -3.0, 3.0);
      km!("softsign",       gpu_softsign,    |x| x / (1.0 + x.abs()), -3.0, 3.0);
      km!("tanhshrink",     gpu_tanhshrink,  |x| x - x.tanh(), -3.0, 3.0);
      km!("logsigmoid",     gpu_logsigmoid,  |x| -((-x).max(0.0) + (-x.abs()).exp().ln_1p()), -3.0, 3.0);
      km!("gelu",           gpu_gelu_exact,  |x| 0.5 * x * (1.0 + libm::erf(x * 0.7071067811865476)), -3.0, 3.0);
      km!("softshrink",     gpu_softshrink,  |x| if x > 0.5 { x - 0.5 } else if x < -0.5 { x + 0.5 } else { 0.0 }, -3.0, 3.0);
      km!("celu",           celu1,           |x| x.max(0.0) + (1.0 * ((x).exp() - 1.0)).min(0.0), -3.0, 3.0);
      km!("hardshrink",     hardshrink05,    |x| if x.abs() > 0.5 { x } else { 0.0 }, -3.0, 3.0);
      km!("thresholdedrelu",thresh1,         |x| if x > 1.0 { x } else { 0.0 }, -3.0, 3.0);

      // ── new activationx_ ops (same-size elementwise, formula oracle) ──
      ax!("relu_squared",     launch_activationx_relu_squared,     |x: f64| { let r = x.max(0.0); r * r }, -3.0, 3.0);
      ax!("shifted_softplus", launch_activationx_shifted_softplus, |x: f64| (x.max(0.0) + (-x.abs()).exp().ln_1p()) - std::f64::consts::LN_2, -3.0, 3.0);
      m
}

// ── special-cased proofs for the multi-arg activationx ops ──────────────────
// squareplus(x; b) = (x + sqrt(x^2 + b)) / 2 ; b chosen != 0.
fn check_squareplus() -> Option<String> {
      let b = 4.0;
      let xs = probes(-3.0, 3.0, 32);
      let bx = GpuBuffer::upload(&xs).unwrap();
      let o = GpuBuffer::alloc(xs.len()).unwrap();
      unsafe { launch_activationx_squareplus(bx.ptr_raw() as *const c_void, o.ptr_raw(), xs.len() as i32, b, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut got = vec![0.0; xs.len()];
      o.download(&mut got).unwrap();
      for (x, g) in xs.iter().zip(&got) {
            let want = 0.5 * (x + (x * x + b).sqrt());
            if !approx(*g, want) { return Some(format!("squareplus(b={b}): x={x} got={g} want={want}")); }
      }
      None
}
// star_relu(x; s, b) = s * relu(x)^2 + b ; s != 1 && b != 0 so test is non-vacuous.
fn check_star_relu() -> Option<String> {
      let (sc, b) = (0.8, -0.5);
      let xs = probes(-3.0, 3.0, 32);
      let bx = GpuBuffer::upload(&xs).unwrap();
      let o = GpuBuffer::alloc(xs.len()).unwrap();
      unsafe { launch_activationx_star_relu(bx.ptr_raw() as *const c_void, o.ptr_raw(), xs.len() as i32, sc, b, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut got = vec![0.0; xs.len()];
      o.download(&mut got).unwrap();
      for (x, g) in xs.iter().zip(&got) {
            let r = x.max(0.0);
            let want = sc * r * r + b;
            if !approx(*g, want) { return Some(format!("star_relu(s={sc},b={b}): x={x} got={g} want={want}")); }
      }
      None
}
// prelu(x, alpha[]) = max(0,x) + alpha*min(0,x) ; per-element alpha array.
fn check_prelu() -> Option<String> {
      let xs = probes(-3.0, 3.0, 32);
      let alpha: Vec<f64> = (0..xs.len()).map(|i| 0.01 + 0.3 * (i as f64) / xs.len() as f64).collect();
      let bx = GpuBuffer::upload(&xs).unwrap();
      let ba = GpuBuffer::upload(&alpha).unwrap();
      let o = GpuBuffer::alloc(xs.len()).unwrap();
      unsafe { launch_activationx_prelu(bx.ptr_raw() as *const c_void, ba.ptr_raw() as *const c_void, o.ptr_raw(), xs.len() as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut got = vec![0.0; xs.len()];
      o.download(&mut got).unwrap();
      for ((x, a), g) in xs.iter().zip(&alpha).zip(&got) {
            let want = if *x > 0.0 { *x } else { a * x };
            if !approx(*g, want) { return Some(format!("prelu: x={x} a={a} got={g} want={want}")); }
      }
      None
}
// glu(x): contiguous split; out[i] = a[i] * sigmoid(b[i]).
fn check_glu() -> Option<String> {
      let half = 24usize;
      let mut x: Vec<f64> = probes(-3.0, 3.0, half);
      x.extend(probes(-5.0, 5.0, half)); // gate part b
      let bx = GpuBuffer::upload(&x).unwrap();
      let o = GpuBuffer::alloc(half).unwrap();
      unsafe { launch_activationx_glu(bx.ptr_raw() as *const c_void, o.ptr_raw(), half as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut got = vec![0.0; half];
      o.download(&mut got).unwrap();
      for i in 0..half {
            let (a, b) = (x[i], x[i + half]);
            let want = a * (1.0 / (1.0 + (-b).exp()));
            if !approx(got[i], want) { return Some(format!("glu: i={i} got={} want={want}", got[i])); }
      }
      None
}
// reglu(x): contiguous split; out[i] = a[i] * relu(b[i]).
fn check_reglu() -> Option<String> {
      let half = 24usize;
      let mut x: Vec<f64> = probes(-3.0, 3.0, half);
      x.extend(probes(-5.0, 5.0, half));
      let bx = GpuBuffer::upload(&x).unwrap();
      let o = GpuBuffer::alloc(half).unwrap();
      unsafe { launch_activationx_reglu(bx.ptr_raw() as *const c_void, o.ptr_raw(), half as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut got = vec![0.0; half];
      o.download(&mut got).unwrap();
      for i in 0..half {
            let (a, b) = (x[i], x[i + half]);
            let want = a * b.max(0.0);
            if !approx(got[i], want) { return Some(format!("reglu: i={i} got={} want={want}", got[i])); }
      }
      None
}
// crelu(x) = concat(relu(x), relu(-x)), out length 2n.
fn check_crelu() -> Option<String> {
      let xs = probes(-3.0, 3.0, 32);
      let n = xs.len();
      let bx = GpuBuffer::upload(&xs).unwrap();
      let o = GpuBuffer::alloc(2 * n).unwrap();
      unsafe { launch_activationx_crelu(bx.ptr_raw() as *const c_void, o.ptr_raw(), n as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut got = vec![0.0; 2 * n];
      o.download(&mut got).unwrap();
      for (i, x) in xs.iter().enumerate() {
            let want_pos = x.max(0.0);
            let want_neg = (-x).max(0.0);
            if !approx(got[i], want_pos) { return Some(format!("crelu pos: i={i} got={} want={want_pos}", got[i])); }
            if !approx(got[i + n], want_neg) { return Some(format!("crelu neg: i={i} got={} want={want_neg}", got[i + n])); }
      }
      None
}
// softmin(x) = softmax(-x) row-wise. Oracle = CPU stable softmax of -x.
fn check_softmin() -> Option<String> {
      let (rows, cols) = (5usize, 7usize);
      let mut x = vec![0.0f64; rows * cols];
      for r in 0..rows { for c in 0..cols { x[r * cols + c] = ((r as f64) - 2.0) + 0.5 * (c as f64) - 1.5; } }
      let bx = GpuBuffer::upload(&x).unwrap();
      let o = GpuBuffer::alloc(rows * cols).unwrap();
      unsafe { launch_activationx_softmin(bx.ptr_raw() as *const c_void, o.ptr_raw(), rows as i32, cols as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut got = vec![0.0; rows * cols];
      o.download(&mut got).unwrap();
      for r in 0..rows {
            let row = &x[r * cols..(r + 1) * cols];
            let neg: Vec<f64> = row.iter().map(|v| -v).collect();
            let mx = neg.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let exps: Vec<f64> = neg.iter().map(|v| (v - mx).exp()).collect();
            let sum: f64 = exps.iter().sum();
            for c in 0..cols {
                  let want = exps[c] / sum;
                  if !approx(got[r * cols + c], want) {
                        return Some(format!("softmin: r={r} c={c} got={} want={want}", got[r * cols + c]));
                  }
            }
      }
      None
}

// ── canon: activation JSON name -> registry key (true synonyms only) ─────────
fn canon(name: &str) -> String {
      // last path/namespace segment, lowercased
      let mut base = name.rsplit(['.', ':', '$', '/']).next().unwrap_or(name).to_lowercase();
      // strip framework-specific prefixes baked into the leaf
      for p in ["miopenactivationforward_", "cudnnactivation_", "cudnnpointwise_", "cutensorop_", "linearcombination"] {
            if let Some(s) = base.strip_prefix(p) { base = s.to_string(); }
      }
      base = base.trim_start_matches('_').to_string();
      let alias: &[(&str, &str)] = &[
            // relu family (true relu only; relu6 / leaky distinct)
            ("relu", "relu"), ("reluop", "relu"), ("cudnn_relu", "relu"),
            ("relu6", "relu6"),
            // sigmoid
            ("sigmoid", "sigmoid"),
            // tanh
            ("tanh", "tanh"),
            // silu / swish are the same function
            ("silu", "silu"), ("swish", "silu"), ("siluop", "silu"), ("fused_silu", "silu"),
            // gelu EXACT only (erf). tanh/new/fast/quick/approx variants -> backlog.
            ("gelu", "gelu"), ("geluop", "gelu"),
            // elu / selu / celu
            ("elu", "elu"), ("selu", "selu"), ("celu", "celu"),
            // mish / softplus / softsign
            ("mish", "mish"), ("softplus", "softplus"), ("softrelu", "softplus"),
            ("softsign", "softsign"), ("soft_sign", "softsign"),
            // hard variants (distinct from smooth)
            ("hardswish", "hardswish"), ("hard_swish", "hardswish"),
            ("hardsigmoid", "hardsigmoid"), ("hard_sigmoid", "hardsigmoid"),
            ("hardtanh", "hardtanh"), ("hard_tanh", "hardtanh"),
            // shrink / logsigmoid / thresholded
            ("tanhshrink", "tanhshrink"), ("softshrink", "softshrink"), ("hardshrink", "hardshrink"),
            ("logsigmoid", "logsigmoid"), ("log_sigmoid", "logsigmoid"),
            ("thresholdedrelu", "thresholdedrelu"),
            // softmax (accurate/forward) and log_softmax
            ("softmax", "softmax"), ("softmaxop", "softmax"), ("soft_max", "softmax"),
            ("softmax_accurate", "softmax"), ("softmaxforward", "softmax"),
            ("logsoftmax", "log_softmax"), ("log_softmax", "log_softmax"), ("softmax_log", "log_softmax"),
            // gated
            ("swiglu", "swiglu"), ("geglu", "geglu"),
            // new activationx ops
            ("prelu", "prelu"),
            ("glu", "glu"), ("reglu", "reglu"), ("crelu", "crelu"),
            ("softmin", "softmin"),
            ("relu_squared", "relu_squared"), ("squareplus", "squareplus"),
            ("star_relu", "star_relu"), ("shifted_softplus", "shifted_softplus"),
      ];
      for (a, c) in alias { if base == *a { return c.to_string(); } }
      base
}

fn load_activation() -> Vec<String> {
      let dir = common::inventory_dir();
      let mut items = Vec::new();
      for e in std::fs::read_dir(&dir).expect("no kernel_inventory").flatten() {
            let p = e.path();
            if p.extension().map_or(false, |x| x == "json") {
                  let Ok(txt) = std::fs::read_to_string(&p) else { continue; };
                  let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else { continue; };
                  if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
                        for k in ks {
                              if k.get("category").and_then(|c| c.as_str()) != Some("activation") { continue; }
                              if let Some(name) = k.get("name").and_then(|n| n.as_str()) {
                                    if !name.is_empty() { items.push(name.to_string()); }
                              }
                        }
                  }
            }
      }
      items.sort();
      items.dedup();
      items
}

#[test]
fn prove_activation() {
      let items = load_activation();
      assert!(!items.is_empty(), "no activation items in inventory");
      let reg = unary_registry();

      let mut failures: Vec<String> = Vec::new();
      let mut op_ok: HashMap<&str, bool> = HashMap::new();

      // ── unary registry ops (existing + new same-size activationx) ──
      for (k, op) in reg.iter() {
            let xs = probes(op.lo, op.hi, 32);
            let got = (op.run)(&xs);
            let ok = xs.iter().zip(&got).all(|(x, g)| approx(*g, (op.oracle)(*x)));
            op_ok.insert(*k, ok);
            if !ok { failures.push(format!("registered op {k} FAILED oracle")); }
      }

      // ── row-wise softmax / log_softmax (existing kernels, (rows,cols) sig) ──
      {
            use gpu_core::kernels::{gpu_softmax_rows, gpu_log_softmax_rows};
            let (rows, cols) = (5usize, 7usize);
            let mut x = vec![0.0f64; rows * cols];
            for r in 0..rows { for c in 0..cols { x[r * cols + c] = ((r as f64) - 2.0) + 0.4 * (c as f64) - 1.2; } }
            let bx = GpuBuffer::upload(&x).unwrap();

            let sm = gpu_softmax_rows(&bx, rows, cols).unwrap();
            let mut gsm = vec![0.0; rows * cols]; sm.download(&mut gsm).unwrap();
            let lsm = gpu_log_softmax_rows(&bx, rows, cols).unwrap();
            let mut glsm = vec![0.0; rows * cols]; lsm.download(&mut glsm).unwrap();

            let mut sm_ok = true; let mut lsm_ok = true;
            for r in 0..rows {
                  let row = &x[r * cols..(r + 1) * cols];
                  let mx = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                  let exps: Vec<f64> = row.iter().map(|v| (v - mx).exp()).collect();
                  let sum: f64 = exps.iter().sum();
                  for c in 0..cols {
                        let w_sm = exps[c] / sum;
                        let w_lsm = (row[c] - mx) - sum.ln();
                        if !approx(gsm[r * cols + c], w_sm) { sm_ok = false; }
                        if !approx(glsm[r * cols + c], w_lsm) { lsm_ok = false; }
                  }
            }
            op_ok.insert("softmax", sm_ok);
            op_ok.insert("log_softmax", lsm_ok);
            if !sm_ok { failures.push("softmax row FAILED oracle".into()); }
            if !lsm_ok { failures.push("log_softmax row FAILED oracle".into()); }
      }

      // ── leaky_relu (existing, has alpha param) ──
      {
            use gpu_core::kernels::gpu_leaky_relu;
            let alpha = 0.01;
            let xs = probes(-3.0, 3.0, 32);
            let bx = GpuBuffer::upload(&xs).unwrap();
            let o = gpu_leaky_relu(&bx, xs.len(), alpha).unwrap();
            let mut got = vec![0.0; xs.len()]; o.download(&mut got).unwrap();
            let ok = xs.iter().zip(&got).all(|(x, g)| approx(*g, if *x > 0.0 { *x } else { alpha * x }));
            op_ok.insert("leaky_relu", ok);
            if !ok { failures.push("leaky_relu FAILED oracle".into()); }
      }

      // ── gated swiglu / geglu (existing, binary a,b) ──
      {
            use gpu_core::k_gapact::{gpu_swiglu, gpu_geglu};
            let a = probes(-3.0, 3.0, 32);
            let b = probes(-5.0, 5.0, 32);
            let ba = GpuBuffer::upload(&a).unwrap();
            let bb = GpuBuffer::upload(&b).unwrap();
            let sw = gpu_swiglu(&ba, &bb, a.len()).unwrap();
            let mut gsw = vec![0.0; a.len()]; sw.download(&mut gsw).unwrap();
            let ge = gpu_geglu(&ba, &bb, a.len()).unwrap();
            let mut gge = vec![0.0; a.len()]; ge.download(&mut gge).unwrap();
            let mut sw_ok = true; let mut ge_ok = true;
            for i in 0..a.len() {
                  let w_sw = a[i] * (b[i] / (1.0 + (-b[i]).exp()));
                  let w_ge = a[i] * (0.5 * b[i] * (1.0 + libm::erf(b[i] * 0.7071067811865476)));
                  if !approx(gsw[i], w_sw) { sw_ok = false; }
                  if !approx(gge[i], w_ge) { ge_ok = false; }
            }
            op_ok.insert("swiglu", sw_ok);
            op_ok.insert("geglu", ge_ok);
            if !sw_ok { failures.push("swiglu FAILED oracle".into()); }
            if !ge_ok { failures.push("geglu FAILED oracle".into()); }
      }

      // ── new multi-arg / shape-changing activationx ops ──
      {
            let multis: [(&str, fn() -> Option<String>); 7] = [
                  ("squareplus", check_squareplus),
                  ("star_relu", check_star_relu),
                  ("prelu", check_prelu),
                  ("glu", check_glu),
                  ("reglu", check_reglu),
                  ("crelu", check_crelu),
                  ("softmin", check_softmin),
            ];
            for (k, f) in multis {
                  match f() {
                        None => { op_ok.insert(k, true); }
                        Some(msg) => { op_ok.insert(k, false); failures.push(msg); }
                  }
            }
      }

      // ── walk inventory: an item is proven if its canon maps to a passing op ──
      let total = items.len();
      let mut proven = 0usize;
      let mut proven_keys: BTreeSet<String> = Default::default();
      for name in &items {
            let key = canon(name);
            if let Some(&ok) = op_ok.get(key.as_str()) {
                  if ok { proven += 1; proven_keys.insert(key); }
            }
      }

      let implemented = "prelu, glu, reglu, crelu, softmin, relu_squared, squareplus, star_relu, shifted_softplus";

      eprintln!("\n=== PROVE activation ===");
      eprintln!("PROVE activation: {} / {}", proven, total);
      let mut keys: Vec<&str> = op_ok.keys().copied().collect();
      keys.sort();
      eprintln!("registered ops ({}): {}", keys.len(), keys.join(", "));
      eprintln!("proven canonical ops ({}): {}", proven_keys.len(), proven_keys.iter().cloned().collect::<Vec<_>>().join(", "));
      eprintln!("new ops implemented: {}", implemented);

      assert!(failures.is_empty(), "activation op(s) FAILED oracle: {:?}", failures);
      assert!(proven > 0, "zero activation items proven");

      eprintln!("RESULT activation: proven={} total={} green=true implemented={}", proven, total, implemented);
}
