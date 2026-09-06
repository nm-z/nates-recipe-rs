use recipe::*;

fn main() {
	let bundle = "target/issue-623-c0.ogdl";
	let data = recipe.data("data/image/archive_zip.zip")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.conv(8, 1).ln().norm(batch).iq(2).xs
		.svm().sigmoid().qi(3).k.m
		.loss(focal);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(45454603835217)
		.epochs(1)
		.log(all)
		.stop(0.0)
		.int(1)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	let output = recipe.infer(bundle, &[0.0; 6912]);
	assert!(output.iter().all(|value| value.is_finite()), "inference is not finite: {output:?}");
}
