use core::fmt;
use std::{
	collections::BTreeSet,
	fmt::Write as _,
	io::{BufRead, Read, Seek, SeekFrom, Write as IoWrite},
};

use crate::{GgufEndian, GgufError, GgufLimits, GgufMetadataType, GgufTensorType};

const STRUCTURAL_SCHEMA: &str = "recipe-gguf-structural-v1";
const SCALAR_CHUNK_VALUES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GgufOgdlErrorKind {
	Gguf,
	Io,
	InvalidUtf8,
	InvalidSyntax,
	InvalidStructure,
	InvalidValue,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GgufOgdlError {
	kind: GgufOgdlErrorKind,
	path: String,
	detail: String,
}

impl GgufOgdlError {
	fn new(kind: GgufOgdlErrorKind, path: impl Into<String>, detail: impl Into<String>) -> Self {
		Self {
			kind,
			path: path.into(),
			detail: detail.into(),
		}
	}

	#[must_use]
	pub const fn kind(&self) -> GgufOgdlErrorKind { self.kind }

	#[must_use]
	pub fn path(&self) -> &str { &self.path }

	#[must_use]
	pub fn detail(&self) -> &str { &self.detail }
}

impl fmt::Display for GgufOgdlError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { write!(formatter, "{}: {}", self.path, self.detail) }
}

impl std::error::Error for GgufOgdlError {}

impl From<GgufError> for GgufOgdlError {
	fn from(error: GgufError) -> Self { Self::new(GgufOgdlErrorKind::Gguf, "<gguf>", error.to_string()) }
}

pub type GgufOgdlResult<T> = Result<T, GgufOgdlError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FloatFormat {
	F16,
	Bf16,
	F32,
	F64,
}

impl FloatFormat {
	const fn name(self) -> &'static str {
		match self {
			Self::F16 => "f16",
			Self::Bf16 => "bf16",
			Self::F32 => "f32",
			Self::F64 => "f64",
		}
	}

	const fn widths(self) -> (u32, u32, u32) {
		match self {
			Self::F16 => (16, 5, 10),
			Self::Bf16 => (16, 8, 7),
			Self::F32 => (32, 8, 23),
			Self::F64 => (64, 11, 52),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FloatParts {
	sign: u64,
	exponent: u64,
	fraction: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FieldValues {
	Unsigned {
		kind: &'static str,
		maximum: u64,
		values: Vec<u64>,
	},
	Signed {
		kind: &'static str,
		minimum: i64,
		maximum: i64,
		values: Vec<i64>,
	},
	Float {
		format: FloatFormat,
		values: Vec<FloatParts>,
	},
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StructuralField {
	name: &'static str,
	values: FieldValues,
}

const STREAM_DEFAULT_ALIGNMENT: u32 = 32;
const STREAM_METADATA_KEY_BYTES_MAX: u64 = 65_535;
const STREAM_TENSOR_NAME_BYTES_MAX: u64 = 64;
const STREAM_ZERO_BUFFER_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StreamTensor {
	name: String,
	dimensions: Vec<u64>,
	tensor_type: GgufTensorType,
	offset: u64,
	encoded_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StreamGgufHeader {
	endian: GgufEndian,
	alignment: u32,
	metadata_count: u64,
	architecture_type: Option<GgufMetadataType>,
	architecture: Option<String>,
	tensors: Vec<StreamTensor>,
	data_start: u64,
}

#[derive(Debug)]
struct StreamArrayFrame {
	element_type: GgufMetadataType,
	remaining: u64,
	child_depth: usize,
}

#[derive(Debug)]
struct StreamMetadataScalar {
	text: String,
	u32_value: Option<u32>,
	string_value: Option<String>,
}

#[derive(Debug, Default)]
struct StreamMetadataRoot {
	u32_value: Option<u32>,
	string_value: Option<String>,
}

#[derive(Debug)]
struct StreamBinaryReader<'a, R> {
	input: &'a mut R,
	endian: GgufEndian,
	position: u64,
	file_bytes: u64,
	string_bytes_remaining: u64,
	array_elements_remaining: u64,
	array_depth_limit: u64,
}

impl<'a, R: Read> StreamBinaryReader<'a, R> {
	fn new(input: &'a mut R, endian: GgufEndian, position: u64, file_bytes: u64, limits: GgufLimits) -> Self {
		Self {
			input,
			endian,
			position,
			file_bytes,
			string_bytes_remaining: limits.string_bytes().get(),
			array_elements_remaining: limits.array_elements().get(),
			array_depth_limit: limits.array_depth().get(),
		}
	}

	const fn position(&self) -> u64 { self.position }

	fn read_exact(&mut self, bytes: &mut [u8], context: &str) -> GgufOgdlResult<()> {
		let length = host_u64(bytes.len(), context)?;
		let end = self
			.position
			.checked_add(length)
			.ok_or_else(|| invalid_overflow(context, "binary input position overflowed u64"))?;
		if end > self.file_bytes {
			return Err(invalid_structure(
				context,
				format!(
					"field ends at byte {end}, input has {} bytes",
					self.file_bytes
				),
			));
		}
		self.input
			.read_exact(bytes)
			.map_err(|error| stream_io_error(context, error))?;
		self.position = end;
		Ok(())
	}

	fn bytes(&mut self, length: u64, context: &str) -> GgufOgdlResult<Vec<u8>> {
		let mut bytes = vec![0_u8; host_usize(length, context)?];
		self.read_exact(&mut bytes, context)?;
		Ok(bytes)
	}

	fn u8(&mut self, context: &str) -> GgufOgdlResult<u8> {
		let mut bytes = [0_u8; 1];
		self.read_exact(&mut bytes, context)?;
		Ok(bytes[0])
	}

	fn i8(&mut self, context: &str) -> GgufOgdlResult<i8> { Ok(i8::from_ne_bytes([self.u8(context)?])) }

	fn u16(&mut self, context: &str) -> GgufOgdlResult<u16> {
		let mut bytes = [0_u8; 2];
		self.read_exact(&mut bytes, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u16::from_le_bytes(bytes),
			GgufEndian::Big => u16::from_be_bytes(bytes),
		})
	}

	fn i16(&mut self, context: &str) -> GgufOgdlResult<i16> {
		let mut bytes = [0_u8; 2];
		self.read_exact(&mut bytes, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i16::from_le_bytes(bytes),
			GgufEndian::Big => i16::from_be_bytes(bytes),
		})
	}

	fn u32(&mut self, context: &str) -> GgufOgdlResult<u32> {
		let mut bytes = [0_u8; 4];
		self.read_exact(&mut bytes, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u32::from_le_bytes(bytes),
			GgufEndian::Big => u32::from_be_bytes(bytes),
		})
	}

	fn i32(&mut self, context: &str) -> GgufOgdlResult<i32> {
		let mut bytes = [0_u8; 4];
		self.read_exact(&mut bytes, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i32::from_le_bytes(bytes),
			GgufEndian::Big => i32::from_be_bytes(bytes),
		})
	}

	fn u64(&mut self, context: &str) -> GgufOgdlResult<u64> {
		let mut bytes = [0_u8; 8];
		self.read_exact(&mut bytes, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u64::from_le_bytes(bytes),
			GgufEndian::Big => u64::from_be_bytes(bytes),
		})
	}

	fn i64(&mut self, context: &str) -> GgufOgdlResult<i64> {
		let mut bytes = [0_u8; 8];
		self.read_exact(&mut bytes, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i64::from_le_bytes(bytes),
			GgufEndian::Big => i64::from_be_bytes(bytes),
		})
	}

	fn string(&mut self, context: &str, per_value_limit: u64) -> GgufOgdlResult<String> {
		let length = self.u64(&format!("{context}.length"))?;
		if length > per_value_limit {
			return Err(invalid_value(
				context,
				format!("string has {length} bytes, per-value limit is {per_value_limit}"),
			));
		}
		if length > self.string_bytes_remaining {
			return Err(invalid_value(
				context,
				format!(
					"string consumes {length} bytes, aggregate remaining budget is {}",
					self.string_bytes_remaining
				),
			));
		}
		let bytes = self.bytes(length, context)?;
		let value = String::from_utf8(bytes).map_err(|error| {
			GgufOgdlError::new(
				GgufOgdlErrorKind::InvalidUtf8,
				context,
				format!("string is not valid UTF-8: {}", error.utf8_error()),
			)
		})?;
		self.string_bytes_remaining -= length;
		Ok(value)
	}

	fn metadata_value<F>(&mut self, value_type: GgufMetadataType, depth: usize, label: &'static str, mut emit: F) -> GgufOgdlResult<StreamMetadataRoot>
	where F: FnMut(usize, &str) -> GgufOgdlResult<()> {
		let mut frames = Vec::<StreamArrayFrame>::new();
		let mut next_type = value_type;
		let mut next_depth = depth;
		let mut next_label = label;
		let mut next_is_root = true;
		let mut root = StreamMetadataRoot::default();
		loop {
			if next_type == GgufMetadataType::Array {
				let nested_depth = host_u64(frames.len(), "metadata array depth")?
					.checked_add(1)
					.ok_or_else(|| invalid_overflow("metadata", "array depth overflowed u64"))?;
				if nested_depth > self.array_depth_limit {
					return Err(invalid_value(
						"metadata",
						format!(
							"array depth {nested_depth} exceeds configured limit {}",
							self.array_depth_limit
						),
					));
				}
				let element_type = GgufMetadataType::parse(self.u32("metadata.array.element_type")?)?;
				let length = self.u64("metadata.array.length")?;
				if length > self.array_elements_remaining {
					return Err(invalid_value(
						"metadata",
						format!(
							"array has {length} elements, aggregate remaining budget is {}",
							self.array_elements_remaining
						),
					));
				}
				self.array_elements_remaining -= length;
				emit(
					next_depth,
					&format!("{next_label} array {}", metadata_type_name(element_type)),
				)?;
				if next_is_root {
					root = StreamMetadataRoot::default();
				}
				if length > 0 {
					frames.push(StreamArrayFrame {
						element_type,
						remaining: length,
						child_depth: next_depth
							.checked_add(1)
							.ok_or_else(|| invalid_overflow("metadata", "OGDL indentation depth overflowed usize"))?,
					});
					next_type = element_type;
					next_depth = frames.last().expect("array frame was pushed").child_depth;
					next_label = "element";
					next_is_root = false;
					continue;
				}
			} else {
				let scalar = self.metadata_scalar(next_type, next_label)?;
				emit(next_depth, &scalar.text)?;
				if next_is_root {
					root.u32_value = scalar.u32_value;
					root.string_value = scalar.string_value;
				}
			}

			loop {
				let Some(frame) = frames.last_mut() else {
					return Ok(root);
				};
				frame.remaining -= 1;
				if frame.remaining > 0 {
					next_type = frame.element_type;
					next_depth = frame.child_depth;
					next_label = "element";
					next_is_root = false;
					break;
				}
				frames.pop();
			}
		}
	}

	fn metadata_scalar(&mut self, value_type: GgufMetadataType, label: &str) -> GgufOgdlResult<StreamMetadataScalar> {
		let (text, u32_value, string_value) = match value_type {
			GgufMetadataType::U8 => {
				(
					format!("{label} u8 {}", self.u8("metadata.u8")?),
					None,
					None,
				)
			}
			GgufMetadataType::I8 => {
				(
					format!("{label} i8 {}", self.i8("metadata.i8")?),
					None,
					None,
				)
			}
			GgufMetadataType::U16 => {
				(
					format!("{label} u16 {}", self.u16("metadata.u16")?),
					None,
					None,
				)
			}
			GgufMetadataType::I16 => {
				(
					format!("{label} i16 {}", self.i16("metadata.i16")?),
					None,
					None,
				)
			}
			GgufMetadataType::U32 => {
				let value = self.u32("metadata.u32")?;
				(format!("{label} u32 {value}"), Some(value), None)
			}
			GgufMetadataType::I32 => {
				(
					format!("{label} i32 {}", self.i32("metadata.i32")?),
					None,
					None,
				)
			}
			GgufMetadataType::F32 => {
				let parts = split_float(u64::from(self.u32("metadata.f32")?), FloatFormat::F32);
				(
					format!(
						"{label} f32 {} {} {}",
						parts.sign, parts.exponent, parts.fraction
					),
					None,
					None,
				)
			}
			GgufMetadataType::Bool => {
				let value = match self.u8("metadata.bool")? {
					0 => false,
					1 => true,
					other => {
						return Err(invalid_value(
							"metadata.bool",
							format!("boolean byte is {other}, expected 0 or 1"),
						));
					}
				};
				(format!("{label} bool {value}"), None, None)
			}
			GgufMetadataType::String => {
				let value = self.string("metadata.string", u64::MAX)?;
				(
					format!("{label} string {}", json_string(&value, "metadata.string")?),
					None,
					Some(value),
				)
			}
			GgufMetadataType::U64 => {
				(
					format!("{label} u64 {}", self.u64("metadata.u64")?),
					None,
					None,
				)
			}
			GgufMetadataType::I64 => {
				(
					format!("{label} i64 {}", self.i64("metadata.i64")?),
					None,
					None,
				)
			}
			GgufMetadataType::F64 => {
				let parts = split_float(self.u64("metadata.f64")?, FloatFormat::F64);
				(
					format!(
						"{label} f64 {} {} {}",
						parts.sign, parts.exponent, parts.fraction
					),
					None,
					None,
				)
			}
			GgufMetadataType::Array => {
				return Err(invalid_structure(
					"metadata",
					"array reached scalar stream decoder",
				));
			}
		};
		Ok(StreamMetadataScalar {
			text,
			u32_value,
			string_value,
		})
	}
}

/// Stream one current GGUF v3 file into structural OGDL without retaining its
/// tensor payload or expanded text in memory.
///
/// The input must be seekable because alignment metadata may appear anywhere
/// in the metadata sequence. Recipe validates the complete binary layout on a
/// first pass, then emits metadata and tensor blocks in source order.
pub fn gguf_to_structural_ogdl_stream<R, W>(input: &mut R, output: &mut W, limits: GgufLimits) -> GgufOgdlResult<()>
where
	R: Read + Seek,
	W: IoWrite,
{
	let file_bytes = input
		.seek(SeekFrom::End(0))
		.map_err(|error| stream_io_error("gguf", error))?;
	if file_bytes > limits.file_bytes().get() {
		return Err(invalid_value(
			"gguf",
			format!(
				"input has {file_bytes} bytes, configured limit is {}",
				limits.file_bytes()
			),
		));
	}
	input.seek(SeekFrom::Start(0))
		.map_err(|error| stream_io_error("gguf", error))?;
	let header = inspect_stream_gguf(input, file_bytes, limits)?;

	io_line(output, 0, "gguf")?;
	io_line(output, 1, &format!("schema {STRUCTURAL_SCHEMA}"))?;
	io_line(output, 1, "version 3")?;
	io_line(output, 1, match header.endian {
		GgufEndian::Little => "endian little",
		GgufEndian::Big => "endian big",
	})?;
	io_line(output, 1, &format!("alignment {}", header.alignment))?;
	io_line(output, 1, &format!("file_bytes {file_bytes}"))?;
	io_line(output, 1, "metadata")?;

	input.seek(SeekFrom::Start(24))
		.map_err(|error| stream_io_error("gguf.metadata", error))?;
	let mut reader = StreamBinaryReader::new(input, header.endian, 24, file_bytes, limits);
	for index in 0..header.metadata_count {
		let path = format!("gguf.metadata[{index}]");
		let key = reader.string(&format!("{path}.key"), STREAM_METADATA_KEY_BYTES_MAX)?;
		let value_type = GgufMetadataType::parse(reader.u32(&format!("{path}.type"))?)?;
		io_line(output, 2, "entry")?;
		io_line(
			output,
			3,
			&format!("key {}", json_string(&key, &format!("{path}.key"))?),
		)?;
		reader.metadata_value(value_type, 3, "value", |depth, text| {
			io_line(output, depth, text)
		})?;
	}
	io_line(output, 1, "tensors")?;
	for (index, tensor) in header.tensors.iter().enumerate() {
		let path = format!("gguf.tensors[{index}]");
		io_line(output, 2, "tensor")?;
		io_line(
			output,
			3,
			&format!(
				"name {}",
				json_string(&tensor.name, &format!("{path}.name"))?
			),
		)?;
		let mut dimensions = String::from("dimensions");
		for dimension in &tensor.dimensions {
			write!(dimensions, " {dimension}").expect("writing to String cannot fail");
		}
		io_line(output, 3, &dimensions)?;
		io_line(
			output,
			3,
			&format!("type {}", tensor_type_name(tensor.tensor_type)),
		)?;
		io_line(output, 3, &format!("offset {}", tensor.offset))?;
		io_line(output, 3, "payload")?;
		let file_offset = header
			.data_start
			.checked_add(tensor.offset)
			.ok_or_else(|| invalid_overflow(&path, "absolute tensor offset overflowed u64"))?;
		input.seek(SeekFrom::Start(file_offset))
			.map_err(|error| stream_io_error(&format!("{path}.payload"), error))?;
		write_tensor_payload_stream(input, output, tensor, header.endian, &path)?;
	}
	Ok(())
}

fn inspect_stream_gguf<R: Read + Seek>(input: &mut R, file_bytes: u64, limits: GgufLimits) -> GgufOgdlResult<StreamGgufHeader> {
	let mut prefix = [0_u8; 8];
	input.read_exact(&mut prefix)
		.map_err(|error| stream_io_error("gguf.header", error))?;
	if &prefix[..4] != b"GGUF" {
		return Err(invalid_value(
			"gguf.magic",
			format!("magic bytes are {:?}, expected GGUF", &prefix[..4]),
		));
	}
	let version_bytes: [u8; 4] = prefix[4..8].try_into().map_err(|error| {
		invalid_structure(
			"gguf.version",
			format!("version is not four bytes: {error}"),
		)
	})?;
	let endian = match (
		u32::from_le_bytes(version_bytes),
		u32::from_be_bytes(version_bytes),
	) {
		(3, _) => GgufEndian::Little,
		(_, 3) => GgufEndian::Big,
		(little, big) => {
			return Err(invalid_value(
				"gguf.version",
				format!("structural conversion requires GGUF v3; bytes decode to little={little}, big={big}"),
			));
		}
	};
	let mut reader = StreamBinaryReader::new(input, endian, 8, file_bytes, limits);
	let tensor_count = reader.u64("gguf.tensor_count")?;
	let metadata_count = reader.u64("gguf.metadata_count")?;
	if tensor_count > limits.tensors().get() {
		return Err(invalid_value(
			"gguf.tensor_count",
			format!(
				"count {tensor_count} exceeds configured limit {}",
				limits.tensors()
			),
		));
	}
	if metadata_count > limits.metadata_pairs().get() {
		return Err(invalid_value(
			"gguf.metadata_count",
			format!(
				"count {metadata_count} exceeds configured limit {}",
				limits.metadata_pairs()
			),
		));
	}

	let mut metadata_keys = BTreeSet::new();
	let mut alignment = None;
	let mut architecture_type = None;
	let mut architecture = None;
	for index in 0..metadata_count {
		let path = format!("gguf.metadata[{index}]");
		let key = reader.string(&format!("{path}.key"), STREAM_METADATA_KEY_BYTES_MAX)?;
		validate_stream_metadata_key(&key, &format!("{path}.key"))?;
		if !metadata_keys.insert(key.clone()) {
			return Err(invalid_value(
				&path,
				format!("duplicate metadata key {key:?}"),
			));
		}
		let value_type = GgufMetadataType::parse(reader.u32(&format!("{path}.type"))?)?;
		if key == "general.alignment" && value_type != GgufMetadataType::U32 {
			return Err(invalid_value(
				format!("{path}.value"),
				format!(
					"general.alignment must be u32, found {}",
					metadata_type_name(value_type)
				),
			));
		}
		let root = reader.metadata_value(value_type, 0, "value", |_, _| Ok(()))?;
		if key == "general.alignment" {
			alignment = root.u32_value;
		}
		if key == "general.architecture" {
			architecture_type = Some(value_type);
			architecture = root.string_value;
		}
	}
	let alignment = alignment.unwrap_or(STREAM_DEFAULT_ALIGNMENT);
	if alignment == 0 || alignment % 8 != 0 {
		return Err(invalid_value(
			"gguf.alignment",
			format!("alignment {alignment} is not a nonzero multiple of eight"),
		));
	}

	let tensor_capacity = host_usize(tensor_count, "gguf.tensor_count")?;
	let mut tensors = Vec::with_capacity(tensor_capacity);
	let mut tensor_names = BTreeSet::new();
	for index in 0..tensor_count {
		let path = format!("gguf.tensors[{index}]");
		let name = reader.string(&format!("{path}.name"), STREAM_TENSOR_NAME_BYTES_MAX)?;
		if !tensor_names.insert(name.clone()) {
			return Err(invalid_value(
				&path,
				format!("duplicate tensor name {name:?}"),
			));
		}
		let rank = reader.u32(&format!("{path}.rank"))?;
		if rank > 4 {
			return Err(invalid_value(
				format!("{path}.rank"),
				format!("current GGUF v3 permits at most four dimensions, found {rank}"),
			));
		}
		if rank > limits.rank().get() {
			return Err(invalid_value(
				format!("{path}.rank"),
				format!("rank {rank} exceeds configured limit {}", limits.rank()),
			));
		}
		let mut dimensions = Vec::with_capacity(host_usize(u64::from(rank), "tensor rank")?);
		for axis in 0..rank {
			dimensions.push(reader.u64(&format!("{path}.dimensions[{axis}]"))?);
		}
		let tensor_type = GgufTensorType::parse(reader.u32(&format!("{path}.type"))?)?;
		let offset = reader.u64(&format!("{path}.offset"))?;
		if offset % u64::from(alignment) != 0 {
			return Err(invalid_value(
				format!("{path}.offset"),
				format!("offset {offset} is not aligned to {alignment}"),
			));
		}
		let encoded_bytes = tensor_encoded_bytes(&name, &dimensions, tensor_type)?;
		tensors.push(StreamTensor {
			name,
			dimensions,
			tensor_type,
			offset,
			encoded_bytes,
		});
	}
	let header_end = reader.position();
	let aligned_header_end = align_up(header_end, u64::from(alignment))?;
	let data_start = if tensors.is_empty() {
		if file_bytes == header_end || file_bytes == aligned_header_end {
			file_bytes
		} else {
			return Err(invalid_value(
				"gguf.data",
				format!("tensor-free GGUF ends at {file_bytes}; expected unpadded header end {header_end} or padded end {aligned_header_end}"),
			));
		}
	} else {
		aligned_header_end
	};
	if data_start > file_bytes {
		return Err(invalid_structure(
			"gguf.data",
			format!("aligned tensor data starts at {data_start}, input has {file_bytes} bytes"),
		));
	}
	require_zero_stream_range(input, header_end, data_start, "gguf.header_padding")?;
	let data_bytes = file_bytes - data_start;
	let mut ordered = tensors.iter().collect::<Vec<_>>();
	ordered.sort_by_key(|tensor| tensor.offset);
	let mut cursor = 0_u64;
	for tensor in ordered {
		if tensor.offset < cursor {
			return Err(invalid_value(
				"gguf.tensors",
				format!(
					"tensor {:?} begins at {}, before prior end {cursor}",
					tensor.name, tensor.offset
				),
			));
		}
		let end = tensor
			.offset
			.checked_add(tensor.encoded_bytes)
			.ok_or_else(|| invalid_overflow("gguf.tensors", "tensor span overflowed u64"))?;
		if end > data_bytes {
			return Err(invalid_value(
				"gguf.tensors",
				format!(
					"tensor {:?} ends at {end}, tensor data has {data_bytes} bytes",
					tensor.name
				),
			));
		}
		let padding_start = data_start
			.checked_add(cursor)
			.ok_or_else(|| invalid_overflow("gguf.tensors", "padding start overflowed u64"))?;
		let padding_end = data_start
			.checked_add(tensor.offset)
			.ok_or_else(|| invalid_overflow("gguf.tensors", "padding end overflowed u64"))?;
		require_zero_stream_range(input, padding_start, padding_end, "gguf.tensor_padding")?;
		cursor = end;
	}
	let padded_end = align_up(cursor, u64::from(alignment))?;
	if data_bytes != cursor && data_bytes != padded_end {
		return Err(invalid_value(
			"gguf.data",
			format!("validated tensors end at {cursor}; tensor data has {data_bytes} bytes, expected the exact end or aligned end {padded_end}"),
		));
	}
	let terminal_start = data_start
		.checked_add(cursor)
		.ok_or_else(|| invalid_overflow("gguf.data", "terminal padding start overflowed u64"))?;
	require_zero_stream_range(
		input,
		terminal_start,
		file_bytes,
		"gguf.terminal_tensor_padding",
	)?;
	Ok(StreamGgufHeader {
		endian,
		alignment,
		metadata_count,
		architecture_type,
		architecture,
		tensors,
		data_start,
	})
}

fn validate_stream_metadata_key(key: &str, path: &str) -> GgufOgdlResult<()> {
	if key.is_empty() || host_u64(key.len(), path)? > STREAM_METADATA_KEY_BYTES_MAX {
		return Err(invalid_value(
			path,
			"metadata key length is outside the GGUF v3 range",
		));
	}
	Ok(())
}

fn require_zero_stream_range<R: Read + Seek>(input: &mut R, start: u64, end: u64, path: &str) -> GgufOgdlResult<()> {
	if end < start {
		return Err(invalid_overflow(path, "zero-padding range is reversed"));
	}
	input.seek(SeekFrom::Start(start))
		.map_err(|error| stream_io_error(path, error))?;
	let mut remaining = end - start;
	let mut buffer = [0_u8; STREAM_ZERO_BUFFER_BYTES];
	while remaining > 0 {
		let length = usize::try_from(remaining.min(STREAM_ZERO_BUFFER_BYTES as u64)).map_err(|error| invalid_overflow(path, format!("padding chunk does not fit usize: {error}")))?;
		input.read_exact(&mut buffer[..length])
			.map_err(|error| stream_io_error(path, error))?;
		if buffer[..length].iter().any(|byte| *byte != 0) {
			return Err(invalid_value(path, "padding contains nonzero bytes"));
		}
		remaining -= u64::try_from(length).map_err(|error| invalid_overflow(path, format!("padding chunk does not fit u64: {error}")))?;
	}
	Ok(())
}

fn write_tensor_payload_stream<R: Read, W: IoWrite>(input: &mut R, output: &mut W, tensor: &StreamTensor, endian: GgufEndian, path: &str) -> GgufOgdlResult<()> {
	let block_bytes = host_usize(tensor.tensor_type.block_bytes(), "tensor block bytes")?;
	let block_count = tensor.encoded_bytes / tensor.tensor_type.block_bytes();
	let mut block = vec![0_u8; block_bytes];
	if tensor.tensor_type.block_elements() == 1 {
		let mut next = 0_u64;
		while next < block_count {
			let count = (block_count - next).min(SCALAR_CHUNK_VALUES as u64);
			io_line(output, 4, &format!("chunk {next}"))?;
			let mut combined = None;
			for _ in 0..count {
				input.read_exact(&mut block)
					.map_err(|error| stream_io_error(&format!("{path}.payload"), error))?;
				let mut decoded = decode_block(tensor.tensor_type, endian, &block)?;
				if decoded.len() != 1 {
					return Err(invalid_structure(
						path,
						"scalar tensor block did not decode to one field",
					));
				}
				let field = decoded.remove(0);
				match &mut combined {
					None => combined = Some(field),
					Some(existing) => extend_field(existing, field)?,
				}
			}
			let field = combined.ok_or_else(|| invalid_structure(path, "scalar chunk is empty"))?;
			io_line(output, 5, &field_text(&field))?;
			next = next
				.checked_add(count)
				.ok_or_else(|| invalid_overflow(path, "scalar block index overflowed u64"))?;
		}
	} else {
		for block_index in 0..block_count {
			input.read_exact(&mut block)
				.map_err(|error| stream_io_error(&format!("{path}.payload"), error))?;
			io_line(output, 4, &format!("block {block_index}"))?;
			for field in decode_block(tensor.tensor_type, endian, &block)? {
				io_line(output, 5, &field_text(&field))?;
			}
		}
	}
	Ok(())
}

fn io_line<W: IoWrite>(output: &mut W, depth: usize, text: &str) -> GgufOgdlResult<()> {
	const TABS: &[u8; 64] = b"\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t";
	let mut remaining = depth;
	while remaining > 0 {
		let count = remaining.min(TABS.len());
		output.write_all(&TABS[..count])
			.map_err(|error| stream_io_error("<ogdl-output>", error))?;
		remaining -= count;
	}
	output.write_all(text.as_bytes())
		.and_then(|()| output.write_all(b"\n"))
		.map_err(|error| stream_io_error("<ogdl-output>", error))
}

fn stream_io_error(path: &str, error: std::io::Error) -> GgufOgdlError { GgufOgdlError::new(GgufOgdlErrorKind::Io, path, error.to_string()) }

#[derive(Clone, Debug, PartialEq, Eq)]
struct CanonicalLine {
	number: u64,
	depth: usize,
	text: String,
}

#[derive(Debug)]
struct CanonicalLines<'a, R> {
	input: &'a mut R,
	pending: Option<CanonicalLine>,
	line_number: u64,
}

impl<'a, R: BufRead + Seek> CanonicalLines<'a, R> {
	fn new(input: &'a mut R) -> GgufOgdlResult<Self> {
		input.seek(SeekFrom::Start(0))
			.map_err(|error| stream_io_error("<ogdl>", error))?;
		Ok(Self {
			input,
			pending: None,
			line_number: 0,
		})
	}

	fn reset(&mut self) -> GgufOgdlResult<()> {
		self.input
			.seek(SeekFrom::Start(0))
			.map_err(|error| stream_io_error("<ogdl>", error))?;
		self.pending = None;
		self.line_number = 0;
		Ok(())
	}

	fn next(&mut self) -> GgufOgdlResult<Option<CanonicalLine>> {
		if self.pending.is_some() {
			return Ok(self.pending.take());
		}
		self.read_next()
	}

	fn peek(&mut self) -> GgufOgdlResult<Option<CanonicalLine>> {
		if self.pending.is_none() {
			self.pending = self.read_next()?;
		}
		Ok(self.pending.clone())
	}

	fn read_next(&mut self) -> GgufOgdlResult<Option<CanonicalLine>> {
		let mut raw = String::new();
		let bytes = self
			.input
			.read_line(&mut raw)
			.map_err(|error| stream_io_error("<ogdl>", error))?;
		if bytes == 0 {
			return Ok(None);
		}
		self.line_number = self
			.line_number
			.checked_add(1)
			.ok_or_else(|| invalid_overflow("<ogdl>", "line number overflowed u64"))?;
		if raw.ends_with('\n') {
			raw.pop();
		}
		if raw.ends_with('\r') {
			return Err(invalid_value(
				format!("<ogdl>:{}", self.line_number),
				"canonical structural OGDL uses LF line endings",
			));
		}
		let depth = raw.bytes().take_while(|byte| *byte == b'\t').count();
		let text = raw
			.get(depth..)
			.ok_or_else(|| invalid_structure("<ogdl>", "line indentation is not a UTF-8 boundary"))?
			.to_owned();
		if text.is_empty() {
			return Err(invalid_value(
				format!("<ogdl>:{}", self.line_number),
				"canonical structural OGDL does not contain blank lines",
			));
		}
		Ok(Some(CanonicalLine {
			number: self.line_number,
			depth,
			text,
		}))
	}
}

#[derive(Clone, Copy, Debug)]
struct StructuralBudgets {
	string_bytes_remaining: u64,
	array_elements_remaining: u64,
	array_depth_limit: u64,
}

impl StructuralBudgets {
	fn new(limits: GgufLimits) -> Self {
		Self {
			string_bytes_remaining: limits.string_bytes().get(),
			array_elements_remaining: limits.array_elements().get(),
			array_depth_limit: limits.array_depth().get(),
		}
	}

	fn string(&mut self, value: &str, path: &str, per_value_limit: u64) -> GgufOgdlResult<()> {
		let length = host_u64(value.len(), path)?;
		if length > per_value_limit {
			return Err(invalid_value(
				path,
				format!("string has {length} bytes, per-value limit is {per_value_limit}"),
			));
		}
		if length > self.string_bytes_remaining {
			return Err(invalid_value(
				path,
				format!(
					"string consumes {length} bytes, aggregate remaining budget is {}",
					self.string_bytes_remaining
				),
			));
		}
		self.string_bytes_remaining -= length;
		Ok(())
	}

	fn array_element(&mut self, path: &str) -> GgufOgdlResult<()> {
		if self.array_elements_remaining == 0 {
			return Err(invalid_value(
				path,
				"aggregate metadata array element limit exceeded",
			));
		}
		self.array_elements_remaining -= 1;
		Ok(())
	}
}

#[derive(Debug)]
struct SeekBinaryWriter<'a, W> {
	output: &'a mut W,
	endian: GgufEndian,
	position: u64,
}

impl<'a, W: IoWrite + Seek> SeekBinaryWriter<'a, W> {
	fn new(output: &'a mut W, endian: GgufEndian) -> GgufOgdlResult<Self> {
		output.seek(SeekFrom::Start(0))
			.map_err(|error| stream_io_error("<gguf-output>", error))?;
		Ok(Self {
			output,
			endian,
			position: 0,
		})
	}

	const fn position(&self) -> u64 { self.position }

	fn bytes(&mut self, bytes: &[u8], path: &str) -> GgufOgdlResult<()> {
		self.output
			.write_all(bytes)
			.map_err(|error| stream_io_error(path, error))?;
		self.position = self
			.position
			.checked_add(host_u64(bytes.len(), path)?)
			.ok_or_else(|| invalid_overflow(path, "binary output position overflowed u64"))?;
		Ok(())
	}

	fn u8(&mut self, value: u8) -> GgufOgdlResult<()> { self.bytes(&[value], "<gguf-output>") }

	fn i8(&mut self, value: i8) -> GgufOgdlResult<()> { self.bytes(&value.to_ne_bytes(), "<gguf-output>") }

	fn u16(&mut self, value: u16) -> GgufOgdlResult<()> {
		self.bytes(
			&match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			},
			"<gguf-output>",
		)
	}

	fn i16(&mut self, value: i16) -> GgufOgdlResult<()> {
		self.bytes(
			&match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			},
			"<gguf-output>",
		)
	}

	fn u32(&mut self, value: u32) -> GgufOgdlResult<()> {
		self.bytes(
			&match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			},
			"<gguf-output>",
		)
	}

	fn i32(&mut self, value: i32) -> GgufOgdlResult<()> {
		self.bytes(
			&match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			},
			"<gguf-output>",
		)
	}

	fn u64(&mut self, value: u64) -> GgufOgdlResult<()> {
		self.bytes(
			&match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			},
			"<gguf-output>",
		)
	}

	fn i64(&mut self, value: i64) -> GgufOgdlResult<()> {
		self.bytes(
			&match self.endian {
				GgufEndian::Little => value.to_le_bytes(),
				GgufEndian::Big => value.to_be_bytes(),
			},
			"<gguf-output>",
		)
	}

	fn string(&mut self, value: &str) -> GgufOgdlResult<()> {
		self.u64(host_u64(value.len(), "<gguf-output>.string")?)?;
		self.bytes(value.as_bytes(), "<gguf-output>.string")
	}

	fn zeros(&mut self, length: u64) -> GgufOgdlResult<()> {
		const ZEROES: [u8; STREAM_ZERO_BUFFER_BYTES] = [0; STREAM_ZERO_BUFFER_BYTES];
		let mut remaining = length;
		while remaining > 0 {
			let count = host_usize(
				remaining.min(STREAM_ZERO_BUFFER_BYTES as u64),
				"<gguf-output>.padding",
			)?;
			self.bytes(&ZEROES[..count], "<gguf-output>.padding")?;
			remaining -= host_u64(count, "<gguf-output>.padding")?;
		}
		Ok(())
	}

	fn patch_u64(&mut self, position: u64, value: u64) -> GgufOgdlResult<()> {
		let resume = self.position;
		self.output
			.seek(SeekFrom::Start(position))
			.map_err(|error| stream_io_error("<gguf-output>.metadata-array-length", error))?;
		let bytes = match self.endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		};
		self.output
			.write_all(&bytes)
			.map_err(|error| stream_io_error("<gguf-output>.metadata-array-length", error))?;
		self.output
			.seek(SeekFrom::Start(resume))
			.map_err(|error| stream_io_error("<gguf-output>", error))?;
		Ok(())
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StructuralStreamHeader {
	endian: GgufEndian,
	alignment: u32,
	file_bytes: u64,
	metadata_count: u64,
	tensors: Vec<StreamTensor>,
	data_bytes: u64,
}

/// Stream canonical structural GGUF OGDL into a GGUF v3 file without retaining
/// the expanded tensor text or reconstructed binary image in memory.
///
/// Recipe makes bounded passes over the seekable OGDL input because GGUF places
/// all tensor descriptors before tensor data. `output` must initially be empty.
pub fn structural_ogdl_to_gguf_stream<R, W>(input: &mut R, output: &mut W, limits: GgufLimits) -> GgufOgdlResult<()>
where
	R: BufRead + Seek,
	W: Read + IoWrite + Seek,
{
	let output_bytes = output
		.seek(SeekFrom::End(0))
		.map_err(|error| stream_io_error("<gguf-output>", error))?;
	if output_bytes != 0 {
		return Err(invalid_value(
			"<gguf-output>",
			format!("streaming destination must be empty, found {output_bytes} bytes"),
		));
	}

	let mut lines = CanonicalLines::new(input)?;
	let header = inspect_structural_stream(&mut lines, limits)?;
	lines.reset()?;
	let (second_endian, second_alignment, second_file_bytes) = parse_structural_preamble(&mut lines)?;
	if second_endian != header.endian || second_alignment != header.alignment || second_file_bytes != header.file_bytes {
		return Err(invalid_structure(
			"gguf",
			"structural preamble changed between passes",
		));
	}
	let mut writer = SeekBinaryWriter::new(output, header.endian)?;
	writer.bytes(b"GGUF", "<gguf-output>.magic")?;
	writer.u32(3)?;
	writer.u64(host_u64(header.tensors.len(), "gguf.tensor_count")?)?;
	writer.u64(header.metadata_count)?;
	let mut budgets = StructuralBudgets::new(limits);
	let metadata_count = write_structural_metadata(
		&mut lines,
		&mut budgets,
		limits.metadata_pairs().get(),
		Some(&mut writer),
	)?;
	if metadata_count.count != header.metadata_count {
		return Err(invalid_structure(
			"gguf.metadata",
			"metadata count changed between passes",
		));
	}
	expect_canonical_line(&mut lines, 1, "tensors", "gguf.tensors")?;
	for tensor in &header.tensors {
		writer.string(&tensor.name)?;
		writer.u32(host_u32(tensor.dimensions.len(), "tensor rank")?)?;
		for dimension in &tensor.dimensions {
			writer.u64(*dimension)?;
		}
		writer.u32(tensor.tensor_type.code())?;
		writer.u64(tensor.offset)?;
	}
	let header_end = writer.position();
	let aligned_header_end = align_up(header_end, u64::from(header.alignment))?;
	let data_start = if header.tensors.is_empty() {
		if header.file_bytes == header_end || header.file_bytes == aligned_header_end {
			header.file_bytes
		} else {
			return Err(invalid_value(
				"gguf.file_bytes",
				format!(
					"tensor-free GGUF declares {} bytes; expected unpadded header end {header_end} or padded end {aligned_header_end}",
					header.file_bytes
				),
			));
		}
	} else {
		aligned_header_end
	};
	writer.zeros(data_start - header_end)?;
	let tensor_end = data_start
		.checked_add(header.data_bytes)
		.ok_or_else(|| invalid_overflow("gguf", "reconstructed file length overflowed u64"))?;
	let padded_data_bytes = align_up(header.data_bytes, u64::from(header.alignment))?;
	let padded_end = data_start
		.checked_add(padded_data_bytes)
		.ok_or_else(|| invalid_overflow("gguf", "padded reconstructed file length overflowed u64"))?;
	if header.file_bytes != tensor_end && header.file_bytes != padded_end {
		return Err(invalid_value(
			"gguf.file_bytes",
			format!(
				"declared {} bytes, but the structural layout requires exact end {tensor_end} or aligned end {padded_end}",
				header.file_bytes
			),
		));
	}
	let total = header.file_bytes;
	if total > limits.file_bytes().get() {
		return Err(invalid_value(
			"gguf",
			format!(
				"reconstructed file needs {total} bytes, configured limit is {}",
				limits.file_bytes()
			),
		));
	}
	writer.zeros(total - data_start)?;
	lines.reset()?;
	let (third_endian, third_alignment, third_file_bytes) = parse_structural_preamble(&mut lines)?;
	if third_endian != header.endian || third_alignment != header.alignment || third_file_bytes != header.file_bytes {
		return Err(invalid_structure(
			"gguf",
			"structural preamble changed between passes",
		));
	}
	let mut budgets = StructuralBudgets::new(limits);
	let metadata_count = write_structural_metadata::<R, W>(
		&mut lines,
		&mut budgets,
		limits.metadata_pairs().get(),
		None,
	)?;
	if metadata_count.count != header.metadata_count {
		return Err(invalid_structure(
			"gguf.metadata",
			"metadata count changed between passes",
		));
	}
	expect_canonical_line(&mut lines, 1, "tensors", "gguf.tensors")?;
	for (index, expected) in header.tensors.iter().enumerate() {
		let found = parse_structural_tensor_header(&mut lines, &mut budgets, limits.rank().get(), index)?;
		if &found != expected {
			return Err(invalid_structure(
				format!("gguf.tensors[{index}]"),
				"tensor descriptor changed between passes",
			));
		}
		let begin = data_start
			.checked_add(expected.offset)
			.ok_or_else(|| invalid_overflow("gguf.tensors", "tensor output position overflowed u64"))?;
		output.seek(SeekFrom::Start(begin))
			.map_err(|error| stream_io_error(&format!("gguf.tensors[{index}].payload"), error))?;
		read_structural_tensor_payload(&mut lines, output, expected, header.endian, index)?;
	}
	if let Some(extra) = lines.next()? {
		return Err(invalid_structure(
			format!("<ogdl>:{}", extra.number),
			format!(
				"unexpected trailing line at depth {}: {:?}",
				extra.depth, extra.text
			),
		));
	}
	output.flush()
		.map_err(|error| stream_io_error("<gguf-output>", error))?;
	output.seek(SeekFrom::Start(0))
		.map_err(|error| stream_io_error("<gguf-output>", error))?;
	let reparsed = inspect_stream_gguf(output, total, limits)?;
	if reparsed.alignment != header.alignment || reparsed.tensors != header.tensors {
		return Err(invalid_structure(
			"gguf",
			"streamed GGUF validation disagrees with structural input",
		));
	}
	output.seek(SeekFrom::End(0))
		.map_err(|error| stream_io_error("<gguf-output>", error))?;
	Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StructuralMetadataSummary {
	count: u64,
	alignment: Option<u32>,
}

#[derive(Debug)]
struct StructuralArrayFrame {
	depth: usize,
	element_type: GgufMetadataType,
	count: u64,
	length_position: Option<u64>,
}

fn expect_canonical_line<R: BufRead + Seek>(lines: &mut CanonicalLines<'_, R>, depth: usize, text: &str, path: &str) -> GgufOgdlResult<CanonicalLine> {
	let line = lines
		.next()?
		.ok_or_else(|| invalid_structure(path, format!("expected {text:?}, found end of input")))?;
	if line.depth != depth || line.text != text {
		return Err(invalid_structure(
			path,
			format!(
				"line {} is depth {} text {:?}, expected depth {depth} text {text:?}",
				line.number, line.depth, line.text
			),
		));
	}
	Ok(line)
}

fn take_canonical_line<R: BufRead + Seek>(lines: &mut CanonicalLines<'_, R>, depth: usize, path: &str) -> GgufOgdlResult<CanonicalLine> {
	let line = lines
		.next()?
		.ok_or_else(|| invalid_structure(path, "unexpected end of structural OGDL"))?;
	if line.depth != depth {
		return Err(invalid_structure(
			path,
			format!(
				"line {} has depth {}, expected {depth}: {:?}",
				line.number, line.depth, line.text
			),
		));
	}
	Ok(line)
}

fn parse_structural_preamble<R: BufRead + Seek>(lines: &mut CanonicalLines<'_, R>) -> GgufOgdlResult<(GgufEndian, u32, u64)> {
	expect_canonical_line(lines, 0, "gguf", "gguf")?;
	expect_canonical_line(
		lines,
		1,
		&format!("schema {STRUCTURAL_SCHEMA}"),
		"gguf.schema",
	)?;
	expect_canonical_line(lines, 1, "version 3", "gguf.version")?;
	let endian_line = take_canonical_line(lines, 1, "gguf.endian")?;
	let endian = match endian_line.text.as_str() {
		"endian little" => GgufEndian::Little,
		"endian big" => GgufEndian::Big,
		other => {
			return Err(invalid_value(
				"gguf.endian",
				format!("invalid endian declaration {other:?}"),
			));
		}
	};
	let alignment_line = take_canonical_line(lines, 1, "gguf.alignment")?;
	let alignment = parse_single_u32(
		strip_prefix(&alignment_line.text, "alignment ", "gguf.alignment")?,
		"gguf.alignment",
	)?;
	if alignment == 0 || alignment % 8 != 0 {
		return Err(invalid_value(
			"gguf.alignment",
			format!("alignment {alignment} is not a nonzero multiple of eight"),
		));
	}
	let file_bytes_line = take_canonical_line(lines, 1, "gguf.file_bytes")?;
	let file_bytes = parse_number::<u64>(
		strip_prefix(&file_bytes_line.text, "file_bytes ", "gguf.file_bytes")?,
		"gguf.file_bytes",
		"u64",
	)?;
	if file_bytes == 0 {
		return Err(invalid_value(
			"gguf.file_bytes",
			"binary file length must be nonzero",
		));
	}
	expect_canonical_line(lines, 1, "metadata", "gguf.metadata")?;
	Ok((endian, alignment, file_bytes))
}

/// Read the exact GGUF byte length declared by canonical structural OGDL.
///
/// The reader is rewound before returning so it can be passed directly to
/// [`structural_ogdl_to_gguf_stream`].
pub fn structural_ogdl_declared_gguf_bytes<R>(input: &mut R) -> GgufOgdlResult<u64>
where R: BufRead + Seek {
	let mut lines = CanonicalLines::new(input)?;
	let (_, _, file_bytes) = parse_structural_preamble(&mut lines)?;
	lines.reset()?;
	Ok(file_bytes)
}

fn inspect_structural_stream<R: BufRead + Seek>(lines: &mut CanonicalLines<'_, R>, limits: GgufLimits) -> GgufOgdlResult<StructuralStreamHeader> {
	let (endian, alignment, file_bytes) = parse_structural_preamble(lines)?;
	if file_bytes > limits.file_bytes().get() {
		return Err(invalid_value(
			"gguf.file_bytes",
			format!(
				"declared {file_bytes} bytes, configured limit is {}",
				limits.file_bytes()
			),
		));
	}
	let mut budgets = StructuralBudgets::new(limits);
	let metadata = write_structural_metadata::<R, std::io::Cursor<Vec<u8>>>(lines, &mut budgets, limits.metadata_pairs().get(), None)?;
	if metadata.count > limits.metadata_pairs().get() {
		return Err(invalid_value(
			"gguf.metadata",
			format!(
				"metadata count {} exceeds configured limit {}",
				metadata.count,
				limits.metadata_pairs()
			),
		));
	}
	let metadata_alignment = metadata.alignment.unwrap_or(STREAM_DEFAULT_ALIGNMENT);
	if metadata_alignment != alignment {
		return Err(invalid_value(
			"gguf.alignment",
			format!("structural alignment {alignment} disagrees with general.alignment/default {metadata_alignment}"),
		));
	}
	expect_canonical_line(lines, 1, "tensors", "gguf.tensors")?;
	let mut tensors = Vec::new();
	let mut names = BTreeSet::new();
	while let Some(next) = lines.peek()? {
		if next.depth != 2 || next.text != "tensor" {
			return Err(invalid_structure(
				format!("<ogdl>:{}", next.number),
				format!(
					"expected a depth-2 tensor declaration, found depth {} {:?}",
					next.depth, next.text
				),
			));
		}
		let index = tensors.len();
		let tensor = parse_structural_tensor_header(lines, &mut budgets, limits.rank().get(), index)?;
		if !names.insert(tensor.name.clone()) {
			return Err(invalid_value(
				format!("gguf.tensors[{index}].name"),
				format!("duplicate tensor name {:?}", tensor.name),
			));
		}
		tensors.push(tensor);
		if host_u64(tensors.len(), "gguf.tensor_count")? > limits.tensors().get() {
			return Err(invalid_value(
				"gguf.tensors",
				format!("tensor count exceeds configured limit {}", limits.tensors()),
			));
		}
		while lines.peek()?.is_some_and(|line| line.depth > 3) {
			lines.next()?;
		}
	}

	let mut ordered = tensors.iter().collect::<Vec<_>>();
	ordered.sort_by_key(|tensor| tensor.offset);
	let mut cursor = 0_u64;
	for tensor in ordered {
		if tensor.offset % u64::from(alignment) != 0 {
			return Err(invalid_value(
				"gguf.tensors",
				format!(
					"tensor {:?} offset {} is not aligned to {alignment}",
					tensor.name, tensor.offset
				),
			));
		}
		if tensor.offset < cursor {
			return Err(invalid_value(
				"gguf.tensors",
				format!(
					"tensor {:?} begins at {}, before prior end {cursor}",
					tensor.name, tensor.offset
				),
			));
		}
		cursor = tensor
			.offset
			.checked_add(tensor.encoded_bytes)
			.ok_or_else(|| invalid_overflow("gguf.tensors", "tensor span overflowed u64"))?;
	}
	Ok(StructuralStreamHeader {
		endian,
		alignment,
		file_bytes,
		metadata_count: metadata.count,
		tensors,
		data_bytes: cursor,
	})
}

fn write_structural_metadata<R, W>(lines: &mut CanonicalLines<'_, R>, budgets: &mut StructuralBudgets, metadata_limit: u64, mut writer: Option<&mut SeekBinaryWriter<'_, W>>) -> GgufOgdlResult<StructuralMetadataSummary>
where
	R: BufRead + Seek,
	W: IoWrite + Seek,
{
	let mut count = 0_u64;
	let mut alignment = None;
	let mut keys = BTreeSet::new();
	loop {
		let Some(next) = lines.peek()? else {
			return Err(invalid_structure(
				"gguf.metadata",
				"input ended before the tensors section",
			));
		};
		if next.depth == 1 && next.text == "tensors" {
			break;
		}
		let path = format!("gguf.metadata[{count}]");
		expect_canonical_line(lines, 2, "entry", &path)?;
		let key_line = take_canonical_line(lines, 3, &format!("{path}.key"))?;
		let key_text = strip_prefix(&key_line.text, "key ", &format!("{path}.key"))?;
		let key: String = serde_json::from_str(key_text).map_err(|error| {
			invalid_value(
				format!("{path}.key"),
				format!("metadata key is not a JSON string: {error}"),
			)
		})?;
		validate_stream_metadata_key(&key, &format!("{path}.key"))?;
		budgets.string(&key, &format!("{path}.key"), STREAM_METADATA_KEY_BYTES_MAX)?;
		if !keys.insert(key.clone()) {
			return Err(invalid_value(
				&path,
				format!("duplicate metadata key {key:?}"),
			));
		}
		let value_line = take_canonical_line(lines, 3, &format!("{path}.value"))?;
		let (value_type, payload) = parse_structural_metadata_declaration(&value_line, "value", &path)?;
		if key == "general.alignment" && value_type != GgufMetadataType::U32 {
			return Err(invalid_value(
				format!("{path}.value"),
				format!(
					"general.alignment must be u32, found {}",
					metadata_type_name(value_type)
				),
			));
		}
		if let Some(output) = writer.as_mut() {
			(**output).string(&key)?;
			(**output).u32(value_type.code())?;
		}
		let root_u32 = parse_structural_metadata_value(
			lines,
			budgets,
			&value_line,
			value_type,
			payload,
			&mut writer,
			&format!("{path}.value"),
		)?;
		if key == "general.alignment" {
			alignment = root_u32;
		}
		count = count
			.checked_add(1)
			.ok_or_else(|| invalid_overflow("gguf.metadata", "metadata count overflowed u64"))?;
		if count > metadata_limit {
			return Err(invalid_value(
				"gguf.metadata",
				format!("metadata count {count} exceeds configured limit {metadata_limit}"),
			));
		}
	}
	Ok(StructuralMetadataSummary { count, alignment })
}

fn parse_structural_metadata_declaration(line: &CanonicalLine, label: &str, path: &str) -> GgufOgdlResult<(GgufMetadataType, String)> {
	let declaration = strip_prefix(&line.text, &format!("{label} "), path)?;
	let (kind, payload) = declaration.split_once(' ').unwrap_or((declaration, ""));
	let value_type = parse_metadata_type(kind, path)?;
	if value_type == GgufMetadataType::Array {
		if payload.is_empty() || payload.contains(char::is_whitespace) {
			return Err(invalid_value(
				path,
				"array declaration must name exactly one element type",
			));
		}
		parse_metadata_type(payload, path)?;
	} else if payload.is_empty() {
		return Err(invalid_value(
			path,
			format!("{kind} metadata value is missing"),
		));
	}
	Ok((value_type, payload.to_owned()))
}

fn parse_structural_metadata_value<R, W>(lines: &mut CanonicalLines<'_, R>, budgets: &mut StructuralBudgets, root_line: &CanonicalLine, root_type: GgufMetadataType, root_payload: String, writer: &mut Option<&mut SeekBinaryWriter<'_, W>>, path: &str) -> GgufOgdlResult<Option<u32>>
where
	R: BufRead + Seek,
	W: IoWrite + Seek,
{
	if root_type != GgufMetadataType::Array {
		let value = write_structural_metadata_scalar(root_type, &root_payload, budgets, writer, path)?;
		if lines
			.peek()?
			.is_some_and(|line| line.depth > root_line.depth)
		{
			return Err(invalid_structure(
				path,
				"scalar metadata value has child lines",
			));
		}
		return Ok(value);
	}

	let root_element_type = parse_metadata_type(&root_payload, path)?;
	let mut frames = Vec::new();
	open_structural_array(
		&mut frames,
		root_line.depth,
		root_element_type,
		budgets,
		writer,
		path,
	)?;
	loop {
		let next = lines.peek()?;
		let next_depth = next.as_ref().map(|line| line.depth);
		while next_depth.is_none_or(|depth| depth <= frames.last().expect("root array frame exists").depth) {
			close_structural_array(&mut frames, writer)?;
			if frames.is_empty() {
				return Ok(None);
			}
		}
		let frame = frames.last().expect("array frame exists");
		let expected_depth = frame
			.depth
			.checked_add(1)
			.ok_or_else(|| invalid_overflow(path, "metadata indentation depth overflowed usize"))?;
		let child = lines
			.next()?
			.ok_or_else(|| invalid_structure(path, "metadata array ended unexpectedly"))?;
		if child.depth != expected_depth {
			return Err(invalid_structure(
				path,
				format!(
					"array child line {} has depth {}, expected {expected_depth}",
					child.number, child.depth
				),
			));
		}
		let child_path = format!("{path}[{}]", frame.count);
		let (child_type, child_payload) = parse_structural_metadata_declaration(&child, "element", &child_path)?;
		if child_type != frame.element_type {
			return Err(invalid_value(
				&child_path,
				format!(
					"array declares {} elements but found {}",
					metadata_type_name(frame.element_type),
					metadata_type_name(child_type)
				),
			));
		}
		budgets.array_element(&child_path)?;
		frames.last_mut().expect("array frame exists").count = frame
			.count
			.checked_add(1)
			.ok_or_else(|| invalid_overflow(&child_path, "metadata array length overflowed u64"))?;
		if child_type == GgufMetadataType::Array {
			let element_type = parse_metadata_type(&child_payload, &child_path)?;
			open_structural_array(
				&mut frames,
				child.depth,
				element_type,
				budgets,
				writer,
				&child_path,
			)?;
		} else {
			write_structural_metadata_scalar(child_type, &child_payload, budgets, writer, &child_path)?;
		}
	}
}

fn open_structural_array<W: IoWrite + Seek>(frames: &mut Vec<StructuralArrayFrame>, depth: usize, element_type: GgufMetadataType, budgets: &StructuralBudgets, writer: &mut Option<&mut SeekBinaryWriter<'_, W>>, path: &str) -> GgufOgdlResult<()> {
	let array_depth = host_u64(frames.len(), path)?
		.checked_add(1)
		.ok_or_else(|| invalid_overflow(path, "metadata array depth overflowed u64"))?;
	if array_depth > budgets.array_depth_limit {
		return Err(invalid_value(
			path,
			format!(
				"array depth {array_depth} exceeds configured limit {}",
				budgets.array_depth_limit
			),
		));
	}
	let length_position = if let Some(output) = writer.as_mut() {
		(**output).u32(element_type.code())?;
		let position = (**output).position();
		(**output).u64(0)?;
		Some(position)
	} else {
		None
	};
	frames.push(StructuralArrayFrame {
		depth,
		element_type,
		count: 0,
		length_position,
	});
	Ok(())
}

fn close_structural_array<W: IoWrite + Seek>(frames: &mut Vec<StructuralArrayFrame>, writer: &mut Option<&mut SeekBinaryWriter<'_, W>>) -> GgufOgdlResult<()> {
	let frame = frames
		.pop()
		.ok_or_else(|| invalid_structure("gguf.metadata", "metadata array stack underflowed"))?;
	if let (Some(position), Some(output)) = (frame.length_position, writer.as_mut()) {
		(**output).patch_u64(position, frame.count)?;
	}
	Ok(())
}

fn write_structural_metadata_scalar<W: IoWrite + Seek>(value_type: GgufMetadataType, payload: &str, budgets: &mut StructuralBudgets, writer: &mut Option<&mut SeekBinaryWriter<'_, W>>, path: &str) -> GgufOgdlResult<Option<u32>> {
	macro_rules! write_value {
		($method:ident, $value:expr) => {{
			let value = $value;
			if let Some(output) = writer.as_mut() {
				(**output).$method(value)?;
			}
			value
		}};
	}
	let u32_value = match value_type {
		GgufMetadataType::U8 => {
			write_value!(u8, parse_number::<u8>(payload, path, "u8")?);
			None
		}
		GgufMetadataType::I8 => {
			write_value!(i8, parse_number::<i8>(payload, path, "i8")?);
			None
		}
		GgufMetadataType::U16 => {
			write_value!(u16, parse_number::<u16>(payload, path, "u16")?);
			None
		}
		GgufMetadataType::I16 => {
			write_value!(i16, parse_number::<i16>(payload, path, "i16")?);
			None
		}
		GgufMetadataType::U32 => {
			let value = write_value!(u32, parse_number::<u32>(payload, path, "u32")?);
			Some(value)
		}
		GgufMetadataType::I32 => {
			write_value!(i32, parse_number::<i32>(payload, path, "i32")?);
			None
		}
		GgufMetadataType::F32 => {
			let parts = parse_float_parts(payload, FloatFormat::F32, path)?;
			let bits = u32::try_from(join_float(parts, FloatFormat::F32)?).map_err(|error| invalid_value(path, error.to_string()))?;
			write_value!(u32, bits);
			None
		}
		GgufMetadataType::Bool => {
			let value = match payload {
				"true" => 1,
				"false" => 0,
				other => {
					return Err(invalid_value(
						path,
						format!("boolean must be true or false, found {other:?}"),
					));
				}
			};
			write_value!(u8, value);
			None
		}
		GgufMetadataType::String => {
			let value: String = serde_json::from_str(payload).map_err(|error| invalid_value(path, format!("string is not valid JSON text: {error}")))?;
			budgets.string(&value, path, u64::MAX)?;
			if let Some(output) = writer.as_mut() {
				(**output).string(&value)?;
			}
			None
		}
		GgufMetadataType::U64 => {
			write_value!(u64, parse_number::<u64>(payload, path, "u64")?);
			None
		}
		GgufMetadataType::I64 => {
			write_value!(i64, parse_number::<i64>(payload, path, "i64")?);
			None
		}
		GgufMetadataType::F64 => {
			let parts = parse_float_parts(payload, FloatFormat::F64, path)?;
			write_value!(u64, join_float(parts, FloatFormat::F64)?);
			None
		}
		GgufMetadataType::Array => {
			return Err(invalid_structure(
				path,
				"array reached metadata scalar writer",
			));
		}
	};
	Ok(u32_value)
}

fn parse_structural_tensor_header<R: BufRead + Seek>(lines: &mut CanonicalLines<'_, R>, budgets: &mut StructuralBudgets, rank_limit: u32, index: usize) -> GgufOgdlResult<StreamTensor> {
	let path = format!("gguf.tensors[{index}]");
	expect_canonical_line(lines, 2, "tensor", &path)?;
	let name_line = take_canonical_line(lines, 3, &format!("{path}.name"))?;
	let name_text = strip_prefix(&name_line.text, "name ", &format!("{path}.name"))?;
	let name: String = serde_json::from_str(name_text).map_err(|error| {
		invalid_value(
			format!("{path}.name"),
			format!("name is not a JSON string: {error}"),
		)
	})?;
	budgets.string(&name, &format!("{path}.name"), STREAM_TENSOR_NAME_BYTES_MAX)?;

	let dimensions_line = take_canonical_line(lines, 3, &format!("{path}.dimensions"))?;
	let dimensions_text = match dimensions_line.text.as_str() {
		"dimensions" => "",
		text => strip_prefix(text, "dimensions ", &format!("{path}.dimensions"))?,
	};
	let dimensions = if dimensions_text.is_empty() {
		Vec::new()
	} else {
		dimensions_text
			.split_whitespace()
			.enumerate()
			.map(|(axis, token)| parse_number::<u64>(token, &format!("{path}.dimensions[{axis}]"), "u64"))
			.collect::<GgufOgdlResult<Vec<_>>>()?
	};
	if dimensions.len() > 4 {
		return Err(invalid_value(
			format!("{path}.dimensions"),
			format!(
				"current GGUF v3 permits at most four dimensions, found {}",
				dimensions.len()
			),
		));
	}
	if host_u32(dimensions.len(), &format!("{path}.dimensions"))? > rank_limit {
		return Err(invalid_value(
			format!("{path}.dimensions"),
			format!(
				"tensor rank {} exceeds configured limit {rank_limit}",
				dimensions.len()
			),
		));
	}
	let type_line = take_canonical_line(lines, 3, &format!("{path}.type"))?;
	let tensor_type = parse_tensor_type(
		strip_prefix(&type_line.text, "type ", &format!("{path}.type"))?,
		&format!("{path}.type"),
	)?;
	let offset_line = take_canonical_line(lines, 3, &format!("{path}.offset"))?;
	let offset = parse_number::<u64>(
		strip_prefix(&offset_line.text, "offset ", &format!("{path}.offset"))?,
		&format!("{path}.offset"),
		"u64",
	)?;
	expect_canonical_line(lines, 3, "payload", &format!("{path}.payload"))?;
	let encoded_bytes = tensor_encoded_bytes(&name, &dimensions, tensor_type)?;
	Ok(StreamTensor {
		name,
		dimensions,
		tensor_type,
		offset,
		encoded_bytes,
	})
}

fn read_structural_tensor_payload<R, W>(lines: &mut CanonicalLines<'_, R>, output: &mut W, tensor: &StreamTensor, endian: GgufEndian, index: usize) -> GgufOgdlResult<()>
where
	R: BufRead + Seek,
	W: IoWrite,
{
	let path = format!("gguf.tensors[{index}].payload");
	let block_bytes = host_usize(tensor.tensor_type.block_bytes(), &path)?;
	let block_count = tensor.encoded_bytes / tensor.tensor_type.block_bytes();
	let templates = decode_block(tensor.tensor_type, endian, &vec![0_u8; block_bytes])?;
	if tensor.tensor_type.block_elements() == 1 {
		if templates.len() != 1 {
			return Err(invalid_structure(
				&path,
				"scalar tensor layout has more than one field",
			));
		}
		let mut next = 0_u64;
		while next < block_count {
			expect_canonical_line(
				lines,
				4,
				&format!("chunk {next}"),
				&format!("{path}.chunks"),
			)?;
			let field_line = take_canonical_line(lines, 5, &format!("{path}.chunks[{next}].values"))?;
			let combined = parse_field_text(
				&field_line.text,
				&templates[0],
				None,
				&format!("{path}.chunks[{next}].values"),
			)?;
			let count = field_len(&combined);
			let count_u64 = host_u64(count, &path)?;
			if count == 0 || count > SCALAR_CHUNK_VALUES || count_u64 > block_count - next {
				return Err(invalid_value(
					&path,
					format!(
						"scalar chunk contains {count} values outside the remaining {}",
						block_count - next
					),
				));
			}
			for value_index in 0..count {
				let field = field_value_at(&combined, value_index)?;
				let encoded = encode_block(tensor.tensor_type, endian, &[field])?;
				output.write_all(&encoded)
					.map_err(|error| stream_io_error(&path, error))?;
			}
			next = next
				.checked_add(count_u64)
				.ok_or_else(|| invalid_overflow(&path, "scalar tensor position overflowed u64"))?;
		}
	} else {
		for block_index in 0..block_count {
			let block_path = format!("{path}.blocks[{block_index}]");
			expect_canonical_line(lines, 4, &format!("block {block_index}"), &block_path)?;
			let mut fields = Vec::with_capacity(templates.len());
			for (field_index, template) in templates.iter().enumerate() {
				let field_line = take_canonical_line(lines, 5, &format!("{block_path}.fields[{field_index}]"))?;
				fields.push(parse_field_text(
					&field_line.text,
					template,
					Some(field_len(template)),
					&format!("{block_path}.fields[{field_index}]"),
				)?);
			}
			let encoded = encode_block(tensor.tensor_type, endian, &fields)?;
			output.write_all(&encoded)
				.map_err(|error| stream_io_error(&block_path, error))?;
		}
	}
	if lines.peek()?.is_some_and(|line| line.depth > 3) {
		let extra = lines.next()?.expect("peeked payload line exists");
		return Err(invalid_structure(
			&path,
			format!(
				"unexpected extra payload line {}: {:?}",
				extra.number, extra.text
			),
		));
	}
	Ok(())
}

fn json_string(value: &str, path: &str) -> GgufOgdlResult<String> {
	serde_json::to_string(value).map_err(|error| {
		GgufOgdlError::new(
			GgufOgdlErrorKind::InvalidValue,
			path,
			format!("cannot represent UTF-8 string as JSON text: {error}"),
		)
	})
}

fn field_text(field: &StructuralField) -> String {
	let mut text = String::from(field.name);
	match &field.values {
		FieldValues::Unsigned { kind, values, .. } => {
			write!(text, " {kind}").expect("writing to String cannot fail");
			for value in values {
				write!(text, " {value}").expect("writing to String cannot fail");
			}
		}
		FieldValues::Signed { kind, values, .. } => {
			write!(text, " {kind}").expect("writing to String cannot fail");
			for value in values {
				write!(text, " {value}").expect("writing to String cannot fail");
			}
		}
		FieldValues::Float { format, values } => {
			write!(text, " {}", format.name()).expect("writing to String cannot fail");
			for value in values {
				write!(
					text,
					" {} {} {}",
					value.sign, value.exponent, value.fraction
				)
				.expect("writing to String cannot fail");
			}
		}
	}
	text
}

fn extend_field(destination: &mut StructuralField, source: StructuralField) -> GgufOgdlResult<()> {
	if destination.name != source.name {
		return Err(invalid_structure(
			"tensor.payload",
			"scalar field names changed within one chunk",
		));
	}
	match (&mut destination.values, source.values) {
		(
			FieldValues::Unsigned {
				kind: left_kind,
				maximum: left_maximum,
				values: left,
			},
			FieldValues::Unsigned {
				kind: right_kind,
				maximum: right_maximum,
				values: right,
			},
		) if left_kind == &right_kind && left_maximum == &right_maximum => left.extend(right),
		(
			FieldValues::Signed {
				kind: left_kind,
				minimum: left_minimum,
				maximum: left_maximum,
				values: left,
			},
			FieldValues::Signed {
				kind: right_kind,
				minimum: right_minimum,
				maximum: right_maximum,
				values: right,
			},
		) if left_kind == &right_kind && left_minimum == &right_minimum && left_maximum == &right_maximum => {
			left.extend(right);
		}
		(
			FieldValues::Float {
				format: left_format,
				values: left,
			},
			FieldValues::Float {
				format: right_format,
				values: right,
			},
		) if left_format == &right_format => left.extend(right),
		_ => {
			return Err(invalid_structure(
				"tensor.payload",
				"scalar field type changed within one chunk",
			));
		}
	}
	Ok(())
}

fn decode_block(tensor_type: GgufTensorType, endian: GgufEndian, block: &[u8]) -> GgufOgdlResult<Vec<StructuralField>> {
	let expected = host_usize(tensor_type.block_bytes(), "tensor block bytes")?;
	if block.len() != expected {
		return Err(invalid_structure(
			"tensor.payload",
			format!(
				"{:?} block has {} bytes, expected {expected}",
				tensor_type,
				block.len()
			),
		));
	}
	let mut reader = BlockReader::new(block, endian);
	let fields = match tensor_type {
		GgufTensorType::F32 => vec![float_field("value", FloatFormat::F32, reader.u32("value")?)],
		GgufTensorType::F16 => {
			vec![float_field(
				"value",
				FloatFormat::F16,
				u64::from(reader.u16("value")?),
			)]
		}
		GgufTensorType::Bf16 => {
			vec![float_field(
				"value",
				FloatFormat::Bf16,
				u64::from(reader.u16("value")?),
			)]
		}
		GgufTensorType::F64 => vec![float_field("value", FloatFormat::F64, reader.u64("value")?)],
		GgufTensorType::I8 => {
			vec![signed_field(
				"value",
				"i8",
				i64::from(i8::from_ne_bytes([reader.u8("value")?])),
				i64::from(i8::MIN),
				i64::from(i8::MAX),
			)]
		}
		GgufTensorType::I16 => {
			vec![signed_field(
				"value",
				"i16",
				i64::from(reader.i16("value")?),
				i64::from(i16::MIN),
				i64::from(i16::MAX),
			)]
		}
		GgufTensorType::I32 => {
			vec![signed_field(
				"value",
				"i32",
				i64::from(reader.i32("value")?),
				i64::from(i32::MIN),
				i64::from(i32::MAX),
			)]
		}
		GgufTensorType::I64 => {
			vec![signed_field(
				"value",
				"i64",
				reader.i64("value")?,
				i64::MIN,
				i64::MAX,
			)]
		}
		GgufTensorType::Q1_0 => {
			vec![
				f16_from(&mut reader, "delta")?,
				unsigned_values(
					"sign_bits",
					"u1",
					1,
					unpack_lsb(reader.take(16, "sign bits")?, 1),
				),
			]
		}
		GgufTensorType::Q2_0 => {
			vec![
				f16_from(&mut reader, "delta")?,
				unsigned_values(
					"quant_codes",
					"u2",
					3,
					unpack_lsb(reader.take(16, "quant codes")?, 2),
				),
			]
		}
		GgufTensorType::Q4_0 => {
			vec![
				f16_from(&mut reader, "delta")?,
				unsigned_values(
					"quant_codes",
					"u4",
					15,
					unpack_halves(reader.take(16, "quant codes")?),
				),
			]
		}
		GgufTensorType::Q4_1 => {
			vec![
				f16_from(&mut reader, "delta")?,
				f16_from(&mut reader, "minimum")?,
				unsigned_values(
					"quant_codes",
					"u4",
					15,
					unpack_halves(reader.take(16, "quant codes")?),
				),
			]
		}
		GgufTensorType::Q5_0 => {
			vec![
				f16_from(&mut reader, "delta")?,
				unsigned_values(
					"quant_high_bits",
					"u1",
					1,
					unpack_lsb(reader.take(4, "quant high bits")?, 1),
				),
				unsigned_values(
					"quant_low_codes",
					"u4",
					15,
					unpack_halves(reader.take(16, "quant low codes")?),
				),
			]
		}
		GgufTensorType::Q5_1 => {
			vec![
				f16_from(&mut reader, "delta")?,
				f16_from(&mut reader, "minimum")?,
				unsigned_values(
					"quant_high_bits",
					"u1",
					1,
					unpack_lsb(reader.take(4, "quant high bits")?, 1),
				),
				unsigned_values(
					"quant_low_codes",
					"u4",
					15,
					unpack_halves(reader.take(16, "quant low codes")?),
				),
			]
		}
		GgufTensorType::Q8_0 => {
			vec![
				f16_from(&mut reader, "delta")?,
				i8_array_from(&mut reader, "quant_codes", 32)?,
			]
		}
		GgufTensorType::Q8_1 => {
			vec![
				f16_from(&mut reader, "delta")?,
				f16_from(&mut reader, "scaled_sum")?,
				i8_array_from(&mut reader, "quant_codes", 32)?,
			]
		}
		GgufTensorType::Q2K => {
			let scales = reader.take(16, "scale and minimum codes")?;
			let (scale_codes, minimum_codes) = unpack_nibble_pairs(scales);
			vec![
				unsigned_values("scale_codes", "u4", 15, scale_codes),
				unsigned_values("minimum_codes", "u4", 15, minimum_codes),
				unsigned_values(
					"quant_bitplanes",
					"u2",
					3,
					unpack_lsb(reader.take(64, "quant bitplanes")?, 2),
				),
				f16_from(&mut reader, "scale")?,
				f16_from(&mut reader, "minimum_scale")?,
			]
		}
		GgufTensorType::Q3K => {
			let high_masks = unpack_lsb(reader.take(32, "quant high masks")?, 1);
			let low_codes = unpack_lsb(reader.take(64, "quant low codes")?, 2);
			let scale_codes = unpack_q3k_scales(reader.take(12, "scale codes")?);
			vec![
				unsigned_values("quant_high_masks", "u1", 1, high_masks),
				unsigned_values("quant_low_codes", "u2", 3, low_codes),
				unsigned_values("scale_codes", "u6", 63, scale_codes),
				f16_from(&mut reader, "scale")?,
			]
		}
		GgufTensorType::Q4K => {
			let scale = f16_from(&mut reader, "scale")?;
			let minimum_scale = f16_from(&mut reader, "minimum_scale")?;
			let (scale_codes, minimum_codes) = unpack_k_scale_min(reader.take(12, "scale and minimum codes")?);
			vec![
				scale,
				minimum_scale,
				unsigned_values("scale_codes", "u6", 63, scale_codes),
				unsigned_values("minimum_codes", "u6", 63, minimum_codes),
				unsigned_values(
					"quant_codes",
					"u4",
					15,
					unpack_halves(reader.take(128, "quant codes")?),
				),
			]
		}
		GgufTensorType::Q5K => {
			let scale = f16_from(&mut reader, "scale")?;
			let minimum_scale = f16_from(&mut reader, "minimum_scale")?;
			let (scale_codes, minimum_codes) = unpack_k_scale_min(reader.take(12, "scale and minimum codes")?);
			vec![
				scale,
				minimum_scale,
				unsigned_values("scale_codes", "u6", 63, scale_codes),
				unsigned_values("minimum_codes", "u6", 63, minimum_codes),
				unsigned_values(
					"quant_high_bitplanes",
					"u1",
					1,
					unpack_lsb(reader.take(32, "quant high bitplanes")?, 1),
				),
				unsigned_values(
					"quant_low_codes",
					"u4",
					15,
					unpack_halves(reader.take(128, "quant low codes")?),
				),
			]
		}
		GgufTensorType::Q6K => {
			vec![
				unsigned_values(
					"quant_low_codes",
					"u4",
					15,
					unpack_halves(reader.take(128, "quant low codes")?),
				),
				unsigned_values(
					"quant_high_codes",
					"u2",
					3,
					unpack_lsb(reader.take(64, "quant high codes")?, 2),
				),
				i8_array_from(&mut reader, "scale_codes", 16)?,
				f16_from(&mut reader, "scale")?,
			]
		}
		GgufTensorType::Q8K => {
			vec![
				f32_from(&mut reader, "delta")?,
				i8_array_from(&mut reader, "quant_codes", 256)?,
				i16_array_from(&mut reader, "block_sums", 16)?,
			]
		}
		GgufTensorType::Iq2Xxs => {
			let delta = f16_from(&mut reader, "delta")?;
			let mut grid_indices = Vec::with_capacity(32);
			let mut sign_codes = Vec::with_capacity(32);
			let mut scale_codes = Vec::with_capacity(8);
			for _ in 0..8 {
				let grid_word = reader.u32("grid index word")?;
				grid_indices.extend(endian_u32_bytes(grid_word, endian).map(u64::from));
				let sign_scale = reader.u32("sign and scale word")?;
				for shift in [0_u32, 7, 14, 21] {
					sign_codes.push(u64::from((sign_scale >> shift) & 0x7f));
				}
				scale_codes.push(u64::from(sign_scale >> 28));
			}
			vec![
				delta,
				unsigned_values("grid_indices", "u8", 255, grid_indices),
				unsigned_values("sign_codes", "u7", 127, sign_codes),
				unsigned_values("scale_codes", "u4", 15, scale_codes),
			]
		}
		GgufTensorType::Iq2Xs => {
			let delta = f16_from(&mut reader, "delta")?;
			let mut grid_indices = Vec::with_capacity(32);
			let mut sign_codes = Vec::with_capacity(32);
			for _ in 0..32 {
				let code = reader.u16("grid and sign code")?;
				grid_indices.push(u64::from(code & 0x01ff));
				sign_codes.push(u64::from(code >> 9));
			}
			vec![
				delta,
				unsigned_values("grid_indices", "u9", 511, grid_indices),
				unsigned_values("sign_codes", "u7", 127, sign_codes),
				unsigned_values(
					"scale_codes",
					"u4",
					15,
					unpack_lsb(reader.take(8, "scale codes")?, 4),
				),
			]
		}
		GgufTensorType::Iq3Xxs => {
			let delta = f16_from(&mut reader, "delta")?;
			let grids = u8_array_from(&mut reader, "grid_indices", 64)?;
			let mut sign_codes = Vec::with_capacity(32);
			let mut scale_codes = Vec::with_capacity(8);
			for _ in 0..8 {
				let sign_scale = reader.u32("sign and scale word")?;
				for shift in [0_u32, 7, 14, 21] {
					sign_codes.push(u64::from((sign_scale >> shift) & 0x7f));
				}
				scale_codes.push(u64::from(sign_scale >> 28));
			}
			vec![
				delta,
				grids,
				unsigned_values("sign_codes", "u7", 127, sign_codes),
				unsigned_values("scale_codes", "u4", 15, scale_codes),
			]
		}
		GgufTensorType::Iq1S => {
			let delta = f16_from(&mut reader, "delta")?;
			let grids = u8_array_from(&mut reader, "grid_index_low", 32)?;
			let mut grid_high = Vec::with_capacity(32);
			let mut scale_codes = Vec::with_capacity(8);
			let mut shift_bits = Vec::with_capacity(8);
			for _ in 0..8 {
				let code = reader.u16("grid high, scale, and shift code")?;
				for shift in [0_u32, 3, 6, 9] {
					grid_high.push(u64::from((code >> shift) & 0x07));
				}
				scale_codes.push(u64::from((code >> 12) & 0x07));
				shift_bits.push(u64::from(code >> 15));
			}
			vec![
				delta,
				grids,
				unsigned_values("grid_index_high", "u3", 7, grid_high),
				unsigned_values("scale_codes", "u3", 7, scale_codes),
				unsigned_values("shift_bits", "u1", 1, shift_bits),
			]
		}
		GgufTensorType::Iq4Nl => {
			vec![
				f16_from(&mut reader, "delta")?,
				unsigned_values(
					"nonlinear_quant_codes",
					"u4",
					15,
					unpack_halves(reader.take(16, "nonlinear quant codes")?),
				),
			]
		}
		GgufTensorType::Iq3S => {
			vec![
				f16_from(&mut reader, "delta")?,
				u8_array_from(&mut reader, "grid_index_low", 64)?,
				unsigned_values(
					"grid_index_high_bits",
					"u1",
					1,
					unpack_lsb(reader.take(8, "grid index high bits")?, 1),
				),
				unsigned_values(
					"sign_bits",
					"u1",
					1,
					unpack_lsb(reader.take(32, "sign bits")?, 1),
				),
				unsigned_values(
					"scale_codes",
					"u4",
					15,
					unpack_lsb(reader.take(4, "scale codes")?, 4),
				),
			]
		}
		GgufTensorType::Iq2S => {
			vec![
				f16_from(&mut reader, "delta")?,
				u8_array_from(&mut reader, "grid_index_low", 32)?,
				unsigned_values(
					"sign_bits",
					"u1",
					1,
					unpack_lsb(reader.take(32, "sign bits")?, 1),
				),
				unsigned_values(
					"grid_index_high_codes",
					"u2",
					3,
					unpack_lsb(reader.take(8, "grid index high codes")?, 2),
				),
				unsigned_values(
					"scale_codes",
					"u4",
					15,
					unpack_lsb(reader.take(8, "scale codes")?, 4),
				),
			]
		}
		GgufTensorType::Iq4Xs => {
			let delta = f16_from(&mut reader, "delta")?;
			let scales_high = reader.u16("scale high bits")?;
			let scales_low = reader.take(4, "scale low bits")?;
			vec![
				delta,
				unsigned_values(
					"scale_codes",
					"u6",
					63,
					unpack_iq4xs_scales(scales_high, scales_low),
				),
				unsigned_values(
					"nonlinear_quant_codes",
					"u4",
					15,
					unpack_halves(reader.take(128, "nonlinear quant codes")?),
				),
			]
		}
		GgufTensorType::Iq1M => {
			let grids = u8_array_from(&mut reader, "grid_index_low", 32)?;
			let high_shift = reader.take(16, "grid high and shift codes")?;
			let mut grid_high = Vec::with_capacity(32);
			let mut shift_bits = Vec::with_capacity(32);
			for code in high_shift {
				grid_high.push(u64::from(code & 0x07));
				shift_bits.push(u64::from((code >> 3) & 1));
				grid_high.push(u64::from((code >> 4) & 0x07));
				shift_bits.push(u64::from(code >> 7));
			}
			let mut words = [0_u16; 4];
			for word in &mut words {
				*word = reader.u16("scale word")?;
			}
			let scale_bits = u64::from((words[0] >> 12) | ((words[1] >> 8) & 0x00f0) | ((words[2] >> 4) & 0x0f00) | (words[3] & 0xf000));
			let mut subblock_scales = Vec::with_capacity(16);
			for word in words {
				for shift in [0_u32, 3, 6, 9] {
					subblock_scales.push(u64::from((word >> shift) & 0x07));
				}
			}
			vec![
				grids,
				unsigned_values("grid_index_high", "u3", 7, grid_high),
				unsigned_values("shift_bits", "u1", 1, shift_bits),
				float_field("scale", FloatFormat::F16, scale_bits),
				unsigned_values("subblock_scale_codes", "u3", 7, subblock_scales),
			]
		}
		GgufTensorType::Tq1_0 => {
			vec![
				u8_array_from(&mut reader, "base3_pentad_codes", 48)?,
				u8_array_from(&mut reader, "base3_quad_codes", 4)?,
				f16_from(&mut reader, "delta")?,
			]
		}
		GgufTensorType::Tq2_0 => {
			vec![
				unsigned_values(
					"ternary_quant_codes",
					"u2",
					3,
					unpack_lsb(reader.take(64, "ternary quant codes")?, 2),
				),
				f16_from(&mut reader, "delta")?,
			]
		}
		GgufTensorType::Mxfp4 => {
			vec![
				unsigned_values("scale", "e8m0", 255, vec![u64::from(
					reader.u8("E8M0 scale")?,
				)]),
				unsigned_values(
					"quant_codes",
					"e2m1",
					15,
					unpack_halves(reader.take(16, "E2M1 quant codes")?),
				),
			]
		}
		GgufTensorType::Nvfp4 => {
			vec![
				unsigned_values(
					"subblock_scales",
					"ue4m3",
					255,
					reader.take(4, "UE4M3 subblock scales")?
						.iter()
						.map(|value| u64::from(*value))
						.collect(),
				),
				unsigned_values(
					"quant_codes",
					"e2m1",
					15,
					unpack_nvfp4(reader.take(32, "E2M1 quant codes")?),
				),
			]
		}
	};
	if reader.remaining() != 0 {
		return Err(invalid_structure(
			"tensor.payload",
			format!(
				"{:?} decoder left {} bytes",
				tensor_type,
				reader.remaining()
			),
		));
	}
	Ok(fields)
}

#[derive(Debug)]
struct BlockReader<'a> {
	bytes: &'a [u8],
	endian: GgufEndian,
	position: usize,
}

impl<'a> BlockReader<'a> {
	const fn new(bytes: &'a [u8], endian: GgufEndian) -> Self {
		Self {
			bytes,
			endian,
			position: 0,
		}
	}

	const fn remaining(&self) -> usize { self.bytes.len() - self.position }

	fn take(&mut self, length: usize, context: &str) -> GgufOgdlResult<&'a [u8]> {
		let end = self.position.checked_add(length).ok_or_else(|| {
			invalid_overflow(
				"tensor.payload",
				format!("{context} range overflowed usize"),
			)
		})?;
		let value = self.bytes.get(self.position..end).ok_or_else(|| {
			invalid_structure(
				"tensor.payload",
				format!(
					"{context} ends at {end}, block has {} bytes",
					self.bytes.len()
				),
			)
		})?;
		self.position = end;
		Ok(value)
	}

	fn u8(&mut self, context: &str) -> GgufOgdlResult<u8> { Ok(self.take(1, context)?[0]) }

	fn u16(&mut self, context: &str) -> GgufOgdlResult<u16> {
		let bytes = exact_array::<2>(self.take(2, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u16::from_le_bytes(bytes),
			GgufEndian::Big => u16::from_be_bytes(bytes),
		})
	}

	fn i16(&mut self, context: &str) -> GgufOgdlResult<i16> {
		let bytes = exact_array::<2>(self.take(2, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i16::from_le_bytes(bytes),
			GgufEndian::Big => i16::from_be_bytes(bytes),
		})
	}

	fn u32(&mut self, context: &str) -> GgufOgdlResult<u32> {
		let bytes = exact_array::<4>(self.take(4, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u32::from_le_bytes(bytes),
			GgufEndian::Big => u32::from_be_bytes(bytes),
		})
	}

	fn i32(&mut self, context: &str) -> GgufOgdlResult<i32> {
		let bytes = exact_array::<4>(self.take(4, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i32::from_le_bytes(bytes),
			GgufEndian::Big => i32::from_be_bytes(bytes),
		})
	}

	fn u64(&mut self, context: &str) -> GgufOgdlResult<u64> {
		let bytes = exact_array::<8>(self.take(8, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => u64::from_le_bytes(bytes),
			GgufEndian::Big => u64::from_be_bytes(bytes),
		})
	}

	fn i64(&mut self, context: &str) -> GgufOgdlResult<i64> {
		let bytes = exact_array::<8>(self.take(8, context)?, context)?;
		Ok(match self.endian {
			GgufEndian::Little => i64::from_le_bytes(bytes),
			GgufEndian::Big => i64::from_be_bytes(bytes),
		})
	}
}

fn exact_array<const N: usize>(bytes: &[u8], context: &str) -> GgufOgdlResult<[u8; N]> {
	bytes.try_into().map_err(|error| {
		invalid_structure(
			"tensor.payload",
			format!("{context} is not {N} bytes: {error}"),
		)
	})
}

fn unsigned_values(name: &'static str, kind: &'static str, maximum: u64, values: Vec<u64>) -> StructuralField {
	StructuralField {
		name,
		values: FieldValues::Unsigned {
			kind,
			maximum,
			values,
		},
	}
}

fn signed_values(name: &'static str, kind: &'static str, minimum: i64, maximum: i64, values: Vec<i64>) -> StructuralField {
	StructuralField {
		name,
		values: FieldValues::Signed {
			kind,
			minimum,
			maximum,
			values,
		},
	}
}

fn signed_field(name: &'static str, kind: &'static str, value: i64, minimum: i64, maximum: i64) -> StructuralField { signed_values(name, kind, minimum, maximum, vec![value]) }

fn float_field(name: &'static str, format: FloatFormat, bits: impl Into<u64>) -> StructuralField {
	StructuralField {
		name,
		values: FieldValues::Float {
			format,
			values: vec![split_float(bits.into(), format)],
		},
	}
}

fn f16_from(reader: &mut BlockReader<'_>, name: &'static str) -> GgufOgdlResult<StructuralField> {
	Ok(float_field(
		name,
		FloatFormat::F16,
		u64::from(reader.u16(name)?),
	))
}

fn f32_from(reader: &mut BlockReader<'_>, name: &'static str) -> GgufOgdlResult<StructuralField> {
	Ok(float_field(
		name,
		FloatFormat::F32,
		u64::from(reader.u32(name)?),
	))
}

fn u8_array_from(reader: &mut BlockReader<'_>, name: &'static str, count: usize) -> GgufOgdlResult<StructuralField> {
	Ok(unsigned_values(
		name,
		"u8",
		u64::from(u8::MAX),
		reader.take(count, name)?
			.iter()
			.map(|value| u64::from(*value))
			.collect(),
	))
}

fn i8_array_from(reader: &mut BlockReader<'_>, name: &'static str, count: usize) -> GgufOgdlResult<StructuralField> {
	Ok(signed_values(
		name,
		"i8",
		i64::from(i8::MIN),
		i64::from(i8::MAX),
		reader.take(count, name)?
			.iter()
			.map(|value| i64::from(i8::from_ne_bytes([*value])))
			.collect(),
	))
}

fn i16_array_from(reader: &mut BlockReader<'_>, name: &'static str, count: usize) -> GgufOgdlResult<StructuralField> {
	let values = (0..count)
		.map(|_| reader.i16(name).map(i64::from))
		.collect::<GgufOgdlResult<Vec<_>>>()?;
	Ok(signed_values(
		name,
		"i16",
		i64::from(i16::MIN),
		i64::from(i16::MAX),
		values,
	))
}

fn split_float(bits: u64, format: FloatFormat) -> FloatParts {
	let (width, exponent_width, fraction_width) = format.widths();
	let fraction_mask = (1_u64 << fraction_width) - 1;
	let exponent_mask = (1_u64 << exponent_width) - 1;
	FloatParts {
		sign: (bits >> (width - 1)) & 1,
		exponent: (bits >> fraction_width) & exponent_mask,
		fraction: bits & fraction_mask,
	}
}

fn join_float(parts: FloatParts, format: FloatFormat) -> GgufOgdlResult<u64> {
	let (width, exponent_width, fraction_width) = format.widths();
	let exponent_maximum = (1_u64 << exponent_width) - 1;
	let fraction_maximum = (1_u64 << fraction_width) - 1;
	if parts.sign > 1 || parts.exponent > exponent_maximum || parts.fraction > fraction_maximum {
		return Err(invalid_value(
			"tensor.payload",
			format!(
				"{} components exceed sign=1 exponent={exponent_maximum} fraction={fraction_maximum}",
				format.name()
			),
		));
	}
	Ok((parts.sign << (width - 1)) | (parts.exponent << fraction_width) | parts.fraction)
}

fn unpack_lsb(bytes: &[u8], width: u32) -> Vec<u64> {
	debug_assert!(width > 0 && 8 % width == 0);
	let per_byte = 8 / width;
	let mask = (1_u16 << width) - 1;
	let mut values = Vec::with_capacity(bytes.len() * usize::try_from(per_byte).unwrap_or(0));
	for byte in bytes {
		for index in 0..per_byte {
			values.push(u64::from((u16::from(*byte) >> (index * width)) & mask));
		}
	}
	values
}

fn unpack_halves(bytes: &[u8]) -> Vec<u64> {
	let mut values = Vec::with_capacity(bytes.len() * 2);
	values.extend(bytes.iter().map(|byte| u64::from(byte & 0x0f)));
	values.extend(bytes.iter().map(|byte| u64::from(byte >> 4)));
	values
}

fn unpack_nibble_pairs(bytes: &[u8]) -> (Vec<u64>, Vec<u64>) {
	(
		bytes.iter().map(|byte| u64::from(byte & 0x0f)).collect(),
		bytes.iter().map(|byte| u64::from(byte >> 4)).collect(),
	)
}

fn unpack_q3k_scales(bytes: &[u8]) -> Vec<u64> {
	debug_assert_eq!(bytes.len(), 12);
	let mut low = Vec::with_capacity(16);
	low.extend(bytes[..8].iter().map(|byte| u64::from(byte & 0x0f)));
	low.extend(bytes[..8].iter().map(|byte| u64::from(byte >> 4)));
	let mut high = Vec::with_capacity(16);
	for shift in [0_u32, 2, 4, 6] {
		high.extend(
			bytes[8..12]
				.iter()
				.map(|byte| u64::from((byte >> shift) & 0x03)),
		);
	}
	low.into_iter()
		.zip(high)
		.map(|(low, high)| low | (high << 4))
		.collect()
}

fn unpack_k_scale_min(bytes: &[u8]) -> (Vec<u64>, Vec<u64>) {
	debug_assert_eq!(bytes.len(), 12);
	let mut scales = Vec::with_capacity(8);
	let mut minimums = Vec::with_capacity(8);
	for index in 0..4 {
		scales.push(u64::from(bytes[index] & 0x3f));
		minimums.push(u64::from(bytes[index + 4] & 0x3f));
	}
	for index in 0..4 {
		scales.push(u64::from(
			(bytes[index + 8] & 0x0f) | ((bytes[index] >> 2) & 0x30),
		));
		minimums.push(u64::from(
			(bytes[index + 8] >> 4) | ((bytes[index + 4] >> 2) & 0x30),
		));
	}
	(scales, minimums)
}

fn unpack_iq4xs_scales(high: u16, low: &[u8]) -> Vec<u64> {
	debug_assert_eq!(low.len(), 4);
	(0..8).map(|index| {
		let low_code = (low[index / 2] >> (4 * (index % 2))) & 0x0f;
		let high_code = ((high >> (2 * index)) & 0x03) as u8;
		u64::from(low_code | (high_code << 4))
	})
	.collect()
}

fn unpack_nvfp4(bytes: &[u8]) -> Vec<u64> {
	debug_assert_eq!(bytes.len(), 32);
	let mut values = Vec::with_capacity(64);
	for subblock in bytes.as_chunks::<8>().0 {
		values.extend(subblock.iter().map(|byte| u64::from(byte & 0x0f)));
		values.extend(subblock.iter().map(|byte| u64::from(byte >> 4)));
	}
	values
}

const fn endian_u32_bytes(value: u32, endian: GgufEndian) -> [u8; 4] {
	match endian {
		GgufEndian::Little => value.to_le_bytes(),
		GgufEndian::Big => value.to_be_bytes(),
	}
}

const fn endian_u32(bytes: [u8; 4], endian: GgufEndian) -> u32 {
	match endian {
		GgufEndian::Little => u32::from_le_bytes(bytes),
		GgufEndian::Big => u32::from_be_bytes(bytes),
	}
}

fn parse_field_text(text: &str, template: &StructuralField, expected_count: Option<usize>, path: &str) -> GgufOgdlResult<StructuralField> {
	let mut tokens = text.split_whitespace();
	let name = tokens
		.next()
		.ok_or_else(|| invalid_value(path, "field is empty"))?;
	if name != template.name {
		return Err(invalid_value(
			path,
			format!("field is named {name:?}, expected {:?}", template.name),
		));
	}
	let kind = tokens
		.next()
		.ok_or_else(|| invalid_value(path, "field has no scalar type"))?;
	let parsed = match &template.values {
		FieldValues::Unsigned {
			kind: expected_kind,
			maximum,
			..
		} => {
			if kind != *expected_kind {
				return Err(invalid_value(
					path,
					format!("field type is {kind:?}, expected {expected_kind:?}"),
				));
			}
			let values = tokens
				.enumerate()
				.map(|(index, token)| {
					let value = parse_number::<u64>(token, &format!("{path}[{index}]"), expected_kind)?;
					if value > *maximum {
						return Err(invalid_value(
							format!("{path}[{index}]"),
							format!("{expected_kind} value {value} exceeds {maximum}"),
						));
					}
					Ok(value)
				})
				.collect::<GgufOgdlResult<Vec<_>>>()?;
			unsigned_values(template.name, expected_kind, *maximum, values)
		}
		FieldValues::Signed {
			kind: expected_kind,
			minimum,
			maximum,
			..
		} => {
			if kind != *expected_kind {
				return Err(invalid_value(
					path,
					format!("field type is {kind:?}, expected {expected_kind:?}"),
				));
			}
			let values = tokens
				.enumerate()
				.map(|(index, token)| {
					let value = parse_number::<i64>(token, &format!("{path}[{index}]"), expected_kind)?;
					if value < *minimum || value > *maximum {
						return Err(invalid_value(
							format!("{path}[{index}]"),
							format!("{expected_kind} value {value} lies outside {minimum}..={maximum}"),
						));
					}
					Ok(value)
				})
				.collect::<GgufOgdlResult<Vec<_>>>()?;
			signed_values(template.name, expected_kind, *minimum, *maximum, values)
		}
		FieldValues::Float { format, .. } => {
			if kind != format.name() {
				return Err(invalid_value(
					path,
					format!("field type is {kind:?}, expected {:?}", format.name()),
				));
			}
			let tokens = tokens.collect::<Vec<_>>();
			if tokens.len() % 3 != 0 {
				return Err(invalid_value(
					path,
					"floating field requires sign exponent fraction triples",
				));
			}
			let values = tokens
				.as_chunks::<3>()
				.0
				.iter()
				.enumerate()
				.map(|(index, parts)| parse_float_parts(&parts.join(" "), *format, &format!("{path}[{index}]")))
				.collect::<GgufOgdlResult<Vec<_>>>()?;
			StructuralField {
				name: template.name,
				values: FieldValues::Float {
					format: *format,
					values,
				},
			}
		}
	};
	let count = field_len(&parsed);
	if expected_count.is_some_and(|expected| count != expected) {
		return Err(invalid_value(
			path,
			format!(
				"field has {count} values, expected {}",
				expected_count.unwrap_or(0)
			),
		));
	}
	Ok(parsed)
}

fn field_len(field: &StructuralField) -> usize {
	match &field.values {
		FieldValues::Unsigned { values, .. } => values.len(),
		FieldValues::Signed { values, .. } => values.len(),
		FieldValues::Float { values, .. } => values.len(),
	}
}

fn field_value_at(field: &StructuralField, index: usize) -> GgufOgdlResult<StructuralField> {
	let values = match &field.values {
		FieldValues::Unsigned {
			kind,
			maximum,
			values,
		} => {
			FieldValues::Unsigned {
				kind,
				maximum: *maximum,
				values: vec![*values
					.get(index)
					.ok_or_else(|| invalid_structure("tensor.payload", "missing scalar value"))?],
			}
		}
		FieldValues::Signed {
			kind,
			minimum,
			maximum,
			values,
		} => {
			FieldValues::Signed {
				kind,
				minimum: *minimum,
				maximum: *maximum,
				values: vec![*values
					.get(index)
					.ok_or_else(|| invalid_structure("tensor.payload", "missing scalar value"))?],
			}
		}
		FieldValues::Float { format, values } => {
			FieldValues::Float {
				format: *format,
				values: vec![*values
					.get(index)
					.ok_or_else(|| invalid_structure("tensor.payload", "missing scalar value"))?],
			}
		}
	};
	Ok(StructuralField {
		name: field.name,
		values,
	})
}

fn encode_block(tensor_type: GgufTensorType, endian: GgufEndian, fields: &[StructuralField]) -> GgufOgdlResult<Vec<u8>> {
	let mut writer = BlockWriter::new(endian);
	match tensor_type {
		GgufTensorType::F32 => {
			writer.float(
				FloatFormat::F32,
				float_values(fields, 0, "value", FloatFormat::F32, 1)?[0],
			)?
		}
		GgufTensorType::F16 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "value", FloatFormat::F16, 1)?[0],
			)?
		}
		GgufTensorType::Bf16 => {
			writer.float(
				FloatFormat::Bf16,
				float_values(fields, 0, "value", FloatFormat::Bf16, 1)?[0],
			)?
		}
		GgufTensorType::F64 => {
			writer.float(
				FloatFormat::F64,
				float_values(fields, 0, "value", FloatFormat::F64, 1)?[0],
			)?
		}
		GgufTensorType::I8 => {
			writer.i8(exact_signed::<i8>(
				signed_values_ref(fields, 0, "value", "i8", 1)?[0],
				"value",
			)?)
		}
		GgufTensorType::I16 => {
			writer.i16(exact_signed::<i16>(
				signed_values_ref(fields, 0, "value", "i16", 1)?[0],
				"value",
			)?)
		}
		GgufTensorType::I32 => {
			writer.i32(exact_signed::<i32>(
				signed_values_ref(fields, 0, "value", "i32", 1)?[0],
				"value",
			)?)
		}
		GgufTensorType::I64 => writer.i64(signed_values_ref(fields, 0, "value", "i64", 1)?[0]),
		GgufTensorType::Q1_0 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 1, "sign_bits", "u1", 128)?,
				1,
			)?);
		}
		GgufTensorType::Q2_0 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 1, "quant_codes", "u2", 64)?,
				2,
			)?);
		}
		GgufTensorType::Q4_0 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				1,
				"quant_codes",
				"u4",
				32,
			)?)?);
		}
		GgufTensorType::Q4_1 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 1, "minimum", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				2,
				"quant_codes",
				"u4",
				32,
			)?)?);
		}
		GgufTensorType::Q5_0 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 1, "quant_high_bits", "u1", 32)?,
				1,
			)?);
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				2,
				"quant_low_codes",
				"u4",
				32,
			)?)?);
		}
		GgufTensorType::Q5_1 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 1, "minimum", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 2, "quant_high_bits", "u1", 32)?,
				1,
			)?);
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				3,
				"quant_low_codes",
				"u4",
				32,
			)?)?);
		}
		GgufTensorType::Q8_0 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.signed_array::<i8>(
				signed_values_ref(fields, 1, "quant_codes", "i8", 32)?,
				"quant_codes",
			)?;
		}
		GgufTensorType::Q8_1 => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 1, "scaled_sum", FloatFormat::F16, 1)?[0],
			)?;
			writer.signed_array::<i8>(
				signed_values_ref(fields, 2, "quant_codes", "i8", 32)?,
				"quant_codes",
			)?;
		}
		GgufTensorType::Q2K => {
			writer.bytes.extend(pack_nibble_pairs(
				unsigned_values_ref(fields, 0, "scale_codes", "u4", 16)?,
				unsigned_values_ref(fields, 1, "minimum_codes", "u4", 16)?,
			)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 2, "quant_bitplanes", "u2", 256)?,
				2,
			)?);
			writer.float(
				FloatFormat::F16,
				float_values(fields, 3, "scale", FloatFormat::F16, 1)?[0],
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 4, "minimum_scale", FloatFormat::F16, 1)?[0],
			)?;
		}
		GgufTensorType::Q3K => {
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 0, "quant_high_masks", "u1", 256)?,
				1,
			)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 1, "quant_low_codes", "u2", 256)?,
				2,
			)?);
			writer.bytes.extend(pack_q3k_scales(unsigned_values_ref(
				fields,
				2,
				"scale_codes",
				"u6",
				16,
			)?)?);
			writer.float(
				FloatFormat::F16,
				float_values(fields, 3, "scale", FloatFormat::F16, 1)?[0],
			)?;
		}
		GgufTensorType::Q4K => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "scale", FloatFormat::F16, 1)?[0],
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 1, "minimum_scale", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_k_scale_min(
				unsigned_values_ref(fields, 2, "scale_codes", "u6", 8)?,
				unsigned_values_ref(fields, 3, "minimum_codes", "u6", 8)?,
			)?);
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				4,
				"quant_codes",
				"u4",
				256,
			)?)?);
		}
		GgufTensorType::Q5K => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "scale", FloatFormat::F16, 1)?[0],
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 1, "minimum_scale", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_k_scale_min(
				unsigned_values_ref(fields, 2, "scale_codes", "u6", 8)?,
				unsigned_values_ref(fields, 3, "minimum_codes", "u6", 8)?,
			)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 4, "quant_high_bitplanes", "u1", 256)?,
				1,
			)?);
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				5,
				"quant_low_codes",
				"u4",
				256,
			)?)?);
		}
		GgufTensorType::Q6K => {
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				0,
				"quant_low_codes",
				"u4",
				256,
			)?)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 1, "quant_high_codes", "u2", 256)?,
				2,
			)?);
			writer.signed_array::<i8>(
				signed_values_ref(fields, 2, "scale_codes", "i8", 16)?,
				"scale_codes",
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 3, "scale", FloatFormat::F16, 1)?[0],
			)?;
		}
		GgufTensorType::Q8K => {
			writer.float(
				FloatFormat::F32,
				float_values(fields, 0, "delta", FloatFormat::F32, 1)?[0],
			)?;
			writer.signed_array::<i8>(
				signed_values_ref(fields, 1, "quant_codes", "i8", 256)?,
				"quant_codes",
			)?;
			writer.signed_array::<i16>(
				signed_values_ref(fields, 2, "block_sums", "i16", 16)?,
				"block_sums",
			)?;
		}
		GgufTensorType::Iq2Xxs => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			let grids = unsigned_values_ref(fields, 1, "grid_indices", "u8", 32)?;
			let signs = unsigned_values_ref(fields, 2, "sign_codes", "u7", 32)?;
			let scales = unsigned_values_ref(fields, 3, "scale_codes", "u4", 8)?;
			for group in 0..8 {
				let raw_grids = grids[group * 4..group * 4 + 4]
					.iter()
					.map(|value| u8::try_from(*value).map_err(|error| invalid_value("grid_indices", error.to_string())))
					.collect::<GgufOgdlResult<Vec<_>>>()?;
				writer.u32(endian_u32(
					exact_array::<4>(&raw_grids, "grid indices")?,
					endian,
				));
				let mut sign_scale = u32::try_from(scales[group]).map_err(|error| invalid_value("scale_codes", error.to_string()))? << 28;
				for lane in 0..4 {
					sign_scale |= u32::try_from(signs[group * 4 + lane]).map_err(|error| invalid_value("sign_codes", error.to_string()))? << (7 * lane);
				}
				writer.u32(sign_scale);
			}
		}
		GgufTensorType::Iq2Xs => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			let grids = unsigned_values_ref(fields, 1, "grid_indices", "u9", 32)?;
			let signs = unsigned_values_ref(fields, 2, "sign_codes", "u7", 32)?;
			for (grid, sign) in grids.iter().copied().zip(signs.iter().copied()) {
				writer.u16(u16::try_from(grid | (sign << 9)).map_err(|error| invalid_value("grid and sign code", error.to_string()))?);
			}
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 3, "scale_codes", "u4", 16)?,
				4,
			)?);
		}
		GgufTensorType::Iq3Xxs => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 1, "grid_indices", "u8", 64)?,
				"grid_indices",
			)?;
			let signs = unsigned_values_ref(fields, 2, "sign_codes", "u7", 32)?;
			let scales = unsigned_values_ref(fields, 3, "scale_codes", "u4", 8)?;
			for group in 0..8 {
				let mut sign_scale = u32::try_from(scales[group]).map_err(|error| invalid_value("scale_codes", error.to_string()))? << 28;
				for lane in 0..4 {
					sign_scale |= u32::try_from(signs[group * 4 + lane]).map_err(|error| invalid_value("sign_codes", error.to_string()))? << (7 * lane);
				}
				writer.u32(sign_scale);
			}
		}
		GgufTensorType::Iq1S => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 1, "grid_index_low", "u8", 32)?,
				"grid_index_low",
			)?;
			let grid_high = unsigned_values_ref(fields, 2, "grid_index_high", "u3", 32)?;
			let scales = unsigned_values_ref(fields, 3, "scale_codes", "u3", 8)?;
			let shifts = unsigned_values_ref(fields, 4, "shift_bits", "u1", 8)?;
			for group in 0..8 {
				let mut code = u16::try_from(scales[group] | (shifts[group] << 3)).map_err(|error| invalid_value("scale and shift code", error.to_string()))? << 12;
				for lane in 0..4 {
					code |= u16::try_from(grid_high[group * 4 + lane]).map_err(|error| invalid_value("grid_index_high", error.to_string()))? << (3 * lane);
				}
				writer.u16(code);
			}
		}
		GgufTensorType::Iq4Nl => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				1,
				"nonlinear_quant_codes",
				"u4",
				32,
			)?)?);
		}
		GgufTensorType::Iq3S => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 1, "grid_index_low", "u8", 64)?,
				"grid_index_low",
			)?;
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 2, "grid_index_high_bits", "u1", 64)?,
				1,
			)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 3, "sign_bits", "u1", 256)?,
				1,
			)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 4, "scale_codes", "u4", 8)?,
				4,
			)?);
		}
		GgufTensorType::Iq2S => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 1, "grid_index_low", "u8", 32)?,
				"grid_index_low",
			)?;
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 2, "sign_bits", "u1", 256)?,
				1,
			)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 3, "grid_index_high_codes", "u2", 32)?,
				2,
			)?);
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 4, "scale_codes", "u4", 16)?,
				4,
			)?);
		}
		GgufTensorType::Iq4Xs => {
			writer.float(
				FloatFormat::F16,
				float_values(fields, 0, "delta", FloatFormat::F16, 1)?[0],
			)?;
			let (high, low) = pack_iq4xs_scales(unsigned_values_ref(fields, 1, "scale_codes", "u6", 8)?)?;
			writer.u16(high);
			writer.bytes.extend(low);
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				2,
				"nonlinear_quant_codes",
				"u4",
				256,
			)?)?);
		}
		GgufTensorType::Iq1M => {
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 0, "grid_index_low", "u8", 32)?,
				"grid_index_low",
			)?;
			let grid_high = unsigned_values_ref(fields, 1, "grid_index_high", "u3", 32)?;
			let shifts = unsigned_values_ref(fields, 2, "shift_bits", "u1", 32)?;
			for pair in 0..16 {
				let first = pair * 2;
				writer.bytes
					.push(u8::try_from(grid_high[first] | (shifts[first] << 3) | (grid_high[first + 1] << 4) | (shifts[first + 1] << 7)).map_err(|error| invalid_value("grid high and shift codes", error.to_string()))?);
			}
			let scale_parts = float_values(fields, 3, "scale", FloatFormat::F16, 1)?[0];
			let scale_bits = u16::try_from(join_float(scale_parts, FloatFormat::F16)?).map_err(|error| invalid_value("scale", error.to_string()))?;
			let subblock_scales = unsigned_values_ref(fields, 4, "subblock_scale_codes", "u3", 16)?;
			for word_index in 0..4 {
				let mut word = 0_u16;
				for code_index in 0..4 {
					word |= u16::try_from(subblock_scales[word_index * 4 + code_index]).map_err(|error| invalid_value("subblock_scale_codes", error.to_string()))? << (3 * code_index);
				}
				word |= match word_index {
					0 => (scale_bits & 0x000f) << 12,
					1 => (scale_bits & 0x00f0) << 8,
					2 => (scale_bits & 0x0f00) << 4,
					3 => scale_bits & 0xf000,
					_ => unreachable!(),
				};
				writer.u16(word);
			}
		}
		GgufTensorType::Tq1_0 => {
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 0, "base3_pentad_codes", "u8", 48)?,
				"base3_pentad_codes",
			)?;
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 1, "base3_quad_codes", "u8", 4)?,
				"base3_quad_codes",
			)?;
			writer.float(
				FloatFormat::F16,
				float_values(fields, 2, "delta", FloatFormat::F16, 1)?[0],
			)?;
		}
		GgufTensorType::Tq2_0 => {
			writer.bytes.extend(pack_lsb(
				unsigned_values_ref(fields, 0, "ternary_quant_codes", "u2", 256)?,
				2,
			)?);
			writer.float(
				FloatFormat::F16,
				float_values(fields, 1, "delta", FloatFormat::F16, 1)?[0],
			)?;
		}
		GgufTensorType::Mxfp4 => {
			writer.unsigned_array::<u8>(unsigned_values_ref(fields, 0, "scale", "e8m0", 1)?, "scale")?;
			writer.bytes.extend(pack_halves(unsigned_values_ref(
				fields,
				1,
				"quant_codes",
				"e2m1",
				32,
			)?)?);
		}
		GgufTensorType::Nvfp4 => {
			writer.unsigned_array::<u8>(
				unsigned_values_ref(fields, 0, "subblock_scales", "ue4m3", 4)?,
				"subblock_scales",
			)?;
			writer.bytes.extend(pack_nvfp4(unsigned_values_ref(
				fields,
				1,
				"quant_codes",
				"e2m1",
				64,
			)?)?);
		}
	}
	if fields.len()
		!= decode_block(tensor_type, endian, &vec![
			0;
			host_usize(
				tensor_type.block_bytes(),
				"block bytes"
			)?
		])?
		.len()
	{
		return Err(invalid_structure(
			"tensor.payload",
			"block contains unexpected extra fields",
		));
	}
	let expected = host_usize(tensor_type.block_bytes(), "tensor block bytes")?;
	if writer.bytes.len() != expected {
		return Err(invalid_structure(
			"tensor.payload",
			format!(
				"encoded {:?} block has {} bytes, expected {expected}",
				tensor_type,
				writer.bytes.len()
			),
		));
	}
	Ok(writer.bytes)
}

#[derive(Debug)]
struct BlockWriter {
	bytes: Vec<u8>,
	endian: GgufEndian,
}

impl BlockWriter {
	const fn new(endian: GgufEndian) -> Self {
		Self {
			bytes: Vec::new(),
			endian,
		}
	}

	fn u16(&mut self, value: u16) {
		self.bytes.extend_from_slice(&match self.endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		});
	}

	fn i16(&mut self, value: i16) {
		self.bytes.extend_from_slice(&match self.endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		});
	}

	fn u32(&mut self, value: u32) {
		self.bytes.extend_from_slice(&match self.endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		});
	}

	fn i32(&mut self, value: i32) {
		self.bytes.extend_from_slice(&match self.endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		});
	}

	fn u64(&mut self, value: u64) {
		self.bytes.extend_from_slice(&match self.endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		});
	}

	fn i64(&mut self, value: i64) {
		self.bytes.extend_from_slice(&match self.endian {
			GgufEndian::Little => value.to_le_bytes(),
			GgufEndian::Big => value.to_be_bytes(),
		});
	}

	fn i8(&mut self, value: i8) { self.bytes.extend_from_slice(&value.to_ne_bytes()); }

	fn float(&mut self, format: FloatFormat, value: FloatParts) -> GgufOgdlResult<()> {
		let bits = join_float(value, format)?;
		match format {
			FloatFormat::F16 | FloatFormat::Bf16 => self.u16(u16::try_from(bits).map_err(|error| invalid_value("tensor.payload", error.to_string()))?),
			FloatFormat::F32 => self.u32(u32::try_from(bits).map_err(|error| invalid_value("tensor.payload", error.to_string()))?),
			FloatFormat::F64 => self.u64(bits),
		}
		Ok(())
	}

	fn unsigned_array<T>(&mut self, values: &[u64], path: &str) -> GgufOgdlResult<()>
	where
		T: TryFrom<u64> + Copy,
		<T as TryFrom<u64>>::Error: fmt::Display,
		Self: WriteUnsigned<T>,
	{
		for value in values {
			let converted = T::try_from(*value).map_err(|error| invalid_value(path, error.to_string()))?;
			<Self as WriteUnsigned<T>>::write_unsigned(self, converted);
		}
		Ok(())
	}

	fn signed_array<T>(&mut self, values: &[i64], path: &str) -> GgufOgdlResult<()>
	where
		T: TryFrom<i64> + Copy,
		<T as TryFrom<i64>>::Error: fmt::Display,
		Self: WriteSigned<T>,
	{
		for value in values {
			let converted = T::try_from(*value).map_err(|error| invalid_value(path, error.to_string()))?;
			<Self as WriteSigned<T>>::write_signed(self, converted);
		}
		Ok(())
	}
}

trait WriteUnsigned<T> {
	fn write_unsigned(&mut self, value: T);
}

impl WriteUnsigned<u8> for BlockWriter {
	fn write_unsigned(&mut self, value: u8) { self.bytes.push(value); }
}

impl WriteUnsigned<u16> for BlockWriter {
	fn write_unsigned(&mut self, value: u16) { self.u16(value); }
}

impl WriteUnsigned<u32> for BlockWriter {
	fn write_unsigned(&mut self, value: u32) { self.u32(value); }
}

trait WriteSigned<T> {
	fn write_signed(&mut self, value: T);
}

impl WriteSigned<i8> for BlockWriter {
	fn write_signed(&mut self, value: i8) { self.i8(value); }
}

impl WriteSigned<i16> for BlockWriter {
	fn write_signed(&mut self, value: i16) { self.i16(value); }
}

fn unsigned_values_ref<'a>(fields: &'a [StructuralField], index: usize, name: &str, kind: &str, count: usize) -> GgufOgdlResult<&'a [u64]> {
	let field = fields
		.get(index)
		.ok_or_else(|| invalid_structure("tensor.payload", format!("missing field {name:?}")))?;
	if field.name != name {
		return Err(invalid_structure(
			"tensor.payload",
			format!("field {index} is {:?}, expected {name:?}", field.name),
		));
	}
	let FieldValues::Unsigned {
		kind: actual_kind,
		values,
		..
	} = &field.values
	else {
		return Err(invalid_structure(
			"tensor.payload",
			format!("field {name:?} is not unsigned"),
		));
	};
	if *actual_kind != kind || values.len() != count {
		return Err(invalid_structure(
			"tensor.payload",
			format!(
				"field {name:?} is {actual_kind}[{}], expected {kind}[{count}]",
				values.len()
			),
		));
	}
	Ok(values)
}

fn signed_values_ref<'a>(fields: &'a [StructuralField], index: usize, name: &str, kind: &str, count: usize) -> GgufOgdlResult<&'a [i64]> {
	let field = fields
		.get(index)
		.ok_or_else(|| invalid_structure("tensor.payload", format!("missing field {name:?}")))?;
	if field.name != name {
		return Err(invalid_structure(
			"tensor.payload",
			format!("field {index} is {:?}, expected {name:?}", field.name),
		));
	}
	let FieldValues::Signed {
		kind: actual_kind,
		values,
		..
	} = &field.values
	else {
		return Err(invalid_structure(
			"tensor.payload",
			format!("field {name:?} is not signed"),
		));
	};
	if *actual_kind != kind || values.len() != count {
		return Err(invalid_structure(
			"tensor.payload",
			format!(
				"field {name:?} is {actual_kind}[{}], expected {kind}[{count}]",
				values.len()
			),
		));
	}
	Ok(values)
}

fn float_values<'a>(fields: &'a [StructuralField], index: usize, name: &str, format: FloatFormat, count: usize) -> GgufOgdlResult<&'a [FloatParts]> {
	let field = fields
		.get(index)
		.ok_or_else(|| invalid_structure("tensor.payload", format!("missing field {name:?}")))?;
	if field.name != name {
		return Err(invalid_structure(
			"tensor.payload",
			format!("field {index} is {:?}, expected {name:?}", field.name),
		));
	}
	let FieldValues::Float {
		format: actual_format,
		values,
	} = &field.values
	else {
		return Err(invalid_structure(
			"tensor.payload",
			format!("field {name:?} is not floating"),
		));
	};
	if *actual_format != format || values.len() != count {
		return Err(invalid_structure(
			"tensor.payload",
			format!(
				"field {name:?} is {}[{}], expected {}[{count}]",
				actual_format.name(),
				values.len(),
				format.name()
			),
		));
	}
	Ok(values)
}

fn exact_signed<T>(value: i64, path: &str) -> GgufOgdlResult<T>
where
	T: TryFrom<i64>,
	<T as TryFrom<i64>>::Error: fmt::Display,
{
	T::try_from(value).map_err(|error| invalid_value(path, error.to_string()))
}

fn pack_lsb(values: &[u64], width: u32) -> GgufOgdlResult<Vec<u8>> {
	if width == 0 || 8 % width != 0 {
		return Err(invalid_structure(
			"tensor.payload",
			format!("invalid packed width {width}"),
		));
	}
	let per_byte = usize::try_from(8 / width).map_err(|error| invalid_overflow("tensor.payload", error.to_string()))?;
	if !values.len().is_multiple_of(per_byte) {
		return Err(invalid_structure(
			"tensor.payload",
			format!(
				"{} values do not fill {width}-bit storage bytes",
				values.len()
			),
		));
	}
	let maximum = (1_u64 << width) - 1;
	values.chunks_exact(per_byte)
		.map(|chunk| {
			let mut byte = 0_u8;
			for (index, value) in chunk.iter().copied().enumerate() {
				if value > maximum {
					return Err(invalid_value(
						"tensor.payload",
						format!("packed value {value} exceeds {maximum}"),
					));
				}
				byte |= u8::try_from(value).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << (u32::try_from(index).map_err(|error| invalid_overflow("tensor.payload", error.to_string()))? * width);
			}
			Ok(byte)
		})
		.collect()
}

fn pack_halves(values: &[u64]) -> GgufOgdlResult<Vec<u8>> {
	if !values.len().is_multiple_of(2) {
		return Err(invalid_structure(
			"tensor.payload",
			"nibble halves require an even value count",
		));
	}
	let half = values.len() / 2;
	values[..half]
		.iter()
		.copied()
		.zip(values[half..].iter().copied())
		.map(|(low, high)| {
			if low > 15 || high > 15 {
				return Err(invalid_value("tensor.payload", "nibble value exceeds 15"));
			}
			u8::try_from(low | (high << 4)).map_err(|error| invalid_value("tensor.payload", error.to_string()))
		})
		.collect()
}

fn pack_nibble_pairs(low: &[u64], high: &[u64]) -> GgufOgdlResult<Vec<u8>> {
	if low.len() != high.len() {
		return Err(invalid_structure(
			"tensor.payload",
			"nibble pair lengths differ",
		));
	}
	low.iter()
		.copied()
		.zip(high.iter().copied())
		.map(|(low, high)| {
			if low > 15 || high > 15 {
				return Err(invalid_value("tensor.payload", "nibble value exceeds 15"));
			}
			u8::try_from(low | (high << 4)).map_err(|error| invalid_value("tensor.payload", error.to_string()))
		})
		.collect()
}

fn pack_q3k_scales(values: &[u64]) -> GgufOgdlResult<Vec<u8>> {
	if values.len() != 16 || values.iter().any(|value| *value > 63) {
		return Err(invalid_value(
			"tensor.payload.scale_codes",
			"Q3_K requires sixteen u6 scale codes",
		));
	}
	let mut bytes = vec![0_u8; 12];
	for index in 0..8 {
		bytes[index] = u8::try_from(values[index] & 0x0f).map_err(|error| invalid_value("tensor.payload", error.to_string()))? | (u8::try_from(values[index + 8] & 0x0f).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << 4);
	}
	for group in 0..4 {
		for index in 0..4 {
			let scale_index = group * 4 + index;
			bytes[8 + index] |= u8::try_from((values[scale_index] >> 4) & 0x03).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << (2 * group);
		}
	}
	Ok(bytes)
}

fn pack_k_scale_min(scales: &[u64], minimums: &[u64]) -> GgufOgdlResult<Vec<u8>> {
	if scales.len() != 8 || minimums.len() != 8 || scales.iter().chain(minimums).any(|value| *value > 63) {
		return Err(invalid_value(
			"tensor.payload.scale_codes",
			"K-quant scale layout requires eight u6 scales and eight u6 minimums",
		));
	}
	let mut bytes = vec![0_u8; 12];
	for index in 0..4 {
		bytes[index] = u8::try_from(scales[index] & 0x3f).map_err(|error| invalid_value("tensor.payload", error.to_string()))?;
		bytes[index + 4] = u8::try_from(minimums[index] & 0x3f).map_err(|error| invalid_value("tensor.payload", error.to_string()))?;
		bytes[index + 8] = u8::try_from(scales[index + 4] & 0x0f).map_err(|error| invalid_value("tensor.payload", error.to_string()))? | (u8::try_from(minimums[index + 4] & 0x0f).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << 4);
		bytes[index] |= u8::try_from((scales[index + 4] >> 4) & 0x03).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << 6;
		bytes[index + 4] |= u8::try_from((minimums[index + 4] >> 4) & 0x03).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << 6;
	}
	Ok(bytes)
}

fn pack_iq4xs_scales(values: &[u64]) -> GgufOgdlResult<(u16, Vec<u8>)> {
	if values.len() != 8 || values.iter().any(|value| *value > 63) {
		return Err(invalid_value(
			"tensor.payload.scale_codes",
			"IQ4_XS requires eight u6 scale codes",
		));
	}
	let mut high = 0_u16;
	let mut low = vec![0_u8; 4];
	for (index, value) in values.iter().copied().enumerate() {
		low[index / 2] |= u8::try_from(value & 0x0f).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << (4 * (index % 2));
		high |= u16::try_from((value >> 4) & 0x03).map_err(|error| invalid_value("tensor.payload", error.to_string()))? << (2 * index);
	}
	Ok((high, low))
}

fn pack_nvfp4(values: &[u64]) -> GgufOgdlResult<Vec<u8>> {
	if values.len() != 64 || values.iter().any(|value| *value > 15) {
		return Err(invalid_value(
			"tensor.payload.quant_codes",
			"NVFP4 requires sixty-four E2M1 codes",
		));
	}
	let mut bytes = Vec::with_capacity(32);
	for subblock in values.as_chunks::<16>().0 {
		for index in 0..8 {
			bytes.push(u8::try_from(subblock[index] | (subblock[index + 8] << 4)).map_err(|error| invalid_value("tensor.payload", error.to_string()))?);
		}
	}
	Ok(bytes)
}

fn metadata_type_name(value_type: GgufMetadataType) -> &'static str {
	match value_type {
		GgufMetadataType::U8 => "u8",
		GgufMetadataType::I8 => "i8",
		GgufMetadataType::U16 => "u16",
		GgufMetadataType::I16 => "i16",
		GgufMetadataType::U32 => "u32",
		GgufMetadataType::I32 => "i32",
		GgufMetadataType::F32 => "f32",
		GgufMetadataType::Bool => "bool",
		GgufMetadataType::String => "string",
		GgufMetadataType::Array => "array",
		GgufMetadataType::U64 => "u64",
		GgufMetadataType::I64 => "i64",
		GgufMetadataType::F64 => "f64",
	}
}

fn parse_metadata_type(name: &str, path: &str) -> GgufOgdlResult<GgufMetadataType> {
	match name {
		"u8" => Ok(GgufMetadataType::U8),
		"i8" => Ok(GgufMetadataType::I8),
		"u16" => Ok(GgufMetadataType::U16),
		"i16" => Ok(GgufMetadataType::I16),
		"u32" => Ok(GgufMetadataType::U32),
		"i32" => Ok(GgufMetadataType::I32),
		"f32" => Ok(GgufMetadataType::F32),
		"bool" => Ok(GgufMetadataType::Bool),
		"string" => Ok(GgufMetadataType::String),
		"array" => Ok(GgufMetadataType::Array),
		"u64" => Ok(GgufMetadataType::U64),
		"i64" => Ok(GgufMetadataType::I64),
		"f64" => Ok(GgufMetadataType::F64),
		other => {
			Err(invalid_value(
				path,
				format!("unknown metadata type {other:?}"),
			))
		}
	}
}

fn tensor_type_name(tensor_type: GgufTensorType) -> &'static str {
	match tensor_type {
		GgufTensorType::F32 => "f32",
		GgufTensorType::F16 => "f16",
		GgufTensorType::Q4_0 => "q4_0",
		GgufTensorType::Q4_1 => "q4_1",
		GgufTensorType::Q5_0 => "q5_0",
		GgufTensorType::Q5_1 => "q5_1",
		GgufTensorType::Q8_0 => "q8_0",
		GgufTensorType::Q8_1 => "q8_1",
		GgufTensorType::Q2K => "q2_k",
		GgufTensorType::Q3K => "q3_k",
		GgufTensorType::Q4K => "q4_k",
		GgufTensorType::Q5K => "q5_k",
		GgufTensorType::Q6K => "q6_k",
		GgufTensorType::Q8K => "q8_k",
		GgufTensorType::Iq2Xxs => "iq2_xxs",
		GgufTensorType::Iq2Xs => "iq2_xs",
		GgufTensorType::Iq3Xxs => "iq3_xxs",
		GgufTensorType::Iq1S => "iq1_s",
		GgufTensorType::Iq4Nl => "iq4_nl",
		GgufTensorType::Iq3S => "iq3_s",
		GgufTensorType::Iq2S => "iq2_s",
		GgufTensorType::Iq4Xs => "iq4_xs",
		GgufTensorType::I8 => "i8",
		GgufTensorType::I16 => "i16",
		GgufTensorType::I32 => "i32",
		GgufTensorType::I64 => "i64",
		GgufTensorType::F64 => "f64",
		GgufTensorType::Iq1M => "iq1_m",
		GgufTensorType::Bf16 => "bf16",
		GgufTensorType::Tq1_0 => "tq1_0",
		GgufTensorType::Tq2_0 => "tq2_0",
		GgufTensorType::Mxfp4 => "mxfp4",
		GgufTensorType::Nvfp4 => "nvfp4",
		GgufTensorType::Q1_0 => "q1_0",
		GgufTensorType::Q2_0 => "q2_0",
	}
}

fn parse_tensor_type(name: &str, path: &str) -> GgufOgdlResult<GgufTensorType> {
	match name {
		"f32" => Ok(GgufTensorType::F32),
		"f16" => Ok(GgufTensorType::F16),
		"q4_0" => Ok(GgufTensorType::Q4_0),
		"q4_1" => Ok(GgufTensorType::Q4_1),
		"q5_0" => Ok(GgufTensorType::Q5_0),
		"q5_1" => Ok(GgufTensorType::Q5_1),
		"q8_0" => Ok(GgufTensorType::Q8_0),
		"q8_1" => Ok(GgufTensorType::Q8_1),
		"q2_k" => Ok(GgufTensorType::Q2K),
		"q3_k" => Ok(GgufTensorType::Q3K),
		"q4_k" => Ok(GgufTensorType::Q4K),
		"q5_k" => Ok(GgufTensorType::Q5K),
		"q6_k" => Ok(GgufTensorType::Q6K),
		"q8_k" => Ok(GgufTensorType::Q8K),
		"iq2_xxs" => Ok(GgufTensorType::Iq2Xxs),
		"iq2_xs" => Ok(GgufTensorType::Iq2Xs),
		"iq3_xxs" => Ok(GgufTensorType::Iq3Xxs),
		"iq1_s" => Ok(GgufTensorType::Iq1S),
		"iq4_nl" => Ok(GgufTensorType::Iq4Nl),
		"iq3_s" => Ok(GgufTensorType::Iq3S),
		"iq2_s" => Ok(GgufTensorType::Iq2S),
		"iq4_xs" => Ok(GgufTensorType::Iq4Xs),
		"i8" => Ok(GgufTensorType::I8),
		"i16" => Ok(GgufTensorType::I16),
		"i32" => Ok(GgufTensorType::I32),
		"i64" => Ok(GgufTensorType::I64),
		"f64" => Ok(GgufTensorType::F64),
		"iq1_m" => Ok(GgufTensorType::Iq1M),
		"bf16" => Ok(GgufTensorType::Bf16),
		"tq1_0" => Ok(GgufTensorType::Tq1_0),
		"tq2_0" => Ok(GgufTensorType::Tq2_0),
		"mxfp4" => Ok(GgufTensorType::Mxfp4),
		"nvfp4" => Ok(GgufTensorType::Nvfp4),
		"q1_0" => Ok(GgufTensorType::Q1_0),
		"q2_0" => Ok(GgufTensorType::Q2_0),
		other => {
			Err(invalid_value(
				path,
				format!("unknown current GGUF tensor type {other:?}"),
			))
		}
	}
}

fn tensor_encoded_bytes(name: &str, dimensions: &[u64], tensor_type: GgufTensorType) -> GgufOgdlResult<u64> {
	let first = dimensions.first().copied().unwrap_or(1);
	let block_elements = tensor_type.block_elements();
	if first % block_elements != 0 {
		return Err(invalid_value(
			format!("tensors.{name}.dimensions[0]"),
			format!("extent {first} is not divisible by {block_elements} for {tensor_type:?}"),
		));
	}
	let elements = if dimensions.contains(&0) {
		0
	} else {
		dimensions.iter().try_fold(1_u64, |product, dimension| {
			product.checked_mul(*dimension).ok_or_else(|| {
				invalid_overflow(
					format!("tensors.{name}.dimensions"),
					"dimension product overflowed u64",
				)
			})
		})?
	};
	(elements / block_elements)
		.checked_mul(tensor_type.block_bytes())
		.ok_or_else(|| {
			invalid_overflow(
				format!("tensors.{name}.payload"),
				"encoded byte count overflowed u64",
			)
		})
}

fn strip_prefix<'a>(value: &'a str, prefix: &str, path: &str) -> GgufOgdlResult<&'a str> {
	value.strip_prefix(prefix)
		.filter(|suffix| !suffix.is_empty())
		.ok_or_else(|| invalid_structure(path, format!("expected text beginning with {prefix:?}")))
}

fn parse_single_u32(value: &str, path: &str) -> GgufOgdlResult<u32> {
	if value.contains(char::is_whitespace) {
		return Err(invalid_value(path, "expected one u32 value"));
	}
	parse_number(value, path, "u32")
}

fn parse_number<T>(value: &str, path: &str, type_name: &str) -> GgufOgdlResult<T>
where
	T: core::str::FromStr,
	<T as core::str::FromStr>::Err: fmt::Display,
{
	value.parse().map_err(|error| {
		invalid_value(
			path,
			format!("{value:?} is not a canonical {type_name}: {error}"),
		)
	})
}

fn parse_float_parts(value: &str, format: FloatFormat, path: &str) -> GgufOgdlResult<FloatParts> {
	let tokens = value.split_whitespace().collect::<Vec<_>>();
	if tokens.len() != 3 {
		return Err(invalid_value(
			path,
			format!("{} requires sign exponent fraction", format.name()),
		));
	}
	let parts = FloatParts {
		sign: parse_number(tokens[0], &format!("{path}.sign"), "u1")?,
		exponent: parse_number(tokens[1], &format!("{path}.exponent"), "unsigned exponent")?,
		fraction: parse_number(tokens[2], &format!("{path}.fraction"), "unsigned fraction")?,
	};
	join_float(parts, format).map_err(|error| invalid_value(path, error.detail))?;
	Ok(parts)
}

fn align_up(value: u64, alignment: u64) -> GgufOgdlResult<u64> {
	if alignment == 0 || !alignment.is_multiple_of(8) {
		return Err(invalid_value(
			"gguf.alignment",
			"alignment must be a nonzero multiple of eight",
		));
	}
	let remainder = value % alignment;
	match remainder {
		0 => Ok(value),
		_ => {
			value.checked_add(alignment - remainder)
				.ok_or_else(|| invalid_overflow("gguf.alignment", "alignment calculation overflowed u64"))
		}
	}
}

fn host_u64(value: usize, path: &str) -> GgufOgdlResult<u64> { u64::try_from(value).map_err(|error| invalid_overflow(path, error.to_string())) }

fn host_u32(value: usize, path: &str) -> GgufOgdlResult<u32> { u32::try_from(value).map_err(|error| invalid_overflow(path, error.to_string())) }

fn host_usize(value: u64, path: &str) -> GgufOgdlResult<usize> { usize::try_from(value).map_err(|error| invalid_overflow(path, error.to_string())) }

fn invalid_structure(path: impl Into<String>, detail: impl Into<String>) -> GgufOgdlError { GgufOgdlError::new(GgufOgdlErrorKind::InvalidStructure, path, detail) }

fn invalid_value(path: impl Into<String>, detail: impl Into<String>) -> GgufOgdlError { GgufOgdlError::new(GgufOgdlErrorKind::InvalidValue, path, detail) }

fn invalid_overflow(path: impl Into<String>, detail: impl Into<String>) -> GgufOgdlError { GgufOgdlError::new(GgufOgdlErrorKind::ArithmeticOverflow, path, detail) }
