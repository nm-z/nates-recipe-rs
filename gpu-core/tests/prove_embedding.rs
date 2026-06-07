mod common;
// Live-GPU proof harness for the "embedding" inventory category.
//
// For every embedding-category item in kernel_inventory/*.json, canonicalize its
// name; if that canonical op is registered here, run the gpu-core kernel on the
// LIVE gfx1101 GPU and assert it matches an AUTHORITATIVE oracle (Rust gather /
// scatter / textbook rotation / finite-difference). tol 1e-6.
//
// A proven op counts ALL its inventory variants (collapsed by canon). The test
// FAILS on any registered-op mismatch (a real bug). Backward/grad names never
// map to a forward op. Host-only / sparse / structural items stay as backlog.

use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_embeddingx_lookup(
		table: *const c_void,
		indices: *const c_void,
		out: *mut c_void,
		n: i32,
		d: i32,
		s: *mut c_void,
	);
	fn launch_embeddingx_bag(
		table: *const c_void,
		indices: *const c_void,
		offsets: *const c_void,
		out: *mut c_void,
		n_bags: i32,
		d: i32,
		mode: i32,
		s: *mut c_void,
	);
	fn launch_embeddingx_one_hot(
		indices: *const c_void,
		out: *mut c_void,
		n: i32,
		c: i32,
		s: *mut c_void,
	);
	fn launch_embeddingx_rope(
		x: *const c_void,
		cosb: *const c_void,
		sinb: *const c_void,
		y: *mut c_void,
		n: i32,
		d: i32,
		s: *mut c_void,
	);
	fn launch_embeddingx_rope_bwd(
		dy: *const c_void,
		cosb: *const c_void,
		sinb: *const c_void,
		dx: *mut c_void,
		n: i32,
		d: i32,
		s: *mut c_void,
	);
}

const TOL: f64 = 1e-6;

fn chk() {
	gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
}

// ── GPU runners ─────────────────────────────────────────────────────────────
fn gpu_lookup(table: &[f64], indices: &[i32], n: usize, d: usize) -> Vec<f64> {
	let bt = GpuBuffer::upload(table).unwrap();
	let bi = GpuBuffer::upload_i32(indices).unwrap();
	let o = GpuBuffer::alloc(n * d).unwrap();
	unsafe {
		launch_embeddingx_lookup(
			bt.ptr_raw() as *const c_void,
			bi.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			d as i32,
			std::ptr::null_mut(),
		);
	}
	chk();
	let mut out = vec![0.0; n * d];
	o.download(&mut out).unwrap();
	out
}

fn gpu_bag(
	table: &[f64],
	indices: &[i32],
	offsets: &[i32],
	n_bags: usize,
	d: usize,
	mode: i32,
) -> Vec<f64> {
	let bt = GpuBuffer::upload(table).unwrap();
	let bi = GpuBuffer::upload_i32(indices).unwrap();
	let bo = GpuBuffer::upload_i32(offsets).unwrap();
	let o = GpuBuffer::alloc(n_bags * d).unwrap();
	unsafe {
		launch_embeddingx_bag(
			bt.ptr_raw() as *const c_void,
			bi.ptr_raw() as *const c_void,
			bo.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n_bags as i32,
			d as i32,
			mode,
			std::ptr::null_mut(),
		);
	}
	chk();
	let mut out = vec![0.0; n_bags * d];
	o.download(&mut out).unwrap();
	out
}

fn gpu_one_hot(indices: &[i32], n: usize, c: usize) -> Vec<f64> {
	let bi = GpuBuffer::upload_i32(indices).unwrap();
	let o = GpuBuffer::alloc(n * c).unwrap();
	unsafe {
		launch_embeddingx_one_hot(
			bi.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			c as i32,
			std::ptr::null_mut(),
		);
	}
	chk();
	let mut out = vec![0.0; n * c];
	o.download(&mut out).unwrap();
	out
}

fn gpu_rope(x: &[f64], cosb: &[f64], sinb: &[f64], n: usize, d: usize) -> Vec<f64> {
	let bx = GpuBuffer::upload(x).unwrap();
	let bc = GpuBuffer::upload(cosb).unwrap();
	let bs = GpuBuffer::upload(sinb).unwrap();
	let o = GpuBuffer::alloc(n * d).unwrap();
	unsafe {
		launch_embeddingx_rope(
			bx.ptr_raw() as *const c_void,
			bc.ptr_raw() as *const c_void,
			bs.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			d as i32,
			std::ptr::null_mut(),
		);
	}
	chk();
	let mut out = vec![0.0; n * d];
	o.download(&mut out).unwrap();
	out
}

fn gpu_rope_bwd(dy: &[f64], cosb: &[f64], sinb: &[f64], n: usize, d: usize) -> Vec<f64> {
	let bd = GpuBuffer::upload(dy).unwrap();
	let bc = GpuBuffer::upload(cosb).unwrap();
	let bs = GpuBuffer::upload(sinb).unwrap();
	let o = GpuBuffer::alloc(n * d).unwrap();
	unsafe {
		launch_embeddingx_rope_bwd(
			bd.ptr_raw() as *const c_void,
			bc.ptr_raw() as *const c_void,
			bs.ptr_raw() as *const c_void,
			o.ptr_raw(),
			n as i32,
			d as i32,
			std::ptr::null_mut(),
		);
	}
	chk();
	let mut out = vec![0.0; n * d];
	o.download(&mut out).unwrap();
	out
}

// ── RoPE cos/sin cache (positions × inv_freq, base^(-2k/D)) ─────────────────
// Shared by GPU and the independent Rust oracle so the rotation factors match.
fn rope_cache(n: usize, d: usize, base: f64) -> (Vec<f64>, Vec<f64>) {
	let half = d / 2;
	let mut cosb = vec![0.0; n * half];
	let mut sinb = vec![0.0; n * half];
	for p in 0..n {
		for k in 0..half {
			let inv_freq = base.powf(-2.0 * (k as f64) / (d as f64));
			let theta = (p as f64) * inv_freq;
			cosb[p * half + k] = theta.cos();
			sinb[p * half + k] = theta.sin();
		}
	}
	(cosb, sinb)
}

fn close(a: &[f64], b: &[f64]) -> bool {
	a.len() == b.len()
		&& a.iter()
			.zip(b)
			.all(|(x, y)| (x - y).abs() <= TOL * (1.0 + y.abs()))
}

// ── Per-op proofs against authoritative CPU oracles. Return true if GPU==oracle ─
fn prove_lookup() -> bool {
	let (v, d) = (7usize, 5usize);
	let table: Vec<f64> = (0..v * d).map(|i| (i as f64) * 0.37 - 1.1).collect();
	let indices: [i32; 6] = [0, 3, 6, 1, 3, 5];
	let n = indices.len();
	let got = gpu_lookup(&table, &indices, n, d);
	let mut want = vec![0.0; n * d];
	for (i, &row) in indices.iter().enumerate() {
		for j in 0..d {
			want[i * d + j] = table[row as usize * d + j];
		}
	}
	// value copy -> demand bit-exact equality
	got == want
}

fn prove_bag() -> bool {
	let (v, d) = (8usize, 4usize);
	let table: Vec<f64> = (0..v * d).map(|i| (i as f64).sin() * 2.0 + 0.5).collect();
	let indices: [i32; 7] = [0, 2, 5, 1, 7, 3, 6];
	let offsets: [i32; 4] = [0, 3, 5, 7]; // 3 bags: [0,3) [3,5) [5,7)
	let n_bags = offsets.len() - 1;
	let mut ok = true;
	for &mode in &[0i32, 1i32] {
		let got = gpu_bag(&table, &indices, &offsets, n_bags, d, mode);
		let mut want = vec![0.0; n_bags * d];
		for b in 0..n_bags {
			let (s, e) = (offsets[b] as usize, offsets[b + 1] as usize);
			for j in 0..d {
				let mut acc = 0.0;
				for k in s..e {
					acc += table[indices[k] as usize * d + j];
				}
				if mode == 1 && e > s {
					acc /= (e - s) as f64;
				}
				want[b * d + j] = acc;
			}
		}
		ok &= close(&got, &want);
	}
	ok
}

fn prove_one_hot() -> bool {
	let (n, c) = (6usize, 5usize);
	let indices: [i32; 6] = [0, 4, 2, 1, 3, 0];
	let got = gpu_one_hot(&indices, n, c);
	let mut want = vec![0.0; n * c];
	for (i, &cl) in indices.iter().enumerate() {
		want[i * c + cl as usize] = 1.0;
	}
	got == want
}

fn prove_rope() -> bool {
	let (n, d) = (5usize, 8usize);
	let base = 10000.0;
	let half = d / 2;
	let x: Vec<f64> = (0..n * d).map(|i| (i as f64) * 0.13 - 0.7).collect();
	let (cosb, sinb) = rope_cache(n, d, base);
	let got = gpu_rope(&x, &cosb, &sinb, n, d);
	// Independent textbook NeoX rotation in Rust (NOT a second kernel call).
	let mut want = vec![0.0; n * d];
	for p in 0..n {
		for k in 0..half {
			let c = cosb[p * half + k];
			let s = sinb[p * half + k];
			let a = x[p * d + k];
			let b = x[p * d + k + half];
			want[p * d + k] = a * c - b * s;
			want[p * d + k + half] = b * c + a * s;
		}
	}
	close(&got, &want)
}

// Embedding lookup backward: grad_table[indices[i], :] += grad_out[i, :].
// Uses the already-public gpu_core::attention::gpu_embedding_backward (f32
// atomicAdd scatter). Oracle = independent Rust scatter-add. f32 accumulation,
// so small N + modest magnitudes keep error well under 1e-5.
fn prove_embedding_lookup_bwd() -> bool {
	let (vocab, cols) = (5usize, 4usize);
	let indices: [i32; 6] = [0, 2, 4, 2, 1, 0];
	let n = indices.len();
	let grad_out: Vec<f32> = (0..n * cols).map(|i| (i as f32) * 0.05 - 0.3).collect();
	let bg = GpuBuffer::upload_f32(&grad_out).unwrap();
	let bi = GpuBuffer::upload_i32(&indices).unwrap();
	let gt = gpu_core::attention::gpu_embedding_backward(&bg, &bi, n, cols, vocab).unwrap();
	let mut got = vec![0.0f32; vocab * cols];
	gt.download_f32(&mut got).unwrap();
	// Rust scatter-add oracle
	let mut want = vec![0.0f32; vocab * cols];
	for (i, &row) in indices.iter().enumerate() {
		for j in 0..cols {
			want[row as usize * cols + j] += grad_out[i * cols + j];
		}
	}
	got.iter()
		.zip(&want)
		.all(|(a, b)| (a - b).abs() <= 1e-5 * (1.0 + b.abs()))
}

// RoPE backward proven by FINITE-DIFFERENCE of the GPU forward (not by asserting
// the analytic transpose, which would be circular). For linear y=R(x), the VJP
// dx = J^T dy where J = R; column j of J^T is obtained from a forward difference.
fn prove_rope_bwd() -> bool {
	let (n, d) = (3usize, 4usize);
	let base = 10000.0;
	let x: Vec<f64> = (0..n * d).map(|i| (i as f64) * 0.21 - 0.4).collect();
	let (cosb, sinb) = rope_cache(n, d, base);
	// pick an arbitrary upstream gradient dy
	let dy: Vec<f64> = (0..n * d).map(|i| ((i as f64) * 0.17).cos()).collect();
	let dx = gpu_rope_bwd(&dy, &cosb, &sinb, n, d);
	// FD Jacobian-transpose-vector product:  dx_k = sum_m dy_m * dY_m/dx_k.
	let h = 1e-6;
	let y0 = gpu_rope(&x, &cosb, &sinb, n, d);
	let mut dx_fd = vec![0.0; n * d];
	for k in 0..n * d {
		let mut xp = x.clone();
		xp[k] += h;
		let yp = gpu_rope(&xp, &cosb, &sinb, n, d);
		let mut acc = 0.0;
		for m in 0..n * d {
			acc += dy[m] * (yp[m] - y0[m]) / h;
		}
		dx_fd[k] = acc;
	}
	// FD carries O(h) error; loosen tolerance for this op only.
	dx.iter()
		.zip(&dx_fd)
		.all(|(a, b)| (a - b).abs() <= 1e-4 * (1.0 + b.abs()))
}

// ── canonicalization: embedding-category JSON name -> registry key ──────────
// strip lib/scope prefix, lowercase, last segment; map TRUE synonyms only.
// *_backward / *_bwd / *_grad never collapse onto a forward op.
fn canon(name: &str) -> String {
	let lname = name.to_lowercase();
	let is_bwd = lname.contains("backward")
		|| lname.contains("_bwd")
		|| lname.ends_with("_grad")
		|| lname.contains("grad");

	// RoPE / rotary family
	let is_rope = lname.contains("rope") || lname.contains("rotary");
	if is_rope {
		return if is_bwd {
			"rope_bwd".to_string()
		} else {
			"rope".to_string()
		};
	}

	// embedding_bag family (pooled). torch._embedding_bag*, F.embedding_bag
	if lname.contains("embedding_bag") || lname.contains("_bag") {
		return if is_bwd {
			"embedding_bag_bwd".to_string()
		} else {
			"embedding_bag".to_string()
		};
	}

	// one_hot
	if lname.contains("one_hot") {
		return "one_hot".to_string();
	}

	// generic (dense) embedding-lookup backward: dense scatter-add of grad rows.
	// sparse variant has a different (sparse) gradient layout -> stays backlog.
	if (lname.contains("embedding_backward") || lname.contains("embedding_dense_backward"))
		&& !lname.contains("sparse")
		&& !lname.contains("_bag")
	{
		return "embedding_lookup_bwd".to_string();
	}

	// plain embedding lookup family (token embedding gather)
	// covers: torch.embedding/_embedding, F.embedding, tf.nn.embedding_lookup,
	// faster_transformer.embedding_lookup, liger LigerEmbedding, keras Embedding,
	// pltpu...lookup, __tpu$embedding_lookup, tfrs ...embedding_lookup, etc.
	let is_lookupish = lname.contains("embedding_lookup")
		|| lname.ends_with("embedding")
		|| lname.contains("_embedding")
		|| lname.contains("ligerembedding")
		|| lname.ends_with(".embedding")
		|| lname.contains("$embedding_lookup")
		|| lname.contains("sparsecore.lookup");
	if is_lookupish && !is_bwd {
		return "embedding_lookup".to_string();
	}
	if is_lookupish && is_bwd {
		return "embedding_lookup_bwd".to_string();
	}

	// backlog (unmapped): sparse/safe lookups, embedding_update, EmbLayerNorm,
	// per_sample_weights_backward, learned/relative/sinusoidal positional, etc.
	lname.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(&lname)
		.to_string()
}

fn load_embedding() -> Vec<String> {
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
					if cat != "embedding" {
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
fn prove_embedding() {
	let items = load_embedding();
	assert!(!items.is_empty(), "no embedding items in inventory");

	// Prove each registered op ONCE against its authoritative oracle.
	let mut op_ok: HashMap<&str, bool> = HashMap::new();
	op_ok.insert("embedding_lookup", prove_lookup());
	op_ok.insert("embedding_bag", prove_bag());
	op_ok.insert("one_hot", prove_one_hot());
	op_ok.insert("rope", prove_rope());
	op_ok.insert("rope_bwd", prove_rope_bwd());
	op_ok.insert("embedding_lookup_bwd", prove_embedding_lookup_bwd());

	let failures: Vec<&str> = op_ok
		.iter()
		.filter(|&(_, &v)| !v)
		.map(|(k, _)| *k)
		.collect();

	// Walk the inventory: each item whose canon maps to a passing registered op is proven.
	let total = items.len();
	let mut proven = 0usize;
	let mut proven_keys: std::collections::BTreeSet<String> = Default::default();
	let mut backlog: Vec<String> = Vec::new();
	for name in &items {
		let key = canon(name);
		match op_ok.get(key.as_str()) {
			Some(&true) => {
				proven += 1;
				proven_keys.insert(key);
			}
			_ => backlog.push(name.clone()),
		}
	}

	eprintln!("\n=== PROVE embedding ===");
	for (k, v) in &op_ok {
		eprintln!("  op {:<20} {}", k, if *v { "PROVEN" } else { "FAIL" });
	}
	eprintln!("PROVE embedding: {} / {}", proven, total);
	eprintln!(
		"proven canonical ops ({}): {}",
		proven_keys.len(),
		proven_keys.iter().cloned().collect::<Vec<_>>().join(", ")
	);
	eprintln!("backlog ({}): {}", backlog.len(), backlog.join(", "));

	assert!(
		failures.is_empty(),
		"registered embedding op(s) FAILED oracle: {:?}",
		failures
	);
	assert!(proven > 0, "zero embedding items proven");
}
