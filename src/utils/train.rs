//! Training half of `model`: the full-batch forward+backward fit loop, attention
//! backprop, loss-gradient computation, OGDL checkpoint writing, evaluation, and
//! the ratatui live dashboard. The forward engine and execution enums live in the
//! `recipe-infer` crate; this module drives them but they never depend back on it.

use crate::dataset::{Dataset, collapse_onehot};
use crate::model::{Model, Param, Train};
use recipe_infer::{
	Activation, ELU_ALPHA, FOCAL_ALPHA, FOCAL_GAMMA, LEAKY_ALPHA, LayerKind, LayerParams, LayerSpec,
	Loss, Metric, Saved, Scaler, Scratch, build_layer_params, concat_layer, download_scalar,
	download_vec, forward_into, load_ogdl, metric_gpu, metric_gpu_into, nan_impute_and_apply,
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
	INTERRUPTED.store(true, Ordering::SeqCst);
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

impl Model {

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
	) {
		let inv = 1.0 / n as f64;
		match loss {
			Loss::Mse => kernels::gpu_sub_scale_into(out, y, da, total, 2.0 * inv),
			Loss::Mae => {
				kernels::gpu_sub_scale_into(out, y, da, total, 1.0);
				kernels::gpu_sign_into(da, da, total);
				kernels::gpu_scale_inplace(da, inv, total);
			}
			Loss::Huber => {
				kernels::gpu_sub_scale_into(out, y, da, total, 1.0);
				kernels::gpu_clamp_into(da, da, total, -1.0, 1.0);
				kernels::gpu_scale_inplace(da, inv, total);
			}
			Loss::Ce => {
				// Categorical CE fused with softmax: dz = (softmax(logits) − y)/n,
				// w.r.t. the LOGITS. `out` are logits (linear output layer); softmax
				// couples the k outputs (they sum to 1) — unlike Bce's per-output
				// independence. With a Linear output, backward's grad = da = dz.
				let k = total / n;
				kernels::gpu_softmax_rows_into(out, da, n, k); // da = softmax(logits)
				kernels::gpu_sub_scale_into(da, y, da, total, inv); // da = (softmax − y)/n
			}
			Loss::Bce => {
				// Two-sided BCE gradient (p-y)/(p(1-p))/n — not the one-sided
				// -y/p. With a sigmoid output this chains to dz = p-y.
				kernels::gpu_bce_grad_into(out, y, da, total);
			}
			Loss::Focal => {
				// d focal / d prob, scaled 1/n. `out` is the sigmoid prob; this
				// chains through the sigmoid backward like Bce.
				gpu_core::losses::gpu_focal_grad_into(out, y, da, FOCAL_GAMMA, FOCAL_ALPHA, total);
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
		}
	}

	/// The colored, aligned metric line: `vals[i]` is the precomputed value of
	/// `metrics[i]` (already reduced on the GPU), so this only formats.
	pub(crate) fn metrics_line(&self, metrics: &[Metric], vals: &[f64]) -> String {
		let parts: Vec<String> = metrics
			.iter()
			.zip(vals)
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
	) {
		use gpu_core::linalg::gpu_bmm_into;
		let d = p.dim;
		let heads = p.heads;
		let hd = d / heads;
		let s = p.in_dim / d;
		let m = n * s;
		// out = context·Wo → dcontext (a_dctx), dWo (a_gw); update Wo.
		kernels::gpu_linear_backward_full_into(
			da,
			&sc.a_ctx,
			&p.wo,
			&sc.a_dctx,
			&sc.a_gw,
			&sc.a_dbias,
			&sc.reduce_ws,
			m,
			d,
			d,
		);
		kernels::gpu_sgd_update(&p.wo, &sc.a_gw, self.lr, d * d);
		// context = scores·V → dscores = dcontext·Vᵀ, dV = scoresᵀ·dcontext.
		for hh in 0..heads {
			gpu_bmm_into(
				&sc.a_dscores,
				&sc.a_dctx,
				&sc.a_v,
				n,
				s,
				s,
				hd,
				d,
				d,
				s,
				s * d,
				s * d,
				s * s,
				hh * hd,
				hh * hd,
				hh * n * s * s,
				false,
				true,
			);
			gpu_bmm_into(
				&sc.a_dv,
				&sc.a_scores,
				&sc.a_dctx,
				n,
				s,
				hd,
				s,
				s,
				d,
				d,
				s * s,
				s * d,
				s * d,
				hh * n * s * s,
				hh * hd,
				hh * hd,
				true,
				false,
			);
		}
		kernels::gpu_softmax_backward_into(
			&sc.a_dscores,
			&sc.a_scores,
			&sc.a_dscores,
			n * heads * s,
			s,
		);
		kernels::gpu_scale_inplace(&sc.a_dscores, 1.0 / (hd as f64).sqrt(), n * heads * s * s);
		// scores = Q·Kᵀ → dQ = dscores·K, dK = dscoresᵀ·Q.
		for hh in 0..heads {
			gpu_bmm_into(
				&sc.a_dq,
				&sc.a_dscores,
				&sc.a_k,
				n,
				s,
				hd,
				s,
				s,
				d,
				d,
				s * s,
				s * d,
				s * d,
				hh * n * s * s,
				hh * hd,
				hh * hd,
				false,
				false,
			);
			gpu_bmm_into(
				&sc.a_dk,
				&sc.a_dscores,
				&sc.a_q,
				n,
				s,
				hd,
				s,
				s,
				d,
				d,
				s * s,
				s * d,
				s * d,
				hh * n * s * s,
				hh * hd,
				hh * hd,
				true,
				false,
			);
		}
		// {Q,K,V} = H·{Wq,Wk,Wv}: accumulate dH = dH_q+dH_k+dH_v into da_below; update weights.
		kernels::gpu_linear_backward_full_into(
			&sc.a_dq,
			h,
			&p.w,
			da_below,
			&sc.a_gw,
			&sc.a_dbias,
			&sc.reduce_ws,
			m,
			d,
			d,
		);
		kernels::gpu_sgd_update(&p.w, &sc.a_gw, self.lr, d * d);
		kernels::gpu_linear_backward_full_into(
			&sc.a_dk,
			h,
			&p.wk,
			&sc.a_dctx,
			&sc.a_gw,
			&sc.a_dbias,
			&sc.reduce_ws,
			m,
			d,
			d,
		);
		kernels::gpu_sgd_update(&p.wk, &sc.a_gw, self.lr, d * d);
		kernels::gpu_add_inplace(da_below, &sc.a_dctx, m * d);
		kernels::gpu_linear_backward_full_into(
			&sc.a_dv,
			h,
			&p.wv,
			&sc.a_dctx,
			&sc.a_gw,
			&sc.a_dbias,
			&sc.reduce_ws,
			m,
			d,
			d,
		);
		kernels::gpu_sgd_update(&p.wv, &sc.a_gw, self.lr, d * d);
		kernels::gpu_add_inplace(da_below, &sc.a_dctx, m * d);
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
		);
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
				kernels::gpu_scale_inplace(&sc.embed_grad, 0.0, p.vocab * p.dim);
				kernels::gpu_scatter_add(&sc.embed_grad, x, da, n * p.in_dim, p.dim);
				kernels::gpu_sgd_update(&p.w, &sc.embed_grad, self.lr, p.vocab * p.dim);
				flip = !flip;
				continue;
			}
			if params[l].kind == LayerKind::Attn {
				let a_prev = if l == 0 { x } else { &sc.acts[l - 1] };
				self.attn_backward(&params[l], a_prev, da, da_below, n, sc);
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
					Activation::Relu => { kernels::gpu_relu_backward_into(da, &sc.acts[l], &sc.dz, m); &sc.dz }
					Activation::Sigmoid => { kernels::gpu_sigmoid_backward_into(da, &sc.acts[l], &sc.dz, m); &sc.dz }
					Activation::LeakyRelu => { kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &sc.dz, m, LEAKY_ALPHA); &sc.dz }
					Activation::PRelu => {
						let a = download_scalar(&p.palpha);
						kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &sc.dz, m, a);
						kernels::gpu_relu_into(&sc.preact[l], &sc.prelu_t0, m);
						kernels::gpu_copy_into(&sc.preact[l], &sc.prelu_t1, m);
						kernels::gpu_sub_inplace(&sc.prelu_t1, &sc.prelu_t0, m);
						kernels::gpu_mul_inplace(&sc.prelu_t1, da, m);
						kernels::gpu_reduce_sum_cols_into(&sc.prelu_t1, &sc.prelu_scalar, &sc.reduce_ws, m, 1);
						kernels::gpu_sgd_update(&p.palpha, &sc.prelu_scalar, self.lr, 1);
						&sc.dz
					}
					Activation::Tanh => { kernels::gpu_tanh_backward_into(da, &sc.acts[l], &sc.dz, m); &sc.dz }
					Activation::Elu => { gpu_core::k_gapact::gpu_elu_backward_into(da, &sc.preact[l], &sc.dz, m, ELU_ALPHA); &sc.dz }
					Activation::Selu => { gpu_core::k_gapact::gpu_selu_backward_into(da, &sc.preact[l], &sc.dz, m); &sc.dz }
					Activation::Silu => { kernels::gpu_silu_backward_into(da, &sc.preact[l], &sc.dz, m); &sc.dz }
					Activation::Gelu => { kernels::gpu_gelu_backward_into(da, &sc.preact[l], &sc.dz, m); &sc.dz }
					Activation::Linear => da,
				};
				let a_prev = if l == 0 { x } else { &sc.acts[l - 1] };
				kernels::gpu_conv1d_backward_filter_into(
					grad, a_prev, &sc.dw, &sc.conv_temp, &sc.reduce_ws,
					n, cin, lin, cout, k, stride, sc.conv_wg,
				);
				kernels::gpu_scale_inplace(&sc.db, 0.0, cout);
				kernels::gpu_conv1d_backward_bias_into(grad, &sc.db, n, cout, lout);
				if l > 0 {
					kernels::gpu_conv1d_backward_data_into(grad, &p.w, da_below, n, cin, lin, cout, k, stride);
				}
				kernels::gpu_sgd_update(&p.w, &sc.dw, self.lr, cout * cin * k);
				kernels::gpu_sgd_update(&p.b, &sc.db, self.lr, cout);
				flip = !flip;
				continue;
			}
			let grad = match params[l].act {
				Activation::Relu => {
					kernels::gpu_relu_backward_into(da, &sc.acts[l], &sc.dz, m);
					&sc.dz
				}
				Activation::Sigmoid => {
					kernels::gpu_sigmoid_backward_into(da, &sc.acts[l], &sc.dz, m);
					&sc.dz
				}
				Activation::LeakyRelu => {
					kernels::gpu_leaky_relu_backward_into(
						da,
						&sc.acts[l],
						&sc.dz,
						m,
						LEAKY_ALPHA,
					);
					&sc.dz
				}
				Activation::PRelu => {
					// dx uses the current slope; then update the slope:
					// dα = Σ grad·min(z,0), with min(z,0) = z − relu(z).
					let a = download_scalar(&params[l].palpha);
					kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &sc.dz, m, a);
					kernels::gpu_relu_into(&sc.preact[l], &sc.prelu_t0, m);
					kernels::gpu_copy_into(&sc.preact[l], &sc.prelu_t1, m);
					kernels::gpu_sub_inplace(&sc.prelu_t1, &sc.prelu_t0, m);
					kernels::gpu_mul_inplace(&sc.prelu_t1, da, m);
					kernels::gpu_reduce_sum_cols_into(
						&sc.prelu_t1,
						&sc.prelu_scalar,
						&sc.reduce_ws,
						m,
						1,
					);
					kernels::gpu_sgd_update(
						&params[l].palpha,
						&sc.prelu_scalar,
						self.lr,
						1,
					);
					&sc.dz
				}
				Activation::Tanh => {
					kernels::gpu_tanh_backward_into(da, &sc.acts[l], &sc.dz, m);
					&sc.dz
				}
				// Elu/Selu/Silu/Gelu backward read the saved pre-activation, not the output.
				Activation::Elu => {
					gpu_core::k_gapact::gpu_elu_backward_into(
						da,
						&sc.preact[l],
						&sc.dz,
						m,
						ELU_ALPHA,
					);
					&sc.dz
				}
				Activation::Selu => {
					gpu_core::k_gapact::gpu_selu_backward_into(
						da,
						&sc.preact[l],
						&sc.dz,
						m,
					);
					&sc.dz
				}
				Activation::Silu => {
					kernels::gpu_silu_backward_into(da, &sc.preact[l], &sc.dz, m);
					&sc.dz
				}
				Activation::Gelu => {
					kernels::gpu_gelu_backward_into(da, &sc.preact[l], &sc.dz, m);
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
				kernels::gpu_dgemv_into(a_prev, grad, &sc.dw, n, in_dim, true);
				kernels::gpu_reduce_sum_cols_into(grad, &sc.db, &sc.reduce_ws, n, 1);
				if l > 0 {
					kernels::gpu_dger_into(grad, &params[l].w, da_below, n, in_dim);
				}
			} else if l > 0 {
				kernels::gpu_linear_backward_full_into(
					grad,
					a_prev,
					&params[l].w,
					da_below,
					&sc.dw,
					&sc.db,
					&sc.reduce_ws,
					n,
					out_dim,
					in_dim,
				);
			} else {
				kernels::gpu_linear_backward_weights_only_into(
					grad,
					a_prev,
					&sc.dw,
					&sc.db,
					&sc.reduce_ws,
					n,
					out_dim,
					in_dim,
				);
			}
			kernels::gpu_sgd_update(&params[l].w, &sc.dw, self.lr, in_dim * out_dim);
			kernels::gpu_sgd_update(&params[l].b, &sc.db, self.lr, out_dim);
			// da_below here is [n×(A+C)]; the attn layer below only wants the A
			// attention columns — compact them to the front so the next iteration
			// reads a contiguous [n×A] grad. (The trailing C cat-grads are inputs.)
			if let Some((pf, a, c)) = cc
				&& l == pf
			{
				kernels::gpu_slice_lead_into(da_below, &sc.concat_dgrad, n, a + c, a);
				kernels::gpu_copy_into(&sc.concat_dgrad, da_below, n * a);
			}
			flip = !flip;
		}
	}

	pub(crate) fn fit(&self, data: &Dataset, cfg: &Train, resume: Option<&str>) {
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
			let xbuf = nan_impute_and_apply(&xinput, n, d, sc);
			(xbuf, None)
		} else {
			(zscore_fit(&xraw, n, d, &self.scaler), None)
		};
		let ybuf = {
			let ys = data.y.as_slice().expect("train: y contiguous");
			let mut ydata = ys.to_vec();
			let has_nan = ydata.iter().any(|v| v.is_nan());
			if has_nan {
				let ymean = ydata.iter().filter(|v| !v.is_nan()).sum::<f64>()
					/ ydata.iter().filter(|v| !v.is_nan()).count().max(1) as f64;
				for v in ydata.iter_mut() {
					if v.is_nan() { *v = ymean; }
				}
			}
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
			GpuBuffer::upload(&ydata).expect("upload y")
		};

		// Resumed weights (per-neuron, in save order) or empty for random init.
		let mut resumed = resume.map(load_ogdl).unwrap_or_default();
		// NaNs in the OGDL are dead cells — training never writes them, so the
		// only way they appear is a hand-edited file. Randomize just those cells
		// (He-scaled per neuron), report the fraction, and keep training.
		if !resumed.is_empty() {
			use rand::{Rng as _, SeedableRng as _};
			use rand_distr::StandardNormal;
			let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0xB1A5);
			let total: usize = resumed.iter().map(Saved::len).sum();
			let mut nans = 0usize;
			// Only dense weights/biases get NaN-cell randomization: training never
			// writes NaN, so they only come from a hand-edited file. A NaN in an
			// embed/attn block is a real error and is left to surface downstream.
			for s in resumed.iter_mut() {
				if let Saved::Dense { w, b, .. } = s {
					let scale = (2.0 / w.len().max(1) as f64).sqrt();
					for v in w.iter_mut() {
						if v.is_nan() {
							*v = rng.sample::<f64, _>(StandardNormal) * scale;
							nans += 1;
						}
					}
					if b.is_nan() {
						*b = rng.sample::<f64, _>(StandardNormal) * scale;
						nans += 1;
					}
				}
			}
			if nans > 0 {
				let pct = 100.0 * nans as f64 / total as f64;
				eprintln!(
					"\x1b[32mresume\x1b[0m\n    \x1b[1;31mNaN\x1b[0m\n        path: {}\n        {nans}/{total} weights+biases ({pct:.1}%) were NaN\n        randomized those, continuing",
					resume.unwrap_or("")
				);
			}
		}
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
		let params = match build_layer_params(&self.specs, d, c_cat, vocab, &resumed, did_resume) {
			Ok(p) => p,
			Err(what) => {
				if ask_overwrite(&what) {
					did_resume = false;
					build_layer_params(&self.specs, d, c_cat, vocab, &resumed, false).unwrap_or_else(|e| panic!("{e}"))
				} else {
					panic!("checkpoint mismatch — user declined overwrite");
				}
			}
		};
		let last = params.len() - 1;
		// Output units must equal the target count: y is flat n*k and acts[last]
		// is n*out_dim — they must align element-for-element.
		let k = data.n_targets.max(1);
		assert_eq!(
			params[last].out_dim, k,
			"output layer has {} units but there are {k} target column(s) — make the last .layer({k})",
			params[last].out_dim
		);
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
		let ss_tot = {
			let total = (n * k) as f64;
			let ybar = data.y.iter().sum::<f64>() / total;
			data.y.iter().map(|v| (v - ybar).powi(2)).sum::<f64>()
		};
		// Activation + gradient buffers, allocated once and reused every epoch
		// so steady-state VRAM is flat (no per-epoch sawtooth).
		let sc = Scratch::new(&params, n, false);
		let _alloc_guard = gpu_core::memory::AllocGuard::freeze();
		INTERRUPTED.store(false, Ordering::SeqCst);
		unsafe {
			libc::signal(libc::SIGINT, on_sigint as libc::sighandler_t);
		}
		for e in 0..cfg.epochs {
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			// Forward with this epoch's weights, then backprop + SGD update.
			forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
			let log_now = cfg.log_every > 0
				&& !cfg.metrics.is_empty()
				&& (e % cfg.log_every == 0 || e + 1 == cfg.epochs);
			let stop_metric = if classify {
				Metric::Accuracy
			} else {
				Metric::R2
			};
			let want_score = checkpointing
				|| (log_now && cfg.metrics.contains(&stop_metric))
				|| (plotting && plot_ys.contains(&stop_metric));
			self.backward_step(&params, &xbuf, &ybuf, n, &sc);
			let need_metric = want_score
				|| (log_now && !cfg.metrics.is_empty())
				|| (plotting && !plot_ys.is_empty());
			if need_metric {
				forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
			}
			let out = &sc.acts[last];
			let score = if want_score {
				if classify {
					if k == 1 {
						kernels::gpu_accuracy_into(out, &ybuf, &sc.metric_scalar, n);
					} else {
						kernels::gpu_argmax_accuracy_into(
							out,
							&ybuf,
							&sc.metric_scalar,
							n,
							k,
						);
					}
					download_scalar(&sc.metric_scalar)
				} else {
					kernels::gpu_ss_res_into(out, &ybuf, &sc.metric_scalar, n * k);
					1.0 - download_scalar(&sc.metric_scalar) / ss_tot
				}
			} else {
				f64::NAN
			};
			let loss_scale = if checkpointing {
				let (sign, div) = metric_gpu_into(self.loss, Metric::Loss, out, &ybuf, &sc, n, k, ss_tot);
				sc.download_scalar_deferred();
				Some((sign, div))
			} else {
				None
			};
			let loss = if let Some((sign, div)) = loss_scale {
				sign * sc.sync_deferred_scalar() / div
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
					let parts = &[Param::W, Param::B];
					if Self::saved_score(path, key).is_none_or(|best| score > best) {
						Self::write_ogdl(
							path,
							&Self::dump_ogdl(&params, parts, key, score),
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
		drop(_alloc_guard);
		unsafe {
			libc::signal(libc::SIGINT, libc::SIG_DFL);
		}
		if plotting {
			ratatui::restore();
		}
		let end_score = checkpointing.then(|| {
			forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
			if classify {
				if k == 1 {
					kernels::gpu_accuracy_into(
						&sc.acts[last],
						&ybuf,
						&sc.metric_scalar,
						n,
					);
				} else {
					kernels::gpu_argmax_accuracy_into(
						&sc.acts[last],
						&ybuf,
						&sc.metric_scalar,
						n,
						k,
					);
				}
				download_scalar(&sc.metric_scalar)
			} else {
				kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n * k);
				1.0 - download_scalar(&sc.metric_scalar) / ss_tot
			}
		});
		*self.params.borrow_mut() = params;
		if let Some(s) = end_score {
			let path = checkpoint_path.as_ref().expect("checkpoint path");
			let parts = &[Param::W, Param::B];
			if INTERRUPTED.load(Ordering::SeqCst) {
				let key = self.loss.score_key();
				Self::write_ogdl(
					path,
					&Self::dump_ogdl(&self.params.borrow(), parts, key, s),
				);
				let full =
					std::fs::canonicalize(path).unwrap_or_else(|_| path.as_str().into());
				eprintln!("saved {} ({key} {s:.4})", full.display());
			} else {
				self.save_checkpoint(parts, path, s);
			}
		}
	}

	fn save_checkpoint(&self, parts: &[Param], path: &str, score: f64) {
		let params = self.params.borrow();
		assert!(!params.is_empty(), "save: call train() first");
		let key = self.loss.score_key();
		if !score.is_finite() || Self::saved_score(path, key).is_some_and(|best| score <= best)
		{
			return;
		}
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		Self::write_ogdl(path, &Self::dump_ogdl(&params, parts, key, score));
		let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
		eprintln!(
			"saved {} ({neurons} neurons, {key} {score:.4})",
			full.display()
		);
	}

	/// One OGDL block per layer, in layer order: `embed` (one `{id}=` row per
	/// vocab token), `attn` (`wq/wk/wv/wo` + `bq/bk/bv/bo`), or one `z{k}` block per
	/// dense neuron (`w=` row, `b=` scalar, plus `a=` for a PReLU layer's learned
	/// slope). W rows are laid out to match `load_ogdl`'s distribution. `parts`
	/// gates emission: weights only if `W` requested, biases only if `B`.
	pub(crate) fn dump_ogdl(params: &[LayerParams], parts: &[Param], key: &str, score: f64) -> String {
		let (want_w, want_b) = (parts.contains(&Param::W), parts.contains(&Param::B));
		let join = |v: &[f64]| {
			v.iter()
				.map(|x| x.to_string())
				.collect::<Vec<_>>()
				.join(" ")
		};
		let mut out = format!("{key}={score}\n");
		let mut z = 1;
		for p in params.iter() {
			match p.kind {
				LayerKind::Embed => {
					out.push_str("embed\n");
					if want_w {
						let table = download_vec(&p.w, p.vocab * p.dim);
						for id in 0..p.vocab {
							let row = &table[id * p.dim..(id + 1) * p.dim];
							out.push_str(&format!("    {id}={}\n", join(row)));
						}
					}
				}
				LayerKind::Attn => {
					out.push_str("attn\n");
					let dd = p.dim * p.dim;
					if want_w {
						for (nm, buf) in [
							("wq", &p.w),
							("wk", &p.wk),
							("wv", &p.wv),
							("wo", &p.wo),
						] {
							out.push_str(&format!(
								"    {nm}={}\n",
								join(&download_vec(buf, dd))
							));
						}
					}
					if want_b {
						// Bare attention has a single shared (zero) bias [d];
						// emit it as bq/bk/bv/bo for format completeness.
						let bias = download_vec(&p.b, p.dim);
						for nm in ["bq", "bk", "bv", "bo"] {
							out.push_str(&format!("    {nm}={}\n", join(&bias)));
						}
					}
				}
				LayerKind::Conv => {
					let lin = p.in_dim / p.conv_cin;
					let lout = (lin - p.conv_k) / p.conv_stride + 1;
					let cout = p.out_dim / lout;
					let w_count = cout * p.conv_cin * p.conv_k;
					out.push_str(&format!("conv {} {} {} {}\n", cout, p.conv_cin, p.conv_k, p.conv_stride));
					if want_w {
						let w = download_vec(&p.w, w_count);
						out.push_str(&format!("    w={}\n", join(&w)));
					}
					if want_b {
						let b = download_vec(&p.b, cout);
						out.push_str(&format!("    b={}\n", join(&b)));
					}
				}
				LayerKind::Dense => {
					let w = download_vec(&p.w, p.in_dim * p.out_dim);
					let b = download_vec(&p.b, p.out_dim);
					let slope = (p.act == Activation::PRelu)
						.then(|| download_scalar(&p.palpha));
					for j in 0..p.out_dim {
						out.push_str(&format!("z{z}\n"));
						if want_w {
							let row: Vec<f64> = (0..p.in_dim)
								.map(|i| w[i * p.out_dim + j])
								.collect();
							out.push_str(&format!("    w={}\n", join(&row)));
							if let Some(a) = slope {
								out.push_str(&format!("    a={a}\n"));
							}
						}
						if want_b {
							out.push_str(&format!("    b={}\n", b[j]));
						}
						z += 1;
					}
				}
			}
		}
		out
	}

	/// Write OGDL text, creating any missing parent dirs — saving should make the
	/// file, not fail because the directory isn't there yet.
	pub(crate) fn write_ogdl(path: &str, out: &str) {
		if let Some(parent) = std::path::Path::new(path).parent()
			&& !parent.as_os_str().is_empty()
		{
			std::fs::create_dir_all(parent)
				.unwrap_or_else(|e| panic!("save: mkdir {}: {e}", parent.display()));
		}
		std::fs::write(path, out).unwrap_or_else(|e| panic!("save: write {path}: {e}"));
	}

	pub(crate) fn saved_score(path: &str, key: &str) -> Option<f64> {
		let text = std::fs::read_to_string(path).ok()?;
		for line in text.lines() {
			if let Some((k, v)) = line.trim().split_once('=')
				&& k.trim() == key
			{
				return v.trim().parse().ok();
			}
		}
		None
	}

	pub fn eval(&self, data: &Dataset) {
		let params = self.params.borrow();
		assert!(!params.is_empty(), "eval: call train() first");
		// Mirror fit's two-branch input construction: text token-id columns pass
		// raw to embed→attn; the categorical branch is scaled with the TRAIN-set
		// scaler (same mean/std — eval must see the exact transform training saw).
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats = embed_first && data.text_cols.is_empty() && !data.onehot_groups.is_empty();
		let (collapsed_x, collapsed_embed_cols, _collapsed_vocab) = if embed_cats {
			let (x, ec, v) = collapse_onehot(data);
			(Some(x), ec, v)
		} else {
			(None, Vec::new(), 0)
		};
		let eff_x = collapsed_x.as_ref().unwrap_or(&data.x);
		let eff_text = if embed_cats { &collapsed_embed_cols } else { &data.text_cols };
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
		let (xraw, n, d) = upload(&xinput);
		let scaler = self.scaler.borrow();
		let scaler = scaler
			.as_ref()
			.expect("eval: missing scaler; call train first");
		let (xbuf, x_cat) = if embed_first {
			if cat_cols.is_empty() {
				(xraw, None)
			} else {
				let cat = eff_x.select(ndarray::Axis(1), &cat_cols);
				let (craw, _, c) = upload(&cat);
				(xraw, Some(zscore_apply(&craw, n, c, scaler)))
			}
		} else {
			(zscore_apply(&xraw, n, d, scaler), None)
		};
		let last = params.len() - 1;
		let k = params[last].out_dim;
		// Forward on GPU; accuracy reduced on GPU (no CPU metric computation).
		let sc = Scratch::new(&params, n, true);
		let acts = &sc.acts;
		forward_into(&params, &xbuf, x_cat.as_ref(), n, acts, &sc);
		// Labeled test (a split) → score it on the GPU. Unlabeled (Kaggle test.csv,
		// no target column) → forward pass only, nothing to score against.
		if data.has_target {
			let ybuf = GpuBuffer::upload(data.y.as_slice().expect("eval: y contiguous"))
				.expect("eval ybuf");
			let scalar = GpuBuffer::alloc(1).expect("eval scalar");
			if k == 1 {
				kernels::gpu_accuracy_into(&acts[last], &ybuf, &scalar, n);
			} else {
				kernels::gpu_argmax_accuracy_into(&acts[last], &ybuf, &scalar, n, k);
			}
			let acc = download_scalar(&scalar);
			let correct = (acc * n as f64).round() as usize;
			eprintln!("eval: accuracy = {acc:.4} ({correct}/{n})");
		} else {
			eprintln!("eval: {n} samples (no target column, accuracy unavailable)");
		}
	}
}
