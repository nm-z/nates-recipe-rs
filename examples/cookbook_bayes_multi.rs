//! Prepare and execute two observed categorical Bayesian target conditionals.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_bayes_multi`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/bayes_multi.csv";
const MODEL: &str = "cookbook-bayes-multi.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET).target(["play", "travel"]).split(0.8);
	recipe.model()
		.bayes("play", ["weather", "wind"])
		.bayes("travel", ["weather"]);
	let prepared = recipe.train().save(MODEL).run()?;
	assert_eq!(prepared.kind(), TrainingModelKind::Bayes);
	assert_eq!(prepared.run(), None);
	assert_eq!(prepared.bayes_model().unwrap().conditionals().len(), 2);

	recipe.data(DATASET).exclude(["play", "travel"]);
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.kind(), InferenceModelKind::Bayes);
	assert_eq!(report.bayes_output_count(), 2);
	assert_eq!(report.bayes_output_name(0), Some(b"play".as_slice()));
	assert_eq!(report.bayes_output_name(1), Some(b"travel".as_slice()));
	assert_eq!(report.bayes_output_classes(0), Some(2));
	assert_eq!(report.bayes_output_classes(1), Some(3));
	assert_eq!(report.bayes_output_range(0), Some(0..2));
	assert_eq!(report.bayes_output_range(1), Some(2..5));
	assert_eq!(report.values().len(), 25);
	assert_eq!(
		report.decode_bayes_output_class(0, 0),
		Some(b"falcon".as_slice())
	);
	assert_eq!(
		report.decode_bayes_output_class(1, 2),
		Some(b"walk".as_slice())
	);
	Ok(())
}
