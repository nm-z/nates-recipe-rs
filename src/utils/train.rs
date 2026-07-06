//! Training half of `model`: the full-batch forward+backward fit loop, attention
//! backprop, loss-gradient computation, OGDL checkpoint writing, evaluation, and
//! the ratatui live dashboard. The forward engine and execution enums live in the
//! `recipe-infer` crate; this module drives them but they never depend back on it.
use crate::dataset::{Dataset, collapse_onehot};
use crate::model::{ModelInner, RunData, Train};
use recipe_infer::{
	Activation, LayerKind, LayerParams, LayerSpec,
	Loss, Metric, Scaler, Scratch, build_layer_params, concat_layer,
	forward_into, infer_scored, load_ogdl, metric_gpu, metric_gpu_into,
	pinned_vocab, upload, zscore_apply, zscore_fit,
};
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::symbols::{self, Marker};
use ratatui::text::Span;
use ratatui::widgets::{Axis, Block, Chart, Dataset as ChartDataset, GraphType, Paragraph};
use std::io::IsTerminal as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
/// Set by the SIGINT handler so headless (cooked-mode) Ctrl+C exits gracefully
/// — in TUI raw mode Ctrl+C arrives as a key event instead and is handled there.
pub(crate) static INTERRUPTED: AtomicBool = AtomicBool::new(false);
extern "C" fn on_sigint(_: i32) {
	// Second ^C means OUT NOW — skip checkpoints, async-signal-safe exit.
	if INTERRUPTED.swap(true, Ordering::SeqCst) {
		unsafe { libc::_exit(130) };
	}
}
/// Per-column number colors, applied in `.log([...])` order (cycles past 12).
const PALETTE: [(u8, u8, u8); 12] = [
	(242, 40, 60),   // #F2283C red
	(39, 125, 255),  // #277DFF blue
	(0, 174, 107),   // #00AE6B green
	(255, 194, 0),   // #FFC200 yellow
	(215, 46, 130),  // #D72E82 pink
	(135, 90, 251),  // #875AFB purple
	(255, 122, 0),   // #FF7A00 orange
	(91, 192, 235),  // #5BC0EB
	(157, 121, 188), // #9D79BC
	(46, 83, 57),    // #2E5339
	(3, 252, 186),   // #03FCBA
	(194, 1, 20),    // #C20114
];
/// Palette color for the i-th logged series (cycles).
fn palette(i: usize) -> (u8, u8, u8) {
	PALETTE[i % PALETTE.len()]
}
/// Symmetric-log transform (linthresh = 1): linear in [-1, 1], log10 beyond.
/// Handles negatives and huge magnitudes, so disparate metrics share a y-axis.
fn symlog(y: f64) -> f64 {
	if y.abs() <= 1.0 {
		y
	} else {
		y.signum() * (1.0 + y.abs().log10())
	}
}
/// Inverse of `symlog`, for labeling y ticks back in original units.
fn inv_symlog(v: f64) -> f64 {
	if v.abs() <= 1.0 {
		v
	} else {
		v.signum() * 10f64.powf(v.abs() - 1.0)
	}
}
/// Single-unit time for axis ticks — picks s/m/h by magnitude: `24s`, `2.5m`, `1.2h`.
fn fmt_time_axis(secs: f64) -> String {
	if secs >= 3600.0 {
		format!("{:.1}h", secs / 3600.0)
	} else if secs >= 60.0 {
		format!("{:.1}m", secs / 60.0)
	} else {
		format!("{secs:.0}s")
	}
}
/// Human-readable elapsed time: `45.3s`, `2m 05s`, `1h 03m 20s`.
fn fmt_time(secs: f64) -> String {
	let s = secs as u64;
	let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
	if h > 0 {
		format!("{h}h {m:02}m {sec:02}s")
	} else if m > 0 {
		format!("{m}m {sec:02}s")
	} else {
		format!("{secs:.1}s")
	}
}
/// Compact axis label.
fn fmt_axis(v: f64) -> String {
	let a = v.abs();
	if a >= 1000.0 || (a > 0.0 && a < 0.01) {
		format!("{v:.1e}")
	} else if a >= 1.0 {
		format!("{v:.1}")
	} else {
		format!("{v:.3}")
	}
}
/// Per-fit device-scalar operands the SGD/loss-grad ops now take as 1-elem
/// buffers instead of host f64s: `neg_lr` (= -lr, the DAXPY step), `inv_n`
/// (= 1/n) and `two_inv_n` (= 2/n) for the batch-mean loss gradient, and a
/// reusable `zero` for async accumulator clears. Uploaded ONCE per fit (before
/// the AllocGuard freeze) and reused every epoch — never re-uploaded in-loop.
pub(crate) struct StepScalars {
	pub neg_lr: GpuBuffer,
	pub inv_n: GpuBuffer,
	pub two_inv_n: GpuBuffer,
	pub zero: GpuBuffer,
}
impl StepScalars {
	pub(crate) fn new(lr: f64, n: usize) -> StepScalars {
		let inv = 1.0 / n as f64;
		StepScalars {
			neg_lr: GpuBuffer::upload(&[-lr]).expect("neg_lr"),
			inv_n: GpuBuffer::upload(&[inv]).expect("inv_n"),
			two_inv_n: GpuBuffer::upload(&[2.0 * inv]).expect("two_inv_n"),
			zero: GpuBuffer::upload(&[0.0]).expect("zero"),
		}
	}
}
impl ModelInner {
	/// dL/dA at the output for the chosen loss, scaled by 1/n (batch mean),
	/// written in place into `da` with no allocation. `out` = predictions,
	/// `y` = targets, `total` = n*out_dim. Equals the old allocate-return
	/// `loss_grad` followed by `·(1/n)`, op-for-op.
	pub(crate) fn loss_grad_into(
		loss: Loss,
		out: &GpuBuffer,
		y: &GpuBuffer,
		da: &GpuBuffer,
		n: usize,
		total: usize,
		sc: &Scratch,
		ss: &StepScalars,
	) {
		match loss {
			Loss::Mse => kernels::gpu_sub_scale_into(out, y, &ss.two_inv_n, total, da).expect("mse grad"),
			Loss::Mae => {
				kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da).expect("mae sub");
				kernels::gpu_sign_into(da, total, da).expect("mae sign");
				kernels::gpu_scale_inplace(&ss.inv_n, total, da).expect("mae scale");
			}
			Loss::Huber => {
				kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da).expect("huber sub");
				kernels::gpu_clamp_into(da, &sc.c_neg_one, &sc.c_one, total, da).expect("huber clamp");
				kernels::gpu_scale_inplace(&ss.inv_n, total, da).expect("huber scale");
			}
			Loss::Ce => {
				// Categorical CE fused with softmax: dz = (softmax(logits) − y)/n,
				// w.r.t. the LOGITS. `out` are logits (linear output layer); softmax
				// couples the k outputs (they sum to 1) — unlike Bce's per-output
				// independence. With a Linear output, backward's grad = da = dz.
				let k = total / n;
				kernels::gpu_softmax_rows_into(out, n, k, da).expect("ce softmax"); // da = softmax(logits)
				kernels::gpu_sub_scale_into(da, y, &ss.inv_n, total, da).expect("ce grad"); // da = (softmax − y)/n
			}
			Loss::Bce => {
				// Two-sided BCE gradient (p-y)/(p(1-p))·(1/n) — not the one-sided
				// -y/p. With a sigmoid output this chains to dz = p-y. The global
				// 1/n rides ss.inv_n like every other loss, so per-window OOC
				// calls come out at full-batch scale with no rescale.
				kernels::gpu_bce_grad_into(out, y, &ss.inv_n, total, da).expect("bce grad");
			}
			Loss::Focal => {
				// d focal / d prob, scaled by ss.inv_n. `out` is the sigmoid
				// prob; this chains through the sigmoid backward like Bce.
				gpu_core::losses::gpu_focal_grad_into(out, y, &sc.c_focal_gamma, &sc.c_focal_alpha, &ss.inv_n, total, da).expect("focal grad");
			}
		}
	}
	/// Short column label for a metric.
	fn label(m: Metric) -> &'static str {
		match m {
			Metric::Loss => "loss",
			Metric::Accuracy => "acc",
			Metric::Epoch => "epoch",
			Metric::Lr => "lr",
			Metric::Time => "time",
			Metric::R2 => "r2",
			Metric::Hip => "hip",
		}
	}
	/// One line per layer under each log line: measured fwd+bwd seconds (null-
	/// stream boundary events), analytic GFLOP and VRAM GB from the actual dims,
	/// and the achieved rates as % of the two on-card rooflines (our measured
	/// GEMM ceiling, gfx1101 memory BW). The OOC path prints its streamed-tier
	/// counterpart per sweep with the PCIe/NVMe rooflines it measures at build.
	fn roofline_block(params: &[LayerParams], n: usize, sc: &Scratch) -> String {
		let (fms, bms) = sc.layer_ms(params.len());
		let mut out = String::new();
		let (mut t_ms, mut t_flop, mut t_bytes) = (0.0f64, 0.0f64, 0.0f64);
		for (l, p) in params.iter().enumerate() {
			let w = recipe_infer::layer_fwd(p, n).plus(recipe_infer::layer_bwd(p, n, l == 0));
			let ms = fms[l] + bms[l];
			t_ms += ms;
			t_flop += w.flop;
			t_bytes += w.bytes;
			out.push_str(&Self::roofline_line(
				&format!("L{l} {:<5} {}→{}", Self::kind_label(p), p.in_dim, p.out_dim),
				ms, w.flop, w.bytes,
			));
		}
		if params.len() > 1 {
			out.push_str(&Self::roofline_line("total", t_ms, t_flop, t_bytes));
		}
		out
	}

	fn kind_label(p: &LayerParams) -> &'static str {
		match p.kind {
			LayerKind::Dense => "dense",
			LayerKind::Attn => "attn",
			LayerKind::Conv => "conv",
			LayerKind::Embed => "embed",
		}
	}

	fn roofline_line(label: &str, ms: f64, flop: f64, bytes: f64) -> String {
		let gfs = flop / ms / 1e6; // flop / (ms·1e-3 s) / 1e9
		let gbs = bytes / ms / 1e6;
		format!(
			"    {label:<24} {ms:>8.2}ms  {:>8.3} GFLOP {gfs:>6.1} GF/s {:>3.0}% gemm  {:>7.3} GB {gbs:>6.1} GB/s {:>3.0}% vram\n",
			flop / 1e9,
			100.0 * gfs / recipe_infer::GEMM_GFLOPS,
			bytes / 1e9,
			100.0 * gbs / recipe_infer::VRAM_GBS,
		)
	}

	/// The colored, aligned metric line: `vals[i]` is the precomputed value of
	/// `metrics[i]` (already reduced on the GPU), so this only formats.
	pub(crate) fn metrics_line(&self, metrics: &[Metric], vals: &[f64]) -> String {
		let parts: Vec<String> = metrics
			.iter()
			.zip(vals)
			.filter(|(m, _)| **m != Metric::Hip)
			.enumerate()
			.map(|(i, (&m, &v))| {
				let num = if v.is_nan() && matches!(m, Metric::Lr | Metric::Epoch | Metric::Time) {
					match m {
						Metric::Epoch => format!("{:>5}", "N/A"),
						Metric::Lr => format!("{:>7}", "N/A"),
						Metric::Time => format!("{:>9}", "N/A"),
						_ => unreachable!(),
					}
				} else {
					match m {
						Metric::Epoch => format!("{:>5}", v as usize),
						Metric::Lr => format!("{v:>7}"),
						Metric::Time => format!("{:>9}", fmt_time(v)),
						Metric::Loss => format!("{v:>7.4}"),
						Metric::Accuracy => format!("{v:>6.4}"),
						Metric::R2 => format!("{v:>8.4}"),
						Metric::Hip => unreachable!("Hip filtered from metric line"),
					}
				};
				let (r, g, b) = palette(i);
				format!("{} \x1b[38;2;{r};{g};{b}m{num}\x1b[0m", Self::label(m))
			})
			.collect();
		parts.join("  ")
	}
	/// Render the live dashboard with ratatui: a header block + one Chart widget
	/// per metric (x = epoch), stacked via a Layout that can't overflow.
	fn render_dashboard(
		&self,
		frame: &mut Frame,
		summary: &str,
		rows: &[Vec<f64>],
		ys: &[Metric],
	) {
		let header_h = summary.lines().count() as u16;
		let mut constraints = vec![Constraint::Length(header_h)];
		constraints.extend(ys.iter().map(|_| Constraint::Fill(1)));
		let areas = Layout::vertical(constraints).split(frame.area());
		frame.render_widget(Paragraph::new(summary), areas[0]);
		let xmax = rows.last().map_or(1.0, |r| r[0]).max(1.0);
		let lxmax = xmax.log10().max(1e-9); // x bound in log10(epoch) space
		for (j, &m) in ys.iter().enumerate() {
			// Log x (epoch) + symlog y: the huge early transient (e.g. R2 at
			// -29M, or the initial loss spike) compresses logarithmically while
			// the convergence near the asymptote keeps full linear resolution.
			let pts: Vec<(f64, f64)> = rows
				.iter()
				.map(|r| (r[0].max(1.0).log10(), symlog(r[1 + j])))
				.collect();
			// Bounds live in symlog space; auto-scale tightly to the data so the
			// whole curve fits, with a little padding to keep extremes off edge.
			let lo = pts
				.iter()
				.map(|p| p.1)
				.filter(|v| v.is_finite())
				.fold(f64::INFINITY, f64::min);
			let hi = pts
				.iter()
				.map(|p| p.1)
				.filter(|v| v.is_finite())
				.fold(f64::NEG_INFINITY, f64::max);
			let (ymin, ymax) = if hi > lo {
				let pad = (hi - lo) * 0.05;
				(lo - pad, hi + pad)
			} else if lo.is_finite() {
				(lo - 1.0, lo + 1.0)
			} else {
				(0.0, 1.0)
			};
			// Historical min/max in real units, for the two y tick labels.
			let real_lo = if lo.is_finite() { inv_symlog(lo) } else { 0.0 };
			let real_hi = if hi.is_finite() { inv_symlog(hi) } else { 1.0 };
			let (r, g, b) = palette(j);
			let color = Color::Rgb(r, g, b);
			let ds = ChartDataset::default()
				.marker(Marker::Braille)
				.graph_type(GraphType::Line)
				.style(Style::default().fg(color))
				.data(&pts);
			// Title in the same color as the metric's data, so label ↔ curve.
			// Append the current (latest, untransformed) value: `acc = 0.93`.
			let cur = rows.last().map_or(f64::NAN, |r| r[1 + j]);
			let title = Span::styled(
				format!("{} = {}", Self::label(m), fmt_axis(cur)),
				Style::default().fg(color),
			);
			// Ticks: evenly spaced in transformed space, labeled with the real
			// value via the inverse transform (10^x = elapsed seconds → human
			// readable for x, inv_symlog for y).
			let chart = Chart::new(vec![ds])
				.block(Block::default().title(title))
				.x_axis(Axis::default().bounds([0.0, lxmax]).labels([
					String::new(),                    // origin: implicit
					String::new(),                    // middle: omitted
					fmt_time_axis(10f64.powf(lxmax)), // only the latest time
				]))
				.y_axis(Axis::default().bounds([ymin, ymax]).labels([
					format!("{:>12}", fmt_axis(real_lo)),
					format!("{:>12}", fmt_axis(real_hi)),
				]));
			frame.render_widget(chart, areas[j + 1]);
		}
		// Each Chart draws its own y-axis segment; bridge the title/x-label gaps
		// between them so the shared axis column reads as one continuous line.
		if areas.len() >= 2 {
			let (first, last) = (areas[1], areas[areas.len() - 1]);
			let buf = frame.buffer_mut();
			let mut found = None;
			'find: for x in first.left()..first.right() {
				for y in first.top()..first.bottom() {
					if let Some(c) = buf.cell((x, y))
						&& c.symbol() == symbols::line::VERTICAL
					{
						found = Some((x, c.style()));
						break 'find;
					}
				}
			}
			if let Some((cx, style)) = found {
				// Stop at the last chart's x-axis corner, not its x-label row
				// below it — otherwise the line dangles a tail past the graph.
				for y in first.top()..last.bottom().saturating_sub(1) {
					if let Some(c) = buf.cell_mut((cx, y)) {
						match c.symbol() {
							" " | "" => {
								c.set_symbol(symbols::line::VERTICAL);
								c.set_style(style);
							}
							// Intermediate x-axis corner: tee the vertical
							// straight through to the next chart. The last
							// chart's corner stays └ (nothing below it).
							s if s == symbols::line::BOTTOM_LEFT
								&& y < last.top() =>
							{
								c.set_symbol(symbols::line::VERTICAL_RIGHT);
							}
							_ => {}
						}
					}
				}
			}
		}
	}
	/// Bare multi-head self-attention backward + SGD on Wq/Wk/Wv/Wo. `da` = grad
	/// wrt the layer output, `h` = the layer input (saved by forward), `da_below`
	/// receives dL/dH (= dH_q+dH_k+dH_v). Reverses attn_forward op-for-op. Alloc-free.
	fn attn_backward(
		&self,
		p: &LayerParams,
		h: &GpuBuffer,
		da: &GpuBuffer,
		da_below: &GpuBuffer,
		n: usize,
		sc: &Scratch,
		ss: &StepScalars,
	) {
		let d = p.dim;
		let heads = p.heads;
		let s = p.in_dim / d;
		let m = n * s;
		// out = context·Wo → dcontext (a_dctx), dWo (a_gw); update Wo.
		kernels::gpu_linear_backward_full_into(
			da,
			&sc.a_ctx,
			&p.wo,
			&sc.reduce_ws,
			&sc.dw_partials,
			m,
			d,
			d,
			&sc.a_dctx,
			&sc.a_gw,
			&sc.a_dbias,
		).expect("attn wo backward");
		kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.wo).expect("sgd wo");
		// Flash backward: dQ/dK/dV without materializing scores or dscores — the
		// kernels recompute P tiles from Q,K + the forward's logsumexp (a_lse) and
		// the rowsum term D = Σ dctx∘ctx (a_dsum). Deterministic, no atomics.
		kernels::gpu_flash_attention_backward_into(
			&sc.a_q,
			&sc.a_k,
			&sc.a_v,
			&sc.a_ctx,
			&sc.a_dctx,
			&sc.a_lse,
			n,
			s,
			d,
			heads,
			&sc.a_dsum,
			&sc.a_dq,
			&sc.a_dk,
			&sc.a_dv,
		).expect("flash attn backward");
		// dscores came from ROTATED Q,K, so a_dq/a_dk are gradients w.r.t. the rotated
		// tensors. Un-rotate (RoPE with sgn=-1) to get gradients w.r.t. the raw Q,K
		// projection outputs before back-propagating through Wq/Wk.
		gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.a_dq, &sc.a_dk, &sc.c_neg_one, &sc.c_rope_theta, m, d, heads, s).expect("rope backward");
		// {Q,K,V} = H·{Wq,Wk,Wv}: accumulate dH = dH_q+dH_k+dH_v into da_below; update weights.
		kernels::gpu_linear_backward_full_into(
			&sc.a_dq,
			h,
			&p.w,
			&sc.reduce_ws,
			&sc.dw_partials,
			m,
			d,
			d,
			da_below,
			&sc.a_gw,
			&sc.a_dbias,
		).expect("attn wq backward");
		kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.w).expect("sgd wq");
		kernels::gpu_linear_backward_full_into(
			&sc.a_dk,
			h,
			&p.wk,
			&sc.reduce_ws,
			&sc.dw_partials,
			m,
			d,
			d,
			&sc.a_dctx,
			&sc.a_gw,
			&sc.a_dbias,
		).expect("attn wk backward");
		kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.wk).expect("sgd wk");
		kernels::gpu_add_inplace(&sc.a_dctx, m * d, da_below).expect("attn dh add k");
		kernels::gpu_linear_backward_full_into(
			&sc.a_dv,
			h,
			&p.wv,
			&sc.reduce_ws,
			&sc.dw_partials,
			m,
			d,
			d,
			&sc.a_dctx,
			&sc.a_gw,
			&sc.a_dbias,
		).expect("attn wv backward");
		kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.wv).expect("sgd wv");
		kernels::gpu_add_inplace(&sc.a_dctx, m * d, da_below).expect("attn dh add v");
	}
	/// One backward pass + SGD update, writing every gradient into preallocated
	/// `sc` (no allocation). `sc.acts` must hold this epoch's forward output and
	/// `x` feeds layer 0 as in `forward_into`. Op-for-op identical to the old
	/// in-loop backward: dz = act'(da)·, dw = aᵀ·dz, db = Σ dz, da_below = dz·wᵀ,
	/// then w -= lr·dw, b -= lr·db. `w` is read for da_below before its update.
	pub(crate) fn backward_step(
		&self,
		params: &[LayerParams],
		x: &GpuBuffer,
		ybuf: &GpuBuffer,
		n: usize,
		sc: &Scratch,
		ss: &StepScalars,
	) {
		let last = params.len() - 1;
		// The first dense after the text prefix was fed sc.concat in the forward pass
		// (still resident) — read it back as that layer's input on the way down.
		let cc = concat_layer(params);
		let (da_cur, da_next) = (&sc.da_a, &sc.da_b);
		Self::loss_grad_into(
			self.loss,
			&sc.acts[last],
			ybuf,
			da_cur,
			n,
			n * params[last].out_dim,
			sc,
			ss,
		);
		sc.mark_bwd(params.len());
		let mut flip = false;
		for l in (0..params.len()).rev() {
			let (in_dim, out_dim) = (params[l].in_dim, params[l].out_dim);
			let m = n * out_dim;
			let da = if flip { da_next } else { da_cur };
			let da_below = if flip { da_cur } else { da_next };
			if params[l].kind == LayerKind::Embed {
				// Embed is the input layer (no da_below). Its incoming grad da
				// is [n*in_dim, dim]; scatter-add it by token id into a zeroed
				// table-gradient, then SGD the table. x holds the token ids.
				let p = &params[l];
				kernels::gpu_scale_inplace(&ss.zero, p.vocab * p.dim, &sc.embed_grad).expect("embed zero");
				kernels::gpu_scatter_add(x, da, n * p.in_dim, p.dim, &sc.embed_grad).expect("embed scatter");
				kernels::gpu_sgd_update(&sc.embed_grad, &ss.neg_lr, p.vocab * p.dim, &p.w).expect("sgd embed");
				sc.mark_bwd(l);
				flip = !flip;
				continue;
			}
			if params[l].kind == LayerKind::Attn {
				let a_prev = if l == 0 { x } else { &sc.acts[l - 1] };
				self.attn_backward(&params[l], a_prev, da, da_below, n, sc, ss);
				sc.mark_bwd(l);
				flip = !flip;
				continue;
			}
			if params[l].kind == LayerKind::Conv {
				let p = &params[l];
				let (cin, k, stride) = (p.conv_cin, p.conv_k, p.conv_stride);
				let lin = p.in_dim / cin;
				let cout = p.out_dim / ((lin - k) / stride + 1);
				let lout = (lin - k) / stride + 1;
				let grad = match p.act {
					Activation::Relu => { kernels::gpu_relu_backward_into(da, &sc.acts[l], m, &sc.dz).expect("relu bwd"); &sc.dz }
					Activation::Sigmoid => { kernels::gpu_sigmoid_backward_into(da, &sc.acts[l], m, &sc.dz).expect("sigmoid bwd"); &sc.dz }
					Activation::LeakyRelu => { kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &sc.c_leaky_alpha, m, &sc.dz).expect("leaky bwd"); &sc.dz }
					Activation::PRelu => {
						kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &p.palpha, m, &sc.dz).expect("prelu bwd");
						kernels::gpu_relu_into(&sc.preact[l], m, &sc.prelu_t0).expect("prelu relu");
						kernels::gpu_copy_into(&sc.preact[l], m, &sc.prelu_t1).expect("prelu copy");
						kernels::gpu_sub_inplace(&sc.prelu_t0, m, &sc.prelu_t1).expect("prelu sub");
						kernels::gpu_mul_inplace(da, m, &sc.prelu_t1).expect("prelu mul");
						kernels::gpu_reduce_sum_cols_into(&sc.prelu_t1, &sc.reduce_ws, m, 1, &sc.prelu_scalar).expect("prelu reduce");
						kernels::gpu_sgd_update(&sc.prelu_scalar, &ss.neg_lr, 1, &p.palpha).expect("sgd prelu");
						&sc.dz
					}
					Activation::Tanh => { kernels::gpu_tanh_backward_into(da, &sc.acts[l], m, &sc.dz).expect("tanh bwd"); &sc.dz }
					Activation::Elu => { gpu_core::k_gapact::gpu_elu_backward(da, &sc.preact[l], &sc.c_elu_alpha, m, &sc.dz).expect("elu bwd"); &sc.dz }
					Activation::Selu => { gpu_core::k_gapact::gpu_selu_backward(da, &sc.preact[l], &sc.c_selu_alpha, &sc.c_selu_lambda, m, &sc.dz).expect("selu bwd"); &sc.dz }
					Activation::Silu => { kernels::gpu_silu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("silu bwd"); &sc.dz }
					Activation::Gelu => { kernels::gpu_gelu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("gelu bwd"); &sc.dz }
					Activation::Linear => da,
				};
				let a_prev = if l == 0 { x } else { &sc.acts[l - 1] };
				kernels::gpu_conv1d_backward_filter_into(
					grad, a_prev, &sc.conv_temp, &sc.reduce_ws,
					n, cin, lin, cout, k, stride, sc.conv_wg,
					&sc.dw,
				).expect("conv filter bwd");
				kernels::gpu_scale_inplace(&ss.zero, cout, &sc.db).expect("conv db zero");
				kernels::gpu_conv1d_backward_bias_into(grad, n, cout, lout, &sc.db).expect("conv bias bwd");
				if l > 0 {
					kernels::gpu_conv1d_backward_data_into(grad, &p.w, n, cin, lin, cout, k, stride, da_below).expect("conv data bwd");
				}
				kernels::gpu_sgd_update(&sc.dw, &ss.neg_lr, cout * cin * k, &p.w).expect("sgd conv w");
				kernels::gpu_sgd_update(&sc.db, &ss.neg_lr, cout, &p.b).expect("sgd conv b");
				sc.mark_bwd(l);
				flip = !flip;
				continue;
			}
			let grad = match params[l].act {
				Activation::Relu => {
					kernels::gpu_relu_backward_into(da, &sc.acts[l], m, &sc.dz).expect("relu bwd");
					&sc.dz
				}
				Activation::Sigmoid => {
					kernels::gpu_sigmoid_backward_into(da, &sc.acts[l], m, &sc.dz).expect("sigmoid bwd");
					&sc.dz
				}
				Activation::LeakyRelu => {
					kernels::gpu_leaky_relu_backward_into(
						da,
						&sc.acts[l],
						&sc.c_leaky_alpha,
						m,
						&sc.dz,
					).expect("leaky bwd");
					&sc.dz
				}
				Activation::PRelu => {
					// dx uses the current slope; then update the slope:
					// dα = Σ grad·min(z,0), with min(z,0) = z − relu(z).
					kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &params[l].palpha, m, &sc.dz).expect("prelu bwd");
					kernels::gpu_relu_into(&sc.preact[l], m, &sc.prelu_t0).expect("prelu relu");
					kernels::gpu_copy_into(&sc.preact[l], m, &sc.prelu_t1).expect("prelu copy");
					kernels::gpu_sub_inplace(&sc.prelu_t0, m, &sc.prelu_t1).expect("prelu sub");
					kernels::gpu_mul_inplace(da, m, &sc.prelu_t1).expect("prelu mul");
					kernels::gpu_reduce_sum_cols_into(
						&sc.prelu_t1,
						&sc.reduce_ws,
						m,
						1,
						&sc.prelu_scalar,
					).expect("prelu reduce");
					kernels::gpu_sgd_update(
						&sc.prelu_scalar,
						&ss.neg_lr,
						1,
						&params[l].palpha,
					).expect("sgd prelu");
					&sc.dz
				}
				Activation::Tanh => {
					kernels::gpu_tanh_backward_into(da, &sc.acts[l], m, &sc.dz).expect("tanh bwd");
					&sc.dz
				}
				// Elu/Selu/Silu/Gelu backward read the saved pre-activation, not the output.
				Activation::Elu => {
					gpu_core::k_gapact::gpu_elu_backward(
						da,
						&sc.preact[l],
						&sc.c_elu_alpha,
						m,
						&sc.dz,
					).expect("elu bwd");
					&sc.dz
				}
				Activation::Selu => {
					gpu_core::k_gapact::gpu_selu_backward(
						da,
						&sc.preact[l],
						&sc.c_selu_alpha,
						&sc.c_selu_lambda,
						m,
						&sc.dz,
					).expect("selu bwd");
					&sc.dz
				}
				Activation::Silu => {
					kernels::gpu_silu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("silu bwd");
					&sc.dz
				}
				Activation::Gelu => {
					kernels::gpu_gelu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("gelu bwd");
					&sc.dz
				}
				Activation::Linear => da,
			};
			let at_concat = Some(l) == cc.map(|t| t.0);
			let a_prev = if l == 0 {
				x
			} else if at_concat {
				&sc.concat
			} else {
				&sc.acts[l - 1]
			};
			if out_dim == 1 {
				// dW = a_prevᵀ·grad reduced over n batch rows — split-K across all CUs.
				kernels::gpu_splitk_dw_into(a_prev, grad, &sc.dw_partials, n, 1, in_dim, &sc.dw).expect("splitk dw");
				kernels::gpu_reduce_sum_cols_into(grad, &sc.reduce_ws, n, 1, &sc.db).expect("db reduce");
				if l > 0 {
					kernels::gpu_dger_into(grad, &params[l].w, da_below, n, in_dim).expect("dger");
				}
			} else if l > 0 {
				kernels::gpu_linear_backward_full_into(
					grad,
					a_prev,
					&params[l].w,
					&sc.reduce_ws,
					&sc.dw_partials,
					n,
					out_dim,
					in_dim,
					da_below,
					&sc.dw,
					&sc.db,
				).expect("linear full bwd");
			} else {
				kernels::gpu_linear_backward_weights_only_into(
					grad,
					a_prev,
					&sc.reduce_ws,
					&sc.dw_partials,
					n,
					out_dim,
					in_dim,
					&sc.dw,
					&sc.db,
				).expect("linear weights bwd");
			}
			kernels::gpu_sgd_update(&sc.dw, &ss.neg_lr, in_dim * out_dim, &params[l].w).expect("sgd w");
			kernels::gpu_sgd_update(&sc.db, &ss.neg_lr, out_dim, &params[l].b).expect("sgd b");
			// da_below here is [n×(A+C)]; the attn layer below only wants the A
			// attention columns — compact them to the front so the next iteration
			// reads a contiguous [n×A] grad. (The trailing C cat-grads are inputs.)
			if let Some((pf, a, c)) = cc
				&& l == pf
			{
				kernels::gpu_slice_lead_into(da_below, n, a + c, a, &sc.concat_dgrad).expect("concat slice");
				kernels::gpu_copy_into(&sc.concat_dgrad, n * a, da_below).expect("concat copy");
			}
			sc.mark_bwd(l);
			flip = !flip;
		}
	}
	pub(crate) fn fit(&self, data: &Dataset, cfg: &Train, resume: Option<&str>, net: Option<std::sync::Arc<Vec<crate::wire::Conn>>>) {
		let hip_snap = cfg.metrics.contains(&Metric::Hip).then(gpu_core::callspy::snapshot);
		let rerun = !self.params.borrow().is_empty();
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats = embed_first && data.text_cols.is_empty() && !data.onehot_groups.is_empty();
		let (collapsed_x, collapsed_embed_cols, collapsed_vocab) = if embed_cats {
			let (x, ec, v) = collapse_onehot(data);
			(Some(x), ec, v)
		} else {
			(None, Vec::new(), 0)
		};
		let effective_x = collapsed_x.as_ref().unwrap_or(&data.x);
		let effective_text = if embed_cats { &collapsed_embed_cols } else { &data.text_cols };
		let cat_cols: Vec<usize> = if embed_first {
			(0..effective_x.ncols())
				.filter(|c| !effective_text.contains(c))
				.collect()
		} else {
			Vec::new()
		};
		let c_cat = cat_cols.len();
		let xinput = if embed_first {
			effective_x.select(ndarray::Axis(1), effective_text)
		} else {
			effective_x.clone()
		};
		let vocab = if let Some(v) = pinned_vocab(&self.specs) {
			v
		} else if embed_first {
			if embed_cats {
				collapsed_vocab
			} else {
				xinput.iter().cloned().fold(0.0f64, f64::max) as usize + 1
			}
		} else {
			0
		};
		let start = std::time::Instant::now();
		let _t_data = gpu_core::memory::tag_scope("data");
		let (xraw, n, d) = upload(&xinput);
		// Text token ids pass RAW to the embed lookup (no z-score). The categorical
		// branch IS z-scored on the train set (raw frequency-encoded columns span
		// wildly different magnitudes; unscaled they saturate the dense head). For a
		// non-embed model the whole matrix is the categorical branch. The scaler is
		// fit once here and reused verbatim on eval (no leakage).
		let (xbuf, x_cat) = if embed_first {
			if cat_cols.is_empty() {
				if !rerun {
					*self.scaler.borrow_mut() = Some(Scaler {
						mean: vec![],
						std: vec![],
					});
				}
				(xraw, None)
			} else {
				let cat = effective_x.select(ndarray::Axis(1), &cat_cols);
				let (craw, _, c) = upload(&cat);
				if rerun {
					let sc = self.scaler.borrow();
					let sc = sc.as_ref().expect("rerun without scaler");
					(xraw, Some(zscore_apply(&craw, n, c, sc)))
				} else {
					let ccat = zscore_fit(&craw, n, c, &self.scaler);
					(xraw, Some(ccat))
				}
			}
		} else if rerun {
			let sc = self.scaler.borrow();
			let sc = sc.as_ref().expect("rerun without scaler");
			(zscore_apply(&xraw, n, d, sc), None)
		} else {
			(zscore_fit(&xraw, n, d, &self.scaler), None)
		};
		let y_host = {
			let ys = data.y.as_slice().expect("train: y contiguous");
			let mut ydata = ys.to_vec();
			if !self.loss.is_classification() && !rerun {
				let ymean = ydata.iter().sum::<f64>() / ydata.len() as f64;
				let yvar = ydata.iter().map(|v| (v - ymean).powi(2)).sum::<f64>() / ydata.len() as f64;
				let ystd = (yvar + 1e-8).sqrt();
				for v in ydata.iter_mut() {
					*v = (*v - ymean) / ystd;
				}
				*self.yscaler.borrow_mut() = Some((ymean, ystd));
			} else if !self.loss.is_classification() && rerun {
				if let Some((ymean, ystd)) = *self.yscaler.borrow() {
					for v in ydata.iter_mut() {
						*v = (*v - ymean) / ystd;
					}
				}
			}
			ydata
		};
		// Resumed weights (per-neuron, in save order) or empty for random init.
		let resumed = resume.map(load_ogdl).unwrap_or_default();
		// On a checkpoint/architecture mismatch, ask whether to overwrite with random
		// weights (y) or abort (n). build_layer_params(.., false) re-runs construction with
		// random init, so "overwrite" is a clean fresh start the next save writes over the stale file.
		let mut did_resume = !resumed.is_empty();
		let ask_overwrite = |what: &str| -> bool {
			use std::io::Write;
			eprintln!(
				"\x1b[32mresume\x1b[0m\n    \x1b[1;31mdata does not match\x1b[0m\n        {what}\n        file path={}\n        data path={}",
				resume.unwrap_or(""),
				data.source,
			);
			if !std::io::stdin().is_terminal() {
				return false;
			}
			eprint!("overwrite checkpoint with random weights? [y/N] ");
			std::io::stderr().flush().ok();
			let mut line = String::new();
			std::io::stdin().read_line(&mut line).ok();
			matches!(line.trim(), "y" | "Y" | "yes" | "YES")
		};
		// Build every layer's params, consuming parsed checkpoint blocks when try_resume.
		// A shape/order mismatch returns Err(reason) (not abort) so the caller can prompt.
		// `si` indexes blocks: one per embed/attn layer, one per dense neuron, in order.
		let params = {
			let _t_weights = gpu_core::memory::tag_scope("weights");
			match build_layer_params(&self.specs, d, c_cat, vocab, &resumed, did_resume) {
				Ok(p) => p,
				Err(what) => {
					if ask_overwrite(&what) {
						did_resume = false;
						build_layer_params(&self.specs, d, c_cat, vocab, &resumed, false).unwrap_or_else(|e| panic!("{e}"))
					} else {
						panic!("checkpoint mismatch — user declined overwrite");
					}
				}
			}
		};
		let last = params.len() - 1;
		// A categorical target arrives as ONE class-index column; for multi-class CE
		// the model's output width IS the class count, so expand the index to a
		// one-hot of `out_dim` here (reusing the dense one-hot CE/accuracy path).
		// Binary (bce, out_dim==1) and regression/multi-output use the column as-is.
		let n_targets = data.n_targets.max(1);
		let out_dim = params[last].out_dim;
		let expand_ce = self.loss.is_classification() && n_targets == 1 && out_dim > 1;
		let k = if expand_ce { out_dim } else { n_targets };
		// Output units must equal the target width: y is flat n*k and acts[last] is
		// n*out_dim — they must align element-for-element (the expanded one-hot does).
		assert_eq!(
			out_dim, k,
			"output layer has {out_dim} units but there are {n_targets} target column(s) — make the last .layer({n_targets})"
		);
		let ybuf = if expand_ce {
			let mut oh = vec![0.0f64; n * out_dim];
			for (i, &idx) in y_host.iter().enumerate() {
				if idx.is_finite() {
					let c = idx as usize;
					if c < out_dim {
						oh[i * out_dim + c] = 1.0;
					}
				}
			}
			GpuBuffer::upload(&oh).expect("upload y onehot")
		} else {
			GpuBuffer::upload(&y_host).expect("upload y")
		};
		let summary = if cfg.metrics.is_empty() {
			String::new()
		} else {
			let neurons: usize = params.iter().map(|p| p.out_dim).sum();
			let out = params[last].out_dim;
			let row = |x: usize, l1: &str, y: usize, l2: &str| {
				format!("    {x:>5}  {l1:<11}{y:>5}  {l2}")
			};
			[
				"arch".to_string(),
				row(neurons, "neurons", params.len(), "layers"),
				row(n, "samples", d, "features"),
				row(d, "input_dim", out, "output_dim"),
				"data".to_string(),
				row(n + 1, "rows", d + 1, "columns"),
				row(d, "predictors", out, "targets"),
			]
			.join("\n")
		};
		// Epoch is the x-axis; Time is wall-clock (an axis quantity), not a
		// y-series. Both are excluded from the facets — they're independent
		// variables, not datapoints. They still appear in the metrics header.
		let plot_ys: Vec<Metric> = cfg
			.plot
			.iter()
			.copied()
			.filter(|&m| m != Metric::Epoch && m != Metric::Time)
			.collect();
		let mut plot_rows: Vec<Vec<f64>> = Vec::new();
		// Only take over the screen when stdout is a real terminal; piped or
		// headless runs fall through to the stderr log path. ratatui owns the
		// terminal (alt screen, raw mode, panic-restore hook); Ctrl+C arrives
		// as a key event in raw mode and is handled in the loop.
		let plotting = !cfg.plot.is_empty() && std::io::stdout().is_terminal();
		if !plotting && !rerun {
			if did_resume && let Some(path) = resume {
				let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
				eprintln!("resumed: {}", full.display());
			}
			if !summary.is_empty() {
				eprintln!("{summary}");
				eprintln!(
					"roofline  gemm {} GF/s  vram {} GB/s",
					recipe_infer::GEMM_GFLOPS,
					recipe_infer::VRAM_GBS
				);
			}
		}
		let mut terminal = plotting.then(ratatui::init);
		let mut last_draw = start;
		let checkpoint_path = cfg.resume.as_deref().map(Train::resolve);
		let checkpointing = checkpoint_path.is_some();
		let classify = self.loss.is_classification();
		let mut loss_prev = f64::INFINITY;
		let mut saved = false;
		// Per-epoch metrics reduce to a scalar on the GPU; only the requested ones
		// are downloaded. SS_tot (R²'s denominator) depends only on the constant
		// targets, so compute it once here.
		// On the SAME (z-scored) scale as the training-loop ss_res — `out` predicts
		// the z-scored target, so ss_tot must use the z-scored y too. Using raw
		// data.y here made R² ≈ 1 always (huge ss_tot vs a z-scored residual).
		let ss_tot = {
			let total = y_host.len() as f64;
			let ybar = y_host.iter().sum::<f64>() / total;
			y_host.iter().map(|v| (v - ybar).powi(2)).sum::<f64>()
		};
		// Activation + gradient buffers, allocated once and reused every epoch
		// so steady-state VRAM is flat (no per-epoch sawtooth).
		// Waterfall: when the full-batch scratch exceeds free VRAM, homes spill
		// VRAM → RAM → DISK and every op streams sample windows (crate::ooc).
		let cc_fit = concat_layer(&params);
		let scratch_need = Scratch::vram_bytes(&params, n, false);
		let use_ooc = scratch_need > gpu_core::memory::claimable_bytes();
		let sc = {
			let _t_scratch = gpu_core::memory::tag_scope("scratch");
			if use_ooc { Scratch::new_light(&params, n) } else { Scratch::new(&params, n, false) }
		};
		let mut ooc = use_ooc.then(|| {
			let _t_scratch = gpu_core::memory::tag_scope("waterfall");
			let o = crate::ooc::Ooc::build(&params, n, cc_fit.map(|(_, a, c)| (a, c)), net.clone());
			o.report();
			o
		});
		// Device-scalar operands for SGD (-lr) and the batch-mean loss gradient
		// (1/n, 2/n) + a reusable zero — uploaded once here, reused every epoch.
		let ss = StepScalars::new(self.lr, n);
		let _alloc_guard = gpu_core::memory::AllocGuard::freeze();
		// Saturation law, executable: the GPU must hit 100% at least once in
		// every 5s window of the compute loop or the process aborts.
		gpu_core::hw::arm_saturation_crash();
		INTERRUPTED.store(false, Ordering::SeqCst);
		unsafe {
			libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
		}
		let mut fit_score = f64::NAN;
		for e in 0..cfg.epochs {
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			let log_now = cfg.log_every > 0
				&& !cfg.metrics.is_empty()
				&& (e % cfg.log_every == 0 || e + 1 == cfg.epochs);
			// Per-layer boundary events armed only for epochs whose log line will
			// print (the OOC path prints its own per-sweep roofline lines).
			let time_layers = log_now && !plotting && ooc.is_none();
			sc.set_timing(time_layers);
			// Forward with this epoch's weights, then backprop + SGD update.
			match ooc.as_mut() {
				Some(o) => o.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit),
				None => forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc),
			}
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			let stop_metric = if classify {
				Metric::Accuracy
			} else {
				Metric::R2
			};
			let want_score = checkpointing
				|| (log_now && cfg.metrics.contains(&stop_metric))
				|| (plotting && plot_ys.contains(&stop_metric));
			match ooc.as_mut() {
				Some(o) => o.backward(&params, &xbuf, &ybuf, &sc, &ss, self.loss, cc_fit),
				None => self.backward_step(&params, &xbuf, &ybuf, n, &sc, &ss),
			}
			// Disarm before the metric re-forward so it can't overwrite the epoch's
			// recorded boundaries.
			sc.set_timing(false);
			let need_metric = want_score
				|| (log_now && !cfg.metrics.is_empty())
				|| (plotting && !plot_ys.is_empty());
			if need_metric {
				match ooc.as_mut() {
					Some(o) => o.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit),
					None => forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc),
				}
			}
			// ^C mid metric-forward bails the OOC sweep early: acts[last] still
			// holds the previous weights' output and the loss line would repeat
			// verbatim after an update. Break before computing/printing from it.
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			let out = &sc.acts[last];
			// Per-epoch metrics ride the async copy stream and sync ONCE (no blocking
			// 8-byte D2H per metric): score → metric_scalar_b, loss → metric_scalar.
			let score_is_acc = if want_score {
				if classify {
					if k == 1 {
						kernels::gpu_accuracy_into(out, &ybuf, n, &sc.metric_scalar_b).expect("accuracy");
					} else {
						kernels::gpu_argmax_accuracy_into(out, &ybuf, n, k, &sc.metric_scalar_b).expect("argmax accuracy");
					}
					Some(true)
				} else {
					kernels::gpu_ss_res_into(out, &ybuf, n * k, &sc.metric_scalar_b).expect("ss_res");
					Some(false)
				}
			} else {
				None
			};
			if score_is_acc.is_some() {
				sc.download_scalar_b_deferred();
			}
			let loss_scale = if checkpointing {
				let (sign, div) = metric_gpu_into(self.loss, Metric::Loss, out, &ybuf, &sc, n, k, ss_tot);
				sc.download_scalar_deferred();
				Some((sign, div))
			} else {
				None
			};
			if score_is_acc.is_some() || loss_scale.is_some() {
				sc.sync_copy_stream();
			}
			let score = match score_is_acc {
				Some(true) => sc.deferred_scalar_b(),
				Some(false) => 1.0 - sc.deferred_scalar_b() / ss_tot,
				None => f64::NAN,
			};
			if score.is_finite() {
				fit_score = score;
			}
			let loss = if let Some((sign, div)) = loss_scale {
				sign * sc.deferred_scalar() / div
			} else {
				f64::NAN
			};
			if checkpointing && loss.is_nan() {
				eprintln!("NaN loss at epoch {e} — stopping (weights diverged)");
				break;
			}
			let mut checkpointed = false;
			if checkpointing {
				if !saved && e > 0 && loss > loss_prev {
					saved = true;
					let path = checkpoint_path.as_ref().expect("checkpoint path");
					let key = self.loss.score_key();
					if recipe_infer::saved_score(path, key).is_none_or(|best| score > best) {
						recipe_infer::write_ogdl(
							path,
							&recipe_infer::dump_ogdl(&params, None, key, score),
						);
						checkpointed = true;
					}
				}
				if loss.is_finite() {
					loss_prev = loss;
				}
			}
			let last_epoch = e + 1 == cfg.epochs;
			if log_now || checkpointed || plotting {
				let elapsed = start.elapsed().as_secs_f64();
				if !plotting && (log_now || checkpointed) {
					let vals: Vec<f64> = cfg
						.metrics
						.iter()
						.map(|&m| {
							if m == stop_metric {
								score
							} else {
								metric_gpu(
									self.loss, self.lr, m, out, &ybuf, &sc, n, k, ss_tot, e, elapsed,
								)
							}
						})
						.collect();
					let mut line = self.metrics_line(&cfg.metrics, &vals);
					if checkpointed {
						line.push_str("  \x1b[1;32m← checkpoint\x1b[0m");
					}
					eprintln!("{line}");
					if time_layers {
						eprint!("{}", Self::roofline_block(&params, n, &sc));
					}
				}
				if plotting {
					let mut row = vec![elapsed]; // x = elapsed wall-clock seconds
					for &m in &plot_ys {
						row.push(if m == stop_metric {
							score
						} else {
							metric_gpu(
								self.loss, self.lr, m, out, &ybuf, &sc, n, k, ss_tot, e, elapsed,
							)
						});
					}
					plot_rows.push(row);
					// Throttle live redraws to ~25 fps; always draw the last frame.
					if (e == 0 || last_epoch || last_draw.elapsed().as_millis() >= 40)
						&& let Some(term) = terminal.as_mut()
					{
						let _ = term.draw(|frame| {
							self.render_dashboard(
								frame, &summary, &plot_rows, &plot_ys,
							);
						});
						last_draw = std::time::Instant::now();
					}
					// Quit early on q / Ctrl+C (raw mode delivers them as keys).
					if event::poll(Duration::ZERO).unwrap_or(false)
						&& let Ok(Event::Key(k)) = event::read()
						&& (k.code == KeyCode::Char('q')
							|| (k.code == KeyCode::Char('c')
								&& k.modifiers.contains(KeyModifiers::CONTROL)))
					{
						break;
					}
				}
			}
		}
		gpu_core::hw::disarm_saturation_crash();
		drop(_alloc_guard);
		unsafe {
			libc::signal(libc::SIGINT, libc::SIG_DFL);
		}
		if plotting {
			ratatui::restore();
		}
		let mut ooc_end = ooc;
		let was_interrupted = INTERRUPTED.load(Ordering::SeqCst);
		// Post-update score: checkpoint saves need it, and an out-of-core fit is
		// the ONLY place one can be computed (run()'s scoring forward may not
		// fit), so every OOC fit computes it. Except on ^C — the OOC forward
		// would bail on its first window and the metric would read stale
		// mixed-epoch activations; the last per-epoch score stands instead.
		let want_end = if ooc_end.is_some() { !was_interrupted } else { checkpointing };
		let end_score = want_end.then(|| {
			match ooc_end.as_mut() {
				Some(o) => o.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit),
				None => forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc),
			}
			if classify {
				if k == 1 {
					kernels::gpu_accuracy_into(
						&sc.acts[last],
						&ybuf,
						n,
						&sc.metric_scalar,
					).expect("accuracy");
				} else {
					kernels::gpu_argmax_accuracy_into(
						&sc.acts[last],
						&ybuf,
						n,
						k,
						&sc.metric_scalar,
					).expect("argmax accuracy");
				}
				sc.read_metric_scalar()
			} else {
				kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, n * k, &sc.metric_scalar).expect("ss_res");
				1.0 - sc.read_metric_scalar() / ss_tot
			}
		});
		*self.params.borrow_mut() = params;
		if let Some(s) = end_score
			&& s.is_finite()
		{
			fit_score = s;
		}
		self.fit_score.set(fit_score);
		if let Some(path) = checkpoint_path.as_ref() {
			if was_interrupted {
				// ^C save is unconditional — the weights in memory are the only
				// copy — scored with the best available (end score for an
				// in-VRAM fit, last per-epoch score for OOC, NaN if no epoch
				// finished; honest, never fabricated from a bailed forward).
				let key = self.loss.score_key();
				recipe_infer::write_ogdl(
					path,
					&recipe_infer::dump_ogdl(&self.params.borrow(), None, key, fit_score),
				);
				let full =
					std::fs::canonicalize(path).unwrap_or_else(|_| path.as_str().into());
				eprintln!("saved {} ({key} {fit_score:.4})", full.display());
			} else if let Some(s) = end_score {
				self.save_checkpoint(path, s);
			}
		}
		if let Some(base) = hip_snap {
			eprint!("{}", gpu_core::callspy::report_since(&base));
		}
		// A fit leaves its scratch (~all of VRAM for an out-of-core run) as
		// freed pool slack with a stale verified high-water; a later
		// differently-shaped allocation storm would skip the growth gate yet
		// still map new physical out of fragmented slack (the VmHeap assert),
		// and every claimable-VRAM read would see the slack as gone. Trim +
		// reset after EVERY fit so the gate and the measure stay honest.
		drop(ooc_end);
		drop(sc);
		gpu_core::memory::pool_trim();
	}
	fn save_checkpoint(&self, path: &str, score: f64) {
		let params = self.params.borrow();
		assert!(!params.is_empty(), "save: call train() first");
		let key = self.loss.score_key();
		if !score.is_finite() || recipe_infer::saved_score(path, key).is_some_and(|best| score <= best)
		{
			return;
		}
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		recipe_infer::write_ogdl(path, &recipe_infer::dump_ogdl(&params, None, key, score));
		let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
		eprintln!(
			"saved {} ({neurons} neurons, {key} {score:.4})",
			full.display()
		);
	}
	/// Adapt a `Dataset` to GPU input buffers exactly as training did — collapse
	/// one-hot for an embed-first model, split text vs categorical columns, and
	/// apply the TRAIN-set scaler (same mean/std the model saw). Shared by `eval`
	/// and `Train::run`'s inference branch so forward-only input construction
	/// lives in ONE place; the forward itself is recipe-infer's `infer_scored`.
	pub(crate) fn prep_eval_input(&self, ds: &Dataset) -> (GpuBuffer, Option<GpuBuffer>, usize) {
		let n = ds.x.nrows();
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
		let (collapsed_x, collapsed_embed_cols, _v) = if embed_cats {
			let (x, ec, v) = collapse_onehot(ds);
			(Some(x), ec, v)
		} else {
			(None, Vec::new(), 0)
		};
		let eff_x = collapsed_x.as_ref().unwrap_or(&ds.x);
		let eff_text = if embed_cats { &collapsed_embed_cols } else { &ds.text_cols };
		let cat_cols: Vec<usize> = if embed_first {
			(0..eff_x.ncols()).filter(|c| !eff_text.contains(c)).collect()
		} else {
			Vec::new()
		};
		let xinput = if embed_first {
			eff_x.select(ndarray::Axis(1), eff_text)
		} else {
			eff_x.clone()
		};
		let d = xinput.ncols();
		let scaler = self.scaler.borrow();
		let scaler_ref = scaler.as_ref().expect("eval: missing scaler; train first");
		if embed_first {
			let (xraw, _, _) = upload(&xinput);
			if cat_cols.is_empty() {
				(xraw, None, n)
			} else {
				let cat = eff_x.select(ndarray::Axis(1), &cat_cols);
				let (craw, _, c) = upload(&cat);
				(xraw, Some(zscore_apply(&craw, n, c, scaler_ref)), n)
			}
		} else {
			let (xraw, _, _) = upload(&xinput);
			(zscore_apply(&xraw, n, d, scaler_ref), None, n)
		}
	}
	/// Forward-only evaluation of the trained model on `data`, reporting the
	/// loss-appropriate score (accuracy for classification, R² for regression).
	/// The forward+score is recipe-infer's `infer_scored`; this only adapts the
	/// Dataset. Returns the raw `n*k` predictions.
	pub fn eval(&self, data: &impl RunData) -> Vec<f64> {
		let prepared = data.prepared();
		let ds = prepared.get();
		let (xbuf, x_cat, n) = self.prep_eval_input(ds);
		let params = self.params.borrow();
		assert!(!params.is_empty(), "eval: call train() first");
		let yscaler = *self.yscaler.borrow();
		let metric = if self.loss.is_classification() { Metric::Accuracy } else { Metric::R2 };
		if ds.has_target {
			let ybuf = GpuBuffer::upload(ds.y.as_slice().expect("eval: y contiguous"))
				.expect("eval ybuf");
			let k = params[params.len() - 1].out_dim;
			let total = (n * k) as f64;
			let ybar = ds.y.iter().sum::<f64>() / total;
			let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
			let (preds, vals) = infer_scored(
				&params, &xbuf, x_cat.as_ref(), n, yscaler, Some(&ybuf),
				self.loss, self.lr, std::slice::from_ref(&metric), ss_tot,
			);
			let label = if self.loss.is_classification() { "accuracy" } else { "R2" };
			eprintln!("eval: {label} = {:.4} ({n} samples)", vals[0]);
			preds
		} else {
			let (preds, _) = infer_scored(
				&params, &xbuf, x_cat.as_ref(), n, yscaler, None,
				self.loss, self.lr, &[], 0.0,
			);
			eprintln!("eval: {n} samples (no target column, score unavailable)");
			preds
		}
	}
}
