use anyhow::{Result, ensure};
use recipe::{Data, Infer, Model, Train, mse};
use std::fs;
use std::path::Path;

const TRAIN_F: &str = "crockpot_train.csv";
const DRAW_F: &str = "crockpot_draw.csv";
const TILES_F: &str = "crockpot_tiles.txt";
const MODEL_F: &str = "crockpot.ogdl";

fn rows(path: &str) -> usize {
	let Ok(t) = fs::read_to_string(path) else {
		return 0;
	};
	return t.lines().skip(1).filter(|l| !l.trim().is_empty()).count();
}

fn main() -> Result<()> {
	ensure!(
		Path::new(TRAIN_F).exists(),
		"{TRAIN_F} missing; run crockpot first"
	);
	ensure!(
		Path::new(DRAW_F).exists(),
		"{DRAW_F} missing; run crockpot first"
	);
	let n = rows(TRAIN_F);
	if n < 2 {
		eprintln!("{n} winner row: a fit needs 2+ (one row z-scores to all zeros); crockpot draws random tiles");
		return Ok(());
	}
	let model = Model::new()
		.loss(mse)
		.layer(16)
		.leak()
		.layer(16)
		.leak()
		.layer(3)
		.lr(0.001);
	let d = Data::load(TRAIN_F).target(["tile_m", "tile_n", "tile_k"]);
	let t = match Path::new(MODEL_F).exists() {
		true => Train::new().epochs(1).resume(MODEL_F),
		false => Train::new().epochs(1),
	};
	let stamp = fs::metadata(MODEL_F).and_then(|m| return m.modified()).ok();
	let fitted = t.run(&d, &model);
	let after = fs::metadata(MODEL_F).and_then(|m| return m.modified()).ok();
	if after == stamp {
		fs::remove_file(MODEL_F).ok();
		fitted.save(MODEL_F);
	}
	let neurons: usize = [16, 16, 3].iter().sum();
	println!(
		"r2 {}    loss {}    RMSE {}    neurons {}",
		recipe::tui::paint(0, format!("{:.2}", fitted.score())),
		recipe::tui::paint(1, format!("{:.4}", fitted.loss())),
		recipe::tui::paint(2, format!("{:.3}", fitted.loss().sqrt())),
		recipe::tui::paint(3, neurons)
	);
	let q = Data::load(DRAW_F).target(["tile_m", "tile_n", "tile_k"]);
	let inf = Infer::new();
	inf.run(&model).eval(&q);
	let p = inf.preds();
	ensure!(p.len() >= 3, "predict returned {} values, need 3", p.len());
	fs::write(TILES_F, format!("{} {} {}\n", p[0], p[1], p[2]))?;
	return Ok(());
}
