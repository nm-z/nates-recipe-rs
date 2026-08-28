//! One-file measured contraction schedule experiment.

use recipe::*;

#[rustfmt::skip]
fn main() {
	let data = recipe.data("/home/nate/Desktop/vna-temp-fill-data-v2.zip")
		.target(["fill_percent"])
		.norm(z_score)
		.split(0.8);

	let workload = recipe.model()
		.conv(16, 5).pool(64).gelu()
		.layer(64).gelu()
		.layer(1)
		.loss(mae);

	// B maps each tile's log2 ratio from the analytic tile to measured score.
	let benchmark = recipe.model()
		.layer(24).tanh()
		.layer(1)
		.loss(huber);

	// T maps the contraction state to three log2 tile ratios. Zero keeps the
	// analytic tile, and every finite output maps to a positive integer tile.
	let tiles = recipe.model()
		.layer(24).tanh()
		.layer(3)
		.loss(huber);

	recipe.train()
		.rat(&benchmark, &tiles, -99999999)
		.fp(32)
		.lr(0.001)
		.epochs(1)
		.seed(17)
		.log(all)
		.run(&workload, &data);
}
