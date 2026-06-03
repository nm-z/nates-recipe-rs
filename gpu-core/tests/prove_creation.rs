// Live-GPU proof harness for the "creation" inventory category.
//
// For every creation-category item in kernel_inventory/*.json, canonicalize its
// name; if that canonical generator is registered here, run the gpu-core op on the
// LIVE gfx1101 GPU and assert it matches an AUTHORITATIVE CPU oracle (the defining
// sequence). tol 1e-6 relative. Generators are output-only: no input buffer is
// read, the kernel synthesizes out[i] from i and the scalar params.
//
// Reuses existing kernels where they already cover the op (gpu_fill for
// full/zeros/ones/constant, gpu_eye for square identity, gpu_iota for int
// sequences) and adds new creationx_ kernels only for genuine gaps (arange,
// linspace, logspace, geomspace, tri, rectangular eye).
//
// HONESTY: empty/empty_like return uninitialized memory — no oracle exists, so
// they are NOT mapped to any op (left as backlog, never faked as zeros). full is
// proven with a NONTRIVIAL value so we prove a kernel wrote it, not that alloc
// returned zeros. The test FAILS on any registered-op mismatch (a real bug).

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

// ── new creationx_ launchers (output-first, then f64/i32 params, then n, stream) ─
unsafe extern "C" {
      fn launch_creationx_arange(out: *mut c_void, start: f64, step: f64, n: i32, s: *mut c_void);
      fn launch_creationx_linspace(out: *mut c_void, start: f64, stop: f64, n: i32, s: *mut c_void);
      fn launch_creationx_logspace(out: *mut c_void, start: f64, stop: f64, base: f64, n: i32, s: *mut c_void);
      fn launch_creationx_geomspace(out: *mut c_void, start: f64, stop: f64, n: i32, s: *mut c_void);
      fn launch_creationx_tri(out: *mut c_void, rows: i32, cols: i32, k: i32, s: *mut c_void);
      fn launch_creationx_eye_rect(out: *mut c_void, rows: i32, cols: i32, k: i32, s: *mut c_void);
      fn launch_creationx_arange_i32(out: *mut c_void, start: i32, step: i32, n: i32, s: *mut c_void);
}

const TOL: f64 = 1e-6;

fn close(a: &[f64], b: &[f64]) -> bool {
      a.len() == b.len() && a.iter().zip(b).all(|(g, w)| w.is_finite() && (g - w).abs() <= TOL * (1.0 + w.abs()))
}

// ── GPU runners (download f64) ─────────────────────────────────────────────────
fn run_arange(start: f64, step: f64, n: usize) -> Vec<f64> {
      let o = GpuBuffer::alloc(n).unwrap();
      unsafe { launch_creationx_arange(o.ptr_raw(), start, step, n as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut v = vec![0.0; n]; o.download(&mut v).unwrap(); v
}
fn run_linspace(start: f64, stop: f64, n: usize) -> Vec<f64> {
      let o = GpuBuffer::alloc(n).unwrap();
      unsafe { launch_creationx_linspace(o.ptr_raw(), start, stop, n as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut v = vec![0.0; n]; o.download(&mut v).unwrap(); v
}
fn run_logspace(start: f64, stop: f64, base: f64, n: usize) -> Vec<f64> {
      let o = GpuBuffer::alloc(n).unwrap();
      unsafe { launch_creationx_logspace(o.ptr_raw(), start, stop, base, n as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut v = vec![0.0; n]; o.download(&mut v).unwrap(); v
}
fn run_geomspace(start: f64, stop: f64, n: usize) -> Vec<f64> {
      let o = GpuBuffer::alloc(n).unwrap();
      unsafe { launch_creationx_geomspace(o.ptr_raw(), start, stop, n as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut v = vec![0.0; n]; o.download(&mut v).unwrap(); v
}
fn run_tri(rows: usize, cols: usize, k: i32) -> Vec<f64> {
      let o = GpuBuffer::alloc(rows * cols).unwrap();
      unsafe { launch_creationx_tri(o.ptr_raw(), rows as i32, cols as i32, k, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut v = vec![0.0; rows * cols]; o.download(&mut v).unwrap(); v
}
fn run_eye_rect(rows: usize, cols: usize, k: i32) -> Vec<f64> {
      let o = GpuBuffer::alloc(rows * cols).unwrap();
      unsafe { launch_creationx_eye_rect(o.ptr_raw(), rows as i32, cols as i32, k, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut v = vec![0.0; rows * cols]; o.download(&mut v).unwrap(); v
}
fn run_arange_i32(start: i32, step: i32, n: usize) -> Vec<i32> {
      let o = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>()).unwrap();
      unsafe { launch_creationx_arange_i32(o.ptr_raw(), start, step, n as i32, std::ptr::null_mut()); }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut v = vec![0i32; n]; o.download_i32(&mut v).unwrap(); v
}

// ── proofs (run each registered generator ONCE vs oracle + defining edges) ──────
// Returns (map op_name -> pass/fail, failure messages).
fn run_proofs() -> (HashMap<&'static str, bool>, Vec<String>) {
      let mut ok: HashMap<&'static str, bool> = HashMap::new();
      let mut fails: Vec<String> = Vec::new();
      macro_rules! prove { ($k:literal, $cond:expr) => {{
            let pass = $cond;
            ok.insert($k, pass);
            if !pass { fails.push($k.to_string()); }
      }}; }

      // fill / full / constant / zeros / ones — reuse gpu_fill(n, val).
      // Prove with a NONTRIVIAL value (3.14159) so we prove a kernel WROTE it.
      {
            use gpu_core::kernels::gpu_fill;
            let n = 37usize;
            let val = 3.14159265358979_f64;
            let g = gpu_fill(n, val).unwrap();
            let mut got = vec![0.0; n]; g.download(&mut got).unwrap();
            let want = vec![val; n];
            prove!("full", close(&got, &want));
            // zeros & ones ride the same proven fill kernel, but assert distinctly.
            let z = gpu_fill(n, 0.0).unwrap(); let mut gz = vec![9.0; n]; z.download(&mut gz).unwrap();
            prove!("zeros", gz.iter().all(|v| *v == 0.0));
            let o = gpu_fill(n, 1.0).unwrap(); let mut go = vec![0.0; n]; o.download(&mut go).unwrap();
            prove!("ones", go.iter().all(|v| *v == 1.0));
      }

      // eye / identity — square via gpu_eye; rectangular+offset via creationx_eye_rect.
      {
            use gpu_core::kernels::gpu_eye;
            let n = 5usize;
            let g = gpu_eye(n).unwrap(); let mut got = vec![0.0; n * n]; g.download(&mut got).unwrap();
            // diagonal == 1, off-diagonal == 0 (assert BOTH).
            let mut diag_ok = true; let mut off_ok = true;
            for r in 0..n { for c in 0..n {
                  let v = got[r * n + c];
                  if r == c { if (v - 1.0).abs() > TOL { diag_ok = false; } }
                  else if v.abs() > TOL { off_ok = false; }
            }}
            // rectangular eye with k offset vs CPU oracle.
            let (rr, cc, k) = (4usize, 6usize, 1i32);
            let re = run_eye_rect(rr, cc, k);
            let mut want = vec![0.0; rr * cc];
            for r in 0..rr { for c in 0..cc { if c as i32 == r as i32 + k { want[r * cc + c] = 1.0; } } }
            prove!("eye", diag_ok && off_ok && close(&re, &want));
      }

      // arange: out[i] = start + i*step. Test negative + fractional step.
      {
            let (start, step, n) = (-2.0_f64, 0.25_f64, 30usize);
            let got = run_arange(start, step, n);
            let want: Vec<f64> = (0..n).map(|i| start + i as f64 * step).collect();
            // also a negative step run.
            let got2 = run_arange(5.0, -0.5, 20);
            let want2: Vec<f64> = (0..20).map(|i| 5.0 - 0.5 * i as f64).collect();
            prove!("arange", close(&got, &want) && close(&got2, &want2));
      }

      // linspace: endpoint-INCLUSIVE. assert out[n-1]==stop and out[0]==start; n==1 edge.
      {
            let (start, stop, n) = (-1.5_f64, 4.5_f64, 16usize);
            let got = run_linspace(start, stop, n);
            let want: Vec<f64> = (0..n).map(|i| start + (stop - start) * i as f64 / (n - 1) as f64).collect();
            let edge_first = (got[0] - start).abs() <= TOL;
            let edge_last = (got[n - 1] - stop).abs() <= TOL;
            let one = run_linspace(7.0, 99.0, 1);
            let edge_one = one.len() == 1 && (one[0] - 7.0).abs() <= TOL;
            prove!("linspace", close(&got, &want) && edge_first && edge_last && edge_one);
      }

      // logspace: out[i] = base^linspace(start,stop,n)[i].
      {
            let (start, stop, base, n) = (-1.0_f64, 3.0_f64, 10.0_f64, 12usize);
            let got = run_logspace(start, stop, base, n);
            let want: Vec<f64> = (0..n).map(|i| {
                  let e = start + (stop - start) * i as f64 / (n - 1) as f64;
                  base.powf(e)
            }).collect();
            // base-2 variant
            let got2 = run_logspace(0.0, 5.0, 2.0, 6);
            let want2: Vec<f64> = (0..6).map(|i| 2.0_f64.powf(5.0 * i as f64 / 5.0)).collect();
            prove!("logspace", close(&got, &want) && close(&got2, &want2));
      }

      // geomspace: out[i] = start*(stop/start)^(i/(n-1)). assert endpoints exact.
      {
            let (start, stop, n) = (2.0_f64, 256.0_f64, 9usize);
            let got = run_geomspace(start, stop, n);
            let want: Vec<f64> = (0..n).map(|i| start * (stop / start).powf(i as f64 / (n - 1) as f64)).collect();
            let edge = (got[0] - start).abs() <= TOL && (got[n - 1] - stop).abs() <= TOL;
            prove!("geomspace", close(&got, &want) && edge);
      }

      // tri: lower-triangular ones with diagonal offset k.
      {
            let (rr, cc, k) = (5usize, 5usize, 0i32);
            let got = run_tri(rr, cc, k);
            let mut want = vec![0.0; rr * cc];
            for r in 0..rr { for c in 0..cc { if c as i32 <= r as i32 + k { want[r * cc + c] = 1.0; } } }
            // offset variant (k=1)
            let got2 = run_tri(4, 6, 1);
            let mut want2 = vec![0.0; 24];
            for r in 0..4 { for c in 0..6 { if c as i32 <= r as i32 + 1 { want2[r * 6 + c] = 1.0; } } }
            prove!("tri", close(&got, &want) && close(&got2, &want2));
      }

      // iota (int dtype): reuse gpu_iota -> [0..n-1] (download i32).
      // creationx_arange_i32 proves the general integer sequence start+i*step.
      {
            use gpu_core::catboost::gpu_iota;
            let n = 41usize;
            let g = gpu_iota(n).unwrap();
            let mut got = vec![0i32; n]; g.download_i32(&mut got).unwrap();
            let want: Vec<i32> = (0..n as i32).collect();
            let iota_ok = got == want;
            // general int arange: start=3, step=2
            let ga = run_arange_i32(3, 2, 20);
            let wa: Vec<i32> = (0..20).map(|i| 3 + 2 * i).collect();
            prove!("iota", iota_ok && ga == wa);
      }

      (ok, fails)
}

// ── canonicalize a creation JSON name to a registry generator key ───────────────
// Mirrors inventory_proof.rs: strip lib/namespace prefix to last segment, lowercase.
// Map TRUE synonyms only. *_like variants of fill-style ops share the value-fill
// semantics (zeros_like fills 0, ones_like fills 1, full_like fills val) so they
// map to the same proven kernel. empty/empty_like are DELIBERATELY left unmapped
// (uninitialized memory has no oracle — never faked as zeros).
fn canon(name: &str) -> String {
      let base = name.rsplit(['.', ':', '$']).next().unwrap_or(name).to_lowercase();
      let alias: &[(&str, &str)] = &[
            // value-fill family -> gpu_fill
            ("fill", "full"), ("fill_n", "full"), ("full", "full"), ("full_like", "full"),
            ("constant", "full"), ("tensor_constant", "full"), ("tensorfill", "full"),
            ("device_fill", "full"), ("iconstantlayer", "full"), ("ifilllayer", "full"),
            ("zeros", "zeros"), ("zeros_like", "zeros"), ("zero_", "zeros"),
            ("ones", "ones"), ("ones_like", "ones"),
            // identity matrices
            ("eye", "eye"), ("identity", "eye"),
            // ranges / sequences (f64)
            ("arange", "arange"), ("range", "arange"), ("sequence", "arange"), ("tabulate", "arange"),
            ("linspace", "linspace"),
            ("logspace", "logspace"),
            ("geomspace", "geomspace"),
            // triangular ones generator
            ("tri", "tri"),
            // integer sequences -> gpu_iota / creationx_arange_i32
            ("iota", "iota"), ("vector_iota", "iota"), ("broadcasted_iota", "iota"),
            ("stablehlo_iota", "iota"),
      ];
      for (a, c) in alias { if base == *a { return c.to_string(); } }
      base
}

fn load_creation() -> Vec<String> {
      let dir = format!("{}/../kernel_inventory", env!("CARGO_MANIFEST_DIR"));
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
                              if cat != "creation" { continue; }
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
fn prove_creation() {
      let items = load_creation();
      assert!(!items.is_empty(), "no creation items in inventory");

      let (op_ok, failures) = run_proofs();

      // Walk inventory: each item whose canon maps to a passing registered op is proven.
      let total = items.len();
      let mut proven = 0usize;
      let mut proven_keys: std::collections::BTreeSet<String> = Default::default();
      for name in &items {
            let key = canon(name);
            if let Some(&ok) = op_ok.get(key.as_str()) {
                  if ok { proven += 1; proven_keys.insert(key); }
            }
      }

      eprintln!("\n=== PROVE creation ===");
      eprintln!("PROVE creation: {} / {}", proven, total);
      let mut impls: Vec<&str> = op_ok.keys().copied().collect();
      impls.sort();
      eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
      eprintln!("proven canonical ops ({}): {}", proven_keys.len(), proven_keys.iter().cloned().collect::<Vec<_>>().join(", "));
      eprintln!("note: empty/empty_like = uninitialized memory (no oracle) -> backlog, not faked");

      assert!(failures.is_empty(), "registered creation op(s) FAILED oracle: {:?}", failures);
      assert!(proven > 0, "zero creation items proven");
}
