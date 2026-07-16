//! `modern-bert`: bidirectional encoder (non-causal attention, no RoPE).
use super::common::{Ffn, Spec, layer_spec};
use super::super::{Arena, Model};
use anyhow::Result;
use gpu_core::memory::GpuBuffer;

const SPEC: Spec = Spec::dense(Ffn::GeluGate).encoder();

pub(super) fn layer(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	layer_spec(m, l, &SPEC, h_in, h_out, t, ar, attn_scale)
}
