//! `deepseek2-ocr`: composed via the encoder driver (see llama.cpp/src/models/deepseek2ocr.cpp).
use super::common::{Ffn, Spec, layer_spec};
use super::super::{Arena, Model};
use anyhow::Result;
use gpu_core::memory::GpuBuffer;

const SPEC: Spec = Spec::dense(Ffn::SiluGate).encoder();

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
