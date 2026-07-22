use crate::common;

use gpu_core::memory::GpuBuffer;
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;
use std::fs;
use std::ptr;

unsafe extern "C" {
	fn launch_indexingx_index_select(
		src: *const c_void,
		idx: *const c_void,
		out: *mut c_void,
		n: i32,
		cols: i32,
		s: *mut c_void,
	);
	fn launch_indexingx_where(
		cond: *const c_void,
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i32,
		s: *mut c_void,
	);
	fn launch_indexingx_index_add(
		out: *mut c_void,
		idx: *const c_void,
		src: *const c_void,
		n: i32,
		cols: i32,
		s: *mut c_void,
	);
	fn launch_indexingx_diagonal(m: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_indexingx_take_along_axis(
		src: *const c_void,
		idx: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		k: i32,
		s: *mut c_void,
	);
	fn launch_indexingx_tril(m: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn launch_indexingx_triu(m: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
	fn indexingx_masked_select_workspace_bytes(n: i32) -> usize;
	fn launch_indexingx_masked_select(
		in_: *const c_void,
		flags: *const c_void,
		out: *mut c_void,
		d_num: *mut c_void,
		d_temp: *mut c_void,
		temp_bytes: usize,
		n: i32,
		s: *mut c_void,
	);
}

fn last_err() {
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
}

// ── op runners (GPU) ─────────────────────────────────────────────────────────

fn gpu_index_select(src: &[f64], idx: &[i32], cols: usize) -> Vec<f64> {
	let n = idx.len();
	let bs = {
		let __up = src;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bi = GpuBuffer::upload_i32(idx).unwrap();
	let o = GpuBuffer::alloc(n * cols).unwrap();
	unsafe {
		launch_indexingx_index_select(
			bs.ptr_raw() as *const c_void,
			bi.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			cols as i32,
			ptr::null_mut(),
		);
	}
	last_err();
	let mut out = vec![0.0; n * cols];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn gpu_where(cond: &[f64], a: &[f64], b: &[f64]) -> Vec<f64> {
	let n = cond.len();
	let bc = {
		let __up = cond;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
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
	let o = GpuBuffer::alloc(n).unwrap();
	unsafe {
		launch_indexingx_where(
			bc.ptr_raw() as *const c_void,
			ba.ptr_raw() as *const c_void,
			bb.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			ptr::null_mut(),
		);
	}
	last_err();
	let mut out = vec![0.0; n];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn gpu_index_add(base: &[f64], idx: &[i32], src: &[f64], cols: usize) -> Vec<f64> {
	let rows = base.len() / cols;
	let n = idx.len();
	let bo = {
		let __up = base;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	}; // start from base
	let bi = GpuBuffer::upload_i32(idx).unwrap();
	let bs = {
		let __up = src;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	unsafe {
		launch_indexingx_index_add(
			bo.ptr_raw(),
			bi.ptr_raw() as *const c_void,
			bs.ptr_raw() as *const c_void,
			n as i32,
			cols as i32,
			ptr::null_mut(),
		);
	}
	last_err();
	let mut out = vec![0.0; rows * cols];
	unsafe { bo.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn gpu_diagonal(m: &[f64], n: usize) -> Vec<f64> {
	let bm = {
		let __up = m;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(n).unwrap();
	unsafe {
		launch_indexingx_diagonal(
			bm.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			ptr::null_mut(),
		);
	}
	last_err();
	let mut out = vec![0.0; n];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn gpu_take_along(src: &[f64], idx: &[i32], rows: usize, cols: usize, k: usize) -> Vec<f64> {
	let bs = {
		let __up = src;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bi = GpuBuffer::upload_i32(idx).unwrap();
	let o = GpuBuffer::alloc(rows * k).unwrap();
	unsafe {
		launch_indexingx_take_along_axis(
			bs.ptr_raw() as *const c_void,
			bi.ptr_raw() as *const c_void,
			o.ptr_raw(),
			rows as i32,
			cols as i32,
			k as i32,
			ptr::null_mut(),
		);
	}
	last_err();
	let mut out = vec![0.0; rows * k];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn gpu_tri(m: &[f64], n: usize, upper: bool) -> Vec<f64> {
	let bm = {
		let __up = m;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(n * n).unwrap();
	unsafe {
		if upper {
			launch_indexingx_triu(
				bm.ptr_raw() as *const c_void,
				o.ptr_raw(),
				n as i32,
				ptr::null_mut(),
			);
		} else {
			launch_indexingx_tril(
				bm.ptr_raw() as *const c_void,
				o.ptr_raw(),
				n as i32,
				ptr::null_mut(),
			);
		}
	}
	last_err();
	let mut out = vec![0.0; n * n];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn gpu_masked_select(input: &[f64], flags: &[u8]) -> (usize, Vec<f64>) {
	let n = input.len();
	let bi = {
		let __up = input;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bf = {
		let __u = flags;
		let __b = GpuBuffer::alloc_bytes(__u.len()).unwrap();
		__b.write_u8(__u).unwrap();
		__b
	};
	let o = GpuBuffer::alloc(n).unwrap();
	let bn = GpuBuffer::alloc_bytes(4).unwrap();
	let temp_bytes = unsafe { indexingx_masked_select_workspace_bytes(n as i32) };
	let d_temp = GpuBuffer::alloc_bytes(temp_bytes.max(1)).unwrap();
	unsafe {
		launch_indexingx_masked_select(
			bi.ptr_raw() as *const c_void,
			bf.ptr_raw() as *const c_void,
			o.ptr_raw(),
			bn.ptr_raw(),
			d_temp.ptr_raw(),
			temp_bytes,
			n as i32,
			ptr::null_mut(),
		);
	}
	last_err();
	let mut num = [0i32; 1];
	bn.download_i32(&mut num).unwrap();
	let num = num[0] as usize;
	let mut out = vec![0.0; n];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	(num, out[..num].to_vec())
}

const TOL: f64 = 1e-7;
fn close(a: &[f64], b: &[f64]) -> bool {
	a.len() == b.len()
		&& a.iter()
			.zip(b)
			.all(|(x, y)| (x - y).abs() <= TOL * (1.0 + y.abs()))
}

// ── per-op proofs against authoritative CPU oracle ───────────────────────────

fn prove_index_select() -> bool {
	let cols = 3usize;
	let src: Vec<f64> = (0..12).map(|v| v as f64 * 0.5 - 1.3).collect();
	let idx = [2i32, 0, 3, 1, 0];
	let got = gpu_index_select(&src, &idx, cols);
	let mut want = Vec::new();
	for &r in &idx {
		for j in 0..cols {
			want.push(src[r as usize * cols + j]);
		}
	}
	let row_ok = close(&got, &want);
	let v: Vec<f64> = (0..8).map(|i| (i as f64).sin()).collect();
	let ti = [7i32, 0, 3, 3, 5];
	let g2 = gpu_index_select(&v, &ti, 1);
	let w2: Vec<f64> = ti.iter().map(|&i| v[i as usize]).collect();
	row_ok && close(&g2, &w2)
}

fn prove_where() -> bool {
	let cond = [1.0, 0.0, -2.0, 0.0, 3.5]; // nonzero => take a
	let a = [10.0, 11.0, 12.0, 13.0, 14.0];
	let b = [-1.0, -2.0, -3.0, -4.0, -5.0];
	let got = gpu_where(&cond, &a, &b);
	let want: Vec<f64> = (0..5)
		.map(|i| if cond[i] != 0.0 { a[i] } else { b[i] })
		.collect();
	close(&got, &want)
}

fn prove_index_add() -> bool {
	let cols = 2usize;
	let base: Vec<f64> = (0..8).map(|v| v as f64).collect();
	let idx = [0i32, 2, 0, 3];
	let src: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
	let got = gpu_index_add(&base, &idx, &src, cols);
	let mut want = base.clone();
	for (i, &r) in idx.iter().enumerate() {
		for j in 0..cols {
			want[r as usize * cols + j] += src[i * cols + j];
		}
	}
	close(&got, &want)
}

fn prove_diagonal() -> bool {
	let n = 5usize;
	let m: Vec<f64> = (0..n * n).map(|v| v as f64 * 0.3 - 2.0).collect();
	let got = gpu_diagonal(&m, n);
	let want: Vec<f64> = (0..n).map(|i| m[i * n + i]).collect();
	close(&got, &want)
}

fn prove_take_along() -> bool {
	let (rows, cols, k) = (3usize, 4usize, 2usize);
	let src: Vec<f64> = (0..rows * cols).map(|v| (v as f64).cos()).collect();
	let idx = [3i32, 0, 1, 2, 0, 3]; // row-major [3,2]
	let got = gpu_take_along(&src, &idx, rows, cols, k);
	let mut want = Vec::new();
	for i in 0..rows {
		for j in 0..k {
			want.push(src[i * cols + idx[i * k + j] as usize]);
		}
	}
	close(&got, &want)
}

fn prove_tri() -> bool {
	let n = 4usize;
	let m: Vec<f64> = (1..=n * n).map(|v| v as f64).collect();
	let gl = gpu_tri(&m, n, false);
	let gu = gpu_tri(&m, n, true);
	let mut wl = vec![0.0; n * n];
	let mut wu = vec![0.0; n * n];
	for i in 0..n {
		for j in 0..n {
			let t = i * n + j;
			if i >= j {
				wl[t] = m[t];
			}
			if i <= j {
				wu[t] = m[t];
			}
		}
	}
	close(&gl, &wl) && close(&gu, &wu)
}

fn prove_masked_select() -> bool {
	let input: Vec<f64> = vec![1.0, -2.0, 3.0, 4.0, -5.0, 6.0, 7.0, -8.0];
	let flags: Vec<u8> = vec![1, 0, 1, 0, 0, 1, 1, 0]; // keep where flag != 0
	let (num, out) = gpu_masked_select(&input, &flags);
	let want: Vec<f64> = input
		.iter()
		.zip(&flags)
		.filter(|&(_, &f)| f != 0)
		.map(|(&v, _)| v)
		.collect();
	num == want.len() && close(&out, &want)
}

// ── canonicalization: inventory name -> registered op key (or backlog) ───────
fn canon(name: &str) -> Option<&'static str> {
	let n = name.to_lowercase();
	let seg = n.rsplit(['.', ':', '$']).next().unwrap_or(&n).to_string();

	if seg.contains("gathernd") || seg.contains("gather_nd") {
		return None;
	}
	if seg.contains("scatter_") || seg.ends_with("_scatter") || seg.contains("scatternd")
            || seg.contains("indices")                       // tril_indices/triu_indices
            || seg.contains("diag_embed") || seg.contains("diagflat")
            || seg.contains("matrix_diag") || seg.contains("fill_diagonal")
	{
		return None;
	}

	if seg.contains("masked_select") || seg.contains("apply_boolean_mask") {
		return Some("masked_select");
	}

	if seg.contains("take_along")
		|| seg == "torch.gather"
		|| n == "torch.gather"
		|| seg == "gather" && (n.starts_with("torch.") || n == "tl.gather")
	{
		return Some("take_along_axis");
	}

	if seg.contains("index_select") || seg.contains("torch_index_select") {
		return Some("index_select");
	}
	if (seg == "gather" || seg == "gatherv2" || seg.ends_with("gather"))
		&& !seg.contains("along")
		&& !seg.contains("allgather")
		&& !seg.contains("nd")
	{
		return Some("index_select");
	}
	if seg == "take" || seg == "resourcegather" {
		return Some("index_select");
	}

	if seg == "index_add" || seg == "scatteradd" || seg == "resourcescatteradd" {
		return Some("index_add");
	}

	if seg == "diagonal" {
		return Some("diagonal");
	}

	if seg == "tril" || seg == "triu" {
		return Some("tril");
	} // both prove same kernel pair

	if seg.starts_with("where") {
		return Some("where");
	}
	if seg == "select" || seg == "selectv2" || seg == "select_v2" || seg == "stablehlo_select" {
		return Some("where");
	}

	None
}

fn load_indexing() -> Vec<String> {
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
					if cat != "indexing" {
						continue;
					}
					let name = k
						.get("name")
						.and_then(|nm| nm.as_str())
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
fn prove_indexing() {
	let items = load_indexing();
	assert!(!items.is_empty(), "no indexing items in inventory");

	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	op_ok.insert("index_select", prove_index_select());
	op_ok.insert("where", prove_where());
	op_ok.insert("index_add", prove_index_add());
	op_ok.insert("diagonal", prove_diagonal());
	op_ok.insert("take_along_axis", prove_take_along());
	op_ok.insert("tril", prove_tri());
	op_ok.insert("masked_select", prove_masked_select());

	let failures: Vec<&str> = op_ok
		.iter()
		.filter(|&(_, &ok)| !ok)
		.map(|(k, _)| *k)
		.collect();

	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: BTreeSet<&str> = Default::default();
	for name in &items {
		if let Some(key) = canon(name)
			&& *op_ok.get(key).unwrap_or(&false)
		{
			proven += 1;
			proven_keys.insert(key);
		}
	}

	eprintln!("\n=== PROVE indexing ===");
	eprintln!("PROVE indexing: {} / {}", proven, total);
	let mut impls: Vec<&str> = op_ok.keys().copied().collect();
	impls.sort();
	eprintln!("registered ops ({}): {}", impls.len(), impls.join(", "));
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().copied().collect::<Vec<_>>().join(", ")
	);

	assert!(
		failures.is_empty(),
		"registered indexing op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero indexing items proven");
}
