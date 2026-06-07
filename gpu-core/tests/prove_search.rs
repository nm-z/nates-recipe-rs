mod common;
// Live-GPU proof harness for the "search" inventory category.
//
// For every search-category item in kernel_inventory/*.json, canonicalize its
// name; if that canonical op is registered here, run the gpu-core searchx_ kernel
// on the LIVE gfx1101 GPU and assert it matches an AUTHORITATIVE CPU oracle.
//
// Index-valued ops (searchsorted, bucketize, nonzero, argmax, idamax, find,
// partition_point, ...) return i32 and are checked with EXACT equality — a float
// tolerance would hide the off-by-one errors these ops are prone to. The few
// genuinely host-only / structural items (ANN index build/search, LLM sampling,
// retrieval, scipy peak prominence/width) stay in `total` as backlog, never faked
// into `proven`.
//
// digitize, bucketize and searchsorted share ONE binary-search kernel but each has
// its OWN oracle here: numpy.digitize and torch.bucketize use opposite `right`
// senses, so collapsing them would prove the wrong convention.

use gpu_core::memory::GpuBuffer;
use std::collections::BTreeSet;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_searchx_searchsorted(
		a: *const c_void,
		na: i32,
		v: *const c_void,
		out: *mut c_void,
		nv: i32,
		side: i32,
		s: *mut c_void,
	);
	fn launch_searchx_isin(
		a: *const c_void,
		na: i32,
		x: *const c_void,
		out: *mut c_void,
		nx: i32,
		s: *mut c_void,
	);
	fn launch_searchx_nonzero(
		x: *const c_void,
		n: i32,
		out: *mut c_void,
		cnt: *mut c_void,
		s: *mut c_void,
	);
	fn launch_searchx_find(
		x: *const c_void,
		n: i32,
		target: f64,
		out: *mut c_void,
		s: *mut c_void,
	);
	fn launch_searchx_find_n(
		x: *const c_void,
		n: i32,
		target: f64,
		k: i32,
		out: *mut c_void,
		s: *mut c_void,
	);
	fn launch_searchx_is_sorted(x: *const c_void, n: i32, out: *mut c_void, s: *mut c_void);
	fn launch_searchx_is_sorted_until(x: *const c_void, n: i32, out: *mut c_void, s: *mut c_void);
	fn launch_searchx_partition_point(
		x: *const c_void,
		n: i32,
		pivot: f64,
		out: *mut c_void,
		s: *mut c_void,
	);
	fn launch_searchx_mismatch(
		a: *const c_void,
		b: *const c_void,
		n: i32,
		out: *mut c_void,
		s: *mut c_void,
	);
	fn launch_searchx_argextreme(
		x: *const c_void,
		n: i32,
		mode: i32,
		out: *mut c_void,
		s: *mut c_void,
	);
	fn launch_searchx_minmax_element(x: *const c_void, n: i32, out: *mut c_void, s: *mut c_void);
	fn launch_searchx_argrel(
		x: *const c_void,
		n: i32,
		mode: i32,
		mask: *mut c_void,
		s: *mut c_void,
	);
}

// ── GPU runners (upload f64, download i32) ───────────────────────────────────

fn searchsorted_gpu(a: &[f64], v: &[f64], side: i32) -> Vec<i32> {
	let ba = GpuBuffer::upload(a).unwrap();
	let bv = GpuBuffer::upload(v).unwrap();
	let out = GpuBuffer::alloc_bytes(v.len() * 4).unwrap();
	unsafe {
		launch_searchx_searchsorted(
			ba.ptr_raw() as *const c_void,
			a.len() as i32,
			bv.ptr_raw() as *const c_void,
			out.ptr_raw(),
			v.len() as i32,
			side,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut o = vec![0i32; v.len()];
	out.download_i32(&mut o).unwrap();
	o
}

fn isin_gpu(a: &[f64], x: &[f64]) -> Vec<i32> {
	let ba = GpuBuffer::upload(a).unwrap();
	let bx = GpuBuffer::upload(x).unwrap();
	let out = GpuBuffer::alloc_bytes(x.len() * 4).unwrap();
	unsafe {
		launch_searchx_isin(
			ba.ptr_raw() as *const c_void,
			a.len() as i32,
			bx.ptr_raw() as *const c_void,
			out.ptr_raw(),
			x.len() as i32,
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut o = vec![0i32; x.len()];
	out.download_i32(&mut o).unwrap();
	o
}

fn nonzero_gpu(x: &[f64]) -> Vec<i32> {
	let bx = GpuBuffer::upload(x).unwrap();
	let out = GpuBuffer::alloc_bytes(x.len().max(1) * 4).unwrap();
	let cnt = GpuBuffer::alloc_bytes(4).unwrap();
	unsafe {
		launch_searchx_nonzero(
			bx.ptr_raw() as *const c_void,
			x.len() as i32,
			out.ptr_raw(),
			cnt.ptr_raw(),
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut c = vec![0i32; 1];
	cnt.download_i32(&mut c).unwrap();
	let mut o = vec![0i32; x.len().max(1)];
	out.download_i32(&mut o).unwrap();
	o.truncate(c[0] as usize);
	o
}

fn scalar_i32(x: &[f64], launch: impl Fn(*const c_void, i32, *mut c_void)) -> i32 {
	let bx = GpuBuffer::upload(x).unwrap();
	let out = GpuBuffer::alloc_bytes(4).unwrap();
	launch(bx.ptr_raw() as *const c_void, x.len() as i32, out.ptr_raw());
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut o = vec![0i32; 1];
	out.download_i32(&mut o).unwrap();
	o[0]
}

fn find_gpu(x: &[f64], target: f64) -> i32 {
	scalar_i32(x, |p, n, out| unsafe {
		launch_searchx_find(p, n, target, out, std::ptr::null_mut());
	})
}
fn find_n_gpu(x: &[f64], target: f64, k: i32) -> i32 {
	scalar_i32(x, |p, n, out| unsafe {
		launch_searchx_find_n(p, n, target, k, out, std::ptr::null_mut());
	})
}
fn is_sorted_gpu(x: &[f64]) -> i32 {
	scalar_i32(x, |p, n, out| unsafe {
		launch_searchx_is_sorted(p, n, out, std::ptr::null_mut());
	})
}
fn is_sorted_until_gpu(x: &[f64]) -> i32 {
	scalar_i32(x, |p, n, out| unsafe {
		launch_searchx_is_sorted_until(p, n, out, std::ptr::null_mut());
	})
}
fn partition_point_gpu(x: &[f64], pivot: f64) -> i32 {
	scalar_i32(x, |p, n, out| unsafe {
		launch_searchx_partition_point(p, n, pivot, out, std::ptr::null_mut());
	})
}
fn argextreme_gpu(x: &[f64], mode: i32) -> i32 {
	scalar_i32(x, |p, n, out| unsafe {
		launch_searchx_argextreme(p, n, mode, out, std::ptr::null_mut());
	})
}

fn mismatch_gpu(a: &[f64], b: &[f64]) -> i32 {
	let ba = GpuBuffer::upload(a).unwrap();
	let bb = GpuBuffer::upload(b).unwrap();
	let out = GpuBuffer::alloc_bytes(4).unwrap();
	unsafe {
		launch_searchx_mismatch(
			ba.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			a.len() as i32,
			out.ptr_raw(),
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut o = vec![0i32; 1];
	out.download_i32(&mut o).unwrap();
	o[0]
}

fn minmax_element_gpu(x: &[f64]) -> (i32, i32) {
	let bx = GpuBuffer::upload(x).unwrap();
	let out = GpuBuffer::alloc_bytes(8).unwrap();
	unsafe {
		launch_searchx_minmax_element(
			bx.ptr_raw() as *const c_void,
			x.len() as i32,
			out.ptr_raw(),
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut o = vec![0i32; 2];
	out.download_i32(&mut o).unwrap();
	(o[0], o[1])
}

fn argrel_gpu(x: &[f64], mode: i32) -> Vec<i32> {
	let bx = GpuBuffer::upload(x).unwrap();
	let out = GpuBuffer::alloc_bytes(x.len() * 4).unwrap();
	unsafe {
		launch_searchx_argrel(
			bx.ptr_raw() as *const c_void,
			x.len() as i32,
			mode,
			out.ptr_raw(),
			std::ptr::null_mut(),
		);
	}
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
	let mut o = vec![0i32; x.len()];
	out.download_i32(&mut o).unwrap();
	o
}

// ── CPU oracles (authoritative numpy/torch/thrust/BLAS conventions) ──────────

fn ora_searchsorted(a: &[f64], v: &[f64], right: bool) -> Vec<i32> {
	// numpy: side='left' -> first i with a[i] >= key ; side='right' -> first i with a[i] > key
	v.iter()
		.map(|&key| {
			let mut c = 0i32;
			for &av in a {
				if (right && av <= key) || (!right && av < key) {
					c += 1;
				} else {
					break;
				}
			}
			c
		})
		.collect()
}
fn ora_digitize(x: &[f64], bins: &[f64], right: bool) -> Vec<i32> {
	// numpy.digitize, increasing bins:
	//   right=false -> bins[i-1] <= x < bins[i]  == searchsorted(side='right')
	//   right=true  -> bins[i-1] <  x <= bins[i] == searchsorted(side='left')
	ora_searchsorted(bins, x, !right)
}
fn ora_bucketize(x: &[f64], bnd: &[f64], right: bool) -> Vec<i32> {
	// torch.bucketize:
	//   right=false (default) -> lower_bound == searchsorted(side='left')
	//   right=true            -> upper_bound == searchsorted(side='right')
	ora_searchsorted(bnd, x, right)
}
fn ora_isin(a: &[f64], x: &[f64]) -> Vec<i32> {
	x.iter()
		.map(|k| if a.contains(k) { 1 } else { 0 })
		.collect()
}
fn ora_nonzero(x: &[f64]) -> Vec<i32> {
	x.iter()
		.enumerate()
		.filter(|&(_, &v)| v != 0.0)
		.map(|(i, _)| i as i32)
		.collect()
}
fn ora_find(x: &[f64], t: f64) -> i32 {
	x.iter()
		.position(|&v| v == t)
		.map(|i| i as i32)
		.unwrap_or(x.len() as i32)
}
fn ora_find_n(x: &[f64], t: f64, k: usize) -> i32 {
	let mut run = 0usize;
	for (i, &v) in x.iter().enumerate() {
		if v == t {
			run += 1;
			if run >= k {
				return (i + 1 - k) as i32;
			}
		} else {
			run = 0;
		}
	}
	x.len() as i32
}
fn ora_is_sorted_until(x: &[f64]) -> i32 {
	for i in 1..x.len() {
		if x[i] < x[i - 1] {
			return i as i32;
		}
	}
	x.len() as i32
}
fn ora_partition_point(x: &[f64], pivot: f64) -> i32 {
	x.iter().take_while(|&&v| v < pivot).count() as i32
}
fn ora_mismatch(a: &[f64], b: &[f64]) -> i32 {
	for i in 0..a.len() {
		if a[i] != b[i] {
			return i as i32;
		}
	}
	a.len() as i32
}
fn ora_argmax(x: &[f64]) -> i32 {
	(0..x.len())
		.max_by(|&i, &j| x[i].partial_cmp(&x[j]).unwrap().then(j.cmp(&i)))
		.unwrap() as i32
}
fn ora_argmin(x: &[f64]) -> i32 {
	(0..x.len())
		.min_by(|&i, &j| x[i].partial_cmp(&x[j]).unwrap().then(i.cmp(&j)))
		.unwrap() as i32
}
fn ora_idamax(x: &[f64]) -> i32 {
	// BLAS Idamax: index of first element with max |x| (0-based here).
	let mut best = 0usize;
	let mut bv = x[0].abs();
	for i in 1..x.len() {
		let a = x[i].abs();
		if a > bv {
			bv = a;
			best = i;
		}
	}
	best as i32
}
fn ora_idamin(x: &[f64]) -> i32 {
	let mut best = 0usize;
	let mut bv = x[0].abs();
	for i in 1..x.len() {
		let a = x[i].abs();
		if a < bv {
			bv = a;
			best = i;
		}
	}
	best as i32
}
fn ora_argrel(x: &[f64], mode: i32) -> Vec<i32> {
	// scipy argrelmax/min/extrema, order=1, no boundary wrap. Returns 0/1 mask.
	(0..x.len())
		.map(|i| {
			if i == 0 || i + 1 >= x.len() {
				return 0;
			}
			let (a, v, b) = (x[i - 1], x[i], x[i + 1]);
			let mx = v > a && v > b;
			let mn = v < a && v < b;
			match mode {
				0 => mx as i32,
				1 => mn as i32,
				_ => (mx || mn) as i32,
			}
		})
		.collect()
}

// ── op registry: each key proven once, exact-equality, with defining edge probes ─

fn registered() -> BTreeSet<&'static str> {
	[
		"searchsorted",
		"lower_bound",
		"upper_bound",
		"digitize",
		"bucketize",
		"isin",
		"nonzero",
		"find",
		"find_n",
		"is_sorted",
		"is_sorted_until",
		"partition_point",
		"mismatch",
		"argmax",
		"argmin",
		"minmax_element",
		"idamax",
		"idamin",
		"argrelmax",
		"argrelmin",
		"argrelextrema",
	]
	.into_iter()
	.collect()
}

// Run every registered op against its oracle; return the set that PASSES and a
// list of human-readable failures. Probes deliberately put values EXACTLY on
// array entries so left/right tie-breaks are exercised — without ties, the whole
// searchsorted/bucketize convention is untested.
fn prove_ops(failures: &mut Vec<String>) -> BTreeSet<&'static str> {
	let mut ok: BTreeSet<&'static str> = BTreeSet::new();
	macro_rules! pass {
		($k:expr, $cond:expr, $msg:expr) => {
			if $cond {
				ok.insert($k);
			} else {
				failures.push(format!("{}: {}", $k, $msg));
			}
		};
	}

	// sorted array with DUPLICATES + needles that hit boundaries exactly.
	let a = [1.0, 3.0, 3.0, 5.0, 7.0, 9.0];
	let v = [0.0, 1.0, 3.0, 4.0, 9.0, 10.0];

	// searchsorted left (default): tie -> leftmost. e.g. v=3 -> index 1.
	let g = searchsorted_gpu(&a, &v, 0);
	pass!(
		"searchsorted",
		g == ora_searchsorted(&a, &v, false),
		format!(
			"left got {:?} want {:?}",
			g,
			ora_searchsorted(&a, &v, false)
		)
	);
	// lower_bound == searchsorted left.
	pass!(
		"lower_bound",
		searchsorted_gpu(&a, &v, 0) == ora_searchsorted(&a, &v, false),
		"lower_bound != side=left"
	);
	// upper_bound == searchsorted right: tie -> past the run. v=3 -> index 3.
	let gr = searchsorted_gpu(&a, &v, 1);
	pass!(
		"upper_bound",
		gr == ora_searchsorted(&a, &v, true),
		format!("got {:?} want {:?}", gr, ora_searchsorted(&a, &v, true))
	);
	// The tie MUST differ between left and right at v=3 (index 2 in `v`).
	if g[2] == gr[2] {
		failures.push(format!(
			"searchsorted tie not distinguished: left={} right={}",
			g[2], gr[2]
		));
		ok.remove("searchsorted");
	}

	// digitize: numpy convention, OWN oracle (right flag reversed vs side).
	let bins = [1.0, 3.0, 5.0, 7.0, 9.0];
	let dx = [0.5, 1.0, 3.0, 4.0, 9.0, 9.5];
	// right=false -> searchsorted(side='right')
	let gdf = searchsorted_gpu(&bins, &dx, 1);
	pass!(
		"digitize",
		gdf == ora_digitize(&dx, &bins, false),
		format!(
			"right=false got {:?} want {:?}",
			gdf,
			ora_digitize(&dx, &bins, false)
		)
	);
	// right=true  -> searchsorted(side='left'); must differ at exact-bin ties.
	let gdt = searchsorted_gpu(&bins, &dx, 0);
	if gdt != ora_digitize(&dx, &bins, true) {
		failures.push(format!(
			"digitize right=true got {:?} want {:?}",
			gdt,
			ora_digitize(&dx, &bins, true)
		));
		ok.remove("digitize");
	}

	// bucketize: torch convention, OWN oracle (right flag opposite sense to digitize).
	let bnd = [1.0, 3.0, 5.0, 7.0, 9.0];
	let bx = [0.5, 1.0, 3.0, 6.0, 9.0, 9.5];
	// right=false (default) -> lower_bound == side='left'
	let gbf = searchsorted_gpu(&bnd, &bx, 0);
	pass!(
		"bucketize",
		gbf == ora_bucketize(&bx, &bnd, false),
		format!(
			"right=false got {:?} want {:?}",
			gbf,
			ora_bucketize(&bx, &bnd, false)
		)
	);
	// right=true -> upper_bound == side='right'
	let gbt = searchsorted_gpu(&bnd, &bx, 1);
	if gbt != ora_bucketize(&bx, &bnd, true) {
		failures.push(format!(
			"bucketize right=true got {:?} want {:?}",
			gbt,
			ora_bucketize(&bx, &bnd, true)
		));
		ok.remove("bucketize");
	}
	// bucketize default tie (v=3 == bnd[1]) must land at 1 (lower), not 2.
	if gbf[2] != 1 {
		failures.push(format!("bucketize tie v=3 got {} want 1", gbf[2]));
		ok.remove("bucketize");
	}

	// isin / contains.
	let hay = [2.0, 4.0, 6.0, 8.0];
	let needles = [1.0, 2.0, 5.0, 8.0, 9.0];
	let gi = isin_gpu(&hay, &needles);
	pass!(
		"isin",
		gi == ora_isin(&hay, &needles),
		format!("got {:?} want {:?}", gi, ora_isin(&hay, &needles))
	);

	// nonzero / argwhere / where: compacted indices, EXACT incl. length.
	let nz = [0.0, 5.0, 0.0, -2.0, 0.0, 0.0, 7.0];
	let gn = nonzero_gpu(&nz);
	pass!(
		"nonzero",
		gn == ora_nonzero(&nz),
		format!("got {:?} want {:?}", gn, ora_nonzero(&nz))
	);
	// edge: all-zero -> empty.
	if !nonzero_gpu(&[0.0, 0.0, 0.0]).is_empty() {
		failures.push("nonzero all-zero not empty".into());
		ok.remove("nonzero");
	}

	// find / find_if: first occurrence; absent -> n.
	let fx = [4.0, 1.0, 3.0, 1.0, 2.0];
	pass!(
		"find",
		find_gpu(&fx, 1.0) == ora_find(&fx, 1.0) && find_gpu(&fx, 9.0) == ora_find(&fx, 9.0),
		format!(
			"got {}/{} want {}/{}",
			find_gpu(&fx, 1.0),
			find_gpu(&fx, 9.0),
			ora_find(&fx, 1.0),
			ora_find(&fx, 9.0)
		)
	);

	// find_n: first run of k consecutive matches.
	let fn_x = [1.0, 2.0, 2.0, 2.0, 1.0, 2.0, 2.0];
	pass!(
		"find_n",
		find_n_gpu(&fn_x, 2.0, 3) == ora_find_n(&fn_x, 2.0, 3)
			&& find_n_gpu(&fn_x, 2.0, 4) == ora_find_n(&fn_x, 2.0, 4),
		format!(
			"got {}/{} want {}/{}",
			find_n_gpu(&fn_x, 2.0, 3),
			find_n_gpu(&fn_x, 2.0, 4),
			ora_find_n(&fn_x, 2.0, 3),
			ora_find_n(&fn_x, 2.0, 4)
		)
	);

	// is_sorted.
	pass!(
		"is_sorted",
		is_sorted_gpu(&[1.0, 2.0, 2.0, 5.0]) == 1 && is_sorted_gpu(&[1.0, 3.0, 2.0]) == 0,
		"is_sorted wrong"
	);

	// is_sorted_until: first index breaking order; n if sorted.
	let su = [1.0, 2.0, 5.0, 4.0, 9.0];
	pass!(
		"is_sorted_until",
		is_sorted_until_gpu(&su) == ora_is_sorted_until(&su)
			&& is_sorted_until_gpu(&[1.0, 2.0, 3.0]) == 3,
		format!(
			"got {} want {}",
			is_sorted_until_gpu(&su),
			ora_is_sorted_until(&su)
		)
	);

	// partition_point: first index where pred(x<pivot) is false, on partitioned data.
	let pp = [0.0, 1.0, 2.0, 3.0, 10.0, 11.0];
	pass!(
		"partition_point",
		partition_point_gpu(&pp, 5.0) == ora_partition_point(&pp, 5.0),
		format!(
			"got {} want {}",
			partition_point_gpu(&pp, 5.0),
			ora_partition_point(&pp, 5.0)
		)
	);

	// mismatch: first differing index.
	let ma = [1.0, 2.0, 3.0, 4.0];
	let mb = [1.0, 2.0, 9.0, 4.0];
	pass!(
		"mismatch",
		mismatch_gpu(&ma, &mb) == ora_mismatch(&ma, &mb) && mismatch_gpu(&ma, &ma) == 4,
		format!(
			"got {} want {}",
			mismatch_gpu(&ma, &mb),
			ora_mismatch(&ma, &mb)
		)
	);

	// argmax / max_element ; argmin / min_element : first-occurrence ties.
	let ax = [2.0, 5.0, 5.0, 1.0, 1.0, 4.0];
	pass!(
		"argmax",
		argextreme_gpu(&ax, 0) == ora_argmax(&ax),
		format!("got {} want {}", argextreme_gpu(&ax, 0), ora_argmax(&ax))
	);
	pass!(
		"argmin",
		argextreme_gpu(&ax, 1) == ora_argmin(&ax),
		format!("got {} want {}", argextreme_gpu(&ax, 1), ora_argmin(&ax))
	);

	// minmax_element: (argmin, argmax) in one pass.
	let (gmn, gmx) = minmax_element_gpu(&ax);
	pass!(
		"minmax_element",
		gmn == ora_argmin(&ax) && gmx == ora_argmax(&ax),
		format!(
			"got ({},{}) want ({},{})",
			gmn,
			gmx,
			ora_argmin(&ax),
			ora_argmax(&ax)
		)
	);

	// idamax / idamin: index of max/min ABSOLUTE value (BLAS) — distinct from argmax.
	let dx2 = [1.0, -7.0, 3.0, -2.0, 7.0, 0.5];
	pass!(
		"idamax",
		argextreme_gpu(&dx2, 2) == ora_idamax(&dx2),
		format!("got {} want {}", argextreme_gpu(&dx2, 2), ora_idamax(&dx2))
	);
	pass!(
		"idamin",
		argextreme_gpu(&dx2, 3) == ora_idamin(&dx2),
		format!("got {} want {}", argextreme_gpu(&dx2, 3), ora_idamin(&dx2))
	);
	// Proof that idamax != argmax here (|-7| ties |7| -> first; argmax picks the +7).
	if ora_idamax(&dx2) == ora_argmax(&dx2) {
		failures.push("idamax probe failed to distinguish from argmax".into());
	}

	// argrelmax / argrelmin / argrelextrema: strict local extrema mask, order=1.
	let rx = [0.0, 2.0, 1.0, 3.0, 0.0, -1.0, 2.0];
	pass!(
		"argrelmax",
		argrel_gpu(&rx, 0) == ora_argrel(&rx, 0),
		format!("got {:?} want {:?}", argrel_gpu(&rx, 0), ora_argrel(&rx, 0))
	);
	pass!(
		"argrelmin",
		argrel_gpu(&rx, 1) == ora_argrel(&rx, 1),
		format!("got {:?} want {:?}", argrel_gpu(&rx, 1), ora_argrel(&rx, 1))
	);
	pass!(
		"argrelextrema",
		argrel_gpu(&rx, 2) == ora_argrel(&rx, 2),
		format!("got {:?} want {:?}", argrel_gpu(&rx, 2), ora_argrel(&rx, 2))
	);

	ok
}

// Canonicalize a search-category JSON name to a registry key. Strip lib prefix,
// lowercase, take last segment; map TRUE synonyms only (e.g. max_element==argmax,
// find_if==find, rocblas_*amax==idamax). Host-only/structural items fall through
// to a non-registered key and remain backlog.
fn canon(name: &str) -> String {
	let mut base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	for suf in [
		"_2",
		"_dispatch",
		"_aten",
		"_inplace",
		"_strided_batched",
		"_batched",
	] {
		if let Some(s) = base.strip_suffix(suf) {
			base = s.to_string();
		}
	}
	// BLAS index-of-max/min-abs: i{d,s,c,z}amax / amin under rocblas_/hipblas prefixes.
	for pre in ["rocblas_", "hipblas"] {
		if let Some(s) = base.strip_prefix(pre) {
			base = s.to_string();
		}
	}
	if base.ends_with("amax") && base.starts_with('i') {
		return "idamax".to_string();
	}
	if base.ends_with("amin") && base.starts_with('i') {
		return "idamin".to_string();
	}
	let alias: &[(&str, &str)] = &[
		("searchsorted", "searchsorted"),
		("lower_bound", "lower_bound"),
		("upper_bound", "upper_bound"),
		// thrust/rocprim binary_search return a found-FLAG per needle (membership),
		// not an insertion index -> that contract is isin, not lower_bound.
		("binary_search", "isin"),
		("digitize", "digitize"),
		("bucketize", "bucketize"),
		("isin", "isin"),
		("contains", "isin"),
		("nonzero", "nonzero"),
		("argwhere", "nonzero"),
		("where", "nonzero"),
		("find", "find"),
		("find_if", "find"),
		("find_n", "find_n"),
		("is_sorted", "is_sorted"),
		("is_sorted_until", "is_sorted_until"),
		("partition_point", "partition_point"),
		("mismatch", "mismatch"),
		("argmax", "argmax"),
		("max_element", "argmax"),
		("miopenargmaxforward", "argmax"),
		("argmin", "argmin"),
		("min_element", "argmin"),
		("minmax_element", "minmax_element"),
		// (idamax/idamin BLAS family handled by the prefix/suffix rules above.)
		// signal extrema.
		("argrelmax", "argrelmax"),
		("argrelmin", "argrelmin"),
		("argrelextrema", "argrelextrema"),
		("find_peaks", "argrelmax"),
	];
	for (al, c) in alias {
		if base == *al {
			return c.to_string();
		}
	}
	base
}

fn load_search() -> Vec<String> {
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
					if k.get("category").and_then(|c| c.as_str()) != Some("search") {
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
fn prove_search() {
	let items = load_search();
	assert!(!items.is_empty(), "no search items in inventory");

	let mut failures: Vec<String> = Vec::new();
	let ok = prove_ops(&mut failures);
	let reg = registered();

	// Walk the inventory: an item is proven iff its canon maps to a passing op.
	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<String> = BTreeSet::new();
	let mut backlog: BTreeSet<String> = BTreeSet::new();
	for name in &items {
		let key = canon(name);
		if ok.contains(key.as_str()) {
			proven += 1;
			proven_keys.insert(key);
		} else {
			backlog.insert(key);
		}
	}

	let implemented: Vec<&str> = reg.iter().copied().collect();
	eprintln!("\n=== PROVE search ===");
	eprintln!("PROVE search: {} / {}", proven, total);
	eprintln!(
		"implemented ops ({}): {}",
		implemented.len(),
		implemented.join(", ")
	);
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);
	eprintln!(
		"backlog (host-only/structural, {}): {}",
		backlog.len(),
		backlog.iter().cloned().collect::<Vec<_>>().join(", ")
	);

	assert!(
		failures.is_empty(),
		"registered search op(s) FAILED oracle: {:#?}",
		failures
	);
	assert!(proven > 0, "zero search items proven");

	// Every registered op must have proven at least one inventory item (no dead registrations).
	for r in &reg {
		assert!(
			proven_keys.contains(*r),
			"registered op '{}' proved no inventory item; fix canon() or drop it",
			r
		);
	}
}
