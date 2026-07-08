use gpu_core::memory::GpuBuffer;

#[test]
fn roundtrip() {
	let b = { let __up = &[1.0_f64, 2.0, 3.0]; let __ub = GpuBuffer::alloc(__up.len()).unwrap(); __ub.load(__up).unwrap(); __ub };
	let mut out = [0.0_f64; 3];
	unsafe { b.download_async(&mut out, std::ptr::null_mut()) }.unwrap(); gpu_core::hip::device_synchronize().unwrap();
	assert_eq!(out, [1.0, 2.0, 3.0]);
	eprintln!("GPU round-trip OK: {:?}", out);
}
