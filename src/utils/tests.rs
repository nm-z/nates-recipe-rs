#[cfg(test)]
mod pipeline_tests {
      fn fixture() -> String {
            let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.push("tests/fixture.csv");
            p.display().to_string()
      }

      #[test]
      fn numeric_blanks_drop_rows() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("D")
                  .exclude("GR")
                  .exclude("Geology")
                  .exclude("TVT")
                  .target("MD");
            assert_eq!(data.set.x.ncols(), 3, "3 numeric features (A, B, C)");
            assert_eq!(data.set.x.nrows(), 7, "10 - 3 NaN rows");
            assert_eq!(data.set.x.iter().filter(|v| v.is_nan()).count(), 0);
      }

      #[test]
      fn categorical_feature_one_hot() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("A")
                  .exclude("B")
                  .exclude("C")
                  .exclude("D")
                  .exclude("MD")
                  .target("TVT");
            assert_eq!(data.set.x.ncols(), 4, "GR + 3 Geology one-hot columns");
            assert_eq!(data.set.x.nrows(), 10, "all rows kept");
            assert_eq!(data.set.x.iter().filter(|v| v.is_nan()).count(), 0);
      }

      #[test]
      fn exclude_before_nan() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("B")
                  .exclude("D")
                  .exclude("GR")
                  .exclude("Geology")
                  .exclude("TVT")
                  .target("MD");
            assert_eq!(data.set.x.ncols(), 2, "A, C (B excluded)");
            assert_eq!(data.set.x.nrows(), 9, "only C-blank row dropped, B blanks irrelevant");
      }

      #[test]
      fn split_keeps_all_columns() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("A")
                  .exclude("B")
                  .exclude("C")
                  .exclude("D")
                  .exclude("MD")
                  .split(0.8)
                  .target("TVT");
            let test = data.test.as_ref().expect("split yields a test");
            assert_eq!(data.set.x.ncols(), 4, "GR + 3 Geology one-hot");
            assert_eq!(test.x.ncols(), 4);
      }

      #[test]
      fn set_only_no_test() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("A")
                  .exclude("B")
                  .exclude("C")
                  .exclude("D")
                  .exclude("MD")
                  .target("TVT");
            assert_eq!(data.set.x.ncols(), 4, "GR + 3 Geology one-hot");
            assert!(data.test.is_none());
      }
}
