use anyhow::{Context, Result, anyhow, bail};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::os::unix::fs::FileExt;
use std::path::Path;

pub enum Val {
	U8(u8),
	I8(i8),
	U16(u16),
	I16(i16),
	U32(u32),
	I32(i32),
	F32(f32),
	Bool(bool),
	Str(String),
	U64(u64),
	I64(i64),
	F64(f64),
	Arr(Vec<Val>),
}

pub struct TensorInfo {
	pub dims: Vec<usize>,
	pub ggml_type: u32,
	pub offset: u64,
	pub nbytes: usize,
}

pub struct Gguf {
	file: File,
	pub kv: HashMap<String, Val>,
	pub tensors: HashMap<String, TensorInfo>,
	pub data_start: u64,
}

struct Reader {
	r: BufReader<File>,
	pos: u64,
}

impl Reader {
	fn take(&mut self, n: usize) -> Result<Vec<u8>> {
		let mut buf = vec![0u8; n];
		self.r.read_exact(&mut buf)?;
		self.pos += n as u64;
		Ok(buf)
	}
	fn u8v(&mut self) -> Result<u8> {
		Ok(self.take(1)?[0])
	}
	fn u16v(&mut self) -> Result<u16> {
		let b = self.take(2)?;
		Ok(u16::from_le_bytes([b[0], b[1]]))
	}
	fn u32v(&mut self) -> Result<u32> {
		let b = self.take(4)?;
		Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
	}
	fn u64v(&mut self) -> Result<u64> {
		let b = self.take(8)?;
		Ok(u64::from_le_bytes([
			b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
		]))
	}
	fn string(&mut self) -> Result<String> {
		let n = self.u64v()? as usize;
		let b = self.take(n)?;
		Ok(String::from_utf8_lossy(&b).into_owned())
	}
	fn value(&mut self, t: u32) -> Result<Val> {
		Ok(match t {
			0 => Val::U8(self.u8v()?),
			1 => Val::I8(self.u8v()? as i8),
			2 => Val::U16(self.u16v()?),
			3 => Val::I16(self.u16v()? as i16),
			4 => Val::U32(self.u32v()?),
			5 => Val::I32(self.u32v()? as i32),
			6 => Val::F32(f32::from_bits(self.u32v()?)),
			7 => Val::Bool(self.u8v()? != 0),
			8 => Val::Str(self.string()?),
			9 => {
				let et = self.u32v()?;
				let n = self.u64v()?;
				let mut items = Vec::with_capacity(n as usize);
				for _ in 0..n {
					items.push(self.value(et)?);
				}
				Val::Arr(items)
			}
			10 => Val::U64(self.u64v()?),
			11 => Val::I64(self.u64v()? as i64),
			12 => Val::F64(f64::from_bits(self.u64v()?)),
			_other => bail!("gguf: unknown metadata value type {t}"),
		})
	}
}

impl Gguf {
	pub fn open(path: &Path) -> Result<Gguf> {
		let file =
			File::open(path).with_context(|| format!("gguf: open {}", path.display()))?;
		let clone = file.try_clone().context("gguf: clone file handle")?;
		let mut rd = Reader {
			r: BufReader::new(clone),
			pos: 0,
		};
		let magic = rd.take(4)?;
		if magic.as_slice() != b"GGUF" {
			bail!("gguf: bad magic {magic:?}");
		}
		let version = rd.u32v()?;
		if version < 2 {
			bail!(
				"gguf: version {version} uses u32 counts and lengths; only v2+ (u64) is supported"
			);
		}
		let n_tensors = rd.u64v()?;
		let n_kv = rd.u64v()?;
		let mut kv = HashMap::new();
		for _ in 0..n_kv {
			let key = rd.string()?;
			let t = rd.u32v()?;
			let val = rd.value(t)?;
			kv.insert(key, val);
		}
		let mut raw = Vec::with_capacity(n_tensors as usize);
		for _ in 0..n_tensors {
			let name = rd.string()?;
			let nd = rd.u32v()? as usize;
			let mut dims = Vec::with_capacity(nd);
			for _ in 0..nd {
				dims.push(rd.u64v()? as usize);
			}
			let gt = rd.u32v()?;
			let off = rd.u64v()?;
			raw.push((name, dims, gt, off));
		}
		let align = match kv.get("general.alignment") {
			Some(Val::U32(a)) => *a as u64,
			_other => 32,
		};
		let mut data_start = rd.pos;
		let rem = data_start % align;
		if rem != 0 {
			data_start += align - rem;
		}
		let mut tensors = HashMap::new();
		for (name, dims, gt, off) in raw {
			let elems: usize = dims.iter().product();
			let nbytes = crate::dequant::nbytes_for(gt, elems);
			tensors.insert(
				name,
				TensorInfo {
					dims,
					ggml_type: gt,
					offset: data_start + off,
					nbytes,
				},
			);
		}
		Ok(Gguf {
			file,
			kv,
			tensors,
			data_start,
		})
	}

	pub fn read_raw(&self, name: &str) -> Result<Vec<u8>> {
		let info = self
			.tensors
			.get(name)
			.ok_or_else(|| anyhow!("gguf: no tensor {name}"))?;
		let mut buf = vec![0u8; info.nbytes];
		self.file.read_exact_at(&mut buf, info.offset)?;
		Ok(buf)
	}

	pub fn read_raw_range(&self, name: &str, byte_off: usize, len: usize) -> Result<Vec<u8>> {
		let info = self
			.tensors
			.get(name)
			.ok_or_else(|| anyhow!("gguf: no tensor {name}"))?;
		let mut buf = vec![0u8; len];
		self.file
			.read_exact_at(&mut buf, info.offset + byte_off as u64)?;
		Ok(buf)
	}

	pub fn str_arr(&self, key: &str) -> Result<Vec<String>> {
		match self.kv.get(key) {
			Some(Val::Arr(items)) => items
				.iter()
				.map(|v| match v {
					Val::Str(s) => Ok(s.clone()),
					_other => bail!("gguf: kv {key} array holds a non-string element"),
				})
				.collect(),
			Some(_other) => bail!("gguf: kv {key} is not an array"),
			None => bail!("gguf: kv {key} not found"),
		}
	}

	pub fn f32_arr(&self, key: &str) -> Result<Vec<f32>> {
		match self.kv.get(key) {
			Some(Val::Arr(items)) => items
				.iter()
				.map(|v| match v {
					Val::F32(x) => Ok(*x),
					_other => bail!("gguf: kv {key} array holds a non-f32 element"),
				})
				.collect(),
			Some(_other) => bail!("gguf: kv {key} is not an array"),
			None => bail!("gguf: kv {key} not found"),
		}
	}

	pub fn u32_kv(&self, key: &str) -> Result<u32> {
		match self.kv.get(key) {
			Some(Val::U32(v)) => Ok(*v),
			Some(_other) => bail!("gguf: kv {key} is not u32"),
			None => bail!("gguf: kv {key} not found"),
		}
	}

	pub fn f32_kv(&self, key: &str) -> Result<f32> {
		match self.kv.get(key) {
			Some(Val::F32(v)) => Ok(*v),
			Some(_other) => bail!("gguf: kv {key} is not f32"),
			None => bail!("gguf: kv {key} not found"),
		}
	}
}
