use recipe::*;

fn main() {
	let bundle = "target/issue-515.ogdl";
	let data = recipe.data("data/text/sample_subfolders")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.knn(3).huber().iq(2).xxs
		.forest(2).leak().iq(2).xs
		.loss(rmse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(75147662717921)
		.epochs(1)
		.log(all)
		.stop(0.0)
		.int(1)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
}
