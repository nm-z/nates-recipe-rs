use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-618-c0.ogdl");
	if std::env::var_os("RECIPE_PACKET_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 87]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
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
		.save(&bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	assert!(!report.predictions().is_empty());
	assert!(report.predictions().iter().all(|value| value.is_finite()));
	assert!(std::fs::metadata(&bundle).unwrap().len() > 0);
	println!("bundle {}", bundle.display());
}
