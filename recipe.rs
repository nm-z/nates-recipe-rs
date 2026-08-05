//! Recipe executes one model graph through an explicitly selected host or GPU backend.
//! Attention is three-projection scaled Q/K/V without an output projection; forest is a median-split stump ensemble.
use std::{
	ffi::c_void,
	mem::{size_of, size_of_val},
	ptr,
	sync::OnceLock,
};
#[allow(non_upper_case_globals)]
pub static recipe: Recipe = Recipe;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeError(pub String);
impl RecipeError {
	fn new(message: impl Into<String>) -> Self {
		Self(message.into())
	}
}
pub type Result<T> = std::result::Result<T, RecipeError>;
type Ptr = *mut c_void;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
	Host,
	Amd,
	Nvidia,
}
pub struct Data {
	samples: Vec<f64>,
	targets: Vec<f64>,
	rows: usize,
	features: usize,
	targets_per_row: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Residual {
	Layer(usize),
	Relu,
}
#[derive(Clone, Debug)]
enum Operation {
	Layer(usize),
	Conv(usize, usize),
	Pool(usize),
	KMeans(usize),
	Knn(usize),
	Forest(usize),
	Embed(usize, usize),
	Attention(usize),
	Rnn(usize),
	Gru(usize),
	Lstm(usize),
	Residual(Vec<Residual>),
}
#[derive(Clone, Debug)]
struct Block {
	operation: Operation,
	relu: bool,
}
pub struct Model {
	blocks: Vec<Block>,
	parameters: Option<Vec<f64>>,
}
macro_rules! operation_methods {
	($(fn $method:ident($($argument:ident: $kind:ty),*) = $operation:expr;)+) => {
		$(pub fn $method(self, $($argument: $kind),*) -> Self {
			self.push($operation)
		})+
	};
}
impl Model {
	fn push(mut self, operation: Operation) -> Self {
		self.blocks.push(Block { operation, relu: false });
		self
	}
	pub fn relu(mut self) -> Result<Self> {
		let block = self.blocks.last_mut().ok_or_else(|| RecipeError::new("ReLU requires a preceding block"))?;
		block.relu = true;
		Ok(self)
	}
	operation_methods! {
		fn layer(width: usize) = Operation::Layer(width);
		fn conv(filters: usize, kernel: usize) = Operation::Conv(filters, kernel);
		fn pool(size: usize) = Operation::Pool(size);
		fn kmeans(clusters: usize) = Operation::KMeans(clusters);
		fn knn(neighbors: usize) = Operation::Knn(neighbors);
		fn forest(trees: usize) = Operation::Forest(trees);
		fn embed(dimensions: usize, vocabulary: usize) = Operation::Embed(dimensions, vocabulary);
		fn attn(heads: usize) = Operation::Attention(heads);
		fn rnn(width: usize) = Operation::Rnn(width);
		fn gru(width: usize) = Operation::Gru(width);
		fn lstm(width: usize) = Operation::Lstm(width);
	}
	pub fn residual<const N: usize>(self, parts: [Residual; N]) -> Self {
		self.push(Operation::Residual(parts.into()))
	}
	pub fn parameters(mut self, values: impl IntoIterator<Item = f64>) -> Self {
		self.parameters = Some(values.into_iter().collect());
		self
	}
}
pub struct Recipe;
impl Recipe {
	pub fn data<const R: usize, const F: usize, const T: usize>(
		&self,
		(samples, targets): ([[f64; F]; R], [[f64; T]; R]),
	) -> Data {
		Data {
			samples: samples.into_iter().flatten().collect(),
			targets: targets.into_iter().flatten().collect(),
			rows: R,
			features: F,
			targets_per_row: T,
		}
	}
	pub fn model(&self) -> Model {
		Model { blocks: Vec::new(), parameters: None }
	}
	pub const fn train(&self, backend: Backend) -> Train {
		Train(backend)
	}
}
#[derive(Clone, Copy, Debug)]
struct Shape {
	channels: usize,
	length: usize,
}
impl Shape {
	fn elements(self) -> usize {
		self.channels * self.length
	}
}
#[derive(Clone)]
struct Stage {
	block: Block,
	input: Shape,
	output: Shape,
	offset: usize,
	parameters: usize,
}
struct Graph {
	stages: Vec<Stage>,
	parameters: Vec<f64>,
	output: Shape,
}
fn compile(model: &Model, features: usize, rows: usize, backend: Backend) -> Result<Graph> {
	if model.blocks.is_empty() {
		return Err(RecipeError::new("model must contain a block"));
	}
	if model.blocks.iter().any(|block| is_estimator(&block.operation)) && model.blocks.len() != 1 {
		return Err(RecipeError::new("kmeans, knn, and trees are standalone estimators"));
	}
	let mut shape = Shape { channels: 1, length: features };
	let mut count = 0;
	let mut stages = Vec::with_capacity(model.blocks.len());
	for block in &model.blocks {
		let input = shape;
		let (output, parameters) = compile_block(&block.operation, input, backend)?;
		let output_elements = checked_mul(output.channels, output.length, "block output")?;
		let batch_elements = checked_mul(rows, output_elements, "batch output")?;
		if backend != Backend::Host {
			narrow(batch_elements, "GPU batch output")?;
		}
		stages.push(Stage { block: block.clone(), input, output, offset: count, parameters });
		count = checked_add(count, parameters, "model parameter count")?;
		shape = output;
	}
	let parameters = match &model.parameters {
		Some(values) if values.len() == count => values.clone(),
		Some(_) => return Err(RecipeError::new("model parameter count mismatch")),
		None if count == 0 => Vec::new(),
		None => return Err(RecipeError::new("model parameters must be supplied explicitly")),
	};
	if parameters.iter().any(|value| !value.is_finite()) {
		return Err(RecipeError::new("model parameters must be finite"));
	}
	Ok(Graph { stages, parameters, output: shape })
}
fn compile_block(operation: &Operation, input: Shape, backend: Backend) -> Result<(Shape, usize)> {
	let width = input.elements();
	let result = match operation {
		Operation::Layer(output) => {
			positive(*output, "layer width")?;
			let parameters = checked_mul(*output, width, "layer parameters")?;
			(Shape { channels: *output, length: 1 }, parameters)
		}
		Operation::Conv(filters, kernel) => {
			positive(*filters, "convolution filters")?;
			positive(*kernel, "convolution kernel")?;
			if *kernel > input.length {
				return Err(RecipeError::new("convolution kernel exceeds the input length"));
			}
			let window = checked_mul(input.channels, *kernel, "convolution window")?;
			let parameters = checked_mul(*filters, window, "convolution")?;
			(Shape { channels: *filters, length: input.length - kernel + 1 }, parameters)
		}
		Operation::Pool(size) => {
			positive(*size, "pool window")?;
			(Shape { channels: input.channels, length: input.length.div_ceil(*size) }, 0)
		}
		Operation::Embed(dimensions, vocabulary) => {
			positive(*dimensions, "embedding dimensions")?;
			positive(*vocabulary, "embedding vocabulary")?;
			let parameters = checked_mul(*dimensions, *vocabulary, "embedding table")?;
			(Shape { channels: *dimensions, length: width }, parameters)
		}
		Operation::Attention(heads) => {
			positive(*heads, "attention heads")?;
			if input.channels % heads != 0 {
				return Err(RecipeError::new("attention channels must be divisible by its head count"));
			}
			(
				Shape { channels: input.channels, length: input.length },
				checked_mul(3, checked_mul(input.channels, input.channels, "attention matrix")?, "QKV")?,
			)
		}
		Operation::Rnn(output) => recurrent_shape(input, *output, 1)?,
		Operation::Gru(output) => recurrent_shape(input, *output, 3)?,
		Operation::Lstm(output) => recurrent_shape(input, *output, 4)?,
		Operation::Residual(parts) => residual_shape(input, parts, backend)?,
		Operation::KMeans(size) | Operation::Knn(size) | Operation::Forest(size) => {
			positive(*size, "estimator size")?;
			(Shape { channels: 1, length: 1 }, 0)
		}
	};
	if backend != Backend::Host && is_estimator(operation) {
		return Err(RecipeError::new(format!("{backend:?} does not support standalone estimators")));
	}
	Ok(result)
}
fn recurrent_shape(input: Shape, output: usize, gates: usize) -> Result<(Shape, usize)> {
	positive(output, "recurrent width")?;
	let input_matrix = checked_mul(input.elements(), output, "recurrent input matrix")?;
	let state_matrix = checked_mul(output, output, "recurrent state matrix")?;
	let stride = checked_add(checked_add(input_matrix, state_matrix, "recurrent gate")?, output, "recurrent bias")?;
	Ok((Shape { channels: output, length: 1 }, checked_mul(gates, stride, "recurrent parameters")?))
}
fn residual_shape(input: Shape, parts: &[Residual], backend: Backend) -> Result<(Shape, usize)> {
	if parts.is_empty() {
		return Err(RecipeError::new("residual branch must contain an operation"));
	}
	if backend != Backend::Host && !matches!(parts, [Residual::Layer(_)] | [Residual::Layer(_), Residual::Relu]) {
		return Err(RecipeError::new("GPU residual branches support one layer and an optional ReLU"));
	}
	let input_width = input.elements();
	let mut width = input_width;
	let mut parameters = 0;
	for part in parts {
		if let Residual::Layer(output) = part {
			positive(*output, "residual layer width")?;
			parameters = checked_add(parameters, checked_mul(*output, width, "residual layer")?, "residual")?;
			width = *output;
		}
	}
	if width != input_width {
		return Err(RecipeError::new("residual branch output must match its skip tensor"));
	}
	Ok((Shape { channels: input_width, length: 1 }, parameters))
}
fn is_estimator(operation: &Operation) -> bool {
	matches!(operation, Operation::KMeans(_) | Operation::Knn(_) | Operation::Forest(..))
}
fn checked_add(left: usize, right: usize, role: &str) -> Result<usize> {
	left.checked_add(right).ok_or_else(|| RecipeError::new(format!("{role} overflows")))
}
fn checked_mul(left: usize, right: usize, role: &str) -> Result<usize> {
	left.checked_mul(right).ok_or_else(|| RecipeError::new(format!("{role} overflows")))
}
fn positive(value: usize, role: &str) -> Result<()> {
	if value == 0 { Err(RecipeError::new(format!("{role} must be positive"))) } else { Ok(()) }
}
struct Batch {
	rows: usize,
	shape: Shape,
	values: Vec<f64>,
}
fn forward_host(graph: &Graph, samples: &[f64], rows: usize) -> Result<Batch> {
	let mut batch = Batch { rows, shape: graph.stages[0].input, values: samples.to_vec() };
	for stage in &graph.stages {
		let weights = &graph.parameters[stage.offset..stage.offset + stage.parameters];
		batch = host_stage(&batch, stage, weights)?;
	}
	Ok(batch)
}
fn host_stage(input: &Batch, stage: &Stage, weights: &[f64]) -> Result<Batch> {
	let values = match &stage.block.operation {
		Operation::Layer(width) => dense(input, weights, *width),
		Operation::Conv(filters, kernel) => convolution(input, weights, *filters, *kernel),
		Operation::Pool(size) => pooling(input, *size),
		Operation::Embed(dimensions, vocabulary) => embedding(input, weights, *dimensions, *vocabulary)?,
		Operation::Attention(heads) => attention(input, weights, *heads),
		Operation::Rnn(width) => recurrent(input, weights, *width, Recurrent::Rnn),
		Operation::Gru(width) => recurrent(input, weights, *width, Recurrent::Gru),
		Operation::Lstm(width) => recurrent(input, weights, *width, Recurrent::Lstm),
		Operation::Residual(parts) => residual(input, weights, parts),
		Operation::KMeans(_) | Operation::Knn(_) | Operation::Forest(..) => {
			return Err(RecipeError::new("standalone estimators use fit and predict"));
		}
	};
	let mut batch = Batch { rows: input.rows, shape: stage.output, values };
	if stage.block.relu {
		batch.values.iter_mut().for_each(|value| *value = if *value < 0.0 { 0.0 } else { *value });
	}
	if batch.values.iter().any(|value| !value.is_finite()) {
		return Err(RecipeError::new("model produced a non-finite value"));
	}
	Ok(batch)
}
fn dense(input: &Batch, weights: &[f64], outputs: usize) -> Vec<f64> {
	let inputs = input.shape.elements();
	let mut result = vec![0.0; input.rows * outputs];
	for row in 0..input.rows {
		for output in 0..outputs {
			let mut sum = 0.0;
			for feature in 0..inputs {
				sum += input.values[row * inputs + feature] * weights[feature * outputs + output];
			}
			result[row * outputs + output] = sum;
		}
	}
	result
}
fn convolution(input: &Batch, weights: &[f64], filters: usize, kernel: usize) -> Vec<f64> {
	let positions = input.shape.length - kernel + 1;
	let window = input.shape.channels * kernel;
	let mut result = vec![0.0; input.rows * filters * positions];
	for row in 0..input.rows {
		for filter in 0..filters {
			for position in 0..positions {
				let base = filter * window;
				let mut sum = 0.0;
				for channel in 0..input.shape.channels {
					for offset in 0..kernel {
						let source = row * input.shape.elements();
						let source = source + channel * input.shape.length + position + offset;
						let weight = base + channel * kernel + offset;
						sum += input.values[source] * weights[weight];
					}
				}
				let output = row * filters * positions + filter * positions + position;
				result[output] = sum;
			}
		}
	}
	result
}
fn pooling(input: &Batch, size: usize) -> Vec<f64> {
	let output_length = input.shape.length.div_ceil(size);
	let mut result = vec![f64::NEG_INFINITY; input.rows * input.shape.channels * output_length];
	for row in 0..input.rows {
		for channel in 0..input.shape.channels {
			for window in 0..output_length {
				let start = window * size;
				let end = (start + size).min(input.shape.length);
				let input_base = row * input.shape.elements() + channel * input.shape.length;
				let output = row * input.shape.channels * output_length + channel * output_length + window;
				for index in start..end {
					result[output] = result[output].max(input.values[input_base + index]);
				}
			}
		}
	}
	result
}
fn embedding(input: &Batch, table: &[f64], dimensions: usize, vocabulary: usize) -> Result<Vec<f64>> {
	let tokens = input.shape.elements();
	let mut result = vec![0.0; input.rows * dimensions * tokens];
	for row in 0..input.rows {
		for token in 0..tokens {
			let value = input.values[row * tokens + token];
			if value < 0.0 || value.fract() != 0.0 || value >= vocabulary as f64 {
				return Err(RecipeError::new(format!("embedding index {value} is outside 0..{vocabulary}")));
			}
			let index = value as usize;
			for dimension in 0..dimensions {
				let output = row * dimensions * tokens + dimension * tokens + token;
				result[output] = table[index * dimensions + dimension];
			}
		}
	}
	Ok(result)
}
fn attention(input: &Batch, weights: &[f64], heads: usize) -> Vec<f64> {
	let channels = input.shape.channels;
	let length = input.shape.length;
	let plane = channels * length;
	let matrix = channels * channels;
	let head_width = channels / heads;
	let scale = (head_width as f64).sqrt();
	let mut result = vec![0.0; input.rows * plane];
	let mut qkv = vec![0.0; 3 * plane];
	for row in 0..input.rows {
		let source = &input.values[row * plane..(row + 1) * plane];
		for projection in 0..3 {
			for output in 0..channels {
				for time in 0..length {
					let mut sum = 0.0;
					for channel in 0..channels {
						sum += weights[projection * matrix + output * channels + channel]
							* source[channel * length + time];
					}
					qkv[projection * plane + output * length + time] = sum;
				}
			}
		}
		for head in 0..heads {
			for query in 0..length {
				let scores = (0..length)
					.map(|key| attention_score(&qkv, head, query, key, head_width, length) / scale)
					.collect::<Vec<_>>();
				let maximum = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
				let denominator = scores.iter().map(|score| (score - maximum).exp()).sum::<f64>();
				for channel in 0..head_width {
					let absolute = head * head_width + channel;
					let numerator = scores
						.iter()
						.enumerate()
						.map(|(key, score)| {
							(score - maximum).exp() * qkv[2 * plane + absolute * length + key]
						})
						.sum::<f64>();
					result[row * plane + absolute * length + query] = numerator / denominator;
				}
			}
		}
	}
	result
}
fn attention_score(qkv: &[f64], head: usize, query: usize, key: usize, width: usize, length: usize) -> f64 {
	let plane = qkv.len() / 3;
	(head * width..(head + 1) * width)
		.map(|channel| qkv[channel * length + query] * qkv[plane + channel * length + key])
		.sum()
}
#[derive(Clone, Copy)]
enum Recurrent {
	Rnn = 1,
	Gru = 3,
	Lstm = 4,
}
fn recurrent(input: &Batch, weights: &[f64], width: usize, kind: Recurrent) -> Vec<f64> {
	let gates = kind as usize;
	let features = input.shape.elements();
	let mut result = vec![0.0; input.rows * width];
	let mut state = vec![0.0; width];
	let mut cell = vec![0.0; width];
	for row in 0..input.rows {
		let sample = &input.values[row * features..(row + 1) * features];
		let mut values = vec![0.0; gates * width];
		for gate in 0..gates {
			for output in 0..width {
				values[gate * width + output] =
					recurrent_linear(sample, &state, weights, gate, output, width, None);
			}
		}
		if matches!(kind, Recurrent::Gru) {
			let reset = values[width..2 * width].iter().map(|value| sigmoid(*value)).collect::<Vec<_>>();
			for output in 0..width {
				values[2 * width + output] =
					recurrent_linear(sample, &state, weights, 2, output, width, Some(&reset));
			}
		}
		for output in 0..width {
			state[output] = match kind {
				Recurrent::Rnn => values[output].tanh(),
				Recurrent::Gru => {
					let update = sigmoid(values[output]);
					(1.0 - update) * values[2 * width + output].tanh() + update * state[output]
				}
				Recurrent::Lstm => {
					let input = sigmoid(values[output]);
					let forget = sigmoid(values[width + output]);
					let output_gate = sigmoid(values[2 * width + output]);
					cell[output] = forget * cell[output] + input * values[3 * width + output].tanh();
					output_gate * cell[output].tanh()
				}
			};
		}
		result[row * width..(row + 1) * width].copy_from_slice(&state);
	}
	result
}
fn recurrent_linear(
	input: &[f64],
	state: &[f64],
	weights: &[f64],
	gate: usize,
	output: usize,
	width: usize,
	reset: Option<&[f64]>,
) -> f64 {
	let input_count = input.len() * width;
	let state_count = width * width;
	let stride = input_count + state_count + width;
	let base = gate * stride;
	let mut sum = weights[base + input_count + state_count + output];
	for feature in 0..input.len() {
		sum += input[feature] * weights[base + feature * width + output];
	}
	for hidden in 0..width {
		let state = state[hidden] * reset.map_or(1.0, |reset| reset[hidden]);
		sum += state * weights[base + input_count + hidden * width + output];
	}
	sum
}
fn residual(input: &Batch, weights: &[f64], parts: &[Residual]) -> Vec<f64> {
	let skip = input.values.clone();
	let mut branch = Batch {
		rows: input.rows,
		shape: Shape { channels: input.shape.elements(), length: 1 },
		values: input.values.clone(),
	};
	let mut offset = 0;
	for part in parts {
		match part {
			Residual::Layer(width) => {
				let count = width * branch.shape.elements();
				branch.values = dense(&branch, &weights[offset..offset + count], *width);
				branch.shape.channels = *width;
				offset += count;
			}
			Residual::Relu => {
				for value in &mut branch.values {
					*value = if *value < 0.0 { 0.0 } else { *value };
				}
			}
		}
	}
	branch.values.iter_mut().zip(skip).for_each(|(value, skip)| *value += skip);
	branch.values
}
fn sigmoid(value: f64) -> f64 {
	1.0 / (1.0 + (-value).exp())
}
#[derive(Clone, Copy)]
struct Config {
	epochs: usize,
	rate: f64,
	step: f64,
	kmeans_iterations: usize,
	threads: usize,
}
impl Config {
	fn load() -> Result<Self> {
		Ok(Self {
			epochs: natural("epochs", env!("RECIPE_TRAIN_EPOCHS"))?,
			rate: number("learning rate", env!("RECIPE_TRAIN_LEARNING_RATE"))?,
			step: number("finite difference step", env!("RECIPE_TRAIN_FINITE_DIFFERENCE_STEP"))?,
			kmeans_iterations: natural("kmeans iterations", env!("RECIPE_KMEANS_ITERATIONS"))?,
			threads: natural("GPU threads", env!("RECIPE_GPU_THREADS"))?,
		})
	}
}
fn number(name: &str, text: &str) -> Result<f64> {
	let value = text.parse::<f64>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
	(value.is_finite() && value > 0.0)
		.then_some(value)
		.ok_or_else(|| RecipeError::new(format!("{name} must be finite and positive")))
}
fn natural(name: &str, text: &str) -> Result<usize> {
	let value = text.parse::<usize>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
	positive(value, name).map(|_| value)
}
pub struct Train(Backend);
impl Train {
	pub fn run(&self, model: &Model, data: &Data) -> Result<TrainingReport> {
		let config = Config::load()?;
		prepare(data)?;
		let mut graph = compile(model, data.features, data.rows, self.0)?;
		if is_estimator(&graph.stages[0].block.operation) {
			return fit_estimator(&graph.stages[0].block.operation, data, config);
		}
		let output = graph.output.elements();
		if output != data.targets_per_row {
			return Err(RecipeError::new("model output width does not match target width"));
		}
		let initial_predictions = predict(&graph, data, self.0, config)?;
		let initial_gradient = gradient(&mut graph, data, self.0, config)?;
		for _ in 0..config.epochs {
			let gradient = gradient(&mut graph, data, self.0, config)?;
			graph.parameters
				.iter_mut()
				.zip(gradient)
				.for_each(|(weight, gradient)| *weight -= config.rate * gradient);
		}
		let predictions = predict(&graph, data, self.0, config)?;
		Ok(TrainingReport { initial_predictions, predictions, gradient: initial_gradient })
	}
}
pub struct TrainingReport {
	pub initial_predictions: Vec<f64>,
	pub predictions: Vec<f64>,
	pub gradient: Vec<f64>,
}
fn prepare(data: &Data) -> Result<()> {
	positive(data.rows, "data rows")?;
	positive(data.features, "data features")?;
	positive(data.targets_per_row, "targets per row")?;
	if data.samples.iter().chain(&data.targets).any(|value| !value.is_finite()) {
		return Err(RecipeError::new("data must contain only finite values"));
	}
	Ok(())
}
fn predict(graph: &Graph, data: &Data, backend: Backend, config: Config) -> Result<Vec<f64>> {
	let batch = match backend {
		Backend::Host => forward_host(graph, &data.samples, data.rows)?,
		Backend::Amd | Backend::Nvidia => forward_gpu(graph, &data.samples, data.rows, backend, config)?,
	};
	Ok(batch.values)
}
fn gradient(graph: &mut Graph, data: &Data, backend: Backend, config: Config) -> Result<Vec<f64>> {
	let mut gradient = Vec::with_capacity(graph.parameters.len());
	for parameter in 0..graph.parameters.len() {
		let original = graph.parameters[parameter];
		graph.parameters[parameter] = original + config.step;
		let plus = loss(&predict(graph, data, backend, config)?, data)?;
		graph.parameters[parameter] = original - config.step;
		let minus = loss(&predict(graph, data, backend, config)?, data)?;
		graph.parameters[parameter] = original;
		gradient.push((plus - minus) / (2.0 * config.step));
	}
	Ok(gradient)
}
fn loss(predictions: &[f64], data: &Data) -> Result<f64> {
	let count = data.rows * data.targets_per_row;
	let result = predictions[..count]
		.iter()
		.zip(&data.targets[..count])
		.map(|(prediction, target)| (prediction - target).powi(2))
		.sum::<f64>()
		/ count as f64;
	if result.is_finite() { Ok(result) } else { Err(RecipeError::new("loss is not finite")) }
}
fn fit_estimator(operation: &Operation, data: &Data, config: Config) -> Result<TrainingReport> {
	if data.targets_per_row != 1 {
		return Err(RecipeError::new("standalone estimators require one target per row"));
	}
	let predictions = match operation {
		Operation::KMeans(clusters) => kmeans_predict(data, *clusters, config)?,
		Operation::Knn(neighbors) => knn_predict(data, *neighbors)?,
		Operation::Forest(trees) => forest_predict(data, *trees)?,
		_ => return Err(RecipeError::new("operation is not a supported estimator")),
	};
	if predictions.iter().any(|value| !value.is_finite()) {
		return Err(RecipeError::new("estimator produced a non-finite prediction"));
	}
	Ok(TrainingReport { initial_predictions: predictions.clone(), predictions, gradient: Vec::new() })
}
fn kmeans_predict(data: &Data, clusters: usize, config: Config) -> Result<Vec<f64>> {
	if clusters > data.rows {
		return Err(RecipeError::new("kmeans clusters exceed training rows"));
	}
	let mut centroids = data.samples[..clusters * data.features].to_vec();
	for _ in 0..config.kmeans_iterations {
		let assignments = nearest_centroids(&data.samples, &centroids, data.features)?;
		for cluster in 0..clusters {
			for feature in 0..data.features {
				let members = assignments.iter().enumerate().filter(|(_, value)| **value == cluster);
				let (sum, count) = members.fold((0.0, 0), |(sum, count), (row, _)| {
					(sum + data.samples[row * data.features + feature], count + 1)
				});
				if count == 0 {
					return Err(RecipeError::new("kmeans produced an empty cluster"));
				}
				centroids[cluster * data.features + feature] = sum / count as f64;
			}
		}
	}
	Ok(nearest_centroids(&data.samples, &centroids, data.features)?
		.into_iter()
		.map(|cluster| cluster as f64)
		.collect())
}
fn nearest_centroids(samples: &[f64], centroids: &[f64], features: usize) -> Result<Vec<usize>> {
	samples
		.chunks_exact(features)
		.map(|sample| {
			centroids
				.chunks_exact(features)
				.enumerate()
				.min_by(|(_, left), (_, right)| distance(sample, left).total_cmp(&distance(sample, right)))
				.map(|(cluster, _)| cluster)
				.ok_or_else(|| RecipeError::new("kmeans requires a centroid"))
		})
		.collect()
}
fn knn_predict(data: &Data, neighbors: usize) -> Result<Vec<f64>> {
	if neighbors > data.rows {
		return Err(RecipeError::new("knn neighbors exceed training rows"));
	}
	Ok(data
		.samples
		.chunks_exact(data.features)
		.map(|sample| {
			let mut rows = (0..data.rows).collect::<Vec<_>>();
			rows.sort_by(|left, right| {
				let left = &data.samples[*left * data.features..(*left + 1) * data.features];
				let right = &data.samples[*right * data.features..(*right + 1) * data.features];
				distance(sample, left).total_cmp(&distance(sample, right))
			});
			rows[..neighbors].iter().map(|row| data.targets[*row]).sum::<f64>() / neighbors as f64
		})
		.collect())
}
fn distance(left: &[f64], right: &[f64]) -> f64 {
	left.iter().zip(right).map(|(left, right)| (left - right).powi(2)).sum()
}
struct Stump(usize, f64, f64, f64);
fn forest_predict(data: &Data, trees: usize) -> Result<Vec<f64>> {
	if data.rows < 2 {
		return Err(RecipeError::new("forest fitting requires at least two training rows"));
	}
	let forest = (0..trees).map(|tree| fit_stump(data, tree % data.features)).collect::<Vec<_>>();
	Ok(data
		.samples
		.chunks_exact(data.features)
		.map(|sample| {
			forest.iter()
				.map(
					|Stump(feature, threshold, left, right)| {
						if sample[*feature] <= *threshold { *left } else { *right }
					},
				)
				.sum::<f64>() / trees as f64
		})
		.collect())
}
fn fit_stump(data: &Data, feature: usize) -> Stump {
	let mut ordered = (0..data.rows).collect::<Vec<_>>();
	ordered.sort_by(|left, right| {
		data.samples[*left * data.features + feature].total_cmp(&data.samples[*right * data.features + feature])
	});
	let split = ordered.len() / 2;
	let left_value = data.samples[ordered[split - 1] * data.features + feature];
	let right_value = data.samples[ordered[split] * data.features + feature];
	let mean = |rows: &[usize]| rows.iter().map(|row| data.targets[*row]).sum::<f64>() / rows.len() as f64;
	if left_value == right_value {
		let value = mean(&ordered);
		return Stump(feature, left_value, value, value);
	}
	Stump(feature, left_value, mean(&ordered[..split]), mean(&ordered[split..]))
}
fn forward_gpu(graph: &Graph, samples: &[f64], rows: usize, backend: Backend, config: Config) -> Result<Batch> {
	let gpu = gpu(backend)?;
	let mut descriptors = Vec::new();
	let mut parameters = Vec::new();
	let mut values = Vec::new();
	let mut contexts = Vec::new();
	for stage in &graph.stages {
		let (operation, parameter, secondary, context) = gpu_operation(stage, rows)?;
		descriptors.extend([
			narrow(stage.input.elements(), "GPU input width")?,
			narrow(stage.output.elements(), "GPU output width")?,
			narrow(stage.offset, "GPU weight offset")?,
			operation,
			if stage.block.relu { 8 } else { 0 },
		]);
		parameters.extend([parameter, secondary]);
		values.push(Buffer::new(gpu, rows * stage.output.elements() * size_of::<f64>())?);
		contexts.push(Buffer::new(gpu, context.max(1) * size_of::<f64>())?);
	}
	let addresses = |buffers: &[Buffer]| buffers.iter().map(|buffer| buffer.pointer).collect::<Vec<_>>();
	let mut sample_buffer = Buffer::upload(gpu, samples)?;
	let mut weight_buffer = if graph.parameters.is_empty() {
		Buffer::new(gpu, size_of::<f64>())?
	} else {
		Buffer::upload(gpu, &graph.parameters)?
	};
	let mut value_pointers = Buffer::upload(gpu, &addresses(&values))?;
	let mut context_pointers = Buffer::upload(gpu, &addresses(&contexts))?;
	let mut descriptors = Buffer::upload(gpu, &descriptors)?;
	let mut parameters = Buffer::upload(gpu, &parameters)?;
	let mut row_count = narrow(rows, "GPU rows")? as u32;
	let mut stage_count = narrow(graph.stages.len(), "GPU stages")? as u32;
	let mut thread_count = narrow(config.threads, "GPU threads")? as u32;
	let mut arguments = [
		&mut sample_buffer.pointer as *mut _ as *mut c_void,
		&mut weight_buffer.pointer as *mut _ as *mut c_void,
		&mut value_pointers.pointer as *mut _ as *mut c_void,
		&mut context_pointers.pointer as *mut _ as *mut c_void,
		&mut descriptors.pointer as *mut _ as *mut c_void,
		&mut parameters.pointer as *mut _ as *mut c_void,
		&mut row_count as *mut _ as *mut c_void,
		&mut stage_count as *mut _ as *mut c_void,
		&mut thread_count as *mut _ as *mut c_void,
	];
	gpu.launch(thread_count, &mut arguments)?;
	let output = values.last().ok_or_else(|| RecipeError::new("GPU graph has no output"))?;
	let predictions = output.download::<f64>(rows * graph.output.elements())?;
	if predictions.iter().any(|value| !value.is_finite()) {
		return Err(RecipeError::new("GPU graph produced a non-finite value"));
	}
	Ok(Batch { rows, shape: graph.output, values: predictions })
}
fn gpu_operation(stage: &Stage, rows: usize) -> Result<(i32, f64, f64, usize)> {
	let width = stage.output.elements();
	let scalar = |value| narrow(value, "GPU parameter").map(f64::from);
	Ok(match &stage.block.operation {
		Operation::Layer(_) => (0, 0.0, 0.0, 1),
		Operation::Conv(_, kernel) => (1, scalar(*kernel)?, scalar(stage.input.channels)?, 1),
		Operation::Pool(size) => (2, scalar(*size)?, scalar(stage.input.channels)?, 1),
		Operation::Embed(_, vocabulary) => (4, scalar(*vocabulary)?, 0.0, 1),
		Operation::Attention(heads) => {
			(5, scalar(*heads)?, scalar(stage.input.channels)?, gpu_context(rows, width, 3)?)
		}
		Operation::Rnn(_) => (6, 0.0, 0.0, gpu_context(rows, width, 6)?),
		Operation::Gru(_) => (7, 0.0, 0.0, gpu_context(rows, width, 10)?),
		Operation::Lstm(_) => (8, 0.0, 0.0, gpu_context(rows, width, 12)?),
		Operation::Residual(parts) => {
			let relu = f64::from(parts.iter().any(|part| matches!(part, Residual::Relu)));
			(11, relu, 0.0, 1)
		}
		Operation::KMeans(_) | Operation::Knn(_) | Operation::Forest(_) => {
			return Err(RecipeError::new("GPU standalone estimators are unsupported"));
		}
	})
}
fn gpu_context(rows: usize, width: usize, factor: usize) -> Result<usize> {
	let context = checked_mul(factor, checked_mul(rows, width, "GPU context")?, "GPU context")?;
	narrow(context, "GPU context").map(|_| context)
}
fn narrow(value: usize, role: &str) -> Result<i32> {
	i32::try_from(value).map_err(|_| RecipeError::new(format!("{role} exceeds i32")))
}
struct Buffer {
	runtime: &'static Gpu,
	pointer: u64,
}
impl Buffer {
	fn new(runtime: &'static Gpu, bytes: usize) -> Result<Self> {
		let mut pointer = 0;
		runtime.status(unsafe { (runtime.allocate)(&mut pointer, bytes) }, "allocation")?;
		Ok(Self { runtime, pointer })
	}
	fn upload<T>(runtime: &'static Gpu, values: &[T]) -> Result<Self> {
		let buffer = Self::new(runtime, size_of_val(values))?;
		runtime.status(
			unsafe { (runtime.upload)(buffer.pointer, values.as_ptr().cast(), size_of_val(values)) },
			"upload",
		)?;
		Ok(buffer)
	}
	fn download<T: Copy + Default>(&self, count: usize) -> Result<Vec<T>> {
		self.runtime.status(unsafe { (self.runtime.synchronize)() }, "synchronization")?;
		let mut values = vec![T::default(); count];
		self.runtime.status(
			unsafe { (self.runtime.download)(values.as_mut_ptr().cast(), self.pointer, size_of_val(&*values)) },
			"download",
		)?;
		Ok(values)
	}
}
impl Drop for Buffer {
	fn drop(&mut self) {
		unsafe {
			(self.runtime.free)(self.pointer);
		}
	}
}
struct Gpu {
	backend: Backend,
	allocate: unsafe extern "C" fn(*mut u64, usize) -> i32,
	free: unsafe extern "C" fn(u64) -> i32,
	upload: unsafe extern "C" fn(u64, *const c_void, usize) -> i32,
	download: unsafe extern "C" fn(Ptr, u64, usize) -> i32,
	synchronize: unsafe extern "C" fn() -> i32,
	launch: Launch,
	kernel: usize,
}
type Launch = unsafe extern "C" fn(usize, u32, u32, u32, u32, u32, u32, u32, Ptr, *mut Ptr, *mut Ptr) -> i32;
#[cfg(any(feature = "amd", feature = "nvidia"))]
macro_rules! launch_declaration {
	($name:ident) => {
		fn $name(
			function: usize,
			grid_x: u32,
			grid_y: u32,
			grid_z: u32,
			block_x: u32,
			block_y: u32,
			block_z: u32,
			shared_memory: u32,
			stream: Ptr,
			arguments: *mut Ptr,
			extra: *mut Ptr,
		) -> i32;
	};
}
impl Gpu {
	fn status(&self, status: i32, action: &str) -> Result<()> {
		if status == 0 {
			Ok(())
		} else {
			Err(RecipeError::new(format!("{:?} {action} failed: {status}", self.backend)))
		}
	}
	fn launch(&self, threads: u32, arguments: &mut [*mut c_void]) -> Result<()> {
		let stream: Ptr = ptr::null_mut();
		let extra: *mut Ptr = ptr::null_mut();
		let status = unsafe {
			(self.launch)(self.kernel, 1, 1, 1, threads, 1, 1, 0, stream, arguments.as_mut_ptr(), extra)
		};
		self.status(status, "dispatch")
	}
}
static AMD: OnceLock<Result<Gpu>> = OnceLock::new();
static NVIDIA: OnceLock<Result<Gpu>> = OnceLock::new();
fn gpu(backend: Backend) -> Result<&'static Gpu> {
	let loaded = match backend {
		Backend::Amd => AMD.get_or_init(load_amd),
		Backend::Nvidia => NVIDIA.get_or_init(load_nvidia),
		Backend::Host => return Err(RecipeError::new("host is not a GPU backend")),
	};
	match loaded {
		Ok(runtime) => Ok(runtime),
		Err(error) => Err(error.clone()),
	}
}
fn load_amd() -> Result<Gpu> {
	#[cfg(not(feature = "amd"))]
	return Err(RecipeError::new("AMD support is not compiled into this build"));
	#[cfg(feature = "amd")]
	unsafe {
		let mut module = ptr::null_mut();
		let mut kernel = 0;
		let gpu = Gpu {
			backend: Backend::Amd,
			allocate: hipMalloc,
			free: hipFree,
			upload: hipMemcpyHtoD,
			download: hipMemcpyDtoH,
			synchronize: hipDeviceSynchronize,
			launch: hipModuleLaunchKernel,
			kernel: 0,
		};
		gpu.status(hipInit(0), "initialization")?;
		gpu.status(hipSetDevice(0), "device selection")?;
		gpu.status(
			hipModuleLoad(&mut module, concat!(env!("RECIPE_HSA_CODE_OBJECT"), "\0").as_ptr()),
			"module load",
		)?;
		gpu.status(hipModuleGetFunction(&mut kernel, module, b"forward_graph\0".as_ptr()), "kernel load")?;
		Ok(Gpu { kernel, ..gpu })
	}
}
fn load_nvidia() -> Result<Gpu> {
	#[cfg(not(feature = "nvidia"))]
	return Err(RecipeError::new("NVIDIA support is not compiled into this build"));
	#[cfg(feature = "nvidia")]
	unsafe {
		let mut device = 0;
		let mut context = ptr::null_mut();
		let mut module = ptr::null_mut();
		let mut kernel = 0;
		let gpu = Gpu {
			backend: Backend::Nvidia,
			allocate: cuMemAlloc_v2,
			free: cuMemFree_v2,
			upload: cuMemcpyHtoD_v2,
			download: cuMemcpyDtoH_v2,
			synchronize: cuCtxSynchronize,
			launch: cuLaunchKernel,
			kernel: 0,
		};
		gpu.status(cuInit(0), "initialization")?;
		gpu.status(cuDeviceGet(&mut device, 0), "device selection")?;
		gpu.status(cuCtxCreate_v2(&mut context, 0, device), "context creation")?;
		gpu.status(cuModuleLoad(&mut module, concat!(env!("RECIPE_NV_MODULE"), "\0").as_ptr()), "module load")?;
		gpu.status(cuModuleGetFunction(&mut kernel, module, b"forward_graph\0".as_ptr()), "kernel load")?;
		Ok(Gpu { kernel, ..gpu })
	}
}
#[cfg(feature = "amd")]
#[link(name = "amdhip64")]
unsafe extern "C" {
	fn hipInit(flags: u32) -> i32;
	fn hipSetDevice(device: i32) -> i32;
	fn hipMalloc(pointer: *mut u64, bytes: usize) -> i32;
	fn hipFree(pointer: u64) -> i32;
	fn hipMemcpyHtoD(destination: u64, source: *const c_void, bytes: usize) -> i32;
	fn hipMemcpyDtoH(destination: Ptr, source: u64, bytes: usize) -> i32;
	fn hipDeviceSynchronize() -> i32;
	fn hipModuleLoad(module: *mut *mut c_void, path: *const u8) -> i32;
	fn hipModuleGetFunction(function: *mut usize, module: *mut c_void, name: *const u8) -> i32;
	launch_declaration!(hipModuleLaunchKernel);
}
#[cfg(feature = "nvidia")]
#[link(name = "cuda")]
unsafe extern "C" {
	fn cuInit(flags: u32) -> i32;
	fn cuDeviceGet(device: *mut i32, ordinal: i32) -> i32;
	fn cuCtxCreate_v2(context: *mut *mut c_void, flags: u32, device: i32) -> i32;
	fn cuMemAlloc_v2(pointer: *mut u64, bytes: usize) -> i32;
	fn cuMemFree_v2(pointer: u64) -> i32;
	fn cuMemcpyHtoD_v2(destination: u64, source: *const c_void, bytes: usize) -> i32;
	fn cuMemcpyDtoH_v2(destination: *mut c_void, source: u64, bytes: usize) -> i32;
	fn cuCtxSynchronize() -> i32;
	fn cuModuleLoad(module: *mut *mut c_void, path: *const u8) -> i32;
	fn cuModuleGetFunction(function: *mut usize, module: *mut c_void, name: *const u8) -> i32;
	launch_declaration!(cuLaunchKernel);
}
