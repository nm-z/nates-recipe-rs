use recipe::*;

const DATASET: &str = "examples/datasets/rogii-wellbore-geology-prediction";

fn main() -> DeclarationResult<()> {
	let data = recipe.data(DATASET).split(0.8).target("TVT");

	let model = recipe
		.model()
		.loss(mse)
		.layer(1024)
		.gelu()
		.layer(1024)
		.gelu()
		.layer(900)
		.lr(0.0001);

	let declaration = recipe
		.train()
		.epochs(100)
		.log([Loss])
		.declare(&data, &model)?;

	// Declaration is side-effect free. End-to-end lowering and execution will
	// replace this boundary once the public integration is complete.
	assert_eq!(declaration.policy().epoch_bound(), Some(100));
	Ok(())
}
