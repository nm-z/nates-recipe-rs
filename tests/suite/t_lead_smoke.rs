use gpu_core::encoding::gpu_one_hot;
use gpu_core::math_ops::{gpu_max, gpu_reciprocal, gpu_rsqrt};
use gpu_core::memory::GpuBuffer;
use gpu_core::reductions::{
	gpu_cumsum_rows, gpu_dot, gpu_dot_workspace_bytes, gpu_sort, gpu_sum_all,
	gpu_sum_all_workspace_bytes,
};
use gpu_core::rl::gpu_discounted_returns;
use std::ptr;

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
fn sum_entire() {
	let x = {
		let __up = &[1.0, 2.0, 3.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_sum_all_workspace_bytes(4)).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_sum_all(&x, &ws, 4, &out).unwrap();
	let mut s = [0.0; 1];
	unsafe { out.download_async(&mut s, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("sum_all = {}", s[0]);
	assert!((s[0] - 10.0).abs() < 1e-9, "sum_all got {}", s[0]);
}

#[test]
fn ddot() {
	let a = {
		let __up = &[1.0, 2.0, 3.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &[1.0, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let ws = GpuBuffer::alloc_bytes(gpu_dot_workspace_bytes(3)).unwrap();
	let prod = GpuBuffer::alloc(3).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_dot(&a, &b, &ws, &prod, 3, &out).unwrap();
	let mut d = [0.0; 1];
	unsafe { out.download_async(&mut d, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("ddot = {}", d[0]);
	assert!((d[0] - 6.0).abs() < 1e-9, "ddot got {}", d[0]);
}

#[test]
fn rsqrt_recip_max() {
	let x = {
		let __up = &[4.0, 16.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let r = GpuBuffer::alloc(2).unwrap();
	gpu_rsqrt(&x, 2, &r).unwrap();
	let mut out = [0.0; 2];
	unsafe { r.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("rsqrt = {:?}", out);
	approx(&out, &[0.5, 0.25]);

	let y = {
		let __up = &[2.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let rc = GpuBuffer::alloc(2).unwrap();
	gpu_reciprocal(&y, 2, &rc).unwrap();
	unsafe { rc.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("recip = {:?}", out);
	approx(&out, &[0.5, 0.25]);

	let a = {
		let __up = &[1.0, 5.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let b = {
		let __up = &[3.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let m = GpuBuffer::alloc(2).unwrap();
	gpu_max(&a, &b, 2, &m).unwrap();
	unsafe { m.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("max = {:?}", out);
	approx(&out, &[3.0, 5.0]);
}

#[test]
fn sort_pow2() {
	let x = {
		let __up = &[3.0, 1.0, 2.0, 4.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let s = GpuBuffer::alloc(4).unwrap();
	gpu_sort(&x, 4, &s).unwrap();
	let mut out = [0.0; 4];
	unsafe { s.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("sort(n=4) = {:?}", out);
	approx(&out, &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn sort_non_pow2() {
	let x = {
		let __up = &[3.0, 1.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let s = GpuBuffer::alloc(3).unwrap();
	gpu_sort(&x, 3, &s).unwrap();
	let mut out = [0.0; 3];
	unsafe { s.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("sort(n=3) = {:?}", out);
	approx(&out, &[1.0, 2.0, 3.0]);
}

#[test]
fn cumsum_rows() {
	let x = {
		let __up = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let c = GpuBuffer::alloc(6).unwrap();
	gpu_cumsum_rows(&x, 2, 3, &c).unwrap();
	let mut out = [0.0; 6];
	unsafe { c.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("cumsum_rows = {:?}", out);
	approx(&out, &[1.0, 3.0, 6.0, 4.0, 9.0, 15.0]);
}

#[test]
fn one_hot() {
	let labels = GpuBuffer::upload_i32(&[0, 2]).unwrap();
	let oh = GpuBuffer::alloc(6).unwrap();
	gpu_one_hot(&labels, 2, 3, &oh).unwrap();
	let mut out = [0.0; 6];
	unsafe { oh.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("one_hot = {:?}", out);
	approx(&out, &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn discounted_returns() {
	let r = {
		let __up = &[1.0, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let gamma = {
		let __up = &[0.5];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let g = GpuBuffer::alloc(3).unwrap();
	gpu_discounted_returns(&r, &gamma, 3, &g).unwrap();
	let mut out = [0.0; 3];
	unsafe { g.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	eprintln!("discounted_returns = {:?}", out);
	approx(&out, &[1.75, 1.5, 1.0]);
}
