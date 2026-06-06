use crate::{Mat, Vec1};
use ndarray::{Array1, Array2};
use polars::prelude::*;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Context, Result};
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

/// Correlate the files in `dir` into samples by the hash in their filename:
/// the part before `__` (`000d7d20__horizontal_well.csv` → `000d7d20`), or the
/// file stem when there's no `__` (`000d7d20.png` → `000d7d20`). Every file
/// sharing a hash is one sample's set of files. Returns `(hash, files)` sorted
/// Read a CSV into raw string cells (nulls → ""), preserving every row. No type
/// inference or encoding here — schema inference and encoding happen later, once
/// the target/feature roles are known. Returns (headers, rows).
pub(crate) fn read_raw_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let df = read_csv(path.to_str().unwrap_or_default(), true)?;
    let headers: Vec<String> = df.get_column_names().iter().map(|c| c.to_string()).collect();
    let h = df.height();
    let w = df.width();
    let mut rows = vec![vec![String::new(); w]; h];
    for (j, col) in df.get_columns().iter().enumerate() {
        let s = col.cast(&DataType::String)?;
        let ca = s.str()?;
        for (i, v) in ca.into_iter().enumerate() {
            rows[i][j] = v.unwrap_or("").to_string();
        }
    }
    Ok((headers, rows))
}

/// A file's `(group, hash)` given the set of hashes seen as `__` prefixes:
/// - `{hash}__{group}.ext`  → group = `group.ext`, hash = `{hash}` (rogii CSVs).
/// - `{hash}.ext` where `{hash}` is a known prefix → group = ext, hash = `{hash}`
///   (a hash-correlated extra, e.g. rogii `000d7d20.png` → group `png`).
/// - any other `{stem}.ext` → group = `{stem}`, hash = `{stem}`: a STANDALONE table,
///   its own group (so a relational dump of unrelated CSVs — Cities.csv,
///   SampleSubmission.csv … — is NOT collapsed into one `csv` group).
fn group_and_hash(p: &Path, prefixes: &std::collections::HashSet<String>) -> (String, String) {
    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    if let Some((h, rest)) = name.split_once("__") {
        return (rest.to_string(), h.to_string());
    }
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name).to_string();
    if prefixes.contains(&stem) {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or(name).to_string();
        return (ext, stem);
    }
    (stem.clone(), stem)
}

/// A directory parsed into groups by file type (`feature_group`): a Table holds
/// every CSV row tagged by its hash (no collapse, no aggregation — rows are
/// samples), an Image holds one flattened 32x32 RGB row per file, tagged by hash.
pub enum DirGroup {
    Table { name: String, headers: Vec<String>, hashes: Vec<String>, cells: Vec<Vec<String>> },
    Image { name: String, dim: usize, hashes: Vec<String>, pixels: Vec<Vec<f64>> },
}

/// Parse a directory into groups, preserving every row. CSV files of the same
/// group (e.g. all `*__horizontal_well.csv`) stack into one Table; images of the
/// same group into one Image set. Each row carries the hash linking it to its
/// sibling files. Assembly into one training table happens in `prepare`, where
/// the target's group defines the samples and the rest are joined by hash.
pub fn load_dir_groups(dir: &str) -> Result<Vec<DirGroup>> {
    let mut files: Vec<std::path::PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {dir}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    anyhow::ensure!(!files.is_empty(), "no files in {dir}");

    // Pass 1: hashes that appear as a `__` prefix (rogii-style correlation keys).
    let prefixes: std::collections::HashSet<String> = files
        .iter()
        .filter_map(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .and_then(|n| n.split_once("__"))
                .map(|(h, _)| h.to_string())
        })
        .collect();

    // Pass 2: bucket files by (group, hash). A standalone CSV is its own group.
    let mut tables: std::collections::BTreeMap<String, Vec<(String, std::path::PathBuf)>> =
        std::collections::BTreeMap::new();
    let mut images: std::collections::BTreeMap<String, Vec<(String, std::path::PathBuf)>> =
        std::collections::BTreeMap::new();
    for p in files {
        let (g, hash) = group_and_hash(&p, &prefixes);
        if is_image_file(&p) {
            images.entry(g).or_default().push((hash, p));
        } else {
            tables.entry(g).or_default().push((hash, p));
        }
    }

    let mut groups: Vec<DirGroup> = Vec::new();
    for (name, paths) in tables {
        // Union headers across the group's files (a group's files may differ
        // slightly); align each file's rows to the union by header name. The
        // per-file CSV parse dominates the cost, so read the group's files in
        // parallel — an order-preserving collect keeps the union deterministic
        // (paths is sorted). The union/alignment below is cheap and stays serial.
        let parsed: Vec<(String, Vec<String>, Vec<Vec<String>>)> = paths
            .par_iter()
            .filter_map(|(hash, p)| read_raw_csv(p).ok().map(|(h, rs)| (hash.clone(), h, rs)))
            .collect();
        let mut headers: Vec<String> = Vec::new();
        let mut col: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (_hash, h, _rs) in &parsed {
            for hd in h {
                if !col.contains_key(hd) {
                    col.insert(hd.clone(), headers.len());
                    headers.push(hd.clone());
                }
            }
        }
        if headers.is_empty() {
            continue;
        }
        let mut hashes: Vec<String> = Vec::new();
        let mut cells: Vec<Vec<String>> = Vec::new();
        for (hash, h, rs) in parsed {
            let map: Vec<usize> = h.iter().map(|hd| col[hd]).collect();
            for r in rs {
                let mut row = vec![String::new(); headers.len()];
                for (j, v) in r.into_iter().enumerate() {
                    if let Some(&u) = map.get(j) {
                        row[u] = v;
                    }
                }
                hashes.push(hash.clone());
                cells.push(row);
            }
        }
        groups.push(DirGroup::Table { name, headers, hashes, cells });
    }

    if !images.is_empty() {
        let total: usize = images.values().map(|v| v.len()).sum();
        eprintln!("found images in {}", short_path(dir));
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "    {msg} {per_sec} {elapsed} [{bar:30}] {pos}/{len}",
            )
            .expect("progress template")
            .progress_chars("=>-"),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(120));
        let leaf = std::path::Path::new(dir).file_name().and_then(|s| s.to_str()).unwrap_or(dir);
        pb.set_message(format!("decoding images in /{leaf}"));
        for (name, paths) in images {
            let rows: Vec<(String, Vec<f64>)> = paths
                .par_iter()
                .map(|(hash, p)| {
                    let px = image_to_row(p.to_str().unwrap_or_default(), 32, 32)
                        .map(|r| r.to_vec())
                        .unwrap_or_default();
                    pb.inc(1);
                    (hash.clone(), px)
                })
                .collect();
            let (hashes, pixels) = rows.into_iter().unzip();
            groups.push(DirGroup::Image { name, dim: 32 * 32 * 3, hashes, pixels });
        }
        pb.finish();
        eprintln!();
    }
    Ok(groups)
}

/// Display path with `$HOME` collapsed to `~`, so we never print the expanded
/// `/home/<user>/…` prefix.
pub(crate) fn short_path(p: &str) -> String {
    match std::env::var("HOME") {
        Ok(h) if p == h => "~".to_string(),
        Ok(h) => p
            .strip_prefix(&format!("{h}/"))
            .map(|r| format!("~/{r}"))
            .unwrap_or_else(|| p.to_string()),
        Err(_) => p.to_string(),
    }
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
    // thumbnail_exact fast-halves down to the target box (box-averaging per step)
    // instead of resize()'s single huge-support Triangle convolution, whose cost
    // scales with the downscale ratio — pathological for big-photo → 32x32.
    let rgb = img.thumbnail_exact(width, height).to_rgb8();
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
