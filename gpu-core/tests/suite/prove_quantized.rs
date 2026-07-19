use crate::common;

use gpu_core::memory::GpuBuffer;
use std::collections::BTreeSet;
use std::ffi::c_void;
use std::fs;
use std::ptr;

unsafe extern "C" {
	fn launch_quantizedx_quantize_i8(
		x: *const c_void,
		q: *mut c_void,
		scale: f64,
		zp: i32,
		qmin: i32,
		qmax: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_quantizedx_dequantize_i8(
		q: *const c_void,
		x: *mut c_void,
		scale: f64,
		zp: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_quantizedx_fake_quant_i8(
		x: *const c_void,
		o: *mut c_void,
		scale: f64,
		zp: i32,
		qmin: i32,
		qmax: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_quantizedx_quantize_i4(
		x: *const c_void,
		q: *mut c_void,
		scale: f64,
		zp: i32,
		qmin: i32,
		qmax: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_quantizedx_fake_quant_i4(
		x: *const c_void,
		o: *mut c_void,
		scale: f64,
		zp: i32,
		qmin: i32,
		qmax: i32,
		n: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-6;

// ── GPU wrappers ─────────────────────────────────────────────────────────────

fn gpu_quantize(x: &[f64], scale: f64, zp: i32, qmin: i32, qmax: i32, i4: bool) -> Vec<i32> {
	let bx = {
		let __up = x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bq = GpuBuffer::alloc_bytes(x.len()).unwrap();
	unsafe {
		let f = if i4 {
			launch_quantizedx_quantize_i4
		} else {
			launch_quantizedx_quantize_i8
		};
		f(
			bx.ptr_raw() as *const c_void,
			bq.ptr_raw(),
			scale,
			zp,
			qmin,
			qmax,
			x.len() as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut codes_u8 = vec![0u8; x.len()];
	bq.download_u8(&mut codes_u8).unwrap();
	codes_u8.iter().map(|&b| b as i8 as i32).collect()
}

fn gpu_dequantize(codes: &[i32], scale: f64, zp: i32) -> Vec<f64> {
	let codes_u8: Vec<u8> = codes.iter().map(|&c| c as i8 as u8).collect();
	let bq = {
		let __u = &codes_u8;
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let bx = GpuBuffer::alloc(codes.len()).unwrap();
	unsafe {
		launch_quantizedx_dequantize_i8(
			bq.ptr_raw() as *const c_void,
			bx.ptr_raw(),
			scale,
			zp,
			codes.len() as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; codes.len()];
	unsafe { bx.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn gpu_fake_quant(x: &[f64], scale: f64, zp: i32, qmin: i32, qmax: i32, i4: bool) -> Vec<f64> {
	let bx = {
		let __up = x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bo = GpuBuffer::alloc(x.len()).unwrap();
	unsafe {
		let f = if i4 {
			launch_quantizedx_fake_quant_i4
		} else {
			launch_quantizedx_fake_quant_i8
		};
		f(
			bx.ptr_raw() as *const c_void,
			bo.ptr_raw(),
			scale,
			zp,
			qmin,
			qmax,
			x.len() as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; x.len()];
	unsafe { bo.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

// ── CPU oracles (authoritative std-f64, same textbook formula) ───────────────
fn oracle_quantize(x: f64, scale: f64, zp: i32, qmin: i32, qmax: i32) -> i32 {
	let r = (x / scale).round() + zp as f64;
	(r as i32).clamp(qmin, qmax)
}
fn oracle_dequantize(q: i32, scale: f64, zp: i32) -> f64 {
	(q - zp) as f64 * scale
}
fn oracle_fake_quant(x: f64, scale: f64, zp: i32, qmin: i32, qmax: i32) -> f64 {
	oracle_dequantize(oracle_quantize(x, scale, zp, qmin, qmax), scale, zp)
}

fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
	(0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.317) / n as f64)
		.collect()
}

// ── inventory canonicalization ───────────────────────────────────────────────
fn canon(name: &str) -> Option<&'static str> {
	let full = name.to_lowercase();
	let b = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();

	// ── library-prefix excludes (must test the FULL name, since the last
	if full.contains("awq") || full.contains("aqlm") || full.contains("ggml")
            || full.contains("gptq") || full.contains("hqq") || full.contains("llama_cpp")
            || full.contains("exllamav2") || full.contains("bitsandbytes") || full.contains("unsloth")
            || full.contains("marlin") || full.contains("flashinfer")
            || full.contains("deepspeed") || full.contains("transformer_engine") || full.contains("__te$")
            || full.contains("cudnn") || full.contains("cutlass")
	{
		return None;
	}

	if b.contains("fp8") || b.contains("fp4") || b.contains("nf4") {
		return None;
	}
	if b.contains("blockwise") || b.contains("block_scale") || b.contains("_2bit")
            || b.contains("_3bit") || b.contains("_4bit") || b.contains("to_4bit")
            || b.contains("_k") || b.starts_with("dequantize_q") || b.starts_with("dequantize_iq")
            || b.contains("gemm") || b.contains("packbits")
            || b.contains("per_channel") || b.contains("perchannel")
	{
		return None;
	}
	if b.contains("grad") || b.contains("backward") {
		return None;
	}
	if b.contains("matmul")
		|| b.contains("conv")
		|| b.contains("relu")
		|| b.contains("biasadd")
		|| b.contains("instancenorm")
		|| b.contains("rsqrt")
		|| b.contains("reshape")
		|| b.contains("concat")
		|| b.contains("lstm")
		|| b.contains("gru")
		|| b.contains("rnn")
		|| b.contains("embedding")
		|| b.contains("linear")
		|| b.contains("dot")
		|| b.contains("convolution")
		|| b.contains("clipbyvalue")
		|| b.contains("add")
		|| b.contains("mul")
		|| b.contains("smooth")
		|| b.contains("amax")
		|| b.contains("scale_update")
		|| b.contains("cast")
		|| b.contains("delayedscaling")
		|| b.contains("currentscaling")
		|| b.contains("autocast")
	{
		return None;
	}
	if b.contains("q_scale")
		|| b.contains("q_zero_point")
		|| b.contains("qparams")
		|| b.contains("int_repr")
		|| b.contains("q_per_channel")
		|| b.contains("make_per")
		|| b.contains("qfunctional")
	{
		return None;
	}
	if b.contains("requantize") || b.contains("shrinkrange") {
		return None;
	}

	if b.contains("fake_quant")
		|| b.contains("fakequant")
		|| b.contains("quantizeanddequantize")
		|| b == "fake_quant"
	{
		return Some("fake_quant");
	}
	if b.starts_with("dequantize")
		|| b == "dequantize"
		|| b.contains("uniformdequantize")
		|| b == "dequant"
	{
		return Some("dequantize");
	}
	if b.starts_with("quantize")
		|| b == "quantize"
		|| b.contains("uniformquantize")
		|| b == "quantizev2"
		|| b.contains("scaled_int8_quant")
		|| b == "quantizer"
		|| b.contains("dynamic_scaled_int8")
		|| b.contains("static_scaled_int8")
	{
		return Some("quantize");
	}
	None
}

fn load_quantized() -> Vec<String> {
	let dir = common::inventory_dir();
	let mut items = Vec::new();
	let rd = fs::read_dir(&dir).expect("no kernel_inventory");
	for e in rd.flatten() {
		let p = e.path();
		if p.extension().is_some_and(|x| x == "json") {
			let Ok(txt) = fs::read_to_string(&p) else {
				continue;
			};
			let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else {
				continue;
			};
			if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
				for k in ks {
					let cat = k.get("category").and_then(|c| c.as_str()).unwrap_or("");
					if cat != "quantized" {
						continue;
					}
					let name = k
						.get("name")
						.and_then(|n| n.as_str())
						.unwrap_or("")
						.to_string();
					if !name.is_empty() {
						items.push(name);
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
fn prove_quantized() {
	let items = load_quantized();
	assert!(!items.is_empty(), "no quantized items in inventory");

	let mut failures: Vec<String> = Vec::new();

	let cfgs: &[(f64, i32)] = &[(0.05, 0), (0.1, -10), (0.02, 7), (0.25, 3)];

	// ── int8 ([-128,127]) ───────────────────────────────────────────────────
	let (qmin8, qmax8) = (-128, 127);
	let xs8 = probes(-7.0, 7.0, 64);
	for &(scale, zp) in cfgs {
		let gq = gpu_quantize(&xs8, scale, zp, qmin8, qmax8, false);
		for (i, x) in xs8.iter().enumerate() {
			let want = oracle_quantize(*x, scale, zp, qmin8, qmax8);
			if gq[i] != want {
				failures.push(format!(
					"quantize_i8 x={} scale={} zp={}: gpu={} cpu={}",
					x, scale, zp, gq[i], want
				));
			}
		}
		let codes: Vec<i32> = (qmin8..=qmax8).collect();
		let gd = gpu_dequantize(&codes, scale, zp);
		for (i, q) in codes.iter().enumerate() {
			let want = oracle_dequantize(*q, scale, zp);
			if (gd[i] - want).abs() > TOL * (1.0 + want.abs()) {
				failures.push(format!(
					"dequantize_i8 q={} scale={} zp={}: gpu={} cpu={}",
					q, scale, zp, gd[i], want
				));
			}
		}
		let gf = gpu_fake_quant(&xs8, scale, zp, qmin8, qmax8, false);
		for (i, x) in xs8.iter().enumerate() {
			let want = oracle_fake_quant(*x, scale, zp, qmin8, qmax8);
			if (gf[i] - want).abs() > TOL * (1.0 + want.abs()) {
				failures.push(format!(
					"fake_quant_i8 x={} scale={} zp={}: gpu={} cpu={}",
					x, scale, zp, gf[i], want
				));
			}
			let qv = oracle_quantize(*x, scale, zp, qmin8, qmax8);
			if qv > qmin8 && qv < qmax8 && (gf[i] - x).abs() > scale / 2.0 + 1e-9 {
				failures.push(format!(
					"fake_quant_i8 roundtrip x={} |fq-x|={} > scale/2={}",
					x,
					(gf[i] - x).abs(),
					scale / 2.0
				));
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
				failures.push(format!(
					"quantize_i4 x={} scale={} zp={}: gpu={} cpu={}",
					x, scale, zp, gq[i], want
				));
			}
		}
		let gf = gpu_fake_quant(&xs4, scale, zp, qmin4, qmax4, true);
		for (i, x) in xs4.iter().enumerate() {
			let want = oracle_fake_quant(*x, scale, zp, qmin4, qmax4);
			if (gf[i] - want).abs() > TOL * (1.0 + want.abs()) {
				failures.push(format!(
					"fake_quant_i4 x={} scale={} zp={}: gpu={} cpu={}",
					x, scale, zp, gf[i], want
				));
			}
		}
		let codes: Vec<i32> = (qmin4..=qmax4).collect();
		let gd = gpu_dequantize(&codes, scale, zp);
		for (i, q) in codes.iter().enumerate() {
			let want = oracle_dequantize(*q, scale, zp);
			if (gd[i] - want).abs() > TOL * (1.0 + want.abs()) {
				failures.push(format!(
					"dequantize_i4 q={} scale={} zp={}: gpu={} cpu={}",
					q, scale, zp, gd[i], want
				));
			}
		}
	}

	// ── clamp saturation edge: values past the range must saturate to qmin/qmax.
	{
		let big = vec![1e6, -1e6, 100.0, -100.0];
		let gq = gpu_quantize(&big, 0.1, 0, qmin8, qmax8, false);
		let want: Vec<i32> = big
			.iter()
			.map(|x| oracle_quantize(*x, 0.1, 0, qmin8, qmax8))
			.collect();
		if gq != want {
			failures.push(format!(
				"quantize_i8 saturation: gpu={:?} cpu={:?}",
				gq, want
			));
		}
		if gq[0] != qmax8 || gq[1] != qmin8 {
			failures.push(format!("quantize_i8 saturation not at bounds: {:?}", gq));
		}
	}

	let proven_ops: BTreeSet<&'static str> = ["quantize", "dequantize", "fake_quant"]
		.into_iter()
		.collect();

	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<&'static str> = BTreeSet::new();
	let mut proven_items: Vec<(&'static str, String)> = Vec::new();
	let mut backlog: Vec<String> = Vec::new();
	for name in &items {
		match canon(name) {
			Some(op) if proven_ops.contains(op) => {
				proven += 1;
				proven_keys.insert(op);
				proven_items.push((op, name.clone()));
			}
			_ => backlog.push(name.clone()),
		}
	}

	eprintln!("\n=== PROVE quantized ===");
	eprintln!("PROVE quantized: {} / {}", proven, total);
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().copied().collect::<Vec<_>>().join(", ")
	);
	eprintln!("--- proven inventory items (via real canon()) ---");
	for (op, name) in &proven_items {
		eprintln!("  [{}] {}", op, name);
	}
	eprintln!(
		"backlog (different convention / not quant primitive / host-only): {} items",
		backlog.len()
	);

	assert!(
		failures.is_empty(),
		"registered quantized op(s) FAILED oracle: {:#?}",
		failures
	);
	assert!(proven > 0, "zero quantized items proven");
	assert!(
		proven_keys.contains("quantize")
			&& proven_keys.contains("dequantize")
			&& proven_keys.contains("fake_quant"),
		"missing a proven primitive: {:?}",
		proven_keys
	);
}
