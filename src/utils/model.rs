use crate::dataset::{Dataset, collapse_onehot};
use crate::train::INTERRUPTED;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use recipe_infer::{
	LayerParams, Scaler, Scratch, build_layer_params, download_scalar, forward_into, infer_scored,
	load_ogdl_str, pinned_vocab, upload, vram_estimate, zscore_apply,
};
use std::cell::RefCell;
use std::io::IsTerminal as _;
use std::sync::atomic::Ordering;

// The execution-description enums and their user-facing constructor constants now
// live in `recipe-infer` (the inference engine that interprets them); re-export
// them so the existing `nates_recipe::*` / `crate::model::*` API is unchanged.
pub use recipe_infer::{
	Accuracy, Activation, Epoch, LayerSpec, Loss, Lr, Metric, R2, Time, bce, ce, elu, focal, gelu,
	huber, leak, linear, mae, mse, prelu, relu, selu, sig, silu, swish, tanh,
};

/// Accepts `units` (linear dense) or `embed(dim)` / `attn(heads)` for
/// `Model::layer`. Chain `.relu()`, `.leak()`, etc. for activations.
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
pub struct EmbedSpec(usize, Option<usize>);
#[allow(non_upper_case_globals)]
pub fn embed(dim: usize) -> EmbedSpec {
	EmbedSpec(dim, None)
}
impl EmbedSpec {
	/// Pin the token table to exactly `v` rows instead of deriving it from the
	/// data. Use when the vocabulary is a fixed alphabet (e.g. `embed(32).vocab(257)`
	/// for the byte-level detector). `fit`/resume/preflight then use `v` verbatim.
	pub fn vocab(mut self, v: usize) -> EmbedSpec {
		self.1 = Some(v);
		self
	}
}
impl IntoLayer for EmbedSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Embed(self.0, self.1)
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

/// Which parameters `save` writes — pass `w`, `b`, or both (consts in the crate
/// root, kept out of this module so they don't shadow local `w`/`b` bindings).
/// The enum itself lives in recipe-infer beside the OGDL codec it gates.
pub use recipe_infer::Param;

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
	fn infer_only(&self) -> bool;
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
	fn infer_only(&self) -> bool {
		false
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
	fn infer_only(&self) -> bool {
		true
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
	pub(crate) epochs: usize,
	pub(crate) log_every: usize,
	pub(crate) metrics: Vec<Metric>,
	pub(crate) plot: Vec<Metric>,
	pub(crate) resume: Option<String>,
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
	pub(crate) fn resolve(path: &str) -> String {
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

	/// Warm-start from `model.ogdl` in the cwd (skips silently if absent). For a
	/// custom path use [`resume_from`](Self::resume_from).
	pub fn resume(self) -> Train {
		self.resume_from("model.ogdl")
	}

	pub fn resume_from(mut self, path: impl Into<String>) -> Train {
		self.resume = Some(path.into());
		self
	}

	/// The raw forward outputs from the most recent infer `run`, as `(preds, k)`:
	/// `preds` is row-major `n*k` logits, `k` the output width. `None` if the last
	/// run trained (no preds) or never ran. Used by the type detector to read the
	/// per-column class logits without downloading through the save path.
	#[allow(dead_code)]
	pub(crate) fn preds(&self) -> Option<(Vec<f64>, usize)> {
		let last = self.last.borrow();
		last.preds.clone().map(|p| (p, last.k))
	}

	pub fn run(&self, model: &Model, data: &impl RunData) {
		let ds = data.dataset();
		let forward_only = data.infer_only() || !ds.has_target || self.epochs == 0;
		let issues = preflight(model, ds, forward_only);
		if !issues.is_empty() && !confirm_issues(&issues) {
			eprintln!("\x1b[33maborted\x1b[0m");
			return;
		}
		if !forward_only {
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
					let embed_first = matches!(model.specs.first(), Some(LayerSpec::Embed(..)));
					let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
					let (col_x, col_ec, _col_v) = if embed_cats {
						let (x, ec, v) = collapse_onehot(ds);
						(Some(x), ec, v)
					} else {
						(None, Vec::new(), 0)
					};
					let eff_x = col_x.as_ref().unwrap_or(&ds.x);
					let eff_text = if embed_cats { &col_ec } else { &ds.text_cols };
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
					let (xraw, nn, d) = upload(&xinput);
					assert_eq!(nn, n);
					let scaler = model.scaler.borrow();
					let (xbuf, x_cat) = if embed_first {
						if cat_cols.is_empty() {
							(xraw, None)
						} else {
							let cat = eff_x.select(ndarray::Axis(1), &cat_cols);
							let (craw, _, c) = upload(&cat);
							let scaled = if let Some(sc_ref) = scaler.as_ref() {
								zscore_apply(&craw, n, c, sc_ref)
							} else {
								craw
							};
							(xraw, Some(scaled))
						}
					} else {
						let scaled = if let Some(sc_ref) = scaler.as_ref() {
							if sc_ref.mean.is_empty() { xraw } else { zscore_apply(&xraw, n, d, sc_ref) }
						} else {
							xraw
						};
						(scaled, None)
					};
					forward_into(&params, &xbuf, x_cat.as_ref(), n, &sc.acts, &sc);
					if let Some((ymean, ystd)) = *model.yscaler.borrow() {
						kernels::gpu_scale_inplace(&sc.acts[last], ystd, n * k);
						kernels::gpu_add_scalar_inplace(&sc.acts[last], ymean, n * k);
					}
					let ybuf = GpuBuffer::upload(ds.y.as_slice().expect("y contig")).expect("ybuf");
					if model.loss.is_classification() {
						if k == 1 {
							kernels::gpu_accuracy_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n);
						} else {
							kernels::gpu_argmax_accuracy_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n, k);
						}
						download_scalar(&sc.metric_scalar)
					} else {
						let total = (n * k) as f64;
						let ybar = ds.y.iter().sum::<f64>() / total;
						let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
						kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n * k);
						1.0 - download_scalar(&sc.metric_scalar) / ss_tot
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
			let (xbuf, x_cat, n) = model.prep_eval_input(ds);
			let params = model.params.borrow();
			assert!(!params.is_empty(), "run: call train first");
			let k = params[params.len() - 1].out_dim;
			let yscaler = *model.yscaler.borrow();
			let (score, preds) = if ds.has_target && !self.metrics.is_empty() {
				let ybuf = GpuBuffer::upload(ds.y.as_slice().expect("y contig")).expect("ybuf");
				let total = (n * k) as f64;
				let ybar = ds.y.iter().sum::<f64>() / total;
				let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
				let (preds, vals) = infer_scored(
					&params, &xbuf, x_cat.as_ref(), n, yscaler, Some(&ybuf),
					model.loss, model.lr, &self.metrics, ss_tot,
				);
				eprintln!("eval  {}", model.metrics_line(&self.metrics, &vals));
				let stop = if model.loss.is_classification() { Metric::Accuracy } else { Metric::R2 };
				let score = self
					.metrics
					.iter()
					.zip(vals.iter())
					.find(|(m, _)| **m == stop)
					.map_or(f64::NAN, |(_, v)| *v);
				(score, preds)
			} else {
				let (preds, _) = infer_scored(
					&params, &xbuf, x_cat.as_ref(), n, yscaler, None,
					model.loss, model.lr, &[], 0.0,
				);
				(f64::NAN, preds)
			};
			let mut last = self.last.borrow_mut();
			last.model = model as *const Model;
			last.score = score;
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

	/// Save the FULL trained checkpoint — every param the model allocated — to
	/// `model.ogdl` in the cwd. The model decides what to write; nothing is
	/// hardcoded. For a subset, a custom path, or prediction columns, use
	/// [`save_as`](Self::save_as).
	pub fn save(&self) {
		self.save_ogdl(None, "model.ogdl");
	}

	/// Write the model's params to `path` as OGDL. `filter: None` = everything the
	/// model holds; `Some(parts)` = subset. Best-only guard: skips if the file
	/// already holds a better score.
	fn save_ogdl(&self, filter: Option<&[Param]>, path: &str) {
		let last = self.last.borrow();
		if last.model.is_null() {
			return;
		}
		let model = unsafe { &*last.model };
		let params = model.params.borrow();
		assert!(!params.is_empty(), "save: model has no trained params");
		let key = model.loss.score_key();
		let score = last.score;
		let path = Self::resolve(path);
		if !score.is_finite()
			|| recipe_infer::saved_score(&path, key).is_some_and(|best| score <= best)
		{
			return;
		}
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		recipe_infer::write_ogdl(&path, &recipe_infer::dump_ogdl(&params, filter, key, score));
		let full = std::fs::canonicalize(&path).unwrap_or_else(|_| path.as_str().into());
		eprintln!("saved {} ({neurons} neurons, {key} {score:.4})", full.display());
	}

	pub fn save_as<I, S>(&self, items: I, path: impl Into<String>)
	where
		I: IntoIterator<Item = S>,
		S: Into<SaveItem>,
	{
		let items: Vec<SaveItem> = items.into_iter().map(Into::into).collect();
		let all_params = items.iter().all(|i| matches!(i, SaveItem::W | SaveItem::B));
		if all_params {
			let parts: Vec<Param> = items
				.iter()
				.map(|i| match i {
					SaveItem::W => Param::W,
					SaveItem::B => Param::B,
					_ => unreachable!(),
				})
				.collect();
			self.save_ogdl(Some(&parts), &path.into());
		} else {
			let path = path.into();
			let path = Self::resolve(&path);
			let last = self.last.borrow();
			if last.model.is_null() {
				return;
			}
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
	pub(crate) specs: Vec<LayerSpec>,
	pub(crate) loss: Loss,
	pub(crate) lr: f64,
	pub(crate) params: RefCell<Vec<LayerParams>>,
	pub(crate) scaler: RefCell<Option<Scaler>>,
	pub(crate) yscaler: RefCell<Option<(f64, f64)>>,
}

struct Issue {
	what: String,
	have: String,
	need: String,
}

fn preflight(model: &Model, ds: &Dataset, forward_only: bool) -> Vec<Issue> {
	let mut issues = Vec::new();
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let k = ds.n_targets.max(1);

	if model.specs.is_empty() {
		issues.push(Issue {
			what: "model has 0 layers".into(),
			have: "0 layers".into(),
			need: "≥1 (.layer() before .run())".into(),
		});
		return issues;
	}

	let last_out = match model.specs.last() {
		Some(LayerSpec::Dense(u, _)) => *u,
		_ => 0,
	};
	let n_layers = model.specs.len();
	if model.loss == Loss::Bce && last_out != 1 {
		issues.push(Issue {
			what: format!("dense layer {n_layers} outputs {last_out}, bce loss expects 1"),
			have: format!("{last_out} output units"),
			need: "1 (.layer(1).sigmoid())".into(),
		});
	}
	if model.loss == Loss::Focal && last_out != 1 {
		issues.push(Issue {
			what: format!("dense layer {n_layers} outputs {last_out}, focal loss expects 1"),
			have: format!("{last_out} output units"),
			need: "1 (.layer(1).sigmoid())".into(),
		});
	}
	if model.loss == Loss::Ce && k > 1 && last_out != k {
		issues.push(Issue {
			what: format!("dense layer {n_layers} outputs {last_out}, ce loss expects {k}"),
			have: format!("{last_out} output units"),
			need: format!("{k} (one per target column)"),
		});
	}

	let embed_first = matches!(model.specs.first(), Some(LayerSpec::Embed(..)));
	let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
	let (mut free_vram, mut total_vram) = (0usize, 0usize);
	unsafe { gpu_core::hip::hipMemGetInfo(&mut free_vram, &mut total_vram) };
	let (cat_cols, text_d, vocab) = if embed_cats {
		let n_cat = ds.onehot_groups.len();
		let n_oh: usize = ds.onehot_groups.iter().map(|(_, len)| len).sum();
		let passthrough = d - n_oh;
		let total_cats: usize = ds.onehot_groups.iter().map(|(_, len)| len).sum();
		(passthrough, n_cat, total_cats)
	} else if embed_first {
		let tc = ds.text_cols.len();
		let vocab = pinned_vocab(&model.specs)
			.unwrap_or_else(|| ds.x.iter().cloned().fold(0.0f64, f64::max) as usize + 1);
		(d - tc, tc, vocab)
	} else {
		(0, d, 0)
	};
	let need = vram_estimate(&model.specs, n, text_d, k, vocab, cat_cols, forward_only);
	if need > free_vram / 10 * 9 {
		let mode = if forward_only { "inference" } else { "training" };
		issues.push(Issue {
			what: format!("{mode} on {n} rows × {d} features exceeds GPU memory"),
			have: format!("{} free of {} total", crate::data::human_bytes(free_vram), crate::data::human_bytes(total_vram)),
			need: format!("{}", crate::data::human_bytes(need)),
		});
	}

	issues
}

fn confirm_issues(issues: &[Issue]) -> bool {
	if issues.is_empty() {
		return true;
	}
	let interactive = std::io::stdin().is_terminal();
	for (i, issue) in issues.iter().enumerate() {
		eprintln!(
			"\x1b[1;33mpreflight {}/{}\x1b[0m  {}\n    have: {}\n    need: {}",
			i + 1,
			issues.len(),
			issue.what,
			issue.have,
			issue.need,
		);
	}
	if !interactive {
		return false;
	}
	use std::io::Write;
	eprint!("continue anyway? [y/N] ");
	std::io::stderr().flush().ok();
	let mut line = String::new();
	std::io::stdin().read_line(&mut line).ok();
	matches!(line.trim(), "y" | "Y" | "yes" | "YES")
}

impl Model {

	pub fn new() -> Model {
		Model {
			specs: Vec::new(),
			loss: Loss::Mse,
			lr: 0.01,
			params: RefCell::new(Vec::new()),
			scaler: RefCell::new(None),
			yscaler: RefCell::new(None),
		}
	}

	/// Load shipped weights into a freshly-built model for forward-only use, with
	/// NO data and NO file path (`weights` is the OGDL text, e.g. `include_str!`ed).
	/// `proto` carries the architecture (`specs`/`loss`/`lr`); its first `embed`
	/// must pin a fixed vocab. `d` is the model input width (token columns). The
	/// params are built straight from the checkpoint blocks (same builder `fit`
	/// uses); the scaler is set empty (the detector's pure-embed path z-scores
	/// nothing) so `run`'s infer branch finds a scaler and skips scaling.
	pub fn load(weights: &str, proto: Model, d: usize) -> Model {
		let saved = load_ogdl_str(weights);
		let vocab = pinned_vocab(&proto.specs)
			.expect("Model::load: first embed layer must pin a fixed vocab (embed(dim).vocab(v))");
		let params = build_layer_params(&proto.specs, d, 0, vocab, &saved, true)
			.unwrap_or_else(|e| panic!("Model::load: {e}"));
		*proto.params.borrow_mut() = params;
		*proto.scaler.borrow_mut() = Some(Scaler { mean: vec![], std: vec![] });
		*proto.yscaler.borrow_mut() = None;
		proto
	}

	pub fn layer(mut self, spec: impl IntoLayer) -> Model {
		self.specs.push(spec.into_layer());
		self
	}

	fn set_last_activation(mut self, act: Activation) -> Model {
		match self.specs.last_mut() {
			Some(LayerSpec::Dense(_, a)) | Some(LayerSpec::Conv(_, _, _, a)) => *a = act,
			_ => panic!("activation method called but last layer is not dense or conv"),
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

	/// 1D conv: `filters` output channels, `kernel` width, `stride` downsample
	/// factor (1 = none). Stride is a conv parameter, not a separate step.
	pub fn conv(mut self, filters: usize, kernel: usize, stride: usize) -> Model {
		self.specs.push(LayerSpec::Conv(filters, kernel, stride, Activation::Linear));
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
}

impl Default for Model {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod metric_gpu_tests {
	use super::*;
	use recipe_infer::*;
	use std::cell::RefCell;
	use std::sync::LazyLock;

	static CHURN: LazyLock<Option<crate::dataset::Dataset>> = LazyLock::new(|| {
		const TRAIN: &str = "/home/nate/Desktop/playground-series-s6e3/train.csv";
		if !std::path::Path::new(TRAIN).exists() {
			return None;
		}
		let data = crate::dataset::Data::load()
			.set(TRAIN)
			.target("Churn");
		Some(data.set)
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
			conv_cin: 0, conv_k: 0, conv_stride: 0,
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
			conv_cin: 0, conv_k: 0, conv_stride: 0,
			},
		];

		let (xbuf, nn, _d) = upload(x);
		assert_eq!(nn, n);
		let ybuf = GpuBuffer::upload(y.as_slice().expect("y contig")).expect("ybuf");
		let sc = Scratch::new(&params, n, false);
		forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		let last = params.len() - 1;
		let p = download_vec(&sc.acts[last], n);
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
		// metric_gpu reads only loss + lr; lr (0.01) is irrelevant to every metric
		// tested here (it surfaces only for Metric::Lr), so pass it verbatim.
		let out = &sc.acts[last];
		close(
			metric_gpu(Loss::Mse, 0.01, Metric::R2, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			r2_ref,
			"R2",
		);
		close(
			metric_gpu(
				Loss::Mse,
				0.01,
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
			metric_gpu(Loss::Mse, 0.01, Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			mse_ref,
			"MSE",
		);
		close(
			metric_gpu(Loss::Mae, 0.01, Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			mae_ref,
			"MAE",
		);
		close(
			metric_gpu(Loss::Huber, 0.01, Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
			huber_ref,
			"Huber",
		);
		close(
			metric_gpu(Loss::Bce, 0.01, Metric::Loss, out, &ybuf, &sc, n, 1, ss_tot, 0, 0.0),
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

		let (xraw, _, _) = upload(x);
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
			conv_cin: 0, conv_k: 0, conv_stride: 0,
			}
		};
		let params = vec![
			mk(d, h, 11, Activation::Relu),
			mk(h, 1, 22, Activation::Sigmoid),
		];
		let last = params.len() - 1;

		// (1) Two independent forward passes must agree.
		let sc_ref = Scratch::new(&params, n, true);
		forward_into(&params, &xbuf, None, n, &sc_ref.acts, &sc_ref);
		let out_ref = download_vec(&sc_ref.acts[last], n);
		drop(sc_ref);
		let sc = Scratch::new(&params, n, false);
		forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		let out_into = download_vec(&sc.acts[last], n);
		let fwd_diff = out_ref
			.iter()
			.zip(&out_into)
			.map(|(a, b)| (a - b).abs())
			.fold(0.0, f64::max);
		assert!(
			fwd_diff < 1e-9,
			"forward_into not deterministic, maxdiff={fwd_diff}"
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
			yscaler: RefCell::new(None),
		};
		let ybar = y.iter().sum::<f64>() / n as f64;
		let ss_tot: f64 = y.iter().map(|v| (v - ybar).powi(2)).sum();

		forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		model.backward_step(&params, &xbuf, &ybuf, n, &sc);

		const EPOCHS: usize = 10;
		let mut r2s = Vec::with_capacity(EPOCHS);
		{
			let _alloc_guard = gpu_core::memory::AllocGuard::freeze();
			for _ in 0..EPOCHS {
				forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
				model.backward_step(&params, &xbuf, &ybuf, n, &sc);
				kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, &sc.metric_scalar, n);
				r2s.push(1.0 - download_scalar(&sc.metric_scalar) / ss_tot);
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

		let (xraw, _, _) = upload(x);
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
			conv_cin: 0, conv_k: 0, conv_stride: 0,
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
			.map(|p| download_vec(&p.w, p.in_dim * p.out_dim))
			.collect();
		let init_b: Vec<Vec<f64>> = params
			.iter()
			.map(|p| download_vec(&p.b, p.out_dim))
			.collect();

		// --- ping-pong backward (modifies weights via SGD) ---
		let model = Model {
			specs: vec![],
			loss: Loss::Mse,
			lr,
			params: RefCell::new(vec![]),
			scaler: RefCell::new(None),
			yscaler: RefCell::new(None),
		};
		let sc = Scratch::new(&params, n, false);
		forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
		model.backward_step(&params, &xbuf, &ybuf, n, &sc);
		let pp_w: Vec<Vec<f64>> = params
			.iter()
			.map(|p| download_vec(&p.w, p.in_dim * p.out_dim))
			.collect();
		let pp_b: Vec<Vec<f64>> = params
			.iter()
			.map(|p| download_vec(&p.b, p.out_dim))
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
		forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
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
			.map(|p| download_vec(&p.w, p.in_dim * p.out_dim))
			.collect();
		let ref_b: Vec<Vec<f64>> = params
			.iter()
			.map(|p| download_vec(&p.b, p.out_dim))
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

}
