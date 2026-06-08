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
                  .exclude("EventDate")
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
            assert!(matches!(kind_of(&data, "EventDate"), Kind::Temporal));
      }

      #[test]
      fn detect_categorical() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Notes")
                  .exclude("Photo")
                  .exclude("EventDate")
                  .target("MD");
            assert!(matches!(kind_of(&data, "Geology"), Kind::Categorical(_)));
      }

      #[test]
      fn detect_text() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Photo")
                  .exclude("EventDate")
                  .target("MD");
            assert!(matches!(kind_of(&data, "Notes"), Kind::Text(_)));
      }

      #[test]
      fn detect_image() {
            let data = crate::dataset::Data::load()
                  .set(&fixture())
                  .exclude("Notes")
                  .exclude("Photo")
                  .exclude("EventDate")
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
                  .exclude("EventDate")
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
                  .exclude("EventDate")
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
                  .exclude("EventDate")
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
                  .exclude("EventDate")
                  .split(0.8)
                  .target("TVT");
            let test = data.test.as_ref().expect("split yields a test");
            assert_eq!(data.set.x.ncols(), 4, "GR + 3 Geology one-hot");
            assert_eq!(test.x.ncols(), 4);
      }

      fn write_tmp(name: &str, content: &str) -> String {
            let path = format!("/tmp/recipe_test_{name}.csv");
            std::fs::write(&path, content).expect("write tmp csv");
            path
      }

      #[test]
      fn edge_sorted_numeric_with_ties() {
            let path = write_tmp("sorted_price", "\
Price,Brand
100.0,A
100.0,B
200.0,A
200.0,B
300.0,A
400.0,B
500.0,A
500.0,B
600.5,A
750.99,B
");
            let data = crate::dataset::Data::load().set(&path).target("Brand");
            assert!(
                  matches!(kind_of(&data, "Price"), Kind::Numeric),
                  "sorted continuous prices with ties are numeric, not categorical",
            );
      }

      #[test]
      fn edge_id_column_with_gaps() {
            let path = write_tmp("patient_id", "\
patient_number,diagnosis
1001,flu
1005,cold
1012,flu
1013,cold
1050,flu
1051,cold
1099,flu
1100,cold
");
            let data = crate::dataset::Data::load().set(&path).target("diagnosis");
            assert!(
                  matches!(kind_of(&data, "patient_number"), Kind::Numeric),
                  "unique integers with gaps are numeric IDs, not temporal",
            );
      }

      #[test]
      fn edge_categorical_integers() {
            let path = write_tmp("star_ratings", "\
stars,liked
5,yes
3,no
5,yes
1,no
4,yes
3,no
5,yes
2,no
4,yes
1,no
");
            let data = crate::dataset::Data::load().set(&path).target("liked");
            assert!(
                  matches!(kind_of(&data, "stars"), Kind::Categorical(_)),
                  "repeating integer ratings should be categorical",
            );
      }

      #[test]
      fn edge_date_strings_with_repeats() {
            let path = write_tmp("dates_repeat", "\
event_date,count
2024-01-15,10
2024-01-15,11
2024-02-20,20
2024-03-10,30
2024-03-10,31
2024-03-10,32
2024-04-05,40
2024-05-18,50
");
            let data = crate::dataset::Data::load().set(&path).target("count");
            assert!(
                  matches!(kind_of(&data, "event_date"), Kind::Temporal),
                  "repeating date strings are still temporal, not categorical",
            );
      }

      #[test]
      fn edge_mixed_missing_mostly_numeric() {
            let path = write_tmp("mixed_missing", "\
value,label
3.14,a
2.71,b
1.0,a
missing,b
0.5,a
9.9,b
4.2,a
7.7,b
6.28,a
unknown,b
");
            let data = crate::dataset::Data::load().set(&path).target("label");
            assert!(
                  matches!(kind_of(&data, "value"), Kind::Numeric),
                  "80% f64 with unknown string markers should be numeric",
            );
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
                  .exclude("EventDate")
                  .target("TVT");
            assert_eq!(data.set.x.ncols(), 4, "GR + 3 Geology one-hot");
            assert!(data.test.is_none());
      }
}
