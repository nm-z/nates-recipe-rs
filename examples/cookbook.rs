//!`cargo run --example cookbook`

use recipe::*;

type CookbookResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn main() -> CookbookResult {
	wine_quality_regression()?;
	tree_forest()?;
	save_resume()?;
	rnn()?;
	residual()?;
	observability()?;
	multiclass()?;
	multi_target()?;
	lstm()?;
	logarithms()?;
	knn()?;
	kmeans()?;
	inference()?;
	gru()?;
	gguf_llama()?;
	embedding()?;
	data_sources()?;
	convolution_pooling()?;
	bayes_multi()?;
	bayes()?;
	airfoil_perceptron()?;
	Ok(())
}

/// Batch-normalized MSE regression for the UCI Wine Quality dataset.
fn wine_quality_regression() -> CookbookResult {
	const DATASET: &str = "examples/datasets/uci-winequality-semicolon/winequality-red.csv";

	recipe.data(DATASET)
		.target("quality")
		.norm(z_score)
		.split(0.8);
	recipe.model()
		.layer(64)
		.gelu()
		.norm(batch_norm)
		.layer(16)
		.silu()
		.layer(1)
		.loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.exp()
		.log(Loss)
		.save("cookbook-wine-quality-regression.ogdl")
		.run()?;
	Ok(())
}

/// A deterministic two-tree LightGBM-family forest with Recipe-owned leaf training.
fn tree_forest() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";
	const MODEL: &str = "cookbook-tree-forest.ogdl";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().forest(2).lgbm(2).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Accuracy])
		.save(MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}

/// Semantic-model save followed by existence-conditional resume.
fn save_resume() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";
	const FIRST_MODEL: &str = "cookbook-save-resume-first.ogdl";
	const SECOND_MODEL: &str = "cookbook-save-resume-second.ogdl";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.resume(FIRST_MODEL)
		.save(SECOND_MODEL)
		.run()?;
	Ok(())
}

/// Independent numeric rows interpreted as fixed scalar sequences by one vanilla recurrent block.
fn rnn() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";
	const FIRST_MODEL: &str = "cookbook-rnn-first.ogdl";
	const RESUMED_MODEL: &str = "cookbook-rnn-resumed.ogdl";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().rnn(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log(Loss)
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().rnn(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(FIRST_MODEL)
		.save(RESUMED_MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(RESUMED_MODEL);
	let report = recipe.infer().evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}

/// Residual branch with automatic projection when widths differ.
fn residual() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";

	recipe.data(DATASET)
		.target("target")
		.norm(min_max)
		.split(0.75);
	recipe.model()
		.residual([layer(8), relu()])
		.silu()
		.layer(1)
		.loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Brier])
		.save("cookbook-residual.ogdl")
		.run()?;
	Ok(())
}

/// Training log cadence and bounded terminal plotting for regression.
fn observability() -> CookbookResult {
	const DATASET: &str = "examples/datasets/uci-airfoil/airfoil_self_noise.dat";

	recipe.data(DATASET).target("col6").norm(z_score).split(0.8);
	recipe.model().layer(16).tanh().layer(1).loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(2)
		.lr(0.0002)
		.warmup(1)
		.cos()
		.log([Loss, R2, Epoch, Lr, Time, Device])
		.every(1)
		.plot([Loss, R2])
		.run()?;
	Ok(())
}

/// Categorical cross-entropy over the categorical UCI Sonar target.
fn multiclass() -> CookbookResult {
	const DATASET: &str = "examples/datasets/uci-sonar/sonar.all-data";

	recipe.data(DATASET)
		.target("col61")
		.norm(z_score)
		.split(0.8);
	recipe.model().layer(24).gelu().layer(3).loss(ce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0003)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-multiclass.ogdl")
		.run()?;
	Ok(())
}

/// One joint cross-entropy objective over three explicitly ordered targets.
fn multi_target() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/multi_target.csv";

	recipe.data(DATASET)
		.target(["winner_model_b", "winner_model_a", "winner_tie"])
		.norm(z_score)
		.split(0.8);
	recipe.model().layer(12).gelu().layer(3).loss(ce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0003)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-multi-target.ogdl")
		.run()?;
	Ok(())
}

/// Independent numeric rows interpreted as fixed scalar sequences by one LSTM block.
fn lstm() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";
	const FIRST_MODEL: &str = "cookbook-lstm-first.ogdl";
	const RESUMED_MODEL: &str = "cookbook-lstm-resumed.ogdl";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().lstm(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log(Loss)
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().lstm(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(FIRST_MODEL)
		.save(RESUMED_MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(RESUMED_MODEL);
	let report = recipe.infer().evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}

/// Signed `.log()` and ordinary natural `.ln()` activations.
fn logarithms() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model()
		.layer(8)
		.exp()
		.ln()
		.layer(4)
		.exp()
		.log()
		.layer(1)
		.loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.log(Loss)
		.run()?;
	Ok(())
}

/// Prepare, save, and execute one all-output KNN feature reduction.
fn knn() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/knn.csv";
	const MODEL: &str = "cookbook-knn.ogdl";

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

/// Deterministic K-means distance reduction feeding an ordinary dense model.
fn kmeans() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().kmeans(4).layer(8).relu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-kmeans.ogdl")
		.run()?;
	Ok(())
}

/// Train, save, and execute target-free inference from semantic OGDL.
fn inference() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";
	const MODEL: &str = "cookbook-inference.ogdl";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.save(MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}

/// Independent numeric rows interpreted as fixed scalar sequences by one GRU block.
fn gru() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";
	const FIRST_MODEL: &str = "cookbook-gru-first.ogdl";
	const RESUMED_MODEL: &str = "cookbook-gru-resumed.ogdl";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().gru(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log(Loss)
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().gru(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(FIRST_MODEL)
		.save(RESUMED_MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(RESUMED_MODEL);
	let report = recipe.infer().evaluate()?;
	assert_eq!(report.values().len(), 12);
	Ok(())
}

/// Execute the checked-in dense-F32 llama GGUF instrument on exact token IDs.
fn gguf_llama() -> CookbookResult {
	const CORPUS: &str = "examples/datasets/llamacpp-archs-seed42";
	const TOKENS: &str = "examples/datasets/llamacpp-archs-seed42/tokens.txt";
	const MODEL: &str = "examples/datasets/llamacpp-archs-seed42/llama-dense.gguf";
	const REFERENCE: &str = "examples/datasets/llamacpp-archs-seed42/llama-dense.logits";

	recipe.data(TOKENS);
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.kind(), InferenceModelKind::GgufLlama);
	let actual = report.values().collect::<Vec<_>>();
	let expected = read_reference_logits(REFERENCE)?;
	assert_eq!(actual.len(), 128 * 128);
	assert_eq!(actual.len(), expected.len());

	let mut squared_error = 0.0_f64;
	let mut reference_energy = 0.0_f64;
	let mut maximum_error = 0.0_f32;
	for (actual, expected) in actual.iter().zip(&expected) {
		let error = actual - expected;
		squared_error += f64::from(error) * f64::from(error);
		reference_energy += f64::from(*expected) * f64::from(*expected);
		maximum_error = maximum_error.max(error.abs());
	}
	let normalized_mean_square_error = squared_error / reference_energy;
	println!(
		"llama.cpp parity\tcorpus\t{CORPUS}\tnmse\t{normalized_mean_square_error:.9e}\tmax_abs\t{maximum_error:.9e}"
	);
	assert!(
		normalized_mean_square_error < 1.0e-3,
		"GGUF llama logits diverged from the checked-in llama.cpp CPU oracle"
	);
	Ok(())
}

fn read_reference_logits(path: &str) -> CookbookResult<Vec<f32>> {
	let bytes = std::fs::read(path)?;
	if bytes.get(..4) != Some(b"LGT0") || bytes.len() < 12 {
		return Err("reference logits do not have an LGT0 header".into());
	}
	let positions = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
	let vocabulary = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
	let values = bytes[12..]
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one complete LGT0 f32")))
		.collect::<Vec<_>>();
	if values.len()
		!= positions
			.checked_mul(vocabulary)
			.ok_or("LGT0 shape overflowed")?
	{
		return Err("reference logit count differs from its LGT0 header".into());
	}
	Ok(values)
}

/// Fixed-width integer-token rows with an embedding table and causal self-attention.
fn embedding() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/tokens.csv";
	const FIRST_MODEL: &str = "cookbook-embedding-first.ogdl";
	const RESUMED_MODEL: &str = "cookbook-embedding-resumed.ogdl";

	recipe.data(DATASET).target("target").split(0.75);
	recipe.model()
		.embed(4)
		.vocab(8)
		.attn(2)
		.layer(4)
		.relu()
		.layer(1)
		.loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss])
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET).target("target").split(0.75);
	recipe.model()
		.embed(4)
		.vocab(8)
		.attn(2)
		.layer(4)
		.relu()
		.layer(1)
		.loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(FIRST_MODEL)
		.save(RESUMED_MODEL)
		.run()?;

	recipe.data(DATASET).exclude("target");
	recipe.model().load(RESUMED_MODEL);
	let report = recipe.infer().evaluate()?;
	assert_eq!(report.values().len(), 8);
	Ok(())
}

/// Ordered multi-source data declaration with an appended `.set(...)` source.
fn data_sources() -> CookbookResult {
	const RED: &str = "examples/datasets/uci-winequality-semicolon/winequality-red.csv";
	const WHITE: &str = "examples/datasets/uci-winequality-semicolon/winequality-white.csv";

	recipe.data([RED, WHITE])
		.set(RED)
		.target("quality")
		.norm(z_score)
		.split(0.8);
	recipe.model().layer(16).silu().layer(1).loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.log([Loss, R2])
		.save("cookbook-data-sources.ogdl")
		.run()?;
	Ok(())
}

/// Channelwise one-dimensional convolution followed by max pooling.
fn convolution_pooling() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/binary.csv";

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model()
		.conv(2, 2)
		.relu()
		.conv(3, 2)
		.prelu()
		.pool(2)
		.layer(1)
		.loss(focal);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log([Loss, Accuracy])
		.save("cookbook-convolution-pooling.ogdl")
		.run()?;
	Ok(())
}

/// Prepare and execute two observed categorical Bayesian target conditionals.
fn bayes_multi() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/bayes_multi.csv";
	const MODEL: &str = "cookbook-bayes-multi.ogdl";

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

/// Prepare and execute one observed categorical Bayesian conditional.
fn bayes() -> CookbookResult {
	const DATASET: &str = "examples/datasets/cookbook/bayes.csv";
	const MODEL: &str = "cookbook-bayes.ogdl";

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

/// Perceptron block and Huber objective for the UCI Airfoil dataset.
fn airfoil_perceptron() -> CookbookResult {
	const DATASET: &str = "examples/datasets/uci-airfoil/airfoil_self_noise.dat";

	recipe.data(DATASET).target("col6").norm(min_max).split(0.8);
	recipe.model()
		.perc(24)
		.silu()
		.layer(12)
		.huber()
		.layer(1)
		.loss(huber);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0001)
		.cos()
		.log(Loss)
		.save("cookbook-airfoil-perceptron.ogdl")
		.run()?;
	Ok(())
}
