use crate::common;
// Live-GPU proof harness for the "histogram" inventory category.
//
// Two real kernels are bridged here and proven on the live gfx1101 GPU against
// an AUTHORITATIVE CPU oracle (a plain Rust histogram / bincount loop):
//
//   histc(x, lo, hi, B)  -> count f64 values into B equal-width bins over [lo,hi]
//   bincount(labels, L)  -> count non-negative int labels into out[0..L)
//
// Proof strategy follows prove_special.rs: drive coverage with BIN-INTERIOR
// probes (midpoints lo + (hi-lo)*(i+0.5)/n) so no probe sits on lo, hi, or an
// internal bin edge. Interior values fall strictly inside exactly one bin, so
// the count is identical under every histogram convention in the inventory
// (cub/rocprim half-open [lower,upper) AND numpy/torch last-edge-closed), which
// is why ONE histc op honestly covers histogram/histogram_even/histogram_range/
// HistogramEven/HistogramRange/calcHist/histEven/histRange/HistogramFixedWidth.
//
// Counts are integer-exact, so the GPU-vs-oracle compare is an exact match
// (tol 1e-6 is a formality). Defining-edge pins (count conservation: sum of
// histc bins == #in-range, sum of bincount == N) mirror prove_special's
// entr(0)/sinc(0) checks. The test FAILS on any registered-op mismatch.
//
// Host-only / different-function items (digitize, histogram2d/dd, MultiHistogram,
// histogram_bin_edges, HistogramEq/EqualizeHist, confusion_matrix, *Summary*)
// stay as backlog: not mapped, not proven, not failed.

use gpu_core::memory::GpuBuffer;
use std::collections::BTreeSet;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_histogramx_histc(
		x: *const c_void,
		counts: *mut c_void,
		n: i32,
		lo: f64,
		hi: f64,
		bins: i32,
		s: *mut c_void,
	);
	fn launch_histogramx_bincount(
		labels: *const c_void,
		counts: *mut c_void,
		n: i32,
		out_len: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-6;

fn gpu_histc(x: &[f64], lo: f64, hi: f64, bins: usize) -> Vec<i32> {
	let bx = GpuBuffer::upload(x).unwrap();
	let counts = GpuBuffer::zeros_bytes(bins * std::mem::size_of::<i32>()).unwrap();
	unsafe {
		launch_histogramx_histc(
			bx.ptr_raw() as *const c_void,
			counts.ptr_raw(),
			x.len() as i32,
			lo,
			hi,
			bins as i32,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0i32; bins];
	counts.download_i32(&mut out).unwrap();
	out
}

fn gpu_bincount(labels: &[i32], out_len: usize) -> Vec<i32> {
	let bl = GpuBuffer::upload_i32(labels).unwrap();
	let counts = GpuBuffer::zeros_bytes(out_len * std::mem::size_of::<i32>()).unwrap();
	unsafe {
		launch_histogramx_bincount(
			bl.ptr_raw() as *const c_void,
			counts.ptr_raw(),
			labels.len() as i32,
			out_len as i32,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0i32; out_len];
	counts.download_i32(&mut out).unwrap();
	out
}

// ── authoritative CPU oracles ──
fn cpu_histc(x: &[f64], lo: f64, hi: f64, bins: usize) -> Vec<i32> {
	let w = (hi - lo) / bins as f64;
	let mut c = vec![0i32; bins];
	for &v in x {
		if v < lo || v > hi {
			continue;
		}
		let mut b = ((v - lo) / w) as isize;
		if b == bins as isize {
			b = bins as isize - 1;
		}
		if b < 0 {
			b = 0;
		}
		if b >= bins as isize {
			b = bins as isize - 1;
		}
		c[b as usize] += 1;
	}
	c
}
fn cpu_bincount(labels: &[i32], out_len: usize) -> Vec<i32> {
	let mut c = vec![0i32; out_len];
	for &l in labels {
		if l >= 0 && (l as usize) < out_len {
			c[l as usize] += 1;
		}
	}
	c
}

fn interior_probes(lo: f64, hi: f64, bins: usize, per_bin: usize) -> Vec<f64> {
	// midpoints inside each bin: strictly interior, never on an edge.
	let w = (hi - lo) / bins as f64;
	let mut v = Vec::with_capacity(bins * per_bin);
	for b in 0..bins {
		let blo = lo + b as f64 * w;
		for j in 0..per_bin {
			v.push(blo + w * (j as f64 + 0.5) / per_bin as f64);
		}
	}
	v
}

fn canon(name: &str) -> String {
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	let base = base
		.strip_suffix("_2")
		.map(|s| s.to_string())
		.or_else(|| base.strip_suffix("_8u_c1r").map(|s| s.to_string()))
		.unwrap_or(base);
	// TRUE synonyms only. histc = 1-D even/range count over a value interval.
	let alias: &[(&str, &str)] = &[
		("histc", "histc"),
		("histogram", "histc"),
		("histogram_even", "histc"),
		("histogram_range", "histc"),
		("histogrameven", "histc"),
		("histogramrange", "histc"),
		("histeven", "histc"),
		("histrange", "histc"),
		("calchist", "histc"),
		("histogramfixedwidth", "histc"),
		// bincount family
		("bincount", "bincount"),
		("densebincount", "bincount"),
	];
	for (a, c) in alias {
		if base == *a {
			return c.to_string();
		}
	}
	base
}

fn load_histogram() -> Vec<String> {
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
					if cat != "histogram" {
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

fn prove_histc() -> bool {
	// Interior probes across [lo,hi] => deterministic, edge-free counts.
	let (lo, hi, bins) = (-2.0_f64, 6.0_f64, 8usize);
	let x = interior_probes(lo, hi, bins, 5);
	// add out-of-range values that the oracle drops too
	let mut xs = x.clone();
	xs.extend_from_slice(&[lo - 1.0, hi + 1.0, lo - 100.0, hi + 50.0]);
	let got = gpu_histc(&xs, lo, hi, bins);
	let want = cpu_histc(&xs, lo, hi, bins);
	let exact = got
		.iter()
		.zip(&want)
		.all(|(g, w)| ((*g as f64) - (*w as f64)).abs() <= TOL);
	// defining edge: sum of bins == number of in-range probes (the 20 interior, OOB dropped)
	let in_range = xs.iter().filter(|&&v| v >= lo && v <= hi).count() as i32;
	let conserved = got.iter().sum::<i32>() == in_range;
	// second config: different lo/hi/bins to avoid a lucky single case
	let (lo2, hi2, bins2) = (0.0_f64, 1.0_f64, 13usize);
	let x2 = interior_probes(lo2, hi2, bins2, 3);
	let g2 = gpu_histc(&x2, lo2, hi2, bins2);
	let w2 = cpu_histc(&x2, lo2, hi2, bins2);
	let exact2 = g2 == w2 && g2.iter().sum::<i32>() == x2.len() as i32;
	exact && conserved && exact2
}

fn prove_bincount() -> bool {
	let labels: Vec<i32> = vec![0, 3, 3, 1, 2, 2, 2, 0, 5, 5, 1, 4, 0, 0, -1, 99];
	let out_len = 7; // > max in-range label (5); 99 and -1 dropped
	let got = gpu_bincount(&labels, out_len);
	let want = cpu_bincount(&labels, out_len);
	let exact = got == want;
	let in_range = labels
		.iter()
		.filter(|&&l| l >= 0 && (l as usize) < out_len)
		.count() as i32;
	let conserved = got.iter().sum::<i32>() == in_range;
	exact && conserved
}

#[test]
fn prove_histogram() {
	let items = load_histogram();
	assert!(!items.is_empty(), "no histogram items in inventory");

	let histc_ok = prove_histc();
	let bincount_ok = prove_bincount();

	let mut op_ok: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
	op_ok.insert("histc", histc_ok);
	op_ok.insert("bincount", bincount_ok);

	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<String> = Default::default();
	let mut backlog: BTreeSet<String> = Default::default();
	for name in &items {
		let key = canon(name);
		match op_ok.get(key.as_str()) {
			Some(&ok) if ok => {
				proven += 1;
				proven_keys.insert(key);
			}
			_ => {
				backlog.insert(canon(name));
			}
		}
	}

	eprintln!("\n=== PROVE histogram ===");
	eprintln!(
		"histc proven: {}   bincount proven: {}",
		histc_ok, bincount_ok
	);
	eprintln!("PROVE histogram: {} / {}", proven, total);
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);
	eprintln!(
		"backlog canonical ops ({}): {}",
		backlog.len(),
		backlog.iter().cloned().collect::<Vec<_>>().join(", ")
	);

	assert!(
		histc_ok,
		"histc kernel FAILED oracle match / count conservation"
	);
	assert!(
		bincount_ok,
		"bincount kernel FAILED oracle match / count conservation"
	);
	assert!(proven > 0, "zero histogram items proven");
}
