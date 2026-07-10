use crate::dataset::{Dataset, collapse_onehot};
use crate::model::{ModelInner, Train};
use anyhow::Context;
use recipe_infer::{
	LayerSpec,
	Loss, Metric, PlanMode, Pt, SCRATCH_CONSTS, Scaler, Scratch, concat_layer_s,
	load_ogdl, load_ogdl_str, metric_gpu_into_s,
	pinned_vocab, plan_layer_params, plan_layer_params_m, pt, zscore_apply_views,
};
use gpu_core::kernels;
use gpu_core::memory::{GpuBuffer, Stage};
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Style};
use ratatui::symbols::{self, Marker};
use ratatui::text::Span;
use ratatui::widgets::{Axis, Block, Chart, Dataset as ChartDataset, GraphType, Paragraph};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) static INTERRUPTED: std::sync::atomic::AtomicUsize = AtomicUsize::new(0);
extern "C" fn on_sigint(_sig: i32) {
	for _run in std::iter::once(()).take(INTERRUPTED.swap(1, Ordering::SeqCst)) {
		unsafe { libc::_exit(130) };
	}
}
#[derive(Clone, Copy)]
struct Rgb {
	r: u8,
	g: u8,
	b: u8,
}
#[derive(Clone, Copy)]
enum Logged {
	Yes,
	No,
}
enum YesNo {
	Yes,
	No,
}
enum CellFix {
	Fill,
	Corner,
	Leave,
}
enum Step {
	Go,
	Stop,
}
fn gate(flag: usize) -> Step {
	match flag {
		1 => Step::Go,
		_other => Step::Stop,
	}
}
struct YPrep {
	y_flat: Vec<f64>,
	ss_tot: f64,
}
struct EpochMeta {
	epoch: usize,
	elapsed: f64,
	logged: Logged,
}
struct Found {
	cx: u16,
	style: Style,
}
pub(crate) struct EvalInput {
	pub x: GpuBuffer,
	pub x_cat: Option<GpuBuffer>,
	pub n: usize,
}
const PALETTE: [Rgb; 12] = [
	Rgb { r: 242, g: 40, b: 60 },
	Rgb { r: 39, g: 125, b: 255 },
	Rgb { r: 0, g: 174, b: 107 },
	Rgb { r: 255, g: 194, b: 0 },
	Rgb { r: 215, g: 46, b: 130 },
	Rgb { r: 135, g: 90, b: 251 },
	Rgb { r: 255, g: 122, b: 0 },
	Rgb { r: 91, g: 192, b: 235 },
	Rgb { r: 157, g: 121, b: 188 },
	Rgb { r: 46, g: 83, b: 57 },
	Rgb { r: 3, g: 252, b: 186 },
	Rgb { r: 194, g: 1, b: 20 },
];
fn palette(i: usize) -> Rgb {
	PALETTE[i % PALETTE.len()]
}
fn symlog(y: f64) -> f64 {
	match usize::from(y.abs() <= 1.0) {
		1 => y,
		_other => y.signum() * (1.0 + y.abs().log10()),
	}
}
fn inv_symlog(v: f64) -> f64 {
	match usize::from(v.abs() <= 1.0) {
		1 => v,
		_other => v.signum() * 10f64.powf(v.abs() - 1.0),
	}
}
fn fmt_time_axis(secs: f64) -> String {
	match usize::from(secs >= 3600.0) {
		1 => format!("{:.1}h", secs / 3600.0),
		_other => match usize::from(secs >= 60.0) {
			1 => format!("{:.1}m", secs / 60.0),
			_other => format!("{secs:.0}s"),
		},
	}
}
fn fmt_time(secs: f64) -> String {
	let s = secs as u64;
	let h = s / 3600;
	let m = (s % 3600) / 60;
	let sec = s % 60;
	match usize::from(h > 0) {
		1 => format!("{h}h {m:02}m {sec:02}s"),
		_other => match usize::from(m > 0) {
			1 => format!("{m}m {sec:02}s"),
			_other => format!("{secs:.1}s"),
		},
	}
}
fn fmt_axis(v: f64) -> String {
	let a = v.abs();
	match usize::from(a >= 1000.0 || (a > 0.0 && a < 0.01)) {
		1 => format!("{v:.1e}"),
		_other => match usize::from(a >= 1.0) {
			1 => format!("{v:.1}"),
			_other => format!("{v:.3}"),
		},
	}
}
pub struct StepScalars {
	pub neg_lr: GpuBuffer,
	pub inv_n: GpuBuffer,
	pub two_inv_n: GpuBuffer,
	pub zero: GpuBuffer,
}
pub fn loss_grad_into(
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
impl ModelInner {
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
		let mut parts: Vec<String> = Vec::new();
		let mut ci = 0usize;
		for mi in 0..metrics.len().min(vals.len()) {
			let m = metrics[mi];
			let v = vals[mi];
			match usize::from(m == Metric::Hip) {
				1 => continue,
				_nothip => {
					let nan_width = match m {
						Metric::Epoch => match usize::from(v.is_nan()) {
							1 => Some(5usize),
							_other => None,
						},
						Metric::Lr => match usize::from(v.is_nan()) {
							1 => Some(7usize),
							_other => None,
						},
						Metric::Time => match usize::from(v.is_nan()) {
							1 => Some(9usize),
							_other => None,
						},
						_other => None,
					};
					let num = match nan_width {
						Some(w) => format!("{:>w$}", "N/A"),
						None => match m {
							Metric::Epoch => format!("{:>5}", v as usize),
							Metric::Lr => format!("{v:>7}"),
							Metric::Time => format!("{:>9}", fmt_time(v)),
							Metric::Loss => format!("{v:>7.4}"),
							Metric::Accuracy => format!("{v:>6.4}"),
							Metric::R2 => format!("{v:>8.4}"),
							Metric::Hip => continue,
						},
					};
					let c = palette(ci);
					ci += 1;
					parts.push(format!(
						"{} \x1b[38;2;{};{};{}m{num}\x1b[0m",
						Self::label(m),
						c.r,
						c.g,
						c.b
					));
				}
			}
		}
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
		constraints.extend(ys.iter().map(|_m| Constraint::Fill(1)));
		let areas = Layout::vertical(constraints).split(frame.area());
		frame.render_widget(Paragraph::new(summary), areas[0]);
		let xmax = rows.last().map_or(1.0, |r| r[0]).max(1.0);
		let lxmax = xmax.log10().max(1e-9);
		for j in 0..ys.len() {
			let m = ys[j];
			let lo = rows
				.iter()
				.map(|r| symlog(r[1 + j]))
				.filter(|y| y.is_finite())
				.fold(f64::INFINITY, f64::min);
			let hi = rows
				.iter()
				.map(|r| symlog(r[1 + j]))
				.filter(|y| y.is_finite())
				.fold(f64::NEG_INFINITY, f64::max);
			let pts: Vec<Pt> = rows
				.iter()
				.map(|r| pt(r[0].max(1.0).log10(), symlog(r[1 + j])))
				.collect();
			let mut ymin = 0.0;
			let mut ymax = 1.0;
			match usize::from(hi > lo) {
				1 => {
					let pad = (hi - lo) * 0.05;
					ymin = lo - pad;
					ymax = hi + pad;
				}
				_other => {
					let span = usize::from(lo.is_finite());
					ymin = match span {
						1 => lo - 1.0,
						_other => ymin,
					};
					ymax = match span {
						1 => lo + 1.0,
						_other => ymax,
					};
				}
			}
			let real_lo = match usize::from(lo.is_finite()) {
				1 => inv_symlog(lo),
				_other => 0.0,
			};
			let real_hi = match usize::from(hi.is_finite()) {
				1 => inv_symlog(hi),
				_other => 1.0,
			};
			let c = palette(j);
			let color = Color::Rgb(c.r, c.g, c.b);
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
		let Some(first) = areas.get(1).copied() else {
			return;
		};
		let last = areas[areas.len() - 1];
		let buf = frame.buffer_mut();
		let mut found: Option<Found> = None;
		'find: for x in first.left()..first.right() {
			for y in first.top()..first.bottom() {
				let Some(c) = buf.cell(Position::new(x, y)) else {
					continue;
				};
				match usize::from(c.symbol() == symbols::line::VERTICAL) {
					1 => {
						found = Some(Found { cx: x, style: c.style() });
						break 'find;
					}
					_other => continue,
				}
			}
		}
		let Some(f) = found else {
			return;
		};
		for y in first.top()..last.bottom().saturating_sub(1) {
			let Some(c) = buf.cell_mut(Position::new(f.cx, y)) else {
				continue;
			};
			let sym = c.symbol();
			let blank = sym == " " || sym.is_empty();
			let corner = sym == symbols::line::BOTTOM_LEFT && y < last.top();
			let fix = match usize::from(blank) {
				1 => CellFix::Fill,
				_other => match usize::from(corner) {
					1 => CellFix::Corner,
					_other => CellFix::Leave,
				},
			};
			match fix {
				CellFix::Fill => {
					c.set_symbol(symbols::line::VERTICAL);
					c.set_style(f.style);
				}
				CellFix::Corner => {
					c.set_symbol(symbols::line::VERTICAL_RIGHT);
				}
				CellFix::Leave => continue,
			}
		}
	}
	pub(crate) fn fit(&self, data: &Dataset, cfg: &Train, resume: Option<&str>, net: Option<std::sync::Arc<Vec<crate::wire::Conn>>>) -> anyhow::Result<()> {
		let hip_snap = match usize::from(cfg.metrics.contains(&Metric::Hip)) {
			1 => Some(gpu_core::callspy::snapshot()),
			_other => None,
		};
		let led_snap = hip_snap.map(|_snap| gpu_core::memory::xfer_calls_named());
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
		let coll = match usize::from(embed_cats) {
			1 => Some(collapse_onehot(data)),
			_other => None,
		};
		let collapsed_vocab = coll.as_ref().map_or(0, |c| c.vocab);
		let effective_x = coll.as_ref().map_or(&data.x, |c| &c.x);
		let effective_text = match &coll {
			Some(c) => &c.embed_cols,
			None => &data.text_cols,
		};
		let cat_cols: Vec<usize> = match usize::from(embed_first) {
			1 => Vec::from_iter((0..effective_x.ncols()).filter(|c| !effective_text.contains(c))),
			_other => Vec::new(),
		};
		let c_cat = cat_cols.len();
		let xinput = match usize::from(embed_first) {
			1 => effective_x.select(ndarray::Axis(1), effective_text),
			_other => effective_x.clone(),
		};
		let n = xinput.nrows();
		let d = xinput.ncols();
		let vocab = match pinned_vocab(&self.specs) {
			Some(v) => v,
			None => match usize::from(embed_first) {
				1 => match usize::from(embed_cats) {
					1 => collapsed_vocab,
					_other => xinput.iter().cloned().fold(0.0f64, f64::max) as usize + 1,
				},
				_other => 0,
			},
		};
		let cat = match usize::from(embed_first && c_cat > 0) {
			1 => Some(effective_x.select(ndarray::Axis(1), &cat_cols)),
			_other => None,
		};
		let c = cat.as_ref().map_or(0, |m| m.ncols());
		let d_sc = match usize::from(embed_first) {
			1 => c,
			_other => d,
		};
		let resumed = resume.map(load_ogdl).transpose()?.unwrap_or_default();
		let mut did_resume = usize::from(!resumed.is_empty());
		let source = match did_resume {
			1 => resumed,
			_other => match usize::from(rerun) {
				1 => {
					let m = self.saved_ogdl.borrow();
					let mirror = m
						.as_ref()
						.ok_or_else(|| anyhow::anyhow!("rerun without host weight mirror"))?;
					load_ogdl_str(&mirror.text)
						.map_err(|e| anyhow::anyhow!("rerun: parse host weight mirror: {e}"))?
				}
				_other => Vec::new(),
			},
		};
		let ask_overwrite = |what: &str| -> YesNo {
			use std::io::Write;
			eprintln!(
				"\x1b[32mresume\x1b[0m\n    \x1b[1;31mdata does not match\x1b[0m\n        {what}\n        file path={}\n        data path={}",
				resume.unwrap_or(""),
				data.source,
			);
			for _run in std::iter::once(()).take(usize::from(!std::io::stdin().is_terminal())) {
				return YesNo::No;
			}
			eprint!("overwrite checkpoint with random weights? [y/N] ");
			std::io::stderr().flush().ok();
			let mut line = String::new();
			std::io::stdin().read_line(&mut line).ok();
			match usize::from(matches!(line.trim(), "y" | "Y" | "yes" | "YES")) {
				1 => YesNo::Yes,
				_other => YesNo::No,
			}
		};
		let warm = did_resume != 0 || rerun;
		let plan = match plan_layer_params(&self.specs, d, c_cat, vocab, &source, warm) {
			Ok(p) => p,
			Err(what) => {
				let overwrite = did_resume != 0 && matches!(ask_overwrite(&what), YesNo::Yes);
				match usize::from(overwrite) {
					1 => {
						did_resume = 0;
						plan_layer_params_m(&self.specs, d, c_cat, vocab, &[], PlanMode::Fresh)
							.map_err(|e| anyhow::anyhow!(e))?
					}
					_other => match did_resume {
						1 => anyhow::bail!("checkpoint mismatch — user declined overwrite"),
						_other => anyhow::bail!(
							"rerun: host weight mirror does not match this run — {what}"
						),
					},
				}
			}
		};
		let out_dim = plan.out_dim_last();
		let n_targets = data.n_targets.max(1);
		let expand_ce = classify && n_targets == 1 && out_dim > 1;
		let k = match usize::from(expand_ce) {
			1 => out_dim,
			_other => n_targets,
		};
		assert_eq!(
			out_dim, k,
			"output layer has {out_dim} units but there are {n_targets} target column(s) — make the last .layer({n_targets})"
		);
		let yp = {
			let ys = data.y.as_slice().ok_or_else(|| anyhow::anyhow!("train: y contiguous"))?;
			let mut yd = ys.to_vec();
			match usize::from(!classify && !rerun) {
				1 => {
					let ymean = yd.iter().sum::<f64>() / yd.len() as f64;
					let yvar = yd.iter().map(|v| (v - ymean).powi(2)).sum::<f64>() / yd.len() as f64;
					let ystd = (yvar + 1e-8).sqrt();
					for v in yd.iter_mut() {
						*v = (*v - ymean) / ystd;
					}
					*self.yscaler.borrow_mut() = Some(recipe_infer::YScaler { mean: ymean, std: ystd });
				}
				_other => {
					let ys_opt = *self.yscaler.borrow();
					for _run in std::iter::once(()).take(usize::from(!classify && rerun)) {
						ys_opt.map(|ysc| {
							for v in yd.iter_mut() {
								*v = (*v - ysc.mean) / ysc.std;
							}
						});
					}
				}
			}
			let total = yd.len() as f64;
			let ybar = yd.iter().sum::<f64>() / total;
			let ss_tot: f64 = yd.iter().map(|v| (v - ybar).powi(2)).sum();
			let y_flat = match usize::from(expand_ce) {
				1 => {
					let mut oh = vec![0.0f64; n * out_dim];
					for i in 0..yd.len() {
						let idx = yd[i];
						let cc = idx as usize;
						for _run in std::iter::once(()).take(usize::from(idx.is_finite() && cc < out_dim)) {
							oh[i * out_dim + cc] = 1.0;
						}
					}
					oh
				}
				_other => yd,
			};
			YPrep { y_flat, ss_tot }
		};
		let ss_tot = yp.ss_tot;
		let epochs = cfg.epochs.max(1);
		let stop_metric = match usize::from(classify) {
			1 => Metric::Accuracy,
			_other => Metric::R2,
		};
		let mut ring_row: Vec<Metric> = vec![stop_metric];
		let ckpt_loss = match usize::from(checkpointing) {
			1 => Some(Metric::Loss),
			_other => None,
		};
		for m in cfg
			.metrics
			.iter()
			.copied()
			.chain(ckpt_loss)
			.chain(plot_ys.iter().copied())
		{
			let keep = matches!(m, Metric::Loss | Metric::R2 | Metric::Accuracy) && !ring_row.contains(&m);
			ring_row.extend(std::iter::once(m).take(usize::from(keep)));
		}
		let n_rows = ring_row.len();
		let n_timed = 0usize;
		let zf: recipe_infer::ZFit = match usize::from(d_sc == 0) {
			1 => recipe_infer::ZFit {
				mean: Vec::new(),
				std: Vec::new(),
				scaled: Vec::new(),
			},
			_other => {
				let src = match usize::from(embed_first) {
					1 => cat.as_ref().ok_or_else(|| anyhow::anyhow!("cat matrix"))?,
					_other => &xinput,
				};
				let src_std = src.as_standard_layout();
				let sl = src_std.as_slice().ok_or_else(|| anyhow::anyhow!("scale src contiguous"))?;
				match usize::from(rerun) {
					1 => {
						let sc_host = self.scaler.borrow();
						let s = sc_host.as_ref().ok_or_else(|| anyhow::anyhow!("rerun without scaler"))?;
						assert_eq!(s.mean.len(), d_sc, "rerun: feature count changed");
						let scaled = recipe_infer::zscore_apply_host(sl, n, d_sc, &s.mean, &s.std);
						recipe_infer::ZFit {
							mean: s.mean.clone(),
							std: s.std.clone(),
							scaled,
						}
					}
					_other => recipe_infer::zscore_fit_s(sl, n, d_sc),
				}
			}
		};
		let mut stage = Stage::new();
		let w_off = stage.push(plan.host());
		let mut mean_off = 0;
		let mut std_off = 0;
		for _run in std::iter::once(()).take(usize::from(d_sc != 0)) {
			mean_off = stage.push(&zf.mean);
			std_off = stage.push(&zf.std);
		}
		let ring_off = stage.reserve(n_rows * epochs);
		let end_off = stage.reserve(1);
		let w_len = plan.host().len();
		let prefix_len = stage.len_floats();
		let consts_off = stage.push(&SCRATCH_CONSTS);
		let sc_off = stage.push(&[-self.lr, 1.0 / n as f64, 2.0 / n as f64, 0.0]);
		let scaled_off = match usize::from(d_sc > 0) {
			1 => stage.push(&zf.scaled),
			_other => 0,
		};
		let x_off = match usize::from(embed_first) {
			1 => {
				let x_std = xinput.as_standard_layout();
				stage.push(x_std.as_slice().ok_or_else(|| anyhow::anyhow!("xinput contiguous"))?)
			}
			_other => 0,
		};
		let y_off = stage.push(&yp.y_flat);
		let image = stage.into_host();
		let image_floats = image.len();
		let ac_pre = match recipe_infer::concat_layer_dims_s(&plan.dims()) {
			Some(d) => Some(crate::ooc::ConcatAc { a: d.a, c: d.c }),
			None => None,
		};
		let need = image_floats * 8 + crate::ooc::Ooc::min_bytes(&plan.dims(), n, ac_pre);
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
		let concat_fit_raw = concat_layer_s(&params);
		let ac_fit = match &concat_fit_raw {
			Some(d) => Some(crate::ooc::ConcatAc { a: d.a, c: d.c }),
			None => None,
		};
		let concat_fit = match &concat_fit_raw {
			Some(d) => Some(crate::ooc::ConcatFit { pf: d.pf, a: d.a, c: d.c }),
			None => None,
		};
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
		for _run in std::iter::once(()).take(usize::from(d_sc == 0 && !rerun)) {
			*self.scaler.borrow_mut() = Some(Scaler { mean: vec![], std: vec![] });
		}
		let x_cat = match usize::from(d_sc > 0 && embed_first) {
			1 => Some(base.view(scaled_off, n * d_sc)),
			_other => None,
		};
		let xbuf = match usize::from(d_sc > 0) {
			1 => match usize::from(embed_first) {
				1 => base.view(x_off, n * d),
				_other => base.view(scaled_off, n * d_sc),
			},
			_other => base.view(x_off, xinput.len()),
		};
		let ybuf = base.view(y_off, n * k);
		let summary = match usize::from(cfg.metrics.is_empty()) {
			1 => String::new(),
			_other => {
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
			}
		};
		for _run in std::iter::once(()).take(usize::from(!plotting && !rerun)) {
			for _run2 in std::iter::once(()).take(did_resume) {
				resume.map(|path| {
					let full = std::fs::canonicalize(path).unwrap_or_else(|_err| path.into());
					eprintln!("resumed: {}", full.display());
				});
			}
			for _run3 in std::iter::once(()).take(usize::from(!summary.is_empty())) {
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
			let o = crate::ooc::Ooc::build(&params, n, ac_fit, net.clone())?;
			o.report();
			o
		};
		let _guard = gpu_core::memory::AllocGuard::freeze();
		gpu_core::hw::arm_saturation_crash();
		INTERRUPTED.store(0, Ordering::SeqCst);
		unsafe {
			libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
		}
		gpu_core::callspy::mark_loop_start();
		let hip_init = hip_snap.map(|_snap| gpu_core::callspy::snapshot());
		let led_init = hip_snap.map(|_snap| gpu_core::memory::xfer_calls_named());
		let mut fit_score = f64::NAN;
		let mut epoch_meta: Vec<EpochMeta> = Vec::new();
		let ring_every = checkpointing || plotting;
		let mut ring_scale: Vec<recipe_infer::LossScale> =
			vec![recipe_infer::LossScale { sign: 1.0, div: 1.0 }; n_rows];
		for e in 0..cfg.epochs {
			let Step::Go = gate(1_usize.saturating_sub(INTERRUPTED.load(Ordering::SeqCst))) else {
				break;
			};
			let log_now = cfg.log_every > 0
				&& !cfg.metrics.is_empty()
				&& (e % cfg.log_every == 0 || e + 1 == cfg.epochs);
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
			let Step::Go = gate(1_usize.saturating_sub(INTERRUPTED.load(Ordering::SeqCst))) else {
				break;
			};
			ooc.backward(&params, &xbuf, &ybuf, &sc, &ss, self.loss, concat_fit)?;
			let Step::Go = gate(usize::from(log_now || ring_every)) else {
				continue;
			};
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
			let Step::Go = gate(1_usize.saturating_sub(INTERRUPTED.load(Ordering::SeqCst))) else {
				break;
			};
			let out = &sc.acts[last];
			for mi in 0..ring_row.len() {
				let m = ring_row[mi];
				let slot = base.view(ring_off + mi * epochs + e, 1);
				ring_scale[mi] = metric_gpu_into_s(self.loss, m, out, &ybuf, &sc, n, k, ss_tot, &slot)?;
			}
			epoch_meta.push(EpochMeta {
				epoch: e,
				elapsed: start.elapsed().as_secs_f64(),
				logged: match usize::from(log_now) {
					1 => Logged::Yes,
					_other => Logged::No,
				},
			});
		}
		let was_interrupted = INTERRUPTED.load(Ordering::SeqCst) != 0;
		for _run in std::iter::once(()).take(usize::from(!was_interrupted)) {
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
			let end_slot = base.view(end_off, 1);
			match usize::from(classify) {
				1 => match usize::from(k == 1) {
					1 => kernels::gpu_accuracy_into(&sc.acts[last], &ybuf, n, &end_slot).context("accuracy")?,
					_other => kernels::gpu_argmax_accuracy_into(&sc.acts[last], &ybuf, n, k, &end_slot).context("argmax accuracy")?,
				},
				_other => kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, n * k, &end_slot).context("ss_res")?,
			}
		}
		gpu_core::hw::disarm_saturation_crash();
		gpu_core::callspy::mark_loop_end();
		let hip_loop = hip_snap.map(|_snap| gpu_core::callspy::snapshot());
		let led_loop = hip_snap.map(|_snap| gpu_core::memory::xfer_calls_named());
		drop(_guard);
		unsafe {
			libc::signal(libc::SIGINT, libc::SIG_DFL);
		}
		drop(ooc);
		let src = base.as_ptr_offset(0);
		let host = self.park(slab, src, prefix_len, sc)?;
		for _run in std::iter::once(()).take(usize::from(d_sc > 0 && !rerun)) {
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
					let sc = ring_scale[mi];
					sc.sign * raw / sc.div
				}
				_other => f64::NAN,
			}
		};
		let key = self.loss.score_key();
		let mut loss_prev = f64::INFINITY;
		let mut ckpt_saved = 0usize;
		for meta in &epoch_meta {
			let e = meta.epoch;
			let score = val_of(stop_metric, e);
			fit_score = match usize::from(score.is_finite()) {
				1 => score,
				_other => fit_score,
			};
			let mut checkpointed = 0usize;
			let nan_stop = match usize::from(checkpointing) {
				1 => {
					let loss = val_of(Metric::Loss, e);
					let stop = loss.is_nan();
					match usize::from(stop) {
						1 => eprintln!("NaN loss at epoch {e} — stopping (weights diverged)"),
						_other => {
							let carve = ckpt_saved == 0 && e > 0 && loss > loss_prev;
							for _run in std::iter::once(()).take(usize::from(carve)) {
								ckpt_saved = 1;
								let path = checkpoint_path.as_ref().ok_or_else(|| anyhow::anyhow!("checkpoint path"))?;
								let better = recipe_infer::saved_score(path, key)
									.is_none_or(|best| score > best);
								for _run2 in std::iter::once(()).take(usize::from(better)) {
									recipe_infer::write_ogdl(
										path,
										&plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, score),
									)?;
									checkpointed = 1;
								}
							}
							loss_prev = match usize::from(loss.is_finite()) {
								1 => loss,
								_other => loss_prev,
							};
						}
					}
					usize::from(stop)
				}
				_other => 0,
			};
			let Step::Go = gate(1_usize.saturating_sub(nan_stop)) else {
				break;
			};
			let logged_flag = matches!(meta.logged, Logged::Yes);
			let Step::Go = gate(usize::from((logged_flag || checkpointed != 0) && !plotting)) else {
				continue;
			};
			let vals: Vec<f64> = cfg
				.metrics
				.iter()
				.map(|&m| match m {
					Metric::Epoch => e as f64,
					Metric::Lr => self.lr,
					Metric::Time => meta.elapsed,
					Metric::Hip => f64::NAN,
					_other => val_of(m, e),
				})
				.collect();
			let mut line = self.metrics_line(&cfg.metrics, &vals);
			let suffix = match checkpointed {
				1 => "  \x1b[1;32m<- checkpoint\x1b[0m",
				_other => "",
			};
			line.push_str(suffix);
			eprintln!("{line}");
		}
		for _run in std::iter::once(()).take(usize::from(plotting && !epoch_meta.is_empty())) {
			let plot_rows: Vec<Vec<f64>> = epoch_meta
				.iter()
				.map(|meta| {
					let mut row = vec![meta.elapsed];
					for &m in &plot_ys {
						row.push(match m {
							Metric::Lr => self.lr,
							Metric::Hip => f64::NAN,
							_other => val_of(m, meta.epoch),
						});
					}
					row
				})
				.collect();
			let mut term = ratatui::init();
			let _drawn = term.draw(|frame| {
				self.render_dashboard(frame, &summary, &plot_rows, &plot_ys);
			});
			for _run2 in std::iter::once(()).take(usize::from(std::io::stdin().is_terminal())) {
				loop {
					match event::read() {
						Err(_err) => break,
						Ok(ev) => match ev {
							Event::Key(_key) => break,
							_other => continue,
						},
					}
				}
			}
			ratatui::restore();
		}
		let end_val = match usize::from(!was_interrupted) {
			1 => Some(match usize::from(classify) {
				1 => host[end_off],
				_other => 1.0 - host[end_off] / ss_tot,
			}),
			_other => None,
		};
		end_val
			.into_iter()
			.filter(|s| s.is_finite())
			.for_each(|s| fit_score = s);
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		*self.saved_ogdl.borrow_mut() = Some(crate::model::SavedWeights {
			text: plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, fit_score),
			neurons,
			d,
			c_cat,
			vocab,
		});
		checkpoint_path
			.as_deref()
			.map(|path| -> anyhow::Result<()> {
				let sw = self.saved_ogdl.borrow();
				let text = &sw.as_ref().ok_or_else(|| anyhow::anyhow!("fit leaves a mirror"))?.text;
				match usize::from(was_interrupted) {
					1 => {
						let better_on_disk = recipe_infer::saved_score(path, key)
							.is_some_and(|best| !(fit_score.is_finite() && fit_score > best));
						match usize::from(better_on_disk) {
							1 => eprintln!("keeping {path} (better prior {key} on disk)"),
							_other => {
								recipe_infer::write_ogdl(path, text)?;
								let full = std::fs::canonicalize(path).unwrap_or_else(|_err| path.into());
								eprintln!("saved {} ({key} {fit_score:.4})", full.display());
							}
						}
					}
					_other => {
						end_val
							.filter(|s| s.is_finite())
							.filter(|&s| recipe_infer::saved_score(path, key).is_none_or(|best| s > best))
							.map(|s| -> anyhow::Result<()> {
								recipe_infer::write_ogdl(path, text)?;
								let full = std::fs::canonicalize(path).unwrap_or_else(|_err| path.into());
								eprintln!("saved {} ({neurons} neurons, {key} {s:.4})", full.display());
								Ok(())
							})
							.transpose()?;
					}
				}
				Ok(())
			})
			.transpose()?;
		*self.params.borrow_mut() = params;
		self.fit_score.set(fit_score);
		let hip_dump = || -> Option<()> {
			let b0 = hip_snap?;
			let init = hip_init?;
			let lp = hip_loop?;
			let exit = gpu_core::callspy::snapshot();
			eprint!("-- hip init --\n{}", gpu_core::callspy::report_between(&b0, &init));
			eprint!("-- hip loop --\n{}", gpu_core::callspy::report_between(&init, &lp));
			eprint!("-- hip exit --\n{}", gpu_core::callspy::report_between(&lp, &exit));
			Some(())
		};
		hip_dump();
		let led_dump = || -> Option<()> {
			let s0 = led_snap?;
			let i = led_init?;
			let l = led_loop?;
			let ee = gpu_core::memory::xfer_calls_named();
			eprintln!("-- ledger init -- H2D calls {}  D2H calls {}", i.h2d - s0.h2d, i.d2h - s0.d2h);
			eprintln!("-- ledger loop -- H2D calls {}  D2H calls {}", l.h2d - i.h2d, l.d2h - i.d2h);
			eprintln!("-- ledger exit -- H2D calls {}  D2H calls {}", ee.h2d - l.h2d, ee.d2h - l.d2h);
			Some(())
		};
		led_dump();
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
	pub(crate) fn prep_eval_input(&self, ds: &Dataset) -> EvalInput {
		let n = ds.x.nrows();
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
		let coll = match usize::from(embed_cats) {
			1 => Some(collapse_onehot(ds)),
			_other => None,
		};
		let eff_x = coll.as_ref().map_or(&ds.x, |c| &c.x);
		let eff_text = match &coll {
			Some(c) => &c.embed_cols,
			None => &ds.text_cols,
		};
		let cat_cols: Vec<usize> = match usize::from(embed_first) {
			1 => Vec::from_iter((0..eff_x.ncols()).filter(|c| !eff_text.contains(c))),
			_other => Vec::new(),
		};
		let xinput = match usize::from(embed_first) {
			1 => eff_x.select(ndarray::Axis(1), eff_text),
			_other => eff_x.clone(),
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
		match usize::from(embed_first) {
			1 => {
				let xraw = up(&xinput);
				match usize::from(cat_cols.is_empty()) {
					1 => EvalInput {
						x: xraw,
						x_cat: None,
						n,
					},
					_other => {
						let cat = eff_x.select(ndarray::Axis(1), &cat_cols);
						let craw = up(&cat);
						let c = cat.ncols();
						EvalInput {
							x: xraw,
							x_cat: Some(apply(&craw, n, c, scaler_ref)),
							n,
						}
					}
				}
			}
			_other => {
				let xraw = up(&xinput);
				EvalInput {
					x: apply(&xraw, n, d, scaler_ref),
					x_cat: None,
					n,
				}
			}
		}
	}
}
