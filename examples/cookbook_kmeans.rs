//! Deterministic K-means distance reduction feeding an ordinary dense model.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_kmeans`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().kmeans(4).layer(8).relu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-kmeans.ogdl")
		.run()?;
	Ok(())
}
