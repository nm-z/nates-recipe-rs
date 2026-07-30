//! Training log cadence and bounded terminal plotting for regression.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_observability`.

use recipe::*;

const DATASET: &str = "examples/datasets/uci-airfoil/airfoil_self_noise.dat";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET).target("col6").norm(z_score).split(0.8);
	recipe.model().layer(16).tanh().layer(1).loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(2)
		.lr(0.0002)
		.warmup(1)
		.cos()
		.log([Loss, R2, Epoch, Lr, Time, Device])
		.every(1)
		.plot([Loss, R2])
		.run()?;
	Ok(())
}
