//! Signed `.log()` and ordinary natural `.ln()` activations.
//!
//! `.log()` computes `sign(x) * ln(abs(x))` for nonzero inputs; `.ln()` accepts
//! only positive inputs. Each logarithm follows `.exp()` here so its runtime
//! domain is guaranteed by the preceding Recipe operation.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_logarithms`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model()
		.layer(8)
		.exp()
		.ln()
		.layer(4)
		.exp()
		.log()
		.layer(1)
		.loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.log(Loss)
		.run()?;
	Ok(())
}
