#[cfg(test)]
mod pipeline_tests {
	fn fixture(name: &str) -> String {
		let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		p.push("tests/fixtures");
		p.push(name);
		p.display().to_string()
	}

	#[test]
	fn numeric_blanks_drop_rows() {
		let (train, _) = crate::dataset::Data::load()
			.set(&fixture("hw_train.csv"))
			.target("MD")
			.prepare();
		assert_eq!(train.x.ncols(), 3, "3 numeric features (A, B, C)");
		assert_eq!(train.x.nrows(), 7, "10 - 3 NaN rows");
		assert_eq!(train.x.iter().filter(|v| v.is_nan()).count(), 0);
	}

	#[test]
	fn categorical_feature_one_hot() {
		let (train, _) = crate::dataset::Data::load()
			.set(&fixture("tw_train.csv"))
			.target("TVT")
			.prepare();
		assert_eq!(train.x.ncols(), 4, "GR + 3 Geology one-hot columns");
		assert_eq!(train.x.nrows(), 6, "all rows kept");
		assert_eq!(train.x.iter().filter(|v| v.is_nan()).count(), 0);
	}

	#[test]
	fn selection_before_nan() {
		let (train, _) = crate::dataset::Data::load()
			.set(&fixture("tw_train.csv"))
			.test(&fixture("tw_test.csv"))
			.target("TVT")
			.prepare();
		assert_eq!(train.x.ncols(), 1, "only the shared GR is a feature");
		assert_eq!(
			train.x.nrows(),
			6,
			"Geology dropped before NaN, drops no rows"
		);
	}

	#[test]
	fn aligns_to_shared_columns() {
		let (train, test) = crate::dataset::Data::load()
			.set(&fixture("hw_train.csv"))
			.test(&fixture("hw_test.csv"))
			.prepare();
		let test = test.expect("test present");
		assert_eq!(train.x.ncols(), 3, "3 shared columns (A, B, MD)");
		assert_eq!(test.x.ncols(), 3);
	}

	#[test]
	fn split_keeps_all_columns() {
		let (train, test) = crate::dataset::Data::load()
			.set(&fixture("clean.csv"))
			.target("D")
			.split(0.8)
			.prepare();
		let test = test.expect("split yields a test");
		assert_eq!(train.x.ncols(), 3, "A, B, C (D is target)");
		assert_eq!(test.x.ncols(), 3);
	}

	#[test]
	fn set_only_keeps_all_columns() {
		let (train, test) = crate::dataset::Data::load()
			.set(&fixture("clean.csv"))
			.target("D")
			.prepare();
		assert_eq!(train.x.ncols(), 3, "A, B, C (D is target)");
		assert!(test.is_none());
	}
}
