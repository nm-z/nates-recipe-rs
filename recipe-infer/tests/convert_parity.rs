//! The GPU flipper against the CPU codec surface, on real quantized weights.
//!
//! [`gpu_core::infer_ops::gpu_convert`] decodes every ggml block quant on
//! device so the raw file bytes never leave VRAM and the host does no element
//! math. That device decode has to agree with `dequant`'s reference-exact CPU
//! codecs BIT FOR BIT, not approximately: both compute in f32 and `scale = 1.0`
//! is exact, so any difference is a wrong index, a wrong shift, or a wrong
//! scale unpack, never rounding.
//!
//! One `#[test]` per quant, each on the smallest real tensor of that type in
//! the machine's `qwen3-0.6b-<quant>` gguf (from the committed `gguf.toml`) —
//! every block of that tensor, every element of every block.

use anyhow::{Context, Result, bail};
use gpu_core::infer_ops::gpu_convert;
use gpu_core::memory::{Dtype, GpuBuffer};
use recipe_infer::dequant::{convert, dequant_f32};
use recipe_infer::gguf::Gguf;
use std::path::PathBuf;

fn model_path(key: &str) -> Result<PathBuf> {
	let toml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gguf.toml");
	let text = std::fs::read_to_string(&toml).with_context(|| format!("read {}", toml.display()))?;
	for line in text.lines() {
		let trimmed = line.trim_start();
		let Some(rest) = trimmed.strip_prefix(key) else {
			continue;
		};
		let Some((_, value)) = rest.split_once('=') else {
			continue;
		};
		let path = value.trim().trim_matches('"');
		if !path.is_empty() {
			return Ok(PathBuf::from(path));
		}
	}
	bail!("key {key} not found in {}", toml.display());
}

fn parity(model_key: &str, ggml: u32, dt: Dtype) -> Result<()> {
	let path = model_path(model_key)?;
	let g = Gguf::open(&path).with_context(|| format!("open {}", path.display()))?;
	let Some((name, info)) = g
		.tensors
		.iter()
		.filter(|(_n, t)| return t.ggml_type == ggml)
		.min_by_key(|(_n, t)| return t.nbytes)
	else {
		bail!("{model_key}: no tensor of ggml type {ggml} ({dt:?})");
	};
	let elems: usize = info.dims.iter().product();
	let bytes = g.read_raw(name).with_context(|| format!("read {name}"))?;

	let cpu = dequant_f32(ggml, &bytes).with_context(|| format!("cpu decode {name}"))?;
	let src = GpuBuffer::alloc_bytes(bytes.len())
		.map_err(|e| return anyhow::anyhow!("alloc src: {e}"))?
		.as_dtype(dt);
	src.write_u8(&bytes).map_err(|e| return anyhow::anyhow!("upload: {e}"))?;
	let dst = GpuBuffer::alloc_ty(elems, Dtype::F32)
		.map_err(|e| return anyhow::anyhow!("alloc dst: {e}"))?;
	gpu_convert(&src, &dst, elems, 1.0).map_err(|e| return anyhow::anyhow!("convert: {e}"))?;
	let mut gpu = vec![0f64; elems];
	dst.download_host(&mut gpu).map_err(|e| return anyhow::anyhow!("download: {e}"))?;

	assert_eq!(cpu.len(), elems, "{dt:?}: cpu decode length");
	let mut bad = 0usize;
	let mut first = String::new();
	for (i, (&c, &gv)) in cpu.iter().zip(gpu.iter()).enumerate() {
		if f64::from(c) != gv {
			bad += 1;
			if first.is_empty() {
				first = format!("[{i}] cpu {c:e} gpu {gv:e}");
			}
		}
	}
	eprintln!(
		"{dt:?}: {name} {elems} elems ({} blocks) {}",
		elems / dt.block_elems(),
		if bad == 0 { "IDENTICAL".to_owned() } else { format!("{bad} DIFFER, first {first}") }
	);
	assert_eq!(bad, 0, "{dt:?}: {bad}/{elems} elements differ, first {first}");
	return Ok(());
}

#[test]
fn convert_q4_0_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q4_0", 2, Dtype::Q4_0);
}

#[test]
fn convert_q4_1_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q4_1", 3, Dtype::Q4_1);
}

#[test]
fn convert_q5_0_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q5_0", 6, Dtype::Q5_0);
}

#[test]
fn convert_q5_1_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q5_1", 7, Dtype::Q5_1);
}

#[test]
fn convert_q8_0_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q8_0", 8, Dtype::Q8_0);
}

#[test]
fn convert_q2_k_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q2_k", 10, Dtype::Q2K);
}

#[test]
fn convert_q3_k_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q3_k", 11, Dtype::Q3K);
}

#[test]
fn convert_q4_k_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q4_k", 12, Dtype::Q4K);
}

#[test]
fn convert_q5_k_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q5_k", 13, Dtype::Q5K);
}

#[test]
fn convert_q6_k_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-q6_k", 14, Dtype::Q6K);
}

#[test]
fn convert_f16_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-f16", 1, Dtype::F16);
}

#[test]
fn convert_bf16_matches_cpu_codec() -> Result<()> {
	return parity("qwen3-0.6b-bf16", 30, Dtype::BF16);
}

#[test]
fn requant_q8_0_matches_cpu_codec() -> Result<()> {
	let path = model_path("qwen3-0.6b-f16")?;
	let g = Gguf::open(&path).with_context(|| format!("open {}", path.display()))?;
	let Some((name, info)) = g
		.tensors
		.iter()
		.filter(|(_n, t)| return t.ggml_type == 1)
		.min_by_key(|(_n, t)| return t.nbytes)
	else {
		bail!("no f16 tensor");
	};
	let elems: usize = info.dims.iter().product();
	let vals = dequant_f32(1, &g.read_raw(name)?)?;

	let mut raw = Vec::with_capacity(elems * 4);
	for &v in &vals {
		raw.extend_from_slice(&v.to_le_bytes());
	}
	let cpu = convert(Dtype::F32, Dtype::Q8_0, &raw).context("cpu encode")?;

	let src = GpuBuffer::alloc_bytes(raw.len())
		.map_err(|e| return anyhow::anyhow!("alloc src: {e}"))?
		.as_dtype(Dtype::F32);
	src.write_u8(&raw).map_err(|e| return anyhow::anyhow!("upload: {e}"))?;
	let q = GpuBuffer::alloc_bytes(cpu.len())
		.map_err(|e| return anyhow::anyhow!("alloc q: {e}"))?
		.as_dtype(Dtype::Q8_0);
	gpu_convert(&src, &q, elems, 1.0).map_err(|e| return anyhow::anyhow!("requant: {e}"))?;
	let mut gpu = vec![0u8; cpu.len()];
	q.download_u8(&mut gpu).map_err(|e| return anyhow::anyhow!("download: {e}"))?;

	let bad = cpu.iter().zip(gpu.iter()).filter(|(c, g)| return c != g).count();
	let first = cpu
		.iter()
		.zip(gpu.iter())
		.position(|(c, g)| return c != g)
		.map_or_else(|| return "none".to_owned(), |i| return format!("[{i}] cpu {} gpu {}", cpu[i], gpu[i]));
	eprintln!(
		"Q8_0 requant: {name} {elems} elems ({} blocks) {}",
		elems / 32,
		if bad == 0 { "IDENTICAL".to_owned() } else { format!("{bad} bytes DIFFER, first {first}") }
	);
	assert_eq!(bad, 0, "Q8_0 requant: {bad}/{} bytes differ, first {first}", cpu.len());
	return Ok(());
}
