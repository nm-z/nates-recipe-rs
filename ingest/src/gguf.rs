use core::{
	fmt,
	num::{NonZeroU32, NonZeroU64},
	str,
};
use std::collections::BTreeSet;

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const DEFAULT_ALIGNMENT: u32 = 32;
const METADATA_KEY_BYTES_MAX: u64 = 65_535;
const TENSOR_NAME_BYTES_MAX: u64 = 64;
const CURRENT_TENSOR_RANK_MAX: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GgufEndian {
	Little,
	Big,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GgufLimits {
	file_bytes: NonZeroU64,
	metadata_pairs: NonZeroU64,
	tensors: NonZeroU64,
	rank: NonZeroU32,
	string_bytes: NonZeroU64,
	array_elements: NonZeroU64,
	array_depth: NonZeroU64,
}

impl GgufLimits {
	/// Construct nonzero aggregate parser bounds.
	///
	/// # Errors
	///
	/// Returns [`GgufErrorKind::InvalidLimit`] for zero limits.
	pub fn new(file_bytes: u64, metadata_pairs: u64, tensors: u64, rank: u32, string_bytes: u64, array_elements: u64, array_depth: u64) -> GgufResult<Self> {
		Ok(Self {
			file_bytes: nonzero_u64("file byte", file_bytes)?,
			metadata_pairs: nonzero_u64("metadata pair", metadata_pairs)?,
			tensors: nonzero_u64("tensor", tensors)?,
			rank: nonzero_u32("rank", rank)?,
			string_bytes: nonzero_u64("aggregate string byte", string_bytes)?,
			array_elements: nonzero_u64("aggregate array element", array_elements)?,
			array_depth: nonzero_u64("array depth", array_depth)?,
		})
	}

	#[must_use]
	pub const fn file_bytes(self) -> NonZeroU64 { self.file_bytes }

	#[must_use]
	pub const fn metadata_pairs(self) -> NonZeroU64 { self.metadata_pairs }

	#[must_use]
	pub const fn tensors(self) -> NonZeroU64 { self.tensors }

	#[must_use]
	pub const fn rank(self) -> NonZeroU32 { self.rank }

	#[must_use]
	pub const fn string_bytes(self) -> NonZeroU64 { self.string_bytes }

	#[must_use]
	pub const fn array_elements(self) -> NonZeroU64 { self.array_elements }

	#[must_use]
	pub const fn array_depth(self) -> NonZeroU64 { self.array_depth }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GgufMetadataType {
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
	pub(crate) fn parse(code: u32) -> GgufResult<Self> {
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
			other => {
				Err(GgufError::new(
					GgufErrorKind::UnsupportedMetadataType,
					format!("GGUF metadata type code {other} is unsupported"),
				))
			}
		}
	}

	#[must_use]
	pub const fn code(self) -> u32 {
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
pub struct GgufMetadataArray<'a> {
	element_type: GgufMetadataType,
	values: Vec<GgufMetadataValue<'a>>,
}

impl Drop for GgufMetadataArray<'_> {
	fn drop(&mut self) {
		let mut pending = core::mem::take(&mut self.values);
		while let Some(mut value) = pending.pop() {
			if let GgufMetadataValue::Array(array) = &mut value {
				pending.extend(core::mem::take(&mut array.values));
			}
		}
	}
}

impl<'a> GgufMetadataArray<'a> {
	#[must_use]
	pub const fn element_type(&self) -> GgufMetadataType { self.element_type }

	#[must_use]
	pub fn values(&self) -> &[GgufMetadataValue<'a>] { &self.values }
}

/// Metadata payload that retains the encoded scalar type and floating bits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GgufMetadataValue<'a> {
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
	pub const fn value_type(&self) -> GgufMetadataType {
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
pub struct GgufMetadataEntry<'a> {
	key: &'a str,
	value: GgufMetadataValue<'a>,
}

impl<'a> GgufMetadataEntry<'a> {
	#[must_use]
	pub const fn key(&self) -> &'a str { self.key }

	#[must_use]
	pub const fn value(&self) -> &GgufMetadataValue<'a> { &self.value }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GgufTensorType {
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
	Nvfp4,
	Q1_0,
	Q2_0,
}

impl GgufTensorType {
	pub(crate) fn parse(code: u32) -> GgufResult<Self> {
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
			40 => Ok(Self::Nvfp4),
			41 => Ok(Self::Q1_0),
			42 => Ok(Self::Q2_0),
			other => {
				Err(GgufError::new(
					GgufErrorKind::UnsupportedTensorType,
					format!("GGML tensor type code {other} is unsupported or removed"),
				))
			}
		}
	}

	#[must_use]
	pub const fn code(self) -> u32 {
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
			Self::Nvfp4 => 40,
			Self::Q1_0 => 41,
			Self::Q2_0 => 42,
		}
	}

	#[must_use]
	pub const fn block_elements(self) -> u64 {
		match self {
			Self::F32 | Self::F16 | Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::F64 | Self::Bf16 => 1,
			Self::Q4_0 | Self::Q4_1 | Self::Q5_0 | Self::Q5_1 | Self::Q8_0 | Self::Q8_1 | Self::Iq4Nl | Self::Mxfp4 => 32,
			Self::Nvfp4 | Self::Q2_0 => 64,
			Self::Q1_0 => 128,
			Self::Q2K | Self::Q3K | Self::Q4K | Self::Q5K | Self::Q6K | Self::Q8K | Self::Iq2Xxs | Self::Iq2Xs | Self::Iq3Xxs | Self::Iq1S | Self::Iq3S | Self::Iq2S | Self::Iq4Xs | Self::Iq1M | Self::Tq1_0 | Self::Tq2_0 => 256,
		}
	}

	#[must_use]
	pub const fn block_bytes(self) -> u64 {
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
			Self::Nvfp4 => 36,
			Self::Q1_0 => 18,
			Self::Q2_0 => 18,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GgufTensor<'a> {
	name: &'a str,
	dimensions: Vec<u64>,
	tensor_type: GgufTensorType,
	data_offset: u64,
	file_offset: u64,
	encoded_bytes: u64,
}

impl<'a> GgufTensor<'a> {
	#[must_use]
	pub const fn name(&self) -> &'a str { self.name }

	#[must_use]
	pub fn dimensions(&self) -> &[u64] { &self.dimensions }

	#[must_use]
	pub const fn tensor_type(&self) -> GgufTensorType { self.tensor_type }

	#[must_use]
	pub const fn data_offset(&self) -> u64 { self.data_offset }

	#[must_use]
	pub const fn file_offset(&self) -> u64 { self.file_offset }

	#[must_use]
	pub const fn encoded_bytes(&self) -> u64 { self.encoded_bytes }

	#[must_use]
	pub const fn data_end(&self) -> u64 { self.data_offset + self.encoded_bytes }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GgufArchive<'a> {
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
	pub const fn version(&self) -> u32 { self.version }

	#[must_use]
	pub const fn endian(&self) -> GgufEndian { self.endian }

	#[must_use]
	pub const fn alignment(&self) -> u32 { self.alignment }

	#[must_use]
	pub const fn data_start(&self) -> u64 { self.data_start }

	#[must_use]
	pub fn data(&self) -> &'a [u8] { self.data }

	#[must_use]
	pub fn metadata(&self) -> &[GgufMetadataEntry<'a>] { &self.metadata }

	#[must_use]
	pub fn metadata_entry(&self, key: &str) -> Option<&GgufMetadataEntry<'a>> { self.metadata.iter().find(|entry| entry.key == key) }

	#[must_use]
	pub fn tensors(&self) -> &[GgufTensor<'a>] { &self.tensors }

	#[must_use]
	pub fn tensor(&self, name: &str) -> Option<&GgufTensor<'a>> { self.tensors.iter().find(|tensor| tensor.name == name) }

	#[must_use]
	pub fn raw_tensor(&self, name: &str) -> Option<&'a [u8]> {
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
pub enum GgufErrorKind {
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
pub struct GgufError {
	pub kind: GgufErrorKind,
	pub detail: String,
}

impl GgufError {
	fn new(kind: GgufErrorKind, detail: impl ToString) -> Self {
		Self {
			kind,
			detail: detail.to_string(),
		}
	}

	#[must_use]
	pub const fn kind(&self) -> GgufErrorKind { self.kind }

	#[must_use]
	pub fn detail(&self) -> &str { &self.detail }
}

impl fmt::Display for GgufError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { write!(formatter, "{:?}: {}", self.kind, self.detail) }
}

impl std::error::Error for GgufError {}

pub type GgufResult<T> = Result<T, GgufError>;

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
	array_depth_limit: u64,
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

	const fn position(&self) -> usize { self.position }

	const fn remaining(&self) -> usize { self.bytes.len() - self.position }

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

	fn read_u8(&mut self, context: &str) -> GgufResult<u8> { Ok(self.take(1, context)?[0]) }

	fn read_i8(&mut self, context: &str) -> GgufResult<i8> { Ok(i8::from_ne_bytes([self.read_u8(context)?])) }

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

	fn read_metadata_value(&mut self, value_type: GgufMetadataType) -> GgufResult<GgufMetadataValue<'a>> {
		#[derive(Debug)]
		struct ArrayFrame<'a> {
			element_type: GgufMetadataType,
			remaining: u64,
			values: Vec<GgufMetadataValue<'a>>,
		}

		let mut frames = Vec::<ArrayFrame<'a>>::new();
		let mut next_type = value_type;
		loop {
			let mut completed = if next_type == GgufMetadataType::Array {
				let nested_depth = u64::try_from(frames.len())
					.map_err(|error| {
						GgufError::new(
							GgufErrorKind::ArithmeticOverflow,
							format!("metadata array depth cannot be represented by u64: {error}"),
						)
					})?
					.checked_add(1)
					.ok_or_else(|| {
						GgufError::new(
							GgufErrorKind::ArithmeticOverflow,
							"GGUF metadata array depth overflowed u64",
						)
					})?;
				if nested_depth > self.array_depth_limit {
					return Err(GgufError::new(
						GgufErrorKind::ArrayDepthExceeded,
						format!(
							"metadata array depth {nested_depth} exceeds limit {}",
							self.array_depth_limit
						),
					));
				}
				let element_type = GgufMetadataType::parse(self.read_u32("metadata array element type")?)?;
				let length = self.read_u64("metadata array length")?;
				if length > self.array_elements_remaining {
					return Err(GgufError::new(
						GgufErrorKind::ArrayLimitExceeded,
						format!(
							"metadata array has {length} elements, aggregate remaining budget is {}",
							self.array_elements_remaining
						),
					));
				}
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
				let values = Vec::with_capacity(bounded_usize(length, "metadata array length")?);
				if length == 0 {
					GgufMetadataValue::Array(GgufMetadataArray {
						element_type,
						values,
					})
				} else {
					frames.push(ArrayFrame {
						element_type,
						remaining: length,
						values,
					});
					next_type = element_type;
					continue;
				}
			} else {
				self.read_metadata_scalar(next_type)?
			};

			loop {
				let Some(frame) = frames.last_mut() else {
					return Ok(completed);
				};
				frame.values.push(completed);
				frame.remaining -= 1;
				if frame.remaining > 0 {
					next_type = frame.element_type;
					break;
				}
				let frame = frames.pop().expect("completed metadata array frame exists");
				completed = GgufMetadataValue::Array(GgufMetadataArray {
					element_type: frame.element_type,
					values: frame.values,
				});
			}
		}
	}

	fn read_metadata_scalar(&mut self, value_type: GgufMetadataType) -> GgufResult<GgufMetadataValue<'a>> {
		match value_type {
			GgufMetadataType::U8 => Ok(GgufMetadataValue::U8(self.read_u8("uint8 metadata value")?)),
			GgufMetadataType::I8 => Ok(GgufMetadataValue::I8(self.read_i8("int8 metadata value")?)),
			GgufMetadataType::U16 => {
				Ok(GgufMetadataValue::U16(
					self.read_u16("uint16 metadata value")?,
				))
			}
			GgufMetadataType::I16 => {
				Ok(GgufMetadataValue::I16(
					self.read_i16("int16 metadata value")?,
				))
			}
			GgufMetadataType::U32 => {
				Ok(GgufMetadataValue::U32(
					self.read_u32("uint32 metadata value")?,
				))
			}
			GgufMetadataType::I32 => {
				Ok(GgufMetadataValue::I32(
					self.read_i32("int32 metadata value")?,
				))
			}
			GgufMetadataType::F32 => {
				Ok(GgufMetadataValue::F32Bits(
					self.read_u32("float32 metadata bits")?,
				))
			}
			GgufMetadataType::Bool => {
				match self.read_u8("boolean metadata value")? {
					0 => Ok(GgufMetadataValue::Bool(false)),
					1 => Ok(GgufMetadataValue::Bool(true)),
					other => {
						Err(GgufError::new(
							GgufErrorKind::InvalidBoolean,
							format!("GGUF boolean byte is {other}, expected zero or one"),
						))
					}
				}
			}
			GgufMetadataType::String => {
				Ok(GgufMetadataValue::String(
					self.read_string("metadata string", u64::MAX)?,
				))
			}
			GgufMetadataType::U64 => {
				Ok(GgufMetadataValue::U64(
					self.read_u64("uint64 metadata value")?,
				))
			}
			GgufMetadataType::I64 => {
				Ok(GgufMetadataValue::I64(
					self.read_i64("int64 metadata value")?,
				))
			}
			GgufMetadataType::F64 => {
				Ok(GgufMetadataValue::F64Bits(
					self.read_u64("float64 metadata bits")?,
				))
			}
			GgufMetadataType::Array => {
				Err(GgufError::new(
					GgufErrorKind::UnsupportedMetadataType,
					"array metadata reached the scalar decoder",
				))
			}
		}
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
pub fn parse_gguf(bytes: &[u8], limits: GgufLimits) -> GgufResult<GgufArchive<'_>> {
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
		let value = reader.read_metadata_value(value_type)?;
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
		match (rank <= CURRENT_TENSOR_RANK_MAX, rank <= limits.rank.get()) {
			(true, true) => Ok(()),
			(false, _) => {
				return Err(GgufError::new(
					GgufErrorKind::InvalidDimension,
					format!("tensor {name:?} rank {rank} exceeds current GGUF v3 maximum {CURRENT_TENSOR_RANK_MAX}"),
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
		for _axis in 0..rank {
			dimensions.push(reader.read_u64("tensor dimension")?);
		}
		let tensor_type = GgufTensorType::parse(reader.read_u32("tensor type")?)?;
		let data_offset = reader.read_u64("tensor data offset")?;
		match data_offset % alignment_u64 {
			0 => Ok(()),
			remainder => {
				return Err(GgufError::new(
					GgufErrorKind::InvalidOffset,
					format!("tensor {name:?} offset {data_offset} has remainder {remainder} under alignment {alignment}"),
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
	let aligned_header_end = align_up(header_end, alignment_u64)?;
	let file_bytes = u64::try_from(bytes.len()).map_err(|error| {
		GgufError::new(
			GgufErrorKind::ArithmeticOverflow,
			format!("GGUF image length cannot be represented by u64: {error}"),
		)
	})?;
	let data_start = match (tensor_count, file_bytes) {
		(0, length) if length == header_end || length == aligned_header_end => length,
		(0, length) => {
			return Err(GgufError::new(
				GgufErrorKind::TrailingData,
				format!("tensor-free GGUF ends at {length}; expected unpadded header end {header_end} or padded end {aligned_header_end}"),
			));
		}
		(_, _) => aligned_header_end,
	};
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

fn materialize_tensors<'a>(raw_tensors: Vec<RawTensor<'a>>, data_start: u64, data: &'a [u8], alignment: u64) -> GgufResult<Vec<GgufTensor<'a>>> {
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
	let first_dimension = dimensions.first().copied().unwrap_or(1);
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
	let elements = if dimensions.contains(&0) {
		0
	} else {
		dimensions.iter().try_fold(1_u64, |product, dimension| {
			product.checked_mul(*dimension).ok_or_else(|| {
				GgufError::new(
					GgufErrorKind::ArithmeticOverflow,
					format!("tensor {name:?} dimension product overflowed u64"),
				)
			})
		})?
	};
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
	let padded_end = align_up(cursor, alignment)?;
	if data_bytes != cursor && data_bytes != padded_end {
		return Err(GgufError::new(
			GgufErrorKind::TrailingData,
			format!("validated tensors end at {cursor}; tensor data has {data_bytes} bytes, expected the exact end or aligned end {padded_end}"),
		));
	}
	let padding_start = bounded_usize(cursor, "terminal tensor padding start")?;
	let padding_end = bounded_usize(data_bytes, "terminal tensor padding end")?;
	let padding = data.get(padding_start..padding_end).ok_or_else(|| {
		GgufError::new(
			GgufErrorKind::InvalidOffset,
			"terminal tensor padding lies outside tensor data",
		)
	})?;
	require_zero_padding(padding, "terminal tensor padding")
}

fn metadata_alignment(metadata: &[GgufMetadataEntry<'_>], _limits: GgufLimits) -> GgufResult<u32> {
	let entry = metadata
		.iter()
		.find(|entry| entry.key == "general.alignment");
	let alignment = match entry {
		None => DEFAULT_ALIGNMENT,
		Some(found) => {
			match &found.value {
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
			}
		}
	};
	match (alignment > 0, alignment % 8 == 0) {
		(true, true) => Ok(alignment),
		(false, _) => {
			Err(GgufError::new(
				GgufErrorKind::InvalidAlignment,
				"general.alignment must be nonzero",
			))
		}
		(true, false) => {
			Err(GgufError::new(
				GgufErrorKind::InvalidAlignment,
				format!("general.alignment {alignment} is not a multiple of eight"),
			))
		}
	}
}

fn validate_metadata_key(key: &str) -> GgufResult<()> {
	match key.len() {
		0 => {
			Err(GgufError::new(
				GgufErrorKind::InvalidMetadataKey,
				"GGUF metadata key is empty",
			))
		}
		positive_length => {
			match positive_length <= bounded_usize(METADATA_KEY_BYTES_MAX, "metadata key maximum")? {
				true => Ok(()),
				false => {
					Err(GgufError::new(
						GgufErrorKind::InvalidMetadataKey,
						"GGUF metadata key exceeds the format maximum",
					))
				}
			}
		}
	}
}

fn validate_tensor_name(name: &str) -> GgufResult<()> {
	match name.len() <= bounded_usize(TENSOR_NAME_BYTES_MAX, "tensor name maximum")? {
		true => Ok(()),
		false => {
			Err(GgufError::new(
				GgufErrorKind::InvalidTensorName,
				format!("tensor name {name:?} exceeds the 64-byte format maximum"),
			))
		}
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
				2 => {
					Err(GgufError::new(
						GgufErrorKind::UnsupportedEndian,
						"big-endian encoding requires GGUF version 3",
					))
				}
				other_big => {
					Err(GgufError::new(
						GgufErrorKind::UnsupportedVersion,
						format!("GGUF version bytes decode to little-endian {other_little} and big-endian {other_big}"),
					))
				}
			}
		}
	}
}

fn require_count(actual: u64, limit: u64, kind: GgufErrorKind, name: &str) -> GgufResult<()> {
	match actual <= limit {
		true => Ok(()),
		false => {
			Err(GgufError::new(
				kind,
				format!("GGUF declares {actual} {name}s, limit is {limit}"),
			))
		}
	}
}

fn require_zero_padding(bytes: &[u8], context: &str) -> GgufResult<()> {
	let all_zero = bytes.iter().all(|byte| *byte == 0);
	match all_zero {
		true => Ok(()),
		false => {
			Err(GgufError::new(
				GgufErrorKind::NonZeroPadding,
				format!("{context} contains nonzero bytes"),
			))
		}
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
		false => {
			Err(GgufError::new(
				GgufErrorKind::Truncated,
				format!("{context} require at least {required} bytes, only {available} remain"),
			))
		}
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
