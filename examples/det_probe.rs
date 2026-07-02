// Determinism probe: every GPU op the diffusion forward composes, run twice on
// identical inputs, bit-compared. A DIVERGED op is the source of the observed
// run-to-run output variance (atomic f64 accumulation or a stream race).
//
//   cargo run --release --example det_probe

use gpu_core::infer_ops::{
	gpu_gelu_mul_into, gpu_glu_gelu_into, gpu_gqa_attn_into, gpu_rmsnorm_f64_into,
	gpu_rope_partial, gpu_widen_bf16_into,
};
use gpu_core::kernels::{gpu_add_into, gpu_gemm_bt_into, gpu_scale_inplace};
use gpu_core::memory::GpuBuffer;

const T: usize = 54;
const NE: usize = 2816;

fn xsf(st: &mut u64) -> f64 {
	*st ^= *st << 13;
	*st ^= *st >> 7;
	*st ^= *st << 17;
	(*st >> 11) as f64 / (1u64 << 53) as f64 - 0.5
}

fn host(n: usize, seed: u64) -> Vec<f64> {
	let mut st = seed | 1;
	(0..n).map(|_| xsf(&mut st)).collect()
}

fn bithash(a: &[f64]) -> u64 {
	a.iter().fold(0xcbf29ce484222325u64, |h, v| {
		(h ^ v.to_bits()).wrapping_mul(0x100000001b3)
	})
}

fn cmp(name: &str, a: &[f64], b: &[f64]) {
	let diff = a.iter().zip(b).filter(|(x, y)| x.to_bits() != y.to_bits()).count();
	if diff == 0 {
		eprintln!("{name:<26} DETERMINISTIC ({} elems)  hash={:016x}", a.len(), bithash(a));
	} else {
		let mx = a
			.iter()
			.zip(b)
			.map(|(x, y)| (x - y).abs())
			.fold(0.0f64, f64::max);
		eprintln!("{name:<26} DIVERGED: {diff}/{} elems differ, max abs {mx:e}", a.len());
	}
}

fn twice(name: &str, n_out: usize, mut f: impl FnMut(&GpuBuffer)) {
	let out = GpuBuffer::alloc(n_out).expect("alloc out");
	let mut r1 = vec![0.0f64; n_out];
	let mut r2 = vec![0.0f64; n_out];
	f(&out);
	out.download(&mut r1).expect("dl1");
	f(&out);
	out.download(&mut r2).expect("dl2");
	cmp(name, &r1, &r2);
}

fn main() {
	recipe_infer::init().expect("gpu init");
	gpu_core::memory::set_alloc_sync(true);

	let x = GpuBuffer::upload(&host(T * NE, 7)).expect("x");
	let g = GpuBuffer::upload(&host(NE, 11)).expect("g");
	let w = GpuBuffer::upload(&host(8192 * NE, 13)).expect("w");

	twice("rmsnorm", T * NE, |o| gpu_rmsnorm_f64_into(&x, Some(&g), o, T, NE, 1e-6));
	twice("gemm_bt 54x8192x2816", T * 8192, |o| {
		gpu_gemm_bt_into(&x, &w, o, T, 8192, NE).expect("gemm")
	});
	for np in [1usize, 2, 3, 5, 8] {
		twice(&format!("gemm_bt {np}x1408x2816"), np * 1408, |o| {
			gpu_gemm_bt_into(&x, &w, o, np, 1408, NE).expect("gemm small")
		});
	}

	// GQA attention, both gemma4 geometries (q/k/v shaped like the real layer).
	for (hd, nkv, rotary, theta, label) in [
		(256usize, 8usize, 256usize, 1e4, "gqa sliding hd256"),
		(512, 2, 128, 1e6, "gqa full hd512"),
	] {
		let q0 = host(T * 16 * hd, 17);
		let k0 = host(T * nkv * hd, 19);
		let v0 = host(T * nkv * hd, 23);
		let q = GpuBuffer::upload(&q0).expect("q");
		let k = GpuBuffer::upload(&k0).expect("k");
		let v = GpuBuffer::upload(&v0).expect("v");
		gpu_rope_partial(&q, T * 16, hd, rotary, 16, theta);
		gpu_rope_partial(&k, T * nkv, hd, rotary, nkv, theta);
		twice(label, T * 16 * hd, |o| gpu_gqa_attn_into(&q, &k, &v, o, T, 16, nkv, hd, 6));

		// rope itself: re-upload, rotate, download, twice.
		let mut a = vec![0.0f64; T * 16 * hd];
		let mut b = vec![0.0f64; T * 16 * hd];
		q.load(&q0).expect("reload q");
		gpu_rope_partial(&q, T * 16, hd, rotary, 16, theta);
		q.download(&mut a).expect("dl");
		q.load(&q0).expect("reload q");
		gpu_rope_partial(&q, T * 16, hd, rotary, 16, theta);
		q.download(&mut b).expect("dl");
		cmp(&format!("rope {label}"), &a, &b);
	}

	let a2 = GpuBuffer::upload(&host(T * 2112, 29)).expect("a2");
	let b2 = GpuBuffer::upload(&host(T * 2112, 31)).expect("b2");
	twice("gelu_mul", T * 2112, |o| gpu_gelu_mul_into(&a2, &b2, o, T * 2112));
	let gu = GpuBuffer::upload(&host(T * 2 * 704, 37)).expect("gu");
	twice("glu_gelu", T * 704, |o| gpu_glu_gelu_into(&gu, o, T, 704));
	twice("add", T * NE, |o| gpu_add_into(&x, &x, o, T * NE));

	let bf: Vec<u8> = host(T * NE, 41)
		.iter()
		.map(|v| ((*v as f32).to_bits() >> 16) as u16)
		.flat_map(|h| h.to_le_bytes())
		.collect();
	let stage = GpuBuffer::alloc_bytes(bf.len()).expect("stage");
	stage.write_u8(&bf).expect("write");
	twice("widen_bf16", T * NE, |o| gpu_widen_bf16_into(&stage, o, T * NE));

	// scale_inplace: in-place, so reload between runs.
	let mut s1 = vec![0.0f64; T * NE];
	let mut s2 = vec![0.0f64; T * NE];
	let x0 = host(T * NE, 43);
	x.load(&x0).expect("reload");
	gpu_scale_inplace(&x, 0.735, T * NE);
	x.download(&mut s1).expect("dl");
	x.load(&x0).expect("reload");
	gpu_scale_inplace(&x, 0.735, T * NE);
	x.download(&mut s2).expect("dl");
	cmp("scale_inplace", &s1, &s2);

	recipe_infer::shutdown();
}
