// Live-GPU proof harness for the "quantized" inventory category.
//
// Affine per-tensor quantization primitives proven on the gfx1101 GPU:
//   quantize_per_tensor : q = clamp(round(x/scale)+zp, qmin, qmax)   (int exact)
//   dequantize          : x = (q - zp) * scale                       (1e-6)
//   fake_quant          : dequant(quant(x))                          (1e-6)
// Each op is run on-device and checked against an authoritative std-f64 oracle
// running the SAME textbook formula. quantize is asserted INTEGER-EXACT; dequant
// and fake_quant at tol 1e-6. The round-trip bound |fake_quant(x)-x| <= scale/2
// is asserted as the convention sanity check. int8 ([-128,127]) and int4 ([-8,7])
// are both exercised; i8 storage rides through u8 buffers via two's-complement
// bit reinterpretation (no upload_i8 in the API).
//
// A proven op counts ALL its inventory variants (collapsed by canon). Float/
// codebook formats (FP8/FP4/NF4), block/group formats (gptq/hqq/llama_cpp q*_K/
// awq/ggml), quantized COMPUTE (QuantizedMatMul/Conv/lstm/embedding_bag), and
// host-only param accessors (q_scale/q_zero_point/choose_qparams/int_repr) use a
// different convention or are not the quant primitive — they stay backlog, never
// counted, never faked green.

use gpu_core::memory::GpuBuffer;
use std::collections::BTreeSet;
use std::ffi::c_void;

unsafe extern "C" {
      fn launch_quantizedx_quantize_i8(x: *const c_void, q: *mut c_void, scale: f64, zp: i32, qmin: i32, qmax: i32, n: i32, s: *mut c_void);
      fn launch_quantizedx_dequantize_i8(q: *const c_void, x: *mut c_void, scale: f64, zp: i32, n: i32, s: *mut c_void);
      fn launch_quantizedx_fake_quant_i8(x: *const c_void, o: *mut c_void, scale: f64, zp: i32, qmin: i32, qmax: i32, n: i32, s: *mut c_void);
      fn launch_quantizedx_quantize_i4(x: *const c_void, q: *mut c_void, scale: f64, zp: i32, qmin: i32, qmax: i32, n: i32, s: *mut c_void);
      fn launch_quantizedx_fake_quant_i4(x: *const c_void, o: *mut c_void, scale: f64, zp: i32, qmin: i32, qmax: i32, n: i32, s: *mut c_void);
}

const TOL: f64 = 1e-6;

// ── GPU wrappers ─────────────────────────────────────────────────────────────

// quantize: f64 x -> i8 codes (carried as u8 via two's complement reinterpret).
fn gpu_quantize(x: &[f64], scale: f64, zp: i32, qmin: i32, qmax: i32, i4: bool) -> Vec<i32> {
      let bx = GpuBuffer::upload(x).unwrap();
      let bq = GpuBuffer::alloc_bytes(x.len()).unwrap();
      unsafe {
            let f = if i4 { launch_quantizedx_quantize_i4 } else { launch_quantizedx_quantize_i8 };
            f(bx.ptr_raw() as *const c_void, bq.ptr_raw(), scale, zp, qmin, qmax, x.len() as i32, std::ptr::null_mut());
      }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut codes_u8 = vec![0u8; x.len()];
      bq.download_u8(&mut codes_u8).unwrap();
      codes_u8.iter().map(|&b| b as i8 as i32).collect()
}

// dequantize: i8 codes -> f64.
fn gpu_dequantize(codes: &[i32], scale: f64, zp: i32) -> Vec<f64> {
      let codes_u8: Vec<u8> = codes.iter().map(|&c| c as i8 as u8).collect();
      let bq = GpuBuffer::upload_u8(&codes_u8).unwrap();
      let bx = GpuBuffer::alloc(codes.len()).unwrap();
      unsafe {
            launch_quantizedx_dequantize_i8(bq.ptr_raw() as *const c_void, bx.ptr_raw(), scale, zp, codes.len() as i32, std::ptr::null_mut());
      }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut out = vec![0.0; codes.len()];
      bx.download(&mut out).unwrap();
      out
}

// fake_quant: f64 x -> f64 (dequant of quant), stays on f64 buffer.
fn gpu_fake_quant(x: &[f64], scale: f64, zp: i32, qmin: i32, qmax: i32, i4: bool) -> Vec<f64> {
      let bx = GpuBuffer::upload(x).unwrap();
      let bo = GpuBuffer::alloc(x.len()).unwrap();
      unsafe {
            let f = if i4 { launch_quantizedx_fake_quant_i4 } else { launch_quantizedx_fake_quant_i8 };
            f(bx.ptr_raw() as *const c_void, bo.ptr_raw(), scale, zp, qmin, qmax, x.len() as i32, std::ptr::null_mut());
      }
      gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
      let mut out = vec![0.0; x.len()];
      bo.download(&mut out).unwrap();
      out
}

// ── CPU oracles (authoritative std-f64, same textbook formula) ───────────────
fn oracle_quantize(x: f64, scale: f64, zp: i32, qmin: i32, qmax: i32) -> i32 {
      let r = (x / scale).round() + zp as f64;
      (r as i32).clamp(qmin, qmax)
}
fn oracle_dequantize(q: i32, scale: f64, zp: i32) -> f64 { (q - zp) as f64 * scale }
fn oracle_fake_quant(x: f64, scale: f64, zp: i32, qmin: i32, qmax: i32) -> f64 {
      oracle_dequantize(oracle_quantize(x, scale, zp, qmin, qmax), scale, zp)
}

// Probes deliberately avoid x/scale landing on *.5 so the integer-exact assert
// is not hostage to rounding-mode ties.
fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
      (0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.317) / n as f64).collect()
}

// ── inventory canonicalization ───────────────────────────────────────────────
// Map a quantized-category JSON name to one of {quantize, dequantize, fake_quant}
// when (and ONLY when) our affine int per-tensor kernels genuinely implement it.
// Everything else returns None and stays backlog.
fn canon(name: &str) -> Option<&'static str> {
      let full = name.to_lowercase();
      let b = name.rsplit(['.', ':', '$']).next().unwrap_or(name).to_lowercase();

      // ── library-prefix excludes (must test the FULL name, since the last
      //    segment can collide with a legitimate op, e.g. exllamav2.dequantize
      //    has segment "dequantize" identical to torch.dequantize) ──
      // block/group/codebook-format libraries — per-block scales, not affine per-tensor:
      if full.contains("awq") || full.contains("aqlm") || full.contains("ggml")
            || full.contains("gptq") || full.contains("hqq") || full.contains("llama_cpp")
            || full.contains("exllamav2") || full.contains("bitsandbytes") || full.contains("unsloth")
            || full.contains("marlin") || full.contains("flashinfer")
            // deepspeed/te groupwise + fp quantizers:
            || full.contains("deepspeed") || full.contains("transformer_engine") || full.contains("__te$")
            || full.contains("cudnn") || full.contains("cutlass") { return None; }

      // Exclusions on the last segment — different convention or not the quant primitive.
      // float / codebook formats (FP8/FP4/NF4):
      if b.contains("fp8") || b.contains("fp4") || b.contains("nf4") { return None; }
      // block/group-scale formats:
      if b.contains("blockwise") || b.contains("block_scale") || b.contains("_2bit")
            || b.contains("_3bit") || b.contains("_4bit") || b.contains("to_4bit")
            || b.contains("_k") || b.starts_with("dequantize_q") || b.starts_with("dequantize_iq")
            || b.contains("gemm") || b.contains("packbits")
            // per-channel needs a scale VECTOR through the kernel; per-tensor doesn't prove it:
            || b.contains("per_channel") || b.contains("perchannel") { return None; }
      // gradients / STE backward passes are a different function from the forward op:
      if b.contains("grad") || b.contains("backward") { return None; }
      // quantized compute (matmul/conv/relu/lstm/embedding/etc.) — has extra qualifiers:
      if b.contains("matmul") || b.contains("conv") || b.contains("relu") || b.contains("biasadd")
            || b.contains("instancenorm") || b.contains("rsqrt") || b.contains("reshape")
            || b.contains("concat") || b.contains("lstm") || b.contains("gru") || b.contains("rnn")
            || b.contains("embedding") || b.contains("linear") || b.contains("dot")
            || b.contains("convolution") || b.contains("clipbyvalue") || b.contains("add")
            || b.contains("mul") || b.contains("smooth") || b.contains("amax")
            || b.contains("scale_update") || b.contains("cast") || b.contains("delayedscaling")
            || b.contains("currentscaling") || b.contains("autocast") { return None; }
      // host-only param accessors / qparam choosers / repr:
      if b.contains("q_scale") || b.contains("q_zero_point") || b.contains("qparams")
            || b.contains("int_repr") || b.contains("q_per_channel") || b.contains("make_per")
            || b.contains("qfunctional") { return None; }
      // requantize (down-and-shrink / uniform requant) — combined rescale, not primitive:
      if b.contains("requantize") || b.contains("shrinkrange") { return None; }

      // fake_quant family (clamp+round then dequant) — check before quantize/dequant:
      if b.contains("fake_quant") || b.contains("fakequant")
            || b.contains("quantizeanddequantize") || b == "fake_quant" {
            return Some("fake_quant");
      }
      // dequantize family:
      if b.starts_with("dequantize") || b == "dequantize" || b.contains("uniformdequantize")
            || b == "dequant" {
            return Some("dequantize");
      }
      // quantize family (per-tensor affine):
      if b.starts_with("quantize") || b == "quantize" || b.contains("uniformquantize")
            || b == "quantizev2" || b.contains("scaled_int8_quant") || b == "quantizer"
            || b.contains("dynamic_scaled_int8") || b.contains("static_scaled_int8") {
            return Some("quantize");
      }
      None
}

fn load_quantized() -> Vec<String> {
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
                              if cat != "quantized" { continue; }
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
fn prove_quantized() {
      let items = load_quantized();
      assert!(!items.is_empty(), "no quantized items in inventory");

      let mut failures: Vec<String> = Vec::new();

      // Parameter sets exercised for each op. Mix of positive/negative zero_point,
      // non-unit scale, asymmetric ranges.
      let cfgs: &[(f64, i32)] = &[(0.05, 0), (0.1, -10), (0.02, 7), (0.25, 3)];

      // ── int8 ([-128,127]) ───────────────────────────────────────────────────
      let (qmin8, qmax8) = (-128, 127);
      let xs8 = probes(-7.0, 7.0, 64);
      for &(scale, zp) in cfgs {
            // quantize: integer-EXACT vs oracle.
            let gq = gpu_quantize(&xs8, scale, zp, qmin8, qmax8, false);
            for (i, x) in xs8.iter().enumerate() {
                  let want = oracle_quantize(*x, scale, zp, qmin8, qmax8);
                  if gq[i] != want {
                        failures.push(format!("quantize_i8 x={} scale={} zp={}: gpu={} cpu={}", x, scale, zp, gq[i], want));
                  }
            }
            // dequantize: 1e-6 vs oracle over full code range.
            let codes: Vec<i32> = (qmin8..=qmax8).collect();
            let gd = gpu_dequantize(&codes, scale, zp);
            for (i, q) in codes.iter().enumerate() {
                  let want = oracle_dequantize(*q, scale, zp);
                  if (gd[i] - want).abs() > TOL * (1.0 + want.abs()) {
                        failures.push(format!("dequantize_i8 q={} scale={} zp={}: gpu={} cpu={}", q, scale, zp, gd[i], want));
                  }
            }
            // fake_quant: 1e-6 vs oracle + round-trip <= scale/2 sanity.
            let gf = gpu_fake_quant(&xs8, scale, zp, qmin8, qmax8, false);
            for (i, x) in xs8.iter().enumerate() {
                  let want = oracle_fake_quant(*x, scale, zp, qmin8, qmax8);
                  if (gf[i] - want).abs() > TOL * (1.0 + want.abs()) {
                        failures.push(format!("fake_quant_i8 x={} scale={} zp={}: gpu={} cpu={}", x, scale, zp, gf[i], want));
                  }
                  // round-trip bound only where x is inside the representable range.
                  let qv = oracle_quantize(*x, scale, zp, qmin8, qmax8);
                  if qv > qmin8 && qv < qmax8 && (gf[i] - x).abs() > scale / 2.0 + 1e-9 {
                        failures.push(format!("fake_quant_i8 roundtrip x={} |fq-x|={} > scale/2={}", x, (gf[i] - x).abs(), scale / 2.0));
                  }
            }
      }

      // ── int4 ([-8,7]) ────────────────────────────────────────────────────────
      let (qmin4, qmax4) = (-8, 7);
      let xs4 = probes(-2.0, 2.0, 48);
      for &(scale, zp) in &[(0.25, 0), (0.5, 1), (0.2, -2)] {
            let gq = gpu_quantize(&xs4, scale, zp, qmin4, qmax4, true);
            for (i, x) in xs4.iter().enumerate() {
                  let want = oracle_quantize(*x, scale, zp, qmin4, qmax4);
                  if gq[i] != want {
                        failures.push(format!("quantize_i4 x={} scale={} zp={}: gpu={} cpu={}", x, scale, zp, gq[i], want));
                  }
            }
            let gf = gpu_fake_quant(&xs4, scale, zp, qmin4, qmax4, true);
            for (i, x) in xs4.iter().enumerate() {
                  let want = oracle_fake_quant(*x, scale, zp, qmin4, qmax4);
                  if (gf[i] - want).abs() > TOL * (1.0 + want.abs()) {
                        failures.push(format!("fake_quant_i4 x={} scale={} zp={}: gpu={} cpu={}", x, scale, zp, gf[i], want));
                  }
            }
            // int4 dequant reuses the same i8 dequant kernel (codes fit in i8).
            let codes: Vec<i32> = (qmin4..=qmax4).collect();
            let gd = gpu_dequantize(&codes, scale, zp);
            for (i, q) in codes.iter().enumerate() {
                  let want = oracle_dequantize(*q, scale, zp);
                  if (gd[i] - want).abs() > TOL * (1.0 + want.abs()) {
                        failures.push(format!("dequantize_i4 q={} scale={} zp={}: gpu={} cpu={}", q, scale, zp, gd[i], want));
                  }
            }
      }

      // ── clamp saturation edge: values past the range must saturate to qmin/qmax.
      {
            let big = vec![1e6, -1e6, 100.0, -100.0];
            let gq = gpu_quantize(&big, 0.1, 0, qmin8, qmax8, false);
            let want: Vec<i32> = big.iter().map(|x| oracle_quantize(*x, 0.1, 0, qmin8, qmax8)).collect();
            if gq != want {
                  failures.push(format!("quantize_i8 saturation: gpu={:?} cpu={:?}", gq, want));
            }
            if gq[0] != qmax8 || gq[1] != qmin8 {
                  failures.push(format!("quantize_i8 saturation not at bounds: {:?}", gq));
            }
      }

      // Which canonical ops did we actually prove on-device (all green)?
      let proven_ops: BTreeSet<&'static str> = ["quantize", "dequantize", "fake_quant"].into_iter().collect();

      // Walk the inventory: count every item whose canon maps to a proven op.
      let total = items.len();
      let mut proven = 0usize;
      let mut proven_keys: BTreeSet<&'static str> = BTreeSet::new();
      let mut proven_items: Vec<(&'static str, String)> = Vec::new();
      let mut backlog: Vec<String> = Vec::new();
      for name in &items {
            match canon(name) {
                  Some(op) if proven_ops.contains(op) => { proven += 1; proven_keys.insert(op); proven_items.push((op, name.clone())); }
                  _ => backlog.push(name.clone()),
            }
      }

      eprintln!("\n=== PROVE quantized ===");
      eprintln!("PROVE quantized: {} / {}", proven, total);
      eprintln!("proven canonical ops ({}): {}", proven_keys.len(), proven_keys.iter().copied().collect::<Vec<_>>().join(", "));
      eprintln!("--- proven inventory items (via real canon()) ---");
      for (op, name) in &proven_items { eprintln!("  [{}] {}", op, name); }
      eprintln!("backlog (different convention / not quant primitive / host-only): {} items", backlog.len());

      assert!(failures.is_empty(), "registered quantized op(s) FAILED oracle: {:#?}", failures);
      assert!(proven > 0, "zero quantized items proven");
      // All three primitives must be genuinely proven on-device.
      assert!(proven_keys.contains("quantize") && proven_keys.contains("dequantize") && proven_keys.contains("fake_quant"),
            "missing a proven primitive: {:?}", proven_keys);
}
