use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-513.ogdl");
	if std::env::var_os("RECIPE_513_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 3072]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
	let data = recipe.data("data/image/class_subfolders").target("target");
	let model = recipe.model()
		.lstm(8).cos().norm(batch).iq(2).m
		.gru(8).qi(8).0
		.loss(mae);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(14625434013680)
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
	println!("final_loss={:?}; bundle {}", report.final_loss(), bundle.display());
}
