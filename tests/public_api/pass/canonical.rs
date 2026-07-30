#![allow(dead_code)]
#![deny(unused_must_use)]

pub fn canonical_declarations_compile() -> Result<(), Box<dyn std::error::Error>> {
	let data = recipe::recipe
		.data(["train.csv", "more.zip"])
		.set("images")
		.target("label")
		.exclude(recipe::cond!(Age < 0))
		.split(0.8)
		.norm(recipe::z_score);
	let model = recipe::recipe
		.model()
		.layer(32)
		.norm(recipe::layer_norm)
		.relu()
		.layer(1)
		.sigmoid()
		.loss(recipe::bce)
		.grad(recipe::clip(1.0));
	let train = recipe::recipe
		.train()
		.epochs(1)
		.lr(0.0002)
		.cos()
		.optimizer(recipe::adamw)
		.log(recipe::Loss)
		.log([recipe::Loss, recipe::Accuracy]);
	let infer = recipe::recipe
		.infer()
		.log(recipe::Time)
		.log([recipe::Time, recipe::Device]);
	let _inference_policy = infer;
	let _knn = recipe::recipe.model().knn(3);
	let _residual = recipe::recipe
		.model()
		.residual([recipe::layer(64), recipe::relu()]);
	let _logarithms = recipe::recipe.model().layer(4).log().layer(4).ln();
	let _training = recipe::compile_training(&train, &data, &model)?;
	Ok(())
}

pub fn canonical_sequence_compiles() -> Result<(), Box<dyn std::error::Error>> {
	recipe::recipe.data("train.csv").target("label").split(0.8);
	recipe::recipe.model().layer(1).loss(recipe::mse);
	let _report = recipe::recipe
		.train()
		.epochs(1)
		.lr(0.001)
		.exp()
		.save("model.ogdl")
		.run()?;
	recipe::recipe.data("inference.csv");
	recipe::recipe.model().load("model.ogdl");
	let _inference = recipe::recipe.infer().evaluate()?;
	Ok(())
}
