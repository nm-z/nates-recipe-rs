//! The reusable GPU scratch arena: every activation, gradient, metric, and
//! attention temporary, allocated once at fit/eval and reused across epochs so
//! steady-state VRAM is flat. Plus the exact-size pre-checks (`vram_bytes`,
//! `vram_estimate`) that gate a forward/backward pass against free VRAM.

use crate::enums::{Activation, LayerKind, LayerSpec};
use crate::params::{LayerParams, concat_layer};
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;

pub struct Scratch {
	pub acts: Vec<GpuBuffer>,
	// Per-layer pre-activation, saved ONLY for Silu/Gelu (their backward needs the
	// input z, which the in-place activation would otherwise overwrite). Len-1
	// dummy for every other layer.
	pub preact: Vec<GpuBuffer>,
	pub da_a: GpuBuffer,
	pub da_b: GpuBuffer,
	pub dz: GpuBuffer,
	pub dw: GpuBuffer,
	// Split-K weight-grad partials [P×in_dim·out_dim], summed in pass 2 into dw.
	// Sized to the widest Dense layer's P·k·n; len-1 in forward-only.
	pub dw_partials: GpuBuffer,
	pub db: GpuBuffer,
	pub metric_t0: GpuBuffer,
	pub metric_t1: GpuBuffer,
	pub metric_t2: GpuBuffer,
	pub metric_scalar: GpuBuffer,
	// Second scalar slot so the per-epoch score and loss can both ride the async
	// copy stream and sync once, instead of one blocking 8-byte D2H per metric.
	pub metric_scalar_b: GpuBuffer,
	pub reduce_ws: GpuBuffer,
	// Embed layers accumulate the table gradient here ([vocab×dim]) before the
	// SGD step — scatter-add target, separate from the table so the update is
	// `table -= lr·grad`. Len 1 when there's no embed layer.
	pub embed_grad: GpuBuffer,
	// Attention scratch (len 1 when there's no attn layer). q/k/v/ctx are the
	// projected sequences [n*S*d]; scores [n*heads*S*S]; the d* mirrors hold the
	// backward gradients; gw is a [d*d] weight-grad temp reused per projection.
	pub a_q: GpuBuffer,
	pub a_k: GpuBuffer,
	pub a_v: GpuBuffer,
	pub a_ctx: GpuBuffer,
	pub a_scores: GpuBuffer,
	pub a_dctx: GpuBuffer,
	pub a_dq: GpuBuffer,
	pub a_dk: GpuBuffer,
	pub a_dv: GpuBuffer,
	pub a_dscores: GpuBuffer,
	pub a_gw: GpuBuffer,
	pub a_dbias: GpuBuffer,
	// PRelu d_alpha scratch (act-sized temps + a scalar accumulator). Len-1 when
	// no PRelu layer exists.
	pub prelu_t0: GpuBuffer,
	pub prelu_t1: GpuBuffer,
	pub prelu_scalar: GpuBuffer,
	// Two-branch concat: `concat` [n×(A+C)] holds [attn_output | categorical] fed to
	// the first dense layer; `concat_dgrad` [n×A] compacts that dense's input-grad
	// back to the attention width on the backward pass. Len-1 when no concat exists.
	pub concat: GpuBuffer,
	pub concat_dgrad: GpuBuffer,
	pub conv_temp: GpuBuffer,
	pub conv_wg: usize,
	// Inference (forward-only) path: attention uses the chunked KV-cache forward
	// and `a_scores` is a len-1 stub (the O(L²) buffer is never allocated). The
	// training path leaves this false and keeps the full-batch score buffer.
	pub infer: bool,
	pub copy_stream: gpu_core::hip::Stream,
	pub pinned_scalar: *mut f64,
	pub pinned_scalar_b: *mut f64,
}

impl Scratch {
	/// `forward_only` (eval/predict) sizes every BACKWARD-only buffer to len-1 —
	/// they're never read in a forward pass — so inference allocates ~half the VRAM
	/// of training (no second `a_dscores`, no `da`/`dw`/grad mirrors).
	pub fn new(params: &[LayerParams], n: usize, forward_only: bool) -> Scratch {
		let bw = |sz: usize| if forward_only { 1 } else { sz };
		// On OOM, report the buffer name and the size it tried to grab (f64 count →
		// bytes) instead of a bare HipError(2) — full-batch attention scores dominate.
		let alloc = |sz: usize, label: &str| -> GpuBuffer {
			GpuBuffer::alloc(sz).unwrap_or_else(|e| {
				panic!(
					"{label}: GPU alloc of {} ({sz} × f64) failed — {e:?}",
					crate::human_bytes(sz * 8)
				)
			})
		};
		let mut max_ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1);
		let mut max_act = 0usize;
		let mut max_wt = 0usize;
		let mut max_bias = 0usize;
		let mut max_embed_grad = 1usize;
		let mut max_seqd = 1usize;
		let mut max_scores = 1usize;
		let mut max_dd = 1usize;
		let mut max_dw_partials = 1usize;
		let mut has_prelu = false;
		for p in params {
			// Split-K dW partials: Dense reduces n×(in_dim×out_dim); each Attn
			// projection (Wq/Wk/Wv/Wo) reduces (n·s)×(d×d) through the same kernel.
			let dw_dp = match p.kind {
				LayerKind::Dense => kernels::gpu_splitk_dw_partials_elems(n, p.in_dim, p.out_dim),
				LayerKind::Attn => {
					let s = p.in_dim / p.dim;
					kernels::gpu_splitk_dw_partials_elems(n * s, p.dim, p.dim)
				}
				_ => 0,
			};
			if dw_dp > max_dw_partials {
				max_dw_partials = dw_dp;
			}
			if p.act == Activation::PRelu {
				has_prelu = true;
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * p.out_dim, 1);
				if ws > max_ws {
					max_ws = ws;
				}
			}
			let w = kernels::gpu_reduce_sum_cols_workspace_bytes(n, p.out_dim);
			if w > max_ws {
				max_ws = w;
			}
			// da_a/da_b must hold both this layer's output-grad (n·out_dim) and
			// its input-grad da_below (n·in_dim) — the concat dense's in_dim
			// (A+C) can exceed every out_dim, so size to the wider of the two.
			let a = n * p.out_dim.max(p.in_dim);
			if a > max_act {
				max_act = a;
			}
			// dw holds ONLY Dense/Conv weight grads. Attn projections write their
			// d×d grads to a_gw and Embed writes to embed_grad, so neither sizes dw —
			// an attn in_dim×out_dim here is (S·d)² and would blow VRAM for nothing.
			let wt = match p.kind {
				LayerKind::Conv => {
					let lin = p.in_dim / p.conv_cin;
					let lout = (lin - p.conv_k) / p.conv_stride + 1;
					let cout = p.out_dim / lout;
					cout * p.conv_cin * p.conv_k
				}
				LayerKind::Dense => p.in_dim * p.out_dim,
				_ => 0,
			};
			if wt > max_wt {
				max_wt = wt;
			}
			let bias_sz = if p.kind == LayerKind::Conv {
				let lin = p.in_dim / p.conv_cin;
				let lout = (lin - p.conv_k) / p.conv_stride + 1;
				p.out_dim / lout
			} else {
				p.out_dim
			};
			if bias_sz > max_bias {
				max_bias = bias_sz;
			}
			if p.kind == LayerKind::Embed && p.vocab * p.dim > max_embed_grad {
				max_embed_grad = p.vocab * p.dim;
			}
			if p.kind == LayerKind::Attn {
				let s = p.in_dim / p.dim;
				if n * p.in_dim > max_seqd {
					max_seqd = n * p.in_dim;
				}
				if n * p.heads * s * s > max_scores {
					max_scores = n * p.heads * s * s;
				}
				if p.dim * p.dim > max_dd {
					max_dd = p.dim * p.dim;
				}
				// Attn backward reduces over (n*s rows, dim cols) for the bias grad.
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * s, p.dim);
				if ws > max_ws {
					max_ws = ws;
				}
			}
		}
		let mut acts = Vec::with_capacity(params.len());
		let mut preact = Vec::with_capacity(params.len());
		for p in params {
			acts.push(alloc(n * p.out_dim, "scratch acts"));
			let needs_pre = matches!(
				p.act,
				Activation::Silu
					| Activation::Gelu | Activation::Elu
					| Activation::Selu | Activation::PRelu
			);
			preact.push(alloc(
				if needs_pre { n * p.out_dim } else { 1 },
				"scratch preact",
			));
		}
		// Metric temps hold the output (n * out_dim of the last layer) — sized to
		// it so multi-output (k>1) element-wise loss metrics don't overrun.
		let out_elems = n * params.last().map_or(1, |p| p.out_dim);
		// Two-branch concat buffers: `concat` [n×(A+C)] is a FORWARD buffer (eval
		// builds it too); `concat_dgrad` [n×A] is backward-only.
		let (concat_sz, concat_grad_sz) = match concat_layer(params) {
			Some((_, a, c)) => (n * (a + c), n * a),
			None => (1, 1),
		};
		let mut max_conv_fsz = 0usize;
		for p in params {
			if p.kind == LayerKind::Conv {
				let lin = p.in_dim / p.conv_cin;
				let lout = (lin - p.conv_k) / p.conv_stride + 1;
				let cout = p.out_dim / lout;
				let fsz = cout * p.conv_cin * p.conv_k;
				if fsz > max_conv_fsz { max_conv_fsz = fsz; }
			}
		}
		let (conv_temp_buf, conv_wg_count) = if !forward_only && max_conv_fsz > 0 {
			let (mut free, mut total) = (0usize, 0usize);
			unsafe { gpu_core::hip::hipMemGetInfo(&mut free, &mut total) };
			let usable = free / 2;
			let chunks = (usable / (max_conv_fsz * 8)).min(n).max(1);
			let buf = alloc(max_conv_fsz * chunks, "conv_temp");
			let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(chunks, max_conv_fsz);
			if ws > max_ws { max_ws = ws; }
			(buf, chunks)
		} else {
			(alloc(1, "conv_temp"), 0)
		};
		Scratch {
			acts,
			preact,
			da_a: alloc(bw(max_act), "da_a"),
			da_b: alloc(bw(max_act), "da_b"),
			dz: alloc(bw(max_act), "dz"),
			dw: alloc(bw(max_wt), "dw"),
			dw_partials: alloc(bw(max_dw_partials), "dw_partials"),
			db: alloc(bw(max_bias), "db"),
			metric_t0: alloc(out_elems, "metric_t0"),
			metric_t1: alloc(out_elems, "metric_t1"),
			metric_t2: alloc(out_elems, "metric_t2"),
			metric_scalar: alloc(1, "metric_scalar"),
			metric_scalar_b: alloc(1, "metric_scalar_b"),
			reduce_ws: GpuBuffer::alloc_bytes(max_ws).unwrap_or_else(|e| {
				panic!(
					"reduce_ws: GPU alloc of {} failed — {e:?}",
					crate::human_bytes(max_ws)
				)
			}),
			embed_grad: alloc(bw(max_embed_grad), "embed_grad"),
			a_q: alloc(max_seqd, "a_q"),
			a_k: alloc(max_seqd, "a_k"),
			a_v: alloc(max_seqd, "a_v"),
			a_ctx: alloc(max_seqd, "a_ctx"),
			// Inference chunks the query stream and allocates a bounded score block
			// per attn layer at run time, so the full O(L²) buffer is never made.
			a_scores: alloc(if forward_only { 1 } else { max_scores }, "a_scores"),
			a_dctx: alloc(bw(max_seqd), "a_dctx"),
			a_dq: alloc(bw(max_seqd), "a_dq"),
			a_dk: alloc(bw(max_seqd), "a_dk"),
			a_dv: alloc(bw(max_seqd), "a_dv"),
			a_dscores: alloc(bw(max_scores), "a_dscores"),
			a_gw: alloc(bw(max_dd), "a_gw"),
			a_dbias: alloc(bw(max_dd), "a_dbias"),
			prelu_t0: alloc(bw(if has_prelu { max_act } else { 1 }), "prelu_t0"),
			prelu_t1: alloc(bw(if has_prelu { max_act } else { 1 }), "prelu_t1"),
			prelu_scalar: alloc(1, "prelu_scalar"),
			concat: alloc(concat_sz, "concat"),
			concat_dgrad: alloc(bw(concat_grad_sz), "concat_dgrad"),
			conv_temp: conv_temp_buf,
			conv_wg: conv_wg_count,
			infer: forward_only,
			copy_stream: gpu_core::hip::Stream::new().expect("copy stream"),
			pinned_scalar: {
				let ptr = gpu_core::hip::host_malloc(8, 0).expect("pinned scalar");
				ptr as *mut f64
			},
			pinned_scalar_b: {
				let ptr = gpu_core::hip::host_malloc(8, 0).expect("pinned scalar b");
				ptr as *mut f64
			},
		}
	}

	pub fn download_scalar_deferred(&self) {
		unsafe {
			gpu_core::hip::hipMemcpyAsync(
				self.pinned_scalar as *mut std::ffi::c_void,
				self.metric_scalar.ptr_raw() as *const std::ffi::c_void,
				8,
				gpu_core::hip::HIP_MEMCPY_D2H,
				self.copy_stream.raw(),
			);
		}
	}

	/// Enqueue the async D2H of `metric_scalar_b` onto the same copy stream — used
	/// for the per-epoch score so it batches into one sync with the loss copy.
	pub fn download_scalar_b_deferred(&self) {
		unsafe {
			gpu_core::hip::hipMemcpyAsync(
				self.pinned_scalar_b as *mut std::ffi::c_void,
				self.metric_scalar_b.ptr_raw() as *const std::ffi::c_void,
				8,
				gpu_core::hip::HIP_MEMCPY_D2H,
				self.copy_stream.raw(),
			);
		}
	}

	pub fn sync_deferred_scalar(&self) -> f64 {
		self.copy_stream.synchronize().expect("sync copy stream");
		unsafe { *self.pinned_scalar }
	}

	/// Async read of `metric_scalar`: `hipMemcpyAsync` on the copy stream then a
	/// targeted copy-stream sync — never a blocking default-stream `hipMemcpy`.
	/// Used by every per-epoch metric so no metric scalar stalls the compute stream.
	pub fn read_metric_scalar(&self) -> f64 {
		self.download_scalar_deferred();
		self.sync_deferred_scalar()
	}

	/// Drain the copy stream once (both deferred scalar copies complete).
	pub fn sync_copy_stream(&self) {
		self.copy_stream.synchronize().expect("sync copy stream");
	}

	/// Last value copied by `download_scalar_deferred` / `download_scalar_b_deferred`.
	/// Valid only after `sync_copy_stream` (or `sync_deferred_scalar`).
	pub fn deferred_scalar(&self) -> f64 {
		unsafe { *self.pinned_scalar }
	}
	pub fn deferred_scalar_b(&self) -> f64 {
		unsafe { *self.pinned_scalar_b }
	}
}

impl Drop for Scratch {
	fn drop(&mut self) {
		if !self.pinned_scalar.is_null() {
			let _ = unsafe { gpu_core::hip::hipHostFree(self.pinned_scalar as *mut std::ffi::c_void) };
		}
		if !self.pinned_scalar_b.is_null() {
			let _ = unsafe { gpu_core::hip::hipHostFree(self.pinned_scalar_b as *mut std::ffi::c_void) };
		}
	}
}

impl Scratch {
	/// Exact bytes `new()` will allocate for these params at row count `n` — the
	/// SUM of every buffer, mirroring `new()` field-for-field. Used to pre-check a
	/// forward pass (esp. eval, where attention's scores + per-head buffers are
	/// huge) against free VRAM, since an over-budget alloc HIP-asserts (core dump)
	/// rather than returning a catchable error.
	pub fn vram_bytes(params: &[LayerParams], n: usize, forward_only: bool) -> usize {
		let bw = |sz: usize| if forward_only { 1 } else { sz };
		let mut max_ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1);
		let (mut max_act, mut max_wt, mut max_bias) = (0usize, 0usize, 0usize);
		let (mut max_embed_grad, mut max_seqd, mut max_scores, mut max_dd) =
			(1usize, 1usize, 1usize, 1usize);
		let mut max_dw_partials = 1usize;
		let mut has_prelu = false;
		let mut floats = 0usize; // acts + preact (per-layer, variable)
		for p in params {
			floats += n * p.out_dim; // acts[l]
			let dw_dp = match p.kind {
				LayerKind::Dense => kernels::gpu_splitk_dw_partials_elems(n, p.in_dim, p.out_dim),
				LayerKind::Attn => {
					let s = p.in_dim / p.dim;
					kernels::gpu_splitk_dw_partials_elems(n * s, p.dim, p.dim)
				}
				_ => 0,
			};
			if dw_dp > max_dw_partials {
				max_dw_partials = dw_dp;
			}
			let needs_pre = matches!(
				p.act,
				Activation::Silu
					| Activation::Gelu | Activation::Elu
					| Activation::Selu | Activation::PRelu
			);
			floats += if needs_pre { n * p.out_dim } else { 1 }; // preact[l]
			if p.act == Activation::PRelu {
				has_prelu = true;
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * p.out_dim, 1);
				if ws > max_ws {
					max_ws = ws;
				}
			}
			let w = kernels::gpu_reduce_sum_cols_workspace_bytes(n, p.out_dim);
			if w > max_ws {
				max_ws = w;
			}
			if n * p.out_dim.max(p.in_dim) > max_act {
				max_act = n * p.out_dim.max(p.in_dim);
			}
			// Mirror new(): dw is sized only by Dense/Conv (Attn→a_gw, Embed→embed_grad).
			let wt = match p.kind {
				LayerKind::Conv => {
					let lin = p.in_dim / p.conv_cin;
					let lout = (lin - p.conv_k) / p.conv_stride + 1;
					let cout = p.out_dim / lout;
					cout * p.conv_cin * p.conv_k
				}
				LayerKind::Dense => p.in_dim * p.out_dim,
				_ => 0,
			};
			if wt > max_wt {
				max_wt = wt;
			}
			if p.out_dim > max_bias {
				max_bias = p.out_dim;
			}
			if p.kind == LayerKind::Embed && p.vocab * p.dim > max_embed_grad {
				max_embed_grad = p.vocab * p.dim;
			}
			if p.kind == LayerKind::Attn {
				let s = p.in_dim / p.dim;
				if n * p.in_dim > max_seqd {
					max_seqd = n * p.in_dim;
				}
				if n * p.heads * s * s > max_scores {
					max_scores = n * p.heads * s * s;
				}
				if p.dim * p.dim > max_dd {
					max_dd = p.dim * p.dim;
				}
				let ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n * s, p.dim);
				if ws > max_ws {
					max_ws = ws;
				}
			}
		}
		let out_elems = n * params.last().map_or(1, |p| p.out_dim);
		floats += 3 * bw(max_act); // da_a, da_b, dz
		floats += bw(max_wt) + bw(max_dw_partials) + bw(max_bias); // dw, dw_partials, db
		floats += 3 * out_elems + 2; // metric_t0/t1/t2, metric_scalar, metric_scalar_b
		floats += bw(max_embed_grad); // embed_grad
		floats += 4 * max_seqd; // a_q,a_k,a_v,a_ctx (forward)
		floats += 4 * bw(max_seqd); // a_dctx,a_dq,a_dk,a_dv (backward)
		if forward_only {
			// FlashAttention inference: no L×L score buffer at all (the fused kernel
			// streams K,V through shared memory). a_scores/a_dscores are len-1 stubs.
			floats += 2;
		} else {
			floats += 2 * max_scores; // a_scores (fwd), a_dscores (bwd)
		}
		floats += 2 * bw(max_dd); // a_gw, a_dbias
		floats += 2 * bw(if has_prelu { max_act } else { 1 }) + 1; // prelu_t0/t1, prelu_scalar
		match concat_layer(params) {
			// concat (fwd) + concat_dgrad (bwd)
			Some((_, a, c)) => {
				floats += n * (a + c) + bw(n * a);
			}
			None => {
				floats += 1 + bw(1);
			}
		}
		floats * std::mem::size_of::<f64>() + max_ws
	}
}

pub fn vram_estimate(specs: &[LayerSpec], n: usize, d: usize, k: usize, vocab: usize, c_cat: usize, forward_only: bool) -> usize {
	let f8 = std::mem::size_of::<f64>();
	let mut bytes = 0usize;
	bytes += 2 * n * d * f8;
	if c_cat > 0 {
		bytes += 2 * n * c_cat * f8;
	}
	bytes += 3 * d * f8;
	bytes += n * k * f8;
	let mut in_dim = d;
	let mut embed_dim = 0usize;
	let mut fake_params: Vec<(usize, usize, LayerKind, usize, usize, Activation, usize)> = Vec::new();
	for spec in specs {
		match *spec {
			LayerSpec::Embed(dim, _) => {
				let seq = in_dim;
				bytes += vocab * dim * f8;
				bytes += seq * dim * f8;
				let out = seq * dim;
				embed_dim = dim;
				fake_params.push((in_dim, out, LayerKind::Embed, dim, vocab, Activation::Linear, 0));
				in_dim = out;
			}
			LayerSpec::Attn(heads) => {
				let d_tok = if embed_dim > 0 { embed_dim } else { in_dim };
				bytes += 4 * d_tok * d_tok * f8;
				bytes += 4 * d_tok * f8;
				fake_params.push((in_dim, in_dim, LayerKind::Attn, d_tok, 0, Activation::Linear, heads));
			}
			LayerSpec::Conv(filters, kernel, stride, act) => {
				let cin = fake_params.last().map_or(1, |(_, _, kind, ..)| {
					if *kind == LayerKind::Conv { 0 } else { 1 }
				});
				let cin = if cin == 0 {
					let prev = fake_params.last().expect("conv cin");
					prev.1 / ((prev.0 / prev.3.max(1) - prev.4) / prev.6.max(1) + 1).max(1)
				} else {
					cin
				};
				let lin = in_dim / cin;
				let lout = (lin - kernel) / stride + 1;
				bytes += filters * cin * kernel * f8;
				bytes += filters * f8;
				if act == Activation::PRelu {
					bytes += f8;
				}
				let out = filters * lout;
				fake_params.push((in_dim, out, LayerKind::Conv, cin, kernel, act, stride));
				in_dim = out;
			}
			LayerSpec::Dense(units, act) => {
				let actual_in = if c_cat > 0 && !fake_params.is_empty()
					&& matches!(fake_params.last(), Some((_, _, LayerKind::Embed | LayerKind::Attn, ..)))
				{
					in_dim + c_cat
				} else {
					in_dim
				};
				bytes += actual_in * units * f8;
				bytes += units * f8;
				if act == Activation::PRelu {
					bytes += f8;
				}
				fake_params.push((actual_in, units, LayerKind::Dense, 0, 0, act, 0));
				in_dim = units;
			}
		}
	}
	let dummy_params: Vec<LayerParams> = fake_params
		.iter()
		.map(|&(i, o, kind, dim, vocab, act, heads)| {
			let dummy = || GpuBuffer::alloc(1).expect("dummy");
			let (cc, ck, cs) = if kind == LayerKind::Conv { (dim, vocab, heads) } else { (0, 0, 0) };
			LayerParams {
				kind,
				w: dummy(), b: dummy(),
				in_dim: i, out_dim: o, act,
				dim: dim.max(1), vocab,
				wk: dummy(), wv: dummy(), wo: dummy(),
				heads,
				palpha: dummy(),
				conv_cin: cc, conv_k: ck, conv_stride: cs,
			}
		})
		.collect();
	if !dummy_params.is_empty() {
		bytes += Scratch::vram_bytes(&dummy_params, n, forward_only);
	}
	bytes
}
