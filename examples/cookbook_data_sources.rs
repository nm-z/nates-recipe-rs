//! Ordered multi-source data declaration with an appended `.set(...)` source.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_data_sources`.

use recipe::*;

const RED: &str = "examples/datasets/uci-winequality-semicolon/winequality-red.csv";
const WHITE: &str = "examples/datasets/uci-winequality-semicolon/winequality-white.csv";

fn main() -> TrainingResult<()> {
	recipe.data([RED, WHITE])
		.set(RED)
		.target("quality")
		.norm(z_score)
		.split(0.8);
	recipe.model().layer(16).silu().layer(1).loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.log([Loss, R2])
		.save("cookbook-data-sources.ogdl")
		.run()?;
	Ok(())
}
