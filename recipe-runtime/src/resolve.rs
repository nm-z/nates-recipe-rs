use crate::execute::ModelInner;
use ogdl::log::{Write, gpu, prompt};
use pantry::encode::Dataset;
use recipe_infer::{LayerSpec, Loss};
use std::env;
use std::io::{self, IsTerminal};

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
	issues.extend(output_check(model.loss, last_out, n_layers, k));

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
