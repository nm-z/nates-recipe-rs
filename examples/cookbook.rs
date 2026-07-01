#!/usr/bin/env -S cargo run --release --example cookbook --
use recipe::*;

#[allow(dead_code)]
struct Sets {
	numeric: &'static str,
	temporal: &'static str,
	categoric: &'static str,
	ordinal: &'static str,
	text: &'static str,
	image: [&'static str; 2],
}

const SET: Sets = Sets {
	numeric: "datasets/house-prices/train.csv",
	temporal: "datasets/web-traffic-time-series-forecasting/train_1.csv",
	categoric: "datasets/playground-series-s6e3/train.csv",
	ordinal: "datasets/wine-quality/winequality-red.csv",
	text: "datasets/llm-classification-finetuning/train.csv",
	image: [
		"datasets/predict-the-handwriting-images/train.csv",
		"datasets/predict-the-handwriting-images/train_images/",
	],
};

fn main() {
	let nn = Model::new()             // NN
		.loss(bce)
		.layer(64)
		.leak()
		.layer(1)
		.sigmoid()
		.lr(0.001);
	let nn_data = Data::load()
		.set(SET.categoric)
		.split(0.8)
		.exclude("id")
		.target("Churn");
	let nn_train = Train::new()
		.epochs(20)
		.log([Loss, Accuracy]);

	let cnn = Model::new()            // CNN
		.loss(ce)
		.conv(32, 3, 1)
		.leak()
		.conv(64, 3, 1)
		.leak()
		.layer(128)
		.leak()
		.layer(36)
		.lr(0.001);
	let cnn_data = Data::load()
		.set(SET.image[0])
		.set(SET.image[1])
		.split(0.8)
		.target("label");
	let cnn_train = Train::new()
		.epochs(20)
		.log([Loss, Accuracy]);

	let mlp = Model::new()          // MLP
		.loss(mse)
		.layer(128)
		.leak()
		.layer(64)
		.leak()
		.layer(1)
		.lr(0.0001);
	let mlp_data = Data::load()
		.set(SET.numeric)
		.split(0.8)
		.exclude("Id")
		.target("SalePrice");
	let mlp_train = Train::new()
		.epochs(20)
		.log([Loss, R2]);

	let llm = Model::new()          // LLM
		.loss(ce)
		.layer(embed(16))
		.layer(attn(4))
		.layer(32).leak()
		.layer(3)
		.lr(0.001);
	let llm_data = Data::load()
		.set(SET.text)
		.split(0.8)
		.exclude("id")
		.target(["winner_model_a", "winner_model_b", "winner_tie"]);
	let llm_train = Train::new()
		.epochs(20)
		.log([Loss, Accuracy]);

	for (model, data, train) 
	in [(&nn , &nn_data , &nn_train ),
		(&cnn, &cnn_data, &cnn_train),
		(&mlp, &mlp_data, &mlp_train),
		(&llm, &llm_data, &llm_train),]
		{ 
			train.run((model, data)); 
		}
}
























