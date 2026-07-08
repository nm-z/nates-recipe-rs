//! Training half of `model`: the full-batch forward+backward fit loop, attention
//! backprop, loss-gradient computation, OGDL checkpoint writing, evaluation, and
//! the ratatui live dashboard. The forward engine and execution enums live in the
//! `recipe-infer` crate; this module drives them but they never depend back on it.
use crate::dataset::{Dataset, collapse_onehot};
use crate::model::{ModelInner, RunData, Train};
use recipe_infer::{
	Activation, LayerKind, LayerParams, LayerSpec,
	Loss, Metric, SCRATCH_CONSTS, Scaler, Scratch, concat_layer,
	infer_scored, load_ogdl, load_ogdl_str, metric_gpu_into,
	pinned_vocab, plan_layer_params, upload, zscore_apply,
};
use gpu_core::kernels;
use gpu_core::memory::{GpuBuffer, Stage};
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::symbols::{self, Marker};
use ratatui::text::Span;
use ratatui::widgets::{Axis, Block, Chart, Dataset as ChartDataset, GraphType, Paragraph};
use std::io::IsTerminal as _;
use std::sync::atomic::{AtomicBool, Ordering};
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
		gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.c_neg_one, &sc.c_rope_theta, m, d, heads, s, &sc.a_dq, &sc.a_dk).expect("rope backward");
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
					kernels::gpu_dger_into(grad, &params[l].w, n, in_dim, da_below).expect("dger");
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
	/// THE fit: one path, one claim, one loop, one park. The entire init is composed
	/// into ONE `Stage` image and moved with ONE sync H2D that rides the arena
	/// claim/adopt: weights (plan image — device-randn-initialized in place, or
	/// warm-started host-side from a `.resume` file or the rerun mirror), the 12
	/// scratch constants, the 4 step scalars, x/cat/y, and z-score slots (reserved on
	/// a first fit; pushed from the host Scaler on a rerun and broadcast-applied,
	/// never re-fit). Every run drives the windowed `Ooc` engine (all-VRAM runs stream
	/// zero windows → zero loop transfers/syncs); per-epoch metrics reduce into a
	/// device ring brought home in the ONE exit D2H that rides the park op, which then
	/// replays the log lines, the checkpoint divergence semantics, and the end-of-run
	/// TUI plot. Stats cross to the host Scaler exactly ONCE, at exit. The composed
	/// image rides `claim_device_arena_with_image`, so the training H2D is part of the
	/// claim op; the exit prefix D2H rides `park`, so it is part of the park op. A
	/// device that cannot map even the walked-down minimum panics — there is no
	/// routing decision, only fit.
	pub(crate) fn fit(&self, data: &Dataset, cfg: &Train, resume: Option<&str>, net: Option<std::sync::Arc<Vec<crate::wire::Conn>>>) {
		let hip_snap = cfg.metrics.contains(&Metric::Hip).then(gpu_core::callspy::snapshot);
		let led_snap = hip_snap.map(|_| gpu_core::memory::xfer_calls());
		let start = std::time::Instant::now();
		let classify = self.loss.is_classification();
		let rerun = !self.params.borrow().is_empty();
		let checkpoint_path = cfg.resume.as_deref().map(Train::resolve);
		let checkpointing = checkpoint_path.is_some();
		let plotting = !cfg.plot.is_empty() && std::io::stdout().is_terminal();
		let plot_ys: Vec<Metric> = cfg
			.plot
			.iter()
			.copied()
			.filter(|&m| m != Metric::Epoch && m != Metric::Time)
			.collect();
		// ── host prep: column split, vocab, x/cat matrices ──
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats = embed_first && data.text_cols.is_empty() && !data.onehot_groups.is_empty();
		let (collapsed_x, collapsed_ec, collapsed_vocab) = if embed_cats {
			let (x, ec, v) = collapse_onehot(data);
			(Some(x), ec, v)
		} else {
			(None, Vec::new(), 0)
		};
		let effective_x = collapsed_x.as_ref().unwrap_or(&data.x);
		let effective_text = if embed_cats { &collapsed_ec } else { &data.text_cols };
		let cat_cols: Vec<usize> = if embed_first {
			(0..effective_x.ncols()).filter(|c| !effective_text.contains(c)).collect()
		} else {
			Vec::new()
		};
		let c_cat = cat_cols.len();
		let xinput = if embed_first {
			effective_x.select(ndarray::Axis(1), effective_text)
		} else {
			effective_x.clone()
		};
		let n = xinput.nrows();
		let d = xinput.ncols();
		let vocab = if let Some(v) = pinned_vocab(&self.specs) {
			v
		} else if embed_first {
			if embed_cats { collapsed_vocab } else { xinput.iter().cloned().fold(0.0f64, f64::max) as usize + 1 }
		} else {
			0
		};
		let cat = (embed_first && c_cat > 0).then(|| effective_x.select(ndarray::Axis(1), &cat_cols));
		let c = cat.as_ref().map_or(0, |m| m.ncols());
		// z-score width: non-embed → whole matrix; embed+cats → the categorical branch;
		// embed-only tokens are never scaled.
		let d_sc = if embed_first { c } else { d };
		// ── plan weights (host image), derive output width / y ──
		// Warm-start blocks ride the plan's host image into the one H2D: an existing
		// `.resume` file wins; else a rerun warm-starts from the host mirror the prior
		// exit D2H left; else random init.
		let resumed = resume.map(load_ogdl).unwrap_or_default();
		let mut did_resume = !resumed.is_empty();
		let source = if did_resume {
			resumed
		} else if rerun {
			let m = self.saved_ogdl.borrow();
			load_ogdl_str(&m.as_ref().expect("rerun without host weight mirror").text)
		} else {
			Vec::new()
		};
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
		let warm = did_resume || rerun;
		let plan = match plan_layer_params(&self.specs, d, c_cat, vocab, &source, warm) {
			Ok(p) => p,
			Err(what) => {
				if did_resume && ask_overwrite(&what) {
					did_resume = false;
					plan_layer_params(&self.specs, d, c_cat, vocab, &[], false)
						.unwrap_or_else(|e| panic!("{e}"))
				} else if did_resume {
					panic!("checkpoint mismatch — user declined overwrite");
				} else {
					panic!("rerun: host weight mirror does not match this run — {what}");
				}
			}
		};
		let out_dim = plan.out_dim_last();
		let n_targets = data.n_targets.max(1);
		let expand_ce = classify && n_targets == 1 && out_dim > 1;
		let k = if expand_ce { out_dim } else { n_targets };
		assert_eq!(
			out_dim, k,
			"output layer has {out_dim} units but there are {n_targets} target column(s) — make the last .layer({n_targets})"
		);
		let (y_flat, ss_tot) = {
			let ys = data.y.as_slice().expect("train: y contiguous");
			let mut yd = ys.to_vec();
			if !classify && !rerun {
				let ymean = yd.iter().sum::<f64>() / yd.len() as f64;
				let yvar = yd.iter().map(|v| (v - ymean).powi(2)).sum::<f64>() / yd.len() as f64;
				let ystd = (yvar + 1e-8).sqrt();
				for v in yd.iter_mut() {
					*v = (*v - ymean) / ystd;
				}
				*self.yscaler.borrow_mut() = Some((ymean, ystd));
			} else if !classify && rerun {
				if let Some((ymean, ystd)) = *self.yscaler.borrow() {
					for v in yd.iter_mut() {
						*v = (*v - ymean) / ystd;
					}
				}
			}
			// R²'s denominator from the PRE-expansion column — an ss_tot over the
			// one-hot expansion differs; the pre-expansion value is the reported one.
			let total = yd.len() as f64;
			let ybar = yd.iter().sum::<f64>() / total;
			let ss_tot: f64 = yd.iter().map(|v| (v - ybar).powi(2)).sum();
			let y_flat = if expand_ce {
				let mut oh = vec![0.0f64; n * out_dim];
				for (i, &idx) in yd.iter().enumerate() {
					if idx.is_finite() {
						let cc = idx as usize;
						if cc < out_dim {
							oh[i * out_dim + cc] = 1.0;
						}
					}
				}
				oh
			} else {
				yd
			};
			(y_flat, ss_tot)
		};
		// ── compose the ONE init image: exit-region prefix, upload-only suffix ──
		let epochs = cfg.epochs.max(1);
		// Device metric ring rows: the stop metric (always — for fit_score and a ^C
		// snapshot) plus every logged/plotted device-computed metric (loss / R² /
		// accuracy) plus loss when checkpointing (its divergence test reads loss at
		// EVERY epoch), deduped. Epoch / lr / time / hip are host-free and take no
		// row. Each row is `epochs` scalars, filled on device and replayed from the
		// single exit D2H, so logging, checkpoint decisions, and the TUI plot all
		// keep the loop transfer-free.
		let stop_metric = if classify { Metric::Accuracy } else { Metric::R2 };
		let mut ring_row: Vec<Metric> = vec![stop_metric];
		for m in cfg
			.metrics
			.iter()
			.copied()
			.chain(checkpointing.then_some(Metric::Loss))
			.chain(plot_ys.iter().copied())
		{
			if matches!(m, Metric::Loss | Metric::R2 | Metric::Accuracy) && !ring_row.contains(&m) {
				ring_row.push(m);
			}
		}
		let n_rows = ring_row.len();
		// The windowed engine owns the per-layer roofline (its streamed-sweep lines).
		// The Scratch carries no timing event sets — a Scratch-timed forward is never
		// run here (every forward rides the engine), so zero sets to pre-allocate.
		let n_timed = 0usize;
		// ── host-composed z-score (item 1): mean / std / scaled features computed on
		// the host so the init runs ZERO device zscore kernels (in-place 0). The math
		// is the device path's op-for-op (population mean/var, +1e-8, sqrt,
		// (x−mean)/std). mean/std still ride the exit-region prefix so the exit D2H
		// hands them to the Scaler unchanged; the scaled matrix rides the upload-only
		// suffix. The scale source is the categorical branch (embed) or the whole
		// matrix (non-embed); embed token ids are never scaled. ──
		let (mean_host, std_host, scaled_host) = if d_sc == 0 {
			(Vec::new(), Vec::new(), Vec::new())
		} else {
			let src = if embed_first { cat.as_ref().expect("cat matrix") } else { &xinput };
			let src_std = src.as_standard_layout();
			let sl = src_std.as_slice().expect("scale src contiguous");
			if rerun {
				let sc_host = self.scaler.borrow();
				let s = sc_host.as_ref().expect("rerun without scaler");
				assert_eq!(s.mean.len(), d_sc, "rerun: feature count changed");
				let scaled = recipe_infer::zscore_apply_host(sl, n, d_sc, &s.mean, &s.std);
				(s.mean.clone(), s.std.clone(), scaled)
			} else {
				recipe_infer::zscore_fit_host(sl, n, d_sc)
			}
		};
		let mut stage = Stage::new();
		let w_off = stage.push(plan.host());
		// mean/std ride the prefix as REAL bytes (first fit and rerun alike): the
		// exit D2H brings them to the host Scaler exactly as before.
		let (mean_off, std_off) = if d_sc == 0 {
			(0, 0)
		} else {
			(stage.push(&mean_host), stage.push(&std_host))
		};
		let ring_off = stage.reserve(n_rows * epochs); // metric ring: n_rows × epochs scalars
		let end_off = stage.reserve(1); // end-score scalar (stop metric)
		// Everything above is the exit-region PREFIX (weights + stats + ring +
		// end-score), brought home in ONE contiguous D2H at exit. Below is
		// upload-only (consts/scalars/scaled-x/tokens/y), never downloaded.
		let w_len = plan.host().len();
		let prefix_len = stage.len_floats();
		let consts_off = stage.push(&SCRATCH_CONSTS);
		let sc_off = stage.push(&[-self.lr, 1.0 / n as f64, 2.0 / n as f64, 0.0]);
		// The standardized features (real host bytes). Non-embed models feed this as
		// xbuf directly; embed models feed it as the categorical side-input x_cat.
		let scaled_off = if d_sc > 0 { stage.push(&scaled_host) } else { 0 };
		// Raw token ids for an embed model's xbuf (never scaled). Non-embed models
		// need no raw matrix on device — the scaled features above are xbuf.
		let x_off = if embed_first {
			let x_std = xinput.as_standard_layout();
			stage.push(x_std.as_slice().expect("xinput contiguous"))
		} else {
			0
		};
		let y_off = stage.push(&y_flat);
		// ── claim the arena WITH the image (the training H2D rides the claim/adopt op,
		// not a standalone upload). adopt-first: re-arm the prior run's parked slab
		// (rewind + re-zero + image H2D + one drain); else claim fresh, walking the ask
		// down 1/16 per refusal. `footprint` is only the adopt size ask — there is no
		// fit/no-fit gate: the windowed engine sizes itself to whatever the claim
		// mapped. The image lives at the arena front, so `base` is a view spanning it
		// and `base.view(off,len)` addresses any block; scratch bump-carves after it. ──
		let image = stage.into_host();
		let image_floats = image.len();
		// The adopt ask = image + the engine's static carve floor (residents +
		// 1-sample windows) — NOT the run's working set: the engine sizes windows
		// to whatever arena remains and waterfalls the overflow, so any slab above
		// the floor is adoptable. Asking for the full working set would free a
		// perfectly good slab (its bytes become pool slack the fresh claim's
		// counters can't see) and shrink every later run; asking image-only lets a
		// contention-shrunken slab through and the residents overflow it mid-build.
		let cc_pre = recipe_infer::concat_layer_dims(&plan.dims());
		let need = image_floats * 8
			+ crate::ooc::Ooc::min_bytes(&plan.dims(), n, cc_pre.map(|(_, a, c)| (a, c)));
		let slab = gpu_core::memory::adopt_run_backing_with_image(need, &image)
			.or_else(|| gpu_core::memory::claim_device_arena_with_image(&image))
			.unwrap_or_else(|| {
				// Nothing parked AND the walk-down claim failed even at 1 MB: the device
				// cannot map the run at all — a broken device, not a routing event.
				panic!(
					"arena: claim failed — image {} claimable {} free {}",
					crate::data::human_bytes(need),
					crate::data::human_bytes(gpu_core::memory::claimable_bytes()),
					crate::data::human_bytes(gpu_core::memory::vram_free_base()),
				)
			});
		let base = slab.view(0, image_floats);
		// ── materialize weights from the image (pure carve, no init kernel/transfer) ──
		let params = plan.materialize(&base, w_off);
		let last = params.len() - 1;
		let cc_fit = concat_layer(&params);
		let consts_view = base.view(consts_off, 12);
		// THE fit scratch: the windowed engine owns every big full-batch buffer, so the
		// Scratch is the minimal resident set with its 12 constants carved from the
		// image. Carved BEFORE `Ooc::build` seals the arena tail.
		let sc = {
			let _t = gpu_core::memory::tag_scope("scratch");
			Scratch::carve(&params, n, &consts_view, n_timed)
		};
		let ss = StepScalars {
			neg_lr: base.view(sc_off, 1),
			inv_n: base.view(sc_off + 1, 1),
			two_inv_n: base.view(sc_off + 2, 1),
			zero: base.view(sc_off + 3, 1),
		};
		// ── input views (z-score already applied host-side above) ──
		let (xbuf, x_cat) = if d_sc > 0 {
			let scaled = base.view(scaled_off, n * d_sc);
			if embed_first {
				(base.view(x_off, n * d), Some(scaled))
			} else {
				(scaled, None)
			}
		} else {
			// embed-only tokens: no scaling; empty scaler so eval skips it.
			if !rerun {
				*self.scaler.borrow_mut() = Some(Scaler { mean: vec![], std: vec![] });
			}
			(base.view(x_off, xinput.len()), None)
		};
		let ybuf = base.view(y_off, n * k);
		// ── summary + roofline header (stderr on a plain first fit, dashboard header
		// when plotting, silent on a rerun) ──
		let summary = if cfg.metrics.is_empty() {
			String::new()
		} else {
			let neurons: usize = params.iter().map(|p| p.out_dim).sum();
			let out = params[last].out_dim;
			let row = |x: usize, l1: &str, y: usize, l2: &str| format!("    {x:>5}  {l1:<11}{y:>5}  {l2}");
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
		// ── the windowed engine: carves its residents/windows/homes from the arena
		// remainder and seals the tail (must run AFTER the Scratch carve above). An
		// all-VRAM run streams zero windows — every op reads full-batch residents, so
		// the loop moves nothing and syncs nothing. ──
		let mut ooc = {
			let _t = gpu_core::memory::tag_scope("waterfall");
			let o = crate::ooc::Ooc::build(&params, n, cc_fit.map(|(_, a, c)| (a, c)), net.clone());
			o.report();
			o
		};
		// ── loop: zero device allocations (AllocGuard); saturation watchdog live ──
		let _guard = gpu_core::memory::AllocGuard::freeze();
		gpu_core::hw::arm_saturation_crash();
		INTERRUPTED.store(false, Ordering::SeqCst);
		unsafe {
			libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
		}
		gpu_core::callspy::mark_loop_start();
		let hip_init = hip_snap.map(|_| gpu_core::callspy::snapshot());
		let led_init = hip_snap.map(|_| gpu_core::memory::xfer_calls());
		let mut fit_score = f64::NAN;
		// Per-recorded-epoch host records replayed AFTER the loop (batch logging
		// keeps the loop transfer-free): epoch, wall-clock, and whether it was a
		// scheduled log epoch. Checkpoint and plot runs record EVERY epoch (the
		// divergence test and the plot series replay per-epoch from the ring);
		// otherwise only log epochs. The metric values themselves go into the device
		// ring — raw kernel outputs (ss_res / accuracy / loss reduction), transformed
		// host-side at replay. Per-layer roofline is the engine's own streamed-sweep
		// lines, not a Scratch event replay.
		let mut epoch_meta: Vec<(usize, f64, bool)> = Vec::new();
		let ring_every = checkpointing || plotting;
		// Host-side (sign, div) rescale for each ring row, filled once in the loop;
		// constant across epochs (depends only on loss / metric / n / ss_tot).
		let mut ring_scale: Vec<(f64, f64)> = vec![(1.0, 1.0); n_rows];
		for e in 0..cfg.epochs {
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			let log_now = cfg.log_every > 0
				&& !cfg.metrics.is_empty()
				&& (e % cfg.log_every == 0 || e + 1 == cfg.epochs);
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit);
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			ooc.backward(&params, &xbuf, &ybuf, &sc, &ss, self.loss, cc_fit);
			if log_now || ring_every {
				ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit);
				if INTERRUPTED.load(Ordering::SeqCst) {
					break;
				}
				let out = &sc.acts[last];
				// Each ring row's raw device scalar (ss_res / accuracy / loss reduction)
				// reduces DIRECTLY into row mi's slot for epoch e — no D2D, no
				// device→host traffic; the whole ring rides home in the exit D2H.
				for (mi, &m) in ring_row.iter().enumerate() {
					let slot = base.view(ring_off + mi * epochs + e, 1);
					ring_scale[mi] = metric_gpu_into(self.loss, m, out, &ybuf, &sc, n, k, ss_tot, &slot);
				}
				epoch_meta.push((e, start.elapsed().as_secs_f64(), log_now));
			}
		}
		let was_interrupted = INTERRUPTED.load(Ordering::SeqCst);
		// End score (item 3): the loop's FINAL scoring pass — one more forward into
		// the reserved end-score slot on device (no D2H). Executed BEFORE the
		// loop-boundary snapshot so its launches/GEMMs count as loop work, not exit.
		if !was_interrupted {
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit);
			let end_slot = base.view(end_off, 1);
			if classify {
				if k == 1 {
					kernels::gpu_accuracy_into(&sc.acts[last], &ybuf, n, &end_slot).expect("accuracy");
				} else {
					kernels::gpu_argmax_accuracy_into(&sc.acts[last], &ybuf, n, k, &end_slot).expect("argmax accuracy");
				}
			} else {
				kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, n * k, &end_slot).expect("ss_res");
			}
		}
		gpu_core::hw::disarm_saturation_crash();
		gpu_core::callspy::mark_loop_end();
		let hip_loop = hip_snap.map(|_| gpu_core::callspy::snapshot());
		let led_loop = hip_snap.map(|_| gpu_core::memory::xfer_calls());
		drop(_guard);
		unsafe {
			libc::signal(libc::SIGINT, libc::SIG_DFL);
		}
		// ── exit: bring the exit-region prefix home INSIDE the park op (`park`) — the
		// prefix D2H enqueue rides the park, then ONE explicit device_synchronize is
		// the exit's single blocking drain, then the slab is PARKED (not freed) for
		// later eval/save. This is the D2H fold: the exit does no standalone transfer.
		// The engine drops first (its buffers are non-owning arena views; an all-VRAM
		// run's write-behind barrier drains zero lanes) — it never touches the arena,
		// whose park/free the caller owns. ──
		drop(ooc);
		let src = base.as_ptr_offset(0);
		let host = self.park(slab, src, prefix_len, sc);
		// stats → host Scaler (the one crossing to RAM, per the invariant). A rerun
		// pushed the SAME stats in and never re-fit, so nothing crosses back.
		if d_sc > 0 && !rerun {
			*self.scaler.borrow_mut() = Some(Scaler {
				mean: host[mean_off..mean_off + d_sc].to_vec(),
				std: host[std_off..std_off + d_sc].to_vec(),
			});
		}
		// A ring device metric → its reported value, read from its ring row and
		// rescaled host-side (the raw kernel output is ss_res / accuracy / a loss
		// reduction). Host-free metrics never reach here.
		let val_of = |m: Metric, e: usize| -> f64 {
			let Some(mi) = ring_row.iter().position(|&d| d == m) else {
				return f64::NAN;
			};
			let raw = host[ring_off + mi * epochs + e];
			match m {
				Metric::R2 => 1.0 - raw / ss_tot,
				Metric::Accuracy => raw,
				Metric::Loss => {
					let (sign, div) = ring_scale[mi];
					sign * raw / div
				}
				_ => f64::NAN,
			}
		};
		let key = self.loss.score_key();
		// Deferred per-epoch replay in epoch order: the scheduled log lines, and —
		// when checkpointing — the divergence semantics reproduced from the ring rows
		// (first loss rise → save once under the best-score guard, keyed with THAT
		// epoch's score; NaN loss → stop). The bytes written are the exit image's
		// weights — the run's only host copy, so a divergence save carries the final
		// weights (the per-epoch weights never come home under the one-D2H exit
		// contract).
		let mut loss_prev = f64::INFINITY;
		let mut ckpt_saved = false;
		for (e, elapsed, was_log) in &epoch_meta {
			let score = val_of(stop_metric, *e);
			if score.is_finite() {
				fit_score = score;
			}
			let mut checkpointed = false;
			if checkpointing {
				let loss = val_of(Metric::Loss, *e);
				if loss.is_nan() {
					eprintln!("NaN loss at epoch {e} — stopping (weights diverged)");
					break;
				}
				if !ckpt_saved && *e > 0 && loss > loss_prev {
					ckpt_saved = true;
					let path = checkpoint_path.as_ref().expect("checkpoint path");
					if recipe_infer::saved_score(path, key).is_none_or(|best| score > best) {
						recipe_infer::write_ogdl(
							path,
							&plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, score),
						);
						checkpointed = true;
					}
				}
				if loss.is_finite() {
					loss_prev = loss;
				}
			}
			if (*was_log || checkpointed) && !plotting {
				let vals: Vec<f64> = cfg
					.metrics
					.iter()
					.map(|&m| match m {
						Metric::Epoch => *e as f64,
						Metric::Lr => self.lr,
						Metric::Time => *elapsed,
						Metric::Hip => f64::NAN,
						_ => val_of(m, *e),
					})
					.collect();
				let mut line = self.metrics_line(&cfg.metrics, &vals);
				if checkpointed {
					line.push_str("  \x1b[1;32m← checkpoint\x1b[0m");
				}
				eprintln!("{line}");
			}
		}
		// TUI plot fed ONCE from the ring (full series) — same renderer, same
		// facets; only the data source moved from live per-epoch to the exit replay.
		if plotting && !epoch_meta.is_empty() {
			let plot_rows: Vec<Vec<f64>> = epoch_meta
				.iter()
				.map(|(e, elapsed, _)| {
					let mut row = vec![*elapsed];
					for &m in &plot_ys {
						row.push(match m {
							Metric::Lr => self.lr,
							Metric::Hip => f64::NAN,
							_ => val_of(m, *e),
						});
					}
					row
				})
				.collect();
			let mut term = ratatui::init();
			let _ = term.draw(|frame| {
				self.render_dashboard(frame, &summary, &plot_rows, &plot_ys);
			});
			// One frame post-loop (the loop is transfer-free, nothing renders live) —
			// hold the alt screen until a key press so the dashboard is actually seen;
			// an instant restore made a batch-replayed .plot() run invisible.
			if std::io::stdin().is_terminal() {
				loop {
					match event::read() {
						Ok(Event::Key(_)) | Err(_) => break,
						_ => {}
					}
				}
			}
			ratatui::restore();
		}
		// end score → fit_score (authoritative; equals the last epoch's — one more forward).
		let end_val = (!was_interrupted)
			.then(|| if classify { host[end_off] } else { 1.0 - host[end_off] / ss_tot });
		if let Some(s) = end_val
			&& s.is_finite()
		{
			fit_score = s;
		}
		// weights → host OGDL mirror (byte-identical to a device dump): written
		// verbatim on save, and the source a forward-only read rebuilds from once a
		// later run frees these params' arena backing. Carries the fit's derived
		// widths so the rebuild matches shapes exactly.
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		*self.saved_ogdl.borrow_mut() = Some(crate::model::SavedWeights {
			text: plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, fit_score),
			neurons,
			d,
			c_cat,
			vocab,
		});
		// Checkpoint end-of-run save, from the mirror text (zero device transfers):
		// the ^C flush writes the exit image (the only in-memory copy) UNLESS the
		// file already holds a strictly better-scored checkpoint — an interrupted
		// (or NaN-scored) flush must never destroy a good prior run's weights.
		// A completed run keeps the best-only guard on the end score.
		if let Some(path) = checkpoint_path.as_deref() {
			let sw = self.saved_ogdl.borrow();
			let text = &sw.as_ref().expect("fit leaves a mirror").text;
			if was_interrupted {
				let better_on_disk = recipe_infer::saved_score(path, key)
					.is_some_and(|best| !(fit_score.is_finite() && fit_score > best));
				if better_on_disk {
					eprintln!("keeping {path} (better prior {key} on disk)");
				} else {
					recipe_infer::write_ogdl(path, text);
					let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
					eprintln!("saved {} ({key} {fit_score:.4})", full.display());
				}
			} else if let Some(s) = end_val
				&& s.is_finite()
				&& recipe_infer::saved_score(path, key).is_none_or(|best| s > best)
			{
				recipe_infer::write_ogdl(path, text);
				let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
				eprintln!("saved {} ({neurons} neurons, {key} {s:.4})", full.display());
			}
		}
		*self.params.borrow_mut() = params;
		self.fit_score.set(fit_score);
		// No pool_trim: the fit only bump-carves from the arena — it never grows the
		// pool, so there is no freed slack to trim and no stale high-water. A
		// pool_trim's device_synchronize would be a SECOND exit blocking sync; the exit
		// contract is exactly one (the explicit drain inside `park`).
		if let (Some(b0), Some(init), Some(lp)) = (hip_snap, hip_init, hip_loop) {
			for (phase, a, b) in [
				("init", &b0, &init),
				("loop", &init, &lp),
				("exit", &lp, &gpu_core::callspy::snapshot()),
			] {
				eprint!("── hip {phase} ──\n{}", gpu_core::callspy::report_between(a, b));
			}
		}
		if let (Some(s0), Some(i), Some(l)) = (led_snap, led_init, led_loop) {
			let ee = gpu_core::memory::xfer_calls();
			let dh = |a: (usize, usize, usize), b: (usize, usize, usize)| (b.0 - a.0, b.1 - a.1);
			for (phase, (h, dd)) in [("init", dh(s0, i)), ("loop", dh(i, l)), ("exit", dh(l, ee))] {
				eprintln!("── ledger {phase} ── H2D calls {h}  D2H calls {dd}");
			}
		}
	}
	/// The exit/park op: enqueue the prefix D2H INSIDE this call (so the exit does no
	/// standalone transfer), take ONE explicit `device_synchronize` as the exit's
	/// single blocking drain (the Scratch is all arena carves now — its drop drains
	/// nothing), fan the pinned run-pin into `host` on all cores, then park the slab
	/// (kept resident, not freed, for later eval/save). `src` is the arena front
	/// (prefix start); the run-pin grows to any prefix size, so one enqueue moves it.
	fn park(&self, slab: GpuBuffer, src: *mut std::ffi::c_void, prefix_len: usize, sc: Scratch) -> Vec<f64> {
		let mut host = vec![0.0f64; prefix_len];
		let prefix_bytes = prefix_len * 8;
		let inflight = unsafe {
			gpu_core::memory::exit_d2h_enqueue(src, prefix_bytes).expect("exit prefix d2h enqueue")
		};
		// The ONE exit blocking sync: drains the enqueued prefix D2H (and all loop work).
		gpu_core::hip::device_synchronize().expect("exit drain");
		drop(sc); // ordering: released before the pin is fanned into host, drains nothing
		inflight.finish(&mut host);
		gpu_core::memory::park_run_backing(slab);
		host
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
		// Rebuild params from the host mirror if a later run freed their arena backing.
		self.ensure_params_live();
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
