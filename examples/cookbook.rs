#!/usr/bin/env -S cargo run --release --example cookbook --
use ogdl::log::{acc, r2};
use recipe::{Accuracy, Data, Infer, Loss, Model, R2, Train, attn, bce, ce, embed, mse};

#[expect(dead_code)]
struct Sets {
	numeric: &'static str,
	temporal: &'static str,
	categoric: &'static str,
	ordinal: &'static str,
	text: &'static str,
	image: [&'static str; 2],
}

const SET: Sets = Sets {
	numeric: "examples/datasets/house-prices/train.csv",
	temporal: "examples/datasets/web-traffic-time-series-forecasting/train_1.csv",
	categoric: "examples/datasets/playground-series-s6e3/train.csv",
	ordinal: "examples/datasets/wine-quality/winequality-red.csv",
	text: "examples/datasets/llm-classification-finetuning/train.csv",
	image: [
		"examples/datasets/predict-the-handwriting-images/train.csv",
		"examples/datasets/predict-the-handwriting-images/train_images/",
	],
};

fn main() {
	let nn = Model::new() // NN
		.loss(bce)
		.layer(64)
		.leak()
		.layer(1)
		.sigmoid()
		.lr(0.001);
	let nn_data = Data::load(SET.categoric)
		.split(0.8)
		.exclude("id")
		.target("Churn");
	let nn_train = Train::new().epochs(1).log([Loss, Accuracy]);
	let nn_infer = Infer::new().log([acc]);

	let cnn = Model::new() // CNN
		.loss(ce)
		.conv(32, 3, 1)
		.leak()
		.conv(64, 3, 1)
		.leak()
		.layer(128)
		.leak()
		.layer(36)
		.lr(0.001);
	let cnn_data = Data::load(SET.image[0])
		.set(SET.image[1])
		.split(0.8)
		.target("label");
	let cnn_train = Train::new().epochs(1).log([Loss, Accuracy]);
	let cnn_infer = Infer::new().log([acc]);

	let mlp = Model::new() // MLP
		.loss(mse)
		.layer(128)
		.leak()
		.layer(64)
		.leak()
		.layer(1)
		.lr(0.0001);
	let mlp_data = Data::load(SET.numeric)
		.split(0.8)
		.exclude("Id")
		.target("SalePrice");
	let mlp_train = Train::new().epochs(1).log([Loss, R2]);
	let mlp_infer = Infer::new().log([r2]);

	let llm = Model::new() // LLM
		.loss(ce)
		.layer(embed(16))
		.layer(attn(4))
		.layer(32)
		.leak()
		.layer(3)
		.lr(0.001);
	let llm_data =
		Data::load(SET.text)
			.split(0.8)
			.exclude("id")
			.target(["winner_model_a", "winner_model_b", "winner_tie"]);
	let llm_train = Train::new().epochs(1).log([Loss, Accuracy]);
	let llm_infer = Infer::new().log([acc]);

	for (model, data, train, infer) in [
		(&nn, &nn_data, &nn_train, &nn_infer),
		(&cnn, &cnn_data, &cnn_train, &cnn_infer),
		(&mlp, &mlp_data, &mlp_train, &mlp_infer),
		(&llm, &llm_data, &llm_train, &llm_infer),
	] {
		train.run(data, model);
		infer.run(model).eval(&data.datasets().test);
	}
}
