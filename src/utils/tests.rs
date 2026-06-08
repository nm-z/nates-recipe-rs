#[cfg(test)]
mod pipeline_tests {
      fn fixture() -> String {
            let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.push("tests/fixture.csv");
            p.display().to_string()
      }

      use crate::dataset::Kind;

      fn kind_of<'a>(data: &'a crate::dataset::Data, col: &str) -> &'a Kind {
            &data.attrs.iter().find(|a| a.name == col).expect(col).kind
      }

      #[test]
      fn detect_numeric() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("MD");
            assert!(matches!(kind_of(&data, "A"), Kind::Numeric));
            assert!(matches!(kind_of(&data, "B"), Kind::Numeric));
      }

      #[test]
      fn detect_temporal() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("MD");
            assert!(matches!(kind_of(&data, "D"), Kind::Temporal));
            assert!(matches!(kind_of(&data, "GR"), Kind::Temporal));
            assert!(matches!(kind_of(&data, "TVT"), Kind::Temporal));
      }

      #[test]
      fn detect_categorical() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("MD");
            assert!(matches!(kind_of(&data, "Geology"), Kind::Nominal(_)));
      }

      #[test]
      fn detect_text() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Photo")
                  .target("MD");
            assert!(matches!(kind_of(&data, "Notes"), Kind::Text(_)));
      }

      #[test]
      fn detect_image() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("MD");
            assert!(matches!(kind_of(&data, "Photo"), Kind::Image));
      }

      #[test]
      fn numeric_blanks_drop_rows() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("D")
                  .exclude("GR")
                  .exclude("Geology")
                  .exclude("TVT")
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("MD");
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
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("TVT");
            assert_eq!(data.set.x.ncols(), 4, "GR + 3 Geology one-hot columns");
            assert_eq!(data.set.x.nrows(), 10, "all rows kept");
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
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("MD");
            assert_eq!(data.set.x.ncols(), 2, "A, C (B excluded)");
            assert_eq!(data.set.x.nrows(), 9, "only C-blank row dropped");
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
                  .exclude("Notes")
                  .exclude("Photo")
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
                  .exclude("Notes")
                  .exclude("Photo")
                  .target("TVT");
            assert_eq!(data.set.x.ncols(), 4, "GR + 3 Geology one-hot");
            assert!(data.test.is_none());
      }
}
