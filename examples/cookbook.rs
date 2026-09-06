//! `cargo run --example cookbook`.

use recipe::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	const WINE_PATH: &str = "examples/datasets/uci-winequality-semicolon/winequality-red.csv";
	const BINARY_PATH: &str = "examples/datasets/cookbook/binary.csv";
	const TREE_PATH: &str = "cookbook-tree-forest.ogdl";
	const SAVE_FIRST_PATH: &str = "cookbook-save-resume-first.ogdl";
	const SAVE_SECOND_PATH: &str = "cookbook-save-resume-second.ogdl";
	const RNN_FIRST_PATH: &str = "cookbook-rnn-first.ogdl";
	const RNN_RESUMED_PATH: &str = "cookbook-rnn-resumed.ogdl";
	const AIRFOIL_PATH: &str = "examples/datasets/uci-airfoil/airfoil_self_noise.dat";
	const SONAR_PATH: &str = "examples/datasets/uci-sonar/sonar.all-data";
	const MULTI_TARGET_PATH: &str = "examples/datasets/cookbook/multi_target.csv";
	const LSTM_FIRST_PATH: &str = "cookbook-lstm-first.ogdl";
	const LSTM_RESUMED_PATH: &str = "cookbook-lstm-resumed.ogdl";
	const KNN_DATA_PATH: &str = "examples/datasets/cookbook/knn.csv";
	const KNN_MODEL_PATH: &str = "cookbook-knn.ogdl";
	const INFERENCE_MODEL_PATH: &str = "cookbook-inference.ogdl";
	const GRU_FIRST_PATH: &str = "cookbook-gru-first.ogdl";
	const GRU_RESUMED_PATH: &str = "cookbook-gru-resumed.ogdl";
	const LLAMA_TOKENS_PATH: &str = "examples/datasets/llamacpp-archs-seed42/tokens.txt";
	const LLAMA_MODEL_PATH: &str = "examples/datasets/llamacpp-archs-seed42/llama-dense.gguf";
	const EMBEDDING_DATA_PATH: &str = "examples/datasets/cookbook/tokens.csv";
	const EMBEDDING_FIRST_PATH: &str = "cookbook-embedding-first.ogdl";
	const EMBEDDING_RESUMED_PATH: &str = "cookbook-embedding-resumed.ogdl";
	const WHITE_WINE_PATH: &str = "examples/datasets/uci-winequality-semicolon/winequality-white.csv";
	const BAYES_MULTI_DATA_PATH: &str = "examples/datasets/cookbook/bayes_multi.csv";
	const BAYES_MULTI_MODEL_PATH: &str = "cookbook-bayes-multi.ogdl";
	const BAYES_DATA_PATH: &str = "examples/datasets/cookbook/bayes.csv";
	const BAYES_MODEL_PATH: &str = "cookbook-bayes.ogdl";

	recipe.data(WINE_PATH)
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

	recipe.data(BINARY_PATH)
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
		.save(TREE_PATH)
		.run()?;
	recipe.data(BINARY_PATH).exclude("target");
	recipe.model().load(TREE_PATH);
	recipe.infer().log([Time, Device]).evaluate()?;

	recipe.data(BINARY_PATH)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.save(SAVE_FIRST_PATH)
		.run()?;
	recipe.data(BINARY_PATH)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.resume(SAVE_FIRST_PATH)
		.save(SAVE_SECOND_PATH)
		.run()?;

	recipe.data(BINARY_PATH)
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
		.save(RNN_FIRST_PATH)
		.run()?;
	recipe.data(BINARY_PATH)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().rnn(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(RNN_FIRST_PATH)
		.save(RNN_RESUMED_PATH)
		.run()?;
	recipe.data(BINARY_PATH).exclude("target");
	recipe.model().load(RNN_RESUMED_PATH);
	recipe.infer().evaluate()?;

	recipe.data(BINARY_PATH)
		.target("target")
		.norm(min_max)
		.split(0.75);
	recipe.model()
		.residual([layer(8).relu()])
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

	recipe.data(AIRFOIL_PATH)
		.target("col6")
		.norm(z_score)
		.split(0.8);
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

	recipe.data(SONAR_PATH)
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

	recipe.data(MULTI_TARGET_PATH)
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

	recipe.data(BINARY_PATH)
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
		.save(LSTM_FIRST_PATH)
		.run()?;
	recipe.data(BINARY_PATH)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().lstm(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(LSTM_FIRST_PATH)
		.save(LSTM_RESUMED_PATH)
		.run()?;
	recipe.data(BINARY_PATH).exclude("target");
	recipe.model().load(LSTM_RESUMED_PATH);
	recipe.infer().evaluate()?;

	recipe.data(BINARY_PATH)
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

	recipe.data(KNN_DATA_PATH)
		.target(["class_target", "numeric_target"])
		.norm(z_score)
		.split(0.75);
	recipe.model().knn(3);
	recipe.train().save(KNN_MODEL_PATH).run()?;
	recipe.data(KNN_DATA_PATH)
		.exclude(["class_target", "numeric_target"]);
	recipe.model().load(KNN_MODEL_PATH);
	recipe.infer().log([Time, Device]).evaluate()?;

	recipe.data(BINARY_PATH)
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

	recipe.data(BINARY_PATH)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.save(INFERENCE_MODEL_PATH)
		.run()?;
	recipe.data(BINARY_PATH).exclude("target");
	recipe.model().load(INFERENCE_MODEL_PATH);
	recipe.infer().log([Time, Device]).evaluate()?;

	recipe.data(BINARY_PATH)
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
		.save(GRU_FIRST_PATH)
		.run()?;
	recipe.data(BINARY_PATH)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().gru(8).layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.resume(GRU_FIRST_PATH)
		.save(GRU_RESUMED_PATH)
		.run()?;
	recipe.data(BINARY_PATH).exclude("target");
	recipe.model().load(GRU_RESUMED_PATH);
	recipe.infer().evaluate()?;

	recipe.data(LLAMA_TOKENS_PATH);
	recipe.model().load(LLAMA_MODEL_PATH);
	recipe.infer().log([Time, Device]).evaluate()?;

	recipe.data(EMBEDDING_DATA_PATH)
		.target("target")
		.split(0.75);
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
		.log(Loss)
		.save(EMBEDDING_FIRST_PATH)
		.run()?;
	recipe.data(EMBEDDING_DATA_PATH)
		.target("target")
		.split(0.75);
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
		.resume(EMBEDDING_FIRST_PATH)
		.save(EMBEDDING_RESUMED_PATH)
		.run()?;
	recipe.data(EMBEDDING_DATA_PATH).exclude("target");
	recipe.model().load(EMBEDDING_RESUMED_PATH);
	recipe.infer().evaluate()?;

	recipe.data([WINE_PATH, WHITE_WINE_PATH])
		.set(WINE_PATH)
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

	recipe.data(BINARY_PATH)
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

	recipe.data(BAYES_MULTI_DATA_PATH)
		.target(["play", "travel"])
		.split(0.8);
	recipe.model()
		.bayes("play", ["weather", "wind"])
		.bayes("travel", ["weather"]);
	recipe.train().save(BAYES_MULTI_MODEL_PATH).run()?;
	recipe.data(BAYES_MULTI_DATA_PATH)
		.exclude(["play", "travel"]);
	recipe.model().load(BAYES_MULTI_MODEL_PATH);
	recipe.infer().log([Time, Device]).evaluate()?;

	recipe.data(BAYES_DATA_PATH).target("play").split(0.8);
	recipe.model().bayes("play", ["weather", "wind"]);
	recipe.train().save(BAYES_MODEL_PATH).run()?;
	recipe.data(BAYES_DATA_PATH).exclude("play");
	recipe.model().load(BAYES_MODEL_PATH);
	recipe.infer().log([Time, Device]).evaluate()?;

	recipe.data(AIRFOIL_PATH)
		.target("col6")
		.norm(min_max)
		.split(0.8);
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

	return Ok(());
}

#[test]
fn cookbook() -> Result<(), Box<dyn std::error::Error>> { return main(); }
