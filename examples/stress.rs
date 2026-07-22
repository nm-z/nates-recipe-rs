use anyhow::Context;
use ogdl::log::{Opt, Write, opt, probe, r2, set_opt};
use recipe::*;
use std::fmt;
use std::path::Path;

const BANK: &str = "examples/datasets/uci-bank-semicolon/bank-full.csv";
const SEEDS: &str = "examples/datasets/uci-seeds/seeds_dataset.txt";
const WINE: &str = "examples/datasets/uci-wine/wine.data";

fn say(t: impl fmt::Display) {
	set_opt(Opt {
		probe: true,
		..opt()
	});
	Write::block(probe, ogdl!(&t));
}

fn columns(path: &str) -> anyhow::Result<recipe::data::RawCsv> {
	recipe::data::read_raw_csv(Path::new(path)).with_context(|| format!("stress: {path}"))
}

fn main() -> anyhow::Result<()> {
	for (path, want, sep) in [
		(BANK, 17usize, "semicolon"),
		(SEEDS, 8, "tab"),
		(WINE, 14, "comma"),
	] {
		let recipe::data::RawCsv { headers, rows } = columns(path)?;
		if headers.len() != want {
			Write::err(format!(
				"{sep} sniff failed: {path} parsed {} column(s), want {want}",
				headers.len()
			))?;
		}
		if rows.is_empty() {
			Write::err(format!("{path}: no rows parsed"))?;
		}
		say(format!(
			"\x1b[36mstress\x1b[0m  {sep:<10} {path}  {} cols × {} rows",
			headers.len(),
			rows.len()
		));
	}

	let rows = columns(BANK)?.rows.len();
	let data = Data::load(BANK).target("y");
	let model = Model::new()
		.loss(bce)
		.layer(64)
		.leak()
		.layer(1)
		.sigmoid()
		.lr(0.001);
	Train::new()
		.epochs(20)
		.log([Loss, Accuracy, hip])
		.run(&data, &model);

	let infer = Infer::new().log([r2]);
	infer.run(&model).eval(&data);
	let preds = infer.preds();
	if preds.len() != rows {
		Write::err(format!(
			"eval returned {} preds for {rows} rows",
			preds.len()
		))?;
	}
	if !preds.iter().all(|p| p.is_finite()) {
		Write::err("eval produced non-finite predictions")?;
	}
	say("\x1b[36mstress\x1b[0m  ok");
	Ok(())
}
