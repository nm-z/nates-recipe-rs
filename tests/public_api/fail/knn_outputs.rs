pub fn knn_output_count_is_not_a_public_argument() {
	let _ = recipe::recipe.model().knn(4, 3);
}
