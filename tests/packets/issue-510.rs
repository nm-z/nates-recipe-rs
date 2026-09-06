use recipe::*;
use std::fs;

const BUNDLE: &str = "target/issue-510.ogdl";

fn main() {
	if std::env::var("RECIPE_PACKET_INFER").as_deref() == Ok("1") {
		let before = fs::read(BUNDLE).expect("training did not create the bundle");
		let output = recipe.infer(BUNDLE, &[0.0; 20]);
		assert!(!output.is_empty(), "inference returned no values");
		assert!(output.iter().all(|value| value.is_finite()), "inference returned a non-finite value");
		let after = fs::read(BUNDLE).expect("bundle disappeared during inference");
		assert_eq!(before, after, "inference changed the saved bundle");
		println!("inference values={output:?} bundle_bytes={}", after.len());
		return;
	}
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target")
		.split(0.8);
	let model = recipe.model()
		.lgbm().prelu().norm(batch).qi(3).k.m
		.layer(4).log().norm(batch).qi(3).k.l
		.loss(rmse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(39676778323334)
		.epochs(1)
		.log(all)
		.f(6, 9)
		.save(BUNDLE)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
