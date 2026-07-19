use crate::dataset::Dataset;
use crate::train::INTERRUPTED;
use anyhow::Context;
use gpu_core::log::{
	Flag, Opt, Write, acc, chat, data, device, epoch, gpu, loss, lr, net, probe, prompt, r2,
	save, set_opt, time,
};
use gpu_core::memory::GpuBuffer;
use recipe_infer::{
	LayerParams, PlanMode, Scaler, infer_scored, load_ogdl_str, pinned_vocab, plan_layer_params,
	vram_estimate,
};
use std::cell::{Cell, RefCell};
use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::mem;
use std::path::Path;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub use recipe_infer::{
	Accuracy, Activation, Epoch, LayerSpec, Loss, Lr, Metric, R2, Time, bce, ce, elu, focal,
	gelu, hip, huber, leak, linear, mae, mse, prelu, relu, selu, sig, silu, swish, tanh,
};

pub trait IntoLayer {
	fn into_layer(self) -> LayerSpec;
}
impl IntoLayer for usize {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense(self, Activation::Linear)
	}
}

pub struct DenseSpec {
	pub units: usize,
	pub act: Activation,
}
impl IntoLayer for DenseSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Dense(self.units, self.act)
	}
}

pub struct EmbedSpec {
	dim: usize,
	vocab: Option<usize>,
}
pub fn embed(dim: usize) -> EmbedSpec {
	EmbedSpec { dim, vocab: None }
}
impl EmbedSpec {
	pub fn vocab(mut self, v: usize) -> EmbedSpec {
		self.vocab = Some(v);
		self
	}
}
impl IntoLayer for EmbedSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Embed(self.dim, self.vocab)
	}
}

pub struct AttnSpec {
	heads: usize,
}
pub fn attn(heads: usize) -> AttnSpec {
	AttnSpec { heads }
}
impl IntoLayer for AttnSpec {
	fn into_layer(self) -> LayerSpec {
		LayerSpec::Attn(self.heads)
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

impl<'a> Prepared<'a> {
	pub fn get(&self) -> &Dataset {
		match self {
			Prepared::Owned(d) => d,
			Prepared::Borrowed(d) => d,
		}
	}
}

pub enum InferOnly {
	Fit,
	Forward,
}

pub trait RunData {
	fn prepared<'a>(&'a self) -> anyhow::Result<Prepared<'a>>;
	fn target_names(&self) -> Vec<String>;
	fn raw_rows(&self) -> Option<Vec<Vec<String>>>;
	fn raw_headers(&self) -> Option<Vec<String>>;
	fn infer_only(&self) -> InferOnly;
}

impl RunData for Dataset {
	fn prepared<'a>(&'a self) -> anyhow::Result<Prepared<'a>> {
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
	fn infer_only(&self) -> InferOnly {
		InferOnly::Fit
	}
}

impl RunData for Option<Dataset> {
	fn prepared<'a>(&'a self) -> anyhow::Result<Prepared<'a>> {
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
	fn infer_only(&self) -> InferOnly {
		InferOnly::Forward
	}
}

pub enum DataHandle<'a> {
	Ref(&'a dyn RunData),
	Parked(crate::dataset::Data),
}

impl DataHandle<'_> {
	pub fn get(&self) -> &dyn RunData {
		match self {
			DataHandle::Ref(d) => *d,
			DataHandle::Parked(d) => d,
		}
	}
}

pub trait RunArg {
	fn resolve(&self) -> DataHandle<'_>;
}

impl RunArg for &crate::dataset::Data {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl RunArg for &Dataset {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl RunArg for &Option<Dataset> {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl RunArg for &dyn RunData {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl<F: for<'a> Fn(&'a str) -> crate::dataset::Data> RunArg for F {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Parked(crate::dataset::parked_data())
	}
}

pub enum ModelHandle<'a> {
	Ref(&'a Model),
	Parked(Model),
}

impl ModelHandle<'_> {
	pub fn get(&self) -> &Model {
		match self {
			ModelHandle::Ref(m) => m,
			ModelHandle::Parked(m) => m,
		}
	}
}

pub trait ModelArg {
	fn resolve(&self) -> ModelHandle<'_>;
}

impl ModelArg for &Model {
	fn resolve(&self) -> ModelHandle<'_> {
		ModelHandle::Ref(self)
	}
}

impl<F: Fn() -> Model> ModelArg for F {
	fn resolve(&self) -> ModelHandle<'_> {
		ModelHandle::Parked(parked_model())
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
			model: ptr::null(),
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

struct ScorePreds {
	score: f64,
	preds: Vec<f64>,
}

struct Rendered {
	text: String,
	neurons: usize,
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
		let raw = match path {
			"" => "model.ogdl".to_string(),
			"*" => env::current_exe()
				.ok()
				.and_then(|e| e.parent().map(|d| d.join("model.ogdl")))
				.map(|p| p.display().to_string())
				.unwrap_or_else(|| "model.ogdl".to_string()),
			_other => path.to_string(),
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
		let mut wnet = crate::wire::Net::new();
		for alias in nodes {
			wnet = wnet.node(alias);
		}
		self.net = Some(wnet);
		self
	}

	pub fn run(&self, dat: impl RunArg, model: impl ModelArg) -> &Train {
		let dh = dat.resolve();
		let mh = model.resolve();
		self.run_on(dh.get(), mh.get())
	}

	fn run_on(&self, dat: &dyn RunData, model: &Model) -> &Train {
		let model: &ModelInner = &model.inner;
		let run_hip = self
			.metrics
			.iter()
			.find(|m| **m == Metric::Hip)
			.map(|_hip| gpu_core::callspy::snapshot());
		let run_state = gpu_core::callspy::snapshot();
		set_opt(Opt {
			loss: self.metrics.contains(&Metric::Loss),
			acc: self.metrics.contains(&Metric::Accuracy),
			epoch: self.metrics.contains(&Metric::Epoch),
			lr: self.metrics.contains(&Metric::Lr),
			time: self.metrics.contains(&Metric::Time),
			r2: self.metrics.contains(&Metric::R2),
			device: run_hip.is_some(),
			save: !self.metrics.is_empty(),
			prompt: true,
			..Opt::default()
		});
		let prepared = match dat.prepared() {
			Ok(v) => v,
			Err(e) => {
				if e.downcast_ref::<pantry::encode::CeilingExceeded>()
					.is_none()
				{
					Write::error(format!("run: prepare data: {e:#}"));
					return self;
				}
				Write::error(
					"skipped  scenario exceeds the VRAM+RAM+disk ceiling (size above)",
				);
				return self;
			}
		};
		let ds = prepared.get();
		if !(matches!(dat.infer_only(), InferOnly::Fit) && self.epochs >= 1 && ds.has_target) {
			Write::error(
				"run: forward-only data — Train is fit-only; use recipe.infer().run(&model).eval(&data)",
			);
			return self;
		}
		let conns: Option<Arc<Vec<crate::wire::Conn>>> = match self.net.as_ref() {
			Some(wnet) => {
				let cs = match wnet.connect() {
					Ok(v) => v,
					Err(e) => {
						Write::error(format!("net: connect: {e:#}"));
						return self;
					}
				};
				for c in &cs {
					Write::line(
						net,
						&format!(
							"net  pooled {} ({} RAM)",
							c.info.arch,
							crate::data::human_bytes(c.info.ram as usize),
						),
					);
				}
				Some(Arc::new(cs))
			}
			None => None,
		};
		let net_ram: usize = conns.as_ref().map_or(0, |cs| {
			cs.iter()
				.map(|c| (c.info.ram as usize).saturating_sub(crate::ooc::USER_GB))
				.sum()
		});
		let issues = preflight(model, ds, net_ram);
		let Gate::Proceed = confirm_issues(&issues) else {
			Write::error("aborted");
			return self;
		};
		{
			{
				let resume = self.resume.as_deref().map(Self::resolve);
				run_hip
					.as_ref()
					.map(|a| {
						Write::line(device, "-- run pre-fit --");
						Write::block(
							device,
							&gpu_core::callspy::report_since(a).serialize(),
						)
					})
					.unwrap_or(());
				let __fit = model.fit(ds, self, resume.as_deref(), conns);
				if !__fit.is_ok() {
					Write::error(format!(
						"run: fit: {}",
						__fit.as_ref()
							.err()
							.map(|e| format!("{e:#}"))
							.unwrap_or_default()
					));
					model.fit_score.set(f64::NAN);
					let nan_vals: Vec<f64> = vec![f64::NAN; self.metrics.len()];
					Write::always(crate::train::metrics_line(&self.metrics, &nan_vals));
				}
				let post_fit = run_hip.map(|_snap| gpu_core::callspy::snapshot());
				Some(())
					.filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) != 0)
					.map(|_flag| Write::error("interrupted"))
					.unwrap_or(());
				let score = model.fit_score.get();
				post_fit
					.as_ref()
					.map(|p| {
						Write::line(device, "-- run post-fit --");
						Write::block(
							device,
							&gpu_core::callspy::report_since(p).serialize(),
						)
					})
					.unwrap_or(());
				model.arena_gen.set(gpu_core::memory::live_parked_gen());
				let mut last = self.last.borrow_mut();
				last.model = model as *const ModelInner;
				last.score = score;
				last.preds = None;
				last.n = ds.x.nrows();
				last.k = ds.n_targets.max(1);
				last.target_names = dat.target_names();
				last.raw_test_rows = dat.raw_rows();
				last.raw_test_headers = dat.raw_headers();
				drop(last);
				if let Some((tree, errs)) = gpu_core::callspy::state_report(&run_state) {
					Write::block(device, &tree.serialize());
					for e in errs {
						Write::error(&e);
					}
				}
			}
		}
		self
	}

	pub fn save(&self, path: impl SavePath) -> &Train {
		self.save_ogdl(None, &path.or_default());
		self
	}

	fn save_ogdl(&self, filter: Option<&[Param]>, path: &str) {
		let last = self.last.borrow();
		let Some(_stale) = Some(()).filter(|_probe| !last.model.is_null()) else {
			return;
		};
		// SAFETY: last.model was set from a &ModelInner in run/infer and the caller
		let model = unsafe { &*last.model };
		let key = model.loss.score_key();
		let score = last.score;
		let path = Self::resolve(path);
		let allow = score.is_finite()
			&& !recipe_infer::saved_score(&path, key).is_some_and(|best| score <= best);
		let Some(_ok) = Some(()).filter(|_probe| allow) else {
			return;
		};
		let mirror = model.saved_ogdl.borrow();
		let rendered = match mirror.as_ref() {
			Some(m) => Rendered {
				text: m.text.clone(),
				neurons: m.neurons,
			},
			None => {
				let params = model.params.borrow();
				if params.is_empty() {
					Write::error("save: model has no trained params");
					return;
				}
				let Ok(text) = recipe_infer::dump_ogdl(&params, filter, key, score) else {
					return;
				};
				Rendered {
					text,
					neurons: params.iter().map(|p| p.out_dim).sum::<usize>(),
				}
			}
		};
		let __wr = recipe_infer::write_ogdl(&path, &rendered.text);
		if !__wr.is_ok() {
			Write::error(format!(
				"write model file: {}",
				__wr.as_ref()
					.err()
					.map(|e| format!("{e:#}"))
					.unwrap_or_default()
			));
			return;
		}
		let full = fs::canonicalize(&path).unwrap_or_else(|_err| path.as_str().into());
		Write::line(
			save,
			&format!(
				"saved {} ({} neurons, {key} {score:.4})",
				full.display(),
				rendered.neurons
			),
		);
	}
}

impl Default for Train {
	fn default() -> Self {
		Self::new()
	}
}

pub struct Infer {
	flags: Vec<Flag>,
	last: RefCell<LastRun>,
}

impl Infer {
	pub fn new() -> Infer {
		Infer {
			flags: Vec::new(),
			last: RefCell::new(LastRun::default()),
		}
	}

	pub fn log(mut self, flags: impl IntoIterator<Item = Flag>) -> Infer {
		self.flags = flags.into_iter().collect();
		self
	}

	pub fn preds(&self) -> Vec<f64> {
		self.last.borrow().preds.clone().unwrap_or_else(|| {
			Write::error("infer: no predictions — run .eval(&data) first");
			Vec::new()
		})
	}

	pub fn run(&self, model: impl ModelArg) -> &Infer {
		let mh = model.resolve();
		self.run_on(mh.get())
	}

	fn run_on(&self, model: &Model) -> &Infer {
		let has = |f: Flag| self.flags.contains(&f);
		set_opt(Opt {
			loss: has(loss),
			acc: has(acc),
			epoch: has(epoch),
			lr: has(lr),
			time: has(time),
			r2: has(r2),
			device: has(device),
			data: has(data),
			gpu: has(gpu),
			probe: has(probe),
			net: has(net),
			chat: has(chat),
			prompt: true,
			save: !self.flags.is_empty(),
		});
		match &model.inner.gguf {
			Some(path) => {
				if env::var_os("VRAM_PROBE").is_some() || env::var_os("RAM_PROBE").is_some() {
					Write::error(
						"infer: VRAM_PROBE/RAM_PROBE set: the binary's main must call recipe_infer::llm::vram_probe_ask() and gpu_core::memory::ram_probe_ask() and exit with the code before run()",
					);
					return self;
				}
				match Some(()).filter(|_probe| has(chat)) {
					Some(_chat) => {
						crate::tui::chat(path);
						recipe_infer::shutdown();
					}
					None => {
						let prompt_text = env::args().nth(1).unwrap_or_else(|| {
							"The capital of France is".to_string()
						});
						let out = recipe_infer::llm::generate(
							Path::new(path),
							&prompt_text,
							&mut |toks| {
								Write::line(
									prompt,
									recipe_infer::llm::render_toks(toks),
								);
								true
							},
						);
						match out {
							Ok(text) => Write::line(prompt, text),
							Err(e) => Write::error(format!("infer: generate: {e:#}")),
						}
						recipe_infer::shutdown();
					}
				}
			}
			None => {
				let inner: &ModelInner = &model.inner;
				let mut last = self.last.borrow_mut();
				last.model = inner as *const ModelInner;
				last.preds = None;
			}
		}
		self
	}

	pub fn eval(&self, dat: impl RunArg) -> &Infer {
		let dh = dat.resolve();
		self.eval_on(dh.get())
	}

	fn eval_on(&self, dat: &dyn RunData) -> &Infer {
		let last_model = {
			let last = self.last.borrow();
			if last.model.is_null() {
				Write::error("infer: call run(&model) first");
				return self;
			}
			last.model
		};
		// SAFETY: last_model was set from a &ModelInner in run/infer and the caller
		let model: &ModelInner = unsafe { &*last_model };
		let metrics: Vec<Metric> = self
			.flags
			.iter()
			.filter_map(|f| match f {
				Flag::loss => Some(Metric::Loss),
				Flag::acc => Some(Metric::Accuracy),
				Flag::epoch => Some(Metric::Epoch),
				Flag::lr => Some(Metric::Lr),
				Flag::time => Some(Metric::Time),
				Flag::r2 => Some(Metric::R2),
				Flag::device => Some(Metric::Hip),
				_other => None,
			})
			.collect();
		let prepared = match dat.prepared() {
			Ok(v) => v,
			Err(e) => {
				Write::error(format!("eval: prepare data: {e:#}"));
				return self;
			}
		};
		let ds = prepared.get();
		let arena = match model.begin_forward() {
			Ok(a) => a,
			Err(e) => {
				Write::error(format!("eval: {e:#}"));
				return self;
			}
		};
		let ei = match model.prep_eval_input(ds) {
			Ok(v) => v,
			Err(e) => {
				Write::error(format!("{e:#}"));
				model.end_forward(arena);
				return self;
			}
		};
		let params = model.params.borrow();
		if params.is_empty() {
			Write::error("eval: call train first");
			return self;
		}
		let k = params[params.len() - 1].out_dim;
		let yscaler = *model.yscaler.borrow();
		let sp = match Some(()).filter(|_probe| ds.has_target && !metrics.is_empty()) {
			Some(_scored) => {
				let ybuf = {
					let Some(__up) = ds.y.as_slice() else {
						Write::error("eval: metrics: y contig");
						model.end_forward(arena);
						return self;
					};
					let __ub = match GpuBuffer::alloc(__up.len()) {
						Ok(v) => v,
						Err(e) => {
							Write::error(format!("eval: metrics: ybuf: {e:#}"));
							model.end_forward(arena);
							return self;
						}
					};
					let __ld = __ub.load(__up);
					if !__ld.is_ok() {
						Write::error(format!(
							"eval: metrics: ybuf: {}",
							__ld.as_ref()
								.err()
								.map(|e| format!("{e:#}"))
								.unwrap_or_default()
						));
						model.end_forward(arena);
						return self;
					}
					__ub
				};
				let total = (ei.n * k) as f64;
				let ybar = ds.y.iter().sum::<f64>() / total;
				let ss_tot: f64 = ds.y.iter().map(|v| (v - ybar).powi(2)).sum();
				let __sc = infer_scored(
					&params,
					&ei.x,
					ei.x_cat.as_ref(),
					ei.n,
					yscaler,
					Some(&ybuf),
					model.loss,
					model.lr,
					&metrics,
					ss_tot,
				);
				let sc = match __sc {
					Ok(v) => v,
					Err(e) => {
						Write::error(format!("eval: metrics: {e:#}"));
						model.end_forward(arena);
						return self;
					}
				};
				for (mi, m) in metrics.iter().enumerate() {
					let flag = match m {
						Metric::Loss => loss,
						Metric::Accuracy => acc,
						Metric::Epoch => epoch,
						Metric::Lr => lr,
						Metric::Time => time,
						Metric::R2 => r2,
						Metric::Hip => device,
					};
					Write::line(
						flag,
						&format!(
							"eval  {}",
							crate::train::metrics_line(&[*m], &[sc.vals[mi]])
						),
					);
				}
				let stop = Some(Metric::Accuracy)
					.filter(|_m| model.loss.is_classification())
					.unwrap_or(Metric::R2);
				let score = (0..metrics.len())
					.find(|mi| metrics[*mi] == stop)
					.map_or(f64::NAN, |mi| sc.vals[mi]);
				ScorePreds {
					score,
					preds: sc.preds,
				}
			}
			None => {
				let __sc = infer_scored(
					&params,
					&ei.x,
					ei.x_cat.as_ref(),
					ei.n,
					yscaler,
					None,
					model.loss,
					model.lr,
					&[],
					0.0,
				);
				let sc = match __sc {
					Ok(v) => v,
					Err(e) => {
						Write::error(format!("eval: predictions: {e:#}"));
						model.end_forward(arena);
						return self;
					}
				};
				Write::line(
					data,
					&format!(
						"eval: {} samples (no target column, score unavailable)",
						ei.n
					),
				);
				ScorePreds {
					score: f64::NAN,
					preds: sc.preds,
				}
			}
		};
		let mut last = self.last.borrow_mut();
		last.model = model as *const ModelInner;
		last.score = sp.score;
		last.preds = Some(sp.preds);
		last.n = ei.n;
		last.k = k;
		let incoming_names = dat.target_names();
		let kept_names = mem::take(&mut last.target_names);
		last.target_names = Some(incoming_names)
			.filter(|names| !names.is_empty())
			.unwrap_or(kept_names);
		let incoming_rows = dat.raw_rows();
		let kept_rows = last.raw_test_rows.take();
		last.raw_test_rows = incoming_rows.or(kept_rows);
		let incoming_headers = dat.raw_headers();
		let kept_headers = last.raw_test_headers.take();
		last.raw_test_headers = incoming_headers.or(kept_headers);
		drop(last);
		drop(params);
		model.end_forward(arena);
		self
	}
}

impl Default for Infer {
	fn default() -> Self {
		Self::new()
	}
}

fn expand_tilde(path: &str) -> String {
	let Ok(home) = env::var("HOME") else {
		return path.to_string();
	};
	match path {
		"~" => home,
		_other => match path.strip_prefix("~/") {
			Some(rest) => format!("{home}/{rest}"),
			None => path.to_string(),
		},
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
	pub(crate) yscaler: RefCell<Option<recipe_infer::YScaler>>,
	pub(crate) fit_score: Cell<f64>,
	pub(crate) saved_ogdl: RefCell<Option<SavedWeights>>,
	pub(crate) arena_gen: Cell<Option<usize>>,
	pub(crate) rebuild_backing: RefCell<Option<GpuBuffer>>,
	pub(crate) gguf: Option<String>,
}

pub struct Model {
	pub(crate) inner: Box<ModelInner>,
}

thread_local! {
	static PARKED_MODEL: RefCell<Option<Box<ModelInner>>> =
		const { RefCell::new(None) };
}

impl Drop for Model {
	fn drop(&mut self) {
		let inner = mem::replace(&mut self.inner, Box::new(ModelInner::blank()));
		PARKED_MODEL.with(|slot| slot.borrow_mut().replace(inner));
	}
}

pub(crate) fn parked_model() -> Model {
	let inner = PARKED_MODEL.with(|slot| slot.borrow_mut().take());
	let inner = inner.unwrap_or_else(|| {
		Write::error(
			"run: no model configured — chain recipe.model().layer(…) before run(…, model)",
		);
		Box::new(ModelInner::blank())
	});
	Model { inner }
}

#[derive(Clone, Copy)]
enum Gate {
	Proceed,
	Abort,
}

struct Issue {
	what: String,
	have: String,
	need: String,
}

struct CatShape {
	cat_cols: usize,
	text_d: usize,
	vocab: usize,
}

pub(crate) fn plan_footprint(model: &ModelInner, ds: &Dataset) -> usize {
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let k = ds.n_targets.max(1);
	let embed_first = matches!(model.specs.first(), Some(LayerSpec::Embed(..)));
	let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
	let shape = Some(())
		.filter(|_probe| embed_cats)
		.map(|_probe| {
			let n_oh: usize = ds.onehot_groups.iter().map(|g| g.len).sum();
			CatShape {
				cat_cols: d - n_oh,
				text_d: ds.onehot_groups.len(),
				vocab: n_oh,
			}
		})
		.or_else(|| {
			Some(()).filter(|_probe| embed_first).map(|_probe| {
				let tc = ds.text_cols.len();
				let vocab = pinned_vocab(&model.specs).unwrap_or_else(|| {
					ds.x.iter().cloned().fold(0.0f64, f64::max) as usize + 1
				});
				CatShape {
					cat_cols: d - tc,
					text_d: tc,
					vocab,
				}
			})
		})
		.unwrap_or(CatShape {
			cat_cols: 0,
			text_d: d,
			vocab: 0,
		});
	vram_estimate(
		&model.specs,
		n,
		shape.text_d,
		k,
		shape.vocab,
		shape.cat_cols,
		false,
	)
}

impl ModelInner {
	fn blank() -> ModelInner {
		ModelInner {
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
			gguf: None,
		}
	}

	pub(crate) fn ensure_params_live(&self) -> anyhow::Result<()> {
		let Some(g) = self.arena_gen.get() else {
			return Ok(());
		};
		if gpu_core::memory::live_parked_gen() == Some(g) {
			return Ok(());
		}
		let params = {
			let mirror = self.saved_ogdl.borrow();
			let m = mirror.as_ref().ok_or_else(|| {
				anyhow::anyhow!(
					"eval: this model's device weights were freed by a later training run and \
					 there is no host mirror to restore them (pooled out-of-core arena run)"
				)
			})?;
			let saved = load_ogdl_str(&m.text).context("eval: parse host weight mirror")?;
			let plan = plan_layer_params(&self.specs, m.d, m.c_cat, m.vocab, &saved, PlanMode::Warm)
				.map_err(|e| anyhow::anyhow!("eval: rebuild weights from mirror: {e}"))?;
			let host = plan.host();
			let staged = GpuBuffer::alloc(host.len().max(1)).context("rebuild staged alloc")?;
			staged.load(host).context("rebuild staged load")?;
			let params = plan.materialize(&staged, 0);
			*self.rebuild_backing.borrow_mut() = Some(staged);
			params
		};
		*self.params.borrow_mut() = params;
		self.arena_gen.set(gpu_core::memory::live_parked_gen());
		Ok(())
	}

	pub(crate) fn begin_forward(&self) -> anyhow::Result<Option<GpuBuffer>> {
		let slab = self
			.arena_gen
			.get()
			.and_then(|_gen| gpu_core::memory::adopt_run_backing(0));
		if let Err(e) = self.ensure_params_live() {
			self.end_forward(slab);
			return Err(e);
		}
		return Ok(slab);
	}

	pub(crate) fn end_forward(&self, slab: Option<GpuBuffer>) {
		slab.map(|inner_slab| {
			gpu_core::memory::park_run_backing(inner_slab);
			self.arena_gen.set(gpu_core::memory::live_parked_gen());
		})
		.unwrap_or(());
	}
}

fn output_check(lossfn: Loss, last_out: usize, n_layers: usize, k: usize) -> Option<Issue> {
	let (want, need) = match lossfn {
		Loss::Bce | Loss::Focal => (1, "1 (.layer(1).sigmoid())".to_string()),
		Loss::Ce if k > 1 => (k, format!("{k} (one per target column)")),
		Loss::Ce | Loss::Mse | Loss::Mae | Loss::Huber => return None,
	};
	(last_out != want).then(|| Issue {
		what: format!(
			"dense layer {n_layers} outputs {last_out}, {} loss expects {want}",
			lossfn.name()
		),
		have: format!("{last_out} output units"),
		need,
	})
}

fn preflight(model: &ModelInner, ds: &Dataset, net_ram: usize) -> Vec<Issue> {
	let mut issues = Vec::new();
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let k = ds.n_targets.max(1);

	let Some(_first_spec) = model.specs.first() else {
		issues.push(Issue {
			what: "model has 0 layers".into(),
			have: "0 layers".into(),
			need: "≥1 (.layer() before .run())".into(),
		});
		return issues;
	};

	let last_out = match model.specs.last() {
		Some(LayerSpec::Dense(u, _act)) => *u,
		_other => 0,
	};
	let n_layers = model.specs.len();
	issues.extend(output_check(model.loss, last_out, n_layers, k));

	let (free_vram, total_vram) = gpu_core::hip::mem_info()
		.map(|m| (m.free, m.total))
		.unwrap_or((0, 0));
	let need = plan_footprint(model, ds);
	Some(())
		.filter(|_probe| need > free_vram)
		.map(|_probe| {
			let planned = crate::ooc::plan(need, net_ram);
			match planned {
				Some(p) => {
					let net_part = Some(())
						.filter(|_p| p.remote > 0)
						.map(|_p| {
							format!(" + NET {}", crate::data::human_bytes(p.remote))
						})
						.unwrap_or_default();
					Write::line(
						gpu,
						&format!(
							"waterfall  scratch {} -> VRAM {} + RAM {} + DISK {}{net_part}",
							crate::data::human_bytes(need),
							crate::data::human_bytes(p.vram),
							crate::data::human_bytes(p.ram),
							crate::data::human_bytes(p.disk),
						),
					);
				}
				None => {
					issues.push(Issue {
						what: format!(
							"training on {n} rows × {d} features exceeds VRAM+RAM+DISK"
						),
						have: format!(
							"{} free of {} total",
							crate::data::human_bytes(free_vram),
							crate::data::human_bytes(total_vram)
						),
						need: crate::data::human_bytes(need),
					});
				}
			}
		})
		.unwrap_or(());

	issues
}

fn confirm_issues(issues: &[Issue]) -> Gate {
	let Some(_first) = issues.first() else {
		return Gate::Proceed;
	};
	let interactive = io::stdin().is_terminal();
	for i in 0..issues.len() {
		let issue = &issues[i];
		Write::error(&format!(
			"preflight {}/{}  {}",
			i + 1,
			issues.len(),
			issue.what,
		));
		Write::error(&format!("    have: {}", issue.have));
		Write::error(&format!("    need: {}", issue.need));
	}
	let Some(_probe) = Some(()).filter(|_gate| interactive) else {
		return Gate::Abort;
	};
	use std::io::Write as _;
	Write::line(prompt, "continue anyway? [y/N] ");
	io::stderr().flush().ok();
	let mut line = String::new();
	io::stdin().read_line(&mut line).ok();
	match line.trim() {
		"y" | "Y" | "yes" | "YES" => Gate::Proceed,
		_other => Gate::Abort,
	}
}

impl Model {
	pub fn new() -> Model {
		Model {
			inner: Box::new(ModelInner::blank()),
		}
	}

	pub fn load(weights: &str, proto: Model, d: usize) -> Model {
		if weights.ends_with(".gguf") {
			let mut proto = proto;
			proto.inner.gguf = Some(weights.to_string());
			return proto;
		}
		let saved = match load_ogdl_str(weights) {
			Ok(v) => v,
			Err(e) => {
				Write::error(format!("Model::load: parse weights: {e:#}"));
				return proto;
			}
		};
		let inner = &proto.inner;
		let Some(vocab) = pinned_vocab(&inner.specs) else {
			Write::error(
				"Model::load: first embed layer must pin a fixed vocab (embed(dim).vocab(v))",
			);
			return proto;
		};
		let plan = match plan_layer_params(&inner.specs, d, 0, vocab, &saved, PlanMode::Warm) {
			Ok(v) => v,
			Err(e) => {
				Write::error(format!("Model::load: plan layer params: {e:#}"));
				return proto;
			}
		};
		let host = plan.host();
		let staged = match GpuBuffer::alloc(host.len().max(1)) {
			Ok(v) => v,
			Err(e) => {
				Write::error(format!("Model::load staged alloc: {e:#}"));
				return proto;
			}
		};
		let __sl = staged.load(host);
		if !__sl.is_ok() {
			Write::error(format!(
				"Model::load staged load: {}",
				__sl.as_ref()
					.err()
					.map(|e| format!("{e:#}"))
					.unwrap_or_default()
			));
			return proto;
		}
		let params = plan.materialize(&staged, 0);
		*inner.rebuild_backing.borrow_mut() = Some(staged);
		*inner.params.borrow_mut() = params;
		*inner.scaler.borrow_mut() = Some(Scaler {
			mean: vec![],
			std: vec![],
		});
		*inner.yscaler.borrow_mut() = None;
		proto
	}

	pub fn layer(mut self, spec: impl IntoLayer) -> Model {
		self.inner.specs.push(spec.into_layer());
		self
	}

	fn last_activation_slot(&mut self) -> Option<&mut Activation> {
		match self.inner.specs.last_mut() {
			Some(LayerSpec::Dense(_units, a)) => Some(a),
			Some(LayerSpec::Conv(_filters, _kernel, _stride, a)) => Some(a),
			_other => None,
		}
	}

	fn set_last_activation(mut self, act: Activation) -> Model {
		match self.last_activation_slot() {
			Some(slot) => *slot = act,
			None => Write::error(
				"activation method called but last layer is not dense or conv",
			),
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

	pub fn conv(mut self, filters: usize, kernel: usize, stride: usize) -> Model {
		self.inner
			.specs
			.push(LayerSpec::Conv(filters, kernel, stride, Activation::Linear));
		self
	}

	pub fn loss(mut self, lossfn: Loss) -> Model {
		self.inner.loss = lossfn;
		self
	}

	pub fn lr(mut self, rate: f64) -> Model {
		self.inner.lr = rate;
		self
	}
}

impl Default for Model {
	fn default() -> Self {
		Self::new()
	}
}
