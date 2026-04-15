use crate::{Mat, Vec1};
use ndarray::{Array1, Array2};
use polars::prelude::*;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use anyhow::{Context, Result};
use image::imageops::FilterType;
use std::fs;
use std::path::Path;

fn read_csv(path: &str, ragged: bool) -> Result<DataFrame> {
    let opts = CsvReadOptions::default()
        .with_has_header(true)
        .map_parse_options(|o| o.with_null_values(Some(NullValues::AllColumns(vec!["NA".into(), "NaN".into(), "nan".into(), "".into()]))));
    let opts = if ragged { opts.map_parse_options(|o| o.with_truncate_ragged_lines(true)) } else { opts };
    Ok(opts.try_into_reader_with_file_path(Some(path.into()))?.finish()?)
}

pub fn load_csv(predictors_path: &str, targets_path: &str) -> Result<(Mat, Vec1)> {
    let x_df = read_csv(predictors_path, true)?;
    let y_df = read_csv(targets_path, false)?;

    let x = df_to_array2(&x_df)?;
    let y_series = y_df.get_columns()[0].cast(&DataType::Float64)?;
    let y_ca = y_series.f64()?;
    let y_col: Vec1 = y_ca.into_iter().map(|v| v.unwrap_or(f64::NAN)).collect();
    Ok((x, y_col))
}

/// Label-encode string columns, cast everything to f64, then use Polars' built-in to_ndarray.
fn df_to_array2(df: &DataFrame) -> Result<Mat> {
    let encoded: Vec<Column> = df.get_columns().iter().map(|col| {
        match col.cast(&DataType::Float64) {
            Ok(s) => s.into_column(),
            Err(_cast_err) => {
                // Frequency-encode string/categorical columns:
                // each category → its count in the column. No artificial ordinal relationship.
                let str_col = col.str().expect("column expected to be string dtype");
                let mut counts: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
                for v in str_col.into_iter().flatten() {
                    *counts.entry(v.to_string()).or_insert(0.0) += 1.0;
                }
                let codes: Float64Chunked = str_col.into_iter()
                    .map(|v| match v {
                        Some(s) => Some(*counts.get(s).expect("freq encode")),
                        None => None,
                    })
                    .collect_ca(col.name().clone());
                codes.into_column()
            }
        }
    }).collect();
    let numeric_df = DataFrame::new(encoded)?;
    Ok(numeric_df.to_ndarray::<Float64Type>(IndexOrder::Fortran)?)
}

pub fn train_test_split(
    x: &Mat,
    y: &Vec1,
    test_size: f64,
    seed: u64,
) -> (Mat, Mat, Vec1, Vec1) {
    let n = x.nrows();
    let n_test = (n as f64 * test_size).round() as usize;
    let n_train = n - n_test;

    let mut indices: Vec<usize> = (0..n).collect();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    indices.shuffle(&mut rng);

    let train_idx = &indices[..n_train];
    let test_idx = &indices[n_train..];

    let x_train = x.select(ndarray::Axis(0), train_idx);
    let x_test = x.select(ndarray::Axis(0), test_idx);
    let y_train = y.select(ndarray::Axis(0), train_idx);
    let y_test = y.select(ndarray::Axis(0), test_idx);

    (x_train, x_test, y_train, y_test)
}

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "bmp", "gif", "webp", "tiff", "tif", "ico", "pnm", "pbm", "pgm", "ppm",
    "qoi", "dds", "hdr", "exr", "ff",
];

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

fn collect_image_paths(dir: &str) -> Result<Vec<std::path::PathBuf>> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {dir}"))?
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image_file(p))
        .collect();
    paths.sort();
    Ok(paths)
}

/// Load a single image, resize to `width x height`, and return as a flattened `Vec1`.
pub fn image_to_row(path: &str, width: u32, height: u32) -> Result<Vec1> {
    let img = image::open(path).with_context(|| format!("failed to open image: {path}"))?;
    let resized = image::imageops::resize(&img, width, height, FilterType::Triangle);
    let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();
    let raw = rgb.into_raw();
    let row: Vec1 = raw.into_iter().map(|v| v as f64).collect();
    Ok(row)
}

/// Load all images from `dir`, resize each to `width x height`, and return as `Mat`
/// where each row is one flattened RGB image (length = width * height * 3).
pub fn load_image_dir(dir: &str, width: u32, height: u32) -> Result<Mat> {
    let paths = collect_image_paths(dir)?;
    anyhow::ensure!(!paths.is_empty(), "no image files found in {dir}");

    let row_len = (width * height * 3) as usize;
    let mut data = Vec::with_capacity(paths.len() * row_len);

    for path in &paths {
        let row = image_to_row(path.to_str().expect("image path is not valid UTF-8"), width, height)?;
        data.extend(row.iter());
    }

    let n = data.len() / row_len;
    Ok(Array2::from_shape_vec((n, row_len), data)?)
}

/// Load images from a labeled directory structure. Expects subdirectories as labels:
/// - Float names: `dir/0.0/`, `dir/1.0/` -- used directly as labels
/// - String names: `dir/cat/`, `dir/dog/` -- label-encoded alphabetically (0.0, 1.0, ...)
///
/// Returns `(X, y)` where X rows are flattened RGB images, y are float labels.
pub fn load_labeled_image_dir(
    dir: &str,
    width: u32,
    height: u32,
) -> Result<(Mat, Vec1)> {
    let mut subdirs: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {dir}"))?
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();
    anyhow::ensure!(!subdirs.is_empty(), "no subdirectories found in {dir}");

    // Determine labels: try parsing subdir names as f64, otherwise label-encode alphabetically
    let names: Vec<String> = subdirs
        .iter()
        .map(|p| p.file_name().expect("subdir path has no filename component").to_string_lossy().into_owned())
        .collect();
    let all_float = names.iter().all(|n| n.parse::<f64>().is_ok());

    let label_map: Vec<f64> = if all_float {
        names.iter().map(|n| n.parse().expect("subdir name failed f64 parse after all_float check")).collect()
    } else {
        (0..names.len()).map(|i| i as f64).collect()
    };

    let row_len = (width * height * 3) as usize;
    let mut data = Vec::new();
    let mut labels = Vec::new();

    for (subdir, &label) in subdirs.iter().zip(label_map.iter()) {
        let subdir_str = subdir.to_str().expect("subdir path is not valid UTF-8");
        let paths = collect_image_paths(subdir_str)?;
        for path in &paths {
            let row = image_to_row(path.to_str().expect("image path is not valid UTF-8"), width, height)?;
            data.extend(row.iter());
            labels.push(label);
        }
    }

    anyhow::ensure!(!labels.is_empty(), "no images found in any subdirectory of {dir}");
    let n = labels.len();
    let x = Array2::from_shape_vec((n, row_len), data)?;
    let y = Array1::from_vec(labels);
    Ok((x, y))
}
