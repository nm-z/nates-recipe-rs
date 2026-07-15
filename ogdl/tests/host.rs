use ogdl::ogdl;

#[test]
fn runtime_host_values() {
	let host = "sentry";
	let vram = 12868124672_u64;
	ogdl!("measuring:".&host."GPU0"."VRAM".&vram);
	assert_eq!(
		ogdl!("measuring:".&host."GPU0").show(),
		"measuring:\n\tsentry\n\t\tGPU0\n\t\t\tVRAM\t12868124672"
	);
}
