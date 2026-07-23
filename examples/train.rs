use recipe::*;

const ARC: &str = "/home/nate/Desktop/nates-recipe-rs/examples/datasets/rogii-wellbore-geology-prediction";

fn main() {
	recipe.data(ARC).split(0.8).target("TVT");

	recipe.model()
		.loss(mse)
		.layer(1024)
		.gelu()
		.layer(1024)
		.gelu()
		.layer(900)
		.lr(0.0001);

	recipe.train().epochs(100).log([Loss]).run(&data, &model);
}
