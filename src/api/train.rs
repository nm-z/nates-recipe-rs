use crate::api::{InferOnly, ModelArg, RunArg, SavePath};
use ogdl::log::Write;
use recipe_ir::{LogItem, Slot};
use recipe_runtime::execute::{LastRun, ModelInner, RunMeta, TrainCfg};
use std::cell::RefCell;

pub struct Train {
	pub(crate) cfg: TrainCfg,
}

impl Train {
	pub fn new() -> Train {
		Train {
			cfg: TrainCfg {
				epochs: Slot::Unspecified,
				log_every: Slot::Unspecified,
				metrics: Vec::new(),
				device: false,
				plot: Vec::new(),
				resume: None,
				net: None,
				last: RefCell::new(LastRun::default()),
				plot_render: None,
			},
		}
	}

	pub fn epochs(mut self, n: usize) -> Train {
		self.cfg.epochs = Slot::Given(n);
		self
	}

	pub fn log_every(mut self, every: usize) -> Train {
		self.cfg.log_every = Slot::Given(every);
		self
	}

	pub fn log(mut self, items: impl IntoIterator<Item = LogItem>) -> Train {
		for item in items {
			match item {
				LogItem::Metric(m) => self.cfg.metrics.push(m),
				LogItem::Device => self.cfg.device = true,
			}
		}
		self
	}

	pub fn plot(mut self, items: impl IntoIterator<Item = LogItem>) -> Train {
		self.cfg.plot = items
			.into_iter()
			.filter_map(|item| match item {
				LogItem::Metric(m) => Some(m),
				LogItem::Device => None,
			})
			.collect();
		self.cfg.plot_render = None;
		if !self.cfg.plot.is_empty() {
			self.cfg.plot_render = Some(crate::cli::tui::show);
		}
		self
	}

	pub fn resume(mut self, path: impl SavePath) -> Train {
		self.cfg.resume = Some(path.or_default());
		self
	}

	pub fn net<'a>(mut self, nodes: impl IntoIterator<Item = &'a str>) -> Train {
		let mut wnet = recipe_runtime::transport::Net::new();
		for alias in nodes {
			wnet = wnet.node(alias);
		}
		self.cfg.net = Some(wnet);
		self
	}

	pub fn run(&self, dat: impl RunArg, model: impl ModelArg) -> &Train {
		let probe = recipe_runtime::execute::run_begin(&self.cfg);
		let dh = dat.resolve();
		let mh = model.resolve();
		let d = dh.get();
		let m = mh.get();
		let prepared = match d.prepared() {
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
		let fit_ok = matches!(d.infer_only(), InferOnly::Fit)
			&& self.cfg.resolved_epochs() >= 1
			&& ds.has_target;
		let meta = RunMeta {
			target_names: d.target_names(),
			raw_test_rows: d.raw_rows(),
			raw_test_headers: d.raw_headers(),
			fit_ok,
		};
		let inner: &ModelInner = &m.inner;
		recipe_runtime::execute::run_train(&self.cfg, ds, inner, meta, probe);
		self
	}

	pub fn score(&self) -> f64 {
		return self.cfg.last.borrow().score;
	}

	pub fn loss(&self) -> f64 {
		return self.cfg.last.borrow().loss;
	}

	pub fn save(&self, path: impl SavePath) -> &Train {
		let model_ptr = self.cfg.last.borrow().model;
		let Some(_stale) = Some(()).filter(|_probe| !model_ptr.is_null()) else {
			return self;
		};
		// SAFETY: model_ptr was set from a &ModelInner in run and the caller
		let inner = unsafe { &*model_ptr };
		let score = self.cfg.last.borrow().score;
		recipe_runtime::execute::save_ogdl(inner, score, None, &path.or_default());
		self
	}
}

impl Default for Train {
	fn default() -> Self {
		Self::new()
	}
}
