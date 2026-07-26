pub fn banned() {
	let _ = recipe::Condition::new("age", recipe::ComparisonOperator::Less, 0);
}
