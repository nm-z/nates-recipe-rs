pub fn banned() {
	let data = recipe::recipe.data("data.csv");
	let model = recipe::recipe.model().load("model.ogdl");
	let _ = recipe::recipe.infer().evaluate(&data, &model);
}
