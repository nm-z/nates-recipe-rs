use recipe::*;

fn main() {
	let bundle = "target/issue-519.ogdl";
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target");
	let model = recipe.model()
		.layer(8).qi(8).1
		.pool(2).selu().qi(3).k
		.loss(huber);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(8431471041318)
		.epochs(1)
		.log(all)
		.tf(32)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
