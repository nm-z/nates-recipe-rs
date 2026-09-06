use recipe::*;

fn main() {
	let bundle = "target/issue-618-c0.ogdl";
	let data = recipe.data("data/text/records_xml.xml")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.knn(3).norm(batch).iq(2).m
		.layer(8)
		.moe(1, [layer(8), layer(8)]).selu().iq(1).s
		.loss(focal);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(36385327355047)
		.epochs(1)
		.log(all)
		.int(4)
		.save(bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	let output = recipe.infer(bundle, &[0.0; 87]);
	assert!(output.iter().all(|value| value.is_finite()), "inference is not finite: {output:?}");
}
