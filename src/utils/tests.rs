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
mod data_tests {
      use std::fs;
      use std::path::{Path, PathBuf};
      use std::time::{SystemTime, UNIX_EPOCH};

      use image::{Rgb, RgbImage};

      fn unique_test_dir(prefix: &str) -> PathBuf {
            let nanos = SystemTime::now()
                  .duration_since(UNIX_EPOCH)
                  .expect("system clock before unix epoch")
                  .as_nanos();
            let dir = std::env::temp_dir().join(format!("nates_recipe_{prefix}_{nanos}"));
            fs::create_dir_all(&dir).expect("failed to create temp dir");
            dir
      }

      fn write_rgb_image(path: &Path, pixels: &[(u8, u8, u8)], width: u32, height: u32) {
            let mut img = RgbImage::new(width, height);
            for (idx, (r, g, b)) in pixels.iter().copied().enumerate() {
                  let x = (idx as u32) % width;
                  let y = (idx as u32) / width;
                  img.put_pixel(x, y, Rgb([r, g, b]));
            }
            img.save(path).expect("failed to save test image");
      }

      #[test]
      fn test_load_csv_numeric_and_categorical_columns() {
            let dir = unique_test_dir("load_csv");
            let x_path = dir.join("x.csv");
            let y_path = dir.join("y.csv");

            fs::write(
                  &x_path,
                  "num,cat\n1,a\n2,b\n3,a\n",
            )
            .expect("failed writing predictor csv");
            fs::write(&y_path, "target\n10\n20\n30\n").expect("failed writing target csv");

            let (x, y) = crate::data::load_csv(
                  x_path.to_str().expect("utf8 path"),
                  y_path.to_str().expect("utf8 path"),
            )
            .expect("load_csv failed");

            assert_eq!(x.nrows(), 3);
            assert_eq!(x.ncols(), 2);
            assert_eq!(y.len(), 3);
            assert!((x[[0, 0]] - 1.0).abs() < 1e-12);
            assert!((x[[1, 0]] - 2.0).abs() < 1e-12);
            assert!((x[[0, 1]] - 2.0).abs() < 1e-12);
            assert!((x[[1, 1]] - 1.0).abs() < 1e-12);
            assert!((x[[2, 1]] - 2.0).abs() < 1e-12);
            assert!((y[0] - 10.0).abs() < 1e-12);
            assert!((y[2] - 30.0).abs() < 1e-12);
      }

      #[test]
      fn test_train_test_split_is_deterministic_and_preserves_lengths() {
            let x = ndarray::arr2(&[
                  [1.0, 10.0],
                  [2.0, 20.0],
                  [3.0, 30.0],
                  [4.0, 40.0],
                  [5.0, 50.0],
            ]);
            let y = ndarray::arr1(&[100.0, 200.0, 300.0, 400.0, 500.0]);

            let (x_train_a, x_test_a, y_train_a, y_test_a) = crate::data::train_test_split(&x, &y, 0.4, 42);
            let (x_train_b, x_test_b, y_train_b, y_test_b) = crate::data::train_test_split(&x, &y, 0.4, 42);

            assert_eq!(x_train_a.nrows(), 3);
            assert_eq!(x_test_a.nrows(), 2);
            assert_eq!(y_train_a.len(), 3);
            assert_eq!(y_test_a.len(), 2);

            assert_eq!(x_train_a, x_train_b);
            assert_eq!(x_test_a, x_test_b);
            assert_eq!(y_train_a, y_train_b);
            assert_eq!(y_test_a, y_test_b);
      }

      #[test]
      fn test_image_to_row_flattens_rgb_values() {
            let dir = unique_test_dir("image_to_row");
            let path = dir.join("pixels.png");
            write_rgb_image(&path, &[(10, 20, 30), (40, 50, 60)], 2, 1);

            let row = crate::data::image_to_row(path.to_str().expect("utf8 path"), 2, 1)
                  .expect("image_to_row failed");

            assert_eq!(row.len(), 6);
            assert_eq!(row[0], 10.0);
            assert_eq!(row[1], 20.0);
            assert_eq!(row[2], 30.0);
            assert_eq!(row[3], 40.0);
            assert_eq!(row[4], 50.0);
            assert_eq!(row[5], 60.0);
      }

      #[test]
      fn test_load_image_dir_orders_files_and_ignores_non_images() {
            let dir = unique_test_dir("load_image_dir");
            let a_path = dir.join("a.png");
            let b_path = dir.join("b.png");
            let txt_path = dir.join("notes.txt");

            write_rgb_image(&a_path, &[(1, 2, 3)], 1, 1);
            write_rgb_image(&b_path, &[(9, 8, 7)], 1, 1);
            fs::write(&txt_path, "not an image").expect("failed writing text file");

            let x = crate::data::load_image_dir(dir.to_str().expect("utf8 path"), 1, 1)
                  .expect("load_image_dir failed");

            assert_eq!(x.nrows(), 2);
            assert_eq!(x.ncols(), 3);
            assert_eq!(x[[0, 0]], 1.0);
            assert_eq!(x[[0, 1]], 2.0);
            assert_eq!(x[[0, 2]], 3.0);
            assert_eq!(x[[1, 0]], 9.0);
            assert_eq!(x[[1, 1]], 8.0);
            assert_eq!(x[[1, 2]], 7.0);
      }

      #[test]
      fn test_load_image_dir_returns_error_for_empty_image_dir() {
            let dir = unique_test_dir("empty_image_dir");
            let err = crate::data::load_image_dir(dir.to_str().expect("utf8 path"), 1, 1)
                  .expect_err("expected error for directory with no images");
            assert!(err.to_string().contains("no image files found"));
      }

      #[test]
      fn test_load_labeled_image_dir_with_numeric_directory_names() {
            let root = unique_test_dir("labeled_numeric");
            let d1 = root.join("1.5");
            let d2 = root.join("2.0");
            fs::create_dir_all(&d1).expect("create dir 1");
            fs::create_dir_all(&d2).expect("create dir 2");
            write_rgb_image(&d1.join("a.png"), &[(11, 12, 13)], 1, 1);
            write_rgb_image(&d2.join("b.png"), &[(21, 22, 23)], 1, 1);

            let (x, y) = crate::data::load_labeled_image_dir(root.to_str().expect("utf8 path"), 1, 1)
                  .expect("load_labeled_image_dir failed");

            assert_eq!(x.nrows(), 2);
            assert_eq!(x.ncols(), 3);
            assert_eq!(y.len(), 2);
            assert_eq!(y[0], 1.5);
            assert_eq!(y[1], 2.0);
      }

      #[test]
      fn test_load_labeled_image_dir_with_string_directory_names() {
            let root = unique_test_dir("labeled_strings");
            let cat = root.join("cat");
            let dog = root.join("dog");
            fs::create_dir_all(&cat).expect("create cat");
            fs::create_dir_all(&dog).expect("create dog");
            write_rgb_image(&cat.join("1.png"), &[(1, 1, 1)], 1, 1);
            write_rgb_image(&dog.join("1.png"), &[(2, 2, 2)], 1, 1);

            let (_x, y) = crate::data::load_labeled_image_dir(root.to_str().expect("utf8 path"), 1, 1)
                  .expect("load_labeled_image_dir failed");

            assert_eq!(y.len(), 2);
            assert_eq!(y[0], 0.0);
            assert_eq!(y[1], 1.0);
      }

      #[test]
      fn test_load_labeled_image_dir_returns_error_when_no_subdirs() {
            let root = unique_test_dir("labeled_no_subdirs");
            let err = crate::data::load_labeled_image_dir(root.to_str().expect("utf8 path"), 1, 1)
                  .expect_err("expected error for no label subdirectories");
            assert!(err.to_string().contains("no subdirectories found"));
      }
}
