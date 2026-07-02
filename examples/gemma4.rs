// GPU f64 inference for diffusion-gemma-26B-A4B (block-diffusion, text path).
// Weights are bf16 safetensors (52GB); the big projections/experts stream from
// disk and are widened bf16->f64 into a VRAM window right before each hipBLAS
// GEMM (f64 throughout, no quant). Small tensors (norms, router, self-cond) are
// widened once at load and kept resident. The embedding table stays as host
// bf16 for lookups, the soft self-conditioning sum, and the LM head. Diffusion
// sampler (self-conditioning + temperature/xorshift) mirrors the CPU oracle
// (~/Desktop/gemma4/rustgemma/gfO.rs) so intermediate magnitudes are comparable.
//
//   cargo run --release --example gemma4 -- "The capital of France is"

use anyhow::{Context, Result, anyhow, bail};
use gpu_core::infer_ops::{
	gpu_gelu_mul_into, gpu_glu_gelu_into, gpu_gqa_attn_into, gpu_rmsnorm_f64_into,
	gpu_rope_partial, gpu_widen_bf16_into,
};
use gpu_core::kernels::{gpu_add_into, gpu_gemm_bt_into, gpu_scale_inplace};
use gpu_core::memory::GpuBuffer;
use gpu_core::waterfall::{Home, Waterfall};
use recipe_infer::safetensors::parse_safetensors_header;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

// DEBUG timing accumulators (ns): disk read, H2D upload (write_u8), widen kernel.
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

// ── config (config.json) ─────────────────────────────────────────────────────
const NE: usize = 2816; // hidden
const NL: usize = 30; // layers
const NQH: usize = 16; // q heads (both layer types)
const NFF: usize = 2112; // shared mlp intermediate
const NFFE: usize = 704; // expert intermediate
const NEXP: usize = 128; // experts
const USED: usize = 8; // top-k experts
const VOCAB: usize = 262144;
const EPS: f64 = 1e-6;
const NCANVAS: usize = 48;
const MASK: usize = 4; // <mask>
const BOS: u32 = 2; // <bos>
const MASK_SIGNAL: usize = 242122; // "掩" — emitted for still-masked positions

// bf16 half -> f64 (host side; exact pad).
fn bf16(h: u16) -> f64 {
	f32::from_bits((h as u32) << 16) as f64
}

// Per-layer attention geometry. Every 6th layer (5,11,17,23,29) is a full
// attention layer: head_dim 512, 2 kv-heads, partial rotary (128 of 512),
// theta 1e6, and NO v_proj (v reuses k_proj). Sliding layers: head_dim 256,
// 8 kv-heads, full rotary (256), theta 1e4, separate v_proj.
struct Dims {
	hd: usize,
	nkv: usize,
	rotary: usize,
	theta: f64,
	has_v: bool,
}
fn dims(l: usize) -> Dims {
	if l % 6 == 5 {
		Dims { hd: 512, nkv: 2, rotary: 128, theta: 1_000_000.0, has_v: false }
	} else {
		Dims { hd: 256, nkv: 8, rotary: 256, theta: 10_000.0, has_v: true }
	}
}

struct Tensor {
	shard: usize,
	off: usize, // absolute file byte offset of the blob start
	nbytes: usize,
	shape: Vec<usize>,
}

// Largest single streamed weight: full-layer q_proj / o_proj = 8192*2816 f64.
const MAXW: usize = 8192 * 2816;

// Max per-layer projection widths across both attention geometries: q/attn use
// full-layer qd = NQH*512 = 8192; k/v use the sliding-layer kd = 8*256 = 2048
// (wider than the full layer's 2*512). Arena activation buffers size to these.
const QD_MAX: usize = NQH * 512;
const KD_MAX: usize = 8 * 256;
// LM-head vocab tile: cn*NE f64 must fit the reused widen window (MAXW), and
// cn*NE*2 bf16 bytes must fit the staging buffer (MAXW*2) — 8192*NE hits both exactly.
const LM_CHUNK: usize = 8192;

// One expert's bf16 bytes: gate_up (2*NFFE, NE) followed by down (NE, NFFE).
const GU_BYTES: usize = 2 * NFFE * NE * 2;
const DN_BYTES: usize = NFFE * NE * 2;
const SLOT_BYTES: usize = GU_BYTES + DN_BYTES;

// Byte-granular device view (GpuBuffer::view is f64-granular; all our bf16
// offsets are multiples of NE*2 = 5632, so /8 is exact — asserted).
fn bview(buf: &GpuBuffer, off_bytes: usize, len_bytes: usize) -> GpuBuffer {
	assert!(off_bytes % 8 == 0 && len_bytes % 8 == 0, "bview: unaligned {off_bytes}/{len_bytes}");
	buf.view(off_bytes / 8, len_bytes / 8)
}

// Per-step tier hit counters (where expert bytes came from).
static E_VRAM: AtomicU64 = AtomicU64::new(0);
static E_RAM: AtomicU64 = AtomicU64::new(0);
static E_DISK: AtomicU64 = AtomicU64::new(0);

// Load watchdog: hipMallocAsync pool growth stochastically wedges in an HSA
// spin on this driver (gdb-verified). Every load phase bumps the beat; if it
// stalls for 20s the process dies LOUDLY instead of spinning silently forever.
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
				eprintln!("\nLOAD WEDGED: no progress for 20s — hipMallocAsync/HSA spin (known driver race). Aborting.");
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

// Every device buffer the steady-state forward touches, allocated once at
// `Arena::new` (after `t` is known) and reused across all 30 layers and 6 steps.
// Sized to the maximum shape each buffer ever holds; the hot loop writes into the
// leading `used`-element window via the `_into` ops, so no op ever allocates.
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
	ha: GpuBuffer, // ping-pong hidden state A (also holds the per-step base + canvas)
	hb: GpuBuffer, // ping-pong hidden state B
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
	fn new(t: usize) -> Result<Arena> {
		let c = NCANVAS;
		let a = GpuBuffer::alloc;
		Ok(Arena {
			x: a(t * NE)?,
			q: a(t * QD_MAX)?,
			k: a(t * KD_MAX)?,
			v: a(t * KD_MAX)?,
			attn: a(t * QD_MAX)?,
			o: a(t * NE)?,
			attn_out: a(t * NE)?,
			cms: a(t * NE)?,
			g: a(t * NFF)?,
			u: a(t * NFF)?,
			act: a(t * NFF)?,
			mlp0: a(t * NE)?,
			mlp: a(t * NE)?,
			cmoes: a(t * NE)?,
			moe_xg: a(t * NE)?,
			moe_gu: a(t * 2 * NFFE)?,
			moe_ea: a(t * NFFE)?,
			moe_dv: a(t * NE)?,
			mo: a(t * NE)?,
			mop: a(t * NE)?,
			comb: a(t * NE)?,
			ha: a(t * NE)?,
			hb: a(t * NE)?,
			soft: a(c * NE)?,
			scn: a(c * NE)?,
			sg: a(c * NFF)?,
			su: a(c * NFF)?,
			sa: a(c * NFF)?,
			sc_add: a(c * NE)?,
			cur: a(c * NE)?,
			normed: a(c * NE)?,
			hfs: a(c * NE)?,
			lm_out: a(c * LM_CHUNK)?,
		})
	}
}

struct Model {
	shards: Vec<File>,
	big: HashMap<String, Tensor>, // streamed (bf16 on disk)
	stage: GpuBuffer,             // reusable device bf16 staging (MAXW*2 bytes)
	win: GpuBuffer,               // reusable f64 widen window (MAXW floats)
	store: Waterfall,             // VRAM→RAM→DISK home of every big weight blob
	rbuf: RefCell<Vec<u8>>,       // reusable disk read buffer (no per-read alloc/zero)
	// resident f64 on GPU, per layer
	norms: Vec<HashMap<&'static str, GpuBuffer>>,
	decoder_norm: GpuBuffer,
	sc_pre: GpuBuffer,
	sc_gate: GpuBuffer,
	sc_up: GpuBuffer,
	sc_down: GpuBuffer,
	// resident f64 on host, per layer
	rw: Vec<Vec<f64>>,  // router.proj (NEXP,NE)
	gis: Vec<Vec<f64>>, // router.scale (NE,)
	pe: Vec<Vec<f64>>,  // router.per_expert_scale (NEXP,)
	ls: Vec<f64>,       // layer_scalar
	emb: Vec<u8>,       // embed_tokens bf16 bytes (VOCAB*NE*2)
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
	// Read a tensor's raw bytes (or a byte sub-range) from its shard.
	fn read_bytes(&self, t: &Tensor, off: usize, len: usize) -> Result<Vec<u8>> {
		let mut buf = vec![0u8; len];
		let _d = Instant::now();
		self.shards[t.shard]
			.read_exact_at(&mut buf, (t.off + off) as u64)
			.with_context(|| format!("read {len} bytes at shard {}", t.shard))?;
		acc(&DISK_NS, _d);
		Ok(buf)
	}

	// Disk → device through the reusable host buffer: no per-read alloc/zero.
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
		Ok(())
	}

	// Decode a small tensor fully to host f64.
	fn small_f64(&self, name: &str) -> Result<Vec<f64>> {
		let t = self.big.get(name).ok_or_else(|| anyhow!("missing {name}"))?;
		let raw = self.read_bytes(t, 0, t.nbytes)?;
		Ok(raw.chunks_exact(2).map(|c| bf16(u16::from_le_bytes([c[0], c[1]]))).collect())
	}

	// Widen `n` bf16 elems at `off_bytes` of a device bf16 buffer into the shared
	// f64 window; returns a borrow view. The caller must consume it before the
	// next widen (stream-ordered, so enqueued GEMMs read it first).
	fn widen_from(&self, src: &GpuBuffer, off_bytes: usize, n: usize) -> GpuBuffer {
		let _w = Instant::now();
		gpu_widen_bf16_into(&bview(src, off_bytes, n * 2), &self.win, n);
		acc(&WIDEN_NS, _w);
		self.win.view(0, n)
	}

	// Host bytes → stage, timed as H2D.
	fn to_stage(&self, bytes: &[u8]) -> Result<()> {
		let _h = Instant::now();
		self.stage.write_u8(bytes)?;
		acc(&H2D_NS, _h);
		Ok(())
	}

	// Whole-tensor weight through the waterfall: VRAM home widens in place, RAM
	// home bounces to stage, DISK home streams the shard.
	fn stream(&self, name: &str) -> Result<GpuBuffer> {
		let t = self.big.get(name).ok_or_else(|| anyhow!("missing {name}"))?;
		let n = t.nbytes / 2;
		match self.store.home(name) {
			Some(Home::Vram(dev)) => Ok(self.widen_from(dev, 0, n)),
			Some(Home::Ram(bytes)) => {
				self.to_stage(bytes)?;
				Ok(self.widen_from(&self.stage, 0, n))
			}
			_ => {
				self.read_into(t, 0, t.nbytes, &self.stage, 0)?;
				Ok(self.widen_from(&self.stage, 0, n))
			}
		}
	}

	// One expert's bf16 bytes (gate_up ‖ down) as a device view, same three tiers.
	fn expert_slot(&self, l: usize, e: usize) -> Result<GpuBuffer> {
		match self.store.home(&ekey(l, e)) {
			Some(Home::Vram(dev)) => {
				E_VRAM.fetch_add(1, Ordering::Relaxed);
				Ok(bview(dev, 0, SLOT_BYTES))
			}
			Some(Home::Ram(bytes)) => {
				E_RAM.fetch_add(1, Ordering::Relaxed);
				self.to_stage(bytes)?;
				Ok(bview(&self.stage, 0, SLOT_BYTES))
			}
			_ => {
				E_DISK.fetch_add(1, Ordering::Relaxed);
				let gu = self.big.get(&layer_name(l, "experts.gate_up_proj")).ok_or_else(|| anyhow!("no gate_up {l}"))?;
				let dn = self.big.get(&layer_name(l, "experts.down_proj")).ok_or_else(|| anyhow!("no down {l}"))?;
				self.read_into(gu, e * GU_BYTES, GU_BYTES, &self.stage, 0)?;
				self.read_into(dn, e * DN_BYTES, DN_BYTES, &self.stage, GU_BYTES)?;
				Ok(bview(&self.stage, 0, SLOT_BYTES))
			}
		}
	}
}

fn upload_gamma(vals: &[f64], plus_one: bool) -> Result<GpuBuffer> {
	if plus_one {
		let v: Vec<f64> = vals.iter().map(|x| x + 1.0).collect();
		Ok(GpuBuffer::upload(&v)?)
	} else {
		Ok(GpuBuffer::upload(vals)?)
	}
}

fn load_model(dir: &PathBuf) -> Result<Model> {
	let index: serde_json::Value =
		serde_json::from_slice(&std::fs::read(dir.join("model.safetensors.index.json"))?)?;
	let wm = index["weight_map"].as_object().ok_or_else(|| anyhow!("no weight_map"))?;

	// Open each shard once; parse its header for byte ranges.
	let mut shard_paths: Vec<String> = wm.values().map(|v| v.as_str().unwrap_or("").to_string()).collect();
	shard_paths.sort();
	shard_paths.dedup();
	let mut shard_idx: HashMap<String, usize> = HashMap::new();
	let mut shards = Vec::new();
	let mut big: HashMap<String, Tensor> = HashMap::new();
	for (i, sp) in shard_paths.iter().enumerate() {
		shard_idx.insert(sp.clone(), i);
		let path = dir.join(sp);
		// header only: read first 8 bytes for length, then the header json
		let mut lenb = [0u8; 8];
		let f = File::open(&path)?;
		f.read_exact_at(&mut lenb, 0)?;
		let hlen = u64::from_le_bytes(lenb) as usize;
		let mut hdr = vec![0u8; 8 + hlen];
		f.read_exact_at(&mut hdr, 0)?;
		let (data_start, entries) = parse_safetensors_header(&hdr)?;
		for e in entries {
			if !e.name.starts_with("model.decoder.") {
				continue; // text path only; skip encoder/vision tower
			}
			if e.dtype != "BF16" {
				bail!("{}: expected BF16, got {}", e.name, e.dtype);
			}
			big.insert(
				e.name,
				Tensor { shard: i, off: data_start + e.begin, nbytes: e.end - e.begin, shape: e.shape },
			);
		}
		shards.push(f);
	}

	// Norm convention: gemma stores gamma as (1+w) when the mean is ~0; if a
	// checkpoint has folded the +1 the mean is ~1. Decide from layer-0 input norm.
	let probe: Vec<f64> = {
		let t = big.get("model.decoder.layers.0.input_layernorm.weight").ok_or_else(|| anyhow!("no probe norm"))?;
		let mut buf = vec![0u8; t.nbytes];
		shards[t.shard].read_exact_at(&mut buf, t.off as u64)?;
		buf.chunks_exact(2).map(|c| bf16(u16::from_le_bytes([c[0], c[1]]))).collect()
	};
	let mean = probe.iter().sum::<f64>() / probe.len() as f64;
	let plus_one = mean.abs() < 0.5;
	eprintln!("norm probe mean={mean:.4} -> {}", if plus_one { "(1+w) HF convention" } else { "folded x*w" });

	eprintln!("allocating stage+win...");
	let mut m = Model {
		shards,
		big,
		stage: GpuBuffer::alloc_bytes(MAXW * 2)?,
		win: GpuBuffer::alloc(MAXW)?,
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
		ls: Vec::new(),
		emb: Vec::new(),
	};

	// Per-layer resident tensors.
	for l in 0..NL {
		eprint!("\rnorms layer {}/{NL}", l + 1);
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
		m.ls.push(m.small_f64(&p("layer_scalar"))?[0]);
	}

	// Globals.
	eprintln!("\rglobals + embedding table...");
	m.decoder_norm = upload_gamma(&m.small_f64("model.decoder.norm.weight")?, plus_one)?;
	m.sc_pre = upload_gamma(&m.small_f64("model.decoder.self_conditioning.pre_norm.weight")?, plus_one)?;
	m.sc_gate = GpuBuffer::upload(&m.small_f64("model.decoder.self_conditioning.gate_proj.weight")?)?;
	m.sc_up = GpuBuffer::upload(&m.small_f64("model.decoder.self_conditioning.up_proj.weight")?)?;
	m.sc_down = GpuBuffer::upload(&m.small_f64("model.decoder.self_conditioning.down_proj.weight")?)?;

	// Embedding table: keep raw bf16 bytes resident on host.
	let et = m.big.get("model.decoder.embed_tokens.weight").ok_or_else(|| anyhow!("no embed_tokens"))?;
	if et.shape != vec![VOCAB, NE] {
		bail!("embed_tokens shape {:?}", et.shape);
	}
	m.emb = m.read_bytes(et, 0, et.nbytes)?;

	Ok(m)
}

// The per-layer weight names touched every step outside the expert loop.
fn fixed_names(l: usize) -> Vec<String> {
	let mut names = vec![
		layer_name(l, "self_attn.q_proj.weight"),
		layer_name(l, "self_attn.k_proj.weight"),
		layer_name(l, "self_attn.o_proj.weight"),
		layer_name(l, "mlp.gate_proj.weight"),
		layer_name(l, "mlp.up_proj.weight"),
		layer_name(l, "mlp.down_proj.weight"),
	];
	if dims(l).has_v {
		names.push(layer_name(l, "self_attn.v_proj.weight"));
	}
	names
}

// Warm every allocation the forward will ever need besides the waterfall —
// rocBLAS grows its workspace on first use per shape class, and the waterfall
// takes ALL remaining VRAM, so the workspace must exist first.
fn preflight(m: &Model, ar: &Arena, t: usize) -> Result<()> {
	gpu_core::kernels::gpu_gemm_bt_into(&ar.x, &m.win.view(0, 8192 * NE), &ar.q, t, 8192, NE)?;
	beat();
	gpu_core::kernels::gpu_gemm_bt_into(&ar.cms, &m.win.view(0, NFF * NE), &ar.g, t, NFF, NE)?;
	beat();
	gpu_core::kernels::gpu_gemm_bt_into(&ar.act, &m.win.view(0, NE * NFF), &ar.mlp0, t, NE, NFF)?;
	beat();
	gpu_core::kernels::gpu_gemm_bt_into(&ar.moe_xg, &m.win.view(0, 2 * NFFE * NE), &ar.moe_gu, t, 2 * NFFE, NE)?;
	gpu_core::hip::device_synchronize()?;
	beat();
	Ok(())
}

// Waterfall fill, hottest first: the embedding table and per-layer attn+mlp
// weights (touched every step), then experts interleaved expert-major so the
// VRAM tier spans all 30 layers instead of pinning the first N. Runs LAST —
// everything else is already allocated, so "VRAM full" is the allocator refusing.
fn fill_store(m: &mut Model, store: Waterfall) -> Result<()> {
	let mut store = store;
	beat();
	store.place("model.decoder.embed_tokens.weight", m.emb.len(), |dst| {
		dst.copy_from_slice(&m.emb);
		Ok(())
	})?;
	beat();
	for l in 0..NL {
		for name in fixed_names(l) {
			let t = m.big.get(&name).ok_or_else(|| anyhow!("missing {name}"))?;
			store.place(&name, t.nbytes, |dst| m.shards[t.shard].read_exact_at(dst, t.off as u64))?;
			beat();
		}
	}
	for e in 0..NEXP {
		for l in 0..NL {
			let gu = m.big.get(&layer_name(l, "experts.gate_up_proj")).ok_or_else(|| anyhow!("no gate_up {l}"))?;
			let dn = m.big.get(&layer_name(l, "experts.down_proj")).ok_or_else(|| anyhow!("no down {l}"))?;
			store.place(&ekey(l, e), SLOT_BYTES, |dst| {
				m.shards[gu.shard].read_exact_at(&mut dst[..GU_BYTES], (gu.off + e * GU_BYTES) as u64)?;
				m.shards[dn.shard].read_exact_at(&mut dst[GU_BYTES..], (dn.off + e * DN_BYTES) as u64)
			})?;
			beat();
		}
	}
	store.report();
	m.store = store;

	// Loud staleness canary: VRAM homes must read back the shard bytes. A driver
	// serving stale zeros for fresh pool pages dies HERE, not mid-step-0.
	for name in [
		"model.decoder.embed_tokens.weight".to_string(),
		fixed_names(0).remove(0),
		fixed_names(NL - 1).pop().ok_or_else(|| anyhow!("no names"))?,
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

// Host RMSNorm (scale-less) for router input.
fn rnp(x: &[f64]) -> Vec<f64> {
	let inv = 1.0 / ((x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64) + EPS).sqrt();
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

// One transformer layer on GPU, allocation-free: reads hidden `h_in` (t, NE),
// writes the new hidden into `h_out` (t, NE). Every intermediate is a preallocated
// arena buffer written through an `_into` op. RMSNorm is done in-place where the
// input dies (kernel reads the whole row before writing, so aliasing is safe).
fn layer(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	prefix: usize,
	ar: &Arena,
) -> Result<()> {
	let nm = &m.norms[l];
	let d = dims(l);
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, NQH * d.hd, d.nkv * d.hd);
	// Attention. Full layers have no v_proj: v reuses the k_proj weight window.
	let _ta = Instant::now();
	gpu_rmsnorm_f64_into(h_in, Some(&nm["input"]), &ar.x, t, NE, EPS);
	gpu_gemm_bt_into(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, &ar.q, t, qd, NE)?;
	let wk = m.stream(&layer_name(l, "self_attn.k_proj.weight"))?;
	gpu_gemm_bt_into(&ar.x, &wk, &ar.k, t, kd, NE)?;
	if d.has_v {
		gpu_gemm_bt_into(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, &ar.v, t, kd, NE)?;
	} else {
		gpu_gemm_bt_into(&ar.x, &wk, &ar.v, t, kd, NE)?;
	}
	gpu_rmsnorm_f64_into(&ar.q, Some(&nm["q_norm"]), &ar.q, t * NQH, hd, EPS);
	gpu_rmsnorm_f64_into(&ar.k, Some(&nm["k_norm"]), &ar.k, t * nkv, hd, EPS);
	gpu_rmsnorm_f64_into(&ar.v, None, &ar.v, t * nkv, hd, EPS);
	gpu_rope_partial(&ar.q, t * NQH, hd, d.rotary, NQH, d.theta);
	gpu_rope_partial(&ar.k, t * nkv, hd, d.rotary, nkv, d.theta);
	gpu_gqa_attn_into(&ar.q, &ar.k, &ar.v, &ar.attn, t, NQH, nkv, hd, prefix);
	gpu_gemm_bt_into(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, &ar.o, t, NE, qd)?;
	gpu_rmsnorm_f64_into(&ar.o, Some(&nm["post_attn"]), &ar.o, t, NE, EPS);
	gpu_add_into(&ar.o, h_in, &ar.attn_out, t * NE);
	acc(&ATTN_NS, _ta);

	// Shared MLP branch.
	let _tm = Instant::now();
	gpu_rmsnorm_f64_into(&ar.attn_out, Some(&nm["pre_ff"]), &ar.cms, t, NE, EPS);
	gpu_gemm_bt_into(&ar.cms, &m.stream(&layer_name(l, "mlp.gate_proj.weight"))?, &ar.g, t, NFF, NE)?;
	gpu_gemm_bt_into(&ar.cms, &m.stream(&layer_name(l, "mlp.up_proj.weight"))?, &ar.u, t, NFF, NE)?;
	gpu_gelu_mul_into(&ar.g, &ar.u, &ar.act, t * NFF);
	gpu_gemm_bt_into(&ar.act, &m.stream(&layer_name(l, "mlp.down_proj.weight"))?, &ar.mlp0, t, NE, NFF)?;
	gpu_rmsnorm_f64_into(&ar.mlp0, Some(&nm["pf1"]), &ar.mlp, t, NE, EPS);
	acc(&MLP_NS, _tm);

	// MoE branch — routing + grouping on host, expert GEMMs on GPU.
	let _tmoe = Instant::now();
	gpu_rmsnorm_f64_into(&ar.attn_out, Some(&nm["pn2"]), &ar.cmoes, t, NE, EPS);
	let _rt = Instant::now();
	let ao_host = ar.attn_out.download_vec()?;
	let cmoes_host = ar.cmoes.download_vec()?;
	acc(&MOE_RT_NS, _rt);
	let _tr = Instant::now();
	let (rw, gis, pe) = (&m.rw[l], &m.gis[l], &m.pe[l]);
	let inv_sqrt_ne = 1.0 / (NE as f64).sqrt();
	// BTreeMap: deterministic iteration (HashMap order randomizes per process →
	// f64 accumulation order → sampler flips) and offset-ordered disk reads.
	let mut e2p: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
	for p in 0..t {
		let rmn = rnp(&ao_host[p * NE..(p + 1) * NE]);
		let rin: Vec<f64> = (0..NE).map(|xx| rmn[xx] * inv_sqrt_ne * gis[xx]).collect();
		let mut rl = vec![0.0f64; NEXP];
		for (e, rle) in rl.iter_mut().enumerate() {
			let b = e * NE;
			*rle = (0..NE).map(|i| rw[b + i] * rin[i]).sum();
		}
		softmax(&mut rl);
		let mut idx: Vec<usize> = (0..NEXP).collect();
		idx.sort_by(|a, b| rl[*b].partial_cmp(&rl[*a]).unwrap_or(std::cmp::Ordering::Equal));
		idx.truncate(USED);
		let ws: f64 = idx.iter().map(|&e| rl[e]).sum();
		for &e in &idx {
			e2p.entry(e).or_default().push((p, rl[e] / ws));
		}
	}
	acc(&ROUTE_NS, _tr);
	let mut mo_host = vec![0.0f64; t * NE];
	let mut xg = vec![0.0f64; t * NE];
	let mut dv_host = vec![0.0f64; t * NE];
	for (&e, poslist) in &e2p {
		let np = poslist.len();
		for (i, &(p, _)) in poslist.iter().enumerate() {
			xg[i * NE..(i + 1) * NE].copy_from_slice(&cmoes_host[p * NE..(p + 1) * NE]);
		}
		let _rt = Instant::now();
		ar.moe_xg.load(&xg[..np * NE])?;
		acc(&MOE_RT_NS, _rt);
		let es = m.expert_slot(l, e)?;
		let gu_w = m.widen_from(&es, 0, 2 * NFFE * NE);
		gpu_gemm_bt_into(&ar.moe_xg, &gu_w, &ar.moe_gu, np, 2 * NFFE, NE)?;
		gpu_glu_gelu_into(&ar.moe_gu, &ar.moe_ea, np, NFFE);
		let dn_w = m.widen_from(&es, GU_BYTES, NE * NFFE);
		gpu_gemm_bt_into(&ar.moe_ea, &dn_w, &ar.moe_dv, np, NE, NFFE)?;
		let _rt = Instant::now();
		ar.moe_dv.download(&mut dv_host[..np * NE])?;
		acc(&MOE_RT_NS, _rt);
		for (i, &(p, w)) in poslist.iter().enumerate() {
			let s = w * pe[e];
			for xx in 0..NE {
				mo_host[p * NE + xx] += s * dv_host[i * NE + xx];
			}
		}
	}
	ar.mo.load(&mo_host)?;
	gpu_rmsnorm_f64_into(&ar.mo, Some(&nm["pf2"]), &ar.mop, t, NE, EPS);
	acc(&MOE_NS, _tmoe);

	// Combine: post_ffw_norm(mlp+moe), then (attn_out + comb) * layer_scalar.
	gpu_add_into(&ar.mlp, &ar.mop, &ar.comb, t * NE);
	gpu_rmsnorm_f64_into(&ar.comb, Some(&nm["pfw"]), &ar.comb, t, NE, EPS);
	gpu_add_into(&ar.attn_out, &ar.comb, h_out, t * NE);
	gpu_scale_inplace(h_out, m.ls[l], t * NE);
	Ok(())
}

fn layer_name(l: usize, suffix: &str) -> String {
	format!("model.decoder.layers.{l}.{suffix}")
}

// LM head: hfs (ncanvas, NE) @ emb (VOCAB, NE)^T -> host logits (ncanvas, VOCAB),
// tiling the vocab so the widened f64 chunk stays small.
fn lm_head(m: &Model, hfs: &GpuBuffer, ncanvas: usize, ar: &Arena) -> Result<Vec<f64>> {
	let _tl = Instant::now();
	let mut logits = vec![0.0f64; ncanvas * VOCAB];
	let mut out_host = vec![0.0f64; ncanvas * LM_CHUNK];
	let mut c0 = 0;
	while c0 < VOCAB {
		let cn = LM_CHUNK.min(VOCAB - c0);
		// Embedding tile: widen from its waterfall home (VRAM in place; otherwise
		// the host copy bounces through stage). No allocation.
		let w = match m.store.home("model.decoder.embed_tokens.weight") {
			Some(Home::Vram(dev)) => m.widen_from(dev, c0 * NE * 2, cn * NE),
			_ => {
				m.to_stage(&m.emb[c0 * NE * 2..(c0 + cn) * NE * 2])?;
				m.widen_from(&m.stage, 0, cn * NE)
			}
		};
		gpu_gemm_bt_into(hfs, &w, &ar.lm_out, ncanvas, cn, NE)?;
		ar.lm_out.download(&mut out_host[..ncanvas * cn])?;
		for p in 0..ncanvas {
			logits[p * VOCAB + c0..p * VOCAB + c0 + cn].copy_from_slice(&out_host[p * cn..(p + 1) * cn]);
		}
		c0 += cn;
	}
	acc(&LM_NS, _tl);
	Ok(logits)
}

// Greedy longest-match tokenizer over the ▁-prefixed vocab (BOS prepended).
fn tokenize(prompt: &str, rev: &HashMap<String, u32>) -> Vec<u32> {
	let text = format!("\u{2581}{}", prompt.replace(' ', "\u{2581}"));
	let ch: Vec<char> = text.chars().collect();
	let mut toks = vec![BOS];
	let mut i = 0;
	while i < ch.len() {
		let mut best: Option<u32> = None;
		let mut blen = 0;
		for l in (1..=ch.len() - i).rev() {
			let s: String = ch[i..i + l].iter().collect();
			if let Some(&id) = rev.get(&s) {
				best = Some(id);
				blen = l;
				break;
			}
		}
		match best {
			Some(id) => {
				toks.push(id);
				i += blen;
			}
			None => i += 1,
		}
	}
	toks
}

fn main() -> Result<()> {
	let dir = PathBuf::from("/home/nate/Desktop/gemma4/diffusiongemma-26B-A4B-it");
	let prompt = std::env::args().nth(1).unwrap_or_else(|| "The capital of France is".to_string());

	// Vocab: id->token and token->id from tokenizer.json (model.vocab + added).
	let tok_json: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("tokenizer.json"))?)?;
	let mut vocab = vec![String::new(); VOCAB];
	let mut rev: HashMap<String, u32> = HashMap::new();
	if let Some(map) = tok_json["model"]["vocab"].as_object() {
		for (k, v) in map {
			if let Some(id) = v.as_u64() {
				if (id as usize) < VOCAB {
					vocab[id as usize] = k.clone();
					rev.insert(k.clone(), id as u32);
				}
			}
		}
	}
	if let Some(added) = tok_json["added_tokens"].as_array() {
		for a in added {
			if let (Some(id), Some(c)) = (a["id"].as_u64(), a["content"].as_str()) {
				if (id as usize) < VOCAB {
					vocab[id as usize] = c.to_string();
					rev.insert(c.to_string(), id as u32);
				}
			}
		}
	}

	eprintln!("loading model...");
	let t_load = Instant::now();
	let watchdog = arm_watchdog();
	// One-claim lifecycle: no pool warm (the claim is the pool's only customer),
	// then ONE allocation of all free VRAM becomes the process arena — every
	// GpuBuffer after this line carves from it, including hipBLAS's workspace.
	// init → one precalculated claim; exit → its one free.
	gpu_core::memory::skip_pool_warm();
	recipe_infer::init().map_err(|e| anyhow!("gpu init: {e:?}"))?;
	let claim = Waterfall::claim();
	beat();
	let _blas_ws = GpuBuffer::alloc_bytes(128 << 20)?;
	gpu_core::kernels::gpu_blas_workspace(&_blas_ws);
	// Keepalive: load has multi-second host-only gaps (disk reads, tokenize) and
	// the GPU has a 5s runtime-PM autosuspend; the wedges cluster right after
	// those gaps. A 1 Hz trivial device op keeps the queues warm through load.
	let keepalive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
	{
		let ka = keepalive.clone();
		let buf = GpuBuffer::alloc(1)?;
		std::thread::spawn(move || {
			while ka.load(std::sync::atomic::Ordering::Relaxed) {
				if buf.memset_zero(8).is_err() {
					return;
				}
				std::thread::sleep(std::time::Duration::from_secs(1));
			}
		});
	}
	let mut m = load_model(&dir)?;

	// Build the diffusion canvas: prompt tokens + NCANVAS masks.
	let mut toks = tokenize(&prompt, &rev);
	let prefix = toks.len();
	for _ in 0..NCANVAS {
		toks.push(MASK as u32);
	}
	let t = toks.len();
	let scl = (NE as f64).sqrt();
	eprintln!("prompt tokens={prefix} canvas={NCANVAS} total={t}");

	let mut sck: Vec<Vec<(usize, f64)>> = vec![vec![]; NCANVAS];
	let mut pred = vec![MASK as u32; NCANVAS];

	// Preallocate the whole forward arena once (t is now known). After this point
	// the hot loop allocates nothing — the acceptance invariant is that the device
	// pool alloc count is identical before step 0 and after the last step.
	let ar = {
		let _t = gpu_core::memory::tag_scope("arena");
		Arena::new(t)?
	};
	// Everything else the forward allocates now exists (arena, stage, win,
	// rocBLAS workspace via preflight) — the waterfall takes all that remains.
	eprintln!("preflight gemms...");
	preflight(&m, &ar, t)?;
	eprintln!("waterfall fill...");
	fill_store(&mut m, claim)?;
	watchdog.store(false, std::sync::atomic::Ordering::Relaxed);
	keepalive.store(false, std::sync::atomic::Ordering::Relaxed);
	eprintln!("loaded in {:.1}s", t_load.elapsed().as_secs_f64());
	let allocs_before = gpu_core::memory::device_alloc_count();
	let t0 = Instant::now();

	for step in 0..6 {
		// Host: base scaled embeddings for every position → into the ping-pong A buffer.
		let mut base = vec![0.0f64; t * NE];
		for (p, &tk) in toks.iter().enumerate() {
			let b = tk as usize * NE * 2;
			for x in 0..NE {
				base[p * NE + x] = bf16(u16::from_le_bytes([m.emb[b + x * 2], m.emb[b + x * 2 + 1]])) * scl;
			}
		}
		ar.ha.load(&base)?;

		// Canvas rows: add self-conditioning (step>0) then scale-less rmsnorm.
		let coff = prefix * NE;
		let clen = NCANVAS * NE;
		if step > 0 {
			// soft = sum over top-8 prev (prob * emb(id)) * scl, per canvas position
			let mut soft = vec![0.0f64; NCANVAS * NE];
			for (c, top) in sck.iter().enumerate() {
				for &(id, pr) in top {
					let b = id * NE * 2;
					for x in 0..NE {
						soft[c * NE + x] += pr * bf16(u16::from_le_bytes([m.emb[b + x * 2], m.emb[b + x * 2 + 1]]));
					}
				}
				for x in 0..NE {
					soft[c * NE + x] *= scl;
				}
			}
			ar.soft.load(&soft)?;
			gpu_rmsnorm_f64_into(&ar.soft, Some(&m.sc_pre), &ar.scn, NCANVAS, NE, EPS);
			gpu_gemm_bt_into(&ar.scn, &m.sc_gate, &ar.sg, NCANVAS, NFF, NE)?;
			gpu_gemm_bt_into(&ar.scn, &m.sc_up, &ar.su, NCANVAS, NFF, NE)?;
			gpu_gelu_mul_into(&ar.sg, &ar.su, &ar.sa, NCANVAS * NFF);
			gpu_gemm_bt_into(&ar.sa, &m.sc_down, &ar.sc_add, NCANVAS, NE, NFF)?;
			gpu_add_into(&ar.ha.view(coff, clen), &ar.sc_add, &ar.cur, clen);
			gpu_rmsnorm_f64_into(&ar.cur, None, &ar.normed, NCANVAS, NE, EPS);
		} else {
			gpu_rmsnorm_f64_into(&ar.ha.view(coff, clen), None, &ar.normed, NCANVAS, NE, EPS);
		}
		ar.ha.view(coff, clen).copy_from(&ar.normed, clen * 8)?;

		// 30 transformer layers, ping-ponging the hidden state between ha/hb.
		let bithash = |b: &GpuBuffer, n: usize| -> Result<u64> {
			let mut v = vec![0.0f64; n];
			b.view(0, n).download(&mut v)?;
			Ok(v.iter().fold(0xcbf29ce484222325u64, |h, x| (h ^ x.to_bits()).wrapping_mul(0x100000001b3)))
		};
		if step == 0 {
			eprintln!("[hash] step0 input {:016x}", bithash(&ar.ha, t * NE)?);
		}
		let mut src: &GpuBuffer = &ar.ha;
		let mut dst: &GpuBuffer = &ar.hb;
		for l in 0..NL {
			eprint!("\rstep {step} layer {}/{NL} ({:.0}s)", l + 1, t0.elapsed().as_secs_f64());
			layer(&m, l, src, dst, t, prefix, &ar)?;
			std::mem::swap(&mut src, &mut dst);
			if step == 0 {
				eprintln!("\n[hash] step0 layer {l:2} {:016x}", bithash(src, t * NE)?);
			}
		}
		eprint!("\r\x1b[K");
		let hbuf = src; // last buffer written
		let nan = hbuf.download_vec()?.iter().filter(|v| !v.is_finite()).count();
		if nan > 0 {
			bail!("step {step}: {nan} non-finite in h after layers");
		}

		// LM head over canvas positions.
		gpu_rmsnorm_f64_into(&hbuf.view(coff, clen), Some(&m.decoder_norm), &ar.hfs, NCANVAS, NE, EPS);
		let logits = lm_head(&m, &ar.hfs, NCANVAS, &ar)?;

		// Sample each canvas position (top-50, temperature, xorshift) — host.
		let temp = 1.0 - 0.7 * (step as f64 / 6.0);
		for c in 0..NCANVAS {
			let row = &logits[c * VOCAB..(c + 1) * VOCAB];
			let mut cand: Vec<(usize, f64)> = (0..VOCAB)
				.filter(|&tk| tk >= 6 && tk != MASK_SIGNAL && !vocab[tk].starts_with('<'))
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
			for (k, &pr) in probs.iter().enumerate() {
				cum += pr;
				if r <= cum {
					sel = cand[k].0;
					break;
				}
			}
			pred[c] = sel as u32;
			let mut top: Vec<(usize, f64)> = cand.iter().zip(probs.iter()).take(8).map(|(&(id, _), &pr)| (id, pr)).collect();
			let s8: f64 = top.iter().map(|&(_, pr)| pr).sum();
			for e in top.iter_mut() {
				e.1 /= s8;
			}
			sck[c] = top;
		}
		let text: String = pred.iter().map(|&tk| vocab[tk as usize].replace('\u{2581}', " ")).collect();
		eprintln!("step {step} ({:.0}s): {text}", t0.elapsed().as_secs_f64());
	}

	let allocs_after = gpu_core::memory::device_alloc_count();
	eprintln!("steady-state allocs: {}", allocs_after - allocs_before);
	let tot = t0.elapsed().as_secs_f64();
	let s = |a: &AtomicU64| a.load(Ordering::Relaxed) as f64 / 1e9;
	eprintln!(
		"[breakdown] total={tot:.1}s  disk={:.1}s  h2d(write_u8)={:.1}s  widen(launch)={:.1}s",
		s(&DISK_NS), s(&H2D_NS), s(&WIDEN_NS),
	);
	eprintln!(
		"[sections]  attn={:.1}s  mlp={:.1}s  moe={:.1}s (route={:.1}s roundtrips={:.1}s)  lm_head={:.1}s",
		s(&ATTN_NS), s(&MLP_NS), s(&MOE_NS), s(&ROUTE_NS), s(&MOE_RT_NS), s(&LM_NS),
	);
	eprintln!(
		"[experts]   from VRAM={}  from RAM={}  from DISK={}",
		E_VRAM.load(Ordering::Relaxed), E_RAM.load(Ordering::Relaxed), E_DISK.load(Ordering::Relaxed),
	);
	m.store.report();

	let out: String = pred.iter().map(|&tk| vocab[tk as usize].replace('\u{2581}', " ")).collect();
	println!("\n=== OUTPUT ===\n{out}");
	eprintln!("{}", gpu_core::memory::ledger_report());
	recipe_infer::shutdown();
	Ok(())
}
