#!/usr/bin/env -S recipe run
// Generated token by token from API.ogdl.
// Reproduce with: cargo run --bin generate-train -- --seed 42

use recipe::*;

fn main() -> TrainingResult<()> {
	recipe.data("examples/datasets/uci-abalone/abalone.data")
		.target("col9")
		.norm(min_max)
		.split(0.85);

	recipe.model().layer(16).log().layer(1).loss(mae);

	recipe.train()
		.optimizer(adamw)
		.epochs(8)
		.lr(0.0002)
		.cos()
		.warmup(1)
		.log([Device, Epoch, Loss])
		.run()?;
	Ok(())
}
