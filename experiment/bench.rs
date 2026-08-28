//! One-file measured contraction schedule experiment.

use recipe::*;

#[rustfmt::skip]
fn main() {
	let lut = recipe.lut([
		[16, 32, 64, 128, 256],
		[4, 8, 16, 32, 64],
		[8, 16, 32, 64, 128],
	]);

	let benchmark = recipe.model()
		.layer(24).tanh()
		.layer(1)
		.loss(huber);

	let tiles = recipe.model()
		.layer(24).tanh()
		.layer(3)
		.loss(&benchmark);

	recipe.train()
		.fp(32)
		.lr(0.001)
		.epochs(0)
		.seed(17)
		.log(all)
		.rat(&tiles, -99999999.0)
		.run(&benchmark, &lut);
}
