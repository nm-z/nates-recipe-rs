use anyhow::{Context, Result, anyhow, bail};
use gpu_core::infer_ops::{
	gpu_gelu_mul, gpu_gemm_bt_f64, gpu_glu_gelu, gpu_gqa_attn, gpu_rmsnorm_f64,
	gpu_rmsnorm_f64_nogamma, gpu_rope_partial, gpu_scale_f64_inplace, gpu_widen_bf16,
};
use gpu_core::kernels::gpu_add_into;
use gpu_core::log::probe as probe_flag;
use gpu_core::log::{Write, data, gpu};
use gpu_core::memory::GpuBuffer;
use gpu_core::waterfall::{Home, Waterfall};
use crate::gguf::{Gguf, Val};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct Tok {
	pub text: String,
	pub status: TokStatus,
	pub age: u8,
	pub heat: f32,
}

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
	let v = g.kv.get(key).ok_or_else(|| anyhow!("gguf: kv {key} not found"))?;
	as_uint(v)
		.map(|x| x as usize)
		.ok_or_else(|| anyhow!("gguf: kv {key} is not an unsigned integer"))
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

#[allow(dead_code)]
struct LayerDims {
	hd: usize,
	nkv: usize,
	rotary: usize,
	theta: f64,
	has_v: bool,
	sliding: bool,
}

#[allow(dead_code)]
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
	mask_signal: Option<usize>,
	key_length: usize,
	value_length: usize,
	key_length_swa: usize,
	value_length_swa: usize,
	freq_base: f64,
	freq_base_swa: f64,
	softcap: f64,
	dims: Vec<LayerDims>,
	maxw: usize,
	qd_max: usize,
	kd_max: usize,
	lm_chunk: usize,
	gu_bytes: usize,
	dn_bytes: usize,
	slot_bytes: usize,
}

impl Hparams {
	fn from_gguf(g: &Gguf) -> Result<Hparams> {
		let arch = str_kv(g, "general.architecture")?;
		let k = |s: &str| format!("{arch}.{s}");
		let nl = uint_kv(g, &k("block_count"))?;
		let ne = uint_kv(g, &k("embedding_length"))?;
		let nff = uint_kv(g, &k("feed_forward_length"))?;
		let nffe = uint_kv(g, &k("expert_feed_forward_length"))?;
		let nexp = uint_kv(g, &k("expert_count"))?;
		let used = uint_kv(g, &k("expert_used_count"))?;
		let nqh = uint_kv(g, &k("attention.head_count"))?;
		let key_length = uint_kv(g, &k("attention.key_length"))?;
		let value_length = uint_kv(g, &k("attention.value_length"))?;
		let key_length_swa = uint_kv(g, &k("attention.key_length_swa"))?;
		let value_length_swa = uint_kv(g, &k("attention.value_length_swa"))?;
		let head_count_kv = uint_arr(g, &k("attention.head_count_kv"))?;
		let pattern = bool_arr(g, &k("attention.sliding_window_pattern"))?;
		let freq_base = g.f32_kv(&k("rope.freq_base"))? as f64;
		let freq_base_swa = g.f32_kv(&k("rope.freq_base_swa"))? as f64;
		let eps = g.f32_kv(&k("attention.layer_norm_rms_epsilon"))? as f64;
		let softcap = g.f32_kv(&k("final_logit_softcapping"))? as f64;
		let ncanvas = uint_kv(g, "diffusion.canvas_length")?;
		let bos = uint_kv(g, "tokenizer.ggml.bos_token_id")? as u32;
		let mask = uint_kv(g, "tokenizer.ggml.mask_token_id")?;

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
			let (hd, rotary, theta) = if sliding {
				(key_length_swa, key_length_swa, freq_base_swa)
			} else {
				(key_length, 128, freq_base)
			};
			dims.push(LayerDims {
				hd,
				nkv: head_count_kv[l],
				rotary,
				theta,
				has_v,
				sliding,
			});
		}

		let kd_max = dims.iter().map(|d| d.nkv * d.hd).max().unwrap_or(0);
		let qd_max = nqh * key_length;
		let maxw = nqh * key_length * ne;
		let lm_chunk = maxw / ne;
		let gu_bytes = 2 * nffe * ne * 2;
		let dn_bytes = nffe * ne * 2;
		let slot_bytes = gu_bytes + dn_bytes;

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
			mask_signal,
			key_length,
			value_length,
			key_length_swa,
			value_length_swa,
			freq_base,
			freq_base_swa,
			softcap,
			dims,
			maxw,
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

struct Tensor {
	shard: usize,
	off: usize,
	nbytes: usize,
	shape: Vec<usize>,
	gt: u32,
}

fn bview(buf: &GpuBuffer, off_bytes: usize, len_bytes: usize) -> GpuBuffer {
	assert!(
		off_bytes.is_multiple_of(8) && len_bytes.is_multiple_of(8),
		"bview: unaligned {off_bytes}/{len_bytes}"
	);
	buf.view(off_bytes / 8, len_bytes / 8)
}

static E_VRAM: AtomicU64 = AtomicU64::new(0);
static E_RAM: AtomicU64 = AtomicU64::new(0);
static E_DISK: AtomicU64 = AtomicU64::new(0);

static BEAT: AtomicU64 = AtomicU64::new(0);

fn beat() {
	BEAT.fetch_add(1, Ordering::Relaxed);
}

fn arm_watchdog() -> std::sync::Arc<std::sync::atomic::AtomicBool> {
	let armed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
	let flag = armed.clone();
	std::thread::spawn(move || {
		let mut last = u64::MAX;
		loop {
			std::thread::sleep(std::time::Duration::from_secs(20));
			if !flag.load(std::sync::atomic::Ordering::Relaxed) {
				return;
			}
			let b = BEAT.load(Ordering::Relaxed);
			if b == last {
				Write::err(
					"LOAD WEDGED: no progress for 20s — hipMallocAsync/HSA spin (known driver race). Aborting.",
				);
				std::process::abort();
			}
			last = b;
		}
	});
	armed
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
}

impl Arena {
	fn new(hp: &Hparams, t: usize) -> Result<Arena> {
		let c = hp.ncanvas;
		let ne = hp.ne;
		let nff = hp.nff;
		let nffe = hp.nffe;
		let a = GpuBuffer::alloc;
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
	norms: Vec<HashMap<&'static str, GpuBuffer>>,
	decoder_norm: GpuBuffer,
	sc_pre: GpuBuffer,
	sc_gate: GpuBuffer,
	sc_up: GpuBuffer,
	sc_down: GpuBuffer,
	rw: Vec<Vec<f64>>,
	gis: Vec<Vec<f64>>,
	pe: Vec<Vec<f64>>,
	emb: Vec<u8>,
	eps: GpuBuffer,
	theta_full: GpuBuffer,
	theta_slide: GpuBuffer,
	ls_dev: Vec<GpuBuffer>,
	hp: Hparams,
}

const LAYER_NORMS: [(&str, &str); 9] = [
	("input", "input_layernorm.weight"),
	("post_attn", "post_attention_layernorm.weight"),
	("q_norm", "self_attn.q_norm.weight"),
	("k_norm", "self_attn.k_norm.weight"),
	("pre_ff", "pre_feedforward_layernorm.weight"),
	("pf1", "post_feedforward_layernorm_1.weight"),
	("pn2", "pre_feedforward_layernorm_2.weight"),
	("pf2", "post_feedforward_layernorm_2.weight"),
	("pfw", "post_feedforward_layernorm.weight"),
];

impl Model {
	fn qrange(t: &Tensor, off: usize, len: usize) -> Result<(usize, usize)> {
		let (bb, be) = crate::dequant::block_layout(t.gt);
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
		let mut out = crate::dequant::dequant_bf16(t.gt, &qbuf);
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
		let t = self.big.get(name).ok_or_else(|| anyhow!("missing {name}"))?;
		if t.gt != GT_BF16 {
			let (qoff, qlen) = Self::qrange(t, 0, t.nbytes)?;
			let mut qbuf = vec![0u8; qlen];
			self.shards[t.shard]
				.read_exact_at(&mut qbuf, (t.off + qoff) as u64)
				.with_context(|| format!("small {name}"))?;
			let mut f = Vec::new();
			crate::dequant::dequant_f32(t.gt, &qbuf, &mut f);
			f.truncate(t.nbytes / 2);
			return Ok(f.iter().map(|&x| x as f64).collect());
		}
		let raw = self.read_bytes(t, 0, t.nbytes)?;
		Ok(raw
			.chunks_exact(2)
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

	fn widen_from(&self, src: &GpuBuffer, off_bytes: usize, n: usize) -> GpuBuffer {
		let _w = Instant::now();
		gpu_widen_bf16(&bview(src, off_bytes, n * 2), n, &self.win).expect("widen_bf16 launch");
		acc(&WIDEN_NS, _w);
		self.win.view(0, n)
	}

	fn to_stage(&self, bytes: &[u8]) -> Result<()> {
		let _h = Instant::now();
		self.stage.write_u8(bytes)?;
		acc(&H2D_NS, _h);
		Ok(())
	}

	fn stream(&self, name: &str) -> Result<GpuBuffer> {
		let t = self.big.get(name).ok_or_else(|| anyhow!("missing {name}"))?;
		let n = t.nbytes / 2;
		match self.store.home(name) {
			Some(Home::Vram(dev)) => Ok(self.widen_from(dev, 0, n)),
			Some(Home::Ram(bytes)) => {
				self.to_stage(bytes)?;
				Ok(self.widen_from(&self.stage, 0, n))
			}
			_other => {
				self.read_into(t, 0, t.nbytes, &self.stage, 0)?;
				Ok(self.widen_from(&self.stage, 0, n))
			}
		}
	}

	fn expert_slot(&self, l: usize, e: usize) -> Result<GpuBuffer> {
		let (gu_bytes, dn_bytes, slot_bytes) = (self.hp.gu_bytes, self.hp.dn_bytes, self.hp.slot_bytes);
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
				let gu = self
					.big
					.get(&layer_name(l, "experts.gate_up_proj"))
					.ok_or_else(|| anyhow!("no gate_up {l}"))?;
				let dn = self
					.big
					.get(&layer_name(l, "experts.down_proj"))
					.ok_or_else(|| anyhow!("no down {l}"))?;
				self.read_into(gu, e * gu_bytes, gu_bytes, &self.stage, 0)?;
				self.read_into(dn, e * dn_bytes, dn_bytes, &self.stage, gu_bytes)?;
				Ok(bview(&self.stage, 0, slot_bytes))
			}
		}
	}
}

fn upload_gamma(vals: &[f64], plus_one: bool) -> Result<GpuBuffer> {
	if plus_one {
		let v: Vec<f64> = vals.iter().map(|x| x + 1.0).collect();
		let ub = GpuBuffer::alloc(v.len())?;
		ub.load(&v)?;
		Ok(ub)
	} else {
		let ub = GpuBuffer::alloc(vals.len())?;
		ub.load(vals)?;
		Ok(ub)
	}
}

fn hf_name(gg: &str) -> Option<String> {
	let global = |n: &str| Some(format!("model.decoder.{n}"));
	match gg {
		"token_embd.weight" => return global("embed_tokens.weight"),
		"output_norm.weight" => return global("norm.weight"),
		"self_cond_pre_norm.weight" => return global("self_conditioning.pre_norm.weight"),
		"self_cond_gate.weight" => return global("self_conditioning.gate_proj.weight"),
		"self_cond_up.weight" => return global("self_conditioning.up_proj.weight"),
		"self_cond_down.weight" => return global("self_conditioning.down_proj.weight"),
		_other => {}
	}
	let rest = gg.strip_prefix("blk.")?;
	let dot = rest.find('.')?;
	let l: usize = rest[..dot].parse().ok()?;
	let suf = match &rest[dot + 1..] {
		"attn_norm.weight" => "input_layernorm.weight",
		"post_attention_norm.weight" => "post_attention_layernorm.weight",
		"attn_q_norm.weight" => "self_attn.q_norm.weight",
		"attn_k_norm.weight" => "self_attn.k_norm.weight",
		"ffn_norm.weight" => "pre_feedforward_layernorm.weight",
		"post_ffw_norm_1.weight" => "post_feedforward_layernorm_1.weight",
		"pre_ffw_norm_2.weight" => "pre_feedforward_layernorm_2.weight",
		"post_ffw_norm_2.weight" => "post_feedforward_layernorm_2.weight",
		"post_ffw_norm.weight" => "post_feedforward_layernorm.weight",
		"attn_q.weight" => "self_attn.q_proj.weight",
		"attn_k.weight" => "self_attn.k_proj.weight",
		"attn_v.weight" => "self_attn.v_proj.weight",
		"attn_output.weight" => "self_attn.o_proj.weight",
		"ffn_gate.weight" => "mlp.gate_proj.weight",
		"ffn_up.weight" => "mlp.up_proj.weight",
		"ffn_down.weight" => "mlp.down_proj.weight",
		"ffn_gate_up_exps.weight" => "experts.gate_up_proj",
		"ffn_down_exps.weight" => "experts.down_proj",
		"ffn_gate_inp.weight" => "router.proj.weight",
		"ffn_gate_inp.scale" => "router.scale",
		"ffn_down_exps.scale" => "router.per_expert_scale",
		"layer_output_scale.weight" => "layer_scalar",
		_other => return None,
	};
	Some(format!("model.decoder.layers.{l}.{suf}"))
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
	finish_load(vec![f], big, hp)
}

fn finish_load(shards: Vec<File>, big: HashMap<String, Tensor>, hp: Hparams) -> Result<Model> {
	Write::line(data, "allocating stage+win");
	let eps = {
		let ub = GpuBuffer::alloc(1)?;
		ub.load(&[hp.eps])?;
		ub
	};
	let theta_full = {
		let ub = GpuBuffer::alloc(1)?;
		ub.load(&[hp.freq_base])?;
		ub
	};
	let theta_slide = {
		let ub = GpuBuffer::alloc(1)?;
		ub.load(&[hp.freq_base_swa])?;
		ub
	};
	let maxw = hp.maxw;
	let nl = hp.nl;
	let vocab = hp.vocab;
	let ne = hp.ne;
	let mut m = Model {
		shards,
		big,
		stage: GpuBuffer::alloc_bytes(maxw * 2)?,
		win: GpuBuffer::alloc(maxw)?,
		store: Waterfall::new(),
		rbuf: RefCell::new(Vec::new()),
		norms: Vec::new(),
		decoder_norm: GpuBuffer::alloc(1)?,
		sc_pre: GpuBuffer::alloc(1)?,
		sc_gate: GpuBuffer::alloc(1)?,
		sc_up: GpuBuffer::alloc(1)?,
		sc_down: GpuBuffer::alloc(1)?,
		rw: Vec::new(),
		gis: Vec::new(),
		pe: Vec::new(),
		emb: Vec::new(),
		eps,
		theta_full,
		theta_slide,
		ls_dev: Vec::new(),
		hp,
	};

	let probe = m.small_f64("model.decoder.layers.0.input_layernorm.weight")?;
	let mean = probe.iter().sum::<f64>() / probe.len() as f64;
	let plus_one = mean.abs() < 0.5;
	Write::line(
		data,
		format!(
			"norm probe mean={mean:.4} -> {}",
			if plus_one {
				"(1+w) HF convention"
			} else {
				"folded x*w"
			}
		),
	);

	for l in 0..nl {
		Write::line(data, format!("norms layer {}/{}", l + 1, nl));
		beat();
		let p = |n: &str| format!("model.decoder.layers.{l}.{n}");
		let mut nm = HashMap::new();
		for (key, suffix) in LAYER_NORMS {
			nm.insert(key, upload_gamma(&m.small_f64(&p(suffix))?, plus_one)?);
		}
		m.norms.push(nm);
		m.rw.push(m.small_f64(&p("router.proj.weight"))?);
		m.gis.push(m.small_f64(&p("router.scale"))?);
		m.pe.push(m.small_f64(&p("router.per_expert_scale"))?);
		let lsv = m.small_f64(&p("layer_scalar"))?[0];
		m.ls_dev.push({
			let ub = GpuBuffer::alloc(1)?;
			ub.load(&[lsv])?;
			ub
		});
	}

	Write::line(data, "globals + embedding table");
	m.decoder_norm = upload_gamma(&m.small_f64("model.decoder.norm.weight")?, plus_one)?;
	m.sc_pre = upload_gamma(
		&m.small_f64("model.decoder.self_conditioning.pre_norm.weight")?,
		plus_one,
	)?;
	m.sc_gate = {
		let vals = m.small_f64("model.decoder.self_conditioning.gate_proj.weight")?;
		let ub = GpuBuffer::alloc(vals.len())?;
		ub.load(&vals)?;
		ub
	};
	m.sc_up = {
		let vals = m.small_f64("model.decoder.self_conditioning.up_proj.weight")?;
		let ub = GpuBuffer::alloc(vals.len())?;
		ub.load(&vals)?;
		ub
	};
	m.sc_down = {
		let vals = m.small_f64("model.decoder.self_conditioning.down_proj.weight")?;
		let ub = GpuBuffer::alloc(vals.len())?;
		ub.load(&vals)?;
		ub
	};

	let et = m
		.big
		.get("model.decoder.embed_tokens.weight")
		.ok_or_else(|| anyhow!("no embed_tokens"))?;
	if et.shape != vec![vocab, ne] {
		bail!("embed_tokens shape {:?}", et.shape);
	}
	m.emb = m.read_bytes(et, 0, et.nbytes)?;

	Ok(m)
}

fn fixed_names(hp: &Hparams, l: usize) -> Vec<String> {
	let mut names = vec![
		layer_name(l, "self_attn.q_proj.weight"),
		layer_name(l, "self_attn.k_proj.weight"),
		layer_name(l, "self_attn.o_proj.weight"),
		layer_name(l, "mlp.gate_proj.weight"),
		layer_name(l, "mlp.up_proj.weight"),
		layer_name(l, "mlp.down_proj.weight"),
	];
	if hp.dims[l].has_v {
		names.push(layer_name(l, "self_attn.v_proj.weight"));
	}
	names
}

fn preflight(m: &Model, ar: &Arena, t: usize) -> Result<()> {
	let hp = &m.hp;
	gpu_gemm_bt_f64(&ar.x, &m.win.view(0, hp.qd_max * hp.ne), t, hp.qd_max, hp.ne, &ar.q)?;
	beat();
	gpu_gemm_bt_f64(&ar.cms, &m.win.view(0, hp.nff * hp.ne), t, hp.nff, hp.ne, &ar.g)?;
	beat();
	gpu_gemm_bt_f64(&ar.act, &m.win.view(0, hp.ne * hp.nff), t, hp.ne, hp.nff, &ar.mlp0)?;
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

fn fill_store(m: &mut Model, store: Waterfall) -> Result<()> {
	let mut store = store;
	let nl = m.hp.nl;
	let nexp = m.hp.nexp;
	let gu_bytes = m.hp.gu_bytes;
	let dn_bytes = m.hp.dn_bytes;
	let slot_bytes = m.hp.slot_bytes;
	beat();
	store.place("model.decoder.embed_tokens.weight", m.emb.len(), |dst| {
		dst.copy_from_slice(&m.emb);
		Ok(())
	})?;
	beat();
	for l in 0..nl {
		for name in fixed_names(&m.hp, l) {
			let t = m.big.get(&name).ok_or_else(|| anyhow!("missing {name}"))?;
			store.place(&name, t.nbytes, |dst| {
				m.read_host(t, 0, dst).map_err(std::io::Error::other)
			})?;
			beat();
		}
	}
	for e in 0..nexp {
		for l in 0..nl {
			let gu = m
				.big
				.get(&layer_name(l, "experts.gate_up_proj"))
				.ok_or_else(|| anyhow!("no gate_up {l}"))?;
			let dn = m
				.big
				.get(&layer_name(l, "experts.down_proj"))
				.ok_or_else(|| anyhow!("no down {l}"))?;
			store.place(&ekey(l, e), slot_bytes, |dst| {
				m.read_host(gu, e * gu_bytes, &mut dst[..gu_bytes])
					.and_then(|_g| m.read_host(dn, e * dn_bytes, &mut dst[gu_bytes..]))
					.map_err(std::io::Error::other)
			})?;
			beat();
		}
	}
	store.report();
	m.store = store;

	for name in [
		"model.decoder.embed_tokens.weight".to_string(),
		fixed_names(&m.hp, 0).remove(0),
		fixed_names(&m.hp, nl - 1)
			.pop()
			.ok_or_else(|| anyhow!("no names"))?,
	] {
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
					bail!("waterfall {name} stale at byte {off}: upload not visible to GPU reads");
				}
			}
		}
	}
	Ok(())
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
	let nm = &m.norms[l];
	let d = &hp.dims[l];
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, nqh * d.hd, d.nkv * d.hd);
	let theta = if d.sliding {
		&m.theta_slide
	} else {
		&m.theta_full
	};
	let _ta = Instant::now();
	gpu_rmsnorm_f64(h_in, &nm["input"], &m.eps, t, ne, &ar.x)?;
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
	gpu_rmsnorm_f64(&ar.q, &nm["q_norm"], &m.eps, t * nqh, hd, &ar.q)?;
	gpu_rmsnorm_f64(&ar.k, &nm["k_norm"], &m.eps, t * nkv, hd, &ar.k)?;
	gpu_rmsnorm_f64_nogamma(&ar.v, &m.eps, t * nkv, hd, &ar.v)?;
	gpu_rope_partial(theta, t * nqh, hd, d.rotary, nqh, &ar.q)?;
	gpu_rope_partial(theta, t * nkv, hd, d.rotary, nkv, &ar.k)?;
	gpu_gqa_attn(&ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, prefix, &ar.attn)?;
	gpu_gemm_bt_f64(
		&ar.attn,
		&m.stream(&layer_name(l, "self_attn.o_proj.weight"))?,
		t,
		ne,
		qd,
		&ar.o,
	)?;
	gpu_rmsnorm_f64(&ar.o, &nm["post_attn"], &m.eps, t, ne, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	acc(&ATTN_NS, _ta);

	let _tm = Instant::now();
	gpu_rmsnorm_f64(&ar.attn_out, &nm["pre_ff"], &m.eps, t, ne, &ar.cms)?;
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
	gpu_rmsnorm_f64(&ar.mlp0, &nm["pf1"], &m.eps, t, ne, &ar.mlp)?;
	acc(&MLP_NS, _tm);

	let _tmoe = Instant::now();
	gpu_rmsnorm_f64(&ar.attn_out, &nm["pn2"], &m.eps, t, ne, &ar.cmoes)?;
	let _rt = Instant::now();
	let mut ao_host = vec![0.0f64; ar.attn_out.n_floats()];
	let mut cmoes_host = vec![0.0f64; ar.cmoes.n_floats()];
	unsafe {
		ar.attn_out.download_async(&mut ao_host, std::ptr::null_mut())
	}?;
	unsafe {
		ar.cmoes.download_async(&mut cmoes_host, std::ptr::null_mut())
	}?;
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
		idx.sort_by(|a, b| {
			rl[*b].partial_cmp(&rl[*a]).unwrap_or(std::cmp::Ordering::Equal)
		});
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
		let gu_w = m.widen_from(&es, 0, 2 * nffe * ne);
		gpu_gemm_bt_f64(&ar.moe_xg, &gu_w, np, 2 * nffe, ne, &ar.moe_gu)?;
		gpu_glu_gelu(&ar.moe_gu, np, nffe, &ar.moe_ea)?;
		let dn_w = m.widen_from(&es, hp.gu_bytes, ne * nffe);
		gpu_gemm_bt_f64(&ar.moe_ea, &dn_w, np, ne, nffe, &ar.moe_dv)?;
		let _rt = Instant::now();
		unsafe {
			ar.moe_dv.download_async(&mut dv_host[..np * ne], std::ptr::null_mut())
		}?;
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
	gpu_rmsnorm_f64(&ar.mo, &nm["pf2"], &m.eps, t, ne, &ar.mop)?;
	acc(&MOE_NS, _tmoe);

	gpu_add_into(&ar.mlp, &ar.mop, t * ne, &ar.comb)?;
	gpu_rmsnorm_f64(&ar.comb, &nm["pfw"], &m.eps, t, ne, &ar.comb)?;
	gpu_add_into(&ar.attn_out, &ar.comb, t * ne, h_out)?;
	gpu_scale_f64_inplace(&m.ls_dev[l], t * ne, h_out)?;
	Ok(())
}

fn layer_name(l: usize, suffix: &str) -> String {
	format!("model.decoder.layers.{l}.{suffix}")
}

fn lm_head(m: &Model, hfs: &GpuBuffer, ncanvas: usize, ar: &Arena) -> Result<Vec<f64>> {
	let hp = &m.hp;
	let _tl = Instant::now();
	let mut logits = vec![0.0f64; ncanvas * hp.vocab];
	let mut out_host = vec![0.0f64; ncanvas * hp.lm_chunk];
	let mut c0 = 0;
	while c0 < hp.vocab {
		let cn = hp.lm_chunk.min(hp.vocab - c0);
		let w = match m.store.home("model.decoder.embed_tokens.weight") {
			Some(Home::Vram(dev)) => m.widen_from(dev, c0 * hp.ne * 2, cn * hp.ne),
			_other => {
				m.to_stage(&m.emb[c0 * hp.ne * 2..(c0 + cn) * hp.ne * 2])?;
				m.widen_from(&m.stage, 0, cn * hp.ne)
			}
		};
		gpu_gemm_bt_f64(hfs, &w, ncanvas, cn, hp.ne, &ar.lm_out)?;
		unsafe {
			ar.lm_out.download_async(&mut out_host[..ncanvas * cn], std::ptr::null_mut())
		}?;
		gpu_core::hip::device_synchronize()?;
		for p in 0..ncanvas {
			logits[p * hp.vocab + c0..p * hp.vocab + c0 + cn]
				.copy_from_slice(&out_host[p * cn..(p + 1) * cn]);
		}
		c0 += cn;
	}
	acc(&LM_NS, _tl);
	Ok(logits)
}

pub fn vram_probe_gate() {
	let Some(sz) = std::env::var_os("VRAM_PROBE") else {
		return;
	};
	if crate::init().is_err() {
		std::process::exit(2);
	}
	let n: usize = match sz.to_string_lossy().parse() {
		Ok(n) => n,
		Err(_e) => std::process::exit(2),
	};
	std::process::exit(match GpuBuffer::try_alloc_bytes(n) {
		Some(_kept) => 0,
		None => 2,
	});
}

pub fn generate(
	gguf: &Path,
	prompt: &str,
	on_round: &mut dyn FnMut(&[Tok]),
) -> Result<String> {
	vram_probe_gate();
	crate::init().map_err(|e| anyhow!("gpu init: {e:?}"))?;

	let t_load = Instant::now();
	let watchdog = arm_watchdog();
	let claim = {
		let mut want = gpu_core::memory::vram_free_base() & !((1 << 21) - 1);
		Write::line(gpu, format!("claim guess: {:.2} GB", want as f64 / (1u64 << 30) as f64));
		loop {
			if want < (1 << 30) {
				bail!("claim probe: nothing mappable above 1 GB");
			}
			let status = {
				let mut c = std::process::Command::new(std::env::current_exe()?);
				c.env("VRAM_PROBE", want.to_string());
				unsafe {
					c.pre_exec(|| {
						let z = libc::rlimit {
							rlim_cur: 0,
							rlim_max: 0,
						};
						libc::setrlimit(libc::RLIMIT_CORE, &z);
						Ok(())
					});
				}
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
			format!("claim: {:.2} GB (probe-verified)", want as f64 / (1u64 << 30) as f64),
		);
		let slab = gpu_core::memory::claim_device_arena_bytes(want).context("claim device arena")?;
		let w = Waterfall::from_arena(slab);
		Write::line(gpu, format!("[right after claim] {}", gpu_core::memory::ledger_report()));
		w
	};

	beat();
	let mut m = load_model_gguf(gguf)?;

	let (tokenizer, vocab) = {
		let g = Gguf::open(gguf)?;
		(
			crate::tokenizer::from_gguf(&g)?,
			crate::tokenizer::gguf_vocab(&g, m.hp.vocab)?,
		)
	};

	let ne = m.hp.ne;
	let nff = m.hp.nff;
	let ncanvas = m.hp.ncanvas;
	let nl = m.hp.nl;
	let mask = m.hp.mask;
	let vocab_size = m.hp.vocab;
	let mask_signal = m.hp.mask_signal;

	let enc = tokenizer
		.encode(prompt, false)
		.map_err(|e| anyhow!("tokenize: {e}"))?;
	let mut toks = vec![m.hp.bos];
	toks.extend_from_slice(enc.get_ids());
	let prefix = toks.len();
	for _ in 0..ncanvas {
		toks.push(mask as u32);
	}
	let t = toks.len();
	let scl = (ne as f64).sqrt();
	Write::line(data, format!("prompt tokens={prefix} canvas={ncanvas} total={t}"));

	let ar = {
		let _t = gpu_core::memory::tag_scope("arena");
		Arena::new(&m.hp, t)?
	};
	Write::line(data, "preflight gemms");
	preflight(&m, &ar, t)?;
	Write::line(data, "waterfall fill");
	fill_store(&mut m, claim)?;
	watchdog.store(false, Ordering::Relaxed);
	Write::line(gpu, format!("loaded in {:.1}s", t_load.elapsed().as_secs_f64()));
	Write::line(
		gpu,
		format!(
			"hparams arch={} nl={} ne={} experts={} canvas={} vocab={}",
			m.hp.arch, m.hp.nl, m.hp.ne, m.hp.nexp, m.hp.ncanvas, m.hp.vocab
		),
	);

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
					bf16(u16::from_le_bytes([m.emb[b + x * 2], m.emb[b + x * 2 + 1]])) * scl;
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
						soft[c * ne + x] +=
							pr * bf16(u16::from_le_bytes([m.emb[b + x * 2], m.emb[b + x * 2 + 1]]));
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
			gpu_rmsnorm_f64_nogamma(&ar.ha.view(coff, clen), &m.eps, ncanvas, ne, &ar.normed)?;
		}
		ar.ha.view(coff, clen).copy_from(&ar.normed, clen * 8)?;

		let bithash = |b: &GpuBuffer, n: usize| -> Result<u64> {
			let mut v = vec![0.0f64; n];
			unsafe { b.view(0, n).download_async(&mut v, std::ptr::null_mut()) }?;
			gpu_core::hip::device_synchronize()?;
			Ok(v.iter().fold(0xcbf29ce484222325u64, |h, x| {
				(h ^ x.to_bits()).wrapping_mul(0x100000001b3)
			}))
		};
		if step == 0 && gpu_core::log::opt().probe {
			Write::line(probe_flag, format!("[hash] step0 input {:016x}", bithash(&ar.ha, t * ne)?));
		}
		let mut src: &GpuBuffer = &ar.ha;
		let mut dst: &GpuBuffer = &ar.hb;
		for l in 0..nl {
			Write::line(
				gpu,
				format!("step {step} layer {}/{} ({:.0}s)", l + 1, nl, t0.elapsed().as_secs_f64()),
			);
			layer(&m, l, src, dst, t, prefix, &ar)?;
			std::mem::swap(&mut src, &mut dst);
			if step == 0 && gpu_core::log::opt().probe {
				Write::line(probe_flag, format!("[hash] step0 layer {l:2} {:016x}", bithash(src, t * ne)?));
			}
		}
		let hbuf = src;
		let mut hbuf_host = vec![0.0f64; hbuf.n_floats()];
		unsafe { hbuf.download_async(&mut hbuf_host, std::ptr::null_mut()) }?;
		gpu_core::hip::device_synchronize()?;
		let nan = hbuf_host.iter().filter(|v| !v.is_finite()).count();
		if nan > 0 {
			Write::err(format!("step {step}: {nan} non-finite in h after layers"));
			bail!("step {step}: {nan} non-finite in h after layers");
		}

		gpu_rmsnorm_f64(&hbuf.view(coff, clen), &m.decoder_norm, &m.eps, ncanvas, ne, &ar.hfs)?;
		let logits = lm_head(&m, &ar.hfs, ncanvas, &ar)?;

		let temp = 1.0 - 0.7 * (step as f64 / 6.0);
		for c in 0..ncanvas {
			let row = &logits[c * vocab_size..(c + 1) * vocab_size];
			let mut cand: Vec<(usize, f64)> = (0..vocab_size)
				.filter(|&tk| tk >= 6 && Some(tk) != mask_signal && !vocab[tk].starts_with('<'))
				.map(|tk| (tk, row[tk]))
				.collect();
			cand.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
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
				let tk = pred[c] as usize;
				let status = if tk == mask || Some(tk) == mask_signal {
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
		on_round(&toks_ui);
	}

	let allocs_after = gpu_core::memory::device_alloc_count();
	Write::line(gpu, format!("steady-state allocs: {}", allocs_after - allocs_before));
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
		.map(|&tk| vocab[tk as usize].replace('\u{2581}', " "))
		.collect();

	drop(ar);
	drop(m);
	Write::line(gpu, format!("exit: device frees {}", gpu_core::memory::device_free_count()));
	Ok(out)
}
