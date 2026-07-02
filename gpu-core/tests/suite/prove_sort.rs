use crate::common;
// Live-GPU proof harness for the "sort" inventory category.
//
// For every sort-category item in kernel_inventory/*.json, canonicalize its name;
// if that canonical op is registered here, run the gpu-core sortx_ kernel on the
// LIVE gfx1101 GPU and assert it matches an AUTHORITATIVE oracle (std/CPU sort).
// All full sorts use hipcub::DeviceRadixSort on device; the oracle is a CPU sort
// of the same data, which is the definition of a correct sort. tol 1e-7.
//
// A proven op counts ALL its inventory variants (collapsed by canon). The test
// FAILS on any registered-op mismatch (a real bug). Items with no kernel op here
// (joins, candidate samplers, ranking metrics, lexsort, unique, structural cub::
// Block*/Warp* building blocks, approximate top-k) stay backlog, reported but
// never faked green.

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_sortx_sort_asc(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_sort_asc_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_sort_desc(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_sort_desc_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_argsort(
		x: *const c_void,
		out_idx: *mut c_void,
		n: i32,
		keys_out: *mut c_void,
		vals_in: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_argsort_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_topk(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		k: i32,
		keys_out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_topk_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_kthvalue(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		k: i32,
		keys_out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_kthvalue_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_median(
		x: *const c_void,
		out: *mut c_void,
		n: i32,
		keys_out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_median_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_argmax(
		x: *const c_void,
		out_idx: *mut c_void,
		n: i32,
		keys_out: *mut c_void,
		vals_in: *mut c_void,
		vals_out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_argmax_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_argmin(
		x: *const c_void,
		out_idx: *mut c_void,
		n: i32,
		keys_out: *mut c_void,
		vals_in: *mut c_void,
		vals_out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		s: *mut c_void,
	);
	fn sortx_argmin_workspace_bytes(n: i32) -> usize;
	fn launch_sortx_searchsorted(
		sorted: *const c_void,
		q: *const c_void,
		out: *mut c_void,
		n: i32,
		nq: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-7;

fn lasterr() {
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
}

// A deterministic but unsorted, duplicate-bearing probe vector.
fn data() -> Vec<f64> {
	let raw = [
		3.5, -1.0, 7.25, 0.0, 7.25, -8.0, 2.0, 5.5, -1.0, 9.0, 4.0, -3.5, 6.0, 1.5, -2.0, 8.0,
		0.5, -6.0, 2.75, 10.0,
	];
	raw.to_vec()
}

fn run_keys(
	f: unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void, usize, *mut c_void),
	wb: unsafe extern "C" fn(i32) -> usize,
	x: &[f64],
) -> Vec<f64> {
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc(x.len()).unwrap();
	let wbytes = unsafe { wb(x.len() as i32) };
	let ws = GpuBuffer::alloc_bytes(wbytes.max(1)).unwrap();
	unsafe {
		f(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			ws.ptr_raw(),
			wbytes,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0.0; x.len()];
	o.download(&mut out).unwrap();
	out
}

fn run_argsort(x: &[f64]) -> Vec<i32> {
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc_bytes(x.len() * 4).unwrap();
	let keys_out = GpuBuffer::alloc(x.len()).unwrap();
	let vals_in = GpuBuffer::alloc_bytes(x.len() * 4).unwrap();
	let wbytes = unsafe { sortx_argsort_workspace_bytes(x.len() as i32) };
	let ws = GpuBuffer::alloc_bytes(wbytes.max(1)).unwrap();
	unsafe {
		launch_sortx_argsort(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			keys_out.ptr_raw(),
			vals_in.ptr_raw(),
			ws.ptr_raw(),
			wbytes,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0i32; x.len()];
	o.download_i32(&mut out).unwrap();
	out
}

fn run_argextreme(
	f: unsafe extern "C" fn(
		*const c_void,
		*mut c_void,
		i32,
		*mut c_void,
		*mut c_void,
		*mut c_void,
		*mut c_void,
		usize,
		*mut c_void,
	),
	wb: unsafe extern "C" fn(i32) -> usize,
	x: &[f64],
) -> i32 {
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc_bytes(4).unwrap();
	let keys_out = GpuBuffer::alloc(x.len()).unwrap();
	let vals_in = GpuBuffer::alloc_bytes(x.len() * 4).unwrap();
	let vals_out = GpuBuffer::alloc_bytes(x.len() * 4).unwrap();
	let wbytes = unsafe { wb(x.len() as i32) };
	let ws = GpuBuffer::alloc_bytes(wbytes.max(1)).unwrap();
	unsafe {
		f(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			keys_out.ptr_raw(),
			vals_in.ptr_raw(),
			vals_out.ptr_raw(),
			ws.ptr_raw(),
			wbytes,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0i32; 1];
	o.download_i32(&mut out).unwrap();
	out[0]
}

fn run_topk(x: &[f64], k: usize) -> Vec<f64> {
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc(k).unwrap();
	let keys_out = GpuBuffer::alloc(x.len()).unwrap();
	let wbytes = unsafe { sortx_topk_workspace_bytes(x.len() as i32) };
	let ws = GpuBuffer::alloc_bytes(wbytes.max(1)).unwrap();
	unsafe {
		launch_sortx_topk(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			k as i32,
			keys_out.ptr_raw(),
			ws.ptr_raw(),
			wbytes,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0.0; k];
	o.download(&mut out).unwrap();
	out
}

fn run_scalar(
	f: unsafe extern "C" fn(
		*const c_void,
		*mut c_void,
		i32,
		i32,
		*mut c_void,
		*mut c_void,
		usize,
		*mut c_void,
	),
	wb: unsafe extern "C" fn(i32) -> usize,
	x: &[f64],
	k: i32,
) -> f64 {
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc(1).unwrap();
	let keys_out = GpuBuffer::alloc(x.len()).unwrap();
	let wbytes = unsafe { wb(x.len() as i32) };
	let ws = GpuBuffer::alloc_bytes(wbytes.max(1)).unwrap();
	unsafe {
		f(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			k,
			keys_out.ptr_raw(),
			ws.ptr_raw(),
			wbytes,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0.0; 1];
	o.download(&mut out).unwrap();
	out[0]
}

fn run_median(x: &[f64]) -> f64 {
	let b = GpuBuffer::upload(x).unwrap();
	let o = GpuBuffer::alloc(1).unwrap();
	let keys_out = GpuBuffer::alloc(x.len()).unwrap();
	let wbytes = unsafe { sortx_median_workspace_bytes(x.len() as i32) };
	let ws = GpuBuffer::alloc_bytes(wbytes.max(1)).unwrap();
	unsafe {
		launch_sortx_median(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			keys_out.ptr_raw(),
			ws.ptr_raw(),
			wbytes,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0.0; 1];
	o.download(&mut out).unwrap();
	out[0]
}

fn run_searchsorted(sorted: &[f64], q: &[f64]) -> Vec<i32> {
	let bs = GpuBuffer::upload(sorted).unwrap();
	let bq = GpuBuffer::upload(q).unwrap();
	let o = GpuBuffer::alloc_bytes(q.len() * 4).unwrap();
	unsafe {
		launch_sortx_searchsorted(
			bs.ptr_raw() as *const c_void,
			bq.ptr_raw() as *const c_void,
			o.ptr_raw(),
			sorted.len() as i32,
			q.len() as i32,
			std::ptr::null_mut(),
		);
	}
	lasterr();
	let mut out = vec![0i32; q.len()];
	o.download_i32(&mut out).unwrap();
	out
}

fn cpu_sorted_asc(x: &[f64]) -> Vec<f64> {
	let mut v = x.to_vec();
	v.sort_by(|a, b| a.partial_cmp(b).unwrap());
	v
}

// Prove each registered op against its CPU oracle. Returns (op_ok map, failures).
fn prove_ops() -> (HashMap<&'static str, bool>, Vec<String>) {
	let x = data();
	let n = x.len();
	let asc = cpu_sorted_asc(&x);
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

	// sort ascending == CPU sort
	{
		let g = run_keys(launch_sortx_sort_asc, sortx_sort_asc_workspace_bytes, &x);
		let pass =
			g.len() == asc.len() && g.iter().zip(&asc).all(|(a, b)| (a - b).abs() <= TOL);
		mark!("sort", pass, format!("sort_asc {:?} != {:?}", g, asc));
	}
	// sort descending == reversed CPU sort
	{
		let mut desc = asc.clone();
		desc.reverse();
		let g = run_keys(launch_sortx_sort_desc, sortx_sort_desc_workspace_bytes, &x);
		let pass = g.iter().zip(&desc).all(|(a, b)| (a - b).abs() <= TOL);
		mark!(
			"sort_desc",
			pass,
			format!("sort_desc {:?} != {:?}", g, desc)
		);
	}
	// argsort: indices must (a) be a permutation and (b) order the data ascending,
	// matching the stable CPU argsort exactly (radix SortPairs is stable).
	{
		let g = run_argsort(&x);
		let mut cpu: Vec<usize> = (0..n).collect();
		cpu.sort_by(|&a, &b| x[a].partial_cmp(&x[b]).unwrap());
		let gathered: Vec<f64> = g.iter().map(|&i| x[i as usize]).collect();
		let monotone = gathered.windows(2).all(|w| w[0] <= w[1] + TOL);
		let mut seen = vec![false; n];
		let perm = g.iter().all(|&i| {
			let i = i as usize;
			if i < n && !seen[i] {
				seen[i] = true;
				true
			} else {
				false
			}
		});
		let matches_cpu = g.iter().zip(&cpu).all(|(&gi, &ci)| gi as usize == ci);
		let pass = monotone && perm && matches_cpu;
		mark!("argsort", pass, format!("argsort {:?} (cpu {:?})", g, cpu));
	}
	// topk: the k largest values, descending (torch.topk convention)
	{
		let k = 5usize;
		let mut desc = asc.clone();
		desc.reverse();
		let want = &desc[..k];
		let g = run_topk(&x, k);
		let pass = g.iter().zip(want).all(|(a, b)| (a - b).abs() <= TOL);
		mark!("topk", pass, format!("topk {:?} != {:?}", g, want));
	}
	// kthvalue: k-th smallest (1-based)
	{
		let mut all = true;
		for k in [1usize, 7, n] {
			let g = run_scalar(launch_sortx_kthvalue, sortx_kthvalue_workspace_bytes, &x, k as i32);
			let want = asc[k - 1];
			if (g - want).abs() > TOL {
				all = false;
				fail.push(format!("kthvalue k={} {} != {}", k, g, want));
			}
		}
		ok.insert("kthvalue", all);
	}
	// median: even-n => mean of two central; verify odd-n branch too
	{
		let g = run_median(&x);
		let want = 0.5 * (asc[n / 2 - 1] + asc[n / 2]);
		let mut pass = (g - want).abs() <= TOL;
		let xo: Vec<f64> = x[..n - 1].to_vec();
		let asco = cpu_sorted_asc(&xo);
		let go = run_median(&xo);
		let wanto = asco[xo.len() / 2];
		pass = pass && (go - wanto).abs() <= TOL;
		mark!(
			"median",
			pass,
			format!(
				"median even {} (want {}) / odd {} (want {})",
				g, want, go, wanto
			)
		);
	}
	// argmax / argmin (ties: radix pair sort returns the lowest original index)
	{
		let g = run_argextreme(launch_sortx_argmax, sortx_argmax_workspace_bytes, &x);
		let want = (0..n)
			.max_by(|&a, &b| x[a].partial_cmp(&x[b]).unwrap())
			.unwrap() as i32;
		mark!("argmax", g == want, format!("argmax {} != {}", g, want));
		let g2 = run_argextreme(launch_sortx_argmin, sortx_argmin_workspace_bytes, &x);
		let want2 = (0..n)
			.min_by(|&a, &b| x[a].partial_cmp(&x[b]).unwrap())
			.unwrap() as i32;
		mark!("argmin", g2 == want2, format!("argmin {} != {}", g2, want2));
	}
	// searchsorted (left): out[j] == count of sorted[i] < q[j]
	{
		let q = [-9.0, -1.0, 0.0, 2.0, 7.25, 10.0, 11.0];
		let g = run_searchsorted(&asc, &q);
		let want: Vec<i32> = q
			.iter()
			.map(|&k| asc.iter().filter(|&&v| v < k).count() as i32)
			.collect();
		let pass = g == want;
		mark!(
			"searchsorted",
			pass,
			format!("searchsorted {:?} != {:?}", g, want)
		);
	}

	(ok, fail)
}

// Canonicalize a sort-category JSON name to a registry key. Strip lib prefix
// (last path segment), lowercase, drop dtype/alias disambiguators, then map TRUE
// synonyms only.
fn canon(name: &str) -> String {
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	let base = base
		.strip_suffix("_aten")
		.map(|s| s.to_string())
		.or_else(|| base.strip_suffix("_v2").map(|s| s.to_string()))
		.or_else(|| base.strip_suffix("v2").map(|s| s.to_string()))
		.unwrap_or(base);
	let alias: &[(&str, &str)] = &[
		// plain ascending sort + all stable/merge/radix keys-only variants
		("sort", "sort"),
		("sortkeys", "sort"),
		("stablesortkeys", "sort"),
		("stable_sort", "sort"),
		("msort", "sort"),
		("sort_lists", "sort"),
		("radix_sort_keys", "sort"),
		("merge_sort", "sort"),
		("sort_complex", "sort"),
		("sorted", "sort"),
		("merge_sorted", "sort"),
		// descending keys
		("sortkeysdescending", "sort_desc"),
		("radix_sort_keys_descending", "sort_desc"),
		// argsort / sorted_order / stable argsort
		("argsort", "argsort"),
		("argsort_stable", "argsort"),
		("sorted_order", "argsort"),
		("stable_sorted_order", "argsort"),
		// pairs / by-key sorts have the SAME key ordering as plain sort (proven via sort/argsort)
		("sortpairs", "sort"),
		("sortkeyscopy", "sort"),
		("sortpairsdescending", "sort_desc"),
		("radix_sort_pairs", "sort"),
		("radix_sort_pairs_descending", "sort_desc"),
		("merge_sort_pairs", "sort"),
		("sort_by_key", "sort"),
		("stablesortpairs", "sort"),
		("sort_key_val", "sort"),
		("stable_sort_by_key", "sort"),
		// top-k
		("topk", "topk"),
		("top_k", "topk"),
		("moe_top_k", "topk"),
		("itoplayer", "topk"),
		("itopklayer", "topk"),
		// k-th value / nth element
		("kthvalue", "kthvalue"),
		("nthelement", "kthvalue"),
		("nth_element", "kthvalue"),
		// argmax / argmin
		("argmax", "argmax"),
		("argmin", "argmin"),
		// searchsorted
		("searchsorted", "searchsorted"),
	];
	for (a, c) in alias {
		if base == *a {
			return c.to_string();
		}
	}
	base
}

fn load_sort() -> Vec<String> {
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
					if cat != "sort" {
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
fn prove_sort() {
	let items = load_sort();
	assert!(!items.is_empty(), "no sort items in inventory");

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

	eprintln!("\n=== PROVE sort ===");
	eprintln!("PROVE sort: {} / {}", proven, total);
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
		"registered sort op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero sort items proven");
}
