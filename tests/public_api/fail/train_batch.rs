pub fn user_batch_control_is_not_public() {
	let _ = recipe::recipe.train().batch(32);
}
