use recipe::*;
use std::fs;
const BLOCKS: &[fn(Model) -> Model] = &[
	|model| model.pool(1),
	|model| model.layer(8),
	|model| model.layer(8).residual([layer(8)]),
	|model| model.attn(1),
	|model| model.embed(4, 256),
	|model| model.rnn(4),
	|model| model.gru(4),
	|model| model.lstm(4),
	|model| model.perc(8),
	|model| model.moe(2, [const { layer(4) }; 4]),
	|model| model.svm([tanh]),
	|model| model.kmeans(4),
	|model| model.knn(5),
];
const NORMALIZATIONS: usize = 3;
const QUANTIZATIONS: usize = 33;
const PRECISIONS: usize = 9;
fn quantize(model: Model, index: usize) -> Model {
	let Some(index) = index.checked_sub(1) else { return model };
	let (family, bits, variant): (u16, u8, u16) = match index {
		0..=5 => (0, (4 + index / 2 + 2 * usize::from(index >= 4)) as u8, (index % 2) as u16),
		6 => (0, 2, 3),
		7..=10 => (0, 3, (index - 4) as u16),
		11..=13 => (0, 4, (index - 8) as u16),
		14..=16 => (0, 5, (index - 11) as u16),
		17..=18 => (0, (6 + 2 * (index - 17)) as u8, 3),
		19 => (0, 4, 2),
		20..=21 => (1, 1, (index - 17) as u16),
		22..=29 => (1, (2 + (index - 22) / 4) as u8, ((index - 22) % 4 + 1) as u16),
		30 => (1, 4, 2),
		31 => (1, 4, 5),
		_ => unreachable!("quantization index exceeds the matrix"),
	};
	model.quantize(family, bits, variant)
}
fn precision(train: Train, index: usize) -> Train {
	match index {
		0..=3 => train.fp(1 << (index + 3)),
		4..=6 => train.int(1 << (index - 4 + usize::from(index > 4))),
		7 => train.bf(16),
		_ => train.tf(32),
	}
}
fn convolution(model: Model, quantization: usize) -> Model { quantize(model.conv(1, 1), quantization) }
fn build(block: usize, activation: usize, normalization: usize, first: usize, second: Option<usize>) -> Model {
	let model = recipe.model();
	let model = if let Some(second) = second {
		convolution(convolution(model, first), second)
	} else {
		quantize(BLOCKS[block - 1](model), first)
	};
	let model = model.activate(unsafe { std::mem::transmute::<u8, Activation>(activation as u8) });
	if normalization == 0 { model } else { model.norm(if normalization == 1 { batch } else { layer }) }
}
fn data() -> Vec<Data> {
	let mut paths = fs::read_dir("./data")
		.unwrap()
		.flat_map(|family| fs::read_dir(family.unwrap().path()).unwrap())
		.map(|entry| entry.unwrap().path())
		.collect::<Vec<_>>();
	paths.sort();
	assert_eq!(paths.len(), 72, "./data must contain 72 direct grandchildren");
	paths.into_iter().map(|path| {
		let source = path.to_string_lossy().into_owned();
		let data = recipe.data(source.as_str());
		if source.ends_with("autoregressive_lines.txt") { data.target(auto) } else { data.target("target") }
	}).collect()
}
fn random(state: &mut u64, limit: usize) -> usize {
	*state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
	(*state as usize) % limit
}
fn main() {
	let mut state = env!("RECIPE_RANDOM_SEED").parse::<u64>().unwrap();
	let data = data();
	std::panic::set_hook(Box::new(|_| {}));
	loop {
		let dataset = random(&mut state, data.len());
		let block = random(&mut state, BLOCKS.len() + 1);
		let activation = random(&mut state, Activation::Prelu as usize + 1);
		let normalization = random(&mut state, NORMALIZATIONS);
		let first = random(&mut state, QUANTIZATIONS);
		let second = (block == 0).then(|| random(&mut state, QUANTIZATIONS));
		let format = random(&mut state, PRECISIONS);
		let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let model = build(block, activation, normalization, first, second);
			let train = recipe.train()
				.log([Run, Time, Epoch, R2, Loss, blck, atvn, norm, quant]);
			precision(train, format)
				.run(&model, &data[dataset])
		}));
	}
}
