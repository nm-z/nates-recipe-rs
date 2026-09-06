use core::fmt;
use std::{
	collections::{BTreeMap, BTreeSet},
	fs,
	io::{Cursor, Read as _},
	path::{Path, PathBuf},
};

use calamine::Reader as _;
use quick_xml::{Reader, events::Event};
use serde_json::{Map, Value};
use sha2::{Digest as _, Sha256};

use crate::{
	AmbiguousVectorModel, Delimiter, HeaderMode, InferredVectorList, IngestLimits, PreparationRequest, PrepareResult, PreparedDataset, RawTable, SafeTensorLimits, SemanticResult, SemanticType, SourceError, SourceErrorKind, SourceLimit, TableRequest, VectorEncoding,
	gguf::{GgufLimits, GgufMetadataValue, parse_gguf},
	image_header::has_recognized_image_signature,
	parse_safetensors, parse_table,
	prepare::prepare_table_with_semantics,
	read_source_snapshot,
	semantic::{VectorSemantic, VectorSemanticRule, infer_table_vectors_with_semantics},
};

const ARCHIVE_DEPTH_LIMIT: usize = 32;
const METADATA_WIDTH: usize = 14;

/// Structural parser selected for one logical table within a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFormat {
	Delimited,
	Json,
	Text,
	Image,
	Gguf,
	SafeTensors,
	Spreadsheet,
	Presentation,
	Binary,
}

impl SourceFormat {
	#[must_use]
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Delimited => "delimited",
			Self::Json => "json",
			Self::Text => "text",
			Self::Image => "image",
			Self::Gguf => "gguf",
			Self::SafeTensors => "safetensors",
			Self::Spreadsheet => "spreadsheet",
			Self::Presentation => "presentation",
			Self::Binary => "binary",
		}
	}
}

/// Stable failure category for recursive source distillation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DatasetSourceErrorKind {
	InvalidPath,
	Io,
	Symlink,
	EmptySource,
	LimitExceeded,
	MalformedArchive,
	MalformedFormat,
	Ingest,
	ArithmeticOverflow,
}

/// Failure to turn a file, directory, or ZIP container into vectors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetSourceError {
	pub kind: DatasetSourceErrorKind,
	pub path: Option<PathBuf>,
	pub detail: String,
}

impl DatasetSourceError {
	fn new(kind: DatasetSourceErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			path: None,
			detail: detail.into(),
		}
	}

	fn for_path(mut self, path: impl Into<PathBuf>) -> Self {
		self.path = Some(path.into());
		self
	}
}

impl fmt::Display for DatasetSourceError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		if let Some(path) = &self.path {
			write!(formatter, " [{}]", path.display())?;
		}
		Ok(())
	}
}

impl std::error::Error for DatasetSourceError {}

pub type DatasetSourceResult<T> = Result<T, DatasetSourceError>;

/// One rectangular, ordered list of vectors distilled from a filesystem source.
///
/// Rows are samples and columns are vectors. Folder and archive collections
/// retain meaningful source context such as folders, filenames, hashes, byte
/// counts, sample ordinals, sample counts, and source depth. A single regular
/// file exposes only the vectors contained in that file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistilledDataset {
	table: RawTable,
	semantics: Vec<VectorSemanticRule>,
	file_count: usize,
}

impl DistilledDataset {
	#[must_use]
	pub const fn table(&self) -> &RawTable { &self.table }

	#[must_use]
	pub fn sample_count(&self) -> usize { self.table.rows().len() }

	#[must_use]
	pub const fn file_count(&self) -> usize { self.file_count }

	pub fn infer_vectors(&self, model: &impl AmbiguousVectorModel) -> SemanticResult<InferredVectorList> { infer_table_vectors_with_semantics(&self.table, model, &self.semantics) }

	/// Fit semantics and preprocessing state from the exact retained train
	/// partition while preserving source-owned exact semantic declarations.
	///
	/// Unlike separately calling [`Self::infer_vectors`] and applying that result,
	/// this operation never presents validation rows to an infer/classify rule.
	pub fn prepare(&self, request: &PreparationRequest, model: &impl AmbiguousVectorModel) -> PrepareResult<PreparedDataset> { prepare_table_with_semantics(&self.table, request, model, &self.semantics) }
}

#[derive(Clone, Debug)]
struct LogicalTable {
	member: String,
	format: SourceFormat,
	columns: Vec<LogicalColumn>,
	rows: Vec<Vec<Vec<u8>>>,
}

#[derive(Clone, Debug)]
struct LogicalColumn {
	name: Vec<u8>,
	semantic: VectorSemanticRule,
}

#[derive(Clone, Debug)]
struct LogicalField {
	column: LogicalColumn,
	value: Vec<u8>,
}

impl LogicalColumn {
	fn infer(name: impl Into<Vec<u8>>) -> Self {
		Self {
			name: name.into(),
			semantic: VectorSemanticRule::Infer,
		}
	}

	fn classify(name: impl Into<Vec<u8>>) -> Self {
		Self {
			name: name.into(),
			semantic: VectorSemanticRule::Classify,
		}
	}

	fn exact(name: impl Into<Vec<u8>>, semantic_type: SemanticType, encoding: VectorEncoding) -> Self {
		Self {
			name: name.into(),
			semantic: VectorSemanticRule::Exact(VectorSemantic {
				semantic_type,
				encoding,
			}),
		}
	}
}

impl LogicalTable {
	fn new(member: impl Into<String>, format: SourceFormat, columns: Vec<LogicalColumn>, rows: Vec<Vec<Vec<u8>>>) -> DatasetSourceResult<Self> {
		for (index, row) in rows.iter().enumerate() {
			if row.len() != columns.len() {
				return Err(DatasetSourceError::new(
					DatasetSourceErrorKind::MalformedFormat,
					format!(
						"logical record {} contains {} fields, expected {}",
						index + 1,
						row.len(),
						columns.len()
					),
				));
			}
		}
		Ok(Self {
			member: member.into(),
			format,
			columns,
			rows,
		})
	}
}

#[derive(Clone, Debug)]
struct FileContext {
	logical_path: String,
	depth: usize,
	source_index: usize,
	include_source_context: bool,
}

impl FileContext {
	fn child(&self, member: &Path) -> DatasetSourceResult<Self> {
		let member = member.to_str().ok_or_else(|| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::InvalidPath,
				"archive member path is not valid UTF-8",
			)
			.for_path(member)
		})?;
		Ok(Self {
			logical_path: format!("{}/{member}", self.logical_path.trim_end_matches('/')),
			depth: self
				.depth
				.checked_add(Path::new(member).components().count())
				.ok_or_else(|| {
					DatasetSourceError::new(
						DatasetSourceErrorKind::ArithmeticOverflow,
						"source nesting depth overflowed usize",
					)
				})?,
			source_index: self.source_index,
			include_source_context: true,
		})
	}
}

struct Accumulator {
	limits: IngestLimits,
	headers: Vec<Vec<u8>>,
	header_indices: BTreeMap<Vec<u8>, usize>,
	rows: Vec<Vec<Vec<u8>>>,
	semantics: Vec<VectorSemanticRule>,
	file_count: usize,
	leaf_bytes: u64,
}

impl Accumulator {
	fn new(limits: IngestLimits) -> Self {
		Self {
			limits,
			headers: Vec::new(),
			header_indices: BTreeMap::new(),
			rows: Vec::new(),
			semantics: Vec::new(),
			file_count: 0,
			leaf_bytes: 0,
		}
	}

	fn admit_leaf(&mut self, context: &FileContext, bytes: &[u8]) -> DatasetSourceResult<()> {
		let byte_count = u64::try_from(bytes.len()).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::ArithmeticOverflow,
				format!("file byte count cannot be represented as u64: {error}"),
			)
			.for_path(&context.logical_path)
		})?;
		self.leaf_bytes = self.leaf_bytes.checked_add(byte_count).ok_or_else(|| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::ArithmeticOverflow,
				"aggregate source byte count overflowed u64",
			)
			.for_path(&context.logical_path)
		})?;
		if self.leaf_bytes > self.limits.source_bytes().get() {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::LimitExceeded,
				format!(
					"recursive source contains at least {} leaf bytes, limit is {}",
					self.leaf_bytes,
					self.limits.source_bytes()
				),
			)
			.for_path(&context.logical_path));
		}
		self.file_count = self.file_count.checked_add(1).ok_or_else(|| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::ArithmeticOverflow,
				"recursive file count overflowed usize",
			)
		})?;
		Ok(())
	}

	fn append(&mut self, context: &FileContext, bytes: &[u8], mut logical: LogicalTable) -> DatasetSourceResult<()> {
		if logical.rows.is_empty() {
			logical.rows.push(vec![Vec::new(); logical.columns.len()]);
		}
		let sample_count = logical.rows.len();
		let prospective = self.rows.len().checked_add(sample_count).ok_or_else(|| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::ArithmeticOverflow,
				"aggregate sample count overflowed usize",
			)
		})?;
		let prospective_u64 = u64::try_from(prospective).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::ArithmeticOverflow,
				format!("aggregate sample count cannot be represented as u64: {error}"),
			)
		})?;
		if prospective_u64 > self.limits.records().get() {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::LimitExceeded,
				format!(
					"recursive source contains at least {prospective_u64} samples, limit is {}",
					self.limits.records()
				),
			)
			.for_path(&context.logical_path));
		}

		let source_context = if context.include_source_context {
			Some(SourceMetadata::new(
				context,
				bytes,
				logical.format,
				&logical.member,
				sample_count,
			)?)
		} else {
			None
		};
		let source_context_width = source_context.as_ref().map_or(0, |_| METADATA_WIDTH);
		let mut local_indices = Vec::with_capacity(source_context_width + logical.columns.len());
		if let Some(source_context) = &source_context {
			for field in source_context.fields(0) {
				local_indices.push(self.resolve_header(
					field.column.name,
					field.column.semantic,
					&context.logical_path,
				)?);
			}
		}
		let mut local_names = BTreeSet::new();
		for (index, column) in logical.columns.drain(..).enumerate() {
			let LogicalColumn {
				name: header,
				semantic,
			} = column;
			let mut header = if context.include_source_context && is_source_context_header(&header) {
				let mut prefixed = b"data:".to_vec();
				prefixed.extend_from_slice(&header);
				prefixed
			} else if header.is_empty() {
				format!("col{}", index + 1).into_bytes()
			} else {
				header
			};
			if !local_names.insert(header.clone()) {
				let original = header.clone();
				let mut ordinal = 2usize;
				loop {
					header = original.clone();
					header.extend_from_slice(format!("#{ordinal}").as_bytes());
					if local_names.insert(header.clone()) {
						break;
					}
					ordinal = ordinal.checked_add(1).ok_or_else(|| {
						DatasetSourceError::new(
							DatasetSourceErrorKind::ArithmeticOverflow,
							"duplicate column ordinal overflowed usize",
						)
					})?;
				}
			}
			local_indices.push(self.resolve_header(header, semantic, &context.logical_path)?);
		}

		for (sample_index, source_row) in logical.rows.into_iter().enumerate() {
			let mut row = vec![Vec::new(); self.headers.len()];
			let mut values = Vec::with_capacity(source_context_width + source_row.len());
			if let Some(source_context) = &source_context {
				values.extend(
					source_context
						.fields(sample_index)
						.into_iter()
						.map(|field| field.value),
				);
			}
			values.extend(source_row);
			for (value, global_index) in values.into_iter().zip(&local_indices) {
				check_field_bound(&value, self.limits, &context.logical_path)?;
				row[*global_index] = value;
			}
			self.rows.push(row);
		}
		Ok(())
	}

	fn resolve_header(&mut self, header: Vec<u8>, semantic: VectorSemanticRule, path: &str) -> DatasetSourceResult<usize> {
		if let Some(index) = self.header_indices.get(&header).copied() {
			if self.semantics[index] != semantic {
				self.semantics[index] = VectorSemanticRule::Infer;
			}
			return Ok(index);
		}
		let width = self.headers.len().checked_add(1).ok_or_else(|| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::ArithmeticOverflow,
				"aggregate vector count overflowed usize",
			)
			.for_path(path)
		})?;
		if width > self.limits.fields_per_record().get() as usize {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::LimitExceeded,
				format!(
					"recursive source contains at least {width} vectors, limit is {}",
					self.limits.fields_per_record()
				),
			)
			.for_path(path));
		}
		let index = self.headers.len();
		self.header_indices.insert(header.clone(), index);
		self.headers.push(header);
		self.semantics.push(semantic);
		for row in &mut self.rows {
			row.push(Vec::new());
		}
		Ok(index)
	}

	fn finish(self) -> DatasetSourceResult<DistilledDataset> {
		if self.file_count == 0 || self.rows.is_empty() {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::EmptySource,
				"dataset source contains no regular files or samples",
			));
		}
		let table = RawTable::from_parts(self.headers, self.rows).map_err(|error| DatasetSourceError::new(DatasetSourceErrorKind::Ingest, error.to_string()))?;
		Ok(DistilledDataset {
			table,
			semantics: self.semantics,
			file_count: self.file_count,
		})
	}
}

/// Distill a regular file, recursive directory, or recursively nested ZIP.
///
/// The specialized readers cover delimited text, JSON, images, GGUF,
/// safetensors, XLSX, and PPTX. Other UTF-8 files remain one lossless text
/// sample; other opaque files remain one lossless binary sample. Directory and
/// archive traversal is deterministic and refuses symbolic links or escaping
/// archive paths.
///
/// # Errors
///
/// Returns an error when the path or encoded structure is invalid, a configured
/// byte/record/vector bound is exceeded, or no samples can be produced.
pub fn distill_dataset(path: &Path, limits: IngestLimits) -> DatasetSourceResult<DistilledDataset> { distill_datasets([path], limits) }

/// Distill an ordered collection of regular files, recursive directories, or
/// recursively nested ZIPs into one rectangular vector list.
///
/// Declared source order is retained as row order. Aggregate byte, record, and
/// vector limits apply across the whole collection, so splitting one logical
/// dataset across several declarations cannot bypass a bound. When more than
/// one source is declared, regular files retain the same source metadata as
/// directory and archive members, including their declaration index.
///
/// # Errors
///
/// Returns an error under the same fail-closed contract as
/// [`distill_dataset`], or when no source is declared.
pub fn distill_datasets<P>(paths: impl IntoIterator<Item = P>, limits: IngestLimits) -> DatasetSourceResult<DistilledDataset>
where P: AsRef<Path> {
	let paths = paths
		.into_iter()
		.map(|path| path.as_ref().to_path_buf())
		.collect::<Vec<_>>();
	if paths.is_empty() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::EmptySource,
			"at least one dataset source is required",
		));
	}
	let include_source_context = paths.len() > 1;
	let mut accumulator = Accumulator::new(limits);
	for (source_index, path) in paths.iter().enumerate() {
		distill_one_source(path, source_index, include_source_context, &mut accumulator)?;
	}
	accumulator.finish()
}

fn distill_one_source(path: &Path, source_index: usize, include_source_context: bool, accumulator: &mut Accumulator) -> DatasetSourceResult<()> {
	let metadata = fs::symlink_metadata(path).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::Io,
			format!("inspect dataset source: {error}"),
		)
		.for_path(path)
	})?;
	if metadata.file_type().is_symlink() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::Symlink,
			"dataset source must not be a symbolic link",
		)
		.for_path(path));
	}
	if metadata.is_dir() {
		visit_directory(path, path, source_index, accumulator)?;
	} else if metadata.is_file() {
		let bytes = read_regular_file(path, accumulator.limits)?;
		visit_bytes(
			FileContext {
				logical_path: path.display().to_string(),
				depth: 0,
				source_index,
				include_source_context,
			},
			&bytes,
			0,
			accumulator,
		)?;
	} else {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::InvalidPath,
			"dataset source is neither a regular file nor a directory",
		)
		.for_path(path));
	}
	Ok(())
}

fn visit_directory(root: &Path, directory: &Path, source_index: usize, accumulator: &mut Accumulator) -> DatasetSourceResult<()> {
	let mut entries = fs::read_dir(directory)
		.map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::Io,
				format!("read dataset directory: {error}"),
			)
			.for_path(directory)
		})?
		.collect::<Result<Vec<_>, _>>()
		.map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::Io,
				format!("read dataset directory entry: {error}"),
			)
			.for_path(directory)
		})?;
	entries.sort_by_key(fs::DirEntry::file_name);
	for entry in entries {
		let path = entry.path();
		let file_type = entry.file_type().map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::Io,
				format!("inspect dataset directory entry: {error}"),
			)
			.for_path(&path)
		})?;
		if file_type.is_symlink() {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::Symlink,
				"recursive dataset entries must not be symbolic links",
			)
			.for_path(path));
		}
		if file_type.is_dir() {
			visit_directory(root, &path, source_index, accumulator)?;
		} else if file_type.is_file() {
			let relative = path.strip_prefix(root).map_err(|error| {
				DatasetSourceError::new(
					DatasetSourceErrorKind::InvalidPath,
					format!("dataset member is outside its root: {error}"),
				)
				.for_path(&path)
			})?;
			let root_label = root
				.file_name()
				.map(|name| name.to_string_lossy().into_owned())
				.unwrap_or_else(|| root.display().to_string());
			let logical_path = Path::new(&root_label).join(relative);
			let bytes = read_regular_file(&path, accumulator.limits)?;
			visit_bytes(
				FileContext {
					logical_path: logical_path.to_string_lossy().into_owned(),
					depth: relative.components().count(),
					source_index,
					include_source_context: true,
				},
				&bytes,
				0,
				accumulator,
			)?;
		}
	}
	Ok(())
}

fn read_regular_file(path: &Path, limits: IngestLimits) -> DatasetSourceResult<Vec<u8>> {
	let limit = SourceLimit::new(limits.source_bytes().get()).map_err(dataset_error_from_source)?;
	read_source_snapshot(path, limit)
		.map(|snapshot| snapshot.into_bytes())
		.map_err(dataset_error_from_source)
}

fn dataset_error_from_source(error: SourceError) -> DatasetSourceError {
	let kind = match error.kind {
		SourceErrorKind::InvalidLimit | SourceErrorKind::LimitExceeded => DatasetSourceErrorKind::LimitExceeded,
		SourceErrorKind::Io => DatasetSourceErrorKind::Io,
		SourceErrorKind::NotRegularFile => DatasetSourceErrorKind::InvalidPath,
		SourceErrorKind::ArithmeticOverflow => DatasetSourceErrorKind::ArithmeticOverflow,
	};
	DatasetSourceError {
		kind,
		path: error.path,
		detail: error.detail,
	}
}

fn visit_bytes(context: FileContext, bytes: &[u8], archive_depth: usize, accumulator: &mut Accumulator) -> DatasetSourceResult<()> {
	let extension = extension(&context.logical_path);
	if matches!(extension.as_str(), "xlsx") {
		accumulator.admit_leaf(&context, bytes)?;
		for table in parse_spreadsheet(bytes, &context.logical_path, accumulator.limits)? {
			accumulator.append(&context, bytes, table)?;
		}
		return Ok(());
	}
	if matches!(extension.as_str(), "pptx") {
		accumulator.admit_leaf(&context, bytes)?;
		accumulator.append(
			&context,
			bytes,
			parse_presentation(bytes, &context.logical_path, accumulator.limits)?,
		)?;
		return Ok(());
	}
	if extension == "zip" || is_zip(bytes) {
		return visit_archive(context, bytes, archive_depth, accumulator);
	}
	accumulator.admit_leaf(&context, bytes)?;
	for table in parse_leaf(&context, bytes, accumulator.limits)? {
		accumulator.append(&context, bytes, table)?;
	}
	Ok(())
}

fn visit_archive(context: FileContext, bytes: &[u8], archive_depth: usize, accumulator: &mut Accumulator) -> DatasetSourceResult<()> {
	if archive_depth >= ARCHIVE_DEPTH_LIMIT {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::LimitExceeded,
			format!("archive nesting exceeds {ARCHIVE_DEPTH_LIMIT} levels"),
		)
		.for_path(&context.logical_path));
	}
	let reader = Cursor::new(bytes);
	let mut archive = zip::ZipArchive::new(reader).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedArchive,
			format!("open ZIP archive: {error}"),
		)
		.for_path(&context.logical_path)
	})?;
	let mut entries = Vec::with_capacity(archive.len());
	for index in 0..archive.len() {
		let entry = archive.by_index_raw(index).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedArchive,
				format!("inspect ZIP member {index}: {error}"),
			)
			.for_path(&context.logical_path)
		})?;
		if entry.is_dir() {
			continue;
		}
		let enclosed = entry.enclosed_name().ok_or_else(|| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedArchive,
				format!("ZIP member {:?} escapes its archive root", entry.name()),
			)
			.for_path(&context.logical_path)
		})?;
		entries.push((enclosed.to_path_buf(), index, entry.size()));
	}
	entries.sort_by(|left, right| left.0.cmp(&right.0));
	if entries.is_empty() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::EmptySource,
			"ZIP archive contains no files",
		)
		.for_path(&context.logical_path));
	}
	for (member, index, declared_bytes) in entries {
		if declared_bytes > accumulator.limits.source_bytes().get() {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::LimitExceeded,
				format!(
					"ZIP member reports {declared_bytes} bytes, limit is {}",
					accumulator.limits.source_bytes()
				),
			)
			.for_path(context.child(&member)?.logical_path));
		}
		let entry = archive.by_index(index).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedArchive,
				format!("open ZIP member {}: {error}", member.display()),
			)
			.for_path(&context.logical_path)
		})?;
		let take = accumulator
			.limits
			.source_bytes()
			.get()
			.checked_add(1)
			.ok_or_else(|| {
				DatasetSourceError::new(
					DatasetSourceErrorKind::ArithmeticOverflow,
					"ZIP member read bound overflowed u64",
				)
			})?;
		let mut decoded = Vec::with_capacity(
			usize::try_from(declared_bytes)
				.unwrap_or(1_048_576)
				.min(1_048_576),
		);
		entry.take(take)
			.read_to_end(&mut decoded)
			.map_err(|error| {
				DatasetSourceError::new(
					DatasetSourceErrorKind::MalformedArchive,
					format!("read ZIP member {}: {error}", member.display()),
				)
				.for_path(&context.logical_path)
			})?;
		if u64::try_from(decoded.len()).unwrap_or(u64::MAX) > accumulator.limits.source_bytes().get() {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::LimitExceeded,
				format!(
					"expanded ZIP member exceeds {} bytes",
					accumulator.limits.source_bytes()
				),
			)
			.for_path(context.child(&member)?.logical_path));
		}
		visit_bytes(
			context.child(&member)?,
			&decoded,
			archive_depth + 1,
			accumulator,
		)?;
	}
	Ok(())
}

fn parse_leaf(context: &FileContext, bytes: &[u8], limits: IngestLimits) -> DatasetSourceResult<Vec<LogicalTable>> {
	let extension = extension(&context.logical_path);
	match extension.as_str() {
		"csv" | "tsv" => {
			Ok(vec![parse_delimited(
				bytes,
				&extension,
				HeaderMode::Present,
				context,
				limits,
			)?])
		}
		"all-data" | "dat" | "data" | "data-numeric" | "tra" | "trn" => {
			Ok(vec![parse_delimited(
				bytes,
				&extension,
				HeaderMode::Absent,
				context,
				limits,
			)?])
		}
		"json" => parse_json(bytes, context),
		"gguf" => parse_gguf_tables(bytes, context),
		"safetensors" => parse_safetensor_tables(bytes, context),
		"png" if is_png(bytes) => {
			Ok(vec![single_payload(
				SourceFormat::Image,
				b"image",
				bytes,
				SemanticType::Image,
			)])
		}
		"inp" | "out" | "patch" | "sh" => Ok(vec![parse_text(bytes, context)?]),
		"txt" if looks_tabular(bytes) => {
			Ok(vec![parse_delimited(
				bytes,
				&extension,
				HeaderMode::Absent,
				context,
				limits,
			)?])
		}
		"txt" => Ok(vec![parse_text(bytes, context)?]),
		"bin" | "logits" | "model" => {
			Ok(vec![single_payload(
				SourceFormat::Binary,
				b"binary",
				bytes,
				SemanticType::Binary,
			)])
		}
		_ if is_image(bytes) => {
			Ok(vec![single_payload(
				SourceFormat::Image,
				b"image",
				bytes,
				SemanticType::Image,
			)])
		}
		_ if is_probably_text(bytes) && looks_tabular(bytes) => {
			Ok(vec![parse_delimited(
				bytes,
				&extension,
				HeaderMode::Absent,
				context,
				limits,
			)?])
		}
		_ if is_probably_text(bytes) => Ok(vec![parse_text(bytes, context)?]),
		_ => {
			Ok(vec![single_payload(
				SourceFormat::Binary,
				b"binary",
				bytes,
				SemanticType::Binary,
			)])
		}
	}
}

fn parse_delimited(bytes: &[u8], extension: &str, header: HeaderMode, context: &FileContext, limits: IngestLimits) -> DatasetSourceResult<LogicalTable> {
	let delimiter = match extension {
		"tsv" => Delimiter::Tab,
		"csv" => Delimiter::Auto,
		_ => structural_delimiter(bytes).unwrap_or(Delimiter::Auto),
	};
	let raw = parse_table(bytes, TableRequest::new(delimiter, header, limits)).map_err(|error| DatasetSourceError::new(DatasetSourceErrorKind::Ingest, error.to_string()).for_path(&context.logical_path))?;
	LogicalTable::new(
		"",
		SourceFormat::Delimited,
		raw.headers()
			.iter()
			.cloned()
			.map(LogicalColumn::infer)
			.collect(),
		raw.rows().to_vec(),
	)
}

fn parse_text(bytes: &[u8], context: &FileContext) -> DatasetSourceResult<LogicalTable> {
	core::str::from_utf8(bytes).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			format!("text source is not UTF-8: {error}"),
		)
		.for_path(&context.logical_path)
	})?;
	LogicalTable::new(
		"",
		SourceFormat::Text,
		vec![LogicalColumn::exact(
			b"text".to_vec(),
			SemanticType::Text,
			VectorEncoding::Utf8,
		)],
		vec![vec![bytes.to_vec()]],
	)
}

fn single_payload(format: SourceFormat, header: &[u8], bytes: &[u8], semantic_type: SemanticType) -> LogicalTable {
	LogicalTable {
		member: String::new(),
		format,
		columns: vec![LogicalColumn::exact(
			header.to_vec(),
			semantic_type,
			VectorEncoding::Bytes,
		)],
		rows: vec![vec![bytes.to_vec()]],
	}
}

fn parse_json(bytes: &[u8], context: &FileContext) -> DatasetSourceResult<Vec<LogicalTable>> {
	let value: Value = serde_json::from_slice(bytes).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			format!("parse JSON source: {error}"),
		)
		.for_path(&context.logical_path)
	})?;
	match value {
		Value::Array(values) => Ok(vec![json_values_to_table("", values)?]),
		Value::Object(map) => json_object_to_tables(map),
		value => Ok(vec![json_values_to_table("", vec![value])?]),
	}
}

fn json_object_to_tables(map: Map<String, Value>) -> DatasetSourceResult<Vec<LogicalTable>> {
	if map.is_empty() {
		return Ok(vec![json_values_to_table("", vec![Value::Object(map)])?]);
	}
	if map.values().all(is_object_array) {
		return map
			.into_iter()
			.map(|(member, value)| {
				let Value::Array(values) = value else {
					unreachable!("all map values were checked as arrays");
				};
				json_values_to_table(member, values)
			})
			.collect();
	}
	if map.values().all(Value::is_object) {
		let split_shape = map.values().all(|value| {
			value.as_object()
				.is_some_and(|object| !object.is_empty() && object.values().all(Value::is_array))
		});
		if split_shape {
			let mut members = BTreeSet::new();
			for value in map.values() {
				if let Some(object) = value.as_object() {
					members.extend(object.keys().cloned());
				}
			}
			let mut tables = Vec::new();
			for member in members {
				let mut keys = Vec::new();
				let mut rows = Vec::new();
				for (key, value) in &map {
					let Some(Value::Array(values)) = value.get(&member) else {
						continue;
					};
					for value in values {
						keys.push(key.clone());
						rows.push(value.clone());
					}
				}
				tables.push(json_keyed_values_to_table(member, keys, rows)?);
			}
			return Ok(tables);
		}
		let mut keys = Vec::with_capacity(map.len());
		let mut values = Vec::with_capacity(map.len());
		for (key, value) in map {
			keys.push(key);
			values.push(value);
		}
		return Ok(vec![json_keyed_values_to_table("", keys, values)?]);
	}
	Ok(vec![json_values_to_table("", vec![Value::Object(map)])?])
}

fn json_keyed_values_to_table(member: impl Into<String>, keys: Vec<String>, values: Vec<Value>) -> DatasetSourceResult<LogicalTable> {
	let mut table = json_values_to_table(member, values)?;
	let mut rows = keys
		.into_iter()
		.map(|key| vec![key.into_bytes()])
		.collect::<Vec<_>>();
	if rows.len() != table.rows.len() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			"JSON key vector does not align with its values",
		));
	}
	for (row, source) in table.rows.drain(..).zip(&mut rows) {
		source.extend(row);
	}
	table.columns
		.insert(0, LogicalColumn::classify(b"key".to_vec()));
	table.rows = rows;
	Ok(table)
}

fn json_values_to_table(member: impl Into<String>, values: Vec<Value>) -> DatasetSourceResult<LogicalTable> {
	let member = member.into();
	if values.iter().all(Value::is_object) {
		let mut headers = Vec::new();
		let mut seen = BTreeSet::new();
		for value in &values {
			if let Some(object) = value.as_object() {
				for key in object.keys() {
					if seen.insert(key.clone()) {
						headers.push(key.clone());
					}
				}
			}
		}
		let rows = values
			.iter()
			.map(|value| {
				let object = value.as_object();
				headers
					.iter()
					.map(|header| {
						object.and_then(|map| map.get(header))
							.map_or_else(Vec::new, json_cell)
					})
					.collect::<Vec<_>>()
			})
			.collect::<Vec<_>>();
		return LogicalTable::new(
			member,
			SourceFormat::Json,
			headers
				.into_iter()
				.map(String::into_bytes)
				.map(LogicalColumn::infer)
				.collect(),
			rows,
		);
	}
	if values.iter().all(Value::is_array) {
		let width = values
			.iter()
			.filter_map(Value::as_array)
			.map(Vec::len)
			.max()
			.unwrap_or(0);
		let rows = values
			.iter()
			.map(|value| {
				let values = value.as_array();
				(0..width)
					.map(|index| {
						values.and_then(|row| row.get(index))
							.map_or_else(Vec::new, json_cell)
					})
					.collect::<Vec<_>>()
			})
			.collect::<Vec<_>>();
		return LogicalTable::new(
			member,
			SourceFormat::Json,
			(0..width)
				.map(|index| LogicalColumn::infer(format!("col{}", index + 1).into_bytes()))
				.collect(),
			rows,
		);
	}
	LogicalTable::new(
		member,
		SourceFormat::Json,
		vec![LogicalColumn::infer(b"value".to_vec())],
		values.iter().map(|value| vec![json_cell(value)]).collect(),
	)
}

fn json_cell(value: &Value) -> Vec<u8> {
	match value {
		Value::Null => Vec::new(),
		Value::Bool(value) => value.to_string().into_bytes(),
		Value::Number(value) => value.to_string().into_bytes(),
		Value::String(value) => value.as_bytes().to_vec(),
		Value::Array(_) | Value::Object(_) => value.to_string().into_bytes(),
	}
}

fn is_object_array(value: &Value) -> bool {
	value.as_array()
		.is_some_and(|values| values.iter().all(Value::is_object))
}

fn parse_spreadsheet(bytes: &[u8], path: &str, limits: IngestLimits) -> DatasetSourceResult<Vec<LogicalTable>> {
	preflight_zip_expansion(bytes, path, limits, "XLSX")?;
	let cursor = Cursor::new(bytes);
	let mut workbook: calamine::Xlsx<_> = calamine::Xlsx::new(cursor).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			format!("open XLSX workbook: {error}"),
		)
		.for_path(path)
	})?;
	let sheet_names = workbook.sheet_names().to_vec();
	if sheet_names.is_empty() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			"XLSX workbook contains no worksheets",
		)
		.for_path(path));
	}
	let mut tables = Vec::new();
	for sheet in sheet_names {
		let range = workbook.worksheet_range(&sheet).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedFormat,
				format!("read XLSX worksheet {sheet:?}: {error}"),
			)
			.for_path(path)
		})?;
		let mut source_rows = range.rows();
		let Some(first) = source_rows.next() else {
			continue;
		};
		let first_is_data = first.iter().all(excel_cell_is_data);
		let headers = if first_is_data {
			(0..first.len())
				.map(|index| format!("col{}", index + 1).into_bytes())
				.collect::<Vec<_>>()
		} else {
			first.iter()
				.enumerate()
				.map(|(index, value)| {
					let value = excel_cell(value);
					if value.is_empty() {
						format!("col{}", index + 1).into_bytes()
					} else {
						value
					}
				})
				.collect::<Vec<_>>()
		};
		let mut rows = Vec::new();
		if first_is_data {
			rows.push(first.iter().map(excel_cell).collect());
		}
		rows.extend(source_rows.map(|row| {
			let mut values = row.iter().map(excel_cell).collect::<Vec<_>>();
			values.resize(headers.len(), Vec::new());
			values.truncate(headers.len());
			values
		}));
		tables.push(LogicalTable::new(
			sheet,
			SourceFormat::Spreadsheet,
			headers.iter().cloned().map(LogicalColumn::infer).collect(),
			rows,
		)?);
	}
	if tables.is_empty() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::EmptySource,
			"XLSX workbook contains no nonempty worksheets",
		)
		.for_path(path));
	}
	Ok(tables)
}

fn excel_cell_is_data(value: &calamine::Data) -> bool {
	matches!(
		value,
		calamine::Data::Int(_) | calamine::Data::Float(_) | calamine::Data::Bool(_) | calamine::Data::DateTime(_) | calamine::Data::DateTimeIso(_) | calamine::Data::DurationIso(_)
	)
}

fn excel_cell(value: &calamine::Data) -> Vec<u8> {
	match value {
		calamine::Data::Empty => Vec::new(),
		calamine::Data::String(value) => value.as_bytes().to_vec(),
		calamine::Data::Float(value) => value.to_string().into_bytes(),
		calamine::Data::Int(value) => value.to_string().into_bytes(),
		calamine::Data::Bool(value) => value.to_string().into_bytes(),
		calamine::Data::Error(value) => format!("{value:?}").into_bytes(),
		calamine::Data::DateTime(value) => value.to_string().into_bytes(),
		calamine::Data::DateTimeIso(value) | calamine::Data::DurationIso(value) => value.as_bytes().to_vec(),
	}
}

fn parse_presentation(bytes: &[u8], path: &str, limits: IngestLimits) -> DatasetSourceResult<LogicalTable> {
	preflight_zip_expansion(bytes, path, limits, "PPTX")?;
	let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedArchive,
			format!("open PPTX container: {error}"),
		)
		.for_path(path)
	})?;
	let mut slides = Vec::new();
	for index in 0..archive.len() {
		let mut entry = archive.by_index(index).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedArchive,
				format!("open PPTX member {index}: {error}"),
			)
			.for_path(path)
		})?;
		let Some(slide) = pptx_slide_number(entry.name()) else {
			continue;
		};
		let mut xml = String::new();
		entry.read_to_string(&mut xml).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedFormat,
				format!("read PPTX slide {slide}: {error}"),
			)
			.for_path(path)
		})?;
		slides.push((slide, xml));
	}
	slides.sort_by_key(|(slide, _)| *slide);
	if slides.is_empty() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			"PPTX container contains no slides",
		)
		.for_path(path));
	}
	let mut rows = Vec::new();
	for (slide, xml) in slides {
		for text in pptx_paragraphs(&xml, path, slide)? {
			rows.push(vec![slide.to_string().into_bytes(), text.into_bytes()]);
		}
	}
	LogicalTable::new(
		"",
		SourceFormat::Presentation,
		vec![
			LogicalColumn::exact(
				b"slide".to_vec(),
				SemanticType::Numeric,
				VectorEncoding::I32,
			),
			LogicalColumn::exact(b"text".to_vec(), SemanticType::Text, VectorEncoding::Utf8),
		],
		rows,
	)
}

fn pptx_slide_number(name: &str) -> Option<usize> {
	name.strip_prefix("ppt/slides/slide")?
		.strip_suffix(".xml")?
		.parse()
		.ok()
}

fn pptx_paragraphs(xml: &str, path: &str, slide: usize) -> DatasetSourceResult<Vec<String>> {
	let mut reader = Reader::from_str(xml);
	let mut paragraphs = Vec::new();
	let mut paragraph = String::new();
	let mut in_paragraph = false;
	let mut in_text = false;
	loop {
		match reader.read_event().map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedFormat,
				format!("parse PPTX slide {slide} XML: {error}"),
			)
			.for_path(path)
		})? {
			Event::Start(element) => {
				match element.local_name().as_ref() {
					b"p" => in_paragraph = true,
					b"t" => in_text = in_paragraph,
					b"br" => paragraph.push(' '),
					_ => {}
				}
			}
			Event::Empty(element) if element.local_name().as_ref() == b"br" => paragraph.push(' '),
			Event::End(element) => {
				match element.local_name().as_ref() {
					b"p" => {
						in_paragraph = false;
						let text = paragraph.trim();
						if !text.is_empty() {
							paragraphs.push(text.to_owned());
						}
						paragraph.clear();
					}
					b"t" => in_text = false,
					_ => {}
				}
			}
			Event::Text(text) if in_text => {
				let decoded = text.xml10_content().map_err(|error| {
					DatasetSourceError::new(
						DatasetSourceErrorKind::MalformedFormat,
						format!("decode PPTX slide {slide} text: {error}"),
					)
					.for_path(path)
				})?;
				paragraph.push_str(&decoded);
			}
			Event::GeneralRef(reference) if in_text => {
				if let Some(value) = reference.resolve_char_ref().map_err(|error| {
					DatasetSourceError::new(
						DatasetSourceErrorKind::MalformedFormat,
						format!("decode PPTX slide {slide} character reference: {error}"),
					)
					.for_path(path)
				})? {
					paragraph.push(value);
				}
			}
			Event::Eof => break,
			_ => {}
		}
	}
	Ok(paragraphs)
}

fn parse_gguf_tables(bytes: &[u8], context: &FileContext) -> DatasetSourceResult<Vec<LogicalTable>> {
	let file_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX).max(1);
	let limits = GgufLimits::new(
		file_bytes, file_bytes, file_bytes, 64, file_bytes, file_bytes, 64,
	)
	.map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			format!("construct GGUF parser limits: {error}"),
		)
		.for_path(&context.logical_path)
	})?;
	let archive = parse_gguf(bytes, limits).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			format!("parse GGUF source: {error}"),
		)
		.for_path(&context.logical_path)
	})?;
	let mut tables = Vec::new();
	if !archive.metadata().is_empty() {
		let rows = archive
			.metadata()
			.iter()
			.map(|entry| {
				vec![
					entry.key().as_bytes().to_vec(),
					entry.value().value_type().code().to_string().into_bytes(),
					gguf_metadata_value(entry.value()),
				]
			})
			.collect();
		tables.push(LogicalTable::new(
			"metadata",
			SourceFormat::Gguf,
			vec![
				LogicalColumn::classify(b"metadata_key".to_vec()),
				LogicalColumn::classify(b"metadata_type".to_vec()),
				LogicalColumn::infer(b"metadata_value".to_vec()),
			],
			rows,
		)?);
	}
	if !archive.tensors().is_empty() {
		let rows = archive
			.tensors()
			.iter()
			.map(|tensor| {
				vec![
					tensor.name().as_bytes().to_vec(),
					tensor.tensor_type().code().to_string().into_bytes(),
					tensor.dimensions()
						.iter()
						.map(u64::to_string)
						.collect::<Vec<_>>()
						.join("x")
						.into_bytes(),
					tensor.dimensions().len().to_string().into_bytes(),
					tensor.encoded_bytes().to_string().into_bytes(),
					archive
						.raw_tensor(tensor.name())
						.unwrap_or_default()
						.to_vec(),
				]
			})
			.collect();
		tables.push(LogicalTable::new(
			"tensors",
			SourceFormat::Gguf,
			vec![
				LogicalColumn::classify(b"tensor_name".to_vec()),
				LogicalColumn::classify(b"tensor_type".to_vec()),
				LogicalColumn::classify(b"tensor_shape".to_vec()),
				LogicalColumn::exact(
					b"tensor_rank".to_vec(),
					SemanticType::Numeric,
					VectorEncoding::I32,
				),
				LogicalColumn::exact(
					b"tensor_bytes".to_vec(),
					SemanticType::Numeric,
					VectorEncoding::I32,
				),
				LogicalColumn::exact(
					b"binary".to_vec(),
					SemanticType::Binary,
					VectorEncoding::Bytes,
				),
			],
			rows,
		)?);
	}
	if tables.is_empty() {
		tables.push(single_payload(
			SourceFormat::Gguf,
			b"binary",
			bytes,
			SemanticType::Binary,
		));
	}
	Ok(tables)
}

fn gguf_metadata_value(value: &GgufMetadataValue<'_>) -> Vec<u8> {
	match value {
		GgufMetadataValue::U8(value) => value.to_string().into_bytes(),
		GgufMetadataValue::I8(value) => value.to_string().into_bytes(),
		GgufMetadataValue::U16(value) => value.to_string().into_bytes(),
		GgufMetadataValue::I16(value) => value.to_string().into_bytes(),
		GgufMetadataValue::U32(value) => value.to_string().into_bytes(),
		GgufMetadataValue::I32(value) => value.to_string().into_bytes(),
		GgufMetadataValue::F32Bits(value) => format!("f32:0x{value:08x}").into_bytes(),
		GgufMetadataValue::Bool(value) => value.to_string().into_bytes(),
		GgufMetadataValue::String(value) => value.as_bytes().to_vec(),
		GgufMetadataValue::Array(value) => format!("{:?}", value.values()).into_bytes(),
		GgufMetadataValue::U64(value) => value.to_string().into_bytes(),
		GgufMetadataValue::I64(value) => value.to_string().into_bytes(),
		GgufMetadataValue::F64Bits(value) => format!("f64:0x{value:016x}").into_bytes(),
	}
}

fn parse_safetensor_tables(bytes: &[u8], context: &FileContext) -> DatasetSourceResult<Vec<LogicalTable>> {
	let byte_count = u64::try_from(bytes.len()).unwrap_or(u64::MAX).max(1);
	let tensor_limit = u32::try_from(byte_count).unwrap_or(u32::MAX).max(1);
	let limits = SafeTensorLimits::new(byte_count, byte_count, tensor_limit, 64, byte_count).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			format!("construct safetensors parser limits: {error}"),
		)
		.for_path(&context.logical_path)
	})?;
	let archive = parse_safetensors(bytes, limits).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedFormat,
			format!("parse safetensors source: {error}"),
		)
		.for_path(&context.logical_path)
	})?;
	let mut tables = Vec::new();
	if !archive.metadata().is_empty() {
		tables.push(LogicalTable::new(
			"metadata",
			SourceFormat::SafeTensors,
			vec![
				LogicalColumn::classify(b"metadata_key".to_vec()),
				LogicalColumn::infer(b"metadata_value".to_vec()),
			],
			archive
				.metadata()
				.iter()
				.map(|(key, value)| vec![key.as_bytes().to_vec(), value.as_bytes().to_vec()])
				.collect(),
		)?);
	}
	if !archive.entries().is_empty() {
		tables.push(LogicalTable::new(
			"tensors",
			SourceFormat::SafeTensors,
			vec![
				LogicalColumn::classify(b"tensor_name".to_vec()),
				LogicalColumn::classify(b"tensor_type".to_vec()),
				LogicalColumn::classify(b"tensor_shape".to_vec()),
				LogicalColumn::exact(
					b"tensor_rank".to_vec(),
					SemanticType::Numeric,
					VectorEncoding::I32,
				),
				LogicalColumn::exact(
					b"tensor_bytes".to_vec(),
					SemanticType::Numeric,
					VectorEncoding::I32,
				),
				LogicalColumn::exact(
					b"binary".to_vec(),
					SemanticType::Binary,
					VectorEncoding::Bytes,
				),
			],
			archive
				.entries()
				.iter()
				.map(|entry| {
					vec![
						entry.name().as_bytes().to_vec(),
						format!("{:?}", entry.dtype()).into_bytes(),
						entry.shape()
							.iter()
							.map(u64::to_string)
							.collect::<Vec<_>>()
							.join("x")
							.into_bytes(),
						entry.shape().len().to_string().into_bytes(),
						entry.encoded_bytes().to_string().into_bytes(),
						archive
							.encoded_tensor(entry.name())
							.unwrap_or_default()
							.to_vec(),
					]
				})
				.collect(),
		)?);
	}
	if tables.is_empty() {
		tables.push(single_payload(
			SourceFormat::SafeTensors,
			b"binary",
			bytes,
			SemanticType::Binary,
		));
	}
	Ok(tables)
}

#[derive(Clone, Debug)]
struct SourceMetadata {
	source_index: Vec<u8>,
	source_path: Vec<u8>,
	parent_path: Vec<u8>,
	folder: Vec<u8>,
	file_name: Vec<u8>,
	file_stem: Vec<u8>,
	extension: Vec<u8>,
	format: Vec<u8>,
	member: Vec<u8>,
	content_sha256: Vec<u8>,
	file_bytes: Vec<u8>,
	sample_count: Vec<u8>,
	source_depth: Vec<u8>,
}

impl SourceMetadata {
	fn new(context: &FileContext, bytes: &[u8], format: SourceFormat, member: &str, sample_count: usize) -> DatasetSourceResult<Self> {
		let path = Path::new(&context.logical_path);
		let parent_path = path
			.parent()
			.map(|value| value.to_string_lossy().into_owned())
			.unwrap_or_default();
		let folder = path
			.parent()
			.and_then(Path::file_name)
			.map(|value| value.to_string_lossy().into_owned())
			.unwrap_or_default();
		let file_name = path
			.file_name()
			.map(|value| value.to_string_lossy().into_owned())
			.unwrap_or_default();
		let file_stem = path
			.file_stem()
			.map(|value| value.to_string_lossy().into_owned())
			.unwrap_or_default();
		let digest = Sha256::digest(bytes);
		let mut digest_hex = String::with_capacity(64);
		for byte in digest {
			use core::fmt::Write as _;

			write!(digest_hex, "{byte:02x}").map_err(|error| {
				DatasetSourceError::new(
					DatasetSourceErrorKind::MalformedFormat,
					format!("format source digest: {error}"),
				)
			})?;
		}
		Ok(Self {
			source_index: context.source_index.to_string().into_bytes(),
			source_path: context.logical_path.as_bytes().to_vec(),
			parent_path: parent_path.into_bytes(),
			folder: folder.into_bytes(),
			file_name: file_name.into_bytes(),
			file_stem: file_stem.into_bytes(),
			extension: extension(&context.logical_path).into_bytes(),
			format: format.as_str().as_bytes().to_vec(),
			member: member.as_bytes().to_vec(),
			content_sha256: digest_hex.into_bytes(),
			file_bytes: bytes.len().to_string().into_bytes(),
			sample_count: sample_count.to_string().into_bytes(),
			source_depth: context.depth.to_string().into_bytes(),
		})
	}

	fn fields(&self, sample_index: usize) -> [LogicalField; METADATA_WIDTH] {
		[
			LogicalField::categorical(b"source_index", self.source_index.clone()),
			LogicalField::classify(b"source_path", self.source_path.clone()),
			LogicalField::classify(b"parent_path", self.parent_path.clone()),
			LogicalField::classify(b"folder", self.folder.clone()),
			LogicalField::classify(b"file_name", self.file_name.clone()),
			LogicalField::classify(b"file_stem", self.file_stem.clone()),
			LogicalField::classify(b"extension", self.extension.clone()),
			LogicalField::classify(b"format", self.format.clone()),
			LogicalField::classify(b"member", self.member.clone()),
			LogicalField::classify(b"content_sha256", self.content_sha256.clone()),
			LogicalField::numeric(b"file_bytes", self.file_bytes.clone()),
			LogicalField::numeric(b"sample_index", sample_index.to_string().into_bytes()),
			LogicalField::numeric(b"sample_count", self.sample_count.clone()),
			LogicalField::numeric(b"source_depth", self.source_depth.clone()),
		]
	}
}

impl LogicalField {
	fn categorical(name: &'static [u8], value: Vec<u8>) -> Self {
		Self {
			column: LogicalColumn::exact(
				name.to_vec(),
				SemanticType::Categorical,
				VectorEncoding::DictionaryI32,
			),
			value,
		}
	}

	fn classify(name: &'static [u8], value: Vec<u8>) -> Self {
		Self {
			column: LogicalColumn::classify(name.to_vec()),
			value,
		}
	}

	fn numeric(name: &'static [u8], value: Vec<u8>) -> Self {
		Self {
			column: LogicalColumn::exact(name.to_vec(), SemanticType::Numeric, VectorEncoding::I32),
			value,
		}
	}
}

fn is_source_context_header(name: &[u8]) -> bool {
	matches!(
		name,
		b"source_index" | b"source_path" | b"parent_path" | b"folder" | b"file_name" | b"file_stem" | b"extension" | b"format" | b"member" | b"content_sha256" | b"file_bytes" | b"sample_index" | b"sample_count" | b"source_depth"
	)
}

fn check_field_bound(value: &[u8], limits: IngestLimits, path: &str) -> DatasetSourceResult<()> {
	let bytes = u64::try_from(value.len()).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::ArithmeticOverflow,
			format!("field byte count cannot be represented as u64: {error}"),
		)
		.for_path(path)
	})?;
	if bytes > limits.source_bytes().get() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::LimitExceeded,
			format!(
				"one distilled value contains {bytes} bytes, source limit is {}",
				limits.source_bytes()
			),
		)
		.for_path(path));
	}
	if bytes > limits.field_bytes().get() && core::str::from_utf8(value).is_ok() {
		return Err(DatasetSourceError::new(
			DatasetSourceErrorKind::LimitExceeded,
			format!(
				"one textual distilled value contains {bytes} bytes, field limit is {}",
				limits.field_bytes()
			),
		)
		.for_path(path));
	}
	Ok(())
}

fn extension(path: &str) -> String {
	Path::new(path)
		.extension()
		.and_then(|extension| extension.to_str())
		.unwrap_or_default()
		.to_ascii_lowercase()
}

fn preflight_zip_expansion(bytes: &[u8], path: &str, limits: IngestLimits, format: &str) -> DatasetSourceResult<()> {
	let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
		DatasetSourceError::new(
			DatasetSourceErrorKind::MalformedArchive,
			format!("open {format} container: {error}"),
		)
		.for_path(path)
	})?;
	let mut expanded_bytes = 0u64;
	for index in 0..archive.len() {
		let entry = archive.by_index_raw(index).map_err(|error| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::MalformedArchive,
				format!("inspect {format} member {index}: {error}"),
			)
			.for_path(path)
		})?;
		expanded_bytes = expanded_bytes.checked_add(entry.size()).ok_or_else(|| {
			DatasetSourceError::new(
				DatasetSourceErrorKind::ArithmeticOverflow,
				format!("{format} expanded byte count overflowed u64"),
			)
			.for_path(path)
		})?;
		if expanded_bytes > limits.source_bytes().get() {
			return Err(DatasetSourceError::new(
				DatasetSourceErrorKind::LimitExceeded,
				format!(
					"{format} container expands to at least {expanded_bytes} bytes, limit is {}",
					limits.source_bytes()
				),
			)
			.for_path(path));
		}
	}
	Ok(())
}

fn is_zip(bytes: &[u8]) -> bool {
	matches!(
		bytes.get(..4),
		Some(b"PK\x03\x04" | b"PK\x05\x06" | b"PK\x07\x08")
	)
}

fn is_png(bytes: &[u8]) -> bool { bytes.starts_with(b"\x89PNG\r\n\x1a\n") }

fn is_image(bytes: &[u8]) -> bool { has_recognized_image_signature(bytes) }

fn is_probably_text(bytes: &[u8]) -> bool {
	core::str::from_utf8(bytes).is_ok()
		&& bytes
			.iter()
			.all(|byte| !byte.is_ascii_control() || matches!(byte, b'\t' | b'\n' | b'\r'))
}

fn looks_tabular(bytes: &[u8]) -> bool { structural_delimiter(bytes).is_some() }

fn structural_delimiter(bytes: &[u8]) -> Option<Delimiter> {
	let lines = bytes
		.split(|byte| *byte == b'\n')
		.map(|line| line.strip_suffix(b"\r").unwrap_or(line))
		.filter(|line| line.iter().any(|byte| !byte.is_ascii_whitespace()))
		.take(8)
		.collect::<Vec<_>>();
	if lines.len() < 2 {
		return None;
	}
	for (byte, delimiter) in [
		(b'\t', Delimiter::Tab),
		(b';', Delimiter::Semicolon),
		(b',', Delimiter::Comma),
	] {
		let widths = lines
			.iter()
			.map(|line| delimiter_width(line, byte))
			.collect::<Vec<_>>();
		if widths[0] > 1 && widths.iter().all(|width| *width == widths[0]) {
			return Some(delimiter);
		}
	}
	let widths = lines
		.iter()
		.map(|line| {
			line.split(u8::is_ascii_whitespace)
				.filter(|field| !field.is_empty())
				.count()
		})
		.collect::<Vec<_>>();
	if widths[0] > 1
		&& widths.iter().all(|width| *width == widths[0])
		&& lines.iter().all(|line| {
			line.split(u8::is_ascii_whitespace)
				.filter(|field| !field.is_empty())
				.all(|field| core::str::from_utf8(field).is_ok_and(|value| value.parse::<f64>().is_ok()))
		}) {
		Some(Delimiter::AsciiWhitespace)
	} else {
		None
	}
}

fn delimiter_width(line: &[u8], delimiter: u8) -> usize {
	let mut width = 1usize;
	let mut quoted = false;
	let mut index = 0usize;
	while let Some(byte) = line.get(index).copied() {
		if byte == b'"' {
			if quoted && line.get(index + 1) == Some(&b'"') {
				index = index.saturating_add(2);
				continue;
			}
			quoted = !quoted;
		} else if !quoted && byte == delimiter {
			width = width.saturating_add(1);
		}
		index = index.saturating_add(1);
	}
	width
}
