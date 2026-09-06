use recipe::*;
use std::fs;

const BUNDLE: &str = "target/issue-509.ogdl";

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
		.target("target");
	let model = recipe.model()
		.perc(8).relu().qi(4).k.s
		.layer(4).exp().norm(batch).iq(2).xxs
		.loss(ce);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(71674482689610)
		.epochs(1)
		.log(all)
		.stop(0.0)
		.fp(8)
		.save(BUNDLE)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
