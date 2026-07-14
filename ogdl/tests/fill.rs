use ogdl::ogdl;

#[test]
fn fill_and_print_then_select() {
	let host = "engi";
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
");
	let eth = 0.125;
	ogdl!(r"measuring:".&host.r"ETH".r"1GbE".&eth);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"ETH").show(),
		"measuring:\n\tengi\n\t\tETH\n\t\t\t1GbE\t0.125"
	);
	let disk_size = 879998558208u64;
	let disk_speed = 2.171;
	ogdl!(r"measuring:".&host.r"DISK".r"SIZE".&disk_size);
	ogdl!(r"measuring:".&host.r"DISK".r"SATA".&disk_speed);
	assert_eq!(
		ogdl!(r"measuring:".&host.r"DISK").show(),
		"\t\tDISK\n\t\t\tSIZE\t879998558208\n\t\t\tSATA\t2.171"
	);
	ogdl!(r"measuring:".&host.r"GPU0".r"FLOPs"*);
	ogdl!(r"measuring:".&host[0]);
	ogdl!(r"measuring:".&host.r"GPU0"{2});
	ogdl!(r"measuring:".&host.r"GPU0"{});
	assert!(!format!("{}", ogdl!(r"measuring:".&host)).contains("GPU0"));
}
