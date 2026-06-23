use crate::{Attr, Kind, Mat, Vec1};
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use ndarray::{Array1, Array2};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs;
use std::path::Path;

pub fn train_test_split(x: &Mat, y: &Vec1, test_size: f64, seed: u64) -> (Mat, Mat, Vec1, Vec1) {
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

pub fn read_raw_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>)> {
	let disk = std::fs::metadata(path)
		.map(|m| m.len() as usize)
		.unwrap_or(0);
	// Read EVERY line as a record (no implicit header) so the first row can be
	// inspected before deciding its role — a CSV carries no header flag.
	let mut rdr = csv::ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.from_path(path)
		.with_context(|| format!("failed to open {}", path.display()))?;
	let mut records = rdr.byte_records();
	let Some(first) = records.next() else {
		return Ok((Vec::new(), Vec::new())); // empty file → no columns
	};
	let first = first.with_context(|| "failed to read first CSV record")?;
	let first_cells: Vec<String> = first
		.iter()
		.map(|s| String::from_utf8_lossy(s).into_owned())
		.collect();
	let w = first_cells.len();

	// Header detection is a CSV-format question, not a content heuristic: a header
	// row names columns, so at least one cell is a non-number. If EVERY cell parses
	// as f64 (ints, decimals, signs, scientific notation), the first row is data,
	// not names — synthesize col_0..col_{w-1} and keep the row. Binary structural
	// test, no thresholds.
	let headerless = !first_cells.is_empty()
		&& first_cells.iter().all(|c| {
			let t = c.trim();
			!t.is_empty() && t.parse::<f64>().is_ok()
		});
	let headers: Vec<String> = if headerless {
		(0..w).map(|j| format!("col_{j}")).collect()
	} else {
		first_cells.clone()
	};

	let overhead = std::mem::size_of::<String>();
	let est_rows = count_lines(path)?.saturating_sub(usize::from(!headerless));
	let proj = disk.saturating_add(est_rows.saturating_mul(w).saturating_mul(overhead));
	let avail = crate::available_ram_bytes();
	if proj > avail / 10 * 9 {
		eprintln!("\x1b[1;31mcsv too large to parse into RAM\x1b[0m");
		eprintln!(
			"    {}  →  {est_rows} rows × {w} cols = {} (available {})",
			short_path(path.to_str().unwrap_or_default()),
			human_bytes(proj),
			human_bytes(avail)
		);
		panic!(
			"csv too large to parse into RAM: {} — {est_rows} rows × {w} cols = {} (available {})",
			path.display(),
			human_bytes(proj),
			human_bytes(avail)
		);
	}
	let na = |cell: &str| match cell {
		"NA" | "NaN" | "nan" => String::new(),
		s => s.to_string(),
	};
	let mut rows: Vec<Vec<String>> = Vec::new();
	if headerless {
		rows.push(first_cells.iter().map(|c| na(c)).collect());
	}
	for result in records {
		let record = result.with_context(|| "failed to read CSV record")?;
		let mut row = Vec::with_capacity(w);
		for j in 0..w {
			let cell = record.get(j).map_or(std::borrow::Cow::Borrowed(""), String::from_utf8_lossy);
			row.push(na(cell.as_ref()));
		}
		rows.push(row);
	}
	Ok((headers, rows))
}

pub fn human_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	if f >= K * K * K {
		format!("{:.2} GB", f / (K * K * K))
	} else if f >= K * K {
		format!("{:.1} MB", f / (K * K))
	} else {
		format!("{:.1} KB", f / K)
	}
}

fn count_lines(path: &Path) -> Result<usize> {
	use std::io::Read;
	let f = std::fs::File::open(path)
		.with_context(|| format!("failed to open {}", path.display()))?;
	let mut rdr = std::io::BufReader::with_capacity(1 << 20, f);
	let mut buf = [0u8; 1 << 16];
	let mut lines = 0usize;
	loop {
		let n = rdr
			.read(&mut buf)
			.with_context(|| format!("failed to read {}", path.display()))?;
		if n == 0 {
			break;
		}
		lines += buf[..n].iter().filter(|&&c| c == b'\n').count();
	}
	Ok(lines)
}

fn group_and_hash(p: &Path, prefixes: &std::collections::HashSet<String>) -> (String, String) {
	let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
	if let Some((h, rest)) = name.split_once("__") {
		return (rest.to_string(), h.to_string());
	}
	let stem = p
		.file_stem()
		.and_then(|s| s.to_str())
		.unwrap_or(name)
		.to_string();
	if prefixes.contains(&stem) {
		let ext = p
			.extension()
			.and_then(|e| e.to_str())
			.unwrap_or(name)
			.to_string();
		return (ext, stem);
	}
	(stem.clone(), stem)
}

pub enum DirGroup {
	Table {
		name: String,
		headers: Vec<String>,
		hashes: Vec<String>,
		cells: Vec<Vec<String>>,
	},
	Image {
		name: String,
		dim: usize,
		hashes: Vec<String>,
		pixels: Vec<Vec<f64>>,
	},
}

pub fn load_dir_groups(dir: &str) -> Result<Vec<DirGroup>> {
	let mut files: Vec<std::path::PathBuf> = fs::read_dir(dir)
		.with_context(|| format!("failed to read directory: {dir}"))?
		.filter_map(|e| e.ok())
		.map(|e| e.path())
		.filter(|p| p.is_file())
		.collect();
	files.sort();
	anyhow::ensure!(!files.is_empty(), "no files in {dir}");

	let prefixes: std::collections::HashSet<String> = files
		.iter()
		.filter_map(|p| {
			p.file_name()
				.and_then(|s| s.to_str())
				.and_then(|n| n.split_once("__"))
				.map(|(h, _)| h.to_string())
		})
		.collect();

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

		let parsed: Vec<(String, Vec<String>, Vec<Vec<String>>)> = paths
			.par_iter()
			.filter_map(|(hash, p)| match read_raw_csv(p) {
				Ok((h, rs)) => Some((hash.clone(), h, rs)),
				Err(e) => {
					eprintln!("WARN: skipping {}: {e}", p.display());
					None
				}
			})
			.collect();
		let mut headers: Vec<String> = Vec::new();
		let mut col: std::collections::HashMap<String, usize> =
			std::collections::HashMap::new();
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
		groups.push(DirGroup::Table {
			name,
			headers,
			hashes,
			cells,
		});
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
		let leaf = std::path::Path::new(dir)
			.file_name()
			.and_then(|s| s.to_str())
			.unwrap_or(dir);
		pb.set_message(format!("decoding images in /{leaf}"));
		for (name, paths) in images {
			let rows: Vec<(String, Vec<f64>)> = paths
				.par_iter()
				.map(|(hash, p)| {
					let px = match image_to_row(p.to_str().unwrap_or_default(), 32, 32) {
						Ok(r) => r.to_vec(),
						Err(e) => {
							eprintln!("WARN: skipping image {}: {e}", p.display());
							vec![f64::NAN; 32 * 32 * 3]
						}
					};
					pb.inc(1);
					(hash.clone(), px)
				})
				.collect();
			let (hashes, pixels) = rows.into_iter().unzip();
			groups.push(DirGroup::Image {
				name,
				dim: 32 * 32 * 3,
				hashes,
				pixels,
			});
		}
		pb.finish();
		eprintln!();
	}
	Ok(groups)
}

static ZIP_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn load_zip_groups(path: &str) -> Result<Vec<DirGroup>> {
	let n = ZIP_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let tmp = std::env::temp_dir().join(format!("nrecipe_zip_{}_{}", std::process::id(), n));
	fs::create_dir_all(&tmp)
		.with_context(|| format!("failed to create temp dir {}", tmp.display()))?;

	struct TempDir(std::path::PathBuf);
	impl Drop for TempDir {
		fn drop(&mut self) {
			let _ = std::fs::remove_dir_all(&self.0);
		}
	}
	let guard = TempDir(tmp.clone());

	let file = fs::File::open(path).with_context(|| format!("failed to open zip {path}"))?;
	let mut archive =
		zip::ZipArchive::new(file).with_context(|| format!("failed to read zip {path}"))?;
	for i in 0..archive.len() {
		let mut entry = archive.by_index(i)?;

		let Some(rel) = entry.enclosed_name() else {
			continue;
		};
		let out = tmp.join(rel);
		if entry.is_dir() {
			continue;
		}
		if let Some(parent) = out.parent() {
			fs::create_dir_all(parent)
				.with_context(|| format!("failed to create {}", parent.display()))?;
		}
		let mut w = fs::File::create(&out)
			.with_context(|| format!("failed to create {}", out.display()))?;
		std::io::copy(&mut entry, &mut w)
			.with_context(|| format!("failed to extract {}", out.display()))?;
	}

	let dir = tmp.to_str().context("temp dir path is not valid UTF-8")?;
	let groups = load_groups(dir);
	drop(guard);
	Ok(groups)
}

// ── file → columns: all source parsing (csv / arff / dir / zip / sqlite) ─────

pub fn load_groups(path: &str) -> Vec<DirGroup> {
	let p = std::path::Path::new(path);
	let ext = p
		.extension()
		.and_then(|e| e.to_str())
		.map(str::to_ascii_lowercase);
	match ext.as_deref() {

		Some("zip") => return load_zip_groups(path).expect("load zip"),

		Some("db" | "sqlite") => return load_sqlite_groups(path).expect("load sqlite"),
		_ => {}
	}
	if p.is_dir() {
		load_dir_groups(path).expect("load dir")
	} else {
		let (headers, cells) = read_raw_csv(p).expect("read csv");
		let hashes = vec![String::new(); cells.len()];
		vec![DirGroup::Table {
			name: String::new(),
			headers,
			hashes,
			cells,
		}]
	}
}

pub fn split_fields(line: &str) -> Vec<String> {
	let mut out = Vec::new();
	let mut cur = String::new();
	let mut quoted = false;
	for c in line.chars() {
		match c {
			'\'' => quoted = !quoted,
			',' if !quoted => {
				out.push(cur.trim().to_string());
				cur.clear();
			}
			_ => cur.push(c),
		}
	}
	out.push(cur.trim().to_string());
	out
}

fn parse_attribute(line: &str) -> Attr {
	let rest = line["@attribute".len()..].trim();
	let (name, spec) = if let Some(r) = rest.strip_prefix('\'') {
		let end = r.find('\'').expect("attribute: unterminated quoted name");
		(r[..end].to_string(), r[end + 1..].trim())
	} else {
		let end = rest
			.find(char::is_whitespace)
			.expect("attribute: missing type");
		(rest[..end].to_string(), rest[end..].trim())
	};
	let kind = if spec.starts_with('{') {
		let inner = spec.trim_start_matches('{').trim_end_matches('}');
		Kind::Categorical(split_fields(inner))
	} else {
		Kind::Numeric
	};
	Attr { name, kind }
}

pub fn parse_arff(path: &str) -> (Vec<Attr>, Vec<Vec<String>>) {
	let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
		if e.kind() == std::io::ErrorKind::NotFound {
			let cwd = std::env::current_dir()
				.map(|p| p.display().to_string())
				.unwrap_or_else(|_| ".".to_string());
			let name = std::path::Path::new(path)
				.file_name()
				.and_then(|s| s.to_str())
				.unwrap_or(path);
			panic!("couldn't find '{name}' in {cwd}");
		} else {
			panic!("Data: cannot read {path}: {e}");
		}
	});
	let mut attrs = Vec::new();
	let mut rows = Vec::new();
	let mut in_data = false;
	for raw in text.lines() {
		let line = raw.trim();
		if line.is_empty() || line.starts_with('%') {
			continue;
		}
		if in_data {
			rows.push(split_fields(line));
			continue;
		}
		let lower = line.to_ascii_lowercase();
		if lower.starts_with("@attribute") {
			attrs.push(parse_attribute(line));
		} else if lower.starts_with("@data") {
			in_data = true;
		}
	}
	assert!(!attrs.is_empty(), "Data: no @attribute lines in {path}");
	assert!(!rows.is_empty(), "Data: no @data rows in {path}");
	(attrs, rows)
}

pub fn load_sqlite_groups(path: &str) -> Result<Vec<DirGroup>> {
	anyhow::bail!("SQLite loader not yet implemented (.db/.sqlite): {path}")
}

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

pub fn image_to_row(path: &str, width: u32, height: u32) -> Result<Vec1> {
	let img = image::open(path).with_context(|| format!("failed to open image: {path}"))?;

	let rgb = img.thumbnail_exact(width, height).to_rgb8();
	let raw = rgb.into_raw();
	let row: Vec1 = raw.into_iter().map(|v| v as f64).collect();
	Ok(row)
}

pub fn load_image_dir(dir: &str, width: u32, height: u32) -> Result<Mat> {
	let paths = collect_image_paths(dir)?;
	anyhow::ensure!(!paths.is_empty(), "no image files found in {dir}");

	let row_len = (width * height * 3) as usize;
	let mut data = Vec::with_capacity(paths.len() * row_len);

	for path in &paths {
		let row = image_to_row(
			path.to_str().expect("image path is not valid UTF-8"),
			width,
			height,
		)?;
		data.extend(row.iter());
	}

	let n = data.len() / row_len;
	Ok(Array2::from_shape_vec((n, row_len), data)?)
}

pub fn load_labeled_image_dir(dir: &str, width: u32, height: u32) -> Result<(Mat, Vec1)> {
	let mut subdirs: Vec<_> = fs::read_dir(dir)
		.with_context(|| format!("failed to read directory: {dir}"))?
		.filter_map(|entry| entry.ok())
		.map(|e| e.path())
		.filter(|p| p.is_dir())
		.collect();
	subdirs.sort();
	anyhow::ensure!(!subdirs.is_empty(), "no subdirectories found in {dir}");

	let names: Vec<String> = subdirs
		.iter()
		.map(|p| {
			p.file_name()
				.expect("subdir path has no filename component")
				.to_string_lossy()
				.into_owned()
		})
		.collect();
	let all_float = names.iter().all(|n| n.parse::<f64>().is_ok());

	let label_map: Vec<f64> = if all_float {
		names.iter()
			.map(|n| {
				n.parse()
					.expect("subdir name failed f64 parse after all_float check")
			})
			.collect()
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
			let row = image_to_row(
				path.to_str().expect("image path is not valid UTF-8"),
				width,
				height,
			)?;
			data.extend(row.iter());
			labels.push(label);
		}
	}

	anyhow::ensure!(
		!labels.is_empty(),
		"no images found in any subdirectory of {dir}"
	);
	let n = labels.len();
	let x = Array2::from_shape_vec((n, row_len), data)?;
	let y = Array1::from_vec(labels);
	Ok((x, y))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod header_detection_tests {
	use super::read_raw_csv;
	use std::io::Write as _;

	fn tmp(name: &str, body: &str) -> std::path::PathBuf {
		let p = std::env::temp_dir().join(format!("nrs_hdr_{}_{name}", std::process::id()));
		let mut f = std::fs::File::create(&p).unwrap();
		f.write_all(body.as_bytes()).unwrap();
		p
	}

	// All-numeric first row → no header: synthesize col_N and KEEP that row as data.
	#[test]
	fn headerless_numeric_first_row_is_data() {
		let p = tmp("numeric.csv", "1.0,2,3.29662E-05\n4,5,6\n-7,8.5,9\n");
		let (headers, rows) = read_raw_csv(&p).unwrap();
		assert_eq!(headers, vec!["col_0", "col_1", "col_2"]);
		assert_eq!(rows.len(), 3, "first numeric row must be kept, not eaten");
		assert_eq!(rows[0], vec!["1.0", "2", "3.29662E-05"]);
	}

	// A first row with any non-number is a real header: used verbatim, not kept as data.
	#[test]
	fn named_first_row_is_header() {
		let p = tmp("named.csv", "age,city,score\n31,nyc,9.5\n");
		let (headers, rows) = read_raw_csv(&p).unwrap();
		assert_eq!(headers, vec!["age", "city", "score"]);
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0], vec!["31", "nyc", "9.5"]);
	}

	// Single numeric column (the VNA targets shape) → col_0, every value retained.
	#[test]
	fn single_numeric_column_headerless() {
		let p = tmp("single.csv", "3.29662E-05\n1.1\n2.2\n3.3\n");
		let (headers, rows) = read_raw_csv(&p).unwrap();
		assert_eq!(headers, vec!["col_0"]);
		assert_eq!(rows.len(), 4);
	}
}
