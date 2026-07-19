use crate::{Attr, Kind, Mat, Vec1};
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use ndarray::{Array1, Array2};
use rayon::prelude::*;
use recipe_infer::log::{Errored, Write, data};
use std::borrow::Cow;
use std::cmp;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const NLB: u8 = 10;

pub fn sniff_delimiter(path: &Path) -> u8 {
	use std::io::BufRead;
	let Ok(f) = fs::File::open(path) else {
		return b',';
	};
	let mut rdr = io::BufReader::new(f);
	let mut line = Vec::new();
	loop {
		line.clear();
		let Ok(count) = rdr.read_until(NLB, &mut line) else {
			return b',';
		};
		let Some(_more) = count.checked_sub(1) else {
			return b',';
		};
		let Some(_nb) = line.iter().find(|c| !c.is_ascii_whitespace()) else {
			continue;
		};
		let text = String::from_utf8_lossy(&line);
		let commas = text.matches(',').count();
		let semis = text.matches(';').count();
		let tabs = text.matches('\t').count();
		let total_sep = commas + semis + tabs;
		let tokens: Vec<&str> = text.split_whitespace().collect();
		if total_sep == 0
			&& tokens.len() >= 2
			&& tokens.iter().all(|t| t.parse::<f64>().is_ok())
		{
			return b' ';
		}
		return match semis.cmp(&commas) {
			cmp::Ordering::Greater => b';',
			cmp::Ordering::Less | cmp::Ordering::Equal => match tabs.cmp(&commas) {
				cmp::Ordering::Greater => b'\t',
				cmp::Ordering::Less | cmp::Ordering::Equal => b',',
			},
		};
	}
}

enum FirstRow {
	Header,
	Data,
}

pub fn read_raw_csv(path: &Path) -> Result<RawCsv> {
	let delim = sniff_delimiter(path);
	let Some(()) = Some(()).filter(|_u| delim != b' ') else {
		return read_raw_whitespace(path);
	};
	let disk = fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);
	let mut rdr = csv::ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.delimiter(delim)
		.from_path(path)
		.with_context(|| format!("failed to open {}", path.display()))?;
	let mut records = rdr.byte_records();
	let Some(first) = records.next() else {
		return Ok(RawCsv {
			headers: Vec::new(),
			rows: Vec::new(),
		});
	};
	let first = first.with_context(|| "failed to read first CSV record")?;
	let first_cells: Vec<String> = first
		.iter()
		.map(|s| String::from_utf8_lossy(s).into_owned())
		.collect();
	let w = first_cells.len();

	let first_row = match first_cells.first() {
		None => FirstRow::Header,
		Some(_c0) => match first_cells.iter().find(|c| {
			let t = c.trim();
			t.is_empty() || t.parse::<f64>().is_err()
		}) {
			Some(_nn) => FirstRow::Header,
			None => FirstRow::Data,
		},
	};
	let headers: Vec<String> = match first_row {
		FirstRow::Data => (0..w).map(|j| format!("col_{j}")).collect(),
		FirstRow::Header => first_cells.clone(),
	};

	let overhead = mem::size_of::<String>();
	let physical = count_lines(path)?;
	let est_rows = match first_row {
		FirstRow::Data => physical,
		FirstRow::Header => physical.saturating_sub(1),
	};
	let proj = disk.saturating_add(est_rows.saturating_mul(w).saturating_mul(overhead));
	let avail = crate::available_ram_bytes();
	if avail.checked_sub(proj).is_none() {
		Write::error("csv too large to parse into RAM");
		Write::error(format!(
			"    {}  →  {est_rows} rows × {w} cols = {} (available {})",
			short_path(path.to_str().unwrap_or_default()),
			human_bytes(proj),
			human_bytes(avail)
		));
		Write::err(format!(
			"csv too large to parse into RAM: {} — {est_rows} rows × {w} cols = {} (available {})",
			path.display(),
			human_bytes(proj),
			human_bytes(avail)
		))?;
	}
	let na = |cell: &str| match cell {
		"NA" | "NaN" | "nan" => String::new(),
		s => s.to_string(),
	};
	let mut rows: Vec<Vec<String>> = match first_row {
		FirstRow::Data => vec![first_cells.iter().map(|c| na(c)).collect()],
		FirstRow::Header => Vec::new(),
	};
	for result in records {
		let record = result.with_context(|| "failed to read CSV record")?;
		let mut row = Vec::with_capacity(w);
		for j in 0..w {
			let cell = record
				.get(j)
				.map_or(Cow::Borrowed(""), String::from_utf8_lossy);
			row.push(na(cell.as_ref()));
		}
		rows.push(row);
	}
	Ok(RawCsv { headers, rows })
}

pub struct RawCsv {
	pub headers: Vec<String>,
	pub rows: Vec<Vec<String>>,
}

fn read_raw_whitespace(path: &Path) -> Result<RawCsv> {
	use std::io::BufRead;
	let disk = fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);
	let f = fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
	let rdr = io::BufReader::with_capacity(1 << 20, f);
	let mut lines = rdr.lines();
	let first = loop {
		let Some(l) = lines.next() else {
			return Ok(RawCsv {
				headers: Vec::new(),
				rows: Vec::new(),
			});
		};
		let l = l.with_context(|| format!("failed to read {}", path.display()))?;
		let Some(_c0) = l.trim().chars().next() else {
			continue;
		};
		break l;
	};
	let first_cells: Vec<String> = first.split_whitespace().map(str::to_string).collect();
	let w = first_cells.len();
	let headers: Vec<String> = (0..w).map(|j| format!("col_{j}")).collect();

	let overhead = mem::size_of::<String>();
	let est_rows = count_lines(path)?;
	let proj = disk.saturating_add(est_rows.saturating_mul(w).saturating_mul(overhead));
	let avail = crate::available_ram_bytes();
	if avail.checked_sub(proj).is_none() {
		Write::error("csv too large to parse into RAM");
		Write::error(format!(
			"    {}  →  {est_rows} rows × {w} cols = {} (available {})",
			short_path(path.to_str().unwrap_or_default()),
			human_bytes(proj),
			human_bytes(avail)
		));
		Write::err(format!(
			"csv too large to parse into RAM: {} — {est_rows} rows × {w} cols = {} (available {})",
			path.display(),
			human_bytes(proj),
			human_bytes(avail)
		))?;
	}
	let na = |cell: &str| match cell {
		"NA" | "NaN" | "nan" => String::new(),
		s => s.to_string(),
	};
	let mut rows: Vec<Vec<String>> = Vec::new();
	rows.push(first_cells.iter().map(|c| na(c)).collect());
	for line in lines {
		let line = line.with_context(|| format!("failed to read {}", path.display()))?;
		let Some(_c0) = line.trim().chars().next() else {
			continue;
		};
		let mut row = vec![String::new(); w];
		let mut j = 0usize;
		for tok in line.split_whitespace().take(w) {
			row[j] = na(tok);
			j += 1;
		}
		rows.push(row);
	}
	Ok(RawCsv { headers, rows })
}

pub fn human_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	let gb = K * K * K;
	let mb = K * K;
	match f.partial_cmp(&gb) {
		Some(cmp::Ordering::Greater | cmp::Ordering::Equal) => {
			format!("{:.2} GB", f / gb)
		}
		Some(cmp::Ordering::Less) | None => match f.partial_cmp(&mb) {
			Some(cmp::Ordering::Greater | cmp::Ordering::Equal) => {
				format!("{:.1} MB", f / mb)
			}
			Some(cmp::Ordering::Less) | None => format!("{:.1} KB", f / K),
		},
	}
}

fn count_lines(path: &Path) -> Result<usize> {
	use std::io::Read;
	let f = fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
	let mut rdr = io::BufReader::with_capacity(1 << 20, f);
	let mut buf = [0u8; 1 << 16];
	let mut lines = 0usize;
	loop {
		let n = rdr
			.read(&mut buf)
			.with_context(|| format!("failed to read {}", path.display()))?;
		let Some(_more) = n.checked_sub(1) else {
			break;
		};
		lines += buf[..n].iter().filter(|&&c| c == NLB).count();
	}
	Ok(lines)
}

struct GroupHash {
	group: String,
	hash: String,
}

fn group_and_hash(p: &Path, prefixes: &HashSet<String>) -> GroupHash {
	let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
	if let Some(idx) = name.find("__") {
		let h = &name[..idx];
		let rest = &name[idx + 2..];
		return GroupHash {
			group: rest.to_string(),
			hash: h.to_string(),
		};
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
		return GroupHash {
			group: ext,
			hash: stem,
		};
	}
	GroupHash {
		group: stem.clone(),
		hash: stem,
	}
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

enum FileClass {
	Image,
	Table,
}

struct HashedPath {
	hash: String,
	path: PathBuf,
}

struct ParsedTable {
	hash: String,
	headers: Vec<String>,
	rows: Vec<Vec<String>>,
}

struct StemPixels {
	stem: String,
	pixels: Vec<f64>,
}

fn is_junk_name(name: &str) -> Option<()> {
	Some(()).filter(|_u| name.starts_with('.') || name == "__MACOSX")
}

fn walk_data_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
	for entry in fs::read_dir(dir)
		.with_context(|| format!("failed to read directory: {}", dir.display()))?
	{
		let entry =
			entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
		let None = is_junk_name(&entry.file_name().to_string_lossy()) else {
			continue;
		};
		let ft = entry
			.file_type()
			.with_context(|| format!("failed to stat {}", entry.path().display()))?;
		let None = Some(()).filter(|_u| ft.is_symlink()) else {
			continue;
		};
		let p = entry.path();
		for _d in Some(()).filter(|_u| ft.is_dir()).into_iter() {
			walk_data_files(&p, out)?;
		}
		let Some(()) = Some(()).filter(|_u| ft.is_file()) else {
			continue;
		};
		out.push(p);
	}
	Ok(())
}

pub fn load_dir_groups(dir: &str) -> Result<Vec<DirGroup>> {
	let mut files: Vec<PathBuf> = Vec::new();
	walk_data_files(Path::new(dir), &mut files)?;
	files.sort();
	anyhow::ensure!(!files.is_empty(), "no files in {dir}");

	let prefixes: HashSet<String> = files
		.iter()
		.filter_map(|p| {
			let name = p.file_name().and_then(|s| s.to_str())?;
			let idx = name.find("__")?;
			Some(name[..idx].to_string())
		})
		.collect();

	let mut tables: BTreeMap<String, Vec<HashedPath>> = BTreeMap::new();
	let mut images: BTreeMap<String, Vec<HashedPath>> = BTreeMap::new();
	for p in files {
		let gh = group_and_hash(&p, &prefixes);
		match classify_file(&p) {
			FileClass::Image => images.entry(gh.group).or_default().push(HashedPath {
				hash: gh.hash,
				path: p,
			}),
			FileClass::Table => tables.entry(gh.group).or_default().push(HashedPath {
				hash: gh.hash,
				path: p,
			}),
		}
	}

	let mut groups: Vec<DirGroup> = Vec::new();
	let table_names: Vec<String> = tables.keys().cloned().collect();
	for name in table_names {
		let paths = tables.remove(&name).unwrap_or_default();

		let parsed: Vec<ParsedTable> = paths
			.par_iter()
			.filter_map(|hp| match read_raw_csv(&hp.path) {
				Ok(raw) => Some(ParsedTable {
					hash: hp.hash.clone(),
					headers: raw.headers,
					rows: raw.rows,
				}),
				Err(e) => {
					Write::error(format!(
						"WARN: skipping {}: {e}",
						hp.path.display()
					));
					None
				}
			})
			.collect();
		let mut headers: Vec<String> = Vec::new();
		let mut col: HashMap<String, usize> = HashMap::new();
		for pt in &parsed {
			for hd in &pt.headers {
				let None = col.get(hd) else {
					continue;
				};
				col.insert(hd.clone(), headers.len());
				headers.push(hd.clone());
			}
		}
		let Some(_h0) = headers.first() else {
			continue;
		};
		let mut hashes: Vec<String> = Vec::new();
		let mut cells: Vec<Vec<String>> = Vec::new();
		for pt in parsed {
			let map: Vec<usize> = pt.headers.iter().map(|hd| col[hd]).collect();
			for r in pt.rows {
				let mut row = vec![String::new(); headers.len()];
				let mut j = 0usize;
				for v in r {
					let idx = map.get(j);
					j += 1;
					let Some(&u) = idx else {
						continue;
					};
					row[u] = v;
				}
				hashes.push(pt.hash.clone());
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

	let Some(_ne) = images.len().checked_sub(1) else {
		return Ok(groups);
	};
	let total: usize = images.values().map(|v| v.len()).sum();
	Write::line(data, &format!("found images in {}", short_path(dir)));
	let pb = ProgressBar::new(total as u64);
	pb.set_style(
		ProgressStyle::with_template("    {msg} {per_sec} {elapsed} [{bar:30}] {pos}/{len}")
			.map_err(|e| Errored::new(format!("progress template: {e}")))?
			.progress_chars("=>-"),
	);
	pb.enable_steady_tick(Duration::from_millis(120));
	let leaf = Path::new(dir)
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or(dir);
	pb.set_message(format!("decoding images in /{leaf}"));
	let leaf = Path::new(dir)
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or("images")
		.to_string();
	let all: Vec<PathBuf> = images.into_values().flatten().map(|hp| hp.path).collect();
	let first_img = image::open(&all[0])
		.with_context(|| format!("failed to read image dimensions: {}", all[0].display()))?;
	let iw = first_img.width();
	let ih = first_img.height();
	let dim = (iw * ih * 3) as usize;
	let rows: Vec<StemPixels> = all
		.par_iter()
		.map(|p| {
			let stem = p
				.file_stem()
				.and_then(|s| s.to_str())
				.unwrap_or_default()
				.to_string();
			let px = match image_to_row(p.to_str().unwrap_or_default(), iw, ih) {
				Ok(r) => r.to_vec(),
				Err(e) => {
					Write::error(format!(
						"WARN: skipping image {}: {e}",
						p.display()
					));
					vec![f64::NAN; dim]
				}
			};
			pb.inc(1);
			StemPixels { stem, pixels: px }
		})
		.collect();
	let mut hashes: Vec<String> = Vec::with_capacity(rows.len());
	let mut pixels: Vec<Vec<f64>> = Vec::with_capacity(rows.len());
	for sp in rows {
		hashes.push(sp.stem);
		pixels.push(sp.pixels);
	}
	groups.push(DirGroup::Image {
		name: leaf,
		dim,
		hashes,
		pixels,
	});
	pb.finish();
	Write::line(data, "");
	Ok(groups)
}

static ZIP_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn load_zip_groups(path: &str) -> Result<Vec<DirGroup>> {
	let n = ZIP_COUNT.fetch_add(1, Ordering::Relaxed);
	let tmp = env::temp_dir().join(format!("nrecipe_zip_{}_{}", process::id(), n));
	fs::create_dir_all(&tmp)
		.with_context(|| format!("failed to create temp dir {}", tmp.display()))?;

	struct TempDir {
		path: PathBuf,
	}
	impl Drop for TempDir {
		fn drop(&mut self) {
			fs::remove_dir_all(&self.path).ok();
		}
	}
	let guard = TempDir { path: tmp.clone() };

	extract_zip_file(Path::new(path), &tmp)?;
	loop {
		let mut files: Vec<PathBuf> = Vec::new();
		walk_data_files(&tmp, &mut files)?;
		let inner: Vec<PathBuf> = files
			.into_iter()
			.filter(|p| {
				p.extension()
					.and_then(|e| e.to_str())
					.is_some_and(|e| e.eq_ignore_ascii_case("zip"))
			})
			.collect();
		let Some(_more) = inner.len().checked_sub(1) else {
			break;
		};
		for z in inner {
			let dest = z.with_extension("");
			extract_zip_file(&z, &dest)?;
			fs::remove_file(&z)
				.with_context(|| format!("failed to remove inner zip {}", z.display()))?;
		}
	}

	let dir = tmp.to_str().context("temp dir path is not valid UTF-8")?;
	let groups = load_groups(dir)?;
	drop(guard);
	Ok(groups)
}

fn extract_zip_file(zip_path: &Path, dest: &Path) -> Result<()> {
	fs::create_dir_all(dest).with_context(|| format!("failed to create {}", dest.display()))?;
	let file = fs::File::open(zip_path)
		.with_context(|| format!("failed to open zip {}", zip_path.display()))?;
	let mut archive = zip::ZipArchive::new(file)
		.with_context(|| format!("failed to read zip {}", zip_path.display()))?;
	for i in 0..archive.len() {
		let mut entry = archive.by_index(i)?;
		let Some(rel) = entry.enclosed_name() else {
			continue;
		};
		let junk = rel
			.components()
			.find(|c| is_junk_name(&c.as_os_str().to_string_lossy()).is_some());
		let None = junk else {
			continue;
		};
		let out = dest.join(rel);
		let None = Some(()).filter(|_u| entry.is_dir()) else {
			continue;
		};
		for parent in out.parent().into_iter() {
			fs::create_dir_all(parent)
				.with_context(|| format!("failed to create {}", parent.display()))?;
		}
		let mut w = fs::File::create(&out)
			.with_context(|| format!("failed to create {}", out.display()))?;
		io::copy(&mut entry, &mut w)
			.with_context(|| format!("failed to extract {}", out.display()))?;
	}
	Ok(())
}

pub fn load_groups(path: &str) -> Result<Vec<DirGroup>> {
	let p = Path::new(path);
	let ext = p
		.extension()
		.and_then(|e| e.to_str())
		.map(str::to_ascii_lowercase);
	let ext_ref = ext.as_deref();
	if ext_ref == Some("zip") {
		return load_zip_groups(path).context("load zip");
	}
	if matches!(ext_ref, Some("db" | "sqlite")) {
		return load_sqlite_groups(path).context("load sqlite");
	}
	if p.is_dir() {
		return load_dir_groups(path).context("load dir");
	}
	let RawCsv {
		headers,
		rows: cells,
	} = read_raw_csv(p).context("read csv")?;
	let hashes = vec![String::new(); cells.len()];
	return Ok(vec![DirGroup::Table {
		name: String::new(),
		headers,
		hashes,
		cells,
	}]);
}

pub fn split_fields(line: &str) -> Vec<String> {
	let mut out = Vec::new();
	let mut cur = String::new();
	let mut quote = Quote::Out;
	for c in line.chars() {
		match c {
			'\'' => {
				quote = match quote {
					Quote::In => Quote::Out,
					Quote::Out => Quote::In,
				};
			}
			',' => match quote {
				Quote::Out => {
					out.push(cur.trim().to_string());
					cur.clear();
				}
				Quote::In => cur.push(c),
			},
			other => cur.push(other),
		}
	}
	out.push(cur.trim().to_string());
	out
}

#[derive(Clone, Copy)]
enum Quote {
	In,
	Out,
}

struct NameSpec {
	name: String,
	spec: String,
}

fn parse_attribute(line: &str) -> Attr {
	let rest = line["@attribute".len()..].trim();
	let ns = match rest.strip_prefix('\'') {
		Some(r) => match r.find('\'') {
			Some(end) => NameSpec {
				name: r[..end].to_string(),
				spec: r[end + 1..].trim().to_string(),
			},
			None => {
				Write::error("attribute: unterminated quoted name");
				NameSpec {
					name: r.to_string(),
					spec: String::new(),
				}
			}
		},
		None => match rest.find(char::is_whitespace) {
			Some(end) => NameSpec {
				name: rest[..end].to_string(),
				spec: rest[end..].trim().to_string(),
			},
			None => {
				Write::error("attribute: missing type");
				NameSpec {
					name: rest.to_string(),
					spec: String::new(),
				}
			}
		},
	};
	let kind = match ns.spec.strip_prefix('{') {
		Some(_body) => {
			let inner = ns.spec.trim_start_matches('{').trim_end_matches('}');
			Kind::Categorical(split_fields(inner))
		}
		None => Kind::Numeric,
	};
	Attr {
		name: ns.name,
		kind,
	}
}

#[derive(Clone, Copy)]
enum Section {
	Header,
	Data,
}

pub fn parse_arff(path: &str) -> ArffTable {
	let text = match fs::read_to_string(path) {
		Ok(t) => t,
		Err(e) => {
			if e.kind() == io::ErrorKind::NotFound {
				let cwd = env::current_dir()
					.map(|p| p.display().to_string())
					.unwrap_or_else(|_e| ".".to_string());
				let name = Path::new(path)
					.file_name()
					.and_then(|s| s.to_str())
					.unwrap_or(path);
				Write::error(format!("couldn't find '{name}' in {cwd}"));
			} else {
				Write::error(format!("Data: cannot read {path}: {e}"));
			}
			return ArffTable {
				attrs: Vec::new(),
				rows: Vec::new(),
			};
		}
	};
	let mut attrs = Vec::new();
	let mut rows = Vec::new();
	let mut section = Section::Header;
	for raw in text.lines() {
		let line = raw.trim();
		let Some(_c0) = line.chars().next() else {
			continue;
		};
		let None = line.strip_prefix('%') else {
			continue;
		};
		match section {
			Section::Data => rows.push(split_fields(line)),
			Section::Header => {
				let lower = line.to_ascii_lowercase();
				match lower.strip_prefix("@attribute") {
					Some(_rest) => attrs.push(parse_attribute(line)),
					None => {
						let Some(()) = Some(()).filter(|_u| lower.starts_with("@data"))
						else {
							continue;
						};
						section = Section::Data;
					}
				}
			}
		}
	}
	if attrs.is_empty() {
		Write::error(format!("Data: no @attribute lines in {path}"));
		return ArffTable {
			attrs: Vec::new(),
			rows: Vec::new(),
		};
	}
	if rows.is_empty() {
		Write::error(format!("Data: no @data rows in {path}"));
		return ArffTable {
			attrs: Vec::new(),
			rows: Vec::new(),
		};
	}
	return ArffTable { attrs, rows };
}

pub struct ArffTable {
	pub attrs: Vec<Attr>,
	pub rows: Vec<Vec<String>>,
}

pub fn load_sqlite_groups(path: &str) -> Result<Vec<DirGroup>> {
	anyhow::bail!("SQLite loader not yet implemented (.db/.sqlite): {path}")
}

pub(crate) fn short_path(p: &str) -> String {
	let Ok(home) = env::var("HOME") else {
		return p.to_string();
	};
	match p.strip_prefix(&home) {
		Some("") => "~".to_string(),
		Some(rest) => rest
			.strip_prefix('/')
			.map(|r| format!("~/{r}"))
			.unwrap_or_else(|| p.to_string()),
		None => p.to_string(),
	}
}

const IMAGE_EXTENSIONS: &[&str] = &[
	"jpg", "jpeg", "png", "bmp", "gif", "webp", "tiff", "tif", "ico", "pnm", "pbm", "pgm", "ppm",
	"qoi", "dds", "hdr", "exr", "ff",
];

fn classify_file(path: &Path) -> FileClass {
	match path
		.extension()
		.and_then(|e| e.to_str())
		.map(|e| e.to_ascii_lowercase())
		.filter(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
	{
		Some(_ext) => FileClass::Image,
		None => FileClass::Table,
	}
}

fn collect_image_paths(dir: &str) -> Result<Vec<PathBuf>> {
	let mut paths: Vec<PathBuf> = fs::read_dir(dir)
		.with_context(|| format!("failed to read directory: {dir}"))?
		.filter_map(|entry| entry.ok())
		.map(|e| e.path())
		.filter(|p| p.is_file() && matches!(classify_file(p), FileClass::Image))
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
	let mut dat = Vec::with_capacity(paths.len() * row_len);

	for path in &paths {
		let row = image_to_row(
			path.to_str()
				.ok_or_else(|| Errored::new("image path is not valid UTF-8"))?,
			width,
			height,
		)?;
		dat.extend(row.iter());
	}

	let n = dat.len() / row_len;
	Ok(Array2::from_shape_vec([n, row_len], dat)?)
}

pub struct LabeledImages {
	pub x: Mat,
	pub y: Vec1,
}

pub fn load_labeled_image_dir(dir: &str, width: u32, height: u32) -> Result<LabeledImages> {
	let mut subdirs: Vec<PathBuf> = fs::read_dir(dir)
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
			let fname = p
				.file_name()
				.ok_or_else(|| Errored::new("subdir path has no filename component"))?;
			return Ok(fname.to_string_lossy().into_owned());
		})
		.collect::<Result<Vec<String>>>()?;
	let non_float = names.iter().find(|n| n.parse::<f64>().is_err());

	let label_map: Vec<f64> = match non_float {
		None => names
			.iter()
			.map(|n| {
				return n.parse::<f64>().map_err(|e| {
					Errored::new(format!(
						"subdir name failed f64 parse after all_float check: {e}"
					))
					.into()
				});
			})
			.collect::<Result<Vec<f64>>>()?,
		Some(_nf) => (0..names.len()).map(|i| i as f64).collect(),
	};

	let row_len = (width * height * 3) as usize;
	let mut dat = Vec::new();
	let mut labels = Vec::new();

	let mut idx = 0usize;
	for subdir in &subdirs {
		let label = label_map[idx];
		idx += 1;
		let subdir_str = subdir
			.to_str()
			.ok_or_else(|| Errored::new("subdir path is not valid UTF-8"))?;
		let paths = collect_image_paths(subdir_str)?;
		for path in &paths {
			let row = image_to_row(
				path.to_str()
					.ok_or_else(|| Errored::new("image path is not valid UTF-8"))?,
				width,
				height,
			)?;
			dat.extend(row.iter());
			labels.push(label);
		}
	}

	anyhow::ensure!(
		!labels.is_empty(),
		"no images found in any subdirectory of {dir}"
	);
	let n = labels.len();
	let x = Array2::from_shape_vec([n, row_len], dat)?;
	let y = Array1::from_vec(labels);
	Ok(LabeledImages { x, y })
}
