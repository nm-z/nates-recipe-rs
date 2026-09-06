use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-623-c0.ogdl");
	if std::env::var_os("RECIPE_PACKET_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 6912]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
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
		.save(&bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	assert!(!report.predictions().is_empty());
	assert!(report.predictions().iter().all(|value| value.is_finite()));
	assert!(std::fs::metadata(&bundle).unwrap().len() > 0);
	println!("bundle {}", bundle.display());
}
