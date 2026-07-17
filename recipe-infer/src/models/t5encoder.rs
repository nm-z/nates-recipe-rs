//! `t5encoder`: composed via the dense/encoder driver (see llama.cpp/src/models/t5encoder.cpp).
use super::common::{Ffn, Spec, layer_spec};
use super::super::{Arena, Model};
use anyhow::Result;
use gpu_core::memory::GpuBuffer;

const SPEC: Spec = Spec::dense(Ffn::GeluSeq).encoder();

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
