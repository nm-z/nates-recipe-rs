use anyhow::{Result, bail, ensure};

/// The dtypes this codec understands: the GGUF quant grid plus the three
/// full-width float encodings. Quant variants decode only; float variants
/// both decode and encode. `convert` is the single conversion surface.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DType {
      F32,
      F16,
      Bf16,
      Q4_0,
      Q4_1,
      Q5_0,
      Q5_1,
      Q8_0,
      Q2_K,
      Q3_K,
      Q4_K,
      Q5_K,
      Q6_K,
}

impl DType {
      /// Map a GGML type tag to a `DType`; unknown tags bail rather than guess.
      pub fn from_ggml(t: u32) -> Result<DType> {
            let d = match t {
                  0 => DType::F32,
                  1 => DType::F16,
                  2 => DType::Q4_0,
                  3 => DType::Q4_1,
                  6 => DType::Q5_0,
                  7 => DType::Q5_1,
                  8 => DType::Q8_0,
                  10 => DType::Q2_K,
                  11 => DType::Q3_K,
                  12 => DType::Q4_K,
                  13 => DType::Q5_K,
                  14 => DType::Q6_K,
                  30 => DType::Bf16,
                  other => bail!("gguf: unsupported ggml type {other}"),
            };
            return Ok(d);
      }

      fn codec(self) -> &'static dyn Codec {
            match self {
                  DType::F32 => &F32C,
                  DType::F16 => &F16C,
                  DType::Bf16 => &Bf16C,
                  DType::Q4_0 => &Q40C,
                  DType::Q4_1 => &Q41C,
                  DType::Q5_0 => &Q50C,
                  DType::Q5_1 => &Q51C,
                  DType::Q8_0 => &Q80C,
                  DType::Q2_K => &Q2KC,
                  DType::Q3_K => &Q3KC,
                  DType::Q4_K => &Q4KC,
                  DType::Q5_K => &Q5KC,
                  DType::Q6_K => &Q6KC,
            }
      }
}

/// One block layout + codec per dtype. `decode_block` consumes exactly
/// `block_bytes` and appends `block_elems` f32 values; `encode_block` is the
/// inverse. Quant codecs implement decode only.
trait Codec: Sync {
      fn name(&self) -> &'static str;
      fn block_bytes(&self) -> usize;
      fn block_elems(&self) -> usize;
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()>;
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()>;
}

/// THE conversion surface: decode every `src` block to f32, re-encode as `dst`.
pub fn convert(src: DType, dst: DType, bytes: &[u8]) -> Result<Vec<u8>> {
      let sc = src.codec();
      let dc = dst.codec();
      let sbb = sc.block_bytes();
      ensure!(
            bytes.len().is_multiple_of(sbb),
            "convert: {} bytes not a multiple of {} block_bytes ({})",
            bytes.len(),
            sc.name(),
            sbb
      );
      let nb = bytes.len() / sbb;
      let mut vals = Vec::with_capacity(nb * sc.block_elems());
      for b in 0..nb {
            sc.decode_block(&bytes[b * sbb..(b + 1) * sbb], &mut vals)?;
      }
      let dbe = dc.block_elems();
      ensure!(
            vals.len().is_multiple_of(dbe),
            "convert: {} elems not a multiple of {} block_elems ({})",
            vals.len(),
            dc.name(),
            dbe
      );
      let mut out = Vec::with_capacity((vals.len() / dbe) * dc.block_bytes());
      for chunk in vals.chunks_exact(dbe) {
            dc.encode_block(chunk, &mut out)?;
      }
      return Ok(out);
}

/// Block layout `(block_bytes, block_elems)` for a GGML type; aborts on an
/// unsupported tag, matching the loader's fail-loud contract.
pub fn block_layout(t: u32) -> (usize, usize) {
      match DType::from_ggml(t) {
            Ok(d) => {
                  let c = d.codec();
                  (c.block_bytes(), c.block_elems())
            }
            Err(e) => {
                  drop(gpu_core::log::Write::err(format!("{e}")));
                  std::process::abort()
            }
      }
}

pub fn nbytes_for(t: u32, elems: usize) -> usize {
      let (block_bytes, block_elems) = block_layout(t);
      return (elems / block_elems) * block_bytes;
}

/// Decode `bytes` of GGML type `t` to f32 values via the one conversion surface.
pub fn dequant_f32(t: u32, bytes: &[u8]) -> Result<Vec<f32>> {
      let raw = convert(DType::from_ggml(t)?, DType::F32, bytes)?;
      let out = raw
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
      return Ok(out);
}

/// Decode `bytes` of GGML type `t` to little-endian bf16 bytes.
pub fn dequant_bf16(t: u32, bytes: &[u8]) -> Result<Vec<u8>> {
      return convert(DType::from_ggml(t)?, DType::Bf16, bytes);
}

fn f16_to_f32(b: u16) -> f32 {
      let s = (b >> 15) & 1;
      let e = ((b >> 10) & 0x1f) as i32;
      let m = (b & 0x3ff) as u32;
      let bits = if e == 0 {
            if m == 0 {
                  (s as u32) << 31
            } else {
                  let mut e2 = -14i32;
                  let mut m2 = m;
                  while m2 & 0x400 == 0 {
                        m2 <<= 1;
                        e2 -= 1;
                  }
                  m2 &= 0x3ff;
                  ((s as u32) << 31) | (((e2 + 127) as u32) << 23) | (m2 << 13)
            }
      } else if e == 31 {
            ((s as u32) << 31) | (0xff << 23) | (m << 13)
      } else {
            ((s as u32) << 31) | (((e - 15 + 127) as u32) << 23) | (m << 13)
      };
      return f32::from_bits(bits);
}

fn f32_to_f16(x: f32) -> u16 {
      let bits = x.to_bits();
      let sign = ((bits >> 16) & 0x8000) as u16;
      let exp = ((bits >> 23) & 0xff) as i32;
      let mant = bits & 0x7fffff;
      if exp == 0xff {
            let m = if mant != 0 { 0x200 } else { 0 };
            return sign | 0x7c00 | m;
      }
      let e = exp - 127 + 15;
      if e >= 0x1f {
            return sign | 0x7c00;
      }
      if e <= 0 {
            if e < -10 {
                  return sign;
            }
            let m = (mant | 0x800000) >> (14 - e);
            return sign | (m as u16);
      }
      sign | ((e as u16) << 10) | ((mant >> 13) as u16)
}

fn f32_to_bf16(x: f32) -> u16 {
      let bits = x.to_bits();
      if x.is_nan() {
            return ((bits >> 16) as u16) | 0x0040;
      }
      let rounded = bits.wrapping_add(0x7fff + ((bits >> 16) & 1));
      return (rounded >> 16) as u16;
}

fn get_scale_min(j: usize, q: &[u8]) -> (u8, u8) {
      if j < 4 {
            return (q[j] & 63, q[j + 4] & 63);
      }
      return (
            (q[j + 4] & 0xF) | ((q[j - 4] >> 6) << 4),
            (q[j + 4] >> 4) | ((q[j] >> 6) << 4),
      );
}

struct F32Codec;
static F32C: F32Codec = F32Codec;
impl Codec for F32Codec {
      fn name(&self) -> &'static str {
            return "F32";
      }
      fn block_bytes(&self) -> usize {
            return 4;
      }
      fn block_elems(&self) -> usize {
            return 1;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            out.push(f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]));
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            out.extend_from_slice(&vals[0].to_le_bytes());
            return Ok(());
      }
}

struct F16Codec;
static F16C: F16Codec = F16Codec;
impl Codec for F16Codec {
      fn name(&self) -> &'static str {
            return "F16";
      }
      fn block_bytes(&self) -> usize {
            return 2;
      }
      fn block_elems(&self) -> usize {
            return 1;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            out.push(f16_to_f32(u16::from_le_bytes([raw[0], raw[1]])));
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            out.extend_from_slice(&f32_to_f16(vals[0]).to_le_bytes());
            return Ok(());
      }
}

struct Bf16Codec;
static Bf16C: Bf16Codec = Bf16Codec;
impl Codec for Bf16Codec {
      fn name(&self) -> &'static str {
            return "Bf16";
      }
      fn block_bytes(&self) -> usize {
            return 2;
      }
      fn block_elems(&self) -> usize {
            return 1;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let hi = u16::from_le_bytes([raw[0], raw[1]]) as u32;
            out.push(f32::from_bits(hi << 16));
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            out.extend_from_slice(&f32_to_bf16(vals[0]).to_le_bytes());
            return Ok(());
      }
}

/// Q4_0: `dequantize_row_q4_0` (ggml/src/ggml-quants.c). 18B/32: `d`(f16) then
/// 16 packed nibbles; each nibble minus 8, scaled by `d`.
struct Q40Codec;
static Q40C: Q40Codec = Q40Codec;
impl Codec for Q40Codec {
      fn name(&self) -> &'static str {
            return "Q4_0";
      }
      fn block_bytes(&self) -> usize {
            return 18;
      }
      fn block_elems(&self) -> usize {
            return 32;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let d = f16_to_f32(u16::from_le_bytes([raw[0], raw[1]]));
            let qs = &raw[2..18];
            let mut blk = [0f32; 32];
            for j in 0..16 {
                  let x0 = (qs[j] & 0x0F) as i32 - 8;
                  let x1 = (qs[j] >> 4) as i32 - 8;
                  blk[j] = x0 as f32 * d;
                  blk[j + 16] = x1 as f32 * d;
            }
            out.extend_from_slice(&blk);
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q4_0 encode not implemented");
      }
}

/// Q4_1: `dequantize_row_q4_1` (ggml/src/ggml-quants.c). 20B/32: `d`,`m`(f16)
/// then 16 packed nibbles; `nibble*d + m`.
struct Q41Codec;
static Q41C: Q41Codec = Q41Codec;
impl Codec for Q41Codec {
      fn name(&self) -> &'static str {
            return "Q4_1";
      }
      fn block_bytes(&self) -> usize {
            return 20;
      }
      fn block_elems(&self) -> usize {
            return 32;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let d = f16_to_f32(u16::from_le_bytes([raw[0], raw[1]]));
            let m = f16_to_f32(u16::from_le_bytes([raw[2], raw[3]]));
            let qs = &raw[4..20];
            let mut blk = [0f32; 32];
            for j in 0..16 {
                  blk[j] = (qs[j] & 0x0F) as f32 * d + m;
                  blk[j + 16] = (qs[j] >> 4) as f32 * d + m;
            }
            out.extend_from_slice(&blk);
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q4_1 encode not implemented");
      }
}

/// Q5_0: `dequantize_row_q5_0` (ggml/src/ggml-quants.c). 22B/32: `d`(f16),
/// 32-bit high-bit field, then 16 low nibbles; `(q - 16)*d`.
struct Q50Codec;
static Q50C: Q50Codec = Q50Codec;
impl Codec for Q50Codec {
      fn name(&self) -> &'static str {
            return "Q5_0";
      }
      fn block_bytes(&self) -> usize {
            return 22;
      }
      fn block_elems(&self) -> usize {
            return 32;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let d = f16_to_f32(u16::from_le_bytes([raw[0], raw[1]]));
            let qh = u32::from_le_bytes([raw[2], raw[3], raw[4], raw[5]]);
            let ql = &raw[6..22];
            for i in 0..32 {
                  let x = ((qh >> i) & 1) << 4;
                  let q = if i < 16 {
                        (ql[i] & 0xF) as u32 | x
                  } else {
                        (ql[i - 16] >> 4) as u32 | x
                  };
                  out.push(d * (q as f32 - 16.0));
            }
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q5_0 encode not implemented");
      }
}

/// Q5_1: `dequantize_row_q5_1` (ggml/src/ggml-quants.c). 24B/32: `d`,`m`(f16),
/// 32-bit high-bit field, then 16 low nibbles; `q*d + m`.
struct Q51Codec;
static Q51C: Q51Codec = Q51Codec;
impl Codec for Q51Codec {
      fn name(&self) -> &'static str {
            return "Q5_1";
      }
      fn block_bytes(&self) -> usize {
            return 24;
      }
      fn block_elems(&self) -> usize {
            return 32;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let d = f16_to_f32(u16::from_le_bytes([raw[0], raw[1]]));
            let m = f16_to_f32(u16::from_le_bytes([raw[2], raw[3]]));
            let qh = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
            let qs = &raw[8..24];
            let mut blk = [0f32; 32];
            for j in 0..16 {
                  let xh0 = ((qh >> j) << 4) & 0x10;
                  let xh1 = (qh >> (j + 12)) & 0x10;
                  let x0 = ((qs[j] & 0x0F) as u32 | xh0) as f32;
                  let x1 = ((qs[j] >> 4) as u32 | xh1) as f32;
                  blk[j] = x0 * d + m;
                  blk[j + 16] = x1 * d + m;
            }
            out.extend_from_slice(&blk);
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q5_1 encode not implemented");
      }
}

struct Q80Codec;
static Q80C: Q80Codec = Q80Codec;
impl Codec for Q80Codec {
      fn name(&self) -> &'static str {
            return "Q8_0";
      }
      fn block_bytes(&self) -> usize {
            return 34;
      }
      fn block_elems(&self) -> usize {
            return 32;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let d = f16_to_f32(u16::from_le_bytes([raw[0], raw[1]]));
            for i in 0..32 {
                  out.push(d * (raw[2 + i] as i8 as f32));
            }
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q8_0 encode not implemented");
      }
}

/// Q2_K: `dequantize_row_q2_K` (ggml/src/ggml-quants.c). 84B/256: `scales`[16],
/// `qs`[64], then `d`,`dmin`(f16). 2-bit quants with per-16 4-bit scale+min.
struct Q2KCodec;
static Q2KC: Q2KCodec = Q2KCodec;
impl Codec for Q2KCodec {
      fn name(&self) -> &'static str {
            return "Q2_K";
      }
      fn block_bytes(&self) -> usize {
            return 84;
      }
      fn block_elems(&self) -> usize {
            return 256;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let scales = &raw[0..16];
            let qs = &raw[16..80];
            let d = f16_to_f32(u16::from_le_bytes([raw[80], raw[81]]));
            let min = f16_to_f32(u16::from_le_bytes([raw[82], raw[83]]));
            let mut is = 0;
            for nb in 0..2 {
                  let q = &qs[nb * 32..];
                  let mut shift = 0;
                  for _ in 0..4 {
                        let sc = scales[is];
                        is += 1;
                        let (dl, ml) = (d * (sc & 0xF) as f32, min * (sc >> 4) as f32);
                        for l in 0..16 {
                              out.push(dl * (((q[l] >> shift) & 3) as f32) - ml);
                        }
                        let sc = scales[is];
                        is += 1;
                        let (dl, ml) = (d * (sc & 0xF) as f32, min * (sc >> 4) as f32);
                        for l in 0..16 {
                              out.push(dl * (((q[l + 16] >> shift) & 3) as f32) - ml);
                        }
                        shift += 2;
                  }
            }
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q2_K encode not implemented");
      }
}

/// Q3_K: `dequantize_row_q3_K` (ggml/src/ggml-quants.c). 110B/256: `hmask`[32],
/// `qs`[64], `scales`[12], then `d`(f16). 3-bit quants; the 16 6-bit scales are
/// unpacked from the 12 scale bytes exactly as the reference `aux` shuffle.
struct Q3KCodec;
static Q3KC: Q3KCodec = Q3KCodec;
impl Codec for Q3KCodec {
      fn name(&self) -> &'static str {
            return "Q3_K";
      }
      fn block_bytes(&self) -> usize {
            return 110;
      }
      fn block_elems(&self) -> usize {
            return 256;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let hm = &raw[0..32];
            let qs = &raw[32..96];
            let scraw = &raw[96..108];
            let d_all = f16_to_f32(u16::from_le_bytes([raw[108], raw[109]]));
            let kmask1 = 0x03030303u32;
            let kmask2 = 0x0f0f0f0fu32;
            let mut aux = [
                  u32::from_le_bytes([scraw[0], scraw[1], scraw[2], scraw[3]]),
                  u32::from_le_bytes([scraw[4], scraw[5], scraw[6], scraw[7]]),
                  u32::from_le_bytes([scraw[8], scraw[9], scraw[10], scraw[11]]),
                  0u32,
            ];
            let tmp = aux[2];
            aux[2] = ((aux[0] >> 4) & kmask2) | (((tmp >> 4) & kmask1) << 4);
            aux[3] = ((aux[1] >> 4) & kmask2) | (((tmp >> 6) & kmask1) << 4);
            aux[0] = (aux[0] & kmask2) | (((tmp >> 0) & kmask1) << 4);
            aux[1] = (aux[1] & kmask2) | (((tmp >> 2) & kmask1) << 4);
            let mut sc = [0i8; 16];
            for i in 0..4 {
                  let b = aux[i].to_le_bytes();
                  for k in 0..4 {
                        sc[i * 4 + k] = b[k] as i8;
                  }
            }
            let mut m = 1u8;
            let mut is = 0;
            for nb in 0..2 {
                  let q = &qs[nb * 32..];
                  let mut shift = 0;
                  for _ in 0..4 {
                        let dl = d_all * (sc[is] as i32 - 32) as f32;
                        is += 1;
                        for l in 0..16 {
                              let hbit = if hm[l] & m != 0 { 0 } else { 4 };
                              let v = ((q[l] >> shift) & 3) as i8 as i32 - hbit;
                              out.push(dl * v as f32);
                        }
                        let dl = d_all * (sc[is] as i32 - 32) as f32;
                        is += 1;
                        for l in 0..16 {
                              let hbit = if hm[l + 16] & m != 0 { 0 } else { 4 };
                              let v = ((q[l + 16] >> shift) & 3) as i8 as i32 - hbit;
                              out.push(dl * v as f32);
                        }
                        shift += 2;
                        m <<= 1;
                  }
            }
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q3_K encode not implemented");
      }
}

struct Q4KCodec;
static Q4KC: Q4KCodec = Q4KCodec;
impl Codec for Q4KCodec {
      fn name(&self) -> &'static str {
            return "Q4_K";
      }
      fn block_bytes(&self) -> usize {
            return 144;
      }
      fn block_elems(&self) -> usize {
            return 256;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let d = f16_to_f32(u16::from_le_bytes([raw[0], raw[1]]));
            let dmin = f16_to_f32(u16::from_le_bytes([raw[2], raw[3]]));
            let sc = &raw[4..16];
            let qs = &raw[16..144];
            let mut is = 0;
            let mut q = 0;
            for _ in 0..4 {
                  let (s1, m1) = get_scale_min(is, sc);
                  let (d1, mm1) = (d * s1 as f32, dmin * m1 as f32);
                  let (s2, m2) = get_scale_min(is + 1, sc);
                  let (d2, mm2) = (d * s2 as f32, dmin * m2 as f32);
                  for l in 0..32 {
                        out.push(d1 * ((qs[q + l] & 0xF) as f32) - mm1);
                  }
                  for l in 0..32 {
                        out.push(d2 * ((qs[q + l] >> 4) as f32) - mm2);
                  }
                  q += 32;
                  is += 2;
            }
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q4_K encode not implemented");
      }
}

/// Q5_K: `dequantize_row_q5_K` (ggml/src/ggml-quants.c). 176B/256: `d`,`dmin`
/// (f16), `scales`[12], `qh`[32], `qs`[128]. 5-bit quants; low nibble plus the
/// per-element high bit, per-32 6-bit scale+min via `get_scale_min`.
struct Q5KCodec;
static Q5KC: Q5KCodec = Q5KCodec;
impl Codec for Q5KCodec {
      fn name(&self) -> &'static str {
            return "Q5_K";
      }
      fn block_bytes(&self) -> usize {
            return 176;
      }
      fn block_elems(&self) -> usize {
            return 256;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let d = f16_to_f32(u16::from_le_bytes([raw[0], raw[1]]));
            let min = f16_to_f32(u16::from_le_bytes([raw[2], raw[3]]));
            let scales = &raw[4..16];
            let qh = &raw[16..48];
            let qs = &raw[48..176];
            let mut is = 0;
            let mut u1 = 1u8;
            let mut u2 = 2u8;
            for jb in 0..4 {
                  let ql = &qs[jb * 32..];
                  let (s1, mn1) = get_scale_min(is, scales);
                  let (d1, m1) = (d * s1 as f32, min * mn1 as f32);
                  let (s2, mn2) = get_scale_min(is + 1, scales);
                  let (d2, m2) = (d * s2 as f32, min * mn2 as f32);
                  for l in 0..32 {
                        let hi = if qh[l] & u1 != 0 { 16 } else { 0 };
                        out.push(d1 * ((ql[l] & 0xF) as i32 + hi) as f32 - m1);
                  }
                  for l in 0..32 {
                        let hi = if qh[l] & u2 != 0 { 16 } else { 0 };
                        out.push(d2 * ((ql[l] >> 4) as i32 + hi) as f32 - m2);
                  }
                  is += 2;
                  u1 <<= 2;
                  u2 <<= 2;
            }
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q5_K encode not implemented");
      }
}

struct Q6KCodec;
static Q6KC: Q6KCodec = Q6KCodec;
impl Codec for Q6KCodec {
      fn name(&self) -> &'static str {
            return "Q6_K";
      }
      fn block_bytes(&self) -> usize {
            return 210;
      }
      fn block_elems(&self) -> usize {
            return 256;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let ql = &raw[0..128];
            let qh = &raw[128..192];
            let sc: Vec<i8> = raw[192..208].iter().map(|&x| x as i8).collect();
            let d = f16_to_f32(u16::from_le_bytes([raw[208], raw[209]]));
            let mut blk = [0f32; 256];
            for nn in 0..2 {
                  let (qlb, qhb, scb) = (&ql[nn * 64..], &qh[nn * 32..], &sc[nn * 8..]);
                  for l in 0..32 {
                        let is = l / 16;
                        let q1 = ((qlb[l] & 0xF) | ((qhb[l] & 3) << 4)) as i32 - 32;
                        let q2 =
                              ((qlb[l + 32] & 0xF) | (((qhb[l] >> 2) & 3) << 4)) as i32 - 32;
                        let q3 = ((qlb[l] >> 4) | (((qhb[l] >> 4) & 3) << 4)) as i32 - 32;
                        let q4 =
                              ((qlb[l + 32] >> 4) | (((qhb[l] >> 6) & 3) << 4)) as i32 - 32;
                        blk[nn * 128 + l] = d * scb[is] as f32 * q1 as f32;
                        blk[nn * 128 + l + 32] = d * scb[is + 2] as f32 * q2 as f32;
                        blk[nn * 128 + l + 64] = d * scb[is + 4] as f32 * q3 as f32;
                        blk[nn * 128 + l + 96] = d * scb[is + 6] as f32 * q4 as f32;
                  }
            }
            out.extend_from_slice(&blk);
            return Ok(());
      }
      fn encode_block(&self, _vals: &[f32], _out: &mut Vec<u8>) -> Result<()> {
            bail!("Q6_K encode not implemented");
      }
}
