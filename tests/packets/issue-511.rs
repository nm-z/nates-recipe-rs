use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-511.ogdl");
	if std::env::var_os("RECIPE_511_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 20]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
	let data = recipe.data("data/temporal/chronological_splits").target("target").norm(z_score);
	let model = recipe.model().layer(8).res([layer(8), layer(8)]).cos().norm(batch).qi(5).0.layer(8).res([conv(8, 1), conv(8, 1)]).norm(batch).iq(2).xxs.loss(huber);
	let report = recipe.train().optimizer(adamw).lr(0.0001).seed(39864877594399).epochs(1).log(all).f(6, 9).save(&bundle).run(&model, &data);
	assert!(report.final_loss().is_finite());
	assert!(!report.predictions().is_empty());
	assert!(report.predictions().iter().all(|value| value.is_finite()));
	assert!(std::fs::metadata(&bundle).unwrap().len() > 0);
	println!("bundle {}", bundle.display());
}
