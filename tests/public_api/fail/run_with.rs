pub fn banned() {
	let data = recipe::recipe.data("data.csv");
	let model = recipe::recipe.model();
	let _ = recipe::recipe.train().run_with(&data, &model);
}
