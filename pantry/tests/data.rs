use pantry::data::{read_raw_csv, sniff_delimiter};
use std::io::Write as _;

fn tmp(name: &str, body: &str) -> std::path::PathBuf {
	let p = std::env::temp_dir().join(format!("nrs_hdr_{}_{name}", std::process::id()));
	let mut f = std::fs::File::create(&p).unwrap();
	f.write_all(body.as_bytes()).unwrap();
	p
}

#[test]
fn headerless_numeric_first_row_is_data() {
	let p = tmp("numeric.csv", "1.0,2,3.29662E-05\n4,5,6\n-7,8.5,9\n");
	let (headers, rows) = read_raw_csv(&p).unwrap();
	assert_eq!(headers, vec!["col_0", "col_1", "col_2"]);
	assert_eq!(rows.len(), 3, "first numeric row must be kept, not eaten");
	assert_eq!(rows[0], vec!["1.0", "2", "3.29662E-05"]);
}

#[test]
fn named_first_row_is_header() {
	let p = tmp("named.csv", "age,city,score\n31,nyc,9.5\n");
	let (headers, rows) = read_raw_csv(&p).unwrap();
	assert_eq!(headers, vec!["age", "city", "score"]);
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0], vec!["31", "nyc", "9.5"]);
}

#[test]
fn single_numeric_column_headerless() {
	let p = tmp("single.csv", "3.29662E-05\n1.1\n2.2\n3.3\n");
	let (headers, rows) = read_raw_csv(&p).unwrap();
	assert_eq!(headers, vec!["col_0"]);
	assert_eq!(rows.len(), 4);
}

#[test]
fn semicolon_delimiter_is_sniffed() {
	let p = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../datasets/uci-bank-semicolon/bank.csv"));
	assert_eq!(sniff_delimiter(p), b';');
}

#[test]
fn tab_delimiter_is_sniffed() {
	let p = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../datasets/uci-seeds/seeds_dataset.txt"));
	assert_eq!(sniff_delimiter(p), b'\t');
}

#[test]
fn comma_delimiter_is_sniffed() {
	let p = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../datasets/wine-quality/winequality-red.csv"));
	assert_eq!(sniff_delimiter(p), b',');
}
