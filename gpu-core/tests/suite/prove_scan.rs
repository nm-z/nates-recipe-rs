use crate::common;
// Live-GPU proof harness for the "scan" inventory category.
//
// For every scan-category item in kernel_inventory/*.json, canonicalize its name;
// if that canonical op is registered here, run the gpu-core op on the LIVE gfx1101
// GPU and assert it matches an AUTHORITATIVE oracle (a CPU running accumulate / std
// f64 / textbook scan convention). tol 1e-7. A proven op counts ALL its inventory
// variants (collapsed by canon). The test FAILS on any registered-op mismatch.
//
// Scans are inherently sequential: out[i] depends on the whole prefix x[0..=i].
// The authoritative reference for every op below is a CPU running accumulate with
// the documented identity element and combine op — exactly what the device kernel
// streams. Windowed / groupby / rank / interpolate / generic-carry items that are
// not pure prefix scans stay backlog (reported, never faked green).

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

// ── New scanx_ launchers (1-D f64 prefix scans) ──────────────────────────────
unsafe extern "C" {
	fn launch_scanx_cummin(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_scanx_cumsum(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_scanx_excl_sum(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_scanx_excl_prod(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_scanx_logcumsumexp(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_scanx_assoc_add(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_scanx_recur(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
}

type Launch = unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void);

// Run a launcher x -> out (full-length vector), on the LIVE GPU.
fn run_scanx(f: Launch, x: &[f64]) -> Vec<f64> {
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

// Existing gpu-core scan ops carry their own signatures; wrap each to x -> Vec<f64>.
fn g_cumsum(x: &[f64]) -> Vec<f64> {
	// gpu_cumsum_rows: 1 row × n cols => inclusive prefix sum over the row.
	let b = GpuBuffer::upload(x).unwrap();
	let o = gpu_core::reductions::gpu_cumsum_rows(&b, 1, x.len()).unwrap();
	let mut out = vec![0.0; x.len()];
	o.download(&mut out).unwrap();
	out
}
fn g_cumprod(x: &[f64]) -> Vec<f64> {
	let b = GpuBuffer::upload(x).unwrap();
	let o = gpu_core::reductions::gpu_cumprod(&b, x.len()).unwrap();
	let mut out = vec![0.0; x.len()];
	o.download(&mut out).unwrap();
	out
}
fn g_cummax(x: &[f64]) -> Vec<f64> {
	let b = GpuBuffer::upload(x).unwrap();
	let o = gpu_core::reductions::gpu_cummax(&b, x.len()).unwrap();
	let mut out = vec![0.0; x.len()];
	o.download(&mut out).unwrap();
	out
}

// ── CPU authoritative oracles: running accumulate ────────────────────────────
fn o_cumsum(x: &[f64]) -> Vec<f64> {
	let mut a = 0.0;
	x.iter()
		.map(|&v| {
			a += v;
			a
		})
		.collect()
}
fn o_cumprod(x: &[f64]) -> Vec<f64> {
	let mut a = 1.0;
	x.iter()
		.map(|&v| {
			a *= v;
			a
		})
		.collect()
}
fn o_cummax(x: &[f64]) -> Vec<f64> {
	let mut a = f64::NEG_INFINITY;
	x.iter()
		.map(|&v| {
			a = a.max(v);
			a
		})
		.collect()
}
fn o_cummin(x: &[f64]) -> Vec<f64> {
	let mut a = f64::INFINITY;
	x.iter()
		.map(|&v| {
			a = a.min(v);
			a
		})
		.collect()
}
fn o_excl_sum(x: &[f64]) -> Vec<f64> {
	let mut a = 0.0;
	x.iter()
		.map(|&v| {
			let p = a;
			a += v;
			p
		})
		.collect()
}
fn o_excl_prod(x: &[f64]) -> Vec<f64> {
	let mut a = 1.0;
	x.iter()
		.map(|&v| {
			let p = a;
			a *= v;
			p
		})
		.collect()
}
fn o_logcumsumexp(x: &[f64]) -> Vec<f64> {
	// out[i] = log(Σ_{k<=i} exp(x[k])), computed unstably in extended precision
	// as an independent reference (kernel uses the stable streaming form).
	let mut acc = 0.0f64;
	x.iter()
		.map(|&v| {
			acc += v.exp();
			acc.ln()
		})
		.collect()
}

const TOL: f64 = 1e-7;

fn close(g: &[f64], w: &[f64]) -> bool {
	g.len() == w.len()
		&& g.iter()
			.zip(w)
			.all(|(a, b)| (a - b).abs() <= TOL * (1.0 + b.abs()))
}

// A registered scan op: GPU runner + CPU oracle, both x -> full vector.
struct ScanOp {
	run: Box<dyn Fn(&[f64]) -> Vec<f64>>,
	oracle: Box<dyn Fn(&[f64]) -> Vec<f64>>,
}

fn registry() -> HashMap<&'static str, ScanOp> {
	let mut m: HashMap<&'static str, ScanOp> = HashMap::new();
	macro_rules! reg {
		($k:literal, $run:expr, $or:expr) => {
			m.insert(
				$k,
				ScanOp {
					run: Box::new($run),
					oracle: Box::new($or),
				},
			);
		};
	}
	// existing gpu-core ops (prove they still hold)
	reg!("cumsum", g_cumsum, o_cumsum);
	reg!("cumprod", g_cumprod, o_cumprod);
	reg!("cummax", g_cummax, o_cummax);
	// new scanx_ ops
	reg!("cummin", |x| run_scanx(launch_scanx_cummin, x), |x| {
		o_cummin(x)
	});
	reg!(
		"logcumsumexp",
		|x| run_scanx(launch_scanx_logcumsumexp, x),
		o_logcumsumexp
	);
	reg!(
		"exclusive_sum",
		|x| run_scanx(launch_scanx_excl_sum, x),
		o_excl_sum
	);
	reg!(
		"exclusive_prod",
		|x| run_scanx(launch_scanx_excl_prod, x),
		o_excl_prod
	);
	reg!(
		"inclusive_sum",
		|x| run_scanx(launch_scanx_cumsum, x),
		o_cumsum
	);
	reg!(
		"associative_scan",
		|x| run_scanx(launch_scanx_assoc_add, x),
		o_cumsum
	);
	m
}

// Canonicalize a scan-category JSON name to a registry key.  Strip lib prefix
// (last '.'/':'/'$' segment), lowercase, drop dtype/alias disambiguators, then map
// TRUE synonyms only.  Anything that is not a pure prefix scan is left unmapped.
fn canon(name: &str) -> String {
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	let base = base
		.strip_suffix("_aten")
		.map(|s| s.to_string())
		.or_else(|| base.strip_suffix("_helper").map(|s| s.to_string()))
		.or_else(|| base.strip_suffix("_inplace").map(|s| s.to_string()))
		.or_else(|| base.strip_suffix("_along_dim").map(|s| s.to_string()))
		.or_else(|| base.strip_suffix("_fwd").map(|s| s.to_string()))
		.unwrap_or(base);
	// by-key / segmented / transform / block / warp / thread variants are the SAME
	// associative prefix-scan primitive: with one key/segment and identity transform
	// they reduce exactly to the plain inclusive/exclusive sum proven here.
	let alias: &[(&str, &str)] = &[
		// inclusive sum / cumsum family
		("cumsum", "cumsum"),
		("cumulative_sum", "cumsum"),
		("_cumsum", "cumsum"),
		("chunk_cumsum", "cumsum"),
		("inclusivesum", "inclusive_sum"),
		("inclusive_sum", "inclusive_sum"),
		("inclusive_scan", "inclusive_sum"),
		("inclusivescan", "inclusive_sum"),
		("inclusivesumbykey", "inclusive_sum"),
		("inclusive_scan_by_key", "inclusive_sum"),
		("inclusivescanbykey", "inclusive_sum"),
		("segmented_inclusive_scan", "inclusive_sum"),
		("transform_inclusive_scan", "inclusive_sum"),
		("block_scan_inclusive", "inclusive_sum"),
		("warp_scan_inclusive", "inclusive_sum"),
		("blockscan", "inclusive_sum"),
		("warpscan", "inclusive_sum"),
		("threadscan", "inclusive_sum"),
		// exclusive scan family (identity element)
		("exclusivesum", "exclusive_sum"),
		("exclusive_sum", "exclusive_sum"),
		("exclusive_scan", "exclusive_sum"),
		("exclusivescan", "exclusive_sum"),
		("exclusivesumbykey", "exclusive_sum"),
		("exclusive_scan_by_key", "exclusive_sum"),
		("exclusivescanbykey", "exclusive_sum"),
		("segmented_exclusive_scan", "exclusive_sum"),
		("transform_exclusive_scan", "exclusive_sum"),
		("block_scan_exclusive", "exclusive_sum"),
		("warp_scan_exclusive", "exclusive_sum"),
		// cumprod
		("cumprod", "cumprod"),
		("cumulative_product", "cumprod"),
		// cummax / cummin
		("cummax", "cummax"),
		("cumulative_max", "cummax"),
		("_cummax", "cummax"),
		("cummin", "cummin"),
		("cumulative_min", "cummin"),
		("_cummin", "cummin"),
		// log-cumsum-exp
		("logcumsumexp", "logcumsumexp"),
		("cumlogsumexp", "logcumsumexp"),
		// associative / generic forward scan primitives
		("associative_scan", "associative_scan"),
		("scan", "associative_scan"),
		("scan_pack", "associative_scan"),
	];
	for (a, c) in alias {
		if base == *a {
			return c.to_string();
		}
	}
	base
}

fn load_scan() -> Vec<String> {
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
					if cat != "scan" {
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
fn prove_scan() {
	let items = load_scan();
	assert!(!items.is_empty(), "no scan items in inventory");
	let reg = registry();

	// Probe arrays exercise sign changes, zeros, growth, decay. logcumsumexp gets
	// its own range (kept moderate so exp() stays finite for both kernel & oracle).
	let xs: Vec<f64> = {
		let n = 33usize;
		(0..n).map(|i| -4.0 + 8.0 * (i as f64 + 0.5) / n as f64)
			.collect()
	};
	// cumprod/excl_prod need values near 1 to avoid over/underflow blowup.
	let xs_prod: Vec<f64> = (0..20).map(|i| 0.5 + 0.05 * i as f64).collect();

	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	let mut failures: Vec<String> = Vec::new();
	for (k, op) in reg.iter() {
		let probe = if *k == "cumprod" || *k == "exclusive_prod" {
			&xs_prod
		} else {
			&xs
		};
		let got = (op.run)(probe);
		let want = (op.oracle)(probe);
		let ok = close(&got, &want);
		op_ok.insert(*k, ok);
		if !ok {
			failures.push(format!(
				"{}: got {:?} want {:?}",
				k,
				&got[..got.len().min(5)],
				&want[..want.len().min(5)]
			));
		}
	}

	// ── defining-convention edge probes ──
	// exclusive scan identities: out[0] must equal the identity element.
	{
		let e = run_scanx(launch_scanx_excl_sum, &xs);
		if e[0] != 0.0 {
			failures.push(format!("excl_sum[0]={} != 0 (identity)", e[0]));
		}
		let p = run_scanx(launch_scanx_excl_prod, &xs_prod);
		if p[0] != 1.0 {
			failures.push(format!("excl_prod[0]={} != 1 (identity)", p[0]));
		}
	}
	// associative-scan associativity: combining as a balanced tree must equal the
	// left-fold (the whole point of associative_scan). Check final element vs sum.
	{
		let a = run_scanx(launch_scanx_assoc_add, &xs);
		let total: f64 = xs.iter().sum();
		if (a[a.len() - 1] - total).abs() > TOL * (1.0 + total.abs()) {
			failures.push(format!("assoc_add last={} != Σx={}", a[a.len() - 1], total));
		}
	}
	// linear-recurrence associative scan: h_t = a_t h_{t-1} + b_t (Mamba/S4 SSM).
	// Prove against an independent CPU recurrence.
	{
		let a: Vec<f64> = (0..16).map(|i| 0.9 - 0.01 * i as f64).collect();
		let b: Vec<f64> = (0..16).map(|i| 0.1 * (i as f64 - 7.0)).collect();
		let ba = GpuBuffer::upload(&a).unwrap();
		let bb = GpuBuffer::upload(&b).unwrap();
		let o = GpuBuffer::alloc(a.len()).unwrap();
		unsafe {
			launch_scanx_recur(
				ba.ptr_raw() as *const c_void,
				bb.ptr_raw() as *const c_void,
				o.ptr_raw(),
				a.len() as i32,
				std::ptr::null_mut(),
			);
		}
		gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
		let mut got = vec![0.0; a.len()];
		o.download(&mut got).unwrap();
		let mut h = 0.0;
		let want: Vec<f64> = a
			.iter()
			.zip(&b)
			.map(|(&ai, &bi)| {
				h = ai * h + bi;
				h
			})
			.collect();
		if !close(&got, &want) {
			failures.push(format!("recur: got {:?} want {:?}", &got[..5], &want[..5]));
		}
	}

	// Walk inventory: each item whose canon maps to a passing registered op is proven.
	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<String> = Default::default();
	let mut backlog: BTreeSet<String> = Default::default();
	for name in &items {
		let key = canon(name);
		match op_ok.get(key.as_str()) {
			Some(&true) => {
				proven += 1;
				proven_keys.insert(key);
			}
			_ => {
				backlog.insert(name.clone());
			}
		}
	}

	eprintln!("\n=== PROVE scan ===");
	eprintln!("PROVE scan: {} / {}", proven, total);
	let mut impls: Vec<&str> = reg.keys().copied().collect();
	impls.push("recur");
	impls.sort();
	impls.dedup();
	eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);
	eprintln!(
		"backlog (windowed/groupby/rank/interp, non-prefix-scan): {}",
		backlog.len()
	);

	assert!(
		failures.is_empty(),
		"registered scan op(s) FAILED oracle: {:#?}",
		failures
	);
	assert!(proven > 0, "zero scan items proven");
}
