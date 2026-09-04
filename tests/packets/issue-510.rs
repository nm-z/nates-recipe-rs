use recipe::*;

fn main() {
	let bundle = "target/issue-510.ogdl";
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
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
