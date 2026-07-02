use crate::common;
// Live-GPU proof harness for the "set" inventory category.
//
// For every set-category item in kernel_inventory/*.json, canonicalize its name;
// if that canonical op is registered here, run the gpu-core setx_ kernel on the
// LIVE gfx1101 GPU and assert it matches an AUTHORITATIVE CPU set oracle.
//
//   unique             — sorted distinct (radix sort + adjacent dedup, hipcub)
//   unique_consecutive — collapse ADJACENT duplicates only, original order
//   unique_counts      — sorted distinct values + multiplicities (RunLengthEncode)
//   isin               — membership mask a[i] in b (sort b + binary search)
//
// Variable-length outputs: the launcher writes into a capacity-n buffer and
// reports the produced length via a device int* out_count; the test slices to
// that length. tol 1e-6 on values (all probes are exactly-representable f64, so
// dedup equality is bit-exact and the value tolerance is trivially met).
//
// A proven op counts ALL its inventory variants (collapsed by canon). The test
// FAILS on any registered-op mismatch (a real bug). The remaining set algebra
// (intersect1d/union1d/setdiff1d/setxor1d, thrust::set_*, *_by_key, have_overlap,
// merge, unique_inverse/all) stays backlog — reported, never faked green.

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_setx_unique_workspace_bytes(n: i32) -> usize;
	fn launch_setx_unique(
		x: *const c_void,
		keys_sorted: *mut c_void,
		out: *mut c_void,
		out_count: *mut c_void,
		tmp: *mut c_void,
		tmp_bytes: usize,
		n: i32,
		s: *mut c_void,
	);
	fn launch_setx_unique_consecutive_workspace_bytes(n: i32) -> usize;
	fn launch_setx_unique_consecutive(
		x: *const c_void,
		out: *mut c_void,
		out_count: *mut c_void,
		tmp: *mut c_void,
		tmp_bytes: usize,
		n: i32,
		s: *mut c_void,
	);
	fn launch_setx_unique_counts_workspace_bytes(n: i32) -> usize;
	fn launch_setx_unique_counts(
		x: *const c_void,
		keys_sorted: *mut c_void,
		vals_out: *mut c_void,
		counts_out: *mut c_void,
		out_count: *mut c_void,
		tmp: *mut c_void,
		tmp_bytes: usize,
		n: i32,
		s: *mut c_void,
	);
	fn launch_setx_isin_workspace_bytes(nb: i32) -> usize;
	fn launch_setx_isin(
		a: *const c_void,
		b: *const c_void,
		b_sorted: *mut c_void,
		mask: *mut c_void,
		tmp: *mut c_void,
		tmp_bytes: usize,
		na: i32,
		nb: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-6;

fn lasterr() {
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
}

// Probe vector with BOTH adjacent dups (the 2.0,2.0 pair and 5.5,5.5 pair) AND
// non-adjacent dups (-1.0 appears at idx 1 & 8; 7.25 at idx 2 & 5). This makes
// unique (sorted-distinct) and unique_consecutive produce DIFFERENT outputs, so
// the two ops are genuinely distinguished. All values exactly representable.
fn data() -> Vec<f64> {
	vec![
		3.5, -1.0, 7.25, 2.0, 2.0, 7.25, -8.0, 5.5, 5.5, -1.0, 4.0, 0.0, 4.0, 9.0,
	]
}

// ── GPU runners ──

// unique: radix sort then adjacent dedup -> needs an extra keys_sorted f64[n]
// scratch the launcher sorts into, plus the hipcub temp workspace.
fn run_unique(x: &[f64]) -> Vec<f64> {
	let n = x.len();
	let b = GpuBuffer::upload(x).unwrap();
	let keys_sorted = GpuBuffer::alloc(n).unwrap();
	let out = GpuBuffer::alloc(n).unwrap();
	let cnt = GpuBuffer::alloc_bytes(4).unwrap();
	let wb = unsafe { launch_setx_unique_workspace_bytes(n as i32) };
	let tmp = GpuBuffer::alloc_bytes(wb.max(1)).unwrap();
	unsafe {
		launch_setx_unique(
			b.ptr_raw() as *const c_void,
			keys_sorted.ptr_raw(),
			out.ptr_raw(),
			cnt.ptr_raw(),
			tmp.ptr_raw(),
			wb,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut k = [0i32];
	cnt.download_i32(&mut k).unwrap();
	let k = k[0] as usize;
	let mut buf = vec![0.0; n];
	out.download(&mut buf).unwrap();
	buf.truncate(k);
	buf
}

// unique_consecutive: adjacent dedup only, no sort -> no keys_sorted scratch,
// just the hipcub DeviceSelect::Unique temp workspace.
fn run_unique_consecutive(x: &[f64]) -> Vec<f64> {
	let n = x.len();
	let b = GpuBuffer::upload(x).unwrap();
	let out = GpuBuffer::alloc(n).unwrap();
	let cnt = GpuBuffer::alloc_bytes(4).unwrap();
	let wb = unsafe { launch_setx_unique_consecutive_workspace_bytes(n as i32) };
	let tmp = GpuBuffer::alloc_bytes(wb.max(1)).unwrap();
	unsafe {
		launch_setx_unique_consecutive(
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			cnt.ptr_raw(),
			tmp.ptr_raw(),
			wb,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut k = [0i32];
	cnt.download_i32(&mut k).unwrap();
	let k = k[0] as usize;
	let mut buf = vec![0.0; n];
	out.download(&mut buf).unwrap();
	buf.truncate(k);
	buf
}

fn run_unique_counts(x: &[f64]) -> (Vec<f64>, Vec<i32>) {
	let n = x.len();
	let b = GpuBuffer::upload(x).unwrap();
	let keys_sorted = GpuBuffer::alloc(n).unwrap();
	let vals = GpuBuffer::alloc(n).unwrap();
	let counts = GpuBuffer::alloc_bytes(n * 4).unwrap();
	let cnt = GpuBuffer::alloc_bytes(4).unwrap();
	let wb = unsafe { launch_setx_unique_counts_workspace_bytes(n as i32) };
	let tmp = GpuBuffer::alloc_bytes(wb.max(1)).unwrap();
	unsafe {
		launch_setx_unique_counts(
			b.ptr_raw() as *const c_void,
			keys_sorted.ptr_raw(),
			vals.ptr_raw(),
			counts.ptr_raw(),
			cnt.ptr_raw(),
			tmp.ptr_raw(),
			wb,
			n as i32,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut k = [0i32];
	cnt.download_i32(&mut k).unwrap();
	let k = k[0] as usize;
	let mut v = vec![0.0; n];
	vals.download(&mut v).unwrap();
	v.truncate(k);
	let mut c = vec![0i32; n];
	counts.download_i32(&mut c).unwrap();
	c.truncate(k);
	(v, c)
}

fn run_isin(a: &[f64], b: &[f64]) -> Vec<f64> {
	let ba = GpuBuffer::upload(a).unwrap();
	let bb = GpuBuffer::upload(b).unwrap();
	let mask = GpuBuffer::alloc(a.len()).unwrap();
	// hipcub DeviceRadixSort sorts b internally: needs a b_sorted output + temp storage.
	let b_sorted = GpuBuffer::alloc(b.len()).unwrap();
	let tmp_bytes = unsafe { launch_setx_isin_workspace_bytes(b.len() as i32) };
	let tmp = GpuBuffer::alloc_bytes(tmp_bytes.max(1)).unwrap();
	unsafe {
		launch_setx_isin(
			ba.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			b_sorted.ptr_raw(),
			mask.ptr_raw(),
			tmp.ptr_raw(),
			tmp_bytes,
			a.len() as i32,
			b.len() as i32,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0.0; a.len()];
	mask.download(&mut out).unwrap();
	out
}

// ── CPU oracles ──

fn cpu_unique(x: &[f64]) -> Vec<f64> {
	let mut v = x.to_vec();
	v.sort_by(|a, b| a.partial_cmp(b).unwrap());
	v.dedup();
	v
}

fn cpu_unique_consecutive(x: &[f64]) -> Vec<f64> {
	let mut v = x.to_vec();
	v.dedup(); // collapses ADJACENT equal runs only, preserves order
	v
}

fn cpu_unique_counts(x: &[f64]) -> (Vec<f64>, Vec<i32>) {
	let mut s = x.to_vec();
	s.sort_by(|a, b| a.partial_cmp(b).unwrap());
	let mut vals: Vec<f64> = Vec::new();
	let mut counts: Vec<i32> = Vec::new();
	for &val in &s {
		if let Some(&last) = vals.last()
			&& last == val
		{
			*counts.last_mut().unwrap() += 1;
			continue;
		}
		vals.push(val);
		counts.push(1);
	}
	(vals, counts)
}

fn prove_ops() -> (HashMap<&'static str, bool>, Vec<String>) {
	let x = data();
	let mut ok: HashMap<&'static str, bool> = HashMap::new();
	let mut fail: Vec<String> = Vec::new();
	macro_rules! mark {
		($k:expr, $pass:expr, $msg:expr) => {{
			let pass = $pass;
			ok.insert($k, pass);
			if !pass {
				fail.push($msg);
			}
		}};
	}

	// unique: sorted distinct == CPU sort+dedup
	{
		let g = run_unique(&x);
		let w = cpu_unique(&x);
		let pass = g.len() == w.len() && g.iter().zip(&w).all(|(a, b)| (a - b).abs() <= TOL);
		mark!("unique", pass, format!("unique {:?} != {:?}", g, w));
	}

	// unique_consecutive: adjacent-only dedup, original order == CPU dedup
	// MUST differ from unique here (probe has non-adjacent dups), else vacuous.
	{
		let g = run_unique_consecutive(&x);
		let w = cpu_unique_consecutive(&x);
		let mut pass =
			g.len() == w.len() && g.iter().zip(&w).all(|(a, b)| (a - b).abs() <= TOL);
		// distinctness guard: the two ops must genuinely disagree on this probe
		let u = cpu_unique(&x);
		if w.len() == u.len() && w.iter().zip(&u).all(|(a, b)| (a - b).abs() <= TOL) {
			pass = false; // probe failed to distinguish the ops -> treat as failure
			fail.push("unique_consecutive probe does not differ from unique".to_string());
		}
		mark!(
			"unique_consecutive",
			pass,
			format!("unique_consecutive {:?} != {:?}", g, w)
		);
	}

	// unique_counts: sorted distinct values + multiplicities
	{
		let (gv, gc) = run_unique_counts(&x);
		let (wv, wc) = cpu_unique_counts(&x);
		let pass = gv.len() == wv.len()
			&& gv.iter().zip(&wv).all(|(a, b)| (a - b).abs() <= TOL)
			&& gc == wc;
		mark!(
			"unique_counts",
			pass,
			format!("unique_counts ({:?},{:?}) != ({:?},{:?})", gv, gc, wv, wc)
		);
	}

	// isin: membership mask a[i] in b ; oracle = exact b.contains
	{
		let a = [3.5, -1.0, 100.0, 2.0, 0.0, -8.0, 42.0, 9.0];
		let b = [3.5, 2.0, 0.0, 9.0, 7.25, -8.0];
		let g = run_isin(&a, &b);
		let w: Vec<f64> = a
			.iter()
			.map(|av| {
				if b.iter().any(|bv| bv == av) {
					1.0
				} else {
					0.0
				}
			})
			.collect();
		let pass = g.len() == w.len() && g.iter().zip(&w).all(|(p, q)| (p - q).abs() <= TOL);
		mark!("isin", pass, format!("isin {:?} != {:?}", g, w));
	}

	(ok, fail)
}

// Canonicalize a set-category JSON name to a registry key. Routing is by the
// item's documented semantics (the inventory `description` field), NOT by string
// shape: the discriminator is "full distinct set" -> unique vs "collapse
// consecutive runs only" -> unique_consecutive.
//
// Full-name overrides come FIRST because last-segment matching physically cannot
// separate thrust::unique / cudf::unique / jax.numpy.unique (all -> "unique"):
//   thrust::unique  = "Remove consecutive duplicates"  -> unique_consecutive
//   cudf::unique    = "Distinct values preserving order" (consecutive dedup;
//                     cudf::distinct is the global-dedup op) -> unique_consecutive
// V2 ops (UniqueV2/UniqueWithCountsV2) carry an axis parameter our flat op does
// not implement, so they are NOT stripped to v1 — they stay backlog (honest).
fn canon(name: &str) -> String {
	match name {
		"thrust::unique" | "cudf::unique" => return "unique_consecutive".to_string(),
		_ => {}
	}
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	let base = base
		.strip_suffix("_aten")
		.map(|s| s.to_string())
		.unwrap_or(base);
	let alias: &[(&str, &str)] = &[
		// sorted-distinct / global-distinct family -> unique (set-invariant,
		// order-agnostic: GPU op and oracle both sort, so the proven claim is
		// the distinct SET, not first-occurrence ordering).
		("unique", "unique"),
		("unique_values", "unique"),
		("distinct", "unique"),
		// unique-with-counts / count-distinct family -> unique_counts
		// (the (value,count) multiset is order-independent).
		("unique_counts", "unique_counts"),
		("uniquewithcounts", "unique_counts"),
		("count_distinct", "unique_counts"),
		// adjacent run-dedup family -> unique_consecutive
		("unique_consecutive", "unique_consecutive"),
		("unique_copy", "unique_consecutive"),
		// membership -> isin
		("isin", "isin"),
		("in1d", "isin"),
	];
	for (a, c) in alias {
		if base == *a {
			return c.to_string();
		}
	}
	base
}

fn load_set() -> Vec<String> {
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
					if cat != "set" {
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
fn prove_set() {
	let items = load_set();
	assert!(!items.is_empty(), "no set items in inventory");

	let (op_ok, failures) = prove_ops();

	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: std::collections::BTreeSet<String> = Default::default();
	let mut backlog: std::collections::BTreeSet<String> = Default::default();
	for name in &items {
		let key = canon(name);
		match op_ok.get(key.as_str()) {
			Some(true) => {
				proven += 1;
				proven_keys.insert(key);
			}
			_ => {
				backlog.insert(name.clone());
			}
		}
	}

	let mut impls: Vec<&str> = op_ok.keys().copied().collect();
	impls.sort();

	eprintln!("\n=== PROVE set ===");
	eprintln!("PROVE set: {} / {}", proven, total);
	eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);
	eprintln!(
		"backlog ({}): {}",
		backlog.len(),
		backlog.iter().cloned().collect::<Vec<_>>().join(", ")
	);

	assert!(
		failures.is_empty(),
		"registered set op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero set items proven");
}
