use recipe::*;

fn main() {
	let bundle = "target/issue-511.ogdl";
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.layer(8)
		.res([layer(8), layer(8)]).cos().norm(batch).qi(5).0
		.layer(8)
		.res([conv(8, 1), conv(8, 1)]).norm(batch).iq(2).xxs
		.loss(huber);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(39864877594399)
		.epochs(1)
		.log(all)
		.f(6, 9)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
