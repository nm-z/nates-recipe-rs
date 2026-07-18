//! Cross-launch carry equivalence: splitting the KV sequence across several
//! `gpu_flash_gqa`/`gpu_flash_mla` launches with the (m,l,acc) online-softmax
//! state threaded between them must produce output BIT-IDENTICAL to a single
//! null-carry launch over the whole sequence.
//!
//! Contract (f64_infer.hip): carry buffers are ALWAYS passed; kv_off == 0 starts fresh
//! (-inf,0,0), in-launch normalize, kv_off/finalize inert). `Some` carry resumes
//! the prior segment's (m,l,acc), folds this segment's keys at ABSOLUTE position
//! `kv_off + sp`, and either normalizes to `out` (finalize=true) or stores
//! (m,l,acc) back writing no out (finalize=false). Carry is indexed by query
//! block `i*nqh+h`: m_io/l_io are `[t_q*nqh]`, acc_io is `[t_q*nqh*hd]` (GQA) /
//! `[t_q*nqh*hdv]` (MLA). Ascending kv_off keeps the accumulation order and the
//! absolute key positions identical to one launch, so the state round-trips
//! through global as exact T bits and the finalized output matches to the bit.
//!
//! The assertion is exact `to_bits()` equality, not epsilon: the segmented path
//! is designed to be the SAME float operations in the SAME order as the single
//! launch. Any bit difference means the kernel mishandles kv_off, the carry
//! indexing, the state round-trip, or the segment-boundary normalize — all real
//! bugs. Coverage: GQA head ratios nqh/nkv = 1, 2, 4 and an MLA shape; causal
//! (causal_below=t_kv with p_base>0) and bidirectional (causal_below=0); uneven
//! 3-segment splits including a boundary that lands mid-way through the causal
//! frontier (where some queries mask the crossed keys and others do not).

use gpu_core::infer_ops::{gpu_flash_gqa, gpu_flash_mla};
use gpu_core::memory::GpuBuffer;

/// One attention problem plus how the KV sequence is chopped. `hdk` is the dot
/// head width, `hdv` the value-gather width (equal for GQA, distinct for MLA).
struct Case {
	t_q: usize,
	t_kv: usize,
	nqh: usize,
	nkv: usize,
	hdk: usize,
	hdv: usize,
	p_base: usize,
	causal_below: usize,
	segs: &'static [usize],
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

fn bits(b: &GpuBuffer, n: usize) -> Vec<u64> {
	let mut v = vec![0.0f64; n];
	b.download_host(&mut v).expect("download");
	return v.iter().map(|x| x.to_bits()).collect();
}

/// Fresh carry buffers at the (-1e300, 0, 0) start the kernel uses for a cold
/// launch, so segment 0 with carry reproduces a fresh launch exactly.
fn fresh_carry(blocks: usize, acc_len: usize) -> (GpuBuffer, GpuBuffer, GpuBuffer) {
	let m = upload(&vec![-1e300f64; blocks]);
	let l = upload(&vec![0.0f64; blocks]);
	let acc = upload(&vec![0.0f64; acc_len]);
	return (m, l, acc);
}

/// Single null-carry launch over the whole sequence — the reference.
fn single(c: &Case, q: &GpuBuffer, k: &GpuBuffer, v: &GpuBuffer, mla: bool) -> Vec<u64> {
	let out = GpuBuffer::alloc(c.t_q * c.nqh * c.hdv).expect("out");
	let (cm, cl, cacc) = fresh_carry(c.t_q * c.nqh, c.t_q * c.nqh * c.hdv);
	if mla {
		gpu_flash_mla(
			q,
			k,
			v,
			c.t_q,
			c.t_kv,
			c.nqh,
			c.nkv,
			c.hdk,
			c.hdv,
			c.p_base,
			c.causal_below,
			&out,
			&cm,
			&cl,
			&cacc,
			0,
			true,
		)
		.expect("single mla");
	} else {
		gpu_flash_gqa(
			q,
			k,
			v,
			c.t_q,
			c.t_kv,
			c.nqh,
			c.nkv,
			c.hdk,
			0.0,
			c.p_base,
			c.causal_below,
			&out,
			&cm,
			&cl,
			&cacc,
			0,
			true,
		)
		.expect("single gqa");
	}
	return bits(&out, c.t_q * c.nqh * c.hdv);
}

/// Segmented launches over `c.segs`, carry threaded, finalize only on the last.
fn chunked(c: &Case, q: &GpuBuffer, k: &GpuBuffer, v: &GpuBuffer, mla: bool) -> Vec<u64> {
	let out = GpuBuffer::alloc(c.t_q * c.nqh * c.hdv).expect("out");
	let (m, l, acc) = fresh_carry(c.t_q * c.nqh, c.t_q * c.nqh * c.hdv);
	let mut off = 0usize;
	for (idx, &len) in c.segs.iter().enumerate() {
		let last = idx == c.segs.len() - 1;
		if mla {
			let kseg = k.view(off * c.nkv * c.hdk, len * c.nkv * c.hdk);
			let vseg = v.view(off * c.nkv * c.hdv, len * c.nkv * c.hdv);
			gpu_flash_mla(
				q,
				&kseg,
				&vseg,
				c.t_q,
				len,
				c.nqh,
				c.nkv,
				c.hdk,
				c.hdv,
				c.p_base,
				c.causal_below,
				&out,
				&m,
				&l,
				&acc,
				off,
				last,
			)
			.expect("chunk mla");
		} else {
			let kseg = k.view(off * c.nkv * c.hdk, len * c.nkv * c.hdk);
			let vseg = v.view(off * c.nkv * c.hdk, len * c.nkv * c.hdk);
			gpu_flash_gqa(
				q,
				&kseg,
				&vseg,
				c.t_q,
				len,
				c.nqh,
				c.nkv,
				c.hdk,
				0.0,
				c.p_base,
				c.causal_below,
				&out,
				&m,
				&l,
				&acc,
				off,
				last,
			)
			.expect("chunk gqa");
		}
		off += len;
	}
	assert_eq!(off, c.t_kv, "segments must cover exactly t_kv");
	return bits(&out, c.t_q * c.nqh * c.hdv);
}

fn run(cases: &[Case], mla: bool, label: &str) {
	gpu_core::hip::set_device(0).expect("set_device");
	for (n, c) in cases.iter().enumerate() {
		let q = upload(&seeded(11 + n as u64, c.t_q * c.nqh * c.hdk));
		let k = upload(&seeded(22 + n as u64, c.t_kv * c.nkv * c.hdk));
		let v = upload(&seeded(33 + n as u64, c.t_kv * c.nkv * c.hdv));
		let one = single(c, &q, &k, &v, mla);
		let many = chunked(c, &q, &k, &v, mla);
		assert_eq!(
			many, one,
			"{label} case {n} (t_q={} t_kv={} nqh={} nkv={} hdk={} hdv={} p_base={} causal_below={} segs={:?}): chunked carry is not bit-identical to the single launch",
			c.t_q, c.t_kv, c.nqh, c.nkv, c.hdk, c.hdv, c.p_base, c.causal_below, c.segs
		);
	}
	eprintln!(
		"{label}: {} cases chunked == single, bit-identical",
		cases.len()
	);
}

#[test]
fn gqa_chunked_carry_equals_single_launch() {
	let cases = [
		Case {
			t_q: 4,
			t_kv: 12,
			nqh: 4,
			nkv: 4,
			hdk: 16,
			hdv: 16,
			p_base: 3,
			causal_below: 12,
			segs: &[4, 4, 4],
		},
		Case {
			t_q: 3,
			t_kv: 11,
			nqh: 8,
			nkv: 4,
			hdk: 16,
			hdv: 16,
			p_base: 5,
			causal_below: 11,
			segs: &[5, 3, 3],
		},
		Case {
			t_q: 5,
			t_kv: 13,
			nqh: 8,
			nkv: 2,
			hdk: 24,
			hdv: 24,
			p_base: 8,
			causal_below: 13,
			segs: &[2, 7, 4],
		},
		Case {
			t_q: 4,
			t_kv: 12,
			nqh: 4,
			nkv: 4,
			hdk: 16,
			hdv: 16,
			p_base: 3,
			causal_below: 0,
			segs: &[5, 4, 3],
		},
		Case {
			t_q: 3,
			t_kv: 10,
			nqh: 8,
			nkv: 2,
			hdk: 16,
			hdv: 16,
			p_base: 0,
			causal_below: 0,
			segs: &[3, 3, 4],
		},
	];
	run(&cases, false, "gqa");
}

#[test]
fn mla_chunked_carry_equals_single_launch() {
	let cases = [
		Case {
			t_q: 4,
			t_kv: 12,
			nqh: 4,
			nkv: 1,
			hdk: 24,
			hdv: 16,
			p_base: 3,
			causal_below: 12,
			segs: &[4, 5, 3],
		},
		Case {
			t_q: 3,
			t_kv: 10,
			nqh: 4,
			nkv: 1,
			hdk: 24,
			hdv: 16,
			p_base: 0,
			causal_below: 0,
			segs: &[4, 2, 4],
		},
	];
	run(&cases, true, "mla");
}
