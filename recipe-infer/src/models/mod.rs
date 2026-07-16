//! Per-architecture decode dispatch. An architecture is routed here ONLY when
//! its decode graph has been read and verified 1:1 against
//! `llama.cpp/src/models/<arch>.cpp` AND all of its tensors resolve through the
//! loader's `hf_name` mapping. Everything else is a hard error, never a silent
//! fallback: routing an unverified architecture through a mismatched layer would
//! produce wrong tokens under the appearance of support.
//!
//! Verified today: `llama`, `xverse` (identical graph), `qwen3` (adds per-head
//! Q/K RMSNorm). All are RMSNorm + RoPE + causal GQA + SiLU SwiGLU, no bias.
//! These are graph-verified; first-token logit parity against llama.cpp is the
//! next gate before any further architecture (see the gpt2 work in progress).

mod common;
mod llama;
mod qwen3;
mod xverse;

use super::{Arena, Model};
use anyhow::{Result, bail};
use gpu_core::memory::GpuBuffer;

/// The exact signature every per-architecture `layer` must export. The pins
/// below fail to compile if any verified module drifts from it.
type LayerFn = fn(&Model, usize, &GpuBuffer, &GpuBuffer, usize, &Arena, &GpuBuffer) -> Result<()>;
const _: LayerFn = llama::layer;
const _: LayerFn = xverse::layer;
const _: LayerFn = qwen3::layer;

/// GGUF architecture strings with a verified decode composition.
pub(super) const SUPPORTED: &[&str] = &["llama", "xverse", "qwen3"];

/// True if `arch` has a verified composition wired into [`dispatch`].
pub(super) fn supported(arch: &str) -> bool {
	SUPPORTED.contains(&arch)
}

/// Route one decode layer to the verified composition for `m.hp.arch`, or fail
/// hard for any architecture without one.
pub(super) fn dispatch(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	match m.hp.arch.as_str() {
		"llama" => llama::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"xverse" => xverse::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen3" => qwen3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		other => bail!(
			"unsupported architecture {other:?}: recipe-infer has a verified decode \
			 composition only for {SUPPORTED:?}. Other architectures need their graph \
			 verified against llama.cpp and their tensors mapped in hf_name before \
			 they can be dispatched."
		),
	}
}
