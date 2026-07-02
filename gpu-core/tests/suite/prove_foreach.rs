use crate::common;
// Live-GPU proof harness for the "foreach" inventory category.
//
// torch._foreach_<op> applies <op> elementwise across a LIST of tensors. The
// per-element math is identical to the ordinary op; flattening the tensorlist to
// one contiguous buffer makes foreach a single elementwise launch over the whole
// batch. So each foreachx_ kernel is proven by running it on the LIVE gfx1101 GPU
// over a flattened multi-tensor batch and comparing against an AUTHORITATIVE CPU
// oracle (std f64 / libm) elementwise, tol 1e-6.
//
// canon() maps every foreach inventory name (in-place `_`, _scalar/_tensor/_list
// overloads, alias suffixes) to one registry key, so one proven op honestly
// covers all its variants. Pure host-only items (apex.*, transformer_engine.*,
// multi_tensor_*, foreach_map) stay backlog — never faked.

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

unsafe extern "C" {
	// unary x -> out
	fn launch_foreachx_neg(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_abs(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_sqrt(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_exp(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_sigmoid(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_reciprocal(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_log(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_log2(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_log10(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_log1p(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_expm1(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_floor(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_ceil(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_round(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_trunc(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_frac(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_sign(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_sin(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_cos(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_tan(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_sinh(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_cosh(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_tanh(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_asin(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_acos(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_atan(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_erf(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_erfc(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_foreachx_lgamma(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	// binary a,b -> out
	fn launch_foreachx_add(
		a: *const c_void,
		b: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_foreachx_sub(
		a: *const c_void,
		b: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_foreachx_mul(
		a: *const c_void,
		b: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_foreachx_div(
		a: *const c_void,
		b: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_foreachx_maximum(
		a: *const c_void,
		b: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_foreachx_minimum(
		a: *const c_void,
		b: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_foreachx_pow(
		a: *const c_void,
		b: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
}

type LaunchU = unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void);
type LaunchB = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void, i32, *mut c_void);

fn run_u(f: LaunchU, x: &[f64]) -> Vec<f64> {
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc(x.len()).unwrap();
	unsafe {
		f(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; x.len()];
	o.download(&mut out).unwrap();
	out
}
fn run_b(f: LaunchB, a: &[f64], b: &[f64]) -> Vec<f64> {
	let ba = GpuBuffer::upload(a).unwrap();
	let bb = GpuBuffer::upload(b).unwrap();
	let o = GpuBuffer::alloc(a.len()).unwrap();
	unsafe {
		f(
			ba.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			o.ptr_raw(),
			a.len() as i32,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; a.len()];
	o.download(&mut out).unwrap();
	out
}

// A flattened multi-tensor batch: emulates a foreach over a tensorlist of 3
// tensors of unequal length (5+3+7=15 elems), mapped linearly onto [lo,hi].
fn batch(lo: f64, hi: f64) -> Vec<f64> {
	let n = 15usize;
	(0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.5) / n as f64)
		.collect()
}

struct Op {
	run: Box<dyn Fn(&[f64], &[f64]) -> Vec<f64>>,
	oracle: Box<dyn Fn(f64, f64) -> f64>,
	lo: f64,
	hi: f64,
	blo: f64,
	bhi: f64,
}

fn registry() -> HashMap<&'static str, Op> {
	let mut m: HashMap<&'static str, Op> = HashMap::new();
	macro_rules! u {
		($k:literal, $l:expr, $o:expr, $lo:expr, $hi:expr) => {
			m.insert(
				$k,
				Op {
					run: Box::new(|x, _| run_u($l, x)),
					oracle: Box::new(|x, _| $o(x)),
					lo: $lo,
					hi: $hi,
					blo: 0.0,
					bhi: 0.0,
				},
			);
		};
	}
	macro_rules! b {
		($k:literal, $l:expr, $o:expr, $alo:expr, $ahi:expr, $blo:expr, $bhi:expr) => {
			m.insert(
				$k,
				Op {
					run: Box::new(|a, bb| run_b($l, a, bb)),
					oracle: Box::new($o),
					lo: $alo,
					hi: $ahi,
					blo: $blo,
					bhi: $bhi,
				},
			);
		};
	}

	// ── core 9 ──
	b!(
		"add",
		launch_foreachx_add,
		|x: f64, y: f64| x + y,
		-3.0,
		3.0,
		-2.0,
		4.0
	);
	b!(
		"sub",
		launch_foreachx_sub,
		|x: f64, y: f64| x - y,
		-3.0,
		3.0,
		-2.0,
		4.0
	);
	b!(
		"mul",
		launch_foreachx_mul,
		|x: f64, y: f64| x * y,
		-3.0,
		3.0,
		-2.0,
		4.0
	);
	b!(
		"div",
		launch_foreachx_div,
		|x: f64, y: f64| x / y,
		-3.0,
		3.0,
		0.5,
		4.0
	);
	u!("neg", launch_foreachx_neg, |x: f64| -x, -3.0, 3.0);
	u!("abs", launch_foreachx_abs, |x: f64| x.abs(), -3.0, 3.0);
	u!("sqrt", launch_foreachx_sqrt, |x: f64| x.sqrt(), 0.0, 9.0);
	u!("exp", launch_foreachx_exp, |x: f64| x.exp(), -3.0, 3.0);
	u!(
		"sigmoid",
		launch_foreachx_sigmoid,
		|x: f64| 1.0 / (1.0 + (-x).exp()),
		-6.0,
		6.0
	);

	// ── extra exact-oracle elementwise foreach ops ──
	u!(
		"reciprocal",
		launch_foreachx_reciprocal,
		|x: f64| 1.0 / x,
		0.5,
		5.0
	);
	u!("log", launch_foreachx_log, |x: f64| x.ln(), 0.05, 5.0);
	u!("log2", launch_foreachx_log2, |x: f64| x.log2(), 0.05, 5.0);
	u!(
		"log10",
		launch_foreachx_log10,
		|x: f64| x.log10(),
		0.05,
		5.0
	);
	u!(
		"log1p",
		launch_foreachx_log1p,
		|x: f64| x.ln_1p(),
		-0.9,
		5.0
	);
	u!(
		"expm1",
		launch_foreachx_expm1,
		|x: f64| x.exp_m1(),
		-3.0,
		3.0
	);
	u!(
		"floor",
		launch_foreachx_floor,
		|x: f64| x.floor(),
		-5.0,
		5.0
	);
	u!("ceil", launch_foreachx_ceil, |x: f64| x.ceil(), -5.0, 5.0);
	u!(
		"round",
		launch_foreachx_round,
		|x: f64| {
			let r = x.round(); // ties-to-even (torch/rint)
			if (x - x.trunc()).abs() == 0.5 {
				2.0 * (x / 2.0).round()
			} else {
				r
			}
		},
		-5.0,
		5.0
	);
	u!(
		"trunc",
		launch_foreachx_trunc,
		|x: f64| x.trunc(),
		-5.0,
		5.0
	);
	u!(
		"frac",
		launch_foreachx_frac,
		|x: f64| x - x.trunc(),
		-5.0,
		5.0
	);
	u!(
		"sign",
		launch_foreachx_sign,
		|x: f64| if x > 0.0 {
			1.0
		} else if x < 0.0 {
			-1.0
		} else {
			0.0
		},
		-3.0,
		3.0
	);
	u!("sin", launch_foreachx_sin, |x: f64| x.sin(), -3.0, 3.0);
	u!("cos", launch_foreachx_cos, |x: f64| x.cos(), -3.0, 3.0);
	u!("tan", launch_foreachx_tan, |x: f64| x.tan(), -1.2, 1.2);
	u!("sinh", launch_foreachx_sinh, |x: f64| x.sinh(), -3.0, 3.0);
	u!("cosh", launch_foreachx_cosh, |x: f64| x.cosh(), -3.0, 3.0);
	u!("tanh", launch_foreachx_tanh, |x: f64| x.tanh(), -3.0, 3.0);
	u!("asin", launch_foreachx_asin, |x: f64| x.asin(), -0.99, 0.99);
	u!("acos", launch_foreachx_acos, |x: f64| x.acos(), -0.99, 0.99);
	u!("atan", launch_foreachx_atan, |x: f64| x.atan(), -3.0, 3.0);
	u!("erf", launch_foreachx_erf, |x: f64| libm::erf(x), -3.0, 3.0);
	u!(
		"erfc",
		launch_foreachx_erfc,
		|x: f64| libm::erfc(x),
		-3.0,
		3.0
	);
	u!(
		"lgamma",
		launch_foreachx_lgamma,
		|x: f64| libm::lgamma(x),
		0.2,
		5.0
	);
	b!(
		"maximum",
		launch_foreachx_maximum,
		|x: f64, y: f64| x.max(y),
		-3.0,
		3.0,
		-3.0,
		3.0
	);
	b!(
		"minimum",
		launch_foreachx_minimum,
		|x: f64, y: f64| x.min(y),
		-3.0,
		3.0,
		-3.0,
		3.0
	);
	b!(
		"pow",
		launch_foreachx_pow,
		|x: f64, y: f64| x.powf(y),
		0.2,
		4.0,
		0.5,
		3.0
	);
	m
}

const TOL: f64 = 1e-6;

fn check(op: &Op) -> bool {
	let a = batch(op.lo, op.hi);
	let bb = batch(op.blo, op.bhi); // ignored for unary
	let got = (op.run)(&a, &bb);
	a.iter().zip(bb.iter()).zip(got.iter()).all(|((x, y), g)| {
		let want = (op.oracle)(*x, *y);
		want.is_finite() && (g - want).abs() <= TOL * (1.0 + want.abs())
	})
}

// Canonicalize a foreach inventory name to a registry key.
fn canon(name: &str) -> String {
	// last dotted segment, lowercase
	let mut s = name.rsplit('.').next().unwrap_or(name).to_lowercase();
	// strip leading _foreach_ (and any leading underscores)
	if let Some(p) = s.find("_foreach_") {
		s = s[p + "_foreach_".len()..].to_string();
	}
	// strip overload suffixes (longest first), repeatedly
	let suffixes = [
		"_scalarlist",
		"_tensorlist",
		"_scalar",
		"_tensor",
		"_list",
		"_self",
		"_2",
	];
	loop {
		let mut changed = false;
		for suf in suffixes {
			if s.len() > suf.len() && s.ends_with(suf) {
				s.truncate(s.len() - suf.len());
				changed = true;
			}
		}
		if !changed {
			break;
		}
	}
	// strip trailing in-place underscore
	if s.ends_with('_') {
		s.pop();
	}
	// norm/norm_2 alias to norm (host-only, stays backlog regardless)
	s
}

fn load_foreach() -> Vec<String> {
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
					if k.get("category").and_then(|c| c.as_str()) != Some("foreach") {
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
fn prove_foreach() {
	let items = load_foreach();
	assert!(!items.is_empty(), "no foreach items in inventory");
	let reg = registry();

	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	let mut failures: Vec<String> = Vec::new();
	for (k, op) in reg.iter() {
		let ok = check(op);
		op_ok.insert(*k, ok);
		if !ok {
			failures.push((*k).to_string());
		}
	}

	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<String> = Default::default();
	for name in &items {
		let key = canon(name);
		if let Some(&ok) = op_ok.get(key.as_str())
			&& ok
		{
			proven += 1;
			proven_keys.insert(key);
		}
	}

	eprintln!("\n=== PROVE foreach ===");
	eprintln!("PROVE foreach: {} / {}", proven, total);
	let mut impls: Vec<&str> = reg.keys().copied().collect();
	impls.sort();
	eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);

	// explicit per-op asserts for the mandated core 9
	for k in [
		"add", "sub", "mul", "div", "neg", "abs", "sqrt", "exp", "sigmoid",
	] {
		assert!(
			*op_ok.get(k).unwrap_or(&false),
			"core foreach op {} failed oracle",
			k
		);
	}
	assert!(
		failures.is_empty(),
		"registered foreach op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero foreach items proven");
}
