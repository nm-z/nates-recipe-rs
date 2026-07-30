//! A deterministic two-tree LightGBM-family forest with Recipe-owned leaf training.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_tree_forest`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";
const MODEL: &str = "cookbook-tree-forest.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().forest(2).lgbm(2).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Accuracy])
		.save(MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}
