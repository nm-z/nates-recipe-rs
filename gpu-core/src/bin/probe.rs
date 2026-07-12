use gpu_core::memory::GpuBuffer;

fn main() {
	let n = (64usize << 20) / 8;
	let host = vec![4.0f64; n];
	let x = GpuBuffer::alloc(n).expect("probe: alloc");
	x.load(&host).expect("probe: upload");
	gpu_core::math_ops::gpu_rsqrt(&x, n, &x).expect("probe: kernel");
	let mut back = vec![0.0f64; n];
	unsafe { x.download_async(&mut back, std::ptr::null_mut()) }.expect("probe: download");
	gpu_core::hip::device_synchronize().expect("probe: download sync");
	for i in 0..back.len() {
		let v = back[i];
		match (v - 0.5).abs().partial_cmp(&1e-12) {
			Some(std::cmp::Ordering::Greater) => {
				gpu_core::log::Write::err(&format!("probe: mismatch at {i}: {v}"));
				std::process::exit(1);
			}
			Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal) | None => {
				continue;
			}
		}
	}
	gpu_core::hip::device_synchronize().expect("probe: device sync");
	gpu_core::log::set_opt(gpu_core::log::Opt {
		gpu: true,
		..gpu_core::log::Opt::default()
	});
	gpu_core::log::Write::line(gpu_core::log::opt().gpu, "probe: ok");
}
