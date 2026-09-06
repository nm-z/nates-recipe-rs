use recipe::*;

fn main() -> TrainingResult<()> {
	recipe.data("datasets/cookbook/bayes_multi.csv")
		.target(["play", "travel"])
		.split(0.8);

	recipe.model()
		.bayes("play", ["weather", "wind"])
		.bayes("travel", ["weather"]);

	recipe.train()
		.epochs(48)
		.save("cookbook-bayes-multi.ogdl")
		.run()?;
	return Ok(());
}
