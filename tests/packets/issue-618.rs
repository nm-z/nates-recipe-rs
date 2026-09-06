use recipe::*;

fn main() {
	let bundle = "target/issue-618.ogdl";
	let data = recipe.data("data/temporal/series_sqlite.sqlite")
		.target("target");
	let model = recipe.model()
		.kmeans(2).ln().qi(8).1
		.layer(8)
		.moe(1, [layer(8), layer(8)]).elu().norm(batch).qi(3).k.m
		.loss(rmse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(64976989107628)
		.epochs(1)
		.log(all)
		.int(1)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	let output = recipe.infer(bundle, &[0.0; 512]);
	assert!(output.iter().all(|value| value.is_finite()), "inference is not finite: {output:?}");
}
