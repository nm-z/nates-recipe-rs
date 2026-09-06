use recipe::*;

fn main() {
	let directory = std::path::Path::new("target/packets");
	std::fs::create_dir_all(directory).unwrap();
	let bundle = directory.join("issue-520.ogdl");
	if std::env::var_os("RECIPE_520_INFER").is_some() {
		let before = std::fs::read(&bundle).unwrap();
		let output = recipe.infer(&bundle, &[0.0; 784]);
		assert!(!output.is_empty());
		assert!(output.iter().all(|value| value.is_finite()));
		assert_eq!(std::fs::read(&bundle).unwrap(), before);
		println!("inference {output:?}; bundle unchanged");
		return;
	}
	let data = recipe.data("data/image/arrays_npz.npz").target("target");
	let model = recipe.model().attn(1).exp().iq(4).xs.gru(8).elu().norm(batch).iq(2).xs.loss(bce);
	let report = recipe.train().optimizer(adamw).lr(0.001).seed(78620842746232).epochs(1).log(all).stop(0.0).bf(16).save(&bundle).run(&model, &data);
	assert!(report.final_loss().is_finite());
	assert!(!report.predictions().is_empty());
	assert!(report.predictions().iter().all(|value| value.is_finite()));
	assert!(std::fs::metadata(&bundle).unwrap().len() > 0);
	println!("bundle {}", bundle.display());
}
