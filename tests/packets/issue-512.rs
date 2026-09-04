use recipe::*;

fn main() {
	let bundle = "target/issue-512.ogdl";
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.layer(8)
		.res([conv(8, 1), conv(8, 1)]).silu().qi(4).k.s
		.forest(2).silu()
		.loss(rmse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(8243371770253)
		.epochs(1)
		.log(all)
		.tf(32)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
