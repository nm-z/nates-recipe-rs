use crate::common;

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::c_void;
use std::fs;
use std::ptr;

unsafe extern "C" {
	fn launch_elementwise_binaryx_pow(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_hypot(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_fmax(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_fmin(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_copysign(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_nextafter(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_logaddexp(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_logaddexp2(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_remainder(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_floor_divide(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_heaviside(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_ldexp(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_xlogy(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_xlog1py(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_squared_difference(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_elementwise_binaryx_ge(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_le(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_elementwise_binaryx_ne(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
}

type Launch = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void, i32, *mut c_void);

type GpuBin = fn(&GpuBuffer, &GpuBuffer, usize, &GpuBuffer) -> Result<(), gpu_core::HipError>;

enum Op {
	Raw(Launch),
	Wrap(GpuBin),
}

struct BinOp {
	op: Op,
	oracle: fn(f64, f64) -> f64,
	alo: f64,
	ahi: f64,
	blo: f64,
	bhi: f64,
	tol: f64,
}

fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
	(0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.5) / n as f64)
		.collect()
}

fn run_raw(f: Launch, a: &[f64], b: &[f64]) -> Vec<f64> {
	let ba = {
		let __up = a;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(a.len()).unwrap();
	unsafe {
		f(
			ba.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			o.ptr_raw(),
			a.len() as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; a.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn run_wrap(f: GpuBin, a: &[f64], b: &[f64]) -> Vec<f64> {
	let ba = {
		let __up = a;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(a.len()).unwrap();
	f(&ba, &bb, a.len(), &o).unwrap();
	let mut out = vec![0.0; a.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn registry() -> HashMap<&'static str, BinOp> {
	use gpu_core::kernels::{gpu_add_into, gpu_div_into, gpu_eq, gpu_gt, gpu_lt, gpu_mul, gpu_sub};
	use gpu_core::math_ops::{gpu_atan2, gpu_fmod, gpu_max, gpu_min};
	let mut m: HashMap<&'static str, BinOp> = HashMap::new();
	macro_rules! w {
		($k:literal, $g:expr, $o:expr, $alo:expr,$ahi:expr,$blo:expr,$bhi:expr) => {
			m.insert(
				$k,
				BinOp {
					op: Op::Wrap($g),
					oracle: $o,
					alo: $alo,
					ahi: $ahi,
					blo: $blo,
					bhi: $bhi,
					tol: 1e-7,
				},
			);
		};
	}
	macro_rules! r {
		($k:literal, $g:expr, $o:expr, $alo:expr,$ahi:expr,$blo:expr,$bhi:expr) => {
			m.insert(
				$k,
				BinOp {
					op: Op::Raw($g),
					oracle: $o,
					alo: $alo,
					ahi: $ahi,
					blo: $blo,
					bhi: $bhi,
					tol: 1e-7,
				},
			);
		};
	}

	w!("add", gpu_add_into, |a, b| a + b, -3.0, 3.0, -2.3, 3.7);
	w!("sub", gpu_sub, |a, b| a - b, -3.0, 3.0, -2.3, 3.7);
	w!("mul", gpu_mul, |a, b| a * b, -3.0, 3.0, -2.3, 3.7);
	w!("div", gpu_div_into, |a, b| a / b, -3.0, 3.0, 0.5, 4.0);
	w!("atan2", gpu_atan2, |a, b| a.atan2(b), -3.0, 3.0, 0.5, 3.7);
	w!("fmod", gpu_fmod, |a, b| a % b, 1.0, 4.0, 1.7, 4.7);
	w!("maximum", gpu_max, |a, b| a.max(b), -3.0, 3.0, -2.3, 3.7);
	w!("minimum", gpu_min, |a, b| a.min(b), -3.0, 3.0, -2.3, 3.7);
	w!(
		"gt",
		gpu_gt,
		|a, b| if a > b { 1.0 } else { 0.0 },
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	w!(
		"lt",
		gpu_lt,
		|a, b| if a < b { 1.0 } else { 0.0 },
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	w!(
		"eq",
		gpu_eq,
		|a, b| if a == b { 1.0 } else { 0.0 },
		-3.0,
		3.0,
		-2.3,
		3.7
	);

	r!(
		"pow",
		launch_elementwise_binaryx_pow,
		|a: f64, b| a.powf(b),
		0.2,
		4.0,
		-2.0,
		3.0
	);
	r!(
		"hypot",
		launch_elementwise_binaryx_hypot,
		|a: f64, b| a.hypot(b),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"fmax",
		launch_elementwise_binaryx_fmax,
		|a: f64, b| a.max(b),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"fmin",
		launch_elementwise_binaryx_fmin,
		|a: f64, b| a.min(b),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"copysign",
		launch_elementwise_binaryx_copysign,
		|a: f64, b| a.copysign(b),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"nextafter",
		launch_elementwise_binaryx_nextafter,
		|a, b| libm::nextafter(a, b),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"logaddexp",
		launch_elementwise_binaryx_logaddexp,
		|a: f64, b: f64| (a.exp() + b.exp()).ln(),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"logaddexp2",
		launch_elementwise_binaryx_logaddexp2,
		|a: f64, b: f64| (a.exp2() + b.exp2()).log2(),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"remainder",
		launch_elementwise_binaryx_remainder,
		|a: f64, b: f64| a - (a / b).floor() * b,
		-3.0,
		3.0,
		0.7,
		3.7
	);
	r!(
		"floor_divide",
		launch_elementwise_binaryx_floor_divide,
		|a: f64, b: f64| (a / b).floor(),
		-3.0,
		3.0,
		0.7,
		3.7
	);
	r!(
		"heaviside",
		launch_elementwise_binaryx_heaviside,
		|a: f64, b| if a < 0.0 {
			0.0
		} else if a > 0.0 {
			1.0
		} else {
			b
		},
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"ldexp",
		launch_elementwise_binaryx_ldexp,
		|a: f64, b: f64| a * b.exp2(),
		-3.0,
		3.0,
		-2.0,
		3.0
	);
	r!(
		"xlogy",
		launch_elementwise_binaryx_xlogy,
		|a: f64, b: f64| if a == 0.0 { 0.0 } else { a * b.ln() },
		-3.0,
		3.0,
		0.2,
		4.0
	);
	r!(
		"xlog1py",
		launch_elementwise_binaryx_xlog1py,
		|a: f64, b: f64| if a == 0.0 { 0.0 } else { a * b.ln_1p() },
		-3.0,
		3.0,
		0.2,
		4.0
	);
	r!(
		"squared_difference",
		launch_elementwise_binaryx_squared_difference,
		|a: f64, b| (a - b) * (a - b),
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"ge",
		launch_elementwise_binaryx_ge,
		|a, b| if a >= b { 1.0 } else { 0.0 },
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"le",
		launch_elementwise_binaryx_le,
		|a, b| if a <= b { 1.0 } else { 0.0 },
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	r!(
		"ne",
		launch_elementwise_binaryx_ne,
		|a, b| if a != b { 1.0 } else { 0.0 },
		-3.0,
		3.0,
		-2.3,
		3.7
	);
	m
}

fn canon(name: &str) -> String {
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	let alias: &[(&str, &str)] = &[
		("add", "add"),
		("add_", "add"),
		("addv2", "add"),
		("broadcast_add", "add"),
		("vector_add", "add"),
		("cudnnop_add", "add"),
		("cutensorop_add", "add"),
		("cudnnpointwise_add", "add"),
		("stablehlo_add", "add"),
		("cudnnaddtensor", "add"),
		("vxadd", "add"),
		("miopenaddnforward", "add"),
		("subtract", "sub"),
		("sub_", "sub"),
		("broadcast_subtract", "sub"),
		("vector_sub", "sub"),
		("cudnnpointwise_sub", "sub"),
		("stablehlo_subtract", "sub"),
		("vxsubtract", "sub"),
		("multiply", "mul"),
		("mul_", "mul"),
		("broadcast_multiply", "mul"),
		("vector_mul", "mul"),
		("cudnnop_mul", "mul"),
		("cutensorop_mul", "mul"),
		("cudnnpointwise_mul", "mul"),
		("stablehlo_multiply", "mul"),
		("vxmultiply", "mul"),
		("divide", "div"),
		("div_", "div"),
		("true_divide", "div"),
		("truediv", "div"),
		("true_divide_", "div"),
		("realdiv", "div"),
		("fdiv", "div"),
		("div_rn", "div"),
		("vector_div", "div"),
		("broadcast_divide", "div"),
		("cudnnpointwise_div", "div"),
		("stablehlo_divide", "div"),
		("arctan2", "atan2"),
		("broadcast_atan2", "atan2"),
		("truncatemod", "fmod"),
		("max", "maximum"),
		("broadcast_maximum", "maximum"),
		("vector_max", "maximum"),
		("cudnnop_max", "maximum"),
		("cutensorop_max", "maximum"),
		("cudnnpointwise_max", "maximum"),
		("stablehlo_maximum", "maximum"),
		("min", "minimum"),
		("broadcast_minimum", "minimum"),
		("vector_min", "minimum"),
		("cudnnop_min", "minimum"),
		("cutensorop_min", "minimum"),
		("cudnnpointwise_min", "minimum"),
		("stablehlo_minimum", "minimum"),
		("greater", "gt"),
		("cudnnpointwise_cmp_gt", "gt"),
		("less", "lt"),
		("cudnnpointwise_cmp_lt", "lt"),
		("equal", "eq"),
		("cudnnpointwise_cmp_eq", "eq"),
		("greater_equal", "ge"),
		("greaterequal", "ge"),
		("cudnnpointwise_cmp_ge", "ge"),
		("less_equal", "le"),
		("lessequal", "le"),
		("cudnnpointwise_cmp_le", "le"),
		("not_equal", "ne"),
		("notequal", "ne"),
		("cudnnpointwise_cmp_neq", "ne"),
		("power", "pow"),
		("broadcast_power", "pow"),
		("cudnnpointwise_pow", "pow"),
		("stablehlo_power", "pow"),
		("float_power", "pow"),
		("float_power_aten", "pow"),
		("rem", "remainder"),
		("mod", "remainder"),
		("floor_mod", "remainder"),
		("floormod", "remainder"),
		("broadcast_remainder", "remainder"),
		("stablehlo_remainder", "remainder"),
		("floordiv", "floor_divide"),
		("squareddifference", "squared_difference"),
		("next_after", "nextafter"),
		("broadcast_next_after", "nextafter"),
	];
	for (a, c) in alias {
		if base == *a {
			return c.to_string();
		}
	}
	base
}

fn load_inventory() -> Vec<(String, String)> {
	let dir = common::inventory_dir();
	let mut items = Vec::new();
	let Ok(rd) = fs::read_dir(&dir) else {
		panic!("no recipe.db at {dir}");
	};
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
					let name = k
						.get("name")
						.and_then(|n| n.as_str())
						.unwrap_or("")
						.to_string();
					let cat = k
						.get("category")
						.and_then(|c| c.as_str())
						.unwrap_or("?")
						.to_string();
					if cat == "elementwise_binary" && !name.is_empty() {
						items.push((name, cat));
					}
				}
			}
		}
	}
	items
}

#[test]
fn prove_elementwise_binary() {
	let items = load_inventory();
	assert!(
		!items.is_empty(),
		"no elementwise_binary items in inventory"
	);
	let reg = registry();

	let n = 32usize;
	let mut failures: Vec<String> = Vec::new();
	let mut proven_ops: Vec<&'static str> = Vec::new();
	for (k, op) in reg.iter() {
		let a = probes(op.alo, op.ahi, n);
		let b = probes(op.blo, op.bhi, n);
		let got = match op.op {
			Op::Raw(f) => run_raw(f, &a, &b),
			Op::Wrap(f) => run_wrap(f, &a, &b),
		};
		let ok = a.iter().zip(&b).zip(&got).all(|((x, y), g)| {
			let want = (op.oracle)(*x, *y);
			want.is_finite() && (g - want).abs() <= op.tol * (1.0 + want.abs())
		});
		if ok {
			proven_ops.push(k);
		} else {
			let mut msg = format!("op {k}");
			for ((x, y), g) in a.iter().zip(&b).zip(&got) {
				let want = (op.oracle)(*x, *y);
				if !(want.is_finite() && (g - want).abs() <= op.tol * (1.0 + want.abs())) {
					msg = format!("op {k}: a={x} b={y} gpu={g} oracle={want}");
					break;
				}
			}
			failures.push(msg);
		}
	}

	let proven_keys: HashSet<&str> = proven_ops.iter().cloned().collect();
	let total = items.len();
	let mut proven = 0usize;
	let mut covered_ops: BTreeSet<String> = BTreeSet::new();
	for (name, _) in &items {
		let key = canon(name);
		if reg.contains_key(key.as_str()) && proven_keys.contains(key.as_str()) {
			proven += 1;
			covered_ops.insert(key);
		}
	}

	eprintln!("\n=== elementwise_binary proof ===");
	eprintln!(
		"  registered ops: {} ({} passed oracle)",
		reg.len(),
		proven_ops.len()
	);
	eprintln!("  covered ops: {:?}", covered_ops);
	eprintln!("PROVE elementwise_binary: {proven} / {total}");

	assert!(
		failures.is_empty(),
		"{} registered op(s) FAILED oracle: {:?}",
		failures.len(),
		failures
	);
	assert!(proven > 0, "zero items proven — registry/canon broken");
}
