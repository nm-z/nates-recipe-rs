use recipe::*;
use std::hash::{BuildHasher, Hasher};

fn main() {
	let data = recipe
		.data("/home/nate/Desktop/D7-data")
		.target("temper1")
		.exclude([
			"Frequency(Hz)",
			"sample",
			"time",
			"scan",
			"temperx",
			"am1",
			"am2",
			"max6675",
			"minivna",
			"am1rh",
			"am2rh",
		])
		.norm(z_score)
		.split(0.2);
	let blocks: &[fn(Model) -> Model] = &[
		|model| model.layer(64),
		|model| model.layer(8).residual([layer(8), relu()]),
		|model| model.conv(8, 3).pool(64),
		|model| model.attn(1),
		|model| model.embed(4, 8).pool(64),
		|model| model.rnn(8),
		|model| model.gru(8),
		|model| model.lstm(8),
		|model| model.perc(24),
		|model| model.moe(2, [layer(8), layer(8), layer(8), layer(8)]),
		|model| model.svm([tanh, relu, gelu, sigmoid]),
		|model| model.kmeans(4),
		|model| model.knn(5),
	];
	let activations: &[fn(Model) -> Model] = &[
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
	let normalizations: &[Option<Norm>] = &[None, Some(batch), Some(layer)];
	let trials = env!("RECIPE_TRAINING_TRIALS").parse::<usize>().expect("training-trials must be positive");
	let random = || std::collections::hash_map::RandomState::new().build_hasher().finish() as usize;
	let model = activations[random() % activations.len()](blocks[random() % blocks.len()](recipe.model()));
	let model = match normalizations[random() % normalizations.len()] {
		Some(normalization) => model.norm(normalization),
		None => model,
	};
	let model = model.layer(1).loss(mse);
	for seed in 1..=trials {
		let report = recipe
			.train()
			.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
			.seed(seed)
			.epochs(10)
			.lr(0.1)
			.run(&model, &data);
		eprintln!("seed {seed}/{trials} R2 {:.4}", report.r2());
	}
}
