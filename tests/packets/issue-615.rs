use recipe::*;

fn main() {
	let bundle = "target/issue-615.ogdl";
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.layer(8).qi(8).0
		.pool(2).gelu().iq(3).m
		.loss(mse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(85151065342375)
		.epochs(1)
		.log(all)
		.stop(0.8)
		.int(1)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	let output = recipe.infer(bundle, &[0.0; 20]);
	assert!(output.iter().all(|value| value.is_finite()), "inference is not finite: {output:?}");
}
