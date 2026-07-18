//! Flash-attention mask proofs for the KV-cache contract (f64 kernel path).
//!
//! `causal_below` is a position bound: key `sp` is masked for the query at
//! absolute position `p = p_base + i` exactly when `p < causal_below && sp > p`.
//! Two properties are proven here against `gpu_flash_gqa` / `gpu_flash_mla`:
//!
//! 1. Leak-freedom, bitwise: perturbing a K or V cache row at a FUTURE absolute
//!    position must leave every earlier query row's output BIT-identical — the
//!    online softmax may not let a masked key move an accumulator by even one ulp.
//! 2. Reference parity: the flash output matches a straightforward host f64
//!    masked-softmax attention to 1e-12 on seeded shapes covering MHA, GQA
//!    (nqh != nkv), MQA, offset decode (p_base > 0), fully causal
//!    (causal_below = t_kv) and fully bidirectional (causal_below = 0).

use gpu_core::infer_ops::{gpu_flash_gqa, gpu_flash_mla};
use gpu_core::memory::GpuBuffer;

/// One attention problem: kernel-layout dims plus the mask bound. `hdk` is the
/// dot head width, `hdv` the gather width (equal for GQA, distinct for MLA).
struct Shape {
	t_q: usize,
	t_kv: usize,
	nqh: usize,
	nkv: usize,
	hdk: usize,
	hdv: usize,
	p_base: usize,
	causal_below: usize,
}

fn rnd(state: &mut u64) -> f64 {
	*state = state
		.wrapping_mul(6364136223846793005)
		.wrapping_add(1442695040888963407);
	return ((*state >> 33) as f64 / (1u64 << 31) as f64) - 1.0;
}

fn seeded(seed: u64, n: usize) -> Vec<f64> {
	let mut s = seed | 1;
	return (0..n).map(|_| rnd(&mut s)).collect();
}

fn upload(host: &[f64]) -> GpuBuffer {
	let b = GpuBuffer::alloc(host.len()).expect("alloc");
	b.load(host).expect("load");
	return b;
}

fn download(b: &GpuBuffer, n: usize) -> Vec<f64> {
	let mut v = vec![0.0f64; n];
	b.download_host(&mut v).expect("download");
	return v;
}

/// Host f64 reference: two-pass masked-softmax attention with the kernel's
/// exact layouts (q `[t_q,nqh,hdk]`, kc `[t_kv,nkv,hdk]`, vc `[t_kv,nkv,hdv]`,
/// out `[t_q,nqh,hdv]`, kv head = `h / (nqh/nkv)`, q pre-scaled by the caller).
fn host_attn(q: &[f64], kc: &[f64], vc: &[f64], s: &Shape) -> Vec<f64> {
	let mut out = vec![0.0f64; s.t_q * s.nqh * s.hdv];
	for i in 0..s.t_q {
		let p_abs = s.p_base + i;
		for h in 0..s.nqh {
			let kv = h / (s.nqh / s.nkv);
			let qrow = &q[(i * s.nqh + h) * s.hdk..(i * s.nqh + h + 1) * s.hdk];
			let visible: Vec<usize> = (0..s.t_kv)
				.filter(|&sp| !(p_abs < s.causal_below && sp > p_abs))
				.collect();
			let dots: Vec<f64> = visible
				.iter()
				.map(|&sp| {
					let krow = &kc[(sp * s.nkv + kv) * s.hdk..(sp * s.nkv + kv + 1) * s.hdk];
					qrow.iter().zip(krow).map(|(a, b)| a * b).sum()
				})
				.collect();
			let m = dots.iter().copied().fold(f64::NEG_INFINITY, f64::max);
			let w: Vec<f64> = dots.iter().map(|d| (d - m).exp()).collect();
			let wsum: f64 = w.iter().sum();
			let orow = &mut out[(i * s.nqh + h) * s.hdv..(i * s.nqh + h + 1) * s.hdv];
			for (&sp, &wi) in visible.iter().zip(w.iter()) {
				let vrow = &vc[(sp * s.nkv + kv) * s.hdv..(sp * s.nkv + kv + 1) * s.hdv];
				for (o, &v) in orow.iter_mut().zip(vrow) {
					*o += wi * v;
				}
			}
			for o in orow.iter_mut() {
				*o /= wsum;
			}
		}
	}
	return out;
}

fn assert_close(got: &[f64], reference: &[f64], tol: f64, what: &str) {
	assert_eq!(got.len(), reference.len(), "{what}: length mismatch");
	let mut worst = 0.0f64;
	for (i, (g, r)) in got.iter().zip(reference.iter()).enumerate() {
		let err = (g - r).abs() / (1.0 + r.abs());
		if err > worst {
			worst = err;
		}
		assert!(
			err <= tol,
			"{what}: element {i} off by {err:e} (> {tol:e}): got {g} want {r}"
		);
	}
	eprintln!("{what}: worst relative error {worst:e}");
}

#[test]
fn gqa_future_kv_rows_never_touch_earlier_queries() {
	gpu_core::hip::set_device(0).expect("set_device");
	let s = Shape {
		t_q: 3,
		t_kv: 9,
		nqh: 8,
		nkv: 2,
		hdk: 16,
		hdv: 16,
		p_base: 5,
		causal_below: 9,
	};
	let q = seeded(11, s.t_q * s.nqh * s.hdk);
	let k = seeded(22, s.t_kv * s.nkv * s.hdk);
	let v = seeded(33, s.t_kv * s.nkv * s.hdv);
	let (gq, gout) = (
		upload(&q),
		GpuBuffer::alloc(s.t_q * s.nqh * s.hdv).expect("out"),
	);
	let run = |kh: &[f64], vh: &[f64]| -> Vec<u64> {
		let (gk, gv) = (upload(kh), upload(vh));
		gpu_flash_gqa(
			&gq,
			&gk,
			&gv,
			s.t_q,
			s.t_kv,
			s.nqh,
			s.nkv,
			s.hdk,
			0.0,
			s.p_base,
			s.causal_below,
			&gout,
			None,
			None,
			None,
			0,
			true,
		)
		.expect("flash_gqa");
		return download(&gout, s.t_q * s.nqh * s.hdv)
			.iter()
			.map(|x| x.to_bits())
			.collect();
	};
	let base = run(&k, &v);
	let perturb = |src: &[f64], sp: usize| -> Vec<f64> {
		let mut p = src.to_vec();
		for x in p[sp * s.nkv * s.hdk..(sp + 1) * s.nkv * s.hdk].iter_mut() {
			*x += 7.5;
		}
		return p;
	};
	let row_bits = |bits: &[u64], i: usize| return bits[i * s.nqh * s.hdv..(i + 1) * s.nqh * s.hdv].to_vec();
	for (name, got) in [
		("K", run(&perturb(&k, 7), &v)),
		("V", run(&k, &perturb(&v, 7))),
	] {
		for i in 0..2 {
			assert_eq!(
				row_bits(&got, i),
				row_bits(&base, i),
				"{name} row at abs pos 7 leaked into the query at abs pos {} (must be bit-identical)",
				s.p_base + i
			);
		}
		assert_ne!(
			row_bits(&got, 2),
			row_bits(&base, 2),
			"{name} perturbation at abs pos 7 is invisible to the query AT pos 7 — unchanged output means the test lost its teeth"
		);
	}
	for (name, got) in [
		("K", run(&perturb(&k, 8), &v)),
		("V", run(&k, &perturb(&v, 8))),
	] {
		assert_eq!(
			got, base,
			"{name} row at abs pos 8 is future to every query (5..=7) yet changed some output bits"
		);
	}
	eprintln!("future K/V perturbations at pos 7 and 8: earlier query rows bit-identical");
}

#[test]
fn gqa_matches_host_masked_softmax() {
	gpu_core::hip::set_device(0).expect("set_device");
	let shapes = [
		(4, 4, 4, 4, 8, 0, 4),
		(3, 9, 8, 2, 16, 6, 9),
		(2, 7, 6, 1, 32, 5, 7),
		(5, 5, 4, 2, 16, 0, 0),
		(1, 12, 8, 4, 24, 11, 12),
	];
	for (n, &(t_q, t_kv, nqh, nkv, hd, p_base, causal_below)) in shapes.iter().enumerate() {
		let s = Shape {
			t_q,
			t_kv,
			nqh,
			nkv,
			hdk: hd,
			hdv: hd,
			p_base,
			causal_below,
		};
		let q = seeded(100 + n as u64, s.t_q * s.nqh * s.hdk);
		let k = seeded(200 + n as u64, s.t_kv * s.nkv * s.hdk);
		let v = seeded(300 + n as u64, s.t_kv * s.nkv * s.hdv);
		let (gq, gk, gv) = (upload(&q), upload(&k), upload(&v));
		let gout = GpuBuffer::alloc(s.t_q * s.nqh * s.hdv).expect("out");
		gpu_flash_gqa(
			&gq,
			&gk,
			&gv,
			s.t_q,
			s.t_kv,
			s.nqh,
			s.nkv,
			s.hdk,
			0.0,
			s.p_base,
			s.causal_below,
			&gout,
			None,
			None,
			None,
			0,
			true,
		)
		.expect("flash_gqa");
		assert_close(
			&download(&gout, s.t_q * s.nqh * s.hdv),
			&host_attn(&q, &k, &v, &s),
			1e-12,
			&format!(
				"gqa shape {n} (t_q={t_q} t_kv={t_kv} nqh={nqh} nkv={nkv} hd={hd} p_base={p_base} causal_below={causal_below})"
			),
		);
	}
}

#[test]
fn mla_matches_host_masked_softmax() {
	gpu_core::hip::set_device(0).expect("set_device");
	let shapes = [(3, 8, 4, 24, 16, 5, 8), (4, 6, 4, 16, 8, 0, 0)];
	for (n, &(t_q, t_kv, nqh, hdk, hdv, p_base, causal_below)) in shapes.iter().enumerate() {
		let s = Shape {
			t_q,
			t_kv,
			nqh,
			nkv: 1,
			hdk,
			hdv,
			p_base,
			causal_below,
		};
		let q = seeded(400 + n as u64, s.t_q * s.nqh * s.hdk);
		let k = seeded(500 + n as u64, s.t_kv * s.nkv * s.hdk);
		let v = seeded(600 + n as u64, s.t_kv * s.nkv * s.hdv);
		let (gq, gk, gv) = (upload(&q), upload(&k), upload(&v));
		let gout = GpuBuffer::alloc(s.t_q * s.nqh * s.hdv).expect("out");
		gpu_flash_mla(
			&gq,
			&gk,
			&gv,
			s.t_q,
			s.t_kv,
			s.nqh,
			s.nkv,
			s.hdk,
			s.hdv,
			s.p_base,
			s.causal_below,
			&gout,
			None,
			None,
			None,
			0,
			true,
		)
		.expect("flash_mla");
		assert_close(
			&download(&gout, s.t_q * s.nqh * s.hdv),
			&host_attn(&q, &k, &v, &s),
			1e-12,
			&format!(
				"mla shape {n} (t_q={t_q} t_kv={t_kv} nqh={nqh} hdk={hdk} hdv={hdv} p_base={p_base} causal_below={causal_below})"
			),
		);
	}
}
