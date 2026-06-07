mod common;
// Live-GPU proof harness for the "distance" inventory category.
//
// For every distance-category item in kernel_inventory/*.json, canonicalize its
// name; if that canonical op is registered here, run the gpu-core op on the LIVE
// gfx1101 GPU and assert it matches an AUTHORITATIVE CPU oracle (std f64 textbook
// pairwise definitions). tol 1e-7.
//
// Conventions are matched to the kernels, NOT to the inventory name:
//   - pairwise_l2 (existing)      → SQUARED L2, no sqrt    : Σ diff²
//   - pairwise_cosine (existing)  → cosine SIMILARITY      : dot/(‖q‖‖t‖), 0/0→0
//   - pairwise_l1 (existing)      → L1                     : Σ |diff|
//   - pairwise_hamming (existing) → u8 mismatch fraction   : #(q≠t)/dim
//   - distancex_manhattan (new)   → L1
//   - distancex_chebyshev (new)   → Linf  : max|diff|
//   - distancex_minkowski (new)   → Lp    : (Σ|diff|^p)^(1/p)
//   - distancex_braycurtis (new)  → Σ|diff| / Σ|q+t|, 0/0→0
//   - distancex_canberra (new)    → Σ |diff|/(|q|+|t|), per-term 0/0→0
//
// Canonicalization honesty: generic-euclidean inventory names (cdist/pdist/
// pairwise_distance/...) route to minkowski(p=2) = TRUE euclidean (sqrt), never to
// the squared-L2 kernel. Only the explicitly-"squared L2" names map to l2.
// Backward / structural / different-function items stay backlog (never faked green).

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_distancex_manhattan(
		q: *const c_void,
		t: *const c_void,
		o: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		s: *mut c_void,
	);
	fn launch_distancex_chebyshev(
		q: *const c_void,
		t: *const c_void,
		o: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		s: *mut c_void,
	);
	fn launch_distancex_minkowski(
		q: *const c_void,
		t: *const c_void,
		o: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		p: f64,
		s: *mut c_void,
	);
	fn launch_distancex_braycurtis(
		q: *const c_void,
		t: *const c_void,
		o: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		s: *mut c_void,
	);
	fn launch_distancex_canberra(
		q: *const c_void,
		t: *const c_void,
		o: *mut c_void,
		nq: i32,
		nt: i32,
		dim: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-7;

// ── test data: strictly positive [0.5,5] so diffs take both signs and
//    braycurtis / canberra denominators stay nonzero (off the 0/0 edge). ──
fn make_data() -> (Vec<f64>, Vec<f64>, usize, usize, usize) {
	let (nq, nt, dim) = (5usize, 7usize, 4usize);
	let mut q = vec![0.0; nq * dim];
	let mut t = vec![0.0; nt * dim];
	for (i, v) in q.iter_mut().enumerate() {
		*v = 0.5 + 4.5 * (((i * 7 + 3) % 11) as f64) / 10.0;
	}
	for (i, v) in t.iter_mut().enumerate() {
		*v = 0.5 + 4.5 * (((i * 5 + 1) % 11) as f64) / 10.0;
	}
	(q, t, nq, nt, dim)
}

type FlatLaunch =
	unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void, i32, i32, i32, *mut c_void);

fn run_flat_f64(f: FlatLaunch, q: &[f64], t: &[f64], nq: usize, nt: usize, dim: usize) -> Vec<f64> {
	let bq = GpuBuffer::upload(q).unwrap();
	let bt = GpuBuffer::upload(t).unwrap();
	let o = GpuBuffer::alloc(nq * nt).unwrap();
	unsafe {
		f(
			bq.ptr_raw() as *const c_void,
			bt.ptr_raw() as *const c_void,
			o.ptr_raw(),
			nq as i32,
			nt as i32,
			dim as i32,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut out = vec![0.0; nq * nt];
	o.download(&mut out).unwrap();
	out
}

// CPU oracle: apply a per-pair reducer over the (nq x nt) grid.
fn oracle_grid<F: Fn(&[f64], &[f64]) -> f64>(
	q: &[f64],
	t: &[f64],
	nq: usize,
	nt: usize,
	dim: usize,
	f: F,
) -> Vec<f64> {
	let mut out = vec![0.0; nq * nt];
	for qi in 0..nq {
		for ti in 0..nt {
			let a = &q[qi * dim..qi * dim + dim];
			let b = &t[ti * dim..ti * dim + dim];
			out[qi * nt + ti] = f(a, b);
		}
	}
	out
}

fn close(a: &[f64], b: &[f64]) -> Option<(usize, f64, f64)> {
	for (i, (x, y)) in a.iter().zip(b).enumerate() {
		if (x - y).abs() > TOL * (1.0 + y.abs()) {
			return Some((i, *x, *y));
		}
	}
	None
}

// ── canonicalize a distance JSON name to a registry key ──
fn canon(name: &str) -> Option<&'static str> {
	let base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	let base = base
		.strip_suffix("_aten")
		.map(|s| s.to_string())
		.unwrap_or(base);
	// explicitly squared-L2 → l2 kernel (squared, no sqrt)
	match base.as_str() {
		"l2unexpanded" | "l2expanded" => return Some("l2_squared"),
		// cosine SIMILARITY family → cosine kernel
		"cosine_similarity" | "cosineexpanded" | "cosine" => return Some("cosine"),
		// hamming (u8) → hamming kernel
		"hamming" => return Some("hamming"),
		// generic euclidean cross/pairwise distance → minkowski(p=2) = true euclidean
		"cdist"
		| "_cdist_forward"
		| "cdist_forward"
		| "pdist"
		| "_pdist_forward"
		| "pdist_forward"
		| "native_pdist"
		| "pairwise_distance"
		| "pairwise_distances"
		| "pairwise"
		| "pairwise_point_distance" => return Some("euclidean"),
		_ => {}
	}
	None
}

// distance items in the inventory (deduped).
fn load_distance() -> Vec<String> {
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
					if k.get("category").and_then(|c| c.as_str()) != Some("distance") {
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
fn prove_distance() {
	let (q, t, nq, nt, dim) = make_data();
	let mut failures: Vec<String> = Vec::new();
	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	let mut assert_op =
		|key: &'static str, got: Vec<f64>, want: Vec<f64>, failures: &mut Vec<String>| {
			let ok = close(&got, &want).is_none();
			if let Some((i, g, w)) = close(&got, &want) {
				failures.push(format!("{}: idx {} gpu={} cpu={}", key, i, g, w));
			}
			op_ok.insert(key, ok);
		};

	// ── existing kernels (proven against their TRUE convention) ──
	// pairwise_l2 → SQUARED L2
	{
		let got = gpu_core::kernels::gpu_pairwise_l2(
			&GpuBuffer::upload(&q).unwrap(),
			&GpuBuffer::upload(&t).unwrap(),
			nq,
			nt,
			dim,
		)
		.unwrap()
		.download_vec()
		.unwrap();
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
		});
		assert_op("l2_squared", got, want, &mut failures);
	}
	// pairwise_cosine → cosine SIMILARITY
	{
		let got = gpu_core::encoding::gpu_pairwise_cosine(
			&GpuBuffer::upload(&q).unwrap(),
			&GpuBuffer::upload(&t).unwrap(),
			nq,
			nt,
			dim,
		)
		.unwrap()
		.download_vec()
		.unwrap();
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
			let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
			let nb: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
			let den = na * nb;
			if den > 0.0 { dot / den } else { 0.0 }
		});
		assert_op("cosine", got, want, &mut failures);
	}
	// pairwise_l1 → L1
	{
		let got = gpu_core::encoding::gpu_pairwise_l1(
			&GpuBuffer::upload(&q).unwrap(),
			&GpuBuffer::upload(&t).unwrap(),
			nq,
			nt,
			dim,
		)
		.unwrap()
		.download_vec()
		.unwrap();
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
		});
		assert_op("manhattan", got.clone(), want.clone(), &mut failures);
	}
	// pairwise_hamming → u8 mismatch fraction (separate u8 runner + integer oracle)
	{
		let qd = dim;
		let nqh = 5usize;
		let nth = 6usize;
		let mut qu = vec![0u8; nqh * qd];
		let mut tu = vec![0u8; nth * qd];
		for (i, v) in qu.iter_mut().enumerate() {
			*v = ((i * 3 + 1) % 4) as u8;
		}
		for (i, v) in tu.iter_mut().enumerate() {
			*v = ((i * 2 + 2) % 4) as u8;
		}
		let got = gpu_core::encoding::gpu_pairwise_hamming(
			&GpuBuffer::upload_u8(&qu).unwrap(),
			&GpuBuffer::upload_u8(&tu).unwrap(),
			nqh,
			nth,
			qd,
		)
		.unwrap()
		.download_vec()
		.unwrap();
		let mut want = vec![0.0; nqh * nth];
		for qi in 0..nqh {
			for ti in 0..nth {
				let mut m = 0usize;
				for d in 0..qd {
					if qu[qi * qd + d] != tu[ti * qd + d] {
						m += 1;
					}
				}
				want[qi * nth + ti] = m as f64 / qd as f64;
			}
		}
		assert_op("hamming", got, want, &mut failures);
	}

	// ── new distancex_ kernels ──
	// manhattan (new, independent of pairwise_l1): L1
	{
		let got = run_flat_f64(launch_distancex_manhattan, &q, &t, nq, nt, dim);
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
		});
		assert_op("manhattan", got, want, &mut failures);
	}
	// chebyshev: Linf
	{
		let got = run_flat_f64(launch_distancex_chebyshev, &q, &t, nq, nt, dim);
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			a.iter()
				.zip(b)
				.map(|(x, y)| (x - y).abs())
				.fold(0.0_f64, f64::max)
		});
		assert_op("chebyshev", got, want, &mut failures);
	}
	// minkowski(p=3): (Σ|diff|^p)^(1/p)
	{
		let p = 3.0_f64;
		let bq = GpuBuffer::upload(&q).unwrap();
		let bt = GpuBuffer::upload(&t).unwrap();
		let o = GpuBuffer::alloc(nq * nt).unwrap();
		unsafe {
			launch_distancex_minkowski(
				bq.ptr_raw() as *const c_void,
				bt.ptr_raw() as *const c_void,
				o.ptr_raw(),
				nq as i32,
				nt as i32,
				dim as i32,
				p,
				std::ptr::null_mut(),
			);
		}
		gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
		let got = o.download_vec().unwrap();
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			let s: f64 = a.iter().zip(b).map(|(x, y)| (x - y).abs().powf(p)).sum();
			s.powf(1.0 / p)
		});
		assert_op("minkowski", got, want, &mut failures);
	}
	// minkowski(p=2) cross-check == sqrt(l2-squared kernel output)
	{
		let p = 2.0_f64;
		let bq = GpuBuffer::upload(&q).unwrap();
		let bt = GpuBuffer::upload(&t).unwrap();
		let o = GpuBuffer::alloc(nq * nt).unwrap();
		unsafe {
			launch_distancex_minkowski(
				bq.ptr_raw() as *const c_void,
				bt.ptr_raw() as *const c_void,
				o.ptr_raw(),
				nq as i32,
				nt as i32,
				dim as i32,
				p,
				std::ptr::null_mut(),
			);
		}
		gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
		let mink2 = o.download_vec().unwrap();
		// CPU oracle: true euclidean = sqrt(Σ diff²)
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			a.iter()
				.zip(b)
				.map(|(x, y)| (x - y) * (x - y))
				.sum::<f64>()
				.sqrt()
		});
		assert_op("euclidean", mink2.clone(), want, &mut failures);
		// bonus cross-check: GPU minkowski(p=2) == sqrt(GPU squared-L2 kernel)
		let l2sq = gpu_core::kernels::gpu_pairwise_l2(
			&GpuBuffer::upload(&q).unwrap(),
			&GpuBuffer::upload(&t).unwrap(),
			nq,
			nt,
			dim,
		)
		.unwrap()
		.download_vec()
		.unwrap();
		let l2root: Vec<f64> = l2sq.iter().map(|v| v.sqrt()).collect();
		if close(&mink2, &l2root).is_some() {
			failures.push("minkowski(p=2) != sqrt(l2_squared)".into());
		}
	}
	// bray-curtis: Σ|diff| / Σ|q+t|
	{
		let got = run_flat_f64(launch_distancex_braycurtis, &q, &t, nq, nt, dim);
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			let num: f64 = a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum();
			let den: f64 = a.iter().zip(b).map(|(x, y)| (x + y).abs()).sum();
			if den > 0.0 { num / den } else { 0.0 }
		});
		assert_op("braycurtis", got, want, &mut failures);
	}
	// canberra: Σ |diff|/(|q|+|t|), per-term 0/0→0
	{
		let got = run_flat_f64(launch_distancex_canberra, &q, &t, nq, nt, dim);
		let want = oracle_grid(&q, &t, nq, nt, dim, |a, b| {
			a.iter()
				.zip(b)
				.map(|(x, y)| {
					let den = x.abs() + y.abs();
					if den > 0.0 { (x - y).abs() / den } else { 0.0 }
				})
				.sum()
		});
		assert_op("canberra", got, want, &mut failures);
	}

	// defining-edge probes: canberra(0,0)-term → 0 ; braycurtis all-zero pair → 0.
	{
		let zq = vec![0.0; dim];
		let zt = vec![0.0; dim];
		let c = run_flat_f64(launch_distancex_canberra, &zq, &zt, 1, 1, dim);
		if c[0] != 0.0 {
			failures.push(format!("canberra(0,0)={} != 0", c[0]));
		}
		let bc = run_flat_f64(launch_distancex_braycurtis, &zq, &zt, 1, 1, dim);
		if bc[0] != 0.0 {
			failures.push(format!("braycurtis(0,0)={} != 0", bc[0]));
		}
	}

	// walk inventory → proven count
	let items = load_distance();
	assert!(!items.is_empty(), "no distance items in inventory");
	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<&'static str> = BTreeSet::new();
	for name in &items {
		if let Some(key) = canon(name)
			&& *op_ok.get(key).unwrap_or(&false)
		{
			proven += 1;
			proven_keys.insert(key);
		}
	}

	let implemented = [
		"braycurtis",
		"canberra",
		"chebyshev",
		"manhattan",
		"minkowski",
	];
	eprintln!("\n=== PROVE distance ===");
	eprintln!("PROVE distance: {} / {}", proven, total);
	let mut reg: Vec<&str> = op_ok.keys().copied().collect();
	reg.sort();
	eprintln!("registered ops ({}): {}", reg.len(), reg.join(", "));
	eprintln!(
		"new kernels implemented ({}): {}",
		implemented.len(),
		implemented.join(", ")
	);
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().copied().collect::<Vec<_>>().join(", ")
	);

	assert!(
		failures.is_empty(),
		"registered distance op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero distance items proven");
}
