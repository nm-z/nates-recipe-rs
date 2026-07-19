//! Settles the reviewer's allegation that the flash mask leaks future positions,
//! and pins the kernel to a reference masked-softmax attention.
//!
//! `causal_below` is a position bound: for the query at absolute position
//! `p = p_base + i`, key `sp` is masked exactly when `p < causal_below && sp > p`.
//! With `causal_below == t_kv` (fully causal) every new query masks every key past
//! its own absolute position, so:
//!   1. perturbing a K/V row at a FUTURE position must leave every earlier query's
//!      output BIT-IDENTICAL (the masked key contributes a hard -inf score, hence
//!      an exact 0.0 weight — no dependence on its value), and
//!   2. the kernel's online-softmax output matches a straightforward host f64
//!      masked softmax attention, including GQA head sharing.

use gpu_core::infer_ops::gpu_flash_gqa;
use gpu_core::memory::GpuBuffer;
use std::sync::Mutex;

static GPU: Mutex<()> = Mutex::new(());

fn lcg(seed: &mut u64) -> f64 {
	*seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
	return ((*seed >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0;
}

fn upload(v: &[f64]) -> GpuBuffer {
	let b = GpuBuffer::alloc(v.len()).expect("alloc");
	b.load(v).expect("load");
	return b;
}

fn reference(
	q: &[f64],
	k: &[f64],
	v: &[f64],
	t_q: usize,
	t_kv: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	p_base: usize,
) -> Vec<f64> {
	let g = nqh / nkv;
	let mut out = vec![0.0f64; t_q * nqh * hd];
	for i in 0..t_q {
		let p = p_base + i;
		for h in 0..nqh {
			let kv = h / g;
			let mut scores = Vec::new();
			for j in 0..t_kv {
				if j > p {
					continue;
				}
				let mut dot = 0.0;
				for x in 0..hd {
					dot += q[i * nqh * hd + h * hd + x] * k[j * nkv * hd + kv * hd + x];
				}
				scores.push((j, dot));
			}
			let m = scores.iter().fold(f64::MIN, |a, &(_, s)| a.max(s));
			let denom: f64 = scores.iter().map(|&(_, s)| (s - m).exp()).sum();
			for x in 0..hd {
				let mut acc = 0.0;
				for &(j, s) in &scores {
					acc += (s - m).exp() / denom * v[j * nkv * hd + kv * hd + x];
				}
				out[i * nqh * hd + h * hd + x] = acc;
			}
		}
	}
	return out;
}

fn run_flash(
	q: &[f64],
	k: &[f64],
	v: &[f64],
	t_q: usize,
	t_kv: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	p_base: usize,
) -> Vec<f64> {
	let qb = upload(q);
	let kb = upload(k);
	let vb = upload(v);
	let ob = GpuBuffer::alloc(t_q * nqh * hd).expect("out");
	let cm = GpuBuffer::alloc(t_q * nqh).expect("cm");
	let cl = GpuBuffer::alloc(t_q * nqh).expect("cl");
	let cacc = GpuBuffer::alloc(t_q * nqh * hd).expect("cacc");
	gpu_flash_gqa(&qb, &kb, &vb, t_q, t_kv, nqh, nkv, hd, 0.0, p_base, t_kv, &ob, &cm, &cl, &cacc, 0, true).expect("flash");
	gpu_core::hip::device_synchronize().expect("sync");
	let mut out = vec![0.0f64; t_q * nqh * hd];
	ob.download_host(&mut out).expect("download");
	gpu_core::hip::device_synchronize().expect("sync2");
	return out;
}

#[test]
fn flash_mask_does_not_leak_future_positions() {
	let _g = GPU.lock().unwrap_or_else(|p| p.into_inner());
	let (t_q, nqh, nkv, hd, p_base) = (4usize, 4usize, 2usize, 8usize, 3usize);
	let t_kv = p_base + t_q;
	let mut s = 0x1234_5678u64;
	let q: Vec<f64> = (0..t_q * nqh * hd).map(|_| lcg(&mut s)).collect();
	let k: Vec<f64> = (0..t_kv * nkv * hd).map(|_| lcg(&mut s)).collect();
	let v: Vec<f64> = (0..t_kv * nkv * hd).map(|_| lcg(&mut s)).collect();

	let base = run_flash(&q, &k, &v, t_q, t_kv, nqh, nkv, hd, p_base);

	let pf = p_base + t_q - 1;
	let mut k2 = k.clone();
	let mut v2 = v.clone();
	for x in 0..nkv * hd {
		k2[pf * nkv * hd + x] += 7.0;
		v2[pf * nkv * hd + x] -= 5.0;
	}
	let perturbed = run_flash(&q, &k2, &v2, t_q, t_kv, nqh, nkv, hd, p_base);

	for i in 0..t_q - 1 {
		for h in 0..nqh {
			for x in 0..hd {
				let idx = i * nqh * hd + h * hd + x;
				assert_eq!(
					base[idx].to_bits(),
					perturbed[idx].to_bits(),
					"query {i} (abs {}) changed when future key row {pf} was perturbed — mask leaks",
					p_base + i
				);
			}
		}
	}
	let last = (t_q - 1) * nqh * hd;
	assert!(
		(base[last] - perturbed[last]).abs() > 1e-9,
		"last query must see its own position's key"
	);
}

#[test]
fn flash_matches_reference_masked_softmax() {
	let _g = GPU.lock().unwrap_or_else(|p| p.into_inner());
	let shapes = [
		(5usize, 2usize, 2usize, 8usize, 0usize),
		(6, 4, 2, 16, 2),
		(3, 8, 2, 8, 5),
		(4, 4, 1, 8, 1),
		(7, 6, 6, 8, 3),
	];
	let mut s = 0xdead_beefu64;
	for &(t_q, nqh, nkv, hd, p_base) in &shapes {
		let t_kv = p_base + t_q;
		let q: Vec<f64> = (0..t_q * nqh * hd).map(|_| lcg(&mut s)).collect();
		let k: Vec<f64> = (0..t_kv * nkv * hd).map(|_| lcg(&mut s)).collect();
		let v: Vec<f64> = (0..t_kv * nkv * hd).map(|_| lcg(&mut s)).collect();
		let got = run_flash(&q, &k, &v, t_q, t_kv, nqh, nkv, hd, p_base);
		let want = reference(&q, &k, &v, t_q, t_kv, nqh, nkv, hd, p_base);
		let mut maxerr = 0.0f64;
		for (a, b) in got.iter().zip(want.iter()) {
			maxerr = maxerr.max((a - b).abs());
		}
		assert!(
			maxerr < 1e-12,
			"shape t_q={t_q} nqh={nqh} nkv={nkv} hd={hd} p_base={p_base}: max abs err {maxerr:e} vs reference"
		);
	}
}
