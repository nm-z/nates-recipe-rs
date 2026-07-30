//! Residual branch with automatic projection when widths differ.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_residual`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("target")
		.norm(min_max)
		.split(0.75);
	recipe.model()
		.residual([layer(8), relu()])
		.silu()
		.layer(1)
		.loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Brier])
		.save("cookbook-residual.ogdl")
		.run()?;
	Ok(())
}
