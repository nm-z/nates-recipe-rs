// The three import styles, each from a real consumer crate — NO `use ogdl::Chain`
// anywhere (the four methods are inherent on the handle). All read and write the
// same graph via itnl/file/add/del and check the value back.

const SRC: &str = "engi\n\tGPU0\n\t\tVRAM\t12\n\t\tFLOPs\t380\n\tCPU\n\t\tRAM\t31\n";

fn tmp(tag: &str) -> String {
	std::env::temp_dir()
		.join(format!("nrs_ogdl_style_{tag}.ogdl"))
		.to_str()
		.expect("utf8")
		.to_string()
}

// ── style 1: static (dot syntax) ──
#[test]
fn style1_static() {
	use ogdl::*;
	let p = tmp("s1");
	std::fs::write(&p, SRC).expect("seed");
	ogdl.file(&p).itnl(()); // start of chain: read the file in
	assert_eq!(format!("{}", ogdl.itnl("engi.GPU0.VRAM")), "12");
	ogdl.add("HTT", "engi.CPU"); // add child "HTT" under engi.CPU
	ogdl.itnl(()).file(&p); // end of chain: write it back out
	assert!(std::fs::read_to_string(&p).expect("read").contains("HTT"));
}

// ── style 2: struct import (associated fn) — only `use ogdl::Ogdl` ──
#[test]
fn style2_struct() {
	use ogdl::Ogdl;
	let p = tmp("s2");
	std::fs::write(&p, SRC).expect("seed");
	let graph = Ogdl::file(&p).itnl(());
	assert_eq!(format!("{}", graph.itnl("engi.GPU0.FLOPs")), "380");
	graph.itnl(()).file(&p);
	assert_eq!(std::fs::read_to_string(&p).expect("read"), SRC);
}

// ── style 3: crate path (free fn) — no `use` at all ──
#[test]
fn style3_free() {
	let p = tmp("s3");
	std::fs::write(&p, SRC).expect("seed");
	let graph = ogdl::file(&p).itnl(());
	assert_eq!(format!("{}", graph.itnl("engi.CPU.RAM")), "31");
	graph.itnl(()).file(&p);
	assert_eq!(std::fs::read_to_string(&p).expect("read"), SRC);
}
