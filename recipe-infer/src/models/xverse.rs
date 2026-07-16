//! `xverse`: graph verified 1:1 against llama.cpp/src/models/xverse.cpp
//! (xverse). RMSNorm, causal GQA + RoPE, SiLU SwiGLU, no bias.
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
	causal_silu(m, l, false, h_in, h_out, t, ar, attn_scale)
}
