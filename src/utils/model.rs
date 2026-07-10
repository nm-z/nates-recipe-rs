use crate::dataset::Dataset;
use crate::train::INTERRUPTED;
use gpu_core::memory::GpuBuffer;
use recipe_infer::{
	LayerParams, Scaler, infer_scored, load_ogdl_str, pinned_vocab, plan_layer_params,
	vram_estimate,
};
use std::cell::{Cell, RefCell};
use std::io::IsTerminal as _;
use std::sync::atomic::Ordering;

pub use recipe_infer::{
	Accuracy, Activation, Epoch, LayerSpec, Loss, Lr, Metric, R2, Time, bce, ce, elu, focal, gelu,
	hip, huber, leak, linear, mae, mse, prelu, relu, selu, sig, silu, swish, tanh,
};

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

pub struct EmbedSpec(usize, Option<usize>);
pub fn embed(dim: usize) -> EmbedSpec {
	EmbedSpec(dim, None)
}
impl EmbedSpec {
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

pub struct AttnSpec(usize);
pub fn attn(heads: usize) -> AttnSpec {
	AttnSpec(heads)
}
impl IntoLayer for AttnSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Attn(self.0)
	}
}

pub use recipe_infer::Param;

pub trait SavePath {
	fn or_default(self) -> String;
}
impl SavePath for () {
	fn or_default(self) -> String {
		"model.ogdl".to_string()
	}
}
impl SavePath for &str {
	fn or_default(self) -> String {
		self.to_string()
	}
}
impl SavePath for String {
	fn or_default(self) -> String {
		self
	}
}

pub enum Prepared<'a> {
	Owned(Dataset),
	Borrowed(&'a Dataset),
}

impl Prepared<'_> {
	pub fn get(&self) -> &Dataset {
		match self {
			Prepared::Owned(d) => d,
			Prepared::Borrowed(d) => d,
		}
	}
}

pub trait RunData {
	fn prepared(&self) -> anyhow::Result<Prepared<'_>>;
	fn target_names(&self) -> Vec<String>;
	fn raw_rows(&self) -> Option<Vec<Vec<String>>>;
	fn raw_headers(&self) -> Option<Vec<String>>;
	fn infer_only(&self) -> bool;
}

impl RunData for Dataset {
	fn prepared(&self) -> anyhow::Result<Prepared<'_>> {
		Ok(Prepared::Borrowed(self))
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
	fn prepared(&self) -> anyhow::Result<Prepared<'_>> {
		let ds = self
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("no test dataset — use .test() or .split()"))?;
		Ok(Prepared::Borrowed(ds))
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
	model: *const ModelInner,
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
	pub(crate) net: Option<crate::wire::Net>,
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
			net: None,
			last: RefCell::new(LastRun::default()),
		}
	}

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

	pub fn resume(mut self, path: impl SavePath) -> Train {
		self.resume = Some(path.or_default());
		self
	}

	pub fn net<'a>(mut self, nodes: impl IntoIterator<Item = &'a str>) -> Train {
		let mut net = crate::wire::Net::new();
		for alias in nodes {
			net = net.node(alias);
		}
		self.net = Some(net);
		self
	}


	pub fn run(&self, data: &dyn RunData, model: &Model) -> &Train {
		let handle = model;
		let model: &ModelInner = &model.inner;
		let run_hip = self.metrics.contains(&Metric::Hip).then(gpu_core::callspy::snapshot);
		let run_state = gpu_core::callspy::snapshot();
		let prepared = match data.prepared() {
			Err(e) if e.downcast_ref::<pantry::encode::CeilingExceeded>().is_some() => {
				eprintln!("\x1b[33mskipped\x1b[0m  scenario exceeds the VRAM+RAM+disk ceiling (size above)");
				return self;
			}
			other => {
				assert!(other.is_ok(), "run: prepare data: {}", other.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let Ok(v) = other else { loop {} };
				v
			}
		};
		let ds = prepared.get();
		let forward_only = data.infer_only() || !ds.has_target || self.epochs == 0;
		let conns: Option<std::sync::Arc<Vec<crate::wire::Conn>>> = match self.net.as_ref() {
			Some(net) => {
				let cs = net.connect();
				assert!(cs.is_ok(), "net: connect: {}", cs.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let Ok(cs) = cs else { loop {} };
				for c in &cs {
					eprintln!(
						"\x1b[33mnet\x1b[0m  pooled {} ({} RAM)",
						c.info.arch,
						crate::data::human_bytes(c.info.ram as usize),
					);
				}
				Some(std::sync::Arc::new(cs))
			}
			None => None,
		};
		let net_ram: usize = conns.as_ref().map_or(0, |cs| {
			cs.iter().map(|c| (c.info.ram as usize).saturating_sub(crate::ooc::USER_GB)).sum()
		});
		let issues = preflight(model, ds, forward_only, net_ram);
		if !issues.is_empty() && !confirm_issues(&issues) {
			eprintln!("\x1b[33maborted\x1b[0m");
			return self;
		}
		if !forward_only {
			let resume = self.resume.as_deref().map(Self::resolve);
			if let Some(a) = run_hip.as_ref() {
				eprint!("-- run pre-fit --\n{}", gpu_core::callspy::report_since(a));
			}
			let __fit = model.fit(ds, self, resume.as_deref(), conns);
			assert!(__fit.is_ok(), "run: fit: {}", __fit.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let post_fit = run_hip.map(|_| gpu_core::callspy::snapshot());
			if INTERRUPTED.load(Ordering::SeqCst) {
				eprintln!("\x1b[33minterrupted\x1b[0m");
			}
			let score = model.fit_score.get();
			if let Some(p) = post_fit.as_ref() {
				eprint!("-- run post-fit --\n{}", gpu_core::callspy::report_since(p));
			}
			model.arena_gen.set(gpu_core::memory::live_parked_gen());
			let mut last = self.last.borrow_mut();
			last.model = model as *const ModelInner;
			last.score = score;
			last.preds = None;
			last.n = ds.x.nrows();
			last.k = ds.n_targets.max(1);
			last.target_names = data.target_names();
			last.raw_test_rows = data.raw_rows();
			last.raw_test_headers = data.raw_headers();
			drop(last);
			if let Some(t) = gpu_core::callspy::state_report(&run_state) {
				eprint!("{t}");
			}
		} else {
			let arena = handle.begin_forward();
			let (xbuf, x_cat, n) = model.prep_eval_input(ds);
			let params = model.params.borrow();
			assert!(!params.is_empty(), "run: call train first");
			let k = params[params.len() - 1].out_dim;
			let yscaler = *model.yscaler.borrow();
			let (score, preds) = if ds.has_target && !self.metrics.is_empty() {
				let ybuf = {
					let __up = ds.y.as_slice();
					assert!(__up.is_some(), "run: eval metrics: y contig");
					let Some(__up) = __up else { loop {} };
					let __ub = GpuBuffer::alloc(__up.len());
					assert!(__ub.is_ok(), "run: eval metrics: ybuf: {}", __ub.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
					let Ok(__ub) = __ub else { loop {} };
					let __ld = __ub.load(__up);
					assert!(__ld.is_ok(), "run: eval metrics: ybuf: {}", __ld.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
					__ub
				};
				let total = (n * k) as f64;
				let ybar = ds.y.iter().sum::<f64>() / total;
				let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
				let __sc = infer_scored(
					&params, &xbuf, x_cat.as_ref(), n, yscaler, Some(&ybuf),
					model.loss, model.lr, &self.metrics, ss_tot,
				);
				assert!(__sc.is_ok(), "run: eval metrics: {}", __sc.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let Ok((preds, vals)) = __sc else { loop {} };
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
				let __sc = infer_scored(
					&params, &xbuf, x_cat.as_ref(), n, yscaler, None,
					model.loss, model.lr, &[], 0.0,
				);
				assert!(__sc.is_ok(), "run: eval predictions: {}", __sc.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
				let Ok((preds, _)) = __sc else { loop {} };
				(f64::NAN, preds)
			};
			let mut last = self.last.borrow_mut();
			last.model = model as *const ModelInner;
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
			drop(last);
			drop(params);
			handle.end_forward(arena);
		}
		self
	}

	pub fn save(&self, path: impl SavePath) -> &Train {
		self.save_ogdl(None, &path.or_default());
		self
	}

	fn save_ogdl(&self, filter: Option<&[Param]>, path: &str) {
		let last = self.last.borrow();
		if last.model.is_null() {
			return;
		}
		let model = unsafe { &*last.model };
		let key = model.loss.score_key();
		let score = last.score;
		let path = Self::resolve(path);
		if !score.is_finite()
			|| recipe_infer::saved_score(&path, key).is_some_and(|best| score <= best)
		{
			return;
		}
		let mirror = model.saved_ogdl.borrow();
		let (text, neurons) = if let Some(m) = mirror.as_ref() {
			(m.text.clone(), m.neurons)
		} else {
			let params = model.params.borrow();
			assert!(!params.is_empty(), "save: model has no trained params");
			(
				recipe_infer::dump_ogdl(&params, filter, key, score),
				params.iter().map(|p| p.out_dim).sum::<usize>(),
			)
		};
		let __wr = recipe_infer::write_ogdl(&path, &text);
		assert!(__wr.is_ok(), "write model file: {}", __wr.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
		let full = std::fs::canonicalize(&path).unwrap_or_else(|_| path.as_str().into());
		eprintln!("saved {} ({neurons} neurons, {key} {score:.4})", full.display());
	}

}

impl Default for Train {
	fn default() -> Self {
		Self::new()
	}
}

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

#[doc(hidden)]
pub(crate) struct SavedWeights {
	pub(crate) text: String,
	pub(crate) neurons: usize,
	pub(crate) d: usize,
	pub(crate) c_cat: usize,
	pub(crate) vocab: usize,
}

pub(crate) struct ModelInner {
	pub(crate) specs: Vec<LayerSpec>,
	pub(crate) loss: Loss,
	pub(crate) lr: f64,
	pub(crate) params: RefCell<Vec<LayerParams>>,
	pub(crate) scaler: RefCell<Option<Scaler>>,
	pub(crate) yscaler: RefCell<Option<(f64, f64)>>,
	pub(crate) fit_score: Cell<f64>,
	pub(crate) saved_ogdl: RefCell<Option<SavedWeights>>,
	pub(crate) arena_gen: Cell<Option<usize>>,
	pub(crate) rebuild_backing: RefCell<Option<GpuBuffer>>,
}

pub struct Model {
	pub(crate) inner: Box<ModelInner>,
}

struct Issue {
	what: String,
	have: String,
	need: String,
}

pub(crate) fn plan_footprint(model: &ModelInner, ds: &Dataset, forward_only: bool) -> usize {
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let k = ds.n_targets.max(1);
	let embed_first = matches!(model.specs.first(), Some(LayerSpec::Embed(..)));
	let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
	let (cat_cols, text_d, vocab) = if embed_cats {
		let n_oh: usize = ds.onehot_groups.iter().map(|(_, len)| len).sum();
		(d - n_oh, ds.onehot_groups.len(), n_oh)
	} else if embed_first {
		let tc = ds.text_cols.len();
		let vocab = pinned_vocab(&model.specs)
			.unwrap_or_else(|| ds.x.iter().cloned().fold(0.0f64, f64::max) as usize + 1);
		(d - tc, tc, vocab)
	} else {
		(0, d, 0)
	};
	let base = vram_estimate(&model.specs, n, text_d, k, vocab, cat_cols, forward_only);
	if forward_only {
		return base;
	}
	let d_sc = if embed_first { cat_cols } else { d };
	let zscore_transient = if d_sc > 0 {
		n * d_sc * 8 + gpu_core::kernels::gpu_reduce_sum_cols_workspace_bytes(n, d_sc)
	} else {
		0
	};
	base + zscore_transient
}

impl ModelInner {
	pub(crate) fn ensure_params_live(&self) {
		let Some(g) = self.arena_gen.get() else {
			return;
		};
		if gpu_core::memory::live_parked_gen() == Some(g) {
			return;
		}
		let params = {
			let mirror = self.saved_ogdl.borrow();
			assert!(
				mirror.is_some(),
				"eval: this model's device weights were freed by a later training run and \
				 there is no host mirror to restore them (pooled out-of-core arena run)",
			);
			let Some(m) = mirror.as_ref() else { loop {} };
			let __saved = load_ogdl_str(&m.text);
			assert!(__saved.is_ok(), "eval: parse host weight mirror: {}", __saved.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok(saved) = __saved else { loop {} };
			let __plan = plan_layer_params(&self.specs, m.d, m.c_cat, m.vocab, &saved, true);
			assert!(__plan.is_ok(), "eval: rebuild weights from mirror: {}", __plan.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok(plan) = __plan else { loop {} };
			let host = plan.host();
			let __staged = GpuBuffer::alloc(host.len().max(1));
			assert!(__staged.is_ok(), "rebuild staged alloc: {}", __staged.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok(staged) = __staged else { loop {} };
			let __sl = staged.load(host);
			assert!(__sl.is_ok(), "rebuild staged load: {}", __sl.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let params = plan.materialize(&staged, 0);
			*self.rebuild_backing.borrow_mut() = Some(staged);
			params
		};
		*self.params.borrow_mut() = params;
		self.arena_gen.set(gpu_core::memory::live_parked_gen());
	}
}

fn preflight(model: &ModelInner, ds: &Dataset, forward_only: bool, net_ram: usize) -> Vec<Issue> {
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

	let (mut free_vram, mut total_vram) = (0usize, 0usize);
	unsafe { gpu_core::hip::hipMemGetInfo(&mut free_vram, &mut total_vram) };
	let need = plan_footprint(model, ds, forward_only);
	if need > free_vram {
		let mode = if forward_only { "inference" } else { "training" };
		match (forward_only, crate::ooc::plan(need, net_ram)) {
			(false, Some(p)) => {
				let net_part = if p.remote > 0 {
					format!(" + NET {}", crate::data::human_bytes(p.remote))
				} else {
					String::new()
				};
				eprintln!(
					"\x1b[33mwaterfall\x1b[0m  scratch {} -> VRAM {} + RAM {} + DISK {}{net_part}",
					crate::data::human_bytes(need),
					crate::data::human_bytes(p.vram),
					crate::data::human_bytes(p.ram),
					crate::data::human_bytes(p.disk),
				);
			}
			_ => {
				issues.push(Issue {
					what: format!("{mode} on {n} rows × {d} features exceeds {}", if forward_only { "GPU memory" } else { "VRAM+RAM+DISK" }),
					have: format!("{} free of {} total", crate::data::human_bytes(free_vram), crate::data::human_bytes(total_vram)),
					need: format!("{}", crate::data::human_bytes(need)),
				});
			}
		}
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
			inner: Box::new(ModelInner {
				specs: Vec::new(),
				loss: Loss::Mse,
				lr: 0.01,
				params: RefCell::new(Vec::new()),
				scaler: RefCell::new(None),
				yscaler: RefCell::new(None),
				fit_score: Cell::new(f64::NAN),
				saved_ogdl: RefCell::new(None),
				arena_gen: Cell::new(None),
				rebuild_backing: RefCell::new(None),
			}),
		}
	}

	pub fn load(weights: &str, proto: Model, d: usize) -> Model {
		let __saved = load_ogdl_str(weights);
		assert!(__saved.is_ok(), "Model::load: parse weights: {}", __saved.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
		let Ok(saved) = __saved else { loop {} };
		let inner = &proto.inner;
		let __vocab = pinned_vocab(&inner.specs);
		assert!(__vocab.is_some(), "Model::load: first embed layer must pin a fixed vocab (embed(dim).vocab(v))");
		let Some(vocab) = __vocab else { loop {} };
		let __plan = plan_layer_params(&inner.specs, d, 0, vocab, &saved, true);
		assert!(__plan.is_ok(), "Model::load: plan layer params: {}", __plan.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
		let Ok(plan) = __plan else { loop {} };
		let host = plan.host();
		let __staged = GpuBuffer::alloc(host.len().max(1));
		assert!(__staged.is_ok(), "Model::load staged alloc: {}", __staged.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
		let Ok(staged) = __staged else { loop {} };
		let __sl = staged.load(host);
		assert!(__sl.is_ok(), "Model::load staged load: {}", __sl.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
		let params = plan.materialize(&staged, 0);
		*inner.rebuild_backing.borrow_mut() = Some(staged);
		*inner.params.borrow_mut() = params;
		*inner.scaler.borrow_mut() = Some(Scaler { mean: vec![], std: vec![] });
		*inner.yscaler.borrow_mut() = None;
		proto
	}

	pub fn layer(mut self, spec: impl IntoLayer) -> Model {
		self.inner.specs.push(spec.into_layer());
		self
	}

	fn last_activation_slot(&mut self) -> Option<&mut Activation> {
		match self.inner.specs.last_mut() {
			Some(LayerSpec::Dense(_, a)) | Some(LayerSpec::Conv(_, _, _, a)) => Some(a),
			_ => None,
		}
	}

	fn set_last_activation(mut self, act: Activation) -> Model {
		let __slot = self.last_activation_slot();
		assert!(__slot.is_some(), "activation method called but last layer is not dense or conv");
		let Some(__slot) = __slot else { loop {} };
		*__slot = act;
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

	pub fn conv(mut self, filters: usize, kernel: usize, stride: usize) -> Model {
		self.inner.specs.push(LayerSpec::Conv(filters, kernel, stride, Activation::Linear));
		self
	}

	pub fn loss(mut self, loss: Loss) -> Model {
		self.inner.loss = loss;
		self
	}

	pub fn lr(mut self, lr: f64) -> Model {
		self.inner.lr = lr;
		self
	}

	pub fn eval(&self, data: &dyn RunData) -> Vec<f64> {
		let inner = &self.inner;
		let __prep = data.prepared();
		assert!(__prep.is_ok(), "eval: prepare data: {}", __prep.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
		let Ok(prepared) = __prep else { loop {} };
		let ds = prepared.get();
		let arena = self.begin_forward();
		let (xbuf, x_cat, n) = inner.prep_eval_input(ds);
		let params = inner.params.borrow();
		assert!(!params.is_empty(), "eval: call train() first");
		let yscaler = *inner.yscaler.borrow();
		let metric = if inner.loss.is_classification() { Metric::Accuracy } else { Metric::R2 };
		let preds = if ds.has_target {
			let __ys = ds.y.as_slice();
			assert!(__ys.is_some(), "eval: y contiguous");
			let Some(yslice) = __ys else { loop {} };
			let __yb = GpuBuffer::alloc(yslice.len());
			assert!(__yb.is_ok(), "eval ybuf: {}", __yb.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok(ybuf) = __yb else { loop {} };
			let __yl = ybuf.load(yslice);
			assert!(__yl.is_ok(), "eval ybuf load: {}", __yl.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let k = params[params.len() - 1].out_dim;
			let total = (n * k) as f64;
			let ybar = ds.y.iter().sum::<f64>() / total;
			let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
			let __sc = infer_scored(
				&params, &xbuf, x_cat.as_ref(), n, yscaler, Some(&ybuf),
				inner.loss, inner.lr, std::slice::from_ref(&metric), ss_tot,
			);
			assert!(__sc.is_ok(), "eval: metrics: {}", __sc.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok((preds, vals)) = __sc else { loop {} };
			let label = if inner.loss.is_classification() { "accuracy" } else { "R2" };
			eprintln!("eval: {label} = {:.4} ({n} samples)", vals[0]);
			preds
		} else {
			let __sc = infer_scored(
				&params, &xbuf, x_cat.as_ref(), n, yscaler, None,
				inner.loss, inner.lr, &[], 0.0,
			);
			assert!(__sc.is_ok(), "eval: predictions: {}", __sc.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default());
			let Ok((preds, _)) = __sc else { loop {} };
			eprintln!("eval: {n} samples (no target column, score unavailable)");
			preds
		};
		drop(params);
		self.end_forward(arena);
		preds
	}

	pub(crate) fn begin_forward(&self) -> Option<GpuBuffer> {
		let slab = self.inner.arena_gen.get().and_then(|_| gpu_core::memory::adopt_run_backing(0));
		self.inner.ensure_params_live();
		slab
	}

	pub(crate) fn end_forward(&self, slab: Option<GpuBuffer>) {
		if let Some(slab) = slab {
			gpu_core::memory::park_run_backing(slab);
			self.inner.arena_gen.set(gpu_core::memory::live_parked_gen());
		}
	}
}

impl Default for Model {
	fn default() -> Self {
		Self::new()
	}
}

