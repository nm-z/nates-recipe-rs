use recipe::probe::{GpuDev, Machine, parse_config, write_config};

fn sample(gpus: Vec<GpuDev>) -> Machine {
	Machine {
		host: "engi".to_string(),
		gpus,
		ram: 67_169_726_464,
		ddr5_gbs: 38.4,
		cpu_transfer_gbs: 42.5,
		cpu_gflops: 89.2,
		disk_size: 500_107_862_016,
		sata_gbs: 1.9,
		eth_gbs: 0.125,
	}
}

#[test]
fn beacon_roundtrips_exact() {
	let m = sample(vec![GpuDev {
		vram: 12_884_901_888,
		pcie_gbs: 11.581,
		flops_gflops: 254.7,
		transfer_gbs: 402.3,
	}]);
	assert_eq!(Machine::beacon_decode(&m.beacon_encode()).expect("decode"), m);

	let storage = sample(Vec::new());
	let back = Machine::beacon_decode(&storage.beacon_encode()).expect("decode");
	assert!(back.gpus.is_empty(), "storage node carries no gpu block");
	assert_eq!(back, storage);
}

#[test]
fn config_ogdl_roundtrips() {
	let a = sample(vec![GpuDev {
		vram: 12_884_901_888,
		pcie_gbs: 11.581,
		flops_gflops: 254.7,
		transfer_gbs: 402.3,
	}]);
	let mut b = sample(Vec::new());
	b.host = "archy".to_string();
	let text = write_config(&[a.clone(), b.clone()]);
	let parsed = parse_config(&text);
	assert_eq!(parsed.len(), 2, "trailing schema block must not parse as a host");
	assert_eq!(parsed[0], a);
	assert_eq!(parsed[1], b);
	assert!(parsed[1].gpus.is_empty());
}

#[test]
fn config_emits_eth_transfer_schema() {
	let m = sample(vec![GpuDev {
		vram: 12_884_901_888,
		pcie_gbs: 11.581,
		flops_gflops: 254.7,
		transfer_gbs: 402.3,
	}]);
	let text = write_config(&[m]);
	assert!(text.contains("\t\tETH\n"), "ETH section present");
	assert!(text.contains("\t\t\t1GbE\t0.125\n"), "ETH link line present");
	assert!(text.contains("\t\tGPU0\n"), "uppercase GPU0 section");
	assert!(text.contains("\t\tCPU\n"), "uppercase CPU section");
	assert!(text.contains("\t\tDISK\n"), "uppercase DISK section");
	assert!(text.contains("\t\t\tDDR5\t38.400\n"), "CPU DDR5 line");
	assert!(text.contains("\t\t\tTransfer\t42.500\n"), "CPU Transfer line");
	assert!(text.trim_end().ends_with("FLOPs\tGFLOP/s"), "schema is the trailing block");
	let (eth, disk, gpu, cpu) = (
		text.find("\t\tETH\n").unwrap(),
		text.find("\t\tDISK\n").unwrap(),
		text.find("\t\tGPU0\n").unwrap(),
		text.find("\t\tCPU\n").unwrap(),
	);
	assert!(eth < disk && disk < gpu && gpu < cpu, "tier order ETH<DISK<GPU<CPU");
}
