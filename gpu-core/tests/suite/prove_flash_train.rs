use gpu_core::kernels::{gpu_flash_attention_backward_into, gpu_flash_attention_train_into};
use gpu_core::memory::GpuBuffer;

// Deterministic pseudo-random fill (no rand dep in this crate's tests).
fn lcg_fill(v: &mut [f64], mut state: u64) {
	for x in v.iter_mut() {
		state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
		*x = ((state >> 33) as f64 / (1u64 << 31) as f64) - 0.5;
	}
}

// CPU oracle for one (sample, head): full softmax attention forward + backward.
// Layouts match the kernels: q/k/v/ctx are [n, s, d] with head slice hh*hd,
// lse is [n][heads][s].
#[allow(clippy::too_many_arguments)]
fn cpu_attention(
	q: &[f64],
	k: &[f64],
	v: &[f64],
	dctx: &[f64],
	n: usize,
	s: usize,
	d: usize,
	heads: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
	let hd = d / heads;
	let scale = 1.0 / (hd as f64).sqrt();
	let mut ctx = vec![0.0; n * s * d];
	let mut lse = vec![0.0; n * heads * s];
	let mut dq = vec![0.0; n * s * d];
	let mut dk = vec![0.0; n * s * d];
	let mut dv = vec![0.0; n * s * d];
	let at = |bi: usize, pos: usize, hh: usize, c: usize| (bi * s + pos) * d + hh * hd + c;
	for bi in 0..n {
		for hh in 0..heads {
			let mut p = vec![0.0; s * s];
			for i in 0..s {
				let mut row = vec![0.0; s];
				let mut m = f64::NEG_INFINITY;
				for j in 0..s {
					let mut dot = 0.0;
					for c in 0..hd {
						dot += q[at(bi, i, hh, c)] * k[at(bi, j, hh, c)];
					}
					row[j] = dot * scale;
					if row[j] > m {
						m = row[j];
					}
				}
				let mut l = 0.0;
				for j in 0..s {
					row[j] = (row[j] - m).exp();
					l += row[j];
				}
				lse[(bi * heads + hh) * s + i] = m + l.ln();
				for j in 0..s {
					p[i * s + j] = row[j] / l;
					for c in 0..hd {
						ctx[at(bi, i, hh, c)] += p[i * s + j] * v[at(bi, j, hh, c)];
					}
				}
			}
			// dP = dctx·Vᵀ; dS = P∘(dP − rowsum(dP∘P))·scale; dQ = dS·K;
			// dK = dSᵀ·Q; dV = Pᵀ·dctx.
			for i in 0..s {
				let mut dp = vec![0.0; s];
				let mut rowsum = 0.0;
				for j in 0..s {
					for c in 0..hd {
						dp[j] += dctx[at(bi, i, hh, c)] * v[at(bi, j, hh, c)];
					}
					rowsum += dp[j] * p[i * s + j];
				}
				for j in 0..s {
					let ds = p[i * s + j] * (dp[j] - rowsum) * scale;
					for c in 0..hd {
						dq[at(bi, i, hh, c)] += ds * k[at(bi, j, hh, c)];
						dk[at(bi, j, hh, c)] += ds * q[at(bi, i, hh, c)];
						dv[at(bi, j, hh, c)] += p[i * s + j] * dctx[at(bi, i, hh, c)];
					}
				}
			}
		}
	}
	(ctx, lse, dq, dk, dv)
}

fn maxdiff(a: &[f64], b: &[f64]) -> f64 {
	a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max)
}

// Flash training forward (context + logsumexp) and the three backward kernels
// must reproduce full-softmax attention math to f64 tolerance. s=100 crosses a
// partial FA_BK=64 key tile and multiple FA_TQ=32 query tiles.
#[test]
fn flash_train_matches_cpu_oracle() {
	gpu_core::hip::set_device(0).expect("set_device");
	let (n, heads, d, s) = (2usize, 2usize, 8usize, 100usize);
	let len = n * s * d;
	let mut q = vec![0.0; len];
	let mut k = vec![0.0; len];
	let mut v = vec![0.0; len];
	let mut dctx = vec![0.0; len];
	lcg_fill(&mut q, 11);
	lcg_fill(&mut k, 22);
	lcg_fill(&mut v, 33);
	lcg_fill(&mut dctx, 44);

	let (c_ctx, c_lse, c_dq, c_dk, c_dv) = cpu_attention(&q, &k, &v, &dctx, n, s, d, heads);

	let gq = GpuBuffer::upload(&q).expect("q");
	let gk = GpuBuffer::upload(&k).expect("k");
	let gv = GpuBuffer::upload(&v).expect("v");
	let gdctx = GpuBuffer::upload(&dctx).expect("dctx");
	let gctx = GpuBuffer::alloc(len).expect("ctx");
	let glse = GpuBuffer::alloc(n * heads * s).expect("lse");
	let gdsum = GpuBuffer::alloc(n * heads * s).expect("dsum");
	let gdq = GpuBuffer::alloc(len).expect("dq");
	let gdk = GpuBuffer::alloc(len).expect("dk");
	let gdv = GpuBuffer::alloc(len).expect("dv");

	gpu_flash_attention_train_into(&gq, &gk, &gv, &gctx, &glse, n, s, d, heads);
	gpu_flash_attention_backward_into(
		&gq, &gk, &gv, &gctx, &gdctx, &glse, &gdsum, &gdq, &gdk, &gdv, n, s, d, heads,
	);

	let dl = |b: &GpuBuffer, l: usize| {
		let mut h = vec![0.0; l];
		b.download(&mut h).expect("download");
		h
	};
	let checks = [
		("ctx", maxdiff(&dl(&gctx, len), &c_ctx)),
		("lse", maxdiff(&dl(&glse, n * heads * s), &c_lse)),
		("dq", maxdiff(&dl(&gdq, len), &c_dq)),
		("dk", maxdiff(&dl(&gdk, len), &c_dk)),
		("dv", maxdiff(&dl(&gdv, len), &c_dv)),
	];
	for (name, diff) in checks {
		eprintln!("flash-train {name}: maxdiff={diff:e}");
		assert!(diff < 1e-12, "flash-train {name} diverged from CPU oracle: {diff:e}");
	}
}
