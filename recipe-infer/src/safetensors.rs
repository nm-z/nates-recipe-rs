use anyhow::{Context, Result, anyhow, bail};

pub fn parse_safetensors_shaped(bytes: &[u8]) -> Result<Vec<(String, Vec<usize>, Vec<f64>)>> {
	if bytes.len() < 8 {
		bail!("safetensors: {} bytes is too short for the 8-byte header length", bytes.len());
	}
	let n = u64::from_le_bytes(bytes[..8].try_into().context("8-byte header len")?) as usize;
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
		out.push((name, shape.iter().map(|&d| d as usize).collect(), decode(&dtype, raw)?));
	}
	Ok(out)
}

pub struct TensorEntry {
	pub name: String,
	pub dtype: String,
	pub shape: Vec<usize>,
	pub begin: usize,
	pub end: usize,
}

pub fn parse_safetensors_header(bytes: &[u8]) -> Result<(usize, Vec<TensorEntry>)> {
	if bytes.len() < 8 {
		bail!("safetensors: {} bytes is too short for the 8-byte header length", bytes.len());
	}
	let n = u64::from_le_bytes(bytes[..8].try_into().context("8-byte header len")?) as usize;
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

pub fn parse_safetensors(bytes: &[u8]) -> Result<Vec<(String, Vec<f64>)>> {
	Ok(parse_safetensors_shaped(bytes)?.into_iter().map(|(n, _, v)| (n, v)).collect())
}

fn elem_size(dtype: &str) -> Option<usize> {
	Some(match dtype {
		"BOOL" | "U8" | "I8" => 1,
		"F16" | "BF16" | "I16" | "U16" => 2,
		"F32" | "I32" | "U32" => 4,
		"F64" | "I64" | "U64" => 8,
		_ => return None,
	})
}

pub fn decode(dtype: &str, raw: &[u8]) -> Result<Vec<f64>> {
	Ok(match dtype {
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
		_ => bail!("decode: unsupported dtype '{dtype}'"),
	})
}

fn arr4(c: &[u8]) -> [u8; 4] {
	let mut a = [0u8; 4];
	a.copy_from_slice(c);
	a
}

fn arr8(c: &[u8]) -> [u8; 8] {
	let mut a = [0u8; 8];
	a.copy_from_slice(c);
	a
}

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

pub enum Json {
	Obj(Vec<(String, Json)>),
	Arr(Vec<Json>),
	Str(String),
	Num(f64),
}

fn field<'a>(fields: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
	fields.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

pub fn field_str(fields: &[(String, Json)], name: &str) -> Option<String> {
	match field(fields, name) {
		Some(Json::Str(s)) => Some(s.clone()),
		_ => None,
	}
}

pub fn field_arr(fields: &[(String, Json)], name: &str) -> Option<Vec<f64>> {
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

pub fn parse_json(s: &str) -> Result<Json> {
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
	*p += 1;
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
	*p += 1;
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
