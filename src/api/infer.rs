use crate::api::model::Model;
use crate::api::{ModelArg, RunArg, RunData};
use ogdl::log::{
	Flag, Opt, Write, acc, chat, data, device, epoch, gpu, loss, lr, net, probe, r2, set_opt,
	time,
};
use recipe_runtime::execute::{InferCfg, LastRun, ModelInner};
use std::cell::RefCell;
use std::mem;

pub struct Infer {
	pub(crate) cfg: InferCfg,
}

impl Infer {
	pub fn new() -> Infer {
		Infer {
			cfg: InferCfg {
				flags: Vec::new(),
				last: RefCell::new(LastRun::default()),
			},
		}
	}

	pub fn log(mut self, flags: impl IntoIterator<Item = Flag>) -> Infer {
		self.cfg.flags = flags.into_iter().collect();
		self
	}

	pub fn preds(&self) -> Vec<f64> {
		self.cfg.last.borrow().preds.clone().unwrap_or_else(|| {
			Write::error("infer: no predictions — run .eval(&data) first");
			Vec::new()
		})
	}

	pub fn run(&self, model: impl ModelArg) -> &Infer {
		let mh = model.resolve();
		self.run_on(mh.get())
	}

	fn run_on(&self, model: &Model) -> &Infer {
		let has = |f: Flag| self.cfg.flags.contains(&f);
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
			save: !self.cfg.flags.is_empty(),
		});
		match &model.inner.gguf {
			Some(path) => {
				match Some(()).filter(|_probe| has(chat)) {
					Some(_chat) => {
						crate::cli::tui::chat(path);
						recipe_infer::shutdown();
					}
					None => {
						recipe_runtime::execute::generate_gguf(path);
					}
				}
			}
			None => {
				let inner: &ModelInner = &model.inner;
				let mut last = self.cfg.last.borrow_mut();
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
			let last = self.cfg.last.borrow();
			if last.model.is_null() {
				Write::error("infer: call run(&model) first");
				return self;
			}
			last.model
		};
		// SAFETY: last_model was set from a &ModelInner in run/infer and the caller
		let model: &ModelInner = unsafe { &*last_model };
		let prepared = match dat.prepared() {
			Ok(v) => v,
			Err(e) => {
				Write::error(format!("eval: prepare data: {e:#}"));
				return self;
			}
		};
		let ds = prepared.get();
		recipe_runtime::execute::eval_run(&self.cfg, ds, model);
		let mut last = self.cfg.last.borrow_mut();
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
		self
	}
}

impl Default for Infer {
	fn default() -> Self {
		Self::new()
	}
}
