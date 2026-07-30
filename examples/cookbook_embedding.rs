//! Fixed-width integer-token rows with one learned embedding table and one
//! causal multi-head self-attention block.
//!
//! Feature-column order is sequence order. Every feature value is an exact
//! token ID in `0..8`; Recipe performs no tokenizer fitting or numeric input
//! normalization.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_embedding`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/tokens.csv";
const FIRST_MODEL: &str = "cookbook-embedding-first.ogdl";
const RESUMED_MODEL: &str = "cookbook-embedding-resumed.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET).target("target").split(0.75);
	recipe.model()
		.embed(4)
		.vocab(8)
		.attn(2)
		.layer(4)
		.relu()
		.layer(1)
		.loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss])
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET).target("target").split(0.75);
	recipe.model()
		.embed(4)
		.vocab(8)
		.attn(2)
		.layer(4)
		.relu()
		.layer(1)
		.loss(mse);
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
	assert_eq!(report.values().len(), 8);
	Ok(())
}
