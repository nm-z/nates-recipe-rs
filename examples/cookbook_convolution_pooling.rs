//! Channelwise one-dimensional convolution followed by max pooling.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_convolution_pooling`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model()
		.conv(2, 2)
		.relu()
		.conv(3, 2)
		.prelu()
		.pool(2)
		.layer(1)
		.loss(focal);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-convolution-pooling.ogdl")
		.run()?;
	Ok(())
}
