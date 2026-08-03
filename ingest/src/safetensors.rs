use core::{ fmt, num::{NonZeroU32, NonZeroU64}, }; use std::collections::{BTreeMap, BTreeSet};

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};

/// Calculation-independent safetensors element encoding.
///
/// The format layer retains encoded bytes. Converting any of these encodings
/// into Recipe's f32/int32 calculation payload is a separately scheduled GPU
/// operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SafeTensorDType { Bool, U8, I8, U16, I16, U32, I32, U64, I64, F16, Bf16, F32, F64, }

impl SafeTensorDType { fn parse(value: &str) -> SafeTensorResult<Self> { match value {
			"BOOL" => Ok(Self::Bool),
			"U8" => Ok(Self::U8),
			"I8" => Ok(Self::I8),
			"U16" => Ok(Self::U16),
			"I16" => Ok(Self::I16),
			"U32" => Ok(Self::U32),
			"I32" => Ok(Self::I32),
			"U64" => Ok(Self::U64),
			"I64" => Ok(Self::I64),
			"F16" => Ok(Self::F16),
			"BF16" => Ok(Self::Bf16),
			"F32" => Ok(Self::F32),
			"F64" => Ok(Self::F64),
			other => { Err(SafeTensorError::new( SafeTensorErrorKind::UnsupportedDType,
					format!("unsupported safetensors dtype {other:?}"),
				)) } } }

	#[must_use]
	pub const fn element_bytes(self) -> u64 { match self { Self::Bool | Self::U8 | Self::I8 => 1,
			Self::U16 | Self::I16 | Self::F16 | Self::Bf16 => 2, Self::U32 | Self::I32 | Self::F32 => 4,
			Self::U64 | Self::I64 | Self::F64 => 8, } } }

/// Caller-fixed parser bounds for one safetensors image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SafeTensorLimits { header_bytes: NonZeroU64, data_bytes: NonZeroU64, tensors: NonZeroU32, rank: NonZeroU32,
	name_bytes: NonZeroU64, }

impl SafeTensorLimits {
	/// Construct nonzero bounds for an ahead-of-run parse.
	///
	/// # Errors
	///
	/// Returns [`SafeTensorErrorKind::InvalidLimit`] when any bound is zero.
	pub fn new( header_bytes: u64, data_bytes: u64, tensors: u32, rank: u32, name_bytes: u64, ) -> SafeTensorResult<Self> {
		Ok(Self {
			header_bytes: nonzero_u64("header byte", header_bytes)?,
			data_bytes: nonzero_u64("data byte", data_bytes)?,
			tensors: NonZeroU32::new(tensors).ok_or_else(|| invalid_limit("tensor"))?,
			rank: NonZeroU32::new(rank).ok_or_else(|| invalid_limit("rank"))?,
			name_bytes: nonzero_u64("tensor-name byte", name_bytes)?,
		}) }

	#[must_use]
	pub const fn header_bytes(self) -> NonZeroU64 { self.header_bytes }

	#[must_use]
	pub const fn data_bytes(self) -> NonZeroU64 { self.data_bytes }

	#[must_use]
	pub const fn tensors(self) -> NonZeroU32 { self.tensors }

	#[must_use]
	pub const fn rank(self) -> NonZeroU32 { self.rank }

	#[must_use]
	pub const fn name_bytes(self) -> NonZeroU64 { self.name_bytes } }

/// One validated tensor entry. Offsets are relative to the archive data
/// section and describe immutable encoded bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeTensorEntry { name: String, dtype: SafeTensorDType, shape: Vec<u64>, begin: u64, end: u64, }

impl SafeTensorEntry {
	#[must_use]
	pub fn name(&self) -> &str { &self.name }

	#[must_use]
	pub const fn dtype(&self) -> SafeTensorDType { self.dtype }

	#[must_use]
	pub fn shape(&self) -> &[u64] { &self.shape }

	#[must_use]
	pub const fn begin(&self) -> u64 { self.begin }

	#[must_use]
	pub const fn end(&self) -> u64 { self.end }

	#[must_use]
	pub const fn encoded_bytes(&self) -> u64 { self.end - self.begin } }

/// Fully validated, borrowed safetensors image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeTensorArchive<'a> {
	data_start: u64,
	data: &'a [u8],
	metadata: BTreeMap<String, String>, entries: Vec<SafeTensorEntry>, }

impl<'a> SafeTensorArchive<'a> {
	#[must_use]
	pub const fn data_start(&self) -> u64 { self.data_start }

	#[must_use]
	pub fn data(&self) -> &'a [u8] { self.data }

	#[must_use]
	pub const fn metadata(&self) -> &BTreeMap<String, String> { &self.metadata }

	#[must_use]
	pub fn entries(&self) -> &[SafeTensorEntry] { &self.entries }

	#[must_use]
	pub fn entry(&self, name: &str) -> Option<&SafeTensorEntry> { self.entries
			.binary_search_by(|entry| entry.name.as_str().cmp(name)) .ok() .map(|index| &self.entries[index]) }

	#[must_use]
	pub fn encoded_tensor(&self, name: &str) -> Option<&'a [u8]> {
		let entry = self.entry(name)?; let begin = usize::try_from(entry.begin).ok()?;
		let end = usize::try_from(entry.end).ok()?; self.data.get(begin..end) } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SafeTensorErrorKind { InvalidLimit, Truncated, HeaderLimitExceeded, DataLimitExceeded, TensorLimitExceeded,
	RankLimitExceeded, NameLimitExceeded, MalformedHeader, DuplicateField, UnsupportedDType, InvalidShape, InvalidOffset,
	NonContiguousData, ArithmeticOverflow, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeTensorError { pub kind: SafeTensorErrorKind, pub detail: String, }

impl SafeTensorError { fn new(kind: SafeTensorErrorKind, detail: impl Into<String>) -> Self { Self { kind,
			detail: detail.into(), } } }

impl fmt::Display for SafeTensorError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	} }

impl std::error::Error for SafeTensorError {}

pub type SafeTensorResult<T> = Result<T, SafeTensorError>;

/// Parse and validate one complete safetensors image without decoding tensor
/// payload values.
///
/// The returned archive borrows the caller's bytes. Header parsing is bounded,
/// tensor names and fields must be unique, encoded spans must exactly match
/// shape and dtype, and the data section must be covered contiguously without
/// overlap or trailing unowned bytes.
///
/// # Errors
///
/// Returns a fail-closed structural or configured-bound error without
/// exposing a partial archive.
pub fn parse_safetensors(bytes: &[u8], limits: SafeTensorLimits) -> SafeTensorResult<SafeTensorArchive<'_>> {
	let length_prefix = bytes.get(..8).ok_or_else(|| { SafeTensorError::new( SafeTensorErrorKind::Truncated, format!(
				"safetensors image has {} bytes, fewer than its 8-byte header length",
				bytes.len() ), ) })?; let header_bytes = u64::from_le_bytes( length_prefix .try_into()
			.expect("an exact eight-byte slice converts to an array"),
	); if header_bytes > limits.header_bytes.get() { return Err(SafeTensorError::new(
			SafeTensorErrorKind::HeaderLimitExceeded, format!(
				"safetensors header has {header_bytes} bytes, limit is {}",
				limits.header_bytes ), )); }
	let data_start = 8_u64.checked_add(header_bytes).ok_or_else(|| { SafeTensorError::new(
			SafeTensorErrorKind::ArithmeticOverflow,
			"safetensors data offset overflowed u64",
		) })?; let data_start_usize = usize::try_from(data_start).map_err(|error| { SafeTensorError::new(
			SafeTensorErrorKind::ArithmeticOverflow,
			format!("safetensors data offset cannot address host memory: {error}"),
		) })?; let header = bytes.get(8..data_start_usize).ok_or_else(|| { SafeTensorError::new(
			SafeTensorErrorKind::Truncated, format!(
				"safetensors header ends at byte {data_start}, image has {} bytes",
				bytes.len() ), ) })?; let data = bytes .get(data_start_usize..)
		.expect("validated data start is within the complete image");
	let data_bytes = u64::try_from(data.len()).map_err(|error| { SafeTensorError::new(
			SafeTensorErrorKind::ArithmeticOverflow,
			format!("safetensors data length cannot be represented as u64: {error}"),
		) })?; if data_bytes > limits.data_bytes.get() { return Err(SafeTensorError::new(
			SafeTensorErrorKind::DataLimitExceeded, format!(
				"safetensors data section has {data_bytes} bytes, limit is {}",
				limits.data_bytes ), )); }

	let raw = parse_header(header)?; let tensor_count = u32::try_from(raw.tensors.len()).map_err(|error| {
		SafeTensorError::new( SafeTensorErrorKind::ArithmeticOverflow,
			format!("tensor count cannot be represented as u32: {error}"),
		) })?; if tensor_count > limits.tensors.get() { return Err(SafeTensorError::new(
			SafeTensorErrorKind::TensorLimitExceeded, format!(
				"safetensors header declares {tensor_count} tensors, limit is {}",
				limits.tensors ), )); }

	let mut entries = Vec::with_capacity(raw.tensors.len()); for (name, tensor) in raw.tensors {
		validate_name(&name, limits)?; let rank = u32::try_from(tensor.shape.len()).map_err(|error| { SafeTensorError::new(
				SafeTensorErrorKind::ArithmeticOverflow,
				format!("tensor {name:?} rank cannot be represented as u32: {error}"),
			) })?; if rank > limits.rank.get() { return Err(SafeTensorError::new( SafeTensorErrorKind::RankLimitExceeded,
				format!("tensor {name:?} has rank {rank}, limit is {}", limits.rank),
			)); }
		let dtype = SafeTensorDType::parse(&tensor.dtype)?;
		let elements = tensor.shape.iter().try_fold(1_u64, |product, dimension| {
			product.checked_mul(*dimension).ok_or_else(|| { SafeTensorError::new( SafeTensorErrorKind::ArithmeticOverflow,
					format!("tensor {name:?} shape product overflowed u64"),
				) }) })?; let expected_bytes = elements.checked_mul(dtype.element_bytes()).ok_or_else(|| { SafeTensorError::new(
				SafeTensorErrorKind::ArithmeticOverflow,
				format!("tensor {name:?} encoded byte count overflowed u64"),
			) })?; let [begin, end] = tensor.data_offsets; let encoded_bytes = end.checked_sub(begin).ok_or_else(|| {
			SafeTensorError::new( SafeTensorErrorKind::InvalidOffset,
				format!("tensor {name:?} has reversed offsets [{begin}, {end}]"),
			) })?; if end > data_bytes { return Err(SafeTensorError::new( SafeTensorErrorKind::InvalidOffset,
				format!("tensor {name:?} ends at {end}, beyond the {data_bytes}-byte data section"),
			)); }
		if encoded_bytes != expected_bytes { return Err(SafeTensorError::new( SafeTensorErrorKind::InvalidShape, format!(
					"tensor {name:?} span is {encoded_bytes} bytes, shape and {:?} require {expected_bytes}",
					dtype ), )); }
		entries.push(SafeTensorEntry { name, dtype, shape: tensor.shape, begin, end, }); }

	validate_contiguous_data(&entries, data_bytes)?; entries.sort_by(|left, right| left.name.cmp(&right.name));
	Ok(SafeTensorArchive { data_start, data, metadata: raw.metadata, entries, }) }

fn validate_name(name: &str, limits: SafeTensorLimits) -> SafeTensorResult<()> { if name.is_empty() {
		return Err(SafeTensorError::new( SafeTensorErrorKind::MalformedHeader,
			"safetensors tensor name is empty",
		)); }
	let bytes = u64::try_from(name.len()).map_err(|error| { SafeTensorError::new( SafeTensorErrorKind::ArithmeticOverflow,
			format!("tensor name length cannot be represented as u64: {error}"),
		) })?; if bytes > limits.name_bytes.get() { Err(SafeTensorError::new( SafeTensorErrorKind::NameLimitExceeded, format!(
				"tensor name {name:?} has {bytes} bytes, limit is {}",
				limits.name_bytes ), )) } else { Ok(()) } }

fn validate_contiguous_data(entries: &[SafeTensorEntry], data_bytes: u64) -> SafeTensorResult<()> {
	let mut spans = entries .iter() .map(|entry| (entry.begin, entry.end, entry.name.as_str())) .collect::<Vec<_>>();
	spans.sort_unstable(); let mut cursor = 0_u64; for (begin, end, name) in spans { if begin != cursor {
			return Err(SafeTensorError::new( SafeTensorErrorKind::NonContiguousData,
				format!("tensor {name:?} begins at {begin}, expected contiguous offset {cursor}"),
			)); }
		cursor = end; }
	if cursor != data_bytes { return Err(SafeTensorError::new( SafeTensorErrorKind::NonContiguousData,
			format!("validated tensors cover {cursor} bytes, data section has {data_bytes}"),
		)); }
	Ok(()) }

#[derive(Debug)]
struct RawHeader { metadata: BTreeMap<String, String>, tensors: BTreeMap<String, RawTensor>, }

impl<'de> Deserialize<'de> for RawHeader {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: Deserializer<'de> {
		deserializer.deserialize_map(HeaderVisitor) } }

#[derive(Debug)]
struct HeaderVisitor;

impl<'de> Visitor<'de> for HeaderVisitor {
	type Value = RawHeader;

	fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str("a safetensors JSON object") }

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where A: MapAccess<'de> {
		let mut names = BTreeSet::new(); let mut metadata = None; let mut tensors = BTreeMap::new();
		while let Some(name) = map.next_key::<String>()? { if !names.insert(name.clone()) {
				return Err(de::Error::custom(format!(
					"duplicate top-level field {name:?}"
				))); }
			if name == "__metadata__" {
				metadata = Some(map.next_value::<Metadata>()?.0); } else { let tensor = map.next_value::<RawTensor>()?;
				tensors.insert(name, tensor); } }
		Ok(RawHeader { metadata: metadata.unwrap_or_default(), tensors, }) } }

#[derive(Debug)]
struct Metadata(BTreeMap<String, String>);

impl<'de> Deserialize<'de> for Metadata {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: Deserializer<'de> {
		deserializer.deserialize_map(MetadataVisitor) } }

#[derive(Debug)]
struct MetadataVisitor;

impl<'de> Visitor<'de> for MetadataVisitor {
	type Value = Metadata;

	fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str("a string-to-string safetensors metadata object") }

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where A: MapAccess<'de> {
		let mut metadata = BTreeMap::new(); while let Some((key, value)) = map.next_entry::<String, String>()? {
			if metadata.insert(key.clone(), value).is_some() { return Err(de::Error::custom(format!(
					"duplicate metadata field {key:?}"
				))); } }
		Ok(Metadata(metadata)) } }

#[derive(Debug)]
struct RawTensor { dtype: String, shape: Vec<u64>, data_offsets: [u64; 2], }

impl<'de> Deserialize<'de> for RawTensor {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: Deserializer<'de> {
		deserializer.deserialize_map(TensorVisitor) } }

#[derive(Debug)]
struct TensorVisitor;

impl<'de> Visitor<'de> for TensorVisitor {
	type Value = RawTensor;

	fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str("a safetensors tensor object") }

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where A: MapAccess<'de> {
		let mut dtype = None; let mut shape = None; let mut data_offsets = None;
		while let Some(field) = map.next_key::<String>()? { match field.as_str() {
				"dtype" => set_once(&mut dtype, map.next_value()?, "dtype")?,
				"shape" => set_once(&mut shape, map.next_value()?, "shape")?,
				"data_offsets" => {
					set_once(&mut data_offsets, map.next_value()?, "data_offsets")?;
				}
				other => { return Err(de::Error::unknown_field(other, &[
						"dtype",
						"shape",
						"data_offsets",
					])); } } }
		Ok(RawTensor {
			dtype: dtype.ok_or_else(|| de::Error::missing_field("dtype"))?,
			shape: shape.ok_or_else(|| de::Error::missing_field("shape"))?,
			data_offsets: data_offsets.ok_or_else(|| de::Error::missing_field("data_offsets"))?,
		}) } }

fn set_once<E, T>(slot: &mut Option<T>, value: T, field: &'static str) -> Result<(), E>
where E: de::Error { if slot.replace(value).is_some() {
		Err(E::custom(format!("duplicate tensor field {field:?}")))
	} else { Ok(()) } }

fn parse_header(bytes: &[u8]) -> SafeTensorResult<RawHeader> {
	let mut deserializer = serde_json::Deserializer::from_slice(bytes);
	let header = RawHeader::deserialize(&mut deserializer).map_err(|error| {
		let kind = if error.to_string().contains("duplicate") {
			SafeTensorErrorKind::DuplicateField } else { SafeTensorErrorKind::MalformedHeader };
		SafeTensorError::new(kind, format!("invalid safetensors header: {error}"))
	})?; deserializer.end().map_err(|error| { SafeTensorError::new( SafeTensorErrorKind::MalformedHeader,
			format!("trailing safetensors header content: {error}"),
		) })?; Ok(header) }

fn nonzero_u64(name: &str, value: u64) -> SafeTensorResult<NonZeroU64> { NonZeroU64::new(value).ok_or_else(|| invalid_limit(name)) }

fn invalid_limit(name: &str) -> SafeTensorError { SafeTensorError::new( SafeTensorErrorKind::InvalidLimit,
		format!("{name} limit must be nonzero"),
	) }
