use ogdl::ogdl;

#[test]
fn full_probe_transcript() {
	let host = "engi";
	let eth = 0.125;
	let disk_size = 879998558208u64;
	let disk_speed = 2.171;
	let vram = 12868124672u64;
	let pcie = 16.728;
	let flops = 542.1;
	let transfer = 179.824;
	let ram = 33233760256u64;
	let ddr5 = 22.741;
	let cpu_flops = 135.8;
	let cpu_transfer = 59.507;
	ogdl!(r"
measuring:
	engi
		ETH
			1GbE
		DISK
			SIZE
			SATA
		GPU0
			VRAM
			PCIe
			FLOPs
			Transfer
		CPU
			RAM
			DDR5
			FLOPs
			Transfer
");
	assert_eq!(
		ogdl!(r"measuring:".&host.r"ETH").show(),
		"measuring:\n\tengi\n\t\tETH"
	);
	ogdl!(r"measuring:".&host.r"ETH".r"1GbE".&eth);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"DISK").show(),
		"\t\t\t1GbE\t0.125\n\t\tDISK"
	);
	ogdl!(r"measuring:".&host.r"DISK".r"SIZE".&disk_size);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"DISK".r"SIZE").show(),
		"\t\t\tSIZE\t879998558208"
	);
	ogdl!(r"measuring:".&host.r"DISK".r"SATA".&disk_speed);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"GPU0").show(),
		"\t\t\tSATA\t2.171\n\t\tGPU0"
	);
	ogdl!(r"measuring:".&host.r"GPU0".r"VRAM".&vram);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"GPU0".r"VRAM").show(),
		"\t\t\tVRAM\t12868124672"
	);
	ogdl!(r"measuring:".&host.r"GPU0".r"PCIe".&pcie);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"GPU0".r"PCIe").show(),
		"\t\t\tPCIe\t16.728"
	);
	ogdl!(r"measuring:".&host.r"GPU0".r"FLOPs".&flops);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"GPU0".r"FLOPs").show(),
		"\t\t\tFLOPs\t542.1"
	);
	ogdl!(r"measuring:".&host.r"GPU0".r"Transfer".&transfer);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"CPU").show(),
		"\t\t\tTransfer\t179.824\n\t\tCPU"
	);
	ogdl!(r"measuring:".&host.r"CPU".r"RAM".&ram);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"CPU".r"RAM").show(),
		"\t\t\tRAM\t33233760256"
	);
	ogdl!(r"measuring:".&host.r"CPU".r"DDR5".&ddr5);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"CPU".r"DDR5").show(),
		"\t\t\tDDR5\t22.741"
	);
	ogdl!(r"measuring:".&host.r"CPU".r"FLOPs".&cpu_flops);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"CPU".r"FLOPs").show(),
		"\t\t\tFLOPs\t135.8"
	);
	ogdl!(r"measuring:".&host.r"CPU".r"Transfer".&cpu_transfer);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"CPU".r"Transfer").show(),
		"\t\t\tTransfer\t59.507"
	);
}
