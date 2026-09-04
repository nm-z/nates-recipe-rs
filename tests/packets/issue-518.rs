use recipe::*;

fn main() {
	let bundle = "target/issue-518.ogdl";
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.layer(8)
		.res([conv(8, 1), relu(), layer(8)]).sigmoid().iq(3).xxs
		.layer(8)
		.moe(1, [layer(8), layer(8)]).sigmoid().qi(3).k
		.loss(focal);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(98076090604027)
		.epochs(1)
		.log(all)
		.int(8)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
