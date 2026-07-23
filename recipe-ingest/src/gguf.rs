use core::fmt;
use core::num::{NonZeroU32, NonZeroU64};
use core::str;
use std::collections::BTreeSet;

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const DEFAULT_ALIGNMENT: u32 = 32;
const METADATA_KEY_BYTES_MAX: u64 = 65_535;
const TENSOR_NAME_BYTES_MAX: u64 = 64;
const ARRAY_DEPTH_HARD_MAX: u32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GgufEndian {
	Little,
	Big,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GgufLimits {
	file_bytes: NonZeroU64,
	metadata_pairs: NonZeroU64,
	tensors: NonZeroU64,
	rank: NonZeroU32,
	string_bytes: NonZeroU64,
	array_elements: NonZeroU64,
	array_depth: NonZeroU32,
}

impl GgufLimits {
	/// Construct nonzero aggregate parser bounds.
	///
	/// # Errors
	///
	/// Returns [`GgufErrorKind::InvalidLimit`] for zero limits or an array
	/// depth above the fixed recursion ceiling.
	pub(crate) fn new(
		file_bytes: u64,
		metadata_pairs: u64,
		tensors: u64,
		rank: u32,
		string_bytes: u64,
		array_elements: u64,
		array_depth: u32,
	) -> GgufResult<Self> {
		let depth = nonzero_u32("array depth", array_depth)?;
		match depth.get() <= ARRAY_DEPTH_HARD_MAX {
			true => Ok(Self {
				file_bytes: nonzero_u64("file byte", file_bytes)?,
				metadata_pairs: nonzero_u64("metadata pair", metadata_pairs)?,
				tensors: nonzero_u64("tensor", tensors)?,
				rank: nonzero_u32("rank", rank)?,
				string_bytes: nonzero_u64("aggregate string byte", string_bytes)?,
				array_elements: nonzero_u64("aggregate array element", array_elements)?,
				array_depth: depth,
			}),
			false => Err(GgufError::new(
				GgufErrorKind::InvalidLimit,
				format!("array depth limit exceeds hard ceiling {ARRAY_DEPTH_HARD_MAX}"),
			)),
		}
	}

	#[must_use]
	pub(crate) const fn file_bytes(self) -> NonZeroU64 {
		self.file_bytes
	}

	#[must_use]
	pub(crate) const fn metadata_pairs(self) -> NonZeroU64 {
		self.metadata_pairs
	}

	#[must_use]
	pub(crate) const fn tensors(self) -> NonZeroU64 {
		self.tensors
	}

	#[must_use]
	pub(crate) const fn rank(self) -> NonZeroU32 {
		self.rank
	}

	#[must_use]
	pub(crate) const fn string_bytes(self) -> NonZeroU64 {
		self.string_bytes
	}

	#[must_use]
	pub(crate) const fn array_elements(self) -> NonZeroU64 {
		self.array_elements
	}

	#[must_use]
	pub(crate) const fn array_depth(self) -> NonZeroU32 {
		self.array_depth
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum GgufMetadataType {
	U8,
	I8,
	U16,
	I16,
	U32,
	I32,
	F32,
	Bool,
	String,
	Array,
	U64,
	I64,
	F64,
}

impl GgufMetadataType {
	fn parse(code: u32) -> GgufResult<Self> {
		match code {
			0 => Ok(Self::U8),
			1 => Ok(Self::I8),
			2 => Ok(Self::U16),
			3 => Ok(Self::I16),
			4 => Ok(Self::U32),
			5 => Ok(Self::I32),
			6 => Ok(Self::F32),
			7 => Ok(Self::Bool),
			8 => Ok(Self::String),
			9 => Ok(Self::Array),
			10 => Ok(Self::U64),
			11 => Ok(Self::I64),
			12 => Ok(Self::F64),
			other => Err(GgufError::new(
				GgufErrorKind::UnsupportedMetadataType,
				format!("GGUF metadata type code {other} is unsupported"),
			)),
		}
	}

	#[must_use]
	pub(crate) const fn code(self) -> u32 {
		match self {
			Self::U8 => 0,
			Self::I8 => 1,
			Self::U16 => 2,
			Self::I16 => 3,
			Self::U32 => 4,
			Self::I32 => 5,
			Self::F32 => 6,
			Self::Bool => 7,
			Self::String => 8,
			Self::Array => 9,
			Self::U64 => 10,
			Self::I64 => 11,
			Self::F64 => 12,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GgufMetadataArray<'a> {
	element_type: GgufMetadataType,
	values: Vec<GgufMetadataValue<'a>>,
}

impl<'a> GgufMetadataArray<'a> {
	#[must_use]
	pub(crate) const fn element_type(&self) -> GgufMetadataType {
		self.element_type
	}

	#[must_use]
	pub(crate) fn values(&self) -> &[GgufMetadataValue<'a>] {
		&self.values
	}
}

/// Metadata payload that retains the encoded scalar type and floating bits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GgufMetadataValue<'a> {
	U8(u8),
	I8(i8),
	U16(u16),
	I16(i16),
	U32(u32),
	I32(i32),
	F32Bits(u32),
	Bool(bool),
	String(&'a str),
	Array(GgufMetadataArray<'a>),
	U64(u64),
	I64(i64),
	F64Bits(u64),
}

impl GgufMetadataValue<'_> {
	#[must_use]
	pub(crate) const fn value_type(&self) -> GgufMetadataType {
		match self {
			Self::U8(..) => GgufMetadataType::U8,
			Self::I8(..) => GgufMetadataType::I8,
			Self::U16(..) => GgufMetadataType::U16,
			Self::I16(..) => GgufMetadataType::I16,
			Self::U32(..) => GgufMetadataType::U32,
			Self::I32(..) => GgufMetadataType::I32,
			Self::F32Bits(..) => GgufMetadataType::F32,
			Self::Bool(..) => GgufMetadataType::Bool,
			Self::String(..) => GgufMetadataType::String,
			Self::Array(..) => GgufMetadataType::Array,
			Self::U64(..) => GgufMetadataType::U64,
			Self::I64(..) => GgufMetadataType::I64,
			Self::F64Bits(..) => GgufMetadataType::F64,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GgufMetadataEntry<'a> {
	key: &'a str,
	value: GgufMetadataValue<'a>,
}

impl<'a> GgufMetadataEntry<'a> {
	#[must_use]
	pub(crate) const fn key(&self) -> &'a str {
		self.key
	}

	#[must_use]
	pub(crate) const fn value(&self) -> &GgufMetadataValue<'a> {
		&self.value
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum GgufTensorType {
	F32,
	F16,
	Q4_0,
	Q4_1,
	Q5_0,
	Q5_1,
	Q8_0,
	Q8_1,
	Q2K,
	Q3K,
	Q4K,
	Q5K,
	Q6K,
	Q8K,
	Iq2Xxs,
	Iq2Xs,
	Iq3Xxs,
	Iq1S,
	Iq4Nl,
	Iq3S,
	Iq2S,
	Iq4Xs,
	I8,
	I16,
	I32,
	I64,
	F64,
	Iq1M,
	Bf16,
	Tq1_0,
	Tq2_0,
	Mxfp4,
}

impl GgufTensorType {
	fn parse(code: u32) -> GgufResult<Self> {
		match code {
			0 => Ok(Self::F32),
			1 => Ok(Self::F16),
			2 => Ok(Self::Q4_0),
			3 => Ok(Self::Q4_1),
			6 => Ok(Self::Q5_0),
			7 => Ok(Self::Q5_1),
			8 => Ok(Self::Q8_0),
			9 => Ok(Self::Q8_1),
			10 => Ok(Self::Q2K),
			11 => Ok(Self::Q3K),
			12 => Ok(Self::Q4K),
			13 => Ok(Self::Q5K),
			14 => Ok(Self::Q6K),
			15 => Ok(Self::Q8K),
			16 => Ok(Self::Iq2Xxs),
			17 => Ok(Self::Iq2Xs),
			18 => Ok(Self::Iq3Xxs),
			19 => Ok(Self::Iq1S),
			20 => Ok(Self::Iq4Nl),
			21 => Ok(Self::Iq3S),
			22 => Ok(Self::Iq2S),
			23 => Ok(Self::Iq4Xs),
			24 => Ok(Self::I8),
			25 => Ok(Self::I16),
			26 => Ok(Self::I32),
			27 => Ok(Self::I64),
			28 => Ok(Self::F64),
			29 => Ok(Self::Iq1M),
			30 => Ok(Self::Bf16),
			34 => Ok(Self::Tq1_0),
			35 => Ok(Self::Tq2_0),
			39 => Ok(Self::Mxfp4),
			other => Err(GgufError::new(
				GgufErrorKind::UnsupportedTensorType,
				format!("GGML tensor type code {other} is unsupported or removed"),
			)),
		}
	}

	#[must_use]
	pub(crate) const fn code(self) -> u32 {
		match self {
			Self::F32 => 0,
			Self::F16 => 1,
			Self::Q4_0 => 2,
			Self::Q4_1 => 3,
			Self::Q5_0 => 6,
			Self::Q5_1 => 7,
			Self::Q8_0 => 8,
			Self::Q8_1 => 9,
			Self::Q2K => 10,
			Self::Q3K => 11,
			Self::Q4K => 12,
			Self::Q5K => 13,
			Self::Q6K => 14,
			Self::Q8K => 15,
			Self::Iq2Xxs => 16,
			Self::Iq2Xs => 17,
			Self::Iq3Xxs => 18,
			Self::Iq1S => 19,
			Self::Iq4Nl => 20,
			Self::Iq3S => 21,
			Self::Iq2S => 22,
			Self::Iq4Xs => 23,
			Self::I8 => 24,
			Self::I16 => 25,
			Self::I32 => 26,
			Self::I64 => 27,
			Self::F64 => 28,
			Self::Iq1M => 29,
			Self::Bf16 => 30,
			Self::Tq1_0 => 34,
			Self::Tq2_0 => 35,
			Self::Mxfp4 => 39,
		}
	}

	#[must_use]
	pub(crate) const fn block_elements(self) -> u64 {
		match self {
			Self::F32 | Self::F16 | Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::F64 | Self::Bf16 => 1,
			Self::Q4_0
			| Self::Q4_1
			| Self::Q5_0
			| Self::Q5_1
			| Self::Q8_0
			| Self::Q8_1
			| Self::Iq4Nl
			| Self::Mxfp4 => 32,
			Self::Q2K
			| Self::Q3K
			| Self::Q4K
			| Self::Q5K
			| Self::Q6K
			| Self::Q8K
			| Self::Iq2Xxs
			| Self::Iq2Xs
			| Self::Iq3Xxs
			| Self::Iq1S
			| Self::Iq3S
			| Self::Iq2S
			| Self::Iq4Xs
			| Self::Iq1M
			| Self::Tq1_0
			| Self::Tq2_0 => 256,
		}
	}

	#[must_use]
	pub(crate) const fn block_bytes(self) -> u64 {
		match self {
			Self::I8 => 1,
			Self::F16 | Self::I16 | Self::Bf16 => 2,
			Self::F32 | Self::I32 => 4,
			Self::I64 | Self::F64 => 8,
			Self::Q4_0 | Self::Iq4Nl => 18,
			Self::Q4_1 => 20,
			Self::Q5_0 => 22,
			Self::Q5_1 => 24,
			Self::Q8_0 => 34,
			Self::Q8_1 => 36,
			Self::Q2K => 84,
			Self::Q3K | Self::Iq3S => 110,
			Self::Q4K => 144,
			Self::Q5K => 176,
			Self::Q6K => 210,
			Self::Q8K => 292,
			Self::Iq2Xxs | Self::Tq2_0 => 66,
			Self::Iq2Xs => 74,
			Self::Iq3Xxs => 98,
			Self::Iq1S => 50,
			Self::Iq2S => 82,
			Self::Iq4Xs => 136,
			Self::Iq1M => 56,
			Self::Tq1_0 => 54,
			Self::Mxfp4 => 17,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GgufTensor<'a> {
	name: &'a str,
	dimensions: Vec<u64>,
	tensor_type: GgufTensorType,
	data_offset: u64,
	file_offset: u64,
	encoded_bytes: u64,
}

impl<'a> GgufTensor<'a> {
	#[must_use]
	pub(crate) const fn name(&self) -> &'a str {
		self.name
	}

	#[must_use]
	pub(crate) fn dimensions(&self) -> &[u64] {
		&self.dimensions
	}

	#[must_use]
	pub(crate) const fn tensor_type(&self) -> GgufTensorType {
		self.tensor_type
	}

	#[must_use]
	pub(crate) const fn data_offset(&self) -> u64 {
		self.data_offset
	}

	#[must_use]
	pub(crate) const fn file_offset(&self) -> u64 {
		self.file_offset
	}

	#[must_use]
	pub(crate) const fn encoded_bytes(&self) -> u64 {
		self.encoded_bytes
	}

	#[must_use]
	pub(crate) const fn data_end(&self) -> u64 {
		self.data_offset + self.encoded_bytes
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GgufArchive<'a> {
	version: u32,
	endian: GgufEndian,
	alignment: u32,
	data_start: u64,
	data: &'a [u8],
	metadata: Vec<GgufMetadataEntry<'a>>,
	tensors: Vec<GgufTensor<'a>>,
}

impl<'a> GgufArchive<'a> {
	#[must_use]
	pub(crate) const fn version(&self) -> u32 {
		self.version
	}

	#[must_use]
	pub(crate) const fn endian(&self) -> GgufEndian {
		self.endian
	}

	#[must_use]
	pub(crate) const fn alignment(&self) -> u32 {
		self.alignment
	}

	#[must_use]
	pub(crate) const fn data_start(&self) -> u64 {
		self.data_start
	}

	#[must_use]
	pub(crate) fn data(&self) -> &'a [u8] {
		self.data
	}

	#[must_use]
	pub(crate) fn metadata(&self) -> &[GgufMetadataEntry<'a>] {
		&self.metadata
	}

	#[must_use]
	pub(crate) fn metadata_entry(&self, key: &str) -> Option<&GgufMetadataEntry<'a>> {
		self.metadata.iter().find(|entry| entry.key == key)
	}

	#[must_use]
	pub(crate) fn tensors(&self) -> &[GgufTensor<'a>] {
		&self.tensors
	}

	#[must_use]
	pub(crate) fn tensor(&self, name: &str) -> Option<&GgufTensor<'a>> {
		self.tensors.iter().find(|tensor| tensor.name == name)
	}

	#[must_use]
	pub(crate) fn raw_tensor(&self, name: &str) -> Option<&'a [u8]> {
		let tensor = self.tensor(name)?;
		let begin = match usize::try_from(tensor.data_offset) {
			Ok(value) => value,
			Err(..) => return None,
		};
		let end = match usize::try_from(tensor.data_end()) {
			Ok(value) => value,
			Err(..) => return None,
		};
		self.data.get(begin..end)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum GgufErrorKind {
	InvalidLimit,
	FileLimitExceeded,
	Truncated,
	InvalidMagic,
	UnsupportedVersion,
	UnsupportedEndian,
	MetadataLimitExceeded,
	TensorLimitExceeded,
	RankLimitExceeded,
	StringLimitExceeded,
	ArrayLimitExceeded,
	ArrayDepthExceeded,
	InvalidUtf8,
	InvalidMetadataKey,
	DuplicateMetadata,
	UnsupportedMetadataType,
	InvalidBoolean,
	InvalidAlignment,
	InvalidTensorName,
	DuplicateTensor,
	UnsupportedTensorType,
	InvalidDimension,
	InvalidOffset,
	OverlappingTensor,
	NonZeroPadding,
	TrailingData,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GgufError {
	pub(crate) kind: GgufErrorKind,
	pub(crate) detail: String,
}

impl GgufError {
	fn new(kind: GgufErrorKind, detail: impl ToString) -> Self {
		Self {
			kind,
			detail: detail.to_string(),
		}
	}

	#[must_use]
	pub(crate) const fn kind(&self) -> GgufErrorKind {
		self.kind
	}

	#[must_use]
	pub(crate) fn detail(&self) -> &str {
		&self.detail
	}
}

impl fmt::Display for GgufError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	}
}

impl std::error::Error for GgufError {}

pub(crate) type GgufResult<T> = Result<T, GgufError>;

#[derive(Clone, Debug)]
struct RawTensor<'a> {
	name: &'a str,
	dimensions: Vec<u64>,
	tensor_type: GgufTensorType,
	data_offset: u64,
}

#[derive(Debug)]
struct Reader<'a> {
	bytes: &'a [u8],
	endian: GgufEndian,
	position: usize,
	string_bytes_remaining: u64,
	array_elements_remaining: u64,
	array_depth_limit: u32,
}

impl<'a> Reader<'a> {
	const fn new(bytes: &'a [u8], endian: GgufEndian, limits: GgufLimits) -> Self {
		Self {
			bytes,
			endian,
			position: 8,
			string_bytes_remaining: limits.string_bytes.get(),
			array_elements_remaining: limits.array_elements.get(),
			array_depth_limit: limits.array_depth.get(),
		}
	}

	const fn position(&self) -> usize {
		self.position
	}

	const fn remaining(&self) -> usize {
		self.bytes.len() - self.position
	}

	fn take(&mut self, length: usize, context: &str) -> GgufResult<&'a [u8]> {
		let end = self.position.checked_add(length).ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				format!("{context} byte range overflowed host address space"),
			)
		})?;
		let value = self.bytes.get(self.position..end).ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::Truncated,
				format!(
					"{context} ends at byte {end}, GGUF image has {} bytes",
					self.bytes.len()
				),
			)
		})?;
		self.position = end;
		Ok(value)
	}

	fn read_u8(&mut self, context: &str) -> GgufResult<u8> {
		Ok(self.take(1, context)?[0])
	}

	fn read_i8(&mut self, context: &str) -> GgufResult<i8> {
		Ok(i8::from_ne_bytes([self.read_u8(context)?]))
	}

	fn read_u16(&mut self, context: &str) -> GgufResult<u16> {
		let bytes = two_bytes(self.take(2, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u16::from_le_bytes(bytes),
			GgufEndian::Big => u16::from_be_bytes(bytes),
		})
	}

	fn read_i16(&mut self, context: &str) -> GgufResult<i16> {
		let bytes = two_bytes(self.take(2, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i16::from_le_bytes(bytes),
			GgufEndian::Big => i16::from_be_bytes(bytes),
		})
	}

	fn read_u32(&mut self, context: &str) -> GgufResult<u32> {
		let bytes = four_bytes(self.take(4, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u32::from_le_bytes(bytes),
			GgufEndian::Big => u32::from_be_bytes(bytes),
		})
	}

	fn read_i32(&mut self, context: &str) -> GgufResult<i32> {
		let bytes = four_bytes(self.take(4, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i32::from_le_bytes(bytes),
			GgufEndian::Big => i32::from_be_bytes(bytes),
		})
	}

	fn read_u64(&mut self, context: &str) -> GgufResult<u64> {
		let bytes = eight_bytes(self.take(8, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u64::from_le_bytes(bytes),
			GgufEndian::Big => u64::from_be_bytes(bytes),
		})
	}

	fn read_i64(&mut self, context: &str) -> GgufResult<i64> {
		let bytes = eight_bytes(self.take(8, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i64::from_le_bytes(bytes),
			GgufEndian::Big => i64::from_be_bytes(bytes),
		})
	}

	fn read_string(&mut self, context: &str, per_value_limit: u64) -> GgufResult<&'a str> {
		let length = self.read_u64(&format!("{context} length"))?;
		match length <= per_value_limit {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::StringLimitExceeded,
					format!("{context} has {length} bytes, per-value limit is {per_value_limit}"),
				));
			}
		}?;
		match length <= self.string_bytes_remaining {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::StringLimitExceeded,
					format!(
						"{context} consumes {length} bytes, aggregate remaining budget is {}",
						self.string_bytes_remaining
					),
				));
			}
		}?;
		let host_length = bounded_usize(length, context)?;
		let raw = self.take(host_length, context)?;
		let value = str::from_utf8(raw).map_err(|error| {
			GgufError::new(
				GgufErrorKind::InvalidUtf8,
				format!("{context} is not valid UTF-8: {error}"),
			)
		})?;
		self.string_bytes_remaining -= length;
		Ok(value)
	}

	fn read_metadata_value(&mut self, value_type: GgufMetadataType, depth: u32) -> GgufResult<GgufMetadataValue<'a>> {
		match value_type {
			GgufMetadataType::U8 => Ok(GgufMetadataValue::U8(self.read_u8("uint8 metadata value")?)),
			GgufMetadataType::I8 => Ok(GgufMetadataValue::I8(self.read_i8("int8 metadata value")?)),
			GgufMetadataType::U16 => Ok(GgufMetadataValue::U16(
				self.read_u16("uint16 metadata value")?,
			)),
			GgufMetadataType::I16 => Ok(GgufMetadataValue::I16(
				self.read_i16("int16 metadata value")?,
			)),
			GgufMetadataType::U32 => Ok(GgufMetadataValue::U32(
				self.read_u32("uint32 metadata value")?,
			)),
			GgufMetadataType::I32 => Ok(GgufMetadataValue::I32(
				self.read_i32("int32 metadata value")?,
			)),
			GgufMetadataType::F32 => Ok(GgufMetadataValue::F32Bits(
				self.read_u32("float32 metadata bits")?,
			)),
			GgufMetadataType::Bool => match self.read_u8("boolean metadata value")? {
				0 => Ok(GgufMetadataValue::Bool(false)),
				1 => Ok(GgufMetadataValue::Bool(true)),
				other => Err(GgufError::new(
					GgufErrorKind::InvalidBoolean,
					format!("GGUF boolean byte is {other}, expected zero or one"),
				)),
			},
			GgufMetadataType::String => Ok(GgufMetadataValue::String(
				self.read_string("metadata string", u64::MAX)?,
			)),
			GgufMetadataType::Array => self.read_metadata_array(depth),
			GgufMetadataType::U64 => Ok(GgufMetadataValue::U64(
				self.read_u64("uint64 metadata value")?,
			)),
			GgufMetadataType::I64 => Ok(GgufMetadataValue::I64(
				self.read_i64("int64 metadata value")?,
			)),
			GgufMetadataType::F64 => Ok(GgufMetadataValue::F64Bits(
				self.read_u64("float64 metadata bits")?,
			)),
		}
	}

	fn read_metadata_array(&mut self, depth: u32) -> GgufResult<GgufMetadataValue<'a>> {
		let nested_depth = depth.checked_add(1).ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				"GGUF metadata array depth overflowed u32",
			)
		})?;
		match nested_depth <= self.array_depth_limit {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::ArrayDepthExceeded,
					format!(
						"metadata array depth {nested_depth} exceeds limit {}",
						self.array_depth_limit
					),
				));
			}
		}?;
		let element_type = GgufMetadataType::parse(self.read_u32("metadata array element type")?)?;
		let length = self.read_u64("metadata array length")?;
		match length <= self.array_elements_remaining {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::ArrayLimitExceeded,
					format!(
						"metadata array has {length} elements, aggregate remaining budget is {}",
						self.array_elements_remaining
					),
				));
			}
		}?;
		self.array_elements_remaining -= length;
		let minimum_bytes = minimum_metadata_value_bytes(element_type)
			.checked_mul(length)
			.ok_or_else(|| {
				GgufError::new(
					GgufErrorKind::ArithmeticOverflow,
					"minimum metadata array byte count overflowed u64",
				)
			})?;
		require_available_bytes(minimum_bytes, self.remaining(), "metadata array values")?;
		let capacity = bounded_usize(length, "metadata array length")?;
		let mut values = Vec::with_capacity(capacity);
		for element_index in 0..length {
			let value = self.read_metadata_value(element_type, nested_depth)?;
			debug_assert!(element_index < length);
			values.push(value);
		}
		Ok(GgufMetadataValue::Array(GgufMetadataArray {
			element_type,
			values,
		}))
	}
}

/// Parse one complete GGUF v2/v3 image without decoding tensor payloads.
///
/// The result borrows all strings and tensor bytes from `bytes`. Structural
/// counts, recursion, string storage, tensor ranks, alignment, block layouts,
/// and data spans are validated before the archive is returned.
///
/// # Errors
///
/// Returns a fail-closed bound, encoding, layout, or arithmetic error.
pub(crate) fn parse_gguf(bytes: &[u8], limits: GgufLimits) -> GgufResult<GgufArchive<'_>> {
	let file_bytes = u64::try_from(bytes.len()).map_err(|error| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			format!("GGUF file length cannot be represented by u64: {error}"),
		)
	})?;
	match file_bytes <= limits.file_bytes.get() {
		true => Ok(()),
		false => {
			return Err(GgufError::new(
				GgufErrorKind::FileLimitExceeded,
				format!(
					"GGUF image has {file_bytes} bytes, limit is {}",
					limits.file_bytes
				),
			));
		}
	}?;

	let magic = bytes.get(..4).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::Truncated,
			format!("GGUF image has {} bytes, fewer than the magic", bytes.len()),
		)
	})?;
	match magic == GGUF_MAGIC {
		true => Ok(()),
		false => {
			return Err(GgufError::new(
				GgufErrorKind::InvalidMagic,
				format!("GGUF magic bytes are {magic:?}, expected {GGUF_MAGIC:?}"),
			));
		}
	}?;

	let version_bytes = bytes.get(4..8).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::Truncated,
			"GGUF image ends before the version field",
		)
	})?;
	let (version, endian) = detect_version(four_bytes(version_bytes, "GGUF version")?)?;
	let mut reader = Reader::new(bytes, endian, limits);
	let tensor_count = reader.read_u64("tensor count")?;
	let metadata_count = reader.read_u64("metadata pair count")?;
	require_count(
		metadata_count,
		limits.metadata_pairs.get(),
		GgufErrorKind::MetadataLimitExceeded,
		"metadata pair",
	)?;
	require_count(
		tensor_count,
		limits.tensors.get(),
		GgufErrorKind::TensorLimitExceeded,
		"tensor",
	)?;
	let minimum_metadata_bytes = metadata_count.checked_mul(13).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			"minimum metadata section byte count overflowed u64",
		)
	})?;
	let minimum_tensor_bytes = tensor_count.checked_mul(33).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			"minimum tensor-info section byte count overflowed u64",
		)
	})?;
	let minimum_header_bytes = minimum_metadata_bytes
		.checked_add(minimum_tensor_bytes)
		.ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				"minimum GGUF header byte count overflowed u64",
			)
		})?;
	require_available_bytes(
		minimum_header_bytes,
		reader.remaining(),
		"declared metadata and tensor-info records",
	)?;

	let metadata_capacity = bounded_usize(metadata_count, "metadata pair count")?;
	let mut metadata = Vec::with_capacity(metadata_capacity);
	let mut metadata_names = BTreeSet::new();
	for pair_index in 0..metadata_count {
		let key = reader.read_string("metadata key", METADATA_KEY_BYTES_MAX)?;
		validate_metadata_key(key)?;
		match metadata_names.insert(key) {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::DuplicateMetadata,
					format!("metadata key {key:?} occurs more than once"),
				));
			}
		}?;
		let value_type = GgufMetadataType::parse(reader.read_u32("metadata value type")?)?;
		let value = reader.read_metadata_value(value_type, 0)?;
		debug_assert!(pair_index < metadata_count);
		metadata.push(GgufMetadataEntry { key, value });
	}
	let alignment = metadata_alignment(&metadata, limits)?;
	let alignment_u64 = u64::from(alignment);

	let tensor_capacity = bounded_usize(tensor_count, "tensor count")?;
	let mut raw_tensors = Vec::with_capacity(tensor_capacity);
	let mut tensor_names = BTreeSet::new();
	for tensor_index in 0..tensor_count {
		let name = reader.read_string("tensor name", TENSOR_NAME_BYTES_MAX)?;
		validate_tensor_name(name)?;
		match tensor_names.insert(name) {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::DuplicateTensor,
					format!("tensor name {name:?} occurs more than once"),
				));
			}
		}?;
		let rank = reader.read_u32("tensor rank")?;
		match (rank > 0, rank <= limits.rank.get()) {
			(true, true) => Ok(()),
			(false, true) | (false, false) => {
				return Err(GgufError::new(
					GgufErrorKind::InvalidDimension,
					format!("tensor {name:?} has zero rank"),
				));
			}
			(true, false) => {
				return Err(GgufError::new(
					GgufErrorKind::RankLimitExceeded,
					format!("tensor {name:?} rank {rank} exceeds limit {}", limits.rank),
				));
			}
		}?;
		let mut dimensions = Vec::with_capacity(usize::try_from(rank).map_err(|error| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				format!("tensor rank cannot address host memory: {error}"),
			)
		})?);
		for axis in 0..rank {
			let dimension = reader.read_u64("tensor dimension")?;
			match dimension > 0 {
				true => dimensions.push(dimension),
				false => {
					return Err(GgufError::new(
						GgufErrorKind::InvalidDimension,
						format!("tensor {name:?} axis {axis} has zero extent"),
					));
				}
			}
		}
		let tensor_type = GgufTensorType::parse(reader.read_u32("tensor type")?)?;
		let data_offset = reader.read_u64("tensor data offset")?;
		match data_offset % alignment_u64 {
			0 => Ok(()),
			remainder => {
				return Err(GgufError::new(
					GgufErrorKind::InvalidOffset,
					format!(
						"tensor {name:?} offset {data_offset} has remainder {remainder} under alignment {alignment}"
					),
				));
			}
		}?;
		debug_assert!(tensor_index < tensor_count);
		raw_tensors.push(RawTensor {
			name,
			dimensions,
			tensor_type,
			data_offset,
		});
	}

	let header_end = u64::try_from(reader.position()).map_err(|error| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			format!("GGUF header position cannot be represented by u64: {error}"),
		)
	})?;
	let data_start = align_up(header_end, alignment_u64)?;
	let data_start_host = bounded_usize(data_start, "tensor data start")?;
	let header_padding = bytes
		.get(reader.position()..data_start_host)
		.ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::Truncated,
				format!(
					"aligned tensor data begins at {data_start}, image has {} bytes",
					bytes.len()
				),
			)
		})?;
	require_zero_padding(header_padding, "header-to-data padding")?;
	let data = bytes.get(data_start_host..).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::Truncated,
			"tensor data start lies beyond the GGUF image",
		)
	})?;
	let tensors = materialize_tensors(raw_tensors, data_start, data, alignment_u64)?;

	Ok(GgufArchive {
		version,
		endian,
		alignment,
		data_start,
		data,
		metadata,
		tensors,
	})
}

fn materialize_tensors<'a>(
	raw_tensors: Vec<RawTensor<'a>>,
	data_start: u64,
	data: &'a [u8],
	alignment: u64,
) -> GgufResult<Vec<GgufTensor<'a>>> {
	let data_bytes = u64::try_from(data.len()).map_err(|error| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			format!("GGUF tensor data length cannot be represented by u64: {error}"),
		)
	})?;
	let mut tensors = Vec::with_capacity(raw_tensors.len());
	for raw in raw_tensors {
		let encoded_bytes = tensor_encoded_bytes(raw.name, &raw.dimensions, raw.tensor_type)?;
		let data_end = raw.data_offset.checked_add(encoded_bytes).ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				format!("tensor {:?} data end overflowed u64", raw.name),
			)
		})?;
		match data_end <= data_bytes {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::InvalidOffset,
					format!(
						"tensor {:?} ends at {data_end}, tensor data has {data_bytes} bytes",
						raw.name
					),
				));
			}
		}?;
		let file_offset = data_start.checked_add(raw.data_offset).ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				format!("tensor {:?} absolute file offset overflowed u64", raw.name),
			)
		})?;
		tensors.push(GgufTensor {
			name: raw.name,
			dimensions: raw.dimensions,
			tensor_type: raw.tensor_type,
			data_offset: raw.data_offset,
			file_offset,
			encoded_bytes,
		});
	}
	validate_tensor_spans(&tensors, data, alignment)?;
	Ok(tensors)
}

fn tensor_encoded_bytes(name: &str, dimensions: &[u64], tensor_type: GgufTensorType) -> GgufResult<u64> {
	let block_elements = tensor_type.block_elements();
	let first_dimension = dimensions[0];
	match first_dimension % block_elements {
		0 => Ok(()),
		remainder => {
			return Err(GgufError::new(
				GgufErrorKind::InvalidDimension,
				format!(
					"tensor {name:?} first dimension {first_dimension} has remainder {remainder} for {}-element {:?} blocks",
					block_elements, tensor_type
				),
			));
		}
	}?;
	let elements = dimensions.iter().try_fold(1_u64, |product, dimension| {
		product.checked_mul(*dimension).ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				format!("tensor {name:?} dimension product overflowed u64"),
			)
		})
	})?;
	let blocks = elements / block_elements;
	blocks.checked_mul(tensor_type.block_bytes())
		.ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::ArithmeticOverflow,
				format!("tensor {name:?} encoded byte count overflowed u64"),
			)
		})
}

fn validate_tensor_spans(tensors: &[GgufTensor<'_>], data: &[u8], alignment: u64) -> GgufResult<()> {
	let mut spans = tensors.iter().collect::<Vec<_>>();
	spans.sort_by_key(|tensor| tensor.data_offset);
	let mut cursor = 0_u64;
	for tensor in spans {
		match tensor.data_offset >= cursor {
			true => Ok(()),
			false => {
				return Err(GgufError::new(
					GgufErrorKind::OverlappingTensor,
					format!(
						"tensor {:?} begins at {}, before prior tensor end {cursor}",
						tensor.name, tensor.data_offset
					),
				));
			}
		}?;
		match tensor.data_offset % alignment {
			0 => Ok(()),
			remainder => {
				return Err(GgufError::new(
					GgufErrorKind::InvalidOffset,
					format!(
						"tensor {:?} offset {} has alignment remainder {remainder}",
						tensor.name, tensor.data_offset
					),
				));
			}
		}?;
		let padding_start = bounded_usize(cursor, "tensor padding start")?;
		let padding_end = bounded_usize(tensor.data_offset, "tensor padding end")?;
		let padding = data.get(padding_start..padding_end).ok_or_else(|| {
			GgufError::new(
				GgufErrorKind::InvalidOffset,
				format!(
					"padding before tensor {:?} lies outside tensor data",
					tensor.name
				),
			)
		})?;
		require_zero_padding(padding, "inter-tensor padding")?;
		cursor = tensor.data_end();
	}
	let data_bytes = u64::try_from(data.len()).map_err(|error| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			format!("tensor data length cannot be represented by u64: {error}"),
		)
	})?;
	match cursor == data_bytes {
		true => Ok(()),
		false => Err(GgufError::new(
			GgufErrorKind::TrailingData,
			format!("validated tensors end at {cursor}, tensor data has {data_bytes} bytes"),
		)),
	}
}

fn metadata_alignment(metadata: &[GgufMetadataEntry<'_>], limits: GgufLimits) -> GgufResult<u32> {
	let entry = metadata
		.iter()
		.find(|entry| entry.key == "general.alignment");
	let alignment = match entry {
		None => DEFAULT_ALIGNMENT,
		Some(found) => match &found.value {
			GgufMetadataValue::U32(value) => *value,
			other => {
				return Err(GgufError::new(
					GgufErrorKind::InvalidAlignment,
					format!(
						"general.alignment has type {:?}, expected U32",
						other.value_type()
					),
				));
			}
		},
	};
	match (
		alignment > 0,
		alignment % 8 == 0,
		u64::from(alignment) <= limits.file_bytes.get(),
	) {
		(true, true, true) => Ok(alignment),
		(false, true, true) | (false, false, true) | (false, true, false) | (false, false, false) => {
			Err(GgufError::new(
				GgufErrorKind::InvalidAlignment,
				"general.alignment must be nonzero",
			))
		}
		(true, false, true) | (true, false, false) => Err(GgufError::new(
			GgufErrorKind::InvalidAlignment,
			format!("general.alignment {alignment} is not a multiple of eight"),
		)),
		(true, true, false) => Err(GgufError::new(
			GgufErrorKind::InvalidAlignment,
			format!(
				"general.alignment {alignment} exceeds file byte limit {}",
				limits.file_bytes
			),
		)),
	}
}

fn validate_metadata_key(key: &str) -> GgufResult<()> {
	match key.len() {
		0 => Err(GgufError::new(
			GgufErrorKind::InvalidMetadataKey,
			"GGUF metadata key is empty",
		)),
		positive_length => {
			match positive_length <= bounded_usize(METADATA_KEY_BYTES_MAX, "metadata key maximum")? {
				true => Ok(()),
				false => Err(GgufError::new(
					GgufErrorKind::InvalidMetadataKey,
					"GGUF metadata key exceeds the format maximum",
				)),
			}
		}
	}?;
	for segment in key.split('.') {
		match segment.len() {
			0 => Err(GgufError::new(
				GgufErrorKind::InvalidMetadataKey,
				format!("metadata key {key:?} contains an empty hierarchy segment"),
			)),
			1.. => Ok(()),
		}?;
		for byte in segment.bytes() {
			let valid = byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_';
			match valid {
				true => Ok(()),
				false => Err(GgufError::new(
					GgufErrorKind::InvalidMetadataKey,
					format!("metadata key {key:?} is not hierarchical lower snake case"),
				)),
			}?;
		}
	}
	Ok(())
}

fn validate_tensor_name(name: &str) -> GgufResult<()> {
	match name.len() {
		0 => Err(GgufError::new(
			GgufErrorKind::InvalidTensorName,
			"GGUF tensor name is empty",
		)),
		positive_length => match positive_length <= bounded_usize(TENSOR_NAME_BYTES_MAX, "tensor name maximum")? {
			true => Ok(()),
			false => Err(GgufError::new(
				GgufErrorKind::InvalidTensorName,
				format!("tensor name {name:?} exceeds the 64-byte format maximum"),
			)),
		},
	}
}

fn detect_version(bytes: [u8; 4]) -> GgufResult<(u32, GgufEndian)> {
	let little = u32::from_le_bytes(bytes);
	match little {
		2 | 3 => Ok((little, GgufEndian::Little)),
		other_little => {
			let big = u32::from_be_bytes(bytes);
			match big {
				3 => Ok((3, GgufEndian::Big)),
				2 => Err(GgufError::new(
					GgufErrorKind::UnsupportedEndian,
					"big-endian encoding requires GGUF version 3",
				)),
				other_big => Err(GgufError::new(
					GgufErrorKind::UnsupportedVersion,
					format!(
						"GGUF version bytes decode to little-endian {other_little} and big-endian {other_big}"
					),
				)),
			}
		}
	}
}

fn require_count(actual: u64, limit: u64, kind: GgufErrorKind, name: &str) -> GgufResult<()> {
	match actual <= limit {
		true => Ok(()),
		false => Err(GgufError::new(
			kind,
			format!("GGUF declares {actual} {name}s, limit is {limit}"),
		)),
	}
}

fn require_zero_padding(bytes: &[u8], context: &str) -> GgufResult<()> {
	let all_zero = bytes.iter().all(|byte| *byte == 0);
	match all_zero {
		true => Ok(()),
		false => Err(GgufError::new(
			GgufErrorKind::NonZeroPadding,
			format!("{context} contains nonzero bytes"),
		)),
	}
}

const fn minimum_metadata_value_bytes(value_type: GgufMetadataType) -> u64 {
	match value_type {
		GgufMetadataType::U8 | GgufMetadataType::I8 | GgufMetadataType::Bool => 1,
		GgufMetadataType::U16 | GgufMetadataType::I16 => 2,
		GgufMetadataType::U32 | GgufMetadataType::I32 | GgufMetadataType::F32 => 4,
		GgufMetadataType::String | GgufMetadataType::U64 | GgufMetadataType::I64 | GgufMetadataType::F64 => 8,
		GgufMetadataType::Array => 12,
	}
}

fn require_available_bytes(required: u64, remaining: usize, context: &str) -> GgufResult<()> {
	let available = u64::try_from(remaining).map_err(|error| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			format!("remaining host byte count cannot be represented by u64: {error}"),
		)
	})?;
	match required <= available {
		true => Ok(()),
		false => Err(GgufError::new(
			GgufErrorKind::Truncated,
			format!("{context} require at least {required} bytes, only {available} remain"),
		)),
	}
}

fn align_up(value: u64, alignment: u64) -> GgufResult<u64> {
	let remainder = value % alignment;
	let padding = (alignment - remainder) % alignment;
	value.checked_add(padding).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			"GGUF alignment calculation overflowed u64",
		)
	})
}

fn bounded_usize(value: u64, context: &str) -> GgufResult<usize> {
	usize::try_from(value).map_err(|error| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			format!("{context} cannot address host memory: {error}"),
		)
	})
}

fn two_bytes(bytes: &[u8], context: &str) -> GgufResult<[u8; 2]> {
	<[u8; 2]>::try_from(bytes).map_err(|error| {
		GgufError::new(
			GgufErrorKind::Truncated,
			format!("{context} is not exactly two bytes: {error}"),
		)
	})
}

fn four_bytes(bytes: &[u8], context: &str) -> GgufResult<[u8; 4]> {
	<[u8; 4]>::try_from(bytes).map_err(|error| {
		GgufError::new(
			GgufErrorKind::Truncated,
			format!("{context} is not exactly four bytes: {error}"),
		)
	})
}

fn eight_bytes(bytes: &[u8], context: &str) -> GgufResult<[u8; 8]> {
	<[u8; 8]>::try_from(bytes).map_err(|error| {
		GgufError::new(
			GgufErrorKind::Truncated,
			format!("{context} is not exactly eight bytes: {error}"),
		)
	})
}

fn nonzero_u64(name: &str, value: u64) -> GgufResult<NonZeroU64> {
	NonZeroU64::new(value).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::InvalidLimit,
			format!("{name} limit must be nonzero"),
		)
	})
}

fn nonzero_u32(name: &str, value: u32) -> GgufResult<NonZeroU32> {
	NonZeroU32::new(value).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::InvalidLimit,
			format!("{name} limit must be nonzero"),
		)
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	const F32_PAYLOAD_BITS: u32 = 0x7fc0_1234;
	const F64_PAYLOAD_BITS: u64 = 0x7ff8_0000_0000_5678;

	#[derive(Debug)]
	struct Encoder {
		endian: GgufEndian,
		bytes: Vec<u8>,
	}

	impl Encoder {
		fn begin(version: u32, endian: GgufEndian, tensors: u64, metadata: u64) -> Self {
			let mut encoder = Self {
				endian,
				bytes: b"GGUF".to_vec(),
			};
			encoder.u32(version);
			encoder.u64(tensors);
			encoder.u64(metadata);
			encoder
		}

		fn position(&self) -> usize {
			self.bytes.len()
		}

		fn u8(&mut self, value: u8) {
			self.bytes.push(value);
		}

		fn u16(&mut self, value: u16) {
			let encoded = match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			};
			self.bytes.extend_from_slice(&encoded);
		}

		fn u32(&mut self, value: u32) {
			let encoded = match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			};
			self.bytes.extend_from_slice(&encoded);
		}

		fn u64(&mut self, value: u64) {
			let encoded = match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			};
			self.bytes.extend_from_slice(&encoded);
		}

		fn string(&mut self, value: &str) {
			self.u64(host_u64(value.len()));
			self.bytes.extend_from_slice(value.as_bytes());
		}

		fn metadata_u8(&mut self, key: &str, value: u8) {
			self.string(key);
			self.u32(GgufMetadataType::U8.code());
			self.u8(value);
		}

		fn align_zero(&mut self, alignment: u64) {
			let aligned = parsed_align_up(host_u64(self.bytes.len()), alignment);
			self.bytes.resize(host_usize(aligned), 0);
		}
	}

	#[derive(Debug)]
	struct Fixture {
		bytes: Vec<u8>,
		header_end: usize,
		data_start: usize,
		first_dimension: usize,
		boolean_value: usize,
		float_type: usize,
		quant_dimension: usize,
		quant_type: usize,
		quant_offset: usize,
	}

	fn fixture(version: u32, endian: GgufEndian) -> Fixture {
		let mut encoder = Encoder::begin(version, endian, 2, 6);

		encoder.string("general.alignment");
		encoder.u32(GgufMetadataType::U32.code());
		encoder.u32(DEFAULT_ALIGNMENT);

		encoder.string("general.name");
		encoder.u32(GgufMetadataType::String.code());
		encoder.string("borrowed");

		encoder.string("test.float");
		let float_type = encoder.position();
		encoder.u32(GgufMetadataType::F32.code());
		encoder.u32(F32_PAYLOAD_BITS);

		encoder.string("test.double");
		encoder.u32(GgufMetadataType::F64.code());
		encoder.u64(F64_PAYLOAD_BITS);

		encoder.string("test.nested");
		encoder.u32(GgufMetadataType::Array.code());
		encoder.u32(GgufMetadataType::Array.code());
		encoder.u64(1);
		encoder.u32(GgufMetadataType::U16.code());
		encoder.u64(2);
		encoder.u16(0x1122);
		encoder.u16(0x3344);

		encoder.string("test.enabled");
		encoder.u32(GgufMetadataType::Bool.code());
		let boolean_value = encoder.position();
		encoder.u8(1);

		encoder.string("weights");
		encoder.u32(1);
		let first_dimension = encoder.position();
		encoder.u64(2);
		encoder.u32(GgufTensorType::F32.code());
		encoder.u64(0);

		encoder.string("quant");
		encoder.u32(1);
		let quant_dimension = encoder.position();
		encoder.u64(32);
		let quant_type = encoder.position();
		encoder.u32(GgufTensorType::Q4_0.code());
		let quant_offset = encoder.position();
		encoder.u64(32);

		let header_end = encoder.position();
		encoder.align_zero(u64::from(DEFAULT_ALIGNMENT));
		let data_start = encoder.position();
		encoder
			.bytes
			.extend_from_slice(&[0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]);
		encoder.bytes.resize(data_start + 32, 0);
		encoder.bytes.extend_from_slice(&[0xa5; 18]);

		Fixture {
			bytes: encoder.bytes,
			header_end,
			data_start,
			first_dimension,
			boolean_value,
			float_type,
			quant_dimension,
			quant_type,
			quant_offset,
		}
	}

	fn generous_limits(bytes: usize) -> GgufLimits {
		limits(host_u64(bytes), 64, 64, 8, 16_384, 16_384, 8)
	}

	fn limits(
		file_bytes: u64,
		metadata_pairs: u64,
		tensors: u64,
		rank: u32,
		string_bytes: u64,
		array_elements: u64,
		array_depth: u32,
	) -> GgufLimits {
		match GgufLimits::new(
			file_bytes,
			metadata_pairs,
			tensors,
			rank,
			string_bytes,
			array_elements,
			array_depth,
		) {
			Ok(value) => value,
			Err(error) => panic!("test limits must be valid: {error}"),
		}
	}

	fn parse_valid<'a>(bytes: &'a [u8], limits: GgufLimits) -> GgufArchive<'a> {
		match parse_gguf(bytes, limits) {
			Ok(archive) => archive,
			Err(error) => panic!("valid fixture was rejected: {error}"),
		}
	}

	fn error_kind(bytes: &[u8], limits: GgufLimits) -> GgufErrorKind {
		match parse_gguf(bytes, limits) {
			Ok(archive) => panic!("malformed fixture parsed successfully: {archive:?}"),
			Err(error) => error.kind(),
		}
	}

	fn host_u64(value: usize) -> u64 {
		match u64::try_from(value) {
			Ok(converted) => converted,
			Err(error) => panic!("test host length did not fit u64: {error}"),
		}
	}

	fn host_usize(value: u64) -> usize {
		match usize::try_from(value) {
			Ok(converted) => converted,
			Err(error) => panic!("test GGUF length did not fit usize: {error}"),
		}
	}

	fn parsed_align_up(value: u64, alignment: u64) -> u64 {
		match align_up(value, alignment) {
			Ok(aligned) => aligned,
			Err(error) => panic!("test alignment failed: {error}"),
		}
	}

	fn patch_u32(bytes: &mut [u8], endian: GgufEndian, position: usize, value: u32) {
		let encoded = match endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		};
		bytes[position..position + 4].copy_from_slice(&encoded);
	}

	fn patch_u64(bytes: &mut [u8], endian: GgufEndian, position: usize, value: u64) {
		let encoded = match endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		};
		bytes[position..position + 8].copy_from_slice(&encoded);
	}

	fn rank_two_fixture() -> Vec<u8> {
		let mut encoder = Encoder::begin(3, GgufEndian::Little, 1, 0);
		encoder.string("rank");
		encoder.u32(2);
		encoder.u64(1);
		encoder.u64(1);
		encoder.u32(GgufTensorType::I8.code());
		encoder.u64(0);
		encoder.align_zero(u64::from(DEFAULT_ALIGNMENT));
		encoder.u8(0x5a);
		encoder.bytes
	}

	fn duplicate_metadata_fixture() -> Vec<u8> {
		let mut encoder = Encoder::begin(3, GgufEndian::Little, 0, 2);
		encoder.metadata_u8("dup.key", 1);
		encoder.metadata_u8("dup.key", 2);
		encoder.bytes
	}

	fn duplicate_tensor_fixture() -> Vec<u8> {
		let mut encoder = Encoder::begin(3, GgufEndian::Little, 2, 0);
		encoder.string("same");
		encoder.u32(1);
		encoder.u64(1);
		encoder.u32(GgufTensorType::I8.code());
		encoder.u64(0);
		encoder.string("same");
		encoder.u32(1);
		encoder.u64(1);
		encoder.u32(GgufTensorType::I8.code());
		encoder.u64(0);
		encoder.bytes
	}

	fn invalid_key_fixture(key: &str) -> Vec<u8> {
		let mut encoder = Encoder::begin(3, GgufEndian::Little, 0, 1);
		encoder.metadata_u8(key, 1);
		encoder.bytes
	}

	fn invalid_tensor_name_fixture(name: &str) -> Vec<u8> {
		let mut encoder = Encoder::begin(3, GgufEndian::Little, 1, 0);
		encoder.string(name);
		encoder.u32(1);
		encoder.u64(1);
		encoder.u32(GgufTensorType::I8.code());
		encoder.u64(0);
		encoder.u8(0);
		encoder.bytes
	}

	#[test]
	fn parses_supported_endianness_and_borrows_payloads() {
		for endian in [GgufEndian::Little, GgufEndian::Big] {
			let sample = fixture(3, endian);
			let archive = parse_valid(&sample.bytes, generous_limits(sample.bytes.len()));
			assert_eq!(archive.version(), 3);
			assert_eq!(archive.endian(), endian);
			assert_eq!(archive.alignment(), DEFAULT_ALIGNMENT);
			assert_eq!(archive.data_start(), host_u64(sample.data_start));
			assert_eq!(archive.data().len(), 50);
			assert_eq!(archive.metadata().len(), 6);
			assert_eq!(archive.metadata()[0].key(), "general.alignment");
			assert_eq!(archive.tensors().len(), 2);
			assert_eq!(archive.tensors()[0].name(), "weights");
			assert_eq!(archive.tensors()[1].name(), "quant");

			let Some(float) = archive.metadata_entry("test.float") else {
				panic!("float metadata was not indexed");
			};
			assert_eq!(float.value(), &GgufMetadataValue::F32Bits(F32_PAYLOAD_BITS));
			assert_eq!(float.value().value_type(), GgufMetadataType::F32);

			let Some(double) = archive.metadata_entry("test.double") else {
				panic!("double metadata was not indexed");
			};
			assert_eq!(
				double.value(),
				&GgufMetadataValue::F64Bits(F64_PAYLOAD_BITS)
			);

			let Some(nested) = archive.metadata_entry("test.nested") else {
				panic!("nested metadata was not indexed");
			};
			let GgufMetadataValue::Array(outer) = nested.value() else {
				panic!("nested metadata was not an array");
			};
			assert_eq!(outer.element_type(), GgufMetadataType::Array);
			assert_eq!(outer.values().len(), 1);
			let GgufMetadataValue::Array(inner) = &outer.values()[0] else {
				panic!("outer array element was not an array");
			};
			assert_eq!(inner.element_type(), GgufMetadataType::U16);
			assert_eq!(
				inner.values(),
				&[
					GgufMetadataValue::U16(0x1122),
					GgufMetadataValue::U16(0x3344),
				]
			);

			let Some(weights) = archive.tensor("weights") else {
				panic!("weights tensor was not indexed");
			};
			assert_eq!(weights.dimensions(), &[2]);
			assert_eq!(weights.tensor_type(), GgufTensorType::F32);
			assert_eq!(weights.data_offset(), 0);
			assert_eq!(weights.file_offset(), host_u64(sample.data_start));
			assert_eq!(weights.encoded_bytes(), 8);
			assert_eq!(weights.data_end(), 8);
			let Some(raw) = archive.raw_tensor("weights") else {
				panic!("weights tensor had no raw span");
			};
			assert_eq!(raw, &[0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]);
			assert_eq!(raw.as_ptr(), sample.bytes[sample.data_start..].as_ptr());

			let Some(quant) = archive.tensor("quant") else {
				panic!("quant tensor was not indexed");
			};
			assert_eq!(quant.dimensions(), &[32]);
			assert_eq!(quant.tensor_type(), GgufTensorType::Q4_0);
			assert_eq!(quant.data_offset(), 32);
			assert_eq!(quant.encoded_bytes(), 18);
			assert_eq!(archive.raw_tensor("missing"), None);
		}

		let version_two = fixture(2, GgufEndian::Little);
		let archive = parse_valid(&version_two.bytes, generous_limits(version_two.bytes.len()));
		assert_eq!(archive.version(), 2);
		assert_eq!(archive.endian(), GgufEndian::Little);
	}

	#[test]
	fn rejects_version_endian_and_scalar_encoding_errors() {
		let big_v2 = fixture(2, GgufEndian::Big);
		assert_eq!(
			error_kind(&big_v2.bytes, generous_limits(big_v2.bytes.len())),
			GgufErrorKind::UnsupportedEndian
		);

		let sample = fixture(3, GgufEndian::Little);
		let mut invalid_magic = sample.bytes.clone();
		invalid_magic[0] = b'X';
		assert_eq!(
			error_kind(&invalid_magic, generous_limits(invalid_magic.len())),
			GgufErrorKind::InvalidMagic
		);

		let mut invalid_version = sample.bytes.clone();
		patch_u32(&mut invalid_version, GgufEndian::Little, 4, 4);
		assert_eq!(
			error_kind(&invalid_version, generous_limits(invalid_version.len())),
			GgufErrorKind::UnsupportedVersion
		);

		let mut invalid_boolean = sample.bytes.clone();
		invalid_boolean[sample.boolean_value] = 2;
		assert_eq!(
			error_kind(&invalid_boolean, generous_limits(invalid_boolean.len())),
			GgufErrorKind::InvalidBoolean
		);

		let mut unknown_metadata_type = sample.bytes.clone();
		patch_u32(
			&mut unknown_metadata_type,
			GgufEndian::Little,
			sample.float_type,
			13,
		);
		assert_eq!(
			error_kind(
				&unknown_metadata_type,
				generous_limits(unknown_metadata_type.len())
			),
			GgufErrorKind::UnsupportedMetadataType
		);

		let mut removed_tensor_type = sample.bytes.clone();
		patch_u32(
			&mut removed_tensor_type,
			GgufEndian::Little,
			sample.quant_type,
			31,
		);
		assert_eq!(
			error_kind(
				&removed_tensor_type,
				generous_limits(removed_tensor_type.len())
			),
			GgufErrorKind::UnsupportedTensorType
		);
	}

	#[test]
	fn enforces_caller_bounds_before_unbounded_work() {
		let sample = fixture(3, GgufEndian::Little);
		let byte_count = host_u64(sample.bytes.len());
		let declared_limits = generous_limits(sample.bytes.len());
		assert_eq!(declared_limits.file_bytes().get(), byte_count);
		assert_eq!(declared_limits.metadata_pairs().get(), 64);
		assert_eq!(declared_limits.tensors().get(), 64);
		assert_eq!(declared_limits.rank().get(), 8);
		assert_eq!(declared_limits.string_bytes().get(), 16_384);
		assert_eq!(declared_limits.array_elements().get(), 16_384);
		assert_eq!(declared_limits.array_depth().get(), 8);
		assert_eq!(
			error_kind(
				&sample.bytes,
				limits(byte_count - 1, 64, 64, 8, 16_384, 16_384, 8)
			),
			GgufErrorKind::FileLimitExceeded
		);
		assert_eq!(
			error_kind(
				&sample.bytes,
				limits(byte_count, 5, 64, 8, 16_384, 16_384, 8)
			),
			GgufErrorKind::MetadataLimitExceeded
		);
		assert_eq!(
			error_kind(
				&sample.bytes,
				limits(byte_count, 64, 1, 8, 16_384, 16_384, 8)
			),
			GgufErrorKind::TensorLimitExceeded
		);
		assert_eq!(
			error_kind(&sample.bytes, limits(byte_count, 64, 64, 8, 16, 16_384, 8)),
			GgufErrorKind::StringLimitExceeded
		);
		assert_eq!(
			error_kind(&sample.bytes, limits(byte_count, 64, 64, 8, 16_384, 2, 8)),
			GgufErrorKind::ArrayLimitExceeded
		);
		assert_eq!(
			error_kind(
				&sample.bytes,
				limits(byte_count, 64, 64, 8, 16_384, 16_384, 1)
			),
			GgufErrorKind::ArrayDepthExceeded
		);

		let rank_two = rank_two_fixture();
		assert_eq!(
			error_kind(
				&rank_two,
				limits(host_u64(rank_two.len()), 4, 4, 1, 256, 16, 4)
			),
			GgufErrorKind::RankLimitExceeded
		);

		let mut impossible_count = sample.bytes.clone();
		patch_u64(&mut impossible_count, GgufEndian::Little, 8, 10_000);
		assert_eq!(
			error_kind(
				&impossible_count,
				limits(
					host_u64(impossible_count.len()),
					64,
					10_000,
					8,
					16_384,
					16_384,
					8
				)
			),
			GgufErrorKind::Truncated
		);

		let zero_limit = GgufLimits::new(0, 1, 1, 1, 1, 1, 1);
		let Err(zero_error) = zero_limit else {
			panic!("zero file limit was accepted");
		};
		assert_eq!(zero_error.kind(), GgufErrorKind::InvalidLimit);
		assert!(zero_error.detail().contains("nonzero"));

		let excessive_depth = GgufLimits::new(1, 1, 1, 1, 1, 1, ARRAY_DEPTH_HARD_MAX + 1);
		let Err(depth_error) = excessive_depth else {
			panic!("excessive recursion limit was accepted");
		};
		assert_eq!(depth_error.kind(), GgufErrorKind::InvalidLimit);
	}

	#[test]
	fn rejects_duplicate_and_invalid_names() {
		let duplicate_metadata = duplicate_metadata_fixture();
		assert_eq!(
			error_kind(
				&duplicate_metadata,
				generous_limits(duplicate_metadata.len())
			),
			GgufErrorKind::DuplicateMetadata
		);

		let duplicate_tensor = duplicate_tensor_fixture();
		assert_eq!(
			error_kind(&duplicate_tensor, generous_limits(duplicate_tensor.len())),
			GgufErrorKind::DuplicateTensor
		);

		for key in ["General.name", "bad..key", "bad-key"] {
			let invalid = invalid_key_fixture(key);
			assert_eq!(
				error_kind(&invalid, generous_limits(invalid.len())),
				GgufErrorKind::InvalidMetadataKey
			);
		}

		let long_name = "n".repeat(65);
		let invalid_name = invalid_tensor_name_fixture(&long_name);
		assert_eq!(
			error_kind(&invalid_name, generous_limits(invalid_name.len())),
			GgufErrorKind::StringLimitExceeded
		);
	}

	#[test]
	fn rejects_invalid_dimensions_offsets_and_spans() {
		let sample = fixture(3, GgufEndian::Little);

		let mut zero_dimension = sample.bytes.clone();
		patch_u64(
			&mut zero_dimension,
			GgufEndian::Little,
			sample.first_dimension,
			0,
		);
		assert_eq!(
			error_kind(&zero_dimension, generous_limits(zero_dimension.len())),
			GgufErrorKind::InvalidDimension
		);

		let mut block_mismatch = sample.bytes.clone();
		patch_u64(
			&mut block_mismatch,
			GgufEndian::Little,
			sample.quant_dimension,
			31,
		);
		assert_eq!(
			error_kind(&block_mismatch, generous_limits(block_mismatch.len())),
			GgufErrorKind::InvalidDimension
		);

		let mut arithmetic_overflow = sample.bytes.clone();
		patch_u64(
			&mut arithmetic_overflow,
			GgufEndian::Little,
			sample.first_dimension,
			u64::MAX,
		);
		assert_eq!(
			error_kind(
				&arithmetic_overflow,
				generous_limits(arithmetic_overflow.len())
			),
			GgufErrorKind::ArithmeticOverflow
		);

		let mut misaligned = sample.bytes.clone();
		patch_u64(&mut misaligned, GgufEndian::Little, sample.quant_offset, 16);
		assert_eq!(
			error_kind(&misaligned, generous_limits(misaligned.len())),
			GgufErrorKind::InvalidOffset
		);

		let mut overlap = sample.bytes.clone();
		patch_u64(&mut overlap, GgufEndian::Little, sample.quant_offset, 0);
		assert_eq!(
			error_kind(&overlap, generous_limits(overlap.len())),
			GgufErrorKind::OverlappingTensor
		);

		let mut out_of_range = sample.bytes.clone();
		patch_u64(
			&mut out_of_range,
			GgufEndian::Little,
			sample.quant_offset,
			64,
		);
		assert_eq!(
			error_kind(&out_of_range, generous_limits(out_of_range.len())),
			GgufErrorKind::InvalidOffset
		);
	}

	#[test]
	fn rejects_padding_trailing_bytes_and_every_truncated_prefix() {
		let sample = fixture(3, GgufEndian::Little);
		assert!(sample.header_end < sample.data_start);

		let mut header_padding = sample.bytes.clone();
		header_padding[sample.header_end] = 1;
		assert_eq!(
			error_kind(&header_padding, generous_limits(header_padding.len())),
			GgufErrorKind::NonZeroPadding
		);

		let mut tensor_padding = sample.bytes.clone();
		tensor_padding[sample.data_start + 8] = 1;
		assert_eq!(
			error_kind(&tensor_padding, generous_limits(tensor_padding.len())),
			GgufErrorKind::NonZeroPadding
		);

		let mut trailing = sample.bytes.clone();
		trailing.push(0);
		assert_eq!(
			error_kind(&trailing, generous_limits(trailing.len())),
			GgufErrorKind::TrailingData
		);

		for prefix_length in 0..sample.bytes.len() {
			let prefix = &sample.bytes[..prefix_length];
			assert!(
				parse_gguf(prefix, generous_limits(sample.bytes.len())).is_err(),
				"truncated prefix of {prefix_length} bytes parsed successfully"
			);
		}
	}
}
