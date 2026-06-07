use crate::dataset::Dataset;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::symbols::{self, Marker};
use ratatui::text::Span;
use ratatui::widgets::{Axis, Block, Chart, Dataset as ChartDataset, GraphType, Paragraph};
use std::cell::RefCell;
use std::io::IsTerminal as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Set by the SIGINT handler so headless (cooked-mode) Ctrl+C exits gracefully
/// — in TUI raw mode Ctrl+C arrives as a key event instead and is handled there.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigint(_: i32) {
      INTERRUPTED.store(true, Ordering::SeqCst);
}

#[derive(Clone, Copy, PartialEq)]
pub enum Activation {
      Relu,
      Sigmoid,
      Linear,
}

/// Accepts `units` (linear) or `(units, activation)` for `Model::layer`.
pub trait IntoLayer {
      fn into_layer(self) -> (usize, Activation);
}
impl IntoLayer for usize {
      fn into_layer(self) -> (usize, Activation) {
            (self, Activation::Linear)
      }
}
impl IntoLayer for (usize, Activation) {
      fn into_layer(self) -> (usize, Activation) {
            self
      }
}

#[derive(Clone, Copy)]
pub enum Loss {
      Mse,
      Mae,
      Ce,
      Huber,
}

#[allow(non_upper_case_globals)]
pub const relu: Activation = Activation::Relu;
#[allow(non_upper_case_globals)]
pub const sigmoid: Activation = Activation::Sigmoid;
#[allow(non_upper_case_globals)]
pub const linear: Activation = Activation::Linear;
#[allow(non_upper_case_globals)]
pub const mse: Loss = Loss::Mse;
#[allow(non_upper_case_globals)]
pub const mae: Loss = Loss::Mae;
#[allow(non_upper_case_globals)]
pub const ce: Loss = Loss::Ce;
#[allow(non_upper_case_globals)]
pub const huber: Loss = Loss::Huber;

/// Which parameters `save` writes — pass `w`, `b`, or both (consts in the crate
/// root, kept out of this module so they don't shadow local `w`/`b` bindings).
#[derive(Clone, Copy, PartialEq)]
pub enum Param {
      W,
      B,
}

/// How to run training: epochs, logging, plotting, resume, and save. Holds the
/// "run" config so `Model` stays pure architecture and `Data` stays pure data.
pub struct Train {
      epochs: usize,
      log_every: usize,
      metrics: Vec<Metric>,
      plot: Vec<Metric>,
      resume: Option<String>,
      // None = save never called; Some((parts, path)) = called (parts may be empty).
      save: Option<(Vec<Param>, String)>,
}

impl Train {
      pub fn new() -> Train {
            Train {
                  epochs: 1,
                  log_every: 1,
                  metrics: Vec::new(),
                  plot: Vec::new(),
                  resume: None,
                  save: None,
            }
      }

      /// Resolve a path arg: `""` → `model.ogdl` (cwd), `"*"` → next to the
      /// running binary, anything else → used verbatim.
      fn resolve(path: &str) -> String {
            let raw = if path.is_empty() {
                  "model.ogdl".to_string()
            } else if path == "*" {
                  std::env::current_exe()
                        .ok()
                        .and_then(|e| e.parent().map(|d| d.join("model.ogdl")))
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "model.ogdl".to_string())
            } else {
                  path.to_string()
            };
            expand_tilde(&raw)
      }

      pub fn epochs(mut self, n: usize) -> Train {
            self.epochs = n;
            self
      }

      pub fn log_every(mut self, every: usize) -> Train {
            self.log_every = every;
            self
      }

      pub fn log(mut self, metrics: &[Metric]) -> Train {
            self.metrics = metrics.to_vec();
            self
      }

      pub fn plot(mut self, metrics: &[Metric]) -> Train {
            self.plot = metrics.to_vec();
            self
      }

      pub fn resume(mut self, path: impl Into<String>) -> Train {
            self.resume = Some(path.into());
            self
      }

      pub fn save(mut self, parts: &[Param], path: impl Into<String>) -> Train {
            self.save = Some((parts.to_vec(), path.into()));
            self
      }

      /// Train `model` on the data's train set, save (if requested), then eval on
      /// the test set — but only if there is one (`.split` or `.test` was set).
      pub fn run(&self, model: &Model, data: &crate::dataset::Data) {
            // Resuming but never calling save = load weights, train, discard. Refuse.
            if self.resume.is_some() && self.save.is_none() {
                  eprintln!(
                        "\x1b[1;31mresume without save\x1b[0m\n\
                         \x20   you'd load weights, train, then throw them away\n\
                         \x20   add .save(&[w, b], \"*\") to write back next to your script"
                  );
                  std::process::exit(1);
            }
            let (train, test) = data.prepare();
            let resume = self.resume.as_deref().map(Self::resolve);
            model.fit(&train, self, resume.as_deref());
            // Saving is owned by fit()'s trailing stop-loss (checkpoint on the first R²
            // drop, else the final weights at the end) — do NOT save here, or a blow-up
            // would overwrite the good checkpoint.
            if let Some(test) = &test {
                  model.eval(test);
            }
      }
}

impl Default for Train {
      fn default() -> Self {
            Self::new()
      }
}

/// Per-column number colors, applied in `.log(&[...])` order (cycles past 12).
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

/// Expand a leading `~` (the shell doesn't, since the path arrives as a literal
/// string) to `$HOME`. Anything else is returned unchanged.
fn expand_tilde(path: &str) -> String {
      match std::env::var("HOME") {
            Ok(home) if path == "~" => home,
            Ok(home) => match path.strip_prefix("~/") {
                  Some(rest) => format!("{home}/{rest}"),
                  None => path.to_string(),
            },
            Err(_) => path.to_string(),
      }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Metric {
      Loss,
      Accuracy,
      Epoch,
      Lr,
      Time,
      R2,
}

#[allow(non_upper_case_globals)]
pub const Loss: Metric = Metric::Loss;
#[allow(non_upper_case_globals)]
pub const Accuracy: Metric = Metric::Accuracy;
#[allow(non_upper_case_globals)]
pub const Epoch: Metric = Metric::Epoch;
#[allow(non_upper_case_globals)]
pub const Lr: Metric = Metric::Lr;
#[allow(non_upper_case_globals)]
pub const Time: Metric = Metric::Time;
#[allow(non_upper_case_globals)]
pub const R2: Metric = Metric::R2;

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

struct LayerParams {
      w: GpuBuffer,
      b: GpuBuffer,
      in_dim: usize,
      out_dim: usize,
      act: Activation,
}

struct Scratch {
      acts: Vec<GpuBuffer>,
      da_a: GpuBuffer,
      da_b: GpuBuffer,
      dz: GpuBuffer,
      dw: GpuBuffer,
      db: GpuBuffer,
      metric_t0: GpuBuffer,
      metric_t1: GpuBuffer,
      metric_t2: GpuBuffer,
      metric_scalar: GpuBuffer,
      reduce_ws: GpuBuffer,
}

impl Scratch {
      fn new(params: &[LayerParams], n: usize) -> Scratch {
            let mut max_ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1);
            let mut max_act = 0usize;
            let mut max_wt = 0usize;
            let mut max_bias = 0usize;
            for p in params {
                  let w = kernels::gpu_reduce_sum_cols_workspace_bytes(n, p.out_dim);
                  if w > max_ws { max_ws = w; }
                  let a = n * p.out_dim;
                  if a > max_act { max_act = a; }
                  let wt = p.in_dim * p.out_dim;
                  if wt > max_wt { max_wt = wt; }
                  if p.out_dim > max_bias { max_bias = p.out_dim; }
            }
            let mut acts = Vec::with_capacity(params.len());
            for p in params {
                  acts.push(GpuBuffer::alloc(n * p.out_dim).expect("scratch acts"));
            }
            Scratch {
                  acts,
                  da_a: GpuBuffer::alloc(max_act).expect("scratch da_a"),
                  da_b: GpuBuffer::alloc(max_act).expect("scratch da_b"),
                  dz: GpuBuffer::alloc(max_act).expect("scratch dz"),
                  dw: GpuBuffer::alloc(max_wt).expect("scratch dw"),
                  db: GpuBuffer::alloc(max_bias).expect("scratch db"),
                  metric_t0: GpuBuffer::alloc(n).expect("scratch metric_t0"),
                  metric_t1: GpuBuffer::alloc(n).expect("scratch metric_t1"),
                  metric_t2: GpuBuffer::alloc(n).expect("scratch metric_t2"),
                  metric_scalar: GpuBuffer::alloc(1).expect("scratch metric_scalar"),
                  reduce_ws: GpuBuffer::alloc_bytes(max_ws).expect("scratch reduce_ws"),
            }
      }
}

pub struct Model {
      specs: Vec<(usize, Activation)>,
      loss: Loss,
      lr: f64,
      params: RefCell<Vec<LayerParams>>,
}

impl Model {
      pub fn new() -> Model {
            Model {
                  specs: Vec::new(),
                  loss: Loss::Mse,
                  lr: 0.01,
                  params: RefCell::new(Vec::new()),
            }
      }

      /// dL/dA at the output for the chosen loss, scaled by 1/n (batch mean),
      /// written in place into `da` with no allocation. `out` = predictions,
      /// `y` = targets, `total` = n*out_dim. Equals the old allocate-return
      /// `loss_grad` followed by `·(1/n)`, op-for-op.
      fn loss_grad_into(loss: Loss, out: &GpuBuffer, y: &GpuBuffer, da: &GpuBuffer, n: usize, total: usize) {
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
                        kernels::gpu_div_into(y, out, da, total);
                        kernels::gpu_scale_inplace(da, -inv, total);
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

      /// Raw numeric value of a metric this epoch (p = downloaded predictions).
      fn metric_num(&self, m: Metric, epoch: usize, p: &[f64], y: &crate::Vec1, n: usize, elapsed: f64) -> f64 {
            match m {
                  Metric::Epoch => epoch as f64,
                  Metric::Lr => self.lr,
                  Metric::Time => elapsed,
                  Metric::Accuracy => {
                        (0..n).filter(|&i| (p[i] >= 0.5) == (y[i] >= 0.5)).count() as f64 / n as f64
                  }
                  Metric::Loss => {
                        let eps = 1e-7;
                        -(0..n)
                              .map(|i| {
                                    let pi = p[i].clamp(eps, 1.0 - eps);
                                    y[i] * pi.ln() + (1.0 - y[i]) * (1.0 - pi).ln()
                              })
                              .sum::<f64>()
                              / n as f64
                  }
                  Metric::R2 => {
                        let ybar = y.iter().sum::<f64>() / n as f64;
                        let ss_tot: f64 = y.iter().map(|v| (v - ybar).powi(2)).sum();
                        let ss_res: f64 = (0..n).map(|i| (y[i] - p[i]).powi(2)).sum();
                        1.0 - ss_res / ss_tot
                  }
            }
      }

      /// One metric this epoch as a single GPU-reduced scalar, downloading only that
      /// scalar (never the n predictions). `out` = output activations (n×1, on GPU);
      /// `ss_tot` is precomputed once since the targets are fixed. R²/MSE/accuracy go
      /// through fused single-pass kernels (`gpu_ss_res_into`/`gpu_mse_into`/
      /// `gpu_accuracy_into`); MAE/Huber/CE go through `_into` variants writing into the
      /// preallocated `sc.metric_t*` temporaries — so the whole path allocates nothing.
      /// Matches `metric_num` exactly except accuracy differs only at the measure-zero
      /// p==0.5 tie (sigmoid outputs never land there).
      fn metric_gpu(&self, m: Metric, out: &GpuBuffer, ybuf: &GpuBuffer, sc: &Scratch, n: usize, ss_tot: f64, epoch: usize, elapsed: f64) -> f64 {
            match m {
                  Metric::Epoch => epoch as f64,
                  Metric::Lr => self.lr,
                  Metric::Time => elapsed,
                  Metric::R2 => {
                        kernels::gpu_ss_res_into(out, ybuf, &sc.metric_scalar, n);
                        1.0 - Self::download_scalar(&sc.metric_scalar) / ss_tot
                  }
                  Metric::Accuracy => {
                        kernels::gpu_accuracy_into(out, ybuf, &sc.metric_scalar, n);
                        Self::download_scalar(&sc.metric_scalar)
                  }
                  // The Loss metric is the model's ACTUAL loss (self.loss), not hardcoded.
                  Metric::Loss => {
                        let nf = n as f64;
                        match self.loss {
                              Loss::Mse => {
                                    kernels::gpu_mse_into(out, ybuf, &sc.metric_scalar, n);
                                    Self::download_scalar(&sc.metric_scalar)
                              }
                              Loss::Mae => {
                                    kernels::gpu_sub_scale_into(out, ybuf, &sc.metric_t0, n, 1.0);
                                    kernels::gpu_abs_into(&sc.metric_t0, &sc.metric_t0, n);
                                    kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.metric_scalar, &sc.reduce_ws, n, 1);
                                    Self::download_scalar(&sc.metric_scalar) / nf
                              }
                              Loss::Huber => {
                                    // delta=1: 0.5 r² for |r|≤1 else |r|-0.5, written as
                                    // 0.5·clamp(r,-1,1)² + |r| - |clamp(r,-1,1)|.
                                    kernels::gpu_sub_scale_into(out, ybuf, &sc.metric_t0, n, 1.0); // r
                                    kernels::gpu_clamp_into(&sc.metric_t0, &sc.metric_t1, n, -1.0, 1.0); // rc
                                    kernels::gpu_copy_into(&sc.metric_t1, &sc.metric_t2, n); // e = rc
                                    kernels::gpu_mul_inplace(&sc.metric_t2, &sc.metric_t1, n); // e = rc²
                                    kernels::gpu_scale_inplace(&sc.metric_t2, 0.5, n); // e = 0.5 rc²
                                    kernels::gpu_abs_into(&sc.metric_t0, &sc.metric_t0, n); // |r|
                                    kernels::gpu_add_inplace(&sc.metric_t2, &sc.metric_t0, n); // e += |r|
                                    kernels::gpu_abs_into(&sc.metric_t1, &sc.metric_t1, n); // |rc|
                                    kernels::gpu_sub_inplace(&sc.metric_t2, &sc.metric_t1, n); // e -= |rc|
                                    kernels::gpu_reduce_sum_cols_into(&sc.metric_t2, &sc.metric_scalar, &sc.reduce_ws, n, 1);
                                    Self::download_scalar(&sc.metric_scalar) / nf
                              }
                              Loss::Ce => {
                                    let eps = 1e-7;
                                    kernels::gpu_clamp_into(out, &sc.metric_t0, n, eps, 1.0 - eps); // pc
                                    kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t1, n); // ln pc
                                    kernels::gpu_mul_inplace(&sc.metric_t1, ybuf, n); // y·ln pc
                                    kernels::gpu_scale_inplace(&sc.metric_t0, -1.0, n); // -pc
                                    kernels::gpu_add_scalar_inplace(&sc.metric_t0, 1.0, n); // 1-pc
                                    kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t0, n); // ln(1-pc)
                                    kernels::gpu_copy_into(ybuf, &sc.metric_t2, n); // y
                                    kernels::gpu_scale_inplace(&sc.metric_t2, -1.0, n); // -y
                                    kernels::gpu_add_scalar_inplace(&sc.metric_t2, 1.0, n); // 1-y
                                    kernels::gpu_mul_inplace(&sc.metric_t2, &sc.metric_t0, n); // (1-y)·ln(1-pc)
                                    kernels::gpu_add_inplace(&sc.metric_t1, &sc.metric_t2, n); // sum terms
                                    kernels::gpu_reduce_sum_cols_into(&sc.metric_t1, &sc.metric_scalar, &sc.reduce_ws, n, 1);
                                    -Self::download_scalar(&sc.metric_scalar) / nf
                              }
                        }
                  }
            }
      }

      /// The colored, aligned metric line: `vals[i]` is the precomputed value of
      /// `metrics[i]` (already reduced on the GPU), so this only formats.
      fn metrics_line(&self, metrics: &[Metric], vals: &[f64]) -> String {
            let parts: Vec<String> = metrics
                  .iter()
                  .zip(vals)
                  .enumerate()
                  .map(|(i, (&m, &v))| {
                        let num = match m {
                              Metric::Epoch => format!("{:>5}", v as usize),
                              Metric::Lr => format!("{v:>7}"),
                              Metric::Time => format!("{:>9}", fmt_time(v)),
                              Metric::Loss => format!("{v:>7.4}"),
                              Metric::Accuracy => format!("{v:>6.4}"),
                              Metric::R2 => format!("{v:>8.4}"),
                        };
                        let (r, g, b) = palette(i);
                        format!("{} \x1b[38;2;{r};{g};{b}m{num}\x1b[0m", Self::label(m))
                  })
                  .collect();
            parts.join("  ")
      }

      /// Render the live dashboard with ratatui: a header block + one Chart widget
      /// per metric (x = epoch), stacked via a Layout that can't overflow.
      fn render_dashboard(&self, frame: &mut Frame, summary: &str, rows: &[Vec<f64>], ys: &[Metric]) {
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
                  let pts: Vec<(f64, f64)> = rows.iter()
                        .map(|r| (r[0].max(1.0).log10(), symlog(r[1 + j])))
                        .collect();
                  // Bounds live in symlog space; auto-scale tightly to the data so the
                  // whole curve fits, with a little padding to keep extremes off edge.
                  let lo = pts.iter().map(|p| p.1).filter(|v| v.is_finite())
                        .fold(f64::INFINITY, f64::min);
                  let hi = pts.iter().map(|p| p.1).filter(|v| v.is_finite())
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
                              String::new(),                          // origin: implicit
                              String::new(),                          // middle: omitted
                              fmt_time_axis(10f64.powf(lxmax)),       // only the latest time
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
                              if let Some(c) = buf.cell((x, y)) {
                                    if c.symbol() == symbols::line::VERTICAL {
                                          found = Some((x, c.style()));
                                          break 'find;
                                    }
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
                                          s if s == symbols::line::BOTTOM_LEFT && y < last.top() => {
                                                c.set_symbol(symbols::line::VERTICAL_RIGHT);
                                          }
                                          _ => {}
                                    }
                              }
                        }
                  }
            }
      }

      pub fn layer(mut self, spec: impl IntoLayer) -> Model {
            self.specs.push(spec.into_layer());
            self
      }

      pub fn loss(mut self, loss: Loss) -> Model {
            self.loss = loss;
            self
      }

      /// Set the learning rate. To reset between runs, rebind:
      /// `let model = model.lr(1e-8); train.run(&model, &data);`.
      pub fn lr(mut self, lr: f64) -> Model {
            self.lr = lr;
            self
      }

      fn upload(x: &crate::Mat) -> (GpuBuffer, usize, usize) {
            let std = x.as_standard_layout();
            let slice = std.as_slice().expect("upload: non-contiguous");
            (
                  GpuBuffer::upload(slice).expect("upload x"),
                  x.nrows(),
                  x.ncols(),
            )
      }

      /// Forward pass; returns activations A[0..=L] where A[0]=X and A[L]=prediction.
      fn forward(params: &[LayerParams], x: &GpuBuffer, n: usize) -> Vec<GpuBuffer> {
            let mut acts = vec![kernels::gpu_copy(x, n * params[0].in_dim).expect("copy x")];
            for p in params {
                  let prev = acts.last().expect("forward: prev act");
                  let z = kernels::gpu_gemm(prev, &p.w, n, p.out_dim, p.in_dim)
                        .expect("forward gemm");
                  let z = kernels::gpu_bias_add(&z, &p.b, n, p.out_dim).expect("forward bias");
                  let a = match p.act {
                        Activation::Relu => kernels::gpu_relu(&z, n * p.out_dim),
                        Activation::Sigmoid => kernels::gpu_sigmoid(&z, n * p.out_dim),
                        Activation::Linear => kernels::gpu_copy(&z, n * p.out_dim),
                  }
                  .expect("forward activation");
                  acts.push(a);
            }
            acts
      }

      /// Forward pass writing each layer's output into the preallocated `acts`
      /// (no allocation). The input `x` feeds layer 0 directly (no copy); the
      /// activation is applied in place (`acts[l]` holds the pre-activation, then
      /// is overwritten by its own activation). `acts[last]` ends as predictions.
      fn forward_into(params: &[LayerParams], x: &GpuBuffer, n: usize, acts: &[GpuBuffer]) {
            for (l, p) in params.iter().enumerate() {
                  let prev = if l == 0 { x } else { &acts[l - 1] };
                  // out_dim==1: z = X@w + b is a matrix-vector product. rocBLAS dispatches
                  // a full GEMM tile (32×32) for one output column, wasting 31/32 of it —
                  // dgemv reads the same operands once and is memory-bound, ~33× faster.
                  if p.out_dim == 1 {
                        kernels::gpu_matvec_bias_into(prev, &p.w, &p.b, &acts[l], n, p.in_dim);
                  } else {
                        kernels::gpu_linear_into(prev, &p.w, &p.b, &acts[l], n, p.out_dim, p.in_dim);
                  }
                  let m = n * p.out_dim;
                  match p.act {
                        Activation::Relu => kernels::gpu_relu_into(&acts[l], &acts[l], m),
                        Activation::Sigmoid => kernels::gpu_sigmoid_into(&acts[l], &acts[l], m),
                        Activation::Linear => {}
                  }
            }
      }

      /// One backward pass + SGD update, writing every gradient into preallocated
      /// `sc` (no allocation). `sc.acts` must hold this epoch's forward output and
      /// `x` feeds layer 0 as in `forward_into`. Op-for-op identical to the old
      /// in-loop backward: dz = act'(da)·, dw = aᵀ·dz, db = Σ dz, da_below = dz·wᵀ,
      /// then w -= lr·dw, b -= lr·db. `w` is read for da_below before its update.
      fn backward_step(&self, params: &[LayerParams], x: &GpuBuffer, ybuf: &GpuBuffer, n: usize, sc: &Scratch) {
            let last = params.len() - 1;
            let (da_cur, da_next) = (&sc.da_a, &sc.da_b);
            Self::loss_grad_into(self.loss, &sc.acts[last], ybuf, da_cur, n, n * params[last].out_dim);
            let mut flip = false;
            for l in (0..params.len()).rev() {
                  let (in_dim, out_dim) = (params[l].in_dim, params[l].out_dim);
                  let m = n * out_dim;
                  let da = if flip { da_next } else { da_cur };
                  let da_below = if flip { da_cur } else { da_next };
                  let grad = match params[l].act {
                        Activation::Relu => {
                              kernels::gpu_relu_backward_into(da, &sc.acts[l], &sc.dz, m);
                              &sc.dz
                        }
                        Activation::Sigmoid => {
                              kernels::gpu_sigmoid_backward_into(da, &sc.acts[l], &sc.dz, m);
                              &sc.dz
                        }
                        Activation::Linear => da,
                  };
                  let a_prev = if l == 0 { x } else { &sc.acts[l - 1] };
                  if out_dim == 1 {
                        kernels::gpu_dgemv_into(a_prev, grad, &sc.dw, n, in_dim, true);
                        kernels::gpu_reduce_sum_cols_into(grad, &sc.db, &sc.reduce_ws, n, 1);
                        if l > 0 {
                              kernels::gpu_dger_into(grad, &params[l].w, da_below, n, in_dim);
                        }
                  } else if l > 0 {
                        kernels::gpu_linear_backward_full_into(
                              grad, a_prev, &params[l].w, da_below, &sc.dw, &sc.db, &sc.reduce_ws, n, out_dim, in_dim,
                        );
                  } else {
                        kernels::gpu_linear_backward_weights_only_into(
                              grad, a_prev, &sc.dw, &sc.db, &sc.reduce_ws, n, out_dim, in_dim,
                        );
                  }
                  kernels::gpu_sgd_update(&params[l].w, &sc.dw, self.lr, in_dim * out_dim);
                  kernels::gpu_sgd_update(&params[l].b, &sc.db, self.lr, out_dim);
                  flip = !flip;
            }
      }

      /// Copy a GPU buffer of `len` f64s back to host.
      fn download_vec(buf: &GpuBuffer, len: usize) -> Vec<f64> {
            let mut v = vec![0.0f64; len];
            buf.download(&mut v).expect("gpu download");
            v
      }

      /// Download a single-element GPU buffer (a reduced scalar) to the host.
      fn download_scalar(buf: &GpuBuffer) -> f64 {
            let mut v = [0.0f64];
            buf.download(&mut v).expect("scalar download");
            v[0]
      }

      /// Forward pass + download of the output layer's predictions.
      fn predict(params: &[LayerParams], x: &GpuBuffer, n: usize) -> Vec<f64> {
            let acts = Self::forward(params, x, n);
            Self::download_vec(acts.last().expect("predict: no output"), n)
      }

      fn vram_required(specs: &[(usize, Activation)], n: usize, d: usize) -> usize {
            let mut bytes = 0usize;
            bytes += n * d * 8;
            bytes += n * 8;
            let mut max_act = 0usize;
            let mut max_wt = 0usize;
            let mut in_dim = d;
            for &(units, _) in specs {
                  bytes += in_dim * units * 8;
                  bytes += units * 8;
                  bytes += n * units * 8;
                  let a = n * units;
                  if a > max_act { max_act = a; }
                  let w = in_dim * units;
                  if w > max_wt { max_wt = w; }
                  in_dim = units;
            }
            bytes += 3 * max_act * 8;
            bytes += max_wt * 8;
            bytes += 3 * n * 8;
            bytes += 8 + 4096;
            bytes
      }

      fn fit(&self, data: &Dataset, cfg: &Train, resume: Option<&str>) {
            let rerun = !self.params.borrow().is_empty();
            let n = data.x.nrows();
            let d = data.x.ncols();
            let need = Self::vram_required(&self.specs, n, d);
            let mut free = 0usize;
            let mut total = 0usize;
            unsafe { gpu_core::hip::hipMemGetInfo(&mut free, &mut total) };
            if need > free * 9 / 10 {
                  eprintln!("VRAM budget exceeded: need {:.0} MB, free {:.0} MB / {:.0} MB total\n\
                        reduce layer widths or sample count (n={n})",
                        need as f64 / 1048576.0, free as f64 / 1048576.0, total as f64 / 1048576.0);
                  std::process::exit(1);
            }
            let start = std::time::Instant::now();
            let (xbuf, n, d) = Self::upload(&data.x);
            let ybuf = GpuBuffer::upload(
                  data.y.as_slice().expect("train: y contiguous"),
            )
            .expect("upload y");

            // Resumed weights (per-neuron, in save order) or empty for random init.
            let mut resumed = resume.map(Self::load_ogdl).unwrap_or_default();
            // NaNs in the OGDL are dead cells — training never writes them, so the
            // only way they appear is a hand-edited file. Randomize just those cells
            // (He-scaled per neuron), report the fraction, and keep training.
            if !resumed.is_empty() {
                  use rand::{Rng as _, SeedableRng as _};
                  use rand_distr::StandardNormal;
                  let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0xB1A5);
                  let total: usize = resumed.iter().map(|(ws, _)| ws.len() + 1).sum();
                  let mut nans = 0usize;
                  for (ws, b) in resumed.iter_mut() {
                        let scale = (2.0 / ws.len().max(1) as f64).sqrt();
                        for v in ws.iter_mut() {
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
                  if nans > 0 {
                        let pct = 100.0 * nans as f64 / total as f64;
                        eprintln!(
                              "\x1b[32mresume\x1b[0m\n    \x1b[1;31mNaN\x1b[0m\n        path: {}\n        {nans}/{total} weights+biases ({pct:.1}%) were NaN\n        randomized those, continuing",
                              resume.unwrap_or("")
                        );
                  }
            }
            let did_resume = !resumed.is_empty();
            if !resumed.is_empty() {
                  // The OGDL's per-neuron weight counts must match this arch fed by
                  // this data (first layer's in_dim = feature count d). Mismatch =
                  // wrong file or changed data/shape; say exactly what's off and stop.
                  let expected: usize = self.specs.iter().map(|&(u, _)| u).sum();
                  let mut why = (resumed.len() != expected).then(|| {
                        format!("OGDL has {} neurons, this model has {expected}", resumed.len())
                  });
                  if why.is_none() {
                        let mut idx = 0;
                        let mut din = d;
                        for (li, &(units, _)) in self.specs.iter().enumerate() {
                              for _ in 0..units {
                                    let got = resumed[idx].0.len();
                                    if got != din {
                                          why = Some(format!(
                                                "layer {li} expects {din} weights/neuron, OGDL has {got} (data feature count differs?)"
                                          ));
                                          break;
                                    }
                                    idx += 1;
                              }
                              if why.is_some() {
                                    break;
                              }
                              din = units;
                        }
                  }
                  if why.is_some() {
                        let ogdl_feat = resumed.first().map_or(0, |(w, _)| w.len());
                        let path = resume.unwrap_or("");
                        eprintln!(
                              "\x1b[32mresume\x1b[0m\n\
                               \x20   \x1b[1;31mdata does not match\x1b[0m\n\
                               \x20       file\n\
                               \x20           path={path}\n\
                               \x20           features={ogdl_feat}\n\
                               \x20           neurons={}\n\
                               \x20       data\n\
                               \x20           path={}\n\
                               \x20           features={d}\n\
                               \x20           neurons={expected}",
                              resumed.len(),
                              data.source,
                        );
                        std::process::exit(1);
                  }
            }
            let mut neuron = 0;
            let mut params: Vec<LayerParams> = Vec::new();
            let mut in_dim = d;
            for (li, &(units, act)) in self.specs.iter().enumerate() {
                  let (w, b) = if resumed.is_empty() {
                        let scale = (2.0 / in_dim as f64).sqrt();
                        let w0 = kernels::gpu_randn(in_dim * units, 1234 + (li as u32) * 7919)
                              .expect("randn w");
                        let w = kernels::gpu_scale(&w0, scale, in_dim * units).expect("scale w");
                        let b = GpuBuffer::upload(&vec![0.0f64; units]).expect("upload b");
                        (w, b)
                  } else {
                        // Distribute saved neurons back into this layer's W (in_dim×units,
                        // row-major: index i*units+j) and bias[j], matching save's layout.
                        let mut wh = vec![0.0f64; in_dim * units];
                        let mut bh = vec![0.0f64; units];
                        for j in 0..units {
                              let (ws, bias) = &resumed[neuron];
                              for i in 0..in_dim {
                                    wh[i * units + j] = ws[i];
                              }
                              bh[j] = *bias;
                              neuron += 1;
                        }
                        (GpuBuffer::upload(&wh).expect("upload w"), GpuBuffer::upload(&bh).expect("upload b"))
                  };
                  params.push(LayerParams { w, b, in_dim, out_dim: units, act });
                  in_dim = units;
            }
            let last = params.len() - 1;
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
                  if did_resume {
                        if let Some(path) = resume {
                              let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
                              eprintln!("resumed: {}", full.display());
                        }
                  }
                  if !summary.is_empty() {
                        eprintln!("{summary}");
                  }
            }
            let mut terminal = plotting.then(ratatui::init);
            let mut last_draw = start;
            // Trailing stop-loss on train R²: hold while it climbs. The first epoch it
            // ticks down (blow-up is exponential, so that first dip is tiny and the
            // in-VRAM weights are still ≈ the peak), save those weights once and never
            // again. If it climbs the whole way, the only save is at the end below.
            // Requires a save destination; no-op without one.
            let checkpointing = cfg.save.as_ref().is_some_and(|(p, _)| !p.is_empty());
            let mut r2_prev = f64::NAN;
            let mut saved = false;
            // Per-epoch metrics reduce to a scalar on the GPU; only the requested ones
            // are downloaded. SS_tot (R²'s denominator) depends only on the constant
            // targets, so compute it once here.
            let ss_tot = {
                  let ybar = data.y.iter().sum::<f64>() / n as f64;
                  data.y.iter().map(|v| (v - ybar).powi(2)).sum::<f64>()
            };
            // Activation + gradient buffers, allocated once and reused every epoch
            // so steady-state VRAM is flat (no per-epoch sawtooth).
            let sc = Scratch::new(&params, n);
            gpu_core::memory::alloc_freeze();
            INTERRUPTED.store(false, Ordering::SeqCst);
            unsafe { libc::signal(libc::SIGINT, on_sigint as libc::sighandler_t); }
            for e in 0..cfg.epochs {
                  if INTERRUPTED.load(Ordering::SeqCst) {
                        break;
                  }
                  Self::forward_into(&params, &xbuf, n, &sc.acts);
                  let out = &sc.acts[last];
                  let log_now = cfg.log_every > 0
                        && !cfg.metrics.is_empty()
                        && (e % cfg.log_every == 0 || e + 1 == cfg.epochs);
                  // R² is needed when the trailing stop is armed, or when it will be
                  // logged/plotted this epoch. Compute it ONCE (here, deferred) and reuse
                  // for both the stop decision and the display — never twice.
                  let want_r2 = checkpointing
                        || (log_now && cfg.metrics.contains(&Metric::R2))
                        || (plotting && plot_ys.contains(&Metric::R2));
                  // Deferred trailing-stop sync: dispatch the R² reduce now (async, fused
                  // single pass into metric_scalar) so it overlaps the backward GEMMs.
                  // No sync here — backward is dispatched immediately after.
                  if want_r2 {
                        kernels::gpu_ss_res_into(out, &ybuf, &sc.metric_scalar, n);
                  }
                  // Backprop through every layer into the preallocated scratch (no alloc),
                  // dispatched immediately — no hipDeviceSynchronize between forward and
                  // backward. The R² reduce above is ordered ahead of it on the stream.
                  self.backward_step(&params, &xbuf, &ybuf, n, &sc);
                  // Read the R² scalar (8-byte D2H) at epoch end — the lone blocking copy,
                  // landing after backward is already in flight (the deferred sync point).
                  let r2 = if want_r2 {
                        1.0 - Self::download_scalar(&sc.metric_scalar) / ss_tot
                  } else {
                        f64::NAN
                  };
                  let mut checkpointed = false;
                  if checkpointing {
                        // First epoch R² ticks down = trailing stop hit: decide once.
                        // Save the in-VRAM weights (≈ the peak, since blow-up is
                        // exponential) — but ONLY if they beat the file's stored R², so
                        // a resume that blows up straight off the saved weights can't
                        // overwrite a good checkpoint with worse ones.
                        if !saved && e > 0 && r2 < r2_prev {
                              saved = true;
                              let (parts, path) = cfg.save.as_ref().expect("save set");
                              let path = Train::resolve(path);
                              if Self::saved_r2(&path).map_or(true, |best| r2 > best) {
                                    Self::write_ogdl(&path, &Self::dump_ogdl(&params, parts, r2));
                                    checkpointed = true;
                              }
                        }
                        if r2.is_finite() {
                              r2_prev = r2;
                        }
                  }
                  let last_epoch = e + 1 == cfg.epochs;
                  if log_now || checkpointed || plotting {
                        let elapsed = start.elapsed().as_secs_f64();
                        if !plotting && (log_now || checkpointed) {
                              let vals: Vec<f64> = cfg.metrics.iter()
                                    .map(|&m| if m == Metric::R2 { r2 } else { self.metric_gpu(m, out, &ybuf, &sc, n, ss_tot, e, elapsed) })
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
                                    row.push(if m == Metric::R2 { r2 } else { self.metric_gpu(m, out, &ybuf, &sc, n, ss_tot, e, elapsed) });
                              }
                              plot_rows.push(row);
                              // Throttle live redraws to ~25 fps; always draw the last frame.
                              if e == 0
                                    || last_epoch
                                    || last_draw.elapsed().as_millis() >= 40
                              {
                                    if let Some(term) = terminal.as_mut() {
                                          let _ = term.draw(|frame| {
                                                self.render_dashboard(
                                                      frame, &summary, &plot_rows, &plot_ys,
                                                );
                                          });
                                          last_draw = std::time::Instant::now();
                                    }
                              }
                              // Quit early on q / Ctrl+C (raw mode delivers them as keys).
                              if event::poll(Duration::ZERO).unwrap_or(false) {
                                    if let Ok(Event::Key(k)) = event::read() {
                                          if k.code == KeyCode::Char('q')
                                                || (k.code == KeyCode::Char('c')
                                                      && k.modifiers.contains(KeyModifiers::CONTROL))
                                          {
                                                break;
                                          }
                                    }
                              }
                        }
                  }
            }
            gpu_core::memory::alloc_unfreeze();
            unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL); }
            if plotting {
                  ratatui::restore();
            }
            // No post-run recap: the streaming log already ends on the final epoch,
            // and the arch/data summary printed once at the top.
            //
            // End of epochs or Ctrl+C: save the current weights iff their R² beats the
            // file's stored best — keep progress when you stop on a good climb, protect
            // the file when it had blown up. Independent of the trailing stop, so a long
            // post-dip recovery still saves. Model::save enforces the R²/finite gate.
            let end_r2 = checkpointing.then(|| {
                  Self::forward_into(&params, &xbuf, n, &sc.acts);
                  let preds = Self::download_vec(&sc.acts[last], n);
                  self.metric_num(Metric::R2, 0, &preds, &data.y, n, 0.0)
            });
            *self.params.borrow_mut() = params;
            if let Some(r2) = end_r2 {
                  let (parts, path) = cfg.save.as_ref().expect("save set");
                  self.save(parts, &Train::resolve(path), r2);
            }
      }

      /// Dump the requested params to `model.ogdl` in the cwd as one OGDL block
      /// per neuron: `z{k}` with `w1..` (if `W` requested) and `b` (if requested).
      /// Call after `train()`: `model.save(&[W, B], "model.ogdl", r2)`. Refuses to
      /// overwrite a checkpoint whose stored R² already beats `r2` (anti-degradation).
      pub fn save(&self, parts: &[Param], path: &str, r2: f64) {
            let params = self.params.borrow();
            assert!(!params.is_empty(), "save: call train() first");
            // Don't overwrite a better checkpoint, and never save a blown-up (non-finite)
            // R² — "ctrl-c while r2 < file's best: don't save (protect the file)".
            if !r2.is_finite() || Self::saved_r2(path).is_some_and(|best| r2 <= best) {
                  return;
            }
            let neurons: usize = params.iter().map(|p| p.out_dim).sum();
            Self::write_ogdl(path, &Self::dump_ogdl(&params, parts, r2));
            let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
            eprintln!("saved {} ({} neurons, r2 {:.4})", full.display(), neurons, r2);
      }

      /// One OGDL block per neuron (`z{k}` → `w1..` if `W` requested, `b` if `B`),
      /// W laid out row-major `i*out_dim+j` to match `load_ogdl`'s distribution.
      fn dump_ogdl(params: &[LayerParams], parts: &[Param], r2: f64) -> String {
            let (want_w, want_b) = (parts.contains(&Param::W), parts.contains(&Param::B));
            // Header: the R² these weights achieved, so a later run can refuse to
            // overwrite this checkpoint with worse weights (anti-degradation).
            let mut out = format!("r2={r2}\n");
            let mut z = 1;
            for p in params.iter() {
                  let w = Self::download_vec(&p.w, p.in_dim * p.out_dim);
                  let b = Self::download_vec(&p.b, p.out_dim);
                  for j in 0..p.out_dim {
                        out.push_str(&format!("z{z}\n"));
                        if want_w {
                              for i in 0..p.in_dim {
                                    out.push_str(&format!("    w{}={}\n", i + 1, w[i * p.out_dim + j]));
                              }
                        }
                        if want_b {
                              out.push_str(&format!("    b={}\n", b[j]));
                        }
                        z += 1;
                  }
            }
            out
      }

      /// Write OGDL text, creating any missing parent dirs — saving should make the
      /// file, not fail because the directory isn't there yet.
      fn write_ogdl(path: &str, out: &str) {
            if let Some(parent) = std::path::Path::new(path).parent() {
                  if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent)
                              .unwrap_or_else(|e| panic!("save: mkdir {}: {e}", parent.display()));
                  }
            }
            std::fs::write(path, out).unwrap_or_else(|e| panic!("save: write {path}: {e}"));
      }

      /// Parse an OGDL dump into one `(weights, bias)` per neuron, in save order.
      /// A missing file is not an error: it just means "first run" — return empty
      /// so training starts from random init and a later run can resume.
      fn load_ogdl(path: &str) -> Vec<(Vec<f64>, f64)> {
            let text = match std::fs::read_to_string(path) {
                  Ok(t) => t,
                  Err(_) => {
                        eprintln!("no data in {path}, initialized random weights and biases");
                        return Vec::new();
                  }
            };
            let mut neurons: Vec<(Vec<f64>, f64)> = Vec::new();
            for line in text.lines() {
                  let t = line.trim();
                  if t.is_empty() {
                        continue;
                  }
                  match t.split_once('=') {
                        None => neurons.push((Vec::new(), 0.0)), // `z{k}` header
                        Some((k, _)) if k.trim() == "r2" => {} // file's stored best R²
                        Some((k, v)) => {
                              let val: f64 = v.trim().parse().expect("resume: parse value");
                              let cur = neurons.last_mut().expect("resume: w/b before z");
                              if k.trim_start().starts_with('b') {
                                    cur.1 = val;
                              } else {
                                    cur.0.push(val);
                              }
                        }
                  }
            }
            neurons
      }

      /// The R² stored in an OGDL header (`r2=`), or None if the file is missing or
      /// predates the header. Used to refuse overwriting a better checkpoint.
      fn saved_r2(path: &str) -> Option<f64> {
            let text = std::fs::read_to_string(path).ok()?;
            for line in text.lines() {
                  if let Some((k, v)) = line.trim().split_once('=') {
                        if k.trim() == "r2" {
                              return v.trim().parse().ok();
                        }
                  }
            }
            None
      }

      pub fn eval(&self, data: &Dataset) {
            let params = self.params.borrow();
            assert!(!params.is_empty(), "eval: call train() first");
            let (xbuf, n, _d) = Self::upload(&data.x);
            let preds = Self::predict(&params, &xbuf, n);
            if data.has_target {
                  let acc = self.metric_num(Metric::Accuracy, 0, &preds, &data.y, n, 0.0);
                  let correct = (acc * n as f64).round() as usize;
                  eprintln!("eval: accuracy = {acc:.4} ({correct}/{n})");
            }
            let path = std::path::Path::new(&data.source).parent().unwrap_or(std::path::Path::new("."));
            let out = path.join("submission.csv");
            let mut w = std::io::BufWriter::new(std::fs::File::create(&out).expect("create submission.csv"));
            use std::io::Write;
            writeln!(w, "id,prediction").expect("write header");
            for (i, p) in preds.iter().enumerate() {
                  writeln!(w, "{i},{p}").expect("write row");
            }
            eprintln!("eval: {n} predictions → {}", out.display());
      }
}

impl Default for Model {
      fn default() -> Self {
            Self::new()
      }
}

#[cfg(test)]
mod metric_gpu_tests {
      use super::*;
      use std::cell::RefCell;

      // Cross-check every GPU metric against an independent CPU reference on real
      // predictions (random-init forward over the real churn data). Proves the GPU
      // reductions are correct without touching the user's model.ogdl.
      #[test]
      fn gpu_metrics_match_cpu_reference() {
            const TRAIN: &str = "/home/nate/Desktop/playground-series-s6e3/train.csv";
            if !std::path::Path::new(TRAIN).exists() {
                  eprintln!("skip: {TRAIN} absent");
                  return;
            }
            gpu_core::hip::set_device(0).expect("set_device");

            let (train, _) = crate::dataset::Data::load().set(TRAIN).target("Churn").prepare();
            let x = &train.x;
            let y = &train.y;
            let n = x.nrows();
            let d = x.ncols();

            // Two-layer params, random init (as fit does) — just to get real GPU preds.
            let h = 8usize;
            let w1 = kernels::gpu_randn(d * h, 11).expect("w1");
            let b1 = GpuBuffer::upload(&vec![0.0f64; h]).expect("b1");
            let w2 = kernels::gpu_randn(h, 22).expect("w2");
            let b2 = GpuBuffer::upload(&vec![0.0f64; 1]).expect("b2");
            let params = vec![
                  LayerParams { w: w1, b: b1, in_dim: d, out_dim: h, act: Activation::Relu },
                  LayerParams { w: w2, b: b2, in_dim: h, out_dim: 1, act: Activation::Sigmoid },
            ];

            let (xbuf, nn, _d) = Model::upload(x);
            assert_eq!(nn, n);
            let acts = Model::forward(&params, &xbuf, n);
            let out = acts.last().expect("out");
            let p = Model::download_vec(out, n);

            // GPU-side scratch, mirroring fit.
            let ybuf = GpuBuffer::upload(y.as_slice().expect("y contig")).expect("ybuf");
            let sc = Scratch::new(&params, n);
            let ybar = y.iter().sum::<f64>() / n as f64;
            let ss_tot = y.iter().map(|v| (v - ybar).powi(2)).sum::<f64>();

            // Independent CPU references.
            let ss_res: f64 = (0..n).map(|i| (y[i] - p[i]).powi(2)).sum();
            let r2_ref = 1.0 - ss_res / ss_tot;
            let acc_ref = (0..n).filter(|&i| (p[i] >= 0.5) == (y[i] >= 0.5)).count() as f64 / n as f64;
            let mse_ref = (0..n).map(|i| (p[i] - y[i]).powi(2)).sum::<f64>() / n as f64;
            let mae_ref = (0..n).map(|i| (p[i] - y[i]).abs()).sum::<f64>() / n as f64;
            let huber_ref = (0..n).map(|i| { let r = p[i] - y[i]; if r.abs() <= 1.0 { 0.5 * r * r } else { r.abs() - 0.5 } }).sum::<f64>() / n as f64;
            let eps = 1e-7;
            let bce_ref = -(0..n).map(|i| { let pc = p[i].clamp(eps, 1.0 - eps); y[i] * pc.ln() + (1.0 - y[i]) * (1.0 - pc).ln() }).sum::<f64>() / n as f64;

            let close = |a: f64, b: f64, what: &str| {
                  let tol = 1e-6 * a.abs().max(b.abs()).max(1.0);
                  assert!((a - b).abs() <= tol, "{what}: gpu={a} cpu={b} diff={}", (a - b).abs());
            };
            let mdl = |loss: Loss| Model { specs: vec![], loss, lr: 0.01, params: RefCell::new(vec![]) };

            close(mdl(Loss::Mse).metric_gpu(Metric::R2, out, &ybuf, &sc, n, ss_tot, 0, 0.0), r2_ref, "R2");
            close(mdl(Loss::Mse).metric_gpu(Metric::Accuracy, out, &ybuf, &sc, n, ss_tot, 0, 0.0), acc_ref, "Accuracy");
            close(mdl(Loss::Mse).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, ss_tot, 0, 0.0), mse_ref, "MSE");
            close(mdl(Loss::Mae).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, ss_tot, 0, 0.0), mae_ref, "MAE");
            close(mdl(Loss::Huber).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, ss_tot, 0, 0.0), huber_ref, "Huber");
            close(mdl(Loss::Ce).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, ss_tot, 0, 0.0), bce_ref, "BCE");

            eprintln!("OK n={n} d={d}  R2={r2_ref:.6}  acc={acc_ref:.6}  mse={mse_ref:.6}  mae={mae_ref:.6}  huber={huber_ref:.6}  bce={bce_ref:.6}");
      }

      // The preallocated training loop must (1) compute the same forward as the
      // retained allocate-return path, (2) allocate ZERO GPU buffers per epoch in
      // steady state (flat VRAM, no sawtooth), and (3) still gradient-descend
      // (train R² rises). Features are standardized on-GPU with existing reduce +
      // broadcast kernels so the raw frequency-encoded churn columns don't saturate
      // sigmoid — a well-posed problem on real data, not a hand-rolled scaler.
      #[test]
      fn fit_loop_allocations_flat() {
            const TRAIN: &str = "/home/nate/Desktop/playground-series-s6e3/train.csv";
            if !std::path::Path::new(TRAIN).exists() {
                  eprintln!("skip: {TRAIN} absent");
                  return;
            }
            gpu_core::hip::set_device(0).expect("set_device");

            let (train, _) = crate::dataset::Data::load().set(TRAIN).target("Churn").prepare();
            let x = &train.x;
            let y = &train.y;
            let n = x.nrows();
            let d = x.ncols();

            let (xraw, _, _) = Model::upload(x);
            let mean = kernels::gpu_reduce_mean_cols(&xraw, n, d).expect("mean");
            let var = kernels::gpu_reduce_var_cols(&xraw, n, d).expect("var");
            kernels::gpu_add_scalar_inplace(&var, 1e-8, d);
            let std = kernels::gpu_sqrt(&var, d).expect("std");
            let xc = kernels::gpu_broadcast_sub(&xraw, &mean, n * d, d).expect("center");
            let xbuf = kernels::gpu_broadcast_div(&xc, &std, n * d, d).expect("scale");
            let ybuf = GpuBuffer::upload(y.as_slice().expect("y contig")).expect("ybuf");

            // Two-layer relu→sigmoid, He init exactly as fit does.
            let h = 16usize;
            let mk = |fan_in: usize, units: usize, seed: u32, act: Activation| {
                  let scale = (2.0 / fan_in as f64).sqrt();
                  let w0 = kernels::gpu_randn(fan_in * units, seed).expect("randn");
                  let w = kernels::gpu_scale(&w0, scale, fan_in * units).expect("scale w");
                  let b = GpuBuffer::upload(&vec![0.0f64; units]).expect("b");
                  LayerParams { w, b, in_dim: fan_in, out_dim: units, act }
            };
            let params = vec![mk(d, h, 11, Activation::Relu), mk(h, 1, 22, Activation::Sigmoid)];
            let last = params.len() - 1;

            // (1) forward_into must equal the retained allocate-return forward.
            let acts_ref = Model::forward(&params, &xbuf, n);
            let out_ref = Model::download_vec(acts_ref.last().expect("ref out"), n);
            let sc = Scratch::new(&params, n);
            Model::forward_into(&params, &xbuf, n, &sc.acts);
            let out_into = Model::download_vec(&sc.acts[last], n);
            let fwd_diff = out_ref.iter().zip(&out_into).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
            assert!(fwd_diff < 1e-9, "forward_into != forward, maxdiff={fwd_diff}");

            // (2)+(3) Train through the preallocated loop, measuring per-epoch GPU
            // allocations and train R². download_vec is host-only (no GpuBuffer
            // alloc), so reading R² never perturbs the count.
            let model = Model { specs: vec![], loss: Loss::Mse, lr: 0.5, params: RefCell::new(vec![]) };
            let ybar = y.iter().sum::<f64>() / n as f64;
            let ss_tot: f64 = y.iter().map(|v| (v - ybar).powi(2)).sum();
            let cpu_r2 = |p: &[f64]| {
                  let ssr: f64 = (0..n).map(|i| (y[i] - p[i]).powi(2)).sum();
                  1.0 - ssr / ss_tot
            };

            const EPOCHS: usize = 60;
            let mut counts = Vec::with_capacity(EPOCHS);
            let mut r2s = Vec::with_capacity(EPOCHS);
            for _ in 0..EPOCHS {
                  let _ = gpu_core::memory::alloc_count_reset();
                  Model::forward_into(&params, &xbuf, n, &sc.acts);
                  model.backward_step(&params, &xbuf, &ybuf, n, &sc);
                  counts.push(gpu_core::memory::alloc_count_reset());
                  r2s.push(cpu_r2(&Model::download_vec(&sc.acts[last], n)));
            }

            assert!(counts.iter().all(|&c| c == 0), "per-epoch GPU allocs not flat-zero: {counts:?}");
            assert!(r2s.iter().all(|v| v.is_finite()), "non-finite R²: {r2s:?}");
            assert!(r2s[EPOCHS - 1] > r2s[0], "R² did not rise: first={} last={}", r2s[0], r2s[EPOCHS - 1]);

            eprintln!("alloc/epoch (first 5)={:?} ... all_zero={}  R2 first={:.6} last={:.6}",
                  &counts[..5.min(counts.len())], counts.iter().all(|&c| c == 0), r2s[0], r2s[EPOCHS - 1]);
      }

      #[test]
      fn train_rs_model_fits_with_headroom() {
            let specs = vec![
                  (200, Activation::Relu),
                  (100, Activation::Relu),
                  (200, Activation::Relu),
                  (1, Activation::Sigmoid),
            ];
            let n = 594194usize;
            let d = 46usize;
            let need = Model::vram_required(&specs, n, d);
            let need_mb = need as f64 / 1048576.0;

            gpu_core::hip::set_device(0).expect("set_device");
            let mut free = 0usize;
            let mut total = 0usize;
            unsafe { gpu_core::hip::hipMemGetInfo(&mut free, &mut total) };
            let free_mb = free as f64 / 1048576.0;
            let headroom_mb = free_mb - need_mb;

            eprintln!("model needs {need_mb:.0} MB, free {free_mb:.0} MB (total {:.0} MB), headroom {headroom_mb:.0} MB",
                  total as f64 / 1048576.0);
            assert!(headroom_mb > 1500.0,
                  "model needs {need_mb:.0} MB but only {free_mb:.0} MB free — {headroom_mb:.0} MB \
                   headroom, need >1500 MB for rocBLAS workspace + desktop compositing");
      }
}



