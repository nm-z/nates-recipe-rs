use recipe::*;

fn main() {
	let bundle = "target/issue-621.ogdl";
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target")
		.broadcast();
	let model = recipe.model()
		.layer(8)
		.res([layer(8), layer(8)]).selu().qi(8).0
		.layer(8)
		.res([conv(8, 1), relu(), layer(8)]).exp().qi(5).k
		.loss(ce);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(79743068161581)
		.epochs(1)
		.log(all)
		.stop(0.0)
		.tf(32)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	let output = recipe.infer(bundle, &[0.0; 20]);
	assert!(output.iter().all(|value| value.is_finite()), "inference is not finite: {output:?}");
}
