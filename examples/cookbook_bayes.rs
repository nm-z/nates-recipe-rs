//! Prepare and execute one observed categorical Bayesian conditional.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_bayes`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/bayes.csv";
const MODEL: &str = "cookbook-bayes.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET).target("play").split(0.8);
	recipe.model().bayes("play", ["weather", "wind"]);
	let prepared = recipe.train().save(MODEL).run()?;
	assert_eq!(prepared.kind(), TrainingModelKind::Bayes);
	assert_eq!(prepared.run(), None);

	recipe.data(DATASET).exclude("play");
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.kind(), InferenceModelKind::Bayes);
	assert_eq!(report.values().len(), 10);
	assert_eq!(report.decode_bayes_class(0), Some(b"falcon".as_slice()));
	assert_eq!(report.decode_bayes_class(1), Some(b"otter".as_slice()));
	Ok(())
}
