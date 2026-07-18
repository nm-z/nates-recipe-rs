use crate::gguf::{Gguf, Val};
use anyhow::{Context, Result, anyhow, bail};
use gpu_core::infer_ops::{
	gpu_flash_gqa, gpu_gelu_mul, gpu_gemm_bt_f64, gpu_glu_gelu, gpu_rmsnorm_f64,
	gpu_rmsnorm_f64_nogamma, gpu_rope_partial, gpu_scale_f64_inplace, gpu_widen_bf16,
};
use gpu_core::kernels::{gpu_add_into, gpu_copy_into, gpu_layernorm_into};
use gpu_core::log::probe as probe_flag;
use gpu_core::log::{Write, data, gpu};
use gpu_core::memory::{Dtype, GpuBuffer};
use gpu_core::waterfall::{Home, Waterfall};
use std::cell::RefCell;
use std::cmp;
use std::collections::{BTreeMap, HashMap};
use std::env;
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

/// Compute width of the GGUF forward. A gguf runs at its native precision, not
/// forced to f64: f16/bf16/quantized weights dequant to f32 (their canonical
/// compute type) and every activation, scalar, and scratch buffer is carved at
/// this width, so the dtype-generic kernels dispatch to the f32 path throughout.
pub(crate) const FWD_DT: Dtype = Dtype::F32;

/// Process-unique suffix for KV spill file names, so concurrent caches in one
/// process (tests) never collide on a path.
static KV_SPILL_SEQ: AtomicU64 = AtomicU64::new(0);

/// Carves `n` forward-width elements, reporting a carve-miss in the right units.
fn falloc(n: usize) -> Result<GpuBuffer, gpu_core::HipError> {
	return GpuBuffer::alloc_ty(n, FWD_DT);
}

/// GGUF architecture strings whose parity fixtures ALL measure OK against
/// llama.cpp (NMSE <= 1e-4 in archs_parity). Measured, never declared.
pub fn supported_archs() -> &'static [&'static str] {
	models::VERIFIED
}

/// Every GGUF architecture string [`models::dispatch`] can compose a decode
/// for, verified or not. Composable answers "can we attempt a forward";
/// [`supported_archs`] answers "is the output measured correct".
pub fn composable_archs() -> &'static [&'static str] {
	models::COMPOSABLE
}

/// True if `arch` has a dispatchable decode composition.
pub fn arch_composable(arch: &str) -> bool {
	models::supported(arch)
}

/// True if every parity fixture config of `arch` is measured OK.
pub fn arch_supported(arch: &str) -> bool {
	models::verified(arch)
}

/// A loaded headless model for reference validation: claims a fixed 2 GB
/// device arena at open (no VRAM-probe re-exec; intended for small models) and
/// releases it on drop, so sequential opens in one process are valid. One
/// claim + one free per open stays inside the blocking-op budget. Errors if
/// the architecture is unsupported.
struct Headless {
	m: Model,
	ar: Arena,
	attn_scale: GpuBuffer,
}

impl Headless {
	fn open(gguf: &Path, cap: usize) -> Result<Self> {
		crate::init().map_err(|e| anyhow!("gpu init: {e:?}"))?;
		let want = (2usize << 30) & !((1 << 21) - 1);
		let slab =
			gpu_core::memory::claim_device_arena_bytes(want).context("claim device arena")?;
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

/// Headless greedy generation: embeds the exact `toks` (bypassing the tokenizer)
/// and greedily generates `n_new` tokens, returning the generated ids. EVERY arch
/// runs THROUGH the incremental cache (prefill the prompt once, then one row per
/// step: attention layers append their K/V and attend the full cache, recurrent
/// layers carry their scan/conv state) — the one decode path, so the reference
/// oracle validates exactly what generation runs.
pub fn greedy(gguf: &Path, toks: &[u32], n_new: usize) -> Result<Vec<u32>> {
	return greedy_windowed(gguf, toks, n_new, toks.len() + n_new);
}

/// [`greedy`] constrained to a `win`-row VRAM window: rows evicted past the window
/// spill to the host mirror and attention runs the segmented ascending carry, so
/// `greedy_windowed(.., win < len)` agreeing with `greedy(..)` is the driver-level
/// chunked==unchunked spill gate. `greedy` itself is the `win == len + n_new`
/// (never-spilling) case of this function.
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
	for c in toks.chunks(win) {
		forward_rows(&h.m, c, &h.attn_scale, &h.ar, &mut cache, &mut logits, &mut lm_scratch)?;
	}
	let mut generated = Vec::with_capacity(n_new);
	for _ in 0..n_new {
		let next = pick_greedy(&logits, vocab, lsc, softcap);
		generated.push(next);
		if generated.len() >= n_new {
			break;
		}
		forward_rows(&h.m, &[next], &h.attn_scale, &h.ar, &mut cache, &mut logits, &mut lm_scratch)?;
	}
	return Ok(generated);
}

/// Headless single forward: the last-position logits (`vocab` long, f64) for
/// the exact `toks`, for parity comparison against llama.cpp reference logits
/// on the same GGUF file. Runs THROUGH the cache prefill — the same one decode
/// path as generation — then applies the arch's logit scale and final softcap.
pub fn last_logits(gguf: &Path, toks: &[u32]) -> Result<Vec<f64>> {
	anyhow::ensure!(!toks.is_empty(), "last_logits: empty token sequence");
	let h = Headless::open(gguf, toks.len())?;
	let mut cache = KvCache::new(&h.m, toks.len(), 0)?;
	let mut logits = vec![0.0f64; h.m.hp.vocab];
	let mut lm_scratch = vec![0.0f64; h.m.hp.lm_chunk];
	forward_rows(&h.m, toks, &h.attn_scale, &h.ar, &mut cache, &mut logits, &mut lm_scratch)?;
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
		let Some(_keep) = Some(()).filter(|_probe| !matches!(t.status, TokStatus::Draft))
		else {
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

/// Reads an optional unsigned-integer kv, returning `default` when the key is absent.
/// Absence is legal (e.g. expert params on a dense model); a present-but-wrong type still errors.
fn uint_kv_or(g: &Gguf, key: &str, default: usize) -> Result<usize> {
	match g.kv.get(key) {
		None => Ok(default),
		Some(v) => as_uint(v)
			.map(|x| x as usize)
			.ok_or_else(|| anyhow!("gguf: kv {key} is not an unsigned integer")),
	}
}

/// Reads an optional f32 kv as f64, returning `default` when the key is absent.
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
				as_uint(v).map(|x| x as usize).ok_or_else(|| {
					anyhow!("gguf: kv {key} array holds a non-uint element")
				})
			})
			.collect(),
		Some(_other) => bail!("gguf: kv {key} is not an array"),
		None => bail!("gguf: kv {key} not found"),
	}
}

/// Per-layer unsigned hparam: a scalar broadcasts to all `nl` layers; an array
/// must have exactly `nl` elements. Mirrors llama.cpp `get_key_or_arr` (the
/// `n_head_arr`/`n_ff_arr` reads), so hybrid archs that store `head_count` /
/// `feed_forward_length` per layer parse through the same shared vocabulary a
/// scalar arch broadcasts unchanged. `default` is used only when the key is absent.
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
	rotary: usize,
	has_v: bool,
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
	value_length: usize,
	key_length_swa: usize,
	value_length_swa: usize,
	/// lfm2 short-conv cache width (`shortconv.l_cache`), the depthwise-conv kernel
	/// width for shortconv recurrent layers; `0` for non-shortconv arches.
	shortconv_l_cache: usize,
	/// Multi-head Latent Attention ranks + head sub-dims (deepseek2 family). All
	/// zero for non-MLA arches. `q_lora_rank`/`kv_lora_rank` are the latent
	/// bottleneck widths; `head_k_mla`/`head_v_mla` the decompressed per-head key
	/// and value dims (`attention.key_length_mla`/`value_length_mla`); `n_rot` the
	/// RoPE sub-dim (`rope.dimension_count`), so `head_k_mla - n_rot` is the nope part.
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
	/// Selective-SSM dims (mamba family), all zero for non-SSM arches:
	/// `ssm_d_conv` conv kernel width, `ssm_d_inner` inner width (=2*ne for
	/// mamba-1), `ssm_d_state` state size N, `ssm_dt_rank` low-rank dt width,
	/// `ssm_n_group` selective B/C groups (0 read as 1); the dt/B/C RMSNorm
	/// is gated by tensor presence, not a flag.
	ssm_d_conv: usize,
	ssm_d_inner: usize,
	ssm_d_state: usize,
	ssm_dt_rank: usize,
	ssm_n_group: usize,
	/// plamo2 low-rank dt width `max(64, ne/16)` (DERIVED, not a KV): plamo2's
	/// `ssm_x` projects the conv output to `[dt_dim | B | C]` with this dt width,
	/// then `ssm_dt` up-projects it to `n_head`. Zero for the other SSM arches.
	ssm_dt_dim: usize,
	/// Kimi Delta Attention per-head dim (`{arch}.kda.head_dim`), the `d` of the
	/// KDA delta-rule state; `0` for the GDA delta arches (which use `ssm_d_state`)
	/// and non-delta arches.
	kda_head_dim: usize,
	/// KDA head count, derived from the recurrent-layer `ssm_dt.bias` width /
	/// `kda_head_dim`. `0` outside kimi-linear.
	kda_n_head: usize,
	/// Widest gated-delta scratch element count per token (conv_dim, value_dim, the
	/// fused Q+gate projection, the KDA g vector). `0` for non-delta arches, sizing
	/// their delta arena buffers to 1.
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
}

impl Hparams {
	/// True when the arch uses Multi-head Latent Attention (deepseek2 family):
	/// both decompressed head dims are declared, mirroring llama.cpp `is_mla()`.
	fn is_mla(&self) -> bool {
		return self.head_k_mla > 0 && self.head_v_mla > 0;
	}

	/// Parses arch-specific hyperparameters, with several derivations that mirror
	/// llama.cpp defaults for minimal GGUFs:
	/// - head dim = embedding_length / head_count when explicit key_length/value_length are absent.
	/// - minicpm3 is a naive (non-absorbed) MLA: it ships q/kv-lora ranks but no key_length_mla/value_length_mla, so per-head key/value dims derive from the standard key_length/value_length to size the MLA scratch and widen window.
	/// - plamo2 derives its low-rank dt width from n_embd (plamo2.cpp:43).
	/// - recurrent-layer interleave for the delta hybrids comes from the declared recurrent_layers array, else n_head_kv[l]==0 (kimi, mamba-hybrid style); KDA head count = recurrent-layer ssm_dt.bias width / kda.head_dim.
	/// Widen-window sizing (win_elems = widest widened-f64 matrix live at once; stage_bytes = widest raw bf16 region staged at once):
	/// - MLA latent projections (wq_b is q_lora_rank x n_head*head_k_mla) can exceed any standard projection, so the window must cover them.
	/// - SSM projections can exceed any attention/FFN weight (mamba-1 ssm_in = 2*d_inner*ne; mamba-2 = d_in_proj*ne where d_in_proj = 2*d_inner + 2*n_group*d_state + n_head, n_head=dt_rank), so the window covers the widest single-tensor stream.
	/// - gated-delta activation scratch width covers conv_dim, d_inner, the fused Q+gate projection, and the per-head state dim (zero for non-delta arches).
	/// - lfm2 shortconv in_proj is [3*ne, ne], wider than any attention/FFN weight.
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
		let value_length_swa = uint_kv_or(g, &k("attention.value_length_swa"), value_length)?;
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
		let ssm_n_group = if ssm_n_group_raw == 0 { 1 } else { ssm_n_group_raw };
		let ssm_dt_dim = if arch == "plamo2" { (ne / 16).max(64) } else { 0 };
		let kda_head_dim = uint_kv_or(g, &k("kda.head_dim"), 0)?;
		let is_recr: Vec<bool> = match g.kv.get(&k("attention.recurrent_layers")) {
			Some(Val::Arr(_items)) => {
				uint_arr(g, &k("attention.recurrent_layers"))?.iter().map(|&x| x != 0).collect()
			}
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
		let eos = uint_kv_or(g, "tokenizer.ggml.eos_token_id", 1)? as u32;
		let mask = uint_kv_or(g, "tokenizer.ggml.mask_token_id", 0)?;
		let eog = {
			let mut set = vec![eos];
			if let Ok(eot) = uint_kv(g, "tokenizer.ggml.eot_token_id") {
				set.push(eot as u32);
			}
			let pieces = g.str_arr("tokenizer.ggml.tokens").unwrap_or_default();
			for eog_piece in ["<|im_end|>", "<|end|>", "<end_of_turn>", "<|eot_id|>", "<|endoftext|>"] {
				if let Some(id) = pieces.iter().position(|p| p == eog_piece) {
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
			let has_v = g.tensors.contains_key(&format!("blk.{l}.attn_v.weight"));
			let (hd, rotary) = if sliding {
				(key_length_swa, key_length_swa)
			} else {
				(key_length, 128)
			};
			dims.push(LayerDims {
				hd,
				nqh: n_head_arr[l],
				nff: n_ff_arr[l],
				nkv: head_count_kv[l],
				rotary,
				has_v,
				sliding,
			});
		}

		let kd_max = dims.iter().map(|d| d.nkv * d.hd).max().unwrap_or(0);
		let qd_max = dims
			.iter()
			.map(|d| d.nqh * d.hd)
			.max()
			.unwrap_or(nqh * key_length);
		let gu_bytes = 2 * nffe * ne * 2;
		let dn_bytes = nffe * ne * 2;
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
		let shortconv_win = if shortconv_l_cache > 0 { 3 * ne * ne } else { 0 };
		let win_elems = (qd_max.max(kd_max).max(nff).max(2 * nffe) * ne)
			.max(mla_win)
			.max(ssm_win)
			.max(shortconv_win)
			.max(2 * qd_max * ne);
		let lm_chunk = win_elems / ne;
		let stage_bytes = (win_elems * 2).max(slot_bytes);

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
			value_length,
			key_length_swa,
			value_length_swa,
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
		})
	}
}

fn bf16(h: u16) -> f64 {
	f32::from_bits((h as u32) << 16) as f64
}

/// Adds the learned absolute position embedding to `t` rows of `base` starting at
/// absolute position `pos0`. A no-op for RoPE arches (empty `pos_embd`), so the
/// caller stays arch-agnostic.
fn add_pos_embd(m: &Model, base: &mut [f64], pos0: usize, t: usize, ne: usize) {
	if m.pos_embd.is_empty() {
		return;
	}
	for p in 0..t {
		let row = (pos0 + p) * ne * 2;
		for x in 0..ne {
			let b = row + x * 2;
			base[p * ne + x] += bf16(u16::from_le_bytes([m.pos_embd[b], m.pos_embd[b + 1]]));
		}
	}
}

/// Expert feed-forward width taken from the expert weight tensor's own shape, for
/// models that omit or zero `expert_feed_forward_length` in metadata. Gate experts
/// are `[n_embd, n_ff_exp, n_expert]`, a fused gate+up tensor `[n_embd, 2*n_ff_exp,
/// n_expert]`; the first block carrying experts wins (dense-lead layers have none).
/// Ground truth from the tensor, never a guessed constant.
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

const GT_BF16: u32 = u32::MAX;

#[derive(Clone)]
struct Tensor {
	shard: usize,
	off: usize,
	nbytes: usize,
	shape: Vec<usize>,
	gt: u32,
}

fn bview(buf: &GpuBuffer, off_bytes: usize, len_bytes: usize) -> GpuBuffer {
	if !(off_bytes.is_multiple_of(8) && len_bytes.is_multiple_of(8)) {
		Write::error(format!(
			"bview: unaligned {off_bytes}/{len_bytes}"
		));
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
	cmoes: GpuBuffer,
	moe_xg: GpuBuffer,
	moe_gu: GpuBuffer,
	moe_ea: GpuBuffer,
	moe_dv: GpuBuffer,
	mo: GpuBuffer,
	mop: GpuBuffer,
	comb: GpuBuffer,
	ha: GpuBuffer,
	hb: GpuBuffer,
	/// The frozen non-parametric-normed initial embedding (talkie skip source),
	/// stashed once by the decode loop and added to every layer. Sized `1` for
	/// non-talkie arches.
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
	/// MLA attention scratch (deepseek2 family). Sized `1` for non-MLA arches.
	/// `mqa` q latent, `mqb` q per head, `mqn`/`mqp` q nope/rope split, `mqx`
	/// q_nope absorbed via wk_b, `mqc` Qcur; `mkv` kv+rope compressed, `mkc`
	/// kv latent (Vcur), `mkp` k rope, `mkk` Kcur; `mrw` raw attn out, `mav`
	/// value-decompressed via wv_b.
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
	/// Selective-SSM scratch (mamba family). Sized `1` for non-SSM arches.
	/// `ss_x`/`ss_z` the in_proj split, `ss_xc` conv+SiLU output, `ss_db` the
	/// ssm_x projection [dt_lr|B|C], `ss_dtlr`/`ss_bb`/`ss_cc` its column slices,
	/// `ss_dt` the projected time-step, `ss_y` the scan readout. Mamba-2 adds
	/// `ss_xbc` (in_proj xBC split, `[t, conv_dim]`) and `ss_xbcc` (its conv+SiLU
	/// output, from which the scan reads `x`/`B`/`C` by offset); it reuses `ss_z`
	/// for `z`, `ss_dtlr` for the `[t, n_head]` dt, and `ss_y` for the readout.
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
	/// plamo2 in_proj output `[t, 2*d_inner]` before the per-head z/x
	/// de-interleave; sized `1` for the other SSM arches which split `in_proj`
	/// by contiguous weight views instead.
	ss_zx: GpuBuffer,
	/// Gated-delta-net scratch (qwen3.5/next/moe, kimi-linear); sized `1` for the
	/// other arches. `d_qkv` the mixed q|k|v projection, `d_cv` its conv+SiLU
	/// output, `d_q`/`d_k`/`d_v` the per-head split + L2-normed q/k/v (and, on
	/// full-attention layers, the fused Q+gate and its slices), `d_z` the SiLU
	/// output gate, `d_g` the per-head (GDA) or per-channel (KDA) log-decay,
	/// `d_bt` beta, `d_o` the delta-scan readout.
	d_qkv: GpuBuffer,
	d_cv: GpuBuffer,
	d_q: GpuBuffer,
	d_k: GpuBuffer,
	d_v: GpuBuffer,
	d_z: GpuBuffer,
	d_g: GpuBuffer,
	d_bt: GpuBuffer,
	d_o: GpuBuffer,
	/// Query-side online-softmax carry `(cm, cl, cacc)` threaded across the
	/// ascending spill segment launches — per decode row, so it scales with this
	/// arena's width. The K/V staging windows live on the [`KvCache`] (carved once
	/// at cache-open, window-granular), never here.
	cm: GpuBuffer,
	cl: GpuBuffer,
	cacc: GpuBuffer,
}

impl Arena {
	fn new(hp: &Hparams, t: usize) -> Result<Arena> {
		let c = hp.ncanvas.max(1);
		let ne = hp.ne;
		let nff = hp.nff;
		let nffe = hp.nffe;
		let a = |n: usize| -> Result<GpuBuffer> {
			return falloc(n)
				.map_err(|_e| anyhow!("{}", gpu_core::memory::carve_miss_message(n * FWD_DT.elem_size())));
		};
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
			cmoes: a(t * ne)?,
			moe_xg: a(t * ne)?,
			moe_gu: a(t * 2 * nffe)?,
			moe_ea: a(t * nffe)?,
			moe_dv: a(t * ne)?,
			mo: a(t * ne)?,
			mop: a(t * ne)?,
			comb: a(t * ne)?,
			ha: a(t * ne)?,
			hb: a(t * ne)?,
			embd_skip: a(if hp.arch == "talkie" { t * ne } else { 1 })?,
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
	/// talkie per-head scalar Q gain, expanded to a full `[nqh*hd]` per-column
	/// vector so [`gpu_broadcast_mul`] applies each head's scalar over its head_dim.
	/// `None` outside talkie.
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
	emb: Vec<u8>,
	/// Untied output projection (bf16 bytes), when the model ships a separate
	/// `output.weight`. Empty when the LM head is tied to `emb`.
	out: Vec<u8>,
	/// Learned LM-head output bias (`output.bias`, f64), added to the logits for
	/// arches whose graph applies it. Empty otherwise.
	out_b: Vec<f64>,
	/// Learned absolute position embedding table (`position_embd.weight`, bf16
	/// bytes), added to the input embeddings by position. Empty for RoPE arches.
	pos_embd: Vec<u8>,
	eps: GpuBuffer,
	res_scale: GpuBuffer,
	/// `1/sqrt(n_embd_head_k_mla)` device scalar for MLA attention pre-scaling;
	/// `1.0` for non-MLA arches (unused there).
	attn_scale_mla: GpuBuffer,
	theta_full: GpuBuffer,
	theta_slide: GpuBuffer,
	/// LongRoPE per-pair frequency factors (`[n_rot/2]`, minicpm3 `rope_factors_*`);
	/// `None` for arches without per-dim rope scaling.
	rope_factors: Option<GpuBuffer>,
	ls_dev: Vec<GpuBuffer>,
	/// Per-layer selective-SSM parameters loaded as f64 device buffers (mamba
	/// family): `ssm_conv_w` `[d_inner, d_conv]` channel-major, `ssm_conv_b`
	/// `[d_inner]`, `ssm_a` `[d_inner, d_state]` (mamba-1) or `[n_head]` (mamba-2)
	/// channel-major (used directly), `ssm_d` `[d_inner]`/`[n_head]`, `ssm_dt_b`
	/// `[d_inner]`/`[n_head]`, `ssm_norm` the mamba-2 grouped-gated-norm gamma
	/// `[d_inner/n_group, n_group]`. Empty for non-SSM layers.
	ssm_conv_w: Vec<Option<GpuBuffer>>,
	ssm_conv_b: Vec<Option<GpuBuffer>>,
	ssm_a: Vec<Option<GpuBuffer>>,
	ssm_d: Vec<Option<GpuBuffer>>,
	ssm_dt_b: Vec<Option<GpuBuffer>>,
	ssm_norm: Vec<Option<GpuBuffer>>,
	/// Per-layer FFN gate-projection bias (falcon-h1, granitehybrid SwiGLU
	/// biases). `None` when absent.
	ffn_gate_bias: Vec<Option<GpuBuffer>>,
	/// Per-layer mamba-1 dt/B/C RMSNorm gammas (jamba, plamo2), applied to the
	/// low-rank `dt`/`B`/`C` before the `ssm_dt` up-projection. `None` for the
	/// plain mamba variant that omits them.
	ssm_dt_norm: Vec<Option<GpuBuffer>>,
	ssm_b_norm: Vec<Option<GpuBuffer>>,
	ssm_c_norm: Vec<Option<GpuBuffer>>,
	/// Per-layer KDA causal-conv weights (kimi-linear), one `[d_inner, d_conv]`
	/// channel-major buffer per q/k/v projection (kimi convolves q/k/v with three
	/// separate depthwise kernels). `None` for the GDA arches (single mixed conv,
	/// carried in `ssm_conv_w`) and non-recurrent layers.
	ssm_q_conv_w: Vec<Option<GpuBuffer>>,
	ssm_k_conv_w: Vec<Option<GpuBuffer>>,
	ssm_v_conv_w: Vec<Option<GpuBuffer>>,
	/// Per-layer MoE router selection bias (`exp_probs_b`, kimi-linear sigmoid
	/// gating): added to the gating probs for top-k selection only, the weights
	/// themselves stay unbiased. `None` for softmax-router arches.
	exp_probs_b: Vec<Option<GpuBuffer>>,
	hp: Hparams,
}

/// The per-layer norm slot names, one enum variant per [`LAYER_NORMS`] row; each
/// layer's norms live in a flat `[Option<GpuBuffer>; N_NORMS]` indexed by
/// `Nk as usize` (no hashing in the decode loop).
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
	fn norm(&self, l: usize, k: Nk) -> Result<&GpuBuffer> {
		return self.norms[l][k as usize]
			.as_ref()
			.ok_or_else(|| anyhow!("{}: layer {l} has no {:?} norm weight", self.hp.arch, k.name()));
	}

	fn qrange(t: &Tensor, off: usize, len: usize) -> Result<(usize, usize)> {
		let (bb, be) = crate::dequant::block_layout(t.gt)?;
		anyhow::ensure!(
			off.is_multiple_of(2) && len.is_multiple_of(2) && (off / 2).is_multiple_of(be),
			"qrange: read {off}+{len} not block-aligned (be={be})"
		);
		let qoff = off / 2 / be * bb;
		let qlen = (len / 2).div_ceil(be) * bb;
		Ok((qoff, qlen))
	}

	fn read_bytes(&self, t: &Tensor, off: usize, len: usize) -> Result<Vec<u8>> {
		if t.gt == GT_BF16 {
			let mut buf = vec![0u8; len];
			let _d = Instant::now();
			self.shards[t.shard]
				.read_exact_at(&mut buf, (t.off + off) as u64)
				.with_context(|| format!("read {len} bytes at shard {}", t.shard))?;
			acc(&DISK_NS, _d);
			return Ok(buf);
		}
		let (qoff, qlen) = Self::qrange(t, off, len)?;
		let mut qbuf = vec![0u8; qlen];
		let _d = Instant::now();
		self.shards[t.shard]
			.read_exact_at(&mut qbuf, (t.off + qoff) as u64)
			.with_context(|| format!("read {qlen} quant bytes"))?;
		acc(&DISK_NS, _d);
		let mut out = crate::dequant::dequant_bf16(t.gt, &qbuf)?;
		out.truncate(len);
		Ok(out)
	}

	fn read_into(
		&self,
		t: &Tensor,
		off: usize,
		len: usize,
		dst: &GpuBuffer,
		dst_off: usize,
	) -> Result<()> {
		if t.gt != GT_BF16 {
			let bytes = self.read_bytes(t, off, len)?;
			let _h = Instant::now();
			bview(dst, dst_off, len).write_u8(&bytes)?;
			acc(&H2D_NS, _h);
			return Ok(());
		}
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
		Ok(())
	}

	fn small_f64(&self, name: &str) -> Result<Vec<f64>> {
		let t = self
			.big
			.get(name)
			.ok_or_else(|| anyhow!("missing {name}"))?;
		if t.gt != GT_BF16 {
			let (qoff, qlen) = Self::qrange(t, 0, t.nbytes)?;
			let mut qbuf = vec![0u8; qlen];
			self.shards[t.shard]
				.read_exact_at(&mut qbuf, (t.off + qoff) as u64)
				.with_context(|| format!("small {name}"))?;
			let mut f = crate::dequant::dequant_f32(t.gt, &qbuf)?;
			f.truncate(t.nbytes / 2);
			return Ok(f.iter().map(|&x| x as f64).collect());
		}
		let raw = self.read_bytes(t, 0, t.nbytes)?;
		Ok(raw.chunks_exact(2)
			.map(|c| bf16(u16::from_le_bytes([c[0], c[1]])))
			.collect())
	}

	fn read_host(&self, t: &Tensor, off: usize, dst: &mut [u8]) -> Result<()> {
		if t.gt == GT_BF16 {
			let _d = Instant::now();
			self.shards[t.shard]
				.read_exact_at(dst, (t.off + off) as u64)
				.with_context(|| format!("read_host {} bytes", dst.len()))?;
			acc(&DISK_NS, _d);
			return Ok(());
		}
		let bytes = self.read_bytes(t, off, dst.len())?;
		dst.copy_from_slice(&bytes);
		Ok(())
	}

	fn widen_from(&self, src: &GpuBuffer, off_bytes: usize, n: usize) -> Result<GpuBuffer> {
		let _w = Instant::now();
		gpu_widen_bf16(&bview(src, off_bytes, n * 2), n, &self.win)
			.map_err(|e| anyhow!("widen_bf16 launch: {e:?}"))?;
		acc(&WIDEN_NS, _w);
		Ok(self.win.view(0, n))
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
		let n = t.nbytes / 2;
		match self.store.home(name) {
			Some(Home::Vram(dev)) => self.widen_from(dev, 0, n),
			Some(Home::Ram(bytes)) => {
				self.to_stage(bytes)?;
				self.widen_from(&self.stage, 0, n)
			}
			_other => {
				self.read_into(t, 0, t.nbytes, &self.stage, 0)?;
				self.widen_from(&self.stage, 0, n)
			}
		}
	}

	/// True if layer `l` carries expert tensors (fused or separate gate/up). The
	/// dense-vs-MoE choice is per layer and presence-driven, so a model with a
	/// dense-lead prefix (e.g. deepseek) routes each layer correctly with no
	/// per-arch code.
	fn layer_is_moe(&self, l: usize) -> bool {
		self.big.contains_key(&layer_name(l, "experts.gate_proj"))
			|| self.big.contains_key(&layer_name(l, "experts.gate_up_proj"))
	}

	/// Composes expert `e` of layer `l` into `dst` as the packed slot
	/// `[gate | up | down]`, sourcing gate+up from either one fused
	/// `experts.gate_up_proj` tensor or the separate `experts.gate_proj` and
	/// `experts.up_proj` tensors (llama.cpp's two expert storage layouts).
	fn fill_expert(&self, l: usize, e: usize, dst: &mut [u8]) -> Result<()> {
		let (gu_bytes, dn_bytes, half) =
			(self.hp.gu_bytes, self.hp.dn_bytes, self.hp.dn_bytes);
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
			self.hp.dn_bytes,
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

/// Flat gguf -> neutral tensor map for whole-model (non-per-block) tensors. The
/// runtime composes the arch purely by resolving these transfers; no per-arch
/// code decides where a tensor goes.
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
	("self_cond_pre_norm.weight", "self_conditioning.pre_norm.weight"),
	("self_cond_gate.weight", "self_conditioning.gate_proj.weight"),
	("self_cond_up.weight", "self_conditioning.up_proj.weight"),
	("self_cond_down.weight", "self_conditioning.down_proj.weight"),
];

/// Flat gguf `blk.N.<suffix>` -> neutral `layers.N.<suffix>` map. Norm-name
/// variants (attn_norm vs input_layernorm, ffn_norm vs pre_feedforward, ...) and
/// fused sources (attn_qkv, separate expert gate/up) are all vocabulary here;
/// the loader adds nothing per arch. Fused `attn_qkv.weight` lands under one
/// neutral fused role and [`SLICE_MAP`] then feeds q/k/v as row-subranges.
const LAYER_MAP: &[(&str, &str)] = &[
	("attn_norm.weight", "input_layernorm.weight"),
	("attn_norm.bias", "input_layernorm.bias"),
	("attn_norm_2.weight", "self_attn.attn_norm_2.weight"),
	("attn_norm_2.bias", "self_attn.attn_norm_2.bias"),
	("attn_output_norm.weight", "pre_feedforward_layernorm.weight"),
	("post_attention_norm.weight", "post_attention_layernorm.weight"),
	("post_attention_norm.bias", "post_attention_layernorm.bias"),
	("attn_q_norm.weight", "self_attn.q_norm.weight"),
	("attn_k_norm.weight", "self_attn.k_norm.weight"),
	("ffn_norm.weight", "pre_feedforward_layernorm.weight"),
	("ffn_norm.bias", "pre_feedforward_layernorm.bias"),
	("post_ffw_norm_1.weight", "post_feedforward_layernorm_1.weight"),
	("pre_ffw_norm_2.weight", "pre_feedforward_layernorm_2.weight"),
	("post_ffw_norm_2.weight", "post_feedforward_layernorm_2.weight"),
	("post_ffw_norm.weight", "post_feedforward_layernorm.weight"),
	("attn_q.weight", "self_attn.q_proj.weight"),
	("attn_k.weight", "self_attn.k_proj.weight"),
	("attn_v.weight", "self_attn.v_proj.weight"),
	("shortconv.conv.weight", "self_attn.shortconv_conv.weight"),
	("shortconv.in_proj.weight", "self_attn.shortconv_in_proj.weight"),
	("shortconv.out_proj.weight", "self_attn.shortconv_out_proj.weight"),
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

/// How a neutral tensor sources from a fused gguf tensor: a named row-subrange
/// whose split point is set by the layer's attention dims (`qd`, `kd`). This is
/// the transfer description that lets one fused gguf tensor feed several roles.
#[derive(Clone, Copy)]
enum Slice {
	QkvQ,
	QkvK,
	QkvV,
}

/// Derived neutral tensor <- fused source neutral tensor, sliced by [`Slice`].
/// Resolved per layer in [`load_model_gguf`] once the dims are known, so a model
/// shipping a fused `attn_qkv` transparently yields separate q/k/v projections.
const SLICE_MAP: &[(&str, &str, Slice)] = &[
	("self_attn.q_proj.weight", "self_attn.qkv_proj.weight", Slice::QkvQ),
	("self_attn.k_proj.weight", "self_attn.qkv_proj.weight", Slice::QkvK),
	("self_attn.v_proj.weight", "self_attn.qkv_proj.weight", Slice::QkvV),
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
		let elems: usize = info.dims.iter().product();
		let mut shape: Vec<usize> = info.dims.clone();
		shape.reverse();
		big.insert(
			hf,
			Tensor {
				shard: 0,
				off: info.offset as usize,
				nbytes: elems * 2,
				shape,
				gt: info.ggml_type,
			},
		);
	}
	// lfm2/lfm2moe ship the FINAL pre-lm_head norm under the name token_embd_norm (LLM_TENSOR_OUTPUT_NORM_LFM2), applied as output_norm, not an embed norm.
	if matches!(hp.arch.as_str(), "lfm2" | "lfm2moe")
		&& let Some(t) = big.remove("model.decoder.embed_norm.weight")
	{
		big.insert("model.decoder.norm.weight".to_string(), t);
	}
	synth_qkv_slices(&mut big, &hp);
	synth_ffn_slices(&mut big, &hp);
	finish_load(vec![f], big, hp)
}

/// Splits a fused gate+up FFN projection into separate `mlp.gate_proj` and
/// `mlp.up_proj` when a gated model ships them stacked in one `ffn_up` tensor
/// (phi3, glm4, chatglm): a `[2*nff]`-row up tensor with no gate becomes gate =
/// rows `[0, nff)`, up = rows `[nff, 2*nff)`. Shape-driven, so nothing per-arch
/// decides the split.
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
		let (src_shard, src_off, src_gt) = (up.shard, up.off, up.gt);
		let (Ok((q0, _)), Ok((q1, _))) = (
			Model::qrange(up, 0, nff * ne * 2),
			Model::qrange(up, nff * ne * 2, nff * ne * 2),
		) else {
			continue;
		};
		let part = |qoff: usize| Tensor {
			shard: src_shard,
			off: src_off + qoff,
			nbytes: nff * ne * 2,
			shape: vec![nff, ne],
			gt: src_gt,
		};
		big.insert(layer_name(l, "mlp.gate_proj.weight"), part(q0));
		big.insert(up_k, part(q1));
	}
}

/// Feeds q/k/v neutral projections from a fused `attn_qkv` source per [`SLICE_MAP`]
/// when the model ships no separate q/k/v tensors. Each slice is a row-subrange
/// of the fused tensor's data, split at `[qd | kd | kd]` from the layer's dims;
/// a fuse whose row count does not match `qd + 2*kd` (e.g. latent MLA) is left
/// alone, so its absent-projection error stays truthful.
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
			let Ok((qoff, _qlen)) = Model::qrange(src_t, row0 * ne * 2, rows * ne * 2) else {
				continue;
			};
			let sub = Tensor {
				shard: src_t.shard,
				off: src_t.off + qoff,
				nbytes: rows * ne * 2,
				shape: vec![rows, ne],
				gt: src_t.gt,
			};
			big.insert(nk, sub);
		}
	}
}

/// Materializes the loaded tensors into a `Model`, applying several arch-specific
/// fixups:
/// - minicpm3 hardcodes scale_depth=1.4 (minicpm3.cpp:67); the per-layer residual scale is scale_depth/sqrt(n_layer), applied to both the attn and FFN outputs.
/// - talkie's per-head scalar Q gain [n_head] expands to [nqh*hd] so a single broadcast-mul applies each head's scalar across its head_dim.
/// - plamo2's causal conv ships no bias, so a zero one is synthesized for the shared launcher.
/// - LongRoPE per-pair frequency factors (minicpm3): for a context within the original training length the short factors apply (get_rope_factors); the fixtures ship identity factors, so this is exact machinery.
/// - untied LM head: some models (e.g. llama with a separate output.weight) project the final hidden state through their own matrix rather than the tied token embedding, loaded when present so lm_head uses the right one.
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
		let s = if hp.is_mla() { 1.0 / (hp.head_k_mla as f64).sqrt() } else { 1.0 };
		ub.load(&[s])?;
		ub
	};
	let nl = hp.nl;
	let vocab = hp.vocab;
	let ne = hp.ne;
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
		emb: Vec::new(),
		out: Vec::new(),
		out_b: Vec::new(),
		pos_embd: Vec::new(),
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
	Write::line(data, "norm convention: folded x*w (gguf stores gammas as saved)");

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
			let g = upload_gamma(&m.small_f64(&p("post_attention_layernorm.weight"))?, plus_one)?;
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
		m.ssm_conv_w.push(opt_bias(&m, p("self_attn.ssm_conv1d.weight"))?);
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
		m.ssm_norm.push(opt_bias(&m, p("self_attn.ssm_norm.weight"))?);
		m.ffn_gate_bias.push(opt_bias(&m, p("mlp.gate_proj.bias"))?);
		m.ssm_dt_norm.push(opt_bias(&m, p("self_attn.ssm_dt_norm.weight"))?);
		m.ssm_b_norm.push(opt_bias(&m, p("self_attn.ssm_b_norm.weight"))?);
		m.ssm_c_norm.push(opt_bias(&m, p("self_attn.ssm_c_norm.weight"))?);
		m.ssm_q_conv_w.push(opt_bias(&m, p("self_attn.q_conv.weight"))?);
		m.ssm_k_conv_w.push(opt_bias(&m, p("self_attn.k_conv.weight"))?);
		m.ssm_v_conv_w.push(opt_bias(&m, p("self_attn.v_conv.weight"))?);
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
	if m.big.contains_key("model.decoder.embed_norm.weight")
		&& m.big.contains_key("model.decoder.embed_norm.bias")
	{
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
	if m.big.contains_key("model.decoder.self_conditioning.pre_norm.weight") {
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

	let et =
		m.big.get("model.decoder.embed_tokens.weight")
			.ok_or_else(|| anyhow!("no embed_tokens"))?;
	if et.shape != vec![vocab, ne] {
		bail!("embed_tokens shape {:?}", et.shape);
	}
	m.emb = m.read_bytes(et, 0, et.nbytes)?;

	if let Some(ot) = m.big.get("model.decoder.lm_head.weight").cloned() {
		if ot.shape == vec![vocab, ne] {
			m.out = m.read_bytes(&ot, 0, ot.nbytes)?;
		}
	}
	if m.big.contains_key("model.decoder.lm_head.bias") {
		m.out_b = m.small_f64("model.decoder.lm_head.bias")?;
	}
	if let Some(pt) = m.big.get("model.decoder.pos_embd.weight").cloned() {
		m.pos_embd = m.read_bytes(&pt, 0, pt.nbytes)?;
	}

	Ok(m)
}

/// The dense-projection weights actually present for layer `l`, from the fixed
/// role vocabulary. Presence-driven, so a non-gated FFN never demands a gate, a
/// pure-MoE layer contributes no dense FFN weights (its experts load separately),
/// and a fused-qkv model contributes the synthesized q/k/v — the required set
/// derives from the composition, never a fixed per-arch list.
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
	ROLES
		.iter()
		.map(|r| layer_name(l, r))
		.filter(|n| m.big.contains_key(n))
		.collect()
}

fn preflight(m: &Model, ar: &Arena, t: usize) -> Result<()> {
	let hp = &m.hp;
	gpu_gemm_bt_f64(
		&ar.x,
		&m.win.view(0, hp.qd_max * hp.ne),
		t,
		hp.qd_max,
		hp.ne,
		&ar.q,
	)?;
	beat();
	gpu_gemm_bt_f64(
		&ar.cms,
		&m.win.view(0, hp.nff * hp.ne),
		t,
		hp.nff,
		hp.ne,
		&ar.g,
	)?;
	beat();
	gpu_gemm_bt_f64(
		&ar.act,
		&m.win.view(0, hp.ne * hp.nff),
		t,
		hp.ne,
		hp.nff,
		&ar.mlp0,
	)?;
	beat();
	gpu_gemm_bt_f64(
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

/// Places every weight into `store`, then hands the store to `m` UNCONDITIONALLY
/// (even on placement failure) so the arena slab inside it stays reachable for
/// release; the canary readback then proves uploads are GPU-visible.
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
					m.emb[off..off + n].to_vec()
				} else {
					m.read_bytes(t, off, n)?
				};
				let mut got = vec![0u8; n];
				bview(dev, off, n).download_u8(&mut got)?;
				if got != want {
					bail!(
						"waterfall {name} stale at byte {off}: upload not visible to GPU reads"
					);
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
	store.place("model.decoder.embed_tokens.weight", m.emb.len(), |dst| {
		dst.copy_from_slice(&m.emb);
		Ok(())
	})?;
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

fn rnp(x: &[f64], eps: f64) -> Vec<f64> {
	let inv = 1.0 / ((x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64) + eps).sqrt();
	x.iter().map(|v| v * inv).collect()
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

fn layer(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	prefix: usize,
	ar: &Arena,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let nff = hp.nff;
	let nffe = hp.nffe;
	let nqh = hp.nqh;
	let nexp = hp.nexp;
	let used = hp.used;
	let d = &hp.dims[l];
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, nqh * d.hd, d.nkv * d.hd);
	let theta = if d.sliding {
		&m.theta_slide
	} else {
		&m.theta_full
	};
	let _ta = Instant::now();
	gpu_rmsnorm_f64(h_in, m.norm(l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.q_proj.weight"))?,
		t,
		qd,
		ne,
		&ar.q,
	)?;
	let wk = m.stream(&layer_name(l, "self_attn.k_proj.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &wk, t, kd, ne, &ar.k)?;
	if d.has_v {
		gpu_gemm_bt_f64(
			&ar.x,
			&m.stream(&layer_name(l, "self_attn.v_proj.weight"))?,
			t,
			kd,
			ne,
			&ar.v,
		)?;
	} else {
		gpu_gemm_bt_f64(&ar.x, &wk, t, kd, ne, &ar.v)?;
	}
	gpu_rmsnorm_f64(&ar.q, m.norm(l, Nk::QNorm)?, &m.eps, t * nqh, hd, &ar.q)?;
	gpu_rmsnorm_f64(&ar.k, m.norm(l, Nk::KNorm)?, &m.eps, t * nkv, hd, &ar.k)?;
	gpu_rmsnorm_f64_nogamma(&ar.v, &m.eps, t * nkv, hd, &ar.v)?;
	gpu_rope_partial(theta, t * nqh, hd, d.rotary, nqh, &ar.q)?;
	gpu_rope_partial(theta, t * nkv, hd, d.rotary, nkv, &ar.k)?;
	gpu_flash_gqa(&ar.q, &ar.k, &ar.v, t, t, nqh, nkv, hd, 0.0, 0, prefix, &ar.attn, &ar.cm, &ar.cl, &ar.cacc, 0, true)?;
	gpu_gemm_bt_f64(
		&ar.attn,
		&m.stream(&layer_name(l, "self_attn.o_proj.weight"))?,
		t,
		ne,
		qd,
		&ar.o,
	)?;
	gpu_rmsnorm_f64(&ar.o, m.norm(l, Nk::PostAttn)?, &m.eps, t, ne, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	acc(&ATTN_NS, _ta);

	let _tm = Instant::now();
	gpu_rmsnorm_f64(&ar.attn_out, m.norm(l, Nk::PreFf)?, &m.eps, t, ne, &ar.cms)?;
	gpu_gemm_bt_f64(
		&ar.cms,
		&m.stream(&layer_name(l, "mlp.gate_proj.weight"))?,
		t,
		nff,
		ne,
		&ar.g,
	)?;
	gpu_gemm_bt_f64(
		&ar.cms,
		&m.stream(&layer_name(l, "mlp.up_proj.weight"))?,
		t,
		nff,
		ne,
		&ar.u,
	)?;
	gpu_gelu_mul(&ar.g, &ar.u, t * nff, &ar.act)?;
	gpu_gemm_bt_f64(
		&ar.act,
		&m.stream(&layer_name(l, "mlp.down_proj.weight"))?,
		t,
		ne,
		nff,
		&ar.mlp0,
	)?;
	gpu_rmsnorm_f64(&ar.mlp0, m.norm(l, Nk::Pf1)?, &m.eps, t, ne, &ar.mlp)?;
	acc(&MLP_NS, _tm);

	let _tmoe = Instant::now();
	gpu_rmsnorm_f64(&ar.attn_out, m.norm(l, Nk::Pn2)?, &m.eps, t, ne, &ar.cmoes)?;
	let _rt = Instant::now();
	let mut ao_host = vec![0.0f64; ar.attn_out.n_floats()];
	let mut cmoes_host = vec![0.0f64; ar.cmoes.n_floats()];
	ar.attn_out.download_host(&mut ao_host)?;
	ar.cmoes.download_host(&mut cmoes_host)?;
	gpu_core::hip::device_synchronize()?;
	acc(&MOE_RT_NS, _rt);
	let _tr = Instant::now();
	let (rw, gis, pe) = (&m.rw[l], &m.gis[l], &m.pe[l]);
	let inv_sqrt_ne = 1.0 / (ne as f64).sqrt();
	let mut e2p: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
	for p in 0..t {
		let rmn = rnp(&ao_host[p * ne..(p + 1) * ne], hp.eps);
		let rin: Vec<f64> = (0..ne).map(|xx| rmn[xx] * inv_sqrt_ne * gis[xx]).collect();
		let mut rl = vec![0.0f64; nexp];
		for (e, rle) in rl.iter_mut().enumerate() {
			let b = e * ne;
			*rle = (0..ne).map(|i| rw[b + i] * rin[i]).sum();
		}
		softmax(&mut rl);
		let mut idx: Vec<usize> = (0..nexp).collect();
		idx.sort_by(|a, b| rl[*b].partial_cmp(&rl[*a]).unwrap_or(cmp::Ordering::Equal));
		idx.truncate(used);
		let ws: f64 = idx.iter().map(|&e| rl[e]).sum();
		for &e in &idx {
			e2p.entry(e).or_default().push((p, rl[e] / ws));
		}
	}
	acc(&ROUTE_NS, _tr);
	let mut mo_host = vec![0.0f64; t * ne];
	let mut xg = vec![0.0f64; t * ne];
	let mut dv_host = vec![0.0f64; t * ne];
	for (&e, poslist) in &e2p {
		let np = poslist.len();
		for (i, &(p, _)) in poslist.iter().enumerate() {
			xg[i * ne..(i + 1) * ne].copy_from_slice(&cmoes_host[p * ne..(p + 1) * ne]);
		}
		let _rt = Instant::now();
		ar.moe_xg.load(&xg[..np * ne])?;
		acc(&MOE_RT_NS, _rt);
		let es = m.expert_slot(l, e)?;
		let gu_w = m.widen_from(&es, 0, 2 * nffe * ne)?;
		gpu_gemm_bt_f64(&ar.moe_xg, &gu_w, np, 2 * nffe, ne, &ar.moe_gu)?;
		gpu_glu_gelu(&ar.moe_gu, np, nffe, &ar.moe_ea)?;
		let dn_w = m.widen_from(&es, hp.gu_bytes, ne * nffe)?;
		gpu_gemm_bt_f64(&ar.moe_ea, &dn_w, np, ne, nffe, &ar.moe_dv)?;
		let _rt = Instant::now();
		ar.moe_dv.download_host(&mut dv_host[..np * ne])?;
		gpu_core::hip::device_synchronize()?;
		acc(&MOE_RT_NS, _rt);
		for (i, &(p, w)) in poslist.iter().enumerate() {
			let s = w * pe[e];
			for xx in 0..ne {
				mo_host[p * ne + xx] += s * dv_host[i * ne + xx];
			}
		}
	}
	ar.mo.load(&mo_host)?;
	gpu_rmsnorm_f64(&ar.mo, m.norm(l, Nk::Pf2)?, &m.eps, t, ne, &ar.mop)?;
	acc(&MOE_NS, _tmoe);

	gpu_add_into(&ar.mlp, &ar.mop, t * ne, &ar.comb)?;
	gpu_rmsnorm_f64(&ar.comb, m.norm(l, Nk::Pfw)?, &m.eps, t, ne, &ar.comb)?;
	gpu_add_into(&ar.attn_out, &ar.comb, t * ne, h_out)?;
	gpu_scale_f64_inplace(&m.ls_dev[l], t * ne, h_out)?;
	Ok(())
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

/// Allocation-free LM head: chunked vocab GEMM writing into caller-owned
/// `logits`/`out_host` scratch, so the per-token decode loop reuses one pair of
/// host buffers for the whole generation (AllocGuard spirit).
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
		let w = if m.out.is_empty() {
			match m.store.home("model.decoder.embed_tokens.weight") {
				Some(Home::Vram(dev)) => m.widen_from(dev, c0 * hp.ne * 2, cn * hp.ne)?,
				_other => {
					m.to_stage(&m.emb[c0 * hp.ne * 2..(c0 + cn) * hp.ne * 2])?;
					m.widen_from(&m.stage, 0, cn * hp.ne)?
				}
			}
		} else {
			m.to_stage(&m.out[c0 * hp.ne * 2..(c0 + cn) * hp.ne * 2])?;
			m.widen_from(&m.stage, 0, cn * hp.ne)?
		};
		gpu_gemm_bt_f64(hfs, &w, ncanvas, cn, hp.ne, &ar.lm_out)?;
		ar.lm_out.download_host(&mut out_host[..ncanvas * cn])?;
		gpu_core::hip::device_synchronize()?;
		for p in 0..ncanvas {
			logits[p * hp.vocab + c0..p * hp.vocab + c0 + cn]
				.copy_from_slice(&out_host[p * cn..(p + 1) * cn]);
		}
		c0 += cn;
	}
	if models::out_bias(m) && !m.out_b.is_empty() {
		for p in 0..ncanvas {
			for (j, lg) in logits[p * hp.vocab..(p + 1) * hp.vocab].iter_mut().enumerate() {
				*lg += m.out_b[j];
			}
		}
	}
	acc(&LM_NS, _tl);
	Ok(())
}


/// Per-layer KV cache carved once at [`ChatSession::open`] and kept resident for
/// the session: `k[l]`/`v[l]` hold up to `t_max` positions of this layer's
/// attention keys/values (GQA `nkv*hd`, or the compressed `kv_lora+rope` / `kv_lora`
/// for MLA), `len` is how many are populated, `ids` mirrors the token id at each
/// position so a later turn finds its common prefix. One-claim law: allocated at
/// open, never freed or grown mid-run.
/// One layer's cached decode state: the VARIANT is what the layer's mixer
/// actually carries per [`models::layer_cache_shape`] — attention layers hold
/// K/V rows plus their host-tier stores, recurrent mixers hold scan state and
/// conv ping-pong windows, hybrids hold both, pure-conv mixers (shortconv)
/// hold only windows. A scan mixer dispatched onto a `Kv` layer is
/// unrepresentable, not an error path.
pub(crate) enum LayerCache {
	Kv(KvSlot),
	Scan(ScanSlot),
	Conv(ConvSlot),
	KvScan(KvSlot, ScanSlot),
}

/// Attention rows: the VRAM window `[win_base, win_base+win)` in `k`/`v`, and
/// the host-tier stores (`hk`/`hv`, RAM then spill file) for evicted rows.
pub(crate) struct KvSlot {
	pub(crate) k: GpuBuffer,
	pub(crate) v: GpuBuffer,
	pub(crate) hk: HostStore,
	pub(crate) hv: HostStore,
}

/// Recurrent mixer state: the SSM/delta matrix state carried across steps, plus
/// the conv ping-pong windows (read `conv[i]`, write `nxt[i]`, swapped after
/// every forward so the next step reads what this one wrote).
pub(crate) struct ScanSlot {
	pub(crate) rec: GpuBuffer,
	pub(crate) conv: Vec<GpuBuffer>,
	pub(crate) nxt: Vec<GpuBuffer>,
}

/// Conv-only mixer state (shortconv): ping-pong windows, no matrix recurrence.
pub(crate) struct ConvSlot {
	pub(crate) conv: Vec<GpuBuffer>,
	pub(crate) nxt: Vec<GpuBuffer>,
}

impl LayerCache {
	/// The attention rows of this layer — loud error on a non-attention layer
	/// (a mixer/shape disagreement, never a silent recompute).
	pub(crate) fn kv(&self) -> Result<&KvSlot> {
		match self {
			LayerCache::Kv(s) | LayerCache::KvScan(s, _) => return Ok(s),
			_other => return Err(anyhow!("attention on a layer without K/V cache rows")),
		}
	}

	/// This layer's recurrent state — loud error on a layer without one.
	pub(crate) fn rec(&self) -> Result<&GpuBuffer> {
		match self {
			LayerCache::Scan(s) | LayerCache::KvScan(_, s) => return Ok(&s.rec),
			_other => return Err(anyhow!("scan mixer on a layer without recurrent state")),
		}
	}

	/// The `(read, write)` conv windows for conv `i` of this layer's mixer.
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
			_missing => return Err(anyhow!("conv {i} has no cache window (layer_cache_shape mismatch)")),
		}
	}
}

struct KvCache {
	/// Per-layer cached state — see [`LayerCache`].
	layers: Vec<LayerCache>,
	kw: Vec<usize>,
	vw: Vec<usize>,
	conv_sz: Vec<Vec<usize>>,
	rec_sz: Vec<usize>,
	/// VRAM window capacity in rows, and the absolute row currently at window offset 0.
	win: usize,
	win_base: usize,
	/// Double-buffered staging windows host K/V segments are read back into during
	/// a spilled decode, each sized for one full window of the widest layer's rows.
	/// Carved once at cache-open from the claim remainder after the weights and the
	/// resident window (the capacity divisor reserves their per-token cost ALWAYS,
	/// so the stage exists unconditionally), so the readback grain is the window —
	/// independent of any step's decode width.
	stage: KvStage,
	len: usize,
	ids: Vec<u32>,
	t_max: usize,
}

/// The cache's double-buffered spill staging: `sk[i]`/`sv[i]` each hold one full
/// window of the widest layer's K/V rows, so a spilled attention walks the host
/// mirror in window-sized read-ahead blocks.
pub(crate) struct KvStage {
	pub(crate) sk: [GpuBuffer; 2],
	pub(crate) sv: [GpuBuffer; 2],
}

/// Append-only host store for one layer's evicted K/V rows: bytes `[0, ram.len())`
/// live in RAM, everything after in a session-scoped spill file in the same
/// directory [`gpu_core::tiered::Budgets`] measures, deleted on drop. `ram_budget`
/// is this store's prorated share of the measured RAM tier — `ram_data * w / Σw`
/// over every layer's K and V row widths, so all stores cross to disk at the same
/// evicted-row count. The first append that would exceed the budget routes whole
/// to the file, so the RAM/disk boundary always sits on an append (whole-row)
/// edge and every staged block splits into at most one RAM and one file range.
///
/// The two io directions never share a path: eviction writes go through `wf`,
/// staging reads through an independent `rf` descriptor, both positioned
/// (`pwrite`/`pread` — no seek cursor) with no lock between them. NVMe is full
/// duplex, so eviction write-behind and staging read-ahead can run concurrently;
/// nothing in this structure serializes one direction against the other.
pub(crate) struct HostStore {
	ram: Vec<u8>,
	ram_budget: usize,
	wf: Option<fs::File>,
	rf: RefCell<Option<fs::File>>,
	path: PathBuf,
	file_len: usize,
}

impl HostStore {
	fn new(ram_budget: usize, path: PathBuf) -> HostStore {
		return HostStore {
			ram: Vec::new(),
			ram_budget,
			wf: None,
			rf: RefCell::new(None),
			path,
			file_len: 0,
		};
	}

	fn len(&self) -> usize {
		return self.ram.len() + self.file_len;
	}

	fn append_f32(&mut self, vals: &[f32]) -> Result<()> {
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
			.with_context(|| format!("KV spill write at {} in {}", self.file_len, self.path.display()))?;
		self.file_len += bytes.len();
		return Ok(());
	}

	/// Uploads bytes `[off, off+len)` into `dst` from wherever they live: the RAM
	/// range goes up directly, a file range is read into `scratch` first (through
	/// the read-side descriptor, opened lazily). Both ranges are whole rows (the
	/// RAM/disk boundary is append-aligned), so the element-offset view arithmetic
	/// is exact.
	pub(crate) fn stage_into(
		&self,
		off: usize,
		len: usize,
		dst: &GpuBuffer,
		scratch: &mut Vec<u8>,
	) -> Result<()> {
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
				*rf = Some(
					File::open(&self.path)
						.with_context(|| format!("KV spill read-open {}", self.path.display()))?,
				);
			}
			let Some(f) = rf.as_ref() else {
				bail!("KV host store: {} file bytes requested but no spill file", file_n);
			};
			let foff = (off + ram_n - self.ram.len()) as u64;
			scratch.resize(file_n, 0);
			f.read_exact_at(scratch, foff)
				.with_context(|| format!("KV spill read [{foff}, +{file_n}) from {}", self.path.display()))?;
			dst.view(ram_n / es, file_n / es).write_u8(&scratch[..])?;
		}
		return Ok(());
	}

	fn truncate(&mut self, len: usize) {
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
		let shapes: Vec<models::LayerCacheShape> =
			(0..nl).map(|l| return models::layer_cache_shape(m, l)).collect();
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
			let budget = if total_w > 0 { ram_budget / total_w * w } else { 0 };
			let path = spill_dir
				.join(format!("recipe-kv-{}-{sid}-l{l}-{name}.spill", process::id()));
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
					LayerCache::Conv(ConvSlot { conv: sc.conv, nxt: sc.nxt })
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
			ids: Vec::new(),
			t_max: win + host_cap,
		});
	}

	/// Drops cached positions past `keep` (cross-turn: rewind to the common prefix
	/// with the newly templated sequence). Buffers keep their bytes; only `len`/`ids`
	/// shrink, and stale rows are never read (attention bounds itself by `len + new`).
	/// Rewinding below `win_base` truncates the host mirror to the kept rows and
	/// restarts the window empty at `keep` — the kept prefix is then attended from
	/// the host tier. Legal only when no layer carries recurrent state, or when
	/// `keep == len` — a recurrence cannot be partially rewound; see
	/// [`KvCache::clear_scan`].
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

	/// Zeroes every recurrent/conv state and empties the cache, so a recurrent arch
	/// whose new prompt diverges before the end of the cached prefix reprocesses from
	/// row 0 through the same decode path (its scan state cannot be rewound to an
	/// interior position — only the final state is kept).
	fn clear_scan(&mut self) -> Result<()> {
		let es = FWD_DT.elem_size();
		let zero_scan = |s: &ScanSlot, sz: usize, szs: &[usize]| -> Result<()> {
			s.rec.memset_zero(sz.max(1) * es)?;
			for (b, &w) in s.conv.iter().chain(s.nxt.iter()).zip(szs.iter().chain(szs.iter())) {
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
					for (b, &w) in s.conv.iter().chain(s.nxt.iter()).zip(self.conv_sz[l].iter().chain(self.conv_sz[l].iter())) {
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

	/// Ensures the VRAM window can hold `t` more rows. When appending them would
	/// overflow, the resident rows `[win_base, len)` flush to the host byte mirror
	/// (D2H through the ledger) and `win_base` advances to `len` — the window
	/// restarts empty and the flushed rows are then attended from the host tier by
	/// the segmented driver. A within-window session never flushes, so it stays on
	/// the one-launch fits path. `true` iff a flush happened (the layer now spills).
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

/// Greedy argmax over `vocab` logits with the arch's logit scale and final softcap
/// applied (both monotone, so they only matter at the boundary), matching the
/// reference pick so the cached decode stays token-exact.
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

/// Runs one incremental forward over `rows` NEW tokens sitting at absolute
/// positions `cache.len ..`, appending their K/V into every layer's cache and
/// leaving the last row's logits in `logits`. Embed/Q/FFN/norm see only these rows;
/// attention reads the full cache (`len + rows`). A batch call (`rows.len() > 1`) is
/// the turn-1 prefill (`cache.len == 0`); single-row calls are cross-turn suffix
/// tokens and generation steps. Advances `cache.len`/`cache.ids` by `rows`.
fn forward_rows(
	m: &Model,
	rows: &[u32],
	attn_scale: &GpuBuffer,
	ar: &Arena,
	cache: &mut KvCache,
	logits: &mut [f64],
	lm_scratch: &mut [f64],
) -> Result<()> {
	let ne = m.hp.ne;
	let nl = m.hp.nl;
	let t = rows.len();
	let base_off = cache.len;
	let scale = models::embedding_scale(m);
	let mut base = vec![0.0f64; t * ne];
	for (p, &tk) in rows.iter().enumerate() {
		let b = tk as usize * ne * 2;
		for x in 0..ne {
			base[p * ne + x] =
				bf16(u16::from_le_bytes([m.emb[b + x * 2], m.emb[b + x * 2 + 1]])) * scale;
		}
	}
	add_pos_embd(m, &mut base, base_off, t, ne);
	let h0 = ar.ha.view(0, t * ne);
	h0.load(&base)?;
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

/// Set the first time [`forward_rows`] actually appended rows into a resident
/// [`KvCache`]. Read by the chat capability line — a proven-ran flag.
static KV_RAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[must_use]
pub fn kv_cache_ran() -> bool {
	return KV_RAN.load(Ordering::Relaxed);
}

/// Incremental autoregressive decode against the resident KV cache. Prefills the
/// uncached suffix of `toks` (batched when the cache is empty, else one token at a
/// time so the offset-causal attention runs through [`gpu_flash_decode_gqa`]), then
/// generates greedily one token per step — each step embeds/GEMMs/norms ONLY the new
/// token and attends it against the full cache. The cache and its token ids survive
/// the call so the next turn reuses the common prefix.
fn decode_cached(
	m: &Model,
	tokenizer: &crate::tokenizer::Tokenizer,
	toks: &[u32],
	attn_scale: &GpuBuffer,
	cache: &mut KvCache,
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
	let aw = if cache.len == 0 { p_new.min(chunk) } else { 1 };
	let ar = {
		let _t = gpu_core::memory::tag_scope("arena");
		Arena::new(&m.hp, aw.max(1))?
	};
	let mut logits = vec![0.0f64; m.hp.vocab];
	let mut lm_scratch = vec![0.0f64; m.hp.lm_chunk];
	let decode_start = Instant::now();
	if cache.len == 0 {
		for c in toks.chunks(chunk) {
			forward_rows(m, c, attn_scale, &ar, cache, &mut logits, &mut lm_scratch)?;
		}
	} else {
		let suffix = toks[cache.len..].to_vec();
		for tk in &suffix {
			forward_rows(m, &[*tk], attn_scale, &ar, cache, &mut logits, &mut lm_scratch)?;
		}
	}
	let mut out_ids: Vec<u32> = Vec::new();
	let mut next = pick_greedy(&logits, vocab_size, lsc, softcap);
	loop {
		if next == eos || m.hp.eog.contains(&next) {
			break;
		}
		out_ids.push(next);
		let text = tokenizer.decode(&out_ids, true).unwrap_or_default();
		let keep_going = on_round(&[Tok {
			text,
			status: TokStatus::Accepted,
			age: 0,
			heat: 1.0,
		}]);
		if !keep_going || out_ids.len() >= max_new || cache.len >= cache.t_max {
			break;
		}
		forward_rows(m, &[next], attn_scale, &ar, cache, &mut logits, &mut lm_scratch)?;
		next = pick_greedy(&logits, vocab_size, lsc, softcap);
	}
	let elapsed = decode_start.elapsed();
	let body = tokenizer.decode(&out_ids, true).unwrap_or_default();
	let out = format!(
		"{body}\n\n{} tokens, {:.2} tok/s",
		out_ids.len(),
		out_ids.len() as f64 / elapsed.as_secs_f64().max(1e-9)
	);
	drop(ar);
	return Ok(out);
}

pub fn vram_probe_ask() -> Option<i32> {
	let sz = env::var_os("VRAM_PROBE")?;
	if crate::init().is_err() {
		return Some(2);
	}
	let Ok(n) = sz.to_string_lossy().parse::<usize>() else {
		return Some(2);
	};
	Some(match GpuBuffer::try_alloc_bytes(n) {
		Some(_kept) => 0,
		None => 2,
	})
}

/// Probe-verified device-arena claim: spawns a child that tries to map a
/// candidate size, backing off until one succeeds, waits for the child's VRAM
/// to be reclaimed, then claims the slab. Shared by one-shot [`generate`] and
/// the persistent [`ChatSession`] so both take exactly one claim per open.
fn probe_claim() -> Result<Waterfall> {
	let mut want = gpu_core::memory::vram_free_base() & !((1 << 21) - 1);
	Write::line(
		gpu,
		format!("claim guess: {:.2} GB", want as f64 / (1u64 << 30) as f64),
	);
	loop {
		if want < (1 << 30) {
			bail!("claim probe: nothing mappable above 1 GB");
		}
		let status = {
			let mut c = process::Command::new(env::current_exe()?);
			c.stdin(process::Stdio::null());
			c.env("VRAM_PROBE", want.to_string());
			gpu_core::sys::disable_core_dumps(&mut c);
			c.status().context("spawn claim probe")?
		};
		beat();
		if status.success() {
			break;
		}
		Write::line(
			gpu,
			format!(
				"claim probe: {:.2} GB unmappable, backing off",
				want as f64 / (1u64 << 30) as f64
			),
		);
		want -= want / 16;
	}
	Write::line(
		gpu,
		format!(
			"claim: {:.2} GB (probe-verified)",
			want as f64 / (1u64 << 30) as f64
		),
	);
	let need = want + gpu_core::memory::USER_GB;
	let reclaim_start = Instant::now();
	while gpu_core::memory::vram_free_base() < need {
		if reclaim_start.elapsed() > Duration::from_secs(10) {
			Write::line(
				gpu,
				format!(
					"claim: reclaim wait timed out, free={:.2} GB (probe child VRAM not returned)",
					gpu_core::memory::vram_free_base() as f64 / (1u64 << 30) as f64
				),
			);
			break;
		}
		thread::sleep(Duration::from_millis(50));
		beat();
	}
	let slab = gpu_core::memory::claim_device_arena_bytes(want).context("claim device arena")?;
	let w = Waterfall::from_arena(slab);
	Write::line(
		gpu,
		format!("[right after claim] {}", gpu_core::memory::ledger_report()),
	);
	return Ok(w);
}

/// A chat model loaded once and kept resident: [`open`](ChatSession::open) does
/// the single claim + weight load + waterfall fill, then every
/// [`generate_in`](ChatSession::generate_in) call reuses those weights, carving
/// and dropping only a per-message scratch arena. This is what lets a chat run
/// many messages against one load — no re-claim, no re-fill between turns. The
/// weights free once when the session drops.
pub struct ChatSession {
	m: Model,
	tokenizer: crate::tokenizer::Tokenizer,
	vocab: Vec<String>,
	attn_scale: GpuBuffer,
	/// Resident per-layer incremental cache for every token arch (`Some` whenever
	/// `ncanvas == 0`): attention K/V plus recurrent scan/conv state, carried across
	/// turns. `None` only for the diffusion-canvas arches, which decode a fixed canvas.
	cache: KvCache,
}

/// The KV-cache capacity the remaining claim affords, in tokens: no fixed ceiling
/// and no borrowed default — the session holds as many positions as the hardware
/// has room for after the weights land. Measures the per-row cost of the scratch
/// arena with two throwaway probe carves (cost(2 rows) - cost(1 row) is one row;
/// the difference from cost(1) is the row-independent part), then divides the
/// remaining arena by one token's total cost: scratch row + this model's per-layer
/// K/V + the spill staging's two window-granular buffer pairs (widest layer's K
/// and V per window row) — so after the window and worst-case scratch are carved,
/// the remainder IS the staging, by construction. Returns 0 when a single token
/// does not fit, so the caller refuses with sizes instead of silently truncating.
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

/// Elements of K plus V one token occupies across every layer, from the per-layer
/// cache shapes (GQA `nkv*hd`, MLA compressed `kv_lora+rope`/`kv_lora`, or `0` for a
/// recurrent layer). Recurrent layers spend no per-token cache; their fixed state
/// is counted by [`fixed_cache_elems`].
fn kv_row_elems(m: &Model) -> usize {
	let mut n = 0usize;
	for l in 0..m.hp.nl {
		let s = models::layer_cache_shape(m, l);
		n += s.kw + s.vw;
	}
	return n;
}

/// The token-independent cache elements: per-layer recurrent state plus every conv
/// window (allocated twice for the ping-pong). Counted once at capacity time so the
/// per-token divisor sees only the K/V that actually grows with sequence length.
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

/// The KV spill directory: [`gpu_core::tiered::data_dir`], the ONE owner shared
/// with the training OOC spill and — critically — the SAME call the disk-tier
/// budget measurement uses, so the measured free space and the spill bytes are
/// always the same real filesystem (never a tmpfs the RAM tier already counts).
fn kv_spill_dir() -> Result<PathBuf> {
	return gpu_core::tiered::data_dir().context("KV spill dir");
}

/// Rows the RAM + disk waterfall tiers afford PAST the VRAM window, so the cache
/// ceiling is the whole VRAM->RAM->DISK waterfall rather than a fixed VRAM-only
/// t_max. Uses the same [`gpu_core::tiered::Budgets`] measurement as the training
/// out-of-core path (`MemAvailable` and disk free, each minus the `USER_GB`
/// reserve), pointed at [`kv_spill_dir`]; the host K/V pays only the per-row K/V
/// bytes since the scratch arena stays on device.
fn kv_host_tokens(m: &Model) -> Result<usize> {
	let per_row = kv_row_elems(m) * FWD_DT.elem_size();
	if per_row == 0 {
		return Ok(0);
	}
	let b = gpu_core::tiered::Budgets::measure(0, 0, &kv_spill_dir()?);
	return Ok((b.ram_data + b.disk_data) / per_row);
}

/// What [`ChatSession::open`] yields: the resident session, or the named fact
/// that the user cancelled the load — never an Option whose `None` needs a
/// comment to explain.
pub enum Opened {
	Session(Box<ChatSession>),
	Cancelled,
}

impl Opened {
	/// The session, treating a cancel as an error — for callers (tests, one-shot
	/// drivers) that never cancel. Interactive callers match the variants.
	///
	/// # Errors
	/// Returns an error if the load was cancelled.
	pub fn session(self) -> Result<ChatSession> {
		match self {
			Opened::Session(s) => return Ok(*s),
			Opened::Cancelled => return Err(anyhow!("model load cancelled")),
		}
	}
}

impl ChatSession {
	/// Loads `gguf` resident. `load_round` receives empty token snapshots during
	/// the fill so the caller can keep a load UI drawn and cancel; a cancelled
	/// load returns [`Opened::Cancelled`] and frees the claim cleanly.
	pub fn open(
		gguf: &Path,
		load_round: &mut dyn FnMut(&[Tok]) -> bool,
	) -> Result<Opened> {
		if env::var_os("VRAM_PROBE").is_some() || env::var_os("RAM_PROBE").is_some() {
			Write::err(
				"ChatSession: VRAM_PROBE/RAM_PROBE set: the binary's main must call llm::vram_probe_ask() and gpu_core::memory::ram_probe_ask() and exit with the code before any other work",
			)?;
		}
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
		})));
	}

	/// Generates a reply for `prompt` (already chat-templated by the caller) against
	/// the resident weights. For attention arches the per-layer KV cache survives
	/// across calls: the newly templated sequence's token-level common prefix with
	/// the cached ids is detected, the cache is rewound to it, and only the new
	/// suffix runs — so turn N+1 with a long history pays for the new message alone.
	/// Recurrent arches (no cache) recompute the full sequence; diffusion picks the
	/// canvas decode.
	pub fn generate_in(
		&mut self,
		prompt: &str,
		on_round: &mut dyn FnMut(&[Tok]) -> bool,
	) -> Result<String> {
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
				format!("kvcache reuse: cached_prefix={} new_suffix={}", cache.len, toks.len() - cache.len),
			);
			return decode_cached(&self.m, &self.tokenizer, &toks, &self.attn_scale, cache, on_round);
		}
		return decode_canvas(&self.m, &self.vocab, toks, prefix, on_round);
	}
}

impl Drop for ChatSession {
	fn drop(&mut self) {
		if let Some(slab) = self.m.store.take_slab() {
			gpu_core::memory::release_device_arena(slab);
		}
	}
}

/// One-shot generation kept for single-prompt callers (render_once, tests):
/// opens a [`ChatSession`], runs a single message, and drops it. Multi-message
/// callers hold the session and call [`ChatSession::generate_in`] directly.
pub fn generate(
	gguf: &Path,
	prompt: &str,
	on_round: &mut dyn FnMut(&[Tok]) -> bool,
) -> Result<String> {
	let mut session = match ChatSession::open(gguf, on_round)? {
		Opened::Session(s) => *s,
		Opened::Cancelled => return Ok(String::new()),
	};
	return session.generate_in(prompt, on_round);
}

/// Diffusion (canvas) decode against a resident model: iteratively refines the
/// masked canvas region for a fixed number of steps, streaming per-step token
/// snapshots. Carves and drops its own per-message scratch arena.
fn decode_canvas(
	m: &Model,
	vocab: &[String],
	toks: Vec<u32>,
	prefix: usize,
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

	for step in 0..6 {
		let mut base = vec![0.0f64; t * ne];
		for (p, &tk) in toks.iter().enumerate() {
			let b = tk as usize * ne * 2;
			for x in 0..ne {
				base[p * ne + x] =
					bf16(u16::from_le_bytes([m.emb[b + x * 2], m.emb[b + x * 2 + 1]]))
						* scl;
			}
		}
		ar.ha.load(&base)?;

		let coff = prefix * ne;
		let clen = ncanvas * ne;
		if step > 0 {
			let mut soft = vec![0.0f64; ncanvas * ne];
			for (c, top) in sck.iter().enumerate() {
				for &(id, pr) in top {
					let b = id * ne * 2;
					for x in 0..ne {
						soft[c * ne + x] += pr * bf16(u16::from_le_bytes([
							m.emb[b + x * 2],
							m.emb[b + x * 2 + 1],
						]));
					}
				}
				for x in 0..ne {
					soft[c * ne + x] *= scl;
				}
			}
			ar.soft.load(&soft)?;
			gpu_rmsnorm_f64(&ar.soft, &m.sc_pre, &m.eps, ncanvas, ne, &ar.scn)?;
			gpu_gemm_bt_f64(&ar.scn, &m.sc_gate, ncanvas, nff, ne, &ar.sg)?;
			gpu_gemm_bt_f64(&ar.scn, &m.sc_up, ncanvas, nff, ne, &ar.su)?;
			gpu_gelu_mul(&ar.sg, &ar.su, ncanvas * nff, &ar.sa)?;
			gpu_gemm_bt_f64(&ar.sa, &m.sc_down, ncanvas, ne, nff, &ar.sc_add)?;
			gpu_add_into(&ar.ha.view(coff, clen), &ar.sc_add, clen, &ar.cur)?;
			gpu_rmsnorm_f64_nogamma(&ar.cur, &m.eps, ncanvas, ne, &ar.normed)?;
		} else {
			gpu_rmsnorm_f64_nogamma(
				&ar.ha.view(coff, clen),
				&m.eps,
				ncanvas,
				ne,
				&ar.normed,
			)?;
		}
		ar.ha.view(coff, clen).copy_from(&ar.normed, clen * FWD_DT.elem_size())?;

		let bithash = |b: &GpuBuffer, n: usize| -> Result<u64> {
			let mut v = vec![0.0f64; n];
			b.view(0, n).download_host(&mut v)?;
			gpu_core::hip::device_synchronize()?;
			Ok(v.iter().fold(0xcbf29ce484222325u64, |h, x| {
				(h ^ x.to_bits()).wrapping_mul(0x100000001b3)
			}))
		};
		if step == 0 && gpu_core::log::opt().probe {
			Write::line(
				probe_flag,
				format!("[hash] step0 input {:016x}", bithash(&ar.ha, t * ne)?),
			);
		}
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
			layer(m, l, src, dst, t, prefix, &ar)?;
			mem::swap(&mut src, &mut dst);
			if step == 0 && gpu_core::log::opt().probe {
				Write::line(
					probe_flag,
					format!("[hash] step0 layer {l:2} {:016x}", bithash(src, t * ne)?),
				);
			}
		}
		let hbuf = src;
		let mut hbuf_host = vec![0.0f64; hbuf.n_floats()];
		hbuf.download_host(&mut hbuf_host)?;
		gpu_core::hip::device_synchronize()?;
		let nan = hbuf_host.iter().filter(|v| !v.is_finite()).count();
		if nan > 0 {
			Write::err(format!("step {step}: {nan} non-finite in h after layers"))?;
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
				.filter(|&tk| {
					tk >= 6 && Some(tk) != mask_signal && !vocab[tk].starts_with('<')
				})
				.map(|tk| (tk, row[tk]))
				.collect();
			cand.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(cmp::Ordering::Equal));
			cand.truncate(50);
			let ml = cand[0].1;
			let mut probs: Vec<f64> =
				cand.iter().map(|&(_, l)| ((l - ml) / temp).exp()).collect();
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

#[cfg(test)]
mod host_store_tests {
	use super::*;

	/// The waterfall law at the RAM/disk boundary, byte-exact: appends fill the
	/// RAM budget, the first over-budget append routes whole to the spill file,
	/// and a staged read spanning both tiers uploads the exact stored bytes.
	/// Boundary arithmetic: budget 96 B = 3 rows of 8 f32; two 2-row (64 B)
	/// appends -> the second would end at 128 B > 96, so RAM freezes at 64 B
	/// (2 rows) and rows 2..6 live in the file.
	#[test]
	fn ram_boundary_then_disk_roundtrip_and_stage() {
		let es = FWD_DT.elem_size();
		let w = 8usize;
		let path = env::temp_dir().join(format!("recipe-kv-test-{}.spill", process::id()));
		let mut st = HostStore::new(3 * w * es, path.clone());
		let row = |r: usize| -> Vec<f32> {
			return (0..2 * w).map(|i| return (r * 100 + i) as f32 * 0.5).collect();
		};
		st.append_f32(&row(0)).expect("append rows 0-1");
		assert_eq!(st.ram.len(), 2 * w * es, "first append stays in RAM");
		assert_eq!(st.file_len, 0);
		st.append_f32(&row(2)).expect("append rows 2-3");
		assert_eq!(st.ram.len(), 2 * w * es, "RAM frozen at the boundary append");
		assert_eq!(st.file_len, 2 * w * es, "over-budget append went whole to disk");
		assert!(fs::metadata(&path).is_ok(), "spill file exists on disk");
		st.append_f32(&row(4)).expect("append rows 4-5");
		assert_eq!(st.len(), 6 * w * es);
		let mut expect: Vec<f32> = Vec::new();
		for r in [0usize, 2, 4] {
			expect.extend_from_slice(&row(r));
		}
		let dst = GpuBuffer::alloc_ty(6 * w, FWD_DT).expect("stage dst");
		let mut scratch = Vec::new();
		st.stage_into(0, 6 * w * es, &dst, &mut scratch).expect("stage all 6 rows");
		let mut got = vec![0.0f32; 6 * w];
		dst.download_f32(&mut got).expect("download");
		assert_eq!(got, expect, "full-range stage must return the exact stored bytes");
		let dst2 = GpuBuffer::alloc_ty(3 * w, FWD_DT).expect("straddle dst");
		st.stage_into(w * es, 3 * w * es, &dst2, &mut scratch).expect("stage rows 1..4");
		let mut got2 = vec![0.0f32; 3 * w];
		dst2.download_f32(&mut got2).expect("download straddle");
		assert_eq!(got2, expect[w..4 * w], "RAM/file straddling stage must be byte-exact");
		st.append_f32(&row(6)).expect("append rows 6-7 after a read");
		let dst3 = GpuBuffer::alloc_ty(2 * w, FWD_DT).expect("post-append dst");
		st.stage_into(6 * w * es, 2 * w * es, &dst3, &mut scratch)
			.expect("stage rows 6..8 through the cached read descriptor");
		let mut got3 = vec![0.0f32; 2 * w];
		dst3.download_f32(&mut got3).expect("download post-append");
		assert_eq!(
			got3,
			row(6),
			"bytes written through the write descriptor must be visible through the independent read descriptor"
		);
		st.truncate(3 * w * es);
		assert_eq!(st.len(), 3 * w * es);
		assert_eq!(st.file_len, w * es, "truncate above the watermark shrinks only the file");
		st.truncate(w * es);
		assert_eq!(st.file_len, 0, "truncate below the watermark drops the file");
		assert!(fs::metadata(&path).is_err(), "spill file deleted on truncate below RAM");
		st.append_f32(&row(9)).expect("append after truncate");
		assert_eq!(st.ram.len(), 3 * w * es, "RAM regrows to its budget once the file is gone");
		drop(st);
		assert!(fs::metadata(&path).is_err(), "no spill file left after drop");
	}
}
