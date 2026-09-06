use recipe::*;

fn main() {
	let bundle = format!("/tmp/recipe-composition-repro-{}.ogdl", std::process::id());
	let data = recipe.data("data/numeric/semicolon_data.data")
		.target("target");
	let model = recipe.model()
		.lgbm().cos().qi(5).k.m
		.layer(8)
		.moe(1, [layer(8), layer(8)]).selu().norm(batch).qi(4).k.s
		.loss(rmse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(97377517961600)
		.epochs(1)
		.log(all)
		.int(8)
		.save(&bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
