use gpu_core::memory::GpuBuffer;

#[test]
fn roundtrip() {
	let b = GpuBuffer::upload(&[1.0_f64, 2.0, 3.0]).unwrap();
	let mut out = [0.0_f64; 3];
	b.download(&mut out).unwrap();
	assert_eq!(out, [1.0, 2.0, 3.0]);
	eprintln!("GPU round-trip OK: {:?}", out);
}
