use crate::dataset::{Dataset, collapse_onehot};
use crate::model::{ModelInner, Train};
use anyhow::Context as _;
use recipe_infer::{
	LayerSpec,
	Loss, Metric, SCRATCH_CONSTS, Scaler, Scratch, concat_layer,
	load_ogdl, load_ogdl_str, metric_gpu_into,
	pinned_vocab, plan_layer_params, zscore_apply_views,
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
pub(crate) static INTERRUPTED: AtomicBool = AtomicBool::new(false);
extern "C" fn on_sigint(_: i32) {
	if INTERRUPTED.swap(true, Ordering::SeqCst) {
		unsafe { libc::_exit(130) };
	}
}
const PALETTE: [(u8, u8, u8); 12] = [
	(242, 40, 60),
	(39, 125, 255),
	(0, 174, 107),
	(255, 194, 0),
	(215, 46, 130),
	(135, 90, 251),
	(255, 122, 0),
	(91, 192, 235),
	(157, 121, 188),
	(46, 83, 57),
	(3, 252, 186),
	(194, 1, 20),
];
fn palette(i: usize) -> (u8, u8, u8) {
	PALETTE[i % PALETTE.len()]
}
fn symlog(y: f64) -> f64 {
	if y.abs() <= 1.0 {
		y
	} else {
		y.signum() * (1.0 + y.abs().log10())
	}
}
fn inv_symlog(v: f64) -> f64 {
	if v.abs() <= 1.0 {
		v
	} else {
		v.signum() * 10f64.powf(v.abs() - 1.0)
	}
}
fn fmt_time_axis(secs: f64) -> String {
	if secs >= 3600.0 {
		format!("{:.1}h", secs / 3600.0)
	} else if secs >= 60.0 {
		format!("{:.1}m", secs / 60.0)
	} else {
		format!("{secs:.0}s")
	}
}
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
pub(crate) struct StepScalars {
	pub neg_lr: GpuBuffer,
	pub inv_n: GpuBuffer,
	pub two_inv_n: GpuBuffer,
	pub zero: GpuBuffer,
}
impl ModelInner {
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
			Loss::Mse => {
				let r = kernels::gpu_sub_scale_into(out, y, &ss.two_inv_n, total, da);
				assert!(r.is_ok(), "mse grad: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			}
			Loss::Mae => {
				let r = kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da);
				assert!(r.is_ok(), "mae sub: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let r = kernels::gpu_sign_into(da, total, da);
				assert!(r.is_ok(), "mae sign: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let r = kernels::gpu_scale_inplace(&ss.inv_n, total, da);
				assert!(r.is_ok(), "mae scale: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			}
			Loss::Huber => {
				let r = kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da);
				assert!(r.is_ok(), "huber sub: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let r = kernels::gpu_clamp_into(da, &sc.c_neg_one, &sc.c_one, total, da);
				assert!(r.is_ok(), "huber clamp: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let r = kernels::gpu_scale_inplace(&ss.inv_n, total, da);
				assert!(r.is_ok(), "huber scale: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			}
			Loss::Ce => {
				let k = total / n;
				let r = kernels::gpu_softmax_rows_into(out, n, k, da);
				assert!(r.is_ok(), "ce softmax: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let r = kernels::gpu_sub_scale_into(da, y, &ss.inv_n, total, da);
				assert!(r.is_ok(), "ce grad: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			}
			Loss::Bce => {
				let r = kernels::gpu_bce_grad_into(out, y, &ss.inv_n, total, da);
				assert!(r.is_ok(), "bce grad: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			}
			Loss::Focal => {
				let r = gpu_core::losses::gpu_focal_grad_into(out, y, &sc.c_focal_gamma, &sc.c_focal_alpha, &ss.inv_n, total, da);
				assert!(r.is_ok(), "focal grad: {}", r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			}
		}
	}
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

	pub(crate) fn metrics_line(&self, metrics: &[Metric], vals: &[f64]) -> String {
		let parts: Vec<String> = metrics
			.iter()
			.zip(vals)
			.filter(|(m, _)| **m != Metric::Hip)
			.enumerate()
			.filter_map(|(i, (&m, &v))| {
				let nan_width = match m {
					Metric::Epoch if v.is_nan() => Some(5),
					Metric::Lr if v.is_nan() => Some(7),
					Metric::Time if v.is_nan() => Some(9),
					_ => None,
				};
				let num = match (nan_width, m) {
					(Some(w), _) => format!("{:>w$}", "N/A"),
					(None, Metric::Epoch) => format!("{:>5}", v as usize),
					(None, Metric::Lr) => format!("{v:>7}"),
					(None, Metric::Time) => format!("{:>9}", fmt_time(v)),
					(None, Metric::Loss) => format!("{v:>7.4}"),
					(None, Metric::Accuracy) => format!("{v:>6.4}"),
					(None, Metric::R2) => format!("{v:>8.4}"),
					(None, Metric::Hip) => return None,
				};
				let (r, g, b) = palette(i);
				Some(format!("{} \x1b[38;2;{r};{g};{b}m{num}\x1b[0m", Self::label(m)))
			})
			.collect();
		parts.join("  ")
	}
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
		let lxmax = xmax.log10().max(1e-9);
		for (j, &m) in ys.iter().enumerate() {
			let pts: Vec<(f64, f64)> = rows
				.iter()
				.map(|r| (r[0].max(1.0).log10(), symlog(r[1 + j])))
				.collect();
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
			let real_lo = if lo.is_finite() { inv_symlog(lo) } else { 0.0 };
			let real_hi = if hi.is_finite() { inv_symlog(hi) } else { 1.0 };
			let (r, g, b) = palette(j);
			let color = Color::Rgb(r, g, b);
			let ds = ChartDataset::default()
				.marker(Marker::Braille)
				.graph_type(GraphType::Line)
				.style(Style::default().fg(color))
				.data(&pts);
			let cur = rows.last().map_or(f64::NAN, |r| r[1 + j]);
			let title = Span::styled(
				format!("{} = {}", Self::label(m), fmt_axis(cur)),
				Style::default().fg(color),
			);
			let chart = Chart::new(vec![ds])
				.block(Block::default().title(title))
				.x_axis(Axis::default().bounds([0.0, lxmax]).labels([
					String::new(),
					String::new(),
					fmt_time_axis(10f64.powf(lxmax)),
				]))
				.y_axis(Axis::default().bounds([ymin, ymax]).labels([
					format!("{:>12}", fmt_axis(real_lo)),
					format!("{:>12}", fmt_axis(real_hi)),
				]));
			frame.render_widget(chart, areas[j + 1]);
		}
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
				for y in first.top()..last.bottom().saturating_sub(1) {
					if let Some(c) = buf.cell_mut((cx, y)) {
						match c.symbol() {
							" " | "" => {
								c.set_symbol(symbols::line::VERTICAL);
								c.set_style(style);
							}
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
	pub(crate) fn fit(&self, data: &Dataset, cfg: &Train, resume: Option<&str>, net: Option<std::sync::Arc<Vec<crate::wire::Conn>>>) -> anyhow::Result<()> {
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
		let d_sc = if embed_first { c } else { d };
		let resumed = resume.map(load_ogdl).transpose()?.unwrap_or_default();
		let mut did_resume = !resumed.is_empty();
		let source = if did_resume {
			resumed
		} else if rerun {
			let m = self.saved_ogdl.borrow();
			let mirror = m
				.as_ref()
				.ok_or_else(|| anyhow::anyhow!("rerun without host weight mirror"))?;
			load_ogdl_str(&mirror.text)
				.map_err(|e| anyhow::anyhow!("rerun: parse host weight mirror: {e}"))?
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
						.map_err(|e| anyhow::anyhow!(e))?
				} else if did_resume {
					anyhow::bail!("checkpoint mismatch — user declined overwrite");
				} else {
					anyhow::bail!("rerun: host weight mirror does not match this run — {what}");
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
			let ys = data.y.as_slice().ok_or_else(|| anyhow::anyhow!("train: y contiguous"))?;
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
		let epochs = cfg.epochs.max(1);
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
		let n_timed = 0usize;
		let (mean_host, std_host, scaled_host) = if d_sc == 0 {
			(Vec::new(), Vec::new(), Vec::new())
		} else {
			let src = if embed_first { cat.as_ref().ok_or_else(|| anyhow::anyhow!("cat matrix"))? } else { &xinput };
			let src_std = src.as_standard_layout();
			let sl = src_std.as_slice().ok_or_else(|| anyhow::anyhow!("scale src contiguous"))?;
			if rerun {
				let sc_host = self.scaler.borrow();
				let s = sc_host.as_ref().ok_or_else(|| anyhow::anyhow!("rerun without scaler"))?;
				assert_eq!(s.mean.len(), d_sc, "rerun: feature count changed");
				let scaled = recipe_infer::zscore_apply_host(sl, n, d_sc, &s.mean, &s.std);
				(s.mean.clone(), s.std.clone(), scaled)
			} else {
				recipe_infer::zscore_fit_host(sl, n, d_sc)
			}
		};
		let mut stage = Stage::new();
		let w_off = stage.push(plan.host());
		let (mean_off, std_off) = if d_sc == 0 {
			(0, 0)
		} else {
			(stage.push(&mean_host), stage.push(&std_host))
		};
		let ring_off = stage.reserve(n_rows * epochs);
		let end_off = stage.reserve(1);
		let w_len = plan.host().len();
		let prefix_len = stage.len_floats();
		let consts_off = stage.push(&SCRATCH_CONSTS);
		let sc_off = stage.push(&[-self.lr, 1.0 / n as f64, 2.0 / n as f64, 0.0]);
		let scaled_off = if d_sc > 0 { stage.push(&scaled_host) } else { 0 };
		let x_off = if embed_first {
			let x_std = xinput.as_standard_layout();
			stage.push(x_std.as_slice().ok_or_else(|| anyhow::anyhow!("xinput contiguous"))?)
		} else {
			0
		};
		let y_off = stage.push(&y_flat);
		let image = stage.into_host();
		let image_floats = image.len();
		let cc_pre = recipe_infer::concat_layer_dims(&plan.dims());
		let need = image_floats * 8
			+ crate::ooc::Ooc::min_bytes(&plan.dims(), n, cc_pre.map(|(_, a, c)| (a, c)));
		let slab = gpu_core::memory::adopt_run_backing_with_image(need, &image)
			.or_else(|| gpu_core::memory::claim_device_arena_with_image(&image))
			.ok_or_else(|| {
				anyhow::anyhow!(
					"arena: claim failed — image {} claimable {} free {}",
					crate::data::human_bytes(need),
					crate::data::human_bytes(gpu_core::memory::claimable_bytes()),
					crate::data::human_bytes(gpu_core::memory::vram_free_base()),
				)
			})?;
		let base = slab.view(0, image_floats);
		let params = plan.materialize(&base, w_off);
		let last = params.len() - 1;
		let cc_fit = concat_layer(&params);
		let consts_view = base.view(consts_off, 12);
		let sc = {
			let _t = gpu_core::memory::tag_scope("scratch");
			Scratch::carve(&params, n, &consts_view, n_timed)?
		};
		let ss = StepScalars {
			neg_lr: base.view(sc_off, 1),
			inv_n: base.view(sc_off + 1, 1),
			two_inv_n: base.view(sc_off + 2, 1),
			zero: base.view(sc_off + 3, 1),
		};
		let (xbuf, x_cat) = if d_sc > 0 {
			let scaled = base.view(scaled_off, n * d_sc);
			if embed_first {
				(base.view(x_off, n * d), Some(scaled))
			} else {
				(scaled, None)
			}
		} else {
			if !rerun {
				*self.scaler.borrow_mut() = Some(Scaler { mean: vec![], std: vec![] });
			}
			(base.view(x_off, xinput.len()), None)
		};
		let ybuf = base.view(y_off, n * k);
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
		let mut ooc = {
			let _t = gpu_core::memory::tag_scope("waterfall");
			let o = crate::ooc::Ooc::build(&params, n, cc_fit.map(|(_, a, c)| (a, c)), net.clone())?;
			o.report();
			o
		};
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
		let mut epoch_meta: Vec<(usize, f64, bool)> = Vec::new();
		let ring_every = checkpointing || plotting;
		let mut ring_scale: Vec<(f64, f64)> = vec![(1.0, 1.0); n_rows];
		for e in 0..cfg.epochs {
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			let log_now = cfg.log_every > 0
				&& !cfg.metrics.is_empty()
				&& (e % cfg.log_every == 0 || e + 1 == cfg.epochs);
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit)?;
			if INTERRUPTED.load(Ordering::SeqCst) {
				break;
			}
			ooc.backward(&params, &xbuf, &ybuf, &sc, &ss, self.loss, cc_fit)?;
			if log_now || ring_every {
				ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit)?;
				if INTERRUPTED.load(Ordering::SeqCst) {
					break;
				}
				let out = &sc.acts[last];
				for (mi, &m) in ring_row.iter().enumerate() {
					let slot = base.view(ring_off + mi * epochs + e, 1);
					ring_scale[mi] = metric_gpu_into(self.loss, m, out, &ybuf, &sc, n, k, ss_tot, &slot)?;
				}
				epoch_meta.push((e, start.elapsed().as_secs_f64(), log_now));
			}
		}
		let was_interrupted = INTERRUPTED.load(Ordering::SeqCst);
		if !was_interrupted {
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, cc_fit)?;
			let end_slot = base.view(end_off, 1);
			if classify {
				if k == 1 {
					kernels::gpu_accuracy_into(&sc.acts[last], &ybuf, n, &end_slot).context("accuracy")?;
				} else {
					kernels::gpu_argmax_accuracy_into(&sc.acts[last], &ybuf, n, k, &end_slot).context("argmax accuracy")?;
				}
			} else {
				kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, n * k, &end_slot).context("ss_res")?;
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
		drop(ooc);
		let src = base.as_ptr_offset(0);
		let host = self.park(slab, src, prefix_len, sc)?;
		if d_sc > 0 && !rerun {
			*self.scaler.borrow_mut() = Some(Scaler {
				mean: host[mean_off..mean_off + d_sc].to_vec(),
				std: host[std_off..std_off + d_sc].to_vec(),
			});
		}
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
					let path = checkpoint_path.as_ref().ok_or_else(|| anyhow::anyhow!("checkpoint path"))?;
					if recipe_infer::saved_score(path, key).is_none_or(|best| score > best) {
						recipe_infer::write_ogdl(
							path,
							&plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, score),
						)?;
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
					line.push_str("  \x1b[1;32m<- checkpoint\x1b[0m");
				}
				eprintln!("{line}");
			}
		}
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
		let end_val = (!was_interrupted)
			.then(|| if classify { host[end_off] } else { 1.0 - host[end_off] / ss_tot });
		if let Some(s) = end_val
			&& s.is_finite()
		{
			fit_score = s;
		}
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		*self.saved_ogdl.borrow_mut() = Some(crate::model::SavedWeights {
			text: plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, fit_score),
			neurons,
			d,
			c_cat,
			vocab,
		});
		if let Some(path) = checkpoint_path.as_deref() {
			let sw = self.saved_ogdl.borrow();
			let text = &sw.as_ref().ok_or_else(|| anyhow::anyhow!("fit leaves a mirror"))?.text;
			if was_interrupted {
				let better_on_disk = recipe_infer::saved_score(path, key)
					.is_some_and(|best| !(fit_score.is_finite() && fit_score > best));
				if better_on_disk {
					eprintln!("keeping {path} (better prior {key} on disk)");
				} else {
					recipe_infer::write_ogdl(path, text)?;
					let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
					eprintln!("saved {} ({key} {fit_score:.4})", full.display());
				}
			} else if let Some(s) = end_val
				&& s.is_finite()
				&& recipe_infer::saved_score(path, key).is_none_or(|best| s > best)
			{
				recipe_infer::write_ogdl(path, text)?;
				let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
				eprintln!("saved {} ({neurons} neurons, {key} {s:.4})", full.display());
			}
		}
		*self.params.borrow_mut() = params;
		self.fit_score.set(fit_score);
		if let (Some(b0), Some(init), Some(lp)) = (hip_snap, hip_init, hip_loop) {
			for (phase, a, b) in [
				("init", &b0, &init),
				("loop", &init, &lp),
				("exit", &lp, &gpu_core::callspy::snapshot()),
			] {
				eprint!("-- hip {phase} --\n{}", gpu_core::callspy::report_between(a, b));
			}
		}
		if let (Some(s0), Some(i), Some(l)) = (led_snap, led_init, led_loop) {
			let ee = gpu_core::memory::xfer_calls();
			let dh = |a: (usize, usize, usize), b: (usize, usize, usize)| (b.0 - a.0, b.1 - a.1);
			for (phase, (h, dd)) in [("init", dh(s0, i)), ("loop", dh(i, l)), ("exit", dh(l, ee))] {
				eprintln!("-- ledger {phase} -- H2D calls {h}  D2H calls {dd}");
			}
		}
		Ok(())
	}
	fn park(&self, slab: GpuBuffer, src: *mut std::ffi::c_void, prefix_len: usize, sc: Scratch) -> anyhow::Result<Vec<f64>> {
		let mut host = vec![0.0f64; prefix_len];
		let prefix_bytes = prefix_len * 8;
		let inflight = unsafe {
			gpu_core::memory::exit_d2h_enqueue(src, prefix_bytes).context("exit prefix d2h enqueue")?
		};
		gpu_core::hip::device_synchronize().context("exit drain")?;
		drop(sc);
		inflight.finish(&mut host);
		gpu_core::memory::park_run_backing(slab);
		Ok(host)
	}
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
		let scaler_opt = scaler.as_ref();
		assert!(scaler_opt.is_some(), "eval: missing scaler; train first");
		let Some(scaler_ref) = scaler_opt else { loop {} };
		let up = |m: &ndarray::Array2<f64>| -> GpuBuffer {
			let s = m.as_standard_layout();
			let rs = s.as_slice();
			assert!(rs.is_some(), "eval upload: non-contiguous");
			let Some(sl) = rs else { loop {} };
			let rb = GpuBuffer::alloc(sl.len());
			assert!(rb.is_ok(), "eval upload: {}", rb.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok(b) = rb else { loop {} };
			let rl = b.load(sl);
			assert!(rl.is_ok(), "eval upload: {}", rl.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			b
		};
		let apply = |xraw: &GpuBuffer, rows: usize, cols: usize, sc: &Scaler| -> GpuBuffer {
			assert_eq!(sc.mean.len(), cols, "eval: feature count changed");
			assert_eq!(sc.std.len(), cols, "eval: feature count changed");
			let mut st = Stage::new();
			let m_off = st.push(&sc.mean);
			let s_off = st.push(&sc.std);
			let host = st.into_host();
			let ri = GpuBuffer::alloc(host.len().max(1));
			assert!(ri.is_ok(), "eval scaler stage: {}", ri.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok(img) = ri else { loop {} };
			let rl = img.load(&host);
			assert!(rl.is_ok(), "eval scaler stage: {}", rl.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let rz = zscore_apply_views(xraw, rows, cols, &img.view(m_off, cols), &img.view(s_off, cols));
			assert!(rz.is_ok(), "eval zscore: {}", rz.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok(z) = rz else { loop {} };
			z
		};
		if embed_first {
			let xraw = up(&xinput);
			if cat_cols.is_empty() {
				(xraw, None, n)
			} else {
				let cat = eff_x.select(ndarray::Axis(1), &cat_cols);
				let craw = up(&cat);
				let c = cat.ncols();
				(xraw, Some(apply(&craw, n, c, scaler_ref)), n)
			}
		} else {
			let xraw = up(&xinput);
			(apply(&xraw, n, d, scaler_ref), None, n)
		}
	}
}
#[cfg(test)]
mod backward_test_support {
	use super::*;
	use recipe_infer::{Activation, LayerKind, LayerParams};
	impl StepScalars {
		pub(crate) fn new(lr: f64, n: usize) -> StepScalars {
			let inv = 1.0 / n as f64;
			StepScalars {
				neg_lr: { let __up = &[-lr]; let __ub = GpuBuffer::alloc(__up.len()).expect("neg_lr"); __ub.load(__up).expect("neg_lr"); __ub },
				inv_n: { let __up = &[inv]; let __ub = GpuBuffer::alloc(__up.len()).expect("inv_n"); __ub.load(__up).expect("inv_n"); __ub },
				two_inv_n: { let __up = &[2.0 * inv]; let __ub = GpuBuffer::alloc(__up.len()).expect("two_inv_n"); __ub.load(__up).expect("two_inv_n"); __ub },
				zero: { let __up = &[0.0]; let __ub = GpuBuffer::alloc(__up.len()).expect("zero"); __ub.load(__up).expect("zero"); __ub },
			}
		}
	}
	impl ModelInner {
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
			gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.c_neg_one, &sc.c_rope_theta, m, d, heads, s, &sc.a_dq, &sc.a_dk).expect("rope backward");
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
	}
}
