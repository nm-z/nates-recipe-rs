use anyhow::{Result, bail, ensure};

/// Numeric interpretation applied to an image/typed-buffer channel of a given
/// bit width. This is a format axis on the image codecs, not a standalone
/// width: the same channel bits mean different values under each.
///
/// - `Unorm`   raw / (2^w - 1)                       -> [0, 1]
/// - `Snorm`   max(signext / (2^(w-1) - 1), -1)      -> [-1, 1]
/// - `Uscaled` raw as f32 (unsigned integer value)
/// - `Sscaled` signext as f32 (signed integer value)
/// - `Uint`    raw as f32
/// - `Sint`    signext as f32
/// - `Float`   IEEE/mini float of the channel width (10/11/16/32)
/// - `Srgb`    `Unorm` then the sRGB EOTF on colour channels (alpha stays linear)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImgFmt {
      Unorm,
      Snorm,
      Uscaled,
      Sscaled,
      Uint,
      Sint,
      Float,
      Srgb,
}

/// Image packed channel layout: the per-channel bit widths, listed low-order
/// channel first. Channels pack little-endian within the block, the first
/// listed channel occupying the least-significant bits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImgLayout {
      L8,
      L8_8,
      L8_8_8_8,
      L16,
      L16_16,
      L16_16_16_16,
      L32,
      L32_32,
      L32_32_32,
      L32_32_32_32,
      L5_6_5,
      L1_5_5_5,
      L5_5_5_1,
      L10_10_10_2,
      L2_10_10_10,
      L10_11_11,
      L11_11_10,
}

impl ImgLayout {
      /// Per-channel bit widths, low-order channel first.
      pub fn channels(self) -> &'static [u32] {
            return match self {
                  ImgLayout::L8 => &[8],
                  ImgLayout::L8_8 => &[8, 8],
                  ImgLayout::L8_8_8_8 => &[8, 8, 8, 8],
                  ImgLayout::L16 => &[16],
                  ImgLayout::L16_16 => &[16, 16],
                  ImgLayout::L16_16_16_16 => &[16, 16, 16, 16],
                  ImgLayout::L32 => &[32],
                  ImgLayout::L32_32 => &[32, 32],
                  ImgLayout::L32_32_32 => &[32, 32, 32],
                  ImgLayout::L32_32_32_32 => &[32, 32, 32, 32],
                  ImgLayout::L5_6_5 => &[5, 6, 5],
                  ImgLayout::L1_5_5_5 => &[1, 5, 5, 5],
                  ImgLayout::L5_5_5_1 => &[5, 5, 5, 1],
                  ImgLayout::L10_10_10_2 => &[10, 10, 10, 2],
                  ImgLayout::L2_10_10_10 => &[2, 10, 10, 10],
                  ImgLayout::L10_11_11 => &[10, 11, 11],
                  ImgLayout::L11_11_10 => &[11, 11, 10],
            };
      }

      fn nm(self) -> &'static str {
            return match self {
                  ImgLayout::L8 => "img8",
                  ImgLayout::L8_8 => "img8_8",
                  ImgLayout::L8_8_8_8 => "img8_8_8_8",
                  ImgLayout::L16 => "img16",
                  ImgLayout::L16_16 => "img16_16",
                  ImgLayout::L16_16_16_16 => "img16_16_16_16",
                  ImgLayout::L32 => "img32",
                  ImgLayout::L32_32 => "img32_32",
                  ImgLayout::L32_32_32 => "img32_32_32",
                  ImgLayout::L32_32_32_32 => "img32_32_32_32",
                  ImgLayout::L5_6_5 => "img5_6_5",
                  ImgLayout::L1_5_5_5 => "img1_5_5_5",
                  ImgLayout::L5_5_5_1 => "img5_5_5_1",
                  ImgLayout::L10_10_10_2 => "img10_10_10_2",
                  ImgLayout::L2_10_10_10 => "img2_10_10_10",
                  ImgLayout::L10_11_11 => "img10_11_11",
                  ImgLayout::L11_11_10 => "img11_11_10",
            };
      }
}

/// The dtypes this one codec surface understands. The GGUF quant grid, the
/// full-width floats, the typeless bit widths, signed/unsigned/packed integers,
/// the OCP-MX / AMD mini-floats, and the image channel layouts are ALL siblings
/// in this single enum, converted through the one f32 pivot by `convert`. Quant
/// variants decode only; every other variant decodes, and encodes where the
/// encode is meaningful (else it bails, exactly like the quants).
///
/// Lossy through the f32 pivot (documented per codec): `F64`/`F64x2`,
/// `I32`/`U32`/`I64`/`U64` above 2^24, `Tf32`/`Xf32` (10-bit mantissa
/// truncation), and any 32-bit image channel under `Uint`/`Sint`/`Float`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
      B8,
      B16,
      B32,
      B64,
      B96,
      B128,
      B256,
      B512,
      B16x2,
      B16x3,
      B16x4,
      I4,
      I8,
      I16,
      I24,
      I32,
      I64,
      U4,
      U8,
      U16,
      U24,
      U32,
      U64,
      I4x8,
      U4x8,
      I8x4,
      U8x4,
      I16x2,
      U16x2,
      F4e2m1,
      F6e2m3,
      F6e3m2,
      F8e4m3,
      F8e5m2,
      F8e4m3fnuz,
      F8e5m2fnuz,
      Bf8,
      Tf32,
      Xf32,
      F64,
      F4x2e2m1,
      F4x4e2m1,
      F6x2e2m3,
      F6x4e2m3,
      F6x2e3m2,
      F6x4e3m2,
      F8x2e4m3,
      F8x4e4m3,
      F8x2e5m2,
      F8x4e5m2,
      F8x2e4m3fnuz,
      F8x4e4m3fnuz,
      F8x2e5m2fnuz,
      F8x4e5m2fnuz,
      F16x2,
      Bf16x2,
      F32x2,
      F64x2,
      Byte,
      Short,
      Dword,
      Dwordx2,
      Dwordx3,
      Dwordx4,
      Dwordx8,
      Dwordx16,
      Img(ImgLayout, ImgFmt),
}

/// Either a borrowed static codec (the fixed quant/float grid) or an owned,
/// parameterised codec (ints, mini-floats, typeless widths, images). `get`
/// hands back the trait object either way, so `convert` sees one surface.
enum Cx {
      S(&'static dyn Codec),
      Own(Box<dyn Codec>),
}

impl Cx {
      fn get(&self) -> &dyn Codec {
            return match self {
                  Cx::S(c) => *c,
                  Cx::Own(b) => b.as_ref(),
            };
      }
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

      fn codec(self) -> Cx {
            return match self {
                  DType::F32 => Cx::S(&F32C),
                  DType::F16 => Cx::S(&F16C),
                  DType::Bf16 => Cx::S(&Bf16C),
                  DType::Q4_0 => Cx::S(&Q40C),
                  DType::Q4_1 => Cx::S(&Q41C),
                  DType::Q5_0 => Cx::S(&Q50C),
                  DType::Q5_1 => Cx::S(&Q51C),
                  DType::Q8_0 => Cx::S(&Q80C),
                  DType::Q2_K => Cx::S(&Q2KC),
                  DType::Q3_K => Cx::S(&Q3KC),
                  DType::Q4_K => Cx::S(&Q4KC),
                  DType::Q5_K => Cx::S(&Q5KC),
                  DType::Q6_K => Cx::S(&Q6KC),
                  DType::B8 => Cx::Own(Box::new(IntCodec::new("b8", 8, false, 1))),
                  DType::B16 => Cx::Own(Box::new(IntCodec::new("b16", 16, false, 1))),
                  DType::B32 => Cx::Own(Box::new(BitcastCodec::new("b32", 1))),
                  DType::B64 => Cx::Own(Box::new(BitcastCodec::new("b64", 2))),
                  DType::B96 => Cx::Own(Box::new(BitcastCodec::new("b96", 3))),
                  DType::B128 => Cx::Own(Box::new(BitcastCodec::new("b128", 4))),
                  DType::B256 => Cx::Own(Box::new(BitcastCodec::new("b256", 8))),
                  DType::B512 => Cx::Own(Box::new(BitcastCodec::new("b512", 16))),
                  DType::B16x2 => Cx::Own(Box::new(IntCodec::new("b16x2", 16, false, 2))),
                  DType::B16x3 => Cx::Own(Box::new(IntCodec::new("b16x3", 16, false, 3))),
                  DType::B16x4 => Cx::Own(Box::new(IntCodec::new("b16x4", 16, false, 4))),
                  DType::I4 => Cx::Own(Box::new(IntCodec::new("i4", 4, true, 2))),
                  DType::I8 => Cx::Own(Box::new(IntCodec::new("i8", 8, true, 1))),
                  DType::I16 => Cx::Own(Box::new(IntCodec::new("i16", 16, true, 1))),
                  DType::I24 => Cx::Own(Box::new(IntCodec::new("i24", 24, true, 1))),
                  DType::I32 => Cx::Own(Box::new(IntCodec::new("i32", 32, true, 1))),
                  DType::I64 => Cx::Own(Box::new(IntCodec::new("i64", 64, true, 1))),
                  DType::U4 => Cx::Own(Box::new(IntCodec::new("u4", 4, false, 2))),
                  DType::U8 => Cx::Own(Box::new(IntCodec::new("u8", 8, false, 1))),
                  DType::U16 => Cx::Own(Box::new(IntCodec::new("u16", 16, false, 1))),
                  DType::U24 => Cx::Own(Box::new(IntCodec::new("u24", 24, false, 1))),
                  DType::U32 => Cx::Own(Box::new(IntCodec::new("u32", 32, false, 1))),
                  DType::U64 => Cx::Own(Box::new(IntCodec::new("u64", 64, false, 1))),
                  DType::I4x8 => Cx::Own(Box::new(IntCodec::new("i4x8", 4, true, 8))),
                  DType::U4x8 => Cx::Own(Box::new(IntCodec::new("u4x8", 4, false, 8))),
                  DType::I8x4 => Cx::Own(Box::new(IntCodec::new("i8x4", 8, true, 4))),
                  DType::U8x4 => Cx::Own(Box::new(IntCodec::new("u8x4", 8, false, 4))),
                  DType::I16x2 => Cx::Own(Box::new(IntCodec::new("i16x2", 16, true, 2))),
                  DType::U16x2 => Cx::Own(Box::new(IntCodec::new("u16x2", 16, false, 2))),
                  DType::F4e2m1 => Cx::Own(Box::new(FpCodec::e2m1("f4_e2m1", 2))),
                  DType::F6e2m3 => Cx::Own(Box::new(FpCodec::e2m3("f6_e2m3", 4))),
                  DType::F6e3m2 => Cx::Own(Box::new(FpCodec::e3m2("f6_e3m2", 4))),
                  DType::F8e4m3 => Cx::Own(Box::new(FpCodec::e4m3("f8_e4m3", 1, false))),
                  DType::F8e5m2 => Cx::Own(Box::new(FpCodec::e5m2("f8_e5m2", 1, false))),
                  DType::F8e4m3fnuz => {
                        Cx::Own(Box::new(FpCodec::e4m3("f8_e4m3_fnuz", 1, true)))
                  }
                  DType::F8e5m2fnuz => {
                        Cx::Own(Box::new(FpCodec::e5m2("f8_e5m2_fnuz", 1, true)))
                  }
                  DType::Bf8 => Cx::Own(Box::new(FpCodec::e5m2("bf8", 1, false))),
                  DType::Tf32 => Cx::Own(Box::new(WideCodec::new("tf32", WideKind::Tf32, 1))),
                  DType::Xf32 => Cx::Own(Box::new(WideCodec::new("xf32", WideKind::Tf32, 1))),
                  DType::F64 => Cx::Own(Box::new(WideCodec::new("f64", WideKind::F64, 1))),
                  DType::F4x2e2m1 => Cx::Own(Box::new(FpCodec::e2m1("f4x2_e2m1", 2))),
                  DType::F4x4e2m1 => Cx::Own(Box::new(FpCodec::e2m1("f4x4_e2m1", 4))),
                  DType::F6x2e2m3 => Cx::Own(Box::new(FpCodec::e2m3("f6x2_e2m3", 4))),
                  DType::F6x4e2m3 => Cx::Own(Box::new(FpCodec::e2m3("f6x4_e2m3", 4))),
                  DType::F6x2e3m2 => Cx::Own(Box::new(FpCodec::e3m2("f6x2_e3m2", 4))),
                  DType::F6x4e3m2 => Cx::Own(Box::new(FpCodec::e3m2("f6x4_e3m2", 4))),
                  DType::F8x2e4m3 => Cx::Own(Box::new(FpCodec::e4m3("f8x2_e4m3", 2, false))),
                  DType::F8x4e4m3 => Cx::Own(Box::new(FpCodec::e4m3("f8x4_e4m3", 4, false))),
                  DType::F8x2e5m2 => Cx::Own(Box::new(FpCodec::e5m2("f8x2_e5m2", 2, false))),
                  DType::F8x4e5m2 => Cx::Own(Box::new(FpCodec::e5m2("f8x4_e5m2", 4, false))),
                  DType::F8x2e4m3fnuz => {
                        Cx::Own(Box::new(FpCodec::e4m3("f8x2_e4m3_fnuz", 2, true)))
                  }
                  DType::F8x4e4m3fnuz => {
                        Cx::Own(Box::new(FpCodec::e4m3("f8x4_e4m3_fnuz", 4, true)))
                  }
                  DType::F8x2e5m2fnuz => {
                        Cx::Own(Box::new(FpCodec::e5m2("f8x2_e5m2_fnuz", 2, true)))
                  }
                  DType::F8x4e5m2fnuz => {
                        Cx::Own(Box::new(FpCodec::e5m2("f8x4_e5m2_fnuz", 4, true)))
                  }
                  DType::F16x2 => Cx::Own(Box::new(WideCodec::new("f16x2", WideKind::F16, 2))),
                  DType::Bf16x2 => {
                        Cx::Own(Box::new(WideCodec::new("bf16x2", WideKind::Bf16, 2)))
                  }
                  DType::F32x2 => Cx::Own(Box::new(WideCodec::new("f32x2", WideKind::F32, 2))),
                  DType::F64x2 => Cx::Own(Box::new(WideCodec::new("f64x2", WideKind::F64, 2))),
                  DType::Byte => Cx::Own(Box::new(IntCodec::new("byte", 8, false, 1))),
                  DType::Short => Cx::Own(Box::new(IntCodec::new("short", 16, false, 1))),
                  DType::Dword => Cx::Own(Box::new(BitcastCodec::new("dword", 1))),
                  DType::Dwordx2 => Cx::Own(Box::new(BitcastCodec::new("dwordx2", 2))),
                  DType::Dwordx3 => Cx::Own(Box::new(BitcastCodec::new("dwordx3", 3))),
                  DType::Dwordx4 => Cx::Own(Box::new(BitcastCodec::new("dwordx4", 4))),
                  DType::Dwordx8 => Cx::Own(Box::new(BitcastCodec::new("dwordx8", 8))),
                  DType::Dwordx16 => Cx::Own(Box::new(BitcastCodec::new("dwordx16", 16))),
                  DType::Img(l, f) => Cx::Own(Box::new(ImageCodec { layout: l, fmt: f })),
            };
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
      let scx = src.codec();
      let dcx = dst.codec();
      let sc = scx.get();
      let dc = dcx.get();
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

/// Block layout `(block_bytes, block_elems)` for a GGML type.
///
/// # Errors
/// Returns an error naming the unsupported GGML type tag.
pub fn block_layout(t: u32) -> Result<(usize, usize)> {
      let d = DType::from_ggml(t)?;
      let cx = d.codec();
      let c = cx.get();
      return Ok((c.block_bytes(), c.block_elems()));
}

/// # Errors
/// Returns an error naming the unsupported GGML type tag.
pub fn nbytes_for(t: u32, elems: usize) -> Result<usize> {
      let (block_bytes, block_elems) = block_layout(t)?;
      return Ok((elems / block_elems) * block_bytes);
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

/// Read `width` bits (LSB-first, little-endian within the block) starting at
/// bit offset `start` from `raw`.
fn read_bits_le(raw: &[u8], start: usize, width: u32) -> u64 {
      let mut v = 0u64;
      for i in 0..width as usize {
            let bitpos = start + i;
            let bit = (raw[bitpos / 8] >> (bitpos % 8)) & 1;
            v |= (bit as u64) << i;
      }
      return v;
}

/// Write the low `width` bits of `val` (LSB-first, little-endian within the
/// block) into `raw` starting at bit offset `start`. `raw` must be pre-zeroed.
fn write_bits_le(raw: &mut [u8], start: usize, width: u32, val: u64) {
      for i in 0..width as usize {
            let bit = ((val >> i) & 1) as u8;
            let bitpos = start + i;
            raw[bitpos / 8] |= bit << (bitpos % 8);
      }
}

/// Sign-extend the low `width` bits of `u` to a full `i64`.
fn sign_extend(u: u64, width: u32) -> i64 {
      let shift = 64 - width;
      return ((u << shift) as i64) >> shift;
}

/// Signed/unsigned integer lanes packed little-endian within a block. Covers
/// the `iN`/`uN` scalars, the packed `iNxM`/`uNxM` vectors, and the typeless
/// `b8`/`b16`/`b16xM` widths (unsigned). Each lane maps to one f32.
///
/// # Lossy through f32
/// `bits > 24` (`i32`/`u32`/`i64`/`u64`) cannot round-trip integer values above
/// 2^24; the f32 pivot rounds to the nearest representable magnitude.
struct IntCodec {
      nm: &'static str,
      bits: u32,
      signed: bool,
      lanes: usize,
}

impl IntCodec {
      fn new(nm: &'static str, bits: u32, signed: bool, lanes: usize) -> IntCodec {
            return IntCodec {
                  nm,
                  bits,
                  signed,
                  lanes,
            };
      }
}

impl Codec for IntCodec {
      fn name(&self) -> &'static str {
            return self.nm;
      }
      fn block_bytes(&self) -> usize {
            return (self.bits as usize * self.lanes) / 8;
      }
      fn block_elems(&self) -> usize {
            return self.lanes;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            for j in 0..self.lanes {
                  let u = read_bits_le(raw, j * self.bits as usize, self.bits);
                  let v = if self.signed {
                        sign_extend(u, self.bits) as f64
                  } else {
                        u as f64
                  };
                  out.push(v as f32);
            }
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            let mut blk = vec![0u8; self.block_bytes()];
            let mask = if self.bits >= 64 {
                  u64::MAX
            } else {
                  (1u64 << self.bits) - 1
            };
            for (j, &x) in vals.iter().enumerate() {
                  let bits = if self.signed {
                        (x as f64).round() as i64 as u64 & mask
                  } else {
                        let r = (x as f64).round();
                        let r = if r < 0.0 { 0.0 } else { r };
                        (r as u64) & mask
                  };
                  write_bits_le(&mut blk, j * self.bits as usize, self.bits, bits);
            }
            out.extend_from_slice(&blk);
            return Ok(());
      }
}

/// Typeless bit widths that are a multiple of 32 bits: `b32`/`b64`/`b96`/…/`b512`
/// and the `dword*` aliases. Each 32-bit lane is a raw f32 bitcast (no numeric
/// conversion), so any typeless width converts to any other by reinterpreting
/// the same bits, and to/from `F32` losslessly.
struct BitcastCodec {
      nm: &'static str,
      lanes: usize,
}

impl BitcastCodec {
      fn new(nm: &'static str, lanes: usize) -> BitcastCodec {
            return BitcastCodec { nm, lanes };
      }
}

impl Codec for BitcastCodec {
      fn name(&self) -> &'static str {
            return self.nm;
      }
      fn block_bytes(&self) -> usize {
            return 4 * self.lanes;
      }
      fn block_elems(&self) -> usize {
            return self.lanes;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            for j in 0..self.lanes {
                  let o = j * 4;
                  out.push(f32::from_le_bytes([
                        raw[o],
                        raw[o + 1],
                        raw[o + 2],
                        raw[o + 3],
                  ]));
            }
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            for &x in vals {
                  out.extend_from_slice(&x.to_le_bytes());
            }
            return Ok(());
      }
}

/// Special-value discipline of a mini-float format.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FpSpecial {
      /// No infinities and no NaN; every bit pattern is a finite number.
      Finite,
      /// OCP e4m3: no infinities; the single all-ones-exponent, all-ones-mantissa
      /// pattern is NaN, every other max-exponent pattern is a normal number.
      NanMaxMant,
      /// IEEE-like: max exponent with zero mantissa is infinity, otherwise NaN.
      Ieee,
      /// AMD `fnuz`: no infinities; the sign-set, zero-exponent, zero-mantissa
      /// pattern is the sole NaN (there is no negative zero).
      Fnuz,
}

/// Decode a mini-float bit pattern (OCP-MX / AMD conventions) to f32.
fn minifloat_to_f32(
      bits: u32,
      ebits: u32,
      mbits: u32,
      bias: i32,
      special: FpSpecial,
      signed: bool,
) -> f32 {
      let sign = if signed {
            (bits >> (ebits + mbits)) & 1
      } else {
            0
      };
      let exp = (bits >> mbits) & ((1 << ebits) - 1);
      let mant = bits & ((1 << mbits) - 1);
      let emax = (1u32 << ebits) - 1;
      let maxmant = (1u32 << mbits) - 1;
      let sgn = if sign == 1 { -1.0f32 } else { 1.0 };
      if special == FpSpecial::Fnuz && sign == 1 && exp == 0 && mant == 0 {
            return f32::NAN;
      }
      if special == FpSpecial::Ieee && exp == emax {
            if mant == 0 {
                  return sgn * f32::INFINITY;
            }
            return f32::NAN;
      }
      if special == FpSpecial::NanMaxMant && exp == emax && mant == maxmant {
            return f32::NAN;
      }
      let mscale = (1u32 << mbits) as f32;
      if exp == 0 {
            let sub = 2f32.powi(1 - bias) * (mant as f32 / mscale);
            return sgn * sub;
      }
      let val = 2f32.powi(exp as i32 - bias) * (1.0 + mant as f32 / mscale);
      return sgn * val;
}

/// Encode an f32 to the nearest mini-float bit pattern (round half to even).
fn f32_to_minifloat(
      x: f32,
      ebits: u32,
      mbits: u32,
      bias: i32,
      special: FpSpecial,
      signed: bool,
) -> u32 {
      let emax = (1u32 << ebits) - 1;
      let maxmant = (1u32 << mbits) - 1;
      let sfield = if signed && x.is_sign_negative() {
            1u32 << (ebits + mbits)
      } else {
            0
      };
      if x.is_nan() {
            return match special {
                  FpSpecial::Ieee => sfield | (emax << mbits) | 1,
                  FpSpecial::NanMaxMant => sfield | (emax << mbits) | maxmant,
                  FpSpecial::Fnuz => 1u32 << (ebits + mbits),
                  FpSpecial::Finite => sfield,
            };
      }
      let a = x.abs() as f64;
      if a == 0.0 {
            return if special == FpSpecial::Fnuz { 0 } else { sfield };
      }
      let nmax = if special == FpSpecial::Ieee {
            emax - 1
      } else {
            emax
      };
      let top_mant = if special == FpSpecial::NanMaxMant {
            maxmant - 1
      } else {
            maxmant
      };
      let mscale = (1u64 << mbits) as f64;
      let max_val = 2f64.powi(nmax as i32 - bias) * (1.0 + top_mant as f64 / mscale);
      if x.is_infinite() || a > max_val {
            return match special {
                  FpSpecial::Ieee => sfield | (emax << mbits),
                  _ => sfield | (nmax << mbits) | top_mant,
            };
      }
      let min_normal = 2f64.powi(1 - bias);
      if a >= min_normal {
            let e = a.log2().floor() as i32;
            let e = e.clamp(1 - bias, nmax as i32 - bias);
            let frac = a / 2f64.powi(e) - 1.0;
            let mut m = (frac * mscale).round() as i64;
            let mut te = (e + bias) as u32;
            if m >= (1i64 << mbits) {
                  m = 0;
                  te += 1;
            }
            if te > nmax {
                  return match special {
                        FpSpecial::Ieee => sfield | (emax << mbits),
                        _ => sfield | (nmax << mbits) | top_mant,
                  };
            }
            return sfield | (te << mbits) | (m as u32 & maxmant);
      }
      let unit = 2f64.powi(1 - bias - mbits as i32);
      let mut m = (a / unit).round() as i64;
      if m >= (1i64 << mbits) {
            return sfield | (1u32 << mbits);
      }
      if m == 0 {
            return sfield;
      }
      m &= maxmant as i64;
      return sfield | m as u32;
}

/// A signed mini-float: `ebits`+`mbits`+sign, `lanes` of them packed
/// little-endian per block. Covers f4/f6/f8 and all their `fnuz` and packed
/// (`xN`) variants. `bf8` is `f8_e5m2` (OCP), per the OCP FP8 spec.
///
/// Bit layouts implemented (OCP "OCP Microscaling Formats (MX) v1.0" and AMD
/// CDNA fnuz): e2m1 bias 1 (finite), e2m3 bias 1 (finite), e3m2 bias 3
/// (finite), e4m3 bias 7 (no inf, NaN=S.1111.111), e5m2 bias 15 (IEEE inf/NaN),
/// e4m3_fnuz bias 8 (no inf, NaN=1.0000.000), e5m2_fnuz bias 16 (no inf,
/// NaN=1.00000.00). f6x2 is 12 bits (not byte-aligned) so shares the byte-exact
/// 3-byte / 4-lane block with f6x4.
struct FpCodec {
      nm: &'static str,
      ebits: u32,
      mbits: u32,
      bias: i32,
      special: FpSpecial,
      lanes: usize,
}

impl FpCodec {
      fn e2m1(nm: &'static str, lanes: usize) -> FpCodec {
            return FpCodec {
                  nm,
                  ebits: 2,
                  mbits: 1,
                  bias: 1,
                  special: FpSpecial::Finite,
                  lanes,
            };
      }
      fn e2m3(nm: &'static str, lanes: usize) -> FpCodec {
            return FpCodec {
                  nm,
                  ebits: 2,
                  mbits: 3,
                  bias: 1,
                  special: FpSpecial::Finite,
                  lanes,
            };
      }
      fn e3m2(nm: &'static str, lanes: usize) -> FpCodec {
            return FpCodec {
                  nm,
                  ebits: 3,
                  mbits: 2,
                  bias: 3,
                  special: FpSpecial::Finite,
                  lanes,
            };
      }
      fn e4m3(nm: &'static str, lanes: usize, fnuz: bool) -> FpCodec {
            let (bias, special) = if fnuz {
                  (8, FpSpecial::Fnuz)
            } else {
                  (7, FpSpecial::NanMaxMant)
            };
            return FpCodec {
                  nm,
                  ebits: 4,
                  mbits: 3,
                  bias,
                  special,
                  lanes,
            };
      }
      fn e5m2(nm: &'static str, lanes: usize, fnuz: bool) -> FpCodec {
            let (bias, special) = if fnuz {
                  (16, FpSpecial::Fnuz)
            } else {
                  (15, FpSpecial::Ieee)
            };
            return FpCodec {
                  nm,
                  ebits: 5,
                  mbits: 2,
                  bias,
                  special,
                  lanes,
            };
      }
      fn width(&self) -> u32 {
            return 1 + self.ebits + self.mbits;
      }
}

impl Codec for FpCodec {
      fn name(&self) -> &'static str {
            return self.nm;
      }
      fn block_bytes(&self) -> usize {
            return (self.width() as usize * self.lanes) / 8;
      }
      fn block_elems(&self) -> usize {
            return self.lanes;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let w = self.width();
            for j in 0..self.lanes {
                  let bits = read_bits_le(raw, j * w as usize, w) as u32;
                  out.push(minifloat_to_f32(
                        bits,
                        self.ebits,
                        self.mbits,
                        self.bias,
                        self.special,
                        true,
                  ));
            }
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            let w = self.width();
            let mut blk = vec![0u8; self.block_bytes()];
            for (j, &x) in vals.iter().enumerate() {
                  let bits = f32_to_minifloat(
                        x,
                        self.ebits,
                        self.mbits,
                        self.bias,
                        self.special,
                        true,
                  );
                  write_bits_le(&mut blk, j * w as usize, w, bits as u64);
            }
            out.extend_from_slice(&blk);
            return Ok(());
      }
}

/// Wide native-float lane kind.
#[derive(Clone, Copy)]
enum WideKind {
      F16,
      Bf16,
      F32,
      F64,
      Tf32,
}

/// `lanes` native floats packed contiguously per block: `f16x2`, `bf16x2`,
/// `f32x2`, `f64`/`f64x2`, and `tf32`/`xf32`.
///
/// # Lossy through f32
/// `F64` above 2^24-magnitude precision and `Tf32`/`Xf32` (mantissa truncated
/// to 10 bits) do not carry full source precision across the f32 pivot. `Xf32`
/// is modelled as TF32 (e8m10, 32-bit storage) per the absence of a distinct
/// public XF32 bit spec.
struct WideCodec {
      nm: &'static str,
      kind: WideKind,
      lanes: usize,
}

impl WideCodec {
      fn new(nm: &'static str, kind: WideKind, lanes: usize) -> WideCodec {
            return WideCodec { nm, kind, lanes };
      }
      fn lane_bytes(&self) -> usize {
            return match self.kind {
                  WideKind::F16 | WideKind::Bf16 => 2,
                  WideKind::F32 | WideKind::Tf32 => 4,
                  WideKind::F64 => 8,
            };
      }
}

impl Codec for WideCodec {
      fn name(&self) -> &'static str {
            return self.nm;
      }
      fn block_bytes(&self) -> usize {
            return self.lane_bytes() * self.lanes;
      }
      fn block_elems(&self) -> usize {
            return self.lanes;
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let lb = self.lane_bytes();
            for j in 0..self.lanes {
                  let s = &raw[j * lb..(j + 1) * lb];
                  let v = match self.kind {
                        WideKind::F16 => f16_to_f32(u16::from_le_bytes([s[0], s[1]])),
                        WideKind::Bf16 => {
                              f32::from_bits((u16::from_le_bytes([s[0], s[1]]) as u32) << 16)
                        }
                        WideKind::F32 => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                        WideKind::Tf32 => {
                              let word = u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
                              f32::from_bits(word & 0xFFFF_E000)
                        }
                        WideKind::F64 => f64::from_le_bytes([
                              s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
                        ]) as f32,
                  };
                  out.push(v);
            }
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            for &x in vals {
                  match self.kind {
                        WideKind::F16 => out.extend_from_slice(&f32_to_f16(x).to_le_bytes()),
                        WideKind::Bf16 => {
                              out.extend_from_slice(&f32_to_bf16(x).to_le_bytes())
                        }
                        WideKind::F32 => out.extend_from_slice(&x.to_le_bytes()),
                        WideKind::Tf32 => {
                              out.extend_from_slice(&(x.to_bits() & 0xFFFF_E000).to_le_bytes())
                        }
                        WideKind::F64 => out.extend_from_slice(&(x as f64).to_le_bytes()),
                  }
            }
            return Ok(());
      }
}

/// The sRGB electro-optical transfer function (encoded sRGB -> linear).
fn srgb_to_linear(c: f32) -> f32 {
      if c <= 0.04045 {
            return c / 12.92;
      }
      return ((c + 0.055) / 1.055).powf(2.4);
}

/// The inverse sRGB transfer function (linear -> encoded sRGB).
fn linear_to_srgb(l: f32) -> f32 {
      if l <= 0.0031308 {
            return 12.92 * l;
      }
      return 1.055 * l.powf(1.0 / 2.4) - 0.055;
}

/// Decode one image channel of `width` raw bits under `fmt`. `idx`/`count`
/// identify the channel so `Srgb` can leave the alpha channel (last of four)
/// linear, matching Vulkan `_SRGB` semantics.
fn img_channel_decode(u: u64, width: u32, fmt: ImgFmt, idx: usize, count: usize) -> Result<f32> {
      let maxu = ((1u128 << width) - 1) as f64;
      let smax = ((1u64 << (width - 1)) - 1) as f64;
      let v = match fmt {
            ImgFmt::Uint | ImgFmt::Uscaled => u as f64 as f32,
            ImgFmt::Sint | ImgFmt::Sscaled => sign_extend(u, width) as f64 as f32,
            ImgFmt::Unorm => (u as f64 / maxu) as f32,
            ImgFmt::Snorm => ((sign_extend(u, width) as f64 / smax).max(-1.0)) as f32,
            ImgFmt::Srgb => {
                  let n = (u as f64 / maxu) as f32;
                  if count == 4 && idx == 3 {
                        n
                  } else {
                        srgb_to_linear(n)
                  }
            }
            ImgFmt::Float => match width {
                  32 => f32::from_bits(u as u32),
                  16 => f16_to_f32(u as u16),
                  11 => minifloat_to_f32(u as u32, 5, 6, 15, FpSpecial::Ieee, false),
                  10 => minifloat_to_f32(u as u32, 5, 5, 15, FpSpecial::Ieee, false),
                  other => bail!("image float channel width {other} unsupported"),
            },
      };
      return Ok(v);
}

/// Encode one image channel value `x` to `width` raw bits under `fmt`.
fn img_channel_encode(x: f32, width: u32, fmt: ImgFmt, idx: usize, count: usize) -> Result<u64> {
      let maxu = ((1u128 << width) - 1) as f64;
      let smax = ((1u64 << (width - 1)) - 1) as f64;
      let mask = ((1u128 << width) - 1) as u64;
      let v = match fmt {
            ImgFmt::Uint | ImgFmt::Uscaled => {
                  let r = (x as f64).round().max(0.0);
                  (r as u64) & mask
            }
            ImgFmt::Sint | ImgFmt::Sscaled => ((x as f64).round() as i64 as u64) & mask,
            ImgFmt::Unorm => ((x.clamp(0.0, 1.0) as f64 * maxu).round() as u64) & mask,
            ImgFmt::Snorm => {
                  ((x.clamp(-1.0, 1.0) as f64 * smax).round() as i64 as u64) & mask
            }
            ImgFmt::Srgb => {
                  let n = if count == 4 && idx == 3 {
                        x.clamp(0.0, 1.0)
                  } else {
                        linear_to_srgb(x).clamp(0.0, 1.0)
                  };
                  ((n as f64 * maxu).round() as u64) & mask
            }
            ImgFmt::Float => match width {
                  32 => x.to_bits() as u64,
                  16 => f32_to_f16(x) as u64,
                  11 => f32_to_minifloat(x, 5, 6, 15, FpSpecial::Ieee, false) as u64,
                  10 => f32_to_minifloat(x, 5, 5, 15, FpSpecial::Ieee, false) as u64,
                  other => bail!("image float channel width {other} unsupported"),
            },
      };
      return Ok(v);
}

/// An image/typed-buffer codec: a channel bit layout crossed with a numeric
/// format. Channels unpack to one f32 lane each, low-order channel first, the
/// whole block read as a little-endian word.
struct ImageCodec {
      layout: ImgLayout,
      fmt: ImgFmt,
}

impl Codec for ImageCodec {
      fn name(&self) -> &'static str {
            return self.layout.nm();
      }
      fn block_bytes(&self) -> usize {
            let total: u32 = self.layout.channels().iter().sum();
            return total as usize / 8;
      }
      fn block_elems(&self) -> usize {
            return self.layout.channels().len();
      }
      fn decode_block(&self, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
            let ch = self.layout.channels();
            let count = ch.len();
            let mut off = 0usize;
            for (idx, &w) in ch.iter().enumerate() {
                  let u = read_bits_le(raw, off, w);
                  out.push(img_channel_decode(u, w, self.fmt, idx, count)?);
                  off += w as usize;
            }
            return Ok(());
      }
      fn encode_block(&self, vals: &[f32], out: &mut Vec<u8>) -> Result<()> {
            let ch = self.layout.channels();
            let count = ch.len();
            let mut blk = vec![0u8; self.block_bytes()];
            let mut off = 0usize;
            for (idx, &w) in ch.iter().enumerate() {
                  let bits = img_channel_encode(vals[idx], w, self.fmt, idx, count)?;
                  write_bits_le(&mut blk, off, w, bits);
                  off += w as usize;
            }
            out.extend_from_slice(&blk);
            return Ok(());
      }
}
