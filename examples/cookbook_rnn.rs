//! Independent numeric rows interpreted as fixed scalar sequences by one
//! vanilla recurrent block.
//!
//! Feature-column order is time order. Every row starts from an all-zero
//! hidden state, uses the same learned tanh recurrence, and emits only its final
//! hidden state. State never crosses row or run boundaries.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_rnn`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";
const FIRST_MODEL: &str = "cookbook-rnn-first.ogdl";
const RESUMED_MODEL: &str = "cookbook-rnn-resumed.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().rnn(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log(Loss)
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().rnn(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(FIRST_MODEL)
		.save(RESUMED_MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(RESUMED_MODEL);
	let report = recipe.infer().evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}
