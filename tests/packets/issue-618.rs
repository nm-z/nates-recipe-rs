use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-618.ogdl");
	if std::env::var_os("RECIPE_PACKET_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 512]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
	let data = recipe.data("data/temporal/series_sqlite.sqlite")
		.target("target");
	let model = recipe.model()
		.kmeans(2).ln().qi(8).1
		.layer(8)
		.moe(1, [layer(8), layer(8)]).elu().norm(batch).qi(3).k.m
		.loss(rmse);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.001)
		.seed(64976989107628)
		.epochs(1)
		.log(all)
		.int(1)
		.save(&bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	assert!(!report.predictions().is_empty());
	assert!(report.predictions().iter().all(|value| value.is_finite()));
	assert!(std::fs::metadata(&bundle).unwrap().len() > 0);
	println!("bundle {}", bundle.display());
}
