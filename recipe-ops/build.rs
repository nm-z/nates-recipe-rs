use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
	println!("cargo:rerun-if-changed=../operation-surface.txt");
	let source = fs::read_to_string("../operation-surface.txt").expect("operation-surface.txt must be readable");
	let mut parsed = Vec::new();
	let mut totals = BTreeMap::<String, u16>::new();
	for (index, raw_line) in source.lines().enumerate() {
		let line_number = index + 1;
		let line = raw_line.trim_end();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let mut fields = line.split('\t');
		let symbol = fields.next().expect("split always yields one field");
		let source = fields
			.next()
			.unwrap_or_else(|| panic!("operation-surface.txt:{line_number}: missing source field"));
		assert!(
			fields.next().is_none(),
			"operation-surface.txt:{line_number}: expected exactly two tab-separated fields"
		);
		assert!(
			!symbol.is_empty() && !source.is_empty(),
			"operation-surface.txt:{line_number}: fields must be nonempty"
		);
		let total = totals.entry(symbol.to_owned()).or_default();
		*total = total
			.checked_add(1)
			.expect("symbol occurrence count fits u16");
		parsed.push((line_number, symbol.to_owned(), source.to_owned()));
	}

	let mut seen = BTreeMap::<String, u16>::new();
	let mut generated = String::from("pub(crate) const RAW_OPERATION_SURFACE: &[RawSurfaceEntry] = &[\n");
	for (ordinal, (line, symbol, source)) in parsed.iter().enumerate() {
		let occurrence = seen.entry(symbol.clone()).or_default();
		*occurrence = occurrence
			.checked_add(1)
			.expect("symbol occurrence count fits u16");
		let total = totals[symbol];
		generated.push_str(&format!(
			"    RawSurfaceEntry {{ ordinal: {ordinal}, line: {line}, symbol: {symbol:?}, source: {source:?}, occurrence: {occurrence}, occurrences: {total} }},\n"
		));
	}
	generated.push_str("];\n");
	generated.push_str(&format!(
		"pub(crate) const RAW_OPERATION_COUNT: usize = {};\n",
		parsed.len()
	));

	let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo provides OUT_DIR")).join("operation_surface.rs");
	fs::write(output, generated).expect("generated operation inventory must be writable");
}
