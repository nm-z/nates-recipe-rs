//! Perceptron block and Huber objective for the UCI Airfoil dataset.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_airfoil_perceptron`.

use recipe::*;

const DATASET: &str = "examples/datasets/uci-airfoil/airfoil_self_noise.dat";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET).target("col6").norm(min_max).split(0.8);
	recipe.model()
		.perc(24)
		.silu()
		.layer(12)
		.huber()
		.layer(1)
		.loss(huber);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0001)
		.cos()
		.log(Loss)
		.save("cookbook-airfoil-perceptron.ogdl")
		.run()?;
	Ok(())
}
