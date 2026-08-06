use recipe::recipe;

fn main() {
	let mut arguments = std::env::args().skip(1);
	assert_eq!(arguments.next().as_deref(), Some("export"), "usage: recipe export <source.rs>");
	let source = arguments.next().expect("recipe export requires a Rust source path");
	recipe.export(source).unwrap();
}
