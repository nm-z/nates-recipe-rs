pub fn banned() {
	let model = recipe::recipe.model();
	let _ = recipe::recipe.infer().declare(&model);
}
