use gpu_core::hip;
use gpu_core::memory::GpuBuffer;
use gpu_core::moe::{gpu_moe_backward, gpu_moe_route};

#[test]
fn moe_backward_matches_finite_diff() {
	hip::set_device(0).expect("set_device");
	let (n, d, e) = (5usize, 4usize, 3usize);
	let mk = |seed: usize, len: usize, scale: f64| -> Vec<f64> {
		(0..len)
			.map(|i| (((i * 1103515245 + seed * 12345) % 1000) as f64 / 1000.0 - 0.5) * scale)
			.collect()
	};
	let hidden = mk(1, n * d, 1.0);
	let gate_w = mk(2, d * e, 1.0);
	let expert_w = mk(3, e * d * d, 1.0);
	let g = mk(4, n * d, 1.0);

	let hb = GpuBuffer::alloc(hidden.len()).expect("h");
	hb.load(&hidden).expect("h load");
	let gwb = GpuBuffer::alloc(gate_w.len()).expect("gw");
	gwb.load(&gate_w).expect("gw load");
	let ewb = GpuBuffer::alloc(expert_w.len()).expect("ew");
	ewb.load(&expert_w).expect("ew load");
	let gb = GpuBuffer::alloc(g.len()).expect("g");
	gb.load(&g).expect("g load");
	let (d_hidden, d_gate_w, d_expert_w) =
		gpu_moe_backward(&hb, &gwb, &ewb, &gb, n, d, e).expect("bwd");
	let dl = |buf: &GpuBuffer, len: usize| -> Vec<f64> {
		let mut v = vec![0.0f64; len];
		unsafe { buf.download_async(&mut v, std::ptr::null_mut()) }.expect("download");
		hip::device_synchronize().expect("download sync");
		v
	};
	let dh = dl(&d_hidden, n * d);
	let dgw = dl(&d_gate_w, d * e);
	let dew = dl(&d_expert_w, e * d * d);

	let eps = 1e-6;
	let loss = |hh: &[f64], gw: &[f64], ew: &[f64]| -> f64 {
		let hhb = GpuBuffer::alloc(hh.len()).expect("h");
		hhb.load(hh).expect("h load");
		let gwb = GpuBuffer::alloc(gw.len()).expect("gw");
		gwb.load(gw).expect("gw load");
		let ewb = GpuBuffer::alloc(ew.len()).expect("ew");
		ewb.load(ew).expect("ew load");
		let out = gpu_moe_route(&hhb, &gwb, &ewb, n, d, e).expect("fwd");
		let mut o = vec![0.0f64; n * d];
		unsafe { out.download_async(&mut o, std::ptr::null_mut()) }.expect("download out");
		hip::device_synchronize().expect("download out sync");
		o.iter().zip(&g).map(|(a, b)| a * b).sum()
	};
	let fd = |base: &[f64], idx: usize, which: u8| -> f64 {
		let mut p = base.to_vec();
		let mut m = base.to_vec();
		p[idx] += eps;
		m[idx] -= eps;
		match which {
			0 => (loss(&p, &gate_w, &expert_w) - loss(&m, &gate_w, &expert_w)) / (2.0 * eps),
			1 => (loss(&hidden, &p, &expert_w) - loss(&hidden, &m, &expert_w)) / (2.0 * eps),
			_ => (loss(&hidden, &gate_w, &p) - loss(&hidden, &gate_w, &m)) / (2.0 * eps),
		}
	};
	let mut maxdiff = 0.0f64;
	for i in 0..n * d {
		maxdiff = maxdiff.max((fd(&hidden, i, 0) - dh[i]).abs());
	}
	for i in 0..d * e {
		maxdiff = maxdiff.max((fd(&gate_w, i, 1) - dgw[i]).abs());
	}
	for i in 0..e * d * d {
		maxdiff = maxdiff.max((fd(&expert_w, i, 2) - dew[i]).abs());
	}
	eprintln!("moe backward vs finite-diff: maxdiff = {maxdiff:e}");
	assert!(maxdiff < 1e-6, "moe backward != finite diff: {maxdiff:e}");
}
