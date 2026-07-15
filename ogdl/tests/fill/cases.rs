use ogdl::ogdl;

#[test]
fn fill_and_print_then_select() {
	let host = "engi";
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
");
	let eth = 0.125f64;
	ogdl!("measuring:".&host."ETH"."1GbE".&eth);
	assert_eq!(
		ogdl!("measuring:".&host."ETH").show(),
		"measuring:\n\tengi\n\t\tETH\n\t\t\t1GbE\t0.125"
	);
	let disk_size = 879_998_558_208u64;
	let disk_speed = 2.171f64;
	ogdl!("measuring:".&host."DISK"."SIZE".&disk_size);
	ogdl!("measuring:".&host."DISK"."SATA".&disk_speed);
	assert_eq!(
		ogdl!("measuring:".&host."DISK").show(),
		"\t\tDISK\n\t\t\tSIZE\t879998558208\n\t\t\tSATA\t2.171"
	);
	ogdl!("measuring:".&host."GPU0"."FLOPs"*);
	ogdl!("measuring:".&host[0]);
	ogdl!("measuring:".&host."GPU0"{2});
	ogdl!("measuring:".&host."GPU0"{});
	assert!(!format!("{}", ogdl!("measuring:".&host)).contains("GPU0"));
}
