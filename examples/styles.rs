#!/usr/bin/env -S cargo run --release --example styles --

use ogdl::log::{loss, r2};
use recipe::{Data, Infer, Loss, Model, R2, Train, mse, recipe};

const CSV: &str = "examples/datasets/house-prices/train.csv";
const OGDL: &str = "/tmp/recipe-styles.ogdl";

fn main() {
	// ── style 1: static (dot syntax) ────────────────────────────────────
	let data = recipe
		.data(CSV)
		.split(0.8)
		.exclude("Id")
		.target("SalePrice");
	let model = recipe
		.model()
		.layer(64)
		.leak()
		.layer(1)
		.loss(mse)
		.lr(0.0001);
	recipe.train().epochs(5).log([Loss, R2]).run(&data, &model);
	recipe.infer().log([r2]).run(&model).eval(&data);

	// ── style 2: struct (associated function) ───────────────────────────
	let data = Data::load(CSV).split(0.8).exclude("Id").target("SalePrice");
	let model = Model::new().layer(64).leak().layer(1).loss(mse).lr(0.0001);
	Train::new().epochs(5).log([Loss, R2]).run(&data, &model);
	Infer::new().log([r2]).run(&model).eval(&data);

	// ── style 3: crate path (free function) ─────────────────────────────
	let data = recipe::data(CSV)
		.split(0.8)
		.exclude("Id")
		.target("SalePrice");
	let model = recipe::model()
		.layer(64)
		.leak()
		.layer(1)
		.loss(mse)
		.lr(0.0001);
	recipe::train()
		.epochs(5)
		.log_every(1)
		.log([Loss, R2])
		.run(&data, &model)
		.save(OGDL);
	recipe::infer().log([r2]).run(&model).eval(&data);

	// ── holdout: infer().run(model).eval(test) on a forward-only source ──
	let test = data.datasets().test;
	recipe.infer().log([loss, r2]).run(&model).eval(&test);
}
