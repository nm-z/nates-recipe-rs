use gpu_core::infer_ops::{gpu_flash_gqa, gpu_flash_mla};
use gpu_core::memory::GpuBuffer;

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

fn fresh_carry(blocks: usize, acc_len: usize) -> (GpuBuffer, GpuBuffer, GpuBuffer) {
	let m = upload(&vec![-1e300f64; blocks]);
	let l = upload(&vec![0.0f64; blocks]);
	let acc = upload(&vec![0.0f64; acc_len]);
	return (m, l, acc);
}

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
