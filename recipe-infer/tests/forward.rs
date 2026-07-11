use recipe_infer::{
	Activation, GpuBuffer, LayerKind, LayerParams, SCRATCH_CONSTS, Scratch, download_vec,
	forward_into, human_bytes,
};

static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn randn(n: usize, seed: usize) -> GpuBuffer {
	let b = GpuBuffer::alloc(n).expect("randn alloc");
	gpu_core::kernels::gpu_randn(n, seed, &b).expect("randn");
	b
}

fn consts_buf() -> GpuBuffer {
	let b = GpuBuffer::alloc(SCRATCH_CONSTS.len()).expect("consts");
	b.load(&SCRATCH_CONSTS).expect("consts");
	b
}

fn attn_layer(n: usize, heads: usize, d: usize, s: usize) -> (Vec<LayerParams>, GpuBuffer) {
	let in_dim = s * d;
	let params = vec![LayerParams {
		kind: LayerKind::Attn,
		w: randn(d * d, 1),
		b: {
			let __up = &vec![0.0f64; d];
			let __ub = GpuBuffer::alloc(__up.len()).expect("b");
			__ub.load(__up).expect("b");
			__ub
		},
		in_dim,
		out_dim: in_dim,
		act: Activation::Linear,
		dim: d,
		vocab: 0,
		wk: randn(d * d, 2),
		wv: randn(d * d, 3),
		wo: randn(d * d, 4),
		heads,
		palpha: {
			let __up = &[0.0f64];
			let __ub = GpuBuffer::alloc(__up.len()).expect("pa");
			__ub.load(__up).expect("pa");
			__ub
		},
		conv_cin: 0,
		conv_k: 0,
		conv_stride: 0,
	}];
	(params, randn(n * in_dim, 7))
}

#[test]
fn kv_cache_matches_full_attention() {
	let _g = GPU
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	gpu_core::hip::set_device(0).expect("set_device");
	let (n, heads, d, s) = (2usize, 4usize, 16usize, 1200usize);
	let in_dim = s * d;
	let (params, h) = attn_layer(n, heads, d, s);

	let consts = consts_buf();
	let sc_ref = Scratch::new_full(&params, n, &consts).expect("scratch");
	assert!(!sc_ref.infer, "ref must use the full-batch path");
	forward_into(&params, &h, None, n, &sc_ref.acts, &sc_ref).expect("forward");
	let reference = download_vec(&sc_ref.acts[0], n * in_dim);
	drop(sc_ref);

	let sc = Scratch::new_infer(&params, n, &consts).expect("scratch");
	assert!(sc.infer, "inference must use the KV-cache path");
	forward_into(&params, &h, None, n, &sc.acts, &sc).expect("forward");
	let cached = download_vec(&sc.acts[0], n * in_dim);

	let (mut maxdiff, mut maxabs) = (0.0f64, 0.0f64);
	for i in 0..reference.len() {
		maxdiff = maxdiff.max((reference[i] - cached[i]).abs());
		maxabs = maxabs.max(reference[i].abs());
	}
	eprintln!(
		"flash-attn equivalence: n={n} heads={heads} d={d} s={s}  maxdiff={maxdiff:e}  maxabs={maxabs:e}"
	);
	assert!(
		maxdiff <= 1e-9 * maxabs.max(1.0),
		"flash-attn output diverged from full attention: maxdiff={maxdiff:e}"
	);
}

#[test]
fn kv_cache_bounded_memory_long_sequence() {
	let _g = GPU
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	gpu_core::hip::set_device(0).expect("set_device");
	let (n, heads, d, s) = (2usize, 2usize, 16usize, 8192usize);
	let in_dim = s * d;
	let full_scores_bytes = n * heads * s * s * 8;
	let scratch_bytes = Scratch::vram_bytes(&attn_layer(n, heads, d, s).0, n, true);
	eprintln!(
		"flash-attn bounded: S={s}  full a_scores would be {}  whole inference Scratch is {}",
		human_bytes(full_scores_bytes),
		human_bytes(scratch_bytes),
	);
	assert!(
		full_scores_bytes > 1_000_000_000,
		"full buffer must be multi-GB to show the contrast"
	);
	assert!(
		scratch_bytes < full_scores_bytes / 10,
		"kernel-path memory must be a fraction of the L² buffer"
	);

	let (params, h) = attn_layer(n, heads, d, s);
	let consts = consts_buf();
	let sc = Scratch::new_infer(&params, n, &consts).expect("scratch");
	forward_into(&params, &h, None, n, &sc.acts, &sc).expect("forward");
	let _ = download_vec(&sc.acts[0], 1);
	let t0 = std::time::Instant::now();
	forward_into(&params, &h, None, n, &sc.acts, &sc).expect("forward");
	let out = download_vec(&sc.acts[0], n * in_dim);
	let ms = t0.elapsed().as_secs_f64() * 1e3;
	assert!(
		out.iter().all(|v| v.is_finite()),
		"flash-attn output not finite"
	);
	eprintln!(
		"flash-attn bounded: completed in {ms:.2} ms (S={s}, n={n}, heads={heads}, d={d}), out[0]={:.6}",
		out[0]
	);
}

#[test]
fn splitk_dw_matches_rocblas() {
	let _g = GPU
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	gpu_core::hip::set_device(0).expect("set_device");
	eprintln!(
		"split-K uses device multiProcessorCount = {} (queried, not hardcoded)",
		gpu_core::hip::cu_count()
	);
	for &(m, k, n) in &[
		(4096usize, 42usize, 64usize),
		(100_000, 42, 1),
		(777, 130, 96),
	] {
		let input = randn(m * k, 11);
		let grad = randn(m * n, 22);
		let reference = GpuBuffer::alloc(k * n).expect("ref dw");
		gpu_core::kernels::gpu_gemm_at(&input, &grad, k, n, m, &reference).expect("ref dw");
		let partials =
			GpuBuffer::alloc(gpu_core::kernels::gpu_splitk_dw_partials_elems(m, k, n))
				.expect("partials");
		let dw = GpuBuffer::alloc(k * n).expect("dw");
		gpu_core::kernels::gpu_splitk_dw_into(&input, &grad, &partials, m, n, k, &dw)
			.expect("splitk dw");
		let r = download_vec(&reference, k * n);
		let g = download_vec(&dw, k * n);
		let (mut maxdiff, mut maxabs) = (0.0f64, 0.0f64);
		for i in 0..r.len() {
			maxdiff = maxdiff.max((r[i] - g[i]).abs());
			maxabs = maxabs.max(r[i].abs());
		}
		eprintln!("split-K dW m={m} k={k} n={n}: maxdiff={maxdiff:e} maxabs={maxabs:e}");
		assert!(
			maxdiff <= 1e-8 * maxabs.max(1.0),
			"split-K dW diverged from rocBLAS: m={m} k={k} n={n} maxdiff={maxdiff:e}"
		);
	}
}
