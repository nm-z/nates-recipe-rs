#[cfg(test)]
mod lua_gpu_tests {
      #[test]
      fn test_lua_gpu_buffer_roundtrip() {
            gpu_core::hip::set_device(0).expect("GPU set_device failed");
            let lua = mlua::Lua::new();
            crate::lua_runtime::init(&lua).expect("init failed");

            let code = r#"
                  local buf = upload({1.0, 2.0, 3.0, 4.0, 5.0, 6.0}, 2, 3)
                  assert(buf:rows() == 2)
                  assert(buf:cols() == 3)
                  local data = download(buf)
                  assert(#data == 6)
                  assert(data[1] == 1.0)
                  assert(data[6] == 6.0)
                  return true
            "#;
            let result: bool = lua.load(code).eval().expect("Lua eval failed");
            assert!(result);
      }

      #[test]
      fn test_lua_gemm() {
            gpu_core::hip::set_device(0).expect("GPU set_device failed");
            let lua = mlua::Lua::new();
            crate::lua_runtime::init(&lua).expect("init failed");

            let code = r#"
                  local A = upload({1,2,3, 4,5,6}, 2, 3)
                  local B = upload({1,4, 2,5, 3,6}, 3, 2)
                  local C = gemm(A, B, "N", "N")
                  assert(C:rows() == 2)
                  assert(C:cols() == 2)
                  local d = download(C)
                  assert(math.abs(d[1] - 14) < 0.01, "expected 14 got " .. d[1])
                  assert(math.abs(d[2] - 32) < 0.01, "expected 32 got " .. d[2])
                  assert(math.abs(d[3] - 32) < 0.01, "expected 32 got " .. d[3])
                  assert(math.abs(d[4] - 77) < 0.01, "expected 77 got " .. d[4])
                  return true
            "#;
            let result: bool = lua.load(code).eval().expect("Lua eval failed");
            assert!(result);
      }

      #[test]
      fn test_lua_ridge_inline() {
            gpu_core::hip::set_device(0).expect("GPU set_device failed");
            let lua = mlua::Lua::new();
            crate::lua_runtime::init(&lua).expect("init failed");

            let code = r#"
                  local X = upload({1,2, 3,4, 5,6, 7,8, 9,10}, 5, 2)
                  local y = upload({5, 11, 17, 23, 29}, 5, 1)
                  local XtX = gemm(X, X, "T", "N")
                  local R = diag_add(XtX, 0.01)
                  local Xty = gemm(X, y, "T", "N")
                  local W = solve(R, Xty)
                  local yh = gemm(X, W, "N", "N")
                  local d = download(yh)
                  assert(math.abs(d[1] - 5) < 0.1, "expected ~5 got " .. d[1])
                  assert(math.abs(d[5] - 29) < 0.1, "expected ~29 got " .. d[5])
                  return true
            "#;
            let result: bool = lua.load(code).eval().expect("Lua eval failed");
            assert!(result);
      }
}

#[cfg(test)]
mod pipeline_tests {
      const TRAIN_TW: &str =
            "/home/nate/Desktop/rogii-wellbore-geology-prediction/train/000d7d20__typewell.csv";
      const TEST_TW: &str =
            "/home/nate/Desktop/rogii-wellbore-geology-prediction/test/000d7d20__typewell.csv";
      const TRAIN_HW: &str =
            "/home/nate/Desktop/rogii-wellbore-geology-prediction/train/000d7d20__horizontal_well.csv";
      const TEST_HW: &str =
            "/home/nate/Desktop/rogii-wellbore-geology-prediction/test/000d7d20__horizontal_well.csv";

      fn have(paths: &[&str]) -> bool {
            paths.iter().all(|p| std::path::Path::new(p).exists())
      }

      // Numeric blanks become NaN and drop their rows. horizontal_well has GR (2258)
      // + TVT_input (3836) blanks over 4278 rows; with MD as the (clean) target the
      // 12 numeric features carry those NaNs, so 5278-4278 = 1000 clean rows remain.
      #[test]
      fn numeric_blanks_drop_rows() {
            if !have(&[TRAIN_HW]) { return; }
            let (train, _) = crate::dataset::Data::load().set(TRAIN_HW).target("MD").prepare();
            assert_eq!(train.x.ncols(), 12, "12 numeric features (13 cols minus MD target)");
            assert_eq!(train.x.nrows(), 1000, "5278 - 4278 NaN rows");
            assert_eq!(train.x.iter().filter(|v| v.is_nan()).count(), 0, "NaN rows dropped");
      }

      // A categorical feature is one-hot encoded, not turned into NaN. typewell
      // Geology has 10 categories → with TVT as target, features = GR + 10 one-hot
      // columns = 11, no NaN introduced, all 1296 rows kept (blank Geology → all-zero).
      #[test]
      fn categorical_feature_one_hot() {
            if !have(&[TRAIN_TW]) { return; }
            let (train, _) = crate::dataset::Data::load().set(TRAIN_TW).target("TVT").prepare();
            assert_eq!(train.x.ncols(), 11, "GR + 10 Geology one-hot columns");
            assert_eq!(train.x.nrows(), 1296, "no NaN from one-hot; all rows kept");
            assert_eq!(train.x.iter().filter(|v| v.is_nan()).count(), 0);
      }

      // Selection before NaN: with a test set, Geology is train-only → excluded as a
      // feature before NaN handling, so its absence can't drop rows. Feature = GR
      // only (the shared column); GR is clean → all 1296 rows kept.
      #[test]
      fn selection_before_nan() {
            if !have(&[TRAIN_TW, TEST_TW]) { return; }
            let (train, _) = crate::dataset::Data::load()
                  .set(TRAIN_TW)
                  .test(TEST_TW)
                  .target("TVT")
                  .prepare();
            assert_eq!(train.x.ncols(), 1, "only the shared GR is a feature");
            assert_eq!(train.x.nrows(), 1296, "Geology dropped before NaN, drops no rows");
      }

      // Alignment: train 13 cols, test 6 cols, no target named → no auto-target
      // (7 train-only cols is ambiguous). Features = the 6 shared columns in both.
      #[test]
      fn aligns_to_shared_columns() {
            if !have(&[TRAIN_HW, TEST_HW]) { return; }
            let (train, test) = crate::dataset::Data::load().set(TRAIN_HW).test(TEST_HW).prepare();
            let test = test.expect("test present");
            assert_eq!(train.x.ncols(), 6, "6 shared columns");
            assert_eq!(test.x.ncols(), 6);
      }

      // split keeps every column (a row-split can't create train-only columns).
      #[test]
      fn split_keeps_all_columns() {
            if !have(&[TRAIN_HW]) { return; }
            let (train, test) = crate::dataset::Data::load().set(TRAIN_HW).split(0.8).prepare();
            let test = test.expect("split yields a test");
            assert_eq!(train.x.ncols(), 13);
            assert_eq!(test.x.ncols(), 13);
      }

      // set-only: no test → nothing to align → all columns are features, no test set.
      #[test]
      fn set_only_keeps_all_columns() {
            if !have(&[TRAIN_HW]) { return; }
            let (train, test) = crate::dataset::Data::load().set(TRAIN_HW).prepare();
            assert_eq!(train.x.ncols(), 13);
            assert!(test.is_none());
      }
}

// Directory assembly — decodes every image in the dir, so these are #[ignore]d and
// run explicitly (`cargo test -- --ignored`).
#[cfg(test)]
mod dir_assembly_tests {
      const TRAIN_DIR: &str = "/home/nate/Desktop/rogii-wellbore-geology-prediction/train";
      const TEST_DIR: &str = "/home/nate/Desktop/rogii-wellbore-geology-prediction/test";

      fn have() -> bool {
            std::path::Path::new(TRAIN_DIR).exists() && std::path::Path::new(TEST_DIR).exists()
      }

      // The group owning the target (typewell, holds Geology) defines the samples at
      // FULL ROW RESOLUTION — not one-row-per-well. horizontal_well can't hash-align
      // (5278 vs 1296 rows/well) so it's excluded; png broadcasts in the set but is
      // dropped by alignment (test has no images). Shared features = typewell GR+TVT.
      #[test]
      #[ignore]
      fn dir_target_group_defines_samples() {
            if !have() { return; }
            let (train, test) = crate::dataset::Data::load()
                  .set(TRAIN_DIR)
                  .test(TEST_DIR)
                  .target("Geology")
                  .prepare();
            let test = test.expect("test present");
            // Rows preserved, not collapsed to one-per-well (~773 wells × ~1296 rows).
            assert!(train.x.nrows() > 700_000, "got {} rows", train.x.nrows());
            assert_eq!(train.x.ncols(), 2, "typewell:GR + typewell:TVT (png aligned out)");
            assert_eq!(test.x.ncols(), 2);
      }

      // .exclude drops a whole group. Set-only on the dir, png (1/well) would
      // broadcast 3072 pixel columns onto every typewell sample; .exclude("png:*")
      // leaves only typewell GR + TVT. (Without the exclude this materializes ~25GB —
      // exactly why .exclude exists — so we assert only the excluded shape.)
      #[test]
      #[ignore]
      fn exclude_drops_image_group() {
            if !have() { return; }
            let (without, _) = crate::dataset::Data::load()
                  .set(TRAIN_DIR)
                  .target("Geology")
                  .exclude("png:*")
                  .prepare();
            assert_eq!(without.x.ncols(), 2, "png:* excluded → typewell GR + TVT");
      }
}
