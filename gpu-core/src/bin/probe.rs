// Device-health probe (SUITE SPEC v3 R5): alloc 64MB, one kernel, readback,
// compare, device-sync error check. Exit 0 = healthy; any panic/mismatch = poisoned.
use gpu_core::memory::GpuBuffer;

fn main() {
	let n = (64usize << 20) / 8;
	let host = vec![4.0f64; n];
	let x = GpuBuffer::upload(&host).expect("probe: upload");
	gpu_core::math_ops::gpu_rsqrt(&x, n, &x).expect("probe: kernel");
	let mut back = vec![0.0f64; n];
	x.download(&mut back).expect("probe: download");
	for (i, v) in back.iter().enumerate() {
		if (v - 0.5).abs() > 1e-12 {
			eprintln!("probe: mismatch at {i}: {v}");
			std::process::exit(1);
		}
	}
	gpu_core::hip::device_synchronize().expect("probe: device sync");
	println!("probe: ok");
}
