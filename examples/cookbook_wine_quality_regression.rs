//! Batch-normalized MSE regression for the UCI Wine Quality dataset.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_wine_quality_regression`.

use recipe::*;

const DATASET: &str = "examples/datasets/uci-winequality-semicolon/winequality-red.csv";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("quality")
		.norm(z_score)
		.split(0.8);
	recipe.model()
		.layer(64)
		.gelu()
		.norm(batch_norm)
		.layer(16)
		.silu()
		.layer(1)
		.loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.exp()
		.log(Loss)
		.save("cookbook-wine-quality-regression.ogdl")
		.run()?;
	Ok(())
}
