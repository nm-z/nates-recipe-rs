pub fn banned() {
	let data = recipe::recipe.data("data.csv");
	let model = recipe::recipe.model();
	let _ = recipe::recipe.train().declare(&data, &model);
}
