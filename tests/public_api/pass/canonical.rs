#![allow(dead_code)]

pub fn canonical_declarations_compile() {
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
		.batch(0.25)
		.epochs(1)
		.lr(0.0002)
		.cos()
		.optimizer(recipe::adamw)
		.log(recipe::Loss)
		.log([recipe::Loss, recipe::Accuracy]);
	let infer = recipe::recipe
		.infer()
		.log(recipe::Loss)
		.log([recipe::Loss, recipe::Accuracy]);
	let _knn = recipe::recipe.model().knn(3);
	let _residual = recipe::recipe.model()
		.residual([recipe::layer(64), recipe::relu()]);
	let _ = recipe::compile_training(&train, &data, &model);
	let _ = infer.evaluate(&data, &model);
}

pub fn canonical_sequence_compiles() {
	recipe::recipe.data("train.csv").target("label").split(0.8);
	recipe::recipe.model().layer(1).loss(recipe::mse);
	recipe::recipe
		.train()
		.batch(0.5)
		.epochs(1)
		.lr(0.001)
		.exp()
		.run()
		.save("model.ogdl");
	let _ = recipe::recipe.model().load("model.ogdl", 1);
}
