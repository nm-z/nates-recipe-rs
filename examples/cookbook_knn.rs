//! Prepare, save, and execute one all-output KNN feature reduction.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_knn`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/knn.csv";
const MODEL: &str = "cookbook-knn.ogdl";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(DATASET)
		.target(["class_target", "numeric_target"])
		.norm(z_score)
		.split(0.75);
	recipe.model().knn(3);
	let prepared = recipe.train().save(MODEL).run()?;
	assert_eq!(prepared.kind(), TrainingModelKind::Knn);
	assert_eq!(prepared.run(), None);

	recipe.data(DATASET)
		.exclude(["class_target", "numeric_target"]);
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.kind(), InferenceModelKind::Knn);
	let predictions = report
		.knn_predictions()
		.expect("KNN inference returns independently typed outputs");
	assert_eq!(predictions.len(), 2);
	assert_eq!(
		predictions[0].contract().kind(),
		KnnInferencePredictionKind::DiscreteMode
	);
	assert_eq!(
		predictions[1].contract().kind(),
		KnnInferencePredictionKind::NumericMean
	);
	Ok(())
}
