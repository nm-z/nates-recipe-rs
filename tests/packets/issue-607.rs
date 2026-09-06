use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-607.ogdl");
	if std::env::var_os("RECIPE_PACKET_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 20]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target");
	let model = recipe.model()
		.kmeans(2).tanh().norm(batch).qi(5).1
		.layer(8)
		.res([layer(8), layer(8)]).norm(batch).qi(4).nf
		.loss(ce);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(27127951603812)
		.epochs(1)
		.log(all)
		.stop(0.8)
		.int(8)
		.save(&bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	assert!(!report.predictions().is_empty());
	assert!(report.predictions().iter().all(|value| value.is_finite()));
	assert!(std::fs::metadata(&bundle).unwrap().len() > 0);
	println!("bundle {}", bundle.display());
}
