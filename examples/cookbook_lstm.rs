//! Independent numeric rows interpreted as fixed scalar sequences by one
//! long short-term memory block.
//!
//! Feature-column order is time order. Every row starts with exact-zero hidden
//! and cell state. Input, forget, and output gates use sigmoids; the candidate
//! uses tanh; `c = f * c_previous + i * g` and `h = o * tanh(c)`. Only the
//! final hidden state is emitted. State never crosses row or run boundaries.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_lstm`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";
const FIRST_MODEL: &str = "cookbook-lstm-first.ogdl";
const RESUMED_MODEL: &str = "cookbook-lstm-resumed.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().lstm(8).layer(1).loss(bce);
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
	recipe.model().lstm(8).layer(1).loss(bce);
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
