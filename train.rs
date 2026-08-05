use recipe::*;

fn main() {
	let data = recipe
		.data(
			"/home/nate/Desktop/nates-recipe-rs/examples/datasets/uci-bank-semicolon/bank-full.csv",
		)
		.target("y")
		.norm(z_score)
		.split(0.6);
	let checkpoint = recipe
		.model()
		.layer(8)
		.relu()
		.norm(batch)
		.layer(1)
		.loss(mse);
	recipe.train()
		.stop(0.0)
		.optimizer(adamw)
		.epochs(1)
		.lr(0.01)
		.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
		.save("train.ogdl")
		.run(&checkpoint, &data);
	recipe.train()
		.stop(1.0)
		.optimizer(adamw)
		.epochs(2)
		.lr(0.01)
		.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
		.resume("train.ogdl")
		.save("train.ogdl")
		.run(&checkpoint, &data);
	let lloyd = recipe.model().kmeans(4).layer(1).loss(mse);
	recipe.train()
		.optimizer(adamw)
		.epochs(2)
		.lr(0.01)
		.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
		.run(&lloyd, &data);
	let estimator_data = recipe
		.data(
			"/home/nate/Desktop/nates-recipe-rs/examples/datasets/uci-bank-semicolon/bank-full.csv",
		)
		.target("y")
		.norm(z_score)
		.split(0.01);
	for estimator in [
		recipe.model().knn(5).loss(mse),
		recipe.model().forest(2).lgbm(2).loss(mse),
		recipe.model().forest(2).xgbst(2).loss(mse),
	] {
		recipe.train()
			.optimizer(adamw)
			.epochs(2)
			.lr(0.01)
			.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
			.run(&estimator, &estimator_data);
	}
	let topology = recipe
		.model()
		.conv(8, 3)
		.gelu()
		.norm(batch)
		.lstm(8)
		.silu()
		.norm(layer)
		.layer(1)
		.loss(focal);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.01)
		.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
		.run(&topology, &data);

	let blocks: [fn(Model) -> Model; 10] = [
		|model| model.conv(8, 3),
		|model| model.pool(2),
		|model| model.kmeans(4),
		|model| model.embed(4).vocab(8),
		|model| model.layer(64),
		|model| model.rnn(8),
		|model| model.gru(8),
		|model| model.lstm(8),
		|model| model.residual([layer(8), relu()]),
		|model| model.perc(24),
	];
	let activations: [fn(Model) -> Model; 16] = [
		|model| model,
		|model| model.relu(),
		|model| model.gelu(),
		|model| model.silu(),
		|model| model.tanh(),
		|model| model.sigmoid(),
		|model| model.elu(),
		|model| model.selu(),
		|model| model.prelu(),
		|model| model.leak(),
		|model| model.exp(),
		|model| model.ln(),
		|model| model.log(),
		|model| model.cos(),
		|model| model.tan(),
		|model| model.huber(),
	];
	let normalizations = [None, Some(batch), Some(layer)];
	for activation in activations {
		for normalization in normalizations {
			for loss_fn in [mse, mae, huber, bce, ce, focal, rmse] {
				for block in blocks {
					let model = activation(block(recipe.model()));
					let model = match normalization {
						Some(normalization) => model.norm(normalization),
						None => model,
					}
					.layer(1)
					.loss(loss_fn);
					recipe.train()
						.optimizer(adamw)
						.epochs(1)
						.lr(0.01)
						.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
						.run(&model, &data);
				}
			}
		}
	}
}
