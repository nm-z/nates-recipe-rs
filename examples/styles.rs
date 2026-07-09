#!/usr/bin/env -S cargo run --release --example styles --
//! The three import styles, one crate, one dataset. Each block builds the same
//! model on the same data and trains it — only the door differs. Chaining,
//! `let` bindings, `run(data, model)` and `eval` are identical past the
//! constructor.
//!
//!   cargo run --release --example styles

use recipe::*;

const CSV: &str = "datasets/house-prices/train.csv";
const OGDL: &str = "/tmp/recipe-styles.ogdl";

fn main() {
	// ── style 1: static (dot syntax) ────────────────────────────────────
	eprintln!("\x1b[1;36m── style 1: recipe.data() ──\x1b[0m");
	let data = recipe.data(CSV).split(0.8).exclude("Id").target("SalePrice");
	let model = recipe.model().layer(64).leak().layer(1).loss(mse).lr(0.0001);
	recipe.train().epochs(5).log([Loss, R2]).run(data, model);
	recipe.eval(model, data);

	// ── style 2: struct (associated function) ───────────────────────────
	eprintln!("\x1b[1;36m── style 2: Data::load() ──\x1b[0m");
	let data = Data::load(CSV).split(0.8).exclude("Id").target("SalePrice");
	let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.0001);
	Train::new().epochs(5).log([Loss, R2]).run(data, model);
	model.eval(data);

	// ── style 3: crate path (free function) ─────────────────────────────
	eprintln!("\x1b[1;36m── style 3: recipe::data() ──\x1b[0m");
	let data = recipe::data(CSV).split(0.8).exclude("Id").target("SalePrice");
	let model = recipe::model().layer(64).leak().layer(1).loss(mse).lr(0.0001);
	recipe::train().epochs(5).log_every(1).log([Loss, R2]).run(data, model).save(OGDL);
	recipe::eval(model, data);

	// ── holdout: `run(data, model)` with a forward-only data source ──────
	eprintln!("\x1b[1;36m── holdout: run(&Option<Dataset>, model) ──\x1b[0m");
	let (_set, test) = data.datasets();
	Train::new().log([Loss, R2]).run(&test, model);
}
