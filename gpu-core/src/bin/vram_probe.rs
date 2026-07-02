//! Disposable mapping probe: attempt one device allocation of argv[1] bytes and
//! exit 0 (mappable) or 2 (refused). An ask past the true ceiling dies in the
//! driver's uncatchable VmHeap::MapPhysMemory assert — in THIS process, which is
//! the point: the parent reads the exit status and never takes that risk itself.
//! Spawned by the alloc choke with VRAM_PROBE_CHILD=1 so the child's own alloc
//! skips probing (recursion base case).

fn main() {
	let n: usize = std::env::args()
		.nth(1)
		.and_then(|a| a.parse().ok())
		.expect("usage: vram_probe <bytes>");
	// The probe must be a single bare hipMallocAsync — no pool warm, no memset,
	// no kernel launches. A child running the warm's memset can page-fault and
	// the fault handling takes down the PARENT's in-flight work (observed:
	// cookbook warm died with hipErrorIllegalAddress as collateral).
	gpu_core::memory::skip_pool_warm();
	gpu_core::hip::set_device(0).expect("set_device");
	std::process::exit(match gpu_core::memory::GpuBuffer::try_alloc_bytes(n) {
		Some(_) => 0,
		None => 2,
	});
}
