//! Safetensors reader: parse the binary container — an 8-byte little-endian header
//! length, a JSON header mapping each tensor name to its dtype/shape/byte-range, then
//! the raw tensor blob — and upload every tensor into a named GPU buffer. Hand-rolled:
//! the header is a flat, predictable object, so a tiny recursive-descent JSON parser
//! beats pulling serde. Read side only — bytes in, named `GpuBuffer`s out; it knows
//! nothing of layers or models. Integer/half dtypes are widened to f64 to match the
//! framework's f64-only buffers.

use anyhow::{Result, anyhow, bail};
use gpu_core::memory::GpuBuffer;
use std::collections::BTreeMap;
use std::path::Path;

/// Load a `.safetensors` file into one GPU buffer per tensor, keyed by name (the
/// `BTreeMap` sorts and dedups). Every tensor's bytes are decoded to f64 on the host
/// and uploaded once. A missing/short header or an offset outside the blob is a hard
/// error — a clear cause beats a silently truncated tensor.
pub fn load_safetensors<P: AsRef<Path>>(path: P) -> Result<BTreeMap<String, GpuBuffer>> {
	let path = path.as_ref();
	let bytes =
		std::fs::read(path).map_err(|e| anyhow!("safetensors: read {}: {e}", path.display()))?;
	let mut out = BTreeMap::new();
	for (name, vals) in parse_safetensors(&bytes)? {
		out.insert(name, GpuBuffer::upload(&vals)?);
	}
	Ok(out)
}

/// Decode a safetensors byte image into `(name, shape, values)` triples in header order
/// — the GPU-free host core, unit-testable without a device. Skips the optional
/// `__metadata__` entry. Each tensor's `data_offsets` index the blob that follows the
/// JSON header; the byte span is checked against `shape * dtype-size`.
pub fn parse_safetensors_shaped(bytes: &[u8]) -> Result<Vec<(String, Vec<usize>, Vec<f64>)>> {
	if bytes.len() < 8 {
		bail!("safetensors: {} bytes is too short for the 8-byte header length", bytes.len());
	}
	let n = u64::from_le_bytes(bytes[..8].try_into().expect("8-byte header len")) as usize;
	let data_start = 8 + n;
	if bytes.len() < data_start {
		bail!("safetensors: header length {n} exceeds file size {}", bytes.len());
	}
	let header = std::str::from_utf8(&bytes[8..data_start])
		.map_err(|e| anyhow!("safetensors: header is not utf8: {e}"))?;
	let data = &bytes[data_start..];
	let Json::Obj(entries) = parse_json(header)? else {
		bail!("safetensors: header is not a JSON object");
	};
	let mut out = Vec::new();
	for (name, val) in entries {
		if name == "__metadata__" {
			continue;
		}
		let Json::Obj(fields) = val else {
			bail!("safetensors: tensor '{name}' is not an object");
		};
		let dtype = field_str(&fields, "dtype")
			.ok_or_else(|| anyhow!("safetensors: tensor '{name}' missing string dtype"))?;
		let shape = field_arr(&fields, "shape")
			.ok_or_else(|| anyhow!("safetensors: tensor '{name}' missing numeric shape"))?;
		let offsets = field_arr(&fields, "data_offsets")
			.ok_or_else(|| anyhow!("safetensors: tensor '{name}' missing data_offsets"))?;
		if offsets.len() != 2 {
			bail!("safetensors: tensor '{name}' data_offsets must have exactly 2 elements");
		}
		let begin = offsets[0] as usize;
		let end = offsets[1] as usize;
		if begin > end || end > data.len() {
			bail!(
				"safetensors: tensor '{name}' offsets [{begin},{end}] outside {}-byte blob",
				data.len()
			);
		}
		let raw = &data[begin..end];
		let elem = elem_size(&dtype)
			.ok_or_else(|| anyhow!("safetensors: tensor '{name}' unsupported dtype '{dtype}'"))?;
		let count: usize = shape.iter().map(|&d| d as usize).product();
		if raw.len() != count * elem {
			bail!(
				"safetensors: tensor '{name}' byte span {} != shape product {count} * {elem} bytes",
				raw.len()
			);
		}
		out.push((name, shape.iter().map(|&d| d as usize).collect(), decode(&dtype, raw)));
	}
	Ok(out)
}

/// A single tensor's header record: name, dtype, shape, and the byte range
/// `[begin, end)` *relative to the blob* (i.e. relative to `data_start`).
pub struct TensorEntry {
	pub name: String,
	pub dtype: String,
	pub shape: Vec<usize>,
	pub begin: usize,
	pub end: usize,
}

/// Parse only the JSON header of a safetensors image — no blob decode. Returns
/// `(data_start, entries)` where `data_start = 8 + header_len` is the file
/// offset of the blob, so a tensor's file bytes are `[data_start+begin,
/// data_start+end)`. Lets a caller memory-map / seek huge shards and stage
/// individual tensors on demand rather than widening the whole file to f64.
pub fn parse_safetensors_header(bytes: &[u8]) -> Result<(usize, Vec<TensorEntry>)> {
	if bytes.len() < 8 {
		bail!("safetensors: {} bytes is too short for the 8-byte header length", bytes.len());
	}
	let n = u64::from_le_bytes(bytes[..8].try_into().expect("8-byte header len")) as usize;
	let data_start = 8 + n;
	if bytes.len() < data_start {
		bail!("safetensors: header length {n} exceeds file size {}", bytes.len());
	}
	let header = std::str::from_utf8(&bytes[8..data_start])
		.map_err(|e| anyhow!("safetensors: header is not utf8: {e}"))?;
	let Json::Obj(entries) = parse_json(header)? else {
		bail!("safetensors: header is not a JSON object");
	};
	let mut out = Vec::new();
	for (name, val) in entries {
		if name == "__metadata__" {
			continue;
		}
		let Json::Obj(fields) = val else {
			bail!("safetensors: tensor '{name}' is not an object");
		};
		let dtype = field_str(&fields, "dtype")
			.ok_or_else(|| anyhow!("safetensors: tensor '{name}' missing string dtype"))?;
		let shape = field_arr(&fields, "shape")
			.ok_or_else(|| anyhow!("safetensors: tensor '{name}' missing numeric shape"))?;
		let offsets = field_arr(&fields, "data_offsets")
			.ok_or_else(|| anyhow!("safetensors: tensor '{name}' missing data_offsets"))?;
		if offsets.len() != 2 {
			bail!("safetensors: tensor '{name}' data_offsets must have exactly 2 elements");
		}
		out.push(TensorEntry {
			name,
			dtype,
			shape: shape.iter().map(|&d| d as usize).collect(),
			begin: offsets[0] as usize,
			end: offsets[1] as usize,
		});
	}
	Ok((data_start, out))
}

/// `(name, values)` pairs, dropping shape — the original flat view used by
/// `load_safetensors`. Thin wrapper over [`parse_safetensors_shaped`].
pub fn parse_safetensors(bytes: &[u8]) -> Result<Vec<(String, Vec<f64>)>> {
	Ok(parse_safetensors_shaped(bytes)?.into_iter().map(|(n, _, v)| (n, v)).collect())
}

/// Bytes per element for the supported dtypes (`None` = unsupported), used to validate
/// the declared byte span before decoding. Kept in lockstep with `decode`'s arms.
fn elem_size(dtype: &str) -> Option<usize> {
	Some(match dtype {
		"BOOL" | "U8" | "I8" => 1,
		"F16" | "BF16" | "I16" | "U16" => 2,
		"F32" | "I32" | "U32" => 4,
		"F64" | "I64" | "U64" => 8,
		_ => return None,
	})
}

/// Decode a tensor's raw little-endian bytes to f64. The dtype and byte length are
/// pre-validated by `parse_safetensors`/`elem_size`, so `chunks_exact` never drops a
/// remainder and the final arm is unreachable.
fn decode(dtype: &str, raw: &[u8]) -> Vec<f64> {
	match dtype {
		"BOOL" | "U8" => raw.iter().map(|&x| x as f64).collect(),
		"I8" => raw.iter().map(|&x| x as i8 as f64).collect(),
		"I16" => raw.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f64).collect(),
		"U16" => raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]) as f64).collect(),
		"F16" => raw.chunks_exact(2).map(|c| f16_to_f64(u16::from_le_bytes([c[0], c[1]]))).collect(),
		"BF16" => raw
			.chunks_exact(2)
			.map(|c| f32::from_bits((u16::from_le_bytes([c[0], c[1]]) as u32) << 16) as f64)
			.collect(),
		"I32" => raw.chunks_exact(4).map(|c| i32::from_le_bytes(arr4(c)) as f64).collect(),
		"U32" => raw.chunks_exact(4).map(|c| u32::from_le_bytes(arr4(c)) as f64).collect(),
		"F32" => raw.chunks_exact(4).map(|c| f32::from_le_bytes(arr4(c)) as f64).collect(),
		"I64" => raw.chunks_exact(8).map(|c| i64::from_le_bytes(arr8(c)) as f64).collect(),
		"U64" => raw.chunks_exact(8).map(|c| u64::from_le_bytes(arr8(c)) as f64).collect(),
		"F64" => raw.chunks_exact(8).map(|c| f64::from_le_bytes(arr8(c))).collect(),
		_ => unreachable!("decode: dtype '{dtype}' passed elem_size but has no decode arm"),
	}
}

fn arr4(c: &[u8]) -> [u8; 4] {
	c.try_into().expect("chunks_exact(4) yields 4 bytes")
}

fn arr8(c: &[u8]) -> [u8; 8] {
	c.try_into().expect("chunks_exact(8) yields 8 bytes")
}

/// IEEE-754 half (binary16) → f64: split sign/exponent/mantissa, handling subnormals,
/// infinities, and NaN. No `half` dependency — the bit math is a handful of shifts.
fn f16_to_f64(h: u16) -> f64 {
	let sign = if h >> 15 == 1 { -1.0 } else { 1.0 };
	let exp = (h >> 10) & 0x1f;
	let mant = (h & 0x3ff) as f64;
	let val = match exp {
		0 => mant * 2f64.powi(-24),
		0x1f if mant == 0.0 => f64::INFINITY,
		0x1f => f64::NAN,
		_ => (1.0 + mant / 1024.0) * 2f64.powi(exp as i32 - 15),
	};
	sign * val
}

// --- Minimal JSON for the flat safetensors header. Per spec every value is an object,
// array, string (dtype, __metadata__ values), or number (shape, data_offsets) — there
// are no bools/nulls to handle. Just enough to read the documented fields. ---

enum Json {
	Obj(Vec<(String, Json)>),
	Arr(Vec<Json>),
	Str(String),
	Num(f64),
}

fn field<'a>(fields: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
	fields.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn field_str(fields: &[(String, Json)], name: &str) -> Option<String> {
	match field(fields, name) {
		Some(Json::Str(s)) => Some(s.clone()),
		_ => None,
	}
}

fn field_arr(fields: &[(String, Json)], name: &str) -> Option<Vec<f64>> {
	match field(fields, name) {
		Some(Json::Arr(a)) => a
			.iter()
			.map(|v| match v {
				Json::Num(n) => Some(*n),
				_ => None,
			})
			.collect(),
		_ => None,
	}
}

fn parse_json(s: &str) -> Result<Json> {
	let b = s.as_bytes();
	let mut p = 0usize;
	let v = parse_value(b, &mut p)?;
	skip_ws(b, &mut p);
	if p != b.len() {
		bail!("safetensors: trailing bytes after JSON header at offset {p}");
	}
	Ok(v)
}

fn skip_ws(b: &[u8], p: &mut usize) {
	while *p < b.len() && matches!(b[*p], b' ' | b'\t' | b'\n' | b'\r') {
		*p += 1;
	}
}

fn parse_value(b: &[u8], p: &mut usize) -> Result<Json> {
	skip_ws(b, p);
	match b.get(*p).copied() {
		Some(b'{') => parse_obj(b, p),
		Some(b'[') => parse_arr(b, p),
		Some(b'"') => Ok(Json::Str(parse_str(b, p)?)),
		Some(_) => parse_num(b, p),
		None => bail!("safetensors: unexpected end of JSON header"),
	}
}

fn parse_obj(b: &[u8], p: &mut usize) -> Result<Json> {
	*p += 1; // consume '{'
	let mut out = Vec::new();
	skip_ws(b, p);
	if b.get(*p) == Some(&b'}') {
		*p += 1;
		return Ok(Json::Obj(out));
	}
	loop {
		skip_ws(b, p);
		let key = parse_str(b, p)?;
		skip_ws(b, p);
		if b.get(*p) != Some(&b':') {
			bail!("safetensors: expected ':' at offset {}", *p);
		}
		*p += 1;
		let val = parse_value(b, p)?;
		out.push((key, val));
		skip_ws(b, p);
		match b.get(*p).copied() {
			Some(b',') => *p += 1,
			Some(b'}') => {
				*p += 1;
				return Ok(Json::Obj(out));
			}
			_ => bail!("safetensors: expected ',' or '}}' at offset {}", *p),
		}
	}
}

fn parse_arr(b: &[u8], p: &mut usize) -> Result<Json> {
	*p += 1; // consume '['
	let mut out = Vec::new();
	skip_ws(b, p);
	if b.get(*p) == Some(&b']') {
		*p += 1;
		return Ok(Json::Arr(out));
	}
	loop {
		out.push(parse_value(b, p)?);
		skip_ws(b, p);
		match b.get(*p).copied() {
			Some(b',') => *p += 1,
			Some(b']') => {
				*p += 1;
				return Ok(Json::Arr(out));
			}
			_ => bail!("safetensors: expected ',' or ']' at offset {}", *p),
		}
	}
}

fn parse_str(b: &[u8], p: &mut usize) -> Result<String> {
	if b.get(*p) != Some(&b'"') {
		bail!("safetensors: expected '\"' at offset {}", *p);
	}
	*p += 1;
	let mut buf: Vec<u8> = Vec::new();
	loop {
		match b.get(*p).copied() {
			None => bail!("safetensors: unterminated string"),
			Some(b'"') => {
				*p += 1;
				return String::from_utf8(buf)
					.map_err(|e| anyhow!("safetensors: string is not utf8: {e}"));
			}
			Some(b'\\') => {
				*p += 1;
				let mut tmp = [0u8; 4];
				let ch = match b.get(*p).copied() {
					Some(b'"') => '"',
					Some(b'\\') => '\\',
					Some(b'/') => '/',
					Some(b'n') => '\n',
					Some(b't') => '\t',
					Some(b'r') => '\r',
					Some(b'b') => '\u{8}',
					Some(b'f') => '\u{c}',
					Some(b'u') => {
						let hex = b
							.get(*p + 1..*p + 5)
							.ok_or_else(|| anyhow!("safetensors: truncated \\u escape"))?;
						let code = u32::from_str_radix(std::str::from_utf8(hex)?, 16)
							.map_err(|e| anyhow!("safetensors: bad \\u escape: {e}"))?;
						*p += 4;
						char::from_u32(code)
							.ok_or_else(|| anyhow!("safetensors: invalid unicode {code}"))?
					}
					_ => bail!("safetensors: bad escape at offset {}", *p),
				};
				buf.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
				*p += 1;
			}
			Some(c) => {
				buf.push(c);
				*p += 1;
			}
		}
	}
}

fn parse_num(b: &[u8], p: &mut usize) -> Result<Json> {
	let start = *p;
	while *p < b.len() && matches!(b[*p], b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E') {
		*p += 1;
	}
	let s = std::str::from_utf8(&b[start..*p])
		.map_err(|e| anyhow!("safetensors: number is not utf8: {e}"))?;
	let v: f64 = s.parse().map_err(|_| anyhow!("safetensors: bad number '{s}'"))?;
	Ok(Json::Num(v))
}

#[cfg(test)]
mod tests {
	use super::*;

	// Host-only (no GPU): build a tiny safetensors image in memory and confirm the
	// parser honors the 8-byte length prefix, skips __metadata__, preserves header
	// order, and decodes F32/F64/I64 byte spans to f64.
	#[test]
	fn parse_safetensors_decodes_header_and_blob() {
		let header = concat!(
			r#"{"__metadata__":{"format":"pt"},"#,
			r#""a":{"dtype":"F32","shape":[2],"data_offsets":[0,8]},"#,
			r#""b":{"dtype":"F64","shape":[2],"data_offsets":[8,24]},"#,
			r#""c":{"dtype":"I64","shape":[2],"data_offsets":[24,40]}}"#,
		);
		let mut bytes = Vec::new();
		bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
		bytes.extend_from_slice(header.as_bytes());
		bytes.extend_from_slice(&1.5f32.to_le_bytes());
		bytes.extend_from_slice(&(-2.0f32).to_le_bytes());
		bytes.extend_from_slice(&3.0f64.to_le_bytes());
		bytes.extend_from_slice(&4.0f64.to_le_bytes());
		bytes.extend_from_slice(&(-7i64).to_le_bytes());
		bytes.extend_from_slice(&9i64.to_le_bytes());

		let parsed = parse_safetensors(&bytes).expect("parse safetensors image");
		assert_eq!(parsed.len(), 3);
		assert_eq!(parsed[0].0, "a");
		assert_eq!(parsed[0].1, vec![1.5, -2.0]);
		assert_eq!(parsed[1].0, "b");
		assert_eq!(parsed[1].1, vec![3.0, 4.0]);
		assert_eq!(parsed[2].0, "c");
		assert_eq!(parsed[2].1, vec![-7.0, 9.0]);
	}

	// A data_offsets range past the end of the blob is a hard error, not a truncation.
	#[test]
	fn parse_safetensors_rejects_out_of_range_offsets() {
		let header = r#"{"a":{"dtype":"F32","shape":[2],"data_offsets":[0,8]}}"#;
		let mut bytes = Vec::new();
		bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
		bytes.extend_from_slice(header.as_bytes());
		bytes.extend_from_slice(&1.5f32.to_le_bytes()); // only 4 of the claimed 8 bytes
		assert!(parse_safetensors(&bytes).is_err());
	}

	// Real-file smoke: parse the header of every shard in $SAFETENSORS_FILES (header
	// only — never reads the multi-GB blob, so a 4.6 GB shard costs ~100 KB of RAM),
	// then decode just the single smallest tensor of shard 1 to exercise the real
	// dtype path. Ignored by default; run with the env var set.
	#[test]
	#[ignore = "set SAFETENSORS_FILES to space-separated real .safetensors paths"]
	fn read_real_safetensors_headers() {
		use std::collections::BTreeMap;
		use std::io::{Read, Seek, SeekFrom};
		let files = std::env::var("SAFETENSORS_FILES").expect("set SAFETENSORS_FILES");
		let paths: Vec<&str> = files.split_whitespace().collect();
		let mut grand_tensors = 0usize;
		let mut grand_params: u128 = 0;
		let mut smallest: Option<(String, String, u64, u64, u64)> = None;
		for (fi, path) in paths.iter().enumerate() {
			let mut f = std::fs::File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
			let mut lb = [0u8; 8];
			f.read_exact(&mut lb).expect("read header len");
			let n = u64::from_le_bytes(lb);
			let mut hdr = vec![0u8; n as usize];
			f.read_exact(&mut hdr).expect("read header");
			let header = std::str::from_utf8(&hdr).expect("header utf8");
			let Json::Obj(entries) = parse_json(header).expect("parse header json") else {
				panic!("header not a JSON object");
			};
			let (mut count, mut params, mut sample) = (0usize, 0u128, String::new());
			let mut dtypes: BTreeMap<String, usize> = BTreeMap::new();
			let mut min_bytes = u64::MAX;
			for (name, val) in &entries {
				if name == "__metadata__" {
					continue;
				}
				let Json::Obj(fields) = val else { continue };
				let dtype = field_str(fields, "dtype").unwrap_or_default();
				let shape = field_arr(fields, "shape").unwrap_or_default();
				let offs = field_arr(fields, "data_offsets").unwrap_or_default();
				count += 1;
				params += shape.iter().map(|&d| d as u128).product::<u128>();
				*dtypes.entry(dtype.clone()).or_default() += 1;
				if sample.is_empty() {
					let sh: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
					sample = format!("{name} {sh:?} {dtype}");
				}
				if fi == 0 && offs.len() == 2 {
					let (b, e) = (offs[0] as u64, offs[1] as u64);
					if e > b && e - b < min_bytes {
						min_bytes = e - b;
						smallest = Some((name.clone(), dtype, 8 + n, b, e));
					}
				}
			}
			let fname = std::path::Path::new(path)
				.file_name()
				.and_then(|s| s.to_str())
				.unwrap_or(path);
			eprintln!("{fname}: {count} tensors, {params} params, dtypes {dtypes:?}\n    e.g. {sample}");
			assert!(count > 0, "{fname}: parsed 0 tensors");
			grand_tensors += count;
			grand_params += params;
		}
		eprintln!("TOTAL: {grand_tensors} tensors, {grand_params} params across {} shards", paths.len());

		if let Some((name, dtype, data_start, begin, end)) = smallest {
			let mut f = std::fs::File::open(paths[0]).expect("reopen shard 1");
			f.seek(SeekFrom::Start(data_start + begin)).expect("seek tensor");
			let mut raw = vec![0u8; (end - begin) as usize];
			f.read_exact(&mut raw).expect("read tensor bytes");
			let vals = decode(&dtype, &raw);
			let finite = vals.iter().filter(|v| v.is_finite()).count();
			let head: Vec<f64> = vals.iter().take(5).copied().collect();
			eprintln!(
				"decoded smallest tensor '{name}' ({dtype}, {} elems): {finite} finite, first {head:?}",
				vals.len()
			);
			assert_eq!(finite, vals.len(), "decoded NaN/Inf from real weights");
		}
	}
}
