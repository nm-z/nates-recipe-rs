use recipe_infer::dequant::{Dtype, ImgFmt, ImgLayout, convert};
use recipe_infer::gguf::Gguf;
use std::path::Path;

fn f32_bytes(vals: &[f32]) -> Vec<u8> {
      let mut b = Vec::with_capacity(vals.len() * 4);
      for &v in vals {
            b.extend_from_slice(&v.to_le_bytes());
      }
      return b;
}

fn from_f32_bytes(b: &[u8]) -> Vec<f32> {
      return b
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
}

fn decode(dt: Dtype, bytes: &[u8]) -> Vec<f32> {
      let raw = convert(dt, Dtype::F32, bytes).expect("decode to f32");
      return from_f32_bytes(&raw);
}

fn encode(dt: Dtype, vals: &[f32]) -> Vec<u8> {
      return convert(Dtype::F32, dt, &f32_bytes(vals)).expect("encode from f32");
}

/// decode(encode(v)) must reproduce every grid value exactly.
fn roundtrip_grid(dt: Dtype, vals: &[f32]) {
      let enc = encode(dt, vals);
      let dec = decode(dt, &enc);
      assert_eq!(dec.len(), vals.len(), "{dt:?}: length mismatch");
      for (i, (&a, &b)) in vals.iter().zip(dec.iter()).enumerate() {
            if a.is_nan() {
                  assert!(b.is_nan(), "{dt:?}[{i}]: expected NaN, got {b}");
            } else {
                  assert_eq!(a, b, "{dt:?}[{i}]: grid value not stable");
            }
      }
}

#[test]
fn int_codecs_roundtrip_full_range() {
      roundtrip_grid(Dtype::I4, &[-8.0, -1.0, 0.0, 7.0]);
      roundtrip_grid(Dtype::U4, &[0.0, 1.0, 8.0, 15.0]);
      let i8v: Vec<f32> = (-128..=127).map(|x| x as f32).collect();
      roundtrip_grid(Dtype::I8, &i8v);
      let u8v: Vec<f32> = (0..=255).map(|x| x as f32).collect();
      roundtrip_grid(Dtype::U8, &u8v);
      roundtrip_grid(Dtype::I16, &[-32768.0, -1.0, 0.0, 1.0, 32767.0]);
      roundtrip_grid(Dtype::U16, &[0.0, 1.0, 65535.0]);
      roundtrip_grid(Dtype::I24, &[-8388608.0, -1.0, 0.0, 8388607.0]);
      roundtrip_grid(Dtype::U24, &[0.0, 1.0, 16777215.0]);
      roundtrip_grid(Dtype::I32, &[-16777216.0, 0.0, 16777216.0]);
      roundtrip_grid(Dtype::U32, &[0.0, 16777216.0]);
      roundtrip_grid(Dtype::I64, &[-16777216.0, 0.0, 16777216.0]);
      roundtrip_grid(Dtype::U64, &[0.0, 16777216.0]);
}

#[test]
fn packed_int_codecs_roundtrip() {
      roundtrip_grid(Dtype::I4x8, &[-8.0, -1.0, 0.0, 1.0, 2.0, 3.0, 6.0, 7.0]);
      roundtrip_grid(Dtype::U4x8, &[0.0, 1.0, 2.0, 3.0, 8.0, 12.0, 14.0, 15.0]);
      roundtrip_grid(Dtype::I8x4, &[-128.0, -1.0, 0.0, 127.0]);
      roundtrip_grid(Dtype::U8x4, &[0.0, 1.0, 128.0, 255.0]);
      roundtrip_grid(Dtype::I16x2, &[-32768.0, 32767.0]);
      roundtrip_grid(Dtype::U16x2, &[0.0, 65535.0]);
}

#[test]
fn subbyte_packing_is_little_endian_low_nibble_first() {
      assert_eq!(decode(Dtype::U4, &[0x21]), vec![1.0, 2.0]);
      assert_eq!(decode(Dtype::U4, &[0xF3]), vec![3.0, 15.0]);
      assert_eq!(decode(Dtype::I4, &[0xF0]), vec![0.0, -1.0]);
      assert_eq!(decode(Dtype::I4, &[0x8_7u8]), vec![7.0, -8.0]);
}

#[test]
fn typeless_widths_bitcast_and_integer_semantics() {
      let one = 1.0f32;
      assert_eq!(decode(Dtype::B32, &one.to_le_bytes()), vec![1.0]);
      assert_eq!(decode(Dtype::Dword, &one.to_le_bytes()), vec![1.0]);
      roundtrip_grid(Dtype::B64, &[1.0, -2.0]);
      roundtrip_grid(Dtype::B128, &[1.0, 2.0, 3.0, 4.0]);
      roundtrip_grid(Dtype::B512, &(0..16).map(|x| x as f32).collect::<Vec<_>>());
      roundtrip_grid(Dtype::Dwordx8, &(0..8).map(|x| x as f32 * 1.5).collect::<Vec<_>>());
      assert_eq!(decode(Dtype::B8, &[200]), vec![200.0]);
      roundtrip_grid(Dtype::B8, &[0.0, 200.0, 255.0]);
      roundtrip_grid(Dtype::B16, &[0.0, 65535.0]);
      roundtrip_grid(Dtype::B16x2, &[7.0, 65000.0]);
      roundtrip_grid(Dtype::B16x4, &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn typeless_widths_convert_between_each_other() {
      let src = f32_bytes(&[3.5, -7.0]);
      let b64 = convert(Dtype::F32, Dtype::B64, &src).expect("f32->b64");
      let back = convert(Dtype::B64, Dtype::F32, &b64).expect("b64->f32");
      assert_eq!(from_f32_bytes(&back), vec![3.5, -7.0]);
      let dw2 = convert(Dtype::B64, Dtype::Dwordx2, &b64).expect("b64->dwordx2");
      assert_eq!(dw2, b64);
}

fn enumerate_fp_grid(dt: Dtype, width: u32, lanes: usize, block_bytes: usize) -> Vec<f32> {
      let mut finite = Vec::new();
      for p in 0u32..(1u32 << width) {
            let mut acc = 0u64;
            for l in 0..lanes {
                  acc |= (p as u64) << (l as u32 * width);
            }
            let mut blk = vec![0u8; block_bytes];
            for (i, b) in blk.iter_mut().enumerate() {
                  *b = ((acc >> (8 * i)) & 0xFF) as u8;
            }
            for v in decode(dt, &blk) {
                  if v.is_finite() {
                        finite.push(v);
                  }
            }
      }
      return finite;
}

#[test]
fn minifloat_grids_roundtrip() {
      let cases = [
            (Dtype::F4e2m1, 4u32, 2usize, 1usize),
            (Dtype::F6e2m3, 6, 4, 3),
            (Dtype::F6e3m2, 6, 4, 3),
            (Dtype::F8e4m3, 8, 1, 1),
            (Dtype::F8e5m2, 8, 1, 1),
            (Dtype::F8e4m3fnuz, 8, 1, 1),
            (Dtype::F8e5m2fnuz, 8, 1, 1),
            (Dtype::BF8, 8, 1, 1),
      ];
      for (dt, w, lanes, bb) in cases {
            let mut grid = enumerate_fp_grid(dt, w, lanes, bb);
            while grid.len() % lanes != 0 {
                  grid.push(0.0);
            }
            roundtrip_grid(dt, &grid);
      }
}

#[test]
fn packed_minifloats_roundtrip() {
      roundtrip_grid(Dtype::F4x2e2m1, &[0.5, 6.0]);
      roundtrip_grid(Dtype::F4x4e2m1, &[0.0, 1.0, 2.0, 4.0]);
      roundtrip_grid(Dtype::F6x4e2m3, &[0.0, 1.0, 3.5, 7.5]);
      roundtrip_grid(Dtype::F6x2e3m2, &[0.0, 1.0, 14.0, 28.0]);
      roundtrip_grid(Dtype::F8x2e4m3, &[1.0, 448.0]);
      roundtrip_grid(Dtype::F8x4e5m2, &[1.0, 2.0, 4.0, 57344.0]);
      roundtrip_grid(Dtype::F8x4e4m3fnuz, &[1.0, 2.0, 4.0, 240.0]);
      roundtrip_grid(Dtype::F8x2e5m2fnuz, &[1.0, 57344.0]);
}

#[test]
fn wide_floats_roundtrip() {
      roundtrip_grid(Dtype::F64, &[1.0, -2.5, 1e30, 0.0, 3.1415927]);
      roundtrip_grid(Dtype::F64x2, &[1.0, 2.0]);
      roundtrip_grid(Dtype::F16x2, &[1.0, -2.0]);
      roundtrip_grid(Dtype::BF16x2, &[1.0, 2.0]);
      roundtrip_grid(Dtype::F32x2, &[1.5, -7.25]);
      roundtrip_grid(Dtype::Tf32, &[1.0, 2.0, 0.5]);
      roundtrip_grid(Dtype::Xf32, &[1.0, 2.0, 0.5]);
}

#[test]
fn known_answer_f16_bf16() {
      assert_eq!(decode(Dtype::F16, &0x3C00u16.to_le_bytes()), vec![1.0]);
      assert_eq!(decode(Dtype::F16, &0xC000u16.to_le_bytes()), vec![-2.0]);
      assert!(decode(Dtype::F16, &0x7C00u16.to_le_bytes())[0].is_infinite());
      assert_eq!(decode(Dtype::BF16, &0x3F80u16.to_le_bytes()), vec![1.0]);
      assert_eq!(decode(Dtype::BF16, &0x4000u16.to_le_bytes()), vec![2.0]);
}

#[test]
fn known_answer_f8_e4m3() {
      assert_eq!(decode(Dtype::F8e4m3, &[0x00]), vec![0.0]);
      assert_eq!(decode(Dtype::F8e4m3, &[0x38]), vec![1.0]);
      assert_eq!(decode(Dtype::F8e4m3, &[0x78]), vec![256.0]);
      assert_eq!(decode(Dtype::F8e4m3, &[0x7E]), vec![448.0]);
      assert!(decode(Dtype::F8e4m3, &[0x7F])[0].is_nan());
      assert!(!decode(Dtype::F8e4m3, &[0x7E])[0].is_infinite());
}

#[test]
fn known_answer_f8_e5m2() {
      assert_eq!(decode(Dtype::F8e5m2, &[0x00]), vec![0.0]);
      assert_eq!(decode(Dtype::F8e5m2, &[0x3C]), vec![1.0]);
      assert_eq!(decode(Dtype::F8e5m2, &[0x40]), vec![2.0]);
      assert!(decode(Dtype::F8e5m2, &[0x7C])[0].is_infinite());
      assert!(decode(Dtype::F8e5m2, &[0x7D])[0].is_nan());
}

#[test]
fn fnuz_nan_pattern() {
      assert!(decode(Dtype::F8e4m3fnuz, &[0x80])[0].is_nan());
      assert!(decode(Dtype::F8e5m2fnuz, &[0x80])[0].is_nan());
      assert_eq!(decode(Dtype::F8e4m3fnuz, &[0x00]), vec![0.0]);
}

#[test]
fn srgb_eotf_endpoints_and_midpoint() {
      let dt = Dtype::Img(ImgLayout::L8, ImgFmt::Srgb);
      assert_eq!(decode(dt, &[0])[0], 0.0);
      assert_eq!(decode(dt, &[255])[0], 1.0);
      let mid = decode(dt, &[188])[0];
      assert!((mid - 0.5028).abs() < 0.01, "srgb 188 -> {mid}");
}

#[test]
fn image_formats_decode_and_roundtrip() {
      let unorm = Dtype::Img(ImgLayout::L8_8_8_8, ImgFmt::Unorm);
      assert_eq!(decode(unorm, &[0, 255, 0, 255]), vec![0.0, 1.0, 0.0, 1.0]);
      roundtrip_grid(unorm, &[0.0, 1.0, 0.5019608, 0.2509804]);
      let snorm = Dtype::Img(ImgLayout::L8, ImgFmt::Snorm);
      assert_eq!(decode(snorm, &[127])[0], 1.0);
      roundtrip_grid(snorm, &[-1.0, 0.0, 1.0]);
      let uint = Dtype::Img(ImgLayout::L16_16, ImgFmt::Uint);
      roundtrip_grid(uint, &[0.0, 65535.0]);
      let fl = Dtype::Img(ImgLayout::L16_16_16_16, ImgFmt::Float);
      roundtrip_grid(fl, &[1.0, -2.0, 0.5, 4.0]);
      let packed = Dtype::Img(ImgLayout::L5_6_5, ImgFmt::Unorm);
      roundtrip_grid(packed, &[0.0, 0.0, 0.0]);
      assert_eq!(decode(packed, &[0xFF, 0xFF]), vec![1.0, 1.0, 1.0]);
      let f11 = Dtype::Img(ImgLayout::L10_11_11, ImgFmt::Float);
      roundtrip_grid(f11, &[1.0, 2.0, 0.5]);
}

#[test]
fn convert_across_families_preserves_value() {
      let src = f32_bytes(&[5.0, -3.0, 100.0, 0.0]);
      let i8 = convert(Dtype::F32, Dtype::I8, &src).expect("f32->i8");
      let i16 = convert(Dtype::I8, Dtype::I16, &i8).expect("i8->i16");
      let back = convert(Dtype::I16, Dtype::F32, &i16).expect("i16->f32");
      assert_eq!(from_f32_bytes(&back), vec![5.0, -3.0, 100.0, 0.0]);
}

#[test]
fn quant_encode_still_bails() {
      let bytes = vec![0u8; 18];
      assert!(convert(Dtype::F32, Dtype::Q4_0, &f32_bytes(&vec![0.0; 32])).is_err());
      let _ = bytes;
}

#[test]
fn real_gguf_f32_through_bf16_pivot() {
      let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("stories-f32.gguf");
      let g = Gguf::open(&path).expect("open stories-f32.gguf");
      let name = g
            .tensors
            .iter()
            .find(|(_, i)| i.ggml_type == 0 && i.nbytes >= 4096)
            .map(|(n, _)| n.clone())
            .expect("an f32 tensor in the fixture");
      let raw = g.read_raw(&name).expect("read raw tensor bytes");
      let orig = from_f32_bytes(&raw);
      let bf16 = convert(Dtype::F32, Dtype::BF16, &raw).expect("f32->bf16");
      let back = convert(Dtype::BF16, Dtype::F32, &bf16).expect("bf16->f32");
      let rt = from_f32_bytes(&back);
      assert_eq!(rt.len(), orig.len());
      for (a, b) in orig.iter().zip(rt.iter()) {
            let tol = a.abs() * (1.0 / 128.0) + 1e-6;
            assert!((a - b).abs() <= tol, "bf16 pivot drift: {a} vs {b}");
      }
}
