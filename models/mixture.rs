use super::*;

const ACTIVATIONS: [Activation; 16] = [
	Activation::Linear,
	Activation::Cos,
	Activation::Exp,
	Activation::Log,
	Activation::Ln,
	Activation::Huber,
	Activation::Tan,
	Activation::Relu,
	Activation::Leak,
	Activation::Sigmoid,
	Activation::Tanh,
	Activation::Selu,
	Activation::Gelu,
	Activation::Silu,
	Activation::Elu,
	Activation::Prelu,
];

fn reset(graph: &mut Graph, source: i32, shape: Shape) {
	graph.source = source;
	graph.output = shape;
}

fn program(
	graph: &mut Graph,
	first: i32,
	second: i32,
	shape: Shape,
	initial: &[f64],
	program: ScalarProgram,
) -> Result<i32> {
	reset(graph, first, shape);
	push_program(graph, second, initial, program)?;
	Ok(graph.source)
}

fn binary(graph: &mut Graph, first: i32, second: i32, shape: Shape, opcode: ScalarOpcode) -> Result<i32> {
	let mut scalar = ScalarProgram(Vec::new());
	scalar.op(opcode, -1.0, -2.0);
	program(graph, first, second, shape, &[], scalar)
}

fn constant(graph: &mut Graph, source: i32, shape: Shape, value: f64) -> Result<i32> {
	let mut scalar = ScalarProgram(Vec::new());
	scalar.constant(value);
	program(graph, source, -2, shape, &[], scalar)
}

fn parameter(graph: &mut Graph, source: i32, shape: Shape) -> Result<i32> {
	let mut scalar = ScalarProgram(Vec::new());
	scalar.op(ScalarOpcode::Parameter, 0.0, 0.0);
	program(graph, source, -2, shape, &[0.0], scalar)
}

fn activation(
	graph: &mut Graph,
	source: i32,
	shape: Shape,
	value: Activation,
	config: Config,
) -> Result<(i32, Shape)> {
	reset(graph, source, shape);
	if value != Activation::Linear {
		lower_activation(graph, value, config)?;
	}
	Ok((graph.source, graph.output))
}

fn expert(graph: &mut Graph, source: i32, shape: Shape, value: &Residual, config: Config) -> Result<(i32, Shape)> {
	reset(graph, source, shape);
	match value {
		Residual::Layer(width) => lower_project(graph, *width)?,
		Residual::Activation(value) if *value != Activation::Linear => lower_activation(graph, *value, config)?,
		Residual::Activation(_) => {}
	}
	Ok((graph.source, graph.output))
}

fn maximum(graph: &mut Graph, first: i32, second: i32, shape: Shape) -> Result<i32> {
	let mut scalar = ScalarProgram(Vec::new());
	let condition = scalar.op(ScalarOpcode::Greater, -1.0, -2.0);
	scalar.choose(condition, -1.0, -2.0);
	program(graph, first, second, shape, &[], scalar)
}

fn one_minus(graph: &mut Graph, source: i32, shape: Shape) -> Result<i32> {
	let mut scalar = ScalarProgram(Vec::new());
	let one = scalar.constant(1.0);
	scalar.op(ScalarOpcode::Subtract, one, -1.0);
	program(graph, source, -2, shape, &[], scalar)
}

fn greater_than(graph: &mut Graph, value: f64, source: i32, shape: Shape) -> Result<i32> {
	let mut scalar = ScalarProgram(Vec::new());
	let limit = scalar.constant(value);
	scalar.op(ScalarOpcode::Greater, limit, -1.0);
	program(graph, source, -2, shape, &[], scalar)
}

fn rank_mask(graph: &mut Graph, scores: &[i32], selected: usize, shape: Shape, top_k: usize) -> Result<i32> {
	let mut rank = constant(graph, scores[selected], shape, 0.0)?;
	for candidate in 0..scores.len() {
		if candidate == selected {
			continue;
		}
		let higher = binary(graph, scores[candidate], scores[selected], shape, ScalarOpcode::Greater)?;
		let order = if candidate < selected {
			let lower = binary(graph, scores[selected], scores[candidate], shape, ScalarOpcode::Greater)?;
			let unequal = binary(graph, higher, lower, shape, ScalarOpcode::Add)?;
			let tied = one_minus(graph, unequal, shape)?;
			binary(graph, higher, tied, shape, ScalarOpcode::Add)?
		} else {
			higher
		};
		rank = binary(graph, rank, order, shape, ScalarOpcode::Add)?;
	}
	greater_than(graph, top_k as f64, rank, shape)
}

fn select(
	graph: &mut Graph,
	branches: &[i32],
	scores: &[i32],
	shape: Shape,
	top_k: usize,
	config: Config,
) -> Result<()> {
	let mut maximum_score = scores[0];
	for &score in &scores[1..] {
		maximum_score = maximum(graph, maximum_score, score, shape)?;
	}
	let mut exponentials = Vec::with_capacity(scores.len());
	for &score in scores {
		let centered = binary(graph, score, maximum_score, shape, ScalarOpcode::Subtract)?;
		exponentials.push(activation(graph, centered, shape, Activation::Exp, config)?.0);
	}
	let mut denominator = exponentials[0];
	for &value in &exponentials[1..] {
		denominator = binary(graph, denominator, value, shape, ScalarOpcode::Add)?;
	}
	let mut output = None;
	for index in 0..branches.len() {
		let probability = binary(graph, exponentials[index], denominator, shape, ScalarOpcode::Divide)?;
		let mask = rank_mask(graph, scores, index, shape, top_k)?;
		let routed = binary(graph, probability, branches[index], shape, ScalarOpcode::Multiply)?;
		let routed = binary(graph, mask, routed, shape, ScalarOpcode::Multiply)?;
		output = Some(match output {
			Some(previous) => binary(graph, previous, routed, shape, ScalarOpcode::Add)?,
			None => routed,
		});
	}
	reset(graph, output.ok_or_else(|| RecipeError::new("mixture has no output"))?, shape);
	Ok(())
}

pub(super) fn lower_moe(
	graph: &mut Graph,
	top_k: usize,
	experts: &[Residual],
	config: Config,
) -> Result<()> {
	require(!experts.is_empty(), "moe requires an expert")?;
	require(top_k != 0 && top_k <= experts.len(), "moe top-k is invalid")?;
	let source = graph.source;
	let input = graph.output;
	let mut branches = Vec::with_capacity(experts.len());
	let mut output = None;
	for value in experts {
		let (branch, shape) = expert(graph, source, input, value, config)?;
		if let Some(expected) = output {
			require(shape == expected, "moe experts must have one output shape")?;
		}
		output = Some(shape);
		branches.push(branch);
	}
	let output = output.ok_or_else(|| RecipeError::new("moe has no output shape"))?;
	let mut scores = Vec::with_capacity(experts.len());
	for _ in experts {
		reset(graph, source, input);
		lower_project(graph, output.channels)?;
		require(graph.output == output, "moe router shape does not match its experts")?;
		scores.push(graph.source);
	}
	select(graph, &branches, &scores, output, top_k, config)
}

pub(super) fn lower_svm(graph: &mut Graph, choices: &[Activation], config: Config) -> Result<()> {
	let choices = if choices.is_empty() { &ACTIVATIONS } else { choices };
	let source = graph.source;
	let shape = graph.output;
	let mut branches = Vec::with_capacity(choices.len());
	let mut scores = Vec::with_capacity(choices.len());
	for &choice in choices {
		branches.push(activation(graph, source, shape, choice, config)?.0);
		scores.push(parameter(graph, source, shape)?);
	}
	select(graph, &branches, &scores, shape, 1, config)
}
