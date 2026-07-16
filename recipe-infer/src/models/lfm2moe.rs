//! `lfm2moe`: recurrent linear-attention / SSM block composed via the diagonal
//! linear-recurrence scan (see llama.cpp/src/models/lfm2moe.cpp).
use super::common::layer_recurrent;
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
	layer_recurrent(m, l, h_in, h_out, t, ar, attn_scale)
}
