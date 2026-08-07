use recipe::{ArtifactSet, recipe}; fn main() {
	let mut arguments = std::env::args().skip(1);
	let command = arguments.next(); if command.as_deref() == Some("devices") {
		assert!(arguments.next().is_none(), "usage: recipe devices"); for name in recipe.devices() {
			println!("{name}"); }
		return; }
	assert_eq!(command.as_deref(), Some("export"), "usage: recipe export <source.rs>");
	let source = arguments.next().expect("recipe export requires a Rust source path");
	let selection = match arguments.next().as_deref() { None => ArtifactSet::Auto, Some("--amd") => ArtifactSet::Amd,
		Some("--nvidia") => ArtifactSet::Nvidia, _ => panic!("usage: recipe export <source.rs> [--amd|--nvidia]"), };
	assert!(arguments.next().is_none(), "usage: recipe export <source.rs> [--amd|--nvidia]");
	recipe.export(source, selection).unwrap(); }
