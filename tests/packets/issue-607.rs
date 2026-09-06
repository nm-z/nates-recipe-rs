use recipe::*;

fn main() {
	let bundle = "target/issue-607.ogdl";
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target");
	let model = recipe.model()
		.kmeans(2).tanh().norm(batch).qi(5).1
		.layer(8)
		.res([layer(8), layer(8)]).norm(batch).qi(4).nf
		.loss(ce);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(27127951603812)
		.epochs(1)
		.log(all)
		.stop(0.8)
		.int(8)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	let output = recipe.infer(bundle, &[0.0; 20]);
	assert!(output.iter().all(|value| value.is_finite()), "inference is not finite: {output:?}");
}
