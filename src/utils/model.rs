use crate::dataset::Dataset;
use colored::Color;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use std::cell::RefCell;
use std::io::IsTerminal as _;
use std::io::Write as _;
use termplot_rs::ChartContext;

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

/// Restores the terminal (leaves the alternate screen, shows the cursor) when
/// dropped — on normal scope exit AND during a panic unwind, so a mid-train
/// crash never leaves the terminal stuck. It does NOT catch/swallow the panic.
struct TermGuard;
impl TermGuard {
      fn new() -> Self {
            TermGuard
      }
}
impl Drop for TermGuard {
      fn drop(&mut self) {
            let _ = write!(std::io::stdout(), "\x1b[?1049l\x1b[?25h");
            let _ = std::io::stdout().flush();
      }
}

/// Async-signal-safe SIGINT handler: a Drop guard never runs when SIGINT kills
/// the process mid-frame, leaving the terminal in the alt screen with the cursor
/// hidden (and a stray byte from a half-written escape). This restores the
/// terminal with a single raw `write` and exits 130. Installed only while
/// live-plotting; `signal`/`write`/`_exit` are all async-signal-safe.
extern "C" fn restore_term_on_sigint(_sig: libc::c_int) {
      const RESTORE: &[u8] = b"\r\x1b[?1049l\x1b[?25h\r\n";
      unsafe {
            libc::write(1, RESTORE.as_ptr().cast(), RESTORE.len());
            libc::_exit(130);
      }
}

/// The i-th palette color as a truecolor `colored::Color` (cycles past 12).
fn palette_color(i: usize) -> Color {
      let (r, g, b) = PALETTE[i % PALETTE.len()];
      Color::TrueColor { r, g, b }
}

/// Live terminal size (cols, rows) via crossterm's ioctl on the stdout fd —
/// works for a spawned child where `stty </dev/tty` does not. Falls back to a
/// conservative size when there is no terminal (piped output).
fn term_size() -> (usize, usize) {
      match crossterm::terminal::size() {
            Ok((c, r)) if c >= 40 && r >= 8 => (c as usize, r as usize),
            _ => (100, 28),
      }
}

struct LayerParams {
      w: GpuBuffer,
      b: GpuBuffer,
      in_dim: usize,
      out_dim: usize,
      act: Activation,
}

pub struct Model {
      specs: Vec<(usize, Activation)>,
      loss: Loss,
      lr: f64,
      metrics: Vec<Metric>,
      log_every: usize,
      plot: Vec<Metric>,
      params: RefCell<Vec<LayerParams>>,
}

impl Model {
      pub fn new() -> Model {
            Model {
                  specs: Vec::new(),
                  loss: Loss::Mse,
                  lr: 0.01,
                  metrics: Vec::new(),
                  log_every: 1,
                  plot: Vec::new(),
                  params: RefCell::new(Vec::new()),
            }
      }

      pub fn log(mut self, metrics: &[Metric]) -> Model {
            self.metrics = metrics.to_vec();
            self
      }

      pub fn log_every(mut self, every: usize) -> Model {
            self.log_every = every;
            self
      }

      pub fn plot(mut self, metrics: &[Metric]) -> Model {
            self.plot = metrics.to_vec();
            self
      }

      /// dL/dA at the output for the chosen loss (z = predictions, y = targets).
      fn loss_grad(loss: Loss, z: &GpuBuffer, y: &GpuBuffer, n: usize) -> GpuBuffer {
            match loss {
                  Loss::Mse => {
                        let d = kernels::gpu_sub(z, y, n).expect("mse sub");
                        kernels::gpu_scale(&d, 2.0, n).expect("mse scale")
                  }
                  Loss::Mae => {
                        let d = kernels::gpu_sub(z, y, n).expect("mae sub");
                        kernels::gpu_sign(&d, n).expect("mae sign")
                  }
                  Loss::Huber => {
                        let d = kernels::gpu_sub(z, y, n).expect("huber sub");
                        kernels::gpu_clamp(&d, n, -1.0, 1.0).expect("huber clamp")
                  }
                  Loss::Ce => {
                        let q = kernels::gpu_div(y, z, n).expect("ce div");
                        kernels::gpu_scale(&q, -1.0, n).expect("ce scale")
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

      /// The colored, aligned metric line for the current epoch.
      fn metrics_line(&self, epoch: usize, p: &[f64], y: &crate::Vec1, n: usize, elapsed: f64) -> String {
            let parts: Vec<String> = self
                  .metrics
                  .iter()
                  .enumerate()
                  .map(|(i, &m)| {
                        let v = self.metric_num(m, epoch, p, y, n, elapsed);
                        let num = match m {
                              Metric::Epoch => format!("{:>5}", v as usize),
                              Metric::Lr => format!("{v:>7}"),
                              Metric::Time => format!("{v:>5.2}s"),
                              Metric::Loss => format!("{v:>7.4}"),
                              Metric::Accuracy => format!("{v:>6.4}"),
                              Metric::R2 => format!("{v:>8.4}"),
                        };
                        let (r, g, b) = PALETTE[i % PALETTE.len()];
                        format!("{} \x1b[38;2;{r};{g};{b}m{num}\x1b[0m", Self::label(m))
                  })
                  .collect();
            parts.join("  ")
      }

      /// Render one facet per metric (x = epoch) into a tiled grid string,
      /// drawn in-process with termplot-rs (braille). `ys` excludes Epoch.
      fn chart_grid(&self, rows: &[Vec<f64>], ys: &[Metric], tcols: usize, avail_rows: usize) -> String {
            if rows.is_empty() || ys.is_empty() {
                  return String::new();
            }
            let k = if tcols >= 90 { 2 } else { 1 }; // facet columns
            let chart_rows = ys.len().div_ceil(k);
            // No border/title: each box is exactly chart_h braille rows, so the full
            // height budget goes to the data area (8 px of vertical resolution per
            // row). One blank line separates stacked chart-rows. The label is drawn
            // inside the canvas instead of costing a border+title (3 rows).
            let chart_w = ((tcols - (k - 1) * 2) / k).saturating_sub(2).max(12);
            let gaps = chart_rows.saturating_sub(1);
            let chart_h = (avail_rows.saturating_sub(gaps) / chart_rows).max(8);

            let boxes: Vec<Vec<String>> = ys
                  .iter()
                  .enumerate()
                  .map(|(j, &m)| {
                        let pts: Vec<(f64, f64)> =
                              rows.iter().map(|r| (r[0], r[1 + j])).collect();
                        let mut chart = ChartContext::new(chart_w, chart_h);
                        let (rx, ry) = ChartContext::get_auto_range(&pts, 0.05);
                        chart.draw_axes(rx, ry, Some(Color::TrueColor { r: 90, g: 90, b: 90 }));
                        chart.line_chart(&pts, Some(palette_color(j)));
                        chart.text(Self::label(m), 0.0, 0.98, Some(palette_color(j)));
                        chart
                              .canvas
                              .render_with_options(false, None)
                              .split('\n')
                              .map(str::to_string)
                              .collect()
                  })
                  .collect();

            let mut out = String::new();
            for (ri, chunk) in boxes.chunks(k).enumerate() {
                  if ri > 0 {
                        out.push('\n'); // blank line between stacked chart-rows
                  }
                  let h = chunk.iter().map(Vec::len).max().unwrap_or(0);
                  for li in 0..h {
                        for (ci, b) in chunk.iter().enumerate() {
                              if ci > 0 {
                                    out.push_str("  ");
                              }
                              out.push_str(b.get(li).map_or("", String::as_str));
                              out.push_str("\x1b[0m");
                        }
                        out.push('\n');
                  }
            }
            out
      }

      /// Full live frame: summary line, current metrics line, then the facet grid.
      fn dashboard(&self, summary: &str, cur: &str, rows: &[Vec<f64>], ys: &[Metric], tcols: usize, trows: usize) -> String {
            // Header = the multi-line arch/data block + the current-metrics line;
            // the rest of the terminal height is chart area.
            let header_lines = summary.lines().count() + 1;
            let grid = self.chart_grid(rows, ys, tcols, trows.saturating_sub(header_lines));
            format!("{summary}\n{cur}\n{grid}")
      }

      pub fn layer(mut self, spec: impl IntoLayer) -> Model {
            self.specs.push(spec.into_layer());
            self
      }

      pub fn loss(mut self, loss: Loss) -> Model {
            self.loss = loss;
            self
      }

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

      pub fn train(&self, data: &Dataset, epochs: usize) {
            let start = std::time::Instant::now();
            let (xbuf, n, d) = Self::upload(&data.x);
            let ybuf = GpuBuffer::upload(
                  data.y.as_slice().expect("train: y contiguous"),
            )
            .expect("upload y");

            let mut params: Vec<LayerParams> = Vec::new();
            let mut in_dim = d;
            for (li, &(units, act)) in self.specs.iter().enumerate() {
                  let scale = (2.0 / in_dim as f64).sqrt();
                  let w0 = kernels::gpu_randn(in_dim * units, 1234 + (li as u32) * 7919)
                        .expect("randn w");
                  let w = kernels::gpu_scale(&w0, scale, in_dim * units).expect("scale w");
                  let b = GpuBuffer::upload(&vec![0.0f64; units]).expect("upload b");
                  params.push(LayerParams { w, b, in_dim, out_dim: units, act });
                  in_dim = units;
            }
            let last = params.len() - 1;
            let summary = if self.metrics.is_empty() {
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
            let plot_ys: Vec<Metric> = self
                  .plot
                  .iter()
                  .copied()
                  .filter(|&m| m != Metric::Epoch && m != Metric::Time)
                  .collect();
            let mut plot_rows: Vec<Vec<f64>> = Vec::new();

            // Live plotting owns the screen; the scrolling .log is shown inline in
            // the dashboard header instead. Without .plot, fall back to stderr logs.
            // The alternate screen buffer clips (never scrolls), so the in-place
            // redraw can't smear even if a frame slightly overflows.
            // Only take over the screen when stdout is a real terminal; piped or
            // headless runs fall through to the stderr log path (no escape spam).
            let plotting = !self.plot.is_empty() && std::io::stdout().is_terminal();
            if plotting {
                  // SIGINT (Ctrl+C) kills the process before any Drop runs, so a
                  // raw-write handler restores the terminal instead of leaking.
                  unsafe {
                        libc::signal(libc::SIGINT, restore_term_on_sigint as libc::sighandler_t);
                  }
                  let _ = write!(std::io::stdout(), "\x1b[?1049h\x1b[?25l"); // alt screen, hide cursor
                  let _ = std::io::stdout().flush();
            } else if !summary.is_empty() {
                  eprintln!("{summary}");
            }
            let mut last_draw = start;
            let mut final_frame = String::new();
            // Block-scoped so TermGuard::drop restores the terminal the moment the
            // loop exits — on normal completion AND on panic unwind — before the
            // final frame is reprinted to the main buffer below.
            {
            let _term_guard = plotting.then(TermGuard::new);
            for e in 0..epochs {
                  let acts = Self::forward(&params, &xbuf, n);
                  // dL/dA at the output, averaged over the batch, then backprop
                  // through every layer's activation uniformly.
                  let out_last = params[last].out_dim;
                  let g = Self::loss_grad(self.loss, acts.last().expect("output"), &ybuf, n * out_last);
                  let mut grad_out: Option<GpuBuffer> =
                        Some(kernels::gpu_scale(&g, 1.0 / n as f64, n * out_last).expect("scale dA"));
                  for l in (0..params.len()).rev() {
                        let out_dim = params[l].out_dim;
                        let a_l = &acts[l + 1];
                        let da = grad_out.take().expect("dA into layer");
                        let dz = match params[l].act {
                              Activation::Relu => kernels::gpu_relu_backward(&da, a_l, n * out_dim)
                                    .expect("activation backward"),
                              Activation::Sigmoid => {
                                    kernels::gpu_sigmoid_backward(&da, a_l, n * out_dim)
                                          .expect("activation backward")
                              }
                              Activation::Linear => da,
                        };
                        let in_dim = params[l].in_dim;
                        let a_prev = &acts[l];
                        let dw = kernels::gpu_gemm_at(a_prev, &dz, in_dim, out_dim, n)
                              .expect("dw gemm_at");
                        let db = kernels::gpu_reduce_sum_cols(&dz, n, out_dim)
                              .expect("db reduce");
                        if l > 0 {
                              grad_out = Some(
                                    kernels::gpu_gemm_bt(&dz, &params[l].w, n, in_dim, out_dim)
                                          .expect("dA gemm_bt"),
                              );
                        }
                        kernels::gpu_sgd_update(&params[l].w, &dw, self.lr, in_dim * out_dim);
                        kernels::gpu_sgd_update(&params[l].b, &db, self.lr, out_dim);
                  }
                  let log_now = self.log_every > 0
                        && !self.metrics.is_empty()
                        && (e % self.log_every == 0 || e + 1 == epochs);
                  let last_epoch = e + 1 == epochs;
                  if log_now || plotting {
                        let mut p = vec![0.0f64; n];
                        acts.last().expect("preds").download(&mut p).expect("preds download");
                        let elapsed = start.elapsed().as_secs_f64();
                        if !plotting && log_now {
                              eprintln!("{}", self.metrics_line(e, &p, &data.y, n, elapsed));
                        }
                        if plotting {
                              let mut row = vec![e as f64];
                              for &m in &plot_ys {
                                    row.push(self.metric_num(m, e, &p, &data.y, n, elapsed));
                              }
                              plot_rows.push(row);
                              // Throttle live redraws to ~25 fps; always draw the last frame.
                              if e == 0
                                    || last_epoch
                                    || last_draw.elapsed().as_millis() >= 40
                              {
                                    let (tcols, trows) = term_size();
                                    let cur = self.metrics_line(e, &p, &data.y, n, elapsed);
                                    let frame = self
                                          .dashboard(&summary, &cur, &plot_rows, &plot_ys, tcols, trows);
                                    let _ = write!(std::io::stdout(), "\x1b[H{frame}\x1b[0J");
                                    let _ = std::io::stdout().flush();
                                    final_frame = frame;
                                    last_draw = std::time::Instant::now();
                              }
                        }
                  }
            }
            } // end guard scope: terminal restored here (alt screen left, cursor shown)
            if plotting {
                  // Now on the main buffer; persist the final frame in scrollback.
                  let _ = write!(std::io::stdout(), "{final_frame}");
                  let _ = std::io::stdout().flush();
            }
            *self.params.borrow_mut() = params;
      }

      pub fn eval(&self, data: &Dataset) {
            let params = self.params.borrow();
            assert!(!params.is_empty(), "eval: call train() first");
            let (xbuf, n, _d) = Self::upload(&data.x);
            let acts = Self::forward(&params, &xbuf, n);
            let pred = acts.last().expect("eval: prediction");
            let mut probs = vec![0.0f64; n];
            pred.download(&mut probs).expect("eval download");
            let mut correct = 0usize;
            for i in 0..n {
                  let p = if probs[i] >= 0.5 { 1.0 } else { 0.0 };
                  if (p - data.y[i]).abs() < 0.5 {
                        correct += 1;
                  }
            }
            let acc = correct as f64 / n as f64;
            println!("eval: accuracy = {:.4} ({correct}/{n})", acc);
      }
}

impl Default for Model {
      fn default() -> Self {
            Self::new()
      }
}
