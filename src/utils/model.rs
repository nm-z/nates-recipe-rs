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

/// Activation function for a dense layer: `.layer((64, relu))`.
#[derive(Clone, Copy, PartialEq)]
pub enum Activation {
	Relu,
	Sigmoid,
	Linear,
	// Preserve negative signal: LeakyRelu (×0.01 for x<0), PRelu (learnable slope),
	// Elu/Selu (exponential for x<0), Tanh (→[-1,1]), Silu/Swish (x·σ(x)), Gelu
	// (transformer default). Tanh/Leaky/PRelu backprop from the output; Elu/Selu/
	// Silu/Gelu backprop from the pre-activation (saved in forward).
	LeakyRelu,
	PRelu,
	Elu,
	Selu,
	Tanh,
	Silu,
	Gelu,
}

/// Leaky-ReLU negative slope, and PReLU's initial (then learned) slope.
const LEAKY_ALPHA: f64 = 0.01;
const PRELU_INIT: f64 = 0.25;
/// ELU negative-saturation scale (SELU's fixed constants live in gpu-core's selu).
const ELU_ALPHA: f64 = 1.0;

/// A layer in the stack: a dense layer (`units`, activation), or a learned token
/// `Embed`ding lookup (each input column is a token id → `dim`-vector).
#[derive(Clone, Copy)]
pub enum LayerSpec {
	Dense(usize, Activation),
	Embed(usize),
	Attn(usize),
}

/// Accepts `units` (linear dense), `(units, activation)`, or `embed(dim)` for
/// `Model::layer`.
pub trait IntoLayer {
	fn into_layer(self) -> LayerSpec;
}
impl IntoLayer for usize {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense(self, Activation::Linear)
	}
}
impl IntoLayer for (usize, Activation) {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense(self.0, self.1)
	}
}

/// `embed(dim)` layer: a learned `dim`-wide embedding looked up per input token
/// id (the encoder emits free-text columns as token-id sequences). The output is
/// `(input columns) × dim` wide — the flattened sequence of token vectors.
pub struct EmbedSpec(usize);
#[allow(non_upper_case_globals)]
pub fn embed(dim: usize) -> EmbedSpec {
	EmbedSpec(dim)
}
impl IntoLayer for EmbedSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Embed(self.0)
	}
}

/// `attn(heads)` layer: bare multi-head self-attention over the token sequence
/// (Q/K/V projections → scaled dot-product per head → softmax → context → output
/// projection). Input/output width unchanged (`S×d`). `d` must divide by `heads`.
pub struct AttnSpec(usize);
#[allow(non_upper_case_globals)]
pub fn attn(heads: usize) -> AttnSpec {
	AttnSpec(heads)
}
impl IntoLayer for AttnSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Attn(self.0)
	}
}

/// Loss function: `.loss(mse)`, `.loss(ce)`, etc.
#[derive(Clone, Copy, PartialEq)]
pub enum Loss {
	Mse,
	Mae,
	/// Softmax cross-entropy (multi-class).
	Ce,
	/// Binary cross-entropy.
	Bce,
	Huber,
}

impl Loss {
	fn is_classification(self) -> bool {
		matches!(self, Loss::Ce | Loss::Bce)
	}
	fn score_key(self) -> &'static str {
		if self.is_classification() {
			"acc"
		} else {
			"r2"
		}
	}
}

#[allow(non_upper_case_globals)]
pub const relu: Activation = Activation::Relu;
#[allow(non_upper_case_globals)]
pub const sig: Activation = Activation::Sigmoid;
#[allow(non_upper_case_globals)]
pub const linear: Activation = Activation::Linear;
#[allow(non_upper_case_globals)]
pub const leak: Activation = Activation::LeakyRelu;
#[allow(non_upper_case_globals)]
pub const prelu: Activation = Activation::PRelu;
#[allow(non_upper_case_globals)]
pub const elu: Activation = Activation::Elu;
#[allow(non_upper_case_globals)]
pub const selu: Activation = Activation::Selu;
#[allow(non_upper_case_globals)]
pub const tanh: Activation = Activation::Tanh;
#[allow(non_upper_case_globals)]
pub const silu: Activation = Activation::Silu;
#[allow(non_upper_case_globals)]
pub const swish: Activation = Activation::Silu;
#[allow(non_upper_case_globals)]
pub const gelu: Activation = Activation::Gelu;
#[allow(non_upper_case_globals)]
pub const mse: Loss = Loss::Mse;
#[allow(non_upper_case_globals)]
pub const mae: Loss = Loss::Mae;
#[allow(non_upper_case_globals)]
pub const ce: Loss = Loss::Ce;
#[allow(non_upper_case_globals)]
pub const bce: Loss = Loss::Bce;
#[allow(non_upper_case_globals)]
pub const huber: Loss = Loss::Huber;

/// Which parameters `save` writes — pass `w`, `b`, or both (consts in the crate
/// root, kept out of this module so they don't shadow local `w`/`b` bindings).
#[derive(Clone, Copy, PartialEq)]
pub enum Param {
	W,
	B,
}

pub enum SaveItem {
	W,
	B,
	Col(String),
}

impl From<Param> for SaveItem {
	fn from(p: Param) -> Self {
		match p {
			Param::W => SaveItem::W,
			Param::B => SaveItem::B,
		}
	}
}

impl From<&str> for SaveItem {
	fn from(s: &str) -> Self {
		SaveItem::Col(s.to_string())
	}
}

impl From<&String> for SaveItem {
	fn from(s: &String) -> Self {
		SaveItem::Col(s.clone())
	}
}

pub trait RunData {
	fn dataset(&self) -> &Dataset;
	fn target_names(&self) -> Vec<String>;
	fn raw_rows(&self) -> Option<Vec<Vec<String>>>;
	fn raw_headers(&self) -> Option<Vec<String>>;
}

impl RunData for Dataset {
	fn dataset(&self) -> &Dataset {
		self
	}
	fn target_names(&self) -> Vec<String> {
		Vec::new()
	}
	fn raw_rows(&self) -> Option<Vec<Vec<String>>> {
		None
	}
	fn raw_headers(&self) -> Option<Vec<String>> {
		None
	}
}

impl RunData for Option<Dataset> {
	fn dataset(&self) -> &Dataset {
		self.as_ref().expect("no test dataset — use .test() or .split()")
	}
	fn target_names(&self) -> Vec<String> {
		Vec::new()
	}
	fn raw_rows(&self) -> Option<Vec<Vec<String>>> {
		None
	}
	fn raw_headers(&self) -> Option<Vec<String>> {
		None
	}
}

struct LastRun {
	model: *const Model,
	score: f64,
	preds: Option<Vec<f64>>,
	n: usize,
	k: usize,
	target_names: Vec<String>,
	raw_test_rows: Option<Vec<Vec<String>>>,
	raw_test_headers: Option<Vec<String>>,
}

impl Default for LastRun {
	fn default() -> Self {
		LastRun {
			model: std::ptr::null(),
			score: f64::NAN,
			preds: None,
			n: 0,
			k: 0,
			target_names: Vec::new(),
			raw_test_rows: None,
			raw_test_headers: None,
		}
	}
}

pub struct Train {
	epochs: usize,
	log_every: usize,
	metrics: Vec<Metric>,
	plot: Vec<Metric>,
	resume: Option<String>,
	last: RefCell<LastRun>,
}

impl Train {
	pub fn new() -> Train {
		Train {
			epochs: 1,
			log_every: 1,
			metrics: Vec::new(),
			plot: Vec::new(),
			resume: None,
			last: RefCell::new(LastRun::default()),
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

	pub fn log(mut self, metrics: impl IntoIterator<Item = Metric>) -> Train {
		self.metrics = metrics.into_iter().collect();
		self
	}

	pub fn plot(mut self, metrics: impl IntoIterator<Item = Metric>) -> Train {
		self.plot = metrics.into_iter().collect();
		self
	}

	pub fn resume(mut self, path: impl Into<String>) -> Train {
		self.resume = Some(path.into());
		self
	}

	pub fn run(&self, model: &Model, data: &impl RunData) {
		let ds = data.dataset();
		if ds.has_target && self.epochs > 0 {
			let resume = self.resume.as_deref().map(Self::resolve);
			model.fit(ds, self, resume.as_deref());
			if INTERRUPTED.load(Ordering::SeqCst) {
				eprintln!("\x1b[33minterrupted\x1b[0m");
			}
			let score = {
				let params = model.params.borrow();
				if params.is_empty() {
					f64::NAN
				} else {
					let _key = model.loss.score_key();
					let last = params.len() - 1;
					let k = params[last].out_dim;
					let n = ds.x.nrows();
					let sc = Scratch::new(&params, n, true);
					let (xbuf, nn, d) = Model::upload(&ds.x);
					assert_eq!(nn, n);
					let scaler = model.scaler.borrow();
					let xbuf = if let Some(sc_ref) = scaler.as_ref() {
						if sc_ref.mean.is_empty() { xbuf } else { Model::zscore_apply(&xbuf, n, d, sc_ref) }
					} else {
						xbuf
					};
					Model::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
					let ybuf = GpuBuffer::upload(ds.y.as_slice().expect("y contig")).expect("ybuf");
					if model.loss.is_classification() {
						if k == 1 {
							kernels::gpu_accuracy_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n);
						} else {
							kernels::gpu_argmax_accuracy_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n, k);
						}
						Model::download_scalar(&sc.metric_scalar)
					} else {
						let total = (n * k) as f64;
						let ybar = ds.y.iter().sum::<f64>() / total;
						let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
						kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n * k);
						1.0 - Model::download_scalar(&sc.metric_scalar) / ss_tot
					}
				}
			};
			let mut last = self.last.borrow_mut();
			last.model = model as *const Model;
			last.score = score;
			last.preds = None;
			last.n = ds.x.nrows();
			last.k = ds.n_targets.max(1);
			last.target_names = data.target_names();
			last.raw_test_rows = data.raw_rows();
			last.raw_test_headers = data.raw_headers();
		} else {
			let params = model.params.borrow();
			assert!(!params.is_empty(), "run: call train first");
			let last_layer = params.len() - 1;
			let k = params[last_layer].out_dim;
			let n = ds.x.nrows();
			let embed_first = matches!(model.specs.first(), Some(LayerSpec::Embed(_)));
			let cat_cols: Vec<usize> = if embed_first {
				(0..ds.x.ncols()).filter(|c| !ds.text_cols.contains(c)).collect()
			} else {
				Vec::new()
			};
			let xinput = if embed_first {
				ds.x.select(ndarray::Axis(1), &ds.text_cols)
			} else {
				ds.x.clone()
			};
			let (xraw, nn, d) = Model::upload(&xinput);
			assert_eq!(nn, n);
			let scaler = model.scaler.borrow();
			let scaler_ref = scaler.as_ref().expect("run infer: missing scaler; train first");
			let (xbuf, x_cat) = if embed_first {
				if cat_cols.is_empty() {
					(xraw, None)
				} else {
					let cat = ds.x.select(ndarray::Axis(1), &cat_cols);
					let (craw, _, c) = Model::upload(&cat);
					(xraw, Some(Model::zscore_apply(&craw, n, c, scaler_ref)))
				}
			} else {
				(Model::zscore_apply(&xraw, n, d, scaler_ref), None)
			};
			let sc = Scratch::new(&params, n, true);
			Model::forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
			let preds = Model::download_vec(&sc.acts[last_layer], n * k);
			let mut last = self.last.borrow_mut();
			last.model = model as *const Model;
			last.score = f64::NAN;
			last.preds = Some(preds);
			last.n = n;
			last.k = k;
			let tnames = data.target_names();
			if !tnames.is_empty() {
				last.target_names = tnames;
			}
			if let Some(rows) = data.raw_rows() {
				last.raw_test_rows = Some(rows);
			}
			if let Some(headers) = data.raw_headers() {
				last.raw_test_headers = Some(headers);
			}
		}
	}

	pub fn save<I, S>(&self, items: I, path: impl Into<String>)
	where
		I: IntoIterator<Item = S>,
		S: Into<SaveItem>,
	{
		let items: Vec<SaveItem> = items.into_iter().map(Into::into).collect();
		let path = path.into();
		let path = Self::resolve(&path);
		let last = self.last.borrow();
		assert!(!last.model.is_null(), "save: call run() first");
		let all_params = items.iter().all(|i| matches!(i, SaveItem::W | SaveItem::B));
		if all_params {
			let model = unsafe { &*last.model };
			let params = model.params.borrow();
			assert!(!params.is_empty(), "save: model has no trained params");
			let parts: Vec<Param> = items
				.iter()
				.map(|i| match i {
					SaveItem::W => Param::W,
					SaveItem::B => Param::B,
					_ => unreachable!(),
				})
				.collect();
			let key = model.loss.score_key();
			let score = last.score;
			if !score.is_finite()
				|| Model::saved_score(&path, key).is_some_and(|best| score <= best)
			{
				return;
			}
			let neurons: usize = params.iter().map(|p| p.out_dim).sum();
			Model::write_ogdl(&path, &Model::dump_ogdl(&params, &parts, key, score));
			let full = std::fs::canonicalize(&path).unwrap_or_else(|_| path.as_str().into());
			eprintln!("saved {} ({neurons} neurons, {key} {score:.4})", full.display());
		} else {
			let preds = last.preds.as_ref().expect("save columns: run inference first");
			let n = last.n;
			let k = last.k;
			let targets = &last.target_names;
			let headers_opt = last.raw_test_headers.as_ref();
			let rows_opt = last.raw_test_rows.as_ref();
			let mut csv_cols: Vec<(String, Vec<String>)> = Vec::new();
			for item in &items {
				match item {
					SaveItem::Col(name) => {
						if targets.contains(name) || (targets.len() > 1 && targets[0] == *name)
						{
							if k == 1 {
								let col: Vec<String> = (0..n).map(|i| preds[i].to_string()).collect();
								csv_cols.push((targets[0].clone(), col));
							} else {
								for (ti, tname) in targets.iter().enumerate() {
									let col: Vec<String> = (0..n)
										.map(|i| preds[i * k + ti].to_string())
										.collect();
									csv_cols.push((tname.clone(), col));
								}
							}
						} else if let (Some(headers), Some(rows)) = (headers_opt, rows_opt) {
							if let Some(ci) = headers.iter().position(|h| h == name) {
								let col: Vec<String> =
									rows.iter().map(|r| r.get(ci).cloned().unwrap_or_default()).collect();
								csv_cols.push((name.clone(), col));
							} else {
								panic!("save: column '{name}' not found in test data");
							}
						} else {
							panic!("save: no raw test data available for column '{name}'");
						}
					}
					SaveItem::W | SaveItem::B => {
						panic!("save: mixing params and columns is not supported");
					}
				}
			}
			assert!(!csv_cols.is_empty(), "save: no columns to write");
			let mut out = String::new();
			let header: Vec<&str> = csv_cols.iter().map(|(h, _)| h.as_str()).collect();
			out.push_str(&header.join(","));
			out.push('\n');
			for i in 0..n {
				let row: Vec<&str> = csv_cols.iter().map(|(_, col)| col[i].as_str()).collect();
				out.push_str(&row.join(","));
				out.push('\n');
			}
			std::fs::write(&path, &out).unwrap_or_else(|e| panic!("save: {path}: {e}"));
			let full = std::fs::canonicalize(&path).unwrap_or_else(|_| path.as_str().into());
			eprintln!("saved {} ({n} rows)", full.display());
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

/// What to log or plot each epoch: `.log(&[Loss, R2, Lr])`.
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

/// What a layer computes. `Dense`: z = act(X·W + b). `Embed`: each of the
/// `in_dim` input columns is a token id, looked up in the `w` table ([vocab×dim],
/// row-major) → output `in_dim×dim` wide (the flattened token-vector sequence).
#[derive(Clone, Copy, PartialEq)]
enum LayerKind {
	Dense,
	Embed,
	Attn,
}

/// One parsed OGDL block, in layer/neuron order — the resume counterpart of the
/// per-layer save format. `Embed` is the flat [vocab*dim] token table; `Attn` holds
/// the four [d*d] projections and their (zero) [d] biases; `Dense` is one neuron's
/// weight row, bias, and optional learned PReLU slope `a`.
#[derive(Debug, PartialEq)]
enum Saved {
	Embed(Vec<f64>),
	Attn {
		wq: Vec<f64>,
		wk: Vec<f64>,
		wv: Vec<f64>,
		wo: Vec<f64>,
		bq: Vec<f64>,
		bk: Vec<f64>,
		bv: Vec<f64>,
		bo: Vec<f64>,
	},
	Dense {
		w: Vec<f64>,
		b: f64,
		a: Option<f64>,
	},
}

impl Saved {
	/// Element count of this block (weights + biases), for the NaN-fraction report.
	fn len(&self) -> usize {
		match self {
			Saved::Embed(t) => t.len(),
			Saved::Attn {
				wq,
				wk,
				wv,
				wo,
				bq,
				bk,
				bv,
				bo,
			} => {
				wq.len()
					+ wk.len() + wv.len() + wo.len()
					+ bq.len() + bk.len() + bv.len()
					+ bo.len()
			}
			Saved::Dense { w, .. } => w.len() + 1,
		}
	}
}

/// Sinusoidal positional encoding table [seq*dim], row-major: PE[s,2i]=sin(s/10000^(2i/dim)),
/// PE[s,2i+1]=cos(...). `negate` returns -PE (so a broadcast-SUB adds it). Built on host
/// once (no GPU PE kernel); added per row in the embed forward.
fn sinusoidal_pe(seq: usize, dim: usize, negate: bool) -> Vec<f64> {
	let sign = if negate { -1.0 } else { 1.0 };
	let mut pe = vec![0.0f64; seq * dim];
	for s in 0..seq {
		for j in 0..dim {
			let i2 = (j / 2) * 2;
			let freq = 1.0 / 10000f64.powf(i2 as f64 / dim as f64);
			let ang = s as f64 * freq;
			pe[s * dim + j] = sign * if j % 2 == 0 { ang.sin() } else { ang.cos() };
		}
	}
	pe
}

struct LayerParams {
	kind: LayerKind,
	// Dense: weight [in_dim×out_dim]. Embed: token table [vocab×dim]. Attn: Wq [d×d].
	w: GpuBuffer,
	// Dense: bias [out_dim]. Embed: negated positional encoding [in_dim*dim]. Attn: zero bias [d].
	b: GpuBuffer,
	in_dim: usize,
	out_dim: usize,
	act: Activation,
	// Embed: embedding width / table rows. Attn: model dim d (per token) / heads.
	dim: usize,
	vocab: usize,
	// Attn only: K/V/output projections [d×d] each, and head count (else dummy len-1 / 0).
	wk: GpuBuffer,
	wv: GpuBuffer,
	wo: GpuBuffer,
	heads: usize,
	// PRelu only: the learnable negative slope (a single [1] scalar, SGD-updated).
	// Dummy len-1 for every other activation.
	palpha: GpuBuffer,
}

/// If the network is an embed/attn text prefix followed by a dense head, return
/// `(first_dense_index, attn_out_dim A, categorical_dim C)` — the dense at that
/// index reads `concat(prefix_output[A], x_cat[C])`. None when there's no prefix
/// or no extra categorical features (C==0, e.g. all columns are text).
fn concat_layer(params: &[LayerParams]) -> Option<(usize, usize, usize)> {
	for l in 1..params.len() {
		let prev = &params[l - 1];
		if params[l].kind == LayerKind::Dense
			&& matches!(prev.kind, LayerKind::Embed | LayerKind::Attn)
		{
			let a = prev.out_dim;
			let c = params[l].in_dim.saturating_sub(a);
			return (c > 0).then_some((l, a, c));
		}
	}
	None
}

struct Scratch {
	acts: Vec<GpuBuffer>,
	// Per-layer pre-activation, saved ONLY for Silu/Gelu (their backward needs the
	// input z, which the in-place activation would otherwise overwrite). Len-1
	// dummy for every other layer.
	preact: Vec<GpuBuffer>,
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
	// Embed layers accumulate the table gradient here ([vocab×dim]) before the
	// SGD step — scatter-add target, separate from the table so the update is
	// `table -= lr·grad`. Len 1 when there's no embed layer.
	embed_grad: GpuBuffer,
	// Attention scratch (len 1 when there's no attn layer). q/k/v/ctx are the
	// projected sequences [n*S*d]; scores [n*heads*S*S]; the d* mirrors hold the
	// backward gradients; gw is a [d*d] weight-grad temp reused per projection.
	a_q: GpuBuffer,
	a_k: GpuBuffer,
	a_v: GpuBuffer,
	a_ctx: GpuBuffer,
	a_scores: GpuBuffer,
	a_dctx: GpuBuffer,
	a_dq: GpuBuffer,
	a_dk: GpuBuffer,
	a_dv: GpuBuffer,
	a_dscores: GpuBuffer,
	a_gw: GpuBuffer,
	a_dbias: GpuBuffer,
	// PRelu d_alpha scratch (act-sized temps + a scalar accumulator). Len-1 when
	// no PRelu layer exists.
	prelu_t0: GpuBuffer,
	prelu_t1: GpuBuffer,
	prelu_scalar: GpuBuffer,
	// Two-branch concat: `concat` [n×(A+C)] holds [attn_output | categorical] fed to
	// the first dense layer; `concat_dgrad` [n×A] compacts that dense's input-grad
	// back to the attention width on the backward pass. Len-1 when no concat exists.
	concat: GpuBuffer,
	concat_dgrad: GpuBuffer,
	copy_stream: gpu_core::hip::Stream,
	pinned_scalar: *mut f64,
}

impl Scratch {
	/// `forward_only` (eval/predict) sizes every BACKWARD-only buffer to len-1 —
	/// they're never read in a forward pass — so inference allocates ~half the VRAM
	/// of training (no second `a_dscores`, no `da`/`dw`/grad mirrors).
	fn new(params: &[LayerParams], n: usize, forward_only: bool) -> Scratch {
		let bw = |sz: usize| if forward_only { 1 } else { sz };
		// On OOM, report the buffer name and the size it tried to grab (f64 count →
		// bytes) instead of a bare HipError(2) — full-batch attention scores dominate.
		let alloc = |sz: usize, label: &str| -> GpuBuffer {
			GpuBuffer::alloc(sz).unwrap_or_else(|e| {
				panic!(
					"{label}: GPU alloc of {} ({sz} × f64) failed — {e:?}",
					crate::data::human_bytes(sz * 8)
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
		let mut has_prelu = false;
		for p in params {
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
			let wt = p.in_dim * p.out_dim;
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
		Scratch {
			acts,
			preact,
			da_a: alloc(bw(max_act), "da_a"),
			da_b: alloc(bw(max_act), "da_b"),
			dz: alloc(bw(max_act), "dz"),
			dw: alloc(bw(max_wt), "dw"),
			db: alloc(bw(max_bias), "db"),
			metric_t0: alloc(out_elems, "metric_t0"),
			metric_t1: alloc(out_elems, "metric_t1"),
			metric_t2: alloc(out_elems, "metric_t2"),
			metric_scalar: alloc(1, "metric_scalar"),
			reduce_ws: GpuBuffer::alloc_bytes(max_ws).unwrap_or_else(|e| {
				panic!(
					"reduce_ws: GPU alloc of {} failed — {e:?}",
					crate::data::human_bytes(max_ws)
				)
			}),
			embed_grad: alloc(bw(max_embed_grad), "embed_grad"),
			a_q: alloc(max_seqd, "a_q"),
			a_k: alloc(max_seqd, "a_k"),
			a_v: alloc(max_seqd, "a_v"),
			a_ctx: alloc(max_seqd, "a_ctx"),
			a_scores: alloc(max_scores, "a_scores"),
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
			copy_stream: gpu_core::hip::Stream::new().expect("copy stream"),
			pinned_scalar: {
				let ptr = gpu_core::hip::host_malloc(8, 0).expect("pinned scalar");
				ptr as *mut f64
			},
		}
	}

	fn download_scalar_deferred(&self) {
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

	fn sync_deferred_scalar(&self) -> f64 {
		self.copy_stream.synchronize().expect("sync copy stream");
		unsafe { *self.pinned_scalar }
	}
}

impl Drop for Scratch {
	fn drop(&mut self) {
		if !self.pinned_scalar.is_null() {
			let _ = unsafe { gpu_core::hip::hipHostFree(self.pinned_scalar as *mut std::ffi::c_void) };
		}
	}
}

impl Scratch {
	/// Exact bytes `new()` will allocate for these params at row count `n` — the
	/// SUM of every buffer, mirroring `new()` field-for-field. Used to pre-check a
	/// forward pass (esp. eval, where attention's scores + per-head buffers are
	/// huge) against free VRAM, since an over-budget alloc HIP-asserts (core dump)
	/// rather than returning a catchable error.
	fn vram_bytes(params: &[LayerParams], n: usize, forward_only: bool) -> usize {
		let bw = |sz: usize| if forward_only { 1 } else { sz };
		let mut max_ws = kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1);
		let (mut max_act, mut max_wt, mut max_bias) = (0usize, 0usize, 0usize);
		let (mut max_embed_grad, mut max_seqd, mut max_scores, mut max_dd) =
			(1usize, 1usize, 1usize, 1usize);
		let mut has_prelu = false;
		let mut floats = 0usize; // acts + preact (per-layer, variable)
		for p in params {
			floats += n * p.out_dim; // acts[l]
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
			if p.in_dim * p.out_dim > max_wt {
				max_wt = p.in_dim * p.out_dim;
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
		floats += bw(max_wt) + bw(max_bias); // dw, db
		floats += 3 * out_elems + 1; // metric_t0/t1/t2, metric_scalar
		floats += bw(max_embed_grad); // embed_grad
		floats += 4 * max_seqd; // a_q,a_k,a_v,a_ctx (forward)
		floats += 4 * bw(max_seqd); // a_dctx,a_dq,a_dk,a_dv (backward)
		floats += max_scores + bw(max_scores); // a_scores (fwd), a_dscores (bwd)
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

/// Per-feature standardizer fit on the train set, reused verbatim on eval so
/// train and eval see the same scaling (no leakage, no drift).
struct Scaler {
	mean: Vec<f64>,
	std: Vec<f64>,
}

/// Neural network architecture: layers, loss, and learning rate.
///
/// ```rust,no_run
/// # use nates_recipe::*;
/// Model::new()
///     .layer(embed(16))
///     .layer(attn(4))
///     .layer(64).relu()
///     .layer(1)
///     .loss(mse)
///     .lr(0.001);
/// ```
pub struct Model {
	specs: Vec<LayerSpec>,
	loss: Loss,
	lr: f64,
	params: RefCell<Vec<LayerParams>>,
	scaler: RefCell<Option<Scaler>>,
}

impl Model {
	pub fn new() -> Model {
		Model {
			specs: Vec::new(),
			loss: Loss::Mse,
			lr: 0.01,
			params: RefCell::new(Vec::new()),
			scaler: RefCell::new(None),
		}
	}

	/// dL/dA at the output for the chosen loss, scaled by 1/n (batch mean),
	/// written in place into `da` with no allocation. `out` = predictions,
	/// `y` = targets, `total` = n*out_dim. Equals the old allocate-return
	/// `loss_grad` followed by `·(1/n)`, op-for-op.
	fn loss_grad_into(
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

	/// One metric this epoch as a single GPU-reduced scalar, downloading only that
	/// scalar (never the n predictions). `out` = output activations (n×1, on GPU);
	/// `ss_tot` is precomputed once since the targets are fixed. R²/MSE/accuracy go
	/// through fused single-pass kernels (`gpu_ss_res_into`/`gpu_mse_into`/
	/// `gpu_accuracy_into`); MAE/Huber/CE go through `_into` variants writing into the
	/// preallocated `sc.metric_t*` temporaries — so the whole path allocates nothing.
	/// Matches `metric_num` exactly except accuracy differs only at the measure-zero
	/// p==0.5 tie (sigmoid outputs never land there).
	fn metric_gpu(
		&self,
		m: Metric,
		out: &GpuBuffer,
		ybuf: &GpuBuffer,
		sc: &Scratch,
		n: usize,
		k: usize,
		ss_tot: f64,
		epoch: usize,
		elapsed: f64,
	) -> f64 {
		let nk = n * k; // element count: n samples × k outputs, flat row-major
		match m {
			Metric::Epoch => epoch as f64,
			Metric::Lr => self.lr,
			Metric::Time => elapsed,
			Metric::R2 => {
				kernels::gpu_ss_res_into(out, ybuf, &sc.metric_scalar, nk);
				1.0 - Self::download_scalar(&sc.metric_scalar) / ss_tot
			}
			Metric::Accuracy => {
				if k == 1 {
					kernels::gpu_accuracy_into(out, ybuf, &sc.metric_scalar, n);
				} else {
					kernels::gpu_argmax_accuracy_into(out, ybuf, &sc.metric_scalar, n, k);
				}
				Self::download_scalar(&sc.metric_scalar)
			}
			// The Loss metric is the model's ACTUAL loss (self.loss), not hardcoded.
			Metric::Loss => {
				let nf = nk as f64;
				match self.loss {
					Loss::Mse => {
						kernels::gpu_mse_into(out, ybuf, &sc.metric_scalar, nk);
						Self::download_scalar(&sc.metric_scalar)
					}
					Loss::Mae => {
						kernels::gpu_sub_scale_into(out, ybuf, &sc.metric_t0, nk, 1.0);
						kernels::gpu_abs_into(&sc.metric_t0, &sc.metric_t0, nk);
						kernels::gpu_reduce_sum_cols_into(
							&sc.metric_t0,
							&sc.metric_scalar,
							&sc.reduce_ws,
							nk,
							1,
						);
						Self::download_scalar(&sc.metric_scalar) / nf
					}
					Loss::Huber => {
						// delta=1: 0.5 r² for |r|≤1 else |r|-0.5, written as
						// 0.5·clamp(r,-1,1)² + |r| - |clamp(r,-1,1)|.
						kernels::gpu_sub_scale_into(out, ybuf, &sc.metric_t0, nk, 1.0); // r
						kernels::gpu_clamp_into(
							&sc.metric_t0,
							&sc.metric_t1,
							nk,
							-1.0,
							1.0,
						); // rc
						kernels::gpu_copy_into(&sc.metric_t1, &sc.metric_t2, nk); // e = rc
						kernels::gpu_mul_inplace(&sc.metric_t2, &sc.metric_t1, nk); // e = rc²
						kernels::gpu_scale_inplace(&sc.metric_t2, 0.5, nk); // e = 0.5 rc²
						kernels::gpu_abs_into(&sc.metric_t0, &sc.metric_t0, nk); // |r|
						kernels::gpu_add_inplace(&sc.metric_t2, &sc.metric_t0, nk); // e += |r|
						kernels::gpu_abs_into(&sc.metric_t1, &sc.metric_t1, nk); // |rc|
						kernels::gpu_sub_inplace(&sc.metric_t2, &sc.metric_t1, nk); // e -= |rc|
						kernels::gpu_reduce_sum_cols_into(
							&sc.metric_t2,
							&sc.metric_scalar,
							&sc.reduce_ws,
							nk,
							1,
						);
						Self::download_scalar(&sc.metric_scalar) / nf
					}
					Loss::Ce => {
						// Categorical CE: p = softmax(logits); −Σ y·ln(p) / n. y is
						// one-hot so only the true class contributes per sample.
						let eps = 1e-7;
						kernels::gpu_softmax_rows_into(out, &sc.metric_t0, n, k); // p
						kernels::gpu_clamp_into(
							&sc.metric_t0,
							&sc.metric_t0,
							nk,
							eps,
							1.0,
						); // avoid ln(0)
						kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t0, nk); // ln p
						kernels::gpu_mul_inplace(&sc.metric_t0, ybuf, nk); // y·ln p
						kernels::gpu_reduce_sum_cols_into(
							&sc.metric_t0,
							&sc.metric_scalar,
							&sc.reduce_ws,
							nk,
							1,
						);
						-Self::download_scalar(&sc.metric_scalar) / n as f64
					}
					Loss::Bce => {
						let eps = 1e-7;
						kernels::gpu_clamp_into(out, &sc.metric_t0, nk, eps, 1.0 - eps); // pc
						kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t1, nk); // ln pc
						kernels::gpu_mul_inplace(&sc.metric_t1, ybuf, nk); // y·ln pc
						kernels::gpu_scale_inplace(&sc.metric_t0, -1.0, nk); // -pc
						kernels::gpu_add_scalar_inplace(&sc.metric_t0, 1.0, nk); // 1-pc
						kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t0, nk); // ln(1-pc)
						kernels::gpu_copy_into(ybuf, &sc.metric_t2, nk); // y
						kernels::gpu_scale_inplace(&sc.metric_t2, -1.0, nk); // -y
						kernels::gpu_add_scalar_inplace(&sc.metric_t2, 1.0, nk); // 1-y
						kernels::gpu_mul_inplace(&sc.metric_t2, &sc.metric_t0, nk); // (1-y)·ln(1-pc)
						kernels::gpu_add_inplace(&sc.metric_t1, &sc.metric_t2, nk); // sum terms
						kernels::gpu_reduce_sum_cols_into(
							&sc.metric_t1,
							&sc.metric_scalar,
							&sc.reduce_ws,
							nk,
							1,
						);
						-Self::download_scalar(&sc.metric_scalar) / nf
					}
				}
			}
		}
	}

	fn metric_gpu_into(
		&self,
		m: Metric,
		out: &GpuBuffer,
		ybuf: &GpuBuffer,
		sc: &Scratch,
		n: usize,
		k: usize,
		ss_tot: f64,
	) -> (f64, f64) {
		let nk = n * k;
		match m {
			Metric::Loss => {
				match self.loss {
					Loss::Mse => {
						kernels::gpu_mse_into(out, ybuf, &sc.metric_scalar, nk);
						(1.0, 1.0)
					}
					Loss::Mae => {
						kernels::gpu_sub_scale_into(out, ybuf, &sc.metric_t0, nk, 1.0);
						kernels::gpu_abs_into(&sc.metric_t0, &sc.metric_t0, nk);
						kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.metric_scalar, &sc.reduce_ws, nk, 1);
						(1.0, nk as f64)
					}
					Loss::Huber => {
						kernels::gpu_sub_scale_into(out, ybuf, &sc.metric_t0, nk, 1.0);
						kernels::gpu_clamp_into(&sc.metric_t0, &sc.metric_t1, nk, -1.0, 1.0);
						kernels::gpu_copy_into(&sc.metric_t1, &sc.metric_t2, nk);
						kernels::gpu_mul_inplace(&sc.metric_t2, &sc.metric_t1, nk);
						kernels::gpu_scale_inplace(&sc.metric_t2, 0.5, nk);
						kernels::gpu_abs_into(&sc.metric_t0, &sc.metric_t0, nk);
						kernels::gpu_add_inplace(&sc.metric_t2, &sc.metric_t0, nk);
						kernels::gpu_abs_into(&sc.metric_t1, &sc.metric_t1, nk);
						kernels::gpu_sub_inplace(&sc.metric_t2, &sc.metric_t1, nk);
						kernels::gpu_reduce_sum_cols_into(&sc.metric_t2, &sc.metric_scalar, &sc.reduce_ws, nk, 1);
						(1.0, nk as f64)
					}
					Loss::Ce => {
						let eps = 1e-7;
						kernels::gpu_softmax_rows_into(out, &sc.metric_t0, n, k);
						kernels::gpu_clamp_into(&sc.metric_t0, &sc.metric_t0, nk, eps, 1.0);
						kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t0, nk);
						kernels::gpu_mul_inplace(&sc.metric_t0, ybuf, nk);
						kernels::gpu_reduce_sum_cols_into(&sc.metric_t0, &sc.metric_scalar, &sc.reduce_ws, nk, 1);
						(-1.0, n as f64)
					}
					Loss::Bce => {
						let eps = 1e-7;
						kernels::gpu_clamp_into(out, &sc.metric_t0, nk, eps, 1.0 - eps);
						kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t1, nk);
						kernels::gpu_mul_inplace(&sc.metric_t1, ybuf, nk);
						kernels::gpu_scale_inplace(&sc.metric_t0, -1.0, nk);
						kernels::gpu_add_scalar_inplace(&sc.metric_t0, 1.0, nk);
						kernels::gpu_log_into(&sc.metric_t0, &sc.metric_t0, nk);
						kernels::gpu_copy_into(ybuf, &sc.metric_t2, nk);
						kernels::gpu_scale_inplace(&sc.metric_t2, -1.0, nk);
						kernels::gpu_add_scalar_inplace(&sc.metric_t2, 1.0, nk);
						kernels::gpu_mul_inplace(&sc.metric_t2, &sc.metric_t0, nk);
						kernels::gpu_add_inplace(&sc.metric_t1, &sc.metric_t2, nk);
						kernels::gpu_reduce_sum_cols_into(&sc.metric_t1, &sc.metric_scalar, &sc.reduce_ws, nk, 1);
						(-1.0, nk as f64)
					}
				}
			}
			Metric::R2 => {
				kernels::gpu_ss_res_into(out, ybuf, &sc.metric_scalar, nk);
				(1.0, ss_tot)
			}
			_ => (1.0, 1.0),
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

	pub fn layer(mut self, spec: impl IntoLayer) -> Model {
		self.specs.push(spec.into_layer());
		self
	}

	fn set_last_activation(mut self, act: Activation) -> Model {
		if let Some(LayerSpec::Dense(_, a)) = self.specs.last_mut() {
			*a = act;
		} else {
			panic!("activation method called but last layer is not dense");
		}
		self
	}

	pub fn relu(self) -> Model {
		self.set_last_activation(Activation::Relu)
	}
	pub fn leak(self) -> Model {
		self.set_last_activation(Activation::LeakyRelu)
	}
	pub fn sigmoid(self) -> Model {
		self.set_last_activation(Activation::Sigmoid)
	}
	pub fn tanh(self) -> Model {
		self.set_last_activation(Activation::Tanh)
	}
	pub fn selu(self) -> Model {
		self.set_last_activation(Activation::Selu)
	}
	pub fn gelu(self) -> Model {
		self.set_last_activation(Activation::Gelu)
	}
	pub fn silu(self) -> Model {
		self.set_last_activation(Activation::Silu)
	}
	pub fn elu(self) -> Model {
		self.set_last_activation(Activation::Elu)
	}
	pub fn prelu(self) -> Model {
		self.set_last_activation(Activation::PRelu)
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

	/// Per-column z-score fit on the TRAIN set; store mean/std in `scaler` (reused
	/// verbatim at eval, no leakage) and return the scaled [n×d] buffer.
	fn zscore_fit(
		xraw: &GpuBuffer,
		n: usize,
		d: usize,
		scaler: &RefCell<Option<Scaler>>,
	) -> GpuBuffer {
		let mean = kernels::gpu_reduce_mean_cols(xraw, n, d).expect("mean");
		let var = kernels::gpu_reduce_var_cols(xraw, n, d).expect("var");
		kernels::gpu_add_scalar_inplace(&var, 1e-8, d);
		let std = kernels::gpu_sqrt(&var, d).expect("std");
		let xc = kernels::gpu_broadcast_sub(xraw, &mean, n * d, d).expect("center");
		let xbuf = kernels::gpu_broadcast_div(&xc, &std, n * d, d).expect("scale");
		*scaler.borrow_mut() = Some(Scaler {
			mean: Self::download_vec(&mean, d),
			std: Self::download_vec(&std, d),
		});
		xbuf
	}

	/// Apply a fitted scaler to eval features (same mean/std the train set saw).
	fn zscore_apply(xraw: &GpuBuffer, n: usize, d: usize, scaler: &Scaler) -> GpuBuffer {
		assert_eq!(scaler.mean.len(), d, "eval: feature count changed");
		assert_eq!(scaler.std.len(), d, "eval: feature count changed");
		let mean = GpuBuffer::upload(&scaler.mean).expect("upload eval mean");
		let std = GpuBuffer::upload(&scaler.std).expect("upload eval std");
		let xc = kernels::gpu_broadcast_sub(xraw, &mean, n * d, d).expect("eval center");
		kernels::gpu_broadcast_div(&xc, &std, n * d, d).expect("eval scale")
	}

	/// Forward pass writing each layer's output into the preallocated `acts`
	/// (no allocation). The input `x` feeds layer 0 directly (no copy); the
	/// activation is applied in place (`acts[l]` holds the pre-activation, then
	/// is overwritten by its own activation). `acts[last]` ends as predictions.
	fn forward_into(
		params: &[LayerParams],
		x: &GpuBuffer,
		x_cat: Option<&GpuBuffer>,
		n: usize,
		acts: &[GpuBuffer],
		sc: &Scratch,
	) {
		let cc = concat_layer(params);
		for (l, p) in params.iter().enumerate() {
			// The first dense after the text prefix reads concat(attn_out, x_cat):
			// build [n×(A+C)] = [acts[l-1] | x_cat] and feed THAT instead of acts[l-1].
			if let Some((pf, a, c)) = cc
				&& l == pf
			{
				kernels::gpu_concat_into(
					&acts[l - 1],
					x_cat.expect("concat: x_cat missing"),
					&sc.concat,
					n,
					a,
					c,
				);
			}
			let prev = if l == 0 {
				x
			} else if Some(l) == cc.map(|t| t.0) {
				&sc.concat
			} else {
				&acts[l - 1]
			};
			match p.kind {
				LayerKind::Embed => {
					// Each of the in_dim input columns is a token id; gather its
					// dim-vector from the table → [n, in_dim*dim]. Then add the
					// positional encoding (b = -PE [out_dim], so broadcast-SUB adds it).
					kernels::gpu_gather_rows_into(
						&p.w,
						prev,
						&acts[l],
						n * p.in_dim,
						p.dim,
					);
					kernels::gpu_broadcast_sub_into(
						&acts[l],
						&p.b,
						&acts[l],
						n * p.out_dim,
						p.out_dim,
					);
				}
				LayerKind::Attn => Self::attn_forward(p, prev, &acts[l], n, sc),
				LayerKind::Dense => {
					// out_dim==1: z = X@w + b is a matrix-vector product. rocBLAS dispatches
					// a full GEMM tile (32×32) for one output column, wasting 31/32 of it —
					// dgemv reads the same operands once and is memory-bound, ~33× faster.
					if p.out_dim == 1 {
						kernels::gpu_matvec_bias_into(
							prev, &p.w, &p.b, &acts[l], n, p.in_dim,
						);
					} else {
						kernels::gpu_linear_into(
							prev, &p.w, &p.b, &acts[l], n, p.out_dim, p.in_dim,
						);
					}
					let m = n * p.out_dim;
					// Elu/Selu/Silu/Gelu/PRelu backprop from the pre-activation z —
					// save it before the in-place activation overwrites acts[l].
					if matches!(
						p.act,
						Activation::Silu
							| Activation::Gelu | Activation::Elu
							| Activation::Selu | Activation::PRelu
					) {
						kernels::gpu_copy_into(&acts[l], &sc.preact[l], m);
					}
					match p.act {
						Activation::Relu => {
							kernels::gpu_relu_into(&acts[l], &acts[l], m)
						}
						Activation::Sigmoid => {
							kernels::gpu_sigmoid_into(&acts[l], &acts[l], m)
						}
						Activation::LeakyRelu => kernels::gpu_leaky_relu_into(
							&acts[l],
							&acts[l],
							m,
							LEAKY_ALPHA,
						),
						Activation::PRelu => {
							let a = Self::download_scalar(&p.palpha);
							kernels::gpu_leaky_relu_into(
								&sc.preact[l],
								&acts[l],
								m,
								a,
							);
						}
						Activation::Elu => gpu_core::k_gapact::gpu_elu_into(
							&sc.preact[l],
							&acts[l],
							m,
							ELU_ALPHA,
						),
						Activation::Selu => gpu_core::k_gapact::gpu_selu_into(
							&sc.preact[l],
							&acts[l],
							m,
						),
						Activation::Tanh => {
							kernels::gpu_tanh_into(&acts[l], &acts[l], m)
						}
						Activation::Silu => {
							kernels::gpu_silu_into(&sc.preact[l], &acts[l], m)
						}
						Activation::Gelu => {
							kernels::gpu_gelu_into(&sc.preact[l], &acts[l], m)
						}
						Activation::Linear => {}
					}
				}
			}
		}
	}

	/// Bare multi-head self-attention forward. Input `h` = [n, S*d] (d = p.dim,
	/// S = in_dim/d, heads = p.heads, hd = d/heads). Q/K/V = H·{Wq,Wk,Wv}; per
	/// head scores = Q·Kᵀ/√hd → softmax → context = scores·V; out = context·Wo.
	/// Q/K/V/scores/context land in `sc.a_*` for the backward pass. Alloc-free.
	fn attn_forward(p: &LayerParams, h: &GpuBuffer, out: &GpuBuffer, n: usize, sc: &Scratch) {
		let d = p.dim;
		let heads = p.heads;
		let hd = d / heads;
		let s = p.in_dim / d;
		let m = n * s;
		kernels::gpu_linear_into(h, &p.w, &p.b, &sc.a_q, m, d, d);
		kernels::gpu_linear_into(h, &p.wk, &p.b, &sc.a_k, m, d, d);
		kernels::gpu_linear_into(h, &p.wv, &p.b, &sc.a_v, m, d, d);
		// scores[head] = Q_head · K_headᵀ  (per-head sub-matrix views, ld = d)
		for hh in 0..heads {
			gpu_core::linalg::gpu_bmm_into(
				&sc.a_scores,
				&sc.a_q,
				&sc.a_k,
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
		}
		kernels::gpu_scale_inplace(&sc.a_scores, 1.0 / (hd as f64).sqrt(), n * heads * s * s);
		kernels::gpu_softmax_rows_into(&sc.a_scores, &sc.a_scores, n * heads * s, s);
		// context[head] = scores[head] · V_head
		for hh in 0..heads {
			gpu_core::linalg::gpu_bmm_into(
				&sc.a_ctx,
				&sc.a_scores,
				&sc.a_v,
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
		}
		kernels::gpu_linear_into(&sc.a_ctx, &p.wo, &p.b, out, m, d, d);
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
	fn backward_step(
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
					let a = Self::download_scalar(&params[l].palpha);
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

	fn fit(&self, data: &Dataset, cfg: &Train, resume: Option<&str>) {
		assert!(
			!self.specs.is_empty(),
			"model has no layers — call .layer() before .fit()"
		);
		let rerun = !self.params.borrow().is_empty();
		// An embed first layer reads its input columns as token IDS — it consumes
		// ONLY the text token-id columns (the rest are numeric/one-hot, not ids and
		// would explode the vocab). Build the model input from those columns; they
		// pass through raw (no z-score) and the table is sized to their id range.
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(_)));
		assert!(
			!embed_first || !data.text_cols.is_empty(),
			"embed layer but no text columns in the data",
		);
		// Two branches: text token-id columns drive embed→attn; every other column
		// (categorical/numeric) skips the prefix and is concatenated onto the
		// attention output before the dense head. xinput feeds layer 0 (the text
		// branch when embed-first, else the whole matrix).
		let cat_cols: Vec<usize> = if embed_first {
			(0..data.x.ncols())
				.filter(|c| !data.text_cols.contains(c))
				.collect()
		} else {
			Vec::new()
		};
		let c_cat = cat_cols.len();
		let xinput = if embed_first {
			data.x.select(ndarray::Axis(1), &data.text_cols)
		} else {
			data.x.clone()
		};
		let vocab = if embed_first {
			xinput.iter().cloned().fold(0.0f64, f64::max) as usize + 1
		} else {
			0
		};
		// No VRAM pre-check: any estimate that blocks a valid run is worse than
		// none. If a buffer doesn't fit, hipMallocAsync errors at the real size.
		let start = std::time::Instant::now();
		let (xraw, n, d) = Self::upload(&xinput);
		// Text token ids pass RAW to the embed lookup (no z-score). The categorical
		// branch IS z-scored on the train set (raw frequency-encoded columns span
		// wildly different magnitudes; unscaled they saturate the dense head). For a
		// non-embed model the whole matrix is the categorical branch. The scaler is
		// fit once here and reused verbatim on eval (no leakage).
		let (xbuf, x_cat) = if embed_first {
			if cat_cols.is_empty() {
				*self.scaler.borrow_mut() = Some(Scaler {
					mean: vec![],
					std: vec![],
				});
				(xraw, None)
			} else {
				let cat = data.x.select(ndarray::Axis(1), &cat_cols);
				let (craw, _, c) = Self::upload(&cat);
				let ccat = Self::zscore_fit(&craw, n, c, &self.scaler);
				(xraw, Some(ccat))
			}
		} else {
			(Self::zscore_fit(&xraw, n, d, &self.scaler), None)
		};
		let ybuf = GpuBuffer::upload(data.y.as_slice().expect("train: y contiguous"))
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
		// weights (y) or abort (n). build_params(false) re-runs construction with random
		// init, so "overwrite" is a clean fresh start the next save writes over the stale file.
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
			eprint!("        overwrite checkpoint with random weights? [y/N] ");
			std::io::stderr().flush().ok();
			let mut line = String::new();
			std::io::stdin().read_line(&mut line).ok();
			matches!(line.trim(), "y" | "Y" | "yes" | "YES")
		};
		// Build every layer's params, consuming parsed checkpoint blocks when try_resume.
		// A shape/order mismatch returns Err(reason) (not abort) so the caller can prompt.
		// `si` indexes blocks: one per embed/attn layer, one per dense neuron, in order.
		let build_params = |try_resume: bool| -> Result<Vec<LayerParams>, String> {
			let mut si = 0usize;
			let mut params: Vec<LayerParams> = Vec::new();
			let mut in_dim = d;
			let dummy = || GpuBuffer::upload(&[0.0f64]).expect("dummy buf");
			for (li, spec) in self.specs.iter().enumerate() {
				if let LayerSpec::Embed(dim) = *spec {
					// Token table [vocab×dim]. On resume, upload the saved table;
					// else small-random init (embeddings want O(0.1) scale, not He).
					// in_dim columns of token ids → in_dim*dim wide output. `b` holds
					// the NEGATED sinusoidal positional encoding [in_dim*dim], always
					// recomputed (deterministic, never saved). No activation.
					let table = if try_resume {
						let t = match resumed.get(si) {
							Some(Saved::Embed(t)) => t,
							_ => {
								return Err(format!(
									"layer {li}: checkpoint has no embed block here"
								));
							}
						};
						if t.len() != vocab * dim {
							return Err(format!(
								"layer {li} embed: checkpoint table has {} values, model needs {} (vocab {vocab} × dim {dim})",
								t.len(),
								vocab * dim
							));
						}
						si += 1;
						GpuBuffer::upload(t).expect("upload embed table")
					} else {
						let table = kernels::gpu_randn(
							vocab * dim,
							4242 + (li as u32) * 7919,
						)
						.expect("randn embed");
						kernels::gpu_scale_inplace(&table, 0.1, vocab * dim);
						table
					};
					let neg_pe = sinusoidal_pe(in_dim, dim, true);
					let b = GpuBuffer::upload(&neg_pe).expect("upload pe");
					params.push(LayerParams {
						kind: LayerKind::Embed,
						w: table,
						b,
						in_dim,
						out_dim: in_dim * dim,
						act: Activation::Linear,
						dim,
						vocab,
						wk: dummy(),
						wv: dummy(),
						wo: dummy(),
						heads: 0,
						palpha: dummy(),
					});
					in_dim *= dim;
					continue;
				}
				if let LayerSpec::Attn(heads) = *spec {
					// Bare multi-head self-attention; input is [n, S*d] with d = in_dim/S.
					// d (the per-token width) = the previous embed dim. We recover it from
					// the embed layer: in_dim here = S*d, and d = embed dim. heads | d.
					let d_tok = params.last().map_or(in_dim, |p| {
						if p.kind == LayerKind::Embed {
							p.dim
						} else {
							p.out_dim
						}
					});
					assert!(
						in_dim % d_tok == 0,
						"attn: input {in_dim} not a multiple of token dim {d_tok}"
					);
					assert!(
						d_tok.is_multiple_of(heads),
						"attn: token dim {d_tok} not divisible by {heads} heads"
					);
					let need = d_tok * d_tok;
					let (w, wk, wv, wo) = if try_resume {
						let (sq, sk, sv, so) = match resumed.get(si) {
							Some(Saved::Attn { wq, wk, wv, wo, .. }) => {
								(wq, wk, wv, wo)
							}
							_ => {
								return Err(format!(
									"layer {li}: checkpoint has no attn block here"
								));
							}
						};
						for (nm, v) in [("wq", sq), ("wk", sk), ("wv", sv), ("wo", so)]
						{
							if v.len() != need {
								return Err(format!(
									"layer {li} attn {nm}: checkpoint has {} values, model needs {need} (token dim {d_tok}²)",
									v.len()
								));
							}
						}
						si += 1;
						(
							GpuBuffer::upload(sq).expect("upload wq"),
							GpuBuffer::upload(sk).expect("upload wk"),
							GpuBuffer::upload(sv).expect("upload wv"),
							GpuBuffer::upload(so).expect("upload wo"),
						)
					} else {
						let mk = |seed: u32| {
							let w =
								kernels::gpu_randn(need, seed).expect("randn attn");
							kernels::gpu_scale_inplace(
								&w,
								(1.0 / d_tok as f64).sqrt(),
								need,
							);
							w
						};
						(
							mk(7001 + li as u32 * 13),
							mk(7002 + li as u32 * 13),
							mk(7003 + li as u32 * 13),
							mk(7004 + li as u32 * 13),
						)
					};
					params.push(LayerParams {
						kind: LayerKind::Attn,
						w,
						b: GpuBuffer::upload(&vec![0.0f64; d_tok]).expect("attn bias"),
						in_dim,
						out_dim: in_dim,
						act: Activation::Linear,
						dim: d_tok,
						vocab: 0,
						wk,
						wv,
						wo,
						heads,
						palpha: dummy(),
					});
					continue;
				}
				let (units, act) = match *spec {
					LayerSpec::Dense(u, a) => (u, a),
					_ => unreachable!(),
				};
				// First dense after the embed/attn prefix: its input is
				// concat(prefix_out, x_cat), so widen in_dim by the categorical
				// count exactly once (fires only when the prior layer is prefix).
				if c_cat > 0
					&& matches!(
						params.last().map(|p| p.kind),
						Some(LayerKind::Embed | LayerKind::Attn)
					) {
					in_dim += c_cat;
				}
				let (w, b, slope) = if !try_resume {
					let scale = (2.0 / in_dim as f64).sqrt();
					let w = kernels::gpu_randn(in_dim * units, 1234 + (li as u32) * 7919)
						.expect("randn w");
					kernels::gpu_scale_inplace(&w, scale, in_dim * units);
					let b = GpuBuffer::upload(&vec![0.0f64; units]).expect("upload b");
					(w, b, None)
				} else {
					// Distribute saved neurons back into this layer's W (in_dim×units,
					// row-major index i*units+j) and bias[j], matching dump_ogdl's layout.
					// A PReLU layer shares one slope across neurons → take the first `a`.
					let mut wh = vec![0.0f64; in_dim * units];
					let mut bh = vec![0.0f64; units];
					let mut slope = None;
					for j in 0..units {
						let (ws, bias, a) = match resumed.get(si) {
							Some(Saved::Dense { w, b, a }) => (w, *b, *a),
							_ => {
								return Err(format!(
									"layer {li} neuron {j}: checkpoint has no dense (z) block here"
								));
							}
						};
						if ws.len() != in_dim {
							return Err(format!(
								"layer {li} neuron {j}: checkpoint has {} weights, model needs {in_dim} (data feature count differs?)",
								ws.len()
							));
						}
						for i in 0..in_dim {
							wh[i * units + j] = ws[i];
						}
						bh[j] = bias;
						if j == 0 {
							slope = a;
						}
						si += 1;
					}
					(
						GpuBuffer::upload(&wh).expect("upload w"),
						GpuBuffer::upload(&bh).expect("upload b"),
						slope,
					)
				};
				let palpha = if act == Activation::PRelu {
					GpuBuffer::upload(&[slope.unwrap_or(PRELU_INIT)])
						.expect("prelu alpha")
				} else {
					dummy()
				};
				params.push(LayerParams {
					kind: LayerKind::Dense,
					w,
					b,
					in_dim,
					out_dim: units,
					act,
					dim: 0,
					vocab: 0,
					wk: dummy(),
					wv: dummy(),
					wo: dummy(),
					heads: 0,
					palpha,
				});
				in_dim = units;
			}
			// Every saved block must be consumed: a leftover means the checkpoint has
			// more layers/neurons than this architecture (wrong file or changed arch).
			if try_resume && si != resumed.len() {
				return Err(format!(
					"checkpoint has {} saved blocks, this architecture consumed {si}",
					resumed.len()
				));
			}
			Ok(params)
		};
		let params = match build_params(did_resume) {
			Ok(p) => p,
			Err(what) => {
				if ask_overwrite(&what) {
					did_resume = false;
					build_params(false).unwrap_or_else(|e| panic!("{e}"))
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
			Self::forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
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
				Self::forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
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
					Self::download_scalar(&sc.metric_scalar)
				} else {
					kernels::gpu_ss_res_into(out, &ybuf, &sc.metric_scalar, n * k);
					1.0 - Self::download_scalar(&sc.metric_scalar) / ss_tot
				}
			} else {
				f64::NAN
			};
			let loss_scale = if checkpointing {
				let (sign, div) = self.metric_gpu_into(Metric::Loss, out, &ybuf, &sc, n, k, ss_tot);
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
								self.metric_gpu(
									m, out, &ybuf, &sc, n, k, ss_tot, e, elapsed,
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
							self.metric_gpu(
								m, out, &ybuf, &sc, n, k, ss_tot, e, elapsed,
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
			Self::forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
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
				Self::download_scalar(&sc.metric_scalar)
			} else {
				kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n * k);
				1.0 - Self::download_scalar(&sc.metric_scalar) / ss_tot
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
	fn dump_ogdl(params: &[LayerParams], parts: &[Param], key: &str, score: f64) -> String {
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
						let table = Self::download_vec(&p.w, p.vocab * p.dim);
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
								join(&Self::download_vec(buf, dd))
							));
						}
					}
					if want_b {
						// Bare attention has a single shared (zero) bias [d];
						// emit it as bq/bk/bv/bo for format completeness.
						let bias = Self::download_vec(&p.b, p.dim);
						for nm in ["bq", "bk", "bv", "bo"] {
							out.push_str(&format!("    {nm}={}\n", join(&bias)));
						}
					}
				}
				LayerKind::Dense => {
					let w = Self::download_vec(&p.w, p.in_dim * p.out_dim);
					let b = Self::download_vec(&p.b, p.out_dim);
					let slope = (p.act == Activation::PRelu)
						.then(|| Self::download_scalar(&p.palpha));
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
	fn write_ogdl(path: &str, out: &str) {
		if let Some(parent) = std::path::Path::new(path).parent()
			&& !parent.as_os_str().is_empty()
		{
			std::fs::create_dir_all(parent)
				.unwrap_or_else(|e| panic!("save: mkdir {}: {e}", parent.display()));
		}
		std::fs::write(path, out).unwrap_or_else(|e| panic!("save: write {path}: {e}"));
	}

	/// Parse an OGDL dump into one `Saved` block per layer/neuron, in save order
	/// (embed table, attn projections+biases, or one dense neuron each). A missing
	/// file is not an error: it just means "first run" — return empty so training
	/// starts from random init and a later run can resume.
	fn load_ogdl(path: &str) -> Vec<Saved> {
		let text = match std::fs::read_to_string(path) {
			Ok(t) => t,
			Err(_) => {
				eprintln!("no data in {path}, initialized random weights and biases");
				return Vec::new();
			}
		};
		let vals = |s: &str| -> Vec<f64> {
			s.split_whitespace()
				.map(|t| t.parse::<f64>().expect("resume: parse value"))
				.collect()
		};
		// A block accumulates several lines before it's complete, so collect into a
		// mutable `cur` and flush it on the next header (and at EOF).
		enum Cur {
			Embed(Vec<(usize, Vec<f64>)>),
			Attn {
				wq: Vec<f64>,
				wk: Vec<f64>,
				wv: Vec<f64>,
				wo: Vec<f64>,
				bq: Vec<f64>,
				bk: Vec<f64>,
				bv: Vec<f64>,
				bo: Vec<f64>,
			},
			Dense {
				w: Vec<f64>,
				b: f64,
				a: Option<f64>,
			},
		}
		let flush = |cur: Option<Cur>, out: &mut Vec<Saved>| match cur {
			None => {}
			Some(Cur::Embed(mut rows)) => {
				rows.sort_by_key(|(id, _)| *id);
				out.push(Saved::Embed(
					rows.into_iter().flat_map(|(_, v)| v).collect(),
				));
			}
			Some(Cur::Attn {
				wq,
				wk,
				wv,
				wo,
				bq,
				bk,
				bv,
				bo,
			}) => {
				out.push(Saved::Attn {
					wq,
					wk,
					wv,
					wo,
					bq,
					bk,
					bv,
					bo,
				});
			}
			Some(Cur::Dense { w, b, a }) => out.push(Saved::Dense { w, b, a }),
		};
		let mut out: Vec<Saved> = Vec::new();
		let mut cur: Option<Cur> = None;
		for line in text.lines() {
			let t = line.trim();
			if t.is_empty() {
				continue;
			}
			match t.split_once('=') {
				// Bare token = block header: flush the previous block, open a new one.
				None => {
					flush(cur.take(), &mut out);
					cur = Some(match t {
						"embed" => Cur::Embed(Vec::new()),
						"attn" => Cur::Attn {
							wq: vec![],
							wk: vec![],
							wv: vec![],
							wo: vec![],
							bq: vec![],
							bk: vec![],
							bv: vec![],
							bo: vec![],
						},
						_ => Cur::Dense {
							w: Vec::new(),
							b: 0.0,
							a: None,
						}, // z{k}
					});
				}
				Some((k, _)) if matches!(k.trim(), "r2" | "acc") => {}
				Some((k, v)) => {
					let key = k.trim();
					match cur
						.as_mut()
						.expect("resume: value line before any block header")
					{
						Cur::Embed(rows) => {
							rows.push((
								key.parse().expect("resume: embed row id"),
								vals(v),
							));
						}
						Cur::Attn {
							wq,
							wk,
							wv,
							wo,
							bq,
							bk,
							bv,
							bo,
						} => match key {
							"wq" => *wq = vals(v),
							"wk" => *wk = vals(v),
							"wv" => *wv = vals(v),
							"wo" => *wo = vals(v),
							"bq" => *bq = vals(v),
							"bk" => *bk = vals(v),
							"bv" => *bv = vals(v),
							"bo" => *bo = vals(v),
							_ => panic!("resume: unknown attn key {key}"),
						},
						Cur::Dense { w, b, a } => match key {
							"b" => *b = v.trim().parse().expect("resume: dense b"),
							"a" => {
								*a = Some(v
									.trim()
									.parse()
									.expect("resume: dense a"))
							}
							"w" => *w = vals(v),
							// Back-compat: the old format wrote one weight per line
							// (w1=, w2=, …) in order — append each to the vector.
							_ if key.starts_with('w')
								&& key[1..].chars().all(|c| c.is_ascii_digit())
								&& key.len() > 1 =>
							{
								w.push(v
									.trim()
									.parse()
									.expect("resume: dense w{n}"));
							}
							_ => {
								panic!(
									"resume: unrecognized key '{key}' in {path} — incompatible checkpoint; rm {path} to start fresh"
								);
							}
						},
					}
				}
			}
		}
		flush(cur.take(), &mut out);
		out
	}

	fn saved_score(path: &str, key: &str) -> Option<f64> {
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
		// eval builds the SAME (full) Scratch fit uses — its attention buffers
		// (scores + a second dscores + 8 per-head seq buffers) are huge for a big
		// holdout. An over-budget GPU alloc HIP-asserts (core dump) instead of
		// returning a catchable error, so pre-check the EXACT total against free
		// VRAM and skip eval cleanly when it won't fit. Never chunk/minibatch eval.
		{
			let n = data.x.nrows();
			let need = Scratch::vram_bytes(&params, n, true);
			let (mut free, mut total) = (0usize, 0usize);
			unsafe { gpu_core::hip::hipMemGetInfo(&mut free, &mut total) };
			if need > free / 10 * 9 {
				eprintln!(
					"\x1b[33meval skipped\x1b[0m\n    needs {} for {n} rows, only {} free\n    full-batch all-token attention can't hold this holdout — reduce rows/seq or eval fewer samples",
					crate::data::human_bytes(need),
					crate::data::human_bytes(free)
				);
				return;
			}
		}
		// Mirror fit's two-branch input construction: text token-id columns pass
		// raw to embed→attn; the categorical branch is scaled with the TRAIN-set
		// scaler (same mean/std — eval must see the exact transform training saw).
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(_)));
		let cat_cols: Vec<usize> = if embed_first {
			(0..data.x.ncols())
				.filter(|c| !data.text_cols.contains(c))
				.collect()
		} else {
			Vec::new()
		};
		let xinput = if embed_first {
			data.x.select(ndarray::Axis(1), &data.text_cols)
		} else {
			data.x.clone()
		};
		let (xraw, n, d) = Self::upload(&xinput);
		let scaler = self.scaler.borrow();
		let scaler = scaler
			.as_ref()
			.expect("eval: missing scaler; call train first");
		let (xbuf, x_cat) = if embed_first {
			if cat_cols.is_empty() {
				(xraw, None)
			} else {
				let cat = data.x.select(ndarray::Axis(1), &cat_cols);
				let (craw, _, c) = Self::upload(&cat);
				(xraw, Some(Self::zscore_apply(&craw, n, c, scaler)))
			}
		} else {
			(Self::zscore_apply(&xraw, n, d, scaler), None)
		};
		let last = params.len() - 1;
		let k = params[last].out_dim;
		// Forward on GPU; accuracy reduced on GPU (no CPU metric computation).
		let sc = Scratch::new(&params, n, true);
		let acts = &sc.acts;
		Self::forward_into(&params, &xbuf, x_cat.as_ref(), n, acts, &sc);
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
			let acc = Self::download_scalar(&scalar);
			let correct = (acc * n as f64).round() as usize;
			eprintln!("eval: accuracy = {acc:.4} ({correct}/{n})");
		} else {
			eprintln!("eval: {n} samples (no target column, accuracy unavailable)");
		}
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
	use std::sync::LazyLock;

	static CHURN: LazyLock<Option<crate::dataset::Dataset>> = LazyLock::new(|| {
		const TRAIN: &str = "/home/nate/Desktop/playground-series-s6e3/train.csv";
		if !std::path::Path::new(TRAIN).exists() {
			return None;
		}
		let (train, _) = crate::dataset::Data::load()
			.set(TRAIN)
			.target("Churn")
			.prepare();
		Some(train)
	});

	#[test]
	fn gpu_metrics_match_cpu_reference() {
		let Some(train) = CHURN.as_ref() else {
			eprintln!("skip: churn dataset absent");
			return;
		};
		gpu_core::hip::set_device(0).expect("set_device");
		let x = &train.x;
		let y = &train.y;
		let n = x.nrows();
		let d = x.ncols();

		// Two-layer params, random init (as fit does) — just to get real GPU preds.
		let h = 8usize;
		let w1 = kernels::gpu_randn(d * h, 11).expect("w1");
		let b1 = GpuBuffer::upload(&vec![0.0f64; h]).expect("b1");
		let w2 = kernels::gpu_randn(h, 22).expect("w2");
		let b2 = GpuBuffer::upload(&[0.0f64; 1]).expect("b2");
		let params = vec![
			LayerParams {
				kind: LayerKind::Dense,
				w: w1,
				b: b1,
				in_dim: d,
				out_dim: h,
				act: Activation::Relu,
				dim: 0,
				vocab: 0,
				wk: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wv: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wo: GpuBuffer::upload(&[0.0f64]).expect("d"),
				heads: 0,
				palpha: GpuBuffer::upload(&[0.0f64]).expect("d"),
			},
			LayerParams {
				kind: LayerKind::Dense,
				w: w2,
				b: b2,
				in_dim: h,
				out_dim: 1,
				act: Activation::Sigmoid,
				dim: 0,
				vocab: 0,
				wk: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wv: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wo: GpuBuffer::upload(&[0.0f64]).expect("d"),
				heads: 0,
				palpha: GpuBuffer::upload(&[0.0f64]).expect("d"),
			},
		];

		let (xbuf, nn, _d) = Model::upload(x);
		assert_eq!(nn, n);
		let ybuf = GpuBuffer::upload(y.as_slice().expect("y contig")).expect("ybuf");
		let sc = Scratch::new(&params, n, false);
		Model::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		let last = params.len() - 1;
		let p = Model::download_vec(&sc.acts[last], n);
		let ybar = y.iter().sum::<f64>() / n as f64;
		let ss_tot = y.iter().map(|v| (v - ybar).powi(2)).sum::<f64>();

		// Independent CPU references.
		let ss_res: f64 = (0..n).map(|i| (y[i] - p[i]).powi(2)).sum();
		let r2_ref = 1.0 - ss_res / ss_tot;
		let acc_ref =
			(0..n).filter(|&i| (p[i] >= 0.5) == (y[i] >= 0.5)).count() as f64 / n as f64;
		let mse_ref = (0..n).map(|i| (p[i] - y[i]).powi(2)).sum::<f64>() / n as f64;
		let mae_ref = (0..n).map(|i| (p[i] - y[i]).abs()).sum::<f64>() / n as f64;
		let huber_ref = (0..n)
			.map(|i| {
				let r = p[i] - y[i];
				if r.abs() <= 1.0 {
					0.5 * r * r
				} else {
					r.abs() - 0.5
				}
			})
			.sum::<f64>() / n as f64;
		let eps = 1e-7;
		let bce_ref = -(0..n)
			.map(|i| {
				let pc = p[i].clamp(eps, 1.0 - eps);
				y[i] * pc.ln() + (1.0 - y[i]) * (1.0 - pc).ln()
			})
			.sum::<f64>() / n as f64;

		let close = |a: f64, b: f64, what: &str| {
			let tol = 1e-6 * a.abs().max(b.abs()).max(1.0);
			assert!(
				(a - b).abs() <= tol,
				"{what}: gpu={a} cpu={b} diff={}",
				(a - b).abs()
			);
		};
		let mdl = |loss: Loss| Model {
			specs: vec![],
			loss,
			lr: 0.01,
			params: RefCell::new(vec![]),
			scaler: RefCell::new(None),
		};

		let out = &sc.acts[last];
		close(
			mdl(Loss::Mse).metric_gpu(Metric::R2, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			r2_ref,
			"R2",
		);
		close(
			mdl(Loss::Mse).metric_gpu(
				Metric::Accuracy,
				out,
				&ybuf,
				&sc,
				n,
				1,
				ss_tot,
				0,
				0.0,
			),
			acc_ref,
			"Accuracy",
		);
		close(
			mdl(Loss::Mse).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			mse_ref,
			"MSE",
		);
		close(
			mdl(Loss::Mae).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			mae_ref,
			"MAE",
		);
		close(
			mdl(Loss::Huber).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			huber_ref,
			"Huber",
		);
		close(
			mdl(Loss::Bce).metric_gpu(Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			bce_ref,
			"BCE",
		);

		eprintln!(
			"OK n={n} d={d}  R2={r2_ref:.6}  acc={acc_ref:.6}  mse={mse_ref:.6}  mae={mae_ref:.6}  huber={huber_ref:.6}  bce={bce_ref:.6}"
		);
	}

	// The preallocated training loop must (1) compute the same forward as the
	// retained allocate-return path, (2) allocate ZERO GPU buffers per epoch in
	// steady state (flat VRAM, no sawtooth), and (3) still gradient-descend
	// (train R² rises). Features are standardized on-GPU with existing reduce +
	// broadcast kernels so the raw frequency-encoded churn columns don't saturate
	// sigmoid — a well-posed problem on real data, not a hand-rolled scaler.
	#[test]
	fn fit_loop_allocations_flat() {
		let Some(train) = CHURN.as_ref() else {
			eprintln!("skip: churn dataset absent");
			return;
		};
		gpu_core::hip::set_device(0).expect("set_device");
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
			let w = kernels::gpu_randn(fan_in * units, seed).expect("randn");
			kernels::gpu_scale_inplace(&w, scale, fan_in * units);
			let b = GpuBuffer::upload(&vec![0.0f64; units]).expect("b");
			LayerParams {
				kind: LayerKind::Dense,
				w,
				b,
				in_dim: fan_in,
				out_dim: units,
				act,
				dim: 0,
				vocab: 0,
				wk: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wv: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wo: GpuBuffer::upload(&[0.0f64]).expect("d"),
				heads: 0,
				palpha: GpuBuffer::upload(&[0.0f64]).expect("d"),
			}
		};
		let params = vec![
			mk(d, h, 11, Activation::Relu),
			mk(h, 1, 22, Activation::Sigmoid),
		];
		let last = params.len() - 1;

		// (1) predict (temporary acts) must equal forward_into (preallocated acts).
		let out_ref = Model::predict(&params, &xbuf, None, n);
		let sc = Scratch::new(&params, n, false);
		Model::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		let out_into = Model::download_vec(&sc.acts[last], n);
		let fwd_diff = out_ref
			.iter()
			.zip(&out_into)
			.map(|(a, b)| (a - b).abs())
			.fold(0.0, f64::max);
		assert!(
			fwd_diff < 1e-9,
			"predict != forward_into, maxdiff={fwd_diff}"
		);

		// (2)+(3) Train through the preallocated loop, measuring per-epoch GPU
		// allocations and train R². download_vec is host-only (no GpuBuffer
		// alloc), so reading R² never perturbs the count.
		let model = Model {
			specs: vec![],
			loss: Loss::Mse,
			lr: 0.5,
			params: RefCell::new(vec![]),
			scaler: RefCell::new(None),
		};
		let ybar = y.iter().sum::<f64>() / n as f64;
		let ss_tot: f64 = y.iter().map(|v| (v - ybar).powi(2)).sum();

		Model::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		model.backward_step(&params, &xbuf, &ybuf, n, &sc);

		const EPOCHS: usize = 10;
		let mut r2s = Vec::with_capacity(EPOCHS);
		{
			let _alloc_guard = gpu_core::memory::AllocGuard::freeze();
			for _ in 0..EPOCHS {
				Model::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
				model.backward_step(&params, &xbuf, &ybuf, n, &sc);
				kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n);
				r2s.push(1.0 - Model::download_scalar(&sc.metric_scalar) / ss_tot);
			}
		}

		assert!(r2s.iter().all(|v| v.is_finite()), "non-finite R²: {r2s:?}");
		assert!(
			r2s[EPOCHS - 1] > r2s[0],
			"R² did not rise: first={} last={}",
			r2s[0],
			r2s[EPOCHS - 1]
		);

		eprintln!("R2 first={:.6} last={:.6}", r2s[0], r2s[EPOCHS - 1]);
	}

	#[test]
	fn ping_pong_gradients_match_per_layer() {
		let Some(train) = CHURN.as_ref() else {
			eprintln!("skip: churn dataset absent");
			return;
		};
		gpu_core::hip::set_device(0).expect("set_device");
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

		let h = 16usize;
		let lr = 0.01;
		let mk = |fan_in: usize, units: usize, seed: u32, act: Activation| {
			let scale = (2.0 / fan_in as f64).sqrt();
			let w = kernels::gpu_randn(fan_in * units, seed).expect("randn");
			kernels::gpu_scale_inplace(&w, scale, fan_in * units);
			let b = GpuBuffer::upload(&vec![0.0f64; units]).expect("b");
			LayerParams {
				kind: LayerKind::Dense,
				w,
				b,
				in_dim: fan_in,
				out_dim: units,
				act,
				dim: 0,
				vocab: 0,
				wk: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wv: GpuBuffer::upload(&[0.0f64]).expect("d"),
				wo: GpuBuffer::upload(&[0.0f64]).expect("d"),
				heads: 0,
				palpha: GpuBuffer::upload(&[0.0f64]).expect("d"),
			}
		};

		// --- save initial weights ---
		let params = vec![
			mk(d, h, 11, Activation::Relu),
			mk(h, 1, 22, Activation::Sigmoid),
		];
		let last = params.len() - 1;
		let init_w: Vec<Vec<f64>> = params
			.iter()
			.map(|p| Model::download_vec(&p.w, p.in_dim * p.out_dim))
			.collect();
		let init_b: Vec<Vec<f64>> = params
			.iter()
			.map(|p| Model::download_vec(&p.b, p.out_dim))
			.collect();

		// --- ping-pong backward (modifies weights via SGD) ---
		let model = Model {
			specs: vec![],
			loss: Loss::Mse,
			lr,
			params: RefCell::new(vec![]),
			scaler: RefCell::new(None),
		};
		let sc = Scratch::new(&params, n, false);
		Model::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		model.backward_step(&params, &xbuf, &ybuf, n, &sc);
		let pp_w: Vec<Vec<f64>> = params
			.iter()
			.map(|p| Model::download_vec(&p.w, p.in_dim * p.out_dim))
			.collect();
		let pp_b: Vec<Vec<f64>> = params
			.iter()
			.map(|p| Model::download_vec(&p.b, p.out_dim))
			.collect();

		// --- restore initial weights ---
		for (l, p) in params.iter().enumerate() {
			GpuBuffer::upload(&init_w[l])
				.expect("restore w")
				.download(&mut [0.0; 0])
				.ok();
			let wb = GpuBuffer::upload(&init_w[l]).expect("upload w");
			unsafe {
				gpu_core::hip::hipMemcpy(
					p.w.ptr_raw(),
					wb.ptr_raw() as *const std::ffi::c_void,
					init_w[l].len() * 8,
					gpu_core::hip::HIP_MEMCPY_D2D,
				)
			};
			let bb = GpuBuffer::upload(&init_b[l]).expect("upload b");
			unsafe {
				gpu_core::hip::hipMemcpy(
					p.b.ptr_raw(),
					bb.ptr_raw() as *const std::ffi::c_void,
					init_b[l].len() * 8,
					gpu_core::hip::HIP_MEMCPY_D2D,
				)
			};
		}

		// --- per-layer reference backward with same lr ---
		Model::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		let mut ref_da: Vec<GpuBuffer> = Vec::new();
		let mut ref_dz: Vec<GpuBuffer> = Vec::new();
		let mut ref_dw: Vec<GpuBuffer> = Vec::new();
		let mut ref_db: Vec<GpuBuffer> = Vec::new();
		let max_ws = params
			.iter()
			.map(|p| kernels::gpu_reduce_sum_cols_workspace_bytes(n, p.out_dim))
			.max()
			.unwrap_or(0);
		let ref_ws = GpuBuffer::alloc_bytes(
			max_ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1)),
		)
		.expect("ref ws");
		for p in &params {
			ref_da.push(GpuBuffer::alloc(n * p.out_dim).expect("ref da"));
			ref_dz.push(GpuBuffer::alloc(n * p.out_dim).expect("ref dz"));
			ref_dw.push(GpuBuffer::alloc(p.in_dim * p.out_dim).expect("ref dw"));
			ref_db.push(GpuBuffer::alloc(p.out_dim).expect("ref db"));
		}
		Model::loss_grad_into(
			model.loss,
			&sc.acts[last],
			&ybuf,
			&ref_da[last],
			n,
			n * params[last].out_dim,
		);
		for l in (0..params.len()).rev() {
			let (in_dim, out_dim) = (params[l].in_dim, params[l].out_dim);
			let m = n * out_dim;
			let grad = match params[l].act {
				Activation::Relu => {
					kernels::gpu_relu_backward_into(
						&ref_da[l],
						&sc.acts[l],
						&ref_dz[l],
						m,
					);
					&ref_dz[l]
				}
				Activation::Sigmoid => {
					kernels::gpu_sigmoid_backward_into(
						&ref_da[l],
						&sc.acts[l],
						&ref_dz[l],
						m,
					);
					&ref_dz[l]
				}
				Activation::Linear => &ref_da[l],
				_ => unreachable!("this test builds only relu/sigmoid/linear layers"),
			};
			let a_prev = if l == 0 { &xbuf } else { &sc.acts[l - 1] };
			if out_dim == 1 {
				kernels::gpu_dgemv_into(a_prev, grad, &ref_dw[l], n, in_dim, true);
				kernels::gpu_reduce_sum_cols_into(grad, &ref_db[l], &ref_ws, n, 1);
				if l > 0 {
					kernels::gpu_dger_into(grad, &params[l].w, &ref_da[l - 1], n, in_dim);
				}
			} else if l > 0 {
				kernels::gpu_linear_backward_full_into(
					grad,
					a_prev,
					&params[l].w,
					&ref_da[l - 1],
					&ref_dw[l],
					&ref_db[l],
					&ref_ws,
					n,
					out_dim,
					in_dim,
				);
			} else {
				kernels::gpu_linear_backward_weights_only_into(
					grad, a_prev, &ref_dw[l], &ref_db[l], &ref_ws, n, out_dim, in_dim,
				);
			}
			kernels::gpu_sgd_update(&params[l].w, &ref_dw[l], lr, in_dim * out_dim);
			kernels::gpu_sgd_update(&params[l].b, &ref_db[l], lr, out_dim);
		}
		let ref_w: Vec<Vec<f64>> = params
			.iter()
			.map(|p| Model::download_vec(&p.w, p.in_dim * p.out_dim))
			.collect();
		let ref_b: Vec<Vec<f64>> = params
			.iter()
			.map(|p| Model::download_vec(&p.b, p.out_dim))
			.collect();

		// --- compare updated weights ---
		let mut max_w_diff = 0.0f64;
		let mut max_b_diff = 0.0f64;
		for (l, p) in params.iter().enumerate() {
			let _ = p;
			let wd = pp_w[l]
				.iter()
				.zip(&ref_w[l])
				.map(|(a, b)| (a - b).abs())
				.fold(0.0, f64::max);
			let bd = pp_b[l]
				.iter()
				.zip(&ref_b[l])
				.map(|(a, b)| (a - b).abs())
				.fold(0.0, f64::max);
			eprintln!("layer {l}: W maxdiff={wd:.2e}  b maxdiff={bd:.2e}");
			if wd > max_w_diff {
				max_w_diff = wd;
			}
			if bd > max_b_diff {
				max_b_diff = bd;
			}
		}
		assert!(
			max_w_diff < 1e-10,
			"weights mismatch after backward: max abs diff = {max_w_diff:.2e}"
		);
		assert!(
			max_b_diff < 1e-10,
			"biases mismatch after backward: max abs diff = {max_b_diff:.2e}"
		);
	}

	// Host-only: the OGDL parser must read back the documented embed/attn/dense
	// format exactly (no GPU — pure file parse). Mirrors what dump_ogdl writes:
	// an embed table by token id, attn projections + zero biases, and dense
	// neurons with optional PReLU slope `a`.
	#[test]
	fn ogdl_format_roundtrips_host_side() {
		let path = std::env::temp_dir().join("nrs_ogdl_roundtrip.ogdl");
		let text = "\
r2=0.42
embed
    0=-0.0312 0.1847 -0.0551
    1=0.0892 -0.2104 0.0033
attn
    wq=1 2 3 4
    wk=5 6 7 8
    wv=9 10 11 12
    wo=13 14 15 16
    bq=0 0
    bk=0 0
    bv=0 0
    bo=0 0
z1
    w=0.01 -0.02 0.03
    b=0.001
z2
    w=0.04 0.05 0.06
    a=0.25
    b=0.002
";
		std::fs::write(&path, text).expect("write tmp ogdl");
		let parsed = Model::load_ogdl(path.to_str().expect("utf8 path"));
		std::fs::remove_file(&path).ok();
		assert_eq!(parsed.len(), 4);
		assert_eq!(
			parsed[0],
			Saved::Embed(vec![-0.0312, 0.1847, -0.0551, 0.0892, -0.2104, 0.0033])
		);
		assert_eq!(
			parsed[1],
			Saved::Attn {
				wq: vec![1.0, 2.0, 3.0, 4.0],
				wk: vec![5.0, 6.0, 7.0, 8.0],
				wv: vec![9.0, 10.0, 11.0, 12.0],
				wo: vec![13.0, 14.0, 15.0, 16.0],
				bq: vec![0.0, 0.0],
				bk: vec![0.0, 0.0],
				bv: vec![0.0, 0.0],
				bo: vec![0.0, 0.0],
			}
		);
		assert_eq!(
			parsed[2],
			Saved::Dense {
				w: vec![0.01, -0.02, 0.03],
				b: 0.001,
				a: None
			}
		);
		assert_eq!(
			parsed[3],
			Saved::Dense {
				w: vec![0.04, 0.05, 0.06],
				b: 0.002,
				a: Some(0.25)
			}
		);
	}
}
