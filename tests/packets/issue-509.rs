use recipe::*;

fn main() {
	let bundle = "target/issue-509.ogdl";
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
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
