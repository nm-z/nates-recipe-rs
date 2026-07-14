use ogdl::del;
use ogdl::{Graph, NamedChild, Node, Ogdl};
use std::fs;
use std::sync::Arc;

const SAMPLE: &str =
	"engi\n\tGPU0\n\t\tVRAM\t12\n\t\tFLOPs\t380\n\tCPU\n\t\tRAM\t31\n";

#[test]
fn round_trip_itnl_file_itnl() {
	let dir = std::env::temp_dir();
	let p = dir.join("nrs_ogdl_spec_rt.ogdl");
	fs::write(&p, SAMPLE).expect("write");
	let ps = p.to_str().expect("utf8");
	let g = Ogdl::file(ps);
	let back = g.snapshot().serialize();
	assert_eq!(back, SAMPLE, "file -> itnl -> serialize lossless");
}

#[test]
fn select_value() {
	let g = Graph::empty();
	g.with(|r| *r = Node::parse(SAMPLE));
	assert_eq!(format!("{}", g.itnl("engi.GPU0.VRAM")), "12");
	assert_eq!(format!("{}", g.itnl("engi.CPU.RAM")), "31");
}

#[test]
fn index_and_selectors() {
	let root = Node::parse("a\n\tb\n\t\tx\n\tb\n\t\ty\n\t1\n\t\tz\n");
	let a = &root.children[0];
	assert_eq!(a.name, "a");
	assert_eq!(a[0].name, "b");
	assert_eq!(a.select("b").expect("b").children[0].name, "x");
	assert_eq!(a.select("b{2}").expect("b{2}").children[0].name, "y");
	assert_eq!(a.select("1").expect("1").children[0].name, "z");
	assert_eq!(a.select("[2]").expect("[2]").name, "b");
}

#[test]
fn handles_free_on_drop() {
	let g = Graph::empty();
	assert_eq!(Arc::strong_count(&g.root), 1);
	let g2 = g.clone();
	assert_eq!(Arc::strong_count(&g.root), 2);
	drop(g2);
	assert_eq!(Arc::strong_count(&g.root), 1);
	g.add("x", "a");
	assert_eq!(
		Arc::strong_count(&g.root),
		1,
		"chain temporaries leak no refs"
	);
}

#[test]
fn arity_forms_dispatch() {
	let g = Graph::empty();
	g.with(|r| *r = Node::parse("a\n\tx\n\ty\n"));
	let _ = g.itnl(()).itnl("a");
	let a = g.itnl("a");
	g.del(&a[0]);
	assert!(g.itnl("a").children.iter().all(|c| c.name != "x"));
	g.del(NamedChild {
		name: "y",
		parent: &a,
	});
	assert!(g.itnl("a").children.is_empty());
}

#[test]
fn add_del() {
	let g = Graph::empty();
	g.with(|r| *r = Node::parse("a\n\tb\n"));
	g.add("c", "a");
	assert!(
		g.snapshot()
			.select("a")
			.expect("a")
			.children
			.iter()
			.any(|c| c.name == "c")
	);
	del!(g, a.b {});
	assert!(
		!g.snapshot()
			.select("a")
			.expect("a")
			.children
			.iter()
			.any(|c| c.name == "b")
	);
}

#[test]
fn typed_serialize_roundtrip() {
	let g = Graph::empty();
	let weights = vec![0.01_f64, -0.02, 0.03];
	let labels = vec!["cat".to_string(), "dog".to_string()];
	g.add(weights.clone(), "z1.w")
		.add(0.5_f64, "z1.b")
		.add(42_i64, "z1.neurons")
		.add(true, "z1.bias")
		.add("relu", "z1.act")
		.add(labels.clone(), "z1.classes");
	let w: Vec<f64> = g
		.itnl("z1.w")
		.children
		.iter()
		.filter_map(|c| c.name.parse().ok())
		.collect();
	assert_eq!(w, weights);
	assert_eq!(format!("{}", g.itnl("z1.b")), "0.5");
	assert_eq!(format!("{}", g.itnl("z1.neurons")), "42");
	assert_eq!(format!("{}", g.itnl("z1.bias")), "true");
	assert_eq!(format!("{}", g.itnl("z1.act")), "relu");
	let classes: Vec<String> = g
		.itnl("z1.classes")
		.children
		.iter()
		.map(|c| c.name.clone())
		.collect();
	assert_eq!(classes, labels);
}
