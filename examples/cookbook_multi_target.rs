//! One joint cross-entropy objective over three explicitly ordered targets.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_multi_target`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/multi_target.csv";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target(["winner_model_b", "winner_model_a", "winner_tie"])
		.norm(z_score)
		.split(0.8);
	recipe.model().layer(12).gelu().layer(3).loss(ce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0003)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-multi-target.ogdl")
		.run()?;
	Ok(())
}
