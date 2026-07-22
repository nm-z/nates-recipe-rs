use std::env;
use std::fs;

const SRC: &str = "engi\n\tGPU0\n\t\tVRAM\t12\n\t\tFLOPs\t380\n\tCPU\n\t\tRAM\t31\n";

fn tmp(tag: &str) -> String {
	return env::temp_dir()
		.join(format!("nrs_ogdl_style_{tag}.ogdl"))
		.to_string_lossy()
		.into_owned();
}

#[test]
fn style1_static() {
	use ogdl::*;
	let p = tmp("s1");
	fs::write(&p, SRC).expect("seed");
	ogdl.file(&p).itnl(());
	assert_eq!(format!("{}", ogdl.itnl("engi.GPU0.VRAM")), "12");
	let _added = ogdl.add("HTT", "engi.CPU");
	let _written = ogdl.itnl(()).file(&p);
	assert!(fs::read_to_string(&p).expect("read").contains("HTT"));
}

#[test]
fn style2_struct() {
	use ogdl::Ogdl;
	let p = tmp("s2");
	fs::write(&p, SRC).expect("seed");
	let graph = Ogdl::file(&p).itnl(());
	assert_eq!(format!("{}", graph.itnl("engi.GPU0.FLOPs")), "380");
	let _written = graph.itnl(()).file(&p);
	assert_eq!(fs::read_to_string(&p).expect("read"), SRC);
}

#[test]
fn style3_free() {
	let p = tmp("s3");
	fs::write(&p, SRC).expect("seed");
	let graph = ogdl::file(&p).itnl(());
	assert_eq!(format!("{}", graph.itnl("engi.CPU.RAM")), "31");
	let _written = graph.itnl(()).file(&p);
	assert_eq!(fs::read_to_string(&p).expect("read"), SRC);
}
