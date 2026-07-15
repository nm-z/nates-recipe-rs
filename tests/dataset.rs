use pantry::Kind;
use recipe::dataset::{Data, Datasets, SafeTable, safetensors_to_table};
use std::env;
use std::fs;
use std::process;

#[test]
fn data_load_reads_safetensors_source() {
	let header = concat!(
		r#"{"x":{"dtype":"F64","shape":[3,2],"data_offsets":[0,48]},"#,
		r#""y":{"dtype":"F64","shape":[3],"data_offsets":[48,72]}}"#,
	);
	let mut bytes = Vec::new();
	bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
	bytes.extend_from_slice(header.as_bytes());
	for v in [1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0] {
		bytes.extend_from_slice(&v.to_le_bytes());
	}
	for v in [10.0_f64, 20.0, 30.0] {
		bytes.extend_from_slice(&v.to_le_bytes());
	}
	let path = env::temp_dir().join("recipe_st_source_test.safetensors");
	fs::write(&path, &bytes).expect("write temp safetensors");
	let p = path.to_str().expect("temp path utf8");

	let SafeTable { attrs, rows } = safetensors_to_table(p).expect("safetensors table");
	assert_eq!(
		attrs.iter()
			.map(|a| return a.name.as_str())
			.collect::<Vec<_>>(),
		vec!["x:0", "x:1", "y"]
	);
	assert!(attrs.iter().all(|a| matches!(a.kind, Kind::Numeric)));
	assert_eq!(rows.len(), 3);
	assert_eq!(rows[0], vec!["1", "2", "10"]);
	assert_eq!(rows[2], vec!["5", "6", "30"]);

	let data = Data::load(p).split(0.66).target("y");
	let Datasets { train: set, test } = data.datasets();
	assert_eq!(set.x.ncols(), 2, "two feature columns (x:0, x:1)");
	assert_eq!(set.n_targets, 1, "single target (y)");
	let total = set.x.nrows() + test.as_ref().map_or(0, |t| return t.x.nrows());
	assert_eq!(total, 3, "all rows preserved across the split");
	let _ = fs::remove_file(&path);
}

#[test]
fn data_load_materializes_safetensors_shard() {
	let header = concat!(
		r#"{"blk.0.w":{"dtype":"F32","shape":[4,2],"data_offsets":[0,32]},"#,
		r#""blk.0.b":{"dtype":"F64","shape":[4],"data_offsets":[32,64]},"#,
		r#""label":{"dtype":"F64","shape":[4],"data_offsets":[64,96]}}"#,
	);
	let mut bytes = Vec::new();
	bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
	bytes.extend_from_slice(header.as_bytes());
	for v in [10.0_f32, 11.0, 20.0, 21.0, 30.0, 31.0, 40.0, 41.0] {
		bytes.extend_from_slice(&v.to_le_bytes());
	}
	for v in [100.0_f64, 200.0, 300.0, 400.0] {
		bytes.extend_from_slice(&v.to_le_bytes());
	}
	for v in [1.0_f64, 2.0, 3.0, 4.0] {
		bytes.extend_from_slice(&v.to_le_bytes());
	}
	let path = env::temp_dir().join(format!("recipe_st_shard_{}.safetensors", process::id()));
	fs::write(&path, &bytes).expect("write temp safetensors shard");
	let p = path.to_str().expect("temp path utf8");

	let data = Data::load(p).target("label");
	let Datasets { train: set, test } = data.datasets();
	assert_eq!(set.x.nrows(), 4, "four rows from the shared leading dim");
	assert_eq!(
		set.x.ncols(),
		3,
		"three feature cols (blk.0.w:0, blk.0.w:1, blk.0.b)"
	);
	assert_eq!(set.n_targets, 1, "label is the single target");
	assert!(
		test.is_none(),
		"no split and no test set yields one dataset"
	);
	assert_eq!(
		set.x[[0, 0]],
		10.0,
		"F32 weight widened to f64, column order kept"
	);
	assert_eq!(
		set.x[[3, 2]],
		400.0,
		"F64 bias, last feature column, last row"
	);
	let _ = fs::remove_file(&path);
}

#[test]
fn no_target_datasets_is_features_only() {
	let d = Data::load("datasets/uci-wine/wine.data");
	let Datasets { train: set, test } = d.datasets();
	assert_eq!(set.n_targets, 0, "no .target() yields zero targets");
	assert!(set.y.is_empty(), "zero targets carry no y");
	assert_eq!(set.x.nrows(), 178, "every wine row survives");
	assert_eq!(set.x.ncols(), 14, "every wine column becomes a feature");
	assert!(
		test.is_none(),
		"no split and no test path yields one dataset"
	);
}
