use ogdl::ogdl;

#[test]
fn runtime_host_values() {
	let host = "sentry";
	let vram = 12868124672u64;
	ogdl!(r"measuring:".&host.r"GPU0".r"VRAM".&vram);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"GPU0").show(),
		"measuring:\n\tsentry\n\t\tGPU0\n\t\t\tVRAM\t12868124672"
	);
}
