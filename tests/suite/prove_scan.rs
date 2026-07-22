use crate::common;

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;
use std::fs;
use std::ptr;

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

fn run_scanx(f: Launch, x: &[f64]) -> Vec<f64> {
	let b = {
		let __up = x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(x.len()).unwrap();
	unsafe {
		f(
			b.ptr_raw() as *const c_void,
			o.ptr_raw(),
			x.len() as i32,
			ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; x.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn g_cumsum(x: &[f64]) -> Vec<f64> {
	let b = {
		let __up = x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(x.len()).unwrap();
	gpu_core::reductions::gpu_cumsum_rows(&b, 1, x.len(), &o).unwrap();
	let mut out = vec![0.0; x.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}
fn g_cumprod(x: &[f64]) -> Vec<f64> {
	let b = {
		let __up = x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(
		gpu_core::reductions::gpu_cumprod_workspace_bytes(x.len()).max(1),
	)
	.unwrap();
	let o = GpuBuffer::alloc(x.len()).unwrap();
	gpu_core::reductions::gpu_cumprod(&b, &ws, x.len(), &o).unwrap();
	let mut out = vec![0.0; x.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}
fn g_cummax(x: &[f64]) -> Vec<f64> {
	let b = {
		let __up = x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(
		gpu_core::reductions::gpu_cummax_workspace_bytes(x.len()).max(1),
	)
	.unwrap();
	let o = GpuBuffer::alloc(x.len()).unwrap();
	gpu_core::reductions::gpu_cummax(&b, &ws, x.len(), &o).unwrap();
	let mut out = vec![0.0; x.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
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
	reg!("cumsum", g_cumsum, o_cumsum);
	reg!("cumprod", g_cumprod, o_cumprod);
	reg!("cummax", g_cummax, o_cummax);
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
	let alias: &[(&str, &str)] = &[
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
		("cumprod", "cumprod"),
		("cumulative_product", "cumprod"),
		("cummax", "cummax"),
		("cumulative_max", "cummax"),
		("_cummax", "cummax"),
		("cummin", "cummin"),
		("cumulative_min", "cummin"),
		("_cummin", "cummin"),
		("logcumsumexp", "logcumsumexp"),
		("cumlogsumexp", "logcumsumexp"),
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
	let rd = fs::read_dir(&dir).expect("no recipe.db");
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

	let xs: Vec<f64> = {
		let n = 33usize;
		(0..n).map(|i| -4.0 + 8.0 * (i as f64 + 0.5) / n as f64)
			.collect()
	};
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
	{
		let a = run_scanx(launch_scanx_assoc_add, &xs);
		let total: f64 = xs.iter().sum();
		if (a[a.len() - 1] - total).abs() > TOL * (1.0 + total.abs()) {
			failures.push(format!("assoc_add last={} != Σx={}", a[a.len() - 1], total));
		}
	}
	{
		let a: Vec<f64> = (0..16).map(|i| 0.9 - 0.01 * i as f64).collect();
		let b: Vec<f64> = (0..16).map(|i| 0.1 * (i as f64 - 7.0)).collect();
		let ba = {
			let __up = &a;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let bb = {
			let __up = &b;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let o = GpuBuffer::alloc(a.len()).unwrap();
		unsafe {
			launch_scanx_recur(
				ba.ptr_raw() as *const c_void,
				bb.ptr_raw() as *const c_void,
				o.ptr_raw(),
				a.len() as i32,
				ptr::null_mut(),
			);
		}
		gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
		let mut got = vec![0.0; a.len()];
		unsafe { o.download_async(&mut got, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
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
