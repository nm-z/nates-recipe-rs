use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-518.ogdl");
	if std::env::var_os("RECIPE_518_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 20]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
	let data = recipe.data("data/temporal/chronological_splits")
		.target("target")
		.norm(z_score);
	let model = recipe.model()
		.layer(8)
		.res([conv(8, 1), relu(), layer(8)]).sigmoid().iq(3).xxs
		.layer(8)
		.moe(1, [layer(8), layer(8)]).sigmoid().qi(3).k
		.loss(focal);
	let report = recipe.train()
		.optimizer(adamw)
		.lr(0.0001)
		.seed(98076090604027)
		.epochs(1)
		.log(all)
		.int(8)
		.save(&bundle)
		.run(&model, &data);
	assert!(report.final_loss().is_finite());
	assert!(!report.predictions().is_empty());
	assert!(report.predictions().iter().all(|value| value.is_finite()));
	assert!(std::fs::metadata(&bundle).unwrap().len() > 0);
	println!("final_loss={:?}; bundle {}", report.final_loss(), bundle.display());
}
