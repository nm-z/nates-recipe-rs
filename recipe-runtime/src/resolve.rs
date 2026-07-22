use crate::execute::ModelInner;
use ogdl::log::{Write, gpu, prompt};
use pantry::encode::Dataset;
use recipe_ir::{LayerSpec, Loss};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::io::{self, IsTerminal};

thread_local! {
	static MODEL_REGISTRY: RefCell<HashMap<u32, (Vec<LayerSpec>, Loss, recipe_ir::ObjectiveIntent)>> =
		RefCell::new(HashMap::new());
}

pub fn register_model(
	id: recipe_ir::ObjectId,
	specs: Vec<LayerSpec>,
	loss: Loss,
	objective: recipe_ir::ObjectiveIntent,
) {
	MODEL_REGISTRY.with(|reg| {
		reg.borrow_mut().insert(id.0, (specs, loss, objective));
	});
}

pub fn registered(
	id: recipe_ir::ObjectId,
) -> Option<(Vec<LayerSpec>, Loss, recipe_ir::ObjectiveIntent)> {
	return MODEL_REGISTRY.with(|reg| reg.borrow().get(&id.0).cloned());
}

pub fn resolve_path(path: &str) -> String {
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

pub fn expand_tilde(path: &str) -> String {
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

#[derive(Clone, Copy)]
pub enum Gate {
	Proceed,
	Abort,
}

pub struct Issue {
	what: String,
	have: String,
	need: String,
}

pub fn output_check(lossfn: Loss, last_out: usize, n_layers: usize, k: usize) -> Option<Issue> {
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

pub fn preflight(model: &ModelInner, ds: &Dataset, net_ram: usize) -> Vec<Issue> {
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
	issues.extend(output_check(model.loss.get(), last_out, n_layers, k));

	let (free_vram, total_vram) = gpu_core::hip::mem_info()
		.map(|m| (m.free, m.total))
		.unwrap_or((0, 0));
	let need = crate::plan::plan_footprint(model, ds);
	Some(())
		.filter(|_probe| need > free_vram)
		.map(|_probe| {
			let planned = crate::memory::plan(need, net_ram);
			match planned {
				Some(p) => {
					let net_part = Some(())
						.filter(|_p| p.remote > 0)
						.map(|_p| {
							format!(" + NET {}", recipe_infer::human_bytes(p.remote))
						})
						.unwrap_or_default();
					Write::line(
						gpu,
						&format!(
							"waterfall  scratch {} -> VRAM {} + RAM {} + DISK {}{net_part}",
							recipe_infer::human_bytes(need),
							recipe_infer::human_bytes(p.vram),
							recipe_infer::human_bytes(p.ram),
							recipe_infer::human_bytes(p.disk),
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
							recipe_infer::human_bytes(free_vram),
							recipe_infer::human_bytes(total_vram)
						),
						need: recipe_infer::human_bytes(need),
					});
				}
			}
		})
		.unwrap_or(());

	issues
}

pub fn confirm_issues(issues: &[Issue]) -> Gate {
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TargetKind {
	None,
	Continuous,
	Binary,
	Categorical,
}

pub struct DataFacts {
	pub n: usize,
	pub d: usize,
	pub k: usize,
	pub target_kind: TargetKind,
}

pub fn derive_facts(ds: &Dataset) -> DataFacts {
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let k = ds.n_targets.max(1);
	let target_kind = classify_target(ds);
	return DataFacts {
		n,
		d,
		k,
		target_kind,
	};
}

fn classify_target(ds: &Dataset) -> TargetKind {
	let Some(_has) = Some(()).filter(|_probe| ds.has_target) else {
		return TargetKind::None;
	};
	let Some(_single) = Some(()).filter(|_probe| ds.n_targets <= 1) else {
		return TargetKind::Categorical;
	};
	let rows = ds.y.len();
	let Some(_notbin) = Some(()).filter(|_probe| !ds.y.iter().all(|&v| v == 0.0 || v == 1.0))
	else {
		return TargetKind::Binary;
	};
	let all_count = ds
		.y
		.iter()
		.all(|&v| v.is_finite() && v >= 0.0 && v.fract() == 0.0);
	let Some(_int) = Some(()).filter(|_probe| all_count) else {
		return TargetKind::Continuous;
	};
	let mut seen = std::collections::HashSet::new();
	for &v in ds.y.iter() {
		seen.insert(v as u64);
	}
	Some(())
		.filter(|_probe| seen.len() < rows)
		.map(|_probe| TargetKind::Categorical)
		.unwrap_or(TargetKind::Continuous)
}

#[derive(Clone)]
pub struct Note {
	pub subject: String,
	pub chose: String,
	pub because: String,
}

pub enum ResolveError {
	NoCoherentProgram(String),
	Ambiguous(Vec<String>),
}

impl ResolveError {
	pub fn message(&self) -> String {
		return match self {
			ResolveError::NoCoherentProgram(s) => s.clone(),
			ResolveError::Ambiguous(opts) => {
				format!("ambiguous objective: {}", opts.join(", "))
			}
		};
	}
}

pub struct Resolved {
	pub loss: Loss,
	pub lr: f64,
	pub notes: Vec<Note>,
	pub graph: Option<recipe_ir::SemanticGraph>,
}

pub fn resolve_model(
	objective: &recipe_ir::ObjectiveIntent,
	lr_intent: Option<f64>,
	specs: &[LayerSpec],
	facts: &DataFacts,
) -> Result<Resolved, ResolveError> {
	use recipe_ir::ObjectiveIntent as Obj;
	let mut notes = Vec::new();
	let (loss, graph) = match objective {
		Obj::Builtin(l) => (*l, None),
		Obj::Unspecified => {
			let picked = match facts.target_kind {
				TargetKind::Continuous => (Loss::Mse, "mse", "target is continuous"),
				TargetKind::Binary => (Loss::Bce, "bce", "target is binary in {0, 1}"),
				TargetKind::Categorical => {
					(Loss::Ce, "ce", "target is integer-coded categorical")
				}
				TargetKind::None => {
					return Err(ResolveError::NoCoherentProgram(
						"no loss can be chosen: the dataset has no target producer \
						 (no target column and no .target()) so nothing produces y to \
						 compare a prediction against"
							.to_string(),
					));
				}
			};
			notes.push(Note {
				subject: "loss".to_string(),
				chose: picked.1.to_string(),
				because: picked.2.to_string(),
			});
			(picked.0, None)
		}
		Obj::Reference(recipe_ir::ObjectRef::Object(ref_id)) => {
			resolve_reference(*ref_id, specs, facts, &mut notes)?
		}
		Obj::Expression(_) => {
			return Err(ResolveError::NoCoherentProgram(
				"objective is an expression: expressions have no producer yet — no code \
				 path evaluates a user loss expression"
					.to_string(),
			));
		}
	};
	let lr = lr_intent.unwrap_or(0.01);
	let lr_note = lr_intent.is_none() && !matches!(objective, Obj::Builtin(_));
	Some(())
		.filter(|_probe| lr_note)
		.map(|_probe| {
			notes.push(Note {
				subject: "lr".to_string(),
				chose: format!("{lr}"),
				because: "no rate set, framework default".to_string(),
			});
		})
		.unwrap_or(());
	return Ok(Resolved {
		loss,
		lr,
		notes,
		graph,
	});
}

fn resolve_reference(
	ref_id: recipe_ir::ObjectId,
	referrer_specs: &[LayerSpec],
	facts: &DataFacts,
	notes: &mut Vec<Note>,
) -> Result<(Loss, Option<recipe_ir::SemanticGraph>), ResolveError> {
	let Some((ref_specs, ref_loss, ref_obj)) = registered(ref_id) else {
		return Err(ResolveError::NoCoherentProgram(format!(
			"objective references model #{} as its loss, but that model was never registered \
			 as a surrogate source — only a model passed to .loss(&other) is registered, and \
			 this id was not; nothing produces a scalar for the referrer to reduce",
			ref_id.0
		)));
	};

	let referrer_dims = crate::graph::logical_dims(referrer_specs, facts.d);
	let Some(referrer_last) = referrer_dims.last().copied() else {
		return Err(ResolveError::NoCoherentProgram(
			"surrogate reference: the referrer model has no layers, so it has no output to \
			 feed the referenced model"
				.to_string(),
		));
	};
	let referrer_out = referrer_last.out_dim;
	let referenced_dims = crate::graph::logical_dims(&ref_specs, referrer_out);

	let Some(referenced_first) = referenced_dims.first().copied() else {
		return Err(ResolveError::NoCoherentProgram(format!(
			"surrogate dimensional guard: referenced model #{} has no layers, so its first \
			 input dimension cannot match the referrer's {referrer_out}-wide output",
			ref_id.0
		)));
	};
	if referenced_first.in_dim != referrer_out {
		return Err(ResolveError::NoCoherentProgram(format!(
			"surrogate dimensional guard: referrer emits {referrer_out} features but \
			 referenced model #{} takes {} — the surrogate cannot consume the referrer's output",
			ref_id.0, referenced_first.in_dim
		)));
	}

	let referrer_g =
		crate::graph::build_forward_graph(recipe_ir::ObjectId(u32::MAX), &referrer_dims, ref_loss);
	let referenced_g = crate::graph::build_forward_graph(ref_id, &referenced_dims, ref_loss);

	let scalar_empty = referenced_g
		.objectives
		.first()
		.and_then(|o| referenced_g.values.iter().find(|v| v.id == o.scalar))
		.is_some_and(|v| v.shape.dims.is_empty());
	if !scalar_empty {
		return Err(ResolveError::NoCoherentProgram(format!(
			"surrogate scalar guard: referenced model #{}'s objective does not reduce to a \
			 0-dimensional scalar, so it cannot serve as a loss",
			ref_id.0
		)));
	}

	let merged = crate::graph::compose_surrogate(&referrer_g, &referenced_g);

	let reached = crate::graph::rev_reached_ops(&merged);
	let np_referrer = referrer_g.params.len() as u32;
	let mut missing: Vec<u32> = merged
		.params
		.iter()
		.filter(|p| p.id.0 < np_referrer && !reached.contains(&p.owner.0))
		.map(|p| p.owner.0)
		.collect();
	missing.sort_unstable();
	missing.dedup();
	if !missing.is_empty() {
		return Err(ResolveError::NoCoherentProgram(format!(
			"surrogate reverse-path guard: the gradient path from referenced model #{}'s \
			 scalar does not reach referrer ops {missing:?}, which own referrer weights — the \
			 surrogate cannot train those parameters",
			ref_id.0
		)));
	}

	if matches!(ref_obj, recipe_ir::ObjectiveIntent::Reference(_)) {
		return Err(ResolveError::NoCoherentProgram(format!(
			"surrogate contradiction guard: referenced model #{} itself defines its loss by \
			 reference to another model — a chain of surrogates has no ground scalar; give the \
			 referenced model a concrete loss first",
			ref_id.0
		)));
	}

	notes.push(Note {
		subject: "referrer objective".to_string(),
		chose: format!("surrogate loss through referenced model #{}", ref_id.0),
		because: format!(
			"reference is coherent: referrer's {referrer_out}-wide output feeds the referenced \
			 input, the referenced objective reduces to a 0-dim scalar, the reverse path reaches \
			 all {np_referrer} referrer params, and the referenced loss is concrete ({})",
			ref_loss.name()
		),
	});

	return Ok((ref_loss, Some(merged)));
}
