use crate::common;
// Live-GPU proof harness for the "special" inventory category.
//
// For every special-category item in kernel_inventory/*.json, canonicalize its
// name; if that canonical op is registered here, run the gpu-core op (existing
// k_mathx fn or a new specialx_ kernel) on the LIVE gfx1101 GPU and assert it
// matches an AUTHORITATIVE oracle (std f64 / libm / textbook definition from the
// JSON description / inverse round-trip / finite difference). tol 1e-7.
//
// A proven op counts ALL its inventory variants (collapsed by canon). The test
// FAILS on any registered-op mismatch (a real bug). Host-only / structural items
// (tfp.sts.*, accumulators, integrators, interpolators, ...) stay as backlog.

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

// New specialx_ kernels (unary: x->out ; xlogy: x,y->out).
unsafe extern "C" {
	fn launch_specialx_digamma(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_expit(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_logit(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_sinc(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_entr(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_erfinv(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_erfcx(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_i0(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_i1(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_i0e(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_i1e(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_j0(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_j1(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_y0(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_y1(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_ndtr(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_ndtri(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_sinpi(x: *const c_void, o: *mut c_void, n: i32, s: *mut c_void);
	fn launch_specialx_xlogy(
		x: *const c_void,
		y: *const c_void,
		o: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
}

type Launch = unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void);

fn run_specialx(f: Launch, x: &[f64]) -> Vec<f64> {
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

// Existing k_mathx unary ops carry signature (x, n) -> GpuBuffer.
type Km = fn(&GpuBuffer, usize) -> Result<GpuBuffer, gpu_core::hip::HipError>;
fn run_km(f: Km, x: &[f64]) -> Vec<f64> {
	let b = GpuBuffer::upload(x).unwrap();
	let o = f(&b, x.len()).unwrap();
	let mut out = vec![0.0; x.len()];
	o.download(&mut out).unwrap();
	out
}

struct UnaryOp {
	run: Box<dyn Fn(&[f64]) -> Vec<f64>>,
	oracle: Box<dyn Fn(f64) -> f64>,
	lo: f64,
	hi: f64,
}

fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
	(0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.5) / n as f64)
		.collect()
}

fn unary_registry() -> HashMap<&'static str, UnaryOp> {
	use gpu_core::k_mathx::{gpu_erf, gpu_erfc, gpu_lgamma, gpu_tgamma};
	let mut m: HashMap<&'static str, UnaryOp> = HashMap::new();
	macro_rules! sx {
		($k:literal, $launch:expr, $o:expr, $lo:expr, $hi:expr) => {
			m.insert(
				$k,
				UnaryOp {
					run: Box::new(|x| run_specialx($launch, x)),
					oracle: Box::new($o),
					lo: $lo,
					hi: $hi,
				},
			);
		};
	}
	macro_rules! km {
		($k:literal, $g:expr, $o:expr, $lo:expr, $hi:expr) => {
			m.insert(
				$k,
				UnaryOp {
					run: Box::new(|x| run_km($g, x)),
					oracle: Box::new($o),
					lo: $lo,
					hi: $hi,
				},
			);
		};
	}

	// ── existing k_mathx ops (oracle = libm) ──
	km!("erf", gpu_erf, libm::erf, -3.0, 3.0);
	km!("erfc", gpu_erfc, libm::erfc, -3.0, 3.0);
	km!("tgamma", gpu_tgamma, libm::tgamma, 0.2, 4.0);
	km!("lgamma", gpu_lgamma, libm::lgamma, 0.2, 5.0);

	// ── new specialx_ ops ──
	sx!(
		"expit",
		launch_specialx_expit,
		|x| 1.0 / (1.0 + (-x).exp()),
		-6.0,
		6.0
	);
	sx!(
		"logit",
		launch_specialx_logit,
		|x| (x / (1.0 - x)).ln(),
		0.02,
		0.98
	);
	sx!(
		"sinc",
		launch_specialx_sinc,
		|x: f64| if x == 0.0 {
			1.0
		} else {
			(std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
		},
		-3.0,
		3.0
	);
	sx!(
		"entr",
		launch_specialx_entr,
		|x: f64| if x > 0.0 {
			-x * x.ln()
		} else if x == 0.0 {
			0.0
		} else {
			f64::NEG_INFINITY
		},
		0.05,
		3.0
	);
	// erfinv via inverse round-trip: erf(erfinv(x)) == x
	sx!("erfinv", launch_specialx_erfinv, |x| x, -0.99, 0.99); // oracle handled specially below
	sx!(
		"erfcx",
		launch_specialx_erfcx,
		|x: f64| (x * x).exp() * libm::erfc(x),
		-3.0,
		3.0
	);
	// i0/i1 textbook power series
	sx!("i0", launch_specialx_i0, bessel_i0_series, -5.0, 5.0);
	sx!("i1", launch_specialx_i1, bessel_i1_series, -5.0, 5.0);
	sx!(
		"i0e",
		launch_specialx_i0e,
		|x: f64| (-x.abs()).exp() * bessel_i0_series(x),
		-5.0,
		5.0
	);
	sx!(
		"i1e",
		launch_specialx_i1e,
		|x: f64| (-x.abs()).exp() * bessel_i1_series(x),
		-5.0,
		5.0
	);
	sx!("j0", launch_specialx_j0, libm::j0, -8.0, 8.0);
	sx!("j1", launch_specialx_j1, libm::j1, -8.0, 8.0);
	sx!("y0", launch_specialx_y0, libm::y0, 0.5, 8.0);
	sx!("y1", launch_specialx_y1, libm::y1, 0.5, 8.0);
	// ndtr = standard normal CDF = 0.5*erfc(-x/sqrt2) ; ndtri = inverse (round-trip)
	sx!(
		"ndtr",
		launch_specialx_ndtr,
		|x| 0.5 * libm::erfc(-x / std::f64::consts::SQRT_2),
		-3.0,
		3.0
	);
	sx!("ndtri", launch_specialx_ndtri, |x| x, 0.02, 0.98); // round-trip handled specially below
	sx!(
		"sinpi",
		launch_specialx_sinpi,
		|x| (std::f64::consts::PI * x).sin(),
		-2.0,
		2.0
	);
	// digamma oracle = central finite-diff of lgamma (digamma == d/dx ln Gamma)
	sx!(
		"digamma",
		launch_specialx_digamma,
		|x| {
			let h = 1e-5;
			(libm::lgamma(x + h) - libm::lgamma(x - h)) / (2.0 * h)
		},
		1.0,
		6.0
	);
	m
}

// Modified Bessel I0/I1 power series (textbook). |x|<=5, ~25 terms => <1e-12.
fn bessel_i0_series(x: f64) -> f64 {
	let y = (x / 2.0).powi(2);
	let mut term = 1.0;
	let mut sum = 1.0;
	for k in 1..30 {
		term *= y / (k as f64 * k as f64);
		sum += term;
		if term < 1e-18 * sum {
			break;
		}
	}
	sum
}
fn bessel_i1_series(x: f64) -> f64 {
	let y = (x / 2.0).powi(2);
	let mut term = x / 2.0;
	let mut sum = term;
	for k in 1..30 {
		term *= y / (k as f64 * (k as f64 + 1.0));
		sum += term;
		if term.abs() < 1e-18 * sum.abs() {
			break;
		}
	}
	sum
}

// xlogy is binary: x*log(y), x==0 -> 0.
fn run_xlogy(x: &[f64], y: &[f64]) -> Vec<f64> {
	let bx = GpuBuffer::upload(x).unwrap();
	let by = GpuBuffer::upload(y).unwrap();
	let o = GpuBuffer::alloc(x.len()).unwrap();
	unsafe {
		launch_specialx_xlogy(
			bx.ptr_raw() as *const c_void,
			by.ptr_raw() as *const c_void,
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

// Canonicalize a special-category JSON name to a registry key. Mirrors the
// inventory_proof.rs convention (strip lib prefix, lowercase, last segment),
// then maps TRUE synonyms only — never *_backward / *_grad (different functions).
fn canon(name: &str) -> String {
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	// drop trailing "_2"/"_alias"/"_aten" disambiguators (same forward function)
	let base = base
		.strip_suffix("_2")
		.map(|s| s.to_string())
		.or_else(|| base.strip_suffix("_alias").map(|s| s.to_string()))
		.or_else(|| base.strip_suffix("_aten").map(|s| s.to_string()))
		.unwrap_or(base);
	let alias: &[(&str, &str)] = &[
		// sigmoid family
		("expit", "expit"),
		("sigmoid", "expit"),
		// digamma / psi
		("digamma", "digamma"),
		("psi", "digamma"),
		("hpsi", "digamma"),
		// gamma family
		("gamma", "tgamma"),
		("gammaln", "lgamma"),
		("lngamma", "lgamma"),
		// error functions
		("erf", "erf"),
		("erfc", "erfc"),
		("erfcx", "erfcx"),
		("erfinv", "erfinv"),
		// modified bessel I
		("i0", "i0"),
		("i1", "i1"),
		("i0e", "i0e"),
		("i1e", "i1e"),
		("modified_bessel_i0", "i0"),
		("modified_bessel_i1", "i1"),
		// bessel J / Y
		("bessel_j0", "j0"),
		("bessel_j1", "j1"),
		("bessel_y0", "y0"),
		("bessel_y1", "y1"),
		("j0", "j0"),
		("j1", "j1"),
		("y0", "y0"),
		("y1", "y1"),
		// normal CDF / inverse
		("ndtr", "ndtr"),
		("ndtri", "ndtri"),
		// others
		("logit", "logit"),
		("sinc", "sinc"),
		("entr", "entr"),
		("xlogy", "xlogy"),
		("sinpi", "sinpi"),
	];
	for (a, c) in alias {
		if base == *a {
			return c.to_string();
		}
	}
	base
}

fn load_special() -> Vec<String> {
	let dir = common::inventory_dir();
	let mut items = Vec::new();
	let rd = std::fs::read_dir(&dir).expect("no kernel_inventory");
	for e in rd.flatten() {
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
					let cat = k.get("category").and_then(|c| c.as_str()).unwrap_or("");
					if cat != "special" {
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

const TOL: f64 = 1e-7;

// Special-cased proofs that need a different assertion than direct oracle compare.
fn check_special_op(key: &str, reg: &UnaryOp) -> bool {
	match key {
		// erf(erfinv(x)) == x
		"erfinv" => {
			let xs = probes(reg.lo, reg.hi, 24);
			let got = (reg.run)(&xs);
			xs.iter()
				.zip(&got)
				.all(|(x, g)| (libm::erf(*g) - x).abs() <= TOL * (1.0 + x.abs()))
		}
		// ndtr(ndtri(x)) == x  (ndtr oracle proven independently)
		"ndtri" => {
			let xs = probes(reg.lo, reg.hi, 24);
			let got = (reg.run)(&xs);
			xs.iter().zip(&got).all(|(x, g)| {
				let rt = 0.5 * libm::erfc(-g / std::f64::consts::SQRT_2);
				(rt - x).abs() <= 1e-6 * (1.0 + x.abs())
			})
		}
		_ => {
			let xs = probes(reg.lo, reg.hi, 24);
			let got = (reg.run)(&xs);
			xs.iter().zip(&got).all(|(x, g)| {
				let want = (reg.oracle)(*x);
				want.is_finite() && (g - want).abs() <= TOL * (1.0 + want.abs())
			})
		}
	}
}

#[test]
fn prove_special() {
	let items = load_special();
	assert!(!items.is_empty(), "no special items in inventory");
	let reg = unary_registry();

	// Prove each registered op ONCE (and its defining-edge probes), cache pass/fail.
	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	let mut failures: Vec<String> = Vec::new();
	for (k, op) in reg.iter() {
		let ok = check_special_op(k, op);
		op_ok.insert(*k, ok);
		if !ok {
			failures.push((*k).to_string());
		}
	}

	// Defining-edge probes (the conventions are the whole point of these ops):
	// entr(0)=0, sinc(0)=1, xlogy(0,y)=0.
	{
		let e = run_specialx(launch_specialx_entr, &[0.0]);
		if e[0] != 0.0 {
			failures.push(format!("entr(0)={} != 0", e[0]));
		}
		let s = run_specialx(launch_specialx_sinc, &[0.0]);
		if (s[0] - 1.0).abs() > TOL {
			failures.push(format!("sinc(0)={} != 1", s[0]));
		}
		let xl = run_xlogy(&[0.0, 2.0, -1.0], &[5.0, 3.0, 4.0]);
		let want = [0.0, 2.0 * 3.0_f64.ln(), -4.0_f64.ln()];
		if xl.iter()
			.zip(&want)
			.any(|(g, w)| (g - w).abs() > TOL * (1.0 + w.abs()))
		{
			failures.push(format!("xlogy {:?} != {:?}", xl, want));
		}
		op_ok.insert("xlogy", !failures.iter().any(|f| f.starts_with("xlogy")));
	}

	// Walk the inventory: each item whose canon maps to a passing registered op is proven.
	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: std::collections::BTreeSet<String> = Default::default();
	for name in &items {
		let key = canon(name);
		if let Some(&ok) = op_ok.get(key.as_str())
			&& ok
		{
			proven += 1;
			proven_keys.insert(key);
		}
	}

	eprintln!("\n=== PROVE special ===");
	eprintln!("PROVE special: {} / {}", proven, total);
	let mut impls: Vec<&str> = reg.keys().copied().collect();
	impls.push("xlogy");
	impls.sort();
	eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);

	assert!(
		failures.is_empty(),
		"registered special op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero special items proven");
}
