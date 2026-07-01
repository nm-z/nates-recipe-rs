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
	gpu_gelu_mul, gpu_glu_gelu, gpu_gqa_attn, gpu_rmsnorm_f64, gpu_rope_partial, gpu_widen_bf16,
	gpu_widen_bf16_into,
};
use gpu_core::kernels::{gpu_add, gpu_copy, gpu_gemm_bt, gpu_scale};
use gpu_core::memory::GpuBuffer;
use recipe_infer::safetensors::parse_safetensors_header;
use std::collections::HashMap;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::time::Instant;

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

struct Model {
	shards: Vec<File>,
	big: HashMap<String, Tensor>, // streamed (bf16 on disk)
	stage: GpuBuffer,             // reusable device bf16 staging (MAXW*2 bytes)
	win: GpuBuffer,               // reusable f64 widen window (MAXW floats)
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
		self.shards[t.shard]
			.read_exact_at(&mut buf, (t.off + off) as u64)
			.with_context(|| format!("read {len} bytes at shard {}", t.shard))?;
		Ok(buf)
	}

	// Decode a small tensor fully to host f64.
	fn small_f64(&self, name: &str) -> Result<Vec<f64>> {
		let t = self.big.get(name).ok_or_else(|| anyhow!("missing {name}"))?;
		let raw = self.read_bytes(t, 0, t.nbytes)?;
		Ok(raw.chunks_exact(2).map(|c| bf16(u16::from_le_bytes([c[0], c[1]]))).collect())
	}

	// Widen a full big tensor into the reusable window; returns a borrow view of
	// the window (no allocation — avoids hipMallocAsync pool churn/lazy-commit
	// faults). The caller must consume it before the next stream_* call.
	fn stream(&self, name: &str) -> Result<GpuBuffer> {
		let t = self.big.get(name).ok_or_else(|| anyhow!("missing {name}"))?;
		let raw = self.read_bytes(t, 0, t.nbytes)?;
		self.widen_window(&raw)
	}

	// Widen one expert slice `e` (per-expert element count `per`) into the window.
	fn stream_expert(&self, name: &str, e: usize, per: usize) -> Result<GpuBuffer> {
		let t = self.big.get(name).ok_or_else(|| anyhow!("missing {name}"))?;
		let raw = self.read_bytes(t, e * per * 2, per * 2)?;
		self.widen_window(&raw)
	}

	fn widen_window(&self, raw: &[u8]) -> Result<GpuBuffer> {
		let n = raw.len() / 2;
		self.stage.write_u8(raw)?;
		gpu_widen_bf16_into(&self.stage, &self.win, n);
		Ok(self.win.view(0, n))
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

	let mut m = Model {
		shards,
		big,
		stage: GpuBuffer::alloc_bytes(MAXW * 2)?,
		win: GpuBuffer::alloc(MAXW)?,
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

// One transformer layer on GPU; returns new hidden state h (t, NE).
fn layer(m: &Model, l: usize, h: &GpuBuffer, t: usize, prefix: usize) -> Result<GpuBuffer> {
	let nm = &m.norms[l];
	let d = dims(l);
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, NQH * d.hd, d.nkv * d.hd);
	// Attention. Full layers have no v_proj: v reuses the k_proj weight.
	let x = gpu_rmsnorm_f64(h, Some(&nm["input"]), t, NE, EPS)?;
	let q = gpu_gemm_bt(&x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, qd, NE)?;
	let wk = m.stream(&layer_name(l, "self_attn.k_proj.weight"))?;
	let k = gpu_gemm_bt(&x, &wk, t, kd, NE)?;
	let v = if d.has_v {
		gpu_gemm_bt(&x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, NE)?
	} else {
		gpu_gemm_bt(&x, &wk, t, kd, NE)?
	};
	let q = gpu_rmsnorm_f64(&q, Some(&nm["q_norm"]), t * NQH, hd, EPS)?;
	let k = gpu_rmsnorm_f64(&k, Some(&nm["k_norm"]), t * nkv, hd, EPS)?;
	let v = gpu_rmsnorm_f64(&v, None, t * nkv, hd, EPS)?;
	gpu_rope_partial(&q, t * NQH, hd, d.rotary, NQH, d.theta);
	gpu_rope_partial(&k, t * nkv, hd, d.rotary, nkv, d.theta);
	let attn = gpu_gqa_attn(&q, &k, &v, t, NQH, nkv, hd, prefix)?;
	let o = gpu_gemm_bt(&attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, NE, qd)?;
	let o = gpu_rmsnorm_f64(&o, Some(&nm["post_attn"]), t, NE, EPS)?;
	let attn_out = gpu_add(&o, h, t * NE)?;

	// Shared MLP branch.
	let cms = gpu_rmsnorm_f64(&attn_out, Some(&nm["pre_ff"]), t, NE, EPS)?;
	let g = gpu_gemm_bt(&cms, &m.stream(&layer_name(l, "mlp.gate_proj.weight"))?, t, NFF, NE)?;
	let u = gpu_gemm_bt(&cms, &m.stream(&layer_name(l, "mlp.up_proj.weight"))?, t, NFF, NE)?;
	let act = gpu_gelu_mul(&g, &u, t * NFF)?;
	let mlp0 = gpu_gemm_bt(&act, &m.stream(&layer_name(l, "mlp.down_proj.weight"))?, t, NE, NFF)?;
	let mlp = gpu_rmsnorm_f64(&mlp0, Some(&nm["pf1"]), t, NE, EPS)?;

	// MoE branch — routing + grouping on host, expert GEMMs on GPU.
	let cmoes = gpu_rmsnorm_f64(&attn_out, Some(&nm["pn2"]), t, NE, EPS)?;
	let ao_host = attn_out.download_vec()?;
	let cmoes_host = cmoes.download_vec()?;
	let (rw, gis, pe) = (&m.rw[l], &m.gis[l], &m.pe[l]);
	let inv_sqrt_ne = 1.0 / (NE as f64).sqrt();
	let mut e2p: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
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
	let mut mo_host = vec![0.0f64; t * NE];
	for (&e, poslist) in &e2p {
		let np = poslist.len();
		let mut xg = vec![0.0f64; np * NE];
		for (i, &(p, _)) in poslist.iter().enumerate() {
			xg[i * NE..(i + 1) * NE].copy_from_slice(&cmoes_host[p * NE..(p + 1) * NE]);
		}
		let xg = GpuBuffer::upload(&xg)?;
		let gu_w = m.stream_expert(&layer_name(l, "experts.gate_up_proj"), e, 2 * NFFE * NE)?;
		let gu = gpu_gemm_bt(&xg, &gu_w, np, 2 * NFFE, NE)?;
		let ea = gpu_glu_gelu(&gu, np, NFFE)?;
		let dn_w = m.stream_expert(&layer_name(l, "experts.down_proj"), e, NE * NFFE)?;
		let dv = gpu_gemm_bt(&ea, &dn_w, np, NE, NFFE)?;
		let dv_host = dv.download_vec()?;
		for (i, &(p, w)) in poslist.iter().enumerate() {
			let s = w * pe[e];
			for xx in 0..NE {
				mo_host[p * NE + xx] += s * dv_host[i * NE + xx];
			}
		}
	}
	let mo = GpuBuffer::upload(&mo_host)?;
	let mop = gpu_rmsnorm_f64(&mo, Some(&nm["pf2"]), t, NE, EPS)?;

	// Combine: post_ffw_norm(mlp+moe), then (attn_out + comb) * layer_scalar.
	let comb = gpu_add(&mlp, &mop, t * NE)?;
	let comb = gpu_rmsnorm_f64(&comb, Some(&nm["pfw"]), t, NE, EPS)?;
	let hplus = gpu_add(&attn_out, &comb, t * NE)?;
	Ok(gpu_scale(&hplus, m.ls[l], t * NE)?)
}

fn layer_name(l: usize, suffix: &str) -> String {
	format!("model.decoder.layers.{l}.{suffix}")
}

// LM head: hfs (ncanvas, NE) @ emb (VOCAB, NE)^T -> host logits (ncanvas, VOCAB),
// tiling the vocab so the widened f64 chunk stays small.
fn lm_head(m: &Model, hfs: &GpuBuffer, ncanvas: usize) -> Result<Vec<f64>> {
	let mut logits = vec![0.0f64; ncanvas * VOCAB];
	let chunk = 32768usize;
	let mut c0 = 0;
	while c0 < VOCAB {
		let cn = chunk.min(VOCAB - c0);
		let raw = &m.emb[c0 * NE * 2..(c0 + cn) * NE * 2];
		let g = GpuBuffer::upload_u8(raw)?;
		let w = gpu_widen_bf16(&g, cn * NE)?;
		let out = gpu_gemm_bt(hfs, &w, ncanvas, cn, NE)?;
		let out_host = out.download_vec()?;
		for p in 0..ncanvas {
			logits[p * VOCAB + c0..p * VOCAB + c0 + cn].copy_from_slice(&out_host[p * cn..(p + 1) * cn]);
		}
		c0 += cn;
	}
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
	recipe_infer::init().map_err(|e| anyhow!("gpu init: {e:?}"))?;
	// Streaming inference churns fresh buffers with immediate host->device copies;
	// commit each allocation before it is written to (avoids SDMA page faults).
	gpu_core::memory::set_alloc_sync(true);
	let m = load_model(&dir)?;
	eprintln!("loaded in {:.1}s", t_load.elapsed().as_secs_f64());

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
	let t0 = Instant::now();

	for step in 0..6 {
		// Host: base scaled embeddings for every position.
		let mut base = vec![0.0f64; t * NE];
		for (p, &tk) in toks.iter().enumerate() {
			let b = tk as usize * NE * 2;
			for x in 0..NE {
				base[p * NE + x] = bf16(u16::from_le_bytes([m.emb[b + x * 2], m.emb[b + x * 2 + 1]])) * scl;
			}
		}
		let mut h = GpuBuffer::upload(&base)?;

		// Canvas rows: add self-conditioning (step>0) then scale-less rmsnorm.
		let coff = prefix * NE;
		let clen = NCANVAS * NE;
		let cur = if step > 0 {
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
			let soft = GpuBuffer::upload(&soft)?;
			let scn = gpu_rmsnorm_f64(&soft, Some(&m.sc_pre), NCANVAS, NE, EPS)?;
			let sg = gpu_gemm_bt(&scn, &m.sc_gate, NCANVAS, NFF, NE)?;
			let su = gpu_gemm_bt(&scn, &m.sc_up, NCANVAS, NFF, NE)?;
			let sa = gpu_gelu_mul(&sg, &su, NCANVAS * NFF)?;
			let sc_add = gpu_gemm_bt(&sa, &m.sc_down, NCANVAS, NE, NFF)?;
			gpu_add(&h.view(coff, clen), &sc_add, clen)?
		} else {
			gpu_copy(&h.view(coff, clen), clen)?
		};
		let normed = gpu_rmsnorm_f64(&cur, None, NCANVAS, NE, EPS)?;
		h.view(coff, clen).copy_from(&normed, clen * 8)?;

		// 30 transformer layers.
		for l in 0..NL {
			h = layer(&m, l, &h, t, prefix)?;
		}
		let nan = h.download_vec()?.iter().filter(|v| !v.is_finite()).count();
		if nan > 0 {
			bail!("step {step}: {nan} non-finite in h after layers");
		}

		// LM head over canvas positions.
		let hfs = gpu_rmsnorm_f64(&h.view(coff, clen), Some(&m.decoder_norm), NCANVAS, NE, EPS)?;
		let logits = lm_head(&m, &hfs, NCANVAS)?;

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

	let out: String = pred.iter().map(|&tk| vocab[tk as usize].replace('\u{2581}', " ")).collect();
	println!("\n=== OUTPUT ===\n{out}");
	recipe_infer::shutdown();
	Ok(())
}
