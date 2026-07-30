pub fn hidden_save_default_is_not_public() {
	let _ = recipe::recipe.train().save(());
}
