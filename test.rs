use recipe::*;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DATA_MODES: usize = 7;
const AUTOREGRESSIVE_DATA_MODES: usize = 4;
const MODEL_OPERATIONS: usize = 23;
const ACTIVATIONS: usize = 16;
const NORMALIZATIONS: usize = 2;
const QUANTIZATIONS: &[(u16, u8, u16)] = &[
	(0, 4, 0), (0, 4, 1), (0, 5, 0), (0, 5, 1), (0, 8, 0), (0, 8, 1),
	(0, 2, 3), (0, 6, 3), (0, 8, 3),
	(0, 3, 3), (0, 3, 4), (0, 3, 5), (0, 3, 6),
	(0, 4, 3), (0, 4, 4), (0, 4, 5), (0, 5, 3), (0, 5, 4), (0, 5, 5),
	(0, 4, 2),
	(1, 1, 3), (1, 1, 4),
	(1, 2, 1), (1, 2, 2), (1, 2, 3), (1, 2, 4),
	(1, 3, 1), (1, 3, 2), (1, 3, 3), (1, 3, 4),
	(1, 4, 2), (1, 4, 5),
];
const LOSSES: usize = 7;
const ARITHMETICS: usize = 10;
const STOP_MODES: usize = 3;
const LEARNING_RATES: usize = 2;
const LIFECYCLES: usize = 2;

struct DataCase {
	path: String,
	mode: usize,
	autoregressive: bool,
}

#[derive(Clone, Copy)]
struct Stage {
	operation: usize,
	activation: usize,
	normalization: usize,
	quantization: usize,
}

#[derive(Clone, Copy)]
struct TrainCase {
	arithmetic: usize,
	stop: usize,
	rate: usize,
}

fn checked_product(values: &[u64]) -> u64 {
	values.iter().copied().try_fold(1_u64, u64::checked_mul).expect("composition count exceeds u64")
}

fn take(value: &mut u64, radix: usize) -> usize {
	let selected = *value % radix as u64;
	*value /= radix as u64;
	selected as usize
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
	while right != 0 {
		(left, right) = (right, left % right);
	}
	left
}

fn mix(mut value: u64) -> u64 {
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ value >> 31
}

fn seed() -> u64 {
	std::env::var("RECIPE_COMPOSITION_SEED")
		.ok()
		.map(|value| value.parse().expect("RECIPE_COMPOSITION_SEED must be an unsigned integer"))
		.unwrap_or_else(|| {
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.expect("system clock precedes the Unix epoch")
				.as_nanos() as u64
		})
}

fn start_cursor(total: u64) -> u64 {
	std::env::var("RECIPE_COMPOSITION_CURSOR")
		.ok()
		.map(|value| value.parse().expect("RECIPE_COMPOSITION_CURSOR must be an unsigned integer"))
		.unwrap_or(0)
		.min(total)
}

fn datasets() -> Vec<DataCase> {
	let mut families = std::fs::read_dir("data")
		.expect("cannot read data")
		.map(|entry| entry.expect("cannot read a data family").path())
		.filter(|path| path.is_dir())
		.collect::<Vec<_>>();
	families.sort();
	let mut cases = Vec::new();
	for family in families {
		let mut representations = std::fs::read_dir(&family)
			.unwrap_or_else(|error| panic!("cannot read {}: {error}", family.display()))
			.map(|entry| entry.expect("cannot read a data representation").path())
			.filter(|path| {
				!matches!(path.extension().and_then(|value| value.to_str()), Some("py" | "pyi" | "ipynb"))
			})
			.collect::<Vec<_>>();
		representations.sort();
		for path in representations {
			let autoregressive = path.ends_with("autoregressive_lines.txt");
			let modes = if autoregressive { AUTOREGRESSIVE_DATA_MODES } else { DATA_MODES };
			let path = path.to_string_lossy().into_owned();
			cases.extend((0..modes).map(|mode| DataCase { path: path.clone(), mode, autoregressive }));
		}
	}
	assert!(!cases.is_empty(), "data defines no composition cases");
	cases
}

fn data(case: &DataCase) -> Data {
	let mut data = if case.autoregressive {
		recipe.data(auto).set(case.path.clone())
	} else {
		recipe.data(case.path.clone()).target("target")
	};
	match case.mode {
		0 => {}
		1 => data = data.norm(z_score),
		2 => data = data.split(0.8),
		3 if case.autoregressive => data = data.broadcast(),
		3 => data = data.test(case.path.clone()),
		4 => data = data.test(case.path.clone()).split(0.8),
		5 => data = data.broadcast(),
		6 => data = data.norm(z_score).broadcast().test(case.path.clone()).split(0.8),
		_ => unreachable!(),
	}
	data
}

fn stage(mut value: u64) -> Stage {
	Stage {
		operation: take(&mut value, MODEL_OPERATIONS),
		activation: take(&mut value, ACTIVATIONS),
		normalization: take(&mut value, NORMALIZATIONS),
		quantization: take(&mut value, QUANTIZATIONS.len() + 1),
	}
}

fn operation(model: Model, operation: usize) -> Model {
	match operation {
		0 => model.layer(4),
		1 => model.layer(8),
		2 => model.conv(8, 1),
		3 => model.conv(8, 3),
		4 => model.pool(2),
		5 => model.kmeans(2),
		6 => model.knn(3),
		7 => model.svm(),
		8 => model.forest(2),
		9 => model.bayes(),
		10 => model.cbst(),
		11 => model.xgbst(),
		12 => model.lgbm(),
		13 => model.embed(8, 256),
		14 => model.attn(1),
		15 => model.rnn(8),
		16 => model.gru(8),
		17 => model.lstm(8),
		18 => model.layer(8).residual([layer(8), layer(8)]),
		19 => model.layer(8).residual([conv(8, 1), conv(8, 1)]),
		20 => model.layer(8).residual([conv(8, 1), relu(), layer(8)]),
		21 => model.layer(8).moe(1, [layer(8), layer(8)]),
		22 => model.perc(8),
		_ => unreachable!(),
	}
}

fn activation(model: Model, activation: usize) -> Model {
	match activation {
		0 => model,
		1 => model.cos(),
		2 => model.exp(),
		3 => model.log(),
		4 => model.ln(),
		5 => model.huber(),
		6 => model.tan(),
		7 => model.relu(),
		8 => model.leak(),
		9 => model.sigmoid(),
		10 => model.tanh(),
		11 => model.selu(),
		12 => model.gelu(),
		13 => model.silu(),
		14 => model.elu(),
		15 => model.prelu(),
		_ => unreachable!(),
	}
}

fn quantization(model: Model, quantization: usize) -> Model {
	if quantization == 0 {
		return model;
	}
	let (family, bits, variant) = QUANTIZATIONS[quantization - 1];
	model.quantize(family, bits, variant)
}

fn apply(model: Model, stage: Stage) -> Model {
	let model = operation(model, stage.operation);
	let model = activation(model, stage.activation);
	let model = if stage.normalization == 0 { model } else { model.norm(batch) };
	quantization(model, stage.quantization)
}

fn model(mut ordinal: u64) -> (Model, String) {
	let stages = checked_product(&[MODEL_OPERATIONS as u64, ACTIVATIONS as u64, NORMALIZATIONS as u64, (QUANTIZATIONS.len() + 1) as u64]);
	let one = stages;
	let mut model = recipe.model();
	let mut description = Vec::new();
	if ordinal < one {
		let selected = stage(ordinal);
		model = apply(model, selected);
		description.push(format!("{}:{}:{}:{}", selected.operation, selected.activation, selected.normalization, selected.quantization));
	} else {
		ordinal -= one;
		for selected in [stage(ordinal % stages), stage(ordinal / stages)] {
			model = apply(model, selected);
			description.push(format!("{}:{}:{}:{}", selected.operation, selected.activation, selected.normalization, selected.quantization));
		}
	}
	(model, description.join("/"))
}

fn loss(model: Model, loss: usize) -> Model {
	model.loss(match loss {
		0 => mse,
		1 => rmse,
		2 => huber,
		3 => mae,
		4 => bce,
		5 => ce,
		6 => focal,
		_ => unreachable!(),
	})
}

fn train(case: TrainCase, seed: usize) -> Train {
	let train = recipe
		.train()
		.optimizer(adamw)
		.lr([0.001, 0.0001][case.rate])
		.seed(seed)
		.epochs(1)
		.log(all);
	let train = match case.stop {
		0 => train,
		1 => train.stop(0.0),
		2 => train.stop(0.8),
		_ => unreachable!(),
	};
	match case.arithmetic {
		0 => train.fp(8),
		1 => train.fp(16),
		2 => train.fp(32),
		3 => train.fp(64),
		4 => train.int(1),
		5 => train.int(4),
		6 => train.int(8),
		7 => train.bf(16),
		8 => train.tf(32),
		9 => train.f(6, 9),
		_ => unreachable!(),
	}
}

fn input_width(path: &Path) -> usize {
	let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
	let mut shape = text
		.lines()
		.find_map(|line| line.trim().strip_prefix("shape "))
		.expect("saved model has no input shape")
		.split_whitespace();
	let channels = shape.next().expect("saved model has no input channels").parse::<usize>().expect("saved input channels are invalid");
	let length = shape.next().expect("saved model has no input length").parse::<usize>().expect("saved input length is invalid");
	channels.checked_mul(length).expect("saved input width overflows")
}

fn execute(case: u64, data_case: &DataCase, model_ordinal: u64, loss_ordinal: usize, train_case: TrainCase, lifecycle: usize, description: &str) {
	let bundle = PathBuf::from(format!("/tmp/recipe-composition-{}.ogdl", std::process::id()));
	if bundle.exists() {
		std::fs::remove_file(&bundle).unwrap_or_else(|error| panic!("cannot remove {}: {error}", bundle.display()));
	}
	eprintln!("composition {case}: data={} mode={} model={} loss={} arithmetic={} stop={} rate={} lifecycle={}", data_case.path, data_case.mode, description, loss_ordinal, train_case.arithmetic, train_case.stop, train_case.rate, lifecycle);
	let data = data(data_case);
	let (model, _) = model(model_ordinal);
	let model = loss(model, loss_ordinal);
	// Recipe has no checkpoints: save and resume use the same file.
	let report = train(train_case, case as usize).save(&bundle).run(&model, &data);
	assert!(report.final_loss().is_finite(), "composition {case} produced a nonfinite training loss");
	if lifecycle == 1 {
		let report = train(train_case, case as usize).resume(&bundle).save(&bundle).run(&model, &data);
		assert!(report.final_loss().is_finite(), "composition {case} produced a nonfinite resumed loss");
	}
	let input = vec![0.0; input_width(&bundle)];
	let output = recipe.infer(&bundle, &input);
	assert!(!output.is_empty(), "composition {case} inference produced no values");
	assert!(output.iter().all(|value| value.is_finite()), "composition {case} inference produced a nonfinite value");
	std::fs::remove_file(&bundle).unwrap_or_else(|error| panic!("cannot remove {}: {error}", bundle.display()));
}

fn main() {
	let datasets = datasets();
	let stages = checked_product(&[MODEL_OPERATIONS as u64, ACTIVATIONS as u64, NORMALIZATIONS as u64, (QUANTIZATIONS.len() + 1) as u64]);
	let models = stages.checked_add(stages.checked_mul(stages).expect("two-stage model count overflows")).expect("model count overflows");
	let total = checked_product(&[
		datasets.len() as u64,
		models,
		LOSSES as u64,
		ARITHMETICS as u64,
		STOP_MODES as u64,
		LEARNING_RATES as u64,
		LIFECYCLES as u64,
	]);
	let seed = seed();
	let offset = mix(seed) % total;
	let mut step = mix(seed ^ 0x9e37_79b9_7f4a_7c15) % total;
	while gcd(step, total) != 1 {
		step = (step + 1) % total;
	}
	let start = start_cursor(total);
	eprintln!("composition space={total} seed={seed} cursor={start} step={step}");
	for cursor in start..total {
		let mut ordinal = ((offset as u128 + cursor as u128 * step as u128) % total as u128) as u64;
		let data_ordinal = take(&mut ordinal, datasets.len());
		let model_ordinal = ordinal % models;
		ordinal /= models;
		let loss_ordinal = take(&mut ordinal, LOSSES);
		let train_case = TrainCase {
			arithmetic: take(&mut ordinal, ARITHMETICS),
			stop: take(&mut ordinal, STOP_MODES),
			rate: take(&mut ordinal, LEARNING_RATES),
		};
		let lifecycle = take(&mut ordinal, LIFECYCLES);
		assert_eq!(ordinal, 0, "composition decoder left unused state");
		let (_, description) = model(model_ordinal);
		execute(
			((offset as u128 + cursor as u128 * step as u128) % total as u128) as u64,
			&datasets[data_ordinal],
			model_ordinal,
			loss_ordinal,
			train_case,
			lifecycle,
			&description,
		);
	}
}
