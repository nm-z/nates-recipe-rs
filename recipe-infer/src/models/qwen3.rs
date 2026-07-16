//! `qwen3`: graph verified 1:1 against llama.cpp/src/models/qwen3.cpp
//! (qwen3 (adds per-head Q/K RMSNorm)). RMSNorm, causal GQA + RoPE, SiLU SwiGLU, no bias.
use super::common::causal_silu;
use super::super::{Arena, Model};
use anyhow::Result;
use gpu_core::memory::GpuBuffer;

pub(super) fn layer(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	causal_silu(m, l, true, h_in, h_out, t, ar, attn_scale)
}
