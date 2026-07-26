#!/usr/bin/env -S recipe run
use recipe::*;

fn main() {
	const HP: &str = "examples/datasets/house-prices/train.csv";

	recipe.data(HP)
		.exclude("Id")
		.target("SalePrice")
		.norm(z_score)
		.split(0.8);
	recipe.model().branch;
		Branch {
			layer: 64,
			atvn: relu,
		}.layer(1).loss(mse);
	recipe.train()
		.optimizer(adamw)
		.batch(0.1)
		.epochs(20)
		.lr(0.05)
		.cos()
		.log([Loss, R2])
		.run();
}

recipe.model().branch();
	Branch {
		layer: 64,
		atvn: relu,
	}
	Branch {
		layer: 1,
		loss: mse,
	}




// 6. wrap-macro: the one place you get ANY grammar you want
branch! {
    data  csv "d4.csv"
    knn   5
    pool  8
}



recipe.model()
	.residual(
		layer(64),
		relu(),
	).layer(1).loss(mse);













.
