use crate::gguf::{Gguf, Val};
use anyhow::{Context, Result, anyhow, bail};
use gpu_core::infer_ops::{
	gpu_convert, gpu_embed_blend, gpu_gelu_mul, gpu_gemm_bt, gpu_rmsnorm_f64, gpu_rmsnorm_f64_nogamma,
};
use gpu_core::kernels::{gpu_add_into, gpu_copy_into, gpu_layernorm_into};
use gpu_core::memory::{Dtype, GpuBuffer};
use gpu_core::waterfall::{Home, Waterfall};
use ogdl::log::probe as probe_flag;
use ogdl::log::{Write, data, gpu};
use rand::rngs::StdRng;
use rand::{Rng as _, SeedableRng as _};
use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::mem;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[path = "models/mod.rs"]
mod models;

pub const FWD_DT: Dtype = Dtype::F32;

const fn bytes_for(elems: usize, dt: Dtype) -> usize {
	return elems.div_ceil(dt.block_elems()) * dt.block_bytes();
}

static KV_SPILL_SEQ: AtomicU64 = AtomicU64::new(0);

fn falloc(n: usize) -> Result<GpuBuffer, gpu_core::HipError> {
	return GpuBuffer::alloc_ty(n, FWD_DT);
}

fn embed_blend_into(
	m: &Model,
	ar: &Arena,
	picks: &[(usize, f64)],
	rows: usize,
	k: usize,
	scale: f64,
	out: &GpuBuffer,
	scratch: &mut Vec<u8>,
	wts: &mut Vec<f64>,
) -> Result<()> {
	let ne = m.hp.ne;
	let (src, dt) = (&m.emb.bytes, m.emb.dt);
	let rb = bytes_for(ne, dt);
	scratch.clear();
	wts.clear();
	for &(id, w) in picks {
		let b = id * rb;
		scratch.extend_from_slice(&src[b..b + rb]);
		wts.push(w);
	}
	let stage = ar.blend_src.view(0, rows * k * ne).as_dtype(dt);
	stage.write_u8(scratch)?;
	let wdev = ar.blend_w.view(0, rows * k);
	wdev.load(wts)?;
	gpu_embed_blend(&stage, &wdev, out, rows, k, ne, scale)
		.map_err(|e| return anyhow!("embed_blend launch: {e:?}"))?;
	return Ok(());
}

pub fn supported_archs() -> &'static [&'static str] {
	models::VERIFIED
}

pub fn composable_archs() -> &'static [&'static str] {
	models::COMPOSABLE
}

pub fn arch_composable(arch: &str) -> bool {
	models::supported(arch)
}

pub fn arch_supported(arch: &str) -> bool {
	models::verified(arch)
}

struct Headless {
	m: Model,
	ar: Arena,
	attn_scale: GpuBuffer,
}

impl Headless {
	fn open(gguf: &Path, cap: usize) -> Result<Self> {
		crate::init().map_err(|e| anyhow!("gpu init: {e:?}"))?;
		let want = (2usize << 30) & !((1 << 21) - 1);
		let slab = gpu_core::memory::claim_device_arena_bytes(want).context("claim device arena")?;
		let mut claim = Waterfall::from_arena(slab);
		let prep = (|| -> Result<(Model, Arena, GpuBuffer)> {
			let m = load_model_gguf(gguf)?;
			anyhow::ensure!(
				models::supported(&m.hp.arch),
				"headless: unsupported architecture {:?}",
				m.hp.arch
			);
			let ar = Arena::new(&m.hp, cap)?;
			let attn_scale = {
				let hd = m.hp.dims.first().map_or(m.hp.key_length, |d| d.hd);
				let ub = falloc(1)?;
				ub.load(&[1.0 / (hd as f64).sqrt()])?;
				ub
			};
			return Ok((m, ar, attn_scale));
		})();
		let (m, ar, attn_scale) = match prep {
			Ok(v) => v,
			Err(e) => {
				if let Some(slab) = claim.take_slab() {
					gpu_core::memory::release_device_arena(slab);
				}
				return Err(e);
			}
		};
		let mut h = Self { m, ar, attn_scale };
		fill_store(&mut h.m, claim, &mut || false)?;
		return Ok(h);
	}
}

impl Drop for Headless {
	fn drop(&mut self) {
		if let Some(slab) = self.m.store.take_slab() {
			gpu_core::memory::release_device_arena(slab);
		}
	}
}

pub fn greedy(gguf: &Path, toks: &[u32], n_new: usize) -> Result<Vec<u32>> {
	return greedy_windowed(gguf, toks, n_new, toks.len() + n_new);
}

pub fn greedy_windowed(gguf: &Path, toks: &[u32], n_new: usize, win: usize) -> Result<Vec<u32>> {
	anyhow::ensure!(!toks.is_empty(), "greedy: empty token sequence");
	anyhow::ensure!(win >= 1, "greedy: zero-row VRAM window");
	let total = toks.len() + n_new;
	let win = win.min(total);
	let h = Headless::open(gguf, toks.len().min(win))?;
	let softcap = models::final_softcap(&h.m);
	let lsc = models::logit_scale(&h.m);
	let vocab = h.m.hp.vocab;
	let mut cache = KvCache::new(&h.m, win, total - win)?;
	let mut logits = vec![0.0f64; vocab];
	let mut lm_scratch = vec![0.0f64; h.m.hp.lm_chunk];
	let mut emb_scratch: Vec<u8> = Vec::new();
	for c in toks.chunks(win) {
		forward_rows(
			&h.m,
			c,
			&h.attn_scale,
			&h.ar,
			&mut cache,
			&mut logits,
			&mut lm_scratch,
			&mut emb_scratch,
		)?;
	}
	let mut generated = Vec::with_capacity(n_new);
	for _ in 0..n_new {
		let next = pick_greedy(&logits, vocab, lsc, softcap);
		generated.push(next);
		if generated.len() >= n_new {
			break;
		}
		forward_rows(
			&h.m,
			&[next],
			&h.attn_scale,
			&h.ar,
			&mut cache,
			&mut logits,
			&mut lm_scratch,
			&mut emb_scratch,
		)?;
	}
	return Ok(generated);
}

pub fn last_logits(gguf: &Path, toks: &[u32]) -> Result<Vec<f64>> {
	anyhow::ensure!(!toks.is_empty(), "last_logits: empty token sequence");
	let h = Headless::open(gguf, toks.len())?;
	let mut cache = KvCache::new(&h.m, toks.len(), 0)?;
	let mut logits = vec![0.0f64; h.m.hp.vocab];
	let mut lm_scratch = vec![0.0f64; h.m.hp.lm_chunk];
	let mut emb_scratch: Vec<u8> = Vec::new();
	forward_rows(
		&h.m,
		toks,
		&h.attn_scale,
		&h.ar,
		&mut cache,
		&mut logits,
		&mut lm_scratch,
		&mut emb_scratch,
	)?;
	let ls = models::logit_scale(&h.m);
	if ls != 1.0 {
		for v in logits.iter_mut() {
			*v *= ls;
		}
	}
	let fc = models::final_softcap(&h.m);
	if fc > 0.0 {
		for v in logits.iter_mut() {
			*v = fc * (*v / fc).tanh();
		}
	}
	return Ok(logits);
}

#[derive(Clone)]
pub struct Tok {
	pub text: String,
	pub status: TokStatus,
	pub age: u8,
	pub heat: f32,
}

#[derive(Clone, Copy)]
pub enum TokStatus {
	Draft,
	Accepted,
	Recent,
}

pub fn render_toks(toks: &[Tok]) -> String {
	let mut out = String::new();
	for t in toks {
		out.push_str(&t.text);
	}
	out
}

pub fn toks_line(toks: &[Tok]) -> String {
	let mut out = String::new();
	for t in toks {
		let Some(_keep) = Some(()).filter(|_probe| !matches!(t.status, TokStatus::Draft)) else {
			continue;
		};
		out.push_str(&t.text);
	}
	out
}

fn as_uint(v: &Val) -> Option<u64> {
	Some(match v {
		Val::U8(x) => *x as u64,
		Val::U16(x) => *x as u64,
		Val::U32(x) => *x as u64,
		Val::U64(x) => *x,
		Val::I8(x) if *x >= 0 => *x as u64,
		Val::I16(x) if *x >= 0 => *x as u64,
		Val::I32(x) if *x >= 0 => *x as u64,
		Val::I64(x) if *x >= 0 => *x as u64,
		_other => return None,
	})
}

fn uint_kv(g: &Gguf, key: &str) -> Result<usize> {
	let v =
		g.kv.get(key)
			.ok_or_else(|| anyhow!("gguf: kv {key} not found"))?;
	as_uint(v)
		.map(|x| x as usize)
		.ok_or_else(|| anyhow!("gguf: kv {key} is not an unsigned integer"))
}

fn uint_kv_or(g: &Gguf, key: &str, default: usize) -> Result<usize> {
	match g.kv.get(key) {
		None => Ok(default),
		Some(v) => as_uint(v)
			.map(|x| x as usize)
			.ok_or_else(|| anyhow!("gguf: kv {key} is not an unsigned integer")),
	}
}

fn f32_kv_or(g: &Gguf, key: &str, default: f64) -> Result<f64> {
	match g.kv.get(key) {
		None => Ok(default),
		Some(Val::F32(v)) => Ok(f64::from(*v)),
		Some(_other) => bail!("gguf: kv {key} is not f32"),
	}
}

fn uint_arr(g: &Gguf, key: &str) -> Result<Vec<usize>> {
	match g.kv.get(key) {
		Some(Val::Arr(items)) => items
			.iter()
			.map(|v| {
				as_uint(v)
					.map(|x| x as usize)
					.ok_or_else(|| anyhow!("gguf: kv {key} array holds a non-uint element"))
			})
			.collect(),
		Some(_other) => bail!("gguf: kv {key} is not an array"),
		None => bail!("gguf: kv {key} not found"),
	}
}

fn uint_or_arr(g: &Gguf, key: &str, nl: usize, default: usize) -> Result<Vec<usize>> {
	match g.kv.get(key) {
		None => Ok(vec![default; nl]),
		Some(Val::Arr(_items)) => {
			let v = uint_arr(g, key)?;
			anyhow::ensure!(
				v.len() == nl,
				"gguf: kv {key} array len {} != block_count {nl}",
				v.len()
			);
			Ok(v)
		}
		Some(_scalar) => Ok(vec![uint_kv(g, key)?; nl]),
	}
}

fn bool_arr(g: &Gguf, key: &str) -> Result<Vec<bool>> {
	match g.kv.get(key) {
		Some(Val::Arr(items)) => items
			.iter()
			.map(|v| match v {
				Val::Bool(b) => Ok(*b),
				_other => bail!("gguf: kv {key} array holds a non-bool element"),
			})
			.collect(),
		Some(_other) => bail!("gguf: kv {key} is not an array"),
		None => bail!("gguf: kv {key} not found"),
	}
}

fn str_kv(g: &Gguf, key: &str) -> Result<String> {
	match g.kv.get(key) {
		Some(Val::Str(s)) => Ok(s.clone()),
		Some(_other) => bail!("gguf: kv {key} is not a string"),
		None => bail!("gguf: kv {key} not found"),
	}
}

struct LayerDims {
	hd: usize,
	nqh: usize,
	nff: usize,
	nkv: usize,
	sliding: bool,
}

struct Hparams {
	arch: String,
	nl: usize,
	ne: usize,
	nqh: usize,
	nff: usize,
	nffe: usize,
	nexp: usize,
	used: usize,
	vocab: usize,
	eps: f64,
	ncanvas: usize,
	mask: usize,
	bos: u32,
	eos: u32,
	eog: Vec<u32>,
	mask_signal: Option<usize>,
	key_length: usize,
	shortconv_l_cache: usize,
	q_lora_rank: usize,
	kv_lora_rank: usize,
	head_k_mla: usize,
	head_v_mla: usize,
	n_rot: usize,
	freq_base: f64,
	freq_base_swa: f64,
	softcap: f64,
	logit_scale: f64,
	embedding_scale: f64,
	residual_scale: f64,
	alibi_bias: f64,
	ssm_d_conv: usize,
	ssm_d_inner: usize,
	ssm_d_state: usize,
	ssm_dt_rank: usize,
	ssm_n_group: usize,
	ssm_dt_dim: usize,
	kda_head_dim: usize,
	kda_n_head: usize,
	delta_win: usize,
	dims: Vec<LayerDims>,
	win_elems: usize,
	stage_bytes: usize,
	qd_max: usize,
	kd_max: usize,
	lm_chunk: usize,
	gu_bytes: usize,
	dn_bytes: usize,
	slot_bytes: usize,
	moe_gu_dt: Dtype,
	moe_dn_dt: Dtype,
}

impl Hparams {
	fn is_mla(&self) -> bool {
		return self.head_k_mla > 0 && self.head_v_mla > 0;
	}

	fn from_gguf(g: &Gguf) -> Result<Hparams> {
		let arch = str_kv(g, "general.architecture")?;
		let k = |s: &str| format!("{arch}.{s}");
		let nl = uint_kv(g, &k("block_count"))?;
		let ne = uint_kv(g, &k("embedding_length"))?;
		let n_ff_arr = uint_or_arr(g, &k("feed_forward_length"), nl, 0)?;
		let nff = n_ff_arr.iter().copied().max().unwrap_or(0);
		let mut nffe = uint_kv_or(g, &k("expert_feed_forward_length"), 0)?;
		let nexp = uint_kv_or(g, &k("expert_count"), 0)?;
		if nexp > 0 && nffe == 0 {
			nffe = expert_ff_from_tensors(g, nl);
		}
		let used = uint_kv_or(g, &k("expert_used_count"), 0)?;
		let n_head_arr = uint_or_arr(g, &k("attention.head_count"), nl, 0)?;
		let nqh = n_head_arr.iter().copied().max().unwrap_or(0);
		let head_dim_default = if nqh > 0 { ne / nqh } else { 0 };
		let key_length = uint_kv_or(g, &k("attention.key_length"), head_dim_default)?;
		let value_length = uint_kv_or(g, &k("attention.value_length"), head_dim_default)?;
		let key_length_swa = uint_kv_or(g, &k("attention.key_length_swa"), key_length)?;
		let q_lora_rank = uint_kv_or(g, &k("attention.q_lora_rank"), 0)?;
		let kv_lora_rank = uint_kv_or(g, &k("attention.kv_lora_rank"), 0)?;
		let (head_k_mla, head_v_mla) = if arch == "minicpm3" {
			(key_length, value_length)
		} else {
			(
				uint_kv_or(g, &k("attention.key_length_mla"), 0)?,
				uint_kv_or(g, &k("attention.value_length_mla"), 0)?,
			)
		};
		let n_rot = uint_kv_or(g, &k("rope.dimension_count"), 0)?;
		let head_count_kv = match g.kv.get(&k("attention.head_count_kv")) {
			Some(Val::Arr(_items)) => uint_arr(g, &k("attention.head_count_kv"))?,
			Some(_scalar) => vec![uint_kv(g, &k("attention.head_count_kv"))?; nl],
			None => vec![nqh; nl],
		};
		let swa_window = uint_kv_or(g, &k("attention.sliding_window"), 0)?;
		let pattern: Vec<bool> = match g.kv.get(&k("attention.sliding_window_pattern")) {
			Some(Val::Arr(items)) if items.iter().all(|v| matches!(v, Val::Bool(_))) => {
				bool_arr(g, &k("attention.sliding_window_pattern"))?
			}
			other if swa_window > 0 => {
				let period = other
					.and_then(as_uint)
					.map(|x| x as usize)
					.filter(|&p| p > 0)
					.unwrap_or(6);
				(0..nl).map(|l| l % period != period - 1).collect()
			}
			_other => vec![false; nl],
		};
		let freq_base = f32_kv_or(g, &k("rope.freq_base"), 10000.0)?;
		let freq_base_swa = f32_kv_or(g, &k("rope.freq_base_swa"), freq_base)?;
		let eps = if models::norm_is_layer(&arch) {
			f32_kv_or(g, &k("attention.layer_norm_epsilon"), 1e-5)?
		} else {
			f32_kv_or(g, &k("attention.layer_norm_rms_epsilon"), 1e-5)?
		};
		let softcap = f32_kv_or(g, &k("final_logit_softcapping"), 0.0)?;
		let logit_scale = f32_kv_or(g, &k("logit_scale"), 1.0)?;
		let embedding_scale = f32_kv_or(g, &k("embedding_scale"), 0.0)?;
		let residual_scale = f32_kv_or(g, &k("residual_scale"), 0.0)?;
		let alibi_bias = f32_kv_or(g, &k("attention.max_alibi_bias"), 0.0)?;
		let shortconv_l_cache = uint_kv_or(g, &k("shortconv.l_cache"), 0)?;
		let ssm_d_conv = uint_kv_or(g, &k("ssm.conv_kernel"), 0)?;
		let ssm_d_inner = uint_kv_or(g, &k("ssm.inner_size"), 0)?;
		let ssm_d_state = uint_kv_or(g, &k("ssm.state_size"), 0)?;
		let ssm_dt_rank = uint_kv_or(g, &k("ssm.time_step_rank"), 0)?;
		let ssm_n_group_raw = uint_kv_or(g, &k("ssm.group_count"), 0)?;
		let ssm_n_group = if ssm_n_group_raw == 0 {
			1
		} else {
			ssm_n_group_raw
		};
		let ssm_dt_dim = if arch == "plamo2" {
			(ne / 16).max(64)
		} else {
			0
		};
		let kda_head_dim = uint_kv_or(g, &k("kda.head_dim"), 0)?;
		let is_recr: Vec<bool> = match g.kv.get(&k("attention.recurrent_layers")) {
			Some(Val::Arr(_items)) => uint_arr(g, &k("attention.recurrent_layers"))?
				.iter()
				.map(|&x| x != 0)
				.collect(),
			_other => head_count_kv.iter().map(|&kv| kv == 0).collect(),
		};
		let kda_n_head = if kda_head_dim > 0 {
			is_recr
				.iter()
				.position(|&r| r)
				.and_then(|l| g.tensors.get(&format!("blk.{l}.ssm_dt.bias")))
				.map(|tt| tt.dims.iter().product::<usize>() / kda_head_dim)
				.unwrap_or(0)
		} else {
			0
		};
		let ncanvas = uint_kv_or(g, "diffusion.canvas_length", 0)?;
		let bos = uint_kv(g, "tokenizer.ggml.bos_token_id")? as u32;
		let eos = uint_kv(g, "tokenizer.ggml.eos_token_id")? as u32;
		let mask = uint_kv_or(g, "tokenizer.ggml.mask_token_id", 0)?;
		let eog = {
			let mut set = vec![eos];
			for id_key in ["tokenizer.ggml.eot_token_id", "tokenizer.ggml.eom_token_id"] {
				if let Ok(id) = uint_kv(g, id_key) {
					set.push(id as u32);
				}
			}
			set.sort_unstable();
			set.dedup();
			set
		};

		let et = g
			.tensors
			.get("token_embd.weight")
			.ok_or_else(|| anyhow!("gguf: no token_embd.weight tensor"))?;
		let mut eshape = et.dims.clone();
		eshape.reverse();
		anyhow::ensure!(
			eshape.len() == 2 && eshape[1] == ne,
			"token_embd.weight shape {eshape:?} (ne={ne})"
		);
		let vocab = eshape[0];

		let tokens = g.str_arr("tokenizer.ggml.tokens")?;
		let mask_signal = tokens.iter().position(|s| s == "\u{63a9}");

		anyhow::ensure!(
			head_count_kv.len() == nl,
			"head_count_kv len {} != block_count {nl}",
			head_count_kv.len()
		);
		anyhow::ensure!(
			pattern.len() >= nl,
			"sliding_window_pattern len {} < block_count {nl}",
			pattern.len()
		);

		let mut dims = Vec::with_capacity(nl);
		for l in 0..nl {
			let sliding = pattern[l];
			let hd = if sliding { key_length_swa } else { key_length };
			dims.push(LayerDims {
				hd,
				nqh: n_head_arr[l],
				nff: n_ff_arr[l],
				nkv: head_count_kv[l],
				sliding,
			});
		}

		let kd_max = dims.iter().map(|d| d.nkv * d.hd).max().unwrap_or(0);
		let qd_max = dims
			.iter()
			.map(|d| d.nqh * d.hd)
			.max()
			.unwrap_or(nqh * key_length);
		let moe_dt = |suffix: &str| -> Result<Option<Dtype>> {
			for l in 0..nl {
				if let Some(ti) = g.tensors.get(&format!("blk.{l}.{suffix}")) {
					return Ok(Some(crate::dequant::from_ggml(ti.ggml_type)?));
				}
			}
			return Ok(None);
		};
		let moe_gu_dt = match moe_dt("ffn_gate_up_exps.weight")?.or(moe_dt("ffn_gate_exps.weight")?) {
			Some(dt) => dt,
			None if nexp == 0 => FWD_DT,
			None => bail!("gguf: {nexp} experts but no ffn_gate_up_exps/ffn_gate_exps weight in any layer"),
		};
		let moe_dn_dt = match moe_dt("ffn_down_exps.weight")? {
			Some(dt) => dt,
			None if nexp == 0 => FWD_DT,
			None => bail!("gguf: {nexp} experts but no ffn_down_exps weight in any layer"),
		};
		if nexp > 0 {
			anyhow::ensure!(
				(2 * nffe * ne).is_multiple_of(moe_gu_dt.block_elems())
					&& (nffe * ne).is_multiple_of(moe_gu_dt.block_elems()),
				"expert gate/up {}x{ne} not on {moe_gu_dt:?} block boundary",
				2 * nffe
			);
			anyhow::ensure!(
				(nffe * ne).is_multiple_of(moe_dn_dt.block_elems()),
				"expert down {nffe}x{ne} not on {moe_dn_dt:?} block boundary"
			);
		}
		let gu_bytes = bytes_for(2 * nffe * ne, moe_gu_dt);
		let dn_bytes = bytes_for(nffe * ne, moe_dn_dt);
		let slot_bytes = gu_bytes + dn_bytes;
		let mla_win = if head_k_mla > 0 && head_v_mla > 0 {
			let nope = head_k_mla.saturating_sub(n_rot);
			[
				q_lora_rank * ne,
				nqh * head_k_mla * q_lora_rank,
				(kv_lora_rank + n_rot) * ne,
				kv_lora_rank * nope,
				head_v_mla * kv_lora_rank,
			]
			.into_iter()
			.max()
			.unwrap_or(0)
		} else {
			0
		};
		let ssm_win = if ssm_d_inner > 0 {
			[
				2 * ssm_d_inner * ne,
				(2 * ssm_d_inner + 2 * ssm_n_group * ssm_d_state + ssm_dt_rank) * ne,
				(ssm_dt_rank + 2 * ssm_d_state) * ssm_d_inner,
				(ssm_dt_dim + 2 * ssm_d_state) * ssm_d_inner,
				ne * ssm_d_inner,
				ssm_d_inner * ssm_dt_rank,
			]
			.into_iter()
			.max()
			.unwrap_or(0)
		} else {
			0
		};
		let delta_win = if models::is_delta_arch(&arch) {
			let d = ssm_d_state.max(kda_head_dim);
			let di = ssm_d_inner.max(kda_head_dim * kda_n_head);
			let conv_dim = 2 * ssm_n_group * ssm_d_state + di;
			[conv_dim, di, 2 * qd_max, d].into_iter().max().unwrap_or(0)
		} else {
			0
		};
		let shortconv_win = if shortconv_l_cache > 0 {
			3 * ne * ne
		} else {
			0
		};
		let win_elems = (qd_max.max(kd_max).max(nff).max(2 * nffe) * ne)
			.max(mla_win)
			.max(ssm_win)
			.max(shortconv_win)
			.max(2 * qd_max * ne);
		let lm_chunk = win_elems / ne;
		let mut stage_bytes = slot_bytes;
		for (name, info) in &g.tensors {
			if hf_name(name).is_some() {
				let dt = crate::dequant::from_ggml(info.ggml_type)?;
				stage_bytes = stage_bytes.max(bytes_for(win_elems, dt));
			}
		}

		Ok(Hparams {
			arch,
			nl,
			ne,
			nqh,
			nff,
			nffe,
			nexp,
			used,
			vocab,
			eps,
			ncanvas,
			mask,
			bos,
			eos,
			eog,
			win_elems,
			stage_bytes,
			mask_signal,
			key_length,
			shortconv_l_cache,
			q_lora_rank,
			kv_lora_rank,
			head_k_mla,
			head_v_mla,
			n_rot,
			freq_base,
			freq_base_swa,
			softcap,
			logit_scale,
			embedding_scale,
			residual_scale,
			alibi_bias,
			ssm_d_conv,
			ssm_d_inner,
			ssm_d_state,
			ssm_dt_rank,
			ssm_n_group,
			ssm_dt_dim,
			kda_head_dim,
			kda_n_head,
			delta_win,
			dims,
			qd_max,
			kd_max,
			lm_chunk,
			gu_bytes,
			dn_bytes,
			slot_bytes,
			moe_gu_dt,
			moe_dn_dt,
		})
	}
}

fn expert_ff_from_tensors(g: &Gguf, nl: usize) -> usize {
	for l in 0..nl {
		if let Some(ti) = g.tensors.get(&format!("blk.{l}.ffn_gate_exps.weight")) {
			return ti.dims.get(1).copied().unwrap_or(0);
		}
		if let Some(ti) = g.tensors.get(&format!("blk.{l}.ffn_gate_up_exps.weight")) {
			return ti.dims.get(1).map_or(0, |d| d / 2);
		}
	}
	0
}

static DISK_NS: AtomicU64 = AtomicU64::new(0);
static H2D_NS: AtomicU64 = AtomicU64::new(0);
static WIDEN_NS: AtomicU64 = AtomicU64::new(0);
static ATTN_NS: AtomicU64 = AtomicU64::new(0);
static MLP_NS: AtomicU64 = AtomicU64::new(0);
static MOE_NS: AtomicU64 = AtomicU64::new(0);
static MOE_RT_NS: AtomicU64 = AtomicU64::new(0);
static ROUTE_NS: AtomicU64 = AtomicU64::new(0);
static LM_NS: AtomicU64 = AtomicU64::new(0);

fn acc(a: &AtomicU64, t: Instant) {
	a.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
}

#[derive(Clone)]
struct Tensor {
	shard: usize,
	off: usize,
	nbytes: usize,
	shape: Vec<usize>,
	dt: Dtype,
}

impl Tensor {
	fn elems(&self) -> usize {
		return self.shape.iter().product();
	}
}

struct Raw {
	bytes: Vec<u8>,
	dt: Dtype,
}

fn read_raw_at(shards: &[File], t: &Tensor, off: usize, len: usize) -> Result<Vec<u8>> {
	let mut buf = vec![0u8; len];
	let _d = Instant::now();
	shards[t.shard]
		.read_exact_at(&mut buf, (t.off + off) as u64)
		.with_context(|| format!("raw read {len} bytes at shard {}", t.shard))?;
	acc(&DISK_NS, _d);
	return Ok(buf);
}

fn read_whole(shards: &[File], t: &Tensor) -> Result<Raw> {
	return Ok(Raw {
		bytes: read_raw_at(shards, t, 0, t.nbytes)?,
		dt: t.dt,
	});
}

fn bview(buf: &GpuBuffer, off_bytes: usize, len_bytes: usize) -> GpuBuffer {
	if !(off_bytes.is_multiple_of(8) && len_bytes.is_multiple_of(8)) {
		Write::error(format!("bview: unaligned {off_bytes}/{len_bytes}"));
	}
	buf.view(off_bytes / 8, len_bytes / 8)
}

static E_VRAM: AtomicU64 = AtomicU64::new(0);
static E_RAM: AtomicU64 = AtomicU64::new(0);
static E_DISK: AtomicU64 = AtomicU64::new(0);

static BEAT: AtomicU64 = AtomicU64::new(0);

fn beat() {
	BEAT.fetch_add(1, Ordering::Relaxed);
}

struct Watchdog {
	state: Arc<(Mutex<bool>, Condvar)>,
}

impl Watchdog {
	fn disarm(self) {
		let (lock, cv) = &*self.state;
		*lock.lock().unwrap_or_else(|p| p.into_inner()) = true;
		cv.notify_all();
	}
}

fn arm_watchdog() -> Watchdog {
	let state = Arc::new((Mutex::new(false), Condvar::new()));
	let shared = state.clone();
	thread::spawn(move || {
		let (lock, cv) = &*shared;
		let mut disarmed = lock.lock().unwrap_or_else(|p| p.into_inner());
		let mut last = u64::MAX;
		loop {
			let (g, _t) = cv
				.wait_timeout(disarmed, Duration::from_secs(20))
				.unwrap_or_else(|p| p.into_inner());
			disarmed = g;
			if *disarmed {
				return;
			}
			let b = BEAT.load(Ordering::Relaxed);
			if b == last {
				Write::error(
					"LOAD WEDGED: no progress for 20s — hipMallocAsync/HSA spin (known driver race). Press Ctrl-C to stop.",
				);
				return;
			}
			last = b;
		}
	});
	Watchdog { state }
}

fn ekey(l: usize, e: usize) -> String {
	format!("expert.{l}.{e}")
}

struct Arena {
	x: GpuBuffer,
	q: GpuBuffer,
	k: GpuBuffer,
	v: GpuBuffer,
	attn: GpuBuffer,
	o: GpuBuffer,
	attn_out: GpuBuffer,
	cms: GpuBuffer,
	g: GpuBuffer,
	u: GpuBuffer,
	act: GpuBuffer,
	mlp0: GpuBuffer,
	mlp: GpuBuffer,
	moe_xg: GpuBuffer,
	moe_gu: GpuBuffer,
	moe_ea: GpuBuffer,
	moe_dv: GpuBuffer,
	mo: GpuBuffer,
	ha: GpuBuffer,
	hb: GpuBuffer,
	embd_skip: GpuBuffer,
	soft: GpuBuffer,
	scn: GpuBuffer,
	sg: GpuBuffer,
	su: GpuBuffer,
	sa: GpuBuffer,
	sc_add: GpuBuffer,
	cur: GpuBuffer,
	normed: GpuBuffer,
	hfs: GpuBuffer,
	lm_out: GpuBuffer,
	mqa: GpuBuffer,
	mqb: GpuBuffer,
	mqn: GpuBuffer,
	mqp: GpuBuffer,
	mqx: GpuBuffer,
	mqc: GpuBuffer,
	mkv: GpuBuffer,
	mkc: GpuBuffer,
	mkp: GpuBuffer,
	mkk: GpuBuffer,
	mrw: GpuBuffer,
	mav: GpuBuffer,
	ss_x: GpuBuffer,
	ss_z: GpuBuffer,
	ss_xc: GpuBuffer,
	ss_db: GpuBuffer,
	ss_dtlr: GpuBuffer,
	ss_bb: GpuBuffer,
	ss_cc: GpuBuffer,
	ss_dt: GpuBuffer,
	ss_y: GpuBuffer,
	ss_xbc: GpuBuffer,
	ss_xbcc: GpuBuffer,
	ss_zx: GpuBuffer,
	d_qkv: GpuBuffer,
	d_cv: GpuBuffer,
	d_q: GpuBuffer,
	d_k: GpuBuffer,
	d_v: GpuBuffer,
	d_z: GpuBuffer,
	d_g: GpuBuffer,
	d_bt: GpuBuffer,
	d_o: GpuBuffer,
	cm: GpuBuffer,
	cl: GpuBuffer,
	cacc: GpuBuffer,
	blend_src: GpuBuffer,
	blend_w: GpuBuffer,
	finite: GpuBuffer,
}

const BLEND_K: usize = 8;

impl Arena {
	fn new(hp: &Hparams, t: usize) -> Result<Arena> {
		let c = hp.ncanvas.max(1);
		let ne = hp.ne;
		let nff = hp.nff;
		let nffe = hp.nffe;
		let a = |n: usize| -> Result<GpuBuffer> {
			return falloc(n).map_err(|_e| {
				anyhow!(
					"{}",
					gpu_core::memory::carve_miss_message(n * FWD_DT.elem_size())
				)
			});
		};
		let a64 = |n: usize| -> Result<GpuBuffer> {
			return GpuBuffer::alloc_ty(n, Dtype::F64)
				.map_err(|_e| anyhow!("{}", gpu_core::memory::carve_miss_message(n * 8)));
		};
		let blend_rows = t.max(c * BLEND_K);
		let nqh_max = hp.dims.iter().map(|d| return d.nqh).max().unwrap_or(1);
		let sacc = hp.qd_max.max(hp.kv_lora_rank).max(nqh_max * hp.head_k_mla);
		Ok(Arena {
			x: a(t * ne)?,
			q: a(t * hp.qd_max)?,
			k: a(t * hp.kd_max)?,
			v: a(t * hp.kd_max)?,
			attn: a(t * hp.qd_max)?,
			o: a(t * ne)?,
			attn_out: a(t * ne)?,
			cms: a(t * ne)?,
			g: a(t * nff)?,
			u: a(t * nff)?,
			act: a(t * nff)?,
			mlp0: a(t * ne)?,
			mlp: a(t * ne)?,
			moe_xg: a(t * ne)?,
			moe_gu: a(t * 2 * nffe)?,
			moe_ea: a(t * nffe)?,
			moe_dv: a(t * ne)?,
			mo: a(t * ne)?,
			ha: a(t * ne)?,
			hb: a(t * ne)?,
			embd_skip: a(if hp.arch == "talkie" { t * ne } else { 1 })?,
			blend_src: a(blend_rows * ne)?,
			blend_w: a64(blend_rows)?,
			finite: a(1)?,
			soft: a(c * ne)?,
			scn: a(c * ne)?,
			sg: a(c * nff)?,
			su: a(c * nff)?,
			sa: a(c * nff)?,
			sc_add: a(c * ne)?,
			cur: a(c * ne)?,
			normed: a(c * ne)?,
			hfs: a(c * ne)?,
			lm_out: a(c * hp.lm_chunk)?,
			mqa: a((t * hp.q_lora_rank).max(1))?,
			mqb: a((t * hp.nqh * hp.head_k_mla).max(1))?,
			mqn: a((t * hp.nqh * hp.head_k_mla.saturating_sub(hp.n_rot)).max(1))?,
			mqp: a((t * hp.nqh * hp.n_rot).max(1))?,
			mqx: a((t * hp.nqh * hp.kv_lora_rank).max(1))?,
			mqc: a((t * hp.nqh * (hp.kv_lora_rank + hp.n_rot)).max(1))?,
			mkv: a((t * (hp.kv_lora_rank + hp.n_rot)).max(1))?,
			mkc: a((t * hp.kv_lora_rank).max(1))?,
			mkp: a((t * hp.n_rot).max(1))?,
			mkk: a((t * (hp.kv_lora_rank + hp.n_rot)).max(1))?,
			mrw: a((t * hp.nqh * hp.kv_lora_rank).max(1))?,
			mav: a((t * hp.nqh * hp.head_v_mla).max(1))?,
			ss_x: a((t * hp.ssm_d_inner).max(1))?,
			ss_z: a((t * hp.ssm_d_inner).max(1))?,
			ss_xc: a((t * hp.ssm_d_inner).max(1))?,
			ss_db: a((t * (hp.ssm_dt_rank.max(hp.ssm_dt_dim) + 2 * hp.ssm_d_state)).max(1))?,
			ss_dtlr: a((t * hp.ssm_dt_rank.max(hp.ssm_dt_dim)).max(1))?,
			ss_bb: a((t * hp.ssm_d_state).max(1))?,
			ss_cc: a((t * hp.ssm_d_state).max(1))?,
			ss_dt: a((t * hp.ssm_d_inner).max(1))?,
			ss_y: a((t * hp.ssm_d_inner).max(1))?,
			ss_xbc: a((t * (hp.ssm_d_inner + 2 * hp.ssm_n_group * hp.ssm_d_state)).max(1))?,
			ss_xbcc: a((t * (hp.ssm_d_inner + 2 * hp.ssm_n_group * hp.ssm_d_state)).max(1))?,
			ss_zx: a((t * 2 * hp.ssm_d_inner * (hp.ssm_dt_dim.min(1))).max(1))?,
			d_qkv: a((t * hp.delta_win).max(1))?,
			d_cv: a((t * hp.delta_win).max(1))?,
			d_q: a((t * hp.delta_win).max(1))?,
			d_k: a((t * hp.delta_win).max(1))?,
			d_v: a((t * hp.delta_win).max(1))?,
			d_z: a((t * hp.delta_win).max(1))?,
			d_g: a((t * hp.delta_win).max(1))?,
			d_bt: a((t * hp.delta_win).max(1))?,
			d_o: a((t * hp.delta_win).max(1))?,
			cm: a((t * nqh_max).max(1))?,
			cl: a((t * nqh_max).max(1))?,
			cacc: a((t * sacc).max(1))?,
		})
	}
}

struct Model {
	shards: Vec<File>,
	big: HashMap<String, Tensor>,
	stage: GpuBuffer,
	win: GpuBuffer,
	store: Waterfall,
	rbuf: RefCell<Vec<u8>>,
	norms: Vec<[Option<GpuBuffer>; N_NORMS]>,
	norms_b: Vec<[Option<GpuBuffer>; N_NORMS]>,
	o_bias: Vec<Option<GpuBuffer>>,
	q_headscale: Vec<Option<GpuBuffer>>,
	ffn_up_bias: Vec<Option<GpuBuffer>>,
	ffn_down_bias: Vec<Option<GpuBuffer>>,
	embed_norm: Option<(GpuBuffer, GpuBuffer)>,
	decoder_norm: GpuBuffer,
	decoder_norm_b: Option<GpuBuffer>,
	sc_pre: GpuBuffer,
	sc_gate: GpuBuffer,
	sc_up: GpuBuffer,
	sc_down: GpuBuffer,
	rw: Vec<Vec<f64>>,
	gis: Vec<Vec<f64>>,
	pe: Vec<Vec<f64>>,
	emb: Raw,
	pos: Option<Raw>,
	out: Option<Raw>,
	out_b: Vec<f64>,
	eps: GpuBuffer,
	res_scale: GpuBuffer,
	attn_scale_mla: GpuBuffer,
	theta_full: GpuBuffer,
	theta_slide: GpuBuffer,
	rope_factors: Option<GpuBuffer>,
	ls_dev: Vec<GpuBuffer>,
	ssm_conv_w: Vec<Option<GpuBuffer>>,
	ssm_conv_b: Vec<Option<GpuBuffer>>,
	ssm_a: Vec<Option<GpuBuffer>>,
	ssm_d: Vec<Option<GpuBuffer>>,
	ssm_dt_b: Vec<Option<GpuBuffer>>,
	ssm_norm: Vec<Option<GpuBuffer>>,
	ffn_gate_bias: Vec<Option<GpuBuffer>>,
	ssm_dt_norm: Vec<Option<GpuBuffer>>,
	ssm_b_norm: Vec<Option<GpuBuffer>>,
	ssm_c_norm: Vec<Option<GpuBuffer>>,
	ssm_q_conv_w: Vec<Option<GpuBuffer>>,
	ssm_k_conv_w: Vec<Option<GpuBuffer>>,
	ssm_v_conv_w: Vec<Option<GpuBuffer>>,
	exp_probs_b: Vec<Option<GpuBuffer>>,
	hp: Hparams,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Nk {
	Input,
	Attn2,
	PostAttn,
	QNorm,
	KNorm,
	QANorm,
	KvANorm,
	PreFf,
	Pf1,
	Pn2,
	Pf2,
	Pfw,
}

pub(super) const N_NORMS: usize = 12;

impl Nk {
	fn name(self) -> &'static str {
		return match self {
			Nk::Input => "input",
			Nk::Attn2 => "attn2",
			Nk::PostAttn => "post_attn",
			Nk::QNorm => "q_norm",
			Nk::KNorm => "k_norm",
			Nk::QANorm => "q_a_norm",
			Nk::KvANorm => "kv_a_norm",
			Nk::PreFf => "pre_ff",
			Nk::Pf1 => "pf1",
			Nk::Pn2 => "pn2",
			Nk::Pf2 => "pf2",
			Nk::Pfw => "pfw",
		};
	}
}

const LAYER_NORMS: [(Nk, &str); N_NORMS] = [
	(Nk::Input, "input_layernorm.weight"),
	(Nk::Attn2, "self_attn.attn_norm_2.weight"),
	(Nk::PostAttn, "post_attention_layernorm.weight"),
	(Nk::QNorm, "self_attn.q_norm.weight"),
	(Nk::KNorm, "self_attn.k_norm.weight"),
	(Nk::QANorm, "self_attn.q_a_norm.weight"),
	(Nk::KvANorm, "self_attn.kv_a_norm.weight"),
	(Nk::PreFf, "pre_feedforward_layernorm.weight"),
	(Nk::Pf1, "post_feedforward_layernorm_1.weight"),
	(Nk::Pn2, "pre_feedforward_layernorm_2.weight"),
	(Nk::Pf2, "post_feedforward_layernorm_2.weight"),
	(Nk::Pfw, "post_feedforward_layernorm.weight"),
];

impl Model {
	fn qrange(t: &Tensor, elem_off: usize, elems: usize) -> Result<(usize, usize)> {
		let be = t.dt.block_elems();
		anyhow::ensure!(
			elem_off.is_multiple_of(be) && elems.is_multiple_of(be),
			"block-align: element range {elem_off}+{elems} off {:?} block boundary (be={be})",
			t.dt
		);
		return Ok((bytes_for(elem_off, t.dt), bytes_for(elems, t.dt)));
	}

	fn read_raw(&self, t: &Tensor, off: usize, len: usize) -> Result<Vec<u8>> {
		return read_raw_at(&self.shards, t, off, len);
	}

	fn read_into(&self, t: &Tensor, off: usize, len: usize, dst: &GpuBuffer, dst_off: usize) -> Result<()> {
		let mut rb = self.rbuf.borrow_mut();
		if rb.len() < len {
			rb.resize(len, 0);
		}
		let _d = Instant::now();
		self.shards[t.shard]
			.read_exact_at(&mut rb[..len], (t.off + off) as u64)
			.with_context(|| format!("read {len} bytes at shard {}", t.shard))?;
		acc(&DISK_NS, _d);
		let _h = Instant::now();
		bview(dst, dst_off, len).write_u8(&rb[..len])?;
		acc(&H2D_NS, _h);
		return Ok(());
	}

	fn small_f64(&self, name: &str) -> Result<Vec<f64>> {
		let t = self
			.big
			.get(name)
			.ok_or_else(|| anyhow!("missing {name}"))?;
		let n = t.elems();
		let dt = t.dt;
		let raw = self.read_raw(t, 0, t.nbytes)?;
		let whole = raw.len() / dt.block_bytes() * dt.block_elems();
		anyhow::ensure!(whole >= n, "small {name}: {whole} decoded < {n} needed");
		self.to_stage(&raw)?;
		let src = GpuBuffer::borrow(self.stage.ptr, raw.len()).as_dtype(dt);
		let dst = self.win.view(0, whole);
		gpu_convert(&src, &dst, whole, 1.0).map_err(|e| return anyhow!("small {name} convert: {e:?}"))?;
		let mut out = vec![0f64; whole];
		dst.download_host(&mut out)?;
		out.truncate(n);
		return Ok(out);
	}

	fn read_host(&self, t: &Tensor, off: usize, dst: &mut [u8]) -> Result<()> {
		let _d = Instant::now();
		self.shards[t.shard]
			.read_exact_at(dst, (t.off + off) as u64)
			.with_context(|| format!("read_host {} bytes", dst.len()))?;
		acc(&DISK_NS, _d);
		return Ok(());
	}

	fn widen_from(&self, src: &GpuBuffer, off_bytes: usize, n: usize, dt: Dtype) -> Result<GpuBuffer> {
		let _w = Instant::now();
		let raw_len = bytes_for(n, dt);
		gpu_convert(
			&bview(src, off_bytes, raw_len).as_dtype(dt),
			&self.win,
			n,
			1.0,
		)
		.map_err(|e| return anyhow!("convert launch: {e:?}"))?;
		acc(&WIDEN_NS, _w);
		return Ok(self.win.view(0, n));
	}

	fn to_stage(&self, bytes: &[u8]) -> Result<()> {
		let _h = Instant::now();
		self.stage.write_u8(bytes)?;
		acc(&H2D_NS, _h);
		Ok(())
	}

	fn stream(&self, name: &str) -> Result<GpuBuffer> {
		let t = self
			.big
			.get(name)
			.ok_or_else(|| anyhow!("missing {name}"))?;
		let n = t.elems();
		let dt = t.dt;
		match self.store.home(name) {
			Some(Home::Vram(dev)) => self.widen_from(dev, 0, n, dt),
			Some(Home::Ram(bytes)) => {
				self.to_stage(bytes)?;
				self.widen_from(&self.stage, 0, n, dt)
			}
			_other => {
				self.read_into(t, 0, t.nbytes, &self.stage, 0)?;
				self.widen_from(&self.stage, 0, n, dt)
			}
		}
	}

	fn layer_is_moe(&self, l: usize) -> bool {
		self.big.contains_key(&layer_name(l, "experts.gate_proj"))
			|| self
				.big
				.contains_key(&layer_name(l, "experts.gate_up_proj"))
	}

	fn fill_expert(&self, l: usize, e: usize, dst: &mut [u8]) -> Result<()> {
		let (gu_bytes, dn_bytes, half) = (self.hp.gu_bytes, self.hp.dn_bytes, self.hp.gu_bytes / 2);
		if let Some(gu) = self.big.get(&layer_name(l, "experts.gate_up_proj")) {
			self.read_host(gu, e * gu_bytes, &mut dst[..gu_bytes])?;
		} else {
			let g = self
				.big
				.get(&layer_name(l, "experts.gate_proj"))
				.ok_or_else(|| anyhow!("no expert gate {l}"))?;
			let u = self
				.big
				.get(&layer_name(l, "experts.up_proj"))
				.ok_or_else(|| anyhow!("no expert up {l}"))?;
			self.read_host(g, e * half, &mut dst[..half])?;
			self.read_host(u, e * half, &mut dst[half..gu_bytes])?;
		}
		let dn = self
			.big
			.get(&layer_name(l, "experts.down_proj"))
			.ok_or_else(|| anyhow!("no expert down {l}"))?;
		self.read_host(dn, e * dn_bytes, &mut dst[gu_bytes..])?;
		Ok(())
	}

	fn expert_slot(&self, l: usize, e: usize) -> Result<GpuBuffer> {
		let (gu_bytes, dn_bytes, half, slot_bytes) = (
			self.hp.gu_bytes,
			self.hp.dn_bytes,
			self.hp.gu_bytes / 2,
			self.hp.slot_bytes,
		);
		match self.store.home(&ekey(l, e)) {
			Some(Home::Vram(dev)) => {
				E_VRAM.fetch_add(1, Ordering::Relaxed);
				Ok(bview(dev, 0, slot_bytes))
			}
			Some(Home::Ram(bytes)) => {
				E_RAM.fetch_add(1, Ordering::Relaxed);
				self.to_stage(bytes)?;
				Ok(bview(&self.stage, 0, slot_bytes))
			}
			_other => {
				E_DISK.fetch_add(1, Ordering::Relaxed);
				if let Some(gu) = self.big.get(&layer_name(l, "experts.gate_up_proj")) {
					self.read_into(gu, e * gu_bytes, gu_bytes, &self.stage, 0)?;
				} else {
					let g = self
						.big
						.get(&layer_name(l, "experts.gate_proj"))
						.ok_or_else(|| anyhow!("no expert gate {l}"))?;
					let u = self
						.big
						.get(&layer_name(l, "experts.up_proj"))
						.ok_or_else(|| anyhow!("no expert up {l}"))?;
					self.read_into(g, e * half, half, &self.stage, 0)?;
					self.read_into(u, e * half, half, &self.stage, half)?;
				}
				let dn = self
					.big
					.get(&layer_name(l, "experts.down_proj"))
					.ok_or_else(|| anyhow!("no expert down {l}"))?;
				self.read_into(dn, e * dn_bytes, dn_bytes, &self.stage, gu_bytes)?;
				Ok(bview(&self.stage, 0, slot_bytes))
			}
		}
	}
}

fn upload_gamma(vals: &[f64], plus_one: bool) -> Result<GpuBuffer> {
	if plus_one {
		let v: Vec<f64> = vals.iter().map(|x| x + 1.0).collect();
		let ub = falloc(v.len())?;
		ub.load(&v)?;
		Ok(ub)
	} else {
		let ub = falloc(vals.len())?;
		ub.load(vals)?;
		Ok(ub)
	}
}

const GLOBAL_MAP: &[(&str, &str)] = &[
	("token_embd.weight", "embed_tokens.weight"),
	("token_embd_norm.weight", "embed_norm.weight"),
	("token_embd_norm.bias", "embed_norm.bias"),
	("position_embd.weight", "pos_embd.weight"),
	("output.weight", "lm_head.weight"),
	("output.bias", "lm_head.bias"),
	("output_norm.weight", "norm.weight"),
	("output_norm.bias", "norm.bias"),
	("rope_factors_short.weight", "rope_short.weight"),
	("rope_factors_long.weight", "rope_long.weight"),
	(
		"self_cond_pre_norm.weight",
		"self_conditioning.pre_norm.weight",
	),
	(
		"self_cond_gate.weight",
		"self_conditioning.gate_proj.weight",
	),
	("self_cond_up.weight", "self_conditioning.up_proj.weight"),
	(
		"self_cond_down.weight",
		"self_conditioning.down_proj.weight",
	),
];

const LAYER_MAP: &[(&str, &str)] = &[
	("attn_norm.weight", "input_layernorm.weight"),
	("attn_norm.bias", "input_layernorm.bias"),
	("attn_norm_2.weight", "self_attn.attn_norm_2.weight"),
	("attn_norm_2.bias", "self_attn.attn_norm_2.bias"),
	(
		"attn_output_norm.weight",
		"pre_feedforward_layernorm.weight",
	),
	(
		"post_attention_norm.weight",
		"post_attention_layernorm.weight",
	),
	("post_attention_norm.bias", "post_attention_layernorm.bias"),
	("attn_q_norm.weight", "self_attn.q_norm.weight"),
	("attn_k_norm.weight", "self_attn.k_norm.weight"),
	("ffn_norm.weight", "pre_feedforward_layernorm.weight"),
	("ffn_norm.bias", "pre_feedforward_layernorm.bias"),
	(
		"post_ffw_norm_1.weight",
		"post_feedforward_layernorm_1.weight",
	),
	(
		"pre_ffw_norm_2.weight",
		"pre_feedforward_layernorm_2.weight",
	),
	(
		"post_ffw_norm_2.weight",
		"post_feedforward_layernorm_2.weight",
	),
	("post_ffw_norm.weight", "post_feedforward_layernorm.weight"),
	("attn_q.weight", "self_attn.q_proj.weight"),
	("attn_k.weight", "self_attn.k_proj.weight"),
	("attn_v.weight", "self_attn.v_proj.weight"),
	("shortconv.conv.weight", "self_attn.shortconv_conv.weight"),
	(
		"shortconv.in_proj.weight",
		"self_attn.shortconv_in_proj.weight",
	),
	(
		"shortconv.out_proj.weight",
		"self_attn.shortconv_out_proj.weight",
	),
	("attn_q_a.weight", "self_attn.q_a_proj.weight"),
	("attn_q_b.weight", "self_attn.q_b_proj.weight"),
	("attn_kv_a_mqa.weight", "self_attn.kv_a_proj.weight"),
	("attn_k_b.weight", "self_attn.k_b_proj.weight"),
	("attn_v_b.weight", "self_attn.v_b_proj.weight"),
	("attn_kv_b.weight", "self_attn.kv_b_proj.weight"),
	("attn_q_a_norm.weight", "self_attn.q_a_norm.weight"),
	("attn_kv_a_norm.weight", "self_attn.kv_a_norm.weight"),
	("attn_output.weight", "self_attn.o_proj.weight"),
	("attn_output.bias", "self_attn.o_proj.bias"),
	("ffn_up.bias", "mlp.up_proj.bias"),
	("ffn_down.bias", "mlp.down_proj.bias"),
	("ffn_gate.bias", "mlp.gate_proj.bias"),
	("attn_qkv.weight", "self_attn.qkv_proj.weight"),
	("ffn_gate.weight", "mlp.gate_proj.weight"),
	("ffn_up.weight", "mlp.up_proj.weight"),
	("ffn_down.weight", "mlp.down_proj.weight"),
	("ffn_gate_exps.weight", "experts.gate_proj"),
	("ffn_up_exps.weight", "experts.up_proj"),
	("ffn_gate_up_exps.weight", "experts.gate_up_proj"),
	("ffn_down_exps.weight", "experts.down_proj"),
	("ffn_gate_inp.weight", "router.proj.weight"),
	("ffn_gate_inp.scale", "router.scale"),
	("ffn_down_exps.scale", "router.per_expert_scale"),
	("layer_output_scale.weight", "layer_scalar"),
	("ssm_in.weight", "self_attn.ssm_in.weight"),
	("ssm_conv1d.weight", "self_attn.ssm_conv1d.weight"),
	("ssm_conv1d.bias", "self_attn.ssm_conv1d.bias"),
	("ssm_x.weight", "self_attn.ssm_x.weight"),
	("ssm_dt.weight", "self_attn.ssm_dt.weight"),
	("ssm_dt.bias", "self_attn.ssm_dt.bias"),
	("ssm_a", "self_attn.ssm_a"),
	("ssm_d", "self_attn.ssm_d"),
	("ssm_out.weight", "self_attn.ssm_out.weight"),
	("ssm_norm.weight", "self_attn.ssm_norm.weight"),
	("ssm_dt_norm.weight", "self_attn.ssm_dt_norm.weight"),
	("ssm_b_norm.weight", "self_attn.ssm_b_norm.weight"),
	("ssm_c_norm.weight", "self_attn.ssm_c_norm.weight"),
	("ffn_norm", "pre_feedforward_layernorm.weight"),
	("post_attention_norm", "post_attention_layernorm.weight"),
	("post_ffw_norm", "post_feedforward_layernorm.weight"),
	("ssm_dt_norm", "self_attn.ssm_dt_norm.weight"),
	("ssm_b_norm", "self_attn.ssm_b_norm.weight"),
	("ssm_c_norm", "self_attn.ssm_c_norm.weight"),
	("attn_gate.weight", "self_attn.z_gate.weight"),
	("ssm_ba.weight", "self_attn.ssm_ba.weight"),
	("ssm_alpha.weight", "self_attn.ssm_alpha.weight"),
	("ssm_beta.weight", "self_attn.ssm_beta.weight"),
	("ssm_conv1d_q.weight", "self_attn.q_conv.weight"),
	("ssm_conv1d_k.weight", "self_attn.k_conv.weight"),
	("ssm_conv1d_v.weight", "self_attn.v_conv.weight"),
	("ssm_f_a.weight", "self_attn.f_a.weight"),
	("ssm_f_b.weight", "self_attn.f_b.weight"),
	("ssm_g_a.weight", "self_attn.g_a.weight"),
	("ssm_g_b.weight", "self_attn.g_b.weight"),
	("ffn_gate_inp_shexp.weight", "shexp.gate_inp.weight"),
	("ffn_gate_shexp.weight", "shexp.gate.weight"),
	("ffn_up_shexp.weight", "shexp.up.weight"),
	("ffn_down_shexp.weight", "shexp.down.weight"),
	("exp_probs_b.bias", "router.bias"),
];

#[derive(Clone, Copy)]
enum Slice {
	QkvQ,
	QkvK,
	QkvV,
}

const SLICE_MAP: &[(&str, &str, Slice)] = &[
	(
		"self_attn.q_proj.weight",
		"self_attn.qkv_proj.weight",
		Slice::QkvQ,
	),
	(
		"self_attn.k_proj.weight",
		"self_attn.qkv_proj.weight",
		Slice::QkvK,
	),
	(
		"self_attn.v_proj.weight",
		"self_attn.qkv_proj.weight",
		Slice::QkvV,
	),
];

fn hf_name(gg: &str) -> Option<String> {
	for (raw, neutral) in GLOBAL_MAP {
		if gg == *raw {
			return Some(format!("model.decoder.{neutral}"));
		}
	}
	let rest = gg.strip_prefix("blk.")?;
	let dot = rest.find('.')?;
	let l: usize = rest[..dot].parse().ok()?;
	let suffix = &rest[dot + 1..];
	for (raw, neutral) in LAYER_MAP {
		if suffix == *raw {
			return Some(format!("model.decoder.layers.{l}.{neutral}"));
		}
	}
	None
}

fn load_model_gguf(path: &Path) -> Result<Model> {
	let g = Gguf::open(path)?;
	let hp = Hparams::from_gguf(&g)?;
	let f = File::open(path)?;
	let mut big: HashMap<String, Tensor> = HashMap::new();
	for (name, info) in &g.tensors {
		let Some(hf) = hf_name(name) else {
			continue;
		};
		let mut shape: Vec<usize> = info.dims.clone();
		shape.reverse();
		big.insert(
			hf,
			Tensor {
				shard: 0,
				off: info.offset as usize,
				nbytes: info.nbytes,
				shape,
				dt: crate::dequant::from_ggml(info.ggml_type)?,
			},
		);
	}
	if matches!(hp.arch.as_str(), "lfm2" | "lfm2moe")
		&& let Some(t) = big.remove("model.decoder.embed_norm.weight")
	{
		big.insert("model.decoder.norm.weight".to_string(), t);
	}
	synth_qkv_slices(&mut big, &hp);
	synth_ffn_slices(&mut big, &hp);
	finish_load(vec![f], big, hp)
}

fn synth_ffn_slices(big: &mut HashMap<String, Tensor>, hp: &Hparams) {
	let nff = hp.nff;
	for l in 0..hp.nl {
		let up_k = layer_name(l, "mlp.up_proj.weight");
		if big.contains_key(&layer_name(l, "mlp.gate_proj.weight")) {
			continue;
		}
		let Some(up) = big.get(&up_k) else {
			continue;
		};
		if up.shape.first() != Some(&(2 * nff)) || up.shape.len() != 2 {
			continue;
		}
		let ne = up.shape[1];
		let (src_shard, src_off, src_dt) = (up.shard, up.off, up.dt);
		let (Ok((q0, part_bytes)), Ok((q1, _))) = (
			Model::qrange(up, 0, nff * ne),
			Model::qrange(up, nff * ne, nff * ne),
		) else {
			continue;
		};
		let part = |qoff: usize| Tensor {
			shard: src_shard,
			off: src_off + qoff,
			nbytes: part_bytes,
			shape: vec![nff, ne],
			dt: src_dt,
		};
		big.insert(layer_name(l, "mlp.gate_proj.weight"), part(q0));
		big.insert(up_k, part(q1));
	}
}

fn synth_qkv_slices(big: &mut HashMap<String, Tensor>, hp: &Hparams) {
	for l in 0..hp.nl {
		let d = &hp.dims[l];
		let (qd, kd) = (hp.nqh * d.hd, d.nkv * d.hd);
		for (neutral, src, slice) in SLICE_MAP {
			let nk = layer_name(l, neutral);
			if big.contains_key(&nk) {
				continue;
			}
			let Some(src_t) = big.get(&layer_name(l, src)) else {
				continue;
			};
			if src_t.shape.first() != Some(&(qd + 2 * kd)) || src_t.shape.len() != 2 {
				continue;
			}
			let ne = src_t.shape[1];
			let (row0, rows) = match slice {
				Slice::QkvQ => (0, qd),
				Slice::QkvK => (qd, kd),
				Slice::QkvV => (qd + kd, kd),
			};
			let Ok((qoff, sub_bytes)) = Model::qrange(src_t, row0 * ne, rows * ne) else {
				continue;
			};
			let sub = Tensor {
				shard: src_t.shard,
				off: src_t.off + qoff,
				nbytes: sub_bytes,
				shape: vec![rows, ne],
				dt: src_t.dt,
			};
			big.insert(nk, sub);
		}
	}
}

fn finish_load(shards: Vec<File>, big: HashMap<String, Tensor>, hp: Hparams) -> Result<Model> {
	Write::line(data, "allocating stage+win");
	let eps = {
		let ub = falloc(1)?;
		ub.load(&[hp.eps])?;
		ub
	};
	let theta_full = {
		let ub = falloc(1)?;
		ub.load(&[hp.freq_base])?;
		ub
	};
	let theta_slide = {
		let ub = falloc(1)?;
		ub.load(&[hp.freq_base_swa])?;
		ub
	};
	let res_scale = {
		let ub = falloc(1)?;
		let s = if hp.arch == "minicpm3" {
			1.4 / (hp.nl as f64).sqrt()
		} else if hp.residual_scale > 0.0 {
			hp.residual_scale
		} else {
			1.0
		};
		ub.load(&[s])?;
		ub
	};
	let attn_scale_mla = {
		let ub = falloc(1)?;
		let s = if hp.is_mla() {
			1.0 / (hp.head_k_mla as f64).sqrt()
		} else {
			1.0
		};
		ub.load(&[s])?;
		ub
	};
	let nl = hp.nl;
	let vocab = hp.vocab;
	let ne = hp.ne;
	let et = big
		.get("model.decoder.embed_tokens.weight")
		.ok_or_else(|| return anyhow!("no embed_tokens"))?;
	if et.shape != vec![vocab, ne] {
		bail!("embed_tokens shape {:?}", et.shape);
	}
	let emb = read_whole(&shards, et)?;
	let out = match big.get("model.decoder.lm_head.weight") {
		Some(ot) if ot.shape == vec![vocab, ne] => Some(read_whole(&shards, ot)?),
		_other => None,
	};
	let pos = match big.get("model.decoder.pos_embd.weight") {
		Some(pt) => Some(read_whole(&shards, pt)?),
		None => None,
	};
	let mut m = Model {
		shards,
		big,
		stage: GpuBuffer::alloc_bytes(hp.stage_bytes)?,
		win: falloc(hp.win_elems)?,
		store: Waterfall::new(),
		rbuf: RefCell::new(Vec::new()),
		norms: Vec::new(),
		norms_b: Vec::new(),
		o_bias: Vec::new(),
		q_headscale: Vec::new(),
		ffn_up_bias: Vec::new(),
		ffn_down_bias: Vec::new(),
		embed_norm: None,
		decoder_norm: falloc(1)?,
		decoder_norm_b: None,
		sc_pre: falloc(1)?,
		sc_gate: falloc(1)?,
		sc_up: falloc(1)?,
		sc_down: falloc(1)?,
		rw: Vec::new(),
		gis: Vec::new(),
		pe: Vec::new(),
		emb,
		pos,
		out,
		out_b: Vec::new(),
		eps,
		res_scale,
		attn_scale_mla,
		theta_full,
		theta_slide,
		rope_factors: None,
		ls_dev: Vec::new(),
		ssm_conv_w: Vec::new(),
		ssm_conv_b: Vec::new(),
		ssm_a: Vec::new(),
		ssm_d: Vec::new(),
		ssm_dt_b: Vec::new(),
		ssm_norm: Vec::new(),
		ffn_gate_bias: Vec::new(),
		ssm_dt_norm: Vec::new(),
		ssm_b_norm: Vec::new(),
		ssm_c_norm: Vec::new(),
		ssm_q_conv_w: Vec::new(),
		ssm_k_conv_w: Vec::new(),
		ssm_v_conv_w: Vec::new(),
		exp_probs_b: Vec::new(),
		hp,
	};

	let plus_one = false;
	Write::line(
		data,
		"norm convention: folded x*w (gguf stores gammas as saved)",
	);

	for l in 0..nl {
		Write::line(data, format!("norms layer {}/{}", l + 1, nl));
		beat();
		let p = |n: &str| format!("model.decoder.layers.{l}.{n}");
		let mut nm: [Option<GpuBuffer>; N_NORMS] = std::array::from_fn(|_| None);
		let mut nmb: [Option<GpuBuffer>; N_NORMS] = std::array::from_fn(|_| None);
		for (key, suffix) in LAYER_NORMS {
			if m.big.contains_key(&p(suffix)) {
				nm[key as usize] = Some(upload_gamma(&m.small_f64(&p(suffix))?, plus_one)?);
			}
			let bname = p(&suffix.replace(".weight", ".bias"));
			if m.big.contains_key(&bname) {
				let vals = m.small_f64(&bname)?;
				let ub = falloc(vals.len())?;
				ub.load(&vals)?;
				nmb[key as usize] = Some(ub);
			}
		}
		if nm[Nk::PreFf as usize].is_none() && m.big.contains_key(&p("post_attention_layernorm.weight")) {
			let g = upload_gamma(
				&m.small_f64(&p("post_attention_layernorm.weight"))?,
				plus_one,
			)?;
			nm[Nk::PreFf as usize] = Some(g);
		}
		m.norms.push(nm);
		m.norms_b.push(nmb);
		let opt_bias = |m: &Model, name: String| -> Result<Option<GpuBuffer>> {
			if m.big.contains_key(&name) {
				let vals = m.small_f64(&name)?;
				let ub = falloc(vals.len())?;
				ub.load(&vals)?;
				return Ok(Some(ub));
			}
			return Ok(None);
		};
		m.o_bias.push(opt_bias(&m, p("self_attn.o_proj.bias"))?);
		let qhs = if m.hp.arch == "talkie" && m.big.contains_key(&p("self_attn.q_norm.weight")) {
			let per_head = m.small_f64(&p("self_attn.q_norm.weight"))?;
			let (nqh, hd) = (m.hp.dims[l].nqh, m.hp.dims[l].hd);
			let mut expanded = vec![0.0f64; nqh * hd];
			for h in 0..nqh {
				for x in 0..hd {
					expanded[h * hd + x] = per_head[h % per_head.len()];
				}
			}
			let ub = falloc(expanded.len())?;
			ub.load(&expanded)?;
			Some(ub)
		} else {
			None
		};
		m.q_headscale.push(qhs);
		m.ffn_up_bias.push(opt_bias(&m, p("mlp.up_proj.bias"))?);
		m.ffn_down_bias.push(opt_bias(&m, p("mlp.down_proj.bias"))?);
		let opt_small = |m: &Model, name: String| -> Result<Vec<f64>> {
			if m.big.contains_key(&name) {
				m.small_f64(&name)
			} else {
				Ok(Vec::new())
			}
		};
		m.rw.push(opt_small(&m, p("router.proj.weight"))?);
		m.gis.push(opt_small(&m, p("router.scale"))?);
		m.pe.push(opt_small(&m, p("router.per_expert_scale"))?);
		let lsv = if m.big.contains_key(&p("layer_scalar")) {
			m.small_f64(&p("layer_scalar"))?[0]
		} else {
			1.0
		};
		m.ls_dev.push({
			let ub = falloc(1)?;
			ub.load(&[lsv])?;
			ub
		});
		m.ssm_conv_w
			.push(opt_bias(&m, p("self_attn.ssm_conv1d.weight"))?);
		let conv_b = match opt_bias(&m, p("self_attn.ssm_conv1d.bias"))? {
			Some(b) => Some(b),
			None if m.big.contains_key(&p("self_attn.ssm_conv1d.weight")) => {
				let zb = falloc(m.hp.ssm_d_inner.max(1))?;
				zb.load(&vec![0.0f64; m.hp.ssm_d_inner.max(1)])?;
				Some(zb)
			}
			None => None,
		};
		m.ssm_conv_b.push(conv_b);
		m.ssm_a.push(opt_bias(&m, p("self_attn.ssm_a"))?);
		m.ssm_d.push(opt_bias(&m, p("self_attn.ssm_d"))?);
		m.ssm_dt_b.push(opt_bias(&m, p("self_attn.ssm_dt.bias"))?);
		m.ssm_norm
			.push(opt_bias(&m, p("self_attn.ssm_norm.weight"))?);
		m.ffn_gate_bias.push(opt_bias(&m, p("mlp.gate_proj.bias"))?);
		m.ssm_dt_norm
			.push(opt_bias(&m, p("self_attn.ssm_dt_norm.weight"))?);
		m.ssm_b_norm
			.push(opt_bias(&m, p("self_attn.ssm_b_norm.weight"))?);
		m.ssm_c_norm
			.push(opt_bias(&m, p("self_attn.ssm_c_norm.weight"))?);
		m.ssm_q_conv_w
			.push(opt_bias(&m, p("self_attn.q_conv.weight"))?);
		m.ssm_k_conv_w
			.push(opt_bias(&m, p("self_attn.k_conv.weight"))?);
		m.ssm_v_conv_w
			.push(opt_bias(&m, p("self_attn.v_conv.weight"))?);
		m.exp_probs_b.push(opt_bias(&m, p("router.bias"))?);
	}

	Write::line(data, "globals + embedding table");
	if m.big.contains_key("model.decoder.rope_short.weight") {
		let vals = m.small_f64("model.decoder.rope_short.weight")?;
		let ub = falloc(vals.len())?;
		ub.load(&vals)?;
		m.rope_factors = Some(ub);
	}
	if m.big.contains_key("model.decoder.norm.weight") {
		m.decoder_norm = upload_gamma(&m.small_f64("model.decoder.norm.weight")?, plus_one)?;
	}
	if m.big.contains_key("model.decoder.norm.bias") {
		let vals = m.small_f64("model.decoder.norm.bias")?;
		let ub = falloc(vals.len())?;
		ub.load(&vals)?;
		m.decoder_norm_b = Some(ub);
	}
	if m.big.contains_key("model.decoder.embed_norm.weight") && m.big.contains_key("model.decoder.embed_norm.bias") {
		let g = {
			let vals = m.small_f64("model.decoder.embed_norm.weight")?;
			let ub = falloc(vals.len())?;
			ub.load(&vals)?;
			ub
		};
		let b = {
			let vals = m.small_f64("model.decoder.embed_norm.bias")?;
			let ub = falloc(vals.len())?;
			ub.load(&vals)?;
			ub
		};
		m.embed_norm = Some((g, b));
	}
	if m.big
		.contains_key("model.decoder.self_conditioning.pre_norm.weight")
	{
		m.sc_pre = upload_gamma(
			&m.small_f64("model.decoder.self_conditioning.pre_norm.weight")?,
			plus_one,
		)?;
		m.sc_gate = {
			let vals = m.small_f64("model.decoder.self_conditioning.gate_proj.weight")?;
			let ub = falloc(vals.len())?;
			ub.load(&vals)?;
			ub
		};
		m.sc_up = {
			let vals = m.small_f64("model.decoder.self_conditioning.up_proj.weight")?;
			let ub = falloc(vals.len())?;
			ub.load(&vals)?;
			ub
		};
		m.sc_down = {
			let vals = m.small_f64("model.decoder.self_conditioning.down_proj.weight")?;
			let ub = falloc(vals.len())?;
			ub.load(&vals)?;
			ub
		};
	}

	if m.big.contains_key("model.decoder.lm_head.bias") {
		m.out_b = m.small_f64("model.decoder.lm_head.bias")?;
	}

	Ok(m)
}

fn fixed_names(m: &Model, l: usize) -> Vec<String> {
	const ROLES: [&str; 17] = [
		"self_attn.q_proj.weight",
		"self_attn.k_proj.weight",
		"self_attn.v_proj.weight",
		"self_attn.q_a_proj.weight",
		"self_attn.q_b_proj.weight",
		"self_attn.kv_a_proj.weight",
		"self_attn.k_b_proj.weight",
		"self_attn.v_b_proj.weight",
		"self_attn.kv_b_proj.weight",
		"self_attn.o_proj.weight",
		"mlp.gate_proj.weight",
		"mlp.up_proj.weight",
		"mlp.down_proj.weight",
		"self_attn.ssm_in.weight",
		"self_attn.ssm_x.weight",
		"self_attn.ssm_dt.weight",
		"self_attn.ssm_out.weight",
	];
	ROLES.iter()
		.map(|r| layer_name(l, r))
		.filter(|n| m.big.contains_key(n))
		.collect()
}

fn preflight(m: &Model, ar: &Arena, t: usize) -> Result<()> {
	let hp = &m.hp;
	gpu_gemm_bt(
		&ar.x,
		&m.win.view(0, hp.qd_max * hp.ne),
		t,
		hp.qd_max,
		hp.ne,
		&ar.q,
	)?;
	beat();
	gpu_gemm_bt(
		&ar.cms,
		&m.win.view(0, hp.nff * hp.ne),
		t,
		hp.nff,
		hp.ne,
		&ar.g,
	)?;
	beat();
	gpu_gemm_bt(
		&ar.act,
		&m.win.view(0, hp.ne * hp.nff),
		t,
		hp.ne,
		hp.nff,
		&ar.mlp0,
	)?;
	beat();
	gpu_gemm_bt(
		&ar.moe_xg,
		&m.win.view(0, 2 * hp.nffe * hp.ne),
		t,
		2 * hp.nffe,
		hp.ne,
		&ar.moe_gu,
	)?;
	gpu_core::hip::device_synchronize()?;
	beat();
	Ok(())
}

fn fill_store(m: &mut Model, store: Waterfall, cancel: &mut dyn FnMut() -> bool) -> Result<bool> {
	let mut store = store;
	let placed = fill_into(m, &mut store, cancel);
	store.report();
	m.store = store;
	if !placed? {
		return Ok(false);
	}
	let nl = m.hp.nl;
	let mut canary = vec!["model.decoder.embed_tokens.weight".to_string()];
	if let Some(first) = fixed_names(m, 0).into_iter().next() {
		canary.push(first);
	}
	if let Some(last) = fixed_names(m, nl - 1).into_iter().next_back() {
		canary.push(last);
	}
	for name in canary {
		if let Some(Home::Vram(dev)) = m.store.home(&name) {
			let t = &m.big[&name];
			let n = 4096.min(t.nbytes);
			for off in [0, t.nbytes - n] {
				let want = if name.ends_with("embed_tokens.weight") {
					m.emb.bytes[off..off + n].to_vec()
				} else {
					m.read_raw(t, off, n)?
				};
				let mut got = vec![0u8; n];
				bview(dev, off, n).download_u8(&mut got)?;
				if got != want {
					bail!("waterfall {name} stale at byte {off}: upload not visible to GPU reads");
				}
			}
		}
	}
	return Ok(true);
}

fn fill_into(m: &Model, store: &mut Waterfall, cancel: &mut dyn FnMut() -> bool) -> Result<bool> {
	let nl = m.hp.nl;
	let nexp = m.hp.nexp;
	let slot_bytes = m.hp.slot_bytes;
	beat();
	store.place(
		"model.decoder.embed_tokens.weight",
		m.emb.bytes.len(),
		|dst| {
			dst.copy_from_slice(&m.emb.bytes);
			Ok(())
		},
	)?;
	beat();
	if cancel() {
		return Ok(false);
	}
	for l in 0..nl {
		for name in fixed_names(m, l) {
			let t = m.big.get(&name).ok_or_else(|| anyhow!("missing {name}"))?;
			store.place(&name, t.nbytes, |dst| {
				m.read_host(t, 0, dst).map_err(io::Error::other)
			})?;
			beat();
			if cancel() {
				return Ok(false);
			}
		}
	}
	for e in 0..nexp {
		for l in 0..nl {
			if !m.layer_is_moe(l) {
				continue;
			}
			store.place(&ekey(l, e), slot_bytes, |dst| {
				m.fill_expert(l, e, dst).map_err(io::Error::other)
			})?;
			beat();
			if cancel() {
				return Ok(false);
			}
		}
	}
	return Ok(true);
}

fn softmax(v: &mut [f64]) {
	let m = v.iter().cloned().fold(f64::MIN, f64::max);
	let mut s = 0.0;
	for x in v.iter_mut() {
		*x = (*x - m).exp();
		s += *x;
	}
	for x in v.iter_mut() {
		*x /= s;
	}
}

fn xs(st: &mut u64) -> f64 {
	*st ^= *st << 13;
	*st ^= *st >> 7;
	*st ^= *st << 17;
	(*st >> 11) as f64 / (1u64 << 53) as f64
}

fn layer_name(l: usize, suffix: &str) -> String {
	format!("model.decoder.layers.{l}.{suffix}")
}

fn lm_head(m: &Model, hfs: &GpuBuffer, ncanvas: usize, ar: &Arena) -> Result<Vec<f64>> {
	let hp = &m.hp;
	let mut logits = vec![0.0f64; ncanvas * hp.vocab];
	let mut out_host = vec![0.0f64; ncanvas * hp.lm_chunk];
	lm_head_into(m, hfs, ncanvas, ar, &mut logits, &mut out_host)?;
	Ok(logits)
}

fn lm_head_into(
	m: &Model,
	hfs: &GpuBuffer,
	ncanvas: usize,
	ar: &Arena,
	logits: &mut [f64],
	out_host: &mut [f64],
) -> Result<()> {
	let hp = &m.hp;
	let _tl = Instant::now();
	let mut c0 = 0;
	while c0 < hp.vocab {
		let cn = hp.lm_chunk.min(hp.vocab - c0);
		let w = match &m.out {
			Some(out) => {
				let dt = out.dt;
				m.to_stage(&out.bytes[bytes_for(c0 * hp.ne, dt)..bytes_for((c0 + cn) * hp.ne, dt)])?;
				m.widen_from(&m.stage, 0, cn * hp.ne, dt)?
			}
			None => {
				let dt = m.emb.dt;
				match m.store.home("model.decoder.embed_tokens.weight") {
					Some(Home::Vram(dev)) => m.widen_from(dev, bytes_for(c0 * hp.ne, dt), cn * hp.ne, dt)?,
					_other => {
						m.to_stage(
							&m.emb.bytes[bytes_for(c0 * hp.ne, dt)..bytes_for((c0 + cn) * hp.ne, dt)],
						)?;
						m.widen_from(&m.stage, 0, cn * hp.ne, dt)?
					}
				}
			}
		};
		gpu_gemm_bt(hfs, &w, ncanvas, cn, hp.ne, &ar.lm_out)?;
		ar.lm_out.download_host(&mut out_host[..ncanvas * cn])?;
		gpu_core::hip::device_synchronize()?;
		for p in 0..ncanvas {
			logits[p * hp.vocab + c0..p * hp.vocab + c0 + cn].copy_from_slice(&out_host[p * cn..(p + 1) * cn]);
		}
		c0 += cn;
	}
	if models::out_bias(m) && !m.out_b.is_empty() {
		for p in 0..ncanvas {
			for (j, lg) in logits[p * hp.vocab..(p + 1) * hp.vocab]
				.iter_mut()
				.enumerate()
			{
				*lg += m.out_b[j];
			}
		}
	}
	acc(&LM_NS, _tl);
	Ok(())
}

pub(crate) enum LayerCache {
	Kv(KvSlot),
	Scan(ScanSlot),
	Conv(ConvSlot),
	KvScan(KvSlot, ScanSlot),
}

pub(crate) struct KvSlot {
	pub(crate) k: GpuBuffer,
	pub(crate) v: GpuBuffer,
	pub(crate) hk: HostStore,
	pub(crate) hv: HostStore,
}

pub(crate) struct ScanSlot {
	pub(crate) rec: GpuBuffer,
	pub(crate) conv: Vec<GpuBuffer>,
	pub(crate) nxt: Vec<GpuBuffer>,
}

pub(crate) struct ConvSlot {
	pub(crate) conv: Vec<GpuBuffer>,
	pub(crate) nxt: Vec<GpuBuffer>,
}

impl LayerCache {
	pub(crate) fn kv(&self) -> Result<&KvSlot> {
		match self {
			LayerCache::Kv(s) | LayerCache::KvScan(s, _) => return Ok(s),
			_other => return Err(anyhow!("attention on a layer without K/V cache rows")),
		}
	}

	pub(crate) fn rec(&self) -> Result<&GpuBuffer> {
		match self {
			LayerCache::Scan(s) | LayerCache::KvScan(_, s) => return Ok(&s.rec),
			_other => return Err(anyhow!("scan mixer on a layer without recurrent state")),
		}
	}

	pub(crate) fn conv_io(&self, i: usize) -> Result<(&GpuBuffer, &GpuBuffer)> {
		let (c, n) = match self {
			LayerCache::Scan(s) | LayerCache::KvScan(_, s) => (&s.conv, &s.nxt),
			LayerCache::Conv(s) => (&s.conv, &s.nxt),
			LayerCache::Kv(_) => {
				return Err(anyhow!("conv mixer on a layer without conv windows"));
			}
		};
		match (c.get(i), n.get(i)) {
			(Some(r), Some(w)) => return Ok((r, w)),
			_missing => {
				return Err(anyhow!(
					"conv {i} has no cache window (layer_cache_shape mismatch)"
				));
			}
		}
	}
}

struct KvCache {
	layers: Vec<LayerCache>,
	kw: Vec<usize>,
	vw: Vec<usize>,
	conv_sz: Vec<Vec<usize>>,
	rec_sz: Vec<usize>,
	win: usize,
	win_base: usize,
	stage: KvStage,
	len: usize,
	ids: Vec<u32>,
	t_max: usize,
}

pub(crate) struct KvStage {
	pub(crate) sk: [GpuBuffer; 2],
	pub(crate) sv: [GpuBuffer; 2],
}

pub struct HostStore {
	pub ram: Vec<u8>,
	ram_budget: usize,
	wf: Option<fs::File>,
	rf: RefCell<Option<fs::File>>,
	path: PathBuf,
	pub file_len: usize,
}

impl HostStore {
	pub fn new(ram_budget: usize, path: PathBuf) -> HostStore {
		return HostStore {
			ram: Vec::new(),
			ram_budget,
			wf: None,
			rf: RefCell::new(None),
			path,
			file_len: 0,
		};
	}

	pub fn len(&self) -> usize {
		return self.ram.len() + self.file_len;
	}

	pub fn append_f32(&mut self, vals: &[f32]) -> Result<()> {
		let mut bytes = Vec::with_capacity(vals.len() * 4);
		for v in vals {
			bytes.extend_from_slice(&v.to_le_bytes());
		}
		if self.wf.is_none() && self.ram.len() + bytes.len() <= self.ram_budget {
			self.ram.extend_from_slice(&bytes);
			return Ok(());
		}
		if self.wf.is_none() {
			let f = fs::OpenOptions::new()
				.write(true)
				.create(true)
				.truncate(true)
				.open(&self.path)
				.with_context(|| format!("KV spill file {}", self.path.display()))?;
			self.wf = Some(f);
		}
		let Some(f) = &self.wf else {
			bail!("KV spill file {} closed mid-append", self.path.display());
		};
		f.write_all_at(&bytes, self.file_len as u64)
			.with_context(|| {
				format!(
					"KV spill write at {} in {}",
					self.file_len,
					self.path.display()
				)
			})?;
		self.file_len += bytes.len();
		return Ok(());
	}

	pub fn stage_into(&self, off: usize, len: usize, dst: &GpuBuffer, scratch: &mut Vec<u8>) -> Result<()> {
		anyhow::ensure!(
			off + len <= self.len(),
			"KV host store: staging [{off}, {}) past the {} stored bytes",
			off + len,
			self.len()
		);
		let es = FWD_DT.elem_size();
		let ram_n = self.ram.len().saturating_sub(off).min(len);
		if ram_n > 0 {
			dst.write_u8(&self.ram[off..off + ram_n])?;
		}
		let file_n = len - ram_n;
		if file_n > 0 {
			let mut rf = self.rf.borrow_mut();
			if rf.is_none() {
				*rf = Some(File::open(&self.path)
					.with_context(|| format!("KV spill read-open {}", self.path.display()))?);
			}
			let Some(f) = rf.as_ref() else {
				bail!(
					"KV host store: {} file bytes requested but no spill file",
					file_n
				);
			};
			let foff = (off + ram_n - self.ram.len()) as u64;
			scratch.resize(file_n, 0);
			f.read_exact_at(scratch, foff).with_context(|| {
				format!(
					"KV spill read [{foff}, +{file_n}) from {}",
					self.path.display()
				)
			})?;
			dst.view(ram_n / es, file_n / es).write_u8(&scratch[..])?;
		}
		return Ok(());
	}

	pub fn truncate(&mut self, len: usize) {
		if len <= self.ram.len() {
			self.ram.truncate(len);
			*self.rf.borrow_mut() = None;
			if self.wf.take().is_some() {
				let _ = fs::remove_file(&self.path);
			}
			self.file_len = 0;
		} else {
			self.file_len = len - self.ram.len();
		}
	}
}

impl Drop for HostStore {
	fn drop(&mut self) {
		if self.wf.take().is_some() {
			let _ = fs::remove_file(&self.path);
		}
	}
}

impl KvCache {
	fn new(m: &Model, win: usize, host_cap: usize) -> Result<KvCache> {
		let nl = m.hp.nl;
		let es = FWD_DT.elem_size();
		let shapes: Vec<models::LayerCacheShape> = (0..nl)
			.map(|l| return models::layer_cache_shape(m, l))
			.collect();
		let kw: Vec<usize> = shapes.iter().map(|s| return s.kw).collect();
		let vw: Vec<usize> = shapes.iter().map(|s| return s.vw).collect();
		let rec_sz: Vec<usize> = shapes.iter().map(|s| return s.rec).collect();
		let conv_sz: Vec<Vec<usize>> = shapes.iter().map(|s| return s.conv.clone()).collect();
		let total_w: usize = kw.iter().sum::<usize>() + vw.iter().sum::<usize>();
		let (ram_budget, spill_dir) = if host_cap > 0 && total_w > 0 {
			let dir = kv_spill_dir()?;
			(gpu_core::tiered::Budgets::measure(0, 0, &dir).ram_data, dir)
		} else {
			(0, PathBuf::new())
		};
		let sid = KV_SPILL_SEQ.fetch_add(1, Ordering::Relaxed);
		let store = |w: usize, name: &str, l: usize| -> HostStore {
			let budget = if total_w > 0 {
				ram_budget / total_w * w
			} else {
				0
			};
			let path = spill_dir.join(format!(
				"recipe-kv-{}-{sid}-l{l}-{name}.spill",
				process::id()
			));
			return HostStore::new(budget, path);
		};
		let scan_slot = |s: &models::LayerCacheShape| -> Result<ScanSlot> {
			let rec = falloc(s.rec.max(1))?;
			rec.memset_zero(s.rec.max(1) * es)?;
			let mut conv = Vec::with_capacity(s.conv.len());
			let mut nxt = Vec::with_capacity(s.conv.len());
			for &w in &s.conv {
				let a = falloc(w.max(1))?;
				a.memset_zero(w.max(1) * es)?;
				conv.push(a);
				let b = falloc(w.max(1))?;
				b.memset_zero(w.max(1) * es)?;
				nxt.push(b);
			}
			return Ok(ScanSlot { rec, conv, nxt });
		};
		let mut layers = Vec::with_capacity(nl);
		for (l, s) in shapes.iter().enumerate() {
			let kv_slot = |_probe: ()| -> Result<KvSlot> {
				return Ok(KvSlot {
					k: falloc(win * s.kw.max(1))?,
					v: falloc(win * s.vw.max(1))?,
					hk: store(s.kw, "k", l),
					hv: store(s.vw, "v", l),
				});
			};
			let layer = match (s.kw > 0, s.rec > 0, !s.conv.is_empty()) {
				(true, false, false) => LayerCache::Kv(kv_slot(())?),
				(true, _rec, _conv) => LayerCache::KvScan(kv_slot(())?, scan_slot(s)?),
				(false, true, _conv) => LayerCache::Scan(scan_slot(s)?),
				(false, false, true) => {
					let sc = scan_slot(s)?;
					LayerCache::Conv(ConvSlot {
						conv: sc.conv,
						nxt: sc.nxt,
					})
				}
				(false, false, false) => LayerCache::Kv(kv_slot(())?),
			};
			layers.push(layer);
		}
		let skw = kw.iter().copied().max().unwrap_or(0);
		let svw = vw.iter().copied().max().unwrap_or(0);
		let stage = KvStage {
			sk: [falloc(win * skw.max(1))?, falloc(win * skw.max(1))?],
			sv: [falloc(win * svw.max(1))?, falloc(win * svw.max(1))?],
		};
		return Ok(KvCache {
			layers,
			kw,
			vw,
			conv_sz,
			rec_sz,
			win,
			win_base: 0,
			stage,
			len: 0,
			ids: Vec::with_capacity(win + host_cap),
			t_max: win + host_cap,
		});
	}

	fn rewind(&mut self, keep: usize) {
		self.len = keep.min(self.len);
		self.ids.truncate(self.len);
		if self.len < self.win_base {
			let es = FWD_DT.elem_size();
			let (len, kw, vw) = (self.len, &self.kw, &self.vw);
			for (l, layer) in self.layers.iter_mut().enumerate() {
				let (LayerCache::Kv(s) | LayerCache::KvScan(s, _)) = layer else {
					continue;
				};
				s.hk.truncate(len * kw[l] * es);
				s.hv.truncate(len * vw[l] * es);
			}
			self.win_base = self.len;
		}
	}

	fn clear_scan(&mut self) -> Result<()> {
		let es = FWD_DT.elem_size();
		let zero_scan = |s: &ScanSlot, sz: usize, szs: &[usize]| -> Result<()> {
			s.rec.memset_zero(sz.max(1) * es)?;
			for (b, &w) in
				s.conv.iter()
					.chain(s.nxt.iter())
					.zip(szs.iter().chain(szs.iter()))
			{
				b.memset_zero(w.max(1) * es)?;
			}
			return Ok(());
		};
		for (l, layer) in self.layers.iter_mut().enumerate() {
			match layer {
				LayerCache::Scan(s) => zero_scan(s, self.rec_sz[l], &self.conv_sz[l])?,
				LayerCache::KvScan(kv, s) => {
					zero_scan(s, self.rec_sz[l], &self.conv_sz[l])?;
					kv.hk.truncate(0);
					kv.hv.truncate(0);
				}
				LayerCache::Conv(s) => {
					for (b, &w) in
						s.conv.iter()
							.chain(s.nxt.iter())
							.zip(self.conv_sz[l].iter().chain(self.conv_sz[l].iter()))
					{
						b.memset_zero(w.max(1) * es)?;
					}
				}
				LayerCache::Kv(s) => {
					s.hk.truncate(0);
					s.hv.truncate(0);
				}
			}
		}
		self.len = 0;
		self.ids.clear();
		self.win_base = 0;
		return Ok(());
	}

	fn ensure_room(&mut self, t: usize) -> Result<bool> {
		if (self.len - self.win_base) + t <= self.win {
			return Ok(false);
		}
		let resident = self.len - self.win_base;
		let (kw, vw) = (&self.kw, &self.vw);
		for (l, layer) in self.layers.iter_mut().enumerate() {
			let (LayerCache::Kv(s) | LayerCache::KvScan(s, _)) = layer else {
				continue;
			};
			if kw[l] == 0 {
				continue;
			}
			let mut kf = vec![0.0f32; resident * kw[l]];
			let mut vf = vec![0.0f32; resident * vw[l]];
			s.k.download_f32(&mut kf)?;
			s.v.download_f32(&mut vf)?;
			s.hk.append_f32(&kf)?;
			s.hv.append_f32(&vf)?;
		}
		self.win_base = self.len;
		return Ok(true);
	}
}

fn pick_greedy(logits: &[f64], vocab: usize, lsc: f64, softcap: f64) -> u32 {
	let mut best = 0usize;
	let mut bv = f64::MIN;
	for (i, &v) in logits.iter().take(vocab).enumerate() {
		let vs = v * lsc;
		let sv = if softcap > 0.0 {
			softcap * (vs / softcap).tanh()
		} else {
			vs
		};
		if sv > bv {
			bv = sv;
			best = i;
		}
	}
	return best as u32;
}

pub struct Sampler {
	temp: f64,
	top_k: usize,
	top_p: f64,
	rng: StdRng,
	shaped: Vec<f64>,
	order: Vec<u32>,
}

impl Sampler {
	fn fresh() -> Sampler {
		let seed = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_or(0, |d| return d.as_nanos() as u64);
		return Sampler {
			temp: 0.8,
			top_k: 40,
			top_p: 0.95,
			rng: StdRng::seed_from_u64(seed),
			shaped: Vec::new(),
			order: Vec::new(),
		};
	}

	fn pick(&mut self, logits: &[f64], vocab: usize, lsc: f64, softcap: f64) -> u32 {
		if self.temp <= 0.0 {
			return pick_greedy(logits, vocab, lsc, softcap);
		}
		let n = vocab.min(logits.len());
		self.shaped.clear();
		self.shaped.extend(logits.iter().take(n).map(|&v| {
			let vs = v * lsc;
			let sv = if softcap > 0.0 {
				softcap * (vs / softcap).tanh()
			} else {
				vs
			};
			return sv / self.temp;
		}));
		self.order.clear();
		self.order.extend(0..n as u32);
		let k = match self.top_k {
			0 => n,
			k => k.min(n),
		};
		let shaped = &self.shaped;
		self.order.select_nth_unstable_by(k - 1, |&a, &b| {
			return shaped[b as usize].total_cmp(&shaped[a as usize]);
		});
		self.order.truncate(k);
		self.order.sort_unstable_by(|&a, &b| {
			return shaped[b as usize].total_cmp(&shaped[a as usize]);
		});
		let top = shaped[self.order[0] as usize];
		let z: f64 = self
			.order
			.iter()
			.map(|&i| return (shaped[i as usize] - top).exp())
			.sum();
		let mut kept = self.order.len();
		let mut cum = 0.0;
		for (j, &i) in self.order.iter().enumerate() {
			cum += (shaped[i as usize] - top).exp() / z;
			if cum >= self.top_p {
				kept = j + 1;
				break;
			}
		}
		self.order.truncate(kept);
		let zk: f64 = self
			.order
			.iter()
			.map(|&i| return (shaped[i as usize] - top).exp())
			.sum();
		let mut draw = self.rng.random::<f64>() * zk;
		for &i in &self.order {
			draw -= (shaped[i as usize] - top).exp();
			if draw <= 0.0 {
				return i;
			}
		}
		return self.order[kept - 1];
	}
}

fn forward_rows(
	m: &Model,
	rows: &[u32],
	attn_scale: &GpuBuffer,
	ar: &Arena,
	cache: &mut KvCache,
	logits: &mut [f64],
	lm_scratch: &mut [f64],
	emb_scratch: &mut Vec<u8>,
) -> Result<()> {
	let ne = m.hp.ne;
	let nl = m.hp.nl;
	let t = rows.len();
	let base_off = cache.len;
	let scale = models::embedding_scale(m);
	let (src, dt) = (&m.emb.bytes, m.emb.dt);
	let rb = bytes_for(ne, dt);
	emb_scratch.clear();
	for &tk in rows {
		let b = tk as usize * rb;
		emb_scratch.extend_from_slice(&src[b..b + rb]);
	}
	let h0 = ar.ha.view(0, t * ne);
	let stage = ar.x.view(0, t * ne).as_dtype(dt);
	stage.write_u8(emb_scratch)?;
	gpu_convert(&stage, &h0, t * ne, scale)?;
	if let Some(pos) = &m.pos {
		let rbp = bytes_for(ne, pos.dt);
		let row = base_off * rbp;
		let pstage = ar.x.view(0, t * ne).as_dtype(pos.dt);
		pstage.write_u8(&pos.bytes[row..row + t * rbp])?;
		let pe = ar.attn_out.view(0, t * ne);
		gpu_convert(&pstage, &pe, t * ne, 1.0)?;
		gpu_add_into(&pe, &h0, t * ne, &h0)?;
	}
	if let Some((g, b)) = &m.embed_norm {
		gpu_layernorm_into(&h0, g, b, &m.eps, t, ne, &h0)?;
	}
	if models::embd_skip(m) {
		gpu_rmsnorm_f64_nogamma(&h0, &m.eps, t, ne, &h0)?;
		gpu_copy_into(&h0, t * ne, &ar.embd_skip)?;
	}
	cache.ensure_room(t)?;
	let mut src: &GpuBuffer = &ar.ha;
	let mut dst: &GpuBuffer = &ar.hb;
	for l in 0..nl {
		let dec = models::DecCtx {
			cached: cache.len,
			win_base: cache.win_base,
			win: cache.win,
			state: &cache.layers[l],
			stage: &cache.stage,
		};
		models::dispatch(m, l, src, dst, t, ar, attn_scale, &dec)?;
		mem::swap(&mut src, &mut dst);
	}
	for layer in cache.layers.iter_mut() {
		match layer {
			LayerCache::Scan(s) | LayerCache::KvScan(_, s) => mem::swap(&mut s.conv, &mut s.nxt),
			LayerCache::Conv(s) => mem::swap(&mut s.conv, &mut s.nxt),
			LayerCache::Kv(_) => {}
		}
	}
	let last_h = src.view((t - 1) * ne, ne);
	models::decoder_norm(m, &last_h, 1, ne, &ar.hfs)?;
	lm_head_into(m, &ar.hfs, 1, ar, logits, lm_scratch)?;
	cache.len += t;
	cache.ids.extend_from_slice(rows);
	KV_RAN.store(true, Ordering::Relaxed);
	return Ok(());
}

static KV_RAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[must_use]
pub fn kv_cache_ran() -> bool {
	return KV_RAN.load(Ordering::Relaxed);
}

fn decode_tokens(tokenizer: &crate::tokenizer::Tokenizer, ids: &[u32]) -> Result<String> {
	return tokenizer
		.decode(ids, true)
		.map_err(|e| return anyhow!("decode {} tokens: {e}", ids.len()));
}

fn decode_cached(
	m: &Model,
	tokenizer: &crate::tokenizer::Tokenizer,
	toks: &[u32],
	attn_scale: &GpuBuffer,
	cache: &mut KvCache,
	smp: &mut Sampler,
	on_round: &mut dyn FnMut(&[Tok]) -> bool,
) -> Result<String> {
	let vocab_size = m.hp.vocab;
	let eos = m.hp.eos;
	let softcap = models::final_softcap(m);
	let lsc = models::logit_scale(m);
	let max_new = 256usize;
	let per_row = kv_row_elems(m) * FWD_DT.elem_size();
	if toks.len() > cache.t_max {
		bail!(
			"KV cache: sequence of {} tokens ({}) exceeds the VRAM+RAM+disk waterfall ceiling of {} tokens ({}) — all tiers exhausted (arch {})",
			toks.len(),
			crate::human_bytes(toks.len() * per_row),
			cache.t_max,
			crate::human_bytes(cache.t_max * per_row),
			m.hp.arch
		);
	}
	let p_new = toks.len() - cache.len;
	let chunk = cache.win.max(1);
	let aw = p_new.min(chunk).max(1);
	let ar = {
		let _t = gpu_core::memory::tag_scope("arena");
		Arena::new(&m.hp, aw.max(1))?
	};
	let mut logits = vec![0.0f64; m.hp.vocab];
	let mut lm_scratch = vec![0.0f64; m.hp.lm_chunk];
	let mut emb_scratch: Vec<u8> = Vec::new();
	let decode_start = Instant::now();
	if cache.len == 0 {
		for c in toks.chunks(chunk) {
			forward_rows(
				m,
				c,
				attn_scale,
				&ar,
				cache,
				&mut logits,
				&mut lm_scratch,
				&mut emb_scratch,
			)?;
		}
	} else {
		let suffix = toks[cache.len..].to_vec();
		for c in suffix.chunks(chunk) {
			forward_rows(
				m,
				c,
				attn_scale,
				&ar,
				cache,
				&mut logits,
				&mut lm_scratch,
				&mut emb_scratch,
			)?;
		}
	}
	let mut out_ids: Vec<u32> = Vec::new();
	let mut next = smp.pick(&logits, vocab_size, lsc, softcap);
	loop {
		if next == eos || m.hp.eog.contains(&next) {
			break;
		}
		out_ids.push(next);
		let text = decode_tokens(tokenizer, &out_ids)?;
		let keep_going = on_round(&[Tok {
			text,
			status: TokStatus::Accepted,
			age: 0,
			heat: 1.0,
		}]);
		if !keep_going || out_ids.len() >= max_new || cache.len >= cache.t_max {
			break;
		}
		forward_rows(
			m,
			&[next],
			attn_scale,
			&ar,
			cache,
			&mut logits,
			&mut lm_scratch,
			&mut emb_scratch,
		)?;
		next = smp.pick(&logits, vocab_size, lsc, softcap);
	}
	let elapsed = decode_start.elapsed();
	let body = decode_tokens(tokenizer, &out_ids)?;
	let out = format!(
		"{body}\n\n{} tokens, {:.2} tok/s",
		out_ids.len(),
		out_ids.len() as f64 / elapsed.as_secs_f64().max(1e-9)
	);
	drop(ar);
	return Ok(out);
}

pub fn vram_probe_ask() -> Result<usize> {
	crate::init().map_err(|e| return anyhow!("gpu init: {e:?}"))?;
	return Ok(gpu_core::memory::vram_free_base());
}

fn probe_claim() -> Result<Waterfall> {
	let want = gpu_core::memory::vram_free_base().saturating_sub(gpu_core::memory::USER_GB) & !((1 << 21) - 1);
	if want < (1 << 30) {
		bail!("claim probe: nothing mappable above 1 GB");
	}
	Write::line(
		gpu,
		format!(
			"claim: {:.2} GB (measured)",
			want as f64 / (1u64 << 30) as f64
		),
	);
	let slab = gpu_core::memory::claim_device_arena_bytes(want).context("claim device arena")?;
	let w = Waterfall::from_arena(slab);
	Write::line(
		gpu,
		format!("[right after claim] {}", gpu_core::memory::ledger_report()),
	);
	return Ok(w);
}

pub struct ChatSession {
	m: Model,
	tokenizer: crate::tokenizer::Tokenizer,
	vocab: Vec<String>,
	attn_scale: GpuBuffer,
	cache: KvCache,
	sampler: Sampler,
}

impl ChatSession {
	#[must_use]
	pub fn temp(mut self, t: f64) -> ChatSession {
		self.sampler.temp = t;
		return self;
	}

	#[must_use]
	pub fn top_k(mut self, k: usize) -> ChatSession {
		self.sampler.top_k = k;
		return self;
	}

	#[must_use]
	pub fn top_p(mut self, p: f64) -> ChatSession {
		self.sampler.top_p = p;
		return self;
	}
}

fn kv_capacity_tokens(m: &Model) -> Result<usize> {
	let es = FWD_DT.elem_size();
	let cache_row = kv_row_elems(m) * es;
	let fixed_cache = fixed_cache_elems(m) * es;
	let stage_row = {
		let mut skw = 0usize;
		let mut svw = 0usize;
		for l in 0..m.hp.nl {
			let s = models::layer_cache_shape(m, l);
			skw = skw.max(s.kw);
			svw = svw.max(s.vw);
		}
		2 * (skw + svw) * es
	};
	let avail = gpu_core::memory::arena_remaining();
	let cost1 = {
		let _probe = Arena::new(&m.hp, 1)?;
		avail.saturating_sub(gpu_core::memory::arena_remaining())
	};
	let cost2 = {
		let _probe = Arena::new(&m.hp, 2)?;
		avail.saturating_sub(gpu_core::memory::arena_remaining())
	};
	let arena_row = cost2.saturating_sub(cost1);
	let arena_fixed = cost1.saturating_sub(arena_row);
	let per_token = arena_row + cache_row + stage_row;
	if per_token == 0 {
		return Ok(0);
	}
	let free = gpu_core::memory::arena_remaining().saturating_sub(arena_fixed + fixed_cache);
	return Ok(free / per_token);
}

fn kv_row_elems(m: &Model) -> usize {
	let mut n = 0usize;
	for l in 0..m.hp.nl {
		let s = models::layer_cache_shape(m, l);
		n += s.kw + s.vw;
	}
	return n;
}

fn fixed_cache_elems(m: &Model) -> usize {
	let mut n = 0usize;
	for l in 0..m.hp.nl {
		let s = models::layer_cache_shape(m, l);
		n += s.rec;
		let conv: usize = s.conv.iter().map(|&w| w.max(1)).sum();
		n += 2 * conv;
	}
	return n;
}

fn kv_spill_dir() -> Result<PathBuf> {
	return gpu_core::tiered::data_dir().context("KV spill dir");
}

fn kv_host_tokens(m: &Model) -> Result<usize> {
	let per_row = kv_row_elems(m) * FWD_DT.elem_size();
	if per_row == 0 {
		return Ok(0);
	}
	let b = gpu_core::tiered::Budgets::measure(0, 0, &kv_spill_dir()?);
	return Ok((b.ram_data + b.disk_data) / per_row);
}

pub enum Opened {
	Session(Box<ChatSession>),
	Cancelled,
}

impl Opened {
	pub fn session(self) -> Result<ChatSession> {
		match self {
			Opened::Session(s) => return Ok(*s),
			Opened::Cancelled => return Err(anyhow!("model load cancelled")),
		}
	}
}

impl ChatSession {
	pub fn open(gguf: &Path, load_round: &mut dyn FnMut(&[Tok]) -> bool) -> Result<Opened> {
		crate::init().map_err(|e| anyhow!("gpu init: {e:?}"))?;
		let t_load = Instant::now();
		let watchdog = arm_watchdog();
		let claim = probe_claim()?;
		beat();
		let mut m = load_model_gguf(gguf)?;
		let (tokenizer, vocab) = {
			let g = Gguf::open(gguf)?;
			(
				crate::tokenizer::from_gguf(&g)?,
				crate::tokenizer::gguf_vocab(&g, m.hp.vocab)?,
			)
		};
		let attn_scale = {
			let hd = m.hp.dims.first().map_or(m.hp.key_length, |d| d.hd);
			let ub = falloc(1)?;
			ub.load(&[1.0 / (hd as f64).sqrt()])?;
			ub
		};
		if !fill_store(&mut m, claim, &mut || !load_round(&[]))? {
			return Ok(Opened::Cancelled);
		}
		watchdog.disarm();
		let cache = if m.hp.ncanvas == 0 {
			let win = kv_capacity_tokens(&m)?;
			if win == 0 {
				bail!(
					"KV cache: not one token fits — {} bytes free in the claim, {} bytes per token ({} layers)",
					gpu_core::memory::arena_remaining(),
					kv_row_elems(&m) * FWD_DT.elem_size(),
					m.hp.nl
				);
			}
			let host = kv_host_tokens(&m)?;
			let _t = gpu_core::memory::tag_scope("kvcache");
			KvCache::new(&m, win, host)?
		} else {
			let win = kv_capacity_tokens(&m)?.max(1);
			let _t = gpu_core::memory::tag_scope("kvcache");
			KvCache::new(&m, win, 0)?
		};
		Write::line(
			gpu,
			format!(
				"loaded in {:.1}s (arch={} nl={} ne={} experts={} canvas={} vocab={} softcap={} kvcache_rows={})",
				t_load.elapsed().as_secs_f64(),
				m.hp.arch,
				m.hp.nl,
				m.hp.ne,
				m.hp.nexp,
				m.hp.ncanvas,
				m.hp.vocab,
				m.hp.softcap,
				cache.t_max,
			),
		);
		return Ok(Opened::Session(Box::new(ChatSession {
			m,
			tokenizer,
			vocab,
			attn_scale,
			cache,
			sampler: Sampler::fresh(),
		})));
	}

	pub fn generate_in(&mut self, prompt: &str, on_round: &mut dyn FnMut(&[Tok]) -> bool) -> Result<String> {
		let ncanvas = self.m.hp.ncanvas;
		let mask = self.m.hp.mask;
		let enc = self
			.tokenizer
			.encode(prompt, false)
			.map_err(|e| anyhow!("tokenize: {e}"))?;
		let mut toks = vec![self.m.hp.bos];
		toks.extend_from_slice(enc.get_ids());
		let prefix = toks.len();
		for _ in 0..ncanvas {
			toks.push(mask as u32);
		}
		let t = toks.len();
		Write::line(
			data,
			format!("prompt tokens={prefix} canvas={ncanvas} total={t}"),
		);
		if ncanvas == 0 {
			let cache = &mut self.cache;
			let mut keep = 0usize;
			while keep < cache.ids.len() && keep < toks.len() && cache.ids[keep] == toks[keep] {
				keep += 1;
			}
			let keep = keep.min(toks.len().saturating_sub(1));
			if models::arch_has_recurrence(&self.m.hp.arch) && keep < cache.len {
				cache.clear_scan()?;
			} else {
				cache.rewind(keep);
			}
			Write::line(
				data,
				format!(
					"kvcache reuse: cached_prefix={} new_suffix={}",
					cache.len,
					toks.len() - cache.len
				),
			);
			return decode_cached(
				&self.m,
				&self.tokenizer,
				&toks,
				&self.attn_scale,
				cache,
				&mut self.sampler,
				on_round,
			);
		}
		let cache = &mut self.cache;
		return refine_canvas(
			&self.m,
			&self.vocab,
			toks,
			prefix,
			&self.attn_scale,
			cache,
			on_round,
		);
	}
}

impl Drop for ChatSession {
	fn drop(&mut self) {
		if let Some(slab) = self.m.store.take_slab() {
			gpu_core::memory::release_device_arena(slab);
		}
	}
}

pub fn generate(gguf: &Path, prompt: &str, on_round: &mut dyn FnMut(&[Tok]) -> bool) -> Result<String> {
	let mut session = match ChatSession::open(gguf, on_round)? {
		Opened::Session(s) => *s,
		Opened::Cancelled => return Ok(String::new()),
	};
	return session.generate_in(prompt, on_round);
}

fn refine_canvas(
	m: &Model,
	vocab: &[String],
	toks: Vec<u32>,
	prefix: usize,
	attn_scale: &GpuBuffer,
	cache: &mut KvCache,
	on_round: &mut dyn FnMut(&[Tok]) -> bool,
) -> Result<String> {
	let ne = m.hp.ne;
	let nff = m.hp.nff;
	let ncanvas = m.hp.ncanvas;
	let nl = m.hp.nl;
	let mask = m.hp.mask;
	let vocab_size = m.hp.vocab;
	let mask_signal = m.hp.mask_signal;
	let t = toks.len();
	let scl = (ne as f64).sqrt();

	let ar = {
		let _t = gpu_core::memory::tag_scope("arena");
		Arena::new(&m.hp, t)?
	};
	Write::line(data, "preflight gemms");
	preflight(m, &ar, t)?;

	let allocs_before = gpu_core::memory::device_alloc_count();
	let t0 = Instant::now();

	let mut sck: Vec<Vec<(usize, f64)>> = vec![vec![]; ncanvas];
	let mut pred = vec![mask as u32; ncanvas];
	let mut prev = pred.clone();
	let mut ages = vec![0u8; ncanvas];
	let mut heats = vec![0f32; ncanvas];

	let mut picks: Vec<(usize, f64)> = Vec::with_capacity(t.max(ncanvas * BLEND_K));
	let mut emb_scratch: Vec<u8> = Vec::new();
	let mut emb_wts: Vec<f64> = Vec::new();

	for step in 0..6 {
		picks.clear();
		picks.extend(toks.iter().map(|&tk| return (tk as usize, 1.0)));
		embed_blend_into(
			m,
			&ar,
			&picks,
			t,
			1,
			scl,
			&ar.ha,
			&mut emb_scratch,
			&mut emb_wts,
		)?;

		let coff = prefix * ne;
		let clen = ncanvas * ne;
		if step > 0 {
			picks.clear();
			for top in &sck {
				let last = top.last().copied().unwrap_or((mask, 0.0));
				for j in 0..BLEND_K {
					picks.push(top.get(j).copied().unwrap_or((last.0, 0.0)));
				}
			}
			embed_blend_into(
				m,
				&ar,
				&picks,
				ncanvas,
				BLEND_K,
				scl,
				&ar.soft,
				&mut emb_scratch,
				&mut emb_wts,
			)?;
			gpu_rmsnorm_f64(&ar.soft, &m.sc_pre, &m.eps, ncanvas, ne, &ar.scn)?;
			gpu_gemm_bt(&ar.scn, &m.sc_gate, ncanvas, nff, ne, &ar.sg)?;
			gpu_gemm_bt(&ar.scn, &m.sc_up, ncanvas, nff, ne, &ar.su)?;
			gpu_gelu_mul(&ar.sg, &ar.su, ncanvas * nff, &ar.sa)?;
			gpu_gemm_bt(&ar.sa, &m.sc_down, ncanvas, ne, nff, &ar.sc_add)?;
			gpu_add_into(&ar.ha.view(coff, clen), &ar.sc_add, clen, &ar.cur)?;
			gpu_rmsnorm_f64_nogamma(&ar.cur, &m.eps, ncanvas, ne, &ar.normed)?;
		} else {
			gpu_rmsnorm_f64_nogamma(&ar.ha.view(coff, clen), &m.eps, ncanvas, ne, &ar.normed)?;
		}
		ar.ha.view(coff, clen)
			.copy_from(&ar.normed, clen * FWD_DT.elem_size())?;

		let bithash = |b: &GpuBuffer, n: usize| -> Result<u64> {
			let mut v = vec![0.0f64; n];
			b.view(0, n).download_host(&mut v)?;
			gpu_core::hip::device_synchronize()?;
			Ok(v.iter().fold(0xcbf29ce484222325u64, |h, x| {
				(h ^ x.to_bits()).wrapping_mul(0x100000001b3)
			}))
		};
		if step == 0 && ogdl::log::opt().probe {
			Write::line(
				probe_flag,
				format!("[hash] step0 input {:016x}", bithash(&ar.ha, t * ne)?),
			);
		}
		cache.rewind(0);
		cache.ensure_room(t)?;
		let mut src: &GpuBuffer = &ar.ha;
		let mut dst: &GpuBuffer = &ar.hb;
		for l in 0..nl {
			Write::line(
				gpu,
				format!(
					"step {step} layer {}/{} ({:.0}s)",
					l + 1,
					nl,
					t0.elapsed().as_secs_f64()
				),
			);
			let dec = models::DecCtx {
				cached: cache.len,
				win_base: cache.win_base,
				win: cache.win,
				state: &cache.layers[l],
				stage: &cache.stage,
			};
			models::dispatch(m, l, src, dst, t, &ar, attn_scale, &dec)?;
			mem::swap(&mut src, &mut dst);
			if step == 0 && ogdl::log::opt().probe {
				Write::line(
					probe_flag,
					format!("[hash] step0 layer {l:2} {:016x}", bithash(src, t * ne)?),
				);
			}
		}
		let hbuf = src;
		gpu_core::math_ops::gpu_isfinite_all(hbuf, t * ne, &ar.finite)
			.map_err(|e| return anyhow!("isfinite_all: {e:?}"))?;
		let mut flag = [0u8; 4];
		ar.finite.download_u8(&mut flag)?;
		if i32::from_le_bytes(flag) == 0 {
			Write::err(format!("step {step}: non-finite in h after layers"))?;
		}

		gpu_rmsnorm_f64(
			&hbuf.view(coff, clen),
			&m.decoder_norm,
			&m.eps,
			ncanvas,
			ne,
			&ar.hfs,
		)?;
		let logits = lm_head(m, &ar.hfs, ncanvas, &ar)?;

		let temp = 1.0 - 0.7 * (step as f64 / 6.0);
		for c in 0..ncanvas {
			let row = &logits[c * vocab_size..(c + 1) * vocab_size];
			let mut cand: Vec<(usize, f64)> = (0..vocab_size)
				.filter(|&tk| tk >= 6 && Some(tk) != mask_signal && !vocab[tk].starts_with('<'))
				.map(|tk| (tk, row[tk]))
				.collect();
			cand.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(cmp::Ordering::Equal));
			cand.truncate(50);
			let ml = cand[0].1;
			let mut probs: Vec<f64> = cand.iter().map(|&(_, l)| ((l - ml) / temp).exp()).collect();
			let sum: f64 = probs.iter().sum();
			for x in probs.iter_mut() {
				*x /= sum;
			}
			let mut st = ((step as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15))
				^ ((c as u64 + 1).wrapping_mul(0x85EB_CA77_C2B2_AE63));
			st |= 1;
			let r = xs(&mut st);
			let mut cum = 0.0;
			let mut sel = cand[0].0;
			let mut selp = probs[0];
			for (k, &pr) in probs.iter().enumerate() {
				cum += pr;
				if r <= cum {
					sel = cand[k].0;
					selp = pr;
					break;
				}
			}
			pred[c] = sel as u32;
			heats[c] = selp as f32;
			let mut top: Vec<(usize, f64)> = cand
				.iter()
				.zip(probs.iter())
				.take(8)
				.map(|(&(id, _), &pr)| (id, pr))
				.collect();
			let s8: f64 = top.iter().map(|&(_, pr)| pr).sum();
			for e in top.iter_mut() {
				e.1 /= s8;
			}
			sck[c] = top;
		}

		let toks_ui: Vec<Tok> = (0..ncanvas)
			.map(|c| {
				let mut tk = pred[c] as usize;
				let undecided = tk == mask || Some(tk) == mask_signal;
				if undecided
					&& let Some(&(best, _p)) = sck[c]
						.iter()
						.find(|&&(id, _p)| id != mask && Some(id) != mask_signal)
				{
					tk = best;
				}
				let status = if undecided {
					TokStatus::Draft
				} else if pred[c] != prev[c] {
					TokStatus::Recent
				} else {
					TokStatus::Accepted
				};
				ages[c] = if pred[c] != prev[c] {
					0
				} else {
					ages[c].saturating_add(1)
				};
				Tok {
					text: vocab[tk].replace('\u{2581}', " "),
					status,
					age: ages[c],
					heat: heats[c],
				}
			})
			.collect();
		prev = pred.clone();
		if !on_round(&toks_ui) {
			break;
		}
	}

	let allocs_after = gpu_core::memory::device_alloc_count();
	Write::line(
		gpu,
		format!("steady-state allocs: {}", allocs_after - allocs_before),
	);
	let tot = t0.elapsed().as_secs_f64();
	let s = |a: &AtomicU64| a.load(Ordering::Relaxed) as f64 / 1e9;
	Write::line(
		gpu,
		format!(
			"[breakdown] total={tot:.1}s  disk={:.1}s  h2d(write_u8)={:.1}s  widen(launch)={:.1}s",
			s(&DISK_NS),
			s(&H2D_NS),
			s(&WIDEN_NS),
		),
	);
	Write::line(
		gpu,
		format!(
			"[sections]  attn={:.1}s  mlp={:.1}s  moe={:.1}s (route={:.1}s roundtrips={:.1}s)  lm_head={:.1}s",
			s(&ATTN_NS),
			s(&MLP_NS),
			s(&MOE_NS),
			s(&ROUTE_NS),
			s(&MOE_RT_NS),
			s(&LM_NS),
		),
	);
	Write::line(
		gpu,
		format!(
			"[experts]   from VRAM={}  from RAM={}  from DISK={}",
			E_VRAM.load(Ordering::Relaxed),
			E_RAM.load(Ordering::Relaxed),
			E_DISK.load(Ordering::Relaxed),
		),
	);
	m.store.report();

	let out: String = pred
		.iter()
		.map(|&tk| tk as usize)
		.filter(|&tk| tk != mask && Some(tk) != mask_signal)
		.map(|tk| vocab[tk].replace('\u{2581}', " "))
		.collect();

	drop(ar);
	Ok(out)
}
