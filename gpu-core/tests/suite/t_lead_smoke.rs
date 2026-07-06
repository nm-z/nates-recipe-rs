// Lead smoke test: trivially-verifiable functions across domains, executed on the GPU.
use gpu_core::encoding::gpu_one_hot;
use gpu_core::linalg::gpu_ddot;
use gpu_core::math_ops::{gpu_max, gpu_reciprocal, gpu_rsqrt};
use gpu_core::memory::GpuBuffer;
use gpu_core::reductions::{gpu_cumsum_rows, gpu_sort, gpu_sum_all, gpu_sum_all_workspace_bytes};
use gpu_core::rl::gpu_discounted_returns;

fn approx(a: &[f64], b: &[f64]) {
	assert_eq!(a.len(), b.len(), "len mismatch: {:?} vs {:?}", a, b);
	for (i, (x, y)) in a.iter().zip(b).enumerate() {
		assert!(x.is_finite(), "non-finite at {}: {:?}", i, a);
		assert!(
			(x - y).abs() < 1e-6 * (1.0 + y.abs()),
			"idx {}: got {} want {} (full {:?})",
			i,
			x,
			y,
			a
		);
	}
}

#[test]
fn sum_all() {
	let x = GpuBuffer::upload(&[1.0, 2.0, 3.0, 4.0]).unwrap();
	let ws = GpuBuffer::alloc_bytes(gpu_sum_all_workspace_bytes(4)).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_sum_all(&x, &ws, 4, &out).unwrap();
	let mut s = [0.0; 1];
	out.download(&mut s).unwrap();
	eprintln!("sum_all = {}", s[0]);
	assert!((s[0] - 10.0).abs() < 1e-9, "sum_all got {}", s[0]);
}

#[test]
fn ddot() {
	let a = GpuBuffer::upload(&[1.0, 2.0, 3.0]).unwrap();
	let b = GpuBuffer::upload(&[1.0, 1.0, 1.0]).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_ddot(&a, &b, 3, &out).unwrap();
	let mut d = [0.0; 1];
	out.download(&mut d).unwrap();
	eprintln!("ddot = {}", d[0]);
	assert!((d[0] - 6.0).abs() < 1e-9, "ddot got {}", d[0]);
}

#[test]
fn rsqrt_recip_max() {
	let x = GpuBuffer::upload(&[4.0, 16.0]).unwrap();
	let r = GpuBuffer::alloc(2).unwrap();
	gpu_rsqrt(&x, 2, &r).unwrap();
	let mut out = [0.0; 2];
	r.download(&mut out).unwrap();
	eprintln!("rsqrt = {:?}", out);
	approx(&out, &[0.5, 0.25]);

	let y = GpuBuffer::upload(&[2.0, 4.0]).unwrap();
	let rc = GpuBuffer::alloc(2).unwrap();
	gpu_reciprocal(&y, 2, &rc).unwrap();
	rc.download(&mut out).unwrap();
	eprintln!("recip = {:?}", out);
	approx(&out, &[0.5, 0.25]);

	let a = GpuBuffer::upload(&[1.0, 5.0]).unwrap();
	let b = GpuBuffer::upload(&[3.0, 2.0]).unwrap();
	let m = GpuBuffer::alloc(2).unwrap();
	gpu_max(&a, &b, 2, &m).unwrap();
	m.download(&mut out).unwrap();
	eprintln!("max = {:?}", out);
	approx(&out, &[3.0, 5.0]);
}

#[test]
fn sort_pow2() {
	let x = GpuBuffer::upload(&[3.0, 1.0, 2.0, 4.0]).unwrap();
	let s = GpuBuffer::alloc(4).unwrap();
	gpu_sort(&x, 4, &s).unwrap();
	let mut out = [0.0; 4];
	s.download(&mut out).unwrap();
	eprintln!("sort(n=4) = {:?}", out);
	approx(&out, &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn sort_non_pow2() {
	let x = GpuBuffer::upload(&[3.0, 1.0, 2.0]).unwrap();
	let s = GpuBuffer::alloc(3).unwrap();
	gpu_sort(&x, 3, &s).unwrap();
	let mut out = [0.0; 3];
	s.download(&mut out).unwrap();
	eprintln!("sort(n=3) = {:?}", out);
	approx(&out, &[1.0, 2.0, 3.0]);
}

#[test]
fn cumsum_rows() {
	let x = GpuBuffer::upload(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
	let c = GpuBuffer::alloc(6).unwrap();
	gpu_cumsum_rows(&x, 2, 3, &c).unwrap();
	let mut out = [0.0; 6];
	c.download(&mut out).unwrap();
	eprintln!("cumsum_rows = {:?}", out);
	approx(&out, &[1.0, 3.0, 6.0, 4.0, 9.0, 15.0]);
}

#[test]
fn one_hot() {
	let labels = GpuBuffer::upload_i32(&[0, 2]).unwrap();
	let oh = GpuBuffer::alloc(6).unwrap();
	gpu_one_hot(&labels, 2, 3, &oh).unwrap();
	let mut out = [0.0; 6];
	oh.download(&mut out).unwrap();
	eprintln!("one_hot = {:?}", out);
	approx(&out, &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn discounted_returns() {
	let r = GpuBuffer::upload(&[1.0, 1.0, 1.0]).unwrap();
	let gamma = GpuBuffer::upload(&[0.5]).unwrap();
	let g = GpuBuffer::alloc(3).unwrap();
	gpu_discounted_returns(&r, &gamma, 3, &g).unwrap();
	let mut out = [0.0; 3];
	g.download(&mut out).unwrap();
	eprintln!("discounted_returns = {:?}", out);
	approx(&out, &[1.75, 1.5, 1.0]);
}
