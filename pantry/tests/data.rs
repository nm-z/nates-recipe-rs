use pantry::data::{DirGroup, RawCsv, load_groups, load_zip_groups, read_raw_csv, sniff_delimiter};
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
	let RawCsv { headers, rows } = read_raw_csv(&p).unwrap();
	assert_eq!(headers, vec!["col_0", "col_1", "col_2"]);
	assert_eq!(rows.len(), 3, "first numeric row must be kept, not eaten");
	assert_eq!(rows[0], vec!["1.0", "2", "3.29662E-05"]);
}

#[test]
fn named_first_row_is_header() {
	let p = tmp("named.csv", "age,city,score\n31,nyc,9.5\n");
	let RawCsv { headers, rows } = read_raw_csv(&p).unwrap();
	assert_eq!(headers, vec!["age", "city", "score"]);
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0], vec!["31", "nyc", "9.5"]);
}

#[test]
fn single_numeric_column_headerless() {
	let p = tmp("single.csv", "3.29662E-05\n1.1\n2.2\n3.3\n");
	let RawCsv { headers, rows } = read_raw_csv(&p).unwrap();
	assert_eq!(headers, vec!["col_0"]);
	assert_eq!(rows.len(), 4);
}

#[test]
fn semicolon_delimiter_is_sniffed() {
	let p = std::path::Path::new(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../datasets/uci-bank-semicolon/bank.csv"
	));
	assert_eq!(sniff_delimiter(p), b';');
}

#[test]
fn tab_delimiter_is_sniffed() {
	let p = std::path::Path::new(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../datasets/uci-seeds/seeds_dataset.txt"
	));
	assert_eq!(sniff_delimiter(p), b'\t');
}

#[test]
fn comma_delimiter_is_sniffed() {
	let p = std::path::Path::new(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../datasets/wine-quality/winequality-red.csv"
	));
	assert_eq!(sniff_delimiter(p), b',');
}

#[test]
fn space_delimiter_is_sniffed() {
	let p = std::path::Path::new(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../datasets/uci-har-sensor/UCI HAR Dataset/train/X_train.txt"
	));
	assert_eq!(sniff_delimiter(p), b' ');
}

#[test]
fn prose_line_is_not_space_delimited() {
	let p = tmp("prose.txt", "the quick brown fox\njumps over 3 dogs\n");
	assert_eq!(sniff_delimiter(&p), b',');
	let RawCsv { headers, rows } = read_raw_csv(&p).unwrap();
	assert_eq!(headers, vec!["the quick brown fox"]);
	assert_eq!(rows, vec![vec!["jumps over 3 dogs"]]);
}

#[test]
fn whitespace_matrix_headerless_keeps_rows() {
	let p = std::path::Path::new(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../datasets/uci-har-sensor/UCI HAR Dataset/train/X_train.txt"
	));
	let RawCsv { headers, rows } = read_raw_csv(p).unwrap();
	assert_eq!(headers.len(), 561);
	assert_eq!(headers[0], "col_0");
	assert_eq!(headers[560], "col_560");
	assert_eq!(rows.len(), 7352);
	assert!(rows.iter().all(|r| r.len() == 561));
	assert_eq!(rows[0][0].parse::<f64>().unwrap(), 2.8858451e-001);
}

#[test]
fn dir_groups_found_recursively() {
	let d = concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../datasets/uci-har-sensor/UCI HAR Dataset"
	);
	let names: Vec<String> = load_groups(d)
		.iter()
		.map(|g| match g {
			DirGroup::Table { name, .. } | DirGroup::Image { name, .. } => name.clone(),
		})
		.collect();
	assert!(
		names.iter().any(|n| n == "X_train"),
		"X_train group from train/ subdir: {names:?}"
	);
	assert!(
		names.iter().any(|n| n == "X_test"),
		"X_test group from test/ subdir: {names:?}"
	);
}

#[test]
fn har_zip_extracts_wrapped_tables() {
	let p = concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../datasets/uci-har-sensor/har.zip"
	);
	let groups = load_zip_groups(p).unwrap();
	let mut x_train_cols = 0usize;
	let mut y_train_rows = 0usize;
	for g in &groups {
		if let DirGroup::Table {
			name,
			headers,
			cells,
			..
		} = g
		{
			assert!(!name.starts_with('.'), "junk group leaked: {name}");
			if name == "X_train" {
				x_train_cols = headers.len();
			}
			if name == "y_train" {
				y_train_rows = cells.len();
			}
		}
	}
	assert_eq!(x_train_cols, 561, "X_train parsed as a 561-column matrix");
	assert_eq!(y_train_rows, 7352, "y_train rows survive extraction");
}

#[test]
fn nested_zip_extracts_inner_tables() {
	let inner_path =
		std::env::temp_dir().join(format!("nrs_zip_inner_{}.zip", std::process::id()));
	let outer_path =
		std::env::temp_dir().join(format!("nrs_zip_outer_{}.zip", std::process::id()));
	let opts = zip::write::SimpleFileOptions::default();
	let mut inner = zip::ZipWriter::new(std::fs::File::create(&inner_path).unwrap());
	inner.start_file("wrap/points.csv", opts).unwrap();
	inner.write_all(b"a,b\n1,2\n3,4\n").unwrap();
	inner.finish().unwrap();
	let mut outer = zip::ZipWriter::new(std::fs::File::create(&outer_path).unwrap());
	outer.start_file("bundle/inner.zip", opts).unwrap();
	outer.write_all(&std::fs::read(&inner_path).unwrap())
		.unwrap();
	outer.start_file("__MACOSX/bundle/._inner.zip", opts)
		.unwrap();
	outer.write_all(b"applejunk").unwrap();
	outer.start_file("bundle/.DS_Store", opts).unwrap();
	outer.write_all(b"macjunk").unwrap();
	outer.finish().unwrap();
	let groups = load_zip_groups(outer_path.to_str().unwrap()).unwrap();
	let _ = std::fs::remove_file(&inner_path);
	let _ = std::fs::remove_file(&outer_path);
	assert_eq!(groups.len(), 1, "junk entries must not become groups");
	let DirGroup::Table {
		name,
		headers,
		cells,
		..
	} = &groups[0]
	else {
		panic!("expected a table group");
	};
	assert_eq!(name, "points");
	assert_eq!(headers, &["a", "b"]);
	assert_eq!(
		cells,
		&[
			vec!["1".to_string(), "2".to_string()],
			vec!["3".to_string(), "4".to_string()]
		]
	);
}
