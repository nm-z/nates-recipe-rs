//! Categorical cross-entropy over the categorical UCI Sonar target.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_multiclass`.

use recipe::*;

const DATASET: &str = "examples/datasets/uci-sonar/sonar.all-data";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("col61")
		.norm(z_score)
		.split(0.8);
	recipe.model().layer(24).gelu().layer(3).loss(ce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0003)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-multiclass.ogdl")
		.run()?;
	Ok(())
}
