use core::{ fmt, num::{NonZeroU32, NonZeroU64}, }; use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delimiter { Auto, Comma, Semicolon, Tab, AsciiWhitespace, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderMode { Present, Absent, }

/// Caller-fixed bounds for pre-run source framing.
///
/// Bounds are part of preparation input. Parsing cannot grow a supposedly
/// static data image beyond them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IngestLimits { source_bytes: NonZeroU64, records: NonZeroU64, fields_per_record: NonZeroU32,
	field_bytes: NonZeroU64, }

impl IngestLimits {
	/// Create nonzero bounds for one source.
	///
	/// # Errors
	///
	/// Returns [`IngestErrorKind::InvalidLimit`] when any bound is zero.
	pub fn new(source_bytes: u64, records: u64, fields_per_record: u32, field_bytes: u64) -> IngestResult<Self> { Ok(Self {
			source_bytes: nonzero_u64("source byte", source_bytes)?,
			records: nonzero_u64("record", records)?,
			fields_per_record: NonZeroU32::new(fields_per_record)
				.ok_or_else(|| invalid_limit("field-per-record"))?,
			field_bytes: nonzero_u64("field byte", field_bytes)?,
		}) }

	#[must_use]
	pub const fn source_bytes(self) -> NonZeroU64 { self.source_bytes }

	#[must_use]
	pub const fn records(self) -> NonZeroU64 { self.records }

	#[must_use]
	pub const fn fields_per_record(self) -> NonZeroU32 { self.fields_per_record }

	#[must_use]
	pub const fn field_bytes(self) -> NonZeroU64 { self.field_bytes } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableRequest { pub delimiter: Delimiter, pub header: HeaderMode, pub limits: IngestLimits, }

impl TableRequest {
	#[must_use]
	pub const fn new(delimiter: Delimiter, header: HeaderMode, limits: IngestLimits) -> Self { Self { delimiter, header,
			limits, } } }

/// Raw framed bytes. No type detection, encoding, imputation, scaling, or
/// other payload calculation has occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTable { delimiter: Delimiter, headers: Vec<Vec<u8>>, rows: Vec<Vec<Vec<u8>>>, }

impl RawTable { pub(crate) fn from_parts(headers: Vec<Vec<u8>>, rows: Vec<Vec<Vec<u8>>>) -> IngestResult<Self> {
		for (index, row) in rows.iter().enumerate() { if row.len() != headers.len() { return Err(IngestError::new(
					IngestErrorKind::InconsistentWidth, format!(
						"record {} has {} fields, expected {}",
						index + 1, row.len(), headers.len() ), )); } }
		Ok(Self { delimiter: Delimiter::Auto, headers, rows, }) }

	#[must_use]
	pub const fn delimiter(&self) -> Delimiter { self.delimiter }

	#[must_use]
	pub fn headers(&self) -> &[Vec<u8>] { &self.headers }

	#[must_use]
	pub fn rows(&self) -> &[Vec<Vec<u8>>] { &self.rows }

	#[must_use]
	pub fn width(&self) -> usize { self.headers .len() .max(self.rows.first().map_or(0, Vec::len)) } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngestErrorKind { InvalidLimit, Io, SourceLimitExceeded, RecordLimitExceeded, FieldLimitExceeded,
	FieldByteLimitExceeded, MalformedTable, InconsistentWidth, ArithmeticOverflow, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestError { pub kind: IngestErrorKind, pub path: Option<PathBuf>, pub detail: String, }

impl IngestError { fn new(kind: IngestErrorKind, detail: impl Into<String>) -> Self { Self { kind, path: None,
			detail: detail.into(), } }

}

impl fmt::Display for IngestError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		if let Some(path) = &self.path {
			write!(formatter, " [{}]", path.display())?;
		}
		Ok(()) } }

impl std::error::Error for IngestError {}

pub type IngestResult<T> = Result<T, IngestError>;

/// Read one source exactly once into a bounded pre-run framing buffer.
pub fn parse_table(bytes: &[u8], request: TableRequest) -> IngestResult<RawTable> {
	let byte_count = u64::try_from(bytes.len()).map_err(|error| { IngestError::new( IngestErrorKind::ArithmeticOverflow,
			format!("source length cannot be represented as u64: {error}"),
		) })?; if byte_count > request.limits.source_bytes.get() { return Err(IngestError::new(
			IngestErrorKind::SourceLimitExceeded, format!(
				"source contains {byte_count} bytes, limit is {}",
				request.limits.source_bytes ), )); }
	let delimiter = match request.delimiter { Delimiter::Auto => sniff_delimiter(bytes), explicit => explicit, };
	let records = match delimiter { Delimiter::AsciiWhitespace => parse_whitespace_records(bytes, request.limits)?,
		_ => parse_csv_records(bytes, delimiter, request.limits)?, }; assemble_table(records, delimiter, request.header) }

fn parse_csv_records(bytes: &[u8], delimiter: Delimiter, limits: IngestLimits) -> IngestResult<Vec<Vec<Vec<u8>>>> {
	let separator = match delimiter {
		Delimiter::Comma => b',',
		Delimiter::Semicolon => b';',
		Delimiter::Tab => b'\t',
		Delimiter::Auto | Delimiter::AsciiWhitespace => { return Err(IngestError::new( IngestErrorKind::MalformedTable,
				"CSV parser received a non-CSV delimiter",
			)); } }; let mut reader = csv::ReaderBuilder::new() .has_headers(false) .flexible(false) .delimiter(separator)
		.from_reader(bytes); let mut records = Vec::new(); for result in reader.byte_records() {
		let record = result.map_err(|error| { IngestError::new( IngestErrorKind::MalformedTable,
				format!("CSV framing failed: {error}"),
			) })?; check_record_bound(records.len(), record.len(), limits)?; let mut fields = Vec::with_capacity(record.len());
		for field in &record { check_field_bound(field, limits)?; fields.push(field.to_vec()); }
		records.push(fields); }
	Ok(records) }

fn parse_whitespace_records(bytes: &[u8], limits: IngestLimits) -> IngestResult<Vec<Vec<Vec<u8>>>> {
	let mut records = Vec::new();
	for line in bytes.split(|byte| *byte == b'\n') {
		let line = line.strip_suffix(b"\r").unwrap_or(line);
		let fields = line .split(|byte| byte.is_ascii_whitespace()) .filter(|field| !field.is_empty()) .collect::<Vec<_>>();
		if fields.is_empty() { continue; }
		check_record_bound(records.len(), fields.len(), limits)?; let mut owned = Vec::with_capacity(fields.len());
		for field in fields { check_field_bound(field, limits)?; owned.push(field.to_vec()); }
		records.push(owned); }
	validate_widths(&records)?; Ok(records) }

fn assemble_table(mut records: Vec<Vec<Vec<u8>>>, delimiter: Delimiter, header: HeaderMode) -> IngestResult<RawTable> {
	if records.is_empty() { return Ok(RawTable { delimiter, headers: Vec::new(), rows: Vec::new(), }); }
	validate_widths(&records)?; let width = records.first().map_or(0, Vec::len); let headers = match header {
		HeaderMode::Present => records.remove(0), HeaderMode::Absent => { (1..=width)
				.map(|index| format!("col{index}").into_bytes())
				.collect() } }; Ok(RawTable { delimiter, headers, rows: records, }) }

fn check_record_bound(records_seen: usize, field_count: usize, limits: IngestLimits) -> IngestResult<()> {
	let record_number = u64::try_from(records_seen) .ok() .and_then(|count| count.checked_add(1)) .ok_or_else(|| {
			IngestError::new( IngestErrorKind::ArithmeticOverflow,
				"record count exceeds u64",
			) })?; if record_number > limits.records.get() { return Err(IngestError::new( IngestErrorKind::RecordLimitExceeded,
			format!("record {record_number} exceeds limit {}", limits.records),
		)); }
	let fields = u32::try_from(field_count).map_err(|error| { IngestError::new( IngestErrorKind::FieldLimitExceeded,
			format!("field count exceeds u32: {error}"),
		) })?; if fields > limits.fields_per_record.get() { return Err(IngestError::new( IngestErrorKind::FieldLimitExceeded,
			format!(
				"record {record_number} contains {fields} fields, limit is {}",
				limits.fields_per_record ), )); }
	Ok(()) }

fn check_field_bound(field: &[u8], limits: IngestLimits) -> IngestResult<()> {
	let bytes = u64::try_from(field.len()).map_err(|error| { IngestError::new( IngestErrorKind::ArithmeticOverflow,
			format!("field length cannot be represented as u64: {error}"),
		) })?; if bytes > limits.field_bytes.get() { Err(IngestError::new( IngestErrorKind::FieldByteLimitExceeded, format!(
				"field contains {bytes} bytes, limit is {}",
				limits.field_bytes ), )) } else { Ok(()) } }

fn validate_widths(records: &[Vec<Vec<u8>>]) -> IngestResult<()> {
	let Some(width) = records.first().map(Vec::len) else { return Ok(()); };
	for (index, record) in records.iter().enumerate().skip(1) { if record.len() != width { return Err(IngestError::new(
				IngestErrorKind::InconsistentWidth, format!(
					"record {} has {} fields, expected {width}",
					index + 1, record.len() ), )); } }
	Ok(()) }

fn sniff_delimiter(bytes: &[u8]) -> Delimiter { let line = bytes
		.split(|byte| *byte == b'\n')
		.map(|line| line.strip_suffix(b"\r").unwrap_or(line))
		.find(|line| line.iter().any(|byte| !byte.is_ascii_whitespace())) .unwrap_or_default();
	let mut counts = [(Delimiter::Comma, 0_u64); 3]; counts[1].0 = Delimiter::Semicolon; counts[2].0 = Delimiter::Tab;
	let mut quoted = false; let mut unquoted_space = false; let mut index = 0_usize;
	while let Some(byte) = line.get(index).copied() {
		if byte == b'"' {
			if quoted && line.get(index + 1) == Some(&b'"') {
				index += 2; continue; }
			quoted = !quoted; } else if !quoted { match byte {
				b',' => counts[0].1 += 1,
				b';' => counts[1].1 += 1,
				b'\t' => counts[2].1 += 1,
				b' ' => unquoted_space = true,
				_ => {} } }
		index += 1; }
	counts.sort_by(|left, right| { right.1 .cmp(&left.1)
			.then_with(|| delimiter_rank(left.0).cmp(&delimiter_rank(right.0))) }); if unquoted_space
		&& counts .iter() .filter(|(delimiter, _)| matches!(delimiter, Delimiter::Comma | Delimiter::Semicolon))
			.all(|(_, count)| *count == 0)
		&& line .split(|byte| byte.is_ascii_whitespace()) .filter(|field| !field.is_empty()) .take(2) .count() == 2
	{ Delimiter::AsciiWhitespace } else if counts[0].1 != 0 { counts[0].0 } else if line
		.split(|byte| byte.is_ascii_whitespace()) .filter(|field| !field.is_empty()) .take(2) .count() == 2
	{ Delimiter::AsciiWhitespace } else { Delimiter::Comma } }

const fn delimiter_rank(delimiter: Delimiter) -> u8 { match delimiter { Delimiter::Comma => 0,
		Delimiter::Semicolon => 1, Delimiter::Tab => 2, Delimiter::Auto => 3, Delimiter::AsciiWhitespace => 4, } }

fn nonzero_u64(name: &str, value: u64) -> IngestResult<NonZeroU64> { NonZeroU64::new(value).ok_or_else(|| invalid_limit(name)) }

fn invalid_limit(name: &str) -> IngestError { IngestError::new( IngestErrorKind::InvalidLimit,
		format!("{name} limit must be nonzero"),
	) }
