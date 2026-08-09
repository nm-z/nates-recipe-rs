//! Recipe executes one model graph after automatically probing a compiled discrete GPU backend.
//! Attention is three-projection scaled Q/K/V without an output projection.
#![allow(non_upper_case_globals)]
mod bundle {
	use super::*;
	use std::{collections::BTreeMap, io::Write as _, str::FromStr};
	#[derive(Clone)]
	pub(super) struct StoredGraph {
		pub graph: Graph,
		pub inputs: Vec<String>,
		pub outputs: Vec<String>,
		pub norm_mean: Vec<f64>,
		pub norm_scale: Vec<f64>,
		pub target_min: f64,
		pub target_span: f64,
		pub bn_stats: Vec<f64>,
	}
	#[derive(Default)]
	struct Builder {
		inputs: Vec<String>,
		outputs: Vec<String>,
		input: Option<Shape>,
		output: Option<Shape>,
		nodes: Vec<Node>,
		arguments: usize,
		parameters: Vec<f64>,
		frozen: Vec<u8>,
		programs: Vec<f64>,
		state: TrainingState,
		norm_mean: Vec<f64>,
		norm_scale: Vec<f64>,
		target_min: f64,
		target_span: f64,
		bn_stats: Vec<f64>,
	}
	impl Builder {
		fn finish(self) -> Result<StoredGraph> {
			let (input, output) = (
				self.input.ok_or_else(|| RecipeError::new("model graph has no input shape"))?,
				self.output.ok_or_else(|| RecipeError::new("model graph has no output shape"))?,
			);
			require(!self.nodes.is_empty(), "model graph has no nodes")?;
			require(self.arguments == self.nodes.len(), "model graph node arguments are incomplete")?;
			require(self.parameters.len() == self.frozen.len(), "model graph frozen weights are incomplete")?;
			for (name, values) in [
				("moments", &self.state.moments),
				("variances", &self.state.variances),
				("best weights", &self.state.best),
			] {
				require(
					values.is_empty() || values.len() == self.parameters.len(),
					format!("model graph {name} are incomplete"),
				)?;
			}
			require(
				self.state.best_loss.is_empty() || self.state.best_loss.len() == 4,
				"model graph best loss state is incomplete",
			)?;
			require(self.inputs.len() == input.elements(), "model graph input schema has the wrong width")?;
			require(self.outputs.len() == output.elements(), "model graph output schema has the wrong width")?;
			require(
				self.norm_mean.len() == self.norm_scale.len()
					&& (self.norm_mean.is_empty() || self.norm_mean.len() == self.inputs.len()),
				"model graph normalization stats have the wrong width",
			)?;
			let source = self.nodes.len() as i32 - 1;
			Ok(StoredGraph {
				graph: Graph {
					nodes: self.nodes, parameters: self.parameters, frozen: self.frozen,
					programs: self.programs, input, output, source, state: self.state,
					block_index: 0, block_kind: "",
				},
				inputs: self.inputs,
				outputs: self.outputs,
				norm_mean: self.norm_mean, norm_scale: self.norm_scale,
				target_min: self.target_min, target_span: self.target_span,
				bn_stats: self.bn_stats,
			})
		}
	}
	fn values<T: FromStr>(text: &str, role: &str) -> Result<Vec<T>>
	where
		T::Err: fmt::Display,
	{
		text.split_whitespace()
			.map(|value| value.parse().map_err(|error| RecipeError::new(format!("invalid {role}: {error}"))))
			.collect()
	}
	fn primitive(value: i32) -> Result<Primitive> {
		match value {
			0 => Ok(Primitive::Contraction),
			2 => Ok(Primitive::Pool),
			3 => Ok(Primitive::Gather),
			4 => Ok(Primitive::Attention),
			5 => Ok(Primitive::Scan),
			6 => Ok(Primitive::Elementwise),
			7 => Ok(Primitive::Route),
			8 => Ok(Primitive::Normalize),
			_ => Err(RecipeError::new(format!("invalid primitive {value}"))),
		}
	}
	fn node(value: &str) -> Result<Node> {
		let value = values::<i32>(value, "node descriptor")?;
		require(value.len() == 11, "model graph node descriptor has the wrong width")?;
		Ok(Node {
			op: primitive(value[0])?, source: value[1], second: value[2],
			input: Shape { channels: value[3] as usize, length: value[4] as usize },
			output: Shape { channels: value[5] as usize, length: value[6] as usize },
			offset: value[7] as usize, parameters: value[8] as usize, argument: [0.0; 9],
			program_offset: value[9] as usize, program_count: value[10] as usize,
			block_index: 0, block_kind: "",
		})
	}
	pub(super) fn load(path: &str) -> Result<(String, Vec<StoredGraph>)> {
		require(path.ends_with(".ogdl"), "model path requires .ogdl")?;
		let document =
			fs::read_to_string(path).map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
		let (mut schema, mut graphs) = (String::new(), Vec::new());
		let mut current: Option<Builder> = None;
		for line in document.lines().map(str::trim) {
			if line == "recipe-model" {
				continue;
			}
			if line == "graph" {
				if let Some(value) = current.take() {
					graphs.push(value.finish()?)
				}
				current = Some(Builder::default());
				continue;
			}
			let (kind, value) = line.split_once(' ').unwrap_or((line, ""));
			if kind == "schema" {
				schema = value.to_owned();
				continue;
			}
			let builder = current.as_mut().ok_or_else(|| RecipeError::new("model value precedes graph"))?;
			match kind {
				"in" => builder.inputs.push(value.to_owned()),
				"out" => builder.outputs.push(value.to_owned()),
				"shape" => {
					let shape = values::<usize>(value, "model shape")?;
					require(shape.len() == 4, "model graph shape has the wrong width")?;
					builder.input = Some(Shape { channels: shape[0], length: shape[1] });
					builder.output = Some(Shape { channels: shape[2], length: shape[3] });
				}
				"node" => builder.nodes.push(node(value)?),
				"arguments" => {
					let argument = values::<f64>(value, "node argument")?;
					require(argument.len() == 9, "model graph node argument has the wrong width")?;
					builder
						.nodes
						.last_mut()
						.ok_or_else(|| RecipeError::new("argument precedes node"))?
						.argument
						.copy_from_slice(&argument);
					builder.arguments += 1;
				}
				"programs" => builder.programs = values(value, "scalar program")?,
				"norm_mean" => builder.norm_mean = values(value, "normalization mean")?,
				"norm_scale" => builder.norm_scale = values(value, "normalization scale")?,
				"target_min" => {
					builder.target_min = value.parse()
						.map_err(|error| RecipeError::new(format!("invalid target min: {error}")))?
				}
				"target_span" => {
					builder.target_span = value.parse()
						.map_err(|error| RecipeError::new(format!("invalid target span: {error}")))?
				}
				"bn_stats" => builder.bn_stats = values(value, "batch norm stats")?,
				"weights" => builder.parameters = values(value, "weight")?,
				"frozen" => builder.frozen = values(value, "frozen weight")?,
				"moments" => builder.state.moments = values(value, "Adam moment")?,
				"variances" => builder.state.variances = values(value, "Adam variance")?,
				"best" => builder.state.best = values(value, "best weight")?,
				"best_loss" => builder.state.best_loss = values(value, "best loss")?,
				"epoch" => {
					builder.state.epoch = value.parse()
						.map_err(|error| RecipeError::new(format!("invalid epoch: {error}")))?
				}
				"training_rows" => {
					builder.state.training_rows = value.parse()
						.map_err(|error| RecipeError::new(format!("invalid training_rows: {error}")))?
				}
				"trained_samples" => builder.state.trained_samples = values(value, "trained sample identity")?,
				"" => {}
				_ => return Err(RecipeError::new(format!("invalid model value: {line}"))),
			}
		}
		if let Some(value) = current {
			graphs.push(value.finish()?)
		}
		require(!graphs.is_empty(), "model has no graphs")?;
		Ok((schema, graphs))
	}
	fn join<T: ToString>(values: &[T]) -> String {
		values.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")
	}
	pub(super) fn save(path: &str, schema: &str, graphs: &[StoredGraph]) -> Result<()> {
		require(path.ends_with(".ogdl"), "save requires an .ogdl model")?;
		require(!graphs.is_empty(), "model bundle has no graphs")?;
		fn field(document: &mut String, key: &str, value: &str) {
			document.push_str(&format!("        {key} {value}\n"));
		}
		let mut document = format!("recipe-model\n    schema {schema}\n");
		for stored in graphs {
			document.push_str("    graph\n");
			for name in &stored.inputs { document.push_str(&format!("        in {name}\n")) }
			for name in &stored.outputs { document.push_str(&format!("        out {name}\n")) }
			let graph = &stored.graph;
			field(&mut document, "shape", &format!("{} {} {} {}", graph.input.channels, graph.input.length, graph.output.channels, graph.output.length));
			for node in &graph.nodes {
				let d = [node.op as i32, node.source, node.second, node.input.channels as i32, node.input.length as i32,
					node.output.channels as i32, node.output.length as i32, node.offset as i32, node.parameters as i32,
					node.program_offset as i32, node.program_count as i32];
				field(&mut document, "node", &join(&d));
				field(&mut document, "   arguments", &join(&node.argument));
			}
			for (key, value) in [
				("programs", join(&graph.programs)), ("norm_mean", join(&stored.norm_mean)),
				("norm_scale", join(&stored.norm_scale)), ("weights", join(&graph.parameters)),
				("frozen", join(&graph.frozen)), ("moments", join(&graph.state.moments)),
				("variances", join(&graph.state.variances)), ("best", join(&graph.state.best)),
				("best_loss", join(&graph.state.best_loss)), ("epoch", graph.state.epoch.to_string()),
				("training_rows", graph.state.training_rows.to_string()),
				("trained_samples", join(&graph.state.trained_samples)),
			] { field(&mut document, key, &value) }
			if stored.target_span != 0.0 {
				field(&mut document, "target_min", &stored.target_min.to_string());
				field(&mut document, "target_span", &stored.target_span.to_string());
			}
			if !stored.bn_stats.is_empty() { field(&mut document, "bn_stats", &join(&stored.bn_stats)) }
		}
		fs::write(path, document).map_err(|error| RecipeError::new(format!("cannot write {path}: {error}")))?;
		eprintln!("saved: {path}");
		Ok(())
	}
	fn same_node(a: &Node, b: &Node) -> bool {
		a.op == b.op
			&& a.source == b.source
			&& a.second == b.second
			&& a.input == b.input
			&& a.output == b.output
			&& a.offset == b.offset
			&& a.parameters == b.parameters
			&& a.argument.map(f64::to_bits) == b.argument.map(f64::to_bits)
			&& a.program_offset == b.program_offset
			&& a.program_count == b.program_count
	}
	fn same_graph(a: &StoredGraph, b: &StoredGraph) -> bool {
		a.inputs == b.inputs
			&& a.outputs == b.outputs
			&& a.graph.input == b.graph.input
			&& a.graph.output == b.graph.output
			&& a.graph.frozen == b.graph.frozen
			&& a.graph.programs == b.graph.programs
			&& a.graph.parameters.len() == b.graph.parameters.len()
			&& a.graph.nodes.len() == b.graph.nodes.len()
			&& a.graph.nodes.iter().zip(&b.graph.nodes).all(|(a, b)| same_node(a, b))
	}
	pub(super) fn restore(path: &str, schema: &str, graphs: &mut [StoredGraph], identities: &[u64]) -> Result<()> {
		if !fs::exists(path).map_err(|error| RecipeError::new(format!("cannot inspect {path}: {error}")))? {
			return save(path, schema, graphs);
		}
		let (stored_schema, stored) = load(path)?;
		let matches = stored_schema == schema
			&& stored.len() == graphs.len()
			&& stored.iter().zip(graphs.iter()).all(|(a, b)| same_graph(a, b));
		if matches {
			for (current, saved) in graphs.iter_mut().zip(&stored) {
				let saved_boundary = saved.graph.state.training_rows;
				let current_boundary = current.graph.state.training_rows;
				if saved_boundary != 0 {
					require(!saved.graph.state.trained_samples.is_empty(),
						"resume rejected: saved model has no training membership identity")?;
					require(current_boundary <= identities.len(), "current training membership is incomplete")?;
					let trained = saved.graph.state.trained_samples.iter().copied().collect::<BTreeSet<_>>();
					let overlap = identities[current_boundary..].iter().filter(|value| trained.contains(value)).count();
					require(overlap == 0, format!("resume rejected: {overlap} evaluation samples were previously trained, current boundary is {current_boundary} and saved boundary was {saved_boundary}"))?;
				}
				let same = |a: &[f64], b: &[f64]| a.len() == b.len()
					&& a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits());
				require(same(&current.norm_mean, &saved.norm_mean) && same(&current.norm_scale, &saved.norm_scale)
					&& current.target_min.to_bits() == saved.target_min.to_bits()
					&& current.target_span.to_bits() == saved.target_span.to_bits(), format!(
					"resume rejected: fitted preprocessing differs, current boundary is {current_boundary} and saved boundary was {saved_boundary}"))?;
			}
			for (current, saved) in graphs.iter_mut().zip(stored) {
				let current_training_rows = current.graph.state.training_rows;
				current.graph.parameters = saved.graph.parameters;
				current.graph.state = saved.graph.state;
				current.graph.state.training_rows = current_training_rows;
			}
			return Ok(());
		}
		eprint!("mismatch: overwrite {path}? Y/n ");
		std::io::stderr().flush().map_err(|error| RecipeError::new(format!("cannot prompt: {error}")))?;
		let mut answer = String::new();
		std::io::stdin()
			.read_line(&mut answer)
			.map_err(|error| RecipeError::new(format!("cannot read answer: {error}")))?;
		require(
			answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y"),
			"model mismatch not overwritten",
		)?;
		save(path, schema, graphs)
	}
	pub(super) fn normalize_input(samples: &mut [f64], stored: &StoredGraph) -> Result<()> {
		if stored.norm_mean.is_empty() {
			return Ok(());
		}
		require(stored.norm_mean.len() == samples.len(), "model normalization stats have the wrong width")?;
		for (value, (mean, scale)) in samples.iter_mut().zip(stored.norm_mean.iter().zip(&stored.norm_scale)) {
			*value = (*value - mean) / scale;
		}
		Ok(())
	}
	pub(super) fn decode_output(result: &mut [f64], stored: &StoredGraph) {
		if stored.target_span > 0.0 {
			for value in result.iter_mut() {
				*value = stored.target_min + stored.target_span * logistic(*value);
			}
		}
	}
	pub(super) fn run_infer(
		path: &str,
		input: &[f64],
		forward: impl Fn(&StoredGraph, &[f64]) -> Result<Vec<f64>>,
	) -> Result<Vec<f64>> {
		let (_, graphs) = load(path)?;
		let first = graphs.first().ok_or_else(|| RecipeError::new("model has no graph"))?;
		require(input.len() == first.inputs.len(), "model input has the wrong width")?;
		let mut values = first.inputs.iter().cloned().zip(input.iter().copied()).collect::<BTreeMap<_, _>>();
		let mut result = Vec::new();
		for stored in graphs {
			let mut samples = stored
				.inputs
				.iter()
				.map(|name| {
					values.get(name).copied().ok_or_else(|| RecipeError::new(format!("input {name:?} is absent")))
				})
				.collect::<Result<Vec<_>>>()?;
			normalize_input(&mut samples, &stored)?;
			result = forward(&stored, &samples)?;
			decode_output(&mut result, &stored);
			require(result.len() == stored.outputs.len(), "model output has the wrong width")?;
			for (name, value) in stored.outputs.iter().cloned().zip(result.iter().copied()) {
				values.insert(name, value);
			}
		}
		Ok(result)
	}
}
use std::{
	collections::{BTreeMap, BTreeSet},
	error::Error,
	ffi::c_void,
	fmt, fs,
	mem::{size_of, size_of_val},
	path::{Path, PathBuf},
	ptr,
	sync::{
		Mutex, OnceLock,
		atomic::{AtomicBool, AtomicU64, Ordering},
	},
	time::Instant,
};
pub static recipe: Recipe = Recipe;
static RUN: AtomicU64 = AtomicU64::new(0);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
const SIGINT: i32 = 2;
const INTERRUPTED_EXIT: i32 = 128 + SIGINT;
static SIGNAL: OnceLock<usize> = OnceLock::new();
extern "C" fn interrupt(_: i32) {
	if !INTERRUPTED.swap(true, Ordering::AcqRel) {
		let message = b"interrupt received, finishing checkpoint\n";
		unsafe {
			write(2, message.as_ptr().cast(), message.len());
		}
	}
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeError(String);
impl RecipeError {
	fn new(message: impl Into<String>) -> Self {
		Self(message.into())
	}
}
impl fmt::Display for RecipeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(&self.0)
	}
}
impl Error for RecipeError {}
pub type Result<T> = std::result::Result<T, RecipeError>;
type Ptr = *mut c_void;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
	Amd,
	Nvidia,
}
pub struct Data {
	sources: Vec<String>,
	target: Vec<String>,
	exclusions: Vec<String>,
	routes: Vec<Route>,
	normalize: bool,
	split: f64,
	prepared: OnceLock<Result<Prepared>>,
}
#[derive(Clone)]
struct Route {
	inputs: Vec<String>,
	outputs: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Residual {
	Layer(usize),
	Activation(Activation),
}
pub const fn layer(width: usize) -> Residual {
	Residual::Layer(width)
}
type FitFn = fn(usize, &Prepared, usize, Config, bool) -> Result<Predictor>;
#[derive(Clone, Copy, Debug)]
struct Estimator {
	fit: FitFn,
	param: usize,
	name: &'static str,
}
impl PartialEq for Estimator {
	fn eq(&self, other: &Self) -> bool {
		self.param == other.param && self.name == other.name
	}
}
impl Eq for Estimator {}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Operation {
	Layer(usize),
	Conv(usize, usize),
	Pool(usize),
	Estimator(Estimator),
	Embed(usize, usize),
	Attention(usize),
	Rnn(usize),
	Gru(usize),
	Lstm(usize),
	Residual(Vec<Residual>),
	Moe(usize, Vec<Residual>),
	Svm(Vec<Activation>),
	Perceptron(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Activation {
	Linear,
	Cos,
	Exp,
	Log,
	Ln,
	Huber,
	Tan,
	Relu,
	Leak,
	Sigmoid,
	Tanh,
	Selu,
	Gelu,
	Silu,
	Elu,
	Prelu,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockNormalization {
	Batch,
	Layer,
}
macro_rules! slots { ($(fn $name:ident = $value:ident),+ $(,)?) => {$(pub const fn $name() -> Residual {
	Residual::Activation(Activation::$value) })+}; }
pub mod atv {
	use super::{Activation, Residual};
	slots! {
	fn linear = Linear, fn cos = Cos, fn exp = Exp, fn log = Log, fn ln = Ln, fn huber = Huber,
	fn tan = Tan, fn relu = Relu, fn leak = Leak, fn sigmoid = Sigmoid, fn tanh = Tanh,
	fn selu = Selu, fn gelu = Gelu, fn silu = Silu, fn elu = Elu, fn prelu = Prelu, }
}
pub use atv::{cos, elu, exp, gelu, leak, linear, ln, log, prelu, relu, selu, sigmoid, silu, tan, tanh};
#[derive(Clone, Debug, PartialEq, Eq)]
struct Block {
	operation: Operation,
	activation: Activation,
	normalization: Option<BlockNormalization>,
}
pub struct Model {
	blocks: Vec<Block>,
	loss: LossFunction,
	downstream: Option<Vec<Block>>,
}
pub trait ModelLoss {
	fn apply(self, model: &mut Model);
}
impl ModelLoss for LossFunction {
	fn apply(self, model: &mut Model) {
		model.loss = self;
		model.downstream = None;
	}
}
impl ModelLoss for &Model {
	fn apply(self, model: &mut Model) {
		model.downstream = Some(self.blocks.clone());
	}
}
macro_rules! operation_methods { ($(fn $method:ident($($argument:ident: $kind:ty),*) = $operation:expr;)+) => {
$(pub fn $method(self, $($argument: $kind),*) -> Self { self.push($operation) })+ }; }
impl Model {
	fn push(mut self, operation: Operation) -> Self {
		self.blocks.push(Block { operation, activation: Activation::Linear, normalization: None });
		self
	}
	fn activate(mut self, activation: Activation) -> Self {
		let block = self.blocks.last_mut().unwrap_or_else(|| panic!("activation requires a preceding block"));
		if block.normalization.is_some() {
			panic!("activation must precede normalization");
		}
		block.activation = activation;
		self
	}
	operation_methods! {
	fn layer(width: usize) = Operation::Layer(width);
	fn conv(filters: usize, kernel: usize) = Operation::Conv(filters, kernel);
	fn pool(size: usize) = Operation::Pool(size);
	fn kmeans(clusters: usize) = Operation::Estimator(Estimator { fit: fit_kmeans, param: clusters, name: "kmeans" });
	fn knn(neighbors: usize) = Operation::Estimator(Estimator { fit: fit_knn, param: neighbors, name: "knn" });
	fn embed(dimensions: usize, vocabulary: usize) = Operation::Embed(dimensions, vocabulary);
	fn attn(heads: usize) = Operation::Attention(heads);
	fn rnn(width: usize) = Operation::Rnn(width);
	fn gru(width: usize) = Operation::Gru(width);
	fn lstm(width: usize) = Operation::Lstm(width);
	fn perc(width: usize) = Operation::Perceptron(width); }
	pub fn residual<const N: usize>(self, parts: [Residual; N]) -> Self {
		self.push(Operation::Residual(parts.into()))
	}
	pub fn moe<const N: usize>(self, top_k: usize, experts: [Residual; N]) -> Self {
		self.push(Operation::Moe(top_k, experts.into()))
	}
	pub fn svm<const N: usize>(self, choices: [fn() -> Residual; N]) -> Self {
		let choices = choices
			.into_iter()
			.map(|choice| match choice() {
				Residual::Activation(value) => value,
				Residual::Layer(_) => panic!("SVM choices must be activations"),
			})
			.collect();
		self.push(Operation::Svm(choices))
	}
	pub fn norm(mut self, normalization: Normalization) -> Self {
		let block = self.blocks.last_mut().unwrap_or_else(|| panic!("normalization requires a preceding block"));
		block.normalization = Some(if normalization as usize == batch as usize {
			BlockNormalization::Batch
		} else {
			BlockNormalization::Layer
		});
		self
	}
	pub fn loss(mut self, loss: impl ModelLoss) -> Self {
		loss.apply(&mut self);
		self
	}
	fn description(&self, metrics: &[Metric]) -> String {
		let has = |value| metrics.iter().any(|metric| metric.0 == value);
		let (operation, activation, normalization) = (has(5), has(6), has(7));
		let output = usize::from(matches!(
			self.blocks.last(),
			Some(Block { operation: Operation::Layer(1), activation: Activation::Linear, normalization: None })
		));
		self.blocks
			.iter()
			.take(self.blocks.len() - output)
			.filter_map(|block| {
				let mut names = Vec::new();
				if operation {
					names.push(block.operation.name());
				}
				if activation && block.activation != Activation::Linear {
					names.push(block.activation.name());
				}
				if normalization && let Some(name) = block.normalization.map(BlockNormalization::name) {
					names.push(name);
				}
				(!names.is_empty()).then(|| names.join("."))
			})
			.collect::<Vec<_>>()
			.join("/")
	}
}
impl Estimator {
	const fn name(&self) -> &'static str {
		self.name
	}
}
impl Operation {
	const fn name(&self) -> &'static str {
		match self {
			Self::Layer(_) => "layer", Self::Conv(..) => "conv", Self::Pool(_) => "pool",
			Self::Estimator(value) => value.name(), Self::Embed(..) => "embed",
			Self::Attention(_) => "attn", Self::Rnn(_) => "rnn", Self::Gru(_) => "gru",
			Self::Lstm(_) => "lstm", Self::Residual(_) => "residual", Self::Moe(..) => "moe",
			Self::Svm(_) => "svm", Self::Perceptron(_) => "perc",
		}
	}
}
impl Activation {
	const fn name(self) -> &'static str {
		match self {
			Self::Linear => "linear", Self::Cos => "cos", Self::Exp => "exp", Self::Log => "log",
			Self::Ln => "ln", Self::Huber => "huber", Self::Tan => "tan", Self::Relu => "relu",
			Self::Leak => "leak", Self::Sigmoid => "sigmoid", Self::Tanh => "tanh", Self::Selu => "selu",
			Self::Gelu => "gelu", Self::Silu => "silu", Self::Elu => "elu", Self::Prelu => "prelu",
		}
	}
}
impl BlockNormalization {
	const fn name(self) -> &'static str {
		match self {
			Self::Batch => "bnorm",
			Self::Layer => "lnorm",
		}
	}
}
macro_rules! activations { ($(fn $method:ident = $activation:ident;)+) => {$(impl Model { pub fn $method(self) -> Self {
self.activate(Activation::$activation) } })+}; }
activations! {
fn cos = Cos;
fn exp = Exp;
fn log = Log;
fn ln = Ln;
fn huber = Huber;
fn tan = Tan;
fn relu = Relu;
fn leak = Leak;
fn sigmoid = Sigmoid;
fn tanh = Tanh;
fn selu = Selu;
fn gelu = Gelu;
fn silu = Silu;
fn elu = Elu;
fn prelu = Prelu; }
pub struct Recipe;
#[derive(Clone)]
pub struct DeviceInfo {
	pub name: String,
	pub host: String,
}
#[derive(Clone)]
pub struct DeviceLink {
	pub from: String,
	pub to: String,
	pub latency_ms: f64,
	pub bytes_per_second: f64,
}
#[derive(Clone)]
pub struct Topology {
	pub devices: Vec<DeviceInfo>,
	pub links: Vec<DeviceLink>,
}
pub struct Adamw;
#[derive(Clone, Copy)]
pub struct LossFunction(u8);
#[derive(Clone, Copy)]
pub struct Metric(u8);
pub struct ZScore;
pub type Normalization = fn(usize) -> Residual;
pub type Norm = Normalization;
pub type Loss = LossFunction;
pub const adamw: Adamw = Adamw;
pub const mse: LossFunction = LossFunction(0);
pub const rmse: LossFunction = LossFunction(1);
pub const huber: LossFunction = LossFunction(2);
pub const mae: LossFunction = LossFunction(3);
pub const bce: LossFunction = LossFunction(4);
pub const ce: LossFunction = LossFunction(5);
pub const focal: LossFunction = LossFunction(6);
pub const Run: Metric = Metric(0);
pub const Loss: Metric = Metric(1);
pub const R2: Metric = Metric(2);
pub const Time: Metric = Metric(3);
pub const Epoch: Metric = Metric(4);
pub const blck: Metric = Metric(5);
pub const atvn: Metric = Metric(6);
pub const norm: Metric = Metric(7);
pub const z_score: ZScore = ZScore;
pub const batch: Normalization = batch_marker;
const fn batch_marker(_: usize) -> Residual {
	Residual::Activation(Activation::Relu)
}
impl LossFunction {
	const fn name(self) -> &'static str {
		match self.0 { 0 => "mse", 1 => "rmse", 2 => "huber", 3 => "mae", 4 => "bce", 5 => "ce", 6 => "focal", _ => unreachable!() }
	}
	fn value(self, prediction: f64, target: f64, threshold: f64) -> f64 {
		let difference = prediction - target;
		let probability = logistic(prediction).clamp(f64::EPSILON, 1.0 - f64::EPSILON);
		match self.0 {
			0 | 1 => difference * difference,
			2 => {
				let absolute = difference.abs();
				if absolute <= threshold {
					0.5 * difference * difference
				} else {
					threshold * (absolute - 0.5 * threshold)
				}
			}
			3 => difference.abs(),
			4 | 5 => -target * probability.ln() - (1.0 - target) * (1.0 - probability).ln(),
			6 => {
				let correct = if target >= 0.5 { probability } else { 1.0 - probability };
				-(1.0 - correct).powi(2) * correct.ln()
			}
			_ => f64::NAN,
		}
	}
}
impl Recipe {
	pub fn data(&self, sources: impl IntoDataSources) -> Data {
		Data {
			sources: sources.into_data_sources(),
			target: Vec::new(),
			exclusions: Vec::new(),
			routes: Vec::new(),
			normalize: false,
			split: 1.0,
			prepared: OnceLock::new(),
		}
	}
	pub fn model(&self) -> Model {
		Model { blocks: Vec::new(), loss: mse, downstream: None }
	}
	pub fn devices(&self) -> Vec<String> {
		self.topology().devices.into_iter().map(|device| device.name).collect()
	}
	pub fn topology(&self) -> Topology {
		topology().unwrap_or_else(|error| panic!("{error}"))
	}
	pub const fn train(&self) -> Train {
		Train {
			epochs: 1,
			learning_rate: 0.001,
			log_metrics: Vec::new(),
			stop: None,
			resume: None,
			save: None,
			seed: None,
		}
	}
	pub fn export(&self, source: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
		let source = source.as_ref();
		require(
			source.extension().and_then(|value| value.to_str()) == Some("rs"),
			"export requires a Rust source",
		)?;
		fs::metadata(source)
			.map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", source.display())))?;
		let found = devices()?;
		let backends = [Backend::Amd, Backend::Nvidia]
			.into_iter()
			.filter(|backend| found.iter().any(|gpu| gpu.backend == *backend))
			.collect::<Vec<_>>();
		let mut outputs = Vec::new();
		for backend in backends {
			let artifacts = match backend {
				Backend::Amd => vec![
					mapped_artifacts(option_env!("RECIPE_HSA_CODE_OBJECTS"), "hsaco")?,
					mapped_artifacts(option_env!("RECIPE_HSA_ASSEMBLIES"), "amd.s")?,
				]
				.concat(),
				Backend::Nvidia => vec![(
					"ptx".to_owned(),
					option_env!("RECIPE_NV_PTX")
						.ok_or_else(|| RecipeError::new("Nvidia artifacts were not compiled"))?,
				)],
			};
			for (extension, compiled) in artifacts {
				let output = source.with_file_name(format!("recipe.{extension}"));
				fs::copy(compiled, &output).map_err(|error| {
					RecipeError::new(format!("cannot export {}: {error}", output.display()))
				})?;
				eprintln!("exported: {}", output.display());
				outputs.push(output);
			}
		}
		Ok(outputs)
	}
}
fn mapped_artifacts<'a>(mapping: Option<&'a str>, suffix: &str) -> Result<Vec<(String, &'a str)>> {
	let mapping = mapping.ok_or_else(|| RecipeError::new("AMD artifacts were not compiled"))?;
	Ok(mapping
		.split(';')
		.filter_map(|value| value.split_once('='))
		.map(|(target, path)| (format!("{target}.{suffix}"), path))
		.collect())
}
fn inject_bn_stats(graph: &Graph, bn_stats: &[f64], contexts: &[Buffer]) -> Result<()> {
	if bn_stats.is_empty() { return Ok(()) }
	let mut offset = 0;
	for (index, node) in graph.nodes.iter().enumerate() {
		if node.op == Primitive::Normalize && node.argument[0] == 0.0 {
			let channels = node.output.channels;
			let needed = 2 * channels;
			if offset + needed <= bn_stats.len() {
				contexts[index].write(0, &bn_stats[offset..offset + needed])?;
				offset += needed;
			}
		}
	}
	Ok(())
}
fn extract_bn_stats(graph: &Graph, contexts: &[Buffer]) -> Result<Vec<f64>> {
	let mut stats = Vec::new();
	for (index, node) in graph.nodes.iter().enumerate() {
		if node.op == Primitive::Normalize && node.argument[0] == 0.0 {
			let channels = node.output.channels;
			stats.extend(contexts[index].download_range::<f64>(0, 2 * channels)?);
		}
	}
	Ok(stats)
}
fn cpu_forward(graph: &Graph, samples: &[f64]) -> Result<Vec<f64>> {
	let inputs = graph.input.elements();
	require(inputs != 0 && !samples.is_empty() && samples.len() % inputs == 0, "model input batch is invalid")?;
	let rows = samples.len() / inputs;
	let program_base = checked_mul(graph.nodes.len(), 9, "node arguments")?;
	let (mut descriptors, mut arguments, mut buffers, mut contexts, mut tiles) =
		(Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
	for node in &graph.nodes {
		descriptors.extend(node_descriptor(node, program_base)?);
		arguments.extend(node.argument);
		buffers.push(vec![0.0_f64; graph_rows_buffer(node.output, rows)? / size_of::<f64>()]);
		contexts.push(vec![0.0_f64; node_context(node, rows)? / size_of::<f64>()]);
		tiles.push(Tile {
			m: narrow(node.output.length.max(1), "CPU tile M")? as u32,
			n: narrow(node.output.channels.max(1), "CPU tile N")? as u32,
			k: narrow(node.input.elements().clamp(1, TILE_CAPACITY), "CPU tile K")? as u32,
		});
	}
	arguments.extend(&graph.programs);
	let mut value_pointers = buffers.iter_mut().map(Vec::as_mut_ptr).collect::<Vec<_>>();
	let mut context_pointers = contexts.iter_mut().map(Vec::as_mut_ptr).collect::<Vec<_>>();
	let mut timings = vec![0_u64; graph.nodes.len()];
	let weights: &[f64] = if graph.parameters.is_empty() { &[0.0] } else { &graph.parameters };
	unsafe {
		recipe_forward_cpu(
			samples.as_ptr(),
			weights.as_ptr(),
			value_pointers.as_mut_ptr(),
			context_pointers.as_mut_ptr(),
			descriptors.as_ptr(),
			arguments.as_ptr(),
			narrow(rows, "CPU rows")?,
			narrow(graph.nodes.len(), "CPU nodes")?,
			1,
			1,
			1,
			1,
			timings.as_mut_ptr(),
			tiles.as_ptr(),
		);
	}
	buffers.pop().ok_or_else(|| RecipeError::new("model graph has no nodes"))
}
impl Recipe {
	pub fn infer(&self, path: impl AsRef<Path>, input: &[f64]) -> Vec<f64> {
		let path = path.as_ref().to_string_lossy();
		let result = if std::env::var_os("RECIPE_FORCE_CPU").is_none()
			&& let Ok(gpus) = devices()
			&& let Some(gpu) = gpus.first()
		{
			bundle::run_infer(&path, input, |stored, samples| {
				let mut tape = GpuTape::new(&stored.graph, samples, &[], gpu)?;
				inject_bn_stats(&stored.graph, &stored.bn_stats, &tape.contexts)?;
				tape.forward()?;
				tape.predictions()
			})
		} else {
			bundle::run_infer(&path, input, |stored, samples| cpu_forward(&stored.graph, samples))
		};
		result.unwrap_or_else(|error| panic!("{error}"))
	}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shape {
	channels: usize,
	length: usize,
}
impl Shape {
	fn elements(self) -> usize {
		self.channels * self.length
	}
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum Primitive {
	Contraction = 0,
	Pool = 2,
	Gather = 3,
	Attention = 4,
	Scan = 5,
	Elementwise = 6,
	Route = 7,
	Normalize = 8,
	#[allow(dead_code, reason = "reserved for format-independent quantized lowering")]
	Quantized = 9,
}
#[derive(Clone, Copy)]
#[repr(i32)]
enum ScalarOpcode { Add, Constant, Parameter, Subtract, Multiply, Divide, Absolute, Exp, Log, Sin = 10, Cos, Tanh, Greater }
struct ScalarProgram(Vec<f64>);
impl ScalarProgram {
	fn op(&mut self, opcode: ScalarOpcode, left: f64, right: f64) -> f64 {
		let result = (self.0.len() / 3) as f64;
		self.0.extend([opcode as i32 as f64, left, right]);
		result
	}
	fn constant(&mut self, value: f64) -> f64 { self.op(ScalarOpcode::Constant, value, 0.0) }
	fn choose(&mut self, condition: f64, yes: f64, no: f64) -> f64 {
		let one = self.constant(1.0);
		let inv = self.op(ScalarOpcode::Subtract, one, condition);
		let (a, b) = (self.op(ScalarOpcode::Multiply, condition, yes), self.op(ScalarOpcode::Multiply, inv, no));
		self.op(ScalarOpcode::Add, a, b)
	}
	fn unary(&mut self, opcode: ScalarOpcode, value: f64) -> f64 { self.op(opcode, value, 0.0) }
}
impl Node {
	fn identity(&self, index: usize) -> String {
		let prim = match self.op {
			Primitive::Contraction => "Contraction", Primitive::Pool => "Pool", Primitive::Gather => "Gather",
			Primitive::Attention => "Attention", Primitive::Scan => "Scan", Primitive::Elementwise => "Elementwise",
			Primitive::Route => "Route", Primitive::Normalize => "Normalize", Primitive::Quantized => "Quantized",
		};
		format!("block {} {}, node {} {}, input {}x{}, output {}x{}, offset={} count={}",
			self.block_index, self.block_kind, index, prim,
			self.input.channels, self.input.length, self.output.channels, self.output.length,
			self.offset, self.parameters)
	}
}
#[derive(Clone)]
struct Node {
	op: Primitive,
	source: i32,
	second: i32,
	input: Shape,
	output: Shape,
	offset: usize,
	parameters: usize,
	argument: [f64; 9],
	program_offset: usize,
	program_count: usize,
	block_index: usize,
	block_kind: &'static str,
}
impl Node {
	fn tile_dimensions(&self) -> Option<[f64; 3]> {
		match self.op {
			Primitive::Contraction => Some([
				self.output.length as f64,
				self.output.channels as f64,
				self.input.channels as f64 * self.argument[0].max(1.0),
			]),
			Primitive::Quantized => Some([
				self.output.length as f64,
				self.output.channels as f64,
				self.input.elements() as f64,
			]),
			Primitive::Attention => {
				Some([self.output.length as f64, self.output.channels as f64, self.input.channels as f64])
			}
			_ => None,
		}
	}
}
#[derive(Clone, Default)]
struct TrainingState {
	moments: Vec<f64>,
	variances: Vec<f64>,
	best: Vec<f64>,
	best_loss: Vec<f64>,
	trained_samples: Vec<u64>,
	epoch: usize,
	training_rows: usize,
}
#[derive(Clone)]
struct Graph {
	nodes: Vec<Node>,
	parameters: Vec<f64>,
	frozen: Vec<u8>,
	programs: Vec<f64>,
	input: Shape,
	output: Shape,
	source: i32,
	state: TrainingState,
	block_index: usize,
	block_kind: &'static str,
}
impl Graph {
	fn new(shape: Shape) -> Self {
		Self {
			nodes: Vec::new(), parameters: Vec::new(), frozen: Vec::new(), programs: Vec::new(),
			input: shape, output: shape, source: -1, state: TrainingState::default(),
			block_index: 0, block_kind: "",
		}
	}
}
fn compile(model: &Model, data: &Prepared, rows: usize, gpu: &'static Gpu, config: Config) -> Result<Graph> {
	compile_graph(model, data, rows, gpu, config, None)
}
fn compile_output(
	model: &Model,
	data: &Prepared,
	rows: usize,
	gpu: &'static Gpu,
	config: Config,
	output: usize,
) -> Result<Graph> {
	compile_graph(model, data, rows, gpu, config, Some(output))
}
fn compile_graph(
	model: &Model,
	data: &Prepared,
	rows: usize,
	gpu: &'static Gpu,
	config: Config,
	expected: Option<usize>,
) -> Result<Graph> {
	require(!model.blocks.is_empty(), "model must contain a block")?;
	let sequential =
		matches!(model.blocks[0].operation, Operation::Conv(..) | Operation::Pool(..) | Operation::Embed(..));
	let shape = if sequential {
		data.sequence.unwrap_or(Shape { channels: 1, length: data.features })
	} else {
		Shape { channels: data.features, length: 1 }
	};
	let mut graph = Graph::new(shape);
	for (index, block) in model.blocks.iter().enumerate() {
		graph.block_index = index;
		graph.block_kind = block.operation.name();
		lower_block(&mut graph, block, data, rows, gpu, config)?;
	}
	if let Some(expected) = expected {
		require(graph.output.elements() == expected, "model output width does not match .out()")?;
	} else if graph.output.elements() != 1 {
		let length = graph.output.length;
		lower_conv(&mut graph, 1, length)?;
	}
	initialize_graph(&mut graph, config);
	if expected.is_none() {
		if let Some(offset) = output_bias_offset(&graph) {
			let mean = data.targets[..rows].iter().sum::<f64>() / rows as f64;
			graph.parameters[offset] = mean;
		}
	}
	Ok(graph)
}
#[derive(Clone, Copy)]
struct Field {
	source: i32,
	stride: usize,
	index: usize,
}
fn field(fields: &[(String, Field)], name: &str) -> Result<Field> {
	fields.iter()
		.find(|value| value.0 == name)
		.map(|value| value.1)
		.ok_or_else(|| RecipeError::new(format!("RAT value {name:?} is not yet available")))
}
fn route_fields(graph: &mut Graph, fields: &[Field]) -> Result<()> {
	let offset = graph.programs.len();
	for value in fields {
		graph.programs.extend([f64::from(value.source), value.stride as f64, value.index as f64]);
	}
	let output = Shape { channels: fields.len(), length: 1 };
	push_node(graph, Primitive::Route, output, 0, [0.0; 9], -2)?;
	let node = graph.nodes.last_mut().ok_or_else(|| RecipeError::new("route node is absent"))?;
	node.program_offset = offset;
	node.program_count = fields.len();
	Ok(())
}
fn route_graph(graph: &mut Graph, names: &[String], fields: &[(String, Field)], normalized: bool) -> Result<()> {
	let selected = names.iter().map(|name| field(fields, name)).collect::<Result<Vec<_>>>()?;
	route_fields(graph, &selected)?;
	if normalized {
		let epsilon = number("normalization epsilon", env!("RECIPE_NORMALIZATION_EPSILON"))?;
		let output = graph.output;
		push_node(graph, Primitive::Normalize, output, 0, arguments(0.0, epsilon), -2)?;
	}
	Ok(())
}
fn append_graph(graph: &mut Graph, mut part: Graph) -> Result<i32> {
	let source = graph.source;
	let (node_base, weight_base) = (narrow(graph.nodes.len(), "RAT graph nodes")?, graph.parameters.len());
	let program_base = graph.programs.len();
	for node in &mut part.nodes {
		node.source = if node.source < 0 { source } else { node.source + node_base };
		if node.second >= 0 {
			node.second += node_base
		}
		node.offset = checked_add(node.offset, weight_base, "RAT weight offset")?;
		if node.program_count != 0 {
			node.program_offset = checked_add(node.program_offset, program_base, "RAT program offset")?;
		}
	}
	graph.parameters.extend(part.parameters);
	graph.frozen.extend(part.frozen);
	graph.programs.extend(part.programs);
	graph.nodes.extend(part.nodes);
	graph.output = part.output;
	graph.source = narrow(graph.nodes.len(), "RAT graph nodes")? - 1;
	Ok(graph.source)
}
fn lower_block(
	graph: &mut Graph,
	block: &Block,
	data: &Prepared,
	rows: usize,
	gpu: &'static Gpu,
	config: Config,
) -> Result<()> {
	let skip = graph.source;
	match &block.operation {
		Operation::Layer(width) | Operation::Perceptron(width) => lower_project(graph, *width)?,
		Operation::Conv(f, k) => lower_conv(graph, *f, *k)?,
		Operation::Pool(size) => lower_pool(graph, *size)?,
		Operation::Embed(dimensions, vocabulary) => lower_gather(graph, *dimensions, *vocabulary)?,
		Operation::Attention(heads) => lower_attention(graph, *heads)?,
		Operation::Rnn(width) => lower_scan(graph, *width, 1)?,
		Operation::Gru(width) => lower_scan(graph, *width, 3)?,
		Operation::Lstm(width) => lower_scan(graph, *width, 4)?,
		Operation::Residual(parts) => lower_residual(graph, parts, skip, config)?,
		Operation::Moe(top_k, experts) => lower_moe(graph, *top_k, experts, config)?,
		Operation::Svm(choices) => lower_svm(graph, choices, config)?,
		Operation::Estimator(estimator) => {
			initialize_graph(graph, config);
			lower_estimator(graph, estimator, data, rows, gpu, config)?
		}
	}
	if block.activation != Activation::Linear {
		lower_activation(graph, block.activation, config)?;
	}
	if let Some(normalization) = block.normalization {
		let epsilon = number("normalization epsilon", env!("RECIPE_NORMALIZATION_EPSILON"))?;
		push_node(
			graph,
			Primitive::Normalize,
			graph.output,
			0,
			arguments(normalization as u8 as f64, epsilon),
			-2,
		)?;
	}
	let elements = checked_mul(rows, graph.output.elements(), "node batch")?;
	narrow(elements, "GPU node batch")?;
	Ok(())
}
fn push_node(
	graph: &mut Graph,
	op: Primitive,
	output: Shape,
	parameters: usize,
	argument: [f64; 9],
	second: i32,
) -> Result<()> {
	let (source, offset) = (graph.source, graph.parameters.len());
	graph.parameters.resize(checked_add(offset, parameters, "model parameters")?, 0.0);
	graph.frozen.resize(graph.parameters.len(), 0);
	graph.nodes.push(Node {
		op, source, second, input: graph.output, output, offset, parameters, argument,
		program_offset: 0, program_count: 0, block_index: graph.block_index, block_kind: graph.block_kind,
	});
	graph.output = output;
	graph.source = graph.nodes.len() as i32 - 1;
	Ok(())
}
fn push_program(graph: &mut Graph, second: i32, initial: &[f64], program: ScalarProgram) -> Result<()> {
	let (program_offset, program_count) = (graph.programs.len(), program.0.len() / 3);
	graph.programs.extend(program.0);
	let arguments = arguments(0.0, 0.0);
	push_node(graph, Primitive::Elementwise, graph.output, initial.len(), arguments, second)?;
	let node = graph.nodes.last_mut().ok_or_else(|| RecipeError::new("scalar program node is absent"))?;
	graph.parameters[node.offset..node.offset + initial.len()].copy_from_slice(initial);
	node.program_offset = program_offset;
	node.program_count = program_count;
	Ok(())
}
fn lower_activation(graph: &mut Graph, activation: Activation, config: Config) -> Result<()> {
	let (mut program, x) = (ScalarProgram(Vec::new()), -1.0);
	let (zero, one) = (program.constant(0.0), program.constant(1.0));
	let positive = program.op(ScalarOpcode::Greater, x, zero);
	let constant = |program: &mut ScalarProgram, value| program.constant(value);
	let result = match activation {
		Activation::Cos => program.unary(ScalarOpcode::Cos, x),
		Activation::Exp => program.unary(ScalarOpcode::Exp, x),
		Activation::Log | Activation::Ln => {
			let absolute = program.unary(ScalarOpcode::Absolute, x);
			let shifted = program.op(ScalarOpcode::Add, one, absolute);
			let magnitude = program.unary(ScalarOpcode::Log, shifted);
			let negative = program.op(ScalarOpcode::Subtract, zero, magnitude);
			let signed = program.choose(positive, magnitude, negative);
			if activation == Activation::Log {
				let base = constant(&mut program, std::f64::consts::LN_10);
				program.op(ScalarOpcode::Divide, signed, base)
			} else {
				signed
			}
		}
		Activation::Huber => {
			let threshold = constant(&mut program, config.activation[7]);
			let absolute = program.unary(ScalarOpcode::Absolute, x);
			let large = program.op(ScalarOpcode::Greater, absolute, threshold);
			let square = program.op(ScalarOpcode::Multiply, x, x);
			let half = constant(&mut program, 0.5);
			let small = program.op(ScalarOpcode::Multiply, half, square);
			let half_threshold = program.op(ScalarOpcode::Multiply, half, threshold);
			let excess = program.op(ScalarOpcode::Subtract, absolute, half_threshold);
			let tail = program.op(ScalarOpcode::Multiply, threshold, excess);
			program.choose(large, tail, small)
		}
		Activation::Tan => {
			let sine = program.unary(ScalarOpcode::Sin, x);
			let cosine = program.unary(ScalarOpcode::Cos, x);
			program.op(ScalarOpcode::Divide, sine, cosine)
		}
		Activation::Relu => program.op(ScalarOpcode::Multiply, positive, x),
		Activation::Leak | Activation::Elu | Activation::Selu | Activation::Prelu => {
			let negative = match activation {
				Activation::Leak => {
					let slope = constant(&mut program, config.activation[0]);
					program.op(ScalarOpcode::Multiply, slope, x)
				}
				Activation::Prelu => {
					let slope = program.op(ScalarOpcode::Parameter, 0.0, 0.0);
					program.op(ScalarOpcode::Multiply, slope, x)
				}
				_ => {
					let exponential = program.unary(ScalarOpcode::Exp, x);
					let shifted = program.op(ScalarOpcode::Subtract, exponential, one);
					let alpha = constant(
						&mut program,
						config.activation[usize::from(activation == Activation::Selu) + 2],
					);
					program.op(ScalarOpcode::Multiply, alpha, shifted)
				}
			};
			let selected = program.choose(positive, x, negative);
			if activation == Activation::Selu {
				let scale = constant(&mut program, config.activation[4]);
				program.op(ScalarOpcode::Multiply, scale, selected)
			} else {
				selected
			}
		}
		Activation::Sigmoid | Activation::Silu => {
			let half = constant(&mut program, 0.5);
			let half_x = program.op(ScalarOpcode::Multiply, half, x);
			let curved = program.unary(ScalarOpcode::Tanh, half_x);
			let shifted = program.op(ScalarOpcode::Add, curved, one);
			let sigmoid = program.op(ScalarOpcode::Multiply, half, shifted);
			if activation == Activation::Silu { program.op(ScalarOpcode::Multiply, x, sigmoid) } else { sigmoid }
		}
		Activation::Tanh => program.unary(ScalarOpcode::Tanh, x),
		Activation::Gelu => {
			let square = program.op(ScalarOpcode::Multiply, x, x);
			let cube = program.op(ScalarOpcode::Multiply, square, x);
			let cubic = constant(&mut program, config.activation[6]);
			let curved = program.op(ScalarOpcode::Multiply, cubic, cube);
			let sum = program.op(ScalarOpcode::Add, x, curved);
			let scale = constant(&mut program, config.activation[5]);
			let argument = program.op(ScalarOpcode::Multiply, scale, sum);
			let tanh = program.unary(ScalarOpcode::Tanh, argument);
			let shifted = program.op(ScalarOpcode::Add, one, tanh);
			let half = constant(&mut program, 0.5);
			let half_x = program.op(ScalarOpcode::Multiply, half, x);
			program.op(ScalarOpcode::Multiply, half_x, shifted)
		}
		Activation::Linear => unreachable!(),
	};
	let initial = if activation == Activation::Prelu { &config.activation[1..2] } else { &[] };
	debug_assert_eq!(result as usize + 1, program.0.len() / 3);
	push_program(graph, -2, initial, program)
}
fn lower_project(graph: &mut Graph, channels: usize) -> Result<()> {
	require(channels != 0, "layer width must be positive")?;
	let (parameters, output) = (
		checked_add(checked_mul(graph.output.channels, channels, "projection matrix")?, channels, "projection bias")?,
		Shape { channels, length: graph.output.length },
	);
	push_node(graph, Primitive::Contraction, output, parameters, [0.0; 9], -2)
}
fn lower_conv(graph: &mut Graph, filters: usize, kernel: usize) -> Result<()> {
	require(filters != 0 && kernel != 0, "convolution dimensions must be positive")?;
	require(kernel <= graph.output.length, "convolution kernel exceeds sequence length")?;
	let parameters = checked_add(checked_mul(filters,
		checked_mul(graph.output.channels, kernel, "convolution window")?, "conv matrix")?, filters, "conv bias")?;
	let output = Shape { channels: filters, length: graph.output.length - kernel + 1 };
	push_node(graph, Primitive::Contraction, output, parameters, arguments(kernel as f64, 0.0), -2)
}
fn output_bias_offset(graph: &Graph) -> Option<usize> {
	graph.nodes.iter().rev().find(|node| node.op == Primitive::Contraction)
		.map(|node| node.offset + node.parameters - node.output.channels)
}
fn lower_pool(graph: &mut Graph, size: usize) -> Result<()> {
	require(size != 0, "pool window must be positive")?;
	let output = Shape { channels: graph.output.channels, length: graph.output.length.div_ceil(size) };
	push_node(graph, Primitive::Pool, output, 0, arguments(size as f64, 0.0), -2)
}
fn lower_gather(graph: &mut Graph, dimensions: usize, vocabulary: usize) -> Result<()> {
	require(dimensions != 0 && vocabulary != 0, "embedding dimensions must be positive")?;
	let (parameters, output) = (
		checked_mul(dimensions, vocabulary, "embedding table")?,
		Shape { channels: dimensions, length: graph.output.elements() },
	);
	push_node(graph, Primitive::Gather, output, parameters, arguments(vocabulary as f64, 0.0), -2)
}
fn lower_attention(graph: &mut Graph, heads: usize) -> Result<()> {
	require(heads != 0 && graph.output.channels % heads == 0, "attention head partition is invalid")?;
	let (input, source) = (graph.output, graph.source);
	let mut projections = Vec::new();
	for _ in 0..3 {
		reset(graph, source, input);
		lower_project(graph, input.channels)?;
		projections.push(graph.source);
	}
	let mut fields = Vec::new();
	for source in projections {
		fields.extend((0..input.elements()).map(|index| Field { source, stride: input.elements(), index }));
	}
	route_fields(graph, &fields)?;
	let width = input.channels / heads;
	push_node(graph, Primitive::Attention, input, 0,
		[heads as f64, heads as f64, width as f64, 0.0, 0.0, (width as f64).sqrt(), 0.0, 0.0, 0.0], -2)?;
	lower_project(graph, input.channels)
}
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
fn activation(graph: &mut Graph, source: i32, shape: Shape, value: Activation, config: Config) -> Result<(i32, Shape)> {
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
	let mut weighted = Vec::with_capacity(scores.len());
	for (index, &score) in scores.iter().enumerate() {
		let centered = binary(graph, score, maximum_score, shape, ScalarOpcode::Subtract)?;
		let exponential = activation(graph, centered, shape, Activation::Exp, config)?.0;
		let mask = rank_mask(graph, scores, index, shape, top_k)?;
		weighted.push(binary(graph, mask, exponential, shape, ScalarOpcode::Multiply)?);
	}
	let mut denominator = weighted[0];
	for &value in &weighted[1..] {
		denominator = binary(graph, denominator, value, shape, ScalarOpcode::Add)?;
	}
	let mut output = None;
	for (index, &branch) in branches.iter().enumerate() {
		let probability = binary(graph, weighted[index], denominator, shape, ScalarOpcode::Divide)?;
		let routed = binary(graph, probability, branch, shape, ScalarOpcode::Multiply)?;
		output = Some(match output {
			Some(previous) => binary(graph, previous, routed, shape, ScalarOpcode::Add)?,
			None => routed,
		});
	}
	reset(graph, output.ok_or_else(|| RecipeError::new("selection has no output"))?, shape);
	Ok(())
}
fn lower_moe(graph: &mut Graph, top_k: usize, experts: &[Residual], config: Config) -> Result<()> {
	require(!experts.is_empty(), "moe requires an expert")?;
	require(top_k != 0 && top_k <= experts.len(), "moe top-k is invalid")?;
	let (source, input, mut branches) = (graph.source, graph.output, Vec::with_capacity(experts.len()));
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
fn lower_svm(graph: &mut Graph, choices: &[Activation], config: Config) -> Result<()> {
	let choices = if choices.is_empty() { &ACTIVATIONS } else { choices };
	let (source, shape, mut branches) = (graph.source, graph.output, Vec::with_capacity(choices.len()));
	let mut scores = Vec::with_capacity(choices.len());
	for &choice in choices {
		branches.push(activation(graph, source, shape, choice, config)?.0);
		scores.push(parameter(graph, source, shape)?);
	}
	select(graph, &branches, &scores, shape, 1, config)
}
fn lower_scan(graph: &mut Graph, channels: usize, gates: usize) -> Result<()> {
	require(channels != 0, "recurrent width must be positive")?;
	let (input, state) = (
		checked_mul(graph.output.channels, channels, "scan input matrix")?,
		checked_mul(channels, channels, "scan state matrix")?,
	);
	let stride = checked_add(checked_add(input, state, "scan gate")?, channels, "scan bias")?;
	let output = Shape { channels, length: graph.output.length };
	push_node(
		graph,
		Primitive::Scan,
		output,
		checked_mul(gates, stride, "scan parameters")?,
		arguments(gates as f64, 0.0),
		-2,
	)
}
fn lower_residual(graph: &mut Graph, parts: &[Residual], skip: i32, config: Config) -> Result<()> {
	let shape = graph.output;
	require(!parts.is_empty(), "residual branch must contain an operation")?;
	for part in parts {
		match part {
			Residual::Layer(width) => lower_project(graph, *width)?,
			Residual::Activation(activation) => lower_activation(graph, *activation, config)?,
		}
	}
	require(
		graph.output.channels == shape.channels && graph.output.length == shape.length,
		"residual shape mismatch",
	)?;
	let mut program = ScalarProgram(Vec::new());
	program.op(ScalarOpcode::Add, -1.0, -2.0);
	push_program(graph, skip, &[], program)
}
fn lower_estimator(
	graph: &mut Graph,
	estimator: &Estimator,
	data: &Prepared,
	rows: usize,
	gpu: &'static Gpu,
	config: Config,
) -> Result<()> {
	let input = graph.output;
	let inputs = graph_inputs(graph, &data.samples, &data.targets, rows, gpu)?;
	let mut samples = inputs.clone();
	samples.extend_from_slice(&inputs);
	let mut targets = data.targets[..rows].to_vec();
	targets.extend_from_within(..);
	let paired = Prepared {
		samples,
		targets,
		rows: checked_mul(rows, 2, "paired estimator rows")?,
		features: input.elements(),
		schema: String::new(),
		sequence: None,
		norm_mean: Vec::new(),
		norm_scale: Vec::new(),
		identities: Vec::new(),
	};
	let teacher = {
		require(paired.rows > rows, "estimator split must retain test rows")?;
		let predict = estimator.fit(&paired, rows, config, true)?;
		paired.samples[rows * input.elements()..]
			.chunks_exact(input.elements())
			.enumerate()
			.map(|(row, query)| predict(row, query))
			.collect::<Vec<_>>()
	};
	append_graph(graph, fit_surrogate(input, &inputs, &teacher, config.surrogate_width, gpu, config)?).map(drop)
}
fn initialize_graph(graph: &mut Graph, config: Config) {
	let mut state = config.random_seed as u64;
	for node in &graph.nodes {
		if node.op == Primitive::Elementwise {
			continue;
		}
		let fan_in = (node.parameters / node.output.channels.max(1)).max(1) as f64;
		let scale = config.initial / fan_in.sqrt();
		for index in node.offset..node.offset + node.parameters {
			if graph.frozen[index] == 0 {
				state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
				graph.parameters[index] = ((state >> 11) as f64 / ((1_u64 << 53) as f64) * 2.0 - 1.0) * scale;
			}
		}
		if node.op == Primitive::Contraction {
			graph.parameters[node.offset + node.parameters - node.output.channels..node.offset + node.parameters].fill(0.0);
		}
		if node.op == Primitive::Scan {
			let channels = node.output.channels;
			let input_matrix = node.input.channels * channels;
			let state_matrix = channels * channels;
			let stride = input_matrix + state_matrix + channels;
			for gate in 0..node.argument[0] as usize {
				graph.parameters[node.offset + gate * stride + input_matrix + state_matrix..
					node.offset + (gate + 1) * stride].fill(0.0);
			}
			if node.argument[0] as usize == 4 {
				graph.parameters[node.offset + stride + input_matrix + state_matrix..
					node.offset + stride * 2].fill(1.0);
			}
		}
	}
}
fn arguments(first: f64, second: f64) -> [f64; 9] {
	[first, second, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
}
fn checked_add(left: usize, right: usize, role: &str) -> Result<usize> {
	left.checked_add(right).ok_or_else(|| RecipeError::new(format!("{role} overflows")))
}
fn checked_mul(left: usize, right: usize, role: &str) -> Result<usize> {
	left.checked_mul(right).ok_or_else(|| RecipeError::new(format!("{role} overflows")))
}
fn require(condition: bool, message: impl Into<String>) -> Result<()> {
	condition.then_some(()).ok_or_else(|| RecipeError::new(message))
}
fn logistic(value: f64) -> f64 {
	1.0 / (1.0 + (-value).exp())
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct Tile {
	m: u32,
	n: u32,
	k: u32,
}
impl Tile {
	fn proposed(values: &mut [f64], minimum: Self, maximum: Self) -> Result<Self> {
		require(values.len() >= 3, "RAT requires M, N, and K proposal outputs")?;
		let dimension = |value: f64, minimum: u32, maximum: u32| -> Result<u32> {
			require((0.0..=1.0).contains(&value), "RAT tile proposal must be normalized")?;
			require(minimum <= maximum, "RAT tile range is invalid")?;
			Ok(minimum + (value * f64::from(maximum - minimum)).floor() as u32)
		};
		let tile = Self {
			m: dimension(values[0], minimum.m, maximum.m)?,
			n: dimension(values[1], minimum.n, maximum.n)?,
			k: dimension(values[2], minimum.k, maximum.k)?,
		};
		values[..3].copy_from_slice(&[f64::from(tile.m), f64::from(tile.n), f64::from(tile.k)]);
		Ok(tile)
	}
}
#[derive(Clone, Copy)]
struct Config {
	kmeans_iterations: usize,
	surrogate_epochs: usize,
	surrogate_width: usize,
	surrogate_rate: f64,
	initial: f64,
	beta1: f64,
	beta2: f64,
	epsilon: f64,
	decay: f64,
	rat_batch: usize,
	random_seed: usize,
	activation: [f64; 8],
}
impl Config {
	fn load() -> Result<Self> {
		Ok(Self {
			kmeans_iterations: natural("kmeans iterations", env!("RECIPE_KMEANS_ITERATIONS"))?,
			surrogate_epochs: natural("surrogate epochs", env!("RECIPE_SURROGATE_EPOCHS"))?,
			surrogate_width: natural("surrogate width", env!("RECIPE_SURROGATE_WIDTH"))?,
			surrogate_rate: number("surrogate rate", env!("RECIPE_SURROGATE_RATE"))?,
			rat_batch: natural("RAT batch rows", env!("RECIPE_RAT_BATCH_ROWS"))?,
			random_seed: natural("random seed", env!("RECIPE_RANDOM_SEED"))?,
			initial: number("initial weight", env!("RECIPE_TRAIN_INITIAL_WEIGHT"))?,
			beta1: number("AdamW beta1", env!("RECIPE_ADAMW_BETA1"))?,
			beta2: number("AdamW beta2", env!("RECIPE_ADAMW_BETA2"))?,
			epsilon: number("AdamW epsilon", env!("RECIPE_ADAMW_EPSILON"))?,
			decay: number("AdamW weight decay", env!("RECIPE_ADAMW_WEIGHT_DECAY"))?,
			activation: [
				number("leak slope", env!("RECIPE_LEAK_SLOPE"))?,
				number("PReLU slope", env!("RECIPE_PRELU_SLOPE"))?,
				number("ELU alpha", env!("RECIPE_ELU_ALPHA"))?,
				number("SELU alpha", env!("RECIPE_SELU_ALPHA"))?,
				number("SELU scale", env!("RECIPE_SELU_SCALE"))?,
				number("GELU scale", env!("RECIPE_GELU_SCALE"))?,
				number("GELU cubic", env!("RECIPE_GELU_CUBIC"))?,
				number("Huber threshold", env!("RECIPE_HUBER_THRESHOLD"))?,
			],
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
	require(value != 0, format!("{name} must be positive")).map(|_| value)
}
fn multi_device() -> bool {
	env!("RECIPE_MULTI_DEVICE") == "true"
}
fn stored_graph(graph: &Graph, data: &Data, scale: Option<TargetScale>) -> bundle::StoredGraph {
	let inputs = (0..graph.input.elements()).map(|index| format!("input{index}")).collect();
	let output = data.target.first().cloned().unwrap_or_else(|| "target".to_owned());
	let (norm_mean, norm_scale) = match data.prepared.get() {
		Some(Ok(prepared)) => (prepared.norm_mean.clone(), prepared.norm_scale.clone()),
		_ => (Vec::new(), Vec::new()),
	};
	let (target_min, target_span) = scale.map_or((0.0, 0.0), |s| (s.minimum, s.span));
	bundle::StoredGraph {
		graph: graph.clone(), inputs, outputs: vec![output], norm_mean, norm_scale, target_min, target_span, bn_stats: Vec::new(),
	}
}
struct GpuTape {
	gpu: &'static Gpu,
	values: Vec<Buffer>,
	contexts: Vec<Buffer>,
	_adjoints: Vec<Buffer>,
	samples: Buffer,
	input_adjoint: Buffer,
	targets: Buffer,
	weights: Buffer,
	frozen: Buffer,
	best: Buffer,
	moments: Buffer,
	variances: Buffer,
	gradient: Buffer,
	metrics: Buffer,
	best_loss: Buffer,
	value_pointers: Buffer,
	context_pointers: Buffer,
	adjoint_pointers: Buffer,
	descriptors: Buffer,
	arguments: Buffer,
	timings: Buffer,
	tiles: Buffer,
	rows: u32,
	nodes: u32,
	parameters: u32,
	threads: u32,
	step: u32,
	input: usize,
	output: usize,
	capacity: usize,
	tile: Tile,
}
#[derive(Clone, Copy)]
#[repr(u32)]
enum EpochPhase {
	Full,
	Gradient,
	Optimizer,
}
macro_rules! ptrs { ($($e:expr),* $(,)?) => { [$(&mut $e as *mut _ as Ptr),*] } }
impl GpuTape {
	fn new(graph: &Graph, samples: &[f64], targets: &[f64], gpu: &'static Gpu) -> Result<Self> {
		let inputs = graph.input.elements();
		require(inputs != 0 && !samples.is_empty() && samples.len() % inputs == 0, "model input batch is invalid")?;
		let rows = samples.len() / inputs;
		let training = !targets.is_empty();
		require(
			targets.is_empty() || targets.len() == rows || targets.len() == rows * graph.output.elements(),
			"target batch is invalid",
		)?;
		let tile = gpu.prior_tile(graph)?;
		let (mut descriptors, mut arguments, mut values, mut contexts, mut adjoints) =
			(Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
		let program_base = checked_mul(graph.nodes.len(), 9, "node arguments")?;
		for (index, node) in graph.nodes.iter().enumerate() {
			descriptors.extend(node_descriptor(node, program_base)?);
			arguments.extend(node.argument);
			let elements = graph_rows_buffer(node.output, rows)?;
			let id = || node.identity(index);
			values.push(Buffer::new(gpu, elements).map_err(|e| RecipeError::new(format!("{e}, {}", id())))?);
			let adjoint_bytes = if training { elements } else { size_of::<f64>() };
			adjoints.push(Buffer::upload(gpu, &vec![0_u8; adjoint_bytes]).map_err(|e| RecipeError::new(format!("{e}, {}", id())))?);
			let context = Buffer::new(gpu, node_context(node, rows)?).map_err(|e| RecipeError::new(format!("{e}, {}", id())))?;
			if node.op == Primitive::Attention && node.argument[4] == 1.0 {
				context.write(0, &[0.0])?;
			}
			contexts.push(context);
		}
		arguments.extend(&graph.programs);
		let addresses = |buffers: &[Buffer]| buffers.iter().map(|buffer| buffer.pointer).collect::<Vec<_>>();
		let zeros = vec![0.0; if training { graph.parameters.len().max(1) } else { 1 }];
		fn resumed<'a>(saved: &'a [f64], cold: &'a [f64]) -> &'a [f64] {
			if saved.is_empty() { cold } else { saved }
		}
		let cold_loss = [f64::INFINITY, f64::NAN, f64::NAN, f64::INFINITY];
		let target_values = if targets.is_empty() { vec![0.0] } else { targets.to_vec() };
		Ok(Self {
			gpu,
			value_pointers: Buffer::upload(gpu, &addresses(&values))?,
			context_pointers: Buffer::upload(gpu, &addresses(&contexts))?,
			adjoint_pointers: Buffer::upload(gpu, &addresses(&adjoints))?,
			descriptors: Buffer::upload(gpu, &descriptors)?,
			arguments: Buffer::upload(gpu, &arguments)?,
			samples: Buffer::upload(gpu, samples)?,
			timings: Buffer::upload(gpu, &vec![0_u64; graph.nodes.len().max(1)])?,
			tiles: Buffer::upload(gpu, &vec![tile; graph.nodes.len().max(1)])?,
			input_adjoint: Buffer::upload(gpu, &vec![0.0; if training { samples.len() } else { 1 }])?,
			targets: Buffer::upload(gpu, &target_values)?,
			weights: Buffer::upload(gpu, &graph.parameters)?,
			frozen: Buffer::upload(gpu, if training && !graph.frozen.is_empty() { &graph.frozen } else { &[1] })?,
			best: Buffer::upload(gpu, if training { resumed(&graph.state.best, &graph.parameters) } else { &zeros })?,
			moments: Buffer::upload(gpu, if training { resumed(&graph.state.moments, &zeros) } else { &zeros })?,
			variances: Buffer::upload(gpu, if training { resumed(&graph.state.variances, &zeros) } else { &zeros })?,
			gradient: Buffer::upload(gpu, &zeros)?,
			metrics: Buffer::upload(gpu, &[0.0, 0.0, 0.0])?,
			best_loss: Buffer::upload(gpu, if training { resumed(&graph.state.best_loss, &cold_loss) } else { &zeros })?,
			rows: narrow(rows, "GPU rows")? as u32,
			nodes: narrow(graph.nodes.len(), "GPU nodes")? as u32,
			parameters: narrow(graph.parameters.len(), "GPU parameters")? as u32,
			threads: 0,
			step: narrow(graph.state.epoch, "optimizer epoch")? as u32,
			input: graph.input.elements(),
			output: graph.output.elements(),
			values,
			contexts: contexts,
			_adjoints: adjoints,
			capacity: rows,
			tile,
		})
	}
	fn forward(&mut self) -> Result<()> {
		let d = self.gpu.forward;
		self.threads = d.geometry.threads(u32::MAX)?;
		let mut a = self.forward_arguments();
		self.gpu.launch(d, &mut a, self.threads, self.tile)
	}
	fn forward_arguments(&mut self) -> [*mut c_void; 14] {
		ptrs![self.samples.pointer, self.weights.pointer, self.value_pointers.pointer,
			self.context_pointers.pointer, self.descriptors.pointer, self.arguments.pointer,
			self.rows, self.nodes, self.threads, self.tile.m, self.tile.n, self.tile.k,
			self.timings.pointer, self.tiles.pointer]
	}
	fn timings(&self) -> Result<Vec<u64>> { self.timings.download(self.nodes as usize) }
	fn predictions(&self) -> Result<Vec<f64>> {
		self.values.last().ok_or_else(|| RecipeError::new("GPU tape is empty"))?.download(self.rows as usize * self.output)
	}
	fn launch_epoch(
		&mut self,
		rate: f64,
		loss: LossFunction,
		tolerance: f64,
		config: Config,
		direct: bool,
		phase: EpochPhase,
	) -> Result<()> {
		let mut loss = if direct { 7 } else { loss.0 as u32 };
		let mut huber_threshold = config.activation[7];
		require(self.step != 0, "optimizer epoch is absent")?;
		let (mut step, mut rate, mut beta1, mut beta2, mut epsilon, mut decay, mut tolerance) =
			(self.step, rate, config.beta1, config.beta2, config.epsilon, config.decay, tolerance);
		let (mut beta1_power, mut beta2_power) = (
			config.beta1.powi(self.step as i32), config.beta2.powi(self.step as i32),
		);
		let mut phase = phase as u32;
		let mut call = ptrs![
			self.samples.pointer, self.input_adjoint.pointer, self.targets.pointer,
			self.weights.pointer, self.frozen.pointer, self.best.pointer,
			self.value_pointers.pointer, self.context_pointers.pointer, self.adjoint_pointers.pointer,
			self.descriptors.pointer, self.arguments.pointer, self.metrics.pointer,
			self.gradient.pointer, self.moments.pointer, self.variances.pointer, self.best_loss.pointer,
			self.rows, self.nodes, self.parameters, loss, huber_threshold,
			rate, beta1, beta2, beta1_power, beta2_power, epsilon, decay, tolerance, step,
			self.threads, self.tile.m, self.tile.n, self.tile.k, phase,
			self.timings.pointer, self.tiles.pointer
		];
		let dispatch = self.gpu.epoch;
		self.threads = dispatch.geometry.threads(self.rows)?;
		self.gpu.launch(dispatch, &mut call, self.threads, self.tile)
	}
	fn epoch(
		&mut self,
		rate: f64,
		loss: LossFunction,
		tolerance: f64,
		config: Config,
		direct: bool,
	) -> Result<(f64, bool)> {
		self.launch_epoch(rate, loss, tolerance, config, direct, EpochPhase::Full)?;
		let metrics = self.metrics.download::<f64>(3)?;
		Ok((metrics[0], metrics[1] != 0.0))
	}
	fn gradient(&mut self, rate: f64, loss: LossFunction, tolerance: f64, config: Config, direct: bool) -> Result<()> {
		self.launch_epoch(rate, loss, tolerance, config, direct, EpochPhase::Gradient)
	}
	fn update(&mut self, rate: f64, loss: LossFunction, tolerance: f64, config: Config, direct: bool) -> Result<()> {
		self.launch_epoch(rate, loss, tolerance, config, direct, EpochPhase::Optimizer)
	}
	fn advance(&mut self) -> Result<()> { self.step = self.step.checked_add(1).ok_or_else(|| RecipeError::new("optimizer epoch overflows"))?; Ok(()) }
	fn write_samples(&self, values: &[f64]) -> Result<()> { require(values.len() == self.capacity * self.input, "RAT input batch has the wrong shape")?; self.samples.write(0, values) }
	fn write_targets(&self, values: &[f64]) -> Result<()> { require(values.len() == self.capacity * self.output, "RAT target batch has the wrong shape")?; self.targets.write(0, values) }
	fn trainable(&self, graph: &Graph, range: (usize, usize)) -> Result<()> {
		require(
			graph.frozen.len() == self.parameters as usize && range.1 <= graph.frozen.len(),
			"RAT model range is invalid",
		)?;
		let frozen = graph
			.frozen
			.iter()
			.enumerate()
			.map(|(index, value)| u8::from(*value != 0 || index < range.0 || index >= range.1))
			.collect::<Vec<_>>();
		self.frozen.write(0, &frozen)
	}
	fn node_values(&self, node: usize, width: usize) -> Result<Vec<f64>> {
		require(node < self.values.len(), "RAT proposal node is absent")?;
		self.values[node].download(self.capacity * width)
	}
	fn weights(&self) -> Result<Vec<f64>> { self.weights.download(self.parameters as usize) }
	fn capture(&self, graph: &mut Graph) -> Result<()> {
		graph.parameters = self.weights()?;
		graph.state.moments = self.moments.download(self.parameters as usize)?;
		graph.state.variances = self.variances.download(self.parameters as usize)?;
		graph.state.epoch = self.step as usize;
		Ok(())
	}
	fn gradient_values(&self) -> Result<Vec<f64>> { self.gradient.download(self.parameters as usize) }
	fn write_gradient(&self, values: &[f64]) -> Result<()> { self.gradient.write(0, values) }
	fn optimizer_state(&self) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
		Ok((self.weights()?, self.moments.download(self.parameters as usize)?,
			self.variances.download(self.parameters as usize)?))
	}
	fn write_tiles(&mut self, values: &[Tile]) -> Result<()> {
		require(values.len() == self.nodes as usize, "GPU tile count does not match graph nodes")?;
		self.tile = values.iter().fold(Tile { m: 1, n: 1, k: 1 }, |prior, tile| Tile {
			m: prior.m.max(tile.m),
			n: prior.n.max(tile.n),
			k: prior.k.max(tile.k),
		});
		self.tiles.write(0, values)
	}
	fn fill_tile(&mut self, tile: Tile) -> Result<()> {
		self.write_tiles(&std::iter::repeat_n(tile, self.nodes as usize).collect::<Vec<_>>())
	}
	fn write_weights(&self, values: &[f64]) -> Result<()> {
		self.weights.write(0, values)
	}
}
fn tape_bytes(graph: &Graph, rows: usize, targets: bool) -> Result<usize> {
	let mut bytes = 0_usize;
	let mut add = |value| -> Result<()> {
		bytes = checked_add(bytes, value, "GPU tape bytes")?;
		Ok(())
	};
	for node in &graph.nodes {
		let values = graph_rows_buffer(node.output, rows)?;
		add(if targets { checked_mul(2, values, "GPU values and adjoints")? } else { values })?;
		add(node_context(node, rows)?)?;
	}
	let pointers = checked_mul(3 * graph.nodes.len(), size_of::<u64>(), "GPU tape pointers")?;
	add(pointers)?;
	add(checked_mul(graph.nodes.len() * 11, size_of::<i32>(), "GPU descriptors")?)?;
	add(checked_mul(graph.nodes.len() * 9 + graph.programs.len(), size_of::<f64>(), "GPU arguments")?)?;
	let samples = graph_rows_buffer(graph.input, rows)?;
	add(if targets { checked_mul(2, samples, "GPU samples and input adjoints")? } else { samples })?;
	add(checked_mul(graph.nodes.len().max(1), size_of::<u64>() + size_of::<Tile>(), "GPU node metadata")?)?;
	add(if targets { graph_rows_buffer(graph.output, rows)? } else { size_of::<f64>() })?;
	let parameters = checked_mul(graph.parameters.len().max(1), size_of::<f64>(), "GPU parameter bytes")?;
	add(checked_mul(if targets { 5 } else { 1 }, parameters, "GPU parameter bytes")?)?;
	add(if targets { graph.frozen.len().max(1) } else { 1 })?;
	add(7 * size_of::<f64>())?;
	Ok(bytes)
}
fn tape_row_limit(graph: &Graph, memory: u64, rows: usize, targets: bool) -> Result<usize> {
	let memory = usize::try_from(memory).unwrap_or(usize::MAX);
	let (mut low, mut high) = (0, rows);
	while low < high {
		let middle = low + (high - low).div_ceil(2);
		if tape_bytes(graph, middle, targets)? <= memory {
			low = middle;
		} else {
			high = middle - 1;
		}
	}
	Ok(low)
}
struct Placement {
	tape: GpuTape,
	rows: Vec<usize>,
	share: f64,
}
struct DeviceTape {
	shards: Vec<Placement>,
	targets: Vec<f64>,
	best: Vec<f64>,
	best_moments: Vec<f64>,
	best_variances: Vec<f64>,
	best_epoch: u32,
	best_loss: [f64; 4],
	replicated: bool,
	input: usize,
	output: usize,
	capacity: usize,
	step: u32,
}
impl DeviceTape {
	fn new(graph: &Graph, samples: &[f64], targets: &[f64], gpus: &[&'static Gpu]) -> Result<Self> {
		require(!gpus.is_empty(), "execution requires a GPU")?;
		let input = graph.input.elements();
		require(input != 0 && !samples.is_empty() && samples.len() % input == 0, "model input batch is invalid")?;
		let (capacity, output) = (samples.len() / input, graph.output.elements());
		require(targets.is_empty() || targets.len() == capacity * output, "target batch is invalid")?;
		let limits = gpus
			.iter()
			.map(|gpu| tape_row_limit(graph, gpu.memory, capacity, !targets.is_empty()))
			.collect::<Result<Vec<_>>>()?;
		for (gpu, rows) in gpus.iter().zip(&limits) {
			if *rows == 0 {
				eprintln!("excluded: {} (insufficient memory)", gpu.name);
			}
		}
		let viable = gpus.iter().zip(limits).filter(|(_, rows)| *rows != 0).collect::<Vec<_>>();
		require(!viable.is_empty(), "no GPU can fit one training row")?;
		let replicated = capacity < viable.len();
		let mut rows = vec![Vec::new(); viable.len()];
		if replicated {
			for values in &mut rows {
				values.extend(0..capacity);
			}
		} else {
			let total = viable.iter().map(|(_, limit)| *limit).sum::<usize>();
			require(total >= capacity, "the available GPU memory cannot fit the training batch")?;
			for row in 0..capacity {
				let index = viable
					.iter()
					.enumerate()
					.filter(|(index, (_, limit))| rows[*index].len() < *limit)
					.max_by_key(|(index, (_, limit))| {
						limit.saturating_mul(row + 1)
							.saturating_sub(rows[*index].len().saturating_mul(total))
					})
					.map(|(index, _)| index)
					.ok_or_else(|| RecipeError::new("GPU row placement is exhausted"))?;
				rows[index].push(row);
			}
		}
		let viable_count = viable.len();
		let mut shards = Vec::new();
		for ((&gpu, _), rows) in viable.into_iter().zip(rows) {
			let share = if replicated { 1.0 / viable_count as f64 } else { rows.len() as f64 / capacity as f64 };
			let packed_targets = if targets.is_empty() { Vec::new() } else { Self::pack(targets, output, &rows) };
			let tape = GpuTape::new(graph, &Self::pack(samples, input, &rows), &packed_targets, gpu)?;
			shards.push(Placement { tape, rows, share });
		}
		let best = if graph.state.best.is_empty() { graph.parameters.clone() } else { graph.state.best.clone() };
		require(best.len() == graph.parameters.len(), "saved best model has the wrong shape")?;
		let best_loss = if graph.state.best_loss.is_empty() {
			[f64::INFINITY, f64::NAN, f64::NAN, f64::INFINITY]
		} else {
			graph.state
				.best_loss
				.as_slice()
				.try_into()
				.map_err(|_| RecipeError::new("saved loss state is invalid"))?
		};
		let step = narrow(graph.state.epoch, "optimizer epoch")? as u32;
		let saved = |values: &Vec<f64>| if values.is_empty() { [0.0].repeat(best.len()) } else { values.clone() };
		let (moments, variances) = (saved(&graph.state.moments), saved(&graph.state.variances));
		Ok(Self {
			shards,
			targets: targets.to_vec(),
			best,
			best_moments: moments,
			best_variances: variances,
			best_epoch: step,
			best_loss,
			replicated,
			input,
			output,
			capacity,
			step,
		})
	}
	fn pack(values: &[f64], width: usize, rows: &[usize]) -> Vec<f64> {
		let mut packed = Vec::with_capacity(rows.len() * width);
		for &row in rows {
			packed.extend_from_slice(&values[row * width..(row + 1) * width]);
		}
		packed
	}
	fn map<T: Send, F: Fn(&mut GpuTape) -> Result<T> + Sync>(&mut self, action: F) -> Result<Vec<T>> {
		std::thread::scope(|scope| {
			let workers = self
				.shards
				.iter_mut()
				.map(|shard| {
					let action = &action;
					scope.spawn(move || action(&mut shard.tape))
				})
				.collect::<Vec<_>>();
			workers
				.into_iter()
				.map(|worker| worker.join().map_err(|_| RecipeError::new("GPU dispatch panicked"))?)
				.collect::<Result<Vec<_>>>()
		})
	}
	fn forward(&mut self) -> Result<()> { self.map(GpuTape::forward).map(|_| ()) }
	fn collect(&self, width: usize, values: impl Fn(&GpuTape) -> Result<Vec<f64>>) -> Result<Vec<f64>> {
		if self.replicated {
			return values(&self.shards[0].tape);
		}
		let mut result = vec![0.0; self.capacity * width];
		for shard in &self.shards {
			let local = values(&shard.tape)?;
			for (index, &row) in shard.rows.iter().enumerate() {
				result[row * width..(row + 1) * width]
					.copy_from_slice(&local[index * width..(index + 1) * width]);
			}
		}
		Ok(result)
	}
	fn predictions(&self) -> Result<Vec<f64>> { self.collect(self.output, GpuTape::predictions) }
	fn advance(&mut self) -> Result<()> { self.map(|tape| tape.advance())?; self.step = self.shards[0].tape.step; Ok(()) }
	fn observe(&mut self, loss: f64, tolerance: f64) -> Result<bool> {
		let (old_best, last, trail) = (self.best_loss[0], self.best_loss[1], self.best_loss[2]);
		let better = loss < old_best;
		if better {
			(self.best, self.best_moments, self.best_variances) = self.shards[0].tape.optimizer_state()?;
			self.best_epoch = self.shards[0].tape.step.saturating_sub(1);
		}
		let next_trail = if last.is_finite() && !trail.is_finite() && loss > last { last } else { trail };
		let trigger =
			next_trail.is_finite() && loss > last * (1.0 + tolerance) && loss < next_trail && tolerance > 0.0;
		self.best_loss[0] = if better { loss } else { old_best };
		self.best_loss[1] = loss;
		self.best_loss[2] = next_trail;
		if trigger {
			self.best_loss[3] = self.best_loss[0];
		}
		Ok(trigger)
	}
	fn epoch(
		&mut self,
		rate: f64,
		loss: LossFunction,
		tolerance: f64,
		config: Config,
		direct: bool,
	) -> Result<(f64, bool)> {
		let gradients = self.map(|tape| {
			tape.gradient(rate, loss, tolerance, config, direct)?;
			tape.gradient_values()
		})?;
		let mut gradient = vec![0.0; self.shards[0].tape.parameters as usize];
		for (shard, values) in self.shards.iter().zip(gradients) {
			for (total, value) in gradient.iter_mut().zip(values) {
				*total += value * shard.share;
			}
		}
		let objective = if direct {
			0.0
		} else {
			model_loss(&self.predictions()?, &self.targets, loss, config.activation[7])
		};
		let saved = if direct { false } else { self.observe(objective, tolerance)? };
		self.map(|tape| {
			tape.write_gradient(&gradient)?;
			tape.update(rate, loss, tolerance, config, direct)
		})?;
		Ok((objective, saved))
	}
	fn write_samples(&self, values: &[f64]) -> Result<()> {
		require(values.len() == self.capacity * self.input, "RAT input batch has the wrong shape")?;
		for shard in &self.shards { shard.tape.write_samples(&Self::pack(values, self.input, &shard.rows))? }
		Ok(())
	}
	fn write_targets(&mut self, values: &[f64]) -> Result<()> {
		require(values.len() == self.capacity * self.output, "RAT target batch has the wrong shape")?;
		self.targets.copy_from_slice(values);
		for shard in &self.shards { shard.tape.write_targets(&Self::pack(values, self.output, &shard.rows))? }
		Ok(())
	}
	fn trainable(&self, graph: &Graph, range: (usize, usize)) -> Result<()> {
		for shard in &self.shards { shard.tape.trainable(graph, range)? }
		Ok(())
	}
	fn node_values(&self, node: usize, width: usize) -> Result<Vec<f64>> { self.collect(width, |tape| tape.node_values(node, width)) }
	fn placement(&self, row: usize) -> Result<usize> { require(row < self.capacity, "GPU row is out of range")?; Ok(row % self.shards.len()) }
	fn proposal_limit(&self, row: usize, values: [f64; 3]) -> Result<Tile> { self.shards[self.placement(row)?].tape.gpu.proposal_limit(values) }
	fn set_tile(&mut self, row: usize, tile: Tile) -> Result<()> { let p = self.placement(row)?; self.shards[p].tape.fill_tile(tile) }
	fn weights(&self) -> Result<Vec<f64>> { self.shards[0].tape.weights() }
	fn restore_best(&mut self) -> Result<()> {
		for shard in &self.shards { shard.tape.write_weights(&self.best)? }
		Ok(())
	}
	fn capture(&self, graph: &mut Graph, best: bool) -> Result<()> {
		self.shards[0].tape.capture(graph)?;
		if best {
			graph.parameters = self.best.clone();
			(graph.state.moments, graph.state.variances) =
				(self.best_moments.clone(), self.best_variances.clone());
			graph.state.epoch = self.best_epoch as usize;
		}
		graph.state.best = self.best.clone();
		graph.state.best_loss = self.best_loss.to_vec();
		Ok(())
	}
	fn tile(&self) -> Tile { self.shards[0].tape.tile }
}
fn checkpoint(
	path: &str,
	schema: &str,
	stored: &mut bundle::StoredGraph,
	tape: &DeviceTape,
	best: bool,
) -> Result<()> {
	if let Ok((_, saved)) = bundle::load(path) {
		if saved.first().and_then(|g| g.graph.state.best_loss.first().copied()).is_some_and(|v| v <= tape.best_loss[0]) {
			return Ok(eprintln!("kept: {path}"));
		}
	}
	tape.capture(&mut stored.graph, best)?;
	bundle::save(path, schema, std::slice::from_ref(stored))
}
fn node_descriptor(node: &Node, program_base: usize) -> Result<[i32; 11]> {
	let program_offset = if node.program_count == 0 {
		0
	} else {
		checked_add(program_base, node.program_offset, "scalar program offset")?
	};
	Ok([
		node.op as i32,
		node.source,
		node.second,
		narrow(node.input.channels, "input channels")?,
		narrow(node.input.length, "input length")?,
		narrow(node.output.channels, "output channels")?,
		narrow(node.output.length, "output length")?,
		narrow(node.offset, "weight offset")?,
		narrow(node.parameters, "parameter count")?,
		narrow(program_offset, "program offset")?,
		narrow(node.program_count, "scalar instruction count")?,
	])
}
fn graph_rows_buffer(shape: Shape, rows: usize) -> Result<usize> {
	checked_mul(checked_mul(rows, shape.elements(), "node elements")?, size_of::<f64>(), "node bytes")
}
fn node_context(node: &Node, rows: usize) -> Result<usize> {
	if node.op == Primitive::Attention && node.argument[4] == 1.0 {
		let values = checked_mul(node.argument[3] as usize,
			2 * node.argument[1] as usize * node.argument[2] as usize, "attention cache")?;
		return checked_add(size_of::<f64>(), checked_mul(values, size_of::<u16>(), "FP16 attention cache")?,
			"attention context");
	}
	let elements = match node.op {
		Primitive::Elementwise => checked_mul(
			2 * node.program_count,
			checked_mul(rows, node.output.elements(), "program batch")?,
			"program",
		)?,
		Primitive::Scan => {
			let (state_count, gates) =
				(checked_mul(rows, node.output.elements(), "scan batch")?, node.argument[0] as usize);
			let states = checked_mul(2 * gates + 1, state_count, "scan states")?;
			let gradients = checked_mul(rows, node.parameters, "scan gradients")?;
			checked_add(states, checked_add(gradients, 2 * rows * node.output.channels, "scan scratch")?, "scan")?
		}
		Primitive::Pool => checked_mul(rows, node.output.elements(), "pool context")?,
		Primitive::Normalize => {
			let groups = node.output.channels.max(checked_mul(rows, node.output.length, "layer groups")?);
			checked_mul(4, groups, "normalization context")?
		}
		_ => 1,
	};
	checked_mul(elements.max(1), size_of::<f64>(), "context bytes")
}
fn narrow(value: usize, role: &str) -> Result<i32> {
	i32::try_from(value).map_err(|_| RecipeError::new(format!("{role} exceeds i32")))
}
struct Buffer {
	runtime: &'static Gpu,
	pointer: u64,
	bytes: usize,
}
impl Buffer {
	fn new(runtime: &'static Gpu, bytes: usize) -> Result<Self> {
		Ok(Self { runtime, pointer: runtime.allocate(bytes)?, bytes })
	}
	fn upload<T>(runtime: &'static Gpu, values: &[T]) -> Result<Self> {
		let buffer = Self::new(runtime, size_of_val(values))?;
		runtime.upload(buffer.pointer, values.as_ptr().cast(), size_of_val(values))?;
		Ok(buffer)
	}
	fn write<T>(&self, offset: usize, values: &[T]) -> Result<()> {
		let start = checked_mul(offset, size_of::<T>(), "GPU write offset")?;
		require(checked_add(start, size_of_val(values), "GPU write")? <= self.bytes, "GPU write exceeds buffer")?;
		self.runtime.upload(self.pointer + start as u64, values.as_ptr().cast(), size_of_val(values))
	}
	fn download<T: Copy + Default>(&self, count: usize) -> Result<Vec<T>> {
		self.download_range(0, count)
	}
	fn download_range<T: Copy + Default>(&self, offset: usize, count: usize) -> Result<Vec<T>> {
		let start = checked_mul(offset, size_of::<T>(), "GPU read offset")?;
		let mut values = std::iter::repeat_n(T::default(), count).collect::<Vec<_>>();
		require(checked_add(start, size_of_val(&*values), "GPU read")? <= self.bytes, "GPU read exceeds buffer")?;
		self.runtime.synchronize()?;
		self.runtime.download(values.as_mut_ptr().cast(), self.pointer + start as u64, size_of_val(&*values))?;
		Ok(values)
	}
}
impl Drop for Buffer {
	fn drop(&mut self) {
		self.runtime.free(self.pointer);
	}
}
#[derive(Clone, Copy)]
struct Kernel {
	object: u64,
	shared: u32,
	#[cfg(amd)]
	kernarg: usize,
	#[cfg(amd)]
	private: u32,
	layout: &'static [u8],
}
#[derive(Clone, Copy)]
struct Dispatch {
	kernel: Kernel,
	geometry: Geometry,
}
impl Kernel {
	const fn remote(object: u64, shared: u32, layout: &'static [u8]) -> Self {
		Self {
			object,
			shared,
			#[cfg(amd)]
			kernarg: 0,
			#[cfg(amd)]
			private: 0,
			layout,
		}
	}
}
const FORWARD_ARGS: &[u8] = b"88888844444488";
const EPOCH_ARGS: &[u8] = b"8888888888888888444488888888844444488";
#[cfg(nvidia)]
struct Cuda {
	context: Ptr,
	set: unsafe extern "C" fn(Ptr) -> i32,
	allocate: unsafe extern "C" fn(*mut u64, usize) -> i32,
	free: unsafe extern "C" fn(u64) -> i32,
	upload: unsafe extern "C" fn(u64, *const c_void, usize) -> i32,
	download: unsafe extern "C" fn(Ptr, u64, usize) -> i32,
	synchronize: unsafe extern "C" fn() -> i32,
	launch: unsafe extern "C" fn(usize, u32, u32, u32, u32, u32, u32, u32, Ptr, *mut Ptr) -> i32,
}
#[cfg(nvidia)]
impl Kernel {
	const fn cuda(object: usize, shared: u32, _layout: &'static [u8]) -> Self {
		Self {
			object: object as u64,
			shared,
			#[cfg(amd)]
			kernarg: 0,
			#[cfg(amd)]
			private: 0,
			layout: _layout,
		}
	}
}
#[cfg(amd)]
#[allow(dead_code)]
struct Hsa {
	allocate: unsafe extern "C" fn(u64, usize, u32, *mut Ptr) -> i32,
	free: unsafe extern "C" fn(Ptr) -> i32,
	allow: unsafe extern "C" fn(u32, *const u64, *const u32, *const c_void) -> i32,
	copy: unsafe extern "C" fn(Ptr, *const c_void, usize) -> i32,
	store: unsafe extern "C" fn(u64, i64),
	wait: unsafe extern "C" fn(u64, i32, i64, u64, i32) -> i64,
	write: unsafe extern "C" fn(*const HsaQueue, u64) -> u64,
	queue: Ptr,
	signal: u64,
	cpu_agent: u64,
	vram_pool: u64,
	kernarg_pool: u64,
	kernarg_size: usize,
	kernarg: Ptr,
}
struct Remote {
	io: Mutex<Worker>,
}
enum Driver {
	#[cfg(amd)]
	Hsa(Hsa),
	#[cfg(nvidia)]
	Cuda(Cuda),
	Remote(Remote),
}
#[allow(dead_code)]
struct Gpu {
	name: String,
	backend: Backend,
	driver: Driver,
	forward: Dispatch,
	epoch: Dispatch,
	memory: u64,
	clock: u64,
	shared_limit: u32,
	dispatch: Mutex<()>,
}
unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}
#[cfg(amd)]
#[repr(C)]
struct HsaQueue {
	kind: u32,
	features: u32,
	base: Ptr,
	doorbell: u64,
	size: u32,
	reserved: u32,
	id: u64,
}
#[cfg(amd)]
#[repr(C)]
struct HsaPacket {
	header: u16,
	setup: u16,
	workgroup_x: u16,
	workgroup_y: u16,
	workgroup_z: u16,
	reserved0: u16,
	grid_x: u32,
	grid_y: u32,
	grid_z: u32,
	private: u32,
	group: u32,
	object: u64,
	kernarg: Ptr,
	reserved1: u64,
	completion: u64,
}
#[cfg(nvidia)] type NvQuery = unsafe extern "C" fn(*mut i32, i32, i32) -> i32;
#[cfg(any(amd, nvidia))]
struct Library(Ptr);
#[cfg(any(amd, nvidia))]
impl Library {
	fn open(name: &str) -> Result<Self> {
		let name = format!("{name}\0");
		let handle = unsafe { dlopen(name.as_ptr().cast(), 2) };
		require(!handle.is_null(), format!("cannot load {name:?}"))?;
		Ok(Self(handle))
	}
	fn function<F: Copy>(&self, name: &[u8]) -> Result<F> {
		let pointer = unsafe { dlsym(self.0, name.as_ptr().cast()) };
		require(!pointer.is_null(), format!("runtime symbol {:?} is absent", name))?;
		Ok(unsafe { std::mem::transmute_copy(&pointer) })
	}
}
#[cfg(any(amd, nvidia))]
fn driver_status(backend: Backend, status: i32, action: &str) -> Result<()> {
	(status == 0).then_some(()).ok_or_else(|| RecipeError::new(format!("{backend:?} {action} failed: {status}")))
}
impl Gpu {
	#[cfg(any(amd, nvidia))]
	fn status(&self, status: i32, action: &str) -> Result<()> {
		driver_status(self.backend, status, action)
	}
	fn activate(&self) -> Result<()> {
		match &self.driver {
			#[cfg(nvidia)]
			Driver::Cuda(driver) => self.status(unsafe { (driver.set)(driver.context) }, "context"),
			#[cfg(amd)]
			Driver::Hsa(_) => Ok(()),
			Driver::Remote(_) => Ok(()),
		}
	}
	fn shared_values(&self) -> Result<u32> {
		let fixed = self.forward.kernel.shared.max(self.epoch.kernel.shared);
		let shared = self
			.shared_limit
			.checked_sub(fixed)
			.ok_or_else(|| RecipeError::new("GPU kernel exceeds shared memory"))?;
		let shared_values = shared / size_of::<f64>() as u32;
		require(shared_values != 0, "GPU has no shared memory for contraction")?;
		Ok(shared_values)
	}
	fn tile_limit(&self, graph: &Graph) -> Result<Tile> {
		let shared_values = self.shared_values()?;
		let m = graph.nodes.iter().map(|node| node.output.length).max().unwrap_or(graph.output.length).max(1);
		let n = graph.nodes.iter().map(|node| node.output.channels).max().unwrap_or(graph.output.channels).max(1);
		let k = graph.nodes.iter().map(|node| node.input.elements()).max().unwrap_or(graph.input.elements()).max(1);
		Ok(Tile {
			m: narrow(m, "contraction M tile limit")? as u32,
			n: narrow(n, "contraction N tile limit")? as u32,
			k: (narrow(k, "contraction K tile limit")? as u32).min(shared_values),
		})
	}
	fn proposal_limit(&self, values: [f64; 3]) -> Result<Tile> {
		let dimension = |value: f64, name| -> Result<u32> {
			require(
				value.is_finite() && value >= 1.0 && value.fract() == 0.0,
				format!("RAT {name} must be a positive integer"),
			)?;
			Ok(narrow(value as usize, name)? as u32)
		};
		Ok(Tile {
			m: dimension(values[0], "M")?,
			n: dimension(values[1], "N")?,
			k: dimension(values[2], "K")?.min(self.shared_values()?),
		})
	}
	fn prior_tile(&self, graph: &Graph) -> Result<Tile> {
		let limit = self.tile_limit(graph)?;
		let block = self.forward.geometry.block.min(self.epoch.geometry.block);
		Ok(Tile { m: limit.m.min(block), n: limit.n.min(block), k: limit.k.min(block) })
	}
	#[cfg_attr(not(any(amd, nvidia)), allow(unused_unsafe))]
	fn allocate(&self, bytes: usize) -> Result<u64> {
		self.activate()?;
		unsafe {
			match &self.driver {
				#[cfg(nvidia)]
				Driver::Cuda(driver) => {
					let mut pointer = 0;
					self.status((driver.allocate)(&mut pointer, bytes), "allocation")?;
					Ok(pointer)
				}
				#[cfg(amd)]
				Driver::Hsa(driver) => {
					let mut pointer = ptr::null_mut();
					self.status((driver.allocate)(driver.vram_pool, bytes, 0, &mut pointer), "allocation")?;
					self.status(
						(driver.allow)(1, &driver.cpu_agent, ptr::null(), pointer),
						"CPU allocation access",
					)?;
					Ok(pointer as u64)
				}
				Driver::Remote(driver) => driver.allocate(bytes),
			}
		}
	}
	#[cfg_attr(not(any(amd, nvidia)), allow(unused_unsafe))]
	fn free(&self, pointer: u64) {
		unsafe {
			match &self.driver {
				#[cfg(nvidia)]
				Driver::Cuda(driver) => {
					(driver.set)(driver.context);
					(driver.free)(pointer);
				}
				#[cfg(amd)]
				Driver::Hsa(driver) => {
					(driver.free)(pointer as Ptr);
				}
				Driver::Remote(driver) => driver.free(pointer),
			}
		}
	}
	#[cfg_attr(not(any(amd, nvidia)), allow(unused_unsafe))]
	fn upload(&self, dst: u64, src: *const c_void, bytes: usize) -> Result<()> {
		self.activate()?;
		unsafe {
			match &self.driver {
				#[cfg(nvidia)]
				Driver::Cuda(driver) => self.status((driver.upload)(dst, src, bytes), "upload"),
				#[cfg(amd)]
				Driver::Hsa(driver) => self.status((driver.copy)(dst as Ptr, src, bytes), "upload"),
				Driver::Remote(driver) => driver.upload(dst, src, bytes),
			}
		}
	}
	#[cfg_attr(not(any(amd, nvidia)), allow(unused_unsafe))]
	fn download(&self, dst: Ptr, src: u64, bytes: usize) -> Result<()> {
		self.activate()?;
		unsafe {
			match &self.driver {
				#[cfg(nvidia)]
				Driver::Cuda(cuda) => self.status((cuda.download)(dst, src, bytes), "download"),
				#[cfg(amd)]
				Driver::Hsa(driver) => self.status((driver.copy)(dst, src as *const c_void, bytes), "download"),
				Driver::Remote(driver) => driver.download(dst, src, bytes),
			}
		}
	}
	#[cfg_attr(not(any(amd, nvidia)), allow(unused_unsafe))]
	fn synchronize(&self) -> Result<()> {
		self.activate()?;
		unsafe {
			match &self.driver {
				#[cfg(nvidia)]
				Driver::Cuda(driver) => self.status((driver.synchronize)(), "synchronization"),
				#[cfg(amd)]
				Driver::Hsa(driver) => require((driver.wait)(driver.signal, 0, 0, u64::MAX, 1) == 0, "AMD synchronization failed"),
				Driver::Remote(driver) => driver.synchronize(),
			}
		}
	}
	#[cfg_attr(not(any(amd, nvidia)), allow(unused_unsafe, unused_variables))]
	fn launch(&self, dispatch: Dispatch, arguments: &mut [Ptr], threads: u32, tile: Tile) -> Result<()> {
		require(!INTERRUPTED.load(Ordering::Acquire), "interrupted before GPU dispatch")?;
		self.activate()?;
		let block = dispatch.geometry.block;
		let kernel = dispatch.kernel;
		let dynamic = shared_bytes(tile.k)?;
		let shared = kernel
			.shared
			.checked_add(dynamic)
			.ok_or_else(|| RecipeError::new("GPU shared memory size overflows"))?;
		require(shared <= self.shared_limit, "GPU shared memory exceeds its device limit")?;
		let _guard = self.dispatch.lock().map_err(|_| RecipeError::new("GPU dispatch lock is poisoned"))?;
		unsafe {
			match &self.driver {
				#[cfg(nvidia)]
				Driver::Cuda(driver) => {
					let stream = ptr::null_mut();
					self.status(
						(driver.launch)(
							kernel.object as usize,
							threads / block,
							1,
							1,
							block,
							1,
							1,
							dynamic,
							stream,
							arguments.as_mut_ptr(),
						),
						"dispatch",
					)
				}
				#[cfg(amd)]
				Driver::Hsa(driver) => {
					require(arguments.len() == kernel.layout.len(), "kernel argument count is invalid")?;
					ptr::write_bytes(driver.kernarg.cast::<u8>(), 0, driver.kernarg_size);
					let mut offset = 0;
					for (argument, kind) in arguments.iter().zip(kernel.layout) {
						let bytes = usize::from(*kind - b'0');
						ptr::copy_nonoverlapping(
							(*argument).cast::<u8>(),
							driver.kernarg.cast::<u8>().add(offset),
							bytes,
						);
						offset += bytes;
					}
					require(
						offset <= kernel.kernarg && kernel.kernarg <= driver.kernarg_size,
						"kernarg layout is invalid",
					)?;
					(driver.store)(driver.signal, 1);
					let queue = &mut *(driver.queue as *mut HsaQueue);
					let index = (driver.write)(queue, 1);
					let packet =
						queue.base.cast::<HsaPacket>().add(index as usize & (queue.size as usize - 1));
					packet.write(HsaPacket {
						header: 0,
						setup: 1,
						workgroup_x: block as u16,
						workgroup_y: 1,
						workgroup_z: 1,
						reserved0: 0,
						grid_x: threads,
						grid_y: 1,
						grid_z: 1,
						private: kernel.private,
						group: shared,
						object: kernel.object,
						kernarg: driver.kernarg,
						reserved1: 0,
						completion: driver.signal,
					});
					std::sync::atomic::fence(Ordering::Release);
					let header = &*(&mut (*packet).header as *mut u16 as *mut std::sync::atomic::AtomicU16);
					header.store(2 | 2 << 9 | 2 << 11, Ordering::Release);
					(driver.store)(queue.doorbell, index as i64);
					require((driver.wait)(driver.signal, 0, 0, u64::MAX, 1) == 0, "AMD dispatch failed")
				}
				Driver::Remote(driver) => driver.launch(dispatch, arguments, threads, tile),
			}
		}
	}
}
static DEVICES: OnceLock<Result<Vec<Gpu>>> = OnceLock::new();
fn devices() -> Result<&'static [Gpu]> {
	DEVICES
		.get_or_init(|| {
			let mut found = Vec::new();
			let mut errors = Vec::new();
			for load in [load_amd as fn() -> Result<Vec<Gpu>>, load_nvidia] {
				match load() {
					Ok(mut devices) => found.append(&mut devices),
					Err(error) => errors.push(error.to_string()),
				}
			}
			if found.is_empty() { Err(RecipeError::new(errors.join("; "))) } else { Ok(found) }
		})
		.as_ref()
		.map(Vec::as_slice)
		.map_err(Clone::clone)
}
fn device(name: Option<&str>) -> Result<&'static Gpu> {
	let found = devices()?;
	if let Some(name) = name {
		return found
			.iter()
			.find(|gpu| gpu.name == name)
			.ok_or_else(|| RecipeError::new(format!("GPU {name:?} is absent")));
	}
	require(found.len() == 1, "multiple GPUs require named selection")?;
	Ok(&found[0])
}
fn worker_list() -> Result<()> {
	let host = fs::read_to_string("/etc/hostname")
		.map_err(|error| RecipeError::new(format!("cannot read hostname: {error}")))?;
	for gpu in devices()? {
		println!(
			"{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
			host.trim(),
			gpu.name,
			gpu.backend,
			gpu.forward.geometry.groups,
			gpu.forward.geometry.block,
			gpu.epoch.geometry.groups,
			gpu.epoch.geometry.block,
			gpu.shared_limit,
			gpu.forward.kernel.shared,
			gpu.epoch.kernel.shared,
			gpu.memory,
			gpu.clock
		);
	}
	Ok(())
}
fn worker_serve(name: &str) -> Result<()> {
	let gpu = device(Some(name))?;
	let mut input = std::io::stdin().lock();
	let mut output = std::io::stdout().lock();
	loop {
		let mut command = [0];
		if remote_io("read from", input.read(&mut command))? == 0 {
			return Ok(());
		}
		let result = (|| -> Result<Vec<u8>> {
			match command[0] {
				1 => Ok(gpu.allocate(get_u64(&mut input)? as usize)?.to_le_bytes().to_vec()),
				2 => {
					gpu.free(get_u64(&mut input)?);
					Ok(Vec::new())
				}
				3 => {
					let pointer = get_u64(&mut input)?;
					let bytes = get_u64(&mut input)? as usize;
					let mut payload = vec![0_u8; bytes];
					remote_io("read from", input.read_exact(&mut payload))?;
					gpu.upload(pointer, payload.as_ptr().cast(), bytes)?;
					Ok(Vec::new())
				}
				4 => {
					let pointer = get_u64(&mut input)?;
					let bytes = get_u64(&mut input)? as usize;
					let mut payload = vec![0_u8; bytes];
					gpu.download(payload.as_mut_ptr().cast(), pointer, bytes)?;
					Ok(payload)
				}
				5 => {
					gpu.synchronize()?;
					Ok(Vec::new())
				}
				6 => {
					let mut kernel = [0];
					remote_io("read from", input.read_exact(&mut kernel))?;
					let threads = get_u64(&mut input)? as u32;
					let tile = Tile {
						m: get_u64(&mut input)? as u32,
						n: get_u64(&mut input)? as u32,
						k: get_u64(&mut input)? as u32,
					};
					let count = get_u64(&mut input)? as usize;
					let mut values = (0..count).map(|_| get_u64(&mut input)).collect::<Result<Vec<_>>>()?;
					let mut arguments =
						values.iter_mut().map(|value| value as *mut u64 as Ptr).collect::<Vec<_>>();
					let dispatch = match kernel[0] {
						0 => gpu.forward,
						1 => gpu.epoch,
						_ => return Err(RecipeError::new("remote Recipe kernel is invalid")),
					};
					gpu.launch(dispatch, &mut arguments, threads, tile)?;
					Ok(Vec::new())
				}
				_ => Err(RecipeError::new("remote Recipe command is invalid")),
			}
		})();
		write_response(&mut output, result)?;
		remote_io("flush", output.flush())?;
	}
}
extern "C" fn worker_init() {
	if let Ok(mode) = std::env::var("RECIPE_WORKER") {
		let result = if mode == "list" {
			worker_list()
		} else if let Some(name) = mode.strip_prefix("serve|") {
			worker_serve(name)
		} else {
			Err(RecipeError::new("Recipe worker mode is invalid"))
		};
		let status = result.map_or_else(
			|error| {
				eprintln!("{error}");
				1
			},
			|_| 0,
		);
		std::process::exit(status)
	}
}
#[used]
#[unsafe(link_section = ".init_array")]
static WORKER_INIT: extern "C" fn() = worker_init;
fn transfer_time(source: &'static Gpu, target: &'static Gpu, bytes: usize, repetitions: usize) -> Result<f64> {
	let payload = vec![0_u8; bytes];
	let source = Buffer::upload(source, &payload)?;
	let started = Instant::now();
	for _ in 0..repetitions {
		let payload = source.download::<u8>(bytes)?;
		let target = Buffer::upload(target, &payload)?;
		std::hint::black_box(target.pointer);
	}
	Ok(started.elapsed().as_secs_f64() / repetitions as f64)
}
fn ssh_config() -> Result<PathBuf> {
	let path = PathBuf::from(env!("RECIPE_SSH_CONFIG"));
	if path.is_absolute() {
		return Ok(path);
	}
	let home = std::env::var_os("HOME").ok_or_else(|| RecipeError::new("HOME is absent"))?;
	Ok(PathBuf::from(home).join(path))
}
fn ssh_hosts() -> Result<Vec<String>> {
	let text = fs::read_to_string(ssh_config()?)
		.map_err(|error| RecipeError::new(format!("cannot read SSH config: {error}")))?;
	let mut hosts = text
		.lines()
		.filter_map(|line| line.trim().strip_prefix("Host "))
		.flat_map(str::split_whitespace)
		.filter(|host| !host.contains(['*', '?', '!']))
		.map(str::to_owned)
		.collect::<Vec<_>>();
	hosts.sort();
	hosts.dedup();
	Ok(hosts)
}
#[derive(Clone)]
struct RemoteNode {
	info: DeviceInfo,
	transport: String,
	device: String,
	backend: Backend,
	forward: Geometry,
	epoch: Geometry,
	memory: u64,
	clock: u64,
	shared_limit: u32,
	forward_shared: u32,
	epoch_shared: u32,
}
struct Worker {
	child: Child,
	input: ChildStdin,
	output: ChildStdout,
}
fn remote_io<T>(action: &str, result: io::Result<T>) -> Result<T> {
	result.map_err(|error| RecipeError::new(format!("cannot {action} remote Recipe worker: {error}")))
}
fn put_u64(output: &mut impl Write, value: u64) -> Result<()> {
	remote_io("write to", output.write_all(&value.to_le_bytes()))
}
fn get_u64(input: &mut impl Read) -> Result<u64> {
	let mut bytes = u64::default().to_le_bytes();
	remote_io("read from", input.read_exact(&mut bytes))?;
	Ok(u64::from_le_bytes(bytes))
}
fn write_response(output: &mut impl Write, result: Result<Vec<u8>>) -> Result<()> {
	let (status, payload) = match result {
		Ok(payload) => (0, payload),
		Err(error) => (1, error.to_string().into_bytes()),
	};
	remote_io("write to", output.write_all(&[status]))?;
	put_u64(output, payload.len() as u64)?;
	remote_io("write to", output.write_all(&payload))
}
fn read_response(worker: &mut Worker) -> Result<Vec<u8>> {
	let mut status = [0];
	remote_io("read from", worker.output.read_exact(&mut status))?;
	let bytes = get_u64(&mut worker.output)? as usize;
	let mut payload = vec![0_u8; bytes];
	remote_io("read from", worker.output.read_exact(&mut payload))?;
	if status[0] == 0 {
		return Ok(payload);
	}
	let message = String::from_utf8(payload).map_err(|_| RecipeError::new("remote Recipe error is not UTF-8"))?;
	Err(RecipeError::new(message))
}
impl Remote {
	fn worker(&self) -> Result<std::sync::MutexGuard<'_, Worker>> {
		self.io.lock().map_err(|_| RecipeError::new("remote Recipe worker lock is poisoned"))
	}
	fn allocate(&self, bytes: usize) -> Result<u64> {
		let mut worker = self.worker()?;
		remote_io("write to", worker.input.write_all(&[1]))?;
		put_u64(&mut worker.input, bytes as u64)?;
		remote_io("flush", worker.input.flush())?;
		let payload = read_response(&mut worker)?;
		require(payload.len() == size_of::<u64>(), "remote allocation response is invalid")?;
		Ok(u64::from_le_bytes(payload.try_into().map_err(|_| RecipeError::new("remote pointer is invalid"))?))
	}
	fn free(&self, pointer: u64) {
		if let Ok(mut worker) = self.worker() {
			let _ = remote_io("write to", worker.input.write_all(&[2]));
			let _ = put_u64(&mut worker.input, pointer);
			let _ = worker.input.flush();
			let _ = read_response(&mut worker);
		}
	}
	fn upload(&self, dst: u64, src: *const c_void, bytes: usize) -> Result<()> {
		let mut worker = self.worker()?;
		remote_io("write to", worker.input.write_all(&[3]))?;
		put_u64(&mut worker.input, dst)?;
		put_u64(&mut worker.input, bytes as u64)?;
		remote_io(
			"write to",
			worker.input.write_all(unsafe { std::slice::from_raw_parts(src.cast::<u8>(), bytes) }),
		)?;
		remote_io("flush", worker.input.flush())?;
		read_response(&mut worker).map(|_| ())
	}
	fn download(&self, dst: Ptr, src: u64, bytes: usize) -> Result<()> {
		let mut worker = self.worker()?;
		remote_io("write to", worker.input.write_all(&[4]))?;
		put_u64(&mut worker.input, src)?;
		put_u64(&mut worker.input, bytes as u64)?;
		remote_io("flush", worker.input.flush())?;
		let payload = read_response(&mut worker)?;
		require(payload.len() == bytes, "remote download response has the wrong size")?;
		unsafe { ptr::copy_nonoverlapping(payload.as_ptr(), dst.cast::<u8>(), bytes) };
		Ok(())
	}
	fn synchronize(&self) -> Result<()> {
		let mut worker = self.worker()?;
		remote_io("write to", worker.input.write_all(&[5]))?;
		remote_io("flush", worker.input.flush())?;
		read_response(&mut worker).map(|_| ())
	}
	fn launch(&self, dispatch: Dispatch, arguments: &mut [Ptr], threads: u32, tile: Tile) -> Result<()> {
		let mut worker = self.worker()?;
		remote_io("write to", worker.input.write_all(&[6, dispatch.kernel.object as u8]))?;
		for value in
			[u64::from(threads), u64::from(tile.m), u64::from(tile.n), u64::from(tile.k), arguments.len() as u64]
		{
			put_u64(&mut worker.input, value)?;
		}
		for (argument, kind) in arguments.iter().zip(dispatch.kernel.layout) {
			let mut value = u64::default().to_ne_bytes();
			unsafe {
				ptr::copy_nonoverlapping(
					(*argument).cast::<u8>(),
					value.as_mut_ptr(),
					usize::from(*kind - b'0'),
				)
			};
			put_u64(&mut worker.input, u64::from_ne_bytes(value))?;
		}
		remote_io("flush", worker.input.flush())?;
		read_response(&mut worker).map(|_| ())
	}
}
fn worker_process(host: &str, mode: &str) -> Result<Worker> {
	let executable = fs::read(
		std::env::current_exe()
			.map_err(|error| RecipeError::new(format!("cannot locate Recipe executable: {error}")))?,
	)
	.map_err(|error| RecipeError::new(format!("cannot read Recipe executable: {error}")))?;
	let command = format!(
		concat!(
			"worker=$(mktemp /tmp/recipe-worker.XXXXXX)\x3b trap 'rm -f \"$worker\"' EXIT\x3b ",
			"dd bs={} count=1 iflag=fullblock of=\"$worker\" status=none\x3b chmod 700 \"$worker\"\x3b ",
			"RECIPE_WORKER='{}' \"$worker\"",
		),
		executable.len(),
		mode
	);
	let mut child = Command::new("ssh")
		.arg("-F")
		.arg(ssh_config()?)
		.args(["-o", "BatchMode=yes", "-o"])
		.arg(format!("ConnectTimeout={}", env!("RECIPE_SSH_CONNECT_TIMEOUT")))
		.arg(host)
		.arg(command)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::inherit())
		.spawn()
		.map_err(|error| RecipeError::new(format!("cannot start Recipe worker on {host}: {error}")))?;
	let mut input = child.stdin.take().ok_or_else(|| RecipeError::new("Recipe worker stdin is absent"))?;
	let output = child.stdout.take().ok_or_else(|| RecipeError::new("Recipe worker stdout is absent"))?;
	input.write_all(&executable)
		.map_err(|error| RecipeError::new(format!("cannot upload Recipe worker: {error}")))?;
	Ok(Worker { child, input, output })
}
fn remote_nodes(host: &str) -> Result<Vec<RemoteNode>> {
	let Worker { mut child, input, mut output } = worker_process(host, "list")?;
	drop(input);
	let mut text = String::new();
	output.read_to_string(&mut text)
		.map_err(|error| RecipeError::new(format!("cannot read Recipe worker: {error}")))?;
	require(
		child.wait().map_err(|error| RecipeError::new(format!("cannot wait for {host}: {error}")))?.success(),
		format!("Recipe worker on {host} failed"),
	)?;
	text.lines()
		.map(|line| {
			let fields = line.split('|').collect::<Vec<_>>();
			require(fields.len() == 12, "Recipe worker device is invalid")?;
			let backend = match fields[2] {
				"Amd" => Backend::Amd,
				"Nvidia" => Backend::Nvidia,
				_ => return Err(RecipeError::new("Recipe worker backend is invalid")),
			};
			let positive = |index| -> Result<u32> {
				Ok(narrow(natural("remote device value", fields[index])?, "remote device value")? as u32)
			};
			let fixed = |index: usize| {
				fields[index]
					.parse::<u32>()
					.map_err(|error| RecipeError::new(format!("invalid remote device value: {error}")))
			};
			Ok(RemoteNode {
				info: DeviceInfo { name: format!("{}:{}", fields[0], fields[1]), host: fields[0].to_owned() },
				transport: host.to_owned(),
				device: fields[1].to_owned(),
				backend,
				forward: Geometry { groups: positive(3)?, block: positive(4)? },
				epoch: Geometry { groups: positive(5)?, block: positive(6)? },
				shared_limit: positive(7)?,
				forward_shared: fixed(8)?,
				epoch_shared: fixed(9)?,
				memory: fields[10]
					.parse()
					.map_err(|error| RecipeError::new(format!("invalid remote memory: {error}")))?,
				clock: natural("remote timestamp frequency", fields[11])? as u64,
			})
		})
		.collect()
}
fn host_has_gpu(host: &str) -> Result<bool> {
	Ok(Command::new("ssh")
		.arg("-F")
		.arg(ssh_config()?)
		.args(["-o", "BatchMode=yes", "-o"])
		.arg(format!("ConnectTimeout={}", env!("RECIPE_SSH_CONNECT_TIMEOUT")))
		.arg(host)
		.arg("test -d /sys/class/kfd/kfd/topology/nodes || command -v nvidia-smi >/dev/null")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.map_err(|error| RecipeError::new(format!("cannot probe {host}: {error}")))?
		.success())
}
static REMOTE_NODES: OnceLock<Result<Vec<RemoteNode>>> = OnceLock::new();
fn reachable_nodes() -> Result<&'static [RemoteNode]> {
	REMOTE_NODES
		.get_or_init(|| {
			let hosts = ssh_hosts()?;
			let capable = std::thread::scope(|scope| {
				let workers = hosts.iter().map(|host| scope.spawn(|| host_has_gpu(host))).collect::<Vec<_>>();
				workers
					.into_iter()
					.map(|worker| worker.join().map_err(|_| RecipeError::new("GPU host probe panicked"))?)
					.collect::<Result<Vec<_>>>()
			})?;
			let mut nodes = Vec::new();
			for (host, capable) in hosts.iter().zip(capable) {
				if capable {
					nodes.extend(remote_nodes(host)?);
				}
			}
			Ok(nodes)
		})
		.as_ref()
		.map(Vec::as_slice)
		.map_err(Clone::clone)
}
fn remote_gpu(node: &RemoteNode) -> Result<Gpu> {
	let worker = worker_process(&node.transport, &format!("serve|{}", node.device))?;
	Ok(Gpu {
		name: node.info.name.clone(), backend: node.backend,
		driver: Driver::Remote(Remote { io: Mutex::new(worker) }),
		forward: Dispatch { kernel: Kernel::remote(0, node.forward_shared, FORWARD_ARGS), geometry: node.forward },
		epoch: Dispatch { kernel: Kernel::remote(1, node.epoch_shared, EPOCH_ARGS), geometry: node.epoch },
		memory: node.memory, clock: node.clock, shared_limit: node.shared_limit, dispatch: Mutex::new(()),
	})
}
static REMOTE_GPUS: OnceLock<Result<Vec<Gpu>>> = OnceLock::new();
fn remote_gpus() -> Result<&'static [Gpu]> {
	REMOTE_GPUS
		.get_or_init(|| reachable_nodes()?.iter().map(remote_gpu).collect())
		.as_ref()
		.map(Vec::as_slice)
		.map_err(Clone::clone)
}
fn all_gpus() -> Result<Vec<&'static Gpu>> {
	let mut found = devices()?.iter().collect::<Vec<_>>();
	if multi_device() {
		found.extend(remote_gpus()?.iter());
	}
	Ok(found)
}
static TOPOLOGY: OnceLock<Result<Topology>> = OnceLock::new();
fn topology() -> Result<Topology> {
	TOPOLOGY.get_or_init(discover_topology).as_ref().cloned().map_err(Clone::clone)
}
fn discover_topology() -> Result<Topology> {
	let found = devices()?;
	let latency_bytes = natural("topology latency bytes", env!("RECIPE_TOPOLOGY_LATENCY_BYTES"))?;
	let bandwidth_bytes = natural("topology bandwidth bytes", env!("RECIPE_TOPOLOGY_BANDWIDTH_BYTES"))?;
	let repetitions = natural("topology repetitions", env!("RECIPE_TOPOLOGY_REPETITIONS"))?;
	require(latency_bytes != 0 && bandwidth_bytes >= latency_bytes && repetitions != 0, "topology probe is invalid")?;
	let host = fs::read_to_string("/etc/hostname")
		.map_err(|error| RecipeError::new(format!("cannot read hostname: {error}")))?;
	let host = host.trim().to_owned();
	let mut nodes = found
		.iter()
		.map(|gpu| DeviceInfo { name: format!("{host}:{}", gpu.name), host: host.clone() })
		.collect::<Vec<_>>();
	let remote: &[RemoteNode] = if multi_device() { reachable_nodes()? } else { &[] };
	nodes.extend(remote.iter().map(|node| node.info.clone()));
	let name = |gpu: &Gpu| {
		if matches!(&gpu.driver, Driver::Remote(_)) { gpu.name.clone() } else { format!("{host}:{}", gpu.name) }
	};
	let active = all_gpus()?;
	let mut links = Vec::new();
	for source in &active {
		for target in &active {
			let latency = transfer_time(source, target, latency_bytes, repetitions)?;
			let duration = transfer_time(source, target, bandwidth_bytes, repetitions)?;
			links.push(DeviceLink {
				from: name(source),
				to: name(target),
				latency_ms: latency * 1000.0,
				bytes_per_second: bandwidth_bytes as f64 / duration,
			});
		}
	}
	Ok(Topology { devices: nodes, links })
}
#[cfg(amd)]
type HsaInfo = unsafe extern "C" fn(u64, i32, Ptr) -> i32;
#[cfg(amd)]
struct HsaQuery {
	info: HsaInfo,
	attribute: i32,
	expected: u32,
	secondary: i32,
	mask: u32,
	found: u64,
}
#[cfg(amd)]
extern "C" fn collect_hsa(handle: u64, pointer: Ptr) -> i32 {
	unsafe {
		let query = &mut *pointer.cast::<HsaQuery>();
		let mut value = 0;
		let mut status = (query.info)(handle, query.attribute, (&mut value as *mut u32).cast());
		if status != 0 || value != query.expected {
			return status;
		}
		if query.secondary >= 0 {
			status = (query.info)(handle, query.secondary, (&mut value as *mut u32).cast());
			if status != 0 || value & query.mask == 0 {
				return status;
			}
		}
		if query.found == 0 {
			query.found = handle;
		}
		0
	}
}
#[cfg(amd)]
struct HsaGpuQuery {
	info: HsaInfo,
	found: Vec<u64>,
}
#[cfg(amd)]
extern "C" fn collect_discrete_hsa(handle: u64, pointer: Ptr) -> i32 {
	unsafe {
		let query = &mut *pointer.cast::<HsaGpuQuery>();
		let mut device = 0_u32;
		let mut status = (query.info)(handle, 17, (&mut device as *mut u32).cast());
		if status != 0 || device != 1 {
			return status;
		}
		let mut properties = 0_u64;
		status = (query.info)(handle, 0xA114, (&mut properties as *mut u64).cast());
		if status != 0 || properties & 1 != 0 {
			return status;
		}
		query.found.push(handle);
		0
	}
}
#[cfg(amd)]
type HsaSymbol = unsafe extern "C" fn(u64, *const u8, *const u64, *mut u64) -> i32;
#[cfg(amd)]
type HsaSymbolInfo = unsafe extern "C" fn(u64, i32, Ptr) -> i32;
#[cfg(amd)]
unsafe fn hsa_kernel(
	symbol: HsaSymbol,
	info: HsaSymbolInfo,
	executable: u64,
	agent: u64,
	name: &'static [u8],
	layout: &'static [u8],
) -> Result<Kernel> {
	let mut handle = 0;
	driver_status(Backend::Amd, unsafe { symbol(executable, name.as_ptr(), &agent, &mut handle) }, "kernel lookup")?;
	let mut kernel = Kernel { object: 0, shared: 0, kernarg: 0, private: 0, layout };
	for (attribute, output) in [
		(22, (&mut kernel.object as *mut u64).cast()),
		(11, (&mut kernel.kernarg as *mut usize).cast()),
		(13, (&mut kernel.shared as *mut u32).cast()),
		(14, (&mut kernel.private as *mut u32).cast()),
	] {
		driver_status(Backend::Amd, unsafe { info(handle, attribute, output) }, "kernel metadata")?;
	}
	Ok(kernel)
}
#[cfg(amd)]
fn kfd_property(text: &str, name: &str) -> Result<u32> {
	text.lines()
		.find_map(|line| line.split_once(' ').filter(|value| value.0 == name))
		.ok_or_else(|| RecipeError::new(format!("KFD property {name:?} is absent")))?
		.1
		.parse::<u32>()
		.map_err(|error| RecipeError::new(format!("KFD property {name:?} is invalid: {error}")))
}
#[cfg(amd)]
include!(concat!(env!("OUT_DIR"), "/hsa-embed.rs"));
#[cfg(amd)]
fn hsa_artifact(target: &str) -> Result<&'static [u8]> {
	HSA_CODE_OBJECTS
		.iter()
		.find_map(|entry| (entry.0 == target).then_some(entry.1))
		.ok_or_else(|| RecipeError::new(format!("HSA artifact for {target} is absent")))
}
fn load_amd() -> Result<Vec<Gpu>> {
	#[cfg(not(amd))]
	return Err(RecipeError::new("AMD support is not compiled into this build"));
	#[cfg(amd)]
	unsafe {
		let runtime = Library::open(env!("RECIPE_HSA_RUNTIME"))?;
		let init: unsafe extern "C" fn() -> i32 = runtime.function(b"hsa_init\0")?;
		let iterate: unsafe extern "C" fn(extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 =
			runtime.function(b"hsa_iterate_agents\0")?;
		let info: HsaInfo = runtime.function(b"hsa_agent_get_info\0")?;
		let check = |s, a| driver_status(Backend::Amd, s, a);
		check(init(), "initialization")?;
		let mut cpu = HsaQuery { info, attribute: 17, expected: 0, secondary: -1, mask: 0, found: 0 };
		let mut gpu = HsaGpuQuery { info, found: Vec::new() };
		check(iterate(collect_hsa, (&mut cpu as *mut HsaQuery).cast()), "CPU agent")?;
		check(iterate(collect_discrete_hsa, (&mut gpu as *mut HsaGpuQuery).cast()), "GPU agent")?;
		require(cpu.found != 0 && !gpu.found.is_empty(), "AMD CPU or discrete GPU agent is absent")?;
		gpu.found
			.into_iter()
			.enumerate()
			.map(|(index, agent)| load_amd_gpu(&runtime, info, cpu.found, agent, index))
			.collect()
	}
}
#[cfg(amd)]
fn load_amd_gpu(runtime: &Library, info: HsaInfo, cpu_agent: u64, agent: u64, index: usize) -> Result<Gpu> {
	unsafe {
		let pool_info: HsaInfo = runtime.function(b"hsa_amd_memory_pool_get_info\0")?;
		let pool_iterate: unsafe extern "C" fn(u64, extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 =
			runtime.function(b"hsa_amd_agent_iterate_memory_pools\0")?;
		let check = |s, a| driver_status(Backend::Amd, s, a);
		let mut vram = HsaQuery { info: pool_info, attribute: 0, expected: 0, secondary: 1, mask: 4, found: 0 };
		let mut kernarg = HsaQuery { info: pool_info, attribute: 0, expected: 0, secondary: 1, mask: 1, found: 0 };
		check(pool_iterate(agent, collect_hsa, (&mut vram as *mut HsaQuery).cast()), "VRAM pools")?;
		check(pool_iterate(cpu_agent, collect_hsa, (&mut kernarg as *mut HsaQuery).cast()), "KERNARG pools")?;
		require(vram.found != 0 && kernarg.found != 0, "AMD VRAM or KERNARG pool is absent")?;
		let (mut memory, mut clock) = (0_usize, 0_u64);
		check(pool_info(vram.found, 2, (&mut memory as *mut usize).cast()), "VRAM size")?;
		check(info(agent, 0xA016, (&mut clock as *mut u64).cast()), "timestamp frequency")?;
		require(clock != 0, "AMD timestamp frequency must be positive")?;
		let (mut wave, mut workgroup, mut available, mut node, mut cus) = (0_u32, 0_u32, 0_u32, 0_u32, 0_u32);
		for (attribute, output, action) in [
			(6, (&mut wave as *mut u32).cast(), "wave query"),
			(8, (&mut workgroup as *mut u32).cast(), "workgroup query"),
			(0xA002, (&mut available as *mut u32).cast(), "CU query"),
			(0xA004, (&mut node as *mut u32).cast(), "KFD node query"),
			(0xA014, (&mut cus as *mut u32).cast(), "cooperative CU query"),
		] {
			check(info(agent, attribute, output), action)?;
		}
		require(cus <= available, "AMD cooperative CU count exceeds available CUs")?;
		let path = format!("/sys/class/kfd/kfd/topology/nodes/{node}/properties");
		let properties = fs::read_to_string(&path)
			.map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
		let gfx = kfd_property(&properties, "gfx_target_version")?;
		let target = format!("gfx{}{}{}", gfx / 10000, gfx / 100 % 100, gfx % 100);
		let code = hsa_artifact(&target)?;
		let reader_create: unsafe extern "C" fn(*const c_void, usize, *mut u64) -> i32 =
			runtime.function(b"hsa_code_object_reader_create_from_memory\0")?;
		let executable_create: unsafe extern "C" fn(i32, i32, Ptr, *mut u64) -> i32 =
			runtime.function(b"hsa_executable_create_alt\0")?;
		let executable_load: unsafe extern "C" fn(u64, u64, u64, Ptr, Ptr) -> i32 =
			runtime.function(b"hsa_executable_load_agent_code_object\0")?;
		let executable_freeze: unsafe extern "C" fn(u64, Ptr) -> i32 =
			runtime.function(b"hsa_executable_freeze\0")?;
		let symbol: HsaSymbol = runtime.function(b"hsa_executable_get_symbol_by_name\0")?;
		let symbol_info: HsaSymbolInfo = runtime.function(b"hsa_executable_symbol_get_info\0")?;
		let (mut reader, mut executable) = (0, 0);
		check(reader_create(code.as_ptr().cast(), code.len(), &mut reader), "code-object reader")?;
		check(executable_create(1, 0, ptr::null_mut(), &mut executable), "executable creation")?;
		check(executable_load(executable, agent, reader, ptr::null_mut(), ptr::null_mut()), "code-object load")?;
		check(executable_freeze(executable, ptr::null_mut()), "executable freeze")?;
		let forward = hsa_kernel(symbol, symbol_info, executable, agent, b"forward_graph.kd\0", FORWARD_ARGS)?;
		let epoch = hsa_kernel(symbol, symbol_info, executable, agent, b"tape_epoch_graph.kd\0", EPOCH_ARGS)?;
		let forward_resources = Resources { shared: forward.shared, max_block: workgroup };
		let epoch_resources = Resources { shared: epoch.shared, max_block: workgroup };
		let lds = kfd_property(&properties, "lds_size_in_kb")?
			.checked_mul(1024)
			.ok_or_else(|| RecipeError::new("AMD LDS size overflows"))?;
		let forward_geometry = amd(cus, wave, workgroup, lds, forward_resources)?;
		let epoch_geometry = amd(cus, wave, workgroup, lds, epoch_resources)?;
		let queue_create: unsafe extern "C" fn(u64, u32, u32, Ptr, Ptr, u32, u32, *mut Ptr) -> i32 =
			runtime.function(b"hsa_queue_create\0")?;
		let signal_create: unsafe extern "C" fn(i64, u32, *const u64, *mut u64) -> i32 =
			runtime.function(b"hsa_signal_create\0")?;
		let allocate: unsafe extern "C" fn(u64, usize, u32, *mut Ptr) -> i32 =
			runtime.function(b"hsa_amd_memory_pool_allocate\0")?;
		let allow: unsafe extern "C" fn(u32, *const u64, *const u32, *const c_void) -> i32 =
			runtime.function(b"hsa_amd_agents_allow_access\0")?;
		let (ka_size, mut ka) = (forward.kernarg.max(epoch.kernarg), ptr::null_mut());
		let (mut queue, mut completion) = (ptr::null_mut(), 0);
		driver_status(
			Backend::Amd,
			queue_create(agent, 256, 2, ptr::null_mut(), ptr::null_mut(), u32::MAX, u32::MAX, &mut queue),
			"queue creation",
		)?;
		check(signal_create(0, 0, ptr::null(), &mut completion), "signal creation")?;
		check(allocate(kernarg.found, ka_size, 0, &mut ka), "KERNARG allocation")?;
		check(allow(1, &agent, ptr::null(), ka), "GPU KERNARG access")?;
		eprintln!("AMD forward block {} epoch block {}", forward_geometry.block, epoch_geometry.block);
		Ok(Gpu {
			name: format!("amd{index}"), backend: Backend::Amd,
			driver: Driver::Hsa(Hsa {
				allocate, allow, queue, cpu_agent, kernarg_size: ka_size, kernarg: ka,
				free: runtime.function(b"hsa_amd_memory_pool_free\0")?,
				copy: runtime.function(b"hsa_memory_copy\0")?,
				store: runtime.function(b"hsa_signal_store_screlease\0")?,
				wait: runtime.function(b"hsa_signal_wait_scacquire\0")?,
				write: runtime.function(b"hsa_queue_add_write_index_scacq_screl\0")?,
				signal: completion, vram_pool: vram.found, kernarg_pool: kernarg.found,
			}),
			forward: Dispatch { kernel: forward, geometry: forward_geometry },
			epoch: Dispatch { kernel: epoch, geometry: epoch_geometry },
			memory: memory as u64, clock, shared_limit: lds, dispatch: Mutex::new(()),
		})
	}
}
fn load_nvidia() -> Result<Vec<Gpu>> {
	#[cfg(not(nvidia))]
	return Err(RecipeError::new("NVIDIA support is not compiled into this build"));
	#[cfg(nvidia)]
	unsafe {
		const MAX_BLOCK: i32 = 1;
		const BLOCK_LDS: i32 = 8;
		const WAVE: i32 = 10;
		const CUS: i32 = 16;
		const INTEGRATED: i32 = 18;
		const THREADS_PER_SM: i32 = 39;
		const SM_LDS: i32 = 81;
		const REGISTERS_PER_SM: i32 = 82;
		const COOPERATIVE: i32 = 95;
		const NANOSECOND_TIMER_HZ: u64 = 1_000_000_000;
		let runtime = Library::open(env!("RECIPE_NV_RUNTIME"))?;
		let init: unsafe extern "C" fn(u32) -> i32 = runtime.function(b"cuInit\0")?;
		let count_devices: unsafe extern "C" fn(*mut i32) -> i32 = runtime.function(b"cuDeviceGetCount\0")?;
		let get_device: unsafe extern "C" fn(*mut i32, i32) -> i32 = runtime.function(b"cuDeviceGet\0")?;
		let attribute: NvQuery = runtime.function(b"cuDeviceGetAttribute\0")?;
		let total: unsafe extern "C" fn(*mut usize, i32) -> i32 = runtime.function(b"cuDeviceTotalMem_v2\0")?;
		let create: unsafe extern "C" fn(*mut Ptr, u32, i32) -> i32 = runtime.function(b"cuCtxCreate_v2\0")?;
		let load: unsafe extern "C" fn(*mut Ptr, *const c_void) -> i32 = runtime.function(b"cuModuleLoadData\0")?;
		let function: unsafe extern "C" fn(*mut usize, Ptr, *const u8) -> i32 = runtime.function(b"cuModuleGetFunction\0")?;
		let function_attribute: unsafe extern "C" fn(*mut i32, i32, usize) -> i32 = runtime.function(b"cuFuncGetAttribute\0")?;
		let occupancy: unsafe extern "C" fn(*mut i32, usize, i32, usize) -> i32 = runtime.function(b"cuOccupancyMaxActiveBlocksPerMultiprocessor\0")?;
		let check = |s, a| driver_status(Backend::Nvidia, s, a);
		let mut count = 0;
		check(init(0), "initialization")?;
		check(count_devices(&mut count), "device enumeration")?;
		let load_device = |device, index| -> Result<Gpu> {
			let check = |s, a| driver_status(Backend::Nvidia, s, a);
			let (mut forward, mut epoch) = (0, 0);
			let (mut context, mut module) = (ptr::null_mut(), ptr::null_mut());
			let (mut cus, mut wave, mut workgroup, mut block_lds, mut sm_lds, mut registers, mut threads, mut cooperative)
				= (0, 0, 0, 0, 0, 0, 0, 0);
			let mut memory = 0;
			check(total(&mut memory, device), "VRAM size")?;
			for (kind, output, action) in [
				(CUS, &mut cus, "SM query"),
				(WAVE, &mut wave, "warp query"),
				(MAX_BLOCK, &mut workgroup, "workgroup query"),
				(BLOCK_LDS, &mut block_lds, "workgroup LDS query"),
				(SM_LDS, &mut sm_lds, "SM LDS query"),
				(REGISTERS_PER_SM, &mut registers, "register query"),
				(THREADS_PER_SM, &mut threads, "resident thread query"),
				(COOPERATIVE, &mut cooperative, "cooperative launch query"),
			] {
				check(attribute(output, kind, device), action)?;
			}
			require(cooperative != 0, "Nvidia device does not support cooperative launch")?;
			check(create(&mut context, 0, device), "context creation")?;
			let image = std::ffi::CString::new(include_bytes!(env!("RECIPE_NV_PTX")).as_slice())
				.map_err(|error| RecipeError::new(format!("Nvidia PTX contains a zero byte: {error}")))?;
			check(load(&mut module, image.as_ptr().cast()), "module load")?;
			check(function(&mut forward, module, b"forward_graph\0".as_ptr()), "forward")?;
			check(function(&mut epoch, module, b"tape_epoch_graph\0".as_ptr()), "epoch")?;
			let resource = |kernel| -> Result<(Resources, u32)> {
				let (mut max_block, mut shared, mut used_registers) = (0, 0, 0);
				for (kind, output, action) in [
					(0, &mut max_block, "kernel workgroup query"),
					(1, &mut shared, "kernel LDS query"),
					(4, &mut used_registers, "kernel register query"),
				] {
					check(function_attribute(output, kind, kernel), action)?;
				}
				require(
					max_block > 0 && shared >= 0 && used_registers > 0,
					"Nvidia kernel resources are invalid",
				)?;
				Ok((Resources { shared: shared as u32, max_block: max_block as u32 }, used_registers as u32))
			};
			let (forward_resource, forward_registers) = resource(forward)?;
			let (epoch_resource, epoch_registers) = resource(epoch)?;
			let geometry = |resources: Resources, used_registers: u32| -> Result<Geometry> {
				let register_wave = used_registers.checked_mul(wave as u32)
					.ok_or_else(|| RecipeError::new("Nvidia wave register count overflows"))?;
				let observed = (registers as u32 / register_wave).min(threads as u32 / wave as u32);
				nvidia(cus as u32, wave as u32, workgroup as u32, block_lds as u32, sm_lds as u32, observed, resources)
			};
			let forward_geometry = geometry(forward_resource, forward_registers)?;
			let epoch_geometry = geometry(epoch_resource, epoch_registers)?;
			for (kernel, geometry, action) in
				[(forward, forward_geometry, "forward occupancy"), (epoch, epoch_geometry, "epoch occupancy")]
			{
				let mut active = 0;
				driver_status(
					Backend::Nvidia,
					occupancy(&mut active, kernel, geometry.block as i32, 0),
					action,
				)?;
				require(active > 0, format!("Nvidia {action} has no resident workgroup"))?;
			}
			let cuda = Cuda {
				context,
				set: runtime.function(b"cuCtxSetCurrent\0")?,
				allocate: runtime.function(b"cuMemAlloc_v2\0")?,
				free: runtime.function(b"cuMemFree_v2\0")?,
				upload: runtime.function(b"cuMemcpyHtoD_v2\0")?,
				download: runtime.function(b"cuMemcpyDtoH_v2\0")?,
				synchronize: runtime.function(b"cuCtxSynchronize\0")?,
				launch: runtime.function(b"cuLaunchCooperativeKernel\0")?,
			};
			eprintln!("Nvidia forward block {} epoch block {}", forward_geometry.block, epoch_geometry.block);
			Ok(Gpu {
				name: format!("nv{index}"), backend: Backend::Nvidia, driver: Driver::Cuda(cuda),
				forward: Dispatch { kernel: Kernel::cuda(forward, forward_resource.shared, FORWARD_ARGS), geometry: forward_geometry },
				epoch: Dispatch { kernel: Kernel::cuda(epoch, epoch_resource.shared, EPOCH_ARGS), geometry: epoch_geometry },
				memory: memory as u64, clock: NANOSECOND_TIMER_HZ,
				shared_limit: (block_lds as u32).min(sm_lds as u32), dispatch: Mutex::new(()),
			})
		};
		let mut found = Vec::new();
		for ordinal in 0..count {
			let (mut gpu, mut integrated) = (0, 0);
			check(get_device(&mut gpu, ordinal), "device enumeration")?;
			check(attribute(&mut integrated, INTEGRATED, gpu), "device probe")?;
			if integrated == 0 {
				found.push(load_device(gpu, found.len())?)
			}
		}
		require(!found.is_empty(), "Nvidia has no discrete GPU")?;
		Ok(found)
	}
}
#[cfg(any(amd, nvidia))]
#[link(name = "dl")]
unsafe extern "C" {
	fn dlopen(name: *const std::ffi::c_char, flags: i32) -> Ptr;
	fn dlsym(handle: Ptr, name: *const std::ffi::c_char) -> Ptr;
}
unsafe extern "C" {
	fn signal(number: i32, handler: extern "C" fn(i32)) -> usize;
	fn write(file: i32, bytes: *const c_void, length: usize) -> isize;
}
const TILE_CAPACITY: usize = 65536;
unsafe extern "C" {
	fn recipe_forward_cpu(
		samples: *const f64,
		weights: *const f64,
		value_pointers: *const *mut f64,
		context_pointers: *const *mut f64,
		descriptors: *const i32,
		arguments: *const f64,
		rows: i32,
		nodes: i32,
		threads: i32,
		tile_m: i32,
		tile_n: i32,
		tile_k: i32,
		timings: *mut u64,
		tiles: *const Tile,
	);
}
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
fn graph_inputs(graph: &Graph, samples: &[f64], targets: &[f64], rows: usize, gpu: &'static Gpu) -> Result<Vec<f64>> {
	let input_count = checked_mul(rows, graph.input.elements(), "estimator input slice")?;
	if graph.nodes.is_empty() {
		return Ok(samples[..rows * graph.output.elements()].to_vec());
	}
	let mut tape = GpuTape::new(graph, &samples[..input_count], &targets[..rows], gpu)?;
	tape.forward()?;
	tape.predictions()
}
fn fit_surrogate(
	input: Shape,
	samples: &[f64],
	targets: &[f64],
	hidden: usize,
	gpu: &'static Gpu,
	config: Config,
) -> Result<Graph> {
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
		sequence: None,
		norm_mean: Vec::new(),
		norm_scale: Vec::new(),
		identities: Vec::new(),
	};
	let mut graph = compile_output(&model, &prepared, prepared.rows, gpu, config, 1)?;
	let mut tape = GpuTape::new(&graph, samples, targets, gpu)?;
	for _ in 0..config.surrogate_epochs {
		tape.advance()?;
		tape.epoch(config.surrogate_rate, mse, 0.0, config, false)?;
	}
	tape.capture(&mut graph)?;
	graph.frozen.fill(1);
	Ok(graph)
}
type Predictor = Box<dyn Fn(usize, &[f64]) -> f64>;
fn fit_kmeans(clusters: usize, data: &Prepared, rows: usize, config: Config, _: bool) -> Result<Predictor> {
	require(clusters != 0 && clusters <= rows, "kmeans cluster count is invalid")?;
	let mut centers = data.samples[..clusters * data.features].to_vec();
	let mut assignments = vec![0_usize; rows];
	let mut distances = vec![0.0; rows];
	for _ in 0..config.kmeans_iterations {
		for (row, sample) in data.samples[..rows * data.features].chunks_exact(data.features).enumerate() {
			let selected = nearest(sample, &centers, data.features);
			assignments[row] = selected.0;
			distances[row] = selected.1;
		}
		for cluster in 0..clusters {
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
					.max_by(|a, b| a.1.total_cmp(b.1))
					.map(|value| value.0)
					.ok_or_else(|| RecipeError::new("kmeans has no training row"))?;
				centers[cluster * data.features..(cluster + 1) * data.features]
					.copy_from_slice(&data.samples[worst * data.features..(worst + 1) * data.features]);
				distances[worst] = -1.0;
			} else {
				for feature in 0..data.features {
					centers[cluster * data.features + feature] =
						members.iter().map(|row| data.samples[row * data.features + feature]).sum::<f64>()
							/ members.len() as f64;
				}
			}
		}
	}
	let features = data.features;
	Ok(Box::new(move |_, query| nearest(query, &centers, features).0 as f64))
}
fn fit_knn(count: usize, data: &Prepared, rows: usize, _: Config, exclude: bool) -> Result<Predictor> {
	let maximum = rows - usize::from(exclude);
	require(count != 0 && count <= maximum, "knn neighbor count is invalid")?;
	let features = data.features;
	let samples = data.samples[..rows * features].to_vec();
	let targets = data.targets[..rows].to_vec();
	Ok(Box::new(move |row, query| {
		let mut nearest = samples
			.chunks_exact(features)
			.enumerate()
			.filter(|value| !exclude || value.0 != row)
			.map(|value| (distance(query, value.1), targets[value.0]))
			.collect::<Vec<_>>();
		nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
		nearest.iter().take(count).map(|value| value.1).sum::<f64>() / count as f64
	}))
}
impl Estimator {
	fn fit(&self, data: &Prepared, rows: usize, config: Config, exclude: bool) -> Result<Predictor> {
		(self.fit)(self.param, data, rows, config, exclude)
	}
}
const DOUBLE_BUFFER_VALUES: u32 = 2;
fn shared_bytes(tile: u32) -> Result<u32> {
	tile.max(DOUBLE_BUFFER_VALUES)
		.checked_mul(size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("GPU shared memory size overflows"))
}
#[cfg(any(amd, nvidia))]
#[derive(Clone, Copy)]
struct Resources {
	pub shared: u32,
	pub max_block: u32,
}
#[derive(Clone, Copy)]
struct Geometry {
	pub groups: u32,
	pub block: u32,
}
impl Geometry {
	pub fn threads(self, work: u32) -> Result<u32> {
		self.groups
			.min(work.div_ceil(self.block))
			.checked_mul(self.block)
			.filter(|value| *value != 0)
			.ok_or_else(|| RecipeError::new("GPU launch size overflows"))
	}
}
#[cfg(any(amd, nvidia))]
fn geometry(
	cus: u32,
	wave: u32,
	workgroup: u32,
	lds: u32,
	groups_per_cu: u32,
	resources: Resources,
) -> Result<Geometry> {
	require(wave != 0 && wave <= workgroup && wave <= resources.max_block, "GPU wave exceeds kernel workgroup")?;
	let waves = groups_per_cu.min(workgroup / wave).min(resources.max_block / wave);
	require(waves != 0, "GPU has no resident wave")?;
	let block = waves.checked_mul(wave).ok_or_else(|| RecipeError::new("GPU workgroup size overflows"))?;
	let shared = resources
		.shared
		.checked_add(shared_bytes(0)?)
		.ok_or_else(|| RecipeError::new("GPU shared memory size overflows"))?;
	require(shared <= lds, "GPU tile exceeds local memory")?;
	Ok(Geometry { groups: cus, block })
}
#[cfg(amd)]
fn amd(cus: u32, wave: u32, workgroup: u32, lds: u32, resources: Resources) -> Result<Geometry> {
	geometry(cus, wave, workgroup, lds, u32::from(wave != 0), resources)
}
#[cfg(nvidia)]
fn nvidia(
	cus: u32,
	wave: u32,
	workgroup: u32,
	block_lds: u32,
	sm_lds: u32,
	waves_per_cu: u32,
	resources: Resources,
) -> Result<Geometry> {
	let shared = resources
		.shared
		.checked_add(shared_bytes(0)?)
		.ok_or_else(|| RecipeError::new("Nvidia shared memory size overflows"))?;
	require(shared <= block_lds, "Nvidia tile exceeds workgroup shared memory")?;
	geometry(cus, wave, workgroup, sm_lds, waves_per_cu, resources)
}
pub trait IntoDataSources { fn into_data_sources(self) -> Vec<String>; }
impl IntoDataSources for &str { fn into_data_sources(self) -> Vec<String> { vec![self.to_owned()] } }
impl IntoDataSources for String { fn into_data_sources(self) -> Vec<String> { vec![self] } }
impl<T: Into<String>, const N: usize> IntoDataSources for [T; N] {
	fn into_data_sources(self) -> Vec<String> { self.into_iter().map(Into::into).collect() }
}
impl<T: Into<String>> IntoDataSources for Vec<T> {
	fn into_data_sources(self) -> Vec<String> { self.into_iter().map(Into::into).collect() }
}
impl<T: Clone + Into<String>> IntoDataSources for &[T] {
	fn into_data_sources(self) -> Vec<String> { self.iter().cloned().map(Into::into).collect() }
}
impl Data {
	pub fn target(mut self, target: impl IntoDataSources) -> Self { self.target = target.into_data_sources(); self }
	pub fn r#in(mut self, names: impl IntoDataSources) -> Self {
		self.routes.push(Route { inputs: names.into_data_sources(), outputs: Vec::new() }); self
	}
	pub fn out(mut self, names: impl IntoDataSources) -> Self {
		self.routes.last_mut().unwrap_or_else(|| panic!(".out() requires a preceding .r#in()")).outputs =
			names.into_data_sources(); self
	}
	pub fn exclude(mut self, names: impl IntoDataSources) -> Self { self.exclusions = names.into_data_sources(); self }
	pub fn set(mut self, source: impl Into<String>) -> Self { self.sources.push(source.into()); self }
	pub const fn norm(mut self, _: ZScore) -> Self { self.normalize = true; self }
	pub const fn split(mut self, fraction: f64) -> Self { self.split = fraction; self }
}
struct Prepared {
	samples: Vec<f64>,
	targets: Vec<f64>,
	rows: usize,
	features: usize,
	schema: String,
	sequence: Option<Shape>,
	norm_mean: Vec<f64>,
	norm_scale: Vec<f64>,
	identities: Vec<u64>,
}
struct ChildTable {
	name: String,
	headers: Vec<String>,
	rows: usize,
}
struct Table {
	name: String,
	headers: Vec<String>,
	rows: Vec<Vec<String>>,
	children: Vec<ChildTable>,
}
enum FeatureType {
	Numeric(&'static str),
	Categorical(Vec<String>),
	Text(usize),
}
fn prepare(data: &Data) -> Result<&Prepared> {
	match data.prepared.get_or_init(|| prepare_data(data)) {
		Ok(prepared) => Ok(prepared),
		Err(error) => Err(error.clone()),
	}
}
fn column_match(name: &str, table: &Table, header: &str, column: usize) -> bool {
	name == header
		|| name == format!("{}.{}", table.name, header)
		|| name == format!("col{}", column + 1)
		|| name == format!("{}.col{}", table.name, column + 1)
		|| header.strip_suffix(name).is_some_and(|prefix| prefix.ends_with('.'))
		|| header.rsplit_once('.').is_some_and(|(base, row)| {
			row.parse::<usize>().is_ok()
				&& (base == name || base.strip_suffix(name).is_some_and(|prefix| prefix.ends_with('.')))
		})
}
fn print_table(table: &Table, fit: usize, targets: &[String], exclusions: &[String]) {
	eprintln!("{:<37} {:<10} {:<9} {} samples", table.name, "", "", table.rows.len());
	let mut base = 0;
	for child in &table.children {
		let unit = if child.rows == 1 { "row/sample" } else { "rows/sample" };
		eprintln!("{:<37} {:<10} {:<9} {} {unit}", format!("  {}", child.name), "", "", child.rows);
		for (column, header) in child.headers.iter().enumerate().filter(|_| child.rows != 0) {
			let index = base + column;
			let tagged = |names: &[String]| names.iter().any(|name| column_match(name, table, header, index));
			let tag = if tagged(targets) { "\x1b[32m[target]\x1b[0m  " }
				else if tagged(exclusions) { "\x1b[31m[excluded]\x1b[0m" } else { "          " };
			let kind = infer_feature(table, index, fit).name();
			let same = (1..child.rows).all(|r| infer_feature(table, base + r * child.headers.len() + column, fit).name() == kind);
			let kind = if same { kind } else { "mixed" };
			if child.rows == 1 {
				let n = (0..table.rows.len()).filter(|&s| table.rows[s].get(index).is_some_and(|v| !v.is_empty())).count();
				eprintln!("{:<37} {tag} {kind:<9} {n}", format!("    {header}"));
			} else {
				eprintln!("{:<37} {tag} {kind:<9}", format!("    {header}"));
			}
		}
		base += child.rows * child.headers.len();
	}
}
fn prepare_data(data: &Data) -> Result<Prepared> {
	let mut paths = Vec::new();
	for source in &data.sources {
		collect_files(&expand_home(source)?, &mut paths)?;
	}
	paths.sort();
	paths.dedup();
	let mut grouped = Vec::new();
	for path in paths {
		if !path.extension().and_then(|value| value.to_str()).is_some_and(is_table) {
			continue;
		}
		let bytes = fs::read(&path)
			.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
		let directory = path.parent().unwrap_or_else(|| Path::new("")).to_owned();
		grouped.push((directory, parse_table(&path, &bytes)?));
	}
	let mut tables = merge_captures(grouped, &data.target)?;
	tables = merge_partitions(tables, &data.target, &data.exclusions)?;
	require(!tables.is_empty(), "data source contains no supported table")?;
	let mut selected = Vec::new();
	for name in &data.target {
		let mut matches = Vec::new();
		for (table, value) in tables.iter().enumerate() {
			for (column, header) in value.headers.iter().enumerate() {
				if column_match(name, value, header, column) {
					matches.push((table, column));
				}
			}
		}
		require(matches.len() == 1, format!("target {name:?} must identify exactly one feature"))?;
		selected.push(matches[0]);
	}
	let table_index = selected.first().map_or(0, |target| target.0);
	let row_count = tables[table_index].rows.len();
	require(selected.iter().all(|target| tables[target.0].rows.len() == row_count), "target row counts differ")?;
	eprintln!("Feature name:                                    Dtype:    Samples:");
	for value in &tables {
		print_table(value, row_count, &data.target, &data.exclusions);
	}
	let mut columns = Vec::new();
	for (table, value) in tables.iter().enumerate() {
		if value.rows.len() != row_count {
			eprintln!("excluded: {} ({} rows, target table has {row_count})", value.name, value.rows.len());
		}
		if value.rows.len() == row_count {
			for (column, header) in value.headers.iter().enumerate() {
				let excluded = data.exclusions.iter().any(|name| column_match(name, value, header, column));
				if !selected.contains(&(table, column)) && !excluded {
					columns.push((table, column, infer_feature(value, column, row_count)));
				}
			}
		}
	}
	let features = columns.iter().map(|column| column.2.width()).sum();
	let mut sequence_widths = BTreeMap::new();
	let repeated = columns.iter().all(|column| {
		tables[column.0].headers[column.1]
			.rsplit_once('.')
			.and_then(|value| {
				value.1
					.parse::<usize>()
					.ok()
					.map(|row| *sequence_widths.entry(row).or_insert(0) += column.2.width())
			})
			.is_some()
	});
	let sequence = (repeated
		&& sequence_widths.len() > 1
		&& sequence_widths.keys().copied().eq(1..=sequence_widths.len())
		&& sequence_widths.values().all(|width| *width == sequence_widths[&1]))
	.then(|| Shape { channels: sequence_widths[&1], length: sequence_widths.len() });
	require(features != 0, "dataset has no training features")?;
	let target_categories =
		selected.iter().map(|target| categories(&tables[target.0], target.1, row_count)).collect::<Vec<_>>();
	let mut samples = Vec::new();
	let mut targets = Vec::new();
	let mut missing = vec![0_usize; columns.len()];
	for row in 0..row_count {
		let mut encoded = Vec::with_capacity(features);
		let valid = columns.iter().all(|column| {
			tables[column.0].rows[row].get(column.1).is_some_and(|value| encode(value, &column.2, &mut encoded))
		});
		if valid {
			if let Some(shape) = sequence {
				let mut ordered = Vec::with_capacity(features);
				for channel in 0..shape.channels {
					for position in 0..shape.length {
						ordered.push(encoded[position * shape.channels + channel]);
					}
				}
				encoded = ordered;
			}
		}
		if valid && selected.is_empty() {
			samples.extend_from_slice(&encoded);
			targets.push(0.0);
			for (count, column) in missing.iter_mut().zip(&columns) {
				*count += usize::from(tables[column.0].rows[row][column.1].is_empty());
			}
		} else if valid {
			for (target, categories) in selected.iter().zip(&target_categories) {
				let value = tables[target.0].rows[row].get(target.1);
				let target = value.and_then(|value| value.parse::<f64>().ok()).or_else(|| {
					value.and_then(|value| categories.iter().position(|category| category == value))
						.map(|value| value as f64)
				});
				if let Some(target) = target
					&& target.is_finite()
				{
					samples.extend_from_slice(&encoded);
					targets.push(target);
					for (count, column) in missing.iter_mut().zip(&columns) {
						*count += usize::from(tables[column.0].rows[row][column.1].is_empty());
					}
				}
			}
		}
	}
	let rows = targets.len();
	require(rows != 0, "dataset has no complete training rows")?;
	for (column, count) in columns.iter().zip(missing).filter(|value| value.1 != 0) {
		eprintln!("imputed: {}.{} {count}", tables[column.0].name, tables[column.0].headers[column.1]);
	}
	let mut identities = samples.chunks_exact(features).zip(&targets).enumerate()
		.map(|(row, (sample, target))| sample_identity(sample, *target, row)).collect();
	shuffle(&mut samples, &mut targets, &mut identities, features)?;
	let (norm_mean, norm_scale) = if data.normalize {
		normalize_samples(&mut samples, features, ((rows as f64) * data.split).floor() as usize)?
	} else {
		impute_missing(&mut samples);
		(Vec::new(), Vec::new())
	};
	let schema = columns
		.iter()
		.map(|column| {
			format!("{}.{}:{}", tables[column.0].name, tables[column.0].headers[column.1], column.2.width())
		})
		.collect::<Vec<_>>()
		.join("|") + "->"
		+ &data.target.join("|");
	Ok(Prepared { samples, targets, rows, features, schema, sequence, norm_mean, norm_scale, identities })
}
fn normalize_samples(samples: &mut [f64], features: usize, fit: usize) -> Result<(Vec<f64>, Vec<f64>)> {
	require(fit != 0, "split must retain normalization rows")?;
	let epsilon = number("normalization epsilon", env!("RECIPE_NORMALIZATION_EPSILON"))?;
	let (mut means, mut scales) = (Vec::with_capacity(features), Vec::with_capacity(features));
	for column in 0..features {
		let valid = (0..fit).filter(|&row| samples[row * features + column].is_finite()).collect::<Vec<_>>();
		let count = valid.len().max(1) as f64;
		let mean = valid.iter().map(|&row| samples[row * features + column]).sum::<f64>() / count;
		let variance = valid.iter().map(|&row| (samples[row * features + column] - mean).powi(2)).sum::<f64>() / count;
		let scale = (variance + epsilon).sqrt();
		for row in 0..samples.len() / features {
			let value = &mut samples[row * features + column];
			*value = (if value.is_finite() { *value } else { mean } - mean) / scale;
		}
		means.push(mean);
		scales.push(scale);
	}
	Ok((means, scales))
}
fn impute_missing(samples: &mut [f64]) {
	for value in samples.iter_mut() { if !value.is_finite() { *value = 0.0 } }
}
fn sample_identity(sample: &[f64], target: f64, row: usize) -> u64 {
	const OFFSET: u64 = 14695981039346656037;
	const PRIME: u64 = 1099511628211;
	let hash = sample.iter().copied().chain(std::iter::once(target))
		.fold(OFFSET, |hash, value| (hash ^ value.to_bits()).wrapping_mul(PRIME));
	(hash ^ row as u64).wrapping_mul(PRIME)
}
fn is_table(extension: &str) -> bool {
	matches!(extension.to_ascii_lowercase().as_str(), "csv" | "tsv" | "txt")
}
fn expand_home(source: &str) -> Result<PathBuf> {
	if source == "~" || source.starts_with("~/") {
		let home = std::env::var_os("HOME").ok_or_else(|| RecipeError::new("HOME is absent"))?;
		return Ok(PathBuf::from(home).join(source.trim_start_matches("~/")));
	}
	Ok(PathBuf::from(source))
}
fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
	let metadata = fs::metadata(path)
		.map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", path.display())))?;
	if metadata.is_file() {
		files.push(path.to_owned());
		return Ok(());
	}
	let mut children = fs::read_dir(path)
		.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?
		.collect::<std::io::Result<Vec<_>>>()
		.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
	children.sort_by_key(fs::DirEntry::path);
	for child in children {
		collect_files(&child.path(), files)?;
	}
	Ok(())
}
fn target_column(table: &Table, name: &str) -> Option<usize> {
	table.headers.iter().enumerate().position(|(column, header)| column_match(name, table, header, column))
}
fn merge_captures(tables: Vec<(PathBuf, Table)>, targets: &[String]) -> Result<Vec<Table>> {
	let mut groups = BTreeMap::<PathBuf, Vec<Table>>::new();
	for (directory, table) in tables {
		groups.entry(directory).or_default().push(table);
	}
	let valid = |group: &[Table]| {
		group.len() > 1
			&& targets.iter().all(|target| {
				group.iter().filter(|table| target_column(table, target).is_some()).count() == 1
					&& group
						.iter()
						.find(|table| target_column(table, target).is_some())
						.is_some_and(|table| table.rows.len() == 1)
			})
	};
	if targets.is_empty() || groups.values().filter(|group| valid(group)).count() < 2 {
		let mut tables = Vec::new();
		for mut group in groups.into_values() {
			let rows = group.iter().map(|table| table.rows.len()).max().unwrap_or(0);
			require(rows != 0, "table group has no samples")?;
			for table in &mut group {
				let count = table.rows.len();
				require(count != 0 && rows % count == 0, "table rows do not broadcast")?;
				if count != rows {
					eprintln!("aligned: {} ({count} rows cycled to {rows})", table.name);
					table.rows = table.rows.iter().cloned().cycle().take(rows).collect();
				}
			}
			tables.extend(group);
		}
		return Ok(tables);
	}
	let mut captures = groups.into_values().filter(|group| valid(group)).collect::<Vec<_>>();
	let key = |table: &Table| (table.headers.join("\0"), table.rows.len());
	for capture in &mut captures {
		capture.sort_by_key(&key);
	}
	let schemas = captures[0].iter().map(|table| (table.headers.clone(), table.rows.len())).collect::<Vec<_>>();
	for capture in &captures {
		require(capture.len() == schemas.len(), "capture table counts differ")?;
		require(
			capture
				.iter()
				.zip(&schemas)
				.all(|(table, schema)| table.headers == schema.0 && table.rows.len() == schema.1),
			"capture table schemas differ",
		)?;
	}
	let names = (0..schemas.len())
		.map(|index| {
			let name = &captures[0][index].name;
			if captures.iter().all(|capture| capture[index].name == *name) {
				name.clone()
			} else {
				format!("table{}", index + 1)
			}
		})
		.collect::<Vec<_>>();
	let children = names
		.iter()
		.zip(&schemas)
		.map(|(name, schema)| ChildTable { name: name.clone(), headers: schema.0.clone(), rows: schema.1 })
		.collect();
	let mut headers = Vec::new();
	for (table, name) in captures[0].iter().zip(&names) {
		for row in 0..table.rows.len() {
			for header in &table.headers {
				if targets.contains(header) {
					headers.push(header.clone());
				} else if table.rows.len() == 1 {
					headers.push(format!("{name}.{header}"));
				} else {
					headers.push(format!("{name}.{header}.{}", row + 1));
				}
			}
		}
	}
	let mut rows = Vec::with_capacity(captures.len());
	for capture in captures {
		let row = capture.into_iter().flat_map(|table| table.rows.into_iter().flatten()).collect::<Vec<_>>();
		require(row.len() == headers.len(), "capture value width differs")?;
		rows.push(row);
	}
	Ok(vec![Table { name: "data".to_owned(), headers, rows, children }])
}
fn merge_partitions(mut tables: Vec<Table>, targets: &[String], exclusions: &[String]) -> Result<Vec<Table>> {
	if targets.is_empty() || targets.iter().any(|target| target.contains('.')) {
		return Ok(tables);
	}
	let members = tables
		.iter()
		.enumerate()
		.filter_map(|(index, table)| {
			targets.iter().all(|target| target_column(table, target).is_some()).then_some(index)
		})
		.collect::<Vec<_>>();
	if members.len() < 2 {
		return Ok(tables);
	}
	let mut headers = Vec::new();
	for &index in &members {
		for header in &tables[index].headers {
			if !headers.contains(header) {
				headers.push(header.clone())
			}
		}
	}
	let union = Table { name: "data".to_owned(), headers: headers.clone(), rows: Vec::new(), children: Vec::new() };
	for &index in &members {
		for (column, header) in headers.iter().enumerate() {
			let ignored = targets.iter().chain(exclusions).any(|name| column_match(name, &union, header, column));
			require(ignored || tables[index].headers.contains(header),
				format!("feature {header:?} is absent from partition {:?}", tables[index].name))?;
		}
	}
	let mut rows = Vec::new();
	for index in members {
		let positions = tables[index]
			.headers
			.iter()
			.map(|header| headers.iter().position(|value| value == header).unwrap())
			.collect::<Vec<_>>();
		for row in std::mem::take(&mut tables[index].rows) {
			let mut merged = std::iter::repeat_with(String::new).take(headers.len()).collect::<Vec<_>>();
			for (column, value) in row.into_iter().enumerate() {
				merged[positions[column]] = value;
			}
			rows.push(merged);
		}
	}
	let name = "data".to_owned();
	let children = vec![ChildTable { name: name.clone(), headers: headers.clone(), rows: 1 }];
	Ok(vec![Table { name, headers, rows, children }])
}
fn parse_table(path: &Path, bytes: &[u8]) -> Result<Table> {
	let first = bytes.split(|byte| *byte == b'\n').next().unwrap_or_default();
	let delimiter = [b',', b';', b'\t']
		.into_iter()
		.max_by_key(|delimiter| first.iter().filter(|byte| *byte == delimiter).count())
		.unwrap_or(b',');
	let mut rows = records(bytes, delimiter)?;
	require(!rows.is_empty(), format!("dataset {} is empty", path.display()))?;
	let first = rows.remove(0);
	let headerless = first.iter().all(|value| value.parse::<f64>().is_ok());
	let headers =
		if headerless { (1..=first.len()).map(|column| format!("col{column}")).collect() } else { first.clone() };
	if headerless {
		rows.insert(0, first);
	}
	let width = headers.len();
	rows.retain(|row| row.len() == width);
	let name = path.file_stem().and_then(|value| value.to_str()).unwrap_or("data").to_owned();
	let children = vec![ChildTable { name: name.clone(), headers: headers.clone(), rows: 1 }];
	Ok(Table { name, headers, rows, children })
}
fn records(bytes: &[u8], delimiter: u8) -> Result<Vec<Vec<String>>> {
	let (mut rows, mut row, mut field, mut quoted) = (Vec::new(), Vec::new(), Vec::new(), false);
	let mut index = 0;
	while index < bytes.len() {
		let byte = bytes[index];
		if byte == b'"' {
			if quoted && bytes.get(index + 1) == Some(&b'"') {
				field.push(byte);
				index += 1;
			} else {
				quoted = !quoted;
			}
		} else if byte == delimiter && !quoted {
			row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
			field = Vec::new();
		} else if byte == b'\n' && !quoted {
			let value = String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?;
			row.push(value.trim_end_matches('\r').to_owned());
			field = Vec::new();
			if row.iter().any(|value| !value.is_empty()) {
				rows.push(row);
			}
			row = Vec::new();
		} else {
			field.push(byte);
		}
		index += 1;
	}
	require(!quoted, "unterminated quoted feature")?;
	if !field.is_empty() || !row.is_empty() {
		row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
		rows.push(row);
	}
	Ok(rows)
}
fn categories(table: &Table, column: usize, rows: usize) -> Vec<String> {
	table.rows
		.iter()
		.take(rows)
		.filter_map(|row| row.get(column))
		.filter(|value| !value.is_empty())
		.cloned()
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect()
}
fn infer_feature(table: &Table, column: usize, rows: usize) -> FeatureType {
	let values = table
		.rows
		.iter()
		.take(rows)
		.filter_map(|row| row.get(column))
		.filter(|value| !value.is_empty())
		.collect::<Vec<_>>();
	if !values.is_empty() && values.iter().all(|value| value.parse::<f64>().is_ok()) {
		return FeatureType::Numeric("f64");
	}
	let categories = categories(table, column, rows);
	if categories.len() < values.len() {
		FeatureType::Categorical(categories)
	} else {
		FeatureType::Text(values.iter().map(|value| value.len()).max().unwrap_or(0))
	}
}
impl FeatureType {
	const fn name(&self) -> &'static str {
		match self {
			Self::Numeric(name) => name,
			Self::Categorical(_) => "categoric",
			Self::Text(_) => "string",
		}
	}
	fn width(&self) -> usize {
		match self {
			Self::Numeric(_) => 1,
			Self::Categorical(values) => values.len(),
			Self::Text(width) => *width,
		}
	}
}
fn encode(value: &str, kind: &FeatureType, output: &mut Vec<f64>) -> bool {
	if value.is_empty() {
		output.resize(output.len() + kind.width(), f64::NAN);
		return true;
	}
	match kind {
		FeatureType::Numeric(_) => value.parse::<f64>().is_ok_and(|value| {
			output.push(value);
			value.is_finite()
		}),
		FeatureType::Categorical(categories) => {
			let found = categories.iter().position(|category| category == value);
			output.extend((0..categories.len()).map(|index| f64::from(found == Some(index))));
			found.is_some()
		}
		FeatureType::Text(width) => {
			output.extend(value.bytes().map(f64::from).chain(std::iter::repeat(0.0)).take(*width));
			value.len() <= *width
		}
	}
}
fn shuffle(samples: &mut Vec<f64>, targets: &mut Vec<f64>, identities: &mut Vec<u64>, features: usize) -> Result<()> {
	let mut seed = env!("RECIPE_RANDOM_SEED")
		.parse::<u64>()
		.map_err(|error| RecipeError::new(format!("invalid random seed: {error}")))?;
	let mut order = (0..targets.len()).collect::<Vec<_>>();
	for index in (1..order.len()).rev() {
		seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
		order.swap(index, (seed as usize) % (index + 1));
	}
	let old_samples = std::mem::take(samples);
	let old_targets = std::mem::take(targets);
	let old_identities = std::mem::take(identities);
	for row in order {
		samples.extend_from_slice(&old_samples[row * features..(row + 1) * features]);
		targets.push(old_targets[row]);
		identities.push(old_identities[row]);
	}
	Ok(())
}
pub struct Train {
	epochs: usize,
	learning_rate: f64,
	log_metrics: Vec<Metric>,
	stop: Option<f64>,
	resume: Option<String>,
	save: Option<String>,
	seed: Option<usize>,
}
impl Train {
	pub const fn seed(mut self, value: usize) -> Self { self.seed = Some(value); self }
	pub const fn stop(mut self, value: f64) -> Self { self.stop = Some(value); self }
	pub const fn optimizer(self, _: Adamw) -> Self { self }
	pub const fn epochs(mut self, value: usize) -> Self { self.epochs = value; self }
	pub const fn lr(mut self, value: f64) -> Self { self.learning_rate = value; self
	}
	pub fn log<const N: usize>(mut self, metrics: [Metric; N]) -> Self { self.log_metrics = metrics.into(); self }
	pub fn save(mut self, path: impl Into<String>) -> Self { self.save = Some(path.into()); self }
	pub fn resume(mut self, path: impl Into<String>) -> Self { self.resume = Some(path.into()); self }
	pub fn run(&self, model: &Model, data: &Data) -> TrainingReport {
		SIGNAL.get_or_init(|| unsafe { signal(SIGINT, interrupt) });
		if INTERRUPTED.load(Ordering::Acquire) {
			std::process::exit(INTERRUPTED_EXIT);
		}
		self.try_run(model, data).unwrap_or_else(|error| panic!("{error}"))
	}
	fn try_run(&self, model: &Model, data: &Data) -> Result<TrainingReport> {
		drop(topology()?);
		let (gpus, mut config) = (all_gpus()?, Config::load()?);
		if let Some(seed) = self.seed {
			config.random_seed = seed;
		}
		require(model.downstream.is_none(), "model-valued loss requires .rat()")?;
		let prepared = prepare(data)?;
		let training_rows = ((prepared.rows as f64) * data.split).floor() as usize;
		require(training_rows != 0 && training_rows <= prepared.rows, "split must select training rows")?;
		let probability = model.loss.0 >= 4;
		let scale = probability.then(|| TargetScale::fit(&prepared.targets[..training_rows]));
		let target_values = prepared
			.targets
			.iter()
			.map(|target| scale.map_or(*target, |scale| scale.encode(*target)))
			.collect::<Vec<_>>();
		let (run, mut graph) =
			(RUN.fetch_add(1, Ordering::Relaxed) + 1, compile(model, prepared, training_rows, gpus[0], config)?);
		graph.state.training_rows = training_rows;
		if let Some(scale) = scale
			&& let Some(offset) = output_bias_offset(&graph)
		{
			let mean = target_values[..training_rows].iter().sum::<f64>() / training_rows as f64;
			graph.parameters[offset] = scale.logit(mean);
		}
		let mut stored = stored_graph(&graph, data, scale);
		require(stored.graph.output.elements() == 1, "model output width must be one")?;
		if let Some(path) = &self.resume {
			bundle::restore(path, &prepared.schema, std::slice::from_mut(&mut stored), &prepared.identities)?;
			eprintln!("resumed: {path}");
		}
		stored.graph.state.trained_samples.extend_from_slice(&prepared.identities[..training_rows]);
		stored.graph.state.trained_samples.sort_unstable();
		stored.graph.state.trained_samples.dedup();
		let (samples, targets) =
			(&prepared.samples[..training_rows * prepared.features], &target_values[..training_rows]);
		let (mut tape, mut runtime) =
			(DeviceTape::new(&stored.graph, samples, targets, &gpus)?, tile_runtime(&gpus, config)?);
		self.finish_dispatch(tape.forward(), &mut stored, &prepared.schema, &tape, None)?;
		let initial_predictions = tape.predictions()?;
		let initial_loss = model_loss(&initial_predictions, targets, model.loss, config.activation[7]);
		let tolerance = self.stop.unwrap_or(0.0);
		require(tolerance.is_finite() && (0.0..=1.0).contains(&tolerance), "stop must be between zero and one")?;
		for _ in 0..self.epochs {
			if INTERRUPTED.load(Ordering::Acquire) {
				self.finish_dispatch::<()>(Err(RecipeError::new("interrupted")), &mut stored, &prepared.schema, &tape, None).ok();
				break;
			}
			let proposed = runtime.propose(&stored.graph, &mut tape);
			let cases = self.finish_dispatch(proposed, &mut stored, &prepared.schema, &tape, None)?;
			let started = Instant::now();
			let dispatched = tape
				.advance()
				.and_then(|_| tape.epoch(self.learning_rate, model.loss, tolerance, config, false));
			let (loss, saved) = self.finish_dispatch(dispatched, &mut stored, &prepared.schema, &tape, None)?;
			self.finish_dispatch(Ok(()), &mut stored, &prepared.schema, &tape, saved.then_some(true))?;
			let (predictions, seconds) = (tape.predictions()?, started.elapsed().as_secs_f64());
			let learned = runtime.learn(&cases, &tape, config);
			self.finish_dispatch(learned, &mut stored, &prepared.schema, &tape, None)?;
			self.print(model, run, tape.step as usize, loss, targets, &predictions, seconds, saved);
		}
		if self.stop.is_some() {
			tape.restore_best()?;
		}
		self.finish_dispatch(tape.forward(), &mut stored, &prepared.schema, &tape, None)?;
		stored.bn_stats = extract_bn_stats(&stored.graph, &tape.shards[0].tape.contexts)?;
		let raw_predictions = tape.predictions()?;
		let final_loss = model_loss(&raw_predictions, targets, model.loss, config.activation[7]);
		let predictions = raw_predictions
			.iter()
			.map(|value| scale.map_or(*value, |scale| scale.decode(*value)))
			.collect::<Vec<_>>();
		let r2 = if training_rows == prepared.rows {
			coefficient(&prepared.targets, &predictions)
		} else {
			let mut graph = stored.graph.clone();
			graph.parameters = tape.weights()?;
			let (start, validation_targets) =
				(training_rows * prepared.features, &target_values[training_rows..]);
			let mut validation = DeviceTape::new(&graph, &prepared.samples[start..], validation_targets, &gpus)?;
			validation
				.forward()
				.and_then(|_| validation.predictions())
				.map(|values| {
					values.into_iter()
						.map(|value| scale.map_or(value, |scale| scale.decode(value)))
						.collect::<Vec<_>>()
				})
				.map(|values| coefficient(&prepared.targets[training_rows..], &values))?
		};
		if training_rows < prepared.rows {
			eprintln!("Evaluation: r2 {r2:.4}");
		}
		self.finish_dispatch(Ok(()), &mut stored, &prepared.schema, &tape, Some(self.stop.is_some()))?;
		Ok(TrainingReport(initial_loss, final_loss, initial_predictions, predictions, r2, tape.tile()))
	}
	fn finish_dispatch<T>(
		&self,
		result: Result<T>,
		stored: &mut bundle::StoredGraph,
		schema: &str,
		tape: &DeviceTape,
		save: Option<bool>,
	) -> Result<T> {
		let interrupted = INTERRUPTED.load(Ordering::Acquire);
		let save = if interrupted { Some(self.stop.is_some()) } else { save };
		if let Some(best) = save
			&& let Some(path) = &self.save
		{
			checkpoint(path, schema, stored, tape, best)?;
		}
		if interrupted {
			std::process::exit(INTERRUPTED_EXIT)
		}
		result
	}
	fn print(
		&self,
		model: &Model,
		run: u64,
		epoch: usize,
		loss: f64,
		targets: &[f64],
		predictions: &[f64],
		seconds: f64,
		checkpoint: bool,
	) {
		if self.log_metrics.is_empty() {
			return;
		}
		let topology = model.description(&self.log_metrics);
		let r2 = coefficient(targets, predictions);
		let time = seconds * 1000.0;
		let mut values = Vec::new();
		let mut topology_printed = false;
		for metric in &self.log_metrics {
			let value = match metric.0 {
				0 => format!("run \x1b[38\x3b2\x3b242\x3b40\x3b60m{run:>5}\x1b[0m"),
				1 => format!("{} \x1b[38\x3b2\x3b0\x3b174\x3b107m{loss:.4}\x1b[0m", model.loss.name()),
				2 => format!("r2 \x1b[38\x3b2\x3b39\x3b125\x3b255m{r2:>7.4}\x1b[0m"),
				3 => format!("time \x1b[38\x3b2\x3b255\x3b194\x3b0m{time:>9.3} ms\x1b[0m"),
				4 => format!("epoch \x1b[38\x3b2\x3b135\x3b90\x3b251m{epoch}\x1b[0m"),
				5..=7 if !topology_printed && !topology.is_empty() => {
					topology_printed = true;
					topology.clone()
				}
				5..=7 => continue,
				_ => unreachable!(),
			};
			values.push(value);
		}
		if checkpoint && self.stop.is_some() {
			values.push("\x1b[1\x3b32m← checkpoint\x1b[0m".to_owned());
		}
		eprintln!("{}", values.join("  "));
	}
}
pub struct TrainingReport(f64, f64, Vec<f64>, Vec<f64>, f64, Tile);
impl TrainingReport {
	pub const fn initial_loss(&self) -> f64 { self.0 }
	pub const fn final_loss(&self) -> f64 { self.1 }
	pub fn initial_predictions(&self) -> &[f64] { &self.2 }
	pub fn predictions(&self) -> &[f64] { &self.3 }
	pub const fn r2(&self) -> f64 { self.4 }
	pub const fn tile(&self) -> [u32; 3] { [self.5.m, self.5.n, self.5.k] }
}
#[derive(Clone, Copy)]
struct TargetScale {
	minimum: f64,
	span: f64,
}
impl TargetScale {
	fn fit(targets: &[f64]) -> Self {
		let minimum = targets.iter().copied().fold(f64::INFINITY, f64::min);
		let maximum = targets.iter().copied().fold(f64::NEG_INFINITY, f64::max);
		Self { minimum, span: maximum - minimum }
	}
	fn encode(self, value: f64) -> f64 {
		(value - self.minimum) / self.span
	}
	fn decode(self, value: f64) -> f64 {
		self.minimum + self.span * logistic(value)
	}
	fn logit(self, value: f64) -> f64 {
		let value = value.clamp(f64::EPSILON, 1.0 - f64::EPSILON);
		(value / (1.0 - value)).ln()
	}
}
fn model_loss(predictions: &[f64], targets: &[f64], loss: LossFunction, threshold: f64) -> f64 {
	let values = predictions.iter().zip(targets);
	let mut result = values.map(|(prediction, target)| loss.value(*prediction, *target, threshold)).sum::<f64>()
		/ targets.len() as f64;
	if loss.0 == 1 {
		result = result.sqrt();
	}
	result
}
fn coefficient(targets: &[f64], predictions: &[f64]) -> f64 {
	let mean = targets.iter().sum::<f64>() / targets.len() as f64;
	let residual = targets.iter().zip(predictions).map(|(target, value)| (target - value).powi(2)).sum::<f64>();
	let total = targets.iter().map(|target| (target - mean).powi(2)).sum::<f64>();
	if total == 0.0 { 0.0 } else { 1.0 - residual / total }
}
use std::{
	io::{self, BufRead, BufReader, Read, Write},
	process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
fn process_io<T>(action: &str, result: io::Result<T>) -> Result<T> {
	result.map_err(|error| RecipeError::new(format!("cannot {action} RAT command: {error}")))
}
struct Frame(Vec<(String, f64)>);
impl Frame {
	fn value(&self, name: &str) -> Result<f64> {
		self.0.iter()
			.find(|value| value.0 == name)
			.map(|value| value.1)
			.ok_or_else(|| RecipeError::new(format!("RAT value {name:?} is absent")))
	}
	fn values(&self, names: &[String]) -> Result<Vec<f64>> {
		names.iter().map(|name| self.value(name)).collect()
	}
}
struct Process {
	child: Child,
	input: Option<ChildStdin>,
	output: BufReader<ChildStdout>,
}
impl Process {
	fn spawn(command: &str) -> Result<Self> {
		require(!command.trim().is_empty(), ".every() requires a command")?;
		let mut child = Command::new(command)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.map_err(|error| RecipeError::new(format!("cannot start {command:?}: {error}")))?;
		let input = child.stdin.take().ok_or_else(|| RecipeError::new("RAT command stdin is absent"))?;
		let output = child.stdout.take().ok_or_else(|| RecipeError::new("RAT command stdout is absent"))?;
		Ok(Self { child, input: Some(input), output: BufReader::new(output) })
	}
	fn read(&mut self, rows: usize) -> Result<Vec<Frame>> {
		let mut frames = Vec::with_capacity(rows);
		for _ in 0..rows {
			let mut values = Vec::new();
			loop {
				let mut line = String::new();
				let bytes = process_io("read", self.output.read_line(&mut line))?;
				require(bytes != 0, "RAT command exited before a blank-line frame terminator")?;
				let line = line.trim();
				if line.is_empty() {
					require(!values.is_empty(), "RAT command returned an empty frame")?;
					break;
				}
				let Some((name, value)) = line.split_once(char::is_whitespace) else { continue };
				let value = value
					.trim()
					.parse::<f64>()
					.map_err(|error| RecipeError::new(format!("RAT value {name:?} is invalid: {error}")))?;
				require(value.is_finite(), format!("RAT value {name:?} must be finite"))?;
				require(
					!values.iter().any(|item: &(String, f64)| item.0 == name),
					format!("RAT value {name:?} is duplicated"),
				)?;
				values.push((name.to_owned(), value));
			}
			frames.push(Frame(values));
		}
		Ok(frames)
	}
	fn write(&mut self, names: &[String], values: &[f64]) -> Result<()> {
		require(!names.is_empty() && values.len() % names.len() == 0, "RAT proposal batch has the wrong width")?;
		let input = self.input.as_mut().ok_or_else(|| RecipeError::new("RAT command stdin is closed"))?;
		for row in values.chunks_exact(names.len()) {
			process_io("write", writeln!(input, "proposal"))?;
			for (name, value) in names.iter().zip(row) {
				process_io("write", writeln!(input, "    {name} {value}"))?;
			}
			process_io("write", writeln!(input))?;
		}
		process_io("flush", input.flush())
	}
}
impl Drop for Process {
	fn drop(&mut self) {
		drop(self.input.take());
		let _ = self.child.wait();
	}
}
struct State {
	stored: bundle::StoredGraph,
	tape: DeviceTape,
	proposals: Vec<(usize, usize)>,
	models: Vec<(usize, usize)>,
	proposal_names: Vec<String>,
	targets: Vec<f64>,
	schema: String,
}
pub struct RatTrain<const N: usize> {
	train: Train,
	models: [Model; N],
	command: Option<String>,
	process: Option<Process>,
	context: Option<Vec<Frame>>,
	state: Option<State>,
}
pub struct RatReport {
	proposal: Vec<f64>,
	prediction: Vec<f64>,
	measurement: Vec<f64>,
}
impl RatReport {
	pub fn proposal(&self) -> &[f64] { &self.proposal }
	pub fn prediction(&self) -> &[f64] { &self.prediction }
	pub fn measurement(&self) -> &[f64] { &self.measurement }
}
fn rat<const N: usize>(train: Train, models: [Model; N]) -> RatTrain<N> {
	RatTrain { train, models, command: None, process: None, context: None, state: None }
}
impl Train {
	pub fn rat(self, proposer: Model, predictor: Model) -> RatTrain<2> {
		rat(self, [proposer, predictor])
	}
	pub fn rats<const N: usize>(self, models: [Model; N]) -> RatTrain<N> {
		rat(self, models)
	}
}
fn rat_schema(data: &Data) -> String {
	data.routes
		.iter()
		.map(|route| format!("{}->{}", route.inputs.join("|"), route.outputs.join("|")))
		.chain(std::iter::once(format!("target->{}", data.target.join("|"))))
		.collect::<Vec<_>>()
		.join("/")
}
fn append_model(
	graph: &mut Graph,
	model: &Model,
	features: usize,
	outputs: usize,
	rows: usize,
	gpu: &'static Gpu,
	config: Config,
	schema: &str,
) -> Result<(i32, (usize, usize))> {
	let prepared = Prepared {
		samples: vec![0.0; rows * features],
		targets: vec![0.0; rows],
		rows,
		features,
		schema: schema.to_owned(),
		sequence: None,
		norm_mean: Vec::new(),
		norm_scale: Vec::new(),
		identities: Vec::new(),
	};
	let part = compile_output(model, &prepared, rows, gpu, config, outputs)?;
	let start = graph.parameters.len();
	let source = append_graph(graph, part)?;
	Ok((source, (start, graph.parameters.len())))
}
fn build<const N: usize>(
	models: &[Model; N],
	train: &Train,
	data: &Data,
	gpus: &[&'static Gpu],
	config: Config,
) -> Result<State> {
	require(N >= 2, "RAT requires an intermediate model and a predictor")?;
	require(data.routes.len() + 1 == N, "RAT requires one .r#in().out() pair per intermediate model")?;
	require(!data.target.is_empty(), "RAT requires .target()")?;
	let (rows, input_names) = (
		config.rat_batch,
		require(!data.routes[0].inputs.is_empty(), "RAT requires an initial input")
			.map(|_| data.routes[0].inputs.clone())?,
	);
	let input = Shape { channels: input_names.len(), length: 1 };
	let mut graph = Graph::new(input);
	let mut fields = Vec::new();
	for (index, name) in input_names.iter().cloned().enumerate() {
		require(!fields.iter().any(|value: &(String, Field)| value.0 == name), "RAT input names must be unique")?;
		fields.push((name, Field { source: -1, stride: input_names.len(), index }));
	}
	let schema = rat_schema(data);
	let (mut proposals, mut ranges, mut proposal_names) = (Vec::new(), Vec::new(), Vec::new());
	for (index, route) in data.routes.iter().enumerate() {
		if let Some(downstream) = &models[index].downstream {
			require(downstream == &models[index + 1].blocks, "model-valued loss must name the next RAT model")?;
		}
		require(!route.inputs.is_empty() && !route.outputs.is_empty(), "RAT route names must not be empty")?;
		route_graph(&mut graph, &route.inputs, &fields, data.normalize)?;
		let (mut source, range) = append_model(
			&mut graph,
			&models[index],
			route.inputs.len(),
			route.outputs.len(),
			rows,
			gpus[0],
			config,
			&schema,
		)?;
		if index == 0 {
			lower_activation(&mut graph, Activation::Sigmoid, config)?;
			source = graph.source;
		}
		proposals.push((source as usize, route.outputs.len()));
		ranges.push(range);
		for (field_index, name) in route.outputs.iter().cloned().enumerate() {
			require(!fields.iter().any(|value| value.0 == name), format!("RAT output {name:?} is duplicated"))?;
			fields.push((name.clone(), Field { source, stride: route.outputs.len(), index: field_index }));
			proposal_names.push(name);
		}
	}
	require(proposal_names.len() >= 3, "RAT requires M, N, and K proposal outputs")?;
	let route = data.routes.last().ok_or_else(|| RecipeError::new("RAT route is absent"))?;
	require(models[N - 1].downstream.is_none(), "the final RAT model requires a scalar loss")?;
	let mut predictor_inputs = route.inputs.clone();
	predictor_inputs.extend(route.outputs.iter().cloned());
	route_graph(&mut graph, &predictor_inputs, &fields, data.normalize)?;
	let (_, range) = append_model(
		&mut graph,
		&models[N - 1],
		predictor_inputs.len(),
		data.target.len(),
		rows,
		gpus[0],
		config,
		&schema,
	)?;
	ranges.push(range);
	let mut stored = bundle::StoredGraph {
		graph,
		inputs: input_names,
		outputs: data.target.clone(),
		norm_mean: Vec::new(), norm_scale: Vec::new(),
		target_min: 0.0, target_span: 0.0, bn_stats: Vec::new(),
	};
	if let Some(path) = &train.resume {
		bundle::restore(path, &schema, std::slice::from_mut(&mut stored), &[])?;
		eprintln!("resumed: {path}");
	}
	let targets = vec![0.0; rows * stored.outputs.len()];
	let tape = DeviceTape::new(
		&stored.graph,
		&vec![0.0; rows * stored.inputs.len()],
		&targets,
		gpus,
	)?;
	Ok(State { stored, tape, proposals, models: ranges, proposal_names, targets, schema })
}
type TileCase = (usize, usize, [f64; 5], Tile, Tile);
struct TileRuntime {
	state: State,
	train: Train,
	predictor: Model,
}
static TILE_RUNTIME: OnceLock<Result<Mutex<TileRuntime>>> = OnceLock::new();
impl TileRuntime {
	fn new(gpus: &[&'static Gpu], config: Config) -> Result<Self> {
		let predictor = recipe.model().layer(config.surrogate_width).tanh().layer(1).loss(mse);
		let proposer = recipe.model().layer(config.surrogate_width).tanh().layer(3).loss(&predictor);
		let models = [proposer, predictor];
		let data = recipe
			.data(Vec::<String>::new())
			.r#in(["dtype", "VRAM", "M", "N", "K"])
			.out(["m", "n", "k"])
			.target(["Mtime"])
			.norm(z_score);
		let train = recipe.train().epochs(config.surrogate_epochs).lr(config.surrogate_rate);
		Ok(Self {
			state: build(&models, &train, &data, gpus, config)?,
			train,
			predictor: models.into_iter().last().unwrap(),
		})
	}
	fn forward(&mut self, cases: &[TileCase]) -> Result<()> {
		let samples =
			(0..self.state.tape.capacity).flat_map(|index| cases[index % cases.len()].2).collect::<Vec<_>>();
		self.state.tape.write_samples(&samples).and_then(|_| self.state.tape.forward())
	}
	#[allow(dead_code, reason = "reserved for the public inference executor")]
	fn infer(&mut self, graph: &Graph, tape: &mut DeviceTape, config: Config) -> Result<Vec<f64>> {
		let cases = self.propose(graph, tape)?;
		tape.forward()?;
		self.learn(&cases, tape, config)?;
		tape.predictions()
	}
	fn propose(&mut self, graph: &Graph, tape: &mut DeviceTape) -> Result<Vec<TileCase>> {
		let mut cases = Vec::new();
		for (gpu, shard) in tape.shards.iter().enumerate() {
			for (node, operation) in graph.nodes.iter().enumerate() {
				if let Some(dimensions) = operation.tile_dimensions() {
					let limit = shard.tape.gpu.proposal_limit(dimensions)?;
					let minimum = Tile {
						m: shard.tape.tile.m.min(limit.m), n: shard.tape.tile.n.min(limit.n),
						k: shard.tape.tile.k.min(limit.k),
					};
					let work = dimensions[0] * f64::from(shard.tape.rows);
					cases.push((
						gpu,
						node,
						[
							size_of::<f64>() as f64 * f64::from(u8::BITS),
							shard.tape.gpu.memory as f64,
							work,
							dimensions[1],
							dimensions[2],
						],
						minimum,
						limit,
					));
				}
			}
		}
		let mut tiles = tape
			.shards
			.iter()
			.map(|shard| std::iter::repeat_n(shard.tape.tile, graph.nodes.len()).collect::<Vec<_>>())
			.collect::<Vec<_>>();
		for chunk in cases.chunks(self.state.tape.capacity) {
			let mut proposal = self.forward(chunk).and_then(|_| self.state.proposal())?;
			for (case, row) in chunk.iter().zip(proposal.chunks_exact_mut(self.state.proposal_names.len())) {
				tiles[case.0][case.1] = Tile::proposed(row, case.3, case.4)?;
			}
		}
		for (shard, values) in tape.shards.iter_mut().zip(tiles) {
			shard.tape.write_tiles(&values)?;
		}
		Ok(cases)
	}
	fn learn(&mut self, cases: &[TileCase], tape: &DeviceTape, config: Config) -> Result<()> {
		let timings = tape.shards.iter().map(|shard| shard.tape.timings()).collect::<Result<Vec<_>>>()?;
		for chunk in cases.chunks(self.state.tape.capacity) {
			self.forward(chunk)?;
			let elapsed = chunk
				.iter()
				.map(|case| {
					let gpu = tape.shards[case.0].tape.gpu;
					timings[case.0][case.1] as f64 * 1_000.0 / gpu.clock as f64
				})
				.collect::<Vec<_>>();
			let measurement =
				(0..self.state.tape.capacity).map(|index| elapsed[index % elapsed.len()]).collect::<Vec<_>>();
			train_rat(&mut self.state, &self.train, &self.predictor, &measurement, config)?;
		}
		Ok(())
	}
}
fn tile_runtime(gpus: &[&'static Gpu], config: Config) -> Result<std::sync::MutexGuard<'static, TileRuntime>> {
	TILE_RUNTIME
		.get_or_init(|| TileRuntime::new(gpus, config).map(Mutex::new))
		.as_ref()
		.map_err(Clone::clone)?
		.lock()
		.map_err(|_| RecipeError::new("tile runtime lock is poisoned"))
}
impl State {
	fn proposal(&self) -> Result<Vec<f64>> {
		let blocks = self
			.proposals
			.iter()
			.map(|&(node, width)| Ok((self.tape.node_values(node, width)?, width)))
			.collect::<Result<Vec<_>>>()?;
		let mut values = Vec::new();
		for row in 0..self.tape.capacity {
			for (block, width) in &blocks {
				values.extend_from_slice(&block[row * width..(row + 1) * width]);
			}
		}
		Ok(values)
	}
}
fn train_rat(
	state: &mut State,
	train: &Train,
	predictor: &Model,
	measurement: &[f64],
	config: Config,
) -> Result<Vec<f64>> {
	state.tape.write_targets(measurement)?;
	state.targets.copy_from_slice(measurement);
	require(state.models.len() >= 2, "tile RAT requires proposer and predictor ranges")?;
	let predictor_range = *state.models.last().ok_or_else(|| RecipeError::new("RAT predictor range is absent"))?;
	let proposer_range = (state.models[0].0, state.models[state.models.len() - 2].1);
	let objective = vec![0.0; measurement.len()];
	let run = (!train.log_metrics.is_empty()).then(|| RUN.fetch_add(1, Ordering::Relaxed) + 1).unwrap_or(0);
	let mut current = Vec::new();
	for _ in 0..train.epochs {
		let started = Instant::now();
		state.tape.advance()?;
		state.tape.write_targets(measurement)?;
		state.tape.trainable(&state.stored.graph, predictor_range)?;
		let (loss, _) = state.tape.epoch(train.learning_rate, predictor.loss, 0.0, config, false)?;
		let predictions = state.tape.predictions()?;
		state.tape.trainable(&state.stored.graph, proposer_range)?;
		state.tape.write_targets(&objective)?;
		state.tape.epoch(train.learning_rate, mse, 0.0, config, false)?;
		let seconds = started.elapsed().as_secs_f64();
		train.print(predictor, run, state.tape.step as usize, loss, &state.targets, &predictions, seconds, false);
		current = predictions;
	}
	state.tape.write_targets(measurement)?;
	Ok(current)
}
impl<const N: usize> RatTrain<N> {
	pub fn every(mut self, command: impl Into<String>) -> Self { self.command = Some(command.into()); self }
	pub fn save(mut self, path: impl Into<String>) -> Self { self.train.save = Some(path.into()); self }
	pub fn resume(mut self, path: impl Into<String>) -> Self { self.train.resume = Some(path.into()); self }
	fn process(&mut self) -> Result<&mut Process> {
		if self.process.is_none() {
			let command = self.command.as_deref().ok_or_else(|| RecipeError::new("RAT requires .every()"))?;
			self.process = Some(Process::spawn(command)?);
		}
		self.process.as_mut().ok_or_else(|| RecipeError::new("RAT command is absent"))
	}
	fn check_interrupt(&mut self, state: Option<&mut State>) -> Result<()> {
		if !INTERRUPTED.load(Ordering::Acquire) {
			return Ok(());
		}
		if let Some(state) = state
			&& let Some(path) = &self.train.save
		{
			checkpoint(path, &state.schema, &mut state.stored, &state.tape, false)?;
		}
		drop(self.process.take());
		std::process::exit(INTERRUPTED_EXIT)
	}
	pub fn run(&mut self, data: &Data) -> RatReport {
		SIGNAL.get_or_init(|| unsafe { signal(SIGINT, interrupt) });
		self.try_run(data).unwrap_or_else(|error| panic!("{error}"))
	}
	fn try_run(&mut self, data: &Data) -> Result<RatReport> {
		if INTERRUPTED.load(Ordering::Acquire) {
			let mut state = self.state.take();
			self.check_interrupt(state.as_mut())?;
		}
		drop(topology()?);
		let gpus = all_gpus()?;
		let config = Config::load()?;
		let context = match self.context.take() {
			Some(context) => context,
			None => self.process()?.read(config.rat_batch)?,
		};
		self.check_interrupt(None)?;
		if self.state.is_none() {
			self.state = Some(build(&self.models, &self.train, data, &gpus, config)?);
		}
		let mut state = self.state.take().ok_or_else(|| RecipeError::new("RAT state is absent"))?;
		let mut samples = Vec::new();
		for frame in &context {
			samples.extend(frame.values(&state.stored.inputs)?);
		}
		state.tape.write_samples(&samples)?;
		state.tape.forward()?;
		let mut proposal = state.proposal()?;
		let width = state.proposal_names.len();
		require(!proposal.is_empty(), "RAT proposal batch is empty")?;
		for (index, (row, frame)) in proposal.chunks_exact_mut(width).zip(&context).enumerate() {
			let limit =
				state.tape.proposal_limit(index, [frame.value("M")?, frame.value("N")?, frame.value("K")?])?;
			let tile = Tile::proposed(row, Tile { m: 1, n: 1, k: 1 }, limit)?;
			state.tape.set_tile(index, tile)?;
		}
		let written = self.process()?.write(&state.proposal_names, &proposal);
		self.check_interrupt(Some(&mut state))?;
		written?;
		let result = self.process()?.read(config.rat_batch);
		self.check_interrupt(Some(&mut state))?;
		let result = result?;
		let mut measurement = Vec::new();
		for frame in &result {
			measurement.extend(frame.values(&data.target)?);
		}
		let prediction = train_rat(&mut state, &self.train, &self.models[N - 1], &measurement, config)?;
		self.check_interrupt(Some(&mut state))?;
		self.context = Some(result);
		if let Some(path) = &self.train.save {
			checkpoint(path, &state.schema, &mut state.stored, &state.tape, false)?;
		}
		self.state = Some(state);
		Ok(RatReport { proposal, prediction, measurement })
	}
}
