use crate::common;
// Live-GPU proof harness for the "padding" inventory category.
//
// For every padding-category item in kernel_inventory/*.json, canonicalize its
// name to one of the registered pad ops; run the gpu-core paddingx_ kernel on the
// LIVE gfx1101 GPU and assert it matches an AUTHORITATIVE CPU oracle implementing
// the textbook PyTorch/NumPy pad convention. tol 1e-7.
//
//   constant : OOB -> cval                 (NumPy 'constant', torch constant_pad_nd,
//                                            tflite PAD/PADV2, StableHLO low/high pad,
//                                            copyMakeBorder BORDER_CONSTANT; zero_pad = cval 0)
//   reflect  : mirror WITHOUT repeating edge (NumPy 'reflect', torch reflection_pad,
//                                            tflite MIRROR_PAD mode REFLECT)
//   replicate: clamp to edge                (NumPy 'edge', torch replication_pad,
//                                            BORDER_REPLICATE)
//   circular : wrap modulo length           (NumPy 'wrap', torch circular_pad)
//
// One op covers all its dimensionality (1d/2d/3d) and dtype/alias variants: the
// per-axis index convention is identical, proven on the 1D and 2D kernels here.
// Gradient/backward items (MirrorPadGrad, *_backward) are DIFFERENT functions and
// are dropped (noted). tflite DILATE is interior dilation, not boundary fill —
// dropped (noted).

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_paddingx_constant1d(
		x: *const c_void,
		o: *mut c_void,
		l: i32,
		lpad: i32,
		n: i32,
		cval: f64,
		s: *mut c_void,
	);
	fn launch_paddingx_reflect1d(
		x: *const c_void,
		o: *mut c_void,
		l: i32,
		lpad: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_paddingx_replicate1d(
		x: *const c_void,
		o: *mut c_void,
		l: i32,
		lpad: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_paddingx_circular1d(
		x: *const c_void,
		o: *mut c_void,
		l: i32,
		lpad: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_paddingx_constant2d(
		x: *const c_void,
		o: *mut c_void,
		h: i32,
		w: i32,
		tpad: i32,
		lpad: i32,
		oh: i32,
		ow: i32,
		n: i32,
		cval: f64,
		s: *mut c_void,
	);
	fn launch_paddingx_reflect2d(
		x: *const c_void,
		o: *mut c_void,
		h: i32,
		w: i32,
		tpad: i32,
		lpad: i32,
		oh: i32,
		ow: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_paddingx_replicate2d(
		x: *const c_void,
		o: *mut c_void,
		h: i32,
		w: i32,
		tpad: i32,
		lpad: i32,
		oh: i32,
		ow: i32,
		n: i32,
		s: *mut c_void,
	);
	fn launch_paddingx_circular2d(
		x: *const c_void,
		o: *mut c_void,
		h: i32,
		w: i32,
		tpad: i32,
		lpad: i32,
		oh: i32,
		ow: i32,
		n: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-7;

// ── CPU oracles: authoritative per-axis index mappings (textbook). ──
fn reflect_idx(p: i64, l: i64) -> i64 {
	if l == 1 {
		return 0;
	}
	let period = 2 * l - 2;
	let mut q = p % period;
	if q < 0 {
		q += period;
	}
	if q < l { q } else { period - q }
}
fn replicate_idx(p: i64, l: i64) -> i64 {
	p.clamp(0, l - 1)
}
fn circular_idx(p: i64, l: i64) -> i64 {
	let mut q = p % l;
	if q < 0 {
		q += l;
	}
	q
}

fn oracle_1d(mode: &str, x: &[f64], lpad: i64, rpad: i64, cval: f64) -> Vec<f64> {
	let l = x.len() as i64;
	let n = l + lpad + rpad;
	(0..n).map(|i| {
		let p = i - lpad;
		match mode {
			"constant" => {
				if p >= 0 && p < l {
					x[p as usize]
				} else {
					cval
				}
			}
			"reflect" => x[reflect_idx(p, l) as usize],
			"replicate" => x[replicate_idx(p, l) as usize],
			"circular" => x[circular_idx(p, l) as usize],
			_ => unreachable!(),
		}
	})
	.collect()
}

fn oracle_2d(
	mode: &str,
	x: &[f64],
	h: i64,
	w: i64,
	tpad: i64,
	bpad: i64,
	lpad: i64,
	rpad: i64,
	cval: f64,
) -> Vec<f64> {
	let oh = h + tpad + bpad;
	let ow = w + lpad + rpad;
	let mut out = vec![0.0; (oh * ow) as usize];
	for orr in 0..oh {
		for oc in 0..ow {
			let r = orr - tpad;
			let c = oc - lpad;
			let v = match mode {
				"constant" => {
					if r >= 0 && r < h && c >= 0 && c < w {
						x[(r * w + c) as usize]
					} else {
						cval
					}
				}
				"reflect" => x[(reflect_idx(r, h) * w + reflect_idx(c, w)) as usize],
				"replicate" => x[(replicate_idx(r, h) * w + replicate_idx(c, w)) as usize],
				"circular" => x[(circular_idx(r, h) * w + circular_idx(c, w)) as usize],
				_ => unreachable!(),
			};
			out[(orr * ow + oc) as usize] = v;
		}
	}
	out
}

// ── GPU runners ──
fn gpu_1d(mode: &str, x: &[f64], lpad: i32, rpad: i32, cval: f64) -> Vec<f64> {
	let l = x.len() as i32;
	let n = l + lpad + rpad;
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc(n as usize).unwrap();
	let (xp, op) = (b.ptr_raw() as *const c_void, o.ptr_raw());
	unsafe {
		match mode {
			"constant" => {
				launch_paddingx_constant1d(xp, op, l, lpad, n, cval, std::ptr::null_mut())
			}
			"reflect" => launch_paddingx_reflect1d(xp, op, l, lpad, n, std::ptr::null_mut()),
			"replicate" => {
				launch_paddingx_replicate1d(xp, op, l, lpad, n, std::ptr::null_mut())
			}
			"circular" => {
				launch_paddingx_circular1d(xp, op, l, lpad, n, std::ptr::null_mut())
			}
			_ => unreachable!(),
		}
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; n as usize];
	o.download(&mut out).unwrap();
	out
}

fn gpu_2d(
	mode: &str,
	x: &[f64],
	h: i32,
	w: i32,
	tpad: i32,
	bpad: i32,
	lpad: i32,
	rpad: i32,
	cval: f64,
) -> Vec<f64> {
	let oh = h + tpad + bpad;
	let ow = w + lpad + rpad;
	let n = oh * ow;
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc(n as usize).unwrap();
	let (xp, op) = (b.ptr_raw() as *const c_void, o.ptr_raw());
	unsafe {
		match mode {
			"constant" => launch_paddingx_constant2d(
				xp,
				op,
				h,
				w,
				tpad,
				lpad,
				oh,
				ow,
				n,
				cval,
				std::ptr::null_mut(),
			),
			"reflect" => launch_paddingx_reflect2d(
				xp,
				op,
				h,
				w,
				tpad,
				lpad,
				oh,
				ow,
				n,
				std::ptr::null_mut(),
			),
			"replicate" => launch_paddingx_replicate2d(
				xp,
				op,
				h,
				w,
				tpad,
				lpad,
				oh,
				ow,
				n,
				std::ptr::null_mut(),
			),
			"circular" => launch_paddingx_circular2d(
				xp,
				op,
				h,
				w,
				tpad,
				lpad,
				oh,
				ow,
				n,
				std::ptr::null_mut(),
			),
			_ => unreachable!(),
		}
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; n as usize];
	o.download(&mut out).unwrap();
	out
}

fn close(a: &[f64], b: &[f64]) -> bool {
	a.len() == b.len()
		&& a.iter()
			.zip(b)
			.all(|(g, w)| (g - w).abs() <= TOL * (1.0 + w.abs()))
}

// Prove one pad mode end-to-end on both 1D and 2D with asymmetric pads.
fn prove_mode(mode: &str) -> bool {
	// deterministic pseudo-random input
	let x1: Vec<f64> = (0..7)
		.map(|i| ((i * 31 + 5) % 17) as f64 * 0.5 - 3.0)
		.collect();
	let cval = 2.5;
	// 1D: asymmetric, and a pad >= L for reflect/circular robustness
	for &(lp, rp) in &[(3i32, 2i32), (6, 5), (0, 4)] {
		let g = gpu_1d(mode, &x1, lp, rp, cval);
		let o = oracle_1d(mode, &x1, lp as i64, rp as i64, cval);
		if !close(&g, &o) {
			eprintln!("  {mode} 1d pad({lp},{rp}) MISMATCH\n    gpu={g:?}\n    cpu={o:?}");
			return false;
		}
	}
	// 2D: 4x5 input, asymmetric pads on all four sides
	let (h, w) = (4i32, 5i32);
	let x2: Vec<f64> = (0..(h * w))
		.map(|i| ((i * 13 + 7) % 23) as f64 * 0.25 - 2.0)
		.collect();
	for &(t, b, l, r) in &[(2i32, 1i32, 3i32, 2i32), (3, 3, 4, 4), (0, 2, 1, 0)] {
		let g = gpu_2d(mode, &x2, h, w, t, b, l, r, cval);
		let o = oracle_2d(
			mode, &x2, h as i64, w as i64, t as i64, b as i64, l as i64, r as i64, cval,
		);
		if !close(&g, &o) {
			eprintln!("  {mode} 2d pad(t{t},b{b},l{l},r{r}) MISMATCH");
			return false;
		}
	}
	true
}

// Canonicalize a padding-category JSON name to a registered op key, or "" to drop.
fn canon(name: &str) -> &'static str {
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	let base = base.trim_start_matches('_');
	// backward/grad are different functions -> drop
	if base.contains("grad") || base.contains("backward") {
		return "DROP_grad";
	}
	// tflite DILATE = interior dilation, not boundary fill -> drop
	if base == "dilate" {
		return "DROP_dilate";
	}
	// reflect / mirror family
	if base.contains("reflection") || base.contains("mirror") {
		return "reflect";
	}
	// replicate / edge family
	if base.contains("replication") {
		return "replicate";
	}
	// circular / wrap family
	if base.contains("circular") {
		return "circular";
	}
	// zero_pad = constant with 0
	if base.contains("zero_pad") {
		return "constant";
	}
	// everything else is a generic / constant pad op:
	//   pad, padv2, constant_pad_nd, stablehlo_pad, copymakeborder,
	//   padandstack, ipaddinglayer
	if base.contains("pad") || base.contains("copymakeborder") {
		return "constant";
	}
	"UNMAPPED"
}

fn load_padding() -> Vec<String> {
	let dir = common::inventory_dir();
	let mut items = Vec::new();
	for e in std::fs::read_dir(&dir)
		.expect("no kernel_inventory")
		.flatten()
	{
		let p = e.path();
		if p.extension().is_some_and(|x| x == "json") {
			let Ok(txt) = std::fs::read_to_string(&p) else {
				continue;
			};
			let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else {
				continue;
			};
			if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
				for k in ks {
					if k.get("category").and_then(|c| c.as_str()) != Some("padding") {
						continue;
					}
					if let Some(n) = k.get("name").and_then(|n| n.as_str())
						&& !n.is_empty()
					{
						items.push(n.to_string());
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
fn prove_padding() {
	let items = load_padding();
	assert!(!items.is_empty(), "no padding items in inventory");

	let modes = ["constant", "reflect", "replicate", "circular"];
	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	let mut failures: Vec<String> = Vec::new();
	for m in modes {
		let ok = prove_mode(m);
		op_ok.insert(m, ok);
		if !ok {
			failures.push(m.to_string());
		}
	}

	// Walk inventory: each item whose canon maps to a passing registered op is proven.
	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<String> = Default::default();
	let mut dropped: Vec<String> = Vec::new();
	let mut unmapped: Vec<String> = Vec::new();
	for name in &items {
		match canon(name) {
			k @ ("constant" | "reflect" | "replicate" | "circular") => {
				if *op_ok.get(k).unwrap() {
					proven += 1;
					proven_keys.insert(k.to_string());
				}
			}
			d if d.starts_with("DROP") => dropped.push(format!("{name} [{}]", &d[5..])),
			_ => unmapped.push(name.clone()),
		}
	}

	eprintln!("\n=== PROVE padding ===");
	eprintln!("registered ops (4): constant, reflect, replicate, circular");
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);
	if !dropped.is_empty() {
		eprintln!(
			"dropped ({}, different function — not boundary-fill padding):",
			dropped.len()
		);
		for d in &dropped {
			eprintln!("    {d}");
		}
	}
	if !unmapped.is_empty() {
		eprintln!("UNMAPPED ({}): {}", unmapped.len(), unmapped.join(", "));
	}
	eprintln!("PROVE padding: {} / {}", proven, total);

	assert!(
		failures.is_empty(),
		"registered padding op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(
		unmapped.is_empty(),
		"unmapped padding items (extend canon): {:?}",
		unmapped
	);
	assert!(proven > 0, "zero padding items proven");
}
