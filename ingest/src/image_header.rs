use core::fmt;

/// Encoded image container recognized by Recipe ingestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EncodedImageFormat {
	Png,
	Jpeg,
	Gif87a,
	Gif89a,
	Bmp,
	WebP,
}

/// Color interpretation established by an encoded image header.
///
/// Absence of this value means the header establishes a component count but
/// does not establish an unambiguous color interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImageColorModel {
	Grayscale,
	GrayscaleAlpha,
	Rgb,
	Rgba,
	Bgr,
	IndexedRgb,
	YCbCr,
	Cmyk,
	Ycck,
}

/// Storage layout of the values retained by ingestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImageValueLayout {
	/// The original compressed or container-encoded file bytes are retained.
	EncodedFile,
}

/// Numeric range contract of the values retained by ingestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImageValueRange {
	/// Values remain opaque encoded bytes; no pixel numeric range is claimed.
	EncodedBytes,
}

/// Header facts for one encoded image variant.
///
/// This does not claim that the retained payload contains decoded pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EncodedImageMetadata {
	format: EncodedImageFormat,
	width: u32,
	height: u32,
	channels: Option<u8>,
	color_model: Option<ImageColorModel>,
	sample_bits: Option<u8>,
	value_layout: ImageValueLayout,
	value_range: ImageValueRange,
}

impl EncodedImageMetadata {
	#[must_use]
	pub const fn format(self) -> EncodedImageFormat { self.format }

	#[must_use]
	pub const fn width(self) -> u32 { self.width }

	#[must_use]
	pub const fn height(self) -> u32 { self.height }

	#[must_use]
	pub const fn channels(self) -> Option<u8> { self.channels }

	#[must_use]
	pub const fn color_model(self) -> Option<ImageColorModel> { self.color_model }

	#[must_use]
	pub const fn sample_bits(self) -> Option<u8> { self.sample_bits }

	#[must_use]
	pub const fn value_layout(self) -> ImageValueLayout { self.value_layout }

	#[must_use]
	pub const fn value_range(self) -> ImageValueRange { self.value_range }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImageHeaderError {
	detail: String,
}

impl ImageHeaderError {
	fn new(detail: impl Into<String>) -> Self {
		Self {
			detail: detail.into(),
		}
	}
}

impl fmt::Display for ImageHeaderError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.detail) }
}

pub(crate) fn inspect_encoded_image(bytes: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
		inspect_png(bytes)
	} else if bytes.starts_with(b"\xff\xd8\xff") {
		inspect_jpeg(bytes)
	} else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
		inspect_gif(bytes)
	} else if bytes.starts_with(b"BM") {
		inspect_bmp(bytes)
	} else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
		inspect_webp(bytes)
	} else {
		Err(ImageHeaderError::new(
			"unrecognized encoded image signature",
		))
	}
}

pub(crate) fn has_recognized_image_signature(bytes: &[u8]) -> bool {
	bytes.starts_with(b"\x89PNG\r\n\x1a\n")
		|| bytes.starts_with(b"\xff\xd8\xff")
		|| bytes.starts_with(b"GIF87a")
		|| bytes.starts_with(b"GIF89a")
		|| bytes.starts_with(b"BM")
		|| bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
}

const fn metadata(
	format: EncodedImageFormat,
	width: u32,
	height: u32,
	channels: Option<u8>,
	color_model: Option<ImageColorModel>,
	sample_bits: Option<u8>,
) -> EncodedImageMetadata {
	EncodedImageMetadata {
		format,
		width,
		height,
		channels,
		color_model,
		sample_bits,
		value_layout: ImageValueLayout::EncodedFile,
		value_range: ImageValueRange::EncodedBytes,
	}
}

fn inspect_png(bytes: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	let header = bytes
		.get(..33)
		.ok_or_else(|| ImageHeaderError::new("truncated PNG IHDR chunk"))?;
	if read_be_u32(header, 8)? != 13 || header.get(12..16) != Some(b"IHDR") {
		return Err(ImageHeaderError::new(
			"PNG signature is not followed by a 13-byte IHDR chunk",
		));
	}
	let width = read_be_u32(header, 16)?;
	let height = read_be_u32(header, 20)?;
	require_dimensions(width, height, "PNG")?;
	if width > i32::MAX as u32 || height > i32::MAX as u32 {
		return Err(ImageHeaderError::new(
			"PNG dimensions exceed the format's 31-bit limit",
		));
	}
	let sample_bits = header[24];
	let color_type = header[25];
	let (channels, color_model, valid_bits): (u8, ImageColorModel, &[u8]) = match color_type {
		0 => (1, ImageColorModel::Grayscale, &[1, 2, 4, 8, 16]),
		2 => (3, ImageColorModel::Rgb, &[8, 16]),
		3 => (1, ImageColorModel::IndexedRgb, &[1, 2, 4, 8]),
		4 => (2, ImageColorModel::GrayscaleAlpha, &[8, 16]),
		6 => (4, ImageColorModel::Rgba, &[8, 16]),
		_ => {
			return Err(ImageHeaderError::new(format!(
				"PNG IHDR has reserved color type {color_type}"
			)));
		}
	};
	if !valid_bits.contains(&sample_bits) {
		return Err(ImageHeaderError::new(format!(
			"PNG color type {color_type} does not permit {sample_bits}-bit samples"
		)));
	}
	if header[26] != 0 || header[27] != 0 || header[28] > 1 {
		return Err(ImageHeaderError::new(
			"PNG IHDR has an unsupported reserved method value",
		));
	}
	let expected_crc = read_be_u32(header, 29)?;
	if png_crc32(&header[12..29]) != expected_crc {
		return Err(ImageHeaderError::new(
			"PNG IHDR CRC does not match its header bytes",
		));
	}
	Ok(metadata(
		EncodedImageFormat::Png,
		width,
		height,
		Some(channels),
		Some(color_model),
		Some(sample_bits),
	))
}

fn inspect_gif(bytes: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	let header = bytes
		.get(..13)
		.ok_or_else(|| ImageHeaderError::new("truncated GIF logical screen descriptor"))?;
	let format = match &header[..6] {
		b"GIF87a" => EncodedImageFormat::Gif87a,
		b"GIF89a" => EncodedImageFormat::Gif89a,
		_ => return Err(ImageHeaderError::new("invalid GIF version signature")),
	};
	let width = u32::from(read_le_u16(header, 6)?);
	let height = u32::from(read_le_u16(header, 8)?);
	require_dimensions(width, height, "GIF")?;
	let packed = header[10];
	let sample_bits = if packed & 0x80 != 0 {
		let table_bits = (packed & 0x07) + 1;
		let colors = 1usize << usize::from(table_bits);
		let table_bytes = colors
			.checked_mul(3)
			.and_then(|size| 13usize.checked_add(size))
			.ok_or_else(|| ImageHeaderError::new("GIF global color table size overflowed usize"))?;
		bytes.get(..table_bytes)
			.ok_or_else(|| ImageHeaderError::new("truncated GIF global color table"))?;
		Some(table_bits)
	} else {
		None
	};
	Ok(metadata(
		format,
		width,
		height,
		Some(1),
		Some(ImageColorModel::IndexedRgb),
		sample_bits,
	))
}

fn inspect_jpeg(bytes: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	if bytes.len() < 4 {
		return Err(ImageHeaderError::new("truncated JPEG marker header"));
	}
	let mut cursor = 2usize;
	let mut jfif = false;
	let mut adobe_transform = None;
	while cursor < bytes.len() {
		if bytes[cursor] != 0xff {
			return Err(ImageHeaderError::new(
				"JPEG header contains data outside a marker segment",
			));
		}
		while bytes.get(cursor) == Some(&0xff) {
			cursor += 1;
		}
		let marker = *bytes
			.get(cursor)
			.ok_or_else(|| ImageHeaderError::new("truncated JPEG marker code"))?;
		cursor += 1;
		if marker == 0x00 {
			return Err(ImageHeaderError::new(
				"JPEG header contains a stuffed marker before scan data",
			));
		}
		if matches!(marker, 0xd8 | 0x01 | 0xd0..=0xd7) {
			continue;
		}
		if marker == 0xd9 {
			return Err(ImageHeaderError::new(
				"JPEG ended before a start-of-frame marker",
			));
		}
		if marker == 0xda {
			return Err(ImageHeaderError::new(
				"JPEG scan begins before a start-of-frame marker",
			));
		}
		let segment_length = usize::from(read_be_u16(bytes, cursor)?);
		if segment_length < 2 {
			return Err(ImageHeaderError::new(
				"JPEG marker segment length is smaller than two bytes",
			));
		}
		let payload_start = cursor
			.checked_add(2)
			.ok_or_else(|| ImageHeaderError::new("JPEG marker offset overflowed usize"))?;
		let payload_end = cursor
			.checked_add(segment_length)
			.ok_or_else(|| ImageHeaderError::new("JPEG marker length overflowed usize"))?;
		let payload = bytes
			.get(payload_start..payload_end)
			.ok_or_else(|| ImageHeaderError::new("truncated JPEG marker segment"))?;
		match marker {
			0xe0 => jfif |= payload.starts_with(b"JFIF\0"),
			0xee if payload.starts_with(b"Adobe") && payload.len() >= 12 => {
				adobe_transform = payload.get(11).copied();
			}
			_ if is_jpeg_start_of_frame(marker) => {
				return jpeg_frame_metadata(payload, jfif, adobe_transform);
			}
			_ => {}
		}
		cursor = payload_end;
	}
	Err(ImageHeaderError::new("JPEG has no start-of-frame marker"))
}

const fn is_jpeg_start_of_frame(marker: u8) -> bool {
	matches!(
		marker,
		0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
	)
}

fn jpeg_frame_metadata(
	payload: &[u8],
	jfif: bool,
	adobe_transform: Option<u8>,
) -> Result<EncodedImageMetadata, ImageHeaderError> {
	let frame = payload
		.get(..6)
		.ok_or_else(|| ImageHeaderError::new("truncated JPEG start-of-frame payload"))?;
	let sample_bits = frame[0];
	if sample_bits == 0 {
		return Err(ImageHeaderError::new("JPEG sample precision is zero"));
	}
	let height = u32::from(read_be_u16(frame, 1)?);
	let width = u32::from(read_be_u16(frame, 3)?);
	require_dimensions(width, height, "JPEG")?;
	let channels = frame[5];
	if channels == 0 {
		return Err(ImageHeaderError::new("JPEG frame declares zero components"));
	}
	let expected = usize::from(channels)
		.checked_mul(3)
		.and_then(|components| 6usize.checked_add(components))
		.ok_or_else(|| ImageHeaderError::new("JPEG component table size overflowed usize"))?;
	let components = payload
		.get(6..expected)
		.ok_or_else(|| ImageHeaderError::new("truncated JPEG frame component table"))?;
	let ids = components
		.chunks_exact(3)
		.map(|component| component[0])
		.collect::<Vec<_>>();
	let color_model = match channels {
		1 => Some(ImageColorModel::Grayscale),
		3 if adobe_transform == Some(0) || ids.as_slice() == b"RGB" => Some(ImageColorModel::Rgb),
		3 if adobe_transform == Some(1) || jfif || ids == [1, 2, 3] => Some(ImageColorModel::YCbCr),
		4 if adobe_transform == Some(2) => Some(ImageColorModel::Ycck),
		4 if adobe_transform == Some(0) || ids.as_slice() == b"CMYK" => Some(ImageColorModel::Cmyk),
		_ => None,
	};
	Ok(metadata(
		EncodedImageFormat::Jpeg,
		width,
		height,
		Some(channels),
		color_model,
		Some(sample_bits),
	))
}

fn inspect_bmp(bytes: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	bytes.get(..18)
		.ok_or_else(|| ImageHeaderError::new("truncated BMP file and DIB header"))?;
	let dib_size = usize::try_from(read_le_u32(bytes, 14)?)
		.map_err(|error| ImageHeaderError::new(format!("BMP DIB size cannot address host memory: {error}")))?;
	if dib_size < 12 {
		return Err(ImageHeaderError::new(format!(
			"BMP DIB header is only {dib_size} bytes"
		)));
	}
	let dib_end = 14usize
		.checked_add(dib_size)
		.ok_or_else(|| ImageHeaderError::new("BMP DIB header size overflowed usize"))?;
	bytes.get(..dib_end)
		.ok_or_else(|| ImageHeaderError::new("truncated BMP DIB header"))?;
	let (width, height, planes, bits_per_pixel) = if dib_size == 12 {
		(
			u32::from(read_le_u16(bytes, 18)?),
			u32::from(read_le_u16(bytes, 20)?),
			read_le_u16(bytes, 22)?,
			read_le_u16(bytes, 24)?,
		)
	} else if dib_size >= 16 {
		let width_raw = read_le_u32(bytes, 18)?;
		let height_raw = read_le_u32(bytes, 22)?;
		let windows_header = matches!(dib_size, 40 | 52 | 56 | 108 | 124);
		let width = if windows_header {
			u32::try_from(i32::from_le_bytes(width_raw.to_le_bytes()))
				.map_err(|_| ImageHeaderError::new("BMP width is not positive"))?
		} else {
			width_raw
		};
		let height = if windows_header {
			let signed = i32::from_le_bytes(height_raw.to_le_bytes());
			signed.checked_abs()
				.and_then(|value| u32::try_from(value).ok())
				.ok_or_else(|| {
					ImageHeaderError::new("BMP height cannot be represented as a positive dimension")
				})?
		} else {
			height_raw
		};
		(
			width,
			height,
			read_le_u16(bytes, 26)?,
			read_le_u16(bytes, 28)?,
		)
	} else {
		return Err(ImageHeaderError::new(format!(
			"BMP DIB header size {dib_size} does not contain dimensions and a pixel contract"
		)));
	};
	require_dimensions(width, height, "BMP")?;
	if planes != 1 {
		return Err(ImageHeaderError::new(format!(
			"BMP declares {planes} color planes instead of one"
		)));
	}
	if bits_per_pixel == 0 {
		return Err(ImageHeaderError::new("BMP declares zero bits per pixel"));
	}
	let (channels, color_model, sample_bits) = match bits_per_pixel {
		1 | 2 | 4 | 8 => {
			(
				Some(1),
				Some(ImageColorModel::IndexedRgb),
				u8::try_from(bits_per_pixel).ok(),
			)
		}
		24 => (Some(3), Some(ImageColorModel::Bgr), Some(8)),
		48 => (Some(3), Some(ImageColorModel::Bgr), Some(16)),
		_ => (None, None, None),
	};
	Ok(metadata(
		EncodedImageFormat::Bmp,
		width,
		height,
		channels,
		color_model,
		sample_bits,
	))
}

fn inspect_webp(bytes: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	let riff = bytes
		.get(..12)
		.ok_or_else(|| ImageHeaderError::new("truncated WebP RIFF header"))?;
	let riff_size = usize::try_from(read_le_u32(riff, 4)?).map_err(|error| {
		ImageHeaderError::new(format!(
			"WebP RIFF size cannot address host memory: {error}"
		))
	})?;
	let declared_end = riff_size
		.checked_add(8)
		.ok_or_else(|| ImageHeaderError::new("WebP RIFF size overflowed usize"))?;
	if riff_size < 4 || declared_end > bytes.len() {
		return Err(ImageHeaderError::new("truncated WebP RIFF payload"));
	}
	let mut cursor = 12usize;
	let mut separate_alpha = false;
	while cursor < declared_end {
		let chunk_header_end = cursor
			.checked_add(8)
			.ok_or_else(|| ImageHeaderError::new("WebP chunk header offset overflowed usize"))?;
		let chunk_header = bytes
			.get(cursor..chunk_header_end)
			.ok_or_else(|| ImageHeaderError::new("truncated WebP chunk header"))?;
		let chunk_size = usize::try_from(read_le_u32(chunk_header, 4)?).map_err(|error| {
			ImageHeaderError::new(format!(
				"WebP chunk size cannot address host memory: {error}"
			))
		})?;
		let payload_start = chunk_header_end;
		let payload_end = payload_start
			.checked_add(chunk_size)
			.ok_or_else(|| ImageHeaderError::new("WebP chunk size overflowed usize"))?;
		if payload_end > declared_end {
			return Err(ImageHeaderError::new(
				"WebP chunk exceeds its declared RIFF payload",
			));
		}
		let payload = bytes
			.get(payload_start..payload_end)
			.ok_or_else(|| ImageHeaderError::new("truncated WebP chunk payload"))?;
		match &chunk_header[..4] {
			b"VP8X" => return inspect_webp_extended(payload),
			b"VP8L" => return inspect_webp_lossless(payload),
			b"VP8 " => return inspect_webp_lossy(payload, separate_alpha),
			b"ALPH" => separate_alpha = true,
			_ => {}
		}
		let padded_size = chunk_size
			.checked_add(chunk_size & 1)
			.ok_or_else(|| ImageHeaderError::new("WebP padded chunk size overflowed usize"))?;
		cursor = payload_start
			.checked_add(padded_size)
			.ok_or_else(|| ImageHeaderError::new("WebP next chunk offset overflowed usize"))?;
		if cursor > declared_end {
			return Err(ImageHeaderError::new("truncated WebP chunk padding"));
		}
	}
	Err(ImageHeaderError::new(
		"WebP has no dimension-bearing VP8, VP8L, or VP8X chunk",
	))
}

fn inspect_webp_extended(payload: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	let header = payload
		.get(..10)
		.ok_or_else(|| ImageHeaderError::new("truncated WebP VP8X header"))?;
	if header[1..4] != [0, 0, 0] {
		return Err(ImageHeaderError::new(
			"WebP VP8X reserved bytes are nonzero",
		));
	}
	let width = read_le_u24(header, 4)?
		.checked_add(1)
		.ok_or_else(|| ImageHeaderError::new("WebP VP8X width overflowed u32"))?;
	let height = read_le_u24(header, 7)?
		.checked_add(1)
		.ok_or_else(|| ImageHeaderError::new("WebP VP8X height overflowed u32"))?;
	let alpha = header[0] & 0x10 != 0;
	Ok(metadata(
		EncodedImageFormat::WebP,
		width,
		height,
		Some(if alpha { 4 } else { 3 }),
		Some(if alpha {
			ImageColorModel::Rgba
		} else {
			ImageColorModel::Rgb
		}),
		Some(8),
	))
}

fn inspect_webp_lossless(payload: &[u8]) -> Result<EncodedImageMetadata, ImageHeaderError> {
	let header = payload
		.get(..5)
		.ok_or_else(|| ImageHeaderError::new("truncated WebP VP8L header"))?;
	if header[0] != 0x2f {
		return Err(ImageHeaderError::new("invalid WebP VP8L signature byte"));
	}
	if header[4] & 0xe0 != 0 {
		return Err(ImageHeaderError::new(
			"WebP VP8L header has a nonzero version",
		));
	}
	let width = 1 + u32::from(header[1]) + (u32::from(header[2] & 0x3f) << 8);
	let height = 1 + u32::from(header[2] >> 6) + (u32::from(header[3]) << 2) + (u32::from(header[4] & 0x0f) << 10);
	let alpha = header[4] & 0x10 != 0;
	Ok(metadata(
		EncodedImageFormat::WebP,
		width,
		height,
		Some(if alpha { 4 } else { 3 }),
		Some(if alpha {
			ImageColorModel::Rgba
		} else {
			ImageColorModel::Rgb
		}),
		Some(8),
	))
}

fn inspect_webp_lossy(payload: &[u8], alpha: bool) -> Result<EncodedImageMetadata, ImageHeaderError> {
	let header = payload
		.get(..10)
		.ok_or_else(|| ImageHeaderError::new("truncated WebP VP8 frame header"))?;
	if header[0] & 1 != 0 || header[3..6] != [0x9d, 0x01, 0x2a] {
		return Err(ImageHeaderError::new(
			"WebP VP8 chunk does not begin with a key-frame header",
		));
	}
	let width = u32::from(read_le_u16(header, 6)? & 0x3fff);
	let height = u32::from(read_le_u16(header, 8)? & 0x3fff);
	require_dimensions(width, height, "WebP VP8")?;
	Ok(metadata(
		EncodedImageFormat::WebP,
		width,
		height,
		Some(if alpha { 4 } else { 3 }),
		Some(if alpha {
			ImageColorModel::Rgba
		} else {
			ImageColorModel::YCbCr
		}),
		Some(8),
	))
}

fn require_dimensions(width: u32, height: u32, format: &str) -> Result<(), ImageHeaderError> {
	if width == 0 || height == 0 {
		return Err(ImageHeaderError::new(format!(
			"{format} declares zero-sized dimensions {width}x{height}"
		)));
	}
	Ok(())
}

fn png_crc32(bytes: &[u8]) -> u32 {
	let mut crc = u32::MAX;
	for byte in bytes {
		crc ^= u32::from(*byte);
		for _ in 0..8 {
			let mask = 0u32.wrapping_sub(crc & 1);
			crc = (crc >> 1) ^ (0xedb8_8320 & mask);
		}
	}
	!crc
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, ImageHeaderError> {
	let value = bytes
		.get(offset..offset.saturating_add(2))
		.ok_or_else(|| ImageHeaderError::new("truncated big-endian u16 image header field"))?;
	Ok(u16::from_be_bytes([value[0], value[1]]))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, ImageHeaderError> {
	let value = bytes
		.get(offset..offset.saturating_add(4))
		.ok_or_else(|| ImageHeaderError::new("truncated big-endian u32 image header field"))?;
	Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Result<u16, ImageHeaderError> {
	let value = bytes
		.get(offset..offset.saturating_add(2))
		.ok_or_else(|| ImageHeaderError::new("truncated little-endian u16 image header field"))?;
	Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_le_u24(bytes: &[u8], offset: usize) -> Result<u32, ImageHeaderError> {
	let value = bytes
		.get(offset..offset.saturating_add(3))
		.ok_or_else(|| ImageHeaderError::new("truncated little-endian u24 image header field"))?;
	Ok(u32::from(value[0]) | (u32::from(value[1]) << 8) | (u32::from(value[2]) << 16))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, ImageHeaderError> {
	let value = bytes
		.get(offset..offset.saturating_add(4))
		.ok_or_else(|| ImageHeaderError::new("truncated little-endian u32 image header field"))?;
	Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}
