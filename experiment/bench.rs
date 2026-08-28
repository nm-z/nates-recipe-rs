//! One-file measured contraction experiment.

use recipe::*;

fn main() {
	let data = recipe.data("data/numeric/single_csv.csv").target("target").split(0.8);

	let model = recipe.model().layer(24).tanh().layer(1).loss(huber);

	recipe.train().fp(32).lr(0.001).epochs(8).seed(17).log(all).run(&model, &data);
}
