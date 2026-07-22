#![allow(unsafe_code)]
pub mod chat;
pub mod dequant;
pub mod gguf;
pub mod llm;
pub mod params;
pub mod safetensors;
pub mod scratch;
pub mod tokenizer;

use gpu_core::kernels;
use recipe_ir::{Activation, ConcatDims, LayerKind};
use std::cmp::Ordering;

pub use gpu_core::hip::device_synchronize;
pub use gpu_core::memory::{
	ExitD2H, GpuBuffer, Stage, adopt_run_backing_with_image, claim_device_arena_bytes,
	claim_device_arena_bytes_with_image, claim_device_arena_with_image, claimable_bytes,
	device_arena_active, exit_d2h_enqueue, exit_d2h_enqueue_buf, park_run_backing,
	release_device_arena,
};
pub use gpu_core::tiered;
pub use params::*;
pub use scratch::*;

pub fn init() -> Result<(), gpu_core::HipError> {
	gpu_core::hip::set_device(0)?;
	gpu_core::hip::retain_mempool(0)
}

pub fn shutdown() {
	gpu_core::memory::free_pinned_slots();
	gpu_core::kernels::gpu_shutdown();
}

pub fn forward_into(
	params: &[LayerParams],
	x: &GpuBuffer,
	x_cat: Option<&GpuBuffer>,
	n: usize,
	acts: &[GpuBuffer],
	sc: &Scratch,
) -> anyhow::Result<()> {
	let cc = concat_layer(params);
	sc.mark_fwd(0);
	for l in 0..params.len() {
		let p = &params[l];
		for ConcatDims { a, c, .. } in cc.filter(|cd| cd.pf == l).into_iter() {
			kernels::gpu_concat_into(
				&acts[l - 1],
				x_cat.ok_or_else(|| anyhow::anyhow!("concat: x_cat missing"))?,
				n,
				a,
				c,
				&sc.concat,
			)?;
		}
		let prev = match l.checked_sub(1) {
			None => x,
			Some(lm1) => match cc.filter(|cd| cd.pf == l) {
				Some(_cd) => &sc.concat,
				None => &acts[lm1],
			},
		};
		match p.kind {
			LayerKind::Embed => {
				kernels::gpu_gather_rows_into(&p.w, prev, n * p.in_dim, p.dim, &acts[l])?;
				kernels::gpu_broadcast_sub_into(
					&acts[l],
					&p.b,
					n * p.out_dim,
					p.out_dim,
					&acts[l],
				)?;
			}
			LayerKind::Attn => match Some(()).filter(|_u| sc.infer) {
				Some(()) => attn_forward_cached(p, prev, &acts[l], n, sc)?,
				None => attn_forward(p, prev, &acts[l], n, sc)?,
			},
			LayerKind::Conv => {
				let cin = p.conv_cin;
				let k = p.conv_k;
				let stride = p.conv_stride;
				let lin = p.in_dim / cin;
				let cout = p.out_dim / ((lin - k) / stride + 1);
				kernels::gpu_conv1d_into(
					prev, &p.w, &p.b, n, cin, lin, cout, k, stride, &acts[l],
				)?;
				let m = n * p.out_dim;
				match Some(()).filter(|_u| {
					matches!(
						p.act,
						Activation::Silu
							| Activation::Gelu | Activation::Elu
							| Activation::Selu | Activation::PRelu
					)
				}) {
					Some(()) => kernels::gpu_copy_into(&acts[l], m, &sc.preact[l]),
					None => Ok(()),
				}?;
				match p.act {
					Activation::Relu => kernels::gpu_relu_into(&acts[l], m, &acts[l]),
					Activation::Sigmoid => {
						kernels::gpu_sigmoid_into(&acts[l], m, &acts[l])
					}
					Activation::LeakyRelu => kernels::gpu_leaky_relu_into(
						&acts[l],
						&sc.c_leaky_alpha,
						m,
						&acts[l],
					),
					Activation::PRelu => kernels::gpu_leaky_relu_into(
						&sc.preact[l],
						&p.palpha,
						m,
						&acts[l],
					),
					Activation::Elu => gpu_core::k_gapact::gpu_elu(
						&sc.preact[l],
						&sc.c_elu_alpha,
						m,
						&acts[l],
					),
					Activation::Selu => gpu_core::k_gapact::gpu_selu(
						&sc.preact[l],
						&sc.c_selu_alpha,
						&sc.c_selu_lambda,
						m,
						&acts[l],
					),
					Activation::Tanh => kernels::gpu_tanh_into(&acts[l], m, &acts[l]),
					Activation::Silu => {
						kernels::gpu_silu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Gelu => {
						kernels::gpu_gelu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Linear => Ok(()),
				}?;
			}
			LayerKind::Dense => {
				match p.out_dim.cmp(&1) {
					Ordering::Equal => kernels::gpu_matvec_bias_into(
						prev, &p.w, &p.b, n, p.in_dim, &acts[l],
					)?,
					Ordering::Less | Ordering::Greater => kernels::gpu_linear_into(
						prev, &p.w, &p.b, n, p.out_dim, p.in_dim, &acts[l],
					)?,
				}
				let m = n * p.out_dim;
				match Some(()).filter(|_u| {
					matches!(
						p.act,
						Activation::Silu
							| Activation::Gelu | Activation::Elu
							| Activation::Selu | Activation::PRelu
					)
				}) {
					Some(()) => kernels::gpu_copy_into(&acts[l], m, &sc.preact[l]),
					None => Ok(()),
				}?;
				match p.act {
					Activation::Relu => kernels::gpu_relu_into(&acts[l], m, &acts[l]),
					Activation::Sigmoid => {
						kernels::gpu_sigmoid_into(&acts[l], m, &acts[l])
					}
					Activation::LeakyRelu => kernels::gpu_leaky_relu_into(
						&acts[l],
						&sc.c_leaky_alpha,
						m,
						&acts[l],
					),
					Activation::PRelu => kernels::gpu_leaky_relu_into(
						&sc.preact[l],
						&p.palpha,
						m,
						&acts[l],
					),
					Activation::Elu => gpu_core::k_gapact::gpu_elu(
						&sc.preact[l],
						&sc.c_elu_alpha,
						m,
						&acts[l],
					),
					Activation::Selu => gpu_core::k_gapact::gpu_selu(
						&sc.preact[l],
						&sc.c_selu_alpha,
						&sc.c_selu_lambda,
						m,
						&acts[l],
					),
					Activation::Tanh => kernels::gpu_tanh_into(&acts[l], m, &acts[l]),
					Activation::Silu => {
						kernels::gpu_silu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Gelu => {
						kernels::gpu_gelu_into(&sc.preact[l], m, &acts[l])
					}
					Activation::Linear => Ok(()),
				}?;
			}
		}
		sc.mark_fwd(l + 1);
	}
	Ok(())
}

pub fn attn_forward(
	p: &LayerParams,
	h: &GpuBuffer,
	out: &GpuBuffer,
	n: usize,
	sc: &Scratch,
) -> anyhow::Result<()> {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q)?;
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k)?;
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v)?;
	gpu_core::rope::gpu_rope_qk_heads_inplace(
		&sc.c_one,
		&sc.c_rope_theta,
		m,
		d,
		heads,
		s,
		&sc.a_q,
		&sc.a_k,
	)?;
	kernels::gpu_flash_attention_train_into(
		&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx, &sc.a_lse,
	)?;
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out)?;
	Ok(())
}

pub fn attn_forward_cached(
	p: &LayerParams,
	h: &GpuBuffer,
	out: &GpuBuffer,
	n: usize,
	sc: &Scratch,
) -> anyhow::Result<()> {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_into(h, &p.w, &p.b, m, d, d, &sc.a_q)?;
	kernels::gpu_linear_into(h, &p.wk, &p.b, m, d, d, &sc.a_k)?;
	kernels::gpu_linear_into(h, &p.wv, &p.b, m, d, d, &sc.a_v)?;
	gpu_core::rope::gpu_rope_qk_heads_inplace(
		&sc.c_one,
		&sc.c_rope_theta,
		m,
		d,
		heads,
		s,
		&sc.a_q,
		&sc.a_k,
	)?;
	kernels::gpu_flash_attention_into(&sc.a_q, &sc.a_k, &sc.a_v, n, s, d, heads, &sc.a_ctx)?;
	kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, m, d, d, out)?;
	Ok(())
}

struct ByteUnit {
	floor: f64,
	div: f64,
	prec: usize,
	label: &'static str,
}

pub fn human_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	let units = [
		ByteUnit {
			floor: K * K * K,
			div: K * K * K,
			prec: 2,
			label: "GB",
		},
		ByteUnit {
			floor: K * K,
			div: K * K,
			prec: 1,
			label: "MB",
		},
		ByteUnit {
			floor: 0.0,
			div: K,
			prec: 1,
			label: "KB",
		},
	];
	let pick = units.iter().find(|u| f >= u.floor).unwrap_or(&units[2]);
	format!("{:.prec$} {}", f / pick.div, pick.label, prec = pick.prec)
}

#[ctor::ctor]
fn probe_child_answer() {
	if std::env::var_os("VRAM_PROBE").is_some() || std::env::var_os("RAM_PROBE").is_some() {
		if let Some(code) = llm::vram_probe_ask() {
			std::process::exit(code);
		}
		if let Some(code) = gpu_core::memory::ram_probe_ask() {
			std::process::exit(code);
		}
	}
}
