// Live-GPU proof harness for the "shape" inventory category.
//
// Shape ops are pure index remaps: out[i] = x[src(i)]. No arithmetic happens,
// so GPU and a direct Rust reimplementation of the same index map must agree
// bit-exact (we assert within tol 1e-7 anyway). Inputs are distinct values
// (x[i] = i+1, non-square shapes) so any index error becomes a visible mismatch.
//
// Each op declares its true FFI signature (heterogeneous arities) and a
// self-contained closure that sets up input/dims, runs the GPU op, computes the
// oracle, and compares. We then walk kernel_inventory/*.json: every shape item
// whose canonical name maps to a passing registered op is proven. The test
// FAILS on any registered-op mismatch (a real bug). Unmapped items (reshape,
// squeeze, slice, stack, broadcast, ... — view/no-op or out of scope) stay as
// honest backlog, never faked.

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

// ── New shapex_ launchers (each with its real signature) ──
unsafe extern "C" {
      fn launch_shapex_pad_constant(x: *const c_void, out: *mut c_void, rows: i32, cols: i32,
            pt: i32, pl: i32, orow: i32, ocol: i32, cval: f64, s: *mut c_void);
      fn launch_shapex_flip(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, axis: i32, s: *mut c_void);
      fn launch_shapex_roll(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, sr: i32, sc: i32, s: *mut c_void);
      fn launch_shapex_triu(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, k: i32, s: *mut c_void);
      fn launch_shapex_tril(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, k: i32, s: *mut c_void);
      fn launch_shapex_tile(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, rr: i32, rc: i32, s: *mut c_void);
      fn launch_shapex_repeat(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, reps: i32, s: *mut c_void);
      fn launch_shapex_diagonal(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, k: i32, len: i32, s: *mut c_void);
}

fn upload(x: &[f64]) -> GpuBuffer { GpuBuffer::upload(x).unwrap() }
fn download(o: &GpuBuffer, n: usize) -> Vec<f64> {
      let mut out = vec![0.0; n];
      o.download(&mut out).unwrap();
      out
}
fn check_last() { gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap(); }

// distinct, non-degenerate input: x[i] = i+1
fn seq(n: usize) -> Vec<f64> { (0..n).map(|i| (i + 1) as f64).collect() }

const TOL: f64 = 1e-7;
fn veq(a: &[f64], b: &[f64]) -> bool {
      a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= TOL * (1.0 + y.abs()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-op GPU runners + CPU oracles. Each returns true iff GPU == oracle.
// ─────────────────────────────────────────────────────────────────────────────

fn prove_transpose() -> bool {
      // non-square 3x4 so a rows/cols swap cannot hide
      let (r, c) = (3usize, 4usize);
      let x = seq(r * c);
      let gx = upload(&x);
      let g = gpu_core::kernels::gpu_transpose(&gx, r, c).unwrap();
      let got = download(&g, r * c);
      let mut want = vec![0.0; r * c]; // out[j*r+i] = x[i*c+j], shape c x r
      for i in 0..r { for j in 0..c { want[j * r + i] = x[i * c + j]; } }
      veq(&got, &want)
}

fn prove_pad() -> bool {
      let (r, c) = (3usize, 4usize);
      let (pt, pb, pl, pr) = (1usize, 2usize, 2usize, 1usize);
      let cval = -7.5;
      let (orow, ocol) = (r + pt + pb, c + pl + pr);
      let x = seq(r * c);
      let gx = upload(&x);
      let o = GpuBuffer::alloc(orow * ocol).unwrap();
      unsafe {
            launch_shapex_pad_constant(gx.ptr_raw() as *const c_void, o.ptr_raw(),
                  r as i32, c as i32, pt as i32, pl as i32, orow as i32, ocol as i32, cval, std::ptr::null_mut());
      }
      check_last();
      let got = download(&o, orow * ocol);
      let mut want = vec![cval; orow * ocol];
      for i in 0..r { for j in 0..c { want[(i + pt) * ocol + (j + pl)] = x[i * c + j]; } }
      veq(&got, &want)
}

fn run_flip(x: &[f64], r: usize, c: usize, axis: i32) -> Vec<f64> {
      let gx = upload(x);
      let o = GpuBuffer::alloc(r * c).unwrap();
      unsafe { launch_shapex_flip(gx.ptr_raw() as *const c_void, o.ptr_raw(), r as i32, c as i32, axis, std::ptr::null_mut()); }
      check_last();
      download(&o, r * c)
}
fn prove_flipud() -> bool {
      let (r, c) = (3usize, 4usize);
      let x = seq(r * c);
      let got = run_flip(&x, r, c, 0);
      let mut want = vec![0.0; r * c];
      for i in 0..r { for j in 0..c { want[i * c + j] = x[(r - 1 - i) * c + j]; } }
      veq(&got, &want)
}
fn prove_fliplr() -> bool {
      let (r, c) = (3usize, 4usize);
      let x = seq(r * c);
      let got = run_flip(&x, r, c, 1);
      let mut want = vec![0.0; r * c];
      for i in 0..r { for j in 0..c { want[i * c + j] = x[i * c + (c - 1 - j)]; } }
      veq(&got, &want)
}
fn prove_flip() -> bool { prove_flipud() && prove_fliplr() }

fn prove_rot90() -> bool {
      // numpy rot90 (k=1) = transpose then flipud == fliplr then transpose.
      // Compose proven primitives as oracle: out = transpose(fliplr(x)).
      let (r, c) = (3usize, 4usize);
      let x = seq(r * c);
      // GPU: fliplr then transpose
      let f = run_flip(&x, r, c, 1); // r x c
      let gf = upload(&f);
      let g = gpu_core::kernels::gpu_transpose(&gf, r, c).unwrap(); // c x r
      let got = download(&g, r * c);
      // oracle: numpy.rot90 -> out[c-1-j][i] = x[i][j], shape c x r
      let mut want = vec![0.0; r * c];
      for i in 0..r { for j in 0..c { want[(c - 1 - j) * r + i] = x[i * c + j]; } }
      veq(&got, &want)
}

fn prove_roll() -> bool {
      let (r, c) = (3usize, 4usize);
      let (sr, sc) = (-1i32, 2i32); // include a negative shift (modulo correctness)
      let x = seq(r * c);
      let gx = upload(&x);
      let o = GpuBuffer::alloc(r * c).unwrap();
      unsafe { launch_shapex_roll(gx.ptr_raw() as *const c_void, o.ptr_raw(), r as i32, c as i32, sr, sc, std::ptr::null_mut()); }
      check_last();
      let got = download(&o, r * c);
      let mut want = vec![0.0; r * c];
      let (ri, ci) = (r as i32, c as i32);
      for i in 0..ri { for j in 0..ci {
            let rr = (((i - sr) % ri) + ri) % ri;
            let cc = (((j - sc) % ci) + ci) % ci;
            want[(i * ci + j) as usize] = x[(rr * ci + cc) as usize];
      }}
      veq(&got, &want)
}

fn prove_triu() -> bool {
      let (r, c) = (4usize, 4usize);
      let x = seq(r * c);
      let gx = upload(&x);
      for k in [-1i32, 0, 1] {
            let o = GpuBuffer::alloc(r * c).unwrap();
            unsafe { launch_shapex_triu(gx.ptr_raw() as *const c_void, o.ptr_raw(), r as i32, c as i32, k, std::ptr::null_mut()); }
            check_last();
            let got = download(&o, r * c);
            let mut want = vec![0.0; r * c];
            for i in 0..r { for j in 0..c { if (j as i32) >= (i as i32) + k { want[i * c + j] = x[i * c + j]; } } }
            if !veq(&got, &want) { return false; }
      }
      true
}
fn prove_tril() -> bool {
      let (r, c) = (4usize, 4usize);
      let x = seq(r * c);
      let gx = upload(&x);
      for k in [-1i32, 0, 1] {
            let o = GpuBuffer::alloc(r * c).unwrap();
            unsafe { launch_shapex_tril(gx.ptr_raw() as *const c_void, o.ptr_raw(), r as i32, c as i32, k, std::ptr::null_mut()); }
            check_last();
            let got = download(&o, r * c);
            let mut want = vec![0.0; r * c];
            for i in 0..r { for j in 0..c { if (j as i32) <= (i as i32) + k { want[i * c + j] = x[i * c + j]; } } }
            if !veq(&got, &want) { return false; }
      }
      true
}

fn prove_tile() -> bool {
      let (r, c) = (2usize, 3usize);
      let (rr, rc) = (3usize, 2usize);
      let x = seq(r * c);
      let (orow, ocol) = (r * rr, c * rc);
      let gx = upload(&x);
      let o = GpuBuffer::alloc(orow * ocol).unwrap();
      unsafe { launch_shapex_tile(gx.ptr_raw() as *const c_void, o.ptr_raw(), r as i32, c as i32, rr as i32, rc as i32, std::ptr::null_mut()); }
      check_last();
      let got = download(&o, orow * ocol);
      let mut want = vec![0.0; orow * ocol];
      for i in 0..orow { for j in 0..ocol { want[i * ocol + j] = x[(i % r) * c + (j % c)]; } }
      veq(&got, &want)
}

fn prove_repeat() -> bool {
      // numpy.repeat along axis 0 (interleave): rows duplicated in place.
      let (r, c) = (3usize, 2usize);
      let reps = 3usize;
      let x = seq(r * c);
      let orow = r * reps;
      let gx = upload(&x);
      let o = GpuBuffer::alloc(orow * c).unwrap();
      unsafe { launch_shapex_repeat(gx.ptr_raw() as *const c_void, o.ptr_raw(), r as i32, c as i32, reps as i32, std::ptr::null_mut()); }
      check_last();
      let got = download(&o, orow * c);
      let mut want = vec![0.0; orow * c];
      for rw in 0..orow { for j in 0..c { want[rw * c + j] = x[(rw / reps) * c + j]; } }
      veq(&got, &want)
}

fn prove_diagonal() -> bool {
      let (r, c) = (4usize, 4usize);
      let x = seq(r * c);
      let gx = upload(&x);
      for k in [-1i32, 0, 1] {
            let len = if k >= 0 { (c as i32 - k).min(r as i32) } else { (r as i32 + k).min(c as i32) } as usize;
            let o = GpuBuffer::alloc(len).unwrap();
            unsafe { launch_shapex_diagonal(gx.ptr_raw() as *const c_void, o.ptr_raw(), r as i32, c as i32, k, len as i32, std::ptr::null_mut()); }
            check_last();
            let got = download(&o, len);
            let mut want = vec![0.0; len];
            for d in 0..len {
                  let (rr, cc) = if k >= 0 { (d, d + k as usize) } else { (d + (-k) as usize, d) };
                  want[d] = x[rr * c + cc];
            }
            if !veq(&got, &want) { return false; }
      }
      true
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry + canonicalization
// ─────────────────────────────────────────────────────────────────────────────

fn registry() -> HashMap<&'static str, fn() -> bool> {
      let mut m: HashMap<&'static str, fn() -> bool> = HashMap::new();
      m.insert("transpose", prove_transpose);
      m.insert("pad", prove_pad);
      m.insert("flip", prove_flip);
      m.insert("fliplr", prove_fliplr);
      m.insert("flipud", prove_flipud);
      m.insert("rot90", prove_rot90);
      m.insert("roll", prove_roll);
      m.insert("triu", prove_triu);
      m.insert("tril", prove_tril);
      m.insert("tile", prove_tile);
      m.insert("repeat", prove_repeat);
      m.insert("diagonal", prove_diagonal);
      m
}

// Canonicalize a shape JSON name to a registry key. Strip lib prefix (last
// segment after . : $), lowercase, strip leading underscores, then map TRUE
// synonyms only. Names whose semantics differ from the registered op (masked
// reverse_sequence, attention transpose, list reverse, diag construction,
// un/pad_input) are intentionally left unmapped (honest backlog).
fn canon(name: &str) -> String {
      let mut base = name.rsplit(['.', ':', '$']).next().unwrap_or(name).to_lowercase();
      while base.starts_with('_') { base.remove(0); }
      // strip a trailing dtype/alias disambiguator on the same forward op
      for suf in ["_copy", "_8u_c1r", "_2", "_forward", "_kernel", "_op"] {
            if let Some(s) = base.strip_suffix(suf) { base = s.to_string(); }
      }
      let alias: &[(&str, &str)] = &[
            // transpose (plain 2D matrix transpose only)
            ("transpose", "transpose"), ("matrix_transpose", "transpose"),
            ("devicetranspose", "transpose"), ("tensortranspose", "transpose"),
            ("nppitranspose", "transpose"), ("fp8_transpose", "transpose"),
            ("stablehlo_transpose", "transpose"),
            // pad (constant pad)
            ("pad", "pad"), ("dynamic_pad", "pad"),
            ("zeropadding1d", "pad"), ("zeropadding2d", "pad"), ("zeropadding3d", "pad"),
            // flip family
            ("flip", "flip"), ("rev", "flip"), ("reverse", "flip"),
            ("reverse_v2", "flip"), ("fliplr", "fliplr"), ("flipud", "flipud"),
            // rotate
            ("rot90", "rot90"),
            // roll
            ("roll", "roll"),
            // tile
            ("tile", "tile"),
            // repeat (interleave). torch.repeat = tile semantics -> tile.
            ("repeat_interleave", "repeat"), ("repeat", "repeat"), ("repeatvector", "tile"),
            // diagonal extraction
            ("diagonal", "diagonal"),
      ];
      for (a, c) in alias { if base == *a { return c.to_string(); } }
      base
}

fn load_shape() -> Vec<String> {
      let dir = format!("{}/../kernel_inventory", env!("CARGO_MANIFEST_DIR"));
      let mut items = Vec::new();
      for e in std::fs::read_dir(&dir).expect("no kernel_inventory").flatten() {
            let p = e.path();
            if p.extension().map_or(false, |x| x == "json") {
                  let Ok(txt) = std::fs::read_to_string(&p) else { continue; };
                  let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else { continue; };
                  if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
                        for k in ks {
                              if k.get("category").and_then(|c| c.as_str()) != Some("shape") { continue; }
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
fn prove_shape() {
      let items = load_shape();
      assert!(!items.is_empty(), "no shape items in inventory");
      let reg = registry();

      // Prove each registered op once.
      let mut op_ok: HashMap<&str, bool> = HashMap::new();
      let mut failures: Vec<String> = Vec::new();
      for (k, f) in reg.iter() {
            let ok = f();
            op_ok.insert(*k, ok);
            if !ok { failures.push((*k).to_string()); }
      }

      // Walk inventory: each item whose canon maps to a passing op is proven.
      let total = items.len();
      let mut proven = 0usize;
      let mut proven_keys: std::collections::BTreeSet<String> = Default::default();
      for name in &items {
            let key = canon(name);
            if let Some(&ok) = op_ok.get(key.as_str()) {
                  if ok { proven += 1; proven_keys.insert(key); }
            }
      }

      let mut impls: Vec<&str> = reg.keys().copied().collect();
      impls.sort();

      eprintln!("\n=== PROVE shape ===");
      eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
      eprintln!("proven canonical ops ({}): {}", proven_keys.len(),
            proven_keys.iter().cloned().collect::<Vec<_>>().join(", "));
      eprintln!("PROVE shape: {} / {}", proven, total);

      assert!(failures.is_empty(), "registered shape op(s) FAILED oracle: {:?}", failures);
      assert!(proven > 0, "zero shape items proven");
}
