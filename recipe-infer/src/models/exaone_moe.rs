//! `exaone-moe`: top-k SiLU SwiGLU mixture-of-experts (llama.cpp/src/models/exaone-moe.cpp).
use super::common::{Ffn, Spec, layer_moe};
use super::super::{Arena, Model};
use anyhow::Result;
use gpu_core::memory::GpuBuffer;

const SPEC: Spec = Spec::dense(Ffn::SiluGate).qk();

pub(super) fn layer(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	layer_moe(m, l, &SPEC, h_in, h_out, t, ar, attn_scale)
}
