use super::*;

fn distance(left: &[f64], right: &[f64]) -> f64 {
	left.iter().zip(right).map(|(a, b)| (a - b).powi(2)).sum()
}

fn nearest(query: &[f64], state: &[f64], features: usize) -> (usize, f64) {
	state.chunks_exact(features)
		.enumerate()
		.map(|(index, row)| (index, distance(query, row)))
		.min_by(|left, right| left.1.total_cmp(&right.1))
		.unwrap_or((0, f64::INFINITY))
}

fn graph_inputs(graph: &Graph, samples: &[f64], targets: &[f64], rows: usize, backend: Backend) -> Result<Vec<f64>> {
	if graph.nodes.is_empty() {
		return Ok(samples[..rows * graph.output.elements()].to_vec());
	}
	let mut tape = DeviceTape::new(graph, samples, &targets[..rows], backend)?;
	tape.forward()?;
	tape.predictions()
}

fn fit_surrogate(
	input: Shape,
	samples: &[f64],
	targets: &[f64],
	hidden: usize,
	backend: Backend,
	config: Config,
) -> Result<Vec<f64>> {
	require(!targets.is_empty(), "surrogate requires teacher outputs")?;
	let sample_count = checked_mul(targets.len(), input.elements(), "surrogate samples")?;
	require(samples.len() == sample_count, "surrogate sample batch is invalid")?;
	let model = recipe.model().layer(hidden).tanh().layer(1);
	let prepared = Prepared {
		samples: samples.to_vec(),
		targets: targets.to_vec(),
		rows: targets.len(),
		features: input.elements(),
		schema: String::new(),
	};
	let graph = compile_output(&model, &prepared, prepared.rows, backend, config, 1)?;
	let mut tape = DeviceTape::new(&graph, samples, targets, backend)?;
	for step in 1..=config.surrogate_epochs {
		tape.epoch(step, config.surrogate_rate, mse, 0.0, config, false)?;
	}
	tape.weights(false)
}

fn estimator_predict(
	operation: &Operation,
	data: &Prepared,
	training_rows: usize,
	config: Config,
	exclude_self: bool,
) -> Result<Vec<f64>> {
	let test_rows = data.rows - training_rows;
	require(test_rows != 0, "estimator split must retain test rows")?;
	let (kind, argument, state_size) = match operation {
		Operation::KMeans(clusters) => {
			require(*clusters != 0 && *clusters <= training_rows, "kmeans cluster count is invalid")?;
			(0, *clusters, clusters * data.features)
		}
		Operation::Knn(neighbors) => {
			let maximum = training_rows - usize::from(exclude_self);
			require(*neighbors != 0 && *neighbors <= maximum, "knn neighbor count is invalid")?;
			(1, *neighbors, training_rows * (data.features + 1))
		}
		_ => return Err(RecipeError::new("operation is not a supported estimator")),
	};
	let mut fitted = Vec::with_capacity(state_size);
	if kind == 0 {
		fitted.extend_from_slice(&data.samples[..state_size]);
		let mut assignments = std::iter::repeat_n(0_usize, training_rows).collect::<Vec<_>>();
		let mut distances = std::iter::repeat_n(0.0, training_rows).collect::<Vec<_>>();
		for _ in 0..config.kmeans_iterations {
			for row in 0..training_rows {
				let sample = &data.samples[row * data.features..(row + 1) * data.features];
				let selected = nearest(sample, &fitted, data.features);
				assignments[row] = selected.0;
				distances[row] = selected.1;
			}
			for cluster in 0..argument {
				let members = assignments
					.iter()
					.enumerate()
					.filter(|value| *value.1 == cluster)
					.map(|value| value.0)
					.collect::<Vec<_>>();
				if members.is_empty() {
					let worst = distances
						.iter()
						.enumerate()
						.max_by(|left, right| left.1.total_cmp(right.1))
						.map(|value| value.0)
						.ok_or_else(|| RecipeError::new("kmeans has no training row"))?;
					fitted[cluster * data.features..(cluster + 1) * data.features]
						.copy_from_slice(&data.samples[worst * data.features..(worst + 1) * data.features]);
					distances[worst] = -1.0;
				} else {
					for feature in 0..data.features {
						fitted[cluster * data.features + feature] = members
							.iter()
							.map(|row| data.samples[row * data.features + feature])
							.sum::<f64>() / members.len() as f64;
					}
				}
			}
		}
	} else {
		fitted.extend_from_slice(&data.samples[..training_rows * data.features]);
		fitted.extend_from_slice(&data.targets[..training_rows]);
	}
	let inputs = &data.samples[training_rows * data.features..];
	Ok((0..test_rows)
		.map(|row| {
			let query = &inputs[row * data.features..(row + 1) * data.features];
			if kind == 0 {
				nearest(query, &fitted, data.features).0 as f64
			} else {
				let mut neighbors = fitted[..training_rows * data.features]
					.chunks_exact(data.features)
					.enumerate()
					.filter(|value| !exclude_self || value.0 != row)
					.map(|value| (distance(query, value.1), fitted[training_rows * data.features + value.0]))
					.collect::<Vec<_>>();
				neighbors.sort_by(|left, right| left.0.total_cmp(&right.0));
				neighbors.iter().take(argument).map(|value| value.1).sum::<f64>() / argument as f64
			}
		})
		.collect())
}

pub(super) fn lower_estimator(
	graph: &mut Graph,
	operation: &Operation,
	data: &Prepared,
	rows: usize,
	backend: Backend,
	config: Config,
) -> Result<()> {
	let input = graph.output;
	let source = graph.source;
	let inputs = graph_inputs(graph, &data.samples, &data.targets, rows, backend)?;
	let mut samples = inputs.clone();
	samples.extend_from_slice(&inputs);
	let mut targets = data.targets[..rows].to_vec();
	targets.extend_from_within(..);
	let paired = Prepared { samples, targets, rows: rows * 2, features: input.elements(), schema: String::new() };
	let teacher = estimator_predict(operation, &paired, rows, config, true)?;
	let hidden = match operation {
		Operation::KMeans(value) | Operation::Knn(value) => *value,
		_ => unreachable!(),
	};
	let surrogate = fit_surrogate(input, &inputs, &teacher, hidden, backend, config)?;
	let first = checked_mul(hidden, input.elements(), "surrogate input")?;
	require(surrogate.len() == first + hidden, "surrogate state has the wrong size")?;
	push_frozen(
		graph,
		Primitive::Contraction,
		input,
		Shape { channels: hidden, length: 1 },
		&surrogate[..first],
		arguments(input.length as f64, 0.0),
		source,
	)?;
	lower_activation(graph, Activation::Tanh, config)?;
	let source = graph.source;
	push_frozen(
		graph,
		Primitive::Contraction,
		Shape { channels: hidden, length: 1 },
		Shape { channels: 1, length: 1 },
		&surrogate[first..],
		[0.0; 9],
		source,
	)
}
