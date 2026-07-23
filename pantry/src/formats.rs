use anyhow::{bail, Context, Result, ensure};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// ─

pub trait UsrData {
	fn parse(&self) -> Result<Vec<DataVec>>;
}

pub struct DataVec {
	pub name: String,
	pub values: Vec<String>,
	pub kind: DataType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
	Numeric,
	Temporal,
	Categoric,
	Ordinal,
	Text,
	Image,
}

impl Default for DataType {
	fn default() -> Self { return Self::Text; }
}

pub struct File {
	pub path: PathBuf,
}

pub struct Dir {
	pub path: PathBuf,
}

pub struct Zip {
	pub path: PathBuf,
}

// ─

const IMAGE_EXTENSIONS: &[&str] = &[
	"jpg", "jpeg", "png", "bmp", "gif", "webp", "tiff", "tif",
	"ico", "pnm", "pbm", "pgm", "ppm", "qoi", "dds", "hdr", "exr", "ff",
];

impl UsrData for File {
	fn parse(&self) -> Result<Vec<DataVec>> {
		let extension = self.path.extension()
			.and_then(|e| e.to_str())
			.unwrap_or("")
			.to_ascii_lowercase();

		let mut vectors = match extension.as_str() {
			"csv" | "tsv" | "txt" | "dat" | "data" => self.parse_csv()?,
			"arff" => self.parse_arff()?,
			"json" => self.parse_json()?,
			"jsonl" | "ndjson" => self.parse_jsonl()?,
			"parquet" => self.parse_parquet()?,
			"arrow" | "feather" => self.parse_arrow()?,
			"npy" => self.parse_npy()?,
			"npz" => self.parse_npz()?,
			"h5" | "hdf5" => self.parse_hdf5()?,
			"xlsx" | "xls" | "xlsb" | "ods" => self.parse_excel()?,
			"mat" => self.parse_mat()?,
			"tfrecord" => self.parse_tfrecord()?,
			"avro" => self.parse_avro()?,
			"db" | "sqlite" => self.parse_sqlite()?,
			ext if IMAGE_EXTENSIONS.contains(&ext) => self.parse_image()?,
			_ => bail!("unsupported file type: {}", self.path.display()),
		};

		assign_data_types(&mut vectors)?;
		return Ok(vectors);
	}
}

impl UsrData for Dir {
	fn parse(&self) -> Result<Vec<DataVec>> {
		let mut vectors = Vec::new();
		let mut entries: Vec<PathBuf> = fs::read_dir(&self.path)
			.with_context(|| format!("failed to read directory: {}", self.path.display()))?
			.filter_map(|e| e.ok())
			.map(|e| e.path())
			.collect();
		entries.sort();
		for path in entries {
			let name = path.file_name()
				.and_then(|n| n.to_str())
				.unwrap_or("");
			if name.starts_with('.') || name == "__MACOSX" {
				continue;
			}
			let mut parsed = if path.is_dir() {
				(Dir { path }).parse()?
			} else {
				(File { path }).parse()?
			};
			vectors.append(&mut parsed);
		}
		return Ok(vectors);
	}
}

impl UsrData for Zip {
	fn parse(&self) -> Result<Vec<DataVec>> {
		let extracted = extract_zip(&self.path)?;
		struct Guard(PathBuf);
		impl Drop for Guard {
			fn drop(&mut self) { fs::remove_dir_all(&self.0).ok(); }
		}
		let _cleanup = Guard(extracted.clone());
		return (Dir { path: extracted }).parse();
	}
}

// ─

pub fn assign_data_types(vectors: &mut [DataVec]) -> Result<()> {
	for vector in vectors.iter_mut() {
		if vector.kind == DataType::Image {
			continue;
		}
		vector.kind = detect_data_type(&vector.values);
	}
	return Ok(());
}

pub fn detect_data_type(values: &[String]) -> DataType {
	let non_empty: Vec<&str> = values.iter()
		.map(String::as_str)
		.filter(|s| !s.is_empty())
		.collect();

	if non_empty.is_empty() {
		return DataType::Text;
	}

	if non_empty.iter().all(|s| s.parse::<f64>().is_ok()) {
		return DataType::Numeric;
	}

	if non_empty.iter().all(|s| looks_temporal(s)) {
		return DataType::Temporal;
	}

	let distinct: HashSet<&str> = non_empty.iter().copied().collect();
	if distinct.len() == non_empty.len() {
		return DataType::Text;
	}

	return DataType::Categoric;
}

fn looks_temporal(s: &str) -> bool {
	let t = s.trim();
	if t.len() < 8 {
		return false;
	}
	let digit_count = t.chars().filter(|c| c.is_ascii_digit()).count();
	if digit_count < 4 {
		return false;
	}
	let parts: Vec<&str> = t.split(|c: char| c == '-' || c == '/' || c == '.').collect();
	if parts.len() < 3 {
		return false;
	}
	return parts.iter().all(|p| {
		p.chars().all(|c| c.is_ascii_digit() || c == ':' || c == 'T' || c == ' ' || c == 'Z')
	});
}

// ─

fn rows_to_columns(headers: Vec<String>, rows: &[Vec<String>]) -> Vec<DataVec> {
	let ncols = headers.len();
	let mut columns: Vec<Vec<String>> = (0..ncols)
		.map(|_| Vec::with_capacity(rows.len()))
		.collect();
	for row in rows {
		for j in 0..ncols {
			columns[j].push(row.get(j).cloned().unwrap_or_default());
		}
	}
	return headers.into_iter().zip(columns)
		.map(|(name, values)| DataVec {
			name,
			values,
			kind: DataType::default(),
		})
		.collect();
}

fn extract_zip(path: &Path) -> Result<PathBuf> {
	use std::sync::atomic::{AtomicUsize, Ordering};
	static COUNT: AtomicUsize = AtomicUsize::new(0);
	let n = COUNT.fetch_add(1, Ordering::Relaxed);
	let tmp = std::env::temp_dir().join(format!("pantry_zip_{}_{n}", std::process::id()));
	fs::create_dir_all(&tmp)
		.with_context(|| format!("failed to create {}", tmp.display()))?;
	let file = fs::File::open(path)
		.with_context(|| format!("failed to open zip {}", path.display()))?;
	let mut archive = zip::ZipArchive::new(file)
		.with_context(|| format!("failed to read zip {}", path.display()))?;
	for i in 0..archive.len() {
		let mut entry = archive.by_index(i)
			.with_context(|| format!("failed to read zip entry {i}"))?;
		let Some(rel) = entry.enclosed_name() else { continue };
		if entry.is_dir() {
			continue;
		}
		let out = tmp.join(rel);
		if let Some(parent) = out.parent() {
			fs::create_dir_all(parent)?;
		}
		let mut w = fs::File::create(&out)
			.with_context(|| format!("failed to create {}", out.display()))?;
		io::copy(&mut entry, &mut w)?;
	}
	return Ok(tmp);
}

// ─

impl File {
	fn parse_csv(&self) -> Result<Vec<DataVec>> {
		let raw = crate::data::read_raw_csv(&self.path)?;
		return Ok(rows_to_columns(raw.headers, &raw.rows));
	}

	fn parse_arff(&self) -> Result<Vec<DataVec>> {
		let path_str = self.path.to_str()
			.context("path is not valid UTF-8")?;
		let table = crate::data::parse_arff(path_str);
		let headers: Vec<String> = table.attrs.iter().map(|a| a.name.clone()).collect();
		return Ok(rows_to_columns(headers, &table.rows));
	}

	fn parse_image(&self) -> Result<Vec<DataVec>> {
		let img = image::open(&self.path)
			.with_context(|| format!("failed to open image: {}", self.path.display()))?;
		let rgb = img.to_rgb8();
		let raw = rgb.into_raw();
		let values: Vec<String> = raw.iter().map(|&v| (v as f64).to_string()).collect();
		let stem = self.path.file_stem()
			.and_then(|s| s.to_str())
			.unwrap_or("image");
		return Ok(vec![DataVec {
			name: stem.to_string(),
			values,
			kind: DataType::Image,
		}]);
	}

	fn parse_sqlite(&self) -> Result<Vec<DataVec>> {
		bail!("SQLite loader not yet implemented: {}", self.path.display())
	}
}

// ─ JSON

impl File {
	fn parse_json(&self) -> Result<Vec<DataVec>> {
		let text = fs::read_to_string(&self.path)
			.with_context(|| format!("failed to read {}", self.path.display()))?;
		let val: serde_json::Value = serde_json::from_str(&text)
			.with_context(|| format!("failed to parse JSON: {}", self.path.display()))?;
		return match val {
			serde_json::Value::Array(arr) => json_values_to_columns(arr),
			serde_json::Value::Object(_) => {
				let mut rows = Vec::new();
				collect_json_rows(val, &mut rows);
				json_values_to_columns(rows)
			}
			_ => bail!("JSON root must be object or array: {}", self.path.display()),
		};
	}

	fn parse_jsonl(&self) -> Result<Vec<DataVec>> {
		let text = fs::read_to_string(&self.path)
			.with_context(|| format!("failed to read {}", self.path.display()))?;
		let mut values = Vec::new();
		for (i, line) in text.lines().enumerate() {
			let trimmed = line.trim();
			if trimmed.is_empty() {
				continue;
			}
			let val: serde_json::Value = serde_json::from_str(trimmed)
				.with_context(|| format!("failed to parse JSON at line {}", i + 1))?;
			values.push(val);
		}
		return json_values_to_columns(values);
	}
}

fn json_values_to_columns(values: Vec<serde_json::Value>) -> Result<Vec<DataVec>> {
	if values.is_empty() {
		return Ok(Vec::new());
	}
	return match &values[0] {
		serde_json::Value::Object(_) => json_objects_to_columns(&values),
		serde_json::Value::Array(arr) => json_arrays_to_columns(&values, arr.len()),
		_ => {
			let col = values.iter().map(json_cell).collect();
			return Ok(vec![DataVec {
				name: "value".to_string(),
				values: col,
				kind: DataType::default(),
			}]);
		}
	};
}

fn collect_json_rows(val: serde_json::Value, rows: &mut Vec<serde_json::Value>) {
	match val {
		serde_json::Value::Object(map) if !map.is_empty() && map.values().all(serde_json::Value::is_object) => {
			for (_, inner) in map {
				collect_json_rows(inner, rows);
			}
		}
		serde_json::Value::Object(map) if !map.is_empty() && map.values().all(is_object_array) => {
			for (_, field) in map {
				if let serde_json::Value::Array(items) = field {
					for item in items {
						collect_json_rows(item, rows);
					}
				}
			}
		}
		other => rows.push(other),
	}
	return;
}

fn is_object_array(val: &serde_json::Value) -> bool {
	return match val {
		serde_json::Value::Array(arr) => arr.iter().all(serde_json::Value::is_object),
		_ => false,
	};
}

fn json_objects_to_columns(values: &[serde_json::Value]) -> Result<Vec<DataVec>> {
	let mut key_order: Vec<String> = Vec::new();
	let mut seen: HashSet<String> = HashSet::new();
	for val in values {
		if let serde_json::Value::Object(map) = val {
			for key in map.keys() {
				if seen.insert(key.clone()) {
					key_order.push(key.clone());
				}
			}
		}
	}
	let mut columns: Vec<Vec<String>> = vec![Vec::with_capacity(values.len()); key_order.len()];
	for val in values {
		let map = val.as_object();
		for (j, key) in key_order.iter().enumerate() {
			let cell = map
				.and_then(|m| m.get(key))
				.map_or(String::new(), json_cell);
			columns[j].push(cell);
		}
	}
	return Ok(key_order.into_iter().zip(columns)
		.map(|(name, values)| DataVec { name, values, kind: DataType::default() })
		.collect());
}

fn json_arrays_to_columns(values: &[serde_json::Value], width: usize) -> Result<Vec<DataVec>> {
	let headers: Vec<String> = (0..width).map(|j| format!("col_{j}")).collect();
	let mut columns: Vec<Vec<String>> = vec![Vec::with_capacity(values.len()); width];
	for val in values {
		let arr = val.as_array();
		for j in 0..width {
			let cell = arr
				.and_then(|a| a.get(j))
				.map_or(String::new(), json_cell);
			columns[j].push(cell);
		}
	}
	return Ok(headers.into_iter().zip(columns)
		.map(|(name, values)| DataVec { name, values, kind: DataType::default() })
		.collect());
}

fn json_cell(val: &serde_json::Value) -> String {
	return match val {
		serde_json::Value::Null => String::new(),
		serde_json::Value::Bool(b) => b.to_string(),
		serde_json::Value::Number(n) => n.to_string(),
		serde_json::Value::String(s) => s.clone(),
		serde_json::Value::Array(_) => {
			let mut leaves = Vec::new();
			json_flatten(val, &mut leaves);
			leaves.join(" ")
		}
		other => other.to_string(),
	};
}

fn json_flatten(val: &serde_json::Value, leaves: &mut Vec<String>) {
	match val {
		serde_json::Value::Null => {}
		serde_json::Value::Bool(b) => leaves.push(b.to_string()),
		serde_json::Value::Number(n) => leaves.push(n.to_string()),
		serde_json::Value::String(s) => leaves.push(s.clone()),
		serde_json::Value::Array(arr) => {
			for item in arr {
				json_flatten(item, leaves);
			}
		}
		other => leaves.push(other.to_string()),
	}
	return;
}

// ─ Parquet + Arrow IPC

impl File {
	fn parse_parquet(&self) -> Result<Vec<DataVec>> {
		use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

		let file = fs::File::open(&self.path)
			.with_context(|| format!("failed to open {}", self.path.display()))?;
		let builder = ParquetRecordBatchReaderBuilder::try_new(file)
			.with_context(|| format!("failed to read parquet: {}", self.path.display()))?;
		let schema = builder.schema().clone();
		let reader = builder.build()
			.with_context(|| format!("failed to build parquet reader: {}", self.path.display()))?;
		let mut batches = Vec::new();
		for batch in reader {
			batches.push(batch.with_context(|| "failed to read parquet batch")?);
		}
		return arrow_batches_to_columns(&schema, &batches);
	}

	fn parse_arrow(&self) -> Result<Vec<DataVec>> {
		use arrow::ipc::reader::FileReader;

		let file = fs::File::open(&self.path)
			.with_context(|| format!("failed to open {}", self.path.display()))?;
		let reader = FileReader::try_new(file, None)
			.with_context(|| format!("failed to read arrow IPC: {}", self.path.display()))?;
		let schema = reader.schema();
		let mut batches = Vec::new();
		for batch in reader {
			batches.push(batch.with_context(|| "failed to read arrow batch")?);
		}
		return arrow_batches_to_columns(&schema, &batches);
	}
}

fn arrow_batches_to_columns(
	schema: &arrow::datatypes::Schema,
	batches: &[arrow::record_batch::RecordBatch],
) -> Result<Vec<DataVec>> {
	let headers: Vec<String> = schema.fields().iter()
		.map(|f| f.name().clone())
		.collect();
	let ncols = headers.len();
	let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
	let mut columns: Vec<Vec<String>> = (0..ncols)
		.map(|_| Vec::with_capacity(total_rows))
		.collect();
	for batch in batches {
		let nrows = batch.num_rows();
		for col in 0..ncols {
			let array = batch.column(col);
			for row in 0..nrows {
				columns[col].push(arrow_cell(array.as_ref(), row));
			}
		}
	}
	return Ok(headers.into_iter().zip(columns)
		.map(|(name, values)| DataVec { name, values, kind: DataType::default() })
		.collect());
}

fn arrow_cell(array: &dyn arrow::array::Array, row: usize) -> String {
	if array.is_null(row) {
		return String::new();
	}
	use arrow::array::*;
	use arrow::datatypes::DataType as AT;
	return match array.data_type() {
		AT::Float64 => array.as_any().downcast_ref::<Float64Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::Float32 => array.as_any().downcast_ref::<Float32Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::Int64 => array.as_any().downcast_ref::<Int64Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::Int32 => array.as_any().downcast_ref::<Int32Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::Int16 => array.as_any().downcast_ref::<Int16Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::Int8 => array.as_any().downcast_ref::<Int8Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::UInt64 => array.as_any().downcast_ref::<UInt64Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::UInt32 => array.as_any().downcast_ref::<UInt32Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::UInt16 => array.as_any().downcast_ref::<UInt16Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::UInt8 => array.as_any().downcast_ref::<UInt8Array>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::Boolean => array.as_any().downcast_ref::<BooleanArray>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::Utf8 => array.as_any().downcast_ref::<StringArray>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		AT::LargeUtf8 => array.as_any().downcast_ref::<LargeStringArray>()
			.map_or(String::new(), |a| a.value(row).to_string()),
		_ => String::new(),
	};
}

// ─ NumPy

impl File {
	fn parse_npy(&self) -> Result<Vec<DataVec>> {
		let data = fs::read(&self.path)
			.with_context(|| format!("failed to read {}", self.path.display()))?;
		let (info, payload) = parse_npy_bytes(&data)
			.with_context(|| format!("failed to parse npy: {}", self.path.display()))?;
		let total: usize = info.shape.iter().product();
		let values = npy_to_f64_strings(payload, &info.descr, total)?;
		let nrows = info.shape.first().copied().unwrap_or(total);
		let ncols = if info.shape.len() >= 2 { info.shape[1] } else { 1 };
		let headers: Vec<String> = (0..ncols).map(|j| format!("col_{j}")).collect();
		let mut columns: Vec<Vec<String>> = (0..ncols)
			.map(|_| Vec::with_capacity(nrows))
			.collect();
		for row in 0..nrows {
			for col in 0..ncols {
				let idx = if info.fortran_order {
					col * nrows + row
				} else {
					row * ncols + col
				};
				columns[col].push(values.get(idx).cloned().unwrap_or_default());
			}
		}
		return Ok(headers.into_iter().zip(columns)
			.map(|(name, values)| DataVec { name, values, kind: DataType::default() })
			.collect());
	}

	fn parse_npz(&self) -> Result<Vec<DataVec>> {
		let file = fs::File::open(&self.path)
			.with_context(|| format!("failed to open {}", self.path.display()))?;
		let mut archive = zip::ZipArchive::new(file)
			.with_context(|| format!("failed to read npz: {}", self.path.display()))?;
		let mut vectors = Vec::new();
		for i in 0..archive.len() {
			let mut entry = archive.by_index(i)
				.with_context(|| format!("failed to read npz entry {i}"))?;
			let entry_name = entry.name().to_string();
			if !entry_name.ends_with(".npy") {
				continue;
			}
			let array_name = entry_name.trim_end_matches(".npy");
			let mut buf = Vec::new();
			io::Read::read_to_end(&mut entry, &mut buf)?;
			let (info, payload) = parse_npy_bytes(&buf)
				.with_context(|| format!("failed to parse npy entry: {entry_name}"))?;
			let total: usize = info.shape.iter().product();
			let values = npy_to_f64_strings(payload, &info.descr, total)?;
			let nrows = info.shape.first().copied().unwrap_or(total);
			let ncols = if info.shape.len() >= 2 { info.shape[1] } else { 1 };
			for col in 0..ncols {
				let col_name = if ncols > 1 {
					format!("{array_name}_{col}")
				} else {
					array_name.to_string()
				};
				let mut col_vals = Vec::with_capacity(nrows);
				for row in 0..nrows {
					let idx = if info.fortran_order {
						col * nrows + row
					} else {
						row * ncols + col
					};
					col_vals.push(values.get(idx).cloned().unwrap_or_default());
				}
				vectors.push(DataVec {
					name: col_name,
					values: col_vals,
					kind: DataType::default(),
				});
			}
		}
		return Ok(vectors);
	}
}

struct NpyInfo {
	descr: String,
	fortran_order: bool,
	shape: Vec<usize>,
}

fn parse_npy_bytes(data: &[u8]) -> Result<(NpyInfo, &[u8])> {
	ensure!(data.len() >= 10, "npy file too short");
	ensure!(
		data[0] == 0x93 && data[1] == b'N' && data[2] == b'U'
			&& data[3] == b'M' && data[4] == b'P' && data[5] == b'Y',
		"not a valid npy file (bad magic)"
	);
	let major = data[6];
	let header_len: usize;
	let header_start: usize;
	if major <= 1 {
		ensure!(data.len() >= 10, "npy v1 header too short");
		header_len = u16::from_le_bytes([data[8], data[9]]) as usize;
		header_start = 10;
	} else {
		ensure!(data.len() >= 12, "npy v2 header too short");
		header_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
		header_start = 12;
	}
	let header_end = header_start + header_len;
	ensure!(data.len() >= header_end, "npy header truncated");
	let header = std::str::from_utf8(&data[header_start..header_end])
		.context("npy header is not valid UTF-8")?;
	let descr = npy_extract_str(header, "descr")
		.context("missing 'descr' in npy header")?
		.to_string();
	let fortran_order = npy_extract_bool(header, "fortran_order");
	let shape = npy_extract_shape(header)
		.context("missing 'shape' in npy header")?;
	let payload = &data[header_end..];
	return Ok((NpyInfo { descr, fortran_order, shape }, payload));
}

fn npy_extract_str<'a>(header: &'a str, key: &str) -> Option<&'a str> {
	let needle = format!("'{key}':");
	let rest = header.split_once(&needle)?.1.trim();
	let open = rest.find('\'')?;
	let close = rest[open + 1..].find('\'')?;
	return Some(&rest[open + 1..open + 1 + close]);
}

fn npy_extract_bool(header: &str, key: &str) -> bool {
	let needle = format!("'{key}':");
	return header.split_once(&needle)
		.map_or(false, |(_, rest)| rest.trim().starts_with("True"));
}

fn npy_extract_shape(header: &str) -> Option<Vec<usize>> {
	let rest = header.split_once("'shape':")?.1.trim();
	let open = rest.find('(')?;
	let close = rest.find(')')?;
	let inner = &rest[open + 1..close];
	let dims: Vec<usize> = inner.split(',')
		.filter_map(|s| {
			let t = s.trim();
			if t.is_empty() { return None; }
			return Some(t.parse::<usize>().ok()?);
		})
		.collect();
	return Some(dims);
}

fn npy_to_f64_strings(raw: &[u8], descr: &str, count: usize) -> Result<Vec<String>> {
	let chars: Vec<char> = descr.chars().collect();
	ensure!(chars.len() >= 3, "invalid npy dtype descriptor: {descr}");
	let endian = chars[0];
	let dtype = chars[1];
	let size_str: String = chars[2..].iter().collect();
	let size: usize = size_str.parse()
		.with_context(|| format!("invalid dtype size in: {descr}"))?;
	let le = endian == '<' || endian == '|' || endian == '=';
	let mut out = Vec::with_capacity(count);
	for i in 0..count {
		let start = i * size;
		let end = start + size;
		ensure!(end <= raw.len(), "npy data truncated at element {i}");
		let bytes = &raw[start..end];
		let val: f64 = match (dtype, size) {
			('f', 8) => {
				let b: [u8; 8] = bytes.try_into()?;
				if le { f64::from_le_bytes(b) } else { f64::from_be_bytes(b) }
			}
			('f', 4) => {
				let b: [u8; 4] = bytes.try_into()?;
				f64::from(if le { f32::from_le_bytes(b) } else { f32::from_be_bytes(b) })
			}
			('i', 8) => {
				let b: [u8; 8] = bytes.try_into()?;
				(if le { i64::from_le_bytes(b) } else { i64::from_be_bytes(b) }) as f64
			}
			('i', 4) => {
				let b: [u8; 4] = bytes.try_into()?;
				f64::from(if le { i32::from_le_bytes(b) } else { i32::from_be_bytes(b) })
			}
			('i', 2) => {
				let b: [u8; 2] = bytes.try_into()?;
				f64::from(if le { i16::from_le_bytes(b) } else { i16::from_be_bytes(b) })
			}
			('i', 1) => f64::from(bytes[0] as i8),
			('u', 8) => {
				let b: [u8; 8] = bytes.try_into()?;
				(if le { u64::from_le_bytes(b) } else { u64::from_be_bytes(b) }) as f64
			}
			('u', 4) => {
				let b: [u8; 4] = bytes.try_into()?;
				f64::from(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
			}
			('u', 2) => {
				let b: [u8; 2] = bytes.try_into()?;
				f64::from(if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) })
			}
			('u', 1) => f64::from(bytes[0]),
			('b', 1) => {
				if bytes[0] == 0 { 0.0 } else { 1.0 }
			}
			_ => bail!("unsupported npy dtype: {descr}"),
		};
		out.push(val.to_string());
	}
	return Ok(out);
}

// ─ HDF5

impl File {
	fn parse_hdf5(&self) -> Result<Vec<DataVec>> {
		let file = hdf5::File::open(&self.path)
			.with_context(|| format!("failed to open HDF5: {}", self.path.display()))?;
		let mut vectors = Vec::new();
		hdf5_collect(&file, "", &mut vectors)?;
		return Ok(vectors);
	}
}

fn hdf5_collect(group: &hdf5::Group, prefix: &str, vectors: &mut Vec<DataVec>) -> Result<()> {
	let names = group.member_names()
		.with_context(|| format!("failed to list HDF5 group members at '{prefix}'"))?;
	for name in &names {
		let full = if prefix.is_empty() {
			name.clone()
		} else {
			format!("{prefix}/{name}")
		};
		if let Ok(ds) = group.dataset(name) {
			let shape = ds.shape();
			match shape.len() {
				0 => {
					let data: Vec<f64> = ds.read_raw()
						.with_context(|| format!("failed to read HDF5 dataset '{full}'"))?;
					vectors.push(DataVec {
						name: full,
						values: data.iter().map(|v| v.to_string()).collect(),
						kind: DataType::default(),
					});
				}
				1 => {
					let data: Vec<f64> = ds.read_raw()
						.with_context(|| format!("failed to read HDF5 dataset '{full}'"))?;
					vectors.push(DataVec {
						name: full,
						values: data.iter().map(|v| v.to_string()).collect(),
						kind: DataType::default(),
					});
				}
				_ => {
					let nrows = shape[0];
					let ncols = shape[1];
					let data: Vec<f64> = ds.read_raw()
						.with_context(|| format!("failed to read HDF5 dataset '{full}'"))?;
					for col in 0..ncols {
						let col_name = format!("{full}_{col}");
						let values: Vec<String> = (0..nrows)
							.map(|row| {
								data.get(row * ncols + col)
									.map_or(String::new(), |v| v.to_string())
							})
							.collect();
						vectors.push(DataVec {
							name: col_name,
							values,
							kind: DataType::default(),
						});
					}
				}
			}
			continue;
		}
		if let Ok(sub) = group.group(name) {
			hdf5_collect(&sub, &full, vectors)?;
		}
	}
	return Ok(());
}

// ─ Excel

impl File {
	fn parse_excel(&self) -> Result<Vec<DataVec>> {
		use calamine::Reader;

		let mut wb = calamine::open_workbook_auto(&self.path)
			.with_context(|| format!("failed to open workbook: {}", self.path.display()))?;
		let sheets = wb.sheet_names().to_vec();
		ensure!(!sheets.is_empty(), "no sheets in workbook: {}", self.path.display());
		let range = wb.worksheet_range(&sheets[0])
			.with_context(|| format!("failed to read sheet '{}'", sheets[0]))?;
		let mut row_iter = range.rows();
		let Some(first) = row_iter.next() else {
			return Ok(Vec::new());
		};
		let all_numeric = first.iter().all(|c| matches!(c, calamine::Data::Float(_) | calamine::Data::Int(_)));
		let headers: Vec<String>;
		let mut rows: Vec<Vec<String>> = Vec::new();
		if all_numeric {
			headers = (0..first.len()).map(|j| format!("col_{j}")).collect();
			rows.push(first.iter().map(excel_cell).collect());
		} else {
			headers = first.iter().enumerate()
				.map(|(i, c)| {
					let s = excel_cell(c);
					if s.is_empty() { format!("col_{i}") } else { s }
				})
				.collect();
		}
		for row in row_iter {
			rows.push(row.iter().map(excel_cell).collect());
		}
		return Ok(rows_to_columns(headers, &rows));
	}
}

fn excel_cell(cell: &calamine::Data) -> String {
	use calamine::Data;
	return match cell {
		Data::Empty => String::new(),
		Data::String(s) => s.clone(),
		Data::Float(f) => f.to_string(),
		Data::Int(i) => i.to_string(),
		Data::Bool(b) => b.to_string(),
		_ => format!("{cell:?}"),
	};
}

// ─ MATLAB

impl File {
	fn parse_mat(&self) -> Result<Vec<DataVec>> {
		let raw = fs::read(&self.path)
			.with_context(|| format!("failed to read {}", self.path.display()))?;
		if raw.starts_with(&[0x89, b'H', b'D', b'F', 0x0D, 0x0A, 0x1A, 0x0A]) {
			return self.parse_hdf5();
		}
		let cursor = io::Cursor::new(&raw);
		let mat = matfile::MatFile::parse(cursor)
			.with_context(|| format!("failed to parse MAT file: {}", self.path.display()))?;
		let mut vectors = Vec::new();
		for array in mat.arrays() {
			let name = array.name().to_string();
			let dims = array.size();
			let nrows = dims.first().copied().unwrap_or(0);
			let ncols = dims.get(1).copied().unwrap_or(1);
			let f64_vals = mat_array_to_f64s(array);
			for col in 0..ncols {
				let col_name = if ncols > 1 {
					format!("{name}_{col}")
				} else {
					name.clone()
				};
				let values: Vec<String> = (0..nrows)
					.map(|row| {
						let idx = col * nrows + row;
						f64_vals.get(idx).map_or(String::new(), |v| v.to_string())
					})
					.collect();
				vectors.push(DataVec {
					name: col_name,
					values,
					kind: DataType::default(),
				});
			}
		}
		return Ok(vectors);
	}
}

fn mat_array_to_f64s(array: &matfile::Array) -> Vec<f64> {
	use matfile::NumericData;
	return match array.data() {
		NumericData::Double { real, .. } => real.clone(),
		NumericData::Single { real, .. } => real.iter().map(|&v| f64::from(v)).collect(),
		NumericData::Int8 { real, .. } => real.iter().map(|&v| f64::from(v)).collect(),
		NumericData::UInt8 { real, .. } => real.iter().map(|&v| f64::from(v)).collect(),
		NumericData::Int16 { real, .. } => real.iter().map(|&v| f64::from(v)).collect(),
		NumericData::UInt16 { real, .. } => real.iter().map(|&v| f64::from(v)).collect(),
		NumericData::Int32 { real, .. } => real.iter().map(|&v| f64::from(v)).collect(),
		NumericData::UInt32 { real, .. } => real.iter().map(|&v| f64::from(v)).collect(),
		NumericData::Int64 { real, .. } => real.iter().map(|&v| v as f64).collect(),
		NumericData::UInt64 { real, .. } => real.iter().map(|&v| v as f64).collect(),
	};
}

// ─ TFRecord

impl File {
	fn parse_tfrecord(&self) -> Result<Vec<DataVec>> {
		let data = fs::read(&self.path)
			.with_context(|| format!("failed to read {}", self.path.display()))?;
		let mut pos = 0usize;
		let mut all_keys: Vec<String> = Vec::new();
		let mut seen: HashSet<String> = HashSet::new();
		let mut records: Vec<Vec<(String, TfFeature)>> = Vec::new();

		while pos < data.len() {
			let (frame, next) = tfrecord_frame(&data, pos)?;
			let features = tf_parse_example(frame)?;
			for (key, _) in &features {
				if seen.insert(key.clone()) {
					all_keys.push(key.clone());
				}
			}
			records.push(features);
			pos = next;
		}

		let key_idx: HashMap<&str, usize> = all_keys.iter()
			.enumerate()
			.map(|(i, k)| (k.as_str(), i))
			.collect();
		let mut columns: Vec<Vec<String>> = (0..all_keys.len())
			.map(|_| Vec::with_capacity(records.len()))
			.collect();
		for record in &records {
			let mut row = vec![String::new(); all_keys.len()];
			for (key, feature) in record {
				if let Some(&idx) = key_idx.get(key.as_str()) {
					row[idx] = tf_feature_to_string(feature);
				}
			}
			for (j, val) in row.into_iter().enumerate() {
				columns[j].push(val);
			}
		}
		return Ok(all_keys.into_iter().zip(columns)
			.map(|(name, values)| DataVec { name, values, kind: DataType::default() })
			.collect());
	}
}

enum TfFeature {
	Floats(Vec<f32>),
	Ints(Vec<i64>),
	Bytes(Vec<Vec<u8>>),
}

fn tf_feature_to_string(f: &TfFeature) -> String {
	return match f {
		TfFeature::Floats(v) if v.len() == 1 => v[0].to_string(),
		TfFeature::Floats(v) => format!("{v:?}"),
		TfFeature::Ints(v) if v.len() == 1 => v[0].to_string(),
		TfFeature::Ints(v) => format!("{v:?}"),
		TfFeature::Bytes(v) if v.len() == 1 => String::from_utf8_lossy(&v[0]).into_owned(),
		TfFeature::Bytes(v) => format!("[{} byte lists]", v.len()),
	};
}

fn tfrecord_frame<'a>(data: &'a [u8], pos: usize) -> Result<(&'a [u8], usize)> {
	ensure!(pos + 12 <= data.len(), "tfrecord: unexpected end of file at offset {pos}");
	let length_bytes: [u8; 8] = data[pos..pos + 8].try_into()?;
	let length = u64::from_le_bytes(length_bytes) as usize;
	let data_start = pos + 12;
	ensure!(
		data_start + length + 4 <= data.len(),
		"tfrecord: truncated record at offset {pos}, length {length}"
	);
	let record = &data[data_start..data_start + length];
	let next = data_start + length + 4;
	return Ok((record, next));
}

fn tf_read_varint(data: &[u8], mut pos: usize) -> Result<(u64, usize)> {
	let mut result: u64 = 0;
	let mut shift = 0u32;
	loop {
		ensure!(pos < data.len(), "protobuf varint: unexpected end of data");
		let byte = data[pos];
		pos += 1;
		result |= u64::from(byte & 0x7F) << shift;
		if byte & 0x80 == 0 {
			return Ok((result, pos));
		}
		shift += 7;
		ensure!(shift < 64, "protobuf varint: too many continuation bytes");
	}
}

fn tf_skip_field(data: &[u8], wire_type: u32, pos: usize) -> Result<usize> {
	return match wire_type {
		0 => { let (_, p) = tf_read_varint(data, pos)?; Ok(p) }
		1 => Ok(pos + 8),
		2 => {
			let (len, p) = tf_read_varint(data, pos)?;
			Ok(p + len as usize)
		}
		5 => Ok(pos + 4),
		_ => bail!("protobuf: unknown wire type {wire_type}"),
	};
}

fn tf_parse_example(data: &[u8]) -> Result<Vec<(String, TfFeature)>> {
	let mut pos = 0usize;
	while pos < data.len() {
		let (tag, p) = tf_read_varint(data, pos)?;
		pos = p;
		let field = (tag >> 3) as u32;
		let wire = (tag & 7) as u32;
		if field == 1 && wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			return tf_parse_features(&data[pos..pos + len as usize]);
		}
		pos = tf_skip_field(data, wire, pos)?;
	}
	return Ok(Vec::new());
}

fn tf_parse_features(data: &[u8]) -> Result<Vec<(String, TfFeature)>> {
	let mut result = Vec::new();
	let mut pos = 0usize;
	while pos < data.len() {
		let (tag, p) = tf_read_varint(data, pos)?;
		pos = p;
		let field = (tag >> 3) as u32;
		let wire = (tag & 7) as u32;
		if field == 1 && wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			let entry = tf_parse_map_entry(&data[pos..pos + len as usize])?;
			result.push(entry);
			pos += len as usize;
		} else {
			pos = tf_skip_field(data, wire, pos)?;
		}
	}
	return Ok(result);
}

fn tf_parse_map_entry(data: &[u8]) -> Result<(String, TfFeature)> {
	let mut key = String::new();
	let mut feature = TfFeature::Floats(Vec::new());
	let mut pos = 0usize;
	while pos < data.len() {
		let (tag, p) = tf_read_varint(data, pos)?;
		pos = p;
		let field = (tag >> 3) as u32;
		let wire = (tag & 7) as u32;
		if field == 1 && wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			key = String::from_utf8_lossy(&data[pos..pos + len as usize]).into_owned();
			pos += len as usize;
		} else if field == 2 && wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			feature = tf_parse_feature(&data[pos..pos + len as usize])?;
			pos += len as usize;
		} else {
			pos = tf_skip_field(data, wire, pos)?;
		}
	}
	return Ok((key, feature));
}

fn tf_parse_feature(data: &[u8]) -> Result<TfFeature> {
	let mut pos = 0usize;
	while pos < data.len() {
		let (tag, p) = tf_read_varint(data, pos)?;
		pos = p;
		let field = (tag >> 3) as u32;
		let wire = (tag & 7) as u32;
		if wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			let inner = &data[pos..pos + len as usize];
			pos += len as usize;
			return match field {
				1 => Ok(tf_parse_bytes_list(inner)?),
				2 => Ok(TfFeature::Floats(tf_parse_float_list(inner)?)),
				3 => Ok(TfFeature::Ints(tf_parse_int64_list(inner)?)),
				_ => { continue; }
			};
		}
		pos = tf_skip_field(data, wire, pos)?;
	}
	return Ok(TfFeature::Floats(Vec::new()));
}

fn tf_parse_float_list(data: &[u8]) -> Result<Vec<f32>> {
	let mut result = Vec::new();
	let mut pos = 0usize;
	while pos < data.len() {
		let (tag, p) = tf_read_varint(data, pos)?;
		pos = p;
		let field = (tag >> 3) as u32;
		let wire = (tag & 7) as u32;
		if field == 1 && wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			let end = pos + len as usize;
			while pos + 4 <= end {
				let b: [u8; 4] = data[pos..pos + 4].try_into()?;
				result.push(f32::from_le_bytes(b));
				pos += 4;
			}
		} else if field == 1 && wire == 5 {
			let b: [u8; 4] = data[pos..pos + 4].try_into()?;
			result.push(f32::from_le_bytes(b));
			pos += 4;
		} else {
			pos = tf_skip_field(data, wire, pos)?;
		}
	}
	return Ok(result);
}

fn tf_parse_int64_list(data: &[u8]) -> Result<Vec<i64>> {
	let mut result = Vec::new();
	let mut pos = 0usize;
	while pos < data.len() {
		let (tag, p) = tf_read_varint(data, pos)?;
		pos = p;
		let field = (tag >> 3) as u32;
		let wire = (tag & 7) as u32;
		if field == 1 && wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			let end = pos + len as usize;
			while pos < end {
				let (val, p) = tf_read_varint(data, pos)?;
				result.push(val as i64);
				pos = p;
			}
		} else if field == 1 && wire == 0 {
			let (val, p) = tf_read_varint(data, pos)?;
			result.push(val as i64);
			pos = p;
		} else {
			pos = tf_skip_field(data, wire, pos)?;
		}
	}
	return Ok(result);
}

fn tf_parse_bytes_list(data: &[u8]) -> Result<TfFeature> {
	let mut result = Vec::new();
	let mut pos = 0usize;
	while pos < data.len() {
		let (tag, p) = tf_read_varint(data, pos)?;
		pos = p;
		let field = (tag >> 3) as u32;
		let wire = (tag & 7) as u32;
		if field == 1 && wire == 2 {
			let (len, p) = tf_read_varint(data, pos)?;
			pos = p;
			result.push(data[pos..pos + len as usize].to_vec());
			pos += len as usize;
		} else {
			pos = tf_skip_field(data, wire, pos)?;
		}
	}
	return Ok(TfFeature::Bytes(result));
}

// ─ Avro

impl File {
	fn parse_avro(&self) -> Result<Vec<DataVec>> {
		let file = fs::File::open(&self.path)
			.with_context(|| format!("failed to open {}", self.path.display()))?;
		let reader = apache_avro::Reader::new(file)
			.with_context(|| format!("failed to read avro: {}", self.path.display()))?;
		let schema = reader.writer_schema().clone();
		let headers: Vec<String> = match &schema {
			apache_avro::Schema::Record(rs) => {
				rs.fields.iter().map(|f| f.name.clone()).collect()
			}
			_ => Vec::new(),
		};
		let mut rows: Vec<Vec<String>> = Vec::new();
		for val_result in reader {
			let val = val_result
				.with_context(|| format!("failed to read avro record: {}", self.path.display()))?;
			match val {
				apache_avro::types::Value::Record(fields) => {
					if headers.is_empty() {
						let row: Vec<String> = fields.iter()
							.map(|(_, v)| avro_cell(v))
							.collect();
						rows.push(row);
					} else {
						let field_map: HashMap<&str, &apache_avro::types::Value> = fields.iter()
							.map(|(k, v)| (k.as_str(), v))
							.collect();
						let row: Vec<String> = headers.iter()
							.map(|h| field_map.get(h.as_str()).map_or(String::new(), |v| avro_cell(v)))
							.collect();
						rows.push(row);
					}
				}
				other => {
					rows.push(vec![avro_cell(&other)]);
				}
			}
		}
		let hdrs = if headers.is_empty() && !rows.is_empty() {
			(0..rows[0].len()).map(|j| format!("col_{j}")).collect()
		} else {
			headers
		};
		return Ok(rows_to_columns(hdrs, &rows));
	}
}

fn avro_cell(val: &apache_avro::types::Value) -> String {
	use apache_avro::types::Value;
	return match val {
		Value::Null => String::new(),
		Value::Boolean(b) => b.to_string(),
		Value::Int(i) => i.to_string(),
		Value::Long(i) => i.to_string(),
		Value::Float(f) => f.to_string(),
		Value::Double(f) => f.to_string(),
		Value::String(s) => s.clone(),
		Value::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
		Value::Enum(_idx, s) => s.clone(),
		Value::Union(_idx, inner) => avro_cell(inner),
		Value::Fixed(_size, b) => String::from_utf8_lossy(b).into_owned(),
		Value::Date(d) => d.to_string(),
		_ => format!("{val:?}"),
	};
}
