use gpu_core::linalg::gpu_bmm_into;
use gpu_core::memory::GpuBuffer;

// CPU reference: per batch, C(m×n) = opA(A)·opB(B), all row-major contiguous.
fn cpu_bmm(
	a: &[f64],
	b: &[f64],
	batch: usize,
	m: usize,
	n: usize,
	k: usize,
	ta: bool,
	tb: bool,
) -> Vec<f64> {
	let mut c = vec![0.0f64; batch * m * n];
	for bi in 0..batch {
		let ao = bi * m * k;
		let bo = bi * k * n;
		let co = bi * m * n;
		for i in 0..m {
			for j in 0..n {
				let mut s = 0.0;
				for p in 0..k {
					let av = if ta {
						a[ao + p * m + i]
					} else {
						a[ao + i * k + p]
					};
					let bv = if tb {
						b[bo + j * k + p]
					} else {
						b[bo + p * n + j]
					};
					s += av * bv;
				}
				c[co + i * n + j] = s;
			}
		}
	}
	c
}

fn run_case(batch: usize, m: usize, n: usize, k: usize, ta: bool, tb: bool) {
	gpu_core::hip::set_device(0).expect("dev");
	let a_rows = if ta { k } else { m };
	let a_cols = if ta { m } else { k };
	let b_rows = if tb { n } else { k };
	let b_cols = if tb { k } else { n };
	let a: Vec<f64> = (0..batch * a_rows * a_cols)
		.map(|i| (i as f64 * 0.7).sin())
		.collect();
	let b: Vec<f64> = (0..batch * b_rows * b_cols)
		.map(|i| (i as f64 * 0.9).cos())
		.collect();
	let want = cpu_bmm(&a, &b, batch, m, n, k, ta, tb);
	let ag = GpuBuffer::upload(&a).expect("a");
	let bg = GpuBuffer::upload(&b).expect("b");
	let cg = GpuBuffer::alloc(batch * m * n).expect("c");
	gpu_bmm_into(
		&cg,
		&ag,
		&bg,
		batch,
		m,
		n,
		k,
		a_cols,
		b_cols,
		n,
		a_rows * a_cols,
		b_rows * b_cols,
		m * n,
		0,
		0,
		0,
		ta,
		tb,
	);
	let mut got = vec![0.0f64; batch * m * n];
	cg.download(&mut got).expect("dl");
	let maxd = want
		.iter()
		.zip(&got)
		.map(|(x, y)| (x - y).abs())
		.fold(0.0, f64::max);
	assert!(maxd < 1e-9, "bmm ta={ta} tb={tb} maxdiff={maxd:.2e}");
}

#[test]
fn bmm_all_transpose_modes() {
	run_case(3, 5, 7, 4, false, false);
	run_case(3, 5, 7, 4, false, true);
	run_case(3, 5, 7, 4, true, false);
	run_case(3, 5, 7, 4, true, true);
}

// Per-head view: Q packed [S, heads*hd], one head = hd columns with lda = heads*hd.
#[test]
fn bmm_per_head_offset() {
	gpu_core::hip::set_device(0).expect("dev");
	let (n, s, heads, hd) = (2usize, 4usize, 3usize, 2usize);
	let d = heads * hd;
	// Q,K packed [n, S, d]; compute scores_h[i] = Q_i,h(S×hd) · K_i,hᵀ(hd×S) per head.
	let q: Vec<f64> = (0..n * s * d).map(|i| (i as f64 * 0.3).sin()).collect();
	let kk: Vec<f64> = (0..n * s * d).map(|i| (i as f64 * 0.5).cos()).collect();
	let qg = GpuBuffer::upload(&q).expect("q");
	let kg = GpuBuffer::upload(&kk).expect("k");
	let scores = GpuBuffer::alloc(heads * n * s * s).expect("sc");
	for h in 0..heads {
		// batch over n: A=Q head block, B=K head block, C=scores_h, opB=trans
		gpu_bmm_into(
			&scores,
			&qg,
			&kg,
			n,
			s,
			s,
			hd,
			d,
			d,
			s,
			s * d,
			s * d,
			s * s,
			h * hd,
			h * hd,
			h * n * s * s,
			false,
			true,
		);
	}
	let mut got = vec![0.0f64; heads * n * s * s];
	scores.download(&mut got).expect("dl");
	// CPU ref
	for h in 0..heads {
		for i in 0..n {
			for a in 0..s {
				for b2 in 0..s {
					let mut acc = 0.0;
					for p in 0..hd {
						acc += q[i * s * d + a * d + h * hd + p]
							* kk[i * s * d + b2 * d + h * hd + p];
					}
					let g = got[h * n * s * s + i * s * s + a * s + b2];
					assert!(
						(acc - g).abs() < 1e-9,
						"head {h} i{i} [{a},{b2}] want {acc} got {g}"
					);
				}
			}
		}
	}
}
