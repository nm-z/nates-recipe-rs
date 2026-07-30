//! Train, save, and execute target-free inference from semantic OGDL.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_inference`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";
const MODEL: &str = "cookbook-inference.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.save(MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}
