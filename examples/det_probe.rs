#![allow(unsafe_code)]
// Determinism probe: every GPU op the diffusion forward composes, run twice on
// identical inputs, bit-compared. A DIVERGED op is the source of the observed
// run-to-run output variance (atomic f64 accumulation or a stream race).
//
//   cargo run --release --example det_probe

use anyhow::Context;
use gpu_core::infer_ops::{
	gpu_gelu_mul, gpu_gemm_bt_f64, gpu_glu_gelu, gpu_gqa_attn, gpu_rmsnorm_f64, gpu_rope_partial,
	gpu_scale_f64_inplace, gpu_widen_bf16,
};
use gpu_core::kernels::{gpu_add_into, gpu_gemm_bt_into, gpu_scale_inplace};
use gpu_core::log::{Opt, Write, opt, probe, set_opt};
use gpu_core::memory::GpuBuffer;
use ogdl::ogdl;
use std::fmt;
use std::ptr;
use std::time::Instant;

fn say(t: impl fmt::Display) {
	set_opt(Opt {
		probe: true,
		..opt()
	});
	Write::block(probe, ogdl!(&t));
}

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
	let diff = a
		.iter()
		.zip(b)
		.filter(|(x, y)| x.to_bits() != y.to_bits())
		.count();
	if diff == 0 {
		say(format!(
			"{name:<26} DETERMINISTIC ({} elems)  hash={:016x}",
			a.len(),
			bithash(a)
		));
	} else {
		let mx = a
			.iter()
			.zip(b)
			.map(|(x, y)| (x - y).abs())
			.fold(0.0f64, f64::max);
		say(format!(
			"{name:<26} DIVERGED: {diff}/{} elems differ, max abs {mx:e}",
			a.len()
		));
	}
}

// Plain-Rust CPU oracle for out(m,n) = a(m,k) . b(n,k)^T, row-major.
fn cpu_gemm_bt(a: &[f64], b: &[f64], m: usize, n: usize, k: usize) -> Vec<f64> {
	let mut out = vec![0.0f64; m * n];
	for i in 0..m {
		let arow = &a[i * k..(i + 1) * k];
		for j in 0..n {
			let brow = &b[j * k..(j + 1) * k];
			out[i * n + j] = arow.iter().zip(brow).map(|(x, y)| x * y).sum();
		}
	}
	out
}

// Custom gemm_bt_f64 vs CPU oracle (accuracy) and vs rocBLAS gpu_gemm_bt_into
// (cross-check), plus bit-identity across two runs.
fn check_gemm(label: &str, m: usize, n: usize, k: usize) -> anyhow::Result<()> {
	let av = host(m * k, 1000 + m as u64);
	let bv = host(n * k, 2000 + n as u64 + k as u64);
	let a = {
		let __up = &av;
		let __ub = GpuBuffer::alloc(__up.len()).context("a")?;
		__ub.load(__up).context("a")?;
		__ub
	};
	let b = {
		let __up = &bv;
		let __ub = GpuBuffer::alloc(__up.len()).context("b")?;
		__ub.load(__up).context("b")?;
		__ub
	};
	let cpu = cpu_gemm_bt(&av, &bv, m, n, k);

	let out_c = GpuBuffer::alloc(m * n).context("out_c")?;
	gpu_gemm_bt_f64(&a, &b, m, n, k, &out_c).context("gemm_bt_f64")?;
	let mut gc = vec![0.0f64; m * n];
	unsafe { out_c.download_async(&mut gc, ptr::null_mut()) }.context("dl gc")?;
	gpu_core::hip::device_synchronize().context("dl gc")?;
	let err_cpu = gc
		.iter()
		.zip(&cpu)
		.map(|(x, y)| (x - y).abs())
		.fold(0.0f64, f64::max);

	let out_r = GpuBuffer::alloc(m * n).context("out_r")?;
	gpu_gemm_bt_into(&a, &b, m, n, k, &out_r).context("rocblas gemm")?;
	let mut gr = vec![0.0f64; m * n];
	unsafe { out_r.download_async(&mut gr, ptr::null_mut()) }.context("dl gr")?;
	gpu_core::hip::device_synchronize().context("dl gr")?;
	let err_roc = gc
		.iter()
		.zip(&gr)
		.map(|(x, y)| (x - y).abs())
		.fold(0.0f64, f64::max);

	let pass = if err_cpu < 1e-9 { "PASS" } else { "FAIL" };
	say(format!(
		"{label:<24} m={m:<3} n={n:<5} k={k:<5} max_err_cpu={err_cpu:e} max_err_rocblas={err_roc:e} {pass}"
	));

	twice(&format!("gemm_bt_f64 {label}"), m * n, |o| {
		gpu_gemm_bt_f64(&a, &b, m, n, k, o).context("gemm_bt_f64")
	})?;
	Ok(())
}

// Time ITERS iterations of rocBLAS gpu_gemm_bt_into vs the custom kernel for
// one (m,n,k) shape, report ms + effective GB/s (B read once, the dominant
// bandwidth term) against the 432GB/s gfx1101 floor.
fn bench_shape(label: &str, m: usize, n: usize, k: usize) -> anyhow::Result<()> {
	let a = {
		let __up = &host(m * k, 3000);
		let __ub = GpuBuffer::alloc(__up.len()).context("a")?;
		__ub.load(__up).context("a")?;
		__ub
	};
	let b = {
		let __up = &host(n * k, 4000);
		let __ub = GpuBuffer::alloc(__up.len()).context("b")?;
		__ub.load(__up).context("b")?;
		__ub
	};
	let out = GpuBuffer::alloc(m * n).context("out")?;
	const ITERS: usize = 50;

	gpu_core::hip::device_synchronize().context("sync0")?;
	let t0 = Instant::now();
	for _ in 0..ITERS {
		gpu_gemm_bt_into(&a, &b, m, n, k, &out).context("rocblas")?;
	}
	gpu_core::hip::device_synchronize().context("sync1")?;
	let roc_ms = t0.elapsed().as_secs_f64() * 1000.0 / ITERS as f64;

	let t1 = Instant::now();
	for _ in 0..ITERS {
		gpu_gemm_bt_f64(&a, &b, m, n, k, &out).context("gemm_bt_f64")?;
	}
	gpu_core::hip::device_synchronize().context("sync2")?;
	let cus_ms = t1.elapsed().as_secs_f64() * 1000.0 / ITERS as f64;

	let b_bytes = (n * k * size_of::<f64>()) as f64;
	let floor_ms = b_bytes / 432e9 * 1000.0;
	let roc_gbs = b_bytes / (roc_ms / 1000.0) / 1e9;
	let cus_gbs = b_bytes / (cus_ms / 1000.0) / 1e9;
	say(format!(
		"{label:<24} m={m:<3} n={n:<5} k={k:<5} rocBLAS={roc_ms:8.4}ms ({roc_gbs:7.1} GB/s)  custom={cus_ms:8.4}ms ({cus_gbs:7.1} GB/s)  floor={floor_ms:7.4}ms  ratio={:.2}x",
		cus_ms / roc_ms
	));
	Ok(())
}

fn twice(
	name: &str,
	n_out: usize,
	mut f: impl FnMut(&GpuBuffer) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
	let out = GpuBuffer::alloc(n_out).context("alloc out")?;
	let mut r1 = vec![0.0f64; n_out];
	let mut r2 = vec![0.0f64; n_out];
	f(&out)?;
	unsafe { out.download_async(&mut r1, ptr::null_mut()) }.context("dl1")?;
	gpu_core::hip::device_synchronize().context("dl1")?;
	f(&out)?;
	unsafe { out.download_async(&mut r2, ptr::null_mut()) }.context("dl2")?;
	gpu_core::hip::device_synchronize().context("dl2")?;
	cmp(name, &r1, &r2);
	Ok(())
}

fn main() -> anyhow::Result<()> {
	recipe_infer::init().context("gpu init")?;

	let x = {
		let __up = &host(T * NE, 7);
		let __ub = GpuBuffer::alloc(__up.len()).context("x")?;
		__ub.load(__up).context("x")?;
		__ub
	};
	let g = {
		let __up = &host(NE, 11);
		let __ub = GpuBuffer::alloc(__up.len()).context("g")?;
		__ub.load(__up).context("g")?;
		__ub
	};
	let w = {
		let __up = &host(8192 * NE, 13);
		let __ub = GpuBuffer::alloc(__up.len()).context("w")?;
		__ub.load(__up).context("w")?;
		__ub
	};
	let eps = {
		let __up = &[1e-6f64];
		let __ub = GpuBuffer::alloc(__up.len()).context("eps")?;
		__ub.load(__up).context("eps")?;
		__ub
	};
	let scale = {
		let __up = &[0.735f64];
		let __ub = GpuBuffer::alloc(__up.len()).context("scale")?;
		__ub.load(__up).context("scale")?;
		__ub
	};

	twice("rmsnorm", T * NE, |o| {
		gpu_rmsnorm_f64(&x, &g, &eps, T, NE, o).context("rmsnorm")
	})?;
	twice("gemm_bt 54x8192x2816", T * 8192, |o| {
		gpu_gemm_bt_into(&x, &w, T, 8192, NE, o).context("gemm")
	})?;
	for np in [1usize, 2, 3, 5, 8] {
		twice(&format!("gemm_bt {np}x1408x2816"), np * 1408, |o| {
			gpu_gemm_bt_into(&x, &w, np, 1408, NE, o).context("gemm small")
		})?;
	}

	// GQA attention, both gemma4 geometries (q/k/v shaped like the real layer).
	for (hd, nkv, rotary, theta, label) in [
		(256usize, 8usize, 256usize, 1e4, "gqa sliding hd256"),
		(512, 2, 128, 1e6, "gqa full hd512"),
	] {
		let q0 = host(T * 16 * hd, 17);
		let k0 = host(T * nkv * hd, 19);
		let v0 = host(T * nkv * hd, 23);
		let q = {
			let __up = &q0;
			let __ub = GpuBuffer::alloc(__up.len()).context("q")?;
			__ub.load(__up).context("q")?;
			__ub
		};
		let k = {
			let __up = &k0;
			let __ub = GpuBuffer::alloc(__up.len()).context("k")?;
			__ub.load(__up).context("k")?;
			__ub
		};
		let v = {
			let __up = &v0;
			let __ub = GpuBuffer::alloc(__up.len()).context("v")?;
			__ub.load(__up).context("v")?;
			__ub
		};
		let theta_buf = {
			let __up = &[theta];
			let __ub = GpuBuffer::alloc(__up.len()).context("theta")?;
			__ub.load(__up).context("theta")?;
			__ub
		};
		gpu_rope_partial(&theta_buf, T * 16, hd, rotary, 16, &q).context("rope q")?;
		gpu_rope_partial(&theta_buf, T * nkv, hd, rotary, nkv, &k).context("rope k")?;
		twice(label, T * 16 * hd, |o| {
			gpu_gqa_attn(&q, &k, &v, T, 16, nkv, hd, 6, 0.0, o).context("gqa")
		})?;

		// rope itself: re-upload, rotate, download, twice.
		let mut a = vec![0.0f64; T * 16 * hd];
		let mut b = vec![0.0f64; T * 16 * hd];
		q.load(&q0).context("reload q")?;
		gpu_rope_partial(&theta_buf, T * 16, hd, rotary, 16, &q).context("rope q")?;
		unsafe { q.download_async(&mut a, ptr::null_mut()) }.context("dl")?;
		gpu_core::hip::device_synchronize().context("dl")?;
		q.load(&q0).context("reload q")?;
		gpu_rope_partial(&theta_buf, T * 16, hd, rotary, 16, &q).context("rope q")?;
		unsafe { q.download_async(&mut b, ptr::null_mut()) }.context("dl")?;
		gpu_core::hip::device_synchronize().context("dl")?;
		cmp(&format!("rope {label}"), &a, &b);
	}

	let a2 = {
		let __up = &host(T * 2112, 29);
		let __ub = GpuBuffer::alloc(__up.len()).context("a2")?;
		__ub.load(__up).context("a2")?;
		__ub
	};
	let b2 = {
		let __up = &host(T * 2112, 31);
		let __ub = GpuBuffer::alloc(__up.len()).context("b2")?;
		__ub.load(__up).context("b2")?;
		__ub
	};
	twice("gelu_mul", T * 2112, |o| {
		gpu_gelu_mul(&a2, &b2, T * 2112, o).context("gelu_mul")
	})?;
	let gu = {
		let __up = &host(T * 2 * 704, 37);
		let __ub = GpuBuffer::alloc(__up.len()).context("gu")?;
		__ub.load(__up).context("gu")?;
		__ub
	};
	twice("glu_gelu", T * 704, |o| {
		gpu_glu_gelu(&gu, T, 704, o).context("glu_gelu")
	})?;
	twice("add", T * NE, |o| {
		gpu_add_into(&x, &x, T * NE, o).context("add")
	})?;

	let bf: Vec<u8> = host(T * NE, 41)
		.iter()
		.map(|v| ((*v as f32).to_bits() >> 16) as u16)
		.flat_map(|h| h.to_le_bytes())
		.collect();
	let stage = GpuBuffer::alloc_bytes(bf.len()).context("stage")?;
	stage.write_u8(&bf).context("write")?;
	twice("widen_bf16", T * NE, |o| {
		gpu_widen_bf16(&stage, T * NE, o).context("widen")
	})?;

	// scale_inplace: in-place, so reload between runs.
	let mut s1 = vec![0.0f64; T * NE];
	let mut s2 = vec![0.0f64; T * NE];
	let x0 = host(T * NE, 43);
	x.load(&x0).context("reload")?;
	gpu_scale_inplace(&scale, T * NE, &x).context("scale")?;
	unsafe { x.download_async(&mut s1, ptr::null_mut()) }.context("dl")?;
	gpu_core::hip::device_synchronize().context("dl")?;
	x.load(&x0).context("reload")?;
	gpu_scale_inplace(&scale, T * NE, &x).context("scale")?;
	unsafe { x.download_async(&mut s2, ptr::null_mut()) }.context("dl")?;
	gpu_core::hip::device_synchronize().context("dl")?;
	cmp("scale_inplace", &s1, &s2);

	// scale_f64: in-place, so reload between runs.
	let mut sf1 = vec![0.0f64; T * NE];
	let mut sf2 = vec![0.0f64; T * NE];
	x.load(&x0).context("reload")?;
	gpu_scale_f64_inplace(&scale, T * NE, &x).context("scale_f64")?;
	unsafe { x.download_async(&mut sf1, ptr::null_mut()) }.context("dl")?;
	gpu_core::hip::device_synchronize().context("dl")?;
	x.load(&x0).context("reload")?;
	gpu_scale_f64_inplace(&scale, T * NE, &x).context("scale_f64")?;
	unsafe { x.download_async(&mut sf2, ptr::null_mut()) }.context("dl")?;
	gpu_core::hip::device_synchronize().context("dl")?;
	cmp("scale_f64", &sf1, &sf2);
	let err_scale = s1
		.iter()
		.zip(&sf1)
		.map(|(a, b)| (a - b).abs())
		.fold(0.0f64, f64::max);
	say(format!("scale_f64 vs hipblasDscal   max_err={err_scale:e}"));

	// Custom f64 GEMM-BT: correctness (CPU oracle + rocBLAS cross-check + bit
	// identity) across every real gemma4 shape, m spanning the full MoE (1..10)
	// through full-canvas (48/54) range.
	say("\n-- gemm_bt_f64 correctness --");
	check_gemm("q_proj/lm_head", 54, 8192, 2816)?;
	check_gemm("k_proj", 48, 2048, 2816)?;
	check_gemm("mlp.gate/up", 8, 2112, 2816)?;
	check_gemm("mlp.down", 10, 2816, 2112)?;
	check_gemm("moe.gate_up", 5, 1408, 2816)?;
	check_gemm("moe.down", 1, 2816, 704)?;
	check_gemm("moe.down", 3, 2816, 704)?;

	say("\n-- gemm_bt_f64 benchmark (rocBLAS vs custom, 50 iters) --");
	bench_shape("q_proj/lm_head", 54, 8192, 2816)?;
	bench_shape("k_proj", 48, 2048, 2816)?;
	bench_shape("mlp.gate/up", 8, 2112, 2816)?;
	bench_shape("mlp.down", 10, 2816, 2112)?;
	bench_shape("moe.gate_up", 5, 1408, 2816)?;
	bench_shape("moe.down", 1, 2816, 704)?;

	say(gpu_core::memory::ledger_report());
	recipe_infer::shutdown();
	Ok(())
}
