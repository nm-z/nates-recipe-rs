use recipe::*;

fn main() {
	let bundle = "target/issue-623.ogdl";
	let data = recipe.data("data/image/archive_zip.zip")
		.target("target")
		.broadcast();
	let model = recipe.model()
		.conv(8, 1).leak().iq(3).m
		.xgbst().selu().iq(2).s
		.loss(mse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(93226021025387)
		.epochs(1)
		.log(all)
		.fp(32)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	let output = recipe.infer(bundle, &[0.0; 6912]);
	assert!(output.iter().all(|value| value.is_finite()), "inference is not finite: {output:?}");
}
