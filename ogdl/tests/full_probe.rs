use ogdl::ogdl;

#[test]
fn full_probe_transcript() {
	let host = "engi";
	let eth = 0.125;
	let disk_size = 879998558208_u64;
	let disk_speed = 2.171;
	let vram = 12868124672_u64;
	let pcie = 16.728;
	let flops = 542.1;
	let transfer = 179.824;
	let ram = 33233760256_u64;
	let ddr5 = 22.741;
	let cpu_flops = 135.8;
	let cpu_transfer = 59.507;
	ogdl!("
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
		ogdl!("measuring:".&host."ETH").show(),
		"measuring:\n\tengi\n\t\tETH"
	);
	ogdl!("measuring:".&host."ETH"."1GbE".&eth);
	assert_eq!(
		ogdl!("measuring:".&host."DISK").show(),
		"\t\t\t1GbE\t0.125\n\t\tDISK"
	);
	ogdl!("measuring:".&host."DISK"."SIZE".&disk_size);
	assert_eq!(
		ogdl!("measuring:".&host."DISK"."SIZE").show(),
		"\t\t\tSIZE\t879998558208"
	);
	ogdl!("measuring:".&host."DISK"."SATA".&disk_speed);
	assert_eq!(
		ogdl!("measuring:".&host."GPU0").show(),
		"\t\t\tSATA\t2.171\n\t\tGPU0"
	);
	ogdl!("measuring:".&host."GPU0"."VRAM".&vram);
	assert_eq!(
		ogdl!("measuring:".&host."GPU0"."VRAM").show(),
		"\t\t\tVRAM\t12868124672"
	);
	ogdl!("measuring:".&host."GPU0"."PCIe".&pcie);
	assert_eq!(
		ogdl!("measuring:".&host."GPU0"."PCIe").show(),
		"\t\t\tPCIe\t16.728"
	);
	ogdl!("measuring:".&host."GPU0"."FLOPs".&flops);
	assert_eq!(
		ogdl!("measuring:".&host."GPU0"."FLOPs").show(),
		"\t\t\tFLOPs\t542.1"
	);
	ogdl!("measuring:".&host."GPU0"."Transfer".&transfer);
	assert_eq!(
		ogdl!("measuring:".&host."CPU").show(),
		"\t\t\tTransfer\t179.824\n\t\tCPU"
	);
	ogdl!("measuring:".&host."CPU"."RAM".&ram);
	assert_eq!(
		ogdl!("measuring:".&host."CPU"."RAM").show(),
		"\t\t\tRAM\t33233760256"
	);
	ogdl!("measuring:".&host."CPU"."DDR5".&ddr5);
	assert_eq!(
		ogdl!("measuring:".&host."CPU"."DDR5").show(),
		"\t\t\tDDR5\t22.741"
	);
	ogdl!("measuring:".&host."CPU"."FLOPs".&cpu_flops);
	assert_eq!(
		ogdl!("measuring:".&host."CPU"."FLOPs").show(),
		"\t\t\tFLOPs\t135.8"
	);
	ogdl!("measuring:".&host."CPU"."Transfer".&cpu_transfer);
	assert_eq!(
		ogdl!("measuring:".&host."CPU"."Transfer").show(),
		"\t\t\tTransfer\t59.507"
	);
}
