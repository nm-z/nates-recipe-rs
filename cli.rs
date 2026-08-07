use recipe::recipe; fn main() {
	let mut arguments = std::env::args().skip(1);
	let command = arguments.next(); if command.as_deref() == Some("devices") {
		assert!(arguments.next().is_none(), "usage: recipe devices"); for name in recipe.devices() {
			println!("{name}"); }
		return; }
	assert_eq!(command.as_deref(), Some("export"), "usage: recipe export <source.rs>");
	let source = arguments.next().expect("recipe export requires a Rust source path");
	assert!(arguments.next().is_none(), "usage: recipe export <source.rs>");
	recipe.export(source).unwrap(); }
